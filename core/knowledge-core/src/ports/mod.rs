use std::collections::HashMap;

use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::features::component::Component;
use crate::features::entity::{Entity, EntityType};
use crate::features::relationship::{Relationship, RelationshipType};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EntityVersion {
    pub entity_id: Uuid,
    pub version: i64,
    pub snapshot: serde_json::Value,
    pub created_at: chrono::DateTime<chrono::Utc>,
}

#[async_trait]
pub trait EntityRepository: Send + Sync {
    async fn get(&self, id: Uuid) -> Result<Option<Entity>, StorageError>;
    async fn save(&self, entity: &Entity) -> Result<(), StorageError>;
    async fn delete(&self, id: Uuid) -> Result<(), StorageError>;
    async fn list(&self) -> Result<Vec<Entity>, StorageError>;
    async fn find_by_type(&self, entity_type: &str) -> Result<Vec<Entity>, StorageError>;
    async fn find_by_title(&self, title: &str) -> Result<Vec<Entity>, StorageError>;
    async fn increment_version(&self, id: Uuid) -> Result<(), StorageError>;
    async fn find_by_component_type(
        &self,
        component_type: &str,
    ) -> Result<Vec<Entity>, StorageError>;
    async fn find_by_tag(&self, tag: &str) -> Result<Vec<Entity>, StorageError>;
    async fn get_version_history(
        &self,
        entity_id: Uuid,
    ) -> Result<Vec<EntityVersion>, StorageError>;
}

#[async_trait]
pub trait RelationshipRepository: Send + Sync {
    async fn get(&self, id: Uuid) -> Result<Option<Relationship>, StorageError>;
    async fn save(&self, relationship: &Relationship) -> Result<(), StorageError>;
    async fn update(&self, relationship: &Relationship) -> Result<(), StorageError>;
    async fn delete(&self, id: Uuid) -> Result<(), StorageError>;
    async fn by_source(&self, source_id: Uuid) -> Result<Vec<Relationship>, StorageError>;
    async fn by_target(&self, target_id: Uuid) -> Result<Vec<Relationship>, StorageError>;
    async fn find_by_source_and_target(
        &self,
        source_id: Uuid,
        target_id: Uuid,
    ) -> Result<Option<Relationship>, StorageError>;
    async fn find_by_type(
        &self,
        relationship_type: &str,
    ) -> Result<Vec<Relationship>, StorageError>;
}

