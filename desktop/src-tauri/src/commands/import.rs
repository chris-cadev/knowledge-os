use knowledge_core::features::relationship::Relationship;
use knowledge_core::features::relationship::RelationshipType;
use knowledge_core::ports::{
    ComponentRepository, Event, EventLog, EventType, RelationshipRepository, SearchIndex,
    TransactionalWrite,
};
use knowledge_import::features::importer::{ImportAdapter, MarkdownImporter, PdfImporter};
use std::path::Path;
use tauri::State;

use super::response::*;
use super::store::AppState;

/// Import one or more files into the knowledge base.
///
/// Accepts file paths resolved from the frontend dialog or drag-drop.
/// Supports `.md` and `.pdf` files.
#[tauri::command]
pub async fn import_files(
    state: State<'_, AppState>,
    paths: Vec<String>,
) -> Result<ImportResultResponse, String> {
    let store = &*state.store;

    let markdown_importer = MarkdownImporter::new();
    let pdf_importer = PdfImporter::new();

    let mut created = 0usize;
    let merged = 0usize;
    let mut errors: Vec<ImportErrorResponse> = Vec::new();

    for path_str in &paths {
        let path = Path::new(path_str);

        // Collect files: if path is a directory, discover supported files
        let mut files: Vec<std::path::PathBuf> = Vec::new();
        if path.is_dir() {
            match std::fs::read_dir(path) {
                Ok(entries) => {
                    for entry in entries.flatten() {
                        let file_path = entry.path();
                        let ext = file_path.extension().and_then(|e| e.to_str()).unwrap_or("");
                        if ext.eq_ignore_ascii_case("md") || ext.eq_ignore_ascii_case("pdf") {
                            files.push(file_path);
                        }
                    }
                }
                Err(e) => {
                    errors.push(ImportErrorResponse {
                        path: path_str.clone(),
                        message: format!("failed to read directory: {}", e),
                    });
                    continue;
                }
            }
        } else {
            files.push(path.to_path_buf());
        }

        for file_path in &files {
            // Determine importer based on extension
            let ext = file_path
                .extension()
                .and_then(|e| e.to_str())
                .unwrap_or("")
                .to_lowercase();

            let result = if ext == "pdf" {
                pdf_importer.import(file_path).await
            } else if ext == "md" {
                markdown_importer.import(file_path).await
            } else {
                Err(
                    knowledge_import::features::importer::ImportError::UnsupportedFormat(format!(
                        "unsupported file type: .{}",
                        ext
                    )),
                )
            };

            let import_result = match result {
                Ok(r) => r,
                Err(e) => {
                    errors.push(ImportErrorResponse {
                        path: file_path.to_string_lossy().to_string(),
                        message: e.to_string(),
                    });
                    continue;
                }
            };

            let source_str = file_path.to_string_lossy();

            // Create event
            let event = Event {
                id: uuid::Uuid::new_v4(),
                event_type: EventType::EntityCreated,
                entity_id: import_result.entity.id,
                timestamp: chrono::Utc::now(),
                data: serde_json::json!({"source": source_str}),
            };

            // Save entity + components + event in a transaction
            TransactionalWrite::save_entity_with_components(
                store,
                &import_result.entity,
                &import_result.components,
                &event,
            )
            .await
            .map_err(|e| format!("failed to save entity: {}", e))?;

            // Index for search
            SearchIndex::index_entity(store, &import_result.entity, &import_result.components)
                .await
                .map_err(|e| format!("failed to index entity: {}", e))?;

            // Create cross-reference relationships
            for cross_ref in &import_result.cross_references {
                let target_id = match cross_ref {
                    knowledge_import::features::importer::CrossReference::FileRef {
                        target_path,
                        ..
                    } => {
                        let matching = ComponentRepository::find_by_component_data(
                            store,
                            "Provenance",
                            "source",
                            &target_path.to_string_lossy(),
                        )
                        .await
                        .map_err(|e| e.to_string())?;
                        matching.first().map(|c| c.entity_id)
                    }
                    knowledge_import::features::importer::CrossReference::WikilinkRef {
                        target_name,
                        ..
                    }
                    | knowledge_import::features::importer::CrossReference::MentionRef {
                        target_name,
                    } => {
                        let matching = ComponentRepository::find_by_component_data(
                            store,
                            "Title",
                            "title",
                            target_name,
                        )
                        .await
                        .map_err(|e| e.to_string())?;
                        matching.first().map(|c| c.entity_id)
                    }
                    knowledge_import::features::importer::CrossReference::SectionRef {
                        target_path,
                        ..
                    } => {
                        let matching = ComponentRepository::find_by_component_data(
                            store,
                            "Provenance",
                            "source",
                            &target_path.to_string_lossy(),
                        )
                        .await
                        .map_err(|e| e.to_string())?;
                        matching.first().map(|c| c.entity_id)
                    }
                    knowledge_import::features::importer::CrossReference::UrlRef { .. } => None,
                };

                if let Some(target_id) = target_id {
                    // Skip if relationship already exists
                    let existing = RelationshipRepository::find_by_source_and_target(
                        store,
                        import_result.entity.id,
                        target_id,
                    )
                    .await
                    .map_err(|e| e.to_string())?;

                    if existing.is_none() {
                        let rel = Relationship::new(
                            import_result.entity.id,
                            target_id,
                            RelationshipType::References,
                        );
                        RelationshipRepository::save(store, &rel)
                            .await
                            .map_err(|e| e.to_string())?;

                        let rel_event = Event {
                            id: uuid::Uuid::new_v4(),
                            event_type: EventType::RelationshipCreated,
                            entity_id: import_result.entity.id,
                            timestamp: chrono::Utc::now(),
                            data: serde_json::json!({
                                "target_id": target_id.to_string(),
                                "type": "References",
                            }),
                        };
                        EventLog::append(store, &rel_event)
                            .await
                            .map_err(|e| e.to_string())?;
                    }
                }
            }

            created += 1;
        }
    }

    Ok(ImportResultResponse {
        created,
        merged,
        errors,
    })
}
