# Glossary

> Every project term, defined once. Every document references this glossary. This is the canonical vocabulary of Knowledge OS.

---

## Purpose

This document defines the authoritative vocabulary of the Knowledge Operating System. Every term used across the project is defined here exactly once. All other documents reference this glossary rather than defining terms independently.

The vocabulary is organized by conceptual category, not alphabetically. Each definition includes its meaning within Knowledge OS, its properties, examples, and relationships to other terms.

Every term in this glossary is a first-class concept in the canonical model. Terms are not opinions. Terms are not suggestions. Terms are the language of the system.

---

## Core Concepts

These are the foundational atoms of the knowledge model. Every other concept depends on them.

---

### Entity

A named, typed, versioned object in the knowledge graph. Entities represent real-world concepts: people, organizations, papers, tools, ideas, events, decisions, locations, datasets, collections, and workspaces.

An entity is the atomic unit of knowledge in the system. Nothing in the knowledge model exists outside of entities and their relationships.

**Properties:**

- **Identifier.** A stable, unique, immutable identifier (UUID). Once assigned, it never changes, regardless of renames, merges, or storage migrations.
- **Type.** The category of knowledge the entity represents. Types include `Concept`, `Person`, `Organization`, `Project`, `Book`, `Paper`, `Video`, `Article`, `Tool`, `Technology`, `Question`, `Idea`, `Event`, `Skill`, `Location`, `Dataset`, `Collection`, `Workspace`, `Decision`, `Note`. The type is assigned at creation and never changes.
- **Components.** Zero or more typed data structures that describe aspects of the entity. Components are the building blocks of entities.
- **Version.** A monotonically increasing integer. Every modification increments the version.
- **Provenance.** The origin and history of the entity: when it was imported, from where, and by what process.
- **Status.** Active or archived. Entities are never hard-deleted. Archiving marks an entity as inactive while preserving its history.

**Rules:**

1. Every entity has exactly one type.
2. Entity types are extensible through configuration.
3. Entity types do not determine behavior. Behavior is determined by component assembly.
4. Entities are never hard-deleted. Archiving preserves the entity and its history.
5. Entity identifiers are stable across renames, merges, and storage migrations.

