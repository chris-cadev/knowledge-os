# IP-004: Phase 4 -- Semantic Search

**Status:** Draft
**ADR(s):** [ADR-0017](../../architecture/adrs/adr-0017.md) (Semantic Search via Embeddings)
**PRD(s):** [PRD-0003](../prds/prd-0003-graph-exploration-and-plugins.md) (US6: Semantic search)
**Estimated effort:** ~5 days

---

## Context

ADR-0017 chose an embedding pipeline: extract text from entities, generate vectors via configurable AI provider, store in an in-memory vector store, rank by cosine similarity. Hybrid search combines BM25 and cosine via Reciprocal Rank Fusion.

The `AiAdapter` and `VectorStore` traits are defined as stubs in IP-003 D1. This phase refines them with the full API and implements the search infrastructure.

**Prerequisites:** IP-003 (Plugin System) D1 is complete — `AiAdapter` and `VectorStore` stubs exist in `knowledge-core`.

**Dependency:** IP-002's `EventNotifier` trait is used by D5 to trigger embedding regeneration on content updates.

---

## Deliverables

### D1: Refine AiAdapter and VectorStore Traits

**Purpose:** Extend the stub traits from IP-003 D1 with full API surface

**Files:**

| File                                   | Action | Description                                                                                                          |
| -------------------------------------- | ------ | -------------------------------------------------------------------------------------------------------------------- |
| `core/knowledge-core/src/ports/mod.rs` | Modify | Add `dimensions()` to `AiAdapter`, add `VectorFilter`, `VectorMetadata`, `FusedResult`, `rebuild()` to `VectorStore` |

**Refined types (per ADR-0017):**

```rust
// --- AiAdapter (refined from IP-003 stub) ---

#[async_trait]
pub trait AiAdapter: Send + Sync {
    async fn embed(&self, content: &str) -> Result<Vec<f32>, AiError>;
    fn model_name(&self) -> &str;
    fn dimensions(&self) -> usize; // NEW: added in this phase
}

// --- VectorStore (refined from IP-003 stub) ---

#[async_trait]
pub trait VectorStore: Send + Sync {
    async fn upsert(&self, entity_id: &str, vector: &[f32], metadata: Option<VectorMetadata>) -> Result<(), VectorError>;
    async fn search(&self, query: &[f32], k: usize, filter: Option<VectorFilter>) -> Result<Vec<VectorResult>, VectorError>;
    async fn delete(&self, entity_id: &str) -> Result<(), VectorError>;
    async fn rebuild(&self) -> Result<(), VectorError>; // NEW: added in this phase
}

pub struct VectorFilter {
    pub entity_types: Option<Vec<EntityType>>,
    pub tags: Option<Vec<String>>,
    pub min_score: Option<f64>,
}

pub struct VectorMetadata {
    pub model: String,
    pub entity_type: String,
    pub title: String,
}

pub struct FusedResult {
    pub entity_id: String,
    pub score: f64,
}
```

**Breaking change note:** IP-003 D1 defines `VectorStore::upsert` without `metadata` and `search` without `filter`. This D changes the signatures. Since no external consumers exist yet and the stubs are internal, this is acceptable. The `CapabilityRegistry` in `knowledge-plugin` references `Box<dyn VectorStore>` — the updated trait is backward-compatible at the call site.

**Verification:**
- `cargo check -p knowledge-core` compiles
- `cargo test -p knowledge-core` passes

**Exit criteria:** Refined traits compile, existing tests pass

---

### D2: In-Memory Vector Store

**Purpose:** Implement `VectorStore` with brute-force cosine similarity

**Files:**

| File                                                        | Action | Description                          |
| ----------------------------------------------------------- | ------ | ------------------------------------ |
| `core/knowledge-derive/src/features/search/mod.rs`          | Modify | Add `pub mod vector_store;`          |
| `core/knowledge-derive/src/features/search/vector_store.rs` | Create | `InMemoryVectorStore` implementation |

