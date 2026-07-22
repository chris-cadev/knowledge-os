use knowledge_core::features::entity::Entity;
use knowledge_core::ports::{EntityRepository, StorageError};
use async_trait::async_trait;
use rusqlite::{params, Connection};
use uuid::Uuid;
use std::sync::Mutex;

pub struct SqliteStore {
    conn: Mutex<Connection>,
}

impl SqliteStore {
    pub fn new(path: &str) -> Result<Self, StorageError> {
        let conn = Connection::open(path).map_err(|e| StorageError::Internal(e.to_string()))?;

        conn.execute_batch(
            "CREATE TABLE IF NOT EXISTS entities (
                id TEXT PRIMARY KEY,
                entity_type TEXT NOT NULL,
                created_at TEXT NOT NULL,
                updated_at TEXT NOT NULL,
                version INTEGER NOT NULL
            );"
        ).map_err(|e| StorageError::Internal(e.to_string()))?;

        Ok(Self {
            conn: Mutex::new(conn),
        })
    }
}

#[async_trait]
impl EntityRepository for SqliteStore {
    async fn get(&self, id: Uuid) -> Result<Option<Entity>, StorageError> {
        let conn = self.conn.lock().map_err(|e| StorageError::Internal(e.to_string()))?;
        let mut stmt = conn
            .prepare("SELECT id, entity_type, created_at, updated_at, version FROM entities WHERE id = ?1")
            .map_err(|e| StorageError::Internal(e.to_string()))?;

        let result = stmt.query_row(params![id.to_string()], |row| {
            Ok(Entity {
                id: Uuid::parse_str(&row.get::<_, String>(0)?).unwrap(),
                entity_type: serde_json::from_str(&row.get::<_, String>(1)?).unwrap(),
                created_at: chrono::DateTime::parse_from_rfc3339(&row.get::<_, String>(2)?)
                    .unwrap()
                    .with_timezone(&chrono::Utc),
                updated_at: chrono::DateTime::parse_from_rfc3339(&row.get::<_, String>(3)?)
                    .unwrap()
                    .with_timezone(&chrono::Utc),
                version: row.get(4)?,
            })
        }).map_err(|e| StorageError::Internal(e.to_string()))?;

        Ok(Some(result))
    }

    async fn save(&self, entity: &Entity) -> Result<(), StorageError> {
        let conn = self.conn.lock().map_err(|e| StorageError::Internal(e.to_string()))?;
        conn.execute(
            "INSERT OR REPLACE INTO entities (id, entity_type, created_at, updated_at, version) VALUES (?1, ?2, ?3, ?4, ?5)",
            params![
                entity.id.to_string(),
                serde_json::to_string(&entity.entity_type).unwrap(),
                entity.created_at.to_rfc3339(),
                entity.updated_at.to_rfc3339(),
                entity.version,
            ],
        ).map_err(|e| StorageError::Internal(e.to_string()))?;
        Ok(())
    }

    async fn delete(&self, id: Uuid) -> Result<(), StorageError> {
        let conn = self.conn.lock().map_err(|e| StorageError::Internal(e.to_string()))?;
        conn.execute("DELETE FROM entities WHERE id = ?1", params![id.to_string()])
            .map_err(|e| StorageError::Internal(e.to_string()))?;
        Ok(())
    }

    async fn list(&self) -> Result<Vec<Entity>, StorageError> {
        let conn = self.conn.lock().map_err(|e| StorageError::Internal(e.to_string()))?;
        let mut stmt = conn
            .prepare("SELECT id, entity_type, created_at, updated_at, version FROM entities")
            .map_err(|e| StorageError::Internal(e.to_string()))?;

        let rows = stmt.query_map([], |row| {
            Ok(Entity {
                id: Uuid::parse_str(&row.get::<_, String>(0)?).unwrap(),
                entity_type: serde_json::from_str(&row.get::<_, String>(1)?).unwrap(),
                created_at: chrono::DateTime::parse_from_rfc3339(&row.get::<_, String>(2)?)
                    .unwrap()
                    .with_timezone(&chrono::Utc),
                updated_at: chrono::DateTime::parse_from_rfc3339(&row.get::<_, String>(3)?)
                    .unwrap()
                    .with_timezone(&chrono::Utc),
                version: row.get(4)?,
            })
        }).map_err(|e| StorageError::Internal(e.to_string()))?;

        let mut entities = Vec::new();
        for row in rows {
            entities.push(row.map_err(|e| StorageError::Internal(e.to_string()))?);
        }
        Ok(entities)
    }
}
