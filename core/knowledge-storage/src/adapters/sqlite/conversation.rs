use async_trait::async_trait;
use knowledge_core::ports::{
    CitationSource, ConversationDetail, ConversationRepository, ConversationSummary, MessageDetail,
    MessageRole, StorageError,
};
use rusqlite::OptionalExtension;
use uuid::Uuid;

use super::store::SqliteStore;
use super::ENTITY_COLS;

fn entity_type_json(t: &str) -> String {
    serde_json::to_string(t).unwrap()
}

fn component_type_json(ct: &str) -> String {
    serde_json::to_string(ct).unwrap()
}

fn relationship_type_json(rt: &str) -> String {
    serde_json::to_string(rt).unwrap()
}

fn title_from_data(data: Option<String>) -> String {
    data.and_then(|d| serde_json::from_str::<serde_json::Value>(&d).ok())
        .map(|v| {
            v.as_str()
                .or_else(|| v.get("name").and_then(|n| n.as_str()))
                .unwrap_or("Untitled")
                .to_string()
        })
        .unwrap_or_else(|| "Untitled".to_string())
}

fn extract_role_and_text(data: Option<String>) -> (MessageRole, String) {
    data.and_then(|d| {
        let v: serde_json::Value = serde_json::from_str(&d).ok()?;
        Some((
            v.get("role")
                .and_then(|r| r.as_str())
                .map(|s| match s {
                    "user" => MessageRole::User,
                    "assistant" => MessageRole::Assistant,
                    _ => MessageRole::System,
                })
                .unwrap_or(MessageRole::User),
            v.get("content")
                .or_else(|| v.get("text"))
                .and_then(|t| t.as_str())
                .unwrap_or("")
                .to_string(),
        ))
    })
    .unwrap_or((MessageRole::User, String::new()))
}

fn extract_entity_ids(data: Option<String>) -> Vec<Uuid> {
    data.and_then(|d| {
        let v: serde_json::Value = serde_json::from_str(&d).ok()?;
        v.get("refs")
            .or_else(|| v.get("entity_ids"))
            .and_then(|ids| {
                ids.as_array()?
                    .iter()
                    .map(|id| Uuid::parse_str(id.as_str()?).ok())
                    .collect::<Option<Vec<_>>>()
            })
    })
    .unwrap_or_default()
}

fn parse_rfc3339(s: &str) -> chrono::DateTime<chrono::Utc> {
    chrono::DateTime::parse_from_rfc3339(s)
        .unwrap()
        .with_timezone(&chrono::Utc)
}

#[async_trait]
impl ConversationRepository for SqliteStore {
    async fn list_conversations(&self) -> Result<Vec<ConversationSummary>, StorageError> {
        let conn = self
            .conn
            .lock()
            .map_err(|e| StorageError::Internal(e.to_string()))?;

        let conv_type = entity_type_json("Conversation");
        let has_msg_type = relationship_type_json("HasMessage");
        let title_ct = component_type_json("Title");
        let msg_content_ct = component_type_json("MessageContent");

        let mut stmt = conn
            .prepare(
                "SELECT
                    e.id, e.entity_type, e.is_active, e.created_at, e.updated_at, e.version,
                    (SELECT c.data FROM components c WHERE c.entity_id = e.id AND c.component_type = ?2 LIMIT 1) as title_data,
                    (SELECT COUNT(*) FROM relationships r WHERE r.source_id = e.id AND r.relationship_type = ?3 AND r.is_active = 1) as msg_count,
                    (SELECT MAX(m.created_at) FROM entities m
                     JOIN relationships r2 ON r2.target_id = m.id
                     WHERE r2.source_id = e.id AND r2.relationship_type = ?3 AND r2.is_active = 1 AND m.is_active = 1) as last_msg_at,
                    (SELECT mc.data FROM relationships r3
                     JOIN components mc ON mc.entity_id = r3.target_id AND mc.component_type = ?4
                     WHERE r3.source_id = e.id AND r3.relationship_type = ?3 AND r3.is_active = 1
                     ORDER BY mc.created_at DESC LIMIT 1) as last_msg_content
                FROM entities e
                WHERE e.entity_type = ?1 AND e.is_active = 1
                ORDER BY last_msg_at DESC",
            )
            .map_err(|e| StorageError::Internal(e.to_string()))?;

        let rows = stmt
            .query_map(
                rusqlite::params![conv_type, title_ct, has_msg_type, msg_content_ct],
                |row| {
                    let id = uuid::Uuid::parse_str(&row.get::<_, String>(0)?).unwrap();
                    let created_at: String = row.get(3)?;
                    let updated_at: String = row.get(4)?;
                    let title_data: Option<String> = row.get(6)?;
                    let msg_count: i64 = row.get(7)?;
                    let last_msg_at: Option<String> = row.get(8)?;
                    let last_msg_content: Option<String> = row.get(9)?;

                    let title = title_from_data(title_data);

                    let (last_message_preview, last_message_at) =
                        if let Some(content) = last_msg_content {
                            let preview = content
                                .chars()
                                .take(100)
                                .collect::<String>()
                                .replace('\n', " ");
                            let dt = last_msg_at.as_ref().map(|s| parse_rfc3339(s));
                            (Some(preview), dt)
                        } else {
                            (None, None)
                        };

                    Ok(ConversationSummary {
                        id,
                        title,
                        message_count: msg_count as u32,
                        last_message_preview,
                        last_message_at,
                        created_at: parse_rfc3339(&created_at),
                        updated_at: parse_rfc3339(&updated_at),
                    })
                },
            )
            .map_err(|e| StorageError::Internal(e.to_string()))?;

        rows.map(|r| r.map_err(|e| StorageError::Internal(e.to_string())))
            .collect()
    }

