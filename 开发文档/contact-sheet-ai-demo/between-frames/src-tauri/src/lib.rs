pub mod agent;
mod command_audit;
pub mod command_handler;
mod commands;
pub mod commands_generated;
pub mod db;
pub mod diagnostics;
pub mod domain;
mod edit_commands;
mod edit_state;
pub mod export;
pub mod image_pipeline;
pub mod importer;
pub mod keychain;
mod resource_commands;
use tauri::Manager;
#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_dialog::init())
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
            let handle = app.handle().clone();
            let agent = agent::runtime::AgentEngine::new(store.clone()).with_notify(
                std::sync::Arc::new(move |project| {
                    use tauri::Emitter;
                    let _ = handle.emit(
                        commands_generated::AGENT_TASK_UPDATED_EVENT,
                        serde_json::json!({"projectId":project}),
                    );
                }),
            );
            app.manage(commands::Backend {
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
            commands::agent_request,
            commands::get_agent_projection,
            commands::get_ai_config,
            commands::get_memory_projection,
            commands::execute_memory_command,
            commands::store_api_key,
            commands::delete_api_key,
            commands::get_diagnostics,
            commands::set_project_tags,
            commands::correct_hypothesis,
            commands::set_hypothesis_enabled,
            commands::set_privacy_consent,
            commands::list_projects,
            commands::get_project,
            commands::execute_gui_command,
            commands::choose_import,
            commands::import_photos,
            commands::get_import_job,
            commands::cancel_import,
            commands::retry_import,
            commands::relink_photo,
            commands::export_project
        ])
        .run(tauri::generate_context!())
        .expect("无法启动序间桌面应用");
}
