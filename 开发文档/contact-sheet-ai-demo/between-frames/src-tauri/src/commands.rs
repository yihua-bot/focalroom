use crate::{
    db::Store,
    domain::{AppError, ImportJob, Project, Result},
    export::{self, ExportReceipt},
    image_pipeline,
    importer::ImportManager,
};
use std::{collections::HashMap, path::PathBuf, sync::Arc};
use tauri::{AppHandle, Emitter, Manager, State};
use tauri_plugin_dialog::DialogExt;
use tokio::sync::Mutex;

type ImportGrants = Arc<Mutex<HashMap<String, (String, Vec<PathBuf>)>>>;

pub struct Backend {
    pub cloud_auth: Arc<crate::cloud_auth::CloudAuth>,
    pub agent: crate::agent::runtime::AgentEngine,
    pub store: Store,
    pub cache: PathBuf,
    pub imports: ImportManager,
    pub grants: ImportGrants,
}

#[tauri::command]
pub async fn list_projects(
    state: State<'_, Backend>,
    archived: Option<bool>,
    compact: Option<bool>,
) -> Result<Vec<Project>> {
    let projects = state.store.list(archived.unwrap_or(false)).await?;
    Ok(projects
        .into_iter()
        .map(|project| {
            if compact.unwrap_or(false) {
                crate::photo_inspection::compact_project(project)
            } else {
                project
            }
        })
        .collect())
}

#[tauri::command]
pub async fn delete_project(state: State<'_, Backend>, project_id: String) -> Result<()> {
    state.store.delete(&project_id).await
}
#[tauri::command]
pub async fn get_project(
    state: State<'_, Backend>,
    project_id: String,
    compact: Option<bool>,
) -> Result<Project> {
    let project = state.store.get(&project_id).await?;
    Ok(if compact.unwrap_or(false) {
        crate::photo_inspection::compact_project(project)
    } else {
        project
    })
}
#[tauri::command]
pub async fn get_original_photo(
    app: AppHandle,
    state: State<'_, Backend>,
    project_id: String,
    photo_id: String,
) -> Result<crate::photo_inspection::InspectionSource> {
    let source =
        crate::photo_inspection::original_path(&state.store, &project_id, &photo_id).await?;
    if let Some(path) = source.path() {
        app.asset_protocol_scope()
            .allow_file(path)
            .map_err(|error| AppError::new("ASSET_SCOPE", error.to_string()))?;
    }
    Ok(source)
}

#[tauri::command]
pub async fn canvas_photo_tile(
    app: AppHandle,
    state: State<'_, Backend>,
    project_id: String,
    asset_id: String,
    level: u8,
    x: u32,
    y: u32,
) -> Result<crate::canvas::CanvasTile> {
    // G2-②：大图画布节点按 (源SHA, 版本, 层级, x, y) 内容寻址瓦片；只暴露缓存 JPEG，不开放原图任意读取。
    let tile = state
        .store
        .canvas_photo_tile(&project_id, &asset_id, level, x, y)
        .await?;
    app.asset_protocol_scope()
        .allow_file(&tile.path)
        .map_err(|error| AppError::new("ASSET_SCOPE", error.to_string()))?;
    Ok(tile)
}
#[derive(serde::Serialize)]
#[serde(rename_all = "camelCase")]
pub struct GuiReply {
    ok: bool,
    data: Option<crate::command_handler::CommandReceipt>,
    error: Option<serde_json::Value>,
    revision: Option<i64>,
    request_id: String,
}
#[tauri::command]
pub async fn execute_gui_command(
    app: AppHandle,
    state: State<'_, Backend>,
    request: crate::command_handler::GuiCommandRequest,
    compact: Option<bool>,
) -> Result<GuiReply> {
    let draft_change = matches!(
        request.command,
        crate::commands_generated::GuiCommand::ApplyDraft { .. }
            | crate::commands_generated::GuiCommand::RebaseDraft { .. }
            | crate::commands_generated::GuiCommand::UpdateDraft { .. }
            | crate::commands_generated::GuiCommand::DiscardDraft { .. }
    );
    let request_id = request.command_id.clone();
    let project_id = request.project_id.clone();
    Ok(
        match crate::command_handler::CommandHandler::new(&state.store)
            .execute_gui(request)
            .await
        {
            Ok(mut receipt) => {
                if receipt.changed {
                    let _ = app.emit(crate::commands_generated::PROJECT_CHANGED_EVENT, serde_json::json!({"projectId":project_id,"revision":receipt.revision,"requestId":request_id}));
                    let _ = app.emit(
                        crate::commands_generated::AGENT_TASK_UPDATED_EVENT,
                        serde_json::json!({"projectId":project_id,"requestId":request_id}),
                    );
                    if draft_change {
                        let _ = app.emit(
                            crate::commands_generated::DRAFT_UPDATED_EVENT,
                            serde_json::json!({"projectId":project_id,"requestId":request_id}),
                        );
                    }
                }
                if compact.unwrap_or(false) {
                    receipt.project = receipt
                        .project
                        .map(crate::photo_inspection::compact_project);
                }
                GuiReply {
                    ok: true,
                    revision: Some(receipt.revision),
                    data: Some(receipt),
                    error: None,
                    request_id,
                }
            }
            Err(error) => GuiReply {
                ok: false,
                data: None,
                revision: error.current_revision,
                error: Some(
                    serde_json::json!({"code":error.code,"message":error.message,"retryable":error.retriable,"currentRevision":error.current_revision}),
                ),
                request_id,
            },
        },
    )
}

