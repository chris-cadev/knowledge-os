# IP-009: ChatCompletion Port Trait and Built-in Adapters

**Status:** Draft
**ADR(s):** [ADR-0023](../../architecture/adrs/adr-0023.md) (ChatCompletion Port Trait for LLM Provider Abstraction)
**PRD(s):** [PRD-0007](../prds/prd-0007-knowledge-chat-and-universal-import.md) (F2 Chat View — F2.11 offline Mock, F2.17 mode, F2.13 source toggles)
**Estimated effort:** ~3 days

---

## Context

ADR-0023 establishes a new `ChatCompletion` port trait in `knowledge-core` for LLM provider abstraction. The chat pipeline (IP-011) depends on this trait; the desktop app (IP-008) and the future MCP server (IP-014) wrap providers through the trait. Built-in adapters for OpenAI-compatible endpoints, Ollama, and a Mock provider are required at launch.

**Current state:**
- `core/knowledge-core/src/ports/ai.rs` defines `AiAdapter` for *embedding* generation (used by the search pipeline). This is a separate concern — embeddings are not chat. The new `ChatCompletion` trait is a parallel port for chat.
- `core/knowledge-derivation/src/features/search/providers/openai.rs` provides an `OpenAiAdapter` for the embeddings API (`POST /v1/embeddings`). A new `OpenAiChatAdapter` is needed for the chat completions API (`POST /v1/chat/completions`).
- `core/knowledge-derivation/src/features/search/providers/mod.rs` has the `create_from_config()` factory for embedders. A parallel `create_chat_provider()` factory is needed for chat.
- `desktop/src-tauri/Cargo.toml` already depends on `reqwest` transitively via `knowledge-derivation`. No new dependency is strictly required for HTTP, but a `futures` dependency is needed for the streaming `Stream` type.
- `core/knowledge-derivation/Cargo.toml` already includes `reqwest = { version = "0.12", default-features = false, features = ["json", "rustls-tls"] }`. The `stream` feature is required for SSE.

