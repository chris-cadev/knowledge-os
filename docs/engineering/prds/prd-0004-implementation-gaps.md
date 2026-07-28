# PRD-0004: Implementation Gaps — Cross-PRD Audit

**Status:** Draft
**Date:** 2026-07-27
**Author:** Core maintainers
**Priority:** P1 — Quality and Completeness Layer
**Depends on:** PRD-0001, PRD-0002, PRD-0003

---

## Purpose

This PRD captures the gaps between what PRD-0001, PRD-0002, and PRD-0003 specify and what the current implementation delivers. It consolidates every unspecified, partially implemented, and missing feature into a single work plan. Nothing in this PRD is new scope — it is the remaining work from existing PRDs.

---

## Problem Statement

PRDs 0001–0003 define the full scope of the Knowledge OS v1 system. Implementation has progressed rapidly: the core entity model, storage layer, import pipeline, derived representations, CLI, and plugin infrastructure are all built. But the audit reveals three categories of gaps:

1. **Specified but not implemented** — features the PRDs require but which were deferred or overlooked during implementation.
2. **Partially implemented** — features that work structurally but lack a critical component (e.g., plugin loading exists but custom plugins cannot be loaded).
3. **Specified but architecturally deviated** — features where the implementation took a different path than the PRD prescribed (e.g., BFS traversal instead of recursive CTE).

This PRD organizes these gaps into a prioritized backlog with clear acceptance criteria.

---

## Scope

### In Scope

- Remaining functional requirements from PRD-0001, PRD-0002, PRD-0003
- Architectural deviations that need reconciliation or ADR documentation
- CLI commands that were specified but not implemented
- Non-functional requirements that lack validation (benchmarks, load tests)
- Documentation corrections (tutorial inaccuracies, PRD typos)

### Out of Scope

- New features not covered by existing PRDs
- AI-assisted knowledge construction (Year 2)
- Multi-user collaboration (Year 3)
- Plugin marketplace (Year 4)

---

## Engineering Questions

### 1. Which canonical entities are introduced?

None. All entity types are defined in PRD-0001 (20 types) and PRD-0003 (Collection).

### 2. Which relationships are introduced?

None. The `references` relationship and its subtypes are defined in PRD-0001.

### 3. Which components are introduced?

| Component       | Description                                 | Source PRD |
| --------------- | ------------------------------------------- | ---------- |
| `BinaryContent` | Raw binary data for PDFs and non-text files | PRD-0002   |
| `Embedding`     | Vector representation for semantic search   | PRD-0003   |

### 4. Which events are emitted?

None. All 10 canonical event types are defined in PRD-0001.

### 5. Which derived representations are generated?

None new. The existing search index, view projections, and vector store need to be completed, not extended.

### 6. Which components own the features?

| Feature                 | Owner Component                                          |
| ----------------------- | -------------------------------------------------------- |
| PDF text extraction     | `knowledge-import`                                       |
| Configurable resolution | `knowledge-storage`                                      |
| Plugin loading          | `knowledge-plugin`                                       |
| Semantic search         | `knowledge-derive`                                       |
| NFR benchmarks          | `knowledge-storage`, `knowledge-derive`, `knowledge-cli` |

### 7. Can every derived artifact be regenerated?

Yes. Search indexes rebuild via `kos rebuild-index`. Views render from canonical data. Embeddings will be regenerated from content when a real provider is configured.

### 8. Does the feature violate storage independence?

No. All gaps are implementation completions within the existing adapter pattern.

### 9. Does the feature introduce implementation leakage?

No. The architectural deviations (BFS vs CTE, snapshot vs event-sourced undo) are internal implementation choices that do not leak into the domain model.

### 10. Does the feature preserve the canonical model?

Yes. The `BinaryContent` and `Embedding` components fit within the existing component model.

---

## Gap Inventory

### From PRD-0001: Core Entity Model

