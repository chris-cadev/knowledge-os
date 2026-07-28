use async_trait::async_trait;
use knowledge_core::features::component::{Component, ComponentType};
use knowledge_core::features::entity::Entity;
use knowledge_core::features::relationship::Relationship;
use knowledge_core::ports::{
    Collection, CollectionRepository, ComponentRepository, EntityRepository, EntityResolver,
    EntityVersion, Event, EventLog, MergeAuditEntry, RelationshipRepository, ResolutionCandidate,
    SearchIndex, SearchQuery, SearchResult, StorageError, TransactionalWrite, TraversalConfig,
    TraversalError, TraversalPort, TraversalQuery, TraversalResult,
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

        // Enable foreign key enforcement (required for CASCADE DELETE).
        conn.execute_batch("PRAGMA foreign_keys = ON")
            .map_err(|e| StorageError::Internal(e.to_string()))?;

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
            );

            CREATE TABLE IF NOT EXISTS collections (
                id TEXT PRIMARY KEY,
                name TEXT NOT NULL,
                description TEXT,
                created_at TEXT NOT NULL,
                updated_at TEXT NOT NULL
            );

            CREATE TABLE IF NOT EXISTS collection_members (
                collection_id TEXT NOT NULL,
                entity_id TEXT NOT NULL,
                added_at TEXT NOT NULL,
                PRIMARY KEY (collection_id, entity_id),
                FOREIGN KEY (collection_id) REFERENCES collections(id) ON DELETE CASCADE,
                FOREIGN KEY (entity_id) REFERENCES entities(id)
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

    /// Fetch metadata signals (language, file_size, creation_date, page_count) for an entity
    fn fetch_entity_signals(
        conn: &Connection,
        entity_id: Uuid,
    ) -> Result<crate::fuzzy::EntitySignals, StorageError> {
        let mut signals = crate::fuzzy::EntitySignals::default();
        let mut stmt = conn
            .prepare(
                "SELECT c.data FROM components c WHERE c.entity_id = ?1 AND c.component_type = ?2",
            )
            .map_err(|e| StorageError::Internal(e.to_string()))?;

        // Language component
        let language_ct = Self::component_type_str(&ComponentType::Language);
        let language_json: Result<String, _> = stmt
            .query_row(params![entity_id.to_string(), language_ct], |row| {
                row.get(0)
            });
        if let Ok(json) = language_json {
            if let Ok(lang) = serde_json::from_str::<String>(&json) {
                signals.language = Some(lang);
            }
        }

        // BinaryContent component (for file_size)
        let binary_ct = Self::component_type_str(&ComponentType::BinaryContent);
        let binary_json: Result<String, _> =
            stmt.query_row(params![entity_id.to_string(), binary_ct], |row| row.get(0));
        if let Ok(json) = binary_json {
            if let Ok(data) = serde_json::from_str::<serde_json::Value>(&json) {
                if let Some(size) = data.get("size").and_then(|v| v.as_u64()) {
                    signals.file_size = Some(size);
                }
            }
        }

        // Timeline component (for creation_date)
        let timeline_ct = Self::component_type_str(&ComponentType::Timeline);
        let timeline_json: Result<String, _> = stmt
            .query_row(params![entity_id.to_string(), timeline_ct], |row| {
                row.get(0)
            });
        if let Ok(json) = timeline_json {
            if let Ok(data) = serde_json::from_str::<serde_json::Value>(&json) {
                if let Some(date) = data.get("creation_date").and_then(|v| v.as_str()) {
                    signals.creation_date = Some(date.to_string());
                }
                if let Some(pages) = data.get("page_count").and_then(|v| v.as_u64()) {
                    signals.page_count = Some(pages as u32);
                }
            }
        }

        Ok(signals)
    }

    fn component_type_str(ct: &ComponentType) -> String {
        serde_json::to_string(ct).unwrap()
    }

    fn entity_type_str(entity: &Entity) -> String {
        serde_json::to_string(&entity.entity_type).unwrap()
    }

    fn parse_collection(row: &rusqlite::Row) -> Result<Collection, rusqlite::Error> {
        Ok(Collection {
            id: Uuid::parse_str(&row.get::<_, String>(0)?).unwrap(),
            name: row.get(1)?,
            description: row.get(2)?,
            created_at: chrono::DateTime::parse_from_rfc3339(&row.get::<_, String>(3)?)
                .unwrap()
                .with_timezone(&chrono::Utc),
            updated_at: chrono::DateTime::parse_from_rfc3339(&row.get::<_, String>(4)?)
                .unwrap()
                .with_timezone(&chrono::Utc),
        })
    }
}

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
            .query_row(params![id.to_string()], Self::parse_entity)
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
        let conn = self
            .conn
            .lock()
            .map_err(|e| StorageError::Internal(e.to_string()))?;
        conn.execute(
            "DELETE FROM components WHERE entity_id = ?1",
            params![id.to_string()],
        )
        .map_err(|e| StorageError::Internal(e.to_string()))?;
        conn.execute(
            "DELETE FROM relationships WHERE source_id = ?1 OR target_id = ?1",
            params![id.to_string()],
        )
        .map_err(|e| StorageError::Internal(e.to_string()))?;
        conn.execute(
            "DELETE FROM entities WHERE id = ?1",
            params![id.to_string()],
        )
        .map_err(|e| StorageError::Internal(e.to_string()))?;
        conn.execute(
            "DELETE FROM entities_fts WHERE entity_id = ?1",
            params![id.to_string()],
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
            .query_map(params![quoted_type], Self::parse_entity)
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
                params![Self::component_type_str(&ComponentType::Title), title_json],
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
        // Snapshot current state before incrementing
        let mut stmt = conn
            .prepare(&format!(
                "SELECT {} FROM entities WHERE id = ?1",
                ENTITY_COLS
            ))
            .map_err(|e| StorageError::Internal(e.to_string()))?;
        let entity = stmt
            .query_row(params![id.to_string()], Self::parse_entity)
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
            .query_map(params![quoted_type], Self::parse_entity)
            .map_err(|e| StorageError::Internal(e.to_string()))?;
        rows.map(|r| r.map_err(|e| StorageError::Internal(e.to_string())))
            .collect()
    }

    async fn find_by_tag(&self, tag: &str) -> Result<Vec<Entity>, StorageError> {
        let conn = self
            .conn
            .lock()
            .map_err(|e| StorageError::Internal(e.to_string()))?;
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
        let rows = stmt
            .query_map(
                params![Self::component_type_str(&ComponentType::Tags), like_pattern],
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
            .query_map(params![entity_id.to_string()], |row| {
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
        rows.map(|r| r.map_err(|e| StorageError::Internal(e.to_string())))
            .collect()
    }
}

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
        stmt.query_row(params![id.to_string()], Self::parse_relationship)
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
        let conn = self
            .conn
            .lock()
            .map_err(|e| StorageError::Internal(e.to_string()))?;
        conn.execute(
            "DELETE FROM relationships WHERE id = ?1",
            params![id.to_string()],
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
            .query_map(params![source_id.to_string()], Self::parse_relationship)
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
            .query_map(params![target_id.to_string()], Self::parse_relationship)
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
            params![source_id.to_string(), target_id.to_string()],
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
            .query_map(params![quoted_type], Self::parse_relationship)
            .map_err(|e| StorageError::Internal(e.to_string()))?;
        rows.map(|r| r.map_err(|e| StorageError::Internal(e.to_string())))
            .collect()
    }
}

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
            .query_map(params![entity_id.to_string()], Self::parse_component)
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
        let conn = self
            .conn
            .lock()
            .map_err(|e| StorageError::Internal(e.to_string()))?;
        conn.execute(
            "DELETE FROM components WHERE id = ?1",
            params![id.to_string()],
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
                params![entity_id.to_string(), quoted_type],
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
            params![data_str, id.to_string()],
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
            .query_map(params![quoted_type, like_pattern], Self::parse_component)
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
            params![entity_id.to_string()],
        )
        .map_err(|e| StorageError::Internal(e.to_string()))?;
        Ok(())
    }
}

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
            params![entity.id.to_string()],
        )
        .map_err(|e| StorageError::Internal(e.to_string()))?;
        conn.execute(
            "INSERT INTO entities_fts (entity_id, title, content, tags) VALUES (?1, ?2, ?3, ?4)",
            params![entity.id.to_string(), title, content, tags],
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
            params![entity_id.to_string()],
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
                .query_map(params![fts_query], |row| {
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
                    match estmt.query_row(params![id_str], Self::parse_entity) {
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
                        if let Ok(ftags) =
                            tag_stmt.query_row(params![id_str], |row| row.get::<_, String>(0))
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
                params![entity.id.to_string(), title, content, tags],
            ).map_err(|e| StorageError::Internal(e.to_string()))?;
        }

        Ok(())
    }
}

