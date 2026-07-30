use knowledge_core::ports::{
    ComponentRepository, DatabaseSource, Event, EventType, RelationshipRepository, SearchIndex,
    TransactionalWrite,
};
use knowledge_import::features::importer::database::{
    MySqlDatabaseSource, PostgresDatabaseSource, SqliteDatabaseSource,
};
use knowledge_import::features::importer::{
    ClipboardImporter, DirectoryImporter, ImportAdapter, UrlImporter,
};
use knowledge_plugin::registry::built_in_plugins;
use std::collections::HashMap;
use tauri::State;

use super::response::*;
use super::store::AppState;

/// Import one or more files into the knowledge base using the importer registry.
#[tauri::command]
pub async fn import_files(
    state: State<'_, AppState>,
    paths: Vec<String>,
) -> Result<ImportProgressResponse, String> {
    let store = &*state.store;
    let registry = built_in_plugins();
    let mut items = Vec::new();
    let mut created = 0usize;
    let mut merged = 0usize;
    let mut errors: Vec<ImportErrorResponse> = Vec::new();

    for path_str in &paths {
        let path = std::path::Path::new(path_str);

        if path.is_dir() {
            let dir_importer = DirectoryImporter::new(true);
            let files = match dir_importer.list_files(path) {
                Ok(f) => f,
                Err(e) => {
                    errors.push(ImportErrorResponse {
                        path: path_str.clone(),
                        message: e.to_string(),
                    });
                    continue;
                }
            };
            for file_path in &files {
                import_single_file(
                    store,
                    &registry,
                    file_path,
                    &mut items,
                    &mut created,
                    &mut merged,
                    &mut errors,
                )
                .await;
            }
        } else {
            import_single_file(
                store,
                &registry,
                path,
                &mut items,
                &mut created,
                &mut merged,
                &mut errors,
            )
            .await;
        }
    }

    Ok(ImportProgressResponse {
        items,
        created,
        merged,
        errors,
    })
}