| ID    | Gap                                                                                                                         | Priority | Category        |
| ----- | --------------------------------------------------------------------------------------------------------------------------- | -------- | --------------- |
| G-001 | **F1.6: Query entities by component** — No CLI command or visible query for component-based lookup                          | P2       | Not implemented |
| G-002 | **F1.7: Query entities by tag** — Tag filtering exists as a search flag but no standalone tag query                         | P2       | Partial         |
| G-003 | **F1.8: Version history display** — Entity versioning works internally but `kos get` does not display version history       | P1       | Not implemented |
| G-004 | **F2.2: Update relationship attributes** — No mechanism to update relationship metadata after creation                      | P2       | Not implemented |
| G-005 | **F3.5: Batch import progress reporting** — `indicatif` is a dependency but progress bar is not wired into directory import | P1       | Not implemented |
| G-006 | **NF performance benchmarks** — All NFR latency and throughput targets (NF1.1–NF1.4, NF2.1–NF2.5) lack validation data      | P1       | Not validated   |

### From PRD-0002: Rich Import and Resolution

| ID    | Gap                                                                                                                                                                                  | Priority | Category        |
| ----- | ------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------ | -------- | --------------- |
| G-007 | **F1.2: PDF text body extraction** — Only metadata extraction via `pdf_oxide`; no text content from PDF body                                                                         | P0       | Not implemented |
| G-008 | **F1.4: Scanned PDF graceful fallback** — No fallback to `BinaryContent`-only with warning for image-only PDFs                                                                       | P1       | Not implemented |
| G-009 | **F1.5: PDF import from URL** — URL import uses `reqwest` but PDFs fetched from URLs are not routed to `PdfImporter`                                                                 | P1       | Not implemented |
| G-010 | **F2.3: Configurable merge thresholds per entity type** — Thresholds are global (0.92/0.78), not configurable per entity type as PRD requires                                        | P1       | Partial         |
| G-011 | **F2.6: Resolution strategy per entity type** — Only exact-match and fuzzy (Jaro-Winkler) implemented. Normalized-match and content-match strategies from PRD spec table are missing | P1       | Not implemented |
| G-012 | **BinaryContent component storage** — PRD specifies `BinaryContent` for PDF body storage; not implemented                                                                            | P0       | Not implemented |
| G-013 | **resolution_candidates table** — PRD specifies persisting evaluated candidates for audit; not implemented                                                                           | P2       | Not implemented |
| G-014 | **NF benchmarks** — PDF throughput (20 docs/sec), resolution latency (<50ms), false positive rate (<5%) all unvalidated                                                              | P1       | Not validated   |

### From PRD-0003: Graph Exploration, Views, and Plugins

| ID    | Gap                                                                                                                                  | Priority | Category        |
| ----- | ------------------------------------------------------------------------------------------------------------------------------------ | -------- | --------------- |
| G-015 | **F2.5: View synchronization** — Views render on-demand, not reactively via event-driven invalidation as PRD specifies               | P1       | Not implemented |
| G-016 | **F3.2: Plugin discovery and loading** — Manifest parsing and `CapabilityRegistry` exist, but dynamic loading from disk is not wired | P0       | Partial         |
| G-017 | **F3.4: Custom importer plugins** — Users cannot add new import formats without modifying core source                                | P0       | Not implemented |
| G-018 | **F3.5: Plugin sandboxing** — Resource limits and timeout limits per plugin are not implemented                                      | P2       | Not implemented |
| G-019 | **F3.6: Plugin hot-reload** — Not implemented (P2, acceptable)                                                                       | P2       | Not implemented |
| G-020 | **F4.1: Semantic search** — `kos search --semantic` warns and returns empty; no real AI provider wired                               | P0       | Stub            |
| G-021 | **F4.2: Embedding generation** — Mock embedder only; no configurable AI provider for real embeddings                                 | P0       | Stub            |
| G-022 | **F4.4: Hybrid search** — RRF fusion exists structurally but semantic component always empty; hybrid degrades to keyword-only        | P0       | Stub            |
| G-023 | **F4.5: Embedding model configuration** — No way to configure which embedding model or provider to use                               | P1       | Not implemented |
| G-024 | **`kos plugin install <path>`** — Copies plugin to plugin directory, validates manifest, registers capabilities                           | P0       | Implemented     |
| G-025 | **`kos plugin uninstall <name>`** — Removes plugin from directory, deregisters capabilities                                             | P1       | Implemented     |
| G-026 | **`embeddings` table** — No persistent vector storage; in-memory only, lost on restart                                               | P1       | Not implemented |
| G-027 | **NF benchmarks** — Traversal latency, view rendering latency, plugin load latency all unvalidated                                   | P1       | Not validated   |

