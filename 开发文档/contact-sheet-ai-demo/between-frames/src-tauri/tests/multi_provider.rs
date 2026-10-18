// M3 多供应商原生适配回归：Anthropic Messages / Gemini generateContent 的
// 请求构造、响应解析、计量与能力边界（本地 mock 服务器，无真实外发）。
use between_frames_lib::agent::{
    anthropic::{AnthropicProvider, DEFAULT_BASE_URL as A_BASE},
    context::ModelTurnRequest,
    gemini::{DEFAULT_BASE_URL as G_BASE, GeminiProvider},
    openai::OpenAiConfig,
    provider::ModelProvider,
    router::{self, RegionPolicy, RouteRequest, ServiceTier, TaskType},
    types::*,
};
use serde_json::{Value, json};
use std::{
    io::{BufRead, BufReader, Write},
    net::TcpListener,
    sync::{Arc, Mutex},
    thread,
};

struct Recorded(String, String, String);

/// 极简 HTTP mock：记录最后一次请求（方法/路径/头/体），返回固定 JSON。
fn mock_server(response: Value) -> (String, Arc<Mutex<Vec<Recorded>>>, thread::JoinHandle<()>) {
    let listener = TcpListener::bind("127.0.0.1:0").unwrap();
    let port = listener.local_addr().unwrap().port();
    let seen: Arc<Mutex<Vec<Recorded>>> = Arc::new(Mutex::new(Vec::new()));
    let log = seen.clone();
    let body = response.to_string();
    let handle = thread::spawn(move || {
        for stream in listener.incoming() {
            let Ok(mut stream) = stream else { break };
            let mut reader = BufReader::new(stream.try_clone().unwrap());
            let mut request_line = String::new();
            if reader.read_line(&mut request_line).is_err() {
                break;
            }
            let mut headers = Vec::new();
            let mut content_length = 0usize;
            loop {
                let mut line = String::new();
                reader.read_line(&mut line).unwrap();
                let trimmed = line.trim_end().to_string();
                if trimmed.is_empty() {
                    break;
                }
                if let Some(len) = trimmed
                    .to_ascii_lowercase()
                    .strip_prefix("content-length:")
                    .and_then(|v| v.trim().parse().ok())
                {
                    content_length = len;
                }
                headers.push(trimmed);
            }
            let mut request_body = vec![0u8; content_length];
            if content_length > 0 {
                use std::io::Read;
                reader.read_exact(&mut request_body).unwrap();
            }
            log.lock().unwrap().push(Recorded(
                request_line.trim().to_string(),
                headers.join("\n"),
                String::from_utf8_lossy(&request_body).to_string(),
            ));
            let response_text = format!(
                "HTTP/1.1 200 OK\r\ncontent-type: application/json\r\ncontent-length: {}\r\nconnection: close\r\n\r\n{}",
                body.len(),
                body
            );
            let _ = stream.write_all(response_text.as_bytes());
            let _ = stream.flush();
        }
    });
    (format!("http://127.0.0.1:{port}"), seen, handle)
}

fn turn_request() -> ModelTurnRequest {
    let task = Task {
        id: "t1".into(),
        project_id: "p1".into(),
        goal: "给两版 2 张序列".into(),
        ..minimal_task()
    };
    ModelTurnRequest {
        layers: json!({"system": "系统提示", "task": {"goal": task.goal}}),
        protocol: vec![
            ProtocolMessage {
                kind: "model".into(),
                body: json!({"calls": [{"callId": "c1", "name": "list_photos", "args": {"limit": 5}}]}),
            },
            ProtocolMessage {
                kind: "tool".into(),
                body: json!({"name": "list_photos", "result": {"callId": "c1", "ok": true, "result": {"photos": []}}}),
            },
        ],
        tool_specs: json!({"tools": [{"name": "list_photos", "description": "列出照片", "parameters": {"type": "object", "properties": {}}}]}),
        prompt_version: "test".into(),
        task,
    }
}

fn minimal_task() -> Task {
    Task {
        id: "t1".into(),
        project_id: "p1".into(),
        goal: "目标".into(),
        base_revision: 1,
        memory_revision: 0,
        allowed_photo_ids: vec![],
        selected_photo_ids: vec![],
        target_count: 2,
        state: "running".into(),
        reason: String::new(),
        budget: Budget::default(),
        usage: Usage::default(),
        elapsed_ms: 0,
        question: None,
        answer: None,
        final_draft_ids: vec![],
        pending: None,
        provider_id: "anthropic".into(),
        consent_id: None,
        created_at: String::new(),
        updated_at: String::new(),
    }
}

