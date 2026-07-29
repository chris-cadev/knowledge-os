# IP-014: MCP-Compatible Service Architecture and EntityRetrievalService

**Status:** Draft
**ADR(s):** [ADR-0028](../../architecture/adrs/adr-0028.md) (MCP-Compatible Service Architecture for Chat and Entity Retrieval)
**PRD(s):** [PRD-0007](../prds/prd-0007-knowledge-chat-and-universal-import.md) (F5 MCP architectural compatibility)
**Estimated effort:** ~2 days

---

## Context

ADR-0028 establishes a framework-agnostic service layer for chat and entity retrieval. The `ChatPipeline` (defined in IP-011) is already framework-agnostic. This plan adds the `EntityRetrievalService` and verifies that all service types are serializable for future MCP exposure.

**Current state:**
- `core/knowledge-derivation/src/features/chat/pipeline.rs` (from IP-011) defines `ChatPipeline` with port trait dependencies. No Tauri types.
- `core/knowledge-core/src/ports/` has all the port traits this plan composes: `EntityRepository`, `ComponentRepository`, `RelationshipRepository`, `SearchIndex`, `VectorStore`, `TraversalPort`.
- `desktop/src-tauri/src/commands/store.rs` (from IP-011) extends `AppState` with `ChatPipeline`. `AppState` is the single dependency injection point.
- `core/knowledge-core/src/features/entity/entity_impl.rs` defines `Entity`. The service returns view-specific response types (e.g., `EntityDetail`) that do not expose `Entity` directly to consumers — they expose only the data the consumer needs.

**Dependencies:**
- IP-009 (ChatCompletion trait) — `ChatPipeline` depends on this
- IP-010 (Conversation entities) — `EntityRetrievalService` is a coordinator over the same port traits
- IP-011 (Chat pipeline) — `ChatPipeline` is already framework-agnostic; this plan verifies the constraint

This plan is the final piece of the chat infrastructure. It can be developed in parallel with IP-012 (OCR) and IP-013 (Universal Import). After this plan, the system is ready for a future MCP server implementation (out of scope for this PRD).

---

## Deliverables

### D1: EntityRetrievalService

**Purpose:** Create a coordinator service that aggregates entity, component, and relationship loading for retrieval (used by chat pipeline, future MCP, future REST API).

**Files:**

| File | Action | Description |
|------|--------|-------------|
| `core/knowledge-core/src/services/mod.rs` | Create | `services` module |
| `core/knowledge-core/src/services/entity_retrieval.rs` | Create | `EntityRetrievalService` struct and methods |
| `core/knowledge-core/src/lib.rs` | Modify | Re-export `services` module |

**Implementation:**

