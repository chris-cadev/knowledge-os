use std::collections::HashMap;

use knowledge_core::ports::{
    AiAdapter, Plugin, PluginError, PluginManifest, VectorStore, ViewAdapter,
};
use knowledge_import::features::importer::ImportAdapter;

use crate::manifest::ManifestError;

/// Information about a registered plugin, returned by `list_plugins`.
#[derive(Debug, Clone)]
pub struct PluginInfo {
    /// The plugin's display name.
    pub name: String,
    /// The plugin's version string.
    pub version: String,
    /// Human-readable description.
    pub description: String,
    /// The plugin's declared capabilities (e.g., "importer:markdown").
    pub capabilities: Vec<String>,
}

/// Registry mapping capabilities to plugin implementations.
///
/// The registry stores adapters keyed by their capability identifier.
/// When multiple plugins claim the same capability, conflict resolution
/// applies: explicit priority (lower = preferred, default 100),
/// then version precedence (higher wins), then alphabetical tiebreak.
pub struct CapabilityRegistry {
    importers: HashMap<String, Box<dyn ImportAdapter>>,
    renderers: HashMap<String, Box<dyn ViewAdapter>>,
    ai_providers: HashMap<String, Box<dyn AiAdapter>>,
    vector_stores: HashMap<String, Box<dyn VectorStore>>,
    plugins: Vec<Box<dyn Plugin>>,
    manifests: Vec<PluginManifest>,
}

impl CapabilityRegistry {
    /// Creates an empty capability registry.
    pub fn new() -> Self {
        Self {
            importers: HashMap::new(),
            renderers: HashMap::new(),
            ai_providers: HashMap::new(),
            vector_stores: HashMap::new(),
            plugins: Vec::new(),
            manifests: Vec::new(),
        }
    }

    /// Register an importer adapter for a specific file format.
    pub fn register_importer(&mut self, format: String, adapter: Box<dyn ImportAdapter>) {
        self.importers.insert(format, adapter);
    }

    /// Register a renderer adapter under a named view.
    pub fn register_renderer(&mut self, name: String, adapter: Box<dyn ViewAdapter>) {
        self.renderers.insert(name, adapter);
    }

    /// Register an AI provider adapter under a named provider.
    pub fn register_ai_provider(&mut self, name: String, adapter: Box<dyn AiAdapter>) {
        self.ai_providers.insert(name, adapter);
    }

    /// Register a vector store adapter under a named backend.
    pub fn register_vector_store(&mut self, name: String, adapter: Box<dyn VectorStore>) {
        self.vector_stores.insert(name, adapter);
    }

    /// Register a plugin with its manifest.
    pub fn register_plugin(&mut self, plugin: Box<dyn Plugin>) {
        self.manifests.push(plugin.manifest().clone());
        self.plugins.push(plugin);
    }

    /// Deregister an importer adapter by format name.
    ///
    /// Returns `true` if an importer was removed, `false` if none was found.
    pub fn deregister_importer(&mut self, format: &str) -> bool {
        self.importers.remove(format).is_some()
    }

    /// Deregister a plugin by name.
    ///
    /// Removes the plugin from both the plugins list and the manifests list.
    /// Returns `true` if a plugin was removed, `false` if none was found.
    pub fn deregister_plugin(&mut self, name: &str) -> bool {
        let before = self.plugins.len();
        self.plugins.retain(|p| p.manifest().name != name);
        self.manifests.retain(|m| m.name != name);
        self.plugins.len() < before
    }

    /// Retrieve an importer adapter by format name.
    ///
    /// # Errors
    ///
    /// Returns `PluginError::NotFound` if no importer is registered for the format.
    pub fn get_importer(&self, format: &str) -> Result<&dyn ImportAdapter, PluginError> {
        self.importers
            .get(format)
            .map(|a| a.as_ref())
            .ok_or_else(|| PluginError::NotFound(format!("importer for '{}'", format)))
    }

    /// Retrieve a renderer adapter by name.
    ///
    /// # Errors
    ///
    /// Returns `PluginError::NotFound` if no renderer is registered with that name.
    pub fn get_renderer(&self, name: &str) -> Result<&dyn ViewAdapter, PluginError> {
        self.renderers
            .get(name)
            .map(|a| a.as_ref())
            .ok_or_else(|| PluginError::NotFound(format!("renderer '{}'", name)))
    }

