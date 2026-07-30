use async_trait::async_trait;
use futures::{Stream, StreamExt};
use knowledge_core::ports::*;
use reqwest::Client;
use serde::{Deserialize, Serialize};

pub struct OpenAiChatAdapter {
    client: Client,
    model: String,
    api_key: String,
    base_url: String,
}

impl OpenAiChatAdapter {
    pub fn new(api_key: String, model: String) -> Self {
        Self {
            client: Client::builder()
                .timeout(std::time::Duration::from_secs(60))
                .build()
                .expect("HTTP client"),
            model,
            api_key,
            base_url: "https://api.openai.com/v1".to_string(),
        }
    }

    pub fn with_base_url(mut self, base_url: String) -> Self {
        self.base_url = base_url;
        self
    }
}

#[derive(Debug, Serialize)]
struct OpenAiChatRequestMessage {
    role: String,
    content: String,
}

#[derive(Debug, Serialize)]
struct OpenAiChatRequestBody {
    model: String,
    messages: Vec<OpenAiChatRequestMessage>,
    temperature: f64,
    max_tokens: u32,
    stream: bool,
}

#[derive(Debug, Deserialize)]
#[allow(dead_code)]
struct OpenAiChatChoice {
    index: u32,
    message: Option<OpenAiChatResponseMessage>,
    delta: Option<OpenAiChatDelta>,
    finish_reason: Option<String>,
}

#[derive(Debug, Deserialize)]
#[allow(dead_code)]
struct OpenAiChatResponseMessage {
    role: String,
    content: String,
}

#[derive(Debug, Deserialize)]
struct OpenAiChatDelta {
    content: Option<String>,
}

#[derive(Debug, Deserialize)]
#[allow(dead_code)]
struct OpenAiChatUsage {
    prompt_tokens: u32,
    completion_tokens: u32,
    total_tokens: u32,
}

#[derive(Debug, Deserialize)]
#[allow(dead_code)]
struct OpenAiChatResponse {
    id: String,
    choices: Vec<OpenAiChatChoice>,
    usage: Option<OpenAiChatUsage>,
}

#[derive(Debug, Deserialize)]
struct OpenAiErrorResponse {
    error: OpenAiErrorDetail,
}

#[derive(Debug, Deserialize)]
struct OpenAiErrorDetail {
    message: String,
    #[allow(dead_code)]
    r#type: Option<String>,
    #[allow(dead_code)]
    code: Option<String>,
}

fn build_request_body(request: &ChatRequest, model: &str, stream: bool) -> OpenAiChatRequestBody {
    let mut messages = Vec::new();
    messages.push(OpenAiChatRequestMessage {
        role: "system".to_string(),
        content: request.system_prompt.clone(),
    });
    for msg in &request.messages {
        messages.push(OpenAiChatRequestMessage {
            role: match msg.role {
                MessageRole::System => "system",
                MessageRole::User => "user",
                MessageRole::Assistant => "assistant",
            }
            .to_string(),
            content: msg.content.clone(),
        });
    }
    OpenAiChatRequestBody {
        model: model.to_string(),
        messages,
        temperature: request.config.temperature,
        max_tokens: request.config.max_tokens,
        stream,
    }
}

fn map_openai_error(status: reqwest::StatusCode, body: &str) -> ChatError {
    if status == reqwest::StatusCode::TOO_MANY_REQUESTS {
        return ChatError::RateLimited(body.to_string());
    }
    if let Ok(err) = serde_json::from_str::<OpenAiErrorResponse>(body) {
        return ChatError::Provider(err.error.message);
    }
    ChatError::Provider(format!("HTTP {}: {}", status, body))
}

fn build_chat_response(openai_resp: OpenAiChatResponse) -> ChatResponse {
    let message = openai_resp
        .choices
        .first()
        .and_then(|c| c.message.as_ref())
        .map(|m| m.content.clone())
        .unwrap_or_default();
    ChatResponse {
        message,
        citations: vec![],
        referenced_entities: vec![],
    }
}