#[allow(clippy::too_many_arguments)]
async fn import_single_file(
    store: &knowledge_storage::adapters::sqlite::SqliteStore,
    registry: &knowledge_plugin::registry::CapabilityRegistry,
    file_path: &std::path::Path,
    items: &mut Vec<ImportProgressItem>,
    created: &mut usize,
    _merged: &mut usize,
    errors: &mut Vec<ImportErrorResponse>,
) {
    let path_str = file_path.to_string_lossy().to_string();
    let ext = file_path
        .extension()
        .and_then(|e| e.to_str())
        .unwrap_or("")
        .to_lowercase();

    items.push(ImportProgressItem {
        path: path_str.clone(),
        status: "Processing".to_string(),
        action: None,
        error: None,
        entity_id: None,
    });

    let importer = if path_str.starts_with("http://") || path_str.starts_with("https://") {
        registry.get_importer("url").ok()
    } else {
        // Try by extension first
        let mut imp = registry.get_importer(&ext).ok();
        if imp.is_none() {
            // Try magic bytes detection
            if let Ok(fmt) =
                knowledge_import::features::importer::magic_bytes::detect_format(file_path)
            {
                let fmt_key = match fmt {
                    knowledge_import::features::importer::magic_bytes::DetectedFormat::Pdf => "pdf",
                    knowledge_import::features::importer::magic_bytes::DetectedFormat::Docx => {
                        "docx"
                    }
                    knowledge_import::features::importer::magic_bytes::DetectedFormat::Xlsx => {
                        "xlsx"
                    }
                    knowledge_import::features::importer::magic_bytes::DetectedFormat::Pptx => {
                        "pptx"
                    }
                    knowledge_import::features::importer::magic_bytes::DetectedFormat::Zip => "",
                    knowledge_import::features::importer::magic_bytes::DetectedFormat::Unknown => {
                        ""
                    }
                };
                if !fmt_key.is_empty() {
                    imp = registry.get_importer(fmt_key).ok();
                }
            }
        }
        imp
    };

    let import_result = match importer {
        Some(adapter) => adapter.import(file_path).await,
        None => Err(
            knowledge_import::features::importer::ImportError::UnsupportedFormat(format!(
                "no importer available for .{}",
                ext
            )),
        ),
    };

    let import_result = match import_result {
        Ok(r) => r,
        Err(e) => {
            errors.push(ImportErrorResponse {
                path: path_str.clone(),
                message: e.to_string(),
            });
            if let Some(item) = items.iter_mut().rev().find(|i| i.path == path_str) {
                item.status = "Failed".to_string();
                item.error = Some(e.to_string());
            }
            return;
        }
    };

    let source_str = file_path.to_string_lossy();

    let event = Event {
        id: uuid::Uuid::new_v4(),
        event_type: EventType::EntityCreated,
        entity_id: import_result.entity.id,
        timestamp: chrono::Utc::now(),
        data: serde_json::json!({"source": source_str}),
    };

    if let Err(e) = TransactionalWrite::save_entity_with_components(
        store,
        &import_result.entity,
        &import_result.components,
        &event,
    )
    .await
    {
        errors.push(ImportErrorResponse {
            path: path_str.clone(),
            message: format!("failed to save entity: {}", e),
        });
        if let Some(item) = items.iter_mut().rev().find(|i| i.path == path_str) {
            item.status = "Failed".to_string();
            item.error = Some(format!("failed to save entity: {}", e));
        }
        return;
    }

    let _ =
        SearchIndex::index_entity(store, &import_result.entity, &import_result.components).await;

    for cross_ref in &import_result.cross_references {
        let target_id: Option<uuid::Uuid> = match cross_ref {
            knowledge_import::features::importer::CrossReference::FileRef {
                target_path, ..
            } => ComponentRepository::find_by_component_data(
                store,
                "Provenance",
                "source",
                &target_path.to_string_lossy(),
            )
            .await
            .ok()
            .and_then(|v| v.first().map(|c| c.entity_id)),
            knowledge_import::features::importer::CrossReference::WikilinkRef {
                target_name,
                ..
            }
            | knowledge_import::features::importer::CrossReference::MentionRef { target_name } => {
                ComponentRepository::find_by_component_data(store, "Title", "title", target_name)
                    .await
                    .ok()
                    .and_then(|v| v.first().map(|c| c.entity_id))
            }
            knowledge_import::features::importer::CrossReference::SectionRef {
                target_path,
                ..
            } => ComponentRepository::find_by_component_data(
                store,
                "Provenance",
                "source",
                &target_path.to_string_lossy(),
            )
            .await
            .ok()
            .and_then(|v| v.first().map(|c| c.entity_id)),
            knowledge_import::features::importer::CrossReference::UrlRef { .. } => None,
        };

        if let Some(target_id) = target_id {
            let existing = RelationshipRepository::find_by_source_and_target(
                store,
                import_result.entity.id,
                target_id,
            )
            .await
            .ok()
            .flatten();
            if existing.is_none() {
                let rel = knowledge_core::features::relationship::Relationship::new(
                    import_result.entity.id,
                    target_id,
                    knowledge_core::features::relationship::RelationshipType::References,
                );
                let _ = RelationshipRepository::save(store, &rel).await;
            }
        }
    }

    *created += 1;
    if let Some(item) = items.iter_mut().rev().find(|i| i.path == path_str) {
        item.status = "Imported".to_string();
        item.action = Some("created".to_string());
        item.entity_id = Some(import_result.entity.id.to_string());
    }

    // Record import for undo support
    let _ = knowledge_import::features::importer::record_import(
        file_path,
        vec![import_result.entity.id],
        &ext,
    );
}

/// Import content from a URL.
#[tauri::command]
pub async fn import_url(
    state: State<'_, AppState>,
    url: String,
) -> Result<ImportProgressResponse, String> {
    let store = &*state.store;
    let url_importer = UrlImporter::new();

    let import_result = url_importer
        .import_url(&url)
        .await
        .map_err(|e| format!("URL import failed: {}", e))?;

    let event = Event {
        id: uuid::Uuid::new_v4(),
        event_type: EventType::EntityCreated,
        entity_id: import_result.entity.id,
        timestamp: chrono::Utc::now(),
        data: serde_json::json!({"source": url}),
    };

    TransactionalWrite::save_entity_with_components(
        store,
        &import_result.entity,
        &import_result.components,
        &event,
    )
    .await
    .map_err(|e| format!("failed to save entity: {}", e))?;

    let _ =
        SearchIndex::index_entity(store, &import_result.entity, &import_result.components).await;

    Ok(ImportProgressResponse {
        items: vec![ImportProgressItem {
            path: url.clone(),
            status: "Imported".to_string(),
            action: Some("created".to_string()),
            error: None,
            entity_id: Some(import_result.entity.id.to_string()),
        }],
        created: 1,
        merged: 0,
        errors: vec![],
    })
}

