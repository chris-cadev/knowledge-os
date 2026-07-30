use async_trait::async_trait;
use knowledge_core::features::component::{Component, ComponentType};
use knowledge_core::features::entity::{Entity, EntityType};
use quick_xml::events::Event;
use quick_xml::Reader as XmlReader;
use std::io::Read;
use std::path::Path;
use zip::ZipArchive;

use super::adapter::{ImportAdapter, ImportError, ImportResult};

fn extract_text_from_index_xml(bytes: &[u8]) -> Result<String, ImportError> {
    let mut reader = XmlReader::from_reader(bytes);
    let mut text = String::new();
    let mut in_sf = false;
    loop {
        match reader.read_event() {
            Ok(Event::Start(ref e)) => {
                if e.name().as_ref() == b"sf:p" || e.name().as_ref() == b"sf:span" {
                    in_sf = true;
                }
            }
            Ok(Event::Text(ref e)) if in_sf => {
                if let Ok(t) = e.unescape() {
                    text.push_str(&t);
                }
            }
            Ok(Event::End(ref e)) => {
                if e.name().as_ref() == b"sf:p" {
                    text.push('\n');
                }
                if e.name().as_ref() == b"sf:p" || e.name().as_ref() == b"sf:span" {
                    in_sf = false;
                }
            }
            Ok(Event::Eof) => break,
            Err(_) => break,
            _ => {}
        }
    }
    Ok(text)
}

fn extract_text_from_zip_index_xml(path: &Path) -> Result<String, ImportError> {
    let file = std::fs::File::open(path)?;
    let mut archive = ZipArchive::new(file)
        .map_err(|e| ImportError::Parse(format!("not a valid ZIP/iWork: {}", e)))?;

    let mut buf = Vec::new();
    if archive.by_name("index.xml").is_ok() {
        let mut entry = archive.by_name("index.xml").unwrap();
        entry.read_to_end(&mut buf)?;
    } else if archive.by_name("Index/Document.iwa").is_ok() {
        let mut entry = archive.by_name("Index/Document.iwa").unwrap();
        entry.read_to_end(&mut buf)?;
    } else {
        return Err(ImportError::Parse(
            "missing index.xml or Document.iwa".into(),
        ));
    }
    drop(archive);
    extract_text_from_index_xml(&buf)
}

pub struct PagesImporter;

impl Default for PagesImporter {
    fn default() -> Self {
        Self::new()
    }
}

impl PagesImporter {
    pub fn new() -> Self {
        Self
    }
}

#[async_trait]
impl ImportAdapter for PagesImporter {
    fn can_import(&self, path: &Path) -> bool {
        path.extension()
            .and_then(|e| e.to_str())
            .map(|e| e.eq_ignore_ascii_case("pages"))
            .unwrap_or(false)
    }

    async fn import(&self, path: &Path) -> Result<ImportResult, ImportError> {
        let text = extract_text_from_zip_index_xml(path)?;
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
                    "format": "pages",
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
        &["pages"]
    }
}

pub struct NumbersImporter;

impl Default for NumbersImporter {
    fn default() -> Self {
        Self::new()
    }
}

impl NumbersImporter {
    pub fn new() -> Self {
        Self
    }
}

#[async_trait]
impl ImportAdapter for NumbersImporter {
    fn can_import(&self, path: &Path) -> bool {
        path.extension()
            .and_then(|e| e.to_str())
            .map(|e| e.eq_ignore_ascii_case("numbers"))
            .unwrap_or(false)
    }

    async fn import(&self, path: &Path) -> Result<ImportResult, ImportError> {
        let text = extract_text_from_zip_index_xml(path)?;
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
                    "format": "numbers",
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
        &["numbers"]
    }
}

pub struct KeynoteImporter;

impl Default for KeynoteImporter {
    fn default() -> Self {
        Self::new()
    }
}

impl KeynoteImporter {
    pub fn new() -> Self {
        Self
    }
}

#[async_trait]
impl ImportAdapter for KeynoteImporter {
    fn can_import(&self, path: &Path) -> bool {
        path.extension()
            .and_then(|e| e.to_str())
            .map(|e| e.eq_ignore_ascii_case("key"))
            .unwrap_or(false)
    }

    async fn import(&self, path: &Path) -> Result<ImportResult, ImportError> {
        let text = extract_text_from_zip_index_xml(path)?;
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
                    "format": "keynote",
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
        &["key"]
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_pages_can_import() {
        let importer = PagesImporter::new();
        assert!(importer.can_import(Path::new("test.pages")));
        assert!(importer.can_import(Path::new("test.PAGES")));
        assert!(!importer.can_import(Path::new("test.pdf")));
    }

    #[test]
    fn test_numbers_can_import() {
        let importer = NumbersImporter::new();
        assert!(importer.can_import(Path::new("test.numbers")));
        assert!(!importer.can_import(Path::new("test.pdf")));
    }

    #[test]
    fn test_keynote_can_import() {
        let importer = KeynoteImporter::new();
        assert!(importer.can_import(Path::new("test.key")));
        assert!(!importer.can_import(Path::new("test.pdf")));
    }

    #[test]
    fn test_iwork_supported_extensions() {
        assert_eq!(PagesImporter::new().supported_extensions(), &["pages"]);
        assert_eq!(NumbersImporter::new().supported_extensions(), &["numbers"]);
        assert_eq!(KeynoteImporter::new().supported_extensions(), &["key"]);
    }

    #[test]
    fn test_iwork_unsupported_format() {
        let importer = PagesImporter::new();
        assert!(!importer.can_import(Path::new("test.txt")));
    }
}
