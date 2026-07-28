//! Benchmarks for semantic search operations.
//!
//! Benchmarks embedding pipeline, vector store search, and hybrid search.

use async_trait::async_trait;
use criterion::{black_box, criterion_group, criterion_main, Criterion};
use knowledge_core::ports::*;
use knowledge_derive::features::search::vector_store::InMemoryVectorStore;

// ---------------------------------------------------------------------------
// Inline deterministic embedder (mirrors mock_embedder for benchmark use)
// ---------------------------------------------------------------------------

struct BenchAiAdapter {
    model: String,
    dimensions: usize,
}

impl BenchAiAdapter {
    fn new(model: &str, dimensions: usize) -> Self {
        Self {
            model: model.to_string(),
            dimensions,
        }
    }
}

fn deterministic_hash(content: &str, dimensions: usize) -> Vec<f32> {
    let mut result = vec![0.0f32; dimensions];
    let bytes = content.as_bytes();
    for (i, &byte) in bytes.iter().enumerate() {
        let idx = i % dimensions;
        let val = (byte as f32) / 255.0;
        let angle = (i as f32) * 0.7 + (idx as f32) * 1.3;
        result[idx] += val * angle.sin();
    }
    let norm: f32 = result.iter().map(|x| x * x).sum::<f32>().sqrt();
    if norm > 0.0 {
        for x in &mut result {
            *x /= norm;
        }
    }
    result
}

#[async_trait]
impl AiAdapter for BenchAiAdapter {
    async fn embed(&self, content: &str) -> Result<Vec<f32>, AiError> {
        Ok(deterministic_hash(content, self.dimensions))
    }
    fn model_name(&self) -> &str {
        &self.model
    }
    fn dimensions(&self) -> usize {
        self.dimensions
    }
}

fn bench_embedding_generation(c: &mut Criterion) {
    let rt = tokio::runtime::Runtime::new().unwrap();
    let adapter = BenchAiAdapter::new("bench-model", 128);

    c.bench_function("embedding_generation_128d", |b| {
        b.iter(|| {
            rt.block_on(adapter.embed(black_box(
                "Machine learning is a subset of artificial intelligence that focuses on building systems that learn from data.",
            )))
            .unwrap()
        })
    });
}

fn bench_vector_store_upsert(c: &mut Criterion) {
    let rt = tokio::runtime::Runtime::new().unwrap();
    let store = InMemoryVectorStore::new(128);
    let adapter = BenchAiAdapter::new("bench-model", 128);

    let vector = rt
        .block_on(adapter.embed("test content for upsert benchmark"))
        .unwrap();

    c.bench_function("vector_store_upsert_128d", |b| {
        let mut i = 0u64;
        b.iter(|| {
            i += 1;
            let entity_id = format!("entity-{}", i);
            rt.block_on(store.upsert(black_box(&entity_id), black_box(&vector), None))
                .unwrap()
        })
    });
}

fn bench_vector_store_search(c: &mut Criterion) {
    let rt = tokio::runtime::Runtime::new().unwrap();
    let store = InMemoryVectorStore::new(128);
    let adapter = BenchAiAdapter::new("bench-model", 128);

    // Populate store with 1000 vectors
    for i in 0..1000 {
        let content = format!(
            "Document {} about machine learning and artificial intelligence",
            i
        );
        let vector = rt.block_on(adapter.embed(&content)).unwrap();
        rt.block_on(store.upsert(&format!("entity-{}", i), &vector, None))
            .unwrap();
    }

    let query = rt.block_on(adapter.embed("machine learning")).unwrap();

    c.bench_function("vector_store_search_1000_128d_top10", |b| {
        b.iter(|| {
            rt.block_on(store.search(black_box(&query), 10, None))
                .unwrap()
        })
    });
}

fn bench_vector_store_search_top100(c: &mut Criterion) {
    let rt = tokio::runtime::Runtime::new().unwrap();
    let store = InMemoryVectorStore::new(128);
    let adapter = BenchAiAdapter::new("bench-model", 128);

    // Populate store with 1000 vectors
    for i in 0..1000 {
        let content = format!(
            "Document {} about machine learning and artificial intelligence",
            i
        );
        let vector = rt.block_on(adapter.embed(&content)).unwrap();
        rt.block_on(store.upsert(&format!("entity-{}", i), &vector, None))
            .unwrap();
    }

    let query = rt.block_on(adapter.embed("machine learning")).unwrap();

    c.bench_function("vector_store_search_1000_128d_top100", |b| {
        b.iter(|| {
            rt.block_on(store.search(black_box(&query), 100, None))
                .unwrap()
        })
    });
}

fn bench_hybrid_rrf_fusion(c: &mut Criterion) {
    use knowledge_core::ports::{SearchResult, VectorResult};

    // Create synthetic search results
    let keyword_results: Vec<SearchResult> = (0..50)
        .map(|i| SearchResult {
            entity_id: uuid::Uuid::new_v4(),
            score: 1.0 / (i + 1) as f64,
            confidence: None,
            snippet: None,
        })
        .collect();

    let semantic_results: Vec<VectorResult> = (0..50)
        .map(|i| VectorResult {
            entity_id: uuid::Uuid::new_v4().to_string(),
            score: 1.0 - (i as f64 * 0.01),
            metadata: None,
        })
        .collect();

    c.bench_function("hybrid_rrf_fusion_50_50", |b| {
        b.iter(|| {
            knowledge_derive::features::search::hybrid::reciprocal_rank_fusion(
                black_box(&keyword_results),
                black_box(&semantic_results),
                60,
            )
        })
    });
}

fn bench_cosine_similarity(c: &mut Criterion) {
    let vec_a: Vec<f32> = (0..128).map(|i| i as f32 / 128.0).collect();
    let vec_b: Vec<f32> = (0..128).map(|i| (128 - i) as f32 / 128.0).collect();

    c.bench_function("cosine_similarity_128d", |bench| {
        bench.iter(|| {
            knowledge_derive::features::search::vector_store::cosine_similarity(
                black_box(&vec_a),
                black_box(&vec_b),
            )
        })
    });
}

criterion_group!(
    benches,
    bench_embedding_generation,
    bench_vector_store_upsert,
    bench_vector_store_search,
    bench_vector_store_search_top100,
    bench_hybrid_rrf_fusion,
    bench_cosine_similarity,
);
criterion_main!(benches);