```rust
// core/knowledge-core/src/services/entity_retrieval.rs
use std::collections::BTreeMap;
use async_trait::async_trait;
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::ports::{
    ComponentRepository, EntityRepository, RelationshipRepository, SearchIndex,
    SearchQuery, StorageError, TraversalConfig, TraversalDirection, TraversalPort,
    TraversalQuery,
};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RetrievalFilter {
    pub entity_types: Option<Vec<String>>,
    pub tags: Option<Vec<String>>,
    pub limit: Option<usize>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EntitySummary {
    pub id: Uuid,
    pub entity_type: String,
    pub title: String,
    pub preview: String,
    pub tags: Vec<String>,
    pub updated_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EntityDetail {
    pub id: Uuid,
    pub entity_type: String,
    pub components: BTreeMap<String, serde_json::Value>,
    pub relationships: Vec<RelationshipSummary>,
    pub events: Vec<EventSummary>,
    pub version: i64,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
    pub is_active: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RelationshipSummary {
    pub id: Uuid,
    pub relationship_type: String,
    pub direction: RelationshipDirection,
    pub peer_id: Uuid,
    pub peer_type: String,
    pub peer_title: String,
    pub is_active: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum RelationshipDirection { Outgoing, Incoming }

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EventSummary {
    pub id: Uuid,
    pub event_type: String,
    pub timestamp: DateTime<Utc>,
    pub data: serde_json::Value,
}

pub struct EntityRetrievalService {
    entity_repo: Arc<dyn EntityRepository>,
    component_repo: Arc<dyn ComponentRepository>,
    relationship_repo: Arc<dyn RelationshipRepository>,
    search_index: Arc<dyn SearchIndex>,
    traversal_port: Arc<dyn TraversalPort>,
}

impl EntityRetrievalService {
    pub fn new(
        entity_repo: Arc<dyn EntityRepository>,
        component_repo: Arc<dyn ComponentRepository>,
        relationship_repo: Arc<dyn RelationshipRepository>,
        search_index: Arc<dyn SearchIndex>,
        traversal_port: Arc<dyn TraversalPort>,
    ) -> Self {
        Self { entity_repo, component_repo, relationship_repo, search_index, traversal_port }
    }

    /// Get a single entity with all its components and relationships.
    pub async fn get_entity(&self, id: Uuid) -> Result<EntityDetail, StorageError> {
        let entity = self.entity_repo.get(id).await?
            .ok_or(StorageError::NotFound)?;

        let components_list = self.component_repo.get(id).await?;
        let mut components = BTreeMap::new();
        for c in components_list {
            components.insert(format!("{:?}", c.component_type), c.data);
        }

        let outgoing = self.relationship_repo.by_source(id).await?;
        let incoming = self.relationship_repo.by_target(id).await?;
        let mut relationships = Vec::new();
        for r in outgoing {
            let peer = self.entity_repo.get(r.target_id).await?;
            relationships.push(RelationshipSummary {
                id: r.id,
                relationship_type: format!("{:?}", r.relationship_type),
                direction: RelationshipDirection::Outgoing,
                peer_id: r.target_id,
                peer_type: peer.as_ref().map(|e| e.entity_type.to_string()).unwrap_or_default(),
                peer_title: extract_title(&peer, &self.component_repo).await,
                is_active: r.is_active,
            });
        }
        for r in incoming {
            let peer = self.entity_repo.get(r.source_id).await?;
            relationships.push(RelationshipSummary {
                id: r.id,
                relationship_type: format!("{:?}", r.relationship_type),
                direction: RelationshipDirection::Incoming,
                peer_id: r.source_id,
                peer_type: peer.as_ref().map(|e| e.entity_type.to_string()).unwrap_or_default(),
                peer_title: extract_title(&peer, &self.component_repo).await,
                is_active: r.is_active,
            });
        }

        let events = self.fetch_events(id).await?;

        Ok(EntityDetail {
            id: entity.id,
            entity_type: entity.entity_type.to_string(),
            components,
            relationships,
            events,
            version: entity.version,
            created_at: entity.created_at,
            updated_at: entity.updated_at,
            is_active: entity.is_active,
        })
    }

    /// Get multiple entities (batch) — returns summary list.
    pub async fn get_entities(
        &self,
        ids: &[Uuid],
    ) -> Result<Vec<EntitySummary>, StorageError> {
        let mut results = Vec::with_capacity(ids.len());
        for id in ids {
            if let Some(summary) = self.entity_to_summary(*id).await? {
                results.push(summary);
            }
        }
        Ok(results)
    }

    /// Search entities by query with optional filters.
    pub async fn search(
        &self,
        query: &str,
        filter: &RetrievalFilter,
    ) -> Result<Vec<EntitySummary>, StorageError> {
        let mut search_query = SearchQuery {
            query: query.to_string(),
            entity_type: filter.entity_types.as_ref().and_then(|t| t.first().cloned()),
            tag: filter.tags.as_ref().and_then(|t| t.first().cloned()),
        };
        let results = self.search_index.search(&search_query).await?;
        let limit = filter.limit.unwrap_or(20);

        let mut summaries = Vec::new();
        for r in results.into_iter().take(limit) {
            if let Ok(uuid) = Uuid::parse_str(&r.entity_id.to_string()) {
                if let Some(summary) = self.entity_to_summary(uuid).await? {
                    summaries.push(summary);
                }
            }
        }
        Ok(summaries)
    }

    /// Traverse the relationship graph from a start entity.
    pub async fn traverse(
        &self,
        start: Uuid,
        max_depth: u32,
    ) -> Result<TraversalResult, StorageError> {
        let query = TraversalQuery {
            start_id: start,
            direction: TraversalDirection::Both,
            max_depth: Some(max_depth),
            max_results: Some(100),
            relationship_type: None,
            entity_type_filter: None,
        };
        let config = TraversalConfig::default();
        let results = self.traversal_port.traverse(&query, &config).await?;
        Ok(TraversalResult {
            start_id: start,
            results: results.into_iter().map(|r| r.path).collect(),
        })
    }

    // --- Private helpers ---

    async fn entity_to_summary(&self, id: Uuid) -> Result<Option<EntitySummary>, StorageError> {
        let entity = match self.entity_repo.get(id).await? {
            Some(e) => e,
            None => return Ok(None),
        };
        let components = self.component_repo.get(id).await?;
        let title = extract_title_from_components(&components);
        let preview = extract_preview_from_components(&components);
        let tags = extract_tags_from_components(&components);
        Ok(Some(EntitySummary {
            id: entity.id,
            entity_type: entity.entity_type.to_string(),
            title,
            preview,
            tags,
            updated_at: entity.updated_at,
        }))
    }

    async fn fetch_events(&self, _id: Uuid) -> Result<Vec<EventSummary>, StorageError> {
        // EventLog port is queried separately by the desktop Tauri command
        // (which has access to it). The service can be extended to include events
        // when EventLog is added to its dependencies.
        Ok(vec![])
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TraversalResult {
    pub start_id: Uuid,
    pub results: Vec<Vec<Uuid>>,
}

// --- Helper functions ---

async fn extract_title(
    entity: &Option<crate::features::entity::Entity>,
    component_repo: &Arc<dyn ComponentRepository>,
) -> String {
    match entity {
        Some(e) => {
            let comps = component_repo.get(e.id).await.unwrap_or_default();
            extract_title_from_components(&comps)
        }
        None => String::new(),
    }
}

fn extract_title_from_components(components: &[crate::features::component::Component]) -> String {
    components.iter()
        .find(|c| matches!(c.component_type, crate::features::component::ComponentType::Title))
        .and_then(|c| c.data.get("name").and_then(|v| v.as_str()))
        .unwrap_or("Untitled")
        .to_string()
}

fn extract_preview_from_components(components: &[crate::features::component::Component]) -> String {
    components.iter()
        .find(|c| matches!(c.component_type, crate::features::component::ComponentType::Content))
        .and_then(|c| c.data.get("markdown").and_then(|v| v.as_str()))
        .map(|s| s.chars().take(200).collect())
        .unwrap_or_default()
}

fn extract_tags_from_components(components: &[crate::features::component::Component]) -> Vec<String> {
    components.iter()
        .find(|c| matches!(c.component_type, crate::features::component::ComponentType::Tags))
        .and_then(|c| c.data.get("values").and_then(|v| v.as_array()))
        .map(|arr| arr.iter().filter_map(|v| v.as_str().map(String::from)).collect())
        .unwrap_or_default()
}
```

