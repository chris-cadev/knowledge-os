use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PluginManifest {
    pub name: String,
    pub version: String,
    pub description: String,
    pub author: String,
    pub license: Option<String>,
    pub priority: Option<u32>,
}

impl PluginManifest {
    pub fn effective_priority(&self) -> u32 {
        self.priority.unwrap_or(100)
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum PluginCapability {
    Importer { formats: Vec<String> },
    Renderer { name: String },
    AiProvider { capabilities: Vec<String> },
    VectorStore { name: String },
}

pub trait Plugin: Send + Sync {
    fn manifest(&self) -> &PluginManifest;
    fn activate(&self) -> Result<(), PluginError>;
    fn deactivate(&self) -> Result<(), PluginError>;
}

pub trait PluginMetadata {
    fn manifest(&self) -> PluginManifest;
}

#[derive(Debug, thiserror::Error)]
pub enum PluginError {
    #[error("Plugin not found: {0}")]
    NotFound(String),
    #[error("Plugin activation failed: {0}")]
    ActivationFailed(String),
    #[error("Plugin execution failed: {0}")]
    ExecutionFailed(String),
    #[error("Plugin timeout: {0}")]
    Timeout(String),
}
