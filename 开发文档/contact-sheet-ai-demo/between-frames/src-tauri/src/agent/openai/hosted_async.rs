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
    let id = uuid::Uuid::new_v4().to_string();
    let endpoint = format!("{}/responses/async", provider.base_url);
    let started = tokio::time::Instant::now();
    let submitted = provider
        .client
        .post(&endpoint)
        .bearer_auth(&token)
        .header("x-focalroom-ai-policy", "deepseek-flash-v1")
        .header("x-focalroom-request-id", &id)
        .header("x-focalroom-image-consent", "granted")
        .timeout(Duration::from_secs(30))
        .json(body)
        .send()
        .await;
    let mut job = match submitted {
        Ok(response) if response.status().is_success() => Some(read_job(response, &id).await?),
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
            return Err(unknown(&id));
        }
        if let Some(value) = job.take() {
            if value["jobId"] != id {
                return Err(unknown(&id));
            }
            match value["state"].as_str() {
                Some("COMPLETED") if value["error"].is_null() => {
                    let result = value["result"].clone();
                    if result["status"] != "completed"
                        || result["model"] != provider.model
                        || !result["error"].is_null()
                        || !result["output"].is_array()
                    {
                        return Err(unknown(&id));
                    }
                    return Ok(result);
                }
                Some("PENDING") => tokio::time::sleep(Duration::from_secs(2)).await,
                Some("REJECTED") => {
                    return Err(AppError::new(
                        "AI_PROVIDER_REJECTED",
                        "照片分析未被接受，预留额度已释放",
                    ));
                }
                _ => return Err(unknown(&id)),
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
                job = Some(read_job(response, &id).await?);
                read_failures = 0;
            }
            Ok(response) if matches!(response.status().as_u16(), 401 | 403) => {
                return Err(http_error(response.status()));
            }
            _ if read_failures < 2 => {
                read_failures += 1;
                tokio::time::sleep(Duration::from_secs(2)).await;
            }
            _ => return Err(unknown(&id)),
        }
    }
}
