# PRD-0005: Graph Traversal Performance Optimization

**Status:** Draft
**Date:** 2026-07-28
**Author:** Core maintainers
**Priority:** P1 — Performance Layer
**Depends on:** PRD-0003, PRD-0004

---

## Purpose

This PRD optimizes the graph traversal implementation to meet the non-functional requirements specified in PRD-0003 (NF1.1, NF1.2). The existing traversal uses recursive CTEs with string-based path tracking and redundant edge reconstruction, causing 2-hop traversal at 1,000 entities to take ~457ms and 3-hop traversal to take ~2.8s — far exceeding the targets of <100ms and <500ms respectively. This PRD restructures the traversal to use index-accelerated level-by-level expansion, cached adjacency structures, and post-traversal edge reconstruction from collected paths, without changing the public API.

---

## Problem Statement

PRD-0003 specifies graph traversal performance targets: 2-hop < 100ms, 3-hop < 500ms at 100K entities (NF1.1, NF1.2). The current implementation uses a recursive CTE with comma-separated UUID path strings for cycle detection (`',' || t.path || ',') NOT LIKE ('%,' || e.id || ',%')`), which performs LIKE pattern matching on every recursive step. After the CTE returns, each result calls `reconstruct_edges`, which issues a full table scan of the relationships table — loading every relationship into memory once per result path.

The benchmark at only 1,000 entities with 10 edges/entity shows:
- 2-hop traversal: ~457ms (target: <100ms at 100K entities)
- 3-hop traversal: ~2.8s (target: <500ms at 100K entities)
- 2-hop bidirectional: ~1.57s
- 2-hop with type filter: ~460ms

Without optimization, these numbers scale worse than linearly with entity count. At 100K entities the traversal would be unusable. The implementation does not meet the specified NFRs and cannot be shipped in its current state.

---

## Root Cause Analysis

Four compounding bottlenecks cause the performance failure:

1. **No database indexes on relationship columns.** The `relationships` table has no indexes on `source_id`, `target_id`, or `is_active`. Every recursive CTE join over `r.source_id = t.id` performs a full table scan of the entire relationships table at each recursion level. With 10K relationships and 3-hop traversal at average fan-out 10, this causes approximately 1,110 full table scans (1 + 10 + 100 + 1000).

2. **String-based path tracking for cycle detection.** The CTE constructs comma-separated UUID path strings at every recursive step (`t.path || ',' || r.target_id`), and cycle detection uses `(',' || t.path || ',') NOT LIKE ('%,' || e.id || ',%')`. String concatenation and LIKE with wildcards are O(n) operations that grow with path length. UUIDs are 36 characters each, so a path of 1000 nodes is a 36KB string searched with LIKE at each expansion.

3. **Redundant full-table edge reconstruction.** The `reconstruct_edges` function executes `SELECT source_id, target_id, relationship_type FROM relationships WHERE is_active = 1` — loading every relationship into a HashMap — for every single `TraversalResult`. If a 2-hop traversal returns 100 results, it scans the entire relationships table 100 times.

4. **Row-number-based deduplication overhead.** The CTE uses `ROW_NUMBER() OVER (PARTITION BY id ORDER BY depth, length(path))` to deduplicate nodes visited via multiple paths. This requires a sort operation per partition, adding overhead proportional to the number of intermediate results (which grows exponentially with depth and fan-out).

---

## Scope

### In Scope

- Index creation on `relationships` table for traversal queries
- Replacement of recursive CTE with level-by-level BFS using indexed queries
- Elimination of string-based path tracking and LIKE cycle detection
- Single-pass edge reconstruction using collected path data
- Transaction-level caching of adjacency structures across a single traversal
- Migration strategy for existing databases (additive index creation)
- Performance benchmarks validated against NFR targets
- Bidirectional traversal optimization (incoming + outgoing)

### Out of Scope

- New traversal features or query capabilities
- Changes to the `TraversalPort`, `TraversalQuery`, `TraversalConfig`, or `TraversalResult` types
- Cross-machine distributed traversal (deferred to Year 3)
- In-memory graph database replacement (deferred to scalability phase)
- Changes to view projections or CLI output format
- Semantic search or embedding traversal

---

## Engineering Questions

### 1. Which canonical entities are introduced?

None. The canonical entity model is unchanged.

### 2. Which relationships are introduced?

None. All relationship types are defined in `docs/architecture/domain-model.md`.

