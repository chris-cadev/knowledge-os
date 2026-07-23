# PRD-0001: Core Entity Model and Markdown Import Pipeline

**Status:** Draft
**Date:** 2026-07-22
**Author:** Core maintainers
**Priority:** P0 — Foundation

---

## Purpose

This PRD defines the first implementation of Knowledge OS. It specifies the core entity model, component storage, relationship storage, Markdown import pipeline, and basic search. This is the keystone feature — every downstream capability depends on this working correctly.

---

## Problem Statement

Current knowledge management systems conflate storage with meaning. A file system knows that a file exists. It does not know what the file is about. Knowledge OS bridges this gap by transforming documents into typed entities with components and relationships.

The first implementation must prove this thesis: a Markdown file enters the system and emerges as a first-class entity with typed components, queryable relationships, and a disposable search index.

---

## Scope

### In Scope

- Core entity storage (create, read, update, archive)
- Component attachment and lifecycle
- Relationship storage and 1-hop traversal
- Markdown importer (file → entity with components)
- Entity resolution (duplicate detection and merge at import time)
- Full-text search index (derived, disposable, pluggable)
- CLI interface (`kos` binary) for import and query
- Durable event log for pipeline processing

### Out of Scope

- PDF import (complex parser, deferred to PRD-0002)
- Graph view rendering (UI, deferred)
- Embeddings and semantic search (derived, deferred)
- Plugin system (designed alongside, not critical for first working system)
- Multi-user collaboration (Year 3)
- AI-assisted extraction (Year 2)
- N-hop relationship traversal (deferred to PRD-0003)

---

## Engineering Questions

Every feature proposed for the system must answer these 10 questions per [Engineering Principles](../../philosophy/engineering-principles.md):

### 1. Which canonical entities are introduced?

No new entity types are introduced. The system uses the core entity types defined in `docs/architecture/domain-model.md`: Concept, Person, Organization, Project, Book, Paper, Video, Article, Tool, Technology, Question, Idea, Event, Skill, Location, Dataset, Collection, Workspace, Decision, Note.

The Markdown importer produces entities with type determined by content analysis or user specification. Default type is `Article` for Markdown files.

### 2. Which relationships are introduced?

No new relationship types are introduced. The system uses the core relationship types from `docs/architecture/domain-model.md`. The Markdown importer extracts explicit cross-references as `references` relationships between entities.

### 3. Which components are introduced?

No new component types are introduced. The system uses the core component types from `docs/architecture/domain-model.md`. The Markdown importer produces entities with these components:

| Component    | Source                                                                 |
| ------------ | ---------------------------------------------------------------------- |
| `Title`      | YAML frontmatter `title` field, or first H1 heading, or filename       |
| `Content`    | Full Markdown body                                                     |
| `Tags`       | YAML frontmatter `tags` field                                          |
| `Timeline`   | File modification date for `created_at`, import time for `modified_at` |
| `Language`   | Detected from content (defaults to `en`)                               |
| `Provenance` | Import source path, import timestamp, importer name                    |

### 4. Which events are emitted?

| Event                  | Trigger                      | Consumers            |
| ---------------------- | ---------------------------- | -------------------- |
| `EntityCreated`        | New entity stored            | Search index updater |
| `EntityUpdated`        | Entity component changed     | Search index updater |
| `EntityArchived`       | Entity marked inactive       | Search index updater |
| `RelationshipCreated`  | New relationship stored      | Search index updater |
| `RelationshipArchived` | Relationship marked inactive | Search index updater |

### 5. Which derived representations are generated?

| Derived Artifact       | Source                        | Regeneration                    |
| ---------------------- | ----------------------------- | ------------------------------- |
| Full-text search index | Entity Title + Content + Tags | Drop and rebuild from canonical |
| Entity type index      | Entity type field             | Rebuild from canonical          |
| Tag index              | Entity Tags component         | Rebuild from canonical          |

All derived data is disposable. The search index is rebuilt from canonical data at any time.

### 6. Which layer owns the feature?