#[tauri::command]
pub async fn choose_import(
    app: AppHandle,
    state: State<'_, Backend>,
    project_id: String,
    folder: bool,
) -> Result<Option<String>> {
    state.store.get(&project_id).await?;
    let paths = tokio::task::spawn_blocking(move || {
        let dialog = app.dialog().file().set_title("选择照片，原文件只读引用");
        let paths = if folder {
            dialog.blocking_pick_folder().map(|path| vec![path])
        } else {
            dialog
                .add_filter("照片", image_pipeline::EXTENSIONS)
                .blocking_pick_files()
        };
        paths
            .map(|items| {
                items
                    .into_iter()
                    .map(|item| {
                        item.into_path()
                            .map_err(|error| AppError::new("INVALID_PATH", error.to_string()))
                    })
                    .collect::<Result<Vec<_>>>()
            })
            .transpose()
    })
    .await
    .map_err(|error| AppError::new("DIALOG", error.to_string()))??;
    if let Some(paths) = paths {
        let token = crate::domain::id();
        state
            .grants
            .lock()
            .await
            .insert(token.clone(), (project_id, paths));
        Ok(Some(token))
    } else {
        Ok(None)
    }
}

#[tauri::command]
pub async fn import_photos(
    app: AppHandle,
    state: State<'_, Backend>,
    project_id: String,
    selection_token: String,
) -> Result<String> {
    let (owner, paths) = state
        .grants
        .lock()
        .await
        .remove(&selection_token)
        .ok_or_else(|| {
            AppError::new("PATH_NOT_AUTHORIZED", "请先通过系统文件选择器授权照片路径")
        })?;
    if owner != project_id {
        return Err(AppError::new("PATH_NOT_AUTHORIZED", "文件授权不属于此项目"));
    }
    state
        .imports
        .start(
            state.store.clone(),
            state.cache.clone(),
            project_id,
            paths,
            move |job| {
                let _ = app.emit("import_progress", job);
                if job.status != "running" {
                    let _ = app.emit(
                        crate::commands_generated::PHOTOS_IMPORTED_EVENT,
                        serde_json::json!({"projectId":job.project_id,"jobId":job.job_id}),
                    );
                }
            },
        )
        .await
}
#[tauri::command]
pub async fn get_import_job(state: State<'_, Backend>, job_id: String) -> Result<ImportJob> {
    state.imports.get(&job_id).await
}
#[tauri::command]
pub async fn cancel_import(state: State<'_, Backend>, job_id: String) -> Result<()> {
    state.imports.cancel(&job_id).await
}

#[tauri::command]
pub async fn retry_import(
    app: AppHandle,
    state: State<'_, Backend>,
    job_id: String,
) -> Result<String> {
    let job = state.imports.get(&job_id).await?;
    if job.status == "running" || job.failures.is_empty() {
        return Err(AppError::new("INVALID_INPUT", "没有可重试的失败照片"));
    }
    let paths = job
        .failures
        .iter()
        .map(|failure| PathBuf::from(&failure.path))
        .collect();
    state
        .imports
        .start(
            state.store.clone(),
            state.cache.clone(),
            job.project_id,
            paths,
            move |progress| {
                let _ = app.emit("import_progress", progress);
                if progress.status != "running" { let _ = app.emit(crate::commands_generated::PHOTOS_IMPORTED_EVENT, serde_json::json!({"projectId":progress.project_id,"jobId":progress.job_id})); }
            },
        )
        .await
}