### 3. Which components are introduced?

None. No new component types are added.

### 4. Which events are emitted?

None. Traversal is a read-only operation. No events are emitted.

### 5. Which derived representations are generated?

None. Traversal operates directly on canonical data. No new derived representations are created. View projections that use traversal (graph view) remain derived representations that are rebuilt from canonical data.

### 6. Which layer owns the feature?

Layer 5 — Relationship Engine. The traversal implementation lives in the storage adapter (`knowledge-storage`), but the traversal contract is defined in `knowledge-core`. All changes are contained within the storage adapter.

### 7. Can every derived artifact be regenerated?

This feature does not introduce derived artifacts. The traversal operation reads canonical data directly. No regeneration concern applies.

### 8. Does the feature violate storage independence?

No. The optimization adds indexes (which every SQL database supports) and changes query patterns within the SQLite adapter. The `TraversalPort` trait remains unchanged. Alternative storage adapters implement traversal independently.

### 9. Does the feature introduce implementation leakage?

No. The optimization is fully contained within the storage adapter. The traversal port interface, result types, and configuration are unchanged. Storage-specific constructs (indexes, query patterns) do not leak into the domain layer.

### 10. Does the feature preserve the canonical model?

Yes. Traversal is a read-only operation on canonical entities and relationships. No canonical data is modified. The canonical model is unchanged.

---

## Pipeline Spine Analysis

The pipeline is unchanged. The traversal implementation within Layer 5 (Relationship Engine) is optimized, but the contracts remain identical.

```
  Layer 5  Relationship Engine   ← Optimization target
           TraversalPort::traverse()   (unchanged API)
                 ↓
           BFS expansion using indexed queries   (replaces recursive CTE)
                 ↓
           Post-traversal edge reconstruction    (single pass)
                 ↓
           TraversalResult Vec                  (unchanged output)
```

### Why Level-by-Level BFS Replaces Recursive CTE

The recursive CTE with string-based cycle detection is the wrong tool for this access pattern. SQLite recursive CTEs are optimized for tree-shaped queries where each row is independent. Graph traversal requires shared mutable state (the visited set), which CTEs simulate poorly through string concatenation.

Level-by-level BFS replaces the CTE with N+1 indexed queries (where N is the depth), each of which:
- Uses an equality lookup on `source_id` or `target_id` (index-accelerated)
- Filters results through an in-memory visited set (hash set lookup, O(1))
- Returns results for the next level

Total database round-trips: depth + 1. With indexes, each round-trip is a B-tree lookup of O(log R) where R is the relationship count.

---

## Functional Requirements

### F1: Indexed Relationship Table

| ID   | Requirement                                        | Priority | Acceptance Criteria                                                         |
| ---- | -------------------------------------------------- | -------- | --------------------------------------------------------------------------- |
| F1.1 | Add index on `relationships(source_id, is_active)` | P0       | `EXPLAIN QUERY PLAN` for `WHERE source_id = ? AND is_active = 1` uses index |
| F1.2 | Add index on `relationships(target_id, is_active)` | P0       | `EXPLAIN QUERY PLAN` for `WHERE target_id = ? AND is_active = 1` uses index |
| F1.3 | Migrations run at store initialization             | P0       | Existing databases receive indexes on open without data loss                |
| F1.4 | Index creation is idempotent                       | P0       | `CREATE INDEX IF NOT EXISTS` prevents duplicate index errors                |

### F2: Level-by-Level BFS Traversal

| ID   | Requirement                                      | Priority | Acceptance Criteria                                                                       |
| ---- | ------------------------------------------------ | -------- | ----------------------------------------------------------------------------------------- |
| F2.1 | Outgoing traversal uses level-by-level expansion | P0       | Results match recursive CTE output for all graph topologies (chain, tree, diamond, cycle) |
| F2.2 | Incoming traversal uses level-by-level expansion | P0       | Results match recursive CTE incoming output                                               |
| F2.3 | Bidirectional traversal expands both directions  | P0       | Results match recursive CTE bidirectional output                                          |
| F2.4 | Cycle detection uses in-memory visited set       | P0       | Traversal terminates in cyclic graphs; no infinite loops                                  |
| F2.5 | Relationship type filter applied per level       | P0       | Filtering by `relationship_type` restricts expansion to matching edges                    |
| F2.6 | Entity type filter applied after expansion       | P0       | Filtering by `entity_type` restricts returned nodes, not expansion                        |

