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

/// 图库派生文件的修复入口：命令只从库里取记录、把活交给 image_pipeline。
///
/// 原图只读：这条路径只写图库自己生成的缓存，且只写按这张照片的内容指纹命名的那两份；
/// 本来就是好的文件不动，色彩标注也就不会被白白重写一遍。
#[tauri::command]
pub async fn repair_photo_previews(
    state: State<'_, Backend>,
    project_id: String,
    photo_ids: Vec<String>,
) -> Result<Vec<image_pipeline::PreviewRepair>> {
    if photo_ids.is_empty() {
        return Err(AppError::new("INVALID_INPUT", "没有要点名的照片"));
    }
    if photo_ids.len() > 200 {
        return Err(AppError::new("INVALID_INPUT", "一次最多修复 200 张"));
    }
    state.store.get(&project_id).await?;
    let mut jobs: Vec<image_pipeline::PreviewJob> = vec![];
    for photo_id in &photo_ids {
        jobs.push(preview_job(&state.store.pool, &project_id, photo_id).await?);
    }
    let cache = state.cache.clone();
    tokio::task::spawn_blocking(move || image_pipeline::repair_preview_jobs(&cache, &jobs))
        .await
        .map_err(|error| AppError::new("BACKGROUND", error.to_string()))
}

/// 要读哪两份派生文件、原图在哪，一律取自库里在册的这条记录：调用方只能点名照片，给不了路径。
async fn preview_job(
    pool: &sqlx::SqlitePool,
    project_id: &str,
    photo_id: &str,
) -> Result<image_pipeline::PreviewJob> {
    let row = sqlx::query_as::<_, (String, String, String, String, String)>(
        "SELECT filename,original_path,content_sha256,thumbnail_path,preview_path FROM photos WHERE project_id=? AND id=?",
    )
    .bind(project_id)
    .bind(photo_id)
    .fetch_optional(pool)
    .await?;
    let Some((filename, original_path, content_sha256, thumbnail_path, preview_path)) = row else {
        return Err(AppError::new("NOT_FOUND", "照片不在该项目中"));
    };
    Ok(image_pipeline::PreviewJob {
        photo_id: photo_id.to_owned(),
        filename,
        original_path,
        content_sha256,
        thumbnail_path,
        preview_path,
    })
}

/// 单张本地文件的只读体检：与真动手重算前那道门同一套判断（含原图内容比对），但一个字节都不写。
///
/// 检视器先问清"这张现在能不能读"，再决定要不要摆重算按钮，不对着好文件推销修复。
#[tauri::command]
pub async fn inspect_photo_preview(
    state: State<'_, Backend>,
    project_id: String,
    photo_id: String,
) -> Result<image_pipeline::PreviewRepair> {
    state.store.get(&project_id).await?;
    let job = preview_job(&state.store.pool, &project_id, &photo_id).await?;
    let cache = state.cache.clone();
    tokio::task::spawn_blocking(move || image_pipeline::inspect_preview_job(&cache, &job))
        .await
        .map_err(|error| AppError::new("BACKGROUND", error.to_string()))
}

/// 整项目派生预览的只读点名：不改一个字节，只数出还有几张读不出来、各是因为什么。
#[tauri::command]
pub async fn audit_photo_previews(
    state: State<'_, Backend>,
    project_id: String,
) -> Result<serde_json::Value> {
    crate::preview_audit::audit(&state.store, &state.cache, &project_id).await
}

/// 整项目坏预览的一键修好：先点名，再按一轮的上限重算，修完把这台机器上各份方案的口径一起跟上。
#[tauri::command]
pub async fn repair_all_broken_previews(
    app: AppHandle,
    state: State<'_, Backend>,
    project_id: String,
) -> Result<serde_json::Value> {
    let report = crate::preview_audit::repair_all(&state.store, &state.cache, &project_id).await?;
    if report["repaired"].as_i64().unwrap_or_default() > 0 {
        for event in [
            crate::commands_generated::DRAFT_UPDATED_EVENT,
            crate::commands_generated::AGENT_TASK_UPDATED_EVENT,
        ] {
            let _ = app.emit(event, serde_json::json!({"projectId": project_id}));
        }
    }
    Ok(report)
}