**See also:** [Component](#component), [Relationship](#relationship), [Domain Model](../architecture/domain-model.md)

---

### Component

A data structure that represents one aspect of an entity. Components are the building blocks of entities. Every component has a type that determines its data schema.

Components follow the Entity Component System (ECS) pattern adapted from game engine architecture. Instead of asking "what is this entity?", the system asks "what components does this entity have?".

**Properties:**

- **Type.** The kind of aspect it represents: `Title`, `Description`, `Content`, `BinaryContent`, `Author`, `Tags`, `Timeline`, `Location`, `Language`, `Rating`, `Embedding`, `Summary`, `Classification`, `Confidence`, `VersionHistory`, `Permissions`, `Provenance`, `Thumbnail`, `ExternalIdentifier`, `WorkspaceMembership`.
- **Data.** The typed payload specific to the component type. For example, a `Title` component contains `{ name: string }`. A `Content` component contains `{ markdown: string }`.
- **No identity.** Components are identified by their type within an entity, not by global identifiers. A `Title` component on entity A is a different component than a `Title` component on entity B.

**Rules:**

1. Single responsibility. Each component handles one aspect of the entity.
2. No dependencies. Components do not reference other components.
3. Data only. Components contain payloads, not behavior. Behavior lives in systems.
4. Composable. Any component type may be attached to any entity.
5. Optional. Entities may have any combination of components.
6. No duplicate types per entity. An entity may have at most one component of each type.
7. Versioned with their entity. When a component changes, the entity version increments.

**The same component type may appear on different entity types:**

```
Title component:
  On a Person entity    -->  { name: "Ada Lovelace" }
  On a Paper entity     -->  { name: "Attention Is All You Need" }
  On a Tool entity      -->  { name: "LLVM" }
  On a Concept entity   -->  { name: "Monads" }
```

**See also:** [Entity](#entity), [Composition](../architecture/composition.md), [Domain Model](../architecture/domain-model.md)

---

### Relationship

A typed, directed, attributed edge connecting two entities. Relationships are first-class citizens in the knowledge graph. They are not foreign keys. They are explicit, queryable, versionable objects.

Relationships describe how entities relate to each other: who created what, what references what, what depends on what, what contradicts what.

**Properties:**

- **Source entity.** The entity where the relationship originates.
- **Target entity.** The entity where the relationship points.
- **Type.** The semantic nature of the connection. Types include `created_by`, `authored_by`, `imported_from`, `derived_from`, `references`, `related_to`, `inspired_by`, `contradicts`, `supports`, `depends_on`, `implements`, `requires`, `extends`, `teaches`, `learned_from`, `applies`, `measures`, `contains`, `belongs_to`, `part_of`, `has_part`.
- **Attributes.** Metadata about the relationship: confidence score, source, timestamp.
- **Version.** Relationships are versioned independently of the entities they connect.

**Properties of relationships:**

- **Typed.** Every relationship has a semantic type. The type determines what the relationship means.
- **Directed.** Relationships have a source and a target. `A references B` is distinct from `B references A`.
- **Attributed.** Relationships carry metadata: when the relationship was established, what confidence the system has in it, and what source provided it.
- **Versioned.** Relationships track their history. A relationship that changes confidence over time retains its history.
- **Queryable.** Relationships support graph traversal: "find all papers that reference this concept," "find all tools used by this person," "find all entities that depend on this decision."

**Rules:**

1. Every relationship connects two existing entities. A relationship cannot reference a nonexistent entity.
2. Every relationship has exactly one type. The type never changes after creation.
3. Every relationship is directed. A relationship has a source and a target.
4. Relationships between two entities of the same type and direction are unique.
5. Relationship types are extensible through configuration.

**Relationship extraction mechanisms:**

- Import-time extraction. Explicit references (citations, links, authorship) are extracted when a document is imported.
- Normalization-time extraction. Implicit relationships (entity co-occurrence, conceptual similarity) are identified during normalization.
- AI-assisted extraction. AI models suggest relationships that are not explicitly stated. All AI-suggested relationships are flagged for review.
- Manual creation. Users create relationships directly through the interface.

**See also:** [Entity](#entity), [Knowledge Graph](#knowledge-graph), [Domain Model](../architecture/domain-model.md)

---

### Knowledge

Structured understanding derived from information. Knowledge differs from information in that it requires entities, relationships, components, context, and provenance.

A database stores information. A spreadsheet stores information. A file system stores information. None of them store knowledge.

Knowledge requires:

- **Entities.** Named, typed, identifiable objects that represent real-world concepts.
- **Relationships.** Explicit connections between entities that describe how they relate.
- **Components.** Typed data structures that describe aspects of an entity.
- **Context.** The surrounding information that gives meaning to entities and relationships.
- **Provenance.** The origin and history of every piece of information.

Knowledge OS is a knowledge engine. It constructs and serves knowledge, not information.

**See also:** [Entity](#entity), [Knowledge Model](#knowledge-model), [Knowledge Graph](#knowledge-graph)

---

### Artifact

A piece of content imported into the system. Artifacts are canonical data. They include documents, images, audio, video, code, and any other imported material.

An artifact is the raw material that enters the system through the import layer. The import layer transforms artifacts into entities with components and relationships.

**Properties:**

- **Source reference.** The origin of the artifact: file path, URL, API endpoint.
- **Format.** The original format: Markdown, PDF, HTML, image, audio, video, code.
- **Content.** The raw content of the artifact, stored in object storage.
- **Metadata.** Format-specific metadata extracted during parsing.

**Rules:**

1. Artifacts are canonical data. They cannot be regenerated.
2. Artifacts are stored in object storage, referenced by content-addressed keys.
3. Artifacts may be processed by multiple pipeline stages to extract entities, relationships, and components.
4. The artifact itself is never modified after import. Changes create new versions of the entity, not new versions of the artifact.

**See also:** [Entity](#entity), [Import Layer](../architecture/pipeline.md), [Object Storage](../architecture/storage.md)

---

### Knowledge Model

The canonical representation of all knowledge entities, their components, and their relationships. The knowledge model is the single source of truth. All other representations are derived from it.

The knowledge model contains:

- Entities. First-class knowledge objects.
- Components. Typed data structures attached to entities.
- Relationships. Typed, directed edges connecting entities.

The knowledge model does not contain:

- Search indexes (derived)
- Embeddings (derived)
- Caches (derived)
- Rendered views (derived)
- Any data that can be regenerated from canonical sources

The knowledge model is storage-independent. It represents reality regardless of how entities are physically persisted. Replacing a storage engine never changes the knowledge model.

**See also:** [Entity](#entity), [Canonical Data](#canonical-data), [Mental Model](../architecture/mental-model.md)

---

### Knowledge Graph

The emergent structure that arises from all entities and relationships in the system. The knowledge graph is not a separate storage engine. It is not a graph database. It is the sum of all canonical entities connected by canonical relationships.

**Properties:**

- **Heterogeneous.** It contains many entity types and relationship types. A single graph may include people, papers, concepts, tools, decisions, and events.
- **Attributed.** Entities carry components. Relationships carry attributes. The graph is rich with metadata.
- **Versioned.** Every entity and relationship is versioned. The graph at any point in time can be reconstructed.
- **Evolving.** The graph grows with every import. New entities are added. New relationships are extracted. Existing entities are updated.
- **Queryable.** The graph supports multi-hop traversal, neighborhood discovery, path finding, and pattern matching.

The knowledge graph is the primary query structure. Search indexes, vector stores, and caches are derived projections that optimize specific access patterns against the graph.

**See also:** [Entity](#entity), [Relationship](#relationship), [Mental Model](../architecture/mental-model.md)

---

## Data Concepts

These concepts define what data the system owns, what data it derives, and how data flows.

---

### Canonical Data

Data that represents the authoritative source of truth. Canonical data cannot be recreated from other sources. It requires durability, versioning, and auditing.

**Characteristics:**

- Cannot be recreated
- Represents user knowledge
- Requires durability
- Versioned
- Auditable
- The source of truth

**Examples:**

- Entities (people, papers, concepts, tools)
- Components (titles, descriptions, content, metadata)
- Relationships (created_by, references, depends_on)
- Artifacts (imported documents, images, code)
- Provenance (source attribution, import history)

**Rules:**

1. Canonical data is the source of truth. No derived data overrides canonical data.
2. Canonical data is durable. Canonical data persists across storage engine changes.
3. Canonical data is versioned. Every modification creates a new version.
4. Canonical data is auditable. Every change is traceable to its cause.
5. Canonical data is never hard-deleted. Archiving preserves history.

**See also:** [Derived Data](#derived-data), [Knowledge Model](#knowledge-model), [Data Model](../architecture/data-model.md)

---

### Derived Data

Data generated by computation from canonical data. Derived data optimizes specific access patterns. It is never authoritative. It may be discarded and reconstructed at any time.

**Characteristics:**

- May be recreated
- Optimizes performance, search, AI, navigation
- Never becomes authoritative
- Disposable
- Reproducible

**Examples:**

- Search indexes (inverted indexes for full-text retrieval)
- Embeddings (vector representations for semantic similarity)
- Similarity graphs (computed proximity between entities)
- Recommendations (AI-generated suggestions)
- Learning paths (ordered sequences for progression)
- Knowledge summaries (condensed representations)
- AI context (retrieval-augmented generation payloads)
- Caches (temporary performance optimization)

**Rules:**

1. Derived data may be discarded and rebuilt from canonical data at any time.
2. Derived data never becomes the source of truth.
3. If a derived artifact contains information that cannot be recovered from canonical sources, it is misclassified. Reclassify it as canonical.
4. Derived data is regenerated through event-driven processing when canonical data changes.

**See also:** [Canonical Data](#canonical-data), [Projection](#projection), [Data Model](../architecture/data-model.md)

---

### Metadata

Data about data. In Knowledge OS, metadata is itself canonical data. It includes entity provenance (source, import time, importer), relationship attributes (confidence, source, timestamp), and component metadata (model version, generation time).

Metadata is not separate from the knowledge model. It is part of it. Every entity carries provenance. Every relationship carries attributes. Every component carries metadata.

**Examples:**

- Entity provenance: when the entity was imported, from where, by what process.
- Relationship confidence: how confident the system is in a relationship (0.0 to 1.0).
- Component generation time: when a component was last computed or updated.
- AI model version: which model version produced an AI-generated component.

**See also:** [Provenance](#provenance), [Entity](#entity), [Relationship](#relationship)

---

### Provenance

The origin and history of a piece of information. Provenance answers: where did this come from, when was it created, who created it, and what process produced it.

Provenance is a component type (`Provenance`) that attaches to entities. It records:

- **Source.** The origin of the information: file path, URL, API endpoint.
- **Import time.** When the information was imported into the system.
- **Importer.** Which importer plugin processed the information.
- **Version.** Which version of the importer was used.

Provenance is also recorded for AI-generated content. Every AI output records:

- Which model produced it (provider, model name, version).
- When it was produced (timestamp).
- What context was used (entities, components, relationships).
- What confidence the model assigned.

**See also:** [Metadata](#metadata), [Entity](#entity), [AI](../architecture/ai.md)

---

### Projection

A view of canonical data optimized for a specific access pattern. Projections are derived data. They may be discarded and rebuilt without data loss.

The same canonical entity may appear in many projections:

- Search Index Projection (optimized for text retrieval)
- Graph Projection (optimized for relationship traversal)
- Vector Projection (optimized for semantic similarity)
- Tree View Projection (optimized for hierarchical navigation)
- Timeline Projection (optimized for temporal ordering)
- Table Projection (optimized for structured comparison)
- AI Context Projection (optimized for retrieval-augmented generation)

**Rules:**

1. Derived. Projections are generated from canonical data. They are never canonical themselves.
2. Disposable. Any projection may be discarded and rebuilt without data loss.
3. Synchronized. Projections update when canonical data changes, through event-driven processing.
4. Independent. Each projection is managed by its own subsystem. Projections do not depend on each other.

**See also:** [View](#view), [Derived Data](#derived-data), [Mental Model](../architecture/mental-model.md)

---

### View

A projection of canonical data rendered for a specific interface. Views never own information. Views render knowledge. Every entity may appear in multiple views simultaneously.

A view is the presentation-layer equivalent of a projection. While a projection is a derived data structure optimized for an access pattern, a view is the user-facing rendering of that projection.

**Properties:**

1. Derived. Views are generated from canonical data. They are never canonical themselves.
2. Disposable. Any view may be closed, discarded, or rebuilt without data loss.
3. Synchronized. Views update when canonical data changes. The user always sees current data.
4. Independent. Views do not depend on each other. Closing one view never affects another.

**View types:**

- Tree View (hierarchical navigation)
- Graph View (relationship exploration)
- Timeline (temporal ordering)
- Table (structured comparison)
- Calendar (date-based organization)
- Kanban (status-based workflow)
- Gallery (visual browsing)
- Mind Map (conceptual mapping)
- Conversation (dialogue-based interaction)
- Dashboard (aggregated overview)
- Learning Path (ordered progression)

**See also:** [Projection](#projection), [Presentation Layer](#presentation-layer), [UI Philosophy](../architecture/ui-philosophy.md)

---

### Context

The information assembled for a specific interaction, query, or AI operation. Context is derived from canonical data and determines what information is available and how it is presented.

Context is not a stored artifact. It is assembled on demand for a specific purpose. The same entity may appear in different contexts depending on the interaction.

**Context types:**

- **Workspace context.** All entities, relationships, and collections within a workspace.
- **Entity context.** A specific entity's components, relationships, and related entities.
- **Query context.** The results of a retrieval operation.
- **Conversation context.** Previous turns in an ongoing dialogue.

**Rules:**

1. Context is derived from canonical data. Context is never fabricated.
2. Context is bounded. Context size is limited to prevent information overload.
3. Context is transparent. The user may inspect what information constitutes the current context.
4. Context is adjustable. The user may expand or narrow the context.

**See also:** [AI](../architecture/ai.md), [UI Philosophy](../architecture/ui-philosophy.md)

---

## Extension Concepts

These concepts define how the system grows through plugins, adapters, and AI components.

---

### Plugin

A module that extends the system without modifying the core. Plugins implement adapters for storage engines, importers, exporters, views, AI providers, and automation agents.

Plugins are first-class citizens. The plugin API is designed before the core implementation. Plugins are isolated, replaceable, and declarative.

**Properties:**

- **Manifest.** Every plugin declares a TOML manifest specifying name, version, capabilities, dependencies, and permissions.
- **Capabilities.** What the plugin provides: importer, exporter, renderer, storage, search, vector, graph, cache, ai, automation, view.
- **Lifecycle.** Discovery, registration, activation, execution, deactivation.
- **Sandboxing.** Plugins run in isolated contexts with limited system access.

**Rules:**

1. Plugins cannot modify the core system.
2. Plugins are isolated. A plugin failure does not affect other plugins or the core.
3. Plugins are replaceable. Any plugin may be replaced by another that implements the same interface.
4. Plugins are declarative. Plugin metadata is declared in a manifest, not discovered at runtime.

**See also:** [Capability](#capability), [Importer](#importer), [Renderer](#renderer), [Extensibility](../architecture/extensibility.md)

---

### Capability

A declaration of what a plugin provides. The system queries capabilities to determine which plugins handle which operations.

**Capability types:**

| Capability   | Purpose                          | Interface           |
| ------------ | -------------------------------- | ------------------- |
| `importer`   | Import from external formats     | `ImportAdapter`     |
| `exporter`   | Export to external formats       | `ExportAdapter`     |
| `renderer`   | Render canonical data to output  | `RenderAdapter`     |
| `storage`    | Persist data to a storage engine | `StorageAdapter`    |
| `search`     | Index and retrieve text          | `SearchAdapter`     |
| `vector`     | Store and query embeddings       | `VectorAdapter`     |
| `graph`      | Store and traverse relationships | `GraphAdapter`      |
| `cache`      | Cache derived data               | `CacheAdapter`      |
| `ai`         | Provide AI operations            | `AiAdapter`         |
| `automation` | Automate knowledge operations    | `AutomationAdapter` |
| `view`       | Render a view projection         | `ViewAdapter`       |

**Capability negotiation:**

When the system needs a capability, it queries registered plugins and selects the highest-priority plugin that supports the requested capability.

**See also:** [Plugin](#plugin), [Extensibility](../architecture/extensibility.md)

---

### Importer

A plugin that receives information from an external system and transforms it into an internal representation. Importers are the boundary between the outside world and the knowledge model.

Importers implement the `ImportAdapter` trait. They receive an `ImportSource` and return a list of `ImportedEntity` objects that the parsing layer can consume.

**Built-in importers:**

| Importer | Source              | Output                                         |
| -------- | ------------------- | ---------------------------------------------- |
| Markdown | `.md` files         | Entity with Content component                  |
| PDF      | `.pdf` files        | Entity with Content + BinaryContent components |
| HTML     | URLs, `.html` files | Entity with Content component                  |
| Git      | Git repositories    | Project entity with relationship to commits    |

**Rules:**

1. Importers never perform business logic. They only transform external formats into internal representations.
2. Each importer is a plugin. New formats are added without modifying the core.
3. Importers are format-specific. A Markdown importer handles Markdown. A PDF importer handles PDFs.
4. Importers may be composed. A complex import may use multiple importers sequentially.

**See also:** [Plugin](#plugin), [Artifact](#artifact), [Import Layer](../architecture/pipeline.md)

---

### Exporter

A plugin that transforms canonical data into external formats. Exporters implement the `ExportAdapter` trait.

**Built-in exporters:**

| Exporter | Format     | Output                 |
| -------- | ---------- | ---------------------- |
| Markdown | `.md`      | Markdown files         |
| JSON     | `.json`    | Structured entity data |
| GraphML  | `.graphml` | Graph structure        |

**Rules:**

1. Exporters produce output from canonical data. They do not modify canonical data.
2. Exporters are format-specific. Each exporter handles one output format.
3. Exporters may be added as plugins without modifying the core.

**See also:** [Plugin](#plugin), [Importer](#importer)

---

### Renderer

A plugin that transforms canonical data into output formats for consumption by external systems. Renderers differ from exporters in that they produce output optimized for specific consumption patterns, not just data formats.

**Built-in renderers:**

| Renderer | Format             | Purpose                |
| -------- | ------------------ | ---------------------- |
| HTML     | `.html`            | Web pages              |
| JSON API | `application/json` | API responses          |
| MCP      | MCP protocol       | AI agent communication |

**See also:** [Plugin](#plugin), [Exporter](#exporter), [View](#view)

---

### Agent

An AI-powered component that performs autonomous tasks within the knowledge system. Agents differ from automation in that agents make decisions, while automation follows rules.

**Agent types:**

- **Research agent.** Given a topic, retrieves relevant entities, identifies gaps, and suggests entities to import.
- **Organization agent.** Suggests entity classifications, relationship types, and tag assignments.
- **Curation agent.** Identifies outdated entities, suggests updates, and recommends archiving.
- **Discovery agent.** Finds connections between distant parts of the knowledge graph.

**Rules:**

1. Agents suggest. Agents do not execute. Agent outputs are presented as proposals. A human approves or rejects each proposal.
2. Agents are scoped. Each agent operates within a defined scope: specific entity types, relationship types, or graph regions.
3. Agents are auditable. Every agent action is logged: what was proposed, what was approved, what was rejected.
4. Agents are replaceable. Agent implementations are adapters. Replacing an agent never changes the knowledge model.
5. Agents have budgets. Each agent has resource limits: maximum entities processed, maximum context size, maximum execution time.

**See also:** [Automation](#automation), [AI](../architecture/ai.md), [AI Agent Guidelines](../guides/ai-agent-guidelines.md)

---

### Automation

The process of performing tasks within the knowledge system without direct human intervention for each operation. Automation is bounded by explicit rules and is distinct from agents in that automation follows predefined rules rather than making decisions.

**Automation capabilities:**

- Import automation. Automatically import from configured sources on a schedule.
- Classification automation. Automatically classify imported entities by type and tags.
- Relationship extraction automation. Automatically extract relationships from imported content.
- Summarization automation. Automatically generate summaries for entities without Description components.
- Embedding automation. Automatically generate embeddings for entities with Content components.

**Rules:**

1. Automation is opt-in. No automation runs unless explicitly configured.
2. Automation is auditable. Every automated action is logged with its trigger, input, output, and model version.
3. Automation has limits. Automation never creates canonical entities without human review. Automation may create draft entities flagged for review.
4. Automation is stoppable. Any automation may be paused or stopped at any time.
5. Automation is replaceable. Automation providers are adapters. Replacing one provider never changes the automation rules.

**See also:** [Agent](#agent), [AI](../architecture/ai.md)

---

### Resource

An external system, service, or data source that Knowledge OS interacts with. Resources include storage engines, AI providers, external APIs, and imported data sources.

Resources are accessed through adapters. The system never depends on a specific resource implementation. Replacing a resource requires implementing a new adapter, not changing the domain model.

**Resource categories:**

- Storage resources. PostgreSQL, SQLite, Tantivy, Qdrant, Redis, S3/MinIO.
- AI resources. OpenAI, Anthropic, local models.
- Import resources. File systems, URLs, APIs, databases.
- Export resources. File systems, APIs, external services.

**See also:** [Adapter](#adapter), [Storage Engine](#storage-engine), [Plugin](#plugin)

---

### Adapter

A module that isolates a specific storage engine, AI provider, or external service from the domain model. Adapters implement a common interface. The domain model never depends on a specific implementation.

The adapter pattern is the mechanism through which the system achieves storage independence. Every storage engine, AI provider, and external service is accessed through an adapter.

**Rules:**

1. Adapters implement a common interface defined by the plugin API.
2. The domain model depends on the interface, never on the implementation.
3. Replacing an adapter requires implementing a new adapter that satisfies the same interface.
4. Adapters may be added as plugins without modifying the core.

**See also:** [Plugin](#plugin), [Storage Engine](#storage-engine), [Resource](#resource), [Storage](../architecture/storage.md)

---

## Storage Concepts

These concepts define how data is persisted and accessed.

---

### Storage Engine

A technology that persists data. Storage engines are interchangeable adapters. They optimize access patterns. They do not define meaning.

Knowledge OS uses a polyglot persistence strategy. Different storage engines are chosen for their strengths in specific access patterns. No single engine serves all purposes.

**Storage engine categories:**

| Category           | Purpose                   | Candidates                          |
| ------------------ | ------------------------- | ----------------------------------- |
| Object Storage     | Large binary artifacts    | MinIO, S3, local filesystem         |
| Relational Storage | Canonical structured data | PostgreSQL, SQLite, DuckDB          |
| Graph Storage      | Relationship traversal    | Neo4j, Memgraph, Apache AGE         |
| Search Storage     | Full-text retrieval       | Tantivy, Elasticsearch, Meilisearch |
| Vector Storage     | Semantic retrieval        | Qdrant, Milvus, pgvector, Chroma    |
| Cache Storage      | Performance optimization  | Redis, in-process caches            |

**Rules:**

1. No storage engine defines the knowledge model. The application owns the knowledge model.
2. Storage engines are adapters. They implement a common interface.
3. Replacing one storage engine never changes the architecture.
4. Canonical data uses strong consistency within a single storage engine. Derived data uses eventual consistency.

**See also:** [Adapter](#adapter), [Storage Philosophy](../architecture/storage.md), [Polyglot Persistence](https://martinfowler.com/bliki/PolyglotPersistence.html)

---

### Source of Truth

The authoritative data from which all other representations are derived. In Knowledge OS, the canonical knowledge model is the source of truth.

The source of truth is the data that, if lost, cannot be reconstructed. If the search index is lost, rebuild it. If the embedding store is lost, recompute it. If the canonical model is lost, knowledge is lost.

**Rules:**

1. The canonical knowledge model is the source of truth.
2. Derived data is never the source of truth.
3. AI outputs are never the source of truth without human approval.
4. No storage engine is the source of truth. The application owns the knowledge model.

**See also:** [Canonical Data](#canonical-data), [Knowledge Model](#knowledge-model)

---

## Pipeline Concepts

These concepts define how information flows through the system.

---

### Pipeline

The sequence of layers through which information flows: import, parsing, normalization, knowledge model, relationship engine, derivation, presentation. The pipeline is deterministic, composable, and testable.

The pipeline follows the compiler architecture analogy. Information enters as source material and exits as rendered knowledge projections. Between these endpoints, each layer transforms the data in a specific, isolated, deterministic way.

**Layers:**

1. **Import Layer.** Receive information from external systems.
2. **Parsing Layer.** Extract structured information.
3. **Normalization Layer.** Convert to canonical representations.
4. **Knowledge Model.** Entity storage and lifecycle.
5. **Relationship Engine.** Connect entities through typed edges.
6. **Derivation Layer.** Generate indexes, embeddings, recommendations.
7. **Presentation Layer.** Render projections for human and machine interfaces.

**Properties:**

- Deterministic. The same input always produces the same canonical output.
- Idempotent. Processing the same input twice produces the same result.
- Auditable. Every transformation is recorded. Every entity change is versioned.
- Extensible. New importers, parsers, views, and storage engines are added as plugins.
- Independent. No layer depends on a specific technology. Adapters isolate implementation details.

**See also:** [Pipeline Architecture](../architecture/pipeline.md), [Compilation](../architecture/compilation.md)

---

### Presentation Layer

The layer that renders projections of knowledge for human and machine interfaces. Views are projections. No view owns data.

The presentation layer is the final stage of the pipeline. It takes derived data and renders it in a form optimized for a particular interaction pattern.

**See also:** [View](#view), [Projection](#projection), [Pipeline](#pipeline), [UI Philosophy](../architecture/ui-philosophy.md)

---

## System Concepts

These concepts define how the system is organized and how it grows.

---

### Collection

A curated group of entities organized for a specific purpose. Collections are themselves entities. They may be manually curated or dynamically generated.

Collections provide a way to group related entities without imposing a hierarchy. A paper may belong to a collection called "Papers to Read" without being removed from its other relationships.

**Rules:**

1. Collections are entities. They have components, relationships, and versions.
2. Collections contain other entities through `contains` relationships.
3. Collections may be manually curated by users or dynamically generated by AI agents.
4. Collections do not impose ordering. Ordering is a projection, not a property of the collection.

**See also:** [Entity](#entity), [Workspace](#workspace)

---

### Workspace

A bounded context within the knowledge system. Workspaces scope access, collaboration, and organization. Workspaces are themselves entities.

A workspace provides isolation and access control. Each workspace has its own set of entities, relationships, and permissions. Users are assigned roles within workspaces: viewer, editor, curator, admin.

**Properties:**

- **Name.** A human-readable identifier for the workspace.
- **Members.** Users with assigned roles within the workspace.
- **Access control.** Rules specifying who can read, write, or delete entities within the workspace.
- **Scope.** The set of entities, relationships, and collections within the workspace.

**Rules:**

1. Workspaces are entities. They have components, relationships, and versions.
2. Workspaces provide isolation. Entities in one workspace are not visible in another unless explicitly shared.
3. Workspaces support multi-user collaboration with conflict resolution.
4. Workspaces are extensible. New workspace features are added as plugins.

**See also:** [Entity](#entity), [Collection](#collection), [Synchronization](../architecture/synchronization.md)

---

### Determinism

The property that the same input always produces the same canonical output. Determinism is a fundamental architectural invariant of Knowledge OS.

Determinism means:

- No randomness in the pipeline.
- No non-deterministic side effects.
- No hidden state that affects the pipeline.
- No external dependencies that change behavior.

Determinism enables:

- **Reproducibility.** Re-running the pipeline on the same data produces identical results.
- **Testing.** Pipeline stages can be tested with fixed inputs and expected outputs.
- **Debugging.** Failures can be reproduced by re-running the pipeline.
- **Recovery.** Derived data can be rebuilt by re-running the pipeline from canonical data.

**See also:** [Pipeline](#pipeline), [Compilation](../architecture/compilation.md)

---

### Idempotency

The property that processing the same input twice produces the same result. Idempotency is critical for event processing, where the same event may be delivered multiple times due to network issues, processing failures, or replay scenarios.

**Idempotency strategies:**

- Event deduplication. Track processed event IDs. Skip already-processed events.
- Version-based updates. Use entity version numbers to avoid applying stale updates.
- Upsert operations. Use `INSERT ... ON CONFLICT ... DO UPDATE` for database operations.

**See also:** [Event](#event), [Events](../architecture/events.md), [Pipeline](#pipeline)

---

### Event

A record of a meaningful change to canonical data. Events trigger asynchronous processing through the derivation pipeline. Events are immutable and ordered.

**Event types:**

- Canonical events: `EntityCreated`, `EntityUpdated`, `EntityArchived`, `RelationshipCreated`, `RelationshipUpdated`, `RelationshipArchived`, `ComponentAdded`, `ComponentUpdated`, `ComponentRemoved`.
- Derivation events: `ArtifactImported`, `EmbeddingGenerated`, `SearchIndexed`, `GraphProjected`, `RecommendationGenerated`.

**Event properties:**

- **ID.** Unique event identifier (UUID).
- **Kind.** Type of event.
- **Entity ID.** Affected entity (if applicable).
- **Timestamp.** When the event occurred.
- **Version.** Entity version after this event.
- **Metadata.** Source, actor, correlation ID, causation ID.

**Rules:**

1. Events are immutable once created.
2. Events are ordered by timestamp within a single entity.
3. Events support full replay for rebuilding derived data.
4. Events that fail processing after maximum retries are moved to a dead letter queue.

**See also:** [Pipeline](#pipeline), [Synchronization](#synchronization), [Events](../architecture/events.md)

---

### Synchronization

The process of keeping derived data consistent with canonical data. Synchronization is event-driven and eventually consistent. Derived data may briefly lag behind canonical data.

**Synchronization model:**

1. Canonical data changes are committed atomically.
2. Events are published for derived data generation.
3. Derived data is updated asynchronously.
4. Views may briefly show stale data until derived data catches up.

**Rules:**

1. Eventual consistency. Given enough time, all derived data converges to the correct state.
2. Idempotency. Processing the same event twice produces the same result.
3. Ordered processing per entity. Events for a single entity are processed in order.
4. Durability of canonical data. Canonical changes are committed before events are published.

**See also:** [Event](#event), [Derived Data](#derived-data), [Synchronization Architecture](../architecture/synchronization.md)

---

### Version

A snapshot of an entity or relationship at a specific point in time. Versions enable audit trails, rollback, and temporal queries.

Every entity and relationship is versioned. The version number is a monotonically increasing integer. Every modification increments the version.

**Version types:**

- **Schema version.** The version of the entity, relationship, and component type definitions.
- **Data version.** The version of each individual entity and relationship.

**Rules:**

1. Versions are monotonically increasing. Version numbers never decrease.
2. Versions are immutable. A version represents a specific state and never changes.
3. Versions enable temporal queries. "What did this entity look like at version 5?"
4. Versions enable conflict detection. Two concurrent modifications to the same entity are detected through version mismatches.

**See also:** [Entity](#entity), [Relationship](#relationship), [Versioning](../philosophy/engineering-principles.md)

---

## Anti-Concepts

These are things Knowledge OS explicitly is not. Understanding what the system is not clarifies what it is.

---

### Not a Database

Knowledge OS is not a database. It is a knowledge engine that uses databases as storage adapters. Databases are implementation details. The application owns the knowledge model. If you need a database, use PostgreSQL, SQLite, or DuckDB directly.

### Not an AI Product

Knowledge OS is not a chatbot, a writing assistant, or a code generator. AI is a pipeline component that assists in knowledge construction. The system's value is the knowledge model, not the AI model.

### Not a Note-Taking App

Knowledge OS is not a competitor to Notion, Obsidian, or Apple Notes. Those are document-centric tools. Knowledge OS is entity-centric. Documents are inputs. Entities are the output.

### Not a Search Engine

Knowledge OS does not build a search product. Search is one projection of canonical data. Search retrieves text. Knowledge retrieval surfaces entities.

### Not a Collaboration Platform

Knowledge OS does not build real-time collaborative editing, chat, or video conferencing. Knowledge OS is a knowledge engine. Collaboration tools may consume its API, but collaboration is not our domain.

### Not a Cloud Service

Knowledge OS does not operate infrastructure for users. Knowledge OS is an engine that can be deployed anywhere -- local machine, private cloud, or public cloud. Users own their data. Users own their infrastructure.

**See also:** [Goals and Non-Goals](../philosophy/boundaries.md), [Anti-Goals](../philosophy/philosophy.md)

---

## Further Reading

- [Mental Model](../architecture/mental-model.md) -- The conceptual foundation
- [Domain Model](../architecture/domain-model.md) -- Entity, relationship, and component types
- [Data Model](../architecture/data-model.md) -- Canonical vs derived data
- [Pipeline](../architecture/pipeline.md) -- The seven-layer architecture
- [Storage](../architecture/storage.md) -- Polyglot persistence
- [Composition](../architecture/composition.md) -- Entity component model
- [Events](../architecture/events.md) -- Event-driven architecture
- [Extensibility](../architecture/extensibility.md) -- Plugin system
- [AI](../architecture/ai.md) -- AI integration
- [Philosophy](../philosophy/philosophy.md) -- Core principles
