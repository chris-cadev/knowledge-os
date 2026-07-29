use async_trait::async_trait;
use knowledge_core::features::component::{Component, ComponentType};
use knowledge_core::features::entity::{Entity, EntityType};
use knowledge_core::ports::{ColumnInfo, ColumnMapping, ColumnValue, FieldMapping, ImportPreview};
use std::path::Path;

use super::adapter::{ImportAdapter, ImportError, ImportResult};

pub struct CsvImporter;

impl Default for CsvImporter {
    fn default() -> Self {
        Self::new()
    }
}

impl CsvImporter {
    pub fn new() -> Self {
        Self
    }
}

#[async_trait]
impl ImportAdapter for CsvImporter {
    fn can_import(&self, path: &Path) -> bool {
        path.extension()
            .and_then(|e| e.to_str())
            .map(|e| e.eq_ignore_ascii_case("csv"))
            .unwrap_or(false)
    }

    async fn import(&self, _path: &Path) -> Result<ImportResult, ImportError> {
        Err(ImportError::Parse(
            "CSV import requires column mapping. Use preview() then import_with_mapping().".into(),
        ))
    }

    fn supported_extensions(&self) -> &[&str] {
        &["csv"]
    }
}

impl CsvImporter {
    pub async fn preview(
        &self,
        path: &Path,
        sample_size: usize,
    ) -> Result<ImportPreview, ImportError> {
        let mut reader = csv::ReaderBuilder::new()
            .from_path(path)
            .map_err(|e| ImportError::Parse(format!("CSV open error: {}", e)))?;
        let headers: Vec<String> = reader
            .headers()
            .map_err(|e| ImportError::Parse(e.to_string()))?
            .iter()
            .map(|s| s.to_string())
            .collect();
        let mut rows = Vec::new();
        for (i, record) in reader.records().enumerate() {
            if i >= sample_size {
                break;
            }
            let record = record.map_err(|e| ImportError::Parse(format!("CSV row {}: {}", i, e)))?;
            rows.push(
                record
                    .iter()
                    .map(|s| ColumnValue::Text(s.to_string()))
                    .collect(),
            );
        }
        let row_count = reader.records().count() as u64;
        Ok(ImportPreview {
            columns: headers
                .iter()
                .map(|n| ColumnInfo {
                    name: n.clone(),
                    data_type: "text".to_string(),
                    nullable: true,
                })
                .collect(),
            sample_rows: rows,
            row_count,
        })
    }

