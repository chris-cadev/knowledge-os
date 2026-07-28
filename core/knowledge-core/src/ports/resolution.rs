use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use super::error::StorageError;
use crate::features::entity::Entity;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ResolutionCandidate {
    pub entity_id: Uuid,
    pub confidence: f64,
    pub reason: String,
    pub title_score: Option<f64>,
    pub content_score: Option<f64>,
    pub metadata_score: Option<f64>,
    pub structural_score: Option<f64>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MergeAuditEntry {
    pub id: Uuid,
    pub source_id: Uuid,
    pub source_title: String,
    pub target_id: Uuid,
    pub target_title: String,
    pub strategy: String,
    pub confidence: f64,
    pub timestamp: chrono::DateTime<chrono::Utc>,
    pub reason: String,
    pub snapshot: Option<String>,
}

#[async_trait]
pub trait EntityResolver: Send + Sync {
    async fn find_candidates(
        &self,
        entity: &Entity,
        title: &str,
        content: Option<&str>,
    ) -> Result<Vec<ResolutionCandidate>, StorageError>;
    async fn merge(
        &self,
        canonical_id: Uuid,
        duplicate_id: Uuid,
        confidence: f64,
    ) -> Result<(), StorageError>;
    async fn log_merge(&self, entry: &MergeAuditEntry) -> Result<(), StorageError>;
    async fn undo_merge(&self, merge_id: Uuid) -> Result<(), StorageError>;
    async fn get_merge_history(
        &self,
        entity_id: Uuid,
    ) -> Result<Vec<MergeAuditEntry>, StorageError>;
    async fn get_all_merge_history(&self) -> Result<Vec<MergeAuditEntry>, StorageError>;
}
