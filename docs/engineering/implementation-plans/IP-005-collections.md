# IP-005: Phase 5 -- Collections

**Status:** Complete
**ADR(s):** [ADR-0018](../../architecture/adrs/adr-0018.md) (Collection Entity for Curated Groups)
**PRD(s):** [PRD-0003](../prds/prd-0003-graph-exploration-and-plugins.md) (Collection management)
**Estimated effort:** ~3 days
**Actual effort:** ~2 days

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

**Status:** Complete

**Files:**

| File                                   | Action | Description                                           |
| -------------------------------------- | ------ | ----------------------------------------------------- |
| `core/knowledge-core/src/ports/mod.rs` | Modify | Expand `Collection` struct and `CollectionRepository` trait |

**Implemented types (follows codebase conventions):**

```rust
pub struct Collection {
    pub id: Uuid,
    pub name: String,
    pub description: Option<String>,
    pub created_at: chrono::DateTime<chrono::Utc>,
    pub updated_at: chrono::DateTime<chrono::Utc>,
}

#[async_trait]
pub trait CollectionRepository: Send + Sync {
    async fn create(&self, collection: Collection) -> Result<Collection, StorageError>;
    async fn get(&self, id: Uuid) -> Result<Option<Collection>, StorageError>;
    async fn update(&self, collection: Collection) -> Result<Collection, StorageError>;
    async fn delete(&self, id: Uuid) -> Result<(), StorageError>;
    async fn list(&self) -> Result<Vec<Collection>, StorageError>;

    async fn add_member(&self, collection_id: Uuid, entity_id: Uuid) -> Result<(), StorageError>;
    async fn remove_member(&self, collection_id: Uuid, entity_id: Uuid) -> Result<(), StorageError>;
    async fn get_members(&self, collection_id: Uuid) -> Result<Vec<Entity>, StorageError>;
    async fn get_entity_collections(&self, entity_id: Uuid) -> Result<Vec<Collection>, StorageError>;
    async fn is_member(&self, collection_id: Uuid, entity_id: Uuid) -> Result<bool, StorageError>;
}
```

**Deviation from plan:** The plan specified `String` IDs and `title` field. The existing codebase uses `Uuid` IDs and `name` field. Implementation followed existing codebase conventions for consistency.

**Verification:**
- `cargo check -p knowledge-core` compiles
- `cargo test -p knowledge-core` passes

**Exit criteria:** Collection types compile — ✅

---

### D2: SQLite Collection Storage

**Purpose:** Implement `CollectionRepository` for `SqliteStore`

**Status:** Complete

**Files:**

| File                                                | Action | Description                                                                                              |
| --------------------------------------------------- | ------ | -------------------------------------------------------------------------------------------------------- |
| `core/knowledge-storage/src/adapters/sqlite/mod.rs` | Modify | Add `PRAGMA foreign_keys = ON`, schema migration, implement `CollectionRepository` for `SqliteStore`     |
| `core/knowledge-storage/tests/integration_test.rs`  | Modify | Add 10 collection integration tests                                                                     |

**Schema (per ADR-0018, with revisions):**

```sql
CREATE TABLE IF NOT EXISTS collections (
    id TEXT PRIMARY KEY,
    name TEXT NOT NULL,
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
    FOREIGN KEY (entity_id) REFERENCES entities(id)
);
```

**Foreign key enforcement:** `PRAGMA foreign_keys = ON` is executed in `SqliteStore::new()` after opening the connection. This is required for `ON DELETE CASCADE` to work on the `collection_members` table.

**Cascade delete behavior:** When a collection is deleted, its membership records are also deleted via `ON DELETE CASCADE` on `collection_id`. Entity archive uses application-level cleanup (soft-delete invariant preserved).

**Duplicate membership handling:** `add_member` uses a plain `INSERT` (not `INSERT OR IGNORE`). On UNIQUE constraint violation, the error is caught and mapped to `StorageError::Internal("Entity {entity_id} is already a member of collection {collection_id}")`. The CLI handler checks for "already" in the message to provide a user-friendly error.

