// M3 视觉链路集成测试：本地 HTTP 模拟 Responses API。
// 覆盖：AG-04 无授权零外发、授权后真实观察与计量、缓存命中零上传、
// 幻觉 photo_id 丢弃且配额按真实传输扣减、图像预算触顶暂停。
use between_frames_lib::{
    agent::{
        self, consent, openai::OpenAiConfig, openai::OpenAiProvider, runtime::AgentEngine, types::*,
    },
    command_handler::CommandHandler,
    db::Store,
    domain::{self, Project},
    image_pipeline,
};
use serde_json::{Value, json};
use std::{
    io::{Read, Write},
    net::TcpListener,
    path::Path,
    sync::{Arc, Mutex},
};

static ENV_LOCK: std::sync::LazyLock<tokio::sync::Mutex<()>> =
    std::sync::LazyLock::new(|| tokio::sync::Mutex::new(()));

async fn setup(path: &Path, count: usize) -> (Store, Project) {
    let db = Store::open(&path.join("projects.sqlite")).await.unwrap();
    let p = db.create("M3", "视觉与授权").await.unwrap();
    for i in 0..count {
        let source = path.join(format!("f{i}.jpg"));
        image::RgbImage::from_pixel(32, 24, image::Rgb([(i as u8).wrapping_mul(13), 42, 90]))
            .save(&source)
            .unwrap();
        db.insert_photo(
            &p.id,
            &image_pipeline::prepare(&source, &path.join("cache")).unwrap(),
        )
        .await
        .unwrap();
    }
    let p = db.get(&p.id).await.unwrap();
    (db, p)
}

#[derive(Clone, Copy)]
enum VisionReply {
    Valid,
    Hallucinate,
    Delayed,
    InvalidJson,
    Timeout,
    UnexpectedUsage,
}
struct Script {
    main: Vec<String>, // 按主循环请求序号返回的语义：tool:<name> 或 text
    vision: VisionReply,
}
struct State {
    requests: Vec<Value>,
    main_served: usize,
}
fn spawn_server(script: Script) -> (String, Arc<Mutex<State>>) {
    let listener = TcpListener::bind("127.0.0.1:0").unwrap();
    let addr = listener.local_addr().unwrap();
    let state = Arc::new(Mutex::new(State {
        requests: vec![],
        main_served: 0,
    }));
    let s = state.clone();
    std::thread::spawn(move || {
        for stream in listener.incoming() {
            let Ok(mut stream) = stream else { break };
            let s = s.clone();
            let script_main = script.main.clone();
            let vision = matches!(
                script.vision,
                VisionReply::Valid
                    | VisionReply::Delayed
                    | VisionReply::Timeout
                    | VisionReply::UnexpectedUsage
            );
            let delayed = matches!(script.vision, VisionReply::Delayed);
            let invalid_json = matches!(script.vision, VisionReply::InvalidJson);
            let timeout = matches!(script.vision, VisionReply::Timeout);
            let unexpected_usage = matches!(script.vision, VisionReply::UnexpectedUsage);
            std::thread::spawn(move || {
                let Ok(body) = read_body(&mut stream) else {
                    return;
                };
                let parsed: Value = serde_json::from_str(&body).unwrap_or_default();
                let is_vision = parsed["text"]["format"].is_object();
                // 严格验证真实 Provider 的请求关联，而不是只读 output 内容放过 null call_id。
                if !is_vision {
                    let inputs = parsed["input"].as_array().unwrap();
                    let mut calls = std::collections::HashSet::new();
                    for item in inputs {
                        if item["type"] == "function_call" {
                            calls.insert(item["call_id"].as_str().unwrap().to_owned());
                        } else if item["type"] == "function_call_output" {
                            assert!(
                                calls.remove(
                                    item["call_id"].as_str().expect("回执 call_id 必须为字符串")
                                ),
                                "工具回执必须关联此前唯一调用"
                            );
                        }
                    }
                    assert!(calls.is_empty(), "请求历史必须保持完整调用-回执关联");
                }
                s.lock().unwrap().requests.push(parsed);
                let response = if is_vision {
                    if timeout {
                        std::thread::sleep(std::time::Duration::from_millis(1500));
                    }
                    if delayed {
                        std::thread::sleep(std::time::Duration::from_millis(400));
                    }
                    if invalid_json {
                        envelope(
                            json!([{"type":"message","content":[{"type":"output_text","text":"invalid-json"}]}]),
                        )
                    } else {
                        let response = vision_response(&body, vision);
                        if unexpected_usage {
                            let mut value: Value = serde_json::from_str(&response).unwrap();
                            value["usage"]["total_tokens"] = json!(100000);
                            value.to_string()
                        } else {
                            response
                        }
                    }
                } else {
                    let index = s.lock().unwrap().main_served;
                    s.lock().unwrap().main_served += 1;
                    main_response(&body, script_main.get(index).cloned())
                };
                let reply = format!(
                    "HTTP/1.1 200 OK\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{}",
                    response.len(),
                    response
                );
                let _ = stream.write_all(reply.as_bytes());
                let _ = stream.flush();
            });
        }
    });
    (format!("http://{addr}"), state)
}