| Feature              | Layer                                                          |
| -------------------- | -------------------------------------------------------------- |
| Entity storage       | Layer 4 — Knowledge Model                                      |
| Component storage    | Layer 4 — Knowledge Model                                      |
| Relationship storage | Layer 5 — Relationship Engine                                  |
| Markdown import      | Layer 1 (Import) + Layer 2 (Parsing) + Layer 3 (Normalization) |
| Search index         | Layer 6 — Derivation                                           |

### 7. Can every derived artifact be regenerated?

Yes. The search index is the only derived artifact. It is rebuilt from canonical entity data. No canonical information is lost if the index is dropped.

### 8. Does the feature violate storage independence?

No. Entity storage uses an adapter interface. The initial implementation uses SQLite. The adapter may be replaced without changing the domain model. See ADR-0002.

### 9. Does the feature introduce implementation leakage?

No. The domain model is defined in terms of entities, components, and relationships. Storage details (SQLite, table names, column types) are confined to the storage adapter. The domain model depends on the adapter interface, never on the implementation.

### 10. Does the feature preserve the canonical model?

Yes. The canonical model is the source of truth. The search index is derived. Entities are versioned. Components are attached to entities. Relationships connect entities. All invariants from `docs/architecture/domain-model.md` are preserved.

---

## Pipeline Spine Analysis

The system spine determines the fundamental flow through the pipeline. The correct spine is: **Import → Extract → Resolve → Store → Connect → Search**.

### Why This Spine

The spine must follow this order because of data dependencies:

| Step        | Action                                                          | Why This Order                                                              |
| ----------- | --------------------------------------------------------------- | --------------------------------------------------------------------------- |
| **Import**  | Read raw input from file system, API, or other source           | Nothing exists until data enters the system                                 |
| **Extract** | Transform raw input into structured data (entities, components) | Structured data must exist before we can resolve it                         |
| **Resolve** | Identify duplicates, assign canonical IDs, normalize metadata   | Resolution must happen before storage — dirty data in = dirty graph forever |
| **Store**   | Persist resolved entities to the canonical model                | Storage happens after resolution ensures data quality                       |
| **Connect** | Establish relationships between stored entities                 | Connections require both source and target entities to exist                |
| **Search**  | Index stored entities for retrieval                             | Search indexes what is stored, not what is being processed                  |

### Why Resolution Before Storage

Entity resolution is the critical quality layer. The evidence from the 2026 landscape (see [Landscape 2026](../research/landscape-2026.md)) confirms that enterprise knowledge graphs fail when resolution is deferred:

- Children's Medical Center Dallas found 22% of records were duplicates before proper resolution.
- LLMs consistently produce duplicate entities in GraphRAG pipelines.
- "Extraction is the part that demos, resolution is the part that matters."

If resolution happens after storage, duplicates accumulate and downstream layers (search, relationships, AI) operate on noisy data. The graph degrades before correction is applied.

Resolution before storage means the canonical model is clean from the first import. Every entity in the graph has a verified canonical ID. Every downstream layer operates on resolved, deduplicated data.

### Alternative Spines Considered

| Spine                                                 | Description                           | Why Rejected                                                                                  |
| ----------------------------------------------------- | ------------------------------------- | --------------------------------------------------------------------------------------------- |
| Import → Store → Extract → Connect → Search           | Store raw data first, process later   | Raw data contaminates the canonical model                                                     |
| Import → Extract → Store → Connect → Resolve → Search | Resolve after storage and connections | Duplicates compound before correction; relationships are built on noisy data                  |
| Import → Extract → Connect → Store → Search           | Connect before store                  | Requires entities to exist before storage, which is architecturally incorrect                 |
| Import → Resolve → Extract → Store → Connect → Search | Resolve before extraction             | Resolution requires structured data; raw input cannot be resolved without extraction          |
| Import → Extract → Resolve → Store → Connect → Search | **Chosen spine**                      | Resolution happens on extracted data before storage; canonical model is clean by construction |

### Resolution as Continuous Process

Entity resolution is not a one-time operation at import. It is continuous:

