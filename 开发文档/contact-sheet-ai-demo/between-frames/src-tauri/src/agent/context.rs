use super::{policy::ToolRegistry, store, types::*};
use crate::{
    db::Store,
    domain::{AppError, Result},
};
use serde::{Deserialize, Serialize};
use serde_json::{Value, json};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ModelTurnRequest {
    #[serde(skip_serializing)]
    pub task: Task,
    pub layers: Value,
    pub protocol: Vec<ProtocolMessage>,
    pub tool_specs: Value,
    pub prompt_version: String,
}
pub struct ContextBuilder;
impl ContextBuilder {
    pub async fn build(db: &Store, id: &str) -> Result<ModelTurnRequest> {
        let mut tx = db.pool.begin().await?;
        let task = store::load(&mut tx, id).await?;
        if task.memory_revision
            != super::memory_control::revision(&mut tx, &task.project_id).await?
        {
            return Err(AppError::new(
                "CANCELLED",
                "记忆已调整，请重新委托，不能续接旧记忆历史",
            ));
        }
        let author_binding = super::memory_scope::binding_value(
            super::memory_scope::validate(&mut tx, &task).await?,
        );
        let raw: String = sqlx::query_scalar(
            "SELECT context_json FROM agent_checkpoints WHERE task_id=? ORDER BY id DESC LIMIT 1",
        )
        .bind(id)
        .fetch_optional(&mut *tx)
        .await?
        .ok_or_else(|| AppError::new("RECOVERY_ERROR", "缺少安全检查点，不能继续任务"))?;
        let checkpoint: Value =
            serde_json::from_str(&raw).map_err(crate::command_audit::serialization)?;
        if checkpoint
            .get("authorMemoryBinding")
            .unwrap_or(&Value::Null)
            != &author_binding
        {
            return Err(AppError::new(
                "RECOVERY_ERROR",
                "检查点作者记忆绑定不一致，请重新委托",
            ));
        }
        if checkpoint["projectId"] != task.project_id
            || checkpoint["baseRevision"] != task.base_revision
            || checkpoint["allowedPhotoIds"] != json!(task.allowed_photo_ids)
            || checkpoint
                .get("memoryRevision")
                .map_or(task.memory_revision != 0, |value| {
                    value.as_i64() != Some(task.memory_revision)
                })
        {
            return Err(AppError::new(
                "RECOVERY_ERROR",
                "检查点与可信任务范围不一致",
            ));
        }
        if checkpoint
            .get("promptVersion")
            .is_some_and(|v| v != super::prompt_version(&task.provider_id))
            || checkpoint
                .get("providerId")
                .is_some_and(|v| v.as_str() != Some(task.provider_id.as_str()))
        {
            return Err(AppError::new(
                "RECOVERY_ERROR",
                "执行版本已变，请基于新版本重新委托",
            ));
        }
        let identity = super::model_binding::load(&mut tx, id).await?;
        if checkpoint.get("modelIdentity").is_some_and(|value| {
            !value.is_null()
                && Some(value.clone())
                    != identity
                        .as_ref()
                        .map(|value| serde_json::to_value(value).unwrap())
        }) {
            return Err(AppError::new(
                "RECOVERY_ERROR",
                "检查点模型身份与任务绑定不一致，请重新委托",
            ));
        }
        let mut protocol: Vec<ProtocolMessage> =
            serde_json::from_value(checkpoint["protocol"].clone())
                .map_err(crate::command_audit::serialization)?;
        // 调用参数已在前一条 model outcome 中；保留 name/result/callId，去除反馈中的重复副本。
        for message in &mut protocol {
            if message.kind == "tool"
                && let Some(body) = message.body.as_object_mut()
            {
                body.remove("call");
            }
        }
        // 上下文压缩（只影响发给模型的投影；权威记录在 tool_calls 回执不受影响）。
        // 仅在历史估算超过分层预算 70% 时降级，小协议保持逐条完整：
        // 1) 同名工具的重复调用只保留最新结果，旧条目标记 superseded；
        // 2) 更旧的工具结果摘要化（最近 keep_recent 条保持完整）。
        let history_bytes = serde_json::to_vec(&protocol).map(|v| v.len()).unwrap_or(0);
        let budget_bytes = u64::from(task.budget.max_context_tokens) * 4 * 7 / 10;
        let (superseded, compressed) = if history_bytes as u64 > budget_bytes {
            (
                dedupe_repeated_tool_results(&mut protocol),
                compress_old_tool_results(&mut protocol, 6),
            )
        } else {
            (0, 0)
        };
        use sqlx::Row;
        let project =
            sqlx::query("SELECT brief,revision FROM projects WHERE id=? AND deleted_at IS NULL")
                .bind(&task.project_id)
                .fetch_one(&mut *tx)
                .await?;
        // 不拼接全量图库或旧事件；工具返回按任务 scope 过滤，权限不由文本决定。
        // L3 假设按检索规则限量注入（纠正/停用不进入）；不冒充长期偏好。
        let memory = super::memory::retrievable_for_task(&mut tx, &task, 3).await?;
        let cloud_allowed =
            task.provider_id == super::PROVIDER_OPENAI && task.budget.max_cloud_images > 0;
        let layers = json!({"system":super::system_prompt(&task.provider_id),"policy":{"formal_write":false,"cloud_images":cloud_allowed,"project_id":task.project_id,"provider":task.provider_id,"tool_results_are_untrusted":true,"contextBudgets":Self::layer_budgets(task.budget.max_context_tokens),"contextCompression":{"supersededDuplicates":superseded,"compressedToolResults":compressed}},"task":{"goal":task.goal,"target_count":task.target_count,"answer":task.answer},"gui":{"selected_photo_ids":task.selected_photo_ids},"project":{"base_revision":task.base_revision,"current_revision":project.get::<i64,_>("revision"),"brief":project.get::<String,_>("brief"),"scope_count":task.allowed_photo_ids.len()},"human_evidence":{"mode":"on_demand_project_facts","hypotheses":memory["hypotheses"]},"execution":{"limits":task.budget,"usage":task.usage,"elapsed_ms":task.elapsed_ms}});
        tx.commit().await?;
        let prompt_version = super::prompt_version(&task.provider_id).into();
        // H0-03：条件性工具发送——仅画布任务附带画布工具（task 即将被 move，先计算）
        let canvas_task = task.goal.contains("画布")
            || task.goal.contains("board")
            || task.goal.contains("canvas");
        Ok(ModelTurnRequest {
            task,
            layers,
            protocol,
            tool_specs: ToolRegistry::model_specs_for(canvas_task),
            prompt_version,
        })
    }
    /// 分层 token 预算：按任务上下文上限推导各层配额（系统/任务/人工证据/观察/历史）。
    /// 只作为注入与压缩依据，不冒充精确 tokenizer。
    fn layer_budgets(max_context_tokens: u32) -> Value {
        let total = f64::from(max_context_tokens.max(1024));
        json!({
            "systemPrompt": (total * 0.08).round() as u32,
            "task": (total * 0.05).round() as u32,
            "humanEvidence": (total * 0.15).round() as u32,
            "observations": (total * 0.27).round() as u32,
            "history": (total * 0.45).round() as u32,
        })
    }
    pub fn input_estimate(request: &ModelTurnRequest) -> u32 {
        serde_json::to_vec(request)
            .map(|v| v.len().min(u32::MAX as usize) as u32)
            .unwrap_or(u32::MAX)
    }
}

