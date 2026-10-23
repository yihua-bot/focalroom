//! Focal AI 会话只投影既有执行记录；所有推理统一经过 AgentEngine。
use crate::{
    agent::{model_binding::ModelIdentity, types::Task},
    command_audit::{encode, fingerprint, serialization},
    domain::{self, AppError, Result},
};
use serde::{Deserialize, Serialize};
use serde_json::{Value, json};
use sqlx::{Row, Sqlite, Transaction};

#[cfg(test)]
#[path = "focal_chat_tests.rs"]
mod tests;

const PROMPT: &str = include_str!("../../prompts/focal-agent-v2.md");
const MAX_HISTORY_CHARS: usize = 12000;

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ConversationMessage {
    pub id: i64,
    pub role: String,
    pub content: String,
    pub created_at: String,
}
#[derive(Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub(crate) struct ConversationContext {
    owner: Option<String>,
    recipient: String,
    prompt_sha256: String,
    pub focus_photo_ids: Vec<String>,
    pub history: Vec<ConversationMessage>,
}
pub(crate) fn same_recipient(left: &ConversationContext, right: &ConversationContext) -> bool {
    left.owner == right.owner && left.recipient == right.recipient
}
pub(crate) fn owner(context: &ConversationContext) -> Option<&str> {
    context.owner.as_deref()
}

pub(crate) async fn load(
    tx: &mut Transaction<'_, Sqlite>,
    id: &str,
) -> Result<Option<ConversationContext>> {
    let raw: Option<String> = sqlx::query_scalar("SELECT body FROM task_messages WHERE task_id=? AND kind='conversation_context' ORDER BY id LIMIT 1")
        .bind(id).fetch_optional(&mut **tx).await?;
    raw.map(|s| serde_json::from_str(&s).map_err(serialization))
        .transpose()
}
pub(crate) async fn binding(tx: &mut Transaction<'_, Sqlite>, id: &str) -> Result<Value> {
    Ok(match load(tx, id).await? {
        Some(value) => json!(fingerprint(
            &serde_json::to_value(value).map_err(serialization)?
        )),
        None => Value::Null,
    })
}
pub(crate) async fn messages(
    tx: &mut Transaction<'_, Sqlite>,
    id: &str,
) -> Result<Vec<ConversationMessage>> {
    let rows = sqlx::query("SELECT id,kind,body,created_at FROM task_messages WHERE task_id=? AND kind IN ('user_goal','clarification_answer','clarification_question','conversation_assistant') ORDER BY id")
        .bind(id).fetch_all(&mut **tx).await?;
    Ok(rows
        .iter()
        .map(|r| ConversationMessage {
            id: r.get("id"),
            role: if matches!(
                r.get::<&str, _>("kind"),
                "user_goal" | "clarification_answer"
            ) {
                "user"
            } else {
                "assistant"
            }
            .into(),
            content: r.get("body"),
            created_at: r.get("created_at"),
        })
        .collect())
}
/// 历史只取同账户、同收件服务且旧照片范围仍被本次授权覆盖的会话；客户端不传历史。
pub(crate) async fn bind(
    tx: &mut Transaction<'_, Sqlite>,
    task: &Task,
    identity: Option<&ModelIdentity>,
    focus: &[String],
) -> Result<()> {
    if focus.iter().any(|id| !task.allowed_photo_ids.contains(id))
        || focus.iter().collect::<std::collections::HashSet<_>>().len() != focus.len()
    {
        return Err(AppError::new(
            "PERMISSION_DENIED",
            "当前照片不在本次讨论范围内",
        ));
    }
    if let Some(baseline) = crate::agent::task_scope::baseline(tx, task).await? {
        for asset in baseline.nodes.iter().filter_map(|n| n.asset_id.as_ref()) {
            let permitted: bool = sqlx::query_scalar("SELECT EXISTS(SELECT 1 FROM asset_aliases WHERE asset_id=? AND photo_id IN (SELECT value FROM json_each(?)))")
                .bind(asset).bind(encode(&task.allowed_photo_ids)?).fetch_one(&mut **tx).await?;
            if !permitted {
                return Err(AppError::new(
                    "PERMISSION_DENIED",
                    "画布包含本次范围外的照片，请调整照片范围或画布选区",
                ));
            }
        }
    }
    let owner = identity.and_then(|i| i.account_id.clone());
    let recipient = fingerprint(&identity.map_or_else(
        || json!({"provider": task.provider_id}),
        |i| serde_json::to_value(i).unwrap(),
    ));
    let ids: Vec<String> = sqlx::query_scalar("SELECT id FROM agent_tasks WHERE project_id=? AND id<>? AND state IN ('completed','review_ready','stale') ORDER BY created_at DESC,rowid DESC LIMIT 100")
        .bind(&task.project_id).bind(&task.id).fetch_all(&mut **tx).await?;
    let mut history = Vec::new();
    let mut chars = 0;
    for id in ids {
        // 未绑定真实服务的非离线任务不能推断收件方相同，也就不能复用历史。
        if identity.is_none() && task.provider_id != crate::agent::PROVIDER_ID {
            break;
        }
        let Some(previous) = load(tx, &id).await? else {
            continue;
        };
        if previous.owner != owner || previous.recipient != recipient {
            continue;
        }
        let prior = crate::agent::store::load(tx, &id).await?;
        if prior.memory_revision != task.memory_revision {
            continue;
        }
        if prior
            .allowed_photo_ids
            .iter()
            .any(|id| !task.allowed_photo_ids.contains(id))
        {
            continue;
        }
        let turns = messages(tx, &id).await?;
        let size: usize = turns.iter().map(|m| m.content.chars().count()).sum();
        if history.len() + turns.len() > 24 || chars + size > MAX_HISTORY_CHARS {
            break;
        }
        chars += size;
        history.splice(0..0, turns);
    }
    let context = ConversationContext {
        owner,
        recipient,
        prompt_sha256: fingerprint(&json!(PROMPT)),
        focus_photo_ids: focus.to_vec(),
        history,
    };
    sqlx::query("INSERT INTO task_messages(task_id,kind,body,created_at) VALUES(?,'conversation_context',?,?)")
        .bind(&task.id).bind(encode(&context)?).bind(domain::timestamp()).execute(&mut **tx).await?;
    Ok(())
}
pub(crate) fn prompt(context: &ConversationContext) -> Result<&'static str> {
    if context.prompt_sha256 != fingerprint(&json!(PROMPT)) {
        return Err(AppError::new(
            "RECOVERY_ERROR",
            "对话能力已更新，请重新发送要求",
        ));
    }
    Ok(PROMPT)
}
pub(crate) async fn assistant(
    tx: &mut Transaction<'_, Sqlite>,
    id: &str,
    text: &str,
) -> Result<()> {
    if load(tx, id).await?.is_some() {
        sqlx::query("INSERT INTO task_messages(task_id,kind,body,created_at) VALUES(?,'conversation_assistant',?,?)")
            .bind(id).bind(text).bind(domain::timestamp()).execute(&mut **tx).await?;
    }
    Ok(())
}

