# Extensibility

> Every subsystem supports extension. Plugins extend capabilities without modifying the core.

---

## Plugin System

Knowledge OS is designed as a core system surrounded by plugins. The core provides the canonical model, the pipeline, and the event system. Everything else is a plugin.

### Plugin Principles

1. **Plugins are first-class citizens.** Plugins are not afterthoughts. The plugin API is designed before the core implementation.
2. **Plugins are isolated.** A plugin cannot modify the core system. A plugin extends the system by implementing defined interfaces.
3. **Plugins are replaceable.** Any plugin may be replaced by another plugin that implements the same interface.
4. **Plugins are declarative.** Plugin metadata (name, version, capabilities, dependencies) is declared in a manifest, not discovered at runtime.

### Plugin Lifecycle

```
Discovery  -->  Registration  -->  Activation  -->  Execution  -->  Deactivation
     |              |                  |                |                |
  Find plugin    Register with     Initialize       Run plugin       Clean up
  on disk or     the plugin        plugin state     code on          plugin state
  registry       registry                            trigger
```

### Plugin Manifest

Every plugin declares a manifest:

```toml
[plugin]
name = "markdown-importer"
version = "0.1.0"
description = "Import Markdown files as knowledge entities"

[plugin.capabilities]
importers = ["markdown"]
renderers = []

[plugin.dependencies]
knowledge-os = ">=0.1.0"
```

---

## Capabilities

Capabilities declare what a plugin provides. The system queries capabilities to determine which plugins handle which operations.

### Capability Types

| Capability | Purpose | Interface |
|-----------|---------|-----------|
| `importer` | Import from external formats | `ImportAdapter` |
| `exporter` | Export to external formats | `ExportAdapter` |
| `renderer` | Render canonical data to output | `RenderAdapter` |
| `storage` | Persist data to a storage engine | `StorageAdapter` |
| `search` | Index and retrieve text | `SearchAdapter` |
| `vector` | Store and query embeddings | `VectorAdapter` |
| `graph` | Store and traverse relationships | `GraphAdapter` |
| `cache` | Cache derived data | `CacheAdapter` |
| `ai` | Provide AI operations | `AiAdapter` |
| `automation` | Automate knowledge operations | `AutomationAdapter` |
| `view` | Render a view projection | `ViewAdapter` |

### Capability Negotiation

When the system needs a capability, it queries registered plugins:

```
System needs: import("markdown")
Candidates:
  - markdown-importer v0.1.0 (priority: 10)
  - universal-importer v0.2.0 (priority: 5)
Selected: markdown-importer v0.1.0 (highest priority)
```

Priority is configurable. The system selects the highest-priority plugin that supports the requested capability.

---

## APIs

The plugin API is the contract between the core system and plugins.

### API Principles

1. **Stable.** The API changes infrequently. Breaking changes follow the deprecation policy.
2. **Typed.** All API surfaces use strongly typed interfaces. No stringly-typed contracts.
3. **Async.** All API operations are asynchronous. Plugins may perform I/O without blocking the pipeline.
4. **Idempotent.** All API operations are idempotent. Plugins may be retried without side effects.

### API Layers

```
Plugin API (stable, versioned)
     |
Core Services API (internal, may change)
     |
Storage API (adapter interface)
     |
Event API (pub/sub interface)
```

Plugins interact with the system through the Plugin API. The Core Services API is internal to the system. The Storage API is implemented by storage adapters. The Event API is used for pub/sub communication.

### API Versioning

The Plugin API follows semantic versioning:

- **Major version.** Breaking changes. Old plugins must be updated.
- **Minor version.** New capabilities. Old plugins continue to work.
- **Patch version.** Bug fixes. No plugin changes required.

---

## SDKs

The SDK provides utilities for plugin development.

### SDK Components

**Types crate.** All types used in the plugin API: entity types, component types, relationship types, events, errors.

**Traits crate.** All traits that plugins implement: `ImportAdapter`, `ExportAdapter`, `RenderAdapter`, `StorageAdapter`, etc.

**Testing utilities.** Mock implementations of core services for plugin testing.

**Build utilities.** Plugin manifest validation, capability registration, dependency resolution.

### SDK Versioning

The SDK version matches the Plugin API version. Plugins built against SDK v0.1.0 are compatible with Knowledge OS v0.1.x.

---

## Importers

Importers receive information from external systems and transform it into internal representations.

### Importer Contract

```rust
trait ImportAdapter {
    /// Detect if this importer can handle the given source.
    fn can_import(&self, source: &ImportSource) -> bool;

    /// Import content from the source.
    fn import(&self, source: &ImportSource) -> Result<Vec<ImportedEntity>>;

    /// Return the supported source types.
    fn supported_types(&self) -> &[SourceType];
}
```