1. **At import time:** New input is checked against existing entities before storage.
2. **After storage:** Periodic resolution passes catch duplicates that were missed.
3. **At query time:** Results are presented with entity confidence scores.

This ensures quality is maintained by construction, not corrected after the fact.

---

## Functional Requirements

### F1: Entity Management

| ID   | Requirement                                | Priority | Acceptance Criteria                                       |
| ---- | ------------------------------------------ | -------- | --------------------------------------------------------- |
| F1.1 | Create entities with a type and components | P0       | Entity is created with UUID, type, and initial components |
| F1.2 | Update entity components                   | P0       | Component change increments entity version                |
| F1.3 | Archive entities (soft delete)             | P0       | Entity is marked inactive, history preserved              |
| F1.4 | Restore archived entities                  | P1       | Entity is marked active again                             |
| F1.5 | Query entities by type                     | P0       | Returns all entities of a given type                      |
| F1.6 | Query entities by component                | P1       | Returns entities with a specific component type           |
| F1.7 | Query entities by tag                      | P1       | Returns entities with a specific tag                      |
| F1.8 | Version history for entities               | P1       | Entity version history is queryable                       |

### F2: Relationship Management

| ID   | Requirement                                 | Priority | Acceptance Criteria                                   |
| ---- | ------------------------------------------- | -------- | ----------------------------------------------------- |
| F2.1 | Create typed relationships between entities | P0       | Relationship is created with type, source, and target |
| F2.2 | Update relationship attributes              | P1       | Relationship attributes are updated                   |
| F2.3 | Archive relationships                       | P0       | Relationship is marked inactive                       |
| F2.4 | Traverse relationships (1 hop)              | P0       | From any entity, retrieve directly connected entities |
| F2.5 | Traverse relationships (N hops)             | P1       | From any entity, retrieve entities within N hops      |
| F2.6 | Query relationships by type                 | P0       | Returns all relationships of a given type             |

### F3: Import Pipeline

| ID   | Requirement                   | Priority | Acceptance Criteria                                               |
| ---- | ----------------------------- | -------- | ----------------------------------------------------------------- |
| F3.1 | Import Markdown files         | P0       | Markdown file produces Entity with Content component              |
| F3.2 | Extract metadata from imports | P0       | Author, date, language are extracted from YAML frontmatter        |
| F3.3 | Import from file system       | P0       | Local files are importable                                        |
| F3.4 | Batch import                  | P1       | Multiple files are importable in one operation                    |
| F3.5 | Import progress reporting     | P1       | Import progress is visible to the user                            |
| F3.6 | Import error handling         | P0       | Malformed input is logged, not silently swallowed                 |
| F3.7 | Cross-reference extraction    | P0       | Markdown links between files produce `references` relationships   |
| F3.8 | Idempotent reimport           | P0       | Reimporting same file updates existing entity (version increment) |

### F4: Entity Resolution

| ID   | Requirement               | Priority | Acceptance Criteria                                                |
| ---- | ------------------------- | -------- | ------------------------------------------------------------------ |
| F4.1 | Exact match resolution    | P0       | Duplicate detection by title + entity type                         |
| F4.2 | Fuzzy match resolution    | P1       | Duplicate detection by title similarity (Levenshtein/Jaro-Winkler) |
| F4.3 | Confidence scoring        | P0       | Each resolution candidate has a confidence score                   |
| F4.4 | Auditable merge decisions | P0       | Merge operations are logged with reason and confidence             |
| F4.5 | Import-time resolution    | P0       | New imports checked against existing entities before storage       |

### F5: Search and Retrieval

| ID   | Requirement              | Priority | Acceptance Criteria                                               |
| ---- | ------------------------ | -------- | ----------------------------------------------------------------- |
| F5.1 | Full-text search         | P0       | Query matches against indexed text fields via `SearchIndex` trait |
| F5.2 | Entity type filtering    | P0       | Search results can be filtered by entity type                     |
| F5.3 | Tag filtering            | P1       | Search results can be filtered by tags                            |
| F5.4 | Search result ranking    | P0       | Results are ranked by relevance                                   |
| F5.5 | Search index rebuild     | P0       | Index can be dropped and rebuilt from canonical entities          |
| F5.6 | Pluggable search backend | P0       | Search implementation is swappable via trait interface            |

