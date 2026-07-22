# Composition

> Inheritance is avoided. Entities acquire behavior through composition. Each entity consists of components.

---

## Overview

Knowledge OS uses the Entity Component Model to define entities. This pattern, widely adopted in game engine architecture, replaces deep inheritance hierarchies with flat, composable component assemblies.

Instead of asking "what is this entity?", the system asks "what components does this entity have?" An entity is not defined by its type. It is defined by the set of components attached to it.

This approach is documented extensively in the [Entity Component System](https://en.wikipedia.org/wiki/Entity_component_system) literature, where it has been proven to reduce coupling, improve extensibility, and eliminate the fragility of deep class hierarchies.

---

## The Problem with Inheritance

Traditional object-oriented design uses inheritance to share behavior:

```
Entity
  +-- Document
       +-- ResearchPaper
       +-- Book
       +-- Article
```

This creates several problems:

- **Rigidity.** Adding new combinations of behavior requires new classes.
- **Code duplication.** Similar functionality duplicated across branches.
- **Bloated base classes.** Base classes accumulate responsibilities.
- **Fragile hierarchies.** Changes to a base class cascade to all subclasses.
- **Diamond problem.** Multiple inheritance creates ambiguity.

As noted in the [Vulkan Documentation Project](https://github.khronos.org/Vulkan-Site/tutorial/latest/Building_a_Simple_Engine/Engine_Architecture/03_component_systems.html), deep inheritance hierarchies lead to "rigidity, code duplication, bloated classes, and difficult refactoring."

---

## The Component Model

In the component model, an entity is a container for components. A component is a data structure that represents one aspect of the entity.

```
Entity: "Deep Learning Paper"
  +-- Title { name: "Attention Is All You Need" }
  +-- Description { summary: "..." }
  +-- Content { markdown: "..." }
  +-- Author { people: [Vaswani, Shazeer, ...] }
  +-- Tags { values: ["transformer", "attention", "NLP"] }
  +-- Timeline { created: 2017-06-12 }
  +-- Embedding { vector: [...], model: "text-embedding-3" }
  +-- VersionHistory { versions: [...] }
```

The same entity type can have different components:

```
Entity: "John Smith"
  +-- Title { name: "John Smith" }
  +-- Description { bio: "..." }
  +-- Tags { values: ["researcher", "ML"] }
  +-- Timeline { created: 2024-01-15 }
  +-- Location { city: "San Francisco" }

Entity: "PyTorch"
  +-- Title { name: "PyTorch" }
  +-- Description { summary: "..." }
  +-- Tags { values: ["framework", "ML", "Python"] }
  +-- Timeline { created: 2016-10-18 }
  +-- BinaryContent { repository: "https://github.com/pytorch/pytorch" }
```

---

## Component Types

Components are composable, reusable data structures. Each component has a single responsibility.

**Core components:**

| Component | Purpose |
|-----------|---------|
| `Title` | Human-readable name |
| `Description` | Summary or biography |
| `Content` | Markdown or rich text body |
| `BinaryContent` | File, image, or media reference |
| `Author` | Attribution to people or organizations |
| `Tags` | Categorical labels |
| `Timeline` | Temporal metadata (created, modified, etc.) |
| `Location` | Geographic or spatial data |
| `Language` | Natural language detection |
| `Rating` | Quality or relevance score |

**Advanced components:**

| Component | Purpose |
|-----------|---------|
| `Embedding` | Vector representation for semantic search |
| `VersionHistory` | Change tracking and audit trail |
| `Permissions` | Access control rules |
| `Provenance` | Source attribution and import history |
| `Thumbnail` | Visual preview |

---

## Component Rules

1. **Single responsibility.** Each component handles one aspect of the entity.
2. **No dependencies.** Components do not depend on other components.
3. **Data only.** Components contain data, not behavior. Behavior lives in systems.
4. **Composable.** Any component may be attached to any entity.
5. **Optional.** Entities may have any combination of components.

---

## Systems

Systems operate on entities that have specific component combinations. A system queries for entities with required components and processes them.

```
SearchSystem:
  Query: entities with [Content, Embedding]
  Action: Generate search index entries

RelationshipExtractionSystem:
  Query: entities with [Content, Author]
  Action: Extract and verify relationships

EmbeddingSystem:
  Query: entities with [Content] without [Embedding]
  Action: Generate vector embeddings
```

Systems are decoupled from entities. They operate on component sets, not entity types.

---

## Storage Mapping

Components map to storage as follows:

- **Relational storage:** Entity-component relationships, metadata fields.
- **Object storage:** Binary content, large text fields, images.
- **Vector storage:** Embedding components.
- **Search storage:** Content and tag components (indexed for retrieval).

Adding a new component type does not require schema migrations in all storage engines. Only the engines that store that component's data are affected.

---

## Comparison with ECS in Game Engines

| Game Engine ECS | Knowledge OS ECS |
|----------------|-------------------|
| Entity = game object | Entity = knowledge concept |
| Component = behavior data | Component = entity aspect |
| System = game logic | System = pipeline stage |
| Frame-based updates | Event-driven updates |
| Spatial components | Semantic components |

The pattern is identical. The domain is different.

---

## Further Reading

- [Overview](overview.md) -- System-level architecture
- [Data Model](data-model.md) -- How components relate to canonical data
- [Pipeline](pipeline.md) -- How systems process components
- [Storage](storage.md) -- How components map to storage engines
