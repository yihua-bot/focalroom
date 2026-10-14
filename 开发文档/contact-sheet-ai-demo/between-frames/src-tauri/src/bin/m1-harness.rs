// 测试专用 stdio 驱动；不启动 WebView，不作为桌面产物编译，不开放生产网络端口。
use between_frames_lib::{
    db::Store,
    domain::{AppError, Result},
    export, image_pipeline,
    importer::ImportManager,
};
use serde_json::{Value, json};
use std::{collections::HashMap, path::PathBuf};
use tokio::io::{AsyncBufReadExt, AsyncWriteExt, BufReader};

struct Harness {
    store: Store,
    agent: between_frames_lib::agent::runtime::AgentEngine,
    root: PathBuf,
    imports: ImportManager,
    grants: HashMap<String, String>,
}
fn string<'a>(value: &'a Value, key: &str) -> Result<&'a str> {
    value[key]
        .as_str()
        .ok_or_else(|| AppError::new("INVALID_INPUT", format!("缺少 {key}")))
}

impl Harness {
    async fn execute(&mut self, command: &str, args: &Value) -> Result<Value> {
        let id = args["projectId"].as_str().unwrap_or("");
        let revision = args["expectedRevision"].as_i64().unwrap_or(0);
        let photo = args["photoId"].as_str().unwrap_or("");
        let value = match command {
            "get_agent_projection" => {
                json!(between_frames_lib::agent::projection(&self.store, id).await?)
            }
            "get_memory_projection" => serde_json::to_value(
                between_frames_lib::agent::memory::projection(&self.store, id).await?,
            )
            .unwrap(),
            "correct_hypothesis" => serde_json::to_value(
                between_frames_lib::agent::memory::correct(
                    &self.store,
                    between_frames_lib::agent::memory::HypothesisCorrection {
                        hypothesis_id: args["hypothesisId"].as_i64().unwrap_or(0),
                        project_id: id.to_owned(),
                        correction: string(args, "correction")?.to_owned(),
                    },
                )
                .await?,
            )
            .unwrap(),
            "set_hypothesis_enabled" => serde_json::to_value(
                between_frames_lib::agent::memory::set_enabled(
                    &self.store,
                    id,
                    args["hypothesisId"].as_i64().unwrap_or(0),
                    args["enabled"].as_bool().unwrap_or(false),
                )
                .await?,
            )
            .unwrap(),
            "get_ai_config" => {
                let mut config = serde_json::to_value(self.agent.ai_config()).unwrap();
                config["consent"] = serde_json::to_value(
                    between_frames_lib::agent::consent::effective_view(
                        &self.store,
                        id,
                        between_frames_lib::agent::PROVIDER_OPENAI,
                    )
                    .await?,
                )
                .unwrap_or(serde_json::Value::Null);
                config
            }
            "set_privacy_consent" => {
                let request: between_frames_lib::agent::types::ConsentRequest =
                    serde_json::from_value(args["request"].clone())
                        .map_err(|e| AppError::new("VALIDATION_ERROR", e.to_string()))?;
                json!(
                    between_frames_lib::agent::consent::set(
                        &self.store,
                        between_frames_lib::agent::consent::SetConsent {
                            request_id: request.request_id,
                            project_id: request.project_id,
                            mode: request.mode,
                            provider: request.provider,
                            quota: request.quota,
                        },
                    )
                    .await?
                )
            }
            "agent_request" => {
                let request: between_frames_lib::agent::types::AgentRequest =
                    serde_json::from_value(args["request"].clone())
                        .map_err(|e| AppError::new("VALIDATION_ERROR", e.to_string()))?;
                let request_id = request.request_id.clone();
                match between_frames_lib::command_handler::CommandHandler::new(&self.store)
                    .agent_request(request)
                    .await
                {
                    Ok(task) => {
                        if task.state == "queued" {
                            self.agent.dispatch(&task.id).await;
                        }
                        json!({"ok":true,"data":{"taskId":task.id},"error":null,"revision":null,"requestId":request_id})
                    }
                    Err(e) => {
                        json!({"ok":false,"data":null,"error":{"code":e.code,"message":e.message,"retryable":e.retriable},"revision":e.current_revision,"requestId":request_id})
                    }
                }
            }
            "execute_gui_command" => {
                let request: between_frames_lib::command_handler::GuiCommandRequest =
                    serde_json::from_value(args["request"].clone())
                        .map_err(|error| AppError::new("VALIDATION_ERROR", error.to_string()))?;
                let request_id = request.command_id.clone();
                match between_frames_lib::command_handler::CommandHandler::new(&self.store)
                    .execute_gui(request)
                    .await
                {
                    Ok(receipt) => {
                        json!({"ok":true,"revision":receipt.revision,"requestId":request_id,"data":receipt,"error":null})
                    }
                    Err(error) => {
                        json!({"ok":false,"revision":error.current_revision,"requestId":request_id,"data":null,"error":{"code":error.code,"message":error.message,"retryable":error.retriable,"currentRevision":error.current_revision}})
                    }
                }
            }
            "list_projects" => json!(
                self.store
                    .list(args["archived"].as_bool().unwrap_or(false))
                    .await?
            ),
            "get_project" => json!(self.store.get(id).await?),
            "create_project" => json!(
                self.store
                    .create(string(args, "title")?, string(args, "brief")?)
                    .await?
            ),
            "update_project" => json!(
                self.store
                    .edit(id, string(args, "title")?, string(args, "brief")?, revision)
                    .await?
            ),
            "update_photo_decision" => json!(
                self.store
                    .decide(id, photo, string(args, "status")?, revision)
                    .await?
            ),
            "update_photo_note" => json!(
                self.store
                    .note(id, photo, string(args, "text")?, revision)
                    .await?
            ),
            "set_photo_decisions" | "set_photo_rating" => {
                let ids: Vec<String> = serde_json::from_value(args["photoIds"].clone())
                    .map_err(|_| AppError::new("INVALID_INPUT", "选区输入非法"))?;
                if command == "set_photo_decisions" {
                    json!(
                        self.store
                            .decide_many(id, &ids, string(args, "status")?, revision)
                            .await?
                    )
                } else {
                    let rating = args["rating"]
                        .as_i64()
                        .ok_or_else(|| AppError::new("INVALID_INPUT", "评分输入非法"))?;
                    json!(self.store.rate_many(id, &ids, rating, revision).await?)
                }
            }
            "move_sequence_items" => {
                let ids: Vec<String> = serde_json::from_value(args["photoIds"].clone())
                    .map_err(|_| AppError::new("INVALID_INPUT", "序列输入非法"))?;
                json!(self.store.reorder(id, &ids, revision).await?)
            }
            "create_comparison_event" => json!(
                self.store
                    .compare(
                        id,
                        string(args, "leftId")?,
                        string(args, "rightId")?,
                        args["preferred"].as_str(),
                        string(args, "reason")?,
                        revision
                    )
                    .await?
            ),
            "delete_project" => {
                self.store.archive(id, revision).await?;
                Value::Null
            }
            "restore_project" => json!(self.store.restore(id).await?),
            "choose_import" => {
                self.store.get(id).await?;
                let token = between_frames_lib::domain::id();
                self.grants.insert(token.clone(), id.into());
                json!(token)
            }
            "import_photos" => {
                let token = string(args, "selectionToken")?;
                if self.grants.remove(token).as_deref() != Some(id) {
                    return Err(AppError::new("PATH_NOT_AUTHORIZED", "测试授权令牌无效"));
                }
                json!(
                    self.imports
                        .start(
                            self.store.clone(),
                            self.root.join("cache"),
                            id.into(),
                            vec![self.root.join("fixtures")],
                            |_| {}
                        )
                        .await?
                )
            }
            "get_import_job" => json!(self.imports.get(string(args, "jobId")?).await?),
            "cancel_import" => {
                self.imports.cancel(string(args, "jobId")?).await?;
                Value::Null
            }
            "retry_import" => {
                let job = self.imports.get(string(args, "jobId")?).await?;
                let paths = job
                    .failures
                    .iter()
                    .map(|failure| PathBuf::from(&failure.path))
                    .collect();
                json!(
                    self.imports
                        .start(
                            self.store.clone(),
                            self.root.join("cache"),
                            job.project_id,
                            paths,
                            |_| {}
                        )
                        .await?
                )
            }
            "relink_photo" => {
                let project = self.store.get(id).await?;
                let original = project
                    .photos
                    .iter()
                    .find(|item| item.id == photo)
                    .ok_or_else(|| AppError::new("NOT_FOUND", "照片不存在"))?;
                let filename = PathBuf::from(&original.original_path)
                    .file_name()
                    .unwrap()
                    .to_owned();
                let path = self.root.join("relocated").join(filename);
                let cache = self.root.join("cache");
                let prepared =
                    tokio::task::spawn_blocking(move || image_pipeline::prepare(&path, &cache))
                        .await
                        .unwrap()?;
                json!(self.store.relink(id, photo, &prepared, revision).await?)
            }
            "export_project" => {
                let project = self.store.get(id).await?;
                let format = string(args, "format")?;
                let path = self.root.join("exports").join(format!(
                    "{}-{}.{}",
                    id,
                    between_frames_lib::domain::id(),
                    format
                ));
                json!(export::write_with_options(
                    &project,
                    format,
                    &path,
                    args["includePaths"].as_bool().unwrap_or(false)
                )?)
            }
            "test_restart" => {
                self.store.pool.close().await;
                self.store = Store::open(&self.root.join("projects.sqlite")).await?;
                between_frames_lib::command_handler::CommandHandler::new(&self.store)
                    .recover_agents()
                    .await?;
                self.agent =
                    between_frames_lib::agent::runtime::AgentEngine::new(self.store.clone());
                json!(true)
            }
            "test_fixture_checksums" => {
                let mut checks = Vec::new();
                for entry in std::fs::read_dir(self.root.join("fixtures"))? {
                    let path = entry?.path();
                    if path.is_file() {
                        checks.push((
                            path.file_name().unwrap().to_string_lossy().to_string(),
                            image_pipeline::fingerprint(&path)?,
                        ));
                    }
                }
                checks.sort();
                json!(checks)
            }
            "test_move_source" => {
                let project = self.store.get(id).await?;
                let source = PathBuf::from(&project.photos[0].original_path);
                let target = self
                    .root
                    .join("relocated")
                    .join(source.file_name().unwrap());
                if !source.starts_with(self.root.join("fixtures")) {
                    return Err(AppError::new("INVALID_PATH", "只能移动测试生成的照片"));
                }
                std::fs::rename(source, target)?;
                json!(true)
            }
            _ => return Err(AppError::new("NOT_FOUND", "测试命令不存在")),
        };
        Ok(value)
    }
}

