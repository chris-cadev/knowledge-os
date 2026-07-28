use async_trait::async_trait;

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
}