**Implementation (per ADR-0017):**

```rust
use std::collections::HashMap;
use std::sync::RwLock;
use knowledge_core::ports::{VectorStore, VectorMetadata, VectorFilter, VectorResult, VectorError};

pub struct InMemoryVectorStore {
    vectors: RwLock<HashMap<String, Vec<f32>>>,
    metadata: RwLock<HashMap<String, VectorMetadata>>,
}

fn cosine_similarity(a: &[f32], b: &[f32]) -> f64 {
    let dot: f64 = a.iter().zip(b.iter()).map(|(x, y)| (*x as f64) * (*y as f64)).sum();
    let norm_a: f64 = a.iter().map(|x| (*x as f64).powi(2)).sum::<f64>().sqrt();
    let norm_b: f64 = b.iter().map(|x| (*x as f64).powi(2)).sum::<f64>().sqrt();
    if norm_a == 0.0 || norm_b == 0.0 { 0.0 } else { dot / (norm_a * norm_b) }
}
```

Performance: brute-force works for < 100K vectors. Beyond that, HNSW or sqlite-vec can be added as an alternative `VectorStore` implementation without changing the trait.

**Verification:**
- Unit test: cosine_similarity returns correct values for known vectors
- Unit test: cosine_similarity returns 0.0 for zero vectors
- Unit test: cosine_similarity returns 1.0 for identical vectors
- Unit test: cosine_similarity returns 0.0 for orthogonal vectors
- Unit test: upsert stores vector correctly
- Unit test: search returns top-k results sorted by score
- Unit test: search with filter returns only matching results
- Unit test: delete removes vector
- Unit test: rebuild clears and repopulates
- Unit test: dimension mismatch returns error

**Exit criteria:** Vector store works correctly, all unit tests pass

---

### D3: Hybrid Search with RRF

**Purpose:** Implement Reciprocal Rank Fusion to combine keyword and semantic search

**Files:**

| File                                                  | Action | Description                       |
| ----------------------------------------------------- | ------ | --------------------------------- |
| `core/knowledge-derive/src/features/search/hybrid.rs` | Create | `reciprocal_rank_fusion` function |
| `core/knowledge-derive/src/features/search/mod.rs`    | Modify | Add `pub mod hybrid;`             |

**Implementation (per ADR-0017):**

```rust
use knowledge_core::ports::{SearchResult, VectorResult, FusedResult};
use std::collections::HashMap;

pub fn reciprocal_rank_fusion(
    keyword_results: &[SearchResult],
    semantic_results: &[VectorResult],
    k: usize, // RRF constant, typically 60
) -> Vec<FusedResult> {
    let mut scores: HashMap<String, f64> = HashMap::new();

    for (rank, result) in keyword_results.iter().enumerate() {
        let id = result.entity_id.to_string();
        *scores.entry(id).or_insert(0.0) += 1.0 / (k + rank + 1) as f64;
    }
    for (rank, result) in semantic_results.iter().enumerate() {
        *scores.entry(result.entity_id.clone()).or_insert(0.0) += 1.0 / (k + rank + 1) as f64;
    }

    let mut fused: Vec<FusedResult> = scores.into_iter()
        .map(|(id, score)| FusedResult { entity_id: id, score })
        .collect();
    fused.sort_by(|a, b| b.score.partial_cmp(&a.score).unwrap());
    fused
}
```

Uses the existing `SearchResult` type from `knowledge-core/src/ports/mod.rs` (line 84) which has `entity_id: Uuid`. The `reciprocal_rank_fusion` function converts `Uuid` to `String` for the score map.

RRF is preferred over weighted score combination because BM25 scores and cosine similarity scores are on different scales. RRF operates on ranks, which are comparable.

**Verification:**
- Unit test: fusion merges keyword and semantic results correctly
- Unit test: entity appearing in both lists ranks higher
- Unit test: empty input lists handled gracefully
- Unit test: fusion scores in [0, 1] range
- Unit test: results sorted by score descending

