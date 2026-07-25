# IP-001: Phase 1 -- Graph Traversal

**Status:** Draft
**ADR(s):** [ADR-0014](../../architecture/adrs/adr-0014.md) (Bounded Graph Traversal via Recursive CTE)
**PRD(s):** [PRD-0003](../prds/prd-0003-graph-exploration-and-plugins.md) (US1: Navigate entity relationships)
**Estimated effort:** ~3 days

---

## Context

ADR-0014 chose recursive CTEs in SQLite for graph traversal, with a `TraversalPort` trait in `knowledge-core` and implementation in `knowledge-storage`. The CLI needs a `kos traverse` command. This phase establishes the foundation that US2-US5 depend on.

The current `SqliteStore` (`knowledge-storage/src/adapters/sqlite/mod.rs`, ~1948 lines) already implements `RelationshipRepository` with `by_source()` and `by_target()` -- traversal builds on these existing query patterns.

---

## Deliverables

### D1: Traversal Types and Port Trait

**Purpose:** Define the graph traversal types and port interface in `knowledge-core`

**Files:**

| File                                   | Action | Description                                                                                               |
| -------------------------------------- | ------ | --------------------------------------------------------------------------------------------------------- |
| `core/knowledge-core/src/ports/mod.rs` | Modify | Add `TraversalPort` trait, `TraversalQuery`, `TraversalResult`, `TraversalConfig`, `TraversalError` types |

**New types:**

```rust
pub struct TraversalQuery {
    pub start_id: Uuid,
    pub direction: TraversalDirection,
    pub max_depth: Option<u32>,
    pub max_results: Option<usize>,
    pub relationship_type: Option<RelationshipType>,
    pub entity_type_filter: Option<EntityType>,
}

pub enum TraversalDirection { Outgoing, Incoming, Both }

pub struct TraversalResult {
    pub path: Vec<Uuid>,
    pub edges: Vec<TraversalEdge>,
    pub depth: u32,
}

pub struct TraversalEdge {
    pub source_id: Uuid,
    pub target_id: Uuid,
    pub relationship_type: RelationshipType,
}

pub struct TraversalConfig {
    pub default_max_depth: u32,
    pub default_max_results: usize,
}

#[derive(Debug, thiserror::Error)]
pub enum TraversalError {
    #[error("Start entity not found: {0}")]
    StartNotFound(Uuid),
    #[error("Traversal limit exceeded: {limit} results")]
    LimitExceeded { limit: usize },
    #[error("Storage error: {0}")]
    Storage(#[from] StorageError),
}

#[async_trait]
pub trait TraversalPort {
    async fn traverse(
        &self,
        query: &TraversalQuery,
        config: &TraversalConfig,
    ) -> Result<Vec<TraversalResult>, TraversalError>;
}
```

**Verification:**
- `cargo check -p knowledge-core` compiles
- `cargo test -p knowledge-core` passes (no regressions)

**Exit criteria:** Types compile, existing tests pass

---

### D2: Recursive CTE Traversal Implementation

**Purpose:** Implement `TraversalPort` for `SqliteStore` using recursive CTEs

**Files:**

| File                                                | Action | Description                                                       |
| --------------------------------------------------- | ------ | ----------------------------------------------------------------- |
| `core/knowledge-storage/src/adapters/sqlite/mod.rs` | Modify | Add `impl TraversalPort for SqliteStore` with recursive CTE query |
| `core/knowledge-storage/tests/integration_test.rs`  | Modify | Add traversal integration tests                                   |

**Implementation notes:**

The recursive CTE follows ADR-0014's pattern. For outgoing traversal:

```sql
WITH RECURSIVE traversal AS (
    SELECT e.id, 0 as depth, e.id as path
    FROM entities e
    WHERE e.id = ?1 AND e.is_active = 1
    UNION
    SELECT r.target_id, t.depth + 1, t.path || ',' || r.target_id
    FROM relationships r
    JOIN traversal t ON r.source_id = t.id
    JOIN entities e ON r.target_id = e.id
    WHERE t.depth < ?2
      AND r.is_active = 1
      AND e.is_active = 1
      AND NOT (',' || t.path || ',') LIKE ('%,' || r.target_id || ',%')
)
SELECT DISTINCT id, depth FROM traversal
ORDER BY depth
LIMIT ?3
```

Direction filter:
- **Outgoing:** `r.source_id = t.id` (default)
- **Incoming:** `r.target_id = t.id` (reverse)
- **Both:** `UNION` of outgoing and incoming queries

Relationship type filter: add `AND r.relationship_type = ?4` to recursive step.
Entity type filter: add `AND e.entity_type = ?5` to both base and recursive steps.

Cycle detection: path tracking with `NOT LIKE` check on accumulated path string.

