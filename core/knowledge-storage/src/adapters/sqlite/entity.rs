use async_trait::async_trait;
use knowledge_core::features::component::ComponentType;
use knowledge_core::features::entity::Entity;
use knowledge_core::ports::{EntityRepository, EntityVersion, StorageError};
use rusqlite::OptionalExtension;
use uuid::Uuid;

use super::store::{SqliteStore, ENTITY_COLS};

#[async_trait]
impl EntityRepository for SqliteStore {
    async fn get(&self, id: Uuid) -> Result<Option<Entity>, StorageError> {
        let conn = self
            .conn
            .lock()
            .map_err(|e| StorageError::Internal(e.to_string()))?;
        let mut stmt = conn
            .prepare(&format!(
                "SELECT {} FROM entities WHERE id = ?1",
                ENTITY_COLS
            ))
            .map_err(|e| StorageError::Internal(e.to_string()))?;
        let result = stmt
            .query_row(rusqlite::params![id.to_string()], Self::parse_entity)
            .optional()
            .map_err(|e| StorageError::Internal(e.to_string()))?;
        Ok(result)
    }

    async fn save(&self, entity: &Entity) -> Result<(), StorageError> {
        let conn = self
            .conn
            .lock()
            .map_err(|e| StorageError::Internal(e.to_string()))?;
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
        Ok(())
    }

    async fn delete(&self, id: Uuid) -> Result<(), StorageError> {
        let conn = self
            .conn
            .lock()
            .map_err(|e| StorageError::Internal(e.to_string()))?;
        conn.execute(
            "DELETE FROM components WHERE entity_id = ?1",
            rusqlite::params![id.to_string()],
        )
        .map_err(|e| StorageError::Internal(e.to_string()))?;
        conn.execute(
            "DELETE FROM relationships WHERE source_id = ?1 OR target_id = ?1",
            rusqlite::params![id.to_string()],
        )
        .map_err(|e| StorageError::Internal(e.to_string()))?;
        conn.execute(
            "DELETE FROM entities WHERE id = ?1",
            rusqlite::params![id.to_string()],
        )
        .map_err(|e| StorageError::Internal(e.to_string()))?;
        conn.execute(
            "DELETE FROM entities_fts WHERE entity_id = ?1",
            rusqlite::params![id.to_string()],
        )
        .map_err(|e| StorageError::Internal(e.to_string()))?;
        Ok(())
    }

    async fn list(&self) -> Result<Vec<Entity>, StorageError> {
        let conn = self
            .conn
            .lock()
            .map_err(|e| StorageError::Internal(e.to_string()))?;
        let mut stmt = conn
            .prepare(&format!(
                "SELECT {} FROM entities WHERE is_active = 1",
                ENTITY_COLS
            ))
            .map_err(|e| StorageError::Internal(e.to_string()))?;
        let rows = stmt
            .query_map([], Self::parse_entity)
            .map_err(|e| StorageError::Internal(e.to_string()))?;
        rows.map(|r| r.map_err(|e| StorageError::Internal(e.to_string())))
            .collect()
    }

    async fn find_by_type(&self, entity_type: &str) -> Result<Vec<Entity>, StorageError> {
        let conn = self
            .conn
            .lock()
            .map_err(|e| StorageError::Internal(e.to_string()))?;
        let quoted_type = serde_json::to_string(&entity_type).unwrap();
        let mut stmt = conn
            .prepare(&format!(
                "SELECT {} FROM entities WHERE entity_type = ?1 AND is_active = 1",
                ENTITY_COLS
            ))
            .map_err(|e| StorageError::Internal(e.to_string()))?;
        let rows = stmt
            .query_map(rusqlite::params![quoted_type], Self::parse_entity)
            .map_err(|e| StorageError::Internal(e.to_string()))?;
        rows.map(|r| r.map_err(|e| StorageError::Internal(e.to_string())))
            .collect()
    }

    async fn find_by_title(&self, title: &str) -> Result<Vec<Entity>, StorageError> {
        let conn = self
            .conn
            .lock()
            .map_err(|e| StorageError::Internal(e.to_string()))?;
        let title_json = serde_json::to_string(&title).unwrap();
        let mut stmt = conn
            .prepare(&format!(
                "SELECT {cols} FROM entities e
                 JOIN components c ON c.entity_id = e.id
                 WHERE c.component_type = ?1 AND c.data = ?2 AND e.is_active = 1",
                cols = ENTITY_COLS
                    .replacen("id,", "e.id,", 1)
                    .replacen("entity_type,", "e.entity_type,", 1)
                    .replacen("is_active,", "e.is_active,", 1)
                    .replacen("created_at,", "e.created_at,", 1)
                    .replacen("updated_at,", "e.updated_at,", 1)
                    .replacen("version", "e.version", 1)
            ))
            .map_err(|e| StorageError::Internal(e.to_string()))?;
        let rows = stmt
            .query_map(
                rusqlite::params![Self::component_type_str(&ComponentType::Title), title_json],
                Self::parse_entity,
            )
            .map_err(|e| StorageError::Internal(e.to_string()))?;
        rows.map(|r| r.map_err(|e| StorageError::Internal(e.to_string())))
            .collect()
    }

