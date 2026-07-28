use async_trait::async_trait;
use uuid::Uuid;

use super::error::StorageError;
use crate::features::component::Component;

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
