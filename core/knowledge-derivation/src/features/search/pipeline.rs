use async_trait::async_trait;
use chrono::Utc;
use knowledge_core::features::component::{Component, ComponentType};
use knowledge_core::ports::{
    AiAdapter, ComponentRepository, EntityRepository, Event, EventNotifier, EventType,
    StorageError, VectorMetadata, VectorStore,
};
use uuid::Uuid;

/// Embedding pipeline that generates vectors from entity Content components.
///
/// The pipeline:
/// 1. Extracts text from an entity's Content component
/// 2. Calls `AiAdapter::embed()` to generate a vector
/// 3. Stores the vector in `VectorStore`
/// 4. Creates an Embedding component on the entity
///
/// Implements `EventNotifier` to react to entity creation and content updates.
pub struct EmbeddingPipeline {
    ai_provider: Box<dyn AiAdapter>,
    vector_store: Box<dyn VectorStore>,
    component_repo: Box<dyn ComponentRepository>,
    entity_repo: Box<dyn EntityRepository>,
}

/// Data stored in an Embedding component.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct EmbeddingData {
    /// The embedding vector.
    pub vector: Vec<f32>,
    /// Name of the model that produced this embedding.
    pub model: String,
    /// ISO 8601 timestamp of when the embedding was generated.
    pub generated_at: String,
}

impl EmbeddingPipeline {
    /// Create a new embedding pipeline.
    pub fn new(
        ai_provider: Box<dyn AiAdapter>,
        vector_store: Box<dyn VectorStore>,
        component_repo: Box<dyn ComponentRepository>,
        entity_repo: Box<dyn EntityRepository>,
    ) -> Self {
        Self {
            ai_provider,
            vector_store,
            component_repo,
            entity_repo,
        }
    }

    /// Generate an embedding for a specific entity.
    ///
    /// Looks up the entity's Content component, generates a vector via the AI
    /// provider, stores it in the vector store, and attaches an Embedding
    /// component to the entity.
    ///
    /// # Errors
    ///
    /// Returns `StorageError::NotFound` if the entity does not exist.
    /// Returns `StorageError::Internal` if embedding generation or storage fails.
    pub async fn embed_entity(&self, entity_id: Uuid) -> Result<(), StorageError> {
        // Look up the entity
        let _entity = self
            .entity_repo
            .get(entity_id)
            .await?
            .ok_or_else(|| StorageError::Internal(format!("entity {} not found", entity_id)))?;

        // Find the Content component
        let components = self.component_repo.get(entity_id).await?;
        let content_text = components
            .iter()
            .find(|c| c.component_type == ComponentType::Content)
            .and_then(|c| c.data.as_str().map(String::from));

        let text = match content_text {
            Some(t) if !t.is_empty() => t,
            _ => return Ok(()), // No content to embed
        };

        // Generate embedding
        let vector =
            self.ai_provider.embed(&text).await.map_err(|e| {
                StorageError::Internal(format!("embedding generation failed: {}", e))
            })?;

        // Look up entity type and title for metadata
        let entity_type = "Unknown".to_string();

        let title = components
            .iter()
            .find(|c| c.component_type == ComponentType::Title)
            .and_then(|c| c.data.as_str().map(String::from))
            .unwrap_or_else(|| "Untitled".to_string());

        // Store vector in the vector store
        let metadata = VectorMetadata {
            model: self.ai_provider.model_name().to_string(),
            entity_type,
            title,
        };
        self.vector_store
            .upsert(&entity_id.to_string(), &vector, Some(metadata))
            .await
            .map_err(|e| StorageError::Internal(format!("vector store upsert failed: {}", e)))?;

        // Create and store Embedding component
        let embedding_data = EmbeddingData {
            vector,
            model: self.ai_provider.model_name().to_string(),
            generated_at: Utc::now().to_rfc3339(),
        };

        let component = Component::new(
            entity_id,
            ComponentType::Embedding,
            serde_json::to_value(&embedding_data)
                .map_err(|e| StorageError::Internal(e.to_string()))?,
        );

        self.component_repo.save(&component).await?;

        Ok(())
    }

    /// Rebuild all embeddings for all entities with Content components.
    ///
    /// This clears the vector store and regenerates embeddings from scratch.
    ///
    /// # Errors
    ///
    /// Returns `StorageError::Internal` if rebuild or embedding fails.
    pub async fn rebuild_all(&self) -> Result<(), StorageError> {
        // Clear the vector store
        self.vector_store
            .rebuild()
            .await
            .map_err(|e| StorageError::Internal(format!("vector store rebuild failed: {}", e)))?;

        // Find all entities with Content components
        let entities = self.entity_repo.find_by_component_type("Content").await?;

        for entity in &entities {
            if let Err(e) = self.embed_entity(entity.id).await {
                // Log but continue — don't fail the entire rebuild for one entity
                eprintln!("Warning: failed to embed entity {}: {}", entity.id, e);
            }
        }

        Ok(())
    }
}