#[async_trait]
impl ChatCompletion for OpenAiChatAdapter {
    async fn chat(&self, request: ChatRequest) -> Result<ChatResponse, ChatError> {
        let url = format!("{}/chat/completions", self.base_url);
        let body = build_request_body(&request, &self.model, false);

        let response = self
            .client
            .post(&url)
            .bearer_auth(&self.api_key)
            .json(&body)
            .send()
            .await
            .map_err(|e| ChatError::Network(e.to_string()))?;

        let status = response.status();
        let text = response
            .text()
            .await
            .unwrap_or_else(|_| "Unknown error".to_string());

        if !status.is_success() {
            return Err(map_openai_error(status, &text));
        }

        let openai_resp: OpenAiChatResponse = serde_json::from_str(&text)
            .map_err(|e| ChatError::Provider(format!("Failed to parse response: {}", e)))?;

        Ok(build_chat_response(openai_resp))
    }

    async fn chat_stream(
        &self,
        request: ChatRequest,
    ) -> Result<Box<dyn Stream<Item = ChatDelta> + Send + Unpin>, ChatError> {
        let url = format!("{}/chat/completions", self.base_url);
        let body = build_request_body(&request, &self.model, true);

        let response = self
            .client
            .post(&url)
            .bearer_auth(&self.api_key)
            .json(&body)
            .send()
            .await
            .map_err(|e| ChatError::Network(e.to_string()))?;

        let status = response.status();
        if !status.is_success() {
            let text = response
                .text()
                .await
                .unwrap_or_else(|_| "Unknown error".to_string());
            return Err(map_openai_error(status, &text));
        }

        let stream = response.bytes_stream().flat_map(|chunk_result| {
            let chunk = match chunk_result {
                Ok(c) => c,
                Err(_) => return futures::stream::iter(vec![]),
            };
            let text = String::from_utf8_lossy(&chunk);
            let mut deltas = Vec::new();
            for line in text.lines() {
                if let Some(data) = line.strip_prefix("data: ") {
                    if data == "[DONE]" {
                        deltas.push(ChatDelta {
                            delta: String::new(),
                            citation: None,
                            status: None,
                            finished: true,
                        });
                        continue;
                    }
                    if let Ok(sse) = serde_json::from_str::<OpenAiChatResponse>(data) {
                        if let Some(choice) = sse.choices.first() {
                            if let Some(delta) = &choice.delta {
                                if let Some(content) = &delta.content {
                                    if !content.is_empty() {
                                        deltas.push(ChatDelta {
                                            delta: content.clone(),
                                            citation: None,
                                            status: None,
                                            finished: false,
                                        });
                                    }
                                }
                            }
                            if choice.finish_reason.is_some() {
                                if let Some(last) = deltas.last_mut() {
                                    last.finished = true;
                                }
                            }
                        }
                    }
                }
            }
            futures::stream::iter(deltas)
        });

        Ok(Box::new(Box::pin(stream)))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use wiremock::matchers::{header, method, path};
    use wiremock::{Mock, MockServer, ResponseTemplate};

    fn make_request() -> ChatRequest {
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

    fn mock_response_body() -> serde_json::Value {
        serde_json::json!({
            "id": "chatcmpl-123",
            "object": "chat.completion",
            "created": 1677652288,
            "model": "gpt-4o",
            "choices": [{
                "index": 0,
                "message": {
                    "role": "assistant",
                    "content": "Hello! How can I help you today?"
                },
                "finish_reason": "stop"
            }],
            "usage": {
                "prompt_tokens": 9,
                "completion_tokens": 7,
                "total_tokens": 16
            }
        })
    }

    fn with_v1_base(uri: &str) -> String {
        format!("{}/v1", uri.trim_end_matches('/'))
    }

    #[tokio::test]
    async fn openai_chat_sends_bearer_auth() {
        let mock_server = MockServer::start().await;

        Mock::given(method("POST"))
            .and(path("/v1/chat/completions"))
            .and(header("Authorization", "Bearer test-key-123"))
            .respond_with(ResponseTemplate::new(200).set_body_json(mock_response_body()))
            .mount(&mock_server)
            .await;

        let adapter = OpenAiChatAdapter::new("test-key-123".into(), "gpt-4o".into())
            .with_base_url(with_v1_base(&mock_server.uri()));

        let request = make_request();
        let response = adapter.chat(request).await.unwrap();
        assert_eq!(response.message, "Hello! How can I help you today?");
    }

    #[tokio::test]
    async fn openai_chat_sends_correct_url() {
        let mock_server = MockServer::start().await;

        Mock::given(method("POST"))
            .and(path("/v1/chat/completions"))
            .respond_with(ResponseTemplate::new(200).set_body_json(mock_response_body()))
            .mount(&mock_server)
            .await;

        let adapter = OpenAiChatAdapter::new("test-key".into(), "gpt-4o".into())
            .with_base_url(with_v1_base(&mock_server.uri()));

        let request = make_request();
        adapter.chat(request).await.unwrap();
    }

    #[tokio::test]
    async fn openai_chat_maps_400_to_provider_error() {
        let mock_server = MockServer::start().await;

        Mock::given(method("POST"))
            .and(path("/v1/chat/completions"))
            .respond_with(ResponseTemplate::new(400).set_body_json(serde_json::json!({
                "error": {
                    "message": "Invalid request",
                    "type": "invalid_request_error",
                    "code": null
                }
            })))
            .mount(&mock_server)
            .await;

        let adapter = OpenAiChatAdapter::new("test-key".into(), "gpt-4o".into())
            .with_base_url(with_v1_base(&mock_server.uri()));

        let request = make_request();
        let err = adapter.chat(request).await.unwrap_err();
        assert!(matches!(err, ChatError::Provider(_)));
        assert!(err.to_string().contains("Invalid request"));
    }

    #[tokio::test]
    async fn openai_chat_maps_429_to_rate_limited() {
        let mock_server = MockServer::start().await;

        Mock::given(method("POST"))
            .and(path("/v1/chat/completions"))
            .respond_with(ResponseTemplate::new(429).set_body_string("Rate limit exceeded"))
            .mount(&mock_server)
            .await;

        let adapter = OpenAiChatAdapter::new("test-key".into(), "gpt-4o".into())
            .with_base_url(with_v1_base(&mock_server.uri()));

        let request = make_request();
        let err = adapter.chat(request).await.unwrap_err();
        assert!(matches!(err, ChatError::RateLimited(_)));
    }

    #[tokio::test]
    async fn openai_with_base_url() {
        let mock_server = MockServer::start().await;

        Mock::given(method("POST"))
            .and(path("/v1/chat/completions"))
            .respond_with(ResponseTemplate::new(200).set_body_json(mock_response_body()))
            .mount(&mock_server)
            .await;

        let adapter = OpenAiChatAdapter::new("test-key".into(), "gpt-4o".into())
            .with_base_url(with_v1_base(&mock_server.uri()));

        let request = make_request();
        let response = adapter.chat(request).await.unwrap();
        assert!(!response.message.is_empty());
    }

    #[tokio::test]
    async fn openai_chat_stream_parses_sse() {
        let mock_server = MockServer::start().await;

        let sse_body = "data: {\"id\":\"1\",\"object\":\"chat.completion.chunk\",\"choices\":[{\"index\":0,\"delta\":{\"content\":\"Hello\"},\"finish_reason\":null}]}\n\ndata: {\"id\":\"2\",\"object\":\"chat.completion.chunk\",\"choices\":[{\"index\":0,\"delta\":{\"content\":\" world\"},\"finish_reason\":null}]}\n\ndata: {\"id\":\"3\",\"object\":\"chat.completion.chunk\",\"choices\":[{\"index\":0,\"delta\":{\"content\":\"\"},\"finish_reason\":\"stop\"}]}\n\ndata: [DONE]\n\n";

        Mock::given(method("POST"))
            .and(path("/v1/chat/completions"))
            .respond_with(
                ResponseTemplate::new(200)
                    .set_body_string(sse_body)
                    .insert_header("Content-Type", "text/event-stream"),
            )
            .mount(&mock_server)
            .await;

        let adapter = OpenAiChatAdapter::new("test-key".into(), "gpt-4o".into())
            .with_base_url(with_v1_base(&mock_server.uri()));

        let request = make_request();
        let stream = adapter.chat_stream(request).await.unwrap();
        let deltas: Vec<ChatDelta> = stream.collect().await;

        assert_eq!(deltas.len(), 3);
        assert!(deltas.last().unwrap().finished);
    }
}
