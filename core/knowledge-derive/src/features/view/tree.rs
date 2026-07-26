use async_trait::async_trait;
use knowledge_core::features::component::ComponentType;
use knowledge_core::ports::{
    CollectionRepository, ComponentRepository, EntityRepository, Event, StorageError, TreeData,
    TreeNode, ViewAdapter, ViewFilter, ViewOutput,
};

/// View adapter that renders entities as a hierarchical tree grouped by entity type.
///
/// Each entity type becomes a branch. Within each branch, entities are listed
/// with their title. When a `CollectionRepository` is provided, collections
/// appear as top-level branches containing their member entities.
pub struct TreeViewAdapter {
    entity_repo: Box<dyn EntityRepository>,
    component_repo: Box<dyn ComponentRepository>,
    collection_repo: Option<Box<dyn CollectionRepository>>,
}

impl TreeViewAdapter {
    /// Creates a new tree view adapter.
    ///
    /// If `collection_repo` is `None`, collection branches are omitted.
    pub fn new(
        entity_repo: Box<dyn EntityRepository>,
        component_repo: Box<dyn ComponentRepository>,
        collection_repo: Option<Box<dyn CollectionRepository>>,
    ) -> Self {
        Self {
            entity_repo,
            component_repo,
            collection_repo,
        }
    }

    /// Extracts title from entity components, falling back to entity ID.
    async fn get_title(&self, entity_id: uuid::Uuid) -> Result<String, StorageError> {
        let components = self.component_repo.get(entity_id).await?;
        Ok(components
            .iter()
            .find(|c| c.component_type == ComponentType::Title)
            .and_then(|c| c.data.as_str().map(String::from))
            .unwrap_or_else(|| "Untitled".to_string()))
    }
}

#[async_trait]
impl ViewAdapter for TreeViewAdapter {
    fn name(&self) -> &str {
        "tree"
    }

    async fn render(&self, filter: &ViewFilter) -> Result<ViewOutput, StorageError> {
        let mut roots = Vec::new();

        // Add collection branches if collection_repo is available
        if let Some(ref collection_repo) = self.collection_repo {
            let collections = collection_repo.list().await?;
            for collection in collections {
                let members = collection_repo.get_members(collection.id).await?;
                let mut children = Vec::new();
                for entity in members {
                    let label = self.get_title(entity.id).await?;
                    children.push(TreeNode {
                        entity,
                        label,
                        children: Vec::new(),
                    });
                }

                roots.push(TreeNode {
                    entity: knowledge_core::features::entity::Entity {
                        id: collection.id,
                        entity_type: knowledge_core::features::entity::EntityType::new(
                            "Collection",
                        ),
                        is_active: true,
                        created_at: collection.created_at,
                        updated_at: collection.updated_at,
                        version: 1,
                    },
                    label: collection.name.clone(),
                    children,
                });
            }
        }

        // Get entities (filtered or all)
        let entities = match &filter.entity_types {
            Some(types) if types.len() == 1 => {
                self.entity_repo.find_by_type(types[0].as_str()).await?
            }
            _ => self.entity_repo.list().await?,
        };

        // Group by entity type
        let mut grouped: Vec<(String, Vec<_>)> = Vec::new();
        for entity in &entities {
            // Apply entity type filter (multi-type case)
            if let Some(ref types) = filter.entity_types {
                if !types.iter().any(|t| t == &entity.entity_type) {
                    continue;
                }
            }

            // Apply tag filter
            if let Some(ref tags) = filter.tags {
                let components = self.component_repo.get(entity.id).await?;
                let entity_tags: Vec<String> = components
                    .iter()
                    .filter(|c| c.component_type == ComponentType::Tags)
                    .filter_map(|c| c.data.as_array())
                    .flatten()
                    .filter_map(|v| v.as_str().map(String::from))
                    .collect();
                if !tags.iter().any(|t| entity_tags.contains(t)) {
                    continue;
                }
            }

            let type_name = entity.entity_type.to_string();
            if let Some(entry) = grouped.iter_mut().find(|(t, _)| t == &type_name) {
                entry.1.push(entity.clone());
            } else {
                grouped.push((type_name, vec![entity.clone()]));
            }
        }

        // Build tree nodes from grouped entities
        for (_type_name, entities) in grouped {
            let mut children = Vec::new();
            for entity in entities {
                let label = self.get_title(entity.id).await?;
                children.push(TreeNode {
                    entity,
                    label,
                    children: Vec::new(),
                });
            }
            // Use a synthetic entity for the group branch
            if let Some(first) = children.first() {
                let group_entity =
                    knowledge_core::features::entity::Entity::new(first.entity.entity_type.clone());
                roots.push(TreeNode {
                    entity: group_entity,
                    label: first.entity.entity_type.to_string(),
                    children,
                });
            }
        }

        // Apply max_results if specified
        if let Some(max) = filter.max_results {
            roots.truncate(max);
        }

        Ok(ViewOutput::Tree(TreeData { roots }))
    }

