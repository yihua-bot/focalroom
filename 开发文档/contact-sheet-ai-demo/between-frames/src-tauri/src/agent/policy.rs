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
    pub fn model_specs() -> Value {
        Value::Array(Self::specs()["tools"].as_array().unwrap().iter().map(|tool|json!({"name":tool["name"],"description":tool["description"],"parameters":tool["parameters"],"strict":true})).collect())
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
    /// v2 别名解析为规范工具名（无别名返回原名）；M5 评论类工具显式拒绝。
    pub fn canonical(name: &str) -> Result<String> {
        if matches!(name, "get_comments" | "summarize_review_disagreement") {
            return Err(AppError::new(
                "NOT_IMPLEMENTED",
                "评论协作工具在 M5 交付；当前任务请使用既有工具集",
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
    pub async fn validate(
        tx: &mut Transaction<'_, Sqlite>,
        task: &Task,
        call: &ToolCall,
    ) -> Result<()> {
        ToolRegistry::validate(call)?;
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
    pub fn model_budget(task: &Task, input: u32) -> Result<()> {
        let b = &task.budget;
        let u = &task.usage;
        if u.model_turns >= b.max_model_turns
            || u.tool_calls >= b.max_tool_calls
            || task.elapsed_ms >= u64::from(b.max_duration_seconds) * 1000
            || input > b.max_context_tokens
            || u.input_estimate
                .saturating_add(u.output_estimate)
                .saturating_add(input)
                .saturating_add(b.max_output_tokens)
                > b.max_total_tokens
            || u.cloud_images > b.max_cloud_images
            || u.cost_minor_usd > b.max_cost_minor_usd
            || u.cost_minor_usd
                .saturating_add(super::openai::model_reservation(task, input))
                > b.max_cost_minor_usd
        {
            return Err(AppError::new(
                "BUDGET_EXCEEDED",
                "后端预算已达到上限，请调整预算后恢复或取消",
            ));
        }
        Ok(())
    }
}
