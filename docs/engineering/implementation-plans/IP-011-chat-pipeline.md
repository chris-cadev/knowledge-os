# IP-011: Chat Pipeline and Context Assembly (RAG)

**Status:** Draft
**ADR(s):** [ADR-0025](../../architecture/adrs/adr-0025.md) (Chat Context Assembly as a Derivation Layer Pipeline), [ADR-0023](../../architecture/adrs/adr-0023.md) (ChatCompletion Port Trait)
**PRD(s):** [PRD-0007](../prds/prd-0007-knowledge-chat-and-universal-import.md) (F2.6 streaming, F2.12 citations, F2.13 source toggles, F2.14 processing states, F2.17 Fast/Thinking mode, F2.19 stop generation)
**Estimated effort:** ~5 days

---

## Context

ADR-0025 establishes the chat pipeline as a derivation-layer (Layer 6) feature. The pipeline assembles AI context from canonical entities, calls `ChatCompletion` (defined in IP-009), persists the conversation (using entities from IP-010), and emits streaming deltas with intermediate processing status. The pipeline is framework-agnostic — invoked from Tauri commands and (in the future) from the MCP service (IP-014).

**Current state:**
- `core/knowledge-derivation/src/features/search/pipeline.rs` has the `EmbeddingPipeline` as a reference for derivation-layer pipeline structure. The chat pipeline follows the same pattern: dependency injection via constructor, async methods, event emission.
- `core/knowledge-derivation/src/features/search/hybrid.rs` has `reciprocal_rank_fusion()` for combining keyword and semantic search results. The chat pipeline reuses this for context retrieval when no explicit `@`-references are provided.
- `core/knowledge-derivation/src/features/view/` has 4 view adapters that follow a consistent `new(deps...)` constructor pattern.
- `desktop/src-tauri/src/commands/` has 11 IPC commands (per IP-008). New chat commands (`chat_send`, `chat_stream`, `chat_search_entities`, `chat_list_conversations`, `chat_rename_conversation`, `chat_delete_conversation`, `chat_stop_stream`, `chat_send_feedback`) are added by this plan.
- `desktop/src-tauri/src/commands/store.rs` defines `AppState { store: Arc<Mutex<SqliteStore>> }`. `AppState` is extended with `ChatPipeline` and `ChatCompletion` provider.

**Dependencies:**
- IP-009 (ChatCompletion trait + adapters) must be complete
- IP-010 (Conversation entities + ConversationRepository) must be complete
- IP-004 (semantic search) is already complete (referenced for `SearchIndex` and `VectorStore`)

This plan depends on IP-009 and IP-010. It can be developed in parallel with IP-012 (OCR) and IP-013 (Universal Import) — the chat pipeline is independent of the import pipeline.

---

## Deliverables

### D1: ChatPipeline Struct and Constructor

**Purpose:** Create the `ChatPipeline` struct with dependency injection and a `chat()` method that orchestrates the full chat flow.

**Files:**

| File | Action | Description |
|------|--------|-------------|
| `core/knowledge-derivation/src/features/chat/mod.rs` | Modify | Add `pipeline` submodule |
| `core/knowledge-derivation/src/features/chat/pipeline.rs` | Create | `ChatPipeline` struct, `ChatResult` type, `chat()` method |

**Implementation:**