    async fn increment_version(&self, id: Uuid) -> Result<(), StorageError> {
        let conn = self
            .conn
            .lock()
            .map_err(|e| StorageError::Internal(e.to_string()))?;
        let mut stmt = conn
            .prepare(&format!(
                "SELECT {} FROM entities WHERE id = ?1",
                ENTITY_COLS
            ))
            .map_err(|e| StorageError::Internal(e.to_string()))?;
        let entity = stmt
            .query_row(rusqlite::params![id.to_string()], Self::parse_entity)
            .map_err(|e| StorageError::Internal(e.to_string()))?;

        let snapshot = serde_json::json!({
            "entity_type": serde_json::to_string(&entity.entity_type).unwrap(),
            "is_active": entity.is_active,
            "version": entity.version,
            "updated_at": entity.updated_at.to_rfc3339(),
        });

        conn.execute(
            "INSERT INTO entity_versions (entity_id, version, snapshot, created_at) VALUES (?1, ?2, ?3, ?4)",
            rusqlite::params![id.to_string(), entity.version, snapshot.to_string(), chrono::Utc::now().to_rfc3339()],
        ).map_err(|e| StorageError::Internal(e.to_string()))?;

        conn.execute(
            "UPDATE entities SET version = version + 1, updated_at = ?1 WHERE id = ?2",
            rusqlite::params![chrono::Utc::now().to_rfc3339(), id.to_string()],
        )
        .map_err(|e| StorageError::Internal(e.to_string()))?;
        Ok(())
    }

    async fn find_by_component_type(
        &self,
        component_type: &str,
    ) -> Result<Vec<Entity>, StorageError> {
        let conn = self
            .conn
            .lock()
            .map_err(|e| StorageError::Internal(e.to_string()))?;
        let quoted_type = serde_json::to_string(&component_type).unwrap();
        let mut stmt = conn
            .prepare(&format!(
                "SELECT DISTINCT {cols} FROM entities e
                 JOIN components c ON c.entity_id = e.id
                 WHERE c.component_type = ?1 AND e.is_active = 1",
                cols = ENTITY_COLS
                    .replacen("id,", "e.id,", 1)
                    .replacen("entity_type,", "e.entity_type,", 1)
                    .replacen("is_active,", "e.is_active,", 1)
                    .replacen("created_at,", "e.created_at,", 1)
                    .replacen("updated_at,", "e.updated_at,", 1)
                    .replacen("version", "e.version", 1)
            ))
            .map_err(|e| StorageError::Internal(e.to_string()))?;
        let rows = stmt
            .query_map(rusqlite::params![quoted_type], Self::parse_entity)
            .map_err(|e| StorageError::Internal(e.to_string()))?;
        rows.map(|r| r.map_err(|e| StorageError::Internal(e.to_string())))
            .collect()
    }

    async fn find_by_tag(&self, tag: &str) -> Result<Vec<Entity>, StorageError> {
        let conn = self
            .conn
            .lock()
            .map_err(|e| StorageError::Internal(e.to_string()))?;
        let tag_json = serde_json::to_string(&tag).unwrap();
        let like_pattern = format!("%{}%", tag_json);
        let mut stmt = conn
            .prepare(&format!(
                "SELECT DISTINCT {cols} FROM entities e
                 JOIN components c ON c.entity_id = e.id
                 WHERE c.component_type = ?1 AND c.data LIKE ?2 AND e.is_active = 1",
                cols = ENTITY_COLS
                    .replacen("id,", "e.id,", 1)
                    .replacen("entity_type,", "e.entity_type,", 1)
                    .replacen("is_active,", "e.is_active,", 1)
                    .replacen("created_at,", "e.created_at,", 1)
                    .replacen("updated_at,", "e.updated_at,", 1)
                    .replacen("version", "e.version", 1)
            ))
            .map_err(|e| StorageError::Internal(e.to_string()))?;
        let rows = stmt
            .query_map(
                rusqlite::params![Self::component_type_str(&ComponentType::Tags), like_pattern],
                Self::parse_entity,
            )
            .map_err(|e| StorageError::Internal(e.to_string()))?;
        rows.map(|r| r.map_err(|e| StorageError::Internal(e.to_string())))
            .collect()
    }

    async fn get_version_history(
        &self,
        entity_id: Uuid,
    ) -> Result<Vec<EntityVersion>, StorageError> {
        let conn = self
            .conn
            .lock()
            .map_err(|e| StorageError::Internal(e.to_string()))?;
        let mut stmt = conn
            .prepare("SELECT entity_id, version, snapshot, created_at FROM entity_versions WHERE entity_id = ?1 ORDER BY version DESC")
            .map_err(|e| StorageError::Internal(e.to_string()))?;
        let rows = stmt
            .query_map(rusqlite::params![entity_id.to_string()], |row| {
                Ok(EntityVersion {
                    entity_id: uuid::Uuid::parse_str(&row.get::<_, String>(0)?).unwrap(),
                    version: row.get(1)?,
                    snapshot: serde_json::from_str(&row.get::<_, String>(2)?).unwrap(),
                    created_at: chrono::DateTime::parse_from_rfc3339(&row.get::<_, String>(3)?)
                        .unwrap()
                        .with_timezone(&chrono::Utc),
                })
            })
            .map_err(|e| StorageError::Internal(e.to_string()))?;
        rows.map(|r| r.map_err(|e| StorageError::Internal(e.to_string())))
            .collect()
    }
}