#[tauri::command]
pub async fn relink_photo(
    app: AppHandle,
    state: State<'_, Backend>,
    project_id: String,
    photo_id: String,
    expected_revision: i64,
) -> Result<Option<Project>> {
    let project = state.store.get(&project_id).await?;
    if !project.photos.iter().any(|photo| photo.id == photo_id) {
        return Err(AppError::new("NOT_FOUND", "照片不在该项目中"));
    }
    let cache = state.cache.clone();
    let prepared = tokio::task::spawn_blocking(move || {
        let Some(path) = app
            .dialog()
            .file()
            .set_title("重新定位原图（必须为同一文件）")
            .add_filter("照片", image_pipeline::EXTENSIONS)
            .blocking_pick_file()
        else {
            return Ok(None);
        };
        let path = path
            .into_path()
            .map_err(|error| AppError::new("INVALID_PATH", error.to_string()))?;
        image_pipeline::prepare(&path, &cache).map(Some)
    })
    .await
    .map_err(|error| AppError::new("BACKGROUND", error.to_string()))??;
    if let Some(prepared) = prepared {
        state
            .store
            .relink(&project_id, &photo_id, &prepared, expected_revision)
            .await
            .map(Some)
    } else {
        Ok(None)
    }
}

#[tauri::command]
pub async fn export_project(
    app: AppHandle,
    state: State<'_, Backend>,
    project_id: String,
    format: String,
    include_paths: Option<bool>,
) -> Result<Option<ExportReceipt>> {
    let destination = export::destination(&format)?;
    let snapshot = state.store.get(&project_id).await?;
    tokio::task::spawn_blocking(move || {
        let path = match destination {
            export::ExportDestination::File(extension) => {
                let filename = format!("between-frames-{}.{}", snapshot.id, extension);
                let Some(path) = app
                    .dialog()
                    .file()
                    .set_title("导出编辑快照（选择新的文件名）")
                    .set_file_name(&filename)
                    .add_filter("导出文件", &[extension])
                    .blocking_save_file()
                else {
                    return Ok(None);
                };
                path.into_path()
                    .map_err(|error| AppError::new("INVALID_PATH", error.to_string()))?
            }
            export::ExportDestination::Directory => {
                let Some(parent) = app
                    .dialog()
                    .file()
                    .set_title("选择导出的父文件夹（将在其中新建文件夹）")
                    .blocking_pick_folder()
                else {
                    return Ok(None);
                };
                parent
                    .into_path()
                    .map_err(|error| AppError::new("INVALID_PATH", error.to_string()))?
                    .join(format!(
                        "focalroom-{}-{}-{}",
                        format,
                        snapshot.id,
                        crate::domain::id()
                    ))
            }
        };
        export::write_with_options(&snapshot, &format, &path, include_paths.unwrap_or(false))
            .map(Some)
    })
    .await
    .map_err(|error| AppError::new("BACKGROUND", error.to_string()))?
}

#[tauri::command]
pub async fn get_sharing_status(
    state: State<'_, Backend>,
    project_id: String,
) -> Result<serde_json::Value> {
    crate::shared_cloud::status(&state.store, &project_id).await
}
#[tauri::command]
pub async fn open_shared_review(state: State<'_, Backend>, project_id: String) -> Result<()> {
    let url = crate::shared_cloud::review_url(&state.store, &project_id).await?;
    #[cfg(target_os = "macos")]
    {
        tokio::task::spawn_blocking(move || {
            let status = std::process::Command::new("/usr/bin/open")
                .arg("--")
                .arg(url)
                .stdin(std::process::Stdio::null())
                .stdout(std::process::Stdio::null())
                .stderr(std::process::Stdio::null())
                .status()
                .map_err(|_| AppError::new("BROWSER_OPEN_FAILED", "无法打开系统浏览器"))?;
            if status.success() {
                Ok(())
            } else {
                Err(AppError::new("BROWSER_OPEN_FAILED", "无法打开系统浏览器"))
            }
        })
        .await
        .map_err(|_| AppError::new("BROWSER_OPEN_FAILED", "无法打开系统浏览器"))?
    }
    #[cfg(not(target_os = "macos"))]
    {
        let _ = url;
        Err(AppError::new(
            "UNSUPPORTED_PLATFORM",
            "请复制评片地址到系统浏览器",
        ))
    }
}
#[tauri::command]
pub async fn share_project(
    app: AppHandle,
    state: State<'_, Backend>,
    request: crate::shared_cloud::ShareRequest,
) -> Result<serde_json::Value> {
    let id = request.project_id.clone();
    let result = {
        let token = state.cloud_auth.bearer(&request.base_url).await?;
        crate::shared_cloud::share(&state.store, request, &token).await
    };
    // 成功/失败均刷新：超时也可能已经冻结了项目，前端不能继续显示可写状态。
    let _ = app.emit(
        crate::commands_generated::PROJECT_CHANGED_EVENT,
        serde_json::json!({"projectId":id}),
    );
    result
}

