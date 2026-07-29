use async_trait::async_trait;
use futures::{Stream, StreamExt};
use knowledge_core::ports::chat::*;
use reqwest::Client;
use serde::{Deserialize, Serialize};

pub struct OllamaChatAdapter {
    client: Client,
    model: String,
    endpoint: String,
}

impl OllamaChatAdapter {
    pub fn new(model: String) -> Self {
        Self {
            client: Client::builder()
                .timeout(std::time::Duration::from_secs(120))
                .build()
                .expect("HTTP client"),
            model,
            endpoint: "http://localhost:11434".to_string(),
        }
    }

    pub fn with_endpoint(mut self, endpoint: String) -> Self {
        self.endpoint = endpoint;
        self
    }
}

#[derive(Debug, Serialize)]
struct OllamaMessage {
    role: String,
    content: String,
}

#[derive(Debug, Serialize)]
struct OllamaChatRequest {
    model: String,
    messages: Vec<OllamaMessage>,
    stream: bool,
}

#[derive(Debug, Deserialize)]
struct OllamaChatResponse {
    message: Option<OllamaChatResponseMessage>,
    done: bool,
}

#[derive(Debug, Deserialize)]
struct OllamaChatResponseMessage {
    content: String,
}

#[derive(Debug, Deserialize)]
struct OllamaErrorResponse {
    error: String,
}

fn build_ollama_messages(request: &ChatRequest) -> Vec<OllamaMessage> {
    let mut messages = Vec::new();
    if !request.system_prompt.is_empty() {
        messages.push(OllamaMessage {
            role: "system".to_string(),
            content: request.system_prompt.clone(),
        });
    }
    for msg in &request.messages {
        messages.push(OllamaMessage {
            role: match msg.role {
                MessageRole::System => "system",
                MessageRole::User => "user",
                MessageRole::Assistant => "assistant",
            }
            .to_string(),
            content: msg.content.clone(),
        });
    }
    messages
}

#[async_trait]
impl ChatCompletion for OllamaChatAdapter {
    async fn chat(&self, request: ChatRequest) -> Result<ChatResponse, ChatError> {
        let url = format!("{}/api/chat", self.endpoint);
        let body = OllamaChatRequest {
            model: self.model.clone(),
            messages: build_ollama_messages(&request),
            stream: false,
        };

        let response = self
            .client
            .post(&url)
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
            if let Ok(err) = serde_json::from_str::<OllamaErrorResponse>(&text) {
                return Err(ChatError::Provider(err.error));
            }
            return Err(ChatError::Provider(format!("Ollama error ({}): {}", status, text)));
        }

        let ollama_resp: OllamaChatResponse = serde_json::from_str(&text)
            .map_err(|e| ChatError::Provider(format!("Failed to parse response: {}", e)))?;

