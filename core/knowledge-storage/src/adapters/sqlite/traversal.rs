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

        let mut results = Vec::new();
        for (node_id, depth, path_str) in &limited {
            let path: Vec<Uuid> = path_str
                .split(',')
                .filter(|s| !s.is_empty())
                .map(|s| Uuid::parse_str(s).unwrap())
                .collect();
            let edges = reconstruct_edges(&conn, &path, *node_id, direction_label)?;
            results.push(TraversalResult {
                path,
                edges,
                depth: *depth,
            });
        }

        Ok(results)
    }
}

fn traverse_outgoing(
    conn: &Connection,
    start_id: Uuid,
    max_depth: u32,
    rel_type: Option<&knowledge_core::features::relationship::RelationshipType>,
    entity_type: Option<&knowledge_core::features::entity::EntityType>,
) -> Result<Vec<(Uuid, u32, String)>, StorageError> {
    let rel_type_json = rel_type
        .map(|rt| serde_json::to_string(rt).unwrap())
        .map(|s| format!("AND r.relationship_type = '{}'", s.replace('\'', "''")))
        .unwrap_or_default();

    let entity_type_json = entity_type
        .map(|et| serde_json::to_string(et).unwrap())
        .map(|s| format!("AND e.entity_type = '{}'", s.replace('\'', "''")))
        .unwrap_or_default();

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
        rel_filter = rel_type_json,
        entity_filter = entity_type_json,
    );

    let mut stmt = conn
        .prepare(&sql)
        .map_err(|e| StorageError::Internal(e.to_string()))?;
    let rows = stmt
        .query_map(rusqlite::params![start_id.to_string(), max_depth], |row| {
            let id = Uuid::parse_str(&row.get::<_, String>(0)?).unwrap();
            let depth: u32 = row.get(1)?;
            let path: String = row.get(2)?;
            Ok((id, depth, path))
        })
        .map_err(|e| StorageError::Internal(e.to_string()))?;
    Ok(rows.filter_map(|r| r.ok()).collect())
}

fn traverse_incoming(
    conn: &Connection,
    start_id: Uuid,
    max_depth: u32,
    rel_type: Option<&knowledge_core::features::relationship::RelationshipType>,
    entity_type: Option<&knowledge_core::features::entity::EntityType>,
) -> Result<Vec<(Uuid, u32, String)>, StorageError> {
    let rel_type_json = rel_type
        .map(|rt| serde_json::to_string(rt).unwrap())
        .map(|s| format!("AND r.relationship_type = '{}'", s.replace('\'', "''")))
        .unwrap_or_default();

    let entity_type_json = entity_type
        .map(|et| serde_json::to_string(et).unwrap())
        .map(|s| format!("AND e.entity_type = '{}'", s.replace('\'', "''")))
        .unwrap_or_default();

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
        rel_filter = rel_type_json,
        entity_filter = entity_type_json,
    );

    let mut stmt = conn
        .prepare(&sql)
        .map_err(|e| StorageError::Internal(e.to_string()))?;
    let rows = stmt
        .query_map(rusqlite::params![start_id.to_string(), max_depth], |row| {
            let id = Uuid::parse_str(&row.get::<_, String>(0)?).unwrap();
            let depth: u32 = row.get(1)?;
            let path: String = row.get(2)?;
            Ok((id, depth, path))
        })
        .map_err(|e| StorageError::Internal(e.to_string()))?;
    Ok(rows.filter_map(|r| r.ok()).collect())
}

fn traverse_both(
    conn: &Connection,
    start_id: Uuid,
    max_depth: u32,
    rel_type: Option<&knowledge_core::features::relationship::RelationshipType>,
    entity_type: Option<&knowledge_core::features::entity::EntityType>,
) -> Result<Vec<(Uuid, u32, String)>, StorageError> {
    let rel_type_json = rel_type
        .map(|rt| serde_json::to_string(rt).unwrap())
        .map(|s| format!("AND r.relationship_type = '{}'", s.replace('\'', "''")))
        .unwrap_or_default();

    let entity_type_json = entity_type
        .map(|et| serde_json::to_string(et).unwrap())
        .map(|s| format!("AND e.entity_type = '{}'", s.replace('\'', "''")))
        .unwrap_or_default();

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
        rel_filter = rel_type_json,
        entity_filter = entity_type_json,
    );

    let mut stmt = conn
        .prepare(&sql)
        .map_err(|e| StorageError::Internal(e.to_string()))?;
    let rows = stmt
        .query_map(rusqlite::params![start_id.to_string(), max_depth], |row| {
            let id = Uuid::parse_str(&row.get::<_, String>(0)?).unwrap();
            let depth: u32 = row.get(1)?;
            let path: String = row.get(2)?;
            Ok((id, depth, path))
        })
        .map_err(|e| StorageError::Internal(e.to_string()))?;
    Ok(rows.filter_map(|r| r.ok()).collect())
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

fn reconstruct_edges(
    conn: &Connection,
    path: &[Uuid],
    _target_id: Uuid,
    direction: &str,
) -> Result<Vec<TraversalEdge>, StorageError> {
    if path.len() <= 1 {
        return Ok(Vec::new());
    }

    let mut stmt = conn
        .prepare(
            "SELECT source_id, target_id, relationship_type FROM relationships WHERE is_active = 1",
        )
        .map_err(|e| StorageError::Internal(e.to_string()))?;

    let mut edge_map: HashMap<(Uuid, Uuid), String> = HashMap::new();
    let outgoing_set: HashSet<(Uuid, Uuid)> = {
        let rows = stmt
            .query_map([], |row| {
                let source = Uuid::parse_str(&row.get::<_, String>(0)?).unwrap();
                let target = Uuid::parse_str(&row.get::<_, String>(1)?).unwrap();
                let rel_type: String = row.get(2)?;
                Ok((source, target, rel_type))
            })
            .map_err(|e| StorageError::Internal(e.to_string()))?;
        rows.filter_map(|r| r.ok())
            .map(|(s, t, rt)| {
                edge_map.insert((s, t), rt);
                (s, t)
            })
            .collect()
    };

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
        } else if direction == "both" && outgoing_set.contains(&(to, from)) {
            if let Some(rel_type_str) = edge_map.get(&(to, from)) {
                let rel_type = serde_json::from_str::<
                    knowledge_core::features::relationship::RelationshipType,
                >(&format!("\"{}\"", rel_type_str))
                .unwrap_or(knowledge_core::features::relationship::RelationshipType::References);
                edges.push(TraversalEdge {
                    source_id: to,
                    target_id: from,
                    relationship_type: rel_type,
                });
            }
        }
    }

    if edges.is_empty() && path.len() == 2 {
        let from = path[0];
        let to = path[1];
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
                .unwrap_or(knowledge_core::features::relationship::RelationshipType::References);
                edges.push(TraversalEdge {
                    source_id: to,
                    target_id: from,
                    relationship_type: rel_type,
                });
            }
        }
    }

    Ok(edges)
}
