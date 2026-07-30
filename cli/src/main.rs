use async_trait::async_trait;
use clap::{Parser, Subcommand};
use indicatif::{ProgressBar, ProgressStyle};
use knowledge_core::features::component::{Component, ComponentType};
use knowledge_core::features::entity::{Entity, EntityType};
use knowledge_core::features::relationship::{Relationship, RelationshipType};
use knowledge_core::ports::{
    AiAdapter, Collection, CollectionRepository, ComponentRepository, ConversationRepository,
    EntityRepository, EntityResolver, EntityVersion, Event, EventLog, EventNotifier, Plugin,
    RelationshipRepository, SearchIndex, SearchQuery, SearchResult, StorageError,
    TransactionalWrite, TraversalConfig, TraversalDirection, TraversalError, TraversalPort,
    TraversalQuery, TraversalResult, ViewAdapter, ViewFilter, ViewOutput, ViewRegistry,
};
use knowledge_derivation::features::search::{providers::create_from_config, AiConfig};
use knowledge_derivation::features::view::{
    graph::GraphViewAdapter, table::TableViewAdapter, timeline::TimelineViewAdapter,
    tree::TreeViewAdapter,
};
use knowledge_import::features::importer::ImportAdapter;
use knowledge_plugin::dynamic::load_plugins_from;
use knowledge_plugin::registry::built_in_plugins;
use knowledge_storage::adapters::sqlite::vector_store::SqliteVectorStore;
use knowledge_storage::adapters::sqlite::SqliteStore;
use std::path::{Path, PathBuf};
use std::sync::Arc;
use uuid::Uuid;

// =============================================================================
// Store Wrapper — bridges Arc<SqliteStore> to Box<dyn Repository>
// =============================================================================

/// Wrapper that delegates repository trait methods to an `Arc<SqliteStore>`.
/// Needed because view adapters require `Box<dyn EntityRepository>` etc.,
/// but the store is shared via `Arc` across commands.
struct StoreWrapper(Arc<SqliteStore>);

#[async_trait]
impl EntityRepository for StoreWrapper {
    async fn get(&self, id: Uuid) -> Result<Option<Entity>, StorageError> {
        EntityRepository::get(self.0.as_ref(), id).await
    }
    async fn save(&self, entity: &Entity) -> Result<(), StorageError> {
        EntityRepository::save(self.0.as_ref(), entity).await
    }
    async fn delete(&self, id: Uuid) -> Result<(), StorageError> {
        EntityRepository::delete(self.0.as_ref(), id).await
    }
    async fn list(&self) -> Result<Vec<Entity>, StorageError> {
        EntityRepository::list(self.0.as_ref()).await
    }
    async fn find_by_type(&self, entity_type: &str) -> Result<Vec<Entity>, StorageError> {
        EntityRepository::find_by_type(self.0.as_ref(), entity_type).await
    }
    async fn find_by_title(&self, title: &str) -> Result<Vec<Entity>, StorageError> {
        EntityRepository::find_by_title(self.0.as_ref(), title).await
    }
    async fn increment_version(&self, id: Uuid) -> Result<(), StorageError> {
        EntityRepository::increment_version(self.0.as_ref(), id).await
    }
    async fn find_by_component_type(
        &self,
        component_type: &str,
    ) -> Result<Vec<Entity>, StorageError> {
        EntityRepository::find_by_component_type(self.0.as_ref(), component_type).await
    }
    async fn find_by_tag(&self, tag: &str) -> Result<Vec<Entity>, StorageError> {
        EntityRepository::find_by_tag(self.0.as_ref(), tag).await
    }
    async fn get_version_history(
        &self,
        entity_id: Uuid,
    ) -> Result<Vec<EntityVersion>, StorageError> {
        EntityRepository::get_version_history(self.0.as_ref(), entity_id).await
    }
}

#[async_trait]
impl ComponentRepository for StoreWrapper {
    async fn get(&self, entity_id: Uuid) -> Result<Vec<Component>, StorageError> {
        ComponentRepository::get(self.0.as_ref(), entity_id).await
    }
    async fn save(&self, component: &Component) -> Result<(), StorageError> {
        ComponentRepository::save(self.0.as_ref(), component).await
    }
    async fn delete(&self, id: Uuid) -> Result<(), StorageError> {
        ComponentRepository::delete(self.0.as_ref(), id).await
    }
    async fn find_by_type(
        &self,
        entity_id: Uuid,
        component_type: &str,
    ) -> Result<Vec<Component>, StorageError> {
        ComponentRepository::find_by_type(self.0.as_ref(), entity_id, component_type).await
    }
    async fn update_data(&self, id: Uuid, data: serde_json::Value) -> Result<(), StorageError> {
        ComponentRepository::update_data(self.0.as_ref(), id, data).await
    }
    async fn find_by_component_data(
        &self,
        component_type: &str,
        json_path: &str,
        value: &str,
    ) -> Result<Vec<Component>, StorageError> {
        ComponentRepository::find_by_component_data(
            self.0.as_ref(),
            component_type,
            json_path,
            value,
        )
        .await
    }
    async fn delete_by_entity(&self, entity_id: Uuid) -> Result<(), StorageError> {
        ComponentRepository::delete_by_entity(self.0.as_ref(), entity_id).await
    }
}

#[async_trait]
impl RelationshipRepository for StoreWrapper {
    async fn get(&self, id: Uuid) -> Result<Option<Relationship>, StorageError> {
        RelationshipRepository::get(self.0.as_ref(), id).await
    }
    async fn save(&self, relationship: &Relationship) -> Result<(), StorageError> {
        RelationshipRepository::save(self.0.as_ref(), relationship).await
    }
    async fn update(&self, relationship: &Relationship) -> Result<(), StorageError> {
        RelationshipRepository::update(self.0.as_ref(), relationship).await
    }
    async fn delete(&self, id: Uuid) -> Result<(), StorageError> {
        RelationshipRepository::delete(self.0.as_ref(), id).await
    }
    async fn by_source(&self, source_id: Uuid) -> Result<Vec<Relationship>, StorageError> {
        RelationshipRepository::by_source(self.0.as_ref(), source_id).await
    }
    async fn by_target(&self, target_id: Uuid) -> Result<Vec<Relationship>, StorageError> {
        RelationshipRepository::by_target(self.0.as_ref(), target_id).await
    }
    async fn find_by_source_and_target(
        &self,
        source_id: Uuid,
        target_id: Uuid,
    ) -> Result<Option<Relationship>, StorageError> {
        RelationshipRepository::find_by_source_and_target(self.0.as_ref(), source_id, target_id)
            .await
    }
    async fn find_by_type(
        &self,
        relationship_type: &str,
    ) -> Result<Vec<Relationship>, StorageError> {
        RelationshipRepository::find_by_type(self.0.as_ref(), relationship_type).await
    }
}

#[async_trait]
impl TraversalPort for StoreWrapper {
    async fn traverse(
        &self,
        query: &TraversalQuery,
        config: &TraversalConfig,
    ) -> Result<Vec<TraversalResult>, TraversalError> {
        TraversalPort::traverse(self.0.as_ref(), query, config).await
    }
}

