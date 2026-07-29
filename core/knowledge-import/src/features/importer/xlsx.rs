use async_trait::async_trait;
use calamine::{open_workbook_auto, Data, Reader};
use knowledge_core::features::component::{Component, ComponentType};
use knowledge_core::features::entity::{Entity, EntityType};
use std::path::Path;

use super::adapter::{ImportAdapter, ImportError, ImportResult};

pub struct XlsxImporter;

impl XlsxImporter {
    pub fn new() -> Self {
        Self
    }
}

#[async_trait]
impl ImportAdapter for XlsxImporter {
    fn can_import(&self, path: &Path) -> bool {
        let ext = path
            .extension()
            .and_then(|e| e.to_str())
            .unwrap_or("");
        ext.eq_ignore_ascii_case("xlsx")
            || ext.eq_ignore_ascii_case("xls")
            || ext.eq_ignore_ascii_case("xlsm")
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
        let formatted = if path
            .extension()
            .and_then(|e| e.to_str())
            .map(|e| e.eq_ignore_ascii_case("xlsm"))
            .unwrap_or(false)
        {
            "xlsm"
        } else {
            "xlsx"
        };
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
                    "format": formatted,
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
        &["xlsx", "xls", "xlsm"]
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_can_import() {
        let importer = XlsxImporter::new();
        assert!(importer.can_import(Path::new("test.xlsx")));
        assert!(importer.can_import(Path::new("test.XLSX")));
        assert!(importer.can_import(Path::new("test.xls")));
        assert!(importer.can_import(Path::new("test.xlsm")));
        assert!(!importer.can_import(Path::new("test.txt")));
    }

    #[test]
    fn test_supported_extensions() {
        let importer = XlsxImporter::new();
        assert_eq!(importer.supported_extensions(), &["xlsx", "xls", "xlsm"]);
    }
}
