# IP-002: Phase 2 -- View Projections

**Status:** Draft
**ADR(s):** [ADR-0015](../../architecture/adrs/adr-0015.md) (View Projection System)
**PRD(s):** [PRD-0003](../prds/prd-0003-graph-exploration-and-plugins.md) (US2: Tree view, US3: Graph view, US4: Table view, Timeline view)
**Estimated effort:** ~5 days

---

## Context

ADR-0015 introduced `ViewAdapter` as a trait for rendering canonical data into projections. Four built-in views (tree, graph, table, timeline) implement this trait. Views are updated event-driven -- when canonical data changes, views invalidate and rebuild.

This phase builds on IP-001's traversal results. The graph view and subgraph view use `TraversalPort` to extract bounded subgraphs.

---

## Deliverables

### D1: View Types and ViewAdapter Trait

**Purpose:** Define the view projection types and trait in `knowledge-core`

**Files:**

| File | Action | Description |
|------|--------|-------------|
| `core/knowledge-core/src/ports/mod.rs` | Modify | Add `ViewAdapter` trait, `ViewFilter`, `ViewOutput`, `ViewRegistry`, and all view data types |

**New types (per ADR-0015):**

```rust
#[async_trait]
pub trait ViewAdapter: Send + Sync {
    fn name(&self) -> &str;
    async fn render(&self, filter: &ViewFilter) -> Result<ViewOutput>;
    async fn on_event(&self, event: &Event) -> Result<()>;
}

pub struct ViewFilter {
    pub entity_types: Option<Vec<EntityType>>,
    pub tags: Option<Vec<String>>,
    pub relationship_types: Option<Vec<RelationshipType>>,
    pub max_depth: Option<u32>,
    pub start_entity_id: Option<String>,
    pub sort_by: Option<String>,
    pub sort_order: Option<SortOrder>,
    pub search_query: Option<String>,
}

pub enum SortOrder { Asc, Desc }

pub enum ViewOutput {
    Tree(TreeData),
    Graph(GraphData),
    Table(TableData),
    Timeline(TimelineData),
}

pub struct TreeData { pub roots: Vec<TreeNode> }
pub struct TreeNode { pub entity: Entity, pub children: Vec<TreeNode> }
pub struct GraphData { pub nodes: Vec<GraphNode>, pub edges: Vec<GraphEdge> }
pub struct GraphNode { pub entity: Entity, pub label: String, pub node_type: String }
pub struct GraphEdge { pub source_id: String, pub target_id: String, pub relationship_type: String, pub label: String }
pub struct TableData { pub columns: Vec<TableColumn>, pub rows: Vec<TableRow> }
pub struct TableColumn { pub name: String, pub sortable: bool }
pub struct TableRow { pub cells: Vec<String> }
pub struct TimelineData { pub entries: Vec<TimelineEntry> }
pub struct TimelineEntry { pub entity: Entity, pub timestamp: String, pub label: String }

pub struct ViewRegistry {
    views: HashMap<String, Box<dyn ViewAdapter>>,
}
```

**Verification:**
- `cargo check -p knowledge-core` compiles
- `cargo test -p knowledge-core` passes (no regressions)

**Exit criteria:** Types compile, existing tests pass

---

### D2: Tree View Adapter

**Purpose:** Implement hierarchical navigation grouped by entity type

**Files:**

| File | Action | Description |
|------|--------|-------------|
| `core/knowledge-derive/src/features/view/mod.rs` | Create | Module declaration for view adapters |
| `core/knowledge-derive/src/features/view/tree.rs` | Create | `TreeviewAdapter` implementation |
| `core/knowledge-derive/src/lib.rs` | Modify | Add `pub mod features;` (already exists, add view module) |
| `core/knowledge-derive/src/features/mod.rs` | Modify | Add `pub mod view;` |

**Implementation notes:**

The tree view groups entities by type. Each entity type becomes a branch. Within each branch, entities are listed with their title component.

For collection support (from ADR-0018), collections appear as top-level branches containing their member entities.

```
Knowledge Graph (Tree View)

  Collection: Papers to Read (5)
    Paper: Attention Is All You Need
    Paper: BERT
    ...
  Concept (12)
    Self-Attention Mechanism
    Transformer Architecture
    ...
  Paper (8)
    ...
```

**Verification:**
- Unit test: entities grouped correctly by type
- Unit test: collections appear as branches with correct members
- Unit test: empty canonical data produces empty tree
- Unit test: filter by entity type returns only matching branches

**Exit criteria:** Tree view renders correctly from canonical data

---

### D3: Graph View Adapter

**Purpose:** Implement nodes + edges visualization from entity and relationship data

**Files:**

| File | Action | Description |
|------|--------|-------------|
| `core/knowledge-derive/src/features/view/graph.rs` | Create | `GraphViewAdapter` implementation |

