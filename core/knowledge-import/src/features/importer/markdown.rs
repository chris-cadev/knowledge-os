use async_trait::async_trait;
use knowledge_core::features::component::{Component, ComponentType};
use knowledge_core::features::entity::{Entity, EntityType};
use pulldown_cmark::{Event, Parser, Tag};
use std::collections::HashMap;
use std::path::Path;

use super::adapter::{CrossReference, ImportError, ImportAdapter, ImportResult};

/// Markdown file importer implementing the ImportAdapter trait.
pub struct MarkdownImporter;

impl Default for MarkdownImporter {
    fn default() -> Self {
        Self::new()
    }
}

impl MarkdownImporter {
    pub fn new() -> Self {
        Self
    }

    /// Import Markdown content from a string.
    pub fn import_content(
        &self,
        content: &str,
        source_path: &Path,
    ) -> Result<ImportResult, ImportError> {
        let (frontmatter, body) = parse_frontmatter(content);

        let title = frontmatter
            .get("title")
            .and_then(|v| v.as_str())
            .map(String::from)
            .unwrap_or_else(|| {
                extract_first_h1(body).unwrap_or_else(|| {
                    source_path
                        .file_stem()
                        .and_then(|s| s.to_str())
                        .unwrap_or("Untitled")
                        .to_string()
                })
            });

        let tags: Vec<String> = frontmatter
            .get("tags")
            .and_then(|v| v.as_array())
            .map(|seq| {
                seq.iter()
                    .filter_map(|v| v.as_str().map(String::from))
                    .collect()
            })
            .unwrap_or_default();

        // Entity type inference from frontmatter `type` field
        let entity_type = frontmatter
            .get("type")
            .and_then(|v| v.as_str())
            .map(EntityType::new)
            .unwrap_or_else(|| EntityType::new("Article"));

        let entity = Entity::new(entity_type);

        let mut components = vec![
            Component::new(
                entity.id,
                ComponentType::Title,
                serde_json::to_value(&title).unwrap(),
            ),
            Component::new(
                entity.id,
                ComponentType::Content,
                serde_json::to_value(body).unwrap(),
            ),
        ];

        if !tags.is_empty() {
            components.push(Component::new(
                entity.id,
                ComponentType::Tags,
                serde_json::to_value(&tags).unwrap(),
            ));
        }

        // Author component from frontmatter
        if let Some(author) = frontmatter.get("author").and_then(|v| v.as_str()) {
            components.push(Component::new(
                entity.id,
                ComponentType::Author,
                serde_json::json!(author),
            ));
        }

        // Timeline component
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

        // Language component (default to "en")
        let language = frontmatter
            .get("language")
            .and_then(|v| v.as_str())
            .unwrap_or("en");

        components.push(Component::new(
            entity.id,
            ComponentType::Language,
            serde_json::json!(language),
        ));

        // Provenance
        components.push(Component::new(
            entity.id,
            ComponentType::Provenance,
            serde_json::json!({
                "source": source_path.to_string_lossy(),
                "imported_at": chrono::Utc::now().to_rfc3339(),
            }),
        ));

        let cross_refs = extract_cross_references(body, source_path);

        Ok(ImportResult {
            entity,
            components,
            cross_references: cross_refs,
        })
    }
}

#[async_trait]
impl ImportAdapter for MarkdownImporter {
    fn can_import(&self, path: &Path) -> bool {
        path.extension()
            .and_then(|e| e.to_str())
            .map(|ext| ext.eq_ignore_ascii_case("md"))
            .unwrap_or(false)
    }

    async fn import(&self, path: &Path) -> Result<ImportResult, ImportError> {
        let content = std::fs::read_to_string(path)?;
        self.import_content(&content, path)
    }

    fn supported_extensions(&self) -> &[&str] {
        &["md"]
    }
}

