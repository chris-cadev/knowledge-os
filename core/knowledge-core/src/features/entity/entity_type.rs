use serde::{Deserialize, Deserializer, Serialize, Serializer};

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
        "Conversation",
        "Message",
    ];

    pub fn new(type_name: &str) -> Self {
        let canonical = Self::KNOWN_TYPES
            .iter()
            .find(|&&kt| kt.eq_ignore_ascii_case(type_name));
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
