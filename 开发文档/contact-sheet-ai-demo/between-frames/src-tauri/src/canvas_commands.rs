//! 人工画布命令：身份、版本、领域操作、历史组与不可变回执在同一事务提交。
use crate::{
    canvas::*,
    command_audit::{encode, serialization},
    command_handler::{CommandContext, CommandHandler},
    db::Store,
    domain::{self, AppError, Result},
};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use sqlx::{Sqlite, Transaction};
use std::collections::HashSet;

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct CanvasCommandRequest {
    pub schema_version: u8,
    pub command_id: String,
    pub idempotency_key: String,
    pub project_id: String,
    pub board_id: String,
    pub base_revision: i64,
    pub operations: Vec<CanvasOperation>,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct CanvasOperation {
    pub op_id: String,
    pub action: CanvasAction,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type", content = "data", deny_unknown_fields)]
pub enum CanvasAction {
    CreateBoard(CreateBoardCommand),
    PlacePhoto(PlacePhotoNodeCommand),
    MoveNodes(MoveNodesCommand),
    AddText(AddTextNodeCommand),
    DeleteNode(DeleteNodeCommand),
    AddAnnotation(AddAnnotationCommand),
    ResolveAnnotation(ResolveAnnotationCommand),
    AddArrow(AddArrowCommand),
    AddInk(AddInkCommand),
    GroupNodes(GroupNodesCommand),
    ResizeNode(ResizeNodesCommand),
    SetNodeLocked(SetNodeLockedCommand),
    Undo(UndoCanvasCommand),
    Redo(RedoCanvasCommand),
    AdoptBoard(AdoptBoardDraftCommand),
    ApplyEditorialChanges(crate::canvas_review::ApplyEditorialChanges),
}
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CanvasReceipt {
    pub command_id: String,
    pub project_id: String,
    pub board_id: String,
    pub base_revision: i64,
    pub revision: i64,
    pub changed: bool,
    pub results: Vec<Value>,
}
#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct CanvasSnapshot {
    pub board: Board,
    pub nodes: Vec<CanvasNode>,
    pub annotations: Vec<BoardAnnotation>,
    pub photo_sources: Vec<CanvasPhotoSource>,
}

fn invalid(message: &str) -> AppError {
    AppError::new("VALIDATION_ERROR", message)
}
fn collision() -> AppError {
    AppError::new(
        "IDEMPOTENCY_CONFLICT",
        "无法确认这次操作，请重新打开画布后重试",
    )
}
fn validate(request: &CanvasCommandRequest) -> Result<()> {
    if request.schema_version != 3
        || request.base_revision < 0
        || request.base_revision >= i64::MAX - 1000
        || request.idempotency_key.is_empty()
        || request.idempotency_key.len() > 160
        || request.project_id.is_empty()
        || request.project_id.encode_utf16().count() > 160
        || request.operations.is_empty()
        || request.operations.len() > 300
    {
        return Err(invalid("画布操作内容无效"));
    }
    // 项目保留既有稳定编号；新命令和画布本身仍要求 UUID。
    for id in [&request.command_id, &request.board_id] {
        uuid::Uuid::parse_str(id).map_err(|_| invalid("画布操作标识无效"))?;
    }
    let mut ids = HashSet::new();
    for op in &request.operations {
        uuid::Uuid::parse_str(&op.op_id).map_err(|_| invalid("画布操作标识无效"))?;
        if !ids.insert(&op.op_id) {
            return Err(invalid("同一次操作不能重复包含相同内容"));
        }
        let data = serde_json::to_value(&op.action).map_err(serialization)?;
        if let Some(board) = data["data"]["boardId"].as_str()
            && board != request.board_id
        {
            return Err(invalid("操作不属于当前画布"));
        }
        if matches!(
            op.action,
            CanvasAction::CreateBoard(_)
                | CanvasAction::Undo(_)
                | CanvasAction::Redo(_)
                | CanvasAction::AdoptBoard(_)
                | CanvasAction::ApplyEditorialChanges(_)
        ) && request.operations.len() != 1
        {
            return Err(invalid("此操作须单独提交"));
        }
        let node_ids = match &op.action {
            CanvasAction::MoveNodes(cmd) => Some(&cmd.node_ids),
            CanvasAction::GroupNodes(cmd) => Some(&cmd.node_ids),
            _ => None,
        };
        if let Some(nodes) = node_ids
            && (nodes.is_empty()
                || nodes.len() > 300
                || nodes.iter().collect::<HashSet<_>>().len() != nodes.len())
        {
            return Err(invalid("请选择不同的有效节点"));
        }
        if let CanvasAction::MoveNodes(cmd) = &op.action
            && ![cmd.dx, cmd.dy]
                .iter()
                .all(|v| v.is_finite() && v.abs() <= 1e9)
        {
            return Err(invalid("移动距离无效"));
        }
    }
    Ok(())
}

impl Store {
    pub async fn execute_canvas(&self, request: CanvasCommandRequest) -> Result<CanvasReceipt> {
        let context = CommandContext::human(&request.project_id);
        CommandHandler::new(self)
            .execute_canvas(request, &context)
            .await
    }
    /// 同一个只读事务内读取版本与投影，避免把旧节点与新版本拼在一起。
    pub async fn canvas_snapshot(&self, project: &str, board: &str) -> Result<CanvasSnapshot> {
        let mut tx = self.pool.begin().await?;
        let board = board_tx(&mut tx, project, board)
            .await?
            .ok_or_else(|| AppError::new("NOT_FOUND", "画布不存在"))?;
        let nodes = sqlx::query_as(
            "SELECT * FROM canvas_nodes WHERE board_id=? AND visible=1 ORDER BY z_order,rowid",
        )
        .bind(&board.id)
        .fetch_all(&mut *tx)
        .await?;
        let annotations=sqlx::query_as("SELECT * FROM board_annotations WHERE board_id=? AND visible=1 ORDER BY created_at,rowid")
            .bind(&board.id).fetch_all(&mut *tx).await?;
        let photo_sources=sqlx::query_as("SELECT a.id AS asset_id,MIN(p.thumbnail_path) AS thumbnail_path FROM canvas_nodes n JOIN source_assets a ON a.id=n.asset_id AND a.project_id=? JOIN asset_aliases alias ON alias.asset_id=a.id JOIN photos p ON p.id=alias.photo_id AND p.project_id=a.project_id WHERE n.board_id=? AND n.visible=1 AND n.node_type='photo' GROUP BY a.id")
            .bind(project).bind(&board.id).fetch_all(&mut *tx).await?;
        tx.commit().await?;
        Ok(CanvasSnapshot {
            board,
            nodes,
            annotations,
            photo_sources,
        })
    }
}
async fn board_tx(
    tx: &mut Transaction<'_, Sqlite>,
    project: &str,
    board: &str,
) -> Result<Option<Board>> {
    Ok(
        sqlx::query_as("SELECT * FROM boards WHERE id=? AND project_id=?")
            .bind(board)
            .bind(project)
            .fetch_optional(&mut **tx)
            .await?,
    )
}

impl CommandHandler<'_> {
    pub async fn execute_canvas(
        &self,
        request: CanvasCommandRequest,
        context: &CommandContext,
    ) -> Result<CanvasReceipt> {
        // 权限在回执查找前验证，回放不构成绕过身份的读通道。
        if !context.permits_human(&request.project_id) {
            return Err(AppError::new("PERMISSION_DENIED", "没有修改此画布的权限"));
        }
        validate(&request)?;
        let request_json = encode(&request)?;
        if request_json.len() > 2 * 1024 * 1024 {
            return Err(invalid("本次画布操作过大，请分次完成"));
        }
        let mut tx = self.store.pool.begin_with("BEGIN IMMEDIATE").await?;
        let saved:Vec<(String,String)>=sqlx::query_as("SELECT request_json,receipt_json FROM canvas_command_events WHERE command_id=? OR idempotency_key=?")
            .bind(&request.command_id).bind(&request.idempotency_key).fetch_all(&mut *tx).await?;
        if !saved.is_empty() {
            if saved.len() != 1 || saved[0].0 != request_json {
                return Err(collision());
            }
            let receipt = serde_json::from_str(&saved[0].1).map_err(serialization)?;
            tx.commit().await?;
            return Ok(receipt);
        }
        for op in &request.operations {
            let exists: bool = sqlx::query_scalar(
                "SELECT EXISTS(SELECT 1 FROM canvas_operation_ids WHERE op_id=?)",
            )
            .bind(&op.op_id)
            .fetch_one(&mut *tx)
            .await?;
            if exists {
                return Err(collision());
            }
        }
        crate::agent::promotion::ensure_local_write(&mut tx, &request.project_id).await?;
        let active: bool = sqlx::query_scalar(
            "SELECT EXISTS(SELECT 1 FROM projects WHERE id=? AND deleted_at IS NULL)",
        )
        .bind(&request.project_id)
        .fetch_one(&mut *tx)
        .await?;
        if !active {
            return Err(AppError::new("NOT_FOUND", "项目不存在或已归档"));
        }
        let existing = board_tx(&mut tx, &request.project_id, &request.board_id).await?;
        let creating = matches!(request.operations[0].action, CanvasAction::CreateBoard(_));
        let adopting = matches!(request.operations[0].action, CanvasAction::AdoptBoard(_));
        let before = existing.as_ref().map(|b| b.revision).unwrap_or(0);
        if creating && existing.is_some() {
            return Err(collision());
        }
        if let CanvasAction::CreateBoard(cmd) = &request.operations[0].action {
            let duplicate: bool = sqlx::query_scalar(
                "SELECT EXISTS(SELECT 1 FROM boards WHERE id=? OR (project_id=? AND title=?))",
            )
            .bind(&request.board_id)
            .bind(&request.project_id)
            .bind(cmd.title.trim())
            .fetch_one(&mut *tx)
            .await?;
            if duplicate {
                return Err(AppError::new("CONFLICT", "已有同名画布，请使用其他名称"));
            }
        }
        if !creating && existing.is_none() {
            return Err(AppError::new("NOT_FOUND", "画布不存在"));
        }
        if request.base_revision != before {
            let mut error = AppError::new("REVISION_CONFLICT", "画布已更新，请刷新后重新操作");
            error.current_revision = Some(before);
            return Err(error);
        }
        if let Some(board) = &existing {
            if board.state == "frozen" || (board.owning_task_id.is_some() && !adopting) {
                return Err(AppError::new(
                    "PERMISSION_DENIED",
                    "此画布暂不可编辑，请先采纳草案或打开可编辑版本",
                ));
            }
            if adopting {
                let ready:bool=sqlx::query_scalar("SELECT EXISTS(SELECT 1 FROM agent_tasks t JOIN projects p ON p.id=t.project_id WHERE t.id=? AND t.project_id=? AND t.base_revision=p.revision AND t.state IN ('review_ready','completed'))")
                    .bind(&board.owning_task_id).bind(&request.project_id).fetch_one(&mut *tx).await?;
                if board.state != "draft" || !ready {
                    return Err(AppError::new(
                        "CONFLICT",
                        "请等待选片任务完成后再采纳此画布",
                    ));
                }
            }
        }
        let history_cursor: i64 =
            sqlx::query_scalar("SELECT COALESCE(MAX(rowid),0) FROM canvas_op_log")
                .fetch_one(&mut *tx)
                .await?;
        let mut results = Vec::with_capacity(request.operations.len());
        let mut changed = false;
        for op in &request.operations {
            let (result, applied) = self.apply_canvas_operation(&mut tx, &request, op).await?;
            changed |= applied;
            results.push(result);
        }
        // 保留旧触发器兼容内部草案写入；正式命令按整次语义操作推进一次版本。
        let revision = if creating {
            1
        } else {
            before + i64::from(changed)
        };
        if changed {
            sqlx::query("UPDATE boards SET revision=?,updated_at=? WHERE id=? AND project_id=?")
                .bind(revision)
                .bind(domain::timestamp())
                .bind(&request.board_id)
                .bind(&request.project_id)
                .execute(&mut *tx)
                .await?;
        }
        sqlx::query("UPDATE canvas_op_log SET command_group=? WHERE rowid>? AND board_id=?")
            .bind(&request.command_id)
            .bind(history_cursor)
            .bind(&request.board_id)
            .execute(&mut *tx)
            .await?;
        let receipt = CanvasReceipt {
            command_id: request.command_id.clone(),
            project_id: request.project_id.clone(),
            board_id: request.board_id.clone(),
            base_revision: before,
            revision,
            changed,
            results,
        };
        sqlx::query("INSERT INTO canvas_command_events(command_id,idempotency_key,project_id,board_id,actor,base_revision,revision,request_json,receipt_json,created_at) VALUES(?,?,?,?,'human',?,?,?,?,?)")
            .bind(&request.command_id).bind(&request.idempotency_key).bind(&request.project_id).bind(&request.board_id)
            .bind(before).bind(revision).bind(request_json).bind(encode(&receipt)?).bind(domain::timestamp()).execute(&mut *tx).await?;
        for op in &request.operations {
            sqlx::query("INSERT INTO canvas_operation_ids(op_id,command_id) VALUES(?,?)")
                .bind(&op.op_id)
                .bind(&request.command_id)
                .execute(&mut *tx)
                .await?;
        }
        tx.commit().await?;
        Ok(receipt)
    }

    async fn apply_canvas_operation(
        &self,
        tx: &mut Transaction<'_, Sqlite>,
        request: &CanvasCommandRequest,
        op: &CanvasOperation,
    ) -> Result<(Value, bool)> {
        let p = &request.project_id;
        let id = Some(op.op_id.as_str());
        let cursor: i64 = sqlx::query_scalar("SELECT COALESCE(MAX(rowid),0) FROM canvas_op_log")
            .fetch_one(&mut **tx)
            .await?;
        let result = match &op.action {
            CanvasAction::ApplyEditorialChanges(cmd) => {
                if cmd.selection.expected_target_revision != request.base_revision {
                    return Err(invalid("预览与当前画布不一致，请重新比较"));
                }
                self.apply_editorial_tx(tx, p, &request.board_id, cmd)
                    .await?;
                Value::Null
            }
            CanvasAction::CreateBoard(cmd) => serde_json::to_value(
                self.create_board_tx(tx, p, &cmd.title, Some(&request.board_id))
                    .await?,
            )
            .map_err(serialization)?,
            CanvasAction::PlacePhoto(cmd) => {
                serde_json::to_value(self.place_photo_node_tx(tx, p, cmd, id).await?)
                    .map_err(serialization)?
            }
            CanvasAction::MoveNodes(cmd) => {
                let bounds:bool=sqlx::query_scalar("SELECT EXISTS(SELECT 1 FROM canvas_nodes WHERE board_id=? AND id IN (SELECT value FROM json_each(?)) AND (ABS(x+?)>1000000000 OR ABS(y+?)>1000000000))")
                    .bind(&request.board_id).bind(encode(&cmd.node_ids)?).bind(cmd.dx).bind(cmd.dy).fetch_one(&mut **tx).await?;
                if bounds {
                    return Err(invalid("节点位置超出画布范围"));
                }
                self.move_nodes_tx(tx, p, cmd).await?;
                Value::Null
            }
            CanvasAction::AddText(cmd) => {
                serde_json::to_value(self.add_text_node_tx(tx, p, cmd, id).await?)
                    .map_err(serialization)?
            }
            CanvasAction::DeleteNode(cmd) => {
                self.delete_node_tx(tx, p, cmd).await?;
                Value::Null
            }
            CanvasAction::AddAnnotation(cmd) => {
                serde_json::to_value(self.add_annotation_tx(tx, p, cmd, id).await?)
                    .map_err(serialization)?
            }
            CanvasAction::ResolveAnnotation(cmd) => {
                let same_board:bool=sqlx::query_scalar("SELECT EXISTS(SELECT 1 FROM board_annotations WHERE id=? AND board_id=? AND visible=1)")
                    .bind(&cmd.annotation_id).bind(&request.board_id).fetch_one(&mut **tx).await?;
                if !same_board {
                    return Err(AppError::new("NOT_FOUND", "评论不属于此画布"));
                }
                self.resolve_annotation_tx(tx, p, cmd).await?;
                Value::Null
            }
            CanvasAction::AddArrow(cmd) => {
                serde_json::to_value(self.add_arrow_tx(tx, p, cmd, id).await?)
                    .map_err(serialization)?
            }
            CanvasAction::AddInk(cmd) => {
                serde_json::to_value(self.add_ink_tx(tx, p, cmd, id).await?)
                    .map_err(serialization)?
            }
            CanvasAction::GroupNodes(cmd) => {
                serde_json::to_value(self.group_nodes_cmd_tx(tx, p, cmd, id).await?)
                    .map_err(serialization)?
            }
            CanvasAction::ResizeNode(cmd) => {
                self.resize_node_tx(tx, p, cmd).await?;
                Value::Null
            }
            CanvasAction::SetNodeLocked(cmd) => {
                self.set_node_locked_tx(tx, p, cmd).await?;
                Value::Null
            }
            CanvasAction::Undo(cmd) => {
                let changed = self.undo_canvas_tx(tx, p, cmd).await?;
                return Ok((Value::Bool(changed), changed));
            }
            CanvasAction::Redo(cmd) => {
                let changed = self.redo_canvas_tx(tx, p, cmd).await?;
                return Ok((Value::Bool(changed), changed));
            }
            CanvasAction::AdoptBoard(cmd) => {
                self.adopt_board_draft_tx(tx, p, cmd).await?;
                sqlx::query("UPDATE boards SET state='official' WHERE id=?")
                    .bind(&request.board_id)
                    .execute(&mut **tx)
                    .await?;
                return Ok((Value::Null, true));
            }
        };
        let changed = matches!(op.action, CanvasAction::CreateBoard(_))
            || sqlx::query_scalar::<_, bool>(
                "SELECT EXISTS(SELECT 1 FROM canvas_op_log WHERE rowid>? AND board_id=?)",
            )
            .bind(cursor)
            .bind(&request.board_id)
            .fetch_one(&mut **tx)
            .await?;
        Ok((result, changed))
    }
}

#[cfg(test)]
#[path = "canvas_command_tests.rs"]
mod tests;