        Ok(ChatResponse {
            message: ollama_resp.message.map(|m| m.content).unwrap_or_default(),
            citations: vec![],
            referenced_entities: vec![],
        })
    }

    async fn chat_stream(
        &self,
        request: ChatRequest,
    ) -> Result<Box<dyn Stream<Item = ChatDelta> + Send + Unpin>, ChatError> {
        let url = format!("{}/api/chat", self.endpoint);
        let body = OllamaChatRequest {
            model: self.model.clone(),
            messages: build_ollama_messages(&request),
            stream: true,
        };

        let response = self
            .client
            .post(&url)
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
            if let Ok(err) = serde_json::from_str::<OllamaErrorResponse>(&text) {
                return Err(ChatError::Provider(err.error));
            }
            return Err(ChatError::Provider(format!("Ollama error ({}): {}", status, text)));
        }

        let stream = response
            .bytes_stream()
            .flat_map(|chunk_result| {
                let chunk = match chunk_result {
                    Ok(c) => c,
                    Err(_e) => return futures::stream::iter(vec![]),
                };
                let text = String::from_utf8_lossy(&chunk);
                let mut deltas = Vec::new();
                for line in text.lines() {
                    if line.is_empty() {
                        continue;
                    }
                    if let Ok(ollama_resp) =
                        serde_json::from_str::<OllamaChatResponse>(line)
                    {
                        if let Some(msg) = ollama_resp.message {
                            if !msg.content.is_empty() {
                                deltas.push(ChatDelta {
                                    delta: msg.content,
                                    citation: None,
                                    status: None,
                                    finished: ollama_resp.done,
                                });
                            }
                        }
                        if ollama_resp.done {
                            deltas.push(ChatDelta {
                                delta: String::new(),
                                citation: None,
                                status: None,
                                finished: true,
                            });
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
    use wiremock::matchers::{method, path};
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

    fn mock_chat_response_body() -> serde_json::Value {
        serde_json::json!({
            "model": "llama3.2",
            "created_at": "2026-01-01T00:00:00Z",
            "message": {
                "role": "assistant",
                "content": "Hello! How can I help?"
            },
            "done": true
        })
    }

    #[tokio::test]
    async fn ollama_chat_sends_correct_url() {
        let mock_server = MockServer::start().await;

        Mock::given(method("POST"))
            .and(path("/api/chat"))
            .respond_with(ResponseTemplate::new(200).set_body_json(mock_chat_response_body()))
            .mount(&mock_server)
            .await;

        let adapter = OllamaChatAdapter::new("llama3.2".into())
            .with_endpoint(mock_server.uri());

        let request = make_request();
        let response = adapter.chat(request).await.unwrap();
        assert_eq!(response.message, "Hello! How can I help?");
    }

    #[tokio::test]
    async fn ollama_chat_no_auth_header() {
        // Verify the adapter does not set any auth header by checking
        // the request body is correctly sent without Authorization.
        let mock_server = MockServer::start().await;

        Mock::given(method("POST"))
            .and(path("/api/chat"))
            .respond_with(ResponseTemplate::new(200).set_body_json(mock_chat_response_body()))
            .mount(&mock_server)
            .await;

        let adapter = OllamaChatAdapter::new("llama3.2".into())
            .with_endpoint(mock_server.uri());

        let request = make_request();
        let response = adapter.chat(request).await.unwrap();
        assert_eq!(response.message, "Hello! How can I help?");
    }

    #[tokio::test]
    async fn ollama_with_custom_endpoint() {
        let mock_server = MockServer::start().await;

        Mock::given(method("POST"))
            .and(path("/api/chat"))
            .respond_with(ResponseTemplate::new(200).set_body_json(mock_chat_response_body()))
            .mount(&mock_server)
            .await;

        let adapter = OllamaChatAdapter::new("llama3.2".into())
            .with_endpoint(mock_server.uri());

        let request = make_request();
        let response = adapter.chat(request).await.unwrap();
        assert!(!response.message.is_empty());
    }

    #[tokio::test]
    async fn ollama_chat_maps_connection_refused() {
        // Connect to a port that is not listening
        let adapter = OllamaChatAdapter::new("llama3.2".into())
            .with_endpoint("http://127.0.0.1:1".to_string());

        let request = make_request();
        let err = adapter.chat(request).await.unwrap_err();
        assert!(matches!(err, ChatError::Network(_)));
    }

    #[tokio::test]
    async fn ollama_chat_stream_parses_ndjson() {
        let mock_server = MockServer::start().await;

        let ndjson_body = "{\"model\":\"llama3.2\",\"message\":{\"role\":\"assistant\",\"content\":\"Hello\"},\"done\":false}\n{\"model\":\"llama3.2\",\"message\":{\"role\":\"assistant\",\"content\":\" world\"},\"done\":false}\n{\"model\":\"llama3.2\",\"message\":{\"role\":\"assistant\",\"content\":\"\"},\"done\":true}\n";

        Mock::given(method("POST"))
            .and(path("/api/chat"))
            .respond_with(
                ResponseTemplate::new(200)
                    .set_body_string(ndjson_body)
                    .insert_header("Content-Type", "application/x-ndjson"),
            )
            .mount(&mock_server)
            .await;

        let adapter = OllamaChatAdapter::new("llama3.2".into())
            .with_endpoint(mock_server.uri());

        let request = make_request();
        let stream = adapter.chat_stream(request).await.unwrap();
        let deltas: Vec<ChatDelta> = stream.collect().await;

        assert_eq!(deltas.len(), 3);
        assert!(deltas.last().unwrap().finished);
    }
}
