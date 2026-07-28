use async_trait::async_trait;
use uuid::Uuid;

use super::error::StorageError;
use crate::features::relationship::Relationship;

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
