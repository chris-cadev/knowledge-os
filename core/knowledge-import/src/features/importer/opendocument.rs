use async_trait::async_trait;
use knowledge_core::features::component::{Component, ComponentType};
use knowledge_core::features::entity::{Entity, EntityType};
use quick_xml::events::Event;
use quick_xml::Reader as XmlReader;
use std::io::Read;
use std::path::Path;
use zip::ZipArchive;

use super::adapter::{ImportAdapter, ImportError, ImportResult};

fn strip_xml_tags(xml: &str) -> String {
    let mut reader = XmlReader::from_str(xml);
    let mut text = String::new();
    let mut in_text = false;
    loop {
        match reader.read_event() {
            Ok(Event::Start(ref e)) => {
                in_text = e.name().as_ref() == b"text:p"
                    || e.name().as_ref() == b"text:h"
                    || e.name().as_ref() == b"text:span"
            }
            Ok(Event::Text(ref e)) if in_text => {
                if let Ok(t) = e.unescape() {
                    text.push_str(&t);
                }
            }
            Ok(Event::End(ref e)) => {
                if e.name().as_ref() == b"text:p" || e.name().as_ref() == b"text:h" {
                    text.push('\n');
                }
            }
            Ok(Event::Eof) => break,
            Err(_) => break,
            _ => {}
        }
    }
    text
}

fn extract_odf_text(path: &Path, expected_content_path: &str) -> Result<String, ImportError> {
    let file = std::fs::File::open(path)?;
    let mut archive = ZipArchive::new(file)
        .map_err(|e| ImportError::Parse(format!("not a valid ZIP/ODF: {}", e)))?;
    let mut content_file = archive
        .by_name(expected_content_path)
        .map_err(|_| ImportError::Parse(format!("missing {}", expected_content_path)))?;
    let mut content = String::new();
    content_file.read_to_string(&mut content)?;
    let text = strip_xml_tags(&content);
    Ok(text)
}

fn extract_ods_text(path: &Path) -> Result<String, ImportError> {
    let file = std::fs::File::open(path)?;
    let mut archive = ZipArchive::new(file)
        .map_err(|e| ImportError::Parse(format!("not a valid ZIP/ODS: {}", e)))?;
    let mut content_file = archive
        .by_name("content.xml")
        .map_err(|_| ImportError::Parse("missing content.xml".into()))?;
    let mut content = String::new();
    content_file.read_to_string(&mut content)?;

    let mut reader = XmlReader::from_str(&content);
    let mut text = String::new();
    let mut in_cell = false;
    loop {
        match reader.read_event() {
            Ok(Event::Start(ref e)) => {
                if e.name().as_ref() == b"table:table-cell" || e.name().as_ref() == b"text:p" {
                    in_cell = true;
                }
            }
            Ok(Event::Text(ref e)) if in_cell => {
                if let Ok(t) = e.unescape() {
                    text.push_str(&t);
                }
            }
            Ok(Event::End(ref e)) => {
                if e.name().as_ref() == b"table:table-cell" {
                    text.push('\t');
                } else if e.name().as_ref() == b"table:table-row" {
                    text.push('\n');
                }
                if e.name().as_ref() == b"table:table-cell" || e.name().as_ref() == b"text:p" {
                    in_cell = false;
                }
            }
            Ok(Event::Eof) => break,
            Err(_) => break,
            _ => {}
        }
    }
    Ok(text)
}

pub struct OdtImporter;

impl Default for OdtImporter {
    fn default() -> Self {
        Self::new()
    }
}

impl OdtImporter {
    pub fn new() -> Self {
        Self
    }
}

#[async_trait]
impl ImportAdapter for OdtImporter {
    fn can_import(&self, path: &Path) -> bool {
        matches!(
            path.extension()
                .and_then(|e| e.to_str())
                .map(|e| e.to_lowercase())
                .as_deref(),
            Some("odt") | Some("ott")
        )
    }

