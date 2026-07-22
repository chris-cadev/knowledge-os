# User Interface Philosophy

> Every interface is a projection. No interface owns data. Views render knowledge. Knowledge remains canonical.

---

## Views

A view is a projection of canonical data rendered for a specific interaction pattern. Views never own information. Views never modify canonical data directly. Views read canonical data and render it in a form optimized for a particular task.

Every entity may appear in multiple views simultaneously. A research paper may appear in a graph view (showing its relationships), a timeline view (showing its publication date), a table view (comparing it with other papers), and a conversation view (answering a question about it). The paper is not duplicated. The canonical entity is rendered differently in each projection.

### View Properties

1. **Derived.** Views are generated from canonical data. They are never canonical themselves.
2. **Disposable.** Any view may be closed, discarded, or rebuilt without data loss.
3. **Synchronized.** Views update when canonical data changes. The user always sees current data.
4. **Independent.** Views do not depend on each other. Closing one view never affects another.

---

## Navigation

Navigation in Knowledge OS is entity-centric, not location-centric.

### Universal Navigation Rule

Every entity is reachable from every other entity through explicit relationships. There are no dead ends. There are no orphaned views. The knowledge graph is connected.

### Navigation Patterns

**Entity navigation.** From any entity, the user may traverse any relationship to reach a connected entity. The navigation path is explicit: `Entity A --[relationship_type]--> Entity B`.

**Search navigation.** The user issues a query. The system retrieves relevant entities. The user selects an entity to navigate to it.

**Graph navigation.** The user explores the knowledge graph visually, following edges between entity nodes.

**Hierarchical navigation.** The user navigates through collections and workspaces, drilling down from groups to individual entities.

**Temporal navigation.** The user navigates through time, viewing entities by creation date, modification date, or event timeline.

### Navigation Rules

1. **Navigation never loses context.** The user always knows where they are in the knowledge graph.
2. **Navigation is reversible.** Every navigation action can be undone by traversing back.
3. **Navigation is auditable.** The navigation history is preserved for review.
4. **Navigation is fast.** The system optimizes for common navigation patterns through derived projections.

---

## Context

Every interaction in Knowledge OS occurs within a context. Context determines what information is available and how it is presented.

### Context Types

**Workspace context.** The user is within a workspace. All entities, relationships, and collections within the workspace are accessible.

**Entity context.** The user is viewing a specific entity. The entity's components, relationships, and related entities are available.

**Query context.** The user has issued a query. The retrieval results form the context for subsequent interactions.

**Conversation context.** The user is in a dialogue with the system. Previous turns in the conversation provide context for subsequent exchanges.

### Context Rules

1. **Context is derived from canonical data.** Context is assembled from entities, components, and relationships.
2. **Context is bounded.** Context size is limited to prevent information overload.
3. **Context is transparent.** The user may inspect what information constitutes the current context.
4. **Context is adjustable.** The user may expand or narrow the context.

---

## Spatial Interfaces

Spatial interfaces render knowledge as objects in a navigable space.

### Graph View

The graph view renders entities as nodes and relationships as edges. The user explores the knowledge graph visually.

**Properties:**
- Nodes represent entities. Node size may reflect importance, connection count, or custom metrics.
- Edges represent relationships. Edge labels show relationship type.
- Layout algorithms position nodes to minimize edge crossing and maximize readability.
- The user may pan, zoom, filter, and select nodes.

**Use cases:**
- Exploring relationships between concepts
- Discovering connections between distant parts of the knowledge graph
- Visualizing the structure of a domain

### Mind Map View

The mind map view renders knowledge as a radial hierarchy emanating from a central concept.

**Properties:**
- The central node is the focus entity.
- Branches radiate outward through relationship traversal.
- Branch depth is configurable.
- Nodes are sized and colored by type or metadata.

**Use cases:**
- Conceptual mapping of a topic
- Brainstorming and idea generation
- Structuring a learning path

---

## Graph Interfaces

Graph interfaces render knowledge as interconnected structures optimized for relationship exploration.

### Neighborhood View

The neighborhood view shows all entities within N hops of a focus entity.

```
Focus: Concept("machine learning")
Depth: 2 hops
Result: Subgraph of related concepts, people, papers, tools
```

### Path View

The path view shows the connection between two entities.

