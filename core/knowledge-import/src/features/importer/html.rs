use async_trait::async_trait;
use knowledge_core::features::component::{Component, ComponentType};
use knowledge_core::features::entity::{Entity, EntityType};
use knowledge_core::ports::{PluginManifest, PluginMetadata};
use scraper::{Html, Selector};
use std::path::Path;

use super::adapter::{CrossReference, ImportAdapter, ImportError, ImportResult};

pub struct HtmlImporter;

impl Default for HtmlImporter {
    fn default() -> Self {
        Self::new()
    }
}

impl HtmlImporter {
    pub fn new() -> Self {
        Self
    }
}

impl PluginMetadata for HtmlImporter {
    fn manifest(&self) -> PluginManifest {
        PluginManifest {
            name: "html-importer".to_string(),
            version: "0.1.0".to_string(),
            description: "Import HTML files as knowledge entities".to_string(),
            author: "Knowledge OS".to_string(),
            license: Some("MIT".to_string()),
            priority: Some(100),
        }
    }
}

impl HtmlImporter {
    pub fn import_content(
        &self,
        content: &str,
        source_path: &Path,
    ) -> Result<ImportResult, ImportError> {
        let metadata = extract_html_metadata(content);

        let title = metadata
            .title
            .or_else(|| extract_first_h1(content))
            .unwrap_or_else(|| {
                source_path
                    .file_stem()
                    .and_then(|s| s.to_str())
                    .unwrap_or("Untitled")
                    .to_string()
            });

        let markdown_body = html_to_markdown(content);

        let entity = Entity::new(EntityType::new("Article"));

        let mut components = vec![
            Component::new(
                entity.id,
                ComponentType::Title,
                serde_json::to_value(&title).unwrap(),
            ),
            Component::new(
                entity.id,
                ComponentType::Content,
                serde_json::to_value(&markdown_body).unwrap(),
            ),
        ];

        if let Some(author) = &metadata.author {
            components.push(Component::new(
                entity.id,
                ComponentType::Author,
                serde_json::json!(author),
            ));
        }

        if !metadata.keywords.is_empty() {
            components.push(Component::new(
                entity.id,
                ComponentType::Tags,
                serde_json::json!(metadata.keywords),
            ));
        }

        if let Some(description) = &metadata.description {
            components.push(Component::new(
                entity.id,
                ComponentType::Description,
                serde_json::json!(description),
            ));
        }

        let file_date = std::fs::metadata(source_path)
            .and_then(|m| m.modified())
            .ok()
            .map(|t| {
                let datetime: chrono::DateTime<chrono::Utc> = t.into();
                datetime.to_rfc3339()
            })
            .unwrap_or_else(|| chrono::Utc::now().to_rfc3339());

        components.push(Component::new(
            entity.id,
            ComponentType::Timeline,
            serde_json::json!({
                "created_at": file_date,
                "imported_at": chrono::Utc::now().to_rfc3339(),
            }),
        ));

        let language = metadata.language.unwrap_or_else(|| "en".to_string());
        components.push(Component::new(
            entity.id,
            ComponentType::Language,
            serde_json::json!(language),
        ));

        components.push(Component::new(
            entity.id,
            ComponentType::Provenance,
            serde_json::json!({
                "source": source_path.to_string_lossy(),
                "imported_at": chrono::Utc::now().to_rfc3339(),
                "format": "html",
            }),
        ));

        let cross_refs = extract_html_links(content, source_path);

        Ok(ImportResult {
            entity,
            components,
            cross_references: cross_refs,
        })
    }
}

#[async_trait]
impl ImportAdapter for HtmlImporter {
    fn can_import(&self, path: &Path) -> bool {
        path.extension()
            .and_then(|e| e.to_str())
            .map(|ext| ext.eq_ignore_ascii_case("html") || ext.eq_ignore_ascii_case("htm"))
            .unwrap_or(false)
    }

    async fn import(&self, path: &Path) -> Result<ImportResult, ImportError> {
        let content = std::fs::read_to_string(path)?;
        self.import_content(&content, path)
    }

    fn supported_extensions(&self) -> &[&str] {
        &["html", "htm"]
    }
}

pub struct HtmlMetadata {
    pub title: Option<String>,
    pub author: Option<String>,
    pub description: Option<String>,
    pub language: Option<String>,
    pub keywords: Vec<String>,
}

pub fn extract_html_metadata(html: &str) -> HtmlMetadata {
    let document = Html::parse_fragment(html);

    let title = Selector::parse("title")
        .ok()
        .and_then(|sel| document.select(&sel).next())
        .map(|el| el.text().collect::<String>().trim().to_string())
        .filter(|t| !t.is_empty());

    let author =
        meta_content(&document, "author").or_else(|| meta_content(&document, "dc.creator"));

    let description =
        meta_content(&document, "description").or_else(|| og_content(&document, "description"));

    let language = Selector::parse("html[lang]")
        .ok()
        .and_then(|sel| document.select(&sel).next())
        .and_then(|el| el.value().attr("lang").map(|s| s.to_string()));

    let keywords = meta_content(&document, "keywords")
        .map(|kw| {
            kw.split(',')
                .map(|s| s.trim().to_string())
                .filter(|s| !s.is_empty())
                .collect()
        })
        .unwrap_or_default();

    HtmlMetadata {
        title,
        author,
        description,
        language,
        keywords,
    }
}

