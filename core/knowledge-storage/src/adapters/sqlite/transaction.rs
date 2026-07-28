use async_trait::async_trait;
use knowledge_core::features::component::Component;
use knowledge_core::features::entity::Entity;
use knowledge_core::ports::{Event, StorageError, TransactionalWrite};

use super::store::SqliteStore;

#[async_trait]
impl TransactionalWrite for SqliteStore {
    async fn save_entity_with_components(
        &self,
        entity: &Entity,
        components: &[Component],
        event: &Event,
    ) -> Result<(), StorageError> {
        let conn = self
            .conn
            .lock()
            .map_err(|e| StorageError::Internal(e.to_string()))?;

        conn.execute_batch("BEGIN IMMEDIATE")
            .map_err(|e| StorageError::Internal(e.to_string()))?;

        let result = (|| -> Result<(), StorageError> {
            conn.execute(
                "INSERT OR REPLACE INTO entities (id, entity_type, is_active, created_at, updated_at, version) VALUES (?1, ?2, ?3, ?4, ?5, ?6)",
                rusqlite::params![
                    entity.id.to_string(),
                    Self::entity_type_str(entity),
                    entity.is_active as i32,
                    entity.created_at.to_rfc3339(),
                    entity.updated_at.to_rfc3339(),
                    entity.version,
                ],
            ).map_err(|e| StorageError::Internal(e.to_string()))?;

            for component in components {
                conn.execute(
                    "INSERT OR REPLACE INTO components (id, entity_id, component_type, data, created_at, version) VALUES (?1, ?2, ?3, ?4, ?5, ?6)",
                    rusqlite::params![
                        component.id.to_string(),
                        component.entity_id.to_string(),
                        Self::component_type_str(&component.component_type),
                        component.data.to_string(),
                        component.created_at.to_rfc3339(),
                        component.version,
                    ],
                ).map_err(|e| StorageError::Internal(e.to_string()))?;
            }

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

    async fn update_entity_with_components(
        &self,
        entity: &Entity,
        components: &[Component],
        event: &Event,
    ) -> Result<(), StorageError> {
        let conn = self
            .conn
            .lock()
            .map_err(|e| StorageError::Internal(e.to_string()))?;

        conn.execute_batch("BEGIN IMMEDIATE")
            .map_err(|e| StorageError::Internal(e.to_string()))?;

        let result = (|| -> Result<(), StorageError> {
            conn.execute(
                "INSERT OR REPLACE INTO entities (id, entity_type, is_active, created_at, updated_at, version) VALUES (?1, ?2, ?3, ?4, ?5, ?6)",
                rusqlite::params![
                    entity.id.to_string(),
                    Self::entity_type_str(entity),
                    entity.is_active as i32,
                    entity.created_at.to_rfc3339(),
                    entity.updated_at.to_rfc3339(),
                    entity.version,
                ],
            ).map_err(|e| StorageError::Internal(e.to_string()))?;

            conn.execute(
                "DELETE FROM components WHERE entity_id = ?1",
                rusqlite::params![entity.id.to_string()],
            )
            .map_err(|e| StorageError::Internal(e.to_string()))?;

            for component in components {
                conn.execute(
                    "INSERT OR REPLACE INTO components (id, entity_id, component_type, data, created_at, version) VALUES (?1, ?2, ?3, ?4, ?5, ?6)",
                    rusqlite::params![
                        component.id.to_string(),
                        component.entity_id.to_string(),
                        Self::component_type_str(&component.component_type),
                        component.data.to_string(),
                        component.created_at.to_rfc3339(),
                        component.version,
                    ],
                ).map_err(|e| StorageError::Internal(e.to_string()))?;
            }

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
}
