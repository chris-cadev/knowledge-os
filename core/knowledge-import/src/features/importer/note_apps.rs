use async_trait::async_trait;
use knowledge_core::features::component::{Component, ComponentType};
use knowledge_core::features::entity::{Entity, EntityType};
use quick_xml::events::Event;
use quick_xml::Reader as XmlReader;
use std::path::Path;

use super::adapter::{ImportAdapter, ImportError, ImportResult};
use super::markdown::MarkdownImporter;

pub struct EnexImporter;

impl EnexImporter {
    pub fn new() -> Self {
        Self
    }
}

#[async_trait]
impl ImportAdapter for EnexImporter {
    fn can_import(&self, path: &Path) -> bool {
        path.extension()
            .and_then(|e| e.to_str())
            .map(|e| e.eq_ignore_ascii_case("enex"))
            .unwrap_or(false)
    }

    async fn import(&self, _path: &Path) -> Result<ImportResult, ImportError> {
        Err(ImportError::Parse(
            "ENEX import creates multiple entities. Use import_enex() for multi-entity import."
                .into(),
        ))
    }

    fn supported_extensions(&self) -> &[&str] {
        &["enex"]
    }
}

impl EnexImporter {
    pub async fn import_enex(&self, path: &Path) -> Result<Vec<ImportResult>, ImportError> {
        let content = std::fs::read_to_string(path)?;
        let mut reader = XmlReader::from_str(&content);
        let mut results = Vec::new();
        let mut in_note = false;
        let mut in_title = false;
        let mut in_content = false;
        let mut in_tag = false;
        let mut current_title = String::new();
        let mut current_content = String::new();
        let mut current_tags: Vec<String> = Vec::new();

        loop {
            match reader.read_event() {
                Ok(Event::Start(ref e)) => {
                    if e.name().as_ref() == b"note" {
                        in_note = true;
                        current_title.clear();
                        current_content.clear();
                        current_tags.clear();
                    } else if in_note && e.name().as_ref() == b"title" {
                        in_title = true;
                    } else if in_note && e.name().as_ref() == b"content" {
                        in_content = true;
                    } else if in_note && e.name().as_ref() == b"tag" {
                        in_tag = true;
                    }
                }
                Ok(Event::Text(ref e)) => {
                    if let Ok(t) = e.unescape() {
                        if in_title {
                            current_title.push_str(&t);
                        } else if in_content {
                            current_content.push_str(&t);
                        } else if in_tag {
                            current_tags.push(t.to_string());
                        }
                    }
                }
                Ok(Event::End(ref e)) => {
                    if e.name().as_ref() == b"title" {
                        in_title = false;
                    } else if e.name().as_ref() == b"content" {
                        in_content = false;
                    } else if e.name().as_ref() == b"tag" {
                        in_tag = false;
                    } else if e.name().as_ref() == b"note" && in_note {
                        in_note = false;
                        let title = if current_title.is_empty() {
                            "Untitled Note"
                        } else {
                            &current_title
                        };
                        let entity = Entity::new(EntityType::new("Note"));
                        let mut components = vec![
                            Component::new(
                                entity.id,
                                ComponentType::Title,
                                serde_json::json!(title),
                            ),
                            Component::new(
                                entity.id,
                                ComponentType::Content,
                                serde_json::json!(current_content),
                            ),
                            Component::new(
                                entity.id,
                                ComponentType::Provenance,
                                serde_json::json!({
                                    "source": path.to_string_lossy(),
                                    "imported_at": chrono::Utc::now().to_rfc3339(),
                                    "format": "enex",
                                }),
                            ),
                        ];
                        if !current_tags.is_empty() {
                            components.push(Component::new(
                                entity.id,
                                ComponentType::Tags,
                                serde_json::json!(current_tags),
                            ));
                        }
                        results.push(ImportResult {
                            entity,
                            components,
                            cross_references: vec![],
                        });
                    }
                }
                Ok(Event::Eof) => break,
                Err(_) => break,
                _ => {}
            }
        }

        if results.is_empty() {
            return Err(ImportError::Parse(
                "No notes found in ENEX file".into(),
            ));
        }
        Ok(results)
    }
}

pub struct OpmlImporter;

impl OpmlImporter {
    pub fn new() -> Self {
        Self
    }
}

#[async_trait]
impl ImportAdapter for OpmlImporter {
    fn can_import(&self, path: &Path) -> bool {
        path.extension()
            .and_then(|e| e.to_str())
            .map(|e| e.eq_ignore_ascii_case("opml"))
            .unwrap_or(false)
    }

