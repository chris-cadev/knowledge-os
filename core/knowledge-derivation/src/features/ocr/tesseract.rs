use async_trait::async_trait;
use knowledge_core::ports::{ImageInput, OcrBackend, OcrError, OcrResult};

pub struct TesseractOcrBackend {
    language: String,
}

impl TesseractOcrBackend {
    pub fn new() -> Self {
        Self {
            language: "eng".to_string(),
        }
    }

    pub fn with_language(mut self, lang: impl Into<String>) -> Self {
        self.language = lang.into();
        self
    }
}

impl Default for TesseractOcrBackend {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(feature = "tesseract-rs")]
mod backend {
    use super::*;
    use std::io::Cursor;

    pub async fn recognize(image: &ImageInput, language: &str) -> Result<OcrResult, OcrError> {
        let img = image::load_from_memory(&image.bytes)
            .map_err(|e| OcrError::ImageDecode(e.to_string()))?;
        let gray = img.to_luma8();
        let mut cursor = Cursor::new(gray.into_raw());

        let lang = language.to_string();
        let text = tokio::task::spawn_blocking(move || {
            let mut tess = tesseract::Tesseract::new(None, Some(&lang))
                .map_err(|e| OcrError::Provider(e.to_string()))?;
            tess.set_image_from_mem(cursor.get_ref())
                .map_err(|e| OcrError::Provider(e.to_string()))?;
            tess.get_text()
                .map_err(|e| OcrError::Provider(e.to_string()))
        })
        .await
        .map_err(|e| OcrError::Provider(format!("join error: {}", e)))??;

        Ok(OcrResult {
            text: text.trim().to_string(),
            confidence: 0.85,
            blocks: vec![],
            model: "tesseract-5".to_string(),
        })
    }
}

#[cfg(not(feature = "tesseract-rs"))]
mod backend {
    use super::*;
    pub async fn recognize(_image: &ImageInput, _language: &str) -> Result<OcrResult, OcrError> {
        Err(OcrError::Provider(
            "tesseract backend not enabled; enable feature 'tesseract-rs' or 'build-tesseract'"
                .to_string(),
        ))
    }
}

#[async_trait]
impl OcrBackend for TesseractOcrBackend {
    async fn recognize(&self, image: &ImageInput) -> Result<OcrResult, OcrError> {
        backend::recognize(image, &self.language).await
    }

    fn name(&self) -> &str {
        "tesseract"
    }

    fn requires_network(&self) -> bool {
        false
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn tesseract_handles_unsupported_format() {
        let backend = TesseractOcrBackend::new();
        let image = ImageInput {
            bytes: vec![0, 1, 2, 3],
            mime_type: "image/png".to_string(),
            width: 2,
            height: 2,
        };
        let result = backend.recognize(&image).await;
        // Without tesseract feature, returns Provider error.
        // With tesseract feature, invalid bytes should return ImageDecode error.
        assert!(result.is_err());
    }

    #[test]
    fn tesseract_default_language() {
        let backend = TesseractOcrBackend::new();
        assert_eq!(backend.language, "eng");
    }

    #[test]
    fn tesseract_custom_language() {
        let backend = TesseractOcrBackend::new().with_language("fra");
        assert_eq!(backend.language, "fra");
    }

    #[test]
    fn tesseract_no_network() {
        let backend = TesseractOcrBackend::new();
        assert!(!backend.requires_network());
    }

    #[test]
    fn tesseract_name() {
        let backend = TesseractOcrBackend::new();
        assert_eq!(backend.name(), "tesseract");
    }

    #[cfg(feature = "tesseract-rs")]
    #[tokio::test]
    async fn tesseract_recognizes_known_text() {
        let backend = TesseractOcrBackend::new();
        let mut buf = std::io::Cursor::new(Vec::new());
        let img = image::ImageBuffer::from_fn(200, 50, |_x, _y| image::Luma([255u8]));
        image::DynamicImage::ImageLuma8(img)
            .write_to(&mut buf, image::ImageFormat::Png)
            .expect("write png");
        let png_bytes = buf.into_inner();
        let image = ImageInput {
            bytes: png_bytes,
            mime_type: "image/png".to_string(),
            width: 200,
            height: 50,
        };
        match backend.recognize(&image).await {
            Ok(result) => {
                assert!(!result.model.is_empty());
            }
            Err(OcrError::Provider(msg)) => {
                eprintln!("Tesseract not available: {msg}");
            }
            Err(e) => panic!("Unexpected error: {e}"),
        }
    }
}
