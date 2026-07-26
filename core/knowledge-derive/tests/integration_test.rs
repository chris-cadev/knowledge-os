use async_trait::async_trait;
use knowledge_core::features::component::{Component, ComponentType};
use knowledge_core::features::entity::{Entity, EntityType};
use knowledge_core::ports::{
    ComponentRepository, EntityRepository, EntityVersion, Event, EventNotifier, EventType,
    StorageError, ViewFilter, ViewRegistry,
};
use std::collections::HashMap;
use uuid::Uuid;

// ---------------------------------------------------------------------------
// Mock repositories (shared across integration tests)
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
    ) -> Result<Vec<EntityVersion>, StorageError> {
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

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

fn make_entity(entity_type: &str) -> Entity {
    Entity::new(EntityType::new(entity_type))
}

fn make_title_component(entity_id: Uuid, title: &str) -> Component {
    Component::new(entity_id, ComponentType::Title, serde_json::json!(title))
}

fn make_event(event_type: EventType, entity_id: Uuid) -> Event {
    Event {
        id: Uuid::new_v4(),
        event_type,
        entity_id,
        timestamp: chrono::Utc::now(),
        data: serde_json::json!({}),
    }
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[tokio::test]
async fn test_registry_renders_named_view() {
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

    let mut registry = ViewRegistry::new();
    registry.register(Box::new(
        knowledge_derive::features::view::tree::TreeViewAdapter::new(
            Box::new(entity_repo),
            Box::new(component_repo),
            None,
        ),
    ));

    let output = registry
        .render("tree", &ViewFilter::default())
        .await
        .unwrap();
    match output {
        knowledge_core::ports::ViewOutput::Tree(tree) => {
            assert_eq!(tree.roots.len(), 2);
        }
        other => panic!("Expected Tree output, got {:?}", other),
    }
}

#[tokio::test]
async fn test_registry_returns_error_for_unknown_view() {
    let registry = ViewRegistry::new();
    let result = registry.render("nonexistent", &ViewFilter::default()).await;
    assert!(result.is_err());
}

#[tokio::test]
async fn test_registry_dispatches_on_event_to_all_views() {
    let entity_repo = MockEntityRepo::default();
    let component_repo = MockComponentRepo::default();

    let mut registry = ViewRegistry::new();
    registry.register(Box::new(
        knowledge_derive::features::view::tree::TreeViewAdapter::new(
            Box::new(entity_repo),
            Box::new(component_repo),
            None,
        ),
    ));

    let event = make_event(EventType::EntityCreated, Uuid::new_v4());
    // on_event should not fail for any registered view
    registry.notify(&event).await.unwrap();
}

#[tokio::test]
async fn test_registry_list_views() {
    let entity_repo = MockEntityRepo::default();
    let component_repo = MockComponentRepo::default();

    let mut registry = ViewRegistry::new();
    registry.register(Box::new(
        knowledge_derive::features::view::tree::TreeViewAdapter::new(
            Box::new(entity_repo),
            Box::new(component_repo),
            None,
        ),
    ));

    let views = registry.list_views();
    assert_eq!(views.len(), 1);
    assert!(views.contains(&"tree".to_string()));
}

#[tokio::test]
async fn test_registry_render_with_filter() {
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

    let mut registry = ViewRegistry::new();
    registry.register(Box::new(
        knowledge_derive::features::view::tree::TreeViewAdapter::new(
            Box::new(entity_repo),
            Box::new(component_repo),
            None,
        ),
    ));

    // Filter to Concept only
    let filter = ViewFilter {
        entity_types: Some(vec![EntityType::new("Concept")]),
        ..Default::default()
    };
    let output = registry.render("tree", &filter).await.unwrap();
    match output {
        knowledge_core::ports::ViewOutput::Tree(tree) => {
            assert_eq!(tree.roots.len(), 1);
            assert_eq!(tree.roots[0].entity.entity_type, EntityType::new("Concept"));
        }
        other => panic!("Expected Tree output, got {:?}", other),
    }
}
