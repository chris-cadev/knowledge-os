use async_trait::async_trait;
use base64::Engine;
use knowledge_core::ports::{ImageInput, OcrBackend, OcrError, OcrResult};

pub struct ApiOcrBackend {
    client: reqwest::Client,
    model: String,
    api_key: String,
    base_url: String,
}

impl ApiOcrBackend {
    pub fn new(api_key: String, model: String) -> Self {
        Self {
            client: reqwest::Client::builder()
                .timeout(std::time::Duration::from_secs(60))
                .build()
                .expect("HTTP client"),
            model,
            api_key,
            base_url: "https://api.openai.com/v1".to_string(),
        }
    }

    pub fn with_base_url(mut self, url: impl Into<String>) -> Self {
        self.base_url = url.into();
        self
    }
}

#[async_trait]
impl OcrBackend for ApiOcrBackend {
    async fn recognize(&self, image: &ImageInput) -> Result<OcrResult, OcrError> {
        let b64 = base64::engine::general_purpose::STANDARD.encode(&image.bytes);
        let data_url = format!("data:{};base64,{}", image.mime_type, b64);

        let body = serde_json::json!({
            "model": self.model,
            "messages": [{
                "role": "user",
                "content": [
                    {"type": "text", "text": "Extract all text from this image. Return only the text."},
                    {"type": "image_url", "image_url": {"url": data_url}}
                ]
            }],
            "max_tokens": 4096
        });

        let resp = self
            .client
            .post(format!("{}/chat/completions", self.base_url))
            .header("Authorization", format!("Bearer {}", self.api_key))
            .header("Content-Type", "application/json")
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
            let msg = json
                .pointer("/error/message")
                .and_then(|v| v.as_str())
                .unwrap_or("unknown");
            return Err(OcrError::Provider(format!(
                "API returned {}: {}",
                status, msg
            )));
        }

        let text = json
            .pointer("/choices/0/message/content")
            .and_then(|v| v.as_str())
            .unwrap_or("")
            .to_string();

        Ok(OcrResult {
            text,
            confidence: 0.95,
            blocks: vec![],
            model: self.model.clone(),
        })
    }

    fn name(&self) -> &str {
        "api"
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
    async fn api_recognize_sends_vision_format() {
        let server = MockServer::start();
        let mock = server.mock(|when, then| {
            when.method(POST)
                .path("/chat/completions")
                .header("Authorization", "Bearer test-key")
                .json_body_partial(serde_json::to_string(&json!({"model": "gpt-4o"})).unwrap());
            then.status(200).json_body(json!({
                "choices": [{
                    "message": {
                        "content": "Extracted text from vision API"
                    }
                }]
            }));
        });

        let backend = ApiOcrBackend::new("test-key".to_string(), "gpt-4o".to_string())
            .with_base_url(server.base_url());
        let image = ImageInput {
            bytes: vec![1, 2, 3],
            mime_type: "image/png".to_string(),
            width: 10,
            height: 10,
        };
        let result = backend.recognize(&image).await.unwrap();
        assert_eq!(result.text, "Extracted text from vision API");
        mock.assert();
    }

    #[tokio::test]
    async fn api_recognize_with_lm_studio() {
        let server = MockServer::start();
        let mock = server.mock(|when, then| {
            when.method(POST).path("/chat/completions");
            then.status(200).json_body(json!({
                "choices": [{
                    "message": {
                        "content": "LM Studio result"
                    }
                }]
            }));
        });

        let backend = ApiOcrBackend::new("not-needed".to_string(), "llama-3.2-vision".to_string())
            .with_base_url(server.base_url());
        let image = ImageInput {
            bytes: vec![1, 2, 3],
            mime_type: "image/png".to_string(),
            width: 10,
            height: 10,
        };
        let result = backend.recognize(&image).await.unwrap();
        assert_eq!(result.text, "LM Studio result");
        mock.assert();
    }

    #[tokio::test]
    async fn api_recognize_maps_400_to_provider_error() {
        let server = MockServer::start();
        let mock = server.mock(|when, then| {
            when.method(POST).path("/chat/completions");
            then.status(400).json_body(json!({
                "error": {
                    "message": "invalid_api_key",
                    "type": "invalid_request_error"
                }
            }));
        });

        let backend = ApiOcrBackend::new("bad-key".to_string(), "gpt-4o".to_string())
            .with_base_url(server.base_url());
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

    #[tokio::test]
    async fn api_recognize_maps_network_error() {
        let backend = ApiOcrBackend::new("key".to_string(), "gpt-4o".to_string())
            .with_base_url("http://localhost:1".to_string());
        let image = ImageInput {
            bytes: vec![1, 2, 3],
            mime_type: "image/png".to_string(),
            width: 10,
            height: 10,
        };
        let result = backend.recognize(&image).await;
        assert!(matches!(result, Err(OcrError::Network(_))));
    }
}
