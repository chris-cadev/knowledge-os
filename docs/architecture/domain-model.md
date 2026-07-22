# Domain Model

> Every first-class concept in the system is defined here. This document is the authoritative reference for entity types, relationship types, component types, invariants, and extensibility rules.

---

## Entity Types

Entity types define the categories of knowledge the system recognizes. Every entity belongs to exactly one type. The type determines which component types are expected but never which components are required.

### Core Entity Types

| Type | Description | Example |
|------|-------------|---------|
| `Concept` | An abstract idea, principle, or theory | Monads, Machine Learning, Supply and Demand |
| `Person` | A human individual | Ada Lovelace, Geoffrey Hinton |
| `Organization` | A company, institution, or group | Google, MIT, Apache Foundation |
| `Project` | A named effort with a goal | Knowledge OS, Linux, Wikipedia |
| `Book` | A published book | Designing Data-Intensive Applications |
| `Paper` | A research paper or article | Attention Is All You Need |
| `Video` | A video recording | A conference talk, a tutorial |
| `Article` | A written piece (blog post, essay) | A technical blog post |
| `Tool` | A software tool or utility | LLVM, ripgrep, PostgreSQL |
| `Technology` | A technology or framework | Rust, Kubernetes, GraphQL |
| `Question` | A question that seeks an answer | "How does backpropagation work?" |
| `Idea` | An unvalidated concept or hypothesis | "Knowledge graphs could replace databases" |
| `Event` | A temporal occurrence | A conference, a release, a decision point |
| `Skill` | A competency or capability | Rust programming, systems design |
| `Location` | A geographic or virtual place | San Francisco, a GitHub repository |
| `Dataset` | A structured collection of data | ImageNet, MNIST |
| `Collection` | A curated group of entities | "Papers to read," "AI tools" |
| `Workspace` | A bounded context for collaboration | A team's knowledge space |
| `Decision` | An architectural or product decision | "Use event sourcing for the pipeline" |
| `Note` | A user-created annotation | A thought, a reminder, a comment |

### Entity Type Rules

1. **Every entity has exactly one type.** The type is assigned at creation and never changes.
2. **Types are extensible.** New entity types are added through configuration, not code changes.
3. **Types do not determine behavior.** Behavior is determined by component assembly, not entity type.
4. **Types are metadata.** The type is a label that aids classification. It does not constrain what components an entity may have.

---

## Relationship Types

Relationship types define the semantic nature of connections between entities. Every relationship has exactly one type.

### Structural Relationships

| Type | Source | Target | Description |
|------|--------|--------|-------------|
| `contains` | Collection, Workspace | Any entity | The source contains the target |
| `belongs_to` | Any entity | Collection, Workspace | The target contains the source |
| `part_of` | Any entity | Any entity | The target is a larger whole |
| `has_part` | Any entity | Any entity | The source is a larger whole |

### Provenance Relationships

| Type | Source | Target | Description |
|------|--------|--------|-------------|
| `created_by` | Any entity | Person, Organization | The target created the source |
| `authored_by` | Paper, Article, Book | Person | The target authored the source |
| `imported_from` | Any entity | External system | The source was imported from the target |
| `derived_from` | Any entity | Any entity | The source was derived from the target |

### Semantic Relationships

| Type | Source | Target | Description |
|------|--------|--------|-------------|
| `references` | Any entity | Any entity | The source explicitly references the target |
| `related_to` | Any entity | Any entity | The source is semantically related to the target |
| `inspired_by` | Any entity | Any entity | The source was inspired by the target |
| `contradicts` | Any entity | Any entity | The source contradicts the target |
| `supports` | Any entity | Any entity | The source provides evidence for the target |

### Dependency Relationships

| Type | Source | Target | Description |
|------|--------|--------|-------------|
| `depends_on` | Any entity | Any entity | The source requires the target |
| `implements` | Tool, Technology | Concept | The source implements the target |
| `requires` | Any entity | Any entity | The source needs the target to function |
| `extends` | Any entity | Any entity | The source builds upon the target |

### Knowledge Relationships

| Type | Source | Target | Description |
|------|--------|--------|-------------|
| `teaches` | Paper, Book, Video, Article | Concept, Skill | The source teaches the target |
| `learned_from` | Person | Paper, Book, Course | The person learned from the source |
| `applies` | Tool, Technology | Concept | The source applies the target concept |
| `measures` | Dataset, Tool | Concept | The source measures the target |

### Relationship Type Rules

1. **Every relationship has exactly one type.** The type is assigned at creation and never changes.
2. **Types are extensible.** New relationship types are added through configuration.
3. **Direction matters.** `A references B` is distinct from `B references A`.
4. **Types carry semantic meaning.** The type determines what the relationship means, not just that a connection exists.

---

## Component Types

Component types define the aspects of entities. Every component belongs to exactly one type. The component type determines the structure of the data payload.

### Core Components

| Type | Payload | Description |
|------|---------|-------------|
| `Title` | `{ name: string }` | Human-readable name |
| `Description` | `{ text: string }` | Summary, biography, or abstract |
| `Content` | `{ markdown: string }` | Full text body in Markdown |
| `BinaryContent` | `{ reference: string, mime_type: string, size: u64 }` | Reference to binary data in object storage |
| `Tags` | `{ values: string[] }` | Categorical labels |
| `Timeline` | `{ created_at: DateTime, modified_at: DateTime, ... }` | Temporal metadata |
| `Language` | `{ code: string }` | Natural language (ISO 639-1) |
| `Rating` | `{ score: f64, scale: f64 }` | Quality or relevance score |

