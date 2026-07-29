use std::ffi::{c_char, CStr};
use std::path::Path;
use std::sync::Arc;

use knowledge_core::ports::{Plugin, PluginError, PluginManifest};

/// A dynamically loaded plugin from a shared library.
///
/// Wraps a `libloading::Library` handle and the imported functions.
/// The library is kept alive for the lifetime of this struct.
pub struct DynamicPlugin {
    _library: Arc<libloading::Library>,
    manifest: PluginManifest,
}

impl DynamicPlugin {
    /// Load a plugin from a shared library file.
    ///
    /// The library must export a function named `kos_plugin_manifest` that
    /// returns a `*const c_char` (JSON string), and optionally
    /// `kos_plugin_create_importer` that returns a `*mut c_void`.
    ///
    /// # Safety
    ///
    /// Loading shared libraries is inherently unsafe. The plugin must be
    /// compiled against a compatible version of Knowledge OS.
    pub unsafe fn load(lib_path: &Path) -> Result<Self, PluginError> {
        let library = Arc::new(libloading::Library::new(lib_path).map_err(|e| {
            PluginError::LoadFailed(format!("failed to load plugin library: {}", e))
        })?);

        // Load manifest
        let manifest_fn: libloading::Symbol<unsafe extern "C" fn() -> *const c_char> =
            library.get(b"kos_plugin_manifest").map_err(|_| {
                PluginError::LoadFailed("plugin missing kos_plugin_manifest symbol".to_string())
            })?;

        let manifest_ptr = manifest_fn();
        if manifest_ptr.is_null() {
            return Err(PluginError::LoadFailed(
                "plugin manifest function returned null".to_string(),
            ));
        }

        let manifest_str = CStr::from_ptr(manifest_ptr)
            .to_str()
            .map_err(|e| PluginError::LoadFailed(format!("invalid manifest UTF-8: {}", e)))?;

        let manifest_data: serde_json::Value = serde_json::from_str(manifest_str)
            .map_err(|e| PluginError::LoadFailed(format!("invalid manifest JSON: {}", e)))?;

        let manifest = PluginManifest {
            name: manifest_data["name"]
                .as_str()
                .unwrap_or("unknown")
                .to_string(),
            version: manifest_data["version"]
                .as_str()
                .unwrap_or("0.0.0")
                .to_string(),
            description: manifest_data["description"]
                .as_str()
                .unwrap_or("")
                .to_string(),
            author: manifest_data["author"].as_str().unwrap_or("").to_string(),
            license: manifest_data["license"].as_str().map(String::from),
            priority: manifest_data["priority"].as_u64().map(|u| u as u32),
        };

        Ok(Self {
            _library: library,
            manifest,
        })
    }
}

impl Plugin for DynamicPlugin {
    fn manifest(&self) -> &PluginManifest {
        &self.manifest
    }

    fn activate(&self) -> Result<(), PluginError> {
        Ok(())
    }

    fn deactivate(&self) -> Result<(), PluginError> {
        Ok(())
    }
}

/// Discover and load all plugins from a plugin directory.
///
/// Scans the directory for `.so` / `.dylib` / `.dll` files and attempts
/// to load each one as a Knowledge OS plugin. Skips files that cannot
/// be loaded with a warning.
pub fn load_plugins_from(plugin_dir: &Path) -> Vec<DynamicPlugin> {
    let mut plugins = Vec::new();

    if !plugin_dir.is_dir() {
        return plugins;
    }

    let entries = match std::fs::read_dir(plugin_dir) {
        Ok(entries) => entries,
        Err(_) => return plugins,
    };

    for entry in entries.flatten() {
        let path = entry.path();

        // Check if this is a shared library
        let is_so = path.extension().map(|e| e == "so").unwrap_or(false);
        let is_dylib = path.extension().map(|e| e == "dylib").unwrap_or(false);
        let is_dll = path.extension().map(|e| e == "dll").unwrap_or(false);

        if !is_so && !is_dylib && !is_dll {
            continue;
        }

        // Check for accompanying plugin.toml manifest
        let manifest_path = path.with_file_name("plugin.toml");
        let manifest_exists = manifest_path.exists();

        if !manifest_exists {
            // Continue without warning - may be a system library
            continue;
        }

        match unsafe { DynamicPlugin::load(&path) } {
            Ok(plugin) => {
                plugins.push(plugin);
            }
            Err(e) => {
                eprintln!("Warning: failed to load plugin '{}': {}", path.display(), e);
            }
        }
    }

    plugins
}

#[cfg(test)]
mod tests {}