**Exit criteria:** RRF fusion works correctly

---

### D4: Mock AI Provider for Testing

**Purpose:** Create a deterministic mock embedder for tests

**Files:**

| File                                                         | Action | Description                                            |
| ------------------------------------------------------------ | ------ | ------------------------------------------------------ |
| `core/knowledge-derive/src/features/search/mock_embedder.rs` | Create | `MockAiAdapter` that produces deterministic embeddings |

**Implementation notes:**

The mock embedder hashes input text to produce deterministic vectors. This allows tests to verify semantic search without depending on external AI providers.

```rust
pub struct MockAiAdapter {
    model: String,
    dimensions: usize,
}

impl AiAdapter for MockAiAdapter {
    async fn embed(&self, content: &str) -> Result<Vec<f32>, AiError> {
        // Deterministic: hash content to produce consistent vectors
        // Similar content produces similar vectors (for testing)
    }
    fn model_name(&self) -> &str { &self.model }
    fn dimensions(&self) -> usize { self.dimensions }
}
```

**Important:** This module is test infrastructure. It must be behind `#[cfg(test)]` or placed in a `tests/` directory to avoid compiling into production code.

```rust
// In features/search/mod.rs
#[cfg(test)]
pub mod mock_embedder;
```

**Verification:**
- Unit test: same input produces same embedding
- Unit test: different inputs produce different embeddings
- Unit test: embedding dimension matches `dimensions()`

**Exit criteria:** Mock embedder works for deterministic testing

---

### D5: Embedding Pipeline Integration

**Purpose:** Wire embedding generation to the import pipeline

**Files:**

| File                                                    | Action | Description                                                           |
| ------------------------------------------------------- | ------ | --------------------------------------------------------------------- |
| `core/knowledge-derive/src/features/search/pipeline.rs` | Create | `EmbeddingPipeline` that generates embeddings from Content components |
| `core/knowledge-derive/src/features/search/mod.rs`      | Modify | Add `pub mod pipeline;`                                               |

**Implementation notes:**

The embedding pipeline:
1. Extract text from entity's Content component
2. Call `AiAdapter::embed()` to generate vector
3. Create Embedding component with vector, model name, and timestamp
4. Store vector in `VectorStore`
5. Attach Embedding component to entity

Pipeline triggers:
- On import: new entities with Content get embeddings generated
- On content update: embeddings are regenerated via `EventNotifier` (from IP-002)
- On rebuild: all embeddings are regenerated from canonical Content components

```rust
pub struct EmbeddingPipeline {
    ai_provider: Box<dyn AiAdapter>,
    vector_store: Box<dyn VectorStore>,
    component_repo: Box<dyn ComponentRepository>,
}
```

**Event integration:** The pipeline implements `EventNotifier` from IP-002. When a `ComponentUpdated` or `EntityCreated` event is received, the pipeline checks if the entity has a Content component and regenerates the embedding.

```rust
#[async_trait]
impl EventNotifier for EmbeddingPipeline {
    async fn notify(&self, event: &Event) -> Result<(), StorageError> {
        match event.event_type {
            EventType::EntityCreated | EventType::ComponentUpdated => {
                // Check if entity has Content component
                // If so, regenerate embedding
            }
            _ => {}
        }
        Ok(())
    }
}
```

**Verification:**
- Integration test: content component produces embedding component
- Integration test: mock embedder generates deterministic embeddings
- Integration test: vector stored in InMemoryVectorStore
- Integration test: embedding regeneration produces same vectors

**Exit criteria:** Embedding pipeline works end-to-end with mock embedder

---

### D6: Search Mode Integration and CLI

**Purpose:** Extend `kos search` with `--semantic` and `--hybrid` flags

**Files:**

| File                                            | Action | Description                                                  |
| ----------------------------------------------- | ------ | ------------------------------------------------------------ |
| `cli/src/main.rs`                               | Modify | Add `--semantic` and `--hybrid` flags to `Search` subcommand |
| `cli/features/prd-0003/semantic-search.feature` | Create | BDD scenarios for semantic and hybrid search                 |
| `cli/tests/cucumber.rs`                         | Modify | Add step definitions for semantic search                     |

