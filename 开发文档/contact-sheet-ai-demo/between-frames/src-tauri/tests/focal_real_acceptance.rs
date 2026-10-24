//! 显式授权的真实模型自动化验收；默认不外发、不消耗额度，不替代摄影师盲评。
use between_frames_lib::{
    agent::{self, types::*},
    cloud_auth::CloudAuth,
    cloud_credentials::CredentialStore,
    db::Store,
    domain, image_pipeline,
};
use serde_json::json;
use std::{path::PathBuf, sync::Arc};

const HOSTED_ORIGIN: &str = "https://team.focroom.cc";
const HOSTED_ENDPOINT: &str = "https://team.focroom.cc/api/v2/ai/deepseek-flash-v1/responses";
fn required(name: &str) -> String {
    std::env::var(name)
        .ok()
        .filter(|s| !s.trim().is_empty())
        .unwrap_or_else(|| panic!("缺少 {name}，真实验收不能记为通过"))
}
// 只在本验收进程持有会话快照；不改写或删除用户钥匙串，也不把 token 放入日志/数据库。
struct SessionSnapshot(String);
impl CredentialStore for SessionSnapshot {
    fn read(&self, _: &str) -> domain::Result<Option<String>> {
        Ok(Some(self.0.clone()))
    }
    fn write(&self, _: &str, _: &str) -> domain::Result<()> {
        Err(domain::AppError::new(
            "READ_ONLY",
            "真实验收不能改写登录凭据",
        ))
    }
    fn delete(&self, _: &str) -> domain::Result<()> {
        Err(domain::AppError::new(
            "READ_ONLY",
            "真实验收不能删除登录凭据",
        ))
    }
}
fn session_snapshot() -> String {
    let raw = if let Ok(path) = std::env::var("FOCALROOM_REAL_SESSION_FILE") {
        let path = PathBuf::from(path);
        let metadata = std::fs::symlink_metadata(&path).expect("会话文件不可读");
        assert!(
            metadata.is_file() && !metadata.file_type().is_symlink() && metadata.len() <= 32768,
            "会话必须是有界普通文件，不能使用符号链接"
        );
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            assert_eq!(
                metadata.permissions().mode() & 0o077,
                0,
                "会话文件不能允许其他用户读取"
            );
        }
        std::fs::read_to_string(path).expect("会话文件不可读")
    } else {
        #[cfg(target_os = "macos")]
        {
            use security_framework::item::{ItemClass, ItemSearchOptions, SearchResult};
            let namespace = std::env::var("FOCALROOM_REAL_KEYCHAIN_NAMESPACE")
                .unwrap_or_else(|_| "studio.betweenframes.desktop".into());
            if std::env::var("FOCALROOM_REAL_REQUEST_KEYCHAIN_ACCESS").as_deref()
                == Ok("request-system-prompt")
            {
                between_frames_lib::cloud_credentials::SystemCredentials
                    .read(&format!("{namespace}.cloud-session"))
                    .expect("系统未允许读取登录会话")
                    .expect("请先在正式客户端登录")
            } else {
                let result = ItemSearchOptions::new()
                    .class(ItemClass::generic_password())
                    .service(&format!("{namespace}.cloud-session"))
                    .account("cloud-session")
                    .load_data(true)
                    .limit(1)
                    .skip_authenticated_items(true)
                    .search();
                let mut result = result.unwrap_or_else(|e| panic!("没有可非交互读取的登录会话（系统错误 {}）；请在正式客户端登录，或提供私有会话文件路径。未调用模型", e.code()));
                match result.pop() {
                    Some(SearchResult::Data(bytes)) => {
                        String::from_utf8(bytes).expect("会话格式错误")
                    }
                    _ => panic!("没有可用会话，未调用模型"),
                }
            }
        }
        #[cfg(not(target_os = "macos"))]
        {
            panic!("当前平台请提供 FOCALROOM_REAL_SESSION_FILE，未调用模型")
        }
    };
    let value: serde_json::Value = serde_json::from_str(&raw).expect("会话格式错误");
    let session = &value["session"];
    assert!(
        session["token"]
            .as_str()
            .is_some_and(|s| s.len() >= 32 && s.len() <= 4096)
            && session["user"]["id"]
                .as_str()
                .is_some_and(|s| !s.is_empty()),
        "没有已登录账户，未调用模型"
    );
    json!({"pending":null,"session":session}).to_string()
}
async fn hosted_auth() -> Arc<CloudAuth> {
    assert_eq!(
        required("FOCALROOM_REAL_APPROVED_ENDPOINT"),
        HOSTED_ENDPOINT
    );
    assert_eq!(required("FOCALROOM_REAL_APPROVED_MODEL"), "deepseek-flash");
    let auth = Arc::new(
        CloudAuth::new(
            HOSTED_ORIGIN,
            "focal-real-acceptance",
            Arc::new(SessionSnapshot(session_snapshot())),
        )
        .unwrap(),
    );
    let provider = agent::openai::OpenAiProvider::hosted(auth.clone())
        .bind_account()
        .unwrap();
    provider.validate_hosted_account().await.unwrap();
    auth
}
// 可复用上一轮隔离验收已生成的真实像素；仍重读全部原图哈希，不复制旧分析或会话。
async fn prepared_previews()
-> Option<std::collections::HashMap<PathBuf, image_pipeline::PreparedPhoto>> {
    let path = std::env::var("FOCALROOM_REAL_PREPARED_PROJECT_DIR").ok()?;
    let root = PathBuf::from(path)
        .canonicalize()
        .expect("既有验收目录不可读");
    assert!(
        root.file_name()
            .unwrap()
            .to_string_lossy()
            .starts_with("focal-real-300-")
    );
    let record: serde_json::Value =
        serde_json::from_slice(&std::fs::read(root.join("first-result.json")).unwrap()).unwrap();
    assert_eq!(record["sourceHashesUnchanged"], true);
    assert_eq!(record["originalUnchanged"], true);
    let options = sqlx::sqlite::SqliteConnectOptions::new()
        .filename(root.join("projects.sqlite"))
        .read_only(true);
    let pool = sqlx::sqlite::SqlitePoolOptions::new()
        .max_connections(1)
        .connect_with(options)
        .await
        .unwrap();
    use sqlx::Row;
    let rows=sqlx::query("SELECT original_path,filename,content_sha256,file_size,width,height FROM photos ORDER BY import_index").fetch_all(&pool).await.unwrap();
    assert_eq!(rows.len(), 300);
    let mut result = std::collections::HashMap::new();
    for row in rows {
        let path = PathBuf::from(row.get::<String, _>("original_path"));
        let hash: String = row.get("content_sha256");
        assert_eq!(
            image_pipeline::fingerprint(&path).unwrap(),
            hash,
            "真实源文件已改变"
        );
        let thumbnail = root.join("cache").join(format!("{hash}-320.jpg"));
        let preview = root.join("cache").join(format!("{hash}-1400.jpg"));
        for file in [&thumbnail, &preview] {
            let metadata = std::fs::symlink_metadata(file).unwrap();
            assert!(metadata.is_file() && !metadata.file_type().is_symlink());
        }
        result.insert(
            path.clone(),
            image_pipeline::PreparedPhoto {
                path,
                filename: row.get("filename"),
                hash,
                size: row.get::<i64, _>("file_size") as u64,
                width: row.get::<i64, _>("width") as u32,
                height: row.get::<i64, _>("height") as u32,
                thumbnail,
                preview,
            },
        );
    }
    pool.close().await;
    Some(result)
}
#[tokio::test]
#[ignore = "显式批准后只核对生产托管账户与余额；不调用模型、不上传照片"]
async fn hosted_account_preflight_without_model_calls() {
    assert_eq!(
        required("FOCALROOM_REAL_HOSTED_PREFLIGHT_APPROVAL"),
        "account-and-quota-only"
    );
    let auth = hosted_auth().await;
    let quota = auth.ai_account().await.unwrap();
    let summary = json!({"endpoint":HOSTED_ENDPOINT,"model":"deepseek-flash", "authenticated":true,
        "verified":quota["verified"],"enabled":quota["enabled"],"available":quota["available"],
        "spent":quota["spent"],"reserved":quota["reserved"],"modelCalls":0,"uploadedPreviews":0});
    let output = tempfile::Builder::new()
        .prefix("focal-hosted-preflight-")
        .tempdir()
        .unwrap()
        .keep();
    std::fs::write(
        output.join("preflight.json"),
        serde_json::to_vec_pretty(&summary).unwrap(),
    )
    .unwrap();
    eprintln!("托管账户预检（不调用模型）：{}", summary);
    eprintln!("预检证据：{}", output.display());
}
#[tokio::test]
#[ignore = "显式批准后仅发送一张生成色块验证生产异步图片；不使用真实照片，不进离线 CI"]
async fn hosted_generated_preview_async_smoke() {
    assert_eq!(
        required("FOCALROOM_REAL_IMAGE_SMOKE_APPROVAL"),
        "one-generated-preview-max5c"
    );
    let auth = hosted_auth().await;
    let provider = agent::openai::OpenAiProvider::hosted(auth.clone())
        .bind_account()
        .unwrap();
    let before = auth.ai_account().await.unwrap();
    assert_eq!(
        before["reserved"].as_u64(),
        Some(0),
        "仍有待确认预留，未新增模型请求"
    );
    let directory = tempfile::Builder::new()
        .prefix("focal-hosted-image-smoke-")
        .tempdir()
        .unwrap()
        .keep();
    let path = directory.join("generated.png");
    image::RgbImage::from_pixel(64, 64, image::Rgb([255, 0, 0]))
        .save(&path)
        .unwrap();
    let hash = image_pipeline::fingerprint(&path).unwrap();
    let bytes = agent::upload_compress::compress_for_upload(&path).unwrap();
    let encoded = base64::Engine::encode(&base64::engine::general_purpose::STANDARD, &bytes);
    let estimate = provider.rate_card().vision_reservation(1, 8192, 256);
    assert!(estimate <= 5, "小图验证估算不能超过批准的 5 美分");
    let started = std::time::Instant::now();
    let response = provider
        .observe(
            "连接验证素材，不包含真实照片",
            "描述画面主要颜色；只报告可见事实，不猜测拍摄内容。",
            &[("generated-color-1".into(), encoded)],
            256,
        )
        .await;
    let after = auth.ai_account().await.ok();
    let elapsed = started.elapsed().as_millis();
    let unchanged = image_pipeline::fingerprint(&path).unwrap() == hash;
    let (observations, tokens, error) = match &response {
        Ok((result, tokens)) => (Some(result.clone()), Some(*tokens), None),
        Err(error) => (
            None,
            None,
            Some(json!({"code":error.code,"request":error.details})),
        ),
    };
    let color_correct = observations.as_ref().is_some_and(|result| {
        let rows = result["observations"].as_array();
        rows.is_some_and(|rows| {
            rows.len() == 1
                && rows[0]["photo_id"] == "generated-color-1"
                && rows[0]["visible_facts"].as_array().is_some_and(|facts| {
                    facts.iter().any(|fact| {
                        fact.as_str().is_some_and(|text| {
                            text.contains('红') || text.to_lowercase().contains("red")
                        })
                    })
                })
        })
    });
    let report = json!({"endpoint":HOSTED_ENDPOINT,"model":"deepseek-flash","imageSource":"generated-64px-solid-color","previewCount":1,"previewBytes":bytes.len(),"elapsedMs":elapsed,"costEstimateCeilingMinorUsd":5,"preprocessingVersion":agent::vision::VISION_PROMPT_VERSION,"observations":observations,"reportedTokens":tokens,"error":error,"quotaBefore":before,"quotaAfter":after,"generatedSourceUnchanged":unchanged,"colorCorrect":color_correct,"professionalQualityAccepted":false});
    std::fs::write(
        directory.join("result.json"),
        serde_json::to_vec_pretty(&report).unwrap(),
    )
    .unwrap();
    eprintln!("单图异步验证证据：{}", directory.display());
    assert!(unchanged);
    response.unwrap();
    assert!(color_correct, "返回成功不等于图像事实正确，详见验证记录");
    assert_eq!(
        report["quotaAfter"]["reserved"].as_u64(),
        Some(0),
        "须核对请求级计量，不能直接继续发图"
    );
}

