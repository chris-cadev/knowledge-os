use async_trait::async_trait;
use knowledge_core::features::relationship::Relationship;
use knowledge_core::ports::{RelationshipRepository, StorageError};
use rusqlite::OptionalExtension;
use uuid::Uuid;

use super::store::SqliteStore;

#[async_trait]
impl RelationshipRepository for SqliteStore {
    async fn get(&self, id: Uuid) -> Result<Option<Relationship>, StorageError> {
        let conn = self
            .conn
            .lock()
            .map_err(|e| StorageError::Internal(e.to_string()))?;
        let mut stmt = conn
            .prepare("SELECT id, source_id, target_id, relationship_type, is_active, created_at FROM relationships WHERE id = ?1")
            .map_err(|e| StorageError::Internal(e.to_string()))?;
        stmt.query_row(rusqlite::params![id.to_string()], Self::parse_relationship)
            .optional()
            .map_err(|e| StorageError::Internal(e.to_string()))
    }

    async fn save(&self, relationship: &Relationship) -> Result<(), StorageError> {
        let conn = self
            .conn
            .lock()
            .map_err(|e| StorageError::Internal(e.to_string()))?;
        conn.execute(
            "INSERT OR REPLACE INTO relationships (id, source_id, target_id, relationship_type, is_active, created_at) VALUES (?1, ?2, ?3, ?4, ?5, ?6)",
            rusqlite::params![
                relationship.id.to_string(),
                relationship.source_id.to_string(),
                relationship.target_id.to_string(),
                serde_json::to_string(&relationship.relationship_type).unwrap(),
                relationship.is_active as i32,
                relationship.created_at.to_rfc3339(),
            ],
        ).map_err(|e| StorageError::Internal(e.to_string()))?;
        Ok(())
    }

    async fn update(&self, relationship: &Relationship) -> Result<(), StorageError> {
        RelationshipRepository::save(self, relationship).await
    }

    async fn delete(&self, id: Uuid) -> Result<(), StorageError> {
        let conn = self
            .conn
            .lock()
            .map_err(|e| StorageError::Internal(e.to_string()))?;
        conn.execute(
            "DELETE FROM relationships WHERE id = ?1",
            rusqlite::params![id.to_string()],
        )
        .map_err(|e| StorageError::Internal(e.to_string()))?;
        Ok(())
    }

    async fn by_source(&self, source_id: Uuid) -> Result<Vec<Relationship>, StorageError> {
        let conn = self
            .conn
            .lock()
            .map_err(|e| StorageError::Internal(e.to_string()))?;
        let mut stmt = conn
            .prepare("SELECT id, source_id, target_id, relationship_type, is_active, created_at FROM relationships WHERE source_id = ?1 AND is_active = 1")
            .map_err(|e| StorageError::Internal(e.to_string()))?;
        let rows = stmt
            .query_map(
                rusqlite::params![source_id.to_string()],
                Self::parse_relationship,
            )
            .map_err(|e| StorageError::Internal(e.to_string()))?;
        rows.map(|r| r.map_err(|e| StorageError::Internal(e.to_string())))
            .collect()
    }

    async fn by_target(&self, target_id: Uuid) -> Result<Vec<Relationship>, StorageError> {
        let conn = self
            .conn
            .lock()
            .map_err(|e| StorageError::Internal(e.to_string()))?;
        let mut stmt = conn
            .prepare("SELECT id, source_id, target_id, relationship_type, is_active, created_at FROM relationships WHERE target_id = ?1 AND is_active = 1")
            .map_err(|e| StorageError::Internal(e.to_string()))?;
        let rows = stmt
            .query_map(
                rusqlite::params![target_id.to_string()],
                Self::parse_relationship,
            )
            .map_err(|e| StorageError::Internal(e.to_string()))?;
        rows.map(|r| r.map_err(|e| StorageError::Internal(e.to_string())))
            .collect()
    }

    async fn find_by_source_and_target(
        &self,
        source_id: Uuid,
        target_id: Uuid,
    ) -> Result<Option<Relationship>, StorageError> {
        let conn = self
            .conn
            .lock()
            .map_err(|e| StorageError::Internal(e.to_string()))?;
        let mut stmt = conn
            .prepare("SELECT id, source_id, target_id, relationship_type, is_active, created_at FROM relationships WHERE source_id = ?1 AND target_id = ?2 AND is_active = 1")
            .map_err(|e| StorageError::Internal(e.to_string()))?;
        stmt.query_row(
            rusqlite::params![source_id.to_string(), target_id.to_string()],
            Self::parse_relationship,
        )
        .optional()
        .map_err(|e| StorageError::Internal(e.to_string()))
    }

    async fn find_by_type(
        &self,
        relationship_type: &str,
    ) -> Result<Vec<Relationship>, StorageError> {
        let conn = self
            .conn
            .lock()
            .map_err(|e| StorageError::Internal(e.to_string()))?;
        let quoted_type = serde_json::to_string(&relationship_type).unwrap();
        let mut stmt = conn
            .prepare("SELECT id, source_id, target_id, relationship_type, is_active, created_at FROM relationships WHERE relationship_type = ?1 AND is_active = 1")
            .map_err(|e| StorageError::Internal(e.to_string()))?;
        let rows = stmt
            .query_map(rusqlite::params![quoted_type], Self::parse_relationship)
            .map_err(|e| StorageError::Internal(e.to_string()))?;
        rows.map(|r| r.map_err(|e| StorageError::Internal(e.to_string())))
            .collect()
    }
}
