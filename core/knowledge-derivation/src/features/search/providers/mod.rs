pub mod openai;

use async_trait::async_trait;
use knowledge_core::ports::{AiAdapter, AiError};

/// Deterministic mock embedder using hash-based vectors.
///
/// Produces vectors by hashing input text. Similar inputs produce similar
/// vectors (adjacent hash bits). Useful for testing and development when
/// no real AI provider is available.
pub struct MockAiAdapter {
    model: String,
    dimensions: usize,
}

impl MockAiAdapter {
    pub fn new(model: &str, dimensions: usize) -> Self {
        Self {
            model: model.to_string(),
            dimensions,
        }
    }
}

impl Default for MockAiAdapter {
    fn default() -> Self {
        Self::new("mock-model", 128)
    }
}

#[async_trait]
impl AiAdapter for MockAiAdapter {
    async fn embed(&self, content: &str) -> Result<Vec<f32>, AiError> {
        use std::hash::{Hash, Hasher};
        let mut hasher = std::collections::hash_map::DefaultHasher::new();
        content.hash(&mut hasher);
        let seed = hasher.finish();

        let mut vec = Vec::with_capacity(self.dimensions);
        for i in 0..self.dimensions {
            let val = ((seed.wrapping_add((i as u64).wrapping_mul(0x9e3779b97f4a7c15)) & 0xFF) as f32 - 128.0)
                / 128.0;
            vec.push(val);
        }

        let norm: f32 = vec.iter().map(|x| x * x).sum::<f32>().sqrt();
        if norm > 0.0 {
            for x in &mut vec {
                *x /= norm;
            }
        }

        Ok(vec)
    }

    fn model_name(&self) -> &str {
        &self.model
    }

    fn dimensions(&self) -> usize {
        self.dimensions
    }
}

/// Parse a provider configuration string into an AI adapter.
///
/// Supported formats:
/// - `mock` or `mock://dimensions` — deterministic mock adapter
/// - `openai://MODEL?api_key=KEY` — OpenAI with explicit key
/// - `openai://MODEL` — OpenAI with key from OPENAI_API_KEY env var
///
/// If `OPENAI_API_KEY` is set and no provider is specified, defaults to OpenAI.
pub fn create_from_config(config: &str) -> Result<Box<dyn AiAdapter>, AiError> {
    if config == "mock" || config.starts_with("mock://") {
        let dims = if let Some(dims_str) = config.strip_prefix("mock://") {
            dims_str.parse::<usize>().unwrap_or(128)
        } else {
            128
        };
        return Ok(Box::new(MockAiAdapter::new("mock-model", dims)));
    }

    if let Some(rest) = config.strip_prefix("openai://") {
        let parts: Vec<&str> = rest.split('?').collect();
        let model = parts[0];

        let api_key = if let Some(query) = parts.get(1) {
            if let Some(key_val) = query.strip_prefix("api_key=") {
                key_val.to_string()
            } else {
                return Err(AiError::Provider(format!(
                    "Invalid OpenAI config: {}",
                    config
                )));
            }
        } else {
            std::env::var("OPENAI_API_KEY")
                .map_err(|_| AiError::Provider("OPENAI_API_KEY not set".to_string()))?
        };

        return Ok(Box::new(openai::OpenAiAdapter::new(
            api_key,
            model.to_string(),
        )));
    }

    // Default: try OpenAI from env, fall back to mock
    if let Ok(api_key) = std::env::var("OPENAI_API_KEY") {
        return Ok(Box::new(openai::OpenAiAdapter::new(
            api_key,
            "text-embedding-3-small".to_string(),
        )));
    }

    eprintln!("Warning: No AI provider configured and OPENAI_API_KEY not set. Using mock embedder.");
    eprintln!("  Set OPENAI_API_KEY or configure via --ai-provider flag.");
    eprintln!("  Supported providers: mock://DIMS, openai://MODEL?api_key=KEY");

    Ok(Box::new(MockAiAdapter::default()))
}
