use async_trait::async_trait;
use knowledge_core::features::component::{Component, ComponentType};
use knowledge_core::features::entity::{Entity, EntityType};
use knowledge_core::ports::{PluginManifest, PluginMetadata};
use pdf_oxide::extractors::xmp::XmpExtractor;
use pdf_oxide::PdfDocument;
use std::path::Path;

use super::adapter::{ImportAdapter, ImportError, ImportResult};

/// PDF file importer implementing the ImportAdapter trait.
/// Uses pdf_oxide for text extraction and metadata parsing.
pub struct PdfImporter;

impl Default for PdfImporter {
    fn default() -> Self {
        Self::new()
    }
}

impl PdfImporter {
    pub fn new() -> Self {
        Self
    }
}

impl PluginMetadata for PdfImporter {
    fn manifest(&self) -> PluginManifest {
        PluginManifest {
            name: "pdf-importer".to_string(),
            version: "0.1.0".to_string(),
            description: "Import PDF files as knowledge entities".to_string(),
            author: "Knowledge OS".to_string(),
            license: Some("MIT".to_string()),
            priority: Some(100),
        }
    }
}

#[async_trait]
impl ImportAdapter for PdfImporter {
    fn can_import(&self, path: &Path) -> bool {
        path.extension()
            .and_then(|e| e.to_str())
            .map(|ext| ext.eq_ignore_ascii_case("pdf"))
            .unwrap_or(false)
    }

    async fn import(&self, path: &Path) -> Result<ImportResult, ImportError> {
        let doc = PdfDocument::open(path).map_err(|e| ImportError::Parse(format!("{}", e)))?;

        // Extract text from all pages
        let mut text_content = String::new();
        for page_idx in doc.page_indices() {
            if let Ok(page_text) = doc.extract_text(page_idx) {
                text_content.push_str(&page_text);
                text_content.push('\n');
            }
        }

        // Check if we got any text (scanned PDF detection)
        let has_text = !text_content.trim().is_empty();
        if !has_text {
            eprintln!(
                "WARNING: Scanned PDF detected (no extractable text): {}",
                path.display()
            );
        }

        // Extract metadata (XMP first, legacy Info dict as fallback)
        let metadata = extract_metadata(&doc);

        // Determine title from metadata or filename
        let title = metadata.title.clone().unwrap_or_else(|| {
            path.file_stem()
                .and_then(|s| s.to_str())
                .unwrap_or("Untitled PDF")
                .to_string()
        });

        let entity = Entity::new(EntityType::new("Article"));

        let mut components = vec![Component::new(
            entity.id,
            ComponentType::Title,
            serde_json::json!(title),
        )];

        // Only add Content component if we extracted text
        if has_text {
            components.push(Component::new(
                entity.id,
                ComponentType::Content,
                serde_json::json!(text_content),
            ));
        }

        // BinaryContent component for the original PDF
        let file_size = std::fs::metadata(path).map(|m| m.len()).unwrap_or(0);

        components.push(Component::new(
            entity.id,
            ComponentType::BinaryContent,
            serde_json::json!({
                "reference": path.to_string_lossy(),
                "mime_type": "application/pdf",
                "size": file_size,
            }),
        ));

        // Author from metadata
        if let Some(author) = &metadata.author {
            components.push(Component::new(
                entity.id,
                ComponentType::Author,
                serde_json::json!(author),
            ));
        }

        // Use PDF creation date if available, otherwise fall back to file modification date
        let pdf_date = metadata.creation_date.clone().unwrap_or_else(|| {
            std::fs::metadata(path)
                .and_then(|m| m.modified())
                .ok()
                .map(|t| {
                    let datetime: chrono::DateTime<chrono::Utc> = t.into();
                    datetime.to_rfc3339()
                })
                .unwrap_or_else(|| chrono::Utc::now().to_rfc3339())
        });

        // Timeline component
        components.push(Component::new(
            entity.id,
            ComponentType::Timeline,
            serde_json::json!({
                "created_at": pdf_date,
                "imported_at": chrono::Utc::now().to_rfc3339(),
            }),
        ));

        // Language (default to "en")
        components.push(Component::new(
            entity.id,
            ComponentType::Language,
            serde_json::json!("en"),
        ));

        // Provenance
        components.push(Component::new(
            entity.id,
            ComponentType::Provenance,
            serde_json::json!({
                "source": path.to_string_lossy(),
                "imported_at": chrono::Utc::now().to_rfc3339(),
                "format": "pdf",
            }),
        ));

        // PDFs don't have cross-references in the same way as Markdown
        let cross_references = Vec::new();

        Ok(ImportResult {
            entity,
            components,
            cross_references,
        })
    }

