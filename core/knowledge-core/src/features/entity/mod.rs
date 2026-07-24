use chrono::{DateTime, Utc};
use serde::{Deserialize, Deserializer, Serialize, Serializer};
use uuid::Uuid;

/// String-based entity type with validation against known types.
/// New types are added through configuration, not code changes.
#[derive(Debug, Clone, PartialEq)]
pub struct EntityType(String);

impl EntityType {
    pub const KNOWN_TYPES: &'static [&'static str] = &[
        "Concept",
        "Person",
        "Organization",
        "Project",
        "Book",
        "Paper",
        "Video",
        "Article",
        "Tool",
        "Technology",
        "Question",
        "Idea",
        "Event",
        "Skill",
        "Location",
        "Dataset",
        "Collection",
        "Workspace",
        "Decision",
        "Note",
    ];

    pub fn new(type_name: &str) -> Self {
        // For case-insensitive matching per PRD-0002:
        // If the input matches a known type (case-insensitively), use the canonical form
        // Otherwise, preserve the original casing for custom types
        let canonical = Self::KNOWN_TYPES.iter().find(|&&kt| kt.eq_ignore_ascii_case(type_name));
        match canonical {
            Some(&kt) => Self(kt.to_string()),
            None => Self(type_name.to_string()),
        }
    }

    pub fn as_str(&self) -> &str {
        &self.0
    }

    pub fn is_known(&self) -> bool {
        Self::KNOWN_TYPES.contains(&self.0.as_str())
    }
}

impl Serialize for EntityType {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        serializer.serialize_str(&self.0)
    }
}

impl<'de> Deserialize<'de> for EntityType {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        let s = String::deserialize(deserializer)?;
        Ok(EntityType(s))
    }
}

impl std::fmt::Display for EntityType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.0)
    }
}

impl From<&str> for EntityType {
    fn from(s: &str) -> Self {
        Self::new(s)
    }
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
