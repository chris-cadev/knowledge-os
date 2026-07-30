use std::sync::Arc;

use async_trait::async_trait;
use knowledge_core::features::component::Component;
use knowledge_core::features::entity::Entity;
use knowledge_core::ports::*;
use knowledge_derivation::features::chat::mock::MockChatAdapter;
use knowledge_derivation::features::chat::pipeline::ChatPipeline;
use uuid::Uuid;

struct DummyEntityRepo;

#[async_trait]
impl EntityRepository for DummyEntityRepo {
    async fn get(&self, _id: Uuid) -> Result<Option<Entity>, StorageError> {
        Ok(Some(Entity::new(
            knowledge_core::features::entity::EntityType::new("Conversation"),
        )))
    }
    async fn save(&self, _entity: &Entity) -> Result<(), StorageError> {
        Ok(())
    }
    async fn delete(&self, _id: Uuid) -> Result<(), StorageError> {
        Ok(())
    }
    async fn list(&self) -> Result<Vec<Entity>, StorageError> {
        Ok(vec![])
    }
    async fn find_by_type(&self, _et: &str) -> Result<Vec<Entity>, StorageError> {
        Ok(vec![])
    }
    async fn find_by_title(&self, _t: &str) -> Result<Vec<Entity>, StorageError> {
        Ok(vec![])
    }
    async fn increment_version(&self, _id: Uuid) -> Result<(), StorageError> {
        Ok(())
    }
    async fn find_by_component_type(&self, _ct: &str) -> Result<Vec<Entity>, StorageError> {
        Ok(vec![])
    }
    async fn find_by_tag(&self, _tag: &str) -> Result<Vec<Entity>, StorageError> {
        Ok(vec![])
    }
    async fn get_version_history(&self, _eid: Uuid) -> Result<Vec<EntityVersion>, StorageError> {
        Ok(vec![])
    }
}

struct DummyComponentRepo;

#[async_trait]
impl ComponentRepository for DummyComponentRepo {
    async fn get(&self, _eid: Uuid) -> Result<Vec<Component>, StorageError> {
        Ok(vec![])
    }
    async fn save(&self, _c: &Component) -> Result<(), StorageError> {
        Ok(())
    }
    async fn delete(&self, _id: Uuid) -> Result<(), StorageError> {
        Ok(())
    }
    async fn find_by_type(&self, _eid: Uuid, _ct: &str) -> Result<Vec<Component>, StorageError> {
        Ok(vec![])
    }
    async fn update_data(&self, _id: Uuid, _data: serde_json::Value) -> Result<(), StorageError> {
        Ok(())
    }
    async fn find_by_component_data(
        &self,
        _ct: &str,
        _jp: &str,
        _v: &str,
    ) -> Result<Vec<Component>, StorageError> {
        Ok(vec![])
    }
    async fn delete_by_entity(&self, _eid: Uuid) -> Result<(), StorageError> {
        Ok(())
    }
}

struct DummyRelRepo;

#[async_trait]
impl RelationshipRepository for DummyRelRepo {
    async fn get(
        &self,
        _id: Uuid,
    ) -> Result<Option<knowledge_core::features::relationship::Relationship>, StorageError> {
        Ok(None)
    }
    async fn save(
        &self,
        _r: &knowledge_core::features::relationship::Relationship,
    ) -> Result<(), StorageError> {
        Ok(())
    }
    async fn update(
        &self,
        _r: &knowledge_core::features::relationship::Relationship,
    ) -> Result<(), StorageError> {
        Ok(())
    }
    async fn delete(&self, _id: Uuid) -> Result<(), StorageError> {
        Ok(())
    }
    async fn by_source(
        &self,
        _sid: Uuid,
    ) -> Result<Vec<knowledge_core::features::relationship::Relationship>, StorageError> {
        Ok(vec![])
    }
    async fn by_target(
        &self,
        _tid: Uuid,
    ) -> Result<Vec<knowledge_core::features::relationship::Relationship>, StorageError> {
        Ok(vec![])
    }
    async fn find_by_source_and_target(
        &self,
        _sid: Uuid,
        _tid: Uuid,
    ) -> Result<Option<knowledge_core::features::relationship::Relationship>, StorageError> {
        Ok(None)
    }
    async fn find_by_type(
        &self,
        _rt: &str,
    ) -> Result<Vec<knowledge_core::features::relationship::Relationship>, StorageError> {
        Ok(vec![])
    }
}

struct DummySearchIndex;

#[async_trait]
impl SearchIndex for DummySearchIndex {
    async fn index_entity(&self, _e: &Entity, _c: &[Component]) -> Result<(), StorageError> {
        Ok(())
    }
    async fn remove_entity(&self, _eid: Uuid) -> Result<(), StorageError> {
        Ok(())
    }
    async fn search(&self, _q: &SearchQuery) -> Result<Vec<SearchResult>, StorageError> {
        Ok(vec![])
    }
    async fn rebuild(&self, _e: &[(Entity, Vec<Component>)]) -> Result<(), StorageError> {
        Ok(())
    }
}

struct DummyVectorStore;

#[async_trait]
impl VectorStore for DummyVectorStore {
    async fn upsert(
        &self,
        _eid: &str,
        _v: &[f32],
        _m: Option<VectorMetadata>,
    ) -> Result<(), VectorError> {
        Ok(())
    }
    async fn search(
        &self,
        _q: &[f32],
        _k: usize,
        _f: Option<VectorFilter>,
    ) -> Result<Vec<VectorResult>, VectorError> {
        Ok(vec![])
    }
    async fn delete(&self, _eid: &str) -> Result<(), VectorError> {
        Ok(())
    }
    async fn rebuild(&self) -> Result<(), VectorError> {
        Ok(())
    }
}

#[tokio::test]
async fn chat_pipeline_constructs_without_tauri() {
    let chat_provider = Arc::new(MockChatAdapter::default());
    let entity_repo = Arc::new(DummyEntityRepo);
    let component_repo = Arc::new(DummyComponentRepo);
    let relationship_repo = Arc::new(DummyRelRepo);
    let search_index = Arc::new(DummySearchIndex);
    let vector_store = Arc::new(DummyVectorStore);

    let pipeline = ChatPipeline::new(
        chat_provider,
        entity_repo as Arc<dyn EntityRepository>,
        component_repo as Arc<dyn ComponentRepository>,
        relationship_repo as Arc<dyn RelationshipRepository>,
        search_index as Arc<dyn SearchIndex>,
        vector_store as Arc<dyn VectorStore>,
    );

    let result = pipeline
        .chat(
            None,
            "test",
            &[],
            &SourceToggles::default(),
            ResponseMode::Fast,
        )
        .await;
    assert!(result.is_ok());
}
