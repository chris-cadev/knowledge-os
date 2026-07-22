use async_trait::async_trait;
use uuid::Uuid;

use crate::features::entity::Entity;
use crate::features::relationship::Relationship;
use crate::features::component::Component;

#[async_trait]
pub trait EntityRepository: Send + Sync {
    async fn get(&self, id: Uuid) -> Result<Option<Entity>, StorageError>;
    async fn save(&self, entity: &Entity) -> Result<(), StorageError>;
    async fn delete(&self, id: Uuid) -> Result<(), StorageError>;
    async fn list(&self) -> Result<Vec<Entity>, StorageError>;
}

#[async_trait]
pub trait RelationshipRepository: Send + Sync {
    async fn get(&self, id: Uuid) -> Result<Option<Relationship>, StorageError>;
    async fn save(&self, relationship: &Relationship) -> Result<(), StorageError>;
    async fn delete(&self, id: Uuid) -> Result<(), StorageError>;
    async fn by_source(&self, source_id: Uuid) -> Result<Vec<Relationship>, StorageError>;
    async fn by_target(&self, target_id: Uuid) -> Result<Vec<Relationship>, StorageError>;
}

#[async_trait]
pub trait ComponentRepository: Send + Sync {
    async fn get(&self, entity_id: Uuid) -> Result<Vec<Component>, StorageError>;
    async fn save(&self, component: &Component) -> Result<(), StorageError>;
    async fn delete(&self, id: Uuid) -> Result<(), StorageError>;
}

#[derive(Debug, thiserror::Error)]
pub enum StorageError {
    #[error("not found")]
    NotFound,
    #[error("storage error: {0}")]
    Internal(String),
}