**Verification:**
- `cargo test -p knowledge-storage` passes
- New integration tests cover all 6 test fixtures from ADR-0014:
  1. Chain (A -> B -> C -> D)
  2. Tree (A -> {B, C}, B -> {D, E}, C -> {F})
  3. Cycle (A -> B -> C -> A)
  4. Diamond (A -> {B, C}, B -> D, C -> D)
  5. Bidirectional (A <-> B)
  6. Disconnected (A -> B, C -> D)
- Additional tests: depth limiting, relationship type filter, entity type filter, start-not-found error

**Exit criteria:** Recursive CTE works for all 6 graph structures, all tests pass

---

### D3: CLI `kos traverse` Command

**Purpose:** Expose graph traversal via CLI

**Files:**

| File                                      | Action | Description                                                     |
| ----------------------------------------- | ------ | --------------------------------------------------------------- |
| `cli/src/main.rs`                         | Modify | Add `Traverse` subcommand to `Commands` enum, implement handler |
| `cli/features/prd-0003/traversal.feature` | Create | BDD scenarios for traversal                                     |
| `cli/tests/cucumber.rs`                   | Modify | Add step definitions for traversal                              |

**CLI interface (per PRD-0003):**

```
kos traverse <entity-id> --depth <n> --direction <outgoing|incoming|both> [--type <rel-type>] [--entity-type <type>]
```

**Output format (per PRD-0003 example):**

```
$ kos traverse abc123 --depth 2

Entity: "Attention Is All You Need" (Paper)
  Hop 1:
    -> references -> "Self-Attention Mechanism" (Concept)
    -> authored_by -> "Vaswani" (Person)
    -> references -> "Neural Machine Translation" (Paper)
  Hop 2:
    "Self-Attention Mechanism" -> related_to -> "Transformer Architecture" (Concept)
    "Vaswani" -> belongs_to -> "Google Brain" (Organization)
    "Neural Machine Translation" -> references -> "Sequence to Sequence" (Paper)

Total: 6 entities within 2 hops
```

**BDD scenarios:**

```gherkin
Feature: Graph Traversal
  As a knowledge worker
  I want to explore connections between entities
  So that I can understand how concepts relate to each other

  Scenario: Basic outgoing traversal
    Given I have a chain of entities A -> B -> C
    When I run "kos traverse <A> --depth 2"
    Then I should see entities B and C
    And I should see the relationships between them

  Scenario: Bidirectional traversal
    Given I have entities A <-> B
    When I run "kos traverse <A> --depth 1 --direction both"
    Then I should see entity B

  Scenario: Depth limiting
    Given I have a chain of entities A -> B -> C -> D
    When I run "kos traverse <A> --depth 1"
    Then I should see entity B
    And I should not see entity C

  Scenario: Relationship type filter
    Given I have entities with different relationship types
    When I run "kos traverse <A> --depth 2 --type references"
    Then I should only see entities connected by "references" relationships

  Scenario: Entity type filter
    Given I have entities of different types
    When I run "kos traverse <A> --depth 2 --entity-type concept"
    Then I should only see entities of type "concept"

  Scenario: Nonexistent entity error
    When I run "kos traverse nonexistent-id --depth 2"
    Then I should see an error "Start entity not found"
```

**Verification:**
- `cargo test --test cucumber -p knowledge-cli` passes
- All 6 BDD scenarios pass
- Manual test with real data: `kos import` a few files, then traverse between them

**Exit criteria:** CLI command works end-to-end, BDD tests pass

---

## Execution Order

```
D1 (types/port) -> D2 (SQLite impl) -> D3 (CLI command)
```

D1 is pure type definitions -- no behavior changes. D2 adds the actual recursive CTE. D3 wires it to the CLI.

---

## Verification Strategy

| Level | Command                                            | Coverage                                     |
| ----- | -------------------------------------------------- | -------------------------------------------- |
| Unit  | `cargo test -p knowledge-core`                     | Types compile, no regressions                |
| Unit  | `cargo test -p knowledge-storage`                  | Traversal implementation, 6 graph structures |
| E2E   | `cargo test --test cucumber -p knowledge-cli`      | CLI behavior, 6 BDD scenarios                |
| Lint  | `cargo clippy -- -D warnings && cargo fmt --check` | Code quality                                 |

---

## Exit Criteria

- [ ] `TraversalPort` trait and types in `knowledge-core/src/ports/mod.rs`
- [ ] `TraversalPort` implemented for `SqliteStore` with recursive CTE
- [ ] `kos traverse` command in CLI with `--depth`, `--direction`, `--type`, `--entity-type` flags
- [ ] Integration tests: 6 graph structures + edge cases
- [ ] BDD tests: 6 traversal scenarios
- [ ] `cargo clippy -- -D warnings` passes
- [ ] ADR-0014 updated with Implementation Notes

---

## Implementation Notes

*(Filled in during/after implementation)*
