use async_trait::async_trait;
use knowledge_core::features::component::{Component, ComponentType};
use knowledge_core::features::entity::{Entity, EntityType};
use std::path::Path;

use super::adapter::{ImportAdapter, ImportError, ImportResult};

pub struct ClipboardImporter;

impl ClipboardImporter {
    pub fn new() -> Self {
        Self
    }

    pub fn import_text(&self, text: &str, source: &str) -> Result<ImportResult, ImportError> {
        let entity = Entity::new(EntityType::new("Note"));
        let components = vec![
            Component::new(
                entity.id,
                ComponentType::Title,
                serde_json::json!("Clipboard Content"),
            ),
            Component::new(entity.id, ComponentType::Content, serde_json::json!(text)),
            Component::new(
                entity.id,
                ComponentType::Provenance,
                serde_json::json!({
                    "source": source,
                    "imported_at": chrono::Utc::now().to_rfc3339(),
                    "format": "clipboard-text",
                }),
            ),
        ];

        Ok(ImportResult {
            entity,
            components,
            cross_references: vec![],
        })
    }

    pub fn import_html(&self, html: &str, source: &str) -> Result<ImportResult, ImportError> {
        let entity = Entity::new(EntityType::new("Note"));
        let text = strip_html_tags(html);
        let components = vec![
            Component::new(
                entity.id,
                ComponentType::Title,
                serde_json::json!("Clipboard Content"),
            ),
            Component::new(entity.id, ComponentType::Content, serde_json::json!(text)),
            Component::new(
                entity.id,
                ComponentType::Provenance,
                serde_json::json!({
                    "source": source,
                    "imported_at": chrono::Utc::now().to_rfc3339(),
                    "format": "clipboard-html",
                }),
            ),
        ];

        Ok(ImportResult {
            entity,
            components,
            cross_references: vec![],
        })
    }
}

fn strip_html_tags(html: &str) -> String {
    let mut text = String::new();
    let mut in_tag = false;
    for c in html.chars() {
        match c {
            '<' => in_tag = true,
            '>' if in_tag => {
                in_tag = false;
                text.push(' ');
            }
            _ if !in_tag => text.push(c),
            _ => {}
        }
    }
    text.split_whitespace().collect::<Vec<_>>().join(" ")
}

impl Default for ClipboardImporter {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl ImportAdapter for ClipboardImporter {
    fn can_import(&self, _path: &Path) -> bool {
        false
    }

    async fn import(&self, _path: &Path) -> Result<ImportResult, ImportError> {
        Err(ImportError::UnsupportedFormat(
            "ClipboardImporter does not support file import. Use import_text() or import_html()."
                .into(),
        ))
    }

    fn supported_extensions(&self) -> &[&str] {
        &[]
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_clipboard_imports_text() {
        let importer = ClipboardImporter::new();
        let result = importer
            .import_text("Hello, world!", "manual-paste")
            .unwrap();
        let content = result
            .components
            .iter()
            .find(|c| c.component_type == ComponentType::Content)
            .unwrap();
        assert_eq!(content.data, serde_json::json!("Hello, world!"));
    }

    #[test]
    fn test_clipboard_imports_html() {
        let importer = ClipboardImporter::new();
        let result = importer
            .import_html("<h1>Title</h1><p>Content</p>", "manual-paste")
            .unwrap();
        let content = result
            .components
            .iter()
            .find(|c| c.component_type == ComponentType::Content)
            .unwrap();
        let text = content.data.as_str().unwrap();
        assert!(text.contains("Title"));
        assert!(text.contains("Content"));
    }
}
