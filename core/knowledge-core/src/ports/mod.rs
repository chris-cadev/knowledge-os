use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::features::component::Component;
use crate::features::entity::Entity;
use crate::features::relationship::Relationship;

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
    async fn find_by_component_type(&self, component_type: &str) -> Result<Vec<Entity>, StorageError>;
    async fn find_by_tag(&self, tag: &str) -> Result<Vec<Entity>, StorageError>;
    async fn get_version_history(&self, entity_id: Uuid) -> Result<Vec<EntityVersion>, StorageError>;
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
    async fn find_by_type(&self, relationship_type: &str) -> Result<Vec<Relationship>, StorageError>;
}

#[async_trait]
pub trait ComponentRepository: Send + Sync {
    async fn get(&self, entity_id: Uuid) -> Result<Vec<Component>, StorageError>;
    async fn save(&self, component: &Component) -> Result<(), StorageError>;
    async fn delete(&self, id: Uuid) -> Result<(), StorageError>;
    async fn find_by_type(&self, entity_id: Uuid, component_type: &str) -> Result<Vec<Component>, StorageError>;
    async fn update_data(&self, id: Uuid, data: serde_json::Value) -> Result<(), StorageError>;
    async fn find_by_component_data(&self, component_type: &str, json_path: &str, value: &str) -> Result<Vec<Component>, StorageError>;
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
    async fn index_entity(&self, entity: &Entity, components: &[Component]) -> Result<(), StorageError>;
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
    pub snapshot: Option<String>,  // JSON snapshot of pre-merge state for undo
}

#[async_trait]
pub trait EntityResolver: Send + Sync {
    async fn find_candidates(&self, entity: &Entity, title: &str, content: Option<&str>) -> Result<Vec<ResolutionCandidate>, StorageError>;
    async fn merge(&self, canonical_id: Uuid, duplicate_id: Uuid, confidence: f64) -> Result<(), StorageError>;
    async fn log_merge(&self, entry: &MergeAuditEntry) -> Result<(), StorageError>;
    async fn undo_merge(&self, merge_id: Uuid) -> Result<(), StorageError>;
    async fn get_merge_history(&self, entity_id: Uuid) -> Result<Vec<MergeAuditEntry>, StorageError>;
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