### Built-in Importers

| Importer | Source | Output |
|----------|--------|--------|
| Markdown | `.md` files | Entity with Content component |
| PDF | `.pdf` files | Entity with Content + BinaryContent components |
| HTML | URLs, `.html` files | Entity with Content component |
| Git | Git repositories | Project entity with relationship to commits |

### Custom Importers

Custom importers are added as plugins. The plugin:

1. Implements the `ImportAdapter` trait.
2. Declares the `importer` capability in its manifest.
3. Registers with the plugin system at startup.

---

## Exporters

Exporters transform canonical data into external formats.

### Exporter Contract

```rust
trait ExportAdapter {
    /// Export entities to the target format.
    fn export(&self, entities: &[Entity], format: &ExportFormat) -> Result<Vec<u8>>;

    /// Return the supported export formats.
    fn supported_formats(&self) -> &[ExportFormat];
}
```

### Built-in Exporters

| Exporter | Format | Output |
|----------|--------|--------|
| Markdown | `.md` | Markdown files |
| JSON | `.json` | Structured entity data |
| GraphML | `.graphml` | Graph structure |

---

## Renderers

Renderers transform canonical data into output formats for consumption by external systems.

### Renderer Contract

```rust
trait RenderAdapter {
    /// Render a projection of canonical data.
    fn render(&self, projection: &Projection, format: &RenderFormat) -> Result<Vec<u8>>;

    /// Return the supported render formats.
    fn supported_formats(&self) -> &[RenderFormat];
}
```

### Built-in Renderers

| Renderer | Format | Purpose |
|----------|--------|---------|
| HTML | `.html` | Web pages |
| JSON API | `application/json` | API responses |
| MCP | MCP protocol | AI agent communication |

---

## Storage Adapters

Storage adapters persist data to specific storage engines.

### Storage Adapter Contract

```rust
trait StorageAdapter {
    /// Persist canonical data.
    fn persist(&self, entity: &Entity) -> Result<()>;

    /// Retrieve canonical data.
    fn retrieve(&self, id: &EntityId) -> Result<Option<Entity>>;

    /// Query canonical data.
    fn query(&self, filter: &QueryFilter) -> Result<Vec<Entity>>;

    /// Health check.
    fn health(&self) -> HealthStatus;
}
```

### Built-in Storage Adapters

| Adapter | Engine | Purpose |
|---------|--------|---------|
| SQLite | SQLite | Local relational storage |
| PostgreSQL | PostgreSQL | Production relational storage |
| Tantivy | Tantivy | Full-text search |
| Qdrant | Qdrant | Vector storage |
| Redis | Redis | Cache storage |
| S3 | S3/MinIO | Object storage |

### Storage Adapter Selection

Storage adapters are selected through configuration:

```toml
[storage.relational]
driver = "sqlite"
path = "./data/knowledge.db"

[storage.search]
driver = "tantivy"
path = "./data/search"

[storage.vector]
driver = "qdrant"
url = "http://localhost:6334"

[storage.cache]
driver = "redis"
url = "redis://localhost:6379"

[storage.object]
driver = "s3"
bucket = "knowledge-os"
endpoint = "http://localhost:9000"
```

---

## AI Extensions

AI providers are plugins that supply AI capabilities to the pipeline.

### AI Adapter Contract

```rust
trait AiAdapter {
    /// Classify content.
    fn classify(&self, content: &str, taxonomy: &str) -> Result<Vec<Classification>>;

    /// Extract relationships from content.
    fn extract_relationships(&self, content: &str, entities: &[Entity]) -> Result<Vec<Relationship>>;

    /// Generate a summary.
    fn summarize(&self, content: &str, max_length: usize) -> Result<String>;

    /// Generate an embedding.
    fn embed(&self, content: &str) -> Result<Vec<f64>>;

    /// Answer a question given context.
    fn answer(&self, question: &str, context: &[Entity]) -> Result<AiResponse>;
}
```

### AI Provider Selection

AI providers are configured per operation type:

```toml
[ai.classification]
provider = "openai"
model = "gpt-4"

[ai.embedding]
provider = "openai"
model = "text-embedding-3-small"

[ai.extraction]
provider = "anthropic"
model = "claude-3-opus"
```

Multiple providers may be configured. The system selects the provider based on the operation type and configuration.

---

## Further Reading

- [Pipeline](pipeline.md) -- How plugins fit in the seven-layer architecture
- [Events](events.md) -- How plugins receive and emit events
- [Composition](composition.md) -- How plugins extend the entity component model
- [AI](ai.md) -- How AI plugins integrate with the knowledge model
- [Storage](storage.md) -- How storage adapters are selected and configured
