use knowledge_core::features::component::{Component, ComponentType};
use knowledge_core::ports::{ComponentRepository, EntityRepository, EventLog};
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
    let store = &*state.store;
    let entity_id = Uuid::parse_str(&id).map_err(|e| format!("invalid entity ID: {}", e))?;

    let entity = EntityRepository::get(store, entity_id)
        .await
        .map_err(|e| e.to_string())?
        .ok_or_else(|| format!("entity not found: {}", id))?;

    // Components
    let components = ComponentRepository::get(store, entity.id)
        .await
        .map_err(|e| e.to_string())?;

    let components_data: Vec<ComponentData> = components
        .iter()
        .map(|c| ComponentData {
            component_type: format!("{:?}", c.component_type),
            data: c.data.clone(),
        })
        .collect();

    // Helper to extract title from entity components
    async fn get_title(
        store: &knowledge_storage::adapters::sqlite::SqliteStore,
        entity_id: Uuid,
    ) -> Result<String, String> {
        let comps = ComponentRepository::get(store, entity_id)
            .await
            .map_err(|e| e.to_string())?;
        Ok(comps
            .iter()
            .find(|c| c.component_type == ComponentType::Title)
            .and_then(|c| c.data.as_str().map(String::from))
            .unwrap_or_else(|| "Untitled".to_string()))
    }

    // Outgoing relationships
    let outgoing = knowledge_core::ports::RelationshipRepository::by_source(store, entity.id)
        .await
        .map_err(|e| e.to_string())?;

    let mut outgoing_info = Vec::with_capacity(outgoing.len());
    for rel in outgoing {
        let target_title = get_title(store, rel.target_id).await?;
        outgoing_info.push(RelationshipInfo {
            id: rel.id.to_string(),
            relationship_type: format!("{:?}", rel.relationship_type),
            source_id: rel.source_id.to_string(),
            target_id: rel.target_id.to_string(),
            source_title: String::new(),
            target_title,
            is_active: rel.is_active,
        });
    }

    // Incoming relationships
    let incoming = knowledge_core::ports::RelationshipRepository::by_target(store, entity.id)
        .await
        .map_err(|e| e.to_string())?;

    let mut incoming_info = Vec::with_capacity(incoming.len());
    for rel in incoming {
        let source_title = get_title(store, rel.source_id).await?;
        incoming_info.push(RelationshipInfo {
            id: rel.id.to_string(),
            relationship_type: format!("{:?}", rel.relationship_type),
            source_id: rel.source_id.to_string(),
            target_id: rel.target_id.to_string(),
            source_title,
            target_title: String::new(),
            is_active: rel.is_active,
        });
    }

    // Events
    let events = EventLog::list_by_entity(store, entity.id)
        .await
        .map_err(|e| e.to_string())?;

    let events_info: Vec<EventInfo> = events
        .iter()
        .map(|e| EventInfo {
            id: e.id.to_string(),
            event_type: format!("{:?}", e.event_type),
            timestamp: e.timestamp.to_rfc3339(),
            data: e.data.clone(),
        })
        .collect();

    // Version history
    let versions = EntityRepository::get_version_history(store, entity.id)
        .await
        .map_err(|e| e.to_string())?;

    let versions_info: Vec<VersionInfo> = versions
        .iter()
        .map(|v| VersionInfo {
            version: v.version,
            created_at: v.created_at.to_rfc3339(),
        })
        .collect();

    Ok(EntityDetailResponse {
        id: entity.id.to_string(),
        entity_type: entity.entity_type.to_string(),
        is_active: entity.is_active,
        created_at: entity.created_at.to_rfc3339(),
        updated_at: entity.updated_at.to_rfc3339(),
        components: components_data,
        outgoing_relationships: outgoing_info,
        incoming_relationships: incoming_info,
        events: events_info,
        versions: versions_info,
    })
}
