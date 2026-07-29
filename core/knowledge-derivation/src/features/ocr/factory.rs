use knowledge_core::ports::{OcrBackend, OcrError};

pub fn create_ocr_backend(config: &str) -> Result<Box<dyn OcrBackend>, OcrError> {
    if config == "mock" || config.starts_with("mock://") {
        return Ok(Box::new(super::mock::MockOcrBackend::default()));
    }

    if config == "tesseract" || config.starts_with("tesseract://") {
        let lang = config.strip_prefix("tesseract://").unwrap_or("eng");
        return Ok(Box::new(
            super::tesseract::TesseractOcrBackend::new().with_language(if lang.is_empty() {
                "eng"
            } else {
                lang
            }),
        ));
    }

    if let Some(rest) = config.strip_prefix("ollama://") {
        let parts: Vec<&str> = rest.split('?').collect();
        let model = parts[0];
        if model.is_empty() {
            return Err(OcrError::Provider(
                "ollama:// requires a model name, e.g. ollama://deepseek-ocr".to_string(),
            ));
        }
        let mut backend = super::ollama::OllamaOcrBackend::new(model);
        if parts.len() > 1 {
            for param in parts[1].split('&') {
                if let Some(url) = param.strip_prefix("url=") {
                    backend = backend.with_endpoint(url);
                }
            }
        }
        return Ok(Box::new(backend));
    }

    if let Some(rest) = config.strip_prefix("api://") {
        let parts: Vec<&str> = rest.split('?').collect();
        let model = parts[0];
        let mut api_key = String::new();
        let mut base_url = String::new();
        if parts.len() > 1 {
            for param in parts[1].split('&') {
                if let Some(key) = param.strip_prefix("api_key=") {
                    api_key = key.to_string();
                } else if let Some(url) = param.strip_prefix("base_url=") {
                    base_url = url.to_string();
                }
            }
        }
        if api_key.is_empty() {
            return Err(OcrError::Provider(
                "api:// requires api_key parameter, e.g. api://gpt-4o?api_key=KEY".to_string(),
            ));
        }
        let mut backend = super::api::ApiOcrBackend::new(api_key, model.to_string());
        if !base_url.is_empty() {
            backend = backend.with_base_url(base_url);
        }
        return Ok(Box::new(backend));
    }

    Ok(Box::new(super::mock::MockOcrBackend::default()))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn factory_creates_mock_for_mock_scheme() {
        let backend = create_ocr_backend("mock").unwrap();
        assert_eq!(backend.name(), "mock");
    }

    #[test]
    fn factory_creates_tesseract_default() {
        let backend = create_ocr_backend("tesseract").unwrap();
        assert_eq!(backend.name(), "tesseract");
    }

    #[test]
    fn factory_creates_tesseract_with_custom_language() {
        let backend = create_ocr_backend("tesseract://fra").unwrap();
        assert_eq!(backend.name(), "tesseract");
    }

    #[test]
    fn factory_creates_ollama_for_ollama_scheme() {
        let backend = create_ocr_backend("ollama://deepseek-ocr").unwrap();
        assert_eq!(backend.name(), "ollama");
    }

    #[test]
    fn factory_creates_api_for_api_scheme() {
        let backend = create_ocr_backend("api://gpt-4o?api_key=sk-test").unwrap();
        assert_eq!(backend.name(), "api");
    }

    #[test]
    fn factory_ollama_missing_model_returns_error() {
        let result = create_ocr_backend("ollama://");
        assert!(result.is_err());
    }

    #[test]
    fn factory_api_missing_key_returns_error() {
        let result = create_ocr_backend("api://gpt-4o");
        assert!(result.is_err());
    }

    #[test]
    fn factory_defaults_to_mock() {
        let backend = create_ocr_backend("unknown-scheme").unwrap();
        assert_eq!(backend.name(), "mock");
    }

    #[test]
    fn factory_ollama_with_custom_url() {
        let backend = create_ocr_backend("ollama://llama-vision?url=http://custom:11434").unwrap();
        assert_eq!(backend.name(), "ollama");
    }

    #[test]
    fn factory_api_with_custom_base_url() {
        let backend = create_ocr_backend(
            "api://llama-3.2-vision?api_key=test&base_url=http://localhost:1234/v1",
        )
        .unwrap();
        assert_eq!(backend.name(), "api");
    }
}