### F3: Edge Reconstruction

| ID   | Requirement                                             | Priority | Acceptance Criteria                                                    |
| ---- | ------------------------------------------------------- | -------- | ---------------------------------------------------------------------- |
| F3.1 | Edges reconstructed from paths in a single pass         | P0       | One query per traversal fetches all edges along discovered paths       |
| F3.2 | Edge query uses indexed lookups, not full table scan    | P0       | `EXPLAIN QUERY PLAN` confirms index usage                              |
| F3.3 | Edge data cached for the duration of a single traversal | P0       | Multiple results share the same edge cache without re-querying         |
| F3.4 | Edge reconstruction produces identical output           | P0       | Traversal results contain the same edges as the current implementation |

---

## Non-Functional Requirements

### NF1: Performance

| ID    | Requirement                      | Target  | Acceptable | Current (1K ents) |
| ----- | -------------------------------- | ------- | ---------- | ----------------- |
| NF1.1 | 2-hop outgoing traversal latency | < 10ms  | < 50ms     | ~457ms            |
| NF1.2 | 3-hop outgoing traversal latency | < 50ms  | < 200ms    | ~2.8s             |
| NF1.3 | 2-hop bidirectional traversal    | < 20ms  | < 100ms    | ~1.57s            |
| NF1.4 | 2-hop traversal with type filter | < 10ms  | < 50ms     | ~460ms            |
| NF1.5 | 2-hop traversal at 100K entities | < 100ms | < 500ms    | Not tested        |
| NF1.6 | 3-hop traversal at 100K entities | < 500ms | < 2s       | Not tested        |

### NF2: Scalability

| ID    | Requirement          | Target                                         |
| ----- | -------------------- | ---------------------------------------------- |
| NF2.1 | Entity volume        | 100K entities                                  |
| NF2.2 | Relationship volume  | 1M relationships                               |
| NF2.3 | Average fan-out      | 10 edges/entity                                |
| NF2.4 | Database round-trips | depth + 1 per traversal                        |
| NF2.5 | Memory per traversal | proportional to visited nodes, not total graph |

### NF3: Correctness

| ID    | Requirement                     | Target                                        |
| ----- | ------------------------------- | --------------------------------------------- |
| NF3.1 | Result parity with CTE approach | All existing traversal tests pass             |
| NF3.2 | Cycle termination               | Traversal terminates for any graph            |
| NF3.3 | Deterministic output            | Same query on same data produces same results |

---

## User Stories

### US1: Fast Entity Exploration

**As a** knowledge worker,
**I want to** explore relationships between entities without noticeable delay,
**So that** graph exploration feels instantaneous and encourages iterative discovery.

**Acceptance criteria:**
1. `kos traverse <entity-id> --depth 2` returns results in under 100ms for 100K entities.
2. `kos traverse <entity-id> --depth 3` returns results in under 500ms for 100K entities.
3. Progress indication is not needed because traversal completes before the user perceives delay.

### US2: Graph View Responsiveness

**As a** knowledge worker,
**I want to** view entity subgraphs without waiting,
**So that** I can navigate my knowledge graph interactively.

**Acceptance criteria:**
1. `kos view graph --from <entity-id> --depth 2` renders in under 200ms for 100K entities.
2. Graph view does not block the CLI for more than 500ms during traversal.

---

## Architecture

### Crate Changes

| Crate               | Change                                                                                           |
| ------------------- | ------------------------------------------------------------------------------------------------ |
| `knowledge-storage` | Rewrite `traversal.rs` — replace CTE with level-by-level BFS. Add index migration to `store.rs`. |

No other crate changes. The `TraversalPort`, `TraversalQuery`, `TraversalConfig`, `TraversalResult`, and `TraversalEdge` types in `knowledge-core` are unchanged.

### Storage Schema Changes

Add the following indexes to the `SqliteStore::new` initialization:

```sql
CREATE INDEX IF NOT EXISTS idx_relationships_source_active
    ON relationships(source_id, is_active);

CREATE INDEX IF NOT EXISTS idx_relationships_target_active
    ON relationships(target_id, is_active);
```

These are composite indexes covering the two most common query patterns:
- `WHERE source_id = ? AND is_active = 1` (outgoing traversal)
- `WHERE target_id = ? AND is_active = 1` (incoming traversal)

The `is_active` column is included to enable index-only filtering: the index can satisfy both the entity lookup and the active check without touching the table row.

