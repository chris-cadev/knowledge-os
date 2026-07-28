use knowledge_core::features::component::{Component, ComponentType};
use knowledge_core::features::entity::{Entity, EntityType};
use knowledge_core::features::relationship::{Relationship, RelationshipType};
use knowledge_core::ports::{
    ComponentRepository, EntityRepository, Event, EventLog, EventType, RelationshipRepository,
    SearchIndex, SearchQuery, TransactionalWrite,
};

use super::store::SqliteStore;

fn test_store() -> SqliteStore {
    SqliteStore::new(":memory:").unwrap()
}

#[tokio::test]
async fn test_entity_crud() {
    let store = test_store();
    let mut entity = Entity::new(EntityType::new("Article"));

    EntityRepository::save(&store, &entity).await.unwrap();
    let loaded = EntityRepository::get(&store, entity.id)
        .await
        .unwrap()
        .unwrap();
    assert_eq!(loaded.id, entity.id);
    assert_eq!(loaded.entity_type, EntityType::new("Article"));
    assert!(loaded.is_active);

    entity.touch();
    EntityRepository::save(&store, &entity).await.unwrap();
    let loaded = EntityRepository::get(&store, entity.id)
        .await
        .unwrap()
        .unwrap();
    assert_eq!(loaded.version, 2);

    let all = EntityRepository::list(&store).await.unwrap();
    assert_eq!(all.len(), 1);

    let articles = EntityRepository::find_by_type(&store, "Article")
        .await
        .unwrap();
    assert_eq!(articles.len(), 1);

    EntityRepository::delete(&store, entity.id).await.unwrap();
    let all = EntityRepository::list(&store).await.unwrap();
    assert!(all.is_empty());
}

#[tokio::test]
async fn test_component_crud() {
    let store = test_store();
    let entity = Entity::new(EntityType::new("Note"));
    EntityRepository::save(&store, &entity).await.unwrap();

    let component = Component::new(
        entity.id,
        ComponentType::Title,
        serde_json::json!("Test Title"),
    );
    ComponentRepository::save(&store, &component).await.unwrap();

    let components = ComponentRepository::get(&store, entity.id).await.unwrap();
    assert_eq!(components.len(), 1);
    assert_eq!(components[0].component_type, ComponentType::Title);

    let found = ComponentRepository::find_by_type(&store, entity.id, "Title")
        .await
        .unwrap();
    assert_eq!(found.len(), 1);

    ComponentRepository::update_data(&store, component.id, serde_json::json!("Updated Title"))
        .await
        .unwrap();
    let updated = ComponentRepository::get(&store, entity.id).await.unwrap();
    assert_eq!(updated[0].data, serde_json::json!("Updated Title"));
    assert_eq!(updated[0].version, 2);

    ComponentRepository::delete(&store, component.id)
        .await
        .unwrap();
    let components = ComponentRepository::get(&store, entity.id).await.unwrap();
    assert!(components.is_empty());
}

#[tokio::test]
async fn test_relationship_crud() {
    let store = test_store();
    let entity1 = Entity::new(EntityType::new("Article"));
    let entity2 = Entity::new(EntityType::new("Concept"));
    EntityRepository::save(&store, &entity1).await.unwrap();
    EntityRepository::save(&store, &entity2).await.unwrap();

    let rel = Relationship::new(entity1.id, entity2.id, RelationshipType::References);
    RelationshipRepository::save(&store, &rel).await.unwrap();

    let rels = RelationshipRepository::by_source(&store, entity1.id)
        .await
        .unwrap();
    assert_eq!(rels.len(), 1);

    let rels = RelationshipRepository::by_target(&store, entity2.id)
        .await
        .unwrap();
    assert_eq!(rels.len(), 1);

    let found = RelationshipRepository::find_by_source_and_target(&store, entity1.id, entity2.id)
        .await
        .unwrap();
    assert!(found.is_some());

    let refs = RelationshipRepository::find_by_type(&store, "References")
        .await
        .unwrap();
    assert_eq!(refs.len(), 1);

    RelationshipRepository::delete(&store, rel.id)
        .await
        .unwrap();
    let rels = RelationshipRepository::by_source(&store, entity1.id)
        .await
        .unwrap();
    assert!(rels.is_empty());
}

#[tokio::test]
async fn test_search_index() {
    let store = test_store();
    let entity = Entity::new(EntityType::new("Article"));
    let entity_id = entity.id;

    let components = vec![
        Component::new(
            entity_id,
            ComponentType::Title,
            serde_json::json!("Test Title"),
        ),
        Component::new(
            entity_id,
            ComponentType::Content,
            serde_json::json!("Some content here"),
        ),
        Component::new(
            entity_id,
            ComponentType::Tags,
            serde_json::json!(["rust", "test"]),
        ),
    ];

    store.index_entity(&entity, &components).await.unwrap();

    let results = store
        .search(&SearchQuery {
            query: "Test".to_string(),
            entity_type: None,
            tag: None,
        })
        .await
        .unwrap();
    assert_eq!(results.len(), 1);
    assert_eq!(results[0].entity_id, entity_id);
    assert!(results[0].score < 0.0);

    let results = store
        .search(&SearchQuery {
            query: "content".to_string(),
            entity_type: None,
            tag: Some("rust".to_string()),
        })
        .await
        .unwrap();
    assert_eq!(results.len(), 1);

    let results = store
        .search(&SearchQuery {
            query: "nonexistent".to_string(),
            entity_type: None,
            tag: None,
        })
        .await
        .unwrap();
    assert!(results.is_empty());
}

