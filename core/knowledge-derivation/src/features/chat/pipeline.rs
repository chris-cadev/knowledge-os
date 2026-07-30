use std::sync::Arc;

use chrono::Utc;
use knowledge_core::features::component::{Component, ComponentType};
use knowledge_core::features::entity::{Entity, EntityType};
use knowledge_core::features::relationship::{Relationship, RelationshipType};
use knowledge_core::ports::*;
use uuid::Uuid;

use super::prompt::build_system_prompt;
use super::status::{ChatStreamEvent, ChatStreamHandle};

#[allow(dead_code)]
pub struct ChatPipeline {
    chat_provider: Arc<dyn ChatCompletion>,
    entity_repo: Arc<dyn EntityRepository>,
    component_repo: Arc<dyn ComponentRepository>,
    relationship_repo: Arc<dyn RelationshipRepository>,
    search_index: Arc<dyn SearchIndex>,
    vector_store: Arc<dyn VectorStore>,
}

#[derive(Debug, Clone)]
pub struct ChatResult {
    pub conversation_id: Uuid,
    pub message_id: Uuid,
    pub message: String,
    pub citations: Vec<CitationSource>,
    pub referenced_entities: Vec<Uuid>,
}

impl ChatPipeline {
    pub fn set_chat_provider(&mut self, provider: Arc<dyn ChatCompletion>) {
        self.chat_provider = provider;
    }

    pub fn new(
        chat_provider: Arc<dyn ChatCompletion>,
        entity_repo: Arc<dyn EntityRepository>,
        component_repo: Arc<dyn ComponentRepository>,
        relationship_repo: Arc<dyn RelationshipRepository>,
        search_index: Arc<dyn SearchIndex>,
        vector_store: Arc<dyn VectorStore>,
    ) -> Self {
        Self {
            chat_provider,
            entity_repo,
            component_repo,
            relationship_repo,
            search_index,
            vector_store,
        }
    }

    pub async fn chat(
        &self,
        conversation_id: Option<Uuid>,
        user_message: &str,
        entity_refs: &[Uuid],
        source_toggles: &SourceToggles,
        mode: ResponseMode,
    ) -> Result<ChatResult, ChatError> {
        let conv_id = match conversation_id {
            Some(id) => id,
            None => self
                .create_conversation()
                .await
                .map_err(|e| ChatError::Provider(e.to_string()))?,
        };

        let _user_msg_id = self
            .persist_message(conv_id, MessageRole::User, user_message, entity_refs)
            .await
            .map_err(|e| ChatError::Provider(e.to_string()))?;

        let context_entities = if source_toggles.knowledge_graph {
            if !entity_refs.is_empty() {
                self.build_context_for_entities(entity_refs).await
            } else {
                self.search_context(user_message, 10).await
            }
        } else {
            vec![]
        };

        let system_prompt = build_system_prompt(&context_entities, source_toggles);

        let request = ChatRequest {
            system_prompt,
            messages: vec![Message {
                role: MessageRole::User,
                content: user_message.to_string(),
                entity_refs: entity_refs.to_vec(),
            }],
            context_entities: context_entities.clone(),
            mode: mode.clone(),
            source_toggles: source_toggles.clone(),
            config: ChatConfig::default(),
        };

        let response = self.chat_provider.chat(request).await?;

        let assistant_msg_id = self
            .persist_message_with_citations(conv_id, &response.message, &response.citations)
            .await
            .map_err(|e| ChatError::Provider(e.to_string()))?;

        Ok(ChatResult {
            conversation_id: conv_id,
            message_id: assistant_msg_id,
            message: response.message,
            citations: response.citations,
            referenced_entities: response.referenced_entities,
        })
    }

