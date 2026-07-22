use axum::{
    extract::State,
    routing::{get, post},
    Json, Router,
};
use knowledge_core::features::entity::{Entity, EntityType};
use knowledge_core::ports::EntityRepository;
use knowledge_storage::adapters::sqlite::SqliteStore;
use std::sync::Arc;

#[derive(Clone)]
pub struct AppState {
    pub store: Arc<dyn EntityRepository>,
}

#[tokio::main]
async fn main() {
    tracing_subscriber::fmt::init();

    let store = Arc::new(SqliteStore::new("knowledge.db").expect("failed to open database"));
    let state = AppState { store };

    let app = Router::new()
        .route("/health", get(health))
        .route("/v1/entities", post(create_entity).get(list_entities))
        .route("/v1/entities/{id}", get(get_entity))
        .with_state(state);

    let listener = tokio::net::TcpListener::bind("0.0.0.0:8080").await.unwrap();
    tracing::info!("listening on {}", listener.local_addr().unwrap());
    axum::serve(listener, app).await.unwrap();
}

async fn health() -> Json<serde_json::Value> {
    Json(serde_json::json!({ "status": "ok" }))
}

async fn create_entity(
    State(state): State<AppState>,
    Json(_payload): Json<serde_json::Value>,
) -> Json<Entity> {
    let entity = Entity::new(EntityType::Note);
    state.store.save(&entity).await.unwrap();
    Json(entity)
}

async fn list_entities(State(state): State<AppState>) -> Json<Vec<Entity>> {
    let entities = state.store.list().await.unwrap();
    Json(entities)
}

async fn get_entity(
    State(state): State<AppState>,
    axum::extract::Path(id): axum::extract::Path<String>,
) -> Json<Entity> {
    let id = uuid::Uuid::parse_str(&id).unwrap();
    let entity = state.store.get(id).await.unwrap().unwrap();
    Json(entity)
}
