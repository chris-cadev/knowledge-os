use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum RelationshipType {
    References,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Relationship {
    pub id: Uuid,
    pub source_id: Uuid,
    pub target_id: Uuid,
    pub relationship_type: RelationshipType,
    pub is_active: bool,
    pub created_at: DateTime<Utc>,
}

impl Relationship {
    pub fn new(source_id: Uuid, target_id: Uuid, relationship_type: RelationshipType) -> Self {
        Self {
            id: Uuid::new_v4(),
            source_id,
            target_id,
            relationship_type,
            is_active: true,
            created_at: Utc::now(),
        }
    }

    pub fn archive(&mut self) {
        self.is_active = false;
    }
}
