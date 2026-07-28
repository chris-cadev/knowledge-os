use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use super::error::StorageError;
use crate::features::component::Component;
use crate::features::entity::Entity;

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
