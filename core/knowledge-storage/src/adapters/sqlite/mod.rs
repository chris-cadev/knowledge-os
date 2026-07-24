use async_trait::async_trait;
use knowledge_core::features::component::{Component, ComponentType};
use knowledge_core::features::entity::Entity;
use knowledge_core::features::relationship::Relationship;
use knowledge_core::ports::{
    ComponentRepository, EntityRepository, EntityResolver, EntityVersion, Event, EventLog,
    MergeAuditEntry, ResolutionCandidate, RelationshipRepository, SearchIndex, SearchQuery, SearchResult,
    StorageError, TransactionalWrite,
};
use rusqlite::{params, Connection, OptionalExtension};
use std::sync::Mutex;
use uuid::Uuid;

pub struct SqliteStore {
    conn: Mutex<Connection>,
}

const ENTITY_COLS: &str = "id, entity_type, is_active, created_at, updated_at, version";

impl SqliteStore {
    pub fn new(path: &str) -> Result<Self, StorageError> {
        let conn = Connection::open(path).map_err(|e| StorageError::Internal(e.to_string()))?;

        conn.execute_batch(
            "CREATE TABLE IF NOT EXISTS entities (
                id TEXT PRIMARY KEY,
                entity_type TEXT NOT NULL,
                is_active INTEGER NOT NULL DEFAULT 1,
                created_at TEXT NOT NULL,
                updated_at TEXT NOT NULL,
                version INTEGER NOT NULL
            );

            CREATE TABLE IF NOT EXISTS components (
                id TEXT PRIMARY KEY,
                entity_id TEXT NOT NULL,
                component_type TEXT NOT NULL,
                data TEXT NOT NULL,
                created_at TEXT NOT NULL,
                version INTEGER NOT NULL,
                FOREIGN KEY (entity_id) REFERENCES entities(id)
            );

            CREATE TABLE IF NOT EXISTS relationships (
                id TEXT PRIMARY KEY,
                source_id TEXT NOT NULL,
                target_id TEXT NOT NULL,
                relationship_type TEXT NOT NULL,
                is_active INTEGER NOT NULL DEFAULT 1,
                created_at TEXT NOT NULL,
                FOREIGN KEY (source_id) REFERENCES entities(id),
                FOREIGN KEY (target_id) REFERENCES entities(id)
            );

            CREATE TABLE IF NOT EXISTS events (
                id TEXT PRIMARY KEY,
                event_type TEXT NOT NULL,
                entity_id TEXT NOT NULL,
                timestamp TEXT NOT NULL,
                data TEXT NOT NULL
            );

            CREATE VIRTUAL TABLE IF NOT EXISTS entities_fts USING fts5(
                entity_id UNINDEXED,
                title,
                content,
                tags
            );

            CREATE TABLE IF NOT EXISTS entity_versions (
                entity_id TEXT NOT NULL,
                version INTEGER NOT NULL,
                snapshot TEXT NOT NULL,
                created_at TEXT NOT NULL,
                PRIMARY KEY (entity_id, version),
                FOREIGN KEY (entity_id) REFERENCES entities(id)
            );

            CREATE TABLE IF NOT EXISTS resolution_log (
                id TEXT PRIMARY KEY,
                source_id TEXT NOT NULL,
                source_title TEXT NOT NULL,
                target_id TEXT NOT NULL,
                target_title TEXT NOT NULL,
                strategy TEXT NOT NULL,
                confidence REAL NOT NULL,
                timestamp TEXT NOT NULL,
                reason TEXT NOT NULL,
                snapshot TEXT,  -- JSON snapshot of pre-merge state for undo
                FOREIGN KEY (target_id) REFERENCES entities(id)
            );

            CREATE TABLE IF NOT EXISTS resolution_candidates (
                id TEXT PRIMARY KEY,
                source_entity_id TEXT NOT NULL,
                candidate_entity_id TEXT NOT NULL,
                confidence REAL NOT NULL,
                strategy TEXT NOT NULL,
                evaluated_at TEXT NOT NULL,
                FOREIGN KEY (source_entity_id) REFERENCES entities(id),
                FOREIGN KEY (candidate_entity_id) REFERENCES entities(id)
            );",
        )
        .map_err(|e| StorageError::Internal(e.to_string()))?;

        Ok(Self {
            conn: Mutex::new(conn),
        })
    }

    fn parse_entity(row: &rusqlite::Row) -> Result<Entity, rusqlite::Error> {
        Ok(Entity {
            id: Uuid::parse_str(&row.get::<_, String>(0)?).unwrap(),
            entity_type: serde_json::from_str(&row.get::<_, String>(1)?).unwrap(),
            is_active: row.get::<_, i32>(2)? != 0,
            created_at: chrono::DateTime::parse_from_rfc3339(&row.get::<_, String>(3)?)
                .unwrap()
                .with_timezone(&chrono::Utc),
            updated_at: chrono::DateTime::parse_from_rfc3339(&row.get::<_, String>(4)?)
                .unwrap()
                .with_timezone(&chrono::Utc),
            version: row.get(5)?,
        })
    }

    fn parse_relationship(row: &rusqlite::Row) -> Result<Relationship, rusqlite::Error> {
        Ok(Relationship {
            id: Uuid::parse_str(&row.get::<_, String>(0)?).unwrap(),
            source_id: Uuid::parse_str(&row.get::<_, String>(1)?).unwrap(),
            target_id: Uuid::parse_str(&row.get::<_, String>(2)?).unwrap(),
            relationship_type: serde_json::from_str(&row.get::<_, String>(3)?).unwrap(),
            is_active: row.get::<_, i32>(4)? != 0,
            created_at: chrono::DateTime::parse_from_rfc3339(&row.get::<_, String>(5)?)
                .unwrap()
                .with_timezone(&chrono::Utc),
        })
    }

    fn parse_component(row: &rusqlite::Row) -> Result<Component, rusqlite::Error> {
        Ok(Component {
            id: Uuid::parse_str(&row.get::<_, String>(0)?).unwrap(),
            entity_id: Uuid::parse_str(&row.get::<_, String>(1)?).unwrap(),
            component_type: serde_json::from_str(&row.get::<_, String>(2)?).unwrap(),
            data: serde_json::from_str(&row.get::<_, String>(3)?).unwrap(),
            created_at: chrono::DateTime::parse_from_rfc3339(&row.get::<_, String>(4)?)
                .unwrap()
                .with_timezone(&chrono::Utc),
            version: row.get(5)?,
        })
    }

    fn parse_event(row: &rusqlite::Row) -> Result<Event, rusqlite::Error> {
        Ok(Event {
            id: Uuid::parse_str(&row.get::<_, String>(0)?).unwrap(),
            event_type: serde_json::from_str(&row.get::<_, String>(1)?).unwrap(),
            entity_id: Uuid::parse_str(&row.get::<_, String>(2)?).unwrap(),
            timestamp: chrono::DateTime::parse_from_rfc3339(&row.get::<_, String>(3)?)
                .unwrap()
                .with_timezone(&chrono::Utc),
            data: serde_json::from_str(&row.get::<_, String>(4)?).unwrap(),
        })
    }

    fn component_type_str(ct: &ComponentType) -> String {
        serde_json::to_string(ct).unwrap()
    }

    fn entity_type_str(entity: &Entity) -> String {
        serde_json::to_string(&entity.entity_type).unwrap()
    }
}