```rust
// core/knowledge-derivation/src/features/chat/pipeline.rs
use std::sync::Arc;
use async_trait::async_trait;
use chrono::Utc;
use futures::Stream;
use knowledge_core::features::component::{Component, ComponentType};
use knowledge_core::features::entity::{Entity, EntityType};
use knowledge_core::features::relationship::{Relationship, RelationshipType};
use knowledge_core::ports::chat::*;
use knowledge_core::ports::{
    ComponentRepository, EntityRepository, Event, EventLog, EventType, RelationshipRepository,
    SearchIndex, SearchQuery, StorageError, VectorStore, VectorMetadata,
};
use uuid::Uuid;

pub struct ChatPipeline {
    chat_provider: Arc<dyn ChatCompletion>,
    entity_repo: Arc<dyn EntityRepository>,
    component_repo: Arc<dyn ComponentRepository>,
    relationship_repo: Arc<dyn RelationshipRepository>,
    search_index: Arc<dyn SearchIndex>,
    vector_store: Arc<dyn VectorStore>,
}

#[derive(Debug, Clone)]
pub struct ChatResult {
    pub conversation_id: Uuid,
    pub message_id: Uuid,
    pub message: String,
    pub citations: Vec<CitationSource>,
    pub referenced_entities: Vec<Uuid>,
}

impl ChatPipeline {
    pub fn new(
        chat_provider: Arc<dyn ChatCompletion>,
        entity_repo: Arc<dyn EntityRepository>,
        component_repo: Arc<dyn ComponentRepository>,
        relationship_repo: Arc<dyn RelationshipRepository>,
        search_index: Arc<dyn SearchIndex>,
        vector_store: Arc<dyn VectorStore>,
    ) -> Self {
        Self { chat_provider, entity_repo, component_repo, relationship_repo, search_index, vector_store }
    }

    /// Send a chat message with optional entity references.
    /// Returns the AI response and persists the conversation.
    pub async fn chat(
        &self,
        conversation_id: Option<Uuid>,
        user_message: &str,
        entity_refs: &[Uuid],
        source_toggles: &SourceToggles,
        mode: ResponseMode,
    ) -> Result<ChatResult, ChatError> {
        // 1. Load or create conversation
        let conv_id = match conversation_id {
            Some(id) => id,
            None => self.create_conversation().await
                .map_err(|e| ChatError::Provider(e.to_string()))?,
        };

        // 2. Persist user Message with EntityRefs
        let user_msg_id = self.persist_message(
            conv_id,
            MessageRole::User,
            user_message,
            entity_refs,
        ).await.map_err(|e| ChatError::Provider(e.to_string()))?;

        // 3. Build entity context (explicit refs OR search)
        let context_entities = if source_toggles.knowledge_graph {
            if !entity_refs.is_empty() {
                self.build_context_for_entities(entity_refs).await
            } else {
                self.search_context(user_message, 10).await
            }
        } else {
            vec![]
        };

        // 4. Build system prompt
        let system_prompt = build_system_prompt(&context_entities, source_toggles);

        // 5. Build ChatRequest
        let request = ChatRequest {
            system_prompt,
            messages: vec![Message {
                role: MessageRole::User,
                content: user_message.to_string(),
                entity_refs: entity_refs.to_vec(),
            }],
            context_entities: context_entities.clone(),
            mode: mode.clone(),
            source_toggles: source_toggles.clone(),
            config: ChatConfig::default(),
        };

        // 6. Call provider
        let response = self.chat_provider.chat(request).await?;

        // 7. Persist assistant Message with citations
        let assistant_msg_id = self.persist_message_with_citations(
            conv_id,
            MessageRole::Assistant,
            &response.message,
            &response.citations,
        ).await.map_err(|e| ChatError::Provider(e.to_string()))?;

        // 8. Emit ChatContextRetrieved event (derivation event)
        // (handled by event log, see D3)

        Ok(ChatResult {
            conversation_id: conv_id,
            message_id: assistant_msg_id,
            message: response.message,
            citations: response.citations,
            referenced_entities: response.referenced_entities,
        })
    }

    // ... helper methods
}
```

The `create_conversation`, `persist_message`, `build_context_for_entities`, `search_context`, and `persist_message_with_citations` methods are private helpers.

**Tests** in same file:
- `pipeline_creates_conversation_on_none` — `chat(None, ...)` creates a new conversation
- `pipeline_persists_user_message` — user Message entity is created with `MessageContent` and `EntityRefs`
- `pipeline_calls_provider` — `MockChatAdapter` receives the expected `ChatRequest`
- `pipeline_persists_assistant_message` — assistant Message is created with citations
- `pipeline_handles_empty_entity_refs` — empty refs triggers search
- `pipeline_handles_disabled_knowledge_graph` — `SourceToggles { knowledge_graph: false, ..}` skips search and context

**Verification:**
- `cargo test -p knowledge-derivation` passes with 6 new tests
- `cargo test --workspace` passes (no regressions)

**Exit criteria:** `ChatPipeline::chat()` is end-to-end functional with `MockChatAdapter`.

---

### D2: System Prompt Template and Citation Extraction

**Purpose:** Implement the system prompt construction with the entity context template and the post-response citation extraction.

**Files:**

