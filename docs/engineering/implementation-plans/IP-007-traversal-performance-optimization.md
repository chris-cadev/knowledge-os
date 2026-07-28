# IP-007: Phase 7 — Traversal Performance Optimization

**Status:** Draft
**ADR(s):** [ADR-0019](../../architecture/adrs/adr-0019.md) (Level-by-Level BFS for Graph Traversal), [ADR-0020](../../architecture/adrs/adr-0020.md) (Composite Indexes for Relationship Traversal)
**PRD(s):** [PRD-0005](../prds/prd-0005-traversal-performance-optimization.md) (F1.1–F1.4, F2.1–F2.6, F3.1–F3.4, NF1.1–NF1.6)
**Estimated effort:** ~8 days

---

## Context

ADR-0014 chose recursive CTEs in SQLite for graph traversal. The implementation (435 lines in `core/knowledge-storage/src/adapters/sqlite/traversal.rs`) uses comma-separated UUID path strings for cycle detection and reconstructs edges via full-table scans of the `relationships` table per result. The approach fails PRD-0003's NFR targets by a wide margin: 2-hop traversal at 1,000 entities takes ~457ms (target: <100ms at 100K entities), and 3-hop takes ~2.8s (target: <500ms at 100K entities).

ADR-0019 replaces the recursive CTE with level-by-level BFS using indexed SQL queries and an in-memory `HashSet` for cycle detection. ADR-0020 adds composite indexes on `(source_id, is_active)` and `(target_id, is_active)` to eliminate full table scans. Both ADRs are implemented in this plan — indexes first, then the BFS algorithm, then edge reconstruction optimization.

The existing integration tests at `core/knowledge-storage/tests/integration_test.rs` (14 traversal test calls covering chain, tree, diamond, cycle, bidirectional, disconnected, depth limits, type filters, and error cases) serve as the correctness baseline. All must pass with the new implementation.

---

## Deliverables

### D1: Database Indexes for Relationship Traversal

**Purpose:** Add composite indexes to the `relationships` table so traversal queries use index seeks instead of full table scans. Implement idempotent migration for existing databases.

**Files:**

| File | Action | Description |
|------|--------|-------------|
| `core/knowledge-storage/src/adapters/sqlite/store.rs` | Modify | Add `CREATE INDEX IF NOT EXISTS` statements for `idx_relationships_source_active` and `idx_relationships_target_active` after the `CREATE TABLE` batch in `SqliteStore::new` (after line 116) |

**Implementation:**

Add two `conn.execute_batch` or `conn.execute` calls after the table creation batch at line 116–118 of `store.rs`:

```rust
// After line 118: .map_err(|e| StorageError::Internal(e.to_string()))?;

conn.execute_batch(
    "CREATE INDEX IF NOT EXISTS idx_relationships_source_active
        ON relationships(source_id, is_active);

     CREATE INDEX IF NOT EXISTS idx_relationships_target_active
        ON relationships(target_id, is_active);"
)
.map_err(|e| StorageError::Internal(e.to_string()))?;
```

The indexes are additive — they do not change the schema of canonical tables. The `CREATE INDEX IF NOT EXISTS` syntax ensures idempotency: existing databases are upgraded automatically, and repeat calls do not error.

**Verification:**
- `cargo test -p knowledge-storage` passes (no regressions)
- Add a test that opens a fresh `:memory:` store, runs `EXPLAIN QUERY PLAN` for `SELECT target_id FROM relationships WHERE source_id = ? AND is_active = 1`, and asserts the output contains `USING INDEX idx_relationships_source_active` (not `SCAN`).
- Verify idempotency: open two `SqliteStore` instances in sequence against the same file path, assert the second open does not fail.

**Exit criteria:** Indexes created, EXPLAIN QUERY PLAN shows index usage, existing tests pass, idempotency confirmed.

---

### D2: Level-by-Level BFS (Outgoing Traversal)

**Purpose:** Rewrite `traverse_outgoing` to use level-by-level expansion with indexed queries and an in-memory visited set. Retain the old CTE code under `#[cfg(test)]` for parity comparison. Add `reconstruct_path_to` helper for path building.

**Files:**