    pub async fn chat_stream(
        &self,
        conversation_id: Option<Uuid>,
        user_message: &str,
        entity_refs: &[Uuid],
        source_toggles: &SourceToggles,
        mode: ResponseMode,
    ) -> Result<ChatStreamHandle, ChatError> {
        let conv_id = match conversation_id {
            Some(id) => id,
            None => self
                .create_conversation()
                .await
                .map_err(|e| ChatError::Provider(e.to_string()))?,
        };

        let user_msg_id = self
            .persist_message(conv_id, MessageRole::User, user_message, entity_refs)
            .await
            .map_err(|e| ChatError::Provider(e.to_string()))?;

        let context_entities = if source_toggles.knowledge_graph {
            if !entity_refs.is_empty() {
                self.build_context_for_entities(entity_refs).await
            } else {
                self.search_context(user_message, 10).await
            }
        } else {
            vec![]
        };

        let system_prompt = build_system_prompt(&context_entities, source_toggles);

        let request = ChatRequest {
            system_prompt,
            messages: vec![Message {
                role: MessageRole::User,
                content: user_message.to_string(),
                entity_refs: entity_refs.to_vec(),
            }],
            context_entities: context_entities.clone(),
            mode,
            source_toggles: source_toggles.clone(),
            config: ChatConfig::default(),
        };

        let (cancel_tx, cancel_rx) = tokio::sync::watch::channel(false);
        let (event_tx, event_rx) = tokio::sync::mpsc::unbounded_channel::<ChatStreamEvent>();

        let provider = self.chat_provider.clone();
        let entity_repo = self.entity_repo.clone();
        let component_repo = self.component_repo.clone();
        let relationship_repo = self.relationship_repo.clone();

        tokio::spawn(async move {
            let mut cancel_rx = cancel_rx;

            if *cancel_rx.borrow_and_update() {
                return;
            }

            let _ = event_tx.send(ChatStreamEvent::Status(ProcessingStatus::Searching {
                detail: "Searching knowledge graph...".into(),
            }));

            if *cancel_rx.borrow_and_update() {
                return;
            }

            let count = context_entities.len() as u32;
            let _ = event_tx.send(ChatStreamEvent::Status(ProcessingStatus::ReadingEntities {
                count,
            }));

            if *cancel_rx.borrow_and_update() {
                return;
            }

            let _ = event_tx.send(ChatStreamEvent::Status(ProcessingStatus::Generating));

            if *cancel_rx.borrow_and_update() {
                return;
            }

            match provider.chat_stream(request).await {
                Ok(mut provider_stream) => {
                    use futures::StreamExt;
                    let mut message_buffer = String::new();
                    let mut citations = vec![];

                    while let Some(delta) = provider_stream.next().await {
                        if *cancel_rx.borrow_and_update() {
                            return;
                        }
                        message_buffer.push_str(&delta.delta);
                        let _ = event_tx.send(ChatStreamEvent::Delta(delta));
                    }

                    citations =
                        super::citations::extract_citations(&message_buffer, &context_entities);

                    let assistant_id = persist_assistant_message(
                        &entity_repo,
                        &component_repo,
                        &relationship_repo,
                        conv_id,
                        &message_buffer,
                        &citations,
                    )
                    .await
                    .unwrap_or_else(|_| uuid::Uuid::new_v4());

                    let _ = event_tx.send(ChatStreamEvent::Done {
                        assistant_message_id: assistant_id,
                        citations,
                    });
                }
                Err(e) => {
                    let _ = event_tx.send(ChatStreamEvent::Error(e));
                }
            }
        });

        let stream = tokio_stream::wrappers::UnboundedReceiverStream::new(event_rx);

        Ok(ChatStreamHandle {
            conversation_id: conv_id,
            user_message_id: user_msg_id,
            stream: Box::new(stream),
            cancel: cancel_tx,
        })
    }

    async fn create_conversation(&self) -> Result<Uuid, StorageError> {
        let entity = Entity::new(EntityType::new("Conversation"));
        let conv_id = entity.id;

        let title_component = Component::new(
            conv_id,
            ComponentType::Title,
            serde_json::json!({"name": "New Conversation"}),
        );

        let provenance = Component::new(
            conv_id,
            ComponentType::Provenance,
            serde_json::json!({
                "created_at": Utc::now().to_rfc3339(),
                "source": "chat_pipeline"
            }),
        );

        self.entity_repo.save(&entity).await?;
        self.component_repo.save(&title_component).await?;
        self.component_repo.save(&provenance).await?;

        Ok(conv_id)
    }

