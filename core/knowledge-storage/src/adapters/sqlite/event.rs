use async_trait::async_trait;
use knowledge_core::ports::{Event, EventLog, StorageError};
use uuid::Uuid;

use super::store::SqliteStore;

#[async_trait]
impl EventLog for SqliteStore {
    async fn append(&self, event: &Event) -> Result<(), StorageError> {
        let conn = self
            .conn
            .lock()
            .map_err(|e| StorageError::Internal(e.to_string()))?;
        conn.execute(
            "INSERT INTO events (id, event_type, entity_id, timestamp, data) VALUES (?1, ?2, ?3, ?4, ?5)",
            rusqlite::params![
                event.id.to_string(),
                serde_json::to_string(&event.event_type).unwrap(),
                event.entity_id.to_string(),
                event.timestamp.to_rfc3339(),
                event.data.to_string(),
            ],
        ).map_err(|e| StorageError::Internal(e.to_string()))?;
        Ok(())
    }

    async fn list_by_entity(&self, entity_id: Uuid) -> Result<Vec<Event>, StorageError> {
        let conn = self
            .conn
            .lock()
            .map_err(|e| StorageError::Internal(e.to_string()))?;
        let mut stmt = conn
            .prepare("SELECT id, event_type, entity_id, timestamp, data FROM events WHERE entity_id = ?1 ORDER BY timestamp DESC")
            .map_err(|e| StorageError::Internal(e.to_string()))?;
        let rows = stmt
            .query_map(rusqlite::params![entity_id.to_string()], Self::parse_event)
            .map_err(|e| StorageError::Internal(e.to_string()))?;
        rows.map(|r| r.map_err(|e| StorageError::Internal(e.to_string())))
            .collect()
    }
}
