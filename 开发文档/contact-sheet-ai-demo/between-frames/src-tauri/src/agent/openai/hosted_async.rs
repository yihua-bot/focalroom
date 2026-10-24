//! 托管图片请求只提交一次，随后只读轮询；不信任服务端提供的跳转地址。
use super::OpenAiProvider;
use crate::domain::{AppError, Result};
use serde_json::{Value, json};
use std::time::Duration;

pub(super) fn has_images(value: &Value) -> bool {
    match value {
        Value::Object(map) => value["type"] == "input_image" || map.values().any(has_images),
        Value::Array(items) => items.iter().any(has_images),
        _ => false,
    }
}
pub(super) fn http_error(status: reqwest::StatusCode) -> AppError {
    let (code, message) = match status.as_u16() {
        401 => ("AUTH_REQUIRED", "请登录 FocalRoom 后重新委托"),
        402 => ("AI_QUOTA_EXHAUSTED", "AI 额度不足，请联系 FocalRoom"),
        403 => ("AI_PERMISSION_DENIED", "请确认邮箱已验证，并重新授权"),
        409 => (
            "AI_REQUEST_CONFLICT",
            "服务配置或请求状态已改变，请重新委托",
        ),
        503 => ("AI_NOT_CONFIGURED", "FocalRoom AI 暂未开放"),
        _ => ("AI_RESULT_UNKNOWN", "暂未取得处理结果，请勿重复提交"),
    };
    AppError::new(code, message)
}
fn unknown(id: &str) -> AppError {
    let mut error = AppError::new("AI_RESULT_UNKNOWN", "暂未取得照片分析结果，请勿重复提交");
    error.details = Some(json!({"requestId":id}));
    error
}
async fn read_job(mut response: reqwest::Response, id: &str) -> Result<Value> {
    let mut bytes = Vec::new();
    while let Some(chunk) = response.chunk().await.map_err(|_| unknown(id))? {
        if bytes.len().saturating_add(chunk.len()) > 2 * 1024 * 1024 + 4096 {
            return Err(unknown(id));
        }
        bytes.extend_from_slice(&chunk);
    }
    serde_json::from_slice(&bytes).map_err(|_| unknown(id))
}
pub(super) async fn post(provider: &OpenAiProvider, body: &Value) -> Result<Value> {
    post_with_id(
        provider,
        body,
        &uuid::Uuid::new_v4().to_string(),
        false,
        std::future::ready(Ok(())),
    )
    .await
}
pub(super) async fn post_with_id(
    provider: &OpenAiProvider,
    body: &Value,
    id: &str,
    allow_queued_recovery: bool,
    guard: impl std::future::Future<Output = Result<()>>,
) -> Result<Value> {
    validate_id(id)?;
    let auth = provider.hosted.as_ref().expect("hosted image transport");
    let expected = provider
        .hosted_account
        .as_deref()
        .ok_or_else(|| AppError::new("AUTH_REQUIRED", "请先登录 FocalRoom"))?;
    let token = auth.bearer(auth.origin()).await?;
    let check_account = || -> Result<()> {
        if auth.account_id()? != expected {
            return Err(AppError::new("AUTH_CHANGED", "账户已改变，请重新委托"));
        }
        Ok(())
    };
    check_account()?;
    guard.await?;
    check_account()?;
    let endpoint = format!("{}/responses/async", provider.base_url);
    let started = tokio::time::Instant::now();
    let mut request = provider
        .client
        .post(&endpoint)
        .bearer_auth(&token)
        .header("x-focalroom-ai-policy", "deepseek-flash-v1")
        .header("x-focalroom-request-id", id)
        .header("x-focalroom-image-consent", "granted")
        .timeout(Duration::from_secs(30))
        .json(body);
    if allow_queued_recovery {
        request = request.header("x-focalroom-recovery-consent", "encrypted-queue-v1");
    }
    let submitted = request.send().await;
    let mut job = match submitted {
        Ok(response) if response.status().is_success() => Some(read_job(response, id).await?),
        Ok(response) if response.status().is_client_error() => {
            return Err(http_error(response.status()));
        }
        // 提交回执丢失时查询已知编号，不重发 POST，不把 404 当作未消费的证明。
        _ => None,
    };
    let mut read_failures = 0;
    loop {
        check_account()?;
        if started.elapsed() >= Duration::from_secs(330) {
            return Err(unknown(id));
        }
        if let Some(value) = job.take() {
            if value["jobId"] != id {
                return Err(unknown(id));
            }
            match classify(
                provider,
                value,
                id,
                body["max_output_tokens"].as_u64().unwrap_or(0),
            )? {
                HostedJob::Completed(result) => return Ok(result),
                HostedJob::Pending => tokio::time::sleep(Duration::from_secs(2)).await,
                HostedJob::Rejected => {
                    return Err(AppError::new(
                        "AI_PROVIDER_REJECTED",
                        "照片分析未被接受，预留额度已释放",
                    ));
                }
                _ => return Err(unknown(id)),
            }
        }
        check_account()?;
        let response = provider
            .client
            .get(format!("{endpoint}/{id}"))
            .bearer_auth(&token)
            .timeout(Duration::from_secs(15))
            .send()
            .await;
        match response {
            Ok(response) if response.status().is_success() => {
                job = Some(read_job(response, id).await?);
                read_failures = 0;
            }
            Ok(response) if matches!(response.status().as_u16(), 401 | 403) => {
                return Err(http_error(response.status()));
            }
            _ if read_failures < 2 => {
                read_failures += 1;
                tokio::time::sleep(Duration::from_secs(2)).await;
            }
            _ => return Err(unknown(id)),
        }
    }
}

