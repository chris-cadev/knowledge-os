use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use super::error::StorageError;
use crate::features::entity::Entity;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Collection {
    pub id: Uuid,
    pub name: String,
    pub description: Option<String>,
    pub created_at: chrono::DateTime<chrono::Utc>,
    pub updated_at: chrono::DateTime<chrono::Utc>,
}

#[async_trait]
pub trait CollectionRepository: Send + Sync {
    async fn create(&self, collection: Collection) -> Result<Collection, StorageError>;
    async fn get(&self, id: Uuid) -> Result<Option<Collection>, StorageError>;
    async fn update(&self, collection: Collection) -> Result<Collection, StorageError>;
    async fn delete(&self, id: Uuid) -> Result<(), StorageError>;
    async fn list(&self) -> Result<Vec<Collection>, StorageError>;
    async fn add_member(&self, collection_id: Uuid, entity_id: Uuid) -> Result<(), StorageError>;
    async fn remove_member(&self, collection_id: Uuid, entity_id: Uuid)
        -> Result<(), StorageError>;
    async fn get_members(&self, collection_id: Uuid) -> Result<Vec<Entity>, StorageError>;
    async fn get_entity_collections(
        &self,
        entity_id: Uuid,
    ) -> Result<Vec<Collection>, StorageError>;
    async fn is_member(&self, collection_id: Uuid, entity_id: Uuid) -> Result<bool, StorageError>;
}