    async fn import(&self, _path: &Path) -> Result<ImportResult, ImportError> {
        Err(ImportError::Parse(
            "OPML import creates multiple entities. Use import_opml() for multi-entity import."
                .into(),
        ))
    }

    fn supported_extensions(&self) -> &[&str] {
        &["opml"]
    }
}

impl OpmlImporter {
    pub async fn import_opml(&self, path: &Path) -> Result<Vec<ImportResult>, ImportError> {
        let content = std::fs::read_to_string(path)?;
        let mut reader = XmlReader::from_str(&content);
        let mut results = Vec::new();
        let mut in_outline = false;
        let mut current_text = String::new();

        let outline_tag = b"outline";
        loop {
            match reader.read_event() {
                Ok(Event::Empty(ref e)) if e.name().as_ref() == outline_tag => {
                    current_text.clear();
                    for attr in e.attributes() {
                        let attr = attr.map_err(|e| ImportError::Parse(e.to_string()))?;
                        if attr.key.as_ref() == b"text" {
                            if let Ok(v) = attr.unescape_value() {
                                current_text = v.to_string();
                            }
                        }
                    }
                    if !current_text.is_empty() {
                        let entity = Entity::new(EntityType::new("Article"));
                        let components = vec![
                            Component::new(
                                entity.id,
                                ComponentType::Title,
                                serde_json::json!(&current_text),
                            ),
                            Component::new(
                                entity.id,
                                ComponentType::Provenance,
                                serde_json::json!({
                                    "source": path.to_string_lossy(),
                                    "imported_at": chrono::Utc::now().to_rfc3339(),
                                    "format": "opml",
                                }),
                            ),
                        ];
                        results.push(ImportResult {
                            entity,
                            components,
                            cross_references: vec![],
                        });
                    }
                }
                Ok(Event::Start(ref e)) if e.name().as_ref() == outline_tag => {
                    current_text.clear();
                    for attr in e.attributes() {
                        let attr = attr.map_err(|e| ImportError::Parse(e.to_string()))?;
                        if attr.key.as_ref() == b"text" {
                            if let Ok(v) = attr.unescape_value() {
                                current_text = v.to_string();
                            }
                        }
                    }
                }
                Ok(Event::End(ref e)) if e.name().as_ref() == outline_tag => {
                    if !current_text.is_empty() {
                        let entity = Entity::new(EntityType::new("Article"));
                        let components = vec![
                            Component::new(
                                entity.id,
                                ComponentType::Title,
                                serde_json::json!(&current_text),
                            ),
                            Component::new(
                                entity.id,
                                ComponentType::Provenance,
                                serde_json::json!({
                                    "source": path.to_string_lossy(),
                                    "imported_at": chrono::Utc::now().to_rfc3339(),
                                    "format": "opml",
                                }),
                            ),
                        ];
                        results.push(ImportResult {
                            entity,
                            components,
                            cross_references: vec![],
                        });
                    }
                    current_text.clear();
                }
                Ok(Event::Eof) => break,
                Err(_) => break,
                _ => {}
            }
        }

        if results.is_empty() {
            return Err(ImportError::Parse(
                "No outlines found in OPML file".into(),
            ));
        }
        Ok(results)
    }
}

pub struct NotionJsonImporter;

impl NotionJsonImporter {
    pub fn new() -> Self {
        Self
    }
}

#[async_trait]
impl ImportAdapter for NotionJsonImporter {
    fn can_import(&self, path: &Path) -> bool {
        path.extension()
            .and_then(|e| e.to_str())
            .map(|e| e.eq_ignore_ascii_case("json"))
            .unwrap_or(false)
            && path
                .file_stem()
                .and_then(|s| s.to_str())
                .map(|s| s.to_lowercase().contains("notion"))
                .unwrap_or(false)
    }

