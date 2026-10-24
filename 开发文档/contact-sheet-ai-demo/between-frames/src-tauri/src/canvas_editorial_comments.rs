//! 审稿讨论追加到人工命令日志；批注不会改变草案像素、节点版本或正式作品。
use super::*;
use serde::Deserialize;

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct AddEditorialComment {
    pub draft_id: String,
    pub draft_revision: i64,
    pub change_id: Option<String>,
    pub thread_id: Option<String>,
    pub body: String,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct ResolveEditorialThread {
    pub draft_id: String,
    pub draft_revision: i64,
    pub thread_id: String,
    pub resolved: bool,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct EditorialMessage {
    pub id: String,
    pub thread_id: String,
    pub change_id: Option<String>,
    pub body: String,
    pub root_body: String,
    pub created_at: String,
    pub resolved: bool,
    pub is_reply: bool,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct EditorialDiscussion {
    pub draft_id: String,
    pub draft_revision: i64,
    pub cursor: i64,
    pub messages: Vec<EditorialMessage>,
    pub total_messages: usize,
    pub event_count: usize,
    pub next_offset: Option<usize>,
}

async fn full_discussion_tx(
    tx: &mut Transaction<'_, Sqlite>,
    project: &str,
    draft: &str,
    revision: i64,
    cursor: Option<i64>,
) -> Result<EditorialDiscussion> {
    if revision < 1 || cursor.is_some_and(|v| v < 0) {
        return Err(AppError::new("VALIDATION_ERROR", "请重新打开要讨论的方案"));
    }
    let valid: bool = sqlx::query_scalar("SELECT EXISTS(SELECT 1 FROM boards b JOIN projects p ON p.id=b.project_id JOIN board_draft_tasks t ON t.board_id=b.id WHERE b.id=? AND b.project_id=? AND b.revision>=? AND p.deleted_at IS NULL)")
        .bind(draft).bind(project).bind(revision).fetch_one(&mut **tx).await?;
    if !valid {
        return Err(AppError::new("NOT_FOUND", "这份方案的讨论不存在"));
    }
    let rows: Vec<(i64,String,String)> = sqlx::query_as("SELECT e.rowid,e.request_json,e.created_at FROM canvas_command_events e WHERE e.project_id=? AND e.board_id=? AND (? IS NULL OR e.rowid<=?) AND json_extract(e.request_json,'$.operations[0].action.type') IN ('AddEditorialComment','ResolveEditorialThread') AND json_extract(e.request_json,'$.operations[0].action.data.draftRevision')=? ORDER BY e.rowid LIMIT 5001")
        .bind(project).bind(draft).bind(cursor).bind(cursor).bind(revision).fetch_all(&mut **tx).await?;
    if rows.len() > 5000 {
        return Err(AppError::new(
            "VALIDATION_ERROR",
            "本版讨论较多，请创建下一版方案后继续",
        ));
    }
    let event_count = rows.len();
    let effective_cursor = cursor.unwrap_or_else(|| rows.last().map_or(0, |r| r.0));
    let mut roots: HashMap<String, (Option<String>, String)> = HashMap::new();
    let mut resolved = HashMap::new();
    let mut messages = Vec::new();
    for (_, raw, created_at) in rows {
        let request: crate::canvas_commands::CanvasCommandRequest =
            serde_json::from_str(&raw).map_err(serialization)?;
        let operation = request
            .operations
            .first()
            .ok_or_else(|| AppError::new("DATABASE", "讨论记录不完整"))?;
        match &operation.action {
            crate::canvas_commands::CanvasAction::AddEditorialComment(cmd) => {
                let thread = cmd.thread_id.as_ref().unwrap_or(&operation.op_id).clone();
                if cmd.thread_id.is_none() {
                    roots.insert(thread.clone(), (cmd.change_id.clone(), cmd.body.clone()));
                }
                let (change, root_body) = roots
                    .get(&thread)
                    .ok_or_else(|| AppError::new("DATABASE", "批注关联记录不完整"))?;
                messages.push(EditorialMessage {
                    id: operation.op_id.clone(),
                    thread_id: thread,
                    change_id: change.clone(),
                    body: cmd.body.clone(),
                    root_body: root_body.clone(),
                    created_at,
                    resolved: false,
                    is_reply: cmd.thread_id.is_some(),
                });
            }
            crate::canvas_commands::CanvasAction::ResolveEditorialThread(cmd) => {
                resolved.insert(cmd.thread_id.clone(), cmd.resolved);
            }
            _ => {}
        }
    }
    for message in &mut messages {
        message.resolved = resolved.get(&message.thread_id).copied().unwrap_or(false);
    }
    let total_messages = messages.len();
    Ok(EditorialDiscussion {
        draft_id: draft.into(),
        draft_revision: revision,
        cursor: effective_cursor,
        messages,
        total_messages,
        event_count,
        next_offset: None,
    })
}

pub(crate) async fn discussion_tx(
    tx: &mut Transaction<'_, Sqlite>,
    project: &str,
    draft: &str,
    revision: i64,
    cursor: Option<i64>,
    offset: usize,
    limit: usize,
) -> Result<EditorialDiscussion> {
    if !(1..=100).contains(&limit) || offset > 5000 {
        return Err(AppError::new("VALIDATION_ERROR", "讨论页码无效"));
    }
    let mut result = full_discussion_tx(tx, project, draft, revision, cursor).await?;
    result.next_offset = (offset + limit < result.total_messages).then_some(offset + limit);
    result.messages = result
        .messages
        .into_iter()
        .skip(offset)
        .take(limit)
        .collect();
    Ok(result)
}

async fn writable_discussion(
    tx: &mut Transaction<'_, Sqlite>,
    project: &str,
    draft: &str,
    revision: i64,
) -> Result<CanvasComparison> {
    let comparison = comparison_tx(tx, project, draft).await?;
    if comparison.draft.revision != revision {
        return Err(AppError::new(
            "REVISION_CONFLICT",
            "方案已更新，请重新查看后讨论",
        ));
    }
    let running: bool = sqlx::query_scalar("SELECT EXISTS(SELECT 1 FROM board_draft_tasks d JOIN agent_tasks t ON t.id=d.task_id WHERE d.board_id=? AND t.state IN('queued','planning','running','needs_input'))").bind(draft).fetch_one(&mut **tx).await?;
    if running {
        return Err(AppError::new("CONFLICT", "请等方案生成完成后再添加意见"));
    }
    Ok(comparison)
}
pub(crate) async fn validate_comment(
    tx: &mut Transaction<'_, Sqlite>,
    project: &str,
    cmd: &AddEditorialComment,
) -> Result<()> {
    let comparison = writable_discussion(tx, project, &cmd.draft_id, cmd.draft_revision).await?;
    if cmd.body.trim().is_empty()
        || cmd.body.encode_utf16().count() > 2000
        || cmd
            .change_id
            .as_ref()
            .is_some_and(|id| !comparison.changes.iter().any(|c| &c.id == id))
    {
        return Err(AppError::new(
            "VALIDATION_ERROR",
            "请为当前建议填写有效的批注意见",
        ));
    }
    let discussion =
        full_discussion_tx(tx, project, &cmd.draft_id, cmd.draft_revision, None).await?;
    if let Some(thread) = &cmd.thread_id {
        let root = discussion
            .messages
            .iter()
            .find(|m| m.thread_id == *thread && !m.is_reply)
            .ok_or_else(|| AppError::new("NOT_FOUND", "这条批注不属于当前方案"))?;
        if root.change_id != cmd.change_id || root.resolved {
            return Err(AppError::new(
                "CONFLICT",
                "请先重新打开这条批注，再继续回复",
            ));
        }
    }
    if discussion.total_messages >= 2000 || discussion.event_count >= 5000 {
        return Err(AppError::new(
            "VALIDATION_ERROR",
            "本版讨论较多，请创建下一版方案后继续",
        ));
    }
    Ok(())
}
pub(crate) async fn validate_resolution(
    tx: &mut Transaction<'_, Sqlite>,
    project: &str,
    cmd: &ResolveEditorialThread,
) -> Result<()> {
    writable_discussion(tx, project, &cmd.draft_id, cmd.draft_revision).await?;
    let discussion =
        full_discussion_tx(tx, project, &cmd.draft_id, cmd.draft_revision, None).await?;
    if discussion.event_count >= 5000 {
        return Err(AppError::new(
            "VALIDATION_ERROR",
            "本版讨论较多，请创建下一版方案后继续",
        ));
    }
    if discussion
        .messages
        .iter()
        .any(|m| m.thread_id == cmd.thread_id && !m.is_reply)
    {
        Ok(())
    } else {
        Err(AppError::new("NOT_FOUND", "这条批注不属于当前方案"))
    }
}
impl Store {
    pub async fn editorial_discussion(
        &self,
        project: &str,
        draft: &str,
        revision: i64,
        offset: usize,
        cursor: Option<i64>,
    ) -> Result<EditorialDiscussion> {
        let mut tx = self.pool.begin().await?;
        let result = discussion_tx(&mut tx, project, draft, revision, cursor, offset, 30).await?;
        tx.commit().await?;
        Ok(result)
    }
}
