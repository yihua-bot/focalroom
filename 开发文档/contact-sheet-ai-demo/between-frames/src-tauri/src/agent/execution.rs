use super::{policy::PolicyGate, store, tools, types::*, vision::VisionPrefetch};
use crate::{
    command_audit::{encode, fingerprint},
    db::Store,
    domain::{AppError, Result},
};
use serde_json::json;
use sqlx::Row;
pub(crate) async fn state(db: &Store, id: &str, next: &str, reason: &str) -> Result<()> {
    let mut tx = db.pool.begin_with("BEGIN IMMEDIATE").await?;
    let mut t = store::load(&mut tx, id).await?;
    store::transition(&mut tx, &mut t, next, reason).await?;
    store::checkpoint(&mut tx, &t).await?;
    tx.commit().await?;
    Ok(())
}
pub(crate) async fn outcome(
    db: &Store,
    id: &str,
    result: ModelOutcome,
    input: u32,
    elapsed: u64,
) -> Result<()> {
    let mut tx = db.pool.begin_with("BEGIN IMMEDIATE").await?;
    let mut t = store::load(&mut tx, id).await?;
    if t.state != "running" {
        return Err(AppError::new(
            "CANCELLED",
            "任务已停止，丢弃未提交的模型结果",
        ));
    }

    let bytes = serde_json::to_vec(&result).unwrap().len() as u32;
    t.usage.output_estimate = t.usage.output_estimate.saturating_add(bytes);
    t.elapsed_ms += elapsed;
    sqlx::query("UPDATE model_usage SET output_estimate=? WHERE task_id=? AND request_id=?")
        .bind(bytes as i64)
        .bind(&t.id)
        .bind(format!("turn-{}", t.usage.model_turns))
        .execute(&mut *tx)
        .await?;
    let _ = input;
    if bytes > t.budget.max_output_tokens
        || t.elapsed_ms >= u64::from(t.budget.max_duration_seconds) * 1000
        || t.usage
            .input_estimate
            .saturating_add(t.usage.output_estimate)
            > t.budget.max_total_tokens
    {
        store::transition(
            &mut tx,
            &mut t,
            "paused",
            "模型输出或时间达到预算上限，未执行工具",
        )
        .await?;
    } else {
        if let ModelOutcome::ToolCalls { calls } = &result
            && (calls.is_empty()
                || calls.len() > 16
                || calls
                    .iter()
                    .map(|c| &c.call_id)
                    .collect::<std::collections::HashSet<_>>()
                    .len()
                    != calls.len())
        {
            return Err(AppError::new("VALIDATION_ERROR", "模型工具批次无效"));
        }
        t.pending = Some(result.clone());
        store::step(
            &mut tx,
            id,
            "model",
            {let mut body=serde_json::to_value(&result).unwrap();body["executionVersion"]=json!({"providerId":t.provider_id,"promptVersion":super::prompt_version(&t.provider_id)});body},
            "模型回合已保存；下一步由工具观察决定",
        )
        .await?;
        store::save(&mut tx, &t).await?;
    }
    store::checkpoint(&mut tx, &t).await?;
    tx.commit().await?;
    Ok(())
}
pub(crate) async fn tool(
    db: &Store,
    id: &str,
    call: &ToolCall,
    vision_data: Option<&VisionPrefetch>,
) -> Result<ToolResult> {
    let mut tx = db.pool.begin_with("BEGIN IMMEDIATE").await?;
    let mut t = store::load(&mut tx, id).await?;
    // 身份和 task scope 在回放前检查，指纹不允许同 call ID 更换参数。
    if call.call_id.is_empty() || call.call_id.len() > 160 {
        return Err(AppError::new("VALIDATION_ERROR", "call_id 不合法"));
    }
    if call.args["project_id"] != t.project_id {
        return Err(AppError::new("PERMISSION_DENIED", "项目 scope 不匹配"));
    }
    let fp = fingerprint(&serde_json::to_value(call).unwrap());
    if let Some(row) =
        sqlx::query("SELECT fingerprint,result_json FROM tool_calls WHERE task_id=? AND call_id=?")
            .bind(id)
            .bind(&call.call_id)
            .fetch_optional(&mut *tx)
            .await?
    {
        if row.get::<String, _>("fingerprint") != fp {
            return Err(AppError::new(
                "IDEMPOTENCY_CONFLICT",
                "同 call_id 参数不一致",
            ));
        }
        return serde_json::from_str(&row.get::<String, _>("result_json"))
            .map_err(crate::command_audit::serialization);
    }
    if t.state != "running" {
        return Err(AppError::new("CANCELLED", "任务已停止"));
    }
    if t.usage.tool_calls >= t.budget.max_tool_calls
        || t.elapsed_ms >= u64::from(t.budget.max_duration_seconds) * 1000
    {
        store::transition(&mut tx, &mut t, "paused", "工具调用达到预算上限").await?;
        store::checkpoint(&mut tx, &t).await?;
        tx.commit().await?;
        return Err(AppError::new("BUDGET_EXCEEDED", "工具预算耗尽"));
    }
    let started = std::time::Instant::now();
    t.usage.tool_calls += 1;
    sqlx::query("SAVEPOINT tool_effect")
        .execute(&mut *tx)
        .await?;
    let validated = PolicyGate::validate(&mut tx, &t, call).await;
    let execution = if let Err(e) = validated {
        Err(e)
    } else {
        tools::execute(&mut tx, &mut t, call, vision_data).await
    };
    let mut result = match execution {
        Ok(v) => ToolResult::success(&call.call_id, v),
        Err(e) => {
            sqlx::query("ROLLBACK TO tool_effect")
                .execute(&mut *tx)
                .await?;
            // 回滚业务作用后重新读取状态，保留一次实际拒绝计数。
            t = store::load(&mut tx, id).await?;
            t.usage.tool_calls += 1;
            ToolResult::failure(&call.call_id, e)
        }
    };
    sqlx::query("RELEASE tool_effect").execute(&mut *tx).await?;
    result.duration_ms = started.elapsed().as_millis() as u64;
    t.elapsed_ms += result.duration_ms;
    store::save(&mut tx, &t).await?;
    sqlx::query("INSERT INTO tool_calls VALUES(?,?,?,?,?)")
        .bind(id)
        .bind(&call.call_id)
        .bind(&call.name)
        .bind(fp)
        .bind(encode(&result)?)
        .execute(&mut *tx)
        .await?;
    let summary = if !result.ok {
        format!(
            "工具校验拒绝或执行失败：{}",
            result
                .error
                .as_ref()
                .map(|e| e.message.as_str())
                .unwrap_or("未知错误")
        )
    } else {
        let v = result.result.as_ref().unwrap();
        match call.name.as_str() {
            "get_project_context" => "读取项目目标与任务范围".into(),
            "list_photos" | "search_photos" => format!(
                "检索了 {} 张授权照片的人工元数据",
                v["photos"].as_array().map_or(0, Vec::len)
            ),
            "get_sequence" => format!(
                "读取序列与选区，共 {} 张",
                v["photo_ids"].as_array().map_or(0, Vec::len)
            ),
            "inspect_photos" | "compare_photos" => {
                let count = v["photos"].as_array().map_or(0, Vec::len);
                if v["mode"] == "vision_observed" {
                    format!(
                        "视觉观察了 {} 张照片（上传 {} 张，缓存命中 {} 张）",
                        count,
                        v["uploaded_images"].as_u64().unwrap_or(0),
                        v["cache_hits"].as_u64().unwrap_or(0)
                    )
                } else {
                    format!("对照了 {count} 张人工元数据；没有读图")
                }
            }
            "retrieve_preferences" => format!(
                "读取 {} 条项目人工线索",
                v["human_facts"].as_array().map_or(0, Vec::len)
            ),
            "create_selection_draft" => format!("创建 {} 项候选草案", v["count"]),
            "update_draft_sequence" => format!("修订草案至 {} 项", v["count"]),
            "evaluate_draft" => format!(
                "核验 {} 项草案，人工重复组 {} 组",
                v["count"],
                v["duplicate_groups"].as_array().map_or(0, Vec::len)
            ),
            "request_feedback" => "提出人工问题，等待答复".into(),
            "finish_task" => format!(
                "提交 {} 份草案等待审核",
                v["draft_ids"].as_array().map_or(0, Vec::len)
            ),
            _ => "受限工具已完成".into(),
        }
    };
    store::step(
        &mut tx,
        id,
        "tool",
        json!({"name":call.name,"call":call,"result":result}),
        &summary,
    )
    .await?;
    store::checkpoint(&mut tx, &t).await?;
    tx.commit().await?;
    Ok(result)
}
pub(crate) async fn clear(db: &Store, id: &str) -> Result<()> {
    let mut tx = db.pool.begin_with("BEGIN IMMEDIATE").await?;
    let mut t = store::load(&mut tx, id).await?;
    if t.state != "running" {
        return Ok(());
    }
    t.pending = None;
    store::save(&mut tx, &t).await?;
    store::checkpoint(&mut tx, &t).await?;
    tx.commit().await?;
    Ok(())
}
pub(crate) async fn final_outcome(db: &Store, id: &str, outcome: ModelOutcome) -> Result<()> {
    let mut tx = db.pool.begin_with("BEGIN IMMEDIATE").await?;
    let mut t = store::load(&mut tx, id).await?;
    if t.state != "running" {
        return Err(AppError::new("CANCELLED", "任务已停止"));
    }
    match outcome {
        ModelOutcome::NeedsInput { question } => {
            if question.is_empty() || question.chars().count() > 2000 {
                return Err(AppError::new("VALIDATION_ERROR", "问题不合法"));
            }
            t.question = Some(question);
            store::transition(&mut tx, &mut t, "needs_input", "等待人工答复").await?;
        }
        ModelOutcome::FinalDrafts { draft_ids, summary } => {
            tools::validate_final(&mut tx, &t, &draft_ids).await?;
            t.final_draft_ids = draft_ids;
            store::transition(&mut tx, &mut t, "review_ready", &summary).await?;
            let revision: i64 = sqlx::query_scalar("SELECT revision FROM projects WHERE id=?")
                .bind(&t.project_id)
                .fetch_one(&mut *tx)
                .await?;
            if revision != t.base_revision {
                store::transition(&mut tx, &mut t, "stale", "基线变化，需重新比较").await?;
            }
        }
        ModelOutcome::NoAction { reason } => {
            store::transition(&mut tx, &mut t, "completed", &reason).await?
        }
        _ => return Err(AppError::new("VALIDATION_ERROR", "不是最终结果")),
    }
    t.pending = None;
    store::save(&mut tx, &t).await?;
    store::checkpoint(&mut tx, &t).await?;
    tx.commit().await?;
    Ok(())
}

