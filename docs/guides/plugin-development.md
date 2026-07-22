# Plugin Development Guide

> Plugins extend capabilities without modifying the core. Every subsystem supports extension.

---

## Overview

Knowledge OS is designed as a core system surrounded by plugins. Plugins implement adapters for storage engines, importers, exporters, renderers, AI providers, and automation agents.

This guide explains how to develop, test, and distribute plugins.

---

## Plugin Types

| Type | Interface | Purpose |
|------|-----------|---------|
| `importer` | `ImportAdapter` | Import from external formats |
| `exporter` | `ExportAdapter` | Export to external formats |
| `renderer` | `RenderAdapter` | Render canonical data to output |
| `storage` | `StorageAdapter` | Persist data to a storage engine |
| `search` | `SearchAdapter` | Index and retrieve text |
| `vector` | `VectorAdapter` | Store and query embeddings |
| `graph` | `GraphAdapter` | Store and traverse relationships |
| `cache` | `CacheAdapter` | Cache derived data |
| `ai` | `AiAdapter` | Provide AI operations |
| `automation` | `AutomationAdapter` | Automate knowledge operations |
| `view` | `ViewAdapter` | Render a view projection |

---

## Plugin Manifest

Every plugin has a manifest file (`knowledge-plugin.toml`):

```toml
[plugin]
name = "my-custom-importer"
version = "0.1.0"
description = "Import custom format files as knowledge entities"
author = "Your Name"
license = "MIT"

[plugin.capabilities]
importers = ["custom-format"]

[plugin.dependencies]
knowledge-os = ">=0.1.0"

[plugin.permissions]
files = ["read"]
network = []
```

### Manifest Fields

- `name`: Unique plugin identifier (snake_case)
- `version`: Semantic version
- `description`: Human-readable description
- `author`: Plugin author
- `license`: License identifier
- `capabilities`: What the plugin provides
- `dependencies`: Required Knowledge OS version
- `permissions`: System permissions required

---

## Implementing an Importer

### Step 1: Implement the ImportAdapter Trait

```rust
use knowledge_os::plugin::{ImportAdapter, ImportSource, ImportedEntity};

pub struct MyImporter;

impl ImportAdapter for MyImporter {
    fn can_import(&self, source: &ImportSource) -> bool {
        source.path.ends_with(".myformat")
    }

    fn import(&self, source: &ImportSource) -> Result<Vec<ImportedEntity>> {
        let content = std::fs::read_to_string(&source.path)?;
        let entities = parse_myformat(&content)?;
        Ok(entities)
    }

    fn supported_types(&self) -> &[&str] {
        &["custom-format"]
    }
}
```

### Step 2: Register the Plugin

```rust
use knowledge_os::plugin::register;

fn main() {
    register(MyImporter);
}
```

### Step 3: Build and Test

```bash
cargo build --release
cargo test
```

---

## Implementing a Storage Adapter

### Step 1: Implement the StorageAdapter Trait

```rust
use knowledge_os::plugin::{StorageAdapter, Entity, EntityId, QueryFilter, HealthStatus};

pub struct MyStorage {
    connection: MyDatabaseConnection,
}

impl StorageAdapter for MyStorage {
    async fn persist(&self, entity: &Entity) -> Result<()> {
        self.connection.upsert(entity).await
    }

    async fn retrieve(&self, id: &EntityId) -> Result<Option<Entity>> {
        self.connection.find_by_id(id).await
    }

    async fn query(&self, filter: &QueryFilter) -> Result<Vec<Entity>> {
        self.connection.query(filter).await
    }

    async fn health(&self) -> HealthStatus {
        match self.connection.ping().await {
            Ok(_) => HealthStatus::Healthy,
            Err(_) => HealthStatus::Unhealthy,
        }
    }
}
```

### Step 2: Implement Configuration

```rust
use serde::Deserialize;

#[derive(Deserialize)]
pub struct MyStorageConfig {
    pub url: String,
    pub database: String,
}
```

---

## Implementing an AI Adapter

### Step 1: Implement the AiAdapter Trait

```rust
use knowledge_os::plugin::{AiAdapter, Classification, Relationship, Entity, AiResponse};

pub struct MyAiProvider {
    client: MyApiClient,
}

impl AiAdapter for MyAiProvider {
    async fn classify(&self, content: &str, taxonomy: &str) -> Result<Vec<Classification>> {
        self.client.classify(content, taxonomy).await
    }

    async fn extract_relationships(&self, content: &str, entities: &[Entity]) -> Result<Vec<Relationship>> {
        self.client.extract(content, entities).await
    }

    async fn summarize(&self, content: &str, max_length: usize) -> Result<String> {
        self.client.summarize(content, max_length).await
    }

    async fn embed(&self, content: &str) -> Result<Vec<f64>> {
        self.client.embed(content).await
    }

    async fn answer(&self, question: &str, context: &[Entity]) -> Result<AiResponse> {
        self.client.answer(question, context).await
    }
}
```

---

## Testing Plugins

### Unit Tests

```rust
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_can_import() {
        let importer = MyImporter;
        let source = ImportSource { path: "test.myformat".into() };
        assert!(importer.can_import(&source));
    }

    #[test]
    fn test_import() {
        let importer = MyImporter;
        let source = ImportSource { path: "fixtures/test.myformat".into() };
        let entities = importer.import(&source).unwrap();
        assert!(!entities.is_empty());
    }
}
```

### Integration Tests

```rust
#[tokio::test]
async fn test_storage_adapter() {
    let storage = MyStorage::new(test_config());
    let entity = create_test_entity();
    storage.persist(&entity).await.unwrap();
    let retrieved = storage.retrieve(&entity.id).await.unwrap();
    assert_eq!(retrieved.unwrap().id, entity.id);
}
```

---

## Distribution

### Local Installation

```bash
knowledge-os plugin install ./target/release/libmy_importer.so
```

### Marketplace Installation

```bash
knowledge-os plugin install my-custom-importer
```

### Publishing

1. Build the plugin in release mode.
2. Sign the plugin binary.
3. Publish to the plugin marketplace.

---

## Further Reading

- [Extensibility](../architecture/extensibility.md) -- Plugin system architecture
- [AI](../architecture/ai.md) -- AI adapter integration
- [Storage](../architecture/storage.md) -- Storage adapter patterns
