//! v2.0 M0 的纯契约与权限基座；不连接数据库、不替代 v1 正式命令通道。
mod kind_generated;
pub mod schema;
pub use kind_generated::CommandKind;
use serde::{Deserialize, Serialize};
use serde_json::{Value, json};
use std::{collections::HashSet, sync::LazyLock};

static COMMAND_SCHEMA: LazyLock<Value> = LazyLock::new(|| {
    serde_json::from_str(include_str!(
        "../../../packages/contracts/generated/command-envelope.schema.json"
    ))
    .expect("生成的 v2 命令契约必须可解析")
});

#[derive(Debug, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct CommandEnvelope {
    pub version: String,
    pub command_id: String,
    pub workspace_id: String,
    pub project_id: String,
    pub expected_revision: i64,
    pub idempotency_key: String,
    pub kind: CommandKind,
    pub payload: Value,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub client_trace_id: Option<String>,
}
#[derive(Debug)]
pub struct ValidatedCommand(CommandEnvelope);

pub fn parse_command(mut value: Value) -> Result<ValidatedCommand, &'static str> {
    normalize_wire_numbers(&mut value);
    if !schema::valid(&COMMAND_SCHEMA, &value) {
        return Err("VALIDATION_ERROR");
    }
    let command: CommandEnvelope = serde_json::from_value(value).map_err(|_| "VALIDATION_ERROR")?;
    let payload = &command.payload;
    let unique = |values: &Value| {
        let values = values.as_array().expect("数组已经过 Schema 校验");
        values
            .iter()
            .map(Value::to_string)
            .collect::<HashSet<_>>()
            .len()
            == values.len()
    };
    if command.kind == CommandKind::PhotoSetSelection && !unique(&payload["photo_ids"])
        || command.kind == CommandKind::ProposalApply && !unique(&payload["accepted_operation_ids"])
    {
        return Err("VALIDATION_ERROR");
    }
    if command.kind == CommandKind::ProposalCreate {
        let operations = payload["operations"].as_array().unwrap();
        if operations
            .iter()
            .map(|item| &item["operation_id"])
            .collect::<HashSet<_>>()
            .len()
            != operations.len()
            || payload["base_revision"].as_i64() != Some(command.expected_revision)
        {
            return Err("VALIDATION_ERROR");
        }
    }
    Ok(ValidatedCommand(command))
}

// JSON 中 3 与 3.0 在 JS 中相同；只规范化可安全表示的整数，不改变 v1 校验。
fn normalize_wire_numbers(value: &mut Value) {
    match value {
        Value::Number(number) if number.is_f64() => {
            if let Some(float) = number.as_f64()
                && float.fract() == 0.0
                && float.abs() <= 9_007_199_254_740_991.0
            {
                *value = Value::from(float as i64);
            }
        }
        Value::Array(values) => values.iter_mut().for_each(normalize_wire_numbers),
        Value::Object(values) => values.values_mut().for_each(normalize_wire_numbers),
        _ => {}
    }
}

#[derive(Debug, Clone, Copy, Deserialize, PartialEq, Eq)]
pub enum Role {
    OWNER,
    ADMIN,
    EDITOR,
    REVIEWER,
    VIEWER,
}
#[derive(Debug, Clone, Copy, Deserialize, PartialEq, Eq)]
pub enum ProjectMode {
    LOCAL,
    SHARED,
}
#[derive(Debug, Clone, Copy, Deserialize, PartialEq, Eq)]
#[allow(non_camel_case_types)]
pub enum LocalState {
    LOCAL,
    PROMOTING,
    SHARED_CACHE,
}
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AuthorityTarget {
    Local,
    Cloud,
}
#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ProjectAuthority {
    pub workspace_id: String,
    pub project_id: String,
    pub mode: ProjectMode,
    pub local_state: LocalState,
    pub revision: i64,
}
// 不实现 Deserialize：Actor 必须由可信入口/session/task 颁发。
#[derive(Debug)]
pub enum Actor {
    User {
        id: String,
        role: Role,
        can_approve: bool,
        can_edit_formal: bool,
        can_propose: bool,
    },
    Agent {
        id: String,
        expires_at_ms: u64,
    },
}
#[derive(Debug)]
pub struct Principal {
    pub workspace_id: String,
    pub project_id: String,
    pub actor: Actor,
}

fn authorize(
    command: &CommandEnvelope,
    principal: &Principal,
    project: &ProjectAuthority,
    target: AuthorityTarget,
    now: u64,
) -> Result<(), &'static str> {
    if command.workspace_id != project.workspace_id
        || command.project_id != project.project_id
        || principal.workspace_id != project.workspace_id
        || principal.project_id != project.project_id
    {
        return Err("PERMISSION_DENIED");
    }
    use CommandKind::*;
    match &principal.actor {
        Actor::Agent { expires_at_ms, .. } => {
            if *expires_at_ms <= now {
                return Err("AUTH_REVOKED");
            }
            if command.kind != ProposalCreate {
                return Err("PERMISSION_DENIED");
            }
        }
        Actor::User {
            role,
            can_approve,
            can_edit_formal,
            can_propose,
            ..
        } => {
            let owner = matches!(role, Role::OWNER | Role::ADMIN);
            let allowed = match command.kind {
                ProposalApply => owner || *role == Role::EDITOR && *can_approve,
                PhotoSetSelection | SequenceMoveItem | SequenceAddItem | SequenceRemoveItem => {
                    owner || *role == Role::EDITOR && *can_edit_formal
                }
                ProposalCreate | ProposalSubmit => owner || *can_propose,
                CommentAdd | CommentResolve => true,
            };
            if *role == Role::VIEWER || !allowed {
                return Err("PERMISSION_DENIED");
            }
        }
    }
    if target == AuthorityTarget::Local
        && (project.mode != ProjectMode::LOCAL || project.local_state != LocalState::LOCAL)
        || target == AuthorityTarget::Cloud && project.mode != ProjectMode::SHARED
    {
        return Err("AUTHORITY_MISMATCH");
    }
    if command.expected_revision != project.revision {
        return Err("REVISION_CONFLICT");
    }
    Ok(())
}

pub fn dispatch(
    input: Value,
    principal: &Principal,
    project: &ProjectAuthority,
    target: AuthorityTarget,
    now: u64,
) -> Value {
    let command = match parse_command(input) {
        Ok(command) => command,
        Err(code) => {
            return json!({"ok":false,"command_id":"","error":{"code":code,"message":"命令不符合 v2.0 契约"}});
        }
    };
    let code = authorize(&command.0, principal, project, target, now)
        .err()
        .unwrap_or("NOT_IMPLEMENTED");
    let message = if code == "NOT_IMPLEMENTED" {
        "v2.0 写入入口尚未开放"
    } else {
        "命令未获执行，请检查权限、项目模式或版本"
    };
    json!({"ok":false,"command_id":command.0.command_id,"error":{"code":code,"message":message}})
}
