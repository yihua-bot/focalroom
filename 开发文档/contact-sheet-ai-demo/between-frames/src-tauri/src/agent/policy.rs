use super::types::{Task, ToolCall};
use crate::domain::{AppError, Result};
use serde_json::{Value, json};
use sqlx::{Sqlite, Transaction};

pub struct ToolRegistry;
impl ToolRegistry {
    pub fn specs() -> Value {
        serde_json::from_str(include_str!("../../../contracts/agent-tool-contracts.json"))
            .expect("工具契约")
    }
    /// 全量工具定义（不含画布工具——画布工具仅 canvas 任务发送）。
    pub fn model_specs() -> Value {
        Self::model_specs_for(false)
    }
    /// 条件性工具发送：canvas=true 时附加画布工具。
    /// 非画布任务不发送画布工具，避免请求膨胀与越权提示。
    pub fn model_specs_for(canvas: bool) -> Value {
        Self::specs()["tools"]
            .as_array()
            .unwrap()
            .iter()
            .filter(|tool| {
                canvas || !super::task_scope::CANVAS_TOOLS.contains(&tool["name"].as_str().unwrap_or(""))
            })
            .map(|tool| {
                json!({"type":"function","name":tool["name"],"description":tool["description"],"parameters":tool["parameters"],"strict":true})
            })
            .collect::<Vec<_>>()
            .into()
    }
    pub fn model_specs_for_grant(grant: &super::task_scope::TaskGrant) -> Value {
        Self::model_specs_for(true)
            .as_array()
            .unwrap()
            .iter()
            .filter(|tool| grant.tools.iter().any(|name| tool["name"] == *name))
            .cloned()
            .collect::<Vec<_>>()
            .into()
    }
    pub fn validate(call: &ToolCall) -> Result<()> {
        if call.call_id.is_empty() || call.call_id.len() > 160 {
            return Err(AppError::new("VALIDATION_ERROR", "call_id 不合法"));
        }
        let specs = Self::specs();
        let spec = specs["tools"]
            .as_array()
            .unwrap()
            .iter()
            .find(|t| {
                t["name"] == call.name || t.get("v2Alias") == Some(&serde_json::json!(call.name))
            })
            .ok_or_else(|| AppError::new("PERMISSION_DENIED", "工具不在摄影工具白名单"))?;
        validate_schema(&spec["parameters"], &call.args)
    }
    /// v2 别名解析为规范工具名（无别名返回原名）；M5 评论协作分析在云端 Worker 执行。
    pub fn canonical(name: &str) -> Result<String> {
        if matches!(name, "summarize_review_disagreement") {
            return Err(AppError::new(
                "NOT_IMPLEMENTED",
                "评论分歧分析由共享服务的云 Agent Worker 执行：项目所有者在 Web 端显式授权模型发送后排队，本地任务不读取项目评论",
            ));
        }
        let specs = Self::specs();
        Ok(specs["tools"]
            .as_array()
            .unwrap()
            .iter()
            .find(|t| t["name"] == name || t.get("v2Alias") == Some(&serde_json::json!(name)))
            .map(|t| t["name"].as_str().unwrap().to_owned())
            .unwrap_or_else(|| name.to_owned()))
    }
}
pub(crate) fn validate_schema(schema: &Value, value: &Value) -> Result<()> {
    if betweenframes_local_domain::schema::valid(schema, value) {
        Ok(())
    } else {
        Err(AppError::new(
            "VALIDATION_ERROR",
            "工具参数不符合严格 schema",
        ))
    }
}
pub struct PolicyGate;
impl PolicyGate {
    /// 只收紧本轮输出，任务总预算和其他执行门槛保持权威值。
    pub(crate) fn output_limit(
        task: &Task,
        input: u32,
        cost_limit: u32,
        rate: super::pricing::RateCard,
    ) -> Result<u32> {
        if task.provider_id != super::PROVIDER_OPENAI {
            return Ok(task.budget.max_output_tokens);
        }
        let remaining = task
            .budget
            .max_cost_minor_usd
            .min(cost_limit)
            .saturating_sub(task.usage.cost_minor_usd);
        if rate.model_cost(input, 128) > remaining {
            return Err(super::budget::exceeded(
                super::budget::LimitKind::Cost,
                u64::from(task.usage.cost_minor_usd),
                u64::from(rate.model_cost(input, 128)),
                u64::from(task.budget.max_cost_minor_usd.min(cost_limit)),
                "model",
            ));
        }
        let (mut low, mut high) = (128, task.budget.max_output_tokens);
        while low < high {
            let middle = low + (high - low).div_ceil(2);
            if rate.model_cost(input, middle) <= remaining {
                low = middle;
            } else {
                high = middle - 1;
            }
        }
        Ok(low)
    }
    pub async fn validate(
        tx: &mut Transaction<'_, Sqlite>,
        task: &Task,
        call: &ToolCall,
    ) -> Result<()> {
        ToolRegistry::validate(call).map_err(|error| {
            if call.name == "list_photos" && error.code == "VALIDATION_ERROR" {
                AppError::new("VALIDATION_ERROR", format!("list_photos 参数不合法；status 仅允许 keep/reject/undecided 或 JSON null。首个分页可用以下完整参数：{}。null 不加引号，不能写成字符串。", json!({"project_id":task.project_id,"status":null,"limit":100,"cursor":null})))
            } else { error }
        })?;
        super::task_scope::require_tool(tx, task, &call.name).await?;
        if call.args["project_id"] != task.project_id {
            return Err(AppError::new("PERMISSION_DENIED", "工具不能访问其他项目"));
        }
        if task.state != "running" {
            return Err(AppError::new("CANCELLED", "任务已暂停或停止"));
        }
        let active: i64 =
            sqlx::query_scalar("SELECT COUNT(*) FROM projects WHERE id=? AND deleted_at IS NULL")
                .bind(&task.project_id)
                .fetch_one(&mut **tx)
                .await?;
        if active != 1 {
            return Err(AppError::new("PERMISSION_DENIED", "项目不可访问"));
        }
        let mut ids = vec![];
        if let Some(arr) = call.args["photo_ids"]
            .as_array()
            .or(call.args["ordered_photo_ids"].as_array())
        {
            ids.extend(arr.iter().filter_map(Value::as_str));
        }
        if let Some(items) = call.args["items"].as_array() {
            ids.extend(items.iter().filter_map(|i| i["photo_id"].as_str()));
        }
        for id in ids {
            if !task.allowed_photo_ids.iter().any(|p| p == id) {
                return Err(AppError::new("PERMISSION_DENIED", "照片不在此任务授权范围"));
            }
            let exists: i64 =
                sqlx::query_scalar("SELECT COUNT(*) FROM photos WHERE id=? AND project_id=?")
                    .bind(id)
                    .bind(&task.project_id)
                    .fetch_one(&mut **tx)
                    .await?;
            if exists != 1 {
                return Err(AppError::new("PERMISSION_DENIED", "照片不属于此项目"));
            }
        }
        if call.args.get("draft_id").is_some() {
            let owned:i64=sqlx::query_scalar("SELECT COUNT(*) FROM drafts WHERE id=? AND task_id=? AND project_id=? AND state='pending'").bind(call.args["draft_id"].as_str()).bind(&task.id).bind(&task.project_id).fetch_one(&mut **tx).await?;
            if owned != 1 {
                return Err(AppError::new("PERMISSION_DENIED", "草案不属于当前任务"));
            }
        }
        Ok(())
    }
    pub fn model_budget(
        task: &Task,
        input: u32,
        cost_limit: u32,
        rate: super::pricing::RateCard,
    ) -> Result<()> {
        let b = &task.budget;
        let u = &task.usage;
        use super::budget::LimitKind::*;
        let checks = [
            (
                Time,
                task.elapsed_ms,
                1,
                u64::from(b.max_duration_seconds) * 1000,
            ),
            (
                Rounds,
                u64::from(u.model_turns),
                1,
                u64::from(b.max_model_turns),
            ),
            (
                Steps,
                u64::from(u.tool_calls),
                1,
                u64::from(b.max_tool_calls),
            ),
            (
                Context,
                0,
                u64::from(input),
                u64::from(b.max_context_tokens),
            ),
            (
                Tokens,
                u64::from(u.input_estimate) + u64::from(u.output_estimate),
                u64::from(input) + u64::from(b.max_output_tokens),
                u64::from(b.max_total_tokens),
            ),
            (
                Images,
                u64::from(u.cloud_images),
                0,
                u64::from(b.max_cloud_images),
            ),
            (
                Cost,
                u64::from(u.cost_minor_usd),
                u64::from(rate.reservation(task, input)),
                u64::from(b.max_cost_minor_usd.min(cost_limit)),
            ),
        ];
        for (kind, used, requested, limit) in checks {
            if used.saturating_add(requested) > limit {
                return Err(super::budget::exceeded(
                    kind, used, requested, limit, "model",
                ));
            }
        }
        Ok(())
    }
}
