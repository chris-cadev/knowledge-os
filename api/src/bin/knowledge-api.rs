use std::collections::HashMap;
use std::sync::Arc;

use axum::{
    extract::{Path, Query, State},
    http::StatusCode,
    response::IntoResponse,
    routing::{delete, get, post},
    Json, Router,
};
use knowledge_core::features::component::{Component, ComponentType};
use knowledge_core::features::entity::{Entity, EntityType};
use knowledge_core::features::relationship::{Relationship, RelationshipType};
use knowledge_core::ports::{
    Collection, CollectionRepository, ComponentRepository, EntityRepository,
    EventLog, RelationshipRepository, SearchIndex, SearchQuery, TraversalConfig,
    TraversalDirection, TraversalPort, TraversalQuery, TransactionalWrite, ViewFilter, ViewOutput,
    ViewRegistry,
};
use knowledge_derivation::features::view::{
    graph::GraphViewAdapter, table::TableViewAdapter, timeline::TimelineViewAdapter,
    tree::TreeViewAdapter,
};
use knowledge_storage::adapters::sqlite::SqliteStore;
use serde::{Deserialize, Serialize};
use uuid::Uuid;

#[derive(Clone)]
struct ApiState {
    store: Arc<SqliteStore>,
}

#[derive(Serialize)]
struct ApiError {
    error: String,
}

impl IntoResponse for ApiError {
    fn into_response(self) -> axum::response::Response {
        (StatusCode::BAD_REQUEST, Json(self)).into_response()
    }
}

#[derive(Debug, Deserialize)]
struct SearchParams {
    q: String,
    #[serde(rename = "type")]
    entity_type: Option<String>,
    tag: Option<String>,
}

#[derive(Debug, Deserialize)]
struct CreateEntityPayload {
    entity_type: String,
    title: String,
    content: Option<String>,
    tags: Option<Vec<String>>,
}

#[derive(Debug, Serialize)]
struct EntitySummary {
    id: String,
    entity_type: String,
    title: String,
    is_active: bool,
    created_at: String,
    updated_at: String,
}

#[derive(Debug, Serialize)]
struct EntityDetail {
    id: String,
    entity_type: String,
    is_active: bool,
    created_at: String,
    updated_at: String,
    components: Vec<ComponentData>,
    outgoing_relationships: Vec<RelationshipData>,
    incoming_relationships: Vec<RelationshipData>,
    events: Vec<EventData>,
}

#[derive(Debug, Serialize)]
struct ComponentData {
    component_type: String,
    data: serde_json::Value,
}

#[derive(Debug, Serialize)]
struct RelationshipData {
    id: String,
    relationship_type: String,
    source_id: String,
    target_id: String,
}

#[derive(Debug, Serialize)]
struct EventData {
    event_type: String,
    timestamp: String,
    data: serde_json::Value,
}

#[derive(Debug, Deserialize)]
struct CreateRelationshipPayload {
    source_id: String,
    target_id: String,
}

#[derive(Debug, Deserialize)]
struct TraverseParams {
    start_id: String,
    depth: Option<u32>,
    direction: Option<String>,
    entity_type: Option<String>,
}

#[derive(Debug, Serialize)]
struct TraverseResult {
    depth: u32,
    entity_id: String,
    path: Vec<String>,
}

#[derive(Debug, Deserialize)]
struct CollectionPayload {
    name: String,
    description: Option<String>,
}

#[derive(Debug, Serialize)]
struct CollectionSummary {
    id: String,
    name: String,
    description: Option<String>,
    member_count: usize,
    created_at: String,
}