pub(super) async fn cancel(provider: &OpenAiProvider, id: &str) -> Result<bool> {
    validate_id(id)?;
    let auth = provider
        .hosted
        .as_ref()
        .ok_or_else(|| AppError::new("RECOVERY_ERROR", "这次分析不支持撤回排队"))?;
    let expected = provider
        .hosted_account
        .as_deref()
        .ok_or_else(|| AppError::new("AUTH_REQUIRED", "请先登录"))?;
    let token = auth.bearer(auth.origin()).await?;
    if auth.account_id()? != expected {
        return Err(AppError::new("AUTH_CHANGED", "账户已改变"));
    }
    let response = provider
        .client
        .post(format!("{}/responses/async/{id}/cancel", provider.base_url))
        .bearer_auth(token)
        .json(&json!({}))
        .timeout(Duration::from_secs(15))
        .send()
        .await
        .map_err(|_| unknown(id))?;
    if !response.status().is_success() {
        return Err(unknown(id));
    }
    let body = read_job(response, id).await?;
    if auth.account_id()? != expected {
        return Err(AppError::new("AUTH_CHANGED", "账户已改变"));
    }
    body["cancelled"].as_bool().ok_or_else(|| unknown(id))
}

pub(crate) enum HostedJob {
    Pending,
    Completed(Value),
    Rejected,
    Unknown,
    Expired,
}
fn validate_id(id: &str) -> Result<()> {
    if uuid::Uuid::parse_str(id).is_ok_and(|uuid| uuid.to_string() == id) {
        Ok(())
    } else {
        Err(AppError::new(
            "RECOVERY_ERROR",
            "无法识别这次分析，请联系支持",
        ))
    }
}
fn classify(
    provider: &OpenAiProvider,
    value: Value,
    id: &str,
    output_limit: u64,
) -> Result<HostedJob> {
    if value["jobId"] != id {
        return Err(unknown(id));
    }
    Ok(match value["state"].as_str() {
        Some("COMPLETED") if value["error"].is_null() => {
            let result = value["result"].clone();
            let usage = &result["usage"];
            let valid_usage = match (
                usage["input_tokens"].as_u64(),
                usage["output_tokens"].as_u64(),
                usage["total_tokens"].as_u64(),
            ) {
                (Some(input), Some(output), Some(total)) => {
                    input.checked_add(output) == Some(total)
                        && output <= output_limit
                        && total <= u64::from(u32::MAX)
                }
                _ => false,
            };
            if result["status"] != "completed"
                || result["model"] != provider.model
                || !result["error"].is_null()
                || !result["output"].is_array()
                || !valid_usage
            {
                return Err(unknown(id));
            }
            HostedJob::Completed(result)
        }
        Some("PENDING") => HostedJob::Pending,
        Some("REJECTED") => HostedJob::Rejected,
        Some("UNKNOWN") => HostedJob::Unknown,
        Some("COMPLETED") if value["error"] == "AI_RESULT_EXPIRED" => HostedJob::Expired,
        _ => return Err(unknown(id)),
    })
}
/// 恢复入口仅查询固定端点；不持有图片正文，不发送 POST，不跟随 pollUrl。
pub(super) async fn query(
    provider: &OpenAiProvider,
    id: &str,
    output_limit: u32,
) -> Result<HostedJob> {
    validate_id(id)?;
    let auth = provider
        .hosted
        .as_ref()
        .ok_or_else(|| AppError::new("RECOVERY_ERROR", "这次分析不支持查询"))?;
    let expected = provider
        .hosted_account
        .as_deref()
        .ok_or_else(|| AppError::new("AUTH_REQUIRED", "请先登录 FocalRoom"))?;
    let token = auth.bearer(auth.origin()).await?;
    if auth.account_id()? != expected {
        return Err(AppError::new(
            "AUTH_CHANGED",
            "账户已改变，请重新登录原账户",
        ));
    }
    let response = provider
        .client
        .get(format!("{}/responses/async/{id}", provider.base_url))
        .bearer_auth(token)
        .timeout(Duration::from_secs(15))
        .send()
        .await
        .map_err(|_| unknown(id))?;
    if !response.status().is_success() {
        return Err(if matches!(response.status().as_u16(), 401 | 403) {
            http_error(response.status())
        } else {
            unknown(id)
        });
    }
    let value = read_job(response, id).await?;
    if auth.account_id()? != expected {
        return Err(AppError::new(
            "AUTH_CHANGED",
            "账户已改变，请重新登录原账户",
        ));
    }
    classify(provider, value, id, u64::from(output_limit))
}