async fn grant(db: &Store, project: &Project, mode: &str) -> ConsentView {
    consent::set(
        db,
        consent::SetConsent {
            request_id: domain::id(),
            project_id: project.id.clone(),
            mode: mode.into(),
            provider: "openai".into(),
            quota: 10,
        },
    )
    .await
    .unwrap()
}

#[tokio::test]
async fn zero_cost_and_exhausted_tool_budget_stop_before_upload() {
    let _guard = ENV_LOCK.lock().await;
    unsafe { std::env::set_var("BETWEEN_FRAMES_OPENAI_API_KEY", "test-key-abcd") };
    for (cost, tools) in [(0, 40), (100, 1), (30, 40)] {
        let temp = tempfile::tempdir().unwrap();
        let (db, p) = setup(temp.path(), 2).await;
        let consent = grant(&db, &p, "project").await;
        let (base, state) = spawn_server(Script {
            main: vec![
                "tool:list_photos".into(),
                "tool:inspect_from_history".into(),
            ],
            vision: VisionReply::Valid,
        });
        let task = CommandHandler::new(&db)
            .agent_request(AgentRequest {
                request_id: domain::id(),
                project_id: p.id.clone(),
                command: TaskCommand::StartTask {
                    goal: "视觉检查".into(),
                    selected_photo_ids: p.photos.iter().map(|p| p.id.clone()).collect(),
                    target_count: 2,
                    budget: Budget {
                        max_cloud_images: 2,
                        max_cost_minor_usd: cost,
                        max_tool_calls: tools,
                        ..Budget::default()
                    },
                    consent_id: Some(consent.id),
                },
            })
            .await
            .unwrap();
        engine(&db, &base).run(&task.id).await.unwrap();
        let view = agent::projection(&db, &p.id).await.unwrap();
        assert_eq!(view.tasks[0].state, "paused");
        assert_eq!(image_requests(&state.lock().unwrap()), 0);
        assert_eq!(
            consent::effective_view(&db, &p.id, "openai")
                .await
                .unwrap()
                .unwrap()
                .used,
            0
        );
        assert!(view.tasks[0].usage.cost_minor_usd <= cost);
    }
    unsafe { std::env::remove_var("BETWEEN_FRAMES_OPENAI_API_KEY") };
}

#[tokio::test]
async fn task_consent_is_bound_once_and_never_used_as_project_fallback() {
    let _guard = ENV_LOCK.lock().await;
    unsafe { std::env::set_var("BETWEEN_FRAMES_OPENAI_API_KEY", "test-key-abcd") };
    let temp = tempfile::tempdir().unwrap();
    let (db, p) = setup(temp.path(), 2).await;
    let consent = grant(&db, &p, "task").await;
    let ids: Vec<_> = p.photos.iter().map(|p| p.id.clone()).collect();
    let (base, state) = spawn_server(Script {
        main: vec![
            "tool:list_photos".into(),
            "tool:inspect_from_history".into(),
            "text".into(),
            "tool:list_photos".into(),
            "tool:inspect_from_history".into(),
            "text".into(),
        ],
        vision: VisionReply::Valid,
    });
    let first = start_vision_task(&db, &p, ids.clone(), 2, Some(consent.id)).await;
    let owner: String = sqlx::query_scalar("SELECT task_id FROM privacy_consents WHERE id=?")
        .bind(consent.id)
        .fetch_one(&db.pool)
        .await
        .unwrap();
    assert_eq!(owner, first.id);
    engine(&db, &base).run(&first.id).await.unwrap();
    let reused = CommandHandler::new(&db)
        .agent_request(AgentRequest {
            request_id: domain::id(),
            project_id: p.id.clone(),
            command: TaskCommand::StartTask {
                goal: "第二次委托".into(),
                selected_photo_ids: ids.clone(),
                target_count: 2,
                budget: Budget {
                    max_cloud_images: 2,
                    ..Budget::default()
                },
                consent_id: Some(consent.id),
            },
        })
        .await
        .unwrap_err();
    assert_eq!(reused.code, "PERMISSION_DENIED");
    // 去掉观察缓存，证明第二个任务不能借用旧授权外发新的请求。
    let second = start_vision_task(&db, &p, ids, 2, None).await;
    sqlx::query("UPDATE photo_observations SET prompt_version='test-old'")
        .execute(&db.pool)
        .await
        .unwrap();
    engine(&db, &base).run(&second.id).await.unwrap();
    assert_eq!(image_requests(&state.lock().unwrap()), 1);
    assert_eq!(
        agent::projection(&db, &p.id).await.unwrap().tasks[0]
            .usage
            .cloud_images,
        0
    );
    unsafe { std::env::remove_var("BETWEEN_FRAMES_OPENAI_API_KEY") };
}