#[async_trait]
impl EventLog for SqliteStore {
    async fn append(&self, event: &Event) -> Result<(), StorageError> {
        let conn = self
            .conn
            .lock()
            .map_err(|e| StorageError::Internal(e.to_string()))?;
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
        let conn = self
            .conn
            .lock()
            .map_err(|e| StorageError::Internal(e.to_string()))?;
        let mut stmt = conn
            .prepare("SELECT id, event_type, entity_id, timestamp, data FROM events WHERE entity_id = ?1 ORDER BY timestamp DESC")
            .map_err(|e| StorageError::Internal(e.to_string()))?;
        let rows = stmt
            .query_map(params![entity_id.to_string()], Self::parse_event)
            .map_err(|e| StorageError::Internal(e.to_string()))?;
        rows.map(|r| r.map_err(|e| StorageError::Internal(e.to_string())))
            .collect()
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
        let conn = self
            .conn
            .lock()
            .map_err(|e| StorageError::Internal(e.to_string()))?;

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
        let conn = self
            .conn
            .lock()
            .map_err(|e| StorageError::Internal(e.to_string()))?;

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
            )
            .map_err(|e| StorageError::Internal(e.to_string()))?;

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

        let entities: Vec<Entity> = stmt
            .query_map(
                params![entity_type_json, entity.id.to_string()],
                Self::parse_entity,
            )
            .map_err(|e| StorageError::Internal(e.to_string()))?
            .filter_map(|r| r.ok())
            .collect();

