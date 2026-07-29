use knowledge_core::features::entity::Entity;

pub mod hybrid;
pub mod pipeline;
pub mod providers;
pub mod vector_store;

// Re-export key types for convenience
pub use providers::MockAiAdapter;
pub use providers::{create_from_config, openai};

/// Configuration for the AI provider used by the derivation layer.
#[derive(Debug, Clone)]
pub struct AiConfig {
    /// Provider configuration string (e.g. "mock://128", "openai://text-embedding-3-small")
    pub provider: String,
}

impl Default for AiConfig {
    fn default() -> Self {
        Self {
            provider: "mock://128".to_string(),
        }
    }
}

impl AiConfig {
    /// Create from a provider string, with auto-detection of OpenAI from env.
    pub fn from_provider(provider: &str) -> Self {
        Self {
            provider: provider.to_string(),
        }
    }

    /// Create from env: KOS_AI_PROVIDER or OPENAI_API_KEY.
    pub fn from_env() -> Self {
        if let Ok(provider) = std::env::var("KOS_AI_PROVIDER") {
            return Self { provider };
        }
        if std::env::var("OPENAI_API_KEY").is_ok() {
            return Self {
                provider: "openai://text-embedding-3-small".to_string(),
            };
        }
        Self::default()
    }
}

pub struct SearchIndex;

impl Default for SearchIndex {
    fn default() -> Self {
        Self::new()
    }
}

impl SearchIndex {
    pub fn new() -> Self {
        Self
    }

    pub async fn index(&self, _entity: &Entity) -> Result<(), Box<dyn std::error::Error>> {
        // TODO: implement indexing
        Ok(())
    }

    pub async fn search(&self, _query: &str) -> Result<Vec<Entity>, Box<dyn std::error::Error>> {
        // TODO: implement search
        Ok(vec![])
    }
}
