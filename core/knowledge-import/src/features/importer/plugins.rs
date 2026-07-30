use async_trait::async_trait;
use knowledge_core::ports::{Plugin, PluginError, PluginManifest, PluginMetadata};
use std::path::Path;

use crate::features::importer::adapter::{ImportAdapter, ImportError, ImportResult};

// =============================================================================
// Generic Plugin Adapter
// =============================================================================

/// Generic wrapper that bridges any `ImportAdapter + PluginMetadata` into the `Plugin` trait.
///
/// Eliminates the need to write per-importer wrapper structs. Instead of creating
/// `MarkdownImporterPlugin { manifest, inner: MarkdownImporter }`, you implement
/// `ImportAdapter` and `PluginMetadata` on your importer directly, then wrap it:
///
/// ```ignore
/// let plugin = PluginAdapter::new(MyImporter);
/// registry.register_plugin(Box::new(plugin));
/// ```
pub struct PluginAdapter<T> {
    manifest: PluginManifest,
    inner: T,
}

impl<T: PluginMetadata> PluginAdapter<T> {
    /// Create a new plugin adapter wrapping the given importer.
    pub fn new(inner: T) -> Self {
        Self {
            manifest: inner.manifest(),
            inner,
        }
    }
}

impl<T: PluginMetadata + Send + Sync> Plugin for PluginAdapter<T> {
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

#[async_trait]
impl<T: ImportAdapter + Send + Sync> ImportAdapter for PluginAdapter<T> {
    fn can_import(&self, path: &Path) -> bool {
        self.inner.can_import(path)
    }

    async fn import(&self, path: &Path) -> Result<ImportResult, ImportError> {
        self.inner.import(path).await
    }

    fn supported_extensions(&self) -> &[&str] {
        self.inner.supported_extensions()
    }
}

// =============================================================================
// Built-in Importer Plugins
// =============================================================================

/// Create a Markdown importer plugin.
pub fn markdown_plugin() -> PluginAdapter<crate::features::importer::markdown::MarkdownImporter> {
    PluginAdapter::new(crate::features::importer::markdown::MarkdownImporter::new())
}

/// Create a PDF importer plugin.
pub fn pdf_plugin() -> PluginAdapter<crate::features::importer::pdf::PdfImporter> {
    PluginAdapter::new(crate::features::importer::pdf::PdfImporter::new())
}

/// Create a URL importer plugin.
pub fn url_plugin() -> PluginAdapter<crate::features::importer::url::UrlImporter> {
    PluginAdapter::new(crate::features::importer::url::UrlImporter::new())
}

/// Create an HTML importer plugin.
pub fn html_plugin() -> PluginAdapter<crate::features::importer::html::HtmlImporter> {
    PluginAdapter::new(crate::features::importer::html::HtmlImporter::new())
}

/// Create an image importer plugin without OCR.
pub fn image_plugin() -> PluginAdapter<crate::features::importer::image::ImageImporter> {
    PluginAdapter::new(crate::features::importer::image::ImageImporter::new())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_plugin_adapter_manifest() {
        let plugin = markdown_plugin();
        let manifest = plugin.manifest();
        assert_eq!(manifest.name, "markdown-importer");
        assert_eq!(manifest.version, "0.1.0");
        assert_eq!(manifest.author, "Knowledge OS");
    }

    #[test]
    fn test_plugin_adapter_can_import() {
        let plugin = markdown_plugin();
        assert!(plugin.can_import(std::path::Path::new("test.md")));
        assert!(!plugin.can_import(std::path::Path::new("test.pdf")));
    }

    #[test]
    fn test_plugin_adapter_activate_deactivate() {
        let plugin = markdown_plugin();
        assert!(plugin.activate().is_ok());
        assert!(plugin.deactivate().is_ok());
    }

    #[test]
    fn test_pdf_plugin_adapter() {
        let plugin = pdf_plugin();
        let manifest = plugin.manifest();
        assert_eq!(manifest.name, "pdf-importer");
        assert!(plugin.can_import(std::path::Path::new("test.pdf")));
        assert!(!plugin.can_import(std::path::Path::new("test.md")));
    }

    #[test]
    fn test_url_plugin_adapter() {
        let plugin = url_plugin();
        let manifest = plugin.manifest();
        assert_eq!(manifest.name, "url-importer");
        assert!(plugin.can_import(std::path::Path::new("https://example.com")));
        assert!(plugin.can_import(std::path::Path::new("http://example.com")));
        assert!(!plugin.can_import(std::path::Path::new("test.md")));
    }

    #[test]
    fn test_all_plugins_register_with_capability_registry() {
        use knowledge_plugin::registry::CapabilityRegistry;

        let mut registry = CapabilityRegistry::new();
        let md = markdown_plugin();
        let pdf = pdf_plugin();
        let url = url_plugin();

        registry.register_plugin(Box::new(md));
        registry.register_plugin(Box::new(pdf));
        registry.register_plugin(Box::new(url));

        assert_eq!(registry.plugin_count(), 3);
        let plugins = registry.list_plugins();
        let names: Vec<&str> = plugins.iter().map(|p| p.name.as_str()).collect();
        assert!(names.contains(&"markdown-importer"));
        assert!(names.contains(&"pdf-importer"));
        assert!(names.contains(&"url-importer"));
    }
}