```
Source: Concept("backpropagation")
Target: Tool("TensorFlow")
Path: backpropagation --implements--> neural networks --implemented_by--> TensorFlow
```

### Cluster View

The cluster view identifies and renders groups of densely connected entities.

```
Cluster: "Deep Learning Ecosystem"
Entities: neural networks, transformers, attention, PyTorch, TensorFlow, GPUs
Edges: dense interconnections within the cluster
```

---

## Search Interfaces

Search interfaces retrieve entities matching a query and present them in a structured format.

### Search Bar

The search bar accepts natural language queries and returns ranked results.

**Capabilities:**
- Full-text matching against indexed fields
- Semantic similarity matching against embeddings
- Entity type filtering
- Relationship-aware ranking
- Autocomplete from entity titles and tags

### Advanced Search

Advanced search provides structured query building.

**Capabilities:**
- Boolean operators (AND, OR, NOT)
- Field-specific queries (title, tags, content, author)
- Relationship queries (entities that reference X, entities created by Y)
- Temporal queries (entities modified after date Z)

### Search Results

Search results are rendered as a list of entities with:
- Entity type and title
- Relevant snippet or summary
- Relationship to the query (why this result was returned)
- Quick actions (view, add to collection, create relationship)

---

## AI Interfaces

AI interfaces enable conversational interaction with the knowledge graph.

### Conversation View

The conversation view is a dialogue between the user and the system.

**Capabilities:**
- Natural language questions about the knowledge graph
- Multi-turn conversations with context retention
- Source citations for every claim
- Follow-up suggestions based on conversation history

### AI Context Panel

The AI context panel shows what information the AI is using to generate responses.

**Properties:**
- Lists entities included in the AI context
- Shows relationship graph used for reasoning
- Displays confidence scores for AI-generated content
- Allows the user to add or remove entities from context

### AI Suggestion Panel

The AI suggestion panel presents proposed actions for user review.

**Capabilities:**
- Entity classification suggestions
- Relationship creation suggestions
- Summary generation proposals
- Organization and curation recommendations

---

## Progressive Disclosure

Simple tasks have simple interfaces. Complex capabilities are available but not forced.

### Disclosure Levels

**Level 1 -- Essentials.** The default view shows the most important information: entity title, summary, and key relationships. This is sufficient for most interactions.

**Level 2 -- Details.** Expanding an entity reveals all components: full content, metadata, tags, timeline, and all relationships. This is sufficient for detailed review.

**Level 3 -- Advanced.** Advanced panels reveal system internals: version history, provenance, AI context, storage details, event history. This is sufficient for debugging and auditing.

**Level 4 -- Developer.** Developer tools expose the raw data model: canonical entity JSON, relationship graphs, component schemas, event streams. This is sufficient for plugin development and system administration.

### Disclosure Rules

1. **Default to simplicity.** The initial view is always the simplest useful view.
2. **Never hide information.** Complex information is available at deeper disclosure levels, never removed.
3. **Respect user expertise.** The system adapts disclosure level based on user behavior and preferences.
4. **Maintain consistency.** Disclosure levels follow the same pattern across all views.

---

## Universal Navigation

Every interface in Knowledge OS follows the same navigation principles.

### Navigation Axioms

1. **Every entity has a URL.** Every entity is addressable. Links to entities are stable and persistent.
2. **Every view is a projection.** No view owns data. Changing views never changes the underlying knowledge.
3. **Every transition is reversible.** Navigation history enables backtracking.
4. **Every context is inspectable.** The user may always examine what data underlies the current view.

### Keyboard Navigation

Every interface supports keyboard navigation:
- `Tab` moves between entities
- `Enter` opens the selected entity
- `Escape` returns to the previous view
- `Arrow keys` traverse relationships
- `/` opens search
- `?` shows keyboard shortcuts

### Accessibility

Every interface follows accessibility standards:
- Screen reader support for all views
- Keyboard-only navigation for all operations
- High contrast mode for visual accessibility
- Adjustable text size and spacing

---

## Further Reading

- [Mental Model](mental-model.md) -- The conceptual foundation for projections
- [Pipeline](pipeline.md) -- How views are rendered from canonical data
- [AI](ai.md) -- How AI interfaces interact with the knowledge model
- [Composition](composition.md) -- How entities map to view components