        // Get titles for each entity
        let mut title_stmt = conn
            .prepare(
                "SELECT c.data FROM components c WHERE c.entity_id = ?1 AND c.component_type = ?2",
            )
            .map_err(|e| StorageError::Internal(e.to_string()))?;

        // Get content for each entity (for ContentMatch strategy)
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
            let title_json: Result<String, _> =
                title_stmt.query_row(params![e.id.to_string(), title_ct], |row| row.get(0));

            if let Ok(title_json) = title_json {
                if let Ok(t) = serde_json::from_str::<String>(&title_json) {
                    // Also fetch content for this entity
                    let content_json: Result<String, _> = content_stmt
                        .query_row(params![e.id.to_string(), content_ct], |row| row.get(0));
                    let c = content_json
                        .ok()
                        .and_then(|json| serde_json::from_str::<String>(&json).ok());

                    // Fetch metadata components for composite scoring
                    let signals = Self::fetch_entity_signals(&conn, e.id)?;

                    entity_data.push((e.clone(), t, c, signals));
                }
            }
        }

        // Build incoming entity signals
        let incoming_signals = Self::fetch_entity_signals(&conn, entity.id)?;

        // Use composite resolver for candidate matching
        let resolver = crate::fuzzy::FuzzyEntityResolver::new();
        let composite_candidates = resolver.find_candidates_composite(
            entity,
            title,
            content,
            &incoming_signals,
            &entity_data,
        );

        // Convert CompositeCandidate to ResolutionCandidate
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
                params![canonical_id.to_string(), duplicate_id.to_string()],
            )
            .map_err(|e| StorageError::Internal(e.to_string()))?;

            conn.execute(
                "UPDATE relationships SET target_id = ?1 WHERE target_id = ?2",
                params![canonical_id.to_string(), duplicate_id.to_string()],
            )
            .map_err(|e| StorageError::Internal(e.to_string()))?;

            conn.execute(
                "DELETE FROM relationships WHERE source_id = ?1 AND target_id = ?1",
                params![canonical_id.to_string()],
            )
            .map_err(|e| StorageError::Internal(e.to_string()))?;

            conn.execute(
                "UPDATE components SET entity_id = ?1 WHERE entity_id = ?2",
                params![canonical_id.to_string(), duplicate_id.to_string()],
            )
            .map_err(|e| StorageError::Internal(e.to_string()))?;

            conn.execute(
                "DELETE FROM entities_fts WHERE entity_id = ?1",
                params![duplicate_id.to_string()],
            )
            .map_err(|e| StorageError::Internal(e.to_string()))?;

            conn.execute(
                "DELETE FROM entity_versions WHERE entity_id = ?1",
                params![duplicate_id.to_string()],
            )
            .map_err(|e| StorageError::Internal(e.to_string()))?;

            conn.execute(
                "DELETE FROM events WHERE entity_id = ?1",
                params![duplicate_id.to_string()],
            )
            .map_err(|e| StorageError::Internal(e.to_string()))?;

            conn.execute(
                "DELETE FROM entities WHERE id = ?1",
                params![duplicate_id.to_string()],
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
        let conn = self
            .conn
            .lock()
            .map_err(|e| StorageError::Internal(e.to_string()))?;

        // Get the merge entry with snapshot
        let entry = conn
            .query_row(
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
            // 1. Remove target's transferred components (they now point to source after merge)
            conn.execute(
                "DELETE FROM components WHERE entity_id = ?1",
                params![target_id.to_string()],
            )
            .map_err(|e| StorageError::Internal(e.to_string()))?;

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
                        params![rel_id, target_id.to_string(), target_ref, rel_type, is_active as i32, created_at],
                    ).map_err(|e| StorageError::Internal(e.to_string()))?;
                }
            }

            // 4. Recreate source entity row from snapshot
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
            .query_map(params![entity_id.to_string()], |row| {
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

        let entries: Vec<MergeAuditEntry> = entries.filter_map(|r| r.ok()).collect();

        Ok(entries)
    }
}

