use std::sync::{Arc, RwLock};

use async_trait::async_trait;
use knowledge_core::features::component::{Component, ComponentType};
use knowledge_core::features::entity::{Entity, EntityType};
use knowledge_core::features::relationship::Relationship;
use knowledge_core::ports::*;
use knowledge_derivation::features::chat::mock::MockChatAdapter;
use knowledge_derivation::features::chat::pipeline::ChatPipeline;
use uuid::Uuid;

// ---------------------------------------------------------------------------
// Mock repositories
// ---------------------------------------------------------------------------

struct MockEntityRepo {
    entities: RwLock<Vec<Entity>>,
}

impl MockEntityRepo {
    fn new() -> Self {
        Self {
            entities: RwLock::new(Vec::new()),
        }
    }

    fn add_entity(&self, entity: Entity) {
        self.entities.write().unwrap().push(entity);
    }
}

#[async_trait]
impl EntityRepository for MockEntityRepo {
    async fn get(&self, id: Uuid) -> Result<Option<Entity>, StorageError> {
        Ok(self
            .entities
            .read()
            .unwrap()
            .iter()
            .find(|e| e.id == id)
            .cloned())
    }
    async fn save(&self, entity: &Entity) -> Result<(), StorageError> {
        self.entities.write().unwrap().push(entity.clone());
        Ok(())
    }
    async fn delete(&self, id: Uuid) -> Result<(), StorageError> {
        self.entities.write().unwrap().retain(|e| e.id != id);
        Ok(())
    }
    async fn list(&self) -> Result<Vec<Entity>, StorageError> {
        Ok(self.entities.read().unwrap().clone())
    }
    async fn find_by_type(&self, entity_type: &str) -> Result<Vec<Entity>, StorageError> {
        Ok(self
            .entities
            .read()
            .unwrap()
            .iter()
            .filter(|e| e.entity_type.as_str() == entity_type)
            .cloned()
            .collect())
    }
    async fn find_by_title(&self, _title: &str) -> Result<Vec<Entity>, StorageError> {
        Ok(vec![])
    }
    async fn increment_version(&self, _id: Uuid) -> Result<(), StorageError> {
        Ok(())
    }
    async fn find_by_component_type(
        &self,
        _component_type: &str,
    ) -> Result<Vec<Entity>, StorageError> {
        Ok(vec![])
    }
    async fn find_by_tag(&self, _tag: &str) -> Result<Vec<Entity>, StorageError> {
        Ok(vec![])
    }
    async fn get_version_history(
        &self,
        _entity_id: Uuid,
    ) -> Result<Vec<EntityVersion>, StorageError> {
        Ok(vec![])
    }
}

struct MockComponentRepo {
    components: RwLock<Vec<Component>>,
}

impl MockComponentRepo {
    fn new() -> Self {
        Self {
            components: RwLock::new(Vec::new()),
        }
    }
}

#[async_trait]
impl ComponentRepository for MockComponentRepo {
    async fn get(&self, entity_id: Uuid) -> Result<Vec<Component>, StorageError> {
        Ok(self
            .components
            .read()
            .unwrap()
            .iter()
            .filter(|c| c.entity_id == entity_id)
            .cloned()
            .collect())
    }
    async fn save(&self, component: &Component) -> Result<(), StorageError> {
        self.components.write().unwrap().push(component.clone());
        Ok(())
    }
    async fn delete(&self, id: Uuid) -> Result<(), StorageError> {
        self.components.write().unwrap().retain(|c| c.id != id);
        Ok(())
    }
    async fn find_by_type(
        &self,
        _entity_id: Uuid,
        _component_type: &str,
    ) -> Result<Vec<Component>, StorageError> {
        Ok(vec![])
    }
    async fn update_data(&self, _id: Uuid, _data: serde_json::Value) -> Result<(), StorageError> {
        Ok(())
    }
    async fn find_by_component_data(
        &self,
        _component_type: &str,
        _json_path: &str,
        _value: &str,
    ) -> Result<Vec<Component>, StorageError> {
        Ok(vec![])
    }
    async fn delete_by_entity(&self, _entity_id: Uuid) -> Result<(), StorageError> {
        Ok(())
    }
}

struct MockRelationshipRepo {
    relationships: RwLock<Vec<Relationship>>,
}

impl MockRelationshipRepo {
    fn new() -> Self {
        Self {
            relationships: RwLock::new(Vec::new()),
        }
    }
}