### Documentation and Consistency Gaps

| ID    | Gap                                                                                                            | Priority | Category  |
| ----- | -------------------------------------------------------------------------------------------------------------- | -------- | --------- |
| G-028 | **PRD-0003 typo** — Line 532: `kos plugin卸载` uses Chinese character for "uninstall"                          | P2       | Typo      |
| G-029 | **PRD-0002 crate deviation** — PRD specifies `lopdf`; implementation uses `pdf_oxide`                          | P2       | Deviation |
| G-030 | **PRD-0002 undo architecture** — PRD specifies event-sourced undo; implementation uses snapshot-based undo     | P2       | Deviation |
| G-031 | **PRD-0003 traversal architecture** — PRD specifies recursive CTE; implementation uses BFS in application code | P2       | Deviation |
| G-032 | **Tutorial binary name** — `first-import.md` uses `knowledge-os` instead of `kos`                              | P1       | Fixed     |
| G-033 | **Tutorial command name** — `first-import.md` uses `inspect` instead of `get`                                  | P1       | Fixed     |
| G-034 | **Tutorial undefined command** — `first-import.md` uses `derived status` which does not exist                  | P1       | Fixed     |

---

## Functional Requirements

### F1: PDF Text Extraction (from PRD-0002)

| ID   | Requirement                                 | Priority | Acceptance Criteria                                                                                 |
| ---- | ------------------------------------------- | -------- | --------------------------------------------------------------------------------------------------- |
| F1.1 | Extract text body from PDF files            | P0       | `kos import paper.pdf` produces an entity with a `Content` component containing extracted text      |
| F1.2 | Extract PDF metadata (title, authors, date) | P0       | Metadata from PDF properties populates `Title`, `Author`, `Timeline` components                     |
| F1.3 | Handle scanned/image-only PDFs gracefully   | P1       | Scanned PDFs produce an entity with `BinaryContent` only; a warning is logged; import does not fail |
| F1.4 | Import PDF from URL                         | P1       | `kos import https://example.com/paper.pdf` fetches the PDF and routes it through the PDF importer   |

### F2: Configurable Resolution (from PRD-0002)

| ID   | Requirement                                   | Priority | Acceptance Criteria                                                                    |
| ---- | --------------------------------------------- | -------- | -------------------------------------------------------------------------------------- |
| F2.1 | Configurable merge thresholds per entity type | P1       | `kos` supports a config file or flags to set per-type auto-merge and review thresholds |
| F2.2 | Normalized-match resolution strategy          | P1       | Titles matching after lowercase + whitespace normalization trigger merge               |
| F2.3 | Content-match resolution strategy             | P2       | Entities with similar body text but different titles are flagged as candidates         |
| F2.4 | Persist resolution candidates                 | P2       | Evaluated candidates stored in `resolution_candidates` table for audit                 |

### F3: Plugin System Completion (from PRD-0003)

| ID   | Requirement                            | Priority | Acceptance Criteria                                                                                               |
| ---- | -------------------------------------- | -------- | ----------------------------------------------------------------------------------------------------------------- |
| F3.1 | Plugin discovery and loading from disk | P0       | `kos` scans a plugin directory on startup and loads manifests                                                     |
| F3.2 | Custom importer plugins                | P0       | A user-written `.wasm` or native plugin implementing `ImportAdapter` is discovered and available via `kos import` |
| F3.3 | `kos plugin install <path>`            | P0       | Copies plugin to plugin directory, validates manifest, registers capabilities                                     |
| F3.4 | `kos plugin uninstall <name>`          | P1       | Removes plugin from directory, deregisters capabilities                                                           |
| F3.5 | Plugin resource limits                 | P2       | Configurable memory and CPU limits per plugin                                                                     |
| F3.6 | Plugin timeout limits                  | P2       | Configurable execution timeout per plugin invocation                                                              |
| F3.7 | Plugin hot-reload                      | P2       | Detect plugin file changes and reload without restart                                                             |

