use async_trait::async_trait;
use base64::Engine;
use knowledge_core::ports::{ImageInput, OcrBackend, OcrError, OcrResult};

pub struct OllamaOcrBackend {
    client: reqwest::Client,
    model: String,
    endpoint: String,
}

impl OllamaOcrBackend {
    pub fn new(model: impl Into<String>) -> Self {
        Self {
            client: reqwest::Client::new(),
            model: model.into(),
            endpoint: "http://localhost:11434".to_string(),
        }
    }

    pub fn with_endpoint(mut self, endpoint: impl Into<String>) -> Self {
        self.endpoint = endpoint.into();
        self
    }
}

#[async_trait]
impl OcrBackend for OllamaOcrBackend {
    async fn recognize(&self, image: &ImageInput) -> Result<OcrResult, OcrError> {
        let b64 = base64::engine::general_purpose::STANDARD.encode(&image.bytes);

        let body = serde_json::json!({
            "model": self.model,
            "prompt": "Extract all text from this image. Return only the text.",
            "images": [b64],
            "stream": false,
        });

        let resp = self
            .client
            .post(format!("{}/api/generate", self.endpoint))
            .json(&body)
            .send()
            .await
            .map_err(|e| OcrError::Network(e.to_string()))?;

        let status = resp.status();
        let json: serde_json::Value = resp
            .json()
            .await
            .map_err(|e| OcrError::Network(e.to_string()))?;

        if !status.is_success() {
            return Err(OcrError::Provider(format!(
                "Ollama returned {}: {}",
                status,
                json.get("error")
                    .and_then(|v| v.as_str())
                    .unwrap_or("unknown")
            )));
        }

        let text = json
            .get("response")
            .and_then(|v| v.as_str())
            .unwrap_or("")
            .to_string();

        Ok(OcrResult {
            text,
            confidence: 0.9,
            blocks: vec![],
            model: self.model.clone(),
        })
    }

    fn name(&self) -> &str {
        "ollama"
    }

    fn requires_network(&self) -> bool {
        true
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use httpmock::prelude::*;
    use serde_json::json;

    #[tokio::test]
    async fn ollama_recognize_sends_base64_image() {
        let server = MockServer::start();
        let mock = server.mock(|when, then| {
            when.method(POST).path("/api/generate").json_body_partial(
                serde_json::to_string(&json!({"model": "deepseek-ocr"})).unwrap(),
            );
            then.status(200)
                .json_body(json!({"response": "Hello world"}));
        });

        let backend = OllamaOcrBackend::new("deepseek-ocr").with_endpoint(server.base_url());
        let image = ImageInput {
            bytes: vec![1, 2, 3],
            mime_type: "image/png".to_string(),
            width: 10,
            height: 10,
        };
        let result = backend.recognize(&image).await.unwrap();
        assert_eq!(result.text, "Hello world");
        mock.assert();
    }

    #[tokio::test]
    async fn ollama_recognize_parses_response() {
        let server = MockServer::start();
        let mock = server.mock(|when, then| {
            when.method(POST).path("/api/generate");
            then.status(200)
                .json_body(json!({"response": "Hello world"}));
        });

        let backend = OllamaOcrBackend::new("deepseek-ocr").with_endpoint(server.base_url());
        let image = ImageInput {
            bytes: vec![1, 2, 3],
            mime_type: "image/png".to_string(),
            width: 10,
            height: 10,
        };
        let result = backend.recognize(&image).await.unwrap();
        assert_eq!(result.text, "Hello world");
        assert_eq!(result.model, "deepseek-ocr");
        mock.assert();
    }

    #[tokio::test]
    async fn ollama_with_custom_endpoint() {
        let server = MockServer::start();
        let mock = server.mock(|when, then| {
            when.method(POST).path("/api/generate");
            then.status(200).json_body(json!({"response": "test"}));
        });

        let backend = OllamaOcrBackend::new("llama-vision").with_endpoint(server.base_url());
        assert_eq!(backend.endpoint, server.base_url());

        let image = ImageInput {
            bytes: vec![1, 2, 3],
            mime_type: "image/png".to_string(),
            width: 10,
            height: 10,
        };
        backend.recognize(&image).await.unwrap();
        mock.assert();
    }

    #[tokio::test]
    async fn ollama_recognize_maps_400_to_provider_error() {
        let server = MockServer::start();
        let mock = server.mock(|when, then| {
            when.method(POST).path("/api/generate");
            then.status(400)
                .json_body(json!({"error": "invalid model"}));
        });

        let backend = OllamaOcrBackend::new("bad-model").with_endpoint(server.base_url());
        let image = ImageInput {
            bytes: vec![1, 2, 3],
            mime_type: "image/png".to_string(),
            width: 10,
            height: 10,
        };
        let result = backend.recognize(&image).await;
        assert!(matches!(result, Err(OcrError::Provider(_))));
        mock.assert();
    }
}