---

## Non-Functional Requirements

### NF1: Performance

| ID    | Requirement                  | Target       | Acceptable  |
| ----- | ---------------------------- | ------------ | ----------- |
| NF1.1 | Import throughput (Markdown) | 100 docs/sec | 50 docs/sec |
| NF1.3 | Search query latency         | < 50ms       | < 200ms     |
| NF1.4 | Entity retrieval latency     | < 10ms       | < 50ms      |
| NF1.6 | Entity creation latency      | < 10ms       | < 50ms      |

### NF2: Scalability

| ID    | Requirement         | Target           |
| ----- | ------------------- | ---------------- |
| NF2.1 | Entity volume       | 100K entities    |
| NF2.2 | Relationship volume | 1M relationships |
| NF2.3 | Component volume    | 10M components   |
| NF2.4 | Concurrent users    | 1 user (Year 1)  |
| NF2.5 | Import batch size   | 10K files        |

### NF3: Reliability

| ID    | Requirement                  | Target                                       |
| ----- | ---------------------------- | -------------------------------------------- |
| NF3.1 | Data durability              | Zero canonical data loss                     |
| NF3.2 | Derived data rebuildability  | All derived data rebuildable from canonical  |
| NF3.3 | Event processing reliability | Zero event loss (durable event log)          |
| NF3.4 | Pipeline idempotency         | Reprocessing same input produces same output |

### NF4: Usability

| ID    | Requirement                   | Acceptance Criteria                                                              |
| ----- | ----------------------------- | -------------------------------------------------------------------------------- |
| NF4.1 | First import within 5 minutes | User can import a Markdown file and see results within 5 minutes of installation |

---

## User Stories

### US1: Import a Markdown File

**As a** knowledge worker,
**I want to** import a Markdown file into Knowledge OS,
**So that** it becomes a typed entity with components.

**Acceptance criteria:**
1. User runs `kos import <file.md>`.
2. System creates an Entity with type `Article` (default).
3. System extracts Title from YAML frontmatter or first H1 heading.
4. System extracts Content from the Markdown body.
5. System extracts Tags from YAML frontmatter.
6. System sets Timeline from file metadata.
7. System stores Provenance with import source and timestamp.
8. System checks for duplicate entities (resolution) before storage.
9. Entity is stored in the canonical model.
10. Search index is updated.
11. If a duplicate is found, the existing entity is updated (version incremented).

### US2: Import a Directory of Markdown Files

**As a** knowledge worker,
**I want to** import a folder of Markdown files,
**So that** each file becomes a separate entity.

**Acceptance criteria:**
1. User runs `kos import <directory>`.
2. System imports all `.md` files in the directory.
3. Each file produces a separate entity.
4. Cross-references between files are extracted as `references` relationships.
5. All entities appear in the knowledge graph.
6. Search index is updated for all entities.

### US3: Search for Entities

**As a** knowledge worker,
**I want to** search for entities by topic,
**So that** I can find relevant information quickly.

**Acceptance criteria:**
1. User runs `kos search "query"`.
2. System returns entities matching the query.
3. Results are ranked by relevance.
4. Results include entity type, title, and snippet.
5. User can filter results by entity type (`--type paper`).
6. User can filter results by tags (`--tag machine-learning`).

### US4: View Entity Details

**As a** knowledge worker,
**I want to** view the details of an entity,
**So that** I can inspect its components and relationships.

**Acceptance criteria:**
1. User runs `kos get <entity-id>`.
2. System displays entity type, components, and version.
3. System displays directly connected entities (1-hop traversal).
4. System displays relationship types and directions.

### US5: List Entities by Type

**As a** knowledge worker,
**I want to** list all entities of a given type,
**So that** I can browse my knowledge by category.

**Acceptance criteria:**
1. User runs `kos list --type article`.
2. System returns all entities of type `Article`.
3. Results include entity ID, title, and tags.
4. User can combine with search query.

---

