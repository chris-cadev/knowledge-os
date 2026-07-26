# Plugin Development Guide

> Plugins extend Knowledge OS capabilities without modifying the core. Every subsystem supports extension.

---

## Overview

Knowledge OS is designed as a core system surrounded by plugins. Plugins implement adapters for importers, renderers, AI providers, vector stores, and other capabilities.

The plugin system provides:
- **`Plugin` trait** -- lifecycle management (activate/deactivate)
- **`PluginMetadata` trait** -- manifest metadata for any adapter
- **`PluginAdapter<T>`** -- generic wrapper bridging adapters into the plugin system
- **`CapabilityRegistry`** -- central registry mapping capabilities to implementations
- **Error boundaries** -- sandboxed execution with 30-second timeouts

---

## Quick Start: Creating an Importer Plugin

The minimum viable plugin requires two trait implementations on a single struct:

### Step 1: Implement the Adapter

```rust
use async_trait::async_trait;
use knowledge_core::ports::{PluginManifest, PluginMetadata};
use knowledge_import::features::importer::{ImportAdapter, ImportError, ImportResult};
use std::path::Path;

pub struct BibTexImporter;

#[async_trait]
impl ImportAdapter for BibTexImporter {
    fn can_import(&self, path: &Path) -> bool {
        path.extension()
            .and_then(|e| e.to_str())
            .map(|ext| ext.eq_ignore_ascii_case("bib"))
            .unwrap_or(false)
    }

    async fn import(&self, path: &Path) -> Result<ImportResult, ImportError> {
        let content = std::fs::read_to_string(path)?;
        let entity = parse_bibtex(&content);
        Ok(ImportResult {
            entity,
            components: vec![],
            cross_references: vec![],
        })
    }

    fn supported_extensions(&self) -> &[&str] {
        &["bib"]
    }
}
```

### Step 2: Implement PluginMetadata

```rust
impl PluginMetadata for BibTexImporter {
    fn manifest(&self) -> PluginManifest {
        PluginManifest {
            name: "bibtex-importer".to_string(),
            version: "0.1.0".to_string(),
            description: "Import BibTeX bibliography files".to_string(),
            author: "Your Name".to_string(),
            license: Some("MIT".to_string()),
            priority: Some(100),
        }
    }
}
```

### Step 3: Wrap and Register

```rust
use knowledge_import::features::importer::PluginAdapter;
use knowledge_plugin::registry::CapabilityRegistry;

let adapter = BibTexImporter;
let plugin = PluginAdapter::new(adapter);

let mut registry = CapabilityRegistry::new();
registry.register_plugin(Box::new(plugin));
```

That's it. No wrapper struct needed.

---

## Architecture

### Crate Structure

| Crate | Provides |
|-------|----------|
| `knowledge-core` | `Plugin`, `PluginMetadata`, `PluginManifest`, `PluginError` traits/types |
| `knowledge-import` | `ImportAdapter`, `PluginAdapter<T>`, built-in importer plugins |
| `knowledge-plugin` | `CapabilityRegistry`, `safe_call` error boundary, plugin discovery |

### Dependency Chain

```
knowledge-plugin → knowledge-import → knowledge-core
```

### Key Types

**`Plugin` trait** (`knowledge-core::ports`):
```rust
pub trait Plugin: Send + Sync {
    fn manifest(&self) -> &PluginManifest;
    fn activate(&self) -> Result<(), PluginError>;
    fn deactivate(&self) -> Result<(), PluginError>;
}
```

**`PluginMetadata` trait** (`knowledge-core::ports`):
```rust
pub trait PluginMetadata {
    fn manifest(&self) -> PluginManifest;
}
```

**`PluginAdapter<T>`** (`knowledge_import::features::importer::plugins`):
```rust
pub struct PluginAdapter<T> {
    manifest: PluginManifest,
    inner: T,
}

impl<T: PluginMetadata + Send + Sync> Plugin for PluginAdapter<T> { ... }
impl<T: ImportAdapter + Send + Sync> ImportAdapter for PluginAdapter<T> { ... }
```

