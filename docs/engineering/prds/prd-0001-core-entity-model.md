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
- Full-text search index (derived, disposable)
- CLI interface for import and query

### Out of Scope

- PDF import (complex parser, deferred to PRD-0002)
- Graph view rendering (UI, deferred)
- Embeddings and semantic search (derived, deferred)
- Plugin system (designed alongside, not critical for first working system)
- Multi-user collaboration (Year 3)
- AI-assisted extraction (Year 2)

---

## Engineering Questions

Every feature proposed for the system must answer these 10 questions per `CONTRIBUTING.md`:

### 1. Which canonical entities are introduced?

No new entity types are introduced. The system uses the core entity types defined in `docs/architecture/domain-model.md`: Concept, Person, Organization, Project, Book, Paper, Video, Article, Tool, Technology, Question, Idea, Event, Skill, Location, Dataset, Collection, Workspace, Decision, Note.

The Markdown importer produces entities with type determined by content analysis or user specification. Default type is `Article` for Markdown files.

### 2. Which relationships are introduced?

No new relationship types are introduced. The system uses the core relationship types from `docs/architecture/domain-model.md`. The Markdown importer extracts explicit cross-references as `references` relationships between entities.

### 3. Which components are introduced?

No new component types are introduced. The system uses the core component types from `docs/architecture/domain-model.md`. The Markdown importer produces entities with these components:

| Component | Source |
|-----------|--------|
| `Title` | YAML frontmatter `title` field, or first H1 heading, or filename |
| `Content` | Full Markdown body |
| `Tags` | YAML frontmatter `tags` field |
| `Timeline` | File modification date for `created_at`, import time for `modified_at` |
| `Language` | Detected from content (defaults to `en`) |
| `Provenance` | Import source path, import timestamp, importer name |

### 4. Which events are emitted?

| Event | Trigger | Consumers |
|-------|---------|-----------|
| `EntityCreated` | New entity stored | Search index updater |
| `EntityUpdated` | Entity component changed | Search index updater |
| `EntityArchived` | Entity marked inactive | Search index updater |
| `RelationshipCreated` | New relationship stored | Search index updater |
| `RelationshipArchived` | Relationship marked inactive | Search index updater |

### 5. Which derived representations are generated?

| Derived Artifact | Source | Regeneration |
|-----------------|--------|-------------|
| Full-text search index | Entity Title + Content + Tags | Drop and rebuild from canonical |
| Entity type index | Entity type field | Rebuild from canonical |
| Tag index | Entity Tags component | Rebuild from canonical |

All derived data is disposable. The search index is rebuilt from canonical data at any time.

### 6. Which layer owns the feature?

| Feature | Layer |
|---------|-------|
| Entity storage | Layer 4 — Knowledge Model |
| Component storage | Layer 4 — Knowledge Model |
| Relationship storage | Layer 5 — Relationship Engine |
| Markdown import | Layer 1 (Import) + Layer 2 (Parsing) + Layer 3 (Normalization) |
| Search index | Layer 6 — Derivation |

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

| Step | Action | Why This Order |
|------|--------|----------------|
| **Import** | Read raw input from file system, API, or other source | Nothing exists until data enters the system |
| **Extract** | Transform raw input into structured data (entities, components) | Structured data must exist before we can resolve it |
| **Resolve** | Identify duplicates, assign canonical IDs, normalize metadata | Resolution must happen before storage — dirty data in = dirty graph forever |
| **Store** | Persist resolved entities to the canonical model | Storage happens after resolution ensures data quality |
| **Connect** | Establish relationships between stored entities | Connections require both source and target entities to exist |
| **Search** | Index stored entities for retrieval | Search indexes what is stored, not what is being processed |

### Why Resolution Before Storage

Entity resolution is the critical quality layer. The evidence from the 2026 landscape (see [Landscape 2026](../research/landscape-2026.md)) confirms that enterprise knowledge graphs fail when resolution is deferred:

- Children's Medical Center Dallas found 22% of records were duplicates before proper resolution.
- LLMs consistently produce duplicate entities in GraphRAG pipelines.
- "Extraction is the part that demos, resolution is the part that matters."

If resolution happens after storage, duplicates accumulate and downstream layers (search, relationships, AI) operate on noisy data. The graph degrades before correction is applied.

Resolution before storage means the canonical model is clean from the first import. Every entity in the graph has a verified canonical ID. Every downstream layer operates on resolved, deduplicated data.

### Alternative Spines Considered