#[tauri::command]
pub async fn cloud_author_memories(
    state: State<'_, Backend>,
    project_id: String,
) -> Result<serde_json::Value> {
    let (base, _) = crate::shared_cloud::author_memory_binding(&state.store, &project_id).await?;
    let token = state.cloud_auth.bearer(&base).await?;
    crate::shared_cloud::author_memories(&state.store, &project_id, &token).await
}

#[tauri::command]
pub async fn get_agent_projection(
    state: State<'_, Backend>,
    project_id: String,
) -> Result<crate::agent::types::AgentProjection> {
    let account = match state.cloud_auth.account_id() {
        Ok(id) => id,
        Err(e) if e.code == "AUTH_REQUIRED" => {
            return Ok(crate::agent::types::AgentProjection {
                tasks: vec![],
                drafts: vec![],
            });
        }
        Err(e) => return Err(e),
    };
    let projection =
        crate::agent::store::projection_for_account(&state.store, &project_id, &account).await?;
    if state.cloud_auth.account_id()? != account {
        return Err(AppError::new(
            "AUTH_REQUIRED",
            "账户已改变，请重新打开 Focal AI",
        ));
    }
    Ok(projection)
}
#[tauri::command]
pub async fn get_ai_config(
    state: State<'_, Backend>,
    project_id: String,
) -> Result<serde_json::Value> {
    let mut config = serde_json::to_value(state.agent.ai_config())
        .map_err(|e| AppError::new("VALIDATION_ERROR", e.to_string()))?;
    if config["keySource"] == "focalroom" {
        match state.cloud_auth.ai_account().await {
            Ok(account) => {
                config["hostedAccount"] = account;
            }
            Err(error) => {
                config["hostedError"] = error.message.into();
            }
        }
    }
    config["consent"] = serde_json::to_value(
        crate::agent::consent::effective_view(
            &state.store,
            &project_id,
            config["consentProvider"].as_str().unwrap_or("openai"),
        )
        .await?,
    )
    .unwrap_or(serde_json::Value::Null);
    Ok(config)
}
#[tauri::command]
pub async fn set_privacy_consent(
    state: State<'_, Backend>,
    request: crate::agent::types::ConsentRequest,
) -> Result<crate::agent::types::ConsentView> {
    state.agent.set_consent(request).await
}
#[tauri::command]
pub fn store_api_key(key: String) -> Result<serde_json::Value> {
    crate::keychain::store_api_key(key.trim())?;
    Ok(serde_json::json!({"ok": true, "keySource": "keychain"}))
}
#[tauri::command]
pub fn delete_api_key() -> Result<serde_json::Value> {
    // 条目不存在视为已删除，保持幂等。
    let _ = crate::keychain::delete_api_key();
    Ok(serde_json::json!({"ok": true, "keySource": null}))
}
#[tauri::command]
pub async fn set_project_tags(
    state: State<'_, Backend>,
    project_id: String,
    tags: Vec<String>,
) -> Result<serde_json::Value> {
    let cleaned = state.store.set_project_tags(&project_id, &tags).await?;
    Ok(serde_json::json!({"ok": true, "tags": cleaned}))
}
#[tauri::command]
pub async fn get_diagnostics(state: State<'_, Backend>) -> Result<crate::diagnostics::Diagnostics> {
    let root = state
        .cache
        .parent()
        .ok_or_else(|| AppError::new("DIAGNOSTICS_UNAVAILABLE", "存储统计暂不可用"))?;
    crate::diagnostics::collect(&state.store, root, state.agent.ai_config()).await
}
#[tauri::command]
pub async fn execute_v2_command(
    state: State<'_, Backend>,
    envelope: serde_json::Value,
    principal: String,
) -> Result<serde_json::Value> {
    // principal 是受信入口标识（本地单机：local-owner）；角色/权限由 local-domain 判定，
    // 客户端不能自报 OWNER/EDITOR 等角色。
    let project_id = envelope["project_id"]
        .as_str()
        .unwrap_or_default()
        .to_owned();
    let trusted = match principal.as_str() {
        "local-owner" => crate::v2_bridge::local_owner_principal("local-workspace", &project_id),
        other => {
            return Err(crate::domain::AppError::new(
                "PERMISSION_DENIED",
                format!("未知的受信主体标识：{other}"),
            ));
        }
    };
    let validated = match betweenframes_local_domain::parse_command(envelope) {
        Ok(validated) => validated,
        Err(code) => {
            return Ok(
                serde_json::json!({"ok": false, "command_id": "", "revision": null, "error": {"code": code, "message": "命令不符合 v2.0 契约"}}),
            );
        }
    };
    let authority = crate::v2_bridge::authority_for(&state.store, &project_id).await?;
    Ok(crate::v2_bridge::execute(
        &state.store,
        crate::v2_bridge::V2Execution {
            envelope: validated,
            principal: trusted,
            authority,
        },
        now_ms(),
    )
    .await)
}
fn now_ms() -> u64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|duration| duration.as_millis() as u64)
        .unwrap_or(0)
}
#[tauri::command]
pub async fn get_proposals(
    state: State<'_, Backend>,
    project_id: String,
) -> Result<serde_json::Value> {
    crate::agent::proposals::list(&state.store, &project_id, 50).await
}
#[tauri::command]
pub async fn get_memory_projection(
    state: State<'_, Backend>,
    project_id: String,
    query: Option<String>,
) -> Result<crate::agent::memory::MemoryProjection> {
    crate::agent::memory::projection_for_query(
        &state.store,
        &project_id,
        query.as_deref().unwrap_or(""),
    )
    .await
}
#[tauri::command]
pub async fn execute_memory_command(
    app: AppHandle,
    state: State<'_, Backend>,
    request: crate::agent::memory_control::MemoryRequest,
) -> Result<serde_json::Value> {
    if let crate::agent::memory_control::MemoryCommand::RememberPreference {
        expected_account_id,
        ..
    }
    | crate::agent::memory_control::MemoryCommand::DismissPreferenceProposal {
        expected_account_id,
        ..
    } = &request.command
        && expected_account_id.as_ref() != Some(&state.cloud_auth.account_id()?)
    {
        return Err(AppError::new(
            "AUTH_REQUIRED",
            "账户已改变，请重新打开这条建议",
        ));
    }
    let request_id = request.request_id.clone();
    let project_id = request.project_id.clone();
    let result = crate::command_handler::CommandHandler::new(&state.store)
        .execute_memory(request)
        .await;
    if result.as_ref().is_ok_and(|receipt| receipt.changed) {
        let _ = app.emit(
            crate::commands_generated::AGENT_TASK_UPDATED_EVENT,
            serde_json::json!({"projectId":project_id}),
        );
    }
    Ok(crate::agent::memory_control::reply(&request_id, result))
}
#[tauri::command]
pub async fn correct_hypothesis(
    state: State<'_, Backend>,
    project_id: String,
    hypothesis_id: i64,
    correction: String,
) -> Result<crate::agent::memory::HypothesisView> {
    crate::agent::memory::correct(
        &state.store,
        crate::agent::memory::HypothesisCorrection {
            hypothesis_id,
            project_id,
            correction,
        },
    )
    .await
}
#[tauri::command]
pub async fn set_hypothesis_enabled(
    state: State<'_, Backend>,
    project_id: String,
    hypothesis_id: i64,
    enabled: bool,
) -> Result<crate::agent::memory::HypothesisView> {
    crate::agent::memory::set_enabled(&state.store, &project_id, hypothesis_id, enabled).await
}
#[tauri::command]
pub async fn query_vision_result(
    state: State<'_, Backend>,
    project_id: String,
    task_id: String,
) -> Result<serde_json::Value> {
    serde_json::to_value(
        state
            .agent
            .query_vision_result(&project_id, &task_id)
            .await?,
    )
    .map_err(crate::command_audit::serialization)
}
#[tauri::command]
pub async fn agent_request(
    app: AppHandle,
    state: State<'_, Backend>,
    request: crate::agent::types::AgentRequest,
) -> Result<serde_json::Value> {
    let request_id = request.request_id.clone();
    let project_id = request.project_id.clone();
    let result = state.agent.request(request).await;
    Ok(match result {
        Ok(task) => {
            let id = task.id.clone();
            let _ = app.emit(
                crate::commands_generated::AGENT_TASK_UPDATED_EVENT,
                serde_json::json!({"projectId":project_id,"requestId":request_id}),
            );
            if task.state == "queued" {
                state.agent.dispatch(&id).await;
            }
            serde_json::json!({"ok":true,"data":{"taskId":id},"error":null,"revision":null,"requestId":request_id})
        }
        Err(e) => {
            serde_json::json!({"ok":false,"data":null,"error":{"code":e.code,"message":e.message,"retryable":e.retriable},"revision":e.current_revision,"requestId":request_id})
        }
    })
}

