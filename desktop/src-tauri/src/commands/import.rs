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
use std::time::Instant;
use tauri::State;
use uuid::Uuid;

use super::response::*;
use super::store::AppState;

/// Import one or more files into the knowledge base using the importer registry.
#[tauri::command]
pub async fn import_files(
    state: State<'_, AppState>,
    paths: Vec<String>,
) -> Result<ImportProgressResponse, String> {
    let correlation_id = Uuid::new_v4();
    let start_time = Instant::now();

    log::info!(
        "import.started: correlation_id={}, file_count={}, paths={:?}",
        correlation_id,
        paths.len(),
        paths
    );

    let store = &*state.store;
    let registry = built_in_plugins();
    let mut items = Vec::new();
    let mut created = 0usize;
    let mut merged = 0usize;
    let mut errors: Vec<ImportErrorResponse> = Vec::new();

    for path_str in &paths {
        let path = std::path::Path::new(path_str);
        log::debug!(
            "import.processing_path: correlation_id={}, path={}, is_dir={}",
            correlation_id,
            path_str,
            path.is_dir()
        );

        if path.is_dir() {
            log::info!(
                "import.directory.detected: correlation_id={}, path={}",
                correlation_id,
                path_str
            );
            let global_kosignore = state.data_dir.join(".kosignore");
            let gi = knowledge_import::features::importer::ignore_config::resolve_ignore(
                path,
                Some(global_kosignore.as_path()),
            );
            let dir_importer = DirectoryImporter::new(true).with_ignore(gi);
            let files = match dir_importer.list_files(path) {
                Ok(f) => {
                    log::info!(
                        "import.directory.listed: correlation_id={}, file_count={}",
                        correlation_id,
                        f.len()
                    );
                    f
                }
                Err(e) => {
                    log::error!(
                        "import.directory.list_failed: correlation_id={}, path={}, error={}",
                        correlation_id,
                        path_str,
                        e
                    );
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
                    correlation_id,
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
                correlation_id,
            )
            .await;
        }
    }

    let duration = start_time.elapsed();
    log::info!(
        "import.completed: correlation_id={}, created={}, merged={}, errors={}, duration_ms={}",
        correlation_id,
        created,
        merged,
        errors.len(),
        duration.as_millis()
    );

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
    correlation_id: Uuid,
) {
    let file_start_time = Instant::now();
    let path_str = file_path.to_string_lossy().to_string();
    let ext = file_path
        .extension()
        .and_then(|e| e.to_str())
        .unwrap_or("")
        .to_lowercase();

    log::info!(
        "import.file.started: correlation_id={}, path={}, extension={}",
        correlation_id,
        path_str,
        if ext.is_empty() { "none" } else { &ext }
    );

    items.push(ImportProgressItem {
        path: path_str.clone(),
        status: "Processing".to_string(),
        action: None,
        error: None,
        entity_id: None,
    });

    let importer = if path_str.starts_with("http://") || path_str.starts_with("https://") {
        log::debug!(
            "import.file.url_detected: correlation_id={}, path={}",
            correlation_id,
            path_str
        );
        registry.get_importer("url").ok()
    } else {
        // Try by extension first
        let mut imp = registry.get_importer(&ext).ok();
        if imp.is_none() {
            log::debug!(
                "import.file.extension_importer_not_found: correlation_id={}, extension={}, trying_magic_bytes",
                correlation_id,
                ext
            );
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
                    log::info!(
                        "import.file.magic_bytes_detected: correlation_id={}, format={}, path={}",
                        correlation_id,
                        fmt_key,
                        path_str
                    );
                    imp = registry.get_importer(fmt_key).ok();
                }
            }
        }
        imp
    };

    if importer.is_some() {
        log::info!(
            "import.file.importer_selected: correlation_id={}, path={}",
            correlation_id,
            path_str
        );
    } else {
        log::warn!(
            "import.file.no_importer_found: correlation_id={}, extension={}, path={}",
            correlation_id,
            ext,
            path_str
        );
    }

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
        Ok(r) => {
            log::info!(
                "import.file.parsed: correlation_id={}, path={}, entity_id={}",
                correlation_id,
                path_str,
                r.entity.id
            );
            r
        }
        Err(e) => {
            log::error!(
                "import.file.parse_failed: correlation_id={}, path={}, error={}, stage=parsing",
                correlation_id,
                path_str,
                e
            );
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

    log::debug!(
        "import.file.saving_entity: correlation_id={}, entity_id={}, components={}, cross_refs={}",
        correlation_id,
        import_result.entity.id,
        import_result.components.len(),
        import_result.cross_references.len()
    );

    if let Err(e) = TransactionalWrite::save_entity_with_components(
        store,
        &import_result.entity,
        &import_result.components,
        &event,
    )
    .await
    {
        log::error!(
            "import.file.save_failed: correlation_id={}, path={}, entity_id={}, error={}, stage=saving",
            correlation_id,
            path_str,
            import_result.entity.id,
            e
        );
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

    log::debug!(
        "import.file.entity_saved: correlation_id={}, entity_id={}, path={}",
        correlation_id,
        import_result.entity.id,
        path_str
    );

    let _ =
        SearchIndex::index_entity(store, &import_result.entity, &import_result.components).await;

    log::debug!(
        "import.file.entity_indexed: correlation_id={}, entity_id={}",
        correlation_id,
        import_result.entity.id
    );

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
                log::debug!(
                    "import.file.cross_reference.created: correlation_id={}, source={}, target={}",
                    correlation_id,
                    import_result.entity.id,
                    target_id
                );
            } else {
                log::trace!(
                    "import.file.cross_reference.exists: correlation_id={}, source={}, target={}",
                    correlation_id,
                    import_result.entity.id,
                    target_id
                );
            }
        } else {
            log::trace!(
                "import.file.cross_reference.target_not_found: correlation_id={}, source={}, ref_type={}",
                correlation_id,
                import_result.entity.id,
                match cross_ref {
                    knowledge_import::features::importer::CrossReference::FileRef { .. } => "FileRef",
                    knowledge_import::features::importer::CrossReference::WikilinkRef { .. } => "WikilinkRef",
                    knowledge_import::features::importer::CrossReference::MentionRef { .. } => "MentionRef",
                    knowledge_import::features::importer::CrossReference::SectionRef { .. } => "SectionRef",
                    knowledge_import::features::importer::CrossReference::UrlRef { .. } => "UrlRef",
                }
            );
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

    let file_duration = file_start_time.elapsed();
    log::info!(
        "import.file.completed: correlation_id={}, path={}, entity_id={}, duration_ms={}",
        correlation_id,
        path_str,
        import_result.entity.id,
        file_duration.as_millis()
    );
}

