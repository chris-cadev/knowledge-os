use async_trait::async_trait;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ImageInput {
    pub bytes: Vec<u8>,
    pub mime_type: String,
    pub width: u32,
    pub height: u32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BoundingBox {
    pub x: u32,
    pub y: u32,
    pub width: u32,
    pub height: u32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TextBlock {
    pub text: String,
    pub bbox: BoundingBox,
    pub confidence: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OcrResult {
    pub text: String,
    pub confidence: f64,
    pub blocks: Vec<TextBlock>,
    pub model: String,
}

#[derive(Debug, thiserror::Error)]
pub enum OcrError {
    #[error("provider error: {0}")]
    Provider(String),
    #[error("network error: {0}")]
    Network(String),
    #[error("image decode error: {0}")]
    ImageDecode(String),
    #[error("unsupported format: {0}")]
    UnsupportedFormat(String),
}

#[async_trait]
pub trait OcrBackend: Send + Sync {
    async fn recognize(&self, image: &ImageInput) -> Result<OcrResult, OcrError>;
    fn name(&self) -> &str;
    fn requires_network(&self) -> bool;
}
