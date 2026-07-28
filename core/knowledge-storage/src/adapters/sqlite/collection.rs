use async_trait::async_trait;
use knowledge_core::ports::{Collection, CollectionRepository, StorageError};
use rusqlite::OptionalExtension;
use uuid::Uuid;

use super::store::{SqliteStore, ENTITY_COLS};

#[async_trait]
impl CollectionRepository for SqliteStore {
    async fn create(&self, collection: Collection) -> Result<Collection, StorageError> {
        let conn = self
            .conn
            .lock()
            .map_err(|e| StorageError::Internal(e.to_string()))?;
        conn.execute(
            "INSERT INTO collections (id, name, description, created_at, updated_at) VALUES (?1, ?2, ?3, ?4, ?5)",
            rusqlite::params![
                collection.id.to_string(),
                collection.name,
                collection.description,
                collection.created_at.to_rfc3339(),
                collection.updated_at.to_rfc3339(),
            ],
        )
        .map_err(|e| StorageError::Internal(e.to_string()))?;
        Ok(collection)
    }

    async fn get(&self, id: Uuid) -> Result<Option<Collection>, StorageError> {
        let conn = self
            .conn
            .lock()
            .map_err(|e| StorageError::Internal(e.to_string()))?;
        let mut stmt = conn
            .prepare("SELECT id, name, description, created_at, updated_at FROM collections WHERE id = ?1")
            .map_err(|e| StorageError::Internal(e.to_string()))?;
        stmt.query_row(rusqlite::params![id.to_string()], Self::parse_collection)
            .optional()
            .map_err(|e| StorageError::Internal(e.to_string()))
    }

    async fn update(&self, collection: Collection) -> Result<Collection, StorageError> {
        let conn = self
            .conn
            .lock()
            .map_err(|e| StorageError::Internal(e.to_string()))?;
        let affected = conn
            .execute(
                "UPDATE collections SET name = ?1, description = ?2, updated_at = ?3 WHERE id = ?4",
                rusqlite::params![
                    collection.name,
                    collection.description,
                    collection.updated_at.to_rfc3339(),
                    collection.id.to_string(),
                ],
            )
            .map_err(|e| StorageError::Internal(e.to_string()))?;
        if affected == 0 {
            return Err(StorageError::NotFound);
        }
        Ok(collection)
    }

    async fn delete(&self, id: Uuid) -> Result<(), StorageError> {
        let conn = self
            .conn
            .lock()
            .map_err(|e| StorageError::Internal(e.to_string()))?;
        conn.execute(
            "DELETE FROM collections WHERE id = ?1",
            rusqlite::params![id.to_string()],
        )
        .map_err(|e| StorageError::Internal(e.to_string()))?;
        Ok(())
    }

    async fn list(&self) -> Result<Vec<Collection>, StorageError> {
        let conn = self
            .conn
            .lock()
            .map_err(|e| StorageError::Internal(e.to_string()))?;
        let mut stmt = conn
            .prepare("SELECT id, name, description, created_at, updated_at FROM collections ORDER BY name")
            .map_err(|e| StorageError::Internal(e.to_string()))?;
        let rows = stmt
            .query_map([], Self::parse_collection)
            .map_err(|e| StorageError::Internal(e.to_string()))?;
        rows.map(|r| r.map_err(|e| StorageError::Internal(e.to_string())))
            .collect()
    }

    async fn add_member(&self, collection_id: Uuid, entity_id: Uuid) -> Result<(), StorageError> {
        let conn = self
            .conn
            .lock()
            .map_err(|e| StorageError::Internal(e.to_string()))?;
        let affected = conn
            .execute(
                "INSERT INTO collection_members (collection_id, entity_id, added_at) VALUES (?1, ?2, ?3)",
                rusqlite::params![
                    collection_id.to_string(),
                    entity_id.to_string(),
                    chrono::Utc::now().to_rfc3339(),
                ],
            )
            .map_err(|e| {
                if e.to_string().contains("UNIQUE") || e.to_string().contains("already") {
                    StorageError::Internal(format!(
                        "Entity {} is already a member of collection {}",
                        entity_id, collection_id
                    ))
                } else {
                    StorageError::Internal(e.to_string())
                }
            })?;
        if affected == 0 {
            return Err(StorageError::Internal(format!(
                "Entity {} is already a member of collection {}",
                entity_id, collection_id
            )));
        }
        Ok(())
    }

    async fn remove_member(
        &self,
        collection_id: Uuid,
        entity_id: Uuid,
    ) -> Result<(), StorageError> {
        let conn = self
            .conn
            .lock()
            .map_err(|e| StorageError::Internal(e.to_string()))?;
        conn.execute(
            "DELETE FROM collection_members WHERE collection_id = ?1 AND entity_id = ?2",
            rusqlite::params![collection_id.to_string(), entity_id.to_string()],
        )
        .map_err(|e| StorageError::Internal(e.to_string()))?;
        Ok(())
    }

    async fn get_members(
        &self,
        collection_id: Uuid,
    ) -> Result<Vec<knowledge_core::features::entity::Entity>, StorageError> {
        let conn = self
            .conn
            .lock()
            .map_err(|e| StorageError::Internal(e.to_string()))?;
        let mut stmt = conn
            .prepare(&format!(
                "SELECT {cols} FROM entities e
                 JOIN collection_members cm ON cm.entity_id = e.id
                 WHERE cm.collection_id = ?1 AND e.is_active = 1",
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
                rusqlite::params![collection_id.to_string()],
                Self::parse_entity,
            )
            .map_err(|e| StorageError::Internal(e.to_string()))?;
        rows.map(|r| r.map_err(|e| StorageError::Internal(e.to_string())))
            .collect()
    }

    async fn get_entity_collections(
        &self,
        entity_id: Uuid,
    ) -> Result<Vec<Collection>, StorageError> {
        let conn = self
            .conn
            .lock()
            .map_err(|e| StorageError::Internal(e.to_string()))?;
        let mut stmt = conn
            .prepare(
                "SELECT c.id, c.name, c.description, c.created_at, c.updated_at
                 FROM collections c
                 JOIN collection_members cm ON cm.collection_id = c.id
                 WHERE cm.entity_id = ?1",
            )
            .map_err(|e| StorageError::Internal(e.to_string()))?;
        let rows = stmt
            .query_map(
                rusqlite::params![entity_id.to_string()],
                Self::parse_collection,
            )
            .map_err(|e| StorageError::Internal(e.to_string()))?;
        rows.map(|r| r.map_err(|e| StorageError::Internal(e.to_string())))
            .collect()
    }

    async fn is_member(&self, collection_id: Uuid, entity_id: Uuid) -> Result<bool, StorageError> {
        let conn = self
            .conn
            .lock()
            .map_err(|e| StorageError::Internal(e.to_string()))?;
        let mut stmt = conn
            .prepare("SELECT COUNT(*) FROM collection_members WHERE collection_id = ?1 AND entity_id = ?2")
            .map_err(|e| StorageError::Internal(e.to_string()))?;
        let count: i64 = stmt
            .query_row(
                rusqlite::params![collection_id.to_string(), entity_id.to_string()],
                |row| row.get(0),
            )
            .map_err(|e| StorageError::Internal(e.to_string()))?;
        Ok(count > 0)
    }
}
