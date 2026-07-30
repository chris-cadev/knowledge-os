use knowledge_import::features::importer::ignore_config::DEFAULT_PATTERNS;
use tauri::State;

use super::store::AppState;

#[tauri::command]
pub async fn get_ignore_patterns(state: State<'_, AppState>) -> Result<String, String> {
    let path = state.data_dir.join(".kosignore");
    match std::fs::read_to_string(&path) {
        Ok(content) => Ok(content),
        Err(_) => Ok(DEFAULT_PATTERNS.join("\n")),
    }
}

#[tauri::command]
pub async fn set_ignore_patterns(
    state: State<'_, AppState>,
    patterns: String,
) -> Result<(), String> {
    let path = state.data_dir.join(".kosignore");
    std::fs::write(&path, &patterns).map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn reset_ignore_patterns(state: State<'_, AppState>) -> Result<String, String> {
    let path = state.data_dir.join(".kosignore");
    let _ = std::fs::remove_file(&path);
    Ok(DEFAULT_PATTERNS.join("\n"))
}