/// 审稿方案口径的就地重算：只把库里存着的画面事实和现在的预览再比一遍，不经过模型、不上传照片。
///
/// 算完仍旧只往 `tool_calls` 里写一条回执，然后按方案更新广播一次：界面上的口径本来就是从回执
/// 投影出来的，这里再算一份数字只会造出两处真相。
#[tauri::command]
pub async fn recheck_draft_evidence(
    app: AppHandle,
    state: State<'_, Backend>,
    project_id: String,
    draft_id: String,
) -> Result<serde_json::Value> {
    let summary = crate::agent::recheck::run(&state.store, &project_id, &draft_id).await?;
    for event in [
        crate::commands_generated::DRAFT_UPDATED_EVENT,
        crate::commands_generated::AGENT_TASK_UPDATED_EVENT,
    ] {
        let _ = app.emit(event, serde_json::json!({"projectId": project_id}));
    }
    Ok(summary)
}

/// 接着把整项目还欠着的方案口径补上：修完预览那一轮算不完时，不必重新修图也能往下算。
/// 仍旧只往回执表里写，不经过模型、不上传照片，也不占项目修订号。
#[tauri::command]
pub async fn recheck_project_draft_evidence(
    app: AppHandle,
    state: State<'_, Backend>,
    project_id: String,
) -> Result<serde_json::Value> {
    let report = crate::agent::recheck::project(
        &state.store,
        &project_id,
        crate::agent::recheck::ROUND_LIMIT,
    )
    .await?;
    if report["recheckedDrafts"].as_i64().unwrap_or_default() > 0 {
        for event in [
            crate::commands_generated::DRAFT_UPDATED_EVENT,
            crate::commands_generated::AGENT_TASK_UPDATED_EVENT,
        ] {
            let _ = app.emit(event, serde_json::json!({"projectId": project_id}));
        }
    }
    Ok(report)
}

/// 点名"当年没记像素版本"的历史观察：只读比对本地文件，一个字节都不改，也不问模型。
#[tauri::command]
pub async fn audit_observation_pixel_versions(
    state: State<'_, Backend>,
    project_id: String,
) -> Result<serde_json::Value> {
    crate::observation_backfill::audit(&state.store, &project_id).await
}

