use std::path::PathBuf;
use std::sync::Arc;

use serde::{Deserialize, Serialize};

use super::store::AppState;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProviderConfig {
    pub provider_kind: String,
    pub model: String,
    pub base_url: Option<String>,
    pub api_key: Option<String>,
}

impl Default for ProviderConfig {
    fn default() -> Self {
        Self {
            provider_kind: "mock".into(),
            model: String::new(),
            base_url: None,
            api_key: None,
        }
    }
}

#[derive(Debug, Clone, Serialize)]
pub struct ProviderStatus {
    pub provider: String,
    pub model: String,
    pub base_url: String,
    pub reachable: bool,
    pub latency_ms: u32,
}

#[derive(Debug, Clone, Serialize)]
pub struct TestResult {
    pub success: bool,
    pub latency_ms: u32,
    pub error: Option<String>,
}

pub fn providers_config_path(data_dir: &std::path::Path) -> PathBuf {
    data_dir.join("providers.json")
}

pub async fn load_provider_config(data_dir: &std::path::Path) -> ProviderConfig {
    let path = providers_config_path(data_dir);
    match tokio::fs::read_to_string(&path).await {
        Ok(content) => serde_json::from_str(&content).unwrap_or_default(),
        Err(_) => ProviderConfig::default(),
    }
}

pub async fn save_provider_config(
    data_dir: &std::path::Path,
    config: &ProviderConfig,
) -> Result<(), String> {
    let path = providers_config_path(data_dir);
    let parent = path.parent().unwrap();
    std::fs::create_dir_all(parent).map_err(|e| format!("failed to create config dir: {}", e))?;
    let content =
        serde_json::to_string_pretty(config).map_err(|e| format!("failed to serialize: {}", e))?;
    tokio::fs::write(&path, content)
        .await
        .map_err(|e| format!("failed to write config: {}", e))
}

pub fn create_chat_provider_from_config(
    config: &ProviderConfig,
) -> Result<(Arc<dyn knowledge_core::ports::ChatCompletion>, String), String> {
    let provider_str = match config.provider_kind.as_str() {
        "mock" => "mock://".to_string(),
        "ollama" => {
            let model = if config.model.is_empty() {
                "llama3.2".to_string()
            } else {
                config.model.clone()
            };
            let mut s = format!("ollama://{}", model);
            if let Some(url) = &config.base_url {
                if !url.is_empty() {
                    s.push_str(&format!("?url={}", url));
                }
            }
            s
        }
        "openai-compatible" | "openai" => {
            let model = if config.model.is_empty() {
                "gpt-4o".to_string()
            } else {
                config.model.clone()
            };
            let api_key = config.api_key.clone().unwrap_or_default();
            let mut s = format!("openai://{}?api_key={}", model, api_key);
            if let Some(url) = &config.base_url {
                if !url.is_empty() {
                    s.push_str(&format!("&base_url={}", url));
                }
            }
            s
        }
        _ => "mock://".to_string(),
    };

    let provider =
        knowledge_derivation::features::chat::factory::create_chat_provider(&provider_str)
            .map_err(|e| format!("failed to create provider: {}", e))?;

    let kind = config.provider_kind.clone();
    Ok((Arc::from(provider), kind))
}

#[tauri::command]
pub async fn set_provider(
    state: tauri::State<'_, AppState>,
    provider_kind: String,
    model: String,
    base_url: Option<String>,
    api_key: Option<String>,
) -> Result<ProviderStatus, String> {
    let config = ProviderConfig {
        provider_kind: provider_kind.clone(),
        model: model.clone(),
        base_url: base_url.clone(),
        api_key: api_key.clone(),
    };

    save_provider_config(&state.data_dir, &config).await?;

    let (provider, kind) = create_chat_provider_from_config(&config)?;
    let mut pipeline = state.chat_pipeline.lock().await;
    pipeline.set_chat_provider(provider);
    *state.chat_provider_kind.lock().await = kind;

    let base_url_str = config.base_url.unwrap_or_default();
    Ok(ProviderStatus {
        provider: provider_kind,
        model,
        base_url: base_url_str,
        reachable: true,
        latency_ms: 0,
    })
}

pub async fn load_and_apply_provider(
    state: &AppState,
    config: &ProviderConfig,
) -> Result<String, String> {
    save_provider_config(&state.data_dir, config).await?;
    let (provider, kind) = create_chat_provider_from_config(config)?;
    let mut pipeline = state.chat_pipeline.lock().await;
    pipeline.set_chat_provider(provider);
    *state.chat_provider_kind.lock().await = kind.clone();
    Ok(kind)
}

#[tauri::command]
pub async fn get_providers_status(
    state: tauri::State<'_, AppState>,
) -> Result<ProviderStatus, String> {
    let config = load_provider_config(&state.data_dir).await;
    Ok(ProviderStatus {
        provider: config.provider_kind,
        model: config.model,
        base_url: config.base_url.unwrap_or_default(),
        reachable: true,
        latency_ms: 0,
    })
}

