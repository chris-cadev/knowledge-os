# IP-004: Phase 4 -- Semantic Search

**Status:** Draft
**ADR(s):** [ADR-0017](../../architecture/adrs/adr-0017.md) (Semantic Search via Embeddings)
**PRD(s):** [PRD-0003](../prds/prd-0003-graph-exploration-and-plugins.md) (US6: Semantic search)
**Estimated effort:** ~5 days

---

## Context

ADR-0017 chose an embedding pipeline: extract text from entities, generate vectors via configurable AI provider, store in an in-memory vector store, rank by cosine similarity. Hybrid search combines BM25 and cosine via Reciprocal Rank Fusion.

The `AiAdapter` and `VectorStore` traits are plugin capabilities (from IP-003). This phase implements the traits and the search integration.

---

## Deliverables

### D1: AiAdapter and VectorStore Traits

**Purpose:** Define the AI provider and vector store port interfaces

**Files:**

| File | Action | Description |
|------|--------|-------------|
| `core/knowledge-core/src/ports/mod.rs` | Modify | Add `AiAdapter` trait, `VectorStore` trait, `VectorResult`, `VectorFilter`, `VectorMetadata`, `FusedResult` types |

**New types (per ADR-0017):**

```rust
#[async_trait]
pub trait AiAdapter: Send + Sync {
    async fn embed(&self, content: &str) -> Result<Vec<f32>, AiError>;
    fn model_name(&self) -> &str;
    fn dimensions(&self) -> usize;
}

#[derive(Debug, thiserror::Error)]
pub enum AiError {
    #[error("Provider error: {0}")]
    Provider(String),
    #[error("Network error: {0}")]
    Network(String),
    #[error("Configuration error: {0}")]
    Config(String),
}

#[async_trait]
pub trait VectorStore: Send + Sync {
    async fn upsert(&self, entity_id: &str, vector: &[f32], metadata: Option<VectorMetadata>) -> Result<(), VectorError>;
    async fn search(&self, query: &[f32], k: usize, filter: Option<VectorFilter>) -> Result<Vec<VectorResult>, VectorError>;
    async fn delete(&self, entity_id: &str) -> Result<(), VectorError>;
    async fn rebuild(&self) -> Result<(), VectorError>;
}

#[derive(Debug, thiserror::Error)]
pub enum VectorError {
    #[error("Storage error: {0}")]
    Storage(String),
    #[error("Dimension mismatch: expected {expected}, got {actual}")]
    DimensionMismatch { expected: usize, actual: usize },
}

pub struct VectorResult {
    pub entity_id: String,
    pub score: f64,
    pub metadata: Option<VectorMetadata>,
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

**Verification:**
- `cargo check -p knowledge-core` compiles
- `cargo test -p knowledge-core` passes

**Exit criteria:** Traits and types compile

---

### D2: In-Memory Vector Store

**Purpose:** Implement `VectorStore` with brute-force cosine similarity

**Files:**

| File | Action | Description |
|------|--------|-------------|
| `core/knowledge-derive/src/features/search/vector_store.rs` | Create | `InMemoryVectorStore` implementation |
| `core/knowledge-derive/src/features/search/mod.rs` | Modify | Add vector_store module |
| `core/knowledge-derive/src/features/mod.rs` | Modify | Add search module |

**Implementation (per ADR-0017):**

```rust
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

| File | Action | Description |
|------|--------|-------------|
| `core/knowledge-derive/src/features/search/hybrid.rs` | Create | `reciprocal_rank_fusion` function and `HybridSearch` struct |
| `core/knowledge-derive/src/features/search/mod.rs` | Modify | Add hybrid module |

**Implementation (per ADR-0017):**

