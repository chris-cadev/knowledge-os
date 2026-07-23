use knowledge_core::features::component::ComponentType;
use knowledge_core::features::entity::EntityType;
use knowledge_import::features::importer::Importer;
use std::path::Path;

#[test]
fn test_import_with_full_frontmatter() {
    let content = r#"---
title: "My Research Paper"
tags:
  - machine-learning
  - transformers
author: "Jane Doe"
language: "en"
---

# My Research Paper

This paper explores transformer architectures."#;

    let importer = Importer::new();
    let result = importer.import_content(content, Path::new("paper.md")).unwrap();

    assert_eq!(result.entity.entity_type, EntityType::Article);

    let title = result.components.iter()
        .find(|c| c.component_type == ComponentType::Title)
        .unwrap();
    assert_eq!(title.data, serde_json::json!("My Research Paper"));

    let tags = result.components.iter()
        .find(|c| c.component_type == ComponentType::Tags)
        .unwrap();
    assert_eq!(tags.data, serde_json::json!(["machine-learning", "transformers"]));

    let author = result.components.iter()
        .find(|c| c.component_type == ComponentType::Author)
        .unwrap();
    assert_eq!(author.data, serde_json::json!("Jane Doe"));

    let lang = result.components.iter()
        .find(|c| c.component_type == ComponentType::Language)
        .unwrap();
    assert_eq!(lang.data, serde_json::json!("en"));

    let provenance = result.components.iter()
        .find(|c| c.component_type == ComponentType::Provenance)
        .unwrap();
    assert_eq!(provenance.data.get("source").unwrap().as_str().unwrap(), "paper.md");

    assert!(result.components.iter().any(|c| c.component_type == ComponentType::Timeline));
    assert!(result.components.iter().any(|c| c.component_type == ComponentType::Content));
}

#[test]
fn test_import_without_frontmatter_uses_h1() {
    let content = r#"# My Document Title

Some body content here."#;

    let importer = Importer::new();
    let result = importer.import_content(content, Path::new("test.md")).unwrap();

    let title = result.components.iter()
        .find(|c| c.component_type == ComponentType::Title)
        .unwrap();
    assert_eq!(title.data, serde_json::json!("My Document Title"));

    // Should default language to en
    let lang = result.components.iter()
        .find(|c| c.component_type == ComponentType::Language)
        .unwrap();
    assert_eq!(lang.data, serde_json::json!("en"));

    // Should not have Tags or Author
    assert!(!result.components.iter().any(|c| c.component_type == ComponentType::Tags));
    assert!(!result.components.iter().any(|c| c.component_type == ComponentType::Author));
}

#[test]
fn test_import_without_frontmatter_or_heading_uses_filename() {
    let content = "Just some plain content.";

    let importer = Importer::new();
    let result = importer.import_content(content, Path::new("my-article.md")).unwrap();

    let title = result.components.iter()
        .find(|c| c.component_type == ComponentType::Title)
        .unwrap();
    assert_eq!(title.data, serde_json::json!("my-article"));
}

#[test]
fn test_cross_references_extracted_from_markdown() {
    let content = r#"# Document

See [other file](other.md) and [another](docs/another.md)."#;

    let importer = Importer::new();
    let result = importer.import_content(content, Path::new("test.md")).unwrap();

    assert_eq!(result.cross_references.len(), 2);
    assert!(result.cross_references[0].target_path.to_string_lossy().contains("other.md"));
    assert!(result.cross_references[1].target_path.to_string_lossy().contains("another.md"));
}

#[test]
fn test_cross_references_only_extract_md_links() {
    let content = r#"# Document

See [website](https://example.com) and [file](other.md)."#;

    let importer = Importer::new();
    let result = importer.import_content(content, Path::new("test.md")).unwrap();

    assert_eq!(result.cross_references.len(), 1);
    assert!(result.cross_references[0].target_path.to_string_lossy().contains("other.md"));
}

#[test]
fn test_content_preserved_verbatim() {
    let content = r#"---
title: "Test"
---

# Heading

Some **bold** and *italic* text.

- item 1
- item 2

```rust
fn main() {}
```"#;

    let importer = Importer::new();
    let result = importer.import_content(content, Path::new("test.md")).unwrap();

    let content_comp = result.components.iter()
        .find(|c| c.component_type == ComponentType::Content)
        .unwrap();
    let body = content_comp.data.as_str().unwrap();
    assert!(body.contains("**bold**"));
    assert!(body.contains("*italic*"));
    assert!(body.contains("```rust"));
    assert!(body.contains("- item 1"));
}

#[test]
fn test_import_result_has_all_expected_components() {
    let content = r#"---
title: "Full Test"
tags:
  - tag1
author: "Author"
language: "fr"
---

Body content."#;

    let importer = Importer::new();
    let result = importer.import_content(content, Path::new("test.md")).unwrap();

    let expected_types = vec![
        ComponentType::Title,
        ComponentType::Content,
        ComponentType::Tags,
        ComponentType::Author,
        ComponentType::Timeline,
        ComponentType::Language,
        ComponentType::Provenance,
    ];

    for expected in &expected_types {
        assert!(
            result.components.iter().any(|c| c.component_type == *expected),
            "Missing component type: {:?}", expected
        );
    }
}
