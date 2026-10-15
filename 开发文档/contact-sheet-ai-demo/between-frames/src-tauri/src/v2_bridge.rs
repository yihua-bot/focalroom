// v2.0 M2 执行桥：local-domain 纯鉴权 → v1 CommandHandler 真实执行。
// 可执行切片：photo.set_selection、sequence.move_item/add_item/remove_item。
// proposal.*/comment.* 属 M5 审批与评论，保留 NOT_IMPLEMENTED（不假装成功）。
use crate::{
    command_handler::{CommandHandler, GuiCommandRequest},
    commands_generated::GuiCommand,
    db::Store,
    domain::Result,
};
use betweenframes_local_domain::{
    Actor, AuthorityTarget, CommandKind, Principal, ProjectAuthority, ValidatedCommand,
};
use serde_json::{Value, json};

pub struct V2Execution {
    pub envelope: ValidatedCommand,
    pub principal: Principal,
    pub authority: ProjectAuthority,
}

fn reply(ok: bool, command_id: &str, revision: Option<i64>, code: &str, message: &str) -> Value {
    json!({
        "ok": ok,
        "command_id": command_id,
        "revision": revision,
        "error": if ok { Value::Null } else { json!({"code": code, "message": message}) },
    })
}

fn envelope_parts(envelope: &ValidatedCommand) -> (&str, &str, &str, i64, &CommandKind, &Value) {
    let inner = envelope.envelope();
    (
        &inner.command_id,
        &inner.project_id,
        &inner.idempotency_key,
        inner.expected_revision,
        &inner.kind,
        &inner.payload,
    )
}

pub async fn execute(store: &Store, execution: V2Execution, now: u64) -> Value {
    let V2Execution {
        envelope,
        principal,
        authority,
    } = execution;
    let (command_id, project_id, idempotency_key, expected_revision, kind, payload) =
        envelope_parts(&envelope);
    // 1) 纯函数层：权限/权威先拒；REVISION_CONFLICT 特例——交由 v1 内部幂等裁决，
    //    同命令重放应返回原回执而不是 409（v1 幂等命中先于版本检查）。
    if let Err(code) = betweenframes_local_domain::authorize(
        envelope.envelope(),
        &principal,
        &authority,
        AuthorityTarget::Local,
        now,
    ) && code != "REVISION_CONFLICT"
    {
        return reply(
            false,
            command_id,
            Some(authority.revision),
            code,
            "命令未获执行：权限、项目模式或版本校验失败",
        );
    }
    // 2) 可执行映射；未列入本切片的命令诚实返回未实现。
    let gui = match kind {
        CommandKind::PhotoSetSelection => GuiCommand::SetPhotoDecision {
            photo_ids: payload["photo_ids"]
                .as_array()
                .map(|ids| {
                    ids.iter()
                        .filter_map(|v| v.as_str().map(String::from))
                        .collect()
                })
                .unwrap_or_default(),
            status: match payload["selection_state"].as_str() {
                Some("KEEP") => "keep".to_owned(),
                Some("REJECT") => "reject".to_owned(),
                _ => "undecided".to_owned(),
            },
        },
        CommandKind::SequenceMoveItem
        | CommandKind::SequenceAddItem
        | CommandKind::SequenceRemoveItem => {
            match sequence_command(store, project_id, expected_revision, kind, payload).await {
                Ok(gui) => gui,
                Err(message) => {
                    return reply(false, command_id, None, "VALIDATION_ERROR", &message);
                }
            }
        }
        CommandKind::ProposalCreate | CommandKind::ProposalSubmit | CommandKind::ProposalApply => {
            return reply(
                false,
                command_id,
                Some(authority.revision),
                "NOT_IMPLEMENTED",
                "v2.0 审批提案在 M5 交付；草案能力暂经既有 Agent 通道",
            );
        }
        CommandKind::CommentAdd => {
            let thread = crate::agent::comments::add(
                store,
                project_id,
                &betweenframes_local_domain::actor_label(&principal),
                payload["body"].as_str().unwrap_or(""),
                &payload["anchor"],
            )
            .await;
            return match thread {
                Ok(thread) => {
                    let mut value = reply(true, command_id, None, "", "");
                    value["thread_id"] = json!(thread.id);
                    value["state"] = json!(thread.state);
                    value
                }
                Err(error) => reply(false, command_id, None, &error.code, &error.message),
            };
        }
        CommandKind::CommentResolve => {
            let thread = crate::agent::comments::resolve(
                store,
                project_id,
                payload["thread_id"].as_str().unwrap_or(""),
                payload["state"]
                    .as_str()
                    .is_some_and(|state| state.eq_ignore_ascii_case("resolved")),
            )
            .await;
            return match thread {
                Ok(thread) => {
                    let mut value = reply(true, command_id, None, "", "");
                    value["thread_id"] = json!(thread.id);
                    value["state"] = json!(thread.state);
                    value
                }
                Err(error) => reply(false, command_id, None, &error.code, &error.message),
            };
        }
    };
    // 3) 经唯一 CommandHandler 执行（幂等回执、历史、undo、证据全部继承 v1 语义）。
    let request = GuiCommandRequest {
        command_id: command_id.to_owned(),
        project_id: project_id.to_owned(),
        expected_project_revision: expected_revision,
        causation_id: None,
        idempotency_key: idempotency_key.to_owned(),
        command: gui,
    };
    match CommandHandler::new(store).execute_gui(request).await {
        Ok(receipt) => reply(true, command_id, Some(receipt.revision), "", ""),
        Err(error) => reply(
            false,
            command_id,
            error.current_revision,
            &error.code,
            &error.message,
        ),
    }
}

// v2 序列命令 → v1 语义：序列 = 全部 keep 照片的有序集。
// move：目标已在序列内则重排；add：先 keep 再落位；remove：置为 undecided（移出序列，评分备注保留）。
async fn sequence_command(
    store: &Store,
    project_id: &str,
    revision: i64,
    kind: &CommandKind,
    payload: &Value,
) -> std::result::Result<GuiCommand, String> {
    let project = store
        .get(project_id)
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
    let project = store.get(project_id).await?;
    Ok(ProjectAuthority {
        workspace_id: "local-workspace".into(),
        project_id: project.id,
        mode: betweenframes_local_domain::ProjectMode::LOCAL,
        revision: project.revision,
        local_state: betweenframes_local_domain::LocalState::LOCAL,
    })
}