    async fn get_conversation(
        &self,
        conversation_id: Uuid,
    ) -> Result<Option<ConversationDetail>, StorageError> {
        let conn = self
            .conn
            .lock()
            .map_err(|e| StorageError::Internal(e.to_string()))?;

        let conv_type = entity_type_json("Conversation");
        let has_msg_type = relationship_type_json("HasMessage");
        let title_ct = component_type_json("Title");
        let msg_content_ct = component_type_json("MessageContent");
        let entity_refs_ct = component_type_json("EntityRefs");

        let mut conv_stmt = conn
            .prepare(&format!(
                "SELECT {} FROM entities WHERE id = ?1 AND entity_type = ?2 AND is_active = 1",
                ENTITY_COLS
            ))
            .map_err(|e| StorageError::Internal(e.to_string()))?;

        let conv_entity = conv_stmt
            .query_row(
                rusqlite::params![conversation_id.to_string(), conv_type],
                Self::parse_entity,
            )
            .optional()
            .map_err(|e| StorageError::Internal(e.to_string()))?;

        let conv_entity = match conv_entity {
            Some(e) => e,
            None => return Ok(None),
        };

        let title_data: Option<String> = conn
            .query_row(
                "SELECT data FROM components WHERE entity_id = ?1 AND component_type = ?2 LIMIT 1",
                rusqlite::params![conversation_id.to_string(), title_ct],
                |row| row.get(0),
            )
            .optional()
            .map_err(|e| StorageError::Internal(e.to_string()))?;

        let title = title_data
            .as_ref()
            .map(|_| title_from_data(title_data.clone()))
            .unwrap_or_else(|| "Untitled".to_string());

        let mut msg_stmt = conn
            .prepare(
                "SELECT e.id, e.entity_type, e.is_active, e.created_at, e.updated_at, e.version FROM entities e
                 JOIN relationships r ON r.target_id = e.id
                 WHERE r.source_id = ?1 AND r.relationship_type = ?2 AND r.is_active = 1 AND e.is_active = 1
                 ORDER BY e.created_at ASC",
            )
            .map_err(|e| StorageError::Internal(e.to_string()))?;

        let msg_rows = msg_stmt
            .query_map(
                rusqlite::params![conversation_id.to_string(), has_msg_type],
                Self::parse_entity,
            )
            .map_err(|e| StorageError::Internal(e.to_string()))?;

        let message_entities: Vec<_> = msg_rows
            .map(|r| r.map_err(|e| StorageError::Internal(e.to_string())))
            .collect::<Result<Vec<_>, _>>()?;

        let mut messages = Vec::new();
        for msg_entity in message_entities {
            let msg_id = msg_entity.id;

            let msg_content_data: Option<String> = conn
                .query_row(
                    "SELECT data FROM components WHERE entity_id = ?1 AND component_type = ?2 LIMIT 1",
                    rusqlite::params![msg_id.to_string(), msg_content_ct],
                    |row| row.get(0),
                )
                .optional()
                .map_err(|e| StorageError::Internal(e.to_string()))?;

            let (role, text) = extract_role_and_text(msg_content_data);

            let entity_refs_data: Option<String> = conn
                .query_row(
                    "SELECT data FROM components WHERE entity_id = ?1 AND component_type = ?2 LIMIT 1",
                    rusqlite::params![msg_id.to_string(), entity_refs_ct],
                    |row| row.get(0),
                )
                .optional()
                .map_err(|e| StorageError::Internal(e.to_string()))?;

            let entity_ids = extract_entity_ids(entity_refs_data);

            let mut citations = Vec::new();
            for (i, ref_id) in entity_ids.iter().enumerate() {
                let etype: Option<String> = conn
                    .query_row(
                        "SELECT entity_type FROM entities WHERE id = ?1 AND is_active = 1",
                        rusqlite::params![ref_id.to_string()],
                        |row| row.get(0),
                    )
                    .optional()
                    .map_err(|e| StorageError::Internal(e.to_string()))?;

                if let Some(ref etype_json) = etype {
                    let entity_type_str: String =
                        serde_json::from_str(etype_json).unwrap_or_default();
                    let title_data: Option<String> = conn
                        .query_row(
                            "SELECT data FROM components WHERE entity_id = ?1 AND component_type = ?2 LIMIT 1",
                            rusqlite::params![ref_id.to_string(), component_type_json("Title")],
                            |row| row.get(0),
                        )
                        .optional()
                        .map_err(|e| StorageError::Internal(e.to_string()))?;
                    let title = title_from_data(title_data);
                    let snippet_data: Option<String> = conn
                        .query_row(
                            "SELECT data FROM components WHERE entity_id = ?1 AND component_type = ?2 LIMIT 1",
                            rusqlite::params![ref_id.to_string(), component_type_json("Content")],
                            |row| row.get(0),
                        )
                        .optional()
                        .map_err(|e| StorageError::Internal(e.to_string()))?;
                    let snippet = snippet_data
                        .and_then(|s| serde_json::from_str::<String>(&s).ok())
                        .unwrap_or_default();
                    citations.push(CitationSource {
                        number: i + 1,
                        entity_id: *ref_id,
                        entity_type: entity_type_str,
                        title,
                        snippet: snippet.chars().take(200).collect(),
                    });
                }
            }

            messages.push(MessageDetail {
                id: msg_id,
                role,
                text,
                entity_refs: entity_ids,
                citations,
                created_at: msg_entity.created_at,
            });
        }

        Ok(Some(ConversationDetail {
            id: conversation_id,
            title,
            messages,
            created_at: conv_entity.created_at,
            updated_at: conv_entity.updated_at,
        }))
    }

