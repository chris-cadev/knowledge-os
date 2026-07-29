use async_trait::async_trait;
use knowledge_core::features::component::Component;
use knowledge_core::features::entity::Entity;
use knowledge_core::features::relationship::Relationship;
use knowledge_core::ports::{
    Collection, CollectionRepository, ComponentRepository, EntityRepository,
    EntityVersion, RelationshipRepository, SearchIndex, SearchQuery, SearchResult, StorageError,
    TraversalConfig, TraversalError, TraversalPort, TraversalQuery, TraversalResult,
};
use knowledge_derivation::features::chat::pipeline::ChatPipeline;
use knowledge_storage::adapters::sqlite::SqliteStore;
use std::sync::Arc;
use uuid::Uuid;

/// Shared application state managed by Tauri.
///
/// The inner `SqliteStore` already uses `Mutex<Connection>` for thread safety,
/// so no additional synchronization is needed around the `Arc`.
pub struct AppState {
    pub store: Arc<SqliteStore>,
    pub chat_pipeline: Arc<ChatPipeline>,
    pub chat_provider_kind: String,
}

/// Wraps `Arc<SqliteStore>` to implement port traits for view adapter
/// constructors. Follows the same pattern used by the CLI
/// (`cli/src/main.rs` `StoreWrapper`).
pub struct StoreWrapper(pub Arc<SqliteStore>);

// ---------------------------------------------------------------------------
// EntityRepository delegation
// ---------------------------------------------------------------------------

#[async_trait]
impl EntityRepository for StoreWrapper {
    async fn get(&self, id: Uuid) -> Result<Option<Entity>, StorageError> {
        EntityRepository::get(self.0.as_ref(), id).await
    }
    async fn save(&self, entity: &Entity) -> Result<(), StorageError> {
        EntityRepository::save(self.0.as_ref(), entity).await
    }
    async fn delete(&self, id: Uuid) -> Result<(), StorageError> {
        EntityRepository::delete(self.0.as_ref(), id).await
    }
    async fn list(&self) -> Result<Vec<Entity>, StorageError> {
        EntityRepository::list(self.0.as_ref()).await
    }
    async fn find_by_type(&self, entity_type: &str) -> Result<Vec<Entity>, StorageError> {
        EntityRepository::find_by_type(self.0.as_ref(), entity_type).await
    }
    async fn find_by_title(&self, title: &str) -> Result<Vec<Entity>, StorageError> {
        EntityRepository::find_by_title(self.0.as_ref(), title).await
    }
    async fn increment_version(&self, id: Uuid) -> Result<(), StorageError> {
        EntityRepository::increment_version(self.0.as_ref(), id).await
    }
    async fn find_by_component_type(
        &self,
        component_type: &str,
    ) -> Result<Vec<Entity>, StorageError> {
        EntityRepository::find_by_component_type(self.0.as_ref(), component_type).await
    }
    async fn find_by_tag(&self, tag: &str) -> Result<Vec<Entity>, StorageError> {
        EntityRepository::find_by_tag(self.0.as_ref(), tag).await
    }
    async fn get_version_history(
        &self,
        entity_id: Uuid,
    ) -> Result<Vec<EntityVersion>, StorageError> {
        EntityRepository::get_version_history(self.0.as_ref(), entity_id).await
    }
}

// ---------------------------------------------------------------------------
// ComponentRepository delegation
// ---------------------------------------------------------------------------

#[async_trait]
impl ComponentRepository for StoreWrapper {
    async fn get(&self, entity_id: Uuid) -> Result<Vec<Component>, StorageError> {
        ComponentRepository::get(self.0.as_ref(), entity_id).await
    }
    async fn save(&self, component: &Component) -> Result<(), StorageError> {
        ComponentRepository::save(self.0.as_ref(), component).await
    }
    async fn delete(&self, id: Uuid) -> Result<(), StorageError> {
        ComponentRepository::delete(self.0.as_ref(), id).await
    }
    async fn find_by_type(
        &self,
        entity_id: Uuid,
        component_type: &str,
    ) -> Result<Vec<Component>, StorageError> {
        ComponentRepository::find_by_type(self.0.as_ref(), entity_id, component_type).await
    }
    async fn update_data(&self, id: Uuid, data: serde_json::Value) -> Result<(), StorageError> {
        ComponentRepository::update_data(self.0.as_ref(), id, data).await
    }
    async fn find_by_component_data(
        &self,
        component_type: &str,
        json_path: &str,
        value: &str,
    ) -> Result<Vec<Component>, StorageError> {
        ComponentRepository::find_by_component_data(
            self.0.as_ref(),
            component_type,
            json_path,
            value,
        )
        .await
    }
    async fn delete_by_entity(&self, entity_id: Uuid) -> Result<(), StorageError> {
        ComponentRepository::delete_by_entity(self.0.as_ref(), entity_id).await
    }
}

// ---------------------------------------------------------------------------
// RelationshipRepository delegation
// ---------------------------------------------------------------------------

