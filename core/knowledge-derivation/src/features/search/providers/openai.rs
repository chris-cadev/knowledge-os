use async_trait::async_trait;
use knowledge_core::ports::{AiAdapter, AiError};
use serde::{Deserialize, Serialize};

/// OpenAI embedding adapter using the OpenAI API.
///
/// Requires an API key from OpenAI. Supports text-embedding-3-small,
/// text-embedding-3-large, and text-embedding-ada-002 models.
pub struct OpenAiAdapter {
    api_key: String,
    model: String,
    dimensions: usize,
    client: reqwest::Client,
}

impl OpenAiAdapter {
    /// Create a new OpenAI adapter.
    ///
    /// # Arguments
    ///
    /// * `api_key` - OpenAI API key
    /// * `model` - Model name (e.g., "text-embedding-3-small")
    ///
    /// # Model Dimensions
    ///
    /// - text-embedding-3-small: 1536
    /// - text-embedding-3-large: 3072
    /// - text-embedding-ada-002: 1536
    pub fn new(api_key: String, model: String) -> Self {
        let dimensions = match model.as_str() {
            "text-embedding-3-small" => 1536,
            "text-embedding-3-large" => 3072,
            "text-embedding-ada-002" => 1536,
            _ => 1536, // Default
        };

        let client = reqwest::Client::builder()
            .timeout(std::time::Duration::from_secs(30))
            .build()
            .expect("Failed to create HTTP client");

        Self {
            api_key,
            model,
            dimensions,
            client,
        }
    }

    /// Create an adapter from environment variable OPENAI_API_KEY.
    pub fn from_env(model: String) -> Result<Self, AiError> {
        let api_key = std::env::var("OPENAI_API_KEY")
            .map_err(|_| AiError::Provider("OPENAI_API_KEY not set".to_string()))?;
        Ok(Self::new(api_key, model))
    }
}

#[derive(Debug, Serialize)]
struct EmbeddingRequest {
    input: String,
    model: String,
    encoding_format: String,
}

#[derive(Debug, Deserialize)]
struct EmbeddingResponse {
    data: Vec<EmbeddingData>,
}

#[derive(Debug, Deserialize)]
struct EmbeddingData {
    embedding: Vec<f32>,
}

#[derive(Debug, Deserialize)]
struct OpenAiError {
    error: OpenAiErrorDetail,
}

#[derive(Debug, Deserialize)]
struct OpenAiErrorDetail {
    message: String,
}

#[async_trait]
impl AiAdapter for OpenAiAdapter {
    async fn embed(&self, content: &str) -> Result<Vec<f32>, AiError> {
        let request = EmbeddingRequest {
            input: content.to_string(),
            model: self.model.clone(),
            encoding_format: "float".to_string(),
        };

        let response = self
            .client
            .post("https://api.openai.com/v1/embeddings")
            .header("Authorization", format!("Bearer {}", self.api_key))
            .header("Content-Type", "application/json")
            .json(&request)
            .send()
            .await
            .map_err(|e| AiError::Network(format!("Request failed: {}", e)))?;

        let status = response.status();
        if !status.is_success() {
            let error_text = response
                .text()
                .await
                .unwrap_or_else(|_| "Unknown error".to_string());

            if let Ok(error) = serde_json::from_str::<OpenAiError>(&error_text) {
                return Err(AiError::Provider(format!(
                    "OpenAI API error: {}",
                    error.error.message
                )));
            }

            return Err(AiError::Provider(format!(
                "OpenAI API error ({}): {}",
                status, error_text
            )));
        }

        let embedding_response: EmbeddingResponse = response
            .json()
            .await
            .map_err(|e| AiError::Provider(format!("Failed to parse response: {}", e)))?;

        embedding_response
            .data
            .into_iter()
            .next()
            .map(|d| d.embedding)
            .ok_or_else(|| AiError::Provider("No embedding returned".to_string()))
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

    #[test]
    fn test_model_dimensions() {
        let adapter =
            OpenAiAdapter::new("test-key".to_string(), "text-embedding-3-small".to_string());
        assert_eq!(adapter.dimensions(), 1536);

        let adapter =
            OpenAiAdapter::new("test-key".to_string(), "text-embedding-3-large".to_string());
        assert_eq!(adapter.dimensions(), 3072);

        let adapter =
            OpenAiAdapter::new("test-key".to_string(), "text-embedding-ada-002".to_string());
        assert_eq!(adapter.dimensions(), 1536);
    }

    #[test]
    fn test_model_name() {
        let adapter =
            OpenAiAdapter::new("test-key".to_string(), "text-embedding-3-small".to_string());
        assert_eq!(adapter.model_name(), "text-embedding-3-small");
    }
}