```rust
pub fn reciprocal_rank_fusion(
    keyword_results: &[SearchResult],
    semantic_results: &[VectorResult],
    k: usize, // RRF constant, typically 60
) -> Vec<FusedResult> {
    let mut scores: HashMap<String, f64> = HashMap::new();

    for (rank, result) in keyword_results.iter().enumerate() {
        *scores.entry(result.entity_id.clone()).or_insert(0.0) += 1.0 / (k + rank + 1) as f64;
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

| File | Action | Description |
|------|--------|-------------|
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

**Verification:**
- Unit test: same input produces same embedding
- Unit test: different inputs produce different embeddings
- Unit test: embedding dimension matches `dimensions()`

**Exit criteria:** Mock embedder works for deterministic testing

---

### D5: Embedding Pipeline Integration

**Purpose:** Wire embedding generation to the import pipeline

**Files:**

| File | Action | Description |
|------|--------|-------------|
| `core/knowledge-derive/src/features/search/pipeline.rs` | Create | `EmbeddingPipeline` that generates embeddings from Content components |
| `core/knowledge-derive/src/features/search/mod.rs` | Modify | Add pipeline module |

**Implementation notes:**

The embedding pipeline:
1. Extract text from entity's Content component
2. Call `AiAdapter::embed()` to generate vector
3. Create Embedding component with vector, model name, and timestamp
4. Store vector in `VectorStore`
5. Attach Embedding component to entity

Pipeline runs:
- On import: new entities with Content get embeddings generated
- On content update: embeddings are regenerated
- On rebuild: all embeddings are regenerated from canonical Content components

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

| File | Action | Description |
|------|--------|-------------|
| `cli/src/main.rs` | Modify | Add `--semantic` and `--hybrid` flags to search command |
| `cli/features/prd-0003/semantic-search.feature` | Create | BDD scenarios for semantic and hybrid search |
| `cli/tests/cucumber.rs` | Modify | Add step definitions for semantic search |

**CLI interface (per PRD-0003):**

```
kos search "machine learning" --semantic
kos search "attention mechanism" --hybrid
```

**Search modes (per ADR-0017):**

| Mode | Behavior |
|------|----------|
| `keyword` | FTS5 BM25 search only (existing behavior, default) |
| `semantic` | Embedding cosine similarity only |
| `hybrid` | Both, fused via RRF |

**BDD scenarios:**

```gherkin
Feature: Semantic Search
  As a knowledge worker
  I want to search by meaning, not just keywords
  So that I can find conceptually related entities

  Scenario: Semantic search
    Given I have entities about machine learning topics
    When I run "kos search 'neural networks' --semantic"
    Then I should see semantically related entities
    And results should include entities without exact keyword match

  Scenario: Hybrid search
    Given I have entities about machine learning topics
    When I run "kos search 'transformer' --hybrid"
    Then I should see results combining keyword and semantic matches
    And entities appearing in both result sets should rank higher

  Scenario: Keyword search still works
    Given I have entities about machine learning topics
    When I run "kos search 'transformer'"
    Then I should see keyword-matched entities (existing behavior)

  Scenario: Semantic search without AI provider
    Given no AI provider is configured
    When I run "kos search 'machine learning' --semantic"
    Then I should see an error or fallback to keyword search
```

**Verification:**
- `cargo test --test cucumber -p knowledge-cli` passes
- BDD scenarios: semantic search, hybrid search, keyword fallback, no-provider error
- Manual test: import files, search with `--semantic` and `--hybrid`

**Exit criteria:** Semantic and hybrid search work via CLI, BDD tests pass

---

## Execution Order

```
D1 (traits) -> D2 (vector store) -> D3 (RRF) -> D4 (mock embedder) -> D5 (pipeline) -> D6 (CLI)
```

D1 defines types. D2-D3 implement core logic. D4 provides test infrastructure. D5 wires the pipeline. D6 exposes to CLI.

---

## Verification Strategy

| Level | Command | Coverage |
|-------|---------|----------|
| Unit | `cargo test -p knowledge-derive` | Vector store, RRF, mock embedder |
| Integration | `cargo test -p knowledge-derive --test integration_test` | Embedding pipeline |
| E2E | `cargo test --test cucumber -p knowledge-cli` | CLI search commands |
| Lint | `cargo clippy -- -D warnings && cargo fmt --check` | Code quality |

---

## Exit Criteria

- [ ] `AiAdapter` and `VectorStore` traits in `knowledge-core`
- [ ] `InMemoryVectorStore` with brute-force cosine similarity
- [ ] `reciprocal_rank_fusion` function
- [ ] `MockAiAdapter` for deterministic testing
- [ ] `EmbeddingPipeline` generates embeddings from Content components
- [ ] `kos search --semantic` and `--hybrid` flags
- [ ] BDD tests: 4+ semantic search scenarios
- [ ] `cargo clippy -- -D warnings` passes
- [ ] ADR-0017 updated with Implementation Notes

---

## Implementation Notes

*(Filled in during/after implementation)*
