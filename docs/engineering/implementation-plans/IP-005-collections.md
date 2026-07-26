# IP-005: Phase 5 -- Collections

**Status:** Draft
**ADR(s):** [ADR-0018](../../architecture/adrs/adr-0018.md) (Collection Entity for Curated Groups)
**PRD(s):** [PRD-0003](../prds/prd-0003-graph-exploration-and-plugins.md) (Collection management)
**Estimated effort:** ~3 days

---

## Context

ADR-0018 defined Collections as first-class entities with many-to-many membership. Collections are stored in dedicated tables and used by the tree view for hierarchical grouping. This phase adds collection management to storage and CLI, then integrates with the tree view.

**Prerequisites:**
- IP-001 (Graph Traversal) is complete
- IP-002 (View Projections) D2 is complete — `TreeViewAdapter` exists with `collection_repo: Option<Box<dyn CollectionRepository>>` parameter

---

## Deliverables

### D1: Collection Entity and Repository Trait

**Purpose:** Define collection types and repository interface

**Files:**

| File                                   | Action | Description                                           |
| -------------------------------------- | ------ | ----------------------------------------------------- |
| `core/knowledge-core/src/ports/mod.rs` | Modify | Add `CollectionRepository` trait, `Collection` struct |

**New types (per ADR-0018):**

```rust
pub struct Collection {
    pub id: String,
    pub title: String,
    pub description: Option<String>,
    pub created_at: String,
    pub updated_at: String,
}

#[async_trait]
pub trait CollectionRepository: Send + Sync {
    async fn create(&self, collection: Collection) -> Result<Collection, StorageError>;
    async fn get(&self, id: &str) -> Result<Option<Collection>, StorageError>;
    async fn update(&self, collection: Collection) -> Result<Collection, StorageError>;
    async fn delete(&self, id: &str) -> Result<(), StorageError>;
    async fn list(&self) -> Result<Vec<Collection>, StorageError>;

    async fn add_member(&self, collection_id: &str, entity_id: &str) -> Result<(), StorageError>;
    async fn remove_member(&self, collection_id: &str, entity_id: &str) -> Result<(), StorageError>;
    async fn get_members(&self, collection_id: &str) -> Result<Vec<Entity>, StorageError>;
    async fn get_entity_collections(&self, entity_id: &str) -> Result<Vec<Collection>, StorageError>;
    async fn is_member(&self, collection_id: &str, entity_id: &str) -> Result<bool, StorageError>;
}
```

**Verification:**
- `cargo check -p knowledge-core` compiles
- `cargo test -p knowledge-core` passes

**Exit criteria:** Collection types compile

---

### D2: SQLite Collection Storage

**Purpose:** Implement `CollectionRepository` for `SqliteStore`

**Files:**

| File                                                | Action | Description                                                                                              |
| --------------------------------------------------- | ------ | -------------------------------------------------------------------------------------------------------- |
| `core/knowledge-storage/src/adapters/sqlite/mod.rs` | Modify | Add schema migration for `collections` and `collection_members` tables, implement `CollectionRepository` |
| `core/knowledge-storage/tests/integration_test.rs`  | Modify | Add collection integration tests                                                                         |

**Schema (per ADR-0018):**

```sql
CREATE TABLE IF NOT EXISTS collections (
    id TEXT PRIMARY KEY,
    title TEXT NOT NULL,
    description TEXT,
    created_at TEXT NOT NULL,
    updated_at TEXT NOT NULL
);

CREATE TABLE IF NOT EXISTS collection_members (
    collection_id TEXT NOT NULL,
    entity_id TEXT NOT NULL,
    added_at TEXT NOT NULL,
    PRIMARY KEY (collection_id, entity_id),
    FOREIGN KEY (collection_id) REFERENCES collections(id) ON DELETE CASCADE,
    FOREIGN KEY (entity_id) REFERENCES entities(id) ON DELETE CASCADE
);
```

**Foreign key enforcement:** SQLite does not enforce foreign keys by default. The `SqliteStore::new()` method must execute `PRAGMA foreign_keys = ON` after opening the connection. This is critical for CASCADE DELETE to work.

**Cascade delete behavior:** When an entity is archived (soft-deleted via `is_active = 0`), its collection memberships should also be cleaned up. Two approaches:

1. **Application-level cleanup:** On entity archive, call `remove_member` for all collections containing the entity. This is explicit and auditable.
2. **Foreign key cascade:** `ON DELETE CASCADE` on the foreign key. This requires hard deletes on the `entities` table, which contradicts the domain model's soft-delete invariant.

**Chosen approach:** Application-level cleanup. The `collection_members` table uses foreign keys for referential integrity but NOT cascade delete on entity archive. Instead, the entity archive operation calls `CollectionRepository::remove_member()` for all collections. This preserves the soft-delete invariant while keeping referential integrity.

```sql
-- Revised schema: no CASCADE on entity FK
CREATE TABLE IF NOT EXISTS collection_members (
    collection_id TEXT NOT NULL,
    entity_id TEXT NOT NULL,
    added_at TEXT NOT NULL,
    PRIMARY KEY (collection_id, entity_id),
    FOREIGN KEY (collection_id) REFERENCES collections(id) ON DELETE CASCADE,
    FOREIGN KEY (entity_id) REFERENCES entities(id)
);
```

**Implementation notes:**

Schema migration runs on `SqliteStore::new()`. The `ON DELETE CASCADE` on `collection_id` ensures that when a collection is deleted, its membership records are also deleted. Entity archive uses application-level cleanup.

