// v2 命令桥：权限、权威、业务写入与持久化回执在同一个写事务内裁决。
use crate::{
    command_handler::{CommandContext, CommandHandler, GuiCommandRequest},
    commands_generated::GuiCommand,
    db::Store,
    domain::{AppError, Result},
};
use betweenframes_local_domain::{
    Actor, AuthorityTarget, CommandKind, Principal, ProjectAuthority, ValidatedCommand,
};
use serde_json::{Value, json};
use sqlx::{Row, Sqlite, Transaction};

pub struct V2Execution {
    pub envelope: ValidatedCommand,
    pub principal: Principal,
    pub authority: ProjectAuthority,
}
fn reply(ok: bool, command_id: &str, revision: Option<i64>, code: &str, message: &str) -> Value {
    json!({"ok":ok,"command_id":command_id,"revision":revision,"error":if ok {Value::Null} else {json!({"code":code,"message":message})}})
}
pub async fn execute(store: &Store, execution: V2Execution, now: u64) -> Value {
    let id = execution.envelope.envelope().command_id.clone();
    let result = async {
        let mut tx = store.pool.begin_with("BEGIN IMMEDIATE").await?;
        let value = execute_tx(store, &mut tx, &execution, now).await?;
        tx.commit().await?;
        Ok::<_, AppError>(value)
    }
    .await;
    match result {
        Ok(value) => value,
        Err(error) => reply(
            false,
            &id,
            error.current_revision,
            &error.code,
            &error.message,
        ),
    }
}
async fn execute_tx(
    store: &Store,
    tx: &mut Transaction<'_, Sqlite>,
    execution: &V2Execution,
    now: u64,
) -> Result<Value> {
    let envelope = execution.envelope.envelope();
    let project_id = &envelope.project_id;
    // 调用者快照只能收紧权限；锁内重读才是写入的权威依据。
    let authority = authority_for_tx(tx, project_id).await?;
    for snapshot in [&execution.authority, &authority] {
        if let Err(code) = betweenframes_local_domain::authorize(
            envelope,
            &execution.principal,
            snapshot,
            AuthorityTarget::Local,
            now,
        ) && code != "REVISION_CONFLICT"
        {
            return Err(AppError::new(code, "命令未获执行：权限或项目权威校验失败"));
        }
    }
    let fingerprint = crate::command_audit::fingerprint(
        &json!({"envelope":envelope,"actor":betweenframes_local_domain::actor_label(&execution.principal)}),
    );
    let receipts = sqlx::query("SELECT request_fingerprint,result_json FROM v2_command_receipts WHERE command_id=? OR idempotency_key=?")
        .bind(&envelope.command_id).bind(&envelope.idempotency_key).fetch_all(&mut **tx).await?;
    if !receipts.is_empty() {
        if receipts.len() != 1 || receipts[0].get::<String, _>("request_fingerprint") != fingerprint
        {
            return Err(AppError::new(
                "IDEMPOTENCY_CONFLICT",
                "命令 ID 或幂等键已用于不同请求",
            ));
        }
        return serde_json::from_str(&receipts[0].get::<String, _>("result_json"))
            .map_err(crate::command_audit::serialization);
    }
    if envelope.kind == CommandKind::ProposalApply
        && envelope.expected_revision != authority.revision
    {
        let mut error = AppError::new(
            "REVISION_CONFLICT",
            "项目已更新；请刷新并基于新版本重建提案",
        );
        error.current_revision = Some(authority.revision);
        return Err(error);
    }
    // 旧 v1 正式命令回执仍由 CommandHandler 处理，支持升级前的安全重放。
    let metadata = matches!(
        envelope.kind,
        CommandKind::ProposalCreate
            | CommandKind::ProposalSubmit
            | CommandKind::CommentAdd
            | CommandKind::CommentResolve
    );
    if metadata && envelope.expected_revision != authority.revision {
        let mut error = AppError::new("REVISION_CONFLICT", "项目版本已更新，请刷新后重试");
        error.current_revision = Some(authority.revision);
        return Err(error);
    }
    let payload = &envelope.payload;
    let mut result = reply(true, &envelope.command_id, Some(authority.revision), "", "");
    let gui = match &envelope.kind {
        CommandKind::ProposalCreate => {
            let proposal = crate::agent::proposals::create_tx(
                tx,
                project_id,
                &betweenframes_local_domain::actor_label(&execution.principal),
                None,
                payload["base_revision"].as_i64().unwrap_or(-1),
                payload["title"].as_str().unwrap_or(""),
                payload["operations"].clone(),
            )
            .await?;
            result["proposal_id"] = json!(proposal.id);
            result["proposal_revision"] = json!(proposal.revision);
            result["approval_nonce"] = json!(proposal.approval_nonce);
            None
        }
        CommandKind::ProposalSubmit => {
            let proposal = crate::agent::proposals::submit_tx(
                tx,
                project_id,
                payload["proposal_id"].as_str().unwrap_or(""),
                payload["expected_proposal_revision"].as_i64().unwrap_or(-1),
            )
            .await?;
            result["proposal_id"] = json!(proposal.id);
            result["proposal_revision"] = json!(proposal.revision);
            result["state"] = json!(proposal.state);
            None
        }
        CommandKind::ProposalApply => Some(GuiCommand::ApplyProposal {
            proposal_id: payload["proposal_id"].as_str().unwrap_or("").into(),
            expected_proposal_revision: payload["expected_proposal_revision"]
                .as_i64()
                .unwrap_or(-1),
            accepted_operation_ids: serde_json::from_value(
                payload["accepted_operation_ids"].clone(),
            )
            .map_err(crate::command_audit::serialization)?,
            approval_nonce: payload["approval_nonce"].as_str().unwrap_or("").into(),
        }),
        CommandKind::CommentAdd => {
            let thread = crate::agent::comments::add_tx(
                tx,
                project_id,
                &betweenframes_local_domain::actor_label(&execution.principal),
                payload["body"].as_str().unwrap_or(""),
                &payload["anchor"],
            )
            .await?;
            result["thread_id"] = json!(thread.id);
            result["state"] = json!(thread.state);
            None
        }
        CommandKind::CommentResolve => {
            let thread = crate::agent::comments::resolve_tx(
                tx,
                project_id,
                payload["thread_id"].as_str().unwrap_or(""),
                payload["state"] == "RESOLVED",
            )
            .await?;
            result["thread_id"] = json!(thread.id);
            result["state"] = json!(thread.state);
            None
        }
        CommandKind::PhotoSetSelection => Some(GuiCommand::SetPhotoDecision {
            photo_ids: serde_json::from_value(payload["photo_ids"].clone())
                .map_err(crate::command_audit::serialization)?,
            status: match payload["selection_state"].as_str() {
                Some("KEEP") => "keep",
                Some("REJECT") => "reject",
                _ => "undecided",
            }
            .into(),
        }),
        kind => Some(
            sequence_command(tx, project_id, envelope.expected_revision, kind, payload)
                .await
                .map_err(|message| AppError::new("VALIDATION_ERROR", message))?,
        ),
    };
    if let Some(gui) = gui {
        let request = GuiCommandRequest {
            command_id: envelope.command_id.clone(),
            project_id: project_id.clone(),
            expected_project_revision: envelope.expected_revision,
            causation_id: None,
            idempotency_key: envelope.idempotency_key.clone(),
            command: gui,
        };
        let receipt = CommandHandler::new(store)
            .execute_tx(tx, request, &CommandContext::human(project_id))
            .await?;
        result["revision"] = json!(receipt.revision);
        if envelope.kind == CommandKind::ProposalApply {
            let accepted = payload["accepted_operation_ids"].as_array().unwrap();
            result["outcome"] = json!({"applied":accepted.len(),"failed":0,"discarded":accepted.is_empty(),"results":accepted.iter().map(|id|json!({"operation_id":id,"ok":true,"revision":receipt.revision})).collect::<Vec<_>>()});
        }
    }
    sqlx::query("INSERT INTO v2_command_receipts(command_id,idempotency_key,project_id,request_fingerprint,result_json,created_at) VALUES(?,?,?,?,?,?)")
        .bind(&envelope.command_id).bind(&envelope.idempotency_key).bind(project_id).bind(fingerprint).bind(result.to_string()).bind(crate::domain::timestamp()).execute(&mut **tx).await?;
    Ok(result)
}

