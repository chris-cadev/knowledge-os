use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use super::entity_type::EntityType;

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
        let entity = Entity::new(EntityType::new("Article"));
        assert_eq!(entity.entity_type, EntityType::new("Article"));
        assert!(entity.is_active);
        assert_eq!(entity.version, 1);
    }

    #[test]
    fn test_entity_archive_restore() {
        let mut entity = Entity::new(EntityType::new("Note"));
        assert!(entity.is_active);

        entity.archive();
        assert!(!entity.is_active);

        entity.restore();
        assert!(entity.is_active);
    }

    #[test]
    fn test_entity_touch() {
        let mut entity = Entity::new(EntityType::new("Concept"));
        let original_version = entity.version;

        entity.touch();
        assert_eq!(entity.version, original_version + 1);
    }

    #[test]
    fn test_entity_type_string_based() {
        let et = EntityType::new("Paper");
        assert_eq!(et.as_str(), "Paper");
        assert!(et.is_known());

        let custom = EntityType::new("CustomType");
        assert_eq!(custom.as_str(), "CustomType");
        assert!(!custom.is_known());
    }

    #[test]
    fn test_entity_type_serialization() {
        let et = EntityType::new("Article");
        let json = serde_json::to_string(&et).unwrap();
        assert_eq!(json, "\"Article\"");

        let deserialized: EntityType = serde_json::from_str(&json).unwrap();
        assert_eq!(deserialized, et);
    }
}