    async fn import(&self, path: &Path) -> Result<ImportResult, ImportError> {
        let content = std::fs::read_to_string(path)?;
        let value: serde_json::Value =
            serde_json::from_str(&content).map_err(|e| ImportError::Parse(e.to_string()))?;

        let value_str = value.to_string();

        let pages = match &value {
            serde_json::Value::Array(arr) => arr.clone(),
            serde_json::Value::Object(_) => vec![value],
            _ => {
                return Err(ImportError::Parse(
                    "Notion JSON must be an object or array".into(),
                ));
            }
        };

        for page in pages {
            let title = page
                .get("properties")
                .and_then(|p| p.get("title"))
                .and_then(|t| t.get("title"))
                .and_then(|t| t.as_array())
                .and_then(|arr| arr.first())
                .and_then(|t| t.get("plain_text"))
                .and_then(|t| t.as_str())
                .map(|s| s.to_string())
                .or_else(|| {
                    page.get("id")
                        .and_then(|id| id.as_str())
                        .map(|s| s.to_string())
                })
                .unwrap_or_else(|| "Untitled".to_string());

            let entity = Entity::new(EntityType::new("Page"));
            let components = vec![
                Component::new(entity.id, ComponentType::Title, serde_json::json!(title)),
                Component::new(
                    entity.id,
                    ComponentType::Content,
                    serde_json::json!(value_str),
                ),
                Component::new(
                    entity.id,
                    ComponentType::Provenance,
                    serde_json::json!({
                        "source": path.to_string_lossy(),
                        "imported_at": chrono::Utc::now().to_rfc3339(),
                        "format": "notion",
                    }),
                ),
            ];

            return Ok(ImportResult {
                entity,
                components,
                cross_references: vec![],
            });
        }

        Err(ImportError::Parse("No pages found in Notion JSON".into()))
    }

    fn supported_extensions(&self) -> &[&str] {
        &["json"]
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Write;
    use tempfile::NamedTempFile;

    #[test]
    fn test_enex_can_import() {
        let importer = EnexImporter::new();
        assert!(importer.can_import(Path::new("test.enex")));
        assert!(!importer.can_import(Path::new("test.txt")));
    }

    #[test]
    fn test_opml_can_import() {
        let importer = OpmlImporter::new();
        assert!(importer.can_import(Path::new("test.opml")));
        assert!(!importer.can_import(Path::new("test.txt")));
    }

    #[test]
    fn test_notion_can_import() {
        let importer = NotionJsonImporter::new();
        assert!(importer.can_import(Path::new("notion_export.json")));
        assert!(!importer.can_import(Path::new("test.json")));
    }

    #[tokio::test]
    async fn test_enex_imports_notes() {
        let enex = r#"<?xml version="1.0" encoding="UTF-8"?>
<!DOCTYPE en-export SYSTEM "http://xml.evernote.com/pub/evernote-export3.dtd">
<en-export>
<note><title>Note 1</title><content>Content 1</content><tag>tag1</tag></note>
<note><title>Note 2</title><content>Content 2</content></note>
</en-export>"#;
        let mut file = NamedTempFile::new().unwrap();
        file.write_all(enex.as_bytes()).unwrap();
        file.flush().unwrap();
        let importer = EnexImporter::new();
        let results = importer.import_enex(file.path()).await.unwrap();
        assert_eq!(results.len(), 2);
        let title = results[0]
            .components
            .iter()
            .find(|c| c.component_type == ComponentType::Title)
            .unwrap();
        assert_eq!(title.data, serde_json::json!("Note 1"));
        let tags = results[0]
            .components
            .iter()
            .find(|c| c.component_type == ComponentType::Tags);
        assert!(tags.is_some());
    }

    #[tokio::test]
    async fn test_opml_imports_outline_hierarchy() {
        let opml = r#"<?xml version="1.0" encoding="UTF-8"?>
<opml version="2.0">
<body>
<outline text="Topic 1"/>
<outline text="Topic 2"/>
</body>
</opml>"#;
        let mut file = NamedTempFile::new().unwrap();
        file.write_all(opml.as_bytes()).unwrap();
        file.flush().unwrap();
        let importer = OpmlImporter::new();
        let results = importer.import_opml(file.path()).await.unwrap();
        assert_eq!(results.len(), 2);
        let title = results[0]
            .components
            .iter()
            .find(|c| c.component_type == ComponentType::Title)
            .unwrap();
        assert_eq!(title.data, serde_json::json!("Topic 1"));
    }

    #[tokio::test]
    async fn test_notion_imports_pages() {
        let notion = r#"{
  "object": "page",
  "id": "abc123",
  "properties": {
    "title": {
      "title": [
        { "plain_text": "My Notion Page" }
      ]
    }
  }
}"#;
        let mut file = NamedTempFile::new().unwrap();
        file.write_all(notion.as_bytes()).unwrap();
        file.flush().unwrap();
        let importer = NotionJsonImporter::new();
        let result = importer.import(file.path()).await.unwrap();
        let title = result
            .components
            .iter()
            .find(|c| c.component_type == ComponentType::Title)
            .unwrap();
        assert_eq!(title.data, serde_json::json!("My Notion Page"));
    }
}