#[tauri::command]
/// 导入摄影师盲评质量记录（评测协议产出）；五维分/样本量/理由由后端硬校验。
/// 评测工具与人工导入使用；写入后路由按记录排序（M3 质量驱动选模）。
#[allow(clippy::too_many_arguments)]
pub async fn import_quality_score(
    state: State<'_, Backend>,
    provider_id: String,
    model_id: String,
    task_type: String,
    score_version: String,
    scores: [i64; 5],
    sample_count: u32,
    hard_fail: bool,
    rationale: String,
) -> Result<serde_json::Value> {
    let task = crate::agent::quality::parse_task_type(&task_type)?;
    crate::agent::quality::record_quality_score(
        &state.store.pool,
        &provider_id,
        &model_id,
        task,
        &score_version,
        scores,
        sample_count,
        hard_fail,
        &rationale,
    )
    .await?;
    Ok(serde_json::json!({"ok": true}))
}

#[tauri::command]
pub async fn cloud_login_open(app: AppHandle, state: State<'_, Backend>) -> Result<()> {
    let url = state.cloud_auth.begin().await?;
    if let Err(error) = crate::login_guard::open_browser(&url).await {
        state.cloud_auth.cancel().await?;
        let _ = app.emit("cloud-login-updated", ());
        return Err(error);
    }
    let _ = app.emit("cloud-login-updated", ());
    Ok(())
}
#[tauri::command]
pub async fn cloud_login_status(
    state: State<'_, Backend>,
) -> Result<crate::cloud_auth::CloudStatus> {
    state.cloud_auth.status().await
}
#[tauri::command]
pub async fn cloud_login_cancel(app: AppHandle, state: State<'_, Backend>) -> Result<()> {
    let result = state.cloud_auth.cancel().await;
    let _ = app.emit("cloud-login-updated", ());
    result
}
#[tauri::command]
pub async fn cloud_login_profile(
    state: State<'_, Backend>,
    display_name: Option<String>,
    avatar_url: Option<String>,
) -> Result<crate::cloud_auth::CloudStatus> {
    state
        .cloud_auth
        .update_profile(display_name, avatar_url)
        .await
}
#[tauri::command]
pub async fn cloud_login_logout(app: AppHandle, state: State<'_, Backend>) -> Result<bool> {
    let result = state.cloud_auth.logout().await;
    let _ = app.emit("cloud-login-updated", ());
    result
}