/// Import clipboard content (text or HTML).
#[tauri::command]
pub async fn import_clipboard(
    state: State<'_, AppState>,
    text: String,
    source_format: Option<String>,
) -> Result<ImportProgressResponse, String> {
    let store = &*state.store;
    let clipboard_importer = ClipboardImporter::new();

    let is_html = source_format.as_deref() == Some("html");

    let import_result = if is_html {
        clipboard_importer
            .import_html(&text, "clipboard")
            .map_err(|e| format!("clipboard import failed: {}", e))?
    } else {
        clipboard_importer
            .import_text(&text, "clipboard")
            .map_err(|e| format!("clipboard import failed: {}", e))?
    };

    let event = Event {
        id: uuid::Uuid::new_v4(),
        event_type: EventType::EntityCreated,
        entity_id: import_result.entity.id,
        timestamp: chrono::Utc::now(),
        data: serde_json::json!({"source": "clipboard"}),
    };

    TransactionalWrite::save_entity_with_components(
        store,
        &import_result.entity,
        &import_result.components,
        &event,
    )
    .await
    .map_err(|e| format!("failed to save entity: {}", e))?;

    let _ =
        SearchIndex::index_entity(store, &import_result.entity, &import_result.components).await;

    Ok(ImportProgressResponse {
        items: vec![ImportProgressItem {
            path: "clipboard".to_string(),
            status: "Imported".to_string(),
            action: Some("created".to_string()),
            error: None,
            entity_id: Some(import_result.entity.id.to_string()),
        }],
        created: 1,
        merged: 0,
        errors: vec![],
    })
}

