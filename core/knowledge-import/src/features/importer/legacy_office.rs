use async_trait::async_trait;
use calamine::{open_workbook_auto, Data, Reader};
use knowledge_core::features::component::{Component, ComponentType};
use knowledge_core::features::entity::{Entity, EntityType};
use std::path::Path;

use super::adapter::{ImportAdapter, ImportError, ImportResult};

pub struct DocImporter;

impl DocImporter {
    pub fn new() -> Self {
        Self
    }
}

#[async_trait]
impl ImportAdapter for DocImporter {
    fn can_import(&self, path: &Path) -> bool {
        path.extension()
            .and_then(|e| e.to_str())
            .map(|e| e.eq_ignore_ascii_case("doc"))
            .unwrap_or(false)
    }

    async fn import(&self, _path: &Path) -> Result<ImportResult, ImportError> {
        Err(ImportError::UnsupportedFormat(
            "Legacy .doc format is not supported. Please convert to .docx using Word, LibreOffice, or cloud services like Google Docs.".to_string()
        ))
    }

    fn supported_extensions(&self) -> &[&str] {
        &["doc"]
    }
}

pub struct XlsImporter;

impl XlsImporter {
    pub fn new() -> Self {
        Self
    }
}

#[async_trait]
impl ImportAdapter for XlsImporter {
    fn can_import(&self, path: &Path) -> bool {
        path.extension()
            .and_then(|e| e.to_str())
            .map(|e| e.eq_ignore_ascii_case("xls"))
            .unwrap_or(false)
    }

    async fn import(&self, path: &Path) -> Result<ImportResult, ImportError> {
        let mut workbook = open_workbook_auto(path)
            .map_err(|e| ImportError::Parse(e.to_string()))?;

        let sheet_name = workbook
            .sheet_names()
            .first()
            .ok_or_else(|| ImportError::Parse("workbook has no sheets".into()))?
            .clone();
        let range = workbook
            .worksheet_range(&sheet_name)
            .map_err(|e| ImportError::Parse(e.to_string()))?;

        let mut text = String::new();
        for row in range.rows() {
            let line: Vec<String> = row
                .iter()
                .map(|cell| match cell {
                    Data::String(s) => s.to_string(),
                    Data::Float(f) => f.to_string(),
                    Data::Int(i) => i.to_string(),
                    Data::Bool(b) => b.to_string(),
                    Data::DateTime(d) => d.to_string(),
                    Data::DateTimeIso(s) => s.to_string(),
                    Data::DurationIso(s) => s.to_string(),
                    Data::Error(e) => format!("ERROR:{:?}", e),
                    Data::Empty => String::new(),
                })
                .collect();
            text.push_str(&line.join("\t"));
            text.push('\n');
        }

        let entity = Entity::new(EntityType::new("Article"));
        let title = path
            .file_stem()
            .and_then(|s| s.to_str())
            .unwrap_or("Untitled")
            .to_string();
        let components = vec![
            Component::new(entity.id, ComponentType::Title, serde_json::json!(title)),
            Component::new(
                entity.id,
                ComponentType::Content,
                serde_json::json!(text),
            ),
            Component::new(
                entity.id,
                ComponentType::Provenance,
                serde_json::json!({
                    "source": path.to_string_lossy(),
                    "imported_at": chrono::Utc::now().to_rfc3339(),
                    "format": "xls",
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
        &["xls"]
    }
}

pub struct PptImporter;

impl PptImporter {
    pub fn new() -> Self {
        Self
    }
}

#[async_trait]
impl ImportAdapter for PptImporter {
    fn can_import(&self, path: &Path) -> bool {
        path.extension()
            .and_then(|e| e.to_str())
            .map(|e| e.eq_ignore_ascii_case("ppt"))
            .unwrap_or(false)
    }

    async fn import(&self, _path: &Path) -> Result<ImportResult, ImportError> {
        Err(ImportError::UnsupportedFormat(
            "Legacy .ppt format is not supported. Please convert to .pptx using PowerPoint, LibreOffice, or Google Slides.".to_string()
        ))
    }

    fn supported_extensions(&self) -> &[&str] {
        &["ppt"]
    }
}

pub struct PpsImporter;

impl PpsImporter {
    pub fn new() -> Self {
        Self
    }
}

#[async_trait]
impl ImportAdapter for PpsImporter {
    fn can_import(&self, path: &Path) -> bool {
        path.extension()
            .and_then(|e| e.to_str())
            .map(|e| e.eq_ignore_ascii_case("pps"))
            .unwrap_or(false)
    }

    async fn import(&self, _path: &Path) -> Result<ImportResult, ImportError> {
        Err(ImportError::UnsupportedFormat(
            "Legacy .pps format is not supported. Please convert to .pptx using PowerPoint, LibreOffice, or Google Slides.".to_string()
        ))
    }

    fn supported_extensions(&self) -> &[&str] {
        &["pps"]
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_doc_unsupported() {
        let importer = DocImporter::new();
        assert!(importer.can_import(Path::new("test.doc")));
        assert!(!importer.can_import(Path::new("test.docx")));
    }

    #[test]
    fn test_ppt_unsupported() {
        let importer = PptImporter::new();
        assert!(importer.can_import(Path::new("test.ppt")));
        assert!(!importer.can_import(Path::new("test.pptx")));
    }

    #[test]
    fn test_pps_unsupported() {
        let importer = PpsImporter::new();
        assert!(importer.can_import(Path::new("test.pps")));
        assert!(!importer.can_import(Path::new("test.pptx")));
    }

    #[test]
    fn test_xls_supported() {
        let importer = XlsImporter::new();
        assert!(importer.can_import(Path::new("test.xls")));
        assert!(!importer.can_import(Path::new("test.xlsx")));
    }

    #[tokio::test]
    async fn test_legacy_doc_import_returns_unsupported() {
        let importer = DocImporter::new();
        let result = importer.import(Path::new("test.doc")).await;
        assert!(result.is_err());
        match result.unwrap_err() {
            ImportError::UnsupportedFormat(msg) => assert!(msg.contains("convert")),
            _ => panic!("Expected UnsupportedFormat error"),
        }
    }

    #[tokio::test]
    async fn test_legacy_ppt_import_returns_unsupported() {
        let importer = PptImporter::new();
        let result = importer.import(Path::new("test.ppt")).await;
        assert!(result.is_err());
        match result.unwrap_err() {
            ImportError::UnsupportedFormat(msg) => assert!(msg.contains("convert")),
            _ => panic!("Expected UnsupportedFormat error"),
        }
    }
}