## Architecture

### Crate Structure

| Crate               | Purpose                                                          |
| ------------------- | ---------------------------------------------------------------- |
| `knowledge-core`    | Domain model: entities, components, relationships, ports, events |
| `knowledge-storage` | Storage adapters (SQLite canonical, SQLite FTS5 search)          |
| `knowledge-import`  | Import pipeline: Markdown parsing, entity creation, resolution   |
| `knowledge-derive`  | Derivation pipeline: pluggable search interface                  |
| `knowledge-cli`     | `kos` binary: CLI commands for user interaction                  |
| `knowledge-api`     | REST/MCP API (future, not PRD-0001 scope)                        |

### Storage

| Store              | Technology            | Purpose                                           |
| ------------------ | --------------------- | ------------------------------------------------- |
| Canonical entities | SQLite                | Entity, component, and relationship storage       |
| Search index       | SQLite FTS5 (initial) | Full-text search (derived, disposable, pluggable) |
| Event log          | SQLite                | Durable event log for pipeline processing         |

### Pluggable Search Architecture

The search index is accessed through a `SearchIndex` trait (port). The initial implementation uses SQLite FTS5 for simplicity. Alternative implementations (Tantivy, Quickwit) may be added as separate adapters without changing the domain model.

```rust
#[async_trait]
pub trait SearchIndex: Send + Sync {
    async fn index_entity(&self, entity: &Entity, components: &[Component]) -> Result<()>;
    async fn remove_entity(&self, entity_id: Uuid) -> Result<()>;
    async fn search(&self, query: &SearchQuery) -> Result<SearchResults>;
    async fn rebuild(&self, entities: &[(Entity, Vec<Component>)]) -> Result<()>;
}
```

The search index is derived data. It is disposable and rebuildable from canonical entities at any time.

### Entity Resolution System

Entity resolution is the critical quality layer per ADR-0006. The resolution system is accessed through a trait:

```rust
#[async_trait]
pub trait EntityResolver: Send + Sync {
    async fn find_candidates(&self, entity: &Entity) -> Result<Vec<ResolutionCandidate>>;
    async fn merge(&self, canonical_id: Uuid, duplicate_id: Uuid, confidence: f64) -> Result<()>;
}
```

Resolution strategies:
- **Exact match:** Title + entity type equality (fast, high confidence)
- **Fuzzy match:** Levenshtein / Jaro-Winkler distance on title (handles typos)
- **Configurable per entity type:** Different strategies for Person vs. Concept vs. Article

Resolution runs at import time (before storage) and continuously (periodic passes). Every merge decision is auditable with a confidence score.

### Event System

Every meaningful modification to canonical data emits a durable event. Events are stored in SQLite and processed asynchronously through the derivation pipeline.

Canonical events:
- `EntityCreated`, `EntityUpdated`, `EntityArchived`
- `RelationshipCreated`, `RelationshipUpdated`, `RelationshipArchived`
- `ComponentAdded`, `ComponentUpdated`, `ComponentRemoved`

Events support idempotent processing (via event deduplication), replay (for derived data rebuild), and dead-letter queue (for failed processing).

### Pipeline Flow

```
Markdown File
      |
  Import Layer         knowledge-cli: kos import command
      |
  Parsing Layer        pulldown-cmark: YAML frontmatter + Markdown body
      |
  Normalization Layer  Entity resolution, metadata normalization
      |
  Knowledge Model      Entity + Components stored in SQLite (knowledge-storage)
      |
  Relationship Engine  Cross-reference extraction, relationship storage
      |
  Derivation Layer     Search index update via SearchIndex trait (knowledge-derive)
      |
  Event Log            Durable event emission to SQLite
      |
  (Presentation)       CLI output (knowledge-cli)
```

### Data Flow

1. User invokes `kos import <file.md>`.
2. Import layer reads the file from the file system.
3. Parsing layer extracts YAML frontmatter (title, tags, date, author) and Markdown body.
4. Normalization layer creates an Entity with type, Title, Content, Tags, Timeline, Language, Provenance.
5. Entity resolution checks for duplicates against existing entities.
6. Knowledge model stores the resolved entity in SQLite.
7. Relationship engine extracts cross-references and stores relationships.
8. Derivation layer updates the search index via the `SearchIndex` trait.
9. Event log records all canonical changes.
10. CLI outputs the created entity with formatted results.

