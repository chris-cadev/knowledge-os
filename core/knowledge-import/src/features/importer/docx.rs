use async_trait::async_trait;
use docx_rs::{read_docx, DocumentChild, ParagraphChild, RunChild};
use knowledge_core::features::component::{Component, ComponentType};
use knowledge_core::features::entity::{Entity, EntityType};
use std::path::Path;

use super::adapter::{ImportAdapter, ImportError, ImportResult};

pub struct DocxImporter;

impl DocxImporter {
    pub fn new() -> Self {
        Self
    }
}

impl Default for DocxImporter {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl ImportAdapter for DocxImporter {
    fn can_import(&self, path: &Path) -> bool {
        path.extension()
            .and_then(|e| e.to_str())
            .map(|e| e.eq_ignore_ascii_case("docx"))
            .unwrap_or(false)
    }

    async fn import(&self, path: &Path) -> Result<ImportResult, ImportError> {
        let bytes = std::fs::read(path)?;
        let doc = read_docx(&bytes).map_err(|e| ImportError::Parse(e.to_string()))?;

        let mut text = String::new();
        for child in doc.document.children {
            if let DocumentChild::Paragraph(p) = child {
                for run in p.children {
                    if let ParagraphChild::Run(r) = run {
                        for child in r.children {
                            if let RunChild::Text(t) = child {
                                text.push_str(&t.text);
                                text.push('\n');
                            }
                        }
                    }
                }
            }
        }

        let entity = Entity::new(EntityType::new("Article"));
        let title = path
            .file_stem()
            .and_then(|s| s.to_str())
            .unwrap_or("Untitled")
            .to_string();
        let components = vec![
            Component::new(entity.id, ComponentType::Title, serde_json::json!(title)),
            Component::new(entity.id, ComponentType::Content, serde_json::json!(text)),
            Component::new(
                entity.id,
                ComponentType::Provenance,
                serde_json::json!({
                    "source": path.to_string_lossy(),
                    "imported_at": chrono::Utc::now().to_rfc3339(),
                    "format": "docx",
                }),
            ),
        ];

        Ok(ImportResult {
            entity,
            components,
            cross_references: vec![],
        })
    }

    fn supported_extensions(&self) -> &[&str] {
        &["docx"]
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_can_import() {
        let importer = DocxImporter::new();
        assert!(importer.can_import(Path::new("test.docx")));
        assert!(importer.can_import(Path::new("test.DOCX")));
        assert!(!importer.can_import(Path::new("test.txt")));
        assert!(!importer.can_import(Path::new("test.pdf")));
    }

    #[test]
    fn test_supported_extensions() {
        let importer = DocxImporter::new();
        assert_eq!(importer.supported_extensions(), &["docx"]);
    }
}