**Tests** (in same file):
- `get_entity_returns_components_map`
- `get_entity_includes_outgoing_relationships`
- `get_entity_includes_incoming_relationships`
- `get_entity_not_found_returns_error`
- `get_entities_batch_returns_all`
- `search_returns_filtered_results`
- `search_respects_limit`
- `traverse_returns_paths`

**Verification:**
- `cargo test -p knowledge-core` passes with 8 new tests
- `cargo test -p knowledge-storage` still passes (no regressions)

**Exit criteria:** `EntityRetrievalService` is functional and tested.

---

### D2: Verify ChatPipeline Framework-Agnostic Constraint

**Purpose:** Verify the `ChatPipeline` (from IP-011) has no Tauri or webview dependencies and is consumable from any Rust context.

**Files:**

| File | Action | Description |
|------|--------|-------------|
| `core/knowledge-derivation/src/features/chat/pipeline.rs` | Modify | Add a `#[test]` that constructs the pipeline in isolation (no Tauri runtime, no webview) |
| `core/knowledge-derivation/tests/chat_standalone.rs` | Create | Integration test that constructs the pipeline with only port traits, no Tauri types |

**Test:**

```rust
// core/knowledge-derivation/tests/chat_standalone.rs
use std::sync::Arc;
use knowledge_core::ports::chat::*;
use knowledge_core::ports::*;
use knowledge_derivation::features::chat::{pipeline::ChatPipeline, mock::MockChatAdapter};

#[tokio::test]
async fn chat_pipeline_constructs_without_tauri() {
    // Use mock implementations of all port traits
    let chat_provider = Arc::new(MockChatAdapter::default());
    // ... construct mock entity_repo, component_repo, etc.

    let pipeline = ChatPipeline::new(
        chat_provider,
        entity_repo,
        component_repo,
        relationship_repo,
        search_index,
        vector_store,
    );

    // Verify the pipeline can be called without Tauri runtime
    let result = pipeline.chat(None, "test", &[], &SourceToggles::default(), ResponseMode::Fast).await;
    assert!(result.is_ok());
}
```