This plan establishes the trait, the request/response types, three built-in adapters, and the factory function. The trait is not registered as a plugin capability at launch (per ADR-0023's deferred plugin extension point).

---

## Deliverables

### D1: ChatCompletion Port Trait and Request/Response Types

**Purpose:** Define the `ChatCompletion` port trait in `knowledge-core` with serializable request/response types.

**Files:**

| File | Action | Description |
|------|--------|-------------|
| `core/knowledge-core/src/ports/chat.rs` | Create | New `ChatCompletion` trait, `ChatRequest`, `ChatResponse`, `ChatDelta`, `Message`, `MessageRole`, `EntityContext`, `CitationSource`, `RelationshipSummary`, `ResponseMode`, `SourceToggles`, `ResponseFeedback`, `FeedbackRating`, `FeedbackReason`, `ChatConfig`, `ProcessingStatus`, `ChatError` |
| `core/knowledge-core/src/ports/mod.rs` | Modify | Add `pub mod chat;` and `pub use chat::*;` |

**New types and trait:**

```rust
// core/knowledge-core/src/ports/chat.rs
use async_trait::async_trait;
use chrono::{DateTime, Utc};
use futures::Stream;
use serde::{Deserialize, Serialize};
use uuid::Uuid;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum MessageRole { System, User, Assistant }

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
pub enum ResponseMode { Fast, Thinking }

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SourceToggles {
    pub knowledge_graph: bool,
    pub web_search: bool,
}

impl Default for SourceToggles {
    fn default() -> Self {
        Self { knowledge_graph: true, web_search: false }
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
        Self { temperature: 0.7, max_tokens: 2048, model: None }
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
pub enum FeedbackRating { ThumbsUp, ThumbsDown }

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
```

**Cargo.toml additions** (in `core/knowledge-core/Cargo.toml`):
- `futures = "0.3"` — required for `Stream` trait used in `chat_stream` return type.

**Verification:**
- `cargo check -p knowledge-core` compiles
- `cargo test -p knowledge-core` passes (no regressions to existing 99 tests)
- The trait compiles with `#[async_trait]` macro (existing pattern in `ports/ai.rs`, `ports/entity.rs`)

**Exit criteria:** Trait and all types compile, derive `Serialize`/`Deserialize`, no behavioral change to existing ports.

---

### D2: MockChatAdapter

**Purpose:** Implement `ChatCompletion` for the Mock provider — deterministic responses for testing and offline use.

**Files:**

| File | Action | Description |
|------|--------|-------------|
| `core/knowledge-derivation/src/features/chat/mod.rs` | Create | New module for chat providers |
| `core/knowledge-derivation/src/features/chat/mock.rs` | Create | `MockChatAdapter` implementation |

**Implementation:**

```rust
// core/knowledge-derivation/src/features/chat/mock.rs
use async_trait::async_trait;
use futures::stream::{self, Stream};
use knowledge_core::ports::chat::*;

pub struct MockChatAdapter {
    /// Per-character delay in milliseconds for streaming simulation.
    /// Default 0 (instant). Used in tests for streaming behavior validation.
    pub stream_delay_ms: u64,
}

impl Default for MockChatAdapter {
    fn default() -> Self { Self { stream_delay_ms: 0 } }
}

#[async_trait]
impl ChatCompletion for MockChatAdapter {
    async fn chat(&self, request: ChatRequest) -> Result<ChatResponse, ChatError> {
        let message = build_mock_response(&request);
        let citations = extract_mock_citations(&request, &message);
        let referenced_entities = citations.iter().map(|c| c.entity_id).collect();
        Ok(ChatResponse { message, citations, referenced_entities })
    }

    async fn chat_stream(
        &self,
        request: ChatRequest,
    ) -> Result<Box<dyn Stream<Item = ChatDelta> + Send + Unpin>, ChatError> {
        let message = build_mock_response(&request);
        let citations = extract_mock_citations(&request, &message);
        let chunks: Vec<String> = chunk_message(&message, 8);
        let total = chunks.len();

        let stream = stream::unfold(
            (chunks.into_iter().enumerate(), citations, 0u64),
            move |(mut iter, citations, tick)| async move {
                match iter.next() {
                    Some((i, chunk)) => {
                        if self.stream_delay_ms > 0 {
                            tokio::time::sleep(std::time::Duration::from_millis(
                                self.stream_delay_ms,
                            )).await;
                        }
                        let citation = if i == 0 {
                            citations.first().map(|c| c.number)
                        } else { None };
                        Some((
                            ChatDelta {
                                delta: chunk,
                                citation,
                                status: Some(if i == 0 { ProcessingStatus::Generating } else { ProcessingStatus::Generating }),
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
    request.context_entities.iter().take(1).enumerate().map(|(i, e)| {
        CitationSource {
            number: i + 1,
            entity_id: e.entity_id,
            entity_type: e.entity_type.clone(),
            title: e.title.clone(),
            snippet: e.content.chars().take(200).collect(),
        }
    }).filter(|_| message.contains(&format!("[{}]", 1))).collect()
}

fn chunk_message(msg: &str, size: usize) -> Vec<String> {
    msg.chars().collect::<Vec<_>>().chunks(size).map(|c| c.iter().collect()).collect()
}
```

**Tests** (in same file, `#[cfg(test)] mod tests`):
- `mock_chat_returns_response` — `chat()` returns non-empty message
- `mock_chat_empty_context` — empty context_entities returns no-citation response
- `mock_chat_citations_populated` — non-empty context produces at least one citation
- `mock_stream_produces_deltas` — `chat_stream()` yields multiple `ChatDelta` with `finished: true` on last
- `mock_stream_finished_flag` — final delta has `finished: true`, all others `finished: false`
- `mock_chat_response_serializable` — response round-trips through serde_json

**Verification:**
- `cargo test -p knowledge-derivation` passes with new 6 tests
- `cargo test -p knowledge-core` still passes (no changes to core)

**Exit criteria:** `MockChatAdapter` implements the trait, all 6 tests pass.

---

### D3: OpenAI-Compatible Chat Adapter

**Purpose:** Implement `ChatCompletion` for OpenAI and any OpenAI-compatible chat completions endpoint (LM Studio, vLLM, llama.cpp server, Together AI, Groq).

**Files:**

| File | Action | Description |
|------|--------|-------------|
| `core/knowledge-derivation/src/features/chat/openai.rs` | Create | `OpenAiChatAdapter` calling `POST {base_url}/chat/completions` |
| `core/knowledge-knowledge-derivation/src/features/chat/mod.rs` | Modify | Re-export `OpenAiChatAdapter` |

**Implementation outline:**

```rust
// core/knowledge-derivation/src/features/chat/openai.rs
use async_trait::async_trait;
use futures::stream::{Stream, StreamExt};
use knowledge_core::ports::chat::*;
use reqwest::Client;
use serde::{Deserialize, Serialize};

pub struct OpenAiChatAdapter {
    client: Client,
    model: String,
    api_key: String,
    base_url: String,  // default: "https://api.openai.com/v1"
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

#[async_trait]
impl ChatCompletion for OpenAiChatAdapter {
    async fn chat(&self, request: ChatRequest) -> Result<ChatResponse, ChatError> {
        let url = format!("{}/chat/completions", self.base_url);
        let body = build_request_body(&request, &self.model, false);
        let response = self.client.post(&url)
            .bearer_auth(&self.api_key)
            .json(&body)
            .send().await
            .map_err(|e| ChatError::Network(e.to_string()))?;
        // ... parse, map errors, return ChatResponse
        todo!("parse non-streaming response")
    }

    async fn chat_stream(
        &self,
        request: ChatRequest,
    ) -> Result<Box<dyn Stream<Item = ChatDelta> + Send + Unpin>, ChatError> {
        let url = format!("{}/chat/completions", self.base_url);
        let body = build_request_body(&request, &self.model, true);
        let response = self.client.post(&url)
            .bearer_auth(&self.api_key)
            .json(&body)
            .send().await
            .map_err(|e| ChatError::Network(e.to_string()))?;
        // ... parse SSE stream, map to ChatDelta
        todo!("parse SSE stream")
    }
}
```

**Cargo.toml change** in `core/knowledge-derivation/Cargo.toml`:
- Add `reqwest = { version = "0.12", default-features = false, features = ["json", "rustls-tls", "stream"] }` (add `stream` feature for `bytes_stream()`).

**Tests** (use `mockito` or `httpmock` for in-process HTTP mock):
- `openai_chat_sends_bearer_auth` — verify `Authorization: Bearer ...` header
- `openai_chat_sends_correct_url` — verify POST to `{base_url}/chat/completions`
- `openai_chat_maps_400_to_provider_error` — error response maps to `ChatError::Provider`
- `openai_chat_maps_429_to_rate_limited` — rate limit response maps to `ChatError::RateLimited`
- `openai_chat_stream_parses_sse` — SSE chunks are parsed into deltas
- `openai_with_base_url` — custom base_url is used (e.g., `http://localhost:1234/v1`)

**Verification:**
- `cargo test -p knowledge-derivation` passes with new 6 tests
- Manual: integration test against a real LM Studio instance at `http://localhost:1234/v1` returns a non-empty response

**Exit criteria:** `OpenAiChatAdapter` works with OpenAI API and any OpenAI-compatible endpoint, all 6 tests pass.

---

### D4: Ollama Chat Adapter

**Purpose:** Implement `ChatCompletion` for Ollama local and remote endpoints.

**Files:**

| File | Action | Description |
|------|--------|-------------|
| `core/knowledge-derivation/src/features/chat/ollama.rs` | Create | `OllamaChatAdapter` calling `POST {endpoint}/api/chat` |
| `core/knowledge-derivation/src/features/chat/mod.rs` | Modify | Re-export `OllamaChatAdapter` |

**Implementation outline:**

```rust
// core/knowledge-derivation/src/features/chat/ollama.rs
pub struct OllamaChatAdapter {
    client: Client,
    model: String,
    endpoint: String,  // default: "http://localhost:11434"
}

impl OllamaChatAdapter {
    pub fn new(model: String) -> Self {
        Self {
            client: Client::builder()
                .timeout(std::time::Duration::from_secs(120))  // local models can be slow
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
```

The Ollama chat API uses newline-delimited JSON (not SSE), and accepts an `{"model": "...", "messages": [...], "stream": true}` body. Each line is a JSON object with a `message.content` field for streamed deltas.

**Tests**:
- `ollama_chat_sends_correct_url` — POST to `{endpoint}/api/chat`
- `ollama_chat_no_auth_header` — Ollama does not require authentication
- `ollama_chat_stream_parses_ndjson` — newline-delimited JSON parsed into deltas
- `ollama_with_custom_endpoint` — custom endpoint URL used
- `ollama_chat_maps_connection_refused` — connection error maps to `ChatError::Network`

**Verification:**
- `cargo test -p knowledge-derivation` passes with new 5 tests
- Manual: `curl -X POST http://localhost:11434/api/chat -d '{"model": "llama3.2", ...}'` returns expected JSON shape

**Exit criteria:** `OllamaChatAdapter` works with local and remote Ollama instances, all 5 tests pass.

---

### D5: Factory Function and Integration Tests

**Purpose:** Implement the `create_chat_provider()` factory function and a cross-adapter integration test suite.

**Files:**

| File | Action | Description |
|------|--------|-------------|
| `core/knowledge-derivation/src/features/chat/factory.rs` | Create | `create_chat_provider()` parsing `mock://`, `openai://...`, `ollama://...` |
| `core/knowledge-derivation/src/features/chat/mod.rs` | Modify | Re-export `create_chat_provider` |
| `core/knowledge-derivation/tests/chat_integration.rs` | Create | Cross-adapter integration tests |

**Factory implementation:**

```rust
// core/knowledge-derivation/src/features/chat/factory.rs
use knowledge_core::ports::chat::{ChatCompletion, ChatError};

pub fn create_chat_provider(config: &str) -> Result<Box<dyn ChatCompletion>, ChatError> {
    // "mock://" or "mock" → MockChatAdapter
    if config == "mock" || config.starts_with("mock://") {
        return Ok(Box::new(super::mock::MockChatAdapter::default()));
    }

    // "openai://MODEL?api_key=KEY&base_url=URL" → OpenAiChatAdapter
    if let Some(rest) = config.strip_prefix("openai://") {
        let (model, params) = parse_query(rest);
        let api_key = params.get("api_key")
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

    // "ollama://MODEL" or "ollama://MODEL?url=URL" → OllamaChatAdapter
    if let Some(rest) = config.strip_prefix("ollama://") {
        let (model, params) = parse_query(rest);
        let endpoint = params.get("url").cloned()
            .or_else(|| std::env::var("OLLAMA_HOST").ok())
            .unwrap_or_else(|| "http://localhost:11434".to_string());
        let adapter = super::ollama::OllamaChatAdapter::new(model).with_endpoint(endpoint);
        return Ok(Box::new(adapter));
    }

    // Environment variable fallback
    if let Ok(provider) = std::env::var("KOS_CHAT_PROVIDER") {
        return create_chat_provider(&provider);
    }

    // Default to mock
    Ok(Box::new(super::mock::MockChatAdapter::default()))
}

fn parse_query(s: &str) -> (String, std::collections::HashMap<String, String>) {
    let mut parts = s.split('?');
    let model = parts.next().unwrap_or("").to_string();
    let mut params = std::collections::HashMap::new();
    if let Some(query) = parts.next() {
        for pair in query.split('&') {
            if let Some((k, v)) = pair.split_once('=') {
                params.insert(k.to_string(), v.to_string());
            }
        }
    }
    (model, params)
}
```

**Integration tests** in `tests/chat_integration.rs`:
- `factory_creates_mock_for_mock_scheme` — `create_chat_provider("mock://")` returns a working adapter
- `factory_creates_mock_by_default` — `create_chat_provider("")` returns mock
- `factory_creates_ollama_for_ollama_scheme` — `create_chat_provider("ollama://llama3.2")` returns Ollama adapter
- `factory_creates_openai_with_env_key` — `create_chat_provider("openai://gpt-4o")` with `OPENAI_API_KEY` set returns OpenAI adapter
- `factory_returns_provider_error_for_openai_without_key` — `create_chat_provider("openai://gpt-4o")` without key returns `ChatError::Provider`
- `mock_adapter_round_trip_chat` — `MockChatAdapter::chat()` produces valid response
- `mock_adapter_stream_finished_flag` — last `ChatDelta` has `finished: true`

**Verification:**
- `cargo test -p knowledge-derivation` passes with all new tests (target: 5 factory + 6 mock + 6 openai + 5 ollama + 7 integration = 29 new tests)
- `cargo test --workspace` passes (no regressions to existing 200+ tests)
- `cargo clippy --all-targets --all-features -- -D warnings` clean
- `cargo fmt --check` clean

**Exit criteria:** Factory function handles all schemes, integration tests pass, no regressions.

---

## Execution Order

```
D1 (trait/types) -> D2 (Mock) -> D3 (OpenAI) -> D4 (Ollama) -> D5 (factory + integration)
```

D1 is pure type definitions. D2 has no network dependency and validates the trait shape. D3 and D4 are the real network adapters. D5 wires the factory and verifies all adapters can be instantiated.

---

## Verification Strategy

| Level | Command | Coverage |
|-------|---------|----------|
| Unit | `cargo test -p knowledge-core` | Type compilation, no regressions |
| Unit | `cargo test -p knowledge-derivation` | Mock adapter, factory parsing |
| Integration | `cargo test -p knowledge-derivation --test chat_integration` | Cross-adapter integration |
| Lint | `cargo clippy --all-targets --all-features -- -D warnings && cargo fmt --check` | Code quality |
| Manual | LM Studio at `http://localhost:1234/v1` | Real OpenAI-compatible endpoint |

---

## Exit Criteria

- [ ] `ChatCompletion` trait and 14 supporting types in `core/knowledge-core/src/ports/chat.rs`
- [ ] `MockChatAdapter` with deterministic responses (6 unit tests)
- [ ] `OpenAiChatAdapter` for OpenAI and OpenAI-compatible endpoints (6 unit tests with httpmock)
- [ ] `OllamaChatAdapter` for Ollama local/remote (5 unit tests with httpmock)
- [ ] `create_chat_provider()` factory function (5 integration tests)
- [ ] All request/response types derive `Serialize`/`Deserialize`
- [ ] All existing tests pass (no regressions)
- [ ] `cargo clippy --all-targets --all-features -- -D warnings` passes
- [ ] `cargo fmt --check` passes
- [ ] ADR-0023 updated with Implementation Notes

---

## Impact Analysis

### Structural Changes and Consumers

| Change | Direct Consumers | Transitive Consumers |
|--------|------------------|---------------------|
| `ports/chat.rs` (new) | IP-010 (Conversation entities), IP-011 (Chat pipeline), IP-014 (MCP service) | Desktop chat view, future MCP server, future REST API |
| `features/chat/` module (new) | `ChatPipeline` (IP-011) | Tauri commands, CLI `kos conversation` commands |
| `Cargo.toml` `futures` dep in `knowledge-core` | All crates depending on `knowledge-core` | Workspace-wide rebuild |

### Risk Surface

1. **Trait surface area:** The trait has 2 methods (`chat`, `chat_stream`) and ~14 supporting types. Future provider features (function calling, vision) may require trait extension. **Mitigation:** Provider-specific features are added as new methods on the trait with default implementations, not breaking changes to the existing surface.

2. **Streaming protocol differences:** OpenAI uses SSE (`data: {json}\n\n`), Ollama uses newline-delimited JSON. Each adapter handles its own protocol. **Mitigation:** `ChatDelta` is a uniform output type. Adapter-specific protocol logic is encapsulated.

3. **Mock adapter parity:** The mock must not diverge from real adapter behavior. **Mitigation:** Mock returns a complete `ChatResponse` with citations, not a placeholder. Integration tests verify the mock against the trait contract.

4. **HTTP client timeout:** Long-running LLM requests may exceed default timeouts. **Mitigation:** OpenAI client timeout is 60s, Ollama client timeout is 120s. Streaming requests are not bounded by total time but by per-chunk time.

5. **`futures` dependency in `knowledge-core`:** Adding a new dependency to the foundational crate requires rebuild of all dependent crates. **Mitigation:** `futures` is a lightweight, widely-used crate. The `Stream` trait is essential for streaming responses.

---

## Implementation Notes

*(Filled in during/after implementation — records deviations, discoveries, decisions made during coding)*