    async fn persist_message(
        &self,
        conversation_id: Uuid,
        role: MessageRole,
        content: &str,
        entity_refs: &[Uuid],
    ) -> Result<Uuid, StorageError> {
        let entity = Entity::new(EntityType::new("Message"));
        let msg_id = entity.id;

        let content_component = Component::new(
            msg_id,
            ComponentType::MessageContent,
            serde_json::json!({
                "role": role_to_str(&role),
                "content": content,
            }),
        );

        let refs_component = Component::new(
            msg_id,
            ComponentType::EntityRefs,
            serde_json::json!({
                "refs": entity_refs.iter().map(|r| r.to_string()).collect::<Vec<_>>(),
            }),
        );

        self.entity_repo.save(&entity).await?;
        self.component_repo.save(&content_component).await?;
        self.component_repo.save(&refs_component).await?;

        let relationship = Relationship::new(conversation_id, msg_id, RelationshipType::HasMessage);
        self.relationship_repo.save(&relationship).await?;

        Ok(msg_id)
    }

    async fn persist_message_with_citations(
        &self,
        conversation_id: Uuid,
        content: &str,
        citations: &[CitationSource],
    ) -> Result<Uuid, StorageError> {
        let entity = Entity::new(EntityType::new("Message"));
        let msg_id = entity.id;

        let content_component = Component::new(
            msg_id,
            ComponentType::MessageContent,
            serde_json::json!({
                "role": "assistant",
                "content": content,
            }),
        );

        let cited_ids: Vec<String> = citations.iter().map(|c| c.entity_id.to_string()).collect();
        let refs_component = Component::new(
            msg_id,
            ComponentType::EntityRefs,
            serde_json::json!({
                "refs": cited_ids,
            }),
        );

        self.entity_repo.save(&entity).await?;
        self.component_repo.save(&content_component).await?;
        self.component_repo.save(&refs_component).await?;

        let relationship = Relationship::new(conversation_id, msg_id, RelationshipType::HasMessage);
        self.relationship_repo.save(&relationship).await?;

        Ok(msg_id)
    }

    async fn build_context_for_entities(&self, entity_refs: &[Uuid]) -> Vec<EntityContext> {
        let mut contexts = Vec::new();
        for id in entity_refs {
            if let Ok(Some(entity)) = self.entity_repo.get(*id).await {
                let components = self.component_repo.get(*id).await.unwrap_or_default();
                let title = components
                    .iter()
                    .find(|c| c.component_type == ComponentType::Title)
                    .and_then(|c| c.data.as_str())
                    .unwrap_or("Untitled")
                    .to_string();
                let content = components
                    .iter()
                    .find(|c| c.component_type == ComponentType::Content)
                    .and_then(|c| c.data.as_str())
                    .unwrap_or("")
                    .to_string();
                let tags = components
                    .iter()
                    .find(|c| c.component_type == ComponentType::Tags)
                    .and_then(|c| c.data.get("values").and_then(|v| v.as_array()))
                    .map(|arr| {
                        arr.iter()
                            .filter_map(|v| v.as_str().map(String::from))
                            .collect()
                    })
                    .unwrap_or_default();

                let mut relationships = Vec::new();
                if let Ok(outgoing) = self.relationship_repo.by_source(*id).await {
                    for rel in outgoing {
                        if let Ok(Some(target)) = self.entity_repo.get(rel.target_id).await {
                            let target_comps = self
                                .component_repo
                                .get(rel.target_id)
                                .await
                                .unwrap_or_default();
                            let target_title = target_comps
                                .iter()
                                .find(|c| c.component_type == ComponentType::Title)
                                .and_then(|c| c.data.as_str())
                                .unwrap_or("Untitled")
                                .to_string();
                            relationships.push(RelationshipSummary {
                                relationship_type: format!("{:?}", rel.relationship_type),
                                target_id: rel.target_id,
                                target_title,
                                target_type: target.entity_type.to_string(),
                            });
                        }
                    }
                }

                contexts.push(EntityContext {
                    entity_id: entity.id,
                    entity_type: entity.entity_type.to_string(),
                    title,
                    content,
                    tags,
                    relationships,
                });
            }
        }
        contexts
    }

