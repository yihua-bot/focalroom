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
                && draft
                    .items
                    .iter()
                    .filter(|i| i.proposed_action == "keep")
                    .count()
                    == target
                && draft
                    .items
                    .iter()
                    .map(|i| &i.photo_id)
                    .collect::<HashSet<_>>()
                    .len()
                    == draft.items.len()
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
    run_real_curation("从这 6 张图里形成 2 张的待评审选片草案。只使用工具返回的真实 photo_id；未授权像素观察，依据现有元数据并说明不确定性，不判断画面色彩。使用 finish_task 提交草案，正式序列由人类决定。", false).await;
}

#[tokio::test]
#[ignore = "需要真实 API key 与网络；独立的确定性元数据任务，不替代开放选片质量"]
async fn real_model_deepseek_filename_curation() {
    run_real_curation("按文件名升序选择最前两张，形成一个待评审选片草案并提交。目标只看文件名顺序；不做视觉、评分、评论、偏好或审美判断。只引用工具返回的 photo_id，正式序列保持不变。", true).await;
}

async fn run_real_curation(goal: &str, filename_order: bool) {
    let config = require_real_config();
    let provider = std::sync::Arc::new(agent::openai::OpenAiProvider::new(config));
    let expected_identity = provider.identity();
    let temp = tempfile::tempdir().unwrap();
    let (db, project) = project_with_photos(temp.path(), 6).await;

    // 经唯一通道提交任务；执行时显式注入以上配置，避免隐式切换提供方。
    let request = agent::types::AgentRequest {
        request_id: domain::id(),
        project_id: project.id.clone(),
        command: TaskCommand::StartTask {
            goal: goal.into(),
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
        Some(provider),
    )
    .run(&task.id)
    .await
    .unwrap();

    let projection = agent::projection(&db, &project.id).await.unwrap();
    let task = projection.tasks.iter().find(|t| t.id == task.id).unwrap();
    let binding: String = sqlx::query_scalar(
        "SELECT body_json FROM agent_steps WHERE task_id=? AND kind='model_binding'",
    )
    .bind(&task.id)
    .fetch_one(&db.pool)
    .await
    .unwrap();
    let binding: agent::model_binding::ModelIdentity = serde_json::from_str(&binding).unwrap();
    assert_eq!(binding, expected_identity, "实测任务须保存实际实例身份");
    let checkpoint: String = sqlx::query_scalar(
        "SELECT context_json FROM agent_checkpoints WHERE task_id=? ORDER BY id DESC LIMIT 1",
    )
    .bind(&task.id)
    .fetch_one(&db.pool)
    .await
    .unwrap();
    let checkpoint: serde_json::Value = serde_json::from_str(&checkpoint).unwrap();
    assert_eq!(checkpoint["modelId"], binding.model_id);
    assert_eq!(
        checkpoint["modelIdentity"],
        serde_json::to_value(&binding).unwrap()
    );
    assert_eq!(task.usage.cloud_images, 0, "纯元数据任务不发送图片");
    eprintln!(
        "实际模型绑定已核对：{}；Prompt：{}；图片发送：0",
        binding.model_id,
        agent::prompt_version(agent::PROVIDER_OPENAI)
    );
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
    if filename_order {
        let mut sorted = project.photos.iter().collect::<Vec<_>>();
        sorted.sort_by(|a, b| a.name.cmp(&b.name));
        let expected = sorted
            .iter()
            .take(2)
            .map(|p| p.id.as_str())
            .collect::<Vec<_>>();
        for id in &task.final_draft_ids {
            let draft = projection.drafts.iter().find(|d| &d.id == id).unwrap();
            assert_eq!(
                draft
                    .items
                    .iter()
                    .filter(|i| i.proposed_action == "keep")
                    .map(|i| i.photo_id.as_str())
                    .collect::<Vec<_>>(),
                expected
            );
        }
    }

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
#[ignore = "需要用户明确授权的照片目录与 preview 上传确认；最多 80 张本地导入、3 张云预览"]
async fn real_model_lumix_authorized_agent_smoke() {
    run_lumix_authorized(false).await;
}

#[tokio::test]
#[ignore = "真实探索策展：80 张授权照片，最多 12 张预览、8 张候选；仍需真人评审"]
async fn real_model_lumix_exploratory_curation() {
    run_lumix_authorized(true).await;
}

async fn run_lumix_authorized(creative: bool) {
    let (target, quota) = if creative { (8, 12) } else { (3, 3) };
    let config = require_real_config();
    let model_label = config.model.clone();
    assert_eq!(
        config.base_url, "https://api.deepseek.com",
        "真实素材仅授权到已确认的 DeepSeek 端点"
    );
    assert_eq!(
        std::env::var("BETWEEN_FRAMES_REAL_PREVIEW_AUTHORIZATION").as_deref(),
        Ok(if creative {
            "deepseek-1400px-max12"
        } else {
            "deepseek-1400px-max3"
        })
    );
    let root = std::path::PathBuf::from(
        std::env::var("BETWEEN_FRAMES_REAL_PHOTO_DIR").expect("显式提供已授权目录"),
    );
    let root = root.canonicalize().unwrap();
    let mut sources = vec![];
    for folder in std::fs::read_dir(&root).unwrap() {
        let folder = folder.unwrap();
        // 不跟随链接，不越过用户提供的目录；当前相机 DCIM 只有一层相册。
        if !folder.file_type().unwrap().is_dir() {
            continue;
        }
        for file in std::fs::read_dir(folder.path()).unwrap() {
            let file = file.unwrap();
            if file.file_type().unwrap().is_file()
                && file
                    .path()
                    .extension()
                    .is_some_and(|e| e.eq_ignore_ascii_case("jpg"))
                && !file.file_name().to_string_lossy().starts_with('.')
            {
                sources.push(file.path());
            }
        }
    }
    sources.sort();
    sources.truncate(80);
    assert_eq!(sources.len(), 80, "需要至少 80 张真实 JPEG，不以合成图补齐");
    let mut cached_photos = vec![];
    let directory = if let Ok(previous) = std::env::var("BETWEEN_FRAMES_REAL_REUSE_FIXTURE") {
        let previous = std::path::PathBuf::from(previous).canonicalize().unwrap();
        assert!(previous.starts_with(std::env::temp_dir().canonicalize().unwrap()));
        assert!(
            previous
                .file_name()
                .unwrap()
                .to_string_lossy()
                .starts_with("bf-lumix-agent-")
        );
        assert!(previous.join("projects.sqlite").is_file());
        let fixture = Store::open(&previous.join("projects.sqlite"))
            .await
            .unwrap();
        let ids: Vec<String> = sqlx::query_scalar("SELECT id FROM projects")
            .fetch_all(&fixture.pool)
            .await
            .unwrap();
        assert_eq!(ids.len(), 1);
        let prior = fixture.get(&ids[0]).await.unwrap();
        assert_eq!(prior.title, "授权相机素材工程冒烟");
        assert!(
            prior
                .photos
                .iter()
                .all(|p| sources.contains(&std::path::PathBuf::from(&p.original_path)))
        );
        cached_photos = prior.photos;
        // 只复用已校验像素；旧任务/旧配额不续接，新任务使用新库和新 consent。
        tempfile::Builder::new()
            .prefix("bf-lumix-agent-")
            .tempdir()
            .unwrap()
            .keep()
    } else {
        tempfile::Builder::new()
            .prefix("bf-lumix-agent-")
            .tempdir()
            .unwrap()
            .keep()
    };
    let db = Store::open(&directory.join("projects.sqlite"))
        .await
        .unwrap();
    let existing: Vec<String> = sqlx::query_scalar("SELECT id FROM projects")
        .fetch_all(&db.pool)
        .await
        .unwrap();
    assert!(existing.len() <= 1, "只复用本测试的独立预处理库");
    let tasks: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM agent_tasks")
        .fetch_one(&db.pool)
        .await
        .unwrap();
    assert_eq!(tasks, 0, "已发起过模型请求的目录不得作为新任务自动重传");
    let project = if let Some(id) = existing.first() {
        let project = db.get(id).await.unwrap();
        assert_eq!(project.title, "授权相机素材工程冒烟");
        assert!(
            project
                .photos
                .iter()
                .all(|p| sources.contains(&std::path::PathBuf::from(&p.original_path)))
        );
        project
    } else {
        db.create(
            if creative {
                "LUMIX · 光线与空间"
            } else {
                "授权相机素材工程冒烟"
            },
            if creative {
                "以光线与空间的变化形成有节奏的短序列；仅评估实际观察的抽样候选"
            } else {
                "按文件名选最前 3 张，逐张观察，形成初审草案；此目标不代表摄影质量验收"
            },
        )
        .await
        .unwrap()
    };
    let mut hashes = vec![];
    for source in &sources {
        if let Some(photo) = cached_photos
            .iter()
            .find(|p| std::path::Path::new(&p.original_path) == source)
        {
            assert_eq!(
                image_pipeline::fingerprint(source).unwrap(),
                photo.content_sha256
            );
            let dimensions = image::image_dimensions(&photo.preview_path).unwrap();
            assert!(dimensions.0.max(dimensions.1) <= 1400);
            hashes.push(photo.content_sha256.clone());
            db.insert_photo(
                &project.id,
                &image_pipeline::PreparedPhoto {
                    path: source.clone(),
                    filename: photo.name.clone(),
                    hash: photo.content_sha256.clone(),
                    size: u64::try_from(photo.file_size).unwrap(),
                    width: u32::try_from(photo.width).unwrap(),
                    height: u32::try_from(photo.height).unwrap(),
                    thumbnail: photo.thumbnail_path.clone().into(),
                    preview: photo.preview_path.clone().into(),
                },
            )
            .await
            .unwrap();
            continue;
        }
        let prepared = image_pipeline::prepare(source, &directory.join("cache")).unwrap();
        let dimensions = image::image_dimensions(&prepared.preview).unwrap();
        assert!(dimensions.0.max(dimensions.1) <= 1400);
        hashes.push(prepared.hash.clone());
        db.insert_photo(&project.id, &prepared).await.unwrap();
    }
    let project = db.get(&project.id).await.unwrap();
    assert_eq!(project.photos.len(), 80);
    let consent = agent::consent::set(
        &db,
        agent::consent::SetConsent {
            request_id: domain::id(),
            project_id: project.id.clone(),
            mode: "task".into(),
            provider: agent::PROVIDER_OPENAI.into(),
            quota,
        },
    )
    .await
    .unwrap();
    let task = CommandHandler::new(&db).agent_request(agent::types::AgentRequest {
        request_id: domain::id(), project_id: project.id.clone(),
        command: TaskCommand::StartTask {
            goal: if creative {"目标：用光线与空间的变化形成一组有节奏的 8 张短序列。先 list_photos 获取这 80 张的文件名，在整组文件名顺序中均匀抽样最多 12 张（避免只看相邻连拍），每批最多 3 张 inspect_photos。仅从实际观察的候选中选择 8 张 keep 并排序，理由区分可观察事实与主观解释，说明未观察其他照片的局限，然后 finish_task；不要把未观察照片写成已检查，也不作地点/身份推断。此为探索性试编，最终取舍由人类评审；不探索评论或私人偏好。"} else {"先用 list_photos 获取文件名，按文件名升序取最前 3 张；只对这 3 张调用 inspect_photos 做有授权的像素观察，不扩大候选。形成恰好 3 张 keep 的一个草案，每项理由引用实际可观察事实并标注解释的不确定性，然后 finish_task。其他 77 张无需写入草案；不探索评论或偏好。这是固定候选的真实视觉链路工程测试，不是审美优劣评价。"}.into(),
            selected_photo_ids: vec![], target_count: target,
            budget: Budget { max_cloud_images: quota as u32, ..Budget::default() }, consent_id: Some(consent.id),
        },
    }).await.unwrap();
    std::fs::write(directory.join("registration.json"), serde_json::to_vec_pretty(&serde_json::json!({"taskId":task.id,"goal":task.goal,"target":target,"registeredAt":domain::timestamp(),"model":model_label,"promptVersion":agent::prompt_version(agent::PROVIDER_OPENAI),"budget":task.budget,"tier":"standard","effectiveMaxCostMinorUsd":50})).unwrap()).unwrap();
    let run = agent::runtime::AgentEngine::with_providers(
        db.clone(),
        std::sync::Arc::new(agent::provider::MockProvider),
        Some(std::sync::Arc::new(agent::openai::OpenAiProvider::new(
            config,
        ))),
    )
    .run(&task.id)
    .await;
    let projection = agent::projection(&db, &project.id).await.unwrap();
    let formal = db.get(&project.id).await.unwrap();
    let unchanged =
        serde_json::to_value(&formal).unwrap() == serde_json::to_value(&project).unwrap();
    let after: Vec<_> = sources
        .iter()
        .map(|p| image_pipeline::fingerprint(p).unwrap())
        .collect();
    let task = projection.tasks.iter().find(|t| t.id == task.id).unwrap();
    let trace: Vec<(String, String)> = sqlx::query_as(
        "SELECT tool_name,result_json FROM tool_calls WHERE task_id=? ORDER BY rowid",
    )
    .bind(&task.id)
    .fetch_all(&db.pool)
    .await
    .unwrap();
    let actual_tokens: i64 = sqlx::query_scalar("SELECT COALESCE(SUM(json_extract(body_json,'$.totalTokens')),0) FROM agent_steps WHERE task_id=? AND kind='model_usage_reported'").bind(&task.id).fetch_one(&db.pool).await.unwrap();
    let observed: Vec<String> =
        sqlx::query_scalar("SELECT photo_id FROM photo_observations WHERE kind='inspect_photos'")
            .fetch_all(&db.pool)
            .await
            .unwrap();
    let summary = serde_json::json!({"purpose":if creative {"exploratory curation, human review pending"} else {"engineering smoke, not M3 human quality acceptance"},"target":target,"state":task.state,"reason":task.reason,"usage":task.usage,"mainLoopActualTokens":actual_tokens,"formalUnchanged":unchanged,"sourceHashesUnchanged":hashes==after,"importedPhotos":80,"previewMaximumDimension":1400,"consentQuota":quota,"rateCard":"deepseek_flash_peak20261016","toolNames":trace.iter().map(|x|&x.0).collect::<Vec<_>>()});
    let projection_json = serde_json::to_string_pretty(&projection).unwrap();
    let key = std::env::var(agent::openai::KEY_ENV).unwrap();
    assert!(!projection_json.contains(&key));
    std::fs::write(
        directory.join("summary.json"),
        serde_json::to_vec_pretty(&summary).unwrap(),
    )
    .unwrap();
    std::fs::write(directory.join("projection.json"), projection_json).unwrap();
    std::fs::write(
        directory.join("tool-receipts.json"),
        serde_json::to_vec_pretty(&trace).unwrap(),
    )
    .unwrap();
    std::fs::write(
        directory.join("source-hashes.json"),
        serde_json::to_vec_pretty(&serde_json::json!({"before":hashes,"after":after})).unwrap(),
    )
    .unwrap();
    if creative {
        use sha2::{Digest, Sha256};
        let digest = |value: &serde_json::Value| {
            format!("{:x}", Sha256::digest(serde_json::to_vec(value).unwrap()))
        };
        let selected = task
            .final_draft_ids
            .first()
            .and_then(|id| projection.drafts.iter().find(|d| &d.id == id));
        let calls: Vec<(String, String, String)> = sqlx::query_as(
            "SELECT call_id,tool_name,result_json FROM tool_calls WHERE task_id=? ORDER BY rowid",
        )
        .bind(&task.id)
        .fetch_all(&db.pool)
        .await
        .unwrap();
        let tool_trace: Vec<_> = calls.iter().map(|(call_id,name,raw)| {
            let result: serde_json::Value = serde_json::from_str(raw).unwrap();
            let refs = result["result"]["photos"].as_array().into_iter().flatten().filter_map(|p|p["photo_id"].as_str()).collect::<Vec<_>>();
            serde_json::json!({"callId":call_id,"tool":name,"photoIds":refs,"evidenceRef":format!("projects.sqlite#tool_calls/{}/{call_id}",task.id)})
        }).collect();
        let vision_tokens: i64 = sqlx::query_scalar("SELECT COALESCE(SUM(json_extract(body_json,'$.prefetch.tokens')),0) FROM agent_steps WHERE task_id=? AND kind='vision_settled'").bind(&task.id).fetch_one(&db.pool).await.unwrap();
        let (attempts,reports,reserved,confirmed): (i64,i64,i64,i64) = sqlx::query_as("SELECT SUM(kind='model_attempt'),SUM(kind='model_usage_reported'),SUM(kind='vision_reserved'),SUM(kind='vision_settled' AND json_extract(body_json,'$.prefetch.tokens') IS NOT NULL) FROM agent_steps WHERE task_id=?").bind(&task.id).fetch_one(&db.pool).await.unwrap();
        let all_actual_tokens =
            (attempts == reports && reserved == confirmed).then_some(actual_tokens + vision_tokens);
        let evaluation = serde_json::json!({"protocolVersion":"bf-m3-eval-1","datasetId":"lumix-pilot-20261016","registeredRunIds":[task.id],
            "tasks":[{"id":task.id,"creatorId":"local-photographer","goal":task.goal,"genre":"documentary","phase":"first_curation","targetCount":target,"provenance":"photographer",
                "photos":project.photos.iter().map(|p|serde_json::json!({"id":p.id,"sha256":p.content_sha256})).collect::<Vec<_>>(),
                "authorization":{"holderId":"local-photographer","evidenceRef":"User provided /Volumes/LUMIX/DCIM and authorized DeepSeek 1400px previews in parent conversation on 2026-10-16; ownership not independently verified","photoIds":project.photos.iter().map(|p|&p.id).collect::<Vec<_>>(),"cloudPreviewProviders":["deepseek"]},"feedbackEvidenceRef":null}],
            "runs":[{"id":task.id,"taskId":task.id,"system":"between_frames","version":"working-tree-20261016","model":model_label,"promptVersion":agent::prompt_version(agent::PROVIDER_OPENAI),
                "state":if task.state=="review_ready" {"review_ready"} else if ["paused","needs_input","cancelled"].contains(&task.state.as_str()){task.state.as_str()} else {"failed"},
                "orderedPhotoIds":selected.map(|d|d.items.iter().filter(|i|i.proposed_action=="keep").map(|i|&i.photo_id).collect::<Vec<_>>()).unwrap_or_default(),
                "withinBudget":task.usage.cost_minor_usd<=50,"elapsedMs":task.elapsed_ms,"actualTokens":all_actual_tokens,"estimatedCostMinorUsd":task.usage.cost_minor_usd,"billedCostMinorUsd":null,
                "sourceEvidenceRef":"projects.sqlite + registration.json + summary.json; local engineering collector, not independent audit",
                "explanation":selected.map(|d|serde_json::json!({"summary":d.summary,"items":d.items}).to_string()).unwrap_or_else(||task.reason.clone()),"followUp":null,"trace":tool_trace,
                "audit":{"evidenceRef":"formal-before.json + formal-after.json + source-hashes.json","formalBeforeSha256":digest(&serde_json::to_value(&project).unwrap()),"formalAfterSha256":digest(&serde_json::to_value(&formal).unwrap()),"originalsBeforeSha256":digest(&serde_json::json!(hashes)),"originalsAfterSha256":digest(&serde_json::json!(after)),"illegalWriteCount":u32::from(!unchanged)},"cloudPreviewProvider":"deepseek"}],"reviews":[]});
        std::fs::write(
            directory.join("evaluation-input.json"),
            serde_json::to_vec_pretty(&evaluation).unwrap(),
        )
        .unwrap();
        std::fs::write(
            directory.join("formal-before.json"),
            serde_json::to_vec_pretty(&project).unwrap(),
        )
        .unwrap();
        std::fs::write(
            directory.join("formal-after.json"),
            serde_json::to_vec_pretty(&formal).unwrap(),
        )
        .unwrap();
    }
    eprintln!(
        "真实素材工程结果：{}；本地证据 {}",
        summary,
        directory.display()
    );
    assert!(unchanged);
    assert_eq!(hashes, after);
    run.unwrap();
    assert_eq!(task.budget.max_cost_minor_usd, 100);
    assert!(task.usage.cost_minor_usd <= 50, "保持原 Standard 档位上限");
    let cards: Vec<String> = sqlx::query_scalar("SELECT json_extract(body_json,'$.rateCard') FROM agent_steps WHERE task_id=? AND kind IN('model_attempt','vision_reserved')").bind(&task.id).fetch_all(&db.pool).await.unwrap();
    let expected_card =
        serde_json::to_value(agent::pricing::RateCard::DeepseekFlashPeak20261016).unwrap();
    assert!(
        !cards.is_empty()
            && cards
                .iter()
                .all(|card| expected_card.as_str() == Some(card))
    );
    assert!(has_reviewable_result(
        &projection,
        &project,
        &task.id,
        target as usize
    ));
    assert!(
        (target..=quota as u32).contains(&task.usage.cloud_images),
        "必须实际观察候选，元数据占位不能通过"
    );
    assert!(actual_tokens > 0);
    let mut expected = project.photos.iter().collect::<Vec<_>>();
    expected.sort_by(|a, b| a.name.cmp(&b.name));
    let expected: Vec<_> = expected.iter().take(3).map(|p| p.id.as_str()).collect();
    assert!(observed.len() >= target as usize && observed.len() <= quota as usize);
    assert!(
        creative
            || expected
                .iter()
                .all(|id| observed.iter().any(|seen| seen == id))
    );
    for id in &task.final_draft_ids {
        let draft = projection.drafts.iter().find(|d| &d.id == id).unwrap();
        let kept = draft
            .items
            .iter()
            .filter(|i| i.proposed_action == "keep")
            .map(|i| i.photo_id.as_str())
            .collect::<Vec<_>>();
        assert!(kept.iter().all(|id| observed.iter().any(|seen| seen == id)));
        if !creative {
            assert_eq!(kept, expected);
        }
    }
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
    // 草案可同时说明待定/淘汰项；产品审阅页仅把 keep 当作候选序列。
    let mut mixed = good.clone();
    let selected: Vec<_> = mixed.drafts[0]
        .items
        .iter()
        .map(|i| i.photo_id.clone())
        .collect();
    for photo in p.photos.iter().filter(|p| !selected.contains(&p.id)) {
        let position = mixed.drafts[0].items.len() as i64;
        mixed.drafts[0].items.push(agent::types::DraftItem {
            photo_id: photo.id.clone(),
            proposed_action: "undecided".into(),
            position,
            rationale: "待人工查看".into(),
        });
    }
    assert!(has_reviewable_result(&mixed, &p, &task.id, 2));
    for action in ["keep", "reject"] {
        let mut wrong = mixed.clone();
        if action == "keep" {
            wrong.drafts[0].items[2].proposed_action = action.into();
        } else {
            wrong.drafts[0].items[0].proposed_action = action.into();
        }
        assert!(!has_reviewable_result(&wrong, &p, &task.id, 2));
    }
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
