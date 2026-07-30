use std::path::Path;

use async_trait::async_trait;
use knowledge_core::features::component::{Component, ComponentType};
use knowledge_core::features::entity::{Entity, EntityType};
use knowledge_core::ports::{OcrProvider, PluginManifest, PluginMetadata};

use super::adapter::{ImportAdapter, ImportError, ImportResult};

pub struct ImageImporter {
    ocr_provider: Option<Box<dyn OcrProvider>>,
}

impl ImageImporter {
    pub fn new() -> Self {
        Self { ocr_provider: None }
    }

    pub fn with_ocr(ocr_provider: Box<dyn OcrProvider>) -> Self {
        Self {
            ocr_provider: Some(ocr_provider),
        }
    }
}

impl Default for ImageImporter {
    fn default() -> Self {
        Self::new()
    }
}

impl PluginMetadata for ImageImporter {
    fn manifest(&self) -> PluginManifest {
        PluginManifest {
            name: "image-importer".to_string(),
            version: "0.1.0".to_string(),
            description: "Import image files with OCR text extraction".to_string(),
            author: "Knowledge OS".to_string(),
            license: Some("MIT".to_string()),
            priority: Some(100),
        }
    }
}

const SUPPORTED_EXTENSIONS: &[&str] = &["png", "jpg", "jpeg", "gif", "bmp"];

#[async_trait]
impl ImportAdapter for ImageImporter {
    fn can_import(&self, path: &Path) -> bool {
        path.extension()
            .and_then(|e| e.to_str())
            .map(|ext| {
                ext.eq_ignore_ascii_case("png")
                    || ext.eq_ignore_ascii_case("jpg")
                    || ext.eq_ignore_ascii_case("jpeg")
                    || ext.eq_ignore_ascii_case("gif")
                    || ext.eq_ignore_ascii_case("bmp")
            })
            .unwrap_or(false)
    }