**Verification:**
- Unit test: collection CRUD operations
- Unit test: add_member creates membership record
- Unit test: add_member rejects duplicate membership
- Unit test: remove_member deletes membership record
- Unit test: get_members returns all entities in collection
- Unit test: get_entity_collections returns all collections containing entity
- Unit test: is_member returns correct boolean
- Integration test: deleting collection removes its memberships (CASCADE)
- Integration test: archiving entity removes it from collection memberships (application-level)
- Integration test: entity in multiple collections appears in each member list

**Exit criteria:** Collection storage works, all tests pass

---

### D3: CLI Collection Commands

**Purpose:** Expose collection management via CLI

**Files:**

| File                                        | Action | Description                                                                               |
| ------------------------------------------- | ------ | ----------------------------------------------------------------------------------------- |
| `cli/src/main.rs`                           | Modify | Add `Collection` subcommand with `create`, `list`, `add`, `remove`, `members` subcommands |
| `cli/features/prd-0003/collections.feature` | Create | BDD scenarios for collection management                                                   |
| `cli/tests/cucumber.rs`                     | Modify | Add step definitions for collection commands                                              |

**CLI interface (per ADR-0018):**

```
kos collection create "Papers to Read" --description "Research papers for literature review"
kos collection list
kos collection add <collection-id> <entity-id>
kos collection remove <collection-id> <entity-id>
kos collection members <collection-id>
```

**BDD scenarios:**

```gherkin
Feature: Collection Management
  As a knowledge worker
  I want to group related entities
  So that I can organize my knowledge

  Background:
    Given an empty database

  Scenario: Create collection
    When I run "kos collection create 'Papers to Read' --description 'Research papers'"
    Then the output should contain "Papers to Read"

  Scenario: List collections
    Given I have a collection "Papers to Read"
    When I run "kos collection list"
    Then the output contains "Papers to Read"

  Scenario: Add entity to collection
    Given I have a collection "Papers to Read"
    And I have an entity "Attention Is All You Need"
    When I run "kos collection add <collection-id> <entity-id>"
    Then the entity should be a member of the collection

  Scenario: Remove entity from collection
    Given I have a collection "Papers to Read" with entity "Attention Is All You Need"
    When I run "kos collection remove <collection-id> <entity-id>"
    Then the entity should not be a member of the collection

  Scenario: List collection members
    Given I have a collection "Papers to Read" with entities
    When I run "kos collection members <collection-id>"
    Then the output should contain all entities in the collection
```

**Verification:**
- `cargo test --test cucumber -p knowledge-cli` passes
- BDD scenarios: create, list, add, remove, members

**Exit criteria:** Collection CLI commands work, BDD tests pass

---

### D4: Tree View Collection Integration

**Purpose:** Update tree view to display collections as branches

**Files:**

| File                                              | Action | Description                                 |
| ------------------------------------------------- | ------ | ------------------------------------------- |
| `core/knowledge-derive/src/features/view/tree.rs` | Modify | Add collection branches to tree view output |

**Implementation notes:**

The tree view adapter's `collection_repo` parameter (from IP-002 D2) is now `Some(...)` instead of `None`. The tree view queries `CollectionRepository::list()` and `CollectionRepository::get_members()` to build collection branches.

Collections appear as top-level branches before type-based branches:

```
Knowledge Graph (Tree View)

  Collection: Papers to Read (5)
    Paper: Attention Is All You Need
    Paper: BERT
    ...
  Concept (12)
    Self-Attention Mechanism
    ...
```

An entity may appear in multiple collections simultaneously — the entity is not duplicated, it is projected into each collection view.

**Verification:**
- Unit test: collections appear as branches with correct members
- Unit test: entity in multiple collections appears in each
- Unit test: collection branch shows correct member count
- Unit test: filter by entity type still works with collections
- Unit test: collection_repo=None produces tree without collection branches (existing behavior)

**Exit criteria:** Tree view integrates collections correctly

---

## Execution Order

```
D1 (types) -> D2 (SQLite) -> D3 (CLI) -> D4 (tree view)
```

D1 defines types. D2 implements storage. D3 wires CLI. D4 integrates with tree view (depends on IP-002 D2).

---

## Verification Strategy

| Level       | Command                                                  | Coverage                      |
| ----------- | -------------------------------------------------------- | ----------------------------- |
| Unit        | `cargo test -p knowledge-storage`                        | Collection storage operations |
| Integration | `cargo test -p knowledge-derive --test integration_test` | Tree view with collections    |
| E2E         | `cargo test --test cucumber -p knowledge-cli`            | CLI collection commands       |
| Lint        | `cargo clippy -- -D warnings && cargo fmt --check`       | Code quality                  |

---

## Exit Criteria

- [ ] `Collection` struct and `CollectionRepository` trait in `knowledge-core`
- [ ] `CollectionRepository` implemented for `SqliteStore`
- [ ] `collections` and `collection_members` tables created via migration
- [ ] `PRAGMA foreign_keys = ON` in `SqliteStore::new()`
- [ ] `kos collection create|list|add|remove|members` commands
- [ ] Tree view displays collections as branches
- [ ] BDD tests: 5+ collection scenarios
- [ ] `cargo clippy -- -D warnings` passes
- [ ] ADR-0018 updated with Implementation Notes

---

## Implementation Notes

*(Filled in during/after implementation)*
