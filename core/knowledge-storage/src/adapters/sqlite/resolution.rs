use async_trait::async_trait;
use knowledge_core::features::component::ComponentType;
use knowledge_core::features::entity::Entity;
use knowledge_core::ports::{EntityResolver, MergeAuditEntry, ResolutionCandidate, StorageError};
use uuid::Uuid;

use super::store::{SqliteStore, ENTITY_COLS};

#[async_trait]
impl EntityResolver for SqliteStore {
    async fn find_candidates(
        &self,
        entity: &Entity,
        title: &str,
        content: Option<&str>,
    ) -> Result<Vec<ResolutionCandidate>, StorageError> {
        let conn = self
            .conn
            .lock()
            .map_err(|e| StorageError::Internal(e.to_string()))?;

        let entity_type_json = serde_json::to_string(&entity.entity_type).unwrap();

        let mut stmt = conn
            .prepare(&format!(
                "SELECT {cols} FROM entities e
                 WHERE e.entity_type = ?1 AND e.is_active = 1 AND e.id != ?2",
                cols = ENTITY_COLS
                    .replacen("id,", "e.id,", 1)
                    .replacen("entity_type,", "e.entity_type,", 1)
                    .replacen("is_active,", "e.is_active,", 1)
                    .replacen("created_at,", "e.created_at,", 1)
                    .replacen("updated_at,", "e.updated_at,", 1)
                    .replacen("version", "e.version", 1)
            ))
            .map_err(|e| StorageError::Internal(e.to_string()))?;

        let entities: Vec<Entity> = stmt
            .query_map(
                rusqlite::params![entity_type_json, entity.id.to_string()],
                Self::parse_entity,
            )
            .map_err(|e| StorageError::Internal(e.to_string()))?
            .filter_map(|r| r.ok())
            .collect();

        let mut title_stmt = conn
            .prepare(
                "SELECT c.data FROM components c WHERE c.entity_id = ?1 AND c.component_type = ?2",
            )
            .map_err(|e| StorageError::Internal(e.to_string()))?;

        let mut content_stmt = conn
            .prepare(
                "SELECT c.data FROM components c WHERE c.entity_id = ?1 AND c.component_type = ?2",
            )
            .map_err(|e| StorageError::Internal(e.to_string()))?;

        let title_ct = Self::component_type_str(&ComponentType::Title);
        let content_ct = Self::component_type_str(&ComponentType::Content);

        let mut entity_data: Vec<(Entity, String, Option<String>, crate::fuzzy::EntitySignals)> =
            Vec::new();
        for e in &entities {
            let title_json: Result<String, _> = title_stmt
                .query_row(rusqlite::params![e.id.to_string(), title_ct], |row| {
                    row.get(0)
                });

            if let Ok(title_json) = title_json {
                if let Ok(t) = serde_json::from_str::<String>(&title_json) {
                    let content_json: Result<String, _> = content_stmt
                        .query_row(rusqlite::params![e.id.to_string(), content_ct], |row| {
                            row.get(0)
                        });
                    let c = content_json
                        .ok()
                        .and_then(|json| serde_json::from_str::<String>(&json).ok());

                    let signals = Self::fetch_entity_signals(&conn, e.id)?;

                    entity_data.push((e.clone(), t, c, signals));
                }
            }
        }

        let incoming_signals = Self::fetch_entity_signals(&conn, entity.id)?;

        let resolver = crate::fuzzy::FuzzyEntityResolver::new();
        let composite_candidates = resolver.find_candidates_composite(
            entity,
            title,
            content,
            &incoming_signals,
            &entity_data,
        );

        let candidates = composite_candidates
            .into_iter()
            .map(|c| ResolutionCandidate {
                entity_id: c.entity_id,
                confidence: c.confidence,
                reason: c.reason,
                title_score: Some(c.title_score),
                content_score: Some(c.content_score),
                metadata_score: Some(c.metadata_score),
                structural_score: Some(c.structural_score),
            })
            .collect();

        Ok(candidates)
    }

