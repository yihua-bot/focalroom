use super::types::{Task, ToolCall};
use crate::domain::{AppError, Result};
use serde_json::{Value, json};
use sqlx::{Sqlite, Transaction};
use std::collections::HashSet;

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
            .find(|t| t["name"] == call.name)
            .ok_or_else(|| AppError::new("PERMISSION_DENIED", "工具不在摄影工具白名单"))?;
        validate_schema(&spec["parameters"], &call.args)
    }
}
pub(crate) fn validate_schema(schema: &Value, value: &Value) -> Result<()> {
    let fail = || AppError::new("VALIDATION_ERROR", "工具参数不符合严格 schema");
    if let Some(c) = schema.get("const")
        && c != value
    {
        return Err(fail());
    }
    if let Some(e) = schema.get("enum")
        && !e.as_array().unwrap().contains(value)
    {
        return Err(fail());
    }
    if let Some(branches) = schema.get("oneOf")
        && branches
            .as_array()
            .unwrap()
            .iter()
            .filter(|s| validate_schema(s, value).is_ok())
            .count()
            != 1
    {
        return Err(fail());
    }
    let is_type = |t: &str| match t {
        "object" => value.is_object(),
        "array" => value.is_array(),
        "string" => value.is_string(),
        "integer" => value.as_i64().is_some(),
        "null" => value.is_null(),
        "boolean" => value.is_boolean(),
        _ => false,
    };
    if let Some(t) = schema.get("type")
        && (t.as_str().is_some_and(|t| !is_type(t))
            || t.as_array()
                .is_some_and(|ts| !ts.iter().any(|t| is_type(t.as_str().unwrap_or("")))))
    {
        return Err(fail());
    }
    if let Some(obj) = value.as_object() {
        if let Some(required) = schema.get("required")
            && required
                .as_array()
                .unwrap()
                .iter()
                .any(|k| !obj.contains_key(k.as_str().unwrap()))
        {
            return Err(fail());
        }
        if let Some(props) = schema["properties"].as_object() {
            for (k, v) in obj {
                if let Some(s) = props.get(k) {
                    validate_schema(s, v)?;
                } else if schema["additionalProperties"] == false {
                    return Err(fail());
                }
            }
        }
    }
    if let Some(list) = value.as_array() {
        let n = list.len() as u64;
        if schema["minItems"].as_u64().is_some_and(|v| n < v)
            || schema["maxItems"].as_u64().is_some_and(|v| n > v)
        {
            return Err(fail());
        }
        if schema["uniqueItems"] == true
            && list
                .iter()
                .map(Value::to_string)
                .collect::<HashSet<_>>()
                .len()
                != list.len()
        {
            return Err(fail());
        }
        if let Some(item) = schema.get("items") {
            for v in list {
                validate_schema(item, v)?;
            }
        }
    }
    if let Some(text) = value.as_str() {
        let n = text.chars().count() as u64;
        if schema["minLength"].as_u64().is_some_and(|v| n < v)
            || schema["maxLength"].as_u64().is_some_and(|v| n > v)
        {
            return Err(fail());
        }
    }
    if let Some(n) = value.as_i64()
        && (schema["minimum"].as_i64().is_some_and(|v| n < v)
            || schema["maximum"].as_i64().is_some_and(|v| n > v))
    {
        return Err(fail());
    }
    Ok(())
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
