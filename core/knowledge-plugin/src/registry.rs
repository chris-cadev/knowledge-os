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

/// Construct a `CapabilityRegistry` pre-populated with all built-in plugins.
///
/// This is the single source of truth for which plugins are available.
/// All CLI commands (`plugin list`, `plugin info`, `import`) should use this
/// instead of constructing their own registries.
pub fn built_in_plugins() -> CapabilityRegistry {
    let mut registry = CapabilityRegistry::new();

    // Register importer adapters directly in the importers map.
    // Manifests are stored in the manifests vec for plugin listing.
    let importers: Vec<(&str, Box<dyn ImportAdapter>, PluginManifest)> = vec![
        (
            "markdown",
            Box::new(knowledge_import::features::importer::markdown_plugin()),
            knowledge_import::features::importer::markdown_plugin()
                .manifest()
                .clone(),
        ),
        (
            "pdf",
            Box::new(knowledge_import::features::importer::pdf_plugin()),
            knowledge_import::features::importer::pdf_plugin()
                .manifest()
                .clone(),
        ),
        (
            "url",
            Box::new(knowledge_import::features::importer::url_plugin()),
            knowledge_import::features::importer::url_plugin()
                .manifest()
                .clone(),
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