#[async_trait]
pub trait ComponentRepository: Send + Sync {
    async fn get(&self, entity_id: Uuid) -> Result<Vec<Component>, StorageError>;
    async fn save(&self, component: &Component) -> Result<(), StorageError>;
    async fn delete(&self, id: Uuid) -> Result<(), StorageError>;
    async fn find_by_type(
        &self,
        entity_id: Uuid,
        component_type: &str,
    ) -> Result<Vec<Component>, StorageError>;
    async fn update_data(&self, id: Uuid, data: serde_json::Value) -> Result<(), StorageError>;
    async fn find_by_component_data(
        &self,
        component_type: &str,
        json_path: &str,
        value: &str,
    ) -> Result<Vec<Component>, StorageError>;
    async fn delete_by_entity(&self, entity_id: Uuid) -> Result<(), StorageError>;
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SearchQuery {
    pub query: String,
    pub entity_type: Option<String>,
    pub tag: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SearchResult {
    pub entity_id: Uuid,
    pub score: f64,
    pub confidence: Option<f64>,
    pub snippet: Option<String>,
}

#[async_trait]
pub trait SearchIndex: Send + Sync {
    async fn index_entity(
        &self,
        entity: &Entity,
        components: &[Component],
    ) -> Result<(), StorageError>;
    async fn remove_entity(&self, entity_id: Uuid) -> Result<(), StorageError>;
    async fn search(&self, query: &SearchQuery) -> Result<Vec<SearchResult>, StorageError>;
    async fn rebuild(&self, entities: &[(Entity, Vec<Component>)]) -> Result<(), StorageError>;
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Event {
    pub id: Uuid,
    pub event_type: EventType,
    pub entity_id: Uuid,
    pub timestamp: chrono::DateTime<chrono::Utc>,
    pub data: serde_json::Value,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum EventType {
    EntityCreated,
    EntityUpdated,
    EntityArchived,
    EntityRestored,
    EntityResolved,
    ComponentAdded,
    ComponentUpdated,
    ComponentRemoved,
    RelationshipCreated,
    RelationshipArchived,
}

#[async_trait]
pub trait EventLog: Send + Sync {
    async fn append(&self, event: &Event) -> Result<(), StorageError>;
    async fn list_by_entity(&self, entity_id: Uuid) -> Result<Vec<Event>, StorageError>;
}

#[derive(Debug, thiserror::Error)]
pub enum StorageError {
    #[error("not found")]
    NotFound,
    #[error("storage error: {0}")]
    Internal(String),
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ResolutionCandidate {
    pub entity_id: Uuid,
    pub confidence: f64,
    pub reason: String,
    /// Component scores for composite resolution (None for non-composite strategies)
    pub title_score: Option<f64>,
    pub content_score: Option<f64>,
    pub metadata_score: Option<f64>,
    pub structural_score: Option<f64>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MergeAuditEntry {
    pub id: Uuid,
    pub source_id: Uuid,
    pub source_title: String,
    pub target_id: Uuid,
    pub target_title: String,
    pub strategy: String,
    pub confidence: f64,
    pub timestamp: chrono::DateTime<chrono::Utc>,
    pub reason: String,
    pub snapshot: Option<String>, // JSON snapshot of pre-merge state for undo
}

#[async_trait]
pub trait EntityResolver: Send + Sync {
    async fn find_candidates(
        &self,
        entity: &Entity,
        title: &str,
        content: Option<&str>,
    ) -> Result<Vec<ResolutionCandidate>, StorageError>;
    async fn merge(
        &self,
        canonical_id: Uuid,
        duplicate_id: Uuid,
        confidence: f64,
    ) -> Result<(), StorageError>;
    async fn log_merge(&self, entry: &MergeAuditEntry) -> Result<(), StorageError>;
    async fn undo_merge(&self, merge_id: Uuid) -> Result<(), StorageError>;
    async fn get_merge_history(
        &self,
        entity_id: Uuid,
    ) -> Result<Vec<MergeAuditEntry>, StorageError>;
    async fn get_all_merge_history(&self) -> Result<Vec<MergeAuditEntry>, StorageError>;
}

#[async_trait]
pub trait TransactionalWrite: Send + Sync {
    async fn save_entity_with_components(
        &self,
        entity: &Entity,
        components: &[Component],
        event: &Event,
    ) -> Result<(), StorageError>;

    async fn update_entity_with_components(
        &self,
        entity: &Entity,
        components: &[Component],
        event: &Event,
    ) -> Result<(), StorageError>;
}

// =============================================================================
// Graph Traversal
// =============================================================================

/// Direction for graph traversal queries.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum TraversalDirection {
    /// Follow outgoing edges (source -> target).
    Outgoing,
    /// Follow incoming edges (target -> source).
    Incoming,
    /// Follow edges in both directions.
    Both,
}

/// A query parameterizing a graph traversal from a start entity.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TraversalQuery {
    /// The entity to begin traversal from.
    pub start_id: Uuid,
    /// Which directions to follow.
    pub direction: TraversalDirection,
    /// Maximum depth (hops) to traverse. `None` means use config default.
    pub max_depth: Option<u32>,
    /// Maximum number of result paths. `None` means use config default.
    pub max_results: Option<usize>,
    /// If set, only follow edges with this relationship type.
    pub relationship_type: Option<RelationshipType>,
    /// If set, only include entities of this type in results.
    pub entity_type_filter: Option<EntityType>,
}

/// Default configuration for traversal operations.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TraversalConfig {
    /// Default maximum depth when query does not specify one.
    pub default_max_depth: u32,
    /// Default maximum number of results when query does not specify one.
    pub default_max_results: usize,
}

impl Default for TraversalConfig {
    fn default() -> Self {
        Self {
            default_max_depth: 3,
            default_max_results: 100,
        }
    }
}

/// A single edge encountered during traversal.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TraversalEdge {
    /// Source entity of this edge.
    pub source_id: Uuid,
    /// Target entity of this edge.
    pub target_id: Uuid,
    /// Type of the relationship.
    pub relationship_type: RelationshipType,
}

/// One result path from the traversal.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TraversalResult {
    /// Ordered entity IDs from start to this result.
    pub path: Vec<Uuid>,
    /// Edges that were followed to produce this path.
    pub edges: Vec<TraversalEdge>,
    /// Depth (hop count) of this result.
    pub depth: u32,
}

/// Errors that can occur during graph traversal.
#[derive(Debug, thiserror::Error)]
pub enum TraversalError {
    /// The start entity does not exist or is inactive.
    #[error("Start entity not found: {0}")]
    StartNotFound(Uuid),
    /// The traversal limit was exceeded.
    #[error("Traversal limit exceeded: {limit} results")]
    LimitExceeded { limit: usize },
    /// An error occurred in the storage layer.
    #[error("Storage error: {0}")]
    Storage(#[from] StorageError),
}

/// Port for executing graph traversal queries.
#[async_trait]
pub trait TraversalPort: Send + Sync {
    /// Traverse the entity graph starting from the entity specified in `query`.
    ///
    /// Returns all reachable entities within the configured depth and result limits.
    ///
    /// # Errors
    ///
    /// Returns `TraversalError::StartNotFound` if the start entity does not exist.
    /// Returns `TraversalError::LimitExceeded` if the traversal hits the result limit.
    /// Returns `TraversalError::Storage` on storage layer failures.
    async fn traverse(
        &self,
        query: &TraversalQuery,
        config: &TraversalConfig,
    ) -> Result<Vec<TraversalResult>, TraversalError>;
}

// =============================================================================
// View Projections
// =============================================================================

/// Sort direction for view ordering.
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq)]
pub enum SortOrder {
    Asc,
    Desc,
}

