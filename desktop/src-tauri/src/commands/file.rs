use tauri_plugin_opener::OpenerExt;

use crate::wsl;

/// Open a file or URL in the OS default application.
///
/// URLs (http, https, mailto, tel) are opened in the default browser. Paths
/// are opened in the default application. When running inside WSL, delegates
/// to the Windows host via `cmd.exe start` so that files and URLs open in
/// Windows default applications. On other platforms, uses
/// `tauri-plugin-opener` for native cross-platform behavior.
#[tauri::command]
pub async fn open_in_default_app(app: tauri::AppHandle, path: String) -> Result<(), String> {
    if is_url(&path) {
        if wsl::is_wsl() {
            return wsl::open_url_with_windows_host(&path).await;
        }

        return app
            .opener()
            .open_url(&path, None::<&str>)
            .map_err(|e| format!("failed to open URL: {}", e));
    }

    let canonical = std::path::Path::new(&path);
    let canonical = dunce::canonicalize(canonical)
        .unwrap_or_else(|_| canonical.to_path_buf())
        .to_string_lossy()
        .into_owned();

    if wsl::is_wsl() {
        return wsl::open_file_with_windows_host(&canonical).await;
    }

    app.opener()
        .open_path(canonical, None::<&str>)
        .map_err(|e| format!("failed to open file: {}", e))
}

/// Returns `true` when the input looks like a URL with a known scheme.
fn is_url(input: &str) -> bool {
    let lower = input.to_ascii_lowercase();
    ["http://", "https://", "mailto:", "tel:"]
        .iter()
        .any(|scheme| lower.starts_with(scheme))
}

/// Reveal a file in the OS file manager.
///
/// When running inside WSL, delegates to the Windows host via
/// `explorer.exe /select,<path>` so that folders open in Windows Explorer.
/// On other platforms, uses `tauri-plugin-opener` for native cross-platform
/// behavior.
#[tauri::command]
pub async fn open_source_folder(app: tauri::AppHandle, path: String) -> Result<(), String> {
    let canonical = std::path::Path::new(&path);
    let canonical = dunce::canonicalize(canonical)
        .unwrap_or_else(|_| canonical.to_path_buf())
        .to_string_lossy()
        .into_owned();

    if wsl::is_wsl() {
        return wsl::reveal_file_with_windows_host(&canonical).await;
    }

    app.opener()
        .reveal_item_in_dir(canonical)
        .map_err(|e| format!("failed to open folder: {}", e))
}