#[async_trait]
impl CollectionRepository for SqliteStore {
    async fn create(&self, collection: Collection) -> Result<Collection, StorageError> {
        let conn = self
            .conn
            .lock()
            .map_err(|e| StorageError::Internal(e.to_string()))?;
        conn.execute(
            "INSERT INTO collections (id, name, description, created_at, updated_at) VALUES (?1, ?2, ?3, ?4, ?5)",
            params![
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
        stmt.query_row(params![id.to_string()], Self::parse_collection)
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
                params![
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
        // CASCADE DELETE on collection_members handles membership cleanup.
        conn.execute(
            "DELETE FROM collections WHERE id = ?1",
            params![id.to_string()],
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
                params![
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
            params![collection_id.to_string(), entity_id.to_string()],
        )
        .map_err(|e| StorageError::Internal(e.to_string()))?;
        Ok(())
    }

    async fn get_members(&self, collection_id: Uuid) -> Result<Vec<Entity>, StorageError> {
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
            .query_map(params![collection_id.to_string()], Self::parse_entity)
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
            .query_map(params![entity_id.to_string()], Self::parse_collection)
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
                params![collection_id.to_string(), entity_id.to_string()],
                |row| row.get(0),
            )
            .map_err(|e| StorageError::Internal(e.to_string()))?;
        Ok(count > 0)
    }
}

#[async_trait]
impl TraversalPort for SqliteStore {
    async fn traverse(
        &self,
        query: &TraversalQuery,
        config: &TraversalConfig,
    ) -> Result<Vec<TraversalResult>, TraversalError> {
        let start = EntityRepository::get(self, query.start_id)
            .await?
            .ok_or(TraversalError::StartNotFound(query.start_id))?;

        // Verify start entity is active
        if !start.is_active {
            return Err(TraversalError::StartNotFound(query.start_id));
        }

        let max_depth = query.max_depth.unwrap_or(config.default_max_depth);
        let max_results = query.max_results.unwrap_or(config.default_max_results);

        let conn = self
            .conn
            .lock()
            .map_err(|e| StorageError::Internal(e.to_string()))?;

        let (reachable, direction_label) = match query.direction {
            knowledge_core::ports::TraversalDirection::Outgoing => {
                let reachable = Self::traverse_outgoing(
                    &conn,
                    query.start_id,
                    max_depth,
                    query.relationship_type.as_ref(),
                    query.entity_type_filter.as_ref(),
                )?;
                (reachable, "outgoing")
            }
            knowledge_core::ports::TraversalDirection::Incoming => {
                let reachable = Self::traverse_incoming(
                    &conn,
                    query.start_id,
                    max_depth,
                    query.relationship_type.as_ref(),
                    query.entity_type_filter.as_ref(),
                )?;
                (reachable, "incoming")
            }
            knowledge_core::ports::TraversalDirection::Both => {
                let reachable = Self::traverse_both(
                    &conn,
                    query.start_id,
                    max_depth,
                    query.relationship_type.as_ref(),
                    query.entity_type_filter.as_ref(),
                )?;
                (reachable, "both")
            }
        };

        // Filter out start entity and apply result limit
        let limited: Vec<_> = reachable
            .into_iter()
            .filter(|(id, _, _)| *id != query.start_id)
            .take(max_results)
            .collect();

        // Build results: reconstruct paths and edges via BFS on relationship graph
        let mut results = Vec::new();
        for (node_id, depth, path_str) in &limited {
            let path: Vec<Uuid> = path_str
                .split(',')
                .filter(|s| !s.is_empty())
                .map(|s| Uuid::parse_str(s).unwrap())
                .collect();
            let edges = Self::reconstruct_edges(&conn, &path, *node_id, direction_label)?;
            results.push(TraversalResult {
                path,
                edges,
                depth: *depth,
            });
        }

        Ok(results)
    }
}

impl SqliteStore {
    /// Run a recursive CTE for outgoing traversal.
    ///
    /// Returns `(entity_id, depth)` pairs for all reachable entities.
    fn traverse_outgoing(
        conn: &Connection,
        start_id: Uuid,
        max_depth: u32,
        rel_type: Option<&knowledge_core::features::relationship::RelationshipType>,
        entity_type: Option<&knowledge_core::features::entity::EntityType>,
    ) -> Result<Vec<(Uuid, u32, String)>, StorageError> {
        let rel_type_json = rel_type
            .map(|rt| serde_json::to_string(rt).unwrap())
            .map(|s| format!("AND r.relationship_type = '{}'", s.replace('\'', "''")))
            .unwrap_or_default();

        // Entity types are stored as JSON strings (e.g., "Article" not Article)
        let entity_type_json = entity_type
            .map(|et| serde_json::to_string(et).unwrap())
            .map(|s| format!("AND e.entity_type = '{}'", s.replace('\'', "''")))
            .unwrap_or_default();

        let sql = format!(
            "WITH RECURSIVE traversal(id, depth, path) AS (
                SELECT se.id, 0, se.id
                FROM entities se
                WHERE se.id = ?1 AND se.is_active = 1
                UNION
                SELECT r.target_id, t.depth + 1, t.path || ',' || r.target_id
                FROM relationships r
                JOIN traversal t ON r.source_id = t.id
                JOIN entities e ON r.target_id = e.id
                WHERE t.depth < ?2
                  AND r.is_active = 1
                  AND e.is_active = 1
                  AND (',' || t.path || ',') NOT LIKE ('%,' || e.id || ',%')
                  {rel_filter} {entity_filter}
            )
            SELECT id, depth, path FROM (
                SELECT id, depth, path, ROW_NUMBER() OVER (PARTITION BY id ORDER BY depth, length(path)) AS rn
                FROM traversal
            ) WHERE rn = 1 ORDER BY depth",
            rel_filter = rel_type_json,
            entity_filter = entity_type_json,
        );

        let mut stmt = conn
            .prepare(&sql)
            .map_err(|e| StorageError::Internal(e.to_string()))?;
        let rows = stmt
            .query_map(params![start_id.to_string(), max_depth], |row| {
                let id = Uuid::parse_str(&row.get::<_, String>(0)?).unwrap();
                let depth: u32 = row.get(1)?;
                let path: String = row.get(2)?;
                Ok((id, depth, path))
            })
            .map_err(|e| StorageError::Internal(e.to_string()))?;
        Ok(rows.filter_map(|r| r.ok()).collect())
    }

    /// Run a recursive CTE for incoming traversal (reversed edges).
    ///
    /// Returns `(entity_id, depth)` pairs for all reachable entities.
    fn traverse_incoming(
        conn: &Connection,
        start_id: Uuid,
        max_depth: u32,
        rel_type: Option<&knowledge_core::features::relationship::RelationshipType>,
        entity_type: Option<&knowledge_core::features::entity::EntityType>,
    ) -> Result<Vec<(Uuid, u32, String)>, StorageError> {
        let rel_type_json = rel_type
            .map(|rt| serde_json::to_string(rt).unwrap())
            .map(|s| format!("AND r.relationship_type = '{}'", s.replace('\'', "''")))
            .unwrap_or_default();

        // Entity types are stored as JSON strings (e.g., "Article" not Article)
        let entity_type_json = entity_type
            .map(|et| serde_json::to_string(et).unwrap())
            .map(|s| format!("AND e.entity_type = '{}'", s.replace('\'', "''")))
            .unwrap_or_default();

        let sql = format!(
            "WITH RECURSIVE traversal(id, depth, path) AS (
                SELECT se.id, 0, se.id
                FROM entities se
                WHERE se.id = ?1 AND se.is_active = 1
                UNION
                SELECT r.source_id, t.depth + 1, t.path || ',' || r.source_id
                FROM relationships r
                JOIN traversal t ON r.target_id = t.id
                JOIN entities e ON r.source_id = e.id
                WHERE t.depth < ?2
                  AND r.is_active = 1
                  AND e.is_active = 1
                  AND (',' || t.path || ',') NOT LIKE ('%,' || e.id || ',%')
                  {rel_filter} {entity_filter}
            )
            SELECT id, depth, path FROM (
                SELECT id, depth, path, ROW_NUMBER() OVER (PARTITION BY id ORDER BY depth, length(path)) AS rn
                FROM traversal
            ) WHERE rn = 1 ORDER BY depth",
            rel_filter = rel_type_json,
            entity_filter = entity_type_json,
        );

        let mut stmt = conn
            .prepare(&sql)
            .map_err(|e| StorageError::Internal(e.to_string()))?;
        let rows = stmt
            .query_map(params![start_id.to_string(), max_depth], |row| {
                let id = Uuid::parse_str(&row.get::<_, String>(0)?).unwrap();
                let depth: u32 = row.get(1)?;
                let path: String = row.get(2)?;
                Ok((id, depth, path))
            })
            .map_err(|e| StorageError::Internal(e.to_string()))?;
        Ok(rows.filter_map(|r| r.ok()).collect())
    }

    /// Run a recursive CTE for bidirectional traversal (UNION of outgoing + incoming).
    ///
    /// Returns `(entity_id, depth)` pairs for all reachable entities.
    fn traverse_both(
        conn: &Connection,
        start_id: Uuid,
        max_depth: u32,
        rel_type: Option<&knowledge_core::features::relationship::RelationshipType>,
        entity_type: Option<&knowledge_core::features::entity::EntityType>,
    ) -> Result<Vec<(Uuid, u32, String)>, StorageError> {
        let rel_type_json = rel_type
            .map(|rt| serde_json::to_string(rt).unwrap())
            .map(|s| format!("AND r.relationship_type = '{}'", s.replace('\'', "''")))
            .unwrap_or_default();

        // Entity types are stored as JSON strings (e.g., "Article" not Article)
        let entity_type_json = entity_type
            .map(|et| serde_json::to_string(et).unwrap())
            .map(|s| format!("AND e.entity_type = '{}'", s.replace('\'', "''")))
            .unwrap_or_default();

        let sql = format!(
            "WITH RECURSIVE traversal(id, depth, path) AS (
                SELECT se.id, 0, se.id
                FROM entities se
                WHERE se.id = ?1 AND se.is_active = 1
                UNION
                SELECT r.target_id, t.depth + 1, t.path || ',' || r.target_id
                FROM relationships r
                JOIN traversal t ON r.source_id = t.id
                JOIN entities e ON r.target_id = e.id
                WHERE t.depth < ?2
                  AND r.is_active = 1
                  AND e.is_active = 1
                  AND (',' || t.path || ',') NOT LIKE ('%,' || e.id || ',%')
                  {rel_filter} {entity_filter}
                UNION
                SELECT r.source_id, t.depth + 1, t.path || ',' || r.source_id
                FROM relationships r
                JOIN traversal t ON r.target_id = t.id
                JOIN entities e ON r.source_id = e.id
                WHERE t.depth < ?2
                  AND r.is_active = 1
                  AND e.is_active = 1
                  AND (',' || t.path || ',') NOT LIKE ('%,' || e.id || ',%')
                  {rel_filter} {entity_filter}
            )
            SELECT id, depth, path FROM (
                SELECT id, depth, path, ROW_NUMBER() OVER (PARTITION BY id ORDER BY depth, length(path)) AS rn
                FROM traversal
            ) WHERE rn = 1 ORDER BY depth",
            rel_filter = rel_type_json,
            entity_filter = entity_type_json,
        );

        let mut stmt = conn
            .prepare(&sql)
            .map_err(|e| StorageError::Internal(e.to_string()))?;
        let rows = stmt
            .query_map(params![start_id.to_string(), max_depth], |row| {
                let id = Uuid::parse_str(&row.get::<_, String>(0)?).unwrap();
                let depth: u32 = row.get(1)?;
                let path: String = row.get(2)?;
                Ok((id, depth, path))
            })
            .map_err(|e| StorageError::Internal(e.to_string()))?;
        Ok(rows.filter_map(|r| r.ok()).collect())
    }

