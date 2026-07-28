use async_trait::async_trait;

use super::error::StorageError;
use super::event::Event;
use crate::features::component::Component;
use crate::features::entity::Entity;

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