| File | Action | Description |
|------|--------|-------------|
| `core/knowledge-derivation/src/features/chat/prompt.rs` | Create | `build_system_prompt()` function with the template from PRD-0007 §System Prompt Template |
| `core/knowledge-derivation/src/features/chat/citations.rs` | Create | `extract_citations()` function that maps `[N]` markers in response text to entity context |
| `core/knowledge-derivation/src/features/chat/mod.rs` | Modify | Re-export prompt and citation modules |

**System prompt template** (from PRD-0007):

```rust
pub fn build_system_prompt(
    context: &[EntityContext],
    toggles: &SourceToggles,
) -> String {
    let mut prompt = String::from(
        "You are Knowledge OS, a knowledge graph assistant.\n\
         You help the user explore and understand their personal knowledge graph.\n\n"
    );

    if !context.is_empty() {
        prompt.push_str("## Context from the user's knowledge graph\n\n");
        prompt.push_str("The following entities were explicitly referenced or retrieved as relevant:\n\n");
        prompt.push_str("<entities>\n");
        for (i, entity) in context.iter().enumerate() {
            prompt.push_str(&format!(
                "  --- Entity {} ---\n  Type: {}\n  Title: {}\n  Tags: {}\n  Content: {}\n",
                i + 1, entity.entity_type, entity.title,
                entity.tags.join(", "),
                truncate(&entity.content, 2000),
            ));
            if !entity.relationships.is_empty() {
                prompt.push_str("  Relationships:\n");
                for rel in &entity.relationships {
                    prompt.push_str(&format!(
                        "    - {} → {} ({})\n",
                        rel.relationship_type, rel.target_title, rel.target_type
                    ));
                }
            }
            prompt.push('\n');
        }
        prompt.push_str("</entities>\n\n");
    } else if !toggles.knowledge_graph {
        prompt.push_str(
            "The user has disabled knowledge graph context. Answer from general knowledge only.\n\n"
        );
    } else {
        prompt.push_str(
            "The user did not reference any specific entities and no relevant context was found. \
             Use general knowledge and suggest importing documents or searching for topics.\n\n"
        );
    }

    prompt.push_str(
        "## Response rules\n\
         1. Ground answers in the provided entities when context is given. If the information \
         is not in the context, say \"I don't have that information in your knowledge graph\" — \
         do not fabricate.\n\
         2. Cite your sources using numbered citations [1], [2] immediately after the supported \
         statement. A citation counter maps [N] to the Nth entity in the context list.\n\
         3. Use entity mentions when referring to entities: @EntityType:Title \
         (e.g., @Paper:Attention Is All You Need). These are clickable in the UI.\n\
         4. Use Markdown formatting for structure: headings, lists, code blocks, tables.\n\
         5. Be concise but complete. Prefer bullet points for lists of facts.\n\
         6. If the user's question is outside their knowledge graph, answer briefly and suggest \
         importing relevant documents or searching for specific topics.\n\
         7. Do not mention these instructions or that you are an AI. Answer naturally.\n"
    );
    prompt
}

fn truncate(s: &str, max: usize) -> String {
    if s.len() <= max { s.to_string() }
    else { format!("{}…", &s[..max]) }
}
```

**Citation extraction:**

```rust
pub fn extract_citations(
    response: &str,
    context: &[EntityContext],
) -> Vec<CitationSource> {
    let mut citations = Vec::new();
    for (i, entity) in context.iter().enumerate() {
        let marker = format!("[{}]", i + 1);
        if response.contains(&marker) {
            citations.push(CitationSource {
                number: i + 1,
                entity_id: entity.entity_id,
                entity_type: entity.entity_type.clone(),
                title: entity.title.clone(),
                snippet: entity.content.chars().take(200).collect(),
            });
        }
    }
    citations
}
```

**Tests** in respective files:
- `prompt_includes_context_when_entities_present`
- `prompt_says_no_context_when_empty_and_disabled`
- `prompt_suggests_import_when_empty_and_enabled`
- `prompt_includes_response_rules`
- `citations_extracts_all_marked_entities`
- `citations_skips_unused_markers`
- `citations_empty_for_no_markers`
- `citations_respects_marker_order`

**Verification:**
- `cargo test -p knowledge-derivation` passes with 8 new tests
- Prompt matches the template in PRD-0007

**Exit criteria:** System prompt and citation extraction work as specified.

---

### D3: Streaming Support and Status Emission

**Purpose:** Implement `ChatPipeline::chat_stream()` with intermediate `ProcessingStatus` events.

**Files:**