### F4: Semantic and Hybrid Search (from PRD-0003)

| ID   | Requirement                             | Priority | Acceptance Criteria                                                          |
| ---- | --------------------------------------- | -------- | ---------------------------------------------------------------------------- |
| F4.1 | Configurable AI provider for embeddings | P0       | `kos` accepts configuration for an embedding provider (local or API-based)   |
| F4.2 | Embedding generation on import          | P0       | Imported entities receive embedding vectors from the configured provider     |
| F4.3 | Persistent vector storage               | P1       | Embeddings persist across restarts (not in-memory only)                      |
| F4.4 | Semantic search returns results         | P0       | `kos search "query" --semantic` returns entities ranked by cosine similarity |
| F4.5 | Hybrid search combines results          | P0       | `kos search "query" --hybrid` returns RRF-fused keyword + semantic results   |
| F4.6 | Embedding model configuration           | P1       | User can specify embedding model, dimensions, and provider                   |

### F5: View Synchronization (from PRD-0003)

| ID   | Requirement                           | Priority | Acceptance Criteria                                                                 |
| ---- | ------------------------------------- | -------- | ----------------------------------------------------------------------------------- |
| F5.1 | Views invalidate on canonical changes | P1       | After `kos import` or `kos archive`, subsequent `kos view` calls reflect the change |
| F5.2 | Event-driven view refresh             | P2       | `ViewAdapter::synchronize(&self, event: &Event)` is called on canonical events      |

### F6: CLI Completeness (from PRD-0001)

| ID   | Requirement               | Priority | Acceptance Criteria                                                             |
| ---- | ------------------------- | -------- | ------------------------------------------------------------------------------- |
| F6.1 | Version history display   | P1       | `kos get <id>` shows version history with snapshots                             |
| F6.2 | Batch import progress bar | P1       | `kos import <directory>` shows `indicatif` progress bar with file count and ETA |

### F7: Performance Validation (from all PRDs)

| ID   | Requirement                    | Priority | Acceptance Criteria                                                           |
| ---- | ------------------------------ | -------- | ----------------------------------------------------------------------------- |
| F7.1 | Entity CRUD latency benchmarks | P1       | Create/read/update/archive < 10ms at 100K entities                            |
| F7.2 | Search latency benchmarks      | P1       | Keyword search < 50ms at 100K entities                                        |
| F7.3 | Traversal latency benchmarks   | P1       | 2-hop traversal < 100ms, 3-hop < 500ms at 100K entities                       |
| F7.4 | View rendering benchmarks      | P1       | All view types render < 100ms at 100K entities                                |
| F7.5 | Import throughput benchmarks   | P1       | Markdown import ≥ 100 docs/sec, PDF import ≥ 20 docs/sec                      |
| F7.6 | Volume testing                 | P2       | System functions correctly at 100K entities, 1M relationships, 10M components |

---

## CLI Interface

No new commands. All gaps relate to completing existing commands or adding flags.

### Modified Commands

| Command                 | Change                      | Gap         |
| ----------------------- | --------------------------- | ----------- |
| `kos get <id>`          | Add version history display | G-003, F6.1 |
| `kos import <dir>`      | Add progress bar            | G-005, F6.2 |
| `kos import paper.pdf`  | Add text body extraction    | G-007, F1.1 |
| `kos search --semantic` | Wire real AI provider       | G-020, F4.4 |
| `kos search --hybrid`   | Wire real hybrid fusion     | G-022, F4.5 |

### New Commands

| Command                       | Description                | Gap         |
| ----------------------------- | -------------------------- | ----------- |
| `kos plugin install <path>`   | Install a plugin from disk | G-024, F3.3 |
| `kos plugin uninstall <name>` | Uninstall a plugin         | G-025, F3.4 |

### New Flags