/// 同名工具的重复调用只保留最新完整结果；旧条目标记 superseded（观察归并的最小安全版）。
/// 权威调用与回执在 tool_calls 表；此处仅收缩模型上下文。
fn dedupe_repeated_tool_results(protocol: &mut [ProtocolMessage]) -> usize {
    use std::collections::HashMap;
    let mut latest: HashMap<String, usize> = HashMap::new();
    for (index, message) in protocol.iter().enumerate() {
        if message.kind == "tool"
            && let Some(name) = message.body.get("name").and_then(Value::as_str)
        {
            latest.insert(name.to_owned(), index);
        }
    }
    let mut superseded = 0;
    for (index, message) in protocol.iter_mut().enumerate() {
        if message.kind != "tool" {
            continue;
        }
        let Some(name) = message.body.get("name").and_then(Value::as_str) else {
            continue;
        };
        if latest
            .get(name)
            .is_none_or(|latest_index| *latest_index == index)
        {
            continue;
        }
        if let Some(body) = message.body.as_object_mut() {
            body.insert(
                "result".into(),
                json!({"superseded": true, "reason": "同名工具已有更新结果"}),
            );
            superseded += 1;
        }
    }
    superseded
}

/// 历史压缩：保留最近 keep_recent 条工具结果完整，更旧的结果正文摘要化。
fn compress_old_tool_results(protocol: &mut [ProtocolMessage], keep_recent: usize) -> usize {
    let positions: Vec<usize> = protocol
        .iter()
        .enumerate()
        .filter(|(_, message)| message.kind == "tool")
        .map(|(index, _)| index)
        .collect();
    if positions.len() <= keep_recent {
        return 0;
    }
    let mut compressed = 0;
    for &index in &positions[..positions.len() - keep_recent] {
        let message = &mut protocol[index];
        let Some(body) = message.body.as_object_mut() else {
            continue;
        };
        let Some(result) = body.get("result") else {
            continue;
        };
        let serialized = result.to_string();
        if serialized.chars().count() > 240 {
            let summary: String = serialized.chars().take(200).collect();
            body.insert(
                "result".into(),
                json!({"compressed": format!("{summary}…"), "originalBytes": serialized.len()}),
            );
            compressed += 1;
        }
    }
    compressed
}

