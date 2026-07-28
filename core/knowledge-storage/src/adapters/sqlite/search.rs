use async_trait::async_trait;
use knowledge_core::features::component::{Component, ComponentType};
use knowledge_core::features::entity::Entity;
use knowledge_core::ports::{SearchIndex, SearchQuery, SearchResult, StorageError};
use uuid::Uuid;

use super::store::{SqliteStore, ENTITY_COLS};

#[async_trait]
impl SearchIndex for SqliteStore {
    async fn index_entity(
        &self,
        entity: &Entity,
        components: &[Component],
    ) -> Result<(), StorageError> {
        let conn = self
            .conn
            .lock()
            .map_err(|e| StorageError::Internal(e.to_string()))?;

        let title = components
            .iter()
            .find(|c| c.component_type == ComponentType::Title)
            .and_then(|c| c.data.as_str().map(String::from))
            .unwrap_or_default();

        let content = components
            .iter()
            .find(|c| c.component_type == ComponentType::Content)
            .and_then(|c| c.data.as_str().map(String::from))
            .unwrap_or_default();

        let tags = components
            .iter()
            .find(|c| c.component_type == ComponentType::Tags)
            .and_then(|c| {
                c.data.as_array().map(|a| {
                    a.iter()
                        .filter_map(|v| v.as_str())
                        .collect::<Vec<_>>()
                        .join(", ")
                })
            })
            .unwrap_or_default();

        conn.execute(
            "DELETE FROM entities_fts WHERE entity_id = ?1",
            rusqlite::params![entity.id.to_string()],
        )
        .map_err(|e| StorageError::Internal(e.to_string()))?;
        conn.execute(
            "INSERT INTO entities_fts (entity_id, title, content, tags) VALUES (?1, ?2, ?3, ?4)",
            rusqlite::params![entity.id.to_string(), title, content, tags],
        )
        .map_err(|e| StorageError::Internal(e.to_string()))?;
        Ok(())
    }

    async fn remove_entity(&self, entity_id: Uuid) -> Result<(), StorageError> {
        let conn = self
            .conn
            .lock()
            .map_err(|e| StorageError::Internal(e.to_string()))?;
        conn.execute(
            "DELETE FROM entities_fts WHERE entity_id = ?1",
            rusqlite::params![entity_id.to_string()],
        )
        .map_err(|e| StorageError::Internal(e.to_string()))?;
        Ok(())
    }

    async fn search(&self, query: &SearchQuery) -> Result<Vec<SearchResult>, StorageError> {
        let conn = self
            .conn
            .lock()
            .map_err(|e| StorageError::Internal(e.to_string()))?;

        let fts_query = format!("{} OR {}", query.query, query.query);

        let raw_results: Vec<(String, f64, String)> = {
            let mut stmt = conn.prepare(
                "SELECT entity_id, bm25(entities_fts) as rank, snippet(entities_fts, 2, '<b>', '</b>', '...', 32) as snip FROM entities_fts WHERE entities_fts MATCH ?1 ORDER BY rank"
            ).map_err(|e| StorageError::Internal(e.to_string()))?;
            let results: Vec<_> = stmt
                .query_map(rusqlite::params![fts_query], |row| {
                    Ok((
                        row.get::<_, String>(0)?,
                        row.get::<_, f64>(1)?,
                        row.get::<_, String>(2)?,
                    ))
                })
                .map_err(|e| StorageError::Internal(e.to_string()))?
                .filter_map(|r| r.ok())
                .collect();
            results
        };

        let mut results = Vec::new();
        for (id_str, score, snippet) in raw_results {
            if let Ok(id) = Uuid::parse_str(&id_str) {
                let mut pass = true;

                if let Some(ref t) = query.entity_type {
                    let mut estmt = conn
                        .prepare(&format!(
                            "SELECT {} FROM entities WHERE id = ?1 AND is_active = 1",
                            ENTITY_COLS
                        ))
                        .map_err(|e| StorageError::Internal(e.to_string()))?;
                    match estmt.query_row(rusqlite::params![id_str], Self::parse_entity) {
                        Ok(entity) => {
                            let quoted = serde_json::to_value(t).unwrap();
                            let stored = serde_json::to_value(&entity.entity_type).unwrap();
                            if quoted != stored {
                                pass = false;
                            }
                        }
                        Err(_) => pass = false,
                    }
                }

                if pass {
                    if let Some(ref tag_val) = query.tag {
                        let mut tag_stmt = conn
                            .prepare("SELECT tags FROM entities_fts WHERE entity_id = ?1")
                            .map_err(|e| StorageError::Internal(e.to_string()))?;
                        if let Ok(ftags) = tag_stmt
                            .query_row(rusqlite::params![id_str], |row| row.get::<_, String>(0))
                        {
                            let tags_list: Vec<&str> = ftags.split(", ").collect();
                            if !tags_list.contains(&tag_val.as_str()) {
                                pass = false;
                            }
                        } else {
                            pass = false;
                        }
                    }
                }

                if pass {
                    let snip = if snippet.is_empty() {
                        None
                    } else {
                        Some(snippet)
                    };
                    results.push(SearchResult {
                        entity_id: id,
                        score,
                        confidence: None,
                        snippet: snip,
                    });
                }
            }
        }

        Ok(results)
    }

    async fn rebuild(&self, entities: &[(Entity, Vec<Component>)]) -> Result<(), StorageError> {
        let conn = self
            .conn
            .lock()
            .map_err(|e| StorageError::Internal(e.to_string()))?;

        conn.execute_batch(
            "DROP TABLE IF EXISTS entities_fts;
             CREATE VIRTUAL TABLE entities_fts USING fts5(
                 entity_id UNINDEXED,
                 title,
                 content,
                 tags
             );",
        )
        .map_err(|e| StorageError::Internal(e.to_string()))?;

        for (entity, components) in entities {
            let title = components
                .iter()
                .find(|c| c.component_type == ComponentType::Title)
                .and_then(|c| c.data.as_str().map(String::from))
                .unwrap_or_default();

            let content = components
                .iter()
                .find(|c| c.component_type == ComponentType::Content)
                .and_then(|c| c.data.as_str().map(String::from))
                .unwrap_or_default();

            let tags = components
                .iter()
                .find(|c| c.component_type == ComponentType::Tags)
                .and_then(|c| {
                    c.data.as_array().map(|a| {
                        a.iter()
                            .filter_map(|v| v.as_str())
                            .collect::<Vec<_>>()
                            .join(", ")
                    })
                })
                .unwrap_or_default();

            conn.execute(
                "INSERT INTO entities_fts (entity_id, title, content, tags) VALUES (?1, ?2, ?3, ?4)",
                rusqlite::params![entity.id.to_string(), title, content, tags],
            ).map_err(|e| StorageError::Internal(e.to_string()))?;
        }

        Ok(())
    }
}