#[async_trait]
impl EntityRepository for SqliteStore {
    async fn get(&self, id: Uuid) -> Result<Option<Entity>, StorageError> {
        let conn = self.conn.lock().map_err(|e| StorageError::Internal(e.to_string()))?;
        let mut stmt = conn
            .prepare(&format!("SELECT {} FROM entities WHERE id = ?1", ENTITY_COLS))
            .map_err(|e| StorageError::Internal(e.to_string()))?;
        let result = stmt
            .query_row(params![id.to_string()], Self::parse_entity)
            .optional()
            .map_err(|e| StorageError::Internal(e.to_string()))?;
        Ok(result)
    }

    async fn save(&self, entity: &Entity) -> Result<(), StorageError> {
        let conn = self.conn.lock().map_err(|e| StorageError::Internal(e.to_string()))?;
        conn.execute(
            "INSERT OR REPLACE INTO entities (id, entity_type, is_active, created_at, updated_at, version) VALUES (?1, ?2, ?3, ?4, ?5, ?6)",
            params![
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
        let conn = self.conn.lock().map_err(|e| StorageError::Internal(e.to_string()))?;
        conn.execute("DELETE FROM components WHERE entity_id = ?1", params![id.to_string()])
            .map_err(|e| StorageError::Internal(e.to_string()))?;
        conn.execute("DELETE FROM relationships WHERE source_id = ?1 OR target_id = ?1", params![id.to_string()])
            .map_err(|e| StorageError::Internal(e.to_string()))?;
        conn.execute("DELETE FROM entities WHERE id = ?1", params![id.to_string()])
            .map_err(|e| StorageError::Internal(e.to_string()))?;
        conn.execute("DELETE FROM entities_fts WHERE entity_id = ?1", params![id.to_string()])
            .map_err(|e| StorageError::Internal(e.to_string()))?;
        Ok(())
    }

    async fn list(&self) -> Result<Vec<Entity>, StorageError> {
        let conn = self.conn.lock().map_err(|e| StorageError::Internal(e.to_string()))?;
        let mut stmt = conn
            .prepare(&format!("SELECT {} FROM entities WHERE is_active = 1", ENTITY_COLS))
            .map_err(|e| StorageError::Internal(e.to_string()))?;
        let rows = stmt.query_map([], Self::parse_entity)
            .map_err(|e| StorageError::Internal(e.to_string()))?;
        rows.map(|r| r.map_err(|e| StorageError::Internal(e.to_string()))).collect()
    }

    async fn find_by_type(&self, entity_type: &str) -> Result<Vec<Entity>, StorageError> {
        let conn = self.conn.lock().map_err(|e| StorageError::Internal(e.to_string()))?;
        let quoted_type = serde_json::to_string(&entity_type).unwrap();
        let mut stmt = conn
            .prepare(&format!("SELECT {} FROM entities WHERE entity_type = ?1 AND is_active = 1", ENTITY_COLS))
            .map_err(|e| StorageError::Internal(e.to_string()))?;
        let rows = stmt.query_map(params![quoted_type], Self::parse_entity)
            .map_err(|e| StorageError::Internal(e.to_string()))?;
        rows.map(|r| r.map_err(|e| StorageError::Internal(e.to_string()))).collect()
    }

    async fn find_by_title(&self, title: &str) -> Result<Vec<Entity>, StorageError> {
        let conn = self.conn.lock().map_err(|e| StorageError::Internal(e.to_string()))?;
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
        let rows = stmt.query_map(params![Self::component_type_str(&ComponentType::Title), title_json], Self::parse_entity)
            .map_err(|e| StorageError::Internal(e.to_string()))?;
        rows.map(|r| r.map_err(|e| StorageError::Internal(e.to_string()))).collect()
    }

    async fn increment_version(&self, id: Uuid) -> Result<(), StorageError> {
        let conn = self.conn.lock().map_err(|e| StorageError::Internal(e.to_string()))?;
        // Snapshot current state before incrementing
        let mut stmt = conn
            .prepare(&format!("SELECT {} FROM entities WHERE id = ?1", ENTITY_COLS))
            .map_err(|e| StorageError::Internal(e.to_string()))?;
        let entity = stmt.query_row(params![id.to_string()], Self::parse_entity)
            .map_err(|e| StorageError::Internal(e.to_string()))?;

        let snapshot = serde_json::json!({
            "entity_type": serde_json::to_string(&entity.entity_type).unwrap(),
            "is_active": entity.is_active,
            "version": entity.version,
            "updated_at": entity.updated_at.to_rfc3339(),
        });

        conn.execute(
            "INSERT INTO entity_versions (entity_id, version, snapshot, created_at) VALUES (?1, ?2, ?3, ?4)",
            params![id.to_string(), entity.version, snapshot.to_string(), chrono::Utc::now().to_rfc3339()],
        ).map_err(|e| StorageError::Internal(e.to_string()))?;

        conn.execute(
            "UPDATE entities SET version = version + 1, updated_at = ?1 WHERE id = ?2",
            params![chrono::Utc::now().to_rfc3339(), id.to_string()],
        ).map_err(|e| StorageError::Internal(e.to_string()))?;
        Ok(())
    }

    async fn find_by_component_type(&self, component_type: &str) -> Result<Vec<Entity>, StorageError> {
        let conn = self.conn.lock().map_err(|e| StorageError::Internal(e.to_string()))?;
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
        let rows = stmt.query_map(params![quoted_type], Self::parse_entity)
            .map_err(|e| StorageError::Internal(e.to_string()))?;
        rows.map(|r| r.map_err(|e| StorageError::Internal(e.to_string()))).collect()
    }

    async fn find_by_tag(&self, tag: &str) -> Result<Vec<Entity>, StorageError> {
        let conn = self.conn.lock().map_err(|e| StorageError::Internal(e.to_string()))?;
        // PONYTAIL: Tag matching via FTS5 tag column string search. Ceiling: substring matching,
        // not exact tag matching. Upgrade: JSON path query or normalized tag index.
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
        let rows = stmt.query_map(params![Self::component_type_str(&ComponentType::Tags), like_pattern], Self::parse_entity)
            .map_err(|e| StorageError::Internal(e.to_string()))?;
        rows.map(|r| r.map_err(|e| StorageError::Internal(e.to_string()))).collect()
    }

    async fn get_version_history(&self, entity_id: Uuid) -> Result<Vec<EntityVersion>, StorageError> {
        let conn = self.conn.lock().map_err(|e| StorageError::Internal(e.to_string()))?;
        let mut stmt = conn
            .prepare("SELECT entity_id, version, snapshot, created_at FROM entity_versions WHERE entity_id = ?1 ORDER BY version DESC")
            .map_err(|e| StorageError::Internal(e.to_string()))?;
        let rows = stmt.query_map(params![entity_id.to_string()], |row| {
            Ok(EntityVersion {
                entity_id: Uuid::parse_str(&row.get::<_, String>(0)?).unwrap(),
                version: row.get(1)?,
                snapshot: serde_json::from_str(&row.get::<_, String>(2)?).unwrap(),
                created_at: chrono::DateTime::parse_from_rfc3339(&row.get::<_, String>(3)?)
                    .unwrap()
                    .with_timezone(&chrono::Utc),
            })
        })
        .map_err(|e| StorageError::Internal(e.to_string()))?;
        rows.map(|r| r.map_err(|e| StorageError::Internal(e.to_string()))).collect()
    }
}

#[async_trait]
impl RelationshipRepository for SqliteStore {
    async fn get(&self, id: Uuid) -> Result<Option<Relationship>, StorageError> {
        let conn = self.conn.lock().map_err(|e| StorageError::Internal(e.to_string()))?;
        let mut stmt = conn
            .prepare("SELECT id, source_id, target_id, relationship_type, is_active, created_at FROM relationships WHERE id = ?1")
            .map_err(|e| StorageError::Internal(e.to_string()))?;
        stmt.query_row(params![id.to_string()], Self::parse_relationship)
            .optional()
            .map_err(|e| StorageError::Internal(e.to_string()))
    }

    async fn save(&self, relationship: &Relationship) -> Result<(), StorageError> {
        let conn = self.conn.lock().map_err(|e| StorageError::Internal(e.to_string()))?;
        conn.execute(
            "INSERT OR REPLACE INTO relationships (id, source_id, target_id, relationship_type, is_active, created_at) VALUES (?1, ?2, ?3, ?4, ?5, ?6)",
            params![
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
        let conn = self.conn.lock().map_err(|e| StorageError::Internal(e.to_string()))?;
        conn.execute("DELETE FROM relationships WHERE id = ?1", params![id.to_string()])
            .map_err(|e| StorageError::Internal(e.to_string()))?;
        Ok(())
    }

    async fn by_source(&self, source_id: Uuid) -> Result<Vec<Relationship>, StorageError> {
        let conn = self.conn.lock().map_err(|e| StorageError::Internal(e.to_string()))?;
        let mut stmt = conn
            .prepare("SELECT id, source_id, target_id, relationship_type, is_active, created_at FROM relationships WHERE source_id = ?1 AND is_active = 1")
            .map_err(|e| StorageError::Internal(e.to_string()))?;
        let rows = stmt.query_map(params![source_id.to_string()], Self::parse_relationship)
            .map_err(|e| StorageError::Internal(e.to_string()))?;
        rows.map(|r| r.map_err(|e| StorageError::Internal(e.to_string()))).collect()
    }

    async fn by_target(&self, target_id: Uuid) -> Result<Vec<Relationship>, StorageError> {
        let conn = self.conn.lock().map_err(|e| StorageError::Internal(e.to_string()))?;
        let mut stmt = conn
            .prepare("SELECT id, source_id, target_id, relationship_type, is_active, created_at FROM relationships WHERE target_id = ?1 AND is_active = 1")
            .map_err(|e| StorageError::Internal(e.to_string()))?;
        let rows = stmt.query_map(params![target_id.to_string()], Self::parse_relationship)
            .map_err(|e| StorageError::Internal(e.to_string()))?;
        rows.map(|r| r.map_err(|e| StorageError::Internal(e.to_string()))).collect()
    }

    async fn find_by_source_and_target(
        &self,
        source_id: Uuid,
        target_id: Uuid,
    ) -> Result<Option<Relationship>, StorageError> {
        let conn = self.conn.lock().map_err(|e| StorageError::Internal(e.to_string()))?;
        let mut stmt = conn
            .prepare("SELECT id, source_id, target_id, relationship_type, is_active, created_at FROM relationships WHERE source_id = ?1 AND target_id = ?2 AND is_active = 1")
            .map_err(|e| StorageError::Internal(e.to_string()))?;
        stmt.query_row(params![source_id.to_string(), target_id.to_string()], Self::parse_relationship)
            .optional()
            .map_err(|e| StorageError::Internal(e.to_string()))
    }

    async fn find_by_type(&self, relationship_type: &str) -> Result<Vec<Relationship>, StorageError> {
        let conn = self.conn.lock().map_err(|e| StorageError::Internal(e.to_string()))?;
        let quoted_type = serde_json::to_string(&relationship_type).unwrap();
        let mut stmt = conn
            .prepare("SELECT id, source_id, target_id, relationship_type, is_active, created_at FROM relationships WHERE relationship_type = ?1 AND is_active = 1")
            .map_err(|e| StorageError::Internal(e.to_string()))?;
        let rows = stmt.query_map(params![quoted_type], Self::parse_relationship)
            .map_err(|e| StorageError::Internal(e.to_string()))?;
        rows.map(|r| r.map_err(|e| StorageError::Internal(e.to_string()))).collect()
    }
}

#[async_trait]
impl ComponentRepository for SqliteStore {
    async fn get(&self, entity_id: Uuid) -> Result<Vec<Component>, StorageError> {
        let conn = self.conn.lock().map_err(|e| StorageError::Internal(e.to_string()))?;
        let mut stmt = conn
            .prepare("SELECT id, entity_id, component_type, data, created_at, version FROM components WHERE entity_id = ?1")
            .map_err(|e| StorageError::Internal(e.to_string()))?;
        let rows = stmt.query_map(params![entity_id.to_string()], Self::parse_component)
            .map_err(|e| StorageError::Internal(e.to_string()))?;
        rows.map(|r| r.map_err(|e| StorageError::Internal(e.to_string()))).collect()
    }

    async fn save(&self, component: &Component) -> Result<(), StorageError> {
        let conn = self.conn.lock().map_err(|e| StorageError::Internal(e.to_string()))?;
        conn.execute(
            "INSERT OR REPLACE INTO components (id, entity_id, component_type, data, created_at, version) VALUES (?1, ?2, ?3, ?4, ?5, ?6)",
            params![
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
        let conn = self.conn.lock().map_err(|e| StorageError::Internal(e.to_string()))?;
        conn.execute("DELETE FROM components WHERE id = ?1", params![id.to_string()])
            .map_err(|e| StorageError::Internal(e.to_string()))?;
        Ok(())
    }

    async fn find_by_type(&self, entity_id: Uuid, component_type: &str) -> Result<Vec<Component>, StorageError> {
        let conn = self.conn.lock().map_err(|e| StorageError::Internal(e.to_string()))?;
        let quoted_type = serde_json::to_string(&component_type).unwrap();
        let mut stmt = conn
            .prepare("SELECT id, entity_id, component_type, data, created_at, version FROM components WHERE entity_id = ?1 AND component_type = ?2")
            .map_err(|e| StorageError::Internal(e.to_string()))?;
        let rows = stmt.query_map(params![entity_id.to_string(), quoted_type], Self::parse_component)
            .map_err(|e| StorageError::Internal(e.to_string()))?;
        rows.map(|r| r.map_err(|e| StorageError::Internal(e.to_string()))).collect()
    }

    async fn update_data(&self, id: Uuid, data: serde_json::Value) -> Result<(), StorageError> {
        let conn = self.conn.lock().map_err(|e| StorageError::Internal(e.to_string()))?;
        let data_str = data.to_string();
        conn.execute(
            "UPDATE components SET data = ?1, version = version + 1 WHERE id = ?2",
            params![data_str, id.to_string()],
        ).map_err(|e| StorageError::Internal(e.to_string()))?;
        Ok(())
    }

    async fn find_by_component_data(&self, component_type: &str, _json_path: &str, value: &str) -> Result<Vec<Component>, StorageError> {
        let conn = self.conn.lock().map_err(|e| StorageError::Internal(e.to_string()))?;
        let quoted_type = serde_json::to_string(&component_type).unwrap();
        let like_pattern = format!("%{}%", value);
        let mut stmt = conn
            .prepare("SELECT id, entity_id, component_type, data, created_at, version FROM components WHERE component_type = ?1 AND data LIKE ?2")
            .map_err(|e| StorageError::Internal(e.to_string()))?;
        let rows = stmt.query_map(params![quoted_type, like_pattern], Self::parse_component)
            .map_err(|e| StorageError::Internal(e.to_string()))?;
        rows.map(|r| r.map_err(|e| StorageError::Internal(e.to_string()))).collect()
    }

    async fn delete_by_entity(&self, entity_id: Uuid) -> Result<(), StorageError> {
        let conn = self.conn.lock().map_err(|e| StorageError::Internal(e.to_string()))?;
        conn.execute("DELETE FROM components WHERE entity_id = ?1", params![entity_id.to_string()])
            .map_err(|e| StorageError::Internal(e.to_string()))?;
        Ok(())
    }
}

#[async_trait]
impl SearchIndex for SqliteStore {
    async fn index_entity(&self, entity: &Entity, components: &[Component]) -> Result<(), StorageError> {
        let conn = self.conn.lock().map_err(|e| StorageError::Internal(e.to_string()))?;

        let title = components.iter()
            .find(|c| c.component_type == ComponentType::Title)
            .and_then(|c| c.data.as_str().map(String::from))
            .unwrap_or_default();

        let content = components.iter()
            .find(|c| c.component_type == ComponentType::Content)
            .and_then(|c| c.data.as_str().map(String::from))
            .unwrap_or_default();

        let tags = components.iter()
            .find(|c| c.component_type == ComponentType::Tags)
            .and_then(|c| c.data.as_array().map(|a| {
                a.iter().filter_map(|v| v.as_str()).collect::<Vec<_>>().join(", ")
            }))
            .unwrap_or_default();

        conn.execute(
            "DELETE FROM entities_fts WHERE entity_id = ?1",
            params![entity.id.to_string()],
        ).map_err(|e| StorageError::Internal(e.to_string()))?;
        conn.execute(
            "INSERT INTO entities_fts (entity_id, title, content, tags) VALUES (?1, ?2, ?3, ?4)",
            params![entity.id.to_string(), title, content, tags],
        ).map_err(|e| StorageError::Internal(e.to_string()))?;
        Ok(())
    }

    async fn remove_entity(&self, entity_id: Uuid) -> Result<(), StorageError> {
        let conn = self.conn.lock().map_err(|e| StorageError::Internal(e.to_string()))?;
        conn.execute(
            "DELETE FROM entities_fts WHERE entity_id = ?1",
            params![entity_id.to_string()],
        ).map_err(|e| StorageError::Internal(e.to_string()))?;
        Ok(())
    }

    async fn search(&self, query: &SearchQuery) -> Result<Vec<SearchResult>, StorageError> {
        let conn = self.conn.lock().map_err(|e| StorageError::Internal(e.to_string()))?;

        let fts_query = format!("{} OR {}", query.query, query.query);

        let raw_results: Vec<(String, f64, String)> = {
            let mut stmt = conn.prepare(
                "SELECT entity_id, bm25(entities_fts) as rank, snippet(entities_fts, 2, '<b>', '</b>', '...', 32) as snip FROM entities_fts WHERE entities_fts MATCH ?1 ORDER BY rank"
            ).map_err(|e| StorageError::Internal(e.to_string()))?;
            let results: Vec<_> = stmt.query_map(params![fts_query], |row| {
                Ok((row.get::<_, String>(0)?, row.get::<_, f64>(1)?, row.get::<_, String>(2)?))
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
                    let mut estmt = conn.prepare(
                        &format!("SELECT {} FROM entities WHERE id = ?1 AND is_active = 1", ENTITY_COLS)
                    ).map_err(|e| StorageError::Internal(e.to_string()))?;
                    match estmt.query_row(params![id_str], Self::parse_entity) {
                        Ok(entity) => {
                            let quoted = serde_json::to_value(t).unwrap();
                            let stored = serde_json::to_value(&entity.entity_type).unwrap();
                            if quoted != stored { pass = false; }
                        }
                        Err(_) => pass = false,
                    }
                }

                if pass {
                    if let Some(ref tag_val) = query.tag {
                        let mut tag_stmt = conn.prepare(
                            "SELECT tags FROM entities_fts WHERE entity_id = ?1"
                        ).map_err(|e| StorageError::Internal(e.to_string()))?;
                        if let Ok(ftags) = tag_stmt.query_row(params![id_str], |row| row.get::<_, String>(0)) {
                            let tags_list: Vec<&str> = ftags.split(", ").collect();
                            if !tags_list.contains(&tag_val.as_str()) { pass = false; }
                        } else { pass = false; }
                    }
                }

                if pass {
                    let snip = if snippet.is_empty() { None } else { Some(snippet) };
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
        let conn = self.conn.lock().map_err(|e| StorageError::Internal(e.to_string()))?;

        conn.execute_batch(
            "DROP TABLE IF EXISTS entities_fts;
             CREATE VIRTUAL TABLE entities_fts USING fts5(
                 entity_id UNINDEXED,
                 title,
                 content,
                 tags
             );"
        ).map_err(|e| StorageError::Internal(e.to_string()))?;

        for (entity, components) in entities {
            let title = components.iter()
                .find(|c| c.component_type == ComponentType::Title)
                .and_then(|c| c.data.as_str().map(String::from))
                .unwrap_or_default();

            let content = components.iter()
                .find(|c| c.component_type == ComponentType::Content)
                .and_then(|c| c.data.as_str().map(String::from))
                .unwrap_or_default();

            let tags = components.iter()
                .find(|c| c.component_type == ComponentType::Tags)
                .and_then(|c| c.data.as_array().map(|a| {
                    a.iter().filter_map(|v| v.as_str()).collect::<Vec<_>>().join(", ")
                }))
                .unwrap_or_default();

            conn.execute(
                "INSERT INTO entities_fts (entity_id, title, content, tags) VALUES (?1, ?2, ?3, ?4)",
                params![entity.id.to_string(), title, content, tags],
            ).map_err(|e| StorageError::Internal(e.to_string()))?;
        }

        Ok(())
    }
}

#[async_trait]
impl EventLog for SqliteStore {
    async fn append(&self, event: &Event) -> Result<(), StorageError> {
        let conn = self.conn.lock().map_err(|e| StorageError::Internal(e.to_string()))?;
        conn.execute(
            "INSERT INTO events (id, event_type, entity_id, timestamp, data) VALUES (?1, ?2, ?3, ?4, ?5)",
            params![
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
        let conn = self.conn.lock().map_err(|e| StorageError::Internal(e.to_string()))?;
        let mut stmt = conn
            .prepare("SELECT id, event_type, entity_id, timestamp, data FROM events WHERE entity_id = ?1 ORDER BY timestamp DESC")
            .map_err(|e| StorageError::Internal(e.to_string()))?;
        let rows = stmt.query_map(params![entity_id.to_string()], Self::parse_event)
            .map_err(|e| StorageError::Internal(e.to_string()))?;
        rows.map(|r| r.map_err(|e| StorageError::Internal(e.to_string()))).collect()
    }
}

#[async_trait]
impl TransactionalWrite for SqliteStore {
    async fn save_entity_with_components(
        &self,
        entity: &Entity,
        components: &[Component],
        event: &Event,
    ) -> Result<(), StorageError> {
        let conn = self.conn.lock().map_err(|e| StorageError::Internal(e.to_string()))?;

        // PONYTAIL: Manual BEGIN/COMMIT. Ceiling: no nested transactions, no savepoint support.
        // Upgrade: rusqlite Transaction type or deadpool integration.
        conn.execute_batch("BEGIN IMMEDIATE")
            .map_err(|e| StorageError::Internal(e.to_string()))?;

        let result = (|| -> Result<(), StorageError> {
            conn.execute(
                "INSERT OR REPLACE INTO entities (id, entity_type, is_active, created_at, updated_at, version) VALUES (?1, ?2, ?3, ?4, ?5, ?6)",
                params![
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
                    params![
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
                params![
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
        let conn = self.conn.lock().map_err(|e| StorageError::Internal(e.to_string()))?;

        conn.execute_batch("BEGIN IMMEDIATE")
            .map_err(|e| StorageError::Internal(e.to_string()))?;

        let result = (|| -> Result<(), StorageError> {
            conn.execute(
                "INSERT OR REPLACE INTO entities (id, entity_type, is_active, created_at, updated_at, version) VALUES (?1, ?2, ?3, ?4, ?5, ?6)",
                params![
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
                params![entity.id.to_string()],
            ).map_err(|e| StorageError::Internal(e.to_string()))?;

            for component in components {
                conn.execute(
                    "INSERT OR REPLACE INTO components (id, entity_id, component_type, data, created_at, version) VALUES (?1, ?2, ?3, ?4, ?5, ?6)",
                    params![
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
                params![
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

#[async_trait]
impl EntityResolver for SqliteStore {
    async fn find_candidates(&self, entity: &Entity, title: &str, content: Option<&str>) -> Result<Vec<ResolutionCandidate>, StorageError> {
        let conn = self.conn.lock().map_err(|e| StorageError::Internal(e.to_string()))?;

        // Get all active entities of the same type
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

        let entities: Vec<Entity> = stmt.query_map(
            params![entity_type_json, entity.id.to_string()],
            Self::parse_entity,
        )
        .map_err(|e| StorageError::Internal(e.to_string()))?
        .filter_map(|r| r.ok())
        .collect();

        // Get titles for each entity
        let mut title_stmt = conn
            .prepare("SELECT c.data FROM components c WHERE c.entity_id = ?1 AND c.component_type = ?2")
            .map_err(|e| StorageError::Internal(e.to_string()))?;

        // Get content for each entity (for ContentMatch strategy)
        let mut content_stmt = conn
            .prepare("SELECT c.data FROM components c WHERE c.entity_id = ?1 AND c.component_type = ?2")
            .map_err(|e| StorageError::Internal(e.to_string()))?;

        let title_ct = Self::component_type_str(&ComponentType::Title);
        let content_ct = Self::component_type_str(&ComponentType::Content);

        let mut entity_data: Vec<(Entity, String, Option<String>)> = Vec::new();
        for e in &entities {
            let title_json: Result<String, _> = title_stmt.query_row(
                params![e.id.to_string(), title_ct],
                |row| row.get(0),
            );

            if let Ok(title_json) = title_json {
                if let Ok(t) = serde_json::from_str::<String>(&title_json) {
                    // Also fetch content for this entity
                    let content_json: Result<String, _> = content_stmt.query_row(
                        params![e.id.to_string(), content_ct],
                        |row| row.get(0),
                    );
                    let c = content_json
                        .ok()
                        .and_then(|json| serde_json::from_str::<String>(&json).ok());

                    entity_data.push((e.clone(), t, c));
                }
            }
        }

        // Use fuzzy resolver for candidate matching
        let resolver = crate::fuzzy::FuzzyEntityResolver::new();
        let candidates = resolver.find_candidates(entity, title, content, &entity_data);

        Ok(candidates)
    }

    async fn merge(&self, canonical_id: Uuid, duplicate_id: Uuid, _confidence: f64) -> Result<(), StorageError> {
        let conn = self.conn.lock().map_err(|e| StorageError::Internal(e.to_string()))?;

        conn.execute_batch("BEGIN IMMEDIATE")
            .map_err(|e| StorageError::Internal(e.to_string()))?;

        let result = (|| -> Result<(), StorageError> {
            conn.execute(
                "UPDATE relationships SET source_id = ?1 WHERE source_id = ?2",
                params![canonical_id.to_string(), duplicate_id.to_string()],
            ).map_err(|e| StorageError::Internal(e.to_string()))?;

            conn.execute(
                "UPDATE relationships SET target_id = ?1 WHERE target_id = ?2",
                params![canonical_id.to_string(), duplicate_id.to_string()],
            ).map_err(|e| StorageError::Internal(e.to_string()))?;

            conn.execute(
                "DELETE FROM relationships WHERE source_id = ?1 AND target_id = ?1",
                params![canonical_id.to_string()],
            ).map_err(|e| StorageError::Internal(e.to_string()))?;

            conn.execute(
                "UPDATE components SET entity_id = ?1 WHERE entity_id = ?2",
                params![canonical_id.to_string(), duplicate_id.to_string()],
            ).map_err(|e| StorageError::Internal(e.to_string()))?;

            conn.execute(
                "DELETE FROM entities_fts WHERE entity_id = ?1",
                params![duplicate_id.to_string()],
            ).map_err(|e| StorageError::Internal(e.to_string()))?;

            conn.execute(
                "DELETE FROM entity_versions WHERE entity_id = ?1",
                params![duplicate_id.to_string()],
            ).map_err(|e| StorageError::Internal(e.to_string()))?;

            conn.execute(
                "DELETE FROM events WHERE entity_id = ?1",
                params![duplicate_id.to_string()],
            ).map_err(|e| StorageError::Internal(e.to_string()))?;

            conn.execute(
                "DELETE FROM entities WHERE id = ?1",
                params![duplicate_id.to_string()],
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

    async fn log_merge(&self, entry: &MergeAuditEntry) -> Result<(), StorageError> {
        let conn = self.conn.lock().map_err(|e| StorageError::Internal(e.to_string()))?;

        conn.execute(
            "INSERT INTO resolution_log (id, source_id, source_title, target_id, target_title, strategy, confidence, timestamp, reason, snapshot)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10)",
            params![
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
        let conn = self.conn.lock().map_err(|e| StorageError::Internal(e.to_string()))?;

        // Get the merge entry with snapshot
        let entry = conn.query_row(
            "SELECT source_id, target_id, snapshot FROM resolution_log WHERE id = ?1",
            params![merge_id.to_string()],
            |row| {
                let source_id: String = row.get(0)?;
                let target_id: String = row.get(1)?;
                let snapshot: Option<String> = row.get(2)?;
                Ok((
                    Uuid::parse_str(&source_id).unwrap(),
                    Uuid::parse_str(&target_id).unwrap(),
                    snapshot,
                ))
            },
        ).map_err(|e| StorageError::Internal(e.to_string()))?;

        let (source_id, target_id, snapshot) = entry;

        let snapshot_data: serde_json::Value = if let Some(snap) = &snapshot {
            serde_json::from_str(snap).map_err(|e| StorageError::Internal(format!("Invalid snapshot: {}", e)))?
        } else {
            return Err(StorageError::Internal("No snapshot available for undo".to_string()));
        };

        conn.execute_batch("BEGIN IMMEDIATE")
            .map_err(|e| StorageError::Internal(e.to_string()))?;

        let result = (|| -> Result<(), StorageError> {
            // 1. Remove target's transferred components (they now point to source after merge)
            conn.execute(
                "DELETE FROM components WHERE entity_id = ?1",
                params![target_id.to_string()],
            ).map_err(|e| StorageError::Internal(e.to_string()))?;

            // 2. Restore target's original components from snapshot
            if let Some(target_comps) = snapshot_data["target"]["components"].as_array() {
                for comp in target_comps {
                    let comp_id = comp["id"].as_str().unwrap_or("");
                    let comp_type = comp["component_type"].as_str().unwrap_or("");
                    let data = comp["data"].to_string();
                    let created_at = comp["created_at"].as_str().unwrap_or("");
                    let version = comp["version"].as_i64().unwrap_or(1);

                    conn.execute(
                        "INSERT OR REPLACE INTO components (id, entity_id, component_type, data, created_at, version) VALUES (?1, ?2, ?3, ?4, ?5, ?6)",
                        params![comp_id, target_id.to_string(), comp_type, data, created_at, version],
                    ).map_err(|e| StorageError::Internal(e.to_string()))?;
                }
            }

            // 3. Restore target's original relationships from snapshot
            // First remove any relationships pointing to/from target (from merge reassignment)
            conn.execute(
                "DELETE FROM relationships WHERE source_id = ?1 OR target_id = ?1",
                params![target_id.to_string()],
            ).map_err(|e| StorageError::Internal(e.to_string()))?;

            if let Some(target_rels) = snapshot_data["target"]["relationships"].as_array() {
                for rel in target_rels {
                    let rel_id = rel["id"].as_str().unwrap_or("");
                    let target_ref = rel["target_id"].as_str().unwrap_or("");
                    let rel_type = rel["relationship_type"].as_str().unwrap_or("");
                    let is_active = rel["is_active"].as_bool().unwrap_or(true);
                    let created_at = rel["created_at"].as_str().unwrap_or("");

                    conn.execute(
                        "INSERT OR REPLACE INTO relationships (id, source_id, target_id, relationship_type, is_active, created_at) VALUES (?1, ?2, ?3, ?4, ?5, ?6)",
                        params![rel_id, target_id.to_string(), target_ref, rel_type, is_active as i32, created_at],
                    ).map_err(|e| StorageError::Internal(e.to_string()))?;
                }
            }

            // 4. Recreate source entity row from snapshot
            let source = &snapshot_data["source"];
            let entity_type = source["entity_type"].as_str().unwrap_or("Article");
            let is_active = source["is_active"].as_bool().unwrap_or(true);
            let created_at = source["created_at"].as_str()
                .map(|s| s.to_string())
                .unwrap_or_else(|| chrono::Utc::now().to_rfc3339());
            let updated_at = source["updated_at"].as_str()
                .map(|s| s.to_string())
                .unwrap_or_else(|| chrono::Utc::now().to_rfc3339());
            let version = source["version"].as_i64().unwrap_or(1);

            conn.execute(
                "INSERT OR REPLACE INTO entities (id, entity_type, is_active, created_at, updated_at, version) VALUES (?1, ?2, ?3, ?4, ?5, ?6)",
                params![source_id.to_string(), entity_type, is_active as i32, created_at, updated_at, version],
            ).map_err(|e| StorageError::Internal(e.to_string()))?;

            // 5. Recreate source components from snapshot
            if let Some(source_comps) = source["components"].as_array() {
                for comp in source_comps {
                    let comp_id = comp["id"].as_str().unwrap_or("");
                    let comp_type = comp["component_type"].as_str().unwrap_or("");
                    let data = comp["data"].to_string();
                    let created_at = comp["created_at"].as_str().unwrap_or("");
                    let version = comp["version"].as_i64().unwrap_or(1);

                    conn.execute(
                        "INSERT OR REPLACE INTO components (id, entity_id, component_type, data, created_at, version) VALUES (?1, ?2, ?3, ?4, ?5, ?6)",
                        params![comp_id, source_id.to_string(), comp_type, data, created_at, version],
                    ).map_err(|e| StorageError::Internal(e.to_string()))?;
                }
            }

            // 6. Remove the merge entry
            conn.execute(
                "DELETE FROM resolution_log WHERE id = ?1",
                params![merge_id.to_string()],
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

    async fn get_merge_history(&self, entity_id: Uuid) -> Result<Vec<MergeAuditEntry>, StorageError> {
        let conn = self.conn.lock().map_err(|e| StorageError::Internal(e.to_string()))?;

        let mut stmt = conn
            .prepare(
                "SELECT id, source_id, source_title, target_id, target_title, strategy, confidence, timestamp, reason, snapshot
                 FROM resolution_log WHERE source_id = ?1 OR target_id = ?1 ORDER BY timestamp DESC"
            )
            .map_err(|e| StorageError::Internal(e.to_string()))?;

        let entries = stmt.query_map(params![entity_id.to_string()], |row| {
            Ok(MergeAuditEntry {
                id: Uuid::parse_str(&row.get::<_, String>(0)?).unwrap(),
                source_id: Uuid::parse_str(&row.get::<_, String>(1)?).unwrap(),
                source_title: row.get(2)?,
                target_id: Uuid::parse_str(&row.get::<_, String>(3)?).unwrap(),
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

        let entries: Vec<MergeAuditEntry> = entries
            .filter_map(|r| r.ok())
            .collect();

        Ok(entries)
    }

    async fn get_all_merge_history(&self) -> Result<Vec<MergeAuditEntry>, StorageError> {
        let conn = self.conn.lock().map_err(|e| StorageError::Internal(e.to_string()))?;

        let mut stmt = conn
            .prepare(
                "SELECT id, source_id, source_title, target_id, target_title, strategy, confidence, timestamp, reason, snapshot
                 FROM resolution_log ORDER BY timestamp DESC"
            )
            .map_err(|e| StorageError::Internal(e.to_string()))?;

        let entries = stmt.query_map([], |row| {
            Ok(MergeAuditEntry {
                id: Uuid::parse_str(&row.get::<_, String>(0)?).unwrap(),
                source_id: Uuid::parse_str(&row.get::<_, String>(1)?).unwrap(),
                source_title: row.get(2)?,
                target_id: Uuid::parse_str(&row.get::<_, String>(3)?).unwrap(),
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

        let entries: Vec<MergeAuditEntry> = entries
            .filter_map(|r| r.ok())
            .collect();

        Ok(entries)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use knowledge_core::features::entity::EntityType;
    use knowledge_core::features::component::ComponentType;
    use knowledge_core::features::relationship::RelationshipType;
    use knowledge_core::ports::{EntityRepository, ComponentRepository, RelationshipRepository, SearchIndex, EventLog, EventType};

    fn test_store() -> SqliteStore {
        SqliteStore::new(":memory:").unwrap()
    }

    #[tokio::test]
    async fn test_entity_crud() {
        let store = test_store();
        let mut entity = Entity::new(EntityType::new("Article"));

        EntityRepository::save(&store, &entity).await.unwrap();
        let loaded = EntityRepository::get(&store, entity.id).await.unwrap().unwrap();
        assert_eq!(loaded.id, entity.id);
        assert_eq!(loaded.entity_type, EntityType::new("Article"));
        assert!(loaded.is_active);

        entity.touch();
        EntityRepository::save(&store, &entity).await.unwrap();
        let loaded = EntityRepository::get(&store, entity.id).await.unwrap().unwrap();
        assert_eq!(loaded.version, 2);

        let all = EntityRepository::list(&store).await.unwrap();
        assert_eq!(all.len(), 1);

        let articles = EntityRepository::find_by_type(&store, "Article").await.unwrap();
        assert_eq!(articles.len(), 1);

        EntityRepository::delete(&store, entity.id).await.unwrap();
        let all = EntityRepository::list(&store).await.unwrap();
        assert!(all.is_empty());
    }

    #[tokio::test]
    async fn test_component_crud() {
        let store = test_store();
        let entity = Entity::new(EntityType::new("Note"));
        EntityRepository::save(&store, &entity).await.unwrap();

        let component = Component::new(entity.id, ComponentType::Title, serde_json::json!("Test Title"));
        ComponentRepository::save(&store, &component).await.unwrap();

        let components = ComponentRepository::get(&store, entity.id).await.unwrap();
        assert_eq!(components.len(), 1);
        assert_eq!(components[0].component_type, ComponentType::Title);

        let found = ComponentRepository::find_by_type(&store, entity.id, "Title").await.unwrap();
        assert_eq!(found.len(), 1);

        ComponentRepository::update_data(&store, component.id, serde_json::json!("Updated Title")).await.unwrap();
        let updated = ComponentRepository::get(&store, entity.id).await.unwrap();
        assert_eq!(updated[0].data, serde_json::json!("Updated Title"));
        assert_eq!(updated[0].version, 2);

        ComponentRepository::delete(&store, component.id).await.unwrap();
        let components = ComponentRepository::get(&store, entity.id).await.unwrap();
        assert!(components.is_empty());
    }

    #[tokio::test]
    async fn test_relationship_crud() {
        let store = test_store();
        let entity1 = Entity::new(EntityType::new("Article"));
        let entity2 = Entity::new(EntityType::new("Concept"));
        EntityRepository::save(&store, &entity1).await.unwrap();
        EntityRepository::save(&store, &entity2).await.unwrap();

        let rel = Relationship::new(entity1.id, entity2.id, RelationshipType::References);
        RelationshipRepository::save(&store, &rel).await.unwrap();

        let rels = RelationshipRepository::by_source(&store, entity1.id).await.unwrap();
        assert_eq!(rels.len(), 1);

        let rels = RelationshipRepository::by_target(&store, entity2.id).await.unwrap();
        assert_eq!(rels.len(), 1);

        let found = RelationshipRepository::find_by_source_and_target(&store, entity1.id, entity2.id).await.unwrap();
        assert!(found.is_some());

        let refs = RelationshipRepository::find_by_type(&store, "References").await.unwrap();
        assert_eq!(refs.len(), 1);

        RelationshipRepository::delete(&store, rel.id).await.unwrap();
        let rels = RelationshipRepository::by_source(&store, entity1.id).await.unwrap();
        assert!(rels.is_empty());
    }

    #[tokio::test]
    async fn test_search_index() {
        let store = test_store();
        let entity = Entity::new(EntityType::new("Article"));
        let entity_id = entity.id;

        let components = vec![
            Component::new(entity_id, ComponentType::Title, serde_json::json!("Test Title")),
            Component::new(entity_id, ComponentType::Content, serde_json::json!("Some content here")),
            Component::new(entity_id, ComponentType::Tags, serde_json::json!(["rust", "test"])),
        ];

        store.index_entity(&entity, &components).await.unwrap();

        let results = store.search(&SearchQuery {
            query: "Test".to_string(),
            entity_type: None,
            tag: None,
        }).await.unwrap();
        assert_eq!(results.len(), 1);
        assert_eq!(results[0].entity_id, entity_id);
        assert!(results[0].score < 0.0);

        let results = store.search(&SearchQuery {
            query: "content".to_string(),
            entity_type: None,
            tag: Some("rust".to_string()),
        }).await.unwrap();
        assert_eq!(results.len(), 1);

        let results = store.search(&SearchQuery {
            query: "nonexistent".to_string(),
            entity_type: None,
            tag: None,
        }).await.unwrap();
        assert!(results.is_empty());
    }

    #[tokio::test]
    async fn test_search_rebuild() {
        let store = test_store();
        let entity = Entity::new(EntityType::new("Article"));

        let components = vec![
            Component::new(entity.id, ComponentType::Title, serde_json::json!("Test Title")),
            Component::new(entity.id, ComponentType::Content, serde_json::json!("Some content here")),
        ];

        store.index_entity(&entity, &components).await.unwrap();

        let results = store.search(&SearchQuery {
            query: "Test".to_string(),
            entity_type: None,
            tag: None,
        }).await.unwrap();
        assert_eq!(results.len(), 1);

        store.rebuild(&[(entity.clone(), components.clone())]).await.unwrap();

        let results = store.search(&SearchQuery {
            query: "Test".to_string(),
            entity_type: None,
            tag: None,
        }).await.unwrap();
        assert_eq!(results.len(), 1);
    }

    #[tokio::test]
    async fn test_event_log() {
        let store = test_store();
        let entity = Entity::new(EntityType::new("Article"));

        let event = Event {
            id: Uuid::new_v4(),
            event_type: EventType::EntityCreated,
            entity_id: entity.id,
            timestamp: chrono::Utc::now(),
            data: serde_json::json!({"entity_type": "Article"}),
        };

        store.append(&event).await.unwrap();

        let events = store.list_by_entity(entity.id).await.unwrap();
        assert_eq!(events.len(), 1);
        assert_eq!(events[0].event_type, EventType::EntityCreated);
    }

    #[tokio::test]
    async fn test_increment_version() {
        let store = test_store();
        let entity = Entity::new(EntityType::new("Article"));
        EntityRepository::save(&store, &entity).await.unwrap();

        EntityRepository::increment_version(&store, entity.id).await.unwrap();
        let loaded = EntityRepository::get(&store, entity.id).await.unwrap().unwrap();
        assert_eq!(loaded.version, 2);

        EntityRepository::increment_version(&store, entity.id).await.unwrap();
        let loaded = EntityRepository::get(&store, entity.id).await.unwrap().unwrap();
        assert_eq!(loaded.version, 3);

        let history = EntityRepository::get_version_history(&store, entity.id).await.unwrap();
        assert_eq!(history.len(), 2);
        assert_eq!(history[0].version, 2); // Most recent first
        assert_eq!(history[1].version, 1);
    }

    #[tokio::test]
    async fn test_find_by_component_type() {
        let store = test_store();
        let entity = Entity::new(EntityType::new("Article"));
        EntityRepository::save(&store, &entity).await.unwrap();

        let comp = Component::new(entity.id, ComponentType::Timeline, serde_json::json!({"created_at": "2026-01-01"}));
        ComponentRepository::save(&store, &comp).await.unwrap();

        let found = EntityRepository::find_by_component_type(&store, "Timeline").await.unwrap();
        assert_eq!(found.len(), 1);
        assert_eq!(found[0].id, entity.id);

        let not_found = EntityRepository::find_by_component_type(&store, "Embedding").await.unwrap();
        assert!(not_found.is_empty());
    }

    #[tokio::test]
    async fn test_find_by_tag() {
        let store = test_store();
        let entity = Entity::new(EntityType::new("Article"));
        EntityRepository::save(&store, &entity).await.unwrap();

        let comp = Component::new(entity.id, ComponentType::Tags, serde_json::json!(["rust", "testing"]));
        ComponentRepository::save(&store, &comp).await.unwrap();

        let found = EntityRepository::find_by_tag(&store, "rust").await.unwrap();
        assert_eq!(found.len(), 1);

        let not_found = EntityRepository::find_by_tag(&store, "python").await.unwrap();
        assert!(not_found.is_empty());
    }

    #[tokio::test]
    async fn test_relationship_update() {
        let store = test_store();
        let entity1 = Entity::new(EntityType::new("Article"));
        let entity2 = Entity::new(EntityType::new("Concept"));
        EntityRepository::save(&store, &entity1).await.unwrap();
        EntityRepository::save(&store, &entity2).await.unwrap();

        let mut rel = Relationship::new(entity1.id, entity2.id, RelationshipType::References);
        RelationshipRepository::save(&store, &rel).await.unwrap();

        rel.is_active = false;
        RelationshipRepository::update(&store, &rel).await.unwrap();

        let updated = RelationshipRepository::get(&store, rel.id).await.unwrap().unwrap();
        assert!(!updated.is_active);
    }

    #[tokio::test]
    async fn test_transactional_write() {
        let store = test_store();
        let entity = Entity::new(EntityType::new("Article"));
        let components = vec![
            Component::new(entity.id, ComponentType::Title, serde_json::json!("Transactional Test")),
            Component::new(entity.id, ComponentType::Content, serde_json::json!("Body")),
        ];
        let event = Event {
            id: Uuid::new_v4(),
            event_type: EventType::EntityCreated,
            entity_id: entity.id,
            timestamp: chrono::Utc::now(),
            data: serde_json::json!({}),
        };

        store.save_entity_with_components(&entity, &components, &event).await.unwrap();

        let loaded = EntityRepository::get(&store, entity.id).await.unwrap().unwrap();
        assert_eq!(loaded.id, entity.id);

        let comps = ComponentRepository::get(&store, entity.id).await.unwrap();
        assert_eq!(comps.len(), 2);

        let events = store.list_by_entity(entity.id).await.unwrap();
        assert_eq!(events.len(), 1);
        assert_eq!(events[0].event_type, EventType::EntityCreated);
    }
}