    async fn search_context(&self, query: &str, limit: usize) -> Vec<EntityContext> {
        let search_query = SearchQuery {
            query: query.to_string(),
            entity_type: None,
            tag: None,
        };
        let results = self
            .search_index
            .search(&search_query)
            .await
            .unwrap_or_default();
        let ids: Vec<Uuid> = results
            .into_iter()
            .take(limit)
            .map(|r| r.entity_id)
            .collect();
        self.build_context_for_entities(&ids).await
    }
}

async fn persist_assistant_message(
    entity_repo: &Arc<dyn EntityRepository>,
    component_repo: &Arc<dyn ComponentRepository>,
    relationship_repo: &Arc<dyn RelationshipRepository>,
    conversation_id: Uuid,
    content: &str,
    citations: &[CitationSource],
) -> Result<Uuid, StorageError> {
    let entity = Entity::new(EntityType::new("Message"));
    let msg_id = entity.id;

    let content_component = Component::new(
        msg_id,
        ComponentType::MessageContent,
        serde_json::json!({
            "role": "assistant",
            "content": content,
        }),
    );

    let cited_ids: Vec<String> = citations.iter().map(|c| c.entity_id.to_string()).collect();
    let refs_component = Component::new(
        msg_id,
        ComponentType::EntityRefs,
        serde_json::json!({
            "refs": cited_ids,
        }),
    );

    entity_repo.save(&entity).await?;
    component_repo.save(&content_component).await?;
    component_repo.save(&refs_component).await?;

    let relationship = Relationship::new(conversation_id, msg_id, RelationshipType::HasMessage);
    relationship_repo.save(&relationship).await?;

    Ok(msg_id)
}

