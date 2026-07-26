# IP-002: Phase 2 -- View Projections

**Status:** Done
**ADR(s):** [ADR-0015](../../architecture/adrs/adr-0015.md) (View Projection System)
**PRD(s):** [PRD-0003](../prds/prd-0003-graph-exploration-and-plugins.md) (US2: Tree view, US3: Graph view, US4: Table view, Timeline view)
**Estimated effort:** ~5 days

---

## Context

ADR-0015 introduced `ViewAdapter` as a trait for rendering canonical data into projections. Four built-in views (tree, graph, table, timeline) implement this trait. Views are updated event-driven — when canonical data changes, views invalidate and rebuild.

This phase builds on IP-001's traversal results. The graph view and subgraph view use `TraversalPort` to extract bounded subgraphs.

**Prerequisite:** IP-001 (Graph Traversal) is complete. `TraversalPort` trait and SQLite implementation exist.

**Dependency:** This phase introduces an `EventNotifier` trait that IP-004 (Semantic Search) also depends on for embedding pipeline re-triggering.

---

## Deliverables

### D1: View Types, ViewAdapter Trait, and EventNotifier

**Purpose:** Define the view projection types, trait, and event notification infrastructure in `knowledge-core`

**Files:**

| File                                   | Action | Description                                                                                                   |
| -------------------------------------- | ------ | ------------------------------------------------------------------------------------------------------------- |
| `core/knowledge-core/src/ports/mod.rs` | Modify | Add `ViewAdapter` trait, `ViewFilter`, `ViewOutput`, `ViewRegistry`, `EventNotifier`, and all view data types |

**New types (per ADR-0015):**

```rust
// --- View Filtering ---

pub struct ViewFilter {
    pub entity_types: Option<Vec<EntityType>>,
    pub tags: Option<Vec<String>>,
    pub relationship_types: Option<Vec<RelationshipType>>,
    pub max_depth: Option<u32>,
    pub max_results: Option<usize>,
    pub start_entity_id: Option<String>,
    pub sort_by: Option<String>,
    pub sort_order: Option<SortOrder>,
    pub search_query: Option<String>,
}

pub enum SortOrder { Asc, Desc }

// --- View Output ---

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

// --- View Adapter Trait ---

#[async_trait]
pub trait ViewAdapter: Send + Sync {
    fn name(&self) -> &str;
    async fn render(&self, filter: &ViewFilter) -> Result<ViewOutput, StorageError>;
    async fn on_event(&self, event: &Event) -> Result<(), StorageError>;
}

// --- Event Notification ---

#[async_trait]
pub trait EventNotifier: Send + Sync {
    async fn notify(&self, event: &Event) -> Result<(), StorageError>;
}

// --- View Registry ---

pub struct ViewRegistry {
    views: HashMap<String, Box<dyn ViewAdapter>>,
}

impl ViewRegistry {
    pub fn new() -> Self { Self { views: HashMap::new() } }
    pub fn register(&mut self, view: Box<dyn ViewAdapter>);
    pub async fn render(&self, name: &str, filter: &ViewFilter) -> Result<ViewOutput, StorageError>;
    pub fn list_views(&self) -> Vec<String>;
}

impl EventNotifier for ViewRegistry {
    async fn notify(&self, event: &Event) -> Result<(), StorageError> {
        for view in self.views.values() {
            view.on_event(event).await?;
        }
        Ok(())
    }
}
```

**Design decisions:**

- `ViewFilter` adds `max_results: Option<usize>` (not in ADR-0015) to bound graph view output when no `start_entity_id` is provided.
- `EventNotifier` is a separate trait from `ViewAdapter` so other subsystems (IP-004 embedding pipeline) can implement it without being views.
- `ViewOutput` enum couples view types at the type level — this is acknowledged technical debt per ADR-0015. Will be replaced with `Box<dyn Any>` when view type set stabilizes.

**Verification:**
- `cargo check -p knowledge-core` compiles
- `cargo test -p knowledge-core` passes (no regressions)

**Exit criteria:** Types compile, existing tests pass

---

### D2: Tree View Adapter

**Purpose:** Implement hierarchical navigation grouped by entity type, with collection branches

**Files:**