/// Import content from a URL.
#[tauri::command]
pub async fn import_url(
    state: State<'_, AppState>,
    url: String,
) -> Result<ImportProgressResponse, String> {
    let correlation_id = Uuid::new_v4();
    let start_time = Instant::now();

    log::info!(
        "import.url.started: correlation_id={}, url={}",
        correlation_id,
        url
    );

    let store = &*state.store;
    let url_importer = UrlImporter::new();

    let import_result = url_importer.import_url(&url).await.map_err(|e| {
        log::error!(
            "import.url.fetch_failed: correlation_id={}, url={}, error={}",
            correlation_id,
            url,
            e
        );
        format!("URL import failed: {}", e)
    })?;

    log::info!(
        "import.url.fetched: correlation_id={}, url={}, entity_id={}",
        correlation_id,
        url,
        import_result.entity.id
    );

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
    .map_err(|e| {
        log::error!(
            "import.url.save_failed: correlation_id={}, url={}, entity_id={}, error={}",
            correlation_id,
            url,
            import_result.entity.id,
            e
        );
        format!("failed to save entity: {}", e)
    })?;

    let _ =
        SearchIndex::index_entity(store, &import_result.entity, &import_result.components).await;

    let duration = start_time.elapsed();
    log::info!(
        "import.url.completed: correlation_id={}, url={}, entity_id={}, duration_ms={}",
        correlation_id,
        url,
        import_result.entity.id,
        duration.as_millis()
    );

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
    let correlation_id = Uuid::new_v4();
    let start_time = Instant::now();
    let is_html = source_format.as_deref() == Some("html");

    log::info!(
        "import.clipboard.started: correlation_id={}, format={}, length={}",
        correlation_id,
        if is_html { "html" } else { "text" },
        text.len()
    );

    let store = &*state.store;
    let clipboard_importer = ClipboardImporter::new();

    let import_result = if is_html {
        clipboard_importer
            .import_html(&text, "clipboard")
            .map_err(|e| {
                log::error!(
                    "import.clipboard.parse_failed: correlation_id={}, format=html, error={}",
                    correlation_id,
                    e
                );
                format!("clipboard import failed: {}", e)
            })?
    } else {
        clipboard_importer
            .import_text(&text, "clipboard")
            .map_err(|e| {
                log::error!(
                    "import.clipboard.parse_failed: correlation_id={}, format=text, error={}",
                    correlation_id,
                    e
                );
                format!("clipboard import failed: {}", e)
            })?
    };

    log::info!(
        "import.clipboard.parsed: correlation_id={}, entity_id={}",
        correlation_id,
        import_result.entity.id
    );

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
    .map_err(|e| {
        log::error!(
            "import.clipboard.save_failed: correlation_id={}, entity_id={}, error={}",
            correlation_id,
            import_result.entity.id,
            e
        );
        format!("failed to save entity: {}", e)
    })?;

    let _ =
        SearchIndex::index_entity(store, &import_result.entity, &import_result.components).await;

    let duration = start_time.elapsed();
    log::info!(
        "import.clipboard.completed: correlation_id={}, entity_id={}, duration_ms={}",
        correlation_id,
        import_result.entity.id,
        duration.as_millis()
    );

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
    let correlation_id = Uuid::new_v4();
    let start_time = Instant::now();

    log::info!(
        "import.database.started: correlation_id={}, connection={}, table_filter={:?}",
        correlation_id,
        connection_string,
        tables
    );

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
        log::debug!(
            "import.database.type_detected: correlation_id={}, type=sqlite, path={:?}",
            correlation_id,
            path
        );
        Box::new(SqliteDatabaseSource::new(path))
    } else if connection_string.starts_with("postgres")
        || connection_string.starts_with("postgresql")
    {
        log::debug!(
            "import.database.type_detected: correlation_id={}, type=postgres",
            correlation_id
        );
        Box::new(PostgresDatabaseSource::new(connection_string.clone()))
    } else if connection_string.starts_with("mysql") {
        log::debug!(
            "import.database.type_detected: correlation_id={}, type=mysql",
            correlation_id
        );
        Box::new(MySqlDatabaseSource::new(connection_string.clone()))
    } else {
        log::debug!(
            "import.database.type_defaulted: correlation_id={}, type=sqlite",
            correlation_id
        );
        Box::new(SqliteDatabaseSource::new(std::path::PathBuf::from(
            &connection_string,
        )))
    };

    let available_tables = source.list_tables().await.map_err(|e| {
        log::error!(
            "import.database.list_tables_failed: correlation_id={}, error={}",
            correlation_id,
            e
        );
        format!("failed to list tables: {}", e)
    })?;

    log::info!(
        "import.database.tables_listed: correlation_id={}, available_count={}",
        correlation_id,
        available_tables.len()
    );

    let tables_to_import: Vec<_> = if tables.is_empty() {
        log::info!(
            "import.database.importing_all_tables: correlation_id={}, count={}",
            correlation_id,
            available_tables.len()
        );
        available_tables
    } else {
        let filtered: Vec<_> = available_tables
            .into_iter()
            .filter(|t| tables.contains(&t.name))
            .collect();
        log::info!(
            "import.database.tables_filtered: correlation_id={}, requested={:?}, matched={}",
            correlation_id,
            tables,
            filtered.len()
        );
        filtered
    };

    let mut created = 0usize;
    let mut items = Vec::new();
    let mut errors = Vec::new();

    for table in &tables_to_import {
        log::debug!(
            "import.database.table.processing: correlation_id={}, table={}, columns={}",
            correlation_id,
            table.name,
            table.columns.len()
        );

        let preview = source.preview_table(&table.name, 100).await.map_err(|e| {
            log::error!(
                "import.database.table.preview_failed: correlation_id={}, table={}, error={}",
                correlation_id,
                table.name,
                e
            );
            format!("failed to preview table '{}': {}", table.name, e)
        })?;

        log::info!(
            "import.database.table.previewed: correlation_id={}, table={}, rows={}",
            correlation_id,
            table.name,
            preview.rows.len()
        );

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
                log::error!(
                    "import.database.row.save_failed: correlation_id={}, table={}, entity_id={}, error={}",
                    correlation_id,
                    table.name,
                    entity.id,
                    e
                );
                errors.push(ImportErrorResponse {
                    path: format!("{}:{}", connection_string, table.name),
                    message: e.to_string(),
                });
                continue;
            }

            log::trace!(
                "import.database.row.saved: correlation_id={}, table={}, entity_id={}",
                correlation_id,
                table.name,
                entity.id
            );

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

    let duration = start_time.elapsed();
    log::info!(
        "import.database.completed: correlation_id={}, tables={}, created={}, duration_ms={}",
        correlation_id,
        tables_to_import.len(),
        created,
        duration.as_millis()
    );

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
    let correlation_id = Uuid::new_v4();
    let start_time = Instant::now();

    log::info!(
        "import.recursive.started: correlation_id={}, path={}",
        correlation_id,
        path
    );

    let store = &*state.store;
    let registry = built_in_plugins();
    let dir_path = std::path::Path::new(&path);
    let global_kosignore = state.data_dir.join(".kosignore");
    let gi = knowledge_import::features::importer::ignore_config::resolve_ignore(
        dir_path,
        Some(global_kosignore.as_path()),
    );
    let dir_importer = DirectoryImporter::new(true).with_ignore(gi);

    let files = dir_importer.list_files(dir_path).map_err(|e| {
        log::error!(
            "import.recursive.list_failed: correlation_id={}, path={}, error={}",
            correlation_id,
            path,
            e
        );
        format!("failed to list directory: {}", e)
    })?;

    log::info!(
        "import.recursive.files_listed: correlation_id={}, path={}, file_count={}",
        correlation_id,
        path,
        files.len()
    );

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
            correlation_id,
        )
        .await;
    }

    let duration = start_time.elapsed();
    log::info!(
        "import.recursive.completed: correlation_id={}, path={}, created={}, errors={}, duration_ms={}",
        correlation_id,
        path,
        created,
        errors.len(),
        duration.as_millis()
    );

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
    let correlation_id = Uuid::new_v4();
    let start_time = Instant::now();

    log::info!(
        "import.image.started: correlation_id={}, path={}",
        correlation_id,
        path
    );

    let store = &*state.store;
    let image_importer = knowledge_import::features::importer::ImageImporter::new();
    let file_path = std::path::Path::new(&path);

    let import_result = image_importer.import(file_path).await.map_err(|e| {
        log::error!(
            "import.image.ocr_failed: correlation_id={}, path={}, error={}",
            correlation_id,
            path,
            e
        );
        format!("image import failed: {}", e)
    })?;

    log::info!(
        "import.image.ocr_completed: correlation_id={}, path={}, entity_id={}",
        correlation_id,
        path,
        import_result.entity.id
    );

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
    .map_err(|e| {
        log::error!(
            "import.image.save_failed: correlation_id={}, path={}, entity_id={}, error={}",
            correlation_id,
            path,
            import_result.entity.id,
            e
        );
        format!("failed to save entity: {}", e)
    })?;

    let duration = start_time.elapsed();
    log::info!(
        "import.image.completed: correlation_id={}, path={}, entity_id={}, duration_ms={}",
        correlation_id,
        path,
        import_result.entity.id,
        duration.as_millis()
    );

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
    let correlation_id = Uuid::new_v4();

    log::info!(
        "import.undo.started: correlation_id={}, import_id={:?}",
        correlation_id,
        import_id
    );

    let store = &*state.store;

    let record = if let Some(_id) = import_id {
        // Find specific import by ID (stub for now — uses last import)
        knowledge_import::features::importer::undo_last_import().map_err(|e| {
            log::error!(
                "import.undo.failed: correlation_id={}, error={}",
                correlation_id,
                e
            );
            format!("undo failed: {}", e)
        })?
    } else {
        knowledge_import::features::importer::undo_last_import().map_err(|e| {
            log::error!(
                "import.undo.failed: correlation_id={}, error={}",
                correlation_id,
                e
            );
            format!("undo failed: {}", e)
        })?
    };

    match record {
        Some(import_record) => {
            let mut removed = Vec::new();
            log::info!(
                "import.undo.processing: correlation_id={}, entity_count={}",
                correlation_id,
                import_record.entity_ids.len()
            );
            for entity_id in &import_record.entity_ids {
                let _ = knowledge_core::ports::EntityRepository::delete(store, *entity_id).await;
                let _ = SearchIndex::remove_entity(store, *entity_id).await;
                removed.push(entity_id.to_string());
            }
            log::info!(
                "import.undo.completed: correlation_id={}, removed_count={}",
                correlation_id,
                removed.len()
            );
            Ok(UndoImportResponse {
                removed_entities: removed,
                import_id: import_record.id.to_string(),
            })
        }
        None => {
            log::info!("import.undo.no_record: correlation_id={}", correlation_id);
            Ok(UndoImportResponse {
                removed_entities: vec![],
                import_id: String::new(),
            })
        }
    }
}