**`CapabilityRegistry`** (`knowledge_plugin::registry`):
```rust
pub struct CapabilityRegistry {
    importers: HashMap<String, Box<dyn ImportAdapter>>,
    renderers: HashMap<String, Box<dyn ViewAdapter>>,
    ai_providers: HashMap<String, Box<dyn AiAdapter>>,
    vector_stores: HashMap<String, Box<dyn VectorStore>>,
    plugins: Vec<Box<dyn Plugin>>,
    manifests: Vec<PluginManifest>,
}
```

---

## Manifest Format

Every plugin provides a `PluginManifest` (defined in `knowledge-core::ports`):

```rust
pub struct PluginManifest {
    pub name: String,        // Unique identifier (e.g., "bibtex-importer")
    pub version: String,     // Semantic version (e.g., "0.1.0")
    pub description: String, // Human-readable purpose
    pub author: String,      // Author name
    pub license: Option<String>, // SPDX license identifier
    pub priority: Option<u32>,   // Conflict resolution (lower = preferred, default 100)
}
```

For external plugin discovery, manifests are stored in TOML files (`plugin.toml`):

```toml
[plugin]
name = "bibtex-importer"
version = "0.1.0"
description = "Import BibTeX bibliography files"
author = "Your Name"
license = "MIT"
priority = 100
```

---

## Using the Built-in Registry

The `built_in_plugins()` function returns a pre-populated registry with all built-in importers:

```rust
use knowledge_plugin::registry::built_in_plugins;

let registry = built_in_plugins();
let plugins = registry.list_plugins();

for plugin in &plugins {
    println!("{} v{}", plugin.name, plugin.version);
}

// Retrieve a specific importer
let importer = registry.get_importer("markdown")?;
let result = importer.import(path).await?;
```

---

## Error Handling

Plugin errors are caught by the `safe_call` error boundary:

```rust
use knowledge_plugin::sandbox::safe_call;

let result = safe_call("my-plugin", async {
    // Plugin operation that might fail
    my_adapter.import(path).await.map_err(|e| PluginError::ExecutionFailed(e.to_string()))
}).await;

match result {
    Ok(Some(result)) => { /* success */ }
    Ok(None) => { /* plugin error, logged and swallowed */ }
    Err(PluginError::Timeout(msg)) => { /* timeout */ }
    _ => {}
}
```

---

## Testing Plugins

### Unit Tests

```rust
#[cfg(test)]
mod tests {
    use super::*;
    use knowledge_import::features::importer::PluginAdapter;

    #[test]
    fn test_plugin_manifest() {
        let plugin = PluginAdapter::new(BibTexImporter);
        assert_eq!(plugin.manifest().name, "bibtex-importer");
    }

    #[test]
    fn test_can_import() {
        let plugin = PluginAdapter::new(BibTexImporter);
        assert!(plugin.can_import(Path::new("refs.bib")));
        assert!(!plugin.can_import(Path::new("notes.md")));
    }

    #[test]
    fn test_lifecycle() {
        let plugin = PluginAdapter::new(BibTexImporter);
        assert!(plugin.activate().is_ok());
        assert!(plugin.deactivate().is_ok());
    }
}
```

### Integration Tests

```rust
#[tokio::test]
async fn test_register_and_retrieve() {
    let mut registry = CapabilityRegistry::new();
    let plugin = PluginAdapter::new(BibTexImporter);
    let manifest = plugin.manifest().clone();

    registry.register_plugin(Box::new(plugin));
    registry.register_importer("bibtex".to_string(), Box::new(BibTexImporter));

    assert_eq!(registry.plugin_count(), 1);
    let importer = registry.get_importer("bibtex").unwrap();
    assert!(importer.can_import(Path::new("refs.bib")));
}
```

---

## Plugin Lifecycle

1. **Discovery** -- `discover_plugins()` scans directories for `plugin.toml` files
2. **Resolution** -- `resolve_plugins()` sorts by priority, version, name
3. **Registration** -- `register_plugin()` adds to the registry
4. **Activation** -- `activate_all()` calls each plugin's `activate()`
5. **Execution** -- Plugin operations wrapped in `safe_call()` error boundaries
6. **Deactivation** -- `deactivate_all()` calls each plugin's `deactivate()`

---

## Further Reading

- [Extensibility](../architecture/extensibility.md) -- Plugin system architecture
- [AI](../architecture/ai.md) -- AI adapter integration
- [Storage](../architecture/storage.md) -- Storage adapter patterns
