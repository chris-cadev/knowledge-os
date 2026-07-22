# Scalability

> The architecture scales horizontally through the pipeline and vertically through storage adapters. Every layer scales independently.

---

## Scalability Model

Knowledge OS scales in three dimensions:

1. **Data volume.** The system handles millions of entities and billions of relationships.
2. **Throughput.** The system processes thousands of imports per minute.
3. **Concurrency.** The system supports multiple simultaneous users and AI agents.

---

## Pipeline Scalability

The seven-layer pipeline scales horizontally. Each layer processes work independently through events.

### Layer-Level Scaling

| Layer | Scaling Strategy | Parallelism |
|-------|-----------------|-------------|
| Import | Multiple importers run concurrently | Per-source parallelism |
| Parsing | Parsers run concurrently per document | Per-document parallelism |
| Normalization | Normalizers run concurrently per entity | Per-entity parallelism |
| Knowledge Model | Entity operations are independent | Per-entity parallelism |
| Relationship Engine | Relationship extraction runs concurrently | Per-source parallelism |
| Derivation | Each derivation type runs independently | Per-derivation-type parallelism |
| Presentation | Each view renders independently | Per-view parallelism |

### Event-Driven Scaling

Events enable natural parallelism. When canonical data changes, multiple derivation handlers process the event independently:

```
EntityUpdated
  +---> SearchIndexHandler     (concurrent)
  +---> EmbeddingHandler       (concurrent)
  +---> GraphProjectionHandler (concurrent)
  +---> CacheHandler           (concurrent)
```

Each handler is independent. Each handler scales independently. The system adds capacity by adding handlers.

---

## Storage Scalability

Each storage engine scales according to its own characteristics.

### Relational Storage

- **Vertical scaling.** Add CPU, memory, and disk to the database server.
- **Read replicas.** Replicate the database for read-heavy workloads.
- **Partitioning.** Partition by workspace, entity type, or time range.
- **Connection pooling.** Manage concurrent connections efficiently.

### Object Storage

- **Horizontal scaling.** Object storage (S3, MinIO) scales horizontally by design.
- **Content addressing.** SHA-256 hashes as keys enable deduplication.
- **No query overhead.** Object storage has no query engine. Access is by key only.

### Search Storage

- **Index sharding.** Split the search index across multiple shards.
- **Replication.** Replicate shards for read throughput.
- **Rebuild from canonical.** If capacity is exceeded, rebuild the index from canonical data.

### Vector Storage

- **Horizontal scaling.** Vector databases (Qdrant, Milvus) scale horizontally.
- **Index partitioning.** Partition vectors by entity type or workspace.
- **ANN optimization.** Approximate nearest neighbor algorithms provide sub-linear search.

### Cache Storage

- **Horizontal scaling.** Redis Cluster scales horizontally.
- **Eviction policies.** TTL-based eviction manages memory pressure.
- **Rebuild from canonical.** If capacity is exceeded, caches are cleared and rebuilt.

---

## Concurrency Model

The system supports concurrent operations through isolation and idempotency.

### Entity-Level Concurrency

Each entity is an independent unit. Operations on different entities do not conflict. Operations on the same entity are serialized through version numbers:

```
Entity A: Version 1 --> Version 2 --> Version 3
Entity B: Version 1 --> Version 2
```

### Pipeline-Level Concurrency

Pipeline layers process events concurrently. Each event is processed independently. Idempotency ensures that processing the same event twice produces the same result.

### Storage-Level Concurrency

Storage engines manage concurrency through their own mechanisms:

- **Relational:** Transactions and row-level locking.
- **Object:** Immutable objects. No write conflicts.
- **Search:** Atomic index updates.
- **Vector:** Atomic vector insert/delete.
- **Cache:** Atomic key-value operations.

---

## Capacity Planning

### Entity Volume

| Scale | Entities | Storage Estimate |
|-------|----------|-----------------|
| Small | 1K - 10K | < 1 GB |
| Medium | 10K - 100K | 1 - 10 GB |
| Large | 100K - 1M | 10 - 100 GB |
| Enterprise | 1M - 10M | 100 GB - 1 TB |

### Throughput

| Operation | Single-Node Capacity |
|-----------|---------------------|
| Import | 100-500 documents/minute |
| Search | 1000-5000 queries/second |
| Embedding | 100-500 vectors/second |
| Graph traversal | 1000-10000 traversals/second |

### Bottlenecks

The primary bottlenecks are:

1. **AI operations.** Embedding generation and AI inference are the slowest operations. They bound the derivation pipeline throughput.
2. **Storage I/O.** Large canonical datasets stress relational and object storage.
3. **Network latency.** Remote storage engines (S3, PostgreSQL, Qdrant) introduce network overhead.

---

## Scaling Strategies

### Vertical Scaling

The simplest strategy. Add CPU, memory, and disk to a single node. Works for small to medium deployments.

### Horizontal Scaling

For larger deployments:

- **Multiple pipeline workers.** Run multiple instances of the pipeline, each processing a subset of events.
- **Database read replicas.** Distribute read queries across replicas.
- **Search index sharding.** Distribute the search index across multiple shards.
- **Vector index partitioning.** Distribute vectors across multiple instances.

### Functional Scaling

Separate deployment by function:

- **Import service.** Dedicated to import and parsing.
- **Canonical service.** Dedicated to entity and relationship management.
- **Derivation service.** Dedicated to search indexing, embedding, and caching.
- **API service.** Dedicated to query and rendering.

Each service scales independently based on its own load characteristics.

---

## Further Reading

- [Pipeline](pipeline.md) -- How the pipeline processes work
- [Storage](storage.md) -- How storage engines scale
- [Events](events.md) -- How events enable parallel processing
- [Composition](composition.md) -- How entities are independent units