| File                                              | Action | Description                          |
| ------------------------------------------------- | ------ | ------------------------------------ |
| `core/knowledge-derive/src/features/mod.rs`       | Modify | Add `pub mod view;`                  |
| `core/knowledge-derive/src/features/view/mod.rs`  | Create | Module declaration for view adapters |
| `core/knowledge-derive/src/features/view/tree.rs` | Create | `TreeViewAdapter` implementation     |

**Implementation notes:**

The tree view groups entities by type. Each entity type becomes a branch. Within each branch, entities are listed with their title component.

Collection support (from ADR-0018): collections appear as top-level branches containing their member entities. The tree view adapter takes a `Box<dyn CollectionRepository>` (defined in IP-005 D1) as a constructor parameter. If collections are not yet implemented, the adapter gracefully skips collection branches.

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

**Constructor signature:**

```rust
pub struct TreeViewAdapter {
    entity_repo: Box<dyn EntityRepository>,
    component_repo: Box<dyn ComponentRepository>,
    collection_repo: Option<Box<dyn CollectionRepository>>, // None until IP-005
}

impl TreeViewAdapter {
    pub fn new(
        entity_repo: Box<dyn EntityRepository>,
        component_repo: Box<dyn ComponentRepository>,
        collection_repo: Option<Box<dyn CollectionRepository>>,
    ) -> Self { ... }
}
```

**Verification:**
- Unit test: entities grouped correctly by type
- Unit test: collections appear as branches with correct members (when collection_repo is Some)
- Unit test: empty canonical data produces empty tree
- Unit test: filter by entity type returns only matching branches
- Unit test: collection_repo=None produces tree without collection branches

**Exit criteria:** Tree view renders correctly from canonical data

---

### D3: Graph View Adapter

**Purpose:** Implement nodes + edges visualization from entity and relationship data

**Files:**

| File                                               | Action | Description                       |
| -------------------------------------------------- | ------ | --------------------------------- |
| `core/knowledge-derive/src/features/view/graph.rs` | Create | `GraphViewAdapter` implementation |

**Implementation notes:**

The graph view takes a `Box<dyn TraversalPort>` (from IP-001) as a constructor parameter. When `start_entity_id` is provided in `ViewFilter`, it uses traversal to extract the subgraph. When no start entity is provided, it returns all entities and their direct relationships (bounded by `max_results` from `ViewFilter`, defaulting to 100).

Nodes are labeled with entity type and title. Edges are labeled with relationship type.

**Constructor signature:**

```rust
pub struct GraphViewAdapter {
    entity_repo: Box<dyn EntityRepository>,
    component_repo: Box<dyn ComponentRepository>,
    relationship_repo: Box<dyn RelationshipRepository>,
    traversal_port: Box<dyn TraversalPort>,
}
```

**Verification:**
- Unit test: nodes and edges represent entities and relationships correctly
- Unit test: subgraph from specific entity with depth limit
- Unit test: no start entity returns all entities with direct relationships, bounded by max_results
- Unit test: filter by entity type and tags
- Unit test: empty graph produces empty output

**Exit criteria:** Graph view renders correctly with nodes and edges

---

### D4: Table View Adapter

**Purpose:** Implement sortable, filterable tabular display

**Files:**

| File                                               | Action | Description                       |
| -------------------------------------------------- | ------ | --------------------------------- |
| `core/knowledge-derive/src/features/view/table.rs` | Create | `TableViewAdapter` implementation |

**Implementation notes:**

Table columns: ID, Type, Title, Tags, Created, Updated. Sorting by any column. Filtering by type, tags, and text search.

The table view queries `EntityRepository` for entities matching the filter, then extracts title and tags from components.

**Constructor signature:**

```rust
pub struct TableViewAdapter {
    entity_repo: Box<dyn EntityRepository>,
    component_repo: Box<dyn ComponentRepository>,
}
```

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

| File                                                  | Action | Description                          |
| ----------------------------------------------------- | ------ | ------------------------------------ |
| `core/knowledge-derive/src/features/view/timeline.rs` | Create | `TimelineViewAdapter` implementation |

**Implementation notes:**

Timeline entries are ordered by `created_at` timestamp from the `Timeline` component. Each entry shows the entity title, type, and creation date. Entities without timestamps are excluded.