pub(crate) async fn attempt(db: &Store, id: &str, input: u32) -> Result<()> {
    let mut tx = db.pool.begin_with("BEGIN IMMEDIATE").await?;
    let mut t = store::load(&mut tx, id).await?;
    if t.state != "running" {
        return Err(AppError::new("CANCELLED", "任务已停止"));
    }
    PolicyGate::model_budget(&t, input)?;
    t.usage.model_turns += 1;
    t.usage.input_estimate = t.usage.input_estimate.saturating_add(input);
    sqlx::query("INSERT INTO model_usage VALUES(?,?,?,?,0,0)")
        .bind(id)
        .bind(format!("turn-{}", t.usage.model_turns))
        .bind(input as i64)
        .bind(0i64)
        .execute(&mut *tx)
        .await?;
    store::step(&mut tx,id,"model_attempt",json!({"round":t.usage.model_turns,"startedAt":crate::domain::timestamp(),"promptVersion":super::prompt_version(&t.provider_id),"providerId":t.provider_id}),"发起一轮模型请求").await?;
    store::save(&mut tx, &t).await?;
    tx.commit().await?;
    Ok(())
}
pub(crate) async fn elapsed(db: &Store, id: &str, elapsed: u64) -> Result<()> {
    let mut tx = db.pool.begin_with("BEGIN IMMEDIATE").await?;
    let mut t = store::load(&mut tx, id).await?;
    if t.state != "running" {
        return Ok(());
    }
    t.elapsed_ms = t.elapsed_ms.saturating_add(elapsed);
    store::step(
        &mut tx,
        id,
        "model_wait_settled",
        json!({"round":t.usage.model_turns,"chargedMs":elapsed}),
        "已结算失败/超时等待，未重复执行工具",
    )
    .await?;
    store::save(&mut tx, &t).await?;
    tx.commit().await?;
    Ok(())
}
