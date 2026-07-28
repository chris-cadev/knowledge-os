use async_trait::async_trait;
use knowledge_core::features::component::Component;
use knowledge_core::ports::{ComponentRepository, StorageError};
use uuid::Uuid;

use super::store::SqliteStore;

#[async_trait]
impl ComponentRepository for SqliteStore {
    async fn get(&self, entity_id: Uuid) -> Result<Vec<Component>, StorageError> {
        let conn = self
            .conn
            .lock()
            .map_err(|e| StorageError::Internal(e.to_string()))?;
        let mut stmt = conn
            .prepare("SELECT id, entity_id, component_type, data, created_at, version FROM components WHERE entity_id = ?1")
            .map_err(|e| StorageError::Internal(e.to_string()))?;
        let rows = stmt
            .query_map(
                rusqlite::params![entity_id.to_string()],
                Self::parse_component,
            )
            .map_err(|e| StorageError::Internal(e.to_string()))?;
        rows.map(|r| r.map_err(|e| StorageError::Internal(e.to_string())))
            .collect()
    }

    async fn save(&self, component: &Component) -> Result<(), StorageError> {
        let conn = self
            .conn
            .lock()
            .map_err(|e| StorageError::Internal(e.to_string()))?;
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
        Ok(())
    }

    async fn delete(&self, id: Uuid) -> Result<(), StorageError> {
        let conn = self
            .conn
            .lock()
            .map_err(|e| StorageError::Internal(e.to_string()))?;
        conn.execute(
            "DELETE FROM components WHERE id = ?1",
            rusqlite::params![id.to_string()],
        )
        .map_err(|e| StorageError::Internal(e.to_string()))?;
        Ok(())
    }

    async fn find_by_type(
        &self,
        entity_id: Uuid,
        component_type: &str,
    ) -> Result<Vec<Component>, StorageError> {
        let conn = self
            .conn
            .lock()
            .map_err(|e| StorageError::Internal(e.to_string()))?;
        let quoted_type = serde_json::to_string(&component_type).unwrap();
        let mut stmt = conn
            .prepare("SELECT id, entity_id, component_type, data, created_at, version FROM components WHERE entity_id = ?1 AND component_type = ?2")
            .map_err(|e| StorageError::Internal(e.to_string()))?;
        let rows = stmt
            .query_map(
                rusqlite::params![entity_id.to_string(), quoted_type],
                Self::parse_component,
            )
            .map_err(|e| StorageError::Internal(e.to_string()))?;
        rows.map(|r| r.map_err(|e| StorageError::Internal(e.to_string())))
            .collect()
    }

    async fn update_data(&self, id: Uuid, data: serde_json::Value) -> Result<(), StorageError> {
        let conn = self
            .conn
            .lock()
            .map_err(|e| StorageError::Internal(e.to_string()))?;
        let data_str = data.to_string();
        conn.execute(
            "UPDATE components SET data = ?1, version = version + 1 WHERE id = ?2",
            rusqlite::params![data_str, id.to_string()],
        )
        .map_err(|e| StorageError::Internal(e.to_string()))?;
        Ok(())
    }

    async fn find_by_component_data(
        &self,
        component_type: &str,
        _json_path: &str,
        value: &str,
    ) -> Result<Vec<Component>, StorageError> {
        let conn = self
            .conn
            .lock()
            .map_err(|e| StorageError::Internal(e.to_string()))?;
        let quoted_type = serde_json::to_string(&component_type).unwrap();
        let like_pattern = format!("%{}%", value);
        let mut stmt = conn
            .prepare("SELECT id, entity_id, component_type, data, created_at, version FROM components WHERE component_type = ?1 AND data LIKE ?2")
            .map_err(|e| StorageError::Internal(e.to_string()))?;
        let rows = stmt
            .query_map(
                rusqlite::params![quoted_type, like_pattern],
                Self::parse_component,
            )
            .map_err(|e| StorageError::Internal(e.to_string()))?;
        rows.map(|r| r.map_err(|e| StorageError::Internal(e.to_string())))
            .collect()
    }

    async fn delete_by_entity(&self, entity_id: Uuid) -> Result<(), StorageError> {
        let conn = self
            .conn
            .lock()
            .map_err(|e| StorageError::Internal(e.to_string()))?;
        conn.execute(
            "DELETE FROM components WHERE entity_id = ?1",
            rusqlite::params![entity_id.to_string()],
        )
        .map_err(|e| StorageError::Internal(e.to_string()))?;
        Ok(())
    }
}