    async fn on_event(&self, _event: &Event) -> Result<(), StorageError> {
        // Tree view rebuilds on every render — no cached state to invalidate.
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::Utc;
    use knowledge_core::features::component::{Component, ComponentType};
    use knowledge_core::features::entity::{Entity, EntityType};
    use knowledge_core::ports::{ComponentRepository, EntityRepository, StorageError};
    use std::collections::HashMap;
    use uuid::Uuid;

    // ---------------------------------------------------------------------------
    // In-memory mock repositories for testing
    // ---------------------------------------------------------------------------

    #[derive(Default)]
    struct MockEntityRepo {
        entities: Vec<Entity>,
    }

    #[derive(Default)]
    struct MockComponentRepo {
        components: HashMap<Uuid, Vec<Component>>,
    }

    #[async_trait]
    impl EntityRepository for MockEntityRepo {
        async fn get(&self, id: Uuid) -> Result<Option<Entity>, StorageError> {
            Ok(self.entities.iter().find(|e| e.id == id).cloned())
        }
        async fn save(&self, _entity: &Entity) -> Result<(), StorageError> {
            Ok(())
        }
        async fn delete(&self, _id: Uuid) -> Result<(), StorageError> {
            Ok(())
        }
        async fn list(&self) -> Result<Vec<Entity>, StorageError> {
            Ok(self.entities.clone())
        }
        async fn find_by_type(&self, entity_type: &str) -> Result<Vec<Entity>, StorageError> {
            Ok(self
                .entities
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
        ) -> Result<Vec<knowledge_core::ports::EntityVersion>, StorageError> {
            Ok(vec![])
        }
    }

    #[async_trait]
    impl ComponentRepository for MockComponentRepo {
        async fn get(&self, entity_id: Uuid) -> Result<Vec<Component>, StorageError> {
            Ok(self.components.get(&entity_id).cloned().unwrap_or_default())
        }
        async fn save(&self, _component: &Component) -> Result<(), StorageError> {
            Ok(())
        }
        async fn delete(&self, _id: Uuid) -> Result<(), StorageError> {
            Ok(())
        }
        async fn find_by_type(
            &self,
            _entity_id: Uuid,
            _component_type: &str,
        ) -> Result<Vec<Component>, StorageError> {
            Ok(vec![])
        }
        async fn update_data(
            &self,
            _id: Uuid,
            _data: serde_json::Value,
        ) -> Result<(), StorageError> {
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

    // ---------------------------------------------------------------------------
    // Helper to create entities with titles
    // ---------------------------------------------------------------------------

    fn make_entity(entity_type: &str) -> Entity {
        let mut e = Entity::new(EntityType::new(entity_type));
        e.created_at = Utc::now();
        e.updated_at = Utc::now();
        e
    }

    fn make_title_component(entity_id: Uuid, title: &str) -> Component {
        Component::new(entity_id, ComponentType::Title, serde_json::json!(title))
    }

    // ---------------------------------------------------------------------------
    // Tests
    // ---------------------------------------------------------------------------

    #[tokio::test]
    async fn test_entities_grouped_by_type() {
        let concept = make_entity("Concept");
        let paper = make_entity("Paper");

        let entity_repo = MockEntityRepo {
            entities: vec![concept.clone(), paper.clone()],
        };

        let mut component_data = HashMap::new();
        component_data.insert(
            concept.id,
            vec![make_title_component(concept.id, "Transformer")],
        );
        component_data.insert(
            paper.id,
            vec![make_title_component(paper.id, "Attention Paper")],
        );
        let component_repo = MockComponentRepo {
            components: component_data,
        };

        let adapter = TreeViewAdapter::new(Box::new(entity_repo), Box::new(component_repo), None);

        let output = adapter.render(&ViewFilter::default()).await.unwrap();
        match output {
            ViewOutput::Tree(tree) => {
                // Should have 2 group branches (Concept and Paper)
                assert_eq!(tree.roots.len(), 2);
                // Each group should have 1 child
                for root in &tree.roots {
                    assert_eq!(root.children.len(), 1);
                }
            }
            other => panic!("Expected Tree output, got {:?}", other),
        }
    }

    #[tokio::test]
    async fn test_empty_canonical_data_produces_empty_tree() {
        let entity_repo = MockEntityRepo { entities: vec![] };
        let component_repo = MockComponentRepo::default();

        let adapter = TreeViewAdapter::new(Box::new(entity_repo), Box::new(component_repo), None);

        let output = adapter.render(&ViewFilter::default()).await.unwrap();
        match output {
            ViewOutput::Tree(tree) => {
                assert!(tree.roots.is_empty());
            }
            other => panic!("Expected Tree output, got {:?}", other),
        }
    }

    #[tokio::test]
    async fn test_filter_by_entity_type() {
        let concept = make_entity("Concept");
        let paper = make_entity("Paper");

        let entity_repo = MockEntityRepo {
            entities: vec![concept.clone(), paper.clone()],
        };

        let mut component_data = HashMap::new();
        component_data.insert(
            concept.id,
            vec![make_title_component(concept.id, "Transformer")],
        );
        component_data.insert(
            paper.id,
            vec![make_title_component(paper.id, "Attention Paper")],
        );
        let component_repo = MockComponentRepo {
            components: component_data,
        };

        let adapter = TreeViewAdapter::new(Box::new(entity_repo), Box::new(component_repo), None);

        let filter = ViewFilter {
            entity_types: Some(vec![EntityType::new("Concept")]),
            ..Default::default()
        };
        let output = adapter.render(&filter).await.unwrap();
        match output {
            ViewOutput::Tree(tree) => {
                // Only the Concept group should appear
                assert_eq!(tree.roots.len(), 1);
                assert_eq!(tree.roots[0].children.len(), 1);
                assert_eq!(
                    tree.roots[0].children[0].entity.entity_type,
                    EntityType::new("Concept")
                );
            }
            other => panic!("Expected Tree output, got {:?}", other),
        }
    }

    #[tokio::test]
    async fn test_collection_repo_none_produces_tree_without_collections() {
        let concept = make_entity("Concept");
        let entity_repo = MockEntityRepo {
            entities: vec![concept.clone()],
        };
        let mut component_data = HashMap::new();
        component_data.insert(concept.id, vec![make_title_component(concept.id, "Test")]);
        let component_repo = MockComponentRepo {
            components: component_data,
        };

        let adapter = TreeViewAdapter::new(Box::new(entity_repo), Box::new(component_repo), None);

        let output = adapter.render(&ViewFilter::default()).await.unwrap();
        match output {
            ViewOutput::Tree(tree) => {
                // Only 1 group branch (Concept), no collection branches
                assert_eq!(tree.roots.len(), 1);
            }
            other => panic!("Expected Tree output, got {:?}", other),
        }
    }

    #[tokio::test]
    async fn test_collection_repo_some_adds_collection_branches() {
        use knowledge_core::ports::Collection;

        struct MockCollectionRepo {
            collections: Vec<Collection>,
            members: HashMap<Uuid, Vec<Entity>>,
        }

        #[async_trait]
        impl CollectionRepository for MockCollectionRepo {
            async fn list(&self) -> Result<Vec<Collection>, StorageError> {
                Ok(self.collections.clone())
            }
            async fn get(&self, id: Uuid) -> Result<Option<Collection>, StorageError> {
                Ok(self.collections.iter().find(|c| c.id == id).cloned())
            }
            async fn get_members(&self, collection_id: Uuid) -> Result<Vec<Entity>, StorageError> {
                Ok(self
                    .members
                    .get(&collection_id)
                    .cloned()
                    .unwrap_or_default())
            }
        }

        let concept = make_entity("Concept");
        let member = make_entity("Paper");

        let collection = Collection {
            id: Uuid::new_v4(),
            name: "Papers to Read".to_string(),
            description: None,
            created_at: Utc::now(),
            updated_at: Utc::now(),
        };
        let collection_id = collection.id;

        let collection_repo = MockCollectionRepo {
            collections: vec![collection],
            members: {
                let mut m = HashMap::new();
                m.insert(collection_id, vec![member.clone()]);
                m
            },
        };

        let entity_repo = MockEntityRepo {
            entities: vec![concept.clone()],
        };

        let mut component_data = HashMap::new();
        component_data.insert(
            concept.id,
            vec![make_title_component(concept.id, "Transformer")],
        );
        let component_repo = MockComponentRepo {
            components: component_data,
        };

        let adapter = TreeViewAdapter::new(
            Box::new(entity_repo),
            Box::new(component_repo),
            Some(Box::new(collection_repo)),
        );

        let output = adapter.render(&ViewFilter::default()).await.unwrap();
        match output {
            ViewOutput::Tree(tree) => {
                // Should have 2 roots: 1 collection branch + 1 Concept group branch
                assert_eq!(tree.roots.len(), 2);
                // The collection branch should have 1 member
                let collection_branch = &tree.roots[0];
                assert_eq!(collection_branch.children.len(), 1);
                assert_eq!(
                    collection_branch.entity.entity_type,
                    EntityType::new("Collection")
                );
            }
            other => panic!("Expected Tree output, got {:?}", other),
        }
    }

    #[tokio::test]
    async fn test_view_name() {
        let entity_repo = MockEntityRepo::default();
        let component_repo = MockComponentRepo::default();
        let adapter = TreeViewAdapter::new(Box::new(entity_repo), Box::new(component_repo), None);
        assert_eq!(adapter.name(), "tree");
    }
}