async fn sequence_command(
    tx: &mut sqlx::Transaction<'_, sqlx::Sqlite>,
    project_id: &str,
    revision: i64,
    kind: &CommandKind,
    payload: &Value,
) -> std::result::Result<GuiCommand, String> {
    let project = crate::db::snapshot_tx(tx, project_id, None)
        .await
        .map_err(|_| "项目不可读".to_owned())?;
    let sequence = project.sequence.clone();
    let _ = revision;
    match kind {
        CommandKind::SequenceMoveItem => {
            let item = payload["item_id"]
                .as_str()
                .ok_or("缺少 item_id")?
                .to_owned();
            let to_index = payload["to_index"].as_u64().ok_or("缺少 to_index")? as usize;
            let from = sequence
                .iter()
                .position(|id| *id == item)
                .ok_or("item 不在当前序列")?;
            if to_index >= sequence.len() {
                return Err("to_index 超出序列范围".into());
            }
            let mut next = sequence.clone();
            next.remove(from);
            next.insert(to_index, item);
            Ok(GuiCommand::ReorderSequence {
                ordered_photo_ids: next,
            })
        }
        CommandKind::SequenceAddItem => {
            let photo = payload["photo_id"]
                .as_str()
                .ok_or("缺少 photo_id")?
                .to_owned();
            let to_index = payload["to_index"].as_u64().ok_or("缺少 to_index")? as usize;
            if !project.photos.iter().any(|p| p.id == photo) {
                return Err("照片不属于项目".into());
            }
            if sequence.contains(&photo) {
                return Err("照片已在序列中（应使用 move_item）".into());
            }
            if to_index > sequence.len() {
                return Err("to_index 超出可插入范围".into());
            }
            // 先保留再落位：两步经调用方按序执行会引入两份命令——这里以单条 Reorder 表达
            // 需要照片先为 keep。桥层限制：add_item 仅对已 keep 照片重排落位，否则要求
            // 先 set_selection（M5 的组合命令再扩展）。
            Err("add_item 需先经 photo.set_selection 置为 keep；组合写入在 M5 扩展".into())
        }
        CommandKind::SequenceRemoveItem => {
            let item = payload["item_id"]
                .as_str()
                .ok_or("缺少 item_id")?
                .to_owned();
            if !sequence.contains(&item) {
                return Err("item 不在当前序列".into());
            }
            Ok(GuiCommand::SetPhotoDecision {
                photo_ids: vec![item],
                status: "undecided".into(),
            })
        }
        _ => unreachable!(),
    }
}

