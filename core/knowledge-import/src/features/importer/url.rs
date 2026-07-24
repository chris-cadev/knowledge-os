use async_trait::async_trait;
use std::path::Path;

use super::adapter::{ImportError, ImportAdapter, ImportResult};

/// URL importer that fetches content from web URLs.
pub struct UrlImporter {
    client: reqwest::Client,
}

impl UrlImporter {
    pub fn new() -> Self {
        let client = reqwest::Client::builder()
            .user_agent("KnowledgeOS/0.1")
            .timeout(std::time::Duration::from_secs(30))
            .build()
            .expect("Failed to create HTTP client");

        Self { client }
    }

    /// Import content from a URL string.
    pub async fn import_url(&self, url: &str) -> Result<ImportResult, ImportError> {
        let response = self
            .client
            .get(url)
            .send()
            .await
            .map_err(|e| ImportError::Network(format!("Failed to fetch URL: {}", e)))?;

        let status = response.status();
        if !status.is_success() {
            return Err(ImportError::Network(format!(
                "HTTP {} from {}",
                status, url
            )));
        }

        let content_type = response
            .headers()
            .get("content-type")
            .and_then(|v| v.to_str().ok())
            .unwrap_or("text/html")
            .to_string();

        // Handle PDF content type — download binary and use PdfImporter
        if content_type.contains("pdf") || url.ends_with(".pdf") {
            let bytes = response
                .bytes()
                .await
                .map_err(|e| ImportError::Network(format!("Failed to read PDF response: {}", e)))?;

            return self.import_pdf_from_bytes(&bytes, url).await;
        }

        let body = response
            .text()
            .await
            .map_err(|e| ImportError::Network(format!("Failed to read response: {}", e)))?;

        self.import_from_content(&body, url, &content_type)
    }

    /// Import a PDF from downloaded bytes by writing to a temp file and using PdfImporter.
    async fn import_pdf_from_bytes(
        &self,
        bytes: &[u8],
        url: &str,
    ) -> Result<ImportResult, ImportError> {
        use super::pdf::PdfImporter;

        let tmp = tempfile::NamedTempFile::new()
            .map_err(ImportError::Io)?;
        std::fs::write(tmp.path(), bytes)
            .map_err(ImportError::Io)?;

        let pdf_importer = PdfImporter::new();
        let mut result = pdf_importer.import(tmp.path()).await?;

        // Override the provenance source with the URL
        for comp in &mut result.components {
            if comp.component_type == knowledge_core::features::component::ComponentType::Provenance {
                comp.data = serde_json::json!({
                    "source": url,
                    "imported_at": chrono::Utc::now().to_rfc3339(),
                    "format": "url-pdf",
                });
            }
        }

        Ok(result)
    }

    fn import_from_content(
        &self,
        content: &str,
        url: &str,
        content_type: &str,
    ) -> Result<ImportResult, ImportError> {
        use knowledge_core::features::component::{Component, ComponentType};
        use knowledge_core::features::entity::{Entity, EntityType};

        let entity = Entity::new(EntityType::new("Article"));

        // Extract title from HTML or use URL
        let title = if content_type.contains("html") {
            extract_html_title(content).unwrap_or_else(|| url.to_string())
        } else {
            url.to_string()
        };

        // Convert HTML to text (simple extraction)
        let text_content = if content_type.contains("html") {
            html_to_text(content)
        } else {
            content.to_string()
        };

        let components = vec![
            Component::new(
                entity.id,
                ComponentType::Title,
                serde_json::json!(title),
            ),
            Component::new(
                entity.id,
                ComponentType::Content,
                serde_json::json!(text_content),
            ),
            Component::new(
                entity.id,
                ComponentType::Provenance,
                serde_json::json!({
                    "source": url,
                    "imported_at": chrono::Utc::now().to_rfc3339(),
                    "format": "url",
                    "content_type": content_type,
                }),
            ),
            Component::new(
                entity.id,
                ComponentType::Timeline,
                serde_json::json!({
                    "imported_at": chrono::Utc::now().to_rfc3339(),
                }),
            ),
            Component::new(
                entity.id,
                ComponentType::Language,
                serde_json::json!("en"),
            ),
        ];

        Ok(ImportResult {
            entity,
            components,
            cross_references: vec![],
        })
    }
}

impl Default for UrlImporter {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl ImportAdapter for UrlImporter {
    fn can_import(&self, path: &Path) -> bool {
        path.to_string_lossy().starts_with("http://")
            || path.to_string_lossy().starts_with("https://")
    }

    async fn import(&self, path: &Path) -> Result<ImportResult, ImportError> {
        let url = path.to_string_lossy();
        self.import_url(&url).await
    }

    fn supported_extensions(&self) -> &[&str] {
        &[] // URLs don't have file extensions
    }
}

fn extract_html_title(html: &str) -> Option<String> {
    // Simple regex to find <title> tag
    let re = regex::Regex::new(r"(?i)<title[^>]*>([^<]+)</title>").ok()?;
    let caps = re.captures(html)?;
    Some(caps[1].trim().to_string())
}

fn html_to_text(html: &str) -> String {
    // Very simple HTML to text conversion
    // In production, use a proper HTML parser like html5ever or scraper
    let mut text = String::new();
    let mut in_tag = false;

    for c in html.chars() {
        match c {
            '<' => in_tag = true,
            '>' if in_tag => {
                in_tag = false;
                text.push(' ');
            }
            '&' if !in_tag => {
                // Handle common HTML entities
                text.push(' ');
            }
            _ if !in_tag => {
                text.push(c);
            }
            _ => {}
        }
    }

    // Collapse whitespace
    let mut result = String::new();
    let mut prev_was_space = false;
    for c in text.chars() {
        if c.is_whitespace() {
            if !prev_was_space {
                result.push(' ');
                prev_was_space = true;
            }
        } else {
            result.push(c);
            prev_was_space = false;
        }
    }

    result.trim().to_string()
}

#[cfg(test)]
mod tests {
    use super::*;
    use knowledge_core::features::component::ComponentType;
    use knowledge_core::features::entity::EntityType;

    #[test]
    fn test_can_import_url() {
        let importer = UrlImporter::new();

        assert!(importer.can_import(Path::new("https://example.com")));
        assert!(importer.can_import(Path::new("http://example.com/page")));
        assert!(!importer.can_import(Path::new("file.md")));
    }

    #[test]
    fn test_extract_html_title() {
        let html = r#"<!DOCTYPE html>
<html>
<head><title>My Page Title</title></head>
<body></body>
</html>"#;

        assert_eq!(extract_html_title(html), Some("My Page Title".to_string()));
    }

    #[test]
    fn test_html_to_text() {
        let html = "<h1>Hello</h1><p>This is a <strong>test</strong>.</p>";
        let text = html_to_text(html);
        assert!(text.contains("Hello"));
        assert!(text.contains("test"));
    }

    #[test]
    fn test_import_from_content() {
        let importer = UrlImporter::new();
        let html = r#"<!DOCTYPE html>
<html>
<head><title>Test Page</title></head>
<body><p>Some content here.</p></body>
</html>"#;

        let result = importer
            .import_from_content(html, "https://example.com/test", "text/html")
            .unwrap();

        assert_eq!(result.entity.entity_type, EntityType::new("Article"));
        assert!(!result.components.is_empty());

        let title = result
            .components
            .iter()
            .find(|c| c.component_type == ComponentType::Title)
            .unwrap();
        assert_eq!(title.data, serde_json::json!("Test Page"));
    }
}
