# IP-010: Conversation and Message Canonical Entities

**Status:** Draft
**ADR(s):** [ADR-0024](../../architecture/adrs/adr-0024.md) (Conversation and Message as Canonical Entities)
**PRD(s):** [PRD-0007](../prds/prd-0007-knowledge-chat-and-universal-import.md) (F2.2–F2.5 conversation history, F2.12–F2.18 citations, F2.20 persistence)
**Estimated effort:** ~4 days

---

## Context

ADR-0024 extends the canonical model with two new entity types (`Conversation`, `Message`), two new component types (`MessageContent`, `EntityRefs`), two new relationship types (`has_message`, `referenced_by`), and four new events. The chat pipeline (IP-011) writes to these entities; the desktop chat sidebar (IP-011 D4) lists and navigates them.

**Current state:**
- `core/knowledge-core/src/features/entity/entity_type.rs` defines `EntityType` as a wrapper around `String` with a `KNOWN_TYPES` list. Adding `"Conversation"` and `"Message"` is a one-line extension.
- `core/knowledge-core/src/features/component/mod.rs` defines `ComponentType` as a closed enum (10 variants: `Title`, `Description`, `Content`, `BinaryContent`, `Tags`, `Author`, `Embedding`, `Summary`, `Timeline`, `Language`, `Provenance`). Adding `MessageContent` and `EntityRefs` requires extending the enum.
- `core/knowledge-core/src/features/relationship/mod.rs` defines `RelationshipType` as a closed enum with a single variant `References`. Adding `HasMessage` and `ReferencedBy` requires extending the enum.
- `core/knowledge-core/src/ports/event.rs` defines `EventType` with 9 variants. Adding `ConversationCreated`, `MessageCreated`, `EntityReferenced`, and `ChatContextRetrieved` requires extending the enum.
- `core/knowledge-storage` implements `EntityRepository`, `ComponentRepository`, `RelationshipRepository`, and `EventLog`. The SQLite adapter stores `entity_type`, `component_type`, `relationship_type`, and `event_type` as strings. New type variants are picked up automatically by the string-based serialization.
- `cli/src/main.rs` has commands for entity, import, view, traversal, search. New `kos conversation` commands are needed for listing, showing, renaming, and deleting conversations.
- `desktop/src-tauri/src/commands/entity.rs` has `list_entities`, `get_entity_detail`, `get_entity_source`. New chat-specific commands are added in IP-011 D4.

This plan extends the domain model and storage layer. The chat pipeline that writes to these entities is in IP-011. The desktop UI that displays them is in IP-011 D4.

---

## Deliverables

### D1: Extend Domain Model — EntityType, ComponentType, RelationshipType, EventType

**Purpose:** Add the new types to the closed enums. No behavioral change to existing code.

**Files:**

| File | Action | Description |
|------|--------|-------------|
| `core/knowledge-core/src/features/entity/entity_type.rs` | Modify | Add `"Conversation"` and `"Message"` to `KNOWN_TYPES` |
| `core/knowledge-core/src/features/component/mod.rs` | Modify | Add `MessageContent` and `EntityRefs` variants to `ComponentType` |
| `core/knowledge-core/src/features/relationship/mod.rs` | Modify | Add `HasMessage` and `ReferencedBy` variants to `RelationshipType` |
| `core/knowledge-core/src/ports/event.rs` | Modify | Add 4 new `EventType` variants |

**Edits:**

```rust
// core/knowledge-core/src/features/entity/entity_type.rs
pub const KNOWN_TYPES: &'static [&'static str] = &[
    "Concept", "Person", "Organization", "Project", "Book", "Paper", "Video",
    "Article", "Tool", "Technology", "Question", "Idea", "Event", "Skill",
    "Location", "Dataset", "Collection", "Workspace", "Decision", "Note",
    "Conversation",  // NEW
    "Message",       // NEW
];
```

