use tauri_plugin_opener::OpenerExt;

use crate::wsl;

/// Open a file in the OS default application.
///
/// When running inside WSL, delegates to the Windows host via `cmd.exe start`
/// so that files open in Windows default applications. On other platforms,
/// uses `tauri-plugin-opener` for native cross-platform behavior.
#[tauri::command]
pub async fn open_in_default_app(app: tauri::AppHandle, path: String) -> Result<(), String> {
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

/// Open a URL in the OS default browser.
///
/// When running inside WSL, delegates to the Windows host via `cmd.exe start`
/// so that URLs open in the Windows default browser. On other platforms, uses
/// `tauri-plugin-opener` for native cross-platform behavior.
#[tauri::command]
pub async fn open_url(app: tauri::AppHandle, url: String) -> Result<(), String> {
    if wsl::is_wsl() {
        return wsl::open_url_with_windows_host(&url).await;
    }

    app.opener()
        .open_url(&url, None::<&str>)
        .map_err(|e| format!("failed to open URL: {}", e))
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