#[async_trait]
impl RelationshipRepository for StoreWrapper {
    async fn get(&self, id: Uuid) -> Result<Option<Relationship>, StorageError> {
        RelationshipRepository::get(self.0.as_ref(), id).await
    }
    async fn save(&self, relationship: &Relationship) -> Result<(), StorageError> {
        RelationshipRepository::save(self.0.as_ref(), relationship).await
    }
    async fn update(&self, relationship: &Relationship) -> Result<(), StorageError> {
        RelationshipRepository::update(self.0.as_ref(), relationship).await
    }
    async fn delete(&self, id: Uuid) -> Result<(), StorageError> {
        RelationshipRepository::delete(self.0.as_ref(), id).await
    }
    async fn by_source(&self, source_id: Uuid) -> Result<Vec<Relationship>, StorageError> {
        RelationshipRepository::by_source(self.0.as_ref(), source_id).await
    }
    async fn by_target(&self, target_id: Uuid) -> Result<Vec<Relationship>, StorageError> {
        RelationshipRepository::by_target(self.0.as_ref(), target_id).await
    }
    async fn find_by_source_and_target(
        &self,
        source_id: Uuid,
        target_id: Uuid,
    ) -> Result<Option<Relationship>, StorageError> {
        RelationshipRepository::find_by_source_and_target(self.0.as_ref(), source_id, target_id)
            .await
    }
    async fn find_by_type(
        &self,
        relationship_type: &str,
    ) -> Result<Vec<Relationship>, StorageError> {
        RelationshipRepository::find_by_type(self.0.as_ref(), relationship_type).await
    }
}

// ---------------------------------------------------------------------------
// TraversalPort delegation
// ---------------------------------------------------------------------------

#[async_trait]
impl TraversalPort for StoreWrapper {
    async fn traverse(
        &self,
        query: &TraversalQuery,
        config: &TraversalConfig,
    ) -> Result<Vec<TraversalResult>, TraversalError> {
        TraversalPort::traverse(self.0.as_ref(), query, config).await
    }
}

// ---------------------------------------------------------------------------
// CollectionRepository delegation
// ---------------------------------------------------------------------------

#[async_trait]
impl CollectionRepository for StoreWrapper {
    async fn create(&self, collection: Collection) -> Result<Collection, StorageError> {
        CollectionRepository::create(self.0.as_ref(), collection).await
    }
    async fn get(&self, id: Uuid) -> Result<Option<Collection>, StorageError> {
        CollectionRepository::get(self.0.as_ref(), id).await
    }
    async fn update(&self, collection: Collection) -> Result<Collection, StorageError> {
        CollectionRepository::update(self.0.as_ref(), collection).await
    }
    async fn delete(&self, id: Uuid) -> Result<(), StorageError> {
        CollectionRepository::delete(self.0.as_ref(), id).await
    }
    async fn list(&self) -> Result<Vec<Collection>, StorageError> {
        CollectionRepository::list(self.0.as_ref()).await
    }
    async fn add_member(&self, collection_id: Uuid, entity_id: Uuid) -> Result<(), StorageError> {
        CollectionRepository::add_member(self.0.as_ref(), collection_id, entity_id).await
    }
    async fn remove_member(
        &self,
        collection_id: Uuid,
        entity_id: Uuid,
    ) -> Result<(), StorageError> {
        CollectionRepository::remove_member(self.0.as_ref(), collection_id, entity_id).await
    }
    async fn get_members(&self, collection_id: Uuid) -> Result<Vec<Entity>, StorageError> {
        CollectionRepository::get_members(self.0.as_ref(), collection_id).await
    }
    async fn get_entity_collections(
        &self,
        entity_id: Uuid,
    ) -> Result<Vec<Collection>, StorageError> {
        CollectionRepository::get_entity_collections(self.0.as_ref(), entity_id).await
    }
    async fn is_member(&self, collection_id: Uuid, entity_id: Uuid) -> Result<bool, StorageError> {
        CollectionRepository::is_member(self.0.as_ref(), collection_id, entity_id).await
    }
}

// ---------------------------------------------------------------------------
// SearchIndex delegation
// ---------------------------------------------------------------------------

#[async_trait]
impl SearchIndex for StoreWrapper {
    async fn index_entity(
        &self,
        entity: &Entity,
        components: &[Component],
    ) -> Result<(), StorageError> {
        SearchIndex::index_entity(self.0.as_ref(), entity, components).await
    }
    async fn remove_entity(&self, entity_id: Uuid) -> Result<(), StorageError> {
        SearchIndex::remove_entity(self.0.as_ref(), entity_id).await
    }
    async fn search(&self, query: &SearchQuery) -> Result<Vec<SearchResult>, StorageError> {
        SearchIndex::search(self.0.as_ref(), query).await
    }
    async fn rebuild(&self, entities: &[(Entity, Vec<Component>)]) -> Result<(), StorageError> {
        SearchIndex::rebuild(self.0.as_ref(), entities).await
    }
}
