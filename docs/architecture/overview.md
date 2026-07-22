# System Overview

> The Knowledge Operating System is engineered as a deterministic knowledge compiler.

---

## What It Is

Knowledge OS is a system for ingesting, normalizing, and reasoning over heterogeneous knowledge. It is not a database. It is not a search engine. It is not an AI product. It is a knowledge engine -- a system that transforms raw information into a structured, queryable, versioned knowledge graph.

The core insight is borrowed from compiler architecture: information enters the system as source material, is parsed and normalized into a canonical representation, and is then compiled into derived artifacts optimized for specific access patterns. The canonical model is the executable representation. Everything else is a projection.

---

## The Problem

Current knowledge management systems suffer from a fundamental architectural flaw: they conflate storage with meaning. A file system organizes documents by name and directory. A database organizes records by table and column. Neither understands that a research paper, a person, an organization, and a concept are entities with relationships, components, and versions.

Search does not solve this problem. Search retrieves documents that match a query. It does not understand that a query about "machine learning" should surface papers, people, tools, datasets, and concepts -- all connected through explicit relationships.

AI does not solve this problem either. AI can extract, classify, and summarize, but it does not own the knowledge model. AI outputs are probabilistic, not deterministic. They must be reviewed, versioned, and replaceable.

---

## The Architecture

The system follows a seven-layer pipeline:

```
  Import Layer          Receive information from any external system
       |
  Parsing Layer         Extract structured information
       |
  Normalization Layer   Convert to canonical representations
       |
  Knowledge Model       Everything becomes a first-class entity
       |
  Relationship Engine   Connect entities through typed, versioned edges
       |
  Derivation Layer      Generate search indexes, embeddings, recommendations
       |
  Presentation Layer    Render projections: tree, graph, timeline, table
```

Each layer has one responsibility. Each layer communicates through explicit contracts. No layer bypasses another.

See [Pipeline](pipeline.md) for a detailed description of each layer.

---

## Canonical vs Derived

The system distinguishes between two categories of data:

**Canonical data** cannot be recreated. It represents user knowledge. It requires durability, versioning, and auditing. It is the source of truth.

**Derived data** may be recreated. It optimizes performance, search, AI, and navigation. It is never authoritative. It is always disposable.

This distinction is the most important architectural decision in the system. See [Data Model](data-model.md) for a full explanation.

---

## Storage Independence

The architecture never depends on a specific storage technology. The system uses specialized storage engines:

- **Object storage** for large binary artifacts
- **Relational storage** for canonical structured data
- **Graph storage** for relationship traversal
- **Search storage** for full-text retrieval
- **Vector storage** for semantic retrieval
- **Cache storage** for performance optimization

Any engine may be replaced without changing the domain model. See [Storage](storage.md) for details.

---

## Event-Driven Processing

Every meaningful modification generates events. Events trigger asynchronous processing through the pipeline. Each stage is isolated and idempotent.

See [Events](events.md) for the event system design.

---

## Compiler Perspective

The system is modeled as a deterministic knowledge compiler:

```
External Resource
       |
    Importer
       |
    Parser
       |
    Normalizer
       |
  Canonical Entity Model
       |
  Relationship Extraction
       |
  Knowledge Engine
       |
  Derived Representations
       |
    Views
```

Every transformation has one responsibility. Every stage may be independently replaced. No stage bypasses another.

See [Compilation](compilation.md) for the full compiler analogy.

---

## Extensibility

Every subsystem supports extension through plugins:

- Importers and exporters
- Views and projections
- Storage adapters
- Relationship providers
- Search providers
- Embedding providers
- AI providers
- Automation agents

Plugins extend capabilities without modifying the core.

---

## Further Reading

- [Pipeline](pipeline.md) -- The seven-layer architecture in detail
- [Data Model](data-model.md) -- Canonical vs derived data
- [Storage](storage.md) -- Polyglot persistence strategy
- [Composition](composition.md) -- Entity component model
- [Compilation](compilation.md) -- Compiler perspective
- [Events](events.md) -- Event-driven architecture
- [Philosophy](../philosophy/philosophy.md) -- Core principles