#[tokio::test]
async fn anthropic_native_messages_round_trip() {
    let (base, seen, _handle) = mock_server(json!({
        "content": [
            {"type": "text", "text": "思考中"},
            {"type": "tool_use", "id": "toolu_01", "name": "list_photos", "input": {"limit": 3}}
        ],
        "usage": {"input_tokens": 120, "output_tokens": 30}
    }));
    let provider = AnthropicProvider::new(OpenAiConfig {
        base_url: base.clone(),
        api_key: "anthropic-key".into(),
        model: "claude-sonnet-4-20250514".into(),
    })
    .with_capability_profile(declared_profile(&base, "claude-sonnet-4-20250514"));
    let result = provider.generate_metered(&turn_request()).await.unwrap();
    match result.outcome {
        ModelOutcome::ToolCalls { calls } => {
            assert_eq!(calls[0].call_id, "toolu_01");
            assert_eq!(calls[0].name, "list_photos");
            assert_eq!(calls[0].args["limit"], 3);
        }
        other => panic!("应解析工具调用，实际 {other:?}"),
    }
    let usage = result.usage.unwrap();
    assert_eq!(
        (usage.input_tokens, usage.output_tokens, usage.total_tokens),
        (120, 30, 150)
    );
    // 请求格式：认证头、版本头、messages 结构与工具 schema。
    let log = seen.lock().unwrap();
    let (request_line, headers, body) = (&log[0].0, &log[0].1, &log[0].2);
    assert!(request_line.starts_with("POST /messages"));
    assert!(headers.contains("x-api-key: anthropic-key"));
    assert!(headers.contains("anthropic-version: 2023-06-01"));
    let parsed: Value = serde_json::from_str(body).unwrap();
    assert_eq!(parsed["model"], "claude-sonnet-4-20250514");
    assert_eq!(parsed["system"], "系统提示");
    assert_eq!(parsed["tools"][0]["name"], "list_photos");
    assert!(parsed["tools"][0]["input_schema"].is_object());
    // 工具历史以 tool_use/tool_result 表达。
    let kinds: Vec<&str> = parsed["messages"]
        .as_array()
        .unwrap()
        .iter()
        .flat_map(|m| {
            m["content"]
                .as_array()
                .map(|blocks| {
                    blocks
                        .iter()
                        .map(|b| b["type"].as_str().unwrap())
                        .collect::<Vec<_>>()
                })
                .unwrap_or_default()
        })
        .collect();
    assert!(kinds.contains(&"tool_use") && kinds.contains(&"tool_result"));
}

#[tokio::test]
async fn anthropic_text_only_maps_to_no_action_and_bad_usage_fails() {
    let (base, _seen, _handle) = mock_server(json!({
        "content": [{"type": "text", "text": "没有可执行动作"}],
        "usage": {"input_tokens": 10, "output_tokens": 5}
    }));
    let provider = AnthropicProvider::new(OpenAiConfig {
        base_url: base.clone(),
        api_key: "k".into(),
        model: "claude-sonnet-4-20250514".into(),
    })
    .with_capability_profile(declared_profile(&base, "claude-sonnet-4-20250514"));
    let result = provider.generate_metered(&turn_request()).await.unwrap();
    match result.outcome {
        ModelOutcome::NoAction { reason } => assert!(reason.contains("没有可执行动作")),
        other => panic!("纯文本应为 NoAction，实际 {other:?}"),
    }
    // 计量缺失 → 保留预算预留的错误而不是伪造。
    let (base2, _seen2, _h2) =
        mock_server(json!({"content": [{"type": "text", "text": "x"}], "usage": {}}));
    let provider2 = AnthropicProvider::new(OpenAiConfig {
        base_url: base2.clone(),
        api_key: "k".into(),
        model: "claude-sonnet-4-20250514".into(),
    })
    .with_capability_profile(declared_profile(&base2, "claude-sonnet-4-20250514"));
    assert!(provider2.generate_metered(&turn_request()).await.is_err());
}