// Principal 便捷构造：单机 LOCAL 模式的本地所有者（可信入口注入，客户端不能自报）。
pub fn local_owner_principal(workspace_id: &str, project_id: &str) -> Principal {
    Principal {
        workspace_id: workspace_id.to_owned(),
        project_id: project_id.to_owned(),
        actor: Actor::User {
            id: "local-owner".into(),
            role: betweenframes_local_domain::Role::OWNER,
            can_approve: true,
            can_edit_formal: true,
            can_propose: true,
        },
    }
}

pub fn agent_principal(workspace_id: &str, project_id: &str, expires_at_ms: u64) -> Principal {
    Principal {
        workspace_id: workspace_id.to_owned(),
        project_id: project_id.to_owned(),
        actor: Actor::Agent {
            id: "agent".into(),
            expires_at_ms,
        },
    }
}

pub async fn authority_for(store: &Store, project_id: &str) -> Result<ProjectAuthority> {
    let mut tx = store.pool.begin().await?;
    let result = authority_for_tx(&mut tx, project_id).await?;
    tx.commit().await?;
    Ok(result)
}
async fn authority_for_tx(
    tx: &mut Transaction<'_, Sqlite>,
    project_id: &str,
) -> Result<ProjectAuthority> {
    let revision: i64 = sqlx::query_scalar("SELECT revision FROM projects WHERE id=?")
        .bind(project_id)
        .fetch_optional(&mut **tx)
        .await?
        .ok_or_else(|| AppError::new("NOT_FOUND", "项目不存在"))?;
    let state = crate::agent::promotion::local_state(tx, project_id).await?;
    Ok(ProjectAuthority {
        workspace_id: "local-workspace".into(),
        project_id: project_id.into(),
        revision,
        mode: if state == betweenframes_local_domain::LocalState::SHARED_CACHE {
            betweenframes_local_domain::ProjectMode::SHARED
        } else {
            betweenframes_local_domain::ProjectMode::LOCAL
        },
        local_state: state,
    })
}