#[tauri::command]
pub async fn reset_provider(state: tauri::State<'_, AppState>) -> Result<ProviderStatus, String> {
    let config = ProviderConfig::default();
    let kind = load_and_apply_provider(&state, &config).await?;
    Ok(ProviderStatus {
        provider: kind,
        model: String::new(),
        base_url: String::new(),
        reachable: true,
        latency_ms: 0,
    })
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OcrProviderConfig {
    pub backend: String,
    pub model: String,
    pub base_url: Option<String>,
    pub api_key: Option<String>,
}

impl Default for OcrProviderConfig {
    fn default() -> Self {
        Self {
            backend: "mock".into(),
            model: String::new(),
            base_url: None,
            api_key: None,
        }
    }
}

#[derive(Debug, Clone, Serialize)]
pub struct OcrProviderStatus {
    pub backend: String,
    pub model: String,
    pub base_url: String,
    pub reachable: bool,
}

pub fn ocr_config_path(data_dir: &std::path::Path) -> PathBuf {
    data_dir.join("ocr_config.json")
}

pub async fn load_ocr_config(data_dir: &std::path::Path) -> OcrProviderConfig {
    let path = ocr_config_path(data_dir);
    match tokio::fs::read_to_string(&path).await {
        Ok(content) => serde_json::from_str(&content).unwrap_or_default(),
        Err(_) => OcrProviderConfig::default(),
    }
}

pub async fn save_ocr_config(
    data_dir: &std::path::Path,
    config: &OcrProviderConfig,
) -> Result<(), String> {
    let path = ocr_config_path(data_dir);
    let parent = path.parent().unwrap();
    std::fs::create_dir_all(parent).map_err(|e| format!("failed to create config dir: {}", e))?;
    let content =
        serde_json::to_string_pretty(config).map_err(|e| format!("failed to serialize: {}", e))?;
    tokio::fs::write(&path, content)
        .await
        .map_err(|e| format!("failed to write config: {}", e))
}

#[tauri::command]
pub async fn set_ocr_provider(
    state: tauri::State<'_, AppState>,
    backend: String,
    model: String,
    base_url: Option<String>,
    api_key: Option<String>,
) -> Result<OcrProviderStatus, String> {
    let config = OcrProviderConfig {
        backend: backend.clone(),
        model: model.clone(),
        base_url: base_url.clone(),
        api_key: api_key.clone(),
    };
    save_ocr_config(&state.data_dir, &config).await?;
    let base_url_str = config.base_url.unwrap_or_default();
    Ok(OcrProviderStatus {
        backend,
        model,
        base_url: base_url_str,
        reachable: true,
    })
}

#[tauri::command]
pub async fn get_ocr_provider_status(
    state: tauri::State<'_, AppState>,
) -> Result<OcrProviderStatus, String> {
    let config = load_ocr_config(&state.data_dir).await;
    Ok(OcrProviderStatus {
        backend: config.backend,
        model: config.model,
        base_url: config.base_url.unwrap_or_default(),
        reachable: true,
    })
}

#[tauri::command]
pub async fn reset_ocr_provider(
    state: tauri::State<'_, AppState>,
) -> Result<OcrProviderStatus, String> {
    let config = OcrProviderConfig::default();
    save_ocr_config(&state.data_dir, &config).await?;
    Ok(OcrProviderStatus {
        backend: config.backend,
        model: String::new(),
        base_url: String::new(),
        reachable: true,
    })
}

#[tauri::command]
pub async fn chat_test_provider(
    provider_kind: String,
    model: String,
    base_url: Option<String>,
    api_key: Option<String>,
) -> Result<TestResult, String> {
    let config = ProviderConfig {
        provider_kind,
        model,
        base_url,
        api_key,
    };

    let (provider, _kind) = create_chat_provider_from_config(&config)?;

    let start = std::time::Instant::now();
    let request = knowledge_core::ports::ChatRequest {
        system_prompt: "Respond with exactly 'ok'.".into(),
        messages: vec![knowledge_core::ports::Message {
            role: knowledge_core::ports::MessageRole::User,
            content: "test".into(),
            entity_refs: vec![],
        }],
        context_entities: vec![],
        mode: knowledge_core::ports::ResponseMode::Fast,
        source_toggles: knowledge_core::ports::SourceToggles::default(),
        config: knowledge_core::ports::ChatConfig::default(),
    };

    match provider.chat(request).await {
        Ok(_) => {
            let latency = start.elapsed().as_millis() as u32;
            Ok(TestResult {
                success: true,
                latency_ms: latency,
                error: None,
            })
        }
        Err(e) => {
            let latency = start.elapsed().as_millis() as u32;
            Ok(TestResult {
                success: false,
                latency_ms: latency,
                error: Some(e.to_string()),
            })
        }
    }
}
