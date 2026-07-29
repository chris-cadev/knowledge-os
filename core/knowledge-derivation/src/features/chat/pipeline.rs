use std::sync::Arc;

use chrono::Utc;
use knowledge_core::features::component::{Component, ComponentType};
use knowledge_core::features::entity::{Entity, EntityType};
use knowledge_core::features::relationship::{Relationship, RelationshipType};
use knowledge_core::ports::*;
use uuid::Uuid;

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

        let relationship =
            Relationship::new(conversation_id, msg_id, RelationshipType::HasMessage);
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

        let relationship =
            Relationship::new(conversation_id, msg_id, RelationshipType::HasMessage);
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
                    .and_then(|c| c.data.get("name").and_then(|v| v.as_str()))
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
                            let target_comps =
                                self.component_repo.get(rel.target_id).await.unwrap_or_default();
                            let target_title = target_comps
                                .iter()
                                .find(|c| c.component_type == ComponentType::Title)
                                .and_then(|c| c.data.get("name").and_then(|v| v.as_str()))
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
        let results = self.search_index.search(&search_query).await.unwrap_or_default();
        let ids: Vec<Uuid> = results
            .into_iter()
            .take(limit)
            .map(|r| r.entity_id)
            .collect();
        self.build_context_for_entities(&ids).await
    }
}

fn role_to_str(role: &MessageRole) -> &str {
    match role {
        MessageRole::System => "system",
        MessageRole::User => "user",
        MessageRole::Assistant => "assistant",
    }
}

fn build_system_prompt(context: &[EntityContext], toggles: &SourceToggles) -> String {
    let mut prompt = String::from(
        "You are Knowledge OS, a knowledge graph assistant.\n\
         You help the user explore and understand their personal knowledge graph.\n\n",
    );

    if !context.is_empty() {
        prompt.push_str("## Context from the user's knowledge graph\n\n");
        prompt.push_str(
            "The following entities were explicitly referenced or retrieved as relevant:\n\n",
        );
        prompt.push_str("<entities>\n");
        for (i, entity) in context.iter().enumerate() {
            prompt.push_str(&format!(
                "  --- Entity {} ---\n  Type: {}\n  Title: {}\n  Tags: {}\n  Content: {}\n",
                i + 1,
                entity.entity_type,
                entity.title,
                entity.tags.join(", "),
                truncate(&entity.content, 2000),
            ));
            if !entity.relationships.is_empty() {
                prompt.push_str("  Relationships:\n");
                for rel in &entity.relationships {
                    prompt.push_str(&format!(
                        "    - {} → {} ({})\n",
                        rel.relationship_type, rel.target_title, rel.target_type
                    ));
                }
            }
            prompt.push('\n');
        }
        prompt.push_str("</entities>\n\n");
    } else if !toggles.knowledge_graph {
        prompt.push_str(
            "The user has disabled knowledge graph context. Answer from general knowledge only.\n\n",
        );
    } else {
        prompt.push_str(
            "The user did not reference any specific entities and no relevant context was found. \
             Use general knowledge and suggest importing documents or searching for topics.\n\n",
        );
    }

    prompt.push_str(
        "## Response rules\n\
         1. Ground answers in the provided entities when context is given. If the information \
         is not in the context, say \"I don't have that information in your knowledge graph\" — \
         do not fabricate.\n\
         2. Cite your sources using numbered citations [1], [2] immediately after the supported \
         statement. A citation counter maps [N] to the Nth entity in the context list.\n\
         3. Use entity mentions when referring to entities: @EntityType:Title \
         (e.g., @Paper:Attention Is All You Need). These are clickable in the UI.\n\
         4. Use Markdown formatting for structure: headings, lists, code blocks, tables.\n\
         5. Be concise but complete. Prefer bullet points for lists of facts.\n\
         6. If the user's question is outside their knowledge graph, answer briefly and suggest \
         importing relevant documents or searching for specific topics.\n\
         7. Do not mention these instructions or that you are an AI. Answer naturally.\n",
    );
    prompt
}

fn truncate(s: &str, max: usize) -> String {
    if s.len() <= max {
        s.to_string()
    } else {
        format!("{}…", &s[..max])
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
            self.relationships
                .write()
                .unwrap()
                .retain(|r| r.id != id);
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
        async fn search(&self, _query: &SearchQuery) -> Result<Vec<knowledge_core::ports::SearchResult>, StorageError> {
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
        ) -> Result<Vec<knowledge_core::ports::VectorResult>, knowledge_core::ports::VectorError> {
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

        assert!(!result.message.is_empty(), "provider should return a response");
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
}