**Implementation notes:**

- Schema migration runs on `SqliteStore::new()` before any queries
- `parse_collection` helper converts SQLite rows to `Collection` structs
- All 10 `CollectionRepository` methods implemented: `create`, `get`, `update`, `delete`, `list`, `add_member`, `remove_member`, `get_members`, `get_entity_collections`, `is_member`

**Verification:**
- 10 collection integration tests covering CRUD, membership, cascade delete, duplicate rejection, multi-collection membership
- `cargo test -p knowledge-storage` passes

**Exit criteria:** Collection storage works, all tests pass — ✅

---

### D3: CLI Collection Commands

**Purpose:** Expose collection management via CLI

**Status:** Complete

**Files:**

| File                                        | Action | Description                                                                               |
| ------------------------------------------- | ------ | ----------------------------------------------------------------------------------------- |
| `cli/src/main.rs`                           | Modify | Add `Collection` subcommand with `create`, `list`, `add`, `remove`, `members`, `delete`  |
| `cli/features/prd-0003/collections.feature` | Create | 12 BDD scenarios for collection management                                               |
| `cli/tests/cucumber.rs`                     | Modify | Add step definitions for collection commands, `last_collection_id` state, UUID extraction  |

**CLI interface (follows codebase conventions):**

```
kos collection create <name> [--description <description>]
kos collection list
kos collection add <collection-id> <entity-id>
kos collection remove <collection-id> <entity-id>
kos collection members <collection-id>
kos collection delete <collection-id>
```

**Deviation from plan:** The plan included `update` subcommand but not `delete`. Implementation includes `delete` (with cascade member cleanup) but not `update`. The plan also showed multi-word names like `"Papers to Read"` but the `I run {string}` BDD step uses `split_whitespace()` which splits multi-word names into separate args. BDD scenarios use single-word names (e.g., `Papers_to_Read`) to avoid this limitation.

**BDD scenarios (12 total):**

1. Create a collection with description
2. Create a collection without description
3. List collections shows count
4. List collections when empty
5. Add entity to collection
6. Duplicate member is rejected
7. Remove entity from collection
8. Collection members listed
9. Empty collection shows empty message
10. Delete collection cascades members
11. Add entity to nonexistent collection
12. Entity appears in multiple collections

**Verification:**
- `cargo test --test cucumber -p knowledge-cli` — 90 scenarios, 544 steps, all pass
- `cargo clippy -- -D warnings` passes
- `cargo fmt --check` passes

**Exit criteria:** Collection CLI commands work, BDD tests pass — ✅

---

### D4: Tree View Collection Integration

**Purpose:** Update tree view to display collections as branches

**Status:** Complete (pre-existing implementation + CLI wiring)

**Files:**

| File                                              | Action | Description                                 |
| ------------------------------------------------- | ------ | ------------------------------------------- |
| `core/knowledge-derive/src/features/view/tree.rs` | No change | Already implemented in IP-002 D2          |
| `cli/src/main.rs`                                 | Modify | Pass `Some(...)` collection repo instead of `None` |

**Implementation notes:**

The tree view adapter already supported collections via the `collection_repo: Option<Box<dyn CollectionRepository>>` parameter (implemented in IP-002 D2). The only change needed was wiring: the CLI's `create_view_registry` and `cmd_view` functions now pass `Some(Box::new(StoreWrapper(store.clone())))` instead of `None`.

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
- 6 tree view unit tests pass (including `test_collection_repo_some_adds_collection_branches` and `test_collection_repo_none_produces_tree_without_collections`)
- `cargo test --workspace` passes

**Exit criteria:** Tree view integrates collections correctly — ✅

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
| Unit        | `cargo test -p knowledge-derive tree`                    | Tree view with collections    |
| BDD         | `cargo test --test cucumber -p knowledge-cli`            | CLI collection commands       |
| Full        | `cargo test --workspace`                                 | All tests across all crates   |
| Lint        | `cargo clippy --all-targets --all-features -- -D warnings` | Code quality              |
| Format      | `cargo fmt --check`                                      | Code formatting               |

