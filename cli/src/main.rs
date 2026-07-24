use clap::{Parser, Subcommand};
use knowledge_core::ports::{
    ComponentRepository, EntityRepository, EntityResolver, RelationshipRepository, SearchIndex,
    SearchQuery, EventLog, TransactionalWrite,
};
use knowledge_core::features::component::ComponentType;
use knowledge_storage::adapters::sqlite::SqliteStore;
use indicatif::{ProgressBar, ProgressStyle};
use std::path::PathBuf;
use std::sync::Arc;

#[derive(Parser)]
#[command(name = "kos", about = "Knowledge OS CLI")]
struct Cli {
    /// Path to SQLite database file
    #[arg(short, long, default_value = "knowledge.db", global = true)]
    db: String,

    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    /// Import a Markdown file or directory
    Import {
        /// Path to file or directory
        path: PathBuf,
        /// Output progress as JSON lines (machine-readable)
        #[arg(long)]
        json: bool,
    },
    /// Search entities
    Search {
        /// Search query
        query: String,
        /// Filter by entity type
        #[arg(short = 'T', long)]
        r#type: Option<String>,
        /// Filter by tag
        #[arg(short, long)]
        tag: Option<String>,
    },
    /// Get entity details
    Get {
        /// Entity ID
        id: String,
    },
    /// List entities
    List {
        /// Filter by entity type
        #[arg(short = 'T', long)]
        r#type: Option<String>,
    },
    /// Archive an entity
    Archive {
        /// Entity ID
        id: String,
    },
    /// Restore an entity
    Restore {
        /// Entity ID
        id: String,
    },
    /// Rebuild the search index from canonical data
    RebuildIndex,
    /// Manage entity resolution
    Resolution {
        #[command(subcommand)]
        action: ResolutionCommands,
    },
}

#[derive(Subcommand)]
enum ResolutionCommands {
    /// Show merge history
    Log {
        /// Entity ID to show merge history for (optional, shows all if not provided)
        #[arg(long)]
        entity: Option<String>,
    },
    /// Undo a merge by its merge ID
    Undo {
        /// Merge audit entry ID to undo
        merge_id: String,
    },
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let cli = Cli::parse();
    let store = Arc::new(SqliteStore::new(&cli.db)?);

