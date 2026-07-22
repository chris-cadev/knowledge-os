# Storage Philosophy

> No single database satisfies every access pattern. The architecture uses specialized storage engines. Storage engines optimize access. They do not define meaning.

---

## Overview

Knowledge OS uses a polyglot persistence strategy. Different storage engines are chosen for their strengths in specific access patterns. No single engine serves all purposes. The application owns the knowledge model. Storage engines are interchangeable adapters.

This approach follows Martin Fowler's [Polyglot Persistence](https://martinfowler.com/bliki/PolyglotPersistence.html) philosophy, applied within a single application rather than across enterprise services.

---

## Storage Engines

### Object Storage

**Purpose:** Large binary artifacts.

**Stores:** Images, video, audio, Markdown files, PDFs, source code, archives.

**Properties:**
- Immutable. Objects are referenced, not modified.
- Content-addressed when possible (SHA-256 hashes as keys).
- Scalable to petabytes.
- No query capability. Access by key only.

**Candidates:** MinIO, S3, local filesystem.

---

### Relational Storage

**Purpose:** Canonical structured data.

**Stores:** Identifiers, metadata, ownership, transactions, permissions, schemas, entity-component relationships, configuration.

**Properties:**
- Transactional backbone of the system.
- Strong consistency guarantees.
- ACID compliance for entity lifecycle operations.
- Normalized schema for canonical data.

**Candidates:** PostgreSQL, SQLite, DuckDB.

---

### Graph Storage

**Purpose:** Relationship traversal.

**Supports:** Multi-hop exploration, neighborhood discovery, dependency analysis, semantic navigation.

**Properties:**
- Optimized for traversals (2-4 hops).
- Stores one projection of canonical relationships.
- May be rebuilt from relational canonical data.
- Not the source of truth for relationships.

**Candidates:** Neo4j, Memgraph, or graph extensions on relational databases (e.g., Apache AGE).

---

### Search Storage

**Purpose:** Full-text retrieval.

**Indexes:** Titles, descriptions, Markdown content, OCR text, captions, transcripts.

**Properties:**
- Inverted index for fast text search.
- Supports faceted search, highlighting, and relevance ranking.
- Regenerated whenever canonical content changes.
- Disposable -- rebuildable from canonical data.

**Candidates:** Tantivy (Rust-native), Elasticsearch, OpenSearch, Meilisearch.

---

### Vector Storage

**Purpose:** Semantic retrieval.

**Stores:** Embeddings, similarity indexes, AI retrieval metadata.

**Properties:**
- Approximate nearest neighbor (ANN) search.
- Optimized for cosine similarity and dot product queries.
- Embeddings never replace canonical knowledge.
- Regenerated when embedding models change.

**Candidates:** Qdrant, Milvus, pgvector, Chroma.

---

### Cache Storage

**Purpose:** Performance optimization.

**Stores:** Temporary results, frequently accessed data, computed projections.

**Properties:**
- In-memory or near-memory.
- TTL-based expiration.
- Contains no canonical information.
- May be cleared entirely without data loss.

**Candidates:** Redis, in-process caches, memory-mapped files.

---

## Adapter Pattern

Each storage engine is accessed through an adapter. Adapters implement a common interface. The domain model never depends on a specific storage technology.

```
Domain Model
     |
  Adapter Interface
     |
  +----------+----------+----------+----------+
  | Object   | Relational| Graph    | Search   | ...
  | Storage  | Storage   | Storage  | Storage  |
  +----------+----------+----------+----------+
```

Replacing a storage engine requires:

1. Implementing a new adapter.
2. Migrating data (if canonical).
3. Rebuilding derived data (indexes, embeddings, graphs).

The domain model, the pipeline, and all business logic remain unchanged.

---

## Canonical vs Derived Storage

| Data Category | Storage | Canonical? | Rebuildable? |
|---------------|---------|------------|--------------|
| Entities | Relational | Yes | No |
| Components | Relational + Object | Yes | No |
| Relationships | Graph + Relational | Yes | Partially |
| Artifacts | Object | Yes | No |
| Search indexes | Search engine | No | Yes |
| Embeddings | Vector store | No | Yes |
| Similarity graphs | Graph store | No | Yes |
| Caches | Cache store | No | Yes |

---

## Consistency Model

The system uses an eventual consistency model for derived data:

1. Canonical data changes are committed atomically.
2. Events are published for derived data generation.
3. Derived data is updated asynchronously.
4. Views may briefly show stale data until derived data catches up.

Canonical data uses strong consistency within a single storage engine. Cross-engine consistency is managed through events and idempotent processing.

---

## Failure Recovery

If a storage engine fails:

- **Object storage:** Objects are immutable. Recovery restores from backup.
- **Relational storage:** WAL-based recovery. Canonical data is protected by transactions.
- **Graph storage:** Rebuild from relational canonical data.
- **Search storage:** Rebuild from canonical content.
- **Vector storage:** Recompute embeddings from canonical text.
- **Cache storage:** Populate on demand. No recovery needed.

---

## Implementation Candidates (Rust Ecosystem)

Given the project's likely Rust implementation:

| Engine | Type | Rust Binding |
|--------|------|-------------|
| SQLite | Relational | `rusqlite` |
| PostgreSQL | Relational | `sqlx`, `tokio-postgres` |
| DuckDB | Relational (analytical) | `duckdb-rs` |
| Tantivy | Search | `tantivy` (native Rust) |
| Qdrant | Vector | `qdrant-client` |
| Redis | Cache | `redis-rs` |
| S3/MinIO | Object | `aws-sdk-s3` |

---

## Further Reading

- [Overview](overview.md) -- System-level architecture
- [Data Model](data-model.md) -- Canonical vs derived distinction
- [Pipeline](pipeline.md) -- How storage fits in the pipeline
- [Composition](composition.md) -- How entities map to storage