#[tokio::test]
async fn paused_upload_settles_and_replays_after_restart_without_retransmission() {
    let _guard = ENV_LOCK.lock().await;
    unsafe { std::env::set_var("BETWEEN_FRAMES_OPENAI_API_KEY", "test-key-abcd") };
    let temp = tempfile::tempdir().unwrap();
    let (db, p) = setup(temp.path(), 2).await;
    let consent = grant(&db, &p, "project").await;
    let (base, state) = spawn_server(Script {
        main: vec![
            "tool:list_photos".into(),
            "tool:inspect_from_history".into(),
            "text".into(),
        ],
        vision: VisionReply::Delayed,
    });
    let task = start_vision_task(
        &db,
        &p,
        p.photos.iter().map(|p| p.id.clone()).collect(),
        2,
        Some(consent.id),
    )
    .await;
    let running = engine(&db, &base);
    let task_id = task.id.clone();
    let worker = tokio::spawn(async move { running.run(&task_id).await });
    for _ in 0..100 {
        if image_requests(&state.lock().unwrap()) > 0 {
            break;
        }
        tokio::time::sleep(std::time::Duration::from_millis(10)).await;
    }
    assert_eq!(image_requests(&state.lock().unwrap()), 1);
    CommandHandler::new(&db)
        .agent_request(AgentRequest {
            request_id: domain::id(),
            project_id: p.id.clone(),
            command: TaskCommand::PauseTask {
                task_id: task.id.clone(),
            },
        })
        .await
        .unwrap();
    assert_eq!(worker.await.unwrap().unwrap_err().code, "CANCELLED");
    db.pool.close().await;
    let db = Store::open(&temp.path().join("projects.sqlite"))
        .await
        .unwrap();
    CommandHandler::new(&db).recover_agents().await.unwrap();
    CommandHandler::new(&db)
        .agent_request(AgentRequest {
            request_id: domain::id(),
            project_id: p.id.clone(),
            command: TaskCommand::ResumeTask {
                task_id: task.id.clone(),
                budget: task.budget.clone(),
            },
        })
        .await
        .unwrap();
    engine(&db, &base).run(&task.id).await.unwrap();
    assert_eq!(image_requests(&state.lock().unwrap()), 1);
    let view = agent::projection(&db, &p.id).await.unwrap();
    assert_eq!(view.tasks[0].state, "completed");
    assert_eq!(view.tasks[0].usage.cloud_images, 2);
    let request = agent::context::ContextBuilder::build(&db, &task.id)
        .await
        .unwrap();
    let provider = OpenAiProvider::new(OpenAiConfig {
        base_url: base,
        api_key: "fake".into(),
        model: "test".into(),
    });
    let body = provider.turn_body(&request);
    assert!(
        body["input"]
            .as_array()
            .unwrap()
            .iter()
            .filter(|i| i["type"] == "function_call_output")
            .all(|i| i["call_id"].is_string())
    );
    assert_eq!(body["max_output_tokens"], task.budget.max_output_tokens);
    unsafe { std::env::remove_var("BETWEEN_FRAMES_OPENAI_API_KEY") };
}

