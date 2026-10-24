//! 已知托管请求的本地回执。只保存身份、像素摘要与观察结果；不保存图片或凭据。
use super::{
    model_binding::ModelIdentity,
    openai::{OpenAiProvider, hosted_async::HostedJob},
    store,
    types::*,
    vision::VisionPrefetch,
};
use crate::{
    command_audit::{fingerprint, serialization},
    db::Store,
    domain::{AppError, Result},
};
use serde::{Deserialize, Serialize};
use serde_json::{Value, json};
use sqlx::{Sqlite, Transaction};

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub(crate) struct VisionReceipt {
    #[serde(default, skip_serializing_if = "super::types::is_false")]
    pub allow_queued_recovery: bool,
    pub request_id: String,
    pub identity: ModelIdentity,
    pub body_sha256: String,
    pub photo_hashes: Vec<(String, String)>,
    pub output_limit: u32,
    pub prefetch: VisionPrefetch,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct VisionRecoveryView {
    pub state: String,
    pub images: u32,
}
pub(crate) async fn cancel_for_task(
    db: &Store,
    provider: &OpenAiProvider,
    task: &Task,
) -> Result<()> {
    let receipt = {
        let mut tx = db.pool.begin().await?;
        let Some(body) = reservation(&mut tx, &task.id).await? else {
            return Ok(());
        };
        let receipt: VisionReceipt =
            serde_json::from_value(body["hostedRequest"].clone()).map_err(serialization)?;
        if !receipt.allow_queued_recovery {
            return Ok(());
        }
        let prior:i64=sqlx::query_scalar("SELECT count(*) FROM agent_steps WHERE task_id=? AND kind='vision_cancel_result' AND json_extract(body_json,'$.requestId')=?")
            .bind(&task.id).bind(&receipt.request_id).fetch_one(&mut *tx).await?;
        if prior > 0 {
            return Ok(());
        }
        tx.commit().await?;
        receipt
    };
    let state = if provider.identity() != receipt.identity {
        "unconfirmed"
    } else {
        match provider.cancel_observation_job(&receipt.request_id).await {
            Ok(true) => "cancelled",
            Ok(false) => "already_sent",
            Err(_) => "unconfirmed",
        }
    };
    let mut tx = db.pool.begin_with("BEGIN IMMEDIATE").await?;
    let current = store::load(&mut tx, &task.id).await?;
    if current.state != "cancelled" {
        return Err(AppError::new("RECOVERY_ERROR", "本次任务尚未取消"));
    }
    store::step(
        &mut tx,
        &task.id,
        "vision_cancel_result",
        json!({"requestId":receipt.request_id,"state":state}),
        "本地任务已停止，已记录排队撤回结果",
    )
    .await?;
    tx.commit().await?;
    Ok(())
}
pub(crate) async fn cancellation(
    tx: &mut Transaction<'_, Sqlite>,
    id: &str,
) -> Result<Option<String>> {
    Ok(sqlx::query_scalar("SELECT json_extract(body_json,'$.state') FROM agent_steps WHERE task_id=? AND kind='vision_cancel_result' ORDER BY id DESC LIMIT 1")
        .bind(id).fetch_optional(&mut **tx).await?)
}
pub(crate) async fn binding(tx: &mut Transaction<'_, Sqlite>, task: &str) -> Result<Value> {
    let rows:Vec<String>=sqlx::query_scalar("SELECT body_json FROM agent_steps WHERE task_id=? AND kind='vision_reserved' AND json_type(body_json,'$.hostedRequest')='object' ORDER BY id")
        .bind(task).fetch_all(&mut **tx).await?;
    if rows.is_empty() {
        return Ok(Value::Null);
    }
    Ok(json!(fingerprint(&json!(rows))))
}
pub(crate) async fn reservation(
    tx: &mut Transaction<'_, Sqlite>,
    task: &str,
) -> Result<Option<Value>> {
    let raw:Option<String>=sqlx::query_scalar("SELECT r.body_json FROM agent_steps r WHERE r.task_id=? AND r.kind='vision_reserved' AND json_type(r.body_json,'$.hostedRequest')='object' AND NOT EXISTS(SELECT 1 FROM agent_steps s WHERE s.task_id=r.task_id AND s.kind='vision_settled' AND json_extract(s.body_json,'$.callId')=json_extract(r.body_json,'$.callId')) ORDER BY r.id DESC LIMIT 1")
        .bind(task).fetch_optional(&mut **tx).await?;
    raw.map(|r| serde_json::from_str(&r).map_err(serialization))
        .transpose()
}
pub(crate) async fn projection(
    tx: &mut Transaction<'_, Sqlite>,
    task: &Task,
) -> Result<Option<VisionRecoveryView>> {
    if !matches!(task.state.as_str(), "paused" | "interrupted") {
        return Ok(None);
    }
    let Some(body) = reservation(tx, &task.id).await? else {
        return Ok(None);
    };
    let receipt: VisionReceipt =
        serde_json::from_value(body["hostedRequest"].clone()).map_err(serialization)?;
    let last:Option<String>=sqlx::query_scalar("SELECT json_extract(body_json,'$.state') FROM agent_steps WHERE task_id=? AND kind='vision_result_query' AND json_extract(body_json,'$.requestId')=? ORDER BY id DESC LIMIT 1")
        .bind(&task.id).bind(&receipt.request_id).fetch_optional(&mut **tx).await?;
    Ok(Some(VisionRecoveryView {
        state: last.unwrap_or_else(|| "unknown".into()),
        images: receipt.prefetch.uploaded,
    }))
}
pub(crate) async fn validate(
    tx: &mut Transaction<'_, Sqlite>,
    task: &Task,
    call: &ToolCall,
    receipt: &VisionReceipt,
) -> Result<()> {
    if !matches!(task.state.as_str(), "running" | "paused" | "interrupted") {
        return Err(AppError::new("CANCELLED", "这次分析已结束，不能继续处理"));
    }
    super::policy::PolicyGate::validate_scope(tx, task, call).await?;
    super::memory_scope::validate(tx, task).await?;
    let allowed = crate::focal_chat::load(tx, &task.id)
        .await?
        .is_some_and(|c| c.allow_queued_recovery);
    if allowed != receipt.allow_queued_recovery {
        return Err(AppError::new("RECOVERY_ERROR", "本次分析的恢复选项不一致"));
    }
    if task.memory_revision != super::memory_control::revision(tx, &task.project_id).await?
        || super::model_binding::load(tx, &task.id)
            .await?
            .map(|v| v.observation_identity())
            != Some(receipt.identity.clone())
        || receipt.identity.account_id.is_none()
        || receipt.prefetch.model_id != receipt.identity.model_id
        || receipt.prefetch.cache_version != receipt.identity.cache_version()
        || uuid::Uuid::parse_str(&receipt.request_id).is_err()
        || !(128..=task.budget.max_output_tokens).contains(&receipt.output_limit)
    {
        return Err(AppError::new(
            "RECOVERY_ERROR",
            "账户、服务或上下文已改变，无法续接这次分析",
        ));
    }
    let call_ids: Vec<String> =
        serde_json::from_value(call.args["photo_ids"].clone()).map_err(serialization)?;
    if receipt.prefetch.requested != call_ids
        || receipt.photo_hashes.len() != call_ids.len()
        || receipt
            .photo_hashes
            .iter()
            .any(|(id, _)| !call_ids.contains(id))
    {
        return Err(AppError::new("RECOVERY_ERROR", "这次分析的照片范围不一致"));
    }
    let pending = matches!(&task.pending,Some(ModelOutcome::ToolCalls{calls}) if calls.iter().any(|c| fingerprint(&serde_json::to_value(c).unwrap())==fingerprint(&serde_json::to_value(call).unwrap())));
    if !pending {
        return Err(AppError::new("RECOVERY_ERROR", "未找到这次分析的待办操作"));
    }
    if super::consent::authorize(tx, task, 0).await? != Some(receipt.prefetch.consent_id) {
        return Err(AppError::new(
            "PERMISSION_DENIED",
            "照片使用许可已改变，无法续接这次分析",
        ));
    }
    for (id, hash) in &receipt.photo_hashes {
        let current: Option<String> =
            sqlx::query_scalar("SELECT content_sha256 FROM photos WHERE id=? AND project_id=?")
                .bind(id)
                .bind(&task.project_id)
                .fetch_optional(&mut **tx)
                .await?;
        if current.as_ref() != Some(hash) {
            return Err(AppError::new(
                "REVISION_CONFLICT",
                "照片版本已改变，无法续接这次分析",
            ));
        }
    }
    Ok(())
}
pub(crate) async fn before_dispatch(
    db: &Store,
    task: &str,
    call: &ToolCall,
    request: &str,
) -> Result<()> {
    let mut tx = db.pool.begin().await?;
    let current = store::load(&mut tx, task).await?;
    let body = reservation(&mut tx, task)
        .await?
        .ok_or_else(|| AppError::new("RECOVERY_ERROR", "分析请求记录不存在"))?;
    let receipt: VisionReceipt =
        serde_json::from_value(body["hostedRequest"].clone()).map_err(serialization)?;
    if receipt.request_id != request
        || body["fingerprint"] != fingerprint(&serde_json::to_value(call).unwrap())
    {
        return Err(AppError::new("RECOVERY_ERROR", "分析请求记录不一致"));
    }
    validate(&mut tx, &current, call, &receipt).await?;
    tx.commit().await?;
    Ok(())
}
pub(crate) async fn query(
    db: &Store,
    provider: &OpenAiProvider,
    project: &str,
    id: &str,
) -> Result<VisionRecoveryView> {
    let provider = provider.bind_account()?;
    let (call, receipt, rate) = {
        let mut tx = db.pool.begin().await?;
        let task = store::load(&mut tx, id).await?;
        if task.project_id != project {
            return Err(AppError::new("PERMISSION_DENIED", "无法查看这次分析"));
        }
        if !matches!(task.state.as_str(), "paused" | "interrupted") {
            return Err(AppError::new("TASK_BUSY", "请先暂停这次分析"));
        }
        let body = reservation(&mut tx, id).await?.ok_or_else(|| {
            AppError::new("RECOVERY_ERROR", "这次分析没有可查询的记录，请联系支持")
        })?;
        let receipt: VisionReceipt =
            serde_json::from_value(body["hostedRequest"].clone()).map_err(serialization)?;
        if provider.identity() != receipt.identity {
            return Err(AppError::new("AUTH_CHANGED", "请使用原账户查询这次分析"));
        }
        let call = match &task.pending {
            Some(ModelOutcome::ToolCalls { calls }) => calls
                .iter()
                .find(|c| c.call_id == body["callId"].as_str().unwrap_or_default())
                .cloned(),
            _ => None,
        }
        .ok_or_else(|| AppError::new("RECOVERY_ERROR", "未找到这次分析的待办操作"))?;
        if body["fingerprint"] != fingerprint(&serde_json::to_value(&call).unwrap()) {
            return Err(AppError::new(
                "IDEMPOTENCY_CONFLICT",
                "分析记录与照片范围不一致",
            ));
        }
        validate(&mut tx, &task, &call, &receipt).await?;
        let rate = serde_json::from_value(body["rateCard"].clone()).map_err(serialization)?;
        tx.commit().await?;
        (call, receipt, rate)
    };
    super::context::ContextBuilder::build(db, id).await?;
    let started = std::time::Instant::now();
    let job = provider
        .observation_job(&receipt.request_id, receipt.output_limit)
        .await?;
    if provider.identity() != provider.bind_account()?.identity() {
        return Err(AppError::new("AUTH_CHANGED", "请使用原账户查询这次分析"));
    }
    if let HostedJob::Completed(result) = job {
        let (observation, tokens) = provider.parse_observation(&result)?;
        let mut data = receipt.prefetch.clone();
        let missing: Vec<_> = data
            .requested
            .iter()
            .filter(|id| !data.observations.iter().any(|(cached, _)| cached == *id))
            .cloned()
            .collect();
        let valid: Vec<_> = observation["observations"]
            .as_array()
            .unwrap()
            .iter()
            .filter_map(|v| {
                let id = v["photo_id"].as_str()?;
                missing
                    .iter()
                    .any(|p| p == id)
                    .then(|| (id.to_owned(), v.clone()))
            })
            .collect();
        if valid.is_empty() {
            return Err(AppError::new(
                "MODEL_UNAVAILABLE",
                "分析结果未包含本次照片，无法继续使用",
            ));
        }
        data.observations.extend(valid);
        data.tokens = tokens;
        data.cost_minor_usd = super::pricing::RateCard::vision_cost(rate, data.uploaded, tokens);
        super::cloud::settle(
            db,
            id,
            &call,
            Some(&data),
            Some(data.cost_minor_usd),
            started.elapsed().as_millis() as u64,
        )
        .await?;
        return Ok(VisionRecoveryView {
            state: "completed".into(),
            images: data.uploaded,
        });
    }
    let state = match job {
        HostedJob::Pending => "pending",
        HostedJob::Rejected => "rejected",
        HostedJob::Expired => "expired",
        _ => "unknown",
    };
    let mut tx = db.pool.begin_with("BEGIN IMMEDIATE").await?;
    let task = store::load(&mut tx, id).await?;
    validate(&mut tx, &task, &call, &receipt).await?;
    store::step(
        &mut tx,
        id,
        "vision_result_query",
        json!({"requestId":receipt.request_id,"state":state}),
        "已查询原分析状态，未重新发送照片",
    )
    .await?;
    tx.commit().await?;
    Ok(VisionRecoveryView {
        state: state.into(),
        images: receipt.prefetch.uploaded,
    })
}
