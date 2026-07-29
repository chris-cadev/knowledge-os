mod commands;
mod wsl;

use commands::AppState;
use commands::StoreWrapper;
use knowledge_core::ports::{
    ChatCompletion, ComponentRepository, EntityRepository, RelationshipRepository, SearchIndex,
    TraversalPort, VectorStore,
};
use knowledge_core::services::entity_retrieval::EntityRetrievalService;
use knowledge_derivation::features::chat::pipeline::ChatPipeline;
use knowledge_derivation::features::search::vector_store::InMemoryVectorStore;
use knowledge_storage::adapters::sqlite::SqliteStore;
use std::sync::Arc;
use tauri::Manager;

fn create_chat_provider() -> Result<Arc<dyn ChatCompletion>, String> {
    Ok(Arc::new(
        knowledge_derivation::features::chat::mock::MockChatAdapter::default(),
    ))
}

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

            let wrapper = Arc::new(StoreWrapper(store.clone()));

            let entity_repo: Arc<dyn EntityRepository> = wrapper.clone();
            let component_repo: Arc<dyn ComponentRepository> = wrapper.clone();
            let relationship_repo: Arc<dyn RelationshipRepository> = wrapper.clone();
            let search_index: Arc<dyn SearchIndex> = wrapper.clone();
            let vector_store: Arc<dyn VectorStore> =
                Arc::new(InMemoryVectorStore::new(128));
            let traversal_port: Arc<dyn TraversalPort> = wrapper.clone();

            let chat_provider = create_chat_provider()?;
            let chat_provider_kind = "mock".to_string();

            let chat_pipeline = Arc::new(ChatPipeline::new(
                chat_provider,
                entity_repo.clone(),
                component_repo.clone(),
                relationship_repo.clone(),
                search_index.clone(),
                vector_store,
            ));

            let entity_retrieval = Arc::new(EntityRetrievalService::new(
                entity_repo,
                component_repo,
                relationship_repo,
                search_index,
                traversal_port,
            ));

            app.manage(AppState {
                store,
                chat_pipeline,
                entity_retrieval,
                chat_provider_kind,
            });
            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            commands::chat::chat_send,
            commands::chat::chat_stream,
            commands::chat::chat_search_entities,
            commands::chat::chat_list_conversations,
            commands::chat::chat_delete_conversation,
            commands::chat::chat_rename_conversation,
            commands::chat::chat_stop_stream,
            commands::chat::chat_send_feedback,
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