#[tokio::test]
async fn malformed_vision_response_keeps_upload_reservation_instead_of_free_retry() {
    let _guard = ENV_LOCK.lock().await;
    unsafe { std::env::set_var("BETWEEN_FRAMES_OPENAI_API_KEY", "test-key-abcd") };
    let temp = tempfile::tempdir().unwrap();
    let (db, p) = setup(temp.path(), 2).await;
    let consent = grant(&db, &p, "project").await;
    let (base, state) = spawn_server(Script {
        main: vec![
            "tool:list_photos".into(),
            "tool:inspect_from_history".into(),
        ],
        vision: VisionReply::InvalidJson,
    });
    let task = start_vision_task(
        &db,
        &p,
        p.photos.iter().map(|p| p.id.clone()).collect(),
        2,
        Some(consent.id),
    )
    .await;
    assert!(engine(&db, &base).run(&task.id).await.is_err());
    let view = agent::projection(&db, &p.id).await.unwrap();
    assert_eq!(view.tasks[0].state, "failed");
    assert_eq!(view.tasks[0].usage.cloud_images, 2);
    assert!(view.tasks[0].usage.cost_minor_usd > 0);
    assert_eq!(
        consent::effective_view(&db, &p.id, "openai")
            .await
            .unwrap()
            .unwrap()
            .used,
        2
    );
    assert_eq!(image_requests(&state.lock().unwrap()), 1);
    unsafe { std::env::remove_var("BETWEEN_FRAMES_OPENAI_API_KEY") };
}

#[tokio::test]
async fn timed_out_upload_is_charged_and_resume_does_not_send_it_again() {
    let _guard = ENV_LOCK.lock().await;
    unsafe { std::env::set_var("BETWEEN_FRAMES_OPENAI_API_KEY", "test-key-abcd") };
    let temp = tempfile::tempdir().unwrap();
    let (db, p) = setup(temp.path(), 2).await;
    let consent = grant(&db, &p, "project").await;
    let (base, state) = spawn_server(Script {
        main: vec![
            "tool:list_photos".into(),
            "tool:inspect_from_history".into(),
        ],
        vision: VisionReply::Timeout,
    });
    let task = CommandHandler::new(&db)
        .agent_request(AgentRequest {
            request_id: domain::id(),
            project_id: p.id.clone(),
            command: TaskCommand::StartTask {
                goal: "视觉检查".into(),
                selected_photo_ids: p.photos.iter().map(|p| p.id.clone()).collect(),
                target_count: 2,
                budget: Budget {
                    max_duration_seconds: 1,
                    max_cloud_images: 2,
                    ..Budget::default()
                },
                consent_id: Some(consent.id),
            },
        })
        .await
        .unwrap();
    engine(&db, &base).run(&task.id).await.unwrap();
    let paused = agent::projection(&db, &p.id).await.unwrap();
    assert_eq!(paused.tasks[0].state, "paused");
    assert_eq!(paused.tasks[0].usage.cloud_images, 2);
    assert!(paused.tasks[0].elapsed_ms >= 1000);
    db.pool.close().await;
    let db = Store::open(&temp.path().join("projects.sqlite"))
        .await
        .unwrap();
    CommandHandler::new(&db)
        .agent_request(AgentRequest {
            request_id: domain::id(),
            project_id: p.id.clone(),
            command: TaskCommand::ResumeTask {
                task_id: task.id.clone(),
                budget: Budget {
                    max_duration_seconds: 10,
                    ..task.budget
                },
            },
        })
        .await
        .unwrap();
    engine(&db, &base).run(&task.id).await.unwrap();
    assert_eq!(image_requests(&state.lock().unwrap()), 1);
    assert_eq!(
        agent::projection(&db, &p.id).await.unwrap().tasks[0].state,
        "paused"
    );
    unsafe { std::env::remove_var("BETWEEN_FRAMES_OPENAI_API_KEY") };
}