**Constructor signature:**

```rust
pub struct TimelineViewAdapter {
    entity_repo: Box<dyn EntityRepository>,
    component_repo: Box<dyn ComponentRepository>,
}
```

**Verification:**
- Unit test: entities ordered by creation time
- Unit test: entities without timestamps excluded
- Unit test: filter by entity type
- Unit test: empty data produces empty timeline

**Exit criteria:** Timeline view renders correctly in temporal order

---

### D6: View Registry, CLI Integration, and Event Wiring

**Purpose:** Wire views to CLI commands and connect event notification

**Files:**

| File                                             | Action | Description                                                                 |
| ------------------------------------------------ | ------ | --------------------------------------------------------------------------- |
| `core/knowledge-derive/src/features/view/mod.rs` | Modify | Add `ViewRegistry` impl, register all 4 built-in views, add `pub use`       |
| `cli/src/main.rs`                                | Modify | Add `View` subcommand with `tree`, `graph`, `table`, `timeline` subcommands |
| `cli/features/prd-0003/views.feature`            | Create | BDD scenarios for all 4 views                                               |
| `cli/tests/cucumber.rs`                          | Modify | Add step definitions for view commands                                      |

**CLI interface (per PRD-0003):**

```
kos view tree [--type <entity-type>]
kos view graph [--from <entity-id>] [--depth <n>] [--type <entity-type>]
kos view table [--sort <column>] [--filter <query>] [--type <entity-type>]
kos view timeline [--type <entity-type>]
```

**Event wiring:**

The CLI creates a `ViewRegistry`, registers all 4 built-in views, and wraps it in an `Arc<ViewRegistry>`. After any write operation that produces an `Event`, the CLI calls `registry.notify(&event).await` to trigger view invalidation.

```
CLI write operation
  |
  Produce Event (via TransactionalWrite)
  |
  Save Event to EventLog
  |
  Notify ViewRegistry: registry.notify(&event).await
  |
  ViewRegistry calls on_event() on all registered views
  |
  Views invalidate derived data, rebuild on next render()
```

This approach avoids a full pub/sub event bus — the CLI is the single orchestrator that connects writes to view invalidation.

**BDD scenarios:**

```gherkin
Feature: View Projections
  As a knowledge worker
  I want to view my knowledge in different projections
  So that I can navigate and compare entities effectively

  Background:
    Given an empty database

  @us2
  Scenario: Tree view groups entities by type
    Given a directory with files:
      | filename    | content                              |
      | concept.md  | # Transformer\n\nType: concept       |
      | paper.md    | # Attention Is All You Need\n\nType: paper |
    When I run "kos import <directory>"
    And I run "kos view tree"
    Then the output contains "Concept"
    And the output contains "Paper"
    And the output contains "Transformer"
    And the output contains "Attention Is All You Need"

  @us3
  Scenario: Graph view displays nodes and edges
    Given a directory with files:
      | filename    | content                                       |
      | entity-a.md | # Entity A\n\nReferences [[Entity B]].        |
      | entity-b.md | # Entity B                                    |
    When I run "kos import <directory>"
    And I run "kos view graph"
    Then the output contains "Entity A"
    And the output contains "Entity B"

  @us4
  Scenario: Table view displays entities
    Given a directory with files:
      | filename    | content                              |
      | concept.md  | # Transformer\n\nType: concept       |
    When I run "kos import <directory>"
    And I run "kos view table"
    Then the output contains "Transformer"

  Scenario: Timeline view orders by creation time
    Given a directory with files:
      | filename    | content                              |
      | concept.md  | # Transformer\n\nType: concept       |
    When I run "kos import <directory>"
    And I run "kos view timeline"
    Then the output contains "Transformer"
```

**Verification:**
- `cargo test --test cucumber -p knowledge-cli` passes
- BDD scenarios: tree view, graph view, table view, timeline view
- Manual test: import files, verify views display correctly

**Exit criteria:** All 4 views work via CLI, BDD tests pass

---

## Execution Order

```
D1 (types + event notifier) -> D2 (tree) -> D3 (graph) -> D4 (table) -> D5 (timeline) -> D6 (registry + CLI)
```