**Verification:**
- `cargo test -p knowledge-derivation --test chat_standalone` passes
- The `ChatPipeline` source contains no `use tauri` or `tauri::` references (grep verification)

**Exit criteria:** `ChatPipeline` is verified framework-agnostic.

---

### D3: Service Type Serialization Suite

**Purpose:** Verify all service types derive `Serialize`/`Deserialize` and round-trip through JSON without loss.

**Files:**

| File | Action | Description |
|------|--------|-------------|
| `core/knowledge-core/src/services/entity_retrieval.rs` | Modify | Add `#[cfg(test)]` serialization tests for all service types |
| `core/knowledge-derivation/src/features/chat/pipeline.rs` | Modify | Add `#[cfg(test)]` serialization tests for `ChatRequest`, `ChatResponse`, `ChatDelta` |

**Tests:**

```rust
#[test]
fn entity_summary_serializes() {
    let s = EntitySummary {
        id: Uuid::new_v4(),
        entity_type: "Paper".into(),
        title: "Test".into(),
        preview: "Preview".into(),
        tags: vec!["a".into(), "b".into()],
        updated_at: Utc::now(),
    };
    let json = serde_json::to_string(&s).unwrap();
    let parsed: EntitySummary = serde_json::from_str(&json).unwrap();
    assert_eq!(parsed.id, s.id);
}

#[test]
fn entity_detail_serializes_with_components() {
    let mut components = BTreeMap::new();
    components.insert("Title".to_string(), serde_json::json!({"name": "Test"}));
    let d = EntityDetail {
        id: Uuid::new_v4(),
        entity_type: "Paper".into(),
        components,
        relationships: vec![],
        events: vec![],
        version: 1,
        created_at: Utc::now(),
        updated_at: Utc::now(),
        is_active: true,
    };
    let json = serde_json::to_string(&d).unwrap();
    let parsed: EntityDetail = serde_json::from_str(&json).unwrap();
    assert_eq!(parsed.components.len(), 1);
}

// Similar tests for ChatRequest, ChatResponse, ChatDelta, EntityContext, CitationSource
```

**Verification:**
- `cargo test -p knowledge-core` passes with new serialization tests (5+)
- `cargo test -p knowledge-derivation` passes with new serialization tests (3+)

**Exit criteria:** All service types are JSON-roundtrippable.