/// Import from a database (SQLite/PostgreSQL/MySQL).
#[tauri::command]
pub async fn import_database(
    state: State<'_, AppState>,
    connection_string: String,
    tables: Vec<String>,
) -> Result<ImportProgressResponse, String> {
    let store = &*state.store;
    let source: Box<dyn DatabaseSource> = if connection_string.starts_with("sqlite")
        || connection_string.starts_with("file:")
        || !connection_string.contains("://")
    {
        let path = if connection_string.starts_with("sqlite:///") {
            let p = connection_string.trim_start_matches("sqlite:///");
            let p = p.trim_end_matches("?mode=rwc");
            std::path::PathBuf::from(p)
        } else {
            std::path::PathBuf::from(&connection_string)
        };
        Box::new(SqliteDatabaseSource::new(path))
    } else if connection_string.starts_with("postgres")
        || connection_string.starts_with("postgresql")
    {
        Box::new(PostgresDatabaseSource::new(connection_string.clone()))
    } else if connection_string.starts_with("mysql") {
        Box::new(MySqlDatabaseSource::new(connection_string.clone()))
    } else {
        Box::new(SqliteDatabaseSource::new(std::path::PathBuf::from(
            &connection_string,
        )))
    };

    let available_tables = source
        .list_tables()
        .await
        .map_err(|e| format!("failed to list tables: {}", e))?;

    let tables_to_import: Vec<_> = if tables.is_empty() {
        available_tables
    } else {
        available_tables
            .into_iter()
            .filter(|t| tables.contains(&t.name))
            .collect()
    };

    let mut created = 0usize;
    let mut items = Vec::new();
    let mut errors = Vec::new();

    for table in &tables_to_import {
        let preview = source
            .preview_table(&table.name, 100)
            .await
            .map_err(|e| format!("failed to preview table '{}': {}", table.name, e))?;

        for row in &preview.rows {
            let entity = knowledge_core::features::entity::Entity::new(
                knowledge_core::features::entity::EntityType::new("DatabaseRecord"),
            );

            let row_data: HashMap<String, serde_json::Value> = table
                .columns
                .iter()
                .zip(row.iter())
                .map(|(col, val)| {
                    let json_val = match val {
                        knowledge_core::ports::DbColumnValue::Text(s) => {
                            serde_json::Value::String(s.clone())
                        }
                        knowledge_core::ports::DbColumnValue::Integer(i) => {
                            serde_json::json!(i)
                        }
                        knowledge_core::ports::DbColumnValue::Float(f) => {
                            serde_json::json!(f)
                        }
                        knowledge_core::ports::DbColumnValue::Boolean(b) => {
                            serde_json::json!(b)
                        }
                        knowledge_core::ports::DbColumnValue::Null => serde_json::Value::Null,
                    };
                    (col.name.clone(), json_val)
                })
                .collect();

            let title = row_data
                .get(&table.columns[0].name)
                .and_then(|v| v.as_str())
                .unwrap_or("Untitled")
                .to_string();

            let components = vec![
                knowledge_core::features::component::Component::new(
                    entity.id,
                    knowledge_core::features::component::ComponentType::Title,
                    serde_json::json!(title),
                ),
                knowledge_core::features::component::Component::new(
                    entity.id,
                    knowledge_core::features::component::ComponentType::Content,
                    serde_json::json!(row_data),
                ),
                knowledge_core::features::component::Component::new(
                    entity.id,
                    knowledge_core::features::component::ComponentType::Provenance,
                    serde_json::json!({
                        "source": format!("{}:{}", connection_string, table.name),
                        "imported_at": chrono::Utc::now().to_rfc3339(),
                        "format": "database",
                    }),
                ),
            ];

            let event = Event {
                id: uuid::Uuid::new_v4(),
                event_type: EventType::EntityCreated,
                entity_id: entity.id,
                timestamp: chrono::Utc::now(),
                data: serde_json::json!({"source": format!("{}:{}", connection_string, table.name)}),
            };

            if let Err(e) = store
                .save_entity_with_components(&entity, &components, &event)
                .await
            {
                errors.push(ImportErrorResponse {
                    path: format!("{}:{}", connection_string, table.name),
                    message: e.to_string(),
                });
                continue;
            }

            created += 1;
            items.push(ImportProgressItem {
                path: format!("{}:{}", connection_string, table.name),
                status: "Imported".to_string(),
                action: Some("created".to_string()),
                error: None,
                entity_id: Some(entity.id.to_string()),
            });
        }
    }

    Ok(ImportProgressResponse {
        items,
        created,
        merged: 0,
        errors,
    })
}

/// Import files from a directory recursively.
#[tauri::command]
pub async fn import_file_recursive(
    state: State<'_, AppState>,
    path: String,
) -> Result<ImportProgressResponse, String> {
    let store = &*state.store;
    let registry = built_in_plugins();
    let dir_path = std::path::Path::new(&path);
    let dir_importer = DirectoryImporter::new(true);

    let files = dir_importer
        .list_files(dir_path)
        .map_err(|e| format!("failed to list directory: {}", e))?;

    let mut items = Vec::new();
    let mut created = 0usize;
    let mut merged = 0usize;
    let mut errors: Vec<ImportErrorResponse> = Vec::new();

    for file_path in &files {
        import_single_file(
            store,
            &registry,
            file_path,
            &mut items,
            &mut created,
            &mut merged,
            &mut errors,
        )
        .await;
    }

    Ok(ImportProgressResponse {
        items,
        created,
        merged,
        errors,
    })
}

/// Import an image file with OCR processing.
#[tauri::command]
pub async fn import_image(
    state: State<'_, AppState>,
    path: String,
) -> Result<ImportProgressResponse, String> {
    let store = &*state.store;
    let image_importer = knowledge_import::features::importer::ImageImporter::new();
    let file_path = std::path::Path::new(&path);

    let import_result = image_importer
        .import(file_path)
        .await
        .map_err(|e| format!("image import failed: {}", e))?;

    let event = Event {
        id: uuid::Uuid::new_v4(),
        event_type: EventType::EntityCreated,
        entity_id: import_result.entity.id,
        timestamp: chrono::Utc::now(),
        data: serde_json::json!({"source": path}),
    };

    TransactionalWrite::save_entity_with_components(
        store,
        &import_result.entity,
        &import_result.components,
        &event,
    )
    .await
    .map_err(|e| format!("failed to save entity: {}", e))?;

    Ok(ImportProgressResponse {
        items: vec![ImportProgressItem {
            path: path.clone(),
            status: "Imported".to_string(),
            action: Some("created".to_string()),
            error: None,
            entity_id: Some(import_result.entity.id.to_string()),
        }],
        created: 1,
        merged: 0,
        errors: vec![],
    })
}

