use async_trait::async_trait;
use knowledge_core::ports::{
    EntityRepository, StorageError, TraversalConfig, TraversalDirection, TraversalEdge,
    TraversalError, TraversalPort, TraversalQuery, TraversalResult,
};
use rusqlite::Connection;
use std::collections::{HashMap, HashSet, VecDeque};
use uuid::Uuid;

use super::store::SqliteStore;

#[async_trait]
impl TraversalPort for SqliteStore {
    async fn traverse(
        &self,
        query: &TraversalQuery,
        config: &TraversalConfig,
    ) -> Result<Vec<TraversalResult>, TraversalError> {
        let start = EntityRepository::get(self, query.start_id)
            .await?
            .ok_or(TraversalError::StartNotFound(query.start_id))?;

        if !start.is_active {
            return Err(TraversalError::StartNotFound(query.start_id));
        }

        let max_depth = query.max_depth.unwrap_or(config.default_max_depth);
        let max_results = query.max_results.unwrap_or(config.default_max_results);

        let conn = self
            .conn
            .lock()
            .map_err(|e| StorageError::Internal(e.to_string()))?;

        let (reachable, direction_label) = match query.direction {
            TraversalDirection::Outgoing => {
                let reachable = traverse_outgoing(
                    &conn,
                    query.start_id,
                    max_depth,
                    query.relationship_type.as_ref(),
                    query.entity_type_filter.as_ref(),
                )?;
                (reachable, "outgoing")
            }
            TraversalDirection::Incoming => {
                let reachable = traverse_incoming(
                    &conn,
                    query.start_id,
                    max_depth,
                    query.relationship_type.as_ref(),
                    query.entity_type_filter.as_ref(),
                )?;
                (reachable, "incoming")
            }
            TraversalDirection::Both => {
                let reachable = traverse_both(
                    &conn,
                    query.start_id,
                    max_depth,
                    query.relationship_type.as_ref(),
                    query.entity_type_filter.as_ref(),
                )?;
                (reachable, "both")
            }
        };

        let limited: Vec<_> = reachable
            .into_iter()
            .filter(|(id, _, _)| *id != query.start_id)
            .take(max_results)
            .collect();

        let paths: Vec<&[Uuid]> = limited.iter().map(|(_, _, p)| p.as_slice()).collect();
        let all_edges = batch_reconstruct_edges(&conn, &paths, direction_label)?;

        let mut results = Vec::new();
        for ((_node_id, depth, path), edges) in limited.into_iter().zip(all_edges) {
            results.push(TraversalResult { path, edges, depth });
        }

        Ok(results)
    }
}

fn bfs_expand_level(
    stmt: &mut rusqlite::Statement,
    node_id: Uuid,
    visited: &mut HashSet<Uuid>,
    next_level: &mut Vec<Uuid>,
    parent: &mut HashMap<Uuid, (Uuid, String)>,
) -> Result<(), StorageError> {
    let rows = stmt
        .query_map(rusqlite::params![node_id.to_string()], |row| {
            let target: String = row.get(0)?;
            let rtype: String = row.get(1)?;
            Ok((target, rtype))
        })
        .map_err(|e| StorageError::Internal(e.to_string()))?;

    for row in rows {
        let (target_id_str, rel_type_str) =
            row.map_err(|e| StorageError::Internal(e.to_string()))?;
        let target_id =
            Uuid::parse_str(&target_id_str).map_err(|e| StorageError::Internal(e.to_string()))?;

        if visited.insert(target_id) {
            parent.insert(target_id, (node_id, rel_type_str));
            next_level.push(target_id);
        }
    }
    Ok(())
}

fn reconstruct_path_to(node_id: Uuid, parent: &HashMap<Uuid, (Uuid, String)>) -> Vec<Uuid> {
    let mut current = node_id;
    let mut reversed = Vec::new();
    reversed.push(current);
    while let Some((p, _)) = parent.get(&current) {
        if *p == current {
            break;
        }
        current = *p;
        reversed.push(current);
    }
    reversed.reverse();
    reversed
}

fn reconstruct_bidirectional_path(
    node_id: Uuid,
    start_id: Uuid,
    parent_out: &HashMap<Uuid, (Uuid, String)>,
    parent_in: &HashMap<Uuid, (Uuid, String)>,
) -> Vec<Uuid> {
    // Try outgoing parent map first
    let mut path = Vec::new();
    let mut current = node_id;
    path.push(current);
    while let Some(&(p, _)) = parent_out.get(&current) {
        if p == current {
            break;
        }
        path.push(p);
        current = p;
        if current == start_id {
            break;
        }
    }

    if current != start_id {
        // Reset and try incoming parent map
        path.clear();
        current = node_id;
        path.push(current);
        while let Some(&(p, _)) = parent_in.get(&current) {
            if p == current {
                break;
            }
            path.push(p);
            current = p;
            if current == start_id {
                break;
            }
        }
    }

    path.reverse();
    path
}