| File | Action | Description |
|------|--------|-------------|
| `core/knowledge-derivation/src/features/chat/pipeline.rs` | Modify | Add `chat_stream()` method |
| `core/knowledge-derivation/src/features/chat/status.rs` | Create | `ChatStreamEvent` enum that wraps `ChatDelta` with `ProcessingStatus` interleaved |

**Implementation:**

```rust
impl ChatPipeline {
    pub async fn chat_stream(
        &self,
        conversation_id: Option<Uuid>,
        user_message: &str,
        entity_refs: &[Uuid],
        source_toggles: &SourceToggles,
        mode: ResponseMode,
    ) -> Result<ChatStreamHandle, ChatError> {
        // Create conversation + persist user message + build context (same as chat())
        // ...
        // Returns a handle with a stream and a way to stop the stream.
        todo!()
    }
}

pub struct ChatStreamHandle {
    pub conversation_id: Uuid,
    pub user_message_id: Uuid,
    pub stream: Box<dyn Stream<Item = ChatStreamEvent> + Send + Unpin>,
    /// Cancel handle for stop-stream IPC command.
    cancel: tokio::sync::watch::Sender<bool>,
}

pub enum ChatStreamEvent {
    Status(ProcessingStatus),
    Delta(ChatDelta),
    Done { assistant_message_id: Uuid, citations: Vec<CitationSource> },
    Error(ChatError),
}
```

The stream emits events in this order:
1. `Status(Searching { detail })` when starting search
2. `Status(ReadingEntities { count })` when loading context entities
3. `Status(Generating)` when calling the LLM
4. `Delta(ChatDelta { delta, ... })` for each LLM token
5. `Done { ... }` when complete

The `cancel` channel allows the desktop app to stop mid-stream via `chat_stop_stream` IPC command.

**Tests:**
- `stream_emits_status_before_delta` — first event is a `Status`
- `stream_emits_generating_status_before_first_delta` — `Status(Generating)` precedes first `Delta`
- `stream_finished_flag_on_last_delta` — last `Delta` has `finished: true`
- `stream_done_includes_message_id_and_citations`
- `cancel_stops_stream` — sending `cancel` causes stream to terminate

**Verification:**
- `cargo test -p knowledge-derivation` passes with 5 new streaming tests
- Manual test: `MockChatAdapter` with `stream_delay_ms: 50` produces visible intermediate status

**Exit criteria:** Streaming with status emission works, cancel mechanism works.

---

### D4: Desktop Tauri Chat Commands and Events

**Purpose:** Expose the chat pipeline through Tauri IPC commands and event emissions.

**Files:**

| File | Action | Description |
|------|--------|-------------|
| `desktop/src-tauri/src/commands/chat.rs` | Create | 8 chat IPC commands |
| `desktop/src-tauri/src/commands/store.rs` | Modify | Extend `AppState` with `ChatPipeline` and `ChatCompletion` provider |
| `desktop/src-tauri/src/commands/mod.rs` | Modify | Re-export `chat` module |
| `desktop/src-tauri/src/lib.rs` | Modify | Register chat commands, construct `ChatPipeline` in setup, emit events |
| `desktop/src-tauri/Cargo.toml` | Modify | Add `knowledge-derivation` already present; add `tokio-stream` for stream forwarding |
| `desktop/src/lib/types.ts` | Modify | Add `ChatMessage`, `ChatRequest`, `ChatDelta`, `Citation`, `Conversation`, `MessageDetail`, `ProcessingStatus` TypeScript types |
| `desktop/src/lib/api.ts` | Modify | Add 8 chat IPC wrapper functions |
| `desktop/src/lib/chat-stream.ts` | Create | TypeScript event subscription for chat streaming |

**AppState extension:**

```rust
// desktop/src-tauri/src/commands/store.rs
pub struct AppState {
    pub store: Arc<Mutex<SqliteStore>>,
    pub chat_pipeline: Arc<ChatPipeline>,
    pub chat_provider_kind: String,  // for UI display
}
```

**Chat commands:**

