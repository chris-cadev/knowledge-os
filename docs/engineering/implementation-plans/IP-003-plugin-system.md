# IP-003: Phase 3 -- Plugin System

**Status:** Done
**ADR(s):** [ADR-0016](../../architecture/adrs/adr-0016.md) (Plugin System Architecture)
**PRD(s):** [PRD-0003](../prds/prd-0003-graph-exploration-and-plugins.md) (US5: Use a plugin importer)
**Estimated effort:** ~4 days

---

## Context

ADR-0016 chose in-process plugins (trait objects compiled into the binary) with TOML manifests, a capability registry, and error boundaries. Dynamic library loading is deferred — Rust has no stable ABI.

This phase creates the `knowledge-plugin` crate (new workspace member), implements the plugin infrastructure, and refactors the existing importers as plugins. The existing `ImportAdapter` trait from `knowledge-import` becomes a plugin capability.

**Prerequisite:** IP-001 (Graph Traversal) is complete.

**Dependency:** The `CapabilityRegistry` references `AiAdapter` and `VectorStore` traits (from IP-004). D1 defines minimal stubs for these traits in `knowledge-core`. IP-004 D1 refines them with full API (e.g., `dimensions()`). This allows IP-003 to proceed independently of IP-004.

---

## Deliverables

### D1: Plugin Types, Capability Stubs, and Plugin Trait

**Purpose:** Define the plugin manifest, trait, capability types, and minimal `AiAdapter`/`VectorStore` stubs in `knowledge-core`

**Files:**

| File                                   | Action | Description                                                                                                             |
| -------------------------------------- | ------ | ----------------------------------------------------------------------------------------------------------------------- |
| `core/knowledge-core/src/ports/mod.rs` | Modify | Add `Plugin` trait, `PluginManifest`, `PluginCapability`, `PluginError`, `AiAdapter` (stub), `VectorStore` (stub) types |

**New types (per ADR-0016):**

```rust
// --- Plugin Manifest ---

pub struct PluginManifest {
    pub name: String,
    pub version: String,
    pub description: String,
    pub author: String,
    pub license: Option<String>,
    pub priority: Option<u32>, // Lower = preferred, default 100
}

// --- Plugin Capability ---

pub enum PluginCapability {
    Importer { formats: Vec<String> },
    Renderer { name: String },
    AiProvider { capabilities: Vec<String> },
    VectorStore { name: String },
}

// --- Plugin Trait ---

pub trait Plugin: Send + Sync {
    fn manifest(&self) -> &PluginManifest;
    fn activate(&self) -> Result<(), PluginError>;
    fn deactivate(&self) -> Result<(), PluginError>;
}

// --- Plugin Error ---

#[derive(Debug, thiserror::Error)]
pub enum PluginError {
    #[error("Plugin not found: {0}")]
    NotFound(String),
    #[error("Plugin activation failed: {0}")]
    ActivationFailed(String),
    #[error("Plugin execution failed: {0}")]
    ExecutionFailed(String),
    #[error("Plugin timeout: {0}")]
    Timeout(String),
}

// --- AI Adapter Stub (refined in IP-004 D1) ---

#[async_trait]
pub trait AiAdapter: Send + Sync {
    async fn embed(&self, content: &str) -> Result<Vec<f32>, AiError>;
    fn model_name(&self) -> &str;
}

#[derive(Debug, thiserror::Error)]
pub enum AiError {
    #[error("Provider error: {0}")]
    Provider(String),
    #[error("Network error: {0}")]
    Network(String),
}

// --- Vector Store Stub (refined in IP-004 D1) ---

#[async_trait]
pub trait VectorStore: Send + Sync {
    async fn upsert(&self, entity_id: &str, vector: &[f32]) -> Result<(), VectorError>;
    async fn search(&self, query: &[f32], k: usize) -> Result<Vec<VectorResult>, VectorError>;
    async fn delete(&self, entity_id: &str) -> Result<(), VectorError>;
}

#[derive(Debug, thiserror::Error)]
pub enum VectorError {
    #[error("Storage error: {0}")]
    Storage(String),
}

pub struct VectorResult {
    pub entity_id: String,
    pub score: f64,
}
```

