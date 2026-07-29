use knowledge_core::features::component::ComponentType;
use knowledge_core::ports::{ComponentRepository, EntityRepository, SearchIndex, SearchQuery};
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
    let store = &*state.store;

    let search_query = SearchQuery {
        query,
        entity_type,
        tag,
    };

    let results = SearchIndex::search(store, &search_query)
        .await
        .map_err(|e| e.to_string())?;

    let mut response = Vec::with_capacity(results.len());
    for result in results {
        let entity = EntityRepository::get(store, result.entity_id)
            .await
            .map_err(|e| e.to_string())?;

        let (title, snippet) = if let Some(ref entity) = entity {
            let components = ComponentRepository::get(store, entity.id)
                .await
                .map_err(|e| e.to_string())?;
            let t = components
                .iter()
                .find(|c| c.component_type == ComponentType::Title)
                .and_then(|c| c.data.as_str().map(String::from))
                .unwrap_or_else(|| "Untitled".to_string());
            let s = components
                .iter()
                .find(|c| c.component_type == ComponentType::Content)
                .and_then(|c| c.data.as_str().map(String::from))
                .unwrap_or_default();
            (t, s)
        } else {
            ("Deleted".to_string(), String::new())
        };

        response.push(SearchResultResponse {
            entity_id: result.entity_id.to_string(),
            title,
            entity_type: entity
                .as_ref()
                .map(|e| e.entity_type.to_string())
                .unwrap_or_default(),
            snippet: snippet.chars().take(200).collect(),
            score: result.score,
        });
    }

    Ok(response)
}