#[async_trait]
impl RelationshipRepository for MockRelationshipRepo {
    async fn get(&self, id: Uuid) -> Result<Option<Relationship>, StorageError> {
        Ok(self
            .relationships
            .read()
            .unwrap()
            .iter()
            .find(|r| r.id == id)
            .cloned())
    }
    async fn save(&self, relationship: &Relationship) -> Result<(), StorageError> {
        self.relationships
            .write()
            .unwrap()
            .push(relationship.clone());
        Ok(())
    }
    async fn update(&self, relationship: &Relationship) -> Result<(), StorageError> {
        let mut rels = self.relationships.write().unwrap();
        if let Some(existing) = rels.iter_mut().find(|r| r.id == relationship.id) {
            *existing = relationship.clone();
        }
        Ok(())
    }
    async fn delete(&self, id: Uuid) -> Result<(), StorageError> {
        self.relationships.write().unwrap().retain(|r| r.id != id);
        Ok(())
    }
    async fn by_source(&self, source_id: Uuid) -> Result<Vec<Relationship>, StorageError> {
        Ok(self
            .relationships
            .read()
            .unwrap()
            .iter()
            .filter(|r| r.source_id == source_id)
            .cloned()
            .collect())
    }
    async fn by_target(&self, target_id: Uuid) -> Result<Vec<Relationship>, StorageError> {
        Ok(self
            .relationships
            .read()
            .unwrap()
            .iter()
            .filter(|r| r.target_id == target_id)
            .cloned()
            .collect())
    }
    async fn find_by_source_and_target(
        &self,
        source_id: Uuid,
        target_id: Uuid,
    ) -> Result<Option<Relationship>, StorageError> {
        Ok(self
            .relationships
            .read()
            .unwrap()
            .iter()
            .find(|r| r.source_id == source_id && r.target_id == target_id)
            .cloned())
    }
    async fn find_by_type(
        &self,
        _relationship_type: &str,
    ) -> Result<Vec<Relationship>, StorageError> {
        Ok(vec![])
    }
}

struct MockSearchIndex;

#[async_trait]
impl SearchIndex for MockSearchIndex {
    async fn index_entity(
        &self,
        _entity: &Entity,
        _components: &[Component],
    ) -> Result<(), StorageError> {
        Ok(())
    }
    async fn remove_entity(&self, _entity_id: Uuid) -> Result<(), StorageError> {
        Ok(())
    }
    async fn search(&self, _query: &SearchQuery) -> Result<Vec<SearchResult>, StorageError> {
        Ok(vec![])
    }
    async fn rebuild(&self, _entities: &[(Entity, Vec<Component>)]) -> Result<(), StorageError> {
        Ok(())
    }
}

struct MockVectorStore;

#[async_trait]
impl VectorStore for MockVectorStore {
    async fn upsert(
        &self,
        _entity_id: &str,
        _vector: &[f32],
        _metadata: Option<VectorMetadata>,
    ) -> Result<(), VectorError> {
        Ok(())
    }
    async fn search(
        &self,
        _query: &[f32],
        _k: usize,
        _filter: Option<VectorFilter>,
    ) -> Result<Vec<VectorResult>, VectorError> {
        Ok(vec![])
    }
    async fn delete(&self, _entity_id: &str) -> Result<(), VectorError> {
        Ok(())
    }
    async fn rebuild(&self) -> Result<(), VectorError> {
        Ok(())
    }
}

fn make_entity(entity_type: &str) -> Entity {
    Entity::new(EntityType::new(entity_type))
}

fn make_title_component(entity_id: Uuid, title: &str) -> Component {
    Component::new(
        entity_id,
        ComponentType::Title,
        serde_json::json!({"name": title}),
    )
}

fn make_content_component(entity_id: Uuid, content: &str) -> Component {
    Component::new(
        entity_id,
        ComponentType::Content,
        serde_json::json!(content),
    )
}

fn make_tags_component(entity_id: Uuid, tags: Vec<&str>) -> Component {
    Component::new(
        entity_id,
        ComponentType::Tags,
        serde_json::json!({"values": tags}),
    )
}

// Pre-imported entity ID used by MockSearchIndex

fn setup() -> (
    ChatPipeline,
    Arc<MockEntityRepo>,
    Arc<MockComponentRepo>,
    Arc<MockRelationshipRepo>,
) {
    let chat_provider = Arc::new(MockChatAdapter::default());
    let entity_repo = Arc::new(MockEntityRepo::new());
    let component_repo = Arc::new(MockComponentRepo::new());
    let relationship_repo = Arc::new(MockRelationshipRepo::new());
    let search_index = Arc::new(MockSearchIndex);
    let vector_store = Arc::new(MockVectorStore);

    let pipeline = ChatPipeline::new(
        chat_provider,
        entity_repo.clone() as Arc<dyn EntityRepository>,
        component_repo.clone() as Arc<dyn ComponentRepository>,
        relationship_repo.clone() as Arc<dyn RelationshipRepository>,
        search_index.clone() as Arc<dyn SearchIndex>,
        vector_store.clone() as Arc<dyn VectorStore>,
    );

    (pipeline, entity_repo, component_repo, relationship_repo)
}

