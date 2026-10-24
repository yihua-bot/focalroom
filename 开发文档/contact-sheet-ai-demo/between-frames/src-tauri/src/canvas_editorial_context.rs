//! 跨轮审稿上下文冻结于委托时；旧批注、决定和观察保留旧身份，不重锚为新建议。
use super::*;
use crate::agent::{
    creative::DraftReference,
    types::{Task, ToolResult},
};
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
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct RevisionObservation {
    pub photo_id: String,
    pub photo_hash: String,
    pub model_id: String,
    pub prompt_version: String,
    pub task_id: String,
    pub call_id: String,
    pub visible_facts: Vec<String>,
    pub interpretation: String,
    pub uncertainty: String,
    pub matched_at_delegation: bool,
}
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
    let mut observations = std::collections::BTreeMap::new();
    if let Some(creator) = snapshot["parentTaskId"].as_str() {
        if let Some(prior) = saved(tx, creator).await? {
            for observation in prior
                .observations
                .into_iter()
                .filter(|o| task.allowed_photo_ids.contains(&o.photo_id))
            {
                observations.insert(observation.photo_id.clone(), observation);
            }
        }
        let rows:Vec<(String,String)> = sqlx::query_as("SELECT call_id,result_json FROM tool_calls WHERE task_id=? AND tool_name IN('inspect_photos','compare_photos') ORDER BY rowid LIMIT 1001")
            .bind(creator).fetch_all(&mut **tx).await?;
        if rows.len() > 1000 {
            return Err(AppError::new(
                "VALIDATION_ERROR",
                "这份方案的观察记录较多，请缩小修改范围",
            ));
        }
        for (call_id, raw) in rows {
            let receipt: ToolResult = serde_json::from_str(&raw).map_err(serialization)?;
            let Some(data) = receipt
                .result
                .filter(|v| receipt.ok && v["mode"] == "vision_observed")
            else {
                continue;
            };
            for p in data["provenance"].as_array().into_iter().flatten() {
                let Some(photo) = p["photo_id"]
                    .as_str()
                    .filter(|id| task.allowed_photo_ids.iter().any(|allowed| allowed == id))
                else {
                    continue;
                };
                let Some(hash) = p["photo_hash"]
                    .as_str()
                    .filter(|hash| hash.len() == 64 && hash.bytes().all(|c| c.is_ascii_hexdigit()))
                else {
                    continue;
                };
                let Some(value) = data["observations"]
                    .as_array()
                    .into_iter()
                    .flatten()
                    .find(|v| v["photo_id"] == photo)
                else {
                    continue;
                };
                if !crate::agent::vision::valid_cached_observation(value, photo) {
                    continue;
                }
                observations.insert(
                    photo.to_owned(),
                    RevisionObservation {
                        photo_id: photo.into(),
                        photo_hash: hash.into(),
                        model_id: p["model_id"].as_str().unwrap_or_default().into(),
                        prompt_version: p["prompt_version"].as_str().unwrap_or_default().into(),
                        task_id: creator.into(),
                        call_id: call_id.clone(),
                        visible_facts: serde_json::from_value(value["visible_facts"].clone())
                            .map_err(serialization)?,
                        interpretation: value["interpretation"].as_str().unwrap_or_default().into(),
                        uncertainty: value["uncertainty"].as_str().unwrap_or_default().into(),
                        matched_at_delegation: false,
                    },
                );
            }
        }
    }
    let current:Vec<(String,String)>=sqlx::query_as("SELECT id,content_sha256 FROM photos WHERE project_id=?")
        .bind(&task.project_id).fetch_all(&mut **tx).await?;
    let current:HashMap<_,_>=current.into_iter().collect();
    for observation in observations.values_mut() {
        observation.matched_at_delegation=current.get(&observation.photo_id)==Some(&observation.photo_hash);
    }
    Ok(FrozenRevisionContext {
        parent: parent.clone(),
        parent_title: snapshot["board"]["title"]
            .as_str()
            .unwrap_or_default()
            .into(),
        review_cursor: cursor,
        decisions,
        observations: observations.into_values().collect(),
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
        let Some(context) = saved(&mut tx, &task).await? else {
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
        let mut observations = Vec::new();
        for o in context.observations.iter().skip(offset).take(30) {
            let current: Option<String> =
                sqlx::query_scalar("SELECT content_sha256 FROM photos WHERE project_id=? AND id=?")
                    .bind(project)
                    .bind(&o.photo_id)
                    .fetch_optional(&mut *tx)
                    .await?;
            let mut value = serde_json::to_value(o).map_err(serialization)?;
            value["currentHashMatches"] = json!(current.as_deref() == Some(o.photo_hash.as_str()));
            observations.push(value);
        }
        let parent_task: Option<String> =
            sqlx::query_scalar("SELECT task_id FROM board_draft_tasks WHERE board_id=?")
                .bind(&context.parent.id)
                .fetch_optional(&mut *tx)
                .await?;
        let has_earlier = if let Some(parent_task) = parent_task {
            saved(&mut tx, &parent_task).await?.is_some()
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