```rust
// desktop/src-tauri/src/commands/chat.rs
use tauri::{State, Emitter, AppHandle};
use knowledge_core::ports::chat::*;

#[tauri::command]
pub async fn chat_send(
    state: State<'_, AppState>,
    conversation_id: Option<String>,
    message: String,
    entity_refs: Vec<String>,
    source_toggles: Option<SourceToggles>,
    mode: Option<ResponseMode>,
) -> Result<ChatSendResult, String> {
    let entity_refs: Vec<Uuid> = entity_refs.iter()
        .filter_map(|s| Uuid::parse_str(s).ok())
        .collect();
    let toggles = source_toggles.unwrap_or_default();
    let mode = mode.unwrap_or(ResponseMode::Thinking);

    let conv_id = conversation_id.and_then(|s| Uuid::parse_str(&s).ok());

    let result = state.chat_pipeline.chat(conv_id, &message, &entity_refs, &toggles, mode).await
        .map_err(|e| e.to_string())?;

    Ok(ChatSendResult {
        conversation_id: result.conversation_id.to_string(),
        message_id: result.message_id.to_string(),
        message: result.message,
        citations: result.citations,
        referenced_entities: result.referenced_entities.iter().map(|u| u.to_string()).collect(),
    })
}

#[tauri::command]
pub async fn chat_stream(
    app: AppHandle,
    state: State<'_, AppState>,
    conversation_id: Option<String>,
    message: String,
    entity_refs: Vec<String>,
    source_toggles: Option<SourceToggles>,
    mode: Option<ResponseMode>,
) -> Result<String, String> {
    let conv_id = conversation_id.and_then(|s| Uuid::parse_str(&s).ok());
    let entity_refs: Vec<Uuid> = entity_refs.iter().filter_map(|s| Uuid::parse_str(s).ok()).collect();
    let toggles = source_toggles.unwrap_or_default();
    let mode = mode.unwrap_or(ResponseMode::Thinking);

    let handle = state.chat_pipeline.chat_stream(conv_id, &message, &entity_refs, &toggles, mode).await
        .map_err(|e| e.to_string())?;

    let conversation_id = handle.conversation_id.to_string();
    let user_message_id = handle.user_message_id.to_string();

    // Spawn task to forward stream events to Tauri events
    let app_clone = app.clone();
    tokio::spawn(async move {
        use futures::StreamExt;
        let mut stream = handle.stream;
        while let Some(event) = stream.next().await {
            match event {
                ChatStreamEvent::Status(s) => {
                    let _ = app_clone.emit("chat:status", &s);
                }
                ChatStreamEvent::Delta(d) => {
                    let _ = app_clone.emit("chat:delta", &d);
                }
                ChatStreamEvent::Done { assistant_message_id, citations } => {
                    let _ = app_clone.emit("chat:done", serde_json::json!({
                        "user_message_id": user_message_id,
                        "assistant_message_id": assistant_message_id.to_string(),
                        "citations": citations,
                    }));
                }
                ChatStreamEvent::Error(e) => {
                    let _ = app_clone.emit("chat:error", &e.to_string());
                }
            }
        }
    });

    Ok(conversation_id)
}

#[tauri::command]
pub async fn chat_search_entities(
    state: State<'_, AppState>,
    prefix: String,
) -> Result<Vec<EntitySearchResult>, String> {
    // Use SearchIndex directly (no pipeline) for low-latency autocomplete
    let query = SearchQuery { query: prefix.clone(), entity_type: None, tag: None };
    let results = state.store.search(&query).await.map_err(|e| e.to_string())?;
    // ... map to EntitySearchResult with type, title, preview
    todo!()
}

#[tauri::command]
pub async fn chat_list_conversations(
    state: State<'_, AppState>,
) -> Result<Vec<ConversationSummaryResponse>, String> {
    let conversations = state.store.list_conversations().await.map_err(|e| e.to_string())?;
    // ... map to response type
    todo!()
}

#[tauri::command]
pub async fn chat_delete_conversation(
    state: State<'_, AppState>,
    conversation_id: String,
) -> Result<(), String> {
    let id = Uuid::parse_str(&conversation_id).map_err(|e| e.to_string())?;
    state.store.archive_conversation(id).await.map_err(|e| e.to_string())?;
    Ok(())
}

#[tauri::command]
pub async fn chat_rename_conversation(
    state: State<'_, AppState>,
    conversation_id: String,
    title: String,
) -> Result<(), String> {
    let id = Uuid::parse_str(&conversation_id).map_err(|e| e.to_string())?;
    state.store.rename_conversation(id, &title).await.map_err(|e| e.to_string())?;
    Ok(())
}

#[tauri::command]
pub async fn chat_stop_stream(
    state: State<'_, AppState>,
    conversation_id: String,
) -> Result<(), String> {
    // Cancel the active stream for this conversation
    // (tracked via a registry in AppState)
    todo!()
}

#[tauri::command]
pub async fn chat_send_feedback(
    state: State<'_, AppState>,
    feedback: ResponseFeedback,
) -> Result<(), String> {
    // Persist feedback as a component on the Message entity
    todo!()
}
```

