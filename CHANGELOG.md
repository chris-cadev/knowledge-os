# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/), and this project adheres to [Semantic Versioning](https://semver.org/).

---

## [0.5.0] - 2026-07-27

### Added

- `kos plugin install <path>` — Install a plugin from a directory containing `plugin.toml` to `~/.knowledge-os/plugins/` (G-024, F3.3)
- `kos plugin uninstall <name>` — Uninstall an installed plugin by name (G-025, F3.4)
- `CapabilityRegistry::deregister_importer` and `deregister_plugin` methods for plugin removal
- `KOS_PLUGIN_DIR` environment variable to override the default plugin directory
- BDD tests for plugin install/uninstall (`cli/features/prd-0003/plugin.feature`) — 5 new scenarios covering install, uninstall, missing manifest, not-found, and duplicate detection
- PRD-0004: Implementation Gaps — Cross-PRD Audit (`docs/engineering/prds/prd-0004-implementation-gaps.md`) — Consolidated gap inventory from PRDs 0001–0003 with 34 identified gaps, 30 test cases, and 8-week timeline
- CLI integration tests (`cli/tests/integration_test.rs`) — 12 tests covering import, search, list, get, archive/restore, rebuild-index, cross-refs, batch progress
- Derive integration tests (`core/knowledge-derive/tests/integration_test.rs`) — 12 tests covering views, search pipeline, hybrid RRF fusion
- Plugin integration tests (`core/knowledge-plugin/tests/integration_test.rs`) — 6 tests covering manifest parsing, registry, lifecycle
- BDD tests for extended import (`cli/features/prd-0002/extended-import.feature`) — 10 scenarios covering type inference, cross-refs, batch progress, mixed format
- BDD tests for resolution (`cli/features/prd-0002/resolution.feature`) — 8 scenarios covering exact duplicates, confidence scores, auto-merge, undo, strategy variation
- BDD tests for PDF import (`cli/features/prd-0002/pdf-import.feature`) — 8 scenarios covering invalid PDF handling, mixed directories
- BDD tests for import (`cli/features/prd-0001/import.feature`) — 13 scenarios covering frontmatter, tags, dedup, directory, cross-refs, idempotent reimport
- BDD tests for entity management (`cli/features/prd-0001/entity-management.feature`) — 7 scenarios covering CRUD, versioning, relationships
- BDD tests for search (`cli/features/prd-0001/search.feature`) — 11 scenarios covering keyword, type/tag filter, get, list, archive/restore lifecycle
- BDD tests for traversal (`cli/features/prd-0003/traversal.feature`) — 4 scenarios covering outgoing, bidirectional, depth limit, error handling
- BDD tests for views (`cli/features/prd-0003/views.feature`) — 8 scenarios covering tree, graph, table, timeline, empty database
- BDD tests for plugins (`cli/features/prd-0003/plugin.feature`) — 10 scenarios covering list, info, unknown plugin error, install, uninstall, missing manifest, not-found, duplicate detection
- BDD tests for semantic search (`cli/features/prd-0003/semantic-search.feature`) — 5 scenarios covering keyword default, semantic/hybrid warnings, mutual exclusivity
- BDD E2E tests (`cli/features/prd-0003/e2e-*.feature`) — 4 feature files, 19 scenarios for cross-cutting integration
- Storage integration tests for graph traversal (`core/knowledge-storage/tests/integration_test.rs`) — 12 tests covering chain, tree, cycle, diamond, bidirectional, depth limit, type filters
- Storage integration tests for merge audit (`core/knowledge-storage/tests/integration_test.rs`) — tests for undo, history by source/target
- Benchmarks: graph traversal (`core/knowledge-storage/benches/traversal.rs`), views (`core/knowledge-derive/benches/views.rs`), search (`core/knowledge-derive/benches/search.rs`)

### Changed

- PRD-0004 gaps G-024 and G-025 marked as Implemented; test cases T12 and T13 marked as Done
- README updated with CLI in project layout, technology table (Clap), and status section
- CHANGELOG updated with comprehensive CLI implementation entries
- Tutorial `first-import.md` corrected: binary name `kos` (was `knowledge-os`), command `get` (was `inspect`), removed undefined `derived status` command, updated derived data section to reflect actual `rebuild-index` workflow
- AGENTS.md updated with PRD-0003 and PRD-0004 in repository structure

---

## [0.4.0] - 2026-07-25

### Added