#[tokio::main]
async fn main() {
    let root = PathBuf::from(std::env::args().nth(1).expect("需要测试专用目录"));
    for name in ["fixtures", "cache", "exports", "relocated"] {
        std::fs::create_dir_all(root.join(name)).unwrap();
    }
    if std::fs::read_dir(root.join("fixtures"))
        .unwrap()
        .next()
        .is_none()
    {
        for index in 0..200 {
            let sample = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
                .join(format!("../../assets/frame_{:02}.jpg", index % 28 + 1));
            let mut image = image::open(sample).unwrap().thumbnail(480, 480).to_rgb8();
            image.put_pixel(
                0,
                0,
                image::Rgb([
                    (index % 256) as u8,
                    ((index * 7) % 256) as u8,
                    ((index * 13) % 256) as u8,
                ]),
            );
            let path = root.join("fixtures").join(format!("frame-{index:03}.jpg"));
            image.save(&path).unwrap();
            // JPEG 编码会抹掉单像素变化；测试文件使用唯一 COM 段保证内容指纹不同。
            let bytes = std::fs::read(&path).unwrap();
            let comment = format!("Between Frames automated fixture {index:03}");
            let mut unique = bytes[..2].to_vec();
            unique.extend([0xff, 0xfe]);
            unique.extend(((comment.len() + 2) as u16).to_be_bytes());
            unique.extend(comment.as_bytes());
            unique.extend(&bytes[2..]);
            std::fs::write(path, unique).unwrap();
        }
    }
    let store = Store::open(&root.join("projects.sqlite")).await.unwrap();
    between_frames_lib::command_handler::CommandHandler::new(&store)
        .recover_agents()
        .await
        .unwrap();
    let mut harness = Harness {
        agent: between_frames_lib::agent::runtime::AgentEngine::new(store.clone()),
        store,
        root,
        imports: Default::default(),
        grants: HashMap::new(),
    };
    let mut input = BufReader::new(tokio::io::stdin()).lines();
    let mut output = tokio::io::stdout();
    while let Ok(Some(line)) = input.next_line().await {
        let request: Value = serde_json::from_str(&line).unwrap();
        let result = harness
            .execute(request["command"].as_str().unwrap_or(""), &request["args"])
            .await;
        let response = match result {
            Ok(value) => json!({"id":request["id"],"ok":true,"value":value}),
            Err(error) => json!({"id":request["id"],"ok":false,"error":error}),
        };
        output
            .write_all(format!("{response}\n").as_bytes())
            .await
            .unwrap();
        output.flush().await.unwrap();
    }
}