**TypeScript bindings:**

```typescript
// desktop/src/lib/types.ts additions
export interface ChatMessage {
  role: "system" | "user" | "assistant";
  content: string;
  entity_refs: string[];
}

export interface Citation {
  number: number;
  entity_id: string;
  entity_type: string;
  title: string;
  snippet: string;
}

export interface ChatSendResult {
  conversation_id: string;
  message_id: string;
  message: string;
  citations: Citation[];
  referenced_entities: string[];
}

export interface ChatDelta {
  delta: string;
  citation?: number;
  status?: ProcessingStatus;
  finished: boolean;
}

export interface ProcessingStatus {
  Searching?: { detail: string };
  ReadingEntities?: { count: number };
  Generating?: null;
}

export interface ConversationSummary {
  id: string;
  title: string;
  message_count: number;
  last_message_preview: string | null;
  last_message_at: string | null;
  created_at: string;
  updated_at: string;
}
```

```typescript
// desktop/src/lib/api.ts additions
export async function chatSend(
  conversationId: string | null,
  message: string,
  entityRefs: string[],
  sourceToggles: { knowledge_graph: boolean; web_search: boolean },
  mode: "fast" | "thinking"
): Promise<ChatSendResult> {
  return invoke("chat_send", {
    conversationId,
    message,
    entityRefs,
    sourceToggles,
    mode,
  });
}

// ... similar for chatStream, chatSearchEntities, etc.
```

**Verification:**
- `cargo check -p knowledge-desktop` compiles
- `cargo test -p knowledge-desktop` passes (mock store + mock chat provider)
- `npm run check` passes (TypeScript types match Rust types)
- Manual: `cargo tauri dev` launches, chat tab is functional with MockChatAdapter

**Exit criteria:** 8 chat IPC commands work, TypeScript types match, Tauri events flow.

---

### D5: Chat Pipeline Integration Tests

**Purpose:** End-to-end tests of the chat pipeline against the SQLite store.

**Files:**

| File | Action | Description |
|------|--------|-------------|
| `core/knowledge-derivation/tests/chat_pipeline_integration.rs` | Create | End-to-end pipeline tests with real SQLite store |

**Test scenarios:**

```rust
#[tokio::test]
async fn end_to_end_chat_creates_conversation_and_persists_messages() {
    // Setup: in-memory SqliteStore, MockChatAdapter
    // Action: chat_pipeline.chat(None, "Hello", &[], &toggles, mode).await
    // Assert: Conversation entity created
    // Assert: User Message entity created with role=user
    // Assert: Assistant Message entity created with role=assistant
    // Assert: has_message relationships exist
}

#[tokio::test]
async fn end_to_end_chat_with_explicit_entity_refs_uses_context() {
    // Setup: import 2 entities (Paper, Concept)
    // Action: chat with entity_refs = [paper_id, concept_id]
    // Assert: pipeline's MockChatAdapter receives context_entities with both
    // Assert: assistant Message's EntityRefs includes cited entities
}

#[tokio::test]
async fn end_to_end_chat_without_refs_runs_search() {
    // Setup: import entities
    // Action: chat with entity_refs = []
    // Assert: pipeline runs SearchIndex::search
    // Assert: top-k context_entities passed to provider
}

#[tokio::test]
async fn end_to_end_chat_disables_knowledge_graph() {
    // Setup: import entities
    // Action: chat with source_toggles.knowledge_graph = false
    // Assert: context_entities is empty in ChatRequest
    // Assert: system prompt says "no context"
}

#[tokio::test]
async fn chat_citations_create_referenced_by_relationships() {
    // Setup: 1 entity, MockChatAdapter returns "Based on [1]..."
    // Action: chat
    // Assert: referenced_by relationship from entity to assistant Message
}

#[tokio::test]
async fn chat_response_citations_persisted_in_message_entity() {
    // Setup: 1 entity, MockChatAdapter returns response with [1]
    // Action: chat
    // Assert: assistant Message has EntityRefs component with cited entity id
}
```

