// M3 真实模型验收：OpenAI 兼容提供方；合成图只验证协议，不证明摄影质量。
// 前置：BETWEEN_FRAMES_OPENAI_API_KEY + BETWEEN_FRAMES_AI_BASE_URL + BETWEEN_FRAMES_AI_MODEL。
// 设计为 #[ignore]——需要真实 key 与网络，不进 CI 默认集；用 --ignored --nocapture 运行。
use between_frames_lib::{
    agent,
    agent::types::{AgentProjection, Budget, TaskCommand},
    command_handler::CommandHandler,
    db::Store,
    domain, image_pipeline,
};
use std::path::Path;

fn require_real_config() -> agent::openai::OpenAiConfig {
    let required = |name: &str| {
        std::env::var(name)
            .ok()
            .filter(|s| !s.trim().is_empty())
            .unwrap_or_else(|| panic!("显式运行真实验收必须配置 {name}；缺配置不能记为通过"))
    };
    agent::openai::OpenAiConfig {
        api_key: required(agent::openai::KEY_ENV),
        base_url: required(agent::openai::BASE_URL_ENV)
            .trim_end_matches('/')
            .into(),
        model: required(agent::openai::MODEL_ENV),
    }
}

fn has_reviewable_result(
    view: &AgentProjection,
    project: &domain::Project,
    task_id: &str,
    target: usize,
) -> bool {
    use std::collections::HashSet;
    let Some(task) = view.tasks.iter().find(|t| t.id == task_id) else {
        return false;
    };
    if task.state != "review_ready"
        || task.final_draft_ids.is_empty()
        || task.final_draft_ids.iter().collect::<HashSet<_>>().len() != task.final_draft_ids.len()
        || task.usage.model_turns > task.budget.max_model_turns
        || task.usage.tool_calls > task.budget.max_tool_calls
        || task.usage.cloud_images > task.budget.max_cloud_images
        || task.usage.cost_minor_usd > task.budget.max_cost_minor_usd
        || task
            .usage
            .input_estimate
            .saturating_add(task.usage.output_estimate)
            > task.budget.max_total_tokens
        || task.elapsed_ms >= u64::from(task.budget.max_duration_seconds) * 1000
    {
        return false;
    }
    task.final_draft_ids.iter().all(|id| {
        view.drafts.iter().any(|draft| {
            draft.id == *id
                && draft.task_id == task.id
                && draft.project_id == project.id
                && draft.state == "pending"
                && !draft.stale
                && draft.items.len() == target
                && draft
                    .items
                    .iter()
                    .map(|i| &i.photo_id)
                    .collect::<HashSet<_>>()
                    .len()
                    == target
                && draft.items.iter().enumerate().all(|(position, item)| {
                    item.position == position as i64
                        && project.photos.iter().any(|p| p.id == item.photo_id)
                })
        })
    })
}

async fn project_with_photos(
    path: &Path,
    count: usize,
) -> (Store, between_frames_lib::domain::Project) {
    let db = Store::open(&path.join("projects.sqlite")).await.unwrap();
    let project = db
        .create("真实模型 E2E", "保留色彩有层次的画面")
        .await
        .unwrap();
    for i in 0..count {
        let source = path.join(format!("real{i:02}.png"));
        let shade = (i * 40 + 60) as u8;
        image::RgbImage::from_pixel(640, 480, image::Rgb([shade, shade / 2, 255 - shade]))
            .save(&source)
            .unwrap();
        let prepared = image_pipeline::prepare(&source, &path.join("cache")).unwrap();
        db.insert_photo(&project.id, &prepared).await.unwrap();
    }
    let project = db.get(&project.id).await.unwrap();
    (db, project)
}