/// Undo the last import operation.
#[tauri::command]
pub async fn undo_import(
    state: State<'_, AppState>,
    import_id: Option<String>,
) -> Result<UndoImportResponse, String> {
    let store = &*state.store;

    let record = if let Some(_id) = import_id {
        // Find specific import by ID (stub for now — uses last import)
        knowledge_import::features::importer::undo_last_import()
            .map_err(|e| format!("undo failed: {}", e))?
    } else {
        knowledge_import::features::importer::undo_last_import()
            .map_err(|e| format!("undo failed: {}", e))?
    };

    match record {
        Some(import_record) => {
            let mut removed = Vec::new();
            for entity_id in &import_record.entity_ids {
                let _ = knowledge_core::ports::EntityRepository::delete(store, *entity_id).await;
                let _ = SearchIndex::remove_entity(store, *entity_id).await;
                removed.push(entity_id.to_string());
            }
            Ok(UndoImportResponse {
                removed_entities: removed,
                import_id: import_record.id.to_string(),
            })
        }
        None => Ok(UndoImportResponse {
            removed_entities: vec![],
            import_id: String::new(),
        }),
    }
}

/// Preview a directory before importing (shows file count and format breakdown).
#[tauri::command]
pub async fn import_directory_preview(
    path: String,
    recursive: Option<bool>,
) -> Result<DirectoryPreviewResponse, String> {
    let dir_path = std::path::Path::new(&path);
    let is_recursive = recursive.unwrap_or(true);
    let dir_importer = DirectoryImporter::new(is_recursive);

    let files = dir_importer
        .list_files(dir_path)
        .map_err(|e| format!("failed to list directory: {}", e))?;

    let mut formats: std::collections::HashMap<String, usize> = std::collections::HashMap::new();
    let mut total_size: u64 = 0;

    for f in &files {
        let ext = f
            .extension()
            .and_then(|e| e.to_str())
            .unwrap_or("")
            .to_lowercase();
        *formats
            .entry(if ext.is_empty() {
                "unknown".into()
            } else {
                ext
            })
            .or_insert(0) += 1;
        if let Ok(meta) = std::fs::metadata(f) {
            total_size += meta.len();
        }
    }

    let file_list: Vec<String> = files
        .iter()
        .map(|f| f.to_string_lossy().to_string())
        .collect();

    Ok(DirectoryPreviewResponse {
        file_count: files.len(),
        total_size_bytes: total_size,
        formats,
        files: file_list,
    })
}