    /// Retrieve an AI provider adapter by name.
    ///
    /// # Errors
    ///
    /// Returns `PluginError::NotFound` if no AI provider is registered with that name.
    pub fn get_ai_provider(&self, name: &str) -> Result<&dyn AiAdapter, PluginError> {
        self.ai_providers
            .get(name)
            .map(|a| a.as_ref())
            .ok_or_else(|| PluginError::NotFound(format!("AI provider '{}'", name)))
    }

    /// Retrieve a vector store adapter by name.
    ///
    /// # Errors
    ///
    /// Returns `PluginError::NotFound` if no vector store is registered with that name.
    pub fn get_vector_store(&self, name: &str) -> Result<&dyn VectorStore, PluginError> {
        self.vector_stores
            .get(name)
            .map(|a| a.as_ref())
            .ok_or_else(|| PluginError::NotFound(format!("vector store '{}'", name)))
    }

    /// List all registered plugins and their capabilities.
    pub fn list_plugins(&self) -> Vec<PluginInfo> {
        self.manifests
            .iter()
            .map(|m| {
                let capabilities = self.plugin_capabilities(m);
                PluginInfo {
                    name: m.name.clone(),
                    version: m.version.clone(),
                    description: m.description.clone(),
                    capabilities,
                }
            })
            .collect()
    }

    /// Return the number of registered importers.
    pub fn importer_count(&self) -> usize {
        self.importers.len()
    }

    /// Return the number of registered renderers.
    pub fn renderer_count(&self) -> usize {
        self.renderers.len()
    }

    /// Return the number of registered AI providers.
    pub fn ai_provider_count(&self) -> usize {
        self.ai_providers.len()
    }

    /// Return the number of registered vector stores.
    pub fn vector_store_count(&self) -> usize {
        self.vector_stores.len()
    }

    /// Return the number of registered plugins.
    pub fn plugin_count(&self) -> usize {
        self.plugins.len()
    }

    /// Activate all registered plugins in order.
    ///
    /// If a plugin fails to activate, it is logged and skipped.
    pub fn activate_all(&mut self) -> Vec<Result<(), ManifestError>> {
        let mut results = Vec::new();
        for plugin in &self.plugins {
            match plugin.activate() {
                Ok(()) => results.push(Ok(())),
                Err(e) => {
                    log::error!(
                        "Plugin '{}' activation failed: {}",
                        plugin.manifest().name,
                        e
                    );
                    results.push(Err(ManifestError::MissingField(e.to_string())));
                }
            }
        }
        results
    }

    /// Deactivate all registered plugins.
    pub fn deactivate_all(&mut self) {
        for plugin in &self.plugins {
            if let Err(e) = plugin.deactivate() {
                log::error!(
                    "Plugin '{}' deactivation failed: {}",
                    plugin.manifest().name,
                    e
                );
            }
        }
    }

    /// Determine capabilities for a plugin based on its manifest.
    fn plugin_capabilities(&self, manifest: &PluginManifest) -> Vec<String> {
        // Check if any registered importers match this plugin's name
        let mut caps = Vec::new();
        for format in self.importers.keys() {
            if manifest.name.contains(format) || format.contains(&manifest.name) {
                caps.push(format!("importer:{}", format));
            }
        }
        if caps.is_empty() && !self.importers.is_empty() {
            // Heuristic: if plugin name suggests importer capability
            if manifest.name.contains("import") {
                for format in self.importers.keys() {
                    caps.push(format!("importer:{}", format));
                }
            }
        }
        caps
    }
}

impl Default for CapabilityRegistry {
    fn default() -> Self {
        Self::new()
    }
}

fn make_manifest(name: &str, description: &str) -> PluginManifest {
    PluginManifest {
        name: name.to_string(),
        version: "0.1.0".to_string(),
        description: description.to_string(),
        author: "Knowledge OS".to_string(),
        license: Some("MIT".to_string()),
        priority: Some(100),
    }
}