#[tokio::test]
async fn gemini_native_generate_content_round_trip() {
    let (base, seen, _handle) = mock_server(json!({
        "candidates": [{"content": {"parts": [
            {"functionCall": {"name": "list_photos", "args": {"limit": 7}}}
        ]}, "finishReason": "STOP"}],
        "usageMetadata": {"promptTokenCount": 200, "candidatesTokenCount": 40}
    }));
    let provider = GeminiProvider::new(OpenAiConfig {
        base_url: base.clone(),
        api_key: "gemini-key".into(),
        model: "gemini-2.0-flash".into(),
    })
    .with_capability_profile(declared_profile(&base, "gemini-2.0-flash"));
    let result = provider.generate_metered(&turn_request()).await.unwrap();
    match result.outcome {
        ModelOutcome::ToolCalls { calls } => {
            assert_eq!(calls[0].name, "list_photos");
            assert_eq!(calls[0].args["limit"], 7);
            assert!(
                calls[0].call_id.starts_with("gemini-list_photos-"),
                "本地生成稳定 call_id"
            );
        }
        other => panic!("应解析 functionCall，实际 {other:?}"),
    }
    let usage = result.usage.unwrap();
    assert_eq!((usage.input_tokens, usage.output_tokens), (200, 40));
    let log = seen.lock().unwrap();
    let (request_line, headers, body) = (&log[0].0, &log[0].1, &log[0].2);
    assert!(request_line.starts_with("POST /models/gemini-2.0-flash:generateContent"));
    assert!(headers.contains("x-goog-api-key: gemini-key"));
    let parsed: Value = serde_json::from_str(body).unwrap();
    assert_eq!(parsed["systemInstruction"]["parts"][0]["text"], "系统提示");
    assert_eq!(
        parsed["tools"][0]["functionDeclarations"][0]["name"],
        "list_photos"
    );
    assert!(parsed["generationConfig"]["maxOutputTokens"].is_u64());
}

fn declared_profile(
    endpoint: &str,
    model: &str,
) -> between_frames_lib::agent::capabilities::CapabilityProfile {
    between_frames_lib::agent::capabilities::CapabilityProfile::declared(
        endpoint,
        model,
        &[
            between_frames_lib::agent::capabilities::Capability::ToolCalling,
            between_frames_lib::agent::capabilities::Capability::StructuredOutput,
            between_frames_lib::agent::capabilities::Capability::InstructionFollowing,
        ],
    )
}

#[test]
fn native_catalog_declares_no_vision_and_vision_tasks_are_refused() {
    for base in [A_BASE, G_BASE] {
        let resolved =
            between_frames_lib::agent::capabilities::builtin(base, "claude-sonnet-4-20250514")
                .or_else(|_| {
                    between_frames_lib::agent::capabilities::builtin(G_BASE, "gemini-2.0-flash")
                })
                .expect("官方端点应命中内置目录");
        let names = resolved.profile.names();
        assert!(names.contains(&"tool_calling"));
        assert!(
            !names.contains(&"vision"),
            "原生适配不得声明未实现的视觉能力"
        );
    }
    // 路由：视觉任务在原生供应商（无 vision 能力）上被拒绝。
    let anthropic_route = router::Route {
        provider_id: "anthropic".into(),
        model_id: "claude-sonnet-4-20250514".into(),
        residency: "unknown".into(),
        quality: "unmeasured".into(),
        est_cost_usd: 0.02,
        capabilities: vec!["tool_calling", "structured_output", "instruction_following"],
    };
    let region = RegionPolicy::default();
    let request = RouteRequest {
        task_type: TaskType::VisionObserve,
        tier: ServiceTier::Standard,
        region: &region,
        budget_remaining_usd: 1.0,
        images_required: true,
        image_consent: true,
        quality_scores: &[],
    };
    let refused = router::route_with_catalog(&request, vec![anthropic_route]);
    assert_eq!(refused.unwrap_err().code, "MODEL_CAPABILITY_MISSING");
}

#[test]
fn provider_identity_distinguishes_vendors_and_requires_new_delegation() {
    // 身份 = 供应商 + 型号 + 端点摘要：切换供应商即身份变化，旧任务检查点拒绝续接。
    let anthropic = AnthropicProvider::new(OpenAiConfig {
        base_url: A_BASE.into(),
        api_key: "k1".into(),
        model: "claude-sonnet-4-20250514".into(),
    });
    let gemini = GeminiProvider::new(OpenAiConfig {
        base_url: G_BASE.into(),
        api_key: "k2".into(),
        model: "gemini-2.0-flash".into(),
    });
    assert_ne!(anthropic.identity(), gemini.identity());
    assert_eq!(anthropic.identity().provider_id, "anthropic");
    assert_eq!(gemini.identity().provider_id, "gemini");
    // 能力配置无效时显式报错，不静默回退。
    let bad = AnthropicProvider::new(OpenAiConfig {
        base_url: "https://unknown.example/v1".into(),
        api_key: "k".into(),
        model: "claude-sonnet-4-20250514".into(),
    });
    assert!(bad.configuration_error().is_some());
}