#[tokio::test]
#[ignore = "需要真实 API key（BETWEEN_FRAMES_OPENAI_API_KEY）与网络"]
async fn real_model_deepseek_full_agent_loop() {
    let config = require_real_config();
    let temp = tempfile::tempdir().unwrap();
    let (db, project) = project_with_photos(temp.path(), 6).await;

    // 经唯一通道提交任务；执行时显式注入以上配置，避免隐式切换提供方。
    let request = agent::types::AgentRequest {
        request_id: domain::id(),
        project_id: project.id.clone(),
        command: TaskCommand::StartTask {
            goal: "从这 6 张图里形成 2 张的待评审选片草案。只使用工具返回的真实 photo_id；未授权像素观察，依据现有元数据并说明不确定性，不判断画面色彩。使用 finish_task 提交草案，正式序列由人类决定。"
                .into(),
            selected_photo_ids: vec![],
            target_count: 2,
            budget: Budget::default(),
            consent_id: None,
        },
    };
    let task = CommandHandler::new(&db)
        .agent_request(request)
        .await
        .unwrap();
    // 真实模型运行任务循环。
    agent::runtime::AgentEngine::with_providers(
        db.clone(),
        std::sync::Arc::new(agent::provider::MockProvider),
        Some(std::sync::Arc::new(agent::openai::OpenAiProvider::new(
            config,
        ))),
    )
    .run(&task.id)
    .await
    .unwrap();

    let projection = agent::projection(&db, &project.id).await.unwrap();
    let task = projection.tasks.iter().find(|t| t.id == task.id).unwrap();
    eprintln!("任务状态: {}", task.state);
    eprintln!(
        "停止原因: {}；预算用量: {}",
        task.reason,
        serde_json::to_string(&task.usage).unwrap()
    );
    let names: Vec<String> =
        sqlx::query_scalar("SELECT tool_name FROM tool_calls WHERE task_id=? ORDER BY rowid")
            .bind(&task.id)
            .fetch_all(&db.pool)
            .await
            .unwrap();
    eprintln!("工具轨迹: {}", names.join(" → "));
    assert_eq!(task.provider_id, agent::PROVIDER_OPENAI);
    assert!(
        has_reviewable_result(&projection, &project, &task.id, 2),
        "必须在预算内提交引用真实照片的两张草案，实际状态 {}，最终引用 {} 个",
        task.state,
        task.final_draft_ids.len()
    );

    // Agent 只写草案；正式序列仍为空（人类才能改正式）。
    let formal = db.get(&project.id).await.unwrap();
    assert!(formal.sequence.is_empty(), "Agent 不得改正式序列");
    assert_eq!(
        serde_json::to_value(&formal).unwrap(),
        serde_json::to_value(&project).unwrap(),
        "Agent 不得改正式项目"
    );
    for draft in &projection.drafts {
        for item in &draft.items {
            assert!(
                formal.photos.iter().any(|p| p.id == item.photo_id),
                "草案 photo_id {} 必须来自项目真实照片",
                item.photo_id
            );
        }
    }
    // 真实工具调用被记录。
    let calls: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM tool_calls WHERE task_id=?")
        .bind(&task.id)
        .fetch_one(&db.pool)
        .await
        .expect("查询工具调用");
    assert!(calls >= 1, "至少一次真实工具调用，实际 {calls}");
    // 实测 token 单独保留来源记录，费用仍是保守估算，不能冒充账单。
    let steps: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM agent_steps WHERE task_id=?")
        .bind(&task.id)
        .fetch_one(&db.pool)
        .await
        .unwrap();
    assert!(steps >= 1, "至少一条真实 agent step");
    let input_estimate: i64 = sqlx::query_scalar(
        "SELECT COALESCE(SUM(input_estimate),0) FROM model_usage WHERE task_id=?",
    )
    .bind(&task.id)
    .fetch_one(&db.pool)
    .await
    .unwrap();
    assert!(input_estimate > 0);
    let (reports, actual_tokens): (i64, i64) = sqlx::query_as("SELECT COUNT(*),COALESCE(SUM(json_extract(body_json,'$.totalTokens')),0) FROM agent_steps WHERE task_id=? AND kind='model_usage_reported'").bind(&task.id).fetch_one(&db.pool).await.unwrap();
    assert!(
        reports > 0 && actual_tokens > 0,
        "真实验收必须有提供方报告的 token"
    );
    eprintln!(
        "agent steps: {steps}，实测 token 回执: {reports}，total_tokens: {actual_tokens}；费用估算 {} 美分",
        task.usage.cost_minor_usd
    );
    // 无 key 泄漏到任务数据。
    let all_json = serde_json::to_string(&projection).unwrap();
    let key = std::env::var("BETWEEN_FRAMES_OPENAI_API_KEY").unwrap();
    assert!(!all_json.contains(&key), "API key 不得出现在任务记录中");
    eprintln!(
        "真实模型协议与草案完成验收通过：{calls} 次工具调用，{} 份草案；不证明摄影质量",
        projection.drafts.len()
    );
}

