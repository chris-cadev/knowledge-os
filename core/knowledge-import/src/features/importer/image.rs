use async_trait::async_trait;
use knowledge_core::features::component::{Component, ComponentType};
use knowledge_core::features::entity::{Entity, EntityType};
use std::path::Path;

use super::adapter::{ImportAdapter, ImportError, ImportResult};

pub struct ImageImporter;

impl Default for ImageImporter {
    fn default() -> Self {
        Self::new()
    }
}

impl ImageImporter {
    pub fn new() -> Self {
        Self
    }
}

#[async_trait]
impl ImportAdapter for ImageImporter {
    fn can_import(&self, path: &Path) -> bool {
        let ext = path
            .extension()
            .and_then(|e| e.to_str())
            .map(|e| e.to_lowercase())
            .unwrap_or_default();
        matches!(
            ext.as_str(),
            "png" | "jpg" | "jpeg" | "gif" | "bmp" | "webp" | "tiff" | "tif"
        )
    }

    async fn import(&self, path: &Path) -> Result<ImportResult, ImportError> {
        let bytes = std::fs::read(path)?;
        let file_size = bytes.len();

        let mime_type = path
            .extension()
            .and_then(|e| e.to_str())
            .map(|e| match e.to_lowercase().as_str() {
                "png" => "image/png",
                "jpg" | "jpeg" => "image/jpeg",
                "gif" => "image/gif",
                "bmp" => "image/bmp",
                "webp" => "image/webp",
                "tiff" | "tif" => "image/tiff",
                _ => "application/octet-stream",
            })
            .unwrap_or("application/octet-stream");

        let entity = Entity::new(EntityType::new("Image"));
        let title = path
            .file_stem()
            .and_then(|s| s.to_str())
            .unwrap_or("Untitled Image")
            .to_string();

        let components = vec![
            Component::new(entity.id, ComponentType::Title, serde_json::json!(title)),
            Component::new(
                entity.id,
                ComponentType::BinaryContent,
                serde_json::json!({
                    "reference": path.to_string_lossy(),
                    "mime_type": mime_type,
                    "size": file_size,
                    "bytes": bytes.len(),
                }),
            ),
            Component::new(
                entity.id,
                ComponentType::Provenance,
                serde_json::json!({
                    "source": path.to_string_lossy(),
                    "imported_at": chrono::Utc::now().to_rfc3339(),
                    "format": "image",
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
        &["png", "jpg", "jpeg", "gif", "bmp", "webp", "tiff", "tif"]
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Write;
    use tempfile::NamedTempFile;

    fn create_minimal_png() -> NamedTempFile {
        let mut file = NamedTempFile::with_suffix(".png").unwrap();
        // Minimal valid PNG (8x8 red pixel)
        let png_data: Vec<u8> = vec![
            0x89, 0x50, 0x4E, 0x47, 0x0D, 0x0A, 0x1A, 0x0A, // PNG signature
            0x00, 0x00, 0x00, 0x0D, // IHDR chunk length
            0x49, 0x48, 0x44, 0x52, // IHDR type
            0x00, 0x00, 0x00, 0x01, // width: 1
            0x00, 0x00, 0x00, 0x01, // height: 1
            0x08, // bit depth: 8
            0x02, // color type: RGB
            0x00, // compression
            0x00, // filter
            0x00, // interlace
            0x00, 0x00, 0x00, 0x00, // CRC (dummy)
            0x00, 0x00, 0x00, 0x01, // IDAT chunk length
            0x49, 0x44, 0x41, 0x54, // IDAT type
            0x00, // data
            0x00, 0x00, 0x00, 0x00, // CRC (dummy)
            0x00, 0x00, 0x00, 0x00, // IEND chunk length
            0x49, 0x45, 0x4E, 0x44, // IEND type
            0xAE, 0x42, 0x60, 0x82, // IEND CRC
        ];
        file.write_all(&png_data).unwrap();
        file.flush().unwrap();
        file
    }

    #[test]
    fn test_can_import() {
        let importer = ImageImporter::new();
        assert!(importer.can_import(Path::new("test.png")));
        assert!(importer.can_import(Path::new("test.jpg")));
        assert!(importer.can_import(Path::new("test.jpeg")));
        assert!(importer.can_import(Path::new("test.gif")));
        assert!(!importer.can_import(Path::new("test.txt")));
    }

    #[test]
    fn test_supported_extensions() {
        let importer = ImageImporter::new();
        let exts = importer.supported_extensions();
        assert!(exts.contains(&"png"));
        assert!(exts.contains(&"jpg"));
    }

    #[tokio::test]
    async fn test_image_imports_png() {
        let file = create_minimal_png();
        let importer = ImageImporter::new();
        let result = importer.import(file.path()).await.unwrap();
        assert_eq!(result.entity.entity_type, EntityType::new("Image"));
        let binary = result
            .components
            .iter()
            .find(|c| c.component_type == ComponentType::BinaryContent)
            .unwrap();
        assert_eq!(binary.data.get("mime_type").unwrap(), "image/png");
    }
}