    async fn import(&self, path: &Path) -> Result<ImportResult, ImportError> {
        let text = extract_odf_text(path, "content.xml")?;
        let entity = Entity::new(EntityType::new("Article"));
        let title = path
            .file_stem()
            .and_then(|s| s.to_str())
            .unwrap_or("Untitled")
            .to_string();
        let fmt = path
            .extension()
            .and_then(|e| e.to_str())
            .unwrap_or("odt")
            .to_lowercase();
        let components = vec![
            Component::new(entity.id, ComponentType::Title, serde_json::json!(title)),
            Component::new(entity.id, ComponentType::Content, serde_json::json!(text)),
            Component::new(
                entity.id,
                ComponentType::Provenance,
                serde_json::json!({
                    "source": path.to_string_lossy(),
                    "imported_at": chrono::Utc::now().to_rfc3339(),
                    "format": fmt,
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
        &["odt", "ott"]
    }
}

pub struct OdsImporter;

impl Default for OdsImporter {
    fn default() -> Self {
        Self::new()
    }
}

impl OdsImporter {
    pub fn new() -> Self {
        Self
    }
}

#[async_trait]
impl ImportAdapter for OdsImporter {
    fn can_import(&self, path: &Path) -> bool {
        matches!(
            path.extension()
                .and_then(|e| e.to_str())
                .map(|e| e.to_lowercase())
                .as_deref(),
            Some("ods") | Some("ots")
        )
    }

    async fn import(&self, path: &Path) -> Result<ImportResult, ImportError> {
        let text = extract_ods_text(path)?;
        let entity = Entity::new(EntityType::new("Article"));
        let title = path
            .file_stem()
            .and_then(|s| s.to_str())
            .unwrap_or("Untitled")
            .to_string();
        let fmt = path
            .extension()
            .and_then(|e| e.to_str())
            .unwrap_or("ods")
            .to_lowercase();
        let components = vec![
            Component::new(entity.id, ComponentType::Title, serde_json::json!(title)),
            Component::new(entity.id, ComponentType::Content, serde_json::json!(text)),
            Component::new(
                entity.id,
                ComponentType::Provenance,
                serde_json::json!({
                    "source": path.to_string_lossy(),
                    "imported_at": chrono::Utc::now().to_rfc3339(),
                    "format": fmt,
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
        &["ods", "ots"]
    }
}

pub struct OdpImporter;

impl Default for OdpImporter {
    fn default() -> Self {
        Self::new()
    }
}

impl OdpImporter {
    pub fn new() -> Self {
        Self
    }
}

#[async_trait]
impl ImportAdapter for OdpImporter {
    fn can_import(&self, path: &Path) -> bool {
        matches!(
            path.extension()
                .and_then(|e| e.to_str())
                .map(|e| e.to_lowercase())
                .as_deref(),
            Some("odp") | Some("otp")
        )
    }

    async fn import(&self, path: &Path) -> Result<ImportResult, ImportError> {
        let text = extract_odf_text(path, "content.xml")?;
        let entity = Entity::new(EntityType::new("Article"));
        let title = path
            .file_stem()
            .and_then(|s| s.to_str())
            .unwrap_or("Untitled")
            .to_string();
        let fmt = path
            .extension()
            .and_then(|e| e.to_str())
            .unwrap_or("odp")
            .to_lowercase();
        let components = vec![
            Component::new(entity.id, ComponentType::Title, serde_json::json!(title)),
            Component::new(entity.id, ComponentType::Content, serde_json::json!(text)),
            Component::new(
                entity.id,
                ComponentType::Provenance,
                serde_json::json!({
                    "source": path.to_string_lossy(),
                    "imported_at": chrono::Utc::now().to_rfc3339(),
                    "format": fmt,
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
        &["odp", "otp"]
    }
}

pub struct OdgImporter;

impl Default for OdgImporter {
    fn default() -> Self {
        Self::new()
    }
}

impl OdgImporter {
    pub fn new() -> Self {
        Self
    }
}

#[async_trait]
impl ImportAdapter for OdgImporter {
    fn can_import(&self, path: &Path) -> bool {
        path.extension()
            .and_then(|e| e.to_str())
            .map(|e| e.eq_ignore_ascii_case("odg"))
            .unwrap_or(false)
    }

    async fn import(&self, path: &Path) -> Result<ImportResult, ImportError> {
        let text = extract_odf_text(path, "content.xml")?;
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
                    "format": "odg",
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
        &["odg"]
    }
}

pub struct OttImporter;

impl Default for OttImporter {
    fn default() -> Self {
        Self::new()
    }
}

impl OttImporter {
    pub fn new() -> Self {
        Self
    }
}

#[async_trait]
impl ImportAdapter for OttImporter {
    fn can_import(&self, path: &Path) -> bool {
        path.extension()
            .and_then(|e| e.to_str())
            .map(|e| e.eq_ignore_ascii_case("ott"))
            .unwrap_or(false)
    }

    async fn import(&self, path: &Path) -> Result<ImportResult, ImportError> {
        let text = extract_odf_text(path, "content.xml")?;
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
                    "format": "ott",
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
        &["ott"]
    }
}

pub struct OtsImporter;

impl Default for OtsImporter {
    fn default() -> Self {
        Self::new()
    }
}

impl OtsImporter {
    pub fn new() -> Self {
        Self
    }
}

#[async_trait]
impl ImportAdapter for OtsImporter {
    fn can_import(&self, path: &Path) -> bool {
        path.extension()
            .and_then(|e| e.to_str())
            .map(|e| e.eq_ignore_ascii_case("ots"))
            .unwrap_or(false)
    }

    async fn import(&self, path: &Path) -> Result<ImportResult, ImportError> {
        let text = extract_ods_text(path)?;
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
                    "format": "ots",
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
        &["ots"]
    }
}

pub struct OtpImporter;

impl Default for OtpImporter {
    fn default() -> Self {
        Self::new()
    }
}

impl OtpImporter {
    pub fn new() -> Self {
        Self
    }
}

#[async_trait]
impl ImportAdapter for OtpImporter {
    fn can_import(&self, path: &Path) -> bool {
        path.extension()
            .and_then(|e| e.to_str())
            .map(|e| e.eq_ignore_ascii_case("otp"))
            .unwrap_or(false)
    }

    async fn import(&self, path: &Path) -> Result<ImportResult, ImportError> {
        let text = extract_odf_text(path, "content.xml")?;
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
                    "format": "otp",
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
        &["otp"]
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_odt_can_import() {
        let importer = OdtImporter::new();
        assert!(importer.can_import(Path::new("test.odt")));
        assert!(importer.can_import(Path::new("test.ODT")));
        assert!(!importer.can_import(Path::new("test.pdf")));
    }

    #[test]
    fn test_odt_supported_extensions() {
        let importer = OdtImporter::new();
        assert_eq!(importer.supported_extensions(), &["odt", "ott"]);
    }

    #[test]
    fn test_ods_can_import() {
        let importer = OdsImporter::new();
        assert!(importer.can_import(Path::new("test.ods")));
        assert!(!importer.can_import(Path::new("test.pdf")));
    }

    #[test]
    fn test_odp_can_import() {
        let importer = OdpImporter::new();
        assert!(importer.can_import(Path::new("test.odp")));
        assert!(!importer.can_import(Path::new("test.pdf")));
    }

    #[test]
    fn test_ott_treated_as_odt_template() {
        let importer = OdtImporter::new();
        assert!(importer.can_import(Path::new("test.ott")));
        let importer_ott = OttImporter::new();
        assert!(importer_ott.can_import(Path::new("test.ott")));
    }

    #[test]
    fn test_odf_unsupported_format() {
        let importer = OdtImporter::new();
        assert!(!importer.can_import(Path::new("test.txt")));
    }
}