```rust
// core/knowledge-core/src/features/component/mod.rs
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum ComponentType {
    Title, Description, Content, BinaryContent, Tags, Author,
    Embedding, Summary, Timeline, Language, Provenance,
    MessageContent,  // NEW
    EntityRefs,      // NEW
}
```

```rust
// core/knowledge-core/src/features/relationship/mod.rs
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum RelationshipType {
    References,
    HasMessage,    // NEW
    ReferencedBy,  // NEW
}
```

```rust
// core/knowledge-core/src/ports/event.rs
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum EventType {
    EntityCreated, EntityUpdated, EntityArchived, EntityRestored, EntityResolved,
    ComponentAdded, ComponentUpdated, ComponentRemoved,
    RelationshipCreated, RelationshipArchived,
    ConversationCreated,  // NEW
    MessageCreated,       // NEW
    EntityReferenced,     // NEW
    ChatContextRetrieved, // NEW
}
```

**Verification:**
- `cargo check -p knowledge-core` compiles
- `cargo test -p knowledge-core` passes (existing tests in `entity_impl.rs` still work)
- Existing string-based entity types in the SQLite store automatically recognize `"Conversation"` and `"Message"` because `EntityType::new()` already accepts arbitrary strings

**Exit criteria:** All four enums extended, no regressions.

---

### D2: Add MessageContent and EntityRefs Component Payload Types

**Purpose:** Define the typed payload structures for the new component types.

**Files:**

| File | Action | Description |
|------|--------|-------------|
| `core/knowledge-core/src/features/component/payloads.rs` | Create | `MessageContentData`, `EntityRefsData` payload structs with `From<&Component>` accessors |
| `core/knowledge-core/src/features/component/mod.rs` | Modify | Re-export payload types from `payloads` module |

**New types:**

```rust
// core/knowledge-core/src/features/component/payloads.rs
use serde::{Deserialize, Serialize};
use uuid::Uuid;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct MessageContentData {
    pub role: MessageRole,
    pub text: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum MessageRole { System, User, Assistant }

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct EntityRefsData {
    pub entity_ids: Vec<Uuid>,
}
```

The payload is stored as `serde_json::Value` in the existing `Component.data` field. Accessor methods on `Component` parse the JSON back to the typed payload:

```rust
impl Component {
    pub fn message_content(&self) -> Option<MessageContentData> { /* ... */ }
    pub fn entity_refs(&self) -> Option<EntityRefsData> { /* ... */ }
}
```

**Tests** (in `payloads.rs`, `#[cfg(test)] mod tests`):
- `component_message_content_roundtrip` — create component, serialize to JSON, parse back
- `component_entity_refs_roundtrip` — same for entity refs
- `component_wrong_type_returns_none` — `Content` component does not parse as `MessageContent`
- `message_role_serializes_lowercase` — `MessageRole::User` serializes as `"user"`

**Verification:**
- `cargo test -p knowledge-core` passes with new 4 tests
- All existing component tests still pass

**Exit criteria:** Payload types defined, round-trip serialization works, type discrimination enforced.

---

### D3: Storage Layer — Conversation and Message Repositories

**Purpose:** Add repository methods for the conversation-specific operations (list, rename, delete with cascading archive) on top of the existing `EntityRepository`.

**Files:**

| File | Action | Description |
|------|--------|-------------|
| `core/knowledge-core/src/ports/conversation.rs` | Create | `ConversationRepository` trait (depends on existing port traits) |
| `core/knowledge-core/src/ports/mod.rs` | Modify | Add `pub mod conversation; pub use conversation::*;` |
| `core/knowledge-storage/src/adapters/sqlite/conversation.rs` | Create | SQLite implementation of `ConversationRepository` |
| `core/knowledge-storage/src/adapters/sqlite/mod.rs` | Modify | Re-export conversation module, add `ConversationRepository` to `SqliteStore` impls |
| `core/knowledge-storage/tests/conversation_integration.rs` | Create | Integration tests for conversation operations |