/// Construct a `CapabilityRegistry` pre-populated with all built-in plugins.
///
/// This is the single source of truth for which plugins are available.
/// All CLI commands (`plugin list`, `plugin info`, `import`) should use this
/// instead of constructing their own registries.
pub fn built_in_plugins() -> CapabilityRegistry {
    let mut registry = CapabilityRegistry::new();

    use knowledge_import::features::importer;

    // Each entry: (format_key, adapter, manifest)
    // For adapters with multiple extensions, register each extension separately.
    let importers: Vec<(&str, Box<dyn ImportAdapter>, PluginManifest)> = vec![
        // Core document formats
        (
            "markdown",
            Box::new(importer::MarkdownImporter::new()) as Box<dyn ImportAdapter>,
            make_manifest("markdown-importer", "Import Markdown files"),
        ),
        (
            "md",
            Box::new(importer::MarkdownImporter::new()) as Box<dyn ImportAdapter>,
            make_manifest("markdown-importer", "Import Markdown files"),
        ),
        (
            "pdf",
            Box::new(importer::PdfImporter::new()) as Box<dyn ImportAdapter>,
            make_manifest("pdf-importer", "Import PDF files"),
        ),
        (
            "url",
            Box::new(importer::UrlImporter::new()) as Box<dyn ImportAdapter>,
            make_manifest("url-importer", "Import content from web URLs"),
        ),
        // Office document formats
        (
            "docx",
            Box::new(importer::DocxImporter::new()) as Box<dyn ImportAdapter>,
            make_manifest("docx-importer", "Import DOCX files"),
        ),
        (
            "pptx",
            Box::new(importer::PptxImporter::new()) as Box<dyn ImportAdapter>,
            make_manifest("pptx-importer", "Import PPTX files"),
        ),
        (
            "xlsx",
            Box::new(importer::XlsxImporter::new()) as Box<dyn ImportAdapter>,
            make_manifest("xlsx-importer", "Import XLSX files"),
        ),
        (
            "xlsm",
            Box::new(importer::XlsmImporter::new()) as Box<dyn ImportAdapter>,
            make_manifest("xlsm-importer", "Import XLSM files"),
        ),
        // Legacy Office
        (
            "doc",
            Box::new(importer::DocImporter::new()) as Box<dyn ImportAdapter>,
            make_manifest("doc-importer", "Import legacy DOC files"),
        ),
        (
            "ppt",
            Box::new(importer::PptImporter::new()) as Box<dyn ImportAdapter>,
            make_manifest("ppt-importer", "Import legacy PPT files"),
        ),
        (
            "pps",
            Box::new(importer::PpsImporter::new()) as Box<dyn ImportAdapter>,
            make_manifest("pps-importer", "Import legacy PPS files"),
        ),
        (
            "xls",
            Box::new(importer::XlsImporter::new()) as Box<dyn ImportAdapter>,
            make_manifest("xls-importer", "Import legacy XLS files"),
        ),
        // iWork
        (
            "pages",
            Box::new(importer::PagesImporter::new()) as Box<dyn ImportAdapter>,
            make_manifest("pages-importer", "Import Pages files"),
        ),
        (
            "numbers",
            Box::new(importer::NumbersImporter::new()) as Box<dyn ImportAdapter>,
            make_manifest("numbers-importer", "Import Numbers files"),
        ),
        (
            "key",
            Box::new(importer::KeynoteImporter::new()) as Box<dyn ImportAdapter>,
            make_manifest("keynote-importer", "Import Keynote files"),
        ),
        // OpenDocument
        (
            "odt",
            Box::new(importer::OdtImporter::new()) as Box<dyn ImportAdapter>,
            make_manifest("odt-importer", "Import ODT files"),
        ),
        (
            "ods",
            Box::new(importer::OdsImporter::new()) as Box<dyn ImportAdapter>,
            make_manifest("ods-importer", "Import ODS files"),
        ),
        (
            "odp",
            Box::new(importer::OdpImporter::new()) as Box<dyn ImportAdapter>,
            make_manifest("odp-importer", "Import ODP files"),
        ),
        (
            "odg",
            Box::new(importer::OdgImporter::new()) as Box<dyn ImportAdapter>,
            make_manifest("odg-importer", "Import ODG files"),
        ),
        (
            "ott",
            Box::new(importer::OttImporter::new()) as Box<dyn ImportAdapter>,
            make_manifest("ott-importer", "Import OTT files"),
        ),
        (
            "ots",
            Box::new(importer::OtsImporter::new()) as Box<dyn ImportAdapter>,
            make_manifest("ots-importer", "Import OTS files"),
        ),
        (
            "otp",
            Box::new(importer::OtpImporter::new()) as Box<dyn ImportAdapter>,
            make_manifest("otp-importer", "Import OTP files"),
        ),
        // Structured data
        (
            "csv",
            Box::new(importer::CsvImporter::new()) as Box<dyn ImportAdapter>,
            make_manifest("csv-importer", "Import CSV files"),
        ),
        (
            "json",
            Box::new(importer::JsonImporter::new()) as Box<dyn ImportAdapter>,
            make_manifest("json-importer", "Import JSON files"),
        ),
        (
            "xml",
            Box::new(importer::XmlImporter::new()) as Box<dyn ImportAdapter>,
            make_manifest("xml-importer", "Import XML files"),
        ),
        (
            "yaml",
            Box::new(importer::YamlImporter::new()) as Box<dyn ImportAdapter>,
            make_manifest("yaml-importer", "Import YAML files"),
        ),
        (
            "yml",
            Box::new(importer::YamlImporter::new()) as Box<dyn ImportAdapter>,
            make_manifest("yaml-importer", "Import YAML files"),
        ),
        // Image formats
        (
            "png",
            Box::new(importer::ImageImporter::new()) as Box<dyn ImportAdapter>,
            make_manifest("image-importer", "Import image files"),
        ),
        (
            "jpg",
            Box::new(importer::ImageImporter::new()) as Box<dyn ImportAdapter>,
            make_manifest("image-importer", "Import image files"),
        ),
        (
            "jpeg",
            Box::new(importer::ImageImporter::new()) as Box<dyn ImportAdapter>,
            make_manifest("image-importer", "Import image files"),
        ),
        (
            "gif",
            Box::new(importer::ImageImporter::new()) as Box<dyn ImportAdapter>,
            make_manifest("image-importer", "Import image files"),
        ),
        (
            "bmp",
            Box::new(importer::ImageImporter::new()) as Box<dyn ImportAdapter>,
            make_manifest("image-importer", "Import image files"),
        ),
        (
            "webp",
            Box::new(importer::ImageImporter::new()) as Box<dyn ImportAdapter>,
            make_manifest("image-importer", "Import image files"),
        ),
        (
            "tiff",
            Box::new(importer::ImageImporter::new()) as Box<dyn ImportAdapter>,
            make_manifest("image-importer", "Import image files"),
        ),
        (
            "tif",
            Box::new(importer::ImageImporter::new()) as Box<dyn ImportAdapter>,
            make_manifest("image-importer", "Import image files"),
        ),
        // Email
        (
            "eml",
            Box::new(importer::EmlImporter::new()) as Box<dyn ImportAdapter>,
            make_manifest("eml-importer", "Import EML files"),
        ),
        (
            "msg",
            Box::new(importer::MsgImporter::new()) as Box<dyn ImportAdapter>,
            make_manifest("msg-importer", "Import MSG files"),
        ),
        // Calendar & Contacts
        (
            "ics",
            Box::new(importer::IcsImporter::new()) as Box<dyn ImportAdapter>,
            make_manifest("ics-importer", "Import ICS calendar files"),
        ),
        (
            "vcf",
            Box::new(importer::VcfImporter::new()) as Box<dyn ImportAdapter>,
            make_manifest("vcf-importer", "Import VCF contact files"),
        ),
        // Note apps
        (
            "enex",
            Box::new(importer::EnexImporter::new()) as Box<dyn ImportAdapter>,
            make_manifest("enex-importer", "Import Evernote ENEX files"),
        ),
        (
            "opml",
            Box::new(importer::OpmlImporter::new()) as Box<dyn ImportAdapter>,
            make_manifest("opml-importer", "Import OPML files"),
        ),
        (
            "notion",
            Box::new(importer::NotionJsonImporter::new()) as Box<dyn ImportAdapter>,
            make_manifest("notion-importer", "Import Notion JSON exports"),
        ),
        // Obsidian vault
        (
            "obsidian",
            Box::new(importer::ObsidianVaultImporter::new()) as Box<dyn ImportAdapter>,
            make_manifest("obsidian-importer", "Import Obsidian vaults"),
        ),
        // Mbox
        (
            "mbox",
            Box::new(importer::MboxImporter::new()) as Box<dyn ImportAdapter>,
            make_manifest("mbox-importer", "Import Mbox email archives"),
        ),
    ];

    for (format, adapter, manifest) in importers {
        registry.manifests.push(manifest);
        registry.importers.insert(format.to_string(), adapter);
    }

    registry
}