---

### D4: AppState Extension and Service Wiring

**Purpose:** Wire `EntityRetrievalService` and `ChatPipeline` into `AppState` in the desktop Tauri backend. Both services are constructed in `lib.rs` setup and held in `AppState`.

**Files:**

| File | Action | Description |
|------|--------|-------------|
| `desktop/src-tauri/src/commands/store.rs` | Modify | Add `chat_pipeline: Arc<ChatPipeline>` and `entity_retrieval: Arc<EntityRetrievalService>` to `AppState` |
| `desktop/src-tauri/src/lib.rs` | Modify | Construct services in setup; pass `Arc<SqliteStore>` (which implements all port traits) to each service constructor |

**Implementation:**

```rust
// desktop/src-tauri/src/commands/store.rs
pub struct AppState {
    pub store: Arc<Mutex<SqliteStore>>,
    pub chat_pipeline: Arc<ChatPipeline>,
    pub entity_retrieval: Arc<EntityRetrievalService>,
    pub chat_provider_kind: String,
}
```

**Construction in `lib.rs`:**

```rust
.setup(|app| {
    // ... existing setup

    // Construct services
    let entity_repo: Arc<dyn EntityRepository> = store.clone() as Arc<dyn EntityRepository>;
    let component_repo: Arc<dyn ComponentRepository> = store.clone() as Arc<dyn ComponentRepository>;
    let relationship_repo: Arc<dyn RelationshipRepository> = store.clone() as Arc<dyn RelationshipRepository>;
    let search_index: Arc<dyn SearchIndex> = store.clone() as Arc<dyn SearchIndex>;
    let vector_store: Arc<dyn VectorStore> = store.clone() as Arc<dyn VectorStore>;
    let traversal_port: Arc<dyn TraversalPort> = store.clone() as Arc<dyn TraversalPort>;

    let chat_provider: Arc<dyn ChatCompletion> = create_chat_provider(&provider_config)
        .map_err(|e| format!("failed to create chat provider: {}", e))?;
    let chat_provider_kind = provider_config.to_string();

    let chat_pipeline = Arc::new(ChatPipeline::new(
        chat_provider,
        entity_repo.clone(),
        component_repo.clone(),
        relationship_repo.clone(),
        search_index.clone(),
        vector_store.clone(),
    ));

    let entity_retrieval = Arc::new(EntityRetrievalService::new(
        entity_repo,
        component_repo,
        relationship_repo,
        search_index,
        traversal_port,
    ));

    app.manage(AppState {
        store,
        chat_pipeline,
        entity_retrieval,
        chat_provider_kind,
    });
    Ok(())
})
```

**Note:** The `Arc<SqliteStore>` must be cast to `Arc<dyn PortTrait>` for each port. This is a common Rust pattern but requires explicit conversion. The `StoreWrapper` pattern from IP-008 can be reused, or each port is implemented directly on `Arc<SqliteStore>`.

**Verification:**
- `cargo check -p knowledge-desktop` compiles
- `cargo tauri dev` launches, services are wired

**Exit criteria:** Services are constructed and held in `AppState`.

---

### D5: Service Usage from Tauri Commands

**Purpose:** Refactor existing Tauri commands to use the services instead of calling port traits directly.

**Files:**

| File | Action | Description |
|------|--------|-------------|
| `desktop/src-tauri/src/commands/entity.rs` | Modify | `get_entity_detail` uses `EntityRetrievalService::get_entity()` instead of calling `EntityRepository::get` + `ComponentRepository::get` + `RelationshipRepository::by_source/by_target` directly |
| `desktop/src-tauri/src/commands/search.rs` | Modify | `search_entities` uses `EntityRetrievalService::search()` instead of calling `SearchIndex::search` directly |
| `desktop/src-tauri/src/commands/view.rs` | Modify | `get_graph_view` uses `EntityRetrievalService::traverse()` instead of calling `TraversalPort::traverse` directly |