#[tokio::test]
async fn test_search_rebuild() {
    let store = test_store();
    let entity = Entity::new(EntityType::new("Article"));

    let components = vec![
        Component::new(
            entity.id,
            ComponentType::Title,
            serde_json::json!("Test Title"),
        ),
        Component::new(
            entity.id,
            ComponentType::Content,
            serde_json::json!("Some content here"),
        ),
    ];

    store.index_entity(&entity, &components).await.unwrap();

    let results = store
        .search(&SearchQuery {
            query: "Test".to_string(),
            entity_type: None,
            tag: None,
        })
        .await
        .unwrap();
    assert_eq!(results.len(), 1);

    store
        .rebuild(&[(entity.clone(), components.clone())])
        .await
        .unwrap();

    let results = store
        .search(&SearchQuery {
            query: "Test".to_string(),
            entity_type: None,
            tag: None,
        })
        .await
        .unwrap();
    assert_eq!(results.len(), 1);
}

#[tokio::test]
async fn test_event_log() {
    let store = test_store();
    let entity = Entity::new(EntityType::new("Article"));

    let event = Event {
        id: uuid::Uuid::new_v4(),
        event_type: EventType::EntityCreated,
        entity_id: entity.id,
        timestamp: chrono::Utc::now(),
        data: serde_json::json!({"entity_type": "Article"}),
    };

    store.append(&event).await.unwrap();

    let events = store.list_by_entity(entity.id).await.unwrap();
    assert_eq!(events.len(), 1);
    assert_eq!(events[0].event_type, EventType::EntityCreated);
}

#[tokio::test]
async fn test_increment_version() {
    let store = test_store();
    let entity = Entity::new(EntityType::new("Article"));
    EntityRepository::save(&store, &entity).await.unwrap();

    EntityRepository::increment_version(&store, entity.id)
        .await
        .unwrap();
    let loaded = EntityRepository::get(&store, entity.id)
        .await
        .unwrap()
        .unwrap();
    assert_eq!(loaded.version, 2);

    EntityRepository::increment_version(&store, entity.id)
        .await
        .unwrap();
    let loaded = EntityRepository::get(&store, entity.id)
        .await
        .unwrap()
        .unwrap();
    assert_eq!(loaded.version, 3);

    let history = EntityRepository::get_version_history(&store, entity.id)
        .await
        .unwrap();
    assert_eq!(history.len(), 2);
    assert_eq!(history[0].version, 2);
    assert_eq!(history[1].version, 1);
}

#[tokio::test]
async fn test_find_by_component_type() {
    let store = test_store();
    let entity = Entity::new(EntityType::new("Article"));
    EntityRepository::save(&store, &entity).await.unwrap();

    let comp = Component::new(
        entity.id,
        ComponentType::Timeline,
        serde_json::json!({"created_at": "2026-01-01"}),
    );
    ComponentRepository::save(&store, &comp).await.unwrap();

    let found = EntityRepository::find_by_component_type(&store, "Timeline")
        .await
        .unwrap();
    assert_eq!(found.len(), 1);
    assert_eq!(found[0].id, entity.id);

    let not_found = EntityRepository::find_by_component_type(&store, "Embedding")
        .await
        .unwrap();
    assert!(not_found.is_empty());
}

#[tokio::test]
async fn test_find_by_tag() {
    let store = test_store();
    let entity = Entity::new(EntityType::new("Article"));
    EntityRepository::save(&store, &entity).await.unwrap();

    let comp = Component::new(
        entity.id,
        ComponentType::Tags,
        serde_json::json!(["rust", "testing"]),
    );
    ComponentRepository::save(&store, &comp).await.unwrap();

    let found = EntityRepository::find_by_tag(&store, "rust").await.unwrap();
    assert_eq!(found.len(), 1);

    let not_found = EntityRepository::find_by_tag(&store, "python")
        .await
        .unwrap();
    assert!(not_found.is_empty());
}

#[tokio::test]
async fn test_relationship_update() {
    let store = test_store();
    let entity1 = Entity::new(EntityType::new("Article"));
    let entity2 = Entity::new(EntityType::new("Concept"));
    EntityRepository::save(&store, &entity1).await.unwrap();
    EntityRepository::save(&store, &entity2).await.unwrap();

    let mut rel = Relationship::new(entity1.id, entity2.id, RelationshipType::References);
    RelationshipRepository::save(&store, &rel).await.unwrap();

    rel.is_active = false;
    RelationshipRepository::update(&store, &rel).await.unwrap();

    let updated = RelationshipRepository::get(&store, rel.id)
        .await
        .unwrap()
        .unwrap();
    assert!(!updated.is_active);
}

#[tokio::test]
async fn test_transactional_write() {
    let store = test_store();
    let entity = Entity::new(EntityType::new("Article"));
    let components = vec![
        Component::new(
            entity.id,
            ComponentType::Title,
            serde_json::json!("Transactional Test"),
        ),
        Component::new(entity.id, ComponentType::Content, serde_json::json!("Body")),
    ];
    let event = Event {
        id: uuid::Uuid::new_v4(),
        event_type: EventType::EntityCreated,
        entity_id: entity.id,
        timestamp: chrono::Utc::now(),
        data: serde_json::json!({}),
    };

    store
        .save_entity_with_components(&entity, &components, &event)
        .await
        .unwrap();

    let loaded = EntityRepository::get(&store, entity.id)
        .await
        .unwrap()
        .unwrap();
    assert_eq!(loaded.id, entity.id);

    let comps = ComponentRepository::get(&store, entity.id).await.unwrap();
    assert_eq!(comps.len(), 2);

    let events = store.list_by_entity(entity.id).await.unwrap();
    assert_eq!(events.len(), 1);
    assert_eq!(events[0].event_type, EventType::EntityCreated);
}
