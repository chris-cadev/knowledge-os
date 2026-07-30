mod commands;
mod logger;
mod wsl;

use commands::provider::{self, load_provider_config};
use commands::AppState;
use commands::StoreWrapper;
use knowledge_core::ports::{
    ComponentRepository, EntityRepository, RelationshipRepository, SearchIndex, TraversalPort,
    VectorStore,
};
use knowledge_core::services::entity_retrieval::EntityRetrievalService;
use knowledge_derivation::features::chat::pipeline::ChatPipeline;
use knowledge_derivation::features::search::vector_store::InMemoryVectorStore;
use knowledge_storage::adapters::sqlite::SqliteStore;
use std::sync::Arc;
use tauri::Manager;
use tokio::sync::Mutex;

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_dialog::init())
        .plugin(tauri_plugin_opener::init())
        .setup(|app| {
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

            logger::init(&data_dir).map_err(|e| {
                Box::new(std::io::Error::other(format!(
                    "failed to initialize logger: {}",
                    e
                ))) as Box<dyn std::error::Error>
            })?;

            logger::install_tauri_bridge(app.handle().clone());

            log::info!("app.starting: data_dir={:?}", data_dir);

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
            let vector_store: Arc<dyn VectorStore> = Arc::new(InMemoryVectorStore::new(128));
            let traversal_port: Arc<dyn TraversalPort> = wrapper.clone();

            let config = tauri::async_runtime::block_on(load_provider_config(&data_dir));
            let chat_provider_kind = config.provider_kind.clone();
            let (chat_provider, _kind) = provider::create_chat_provider_from_config(&config)
                .unwrap_or_else(|_| {
                    let mock =
                        knowledge_derivation::features::chat::mock::MockChatAdapter::default();
                    (Arc::new(mock), "mock".to_string())
                });

            let chat_pipeline = Arc::new(Mutex::new(ChatPipeline::new(
                chat_provider,
                entity_repo.clone(),
                component_repo.clone(),
                relationship_repo.clone(),
                search_index.clone(),
                vector_store,
            )));

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
                chat_provider_kind: Arc::new(Mutex::new(chat_provider_kind)),
                data_dir,
            });
            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            commands::chat::chat_send,
            commands::chat::chat_stream,
            commands::chat::chat_search_entities,
            commands::chat::chat_list_conversations,
            commands::chat::chat_get_conversation,
            commands::chat::chat_delete_conversation,
            commands::chat::chat_rename_conversation,
            commands::chat::chat_stop_stream,
            commands::chat::chat_send_feedback,
            commands::provider::set_provider,
            commands::provider::get_providers_status,
            commands::provider::reset_provider,
            commands::provider::set_ocr_provider,
            commands::provider::get_ocr_provider_status,
            commands::provider::reset_ocr_provider,
            commands::provider::chat_test_provider,
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
