use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use super::error::StorageError;
use crate::features::entity::Entity;

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