| Spine | Description | Why Rejected |
|-------|-------------|-------------|
| Import → Store → Extract → Connect → Search | Store raw data first, process later | Raw data contaminates the canonical model |
| Import → Extract → Store → Connect → Resolve → Search | Resolve after storage and connections | Duplicates compound before correction; relationships are built on noisy data |
| Import → Extract → Connect → Store → Search | Connect before store | Requires entities to exist before storage, which is architecturally incorrect |
| Import → Resolve → Extract → Store → Connect → Search | Resolve before extraction | Resolution requires structured data; raw input cannot be resolved without extraction |
| Import → Extract → Resolve → Store → Connect → Search | **Chosen spine** | Resolution happens on extracted data before storage; canonical model is clean by construction |

### Resolution as Continuous Process

Entity resolution is not a one-time operation at import. It is continuous:

1. **At import time:** New input is checked against existing entities before storage.
2. **After storage:** Periodic resolution passes catch duplicates that were missed.
3. **At query time:** Results are presented with entity confidence scores.

This ensures quality is maintained by construction, not corrected after the fact.

---

## Functional Requirements

### F1: Entity Management

| ID | Requirement | Priority | Acceptance Criteria |
|----|-------------|----------|-------------------|
| F1.1 | Create entities with a type and components | P0 | Entity is created with UUID, type, and initial components |
| F1.2 | Update entity components | P0 | Component change increments entity version |
| F1.3 | Archive entities (soft delete) | P0 | Entity is marked inactive, history preserved |
| F1.4 | Restore archived entities | P1 | Entity is marked active again |
| F1.5 | Query entities by type | P0 | Returns all entities of a given type |
| F1.6 | Query entities by component | P1 | Returns entities with a specific component type |
| F1.7 | Query entities by tag | P1 | Returns entities with a specific tag |
| F1.8 | Version history for entities | P1 | Entity version history is queryable |

### F2: Relationship Management

| ID | Requirement | Priority | Acceptance Criteria |
|----|-------------|----------|-------------------|
| F2.1 | Create typed relationships between entities | P0 | Relationship is created with type, source, and target |
| F2.2 | Update relationship attributes | P1 | Relationship attributes are updated |
| F2.3 | Archive relationships | P0 | Relationship is marked inactive |
| F2.4 | Traverse relationships (1 hop) | P0 | From any entity, retrieve directly connected entities |
| F2.5 | Traverse relationships (N hops) | P1 | From any entity, retrieve entities within N hops |
| F2.6 | Query relationships by type | P0 | Returns all relationships of a given type |

### F3: Import Pipeline

| ID | Requirement | Priority | Acceptance Criteria |
|----|-------------|----------|-------------------|
| F3.1 | Import Markdown files | P0 | Markdown file produces Entity with Content component |
| F3.3 | Extract metadata from imports | P0 | Author, date, language are extracted from YAML frontmatter |
| F3.4 | Import from file system | P0 | Local files are importable |
| F3.6 | Batch import | P1 | Multiple files are importable in one operation |
| F3.7 | Import progress reporting | P1 | Import progress is visible to the user |
| F3.8 | Import error handling | P0 | Malformed input is logged, not silently swallowed |

### F4: Search and Retrieval

| ID | Requirement | Priority | Acceptance Criteria |
|----|-------------|----------|-------------------|
| F4.1 | Full-text search | P0 | Query matches against indexed text fields |
| F4.2 | Entity type filtering | P0 | Search results can be filtered by entity type |
| F4.3 | Tag filtering | P1 | Search results can be filtered by tags |
| F4.4 | Search result ranking | P0 | Results are ranked by relevance |

---

## Non-Functional Requirements

### NF1: Performance

| ID | Requirement | Target | Acceptable |
|----|-------------|--------|-----------|
| NF1.1 | Import throughput (Markdown) | 100 docs/sec | 50 docs/sec |
| NF1.3 | Search query latency | < 50ms | < 200ms |
| NF1.4 | Entity retrieval latency | < 10ms | < 50ms |
| NF1.6 | Entity creation latency | < 10ms | < 50ms |

### NF2: Scalability

| ID | Requirement | Target |
|----|-------------|--------|
| NF2.1 | Entity volume | 100K entities |
| NF2.2 | Relationship volume | 1M relationships |
| NF2.3 | Component volume | 10M components |
| NF2.4 | Concurrent users | 1 user (Year 1) |
| NF2.5 | Import batch size | 10K files |

### NF3: Reliability

