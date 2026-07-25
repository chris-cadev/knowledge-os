# IP-003: Phase 3 -- Plugin System

**Status:** Draft
**ADR(s):** [ADR-0016](../../architecture/adrs/adr-0016.md) (Plugin System Architecture)
**PRD(s):** [PRD-0003](../prds/prd-0003-graph-exploration-and-plugins.md) (US5: Use a plugin importer)
**Estimated effort:** ~4 days

---

## Context

ADR-0016 chose in-process plugins (trait objects compiled into the binary) with TOML manifests, a capability registry, and error boundaries. Dynamic library loading is deferred -- Rust has no stable ABI.

This phase creates the `knowledge-plugin` crate (new workspace member), implements the plugin infrastructure, and refactors the existing Markdown importer as the first plugin. The existing `ImportAdapter` trait from `knowledge-import` becomes a plugin capability.

---

## Deliverables

### D1: Plugin Types and Plugin Trait

**Purpose:** Define the plugin manifest, trait, and capability types in `knowledge-core`

**Files:**

| File | Action | Description |
|------|--------|-------------|
| `core/knowledge-core/src/ports/mod.rs` | Modify | Add `Plugin` trait, `PluginManifest`, `PluginCapability`, `PluginError` types |

**New types (per ADR-0016):**

```rust
pub struct PluginManifest {
    pub name: String,
    pub version: String,
    pub description: String,
    pub author: String,
    pub license: Option<String>,
    pub capabilities: Vec<PluginCapability>,
    pub priority: Option<u32>,
}

pub enum PluginCapability {
    Importer { formats: Vec<String> },
    Renderer { name: String },
    AiProvider { capabilities: Vec<String> },
    VectorStore { name: String },
}

pub trait Plugin: Send + Sync {
    fn manifest(&self) -> &PluginManifest;
    fn activate(&self) -> Result<(), PluginError>;
    fn deactivate(&self) -> Result<(), PluginError>;
}

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
```

**Verification:**
- `cargo check -p knowledge-core` compiles
- `cargo test -p knowledge-core` passes

**Exit criteria:** Plugin types compile

---

### D2: Knowledge-Plugin Crate

**Purpose:** Create the `knowledge-plugin` crate with manifest parsing, capability registry, and error boundaries

**Files:**

| File | Action | Description |
|------|--------|-------------|
| `Cargo.toml` | Modify | Add `core/knowledge-plugin` to workspace members |
| `core/knowledge-plugin/Cargo.toml` | Create | Crate manifest with dependencies: knowledge-core, toml, thiserror, tokio |
| `core/knowledge-plugin/src/lib.rs` | Create | Module declarations |
| `core/knowledge-plugin/src/manifest.rs` | Create | TOML manifest parser |
| `core/knowledge-plugin/src/registry.rs` | Create | `CapabilityRegistry` implementation |
| `core/knowledge-plugin/src/sandbox.rs` | Create | `safe_call` error boundary wrapper |
| `core/knowledge-plugin/src/loader.rs` | Create | Plugin discovery and loading |
| `core/knowledge-plugin/tests/integration_test.rs` | Create | Plugin system integration tests |

**CapabilityRegistry (per ADR-0016):**

```rust
pub struct CapabilityRegistry {
    importers: HashMap<String, Box<dyn ImportAdapter>>,
    renderers: HashMap<String, Box<dyn ViewAdapter>>,
    ai_providers: HashMap<String, Box<dyn AiAdapter>>,
    vector_stores: HashMap<String, Box<dyn VectorStore>>,
}

impl CapabilityRegistry {
    pub fn register_importer(&mut self, format: String, adapter: Box<dyn ImportAdapter>);
    pub fn get_importer(&self, format: &str) -> Result<&dyn ImportAdapter, PluginError>;
    // ... similar for other capabilities
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

### D3: Markdown Importer as First Plugin

**Purpose:** Refactor existing Markdown importer to register as a plugin

**Files:**

| File | Action | Description |
|------|--------|-------------|
| `core/knowledge-import/src/features/importer/markdown.rs` | Modify | Implement `Plugin` trait for `MarkdownImporter` |
| `core/knowledge-import/src/features/importer/mod.rs` | Modify | Export `MarkdownImporterPlugin` |

**Implementation notes:**

The existing `MarkdownImporter` already implements `ImportAdapter`. We add `Plugin` implementation:

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

Similarly, `PdfImporter` and `UrlImporter` become plugins:

```rust
pub struct PdfImporterPlugin { manifest: PluginManifest }
pub struct UrlImporterPlugin { manifest: PluginManifest }
```

**Verification:**
- Unit test: MarkdownImporterPlugin manifest matches ImportAdapter capabilities
- Integration test: all 3 importers register correctly with CapabilityRegistry
- Integration test: existing import tests still pass (no regression)

**Exit criteria:** All 3 importers are plugins, existing tests pass

---

### D4: CLI Plugin Commands

**Purpose:** Expose plugin management via CLI

**Files:**

| File | Action | Description |
|------|--------|-------------|
| `cli/src/main.rs` | Modify | Add `Plugin` subcommand with `list` and `info` subcommands |
| `cli/features/prd-0003/plugin.feature` | Create | BDD scenarios for plugin management |
| `cli/tests/cucumber.rs` | Modify | Add step definitions for plugin commands |

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

  Scenario: List plugins
    Given the system has plugins loaded
    When I run "kos plugin list"
    Then I should see a list of plugins with name, version, and capabilities

  Scenario: Plugin info
    Given the system has plugins loaded
    When I run "kos plugin info markdown-importer"
    Then I should see detailed information about the markdown-importer plugin

  Scenario: Plugin failure isolation
    Given a plugin that fails during import
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

D1 defines types. D2 creates the plugin infrastructure. D3 refactors existing importers as plugins. D4 wires to CLI.

---

## Verification Strategy

| Level | Command | Coverage |
|-------|---------|----------|
| Unit | `cargo test -p knowledge-plugin` | Manifest parsing, registry, error boundaries |
| Integration | `cargo test -p knowledge-plugin --test integration_test` | Plugin loading, activation |
| E2E | `cargo test --test cucumber -p knowledge-cli` | CLI plugin commands |
| Regression | `cargo test -p knowledge-import` | Existing import tests still pass |
| Lint | `cargo clippy -- -D warnings && cargo fmt --check` | Code quality |

---

## Exit Criteria

- [ ] `knowledge-plugin` crate created and added to workspace
- [ ] TOML manifest parsing works
- [ ] `CapabilityRegistry` with register/retrieve for all capability types
- [ ] `safe_call` error boundary catches plugin failures
- [ ] Markdown, PDF, URL importers refactored as plugins
- [ ] `kos plugin list` and `kos plugin info` commands
- [ ] BDD tests: 3+ plugin scenarios
- [ ] Existing import tests pass (no regression)
- [ ] `cargo clippy -- -D warnings` passes
- [ ] ADR-0016 updated with Implementation Notes

---

## Implementation Notes

*(Filled in during/after implementation)*
