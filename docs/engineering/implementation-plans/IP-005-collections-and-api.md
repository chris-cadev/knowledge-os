# IP-005: Phase 5 -- Collections and API Surface

**Status:** Draft
**ADR(s):** [ADR-0018](../../architecture/adrs/adr-0018.md) (Collection Entity for Curated Groups)
**PRD(s):** [PRD-0003](../prds/prd-0003-graph-exploration-and-plugins.md) (Collection management, API surface)
**Estimated effort:** ~4 days

---

## Context

ADR-0018 defined Collections as first-class entities with many-to-many membership. Collections are stored in dedicated tables and used by the tree view for hierarchical grouping. This phase adds collection management to storage and CLI, then updates the API surface to expose all new capabilities (traversal, views, plugins, semantic search, collections).

---

## Deliverables

### D1: Collection Entity and Repository Trait

**Purpose:** Define collection types and repository interface

**Files:**

| File | Action | Description |
|------|--------|-------------|
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

| File | Action | Description |
|------|--------|-------------|
| `core/knowledge-storage/src/adapters/sqlite/mod.rs` | Modify | Add schema migration for `collections` and `collection_members` tables, implement `CollectionRepository` |
| `core/knowledge-storage/tests/integration_test.rs` | Modify | Add collection integration tests |

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

**Implementation notes:**

Schema migration runs on `SqliteStore::new()`. Foreign keys with CASCADE delete ensure orphaned memberships are cleaned up when entities or collections are deleted.

**Verification:**
- Unit test: collection CRUD operations
- Unit test: add_member creates membership record
- Unit test: add_member rejects duplicate membership
- Unit test: remove_member deletes membership record
- Unit test: get_members returns all entities in collection
- Unit test: get_entity_collections returns all collections containing entity
- Unit test: is_member returns correct boolean
- Integration test: cascade delete on entity archive
- Integration test: entity in multiple collections appears in each member list

**Exit criteria:** Collection storage works, all tests pass

---

### D3: CLI Collection Commands

**Purpose:** Expose collection management via CLI

**Files:**

| File | Action | Description |
|------|--------|-------------|
| `cli/src/main.rs` | Modify | Add `Collection` subcommand with `create`, `list`, `add`, `remove`, `members` subcommands |
| `cli/features/prd-0003/collections.feature` | Create | BDD scenarios for collection management |
| `cli/tests/cucumber.rs` | Modify | Add step definitions for collection commands |

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

  Scenario: Create collection
    When I run "kos collection create 'Papers to Read' --description 'Research papers'"
    Then I should see a collection created with the given title and description

  Scenario: List collections
    Given I have collections
    When I run "kos collection list"
    Then I should see all collections with their titles and member counts

  Scenario: Add entity to collection
    Given I have a collection and an entity
    When I run "kos collection add <collection-id> <entity-id>"
    Then the entity should be a member of the collection

  Scenario: Remove entity from collection
    Given I have a collection with an entity as member
    When I run "kos collection remove <collection-id> <entity-id>"
    Then the entity should not be a member of the collection

  Scenario: List collection members
    Given I have a collection with entities
    When I run "kos collection members <collection-id>"
    Then I should see all entities in the collection
```

**Verification:**
- `cargo test --test cucumber -p knowledge-cli` passes
- BDD scenarios: create, list, add, remove, members

**Exit criteria:** Collection CLI commands work, BDD tests pass

---

### D4: Tree View Collection Integration

**Purpose:** Update tree view to display collections as branches

**Files:**

| File | Action | Description |
|------|--------|-------------|
| `core/knowledge-derive/src/features/view/tree.rs` | Modify | Add collection branches to tree view output |

**Implementation notes:**

The tree view now displays collections as top-level branches containing their member entities. Collections appear before type-based branches:

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

An entity may appear in multiple collections simultaneously -- the entity is not duplicated, it is projected into each collection view.

**Verification:**
- Unit test: collections appear as branches with correct members
- Unit test: entity in multiple collections appears in each
- Unit test: collection branch shows correct member count
- Unit test: filter by entity type still works with collections

**Exit criteria:** Tree view integrates collections correctly

---

### D5: API Surface Update

**Purpose:** Update the REST API to expose traversal, views, plugins, semantic search, and collections

**Files:**

| File | Action | Description |
|------|--------|-------------|
| `api/src/bin/knowledge-api.rs` | Modify | Add new API endpoints |
| `api/src/features/entity/mod.rs` | Modify | Add entity-related endpoints |
| `docs/engineering/api-specification.md` | Modify | Update API specification with new endpoints |

**New endpoints (per PRD-0003 and existing API patterns):**

```bash
# Traversal
GET /v1/entities/{id}/traverse?depth=2&direction=outgoing&type=references

# Views
GET /v1/views/tree?type=paper
GET /v1/views/graph?from={id}&depth=2
GET /v1/views/table?sort=title&filter=transformer
GET /v1/views/timeline

# Plugins
GET /v1/plugins
GET /v1/plugins/{name}

# Semantic Search
GET /v1/search?q=machine+learning&mode=hybrid

# Collections
POST /v1/collections
GET /v1/collections
GET /v1/collections/{id}
PUT /v1/collections/{id}
DELETE /v1/collections/{id}
POST /v1/collections/{id}/members
DELETE /v1/collections/{id}/members/{entity-id}
GET /v1/collections/{id}/members
```

**Verification:**
- Manual test: each endpoint responds correctly
- Existing API tests pass (no regression)

**Exit criteria:** API endpoints work, specification updated

---

## Execution Order

```
D1 (types) -> D2 (SQLite) -> D3 (CLI) -> D4 (tree view) -> D5 (API)
```

D1 defines types. D2 implements storage. D3 wires CLI. D4 integrates with tree view. D5 updates the API surface.

---

## Verification Strategy

| Level | Command | Coverage |
|-------|---------|----------|
| Unit | `cargo test -p knowledge-storage` | Collection storage operations |
| Integration | `cargo test -p knowledge-derive --test integration_test` | Tree view with collections |
| E2E | `cargo test --test cucumber -p knowledge-cli` | CLI collection commands |
| API | Manual testing with curl/httpie | REST API endpoints |
| Lint | `cargo clippy -- -D warnings && cargo fmt --check` | Code quality |

---

## Exit Criteria

- [ ] `Collection` struct and `CollectionRepository` trait in `knowledge-core`
- [ ] `CollectionRepository` implemented for `SqliteStore`
- [ ] `collections` and `collection_members` tables created via migration
- [ ] `kos collection create|list|add|remove|members` commands
- [ ] Tree view displays collections as branches
- [ ] API endpoints for all new capabilities
- [ ] BDD tests: 5+ collection scenarios
- [ ] `cargo clippy -- -D warnings` passes
- [ ] ADR-0018 updated with Implementation Notes
- [ ] API specification updated

---

## Implementation Notes

*(Filled in during/after implementation)*