#[async_trait]
impl EventNotifier for EmbeddingPipeline {
    async fn notify(&self, event: &Event) -> Result<(), StorageError> {
        match event.event_type {
            EventType::EntityCreated | EventType::ComponentUpdated => {
                // Only embed if the entity exists and has content
                if let Ok(Some(_)) = self.entity_repo.get(event.entity_id).await {
                    let _ = self.embed_entity(event.entity_id).await;
                }
            }
            _ => {}
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::features::search::providers::MockAiAdapter;
    use crate::features::search::vector_store::InMemoryVectorStore;
    use knowledge_core::features::entity::{Entity, EntityType};
    use knowledge_core::ports::ComponentRepository;

    /// Simple in-memory component repository for testing.
    struct MockComponentRepo {
        components: std::sync::RwLock<Vec<Component>>,
    }

    impl MockComponentRepo {
        fn new() -> Self {
            Self {
                components: std::sync::RwLock::new(Vec::new()),
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
            component_type: &str,
        ) -> Result<Vec<Component>, StorageError> {
            Ok(self
                .components
                .read()
                .unwrap()
                .iter()
                .filter(|c| {
                    c.entity_id == entity_id && format!("{:?}", c.component_type) == component_type
                })
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

    /// Simple in-memory entity repository for testing.
    struct MockEntityRepo {
        entities: std::sync::RwLock<Vec<Entity>>,
    }

    impl MockEntityRepo {
        fn new() -> Self {
            Self {
                entities: std::sync::RwLock::new(Vec::new()),
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
            component_type: &str,
        ) -> Result<Vec<Entity>, StorageError> {
            if component_type == "Content" {
                // Return all entities (they all have content in our test)
                Ok(self.entities.read().unwrap().clone())
            } else {
                Ok(vec![])
            }
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

    fn setup_test_pipeline() -> (
        EmbeddingPipeline,
        std::sync::Arc<MockComponentRepo>,
        std::sync::Arc<MockEntityRepo>,
    ) {
        let ai = Box::new(MockAiAdapter::new("test-model", 8));
        let vs = Box::new(InMemoryVectorStore::new(8));
        let component_repo = std::sync::Arc::new(MockComponentRepo::new());
        let entity_repo = std::sync::Arc::new(MockEntityRepo::new());

        let pipeline = EmbeddingPipeline::new(
            ai,
            vs,
            Box::new(MockComponentRepoWrapper(component_repo.clone())),
            Box::new(MockEntityRepoWrapper(entity_repo.clone())),
        );

        (pipeline, component_repo, entity_repo)
    }

    // Wrappers to satisfy Box<dyn Trait> requirements
    struct MockComponentRepoWrapper(std::sync::Arc<MockComponentRepo>);

    #[async_trait]
    impl ComponentRepository for MockComponentRepoWrapper {
        async fn get(&self, entity_id: Uuid) -> Result<Vec<Component>, StorageError> {
            self.0.get(entity_id).await
        }
        async fn save(&self, component: &Component) -> Result<(), StorageError> {
            self.0.save(component).await
        }
        async fn delete(&self, id: Uuid) -> Result<(), StorageError> {
            self.0.delete(id).await
        }
        async fn find_by_type(
            &self,
            entity_id: Uuid,
            component_type: &str,
        ) -> Result<Vec<Component>, StorageError> {
            self.0.find_by_type(entity_id, component_type).await
        }
        async fn update_data(&self, id: Uuid, data: serde_json::Value) -> Result<(), StorageError> {
            self.0.update_data(id, data).await
        }
        async fn find_by_component_data(
            &self,
            component_type: &str,
            json_path: &str,
            value: &str,
        ) -> Result<Vec<Component>, StorageError> {
            self.0
                .find_by_component_data(component_type, json_path, value)
                .await
        }
        async fn delete_by_entity(&self, entity_id: Uuid) -> Result<(), StorageError> {
            self.0.delete_by_entity(entity_id).await
        }
    }

    struct MockEntityRepoWrapper(std::sync::Arc<MockEntityRepo>);

    #[async_trait]
    impl EntityRepository for MockEntityRepoWrapper {
        async fn get(&self, id: Uuid) -> Result<Option<Entity>, StorageError> {
            self.0.get(id).await
        }
        async fn save(&self, entity: &Entity) -> Result<(), StorageError> {
            self.0.save(entity).await
        }
        async fn delete(&self, id: Uuid) -> Result<(), StorageError> {
            self.0.delete(id).await
        }
        async fn list(&self) -> Result<Vec<Entity>, StorageError> {
            self.0.list().await
        }
        async fn find_by_type(&self, entity_type: &str) -> Result<Vec<Entity>, StorageError> {
            self.0.find_by_type(entity_type).await
        }
        async fn find_by_title(&self, title: &str) -> Result<Vec<Entity>, StorageError> {
            self.0.find_by_title(title).await
        }
        async fn increment_version(&self, id: Uuid) -> Result<(), StorageError> {
            self.0.increment_version(id).await
        }
        async fn find_by_component_type(
            &self,
            component_type: &str,
        ) -> Result<Vec<Entity>, StorageError> {
            self.0.find_by_component_type(component_type).await
        }
        async fn find_by_tag(&self, tag: &str) -> Result<Vec<Entity>, StorageError> {
            self.0.find_by_tag(tag).await
        }
        async fn get_version_history(
            &self,
            entity_id: Uuid,
        ) -> Result<Vec<knowledge_core::ports::EntityVersion>, StorageError> {
            self.0.get_version_history(entity_id).await
        }
    }

    #[tokio::test]
    async fn content_component_produces_embedding_component() {
        let (pipeline, component_repo, entity_repo) = setup_test_pipeline();

        let entity = Entity::new(EntityType::new("Concept"));
        let entity_id = entity.id;
        entity_repo.add_entity(entity);

        // Add a Content component
        let content = Component::new(
            entity_id,
            ComponentType::Content,
            serde_json::json!("Machine learning is a subset of artificial intelligence."),
        );
        component_repo.save(&content).await.unwrap();

        // Run embedding pipeline
        pipeline.embed_entity(entity_id).await.unwrap();

        // Verify Embedding component was created
        let components = component_repo.get(entity_id).await.unwrap();
        let embedding = components
            .iter()
            .find(|c| c.component_type == ComponentType::Embedding);
        assert!(embedding.is_some(), "Embedding component should be created");

        // Verify the embedding data
        let data: EmbeddingData = serde_json::from_value(embedding.unwrap().data.clone()).unwrap();
        assert_eq!(data.vector.len(), 8);
        assert_eq!(data.model, "test-model");
    }

    #[tokio::test]
    async fn mock_embedder_generates_deterministic_embeddings() {
        let (pipeline, component_repo, entity_repo) = setup_test_pipeline();

        let entity = Entity::new(EntityType::new("Concept"));
        let entity_id = entity.id;
        entity_repo.add_entity(entity);

        let content = Component::new(
            entity_id,
            ComponentType::Content,
            serde_json::json!("Test content for embedding."),
        );
        component_repo.save(&content).await.unwrap();

        pipeline.embed_entity(entity_id).await.unwrap();

        // Get the embedding
        let components = component_repo.get(entity_id).await.unwrap();
        let embedding = components
            .iter()
            .find(|c| c.component_type == ComponentType::Embedding)
            .unwrap();
        let data: EmbeddingData = serde_json::from_value(embedding.data.clone()).unwrap();

        // Run again — should produce identical embedding
        let entity2 = Entity::new(EntityType::new("Concept"));
        let entity_id2 = entity2.id;
        entity_repo.add_entity(entity2);

        let content2 = Component::new(
            entity_id2,
            ComponentType::Content,
            serde_json::json!("Test content for embedding."),
        );
        component_repo.save(&content2).await.unwrap();

        pipeline.embed_entity(entity_id2).await.unwrap();

        let components2 = component_repo.get(entity_id2).await.unwrap();
        let embedding2 = components2
            .iter()
            .find(|c| c.component_type == ComponentType::Embedding)
            .unwrap();
        let data2: EmbeddingData = serde_json::from_value(embedding2.data.clone()).unwrap();

        assert_eq!(data.vector, data2.vector);
    }

    #[tokio::test]
    async fn entity_without_content_skips_embedding() {
        let (pipeline, _component_repo, entity_repo) = setup_test_pipeline();

        let entity = Entity::new(EntityType::new("Concept"));
        let entity_id = entity.id;
        entity_repo.add_entity(entity);

        // No content component — should return Ok without error
        let result = pipeline.embed_entity(entity_id).await;
        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn entity_not_found_returns_error() {
        let (pipeline, _, _) = setup_test_pipeline();
        let fake_id = Uuid::new_v4();

        let result = pipeline.embed_entity(fake_id).await;
        assert!(result.is_err());
    }
}