    async fn merge(
        &self,
        canonical_id: Uuid,
        duplicate_id: Uuid,
        _confidence: f64,
    ) -> Result<(), StorageError> {
        let conn = self
            .conn
            .lock()
            .map_err(|e| StorageError::Internal(e.to_string()))?;

        conn.execute_batch("BEGIN IMMEDIATE")
            .map_err(|e| StorageError::Internal(e.to_string()))?;

        let result = (|| -> Result<(), StorageError> {
            conn.execute(
                "UPDATE relationships SET source_id = ?1 WHERE source_id = ?2",
                rusqlite::params![canonical_id.to_string(), duplicate_id.to_string()],
            )
            .map_err(|e| StorageError::Internal(e.to_string()))?;

            conn.execute(
                "UPDATE relationships SET target_id = ?1 WHERE target_id = ?2",
                rusqlite::params![canonical_id.to_string(), duplicate_id.to_string()],
            )
            .map_err(|e| StorageError::Internal(e.to_string()))?;

            conn.execute(
                "DELETE FROM relationships WHERE source_id = ?1 AND target_id = ?1",
                rusqlite::params![canonical_id.to_string()],
            )
            .map_err(|e| StorageError::Internal(e.to_string()))?;

            conn.execute(
                "UPDATE components SET entity_id = ?1 WHERE entity_id = ?2",
                rusqlite::params![canonical_id.to_string(), duplicate_id.to_string()],
            )
            .map_err(|e| StorageError::Internal(e.to_string()))?;

            conn.execute(
                "DELETE FROM entities_fts WHERE entity_id = ?1",
                rusqlite::params![duplicate_id.to_string()],
            )
            .map_err(|e| StorageError::Internal(e.to_string()))?;

            conn.execute(
                "DELETE FROM entity_versions WHERE entity_id = ?1",
                rusqlite::params![duplicate_id.to_string()],
            )
            .map_err(|e| StorageError::Internal(e.to_string()))?;

            conn.execute(
                "DELETE FROM events WHERE entity_id = ?1",
                rusqlite::params![duplicate_id.to_string()],
            )
            .map_err(|e| StorageError::Internal(e.to_string()))?;

            conn.execute(
                "DELETE FROM entities WHERE id = ?1",
                rusqlite::params![duplicate_id.to_string()],
            )
            .map_err(|e| StorageError::Internal(e.to_string()))?;

            Ok(())
        })();

        match result {
            Ok(()) => {
                conn.execute_batch("COMMIT")
                    .map_err(|e| StorageError::Internal(e.to_string()))?;
                Ok(())
            }
            Err(e) => {
                conn.execute_batch("ROLLBACK")
                    .map_err(|_| StorageError::Internal("rollback failed".to_string()))?;
                Err(e)
            }
        }
    }

    async fn log_merge(&self, entry: &MergeAuditEntry) -> Result<(), StorageError> {
        let conn = self
            .conn
            .lock()
            .map_err(|e| StorageError::Internal(e.to_string()))?;

        conn.execute(
            "INSERT INTO resolution_log (id, source_id, source_title, target_id, target_title, strategy, confidence, timestamp, reason, snapshot)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10)",
            rusqlite::params![
                entry.id.to_string(),
                entry.source_id.to_string(),
                entry.source_title,
                entry.target_id.to_string(),
                entry.target_title,
                entry.strategy,
                entry.confidence,
                entry.timestamp.to_rfc3339(),
                entry.reason,
                entry.snapshot,
            ],
        ).map_err(|e| StorageError::Internal(e.to_string()))?;

        Ok(())
    }