**Verification:**
- `cargo test -p knowledge-derivation --test chat_pipeline_integration` passes with 6 new tests
- `cargo test --workspace` passes (no regressions)

**Exit criteria:** End-to-end pipeline works against real SQLite store.

---

## Execution Order

```
D1 (pipeline struct) -> D2 (prompt + citations) -> D3 (streaming) -> D4 (Tauri commands) -> D5 (integration)
```

D1 is the core orchestration. D2 adds the prompt template and citation extraction. D3 adds streaming. D4 wires it to Tauri. D5 verifies the full path against the real store.

---

## Verification Strategy

| Level | Command | Coverage |
|-------|---------|----------|
| Unit | `cargo test -p knowledge-derivation` | Pipeline logic, prompt, citations, streaming |
| Integration | `cargo test -p knowledge-derivation --test chat_pipeline_integration` | End-to-end with SQLite |
| Compile | `cargo check -p knowledge-desktop` | Tauri command compilation |
| Type-check | `npm run check` | TypeScript types match |
| Lint | `cargo clippy --all-targets --all-features -- -D warnings && cargo fmt --check` | Code quality |
| Manual | `cargo tauri dev` | Desktop app chat tab |

---

## Exit Criteria

- [ ] `ChatPipeline::chat()` end-to-end functional
- [ ] `ChatPipeline::chat_stream()` with `ProcessingStatus` emission
- [ ] System prompt template matches PRD-0007 specification
- [ ] Citation extraction maps `[N]` markers to entities
- [ ] 8 Tauri IPC commands: `chat_send`, `chat_stream`, `chat_search_entities`, `chat_list_conversations`, `chat_delete_conversation`, `chat_rename_conversation`, `chat_stop_stream`, `chat_send_feedback`
- [ ] Tauri events: `chat:status`, `chat:delta`, `chat:done`, `chat:error`
- [ ] TypeScript types and API wrappers in `desktop/src/lib/`
- [ ] 5 streaming unit tests + 6 integration tests pass
- [ ] All existing tests pass (no regressions)
- [ ] `cargo clippy` and `cargo fmt` clean
- [ ] `npm run check` passes
- [ ] ADR-0025 updated with Implementation Notes

---

## Impact Analysis

### Structural Changes and Consumers

| Change | Direct Consumers | Transitive Consumers |
|--------|------------------|---------------------|
| `ChatPipeline` (new) | Tauri `chat_*` commands | Desktop chat view (frontend), future MCP service (IP-014) |
| `AppState` extension | All Tauri commands | Desktop backend |
| 8 new Tauri commands | `desktop/src/lib/api.ts` | `Chat.svelte` view, `command-palette.ts` |
| Tauri events (`chat:*`) | `chat-stream.ts` | `Chat.svelte` view |

### Risk Surface

1. **Pipeline performance:** The pipeline does entity loading, search, and LLM call per message. For long conversations, the cost is repeated work. **Mitigation:** Each step is bounded. The pipeline is a stateless service; no caching is required at launch. Caching can be added as a future optimization.

2. **Streaming cancellation race condition:** The `chat_stop_stream` command sets a cancel flag, but in-flight `reqwest` calls cannot be cancelled mid-stream. **Mitigation:** The cancel flag is checked between stream events. The provider stream is dropped, ending the local iteration. The partial response is preserved.

3. **System prompt template drift:** The template is hardcoded in `prompt.rs`. If the LLM behavior needs to change, the template must be updated and versioned. **Mitigation:** The template is a single function. Future versions can be added as `build_system_prompt_v2()`. The version is recorded in the `Provenance` component of the assistant Message.

4. **Citation accuracy:** Citation extraction depends on the LLM following the system prompt instruction to use `[N]` markers. If the LLM uses citations inconsistently, the "View sources" footer is incomplete. **Mitigation:** The pipeline tracks which entities were available in context and lists them in the footer's `referenced_entities` even if the inline citation is missing.

5. **Tauri event ordering:** Multiple `chat:delta` events may arrive out of order if the LLM sends them concurrently. **Mitigation:** Tauri events are delivered in the order they are emitted by the Rust task. The frontend appends deltas in order.

---

## Implementation Notes

*(Filled in during/after implementation — records deviations, discoveries, decisions made during coding)*