/// 把能证明的那部分观察补上像素版本，并让收着这些照片的方案口径跟着重算。
/// 全程本地：不上传照片、不花额度，补的也只是能核对上"当初发出去的就是这一份字节"的那部分。
#[tauri::command]
pub async fn backfill_observation_pixel_versions(
    app: AppHandle,
    state: State<'_, Backend>,
    project_id: String,
) -> Result<serde_json::Value> {
    let report = crate::observation_backfill::apply(
        &state.store,
        &project_id,
        crate::observation_backfill::APPLY_LIMIT,
    )
    .await?;
    if report["recheck"]["recheckedDrafts"]
        .as_i64()
        .unwrap_or_default()
        > 0
    {
        for event in [
            crate::commands_generated::DRAFT_UPDATED_EVENT,
            crate::commands_generated::AGENT_TASK_UPDATED_EVENT,
        ] {
            let _ = app.emit(event, serde_json::json!({"projectId": project_id}));
        }
    }
    Ok(report)
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

/// 外部编辑器就地改写原图后，把这条记录更新成文件现在真正的内容。
/// 要读哪个文件由库里这条记录决定，界面只能点名照片，不能指定路径。
#[tauri::command]
pub async fn apply_photo_change(
    state: State<'_, Backend>,
    project_id: String,
    photo_id: String,
    expected_revision: i64,
) -> Result<Project> {
    let source = state.store.photo_source(&project_id, &photo_id).await?;
    let cache = state.cache.clone();
    let prepared = tokio::task::spawn_blocking(move || image_pipeline::prepare(&source, &cache))
        .await
        .map_err(|error| AppError::new("BACKGROUND", error.to_string()))??;
    state
        .store
        .apply_photo_change(&project_id, &photo_id, &prepared, expected_revision)
        .await
}

/// 只读：把每条记录自己的 XMP 边车与台账判断并列出来，不占修订号、不落库。
#[tauri::command]
pub async fn sidecar_proposals(
    state: State<'_, Backend>,
    project_id: String,
) -> Result<Vec<crate::sidecar::SidecarProposal>> {
    let rows = state.store.sidecar_rows(&project_id).await?;
    // 边车是一堆小文件，读它们放进阻塞线程，不占着界面这条异步通道。
    let found = tokio::task::spawn_blocking(move || crate::sidecar::proposals_for(&rows))
        .await
        .map_err(|error| AppError::new("BACKGROUND", error.to_string()))??;
    Ok(found)
}

#[tauri::command]
pub async fn apply_sidecar_judgement(
    state: State<'_, Backend>,
    project_id: String,
    photo_id: String,
    expected_revision: i64,
) -> Result<Project> {
    state
        .store
        .apply_sidecar_judgement(&project_id, &photo_id, expected_revision)
        .await
}

/// 一键回填：只把"本地还空着"的那部分边车判断落库，人工判过的一条不碰。
/// 界面仍然只递项目与期望修订号——要读哪些边车由台账自己说了算。
#[tauri::command]
pub async fn backfill_sidecar_judgements(
    state: State<'_, Backend>,
    project_id: String,
    expected_revision: i64,
) -> Result<crate::sidecar::SidecarBatchOutcome> {
    state
        .store
        .backfill_sidecar_judgements(&project_id, expected_revision)
        .await
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
pub async fn agent_chat(
    state: State<'_, Backend>,
    turns: Vec<crate::agent::types::ChatTurn>,
    request_id: String,
) -> Result<String> {
    // 一次性路径同样带编号：回复在路上丢了才核得回来，两条发送路不能一条可核对一条不可。
    state.agent.chat_turn_with_id(&turns, &request_id).await
}
/// 聊天流式命令：增量经 IPC channel 逐段送到前端，命令返回值仍是服务端的最终回执正文。
/// channel 只活在这次 invoke 里，不经过全局事件总线，切换项目或窗口关闭都不会串台。
#[tauri::command]
pub async fn agent_chat_stream(
    state: State<'_, Backend>,
    turns: Vec<crate::agent::types::ChatTurn>,
    request_id: String,
    on_delta: tauri::ipc::Channel<String>,
) -> Result<String> {
    state
        .agent
        .chat_turn_stream(&turns, &request_id, |fragment| {
            // 前端已断开时 send 失败：继续读完服务端回执，不中断、也不重发。
            let _ = on_delta.send(fragment.to_owned());
        })
        .await
}
/// 断流后的只读核对：只凭前端留存的那个请求编号向服务端查一次回执，绝不重新提交。
/// 界面据此决定这条回复是上屏、继续等，还是承认"结果不明、请勿重复提交"。
#[tauri::command]
pub async fn agent_chat_job(
    state: State<'_, Backend>,
    request_id: String,
) -> Result<serde_json::Value> {
    serde_json::to_value(state.agent.chat_job(&request_id).await?)
        .map_err(crate::command_audit::serialization)
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
#[tauri::command]
pub async fn cloud_stats_overview(state: State<'_, Backend>) -> Result<serde_json::Value> {
    state.cloud_auth.stats_overview().await
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

#[tauri::command]
pub async fn pub_preflight(
    state: State<'_, Backend>,
    project_id: String,
    publication_id: String,
    min_ppi: Option<i64>,
) -> Result<crate::publication_preflight::PreflightReport> {
    state
        .store
        .publication_preflight(&project_id, &publication_id, min_ppi)
        .await
}

/// 审稿面板：这一本的审阅意见。读侧不带修订号、不进幂等队列。
#[tauri::command]
pub async fn pub_review(
    state: State<'_, Backend>,
    project_id: String,
    publication_id: String,
) -> Result<serde_json::Value> {
    crate::agent::comments::publication_review(&state.store, &project_id, &publication_id).await
}

/// 审阅意见的写侧：挂到当前版本、明确解决或重开；回执是刷新后的整份清单。
#[tauri::command]
pub async fn pub_review_write(
    state: State<'_, Backend>,
    project_id: String,
    publication_id: String,
    request: crate::agent::comments::PublicationReviewRequest,
) -> Result<serde_json::Value> {
    crate::agent::comments::publication_review_write(
        &state.store,
        &project_id,
        &publication_id,
        request,
    )
    .await
}

/// 交付出口：把这本的版面导成一张可核对的作品清单 CSV。
#[tauri::command]
pub async fn pub_export(
    app: AppHandle,
    state: State<'_, Backend>,
    project_id: String,
    publication_id: String,
) -> Result<Option<ExportReceipt>> {
    let layout = state
        .store
        .publication_layout(&project_id, &publication_id)
        .await?;
    let csv = crate::publication_export::render_worklist(&layout);
    let filename = crate::publication_export::file_name(&layout.publication.title);
    tokio::task::spawn_blocking(move || {
        let Some(path) = app
            .dialog()
            .file()
            .set_title("导出作品清单（选择新的文件名）")
            .set_file_name(&filename)
            .add_filter("作品清单 CSV", &["csv"])
            .blocking_save_file()
        else {
            return Ok(None);
        };
        let path = path
            .into_path()
            .map_err(|error| AppError::new("INVALID_PATH", error.to_string()))?;
        crate::publication_export::write(&csv, &path).map(Some)
    })
    .await
    .map_err(|error| AppError::new("BACKGROUND", error.to_string()))?
}

// ── G2-③ 色彩管理检视开关 ──

#[derive(serde::Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ColorManagedPreview {
    pub pixel_source: &'static str,
    /// 已变换 = transformed / assumed_srgb；其余状态说明为什么没变换。
    pub status: &'static str,
    pub detail: Option<String>,
    pub source_profile: String,
    pub display_profile: String,
    /// 未做变换时为 None：界面保持缓存原样，不假装换过颜色。
    pub data_url: Option<String>,
}

#[tauri::command]
pub async fn get_color_managed_preview(
    state: State<'_, Backend>,
    project_id: String,
    photo_id: String,
) -> Result<ColorManagedPreview> {
    let outcome =
        crate::photo_inspection::display_managed_preview(&state.store, &project_id, &photo_id)
            .await?;

    Ok(ColorManagedPreview {
        pixel_source: "original",
        status: outcome.status,
        detail: outcome.detail,
        source_profile: outcome.source_profile,
        display_profile: outcome.display_profile,
        data_url: outcome
            .jpeg
            .map(|bytes| format!("data:image/jpeg;base64,{}", base64_encode(&bytes))),
    })
}

/// 100% 检视的按显示器渲染：与预览开关同一套状态契约。`path` 为 None 表示没变换，
/// 界面继续显示缓存预览的重渲染版本，不会拿没变换的原图冒充显示器色彩。
#[tauri::command]
pub async fn get_display_managed_inspection(
    app: AppHandle,
    state: State<'_, Backend>,
    project_id: String,
    photo_id: String,
) -> Result<crate::photo_inspection::ManagedInspection> {
    let outcome =
        crate::photo_inspection::display_managed_inspection(&state.store, &project_id, &photo_id)
            .await?;
    if let Some(path) = &outcome.path {
        app.asset_protocol_scope()
            .allow_file(std::path::Path::new(path))
            .map_err(|error| AppError::new("ASSET_SCOPE", error.to_string()))?;
    }
    Ok(outcome)
}

fn base64_encode(data: &[u8]) -> String {
    use base64::Engine;
    base64::engine::general_purpose::STANDARD.encode(data)
}
