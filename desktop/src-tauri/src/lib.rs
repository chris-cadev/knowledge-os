mod commands;
mod wsl;

use commands::AppState;
use knowledge_storage::adapters::sqlite::SqliteStore;
use std::sync::Arc;
use tauri::Manager;

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_dialog::init())
        .plugin(tauri_plugin_opener::init())
        .setup(|app| {
            if cfg!(debug_assertions) {
                app.handle().plugin(
                    tauri_plugin_log::Builder::default()
                        .level(log::LevelFilter::Info)
                        .build(),
                )?;
            }

            let data_dir = app.path().app_data_dir().map_err(|e| {
                Box::new(std::io::Error::other(format!(
                    "failed to resolve app data dir: {}",
                    e
                ))) as Box<dyn std::error::Error>
            })?;
            std::fs::create_dir_all(&data_dir).map_err(|e| {
                Box::new(std::io::Error::other(format!(
                    "failed to create app data dir: {}",
                    e
                ))) as Box<dyn std::error::Error>
            })?;
            let db_path = data_dir.join("knowledge.db");

            let store = Arc::new(
                SqliteStore::new(db_path.to_str().ok_or_else(|| {
                    Box::new(std::io::Error::other("invalid database path"))
                        as Box<dyn std::error::Error>
                })?)
                .map_err(|e| {
                    Box::new(std::io::Error::other(format!(
                        "failed to open database: {}",
                        e
                    ))) as Box<dyn std::error::Error>
                })?,
            );

            app.manage(AppState { store });
            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            commands::entity::list_entities,
            commands::entity::get_entity_detail,
            commands::entity::get_entity_source,
            commands::file::open_in_default_app,
            commands::file::open_source_folder,
            commands::import::import_files,
            commands::import::import_url,
            commands::import::import_clipboard,
            commands::import::import_database,
            commands::import::import_file_recursive,
            commands::import::import_image,
            commands::import::undo_import,
            commands::import::import_directory_preview,
            commands::import::import_structured_preview,
            commands::import::import_structured,
            commands::search::search_entities,
            commands::view::get_graph_view,
            commands::view::get_tree_view,
            commands::view::get_table_view,
            commands::view::get_timeline_view,
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
