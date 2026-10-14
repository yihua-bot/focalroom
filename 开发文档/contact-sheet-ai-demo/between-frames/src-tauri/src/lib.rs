pub mod agent;
mod command_audit;
pub mod command_handler;
mod commands;
pub mod commands_generated;
pub mod db;
pub mod domain;
mod edit_commands;
mod edit_state;
pub mod export;
pub mod image_pipeline;
pub mod importer;
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
                store,
                cache,
                imports: importer::ImportManager::default(),
                grants: Default::default(),
            });
            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            commands::agent_request,
            commands::get_agent_projection,
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