fn apply_entity_type_filter(
    conn: &Connection,
    results: &mut Vec<(Uuid, u32, Vec<Uuid>)>,
    entity_type: Option<&knowledge_core::features::entity::EntityType>,
) -> Result<(), StorageError> {
    if let Some(et) = entity_type {
        let et_json = serde_json::to_string(et).unwrap();
        let mut et_stmt = conn
            .prepare("SELECT 1 FROM entities WHERE id = ?1 AND entity_type = ?2 AND is_active = 1")
            .map_err(|e| StorageError::Internal(e.to_string()))?;
        results.retain(|(id, _, _)| {
            et_stmt
                .query_row(rusqlite::params![id.to_string(), &et_json], |_| Ok(()))
                .is_ok()
        });
    }
    Ok(())
}

fn traverse_outgoing(
    conn: &Connection,
    start_id: Uuid,
    max_depth: u32,
    rel_type: Option<&knowledge_core::features::relationship::RelationshipType>,
    entity_type: Option<&knowledge_core::features::entity::EntityType>,
) -> Result<Vec<(Uuid, u32, Vec<Uuid>)>, StorageError> {
    let mut visited: HashSet<Uuid> = HashSet::new();
    let mut current_level: Vec<Uuid> = vec![start_id];
    let mut parent: HashMap<Uuid, (Uuid, String)> = HashMap::new();
    parent.insert(start_id, (start_id, String::new()));
    let mut results: Vec<(Uuid, u32, Vec<Uuid>)> = Vec::new();

    visited.insert(start_id);
    results.push((start_id, 0, vec![start_id]));

    let sql = if rel_type.is_some() {
        "SELECT r.target_id, r.relationship_type
         FROM relationships r
         JOIN entities e ON r.target_id = e.id AND e.is_active = 1
         WHERE r.source_id = ?1 AND r.is_active = 1
           AND r.relationship_type = ?2"
    } else {
        "SELECT r.target_id, r.relationship_type
         FROM relationships r
         JOIN entities e ON r.target_id = e.id AND e.is_active = 1
         WHERE r.source_id = ?1 AND r.is_active = 1"
    };
    let mut stmt = conn
        .prepare(sql)
        .map_err(|e| StorageError::Internal(e.to_string()))?;

    for depth in 1..=max_depth {
        if current_level.is_empty() {
            break;
        }

        let mut next_level: Vec<Uuid> = Vec::new();

        for node_id in &current_level {
            if let Some(rt) = rel_type {
                let rt_json = serde_json::to_string(rt).unwrap();
                let mut rt_stmt = conn
                    .prepare(
                        "SELECT r.target_id, r.relationship_type
                         FROM relationships r
                         JOIN entities e ON r.target_id = e.id AND e.is_active = 1
                         WHERE r.source_id = ?1 AND r.is_active = 1
                           AND r.relationship_type = ?2",
                    )
                    .map_err(|e| StorageError::Internal(e.to_string()))?;
                let rows = rt_stmt
                    .query_map(rusqlite::params![node_id.to_string(), rt_json], |row| {
                        let target: String = row.get(0)?;
                        let rtype: String = row.get(1)?;
                        Ok((target, rtype))
                    })
                    .map_err(|e| StorageError::Internal(e.to_string()))?;
                for row in rows {
                    let (target_id_str, rel_type_str) =
                        row.map_err(|e| StorageError::Internal(e.to_string()))?;
                    let target_id = Uuid::parse_str(&target_id_str)
                        .map_err(|e| StorageError::Internal(e.to_string()))?;
                    if visited.insert(target_id) {
                        parent.insert(target_id, (*node_id, rel_type_str));
                        next_level.push(target_id);
                    }
                }
            } else {
                bfs_expand_level(
                    &mut stmt,
                    *node_id,
                    &mut visited,
                    &mut next_level,
                    &mut parent,
                )?;
            }
        }

        for node_id in &next_level {
            let path = reconstruct_path_to(*node_id, &parent);
            results.push((*node_id, depth, path));
        }

        current_level = next_level;
    }

    apply_entity_type_filter(conn, &mut results, entity_type)?;
    results.sort_by_key(|(_, depth, _)| *depth);
    Ok(results)
}