/// Filtering and pagination parameters for view rendering.
#[derive(Debug, Clone, Default)]
pub struct ViewFilter {
    /// Include only entities of these types.
    pub entity_types: Option<Vec<EntityType>>,
    /// Include only entities with any of these tags.
    pub tags: Option<Vec<String>>,
    /// Include only relationships of these types.
    pub relationship_types: Option<Vec<RelationshipType>>,
    /// Maximum traversal depth for graph views.
    pub max_depth: Option<u32>,
    /// Maximum number of results returned.
    pub max_results: Option<usize>,
    /// Entity ID to start graph traversal from.
    pub start_entity_id: Option<String>,
    /// Column or field to sort by.
    pub sort_by: Option<String>,
    /// Sort direction.
    pub sort_order: Option<SortOrder>,
    /// Free-text search query.
    pub search_query: Option<String>,
}

/// Output produced by a view adapter after rendering.
#[derive(Debug, Clone)]
pub enum ViewOutput {
    Tree(TreeData),
    Graph(GraphData),
    Table(TableData),
    Timeline(TimelineData),
}

/// Hierarchical tree representation for tree view.
#[derive(Debug, Clone)]
pub struct TreeData {
    pub roots: Vec<TreeNode>,
}

/// A single node in a tree, containing an entity, a display label, and its children.
#[derive(Debug, Clone)]
pub struct TreeNode {
    pub entity: Entity,
    pub label: String,
    pub children: Vec<TreeNode>,
}

/// Graph representation with nodes and edges for graph view.
#[derive(Debug, Clone)]
pub struct GraphData {
    pub nodes: Vec<GraphNode>,
    pub edges: Vec<GraphEdge>,
}

/// A labeled node in a graph, derived from an entity.
#[derive(Debug, Clone)]
pub struct GraphNode {
    pub entity: Entity,
    pub label: String,
    pub node_type: String,
}

/// A directed edge between two graph nodes.
#[derive(Debug, Clone)]
pub struct GraphEdge {
    pub source_id: String,
    pub target_id: String,
    pub relationship_type: String,
    pub label: String,
}

/// Tabular representation with sortable columns and rows.
#[derive(Debug, Clone)]
pub struct TableData {
    pub columns: Vec<TableColumn>,
    pub rows: Vec<TableRow>,
}

/// Column definition for table view.
#[derive(Debug, Clone)]
pub struct TableColumn {
    pub name: String,
    pub sortable: bool,
}

/// A single row in a table.
#[derive(Debug, Clone)]
pub struct TableRow {
    pub cells: Vec<String>,
}

/// Temporal representation with entries ordered by timestamp.
#[derive(Debug, Clone)]
pub struct TimelineData {
    pub entries: Vec<TimelineEntry>,
}

/// A single entry in a timeline view.
#[derive(Debug, Clone)]
pub struct TimelineEntry {
    pub entity: Entity,
    pub timestamp: String,
    pub label: String,
}

/// Trait for rendering canonical data into a specific view projection.
///
/// Implementors transform repository data into a `ViewOutput` variant.
/// Views are invalidated and rebuilt when canonical data changes via `on_event`.
#[async_trait]
pub trait ViewAdapter: Send + Sync {
    /// Returns the name of this view (e.g., "tree", "graph").
    fn name(&self) -> &str;

