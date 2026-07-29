use knowledge_core::services::entity_retrieval::RetrievalFilter;
use tauri::State;

use super::response::*;
use super::store::AppState;

/// Search entities by query, with optional type and tag filters.
#[tauri::command]
pub async fn search_entities(
    state: State<'_, AppState>,
    query: String,
    entity_type: Option<String>,
    tag: Option<String>,
) -> Result<Vec<SearchResultResponse>, String> {
    let filter = RetrievalFilter {
        entity_types: entity_type.map(|t| vec![t]),
        tags: tag.map(|t| vec![t]),
        limit: Some(50),
    };

    let results = state
        .entity_retrieval
        .search(&query, &filter)
        .await
        .map_err(|e| e.to_string())?;

    let response: Vec<SearchResultResponse> = results
        .into_iter()
        .map(|s| SearchResultResponse {
            entity_id: s.id.to_string(),
            title: s.title,
            entity_type: s.entity_type,
            snippet: s.preview,
            score: 0.0,
        })
        .collect();

    Ok(response)
}