| File | Action | Description |
|------|--------|-------------|
| `core/knowledge-storage/src/adapters/sqlite/traversal.rs` | Modify | Replace `traverse_outgoing` (lines 93–146) with level-by-level BFS. Add `reconstruct_path_to` helper. Add `old_traverse_outgoing` under `#[cfg(test)]`. Update imports. |
| `core/knowledge-storage/src/adapters/sqlite/tests.rs` | Create or Modify | Add `mod tests` module with parity test helpers and unit tests for path reconstruction |

**New function: `traverse_outgoing`**

Replace the CTE at lines 93–146 with:

```rust
fn traverse_outgoing(
    conn: &Connection,
    start_id: Uuid,
    max_depth: u32,
    rel_type: Option<&RelationshipType>,
    entity_type: Option<&EntityType>,
) -> Result<Vec<(Uuid, u32, Vec<Uuid>)>, StorageError> {
    let mut visited: HashSet<Uuid> = HashSet::new();
    let mut current_level: Vec<Uuid> = vec![start_id];
    // (id) -> (parent_id, relationship_type)
    let mut parent: HashMap<Uuid, (Uuid, String)> = HashMap::new();
    parent.insert(start_id, (start_id, String::new()));
    let mut results: Vec<(Uuid, u32, Vec<Uuid>)> = Vec::new();

    visited.insert(start_id);
    results.push((start_id, 0, vec![start_id]));

    // Prepare statement once, reuse per level
    let sql = if rel_type.is_some() {
        "SELECT r.id, r.target_id, r.relationship_type
         FROM relationships r
         WHERE r.source_id = ?1 AND r.is_active = 1
           AND r.relationship_type = ?2"
    } else {
        "SELECT r.id, r.target_id, r.relationship_type
         FROM relationships r
         WHERE r.source_id = ?1 AND r.is_active = 1"
    };
    let mut stmt = conn.prepare(sql)
        .map_err(|e| StorageError::Internal(e.to_string()))?;

    for depth in 1..=max_depth {
        if current_level.is_empty() {
            break;
        }

        let mut next_level: Vec<Uuid> = Vec::new();

        for node_id in &current_level {
            let rows = if let Some(rt) = rel_type {
                let rt_json = serde_json::to_string(rt).unwrap();
                stmt.query_map(
                    rusqlite::params![node_id.to_string(), rt_json],
                    |row| {
                        let target: String = row.get(1)?;
                        let rtype: String = row.get(2)?;
                        Ok((target, rtype))
                    },
                )
            } else {
                stmt.query_map(
                    rusqlite::params![node_id.to_string()],
                    |row| {
                        let target: String = row.get(1)?;
                        let rtype: String = row.get(2)?;
                        Ok((target, rtype))
                    },
                )
            }
            .map_err(|e| StorageError::Internal(e.to_string()))?;

            for row in rows {
                let (target_id_str, rel_type_str) = row
                    .map_err(|e| StorageError::Internal(e.to_string()))?;
                let target_id = Uuid::parse_str(&target_id_str)
                    .map_err(|e| StorageError::Internal(e.to_string()))?;

                if visited.insert(target_id) {
                    parent.insert(target_id, (*node_id, rel_type_str));
                    next_level.push(target_id);
                }
            }
        }

        // Build paths for discovered nodes at this depth
        for node_id in &next_level {
            let path = reconstruct_path_to(*node_id, &parent);
            results.push((*node_id, depth, path));
        }

        current_level = next_level;
    }

    // Entity type filter applied post-expansion
    if let Some(et) = entity_type {
        let et_json = serde_json::to_string(et).unwrap();
        let mut et_stmt = conn.prepare(
            "SELECT 1 FROM entities WHERE id = ?1 AND entity_type = ?2 AND is_active = 1"
        )
        .map_err(|e| StorageError::Internal(e.to_string()))?;
        results.retain(|(id, _, _)| {
            et_stmt.query_row(
                rusqlite::params![id.to_string(), &et_json],
                |_| Ok(()),
            ).is_ok()
        });
    }

    results.sort_by_key(|(_, depth, _)| *depth);
    Ok(results)
}
```

**New function: `reconstruct_path_to`**

```rust
fn reconstruct_path_to(
    node_id: Uuid,
    parent: &HashMap<Uuid, (Uuid, String)>,
) -> Vec<Uuid> {
    let mut current = node_id;
    let mut reversed = Vec::new();
    reversed.push(current);
    while let Some((p, _)) = parent.get(&current) {
        if *p == current {
            break; // reached root
        }
        current = *p;
        reversed.push(current);
    }
    reversed.reverse();
    reversed
}
```

