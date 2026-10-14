use super::{context::ModelTurnRequest, provider::ModelProvider, types::*};
use crate::domain::{AppError, Result};
use serde_json::{Value, json};
use std::{future::Future, pin::Pin};

/// API key 只经环境变量进入进程内存：不写 SQLite、不进日志、不回传前端（仅脱敏尾号）。
pub const KEY_ENV: &str = "BETWEEN_FRAMES_OPENAI_API_KEY";
pub const BASE_URL_ENV: &str = "BETWEEN_FRAMES_AI_BASE_URL";
pub const MODEL_ENV: &str = "BETWEEN_FRAMES_AI_MODEL";
pub const DEFAULT_MODEL: &str = "gpt-4o-mini";
pub const DEFAULT_BASE_URL: &str = "https://api.openai.com/v1";
/// 保守费用上界估算（非账单）：每张上传图 1 美分、每千文本 token 1 美分。
pub const IMAGE_COST_MINOR_USD: u32 = 1;
pub const TEXT_COST_MINOR_PER_K_TOKENS: u32 = 1;

pub struct OpenAiProvider {
    client: reqwest::Client,
    base_url: String,
    api_key: String,
    model: String,
}
pub struct OpenAiConfig {
    pub base_url: String,
    pub api_key: String,
    pub model: String,
}
impl OpenAiConfig {
    pub fn from_env() -> Option<Self> {
        let (api_key, _source) = crate::keychain::resolve_api_key()?;
        Some(Self {
            base_url: std::env::var(BASE_URL_ENV)
                .ok()
                .map(|s| s.trim().trim_end_matches('/').to_owned())
                .filter(|s| !s.is_empty())
                .unwrap_or_else(|| DEFAULT_BASE_URL.into()),
            model: std::env::var(MODEL_ENV)
                .ok()
                .map(|s| s.trim().to_owned())
                .filter(|s| !s.is_empty())
                .unwrap_or_else(|| DEFAULT_MODEL.into()),
            api_key,
        })
    }
    pub fn api_key_tail(&self) -> String {
        self.api_key
            .chars()
            .rev()
            .take(4)
            .collect::<Vec<_>>()
            .into_iter()
            .rev()
            .collect()
    }
}
impl OpenAiProvider {
    pub fn new(config: OpenAiConfig) -> Self {
        Self {
            client: reqwest::Client::builder()
                .timeout(std::time::Duration::from_secs(120))
                .build()
                .expect("reqwest client"),
            base_url: config.base_url,
            api_key: config.api_key,
            model: config.model,
        }
    }
    pub fn model_id(&self) -> &str {
        &self.model
    }
    async fn post(&self, body: &Value) -> Result<Value> {
        let response = self
            .client
            .post(format!("{}/responses", self.base_url))
            .bearer_auth(&self.api_key)
            .json(body)
            .send()
            .await
            .map_err(|e| {
                retriable(AppError::new(
                    "MODEL_UNAVAILABLE",
                    format!("网络请求失败：{e}"),
                ))
            })?;
        let status = response.status();
        let text = response.text().await.map_err(|e| {
            retriable(AppError::new(
                "MODEL_UNAVAILABLE",
                format!("读取响应失败：{e}"),
            ))
        })?;
        if !status.is_success() {
            let is_retriable =
                status.is_server_error() || status == reqwest::StatusCode::TOO_MANY_REQUESTS;
            let mut error = AppError::new(
                "MODEL_UNAVAILABLE",
                format!("模型服务返回 {status}：{}", truncate(&text, 300)),
            );
            error.retriable = is_retriable;
            return Err(error);
        }
        serde_json::from_str(&text).map_err(crate::command_audit::serialization)
    }
    pub fn turn_body(&self, request: &ModelTurnRequest) -> Value {
        let mut input: Vec<Value> = vec![json!({
            "role": "user",
            "content": [{"type": "input_text", "text": user_brief(&request.task)}],
        })];
        for message in &request.protocol {
            match message.kind.as_str() {
                "model" => {
                    if let Some(outcome) = outcome_calls(&message.body) {
                        for call in outcome {
                            input.push(json!({
                                "type": "function_call",
                                "call_id": call["callId"],
                                "name": call["name"],
                                "arguments": call["args"].to_string(),
                            }));
                        }
                    }
                }
                "tool" => {
                    let call_id = &message.body["call"]["callId"];
                    let name = &message.body["call"]["name"];
                    let output = message.body["result"].clone();
                    input.push(json!({
                        "type": "function_call_output",
                        "call_id": call_id,
                        "output": output.to_string(),
                    }));
                    let _ = name;
                }
                _ => {}
            }
        }
        if let Some(answer) = &request.task.answer {
            input.push(json!({
                "role": "user",
                "content": [{"type": "input_text", "text": format!("人工答复：{answer}")}],
            }));
        }
        json!({
            "model": self.model,
            "instructions": request.layers["system"],
            "input": input,
            "tools": request.tool_specs,
            "store": false,
        })
    }
    /// 视觉观察子请求：固定小上下文 + 严格 JSON 输出；照片预览以 data URL 提供。
    pub async fn observe(
        &self,
        brief: &str,
        question: &str,
        images: &[(String, String)], // (photo_id, base64 jpeg)
    ) -> Result<(Value, u32)> {
        let mut content = vec![json!({
            "type": "input_text",
            "text": format!("项目意图：{}\n观察问题：{}\n共 {} 张照片，每张图像前一行标注其 photo_id；输出时原样引用。", brief, question, images.len()),
        })];
        for (photo_id, data) in images {
            content.push(json!({
                "type": "input_text",
                "text": format!("photo_id: {photo_id}"),
            }));
            content.push(json!({
                "type": "input_image",
                "image_url": format!("data:image/jpeg;base64,{data}"),
            }));
        }
        let body = json!({
            "model": self.model,
            "instructions": include_str!("../../../prompts/vision-observe-m3.md"),
            "input": [{"role": "user", "content": content}],
            "text": {"format": {"type": "json_schema", "name": "photo_observations", "strict": true, "schema": observation_schema()}},
            "store": false,
        });
        let response = self.post(&body).await?;
        let usage_tokens = tokens_of(&response);
        let text = output_text(&response);
        let mut observation: Value = serde_json::from_str(&text)
            .map_err(|_| AppError::new("MODEL_UNAVAILABLE", "视觉观察输出不符合 JSON 结构"))?;
        observation["model_id"] = json!(self.model);
        Ok((observation, usage_tokens))
    }
}
fn user_brief(task: &Task) -> String {
    format!(
        "编辑目标：{}\n项目 ID：{}\n目标张数：{}\n当前选区：{} 张",
        task.goal,
        task.project_id,
        task.target_count,
        task.selected_photo_ids.len()
    )
}
fn outcome_calls(body: &Value) -> Option<Vec<Value>> {
    let calls = body["calls"].as_array()?;
    Some(calls.clone())
}
fn output_text(response: &Value) -> String {
    response["output"]
        .as_array()
        .map(|items| {
            items
                .iter()
                .filter_map(|item| {
                    let parts = item["content"].as_array()?;
                    Some(
                        parts
                            .iter()
                            .filter(|part| part["type"] == "output_text")
                            .filter_map(|part| part["text"].as_str())
                            .collect::<Vec<_>>()
                            .join(""),
                    )
                })
                .collect::<Vec<_>>()
                .join("")
        })
        .unwrap_or_default()
}
fn tokens_of(response: &Value) -> u32 {
    response["usage"]["total_tokens"]
        .as_u64()
        .unwrap_or(0)
        .min(u32::MAX as u64) as u32
}
fn truncate(text: &str, max: usize) -> String {
    text.chars().take(max).collect()
}
fn retriable(mut error: AppError) -> AppError {
    error.retriable = true;
    error
}
pub fn observation_schema() -> Value {
    json!({
        "type": "object",
        "properties": {
            "observations": {
                "type": "array",
                "items": {
                    "type": "object",
                    "properties": {
                        "photo_id": {"type": "string"},
                        "visible_facts": {"type": "array", "items": {"type": "string"}, "minItems": 1},
                        "interpretation": {"type": "string"},
                        "uncertainty": {"type": "string"}
                    },
                    "required": ["photo_id", "visible_facts", "interpretation", "uncertainty"],
                    "additionalProperties": false
                }
            }
        },
        "required": ["observations"],
        "additionalProperties": false
    })
}
pub fn text_cost_minor_usd(tokens: u32) -> u32 {
    tokens
        .saturating_mul(TEXT_COST_MINOR_PER_K_TOKENS)
        .div_ceil(1000)
}
impl ModelProvider for OpenAiProvider {
    fn generate<'a>(
        &'a self,
        input: &'a ModelTurnRequest,
    ) -> Pin<Box<dyn Future<Output = Result<ModelOutcome>> + Send + 'a>> {
        Box::pin(async move {
            let body = self.turn_body(input);
            let response = self.post(&body).await?;
            let calls: Vec<ToolCall> = response["output"]
                .as_array()
                .map(|items| {
                    items
                        .iter()
                        .filter(|item| item["type"] == "function_call")
                        .filter_map(|item| {
                            let arguments = item["arguments"].as_str()?;
                            let args: Value = serde_json::from_str(arguments).ok()?;
                            Some(ToolCall {
                                call_id: item["call_id"].as_str()?.into(),
                                name: item["name"].as_str()?.into(),
                                args,
                            })
                        })
                        .collect()
                })
                .unwrap_or_default();
            if !calls.is_empty() {
                return Ok(ModelOutcome::ToolCalls { calls });
            }
            let text = output_text(&response);
            if text.trim().is_empty() {
                return Err(AppError::new(
                    "MODEL_UNAVAILABLE",
                    "模型未返回工具调用或文本",
                ));
            }
            Ok(ModelOutcome::NoAction { reason: text })
        })
    }
}