#[tokio::test]
#[ignore = "需要显式批准的端点/模型、真实凭据、300 张授权照片和最多 60 张预览上传；不进离线 CI"]
async fn unified_300_real_photos_three_boards_and_followup() {
    assert_eq!(
        required("FOCALROOM_REAL_ACCEPTANCE_APPROVAL"),
        "300-photos-max60-previews-two-tasks"
    );
    let (provider, hosted, secret) = match required("FOCALROOM_REAL_TRANSPORT").as_str() {
        "hosted" => {
            let auth = hosted_auth().await;
            let provider = agent::openai::OpenAiProvider::hosted(auth.clone())
                .bind_account()
                .unwrap();
            let token = auth.bearer(HOSTED_ORIGIN).await.unwrap();
            (Arc::new(provider), Some(auth), token)
        }
        "direct" => {
            let config = agent::openai::OpenAiConfig {
                api_key: required(agent::openai::KEY_ENV),
                base_url: required(agent::openai::BASE_URL_ENV)
                    .trim_end_matches('/')
                    .into(),
                model: required(agent::openai::MODEL_ENV),
            };
            assert_eq!(
                config.base_url,
                required("FOCALROOM_REAL_APPROVED_ENDPOINT")
            );
            assert_eq!(config.model, required("FOCALROOM_REAL_APPROVED_MODEL"));
            assert!(config.base_url.starts_with("https://"));
            let key = config.api_key.clone();
            (
                Arc::new(agent::openai::OpenAiProvider::new(config)),
                None,
                key,
            )
        }
        _ => panic!("明确选择 hosted 或 direct，不能自动切换服务"),
    };
    assert!(
        std::env::var("BETWEEN_FRAMES_ANTHROPIC_API_KEY")
            .unwrap_or_default()
            .is_empty()
    );
    assert!(
        std::env::var("BETWEEN_FRAMES_GEMINI_API_KEY")
            .unwrap_or_default()
            .is_empty()
    );
    let root = PathBuf::from(required("BETWEEN_FRAMES_REAL_PHOTO_DIR"))
        .canonicalize()
        .unwrap();
    let mut files = vec![];
    for folder in std::fs::read_dir(&root).unwrap() {
        let folder = folder.unwrap();
        if !folder.file_type().unwrap().is_dir()
            || folder.file_name().to_string_lossy().starts_with('.')
        {
            continue;
        }
        for file in std::fs::read_dir(folder.path()).unwrap() {
            let file = file.unwrap();
            if file.file_type().unwrap().is_file()
                && !file.file_name().to_string_lossy().starts_with('.')
                && file
                    .path()
                    .extension()
                    .is_some_and(|e| e.eq_ignore_ascii_case("jpg"))
            {
                files.push(file.path());
            }
        }
    }
    files.sort();
    files.truncate(300);
    assert_eq!(files.len(), 300, "不以合成图片填充真实素材");
    let directory = tempfile::Builder::new()
        .prefix("focal-real-300-")
        .tempdir()
        .unwrap()
        .keep();
    eprintln!("真实自动化验收目录：{}", directory.display());
    let db = Store::open(&directory.join("projects.sqlite"))
        .await
        .unwrap();
    let project = db
        .create("摄影创作自动化验收", "从同一批照片寻找三条不同创作线索")
        .await
        .unwrap();
    let mut hashes = vec![];
    let mut reused = prepared_previews().await;
    for file in &files {
        let prepared = if let Some(prepared) = &mut reused {
            prepared
                .remove(&file.canonicalize().unwrap())
                .expect("照片不在已验收像素集合中")
        } else {
            image_pipeline::prepare(file, &directory.join("cache")).unwrap()
        };
        hashes.push(prepared.hash.clone());
        assert!(
            db.insert_photo(&project.id, &prepared).await.unwrap(),
            "真实素材重复，不能虚报 300 张候选"
        );
    }
    let original = db.get(&project.id).await.unwrap();
    assert_eq!(original.photos.len(), 300);
    let first_cost_cap: u32 = std::env::var("FOCALROOM_REAL_FIRST_TASK_COST_CAP_MINOR")
        .unwrap_or_else(|_| "50".into())
        .parse()
        .expect("费用上限须为整数美分");
    assert!(
        (1..=50).contains(&first_cost_cap),
        "真实验收不能放大既有上限"
    );
    eprintln!("已只读导入 300 张照片，准备开始第一项真实任务");
    let endpoint = provider.base_url().to_owned();
    let model = provider.model_id().to_owned();
    let identity = provider.identity();
    let consent = agent::consent::set(
        &db,
        agent::consent::SetConsent {
            request_id: domain::id(),
            project_id: project.id.clone(),
            mode: "task".into(),
            provider: identity.consent_provider(),
            quota: 60,
        },
    )
    .await
    .unwrap();
    let quota_before = if let Some(auth) = &hosted {
        Some(auth.ai_account().await.unwrap())
    } else {
        None
    };
    let account = hosted.clone();
    let engine = if let Some(auth) = hosted {
        agent::runtime::AgentEngine::hosted(db.clone(), auth)
    } else {
        agent::runtime::AgentEngine::with_providers(
            db.clone(),
            Arc::new(agent::provider::MockProvider),
            Some(provider),
        )
    };
    let request = |goal: String, consent_id, continuation, budget| AgentRequest {
        request_id: domain::id(),
        project_id: project.id.clone(),
        command: TaskCommand::StartConversation {
            allow_queued_recovery: false,
            expected_account_id: identity.account_id.clone(),
            goal,
            selected_photo_ids: vec![],
            focus_photo_ids: vec![],
            target_count: 20,
            budget,
            consent_id,
            source_board: None,
            continuation,
        },
    };
    let first=engine.request(request("请在这 300 张照片中寻找三条不同的创作线索，每条选 20 张并生成独立画布。先分页检索，按需观察最多 60 张预览，优先复用有效缓存。每份方案必须有真实照片节点及带照片证据的编辑说明，说明实际观察范围和未观察部分，最后送审；不能只回复三段文字，也不能修改正式作品。".into(),Some(consent.id),None,Budget{max_cloud_images:60,max_cost_minor_usd:first_cost_cap,..Budget::default()})).await.unwrap();
    std::fs::write(directory.join("registration.json"),serde_json::to_vec_pretty(&json!({"endpoint":endpoint,"model":model,"tasksMax":2,"firstTask":first.id,"budget":first.budget,"effectiveFirstTaskCostCeilingMinorUsd":first_cost_cap,"effectiveSecondTaskCostCeilingMinorUsd":50,"previewQuota":60,"sourceCount":300,"humanQualityAccepted":false,"quotaBefore":quota_before,"reusedPreparedPreviews":reused.is_some()})).unwrap()).unwrap();
    eprintln!("第一项真实任务已创建：{}", first.id);
    let run = engine.run(&first.id).await;
    let view = agent::projection(&db, &project.id).await.unwrap();
    let quota_after_first = if let Some(auth) = &account {
        auth.ai_account().await.ok()
    } else {
        None
    };
    let first_hashes: Vec<_> = files
        .iter()
        .map(|f| image_pipeline::fingerprint(f).unwrap())
        .collect();
    let first_unchanged = serde_json::to_value(db.get(&project.id).await.unwrap()).unwrap()
        == serde_json::to_value(&original).unwrap();
    std::fs::write(
        directory.join("first-result.json"),
        serde_json::to_vec_pretty(&json!({"projection":view,"originalUnchanged":first_unchanged,"sourceHashesUnchanged":hashes==first_hashes,"quotaAfter":quota_after_first})).unwrap(),
    )
    .unwrap();
    eprintln!("真实自动化验收证据：{}", directory.display());
    assert!(first_unchanged);
    assert_eq!(hashes, first_hashes);
    run.unwrap();
    let first_view = view.tasks.iter().find(|t| t.id == first.id).unwrap();
    assert_eq!(first_view.state, "review_ready");
    assert_eq!(first_view.board_drafts.len(), 3);
    let inspected: std::collections::HashSet<_> = first_view
        .inspected_photos
        .iter()
        .filter_map(|v| v["photo_id"].as_str().map(String::from))
        .collect();
    assert!(!inspected.is_empty(), "真实视觉观察不能被文件名检索替代");
    for b in &first_view.board_drafts {
        let nodes = db.get_board_nodes(&b.id).await.unwrap();
        let photos: Vec<_> = nodes.iter().filter(|n| n.node_type == "photo").collect();
        assert_eq!(photos.len(), 20);
        let mut unique = std::collections::HashSet::new();
        for n in photos {
            let alias: String =
                sqlx::query_scalar("SELECT photo_id FROM asset_aliases WHERE asset_id=? LIMIT 1")
                    .bind(&n.asset_id)
                    .fetch_one(&db.pool)
                    .await
                    .unwrap();
            assert!(inspected.contains(&alias));
            assert!(unique.insert(alias));
        }
        assert!(
            nodes
                .iter()
                .any(|n| n.node_type == "sticky" || n.node_type == "text")
        );
    }
    let parent = &first_view.board_drafts[0];
    let parent_before =
        serde_json::to_value(db.get_board_nodes(&parent.id).await.unwrap()).unwrap();
    let parent_ref =
        serde_json::from_value(json!({"kind":"board","id":parent.id,"revision":parent.revision}))
            .unwrap();
    let second=engine.request(request("继续修改这个方案：保留开篇，重新安排中段的观看节奏，并解释相对上一版的具体变化。生成新的独立修订，保留上一版；这一轮不再上传图片，需要观察时说明已有证据的限制。".into(),None,Some(parent_ref),Budget{max_cost_minor_usd:50,..Budget::default()})).await.unwrap();
    let followup = engine.run(&second.id).await;
    let result = agent::projection(&db, &project.id).await.unwrap();
    let unchanged = serde_json::to_value(db.get(&project.id).await.unwrap()).unwrap()
        == serde_json::to_value(&original).unwrap();
    let after: Vec<_> = files
        .iter()
        .map(|f| image_pipeline::fingerprint(f).unwrap())
        .collect();
    let quota_after = if let Some(auth) = &account {
        auth.ai_account().await.ok()
    } else {
        None
    };
    let safe_summary = json!({"projectId":project.id,"firstTask":first.id,"secondTask":second.id,"originalUnchanged":unchanged,"sourceHashesUnchanged":hashes==after,"humanQualityAccepted":false,"projection":result,"quotaAfter":quota_after});
    let raw = serde_json::to_string_pretty(&safe_summary).unwrap();
    assert!(!raw.contains(&secret));
    std::fs::write(directory.join("result.json"), raw).unwrap();
    followup.unwrap();
    let final_view = result.tasks.iter().find(|t| t.id == second.id).unwrap();
    assert_eq!(final_view.state, "review_ready");
    assert!(!final_view.draft_lineage.is_empty());
    assert_eq!(final_view.usage.cloud_images, 0);
    assert_eq!(
        serde_json::to_value(db.get_board_nodes(&parent.id).await.unwrap()).unwrap(),
        parent_before
    );
    assert!(unchanged);
    assert_eq!(hashes, after);
}