#[tokio::main]
async fn main() {
    tracing_subscriber::fmt::init();

    let store = Arc::new(SqliteStore::new("knowledge.db").expect("failed to open database"));

    let state = ApiState { store };

    let app = Router::new()
        // Health
        .route("/health", get(health))
        // Entities
        .route("/v1/entities", get(list_entities).post(create_entity))
        .route("/v1/entities/:id", get(get_entity).delete(delete_entity))
        .route("/v1/entities/:id/archive", post(archive_entity))
        .route("/v1/entities/:id/restore", post(restore_entity))
        // Components
        .route("/v1/entities/:id/components", get(get_components))
        // Relationships
        .route("/v1/relationships", post(create_relationship))
        .route("/v1/entities/:id/relationships", get(get_entity_relationships))
        // Search
        .route("/v1/search", get(search_entities))
        // Traversal
        .route("/v1/traverse", get(traverse_graph))
        // Views
        .route("/v1/views/tree", get(tree_view))
        .route("/v1/views/graph", get(graph_view))
        .route("/v1/views/table", get(table_view))
        .route("/v1/views/timeline", get(timeline_view))
        // Collections
        .route("/v1/collections", get(list_collections).post(create_collection))
        .route("/v1/collections/:id", delete(delete_collection))
        .route("/v1/collections/:id/members", get(get_collection_members))
        .route("/v1/collections/:id/members/:entity_id", post(add_collection_member))
        .with_state(state);

    let listener = tokio::net::TcpListener::bind("0.0.0.0:8080").await.unwrap();
    tracing::info!("listening on {}", listener.local_addr().unwrap());
    axum::serve(listener, app).await.unwrap();
}

// ---- Health ----

async fn health() -> Json<serde_json::Value> {
    Json(serde_json::json!({ "status": "ok" }))
}

// ---- Entities ----

async fn create_entity(
    State(state): State<ApiState>,
    Json(payload): Json<CreateEntityPayload>,
) -> Result<Json<Entity>, ApiError> {
    let entity = Entity::new(EntityType::new(&payload.entity_type));
    let mut components = vec![Component::new(
        entity.id,
        ComponentType::Title,
        serde_json::json!(payload.title),
    )];
    if let Some(content) = payload.content {
        components.push(Component::new(
            entity.id,
            ComponentType::Content,
            serde_json::json!(content),
        ));
    }
    if let Some(tags) = payload.tags {
        components.push(Component::new(
            entity.id,
            ComponentType::Tags,
            serde_json::json!(tags),
        ));
    }
    let event = knowledge_core::ports::Event {
        id: Uuid::new_v4(),
        event_type: knowledge_core::ports::EventType::EntityCreated,
        entity_id: entity.id,
        timestamp: chrono::Utc::now(),
        data: serde_json::json!({"source": "api"}),
    };
    state
        .store
        .save_entity_with_components(&entity, &components, &event)
        .await
        .map_err(|e| ApiError {
            error: e.to_string(),
        })?;
    Ok(Json(entity))
}

