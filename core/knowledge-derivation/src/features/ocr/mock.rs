use async_trait::async_trait;
use knowledge_core::ports::{BoundingBox, ImageInput, OcrBackend, OcrError, OcrResult, TextBlock};

pub struct MockOcrBackend {
    canned_text: String,
    confidence: f64,
}

impl MockOcrBackend {
    pub fn new() -> Self {
        Self {
            canned_text: "Mock OCR result".to_string(),
            confidence: 0.95,
        }
    }

    pub fn with_text(mut self, text: impl Into<String>) -> Self {
        self.canned_text = text.into();
        self
    }
}

impl Default for MockOcrBackend {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl OcrBackend for MockOcrBackend {
    async fn recognize(&self, _image: &ImageInput) -> Result<OcrResult, OcrError> {
        Ok(OcrResult {
            text: self.canned_text.clone(),
            confidence: self.confidence,
            blocks: vec![TextBlock {
                text: self.canned_text.clone(),
                bbox: BoundingBox {
                    x: 0,
                    y: 0,
                    width: 100,
                    height: 20,
                },
                confidence: self.confidence,
            }],
            model: "mock-ocr".to_string(),
        })
    }

    fn name(&self) -> &str {
        "mock"
    }

    fn requires_network(&self) -> bool {
        false
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn mock_recognize_returns_canned_text() {
        let backend = MockOcrBackend::new().with_text("Hello, World!");
        let image = ImageInput {
            bytes: vec![],
            mime_type: "image/png".to_string(),
            width: 100,
            height: 100,
        };
        let result = backend.recognize(&image).await.unwrap();
        assert_eq!(result.text, "Hello, World!");
        assert!((result.confidence - 0.95).abs() < f64::EPSILON);
    }

    #[tokio::test]
    async fn mock_name_is_mock() {
        let backend = MockOcrBackend::new();
        assert_eq!(backend.name(), "mock");
    }

    #[tokio::test]
    async fn mock_no_network() {
        let backend = MockOcrBackend::new();
        assert!(!backend.requires_network());
    }

    #[tokio::test]
    async fn mock_zero_byte_image() {
        let backend = MockOcrBackend::new();
        let image = ImageInput {
            bytes: vec![],
            mime_type: "image/png".to_string(),
            width: 0,
            height: 0,
        };
        let result = backend.recognize(&image).await.unwrap();
        assert_eq!(result.text, "Mock OCR result");
    }
}
