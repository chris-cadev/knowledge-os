use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

// PONYTAIL: Fixed enum with 20 variants. Ceiling: adding types requires code change.
// Upgrade: String-based entity types with validation, or registry pattern.
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
    pub is_active: bool,
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
            is_active: true,
            created_at: now,
            updated_at: now,
            version: 1,
        }
    }

    pub fn archive(&mut self) {
        self.is_active = false;
        self.updated_at = Utc::now();
    }

    pub fn restore(&mut self) {
        self.is_active = true;
        self.updated_at = Utc::now();
    }

    pub fn touch(&mut self) {
        self.version += 1;
        self.updated_at = Utc::now();
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_entity_creation() {
        let entity = Entity::new(EntityType::Article);
        assert_eq!(entity.entity_type, EntityType::Article);
        assert!(entity.is_active);
        assert_eq!(entity.version, 1);
    }

    #[test]
    fn test_entity_archive_restore() {
        let mut entity = Entity::new(EntityType::Note);
        assert!(entity.is_active);

        entity.archive();
        assert!(!entity.is_active);

        entity.restore();
        assert!(entity.is_active);
    }

    #[test]
    fn test_entity_touch() {
        let mut entity = Entity::new(EntityType::Concept);
        let original_version = entity.version;

        entity.touch();
        assert_eq!(entity.version, original_version + 1);
    }
}