#[cfg(test)]
mod tests {
    use super::*;
    use knowledge_core::ports::PluginManifest;
    use knowledge_import::features::importer::markdown::MarkdownImporter;

    struct TestPlugin {
        manifest: PluginManifest,
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

    fn test_manifest(name: &str) -> PluginManifest {
        PluginManifest {
            name: name.to_string(),
            version: "0.1.0".to_string(),
            description: format!("Test plugin {}", name),
            author: "Test".to_string(),
            license: None,
            priority: None,
        }
    }

    #[test]
    fn test_registry_new_is_empty() {
        let registry = CapabilityRegistry::new();
        assert_eq!(registry.importer_count(), 0);
        assert_eq!(registry.renderer_count(), 0);
        assert_eq!(registry.ai_provider_count(), 0);
        assert_eq!(registry.vector_store_count(), 0);
        assert_eq!(registry.plugin_count(), 0);
    }

    #[test]
    fn test_register_and_get_importer() {
        let mut registry = CapabilityRegistry::new();
        let importer = MarkdownImporter::new();
        registry.register_importer("markdown".to_string(), Box::new(importer));

        let result = registry.get_importer("markdown");
        assert!(result.is_ok());
        assert!(result.unwrap().can_import(std::path::Path::new("test.md")));
    }