fn traverse_incoming(
    conn: &Connection,
    start_id: Uuid,
    max_depth: u32,
    rel_type: Option<&knowledge_core::features::relationship::RelationshipType>,
    entity_type: Option<&knowledge_core::features::entity::EntityType>,
) -> Result<Vec<(Uuid, u32, Vec<Uuid>)>, StorageError> {
    let mut visited: HashSet<Uuid> = HashSet::new();
    let mut current_level: Vec<Uuid> = vec![start_id];
    let mut parent: HashMap<Uuid, (Uuid, String)> = HashMap::new();
    parent.insert(start_id, (start_id, String::new()));
    let mut results: Vec<(Uuid, u32, Vec<Uuid>)> = Vec::new();

    visited.insert(start_id);
    results.push((start_id, 0, vec![start_id]));

    let mut stmt = conn
        .prepare(
            "SELECT r.source_id, r.relationship_type
             FROM relationships r
             JOIN entities e ON r.source_id = e.id AND e.is_active = 1
             WHERE r.target_id = ?1 AND r.is_active = 1",
        )
        .map_err(|e| StorageError::Internal(e.to_string()))?;

    for depth in 1..=max_depth {
        if current_level.is_empty() {
            break;
        }

        let mut next_level: Vec<Uuid> = Vec::new();

        for node_id in &current_level {
            if let Some(rt) = rel_type {
                let rt_json = serde_json::to_string(rt).unwrap();
                let mut rt_stmt = conn
                    .prepare(
                        "SELECT r.source_id, r.relationship_type
                         FROM relationships r
                         JOIN entities e ON r.source_id = e.id AND e.is_active = 1
                         WHERE r.target_id = ?1 AND r.is_active = 1
                           AND r.relationship_type = ?2",
                    )
                    .map_err(|e| StorageError::Internal(e.to_string()))?;
                let rows = rt_stmt
                    .query_map(rusqlite::params![node_id.to_string(), rt_json], |row| {
                        let source: String = row.get(0)?;
                        let rtype: String = row.get(1)?;
                        Ok((source, rtype))
                    })
                    .map_err(|e| StorageError::Internal(e.to_string()))?;
                for row in rows {
                    let (source_id_str, rel_type_str) =
                        row.map_err(|e| StorageError::Internal(e.to_string()))?;
                    let source_id = Uuid::parse_str(&source_id_str)
                        .map_err(|e| StorageError::Internal(e.to_string()))?;
                    if visited.insert(source_id) {
                        parent.insert(source_id, (*node_id, rel_type_str));
                        next_level.push(source_id);
                    }
                }
            } else {
                bfs_expand_level(
                    &mut stmt,
                    *node_id,
                    &mut visited,
                    &mut next_level,
                    &mut parent,
                )?;
            }
        }

        for node_id in &next_level {
            let path = reconstruct_path_to(*node_id, &parent);
            results.push((*node_id, depth, path));
        }

        current_level = next_level;
    }

    apply_entity_type_filter(conn, &mut results, entity_type)?;
    results.sort_by_key(|(_, depth, _)| *depth);
    Ok(results)
}

