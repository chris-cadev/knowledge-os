use knowledge_core::ports::{Plugin, PluginCapability, PluginError, PluginManifest};
use knowledge_import::features::importer::markdown::MarkdownImporter;
use knowledge_plugin::manifest::parse_manifest;
use knowledge_plugin::registry::CapabilityRegistry;

// =============================================================================
// Helper types
// =============================================================================

struct TestPlugin {
    manifest: PluginManifest,
}

impl TestPlugin {
    fn new(name: &str) -> Self {
        Self {
            manifest: PluginManifest {
                name: name.to_string(),
                version: "0.1.0".to_string(),
                description: format!("Test plugin {}", name),
                author: "Test".to_string(),
                license: Some("MIT".to_string()),
                priority: None,
            },
        }
    }
}

impl Plugin for TestPlugin {
    fn manifest(&self) -> &PluginManifest {
        &self.manifest
    }
    fn activate(&self) -> Result<(), PluginError> {
        Ok(())
    }
    fn deactivate(&self) -> Result<(), PluginError> {
        Ok(())
    }
}

// =============================================================================
// Integration Tests
// =============================================================================

#[test]
fn test_manifest_parse_roundtrip() {
    let toml = r#"
[plugin]
name = "markdown-importer"
version = "0.1.0"
description = "Import Markdown files as knowledge entities"
author = "Knowledge OS"
license = "MIT"
priority = 100
"#;

    let manifest = parse_manifest(toml).unwrap();
    assert_eq!(manifest.name, "markdown-importer");
    assert_eq!(manifest.version, "0.1.0");
    assert_eq!(manifest.effective_priority(), 100);
}

#[test]
fn test_capability_registry_register_and_retrieve() {
    let mut registry = CapabilityRegistry::new();
    let importer = MarkdownImporter::new();
    registry.register_importer("markdown".to_string(), Box::new(importer));

    let retrieved = registry.get_importer("markdown").unwrap();
    assert!(retrieved.can_import(std::path::Path::new("test.md")));
    assert!(!retrieved.can_import(std::path::Path::new("test.pdf")));
}

#[test]
fn test_capability_registry_plugin_lifecycle() {
    let mut registry = CapabilityRegistry::new();

    let plugin1 = TestPlugin::new("markdown-importer");
    let plugin2 = TestPlugin::new("pdf-importer");

    registry.register_plugin(Box::new(plugin1));
    registry.register_plugin(Box::new(plugin2));
    assert_eq!(registry.plugin_count(), 2);

    let results = registry.activate_all();
    assert_eq!(results.len(), 2);
    assert!(results[0].is_ok());
    assert!(results[1].is_ok());

    registry.deactivate_all();
}

#[test]
fn test_capability_registry_list_plugins() {
    let mut registry = CapabilityRegistry::new();

    registry.register_plugin(Box::new(TestPlugin::new("markdown-importer")));
    registry.register_plugin(Box::new(TestPlugin::new("pdf-importer")));
    registry.register_plugin(Box::new(TestPlugin::new("url-importer")));

    let plugins = registry.list_plugins();
    assert_eq!(plugins.len(), 3);

    let names: Vec<&str> = plugins.iter().map(|p| p.name.as_str()).collect();
    assert!(names.contains(&"markdown-importer"));
    assert!(names.contains(&"pdf-importer"));
    assert!(names.contains(&"url-importer"));
}

#[test]
fn test_plugin_manifest_metadata() {
    let plugin = TestPlugin::new("test-plugin");
    let manifest = plugin.manifest();

    assert_eq!(manifest.name, "test-plugin");
    assert_eq!(manifest.version, "0.1.0");
    assert_eq!(manifest.description, "Test plugin test-plugin");
    assert_eq!(manifest.author, "Test");
    assert_eq!(manifest.license.as_deref(), Some("MIT"));
}

#[test]
fn test_plugin_capability_declaration() {
    let manifest = PluginManifest {
        name: "bibtex-importer".to_string(),
        version: "0.1.0".to_string(),
        description: "Import BibTeX files".to_string(),
        author: "Test".to_string(),
        license: None,
        priority: Some(50),
    };

    let capability = PluginCapability::Importer {
        formats: vec!["bibtex".to_string()],
    };

    match capability {
        PluginCapability::Importer { formats } => {
            assert_eq!(formats, vec!["bibtex"]);
        }
        _ => panic!("Expected Importer capability"),
    }

    assert_eq!(manifest.effective_priority(), 50);
}
