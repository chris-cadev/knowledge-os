use knowledge_core::features::component::{Component, ComponentType};
use knowledge_core::ports::{ComponentRepository, EntityRepository};
use tauri::State;
use uuid::Uuid;

use super::response::*;
use super::store::AppState;

/// List entities, optionally filtered by entity type.
#[tauri::command]
pub async fn list_entities(
    state: State<'_, AppState>,
    entity_type: Option<String>,
) -> Result<Vec<EntitySummary>, String> {
    let store = &*state.store;

    let entities = match &entity_type {
        Some(t) => EntityRepository::find_by_type(store, t)
            .await
            .map_err(|e| e.to_string())?,
        None => EntityRepository::list(store)
            .await
            .map_err(|e| e.to_string())?,
    };

    let mut result = Vec::with_capacity(entities.len());
    for entity in entities {
        let components = ComponentRepository::get(store, entity.id)
            .await
            .map_err(|e| e.to_string())?;
        let title = components
            .iter()
            .find(|c| c.component_type == ComponentType::Title)
            .and_then(|c| c.data.as_str().map(String::from))
            .unwrap_or_else(|| "Untitled".to_string());

        result.push(EntitySummary {
            id: entity.id.to_string(),
            entity_type: entity.entity_type.to_string(),
            title,
            is_active: entity.is_active,
            created_at: entity.created_at.to_rfc3339(),
            updated_at: entity.updated_at.to_rfc3339(),
        });
    }

    Ok(result)
}

/// Get the source file path for an entity, if one exists.
///
/// Looks up the `Provenance` or `BinaryContent` component to extract
/// the stored source file path. Returns `None` if the entity has no
/// source file (e.g., it was created entirely within the system).
#[tauri::command]
pub async fn get_entity_source(
    state: State<'_, AppState>,
    id: String,
) -> Result<Option<String>, String> {
    let store = &*state.store;
    let entity_id = Uuid::parse_str(&id).map_err(|e| format!("invalid entity ID: {}", e))?;

    let components = ComponentRepository::get(store, entity_id)
        .await
        .map_err(|e| e.to_string())?;

    // Check Provenance component first (Markdown imports)
    if let Some(source) = extract_source_from_provenance(&components) {
        return Ok(Some(source));
    }

    // Fall back to BinaryContent reference (PDF imports)
    if let Some(reference) = extract_reference_from_binary(&components) {
        return Ok(Some(reference));
    }

    Ok(None)
}

/// Extract the source path from a `Provenance` component's `source` field.
fn extract_source_from_provenance(components: &[Component]) -> Option<String> {
    components
        .iter()
        .find(|c| c.component_type == ComponentType::Provenance)
        .and_then(|c| c.data.get("source"))
        .and_then(|v| v.as_str())
        .map(String::from)
}

/// Extract the reference from a `BinaryContent` component's `reference` field.
fn extract_reference_from_binary(components: &[Component]) -> Option<String> {
    components
        .iter()
        .find(|c| c.component_type == ComponentType::BinaryContent)
        .and_then(|c| c.data.get("reference"))
        .and_then(|v| v.as_str())
        .map(String::from)
}

/// Get detailed information about a single entity.
#[tauri::command]
pub async fn get_entity_detail(
    state: State<'_, AppState>,
    id: String,
) -> Result<EntityDetailResponse, String> {
    let entity_id = Uuid::parse_str(&id).map_err(|e| format!("invalid entity ID: {}", e))?;

    let detail = state
        .entity_retrieval
        .get_entity(entity_id)
        .await
        .map_err(|e| e.to_string())?;

    let components_data: Vec<ComponentData> = detail
        .components
        .into_iter()
        .map(|(ct, data)| ComponentData {
            component_type: ct,
            data,
        })
        .collect();

    let outgoing_info: Vec<RelationshipInfo> = detail
        .relationships
        .iter()
        .filter(|r| matches!(r.direction, knowledge_core::services::entity_retrieval::RelationshipDirection::Outgoing))
        .map(|r| RelationshipInfo {
            id: r.id.to_string(),
            relationship_type: r.relationship_type.clone(),
            source_id: String::new(),
            target_id: r.peer_id.to_string(),
            source_title: String::new(),
            target_title: r.peer_title.clone(),
            is_active: r.is_active,
        })
        .collect();

    let incoming_info: Vec<RelationshipInfo> = detail
        .relationships
        .iter()
        .filter(|r| matches!(r.direction, knowledge_core::services::entity_retrieval::RelationshipDirection::Incoming))
        .map(|r| RelationshipInfo {
            id: r.id.to_string(),
            relationship_type: r.relationship_type.clone(),
            source_id: r.peer_id.to_string(),
            target_id: String::new(),
            source_title: r.peer_title.clone(),
            target_title: String::new(),
            is_active: r.is_active,
        })
        .collect();

    let events_info: Vec<EventInfo> = detail
        .events
        .into_iter()
        .map(|e| EventInfo {
            id: e.id.to_string(),
            event_type: e.event_type,
            timestamp: e.timestamp.to_rfc3339(),
            data: e.data,
        })
        .collect();

    Ok(EntityDetailResponse {
        id: detail.id.to_string(),
        entity_type: detail.entity_type,
        is_active: detail.is_active,
        created_at: detail.created_at.to_rfc3339(),
        updated_at: detail.updated_at.to_rfc3339(),
        components: components_data,
        outgoing_relationships: outgoing_info,
        incoming_relationships: incoming_info,
        events: events_info,
        versions: vec![],
    })
}