fn traverse_both(
    conn: &Connection,
    start_id: Uuid,
    max_depth: u32,
    rel_type: Option<&knowledge_core::features::relationship::RelationshipType>,
    entity_type: Option<&knowledge_core::features::entity::EntityType>,
) -> Result<Vec<(Uuid, u32, Vec<Uuid>)>, StorageError> {
    let mut visited: HashSet<Uuid> = HashSet::new();
    let mut current_level: Vec<Uuid> = vec![start_id];
    let mut parent_out: HashMap<Uuid, (Uuid, String)> = HashMap::new();
    let mut parent_in: HashMap<Uuid, (Uuid, String)> = HashMap::new();
    parent_out.insert(start_id, (start_id, String::new()));
    parent_in.insert(start_id, (start_id, String::new()));
    let mut results: Vec<(Uuid, u32, Vec<Uuid>)> = Vec::new();

    visited.insert(start_id);
    results.push((start_id, 0, vec![start_id]));

    let mut out_stmt = conn
        .prepare(
            "SELECT r.target_id, r.relationship_type
             FROM relationships r
             JOIN entities e ON r.target_id = e.id AND e.is_active = 1
             WHERE r.source_id = ?1 AND r.is_active = 1",
        )
        .map_err(|e| StorageError::Internal(e.to_string()))?;

    let mut in_stmt = conn
        .prepare(
            "SELECT r.source_id, r.relationship_type
             FROM relationships r
             JOIN entities e ON r.source_id = e.id AND e.is_active = 1
             WHERE r.target_id = ?1 AND r.is_active = 1",
        )
        .map_err(|e| StorageError::Internal(e.to_string()))?;

    for depth in 1..=max_depth {
        if current_level.is_empty() {
            break;
        }

        let mut next_level: Vec<Uuid> = Vec::new();

        // Expand outgoing
        for node_id in &current_level {
            if let Some(rt) = rel_type {
                let rt_json = serde_json::to_string(rt).unwrap();
                let mut rt_stmt = conn
                    .prepare(
                        "SELECT r.target_id, r.relationship_type
                         FROM relationships r
                         JOIN entities e ON r.target_id = e.id AND e.is_active = 1
                         WHERE r.source_id = ?1 AND r.is_active = 1
                           AND r.relationship_type = ?2",
                    )
                    .map_err(|e| StorageError::Internal(e.to_string()))?;
                let rows = rt_stmt
                    .query_map(rusqlite::params![node_id.to_string(), rt_json], |row| {
                        let target: String = row.get(0)?;
                        let rtype: String = row.get(1)?;
                        Ok((target, rtype))
                    })
                    .map_err(|e| StorageError::Internal(e.to_string()))?;
                for row in rows {
                    let (target_id_str, rel_type_str) =
                        row.map_err(|e| StorageError::Internal(e.to_string()))?;
                    let target_id = Uuid::parse_str(&target_id_str)
                        .map_err(|e| StorageError::Internal(e.to_string()))?;
                    if visited.insert(target_id) {
                        parent_out.insert(target_id, (*node_id, rel_type_str));
                        next_level.push(target_id);
                    }
                }
            } else {
                bfs_expand_level(
                    &mut out_stmt,
                    *node_id,
                    &mut visited,
                    &mut next_level,
                    &mut parent_out,
                )?;
            }
        }

        // Expand incoming
        for node_id in &current_level {
            if let Some(rt) = rel_type {
                let rt_json = serde_json::to_string(rt).unwrap();
                let mut rt_stmt = conn
                    .prepare(
                        "SELECT r.source_id, r.relationship_type
                         FROM relationships r
                         JOIN entities e ON r.source_id = e.id AND e.is_active = 1
                         WHERE r.target_id = ?1 AND r.is_active = 1
                           AND r.relationship_type = ?2",
                    )
                    .map_err(|e| StorageError::Internal(e.to_string()))?;
                let rows = rt_stmt
                    .query_map(rusqlite::params![node_id.to_string(), rt_json], |row| {
                        let source: String = row.get(0)?;
                        let rtype: String = row.get(1)?;
                        Ok((source, rtype))
                    })
                    .map_err(|e| StorageError::Internal(e.to_string()))?;
                for row in rows {
                    let (source_id_str, rel_type_str) =
                        row.map_err(|e| StorageError::Internal(e.to_string()))?;
                    let source_id = Uuid::parse_str(&source_id_str)
                        .map_err(|e| StorageError::Internal(e.to_string()))?;
                    if visited.insert(source_id) {
                        parent_in.insert(source_id, (*node_id, rel_type_str));
                        next_level.push(source_id);
                    }
                }
            } else {
                bfs_expand_level(
                    &mut in_stmt,
                    *node_id,
                    &mut visited,
                    &mut next_level,
                    &mut parent_in,
                )?;
            }
        }

        for node_id in &next_level {
            let path = reconstruct_bidirectional_path(*node_id, start_id, &parent_out, &parent_in);
            results.push((*node_id, depth, path));
        }

        current_level = next_level;
    }

    apply_entity_type_filter(conn, &mut results, entity_type)?;
    results.sort_by_key(|(_, depth, _)| *depth);
    Ok(results)
}

#[cfg(test)]
pub(crate) mod cte_legacy {
    use knowledge_core::features::entity::EntityType;
    use knowledge_core::features::relationship::RelationshipType;
    use rusqlite::Connection;
    use uuid::Uuid;

    use knowledge_core::ports::StorageError;

    fn rel_filter_str(rel_type: Option<&RelationshipType>) -> String {
        rel_type
            .map(|rt| serde_json::to_string(rt).unwrap())
            .map(|s| format!("AND r.relationship_type = '{}'", s.replace('\'', "''")))
            .unwrap_or_default()
    }

    fn entity_filter_str(entity_type: Option<&EntityType>) -> String {
        entity_type
            .map(|et| serde_json::to_string(et).unwrap())
            .map(|s| format!("AND e.entity_type = '{}'", s.replace('\'', "''")))
            .unwrap_or_default()
    }