---

## Exit Criteria

- [x] `Collection` struct and `CollectionRepository` trait in `knowledge-core`
- [x] `CollectionRepository` implemented for `SqliteStore`
- [x] `collections` and `collection_members` tables created via migration
- [x] `PRAGMA foreign_keys = ON` in `SqliteStore::new()`
- [x] `kos collection create|list|add|remove|members|delete` commands
- [x] Tree view displays collections as branches
- [x] BDD tests: 12 collection scenarios
- [x] `cargo clippy --all-targets --all-features -- -D warnings` passes
- [x] `cargo fmt --check` passes
- [x] ADR-0018 updated with Implementation Notes

---

## Deviations from Plan

### 1. `String` IDs → `Uuid` IDs

The plan specified `String` IDs and `title` field (per ADR-0018). The existing codebase uses `Uuid` IDs and `name` field throughout. Implementation followed existing codebase conventions for consistency. All `CollectionRepository` methods accept/return `Uuid` instead of `&str`.

### 2. `title` field → `name` field

The plan specified `title` as the collection name field. The codebase consistently uses `name` for entity identification (e.g., `Collection { id: Uuid, name: String, ... }`). Implementation used `name` to match.

### 3. `chrono::DateTime<Utc>` timestamps

The plan specified `String` timestamps. The codebase uses `chrono::DateTime<chrono::Utc>` throughout. Implementation matched the codebase convention.

### 4. `StorageError::Internal` for duplicate detection

The plan assumed a `StorageError::AlreadyExists` variant. The `StorageError` enum only has `NotFound` and `Internal`. Duplicate membership is detected via `StorageError::Internal` with a descriptive message containing "already a member". The CLI checks for this string to provide a user-friendly error.

### 5. `add_member` uses plain `INSERT` instead of `INSERT OR IGNORE`

The original implementation used `INSERT OR IGNORE` which silently ignored duplicates. The plan required duplicate detection. Changed to plain `INSERT` which triggers a UNIQUE constraint violation, caught and mapped to `StorageError::Internal`.

### 6. No `update` subcommand

The plan did not explicitly list `update` in the CLI interface but the trait includes it. The CLI implementation includes `delete` (which the plan omitted) but not `update`. This can be added in a follow-up.

### 7. BDD collection names must be single-word

The `I run {string}` BDD step uses `split_whitespace()` which splits multi-word names into separate args for clap. BDD scenarios use underscore-separated names (e.g., `Papers_to_Read`) or single-word names. This is a known BDD infrastructure limitation, not a product limitation — the CLI itself handles multi-word names when properly quoted.

---

## Implementation Notes

### Files Modified (6 modified + 1 new)

| File | Lines | Description |
|------|-------|-------------|
| `cli/src/main.rs` | +217 | Collection subcommand, `CollectionRepository` impl for `StoreWrapper`, tree view wiring |
| `cli/tests/cucumber.rs` | +87 | Collection step definitions, `last_collection_id` state, UUID extraction helper |
| `cli/features/prd-0003/collections.feature` | new | 12 BDD scenarios |
| `core/knowledge-core/src/ports/mod.rs` | +82 | Expanded `Collection` struct and `CollectionRepository` trait |
| `core/knowledge-derive/src/features/view/tree.rs` | +40 | Updated `MockCollectionRepo` in tests |
| `core/knowledge-storage/src/adapters/sqlite/mod.rs` | +245 | `CollectionRepository` impl for `SqliteStore`, schema migration, `PRAGMA foreign_keys` |
| `core/knowledge-storage/tests/integration_test.rs` | +349 | 10 collection integration tests |

### Test Results

- **BDD:** 90 scenarios, 544 steps — all pass
- **Storage unit:** 31 tests — all pass
- **Storage integration:** 43 tests (10 new collection tests) — all pass
- **Tree view:** 6 tests (2 collection-specific) — all pass
- **Full workspace:** All tests pass
- **Clippy:** Clean (no warnings)
- **Format:** Clean