fn parse_frontmatter(content: &str) -> (HashMap<String, serde_json::Value>, &str) {
    let trimmed = content.trim_start();
    if !trimmed.starts_with("---") {
        return (HashMap::new(), content);
    }

    let after_first_delimiter = &trimmed[3..];
    if let Some(end_idx) = after_first_delimiter.find("\n---") {
        let yaml_str = &after_first_delimiter[..end_idx];
        let body = &after_first_delimiter[end_idx + 4..];

        match serde_yaml::from_str::<serde_yaml::Value>(yaml_str) {
            Ok(serde_yaml::Value::Mapping(map)) => {
                let mut json_map = HashMap::new();
                for (k, v) in map {
                    if let Some(key) = k.as_str() {
                        if let Some(json_val) = yaml_to_json(&v) {
                            json_map.insert(key.to_string(), json_val);
                        }
                    }
                }
                (json_map, body)
            }
            _ => (HashMap::new(), content),
        }
    } else {
        (HashMap::new(), content)
    }
}

fn yaml_to_json(val: &serde_yaml::Value) -> Option<serde_json::Value> {
    match val {
        serde_yaml::Value::Null => Some(serde_json::Value::Null),
        serde_yaml::Value::Bool(b) => Some(serde_json::Value::Bool(*b)),
        serde_yaml::Value::Number(n) => {
            if let Some(i) = n.as_i64() {
                Some(serde_json::Value::Number(i.into()))
            } else if let Some(f) = n.as_f64() {
                serde_json::Number::from_f64(f).map(serde_json::Value::Number)
            } else {
                None
            }
        }
        serde_yaml::Value::String(s) => Some(serde_json::Value::String(s.clone())),
        serde_yaml::Value::Sequence(seq) => {
            let arr: Vec<serde_json::Value> = seq.iter().filter_map(yaml_to_json).collect();
            Some(serde_json::Value::Array(arr))
        }
        serde_yaml::Value::Mapping(map) => {
            let obj: serde_json::map::Map<String, serde_json::Value> = map
                .iter()
                .filter_map(|(k, v)| {
                    let key = k.as_str()?.to_string();
                    let val = yaml_to_json(v)?;
                    Some((key, val))
                })
                .collect();
            Some(serde_json::Value::Object(obj))
        }
        _ => None,
    }
}

fn extract_first_h1(content: &str) -> Option<String> {
    for line in content.lines() {
        let trimmed = line.trim();
        if let Some(rest) = trimmed.strip_prefix("# ") {
            let rest = rest.trim();
            if !rest.is_empty() {
                return Some(rest.to_string());
            }
        }
    }
    None
}

fn extract_cross_references(content: &str, source_path: &Path) -> Vec<CrossReference> {
    let mut refs = Vec::new();
    let parser = Parser::new(content);

    for event in parser {
        if let Event::Start(Tag::Link(_, dest_url, _)) = event {
            if dest_url.starts_with("http://") || dest_url.starts_with("https://") {
                refs.push(CrossReference::UrlRef {
                    url: dest_url.to_string(),
                    link_text: dest_url.to_string(),
                });
            } else if let Some((file_part, section)) = dest_url.split_once('#') {
                // Section anchor: path/to/file.md#section or just #section
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
                    link_text: dest_url.to_string(),
                });
            } else if dest_url.ends_with(".md") {
                let target_path = source_path
                    .parent()
                    .unwrap_or(Path::new("."))
                    .join(dest_url.as_ref());

                refs.push(CrossReference::FileRef {
                    target_path,
                    link_text: dest_url.to_string(),
                });
            }
        }
    }

    // Extract wikilinks: [[name]]
    for line in content.lines() {
        for cap in regex_captures(r"\[\[([^\]]+)\]\]", line) {
            let target_name = cap[1].to_string();
            refs.push(CrossReference::WikilinkRef {
                target_name,
                link_text: cap[1].to_string(),
            });
        }
    }

    // Extract @mentions: @name
    for line in content.lines() {
        for cap in regex_captures(r"@([a-zA-Z_][a-zA-Z0-9_]*)", line) {
            refs.push(CrossReference::MentionRef {
                target_name: cap[1].to_string(),
            });
        }
    }

    refs
}