**New trait:**

```rust
// core/knowledge-core/src/ports/conversation.rs
use async_trait::async_trait;
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use super::error::StorageError;

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

#[async_trait]
pub trait ConversationRepository: Send + Sync {
    /// List all non-archived conversations, sorted by most recent activity.
    async fn list_conversations(&self) -> Result<Vec<ConversationSummary>, StorageError>;

    /// Load a single conversation with all its messages, ordered by message creation time.
    async fn get_conversation(
        &self,
        conversation_id: Uuid,
    ) -> Result<Option<ConversationDetail>, StorageError>;

    /// Rename a conversation by updating its Title component.
    async fn rename_conversation(
        &self,
        conversation_id: Uuid,
        new_title: &str,
    ) -> Result<(), StorageError>;

    /// Archive a conversation and all its messages.
    /// Does NOT hard-delete (per ADR-0001).
    async fn archive_conversation(
        &self,
        conversation_id: Uuid,
    ) -> Result<(), StorageError>;
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
    pub created_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum MessageRole { System, User, Assistant }

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CitationSource {
    pub number: usize,
    pub entity_id: Uuid,
    pub entity_type: String,
    pub title: String,
    pub snippet: String,
}
```

**SQLite implementation outline** in `core/knowledge-storage/src/adapters/sqlite/conversation.rs`:

```rust
pub struct SqliteConversationRepository<'a> {
    conn: &'a Mutex<Connection>,
}

#[async_trait]
impl ConversationRepository for SqliteConversationRepository<'_> {
    async fn list_conversations(&self) -> Result<Vec<ConversationSummary>, StorageError> {
        let conn = self.conn.lock().await;
        // SQL: SELECT conversation entities joined with COUNT(messages) and MAX(message.created_at)
        // WHERE entity_type = 'Conversation' AND is_active = 1
        // ORDER BY last_message_at DESC
        todo!()
    }
    // ... other methods
}
```

**Note on `Mutex<Connection>`:** The existing `SqliteStore` uses `tokio::sync::Mutex<Connection>` (per IP-008 D1). The `ConversationRepository` borrows the connection. The `SqliteStore` itself implements the trait by passing `&self.conn` to the helper struct.

**Integration tests** in `core/knowledge-storage/tests/conversation_integration.rs`:
- `list_conversations_returns_empty_for_no_data` — empty store returns `vec![]`
- `list_conversations_sorts_by_recency` — most recent conversation first
- `list_conversations_excludes_archived` — archived conversations not in list
- `get_conversation_loads_messages_ordered` — messages in creation order
- `rename_conversation_updates_title` — title component updated, version increments
- `archive_conversation_marks_inactive` — `is_active = false` for conversation and messages
- `archive_conversation_cascades_to_messages` — all `Message` entities in conversation are archived
- `get_conversation_returns_none_for_missing` — nonexistent id returns `Ok(None)`

**Verification:**
- `cargo test -p knowledge-storage` passes with 8 new integration tests
- `cargo test -p knowledge-core` still passes (no regressions)
- `cargo test -p knowledge-derivation` still passes (existing view adapters that filter by entity type continue to work)

**Exit criteria:** `ConversationRepository` trait and SQLite implementation, all 8 integration tests pass.

---

### D4: CLI Conversation Commands

**Purpose:** Expose conversation operations through the `kos conversation` subcommand.

**Files:**

| File | Action | Description |
|------|--------|-------------|
| `cli/src/main.rs` | Modify | Add `Conversation` subcommand enum and handler |
| `cli/features/prd-0007/conversation.feature` | Create | BDD scenarios for conversation operations |
| `cli/tests/cucumber.rs` | Modify | Add step definitions for conversation operations |

**CLI interface (from PRD-0007 §CLI Interface):**

```bash
kos conversation list                            # List all conversations
kos conversation get <id>                       # Show conversation with messages
kos conversation rename <id> <new-title>         # Rename a conversation
kos conversation delete <id>                     # Archive a conversation
```

