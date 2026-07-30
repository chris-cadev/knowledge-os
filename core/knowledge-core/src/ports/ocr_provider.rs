use async_trait::async_trait;
use uuid::Uuid;

#[async_trait]
pub trait OcrProvider: Send + Sync {
    async fn process_image(
        &self,
        entity_id: Uuid,
        image_bytes: Vec<u8>,
        mime_type: String,
    ) -> Result<String, String>;
}