    pub async fn import_with_mapping(
        &self,
        path: &Path,
        mapping: &ColumnMapping,
    ) -> Result<Vec<ImportResult>, ImportError> {
        let mut reader = csv::ReaderBuilder::new()
            .from_path(path)
            .map_err(|e| ImportError::Parse(format!("CSV open error: {}", e)))?;
        let headers: Vec<String> = reader
            .headers()
            .map_err(|e| ImportError::Parse(e.to_string()))?
            .iter()
            .map(|s| s.to_string())
            .collect();

        let mut results = Vec::new();
        for (i, record) in reader.records().enumerate() {
            let record = record.map_err(|e| ImportError::Parse(format!("CSV row {}: {}", i, e)))?;
            let row: Vec<&str> = record.iter().collect();

            if mapping.skip_columns.iter().all(|c| !headers.contains(c)) {
                // Skip if all skip columns exist
            }

            let title = mapping
                .field_mappings
                .iter()
                .find(|(_, m)| matches!(m, FieldMapping::Title))
                .and_then(|(col, _)| headers.iter().position(|h| h == col))
                .and_then(|idx| row.get(idx))
                .unwrap_or(&"Untitled")
                .to_string();

            let entity_type = mapping
                .entity_type_override
                .as_deref()
                .map(EntityType::new)
                .unwrap_or_else(|| EntityType::new("Article"));

            let entity = Entity::new(entity_type);
            let mut components = vec![
                Component::new(entity.id, ComponentType::Title, serde_json::json!(title)),
                Component::new(
                    entity.id,
                    ComponentType::Provenance,
                    serde_json::json!({
                        "source": path.to_string_lossy(),
                        "imported_at": chrono::Utc::now().to_rfc3339(),
                        "format": "csv",
                    }),
                ),
            ];

            for (col_name, field_map) in &mapping.field_mappings {
                if mapping.skip_columns.contains(col_name) {
                    continue;
                }
                let col_idx = match headers.iter().position(|h| h == col_name) {
                    Some(idx) => idx,
                    None => continue,
                };
                let val = match row.get(col_idx) {
                    Some(v) => v.to_string(),
                    None => continue,
                };

                match field_map {
                    FieldMapping::Title => {
                        // Already handled above; update if not default
                        if let Some(c) = components
                            .iter_mut()
                            .find(|c| c.component_type == ComponentType::Title)
                        {
                            c.data = serde_json::json!(val);
                        }
                    }
                    FieldMapping::Description => {
                        components.push(Component::new(
                            entity.id,
                            ComponentType::Description,
                            serde_json::json!(val),
                        ));
                    }
                    FieldMapping::Content => {
                        components.push(Component::new(
                            entity.id,
                            ComponentType::Content,
                            serde_json::json!(val),
                        ));
                    }
                    FieldMapping::Tags { separator } => {
                        let tags: Vec<String> = val
                            .split(separator)
                            .map(|s| s.trim().to_string())
                            .filter(|s| !s.is_empty())
                            .collect();
                        components.push(Component::new(
                            entity.id,
                            ComponentType::Tags,
                            serde_json::json!(tags),
                        ));
                    }
                    FieldMapping::TimelineDate => {
                        components.push(Component::new(
                            entity.id,
                            ComponentType::Timeline,
                            serde_json::json!({
                                "created_at": val,
                                "imported_at": chrono::Utc::now().to_rfc3339(),
                            }),
                        ));
                    }
                    FieldMapping::CustomComponent { component_name } => {
                        components.push(Component::new(
                            entity.id,
                            ComponentType::Provenance,
                            serde_json::json!({
                                "source": path.to_string_lossy(),
                                "imported_at": chrono::Utc::now().to_rfc3339(),
                                "format": "csv",
                                "custom_field": component_name.clone(),
                                "value": val,
                            }),
                        ));
                    }
                }
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

pub struct JsonImporter;

impl Default for JsonImporter {
    fn default() -> Self {
        Self::new()
    }
}

impl JsonImporter {
    pub fn new() -> Self {
        Self
    }
}

#[async_trait]
impl ImportAdapter for JsonImporter {
    fn can_import(&self, path: &Path) -> bool {
        path.extension()
            .and_then(|e| e.to_str())
            .map(|e| e.eq_ignore_ascii_case("json"))
            .unwrap_or(false)
    }

    async fn import(&self, path: &Path) -> Result<ImportResult, ImportError> {
        let content = std::fs::read_to_string(path)?;
        let value: serde_json::Value =
            serde_json::from_str(&content).map_err(|e| ImportError::Parse(e.to_string()))?;

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
                serde_json::json!(value.to_string()),
            ),
            Component::new(
                entity.id,
                ComponentType::Provenance,
                serde_json::json!({
                    "source": path.to_string_lossy(),
                    "imported_at": chrono::Utc::now().to_rfc3339(),
                    "format": "json",
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
        &["json"]
    }
}

pub struct XmlImporter;

impl Default for XmlImporter {
    fn default() -> Self {
        Self::new()
    }
}

impl XmlImporter {
    pub fn new() -> Self {
        Self
    }
}

#[async_trait]
impl ImportAdapter for XmlImporter {
    fn can_import(&self, path: &Path) -> bool {
        path.extension()
            .and_then(|e| e.to_str())
            .map(|e| e.eq_ignore_ascii_case("xml"))
            .unwrap_or(false)
    }

    async fn import(&self, path: &Path) -> Result<ImportResult, ImportError> {
        let content = std::fs::read_to_string(path)?;
        let mut reader = quick_xml::Reader::from_str(&content);
        let mut text = String::new();
        let mut in_element = false;
        loop {
            match reader.read_event() {
                Ok(quick_xml::events::Event::Text(ref e)) if in_element => {
                    if let Ok(t) = e.unescape() {
                        text.push_str(&t);
                        text.push(' ');
                    }
                }
                Ok(quick_xml::events::Event::Start(ref e)) => {
                    in_element = true;
                    if let Ok(name) = String::from_utf8(e.name().as_ref().to_vec()) {
                        text.push_str(&format!("<{}>", name));
                    }
                }
                Ok(quick_xml::events::Event::End(ref e)) => {
                    if let Ok(name) = String::from_utf8(e.name().as_ref().to_vec()) {
                        text.push_str(&format!("</{}>\n", name));
                    }
                    in_element = false;
                }
                Ok(quick_xml::events::Event::Eof) => break,
                Err(_) => break,
                _ => {}
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
                    "format": "xml",
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
        &["xml"]
    }
}

pub struct YamlImporter;

impl Default for YamlImporter {
    fn default() -> Self {
        Self::new()
    }
}

impl YamlImporter {
    pub fn new() -> Self {
        Self
    }
}

#[async_trait]
impl ImportAdapter for YamlImporter {
    fn can_import(&self, path: &Path) -> bool {
        let ext = path.extension().and_then(|e| e.to_str()).unwrap_or("");
        ext.eq_ignore_ascii_case("yaml") || ext.eq_ignore_ascii_case("yml")
    }

    async fn import(&self, path: &Path) -> Result<ImportResult, ImportError> {
        let content = std::fs::read_to_string(path)?;
        let value: serde_yaml::Value =
            serde_yaml::from_str(&content).map_err(|e| ImportError::Parse(e.to_string()))?;

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
                serde_json::json!(format!("{:?}", value)),
            ),
            Component::new(
                entity.id,
                ComponentType::Provenance,
                serde_json::json!({
                    "source": path.to_string_lossy(),
                    "imported_at": chrono::Utc::now().to_rfc3339(),
                    "format": "yaml",
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
        &["yaml", "yml"]
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::HashMap;
    use std::io::Write;
    use tempfile::NamedTempFile;

    fn create_csv(content: &str) -> NamedTempFile {
        let mut file = NamedTempFile::new().unwrap();
        file.write_all(content.as_bytes()).unwrap();
        file.flush().unwrap();
        file
    }

    #[tokio::test]
    async fn test_csv_preview_returns_columns_and_sample() {
        let csv = "Name,Description\nAlpha,First item\nBeta,Second item\n";
        let file = create_csv(csv);
        let importer = CsvImporter::new();
        let preview = importer.preview(file.path(), 10).await.unwrap();
        assert_eq!(preview.columns.len(), 2);
        assert_eq!(preview.columns[0].name, "Name");
        assert_eq!(preview.sample_rows.len(), 2);
    }

    #[tokio::test]
    async fn test_csv_import_with_mapping_creates_entities() {
        let csv = "Name,Description\nAlpha,First item\n";
        let file = create_csv(csv);
        let importer = CsvImporter::new();

        let mut field_mappings = HashMap::new();
        field_mappings.insert("Name".to_string(), FieldMapping::Title);
        field_mappings.insert("Description".to_string(), FieldMapping::Content);

        let mapping = ColumnMapping {
            field_mappings,
            skip_columns: std::collections::HashSet::new(),
            entity_type_override: None,
        };

        let results = importer
            .import_with_mapping(file.path(), &mapping)
            .await
            .unwrap();
        assert_eq!(results.len(), 1);
        let title = results[0]
            .components
            .iter()
            .find(|c| c.component_type == ComponentType::Title)
            .unwrap();
        assert_eq!(title.data, serde_json::json!("Alpha"));
    }

    #[tokio::test]
    async fn test_csv_skip_columns_excluded() {
        let csv = "Name,Description,Internal\nAlpha,First item,secret\n";
        let file = create_csv(csv);
        let importer = CsvImporter::new();

        let mut field_mappings = HashMap::new();
        field_mappings.insert("Name".to_string(), FieldMapping::Title);
        field_mappings.insert("Description".to_string(), FieldMapping::Content);

        let mut skip = std::collections::HashSet::new();
        skip.insert("Internal".to_string());

        let mapping = ColumnMapping {
            field_mappings,
            skip_columns: skip,
            entity_type_override: None,
        };

        let results = importer
            .import_with_mapping(file.path(), &mapping)
            .await
            .unwrap();
        assert_eq!(results.len(), 1);
    }

    #[tokio::test]
    async fn test_csv_tags_with_separator_splits_correctly() {
        let csv = "Name,Tags\nAlpha,rust;testing\n";
        let file = create_csv(csv);
        let importer = CsvImporter::new();

        let mut field_mappings = HashMap::new();
        field_mappings.insert("Name".to_string(), FieldMapping::Title);
        field_mappings.insert(
            "Tags".to_string(),
            FieldMapping::Tags {
                separator: ";".to_string(),
            },
        );

        let mapping = ColumnMapping {
            field_mappings,
            skip_columns: std::collections::HashSet::new(),
            entity_type_override: None,
        };

        let results = importer
            .import_with_mapping(file.path(), &mapping)
            .await
            .unwrap();
        assert_eq!(results.len(), 1);
        let tags = results[0]
            .components
            .iter()
            .find(|c| c.component_type == ComponentType::Tags)
            .unwrap();
        assert_eq!(tags.data, serde_json::json!(["rust", "testing"]));
    }

    #[test]
    fn test_csv_can_import() {
        let importer = CsvImporter::new();
        assert!(importer.can_import(Path::new("test.csv")));
        assert!(!importer.can_import(Path::new("test.txt")));
    }

    #[test]
    fn test_json_can_import() {
        let importer = JsonImporter::new();
        assert!(importer.can_import(Path::new("test.json")));
        assert!(!importer.can_import(Path::new("test.txt")));
    }

    #[test]
    fn test_xml_can_import() {
        let importer = XmlImporter::new();
        assert!(importer.can_import(Path::new("test.xml")));
        assert!(!importer.can_import(Path::new("test.txt")));
    }

    #[test]
    fn test_yaml_can_import() {
        let importer = YamlImporter::new();
        assert!(importer.can_import(Path::new("test.yaml")));
        assert!(importer.can_import(Path::new("test.yml")));
        assert!(!importer.can_import(Path::new("test.txt")));
    }

    #[tokio::test]
    async fn test_json_array_imports_objects() {
        let json = r#"{"items":[{"name":"test"}]}"#;
        let mut file = NamedTempFile::new().unwrap();
        file.write_all(json.as_bytes()).unwrap();
        file.flush().unwrap();
        let importer = JsonImporter::new();
        let result = importer.import(file.path()).await.unwrap();
        assert_eq!(result.entity.entity_type, EntityType::new("Article"));
    }

    #[tokio::test]
    async fn test_yaml_imports_mapping() {
        let yaml = "name: test\nvalue: 123\n";
        let mut file = NamedTempFile::new().unwrap();
        file.write_all(yaml.as_bytes()).unwrap();
        file.flush().unwrap();
        let importer = YamlImporter::new();
        let result = importer.import(file.path()).await.unwrap();
        assert_eq!(result.entity.entity_type, EntityType::new("Article"));
    }
}