    match cli.command {
        Commands::Import { path, json } => cmd_import(store, path, json).await,
        Commands::Search { query, r#type, tag } => {
            cmd_search(store, &query, r#type.as_deref(), tag.as_deref()).await
        }
        Commands::Get { id } => cmd_get(store, &id).await,
        Commands::List { r#type } => cmd_list(store, r#type.as_deref()).await,
        Commands::Archive { id } => cmd_archive(store, &id).await,
        Commands::Restore { id } => cmd_restore(store, &id).await,
        Commands::RebuildIndex => cmd_rebuild_index(store).await,
        Commands::Resolution { action } => match action {
            ResolutionCommands::Log { entity } => cmd_resolution_log(store, entity.as_deref()).await,
            ResolutionCommands::Undo { merge_id } => cmd_resolution_undo(store, &merge_id).await,
        },
    }
}

async fn cmd_import(
    store: Arc<SqliteStore>,
    path: PathBuf,
    json_mode: bool,
) -> Result<(), Box<dyn std::error::Error>> {
    let markdown_importer = knowledge_import::features::importer::MarkdownImporter::new();
    let pdf_importer = knowledge_import::features::importer::PdfImporter::new();
    let url_importer = knowledge_import::features::importer::UrlImporter::new();

    // Check if path is a URL
    let path_str = path.to_string_lossy();
    if path_str.starts_with("http://") || path_str.starts_with("https://") {
        let pb = ProgressBar::new(1);
        pb.set_style(
            ProgressStyle::default_bar()
                .template("[{pos}/{len}] {bar:40} {msg}")
                .unwrap()
                .progress_chars("=>-"),
        );
        pb.set_message(path_str.to_string());

        match import_with_adapter(&store, &url_importer, &path).await {
            Ok(_) => {
                println!("\nImported URL: {}", path_str);
                pb.inc(1);
            }
            Err(e) => {
                eprintln!("\nERROR: {}: {}", path_str, e);
            }
        }
        pb.finish_and_clear();
        return Ok(());
    }

    let mut files = Vec::new();
    if path.is_dir() {
        for entry in std::fs::read_dir(&path)? {
            let entry = entry?;
            let file_path = entry.path();
            let ext = file_path.extension().and_then(|e| e.to_str()).unwrap_or("");
            if ext.eq_ignore_ascii_case("md") || ext.eq_ignore_ascii_case("pdf") {
                files.push(file_path);
            }
        }
    } else {
        files.push(path);
    }

    let total = files.len();
    let mut created = 0;
    let mut merged = 0;
    let mut errors: Vec<String> = Vec::new();

    let pb = if json_mode {
        None
    } else {
        let pb = ProgressBar::new(total as u64);
        pb.set_style(
            ProgressStyle::default_bar()
                .template("[{pos}/{len}] {bar:40} {msg}")
                .unwrap()
                .progress_chars("=>-"),
        );
        Some(pb)
    };

    for (i, file_path) in files.iter().enumerate() {
        let fname = file_path.file_name().unwrap_or_default().to_string_lossy();
        if let Some(ref pb) = pb {
            pb.set_message(fname.to_string());
        }

        // Select adapter based on file extension
        let ext = file_path.extension().and_then(|e| e.to_str()).unwrap_or("");
        let action = if ext.eq_ignore_ascii_case("pdf") {
            import_with_adapter(&store, &pdf_importer, file_path).await
        } else {
            import_with_adapter(&store, &markdown_importer, file_path).await
        };

        match action {
            Ok(action) => {
                let action_str = match action {
                    ImportAction::Created => {
                        created += 1;
                        "created"
                    }
                    ImportAction::Merged => {
                        merged += 1;
                        "merged"
                    }
                };

                if json_mode {
                    println!("{}", serde_json::json!({
                        "event": "import",
                        "file": file_path.to_string_lossy(),
                        "action": action_str,
                        "position": i + 1,
                        "total": total,
                    }));
                } else if let Some(ref pb) = pb {
                    pb.inc(1);
                }
            }
            Err(e) => {
                let err_msg = format!("{}: {}", file_path.display(), e);
                errors.push(err_msg.clone());

                if json_mode {
                    println!("{}", serde_json::json!({
                        "event": "error",
                        "file": file_path.to_string_lossy(),
                        "error": err_msg,
                        "position": i + 1,
                        "total": total,
                    }));
                } else {
                    eprintln!("\nERROR: {}", err_msg);
                    if let Some(ref pb) = pb {
                        pb.inc(1);
                    }
                }
            }
        }
    }

    if let Some(ref pb) = pb {
        pb.finish_and_clear();
    }

    if json_mode {
        println!("{}", serde_json::json!({
            "event": "summary",
            "total": total,
            "created": created,
            "merged": merged,
            "errors": errors.len(),
        }));
    } else {
        println!("\n--- Import Summary ---");
        println!("Total files: {}", total);
        println!("Created: {}", created);
        println!("Duplicates resolved: {}", merged);
        if !errors.is_empty() {
            println!("Errors: {}", errors.len());
            for err in &errors {
                eprintln!("  {}", err);
            }
        }
    }

    Ok(())
}

enum ImportAction {
    Created,
    Merged,
}

async fn import_with_adapter(
    store: &SqliteStore,
    importer: &impl knowledge_import::features::importer::ImportAdapter,
    path: &std::path::Path,
) -> Result<ImportAction, Box<dyn std::error::Error>> {
    let result = importer.import(path).await?;

    let title = result
        .components
        .iter()
        .find(|c| c.component_type == ComponentType::Title)
        .and_then(|c| c.data.as_str().map(String::from))
        .unwrap_or_default();

    let content = result
        .components
        .iter()
        .find(|c| c.component_type == ComponentType::Content)
        .and_then(|c| c.data.as_str().map(String::from));

    // Use fuzzy resolution to find matching entities
    let candidates = EntityResolver::find_candidates(
        store,
        &result.entity,
        &title,
        content.as_deref(),
    )
    .await?;

    // Find the best candidate above threshold (default 0.95)
    // PONYTAIL: Hard-coded threshold, no per-type override. Ceiling: 0.95 works for most types
    // but may be too aggressive for Person (names vary) or too lax for Concept (precise terms).
    // Upgrade: KOS_RESOLUTION_THRESHOLD env var, then kos.toml config when someone needs per-type tuning.
    let threshold = 0.95;
    let best_candidate = candidates
        .into_iter()
        .find(|c| c.confidence >= threshold);

    let (entity, action) = if let Some(candidate) = best_candidate {
        // Merge into existing entity
        let mut existing = EntityRepository::get(store, candidate.entity_id)
            .await?
            .ok_or("Candidate entity not found")?;

        // Get existing entity's title for audit log
        let existing_components = ComponentRepository::get(store, existing.id).await?;
        let existing_title = existing_components
            .iter()
            .find(|c| c.component_type == ComponentType::Title)
            .and_then(|c| c.data.as_str().map(String::from))
            .unwrap_or_default();

        // Snapshot target state before merge for undo
        let existing_relationships = RelationshipRepository::by_source(store, existing.id).await?;

        // Snapshot source components before they're moved
        let source_components_snapshot: Vec<_> = result.components.iter().map(|c| serde_json::json!({
            "id": c.id.to_string(),
            "component_type": serde_json::to_string(&c.component_type).unwrap(),
            "data": c.data,
            "created_at": c.created_at.to_rfc3339(),
            "version": c.version,
        })).collect();

        existing.touch();

        let mut components = result.components;
        for comp in &mut components {
            comp.entity_id = existing.id;
        }

        let event = knowledge_core::ports::Event {
            id: uuid::Uuid::new_v4(),
            event_type: knowledge_core::ports::EventType::EntityUpdated,
            entity_id: existing.id,
            timestamp: chrono::Utc::now(),
            data: serde_json::json!({
                "source": path.to_string_lossy(),
                "resolution": {
                    "confidence": candidate.confidence,
                    "reason": candidate.reason,
                }
            }),
        };

        store
            .update_entity_with_components(&existing, &components, &event)
            .await?;

        // Log the merge decision to the audit trail
        let audit_entry = knowledge_core::ports::MergeAuditEntry {
            id: uuid::Uuid::new_v4(),
            source_id: result.entity.id,
            source_title: title.clone(),
            target_id: existing.id,
            target_title: existing_title.clone(),
            strategy: candidate.reason.clone(),
            confidence: candidate.confidence,
            timestamp: chrono::Utc::now(),
            reason: candidate.reason.clone(),
            snapshot: Some(serde_json::json!({
                "source": {
                    "entity_type": result.entity.entity_type.as_str(),
                    "is_active": result.entity.is_active,
                    "created_at": result.entity.created_at.to_rfc3339(),
                    "updated_at": result.entity.updated_at.to_rfc3339(),
                    "version": result.entity.version,
                    "components": source_components_snapshot,
                },
                "target": {
                    "entity_type": existing.entity_type.as_str(),
                    "is_active": existing.is_active,
                    "created_at": existing.created_at.to_rfc3339(),
                    "updated_at": existing.updated_at.to_rfc3339(),
                    "version": existing.version,
                    "components": existing_components.iter().map(|c| serde_json::json!({
                        "id": c.id.to_string(),
                        "component_type": serde_json::to_string(&c.component_type).unwrap(),
                        "data": c.data,
                        "created_at": c.created_at.to_rfc3339(),
                        "version": c.version,
                    })).collect::<Vec<_>>(),
                    "relationships": existing_relationships.iter().map(|r| serde_json::json!({
                        "id": r.id.to_string(),
                        "target_id": r.target_id.to_string(),
                        "relationship_type": serde_json::to_string(&r.relationship_type).unwrap(),
                        "is_active": r.is_active,
                        "created_at": r.created_at.to_rfc3339(),
                    })).collect::<Vec<_>>(),
                },
            }).to_string()),
        };
        EntityResolver::log_merge(store, &audit_entry).await?;

        println!(
            "  Merged into existing entity {} (confidence: {:.2}, reason: {})",
            existing.id, candidate.confidence, candidate.reason
        );

        (existing, ImportAction::Merged)
    } else {
        // No match found — create new entity
        let event = knowledge_core::ports::Event {
            id: uuid::Uuid::new_v4(),
            event_type: knowledge_core::ports::EventType::EntityCreated,
            entity_id: result.entity.id,
            timestamp: chrono::Utc::now(),
            data: serde_json::json!({"source": path.to_string_lossy()}),
        };

        store
            .save_entity_with_components(&result.entity, &result.components, &event)
            .await?;

        (result.entity, ImportAction::Created)
    };

    // Index for search
    let components = ComponentRepository::get(store, entity.id).await?;
    SearchIndex::index_entity(store, &entity, &components).await?;

    // Create cross-reference relationships using efficient lookup
    for cross_ref in &result.cross_references {
        let target_id = match cross_ref {
            knowledge_import::features::importer::CrossReference::FileRef { target_path, .. } => {
                // Look up target entity by Provenance source path
                let target_path_str = target_path.to_string_lossy();
                let matching_components = ComponentRepository::find_by_component_data(
                    store,
                    "Provenance",
                    "source",
                    &target_path_str,
                )
                .await?;

                matching_components.first().map(|c| c.entity_id)
            }
            knowledge_import::features::importer::CrossReference::WikilinkRef { target_name, .. } |
            knowledge_import::features::importer::CrossReference::MentionRef { target_name } => {
                // Look up target entity by Title component
                let matching_components = ComponentRepository::find_by_component_data(
                    store,
                    "Title",
                    "title",
                    target_name,
                )
                .await?;

                matching_components.first().map(|c| c.entity_id)
            }
            knowledge_import::features::importer::CrossReference::SectionRef { target_path, .. } => {
                // Look up target entity by Provenance source path, store section as metadata
                let target_path_str = target_path.to_string_lossy();
                let matching_components = ComponentRepository::find_by_component_data(
                    store,
                    "Provenance",
                    "source",
                    &target_path_str,
                )
                .await?;

                matching_components.first().map(|c| c.entity_id)
            }
            knowledge_import::features::importer::CrossReference::UrlRef { .. } => {
                // For URL references, we store the URL in the relationship metadata
                // No target entity lookup needed - this is an external reference
                None
            }
        };

        // For URL references, create a relationship with URL metadata but no target entity
        if let knowledge_import::features::importer::CrossReference::UrlRef { url, link_text } = cross_ref {
            let rel = knowledge_core::features::relationship::Relationship::new(
                entity.id,
                entity.id, // Self-reference for external URLs
                knowledge_core::features::relationship::RelationshipType::References,
            );
            RelationshipRepository::save(store, &rel).await?;

            let event = knowledge_core::ports::Event {
                id: uuid::Uuid::new_v4(),
                event_type: knowledge_core::ports::EventType::RelationshipCreated,
                entity_id: entity.id,
                timestamp: chrono::Utc::now(),
                data: serde_json::json!({
                    "type": "References",
                    "url": url,
                    "link_text": link_text,
                    "external": true
                }),
            };
            EventLog::append(store, &event).await?;
            continue;
        }

        // For internal references (FileRef, WikilinkRef, MentionRef, SectionRef)
        if let Some(target_id) = target_id {
            // Check if relationship already exists
            let existing = RelationshipRepository::find_by_source_and_target(
                store,
                entity.id,
                target_id,
            )
            .await?;
            if existing.is_some() {
                continue;
            }

            let rel = knowledge_core::features::relationship::Relationship::new(
                entity.id,
                target_id,
                knowledge_core::features::relationship::RelationshipType::References,
            );
            RelationshipRepository::save(store, &rel).await?;

            // Add section metadata for SectionRef
            let mut event_data = serde_json::json!({
                "target_id": target_id,
                "type": "References"
            });

            if let knowledge_import::features::importer::CrossReference::SectionRef { section, .. } = cross_ref {
                event_data["section"] = serde_json::json!(section);
            }

            let event = knowledge_core::ports::Event {
                id: uuid::Uuid::new_v4(),
                event_type: knowledge_core::ports::EventType::RelationshipCreated,
                entity_id: entity.id,
                timestamp: chrono::Utc::now(),
                data: event_data,
            };
            EventLog::append(store, &event).await?;
        }
    }

    let content = components
        .iter()
        .find(|c| c.component_type == ComponentType::Content)
        .and_then(|c| c.data.as_str().map(String::from))
        .unwrap_or_default();

    let tags = components
        .iter()
        .find(|c| c.component_type == ComponentType::Tags)
        .and_then(|c| {
            c.data.as_array().map(|a| {
                a.iter()
                    .filter_map(|v| v.as_str())
                    .collect::<Vec<_>>()
                    .join(", ")
            })
        })
        .unwrap_or_default();

    match action {
        ImportAction::Created => print!("Created: "),
        ImportAction::Merged => print!("Merged: "),
    }
    println!(
        "Entity {} ({:?}) -- \"{}\"",
        entity.id, entity.entity_type, title
    );
    if !tags.is_empty() {
        println!("  Tags: {}", tags);
    }
    println!("  Content: {} words", content.split_whitespace().count());

    Ok(action)
}

async fn cmd_search(
    store: Arc<SqliteStore>,
    query: &str,
    entity_type: Option<&str>,
    tag: Option<&str>,
) -> Result<(), Box<dyn std::error::Error>> {
    let search_query = SearchQuery {
        query: query.to_string(),
        entity_type: entity_type.map(String::from),
        tag: tag.map(String::from),
    };

    let results = SearchIndex::search(store.as_ref(), &search_query).await?;

    if results.is_empty() {
        println!("No entities found.");
        return Ok(());
    }

    println!("Found {} entities:\n", results.len());

    for result in &results {
        if let Some(entity) = EntityRepository::get(store.as_ref(), result.entity_id).await? {
            let components = ComponentRepository::get(store.as_ref(), entity.id).await?;
            let title = components
                .iter()
                .find(|c| c.component_type == ComponentType::Title)
                .and_then(|c| c.data.as_str().map(String::from))
                .unwrap_or_else(|| "Untitled".to_string());

            let tags = components
                .iter()
                .find(|c| c.component_type == ComponentType::Tags)
                .and_then(|c| {
                    c.data.as_array().map(|a| {
                        a.iter()
                            .filter_map(|v| v.as_str())
                            .collect::<Vec<_>>()
                            .join(", ")
                    })
                })
                .unwrap_or_default();

            let confidence_str = result
                .confidence
                .map(|c| format!(" confidence: {:.0}%", c * 100.0))
                .unwrap_or_default();

            println!(
                "  [{:?}] {} -- \"{}\" (score: {:.2}{})",
                entity.entity_type, entity.id, title, result.score, confidence_str
            );
            if !tags.is_empty() {
                println!("    Tags: {}", tags);
            }
            if let Some(ref snippet) = result.snippet {
                println!("    Snippet: {}", snippet);
            }
        }
    }

    Ok(())
}

async fn cmd_get(
    store: Arc<SqliteStore>,
    id_str: &str,
) -> Result<(), Box<dyn std::error::Error>> {
    let id = uuid::Uuid::parse_str(id_str)?;
    let entity = EntityRepository::get(store.as_ref(), id)
        .await?
        .ok_or("Entity not found")?;

    let components = ComponentRepository::get(store.as_ref(), entity.id).await?;
    let relationships = RelationshipRepository::by_source(store.as_ref(), entity.id).await?;

    println!("Entity: {} ({:?})", entity.id, entity.entity_type);
    println!("  Version: {}", entity.version);
    println!("  Active: {}", entity.is_active);
    println!("  Created: {}", entity.created_at);
    println!("  Updated: {}", entity.updated_at);
    println!("\nComponents:");
    for comp in &components {
        println!("  {:?}: {}", comp.component_type, comp.data);
    }
    println!("\nRelationships (outgoing):");
    for rel in &relationships {
        println!("  {:?} -> {}", rel.relationship_type, rel.target_id);
    }

    let events = EventLog::list_by_entity(store.as_ref(), entity.id).await?;
    if !events.is_empty() {
        println!("\nEvents:");
        for event in &events {
            println!("  [{:?}] {}", event.event_type, event.timestamp);
        }
    }

    let versions = EntityRepository::get_version_history(store.as_ref(), entity.id).await?;
    if !versions.is_empty() {
        println!("\nVersion History:");
        for v in &versions {
            println!("  v{}: {}", v.version, v.snapshot);
        }
    }

    Ok(())
}

async fn cmd_list(
    store: Arc<SqliteStore>,
    entity_type: Option<&str>,
) -> Result<(), Box<dyn std::error::Error>> {
    let entities = match entity_type {
        Some(t) => EntityRepository::find_by_type(store.as_ref(), t).await?,
        None => EntityRepository::list(store.as_ref()).await?,
    };

    if entities.is_empty() {
        println!("No entities found.");
        return Ok(());
    }

    println!("Found {} entities:\n", entities.len());

    for entity in &entities {
        let components = ComponentRepository::get(store.as_ref(), entity.id).await?;
        let title = components
            .iter()
            .find(|c| c.component_type == ComponentType::Title)
            .and_then(|c| c.data.as_str().map(String::from))
            .unwrap_or_else(|| "Untitled".to_string());

        println!("  [{:?}] {} -- \"{}\"", entity.entity_type, entity.id, title);
    }

    Ok(())
}

async fn cmd_archive(
    store: Arc<SqliteStore>,
    id_str: &str,
) -> Result<(), Box<dyn std::error::Error>> {
    let id = uuid::Uuid::parse_str(id_str)?;
    let mut entity = EntityRepository::get(store.as_ref(), id)
        .await?
        .ok_or("Entity not found")?;

    entity.archive();
    EntityRepository::save(store.as_ref(), &entity).await?;

    SearchIndex::remove_entity(store.as_ref(), entity.id).await?;

    let event = knowledge_core::ports::Event {
        id: uuid::Uuid::new_v4(),
        event_type: knowledge_core::ports::EventType::EntityArchived,
        entity_id: entity.id,
        timestamp: chrono::Utc::now(),
        data: serde_json::json!({}),
    };
    EventLog::append(store.as_ref(), &event).await?;

    println!(
        "Archived: Entity {} ({:?})",
        entity.id, entity.entity_type
    );
    Ok(())
}

async fn cmd_restore(
    store: Arc<SqliteStore>,
    id_str: &str,
) -> Result<(), Box<dyn std::error::Error>> {
    let id = uuid::Uuid::parse_str(id_str)?;
    let mut entity = EntityRepository::get(store.as_ref(), id)
        .await?
        .ok_or("Entity not found")?;

    entity.restore();
    EntityRepository::save(store.as_ref(), &entity).await?;

    let components = ComponentRepository::get(store.as_ref(), entity.id).await?;
    SearchIndex::index_entity(store.as_ref(), &entity, &components).await?;

    let event = knowledge_core::ports::Event {
        id: uuid::Uuid::new_v4(),
        event_type: knowledge_core::ports::EventType::EntityRestored,
        entity_id: entity.id,
        timestamp: chrono::Utc::now(),
        data: serde_json::json!({}),
    };
    EventLog::append(store.as_ref(), &event).await?;

    println!(
        "Restored: Entity {} ({:?})",
        entity.id, entity.entity_type
    );
    Ok(())
}

async fn cmd_rebuild_index(store: Arc<SqliteStore>) -> Result<(), Box<dyn std::error::Error>> {
    println!("Rebuilding search index...");

    let entities = EntityRepository::list(store.as_ref()).await?;
    let total = entities.len();

    let pb = ProgressBar::new(total as u64);
    pb.set_style(
        ProgressStyle::default_bar()
            .template("[{pos}/{len}] {bar:40} {msg}")
            .unwrap()
            .progress_chars("=>-"),
    );

    let mut entity_data = Vec::new();
    for entity in &entities {
        let components = ComponentRepository::get(store.as_ref(), entity.id).await?;
        entity_data.push((entity.clone(), components));
        pb.inc(1);
    }

    pb.finish_and_clear();

    SearchIndex::rebuild(store.as_ref(), &entity_data).await?;

    println!("Rebuilt index: {} entities", total);
    Ok(())
}

async fn cmd_resolution_log(
    store: Arc<SqliteStore>,
    entity_id_str: Option<&str>,
) -> Result<(), Box<dyn std::error::Error>> {
    if let Some(id_str) = entity_id_str {
        let entity_id: uuid::Uuid = id_str.parse()?;
        let history = EntityResolver::get_merge_history(store.as_ref(), entity_id).await?;

        if history.is_empty() {
            println!("No merge history for entity {}", entity_id);
            return Ok(());
        }

        println!("Merge history for entity {}:", entity_id);
        println!("---");
        for entry in &history {
            println!("Merge ID:     {}", entry.id);
            println!("Source:       {} ({})", entry.source_title, entry.source_id);
            println!("Target:       {} ({})", entry.target_title, entry.target_id);
            println!("Strategy:     {}", entry.strategy);
            println!("Confidence:   {:.2}", entry.confidence);
            println!("Reason:       {}", entry.reason);
            println!("Timestamp:    {}", entry.timestamp);
            println!("---");
        }
    } else {
        let history = EntityResolver::get_all_merge_history(store.as_ref()).await?;

        if history.is_empty() {
            println!("No merge history.");
            return Ok(());
        }

        println!("All merge history ({} entries):", history.len());
        println!("---");
        for entry in &history {
            println!("Merge ID:     {}", entry.id);
            println!("Source:       {} ({})", entry.source_title, entry.source_id);
            println!("Target:       {} ({})", entry.target_title, entry.target_id);
            println!("Strategy:     {}", entry.strategy);
            println!("Confidence:   {:.2}", entry.confidence);
            println!("Reason:       {}", entry.reason);
            println!("Timestamp:    {}", entry.timestamp);
            println!("---");
        }
    }

    Ok(())
}

async fn cmd_resolution_undo(
    store: Arc<SqliteStore>,
    merge_id_str: &str,
) -> Result<(), Box<dyn std::error::Error>> {
    let merge_id: uuid::Uuid = merge_id_str.parse()?;
    EntityResolver::undo_merge(store.as_ref(), merge_id).await?;
    println!("Undone merge {}", merge_id);
    Ok(())
}