**Note:** This is a refactor. Existing behavior is preserved. The benefit is that the same `EntityRetrievalService` can be used by future transports (MCP, REST) without code duplication.

**Tests** (existing Tauri command tests must continue to pass):
- All command tests in `knowledge-desktop` pass after refactor

**Verification:**
- `cargo test -p knowledge-desktop` passes
- `cargo test --workspace` passes (no regressions)
- `cargo tauri dev` still launches and works

**Exit criteria:** Tauri commands use services. The same service code path is reusable for future MCP server.

---

## Execution Order

```
D1 (service) -> D2 (verify pipeline) -> D3 (serialization) -> D4 (AppState) -> D5 (commands)
```

D1 is the service implementation. D2 verifies the existing pipeline is framework-agnostic. D3 verifies serializability. D4 wires the services into the desktop backend. D5 refactors existing commands to use the services.

---

## Verification Strategy

| Level | Command | Coverage |
|-------|---------|----------|
| Unit | `cargo test -p knowledge-core` | Service implementation, serialization |
| Integration | `cargo test -p knowledge-derivation --test chat_standalone` | Pipeline framework-agnosticism |
| Unit | `cargo test -p knowledge-desktop` | Command refactor |
| Lint | `cargo clippy --all-targets --all-features -- -D warnings && cargo fmt --check` | Code quality |
| Manual | `cargo tauri dev` | Desktop app works |

---

## Exit Criteria

- [ ] `EntityRetrievalService` in `core/knowledge-core/src/services/entity_retrieval.rs`
- [ ] `ChatPipeline` verified framework-agnostic (no Tauri imports)
- [ ] All service types derive `Serialize`/`Deserialize` and round-trip
- [ ] `AppState` includes `chat_pipeline` and `entity_retrieval` services
- [ ] Tauri commands refactored to use services
- [ ] 8 service unit tests + 3+ serialization tests + 1 standalone test pass
- [ ] All existing tests pass (no regressions)
- [ ] `cargo clippy` and `cargo fmt` clean
- [ ] ADR-0028 updated with Implementation Notes

---

## Impact Analysis

### Structural Changes and Consumers

| Change | Direct Consumers | Transitive Consumers |
|--------|------------------|---------------------|
| `EntityRetrievalService` (new) | Tauri commands, `ChatPipeline` (via existing `EntityRepository`), future MCP server, future REST API | Desktop backend, future external integrations |
| `services/` module (new) | All consumers needing entity retrieval with components/relationships | Future transport-specific code |
| `AppState` extension | Tauri commands | Desktop backend startup |
| Command refactor | Future MCP, REST | Same service code path |

### Risk Surface

1. **`EntityRetrievalService` as a service layer:** Adding a service layer between Tauri commands and port traits increases indirection. **Mitigation:** The service is a thin coordinator that calls port traits. The added value is unifying entity/component/relationship loading for retrieval, which is used by 4+ commands.

2. **Arc<SqliteStore> to Arc<dyn PortTrait> conversion:** This requires explicit casting and may have a small overhead. **Mitigation:** The conversion is a one-time cost at AppState construction. Subsequent calls through the trait object are zero-cost.

3. **Service types stability:** The service types are the contract for future MCP and REST. Breaking changes propagate to all transports. **Mitigation:** Type versioning through module paths (`v1`, `v2`). Initial release is `v1`. Breaking changes require a new `v2` module.

4. **Refactor regression risk:** Changing Tauri commands to use services could break existing behavior. **Mitigation:** The refactor preserves the existing command signatures and response types. Existing tests must continue to pass.

5. **Event log not in service:** The current `EntityRetrievalService` does not include the event log (would require adding `EventLog` to its dependencies). **Mitigation:** The Tauri command can still call `EventLog::list_by_entity` directly. A future iteration can add events to the service.

---

## Implementation Notes

*(Filled in during/after implementation — records deviations, discoveries, decisions made during coding)*
