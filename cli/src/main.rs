use clap::{Parser, Subcommand};
use knowledge_core::ports::{
    ComponentRepository, EntityRepository, RelationshipRepository, SearchIndex,
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
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let cli = Cli::parse();
    let store = Arc::new(SqliteStore::new(&cli.db)?);

    match cli.command {
        Commands::Import { path } => cmd_import(store, path).await,
        Commands::Search { query, r#type, tag } => {
            cmd_search(store, &query, r#type.as_deref(), tag.as_deref()).await
        }
        Commands::Get { id } => cmd_get(store, &id).await,
        Commands::List { r#type } => cmd_list(store, r#type.as_deref()).await,
        Commands::Archive { id } => cmd_archive(store, &id).await,
        Commands::Restore { id } => cmd_restore(store, &id).await,
        Commands::RebuildIndex => cmd_rebuild_index(store).await,
    }
}

async fn cmd_import(
    store: Arc<SqliteStore>,
    path: PathBuf,
) -> Result<(), Box<dyn std::error::Error>> {
    let importer = knowledge_import::features::importer::Importer::new();

    let mut files = Vec::new();
    if path.is_dir() {
        for entry in std::fs::read_dir(&path)? {
            let entry = entry?;
            let file_path = entry.path();
            if file_path.extension().and_then(|e| e.to_str()) == Some("md") {
                files.push(file_path);
            }
        }
    } else {
        files.push(path);
    }

    let total = files.len();
    let mut created = 0;
    let mut updated = 0;
    let mut errors: Vec<String> = Vec::new();

    let pb = ProgressBar::new(total as u64);
    pb.set_style(
        ProgressStyle::default_bar()
            .template("[{pos}/{len}] {bar:40} {msg}")
            .unwrap()
            .progress_chars("=>-"),
    );

    for file_path in &files {
        let fname = file_path.file_name().unwrap_or_default().to_string_lossy();
        pb.set_message(fname.to_string());
        match import_single_file(&store, &importer, file_path).await {
            Ok(action) => match action {
                ImportAction::Created => {
                    created += 1;
                    pb.inc(1);
                }
                ImportAction::Updated => {
                    updated += 1;
                    pb.inc(1);
                }
            },
            Err(e) => {
                eprintln!("\nERROR: {}: {}", file_path.display(), e);
                errors.push(format!("{}: {}", file_path.display(), e));
                pb.inc(1);
            }
        }
    }

    pb.finish_and_clear();

    println!("\n--- Import Summary ---");
    println!("Total files: {}", total);
    println!("Created: {}", created);
    println!("Updated: {}", updated);
    if !errors.is_empty() {
        println!("Errors: {}", errors.len());
        for err in &errors {
            eprintln!("  {}", err);
        }
    }

    Ok(())
}

enum ImportAction {
    Created,
    Updated,
}

async fn import_single_file(
    store: &SqliteStore,
    importer: &knowledge_import::features::importer::Importer,
    path: &std::path::Path,
) -> Result<ImportAction, Box<dyn std::error::Error>> {
    let result = importer.import_file(path)?;

    let title = result
        .components
        .iter()
        .find(|c| c.component_type == ComponentType::Title)
        .and_then(|c| c.data.as_str().map(String::from))
        .unwrap_or_default();

    let existing = EntityRepository::find_by_title(store, &title)
        .await?
        .into_iter()
        .find(|e| e.entity_type == result.entity.entity_type);

    let (entity, action) = if let Some(mut existing) = existing {
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
            data: serde_json::json!({"source": path.to_string_lossy()}),
        };

        store
            .update_entity_with_components(&existing, &components, &event)
            .await?;

        (existing, ImportAction::Updated)
    } else {
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
        let target_path_str = cross_ref.target_path.to_string_lossy();
        let matching_components = ComponentRepository::find_by_component_data(
            store,
            "Provenance",
            "source",
            &target_path_str,
        )
        .await?;

        for target_comp in &matching_components {
            let target_id = target_comp.entity_id;

            let existing = RelationshipRepository::find_by_source_and_target(
                store,
                entity.id,
                target_id,
            )
            .await?;
            if existing.is_some() {
                continue;
            }

            let rel =
                knowledge_core::features::relationship::Relationship::new(
                    entity.id,
                    target_id,
                    knowledge_core::features::relationship::RelationshipType::References,
                );
            RelationshipRepository::save(store, &rel).await?;

            let event = knowledge_core::ports::Event {
                id: uuid::Uuid::new_v4(),
                event_type: knowledge_core::ports::EventType::RelationshipCreated,
                entity_id: entity.id,
                timestamp: chrono::Utc::now(),
                data: serde_json::json!({"target_id": target_id, "type": "References"}),
            };
            EventLog::append(store, &event).await?;
            break;
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
        ImportAction::Updated => print!("Updated: "),
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