#[tokio::test]
async fn crashed_in_flight_upload_keeps_reservation_and_cannot_retransmit_on_resume() {
    let _guard = ENV_LOCK.lock().await;
    unsafe { std::env::set_var("BETWEEN_FRAMES_OPENAI_API_KEY", "test-key-abcd") };
    let temp = tempfile::tempdir().unwrap();
    let (db, p) = setup(temp.path(), 2).await;
    let consent = grant(&db, &p, "project").await;
    let (base, state) = spawn_server(Script {
        main: vec![
            "tool:list_photos".into(),
            "tool:inspect_from_history".into(),
        ],
        vision: VisionReply::Delayed,
    });
    let task = start_vision_task(
        &db,
        &p,
        p.photos.iter().map(|p| p.id.clone()).collect(),
        2,
        Some(consent.id),
    )
    .await;
    let runtime = engine(&db, &base);
    let id = task.id.clone();
    let worker = tokio::spawn(async move { runtime.run(&id).await });
    for _ in 0..100 {
        if image_requests(&state.lock().unwrap()) > 0 {
            break;
        }
        tokio::time::sleep(std::time::Duration::from_millis(10)).await;
    }
    assert_eq!(image_requests(&state.lock().unwrap()), 1);
    worker.abort();
    assert!(worker.await.unwrap_err().is_cancelled());
    let before = agent::projection(&db, &p.id).await.unwrap();
    assert_eq!(before.tasks[0].usage.cloud_images, 2);
    db.pool.close().await;
    let db = Store::open(&temp.path().join("projects.sqlite"))
        .await
        .unwrap();
    CommandHandler::new(&db).recover_agents().await.unwrap();
    assert_eq!(
        agent::projection(&db, &p.id).await.unwrap().tasks[0].state,
        "interrupted"
    );
    CommandHandler::new(&db)
        .agent_request(AgentRequest {
            request_id: domain::id(),
            project_id: p.id.clone(),
            command: TaskCommand::ResumeTask {
                task_id: task.id.clone(),
                budget: task.budget,
            },
        })
        .await
        .unwrap();
    engine(&db, &base).run(&task.id).await.unwrap();
    let after = agent::projection(&db, &p.id).await.unwrap();
    assert_eq!(after.tasks[0].state, "paused");
    assert!(after.tasks[0].reason.contains("尚未确认"));
    assert_eq!(after.tasks[0].usage.cloud_images, 2);
    assert_eq!(
        after.tasks[0].usage.cost_minor_usd,
        before.tasks[0].usage.cost_minor_usd
    );
    assert_eq!(image_requests(&state.lock().unwrap()), 1);
    assert_eq!(db.get(&p.id).await.unwrap().revision, p.revision);
    unsafe { std::env::remove_var("BETWEEN_FRAMES_OPENAI_API_KEY") };
}

