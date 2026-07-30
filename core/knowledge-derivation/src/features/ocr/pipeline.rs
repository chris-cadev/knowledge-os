use std::sync::Arc;

use async_trait::async_trait;
use chrono::Utc;
use knowledge_core::features::component::{Component, ComponentType};
use knowledge_core::ports::{
    ComponentRepository, Event, EventLog, EventType, ImageInput, OcrBackend, OcrError, OcrProvider,
    OcrResult,
};
use uuid::Uuid;

pub struct OcrPipeline {
    backend: Arc<dyn OcrBackend>,
    component_repo: Box<dyn ComponentRepository>,
    event_log: Box<dyn EventLog>,
}

impl OcrPipeline {
    pub fn new(
        backend: Arc<dyn OcrBackend>,
        component_repo: Box<dyn ComponentRepository>,
        event_log: Box<dyn EventLog>,
    ) -> Self {
        Self {
            backend,
            component_repo,
            event_log,
        }
    }

    pub async fn process_image(
        &self,
        entity_id: Uuid,
        image_bytes: Vec<u8>,
        mime_type: String,
    ) -> Result<OcrResult, OcrError> {
        let img = image::load_from_memory(&image_bytes)
            .map_err(|e| OcrError::ImageDecode(e.to_string()))?;
        let input = ImageInput {
            bytes: image_bytes,
            mime_type,
            width: img.width(),
            height: img.height(),
        };

        let result = self.backend.recognize(&input).await?;

        let content = Component::new(
            entity_id,
            ComponentType::Content,
            serde_json::json!({ "markdown": result.text }),
        );

        let existing = self
            .component_repo
            .find_by_type(entity_id, "Content")
            .await
            .map_err(|e| OcrError::Provider(e.to_string()))?;

        let event_type = if existing.is_empty() {
            self.component_repo
                .save(&content)
                .await
                .map_err(|e| OcrError::Provider(e.to_string()))?;
            EventType::ComponentAdded
        } else {
            self.component_repo
                .update_data(existing[0].id, content.data)
                .await
                .map_err(|e| OcrError::Provider(e.to_string()))?;
            EventType::ComponentUpdated
        };

        let event = Event {
            id: Uuid::new_v4(),
            event_type,
            entity_id,
            timestamp: Utc::now(),
            data: serde_json::json!({
                "pipeline": "ocr",
                "confidence": result.confidence,
                "model": result.model,
            }),
        };
        let _ = self.event_log.append(&event).await;

        Ok(result)
    }
}

#[async_trait]
impl OcrProvider for OcrPipeline {
    async fn process_image(
        &self,
        entity_id: Uuid,
        image_bytes: Vec<u8>,
        mime_type: String,
    ) -> Result<String, String> {
        self.process_image(entity_id, image_bytes, mime_type)
            .await
            .map(|r| r.text)
            .map_err(|e| e.to_string())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use knowledge_core::features::component::Component;
    use knowledge_core::ports::StorageError;

    struct MockEventLog;

    #[async_trait]
    impl EventLog for MockEventLog {
        async fn append(&self, _event: &Event) -> Result<(), StorageError> {
            Ok(())
        }
        async fn list_by_entity(&self, _entity_id: Uuid) -> Result<Vec<Event>, StorageError> {
            Ok(vec![])
        }
    }

    struct MockComponentRepo;

    #[async_trait::async_trait]
    impl ComponentRepository for MockComponentRepo {
        async fn get(&self, _entity_id: Uuid) -> Result<Vec<Component>, StorageError> {
            Ok(vec![])
        }

        async fn save(&self, _component: &Component) -> Result<(), StorageError> {
            Ok(())
        }

        async fn delete(&self, _id: Uuid) -> Result<(), StorageError> {
            Ok(())
        }

        async fn find_by_type(
            &self,
            _entity_id: Uuid,
            _component_type: &str,
        ) -> Result<Vec<Component>, StorageError> {
            Ok(vec![])
        }

        async fn update_data(
            &self,
            _id: Uuid,
            _data: serde_json::Value,
        ) -> Result<(), StorageError> {
            Ok(())
        }

        async fn find_by_component_data(
            &self,
            _component_type: &str,
            _json_path: &str,
            _value: &str,
        ) -> Result<Vec<Component>, StorageError> {
            Ok(vec![])
        }

        async fn delete_by_entity(&self, _entity_id: Uuid) -> Result<(), StorageError> {
            Ok(())
        }
    }

    #[tokio::test]
    async fn pipeline_processes_image_and_updates_content() {
        let backend = Arc::new(super::super::mock::MockOcrBackend::new().with_text("OCR text"));
        let repo = Box::new(MockComponentRepo);
        let event_log = Box::new(MockEventLog);
        let pipeline = OcrPipeline::new(backend, repo, event_log);

        let mut buf = std::io::Cursor::new(Vec::new());
        let img = image::ImageBuffer::from_fn(10, 10, |_x, _y| image::Luma([255u8]));
        image::DynamicImage::ImageLuma8(img)
            .write_to(&mut buf, image::ImageFormat::Png)
            .expect("write png");

        let result = pipeline
            .process_image(Uuid::new_v4(), buf.into_inner(), "image/png".to_string())
            .await
            .unwrap();

        assert_eq!(result.text, "OCR text");
    }
}