- Collection entity and repository trait (`core/knowledge-core/src/ports/mod.rs`) — `Collection` struct with `Uuid` ID, `name`, `description`, timestamps; `CollectionRepository` trait with 10 methods
- SQLite collection storage (`core/knowledge-storage/src/adapters/sqlite/mod.rs`) — `CollectionRepository` implementation for `SqliteStore`, `collections` and `collection_members` table migrations, `PRAGMA foreign_keys = ON`
- CLI collection commands (`cli/src/main.rs`) — `kos collection create|list|add|remove|members|delete` subcommands
- CLI traverse command (`cli/src/main.rs`) — `kos traverse <entity-id>` with `--depth`, `--direction`, `--type`, `--entity-type` flags; BFS graph traversal with cycle detection
- CLI view commands (`cli/src/main.rs`) — `kos view tree|graph|table|timeline` with per-view flags (type filter, sort, depth, from-entity)
- CLI plugin commands (`cli/src/main.rs`) — `kos plugin list|info` for built-in plugin inspection (markdown-importer, pdf-importer, url-importer)
- CLI resolution commands (`cli/src/main.rs`) — `kos resolution log|undo` for merge audit trail inspection and undo
- Graph traversal (`core/knowledge-storage/src/adapters/sqlite/traversal.rs`) — BFS with depth limiting, direction filter, relationship/entity type filter, cycle detection
- View adapters (`core/knowledge-derive/src/features/view/`) — Tree, Graph, Table, Timeline adapters with `ViewRegistry` and event-driven refresh
- Search pipeline (`core/knowledge-derive/src/features/search/`) — Embedding pipeline, in-memory vector store, RRF hybrid fusion, mock embedder
- PDF importer (`core/knowledge-import/src/importers/pdf.rs`) — PDF metadata extraction via `pdf_oxide`
- URL importer (`core/knowledge-import/src/importers/url.rs`) — URL content fetch via `reqwest` with rustls
- Extended cross-reference extraction — Wikilinks, @mentions, section anchors, external URLs
- Fuzzy entity resolution (`core/knowledge-storage/src/adapters/sqlite/fuzzy.rs`) — Jaro-Winkler confidence scoring with configurable thresholds, three-zone merge model
- Merge audit log with undo (`core/knowledge-storage/src/adapters/sqlite/merge_audit.rs`) — Snapshot-based merge reversal
- Plugin registry (`core/knowledge-plugin/src/registry.rs`) — `CapabilityRegistry` for registering importers, renderers, AI providers, vector stores
- Plugin manifest (`core/knowledge-plugin/src/manifest.rs`) — TOML manifest parsing with capability declarations
- BDD tests for collections (`cli/features/prd-0003/collections.feature`) — 12 scenarios covering CRUD, membership, cascade delete, duplicate rejection, multi-collection membership
- BDD tests for traversal (`cli/features/prd-0003/traversal.feature`) — 4 scenarios covering outgoing, bidirectional, depth limit, error handling
- BDD tests for views (`cli/features/prd-0003/views.feature`) — 8 scenarios covering tree, graph, table, timeline, empty database
- BDD tests for plugins (`cli/features/prd-0003/plugin.feature`) — 5 scenarios covering list, info, unknown plugin error
- BDD tests for semantic search (`cli/features/prd-0003/semantic-search.feature`) — 5 scenarios covering keyword default, semantic/hybrid warnings, mutual exclusivity
- BDD E2E tests (`cli/features/prd-0003/e2e-*.feature`) — 4 feature files, 19 scenarios for cross-cutting integration
- BDD tests for extended import (`cli/features/prd-0002/extended-import.feature`) — 10 scenarios covering type inference, cross-refs, batch progress, mixed format
- BDD tests for resolution (`cli/features/prd-0002/resolution.feature`) — 8 scenarios covering exact duplicates, confidence scores, auto-merge, undo, strategy variation
- BDD tests for PDF import (`cli/features/prd-0002/pdf-import.feature`) — 8 scenarios covering invalid PDF handling, mixed directories
- BDD tests for import (`cli/features/prd-0001/import.feature`) — 13 scenarios covering frontmatter, tags, dedup, directory, cross-refs, idempotent reimport
- BDD tests for entity management (`cli/features/prd-0001/entity-management.feature`) — 7 scenarios covering CRUD, versioning, relationships
- BDD tests for search (`cli/features/prd-0001/search.feature`) — 11 scenarios covering keyword, type/tag filter, get, list, archive/restore lifecycle
- Storage integration tests for collections (`core/knowledge-storage/tests/integration_test.rs`) — 10 tests covering CRUD, membership operations, cascade behavior
- Storage integration tests for graph traversal (`core/knowledge-storage/tests/integration_test.rs`) — 12 tests covering chain, tree, cycle, diamond, bidirectional, depth limit, type filters
- Storage integration tests for merge audit (`core/knowledge-storage/tests/integration_test.rs`) — tests for undo, history by source/target
- CLI integration tests (`cli/tests/integration_test.rs`) — 12 tests covering import, search, list, get, archive/restore, rebuild-index, cross-refs, batch progress
- Derive integration tests (`core/knowledge-derive/tests/integration_test.rs`) — 12 tests covering views, search pipeline, hybrid RRF fusion
- Plugin integration tests (`core/knowledge-plugin/tests/integration_test.rs`) — 6 tests covering manifest parsing, registry, lifecycle
- Benchmarks: graph traversal (`core/knowledge-storage/benches/traversal.rs`), views (`core/knowledge-derive/benches/views.rs`), search (`core/knowledge-derive/benches/search.rs`)
- Tree view collection integration — `TreeViewAdapter` now receives `Some(CollectionRepository)` enabling collection branches in tree view output
- README updated with CLI in project layout, technology table, and status section