fn meta_content(document: &Html, name: &str) -> Option<String> {
    let sel = Selector::parse(&format!("meta[name=\"{name}\"]")).ok()?;
    document
        .select(&sel)
        .next()
        .and_then(|el| el.value().attr("content"))
        .map(|s| s.to_string())
        .filter(|s| !s.is_empty())
}

fn og_content(document: &Html, property: &str) -> Option<String> {
    let sel = Selector::parse(&format!("meta[property=\"og:{property}\"]")).ok()?;
    document
        .select(&sel)
        .next()
        .and_then(|el| el.value().attr("content"))
        .map(|s| s.to_string())
        .filter(|s| !s.is_empty())
}

pub fn html_to_markdown(html: &str) -> String {
    let config = html2text::config::plain();
    config
        .string_from_read(html.as_bytes(), usize::MAX)
        .unwrap_or_else(|_| html_to_text_fallback(html))
}

pub fn extract_html_title(html: &str) -> Option<String> {
    let document = Html::parse_fragment(html);
    Selector::parse("title")
        .ok()
        .and_then(|sel| document.select(&sel).next())
        .map(|el| el.text().collect::<String>().trim().to_string())
        .filter(|t| !t.is_empty())
}

pub fn html_to_text(html: &str) -> String {
    let config = html2text::config::plain();
    config
        .string_from_read(html.as_bytes(), usize::MAX)
        .unwrap_or_else(|_| html_to_text_fallback(html))
}

fn html_to_text_fallback(html: &str) -> String {
    let mut text = String::new();
    let mut in_tag = false;
    for c in html.chars() {
        match c {
            '<' => in_tag = true,
            '>' if in_tag => {
                in_tag = false;
                text.push(' ');
            }
            _ if !in_tag => text.push(c),
            _ => {}
        }
    }
    text.split_whitespace().collect::<Vec<_>>().join(" ")
}

fn extract_first_h1(html: &str) -> Option<String> {
    let document = Html::parse_fragment(html);
    Selector::parse("h1")
        .ok()
        .and_then(|sel| document.select(&sel).next())
        .map(|el| el.text().collect::<String>().trim().to_string())
        .filter(|t| !t.is_empty())
}

