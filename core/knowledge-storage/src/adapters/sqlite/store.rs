use knowledge_core::features::component::{Component, ComponentType};
use knowledge_core::features::entity::Entity;
use knowledge_core::features::relationship::Relationship;
use knowledge_core::ports::{Collection, Event, StorageError};
use rusqlite::Connection;
use std::sync::Mutex;

pub(crate) const ENTITY_COLS: &str = "id, entity_type, is_active, created_at, updated_at, version";

pub struct SqliteStore {
    pub(crate) conn: Mutex<Connection>,
}

impl SqliteStore {
    pub fn new(path: &str) -> Result<Self, StorageError> {
        let conn = Connection::open(path).map_err(|e| StorageError::Internal(e.to_string()))?;

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
                snapshot TEXT,
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

    pub(crate) fn parse_entity(row: &rusqlite::Row) -> Result<Entity, rusqlite::Error> {
        Ok(Entity {
            id: uuid::Uuid::parse_str(&row.get::<_, String>(0)?).unwrap(),
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

    pub(crate) fn parse_relationship(row: &rusqlite::Row) -> Result<Relationship, rusqlite::Error> {
        Ok(Relationship {
            id: uuid::Uuid::parse_str(&row.get::<_, String>(0)?).unwrap(),
            source_id: uuid::Uuid::parse_str(&row.get::<_, String>(1)?).unwrap(),
            target_id: uuid::Uuid::parse_str(&row.get::<_, String>(2)?).unwrap(),
            relationship_type: serde_json::from_str(&row.get::<_, String>(3)?).unwrap(),
            is_active: row.get::<_, i32>(4)? != 0,
            created_at: chrono::DateTime::parse_from_rfc3339(&row.get::<_, String>(5)?)
                .unwrap()
                .with_timezone(&chrono::Utc),
        })
    }

    pub(crate) fn parse_component(row: &rusqlite::Row) -> Result<Component, rusqlite::Error> {
        Ok(Component {
            id: uuid::Uuid::parse_str(&row.get::<_, String>(0)?).unwrap(),
            entity_id: uuid::Uuid::parse_str(&row.get::<_, String>(1)?).unwrap(),
            component_type: serde_json::from_str(&row.get::<_, String>(2)?).unwrap(),
            data: serde_json::from_str(&row.get::<_, String>(3)?).unwrap(),
            created_at: chrono::DateTime::parse_from_rfc3339(&row.get::<_, String>(4)?)
                .unwrap()
                .with_timezone(&chrono::Utc),
            version: row.get(5)?,
        })
    }

    pub(crate) fn parse_event(row: &rusqlite::Row) -> Result<Event, rusqlite::Error> {
        Ok(Event {
            id: uuid::Uuid::parse_str(&row.get::<_, String>(0)?).unwrap(),
            event_type: serde_json::from_str(&row.get::<_, String>(1)?).unwrap(),
            entity_id: uuid::Uuid::parse_str(&row.get::<_, String>(2)?).unwrap(),
            timestamp: chrono::DateTime::parse_from_rfc3339(&row.get::<_, String>(3)?)
                .unwrap()
                .with_timezone(&chrono::Utc),
            data: serde_json::from_str(&row.get::<_, String>(4)?).unwrap(),
        })
    }

    pub(crate) fn fetch_entity_signals(
        conn: &Connection,
        entity_id: uuid::Uuid,
    ) -> Result<crate::fuzzy::EntitySignals, StorageError> {
        let mut signals = crate::fuzzy::EntitySignals::default();
        let mut stmt = conn
            .prepare(
                "SELECT c.data FROM components c WHERE c.entity_id = ?1 AND c.component_type = ?2",
            )
            .map_err(|e| StorageError::Internal(e.to_string()))?;

        let language_ct = Self::component_type_str(&ComponentType::Language);
        let language_json: Result<String, _> = stmt.query_row(
            rusqlite::params![entity_id.to_string(), language_ct],
            |row| row.get(0),
        );
        if let Ok(json) = language_json {
            if let Ok(lang) = serde_json::from_str::<String>(&json) {
                signals.language = Some(lang);
            }
        }

        let binary_ct = Self::component_type_str(&ComponentType::BinaryContent);
        let binary_json: Result<String, _> = stmt
            .query_row(rusqlite::params![entity_id.to_string(), binary_ct], |row| {
                row.get(0)
            });
        if let Ok(json) = binary_json {
            if let Ok(data) = serde_json::from_str::<serde_json::Value>(&json) {
                if let Some(size) = data.get("size").and_then(|v| v.as_u64()) {
                    signals.file_size = Some(size);
                }
            }
        }

        let timeline_ct = Self::component_type_str(&ComponentType::Timeline);
        let timeline_json: Result<String, _> = stmt.query_row(
            rusqlite::params![entity_id.to_string(), timeline_ct],
            |row| row.get(0),
        );
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

    pub(crate) fn component_type_str(ct: &ComponentType) -> String {
        serde_json::to_string(ct).unwrap()
    }

    pub(crate) fn entity_type_str(entity: &Entity) -> String {
        serde_json::to_string(&entity.entity_type).unwrap()
    }

    pub(crate) fn parse_collection(row: &rusqlite::Row) -> Result<Collection, rusqlite::Error> {
        Ok(Collection {
            id: uuid::Uuid::parse_str(&row.get::<_, String>(0)?).unwrap(),
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