### Changed

- ADR-0018 status updated from Proposed to Accepted with implementation notes
- `SqliteStore::new()` now executes `PRAGMA foreign_keys = ON` for cascade delete support
- `add_member` uses plain `INSERT` instead of `INSERT OR IGNORE` to enable duplicate membership detection

---

## [0.1.0] - 2026-07-21

### Added

- Foundational seed manifesto (`docs/foundational-manifesto.md`)
- Engineering architecture constitution (`docs/engineering-architecture.md`)
- Documentation structure following Diataxis framework
- README with architecture overview
- Contributing guidelines
- MIT License
- Architecture Decision Records framework
- Design principles documentation
- Goals and non-goals documentation
- System baseline architecture documentation
- Layered architecture deep dive
- Canonical vs derived data documentation
- Storage philosophy documentation
- Entity component model documentation
- Compiler perspective documentation
- Event-driven architecture documentation
- Project glossary

## [0.2.0] - 2026-07-21

### Added

- Vision document (`docs/philosophy/vision.md`) -- Part I of the manifesto
- Mental model document (`docs/architecture/mental-model.md`) -- Part III of the manifesto
- Domain model document (`docs/architecture/domain-model.md`) -- Part V of the manifesto
- AI architecture document (`docs/architecture/ai.md`) -- Part IX of the manifesto
- UI philosophy document (`docs/architecture/ui-philosophy.md`) -- Part VIII of the manifesto
- Engineering principles document (`docs/philosophy/engineering-principles.md`) -- Part X of the manifesto
- Extensibility document (`docs/architecture/extensibility.md`) -- Part XI of the manifesto
- Product vision document (`docs/philosophy/product-vision.md`) -- Part XII of the manifesto
- Governance document (`docs/philosophy/governance.md`) -- Part XIII of the manifesto
- Scalability document (`docs/architecture/scalability.md`)
- Synchronization document (`docs/architecture/synchronization.md`)
- Testing strategy document (`docs/engineering/testing-strategy.md`)
- Security architecture document (`docs/engineering/security.md`)
- Deployment architecture document (`docs/engineering/deployment.md`)
- Plugin development guide (`docs/guides/plugin-development.md`)
- AI agent guidelines (`docs/guides/ai-agent-guidelines.md`)
- 5 Architecture Decision Records (ADR-0001 through ADR-0005)

### Changed

- Expanded philosophy document with deeper analysis of principles, values, and anti-goals
- Expanded glossary with additional terms: Agent, Automation, Capability, Knowledge Graph, Metadata, Resource, Synchronization
- Updated ADR index with accepted status for all 5 ADRs
- Updated documentation README with complete reading order and new directory structure

## [0.3.0] - 2026-07-21

### Added

- Architectural principles document (`docs/architecture/architectural-principles.md`) -- Part VI consolidated invariants
- Appendices (`docs/appendices.md`) -- Part XV with diagrams, patterns, examples, model tables
- Expanded glossary (`docs/reference/glossary.md`) -- Part XIV canonical vocabulary with ~30 terms
- Expanded vision (`docs/philosophy/vision.md`) -- Part I deepened with concrete examples
- Expanded philosophy (`docs/philosophy/philosophy.md`) -- Part II deepened with implications and anti-goals
- Engineering handbook (`docs/engineering/engineering-handbook.md`) -- Git workflow, code review, CI/CD, debugging
- Operational runbooks (`docs/engineering/operational-runbooks.md`) -- 8 incident response procedures
- Product requirements (`docs/engineering/product-requirements.md`) -- Year 1 scope, FR/NFR, user stories
- UI design system (`docs/engineering/ui-design-system.md`) -- Design tokens, component specs, accessibility
- Tutorial: First Import (`docs/guides/tutorials/first-import.md`) -- Step-by-step walkthrough
- Tutorial: Build a Custom Importer (`docs/guides/tutorials/build-custom-importer.md`) -- Plugin development walkthrough
- API specification (`docs/engineering/api-specification.md`) -- REST and MCP API surfaces
- Infrastructure handbook (`docs/engineering/infrastructure-handbook.md`) -- Provisioning, scaling, monitoring, CI/CD

### Changed

- Converted all ASCII diagrams to Mermaid across 12+ architecture and engineering documents for consistent rendering
- Fixed Diataxis classification table in `docs/README.md` to accurately map each file to its actual content type instead of grouping all `engineering/` as How-to
- Expanded `docs/architecture/pipeline.md` with dedicated Indexing, Embedding, and Search subsections in Layer 6, plus Synchronization cross-reference
- Renamed "Core Belief" heading to "Core Philosophy" in `docs/philosophy/philosophy.md` for manifesto consistency
- Updated `docs/README.md` with new documents and tutorial reading order
- Updated root `README.md` with new status items and appendix documentation section
- Updated `AGENTS.md` with expanded repository structure