    async fn import(&self, path: &Path) -> Result<ImportResult, ImportError> {
        let bytes = std::fs::read(path)?;
        let mime_type = mime_for_extension(path)
            .ok_or_else(|| ImportError::UnsupportedFormat("unknown image mime type".to_string()))?;
        let title = path
            .file_stem()
            .and_then(|s| s.to_str())
            .unwrap_or("Untitled Image")
            .to_string();

        let entity = Entity::new(EntityType::new("Image"));
        let size = bytes.len();

        let mut components = vec![
            Component::new(entity.id, ComponentType::Title, serde_json::json!(title)),
            Component::new(
                entity.id,
                ComponentType::BinaryContent,
                serde_json::json!({
                    "mime_type": &mime_type,
                    "size": size,
                }),
            ),
        if let Some(ref ocr) = self.ocr_provider {
            match ocr.process_image(entity.id, bytes, mime_type).await {
                Ok(text) => {
                    components.push(Component::new(
                        entity.id,
                        ComponentType::Content,
                        serde_json::json!({ "markdown": text }),
                    ));
                }
                Err(e) => {
                    eprintln!("WARNING: OCR failed for {}: {}", path.display(), e);
                }
            }
        }

        components.push(Component::new(
            entity.id,
            ComponentType::Provenance,
            serde_json::json!({
                "source": path.to_string_lossy(),
                "imported_at": chrono::Utc::now().to_rfc3339(),
                "format": "image",
            }),
        ));

        let cross_references = Vec::new();

        Ok(ImportResult {
            entity,
            components,
            cross_references,
        })
    }

    fn supported_extensions(&self) -> &[&str] {
        SUPPORTED_EXTENSIONS
    }
}

fn mime_for_extension(path: &Path) -> Option<String> {
    match path
        .extension()
        .and_then(|e| e.to_str())
        .map(|e| e.to_lowercase())
        .as_deref()
    {
        Some("png") => Some("image/png".to_string()),
        Some("jpg") | Some("jpeg") => Some("image/jpeg".to_string()),
        Some("gif") => Some("image/gif".to_string()),
        Some("bmp") => Some("image/bmp".to_string()),
        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Write;
    use uuid::Uuid;

    struct MockOcrProvider;

    #[async_trait]
    impl OcrProvider for MockOcrProvider {
        async fn process_image(
            &self,
            _entity_id: Uuid,
            _image_bytes: Vec<u8>,
            _mime_type: String,
        ) -> Result<String, String> {
            Ok("OCR extracted text".to_string())
        }
    }

    fn create_minimal_png() -> tempfile::NamedTempFile {
        let mut file = tempfile::NamedTempFile::with_suffix(".png").unwrap();
        // Minimal valid PNG: 1x1 red pixel
        let png_data: Vec<u8> = vec![
            0x89, 0x50, 0x4e, 0x47, 0x0d, 0x0a, 0x1a, 0x0a, // PNG signature
            0x00, 0x00, 0x00, 0x0d, 0x49, 0x48, 0x44, 0x52, // IHDR chunk length + type
            0x00, 0x00, 0x00, 0x01, 0x00, 0x00, 0x00, 0x01, // width=1, height=1
            0x08, 0x02, 0x00, 0x00, 0x00, 0x90, 0x77, 0x53, // bit depth, color type
            0xde, 0x00, 0x00, 0x00, 0x0c, 0x49, 0x48, 0x44, // CRC + IDAT chunk
            0x54, 0x08, 0xd7, 0x63, 0xf8, 0xcf, 0xc0, 0x00, 0x00, 0x00, 0x03, 0x00, 0x01, 0x36,
            0x28, 0x19, 0x00, 0x00, 0x00, 0x00, 0x49, 0x45, 0x4e, 0x44, // IEND chunk
            0xae, 0x42, 0x60, 0x82,
        ];
        file.write_all(&png_data).unwrap();
        file.flush().unwrap();
        file
    }

    #[test]
    fn test_image_adapter_can_import() {
        let adapter = ImageImporter::new();
        assert!(adapter.can_import(Path::new("test.png")));
        assert!(adapter.can_import(Path::new("test.jpg")));
        assert!(adapter.can_import(Path::new("test.jpeg")));
        assert!(adapter.can_import(Path::new("test.gif")));
        assert!(adapter.can_import(Path::new("test.bmp")));
        assert!(adapter.can_import(Path::new("test.JPG")));
        assert!(!adapter.can_import(Path::new("test.pdf")));
        assert!(!adapter.can_import(Path::new("test.md")));
    }

    #[test]
    fn test_supported_extensions() {
        let adapter = ImageImporter::new();
        assert_eq!(
            adapter.supported_extensions(),
            &["png", "jpg", "jpeg", "gif", "bmp"]
        );
    }

    #[tokio::test]
    async fn test_image_import_without_ocr() {
        let file = create_minimal_png();
        let adapter = ImageImporter::new();
        let result = adapter.import(file.path()).await.unwrap();

        assert_eq!(result.entity.entity_type, EntityType::new("Image"));
        assert!(result
            .components
            .iter()
            .any(|c| c.component_type == ComponentType::Title));
        assert!(result
            .components
            .iter()
            .any(|c| c.component_type == ComponentType::BinaryContent));
        assert!(result
            .components
            .iter()
            .any(|c| c.component_type == ComponentType::Provenance));
    }

    #[tokio::test]
    async fn test_image_import_with_ocr() {
        let file = create_minimal_png();
        let ocr = Box::new(MockOcrProvider);
        let adapter = ImageImporter::with_ocr(ocr);
        let result = adapter.import(file.path()).await.unwrap();

        assert!(result
            .components
            .iter()
            .any(|c| c.component_type == ComponentType::Content));
        let content = result
            .components
            .iter()
            .find(|c| c.component_type == ComponentType::Content)
            .unwrap();
        assert_eq!(
            content.data.get("markdown").and_then(|v| v.as_str()),
            Some("OCR extracted text")
        );
    }
}