**Retained CTE code (under `#[cfg(test)]`):**

```rust
#[cfg(test)]
pub(crate) mod cte_legacy {
    // copy of the original traverse_outgoing, traverse_incoming,
    // traverse_both, and reconstruct_edges as they exist at commit <hash>
    // Used for parity testing only.
}
```

The `#[cfg(test)]` module keeps the old code out of release builds while making it available for parity tests.

**Updated imports at line 1–8:**

Add `use std::collections::{HashMap, HashSet};` (already present, keep). The `VecDeque` import can be removed if `reconstruct_path` (line 268) is deleted.

**Verification:**
- `cargo test -p knowledge-storage` passes
- New unit test in `traversal.rs` (or `tests.rs`): `test_reconstruct_path_to_chain` verifies path building for a simple chain
- New parity test: `test_bfs_matches_cte_outgoing_chain`, `test_bfs_matches_cte_outgoing_tree`, `test_bfs_matches_cte_outgoing_cycle` — each creates a graph, runs both `traverse_outgoing` and the old CTE version, asserts identical results

**Exit criteria:** Outgoing BFS produces identical results to old CTE for chain, tree, diamond, and cycle topologies. All existing tests pass.

---

### D3: Incoming and Bidirectional BFS Traversal

**Purpose:** Rewrite `traverse_incoming` and `traverse_both` to use the same level-by-level BFS pattern as D2. Incoming is symmetric to outgoing (query `target_id` instead of `source_id`). Bidirectional expands both directions with dual parent maps.

**Files:**

| File | Action | Description |
|------|--------|-------------|
| `core/knowledge-storage/src/adapters/sqlite/traversal.rs` | Modify | Replace `traverse_incoming` (lines 148–201) and `traverse_both` (lines 203–266) with level-by-level BFS. Retain old versions under `#[cfg(test)]`. |

**`traverse_incoming`:**

Structurally identical to `traverse_outgoing` but uses `target_id` lookup:

```sql
SELECT r.id, r.source_id, r.relationship_type
FROM relationships r
WHERE r.target_id = ?1 AND r.is_active = 1
```

The parent map uses `(node_id) -> (child_id, relationship_type)` semantics (the node at depth-1 is the source/target depending on direction).

**`traverse_both`:**

```rust
fn traverse_both(
    conn: &Connection,
    start_id: Uuid,
    max_depth: u32,
    rel_type: Option<&RelationshipType>,
    entity_type: Option<&EntityType>,
) -> Result<Vec<(Uuid, u32, Vec<Uuid>)>, StorageError> {
    let mut visited: HashSet<Uuid> = HashSet::new();
    let mut current_level: Vec<Uuid> = vec![start_id];
    // Separate parent maps for outgoing and incoming traversals
    let mut parent_out: HashMap<Uuid, (Uuid, String)> = HashMap::new();
    let mut parent_in: HashMap<Uuid, (Uuid, String)> = HashMap::new();
    parent_out.insert(start_id, (start_id, String::new()));
    parent_in.insert(start_id, (start_id, String::new()));
    let mut results: Vec<(Uuid, u32, Vec<Uuid>)> = Vec::new();

    visited.insert(start_id);
    results.push((start_id, 0, vec![start_id]));

    let mut out_stmt = prepare outgoing SQL (same as traverse_outgoing)
    let mut in_stmt = prepare incoming SQL (same as traverse_incoming)

    for depth in 1..=max_depth {
        if current_level.is_empty() {
            break;
        }
        let mut next_level: Vec<Uuid> = Vec::new();

        // Expand outgoing
        for node_id in &current_level {
            expand using out_stmt, insert into next_level if visited.insert()
            record in parent_out
        }

        // Expand incoming
        for node_id in &current_level {
            expand using in_stmt, insert into next_level if visited.insert()
            record in parent_in
        }

        // Path reconstruction uses both parent maps
        for node_id in &next_level {
            let path = reconstruct_bidirectional_path(
                *node_id, start_id, &parent_out, &parent_in
            );
            results.push((*node_id, depth, path));
        }

        current_level = next_level;
    }

    // Entity type filter (same as outgoing)
    // ... 
    Ok(results)
}
```