/// Preview structured data (CSV/JSON/XML/YAML) showing columns and sample rows.
#[tauri::command]
pub async fn import_structured_preview(
    path: String,
    format: String,
) -> Result<StructuredPreviewResponse, String> {
    let file_path = std::path::Path::new(&path);

    match format.to_lowercase().as_str() {
        "csv" => {
            let importer = knowledge_import::features::importer::CsvImporter::new();
            let preview = importer
                .preview(file_path, 10)
                .await
                .map_err(|e| format!("CSV preview failed: {}", e))?;

            let columns: Vec<ColumnSchemaResponse> = preview
                .columns
                .iter()
                .map(|c| ColumnSchemaResponse {
                    name: c.name.clone(),
                    data_type: c.data_type.clone(),
                    nullable: c.nullable,
                })
                .collect();

            let sample_rows: Vec<Vec<serde_json::Value>> = preview
                .sample_rows
                .iter()
                .map(|row| {
                    row.iter()
                        .map(|v| match v {
                            knowledge_core::ports::ColumnValue::Text(s) => {
                                serde_json::Value::String(s.clone())
                            }
                            knowledge_core::ports::ColumnValue::Integer(i) => {
                                serde_json::json!(i)
                            }
                            knowledge_core::ports::ColumnValue::Float(f) => {
                                serde_json::json!(f)
                            }
                            knowledge_core::ports::ColumnValue::Boolean(b) => {
                                serde_json::json!(b)
                            }
                            knowledge_core::ports::ColumnValue::Null => serde_json::Value::Null,
                        })
                        .collect()
                })
                .collect();

            Ok(StructuredPreviewResponse {
                columns,
                sample_rows,
                total_rows: preview.row_count,
                format: "csv".to_string(),
            })
        }
        "json" | "xml" | "yaml" | "yml" => {
            // Simple preview for other structured formats
            let content = std::fs::read_to_string(file_path)
                .map_err(|e| format!("failed to read file: {}", e))?;
            let lines: Vec<&str> = content.lines().take(10).collect();
            let sample_rows: Vec<Vec<serde_json::Value>> = lines
                .iter()
                .map(|l| vec![serde_json::Value::String(l.to_string())])
                .collect();

            Ok(StructuredPreviewResponse {
                columns: vec![ColumnSchemaResponse {
                    name: "content".to_string(),
                    data_type: "text".to_string(),
                    nullable: true,
                }],
                sample_rows,
                total_rows: content.lines().count() as u64,
                format: format.to_lowercase(),
            })
        }
        _ => Err(format!("unsupported structured format: {}", format)),
    }
}

/// Import structured data with column mapping.
#[tauri::command]
pub async fn import_structured(
    state: State<'_, AppState>,
    path: String,
    format: String,
    column_mapping: Option<String>,
) -> Result<ImportProgressResponse, String> {
    let store = &*state.store;
    let file_path = std::path::Path::new(&path);

    match format.to_lowercase().as_str() {
        "csv" => {
            let importer = knowledge_import::features::importer::CsvImporter::new();

            let mapping = match column_mapping {
                Some(json_str) => {
                    serde_json::from_str::<knowledge_core::ports::ColumnMapping>(&json_str)
                        .map_err(|e| format!("invalid column mapping JSON: {}", e))?
                }
                None => {
                    // Auto-detect: first column as title, rest as content
                    let preview = importer
                        .preview(file_path, 1)
                        .await
                        .map_err(|e| format!("preview failed: {}", e))?;
                    let mut field_mappings = std::collections::HashMap::new();
                    if let Some(first_col) = preview.columns.first() {
                        field_mappings.insert(
                            first_col.name.clone(),
                            knowledge_core::ports::FieldMapping::Title,
                        );
                    }
                    if let Some(second_col) = preview.columns.get(1) {
                        field_mappings.insert(
                            second_col.name.clone(),
                            knowledge_core::ports::FieldMapping::Content,
                        );
                    }
                    knowledge_core::ports::ColumnMapping {
                        field_mappings,
                        skip_columns: std::collections::HashSet::new(),
                        entity_type_override: None,
                    }
                }
            };

            let results = importer
                .import_with_mapping(file_path, &mapping)
                .await
                .map_err(|e| format!("CSV import failed: {}", e))?;

            let mut items = Vec::new();
            let mut created = 0usize;
            let mut errors = Vec::new();

            for import_result in &results {
                let event = Event {
                    id: uuid::Uuid::new_v4(),
                    event_type: EventType::EntityCreated,
                    entity_id: import_result.entity.id,
                    timestamp: chrono::Utc::now(),
                    data: serde_json::json!({"source": path}),
                };

                if let Err(e) = store
                    .save_entity_with_components(
                        &import_result.entity,
                        &import_result.components,
                        &event,
                    )
                    .await
                {
                    errors.push(ImportErrorResponse {
                        path: path.clone(),
                        message: e.to_string(),
                    });
                    continue;
                }

                created += 1;
                items.push(ImportProgressItem {
                    path: format!("{}:row{}", path, created),
                    status: "Imported".to_string(),
                    action: Some("created".to_string()),
                    error: None,
                    entity_id: Some(import_result.entity.id.to_string()),
                });
            }

            Ok(ImportProgressResponse {
                items,
                created,
                merged: 0,
                errors,
            })
        }
        _ => Err(format!(
            "import_structured only supports CSV currently, got: {}",
            format
        )),
    }
}