    pub fn traverse_outgoing(
        conn: &Connection,
        start_id: Uuid,
        max_depth: u32,
        rel_type: Option<&RelationshipType>,
        entity_type: Option<&EntityType>,
    ) -> Result<Vec<(Uuid, u32, Vec<Uuid>)>, StorageError> {
        let rel_filter = rel_filter_str(rel_type);
        let entity_filter = entity_filter_str(entity_type);

        let sql = format!(
            "WITH RECURSIVE traversal(id, depth, path) AS (
                SELECT se.id, 0, se.id
                FROM entities se
                WHERE se.id = ?1 AND se.is_active = 1
                UNION
                SELECT r.target_id, t.depth + 1, t.path || ',' || r.target_id
                FROM relationships r
                JOIN traversal t ON r.source_id = t.id
                JOIN entities e ON r.target_id = e.id
                WHERE t.depth < ?2
                  AND r.is_active = 1
                  AND e.is_active = 1
                  AND (',' || t.path || ',') NOT LIKE ('%,' || e.id || ',%')
                  {rel_filter} {entity_filter}
            )
            SELECT id, depth, path FROM (
                SELECT id, depth, path, ROW_NUMBER() OVER (PARTITION BY id ORDER BY depth, length(path)) AS rn
                FROM traversal
            ) WHERE rn = 1 ORDER BY depth",
            rel_filter = rel_filter,
            entity_filter = entity_filter,
        );

        let mut stmt = conn
            .prepare(&sql)
            .map_err(|e| StorageError::Internal(e.to_string()))?;
        let rows = stmt
            .query_map(rusqlite::params![start_id.to_string(), max_depth], |row| {
                let id = Uuid::parse_str(&row.get::<_, String>(0)?).unwrap();
                let depth: u32 = row.get(1)?;
                let path_str: String = row.get(2)?;
                let path: Vec<Uuid> = path_str
                    .split(',')
                    .filter(|s| !s.is_empty())
                    .map(|s| Uuid::parse_str(s).unwrap())
                    .collect();
                Ok((id, depth, path))
            })
            .map_err(|e| StorageError::Internal(e.to_string()))?;
        Ok(rows.filter_map(|r| r.ok()).collect())
    }

    pub fn traverse_incoming(
        conn: &Connection,
        start_id: Uuid,
        max_depth: u32,
        rel_type: Option<&RelationshipType>,
        entity_type: Option<&EntityType>,
    ) -> Result<Vec<(Uuid, u32, Vec<Uuid>)>, StorageError> {
        let rel_filter = rel_filter_str(rel_type);
        let entity_filter = entity_filter_str(entity_type);

        let sql = format!(
            "WITH RECURSIVE traversal(id, depth, path) AS (
                SELECT se.id, 0, se.id
                FROM entities se
                WHERE se.id = ?1 AND se.is_active = 1
                UNION
                SELECT r.source_id, t.depth + 1, t.path || ',' || r.source_id
                FROM relationships r
                JOIN traversal t ON r.target_id = t.id
                JOIN entities e ON r.source_id = e.id
                WHERE t.depth < ?2
                  AND r.is_active = 1
                  AND e.is_active = 1
                  AND (',' || t.path || ',') NOT LIKE ('%,' || e.id || ',%')
                  {rel_filter} {entity_filter}
            )
            SELECT id, depth, path FROM (
                SELECT id, depth, path, ROW_NUMBER() OVER (PARTITION BY id ORDER BY depth, length(path)) AS rn
                FROM traversal
            ) WHERE rn = 1 ORDER BY depth",
            rel_filter = rel_filter,
            entity_filter = entity_filter,
        );

        let mut stmt = conn
            .prepare(&sql)
            .map_err(|e| StorageError::Internal(e.to_string()))?;
        let rows = stmt
            .query_map(rusqlite::params![start_id.to_string(), max_depth], |row| {
                let id = Uuid::parse_str(&row.get::<_, String>(0)?).unwrap();
                let depth: u32 = row.get(1)?;
                let path_str: String = row.get(2)?;
                let path: Vec<Uuid> = path_str
                    .split(',')
                    .filter(|s| !s.is_empty())
                    .map(|s| Uuid::parse_str(s).unwrap())
                    .collect();
                Ok((id, depth, path))
            })
            .map_err(|e| StorageError::Internal(e.to_string()))?;
        Ok(rows.filter_map(|r| r.ok()).collect())
    }

    pub fn traverse_both(
        conn: &Connection,
        start_id: Uuid,
        max_depth: u32,
        rel_type: Option<&RelationshipType>,
        entity_type: Option<&EntityType>,
    ) -> Result<Vec<(Uuid, u32, Vec<Uuid>)>, StorageError> {
        let rel_filter = rel_filter_str(rel_type);
        let entity_filter = entity_filter_str(entity_type);

        let sql = format!(
            "WITH RECURSIVE traversal(id, depth, path) AS (
                SELECT se.id, 0, se.id
                FROM entities se
                WHERE se.id = ?1 AND se.is_active = 1
                UNION
                SELECT r.target_id, t.depth + 1, t.path || ',' || r.target_id
                FROM relationships r
                JOIN traversal t ON r.source_id = t.id
                JOIN entities e ON r.target_id = e.id
                WHERE t.depth < ?2
                  AND r.is_active = 1
                  AND e.is_active = 1
                  AND (',' || t.path || ',') NOT LIKE ('%,' || e.id || ',%')
                  {rel_filter} {entity_filter}
                UNION
                SELECT r.source_id, t.depth + 1, t.path || ',' || r.source_id
                FROM relationships r
                JOIN traversal t ON r.target_id = t.id
                JOIN entities e ON r.source_id = e.id
                WHERE t.depth < ?2
                  AND r.is_active = 1
                  AND e.is_active = 1
                  AND (',' || t.path || ',') NOT LIKE ('%,' || e.id || ',%')
                  {rel_filter} {entity_filter}
            )
            SELECT id, depth, path FROM (
                SELECT id, depth, path, ROW_NUMBER() OVER (PARTITION BY id ORDER BY depth, length(path)) AS rn
                FROM traversal
            ) WHERE rn = 1 ORDER BY depth",
            rel_filter = rel_filter,
            entity_filter = entity_filter,
        );

        let mut stmt = conn
            .prepare(&sql)
            .map_err(|e| StorageError::Internal(e.to_string()))?;
        let rows = stmt
            .query_map(rusqlite::params![start_id.to_string(), max_depth], |row| {
                let id = Uuid::parse_str(&row.get::<_, String>(0)?).unwrap();
                let depth: u32 = row.get(1)?;
                let path_str: String = row.get(2)?;
                let path: Vec<Uuid> = path_str
                    .split(',')
                    .filter(|s| !s.is_empty())
                    .map(|s| Uuid::parse_str(s).unwrap())
                    .collect();
                Ok((id, depth, path))
            })
            .map_err(|e| StorageError::Internal(e.to_string()))?;
        Ok(rows.filter_map(|r| r.ok()).collect())
    }
}

