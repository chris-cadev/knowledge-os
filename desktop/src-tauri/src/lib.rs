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

            // Open (or create) the SQLite database.
            let store = Arc::new(SqliteStore::new("knowledge.db").map_err(|e| {
                Box::new(std::io::Error::other(format!(
                    "failed to open database: {}",
                    e
                ))) as Box<dyn std::error::Error>
            })?);

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
            commands::search::search_entities,
            commands::view::get_graph_view,
            commands::view::get_tree_view,
            commands::view::get_table_view,
            commands::view::get_timeline_view,
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
