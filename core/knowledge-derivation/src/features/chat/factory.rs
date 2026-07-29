use std::collections::HashMap;

use knowledge_core::ports::chat::{ChatCompletion, ChatError};

pub fn create_chat_provider(config: &str) -> Result<Box<dyn ChatCompletion>, ChatError> {
    if config == "mock" || config.starts_with("mock://") {
        return Ok(Box::new(super::mock::MockChatAdapter::default()));
    }

    if let Some(rest) = config.strip_prefix("openai://") {
        let (model, params) = parse_query(rest);
        let api_key = params
            .get("api_key")
            .cloned()
            .or_else(|| std::env::var("OPENAI_API_KEY").ok())
            .ok_or_else(|| ChatError::Provider("OPENAI_API_KEY not set".into()))?;
        let base_url = params.get("base_url").cloned();
        let mut adapter = super::openai::OpenAiChatAdapter::new(api_key, model);
        if let Some(url) = base_url {
            adapter = adapter.with_base_url(url);
        }
        return Ok(Box::new(adapter));
    }

    if let Some(rest) = config.strip_prefix("ollama://") {
        let (model, params) = parse_query(rest);
        let endpoint = params
            .get("url")
            .cloned()
            .or_else(|| std::env::var("OLLAMA_HOST").ok())
            .unwrap_or_else(|| "http://localhost:11434".to_string());
        let adapter =
            super::ollama::OllamaChatAdapter::new(model).with_endpoint(endpoint);
        return Ok(Box::new(adapter));
    }

    if let Ok(provider) = std::env::var("KOS_CHAT_PROVIDER") {
        return create_chat_provider(&provider);
    }

    Ok(Box::new(super::mock::MockChatAdapter::default()))
}

fn parse_query(s: &str) -> (String, HashMap<String, String>) {
    let mut parts = s.split('?');
    let model = parts.next().unwrap_or("").to_string();
    let mut params = HashMap::new();
    if let Some(query) = parts.next() {
        for pair in query.split('&') {
            if let Some((k, v)) = pair.split_once('=') {
                params.insert(k.to_string(), v.to_string());
            }
        }
    }
    (model, params)
}

#[cfg(test)]
mod tests {
    use super::*;
    use knowledge_core::ports::chat::{
        ChatConfig, ChatRequest, Message, MessageRole, ResponseMode, SourceToggles,
    };

    fn test_request() -> ChatRequest {
        ChatRequest {
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
        }
    }

    #[test]
    fn factory_creates_mock_for_mock_scheme() {
        let provider = create_chat_provider("mock://").unwrap();
        let rt = tokio::runtime::Runtime::new().unwrap();
        let response = rt.block_on(provider.chat(test_request())).unwrap();
        assert!(response.message.contains("don't have any entities"));
    }

    #[test]
    fn factory_creates_mock_by_default() {
        let provider = create_chat_provider("").unwrap();
        let rt = tokio::runtime::Runtime::new().unwrap();
        let response = rt.block_on(provider.chat(test_request())).unwrap();
        assert!(!response.message.is_empty());
    }

    #[test]
    fn factory_creates_ollama_for_ollama_scheme() {
        let provider = create_chat_provider("ollama://llama3.2").unwrap();
        let rt = tokio::runtime::Runtime::new().unwrap();
        let result = rt.block_on(provider.chat(test_request()));
        assert!(result.is_err());
        match result {
            Err(ChatError::Network(_)) => {}
            other => panic!(
                "Expected Network error from non-running Ollama, got: {:?}",
                other
            ),
        }
    }

    #[test]
    fn factory_returns_provider_error_for_openai_without_key() {
        std::env::remove_var("OPENAI_API_KEY");
        let result = create_chat_provider("openai://gpt-4o");
        assert!(result.is_err());
        match result {
            Err(ChatError::Provider(msg)) => {
                assert!(msg.contains("OPENAI_API_KEY"));
            }
            _ => panic!("Expected Provider error"),
        }
    }

    #[test]
    fn factory_creates_openai_with_env_key() {
        std::env::set_var("OPENAI_API_KEY", "test-key");
        let provider = create_chat_provider("openai://gpt-4o").unwrap();
        let rt = tokio::runtime::Runtime::new().unwrap();
        let result = rt.block_on(provider.chat(test_request()));
        assert!(result.is_err());
        std::env::remove_var("OPENAI_API_KEY");
    }
}