    /// Reconstruct the shortest path from `start_id` to `target_id` via BFS
    /// on the relationship graph.
    #[allow(dead_code)]
    fn reconstruct_path(
        conn: &Connection,
        start_id: Uuid,
        target_id: Uuid,
    ) -> Result<Vec<Uuid>, StorageError> {
        use std::collections::{HashMap, VecDeque};

        if start_id == target_id {
            return Ok(vec![start_id]);
        }

        // BFS from start to target through active relationships
        let mut visited: HashMap<Uuid, Uuid> = HashMap::new();
        let mut queue = VecDeque::new();
        queue.push_back(start_id);
        visited.insert(start_id, start_id);

        let mut stmt = conn
            .prepare("SELECT source_id, target_id FROM relationships WHERE is_active = 1")
            .map_err(|e| StorageError::Internal(e.to_string()))?;

        // Collect all edges for BFS
        let edges: Vec<(Uuid, Uuid)> = {
            let rows = stmt
                .query_map([], |row| {
                    let source = Uuid::parse_str(&row.get::<_, String>(0)?).unwrap();
                    let target = Uuid::parse_str(&row.get::<_, String>(1)?).unwrap();
                    Ok((source, target))
                })
                .map_err(|e| StorageError::Internal(e.to_string()))?;
            rows.filter_map(|r| r.ok()).collect()
        };

        // Build adjacency list
        let mut outgoing: HashMap<Uuid, Vec<Uuid>> = HashMap::new();
        let mut incoming: HashMap<Uuid, Vec<Uuid>> = HashMap::new();
        for (source, target) in &edges {
            outgoing.entry(*source).or_default().push(*target);
            incoming.entry(*target).or_default().push(*source);
        }

        while let Some(current) = queue.pop_front() {
            if current == target_id {
                // Reconstruct path
                let mut path = vec![target_id];
                let mut current = target_id;
                while let Some(&prev) = visited.get(&current) {
                    if prev == current {
                        break;
                    }
                    path.push(prev);
                    current = prev;
                }
                path.reverse();
                return Ok(path);
            }

            // Follow outgoing edges
            if let Some(targets) = outgoing.get(&current) {
                for &next in targets {
                    if let std::collections::hash_map::Entry::Vacant(e) = visited.entry(next) {
                        e.insert(current);
                        queue.push_back(next);
                    }
                }
            }

            // Follow incoming edges (for bidirectional support)
            if let Some(sources) = incoming.get(&current) {
                for &prev in sources {
                    if let std::collections::hash_map::Entry::Vacant(e) = visited.entry(prev) {
                        e.insert(current);
                        queue.push_back(prev);
                    }
                }
            }
        }

        // Target not reachable from start - return partial path
        Ok(vec![start_id, target_id])
    }