### Traversal Algorithm: Level-by-Level BFS

The recursive CTE is replaced by a loop that executes one indexed query per depth level:

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
    let mut results: Vec<(Uuid, u32, Vec<Uuid>)> = Vec::new();
    
    // Pre-parent map for path reconstruction
    let mut parent: HashMap<Uuid, (Uuid, String)> = HashMap::new();
    parent.insert(start_id, (start_id, String::new()));
    
    // Pre-prepared statements for indexed lookups
    let mut outgoing_stmt = if rel_type.is_some() {
        conn.prepare(
            "SELECT r.id, r.target_id, r.relationship_type
             FROM relationships r
             WHERE r.source_id = ?1 AND r.is_active = 1
               AND r.relationship_type = ?2"
        )?
    } else {
        conn.prepare(
            "SELECT r.id, r.target_id, r.relationship_type
             FROM relationships r
             WHERE r.source_id = ?1 AND r.is_active = 1"
        )?
    };
    
    let depth: u32 = 0;
    results.push((start_id, depth, vec![start_id]));
    visited.insert(start_id);
    
    for depth in 1..=max_depth {
        if current_level.is_empty() {
            break;
        }
        
        let mut next_level: Vec<Uuid> = Vec::new();
        let mut level_edges: Vec<(Uuid, Uuid, String)> = Vec::new();
        
        for node_id in &current_level {
            let params: &[&dyn rusqlite::types::ToSql] = if let Some(rt) = rel_type {
                &[&node_id.to_string(), &serde_json::to_string(rt).unwrap()] as &[&dyn rusqlite::types::ToSql]
            } else {
                &[&node_id.to_string()] as &[&dyn rusqlite::types::ToSql]
            };
            
            let rows = outgoing_stmt.query_map(params, |row| {
                let target_id: String = row.get(1)?;
                let rel_type: String = row.get(2)?;
                Ok((target_id, rel_type))
            })?;
            
            for row in rows {
                let (target_id_str, rel_type_str) = row?;
                let target_id = Uuid::parse_str(&target_id_str).unwrap();
                
                if visited.insert(target_id) {
                    parent.insert(target_id, (*node_id, rel_type_str));
                    next_level.push(target_id);
                    level_edges.push((*node_id, target_id, rel_type_str));
                }
            }
        }
        
        // Reconstruct paths for this level
        for node_id in &next_level {
            let path = reconstruct_path_to(node_id, &parent);
            results.push((*node_id, depth, path));
        }
        
        current_level = next_level;
    }
    
    // Apply entity type filter post-expansion
    if let Some(et) = entity_type {
        let type_filter_str = serde_json::to_string(et).unwrap();
        let mut entity_stmt = conn.prepare(
            "SELECT id FROM entities WHERE id = ?1 AND entity_type = ?2 AND is_active = 1"
        )?;
        results.retain(|(id, _, _)| {
            entity_stmt.query_row(
                rusqlite::params![id.to_string(), &type_filter_str],
                |_| Ok(()),
            ).is_ok()
        });
    }
    
    // Sort by depth for deterministic output
    results.sort_by_key(|(_, depth, _)| *depth);
    
    Ok(results)
}
```

#### Path Reconstruction

Path reconstruction builds the traversal path from the parent map without loading all relationships:

```rust
fn reconstruct_path_to(
    node_id: &Uuid,
    parent: &HashMap<Uuid, (Uuid, String)>,
) -> Vec<Uuid> {
    let mut path = Vec::new();
    let mut current = *node_id;
    
    // Walk back to root
    let mut reversed = Vec::new();
    reversed.push(current);
    while let Some((p, _)) = parent.get(&current) {
        if *p == current { break; }  // root
        current = *p;
        reversed.push(current);
    }
    reversed.reverse();
    
    reversed
}
```

#### Edge Reconstruction (Single Pass)

Edge reconstruction loads only the relationships that appear in discovered paths, not the full table:

```rust
fn reconstruct_edges(
    conn: &Connection,
    all_paths: &[Vec<Uuid>],
) -> Result<Vec<TraversalEdge>, StorageError> {
    // Collect all adjacent pairs across all paths
    let mut pairs: HashSet<(Uuid, Uuid)> = HashSet::new();
    for path in all_paths {
        for window in path.windows(2) {
            pairs.insert((window[0], window[1]));
        }
    }
    
    if pairs.is_empty() {
        return Ok(Vec::new());
    }
    
    // Build a query that fetches edges for all pairs in one round-trip
    // Using a temp table or batched OR query
    let placeholders: Vec<String> = pairs.iter().enumerate().map(|(i, _)| {
        format!("(?{}, ?{})", i * 2 + 1, i * 2 + 2)
    }).collect();
    
    let sql = format!(
        "SELECT source_id, target_id, relationship_type 
         FROM relationships 
         WHERE is_active = 1 
           AND (source_id, target_id) IN ({})",
        placeholders.join(", ")
    );
    
    let params: Vec<String> = pairs.iter()
        .flat_map(|(s, t)| vec![s.to_string(), t.to_string()])
        .collect();
    
    let param_refs: Vec<&dyn rusqlite::types::ToSql> = 
        params.iter().map(|s| s as &dyn rusqlite::types::ToSql).collect();
    
    let mut stmt = conn.prepare(&sql)?;
    let rows = stmt.query_map(param_refs.as_slice(), |row| {
        let source: String = row.get(0)?;
        let target: String = row.get(1)?;
        let rel_type: String = row.get(2)?;
        Ok(TraversalEdge {
            source_id: Uuid::parse_str(&source).unwrap(),
            target_id: Uuid::parse_str(&target).unwrap(),
            relationship_type: serde_json::from_str(&format!("\"{}\"", rel_type)).unwrap(),
        })
    })?;
    
    Ok(rows.filter_map(|r| r.ok()).collect())
}
```

### TraversalResult Assembly

The `traverse` function uses a single traversal context to avoid redundant work:

```rust
async fn traverse(
    &self,
    query: &TraversalQuery,
    config: &TraversalConfig,
) -> Result<Vec<TraversalResult>, TraversalError> {
    // ... validation unchanged ...
    
    let conn = self.conn.lock()?;
    
    // Phase 1: Level-by-level BFS
    let (reachable, _direction_label) = match query.direction {
        TraversalDirection::Outgoing => {
            let r = traverse_outgoing(&conn, query.start_id, max_depth, 
                query.relationship_type.as_ref(), query.entity_type_filter.as_ref())?;
            (r, "outgoing")
        }
        // ... similar for Incoming, Both ...
    };
    
    // Phase 2: Single-pass edge reconstruction
    let all_paths: Vec<Vec<Uuid>> = reachable.iter()
        .filter(|(id, _, _)| *id != query.start_id)
        .take(max_results)
        .map(|(_, _, path)| path.clone())
        .collect();
    
    let edges = reconstruct_edges(&conn, &all_paths)?;
    
    // Phase 3: Assemble results
    let edge_lookup: HashMap<(Uuid, Uuid), TraversalEdge> = edges.into_iter()
        .map(|e| ((e.source_id, e.target_id), e))
        .collect();
    
    let results: Vec<TraversalResult> = all_paths.into_iter().map(|path| {
        let path_edges: Vec<TraversalEdge> = path.windows(2)
            .filter_map(|w| edge_lookup.get(&(w[0], w[1])).cloned())
            .collect();
        let depth = (path.len() as u32) - 1;
        TraversalResult { path, edges: path_edges, depth }
    }).collect();
    
    Ok(results)
}
```

### Bidirectional Optimization

Bidirectional traversal expands in both directions simultaneously but uses the same level-by-level approach. The two direction arms share a single visited set and parent map to detect intersection points:

```rust
fn traverse_both(
    conn: &Connection,
    start_id: Uuid,
    max_depth: u32,
    rel_type: Option<&RelationshipType>,
    entity_type: Option<&EntityType>,
) -> Result<Vec<(Uuid, u32, Vec<Uuid>)>, StorageError> {
    let mut visited: HashSet<Uuid> = HashSet::new();
    let mut outgoing_level: Vec<Uuid> = vec![start_id];
    let mut incoming_level: Vec<Uuid> = vec![start_id];
    let mut parent_out: HashMap<Uuid, Uuid> = HashMap::new();
    let mut parent_in: HashMap<Uuid, Uuid> = HashMap::new();
    let mut results: Vec<(Uuid, u32, Vec<Uuid>)> = Vec::new();
    
    visited.insert(start_id);
    results.push((start_id, 0, vec![start_id]));
    
    // Prepare both statements
    let mut out_stmt = prepare_outgoing(conn, rel_type)?;
    let mut in_stmt = prepare_incoming(conn, rel_type)?;
    
    for depth in 1..=max_depth {
        let mut new_visited: Vec<Uuid> = Vec::new();
        
        // Expand outgoing
        for node_id in &outgoing_level {
            expand_level(&mut out_stmt, node_id, &mut visited, &mut new_visited, &mut parent_out, direction)?;
        }
        
        // Expand incoming
        for node_id in &incoming_level {
            expand_level(&mut in_stmt, node_id, &mut visited, &mut new_visited, &mut parent_in, direction)?;
        }
        
        // Reconstruct paths for new nodes
        for node_id in &new_visited {
            let path = reconstruct_bidirectional_path(*node_id, start_id, &parent_out, &parent_in);
            results.push((*node_id, depth, path));
        }
        
        outgoing_level = new_visited.clone();
        incoming_level = new_visited;
    }
    
    // Apply entity type filter
    // ... same as outgoing ...
    
    Ok(results)
}
```

### Storage Independence

All optimization work is within `knowledge-storage/src/adapters/sqlite/traversal.rs`. The index creation is additive — it does not change the schema of canonical tables. The `TraversalPort` trait and all result types are unchanged. Alternative storage adapters (e.g., PostgreSQL, Neo4j) would use their own traversal strategies.

---

## CLI Interface

No changes. All traversal CLI commands and output formats are identical:

```bash
kos traverse <entity-id>
kos traverse <entity-id> --depth 2
kos traverse <entity-id> --depth 3 --type references
kos traverse <entity-id> --entity-type concept
```

### Output Format

Unchanged from PRD-0003. Example:

```
$ kos traverse abc123 --depth 2