### Identity Components

| Type | Payload | Description |
|------|---------|-------------|
| `Author` | `{ people: EntityRef[], organizations: EntityRef[] }` | Attribution to people or organizations |
| `Thumbnail` | `{ reference: string }` | Visual preview reference |
| `Location` | `{ name: string, lat: f64, lon: f64 }` | Geographic or spatial data |
| `ExternalIdentifier` | `{ system: string, id: string }` | Identifiers from external systems (DOI, ISBN, URL) |

### Knowledge Components

| Type | Payload | Description |
|------|---------|-------------|
| `Embedding` | `{ vector: f64[], model: string }` | Vector representation for semantic search |
| `Summary` | `{ text: string, generated_by: string }` | Condensed representation |
| `Classification` | `{ taxonomy: string, labels: string[] }` | Categorical classification |
| `Confidence` | `{ score: f64, source: string }` | Confidence in the entity's correctness |

### System Components

| Type | Payload | Description |
|------|---------|-------------|
| `VersionHistory` | `{ versions: Version[] }` | Change tracking and audit trail |
| `Permissions` | `{ rules: PermissionRule[] }` | Access control rules |
| `Provenance` | `{ source: string, imported_at: DateTime, importer: string }` | Source attribution and import history |
| `WorkspaceMembership` | `{ workspace: EntityRef, role: string }` | Workspace association |

### Component Type Rules

1. **Every component has exactly one type.** The type determines the data schema.
2. **Types are extensible.** New component types are added through configuration.
3. **Components are optional.** An entity may have zero or more components of any type.
4. **No duplicate types per entity.** An entity may have at most one component of each type.
5. **Components are versioned with their entity.** When a component changes, the entity version increments.

---

## Invariants

Invariants are properties that must always hold. They are never violated, regardless of implementation.

### Entity Invariants

1. **Every entity has a unique, immutable identifier.** Once assigned, the identifier never changes.
2. **Every entity has exactly one type.** The type never changes after creation.
3. **Every entity is versioned.** The version number is a monotonically increasing integer.
4. **Every entity is auditable.** The creation time, source, and modification history are preserved.
5. **Entities are never hard-deleted.** Archiving marks an entity as inactive. The entity and its history are preserved.

### Relationship Invariants

1. **Every relationship connects two existing entities.** A relationship cannot reference a nonexistent entity.
2. **Every relationship has exactly one type.** The type never changes after creation.
3. **Every relationship is directed.** A relationship has a source and a target.
4. **Every relationship is versioned.** The version number tracks relationship modifications.
5. **Relationships are never ambiguous.** A relationship between two entities with the same type and direction is unique.

### Component Invariants

1. **Every component belongs to exactly one entity.**
2. **Every component has exactly one type within its entity.** An entity cannot have two components of the same type.
3. **Components are data only.** Components contain payloads, not behavior.
4. **Components are versioned with their entity.** Component changes increment the entity version.

### Canonical Data Invariants

1. **Canonical data is the source of truth.** No derived data overrides canonical data.
2. **Canonical data is durable.** Canonical data persists across storage engine changes.
3. **Canonical data is versioned.** Every modification creates a new version.
4. **Canonical data is auditable.** Every change is traceable to its cause.

---

## Extensibility Rules

The domain model is designed to grow. These rules govern how it extends.

### Adding Entity Types

New entity types are added through configuration. No code changes are required. The configuration specifies:

- The type name (unique, snake_case)
- Expected component types (optional, for validation)
- Display name and description

### Adding Relationship Types

New relationship types are added through configuration. The configuration specifies:

- The type name (unique, snake_case)
- Allowed source entity types (optional, for validation)
- Allowed target entity types (optional, for validation)
- Display name and description

### Adding Component Types

New component types are added through configuration or plugins. The configuration specifies:

- The type name (unique, snake_case)
- The data schema (typed fields)
- Display name and description

Plugins may introduce component types that require custom rendering or storage logic.

### Extensibility Constraints

1. **Core types are immutable.** The core entity, relationship, and component types defined in this document are permanent. They may be extended, never removed.
2. **New types must be additive.** Adding a type must not break existing entities, relationships, or components.
3. **Custom types must follow the same invariants.** Custom types are subject to the same rules as core types.
4. **Types must be declarative.** Type definitions are data, not code. They are stored in configuration, not in compiled binaries.

---

## Domain Boundaries

The domain model defines what Knowledge OS knows about. Everything outside these boundaries is external.

### Inside the Boundary

- Entity lifecycle (create, update, archive, restore)
- Component attachment and modification
- Relationship creation, modification, and traversal
- Canonical data storage and versioning
- Derived data generation and regeneration
- Event emission and processing
- Projection rendering
- Import and normalization

### Outside the Boundary

- How entities are displayed (presentation is a projection)
- How data is physically stored (storage is an adapter)
- How AI models are selected (AI is a component)
- How external systems integrate (integration is a plugin)
- How users collaborate (collaboration is a workspace concern)
- How data is transmitted over networks (transport is an infrastructure concern)

---

## Further Reading

- [Mental Model](mental-model.md) -- The conceptual foundation
- [Composition](composition.md) -- Entity component model in detail
- [Data Model](data-model.md) -- Canonical vs derived data
- [Pipeline](pipeline.md) -- How the domain model is processed
- [Storage](storage.md) -- How domain objects map to storage