#[tokio::test]
async fn unexpected_provider_usage_is_visible_and_stops_further_requests() {
    let _guard = ENV_LOCK.lock().await;
    unsafe { std::env::set_var("BETWEEN_FRAMES_OPENAI_API_KEY", "test-key-abcd") };
    let temp = tempfile::tempdir().unwrap();
    let (db, p) = setup(temp.path(), 2).await;
    let consent = grant(&db, &p, "project").await;
    let (base, state) = spawn_server(Script {
        main: vec![
            "tool:list_photos".into(),
            "tool:inspect_from_history".into(),
            "text".into(),
        ],
        vision: VisionReply::UnexpectedUsage,
    });
    let task = start_vision_task(
        &db,
        &p,
        p.photos.iter().map(|p| p.id.clone()).collect(),
        2,
        Some(consent.id),
    )
    .await;
    assert_eq!(
        engine(&db, &base).run(&task.id).await.unwrap_err().code,
        "CANCELLED"
    );
    let view = agent::projection(&db, &p.id).await.unwrap();
    assert_eq!(view.tasks[0].state, "paused");
    assert!(view.tasks[0].reason.contains("超出预留"));
    assert!(
        view.tasks[0].usage.cost_minor_usd >= 102,
        "不得通过截断账目隐瞒供应商用量偏差"
    );
    let ledger_cost: i64 =
        sqlx::query_scalar("SELECT SUM(cost_minor_usd) FROM model_usage WHERE task_id=?")
            .bind(&task.id)
            .fetch_one(&db.pool)
            .await
            .unwrap();
    assert_eq!(ledger_cost, i64::from(view.tasks[0].usage.cost_minor_usd));
    assert_eq!(state.lock().unwrap().requests.len(), 3);
    unsafe { std::env::remove_var("BETWEEN_FRAMES_OPENAI_API_KEY") };
}
fn read_body(stream: &mut std::net::TcpStream) -> std::io::Result<String> {
    let mut header = Vec::new();
    let mut byte = [0u8; 1];
    loop {
        stream.read_exact(&mut byte)?;
        header.push(byte[0]);
        if header.ends_with(b"\r\n\r\n") {
            break;
        }
    }
    let headers = String::from_utf8_lossy(&header);
    let length: usize = headers
        .lines()
        .find_map(|line| {
            let (name, value) = line.split_once(':')?;
            name.eq_ignore_ascii_case("content-length")
                .then(|| value.trim().parse().ok())?
        })
        .unwrap_or(0);
    let mut body = vec![0u8; length];
    stream.read_exact(&mut body)?;
    Ok(String::from_utf8_lossy(&body).into_owned())
}
fn envelope(output: Value) -> String {
    json!({"output": output, "usage": {"total_tokens": 120}}).to_string()
}
fn main_response(body: &str, step: Option<String>) -> String {
    let parsed: Value = serde_json::from_str(body).unwrap_or_default();
    let project_id = parsed["input"]
        .as_array()
        .and_then(|items| {
            items.iter().find_map(|item| {
                item["content"]
                    .as_array()?
                    .iter()
                    .filter_map(|part| part["text"].as_str())
                    .find_map(|text| text.split("项目 ID：").nth(1))
                    .map(|rest| rest.split_whitespace().next().unwrap_or("").to_owned())
            })
        })
        .unwrap_or_default();
    let history_ids: Vec<String> = parsed["input"]
        .as_array()
        .map(|items| {
            items
                .iter()
                .filter(|item| item["type"] == "function_call_output")
                .filter_map(|item| item["output"].as_str())
                .filter_map(|output| serde_json::from_str::<Value>(output).ok())
                .flat_map(|output| {
                    output["result"]["photos"]
                        .as_array()
                        .cloned()
                        .unwrap_or_default()
                        .into_iter()
                        .filter_map(|photo| photo["photo_id"].as_str().map(String::from))
                        .collect::<Vec<_>>()
                })
                .collect()
        })
        .unwrap_or_default();
    match step.as_deref() {
        Some("tool:list_photos") => envelope(
            json!([{"type":"function_call","call_id":"call-list","name":"list_photos","arguments":json!({"project_id": project_id, "status": null, "limit": 100, "cursor": null}).to_string()}]),
        ),
        Some("tool:inspect_from_history") if history_ids.len() >= 2 => envelope(
            json!([{"type":"function_call","call_id":"call-inspect","name":"inspect_photos","arguments":json!({"project_id": project_id, "photo_ids": [history_ids[0], history_ids[1]]}).to_string()}]),
        ),
        _ => envelope(
            json!([{"type":"message","content":[{"type":"output_text","text":"观察完成，没有进一步操作。"}]}]),
        ),
    }
}
fn vision_response(body: &str, valid: bool) -> String {
    let parsed: Value = serde_json::from_str(body).unwrap_or_default();
    let mut ids: Vec<String> = vec![];
    if let Some(items) = parsed["input"].as_array() {
        for item in items {
            if let Some(parts) = item["content"].as_array() {
                for part in parts {
                    if part["type"] == "input_text"
                        && let Some(text) = part["text"].as_str()
                        && let Some(id) = text.strip_prefix("photo_id: ")
                    {
                        ids.push(id.trim().to_owned());
                    }
                }
            }
        }
    }
    let observations: Vec<Value> = if valid {
        ids.iter()
            .map(|id| {
                json!({"photo_id": id, "visible_facts": ["画面包含深色主体与均匀背景"], "interpretation": "可能在项目中承担节奏停顿", "uncertainty": "低分辨率预览，细节有限"})
            })
            .collect()
    } else {
        vec![
            json!({"photo_id": "photo-hallucinated", "visible_facts": ["编造"], "interpretation": "无效", "uncertainty": "无"}),
        ]
    };
    let text = json!({"observations": observations}).to_string();
    envelope(json!([{"type":"message","content":[{"type":"output_text","text": text}]}]))
}

async fn start_vision_task(
    db: &Store,
    p: &Project,
    photo_ids: Vec<String>,
    cloud: u32,
    consent_id: Option<i64>,
) -> Task {
    CommandHandler::new(db)
        .agent_request(AgentRequest {
            request_id: domain::id(),
            project_id: p.id.clone(),
            command: TaskCommand::StartTask {
                goal: "用视觉观察前两张并总结".into(),
                selected_photo_ids: photo_ids,
                target_count: 2,
                budget: Budget {
                    max_cloud_images: cloud,
                    ..Budget::default()
                },
                consent_id,
            },
        })
        .await
        .unwrap()
}
fn engine(db: &Store, base_url: &str) -> AgentEngine {
    AgentEngine::with_providers(
        db.clone(),
        Arc::new(between_frames_lib::agent::provider::MockProvider),
        Some(Arc::new(OpenAiProvider::new(OpenAiConfig {
            base_url: base_url.into(),
            api_key: "test-key-abcd".into(),
            model: "test-model".into(),
        }))),
    )
}
fn image_requests(state: &State) -> usize {
    state
        .requests
        .iter()
        .filter(|r| {
            r["input"].as_array().is_some_and(|items| {
                items.iter().any(|item| {
                    item["content"]
                        .as_array()
                        .is_some_and(|parts| parts.iter().any(|p| p["type"] == "input_image"))
                })
            })
        })
        .count()
}

