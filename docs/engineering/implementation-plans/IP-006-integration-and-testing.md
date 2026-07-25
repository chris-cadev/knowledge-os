# IP-006: Phase 6 -- Integration and Testing

**Status:** Draft
**ADR(s):** [ADR-0014](../../architecture/adrs/adr-0014.md), [ADR-0015](../../architecture/adrs/adr-0015.md), [ADR-0016](../../architecture/adrs/adr-0016.md), [ADR-0017](../../architecture/adrs/adr-0017.md), [ADR-0018](../../architecture/adrs/adr-0018.md)
**PRD(s):** [PRD-0003](../prds/prd-0003-graph-exploration-and-plugins.md) (Phase 6: Integration + testing)
**Estimated effort:** ~3 days

---

## Context

PRD-0003 Phase 6 specifies "Integration + testing" as the final phase. After IP-001 through IP-005 deliver individual features, this phase verifies that all components work together correctly, meets performance targets, and validates the complete user experience.

**Prerequisites:** IP-001 through IP-005 are all complete.

---

## Deliverables

### D1: End-to-End Test Suite

**Purpose:** Verify complete user workflows across all PRD-0003 features

**Files:**

| File                                                      | Action | Description                                         |
| --------------------------------------------------------- | ------ | --------------------------------------------------- |
| `cli/features/prd-0003/e2e-traversal-views.feature`       | Create | Cross-feature scenarios: traverse then view results |
| `cli/features/prd-0003/e2e-search-workflows.feature`      | Create | Search workflows: keyword, semantic, hybrid         |
| `cli/features/prd-0003/e2e-plugin-lifecycle.feature`      | Create | Plugin lifecycle: list, info, failure isolation     |
| `cli/features/prd-0003/e2e-collection-navigation.feature` | Create | Collection workflows: create, add, view in tree     |

**Scenarios to cover:**

```gherkin
Feature: End-to-End Traversal and Views

  Background:
    Given an empty database

  Scenario: Traverse and view subgraph as graph
    Given a directory with interconnected files
    When I run "kos import <directory>"
    And I run "kos traverse <entity-id> --depth 2"
    And I run "kos view graph --from <entity-id> --depth 2"
    Then the graph view should contain the same entities as traversal

  Scenario: Traverse and view as table
    Given a directory with files of different types
    When I run "kos import <directory>"
    And I run "kos traverse <entity-id> --depth 1 --entity-type concept"
    And I run "kos view table --type concept"
    Then the table view should contain the traversed concepts

Feature: End-to-End Search Workflows

  Background:
    Given an empty database

  Scenario: Import, search, then traverse results
    Given a directory with files about machine learning
    When I run "kos import <directory>"
    And I run "kos search transformer"
    And I run "kos traverse <result-entity-id> --depth 1"
    Then I should see related entities

  Scenario: Semantic search finds related entities
    Given a directory with files using different terminology for same concepts
    When I run "kos import <directory>"
    And I run "kos search 'neural networks' --semantic"
    Then results should include entities about deep learning

Feature: End-to-End Collection Navigation

  Background:
    Given an empty database

  Scenario: Create collection, add entities, view in tree
    Given a directory with files
    When I run "kos import <directory>"
    And I run "kos collection create 'Research Papers'"
    And I run "kos collection add <collection-id> <entity-id-1>"
    And I run "kos collection add <collection-id> <entity-id-2>"
    And I run "kos view tree"
    Then the tree should show "Research Papers" as a branch
    And the branch should contain the added entities
```

**Verification:**
- `cargo test --test cucumber -p knowledge-cli` passes all new scenarios
- All existing BDD scenarios still pass (no regression)

**Exit criteria:** End-to-end workflows pass across all features

---

### D2: Performance Benchmarks

**Purpose:** Verify PRD-0003 non-functional requirements are met

**Files:**

