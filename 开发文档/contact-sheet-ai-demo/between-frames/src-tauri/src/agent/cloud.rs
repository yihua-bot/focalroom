// 云端尝试账本复用 append-only agent_steps：先持久化预留，再发送；中断不自动重传。
use super::{consent, policy::PolicyGate, store, types::*, vision::VisionPrefetch};
use crate::{
    command_audit::{fingerprint, serialization},
    db::Store,
    domain::{AppError, Result},
};
use serde_json::{Value, json};

pub(crate) async fn replay(
    db: &Store,
    task: &str,
    call: &ToolCall,
) -> Result<Option<VisionPrefetch>> {
    let body: Option<String> = sqlx::query_scalar("SELECT body_json FROM agent_steps WHERE task_id=? AND kind IN('vision_reserved','vision_settled') AND json_extract(body_json,'$.callId')=? ORDER BY id DESC LIMIT 1")
        .bind(task).bind(&call.call_id).fetch_optional(&db.pool).await?;
    let Some(body) = body else {
        return Ok(None);
    };
    let body: Value = serde_json::from_str(&body).map_err(serialization)?;
    if body["fingerprint"] != fingerprint(&serde_json::to_value(call).unwrap()) {
        return Err(AppError::new(
            "IDEMPOTENCY_CONFLICT",
            "视觉调用 ID 已用于不同参数",
        ));
    }
    match body["status"].as_str() {
        Some("completed") => serde_json::from_value(body["prefetch"].clone())
            .map(Some)
            .map_err(serialization),
        Some("failed") => Err(AppError::new(
            "BUDGET_EXCEEDED",
            "此前视觉请求失败；已保留预算记录，不自动重传，请重新委托",
        )),
        _ => Err(AppError::new(
            "BUDGET_EXCEEDED",
            "视觉请求的传输结果尚未确认；预算预留保留，请重新委托，恢复不会重复上传",
        )),
    }
}

pub(crate) async fn reserve(
    db: &Store,
    task: &str,
    call: &ToolCall,
    images: u32,
    cost: u32,
) -> Result<(i64, u64)> {
    let mut tx = db.pool.begin_with("BEGIN IMMEDIATE").await?;
    let mut t = store::load(&mut tx, task).await?;
    PolicyGate::validate(&mut tx, &t, call).await?;
    let existing: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM agent_steps WHERE task_id=? AND kind='vision_reserved' AND json_extract(body_json,'$.callId')=?")
        .bind(task).bind(&call.call_id).fetch_one(&mut *tx).await?;
    if existing > 0 {
        return Err(AppError::new(
            "BUDGET_EXCEEDED",
            "视觉请求已预留，不能重复传输",
        ));
    }
    if t.usage.tool_calls >= t.budget.max_tool_calls
        || t.elapsed_ms >= u64::from(t.budget.max_duration_seconds) * 1000
        || t.usage.cloud_images.saturating_add(images) > t.budget.max_cloud_images
        || t.usage.cost_minor_usd.saturating_add(cost) > t.budget.max_cost_minor_usd
    {
        return Err(AppError::new(
            "BUDGET_EXCEEDED",
            "视觉请求预留超过图像、费用、工具或时间预算；未上传",
        ));
    }
    let consent = consent::authorize(&mut tx, &t, i64::from(images))
        .await?
        .ok_or_else(|| AppError::new("PERMISSION_DENIED", "图像授权已撤回或配额不足；未上传"))?;
    consent::reserve_usage(&mut tx, &mut t, consent, images, cost).await?;
    store::step(&mut tx, task, "vision_reserved", json!({"callId":call.call_id,"fingerprint":fingerprint(&serde_json::to_value(call).unwrap()),"status":"reserved","images":images,"cost":cost,"round":t.usage.model_turns}), "视觉请求预算已预留；中断时不自动重传").await?;
    store::checkpoint(&mut tx, &t).await?;
    tx.commit().await?;
    Ok((
        consent,
        (u64::from(t.budget.max_duration_seconds) * 1000).saturating_sub(t.elapsed_ms),
    ))
}

pub(crate) async fn settle(
    db: &Store,
    task: &str,
    call: &ToolCall,
    data: Option<&VisionPrefetch>,
    cost: Option<u32>,
    elapsed: u64,
) -> Result<()> {
    let mut tx = db.pool.begin_with("BEGIN IMMEDIATE").await?;
    let mut t = store::load(&mut tx, task).await?;
    let raw: String = sqlx::query_scalar("SELECT body_json FROM agent_steps WHERE task_id=? AND kind='vision_reserved' AND json_extract(body_json,'$.callId')=? ORDER BY id DESC LIMIT 1")
        .bind(task).bind(&call.call_id).fetch_one(&mut *tx).await?;
    let mut reserved: Value = serde_json::from_str(&raw).map_err(serialization)?;
    let settled: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM agent_steps WHERE task_id=? AND kind='vision_settled' AND json_extract(body_json,'$.callId')=?")
        .bind(task).bind(&call.call_id).fetch_one(&mut *tx).await?;
    if settled > 0 {
        return Ok(());
    }
    let upper = reserved["cost"].as_u64().unwrap() as u32;
    // 没有可信 usage（超时/网络错误/坏 JSON）时保留预留，不能把未知外发当零消耗。
    let refund = cost.map_or(0, |actual| upper.saturating_sub(actual));
    let overage = cost.map_or(0, |actual| actual.saturating_sub(upper));
    t.usage.cost_minor_usd = t
        .usage
        .cost_minor_usd
        .saturating_sub(refund)
        .saturating_add(overage);
    t.elapsed_ms = t.elapsed_ms.saturating_add(elapsed);
    sqlx::query(
        "UPDATE model_usage SET cost_minor_usd=cost_minor_usd-?+? WHERE task_id=? AND request_id=?",
    )
    .bind(i64::from(refund))
    .bind(i64::from(overage))
    .bind(task)
    .bind(format!("turn-{}", reserved["round"].as_u64().unwrap()))
    .execute(&mut *tx)
    .await?;
    reserved["status"] = json!(if data.is_some() {
        "completed"
    } else {
        "failed"
    });
    reserved["prefetch"] = serde_json::to_value(data).map_err(serialization)?;
    reserved["reportedCost"] = json!(cost);
    store::step(
        &mut tx,
        task,
        "vision_settled",
        reserved,
        "视觉请求已结算；失败或取消后仍保留传输预算记录",
    )
    .await?;
    if overage > 0 && t.state == "running" {
        store::transition(
            &mut tx,
            &mut t,
            "paused",
            "服务商报告的用量超出预留估算；已记录全部用量，请核对模型计量配置后重新委托",
        )
        .await?;
    }
    store::save(&mut tx, &t).await?;
    store::checkpoint(&mut tx, &t).await?;
    tx.commit().await?;
    Ok(())
}
