//! 跨轮审稿上下文冻结于委托时；旧批注、决定和观察保留旧身份，不重锚为新建议。
use super::*;
use crate::agent::{creative::DraftReference, types::Task};
use serde::Deserialize;

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct RevisionDecision {
    pub change_id: String,
    pub node_type: String,
    pub kind: String,
    pub decision: String,
    pub field_decisions: std::collections::BTreeMap<String, String>,
}
pub(crate) use crate::agent::revision_observations::RevisionObservation;
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct FrozenRevisionContext {
    pub parent: DraftReference,
    pub parent_title: String,
    pub review_cursor: i64,
    pub decisions: Vec<RevisionDecision>,
    pub observations: Vec<RevisionObservation>,
}
#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct RevisionContextPage {
    pub draft_id: String,
    pub parent: DraftReference,
    pub parent_title: String,
    pub discussion: EditorialDiscussion,
    pub decisions: Vec<RevisionDecision>,
    pub observations: Vec<Value>,
    pub total_decisions: usize,
    pub total_observations: usize,
    pub next_offset: Option<usize>,
    pub has_earlier: bool,
}
pub(crate) async fn freeze(
    tx: &mut Transaction<'_, Sqlite>,
    task: &Task,
    parent: &DraftReference,
    snapshot: &Value,
    cursor: i64,
) -> Result<FrozenRevisionContext> {
    let comparison = comparison_tx(tx, &task.project_id, &parent.id).await?;
    let decisions = comparison
        .changes
        .into_iter()
        .map(|c| RevisionDecision {
            change_id: c.id,
            node_type: c.node_type,
            kind: c.kind,
            decision: c.decision,
            field_decisions: c.field_decisions,
        })
        .collect();
    let inherited = if let Some(creator) = snapshot["parentTaskId"].as_str() {
        linked(tx, creator, &parent.id)
            .await?
            .map_or_else(Vec::new, |c| c.observations)
    } else {
        Vec::new()
    };
    let observations = crate::agent::revision_observations::collect(
        tx,
        task,
        snapshot["parentTaskId"].as_str(),
        inherited,
    )
    .await?;
    Ok(FrozenRevisionContext {
        parent: parent.clone(),
        parent_title: snapshot["board"]["title"]
            .as_str()
            .unwrap_or_default()
            .into(),
        review_cursor: cursor,
        decisions,
        observations,
    })
}
pub(crate) async fn saved(
    tx: &mut Transaction<'_, Sqlite>,
    task: &str,
) -> Result<Option<FrozenRevisionContext>> {
    let bound = crate::agent::creative::continuation(tx, task).await?;
    let Some(hash) = bound["reviewContextSha256"].as_str() else {
        return Ok(None);
    };
    let raw:Option<String>=sqlx::query_scalar("SELECT body FROM task_messages WHERE task_id=? AND kind='creative_review_context' ORDER BY id LIMIT 1").bind(task).fetch_optional(&mut **tx).await?;
    let value: Value = serde_json::from_str(
        &raw.ok_or_else(|| AppError::new("RECOVERY_ERROR", "修订依据记录不完整"))?,
    )
    .map_err(serialization)?;
    if fingerprint(&value) != hash
        || value["parent"] != bound["reference"]
        || value["reviewCursor"] != bound["reviewCursor"]["cursor"]
    {
        return Err(AppError::new("RECOVERY_ERROR", "修订依据与原委托不一致"));
    }
    serde_json::from_value(value)
        .map(Some)
        .map_err(serialization)
}
async fn linked(
    tx: &mut Transaction<'_, Sqlite>,
    task: &str,
    draft: &str,
) -> Result<Option<FrozenRevisionContext>> {
    let rows:Vec<String>=sqlx::query_scalar("SELECT body_json FROM agent_steps WHERE task_id=? AND kind='draft_lineage' AND json_extract(body_json,'$.childId')=? ORDER BY id LIMIT 2").bind(task).bind(draft).fetch_all(&mut **tx).await?;
    if rows.is_empty() {
        return Ok(None);
    }
    if rows.len() != 1 {
        return Err(AppError::new("RECOVERY_ERROR", "方案来源记录不一致"));
    }
    let lineage: Value = serde_json::from_str(&rows[0]).map_err(serialization)?;
    let Some(context) = saved(tx, task).await? else {
        return Ok(None);
    };
    if lineage["kind"] != "board"
        || lineage["parent"] != serde_json::to_value(&context.parent).map_err(serialization)?
    {
        return Ok(None);
    }
    let bound = crate::agent::creative::continuation(tx, task).await?;
    if lineage["parentSnapshotSha256"] != bound["snapshotSha256"] {
        return Err(AppError::new("RECOVERY_ERROR", "方案来源与修订依据不一致"));
    }
    Ok(Some(context))
}
pub(crate) fn preview(context: &FrozenRevisionContext, offset: usize, limit: usize) -> Value {
    json!({"source":"frozen_parent_review","visualEvidence":false,"decisions":context.decisions.iter().skip(offset).take(limit).collect::<Vec<_>>(),"totalDecisions":context.decisions.len(),"historicalObservations":context.observations.iter().skip(offset).take(limit).collect::<Vec<_>>(),"totalObservations":context.observations.len(),"notice":"决定属于父版，不能自动重锚或视为长期偏好；观察属于原照片哈希和旧任务，未重新观察当前版本"})
}
impl Store {
    pub async fn editorial_revision_context(
        &self,
        project: &str,
        draft: &str,
        offset: usize,
    ) -> Result<Option<RevisionContextPage>> {
        if offset > 5000 {
            return Err(AppError::new("VALIDATION_ERROR", "页码无效"));
        }
        let mut tx = self.pool.begin().await?;
        let task:Option<String>=sqlx::query_scalar("SELECT d.task_id FROM boards b JOIN board_draft_tasks d ON d.board_id=b.id JOIN projects p ON p.id=b.project_id WHERE b.project_id=? AND b.id=? AND p.deleted_at IS NULL").bind(project).bind(draft).fetch_optional(&mut *tx).await?;
        let task = task.ok_or_else(|| AppError::new("NOT_FOUND", "这份方案不存在"))?;
        let Some(context) = linked(&mut tx, &task, draft).await? else {
            return Ok(None);
        };
        let discussion = comments::discussion_tx(
            &mut tx,
            project,
            &context.parent.id,
            context.parent.revision,
            Some(context.review_cursor),
            offset,
            30,
        )
        .await?;
        let observations = crate::agent::revision_observations::project(
            &mut tx,
            project,
            &context.observations,
            offset,
            30,
        )
        .await?;
        let parent_task: Option<String> =
            sqlx::query_scalar("SELECT task_id FROM board_draft_tasks WHERE board_id=?")
                .bind(&context.parent.id)
                .fetch_optional(&mut *tx)
                .await?;
        let has_earlier = if let Some(parent_task) = parent_task {
            linked(&mut tx, &parent_task, &context.parent.id)
                .await?
                .is_some()
        } else {
            false
        };
        let total = context
            .decisions
            .len()
            .max(context.observations.len())
            .max(discussion.total_messages);
        let result = RevisionContextPage {
            draft_id: draft.into(),
            parent: context.parent,
            parent_title: context.parent_title,
            discussion,
            decisions: context
                .decisions
                .iter()
                .skip(offset)
                .take(30)
                .cloned()
                .collect(),
            observations,
            total_decisions: context.decisions.len(),
            total_observations: context.observations.len(),
            next_offset: (offset + 30 < total).then_some(offset + 30),
            has_earlier,
        };
        tx.commit().await?;
        Ok(Some(result))
    }
}
