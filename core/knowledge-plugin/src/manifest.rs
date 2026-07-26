use knowledge_core::ports::PluginManifest;
use serde::Deserialize;

/// Errors that can occur during manifest parsing.
#[derive(Debug, thiserror::Error)]
pub enum ManifestError {
    /// The TOML content could not be parsed.
    #[error("Invalid TOML: {0}")]
    Parse(String),

    /// A required field is missing from the manifest.
    #[error("Missing required field: {0}")]
    MissingField(String),

    /// The manifest file could not be read.
    #[error("IO error: {0}")]
    Io(String),
}

/// Top-level TOML structure for a plugin manifest file.
///
/// The TOML file uses a `[plugin]` section containing the manifest fields.
/// Example:
///
/// ```toml
/// [plugin]
/// name = "markdown-importer"
/// version = "0.1.0"
/// description = "Import Markdown files as knowledge entities"
/// author = "Knowledge OS"
/// license = "MIT"
/// priority = 100
/// ```
#[derive(Debug, Deserialize)]
struct TomlManifest {
    plugin: PluginSection,
}

#[derive(Debug, Deserialize)]
struct PluginSection {
    name: String,
    version: String,
    description: String,
    author: String,
    license: Option<String>,
    priority: Option<u32>,
}

/// Parse a TOML string into a `PluginManifest`.
///
/// # Errors
///
/// Returns `ManifestError::Parse` if the TOML is malformed.
/// Returns `ManifestError::MissingField` if required fields are absent.
pub fn parse_manifest(toml_content: &str) -> Result<PluginManifest, ManifestError> {
    let toml_manifest: TomlManifest =
        toml::from_str(toml_content).map_err(|e| ManifestError::Parse(e.to_string()))?;

    let plugin = toml_manifest.plugin;

    if plugin.name.is_empty() {
        return Err(ManifestError::MissingField("name".to_string()));
    }
    if plugin.version.is_empty() {
        return Err(ManifestError::MissingField("version".to_string()));
    }

    Ok(PluginManifest {
        name: plugin.name,
        version: plugin.version,
        description: plugin.description,
        author: plugin.author,
        license: plugin.license,
        priority: plugin.priority,
    })
}

/// Parse a TOML file at the given path into a `PluginManifest`.
///
/// # Errors
///
/// Returns `ManifestError::Io` if the file cannot be read.
/// Returns `ManifestError::Parse` if the TOML is malformed.
/// Returns `ManifestError::MissingField` if required fields are absent.
pub fn parse_manifest_file(path: &std::path::Path) -> Result<PluginManifest, ManifestError> {
    let content = std::fs::read_to_string(path).map_err(|e| ManifestError::Io(e.to_string()))?;
    parse_manifest(&content)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_valid_manifest() {
        let toml_content = r#"
[plugin]
name = "markdown-importer"
version = "0.1.0"
description = "Import Markdown files"
author = "Knowledge OS"
license = "MIT"
priority = 50
"#;
        let manifest = parse_manifest(toml_content).unwrap();
        assert_eq!(manifest.name, "markdown-importer");
        assert_eq!(manifest.version, "0.1.0");
        assert_eq!(manifest.description, "Import Markdown files");
        assert_eq!(manifest.author, "Knowledge OS");
        assert_eq!(manifest.license.as_deref(), Some("MIT"));
        assert_eq!(manifest.priority, Some(50));
    }

    #[test]
    fn test_parse_manifest_minimal() {
        let toml_content = r#"
[plugin]
name = "minimal-plugin"
version = "1.0.0"
description = "A minimal plugin"
author = "Test"
"#;
        let manifest = parse_manifest(toml_content).unwrap();
        assert_eq!(manifest.name, "minimal-plugin");
        assert_eq!(manifest.version, "1.0.0");
        assert!(manifest.license.is_none());
        assert!(manifest.priority.is_none());
    }

    #[test]
    fn test_parse_manifest_invalid_toml() {
        let toml_content = "this is not valid toml {{{";
        let result = parse_manifest(toml_content);
        assert!(result.is_err());
        match result.unwrap_err() {
            ManifestError::Parse(_) => {}
            other => panic!("Expected Parse error, got {:?}", other),
        }
    }

    #[test]
    fn test_parse_manifest_missing_name() {
        let toml_content = r#"
[plugin]
version = "0.1.0"
description = "No name"
author = "Test"
"#;
        let result = parse_manifest(toml_content);
        assert!(result.is_err());
        match result.unwrap_err() {
            ManifestError::Parse(_) => {}
            other => panic!("Expected error, got {:?}", other),
        }
    }

    #[test]
    fn test_parse_manifest_empty_name() {
        let toml_content = r#"
[plugin]
name = ""
version = "0.1.0"
description = "Empty name"
author = "Test"
"#;
        let result = parse_manifest(toml_content);
        assert!(result.is_err());
        match result.unwrap_err() {
            ManifestError::MissingField(field) => assert_eq!(field, "name"),
            other => panic!("Expected MissingField error, got {:?}", other),
        }
    }

    #[test]
    fn test_parse_manifest_empty_version() {
        let toml_content = r#"
[plugin]
name = "test"
version = ""
description = "Empty version"
author = "Test"
"#;
        let result = parse_manifest(toml_content);
        assert!(result.is_err());
        match result.unwrap_err() {
            ManifestError::MissingField(field) => assert_eq!(field, "version"),
            other => panic!("Expected MissingField error, got {:?}", other),
        }
    }

    #[test]
    fn test_parse_manifest_file_not_found() {
        let result = parse_manifest_file(std::path::Path::new("/nonexistent/plugin.toml"));
        assert!(result.is_err());
        match result.unwrap_err() {
            ManifestError::Io(_) => {}
            other => panic!("Expected Io error, got {:?}", other),
        }
    }
}