| ID | Requirement | Target |
|----|-------------|--------|
| NF3.1 | Data durability | Zero canonical data loss |
| NF3.2 | Derived data rebuildability | All derived data rebuildable from canonical |
| NF3.3 | Event processing reliability | Zero event loss (durable event log) |
| NF3.4 | Pipeline idempotency | Reprocessing same input produces same output |

### NF4: Usability

| ID | Requirement | Acceptance Criteria |
|----|-------------|-------------------|
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
8. Entity is stored in the canonical model.
9. Search index is updated.

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

### Storage

| Store | Technology | Purpose |
|-------|-----------|---------|
| Canonical entities | SQLite | Entity, component, and relationship storage |
| Search index | Tantivy | Full-text search (derived, disposable) |
| Event log | SQLite | Durable event log for pipeline processing |

### Pipeline Flow

```
Markdown File
      |
  Import Layer         kos import command
      |
  Parsing Layer        YAML frontmatter extraction, Markdown parsing
      |
  Normalization Layer  Entity identification, metadata normalization
      |
  Knowledge Model      Entity + Components stored in SQLite
      |
  Relationship Engine  Cross-reference extraction, relationship storage
      |
  Derivation Layer     Search index update via Tantivy
      |
  (Presentation)       CLI output (no UI in this PRD)
```

### Data Flow

1. User invokes `kos import <file.md>`.
2. Import layer reads the file.
3. Parsing layer extracts YAML frontmatter and Markdown body.
4. Normalization layer creates an Entity with type, Title, Content, Tags, Timeline, Language, Provenance.
5. Knowledge model stores the entity in SQLite.
6. Relationship engine extracts cross-references and stores relationships.
7. Derivation layer updates the Tantivy search index.
8. CLI outputs the created entity.

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
- [ ] Full-text search returns ranked results
- [ ] Search supports filtering by entity type and tags
- [ ] All derived data (search index) is rebuildable from canonical
- [ ] CLI commands work as specified
- [ ] Pipeline is idempotent (reimporting same file produces same result)
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

---

## Dependencies

### External Crates

| Crate | Purpose | Justification |
|-------|---------|--------------|
| `rusqlite` | SQLite bindings | Storage adapter for canonical data |
| `tantivy` | Full-text search | Derived search index |
| `pulldown-cmark` | Markdown parsing | Parse Markdown to AST |
| `serde` / `serde_yaml` | Serialization | YAML frontmatter parsing |
| `uuid` | Entity identifiers | Stable, unique, immutable entity IDs |
| `chrono` | Date/time | Timeline component |
| `tokio` | Async runtime | Event processing, pipeline |
| `clap` | CLI argument parsing | Command-line interface |

### Internal Dependencies

- `docs/architecture/domain-model.md` — Entity, component, relationship types
- `docs/architecture/pipeline.md` — Seven-layer architecture
- `docs/architecture/data-model.md` — Canonical vs derived distinction
- `docs/architecture/events.md` — Event system design
- `docs/architecture/storage.md` — Storage adapter pattern

---

## Risks and Mitigations

| Risk | Impact | Likelihood | Mitigation |
|------|--------|-----------|------------|
| SQLite performance at 100K entities | High | Low | Benchmark early; adapter pattern allows PostgreSQL swap |
| Tantivy integration complexity | Medium | Medium | Start with simple indexing; iterate on query complexity |
| Markdown parsing edge cases | Medium | High | Use battle-tested `pulldown-cmark`; extensive test suite |
| Cross-reference extraction accuracy | Medium | High | Start with explicit Markdown links; AI extraction deferred |
| Event ordering guarantees | High | Low | Use SQLite事务 for canonical writes + event emission |

---

## Timeline

| Phase | Duration | Deliverables |
|-------|----------|-------------|
| Phase 1: Core entity model | 2 weeks | Entity CRUD, component storage, relationship storage |
| Phase 2: Markdown importer | 1 week | File parsing, metadata extraction, entity creation |
| Phase 3: Search index | 1 week | Tantivy integration, full-text search, filtering |
| Phase 4: CLI | 1 week | Import, search, get, list, archive commands |
| Phase 5: Integration + testing | 1 week | End-to-end tests, performance benchmarks |

**Total: 6 weeks**

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
- [ADR-0006](../architecture/adrs/adr-0006.md) — Entity Resolution as Critical Layer
- [Landscape 2026](../research/landscape-2026.md) — 2026 knowledge management landscape
- [Open Infrastructure](../philosophy/open-infrastructure.md) — Knowledge as open infrastructure