| Flag              | Command      | Description                                                              | Gap               |
| ----------------- | ------------ | ------------------------------------------------------------------------ | ----------------- |
| `--config <path>` | All commands | Path to configuration file for thresholds, AI provider, plugin directory | G-010, F2.1, F4.1 |

---

## Crate Changes

### knowledge-import

- Add PDF text body extraction (replace or extend `pdf_oxide` metadata-only approach)
- Add scanned PDF detection and `BinaryContent` fallback
- Route URL-fetched PDFs through `PdfImporter`

### knowledge-storage

- Add `resolution_candidates` table for audit
- Add `embeddings` table for persistent vector storage
- Add configurable resolution thresholds per entity type

### knowledge-derive

- Wire real AI provider for embedding generation
- Implement persistent vector store (replace in-memory)
- Complete hybrid search with real semantic results

### knowledge-plugin

- Implement plugin directory scanning and dynamic loading
- Wire plugin installation and uninstallation to CLI
- Add resource and timeout limits for sandboxing

### knowledge-core

- Add `BinaryContent` component type (if not already defined)
- Add `Embedding` component type (if not already defined)

### knowledge-cli

- Add `--config` flag for configuration
- Add `indicatif` progress bar to directory import
- Add version history to `kos get` output
- Add `kos plugin install` and `kos plugin uninstall` commands

---

## Non-Functional Requirements

| ID   | Requirement                    | Target                                          | Validation Method                          |
| ---- | ------------------------------ | ----------------------------------------------- | ------------------------------------------ |
| NF1  | Entity CRUD latency            | < 10ms at 100K entities                         | Criterion benchmark in `knowledge-storage` |
| NF2  | Search latency                 | < 50ms at 100K entities                         | Criterion benchmark in `knowledge-derive`  |
| NF3  | Traversal latency (2-hop)      | < 100ms at 100K entities                        | Criterion benchmark in `knowledge-storage` |
| NF4  | Traversal latency (3-hop)      | < 500ms at 100K entities                        | Criterion benchmark in `knowledge-storage` |
| NF5  | View rendering                 | < 100ms at 100K entities                        | Criterion benchmark in `knowledge-derive`  |
| NF6  | Markdown import throughput     | ≥ 100 docs/sec                                  | Criterion benchmark                        |
| NF7  | PDF import throughput          | ≥ 20 docs/sec                                   | Criterion benchmark                        |
| NF8  | Resolution latency             | < 50ms per candidate                            | Criterion benchmark                        |
| NF9  | Resolution false positive rate | < 5%                                            | Statistical test against known duplicates  |
| NF10 | Volume correctness             | 100K entities, 1M relationships, 10M components | Volume test suite                          |

---

## Test Cases

### PDF Import

| #   | Test Case                                                               | Priority |
| --- | ----------------------------------------------------------------------- | -------- |
| T1  | Import single PDF — extracts text body into `Content` component         | P0       |
| T2  | Import PDF with metadata — extracts title, authors, date                | P0       |
| T3  | Import scanned PDF — produces `BinaryContent` only, logs warning        | P1       |
| T4  | Import PDF from URL — fetches and routes through PDF importer           | P1       |
| T5  | Import mixed directory with PDFs and Markdown — all processed correctly | P1       |

### Configurable Resolution

| #   | Test Case                                                                   | Priority |
| --- | --------------------------------------------------------------------------- | -------- |
| T6  | Set per-type threshold — Person threshold differs from Concept threshold    | P1       |
| T7  | Normalized-match — "Machine Learning" and "machine learning" trigger merge  | P1       |
| T8  | Content-match — different titles, similar body text flagged as candidates   | P2       |
| T9  | Resolution candidates persisted — audit trail includes evaluated candidates | P2       |

### Plugin System

| #   | Test Case                                                                     | Priority |
| --- | ----------------------------------------------------------------------------- | -------- |
| T10 | Plugin discovery — startup scans directory and loads valid manifests          | P0       |
| T11 | Custom importer — user plugin provides new format, available via `kos import` | P0       |
| T12 | `kos plugin install` — copies plugin, validates manifest, registers           | P0       | ✅ Done |
| T13 | `kos plugin uninstall` — removes plugin, deregisters                          | P1       | ✅ Done |
| T14 | Plugin failure isolation — bad plugin does not crash core system              | P1       |
| T15 | Plugin timeout — plugin exceeding timeout is terminated                       | P2       |

