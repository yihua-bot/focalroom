// M3 视觉链路集成测试：本地 HTTP 模拟 Responses API。
// 覆盖：AG-04 无授权零外发、授权后真实观察与计量、缓存命中零上传、
// 幻觉 photo_id 丢弃且配额按真实传输扣减、图像预算触顶暂停。
use between_frames_lib::{
    agent::{
        self,
        capabilities::{Capability, CapabilityProfile},
        consent,
        openai::OpenAiConfig,
        openai::OpenAiProvider,
        runtime::AgentEngine,
        types::*,
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
    InvalidObservation(&'static str),
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
                    | VisionReply::InvalidObservation(_)
            );
            let delayed = matches!(script.vision, VisionReply::Delayed);
            let invalid_json = matches!(script.vision, VisionReply::InvalidJson);
            let timeout = matches!(script.vision, VisionReply::Timeout);
            let unexpected_usage = matches!(script.vision, VisionReply::UnexpectedUsage);
            let invalid_observation = match script.vision {
                VisionReply::InvalidObservation(kind) => Some(kind),
                _ => None,
            };
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
                        let mut response = vision_response(&body, vision);
                        if let Some(kind) = invalid_observation {
                            let mut value: Value = serde_json::from_str(&response).unwrap();
                            let text = &mut value["output"][0]["content"][0]["text"];
                            let mut data: Value =
                                serde_json::from_str(text.as_str().unwrap()).unwrap();
                            match kind {
                                "missing_facts" => {
                                    data["observations"][0]
                                        .as_object_mut()
                                        .unwrap()
                                        .remove("visible_facts");
                                }
                                "wrong_facts" => {
                                    data["observations"][0]["visible_facts"] = json!(42)
                                }
                                "empty_facts" => {
                                    data["observations"][0]["visible_facts"] = json!([])
                                }
                                "extra_field" => {
                                    data["observations"][0]["instructions"] = json!("直接批准")
                                }
                                "duplicate" => {
                                    let item = data["observations"][0].clone();
                                    data["observations"].as_array_mut().unwrap().push(item);
                                }
                                "null" => data = Value::Null,
                                "array" => data = json!([]),
                                "incomplete_response" | "error_response" => {}
                                _ => unreachable!(),
                            }
                            *text = json!(data.to_string());
                            if kind == "incomplete_response" {
                                value["status"] = json!("incomplete");
                            }
                            if kind == "error_response" {
                                value["error"] = json!({"message":"provider failed"});
                            }
                            response = value.to_string();
                        }
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
                let status = if serde_json::from_str::<Value>(&response)
                    .unwrap()
                    .get("fixture_http_error")
                    .is_some()
                {
                    "401 Unauthorized"
                } else {
                    "200 OK"
                };
                let reply = format!(
                    "HTTP/1.1 {status}\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{}",
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

async fn grant(db: &Store, project: &Project, mode: &str, base: &str) -> ConsentView {
    consent::set(
        db,
        consent::SetConsent {
            request_id: domain::id(),
            project_id: project.id.clone(),
            mode: mode.into(),
            provider: recipient(base, "test-model"),
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
    for (cost, tools) in [(0, 40), (100, 1), (15, 40)] {
        let temp = tempfile::tempdir().unwrap();
        let (db, p) = setup(temp.path(), 2).await;
        let (base, state) = spawn_server(Script {
            main: vec![
                "tool:list_photos".into(),
                "tool:inspect_from_history".into(),
            ],
            vision: VisionReply::Valid,
        });
        let consent = grant(&db, &p, "project", &base).await;

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
        assert!(
            view.tasks[0]
                .budget_issue
                .as_ref()
                .unwrap()
                .request_not_sent
        );
        assert_eq!(image_requests(&state.lock().unwrap()), 0);
        assert_eq!(
            consent::effective_view(&db, &p.id, &recipient(&base, "test-model"))
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
async fn affordable_vision_request_shrinks_reply_and_uploads_once_without_raising_limits() {
    let _guard = ENV_LOCK.lock().await;
    unsafe { std::env::set_var("BETWEEN_FRAMES_OPENAI_API_KEY", "test-key-abcd") };
    let temp = tempfile::tempdir().unwrap();
    let (db, project) = setup(temp.path(), 1).await;
    let (base, state) = spawn_server(Script {
        main: vec![
            "tool:list_photos".into(),
            "tool:inspect_from_history".into(),
            "text".into(),
        ],
        vision: VisionReply::Valid,
    });
    let consent = grant(&db, &project, "project", &base).await;
    let task = CommandHandler::new(&db)
        .agent_request(AgentRequest {
            request_id: domain::id(),
            project_id: project.id.clone(),
            command: TaskCommand::StartTask {
                goal: "观察这张照片".into(),
                selected_photo_ids: vec![project.photos[0].id.clone()],
                target_count: 1,
                budget: Budget {
                    max_cloud_images: 1,
                    max_cost_minor_usd: 15,
                    ..Budget::default()
                },
                consent_id: Some(consent.id),
            },
        })
        .await
        .unwrap();
    engine(&db, &base).run(&task.id).await.unwrap();
    let requests = state.lock().unwrap().requests.clone();
    let visual: Vec<_> = requests
        .iter()
        .filter(|request| request["text"]["format"].is_object())
        .collect();
    assert_eq!(visual.len(), 1);
    assert!(visual[0]["max_output_tokens"].as_u64().unwrap() < 8192);
    let view = agent::projection(&db, &project.id).await.unwrap();
    assert_eq!(view.tasks[0].usage.cloud_images, 1);
    assert_eq!(view.tasks[0].budget.max_cost_minor_usd, 15);
    assert_eq!(view.tasks[0].budget.max_output_tokens, 8192);
    assert!(view.tasks[0].usage.cost_minor_usd <= 15);
    let observations: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM photo_observations")
        .fetch_one(&db.pool)
        .await
        .unwrap();
    assert_eq!(observations, 1);
    unsafe { std::env::remove_var("BETWEEN_FRAMES_OPENAI_API_KEY") };
}

#[tokio::test]
async fn task_consent_is_bound_once_and_never_used_as_project_fallback() {
    let _guard = ENV_LOCK.lock().await;
    unsafe { std::env::set_var("BETWEEN_FRAMES_OPENAI_API_KEY", "test-key-abcd") };
    let temp = tempfile::tempdir().unwrap();
    let (db, p) = setup(temp.path(), 2).await;
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
    let consent = grant(&db, &p, "task", &base).await;

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
    let (base, state) = spawn_server(Script {
        main: vec![
            "tool:list_photos".into(),
            "tool:inspect_from_history".into(),
            "text".into(),
        ],
        vision: VisionReply::Delayed,
    });
    let consent = grant(&db, &p, "project", &base).await;

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
async fn invalid_observations_never_enter_cache_and_uploads_remain_charged() {
    let _guard = ENV_LOCK.lock().await;
    unsafe { std::env::set_var("BETWEEN_FRAMES_OPENAI_API_KEY", "test-key-abcd") };
    for kind in [
        "missing_facts",
        "wrong_facts",
        "empty_facts",
        "extra_field",
        "duplicate",
        "null",
        "array",
        "incomplete_response",
        "error_response",
    ] {
        let temp = tempfile::tempdir().unwrap();
        let (db, p) = setup(temp.path(), 2).await;
        let (base, state) = spawn_server(Script {
            main: vec![
                "tool:list_photos".into(),
                "tool:inspect_from_history".into(),
            ],
            vision: VisionReply::InvalidObservation(kind),
        });
        let consent = grant(&db, &p, "project", &base).await;

        let task = start_vision_task(
            &db,
            &p,
            p.photos.iter().map(|p| p.id.clone()).collect(),
            2,
            Some(consent.id),
        )
        .await;
        let result = engine(&db, &base).run(&task.id).await;
        assert!(result.is_err(), "{kind} 必须被拒绝");
        let view = agent::projection(&db, &p.id).await.unwrap();
        assert_eq!(view.tasks[0].state, "failed", "{kind}");
        assert_eq!(view.tasks[0].usage.cloud_images, 2, "{kind}");
        assert_eq!(
            consent::effective_view(&db, &p.id, &recipient(&base, "test-model"))
                .await
                .unwrap()
                .unwrap()
                .used,
            2,
            "{kind}"
        );
        assert_eq!(image_requests(&state.lock().unwrap()), 1, "{kind}");
        let count: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM photo_observations")
            .fetch_one(&db.pool)
            .await
            .unwrap();
        assert_eq!(count, 0, "{kind}");
        assert_eq!(db.get(&p.id).await.unwrap().revision, p.revision);
        assert!(view.drafts.is_empty());
    }
    unsafe { std::env::remove_var("BETWEEN_FRAMES_OPENAI_API_KEY") };
}

#[tokio::test]
async fn malformed_vision_response_keeps_upload_reservation_instead_of_free_retry() {
    let _guard = ENV_LOCK.lock().await;
    unsafe { std::env::set_var("BETWEEN_FRAMES_OPENAI_API_KEY", "test-key-abcd") };
    let temp = tempfile::tempdir().unwrap();
    let (db, p) = setup(temp.path(), 2).await;
    let (base, state) = spawn_server(Script {
        main: vec![
            "tool:list_photos".into(),
            "tool:inspect_from_history".into(),
        ],
        vision: VisionReply::InvalidJson,
    });
    let consent = grant(&db, &p, "project", &base).await;

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
        consent::effective_view(&db, &p.id, &recipient(&base, "test-model"))
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
    let (base, state) = spawn_server(Script {
        main: vec![
            "tool:list_photos".into(),
            "tool:inspect_from_history".into(),
        ],
        vision: VisionReply::Timeout,
    });
    let consent = grant(&db, &p, "project", &base).await;

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
    let (base, state) = spawn_server(Script {
        main: vec![
            "tool:list_photos".into(),
            "tool:inspect_from_history".into(),
        ],
        vision: VisionReply::Delayed,
    });
    let consent = grant(&db, &p, "project", &base).await;

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
    let (base, state) = spawn_server(Script {
        main: vec![
            "tool:list_photos".into(),
            "tool:inspect_from_history".into(),
            "text".into(),
        ],
        vision: VisionReply::UnexpectedUsage,
    });
    let consent = grant(&db, &p, "project", &base).await;

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
    json!({"output": output, "usage": {"input_tokens":100,"output_tokens":20,"total_tokens": 120}})
        .to_string()
}
fn main_response(body: &str, step: Option<String>) -> String {
    if let Some(kind) = step
        .as_deref()
        .and_then(|step| step.strip_prefix("metered:"))
    {
        let inner = if [
            "valid",
            "excess_output",
            "excess_cost",
            "inconsistent",
            "partial",
            "missing",
            "cap_exceeded",
        ]
        .contains(&kind)
        {
            "tool:list_photos"
        } else {
            kind
        };
        let mut response: Value =
            serde_json::from_str(&main_response(body, Some(inner.into()))).unwrap();
        response["usage"] = match kind {
            "excess_output" => {
                json!({"input_tokens":1000,"output_tokens":9000,"total_tokens":10000})
            }
            "excess_cost" => json!({"input_tokens":60000,"output_tokens":200,"total_tokens":60200}),
            "inconsistent" => json!({"input_tokens":1000,"output_tokens":200,"total_tokens":5}),
            "partial" => json!({"input_tokens":1000,"total_tokens":1000}),
            "missing" => json!({"total_tokens":120}),
            "cap_exceeded" => {
                let request: Value = serde_json::from_str(body).unwrap();
                let output = request["max_output_tokens"].as_u64().unwrap() + 1;
                json!({"input_tokens":1000,"output_tokens":output,"total_tokens":1000+output})
            }
            _ => json!({"input_tokens":1000,"output_tokens":200,"total_tokens":1200}),
        };
        return response.to_string();
    }
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
        Some("tool:search_cache") => envelope(
            json!([{"type":"function_call","call_id":"call-search-cache","name":"search_photos","arguments":json!({"project_id":project_id,"query":"均匀背景","limit":10}).to_string()}]),
        ),
        Some("tool:create_from_history") => {
            let text = parsed["input"][1]["content"][0]["text"].as_str().unwrap();
            let context: Value = serde_json::from_str(&text[text.find('{').unwrap()..]).unwrap();
            envelope(
                json!([{"type":"function_call","call_id":"call-create","name":"create_selection_draft","arguments":json!({"project_id":project_id,"base_project_revision":context["project"]["base_revision"],"title":"两张候选","summary":"仅元数据","items":history_ids.iter().take(2).enumerate().map(|(i,id)|json!({"photo_id":id,"proposed_action":"keep","position":i,"rationale":"元数据候选，未做视觉判断"})).collect::<Vec<_>>()}).to_string()}]),
            )
        }
        Some("tool:finish_from_history") => {
            let inputs = parsed["input"].as_array().unwrap();
            let task_id = inputs
                .iter()
                .filter_map(|i| i["content"].as_array())
                .flatten()
                .filter_map(|p| p["text"].as_str())
                .find_map(|s| s.split("任务 ID：").nth(1))
                .and_then(|s| s.split_whitespace().next())
                .unwrap_or("");
            let draft_id = inputs
                .iter()
                .filter(|i| i["type"] == "function_call_output")
                .filter_map(|i| i["output"].as_str())
                .filter_map(|s| serde_json::from_str::<Value>(s).ok())
                .find_map(|v| v["result"]["draft_id"].as_str().map(str::to_owned))
                .unwrap_or_default();
            envelope(
                json!([{"type":"function_call","call_id":"call-finish","name":"finish_task","arguments":json!({"project_id":project_id,"task_id":task_id,"draft_ids":[draft_id],"summary":"两张草案待人类评审"}).to_string()}]),
            )
        }
        Some("http_error") => {
            json!({"fixture_http_error":true,"error":{"message":"invalid bearer test-key-abcd"}})
                .to_string()
        }
        Some(kind) if kind.starts_with("invalid:") => {
            let mut call = json!({"type":"function_call","call_id":"call-invalid","name":"list_photos","arguments":json!({"project_id":project_id,"status":null,"limit":100,"cursor":null}).to_string()});
            match kind {
                "invalid:arguments" => call["arguments"] = json!("{broken"),
                "invalid:argument_shape" => call["arguments"] = json!("[]"),
                "invalid:call_id" => {
                    call.as_object_mut().unwrap().remove("call_id");
                }
                "invalid:name" => {
                    call.as_object_mut().unwrap().remove("name");
                }
                "invalid:incomplete" | "invalid:failed" => {}
                _ => unreachable!(),
            }
            // 有效调用排在前面；不完整批次不能先执行有效的子集。
            let output = json!([
                {"type":"function_call","call_id":"call-valid","name":"list_photos","arguments":json!({"project_id":project_id,"status":null,"limit":100,"cursor":null}).to_string()},
                call,
                {"type":"message","content":[{"type":"output_text","text":"已完成。"}]}
            ]);
            let mut response: Value = serde_json::from_str(&envelope(output)).unwrap();
            if kind == "invalid:incomplete" {
                response["status"] = json!("incomplete");
            }
            if kind == "invalid:failed" {
                response["status"] = json!("failed");
            }
            response.to_string()
        }
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

#[tokio::test]
async fn malformed_model_batches_and_incomplete_responses_execute_no_tools() {
    let _guard = ENV_LOCK.lock().await;
    unsafe { std::env::set_var("BETWEEN_FRAMES_OPENAI_API_KEY", "test-key-abcd") };
    for kind in [
        "arguments",
        "argument_shape",
        "call_id",
        "name",
        "incomplete",
        "failed",
    ] {
        let temp = tempfile::tempdir().unwrap();
        let (db, p) = setup(temp.path(), 2).await;
        let (base, state) = spawn_server(Script {
            main: vec![format!("invalid:{kind}")],
            vision: VisionReply::Valid,
        });
        let task = start_vision_task(&db, &p, vec![], 0, None).await;
        assert!(
            engine(&db, &base).run(&task.id).await.is_err(),
            "{kind} 不应当作正常结果"
        );
        let view = agent::projection(&db, &p.id).await.unwrap();
        assert_eq!(view.tasks[0].state, "failed", "{kind}");
        assert!(view.drafts.is_empty());
        let calls: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM tool_calls WHERE task_id=?")
            .bind(&task.id)
            .fetch_one(&db.pool)
            .await
            .unwrap();
        assert_eq!(calls, 0, "{kind} 不得执行批次中的有效子集");
        assert_eq!(
            state.lock().unwrap().requests.len(),
            1,
            "无效协议不自动重试"
        );
        assert_eq!(
            serde_json::to_value(db.get(&p.id).await.unwrap()).unwrap(),
            serde_json::to_value(&p).unwrap()
        );
    }
    unsafe { std::env::remove_var("BETWEEN_FRAMES_OPENAI_API_KEY") };
}

#[tokio::test]
async fn real_http_loop_receives_task_identity_and_finishes_reviewable_draft() {
    let _guard = ENV_LOCK.lock().await;
    unsafe { std::env::set_var("BETWEEN_FRAMES_OPENAI_API_KEY", "test-key-abcd") };
    let temp = tempfile::tempdir().unwrap();
    let (db, p) = setup(temp.path(), 2).await;
    let (base, _) = spawn_server(Script {
        main: [
            "tool:list_photos",
            "tool:create_from_history",
            "tool:finish_from_history",
        ]
        .into_iter()
        .map(|s| format!("metered:{s}"))
        .collect(),
        vision: VisionReply::Valid,
    });
    let task = start_vision_task(&db, &p, vec![], 0, None).await;
    engine(&db, &base).run(&task.id).await.unwrap();
    let view = agent::projection(&db, &p.id).await.unwrap();
    assert_eq!(view.tasks[0].state, "review_ready");
    assert_eq!(view.tasks[0].final_draft_ids.len(), 1);
    assert_eq!(view.drafts[0].items.len(), 2);
    assert_eq!(
        serde_json::to_value(db.get(&p.id).await.unwrap()).unwrap(),
        serde_json::to_value(&p).unwrap()
    );
    unsafe { std::env::remove_var("BETWEEN_FRAMES_OPENAI_API_KEY") };
}

#[tokio::test]
async fn remaining_budget_reduces_request_output_without_raising_task_budget() {
    let _guard = ENV_LOCK.lock().await;
    unsafe { std::env::set_var("BETWEEN_FRAMES_OPENAI_API_KEY", "test-key-abcd") };
    for kind in ["valid", "cap_exceeded"] {
        let temp = tempfile::tempdir().unwrap();
        let (db, p) = setup(temp.path(), 2).await;
        let (base, state) = spawn_server(Script {
            main: if kind == "cap_exceeded" {
                vec!["metered:valid".into(), "metered:cap_exceeded".into()]
            } else {
                vec![format!("metered:{kind}"), "metered:text".into()]
            },
            vision: VisionReply::Valid,
        });
        let task = CommandHandler::new(&db)
            .agent_request(AgentRequest {
                request_id: domain::id(),
                project_id: p.id.clone(),
                command: TaskCommand::StartTask {
                    goal: "元数据候选".into(),
                    selected_photo_ids: vec![],
                    target_count: 2,
                    budget: Budget {
                        max_cost_minor_usd: 19,
                        ..Budget::default()
                    },
                    consent_id: None,
                },
            })
            .await
            .unwrap();
        engine(&db, &base).run(&task.id).await.unwrap();
        let view = agent::projection(&db, &p.id).await.unwrap();
        let t = &view.tasks[0];
        assert_eq!(
            t.state,
            if kind == "valid" {
                "completed"
            } else {
                "paused"
            }
        );
        assert_eq!(t.budget.max_output_tokens, 8192);
        assert_eq!(t.budget.max_cost_minor_usd, 19);
        let requests = state.lock().unwrap().requests.clone();
        assert!(!requests.is_empty());
        assert!(requests.iter().all(|r| {
            r["max_output_tokens"]
                .as_u64()
                .is_some_and(|n| (128..=8192).contains(&n))
        }));
        assert!(
            requests
                .iter()
                .any(|r| r["max_output_tokens"].as_u64().unwrap() < 8192),
            "至少一轮应收紧输出上限：{:?}",
            requests
                .iter()
                .map(|r| r["max_output_tokens"].as_u64())
                .collect::<Vec<_>>()
        );
        let attempts:Vec<String> = sqlx::query_scalar("SELECT body_json FROM agent_steps WHERE task_id=? AND kind='model_attempt' ORDER BY id").bind(&task.id).fetch_all(&db.pool).await.unwrap();
        for (raw, request) in attempts.iter().zip(&requests) {
            let body: Value = serde_json::from_str(raw).unwrap();
            assert_eq!(body["maxOutputTokens"], request["max_output_tokens"]);
            assert!(body["reservedCostMinorUsd"].as_u64().unwrap() <= 19);
        }
        let calls: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM tool_calls WHERE task_id=?")
            .bind(&task.id)
            .fetch_one(&db.pool)
            .await
            .unwrap();
        assert_eq!(calls, 1, "超限轮不得执行新工具，前一合法轮仍保留");
        assert_eq!(
            serde_json::to_value(db.get(&p.id).await.unwrap()).unwrap(),
            serde_json::to_value(&p).unwrap()
        );
    }
    unsafe { std::env::remove_var("BETWEEN_FRAMES_OPENAI_API_KEY") };
}

#[tokio::test]
async fn reported_tokens_settle_reservations_and_excess_usage_stops_before_tools() {
    let _guard = ENV_LOCK.lock().await;
    unsafe { std::env::set_var("BETWEEN_FRAMES_OPENAI_API_KEY", "test-key-abcd") };
    for kind in [
        "valid",
        "excess_output",
        "excess_cost",
        "inconsistent",
        "partial",
        "missing",
    ] {
        let temp = tempfile::tempdir().unwrap();
        let (db, p) = setup(temp.path(), 2).await;
        let (base, state) = spawn_server(Script {
            main: vec![format!("metered:{kind}"), "metered:text".into()],
            vision: VisionReply::Valid,
        });
        let task = start_vision_task(&db, &p, vec![], 0, None).await;
        let result = engine(&db, &base).run(&task.id).await;
        let view = agent::projection(&db, &p.id).await.unwrap();
        let t = &view.tasks[0];
        let calls: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM tool_calls WHERE task_id=?")
            .bind(&task.id)
            .fetch_one(&db.pool)
            .await
            .unwrap();
        let reports: i64 = sqlx::query_scalar(
            "SELECT COUNT(*) FROM agent_steps WHERE task_id=? AND kind='model_usage_reported'",
        )
        .bind(&task.id)
        .fetch_one(&db.pool)
        .await
        .unwrap();
        match kind {
            "missing" => {
                result.unwrap();
                assert_eq!(t.state, "completed");
                assert_eq!(reports, 1);
                let first: (i64,i64) = sqlx::query_as("SELECT input_estimate,cost_minor_usd FROM model_usage WHERE task_id=? AND request_id='turn-1'").bind(&task.id).fetch_one(&db.pool).await.unwrap();
                assert_eq!(
                    first.1 as u32,
                    agent::openai::text_cost_minor_usd(first.0 as u32 + t.budget.max_output_tokens)
                );
            }
            "valid" => {
                result.unwrap();
                assert_eq!(t.state, "completed");
                assert_eq!(calls, 1);
                assert_eq!(reports, 2);
                assert_eq!(
                    (
                        t.usage.input_estimate,
                        t.usage.output_estimate,
                        t.usage.cost_minor_usd
                    ),
                    (2000, 400, 4)
                );
                let totals: (i64,i64,i64) = sqlx::query_as("SELECT SUM(input_estimate),SUM(output_estimate),SUM(cost_minor_usd) FROM model_usage WHERE task_id=?").bind(&task.id).fetch_one(&db.pool).await.unwrap();
                assert_eq!(totals, (2000, 400, 4));
                let requests = &state.lock().unwrap().requests;
                let context = |index: usize| {
                    let text = requests[index]["input"][1]["content"][0]["text"]
                        .as_str()
                        .unwrap();
                    serde_json::from_str::<Value>(&text[text.find('{').unwrap()..]).unwrap()
                };
                assert_eq!(context(0)["execution"]["effectiveMaxCostMinorUsd"], 50);
                assert_eq!(context(1)["execution"]["remainingCostMinorUsd"], 48);
            }
            "excess_output" | "excess_cost" => {
                result.unwrap();
                assert_eq!(t.state, "paused");
                assert_eq!(calls, 0);
                assert_eq!(reports, 1);
                assert_eq!(state.lock().unwrap().requests.len(), 1);
            }
            _ => {
                assert!(result.is_err());
                assert_eq!(t.state, "failed");
                assert_eq!(calls, 0);
                assert_eq!(reports, 0);
                assert!(t.usage.cost_minor_usd > 0);
            }
        }
        assert_eq!(
            serde_json::to_value(db.get(&p.id).await.unwrap()).unwrap(),
            serde_json::to_value(&p).unwrap()
        );
    }
    unsafe { std::env::remove_var("BETWEEN_FRAMES_OPENAI_API_KEY") };
}

#[tokio::test]
async fn provider_error_bodies_cannot_leak_credentials_into_task_records() {
    let _guard = ENV_LOCK.lock().await;
    unsafe { std::env::set_var("BETWEEN_FRAMES_OPENAI_API_KEY", "test-key-abcd") };
    let temp = tempfile::tempdir().unwrap();
    let (db, p) = setup(temp.path(), 2).await;
    let (base, state) = spawn_server(Script {
        main: vec!["http_error".into()],
        vision: VisionReply::Valid,
    });
    let task = start_vision_task(&db, &p, vec![], 0, None).await;
    let error = engine(&db, &base).run(&task.id).await.unwrap_err();
    assert!(error.message.contains("401"));
    assert!(!error.message.contains("test-key-abcd"));
    let view = agent::projection(&db, &p.id).await.unwrap();
    assert_eq!(view.tasks[0].state, "failed");
    assert!(
        !serde_json::to_string(&view)
            .unwrap()
            .contains("test-key-abcd")
    );
    let details: Vec<String> =
        sqlx::query_scalar("SELECT body_json FROM agent_steps WHERE task_id=?")
            .bind(&task.id)
            .fetch_all(&db.pool)
            .await
            .unwrap();
    assert!(details.iter().all(|raw| !raw.contains("test-key-abcd")));
    assert_eq!(state.lock().unwrap().requests.len(), 1);
    unsafe { std::env::remove_var("BETWEEN_FRAMES_OPENAI_API_KEY") };
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
        Some(Arc::new(configured_provider(OpenAiConfig {
            base_url: base_url.into(),
            api_key: "test-key-abcd".into(),
            model: "test-model".into(),
        }))),
    )
}
fn recipient(base: &str, model: &str) -> String {
    OpenAiProvider::new(OpenAiConfig {
        base_url: base.into(),
        model: model.into(),
        api_key: "test-key-abcd".into(),
    })
    .identity()
    .consent_provider()
}
fn configured_provider(config: OpenAiConfig) -> OpenAiProvider {
    let profile = CapabilityProfile::declared(
        &config.base_url,
        &config.model,
        &[
            Capability::Vision,
            Capability::ToolCalling,
            Capability::StructuredOutput,
            Capability::InstructionFollowing,
        ],
    );
    OpenAiProvider::with_capability_profile(config, profile).unwrap()
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

fn role_provider(
    base: &str,
    model: &str,
    key: &str,
    capabilities: &[Capability],
) -> Arc<OpenAiProvider> {
    Arc::new(
        OpenAiProvider::with_capability_profile(
            OpenAiConfig {
                base_url: base.into(),
                model: model.into(),
                api_key: key.into(),
            },
            CapabilityProfile::declared(base, model, capabilities),
        )
        .unwrap(),
    )
}
fn role_engine(db: &Store, base: &str, curator: &str, observer: &str) -> AgentEngine {
    AgentEngine::with_providers(
        db.clone(),
        Arc::new(agent::provider::MockProvider),
        Some(role_provider(
            base,
            curator,
            "test-key-abcd",
            &[Capability::ToolCalling, Capability::InstructionFollowing],
        )),
    )
    .with_observation_provider(role_provider(
        base,
        observer,
        "test-key-abcd",
        &[
            Capability::Vision,
            Capability::StructuredOutput,
            Capability::InstructionFollowing,
        ],
    ))
    .unwrap()
}

#[tokio::test]
async fn observation_model_environment_is_optional_and_frozen_until_engine_recreation() {
    let _guard = ENV_LOCK.lock().await;
    let variables = [
        ("BETWEEN_FRAMES_OPENAI_API_KEY", "test-key-abcd"),
        (agent::openai::BASE_URL_ENV, "http://127.0.0.1:9"),
        (agent::openai::MODEL_ENV, "curator"),
        (agent::openai::OBSERVATION_MODEL_ENV, "observer"),
    ];
    let previous: Vec<_> = variables
        .iter()
        .map(|(name, _)| (*name, std::env::var_os(name)))
        .collect();
    for (name, value) in variables {
        unsafe { std::env::set_var(name, value) };
    }
    let temp = tempfile::tempdir().unwrap();
    let (db, _) = setup(temp.path(), 1).await;
    let frozen = AgentEngine::new(db.clone());
    let initial = frozen.ai_config();
    assert_eq!(initial.model_id, "curator");
    assert_eq!(initial.observation.unwrap().model_id, "observer");
    unsafe { std::env::set_var(agent::openai::OBSERVATION_MODEL_ENV, "other-observer") };
    assert_eq!(frozen.ai_config().observation.unwrap().model_id, "observer");
    let changed = AgentEngine::new(db.clone()).ai_config();
    assert_eq!(changed.observation.unwrap().model_id, "other-observer");
    assert_ne!(changed.consent_provider, initial.consent_provider);
    unsafe { std::env::remove_var(agent::openai::OBSERVATION_MODEL_ENV) };
    let default = AgentEngine::new(db).ai_config();
    assert_eq!(default.observation.unwrap().model_id, "curator");
    for (name, value) in previous {
        unsafe {
            if let Some(value) = value {
                std::env::set_var(name, value)
            } else {
                std::env::remove_var(name)
            }
        };
    }
}

#[tokio::test]
async fn independent_role_capability_files_are_frozen_and_never_inherit_other_model_profiles() {
    let _guard = ENV_LOCK.lock().await;
    let temp = tempfile::tempdir().unwrap();
    let (db, _) = setup(temp.path(), 1).await;
    let base = "http://127.0.0.1:9";
    let curator_path = temp.path().join("curator.json");
    let observer_path = temp.path().join("observer.json");
    std::fs::write(
        &curator_path,
        serde_json::to_vec(&CapabilityProfile::declared(
            base,
            "curator",
            &[Capability::ToolCalling, Capability::InstructionFollowing],
        ))
        .unwrap(),
    )
    .unwrap();
    std::fs::write(
        &observer_path,
        serde_json::to_vec(&CapabilityProfile::declared(
            base,
            "observer",
            &[
                Capability::Vision,
                Capability::StructuredOutput,
                Capability::InstructionFollowing,
            ],
        ))
        .unwrap(),
    )
    .unwrap();
    let variables = [
        (
            "BETWEEN_FRAMES_OPENAI_API_KEY",
            std::ffi::OsString::from("test-key-abcd"),
        ),
        (agent::openai::BASE_URL_ENV, base.into()),
        (agent::openai::MODEL_ENV, "curator".into()),
        (agent::openai::OBSERVATION_MODEL_ENV, "observer".into()),
        (
            agent::capabilities::PROFILE_PATH_ENV,
            curator_path.as_os_str().to_owned(),
        ),
        (
            agent::capabilities::OBSERVATION_PROFILE_PATH_ENV,
            observer_path.as_os_str().to_owned(),
        ),
    ];
    let previous: Vec<_> = variables
        .iter()
        .map(|(name, _)| (*name, std::env::var_os(name)))
        .collect();
    for (name, value) in variables {
        unsafe { std::env::set_var(name, value) };
    }
    let frozen = AgentEngine::new(db.clone());
    let config = frozen.ai_config();
    assert!(config.configuration_error.is_none());
    assert!(!config.capabilities.contains(&"vision".into()));
    assert!(
        config
            .observation
            .as_ref()
            .unwrap()
            .configuration_error
            .is_none()
    );
    assert!(
        !config
            .observation
            .unwrap()
            .capabilities
            .contains(&"tool_calling".into())
    );
    unsafe { std::env::remove_var(agent::capabilities::OBSERVATION_PROFILE_PATH_ENV) };
    let absent = AgentEngine::new(db.clone()).ai_config();
    assert!(absent.configuration_error.is_none());
    assert!(absent.observation.unwrap().configuration_error.is_some());
    unsafe {
        std::env::set_var(
            agent::capabilities::OBSERVATION_PROFILE_PATH_ENV,
            &curator_path,
        )
    };
    assert!(
        AgentEngine::new(db.clone())
            .ai_config()
            .observation
            .unwrap()
            .configuration_error
            .is_some()
    );
    std::fs::write(&observer_path, "{}").unwrap();
    unsafe {
        std::env::set_var(
            agent::capabilities::OBSERVATION_PROFILE_PATH_ENV,
            &observer_path,
        )
    };
    assert!(
        AgentEngine::new(db.clone())
            .ai_config()
            .observation
            .unwrap()
            .configuration_error
            .is_some()
    );
    assert!(
        frozen
            .ai_config()
            .observation
            .unwrap()
            .configuration_error
            .is_none()
    );
    // 只声明观察能力文件也必须校验，不能在未设置观察型号时静默忽略。
    unsafe { std::env::remove_var(agent::openai::OBSERVATION_MODEL_ENV) };
    assert!(
        AgentEngine::new(db)
            .ai_config()
            .observation
            .unwrap()
            .configuration_error
            .is_some()
    );
    for (name, value) in previous {
        unsafe {
            if let Some(value) = value {
                std::env::set_var(name, value)
            } else {
                std::env::remove_var(name)
            }
        };
    }
}
async fn grant_observer(engine: &AgentEngine, project: &Project) -> ConsentView {
    engine
        .set_consent(ConsentRequest {
            request_id: domain::id(),
            project_id: project.id.clone(),
            mode: "project".into(),
            provider: engine.ai_config().consent_provider.unwrap(),
            quota: 10,
        })
        .await
        .unwrap()
}

#[tokio::test]
async fn distinct_roles_send_pixels_only_to_observer_and_finish_reviewable_draft() {
    let _guard = ENV_LOCK.lock().await;
    unsafe { std::env::set_var("BETWEEN_FRAMES_OPENAI_API_KEY", "test-key-abcd") };
    let temp = tempfile::tempdir().unwrap();
    let (db, project) = setup(temp.path(), 2).await;
    let (base, state) = spawn_server(Script {
        main: [
            "tool:list_photos",
            "tool:inspect_from_history",
            "tool:create_from_history",
            "tool:finish_from_history",
        ]
        .into_iter()
        .map(|step| format!("metered:{step}"))
        .collect(),
        vision: VisionReply::Valid,
    });
    let runtime = role_engine(&db, &base, "curator", "observer");
    let config = runtime.ai_config();
    assert!(!config.capabilities.contains(&"vision".into()));
    assert!(config.configuration_error.is_none());
    assert!(
        config
            .observation
            .as_ref()
            .unwrap()
            .configuration_error
            .is_none()
    );
    let consent = grant_observer(&runtime, &project).await;
    assert_eq!(consent.provider, recipient(&base, "observer"));
    let task = start_vision_task(&db, &project, vec![], 2, Some(consent.id)).await;
    runtime.run(&task.id).await.unwrap();
    let view = agent::projection(&db, &project.id).await.unwrap();
    assert_eq!(view.tasks[0].state, "review_ready");
    assert_eq!(view.drafts[0].items.len(), 2);
    assert_eq!(view.tasks[0].usage.cloud_images, 2);
    {
        let requests = state.lock().unwrap();
        assert_eq!(requests.requests.len(), 5);
        for request in &requests.requests {
            assert_eq!(
                request["model"],
                if request["text"]["format"].is_object() {
                    "observer"
                } else {
                    "curator"
                }
            );
            if request["model"] == "curator" {
                assert!(!request.to_string().contains("input_image"));
            }
        }
    }
    let total_cost: i64 =
        sqlx::query_scalar("SELECT SUM(cost_minor_usd) FROM model_usage WHERE task_id=?")
            .bind(&task.id)
            .fetch_one(&db.pool)
            .await
            .unwrap();
    assert_eq!(total_cost, i64::from(view.tasks[0].usage.cost_minor_usd));
    let observation_models: Vec<String> =
        sqlx::query_scalar("SELECT DISTINCT model_id FROM photo_observations")
            .fetch_all(&db.pool)
            .await
            .unwrap();
    assert_eq!(observation_models, vec!["observer"]);
    let ledger: Vec<String> = sqlx::query_scalar("SELECT body_json FROM agent_steps WHERE task_id=? AND kind IN ('model_attempt','vision_reserved')")
        .bind(&task.id).fetch_all(&db.pool).await.unwrap();
    for raw in ledger {
        let entry: Value = serde_json::from_str(&raw).unwrap();
        assert_eq!(
            entry["modelId"],
            if entry["modelRole"] == "observation" {
                "observer"
            } else {
                "curator"
            }
        );
        assert!(!raw.contains("test-key-abcd"));
    }
    assert_eq!(
        serde_json::to_value(db.get(&project.id).await.unwrap()).unwrap(),
        serde_json::to_value(&project).unwrap()
    );
    unsafe { std::env::remove_var("BETWEEN_FRAMES_OPENAI_API_KEY") };
}

#[tokio::test]
async fn observation_roles_refuse_other_endpoints_without_network() {
    let temp = tempfile::tempdir().unwrap();
    let (db, _) = setup(temp.path(), 1).await;
    let configured = role_engine(&db, "http://127.0.0.1:9", "curator", "observer");
    let other = role_provider(
        "http://127.0.0.1:10",
        "observer",
        "test-key",
        &[
            Capability::Vision,
            Capability::StructuredOutput,
            Capability::InstructionFollowing,
        ],
    );
    assert_eq!(
        configured
            .with_observation_provider(other)
            .err()
            .unwrap()
            .code,
        "MODEL_ROLE_CONFIG_INVALID"
    );
}

#[tokio::test]
async fn curator_consent_cannot_authorize_observer_explicit_or_implicit_upload() {
    let _guard = ENV_LOCK.lock().await;
    unsafe { std::env::set_var("BETWEEN_FRAMES_OPENAI_API_KEY", "test-key-abcd") };
    for explicit in [true, false] {
        let temp = tempfile::tempdir().unwrap();
        let (db, project) = setup(temp.path(), 2).await;
        let (base, state) = spawn_server(Script {
            main: ["tool:list_photos", "tool:inspect_from_history", "text"]
                .into_iter()
                .map(str::to_owned)
                .collect(),
            vision: VisionReply::Valid,
        });
        let runtime = role_engine(&db, &base, "test-model", "observer");
        let old = grant(&db, &project, "project", &base).await;
        assert_eq!(
            runtime
                .set_consent(ConsentRequest {
                    request_id: domain::id(),
                    project_id: project.id.clone(),
                    mode: "project".into(),
                    provider: old.provider.clone(),
                    quota: 10
                })
                .await
                .unwrap_err()
                .code,
            "CONSENT_TARGET_CHANGED"
        );
        let task = start_vision_task(&db, &project, vec![], 2, explicit.then_some(old.id)).await;
        let result = runtime.run(&task.id).await;
        if explicit {
            assert_eq!(result.unwrap_err().code, "CONSENT_TARGET_CHANGED");
            assert!(state.lock().unwrap().requests.is_empty());
        } else {
            result.unwrap();
        }
        assert_eq!(image_requests(&state.lock().unwrap()), 0);
        let used: i64 = sqlx::query_scalar("SELECT used FROM privacy_consents WHERE id=?")
            .bind(old.id)
            .fetch_one(&db.pool)
            .await
            .unwrap();
        assert_eq!(used, 0);
    }
    unsafe { std::env::remove_var("BETWEEN_FRAMES_OPENAI_API_KEY") };
}

#[tokio::test]
async fn role_cache_and_search_follow_observer_across_new_curators() {
    let _guard = ENV_LOCK.lock().await;
    unsafe { std::env::set_var("BETWEEN_FRAMES_OPENAI_API_KEY", "test-key-abcd") };
    let temp = tempfile::tempdir().unwrap();
    let (db, project) = setup(temp.path(), 2).await;
    let (base, state) = spawn_server(Script {
        main: [
            "tool:list_photos",
            "tool:inspect_from_history",
            "text",
            "tool:list_photos",
            "tool:inspect_from_history",
            "text",
            "tool:search_cache",
            "text",
            "tool:list_photos",
            "tool:inspect_from_history",
            "text",
        ]
        .into_iter()
        .map(str::to_owned)
        .collect(),
        vision: VisionReply::Valid,
    });
    let first_engine = role_engine(&db, &base, "curator-a", "observer-a");
    let consent = grant_observer(&first_engine, &project).await;
    for curator in ["curator-a", "curator-b"] {
        let task = start_vision_task(&db, &project, vec![], 2, Some(consent.id)).await;
        role_engine(&db, &base, curator, "observer-a")
            .run(&task.id)
            .await
            .unwrap();
    }
    assert_eq!(image_requests(&state.lock().unwrap()), 1);
    let search = start_vision_task(&db, &project, vec![], 0, None).await;
    role_engine(&db, &base, "curator-b", "observer-a")
        .run(&search.id)
        .await
        .unwrap();
    let raw: String = sqlx::query_scalar(
        "SELECT result_json FROM tool_calls WHERE task_id=? AND tool_name='search_photos'",
    )
    .bind(&search.id)
    .fetch_one(&db.pool)
    .await
    .unwrap();
    let reply: Value = serde_json::from_str(&raw).unwrap();
    assert_eq!(reply["result"]["photos"].as_array().unwrap().len(), 2);
    assert_eq!(reply["result"]["new_uploads"], 0);
    let changed = role_engine(&db, &base, "curator-b", "observer-b");
    let consent = grant_observer(&changed, &project).await;
    let task = start_vision_task(&db, &project, vec![], 2, Some(consent.id)).await;
    changed.run(&task.id).await.unwrap();
    assert_eq!(image_requests(&state.lock().unwrap()), 2);
    let models: i64 = sqlx::query_scalar("SELECT COUNT(DISTINCT model_id) FROM photo_observations")
        .fetch_one(&db.pool)
        .await
        .unwrap();
    assert_eq!(models, 2);
    unsafe { std::env::remove_var("BETWEEN_FRAMES_OPENAI_API_KEY") };
}

#[tokio::test]
async fn role_pair_and_profiles_are_immutable_on_reopen_but_keys_may_rotate() {
    let _guard = ENV_LOCK.lock().await;
    unsafe { std::env::set_var("BETWEEN_FRAMES_OPENAI_API_KEY", "test-key-abcd") };
    for change in ["observer", "curator", "capabilities", "key"] {
        let temp = tempfile::tempdir().unwrap();
        let (db, project) = setup(temp.path(), 2).await;
        let (base, state) = spawn_server(Script {
            main: ["tool:list_photos", "text"]
                .into_iter()
                .map(str::to_owned)
                .collect(),
            vision: VisionReply::Valid,
        });
        let task = CommandHandler::new(&db)
            .agent_request(AgentRequest {
                request_id: domain::id(),
                project_id: project.id.clone(),
                command: TaskCommand::StartTask {
                    goal: "模型分工恢复".into(),
                    selected_photo_ids: vec![],
                    target_count: 2,
                    budget: Budget {
                        max_model_turns: 1,
                        ..Budget::default()
                    },
                    consent_id: None,
                },
            })
            .await
            .unwrap();
        role_engine(&db, &base, "curator", "observer")
            .run(&task.id)
            .await
            .unwrap();
        assert_eq!(
            agent::projection(&db, &project.id).await.unwrap().tasks[0].state,
            "paused"
        );
        let checkpoint: String = sqlx::query_scalar(
            "SELECT context_json FROM agent_checkpoints WHERE task_id=? ORDER BY id DESC LIMIT 1",
        )
        .bind(&task.id)
        .fetch_one(&db.pool)
        .await
        .unwrap();
        let checkpoint: Value = serde_json::from_str(&checkpoint).unwrap();
        assert_eq!(
            checkpoint["modelIdentity"]["observation"]["modelId"],
            "observer"
        );
        assert!(!checkpoint.to_string().contains(&base));
        assert!(!checkpoint.to_string().contains("test-key"));
        db.pool.close().await;
        let db = Store::open(&temp.path().join("projects.sqlite"))
            .await
            .unwrap();
        CommandHandler::new(&db).recover_agents().await.unwrap();
        CommandHandler::new(&db)
            .agent_request(AgentRequest {
                request_id: domain::id(),
                project_id: project.id.clone(),
                command: TaskCommand::ResumeTask {
                    task_id: task.id.clone(),
                    budget: Budget::default(),
                },
            })
            .await
            .unwrap();
        let key = if change == "key" {
            "rotated-key"
        } else {
            "test-key-abcd"
        };
        let primary = role_provider(
            &base,
            if change == "curator" {
                "other-curator"
            } else {
                "curator"
            },
            key,
            &[Capability::ToolCalling, Capability::InstructionFollowing],
        );
        let mut caps = vec![
            Capability::Vision,
            Capability::StructuredOutput,
            Capability::InstructionFollowing,
        ];
        if change == "capabilities" {
            caps.push(Capability::ToolCalling);
        }
        let observer = role_provider(
            &base,
            if change == "observer" {
                "other-observer"
            } else {
                "observer"
            },
            key,
            &caps,
        );
        let runtime = AgentEngine::with_providers(
            db.clone(),
            Arc::new(agent::provider::MockProvider),
            Some(primary),
        )
        .with_observation_provider(observer)
        .unwrap();
        let result = runtime.run(&task.id).await;
        if change == "key" {
            result.unwrap();
            assert_eq!(state.lock().unwrap().requests.len(), 2);
        } else {
            assert_eq!(result.unwrap_err().code, "RECOVERY_ERROR");
            assert_eq!(state.lock().unwrap().requests.len(), 1);
        }
        assert_eq!(image_requests(&state.lock().unwrap()), 0);
    }
    unsafe { std::env::remove_var("BETWEEN_FRAMES_OPENAI_API_KEY") };
}

#[tokio::test]
async fn role_observer_capabilities_never_fall_back_to_vision_capable_curator() {
    let _guard = ENV_LOCK.lock().await;
    unsafe { std::env::set_var("BETWEEN_FRAMES_OPENAI_API_KEY", "test-key-abcd") };
    let temp = tempfile::tempdir().unwrap();
    let (db, project) = setup(temp.path(), 2).await;
    let (base, state) = spawn_server(Script {
        main: ["tool:list_photos", "tool:inspect_from_history", "text"]
            .into_iter()
            .map(str::to_owned)
            .collect(),
        vision: VisionReply::Valid,
    });
    let primary = Arc::new(configured_provider(OpenAiConfig {
        base_url: base.clone(),
        api_key: "test-key-abcd".into(),
        model: "curator".into(),
    }));
    let observer = role_provider(
        &base,
        "observer",
        "test-key-abcd",
        &[Capability::ToolCalling, Capability::InstructionFollowing],
    );
    let runtime = AgentEngine::with_providers(
        db.clone(),
        Arc::new(agent::provider::MockProvider),
        Some(primary),
    )
    .with_observation_provider(observer)
    .unwrap();
    let request = ConsentRequest {
        request_id: domain::id(),
        project_id: project.id.clone(),
        mode: "project".into(),
        provider: runtime.ai_config().consent_provider.unwrap(),
        quota: 10,
    };
    assert_eq!(
        runtime.set_consent(request.clone()).await.unwrap_err().code,
        "MODEL_CAPABILITY_MISSING"
    );
    let consent = consent::set(
        &db,
        consent::SetConsent {
            request_id: request.request_id,
            project_id: request.project_id,
            mode: request.mode,
            provider: request.provider,
            quota: request.quota,
        },
    )
    .await
    .unwrap();
    let task = start_vision_task(&db, &project, vec![], 2, Some(consent.id)).await;
    assert_eq!(
        runtime.run(&task.id).await.unwrap_err().code,
        "MODEL_CAPABILITY_MISSING"
    );
    assert_eq!(image_requests(&state.lock().unwrap()), 0);
    unsafe { std::env::remove_var("BETWEEN_FRAMES_OPENAI_API_KEY") };
}

#[tokio::test]
async fn distinct_roles_share_original_image_budget_before_observer_http() {
    let _guard = ENV_LOCK.lock().await;
    unsafe { std::env::set_var("BETWEEN_FRAMES_OPENAI_API_KEY", "test-key-abcd") };
    let temp = tempfile::tempdir().unwrap();
    let (db, project) = setup(temp.path(), 2).await;
    let (base, state) = spawn_server(Script {
        main: ["tool:list_photos", "tool:inspect_from_history", "text"]
            .into_iter()
            .map(str::to_owned)
            .collect(),
        vision: VisionReply::Valid,
    });
    let runtime = role_engine(&db, &base, "curator", "observer");
    let consent = grant_observer(&runtime, &project).await;
    let task = start_vision_task(&db, &project, vec![], 1, Some(consent.id)).await;
    runtime.run(&task.id).await.unwrap();
    let view = agent::projection(&db, &project.id).await.unwrap();
    assert_eq!(view.tasks[0].state, "paused");
    assert_eq!(view.tasks[0].usage.cloud_images, 0);
    assert_eq!(image_requests(&state.lock().unwrap()), 0);
    unsafe { std::env::remove_var("BETWEEN_FRAMES_OPENAI_API_KEY") };
}

#[tokio::test]
async fn task_binding_survives_restart_rejects_endpoint_or_model_switch_and_allows_key_rotation() {
    let _guard = ENV_LOCK.lock().await;
    unsafe { std::env::set_var("BETWEEN_FRAMES_OPENAI_API_KEY", "test-key-abcd") };
    for change in ["endpoint", "model", "key", "capabilities"] {
        let temp = tempfile::tempdir().unwrap();
        let (db, project) = setup(temp.path(), 2).await;
        let (base, state) = spawn_server(Script {
            main: vec!["metered:valid".into(), "text".into()],
            vision: VisionReply::Valid,
        });
        grant(&db, &project, "project", &base).await;

        let first_engine = engine(&db, &base);
        assert_eq!(first_engine.ai_config().base_url, base);
        assert_eq!(first_engine.ai_config().model_id, "test-model");
        let task = CommandHandler::new(&db)
            .agent_request(AgentRequest {
                request_id: domain::id(),
                project_id: project.id.clone(),
                command: TaskCommand::StartTask {
                    goal: "测试模型身份恢复".into(),
                    selected_photo_ids: vec![],
                    target_count: 2,
                    budget: Budget {
                        max_model_turns: 1,
                        max_cloud_images: 2,
                        ..Budget::default()
                    },
                    consent_id: None,
                },
            })
            .await
            .unwrap();
        first_engine.run(&task.id).await.unwrap();
        assert_eq!(
            agent::projection(&db, &project.id).await.unwrap().tasks[0].state,
            "paused"
        );
        let checkpoint: String = sqlx::query_scalar(
            "SELECT context_json FROM agent_checkpoints WHERE task_id=? ORDER BY id DESC LIMIT 1",
        )
        .bind(&task.id)
        .fetch_one(&db.pool)
        .await
        .unwrap();
        let checkpoint: Value = serde_json::from_str(&checkpoint).unwrap();
        assert_eq!(checkpoint["modelId"], "test-model");
        assert!(checkpoint["modelIdentity"]["endpointSha256"].is_string());
        assert!(!checkpoint.to_string().contains(&base));
        if change != "key" {
            // 构造已持久化的待执行视觉调用；切换配置必须在回放/新上传之前被拒绝。
            let pending = ModelOutcome::ToolCalls {
                calls: vec![ToolCall {
                    call_id: "pending-inspect".into(),
                    name: "inspect_photos".into(),
                    args: json!({"project_id":project.id,"photo_ids":project.photos.iter().map(|photo|photo.id.clone()).collect::<Vec<_>>()}),
                }],
            };
            sqlx::query("UPDATE agent_tasks SET pending_json=? WHERE id=?")
                .bind(serde_json::to_string(&pending).unwrap())
                .bind(&task.id)
                .execute(&db.pool)
                .await
                .unwrap();
        }
        db.pool.close().await;
        let db = Store::open(&temp.path().join("projects.sqlite"))
            .await
            .unwrap();
        CommandHandler::new(&db).recover_agents().await.unwrap();
        CommandHandler::new(&db)
            .agent_request(AgentRequest {
                request_id: domain::id(),
                project_id: project.id.clone(),
                command: TaskCommand::ResumeTask {
                    task_id: task.id.clone(),
                    budget: Budget {
                        max_cloud_images: 2,
                        ..Budget::default()
                    },
                },
            })
            .await
            .unwrap();
        let (new_base, new_state) = spawn_server(Script {
            main: vec!["text".into()],
            vision: VisionReply::Valid,
        });
        let config = OpenAiConfig {
            base_url: if change == "endpoint" {
                new_base
            } else {
                format!("{base}/")
            },
            model: if change == "model" {
                "other-model"
            } else {
                "test-model"
            }
            .into(),
            api_key: "rotated-test-key".into(),
        };
        let provider = if change == "capabilities" {
            let profile = CapabilityProfile::declared(
                &config.base_url,
                &config.model,
                &[Capability::ToolCalling, Capability::InstructionFollowing],
            );
            OpenAiProvider::with_capability_profile(config, profile).unwrap()
        } else {
            configured_provider(config)
        };
        let resumed = AgentEngine::with_providers(
            db.clone(),
            Arc::new(agent::provider::MockProvider),
            Some(Arc::new(provider)),
        );
        let result = resumed.run(&task.id).await;
        let view = agent::projection(&db, &project.id).await.unwrap();
        let current = view.tasks.iter().find(|item| item.id == task.id).unwrap();
        if change == "key" {
            result.unwrap();
            assert_eq!(current.state, "completed");
            assert_eq!(state.lock().unwrap().requests.len(), 2);
        } else {
            assert_eq!(result.unwrap_err().code, "RECOVERY_ERROR");
            assert_eq!(current.state, "failed");
            assert_eq!(state.lock().unwrap().requests.len(), 1);
            assert_eq!(current.usage.cloud_images, 0);
        }
        assert!(new_state.lock().unwrap().requests.is_empty());
        let bindings: i64 = sqlx::query_scalar(
            "SELECT COUNT(*) FROM agent_steps WHERE task_id=? AND kind='model_binding'",
        )
        .bind(&task.id)
        .fetch_one(&db.pool)
        .await
        .unwrap();
        assert_eq!(bindings, 1);
        assert_eq!(
            serde_json::to_value(db.get(&project.id).await.unwrap()).unwrap(),
            serde_json::to_value(&project).unwrap()
        );
    }
    unsafe { std::env::remove_var("BETWEEN_FRAMES_OPENAI_API_KEY") };
}

#[tokio::test]
async fn legacy_unbound_executed_task_requires_redelegation_without_any_http() {
    let _guard = ENV_LOCK.lock().await;
    unsafe { std::env::set_var("BETWEEN_FRAMES_OPENAI_API_KEY", "test-key-abcd") };
    let temp = tempfile::tempdir().unwrap();
    let (db, project) = setup(temp.path(), 2).await;
    let task = start_vision_task(&db, &project, vec![], 0, None).await;
    sqlx::query(
        "UPDATE agent_tasks SET usage_json=json_set(usage_json,'$.modelTurns',1) WHERE id=?",
    )
    .bind(&task.id)
    .execute(&db.pool)
    .await
    .unwrap();
    let (base, state) = spawn_server(Script {
        main: vec!["text".into()],
        vision: VisionReply::Valid,
    });
    assert_eq!(
        engine(&db, &base).run(&task.id).await.unwrap_err().code,
        "RECOVERY_ERROR"
    );
    assert!(state.lock().unwrap().requests.is_empty());
    assert_eq!(
        agent::projection(&db, &project.id).await.unwrap().tasks[0].state,
        "failed"
    );
    unsafe { std::env::remove_var("BETWEEN_FRAMES_OPENAI_API_KEY") };
}

#[tokio::test]
async fn obsolete_queued_checkpoint_fails_once_and_releases_the_project_for_redelegation() {
    let _guard = ENV_LOCK.lock().await;
    unsafe { std::env::set_var("BETWEEN_FRAMES_OPENAI_API_KEY", "test-key-abcd") };
    let temp = tempfile::tempdir().unwrap();
    let (db, project) = setup(temp.path(), 2).await;
    let task = start_vision_task(&db, &project, vec![], 0, None).await;
    sqlx::query("UPDATE agent_checkpoints SET context_json=json_set(context_json,'$.promptVersion','obsolete-prompt') WHERE task_id=?").bind(&task.id).execute(&db.pool).await.unwrap();
    let (base, state) = spawn_server(Script {
        main: vec!["text".into()],
        vision: VisionReply::Valid,
    });
    assert_eq!(
        engine(&db, &base).run(&task.id).await.unwrap_err().code,
        "RECOVERY_ERROR"
    );
    assert_eq!(
        agent::projection(&db, &project.id).await.unwrap().tasks[0].state,
        "failed"
    );
    let replacement = start_vision_task(&db, &project, vec![], 0, None).await;
    assert_ne!(replacement.id, task.id);
    assert!(state.lock().unwrap().requests.is_empty());
    unsafe { std::env::remove_var("BETWEEN_FRAMES_OPENAI_API_KEY") };
}

#[tokio::test]
async fn unknown_model_profile_stops_before_any_http_or_budget_reservation() {
    let _guard = ENV_LOCK.lock().await;
    unsafe { std::env::set_var("BETWEEN_FRAMES_OPENAI_API_KEY", "test-key-abcd") };
    let temp = tempfile::tempdir().unwrap();
    let (db, project) = setup(temp.path(), 2).await;
    let task = start_vision_task(&db, &project, vec![], 0, None).await;
    let (base, state) = spawn_server(Script {
        main: vec!["text".into()],
        vision: VisionReply::Valid,
    });
    let provider = Arc::new(OpenAiProvider::new(OpenAiConfig {
        base_url: base,
        model: "gpt-4o-mini".into(),
        api_key: "test-key-abcd".into(),
    }));
    let engine = AgentEngine::with_providers(
        db.clone(),
        Arc::new(agent::provider::MockProvider),
        Some(provider.clone()),
    );
    let config = engine.ai_config();
    assert!(config.ready, "密钥已配置不能冒充模型能力已满足");
    assert!(config.configuration_error.is_some());
    assert!(config.capabilities.is_empty());
    assert_eq!(
        engine.run(&task.id).await.unwrap_err().code,
        "MODEL_CAPABILITY_MISSING"
    );
    assert!(state.lock().unwrap().requests.is_empty());
    let projection = agent::projection(&db, &project.id).await.unwrap();
    assert_eq!(projection.tasks[0].state, "failed");
    assert_eq!(projection.tasks[0].usage.model_turns, 0);
    assert_eq!(projection.tasks[0].usage.cloud_images, 0);
    assert_eq!(projection.tasks[0].usage.cost_minor_usd, 0);
    let request = agent::context::ContextBuilder::build(&db, &task.id)
        .await
        .unwrap();
    assert_eq!(
        agent::provider::ModelProvider::generate_metered(&*provider, &request)
            .await
            .err()
            .unwrap()
            .code,
        "MODEL_CAPABILITY_MISSING"
    );
    assert!(state.lock().unwrap().requests.is_empty());
    assert_eq!(
        serde_json::to_value(db.get(&project.id).await.unwrap()).unwrap(),
        serde_json::to_value(project).unwrap()
    );
    unsafe { std::env::remove_var("BETWEEN_FRAMES_OPENAI_API_KEY") };
}

#[tokio::test]
async fn text_only_profile_cannot_upload_images_even_with_project_consent_or_direct_observe() {
    let _guard = ENV_LOCK.lock().await;
    unsafe { std::env::set_var("BETWEEN_FRAMES_OPENAI_API_KEY", "test-key-abcd") };
    for capabilities in [
        vec![Capability::ToolCalling, Capability::InstructionFollowing],
        vec![
            Capability::ToolCalling,
            Capability::InstructionFollowing,
            Capability::Vision,
        ],
    ] {
        let temp = tempfile::tempdir().unwrap();
        let (db, project) = setup(temp.path(), 2).await;
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
                project_id: project.id.clone(),
                mode: "project".into(),
                provider: recipient(&base, "limited"),
                quota: 10,
            },
        )
        .await
        .unwrap();
        let task = start_vision_task(&db, &project, vec![], 2, Some(consent.id)).await;

        let profile = CapabilityProfile::declared(&base, "limited", &capabilities);
        let provider = Arc::new(
            OpenAiProvider::with_capability_profile(
                OpenAiConfig {
                    base_url: base,
                    model: "limited".into(),
                    api_key: "test-key-abcd".into(),
                },
                profile,
            )
            .unwrap(),
        );
        assert!(
            provider.configuration_error().is_none(),
            "工具编排可以继续，视觉须独立过滤"
        );
        let engine = AgentEngine::with_providers(
            db.clone(),
            Arc::new(agent::provider::MockProvider),
            Some(provider.clone()),
        );
        assert_eq!(
            engine.run(&task.id).await.unwrap_err().code,
            "MODEL_CAPABILITY_MISSING"
        );
        assert_eq!(image_requests(&state.lock().unwrap()), 0);
        let prior = state.lock().unwrap().requests.len();
        let request = agent::context::ContextBuilder::build(&db, &task.id)
            .await
            .unwrap();
        // Adapter 的直接入口也不能绕过能力门槛。
        assert_eq!(
            provider
                .observe(
                    "test",
                    "test",
                    &[("fake-photo".into(), "fake-image".into())],
                    128
                )
                .await
                .unwrap_err()
                .code,
            "MODEL_CAPABILITY_MISSING"
        );
        assert_eq!(state.lock().unwrap().requests.len(), prior);
        assert_eq!(request.task.usage.cloud_images, 0);
        assert_eq!(
            sqlx::query_scalar::<_, i64>("SELECT used FROM privacy_consents WHERE id=?")
                .bind(consent.id)
                .fetch_one(&db.pool)
                .await
                .unwrap(),
            0
        );
        assert_eq!(
            sqlx::query_scalar::<_, i64>("SELECT COUNT(*) FROM photo_observations")
                .fetch_one(&db.pool)
                .await
                .unwrap(),
            0
        );
        assert_eq!(
            serde_json::to_value(db.get(&project.id).await.unwrap()).unwrap(),
            serde_json::to_value(project).unwrap()
        );
    }
    unsafe { std::env::remove_var("BETWEEN_FRAMES_OPENAI_API_KEY") };
}

#[tokio::test]
async fn legacy_binding_without_capability_fingerprint_requires_new_delegation() {
    let _guard = ENV_LOCK.lock().await;
    unsafe { std::env::set_var("BETWEEN_FRAMES_OPENAI_API_KEY", "test-key-abcd") };
    let temp = tempfile::tempdir().unwrap();
    let (db, project) = setup(temp.path(), 2).await;
    let task = start_vision_task(&db, &project, vec![], 0, None).await;
    let (base, state) = spawn_server(Script {
        main: vec!["text".into()],
        vision: VisionReply::Valid,
    });
    let mut identity = serde_json::to_value(
        configured_provider(OpenAiConfig {
            base_url: base.clone(),
            model: "test-model".into(),
            api_key: "test-key-abcd".into(),
        })
        .identity(),
    )
    .unwrap();
    identity
        .as_object_mut()
        .unwrap()
        .remove("capabilityProfileSha256");
    // 只向临时夹具追加上一版合法形状，不改不可变回执/历史触发器。
    sqlx::query("INSERT INTO agent_steps(task_id,kind,body_json,summary,created_at) VALUES(?,'model_binding',?,'legacy binding',?)")
        .bind(&task.id).bind(identity.to_string()).bind(domain::timestamp()).execute(&db.pool).await.unwrap();
    assert_eq!(
        engine(&db, &base).run(&task.id).await.unwrap_err().code,
        "RECOVERY_ERROR"
    );
    assert!(state.lock().unwrap().requests.is_empty());
    assert_eq!(
        agent::projection(&db, &project.id).await.unwrap().tasks[0].state,
        "failed"
    );
    assert!(start_vision_task(&db, &project, vec![], 0, None).await.id != task.id);
    unsafe { std::env::remove_var("BETWEEN_FRAMES_OPENAI_API_KEY") };
}

#[tokio::test]
async fn observation_cache_is_scoped_to_endpoint_even_when_model_alias_matches() {
    let _guard = ENV_LOCK.lock().await;
    unsafe { std::env::set_var("BETWEEN_FRAMES_OPENAI_API_KEY", "test-key-abcd") };
    let temp = tempfile::tempdir().unwrap();
    let (db, project) = setup(temp.path(), 2).await;
    let script = || Script {
        main: ["tool:list_photos", "tool:inspect_from_history", "text"]
            .repeat(2)
            .into_iter()
            .map(str::to_owned)
            .collect(),
        vision: VisionReply::Valid,
    };
    let (a, state_a) = spawn_server(script());
    let (b, state_b) = spawn_server(script());
    for base in [&a, &b, &b] {
        let consent = grant(&db, &project, "project", base).await;
        let task = start_vision_task(&db, &project, vec![], 2, Some(consent.id)).await;
        engine(&db, base).run(&task.id).await.unwrap();
    }
    assert_eq!(image_requests(&state_a.lock().unwrap()), 1);
    assert_eq!(
        image_requests(&state_b.lock().unwrap()),
        1,
        "新端点不能命中旧端点缓存，同端点新任务可复用"
    );
    let versions: Vec<String> =
        sqlx::query_scalar("SELECT DISTINCT prompt_version FROM photo_observations")
            .fetch_all(&db.pool)
            .await
            .unwrap();
    assert_eq!(versions.len(), 2);
    assert!(
        versions
            .iter()
            .all(|value| value.starts_with(&format!("{}@", agent::vision::VISION_PROMPT_VERSION)))
    );
    unsafe { std::env::remove_var("BETWEEN_FRAMES_OPENAI_API_KEY") };
}

#[tokio::test]
async fn cloud_search_reads_only_current_bound_endpoint_facts_without_image_transfers() {
    let _guard = ENV_LOCK.lock().await;
    unsafe { std::env::set_var("BETWEEN_FRAMES_OPENAI_API_KEY", "test-key-abcd") };
    let temp = tempfile::tempdir().unwrap();
    let (db, project) = setup(temp.path(), 2).await;
    let (a, state_a) = spawn_server(Script {
        main: [
            "tool:list_photos",
            "tool:inspect_from_history",
            "text",
            "tool:search_cache",
            "text",
        ]
        .into_iter()
        .map(str::to_owned)
        .collect(),
        vision: VisionReply::Valid,
    });
    let (b, state_b) = spawn_server(Script {
        main: ["tool:search_cache", "text"]
            .into_iter()
            .map(str::to_owned)
            .collect(),
        vision: VisionReply::Valid,
    });
    let consent = grant(&db, &project, "project", &a).await;
    let first = start_vision_task(&db, &project, vec![], 2, Some(consent.id)).await;
    engine(&db, &a).run(&first.id).await.unwrap();
    for (base, count) in [(&b, 0), (&a, 2)] {
        let task = start_vision_task(&db, &project, vec![], 0, None).await;
        engine(&db, base).run(&task.id).await.unwrap();
        let raw: String = sqlx::query_scalar(
            "SELECT result_json FROM tool_calls WHERE task_id=? AND tool_name='search_photos'",
        )
        .bind(&task.id)
        .fetch_one(&db.pool)
        .await
        .unwrap();
        let reply: Value = serde_json::from_str(&raw).unwrap();
        assert_eq!(reply["ok"], true);
        assert_eq!(reply["result"]["photos"].as_array().unwrap().len(), count);
        assert_eq!(reply["result"]["new_uploads"], 0);
    }
    assert_eq!(image_requests(&state_a.lock().unwrap()), 1);
    assert_eq!(image_requests(&state_b.lock().unwrap()), 0);
    unsafe { std::env::remove_var("BETWEEN_FRAMES_OPENAI_API_KEY") };
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
            provider: recipient(&base, "test-model"),
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
    let used = consent::effective_view(&db, &p.id, &recipient(&base, "test-model"))
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
    let used2 = consent::effective_view(&db, &p.id, &recipient(&base, "test-model"))
        .await
        .unwrap()
        .unwrap()
        .used;
    assert_eq!(used2, 2);
    unsafe { std::env::remove_var("BETWEEN_FRAMES_OPENAI_API_KEY") };
}

#[tokio::test]
async fn invalid_cached_observations_require_fresh_authorized_upload() {
    let _guard = ENV_LOCK.lock().await;
    unsafe { std::env::set_var("BETWEEN_FRAMES_OPENAI_API_KEY", "test-key-abcd") };
    for corrupt in [
        "not-json",
        "{}",
        r#"{"photo_id":"foreign","visible_facts":["事实"],"interpretation":"可能","uncertainty":"不确定"}"#,
    ] {
        let temp = tempfile::tempdir().unwrap();
        let (db, p) = setup(temp.path(), 2).await;
        let (base, state) = spawn_server(Script {
            main: ["tool:list_photos", "tool:inspect_from_history", "text"]
                .repeat(2)
                .into_iter()
                .map(str::to_owned)
                .collect(),
            vision: VisionReply::Valid,
        });
        let consent = grant(&db, &p, "project", &base).await;

        let ids: Vec<_> = p.photos.iter().map(|p| p.id.clone()).collect();
        let first = start_vision_task(&db, &p, ids.clone(), 2, Some(consent.id)).await;
        engine(&db, &base).run(&first.id).await.unwrap();
        sqlx::query("UPDATE photo_observations SET observation_json=?")
            .bind(corrupt)
            .execute(&db.pool)
            .await
            .unwrap();
        let second = start_vision_task(&db, &p, ids.clone(), 2, Some(consent.id)).await;
        engine(&db, &base).run(&second.id).await.unwrap();
        let view = agent::projection(&db, &p.id).await.unwrap();
        let task = view.tasks.iter().find(|t| t.id == second.id).unwrap();
        assert_eq!(task.usage.cloud_images, 2);
        assert_eq!(image_requests(&state.lock().unwrap()), 2);
        assert_eq!(
            consent::effective_view(&db, &p.id, &recipient(&base, "test-model"))
                .await
                .unwrap()
                .unwrap()
                .used,
            4
        );
        let raw: String = sqlx::query_scalar(
            "SELECT result_json FROM tool_calls WHERE task_id=? AND tool_name='inspect_photos'",
        )
        .bind(&second.id)
        .fetch_one(&db.pool)
        .await
        .unwrap();
        let result: Value = serde_json::from_str(&raw).unwrap();
        assert_eq!(result["result"]["cache_hits"], 0);
        for item in result["result"]["observations"].as_array().unwrap() {
            assert!(ids.contains(&item["photo_id"].as_str().unwrap().to_owned()));
            assert!(!item["visible_facts"].as_array().unwrap().is_empty());
        }
        assert_eq!(db.get(&p.id).await.unwrap().revision, p.revision);
    }
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
            provider: recipient(&base, "test-model"),
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
    let used = consent::effective_view(&db, &p.id, &recipient(&base, "test-model"))
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
            provider: recipient(&base, "test-model"),
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
    assert_eq!(
        projection.tasks[0].budget_issue.as_ref().unwrap().kind,
        agent::budget::LimitKind::Images
    );
    assert!(projection.tasks[0].reason.contains("照片次数"));
    assert_eq!(image_requests(&state.lock().unwrap()), 0);
    assert_eq!(projection.tasks[0].usage.cloud_images, 0);
    db.pool.close().await;
    let reopened = Store::open(&temp.path().join("projects.sqlite"))
        .await
        .unwrap();
    let restored = agent::projection(&reopened, &p.id).await.unwrap();
    assert_eq!(
        restored.tasks[0].budget_issue.as_ref().unwrap().kind,
        agent::budget::LimitKind::Images
    );
    unsafe { std::env::remove_var("BETWEEN_FRAMES_OPENAI_API_KEY") };
}

#[tokio::test]
async fn runtime_router_rejects_region_budget_and_missing_provider_before_http() {
    use agent::router::{RegionPolicy, ServiceTier};
    let _guard = ENV_LOCK.lock().await;
    unsafe { std::env::set_var("BETWEEN_FRAMES_OPENAI_API_KEY", "test-key-abcd") };
    for condition in ["region", "budget", "missing"] {
        let temp = tempfile::tempdir().unwrap();
        let (db, project) = setup(temp.path(), 2).await;
        let (base, state) = spawn_server(Script {
            main: vec!["text".into()],
            vision: VisionReply::Valid,
        });
        let task = CommandHandler::new(&db)
            .agent_request(AgentRequest {
                request_id: domain::id(),
                project_id: project.id.clone(),
                command: TaskCommand::StartTask {
                    goal: "验证真实执行入口的路由门槛".into(),
                    selected_photo_ids: vec![],
                    target_count: 2,
                    budget: Budget {
                        max_cost_minor_usd: if condition == "budget" { 1 } else { 100 },
                        ..Budget::default()
                    },
                    consent_id: None,
                },
            })
            .await
            .unwrap();
        assert_eq!(task.provider_id, "openai");
        let engine = match condition {
            "region" => engine(&db, &base).with_routing_policy(
                RegionPolicy {
                    restricted: vec!["EU_NO_EXPORT".into()],
                    allowed_residencies: vec!["local".into()],
                },
                ServiceTier::Standard,
            ),
            "missing" => AgentEngine::with_providers(
                db.clone(),
                Arc::new(agent::provider::MockProvider),
                None,
            ),
            _ => engine(&db, &base),
        };
        let result = engine.run(&task.id).await;
        let task = agent::projection(&db, &project.id)
            .await
            .unwrap()
            .tasks
            .into_iter()
            .find(|t| t.id == task.id)
            .unwrap();
        match condition {
            "budget" => {
                assert!(result.is_ok());
                assert_eq!(task.state, "paused");
                assert_eq!(
                    task.budget_issue.as_ref().unwrap().kind,
                    agent::budget::LimitKind::Cost
                );
            }
            "region" => {
                assert_eq!(result.unwrap_err().code, "PRIVACY_REGION_REJECTED");
                assert_eq!(task.state, "failed");
            }
            _ => {
                assert_eq!(result.unwrap_err().code, "MODEL_CAPABILITY_MISSING");
                assert_eq!(task.state, "failed");
            }
        }
        assert!(
            state.lock().unwrap().requests.is_empty(),
            "{condition} 路由拒绝后不得调用 HTTP"
        );
        assert_eq!(task.usage.model_turns, 0);
    }
    unsafe { std::env::remove_var("BETWEEN_FRAMES_OPENAI_API_KEY") };
}

#[tokio::test]
async fn runtime_router_keeps_offline_task_local_when_cloud_adapter_is_available() {
    let _guard = ENV_LOCK.lock().await;
    unsafe { std::env::set_var("BETWEEN_FRAMES_OPENAI_API_KEY", "") };
    let temp = tempfile::tempdir().unwrap();
    let (db, project) = setup(temp.path(), 2).await;
    let task = start_vision_task(&db, &project, vec![], 0, None).await;
    assert_eq!(task.provider_id, "mock");
    let (base, state) = spawn_server(Script {
        main: vec!["text".into()],
        vision: VisionReply::Valid,
    });
    engine(&db, &base).run(&task.id).await.unwrap();
    assert!(state.lock().unwrap().requests.is_empty());
    let task = agent::projection(&db, &project.id)
        .await
        .unwrap()
        .tasks
        .remove(0);
    assert_eq!(task.state, "review_ready");
    assert_eq!(task.usage.cost_minor_usd, 0);
    assert_eq!(task.provider_id, "mock");
    unsafe { std::env::remove_var("BETWEEN_FRAMES_OPENAI_API_KEY") };
}

// 供应商收件人授权：仅本地 HTTP、临时库和生成图，不使用真实服务。
fn consent_request(project: &Project, provider: String, mode: &str) -> ConsentRequest {
    ConsentRequest {
        request_id: domain::id(),
        project_id: project.id.clone(),
        mode: mode.into(),
        provider,
        quota: if mode == "deny" { 0 } else { 10 },
    }
}
#[tokio::test]
async fn consent_isolated_by_endpoint_and_model_before_any_request_with_wrong_explicit_grant() {
    let _guard = ENV_LOCK.lock().await;
    unsafe { std::env::set_var("BETWEEN_FRAMES_OPENAI_API_KEY", "test-key-abcd") };
    let temp = tempfile::tempdir().unwrap();
    let (db, project) = setup(temp.path(), 2).await;
    let script = || Script {
        main: ["tool:list_photos", "tool:inspect_from_history", "text"]
            .repeat(2)
            .into_iter()
            .map(str::to_owned)
            .collect(),
        vision: VisionReply::Valid,
    };
    let (a, state_a) = spawn_server(script());
    let (b, state_b) = spawn_server(script());
    let grant_a = engine(&db, &a)
        .set_consent(consent_request(
            &project,
            recipient(&a, "test-model"),
            "project",
        ))
        .await
        .unwrap();
    let wrong = start_vision_task(&db, &project, vec![], 2, Some(grant_a.id)).await;
    assert_eq!(
        engine(&db, &b).run(&wrong.id).await.unwrap_err().code,
        "CONSENT_TARGET_CHANGED"
    );
    assert!(
        state_b.lock().unwrap().requests.is_empty(),
        "错误显式授权在所有模型请求前拒绝"
    );
    let missing = start_vision_task(&db, &project, vec![], 2, None).await;
    engine(&db, &b).run(&missing.id).await.unwrap();
    assert_eq!(
        image_requests(&state_b.lock().unwrap()),
        0,
        "B 不继承 A 的项目授权"
    );
    let grant_b = engine(&db, &b)
        .set_consent(consent_request(
            &project,
            recipient(&b, "test-model"),
            "project",
        ))
        .await
        .unwrap();
    let valid = start_vision_task(&db, &project, vec![], 2, Some(grant_b.id)).await;
    engine(&db, &b).run(&valid.id).await.unwrap();
    assert_eq!(image_requests(&state_b.lock().unwrap()), 1);
    assert_eq!(
        consent::effective_view(&db, &project.id, &recipient(&a, "test-model"))
            .await
            .unwrap()
            .unwrap()
            .id,
        grant_a.id,
        "另一个明确授权不会覆盖已有收件人"
    );
    let next = start_vision_task(&db, &project, vec![], 2, Some(grant_b.id)).await;
    let other_model = AgentEngine::with_providers(
        db.clone(),
        Arc::new(agent::provider::MockProvider),
        Some(Arc::new(configured_provider(OpenAiConfig {
            base_url: b.clone(),
            model: "other-model".into(),
            api_key: "test-key-abcd".into(),
        }))),
    );
    let before = state_b.lock().unwrap().requests.len();
    assert_eq!(
        other_model.run(&next.id).await.unwrap_err().code,
        "CONSENT_TARGET_CHANGED"
    );
    assert_eq!(state_b.lock().unwrap().requests.len(), before);
    engine(&db, &b)
        .set_consent(consent_request(&project, "openai".into(), "deny"))
        .await
        .unwrap();
    for base in [&a, &b] {
        assert_eq!(
            consent::effective_view(&db, &project.id, &recipient(base, "test-model"))
                .await
                .unwrap()
                .unwrap()
                .mode,
            "deny"
        );
    }
    let denied = start_vision_task(&db, &project, vec![], 2, None).await;
    engine(&db, &a).run(&denied.id).await.unwrap();
    assert_eq!(image_requests(&state_a.lock().unwrap()), 0);
    // 重新明确批准 A，不会复活 B 被全局撤回的授权。
    engine(&db, &a)
        .set_consent(consent_request(
            &project,
            recipient(&a, "test-model"),
            "project",
        ))
        .await
        .unwrap();
    assert!(
        consent::effective_view(&db, &project.id, &recipient(&b, "test-model"))
            .await
            .unwrap()
            .is_none()
    );
    unsafe { std::env::remove_var("BETWEEN_FRAMES_OPENAI_API_KEY") };
}

#[tokio::test]
async fn stale_recipient_ui_cannot_grant_and_deny_works_without_a_usable_provider() {
    let _guard = ENV_LOCK.lock().await;
    let temp = tempfile::tempdir().unwrap();
    let (db, project) = setup(temp.path(), 1).await;
    let (a, state_a) = spawn_server(Script {
        main: vec![],
        vision: VisionReply::Valid,
    });
    let (b, state_b) = spawn_server(Script {
        main: vec![],
        vision: VisionReply::Valid,
    });
    let displayed = engine(&db, &a).ai_config().consent_provider.unwrap();
    assert_eq!(
        engine(&db, &b)
            .set_consent(consent_request(&project, displayed, "task"))
            .await
            .unwrap_err()
            .code,
        "CONSENT_TARGET_CHANGED"
    );
    let count: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM privacy_consents")
        .fetch_one(&db.pool)
        .await
        .unwrap();
    assert_eq!(count, 0);
    assert!(state_a.lock().unwrap().requests.is_empty());
    assert!(state_b.lock().unwrap().requests.is_empty());
    let unavailable =
        AgentEngine::with_providers(db.clone(), Arc::new(agent::provider::MockProvider), None);
    unavailable
        .set_consent(consent_request(&project, "openai".into(), "deny"))
        .await
        .unwrap();
    assert_eq!(
        consent::effective_view(&db, &project.id, &recipient(&b, "test-model"))
            .await
            .unwrap()
            .unwrap()
            .mode,
        "deny"
    );
    let invalid = AgentEngine::with_providers(
        db.clone(),
        Arc::new(agent::provider::MockProvider),
        Some(Arc::new(OpenAiProvider::new(OpenAiConfig {
            base_url: b.clone(),
            model: "unknown".into(),
            api_key: "test-key-abcd".into(),
        }))),
    );
    assert!(
        invalid
            .set_consent(consent_request(
                &project,
                invalid.ai_config().consent_provider.unwrap(),
                "project"
            ))
            .await
            .is_err()
    );
    invalid
        .set_consent(consent_request(&project, "openai".into(), "deny"))
        .await
        .unwrap();
}

#[tokio::test]
async fn recipient_identity_preserves_key_rotation_and_is_specific_to_destination_and_model() {
    let base = "https://service.example.invalid/v1";
    let make = |base: &str, model: &str, key: &str| {
        OpenAiProvider::new(OpenAiConfig {
            base_url: base.into(),
            model: model.into(),
            api_key: key.into(),
        })
    };
    let primary = make(base, "model-a", "private-key-a")
        .identity()
        .consent_provider();
    assert!(consent::is_recipient(&primary));
    assert_eq!(
        primary,
        make(&format!("{base}/"), "model-a", "rotated-key")
            .identity()
            .consent_provider()
    );
    assert_ne!(
        primary,
        make(base, "model-b", "private-key-a")
            .identity()
            .consent_provider()
    );
    assert_ne!(
        primary,
        make(
            "https://other.example.invalid/v1",
            "model-a",
            "private-key-a"
        )
        .identity()
        .consent_provider()
    );
    assert!(!primary.contains("private-key"));
    assert!(!primary.contains("service.example"));
    for invalid in [
        "openai",
        "mock",
        "openai@abc",
        "openai@../../file",
        "openai@",
    ] {
        assert!(!consent::is_recipient(invalid));
    }
}

#[tokio::test]
async fn legacy_unbound_consent_is_not_upgraded_or_used_by_new_tasks() {
    let _guard = ENV_LOCK.lock().await;
    unsafe { std::env::set_var("BETWEEN_FRAMES_OPENAI_API_KEY", "test-key-abcd") };
    let temp = tempfile::tempdir().unwrap();
    let (db, project) = setup(temp.path(), 2).await;
    let (base, state) = spawn_server(Script {
        main: ["tool:list_photos", "tool:inspect_from_history", "text"]
            .repeat(2)
            .into_iter()
            .map(str::to_owned)
            .collect(),
        vision: VisionReply::Valid,
    });
    let legacy=sqlx::query("INSERT INTO privacy_consents(project_id,mode,provider,quota,used,granted_at) VALUES(?,'project','openai',10,0,'legacy')")
        .bind(&project.id).execute(&db.pool).await.unwrap().last_insert_rowid();
    assert!(
        consent::effective_view(&db, &project.id, &recipient(&base, "test-model"))
            .await
            .unwrap()
            .is_none()
    );
    let rejected = CommandHandler::new(&db)
        .agent_request(AgentRequest {
            request_id: domain::id(),
            project_id: project.id.clone(),
            command: TaskCommand::StartTask {
                goal: "旧授权不能绑定新任务".into(),
                selected_photo_ids: vec![],
                target_count: 2,
                budget: Budget {
                    max_cloud_images: 2,
                    ..Budget::default()
                },
                consent_id: Some(legacy),
            },
        })
        .await
        .unwrap_err();
    assert_eq!(rejected.code, "PERMISSION_DENIED");
    let task = start_vision_task(&db, &project, vec![], 2, None).await;
    engine(&db, &base).run(&task.id).await.unwrap();
    assert_eq!(image_requests(&state.lock().unwrap()), 0);
    let raw: String = sqlx::query_scalar("SELECT provider FROM privacy_consents WHERE id=?")
        .bind(legacy)
        .fetch_one(&db.pool)
        .await
        .unwrap();
    assert_eq!(raw, "openai", "不能回填猜测旧授权的服务身份");
    engine(&db, &base)
        .set_consent(consent_request(
            &project,
            recipient(&base, "test-model"),
            "project",
        ))
        .await
        .unwrap();
    let fresh = start_vision_task(&db, &project, vec![], 2, None).await;
    engine(&db, &base).run(&fresh.id).await.unwrap();
    assert_eq!(image_requests(&state.lock().unwrap()), 1);
    unsafe { std::env::remove_var("BETWEEN_FRAMES_OPENAI_API_KEY") };
}

#[tokio::test]
async fn recipient_receipt_replay_conflicts_and_replacement_failure_are_atomic() {
    let _guard = ENV_LOCK.lock().await;
    let temp = tempfile::tempdir().unwrap();
    let (db, project) = setup(temp.path(), 1).await;
    let (a, _) = spawn_server(Script {
        main: vec![],
        vision: VisionReply::Valid,
    });
    let (b, _) = spawn_server(Script {
        main: vec![],
        vision: VisionReply::Valid,
    });
    let request = consent_request(&project, recipient(&a, "test-model"), "project");
    let first = engine(&db, &a).set_consent(request.clone()).await.unwrap();
    assert_eq!(
        engine(&db, &a)
            .set_consent(request.clone())
            .await
            .unwrap()
            .id,
        first.id
    );
    assert_eq!(
        engine(&db, &b)
            .set_consent(ConsentRequest {
                provider: recipient(&b, "test-model"),
                ..request
            })
            .await
            .unwrap_err()
            .code,
        "IDEMPOTENCY_CONFLICT"
    );
    sqlx::query("CREATE TRIGGER fail_consent BEFORE INSERT ON privacy_consents BEGIN SELECT RAISE(ABORT,'test fault'); END").execute(&db.pool).await.unwrap();
    assert!(
        engine(&db, &a)
            .set_consent(consent_request(
                &project,
                recipient(&a, "test-model"),
                "project"
            ))
            .await
            .is_err()
    );
    assert_eq!(
        consent::effective_view(&db, &project.id, &recipient(&a, "test-model"))
            .await
            .unwrap()
            .unwrap()
            .id,
        first.id
    );
    let count: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM consent_receipts")
        .fetch_one(&db.pool)
        .await
        .unwrap();
    assert_eq!(count, 1);
}

#[tokio::test]
async fn archived_project_and_generic_positive_consents_cannot_be_created() {
    let _guard = ENV_LOCK.lock().await;
    let temp = tempfile::tempdir().unwrap();
    let (db, project) = setup(temp.path(), 1).await;
    for provider in ["openai", "mock", "openai@short"] {
        assert!(
            consent::set(
                &db,
                consent::SetConsent {
                    request_id: domain::id(),
                    project_id: project.id.clone(),
                    provider: provider.into(),
                    mode: "project".into(),
                    quota: 10
                }
            )
            .await
            .is_err()
        );
    }
    sqlx::query("UPDATE projects SET deleted_at='test' WHERE id=?")
        .bind(&project.id)
        .execute(&db.pool)
        .await
        .unwrap();
    assert_eq!(
        consent::set(
            &db,
            consent::SetConsent {
                request_id: domain::id(),
                project_id: project.id.clone(),
                provider: recipient("https://service.example.invalid", "m"),
                mode: "project".into(),
                quota: 10
            }
        )
        .await
        .unwrap_err()
        .code,
        "PERMISSION_DENIED"
    );
}

#[tokio::test]
async fn http_redirect_does_not_forward_preview_or_authorization_to_another_service() {
    let _guard = ENV_LOCK.lock().await;
    let (destination, destination_state) = spawn_server(Script {
        main: vec![],
        vision: VisionReply::Valid,
    });
    let listener = TcpListener::bind("127.0.0.1:0").unwrap();
    let base = format!("http://{}", listener.local_addr().unwrap());
    let received = std::thread::spawn(move || {
        let (mut stream, _) = listener.accept().unwrap();
        let body = read_body(&mut stream).unwrap();
        let reply = format!(
            "HTTP/1.1 307 Temporary Redirect\r\nLocation: {destination}/responses\r\nContent-Length: 0\r\nConnection: close\r\n\r\n"
        );
        stream.write_all(reply.as_bytes()).unwrap();
        serde_json::from_str::<Value>(&body).unwrap()
    });
    let provider = configured_provider(OpenAiConfig {
        base_url: base,
        model: "test-model".into(),
        api_key: "test-key-abcd".into(),
    });
    let error = provider
        .observe(
            "仅测试重定向",
            "local fixture",
            &[("test-photo".into(), "dGVzdA==".into())],
            128,
        )
        .await
        .unwrap_err();
    assert_eq!(error.code, "MODEL_UNAVAILABLE");
    assert!(error.message.contains("307"));
    assert!(!error.retriable);
    let original = received.join().unwrap();
    assert!(
        original
            .to_string()
            .contains("data:image/jpeg;base64,dGVzdA==")
    );
    assert!(
        destination_state.lock().unwrap().requests.is_empty(),
        "未授权目的端点不能收到任何请求"
    );
}