**Command implementation in `cli/src/main.rs`:**

```rust
#[derive(Subcommand)]
enum ConversationCommands {
    List,
    Get { id: String },
    Rename { id: String, title: String },
    Delete { id: String },
}

// In main():
Commands::Conversation { command } => match command {
    ConversationCommands::List => {
        let conversations = store.list_conversations().await?;
        for c in conversations {
            println!("{} ({} messages) - {}",
                c.title, c.message_count,
                c.last_message_at.map(|d| d.to_rfc3339()).unwrap_or_default());
        }
    }
    ConversationCommands::Get { id } => {
        let id = Uuid::parse_str(&id)?;
        let conv = store.get_conversation(id).await?
            .ok_or_else(|| anyhow!("conversation not found"))?;
        println!("# {}", conv.title);
        for msg in conv.messages {
            println!("\n[{}] {}", msg.role, msg.text);
        }
    }
    ConversationCommands::Rename { id, title } => { /* ... */ }
    ConversationCommands::Delete { id } => { /* ... */ }
}
```

**BDD scenarios:**

```gherkin
Feature: Conversation CLI
  Scenario: List empty
    Given an empty knowledge base
    When I run "kos conversation list"
    Then the output is empty

  Scenario: List shows recent first
    Given a conversation "Q3 review" with 5 messages
    And a conversation "Onboarding" with 2 messages
    When I run "kos conversation list"
    Then the output contains "Q3 review" before "Onboarding"

  Scenario: Rename conversation
    Given a conversation "Old name"
    When I run "kos conversation rename <id> New name"
    Then the conversation title is "New name"

  Scenario: Delete conversation archives it
    Given a conversation "Test" with 3 messages
    When I run "kos conversation delete <id>"
    Then the conversation is not in "kos conversation list"
    And all 3 messages are archived
```

**Verification:**
- `cargo test --test cucumber -p knowledge-cli` passes with 4 new BDD scenarios
- `cargo test -p knowledge-cli` passes (no regressions)

**Exit criteria:** CLI commands work end-to-end, BDD tests pass.

---

### D5: Domain Model Documentation Update

**Purpose:** Update `docs/architecture/domain-model.md` and `docs/architecture/events.md` to reflect the new types.

**Files:**

| File | Action | Description |
|------|--------|-------------|
| `docs/architecture/domain-model.md` | Modify | Add `Conversation` and `Message` to Core Entity Types table; add `MessageContent` and `EntityRefs` to Core Components table; add `has_message` and `referenced_by` to Structural Relationships table |
| `docs/architecture/events.md` | Modify | Add `ConversationCreated`, `MessageCreated`, `EntityReferenced` to Canonical Events table; add `ChatContextRetrieved` to Derivation Events table |

**Edits follow the existing table format:**

```markdown
| `Conversation` | A series of messages between user and AI | "Q3 research review" |
| `Message`      | A single turn in a conversation          | A user question or an AI response |
```

```markdown
| `MessageContent` | `{ role: "user"|"assistant"|"system", text: string }` | Text content of a chat message |
| `EntityRefs`     | `{ entity_ids: Uuid[] }`                             | Entities referenced in a message |
```

```markdown
| `has_message`   | Conversation | Message        | The conversation contains this message |
| `referenced_by` | Any entity   | Message        | The entity is referenced by this message |
```

**Verification:**
- Markdown links resolve (no broken cross-references)
- File naming follows `kebab-case.md`
- Tables use the same column structure as the rest of the document

**Exit criteria:** Documentation accurately reflects the domain model extension.

---

## Execution Order

```
D1 (enums) -> D2 (payloads) -> D3 (storage) -> D4 (CLI) -> D5 (docs)
```

D1 is a closed-enum extension with no behavioral change. D2 adds typed payloads. D3 implements the conversation repository. D4 exposes it through the CLI. D5 updates the documentation.

---

## Verification Strategy

