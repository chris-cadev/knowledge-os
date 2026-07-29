use async_trait::async_trait;
use futures::Stream;
use serde::{Deserialize, Serialize};
use uuid::Uuid;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum MessageRole {
    System,
    User,
    Assistant,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Message {
    pub role: MessageRole,
    pub content: String,
    pub entity_refs: Vec<Uuid>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RelationshipSummary {
    pub relationship_type: String,
    pub target_id: Uuid,
    pub target_title: String,
    pub target_type: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EntityContext {
    pub entity_id: Uuid,
    pub entity_type: String,
    pub title: String,
    pub content: String,
    pub tags: Vec<String>,
    pub relationships: Vec<RelationshipSummary>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CitationSource {
    pub number: usize,
    pub entity_id: Uuid,
    pub entity_type: String,
    pub title: String,
    pub snippet: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum ResponseMode {
    Fast,
    Thinking,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SourceToggles {
    pub knowledge_graph: bool,
    pub web_search: bool,
}

impl Default for SourceToggles {
    fn default() -> Self {
        Self {
            knowledge_graph: true,
            web_search: false,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChatConfig {
    pub temperature: f64,
    pub max_tokens: u32,
    pub model: Option<String>,
}

impl Default for ChatConfig {
    fn default() -> Self {
        Self {
            temperature: 0.7,
            max_tokens: 2048,
            model: None,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChatRequest {
    pub system_prompt: String,
    pub messages: Vec<Message>,
    pub context_entities: Vec<EntityContext>,
    pub mode: ResponseMode,
    pub source_toggles: SourceToggles,
    pub config: ChatConfig,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChatResponse {
    pub message: String,
    pub citations: Vec<CitationSource>,
    pub referenced_entities: Vec<Uuid>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ProcessingStatus {
    Searching { detail: String },
    ReadingEntities { count: u32 },
    Generating,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChatDelta {
    pub delta: String,
    pub citation: Option<usize>,
    pub status: Option<ProcessingStatus>,
    pub finished: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum FeedbackRating {
    ThumbsUp,
    ThumbsDown,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum FeedbackReason {
    WrongEntity,
    MissingInfo,
    WrongCitation,
    Other,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ResponseFeedback {
    pub message_id: Uuid,
    pub rating: FeedbackRating,
    pub reason: Option<FeedbackReason>,
    pub comment: Option<String>,
}

#[derive(Debug, thiserror::Error)]
pub enum ChatError {
    #[error("provider error: {0}")]
    Provider(String),
    #[error("network error: {0}")]
    Network(String),
    #[error("rate limited: {0}")]
    RateLimited(String),
    #[error("context too long: {0}")]
    ContextTooLong(String),
}

#[async_trait]
pub trait ChatCompletion: Send + Sync {
    async fn chat(&self, request: ChatRequest) -> Result<ChatResponse, ChatError>;
    async fn chat_stream(
        &self,
        request: ChatRequest,
    ) -> Result<Box<dyn Stream<Item = ChatDelta> + Send + Unpin>, ChatError>;
}
