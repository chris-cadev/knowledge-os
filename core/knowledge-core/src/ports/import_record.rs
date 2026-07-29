use serde::{Deserialize, Serialize};
use uuid::Uuid;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ImportRecord {
    pub id: Uuid,
    pub source_path: String,
    pub entity_ids: Vec<Uuid>,
    pub imported_at: chrono::DateTime<chrono::Utc>,
    pub format: String,
    pub content_hash: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UndoResult {
    pub removed_entities: Vec<Uuid>,
    pub import_record_id: Uuid,
}
