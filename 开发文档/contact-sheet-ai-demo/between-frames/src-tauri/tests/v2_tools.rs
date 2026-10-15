// v2.0 M2 工具别名：v2 命名（create_edit_proposal 等）解析为规范工具执行；
// M5 评论工具显式 NOT_IMPLEMENTED；同一 Mock 循环下 v2/v1 混用轨迹合法。
use between_frames_lib::{
    agent::{self, runtime::AgentEngine, types::*},
    command_handler::CommandHandler,
    db::Store,
    domain::Project,
    image_pipeline,
};
use serde_json::json;
use std::{path::Path, sync::Arc};

async fn setup(path: &Path, count: usize) -> (Store, Project) {
    let db = Store::open(&path.join("projects.sqlite")).await.unwrap();
    let project = db.create("别名轨迹", "").await.unwrap();
    for i in 0..count {
        let source = path.join(format!("t{i}.png"));
        image::RgbImage::from_pixel(32, 24, image::Rgb([(i * 71 % 256) as u8, 60, 100]))
            .save(&source)
            .unwrap();
        db.insert_photo(
            &project.id,
            &image_pipeline::prepare(&source, &path.join("cache")).unwrap(),
        )
        .await
        .unwrap();
    }
    let project = db.get(&project.id).await.unwrap();
    (db, project)
}

// 脚本 Provider：用 v2 工具名跑完整循环（list → proposal 创建 → finish）。
struct V2Names;
impl between_frames_lib::agent::provider::ModelProvider for V2Names {
    fn generate<'a>(
        &'a self,
        input: &'a between_frames_lib::agent::context::ModelTurnRequest,
    ) -> std::pin::Pin<
        Box<
            dyn std::future::Future<Output = between_frames_lib::domain::Result<ModelOutcome>>
                + Send
                + 'a,
        >,
    > {
        Box::pin(async move {
            let tool_calls = input
                .protocol
                .iter()
                .filter(|m| m.kind == "tool")
                .filter_map(|m| m.body["name"].as_str().map(String::from))
                .collect::<Vec<_>>();
            let call = |name: &str, args: serde_json::Value| ModelOutcome::ToolCalls {
                calls: vec![ToolCall {
                    call_id: format!("v2-{name}"),
                    name: name.to_owned(),
                    args,
                }],
            };
            let photos = input
                .protocol
                .iter()
                .filter(|m| m.kind == "tool" && m.body["name"] == "list_photos")
                .find_map(|m| m.body["result"]["result"]["photos"].as_array().cloned())
                .unwrap_or_default();
            let ids: Vec<String> = photos
                .iter()
                .take(2)
                .filter_map(|p| p["photo_id"].as_str().map(String::from))
                .collect();
            Ok(if tool_calls.is_empty() {
                call(
                    "list_photos",
                    json!({"project_id": input.task.project_id, "status": null, "limit": 10, "cursor": null}),
                )
            } else if tool_calls.iter().all(|n| n == "list_photos") && !ids.is_empty() {
                call(
                    "create_edit_proposal",
                    json!({
                        "project_id": input.task.project_id,
                        "base_project_revision": input.task.base_revision,
                        "title": "v2 别名草案",
                        "items": ids.iter().enumerate().map(|(pos, id)| json!({"photo_id": id, "proposed_action": "keep", "position": pos, "rationale": "v2 别名轨迹"})).collect::<Vec<_>>(), "summary": "v2 别名草案；人工最终决定",
                    }),
                )
            } else if tool_calls.iter().any(|n| n == "create_edit_proposal") {
                let draft_id = input
                    .protocol
                    .iter()
                    .filter(|m| m.kind == "tool" && m.body["name"] == "create_edit_proposal")
                    .find_map(|m| {
                        m.body["result"]["result"]["draft_id"]
                            .as_str()
                            .map(String::from)
                    })
                    .unwrap_or_default();
                call(
                    "finish_task",
                    json!({"project_id": input.task.project_id, "task_id": input.task.id, "draft_ids": [draft_id], "summary": "v2 别名闭环"}),
                )
            } else {
                ModelOutcome::NoAction {
                    reason: "完成".into(),
                }
            })
        })
    }
}

