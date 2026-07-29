use async_trait::async_trait;
use knowledge_core::features::component::{Component, ComponentType};
use knowledge_core::features::entity::{Entity, EntityType};
use std::path::Path;

use super::adapter::{ImportAdapter, ImportError, ImportResult};

pub struct MboxImporter;

impl MboxImporter {
    pub fn new() -> Self {
        Self
    }
}

#[async_trait]
impl ImportAdapter for MboxImporter {
    fn can_import(&self, path: &Path) -> bool {
        path.extension()
            .and_then(|e| e.to_str())
            .map(|e| e.eq_ignore_ascii_case("mbox"))
            .unwrap_or(false)
    }

    async fn import(&self, _path: &Path) -> Result<ImportResult, ImportError> {
        Err(ImportError::Parse(
            "Mbox import creates multiple entities. Use import_mbox() for multi-entity import."
                .into(),
        ))
    }

    fn supported_extensions(&self) -> &[&str] {
        &["mbox"]
    }
}

impl MboxImporter {
    pub async fn import_mbox(&self, path: &Path) -> Result<Vec<ImportResult>, ImportError> {
        let content = std::fs::read_to_string(path)?;
        let messages: Vec<&str> = content
            .split("\nFrom ")
            .filter(|s| !s.trim().is_empty())
            .collect();

        let mut results = Vec::new();
        for msg_text in messages {
            let bytes = if results.is_empty() {
                format!("{}\n", msg_text).into_bytes()
            } else {
                format!("From {}\n", msg_text).into_bytes()
            };

            let parsed = mailparse::parse_mail(&bytes)
                .map_err(|e| ImportError::Parse(format!("mbox message parse error: {}", e)))?;

            let subject = parsed
                .headers
                .iter()
                .find(|h| h.get_key_ref().eq_ignore_ascii_case("Subject"))
                .map(|h| h.get_value())
                .unwrap_or_else(|| format!("Message {}", results.len() + 1));

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
                Component::new(
                    entity.id,
                    ComponentType::Title,
                    serde_json::json!(subject),
                ),
                Component::new(
                    entity.id,
                    ComponentType::Content,
                    serde_json::json!(body),
                ),
                Component::new(
                    entity.id,
                    ComponentType::Provenance,
                    serde_json::json!({
                        "source": path.to_string_lossy(),
                        "imported_at": chrono::Utc::now().to_rfc3339(),
                        "format": "mbox",
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

            results.push(ImportResult {
                entity,
                components,
                cross_references: vec![],
            });
        }

        Ok(results)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_can_import() {
        let importer = MboxImporter::new();
        assert!(importer.can_import(Path::new("test.mbox")));
        assert!(!importer.can_import(Path::new("test.txt")));
    }

    #[test]
    fn test_supported_extensions() {
        let importer = MboxImporter::new();
        assert_eq!(importer.supported_extensions(), &["mbox"]);
    }
}