---

## CLI Interface

### Commands

```bash
# Import a single file
kos import <file.md>

# Import a directory
kos import <directory>

# Search entities
kos search "query"
kos search "query" --type article
kos search "query" --tag machine-learning

# Get entity details
kos get <entity-id>

# List entities by type
kos list --type article
kos list --type paper --tag transformer

# Archive an entity
kos archive <entity-id>

# Restore an entity
kos restore <entity-id>
```

### Output Format

```
$ kos import paper.md
Imported: Entity abc123 (Article)
  Title: "Attention Is All You Need"
  Tags: transformer, attention, NLP
  Content: 2847 words
  Search index: updated
```

```
$ kos search "transformer"
Found 3 entities:

  [Article] abc123 — "Attention Is All You Need"
    Tags: transformer, attention, NLP
    Snippet: ...the transformer architecture relies on self-attention...

  [Paper] def456 — "BERT: Pre-training of Deep Bidirectional Transformers"
    Tags: transformer, NLP, pre-training
    Snippet: ...we introduce BERT, which is designed to pre-train...

  [Concept] ghi789 — "Self-Attention Mechanism"
    Tags: transformer, attention, neural-networks
    Snippet: ...self-attention allows the model to relate different positions...
```

---

## Acceptance Criteria

### Definition of Done

- [ ] Entity CRUD operations work (create, read, update, archive, restore)
- [ ] Components attach to entities and increment version on change
- [ ] Relationships connect entities and support 1-hop traversal
- [ ] Markdown import produces entities with correct components
- [ ] YAML frontmatter is extracted (title, tags, date)
- [ ] Cross-references between Markdown files produce relationships
- [ ] Entity resolution detects duplicates at import time
- [ ] Full-text search returns ranked results via pluggable `SearchIndex` trait
- [ ] Search supports filtering by entity type and tags
- [ ] Search index is rebuildable from canonical data
- [ ] CLI commands work as specified
- [ ] Pipeline is idempotent (reimporting same file produces same result)
- [ ] Durable event log records all canonical changes
- [ ] All tests pass
- [ ] No canonical data loss on any failure path

### Test Cases

1. **Import single Markdown file** — Creates entity with correct components
2. **Import directory of Markdown files** — Creates one entity per file
3. **Import with YAML frontmatter** — Extracts title, tags, date
4. **Import without frontmatter** — Uses filename as title, modification date
5. **Reimport same file** — Idempotent, entity version increments
6. **Search by keyword** — Returns matching entities
7. **Search with type filter** — Returns only matching types
8. **Search with tag filter** — Returns only matching tags
9. **Get entity details** — Shows all components and relationships
10. **Archive entity** — Entity marked inactive, removed from search
11. **Restore entity** — Entity marked active, reappears in search
12. **Cross-reference extraction** — Creates `references` relationships
13. **Duplicate detection** — Same title + type triggers resolution
14. **Fuzzy duplicate detection** — Similar titles flagged as candidates
15. **Search index rebuild** — Drop index, rebuild from canonical, search still works
16. **Event emission** — EntityCreated event emitted on import

---

## Testing Strategy

Tests are written alongside each phase of implementation, not deferred to the end. Every feature has corresponding tests before moving to the next phase.

### Test Levels

| Level             | Scope                                                     | Framework                       |
| ----------------- | --------------------------------------------------------- | ------------------------------- |
| Unit tests        | Domain model, component operations, resolution strategies | `#[cfg(test)]` modules          |
| Integration tests | Storage adapter, search index, import pipeline            | `tests/` directories            |
| End-to-end tests  | CLI commands, full import→search flow                     | `tests/` with process execution |

### Test Coverage by Phase

