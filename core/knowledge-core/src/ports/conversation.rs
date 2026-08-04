use async_trait::async_trait;
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use super::error::StorageError;
use crate::ports::chat::{CitationSource, MessageRole, ResponseFeedback};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ConversationSummary {
    pub id: Uuid,
    pub title: String,
    pub message_count: u32,
    pub last_message_preview: Option<String>,
    pub last_message_at: Option<DateTime<Utc>>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ConversationDetail {
    pub id: Uuid,
    pub title: String,
    pub messages: Vec<MessageDetail>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MessageDetail {
    pub id: Uuid,
    pub role: MessageRole,
    pub text: String,
    pub entity_refs: Vec<Uuid>,
    pub citations: Vec<CitationSource>,
    pub feedback: Option<ResponseFeedback>,
    pub created_at: DateTime<Utc>,
}

#[async_trait]
pub trait ConversationRepository: Send + Sync {
    async fn list_conversations(&self) -> Result<Vec<ConversationSummary>, StorageError>;

    async fn get_conversation(
        &self,
        conversation_id: Uuid,
    ) -> Result<Option<ConversationDetail>, StorageError>;

    async fn rename_conversation(
        &self,
        conversation_id: Uuid,
        new_title: &str,
    ) -> Result<(), StorageError>;

    async fn archive_conversation(&self, conversation_id: Uuid) -> Result<(), StorageError>;
}