#[async_trait]
impl CollectionRepository for StoreWrapper {
    async fn create(&self, collection: Collection) -> Result<Collection, StorageError> {
        CollectionRepository::create(self.0.as_ref(), collection).await
    }
    async fn get(&self, id: Uuid) -> Result<Option<Collection>, StorageError> {
        CollectionRepository::get(self.0.as_ref(), id).await
    }
    async fn update(&self, collection: Collection) -> Result<Collection, StorageError> {
        CollectionRepository::update(self.0.as_ref(), collection).await
    }
    async fn delete(&self, id: Uuid) -> Result<(), StorageError> {
        CollectionRepository::delete(self.0.as_ref(), id).await
    }
    async fn list(&self) -> Result<Vec<Collection>, StorageError> {
        CollectionRepository::list(self.0.as_ref()).await
    }
    async fn add_member(&self, collection_id: Uuid, entity_id: Uuid) -> Result<(), StorageError> {
        CollectionRepository::add_member(self.0.as_ref(), collection_id, entity_id).await
    }
    async fn remove_member(
        &self,
        collection_id: Uuid,
        entity_id: Uuid,
    ) -> Result<(), StorageError> {
        CollectionRepository::remove_member(self.0.as_ref(), collection_id, entity_id).await
    }
    async fn get_members(&self, collection_id: Uuid) -> Result<Vec<Entity>, StorageError> {
        CollectionRepository::get_members(self.0.as_ref(), collection_id).await
    }
    async fn get_entity_collections(
        &self,
        entity_id: Uuid,
    ) -> Result<Vec<Collection>, StorageError> {
        CollectionRepository::get_entity_collections(self.0.as_ref(), entity_id).await
    }
    async fn is_member(&self, collection_id: Uuid, entity_id: Uuid) -> Result<bool, StorageError> {
        CollectionRepository::is_member(self.0.as_ref(), collection_id, entity_id).await
    }
}

// =============================================================================
// View Registry Factory
// =============================================================================

/// Creates a `ViewRegistry` pre-loaded with all four built-in views.
///
/// Each call constructs fresh adapter instances backed by `StoreWrapper`.
/// This is called after write operations so the registry can notify views
/// to invalidate cached state.
fn create_view_registry(store: &Arc<SqliteStore>) -> ViewRegistry {
    let mut registry = ViewRegistry::new();
    registry.register(Box::new(TreeViewAdapter::new(
        Box::new(StoreWrapper(store.clone())),
        Box::new(StoreWrapper(store.clone())),
        Some(Box::new(StoreWrapper(store.clone()))),
    )));
    registry.register(Box::new(GraphViewAdapter::new(
        Box::new(StoreWrapper(store.clone())),
        Box::new(StoreWrapper(store.clone())),
        Box::new(StoreWrapper(store.clone())),
        Box::new(StoreWrapper(store.clone())),
    )));
    registry.register(Box::new(TableViewAdapter::new(
        Box::new(StoreWrapper(store.clone())),
        Box::new(StoreWrapper(store.clone())),
    )));
    registry.register(Box::new(TimelineViewAdapter::new(
        Box::new(StoreWrapper(store.clone())),
        Box::new(StoreWrapper(store.clone())),
    )));
    registry
}

/// Notifies all registered views of a canonical event.
///
/// Builds a temporary `ViewRegistry` and calls `on_event` on every
/// registered view. Views rebuild on the next `render()` call.
async fn notify_event(store: &Arc<SqliteStore>, event: &Event) {
    let registry = create_view_registry(store);
    if let Err(e) = registry.notify(event).await {
        eprintln!("Warning: failed to notify views: {}", e);
    }
}

#[derive(Parser)]
#[command(name = "kos", about = "Knowledge OS CLI")]
struct Cli {
    /// Path to SQLite database file
    #[arg(short, long, default_value = "knowledge.db", global = true)]
    db: String,