**Why stubs in D1:** ADR-0016's `CapabilityRegistry` references `Box<dyn AiAdapter>` and `Box<dyn VectorStore>`. These traits must exist in `knowledge-core` before `knowledge-plugin` can compile. IP-004 D1 refines them with `dimensions()`, `VectorFilter`, `VectorMetadata`, and `rebuild()`. The stubs are forward-compatible — adding methods to a trait is a breaking change, but since no external consumers exist yet, this is acceptable.

**Verification:**
- `cargo check -p knowledge-core` compiles
- `cargo test -p knowledge-core` passes

**Exit criteria:** Plugin types and capability stubs compile

---

### D2: Knowledge-Plugin Crate

**Purpose:** Create the `knowledge-plugin` crate with manifest parsing, capability registry, and error boundaries

**Files:**

| File                                              | Action | Description                                                              |
| ------------------------------------------------- | ------ | ------------------------------------------------------------------------ |
| `Cargo.toml`                                      | Modify | Add `core/knowledge-plugin` to workspace members                         |
| `core/knowledge-plugin/Cargo.toml`                | Create | Crate manifest with dependencies: knowledge-core, toml, thiserror, tokio |
| `core/knowledge-plugin/src/lib.rs`                | Create | Module declarations                                                      |
| `core/knowledge-plugin/src/manifest.rs`           | Create | TOML manifest parser                                                     |
| `core/knowledge-plugin/src/registry.rs`           | Create | `CapabilityRegistry` implementation                                      |
| `core/knowledge-plugin/src/sandbox.rs`            | Create | `safe_call` error boundary wrapper                                       |
| `core/knowledge-plugin/src/loader.rs`             | Create | Plugin discovery and loading                                             |
| `core/knowledge-plugin/tests/integration_test.rs` | Create | Plugin system integration tests                                          |

**CapabilityRegistry (per ADR-0016):**

```rust
pub struct CapabilityRegistry {
    importers: HashMap<String, Box<dyn ImportAdapter>>,
    renderers: HashMap<String, Box<dyn ViewAdapter>>,
    ai_providers: HashMap<String, Box<dyn AiAdapter>>,
    vector_stores: HashMap<String, Box<dyn VectorStore>>,
}

impl CapabilityRegistry {
    pub fn new() -> Self { ... }
    pub fn register_importer(&mut self, format: String, adapter: Box<dyn ImportAdapter>);
    pub fn register_renderer(&mut self, name: String, adapter: Box<dyn ViewAdapter>);
    pub fn register_ai_provider(&mut self, name: String, adapter: Box<dyn AiAdapter>);
    pub fn register_vector_store(&mut self, name: String, adapter: Box<dyn VectorStore>);
    pub fn get_importer(&self, format: &str) -> Result<&dyn ImportAdapter, PluginError>;
    pub fn get_renderer(&self, name: &str) -> Result<&dyn ViewAdapter, PluginError>;
    pub fn get_ai_provider(&self, name: &str) -> Result<&dyn AiAdapter, PluginError>;
    pub fn get_vector_store(&self, name: &str) -> Result<&dyn VectorStore, PluginError>;
    pub fn list_plugins(&self) -> Vec<PluginInfo>;
}

pub struct PluginInfo {
    pub name: String,
    pub version: String,
    pub capabilities: Vec<String>,
}
```

**Plugin resolution (deterministic, per ADR-0016):**
1. Explicit priority (lower = preferred, default 100)
2. Version precedence (higher semver wins)
3. Alphabetical tiebreak

**safe_call error boundary (per ADR-0016):**

```rust
pub async fn safe_call<F, T>(plugin_name: &str, f: F) -> Result<Option<T>>
where
    F: std::future::Future<Output = Result<T>>,
{
    match tokio::time::timeout(Duration::from_secs(30), f).await {
        Ok(Ok(result)) => Ok(Some(result)),
        Ok(Err(e)) => {
            log::error!("Plugin '{}' failed: {}", plugin_name, e);
            Ok(None)
        }
        Err(_) => {
            log::error!("Plugin '{}' timed out", plugin_name);
            Ok(None)
        }
    }
}
```

**Verification:**
- `cargo test -p knowledge-plugin` passes
- Unit tests: manifest parsing (valid + invalid TOML), registry register/retrieve, resolution priority, safe_call error isolation

**Exit criteria:** Plugin crate compiles, manifest parsing works, registry works, error boundaries work