async fn list_entities(
    State(state): State<ApiState>,
    Query(params): Query<HashMap<String, String>>,
) -> Result<Json<Vec<EntitySummary>>, ApiError> {
    let entity_type = params.get("type");
    let entities = match entity_type {
        Some(t) => EntityRepository::find_by_type(state.store.as_ref(), t)
            .await
            .map_err(|e| ApiError {
                error: e.to_string(),
            })?,
        None => EntityRepository::list(state.store.as_ref())
            .await
            .map_err(|e| ApiError {
                error: e.to_string(),
            })?,
    };
    let mut result = Vec::new();
    for entity in entities {
        let components = ComponentRepository::get(state.store.as_ref(), entity.id)
            .await
            .map_err(|e| ApiError {
                error: e.to_string(),
            })?;
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
    Ok(Json(result))
}

async fn get_entity(
    State(state): State<ApiState>,
    Path(id): Path<String>,
) -> Result<Json<EntityDetail>, ApiError> {
    let entity_id = Uuid::parse_str(&id).map_err(|e| ApiError {
        error: format!("invalid id: {}", e),
    })?;
    let entity = EntityRepository::get(state.store.as_ref(), entity_id)
        .await
        .map_err(|e| ApiError {
            error: e.to_string(),
        })?
        .ok_or_else(|| ApiError {
            error: "entity not found".to_string(),
        })?;

    let components = ComponentRepository::get(state.store.as_ref(), entity.id)
        .await
        .map_err(|e| ApiError {
            error: e.to_string(),
        })?;

    let outgoing =
        RelationshipRepository::by_source(state.store.as_ref(), entity.id)
            .await
            .map_err(|e| ApiError {
                error: e.to_string(),
            })?
            .into_iter()
            .map(|r| RelationshipData {
                id: r.id.to_string(),
                relationship_type: format!("{:?}", r.relationship_type),
                source_id: r.source_id.to_string(),
                target_id: r.target_id.to_string(),
            })
            .collect();

    let incoming =
        RelationshipRepository::by_target(state.store.as_ref(), entity.id)
            .await
            .map_err(|e| ApiError {
                error: e.to_string(),
            })?
            .into_iter()
            .map(|r| RelationshipData {
                id: r.id.to_string(),
                relationship_type: format!("{:?}", r.relationship_type),
                source_id: r.source_id.to_string(),
                target_id: r.target_id.to_string(),
            })
            .collect();

    let events = EventLog::list_by_entity(state.store.as_ref(), entity.id)
        .await
        .map_err(|e| ApiError {
            error: e.to_string(),
        })?
        .into_iter()
        .map(|e| EventData {
            event_type: format!("{:?}", e.event_type),
            timestamp: e.timestamp.to_rfc3339(),
            data: e.data,
        })
        .collect();

    Ok(Json(EntityDetail {
        id: entity.id.to_string(),
        entity_type: entity.entity_type.to_string(),
        is_active: entity.is_active,
        created_at: entity.created_at.to_rfc3339(),
        updated_at: entity.updated_at.to_rfc3339(),
        components: components
            .into_iter()
            .map(|c| ComponentData {
                component_type: format!("{:?}", c.component_type),
                data: c.data,
            })
            .collect(),
        outgoing_relationships: outgoing,
        incoming_relationships: incoming,
        events,
    }))
}

async fn delete_entity(
    State(state): State<ApiState>,
    Path(id): Path<String>,
) -> Result<Json<serde_json::Value>, ApiError> {
    let entity_id = Uuid::parse_str(&id).map_err(|e| ApiError {
        error: format!("invalid id: {}", e),
    })?;
    EntityRepository::delete(state.store.as_ref(), entity_id)
        .await
        .map_err(|e| ApiError {
            error: e.to_string(),
        })?;
    Ok(Json(serde_json::json!({"status": "deleted"})))
}

async fn archive_entity(
    State(state): State<ApiState>,
    Path(id): Path<String>,
) -> Result<Json<serde_json::Value>, ApiError> {
    let entity_id = Uuid::parse_str(&id).map_err(|e| ApiError {
        error: format!("invalid id: {}", e),
    })?;
    let mut entity = EntityRepository::get(state.store.as_ref(), entity_id)
        .await
        .map_err(|e| ApiError {
            error: e.to_string(),
        })?
        .ok_or_else(|| ApiError {
            error: "entity not found".to_string(),
        })?;
    entity.archive();
    EntityRepository::save(state.store.as_ref(), &entity)
        .await
        .map_err(|e| ApiError {
            error: e.to_string(),
        })?;
    Ok(Json(serde_json::json!({"status": "archived"})))
}

async fn restore_entity(
    State(state): State<ApiState>,
    Path(id): Path<String>,
) -> Result<Json<serde_json::Value>, ApiError> {
    let entity_id = Uuid::parse_str(&id).map_err(|e| ApiError {
        error: format!("invalid id: {}", e),
    })?;
    let mut entity = EntityRepository::get(state.store.as_ref(), entity_id)
        .await
        .map_err(|e| ApiError {
            error: e.to_string(),
        })?
        .ok_or_else(|| ApiError {
            error: "entity not found".to_string(),
        })?;
    entity.restore();
    EntityRepository::save(state.store.as_ref(), &entity)
        .await
        .map_err(|e| ApiError {
            error: e.to_string(),
        })?;
    Ok(Json(serde_json::json!({"status": "restored"})))
}

// ---- Components ----

async fn get_components(
    State(state): State<ApiState>,
    Path(id): Path<String>,
) -> Result<Json<Vec<ComponentData>>, ApiError> {
    let entity_id = Uuid::parse_str(&id).map_err(|e| ApiError {
        error: format!("invalid id: {}", e),
    })?;
    let components = ComponentRepository::get(state.store.as_ref(), entity_id)
        .await
        .map_err(|e| ApiError {
            error: e.to_string(),
        })?;
    Ok(Json(
        components
            .into_iter()
            .map(|c| ComponentData {
                component_type: format!("{:?}", c.component_type),
                data: c.data,
            })
            .collect(),
    ))
}

// ---- Relationships ----

async fn create_relationship(
    State(state): State<ApiState>,
    Json(payload): Json<CreateRelationshipPayload>,
) -> Result<Json<serde_json::Value>, ApiError> {
    let source_id = Uuid::parse_str(&payload.source_id).map_err(|e| ApiError {
        error: format!("invalid source_id: {}", e),
    })?;
    let target_id = Uuid::parse_str(&payload.target_id).map_err(|e| ApiError {
        error: format!("invalid target_id: {}", e),
    })?;
    let rel = Relationship::new(source_id, target_id, RelationshipType::References);
    RelationshipRepository::save(state.store.as_ref(), &rel)
        .await
        .map_err(|e| ApiError {
            error: e.to_string(),
        })?;
    Ok(Json(serde_json::json!({"id": rel.id.to_string()})))
}

async fn get_entity_relationships(
    State(state): State<ApiState>,
    Path(id): Path<String>,
) -> Result<Json<serde_json::Value>, ApiError> {
    let entity_id = Uuid::parse_str(&id).map_err(|e| ApiError {
        error: format!("invalid id: {}", e),
    })?;
    let outgoing: Vec<RelationshipData> =
        RelationshipRepository::by_source(state.store.as_ref(), entity_id)
            .await
            .map_err(|e| ApiError {
                error: e.to_string(),
            })?
            .into_iter()
            .map(|r| RelationshipData {
                id: r.id.to_string(),
                relationship_type: format!("{:?}", r.relationship_type),
                source_id: r.source_id.to_string(),
                target_id: r.target_id.to_string(),
            })
            .collect();
    let incoming: Vec<RelationshipData> =
        RelationshipRepository::by_target(state.store.as_ref(), entity_id)
            .await
            .map_err(|e| ApiError {
                error: e.to_string(),
            })?
            .into_iter()
            .map(|r| RelationshipData {
                id: r.id.to_string(),
                relationship_type: format!("{:?}", r.relationship_type),
                source_id: r.source_id.to_string(),
                target_id: r.target_id.to_string(),
            })
            .collect();
    Ok(Json(serde_json::json!({
        "outgoing": outgoing,
        "incoming": incoming
    })))
}

// ---- Search ----


async fn search_entities(
    State(state): State<ApiState>,
    Query(params): Query<SearchParams>,
) -> Result<Json<Vec<serde_json::Value>>, ApiError> {
    let query = SearchQuery {
        query: params.q,
        entity_type: params.entity_type,
        tag: params.tag,
    };
    let results = SearchIndex::search(state.store.as_ref(), &query)
        .await
        .map_err(|e| ApiError {
            error: e.to_string(),
        })?;

    let mut response = Vec::new();
    for result in results {
        if let Some(entity) = EntityRepository::get(state.store.as_ref(), result.entity_id)
            .await
            .map_err(|e| ApiError {
                error: e.to_string(),
            })? {
            let components = ComponentRepository::get(state.store.as_ref(), entity.id)
                .await
                .map_err(|e| ApiError {
                    error: e.to_string(),
                })?;
            let title = components
                .iter()
                .find(|c| c.component_type == ComponentType::Title)
                .and_then(|c| c.data.as_str().map(String::from))
                .unwrap_or_else(|| "Untitled".to_string());
            response.push(serde_json::json!({
                "entity_id": entity.id.to_string(),
                "entity_type": entity.entity_type.to_string(),
                "title": title,
                "score": result.score,
                "snippet": result.snippet,
            }));
        }
    }
    Ok(Json(response))
}

// ---- Traversal ----

async fn traverse_graph(
    State(state): State<ApiState>,
    Query(params): Query<TraverseParams>,
) -> Result<Json<Vec<TraverseResult>>, ApiError> {
    let start_id = Uuid::parse_str(&params.start_id).map_err(|e| ApiError {
        error: format!("invalid start_id: {}", e),
    })?;
    let direction = match params.direction.as_deref() {
        Some("incoming") => TraversalDirection::Incoming,
        Some("both") => TraversalDirection::Both,
        _ => TraversalDirection::Outgoing,
    };
    let query = TraversalQuery {
        start_id,
        direction,
        max_depth: params.depth,
        max_results: None,
        relationship_type: None,
        entity_type_filter: params.entity_type.map(|t| EntityType::new(&t)),
    };
    let config = TraversalConfig::default();
    let results = TraversalPort::traverse(state.store.as_ref(), &query, &config)
        .await
        .map_err(|e| ApiError {
            error: e.to_string(),
        })?;
    Ok(Json(
        results
            .into_iter()
            .map(|r| {
                let path: Vec<String> = r.path.iter().map(|id| id.to_string()).collect();
                TraverseResult {
                    depth: r.depth,
                    entity_id: r.path.last().copied().unwrap_or(start_id).to_string(),
                    path,
                }
            })
            .collect(),
    ))
}

// ---- Views ----


use async_trait::async_trait;

struct StoreWrapper(Arc<SqliteStore>);

#[async_trait]
impl EntityRepository for StoreWrapper {
    async fn get(&self, id: Uuid) -> Result<Option<Entity>, knowledge_core::ports::StorageError> {
        EntityRepository::get(self.0.as_ref(), id).await
    }
    async fn save(&self, entity: &Entity) -> Result<(), knowledge_core::ports::StorageError> {
        EntityRepository::save(self.0.as_ref(), entity).await
    }
    async fn delete(&self, id: Uuid) -> Result<(), knowledge_core::ports::StorageError> {
        EntityRepository::delete(self.0.as_ref(), id).await
    }
    async fn list(&self) -> Result<Vec<Entity>, knowledge_core::ports::StorageError> {
        EntityRepository::list(self.0.as_ref()).await
    }
    async fn find_by_type(&self, entity_type: &str) -> Result<Vec<Entity>, knowledge_core::ports::StorageError> {
        EntityRepository::find_by_type(self.0.as_ref(), entity_type).await
    }
    async fn find_by_title(&self, title: &str) -> Result<Vec<Entity>, knowledge_core::ports::StorageError> {
        EntityRepository::find_by_title(self.0.as_ref(), title).await
    }
    async fn increment_version(&self, id: Uuid) -> Result<(), knowledge_core::ports::StorageError> {
        EntityRepository::increment_version(self.0.as_ref(), id).await
    }
    async fn find_by_component_type(&self, component_type: &str) -> Result<Vec<Entity>, knowledge_core::ports::StorageError> {
        EntityRepository::find_by_component_type(self.0.as_ref(), component_type).await
    }
    async fn find_by_tag(&self, tag: &str) -> Result<Vec<Entity>, knowledge_core::ports::StorageError> {
        EntityRepository::find_by_tag(self.0.as_ref(), tag).await
    }
    async fn get_version_history(&self, entity_id: Uuid) -> Result<Vec<knowledge_core::ports::EntityVersion>, knowledge_core::ports::StorageError> {
        EntityRepository::get_version_history(self.0.as_ref(), entity_id).await
    }
}

#[async_trait]
impl ComponentRepository for StoreWrapper {
    async fn get(&self, entity_id: Uuid) -> Result<Vec<Component>, knowledge_core::ports::StorageError> {
        ComponentRepository::get(self.0.as_ref(), entity_id).await
    }
    async fn save(&self, component: &Component) -> Result<(), knowledge_core::ports::StorageError> {
        ComponentRepository::save(self.0.as_ref(), component).await
    }
    async fn delete(&self, id: Uuid) -> Result<(), knowledge_core::ports::StorageError> {
        ComponentRepository::delete(self.0.as_ref(), id).await
    }
    async fn find_by_type(&self, entity_id: Uuid, component_type: &str) -> Result<Vec<Component>, knowledge_core::ports::StorageError> {
        ComponentRepository::find_by_type(self.0.as_ref(), entity_id, component_type).await
    }
    async fn update_data(&self, id: Uuid, data: serde_json::Value) -> Result<(), knowledge_core::ports::StorageError> {
        ComponentRepository::update_data(self.0.as_ref(), id, data).await
    }
    async fn find_by_component_data(&self, component_type: &str, json_path: &str, value: &str) -> Result<Vec<Component>, knowledge_core::ports::StorageError> {
        ComponentRepository::find_by_component_data(self.0.as_ref(), component_type, json_path, value).await
    }
    async fn delete_by_entity(&self, entity_id: Uuid) -> Result<(), knowledge_core::ports::StorageError> {
        ComponentRepository::delete_by_entity(self.0.as_ref(), entity_id).await
    }
}

#[async_trait]
impl RelationshipRepository for StoreWrapper {
    async fn get(&self, id: Uuid) -> Result<Option<Relationship>, knowledge_core::ports::StorageError> {
        RelationshipRepository::get(self.0.as_ref(), id).await
    }
    async fn save(&self, relationship: &Relationship) -> Result<(), knowledge_core::ports::StorageError> {
        RelationshipRepository::save(self.0.as_ref(), relationship).await
    }
    async fn update(&self, relationship: &Relationship) -> Result<(), knowledge_core::ports::StorageError> {
        RelationshipRepository::update(self.0.as_ref(), relationship).await
    }
    async fn delete(&self, id: Uuid) -> Result<(), knowledge_core::ports::StorageError> {
        RelationshipRepository::delete(self.0.as_ref(), id).await
    }
    async fn by_source(&self, source_id: Uuid) -> Result<Vec<Relationship>, knowledge_core::ports::StorageError> {
        RelationshipRepository::by_source(self.0.as_ref(), source_id).await
    }
    async fn by_target(&self, target_id: Uuid) -> Result<Vec<Relationship>, knowledge_core::ports::StorageError> {
        RelationshipRepository::by_target(self.0.as_ref(), target_id).await
    }
    async fn find_by_source_and_target(&self, source_id: Uuid, target_id: Uuid) -> Result<Option<Relationship>, knowledge_core::ports::StorageError> {
        RelationshipRepository::find_by_source_and_target(self.0.as_ref(), source_id, target_id).await
    }
    async fn find_by_type(&self, relationship_type: &str) -> Result<Vec<Relationship>, knowledge_core::ports::StorageError> {
        RelationshipRepository::find_by_type(self.0.as_ref(), relationship_type).await
    }
}

#[async_trait]
impl TraversalPort for StoreWrapper {
    async fn traverse(&self, query: &TraversalQuery, config: &TraversalConfig) -> Result<Vec<knowledge_core::ports::TraversalResult>, knowledge_core::ports::TraversalError> {
        TraversalPort::traverse(self.0.as_ref(), query, config).await
    }
}

#[async_trait]
impl CollectionRepository for StoreWrapper {
    async fn create(&self, collection: Collection) -> Result<Collection, knowledge_core::ports::StorageError> {
        CollectionRepository::create(self.0.as_ref(), collection).await
    }
    async fn get(&self, id: Uuid) -> Result<Option<Collection>, knowledge_core::ports::StorageError> {
        CollectionRepository::get(self.0.as_ref(), id).await
    }
    async fn update(&self, collection: Collection) -> Result<Collection, knowledge_core::ports::StorageError> {
        CollectionRepository::update(self.0.as_ref(), collection).await
    }
    async fn delete(&self, id: Uuid) -> Result<(), knowledge_core::ports::StorageError> {
        CollectionRepository::delete(self.0.as_ref(), id).await
    }
    async fn list(&self) -> Result<Vec<Collection>, knowledge_core::ports::StorageError> {
        CollectionRepository::list(self.0.as_ref()).await
    }
    async fn add_member(&self, collection_id: Uuid, entity_id: Uuid) -> Result<(), knowledge_core::ports::StorageError> {
        CollectionRepository::add_member(self.0.as_ref(), collection_id, entity_id).await
    }
    async fn remove_member(&self, collection_id: Uuid, entity_id: Uuid) -> Result<(), knowledge_core::ports::StorageError> {
        CollectionRepository::remove_member(self.0.as_ref(), collection_id, entity_id).await
    }
    async fn get_members(&self, collection_id: Uuid) -> Result<Vec<Entity>, knowledge_core::ports::StorageError> {
        CollectionRepository::get_members(self.0.as_ref(), collection_id).await
    }
    async fn get_entity_collections(&self, entity_id: Uuid) -> Result<Vec<Collection>, knowledge_core::ports::StorageError> {
        CollectionRepository::get_entity_collections(self.0.as_ref(), entity_id).await
    }
    async fn is_member(&self, collection_id: Uuid, entity_id: Uuid) -> Result<bool, knowledge_core::ports::StorageError> {
        CollectionRepository::is_member(self.0.as_ref(), collection_id, entity_id).await
    }
}

fn build_view_registry(state: &ApiState) -> ViewRegistry {
    let mut registry = ViewRegistry::new();
    registry.register(Box::new(TreeViewAdapter::new(
        Box::new(StoreWrapper(state.store.clone())),
        Box::new(StoreWrapper(state.store.clone())),
        Some(Box::new(StoreWrapper(state.store.clone()))),
    )));
    registry.register(Box::new(GraphViewAdapter::new(
        Box::new(StoreWrapper(state.store.clone())),
        Box::new(StoreWrapper(state.store.clone())),
        Box::new(StoreWrapper(state.store.clone())),
        Box::new(StoreWrapper(state.store.clone())),
    )));
    registry.register(Box::new(TableViewAdapter::new(
        Box::new(StoreWrapper(state.store.clone())),
        Box::new(StoreWrapper(state.store.clone())),
    )));
    registry.register(Box::new(TimelineViewAdapter::new(
        Box::new(StoreWrapper(state.store.clone())),
        Box::new(StoreWrapper(state.store.clone())),
    )));
    registry
}

async fn tree_view(
    State(state): State<ApiState>,
) -> Result<Json<serde_json::Value>, ApiError> {
    let registry = build_view_registry(&state);
    let filter = ViewFilter::default();
    let output = registry.render("tree", &filter).await.map_err(|e| ApiError {
        error: e.to_string(),
    })?;
    match output {
        ViewOutput::Tree(data) => Ok(Json(serde_json::json!(data))),
        _ => Err(ApiError { error: "unexpected view output".into() }),
    }
}

async fn graph_view(
    State(state): State<ApiState>,
    Query(params): Query<HashMap<String, String>>,
) -> Result<Json<serde_json::Value>, ApiError> {
    let registry = build_view_registry(&state);
    let filter = ViewFilter {
        start_entity_id: params.get("start_id").cloned(),
        max_depth: params.get("depth").and_then(|d| d.parse().ok()),
        ..Default::default()
    };
    let output = registry.render("graph", &filter).await.map_err(|e| ApiError {
        error: e.to_string(),
    })?;
    match output {
        ViewOutput::Graph(data) => Ok(Json(serde_json::json!(data))),
        _ => Err(ApiError { error: "unexpected view output".into() }),
    }
}

async fn table_view(
    State(state): State<ApiState>,
) -> Result<Json<serde_json::Value>, ApiError> {
    let registry = build_view_registry(&state);
    let filter = ViewFilter::default();
    let output = registry.render("table", &filter).await.map_err(|e| ApiError {
        error: e.to_string(),
    })?;
    match output {
        ViewOutput::Table(data) => Ok(Json(serde_json::json!(data))),
        _ => Err(ApiError { error: "unexpected view output".into() }),
    }
}

async fn timeline_view(
    State(state): State<ApiState>,
) -> Result<Json<serde_json::Value>, ApiError> {
    let registry = build_view_registry(&state);
    let filter = ViewFilter::default();
    let output = registry.render("timeline", &filter).await.map_err(|e| ApiError {
        error: e.to_string(),
    })?;
    match output {
        ViewOutput::Timeline(data) => Ok(Json(serde_json::json!(data))),
        _ => Err(ApiError { error: "unexpected view output".into() }),
    }
}

// ---- Collections ----

async fn list_collections(
    State(state): State<ApiState>,
) -> Result<Json<Vec<CollectionSummary>>, ApiError> {
    let collections = CollectionRepository::list(state.store.as_ref())
        .await
        .map_err(|e| ApiError {
            error: e.to_string(),
        })?;
    let mut result = Vec::new();
    for c in collections {
        let members = CollectionRepository::get_members(state.store.as_ref(), c.id)
            .await
            .map_err(|e| ApiError {
                error: e.to_string(),
            })?;
        result.push(CollectionSummary {
            id: c.id.to_string(),
            name: c.name,
            description: c.description,
            member_count: members.len(),
            created_at: c.created_at.to_rfc3339(),
        });
    }
    Ok(Json(result))
}

async fn create_collection(
    State(state): State<ApiState>,
    Json(payload): Json<CollectionPayload>,
) -> Result<Json<serde_json::Value>, ApiError> {
    let now = chrono::Utc::now();
    let collection = Collection {
        id: Uuid::new_v4(),
        name: payload.name,
        description: payload.description,
        created_at: now,
        updated_at: now,
    };
    let created = CollectionRepository::create(state.store.as_ref(), collection)
        .await
        .map_err(|e| ApiError {
            error: e.to_string(),
        })?;
    Ok(Json(serde_json::json!({
        "id": created.id.to_string(),
        "name": created.name
    })))
}

async fn delete_collection(
    State(state): State<ApiState>,
    Path(id): Path<String>,
) -> Result<Json<serde_json::Value>, ApiError> {
    let coll_id = Uuid::parse_str(&id).map_err(|e| ApiError {
        error: format!("invalid id: {}", e),
    })?;
    CollectionRepository::delete(state.store.as_ref(), coll_id)
        .await
        .map_err(|e| ApiError {
            error: e.to_string(),
        })?;
    Ok(Json(serde_json::json!({"status": "deleted"})))
}

async fn add_collection_member(
    State(state): State<ApiState>,
    Path((collection_id, entity_id)): Path<(String, String)>,
) -> Result<Json<serde_json::Value>, ApiError> {
    let coll_id = Uuid::parse_str(&collection_id).map_err(|e| ApiError {
        error: format!("invalid collection id: {}", e),
    })?;
    let ent_id = Uuid::parse_str(&entity_id).map_err(|e| ApiError {
        error: format!("invalid entity id: {}", e),
    })?;
    CollectionRepository::add_member(state.store.as_ref(), coll_id, ent_id)
        .await
        .map_err(|e| ApiError {
            error: e.to_string(),
        })?;
    Ok(Json(serde_json::json!({"status": "added"})))
}

async fn get_collection_members(
    State(state): State<ApiState>,
    Path(id): Path<String>,
) -> Result<Json<Vec<EntitySummary>>, ApiError> {
    let coll_id = Uuid::parse_str(&id).map_err(|e| ApiError {
        error: format!("invalid id: {}", e),
    })?;
    let members = CollectionRepository::get_members(state.store.as_ref(), coll_id)
        .await
        .map_err(|e| ApiError {
            error: e.to_string(),
        })?;
    let mut result = Vec::new();
    for entity in members {
        let components = ComponentRepository::get(state.store.as_ref(), entity.id)
            .await
            .map_err(|e| ApiError {
                error: e.to_string(),
            })?;
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
    Ok(Json(result))
}