    /// AI provider for embeddings and semantic search
    /// Formats: mock://128, openai://text-embedding-3-small?api_key=KEY
    /// Defaults to mock if OPENAI_API_KEY is not set
    #[arg(long, global = true)]
    ai_provider: Option<String>,

    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    /// Import a file, directory, URL, or database
    Import {
        /// Path to file or directory (or URL)
        path: Option<PathBuf>,
        /// Output progress as JSON lines (machine-readable)
        #[arg(long)]
        json: bool,
        /// Auto-merge confidence threshold (0.0-1.0, default: 0.92)
        #[arg(long, default_value = "0.92")]
        auto_merge_threshold: f64,
        /// Review confidence threshold (0.0-1.0, default: 0.78)
        #[arg(long, default_value = "0.78")]
        review_threshold: f64,
        /// Database connection string (sqlite://, postgres://, mysql://)
        #[arg(long = "connection-string")]
        connection_string: Option<String>,
        /// Tables to import from database (comma-separated)
        #[arg(long)]
        tables: Option<String>,
        /// Import files recursively from directory
        #[arg(long)]
        recursive: bool,
        /// Preview mode for structured data (shows columns and sample rows)
        #[arg(long)]
        preview: bool,
        /// Undo the last import
        #[arg(long)]
        undo: bool,
        /// Import ID to undo (requires --undo)
        #[arg(long)]
        import_id: Option<String>,
        /// Format hint for structured preview (csv, json, xml, yaml)
        #[arg(long)]
        format: Option<String>,
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
        /// Use semantic (embedding) search only
        #[arg(long)]
        semantic: bool,
        /// Use hybrid search (keyword + semantic via RRF)
        #[arg(long)]
        hybrid: bool,
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
    /// Traverse the entity graph
    Traverse {
        /// Entity ID to start traversal from
        entity_id: String,
        /// Maximum depth (hops)
        #[arg(long, default_value = "3")]
        depth: u32,
        /// Traversal direction (outgoing, incoming, both)
        #[arg(long, default_value = "outgoing")]
        direction: String,
        /// Filter by relationship type
        #[arg(short = 't', long)]
        r#type: Option<String>,
        /// Filter by entity type
        #[arg(long)]
        entity_type: Option<String>,
    },
    /// View knowledge in different projections
    View {
        #[command(subcommand)]
        view_type: ViewCommands,
    },
    /// Manage entity collections
    Collection {
        #[command(subcommand)]
        action: CollectionCommands,
    },
    /// Manage conversations
    Conversation {
        #[command(subcommand)]
        action: ConversationCommands,
    },
    /// Manage plugins
    Plugin {
        #[command(subcommand)]
        action: PluginCommands,
    },
}

#[derive(Subcommand)]
enum ViewCommands {
    /// Hierarchical tree view grouped by entity type
    Tree {
        /// Filter by entity type
        #[arg(short = 'T', long)]
        r#type: Option<String>,
    },
    /// Graph view with nodes and edges
    Graph {
        /// Entity ID to start from
        #[arg(short, long)]
        from: Option<String>,
        /// Maximum traversal depth
        #[arg(long, default_value = "3")]
        depth: u32,
        /// Filter by entity type
        #[arg(short = 'T', long)]
        r#type: Option<String>,
    },
    /// Table view with sortable columns
    Table {
        /// Column to sort by
        #[arg(long)]
        sort: Option<String>,
        /// Filter by search query
        #[arg(short, long)]
        filter: Option<String>,
        /// Filter by entity type
        #[arg(short = 'T', long)]
        r#type: Option<String>,
    },
    /// Timeline view ordered by creation time
    Timeline {
        /// Filter by entity type
        #[arg(short = 'T', long)]
        r#type: Option<String>,
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

#[derive(Subcommand)]
enum ConversationCommands {
    /// List all conversations
    List,
    /// Show a conversation with its messages
    Get {
        /// Conversation ID
        id: String,
    },
    /// Rename a conversation
    Rename {
        /// Conversation ID
        id: String,
        /// New title
        title: String,
    },
    /// Archive a conversation
    Delete {
        /// Conversation ID
        id: String,
    },
}

#[derive(Subcommand)]
enum PluginCommands {
    /// List all loaded plugins
    List,
    /// Show details about a specific plugin
    Info {
        /// Plugin name
        name: String,
    },
    /// Install a plugin from a directory containing plugin.toml
    Install {
        /// Path to the plugin directory
        path: PathBuf,
    },
    /// Uninstall a plugin by name
    Uninstall {
        /// Plugin name
        name: String,
    },
}

#[derive(Subcommand)]
enum CollectionCommands {
    /// Create a new collection
    Create {
        /// Collection name
        name: String,
        /// Optional description
        #[arg(long)]
        description: Option<String>,
    },
    /// List all collections
    List,
    /// Add an entity to a collection
    Add {
        /// Collection ID
        collection_id: String,
        /// Entity ID
        entity_id: String,
    },
    /// Remove an entity from a collection
    Remove {
        /// Collection ID
        collection_id: String,
        /// Entity ID
        entity_id: String,
    },
    /// List members of a collection
    Members {
        /// Collection ID
        collection_id: String,
    },
    /// Delete a collection
    Delete {
        /// Collection ID
        collection_id: String,
    },
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let cli = Cli::parse();
    let store = Arc::new(SqliteStore::new(&cli.db)?);

    // Initialize AI provider and vector store for semantic search
    let ai_config = match &cli.ai_provider {
        Some(provider) => AiConfig::from_provider(provider),
        None => AiConfig::from_env(),
    };
    let ai_adapter: Arc<dyn AiAdapter> = Arc::from(create_from_config(&ai_config.provider)?);
    let vector_dimensions = ai_adapter.dimensions();
    let vector_store = Arc::new(SqliteVectorStore::new(
        SqliteStore::new(&cli.db)?,
        vector_dimensions,
    ));

    match cli.command {
        Commands::Import {
            path,
            json,
            auto_merge_threshold,
            review_threshold,
            connection_string,
            tables,
            recursive,
            preview,
            undo,
            import_id,
            format,
        } => {
            cmd_import(
                store.clone(),
                path,
                json,
                Some(ai_adapter.as_ref()),
                Some(vector_store.as_ref()),
                auto_merge_threshold,
                review_threshold,
                connection_string,
                tables,
                recursive,
                preview,
                undo,
                import_id,
                format,
            )
            .await
        }
        Commands::Search {
            query,
            r#type,
            tag,
            semantic,
            hybrid,
        } => {
            cmd_search(
                store,
                vector_store,
                ai_adapter.as_ref(),
                &query,
                r#type.as_deref(),
                tag.as_deref(),
                semantic,
                hybrid,
            )
            .await
        }
        Commands::Get { id } => cmd_get(store, &id).await,
        Commands::List { r#type } => cmd_list(store, r#type.as_deref()).await,
        Commands::Archive { id } => cmd_archive(store, &id).await,
        Commands::Restore { id } => cmd_restore(store, &id).await,
        Commands::RebuildIndex => cmd_rebuild_index(store).await,
        Commands::Resolution { action } => match action {
            ResolutionCommands::Log { entity } => {
                cmd_resolution_log(store, entity.as_deref()).await
            }
            ResolutionCommands::Undo { merge_id } => cmd_resolution_undo(store, &merge_id).await,
        },
        Commands::Traverse {
            entity_id,
            depth,
            direction,
            r#type,
            entity_type,
        } => {
            cmd_traverse(
                store,
                &entity_id,
                depth,
                &direction,
                r#type.as_deref(),
                entity_type.as_deref(),
            )
            .await
        }
        Commands::View { view_type } => cmd_view(store, view_type).await,
        Commands::Collection { action } => cmd_collection(&store, action).await,
        Commands::Conversation { action } => cmd_conversation(&store, action).await,
        Commands::Plugin { action } => match action {
            PluginCommands::List => cmd_plugin_list().await,
            PluginCommands::Info { name } => cmd_plugin_info(&name).await,
            PluginCommands::Install { path } => cmd_plugin_install(path).await,
            PluginCommands::Uninstall { name } => cmd_plugin_uninstall(&name).await,
        },
    }
}

#[allow(clippy::too_many_arguments)]
async fn cmd_import(
    store: Arc<SqliteStore>,
    path: Option<PathBuf>,
    json_mode: bool,
    ai_adapter: Option<&dyn AiAdapter>,
    vector_store: Option<&SqliteVectorStore>,
    auto_merge_threshold: f64,
    review_threshold: f64,
    connection_string: Option<String>,
    tables: Option<String>,
    recursive: bool,
    preview: bool,
    undo: bool,
    import_id: Option<String>,
    format: Option<String>,
) -> Result<(), Box<dyn std::error::Error>> {
    // Handle undo
    if undo {
        return cmd_undo_import(&store, import_id.as_deref()).await;
    }

    // Handle database import
    if let Some(conn_str) = connection_string {
        return cmd_import_database(&store, &conn_str, tables.as_deref(), json_mode).await;
    }

    // Handle preview
    if preview {
        if let Some(ref path) = path {
            return cmd_import_preview(path, format.as_deref(), json_mode).await;
        }
        eprintln!("Error: --preview requires a file path");
        std::process::exit(1);
    }

    let path = match path {
        Some(p) => p,
        None => {
            eprintln!("Error: a path is required for import (use --db for database import)");
            std::process::exit(1);
        }
    };

    // Build plugin registry with all known importers
    let registry = built_in_plugins();

    // Load dynamic plugins from the plugin directory
    let plugin_dir = plugin_dir();
    let dynamic_plugins = load_plugins_from(&plugin_dir);
    for _plugin in &dynamic_plugins {}

    let path_str = path.to_string_lossy();

    // Check if path is a URL
    if path_str.starts_with("http://") || path_str.starts_with("https://") {
        let pb = ProgressBar::new(1);
        pb.set_style(
            ProgressStyle::default_bar()
                .template("[{pos}/{len}] {bar:40} {msg}")
                .unwrap()
                .progress_chars("=>-"),
        );
        pb.set_message(path_str.to_string());

        match registry.get_importer("url") {
            Ok(importer) => {
                match import_with_adapter(
                    &store,
                    importer,
                    &path,
                    ai_adapter,
                    vector_store,
                    auto_merge_threshold,
                    review_threshold,
                )
                .await
                {
                    Ok(_) => {
                        println!("\nImported URL: {}", path_str);
                        pb.inc(1);
                    }
                    Err(e) => {
                        eprintln!("\nERROR: {}: {}", path_str, e);
                    }
                }
            }
            Err(_) => {
                eprintln!("\nERROR: {}: no URL importer available", path_str);
            }
        }
        pb.finish_and_clear();
        return Ok(());
    }

    // Collect files: single file, directory (recursive or flat)
    let mut all_files: Vec<std::path::PathBuf> = Vec::new();
    if path.is_dir() {
        let dir_importer = knowledge_import::features::importer::DirectoryImporter::new(recursive);
        all_files = dir_importer.list_files(&path)?;
    } else {
        all_files.push(path);
    }

    // Filter to only files with registered importers, silently skipping unsupported formats
    let mut files_with_importers: Vec<(std::path::PathBuf, &dyn ImportAdapter)> = Vec::new();
    for file_path in &all_files {
        let ext = file_path
            .extension()
            .and_then(|e| e.to_str())
            .unwrap_or("")
            .to_lowercase();

        let importer = registry.get_importer(&ext).ok().or_else(|| {
            if let Ok(fmt) =
                knowledge_import::features::importer::magic_bytes::detect_format(file_path)
            {
                let key = match fmt {
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
                if !key.is_empty() {
                    registry.get_importer(key).ok()
                } else {
                    None
                }
            } else {
                None
            }
        });

        if let Some(adapter) = importer {
            files_with_importers.push((file_path.clone(), adapter));
        }
    }

    let total_importable = files_with_importers.len();
    let mut created = 0;
    let mut merged = 0;
    let mut errors: Vec<String> = Vec::new();

    let pb = if json_mode {
        None
    } else {
        let pb = ProgressBar::new(total_importable as u64);
        pb.set_style(
            ProgressStyle::default_bar()
                .template("[{pos}/{len}] {bar:40} {msg}")
                .unwrap()
                .progress_chars("=>-"),
        );
        Some(pb)
    };

    for (i, (file_path, importer)) in files_with_importers.iter().enumerate() {
        let fname = file_path.file_name().unwrap_or_default().to_string_lossy();
        if let Some(ref pb) = pb {
            pb.set_message(fname.to_string());
        }

        let action = import_with_adapter(
            &store,
            *importer,
            file_path,
            ai_adapter,
            vector_store,
            auto_merge_threshold,
            review_threshold,
        )
        .await;

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

                // Record import for undo support
                let ext = file_path.extension().and_then(|e| e.to_str()).unwrap_or("");
                let _ = knowledge_import::features::importer::record_import(
                    file_path,
                    vec![uuid::Uuid::nil()],
                    ext,
                );

                if json_mode {
                    println!(
                        "{}",
                        serde_json::json!({
                            "event": "import",
                            "file": file_path.to_string_lossy(),
                            "action": action_str,
                            "position": i + 1,
                            "total": total_importable,
                        })
                    );
                } else if let Some(ref pb) = pb {
                    pb.inc(1);
                }
            }
            Err(e) => {
                let err_msg = format!("{}: {}", file_path.display(), e);
                errors.push(err_msg.clone());

                if json_mode {
                    println!(
                        "{}",
                        serde_json::json!({
                            "event": "error",
                            "file": file_path.to_string_lossy(),
                            "error": err_msg,
                            "position": i + 1,
                            "total": total_importable,
                        })
                    );
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
        println!(
            "{}",
            serde_json::json!({
                "event": "summary",
                "total": total_importable,
                "created": created,
                "merged": merged,
                "errors": errors.len(),
            })
        );
    } else {
        println!("\n--- Import Summary ---");
        println!("Total files: {}", total_importable);
        println!("Created: {}", created);
        println!("Duplicates resolved: {}", merged);
        if !errors.is_empty() {
            println!("Errors: {}", errors.len());
            for err in &errors {
                eprintln!("  {}", err);
            }
        }
    }

    // Notify views of import completion so they invalidate cached state
    if created + merged > 0 {
        let event = knowledge_core::ports::Event {
            id: uuid::Uuid::new_v4(),
            event_type: knowledge_core::ports::EventType::EntityCreated,
            entity_id: uuid::Uuid::nil(),
            timestamp: chrono::Utc::now(),
            data: serde_json::json!({
                "source": "import_batch",
                "created": created,
                "merged": merged,
            }),
        };
        notify_event(&store, &event).await;
    }

    Ok(())
}

/// Undo the last import (or a specific import by ID).
#[allow(unused_variables)]
async fn cmd_undo_import(
    store: &SqliteStore,
    import_id: Option<&str>,
) -> Result<(), Box<dyn std::error::Error>> {
    let record = knowledge_import::features::importer::undo_last_import()?;

    match record {
        Some(import_record) => {
            for entity_id in &import_record.entity_ids {
                if *entity_id != uuid::Uuid::nil() {
                    let _ =
                        knowledge_core::ports::EntityRepository::delete(store, *entity_id).await;
                    let _ =
                        knowledge_core::ports::SearchIndex::remove_entity(store, *entity_id).await;
                }
            }
            println!(
                "Undone import from '{}' ({} entities removed)",
                import_record.source_path,
                import_record.entity_ids.len()
            );
        }
        None => {
            println!("Nothing to undo.");
        }
    }
    Ok(())
}

/// Import tables from a database.
async fn cmd_import_database(
    store: &SqliteStore,
    conn_str: &str,
    tables_str: Option<&str>,
    json_mode: bool,
) -> Result<(), Box<dyn std::error::Error>> {
    use knowledge_core::ports::DatabaseSource;
    use knowledge_import::features::importer::database::{
        MySqlDatabaseSource, PostgresDatabaseSource, SqliteDatabaseSource,
    };

    let source: Box<dyn DatabaseSource> =
        if conn_str.starts_with("sqlite") || !conn_str.contains("://") {
            let path = if conn_str.starts_with("sqlite:///") {
                let p = conn_str.trim_start_matches("sqlite:///");
                let p = p.trim_end_matches("?mode=rwc");
                std::path::PathBuf::from(p)
            } else {
                std::path::PathBuf::from(conn_str)
            };
            Box::new(SqliteDatabaseSource::new(path))
        } else if conn_str.starts_with("postgres") || conn_str.starts_with("postgresql") {
            Box::new(PostgresDatabaseSource::new(conn_str.to_string()))
        } else if conn_str.starts_with("mysql") {
            Box::new(MySqlDatabaseSource::new(conn_str.to_string()))
        } else {
            Box::new(SqliteDatabaseSource::new(std::path::PathBuf::from(
                conn_str,
            )))
        };

    let all_tables = source.list_tables().await?;
    let tables: Vec<_> = match tables_str {
        Some(t) => {
            let names: std::collections::HashSet<String> =
                t.split(',').map(|s| s.trim().to_string()).collect();
            all_tables
                .into_iter()
                .filter(|t| names.contains(&t.name))
                .collect()
        }
        None => all_tables,
    };

    if tables.is_empty() {
        println!("No tables found to import.");
        return Ok(());
    }

    let mut created = 0u64;
    for table in &tables {
        if !json_mode {
            println!(
                "Importing table '{}' ({} rows)...",
                table.name, table.row_count
            );
        }
        let preview = source
            .preview_table(&table.name, table.row_count as usize)
            .await?;
        for row in &preview.rows {
            let entity = knowledge_core::features::entity::Entity::new(
                knowledge_core::features::entity::EntityType::new("DatabaseRecord"),
            );
            let row_data: std::collections::HashMap<String, serde_json::Value> = table
                .columns
                .iter()
                .zip(row.iter())
                .map(|(col, val)| {
                    let json_val = match val {
                        knowledge_core::ports::DbColumnValue::Text(s) => {
                            serde_json::Value::String(s.clone())
                        }
                        knowledge_core::ports::DbColumnValue::Integer(i) => serde_json::json!(i),
                        knowledge_core::ports::DbColumnValue::Float(f) => serde_json::json!(f),
                        knowledge_core::ports::DbColumnValue::Boolean(b) => serde_json::json!(b),
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
                        "source": format!("{}:{}", conn_str, table.name),
                        "imported_at": chrono::Utc::now().to_rfc3339(),
                        "format": "database",
                    }),
                ),
            ];

            let event = knowledge_core::ports::Event {
                id: uuid::Uuid::new_v4(),
                event_type: knowledge_core::ports::EventType::EntityCreated,
                entity_id: entity.id,
                timestamp: chrono::Utc::now(),
                data: serde_json::json!({"source": format!("{}:{}", conn_str, table.name)}),
            };

            store
                .save_entity_with_components(&entity, &components, &event)
                .await?;
            created += 1;
        }
    }

    println!("\n--- Database Import Summary ---");
    println!("Tables imported: {}", tables.len());
    println!("Rows created: {}", created);

    Ok(())
}

/// Preview structured data from a file.
async fn cmd_import_preview(
    path: &std::path::Path,
    format_hint: Option<&str>,
    json_mode: bool,
) -> Result<(), Box<dyn std::error::Error>> {
    let ext = path
        .extension()
        .and_then(|e| e.to_str())
        .unwrap_or("")
        .to_lowercase();
    let fmt = format_hint.unwrap_or(&ext);

    match fmt {
        "csv" => {
            let importer = knowledge_import::features::importer::CsvImporter::new();
            let preview = importer.preview(path, 10).await?;

            if json_mode {
                println!(
                    "{}",
                    serde_json::json!({
                        "format": "csv",
                        "columns": preview.columns.iter().map(|c| {
                            serde_json::json!({
                                "name": c.name,
                                "data_type": c.data_type,
                                "nullable": c.nullable,
                            })
                        }).collect::<Vec<_>>(),
                        "sample_rows": preview.sample_rows,
                        "total_rows": preview.row_count,
                    })
                );
            } else {
                println!("CSV Preview: {}", path.display());
                println!("Columns ({}):", preview.columns.len());
                for col in &preview.columns {
                    println!("  {} ({})", col.name, col.data_type);
                }
                println!("\nSample rows ({} total):", preview.row_count);
                for row in &preview.sample_rows {
                    let vals: Vec<String> = row
                        .iter()
                        .map(|v| match v {
                            knowledge_core::ports::ColumnValue::Text(s) => s.clone(),
                            knowledge_core::ports::ColumnValue::Integer(i) => i.to_string(),
                            knowledge_core::ports::ColumnValue::Float(f) => f.to_string(),
                            knowledge_core::ports::ColumnValue::Boolean(b) => b.to_string(),
                            knowledge_core::ports::ColumnValue::Null => "NULL".to_string(),
                        })
                        .collect();
                    println!("  [{}]", vals.join(", "));
                }
            }
        }
        _ => {
            let content = std::fs::read_to_string(path)?;
            let lines: Vec<&str> = content.lines().take(10).collect();

            if json_mode {
                println!(
                    "{}",
                    serde_json::json!({
                        "format": fmt,
                        "lines": lines,
                        "total_lines": content.lines().count(),
                    })
                );
            } else {
                println!("Preview of {}: {}", fmt, path.display());
                println!(
                    "First {} lines ({} total):",
                    lines.len(),
                    content.lines().count()
                );
                for line in &lines {
                    println!("  {}", line);
                }
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
    importer: &dyn ImportAdapter,
    path: &std::path::Path,
    ai_adapter: Option<&dyn AiAdapter>,
    vector_store: Option<&SqliteVectorStore>,
    auto_merge_threshold: f64,
    review_threshold: f64,
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
    let candidates =
        EntityResolver::find_candidates(store, &result.entity, &title, content.as_deref()).await?;

    // PONYTAIL: Three-zone decision model.
    // >= auto_merge_threshold: auto-merge, review_threshold-auto_merge_threshold: review (skip merge, log), < review_threshold: reject (create new)
    // Exact title matches always merge (same file re-import).
    let best_candidate = candidates
        .into_iter()
        .find(|c| c.confidence >= review_threshold)
        .filter(|c| {
            // Exact title match (title_score == 1.0) — always merge
            if c.title_score.is_some_and(|s| (s - 1.0).abs() < f64::EPSILON) {
                return true;
            }
            // High confidence — auto-merge
            if c.confidence >= auto_merge_threshold {
                return true;
            }
            // Review zone — log and skip
            eprintln!(
                "  Review needed: {} (confidence: {:.2}, title: {:.2}, content: {:.2}, meta: {:.2}, struct: {:.2})",
                c.reason,
                c.confidence,
                c.title_score.unwrap_or(0.0),
                c.content_score.unwrap_or(0.0),
                c.metadata_score.unwrap_or(0.0),
                c.structural_score.unwrap_or(0.0),
            );
            false
        });

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
        let source_components_snapshot: Vec<_> = result
            .components
            .iter()
            .map(|c| {
                serde_json::json!({
                    "id": c.id.to_string(),
                    "component_type": serde_json::to_string(&c.component_type).unwrap(),
                    "data": c.data,
                    "created_at": c.created_at.to_rfc3339(),
                    "version": c.version,
                })
            })
            .collect();

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
            knowledge_import::features::importer::CrossReference::FileRef {
                target_path, ..
            } => {
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
            knowledge_import::features::importer::CrossReference::WikilinkRef {
                target_name,
                ..
            }
            | knowledge_import::features::importer::CrossReference::MentionRef { target_name } => {
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
            knowledge_import::features::importer::CrossReference::SectionRef {
                target_path,
                ..
            } => {
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
        if let knowledge_import::features::importer::CrossReference::UrlRef { url, link_text } =
            cross_ref
        {
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
            let existing =
                RelationshipRepository::find_by_source_and_target(store, entity.id, target_id)
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

            if let knowledge_import::features::importer::CrossReference::SectionRef {
                section,
                ..
            } = cross_ref
            {
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

    // Generate embedding if AI provider is configured and entity has content
    if let (Some(ai), Some(vs)) = (ai_adapter, vector_store) {
        if !content.is_empty() {
            match ai.embed(&content).await {
                Ok(vector) => {
                    let metadata = knowledge_core::ports::VectorMetadata {
                        model: ai.model_name().to_string(),
                        entity_type: entity.entity_type.to_string(),
                        title: title.clone(),
                    };
                    let _ = knowledge_core::ports::VectorStore::upsert(
                        vs,
                        &entity.id.to_string(),
                        &vector,
                        Some(metadata),
                    )
                    .await;
                }
                Err(e) => {
                    eprintln!("  Warning: embedding generation failed: {}", e);
                }
            }
        }
    }

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

#[allow(clippy::too_many_arguments)]
async fn cmd_search(
    store: Arc<SqliteStore>,
    vector_store: Arc<SqliteVectorStore>,
    ai_adapter: &dyn knowledge_core::ports::AiAdapter,
    query: &str,
    entity_type: Option<&str>,
    tag: Option<&str>,
    semantic: bool,
    hybrid: bool,
) -> Result<(), Box<dyn std::error::Error>> {
    // Validate flags
    if semantic && hybrid {
        eprintln!("Error: --semantic and --hybrid are mutually exclusive. Use one or the other.");
        std::process::exit(1);
    }

    // Determine search mode
    let keyword_results = if !semantic {
        // Keyword search (default, or part of hybrid)
        let search_query = SearchQuery {
            query: query.to_string(),
            entity_type: entity_type.map(String::from),
            tag: tag.map(String::from),
        };
        SearchIndex::search(store.as_ref(), &search_query).await?
    } else {
        vec![]
    };

    let semantic_results = if semantic || hybrid {
        // Generate query embedding and search vector store
        let query_vec = match ai_adapter.embed(query).await {
            Ok(v) => v,
            Err(e) => {
                eprintln!("Warning: Embedding generation failed: {}", e);
                return Ok(());
            }
        };

        if query_vec.is_empty() {
            Vec::<knowledge_core::ports::VectorResult>::new()
        } else {
            let filter = entity_type.map(|t| knowledge_core::ports::VectorFilter {
                entity_types: Some(vec![knowledge_core::features::entity::EntityType::new(t)]),
                tags: None,
                min_score: None,
            });

            match knowledge_core::ports::VectorStore::search(&*vector_store, &query_vec, 20, filter)
                .await
            {
                Ok(results) => results,
                Err(e) => {
                    eprintln!("Warning: Vector search failed: {}", e);
                    Vec::new()
                }
            }
        }
    } else {
        Vec::<knowledge_core::ports::VectorResult>::new()
    };

    // Combine results based on mode
    let results: Vec<SearchResult> =
        if hybrid && !keyword_results.is_empty() && !semantic_results.is_empty() {
            // Hybrid: use RRF fusion
            let fused = knowledge_derivation::features::search::hybrid::reciprocal_rank_fusion(
                &keyword_results,
                &semantic_results,
                60,
            );
            // Convert FusedResult back to SearchResult for display
            fused
                .into_iter()
                .filter_map(|f| {
                    uuid::Uuid::parse_str(&f.entity_id)
                        .ok()
                        .map(|id| SearchResult {
                            entity_id: id,
                            score: f.score,
                            confidence: None,
                            snippet: None,
                        })
                })
                .collect()
        } else if semantic {
            // Semantic-only: use semantic results (currently empty without provider)
            // Convert VectorResult to SearchResult for display
            semantic_results
                .into_iter()
                .filter_map(|v| {
                    uuid::Uuid::parse_str(&v.entity_id)
                        .ok()
                        .map(|id| SearchResult {
                            entity_id: id,
                            score: v.score,
                            confidence: None,
                            snippet: None,
                        })
                })
                .collect()
        } else {
            // Keyword-only (default)
            keyword_results
        };

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

async fn cmd_get(store: Arc<SqliteStore>, id_str: &str) -> Result<(), Box<dyn std::error::Error>> {
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

        println!(
            "  [{:?}] {} -- \"{}\"",
            entity.entity_type, entity.id, title
        );
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

    // Notify views of archive so they invalidate cached state
    notify_event(&store, &event).await;

    println!("Archived: Entity {} ({:?})", entity.id, entity.entity_type);
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

    // Notify views of restore so they invalidate cached state
    notify_event(&store, &event).await;

    println!("Restored: Entity {} ({:?})", entity.id, entity.entity_type);
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

async fn cmd_traverse(
    store: Arc<SqliteStore>,
    entity_id_str: &str,
    max_depth: u32,
    direction_str: &str,
    rel_type_str: Option<&str>,
    entity_type_str: Option<&str>,
) -> Result<(), Box<dyn std::error::Error>> {
    let entity_id = uuid::Uuid::parse_str(entity_id_str)?;

    let direction = match direction_str {
        "outgoing" => TraversalDirection::Outgoing,
        "incoming" => TraversalDirection::Incoming,
        "both" => TraversalDirection::Both,
        other => {
            eprintln!(
                "Invalid direction '{}'. Must be one of: outgoing, incoming, both",
                other
            );
            std::process::exit(1);
        }
    };

    let relationship_type = rel_type_str.map(|_s| RelationshipType::References); // Single variant for now

    let entity_type = entity_type_str.map(knowledge_core::features::entity::EntityType::new);

    let query = TraversalQuery {
        start_id: entity_id,
        direction,
        max_depth: Some(max_depth),
        max_results: None,
        relationship_type,
        entity_type_filter: entity_type,
    };

    let config = TraversalConfig::default();
    let results = TraversalPort::traverse(store.as_ref(), &query, &config).await?;

    if results.is_empty() {
        println!("No entities found within {} hops.", max_depth);
        return Ok(());
    }

    // Get start entity info for display
    let start_entity = EntityRepository::get(store.as_ref(), entity_id).await?;
    let start_title = if let Some(ref entity) = start_entity {
        let components = ComponentRepository::get(store.as_ref(), entity.id).await?;
        components
            .iter()
            .find(|c| c.component_type == ComponentType::Title)
            .and_then(|c| c.data.as_str().map(String::from))
            .unwrap_or_else(|| "Untitled".to_string())
    } else {
        "Unknown".to_string()
    };
    let start_type = start_entity
        .as_ref()
        .map(|e| format!("{:?}", e.entity_type))
        .unwrap_or_else(|| "Unknown".to_string());

    println!("Entity: \"{}\" ({})", start_title, start_type);

    // Group results by depth
    let mut by_depth: std::collections::BTreeMap<u32, Vec<_>> = std::collections::BTreeMap::new();
    for result in &results {
        by_depth.entry(result.depth).or_default().push(result);
    }

    for (depth, depth_results) in &by_depth {
        println!("  Hop {}:", depth);
        for result in depth_results {
            let target_id = *result.path.last().unwrap();
            let target_entity = EntityRepository::get(store.as_ref(), target_id).await?;
            let target_title = if let Some(ref entity) = target_entity {
                let components = ComponentRepository::get(store.as_ref(), entity.id).await?;
                components
                    .iter()
                    .find(|c| c.component_type == ComponentType::Title)
                    .and_then(|c| c.data.as_str().map(String::from))
                    .unwrap_or_else(|| "Untitled".to_string())
            } else {
                "Unknown".to_string()
            };
            let target_type = target_entity
                .as_ref()
                .map(|e| format!("{:?}", e.entity_type))
                .unwrap_or_else(|| "Unknown".to_string());

            if let Some(edge) = result.edges.first() {
                println!(
                    "    -> {:?} -> \"{}\" ({})",
                    edge.relationship_type, target_title, target_type
                );
            } else {
                println!("    -> \"{}\" ({})", target_title, target_type);
            }
        }
    }

    println!(
        "\nTotal: {} entities within {} hops",
        results.len(),
        max_depth
    );

    Ok(())
}

async fn cmd_view(
    store: Arc<SqliteStore>,
    view_type: ViewCommands,
) -> Result<(), Box<dyn std::error::Error>> {
    let wrapper = StoreWrapper(store);

    // Build the view filter based on the subcommand
    let (view_name, filter) = match &view_type {
        ViewCommands::Tree { r#type } => {
            let filter = ViewFilter {
                entity_types: r#type.as_ref().map(|t| vec![EntityType::new(t)]),
                ..Default::default()
            };
            ("tree", filter)
        }
        ViewCommands::Graph {
            from,
            depth,
            r#type,
        } => {
            let filter = ViewFilter {
                start_entity_id: from.clone(),
                max_depth: Some(*depth),
                entity_types: r#type.as_ref().map(|t| vec![EntityType::new(t)]),
                ..Default::default()
            };
            ("graph", filter)
        }
        ViewCommands::Table {
            sort,
            filter: search,
            r#type,
        } => {
            let filter = ViewFilter {
                sort_by: sort.clone(),
                sort_order: Some(knowledge_core::ports::SortOrder::Asc),
                search_query: search.clone(),
                entity_types: r#type.as_ref().map(|t| vec![EntityType::new(t)]),
                ..Default::default()
            };
            ("table", filter)
        }
        ViewCommands::Timeline { r#type } => {
            let filter = ViewFilter {
                entity_types: r#type.as_ref().map(|t| vec![EntityType::new(t)]),
                ..Default::default()
            };
            ("timeline", filter)
        }
    };

    // Create the appropriate view adapter
    let adapter: Box<dyn ViewAdapter> = match view_name {
        "tree" => Box::new(TreeViewAdapter::new(
            Box::new(StoreWrapper(wrapper.0.clone())),
            Box::new(StoreWrapper(wrapper.0.clone())),
            Some(Box::new(StoreWrapper(wrapper.0.clone()))),
        )),
        "graph" => Box::new(GraphViewAdapter::new(
            Box::new(StoreWrapper(wrapper.0.clone())),
            Box::new(StoreWrapper(wrapper.0.clone())),
            Box::new(StoreWrapper(wrapper.0.clone())),
            Box::new(StoreWrapper(wrapper.0.clone())),
        )),
        "table" => Box::new(TableViewAdapter::new(
            Box::new(StoreWrapper(wrapper.0.clone())),
            Box::new(StoreWrapper(wrapper.0.clone())),
        )),
        "timeline" => Box::new(TimelineViewAdapter::new(
            Box::new(StoreWrapper(wrapper.0.clone())),
            Box::new(StoreWrapper(wrapper.0.clone())),
        )),
        _ => unreachable!(),
    };

    // Render the view
    let output = adapter.render(&filter).await?;

    // Print the output
    match output {
        ViewOutput::Tree(tree) => {
            if tree.roots.is_empty() {
                println!("No entities found.");
                return Ok(());
            }
            println!("Knowledge Graph (Tree View)\n");
            for root in &tree.roots {
                println!("  {} ({})", root.label, root.children.len());
                for child in &root.children {
                    println!("    {}", child.label);
                }
            }
        }
        ViewOutput::Graph(graph) => {
            if graph.nodes.is_empty() {
                println!("No entities found.");
                return Ok(());
            }
            println!("Knowledge Graph\n");
            println!("Nodes ({}):", graph.nodes.len());
            for node in &graph.nodes {
                println!("  [{}] {} ({})", node.node_type, node.label, node.entity.id);
            }
            if !graph.edges.is_empty() {
                println!("\nEdges ({}):", graph.edges.len());
                for edge in &graph.edges {
                    println!(
                        "  {} --{}--> {}",
                        edge.source_id, edge.label, edge.target_id
                    );
                }
            }
        }
        ViewOutput::Table(table) => {
            if table.rows.is_empty() {
                println!("No entities found.");
                return Ok(());
            }
            // Print header
            let headers: Vec<&str> = table.columns.iter().map(|c| c.name.as_str()).collect();
            println!("{}", headers.join(" | "));
            println!("{}", "-".repeat(80));
            for row in &table.rows {
                println!("{}", row.cells.join(" | "));
            }
        }
        ViewOutput::Timeline(timeline) => {
            if timeline.entries.is_empty() {
                println!("No entities found.");
                return Ok(());
            }
            println!("Timeline\n");
            for entry in &timeline.entries {
                println!(
                    "  [{}] {} -- \"{}\"",
                    entry.entity.entity_type, entry.timestamp, entry.label
                );
            }
        }
    }

    Ok(())
}

async fn cmd_collection(
    store: &Arc<SqliteStore>,
    action: CollectionCommands,
) -> Result<(), Box<dyn std::error::Error>> {
    let wrapper = StoreWrapper(store.clone());

    match action {
        CollectionCommands::Create { name, description } => {
            let now = chrono::Utc::now();
            let collection = Collection {
                id: Uuid::new_v4(),
                name: name.clone(),
                description,
                created_at: now,
                updated_at: now,
            };
            let created = CollectionRepository::create(&wrapper, collection).await?;
            println!("Collection created: {} ({})", created.name, created.id);
        }
        CollectionCommands::List => {
            let collections = CollectionRepository::list(&wrapper).await?;
            if collections.is_empty() {
                println!("No collections found.");
                return Ok(());
            }
            println!("Collections ({}):\n", collections.len());
            for c in &collections {
                let desc = c.description.as_deref().unwrap_or("(no description)");
                println!("  {} ({}) — {}", c.name, c.id, desc);
            }
        }
        CollectionCommands::Add {
            collection_id,
            entity_id,
        } => {
            let coll_id = Uuid::parse_str(&collection_id)?;
            let ent_id = Uuid::parse_str(&entity_id)?;

            // Verify both exist
            match CollectionRepository::get(&wrapper, coll_id).await? {
                Some(c) => println!("Adding entity {} to collection '{}'...", ent_id, c.name),
                None => {
                    eprintln!("Error: Collection {} not found.", coll_id);
                    return Ok(());
                }
            }
            match EntityRepository::get(&wrapper, ent_id).await? {
                Some(e) => println!("  Entity [{}] ({})", e.entity_type, e.id),
                None => {
                    eprintln!("Error: Entity {} not found.", ent_id);
                    return Ok(());
                }
            }

            match CollectionRepository::add_member(&wrapper, coll_id, ent_id).await {
                Ok(()) => println!("Entity added to collection."),
                Err(StorageError::Internal(msg)) => {
                    if msg.contains("already") {
                        eprintln!("Entity is already a member of this collection.");
                    } else {
                        return Err(StorageError::Internal(msg).into());
                    }
                }
                Err(e) => return Err(e.into()),
            }
        }
        CollectionCommands::Remove {
            collection_id,
            entity_id,
        } => {
            let coll_id = Uuid::parse_str(&collection_id)?;
            let ent_id = Uuid::parse_str(&entity_id)?;
            CollectionRepository::remove_member(&wrapper, coll_id, ent_id).await?;
            println!("Entity removed from collection.");
        }
        CollectionCommands::Members { collection_id } => {
            let coll_id = Uuid::parse_str(&collection_id)?;
            match CollectionRepository::get(&wrapper, coll_id).await? {
                Some(c) => {
                    let members = CollectionRepository::get_members(&wrapper, coll_id).await?;
                    if members.is_empty() {
                        println!("Collection '{}' is empty.", c.name);
                        return Ok(());
                    }
                    println!("Collection '{}' ({} members):\n", c.name, members.len());
                    for e in &members {
                        let components = ComponentRepository::get(&wrapper, e.id).await?;
                        let title = components
                            .iter()
                            .find(|c| c.component_type == ComponentType::Title)
                            .and_then(|c| c.data.as_str().map(String::from))
                            .unwrap_or_else(|| "Untitled".to_string());
                        println!("  {} [{}] ({})", title, e.entity_type, e.id);
                    }
                }
                None => {
                    eprintln!("Error: Collection {} not found.", coll_id);
                    return Ok(());
                }
            }
        }
        CollectionCommands::Delete { collection_id } => {
            let coll_id = Uuid::parse_str(&collection_id)?;
            match CollectionRepository::get(&wrapper, coll_id).await? {
                Some(c) => {
                    CollectionRepository::delete(&wrapper, coll_id).await?;
                    println!("Collection deleted: {} ({})", c.name, c.id);
                }
                None => {
                    eprintln!("Error: Collection {} not found.", coll_id);
                    return Ok(());
                }
            }
        }
    }

    Ok(())
}

async fn cmd_conversation(
    store: &Arc<SqliteStore>,
    action: ConversationCommands,
) -> Result<(), Box<dyn std::error::Error>> {
    match action {
        ConversationCommands::List => {
            let conversations = ConversationRepository::list_conversations(store.as_ref()).await?;
            if conversations.is_empty() {
                println!("No conversations found.");
                return Ok(());
            }
            println!("Conversations ({}):\n", conversations.len());
            for c in &conversations {
                let preview = c.last_message_preview.as_deref().unwrap_or("(no messages)");
                let last_at = c
                    .last_message_at
                    .map(|d| d.to_rfc3339())
                    .unwrap_or_else(|| "never".to_string());
                println!(
                    "  {} — \"{}\" ({} messages, last: {})",
                    c.id, c.title, c.message_count, last_at
                );
                if c.message_count > 0 {
                    println!("    Preview: {}", preview);
                }
            }
        }
        ConversationCommands::Get { id } => {
            let conv_id = Uuid::parse_str(&id)?;
            let conv = ConversationRepository::get_conversation(store.as_ref(), conv_id)
                .await?
                .ok_or_else(|| format!("Conversation {} not found", conv_id))?;
            println!("# {} ({})", conv.title, conv.id);
            println!("Created: {}", conv.created_at);
            println!("Messages ({}):\n", conv.messages.len());
            for msg in &conv.messages {
                let role_str = match msg.role {
                    knowledge_core::ports::MessageRole::System => "system",
                    knowledge_core::ports::MessageRole::User => "user",
                    knowledge_core::ports::MessageRole::Assistant => "assistant",
                };
                println!("[{}] {}", role_str, msg.text);
                if !msg.entity_refs.is_empty() {
                    println!(
                        "  References: {}",
                        msg.entity_refs
                            .iter()
                            .map(|id| id.to_string())
                            .collect::<Vec<_>>()
                            .join(", ")
                    );
                }
                println!();
            }
        }
        ConversationCommands::Rename { id, title } => {
            let conv_id = Uuid::parse_str(&id)?;
            ConversationRepository::rename_conversation(store.as_ref(), conv_id, &title).await?;
            println!("Conversation {} renamed to \"{}\".", conv_id, title);
        }
        ConversationCommands::Delete { id } => {
            let conv_id = Uuid::parse_str(&id)?;
            ConversationRepository::archive_conversation(store.as_ref(), conv_id).await?;
            println!("Conversation {} archived.", conv_id);
        }
    }
    Ok(())
}

async fn cmd_plugin_list() -> Result<(), Box<dyn std::error::Error>> {
    use knowledge_plugin::loader::discover_plugins;
    use knowledge_plugin::registry::built_in_plugins;

    let mut registry = built_in_plugins();

    let dir = plugin_dir();
    if let Ok(discovered) = discover_plugins(&dir) {
        for d in &discovered {
            registry.register_plugin(Box::new(StubPlugin {
                manifest: d.manifest.clone(),
            }));
        }
    }

    let plugins = registry.list_plugins();

    if plugins.is_empty() {
        println!("No plugins loaded.");
        return Ok(());
    }

    println!("Plugins ({} loaded):\n", plugins.len());
    for plugin in &plugins {
        let caps_str = if plugin.name.contains("import") {
            "[importer]".to_string()
        } else {
            "[unknown]".to_string()
        };

        println!("  {} v{}    {}", plugin.name, plugin.version, caps_str);
    }

    Ok(())
}

struct StubPlugin {
    manifest: knowledge_core::ports::PluginManifest,
}

impl knowledge_core::ports::Plugin for StubPlugin {
    fn manifest(&self) -> &knowledge_core::ports::PluginManifest {
        &self.manifest
    }
    fn activate(&self) -> Result<(), knowledge_core::ports::PluginError> {
        Ok(())
    }
    fn deactivate(&self) -> Result<(), knowledge_core::ports::PluginError> {
        Ok(())
    }
}

async fn cmd_plugin_info(name: &str) -> Result<(), Box<dyn std::error::Error>> {
    use knowledge_plugin::loader::discover_plugins;
    use knowledge_plugin::registry::built_in_plugins;

    let mut registry = built_in_plugins();

    let dir = plugin_dir();
    if let Ok(discovered) = discover_plugins(&dir) {
        for d in &discovered {
            registry.register_plugin(Box::new(StubPlugin {
                manifest: d.manifest.clone(),
            }));
        }
    }

    let plugins = registry.list_plugins();
    let plugin = plugins.iter().find(|p| p.name == name);

    match plugin {
        Some(info) => {
            println!("Plugin: {}", info.name);
            println!("  Version:     {}", info.version);
            println!("  Description: {}", info.description);
            if !info.capabilities.is_empty() {
                println!("  Capabilities:");
                for cap in &info.capabilities {
                    println!("    - {}", cap);
                }
            }
        }
        None => {
            eprintln!("Plugin '{}' not found.", name);
            std::process::exit(1);
        }
    }

    Ok(())
}

fn plugin_dir() -> PathBuf {
    if let Ok(val) = std::env::var("KOS_PLUGIN_DIR") {
        PathBuf::from(val)
    } else {
        let mut path = dirs::home_dir().unwrap_or_else(|| PathBuf::from("."));
        path.push(".knowledge-os");
        path.push("plugins");
        path
    }
}

async fn cmd_plugin_install(source: PathBuf) -> Result<(), Box<dyn std::error::Error>> {
    use knowledge_plugin::manifest::parse_manifest_file;

    let manifest_path = source.join("plugin.toml");
    if !manifest_path.exists() {
        eprintln!("Error: No plugin.toml found in '{}'.", source.display());
        std::process::exit(1);
    }

    let manifest = parse_manifest_file(&manifest_path).map_err(|e| {
        eprintln!("Error: Invalid plugin manifest: {}", e);
        e
    })?;

    let dest = plugin_dir().join(&manifest.name);
    if dest.exists() {
        eprintln!(
            "Error: Plugin '{}' is already installed. Uninstall it first.",
            manifest.name
        );
        std::process::exit(1);
    }

    std::fs::create_dir_all(&dest).map_err(|e| {
        eprintln!(
            "Error: Could not create plugin directory '{}': {}",
            dest.display(),
            e
        );
        e
    })?;

    copy_dir_recursive(&source, &dest).map_err(|e| {
        eprintln!(
            "Error: Could not copy plugin to '{}': {}",
            dest.display(),
            e
        );
        e
    })?;

    println!(
        "Plugin '{}' v{} installed to '{}'.",
        manifest.name,
        manifest.version,
        dest.display()
    );

    Ok(())
}

async fn cmd_plugin_uninstall(name: &str) -> Result<(), Box<dyn std::error::Error>> {
    let dest = plugin_dir().join(name);
    if !dest.exists() {
        eprintln!("Error: Plugin '{}' is not installed.", name);
        std::process::exit(1);
    }

    let manifest_path = dest.join("plugin.toml");
    let display_name = if manifest_path.exists() {
        match knowledge_plugin::manifest::parse_manifest_file(&manifest_path) {
            Ok(m) => format!("{} v{}", m.name, m.version),
            Err(_) => name.to_string(),
        }
    } else {
        name.to_string()
    };

    std::fs::remove_dir_all(&dest).map_err(|e| {
        eprintln!(
            "Error: Could not remove plugin directory '{}': {}",
            dest.display(),
            e
        );
        e
    })?;

    println!("Plugin '{}' uninstalled.", display_name);

    Ok(())
}

fn copy_dir_recursive(src: &Path, dst: &Path) -> std::io::Result<()> {
    std::fs::create_dir_all(dst)?;
    for entry in std::fs::read_dir(src)? {
        let entry = entry?;
        let ty = entry.file_type()?;
        if ty.is_dir() {
            copy_dir_recursive(&entry.path(), &dst.join(entry.file_name()))?;
        } else {
            std::fs::copy(entry.path(), dst.join(entry.file_name()))?;
        }
    }
    Ok(())
}