// ── G3 Narrative Canvas IPC ──

#[tauri::command]
pub async fn billing_catalog(state: State<'_, Backend>) -> Result<serde_json::Value> {
    state.cloud_auth.billing_catalog().await
}
#[tauri::command]
pub async fn billing_overview(state: State<'_, Backend>) -> Result<serde_json::Value> {
    state.cloud_auth.billing_overview().await
}
#[tauri::command]
pub async fn billing_checkout(
    state: State<'_, Backend>,
    price_id: String,
    request_id: String,
) -> Result<()> {
    let account = state.cloud_auth.account_id()?;
    let value = state
        .cloud_auth
        .billing_checkout(&price_id, &request_id)
        .await?;
    if state.cloud_auth.account_id()? != account {
        return Err(AppError::new(
            "AUTH_REQUIRED",
            "账户已变更，请重新打开订阅页面",
        ));
    }
    crate::billing::open_payment(&value, false).await
}
#[tauri::command]
pub async fn billing_manage(state: State<'_, Backend>) -> Result<()> {
    let account = state.cloud_auth.account_id()?;
    let value = state.cloud_auth.billing_manage().await?;
    if state.cloud_auth.account_id()? != account {
        return Err(AppError::new(
            "AUTH_REQUIRED",
            "账户已变更，请重新打开订阅页面",
        ));
    }
    crate::billing::open_payment(&value, true).await
}
#[tauri::command]
pub async fn billing_cancel_checkout(state: State<'_, Backend>) -> Result<()> {
    state.cloud_auth.billing_cancel_checkout().await?;
    Ok(())
}

#[tauri::command]
pub async fn canvas_execute(
    state: State<'_, Backend>,
    request: crate::canvas_commands::CanvasCommandRequest,
) -> Result<crate::canvas_commands::CanvasReceipt> {
    state.store.execute_canvas(request).await
}

#[tauri::command]
pub async fn canvas_comparison(
    state: State<'_, Backend>,
    project_id: String,
    board_id: String,
) -> Result<crate::canvas_review::CanvasComparison> {
    state.store.canvas_comparison(&project_id, &board_id).await
}

#[tauri::command]
pub async fn selection_revision_context(
    state: State<'_, Backend>,
    project_id: String,
    draft_id: String,
    offset: Option<usize>,
) -> Result<Option<crate::agent::SelectionContextPage>> {
    state
        .store
        .selection_revision_context(&project_id, &draft_id, offset.unwrap_or(0))
        .await
}

#[tauri::command]
pub async fn canvas_editorial_revision_context(
    state: State<'_, Backend>,
    project_id: String,
    draft_id: String,
    offset: Option<usize>,
) -> Result<Option<crate::canvas_review::RevisionContextPage>> {
    state
        .store
        .editorial_revision_context(&project_id, &draft_id, offset.unwrap_or(0))
        .await
}