    /// Render the view for the given filter parameters.
    ///
    /// # Errors
    ///
    /// Returns `StorageError` if the underlying repositories fail.
    async fn render(&self, filter: &ViewFilter) -> Result<ViewOutput, StorageError>;

    /// Called when a canonical event occurs. Views invalidate cached data here.
    ///
    /// # Errors
    ///
    /// Returns `StorageError` if event processing fails.
    async fn on_event(&self, event: &Event) -> Result<(), StorageError>;
}

/// Trait for notifying subscribers of canonical events.
///
/// This is a separate trait from `ViewAdapter` so non-view subsystems
/// (e.g., embedding pipeline re-triggering) can implement it without being views.
#[async_trait]
pub trait EventNotifier: Send + Sync {
    /// Notify all subscribers of a canonical event.
    ///
    /// # Errors
    ///
    /// Returns `StorageError` if any subscriber fails.
    async fn notify(&self, event: &Event) -> Result<(), StorageError>;
}

/// Registry of named view adapters. Dispatches render and event calls.
pub struct ViewRegistry {
    views: HashMap<String, Box<dyn ViewAdapter>>,
}

impl ViewRegistry {
    /// Creates an empty view registry.
    pub fn new() -> Self {
        Self {
            views: HashMap::new(),
        }
    }

    /// Registers a view adapter under its name.
    pub fn register(&mut self, view: Box<dyn ViewAdapter>) {
        let name = view.name().to_string();
        self.views.insert(name, view);
    }

    /// Renders a named view with the given filter parameters.
    ///
    /// # Errors
    ///
    /// Returns `StorageError::NotFound` if the view name is not registered.
    /// Returns `StorageError::Internal` if rendering fails.
    pub async fn render(
        &self,
        name: &str,
        filter: &ViewFilter,
    ) -> Result<ViewOutput, StorageError> {
        self.views
            .get(name)
            .ok_or_else(|| StorageError::Internal(format!("view '{}' not found", name)))?
            .render(filter)
            .await
    }

    /// Returns the names of all registered views.
    pub fn list_views(&self) -> Vec<String> {
        self.views.keys().cloned().collect()
    }
}

impl Default for ViewRegistry {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl EventNotifier for ViewRegistry {
    async fn notify(&self, event: &Event) -> Result<(), StorageError> {
        for view in self.views.values() {
            view.on_event(event).await?;
        }
        Ok(())
    }
}

// =============================================================================
// Collections
// =============================================================================

/// A curated group of entities.
///
/// Collections are first-class entities with many-to-many membership.
/// They are stored in dedicated tables and used by the tree view for
/// hierarchical grouping. Defined in ADR-0018.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Collection {
    pub id: Uuid,
    pub name: String,
    pub description: Option<String>,
    pub created_at: chrono::DateTime<chrono::Utc>,
    pub updated_at: chrono::DateTime<chrono::Utc>,
}

/// Repository for managing curated entity collections.
///
/// Supports CRUD operations on collections and many-to-many membership
/// management between collections and entities.
#[async_trait]
pub trait CollectionRepository: Send + Sync {
    /// Create a new collection.
    ///
    /// # Errors
    ///
    /// Returns `StorageError::Internal` if the storage layer fails.
    async fn create(&self, collection: Collection) -> Result<Collection, StorageError>;

    /// Get a collection by ID.
    ///
    /// # Errors
    ///
    /// Returns `StorageError::Internal` if the storage layer fails.
    async fn get(&self, id: Uuid) -> Result<Option<Collection>, StorageError>;

    /// Update an existing collection.
    ///
    /// # Errors
    ///
    /// Returns `StorageError::NotFound` if the collection does not exist.
    /// Returns `StorageError::Internal` if the storage layer fails.
    async fn update(&self, collection: Collection) -> Result<Collection, StorageError>;

    /// Delete a collection and its membership records.
    ///
    /// # Errors
    ///
    /// Returns `StorageError::Internal` if the storage layer fails.
    async fn delete(&self, id: Uuid) -> Result<(), StorageError>;

    /// List all collections.
    ///
    /// # Errors
    ///
    /// Returns `StorageError::Internal` if the storage layer fails.
    async fn list(&self) -> Result<Vec<Collection>, StorageError>;

    /// Add an entity to a collection.
    ///
    /// # Errors
    ///
    /// Returns `StorageError::Internal` if the storage layer fails.
    async fn add_member(&self, collection_id: Uuid, entity_id: Uuid) -> Result<(), StorageError>;

