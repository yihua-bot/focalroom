//! 托管聊天的流式消费：把网关 /responses/stream 的 SSE 增量逐段交给调用方。
//! 鉴权、策略、额度预留与结算全部在服务端完成，与一次性路径同一套纪律；这里只负责
//! 读帧与转发。唯一允许"重发一次"的失败是网关根本没有这条路由（404/405/501）——
//! 那种请求在路由匹配阶段就被拒掉，服务端尚未预留额度。其余失败一律按结果不明处理：
//! 不重发、不退款，回执仍可查询。
use super::{OpenAiProvider, hosted_async};
use crate::domain::{AppError, Result};
use serde_json::{Value, json};
use std::time::Duration;

/// 从发出请求到拿到响应头的上限：网关要先完成鉴权、策略校验与额度预留才写 200。
const UNTIL_HEADERS: Duration = Duration::from_secs(45);
/// 帧间静默上限：超时即认为上游已断，结果不明，不能当作"没发出去"。
const IDLE: Duration = Duration::from_secs(60);
/// 正文总量上限，与图片任务回执一致，避免异常响应耗尽内存。
const MAX_TOTAL: usize = 2 * 1024 * 1024 + 4096;

pub(super) enum StreamFailure {
    /// 网关未部署该路由：调用方可原样退回一次性请求，这不构成重复计量。
    RouteMissing,
    /// 已经无法确认这次请求的结果。
    Failed(AppError),
}

fn unknown(id: &str) -> StreamFailure {
    let mut error = AppError::new("AI_RESULT_UNKNOWN", "暂未取得回复，请勿重复提交");
    error.details = Some(json!({ "requestId": id }));
    StreamFailure::Failed(error)
}

/// 逐行解析 SSE：服务端每帧写成 `data: {json}\n\n`，且 JSON 字符串内的换行是转义形式，
/// 因此按 '\n' 字节切行不会截断多字节字符。
fn handle_line(
    line: &[u8],
    id: &str,
    on_delta: &mut dyn FnMut(&str),
    done: &mut Option<Value>,
) -> std::result::Result<(), StreamFailure> {
    let text = String::from_utf8(line.to_vec()).map_err(|_| {
        StreamFailure::Failed(AppError::new("AI_RESULT_UNKNOWN", "流式响应不是合法文本"))
    })?;
    let text = text.trim();
    if text.is_empty() || !text.starts_with("data:") {
        return Ok(());
    }
    let payload = text["data:".len()..].trim();
    if payload.is_empty() || payload == "[DONE]" {
        return Ok(());
    }
    let event: Value = serde_json::from_str(payload).map_err(|_| {
        StreamFailure::Failed(AppError::new("AI_RESULT_UNKNOWN", "流式响应帧已损坏"))
    })?;
    match event["type"].as_str() {
        // start 帧回显服务端实际预留的请求编号：与本地编号不一致说明这条回复不属于本次请求。
        Some("start") if event["request_id"] != id => Err(unknown(id)),
        Some("start") => Ok(()),
        Some("delta") => {
            if let Some(fragment) = event["text"].as_str()
                && !fragment.is_empty()
            {
                on_delta(fragment);
            }
            Ok(())
        }
        // done 帧带的是服务端结算后的正式回执，最终上屏文字以它为准，增量只是提前渲染。
        Some("done") => {
            *done = Some(event["reply"].clone());
            Ok(())
        }
        Some("error") => {
            let code = event["code"].as_str().unwrap_or("AI_RESULT_UNKNOWN");
            let message = event["message"]
                .as_str()
                .filter(|message| !message.trim().is_empty())
                .unwrap_or("模型结果无法确认，请勿重复提交");
            Err(StreamFailure::Failed(AppError::new(code, message)))
        }
        // 未知帧忽略：协议往后扩展不应该打断已经到手的回复。
        _ => Ok(()),
    }
}

pub(super) async fn chat_stream(
    provider: &OpenAiProvider,
    body: &Value,
    mut on_delta: impl FnMut(&str),
) -> std::result::Result<Value, StreamFailure> {
    let auth = provider.hosted.as_ref().ok_or_else(|| {
        StreamFailure::Failed(AppError::new("AI_NOT_CONFIGURED", "Focal AI 暂未开放"))
    })?;
    let expected = provider
        .hosted_account
        .as_deref()
        .ok_or_else(|| AppError::new("AUTH_REQUIRED", "请先登录 FocalRoom"))
        .map_err(StreamFailure::Failed)?;
    let id = uuid::Uuid::new_v4().to_string();
    let token = auth
        .bearer(auth.origin())
        .await
        .map_err(StreamFailure::Failed)?;
    // 与一次性路径相同：账户中途换号就停在这里，不把上一条回复记到新账户名下。
    let check_account = || -> Result<()> {
        if auth.account_id()? != expected {
            return Err(AppError::new("AUTH_CHANGED", "账户已改变，请重新委托"));
        }
        Ok(())
    };
    check_account().map_err(StreamFailure::Failed)?;
    // 请求级只限"拿到响应头"的时间；正文另用帧间静默上限判断。共享 client 自带 120 秒
    // 总超时，与一次性聊天路径完全同一口径，流式不会比原来更容易被掐断。
    let sent = tokio::time::timeout(
        UNTIL_HEADERS,
        provider
            .client
            .post(format!("{}/responses/stream", provider.base_url))
            .bearer_auth(&token)
            .header("x-focalroom-ai-policy", "deepseek-flash-v1")
            .header("x-focalroom-request-id", id.clone())
            .header("x-focalroom-image-consent", "granted")
            .json(body)
            .send(),
    )
    .await;
    let mut response = match sent {
        // 没拿到响应头时无法证明服务端没有预留额度，只能按结果不明处理，不重发。
        Err(_) | Ok(Err(_)) => return Err(unknown(&id)),
        Ok(Ok(response)) => response,
    };
    let status = response.status();
    if !status.is_success() {
        return Err(if matches!(status.as_u16(), 404 | 405 | 501) {
            StreamFailure::RouteMissing
        } else {
            StreamFailure::Failed(hosted_async::http_error(status))
        });
    }
    let mut buffer: Vec<u8> = Vec::new();
    let mut done: Option<Value> = None;
    loop {
        // 拿到 200 之后每一段都按静默上限计时：60 秒没有任何字节就判定结果不明。
        let chunk = match tokio::time::timeout(IDLE, response.chunk()).await {
            Err(_) | Ok(Err(_)) => return Err(unknown(&id)),
            Ok(Ok(None)) => break,
            Ok(Ok(Some(bytes))) => bytes,
        };
        if buffer.len() + chunk.len() > MAX_TOTAL {
            return Err(unknown(&id));
        }
        buffer.extend_from_slice(&chunk);
        while let Some(position) = buffer.iter().position(|byte| *byte == b'\n') {
            let line = buffer.drain(..=position).collect::<Vec<u8>>();
            handle_line(&line, &id, &mut on_delta, &mut done)?;
        }
    }
    // 收尾仍有残留说明最后一个帧被截断；正文结束却没有 done 帧同样算没走完。
    let reply = if !buffer.is_empty() { None } else { done };
    let reply = reply.ok_or_else(|| unknown(&id))?;
    if reply["status"] != "completed"
        || reply["model"] != provider.model
        || !reply["error"].is_null()
        || !reply["output"].is_array()
    {
        return Err(unknown(&id));
    }
    check_account().map_err(StreamFailure::Failed)?;
    Ok(reply)
}