#[tokio::test]
#[ignore = "需要真实 API key、显式端点/模型与网络；仅 Provider 视觉协议冒烟"]
async fn real_model_deepseek_vision_observation() {
    // 显式运行只发送下面生成的小图；此测试不经过 AgentEngine，不声称验证 consent/配额。
    let provider = require_real_config();
    let provider = between_frames_lib::agent::openai::OpenAiProvider::new(provider);
    // 生成一张红色测试图（观察应能识别主色）。
    let png = image::RgbImage::from_pixel(120, 90, image::Rgb([200, 60, 50]));
    let mut jpeg_bytes = Vec::new();
    image::DynamicImage::ImageRgb8(png)
        .to_rgb8()
        .write_to(
            &mut std::io::Cursor::new(&mut jpeg_bytes),
            image::ImageFormat::Jpeg,
        )
        .unwrap();
    let b64 = {
        use base64::Engine;
        base64::engine::general_purpose::STANDARD.encode(&jpeg_bytes)
    };
    let photo_id = "synthetic-red".to_owned();
    let (observations, tokens) = provider
        .observe(
            "测试项目",
            "每张图的主色调是什么",
            &[(photo_id.clone(), b64)],
            // 上限包含推理 token；仍限定单张生成图与有界输出。
            4096,
        )
        .await
        .unwrap();
    let text = serde_json::to_string(&observations).unwrap();
    eprintln!(
        "视觉观察返回: {}",
        text.chars().take(200).collect::<String>()
    );
    eprintln!("真实 token: {tokens}");
    assert!(tokens > 0, "提供方必须返回正数 usage.total_tokens");
    let items = observations["observations"].as_array().unwrap();
    assert_eq!(items.len(), 1);
    assert_eq!(items[0]["photo_id"], photo_id);
    assert!(!items[0]["visible_facts"].as_array().unwrap().is_empty());
    eprintln!(
        "Provider 视觉协议冒烟通过：单张生成图、合法结构/ID、{tokens} tokens；非质量与授权验收"
    );
}

#[tokio::test]
async fn completion_gate_rejects_interruption_missing_and_invalid_drafts() {
    let temp = tempfile::tempdir().unwrap();
    let (db, p) = project_with_photos(temp.path(), 4).await;
    let task = CommandHandler::new(&db)
        .agent_request(agent::types::AgentRequest {
            request_id: domain::id(),
            project_id: p.id.clone(),
            command: TaskCommand::StartTask {
                goal: "给两版候选".into(),
                selected_photo_ids: vec![],
                target_count: 2,
                budget: Budget::default(),
                consent_id: None,
            },
        })
        .await
        .unwrap();
    // 明确不提供云 adapter；本测试在默认离线集执行。
    agent::runtime::AgentEngine::with_providers(
        db.clone(),
        std::sync::Arc::new(agent::provider::MockProvider),
        None,
    )
    .run(&task.id)
    .await
    .unwrap();
    let good = agent::projection(&db, &p.id).await.unwrap();
    assert!(has_reviewable_result(&good, &p, &task.id, 2));
    assert_eq!(
        serde_json::to_value(db.get(&p.id).await.unwrap()).unwrap(),
        serde_json::to_value(&p).unwrap()
    );
    for state in ["paused", "needs_input", "completed", "failed", "cancelled"] {
        let mut bad = good.clone();
        bad.tasks[0].state = state.into();
        assert!(!has_reviewable_result(&bad, &p, &task.id, 2), "{state}");
    }
    let mut bad = good.clone();
    bad.drafts.clear();
    assert!(!has_reviewable_result(&bad, &p, &task.id, 2));
    let mut bad = good.clone();
    bad.tasks[0].final_draft_ids.clear();
    assert!(!has_reviewable_result(&bad, &p, &task.id, 2));
    let mut bad = good.clone();
    bad.drafts[0].items[0].photo_id = "hallucinated".into();
    assert!(!has_reviewable_result(&bad, &p, &task.id, 2));
    let mut bad = good.clone();
    bad.drafts[0].task_id = "foreign".into();
    assert!(!has_reviewable_result(&bad, &p, &task.id, 2));
    let mut bad = good.clone();
    bad.tasks[0].usage.cost_minor_usd = bad.tasks[0].budget.max_cost_minor_usd + 1;
    assert!(!has_reviewable_result(&bad, &p, &task.id, 2));
}