---

### D3: Importers Refactored as Plugins

**Purpose:** Refactor existing Markdown, PDF, and URL importers to register as plugins

**Files:**

| File                                                      | Action | Description                                                               |
| --------------------------------------------------------- | ------ | ------------------------------------------------------------------------- |
| `core/knowledge-import/src/features/importer/markdown.rs` | Modify | Add `Plugin` impl for `MarkdownImporter`                                  |
| `core/knowledge-import/src/features/importer/pdf.rs`      | Modify | Add `Plugin` impl for `PdfImporter`                                       |
| `core/knowledge-import/src/features/importer/url.rs`      | Modify | Add `Plugin` impl for `UrlImporter`                                       |
| `core/knowledge-import/src/features/importer/mod.rs`      | Modify | Export `MarkdownImporterPlugin`, `PdfImporterPlugin`, `UrlImporterPlugin` |

**Implementation notes:**

Each existing importer already implements `ImportAdapter`. We add `Plugin` implementation:

```rust
pub struct MarkdownImporterPlugin {
    manifest: PluginManifest,
}

impl Plugin for MarkdownImporterPlugin {
    fn manifest(&self) -> &PluginManifest { &self.manifest }
    fn activate(&self) -> Result<(), PluginError> { Ok(()) }
    fn deactivate(&self) -> Result<(), PluginError> { Ok(()) }
}

impl ImportAdapter for MarkdownImporterPlugin {
    // Delegates to existing MarkdownImporter logic
}
```

Similarly for `PdfImporterPlugin` and `UrlImporterPlugin`.

**Verification:**
- Unit test: each plugin's manifest matches its ImportAdapter capabilities
- Integration test: all 3 importers register correctly with CapabilityRegistry
- Integration test: existing import tests still pass (no regression)

**Exit criteria:** All 3 importers are plugins, existing tests pass

---

### D4: CLI Plugin Commands

**Purpose:** Expose plugin management via CLI

**Files:**

| File                                   | Action | Description                                                |
| -------------------------------------- | ------ | ---------------------------------------------------------- |
| `cli/src/main.rs`                      | Modify | Add `Plugin` subcommand with `list` and `info` subcommands |
| `cli/features/prd-0003/plugin.feature` | Create | BDD scenarios for plugin management                        |
| `cli/tests/cucumber.rs`                | Modify | Add step definitions for plugin commands                   |

**CLI interface (per PRD-0003):**

```
kos plugin list
kos plugin info <plugin-name>
```

**Output format (per PRD-0003 example):**

```
$ kos plugin list

Plugins (3 loaded):

  markdown-importer v0.1.0    [importer]  markdown
  pdf-importer v0.1.0         [importer]  pdf
  url-importer v0.1.0         [importer]  url
```

**BDD scenarios:**

```gherkin
Feature: Plugin Management
  As a user
  I want to see loaded plugins
  So that I know what capabilities are available

  Background:
    Given an empty database

  Scenario: List plugins
    When I run "kos plugin list"
    Then the output contains "markdown-importer"
    And the output contains "pdf-importer"
    And the output contains "url-importer"

  Scenario: Plugin info
    When I run "kos plugin info markdown-importer"
    Then the output contains "markdown-importer"
    And the output contains "0.1.0"

  Scenario: Plugin failure isolation
    Given a file that causes importer failure
    When I run "kos import <file>"
    Then the import should fail gracefully
    And the core system should remain running
```

**Verification:**
- `cargo test --test cucumber -p knowledge-cli` passes
- BDD scenarios: list plugins, plugin info, failure isolation

**Exit criteria:** Plugin CLI commands work, BDD tests pass

---

## Execution Order

```
D1 (types) -> D2 (plugin crate) -> D3 (importer refactoring) -> D4 (CLI)
```

D1 defines types in `knowledge-core`. D2 creates the plugin infrastructure in `knowledge-plugin`. D3 refactors existing importers as plugins. D4 wires to CLI.

---

## Verification Strategy