**Path reconstruction for bidirectional:**

```rust
fn reconstruct_bidirectional_path(
    node_id: Uuid,
    start_id: Uuid,
    parent_out: &HashMap<Uuid, (Uuid, String)>,
    parent_in: &HashMap<Uuid, (Uuid, String)>,
) -> Vec<Uuid> {
    // Walk from node_id back toward start_id using whichever parent map
    // provides the path. The node was reached via outgoing OR incoming expansion.
    let mut path = Vec::new();
    let mut current = node_id;
    path.push(current);

    // Try outgoing parent map first
    while let Some(&(p, _)) = parent_out.get(&current) {
        if p == current { break; }
        path.push(p);
        current = p;
        if current == start_id { break; }
    }

    if current != start_id {
        // Reset and try incoming parent map
        path.clear();
        current = node_id;
        path.push(current);
        while let Some(&(p, _)) = parent_in.get(&current) {
            if p == current { break; }
            path.push(p);
            current = p;
            if current == start_id { break; }
        }
    }

    path.reverse();
    path
}
```

**Verification:**
- `cargo test -p knowledge-storage` passes
- Parity tests: `test_bfs_matches_cte_incoming_*`, `test_bfs_matches_cte_bidirectional_*` for all graph topologies
- Cycle graph with bidirectional: confirms termination (visited set prevents re-entering `A`)

**Exit criteria:** Incoming and bidirectional BFS produce identical results to old CTE. All existing tests pass.

---

### D4: Single-Pass Edge Reconstruction

**Purpose:** Replace the per-result `reconstruct_edges` call (lines 74–87 in `traversal.rs`) with a single batched query that fetches all edges across all paths at once. The `TraversalResult` assembly uses a cached `HashMap<(Uuid, Uuid), TraversalEdge>` instead of loading all relationships per result.

**Files:**

| File | Action | Description |
|------|--------|-------------|
| `core/knowledge-storage/src/adapters/sqlite/traversal.rs` | Modify | Update the `traverse` method to collect all paths first, then call a new `batch_reconstruct_edges` function once. Remove per-result `reconstruct_edges` calls. Remove or gate the old `reconstruct_edges` under `#[cfg(test)]`. |

**Updated `traverse` method (replacing lines 68–90):**

```rust
// Phase 1: BFS expansion (unchanged from previous structure)
let (reachable, _) = match query.direction { ... };

// Phase 2: Collect paths (excluding start node)
let all_paths: Vec<Vec<Uuid>> = reachable
    .into_iter()
    .filter(|(id, _, _)| *id != query.start_id)
    .take(max_results)
    .map(|(_, _, path)| path)
    .collect();

// Phase 3: Single-pass edge reconstruction
let edges = batch_reconstruct_edges(&conn, &all_paths)?;

// Build lookup map
let edge_lookup: HashMap<(Uuid, Uuid), TraversalEdge> = edges
    .into_iter()
    .map(|e| ((e.source_id, e.target_id), e))
    .collect();

// Phase 4: Assemble results
let results: Vec<TraversalResult> = all_paths
    .into_iter()
    .map(|path| {
        let depth = (path.len() as u32).saturating_sub(1);
        let path_edges: Vec<TraversalEdge> = path
            .windows(2)
            .filter_map(|w| edge_lookup.get(&(w[0], w[1])).cloned())
            .collect();
        TraversalResult {
            path,
            edges: path_edges,
            depth,
        }
    })
    .collect();

Ok(results)
```

**New function: `batch_reconstruct_edges`**

