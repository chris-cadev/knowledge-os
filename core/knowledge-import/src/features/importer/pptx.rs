use async_trait::async_trait;
use knowledge_core::features::component::{Component, ComponentType};
use knowledge_core::features::entity::{Entity, EntityType};
use quick_xml::events::Event;
use quick_xml::Reader as XmlReader;
use std::io::Read;
use std::path::Path;
use zip::ZipArchive;

use super::adapter::{ImportAdapter, ImportError, ImportResult};

pub struct PptxImporter;

impl PptxImporter {
    pub fn new() -> Self {
        Self
    }

    fn extract_text_from_slide(xml: &[u8]) -> Result<String, ImportError> {
        let mut reader = XmlReader::from_reader(xml);
        let mut in_t = false;
        let mut text = String::new();

        loop {
            match reader.read_event() {
                Ok(Event::Start(ref e)) | Ok(Event::Empty(ref e)) => {
                    if e.name().as_ref() == b"a:t" {
                        in_t = true;
                    }
                }
                Ok(Event::Text(ref e)) if in_t => {
                    if let Ok(t) = e.unescape() {
                        text.push_str(&t);
                        text.push('\n');
                    }
                }
                Ok(Event::End(ref e)) => {
                    if e.name().as_ref() == b"a:t" {
                        in_t = false;
                    }
                }
                Ok(Event::Eof) => break,
                Err(e) => return Err(ImportError::Parse(format!("XML error: {}", e))),
                _ => {}
            }
        }

        Ok(text)
    }
}

#[async_trait]
impl ImportAdapter for PptxImporter {
    fn can_import(&self, path: &Path) -> bool {
        let ext = path
            .extension()
            .and_then(|e| e.to_str())
            .unwrap_or("");
        ext.eq_ignore_ascii_case("pptx") || ext.eq_ignore_ascii_case("pps")
    }

    async fn import(&self, path: &Path) -> Result<ImportResult, ImportError> {
        let file = std::fs::File::open(path)?;
        let mut archive = ZipArchive::new(file)
            .map_err(|e| ImportError::Parse(format!("not a valid ZIP/PPTX: {}", e)))?;

        let mut slide_texts: Vec<(usize, String)> = Vec::new();

        for i in 0..archive.len() {
            let mut entry = archive.by_index(i).map_err(|e| ImportError::Parse(e.to_string()))?;
            let name = entry.name().to_string();

            if name.starts_with("ppt/slides/slide") && name.ends_with(".xml") {
                let mut content = Vec::new();
                entry.read_to_end(&mut content)?;

                let slide_num: usize = name
                    .trim_start_matches("ppt/slides/slide")
                    .trim_end_matches(".xml")
                    .parse()
                    .unwrap_or(0);

                let text = Self::extract_text_from_slide(&content)?;
                slide_texts.push((slide_num, text));
            }
        }

        slide_texts.sort_by_key(|(num, _)| *num);

        let mut full_text = String::new();
        for (i, text) in &slide_texts {
            full_text.push_str(&format!("--- Slide {} ---\n", i));
            full_text.push_str(text);
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
                serde_json::json!(full_text),
            ),
            Component::new(
                entity.id,
                ComponentType::Provenance,
                serde_json::json!({
                    "source": path.to_string_lossy(),
                    "imported_at": chrono::Utc::now().to_rfc3339(),
                    "format": "pptx",
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
        &["pptx"]
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_can_import() {
        let importer = PptxImporter::new();
        assert!(importer.can_import(Path::new("test.pptx")));
        assert!(importer.can_import(Path::new("test.PPTX")));
        assert!(!importer.can_import(Path::new("test.txt")));
    }

    #[test]
    fn test_supported_extensions() {
        let importer = PptxImporter::new();
        assert_eq!(importer.supported_extensions(), &["pptx"]);
    }
}
