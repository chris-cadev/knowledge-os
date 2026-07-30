use async_trait::async_trait;
use knowledge_core::features::component::{Component, ComponentType};
use knowledge_core::features::entity::{Entity, EntityType};
use std::path::Path;

use super::adapter::{ImportAdapter, ImportError, ImportResult};

pub struct EmlImporter;

impl Default for EmlImporter {
    fn default() -> Self {
        Self::new()
    }
}

impl EmlImporter {
    pub fn new() -> Self {
        Self
    }
}

#[async_trait]
impl ImportAdapter for EmlImporter {
    fn can_import(&self, path: &Path) -> bool {
        path.extension()
            .and_then(|e| e.to_str())
            .map(|e| e.eq_ignore_ascii_case("eml"))
            .unwrap_or(false)
    }

    async fn import(&self, path: &Path) -> Result<ImportResult, ImportError> {
        let bytes = std::fs::read(path)?;
        let parsed = mailparse::parse_mail(&bytes)
            .map_err(|e| ImportError::Parse(format!("EML parse error: {}", e)))?;

        let subject = parsed
            .headers
            .iter()
            .find(|h| h.get_key_ref().eq_ignore_ascii_case("Subject"))
            .map(|h| h.get_value())
            .unwrap_or_else(|| {
                path.file_stem()
                    .and_then(|s| s.to_str())
                    .unwrap_or("Untitled Email")
                    .to_string()
            });

        let from = parsed
            .headers
            .iter()
            .find(|h| h.get_key_ref().eq_ignore_ascii_case("From"))
            .map(|h| h.get_value());

        let _to = parsed
            .headers
            .iter()
            .find(|h| h.get_key_ref().eq_ignore_ascii_case("To"))
            .map(|h| h.get_value());

        let date = parsed
            .headers
            .iter()
            .find(|h| h.get_key_ref().eq_ignore_ascii_case("Date"))
            .map(|h| h.get_value());

        let body = parsed
            .subparts
            .first()
            .and_then(|p| p.get_body().ok())
            .unwrap_or_default();

        let entity = Entity::new(EntityType::new("Email"));

        let mut components = vec![
            Component::new(entity.id, ComponentType::Title, serde_json::json!(subject)),
            Component::new(entity.id, ComponentType::Content, serde_json::json!(body)),
            Component::new(
                entity.id,
                ComponentType::Provenance,
                serde_json::json!({
                    "source": path.to_string_lossy(),
                    "imported_at": chrono::Utc::now().to_rfc3339(),
                    "format": "eml",
                }),
            ),
        ];

        if let Some(from) = from {
            components.push(Component::new(
                entity.id,
                ComponentType::Author,
                serde_json::json!(from),
            ));
        }

        if let Some(date) = date {
            components.push(Component::new(
                entity.id,
                ComponentType::Timeline,
                serde_json::json!({
                    "created_at": date,
                    "imported_at": chrono::Utc::now().to_rfc3339(),
                }),
            ));
        }

        Ok(ImportResult {
            entity,
            components,
            cross_references: vec![],
        })
    }

    fn supported_extensions(&self) -> &[&str] {
        &["eml"]
    }
}

pub struct MsgImporter;

impl Default for MsgImporter {
    fn default() -> Self {
        Self::new()
    }
}

impl MsgImporter {
    pub fn new() -> Self {
        Self
    }
}

#[async_trait]
impl ImportAdapter for MsgImporter {
    fn can_import(&self, path: &Path) -> bool {
        path.extension()
            .and_then(|e| e.to_str())
            .map(|e| e.eq_ignore_ascii_case("msg"))
            .unwrap_or(false)
    }

    async fn import(&self, path: &Path) -> Result<ImportResult, ImportError> {
        let bytes = std::fs::read(path)?;
        let parsed = mailparse::parse_mail(&bytes)
            .map_err(|e| ImportError::Parse(format!("MSG parse error: {}", e)))?;

        let subject = parsed
            .headers
            .iter()
            .find(|h| h.get_key_ref().eq_ignore_ascii_case("Subject"))
            .map(|h| h.get_value())
            .unwrap_or_else(|| {
                path.file_stem()
                    .and_then(|s| s.to_str())
                    .unwrap_or("Untitled Message")
                    .to_string()
            });

        let from = parsed
            .headers
            .iter()
            .find(|h| h.get_key_ref().eq_ignore_ascii_case("From"))
            .map(|h| h.get_value());

        let body = parsed
            .subparts
            .first()
            .and_then(|p| p.get_body().ok())
            .unwrap_or_default();

        let entity = Entity::new(EntityType::new("Email"));
        let mut components = vec![
            Component::new(entity.id, ComponentType::Title, serde_json::json!(subject)),
            Component::new(entity.id, ComponentType::Content, serde_json::json!(body)),
            Component::new(
                entity.id,
                ComponentType::Provenance,
                serde_json::json!({
                    "source": path.to_string_lossy(),
                    "imported_at": chrono::Utc::now().to_rfc3339(),
                    "format": "msg",
                }),
            ),
        ];

        if let Some(from) = from {
            components.push(Component::new(
                entity.id,
                ComponentType::Author,
                serde_json::json!(from),
            ));
        }

        Ok(ImportResult {
            entity,
            components,
            cross_references: vec![],
        })
    }

    fn supported_extensions(&self) -> &[&str] {
        &["msg"]
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_eml_can_import() {
        let importer = EmlImporter::new();
        assert!(importer.can_import(Path::new("test.eml")));
        assert!(importer.can_import(Path::new("test.EML")));
        assert!(!importer.can_import(Path::new("test.txt")));
    }

    #[test]
    fn test_msg_can_import() {
        let importer = MsgImporter::new();
        assert!(importer.can_import(Path::new("test.msg")));
        assert!(!importer.can_import(Path::new("test.txt")));
    }

    #[test]
    fn test_eml_supported_extensions() {
        let importer = EmlImporter::new();
        assert_eq!(importer.supported_extensions(), &["eml"]);
    }

    #[test]
    fn test_msg_supported_extensions() {
        let importer = MsgImporter::new();
        assert_eq!(importer.supported_extensions(), &["msg"]);
    }
}