| Phase                      | Test Focus                                                                                       |
| -------------------------- | ------------------------------------------------------------------------------------------------ |
| Phase 1: Core entity model | Entity CRUD, version incrementing, archive/restore, component lifecycle, relationship operations |
| Phase 2: Storage adapters  | SQLite CRUD for all repository types, schema migrations, error handling                          |
| Phase 3: Search interface  | FTS5 indexing, query parsing, type/tag filtering, rebuild from canonical                         |
| Phase 4: Entity resolution | Exact match, fuzzy match, confidence scoring, merge undo                                         |
| Phase 5: Import pipeline   | Markdown parsing, frontmatter extraction, cross-references, idempotent reimport                  |
| Phase 6: CLI               | Command parsing, output formatting, error messages                                               |
| Phase 7: Event system      | Event emission, idempotent processing, replay                                                    |

### Test Data

Sample Markdown files with varying complexity:
- Simple file without frontmatter
- File with full YAML frontmatter (title, tags, date, author)
- File with cross-references to other files
- Directory of files with inter-linking
- Edge cases: empty files, very large files, non-UTF-8 content

---

## Current State

The project has the core entity model and import pipeline implemented:

| Crate               | Status                                                                                                                                              |
| ------------------- | --------------------------------------------------------------------------------------------------------------------------------------------------- |
| `knowledge-core`    | Complete domain model: Entity, Component, Relationship types; ports with SearchIndex + EventLog traits                                              |
| `knowledge-storage` | Full SQLite adapter: CRUD for entities, components, relationships; FTS5 search with BM25 ranking; event log; SearchIndex + EventLog implementations |
| `knowledge-import`  | Markdown importer with YAML frontmatter, cross-ref extraction, Timeline + Language components                                                       |
| `knowledge-derive`  | Stub (not PRD-0001 scope)                                                                                                                           |
| `knowledge-api`     | REST API with basic CRUD (not PRD-0001 scope)                                                                                                       |
| `knowledge-cli`     | `kos` binary: import, search (with BM25 ranking), get, list, archive, restore, rebuild-index                                                        |

### Implemented Features

- Entity CRUD (create, read, update, archive, restore)
- Component lifecycle with version tracking and `update_data()`
- Relationship storage with 1-hop traversal and type filtering
- Markdown import with YAML frontmatter extraction
- Title extraction: frontmatter → H1 → filename fallback
- Tags extraction from YAML frontmatter
- Timeline component (file mod date + import time)
- Language component (frontmatter `language` field, defaults to `en`)
- Provenance component (source path + import timestamp)
- Cross-reference extraction from Markdown links
- Cross-reference relationship creation (file path matching)
- Idempotent reimport (exact match: title + entity type)
- Full-text search via pluggable `SearchIndex` trait
- FTS5 BM25 ranking
- Search filtering by entity type and tags
- Search index rebuild from canonical data
- Durable event log (EntityCreated, EntityUpdated, EntityArchived, etc.)
- Event emission on all canonical changes
- CLI: `kos import`, `kos search`, `kos get`, `kos list`, `kos archive`, `kos restore`, `kos rebuild-index`

### Not Yet Implemented

- Fuzzy entity resolution (P1, deferred)
- Confidence scoring for resolution (P1, deferred)
- Batch import progress reporting (P1, deferred)
- Search result snippets (P1, deferred)
- Version history table (P1, deferred)

---

## Dependencies

### External Crates

| Crate                  | Purpose              | Justification                                      |
| ---------------------- | -------------------- | -------------------------------------------------- |
| `rusqlite`             | SQLite bindings      | Storage adapter for canonical data and FTS5 search |
| `pulldown-cmark`       | Markdown parsing     | Parse Markdown to AST for import pipeline          |
| `serde` / `serde_yaml` | Serialization        | YAML frontmatter parsing                           |
| `uuid`                 | Entity identifiers   | Stable, unique, immutable entity IDs               |
| `chrono`               | Date/time            | Timeline component                                 |
| `tokio`                | Async runtime        | Event processing, pipeline                         |
| `clap`                 | CLI argument parsing | `kos` command-line interface                       |
| `indicatif`            | Progress bars        | Import progress reporting                          |
| `thiserror`            | Error types          | Structured error handling                          |
| `async-trait`          | Async traits         | Port/adapter interfaces                            |