Entity: "Attention Is All You Need" (Paper)
  Hop 1:
    -> references -> "Self-Attention Mechanism" (Concept)
    -> authored_by -> "Vaswani" (Person)
  Hop 2:
    "Self-Attention Mechanism" -> related_to -> "Transformer Architecture" (Concept)
    "Vaswani" -> belongs_to -> "Google Brain" (Organization)

Total: 4 entities within 2 hops
```

---

## Acceptance Criteria

### Definition of Done

- [ ] Outgoing traversal at depth 2 and 3 matches current CTE output for chain, tree, diamond, and cyclic graph topologies
- [ ] Incoming traversal matches current CTE output
- [ ] Bidirectional traversal matches current CTE output
- [ ] All existing traversal unit tests and integration tests pass
- [ ] `EXPLAIN QUERY PLAN` confirms index usage for all traversal queries
- [ ] 2-hop traversal at 1K entities completes in < 10ms
- [ ] 3-hop traversal at 1K entities completes in < 50ms
- [ ] 2-hop traversal at 100K entities completes in < 100ms
- [ ] 3-hop traversal at 100K entities completes in < 500ms
- [ ] Database migration creates indexes without data loss
- [ ] Second `SqliteStore::new` call does not fail on existing indexes
- [ ] No canonical data is modified during traversal

### Test Cases

1. **Chain graph** — A → B → C → D: 2-hop traversal from A returns B (hop 1) and C (hop 2), not D.
2. **Tree graph** — A → B, A → C, B → D, B → E: 2-hop from A returns B, C (hop 1) and D, E (hop 2).
3. **Diamond graph** — A → B, A → C, B → D, C → D: 2-hop from A returns B, C (hop 1) and D (hop 2, single result).
4. **Cycle graph** — A → B, B → C, C → A: 3-hop from A returns B (hop 1), C (hop 2), terminates without infinite loop.
5. **Disconnected graph** — A → B, C → D: 2-hop from A returns B (hop 1), does not return C or D.
6. **Depth limiting** — 5-hop chain, depth=2: returns only 2 hops.
7. **Relationship type filter** — A references B, A depends_on C: `--type references` only returns B.
8. **Entity type filter** — A references B (Concept), A references C (Paper): `--entity-type concept` only returns B.
9. **Incoming traversal** — B → A, C → A: incoming 1-hop from A returns B and C.
10. **Bidirectional traversal** — A references B, C references A: both directions 1-hop from A returns B and C.
11. **Result parity** — 100 randomized graphs compare CTE output vs BFS output, must match.
12. **Index migration** — Existing database with no indexes is opened, indexes are created, traversal produces correct results.
13. **Start entity not found** — Returns `TraversalError::StartNotFound`.
14. **Start entity inactive** — Returns `TraversalError::StartNotFound`.
15. **Large graph traversal** — 100K entities, 1M relationships, 2-hop traversal completes in < 100ms.

---

## Testing Strategy

| Level       | Scope                                                   | Framework                  |
| ----------- | ------------------------------------------------------- | -------------------------- |
| Unit        | Level expansion, path reconstruction, edge building     | `#[cfg(test)]` modules     |
| Integration | Full traversal against SQLite with all graph topologies | `tests/` integration tests |
| Benchmark   | Micro-benchmarks per operation + end-to-end traversal   | Criterion                  |
| Property    | Random graph generation, CTE parity verification        | `#[cfg(test)]` + proptest  |

