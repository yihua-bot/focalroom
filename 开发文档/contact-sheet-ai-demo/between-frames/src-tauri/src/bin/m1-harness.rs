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

#[path = "../harness_canvas_provider.rs"]
mod canvas_provider;
#[path = "../harness_fixtures.rs"]
mod fixtures;

struct Harness {
    cloud: Option<(String, String, Value)>,
    store: Store,
    agent: between_frames_lib::agent::runtime::AgentEngine,
    root: PathBuf,
    imports: ImportManager,
    grants: HashMap<String, (String, PathBuf)>,
}
fn string<'a>(value: &'a Value, key: &str) -> Result<&'a str> {
    value[key]
        .as_str()
        .ok_or_else(|| AppError::new("INVALID_INPUT", format!("缺少 {key}")))
}

fn payload<T: serde::de::DeserializeOwned>(args: &Value) -> Result<T> {
    serde_json::from_value(args["cmd"].clone())
        .map_err(|error| AppError::new("VALIDATION_ERROR", error.to_string()))
}

impl Harness {
    async fn execute(&mut self, command: &str, args: &Value) -> Result<Value> {
        let id = args["projectId"].as_str().unwrap_or("");
        let revision = args["expectedRevision"].as_i64().unwrap_or(0);
        let photo = args["photoId"].as_str().unwrap_or("");
        let value = match command {
            "canvas_comparison" => json!(
                self.store
                    .canvas_comparison(id, string(args, "boardId")?)
                    .await?
            ),
            "canvas_execute" => {
                json!(
                    self.store
                        .execute_canvas(serde_json::from_value(args["request"].clone()).map_err(
                            |error| AppError::new("VALIDATION_ERROR", error.to_string())
                        )?)
                        .await?
                )
            }
            "canvas_snapshot" => json!(
                self.store
                    .canvas_snapshot(id, string(args, "boardId")?)
                    .await?
            ),
            "canvas_list_boards" => json!(self.store.list_boards(id).await?),
            "canvas_create_board" => {
                json!(self.store.create_board(id, string(args, "title")?).await?)
            }
            "canvas_get_nodes" => {
                json!(self.store.get_board_nodes(string(args, "boardId")?).await?)
            }
            "canvas_get_photo_sources" => json!(
                self.store
                    .board_photo_sources(string(args, "boardId")?)
                    .await?
            ),
            "canvas_get_annotations" => {
                json!(self.store.get_annotations(string(args, "boardId")?).await?)
            }
            "canvas_place_photo" => json!(self.store.place_photo_node(id, &payload(args)?).await?),
            "canvas_move_nodes" => {
                let cmd = between_frames_lib::canvas::MoveNodesCommand {
                    board_id: string(args, "boardId")?.to_owned(),
                    node_ids: serde_json::from_value(args["nodeIds"].clone())
                        .map_err(|error| AppError::new("VALIDATION_ERROR", error.to_string()))?,
                    dx: args["dx"]
                        .as_f64()
                        .ok_or_else(|| AppError::new("VALIDATION_ERROR", "缺少 dx"))?,
                    dy: args["dy"]
                        .as_f64()
                        .ok_or_else(|| AppError::new("VALIDATION_ERROR", "缺少 dy"))?,
                };
                self.store.move_nodes(id, &cmd).await?;
                Value::Null
            }
            "canvas_add_text" => json!(self.store.add_text_node(id, &payload(args)?).await?),
            "canvas_delete_node" => {
                self.store
                    .delete_node(
                        id,
                        &between_frames_lib::canvas::DeleteNodeCommand {
                            board_id: string(args, "boardId")?.to_owned(),
                            node_id: string(args, "nodeId")?.to_owned(),
                        },
                    )
                    .await?;
                Value::Null
            }
            "canvas_add_annotation" => json!(self.store.add_annotation(id, &payload(args)?).await?),
            "canvas_resolve_annotation" => {
                self.store.resolve_annotation(id, &payload(args)?).await?;
                Value::Null
            }
            "canvas_add_arrow" => json!(self.store.add_arrow(id, &payload(args)?).await?),
            "canvas_add_ink" => json!(self.store.add_ink(id, &payload(args)?).await?),
            "canvas_undo" => json!(
                self.store
                    .undo_canvas(
                        id,
                        &between_frames_lib::canvas::UndoCanvasCommand {
                            board_id: string(args, "boardId")?.to_owned(),
                        }
                    )
                    .await?
            ),
            "canvas_redo" => json!(
                self.store
                    .redo_canvas(
                        id,
                        &between_frames_lib::canvas::RedoCanvasCommand {
                            board_id: string(args, "boardId")?.to_owned(),
                        }
                    )
                    .await?
            ),
            "canvas_adopt_board_draft" => {
                self.store
                    .adopt_board_draft(
                        id,
                        &between_frames_lib::canvas::AdoptBoardDraftCommand {
                            board_id: string(args, "boardId")?.to_owned(),
                        },
                    )
                    .await?;
                Value::Null
            }
            "canvas_group_nodes" => json!(self.store.group_nodes_cmd(id, &payload(args)?).await?),
            "canvas_resize_node" => {
                let cmd = serde_json::from_value(args.clone())
                    .map_err(|error| AppError::new("VALIDATION_ERROR", error.to_string()))?;
                self.store.resize_node(id, &cmd).await?;
                Value::Null
            }
            "canvas_set_node_locked" => {
                let cmd = serde_json::from_value(args.clone())
                    .map_err(|error| AppError::new("VALIDATION_ERROR", error.to_string()))?;
                self.store.set_node_locked(id, &cmd).await?;
                Value::Null
            }
            "pub_list" => json!(self.store.list_publications(id).await?),
            "pub_pages" => json!(
                self.store
                    .publication_pages(id, string(args, "publicationId")?)
                    .await?
            ),
            "pub_create" => json!(self.store.create_publication(id, &payload(args)?).await?),
            "pub_generate" => json!(
                self.store
                    .generate_from_sequence(id, &payload(args)?)
                    .await?
            ),
            "set_cloud_test_session" => {
                let origin =
                    between_frames_lib::login_guard::trusted_origin(string(args, "origin")?)?;
                if !origin.starts_with("http://") {
                    return Err(AppError::new("INVALID_INPUT", "测试会话仅允许回环HTTP"));
                }
                let token = string(args, "token")?.to_owned();
                if !between_frames_lib::login_guard::hex64(&token) {
                    return Err(AppError::new("INVALID_INPUT", "测试会话格式错误"));
                }
                let reply = reqwest::Client::new()
                    .get(format!("{origin}/api/v2/auth/me"))
                    .bearer_auth(&token)
                    .send()
                    .await
                    .map_err(|_| AppError::new("CLOUD_UNAVAILABLE", "夹具认证失败"))?;
                if !reply.status().is_success() {
                    return Err(AppError::new("AUTH_REQUIRED", "夹具认证失败"));
                }
                let user: Value = reply
                    .json()
                    .await
                    .map_err(|_| AppError::new("CLOUD_PROTOCOL", "夹具账户格式错误"))?;
                self.cloud = Some((
                    origin,
                    token,
                    json!({"id":user["id"],"email":user["email"],"name":user["name"]}),
                ));
                json!({"ok":true})
            }
            "cloud_login_status" => match &self.cloud {
                Some((origin, _, user)) => {
                    json!({"authenticated":true,"waiting":false,"origin":origin,"user":user,"error":null})
                }
                None => {
                    json!({"authenticated":false,"waiting":false,"origin":"https://team.focroom.cc","user":null,"error":null})
                }
            },
            "get_sharing_status" => {
                between_frames_lib::shared_cloud::status(&self.store, id).await?
            }
            "share_project" => {
                let (origin, token, _) = self
                    .cloud
                    .as_ref()
                    .ok_or_else(|| AppError::new("AUTH_REQUIRED", "请先登录"))?;
                if args["request"]["baseUrl"].as_str() != Some(origin) {
                    return Err(AppError::new("AUTH_ORIGIN_MISMATCH", "登录来源不匹配"));
                }
                between_frames_lib::shared_cloud::share(
                    &self.store,
                    serde_json::from_value(args["request"].clone())
                        .map_err(|_| AppError::new("INVALID_INPUT", "共享参数不合法"))?,
                    token,
                )
                .await?
            }
            "get_agent_projection" => {
                json!(between_frames_lib::agent::projection(&self.store, id).await?)
            }
            "store_api_key" => {
                let key = string(args, "key")?.to_owned();
                between_frames_lib::keychain::store_api_key(&key)?;
                json!({"ok": true, "keySource": "keychain"})
            }
            "delete_api_key" => {
                let _ = between_frames_lib::keychain::delete_api_key();
                json!({"ok": true, "keySource": null})
            }
            "set_project_tags" => {
                let tags: Vec<String> = serde_json::from_value(args["tags"].clone())
                    .map_err(|e| AppError::new("VALIDATION_ERROR", e.to_string()))?;
                json!({"ok": true, "tags": self.store.set_project_tags(id, &tags).await?})
            }
            "get_diagnostics" => json!(
                between_frames_lib::diagnostics::collect(
                    &self.store,
                    &self.root,
                    self.agent.ai_config()
                )
                .await?
            ),
            "execute_v2_command" => {
                let envelope = args["envelope"].clone();
                let principal = string(args, "principal")?.to_owned();
                let project_id = envelope["project_id"]
                    .as_str()
                    .unwrap_or_default()
                    .to_owned();
                let trusted = match principal.as_str() {
                    "local-owner" => between_frames_lib::v2_bridge::local_owner_principal(
                        "local-workspace",
                        &project_id,
                    ),
                    other => {
                        return Err(AppError::new(
                            "PERMISSION_DENIED",
                            format!("未知主体：{other}"),
                        ));
                    }
                };
                match betweenframes_local_domain::parse_command(envelope.clone()) {
                    Ok(validated) => {
                        let authority =
                            between_frames_lib::v2_bridge::authority_for(&self.store, &project_id)
                                .await?;
                        between_frames_lib::v2_bridge::execute(
                            &self.store,
                            between_frames_lib::v2_bridge::V2Execution {
                                envelope: validated,
                                principal: trusted,
                                authority,
                            },
                            4_102_444_800_000,
                        )
                        .await
                    }
                    Err(code) => {
                        json!({"ok": false, "command_id": "", "revision": null, "error": {"code": code, "message": "命令不符合 v2.0 契约"}})
                    }
                }
            }
            "get_proposals" => serde_json::to_value(
                between_frames_lib::agent::proposals::list(&self.store, id, 50).await?,
            )
            .unwrap(),
            "get_memory_projection" => serde_json::to_value(
                between_frames_lib::agent::memory::projection_for_query(
                    &self.store,
                    id,
                    args["query"].as_str().unwrap_or(""),
                )
                .await?,
            )
            .unwrap(),
            "execute_memory_command" => {
                let request: between_frames_lib::agent::memory_control::MemoryRequest =
                    serde_json::from_value(args["request"].clone())
                        .map_err(|error| AppError::new("VALIDATION_ERROR", error.to_string()))?;
                let request_id = request.request_id.clone();
                let result = between_frames_lib::command_handler::CommandHandler::new(&self.store)
                    .execute_memory(request)
                    .await;
                between_frames_lib::agent::memory_control::reply(&request_id, result)
            }
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
                // 隔离测试提供可运行的假模型；正式客户端不会编译此二进制。
                if config["providerId"] == "mock" {
                    config["ready"] = json!(true);
                }
                config["consent"] = serde_json::to_value(
                    between_frames_lib::agent::consent::effective_view(
                        &self.store,
                        id,
                        config["consentProvider"].as_str().unwrap_or("openai"),
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
                json!(self.agent.set_consent(request).await?)
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
                    Ok(mut receipt) => {
                        if args["compact"].as_bool().unwrap_or(false) {
                            receipt.project = receipt
                                .project
                                .map(between_frames_lib::photo_inspection::compact_project);
                        }
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
                    .into_iter()
                    .map(|project| gui_project(project, args))
                    .collect::<Vec<_>>()
            ),
            "get_project" => json!(gui_project(self.store.get(id).await?, args)),
            "get_original_photo" => json!(
                between_frames_lib::photo_inspection::original_path(&self.store, id, photo).await?
            ),
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
                let fixture_set = match args["fixtureSet"].as_str().unwrap_or("primary") {
                    "primary" => "fixtures",
                    "failures" => "fixture-failures",
                    "inspection" => "fixture-inspection",
                    "raw" => "fixture-raw",
                    _ => return Err(AppError::new("PATH_NOT_AUTHORIZED", "未知测试素材集合")),
                };
                self.grants
                    .insert(token.clone(), (id.into(), self.root.join(fixture_set)));
                json!(token)
            }
            "import_photos" => {
                let token = string(args, "selectionToken")?;
                let selection = self
                    .grants
                    .remove(token)
                    .filter(|(project, _)| project == id)
                    .ok_or_else(|| AppError::new("PATH_NOT_AUTHORIZED", "测试授权令牌无效"))?;
                json!(
                    self.imports
                        .start(
                            self.store.clone(),
                            self.root.join("cache"),
                            id.into(),
                            vec![selection.1],
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
                let filename = match export::destination(format)? {
                    export::ExportDestination::File(extension) => {
                        format!("{}-{}.{}", id, between_frames_lib::domain::id(), extension)
                    }
                    export::ExportDestination::Directory => {
                        format!("{}-{}-sidecar", id, between_frames_lib::domain::id())
                    }
                };
                let path = self.root.join("exports").join(filename);
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
                self.agent = between_frames_lib::agent::runtime::AgentEngine::with_provider(
                    self.store.clone(),
                    std::sync::Arc::new(canvas_provider::CanvasScenarioProvider),
                );
                json!(true)
            }
            "test_fixture_info" => fixtures::info(&self.root)?,
            "test_fixture_checksums" => {
                let mut checks = Vec::new();
                let mut directories = vec![self.root.join("fixtures")];
                if args["includeRelocated"].as_bool().unwrap_or(false) {
                    directories.push(self.root.join("relocated"));
                }
                for entry in directories
                    .iter()
                    .map(std::fs::read_dir)
                    .collect::<std::io::Result<Vec<_>>>()?
                    .into_iter()
                    .flatten()
                {
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
                if !source.starts_with(self.root.join("fixtures"))
                    && !source.starts_with(self.root.join("fixture-inspection"))
                {
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

fn gui_project(
    project: between_frames_lib::domain::Project,
    args: &Value,
) -> between_frames_lib::domain::Project {
    if args["compact"].as_bool().unwrap_or(false) {
        between_frames_lib::photo_inspection::compact_project(project)
    } else {
        project
    }
}

#[tokio::main]
async fn main() {
    let root = PathBuf::from(std::env::args().nth(1).expect("需要测试专用目录"));
    for name in ["fixtures", "cache", "exports", "relocated"] {
        std::fs::create_dir_all(root.join(name)).unwrap();
    }
    let profile = std::env::args().nth(2).unwrap_or_else(|| "standard".into());
    fixtures::prepare(&root, &profile).unwrap();
    let store = Store::open(&root.join("projects.sqlite")).await.unwrap();
    between_frames_lib::command_handler::CommandHandler::new(&store)
        .recover_agents()
        .await
        .unwrap();
    let mut harness = Harness {
        cloud: None,
        agent: between_frames_lib::agent::runtime::AgentEngine::with_provider(
            store.clone(),
            std::sync::Arc::new(canvas_provider::CanvasScenarioProvider),
        ),
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
