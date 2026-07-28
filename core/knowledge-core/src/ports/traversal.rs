use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use super::error::StorageError;
use crate::features::entity::EntityType;
use crate::features::relationship::RelationshipType;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum TraversalDirection {
    Outgoing,
    Incoming,
    Both,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TraversalQuery {
    pub start_id: Uuid,
    pub direction: TraversalDirection,
    pub max_depth: Option<u32>,
    pub max_results: Option<usize>,
    pub relationship_type: Option<RelationshipType>,
    pub entity_type_filter: Option<EntityType>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TraversalConfig {
    pub default_max_depth: u32,
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

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TraversalEdge {
    pub source_id: Uuid,
    pub target_id: Uuid,
    pub relationship_type: RelationshipType,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TraversalResult {
    pub path: Vec<Uuid>,
    pub edges: Vec<TraversalEdge>,
    pub depth: u32,
}

#[derive(Debug, thiserror::Error)]
pub enum TraversalError {
    #[error("Start entity not found: {0}")]
    StartNotFound(Uuid),
    #[error("Traversal limit exceeded: {limit} results")]
    LimitExceeded { limit: usize },
    #[error("Storage error: {0}")]
    Storage(#[from] StorageError),
}

#[async_trait]
pub trait TraversalPort: Send + Sync {
    async fn traverse(
        &self,
        query: &TraversalQuery,
        config: &TraversalConfig,
    ) -> Result<Vec<TraversalResult>, TraversalError>;
}