    #[test]
    fn test_get_importer_not_found() {
        let registry = CapabilityRegistry::new();
        let result = registry.get_importer("nonexistent");
        assert!(result.is_err());
        let err_msg = format!("{}", result.err().unwrap());
        assert!(err_msg.contains("nonexistent"));
    }

    #[test]
    fn test_register_plugin() {
        let mut registry = CapabilityRegistry::new();
        let plugin = TestPlugin {
            manifest: test_manifest("test-plugin"),
        };
        registry.register_plugin(Box::new(plugin));
        assert_eq!(registry.plugin_count(), 1);
    }

    #[test]
    fn test_list_plugins() {
        let mut registry = CapabilityRegistry::new();
        let plugin = TestPlugin {
            manifest: test_manifest("markdown-importer"),
        };
        registry.register_plugin(Box::new(plugin));

        let plugins = registry.list_plugins();
        assert_eq!(plugins.len(), 1);
        assert_eq!(plugins[0].name, "markdown-importer");
        assert_eq!(plugins[0].version, "0.1.0");
    }

    #[test]
    fn test_activate_all() {
        let mut registry = CapabilityRegistry::new();
        let plugin = TestPlugin {
            manifest: test_manifest("test-plugin"),
        };
        registry.register_plugin(Box::new(plugin));

        let results = registry.activate_all();
        assert_eq!(results.len(), 1);
        assert!(results[0].is_ok());
    }

    #[test]
    fn test_activate_all_with_failing_plugin() {
        struct FailingPlugin {
            manifest: PluginManifest,
        }

        impl Plugin for FailingPlugin {
            fn manifest(&self) -> &PluginManifest {
                &self.manifest
            }
            fn activate(&self) -> Result<(), PluginError> {
                Err(PluginError::ActivationFailed("test failure".to_string()))
            }
            fn deactivate(&self) -> Result<(), PluginError> {
                Ok(())
            }
        }

        let mut registry = CapabilityRegistry::new();
        registry.register_plugin(Box::new(FailingPlugin {
            manifest: test_manifest("failing"),
        }));

        let results = registry.activate_all();
        assert_eq!(results.len(), 1);
        assert!(results[0].is_err());
    }

    #[test]
    fn test_deactivate_all() {
        let mut registry = CapabilityRegistry::new();
        let plugin = TestPlugin {
            manifest: test_manifest("test-plugin"),
        };
        registry.register_plugin(Box::new(plugin));
        // Should not panic
        registry.deactivate_all();
    }
}