D1 defines the trait and event infrastructure. D2-D5 implement each view independently (no inter-dependencies). D6 wires everything to CLI and adds event notification wiring.

---

## Verification Strategy

| Level       | Command                                                  | Coverage                          |
| ----------- | -------------------------------------------------------- | --------------------------------- |
| Unit        | `cargo test -p knowledge-derive`                         | All 4 view adapters               |
| Integration | `cargo test -p knowledge-derive --test integration_test` | View registry, event notification |
| E2E         | `cargo test --test cucumber -p knowledge-cli`            | CLI view commands                 |
| Lint        | `cargo clippy -- -D warnings && cargo fmt --check`       | Code quality                      |

---

## Exit Criteria

- [x] `ViewAdapter`, `ViewFilter`, `ViewOutput`, `EventNotifier`, `ViewRegistry` in `knowledge-core/src/ports/mod.rs`
- [x] 4 view adapters implemented in `knowledge-derive/src/features/view/`
- [x] `ViewRegistry` with event-driven notification via `EventNotifier`
- [x] `kos view tree|graph|table|timeline` commands in CLI
- [x] BDD tests: 8 view scenarios in `cli/features/prd-0003/views.feature`
- [x] `cargo clippy -- -D warnings` passes
- [x] ADR-0015 updated with Implementation Notes

---

## Implementation Notes

### Deviations from Plan

| Plan                                                                     | Actual                                                                      | Reason                                                                                                     |
| ------------------------------------------------------------------------ | --------------------------------------------------------------------------- | ---------------------------------------------------------------------------------------------------------- |
| D6: `cli/tests/cucumber.rs` needs new step definitions for view commands | No new step definitions needed                                              | Existing steps (`output contains`, `run`) covered all view scenarios.                                      |
| Graph view uses `TraversalPort` for subgraph extraction                  | Graph view directly queries `EntityRepository` and `RelationshipRepository` | Simpler implementation for MVP. `TraversalPort` integration can be added later for bounded subgraph views. |

### Status

- **D1:** ✅ View types, `ViewAdapter` trait, `EventNotifier`, `ViewRegistry` defined
- **D2:** ✅ `TreeViewAdapter` implemented with entity grouping by type
- **D3:** ✅ `GraphViewAdapter` implemented with nodes and edges
- **D4:** ✅ `TableViewAdapter` implemented with sortable columns
- **D5:** ✅ `TimelineViewAdapter` implemented with creation time ordering
- **D6:** ✅ CLI commands and event notification wiring complete

### Files Modified/Created

| File                                                  | Description                                                                                         |
| ----------------------------------------------------- | --------------------------------------------------------------------------------------------------- |
| `core/knowledge-core/src/ports/mod.rs`                | Added `ViewAdapter`, `ViewFilter`, `ViewOutput`, `EventNotifier`, `ViewRegistry`, and related types |
| `core/knowledge-derive/src/features/view/mod.rs`      | Module declaration for 4 view adapters                                                              |
| `core/knowledge-derive/src/features/view/tree.rs`     | `TreeViewAdapter` implementation                                                                    |
| `core/knowledge-derive/src/features/view/graph.rs`    | `GraphViewAdapter` implementation                                                                   |
| `core/knowledge-derive/src/features/view/table.rs`    | `TableViewAdapter` implementation                                                                   |
| `core/knowledge-derive/src/features/view/timeline.rs` | `TimelineViewAdapter` implementation                                                                |
| `cli/src/main.rs`                                     | Added `kos view tree                                                                                | graph | table | timeline` commands |
| `cli/features/prd-0003/views.feature`                 | 8 BDD scenarios for view projections                                                                |

### Test Counts

- **Unit tests:** 29 tests in `knowledge-derive` (view adapters: tree, graph, table, timeline)
- **Integration tests:** 5 tests for `ViewRegistry` (dispatch, render, event notification)
- **BDD tests:** 8 scenarios in `views.feature`

### Verification

All verification passes:
- `cargo clippy --all-targets --all-features -- -D warnings` — clean
- `cargo fmt --check` — clean
- `cargo test -p knowledge-derive` — 56 tests pass (51 unit + 5 integration)
- `cargo test --test cucumber -p knowledge-cli` — 78 scenarios, 437 steps pass