#[tokio::test]
async fn v2_tool_names_complete_full_loop_and_m5_tools_are_rejected() {
    let temp = tempfile::tempdir().unwrap();
    let (db, p) = setup(temp.path(), 3).await;
    let ids: Vec<String> = p.photos.iter().map(|photo| photo.id.clone()).collect();
    let task = CommandHandler::new(&db)
        .agent_request(AgentRequest {
            request_id: between_frames_lib::domain::id(),
            project_id: p.id.clone(),
            command: TaskCommand::StartTask {
                goal: "v2 名字闭环".into(),
                selected_photo_ids: ids,
                target_count: 2,
                budget: Budget::default(),
                consent_id: None,
            },
        })
        .await
        .unwrap();
    AgentEngine::with_provider(db.clone(), Arc::new(V2Names))
        .run(&task.id)
        .await
        .unwrap();
    // v2 别名完成 review_ready，草案经规范通道落库。
    let projection = agent::projection(&db, &p.id).await.unwrap();
    assert_eq!(projection.tasks[0].state, "review_ready");
    assert_eq!(projection.drafts.len(), 1);
    assert_eq!(projection.drafts[0].title, "v2 别名草案");
    // M5 评论工具：显式 NOT_IMPLEMENTED，不进入白名单。
    assert!(between_frames_lib::agent::policy::ToolRegistry::canonical("get_comments").is_err());
    // M5 工具不在白名单：无论参数如何都被拒绝（这里以空参数触发，亦可在更早的 scope 检查被拒）。
    let rejected = CommandHandler::new(&db)
        .agent_tool(
            &task.id,
            &ToolCall {
                call_id: "v2-comments".into(),
                name: "summarize_review_disagreement".into(),
                args: json!({"project_id": p.id}),
            },
            None,
        )
        .await;
    match rejected {
        Ok(receipt) => assert!(!receipt.ok, "M5 工具不得执行成功"),
        Err(error) => assert_ne!(error.code, ""),
    }
    // Agent 全程未触碰正式状态。
    let project = db.get(&p.id).await.unwrap();
    assert!(project.sequence.is_empty());
}

#[test]
fn v2_router_errors_are_readable_and_scope_never_expands() {
    use between_frames_lib::agent::router::*;
    let free = RegionPolicy::default();
    let restricted = RegionPolicy {
        restricted: vec!["EU_NO_EXPORT".into()],
        allowed_residencies: vec!["local".into()],
    };
    // 1) 能力缺失：仅 mock 可用时请求视觉任务。
    let vision_req = RouteRequest {
        task_type: TaskType::VisionObserve,
        tier: ServiceTier::Standard,
        region: &free,
        budget_remaining_usd: 1.0,
        images_required: true,
        image_consent: true,
    };
    let error = route(&vision_req, false).unwrap_err();
    assert_eq!(error.code, "MODEL_CAPABILITY_MISSING");
    assert!(error.message.contains("vision"));
    // 2) 区域拒绝：受限区域 + openai 可用但驻留不匹配。
    let text_req = RouteRequest {
        task_type: TaskType::AgentOrchestrate,
        tier: ServiceTier::Standard,
        region: &restricted,
        budget_remaining_usd: 1.0,
        images_required: false,
        image_consent: false,
    };
    let ok = route(&text_req, true).unwrap();
    assert_eq!(ok.provider_id, "mock", "受限区域回落本地");
    // 3) 隐私拒绝：需要图像但无授权（openai 可用）——不自动扩大范围。
    let no_consent = RouteRequest {
        task_type: TaskType::Curation,
        tier: ServiceTier::Pro,
        region: &free,
        budget_remaining_usd: 1.0,
        images_required: true,
        image_consent: false,
    };
    let error = route(&no_consent, true).unwrap_err();
    assert_eq!(error.code, "PRIVACY_REGION_REJECTED");
    assert!(error.message.contains("不会自动扩大授权范围"));
    // 4) 预算：剩余 0.01 且需云端视觉（mock 无 vision）。
    let broke = RouteRequest {
        task_type: TaskType::VisionObserve,
        tier: ServiceTier::Standard,
        region: &free,
        budget_remaining_usd: 0.01,
        images_required: true,
        image_consent: true,
    };
    let error = route(&broke, true).unwrap_err();
    assert_eq!(error.code, "BUDGET_EXCEEDED");
    // 5) 正常：有 key + 授权 → openai；无图像任务 → 按质量选云端。
    let ok = route(&vision_req, true).unwrap();
    assert_eq!(ok.provider_id, "openai");
    let ok = route(&text_req, false).unwrap();
    assert_eq!(ok.provider_id, "mock");
    // 6) 可解释投影不含密钥/端点。
    let explained = explain(&vision_req, true);
    assert!(explained["outcome"]["providerId"].is_string());
    assert!(explained.to_string().find("sk-").is_none());
}