    async fn undo_merge(&self, merge_id: Uuid) -> Result<(), StorageError> {
        let conn = self
            .conn
            .lock()
            .map_err(|e| StorageError::Internal(e.to_string()))?;

        let entry = conn
            .query_row(
                "SELECT source_id, target_id, snapshot FROM resolution_log WHERE id = ?1",
                rusqlite::params![merge_id.to_string()],
                |row| {
                    let source_id: String = row.get(0)?;
                    let target_id: String = row.get(1)?;
                    let snapshot: Option<String> = row.get(2)?;
                    Ok((
                        uuid::Uuid::parse_str(&source_id).unwrap(),
                        uuid::Uuid::parse_str(&target_id).unwrap(),
                        snapshot,
                    ))
                },
            )
            .map_err(|e| StorageError::Internal(e.to_string()))?;

        let (source_id, target_id, snapshot) = entry;

        let snapshot_data: serde_json::Value = if let Some(snap) = &snapshot {
            serde_json::from_str(snap)
                .map_err(|e| StorageError::Internal(format!("Invalid snapshot: {}", e)))?
        } else {
            return Err(StorageError::Internal(
                "No snapshot available for undo".to_string(),
            ));
        };

        conn.execute_batch("BEGIN IMMEDIATE")
            .map_err(|e| StorageError::Internal(e.to_string()))?;

        let result = (|| -> Result<(), StorageError> {
            conn.execute(
                "DELETE FROM components WHERE entity_id = ?1",
                rusqlite::params![target_id.to_string()],
            )
            .map_err(|e| StorageError::Internal(e.to_string()))?;

            if let Some(target_comps) = snapshot_data["target"]["components"].as_array() {
                for comp in target_comps {
                    let comp_id = comp["id"].as_str().unwrap_or("");
                    let comp_type = comp["component_type"].as_str().unwrap_or("");
                    let data = comp["data"].to_string();
                    let created_at = comp["created_at"].as_str().unwrap_or("");
                    let version = comp["version"].as_i64().unwrap_or(1);

                    conn.execute(
                        "INSERT OR REPLACE INTO components (id, entity_id, component_type, data, created_at, version) VALUES (?1, ?2, ?3, ?4, ?5, ?6)",
                        rusqlite::params![comp_id, target_id.to_string(), comp_type, data, created_at, version],
                    ).map_err(|e| StorageError::Internal(e.to_string()))?;
                }
            }

            conn.execute(
                "DELETE FROM relationships WHERE source_id = ?1 OR target_id = ?1",
                rusqlite::params![target_id.to_string()],
            )
            .map_err(|e| StorageError::Internal(e.to_string()))?;

            if let Some(target_rels) = snapshot_data["target"]["relationships"].as_array() {
                for rel in target_rels {
                    let rel_id = rel["id"].as_str().unwrap_or("");
                    let target_ref = rel["target_id"].as_str().unwrap_or("");
                    let rel_type = rel["relationship_type"].as_str().unwrap_or("");
                    let is_active = rel["is_active"].as_bool().unwrap_or(true);
                    let created_at = rel["created_at"].as_str().unwrap_or("");

                    conn.execute(
                        "INSERT OR REPLACE INTO relationships (id, source_id, target_id, relationship_type, is_active, created_at) VALUES (?1, ?2, ?3, ?4, ?5, ?6)",
                        rusqlite::params![rel_id, target_id.to_string(), target_ref, rel_type, is_active as i32, created_at],
                    ).map_err(|e| StorageError::Internal(e.to_string()))?;
                }
            }

            let source = &snapshot_data["source"];
            let entity_type = source["entity_type"].as_str().unwrap_or("Article");
            let is_active = source["is_active"].as_bool().unwrap_or(true);
            let created_at = source["created_at"]
                .as_str()
                .map(|s| s.to_string())
                .unwrap_or_else(|| chrono::Utc::now().to_rfc3339());
            let updated_at = source["updated_at"]
                .as_str()
                .map(|s| s.to_string())
                .unwrap_or_else(|| chrono::Utc::now().to_rfc3339());
            let version = source["version"].as_i64().unwrap_or(1);

            conn.execute(
                "INSERT OR REPLACE INTO entities (id, entity_type, is_active, created_at, updated_at, version) VALUES (?1, ?2, ?3, ?4, ?5, ?6)",
                rusqlite::params![source_id.to_string(), entity_type, is_active as i32, created_at, updated_at, version],
            ).map_err(|e| StorageError::Internal(e.to_string()))?;

            if let Some(source_comps) = source["components"].as_array() {
                for comp in source_comps {
                    let comp_id = comp["id"].as_str().unwrap_or("");
                    let comp_type = comp["component_type"].as_str().unwrap_or("");
                    let data = comp["data"].to_string();
                    let created_at = comp["created_at"].as_str().unwrap_or("");
                    let version = comp["version"].as_i64().unwrap_or(1);

                    conn.execute(
                        "INSERT OR REPLACE INTO components (id, entity_id, component_type, data, created_at, version) VALUES (?1, ?2, ?3, ?4, ?5, ?6)",
                        rusqlite::params![comp_id, source_id.to_string(), comp_type, data, created_at, version],
                    ).map_err(|e| StorageError::Internal(e.to_string()))?;
                }
            }

            conn.execute(
                "DELETE FROM resolution_log WHERE id = ?1",
                rusqlite::params![merge_id.to_string()],
            )
            .map_err(|e| StorageError::Internal(e.to_string()))?;

            Ok(())
        })();

        match result {
            Ok(()) => {
                conn.execute_batch("COMMIT")
                    .map_err(|e| StorageError::Internal(e.to_string()))?;
                Ok(())
            }
            Err(e) => {
                conn.execute_batch("ROLLBACK")
                    .map_err(|_| StorageError::Internal("rollback failed".to_string()))?;
                Err(e)
            }
        }
    }