    /// Remove an entity from a collection.
    ///
    /// # Errors
    ///
    /// Returns `StorageError::Internal` if the storage layer fails.
    async fn remove_member(&self, collection_id: Uuid, entity_id: Uuid)
        -> Result<(), StorageError>;

    /// Get all entities belonging to a collection.
    ///
    /// # Errors
    ///
    /// Returns `StorageError::Internal` if the storage layer fails.
    async fn get_members(&self, collection_id: Uuid) -> Result<Vec<Entity>, StorageError>;

    /// Get all collections containing the given entity.
    ///
    /// # Errors
    ///
    /// Returns `StorageError::Internal` if the storage layer fails.
    async fn get_entity_collections(
        &self,
        entity_id: Uuid,
    ) -> Result<Vec<Collection>, StorageError>;

    /// Check if an entity is a member of a collection.
    ///
    /// # Errors
    ///
    /// Returns `StorageError::Internal` if the storage layer fails.
    async fn is_member(&self, collection_id: Uuid, entity_id: Uuid) -> Result<bool, StorageError>;
}

// =============================================================================
// Plugin System
// =============================================================================

/// Metadata for a plugin, declared in a TOML manifest.
///
/// Every plugin provides a manifest that describes its identity, version,
/// and optional configuration. The manifest is parsed at startup and
/// validated against the plugin's declared capabilities.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PluginManifest {
    /// Unique name identifying this plugin (e.g., "markdown-importer").
    pub name: String,
    /// Semantic version string (e.g., "0.1.0").
    pub version: String,
    /// Human-readable description of the plugin's purpose.
    pub description: String,
    /// Plugin author name.
    pub author: String,
    /// SPDX license identifier, if applicable.
    pub license: Option<String>,
    /// Priority for conflict resolution (lower = preferred, default 100).
    pub priority: Option<u32>,
}

impl PluginManifest {
    /// Effective priority value, defaulting to 100 when not set.
    pub fn effective_priority(&self) -> u32 {
        self.priority.unwrap_or(100)
    }
}

/// A capability declared by a plugin.
///
/// The capability registry routes requests to plugins based on their
/// declared capabilities. Each variant maps to a specific adapter trait.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum PluginCapability {
    /// Importer plugin handling specific file formats (e.g., "markdown", "pdf").
    Importer { formats: Vec<String> },
    /// Renderer plugin providing a named view projection.
    Renderer { name: String },
    /// AI provider plugin offering specific capabilities (e.g., "embedding", "chat").
    AiProvider { capabilities: Vec<String> },
    /// Vector store plugin with a named storage backend.
    VectorStore { name: String },
}

/// Core trait that all plugins must implement.
///
/// Plugins are in-process trait objects compiled into the binary.
/// They follow a lifecycle: discovery, registration, activation, execution, deactivation.
pub trait Plugin: Send + Sync {
    /// Return the manifest metadata for this plugin.
    fn manifest(&self) -> &PluginManifest;

    /// Activate the plugin. Called once at startup after registration.
    ///
    /// # Errors
    ///
    /// Returns `PluginError::ActivationFailed` if initialization fails.
    fn activate(&self) -> Result<(), PluginError>;

    /// Deactivate the plugin. Called once at shutdown.
    ///
    /// # Errors
    ///
    /// Returns `PluginError::ActivationFailed` if cleanup fails.
    fn deactivate(&self) -> Result<(), PluginError>;
}

/// Trait for types that provide plugin metadata.
///
/// Implement this alongside `ImportAdapter` (or any adapter trait) to make
/// your type usable as a plugin without writing wrapper structs.
/// The generic `PluginAdapter<T>` wrapper in `knowledge-import` bridges
/// `PluginMetadata` + `ImportAdapter` into the `Plugin` trait.
pub trait PluginMetadata {
    /// Return the manifest for this plugin.
    fn manifest(&self) -> PluginManifest;
}

/// Errors that can occur during plugin operations.
#[derive(Debug, thiserror::Error)]
pub enum PluginError {
    /// The requested plugin was not found in the registry.
    #[error("Plugin not found: {0}")]
    NotFound(String),

    /// Plugin initialization or cleanup failed.
    #[error("Plugin activation failed: {0}")]
    ActivationFailed(String),

    /// The plugin encountered an error during execution.
    #[error("Plugin execution failed: {0}")]
    ExecutionFailed(String),

    /// The plugin exceeded its allowed execution time.
    #[error("Plugin timeout: {0}")]
    Timeout(String),
}

