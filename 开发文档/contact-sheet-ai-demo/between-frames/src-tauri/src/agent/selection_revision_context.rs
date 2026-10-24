//! 选片方案的跨轮依据；冻结父版内容和人工记录，不以草案状态冒充当前作品决定。
use super::{
    creative::DraftReference,
    revision_observations::{self, RevisionObservation},
    types::{DraftItem, Task},
};
use crate::{
    command_audit::{encode, fingerprint, serialization},
    commands_generated::GuiCommand,
    db::Store,
    domain::{AppError, Result},
};
use serde::{Deserialize, Serialize};
use serde_json::{Value, json};
use sqlx::{Sqlite, Transaction};
use std::collections::HashMap;

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct SelectionRevisionItem {
    pub photo_id: String,
    pub photo_hash: String,
    pub photo_name: String,
    pub position: i64,
    pub action: String,
    pub rationale: String,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct SelectionFeedback {
    pub message_id: i64,
    pub role: String,
    pub content: String,
    pub created_at: String,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct SelectionReviewEvent {
    pub command_id: String,
    pub kind: String,
    pub created_at: String,
    pub mode: Option<String>,
    pub photo_ids: Vec<String>,
    pub photo_names: Vec<String>,
    pub partial: bool,
    pub undone_at_delegation: Option<bool>,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct FrozenSelectionContext {
    pub parent: DraftReference,
    pub parent_task_id: String,
    pub parent_title: String,
    pub parent_summary: String,
    pub parent_state: String,
    pub parent_base_revision: i64,
    pub items: Vec<SelectionRevisionItem>,
    pub feedback: Vec<SelectionFeedback>,
    pub reviews: Vec<SelectionReviewEvent>,
    pub observations: Vec<RevisionObservation>,
}
#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct SelectionContextPage {
    pub draft_id: String,
    pub parent: DraftReference,
    pub parent_title: String,
    pub parent_summary: String,
    pub parent_state: String,
    pub parent_base_revision: i64,
    pub items: Vec<Value>,
    pub feedback: Vec<SelectionFeedback>,
    pub reviews: Vec<SelectionReviewEvent>,
    pub observations: Vec<Value>,
    pub total_items: usize,
    pub total_feedback: usize,
    pub total_reviews: usize,
    pub total_observations: usize,
    pub next_offset: Option<usize>,
    pub has_earlier: bool,
}
fn too_large() -> AppError {
    AppError::new("VALIDATION_ERROR", "这份方案的修改记录较多，请缩小修改范围")
}

pub(crate) async fn freeze(
    tx: &mut Transaction<'_, Sqlite>,
    task: &Task,
    parent: &DraftReference,
    snapshot: &Value,
) -> Result<Value> {
    let creator = snapshot["parentTaskId"]
        .as_str()
        .ok_or_else(|| AppError::new("RECOVERY_ERROR", "方案来源不完整"))?;
    let photos:Vec<(String,String,String)>=sqlx::query_as("SELECT id,content_sha256,filename FROM photos WHERE project_id=? AND id IN(SELECT value FROM json_each(?))")
        .bind(&task.project_id).bind(encode(&task.allowed_photo_ids)?).fetch_all(&mut **tx).await?;
    let photos: HashMap<_, _> = photos
        .into_iter()
        .map(|(id, hash, name)| (id, (hash, name)))
        .collect();
    let parent_items: Vec<DraftItem> =
        serde_json::from_value(snapshot["items"].clone()).map_err(serialization)?;
    if parent_items.len() > 1000 {
        return Err(too_large());
    }
    let mut items = Vec::new();
    for item in parent_items {
        let (hash, name) = photos
            .get(&item.photo_id)
            .ok_or_else(|| AppError::new("PERMISSION_DENIED", "照片不在本次修改范围内"))?;
        items.push(SelectionRevisionItem {
            photo_id: item.photo_id,
            photo_hash: hash.clone(),
            photo_name: name.clone(),
            position: item.position,
            action: item.proposed_action,
            rationale: item.rationale,
        });
    }
    let parent_base_revision: i64 =
        sqlx::query_scalar("SELECT base_revision FROM drafts WHERE id=? AND project_id=?")
            .bind(&parent.id)
            .bind(&task.project_id)
            .fetch_one(&mut **tx)
            .await?;
    // 在读取正文前按字节检查；只收录父任务本身的提问和人工答复，不复制全项目聊天。
    let (count,bytes):(i64,i64)=sqlx::query_as("SELECT count(*),COALESCE(sum(size),0) FROM (SELECT length(CAST(body AS BLOB)) size FROM task_messages WHERE task_id=? AND kind IN('user_goal','clarification_question','clarification_answer') ORDER BY id LIMIT 1001)")
        .bind(creator).fetch_one(&mut **tx).await?;
    if count > 1000 || bytes > 2 * 1024 * 1024 {
        return Err(too_large());
    }
    let feedback:Vec<(i64,String,String,String)>=sqlx::query_as("SELECT id,kind,body,created_at FROM task_messages WHERE task_id=? AND kind IN('user_goal','clarification_question','clarification_answer') ORDER BY id LIMIT 1000")
        .bind(creator).fetch_all(&mut **tx).await?;
    let feedback = feedback
        .into_iter()
        .map(|(id, kind, content, at)| SelectionFeedback {
            message_id: id,
            role: if kind == "clarification_question" {
                "assistant"
            } else {
                "user"
            }
            .into(),
            content,
            created_at: at,
        })
        .collect();
    let (count,bytes):(i64,i64)=sqlx::query_as("SELECT count(*),COALESCE(sum(size),0) FROM (SELECT length(CAST(command_json AS BLOB)) size FROM command_events WHERE project_id=? AND actor_type='human' AND command_type IN('ApplyDraft','UpdateDraft','RebaseDraft','DiscardDraft') AND json_extract(command_json,'$.command.draftId')=? ORDER BY rowid LIMIT 1001)")
        .bind(&task.project_id).bind(&parent.id).fetch_one(&mut **tx).await?;
    if count > 1000 || bytes > 2 * 1024 * 1024 {
        return Err(too_large());
    }
    let events:Vec<(String,String,String,Option<bool>)>=sqlx::query_as("SELECT e.command_id,e.command_json,e.created_at,CASE WHEN h.group_id IS NULL THEN NULL ELSE (h.active=0 OR h.position>COALESCE(head.cursor,0)) END FROM command_events e LEFT JOIN command_history h ON h.group_id=e.id LEFT JOIN history_heads head ON head.project_id=e.project_id WHERE e.project_id=? AND e.actor_type='human' AND e.command_type IN('ApplyDraft','UpdateDraft','RebaseDraft','DiscardDraft') AND json_extract(e.command_json,'$.command.draftId')=? ORDER BY e.rowid LIMIT 1000")
        .bind(&task.project_id).bind(&parent.id).fetch_all(&mut **tx).await?;
    let mut reviews = Vec::new();
    for (id, raw, at, undone) in events {
        let value: Value = serde_json::from_str(&raw).map_err(serialization)?;
        let command: GuiCommand =
            serde_json::from_value(value["command"].clone()).map_err(serialization)?;
        let (kind, expected, mode, ids) = match command {
            GuiCommand::ApplyDraft {
                expected_draft_revision,
                mode,
                approved_photo_ids,
                ..
            } => (
                "apply",
                expected_draft_revision,
                Some(mode),
                approved_photo_ids,
            ),
            GuiCommand::UpdateDraft {
                expected_draft_revision,
                ordered_photo_ids,
                ..
            } => ("reorder", expected_draft_revision, None, ordered_photo_ids),
            GuiCommand::RebaseDraft {
                expected_draft_revision,
                ..
            } => ("rebase", expected_draft_revision, None, vec![]),
            GuiCommand::DiscardDraft {
                expected_draft_revision,
                ..
            } => ("discard", expected_draft_revision, None, vec![]),
            _ => return Err(AppError::new("RECOVERY_ERROR", "人工决定记录不完整")),
        };
        if value["projectId"] != task.project_id || expected > parent.revision {
            return Err(AppError::new("RECOVERY_ERROR", "人工决定与方案版本不一致"));
        }
        let count = ids.len();
        let ids: Vec<_> = ids
            .into_iter()
            .filter(|id| photos.contains_key(id))
            .collect();
        let names = ids.iter().map(|id| photos[id].1.clone()).collect();
        reviews.push(SelectionReviewEvent {
            command_id: id,
            kind: kind.into(),
            created_at: at,
            mode,
            partial: ids.len() != count,
            photo_ids: ids,
            photo_names: names,
            undone_at_delegation: undone,
        });
    }
    let inherited = linked(tx, creator, &parent.id)
        .await?
        .map_or_else(Vec::new, |context| context.observations);
    let observations = revision_observations::collect(tx, task, Some(creator), inherited).await?;
    let context = FrozenSelectionContext {
        parent: parent.clone(),
        parent_task_id: creator.into(),
        parent_title: snapshot["title"].as_str().unwrap_or_default().into(),
        parent_summary: snapshot["summary"].as_str().unwrap_or_default().into(),
        parent_state: snapshot["state"].as_str().unwrap_or_default().into(),
        parent_base_revision,
        items,
        feedback,
        reviews,
        observations,
    };
    let value = serde_json::to_value(context).map_err(serialization)?;
    let raw = encode(&value)?;
    if raw.len() > 2 * 1024 * 1024 {
        return Err(too_large());
    }
    sqlx::query("INSERT INTO task_messages(task_id,kind,body,created_at) VALUES(?,'selection_review_context',?,?)")
        .bind(&task.id).bind(raw).bind(crate::domain::timestamp()).execute(&mut **tx).await?;
    Ok(value)
}
pub(crate) async fn saved(
    tx: &mut Transaction<'_, Sqlite>,
    task: &str,
) -> Result<Option<FrozenSelectionContext>> {
    let bound = super::creative::continuation(tx, task).await?;
    let Some(hash) = bound["selectionContextSha256"].as_str() else {
        return Ok(None);
    };
    let raw:Option<String>=sqlx::query_scalar("SELECT body FROM task_messages WHERE task_id=? AND kind='selection_review_context' ORDER BY id LIMIT 1").bind(task).fetch_optional(&mut **tx).await?;
    let raw = raw.ok_or_else(|| AppError::new("RECOVERY_ERROR", "修订依据记录不完整"))?;
    if raw.len() > 2 * 1024 * 1024 {
        return Err(too_large());
    }
    let value: Value = serde_json::from_str(&raw).map_err(serialization)?;
    if fingerprint(&value) != hash
        || value["parent"] != bound["reference"]
        || value["parentTaskId"] != bound["parentTaskId"]
    {
        return Err(AppError::new("RECOVERY_ERROR", "修订依据与委托不一致"));
    }
    serde_json::from_value(value)
        .map(Some)
        .map_err(serialization)
}
async fn linked(
    tx: &mut Transaction<'_, Sqlite>,
    task: &str,
    draft: &str,
) -> Result<Option<FrozenSelectionContext>> {
    let rows:Vec<String>=sqlx::query_scalar("SELECT body_json FROM agent_steps WHERE task_id=? AND kind='draft_lineage' AND json_extract(body_json,'$.childId')=? ORDER BY id LIMIT 2").bind(task).bind(draft).fetch_all(&mut **tx).await?;
    if rows.is_empty() {
        return Ok(None);
    }
    if rows.len() != 1 {
        return Err(AppError::new("RECOVERY_ERROR", "方案来源记录不一致"));
    }
    let link: Value = serde_json::from_str(&rows[0]).map_err(serialization)?;
    let Some(context) = saved(tx, task).await? else {
        return Ok(None);
    };
    if link["kind"] != "selection"
        || link["parent"] != serde_json::to_value(&context.parent).map_err(serialization)?
    {
        return Ok(None);
    }
    let bound = super::creative::continuation(tx, task).await?;
    if link["parentSnapshotSha256"] != bound["snapshotSha256"]
        || link["parentTaskId"] != bound["parentTaskId"]
    {
        return Err(AppError::new("RECOVERY_ERROR", "方案来源与修订依据不一致"));
    }
    Ok(Some(context))
}
pub(crate) fn preview(context: &FrozenSelectionContext, offset: usize, limit: usize) -> Value {
    json!({"source":"frozen_selection_review","visualEvidence":false,"parent":context.parent,"id":context.parent.id,"revision":context.parent.revision,"title":context.parent_title,"summary":context.parent_summary,"stateAtDelegation":context.parent_state,"baseRevision":context.parent_base_revision,"frozenForThisTask":true,"count":context.items.len(),"nextOffset":if context.items.len()>offset+limit{Some(offset+limit)}else{None},
        "items":context.items.iter().skip(offset).take(limit).map(|item|json!({"photo_id":item.photo_id,"proposed_action":item.action,"position":item.position,"rationale":item.rationale,"sourceHash":item.photo_hash})).collect::<Vec<_>>(),"feedback":context.feedback.iter().skip(offset).take(limit).collect::<Vec<_>>(),"reviews":context.reviews.iter().skip(offset).take(limit).collect::<Vec<_>>(),"historicalObservations":context.observations.iter().skip(offset).take(limit).collect::<Vec<_>>(),
        "totalItems":context.items.len(),"totalFeedback":context.feedback.len(),"totalReviews":context.reviews.len(),"totalObservations":context.observations.len(),"notice":"父版建议不是人工采纳；历史操作与提问保留原身份，不能当作当前作品或长期偏好；照片观察属于旧源版本，不代表重新看过当前修图版本"})
}
impl Store {
    pub async fn selection_revision_context(
        &self,
        project: &str,
        draft: &str,
        offset: usize,
    ) -> Result<Option<SelectionContextPage>> {
        if offset > 5000 {
            return Err(AppError::new("VALIDATION_ERROR", "页码无效"));
        }
        let mut tx = self.pool.begin().await?;
        let creator:Option<String>=sqlx::query_scalar("SELECT d.task_id FROM drafts d JOIN projects p ON p.id=d.project_id WHERE d.id=? AND d.project_id=? AND p.deleted_at IS NULL").bind(draft).bind(project).fetch_optional(&mut *tx).await?;
        let creator = creator.ok_or_else(|| AppError::new("NOT_FOUND", "这份方案不存在"))?;
        let Some(context) = linked(&mut tx, &creator, draft).await? else {
            return Ok(None);
        };
        let mut items = Vec::new();
        for item in context.items.iter().skip(offset).take(30) {
            let row: Option<(String, String)> = sqlx::query_as(
                "SELECT content_sha256,preview_path FROM photos WHERE project_id=? AND id=?",
            )
            .bind(project)
            .bind(&item.photo_id)
            .fetch_optional(&mut *tx)
            .await?;
            let matches = row
                .as_ref()
                .is_some_and(|(hash, _)| hash == &item.photo_hash);
            let mut value = serde_json::to_value(item).map_err(serialization)?;
            value["currentHashMatches"] = json!(matches);
            value["thumbnailPath"] = json!(row.as_ref().filter(|_| matches).map(|(_, path)| path));
            items.push(value);
        }
        let observations =
            revision_observations::project(&mut tx, project, &context.observations, offset, 30)
                .await?;
        let has_earlier = linked(&mut tx, &context.parent_task_id, &context.parent.id)
            .await?
            .is_some();
        let total = context
            .items
            .len()
            .max(context.feedback.len())
            .max(context.reviews.len())
            .max(context.observations.len());
        let result = SelectionContextPage {
            draft_id: draft.into(),
            parent: context.parent,
            parent_title: context.parent_title,
            parent_summary: context.parent_summary,
            parent_state: context.parent_state,
            parent_base_revision: context.parent_base_revision,
            items,
            feedback: context
                .feedback
                .iter()
                .skip(offset)
                .take(30)
                .cloned()
                .collect(),
            reviews: context
                .reviews
                .iter()
                .skip(offset)
                .take(30)
                .cloned()
                .collect(),
            observations,
            total_items: context.items.len(),
            total_feedback: context.feedback.len(),
            total_reviews: context.reviews.len(),
            total_observations: context.observations.len(),
            next_offset: (offset + 30 < total).then_some(offset + 30),
            has_earlier,
        };
        tx.commit().await?;
        Ok(Some(result))
    }
}