### Test Data

- Synthetic graphs of known topology (chain, tree, diamond, cycle) — identical to existing tests
- Randomized Erdős–Rényi graphs with controlled fan-out for statistical parity testing
- 100K entity + 1M relationship dataset for volume benchmarks

### Parity Testing

The critical test validates that the new implementation produces identical output to the old CTE implementation for any input. This is done by:

1. Generating a random graph with known parameters
2. Running both the old CTE and new BFS traversal
3. Comparing `TraversalResult` vectors for equality (same paths, same edges, same depths)

The old CTE implementation is retained as `traverse_cte` (behind `#[cfg(test)]`) specifically for parity comparison.

---

## Risks and Mitigations

| Risk                                                    | Impact | Likelihood | Mitigation                                                                                     |
| ------------------------------------------------------- | ------ | ---------- | ---------------------------------------------------------------------------------------------- |
| BFS produces different results than CTE for edge cases  | High   | Medium     | Retain CTE code under `#[cfg(test)]` for parity testing on randomized graphs                   |
| Index creation locks database on large datasets         | Medium | Medium     | `CREATE INDEX IF NOT EXISTS` is non-blocking in WAL mode; benchmark index creation time        |
| Deep traversal with high fan-out causes memory pressure | Medium | Low        | Result limit (`max_results`) bounds memory; expansion only tracks visited nodes, not all edges |
| Entity type filter requires additional round-trips      | Low    | Low        | Entity type filter applied post-expansion with a single batched query                          |
| Bidirectional path reconstruction is complex            | Medium | Low        | Dual-parent map approach validated against parity tests                                        |