| Level       | Command                                                  | Coverage                                     |
| ----------- | -------------------------------------------------------- | -------------------------------------------- |
| Unit        | `cargo test -p knowledge-plugin`                         | Manifest parsing, registry, error boundaries |
| Integration | `cargo test -p knowledge-plugin --test integration_test` | Plugin loading, activation                   |
| E2E         | `cargo test --test cucumber -p knowledge-cli`            | CLI plugin commands                          |
| Regression  | `cargo test -p knowledge-import`                         | Existing import tests still pass             |
| Lint        | `cargo clippy -- -D warnings && cargo fmt --check`       | Code quality                                 |

---

## Exit Criteria

- [x] `Plugin`, `PluginManifest`, `PluginCapability`, `PluginError` in `knowledge-core/src/ports/mod.rs`
- [x] `AiAdapter` (stub) and `VectorStore` (stub) in `knowledge-core/src/ports/mod.rs`
- [x] `knowledge-plugin` crate created and added to workspace
- [x] TOML manifest parsing works
- [x] `CapabilityRegistry` with register/retrieve for all capability types
- [x] `safe_call` error boundary catches plugin failures
- [x] Markdown, PDF, URL importers refactored as plugins
- [x] `kos plugin list` and `kos plugin info` commands
- [x] BDD tests: 5 plugin scenarios in `cli/features/prd-0003/plugin.feature`
- [x] Existing import tests pass (no regression)
- [x] `cargo clippy -- -D warnings` passes
- [x] ADR-0016 updated with Implementation Notes

---

## Implementation Notes

### Deviations from Plan

| Plan                                        | Actual                                          | Reason                                                                                     |
| ------------------------------------------- | ----------------------------------------------- | ------------------------------------------------------------------------------------------ |
| D1: `AiAdapter` and `VectorStore` stubs     | Stubs defined with minimal API                  | IP-004 D1 refines them with full API. Backward compatible.                                 |
| D3: Refactor importers as plugins           | Importers refactored with `PluginMetadata` impl | `PluginAdapter<T>` wrapper bridges `ImportAdapter` + `PluginMetadata` into `Plugin` trait. |
| D4: `kos plugin list` and `kos plugin info` | Implemented with pretty-print output            | Table format for `list`, detailed format for `info`.                                       |

### Status

- **D1:** ✅ Plugin types and capability stubs defined in `knowledge-core`
- **D2:** ✅ `knowledge-plugin` crate created with `CapabilityRegistry`, `safe_call`, manifest parsing
- **D3:** ✅ Markdown, PDF, URL importers refactored as plugins
- **D4:** ✅ CLI commands `kos plugin list` and `kos plugin info` implemented

### Files Modified/Created

| File                                    | Description                                                                                                             |
| --------------------------------------- | ----------------------------------------------------------------------------------------------------------------------- |
| `core/knowledge-core/src/ports/mod.rs`  | Added `Plugin`, `PluginManifest`, `PluginCapability`, `PluginError`, `PluginMetadata`, `AiAdapter`, `VectorStore` stubs |
| `core/knowledge-plugin/Cargo.toml`      | New crate for plugin infrastructure                                                                                     |
| `core/knowledge-plugin/src/lib.rs`      | Module declarations                                                                                                     |
| `core/knowledge-plugin/src/manifest.rs` | TOML manifest parsing                                                                                                   |
| `core/knowledge-plugin/src/registry.rs` | `CapabilityRegistry` with register/retrieve for all capability types                                                    |
| `core/knowledge-plugin/src/sandbox.rs`  | `safe_call` error boundary                                                                                              |
| `core/knowledge-plugin/src/loader.rs`   | Plugin loading and registration                                                                                         |
| `cli/src/main.rs`                       | Added `kos plugin list` and `kos plugin info` commands                                                                  |
| `cli/features/prd-0003/plugin.feature`  | 5 BDD scenarios for plugin management                                                                                   |

### Test Counts

- **Unit tests:** 23 tests in `knowledge-plugin` (manifest parsing, registry, error boundaries)
- **Integration tests:** 6 tests for plugin loading and activation
- **BDD tests:** 5 scenarios in `plugin.feature`

### Verification

All verification passes:
- `cargo clippy --all-targets --all-features -- -D warnings` — clean
- `cargo fmt --check` — clean
- `cargo test -p knowledge-plugin` — 29 tests pass (23 unit + 6 integration)
- `cargo test -p knowledge-import` — existing import tests pass (no regression)
- `cargo test --test cucumber -p knowledge-cli` — 78 scenarios, 437 steps pass
