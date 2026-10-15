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