fn extract_html_links(html: &str, source_path: &Path) -> Vec<CrossReference> {
    let document = Html::parse_fragment(html);
    let sel = match Selector::parse("a[href]") {
        Ok(s) => s,
        Err(_) => return vec![],
    };

    let mut refs = Vec::new();
    for element in document.select(&sel) {
        let href = match element.value().attr("href") {
            Some(h) if !h.is_empty() => h,
            _ => continue,
        };

        let link_text: String = element.text().collect::<String>().trim().to_string();
        let link_text = if link_text.is_empty() {
            href.to_string()
        } else {
            link_text
        };

        if href.starts_with("http://") || href.starts_with("https://") {
            refs.push(CrossReference::UrlRef {
                url: href.to_string(),
                link_text,
            });
        } else if href.starts_with("mailto:") || href.starts_with('#') {
            continue;
        } else if let Some((file_part, section)) = href.split_once('#') {
            let target_path = if file_part.is_empty() {
                source_path.to_path_buf()
            } else {
                source_path
                    .parent()
                    .unwrap_or(Path::new("."))
                    .join(file_part)
            };
            refs.push(CrossReference::SectionRef {
                target_path,
                section: section.to_string(),
                link_text,
            });
        } else {
            let target_path = source_path.parent().unwrap_or(Path::new(".")).join(href);
            refs.push(CrossReference::FileRef {
                target_path,
                link_text,
            });
        }
    }

    refs
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_import_basic_html() {
        let html = r#"<!DOCTYPE html>
<html lang="en">
<head><title>Test Page</title></head>
<body><p>Hello world.</p></body>
</html>"#;

        let importer = HtmlImporter::new();
        let result = importer
            .import_content(html, Path::new("test.html"))
            .unwrap();

        assert_eq!(result.entity.entity_type, EntityType::new("Article"));

        let title = result
            .components
            .iter()
            .find(|c| c.component_type == ComponentType::Title)
            .unwrap();
        assert_eq!(title.data, serde_json::json!("Test Page"));

        let language = result
            .components
            .iter()
            .find(|c| c.component_type == ComponentType::Language)
            .unwrap();
        assert_eq!(language.data, serde_json::json!("en"));

        let provenance = result
            .components
            .iter()
            .find(|c| c.component_type == ComponentType::Provenance)
            .unwrap();
        assert_eq!(provenance.data["format"], serde_json::json!("html"));
    }

    #[test]
    fn test_import_html_with_metadata() {
        let html = r#"<html lang="fr">
<head>
    <title>Mon Article</title>
    <meta name="author" content="Jean Dupont">
    <meta name="keywords" content="rust, web, html">
    <meta name="description" content="Un article de test">
</head>
<body><p>Contenu ici.</p></body>
</html>"#;

        let importer = HtmlImporter::new();
        let result = importer
            .import_content(html, Path::new("article.html"))
            .unwrap();

        let title = result
            .components
            .iter()
            .find(|c| c.component_type == ComponentType::Title)
            .unwrap();
        assert_eq!(title.data, serde_json::json!("Mon Article"));

        let author = result
            .components
            .iter()
            .find(|c| c.component_type == ComponentType::Author)
            .unwrap();
        assert_eq!(author.data, serde_json::json!("Jean Dupont"));

        let tags = result
            .components
            .iter()
            .find(|c| c.component_type == ComponentType::Tags)
            .unwrap();
        assert_eq!(tags.data, serde_json::json!(["rust", "web", "html"]));

        let desc = result
            .components
            .iter()
            .find(|c| c.component_type == ComponentType::Description)
            .unwrap();
        assert_eq!(desc.data, serde_json::json!("Un article de test"));

        let lang = result
            .components
            .iter()
            .find(|c| c.component_type == ComponentType::Language)
            .unwrap();
        assert_eq!(lang.data, serde_json::json!("fr"));
    }

    #[test]
    fn test_import_html_fallback_to_h1() {
        let html = r#"<html>
<head></head>
<body><h1>My Heading</h1><p>Content.</p></body>
</html>"#;

        let importer = HtmlImporter::new();
        let result = importer
            .import_content(html, Path::new("no-title.html"))
            .unwrap();

        let title = result
            .components
            .iter()
            .find(|c| c.component_type == ComponentType::Title)
            .unwrap();
        assert_eq!(title.data, serde_json::json!("My Heading"));
    }

    #[test]
    fn test_import_html_fallback_to_filename() {
        let html = "<html><body><p>No title anywhere.</p></body></html>";

        let importer = HtmlImporter::new();
        let result = importer
            .import_content(html, Path::new("my-page.html"))
            .unwrap();

        let title = result
            .components
            .iter()
            .find(|c| c.component_type == ComponentType::Title)
            .unwrap();
        assert_eq!(title.data, serde_json::json!("my-page"));
    }

    #[test]
    fn test_extract_links() {
        let html = r##"<html><body>
<a href="https://example.com">Example</a>
<a href="other.html">Other Page</a>
<a href="docs/guide.html#intro">Guide</a>
<a href="#section">Skip</a>
</body></html>"##;

        let importer = HtmlImporter::new();
        let result = importer
            .import_content(html, Path::new("test.html"))
            .unwrap();

        assert_eq!(result.cross_references.len(), 3);

        let url_ref = result.cross_references[0].as_url_ref().unwrap();
        assert_eq!(url_ref.0, "https://example.com");
        assert_eq!(url_ref.1, "Example");

        let file_ref = result.cross_references[1].as_file_ref().unwrap();
        assert!(file_ref.0.to_string_lossy().contains("other.html"));

        let section_ref = result.cross_references[2].as_section_ref().unwrap();
        assert!(section_ref.0.to_string_lossy().contains("guide.html"));
        assert_eq!(section_ref.1, "intro");
    }

    #[test]
    fn test_can_import() {
        let importer = HtmlImporter::new();
        assert!(importer.can_import(Path::new("page.html")));
        assert!(importer.can_import(Path::new("page.htm")));
        assert!(importer.can_import(Path::new("page.HTML")));
        assert!(!importer.can_import(Path::new("page.md")));
        assert!(!importer.can_import(Path::new("page.pdf")));
    }

    #[test]
    fn test_supported_extensions() {
        let importer = HtmlImporter::new();
        assert_eq!(importer.supported_extensions(), &["html", "htm"]);
    }

    #[test]
    fn test_extract_html_title_fn() {
        assert_eq!(
            extract_html_title("<html><head><title>Hello</title></head></html>"),
            Some("Hello".to_string())
        );
        assert_eq!(extract_html_title("<html><head></head></html>"), None);
    }

    #[test]
    fn test_html_to_text_fn() {
        let text = html_to_text("<h1>Title</h1><p>Some text.</p>");
        assert!(text.contains("Title"));
        assert!(text.contains("Some text."));
    }

    #[test]
    fn test_html_to_markdown_fn() {
        let md = html_to_markdown("<p>Hello <strong>world</strong>.</p>");
        assert!(md.contains("Hello"));
        assert!(md.contains("world"));
    }

    #[test]
    fn test_og_description() {
        let html = r#"<html><head>
<meta property="og:description" content="OG desc here">
</head><body></body></html>"#;

        let metadata = extract_html_metadata(html);
        assert_eq!(metadata.description, Some("OG desc here".to_string()));
    }
}
