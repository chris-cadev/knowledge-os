use knowledge_core::ports::*;
use uuid::Uuid;

pub enum ChatStreamEvent {
    Status(ProcessingStatus),
    Delta(ChatDelta),
    Done {
        assistant_message_id: Uuid,
        citations: Vec<CitationSource>,
    },
    Error(ChatError),
}

pub struct ChatStreamHandle {
    pub conversation_id: Uuid,
    pub user_message_id: Uuid,
    pub stream: Box<dyn futures::Stream<Item = ChatStreamEvent> + Send + Unpin>,
    pub cancel: tokio::sync::watch::Sender<bool>,
}