    /// Reconstruct the edges along the path from `start_id` to `target_id`.
    fn reconstruct_edges(
        conn: &Connection,
        path: &[Uuid],
        _target_id: Uuid,
        direction: &str,
    ) -> Result<Vec<knowledge_core::ports::TraversalEdge>, StorageError> {
        use std::collections::{HashMap, HashSet};

        if path.len() <= 1 {
            return Ok(Vec::new());
        }

        // Query all active relationships
        let mut stmt = conn
            .prepare(
                "SELECT source_id, target_id, relationship_type FROM relationships WHERE is_active = 1",
            )
            .map_err(|e| StorageError::Internal(e.to_string()))?;

        // Build adjacency maps keyed by (source, target)
        let mut edge_map: HashMap<(Uuid, Uuid), String> = HashMap::new();
        let outgoing_set: HashSet<(Uuid, Uuid)> = {
            let rows = stmt
                .query_map([], |row| {
                    let source = Uuid::parse_str(&row.get::<_, String>(0)?).unwrap();
                    let target = Uuid::parse_str(&row.get::<_, String>(1)?).unwrap();
                    let rel_type: String = row.get(2)?;
                    Ok((source, target, rel_type))
                })
                .map_err(|e| StorageError::Internal(e.to_string()))?;
            rows.filter_map(|r| r.ok())
                .map(|(s, t, rt)| {
                    edge_map.insert((s, t), rt);
                    (s, t)
                })
                .collect()
        };

        // Walk the path and find edges between consecutive nodes
        let mut edges = Vec::new();
        for window in path.windows(2) {
            let from = window[0];
            let to = window[1];

            // Try forward edge first (from -> to)
            if let Some(rel_type_str) = edge_map.get(&(from, to)) {
                let rel_type = serde_json::from_str::<
                    knowledge_core::features::relationship::RelationshipType,
                >(&format!("\"{}\"", rel_type_str))
                .unwrap_or(knowledge_core::features::relationship::RelationshipType::References);
                edges.push(knowledge_core::ports::TraversalEdge {
                    source_id: from,
                    target_id: to,
                    relationship_type: rel_type,
                });
            }
            // Try reverse edge (to -> from) if bidirectional
            else if direction == "both" && outgoing_set.contains(&(to, from)) {
                if let Some(rel_type_str) = edge_map.get(&(to, from)) {
                    let rel_type = serde_json::from_str::<
                        knowledge_core::features::relationship::RelationshipType,
                    >(&format!("\"{}\"", rel_type_str))
                    .unwrap_or(
                        knowledge_core::features::relationship::RelationshipType::References,
                    );
                    edges.push(knowledge_core::ports::TraversalEdge {
                        source_id: to,
                        target_id: from,
                        relationship_type: rel_type,
                    });
                }
            }
        }

        // If we didn't find edges for all path segments and this is the target node,
        // try to find the direct edge from start to this node
        if edges.is_empty() && path.len() == 2 {
            let from = path[0];
            let to = path[1];
            if let Some(rel_type_str) = edge_map.get(&(from, to)) {
                let rel_type = serde_json::from_str::<
                    knowledge_core::features::relationship::RelationshipType,
                >(&format!("\"{}\"", rel_type_str))
                .unwrap_or(knowledge_core::features::relationship::RelationshipType::References);
                edges.push(knowledge_core::ports::TraversalEdge {
                    source_id: from,
                    target_id: to,
                    relationship_type: rel_type,
                });
            } else if direction == "both" {
                if let Some(rel_type_str) = edge_map.get(&(to, from)) {
                    let rel_type = serde_json::from_str::<
                        knowledge_core::features::relationship::RelationshipType,
                    >(&format!("\"{}\"", rel_type_str))
                    .unwrap_or(
                        knowledge_core::features::relationship::RelationshipType::References,
                    );
                    edges.push(knowledge_core::ports::TraversalEdge {
                        source_id: to,
                        target_id: from,
                        relationship_type: rel_type,
                    });
                }
            }
        }

        Ok(edges)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use knowledge_core::features::component::ComponentType;
    use knowledge_core::features::entity::EntityType;
    use knowledge_core::features::relationship::RelationshipType;
    use knowledge_core::ports::{
        ComponentRepository, EntityRepository, EventLog, EventType, RelationshipRepository,
        SearchIndex,
    };

    fn test_store() -> SqliteStore {
        SqliteStore::new(":memory:").unwrap()
    }

    #[tokio::test]
    async fn test_entity_crud() {
        let store = test_store();
        let mut entity = Entity::new(EntityType::new("Article"));

        EntityRepository::save(&store, &entity).await.unwrap();
        let loaded = EntityRepository::get(&store, entity.id)
            .await
            .unwrap()
            .unwrap();
        assert_eq!(loaded.id, entity.id);
        assert_eq!(loaded.entity_type, EntityType::new("Article"));
        assert!(loaded.is_active);

        entity.touch();
        EntityRepository::save(&store, &entity).await.unwrap();
        let loaded = EntityRepository::get(&store, entity.id)
            .await
            .unwrap()
            .unwrap();
        assert_eq!(loaded.version, 2);

        let all = EntityRepository::list(&store).await.unwrap();
        assert_eq!(all.len(), 1);

        let articles = EntityRepository::find_by_type(&store, "Article")
            .await
            .unwrap();
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

        let component = Component::new(
            entity.id,
            ComponentType::Title,
            serde_json::json!("Test Title"),
        );
        ComponentRepository::save(&store, &component).await.unwrap();

        let components = ComponentRepository::get(&store, entity.id).await.unwrap();
        assert_eq!(components.len(), 1);
        assert_eq!(components[0].component_type, ComponentType::Title);

        let found = ComponentRepository::find_by_type(&store, entity.id, "Title")
            .await
            .unwrap();
        assert_eq!(found.len(), 1);

        ComponentRepository::update_data(&store, component.id, serde_json::json!("Updated Title"))
            .await
            .unwrap();
        let updated = ComponentRepository::get(&store, entity.id).await.unwrap();
        assert_eq!(updated[0].data, serde_json::json!("Updated Title"));
        assert_eq!(updated[0].version, 2);

        ComponentRepository::delete(&store, component.id)
            .await
            .unwrap();
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

        let rels = RelationshipRepository::by_source(&store, entity1.id)
            .await
            .unwrap();
        assert_eq!(rels.len(), 1);

        let rels = RelationshipRepository::by_target(&store, entity2.id)
            .await
            .unwrap();
        assert_eq!(rels.len(), 1);

        let found =
            RelationshipRepository::find_by_source_and_target(&store, entity1.id, entity2.id)
                .await
                .unwrap();
        assert!(found.is_some());

        let refs = RelationshipRepository::find_by_type(&store, "References")
            .await
            .unwrap();
        assert_eq!(refs.len(), 1);

        RelationshipRepository::delete(&store, rel.id)
            .await
            .unwrap();
        let rels = RelationshipRepository::by_source(&store, entity1.id)
            .await
            .unwrap();
        assert!(rels.is_empty());
    }

    #[tokio::test]
    async fn test_search_index() {
        let store = test_store();
        let entity = Entity::new(EntityType::new("Article"));
        let entity_id = entity.id;

        let components = vec![
            Component::new(
                entity_id,
                ComponentType::Title,
                serde_json::json!("Test Title"),
            ),
            Component::new(
                entity_id,
                ComponentType::Content,
                serde_json::json!("Some content here"),
            ),
            Component::new(
                entity_id,
                ComponentType::Tags,
                serde_json::json!(["rust", "test"]),
            ),
        ];

        store.index_entity(&entity, &components).await.unwrap();

        let results = store
            .search(&SearchQuery {
                query: "Test".to_string(),
                entity_type: None,
                tag: None,
            })
            .await
            .unwrap();
        assert_eq!(results.len(), 1);
        assert_eq!(results[0].entity_id, entity_id);
        assert!(results[0].score < 0.0);

        let results = store
            .search(&SearchQuery {
                query: "content".to_string(),
                entity_type: None,
                tag: Some("rust".to_string()),
            })
            .await
            .unwrap();
        assert_eq!(results.len(), 1);

        let results = store
            .search(&SearchQuery {
                query: "nonexistent".to_string(),
                entity_type: None,
                tag: None,
            })
            .await
            .unwrap();
        assert!(results.is_empty());
    }

    #[tokio::test]
    async fn test_search_rebuild() {
        let store = test_store();
        let entity = Entity::new(EntityType::new("Article"));

        let components = vec![
            Component::new(
                entity.id,
                ComponentType::Title,
                serde_json::json!("Test Title"),
            ),
            Component::new(
                entity.id,
                ComponentType::Content,
                serde_json::json!("Some content here"),
            ),
        ];

        store.index_entity(&entity, &components).await.unwrap();

        let results = store
            .search(&SearchQuery {
                query: "Test".to_string(),
                entity_type: None,
                tag: None,
            })
            .await
            .unwrap();
        assert_eq!(results.len(), 1);

        store
            .rebuild(&[(entity.clone(), components.clone())])
            .await
            .unwrap();

        let results = store
            .search(&SearchQuery {
                query: "Test".to_string(),
                entity_type: None,
                tag: None,
            })
            .await
            .unwrap();
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

        EntityRepository::increment_version(&store, entity.id)
            .await
            .unwrap();
        let loaded = EntityRepository::get(&store, entity.id)
            .await
            .unwrap()
            .unwrap();
        assert_eq!(loaded.version, 2);

        EntityRepository::increment_version(&store, entity.id)
            .await
            .unwrap();
        let loaded = EntityRepository::get(&store, entity.id)
            .await
            .unwrap()
            .unwrap();
        assert_eq!(loaded.version, 3);

        let history = EntityRepository::get_version_history(&store, entity.id)
            .await
            .unwrap();
        assert_eq!(history.len(), 2);
        assert_eq!(history[0].version, 2); // Most recent first
        assert_eq!(history[1].version, 1);
    }

    #[tokio::test]
    async fn test_find_by_component_type() {
        let store = test_store();
        let entity = Entity::new(EntityType::new("Article"));
        EntityRepository::save(&store, &entity).await.unwrap();

        let comp = Component::new(
            entity.id,
            ComponentType::Timeline,
            serde_json::json!({"created_at": "2026-01-01"}),
        );
        ComponentRepository::save(&store, &comp).await.unwrap();

        let found = EntityRepository::find_by_component_type(&store, "Timeline")
            .await
            .unwrap();
        assert_eq!(found.len(), 1);
        assert_eq!(found[0].id, entity.id);

        let not_found = EntityRepository::find_by_component_type(&store, "Embedding")
            .await
            .unwrap();
        assert!(not_found.is_empty());
    }

    #[tokio::test]
    async fn test_find_by_tag() {
        let store = test_store();
        let entity = Entity::new(EntityType::new("Article"));
        EntityRepository::save(&store, &entity).await.unwrap();

        let comp = Component::new(
            entity.id,
            ComponentType::Tags,
            serde_json::json!(["rust", "testing"]),
        );
        ComponentRepository::save(&store, &comp).await.unwrap();

        let found = EntityRepository::find_by_tag(&store, "rust").await.unwrap();
        assert_eq!(found.len(), 1);

        let not_found = EntityRepository::find_by_tag(&store, "python")
            .await
            .unwrap();
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

        let updated = RelationshipRepository::get(&store, rel.id)
            .await
            .unwrap()
            .unwrap();
        assert!(!updated.is_active);
    }

    #[tokio::test]
    async fn test_transactional_write() {
        let store = test_store();
        let entity = Entity::new(EntityType::new("Article"));
        let components = vec![
            Component::new(
                entity.id,
                ComponentType::Title,
                serde_json::json!("Transactional Test"),
            ),
            Component::new(entity.id, ComponentType::Content, serde_json::json!("Body")),
        ];
        let event = Event {
            id: Uuid::new_v4(),
            event_type: EventType::EntityCreated,
            entity_id: entity.id,
            timestamp: chrono::Utc::now(),
            data: serde_json::json!({}),
        };

        store
            .save_entity_with_components(&entity, &components, &event)
            .await
            .unwrap();

        let loaded = EntityRepository::get(&store, entity.id)
            .await
            .unwrap()
            .unwrap();
        assert_eq!(loaded.id, entity.id);

        let comps = ComponentRepository::get(&store, entity.id).await.unwrap();
        assert_eq!(comps.len(), 2);

        let events = store.list_by_entity(entity.id).await.unwrap();
        assert_eq!(events.len(), 1);
        assert_eq!(events[0].event_type, EventType::EntityCreated);
    }
}