### Future Search Adapters (Not in PRD-0001)

| Crate           | Purpose            | When                            |
| --------------- | ------------------ | ------------------------------- |
| `tantivy`       | Full-text search   | PRD-0002 (if FTS5 insufficient) |
| Quickwit client | Distributed search | PRD-0003 (at scale)             |

### Internal Dependencies

- `docs/architecture/domain-model.md` — Entity, component, relationship types
- `docs/architecture/pipeline.md` — Seven-layer architecture
- `docs/architecture/data-model.md` — Canonical vs derived distinction
- `docs/architecture/events.md` — Event system design
- `docs/architecture/storage.md` — Storage adapter pattern
- `docs/architecture/composition.md` — Entity component model

---

## Risks and Mitigations

| Risk                                | Impact | Likelihood | Mitigation                                                        |
| ----------------------------------- | ------ | ---------- | ----------------------------------------------------------------- |
| SQLite performance at 100K entities | High   | Low        | Benchmark early; adapter pattern allows PostgreSQL swap           |
| FTS5 relevance quality              | Medium | Medium     | Start with basic ranking; pluggable interface allows Tantivy swap |
| Markdown parsing edge cases         | Medium | High       | Use battle-tested `pulldown-cmark`; extensive test suite          |
| Cross-reference extraction accuracy | Medium | High       | Start with explicit Markdown links; AI extraction deferred        |
| Entity resolution false positives   | High   | Medium     | Confidence scoring, auditable merge decisions, undo capability    |
| Event ordering guarantees           | High   | Low        | Use SQLite transactions for canonical writes + event emission     |

---

## Timeline

| Phase                          | Duration | Deliverables                                                                       |
| ------------------------------ | -------- | ---------------------------------------------------------------------------------- |
| Phase 1: Core entity model     | 1 week   | Complete domain model, ports, events, entity resolution interface                  |
| Phase 2: Storage adapters      | 1 week   | Full SQLite adapter, component/relationship/event storage                          |
| Phase 3: Search interface      | 3 days   | Pluggable SearchIndex trait, SQLite FTS5 adapter                                   |
| Phase 4: Entity resolution     | 3 days   | Resolution strategies, import-time dedup, confidence scoring                       |
| Phase 5: Import pipeline       | 1 week   | Markdown parsing, frontmatter extraction, cross-references, resolution integration |
| Phase 6: CLI                   | 3 days   | `kos` binary with all commands                                                     |
| Phase 7: Event system          | 2 days   | Durable event log, event processing, search index updates                          |
| Phase 8: Integration + testing | 3 days   | End-to-end tests, performance benchmarks                                           |

**Total: ~5 weeks**

---

## Further Reading

- [Product Requirements](product-requirements.md) — Year 1 functional and non-functional requirements
- [Domain Model](../architecture/domain-model.md) — Entity, relationship, and component types
- [Pipeline](../architecture/pipeline.md) — Seven-layer architecture
- [Data Model](../architecture/data-model.md) — Canonical vs derived data
- [Storage](../architecture/storage.md) — Storage adapter pattern
- [Events](../architecture/events.md) — Event-driven architecture
- [ADR-0001](../architecture/adrs/adr-0001.md) — Knowledge Model as Canonical Source of Truth
- [ADR-0002](../architecture/adrs/adr-0002.md) — Storage Independence via Adapter Pattern
- [ADR-0003](../architecture/adrs/adr-0003.md) — Entity Component Model
- [ADR-0004](../architecture/adrs/adr-0004.md) — Event-Driven Derivation Pipeline
- [ADR-0005](../architecture/adrs/adr-0005.md) — Compiler-Inspired Architecture
- [ADR-0006](../architecture/adrs/adr-0006.md) — Entity Resolution as Critical Layer
- [Landscape 2026](../research/landscape-2026.md) — 2026 knowledge management landscape
- [Open Infrastructure](../philosophy/open-infrastructure.md) — Knowledge as open infrastructure