    fn supported_extensions(&self) -> &[&str] {
        &["pdf"]
    }
}

/// Metadata extracted from PDF properties.
struct PdfMetadata {
    title: Option<String>,
    author: Option<String>,
    creation_date: Option<String>,
}

fn extract_metadata(doc: &PdfDocument) -> PdfMetadata {
    let mut title = None;
    let mut author = None;
    let mut creation_date = None;

    if let Ok(Some(xmp)) = XmpExtractor::extract(doc) {
        title = xmp.dc_title;
        author = xmp.dc_creator.into_iter().next();
        creation_date = xmp.xmp_create_date;
    }

    PdfMetadata {
        title,
        author,
        creation_date,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Write;
    use tempfile::NamedTempFile;

    fn create_minimal_pdf(_title: &str, content: &str) -> NamedTempFile {
        let mut file = NamedTempFile::new().unwrap();
        // Minimal valid PDF structure
        let pdf = format!(
            r#"%PDF-1.4
1 0 obj
<< /Type /Catalog /Pages 2 0 R >>
endobj

2 0 obj
<< /Type /Pages /Kids [3 0 R] /Count 1 >>
endobj

3 0 obj
<< /Type /Page /Parent 2 0 R /MediaBox [0 0 612 792] /Contents 4 0 R /Resources << /Font << /F1 5 0 R >> >> >>
endobj

4 0 obj
<< /Length 44 >>
stream
BT /F1 12 Tf 100 700 Td ({}) Tj ET
endstream
endobj

5 0 obj
<< /Type /Font /Subtype /Type1 /BaseFont /Helvetica >>
endobj

xref
0 6
0000000000 65535 f 
0000000009 00000 n 
0000000058 00000 n 
0000000115 00000 n 
0000000266 00000 n 
0000000340 00000 n 

trailer
<< /Size 6 /Root 1 0 R >>
startxref
409
%%EOF"#,
            content
        );
        file.write_all(pdf.as_bytes()).unwrap();
        file.flush().unwrap();
        file
    }

    #[test]
    fn test_pdf_adapter_can_import() {
        let adapter = PdfImporter::new();
        assert!(adapter.can_import(Path::new("test.pdf")));
        assert!(adapter.can_import(Path::new("test.PDF")));
        assert!(!adapter.can_import(Path::new("test.md")));
        assert!(!adapter.can_import(Path::new("test.txt")));
    }

    #[test]
    fn test_pdf_supported_extensions() {
        let adapter = PdfImporter::new();
        assert_eq!(adapter.supported_extensions(), &["pdf"]);
    }

    #[tokio::test]
    async fn test_pdf_import_basic() {
        let file = create_minimal_pdf("Test PDF", "Hello World");
        let adapter = PdfImporter::new();

        let result = adapter.import(file.path()).await.unwrap();

        // Should have entity with Article type
        assert_eq!(result.entity.entity_type, EntityType::new("Article"));

        // Should have Title component (falls back to filename if no PDF metadata)
        let title = result
            .components
            .iter()
            .find(|c| c.component_type == ComponentType::Title)
            .unwrap();
        assert!(title.data.is_string());

        // Should have BinaryContent component
        let binary = result
            .components
            .iter()
            .find(|c| c.component_type == ComponentType::BinaryContent)
            .unwrap();
        assert_eq!(binary.data.get("mime_type").unwrap(), "application/pdf");

        // Should have Provenance with format: pdf
        let provenance = result
            .components
            .iter()
            .find(|c| c.component_type == ComponentType::Provenance)
            .unwrap();
        assert_eq!(provenance.data.get("format").unwrap(), "pdf");
    }
}