---

## Dependencies

### External Crates

| Crate       | Purpose       | Justification                                                        |
| ----------- | ------------- | -------------------------------------------------------------------- |
| `rusqlite`  | SQLite access | Already a dependency; used for prepared statements and batch queries |
| `criterion` | Benchmarks    | Already a dependency; used for performance validation                |

### Internal Dependencies

- `docs/architecture/domain-model.md` — Entity, relationship, component types (unchanged)
- `docs/architecture/pipeline.md` — Seven-layer architecture, Layer 5 ownership
- `docs/architecture/architectural-principles.md` — Architectural invariants
- `docs/engineering/prds/prd-0003-graph-exploration-and-plugins.md` — NF1.1, NF1.2 traversal performance targets
- `docs/engineering/prds/prd-0004-implementation-gaps.md` — Gap G-031 (BFS vs CTE deviation)
- `core/knowledge-core/src/ports/traversal.rs` — TraversalPort trait (unchanged)
- `core/knowledge-storage/src/adapters/sqlite/traversal.rs` — Implementation file (rewritten)
- `core/knowledge-storage/src/adapters/sqlite/store.rs` — Schema + index migration

---

## Timeline

| Phase                                     | Duration | Deliverables                                            |
| ----------------------------------------- | -------- | ------------------------------------------------------- |
| Phase 1: Index migration + schema         | 1 day    | `store.rs` index creation, idempotency test             |
| Phase 2: Level-by-level BFS (outgoing)    | 2 days   | `traverse_outgoing` rewrite, unit tests, parity tests   |
| Phase 3: Incoming + bidirectional         | 1 day    | `traverse_incoming`, `traverse_both` rewrite            |
| Phase 4: Edge reconstruction optimization | 1 day    | Single-pass `reconstruct_edges`, path-based edge lookup |
| Phase 5: Integration + parity testing     | 1 day    | All graph topology tests pass, CTE parity verified      |
| Phase 6: Benchmarking + tuning            | 1 day    | Criterion benchmarks at 1K and 100K entities, tuning    |
| Phase 7: Code review + documentation      | 1 day    | ADR for BFS vs CTE decision, PRD-0003 update            |

**Total: ~8 working days**
