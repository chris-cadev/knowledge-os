use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum EntityType {
    Concept,
    Person,
    Organization,
    Project,
    Book,
    Paper,
    Video,
    Article,
    Tool,
    Technology,
    Question,
    Idea,
    Event,
    Skill,
    Location,
    Dataset,
    Collection,
    Workspace,
    Decision,
    Note,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Entity {
    pub id: Uuid,
    pub entity_type: EntityType,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
    pub version: i64,
}

impl Entity {
    pub fn new(entity_type: EntityType) -> Self {
        let now = Utc::now();
        Self {
            id: Uuid::new_v4(),
            entity_type,
            created_at: now,
            updated_at: now,
            version: 1,
        }
    }
}