    async fn rename_conversation(
        &self,
        conversation_id: Uuid,
        new_title: &str,
    ) -> Result<(), StorageError> {
        let conn = self
            .conn
            .lock()
            .map_err(|e| StorageError::Internal(e.to_string()))?;

        let title_ct = component_type_json("Title");

        let existing: Option<String> = conn
            .query_row(
                "SELECT id FROM components WHERE entity_id = ?1 AND component_type = ?2 LIMIT 1",
                rusqlite::params![conversation_id.to_string(), title_ct],
                |row| row.get(0),
            )
            .optional()
            .map_err(|e| StorageError::Internal(e.to_string()))?;

        let title_value = serde_json::to_string(&new_title).unwrap();

        if let Some(comp_id) = existing {
            conn.execute(
                "UPDATE components SET data = ?1, version = version + 1 WHERE id = ?2",
                rusqlite::params![title_value, comp_id],
            )
            .map_err(|e| StorageError::Internal(e.to_string()))?;
        } else {
            let comp_id = Uuid::new_v4();
            conn.execute(
                "INSERT INTO components (id, entity_id, component_type, data, created_at, version) VALUES (?1, ?2, ?3, ?4, ?5, 1)",
                rusqlite::params![
                    comp_id.to_string(),
                    conversation_id.to_string(),
                    title_ct,
                    title_value,
                    chrono::Utc::now().to_rfc3339(),
                ],
            )
            .map_err(|e| StorageError::Internal(e.to_string()))?;
        }

        conn.execute(
            "UPDATE entities SET updated_at = ?1 WHERE id = ?2",
            rusqlite::params![chrono::Utc::now().to_rfc3339(), conversation_id.to_string()],
        )
        .map_err(|e| StorageError::Internal(e.to_string()))?;

        Ok(())
    }

    async fn archive_conversation(&self, conversation_id: Uuid) -> Result<(), StorageError> {
        let conn = self
            .conn
            .lock()
            .map_err(|e| StorageError::Internal(e.to_string()))?;

        let has_msg_type = relationship_type_json("HasMessage");

        conn.execute(
            "UPDATE entities SET is_active = 0, updated_at = ?1 WHERE id = ?2",
            rusqlite::params![chrono::Utc::now().to_rfc3339(), conversation_id.to_string()],
        )
        .map_err(|e| StorageError::Internal(e.to_string()))?;

        let mut msg_stmt = conn
            .prepare("SELECT r.target_id FROM relationships r WHERE r.source_id = ?1 AND r.relationship_type = ?2 AND r.is_active = 1")
            .map_err(|e| StorageError::Internal(e.to_string()))?;

        let msg_ids: Vec<String> = msg_stmt
            .query_map(
                rusqlite::params![conversation_id.to_string(), has_msg_type],
                |row| row.get::<_, String>(0),
            )
            .map_err(|e| StorageError::Internal(e.to_string()))?
            .map(|r| r.map_err(|e| StorageError::Internal(e.to_string())))
            .collect::<Result<Vec<_>, _>>()?;

        for msg_id_str in &msg_ids {
            conn.execute(
                "UPDATE entities SET is_active = 0, updated_at = ?1 WHERE id = ?2",
                rusqlite::params![chrono::Utc::now().to_rfc3339(), msg_id_str],
            )
            .map_err(|e| StorageError::Internal(e.to_string()))?;
        }

        Ok(())
    }
}
