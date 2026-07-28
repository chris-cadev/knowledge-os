use async_trait::async_trait;
use serde::{Deserialize, Serialize};

use crate::features::entity::EntityType;

#[async_trait]
pub trait VectorStore: Send + Sync {
    async fn upsert(
        &self,
        entity_id: &str,
        vector: &[f32],
        metadata: Option<VectorMetadata>,
    ) -> Result<(), VectorError>;
    async fn search(
        &self,
        query: &[f32],
        k: usize,
        filter: Option<VectorFilter>,
    ) -> Result<Vec<VectorResult>, VectorError>;
    async fn delete(&self, entity_id: &str) -> Result<(), VectorError>;
    async fn rebuild(&self) -> Result<(), VectorError>;
}

#[derive(Debug, thiserror::Error)]
pub enum VectorError {
    #[error("Storage error: {0}")]
    Storage(String),
    #[error("Dimension mismatch: expected {expected}, got {actual}")]
    DimensionMismatch { expected: usize, actual: usize },
}

#[derive(Debug, Clone, Default)]
pub struct VectorFilter {
    pub entity_types: Option<Vec<EntityType>>,
    pub tags: Option<Vec<String>>,
    pub min_score: Option<f64>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VectorMetadata {
    pub model: String,
    pub entity_type: String,
    pub title: String,
}

#[derive(Debug, Clone)]
pub struct VectorResult {
    pub entity_id: String,
    pub score: f64,
    pub metadata: Option<VectorMetadata>,
}

#[derive(Debug, Clone)]
pub struct FusedResult {
    pub entity_id: String,
    pub score: f64,
}