// =============================================================================
// AI Adapter (refined in IP-004 D1)
// =============================================================================

/// Port for AI operations (embeddings, completions).
///
/// Refined in IP-004 D1 with `dimensions()`. Implementations produce
/// fixed-dimensionality embedding vectors from text content.
#[async_trait]
pub trait AiAdapter: Send + Sync {
    /// Generate an embedding vector for the given content.
    ///
    /// # Errors
    ///
    /// Returns `AiError::Provider` if the AI provider rejects the request.
    /// Returns `AiError::Network` if the network request fails.
    async fn embed(&self, content: &str) -> Result<Vec<f32>, AiError>;

    /// Return the name of the AI model used by this adapter.
    fn model_name(&self) -> &str;

    /// Return the dimensionality of embedding vectors produced by this adapter.
    fn dimensions(&self) -> usize;
}

/// Errors that can occur during AI operations.
#[derive(Debug, thiserror::Error)]
pub enum AiError {
    /// The AI provider returned an error.
    #[error("Provider error: {0}")]
    Provider(String),

    /// A network request to the AI provider failed.
    #[error("Network error: {0}")]
    Network(String),
}

// =============================================================================
// Vector Store (refined in IP-004 D1)
// =============================================================================

/// Port for vector storage operations (similarity search, upsert, delete).
///
/// Refined in IP-004 D1 with `metadata`, `filter`, and `rebuild()`.
/// Implementations store embedding vectors and support nearest-neighbor search.
#[async_trait]
pub trait VectorStore: Send + Sync {
    /// Insert or update a vector for the given entity.
    ///
    /// # Errors
    ///
    /// Returns `VectorError::DimensionMismatch` if the vector length does not
    /// match the expected dimensions.
    /// Returns `VectorError::Storage` on storage failures.
    async fn upsert(
        &self,
        entity_id: &str,
        vector: &[f32],
        metadata: Option<VectorMetadata>,
    ) -> Result<(), VectorError>;

    /// Search for the k nearest vectors to the query vector.
    ///
    /// # Errors
    ///
    /// Returns `VectorError::Storage` on storage failures.
    async fn search(
        &self,
        query: &[f32],
        k: usize,
        filter: Option<VectorFilter>,
    ) -> Result<Vec<VectorResult>, VectorError>;

    /// Delete the vector for the given entity.
    ///
    /// # Errors
    ///
    /// Returns `VectorError::Storage` on storage failures.
    async fn delete(&self, entity_id: &str) -> Result<(), VectorError>;

    /// Rebuild the vector index from scratch.
    ///
    /// Implementations should clear all stored vectors and return success.
    /// Callers are responsible for re-populating the store after rebuild.
    ///
    /// # Errors
    ///
    /// Returns `VectorError::Storage` on storage failures.
    async fn rebuild(&self) -> Result<(), VectorError>;
}

/// Errors that can occur during vector storage operations.
#[derive(Debug, thiserror::Error)]
pub enum VectorError {
    /// The vector store returned an error.
    #[error("Storage error: {0}")]
    Storage(String),

    /// The vector length does not match the expected dimensions.
    #[error("Dimension mismatch: expected {expected}, got {actual}")]
    DimensionMismatch { expected: usize, actual: usize },
}

/// Filter criteria for vector search results.
#[derive(Debug, Clone, Default)]
pub struct VectorFilter {
    /// Include only entities of these types.
    pub entity_types: Option<Vec<EntityType>>,
    /// Include only entities with any of these tags.
    pub tags: Option<Vec<String>>,
    /// Minimum similarity score threshold.
    pub min_score: Option<f64>,
}

/// Metadata attached to a stored vector.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VectorMetadata {
    /// Name of the embedding model that produced this vector.
    pub model: String,
    /// Entity type of the owning entity.
    pub entity_type: String,
    /// Title of the owning entity.
    pub title: String,
}

/// A single result from a vector similarity search.
#[derive(Debug, Clone)]
pub struct VectorResult {
    /// The entity ID that this vector belongs to.
    pub entity_id: String,
    /// The similarity score (higher = more similar).
    pub score: f64,
    /// Metadata attached to the stored vector, if available.
    pub metadata: Option<VectorMetadata>,
}

/// A single result from hybrid (RRF-fused) search.
#[derive(Debug, Clone)]
pub struct FusedResult {
    /// The entity ID of the result.
    pub entity_id: String,
    /// The fused RRF score (higher = more relevant).
    pub score: f64,
}