#[tokio::test]
async fn ag04_without_consent_no_image_leaves_formal_state_untouched() {
    let _guard = ENV_LOCK.lock().await;
    unsafe { std::env::set_var("BETWEEN_FRAMES_OPENAI_API_KEY", "test-key-abcd") };
    let temp = tempfile::tempdir().unwrap();
    let (db, p) = setup(temp.path(), 3).await;
    let ids: Vec<String> = p.photos.iter().map(|photo| photo.id.clone()).collect();
    let (base, state) = spawn_server(Script {
        main: vec![
            "tool:list_photos".into(),
            "tool:inspect_from_history".into(),
            "text".into(),
        ],
        vision: VisionReply::Valid,
    });
    let task = start_vision_task(&db, &p, ids[..2].to_vec(), 2, None).await;
    assert_eq!(task.provider_id, "openai");
    engine(&db, &base).run(&task.id).await.unwrap();
    let projection = agent::projection(&db, &p.id).await.unwrap();
    assert_eq!(projection.tasks[0].state, "completed");
    // AG-04：未授权 → 零图像外发；视觉工具降级为元数据对照并明示。
    assert_eq!(image_requests(&state.lock().unwrap()), 0);
    let result: String = sqlx::query_scalar(
        "SELECT result_json FROM tool_calls WHERE task_id=? AND tool_name='inspect_photos'",
    )
    .bind(&task.id)
    .fetch_one(&db.pool)
    .await
    .unwrap();
    let result: Value = serde_json::from_str(&result).unwrap();
    assert_eq!(result["result"]["mode"], "mock_metadata_only");
    assert_eq!(result["result"]["photos"].as_array().map(Vec::len), Some(2));
    // 主循环确实走了真实模型（两次文本/工具请求），但没有任何图像内容。
    assert!(state.lock().unwrap().requests.len() >= 2);
    unsafe { std::env::remove_var("BETWEEN_FRAMES_OPENAI_API_KEY") };
}

#[tokio::test]
async fn consented_vision_uploads_once_then_cache_hits_without_reupload() {
    let _guard = ENV_LOCK.lock().await;
    unsafe { std::env::set_var("BETWEEN_FRAMES_OPENAI_API_KEY", "test-key-abcd") };
    let temp = tempfile::tempdir().unwrap();
    let (db, p) = setup(temp.path(), 3).await;
    let ids: Vec<String> = p.photos.iter().map(|photo| photo.id.clone()).collect();
    let (base, state) = spawn_server(Script {
        main: vec![
            "tool:list_photos".into(),
            "tool:inspect_from_history".into(),
            "text".into(),
            "tool:list_photos".into(),
            "tool:inspect_from_history".into(),
            "text".into(),
        ],
        vision: VisionReply::Valid,
    });
    let consent = consent::set(
        &db,
        consent::SetConsent {
            request_id: domain::id(),
            project_id: p.id.clone(),
            mode: "project".into(),
            provider: "openai".into(),
            quota: 100,
        },
    )
    .await
    .unwrap();
    // 任务一：上传 2 张并落观察缓存，用量与费用入账。
    let task = start_vision_task(&db, &p, ids[..2].to_vec(), 2, Some(consent.id)).await;
    engine(&db, &base).run(&task.id).await.unwrap();
    let projection = agent::projection(&db, &p.id).await.unwrap();
    assert_eq!(projection.tasks[0].state, "completed");
    assert_eq!(projection.tasks[0].usage.cloud_images, 2);
    assert!(projection.tasks[0].usage.cost_minor_usd >= 2);
    let result: String = sqlx::query_scalar(
        "SELECT result_json FROM tool_calls WHERE task_id=? AND tool_name='inspect_photos'",
    )
    .bind(&task.id)
    .fetch_one(&db.pool)
    .await
    .unwrap();
    let result: Value = serde_json::from_str(&result).unwrap();
    assert_eq!(result["result"]["mode"], "vision_observed");
    assert_eq!(result["result"]["uploaded_images"], 2);
    let observations: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM photo_observations")
        .fetch_one(&db.pool)
        .await
        .unwrap();
    assert_eq!(observations, 2);
    let used = consent::effective_view(&db, &p.id, "openai")
        .await
        .unwrap()
        .unwrap()
        .used;
    assert_eq!(used, 2);
    // 任务二：同照片缓存命中 → 不再上传、不再扣配额。
    let task2 = start_vision_task(&db, &p, ids[..2].to_vec(), 2, Some(consent.id)).await;
    engine(&db, &base).run(&task2.id).await.unwrap();
    assert_eq!(image_requests(&state.lock().unwrap()), 1);
    let projection = agent::projection(&db, &p.id).await.unwrap();
    // 投影按创建时间倒序：tasks[0] 是任务二。
    assert_eq!(projection.tasks[0].usage.cloud_images, 0);
    let used2 = consent::effective_view(&db, &p.id, "openai")
        .await
        .unwrap()
        .unwrap()
        .used;
    assert_eq!(used2, 2);
    unsafe { std::env::remove_var("BETWEEN_FRAMES_OPENAI_API_KEY") };
}

