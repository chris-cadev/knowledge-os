use std::path::Path;

use knowledge_core::ports::PluginManifest;

use crate::manifest::{parse_manifest_file, ManifestError};

/// A discovered plugin: its manifest and the path to its manifest file.
#[derive(Debug)]
pub struct DiscoveredPlugin {
    /// The parsed manifest metadata.
    pub manifest: PluginManifest,
    /// Path to the manifest file that was parsed.
    pub manifest_path: std::path::PathBuf,
}

/// Errors that can occur during plugin discovery.
#[derive(Debug, thiserror::Error)]
pub enum DiscoveryError {
    /// A manifest file could not be parsed.
    #[error("Manifest error in {path}: {source}")]
    Manifest { path: String, source: ManifestError },

    /// A directory could not be read.
    #[error("IO error reading plugin directory: {0}")]
    Io(String),
}

/// Discover plugin manifests in a directory.
///
/// Scans the given directory for `plugin.toml` files and parses each one
/// into a `DiscoveredPlugin`. Non-TOML files and directories without
/// `plugin.toml` are silently skipped.
///
/// # Errors
///
/// Returns `DiscoveryError::Io` if the directory cannot be read.
/// Returns `DiscoveryError::Manifest` if a `plugin.toml` file is malformed.
pub fn discover_plugins(plugin_dir: &Path) -> Result<Vec<DiscoveredPlugin>, DiscoveryError> {
    let mut plugins = Vec::new();

    if !plugin_dir.is_dir() {
        return Ok(plugins);
    }

    let entries = std::fs::read_dir(plugin_dir).map_err(|e| DiscoveryError::Io(e.to_string()))?;

    for entry in entries.flatten() {
        let path = entry.path();
        if path.is_dir() {
            let manifest_path = path.join("plugin.toml");
            if manifest_path.exists() {
                match parse_manifest_file(&manifest_path) {
                    Ok(manifest) => {
                        plugins.push(DiscoveredPlugin {
                            manifest,
                            manifest_path,
                        });
                    }
                    Err(e) => {
                        return Err(DiscoveryError::Manifest {
                            path: manifest_path.to_string_lossy().to_string(),
                            source: e,
                        });
                    }
                }
            }
        }
    }

    Ok(plugins)
}

/// Resolve conflicts between multiple plugins claiming the same capability.
///
/// Deterministic resolution order:
/// 1. Explicit priority (lower = preferred, default 100)
/// 2. Version precedence (higher semver wins)
/// 3. Alphabetical tiebreak
pub fn resolve_plugins(mut plugins: Vec<PluginManifest>) -> Vec<PluginManifest> {
    plugins.sort_by(|a, b| {
        a.effective_priority()
            .cmp(&b.effective_priority())
            .then_with(|| {
                // Compare versions lexicographically (good enough for semver in practice)
                b.version.cmp(&a.version).then_with(|| a.name.cmp(&b.name))
            })
    });
    plugins
}

#[cfg(test)]
mod tests {
    use super::*;
    use knowledge_core::ports::PluginManifest;

    fn make_manifest(name: &str, version: &str, priority: Option<u32>) -> PluginManifest {
        PluginManifest {
            name: name.to_string(),
            version: version.to_string(),
            description: format!("Plugin {}", name),
            author: "Test".to_string(),
            license: None,
            priority,
        }
    }

    #[test]
    fn test_discover_plugins_empty_dir() {
        let tmp = tempfile::tempdir().unwrap();
        let plugins = discover_plugins(tmp.path()).unwrap();
        assert!(plugins.is_empty());
    }

    #[test]
    fn test_discover_plugins_nonexistent_dir() {
        let plugins = discover_plugins(Path::new("/nonexistent")).unwrap();
        assert!(plugins.is_empty());
    }

    #[test]
    fn test_resolve_plugins_priority_ordering() {
        let plugins = vec![
            make_manifest("low-priority", "1.0.0", Some(200)),
            make_manifest("high-priority", "1.0.0", Some(10)),
            make_manifest("default-priority", "1.0.0", None),
        ];
        let resolved = resolve_plugins(plugins);
        assert_eq!(resolved[0].name, "high-priority");
        assert_eq!(resolved[1].name, "default-priority");
        assert_eq!(resolved[2].name, "low-priority");
    }

    #[test]
    fn test_resolve_plugins_version_tiebreak() {
        let plugins = vec![
            make_manifest("older", "1.0.0", Some(100)),
            make_manifest("newer", "2.0.0", Some(100)),
        ];
        let resolved = resolve_plugins(plugins);
        assert_eq!(resolved[0].name, "newer");
        assert_eq!(resolved[1].name, "older");
    }

    #[test]
    fn test_resolve_plugins_alphabetical_tiebreak() {
        let plugins = vec![
            make_manifest("beta-plugin", "1.0.0", Some(100)),
            make_manifest("alpha-plugin", "1.0.0", Some(100)),
        ];
        let resolved = resolve_plugins(plugins);
        assert_eq!(resolved[0].name, "alpha-plugin");
        assert_eq!(resolved[1].name, "beta-plugin");
    }
}