**CLI interface (per PRD-0003):**

```
kos search "machine learning" --semantic
kos search "attention mechanism" --hybrid
```

**Search modes (per ADR-0017):**

| Mode       | Behavior                                           |
| ---------- | -------------------------------------------------- |
| `keyword`  | FTS5 BM25 search only (existing behavior, default) |
| `semantic` | Embedding cosine similarity only                   |
| `hybrid`   | Both, fused via RRF                                |

**BDD scenarios:**

```gherkin
Feature: Semantic Search
  As a knowledge worker
  I want to search by meaning, not just keywords
  So that I can find conceptually related entities

  Background:
    Given an empty database

  Scenario: Keyword search still works (default)
    Given a directory with files:
      | filename    | content                              |
      | concept.md  | # Transformer\n\nType: concept       |
    When I run "kos import <directory>"
    And I run "kos search transformer"
    Then the output contains "Transformer"

  Scenario: Semantic search with mock provider
    Given a directory with files:
      | filename    | content                              |
      | concept.md  | # Neural Network\n\nType: concept    |
    When I run "kos import <directory>"
    And I run "kos search 'deep learning' --semantic"
    Then the output should contain results

  Scenario: Hybrid search combines results
    Given a directory with files:
      | filename    | content                              |
      | concept.md  | # Transformer\n\nType: concept       |
    When I run "kos import <directory>"
    And I run "kos search transformer --hybrid"
    Then the output contains "Transformer"

  Scenario: Semantic search without AI provider
    Given no AI provider is configured
    When I run "kos search 'machine learning' --semantic"
    Then the error output should indicate no provider available
```

**Verification:**
- `cargo test --test cucumber -p knowledge-cli` passes
- BDD scenarios: keyword fallback, semantic search, hybrid search, no-provider error
- Manual test: import files, search with `--semantic` and `--hybrid`

**Exit criteria:** Semantic and hybrid search work via CLI, BDD tests pass

---

## Execution Order

```
D1 (refine traits) -> D2 (vector store) -> D3 (RRF) -> D4 (mock embedder) -> D5 (pipeline) -> D6 (CLI)
```

D1 refines the stubs from IP-003. D2-D3 implement core logic independently. D4 provides test infrastructure. D5 wires the pipeline. D6 exposes to CLI.

---

## Verification Strategy

| Level       | Command                                                  | Coverage                         |
| ----------- | -------------------------------------------------------- | -------------------------------- |
| Unit        | `cargo test -p knowledge-derive`                         | Vector store, RRF, mock embedder |
| Integration | `cargo test -p knowledge-derive --test integration_test` | Embedding pipeline               |
| E2E         | `cargo test --test cucumber -p knowledge-cli`            | CLI search commands              |
| Lint        | `cargo clippy -- -D warnings && cargo fmt --check`       | Code quality                     |

---

## Exit Criteria

- [ ] `AiAdapter` refined with `dimensions()` in `knowledge-core`
- [ ] `VectorStore` refined with `metadata`, `filter`, `rebuild()` in `knowledge-core`
- [ ] `InMemoryVectorStore` with brute-force cosine similarity
- [ ] `reciprocal_rank_fusion` function using existing `SearchResult` type
- [ ] `MockAiAdapter` behind `#[cfg(test)]`
- [ ] `EmbeddingPipeline` generates embeddings from Content components
- [ ] `EmbeddingPipeline` implements `EventNotifier` for content update triggers
- [ ] `kos search --semantic` and `--hybrid` flags
- [ ] BDD tests: 4+ semantic search scenarios
- [ ] `cargo clippy -- -D warnings` passes
- [ ] ADR-0017 updated with Implementation Notes

---

## Implementation Notes

*(Filled in during/after implementation)*