#[tokio::test]
async fn hallucinated_photo_ids_are_dropped_and_quota_charged_for_real_transfer() {
    let _guard = ENV_LOCK.lock().await;
    unsafe { std::env::set_var("BETWEEN_FRAMES_OPENAI_API_KEY", "test-key-abcd") };
    let temp = tempfile::tempdir().unwrap();
    let (db, p) = setup(temp.path(), 2).await;
    let ids: Vec<String> = p.photos.iter().map(|photo| photo.id.clone()).collect();
    let (base, _state) = spawn_server(Script {
        main: vec![
            "tool:list_photos".into(),
            "tool:inspect_from_history".into(),
        ],
        vision: VisionReply::Hallucinate,
    });
    let consent = consent::set(
        &db,
        consent::SetConsent {
            request_id: domain::id(),
            project_id: p.id.clone(),
            mode: "project".into(),
            provider: "openai".into(),
            quota: 100,
        },
    )
    .await
    .unwrap();
    let task = start_vision_task(&db, &p, ids, 2, Some(consent.id)).await;
    let outcome = engine(&db, &base).run(&task.id).await;
    assert!(outcome.is_err());
    let projection = agent::projection(&db, &p.id).await.unwrap();
    assert_eq!(projection.tasks[0].state, "failed");
    // 虚构 photo_id 不落观察表；但真实发生的传输按授权语义扣减配额。
    let observations: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM photo_observations")
        .fetch_one(&db.pool)
        .await
        .unwrap();
    assert_eq!(observations, 0);
    let used = consent::effective_view(&db, &p.id, "openai")
        .await
        .unwrap()
        .unwrap()
        .used;
    assert_eq!(used, 2);
    unsafe { std::env::remove_var("BETWEEN_FRAMES_OPENAI_API_KEY") };
}

#[tokio::test]
async fn cloud_image_budget_cap_pauses_task_before_any_upload() {
    let _guard = ENV_LOCK.lock().await;
    unsafe { std::env::set_var("BETWEEN_FRAMES_OPENAI_API_KEY", "test-key-abcd") };
    let temp = tempfile::tempdir().unwrap();
    let (db, p) = setup(temp.path(), 3).await;
    let ids: Vec<String> = p.photos.iter().map(|photo| photo.id.clone()).collect();
    let (base, state) = spawn_server(Script {
        main: vec![
            "tool:list_photos".into(),
            "tool:inspect_from_history".into(),
        ],
        vision: VisionReply::Valid,
    });
    let consent = consent::set(
        &db,
        consent::SetConsent {
            request_id: domain::id(),
            project_id: p.id.clone(),
            mode: "project".into(),
            provider: "openai".into(),
            quota: 100,
        },
    )
    .await
    .unwrap();
    // 预算只允许 1 张，模型请求 2 张 → 触顶暂停且零上传。
    let task = start_vision_task(&db, &p, ids[..2].to_vec(), 1, Some(consent.id)).await;
    engine(&db, &base).run(&task.id).await.unwrap();
    let projection = agent::projection(&db, &p.id).await.unwrap();
    assert_eq!(projection.tasks[0].state, "paused");
    assert!(projection.tasks[0].reason.contains("预算"));
    assert_eq!(image_requests(&state.lock().unwrap()), 0);
    unsafe { std::env::remove_var("BETWEEN_FRAMES_OPENAI_API_KEY") };
}
