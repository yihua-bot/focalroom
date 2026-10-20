pub mod agent;
pub mod canvas;
pub mod cloud_auth;
pub mod cloud_credentials;
pub mod color_engine;
mod command_audit;
pub mod command_handler;
mod commands;
pub mod commands_generated;
mod cr2_preview;
pub mod db;
pub mod diagnostics;
pub mod domain;
mod edit_commands;
mod edit_state;
pub mod export;
pub mod external_edit;
pub mod image_pipeline;
pub mod importer;
pub mod keychain;
pub mod login_guard;
pub mod photo_inspection;
pub mod product_links;
mod resource_commands;
pub mod seed_demo;
pub mod shared_cloud;
pub mod shared_export;
pub mod v2_bridge;
use tauri::Manager;
#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_dialog::init())
        .plugin(tauri_plugin_deep_link::init())
        .setup(|app| {
            let directory = app.path().app_data_dir()?.join("BetweenFrames");
            let cache = directory.join("cache");
            std::fs::create_dir_all(&cache)?;
            let store =
                tauri::async_runtime::block_on(db::Store::open(&directory.join("projects.sqlite")))
                    .map_err(|error| std::io::Error::other(error.message))?;
            app.asset_protocol_scope().allow_directory(&cache, true)?;
            tauri::async_runtime::block_on(
                command_handler::CommandHandler::new(&store).recover_agents(),
            )
            .map_err(|error| std::io::Error::other(error.message))?;
            let resource_demo = app.path().resource_dir()?.join("resources/demo");
            tauri::async_runtime::block_on(seed_demo::ensure_demo(&store, &resource_demo, &cache))
                .map_err(|error| std::io::Error::other(error.message))?;
            let cloud_origin = login_guard::CLOUD_ORIGIN.to_owned();
            #[cfg(feature = "integration-harness")]
            let cloud_origin = match std::env::var("FOCALROOM_LOGIN_REVIEW_ORIGIN")
                .ok()
                .or_else(|| option_env!("FOCALROOM_LOGIN_REVIEW_ORIGIN").map(str::to_owned))
            {
                Some(origin) => {
                    let trusted = login_guard::trusted_origin(&origin)
                        .map_err(|e| std::io::Error::other(e.message))?;
                    if !trusted.starts_with("http://") {
                        return Err(std::io::Error::other("评审登录仅允许本机回环服务").into());
                    }
                    trusted
                }
                None => cloud_origin,
            };
            let cloud_auth = std::sync::Arc::new(
                cloud_auth::CloudAuth::new(
                    &cloud_origin,
                    &app.config().identifier,
                    std::sync::Arc::new(cloud_credentials::SystemCredentials),
                )
                .map_err(|error| std::io::Error::other(error.message))?,
            );
            cloud_auth.set_avatar_cache(cache.join("avatars"));
            let handle = app.handle().clone();
            let agent = agent::runtime::AgentEngine::hosted(store.clone(), cloud_auth.clone())
                .with_notify(std::sync::Arc::new(move |project| {
                    use tauri::Emitter;
                    let _ = handle.emit(
                        commands_generated::AGENT_TASK_UPDATED_EVENT,
                        serde_json::json!({"projectId":project}),
                    );
                }));

            {
                use tauri_plugin_deep_link::DeepLinkExt;
                let auth = cloud_auth.clone();
                let handle = app.handle().clone();
                let seen = std::sync::Mutex::new(None::<String>);
                let dispatch = std::sync::Arc::new(move |urls: Vec<reqwest::Url>| {
                    for url in urls {
                        // 冷启动查询和运行中事件可能传回同一URL，仅处理一次。
                        use sha2::Digest;
                        let fingerprint =
                            format!("{:x}", sha2::Sha256::digest(url.as_str().as_bytes()));
                        let mut last = seen.lock().unwrap_or_else(|p| p.into_inner());
                        if last.as_ref() == Some(&fingerprint) {
                            continue;
                        }
                        *last = Some(fingerprint);
                        drop(last);
                        let auth = auth.clone();
                        let handle = handle.clone();
                        tauri::async_runtime::spawn(async move {
                            if let Err(error) = auth.complete(url.as_str()).await {
                                auth.remember_error(&error);
                            }
                            use tauri::Emitter;
                            let _ = handle.emit("cloud-login-updated", ());
                        });
                    }
                });
                let listener = dispatch.clone();
                app.deep_link()
                    .on_open_url(move |event| listener(event.urls()));
                if let Some(urls) = app.deep_link().get_current()? {
                    dispatch(urls);
                }
            }
            app.manage(commands::Backend {
                cloud_auth,
                agent,
                store: store.clone(),
                cache: cache.clone(),
                imports: importer::ImportManager::default(),
                grants: Default::default(),
            });
            // 启动后异步清一次孤儿缓存（M1-09）：不阻塞启动，失败只记录不影响使用。
            let sweep_store = store.clone();
            let sweep_cache = cache.clone();
            tauri::async_runtime::spawn(async move {
                let limit = image_pipeline::cache_limit_bytes();
                if let Ok(hashes) = sweep_store.photo_hashes().await
                    && let Ok(reclaimed) =
                        image_pipeline::sweep_orphans(&sweep_cache, &hashes, limit)
                    && reclaimed > 0
                {
                    eprintln!("缓存配额清理：回收 {reclaimed} 字节孤儿缓存");
                }
            });
            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            product_links::open_product_link,
            commands::agent_request,
            commands::get_agent_projection,
            commands::get_ai_config,
            commands::get_memory_projection,
            commands::get_proposals,
            commands::execute_v2_command,
            commands::execute_memory_command,
            commands::store_api_key,
            commands::delete_api_key,
            commands::get_diagnostics,
            commands::set_project_tags,
            commands::import_quality_score,
            commands::correct_hypothesis,
            commands::set_hypothesis_enabled,
            commands::set_privacy_consent,
            commands::list_projects,
            commands::get_project,
            commands::get_original_photo,
            commands::execute_gui_command,
            commands::choose_import,
            commands::import_photos,
            commands::get_import_job,
            commands::cancel_import,
            commands::retry_import,
            commands::relink_photo,
            commands::export_project,
            commands::get_sharing_status,
            commands::open_shared_review,
            commands::cloud_login_open,
            commands::cloud_login_status,
            commands::cloud_login_profile,
            commands::cloud_login_cancel,
            commands::cloud_login_logout,
            commands::share_project,
            commands::canvas_create_board,
            commands::canvas_list_boards,
            commands::canvas_place_photo,
            commands::canvas_move_nodes,
            commands::canvas_get_nodes,
            commands::canvas_add_text,
            commands::canvas_delete_node,
            commands::canvas_add_annotation,
            commands::canvas_resolve_annotation,
            commands::canvas_get_annotations,
            commands::canvas_add_arrow,
            commands::canvas_undo,
            commands::canvas_add_ink
        ])
        .run(tauri::generate_context!())
        .expect("无法启动 FocalRoom 桌面应用");
}