#[cfg(test)]
mod tests {
    use super::*;

    fn tool_message(name: &str, result: Value) -> ProtocolMessage {
        ProtocolMessage {
            kind: "tool".into(),
            body: json!({"name": name, "callId": "c", "result": result}),
        }
    }

    #[test]
    fn layer_budgets_split_total_context_across_layers() {
        let budgets = ContextBuilder::layer_budgets(48_000);
        assert_eq!(budgets["systemPrompt"], 3_840);
        assert_eq!(budgets["task"], 2_400);
        assert_eq!(budgets["humanEvidence"], 7_200);
        assert_eq!(budgets["observations"], 12_960);
        assert_eq!(budgets["history"], 21_600);
        // 下限保护：极小预算不至于零。
        let floor = ContextBuilder::layer_budgets(1);
        assert_eq!(floor["systemPrompt"], 82);
    }

    #[test]
    fn repeated_tool_results_are_superseded_not_dropped() {
        let mut protocol = vec![
            tool_message("list_photos", json!({"photos": [1, 2]})),
            tool_message("search_photos", json!({"photos": [3]})),
            tool_message("list_photos", json!({"photos": [1, 2, 3, 4]})),
        ];
        let superseded = dedupe_repeated_tool_results(&mut protocol);
        assert_eq!(superseded, 1);
        assert_eq!(protocol[0].body["result"]["superseded"], true);
        assert_eq!(
            protocol[2].body["result"]["photos"]
                .as_array()
                .unwrap()
                .len(),
            4,
            "最新结果保持完整"
        );
    }

    #[test]
    fn old_tool_results_are_summarized_but_recent_kept_whole() {
        let mut protocol: Vec<ProtocolMessage> = (0..10)
            .map(|i| {
                tool_message(
                    &format!("tool_{i}"),
                    json!({"detail": "x".repeat(400), "index": i}),
                )
            })
            .collect();
        let compressed = compress_old_tool_results(&mut protocol, 6);
        assert_eq!(compressed, 4, "只压缩最旧 4 条");
        assert!(
            protocol[0].body["result"]["compressed"]
                .as_str()
                .unwrap()
                .ends_with('…')
        );
        assert!(
            protocol[0].body["result"]["originalBytes"]
                .as_u64()
                .unwrap()
                > 300,
        );
        assert_eq!(protocol[9].body["result"]["index"], 9, "最近条目完整保留");
        // 小结果不压缩。
        let mut small = vec![tool_message("t", json!({"ok": true}))];
        assert_eq!(compress_old_tool_results(&mut small, 0), 0);
    }
}