fn role_to_str(role: &MessageRole) -> &str {
    match role {
        MessageRole::System => "system",
        MessageRole::User => "user",
        MessageRole::Assistant => "assistant",
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::features::chat::mock::MockChatAdapter;
    use async_trait::async_trait;
    use std::sync::RwLock;

    struct MockEntityRepo {
        entities: RwLock<Vec<Entity>>,
    }

    impl MockEntityRepo {
        fn new() -> Self {
            Self {
                entities: RwLock::new(Vec::new()),
            }
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
        async fn find_by_type(&self, _entity_type: &str) -> Result<Vec<Entity>, StorageError> {
            Ok(vec![])
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
        ) -> Result<Vec<knowledge_core::ports::EntityVersion>, StorageError> {
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
            entity_id: Uuid,
            _component_type: &str,
        ) -> Result<Vec<Component>, StorageError> {
            Ok(self
                .components
                .read()
                .unwrap()
                .iter()
                .filter(|c| c.entity_id == entity_id)
                .cloned()
                .collect())
        }
        async fn update_data(&self, id: Uuid, data: serde_json::Value) -> Result<(), StorageError> {
            if let Some(comp) = self
                .components
                .write()
                .unwrap()
                .iter_mut()
                .find(|c| c.id == id)
            {
                comp.data = data;
            }
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
        async fn delete_by_entity(&self, entity_id: Uuid) -> Result<(), StorageError> {
            self.components
                .write()
                .unwrap()
                .retain(|c| c.entity_id != entity_id);
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
        async fn search(
            &self,
            _query: &SearchQuery,
        ) -> Result<Vec<knowledge_core::ports::SearchResult>, StorageError> {
            Ok(vec![])
        }
        async fn rebuild(
            &self,
            _entities: &[(Entity, Vec<Component>)],
        ) -> Result<(), StorageError> {
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
            _metadata: Option<knowledge_core::ports::VectorMetadata>,
        ) -> Result<(), knowledge_core::ports::VectorError> {
            Ok(())
        }
        async fn search(
            &self,
            _query: &[f32],
            _k: usize,
            _filter: Option<knowledge_core::ports::VectorFilter>,
        ) -> Result<Vec<knowledge_core::ports::VectorResult>, knowledge_core::ports::VectorError>
        {
            Ok(vec![])
        }
        async fn delete(&self, _entity_id: &str) -> Result<(), knowledge_core::ports::VectorError> {
            Ok(())
        }
        async fn rebuild(&self) -> Result<(), knowledge_core::ports::VectorError> {
            Ok(())
        }
    }

    fn setup_pipeline() -> (
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
    async fn pipeline_creates_conversation_on_none() {
        let (pipeline, entity_repo, _, _) = setup_pipeline();
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

        let entities = entity_repo.list().await.unwrap();
        assert!(
            entities.iter().any(|e| e.id == result.conversation_id),
            "conversation entity should be created"
        );
    }

    #[tokio::test]
    async fn pipeline_persists_user_message() {
        let (pipeline, _, component_repo, _) = setup_pipeline();
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

        let conv_components = component_repo.get(result.conversation_id).await.unwrap();
        let message_components = component_repo.get(result.message_id).await.unwrap();

        assert!(
            conv_components
                .iter()
                .any(|c| c.component_type == ComponentType::Title),
            "conversation should have Title component"
        );
        assert!(
            message_components
                .iter()
                .any(|c| c.component_type == ComponentType::MessageContent),
            "assistant message should have MessageContent component"
        );
    }

    #[tokio::test]
    async fn pipeline_calls_provider() {
        let (pipeline, _, _, _) = setup_pipeline();
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

        assert!(
            !result.message.is_empty(),
            "provider should return a response"
        );
    }

    #[tokio::test]
    async fn pipeline_persists_assistant_message() {
        let (pipeline, _, component_repo, relationship_repo) = setup_pipeline();
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

        let msg_components = component_repo.get(result.message_id).await.unwrap();
        let msg_content = msg_components
            .iter()
            .find(|c| c.component_type == ComponentType::MessageContent)
            .expect("assistant message should have MessageContent");
        assert_eq!(
            msg_content.data.get("role").and_then(|v| v.as_str()),
            Some("assistant")
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
    async fn pipeline_handles_empty_entity_refs() {
        let (pipeline, _, _, _) = setup_pipeline();
        let result = pipeline
            .chat(
                None,
                "Hello",
                &[],
                &SourceToggles::default(),
                ResponseMode::Fast,
            )
            .await
            .unwrap();

        assert!(!result.message.is_empty());
    }

    #[tokio::test]
    async fn pipeline_handles_disabled_knowledge_graph() {
        let (pipeline, _, component_repo, _) = setup_pipeline();
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

        let components = component_repo.get(result.message_id).await.unwrap();
        let msg_content = components
            .iter()
            .find(|c| c.component_type == ComponentType::MessageContent)
            .expect("should have message content");
        let content_str = msg_content
            .data
            .get("content")
            .and_then(|v| v.as_str())
            .unwrap_or("");
        assert!(
            content_str.contains("don't have any entities"),
            "with no entities, should return no-entities response"
        );
    }

    #[tokio::test]
    async fn stream_emits_status_before_delta() {
        let (pipeline, _, _, _) = setup_pipeline();
        let handle = pipeline
            .chat_stream(
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

        use futures::StreamExt;
        let mut stream = handle.stream;
        let first = stream.next().await;
        match first {
            Some(ChatStreamEvent::Status(_)) => {}
            other => panic!(
                "expected Status as first event, got {:?}",
                std::mem::discriminant(&other.unwrap())
            ),
        }
    }

    #[tokio::test]
    async fn stream_emits_generating_status_before_first_delta() {
        let (pipeline, _, _, _) = setup_pipeline();
        let handle = pipeline
            .chat_stream(
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

        use futures::StreamExt;
        let mut stream = handle.stream;
        let mut found_generating = false;
        let mut found_delta = false;
        while let Some(event) = stream.next().await {
            match event {
                ChatStreamEvent::Status(ProcessingStatus::Generating) => {
                    found_generating = true;
                }
                ChatStreamEvent::Delta(_) => {
                    found_delta = true;
                    break;
                }
                _ => {}
            }
        }
        assert!(
            found_generating,
            "should emit Generating before first Delta"
        );
        assert!(found_delta, "should eventually emit a Delta");
    }

    #[tokio::test]
    async fn stream_finished_flag_on_last_delta() {
        let (pipeline, _, _, _) = setup_pipeline();
        let handle = pipeline
            .chat_stream(
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

        use futures::StreamExt;
        let mut stream = handle.stream;
        let mut last_delta_finished = false;
        while let Some(event) = stream.next().await {
            if let ChatStreamEvent::Delta(d) = event {
                last_delta_finished = d.finished;
            }
        }
        assert!(last_delta_finished, "last Delta should have finished: true");
    }

    #[tokio::test]
    async fn stream_done_includes_message_id_and_citations() {
        let (pipeline, _, _, _) = setup_pipeline();
        let handle = pipeline
            .chat_stream(
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

        use futures::StreamExt;
        let mut stream = handle.stream;
        let mut done_received = false;
        while let Some(event) = stream.next().await {
            if let ChatStreamEvent::Done {
                assistant_message_id,
                citations: _,
            } = event
            {
                done_received = true;
                assert_ne!(assistant_message_id, uuid::Uuid::nil());
            }
        }
        assert!(done_received, "should emit Done event");
    }

    #[test]
    fn chat_request_serializes() {
        let req = ChatRequest {
            system_prompt: "You are a helpful assistant.".into(),
            messages: vec![Message {
                role: MessageRole::User,
                content: "Hello".into(),
                entity_refs: vec![],
            }],
            context_entities: vec![],
            mode: ResponseMode::Fast,
            source_toggles: SourceToggles::default(),
            config: ChatConfig::default(),
        };
        let json = serde_json::to_string(&req).unwrap();
        let parsed: ChatRequest = serde_json::from_str(&json).unwrap();
        assert_eq!(parsed.system_prompt, req.system_prompt);
        assert_eq!(parsed.messages.len(), 1);
    }

    #[test]
    fn chat_response_serializes() {
        let resp = ChatResponse {
            message: "Test response".into(),
            citations: vec![],
            referenced_entities: vec![],
        };
        let json = serde_json::to_string(&resp).unwrap();
        let parsed: ChatResponse = serde_json::from_str(&json).unwrap();
        assert_eq!(parsed.message, "Test response");
    }

    #[test]
    fn chat_delta_serializes() {
        let delta = ChatDelta {
            delta: "Hello".into(),
            citation: Some(1),
            status: Some(ProcessingStatus::Generating),
            finished: false,
        };
        let json = serde_json::to_string(&delta).unwrap();
        let parsed: ChatDelta = serde_json::from_str(&json).unwrap();
        assert_eq!(parsed.delta, "Hello");
        assert_eq!(parsed.citation, Some(1));
    }

    #[tokio::test]
    async fn cancel_stops_stream() {
        let (pipeline, _, _, _) = setup_pipeline();
        let handle = pipeline
            .chat_stream(
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

        let _ = handle.cancel.send(true);
        use futures::StreamExt;
        let mut stream = handle.stream;
        let mut count = 0;
        while let Some(_event) = stream.next().await {
            count += 1;
            if count > 10 {
                break;
            }
        }
        assert!(count < 10, "stream should stop early after cancel");
    }
}
