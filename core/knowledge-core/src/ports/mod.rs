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
// Collections (stub — implemented fully in IP-005)
// =============================================================================

/// A curated group of entities.
///
/// This is a minimal definition to support `CollectionRepository` in view adapters.
/// The full implementation is defined in IP-005 and ADR-0018.
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
/// Stub trait for IP-002 view adapters. Full implementation in IP-005.
#[async_trait]
pub trait CollectionRepository: Send + Sync {
    /// List all collections.
    async fn list(&self) -> Result<Vec<Collection>, StorageError>;
    /// Get a collection by ID.
    async fn get(&self, id: Uuid) -> Result<Option<Collection>, StorageError>;
    /// Get all entities belonging to a collection.
    async fn get_members(&self, collection_id: Uuid) -> Result<Vec<Entity>, StorageError>;
}