**Implementation notes:**

The graph view uses `TraversalPort` to extract a bounded subgraph. When `start_entity_id` is provided in `ViewFilter`, it uses traversal to extract the subgraph. When no start entity is provided, it returns all entities and their direct relationships (bounded by `max_results`).

Nodes are labeled with entity type and title. Edges are labeled with relationship type.

**Verification:**
- Unit test: nodes and edges represent entities and relationships correctly
- Unit test: subgraph from specific entity with depth limit
- Unit test: filter by entity type and tags
- Unit test: empty graph produces empty output

**Exit criteria:** Graph view renders correctly with nodes and edges

---

### D4: Table View Adapter

**Purpose:** Implement sortable, filterable tabular display

**Files:**

| File | Action | Description |
|------|--------|-------------|
| `core/knowledge-derive/src/features/view/table.rs` | Create | `TableViewAdapter` implementation |

**Implementation notes:**

Table columns: ID, Type, Title, Tags, Created, Updated. Sorting by any column. Filtering by type, tags, and text search.

The table view queries `EntityRepository` for entities matching the filter, then extracts title and tags from components.

**Verification:**
- Unit test: correct columns and rows
- Unit test: sort by title ascending and descending
- Unit test: filter by entity type
- Unit test: filter by search query
- Unit test: empty data produces empty table

**Exit criteria:** Table view renders correctly with sorting and filtering

---

### D5: Timeline View Adapter

**Purpose:** Implement temporal ordering of entities

**Files:**

| File | Action | Description |
|------|--------|-------------|
| `core/knowledge-derive/src/features/view/timeline.rs` | Create | `TimelineViewAdapter` implementation |

**Implementation notes:**

Timeline entries are ordered by `created_at` timestamp. Each entry shows the entity title, type, and creation date. Entities without timestamps are excluded.

**Verification:**
- Unit test: entities ordered by creation time
- Unit test: entities without timestamps excluded
- Unit test: filter by entity type
- Unit test: empty data produces empty timeline

**Exit criteria:** Timeline view renders correctly in temporal order

---

### D6: View Registry and CLI Integration

**Purpose:** Wire views to CLI commands and event-driven synchronization

**Files:**

| File | Action | Description |
|------|--------|-------------|
| `core/knowledge-derive/src/features/view/mod.rs` | Modify | Add `ViewRegistry` implementation, register all 4 built-in views |
| `cli/src/main.rs` | Modify | Add `View` subcommand with `tree`, `graph`, `table`, `timeline` subcommands |
| `cli/features/prd-0003/views.feature` | Create | BDD scenarios for all 4 views |
| `cli/tests/cucumber.rs` | Modify | Add step definitions for view commands |

**CLI interface (per PRD-0003):**

```
kos view tree [--type <entity-type>]
kos view graph [--from <entity-id>] [--depth <n>] [--type <entity-type>]
kos view table [--sort <column>] [--filter <query>] [--type <entity-type>]
kos view timeline [--type <entity-type>]
```

**Event-driven synchronization:**

The `ViewRegistry` implements `notify_event()` which calls `on_event()` on all registered views. This is triggered by the existing event system (ADR-0004). Views invalidate their derived data and rebuild on next `render()`.

**Verification:**
- `cargo test --test cucumber -p knowledge-cli` passes
- BDD scenarios: tree view, graph view, table view, timeline view, subgraph view
- Manual test: import files, verify views display correctly

**Exit criteria:** All 4 views work via CLI, BDD tests pass

---

## Execution Order

```
D1 (types) -> D2 (tree) -> D3 (graph) -> D4 (table) -> D5 (timeline) -> D6 (registry + CLI)
```

D1 defines the trait. D2-D5 implement each view independently. D6 wires everything to CLI and adds event synchronization.

---

## Verification Strategy

| Level | Command | Coverage |
|-------|---------|----------|
| Unit | `cargo test -p knowledge-derive` | All 4 view adapters |
| Integration | `cargo test -p knowledge-derive --test integration_test` | View registry, event sync |
| E2E | `cargo test --test cucumber -p knowledge-cli` | CLI view commands |
| Lint | `cargo clippy -- -D warnings && cargo fmt --check` | Code quality |

---

## Exit Criteria

- [ ] `ViewAdapter` trait and all view data types in `knowledge-core`
- [ ] 4 view adapters implemented in `knowledge-derive`
- [ ] `ViewRegistry` with event-driven synchronization
- [ ] `kos view tree|graph|table|timeline` commands in CLI
- [ ] BDD tests: 5+ view scenarios
- [ ] `cargo clippy -- -D warnings` passes
- [ ] ADR-0015 updated with Implementation Notes

---

## Implementation Notes

*(Filled in during/after implementation)*