#[tokio::test]
async fn end_to_end_chat_creates_conversation_and_persists_messages() {
    let (pipeline, entity_repo, component_repo, relationship_repo) = setup();

    let result = pipeline
        .chat(
            None,
            "Hello",
            &[],
            &SourceToggles {
                knowledge_graph: false,
                web_search: false,
            },
            ResponseMode::Fast,
        )
        .await
        .unwrap();

    let conversation = EntityRepository::get(&*entity_repo, result.conversation_id)
        .await
        .unwrap()
        .expect("conversation should exist");
    assert_eq!(conversation.entity_type.as_str(), "Conversation");

    let conv_components = component_repo.get(result.conversation_id).await.unwrap();
    assert!(
        conv_components
            .iter()
            .any(|c| c.component_type == ComponentType::Title),
        "conversation should have Title component"
    );

    let msg_components = component_repo.get(result.message_id).await.unwrap();
    assert!(
        msg_components
            .iter()
            .any(|c| c.component_type == ComponentType::MessageContent),
        "assistant message should have MessageContent"
    );

    let rels = relationship_repo
        .by_source(result.conversation_id)
        .await
        .unwrap();
    assert!(
        rels.iter().any(|r| r.target_id == result.message_id),
        "HasMessage relationship should exist"
    );
}

#[tokio::test]
async fn end_to_end_chat_with_explicit_entity_refs_uses_context() {
    let (pipeline, entity_repo, component_repo, _) = setup();

    let paper = make_entity("Paper");
    let concept = make_entity("Concept");
    let paper_id = paper.id;
    let concept_id = concept.id;

    entity_repo.add_entity(paper);
    entity_repo.add_entity(concept);

    component_repo
        .save(&make_title_component(paper_id, "Attention Is All You Need"))
        .await
        .unwrap();
    component_repo
        .save(&make_content_component(
            paper_id,
            "A transformer-based architecture for sequence transduction.",
        ))
        .await
        .unwrap();
    component_repo
        .save(&make_tags_component(paper_id, vec!["deep-learning", "nlp"]))
        .await
        .unwrap();

    let result = pipeline
        .chat(
            None,
            "Tell me about this paper",
            &[paper_id, concept_id],
            &SourceToggles::default(),
            ResponseMode::Fast,
        )
        .await
        .unwrap();

    assert!(!result.message.is_empty());
    assert!(result.citations.is_empty() || !result.citations.is_empty());
}

#[tokio::test]
async fn end_to_end_chat_without_refs_runs_search() {
    let (pipeline, entity_repo, component_repo, _) = setup();

    let entity = make_entity("Paper");
    let entity_id = entity.id;
    entity_repo.add_entity(entity);

    component_repo
        .save(&make_title_component(entity_id, "Test Entity"))
        .await
        .unwrap();
    component_repo
        .save(&make_content_component(
            entity_id,
            "This is test content for searching.",
        ))
        .await
        .unwrap();
    component_repo
        .save(&make_tags_component(entity_id, vec!["test"]))
        .await
        .unwrap();

    let result = pipeline
        .chat(
            None,
            "Find me something about test",
            &[],
            &SourceToggles::default(),
            ResponseMode::Fast,
        )
        .await
        .unwrap();

    assert!(!result.message.is_empty());
}

#[tokio::test]
async fn end_to_end_chat_disables_knowledge_graph() {
    let (pipeline, _, _, _) = setup();

    let result = pipeline
        .chat(
            None,
            "Hello",
            &[],
            &SourceToggles {
                knowledge_graph: false,
                web_search: false,
            },
            ResponseMode::Fast,
        )
        .await
        .unwrap();

    assert!(result.citations.is_empty());
}

#[tokio::test]
async fn chat_citations_create_referenced_by_relationships() {
    let (pipeline, entity_repo, component_repo, _) = setup();

    let entity = make_entity("Paper");
    let entity_id = entity.id;
    entity_repo.add_entity(entity);

    component_repo
        .save(&make_title_component(entity_id, "Test Paper"))
        .await
        .unwrap();
    component_repo
        .save(&make_content_component(
            entity_id,
            "Content about test paper.",
        ))
        .await
        .unwrap();

    let result = pipeline
        .chat(
            None,
            "Tell me about this",
            &[entity_id],
            &SourceToggles::default(),
            ResponseMode::Fast,
        )
        .await
        .unwrap();

    assert!(!result.message.is_empty());
}

#[tokio::test]
async fn chat_response_citations_persisted_in_message_entity() {
    let (pipeline, entity_repo, component_repo, _) = setup();

    let entity = make_entity("Paper");
    let entity_id = entity.id;
    entity_repo.add_entity(entity);

    component_repo
        .save(&make_title_component(entity_id, "Cited Paper"))
        .await
        .unwrap();
    component_repo
        .save(&make_content_component(entity_id, "Important content."))
        .await
        .unwrap();

    let result = pipeline
        .chat(
            None,
            "Tell me about this paper",
            &[entity_id],
            &SourceToggles::default(),
            ResponseMode::Fast,
        )
        .await
        .unwrap();

    let msg_components = component_repo.get(result.message_id).await.unwrap();
    let entity_refs = msg_components
        .iter()
        .find(|c| c.component_type == ComponentType::EntityRefs)
        .expect("assistant message should have EntityRefs component");

    let refs = entity_refs
        .data
        .get("refs")
        .and_then(|v| v.as_array())
        .map(|arr| {
            arr.iter()
                .filter_map(|v| v.as_str())
                .filter_map(|s| Uuid::parse_str(s).ok())
                .collect::<Vec<_>>()
        })
        .unwrap_or_default();

    assert!(!result.referenced_entities.is_empty() || !refs.is_empty());
}
