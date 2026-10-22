use crate::{
    command_audit::{
        bind_aliases, compensated_group, encode, fingerprint, legacy_event, legacy_kind,
        serialization,
    },
    commands_generated::GuiCommand,
    db::{self, Store},
    domain::{self, AppError, Project, Result},
    edit_state::capture,
};
use serde::{Deserialize, Serialize};
use serde_json::json;
use sqlx::Row;

pub use crate::commands_generated::GuiCommandRequest;
impl GuiCommandRequest {
    pub fn new(project: &str, revision: i64, command: GuiCommand) -> Self {
        let id = domain::id();
        Self {
            command_id: id.clone(),
            project_id: project.into(),
            expected_project_revision: revision,
            causation_id: Some(id.clone()),
            idempotency_key: id,
            command,
        }
    }
}
// 身份与 scope 只能由 Rust 应用层构造，不能反序列化 GUI/模型自报身份。
pub struct CommandContext {
    project: String,
    human: bool,
}
impl CommandContext {
    pub(crate) fn permits_human(&self, project: &str) -> bool {
        self.human && self.project == project
    }
    pub fn human(project: &str) -> Self {
        Self {
            project: project.into(),
            human: true,
        }
    }
    pub fn read_only(project: &str) -> Self {
        Self {
            project: project.into(),
            human: false,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CommandReceipt {
    pub request_id: String,
    pub revision: i64,
    pub changed: bool,
    pub transaction_group_id: Option<String>,
    pub project: Option<Project>,
}
pub struct CommandHandler<'a> {
    pub(crate) store: &'a Store,
}
impl<'a> CommandHandler<'a> {
    pub fn new(store: &'a Store) -> Self {
        Self { store }
    }
    pub async fn execute_gui(&self, request: GuiCommandRequest) -> Result<CommandReceipt> {
        let context = CommandContext::human(&request.project_id);
        self.execute(request, &context).await
    }
    pub async fn execute(
        &self,
        request: GuiCommandRequest,
        context: &CommandContext,
    ) -> Result<CommandReceipt> {
        let mut tx = self.store.pool.begin_with("BEGIN IMMEDIATE").await?;
        let result = self.execute_tx(&mut tx, request, context).await?;
        tx.commit().await?;
        Ok(result)
    }
    // 共享事务入口：v2 审批/元数据回执与正式命令不可拆开提交。
    pub(crate) async fn execute_tx(
        &self,
        tx: &mut sqlx::Transaction<'_, sqlx::Sqlite>,
        request: GuiCommandRequest,
        context: &CommandContext,
    ) -> Result<CommandReceipt> {
        if !context.human || context.project != request.project_id {
            return Err(AppError::new(
                "PERMISSION_DENIED",
                "此通道没有修改该项目正式状态的权限",
            ));
        }
        validate_request(&request)?;
        let restoring = matches!(
            request.command,
            GuiCommand::UndoCommandGroup { .. }
                | GuiCommand::RedoCommandGroup { .. }
                | GuiCommand::RestoreSequenceVersion { .. }
                | GuiCommand::RestoreProject {}
        );
        let source = if restoring {
            "human_restore"
        } else {
            "gui_direct"
        };
        let kind = serde_json::to_value(&request.command).map_err(serialization)?["type"]
            .as_str()
            .unwrap_or("")
            .to_owned();
        let fingerprint = fingerprint(
            &json!({"projectId":request.project_id,"actorType":"human","source":source,"expectedProjectRevision":request.expected_project_revision,"causationId":request.causation_id,"command":request.command}),
        );
        let matches=sqlx::query("SELECT DISTINCT e.id,e.request_fingerprint,e.result_json FROM command_events e WHERE e.command_id=? OR e.idempotency_key=? OR EXISTS(SELECT 1 FROM command_receipt_aliases a WHERE a.event_id=e.id AND ((a.kind='command_id' AND a.value=?) OR (a.kind='idempotency_key' AND a.value=?)))")
            .bind(&request.command_id).bind(&request.idempotency_key).bind(&request.command_id).bind(&request.idempotency_key).fetch_all(&mut **tx).await?;
        if !matches.is_empty() {
            if matches.len() != 1
                || matches[0].get::<String, _>("request_fingerprint") != fingerprint
            {
                return Err(AppError::new(
                    "IDEMPOTENCY_CONFLICT",
                    "命令 ID 或幂等键已用于不同请求",
                ));
            }
            let stored: serde_json::Value =
                serde_json::from_str(&matches[0].get::<String, _>("result_json"))
                    .map_err(serialization)?;
            let mut result: CommandReceipt =
                serde_json::from_value(stored.clone()).map_err(serialization)?;
            if let Some(project) = &mut result.project {
                let cursor = if let Some(cursor) = stored.get("eventCursor") {
                    Some(
                        cursor
                            .as_i64()
                            .filter(|value| *value >= 0)
                            .ok_or_else(|| AppError::new("DATABASE", "回执历史游标无效"))?,
                    )
                } else if project.events.is_empty() {
                    // ADR 0026 的旧瘦回执没有游标；正式事件的 revision 只增不减，
                    // 据此恢复当时的截止位置。完整旧回执原样返回，不重查当前状态。
                    Some(sqlx::query_scalar::<_, Option<i64>>("SELECT MAX(rowid) FROM decision_events WHERE project_id=? AND json_valid(context_json) AND CAST(json_extract(context_json,'$.revision') AS INTEGER)<=?")
                        .bind(&project.id).bind(project.revision).fetch_one(&mut **tx).await?.unwrap_or(0))
                } else {
                    None
                };
                if let Some(cursor) = cursor {
                    project.events =
                        crate::command_audit::events_through(tx, &project.id, cursor).await?;
                }
            }
            // 别名必须绑定到匹配到的历史事件 ID（绑 command_id 会在新 ID 重放时 FK 失败）。
            let matched_event: String = matches[0].get("id");
            bind_aliases(tx, &request, &matched_event).await?;
            return Ok(result);
        }
        crate::agent::promotion::ensure_local_write(tx, &request.project_id).await?;
        let creating = matches!(request.command, GuiCommand::CreateProject { .. });
        let current = sqlx::query("SELECT revision,deleted_at FROM projects WHERE id=?")
            .bind(&request.project_id)
            .fetch_optional(&mut **tx)
            .await?;
        let revision = current
            .as_ref()
            .map(|row| row.get::<i64, _>("revision"))
            .unwrap_or(0);
        if (!creating && current.is_none()) || (creating && current.is_some()) {
            return Err(AppError::new(
                if creating {
                    "REVISION_CONFLICT"
                } else {
                    "NOT_FOUND"
                },
                "项目不存在或已创建",
            ));
        }
        if request.expected_project_revision != revision {
            let mut error = AppError::new(
                "REVISION_CONFLICT",
                "项目已在另一窗口更新，请重新加载后再操作",
            );
            error.current_revision = Some(revision);
            return Err(error);
        }
        let archived = current
            .as_ref()
            .is_some_and(|row| row.get::<Option<String>, _>("deleted_at").is_some());
        if archived
            && !matches!(
                request.command,
                GuiCommand::RestoreProject {}
                    | GuiCommand::UndoCommandGroup { .. }
                    | GuiCommand::RedoCommandGroup { .. }
            )
        {
            return Err(AppError::new("NOT_FOUND", "项目已归档"));
        }
        let now = domain::timestamp();
        let before = if creating {
            None
        } else {
            Some(capture(tx, &request.project_id).await?)
        };
        let event_id = domain::id();
        sqlx::query("SAVEPOINT command_projection")
            .execute(&mut **tx)
            .await?;
        let crate::edit_commands::EditEffect {
            undoable,
            force,
            history_operation,
        } = crate::edit_commands::apply_gui(tx, &request, &before, revision, &now, archived)
            .await?;
        let after = capture(tx, &request.project_id).await?;
        let changed = force || before.as_ref() != Some(&after);
        // 无操作回滚临时 timestamp 更新，但仍保存可安全重放的命令回执。
        if !changed && !matches!(request.command, GuiCommand::ApplyProposal { .. }) {
            sqlx::query("ROLLBACK TO command_projection")
                .execute(&mut **tx)
                .await?;
        }
        sqlx::query("RELEASE command_projection")
            .execute(&mut **tx)
            .await?;
        let new_revision = if creating {
            1
        } else {
            revision + i64::from(changed)
        };
        if changed && !creating {
            sqlx::query("UPDATE projects SET revision=?,updated_at=? WHERE id=?")
                .bind(new_revision)
                .bind(&now)
                .bind(&request.project_id)
                .execute(&mut **tx)
                .await?;
        }
        // L2 偏好证据：仅实际生效的人工命令（幂等回放不重复记录；撤销/重做不产生证据）。
        if changed {
            crate::agent::memory::record_from_command(tx, &request, &now).await?;
        }
        let group = if changed && undoable {
            Some(event_id.clone())
        } else {
            None
        };
        if let Some(group) = &group {
            let cursor: i64 =
                sqlx::query_scalar("SELECT cursor FROM history_heads WHERE project_id=?")
                    .bind(&request.project_id)
                    .fetch_optional(&mut **tx)
                    .await?
                    .unwrap_or(0);
            sqlx::query("UPDATE command_history SET active=0 WHERE project_id=? AND position>? AND active=1").bind(&request.project_id).bind(cursor).execute(&mut **tx).await?;
            sqlx::query("INSERT INTO command_history(project_id,position,group_id) VALUES(?,?,?)")
                .bind(&request.project_id)
                .bind(cursor + 1)
                .bind(group)
                .execute(&mut **tx)
                .await?;
            sqlx::query("INSERT INTO history_heads(project_id,cursor) VALUES(?,?) ON CONFLICT(project_id) DO UPDATE SET cursor=excluded.cursor").bind(&request.project_id).bind(cursor+1).execute(&mut **tx).await?;
        }
        if changed {
            let previous = before.as_ref().map(encode).transpose()?;
            let next = encode(&after)?;
            let reason = match &request.command {
                GuiCommand::SetPhotoNote { note, .. } => Some(note.as_str()),
                GuiCommand::RecordComparison { reason, .. } => Some(reason.as_str()),
                _ => None,
            };
            let photo = match &request.command {
                GuiCommand::SetPhotoNote { photo_id, .. }
                | GuiCommand::SetPhotoBookMark { photo_id, .. } => Some(photo_id.as_str()),
                GuiCommand::SetPhotoDecision { photo_ids, .. }
                | GuiCommand::SetPhotoRating { photo_ids, .. }
                    if photo_ids.len() == 1 =>
                {
                    Some(photo_ids[0].as_str())
                }
                _ => None,
            };
            legacy_event(tx,&request.project_id,photo,legacy_kind(&request.command),previous.as_deref(),&next,reason,"user",&json!({"revision":new_revision,"commandId":request.command_id,"source":source,"causationId":request.causation_id,"compensation":history_operation,"compensatesGroupId":compensated_group(&request.command)}).to_string()).await?;
        }
        let transaction_group = changed.then(|| event_id.clone());
        let result = CommandReceipt {
            request_id: request.command_id.clone(),
            revision: new_revision,
            changed,
            transaction_group_id: transaction_group.clone(),
            project: Some(db::snapshot_tx(tx, &request.project_id, None).await?),
        };
        let mut envelope = serde_json::to_value(&request).map_err(serialization)?;
        envelope["actorType"] = json!("human");
        envelope["source"] = json!(source);
        let event_cursor: i64 = sqlx::query_scalar(
            "SELECT COALESCE(MAX(rowid),0) FROM decision_events WHERE project_id=?",
        )
        .bind(&request.project_id)
        .fetch_one(&mut **tx)
        .await?;
        sqlx::query("INSERT INTO command_events VALUES(?,?,?,?,?,?,?,?,?,?,?,?,?,?,?,?,?,?)")
            .bind(&event_id)
            .bind(&request.project_id)
            .bind(&request.command_id)
            .bind(&request.idempotency_key)
            .bind(&fingerprint)
            .bind("human")
            .bind(source)
            .bind(kind)
            .bind(envelope.to_string())
            .bind(compensated_group(&request.command))
            .bind(&request.causation_id)
            .bind(transaction_group)
            .bind(
                before
                    .as_ref()
                    .map(encode)
                    .transpose()?
                    .unwrap_or_else(|| "null".into()),
            )
            .bind(encode(&after)?)
            .bind(encode(&stored_receipt(&result, event_cursor)?)?)
            .bind(revision)
            .bind(new_revision)
            .bind(&now)
            .execute(&mut **tx)
            .await?;
        bind_aliases(tx, &request, &event_id).await?;
        if changed {
            crate::agent::store::mark_stale(tx, &request.project_id, new_revision).await?;
        }
        Ok(result)
    }
}
fn validate_request(request: &GuiCommandRequest) -> Result<()> {
    for value in [
        &request.command_id,
        &request.project_id,
        &request.idempotency_key,
    ] {
        if value.trim().is_empty() || value.chars().count() > 160 {
            return Err(AppError::new("INVALID_INPUT", "命令标识需为 1–160 字"));
        }
    }
    if request
        .causation_id
        .as_ref()
        .is_some_and(|id| id.chars().count() > 160)
    {
        return Err(AppError::new("INVALID_INPUT", "因果标识过长"));
    }
    if request.expected_project_revision < 0 {
        return Err(AppError::new(
            "INVALID_INPUT",
            "正式命令需要非负预期 revision",
        ));
    }
    Ok(())
}

/// 仅持久化历史截止游标，业务快照不变；事件从 append-only 表按游标重建。
fn stored_receipt(receipt: &CommandReceipt, event_cursor: i64) -> Result<serde_json::Value> {
    let mut slim = receipt.clone();
    if let Some(project) = &mut slim.project {
        project.events = Vec::new();
    }
    let mut value = serde_json::to_value(slim).map_err(serialization)?;
    value["eventCursor"] = json!(event_cursor);
    Ok(value)
}
