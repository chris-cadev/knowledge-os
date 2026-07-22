# Philosophy

> Knowledge OS is a knowledge engine, not a database.

---

## Core Belief

The Knowledge Operating System exists because current knowledge management systems conflate storage with meaning. A database stores records. A knowledge engine understands entities, relationships, and context. This distinction is the foundation of every architectural decision in the system.

The problem is not that we lack storage. The problem is that we lack meaning. A file system knows that a file exists. It does not know what the file is about. A database knows that a record exists. It does not know what the record represents. Knowledge requires entities, relationships, components, and context. Knowledge requires a model that represents reality, not just storage locations.

---

## Immutable Principles

These principles never change. Every future decision is validated against them.

### 1. The Canonical Model Is the Source of Truth

The knowledge model represents reality independently of storage technology. Every entity, relationship, component, artifact, and capability is represented within the canonical model. All other representations are derived. Derived data may be regenerated at any time.

This principle is borrowed from the [Canonical Data Model](https://www.enterpriseintegrationpatterns.com/patterns/messaging/CanonicalDataModel.html) pattern described by Hohpe and Woolf, applied at the application level rather than the integration level.

The canonical model is the single source of truth because it is the only representation that cannot be regenerated. If the search index is lost, rebuild it. If the embedding store is lost, recompute it. If the canonical model is lost, knowledge is lost. The canonical model is what the system protects.

### 2. Storage Is an Implementation Detail

No storage engine defines the knowledge model. The application owns the knowledge model. Storage technologies exist to optimize specific access patterns. Replacing one implementation never changes the architecture.

This follows Martin Fowler's [Polyglot Persistence](https://martinfowler.com/bliki/PolyglotPersistence.html) philosophy: choose the right storage technology for each access pattern, never the other way around.

Storage engines are adapters. They implement a common interface. The domain model depends on the interface, never on the implementation. SQLite may be replaced by PostgreSQL. Tantivy may be replaced by Elasticsearch. Qdrant may be replaced by Milvus. The domain model, the pipeline, and all business logic remain unchanged.

### 3. Every Derived Artifact Is Disposable

Search indexes, embeddings, recommendations, similarity graphs, caches -- all may be discarded and reconstructed from canonical data. No derived representation becomes the source of truth.

This mirrors the insight from event sourcing and CQRS: read models are projections of an append-only event log, and can always be rebuilt.

Derived data is disposable because it is reproducible. If a derived artifact contains information that cannot be recovered from canonical sources, it is misclassified. Reclassify it as canonical.

### 4. Composition Over Inheritance

Entities acquire behavior through components, not class hierarchies. Every entity is assembled from reusable, interchangeable parts. This follows the [Entity Component System](https://en.wikipedia.org/wiki/Entity_component_system) pattern, adapted from game engine architecture to knowledge management.

Inheritance creates rigid hierarchies. Composition creates flexible assemblies. A person entity and a paper entity may share a Title component without sharing a base class. A tool entity and a concept entity may share a Tags component without an inheritance relationship. The component model eliminates the diamond problem, the fragile base class problem, and the code duplication problem.

### 5. AI Is an Engine, Not the Source of Truth

Artificial intelligence performs extraction, classification, summarization, relationship inference, and planning. Every AI output is reviewable, versioned, and replaceable. AI assists in knowledge construction but never defines it.

As noted in recent research on [epistemic infrastructure for organizational AI](https://arxiv.org/html/2604.11759v1), the ceiling on organizational AI is not retrieval fidelity but epistemic fidelity -- the system's ability to represent commitment strength, contradiction status, and organizational ignorance as computable properties.

AI outputs are derived data. They are probabilistic, not deterministic. They must be reviewed, versioned, and replaceable. The system never treats AI output as canonical without human approval.

### 6. Every Interface Is a Projection

Views never own information. Views render knowledge. Every entity may appear in multiple projections simultaneously. Views remain synchronized because they render canonical data.

A single entity may appear as a node in a graph, a row in a table, a card in a kanban, a result in search, and a context element in an AI conversation. The entity is not duplicated. The canonical model is rendered differently in each projection.

This principle eliminates data duplication, ensures consistency, and enables new interfaces without data migration.

---

## Design Values

**Determinism.** The pipeline produces identical outputs from identical inputs. Every transformation is reproducible. There is no randomness, no non-deterministic side effects, and no hidden state that affects the pipeline. Determinism enables reproducibility, testing, debugging, and recovery.

**Transparency.** Every AI output is explainable. Every derived artifact traces back to canonical sources. Every relationship is explicit. The system never hides what it knows or how it knows it. Transparency builds trust.

**Independence.** The architecture never depends on a specific database, AI model, search engine, or cloud provider. Adapters isolate implementation details. The domain model remains independent. Independence enables replacement, migration, and evolution without system-wide changes.

**Extensibility.** Every subsystem supports extension through plugins. Plugins extend capabilities without modifying the core. The core system is stable. The ecosystem provides variety. Extensibility enables growth without fragmentation.

**Durability.** Canonical data is versioned, auditable, and persistent. The knowledge model outlives any individual technology choice. Durability ensures that knowledge persists across years, across organizations, and across technological generations.

---

## Engineering Values

**Architecture before implementation.** The complete architectural foundation is written before any code is written. Architecture is not an afterthought. Architecture is the foundation. Every implementation decision references the architecture.

**Features answer questions.** Every feature must answer ten engineering questions before implementation begins. See [CONTRIBUTING.md](../../CONTRIBUTING.md) for the complete list. Features that fail these questions are redesigned.

**Derived data is reproducible.** If a derived artifact cannot be regenerated, it is not derived -- it is canonical, and must be treated accordingly. The system enforces this distinction at every layer.

**Storage layers are replaceable.** Replacing a storage engine must require zero changes to the domain model. The adapter pattern guarantees this. Storage engines are interchangeable. The knowledge model is permanent.

---

## User Values

**Progressive disclosure.** Simple tasks have simple interfaces. Complex capabilities are available but not forced. The default view is always the simplest useful view. Deeper levels of detail are available on demand.

**Universal navigation.** Every entity is reachable from every other entity through explicit relationships. There are no dead ends. There are no orphaned views. The knowledge graph is connected.

**Multiple projections.** The same knowledge is available as a tree, graph, timeline, table, conversation, or any future view type. The user chooses the projection that suits their task. The canonical model remains the same.

---

## Long-Term Principles

**The philosophy remains stable.** Implementation evolves. Architecture evolves. Technology evolves. The philosophy of the Knowledge Operating System remains stable. The principles defined in this document are the foundation upon which all future decisions are made.

**The manifesto is the constitution.** Every future decision made in the project is validated against the seed manifesto. If a proposed feature contradicts the philosophy, the feature is redesigned before implementation. The manifesto is never edited. It is superseded by a new version when the philosophy evolves.

**Knowledge outlives technology.** The canonical knowledge model is designed to persist across decades. Storage engines change. AI models change. User interfaces change. The knowledge model remains. This long-term perspective shapes every architectural decision.

**The ecosystem is the product.** The core engine is necessary but insufficient. The value of Knowledge OS grows with the number of plugins, importers, storage adapters, and integrations. The ecosystem is the product.

---

## Anti-Goals

These are things we explicitly do not build:

- **A database with a knowledge label.** We do not wrap a database and call it a knowledge engine. The knowledge model is owned by the application, not the storage layer. A database stores records. A knowledge engine understands entities, relationships, and context.

- **An AI wrapper.** We do not build a UI around a large language model and call it knowledge management. AI is a component, not the system. The system's value is the knowledge model, not the AI model.

- **A document manager.** We do not organize files in folders and call it knowledge. Documents are inputs. Entities are the output. The document is the raw material. The entity is the refined product.

- **A search engine with ambitions.** We do not build a search system and claim it understands knowledge. Search is one projection of many. Search retrieves text. Knowledge retrieval surfaces entities.

- **A monolithic application.** We do not build a single binary that cannot be extended. Every subsystem supports plugins. The core is stable. The ecosystem provides variety.

- **A cloud service.** We do not operate infrastructure for users. Knowledge OS is an engine that can be deployed anywhere. Users own their data. Users own their infrastructure.

- **A collaboration platform.** We do not build real-time collaborative editing, chat, or video conferencing. Knowledge OS is a knowledge engine. Collaboration tools may consume its API, but collaboration is not our domain.

---

## References

- Hohpe, G. & Woolf, B. *Enterprise Integration Patterns* (2003) -- Canonical Data Model pattern
- Fowler, M. *Patterns of Enterprise Application Architecture* (2002) -- Layering, Domain Model
- Fowler, M. "Polyglot Persistence" (2011) -- Multiple storage technologies
- Entity Component System -- Wikipedia: [Entity component system](https://en.wikipedia.org/wiki/Entity_component_system)
- OIDA Framework -- "Retrieval Is Not Enough: Why Organizational AI Needs Epistemic Infrastructure" (2026)
- Knowledge as Code pattern -- knowledge-as-code.com (2026)