| Level | Command | Coverage |
|-------|---------|----------|
| Unit | `cargo test -p knowledge-core` | Enum extension, payload round-trip |
| Integration | `cargo test -p knowledge-storage` | Conversation operations, cascading archive |
| E2E | `cargo test --test cucumber -p knowledge-cli` | CLI conversation commands |
| Lint | `cargo clippy --all-targets --all-features -- -D warnings && cargo fmt --check` | Code quality |
| Documentation | Manual review of `domain-model.md` and `events.md` | Accurate reflection of types |

---

## Exit Criteria

- [ ] `EntityType::KNOWN_TYPES` includes `"Conversation"` and `"Message"`
- [ ] `ComponentType` enum includes `MessageContent` and `EntityRefs`
- [ ] `RelationshipType` enum includes `HasMessage` and `ReferencedBy`
- [ ] `EventType` enum includes 4 new variants
- [ ] `MessageContentData` and `EntityRefsData` payload types with `Component` accessors
- [ ] `ConversationRepository` trait and `SqliteConversationRepository` implementation
- [ ] `kos conversation list|get|rename|delete` commands
- [ ] 4 BDD scenarios pass
- [ ] `docs/architecture/domain-model.md` updated
- [ ] `docs/architecture/events.md` updated
- [ ] All existing tests pass (no regressions)
- [ ] `cargo clippy` and `cargo fmt` clean
- [ ] ADR-0024 updated with Implementation Notes

---

## Impact Analysis

### Structural Changes and Consumers

| Change | Direct Consumers | Transitive Consumers |
|--------|------------------|---------------------|
| `EntityType` extension | `EntityRepository::find_by_type` (filters by "Conversation"/"Message") | All view adapters, search index, traversal |
| `ComponentType` extension | `ComponentRepository` (new variants pass through unchanged) | Storage adapter, search index, embedding pipeline |
| `RelationshipType` extension | `RelationshipRepository` | `TraversalPort`, graph view, citation tracking |
| `EventType` extension | `EventLog` (filtered by new variants) | Chat pipeline (IP-011) |
| `ConversationRepository` (new) | `ChatPipeline` (IP-011) | Tauri commands (IP-011 D4), CLI (D4) |
| `SqliteStore` implements new trait | `AppState` in desktop, CLI `main.rs` | All consumers of `AppState` |

### Risk Surface

1. **Enum extension is a closed-set change:** Adding to a `#[derive(Serialize, Deserialize, PartialEq)]` enum changes the wire format (the new variants appear in serialized data). **Mitigation:** New variants are additive. Old deserializers ignore unknown variants if a `#[serde(other)]` fallback is added in a future version. For now, all crates deserialize the same version.

2. **Cascading archive:** Archiving a conversation must archive all `Message` entities connected via `has_message`. **Mitigation:** The `archive_conversation` method runs in a single SQLite transaction. Foreign-key integrity is not enforced at the storage layer (it is a port-level concern), so the cascading logic must be implemented explicitly.

3. **Conversation sort performance:** `list_conversations` joins entities, components, and relationships. Performance scales with conversation count and message count. **Mitigation:** A `created_at` index on `entities` and a `created_at` index on `components` (for messages) are sufficient for the expected scale (target: < 10K conversations, < 1M messages).

4. **Type changes affect all dependent crates:** Adding variants to a closed enum in `knowledge-core` requires recompilation of all dependent crates. **Mitigation:** This is a one-time cost. The Cargo workspace's incremental compilation handles this efficiently.

5. **`MessageRole` is defined in two places:** This plan adds `MessageRole` to `component/payloads.rs` and `ports/conversation.rs`. **Mitigation:** Move `MessageRole` to a single location (`component/payloads.rs`) and re-export from `ports/conversation.rs`. The duplicate type is a code smell; consolidating prevents drift.

---

## Implementation Notes

*(Filled in during/after implementation — records deviations, discoveries, decisions made during coding)*
