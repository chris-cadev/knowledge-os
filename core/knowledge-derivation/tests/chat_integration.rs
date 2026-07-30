use knowledge_core::ports::*;
use knowledge_derivation::features::chat::factory::create_chat_provider;

#[test]
fn factory_creates_mock_for_mock_scheme() {
    let provider = create_chat_provider("mock://").unwrap();
    let rt = tokio::runtime::Runtime::new().unwrap();
    let response = rt
        .block_on(provider.chat(ChatRequest {
            system_prompt: "You are helpful.".into(),
            messages: vec![Message {
                role: MessageRole::User,
                content: "Hello".into(),
                entity_refs: vec![],
            }],
            context_entities: vec![],
            mode: ResponseMode::Fast,
            source_toggles: SourceToggles::default(),
            config: ChatConfig::default(),
        }))
        .unwrap();
    assert!(!response.message.is_empty());
}

#[test]
fn factory_creates_ollama_for_ollama_scheme() {
    let provider = create_chat_provider("ollama://llama3.2").unwrap();
    let rt = tokio::runtime::Runtime::new().unwrap();
    let result = rt.block_on(provider.chat(ChatRequest {
        system_prompt: "You are helpful.".into(),
        messages: vec![Message {
            role: MessageRole::User,
            content: "Hello".into(),
            entity_refs: vec![],
        }],
        context_entities: vec![],
        mode: ResponseMode::Fast,
        source_toggles: SourceToggles::default(),
        config: ChatConfig::default(),
    }));
    assert!(result.is_err());
    match result {
        Err(ChatError::Network(_)) => {}
        other => panic!(
            "Expected Network error from non-running Ollama, got: {:?}",
            other
        ),
    }
}

#[tokio::test]
async fn mock_adapter_round_trip_chat() {
    let provider = create_chat_provider("mock://").unwrap();
    let response = provider
        .chat(ChatRequest {
            system_prompt: "You are helpful.".into(),
            messages: vec![Message {
                role: MessageRole::User,
                content: "What do you know?".into(),
                entity_refs: vec![],
            }],
            context_entities: vec![EntityContext {
                entity_id: uuid::Uuid::new_v4(),
                entity_type: "Article".into(),
                title: "Rust Programming".into(),
                content: "Rust is a systems programming language.".into(),
                tags: vec![],
                relationships: vec![],
            }],
            mode: ResponseMode::Fast,
            source_toggles: SourceToggles::default(),
            config: ChatConfig::default(),
        })
        .await
        .unwrap();
    assert!(!response.message.is_empty());
    assert!(!response.citations.is_empty());
    assert_eq!(response.citations[0].number, 1);
}

#[tokio::test]
async fn mock_adapter_stream_finished_flag() {
    let provider = create_chat_provider("mock://").unwrap();
    let mut stream = provider
        .chat_stream(ChatRequest {
            system_prompt: "You are helpful.".into(),
            messages: vec![Message {
                role: MessageRole::User,
                content: "Hello".into(),
                entity_refs: vec![],
            }],
            context_entities: vec![EntityContext {
                entity_id: uuid::Uuid::new_v4(),
                entity_type: "Article".into(),
                title: "Test".into(),
                content: "Test content.".into(),
                tags: vec![],
                relationships: vec![],
            }],
            mode: ResponseMode::Fast,
            source_toggles: SourceToggles::default(),
            config: ChatConfig::default(),
        })
        .await
        .unwrap();
    use futures::StreamExt;
    let mut last_finished = false;
    while let Some(delta) = stream.next().await {
        last_finished = delta.finished;
    }
    assert!(last_finished);
}
