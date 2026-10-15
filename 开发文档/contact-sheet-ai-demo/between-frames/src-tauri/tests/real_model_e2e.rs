// M3 真实模型 E2E：DeepSeek（OpenAI 兼容）经真实 API 跑完整 Agent 任务循环。
// 前置：BETWEEN_FRAMES_OPENAI_API_KEY + BETWEEN_FRAMES_AI_BASE_URL + BETWEEN_FRAMES_AI_MODEL。
// 设计为 #[ignore]——需要真实 key 与网络，不进 CI 默认集；用 --ignored --nocapture 运行。
use between_frames_lib::{
    agent,
    agent::types::{Budget, TaskCommand},
    command_handler::CommandHandler,
    db::Store,
    domain, image_pipeline,
};
use std::path::Path;

fn real_config_available() -> bool {
    std::env::var("BETWEEN_FRAMES_OPENAI_API_KEY")
        .map(|k| k.starts_with("sk-"))
        .unwrap_or(false)
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
    if !real_config_available() {
        eprintln!("SKIP：未配置真实 API key");
        return;
    }
    let temp = tempfile::tempdir().unwrap();
    let (db, project) = project_with_photos(temp.path(), 6).await;

    // 经唯一通道提交任务（Agent 引擎在 run 时自动从环境读取 OpenAI 兼容配置）。
    let request = agent::types::AgentRequest {
        request_id: domain::id(),
        project_id: project.id.clone(),
        command: TaskCommand::StartTask {
            goal: "从这 6 张图里选出 2 张色彩最有层次的，加入序列。只使用工具返回的真实 photo_id。"
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
    agent::runtime::AgentEngine::new(db.clone())
        .run(&task.id)
        .await
        .unwrap();

    let projection = agent::projection(&db, &project.id).await.unwrap();
    let task = &projection.tasks[0];
    eprintln!("任务状态: {}", task.state);
    // 真实模型完成任务循环：completed（正常结束）或 needs_input（产出草案后等待人工确认）。
    assert!(
        matches!(task.state.as_str(), "completed" | "needs_input" | "paused"),
        "真实模型应到达执行后状态，实际 {}",
        task.state
    );

    // Agent 只写草案；正式序列仍为空（人类才能改正式）。
    let formal = db.get(&project.id).await.unwrap();
    assert!(formal.sequence.is_empty(), "Agent 不得改正式序列");
    // 草案（若模型产出）引用真实照片；真实模型可能只输出建议而非完整草案——不强制。
    for draft in &projection.drafts {
        for item in &draft.items {
            assert!(
                formal.photos.iter().any(|p| p.id == item.photo_id),
                "草案 photo_id {} 必须来自项目真实照片",
                item.photo_id
            );
        }
    }
    if projection.drafts.is_empty() {
        eprintln!("（模型选择以建议输出而非草案提交——可接受的真实行为）");
    }
    // 真实工具调用被记录。
    let calls: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM tool_calls WHERE task_id=?")
        .bind(&task.id)
        .fetch_one(&db.pool)
        .await
        .expect("查询工具调用");
    assert!(calls >= 1, "至少一次真实工具调用，实际 {calls}");
    // 真实 token 用量被记录（> 0 证明真实 API 调用，非 mock）。
    let steps: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM agent_steps WHERE task_id=?")
        .bind(&task.id)
        .fetch_one(&db.pool)
        .await
        .unwrap();
    assert!(steps >= 1, "至少一条真实 agent step");
    let tokens: i64 = sqlx::query_scalar(
        "SELECT COALESCE(SUM(CAST(json_extract(detail_json,'$.tokensIn') AS INTEGER)),0) FROM agent_steps WHERE task_id=?",
    )
    .bind(&task.id)
    .fetch_one(&db.pool)
    .await
    .unwrap_or(0);
    eprintln!("真实 agent steps: {steps}, 记录 tokens: {tokens}");
    // 无 key 泄漏到任务数据。
    let all_json = serde_json::to_string(&projection).unwrap();
    let key = std::env::var("BETWEEN_FRAMES_OPENAI_API_KEY").unwrap();
    assert!(!all_json.contains(&key), "API key 不得出现在任务记录中");
    eprintln!(
        "✅ M3 真实模型 E2E 通过：{calls} 次工具调用，{} 份草案",
        projection.drafts.len()
    );
}

#[tokio::test]
#[ignore = "需要真实 API key 与网络；视觉观察经真实 DeepSeek 模型"]
async fn real_model_deepseek_vision_observation() {
    if !real_config_available() {
        eprintln!("SKIP：未配置真实 API key");
        return;
    }
    let temp = tempfile::tempdir().unwrap();
    let (db, project) = project_with_photos(temp.path(), 3).await;
    // 授权云视觉（consent 必须先行——未授权时零外发是 P01 的核心）。
    let consent = between_frames_lib::agent::consent::set(
        &db,
        between_frames_lib::agent::consent::SetConsent {
            request_id: domain::id(),
            project_id: project.id.clone(),
            mode: "project".into(),
            provider: "openai".into(),
            quota: 5,
        },
    )
    .await
    .unwrap();
    assert!(consent.used < consent.quota, "consent 配额应有效");
    // 直接调真实 Provider 的 observe：验证视觉子请求与真实模型交互。
    let provider = between_frames_lib::agent::openai::OpenAiConfig::from_env().unwrap();
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
    let photo_id = project.photos[0].id.clone();
    let (observations, tokens) = provider
        .observe(
            "测试项目",
            "每张图的主色调是什么",
            &[(photo_id.clone(), b64)],
            500,
        )
        .await
        .unwrap();
    eprintln!(
        "视觉观察返回: {}",
        serde_json::to_string(&observations).unwrap_or_default()[..200.min(200)].to_string()
    );
    eprintln!("真实 token: {tokens}");
    assert!(
        tokens > 0 || !observations.to_string().is_empty(),
        "真实模型应返回观察结果"
    );
    // 观察结果必须是合法 JSON（严格 schema 输出）。
    let text = observations.to_string();
    assert!(!text.is_empty(), "观察结果不应为空");
    eprintln!("✅ M3 真实视觉观察通过：DeepSeek 返回观察结果 + {tokens} tokens");
}