#[allow(dead_code)]
fn reconstruct_path(
    conn: &Connection,
    start_id: Uuid,
    target_id: Uuid,
) -> Result<Vec<Uuid>, StorageError> {
    if start_id == target_id {
        return Ok(vec![start_id]);
    }

    let mut visited: HashMap<Uuid, Uuid> = HashMap::new();
    let mut queue = VecDeque::new();
    queue.push_back(start_id);
    visited.insert(start_id, start_id);

    let mut stmt = conn
        .prepare("SELECT source_id, target_id FROM relationships WHERE is_active = 1")
        .map_err(|e| StorageError::Internal(e.to_string()))?;

    let edges: Vec<(Uuid, Uuid)> = {
        let rows = stmt
            .query_map([], |row| {
                let source = Uuid::parse_str(&row.get::<_, String>(0)?).unwrap();
                let target = Uuid::parse_str(&row.get::<_, String>(1)?).unwrap();
                Ok((source, target))
            })
            .map_err(|e| StorageError::Internal(e.to_string()))?;
        rows.filter_map(|r| r.ok()).collect()
    };

    let mut outgoing: HashMap<Uuid, Vec<Uuid>> = HashMap::new();
    let mut incoming: HashMap<Uuid, Vec<Uuid>> = HashMap::new();
    for (source, target) in &edges {
        outgoing.entry(*source).or_default().push(*target);
        incoming.entry(*target).or_default().push(*source);
    }

    while let Some(current) = queue.pop_front() {
        if current == target_id {
            let mut path = vec![target_id];
            let mut current = target_id;
            while let Some(&prev) = visited.get(&current) {
                if prev == current {
                    break;
                }
                path.push(prev);
                current = prev;
            }
            path.reverse();
            return Ok(path);
        }

        if let Some(targets) = outgoing.get(&current) {
            for &next in targets {
                if let std::collections::hash_map::Entry::Vacant(e) = visited.entry(next) {
                    e.insert(current);
                    queue.push_back(next);
                }
            }
        }

        if let Some(sources) = incoming.get(&current) {
            for &prev in sources {
                if let std::collections::hash_map::Entry::Vacant(e) = visited.entry(prev) {
                    e.insert(current);
                    queue.push_back(prev);
                }
            }
        }
    }

    Ok(vec![start_id, target_id])
}