#[tauri::command]
pub async fn canvas_editorial_discussion(
    state: State<'_, Backend>,
    project_id: String,
    draft_id: String,
    revision: i64,
    offset: Option<usize>,
    cursor: Option<i64>,
) -> Result<crate::canvas_review::EditorialDiscussion> {
    state
        .store
        .editorial_discussion(
            &project_id,
            &draft_id,
            revision,
            offset.unwrap_or(0),
            cursor,
        )
        .await
}

#[tauri::command]
pub async fn canvas_simulate_merge(
    state: State<'_, Backend>,
    project_id: String,
    selection: crate::canvas_review::EditorialSelection,
) -> Result<crate::canvas_review::MergeSimulation> {
    state
        .store
        .simulate_editorial_merge(&project_id, &selection)
        .await
}

#[tauri::command]
pub async fn canvas_editorial_history(
    state: State<'_, Backend>,
    project_id: String,
    board_id: String,
    before: Option<i64>,
) -> Result<Vec<crate::canvas_review::EditorialHistoryEntry>> {
    state
        .store
        .editorial_history_page(&project_id, &board_id, before)
        .await
}
#[tauri::command]
pub async fn canvas_simulate_revert(
    state: State<'_, Backend>,
    project_id: String,
    board_id: String,
    command_id: String,
) -> Result<crate::canvas_review::RevertSimulation> {
    state
        .store
        .simulate_editorial_revert(&project_id, &board_id, &command_id)
        .await
}

#[tauri::command]
pub async fn canvas_snapshot(
    state: State<'_, Backend>,
    project_id: String,
    board_id: String,
) -> Result<crate::canvas_commands::CanvasSnapshot> {
    state.store.canvas_snapshot(&project_id, &board_id).await
}

#[tauri::command]
pub async fn canvas_list_boards(
    state: State<'_, Backend>,
    project_id: String,
) -> Result<Vec<crate::canvas::Board>> {
    state.store.list_boards(&project_id).await
}

#[tauri::command]
pub async fn canvas_get_nodes(
    state: State<'_, Backend>,
    board_id: String,
) -> Result<Vec<crate::canvas::CanvasNode>> {
    state.store.get_board_nodes(&board_id).await
}

#[tauri::command]
pub async fn canvas_get_photo_sources(
    state: State<'_, Backend>,
    board_id: String,
) -> Result<Vec<crate::canvas::CanvasPhotoSource>> {
    state.store.board_photo_sources(&board_id).await
}

#[tauri::command]
pub async fn canvas_get_annotations(
    state: State<'_, Backend>,
    board_id: String,
) -> Result<Vec<crate::canvas::BoardAnnotation>> {
    state.store.get_annotations(&board_id).await
}

// ── G7 Publishing IPC ──

#[tauri::command]
pub async fn pub_execute(
    state: State<'_, Backend>,
    request: crate::publication_commands::PublicationCommandRequest,
) -> Result<crate::publication_commands::PublicationReceipt> {
    state.store.execute_publication(request).await
}
#[tauri::command]
pub async fn pub_layout(
    state: State<'_, Backend>,
    project_id: String,
    publication_id: String,
) -> Result<crate::publication_versions::PublicationDocument> {
    state
        .store
        .publication_document(&project_id, &publication_id)
        .await
}
#[tauri::command]
pub async fn pub_versions(
    state: State<'_, Backend>,
    project_id: String,
    publication_id: String,
    before: Option<i64>,
) -> Result<Vec<crate::publication_versions::PublicationVersionInfo>> {
    state
        .store
        .publication_versions(&project_id, &publication_id, before)
        .await
}
#[tauri::command]
pub async fn pub_version(
    state: State<'_, Backend>,
    project_id: String,
    publication_id: String,
    version_id: String,
) -> Result<crate::publication_versions::PublicationVersion> {
    state
        .store
        .publication_version(&project_id, &publication_id, &version_id)
        .await
}

#[tauri::command]
pub async fn pub_create(
    state: State<'_, Backend>,
    project_id: String,
    cmd: crate::publishing::CreatePublicationCommand,
) -> Result<crate::publishing::Publication> {
    state.store.create_publication(&project_id, &cmd).await
}

#[tauri::command]
pub async fn pub_list(
    state: State<'_, Backend>,
    project_id: String,
) -> Result<Vec<crate::publishing::Publication>> {
    state.store.list_publications(&project_id).await
}

#[tauri::command]
pub async fn pub_pages(
    state: State<'_, Backend>,
    project_id: String,
    publication_id: String,
) -> Result<Vec<crate::publishing::PublicationPage>> {
    state
        .store
        .publication_pages(&project_id, &publication_id)
        .await
}

