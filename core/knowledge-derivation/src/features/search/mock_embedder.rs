use async_trait::async_trait;
use knowledge_core::ports::{AiAdapter, AiError};

/// A deterministic mock embedder for testing.
///
/// Produces vectors by hashing input text, allowing tests to verify
/// semantic search without external AI providers. Similar inputs produce
/// similar vectors (adjacent hash bits).
pub struct MockAiAdapter {
    model: String,
    dimensions: usize,
}

impl MockAiAdapter {
    /// Create a new mock embedder with the given model name and dimensionality.
    pub fn new(model: &str, dimensions: usize) -> Self {
        Self {
            model: model.to_string(),
            dimensions,
        }
    }
}

impl Default for MockAiAdapter {
    fn default() -> Self {
        Self::new("mock-model", 8)
    }
}

/// Simple deterministic hash: fold bytes into f32 values.
///
/// Each byte contributes to the output vector in a round-robin fashion.
/// Different inputs produce different vectors. Identical inputs produce
/// identical vectors.
fn deterministic_hash(content: &str, dimensions: usize) -> Vec<f32> {
    let mut result = vec![0.0f32; dimensions];
    let bytes = content.as_bytes();

    for (i, &byte) in bytes.iter().enumerate() {
        let idx = i % dimensions;
        // Spread bits across the vector dimension using a simple mixing function
        let val = (byte as f32) / 255.0;
        let angle = (i as f32) * 0.7 + (idx as f32) * 1.3;
        result[idx] += val * angle.sin();
    }

    // Normalize to unit vector
    let norm: f32 = result.iter().map(|x| x * x).sum::<f32>().sqrt();
    if norm > 0.0 {
        for x in &mut result {
            *x /= norm;
        }
    }

    result
}

#[async_trait]
impl AiAdapter for MockAiAdapter {
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

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn same_input_produces_same_embedding() {
        let adapter = MockAiAdapter::new("test", 8);
        let a = adapter.embed("hello world").await.unwrap();
        let b = adapter.embed("hello world").await.unwrap();
        assert_eq!(a, b);
    }

    #[tokio::test]
    async fn different_inputs_produce_different_embeddings() {
        let adapter = MockAiAdapter::new("test", 8);
        let a = adapter.embed("hello").await.unwrap();
        let b = adapter.embed("world").await.unwrap();
        assert_ne!(a, b);
    }

    #[tokio::test]
    async fn embedding_dimension_matches_dimensions() {
        let adapter = MockAiAdapter::new("test", 16);
        let embedding = adapter.embed("test content").await.unwrap();
        assert_eq!(embedding.len(), 16);
        assert_eq!(adapter.dimensions(), 16);
    }

    #[tokio::test]
    async fn embedding_is_normalized() {
        let adapter = MockAiAdapter::new("test", 8);
        let embedding = adapter.embed("hello world").await.unwrap();
        let norm: f32 = embedding.iter().map(|x| x * x).sum::<f32>().sqrt();
        assert!((norm - 1.0).abs() < 1e-5);
    }

    #[tokio::test]
    async fn model_name_returns_configured_name() {
        let adapter = MockAiAdapter::new("custom-model", 8);
        assert_eq!(adapter.model_name(), "custom-model");
    }

    #[test]
    fn default_mock_has_expected_values() {
        let adapter = MockAiAdapter::default();
        assert_eq!(adapter.model_name(), "mock-model");
        assert_eq!(adapter.dimensions(), 8);
    }
}