```rust
fn batch_reconstruct_edges(
    conn: &Connection,
    all_paths: &[Vec<Uuid>],
) -> Result<Vec<TraversalEdge>, StorageError> {
    // Collect unique (source, target) pairs from all paths
    let mut pairs: Vec<(Uuid, Uuid)> = Vec::new();
    let mut seen: HashSet<(Uuid, Uuid)> = HashSet::new();
    for path in all_paths {
        for window in path.windows(2) {
            let pair = (window[0], window[1]);
            if seen.insert(pair) {
                pairs.push(pair);
            }
        }
    }

    if pairs.is_empty() {
        return Ok(Vec::new());
    }

    // Build batched query using IN clause (SQLite supports tuple IN)
    let placeholders: Vec<String> = pairs
        .iter()
        .enumerate()
        .map(|(i, _)| format!("(?{}, ?{})", i * 2 + 1, i * 2 + 2))
        .collect();

    let sql = format!(
        "SELECT source_id, target_id, relationship_type
         FROM relationships
         WHERE is_active = 1
           AND (source_id, target_id) IN ({})",
        placeholders.join(", ")
    );

    let params: Vec<String> = pairs
        .iter()
        .flat_map(|(s, t)| vec![s.to_string(), t.to_string()])
        .collect();

    let param_refs: Vec<&dyn rusqlite::types::ToSql> =
        params.iter().map(|s| s as &dyn rusqlite::types::ToSql).collect();

    let mut stmt = conn
        .prepare(&sql)
        .map_err(|e| StorageError::Internal(e.to_string()))?;

    let rows = stmt
        .query_map(param_refs.as_slice(), |row| {
            let source: String = row.get(0)?;
            let target: String = row.get(1)?;
            let rel_type: String = row.get(2)?;
            let source_id = Uuid::parse_str(&source)
                .map_err(|e| rusqlite::Error::ToSqlConversionFailure(Box::new(e)))?;
            let target_id = Uuid::parse_str(&target)
                .map_err(|e| rusqlite::Error::ToSqlConversionFailure(Box::new(e)))?;
            let relationship_type = serde_json::from_str(&format!("\"{}\"", rel_type))
                .map_err(|e| {
                    rusqlite::Error::ToSqlConversionFailure(Box::new(e))
                })?;
            Ok(TraversalEdge {
                source_id,
                target_id,
                relationship_type,
            })
        })
        .map_err(|e| StorageError::Internal(e.to_string()))?;

    rows.collect::<Result<Vec<_>, _>>()
        .map_err(|e| StorageError::Internal(e.to_string()))
}
```

**Verification:**
- `cargo test -p knowledge-storage` passes
- All 14 existing traversal integration tests pass with the new edge reconstruction
- Benchmark shows `reconstruct_edges` no longer appears in profiling (was previously the dominant cost)

**Exit criteria:** Edge reconstruction uses one query per traversal, not one per result. All tests pass.

---

### D5: Integration Tests, Parity Testing, and Benchmark Updates

**Purpose:** Add parity tests that compare BFS output against retained CTE code for randomized graphs. Update the Criterion benchmarks with new NFR targets. Add volume benchmarks at 100K entities.

**Files:**

| File | Action | Description |
|------|--------|-------------|
| `core/knowledge-storage/src/adapters/sqlite/traversal.rs` | Modify | Add `#[cfg(test)] mod tests { ... }` or update existing test module with parity tests |
| `core/knowledge-storage/tests/integration_test.rs` | Modify | Add `test_traversal_parity_random_graph` for randomized CTE-BFS comparison |
| `core/knowledge-storage/benches/traversal.rs` | Modify | Update bench targets, add 100K-volume benchmarks, add edge reconstruction micro-benchmark |

**Parity test (in `traversal.rs` test module or integration test):**

```rust
#[cfg(test)]
mod tests {
    use super::*;

    /// For each random graph topology, run both BFS and CTE traversal
    /// and assert exact result equality.
    #[test]
    fn test_bfs_matches_cte_random_graphs() {
        // Generate 50 random graphs with controlled parameters
        for seed in 0..50 {
            let (conn, start_id) = generate_random_graph(seed, 100, 5);
            let config = TraversalConfig::default();

            for depth in 1..=4 {
                for direction in &[TraversalDirection::Outgoing,
                                   TraversalDirection::Incoming,
                                   TraversalDirection::Both] {
                    let bfs_results = bfs_traverse(&conn, start_id, depth, direction, None, None);
                    let cte_results = cte_legacy::cte_traverse(&conn, start_id, depth, direction, None, None);

                    assert_eq!(
                        bfs_results, cte_results,
                        "Mismatch for seed={}, depth={}, direction={:?}",
                        seed, depth, direction
                    );
                }
            }
        }
    }
}
```

**Updated benchmarks in `benches/traversal.rs`:**

Replace the `criterion_group!` section and add:

```rust
fn bench_traversal_2hop_100k(c: &mut Criterion) {
    let rt = tokio::runtime::Runtime::new().unwrap();
    // Create 100K entities with 1M relationships
    // Estimated runtime: 10-15 seconds for setup, then benchmark
    let store = rt.block_on(setup_store(100_000, 10));

    let entities = rt.block_on(EntityRepository::list(&store)).unwrap();
    let start_id = entities[0].id;

    let config = TraversalConfig {
        default_max_depth: 10,
        default_max_results: 1000,
    };

    c.bench_function("traversal_2hop_100k_entities", |b| {
        b.iter(|| {
            rt.block_on(async {
                TraversalPort::traverse(
                    &store,
                    &TraversalQuery {
                        start_id: black_box(start_id),
                        direction: TraversalDirection::Outgoing,
                        max_depth: Some(2),
                        max_results: None,
                        relationship_type: None,
                        entity_type_filter: None,
                    },
                    &config,
                )
                .await
                .unwrap()
            })
        })
    });
}
```

**Benchmark targets (from PRD-0005 NF1):**

| Benchmark | Target | Acceptable |
|-----------|--------|------------|
| 2-hop at 1K entities | < 10ms | < 50ms |
| 3-hop at 1K entities | < 50ms | < 200ms |
| 2-hop bidirectional at 1K | < 20ms | < 100ms |
| 2-hop with type filter at 1K | < 10ms | < 50ms |
| 2-hop at 100K entities | < 100ms | < 500ms |
| 3-hop at 100K entities | < 500ms | < 2s |

**Verification:**
- `cargo test -p knowledge-storage` passes (including parity tests)
- `cargo bench -p knowledge-storage` runs without errors
- 2-hop at 1K under 10ms
- 3-hop at 1K under 50ms

**Exit criteria:** Parity tests pass across 50 random graphs at depths 1–4 for all three directions. Benchmarks meet all NFR targets.

---

## Execution Order

```
D1 (indexes) → D2 (outgoing BFS) → D3 (incoming/bidirectional BFS) → D4 (edge reconstruction) → D5 (tests + benchmarks)
```

D1 must come first because D2's SQL queries depend on indexes for performance (the BFS will work without indexes but will be slow). D2 and D3 can be developed in parallel since the incoming/bidirectional patterns are symmetric. D4 depends on D2/D3 producing the new path data structure (`Vec<Uuid>` instead of `String`). D5 depends on all prior deliverables.

---

## Verification Strategy

| Level | Command | Coverage |
|-------|---------|----------|
| Unit | `cargo test -p knowledge-storage -- lib` | Path reconstruction, CTE parity, edge building |
| Integration | `cargo test -p knowledge-storage --test integration_test` | 14 existing traversal tests + 50 random parity tests |
| Benchmark | `cargo bench -p knowledge-storage` | 2-hop, 3-hop, bidirectional, type-filter at 1K and 100K |
| E2E | `cargo test --test cucumber -p knowledge-cli` | CLI traversal scenarios (unchanged from IP-001) |
| Lint | `cargo clippy -- -D warnings && cargo fmt --check` | Code quality |

---

## Exit Criteria

- [ ] `CREATE INDEX IF NOT EXISTS` statements in `store.rs` — `EXPLAIN QUERY PLAN` confirms index usage
- [ ] `traverse_outgoing` uses level-by-level BFS with in-memory `HashSet` cycle detection
- [ ] `traverse_incoming` uses symmetric level-by-level BFS
- [ ] `traverse_both` uses dual parent maps with shared visited set
- [ ] `reconstruct_path_to` builds paths from parent map (no string concatenation)
- [ ] `batch_reconstruct_edges` fetches all edges in one batched query
- [ ] Old CTE code retained under `#[cfg(test)]` for parity
- [ ] Parity tests pass: 50 random graphs × 3 directions × 4 depths = 600 test cases
- [ ] All 14 existing traversal integration tests pass
- [ ] 2-hop at 100K entities completes in < 100ms
- [ ] 3-hop at 100K entities completes in < 500ms
- [ ] `cargo clippy -- -D warnings` passes
- [ ] ADR-0019 and ADR-0020 updated with Implementation Notes if any deviations

---

## Implementation Notes

*(Filled in during/after implementation — records deviations, discoveries, decisions made during coding)*