#[tauri::command]
pub async fn pub_generate(
    state: State<'_, Backend>,
    project_id: String,
    cmd: crate::publishing::GenerateFromSequenceCommand,
) -> Result<i64> {
    state.store.generate_from_sequence(&project_id, &cmd).await
}

// ── G2-③ 色彩管理检视开关 ──

#[derive(serde::Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ColorManagedPreview {
    pub data_url: String,
    pub source_profile: String,
    pub display_profile: String,
    pub transform_applied: bool,
}

#[tauri::command]
pub async fn get_color_managed_preview(
    state: State<'_, Backend>,
    project_id: String,
    photo_id: String,
) -> Result<ColorManagedPreview> {
    // 读取预览路径
    let row: Option<(String, String)> = sqlx::query_as(
        "SELECT p.preview_path, p.original_path FROM photos p JOIN projects j ON j.id=p.project_id WHERE p.project_id=? AND p.id=? AND j.deleted_at IS NULL",
    )
    .bind(&project_id)
    .bind(&photo_id)
    .fetch_optional(&state.store.pool)
    .await?;
    let (preview_path, original_path) =
        row.ok_or_else(|| crate::domain::AppError::new("NOT_FOUND", "照片不存在"))?;

    // 读取源 ICC profile
    let source_profile =
        crate::color_engine::read_source_profile(std::path::Path::new(&original_path))
            .ok()
            .flatten();

    // 获取显示器 profile
    let display_icc = crate::color_engine::native_transform::display_profile_bytes()
        .unwrap_or_else(|_| Vec::new());

    let source_desc = source_profile
        .as_ref()
        .map(|p| p.description.clone())
        .unwrap_or_else(|| "未嵌入（假定为 sRGB）".to_owned());
    let display_desc = "显示器色彩空间".to_owned();

    // 如果有源 profile 且显示器 profile 可用，做变换
    let (data_url, transform_applied) =
        if let (Some(_src), true) = (&source_profile, !display_icc.is_empty()) {
            // 读预览 JPEG → 解码 → 变换 → 重编码
            let preview_bytes = std::fs::read(&preview_path).map_err(|e| {
                crate::domain::AppError::new("RESOURCE_MISSING", format!("预览缺失：{e}"))
            })?;
            let img = image::load_from_memory(&preview_bytes)
                .map_err(|e| crate::domain::AppError::new("DECODE", e.to_string()))?;
            let rgb = img.to_rgb8();
            let (w, h) = rgb.dimensions();
            let mut pixels = rgb.as_raw().clone();
            match crate::color_engine::native_transform::transform_rgb8(
                &Vec::new(),
                &display_icc,
                &mut pixels,
                w as usize,
                h as usize,
            ) {
                Ok(()) => {
                    let transformed = image::RgbImage::from_raw(w, h, pixels).ok_or_else(|| {
                        crate::domain::AppError::new("VALIDATION_ERROR", "像素重建失败")
                    })?;
                    let mut out = Vec::new();
                    let encoder = image::codecs::jpeg::JpegEncoder::new_with_quality(&mut out, 92);
                    image::DynamicImage::ImageRgb8(transformed)
                        .write_with_encoder(encoder)
                        .map_err(|e| crate::domain::AppError::new("ENCODE", e.to_string()))?;
                    (
                        format!("data:image/jpeg;base64,{}", base64_encode(&out)),
                        true,
                    )
                }
                Err(_) => {
                    // 变换失败回退到原始预览
                    (
                        format!("data:image/jpeg;base64,{}", base64_encode(&preview_bytes)),
                        false,
                    )
                }
            }
        } else {
            (
                format!(
                    "data:image/jpeg;base64,{}",
                    base64_encode(&std::fs::read(&preview_path).unwrap_or_default())
                ),
                false,
            )
        };

    Ok(ColorManagedPreview {
        data_url,
        source_profile: source_desc,
        display_profile: display_desc,
        transform_applied,
    })
}

fn base64_encode(data: &[u8]) -> String {
    use base64::Engine;
    base64::engine::general_purpose::STANDARD.encode(data)
}

#[tauri::command]
pub async fn cloud_session_token() -> Result<Option<String>> {
    use crate::cloud_credentials::{CredentialStore, SystemCredentials};
    let raw = match SystemCredentials.read("studio.betweenframes.app") {
        Ok(Some(v)) => v,
        _ => return Ok(None),
    };
    match serde_json::from_str::<serde_json::Value>(&raw) {
        Ok(v) => Ok(v
            .get("token")
            .and_then(|t| t.as_str())
            .map(|s| s.to_owned())),
        Err(_) => Ok(Some(raw)),
    }
}