pub(crate) async fn inspected(tx: &mut Transaction<'_, Sqlite>, task: &Task) -> Result<Vec<Value>> {
    let rows:Vec<String>=sqlx::query_scalar("SELECT result_json FROM tool_calls WHERE task_id=? AND tool_name IN ('inspect_photos','compare_photos') ORDER BY rowid")
        .bind(&task.id).fetch_all(&mut **tx).await?;
    let mut found = std::collections::BTreeMap::new();
    for raw in rows {
        let receipt: crate::agent::types::ToolResult =
            serde_json::from_str(&raw).map_err(serialization)?;
        if !receipt.ok {
            continue;
        }
        if let Some(result) = receipt.result.filter(|v| v["mode"] == "vision_observed") {
            for photo in result["observations"].as_array().into_iter().flatten() {
                if let Some(id) = photo["photo_id"]
                    .as_str()
                    .filter(|id| task.allowed_photo_ids.iter().any(|p| p == *id))
                {
                    found.insert(id.to_string(), photo.clone());
                }
            }
        }
    }
    Ok(found.into_values().take(100).collect())
}

/// 工具可见性随真实草案状态展开；授权始终由冻结 TaskGrant 决定，不能由模型升级。
pub(crate) fn advertised_tools(
    mut specs: Value,
    selection: bool,
    canvas: bool,
    source: bool,
) -> Value {
    if let Some(tools) = specs.as_array_mut() {
        tools.retain(|tool| match tool["name"].as_str().unwrap_or("") {
            "update_draft_sequence" | "evaluate_draft" | "finish_task" => selection,
            "get_board" => canvas || source,
            "create_board_draft" => true,
            name if crate::agent::task_scope::CANVAS_TOOLS.contains(&name) => canvas,
            _ => true,
        });
    }
    specs
}
