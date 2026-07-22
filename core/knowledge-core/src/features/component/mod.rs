use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum ComponentType {
    Title,
    Description,
    Content,
    Tags,
    Author,
    Embedding,
    Summary,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Component {
    pub id: Uuid,
    pub entity_id: Uuid,
    pub component_type: ComponentType,
    pub data: serde_json::Value,
    pub created_at: DateTime<Utc>,
    pub version: i64,
}

impl Component {
    pub fn new(entity_id: Uuid, component_type: ComponentType, data: serde_json::Value) -> Self {
        Self {
            id: Uuid::new_v4(),
            entity_id,
            component_type,
            data,
            created_at: Utc::now(),
            version: 1,
        }
    }
}