/// Preview a directory before importing (shows file count and format breakdown).
#[tauri::command]
pub async fn import_directory_preview(
    state: State<'_, AppState>,
    path: String,
    recursive: Option<bool>,
) -> Result<DirectoryPreviewResponse, String> {
    let start_time = Instant::now();
    let is_recursive = recursive.unwrap_or(true);

    log::info!(
        "import.preview.directory.started: path={}, recursive={}",
        path,
        is_recursive
    );

    let dir_path = std::path::Path::new(&path);
    let global_kosignore = state.data_dir.join(".kosignore");
    let gi = knowledge_import::features::importer::ignore_config::resolve_ignore(
        dir_path,
        Some(global_kosignore.as_path()),
    );
    let dir_importer = DirectoryImporter::new(is_recursive).with_ignore(gi);

    let files = dir_importer.list_files(dir_path).map_err(|e| {
        log::error!(
            "import.preview.directory.list_failed: path={}, error={}",
            path,
            e
        );
        format!("failed to list directory: {}", e)
    })?;

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

    let duration = start_time.elapsed();
    log::info!(
        "import.preview.directory.completed: path={}, file_count={}, total_size={}, formats={}, duration_ms={}",
        path,
        files.len(),
        total_size,
        formats.len(),
        duration.as_millis()
    );

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
    let start_time = Instant::now();

    log::info!(
        "import.preview.structured.started: path={}, format={}",
        path,
        format
    );

    let file_path = std::path::Path::new(&path);

    let result = match format.to_lowercase().as_str() {
        "csv" => {
            let importer = knowledge_import::features::importer::CsvImporter::new();
            let preview = importer.preview(file_path, 10).await.map_err(|e| {
                log::error!(
                    "import.preview.structured.csv_failed: path={}, error={}",
                    path,
                    e
                );
                format!("CSV preview failed: {}", e)
            })?;

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
            let content = std::fs::read_to_string(file_path).map_err(|e| {
                log::error!(
                    "import.preview.structured.read_failed: path={}, error={}",
                    path,
                    e
                );
                format!("failed to read file: {}", e)
            })?;
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
        _ => {
            log::error!(
                "import.preview.structured.unsupported_format: path={}, format={}",
                path,
                format
            );
            Err(format!("unsupported structured format: {}", format))
        }
    };

    let duration = start_time.elapsed();
    log::info!(
        "import.preview.structured.completed: path={}, format={}, duration_ms={}",
        path,
        format,
        duration.as_millis()
    );

    result
}

/// Import structured data with column mapping.
#[tauri::command]
pub async fn import_structured(
    state: State<'_, AppState>,
    path: String,
    format: String,
    column_mapping: Option<String>,
) -> Result<ImportProgressResponse, String> {
    let correlation_id = Uuid::new_v4();
    let start_time = Instant::now();

    log::info!(
        "import.structured.started: correlation_id={}, path={}, format={}, has_mapping={}",
        correlation_id,
        path,
        format,
        column_mapping.is_some()
    );

    let store = &*state.store;
    let file_path = std::path::Path::new(&path);

    let result = match format.to_lowercase().as_str() {
        "csv" => {
            let importer = knowledge_import::features::importer::CsvImporter::new();

            let mapping = match column_mapping {
                Some(json_str) => {
                    serde_json::from_str::<knowledge_core::ports::ColumnMapping>(&json_str)
                        .map_err(|e| {
                            log::error!(
                                "import.structured.mapping_invalid: correlation_id={}, error={}",
                                correlation_id,
                                e
                            );
                            format!("invalid column mapping JSON: {}", e)
                        })?
                }
                None => {
                    // Auto-detect: first column as title, rest as content
                    let preview = importer.preview(file_path, 1).await.map_err(|e| {
                        log::error!(
                            "import.structured.preview_failed: correlation_id={}, error={}",
                            correlation_id,
                            e
                        );
                        format!("preview failed: {}", e)
                    })?;
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
                .map_err(|e| {
                    log::error!(
                        "import.structured.import_failed: correlation_id={}, error={}",
                        correlation_id,
                        e
                    );
                    format!("CSV import failed: {}", e)
                })?;

            log::info!(
                "import.structured.parsed: correlation_id={}, row_count={}",
                correlation_id,
                results.len()
            );

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
                    log::error!(
                        "import.structured.row.save_failed: correlation_id={}, entity_id={}, error={}",
                        correlation_id,
                        import_result.entity.id,
                        e
                    );
                    errors.push(ImportErrorResponse {
                        path: path.clone(),
                        message: e.to_string(),
                    });
                    continue;
                }

                log::trace!(
                    "import.structured.row.saved: correlation_id={}, entity_id={}",
                    correlation_id,
                    import_result.entity.id
                );

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
        _ => {
            log::error!(
                "import.structured.unsupported_format: correlation_id={}, format={}",
                correlation_id,
                format
            );
            Err(format!(
                "import_structured only supports CSV currently, got: {}",
                format
            ))
        }
    };

    let duration = start_time.elapsed();
    match &result {
        Ok(response) => {
            log::info!(
                "import.structured.completed: correlation_id={}, created={}, errors={}, duration_ms={}",
                correlation_id,
                response.created,
                response.errors.len(),
                duration.as_millis()
            );
        }
        Err(e) => {
            log::error!(
                "import.structured.failed: correlation_id={}, error={}, duration_ms={}",
                correlation_id,
                e,
                duration.as_millis()
            );
        }
    }

    result
}
