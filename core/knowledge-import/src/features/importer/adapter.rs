use async_trait::async_trait;
use knowledge_core::features::component::Component;
use knowledge_core::features::entity::Entity;
use std::path::{Path, PathBuf};

/// Result of importing a single file.
pub struct ImportResult {
    pub entity: Entity,
    pub components: Vec<Component>,
    pub cross_references: Vec<CrossReference>,
}

/// A cross-reference extracted from imported content.
pub enum CrossReference {
    /// Standard Markdown link: [text](path)
    FileRef {
        target_path: PathBuf,
        link_text: String,
    },
    /// URL link: [text](https://...)
    UrlRef {
        url: String,
        link_text: String,
    },
    /// Wikilink: [[name]]
    WikilinkRef {
        target_name: String,
        link_text: String,
    },
    /// @mention: @name
    MentionRef {
        target_name: String,
    },
    /// Section anchor: path#section
    SectionRef {
        target_path: PathBuf,
        section: String,
        link_text: String,
    },
}

impl CrossReference {
    pub fn as_file_ref(&self) -> Option<(&PathBuf, &str)> {
        match self {
            CrossReference::FileRef { target_path, link_text } => Some((target_path, link_text)),
            _ => None,
        }
    }

    pub fn as_url_ref(&self) -> Option<(&str, &str)> {
        match self {
            CrossReference::UrlRef { url, link_text } => Some((url, link_text)),
            _ => None,
        }
    }

    pub fn as_wikilink_ref(&self) -> Option<(&str, &str)> {
        match self {
            CrossReference::WikilinkRef { target_name, link_text } => Some((target_name, link_text)),
            _ => None,
        }
    }

    pub fn as_mention_ref(&self) -> Option<&str> {
        match self {
            CrossReference::MentionRef { target_name } => Some(target_name),
            _ => None,
        }
    }

    pub fn as_section_ref(&self) -> Option<(&PathBuf, &str, &str)> {
        match self {
            CrossReference::SectionRef { target_path, section, link_text } => {
                Some((target_path, section, link_text))
            }
            _ => None,
        }
    }
}

/// Trait for format-specific import adapters.
/// Each format (Markdown, PDF, etc.) implements this trait.
/// The CLI selects the adapter based on file extension.
#[async_trait]
pub trait ImportAdapter: Send + Sync {
    /// Check if this adapter can import the given file.
    fn can_import(&self, path: &Path) -> bool;

    /// Import a file and return the parsed result.
    async fn import(&self, path: &Path) -> Result<ImportResult, ImportError>;

    /// Return the file extensions this adapter supports.
    fn supported_extensions(&self) -> &[&str];
}

/// Errors that can occur during import.
#[derive(Debug, thiserror::Error)]
pub enum ImportError {
    #[error("io error: {0}")]
    Io(#[from] std::io::Error),

    #[error("parse error: {0}")]
    Parse(String),

    #[error("unsupported format: {0}")]
    UnsupportedFormat(String),

    #[error("scanned PDF: no extractable text")]
    ScannedPdf,

    #[error("network error: {0}")]
    Network(String),
}
