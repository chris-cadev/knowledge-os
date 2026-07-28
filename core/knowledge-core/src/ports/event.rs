use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use super::error::StorageError;

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
