use async_trait::async_trait;
use futures::stream::{self, Stream};
use knowledge_core::ports::*;

#[derive(Default)]
pub struct MockChatAdapter {
    pub stream_delay_ms: u64,
}

#[async_trait]
impl ChatCompletion for MockChatAdapter {
    async fn chat(&self, request: ChatRequest) -> Result<ChatResponse, ChatError> {
        let message = build_mock_response(&request);
        let citations = extract_mock_citations(&request, &message);
        let referenced_entities: Vec<uuid::Uuid> = citations.iter().map(|c| c.entity_id).collect();
        Ok(ChatResponse {
            message,
            citations,
            referenced_entities,
        })
    }

    async fn chat_stream(
        &self,
        request: ChatRequest,
    ) -> Result<Box<dyn Stream<Item = ChatDelta> + Send + Unpin>, ChatError> {
        let message = build_mock_response(&request);
        let citations = extract_mock_citations(&request, &message);
        let chunks: Vec<String> = chunk_message(&message, 8);
        let total = chunks.len();
        let delay_ms = self.stream_delay_ms;

        let stream = stream::unfold(
            (chunks.into_iter().enumerate(), citations, 0u64),
            move |(mut iter, citations, tick)| async move {
                match iter.next() {
                    Some((i, chunk)) => {
                        if delay_ms > 0 {
                            tokio::time::sleep(std::time::Duration::from_millis(delay_ms)).await;
                        }
                        let citation = if i == 0 {
                            citations.first().map(|c| c.number)
                        } else {
                            None
                        };
                        Some((
                            ChatDelta {
                                delta: chunk,
                                citation,
                                status: Some(ProcessingStatus::Generating),
                                finished: i + 1 == total,
                            },
                            (iter, citations, tick + 1),
                        ))
                    }
                    None => None,
                }
            },
        );

        Ok(Box::new(Box::pin(stream)))
    }
}

fn build_mock_response(request: &ChatRequest) -> String {
    if request.context_entities.is_empty() {
        return "I don't have any entities to reference. Ask me about your knowledge graph.".into();
    }
    let first = &request.context_entities[0];
    format!(
        "Based on [1], the entity '{}' is relevant. Here is a summary of its content.",
        first.title
    )
}

fn extract_mock_citations(request: &ChatRequest, message: &str) -> Vec<CitationSource> {
    request
        .context_entities
        .iter()
        .take(1)
        .enumerate()
        .map(|(i, e)| CitationSource {
            number: i + 1,
            entity_id: e.entity_id,
            entity_type: e.entity_type.clone(),
            title: e.title.clone(),
            snippet: e.content.chars().take(200).collect(),
        })
        .filter(|_| message.contains(&format!("[{}]", 1)))
        .collect()
}

fn chunk_message(msg: &str, size: usize) -> Vec<String> {
    msg.chars()
        .collect::<Vec<_>>()
        .chunks(size)
        .map(|c| c.iter().collect())
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use futures::StreamExt;

    fn make_request(context_entities: Vec<EntityContext>) -> ChatRequest {
        ChatRequest {
            system_prompt: "You are a helpful assistant.".into(),
            messages: vec![Message {
                role: MessageRole::User,
                content: "What do you know?".into(),
                entity_refs: vec![],
            }],
            context_entities,
            mode: ResponseMode::Fast,
            source_toggles: SourceToggles::default(),
            config: ChatConfig::default(),
        }
    }

    #[tokio::test]
    async fn mock_chat_returns_response() {
        let adapter = MockChatAdapter::default();
        let request = make_request(vec![EntityContext {
            entity_id: uuid::Uuid::new_v4(),
            entity_type: "Article".into(),
            title: "Test Entity".into(),
            content: "Some content".into(),
            tags: vec![],
            relationships: vec![],
        }]);
        let response = adapter.chat(request).await.unwrap();
        assert!(!response.message.is_empty());
    }

    #[tokio::test]
    async fn mock_chat_empty_context() {
        let adapter = MockChatAdapter::default();
        let request = make_request(vec![]);
        let response = adapter.chat(request).await.unwrap();
        assert!(response.citations.is_empty());
    }

    #[tokio::test]
    async fn mock_chat_citations_populated() {
        let adapter = MockChatAdapter::default();
        let request = make_request(vec![EntityContext {
            entity_id: uuid::Uuid::new_v4(),
            entity_type: "Article".into(),
            title: "Test Entity".into(),
            content: "Some content".into(),
            tags: vec![],
            relationships: vec![],
        }]);
        let response = adapter.chat(request).await.unwrap();
        assert!(!response.citations.is_empty());
    }

    #[tokio::test]
    async fn mock_stream_produces_deltas() {
        let adapter = MockChatAdapter::default();
        let request = make_request(vec![EntityContext {
            entity_id: uuid::Uuid::new_v4(),
            entity_type: "Article".into(),
            title: "Test Entity".into(),
            content: "Some content".into(),
            tags: vec![],
            relationships: vec![],
        }]);
        let stream = adapter.chat_stream(request).await.unwrap();
        let deltas: Vec<ChatDelta> = stream.collect().await;
        assert!(!deltas.is_empty());
        assert!(deltas.last().unwrap().finished);
    }

    #[tokio::test]
    async fn mock_stream_finished_flag() {
        let adapter = MockChatAdapter::default();
        let request = make_request(vec![EntityContext {
            entity_id: uuid::Uuid::new_v4(),
            entity_type: "Article".into(),
            title: "Test Entity".into(),
            content: "Some content".into(),
            tags: vec![],
            relationships: vec![],
        }]);
        let stream = adapter.chat_stream(request).await.unwrap();
        let deltas: Vec<ChatDelta> = stream.collect().await;
        for (i, delta) in deltas.iter().enumerate() {
            if i == deltas.len() - 1 {
                assert!(delta.finished);
            } else {
                assert!(!delta.finished);
            }
        }
    }

    #[test]
    fn mock_chat_response_serializable() {
        let response = ChatResponse {
            message: "test response".into(),
            citations: vec![],
            referenced_entities: vec![],
        };
        let json = serde_json::to_string(&response).unwrap();
        let deserialized: ChatResponse = serde_json::from_str(&json).unwrap();
        assert_eq!(response.message, deserialized.message);
    }
}