| File                                          | Action | Description                                   |
| --------------------------------------------- | ------ | --------------------------------------------- |
| `core/knowledge-storage/benches/traversal.rs` | Create | Graph traversal benchmarks                    |
| `core/knowledge-derive/benches/views.rs`      | Create | View rendering benchmarks                     |
| `core/knowledge-derive/benches/search.rs`     | Create | Semantic search benchmarks                    |
| `Cargo.toml`                                  | Modify | Add `criterion` dev-dependency for benchmarks |

**Performance targets (from PRD-0003):**

| ID    | Requirement                               | Target  | Acceptable |
| ----- | ----------------------------------------- | ------- | ---------- |
| NF1.1 | Graph traversal (2 hops at 100K entities) | < 100ms | < 500ms    |
| NF1.2 | Graph traversal (3 hops at 100K entities) | < 500ms | < 2s       |
| NF1.3 | View rendering latency                    | < 100ms | < 500ms    |
| NF1.4 | Plugin load time                          | < 500ms | < 2s       |
| NF1.5 | Semantic search latency                   | < 200ms | < 1s       |

**Benchmark setup:**

Each benchmark creates a synthetic graph with:
- 100K entities
- 1M relationships
- Average branching factor of 10

Benchmarks use `criterion` for statistical rigor. Results are recorded in `Implementation Notes`.

**Verification:**
- `cargo bench -p knowledge-storage -- traversal` runs
- `cargo bench -p knowledge-derive -- views` runs
- `cargo bench -p knowledge-derive -- search` runs
- All benchmarks meet target or acceptable thresholds

**Exit criteria:** All performance targets met

---

### D3: Cross-Plan Integration Tests

**Purpose:** Verify that features from different IPs work together correctly

**Files:**

| File                                               | Action | Description                         |
| -------------------------------------------------- | ------ | ----------------------------------- |
| `core/knowledge-storage/tests/integration_test.rs` | Modify | Add cross-feature integration tests |
| `core/knowledge-derive/tests/integration_test.rs`  | Create | View + search integration tests     |

**Test scenarios:**

1. **Import → Traverse → View:** Import entities, traverse relationships, display as graph view
2. **Import → Search → Semantic:** Import entities, keyword search, semantic search, hybrid search
3. **Import → Collection → Tree View:** Import entities, create collection, add entities, view in tree
4. **Plugin → Import → Embedding:** Import via plugin, generate embeddings, search semantically
5. **Event Notification → View Update:** Create entity, verify view reflects new entity
6. **Event Notification → Embedding Regeneration:** Update content, verify embedding regenerated

**Verification:**
- `cargo test -p knowledge-storage` passes (including new integration tests)
- `cargo test -p knowledge-derive` passes (including new integration tests)
- `cargo test` (workspace) passes

**Exit criteria:** All cross-plan integration tests pass

---

## Execution Order

```
D1 (E2E tests) -> D2 (benchmarks) -> D3 (integration tests)
```

D1 verifies user-facing workflows. D2 verifies performance. D3 verifies internal component integration.

---

## Verification Strategy

| Level       | Command                                            | Coverage                              |
| ----------- | -------------------------------------------------- | ------------------------------------- |
| E2E         | `cargo test --test cucumber -p knowledge-cli`      | All BDD scenarios across all features |
| Benchmark   | `cargo bench`                                      | Performance targets NF1.1–NF1.5       |
| Integration | `cargo test` (workspace)                           | Cross-plan component integration      |
| Regression  | `cargo test` (full workspace)                      | No regressions from any IP            |
| Lint        | `cargo clippy -- -D warnings && cargo fmt --check` | Code quality                          |

---

## Exit Criteria

- [ ] All E2E BDD scenarios pass
- [ ] Performance benchmarks meet NF1.1–NF1.5 targets
- [ ] Cross-plan integration tests pass
- [ ] Full workspace `cargo test` passes
- [ ] `cargo clippy -- -D warnings` passes
- [ ] No regressions from IP-001 through IP-005

---

## Implementation Notes

*(Filled in during/after implementation — records benchmark results, performance findings, and integration issues)*