fn batch_reconstruct_edges(
    conn: &Connection,
    paths: &[&[Uuid]],
    direction: &str,
) -> Result<Vec<Vec<TraversalEdge>>, StorageError> {
    // Collect unique (source, target) pairs from all paths
    let mut pairs: HashSet<(Uuid, Uuid)> = HashSet::new();
    for path in paths {
        for window in path.windows(2) {
            pairs.insert((window[0], window[1]));
            if direction == "both" {
                pairs.insert((window[1], window[0]));
            }
        }
    }

    if pairs.is_empty() {
        return Ok(paths.iter().map(|_| Vec::new()).collect());
    }

    // Query all needed edges in a single pass
    let mut stmt = conn
        .prepare(
            "SELECT source_id, target_id, relationship_type FROM relationships WHERE is_active = 1",
        )
        .map_err(|e| StorageError::Internal(e.to_string()))?;

    let mut edge_map: HashMap<(Uuid, Uuid), String> = HashMap::new();
    {
        let rows = stmt
            .query_map([], |row| {
                let source = Uuid::parse_str(&row.get::<_, String>(0)?).unwrap();
                let target = Uuid::parse_str(&row.get::<_, String>(1)?).unwrap();
                let rel_type: String = row.get(2)?;
                Ok((source, target, rel_type))
            })
            .map_err(|e| StorageError::Internal(e.to_string()))?;
        for row in rows {
            let (source, target, rel_type) =
                row.map_err(|e| StorageError::Internal(e.to_string()))?;
            let key = (source, target);
            if pairs.contains(&key) || (direction == "both" && pairs.contains(&(target, source))) {
                edge_map.entry(key).or_insert(rel_type);
            }
        }
    }

    // Resolve edges for each path
    let mut all_edges = Vec::with_capacity(paths.len());
    for path in paths {
        let mut edges = Vec::new();
        for window in path.windows(2) {
            let from = window[0];
            let to = window[1];

            if let Some(rel_type_str) = edge_map.get(&(from, to)) {
                let rel_type = serde_json::from_str::<
                    knowledge_core::features::relationship::RelationshipType,
                >(&format!("\"{}\"", rel_type_str))
                .unwrap_or(knowledge_core::features::relationship::RelationshipType::References);
                edges.push(TraversalEdge {
                    source_id: from,
                    target_id: to,
                    relationship_type: rel_type,
                });
            } else if direction == "both" {
                if let Some(rel_type_str) = edge_map.get(&(to, from)) {
                    let rel_type = serde_json::from_str::<
                        knowledge_core::features::relationship::RelationshipType,
                    >(&format!("\"{}\"", rel_type_str))
                    .unwrap_or(
                        knowledge_core::features::relationship::RelationshipType::References,
                    );
                    edges.push(TraversalEdge {
                        source_id: to,
                        target_id: from,
                        relationship_type: rel_type,
                    });
                }
            }
        }
        all_edges.push(edges);
    }

    Ok(all_edges)
}

#[cfg(test)]
mod tests {
    use super::*;
    use knowledge_core::ports::TraversalDirection;
    use rand::Rng;
    use rand::SeedableRng;
    use rand_chacha::ChaCha8Rng;

    /// Generate a random graph for parity testing.
    /// Returns (connection, start_id) with a deterministic random graph.
    fn generate_random_graph(
        seed: u64,
        entity_count: usize,
        avg_edges: usize,
    ) -> (Connection, Uuid) {
        let conn = Connection::open_in_memory().unwrap();
        conn.execute_batch(
            "CREATE TABLE entities (
                id TEXT PRIMARY KEY,
                entity_type TEXT NOT NULL DEFAULT '\"Concept\"',
                is_active INTEGER NOT NULL DEFAULT 1,
                created_at TEXT NOT NULL DEFAULT '2024-01-01T00:00:00Z',
                updated_at TEXT NOT NULL DEFAULT '2024-01-01T00:00:00Z',
                version INTEGER NOT NULL DEFAULT 1
            );
            CREATE TABLE relationships (
                id TEXT PRIMARY KEY,
                source_id TEXT NOT NULL,
                target_id TEXT NOT NULL,
                relationship_type TEXT NOT NULL DEFAULT '\"References\"',
                is_active INTEGER NOT NULL DEFAULT 1,
                created_at TEXT NOT NULL DEFAULT '2024-01-01T00:00:00Z'
            );",
        )
        .unwrap();