### Semantic and Hybrid Search

| #   | Test Case                                                               | Priority |
| --- | ----------------------------------------------------------------------- | -------- |
| T16 | Configure AI provider — `kos` accepts provider config                   | P0       |
| T17 | Embedding on import — imported entity receives vector                   | P0       |
| T18 | Semantic search — `kos search --semantic` returns cosine-ranked results | P0       |
| T19 | Hybrid search — `kos search --hybrid` returns RRF-fused results         | P0       |
| T20 | Embedding persistence — vectors survive restart                         | P1       |

### View Synchronization

| #   | Test Case                                                          | Priority |
| --- | ------------------------------------------------------------------ | -------- |
| T21 | Import then view — new entity appears in tree/graph/table/timeline | P1       |
| T22 | Archive then view — archived entity removed from views             | P1       |

### CLI Completeness

| #   | Test Case                                                             | Priority |
| --- | --------------------------------------------------------------------- | -------- |
| T23 | `kos get` shows version history — versions and snapshots displayed    | P1       |
| T24 | Directory import shows progress bar — `indicatif` bar with file count | P1       |

### Performance

| #   | Test Case                                                         | Priority |
| --- | ----------------------------------------------------------------- | -------- |
| T25 | CRUD latency benchmark — all operations < 10ms at 100K entities   | P1       |
| T26 | Search latency benchmark — keyword search < 50ms at 100K entities | P1       |
| T27 | Traversal latency benchmark — 2-hop < 100ms, 3-hop < 500ms        | P1       |
| T28 | View rendering benchmark — all views < 100ms at 100K entities     | P1       |
| T29 | Import throughput benchmark — Markdown ≥ 100/sec, PDF ≥ 20/sec    | P1       |
| T30 | Volume test — 100K entities, 1M relationships, 10M components     | P2       |

---

## Architecture Decision Records

The following architectural deviations from existing PRDs should be reconciled via ADR:

| Deviation                            | PRD      | Implementation             | Recommended Action                  |
| ------------------------------------ | -------- | -------------------------- | ----------------------------------- |
| BFS traversal vs recursive CTE       | PRD-0003 | BFS in application code    | Accept BFS; update PRD-0003 text    |
| Snapshot-based undo vs event-sourced | PRD-0002 | Snapshot with stored state | Accept snapshots; ADR-0019          |
| `pdf_oxide` vs `lopdf`               | PRD-0002 | `pdf_oxide`                | Accept `pdf_oxide`; update PRD-0002 |

---

## Timeline

| Phase                                 | Duration | Deliverables                     |
| ------------------------------------- | -------- | -------------------------------- |
| Phase 1: PDF text extraction          | 1 week   | F1.1–F1.4, T1–T5                 |
| Phase 2: Configurable resolution      | 3 days   | F2.1–F2.4, T6–T9                 |
| Phase 3: Plugin system completion     | 2 weeks  | F3.1–F3.7, T10–T15               |
| Phase 4: Semantic search              | 2 weeks  | F4.1–F4.6, T16–T20               |
| Phase 5: View sync + CLI completeness | 1 week   | F5.1–F5.2, F6.1–F6.2, T21–T24    |
| Phase 6: Performance benchmarks       | 1 week   | F7.1–F7.6, T25–T30               |
| Phase 7: Documentation reconciliation | 3 days   | ADRs for deviations, PRD updates |

**Total: ~8 weeks**

---

## Definition of Done

- [ ] All P0 functional requirements implemented and tested
- [ ] All P1 functional requirements implemented and tested
- [ ] P2 requirements implemented or explicitly deferred with ADR
- [ ] All P0 and P1 test cases passing
- [ ] Performance benchmarks meet NFR targets
- [ ] Architectural deviations reconciled via ADRs
- [ ] CLI surface matches PRD specifications
- [ ] Documentation corrected (tutorial, PRD typos)