    async fn get_merge_history(
        &self,
        entity_id: Uuid,
    ) -> Result<Vec<MergeAuditEntry>, StorageError> {
        let conn = self
            .conn
            .lock()
            .map_err(|e| StorageError::Internal(e.to_string()))?;

        let mut stmt = conn
            .prepare(
                "SELECT id, source_id, source_title, target_id, target_title, strategy, confidence, timestamp, reason, snapshot
                 FROM resolution_log WHERE source_id = ?1 OR target_id = ?1 ORDER BY timestamp DESC"
            )
            .map_err(|e| StorageError::Internal(e.to_string()))?;

        let entries = stmt
            .query_map(rusqlite::params![entity_id.to_string()], |row| {
                Ok(MergeAuditEntry {
                    id: uuid::Uuid::parse_str(&row.get::<_, String>(0)?).unwrap(),
                    source_id: uuid::Uuid::parse_str(&row.get::<_, String>(1)?).unwrap(),
                    source_title: row.get(2)?,
                    target_id: uuid::Uuid::parse_str(&row.get::<_, String>(3)?).unwrap(),
                    target_title: row.get(4)?,
                    strategy: row.get(5)?,
                    confidence: row.get(6)?,
                    timestamp: chrono::DateTime::parse_from_rfc3339(&row.get::<_, String>(7)?)
                        .unwrap()
                        .with_timezone(&chrono::Utc),
                    reason: row.get(8)?,
                    snapshot: row.get(9)?,
                })
            })
            .map_err(|e| StorageError::Internal(e.to_string()))?;

        let entries: Vec<MergeAuditEntry> = entries.filter_map(|r| r.ok()).collect();

        Ok(entries)
    }

    async fn get_all_merge_history(&self) -> Result<Vec<MergeAuditEntry>, StorageError> {
        let conn = self
            .conn
            .lock()
            .map_err(|e| StorageError::Internal(e.to_string()))?;

        let mut stmt = conn
            .prepare(
                "SELECT id, source_id, source_title, target_id, target_title, strategy, confidence, timestamp, reason, snapshot
                 FROM resolution_log ORDER BY timestamp DESC"
            )
            .map_err(|e| StorageError::Internal(e.to_string()))?;

        let entries = stmt
            .query_map([], |row| {
                Ok(MergeAuditEntry {
                    id: uuid::Uuid::parse_str(&row.get::<_, String>(0)?).unwrap(),
                    source_id: uuid::Uuid::parse_str(&row.get::<_, String>(1)?).unwrap(),
                    source_title: row.get(2)?,
                    target_id: uuid::Uuid::parse_str(&row.get::<_, String>(3)?).unwrap(),
                    target_title: row.get(4)?,
                    strategy: row.get(5)?,
                    confidence: row.get(6)?,
                    timestamp: chrono::DateTime::parse_from_rfc3339(&row.get::<_, String>(7)?)
                        .unwrap()
                        .with_timezone(&chrono::Utc),
                    reason: row.get(8)?,
                    snapshot: row.get(9)?,
                })
            })
            .map_err(|e| StorageError::Internal(e.to_string()))?;

        let entries: Vec<MergeAuditEntry> = entries.filter_map(|r| r.ok()).collect();

        Ok(entries)
    }
}