/// Simple regex helper that finds all captures of a pattern in a line.
fn regex_captures<'a>(pattern: &'a str, text: &'a str) -> Vec<regex::Captures<'a>> {
    lazy_static::lazy_static! {
        static ref WIKILINK_RE: regex::Regex = regex::Regex::new(r"\[\[([^\]]+)\]\]").unwrap();
        static ref MENTION_RE: regex::Regex = regex::Regex::new(r"@([a-zA-Z_][a-zA-Z0-9_]*)").unwrap();
    }

    let re = if pattern.contains("\\[\\[") {
        &*WIKILINK_RE
    } else {
        &*MENTION_RE
    };

    re.captures_iter(text).collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_import_with_frontmatter() {
        let content = r#"---
title: "Test Document"
tags:
  - rust
  - testing
author: "Chris"
---

# Test Content

This is a test document."#;

        let importer = MarkdownImporter::new();
        let result = importer.import_content(content, Path::new("test.md")).unwrap();

        assert_eq!(result.entity.entity_type, EntityType::new("Article"));
        assert!(!result.components.is_empty());

        let title = result
            .components
            .iter()
            .find(|c| c.component_type == ComponentType::Title)
            .unwrap();
        assert_eq!(title.data, serde_json::json!("Test Document"));

        let tags = result
            .components
            .iter()
            .find(|c| c.component_type == ComponentType::Tags)
            .unwrap();
        assert_eq!(tags.data, serde_json::json!(["rust", "testing"]));

        let author = result
            .components
            .iter()
            .find(|c| c.component_type == ComponentType::Author)
            .unwrap();
        assert_eq!(author.data, serde_json::json!("Chris"));
    }

    #[test]
    fn test_import_without_frontmatter() {
        let content = r#"# My Document

Some content here."#;

        let importer = MarkdownImporter::new();
        let result = importer.import_content(content, Path::new("test.md")).unwrap();

        let title = result
            .components
            .iter()
            .find(|c| c.component_type == ComponentType::Title)
            .unwrap();
        assert_eq!(title.data, serde_json::json!("My Document"));
    }

    #[test]
    fn test_import_fallback_to_filename() {
        let content = "Just some content without a heading.";

        let importer = MarkdownImporter::new();
        let result = importer.import_content(content, Path::new("my-article.md")).unwrap();

        let title = result
            .components
            .iter()
            .find(|c| c.component_type == ComponentType::Title)
            .unwrap();
        assert_eq!(title.data, serde_json::json!("my-article"));
    }

    #[test]
    fn test_cross_reference_extraction() {
        let content = r#"# Document

See [other file](other.md) for more details."#;

        let importer = MarkdownImporter::new();
        let result = importer.import_content(content, Path::new("test.md")).unwrap();

        assert_eq!(result.cross_references.len(), 1);
        assert!(result.cross_references[0]
            .as_file_ref()
            .unwrap()
            .0
            .to_string_lossy()
            .contains("other.md"));
    }

    #[test]
    fn test_url_reference_extraction() {
        let content = r#"# Document

See [website](https://example.com) for more."#;

        let importer = MarkdownImporter::new();
        let result = importer.import_content(content, Path::new("test.md")).unwrap();

        assert_eq!(result.cross_references.len(), 1);
        assert!(result.cross_references[0]
            .as_url_ref()
            .is_some());
    }

    #[test]
    fn test_section_reference_extraction() {
        let content = r#"# Document

See [section](other.md#introduction) for more."#;

        let importer = MarkdownImporter::new();
        let result = importer.import_content(content, Path::new("test.md")).unwrap();

        assert_eq!(result.cross_references.len(), 1);
        let section_ref = result.cross_references[0].as_section_ref().unwrap();
        assert!(section_ref.0.to_string_lossy().contains("other.md"));
        assert_eq!(section_ref.1, "introduction");
    }

    #[test]
    fn test_frontmatter_type_inference() {
        let content = r#"---
title: "Research Paper"
type: paper
---

# Research Paper

Content here."#;

        let importer = MarkdownImporter::new();
        let result = importer.import_content(content, Path::new("paper.md")).unwrap();

        assert_eq!(result.entity.entity_type, EntityType::new("paper"));
    }

    #[test]
    fn test_adapter_can_import() {
        let adapter = MarkdownImporter::new();
        assert!(adapter.can_import(Path::new("test.md")));
        assert!(adapter.can_import(Path::new("test.MD")));
        assert!(!adapter.can_import(Path::new("test.pdf")));
        assert!(!adapter.can_import(Path::new("test.txt")));
    }
}