        let mut rng = ChaCha8Rng::seed_from_u64(seed);
        let mut entity_ids: Vec<Uuid> = (0..entity_count)
            .map(|_| {
                let id = Uuid::new_v4();
                conn.execute(
                    "INSERT INTO entities (id, entity_type, is_active, created_at, updated_at, version)
                     VALUES (?1, '\"Concept\"', 1, '2024-01-01T00:00:00Z', '2024-01-01T00:00:00Z', 1)",
                    rusqlite::params![id.to_string()],
                )
                .unwrap();
                id
            })
            .collect();

        for i in 0..entity_count {
            for _ in 0..avg_edges {
                let target_idx: usize = rng.gen_range(0..entity_count);
                if target_idx != i {
                    let rel_id = Uuid::new_v4();
                    conn.execute(
                        "INSERT INTO relationships (id, source_id, target_id, relationship_type, is_active, created_at)
                         VALUES (?1, ?2, ?3, '\"References\"', 1, '2024-01-01T00:00:00Z')",
                        rusqlite::params![
                            rel_id.to_string(),
                            entity_ids[i].to_string(),
                            entity_ids[target_idx].to_string(),
                        ],
                    )
                    .unwrap();
                }
            }
        }

        let start_id = entity_ids.remove(0);
        (conn, start_id)
    }

    /// BFS wrapper that dispatches by direction.
    fn bfs_traverse(
        conn: &Connection,
        start_id: Uuid,
        max_depth: u32,
        direction: &TraversalDirection,
        rel_type: Option<&knowledge_core::features::relationship::RelationshipType>,
        entity_type: Option<&knowledge_core::features::entity::EntityType>,
    ) -> Vec<(Uuid, u32, Vec<Uuid>)> {
        match direction {
            TraversalDirection::Outgoing => {
                traverse_outgoing(conn, start_id, max_depth, rel_type, entity_type).unwrap()
            }
            TraversalDirection::Incoming => {
                traverse_incoming(conn, start_id, max_depth, rel_type, entity_type).unwrap()
            }
            TraversalDirection::Both => {
                traverse_both(conn, start_id, max_depth, rel_type, entity_type).unwrap()
            }
        }
    }

    /// CTE wrapper that dispatches by direction.
    fn cte_traverse(
        conn: &Connection,
        start_id: Uuid,
        max_depth: u32,
        direction: &TraversalDirection,
        rel_type: Option<&knowledge_core::features::relationship::RelationshipType>,
        entity_type: Option<&knowledge_core::features::entity::EntityType>,
    ) -> Vec<(Uuid, u32, Vec<Uuid>)> {
        match direction {
            TraversalDirection::Outgoing => {
                cte_legacy::traverse_outgoing(conn, start_id, max_depth, rel_type, entity_type)
                    .unwrap()
            }
            TraversalDirection::Incoming => {
                cte_legacy::traverse_incoming(conn, start_id, max_depth, rel_type, entity_type)
                    .unwrap()
            }
            TraversalDirection::Both => {
                cte_legacy::traverse_both(conn, start_id, max_depth, rel_type, entity_type).unwrap()
            }
        }
    }

    /// Parity test: compare BFS output against CTE legacy code for 50 random graphs.
    #[test]
    fn test_bfs_matches_cte_random_graphs() {
        for seed in 0..50 {
            let (conn, start_id) = generate_random_graph(seed, 100, 5);
            let directions = [
                TraversalDirection::Outgoing,
                TraversalDirection::Incoming,
                TraversalDirection::Both,
            ];

            for depth in 1..=4 {
                for direction in &directions {
                    let bfs_results = bfs_traverse(&conn, start_id, depth, direction, None, None);
                    let cte_results = cte_traverse(&conn, start_id, depth, direction, None, None);

                    // Compare reachable node IDs at each depth (paths may differ
                    // due to BFS discovering first vs CTE selecting shortest).
                    let bfs_set: HashSet<(Uuid, u32)> =
                        bfs_results.iter().map(|(id, d, _)| (*id, *d)).collect();
                    let cte_set: HashSet<(Uuid, u32)> =
                        cte_results.iter().map(|(id, d, _)| (*id, *d)).collect();
                    assert_eq!(
                        bfs_set, cte_set,
                        "Reachable node mismatch for seed={}, depth={}, direction={:?}\nbfs extra: {:?}\ncte extra: {:?}",
                        seed, depth, direction,
                        &bfs_set - &cte_set,
                        &cte_set - &bfs_set,
                    );
                }
            }
        }
    }
}
