use knowledge_core::features::component::{Component, ComponentType};
use knowledge_core::features::entity::{Entity, EntityType};
use knowledge_core::features::relationship::{Relationship, RelationshipType};
use knowledge_core::ports::*;
use knowledge_storage::adapters::sqlite::SqliteStore;

fn test_store() -> SqliteStore {
    SqliteStore::new(":memory:").unwrap()
}

#[tokio::test]
async fn test_entity_full_lifecycle() {
    let store = test_store();

    // Create
    let mut entity = Entity::new(EntityType::Article);
    EntityRepository::save(&store, &entity).await.unwrap();

    // Read
    let loaded = EntityRepository::get(&store, entity.id).await.unwrap().unwrap();
    assert_eq!(loaded.id, entity.id);
    assert!(loaded.is_active);
    assert_eq!(loaded.version, 1);

    // Update via touch
    entity.touch();
    EntityRepository::save(&store, &entity).await.unwrap();
    let loaded = EntityRepository::get(&store, entity.id).await.unwrap().unwrap();
    assert_eq!(loaded.version, 2);

    // Archive
    entity.archive();
    EntityRepository::save(&store, &entity).await.unwrap();
    let loaded = EntityRepository::get(&store, entity.id).await.unwrap().unwrap();
    assert!(!loaded.is_active);

    // List should not include archived
    let all = EntityRepository::list(&store).await.unwrap();
    assert!(all.is_empty());

    // Restore
    entity.restore();
    EntityRepository::save(&store, &entity).await.unwrap();
    let all = EntityRepository::list(&store).await.unwrap();
    assert_eq!(all.len(), 1);

    // Delete
    EntityRepository::delete(&store, entity.id).await.unwrap();
    let loaded = EntityRepository::get(&store, entity.id).await.unwrap();
    assert!(loaded.is_none());
}

#[tokio::test]
async fn test_component_lifecycle_with_version_tracking() {
    let store = test_store();
    let entity = Entity::new(EntityType::Note);
    EntityRepository::save(&store, &entity).await.unwrap();

    // Create component
    let comp = Component::new(entity.id, ComponentType::Title, serde_json::json!("Original"));
    ComponentRepository::save(&store, &comp).await.unwrap();

    let comps = ComponentRepository::get(&store, entity.id).await.unwrap();
    assert_eq!(comps.len(), 1);
    assert_eq!(comps[0].data, serde_json::json!("Original"));
    assert_eq!(comps[0].version, 1);

    // Update component data
    ComponentRepository::update_data(&store, comp.id, serde_json::json!("Updated")).await.unwrap();

    let comps = ComponentRepository::get(&store, entity.id).await.unwrap();
    assert_eq!(comps[0].data, serde_json::json!("Updated"));
    assert_eq!(comps[0].version, 2);

    // Delete component
    ComponentRepository::delete(&store, comp.id).await.unwrap();
    let comps = ComponentRepository::get(&store, entity.id).await.unwrap();
    assert!(comps.is_empty());
}

#[tokio::test]
async fn test_relationship_1hop_traversal() {
    let store = test_store();
    let a = Entity::new(EntityType::Article);
    let b = Entity::new(EntityType::Concept);
    let c = Entity::new(EntityType::Person);
    EntityRepository::save(&store, &a).await.unwrap();
    EntityRepository::save(&store, &b).await.unwrap();
    EntityRepository::save(&store, &c).await.unwrap();

    // a -> b, a -> c
    let r1 = Relationship::new(a.id, b.id, RelationshipType::References);
    let r2 = Relationship::new(a.id, c.id, RelationshipType::References);
    RelationshipRepository::save(&store, &r1).await.unwrap();
    RelationshipRepository::save(&store, &r2).await.unwrap();

    // 1-hop from a
    let outgoing = RelationshipRepository::by_source(&store, a.id).await.unwrap();
    assert_eq!(outgoing.len(), 2);

    // 1-hop to b
    let incoming = RelationshipRepository::by_target(&store, b.id).await.unwrap();
    assert_eq!(incoming.len(), 1);

    // Find specific relationship
    let found = RelationshipRepository::find_by_source_and_target(&store, a.id, b.id).await.unwrap();
    assert!(found.is_some());

    // Query by type
    let refs = RelationshipRepository::find_by_type(&store, "References").await.unwrap();
    assert_eq!(refs.len(), 2);
}

#[tokio::test]
async fn test_search_with_type_and_tag_filtering() {
    let store = test_store();
    let article = Entity::new(EntityType::Article);
    let concept = Entity::new(EntityType::Concept);

    let article_comps = vec![
        Component::new(article.id, ComponentType::Title, serde_json::json!("Rust Programming")),
        Component::new(article.id, ComponentType::Content, serde_json::json!("Rust is a systems language")),
        Component::new(article.id, ComponentType::Tags, serde_json::json!(["rust", "programming"])),
    ];
    let concept_comps = vec![
        Component::new(concept.id, ComponentType::Title, serde_json::json!("Rust Language")),
        Component::new(concept.id, ComponentType::Content, serde_json::json!("Rust is a language")),
        Component::new(concept.id, ComponentType::Tags, serde_json::json!(["rust", "language"])),
    ];

    EntityRepository::save(&store, &article).await.unwrap();
    EntityRepository::save(&store, &concept).await.unwrap();
    store.index_entity(&article, &article_comps).await.unwrap();
    store.index_entity(&concept, &concept_comps).await.unwrap();

    // No filter
    let results = store.search(&SearchQuery {
        query: "Rust".to_string(),
        entity_type: None,
        tag: None,
    }).await.unwrap();
    assert_eq!(results.len(), 2);

    // Filter by type
    let results = store.search(&SearchQuery {
        query: "Rust".to_string(),
        entity_type: Some("Article".to_string()),
        tag: None,
    }).await.unwrap();
    assert_eq!(results.len(), 1);

    // Filter by tag
    let results = store.search(&SearchQuery {
        query: "Rust".to_string(),
        entity_type: None,
        tag: Some("programming".to_string()),
    }).await.unwrap();
    assert_eq!(results.len(), 1);
}

#[tokio::test]
async fn test_search_snippets() {
    let store = test_store();
    let entity = Entity::new(EntityType::Article);
    let comps = vec![
        Component::new(entity.id, ComponentType::Title, serde_json::json!("Transformer Architecture")),
        Component::new(entity.id, ComponentType::Content, serde_json::json!("The transformer model uses self-attention mechanisms for sequence processing")),
    ];
    store.index_entity(&entity, &comps).await.unwrap();

    let results = store.search(&SearchQuery {
        query: "attention".to_string(),
        entity_type: None,
        tag: None,
    }).await.unwrap();

    assert_eq!(results.len(), 1);
    assert!(results[0].snippet.is_some());
    let snippet = results[0].snippet.as_ref().unwrap();
    assert!(snippet.contains("attention"));
}

#[tokio::test]
async fn test_search_rebuild() {
    let store = test_store();
    let entity = Entity::new(EntityType::Article);
    let comps = vec![
        Component::new(entity.id, ComponentType::Title, serde_json::json!("Test")),
        Component::new(entity.id, ComponentType::Content, serde_json::json!("Content")),
    ];
    store.index_entity(&entity, &comps).await.unwrap();

    // Verify search works
    let results = store.search(&SearchQuery {
        query: "Test".to_string(),
        entity_type: None,
        tag: None,
    }).await.unwrap();
    assert_eq!(results.len(), 1);

    // Rebuild
    store.rebuild(&[(entity.clone(), comps.clone())]).await.unwrap();

    // Verify search still works
    let results = store.search(&SearchQuery {
        query: "Test".to_string(),
        entity_type: None,
        tag: None,
    }).await.unwrap();
    assert_eq!(results.len(), 1);
}

#[tokio::test]
async fn test_event_log_full() {
    let store = test_store();
    let entity = Entity::new(EntityType::Article);

    let e1 = Event {
        id: uuid::Uuid::new_v4(),
        event_type: EventType::EntityCreated,
        entity_id: entity.id,
        timestamp: chrono::Utc::now(),
        data: serde_json::json!({}),
    };
    let e2 = Event {
        id: uuid::Uuid::new_v4(),
        event_type: EventType::ComponentAdded,
        entity_id: entity.id,
        timestamp: chrono::Utc::now(),
        data: serde_json::json!({}),
    };

    store.append(&e1).await.unwrap();
    store.append(&e2).await.unwrap();

    let events = store.list_by_entity(entity.id).await.unwrap();
    assert_eq!(events.len(), 2);
    assert_eq!(events[0].event_type, EventType::ComponentAdded);
    assert_eq!(events[1].event_type, EventType::EntityCreated);
}

#[tokio::test]
async fn test_version_history_tracking() {
    let store = test_store();
    let entity = Entity::new(EntityType::Article);
    EntityRepository::save(&store, &entity).await.unwrap();

    EntityRepository::increment_version(&store, entity.id).await.unwrap();
    EntityRepository::increment_version(&store, entity.id).await.unwrap();

    let history = EntityRepository::get_version_history(&store, entity.id).await.unwrap();
    assert_eq!(history.len(), 2);
    assert_eq!(history[0].version, 2);
    assert_eq!(history[1].version, 1);
}

#[tokio::test]
async fn test_transactional_write_entity_and_components() {
    let store = test_store();
    let entity = Entity::new(EntityType::Article);
    let components = vec![
        Component::new(entity.id, ComponentType::Title, serde_json::json!("Title")),
        Component::new(entity.id, ComponentType::Content, serde_json::json!("Body")),
        Component::new(entity.id, ComponentType::Tags, serde_json::json!(["tag1"])),
    ];
    let event = Event {
        id: uuid::Uuid::new_v4(),
        event_type: EventType::EntityCreated,
        entity_id: entity.id,
        timestamp: chrono::Utc::now(),
        data: serde_json::json!({}),
    };

    store.save_entity_with_components(&entity, &components, &event).await.unwrap();

    let loaded = EntityRepository::get(&store, entity.id).await.unwrap().unwrap();
    assert_eq!(loaded.id, entity.id);

    let comps = ComponentRepository::get(&store, entity.id).await.unwrap();
    assert_eq!(comps.len(), 3);

    let events = store.list_by_entity(entity.id).await.unwrap();
    assert_eq!(events.len(), 1);
}

#[tokio::test]
async fn test_update_entity_with_components_replaces_all() {
    let store = test_store();
    let entity = Entity::new(EntityType::Article);
    let components = vec![
        Component::new(entity.id, ComponentType::Title, serde_json::json!("Original Title")),
        Component::new(entity.id, ComponentType::Content, serde_json::json!("Original Body")),
    ];
    let event = Event {
        id: uuid::Uuid::new_v4(),
        event_type: EventType::EntityCreated,
        entity_id: entity.id,
        timestamp: chrono::Utc::now(),
        data: serde_json::json!({}),
    };

    store.save_entity_with_components(&entity, &components, &event).await.unwrap();

    // Update with different components
    let new_entity = {
        let mut e = entity.clone();
        e.touch();
        e
    };
    let new_components = vec![
        Component::new(entity.id, ComponentType::Title, serde_json::json!("New Title")),
        Component::new(entity.id, ComponentType::Content, serde_json::json!("New Body")),
        Component::new(entity.id, ComponentType::Tags, serde_json::json!(["new-tag"])),
    ];
    let update_event = Event {
        id: uuid::Uuid::new_v4(),
        event_type: EventType::EntityUpdated,
        entity_id: entity.id,
        timestamp: chrono::Utc::now(),
        data: serde_json::json!({}),
    };

    store.update_entity_with_components(&new_entity, &new_components, &update_event).await.unwrap();

    let comps = ComponentRepository::get(&store, entity.id).await.unwrap();
    assert_eq!(comps.len(), 3);

    let title = comps.iter().find(|c| c.component_type == ComponentType::Title).unwrap();
    assert_eq!(title.data, serde_json::json!("New Title"));

    let tags = comps.iter().find(|c| c.component_type == ComponentType::Tags).unwrap();
    assert_eq!(tags.data, serde_json::json!(["new-tag"]));
}

#[tokio::test]
async fn test_entity_resolver_exact_match() {
    let store = test_store();

    let existing = Entity::new(EntityType::Article);
    EntityRepository::save(&store, &existing).await.unwrap();
    let title_comp = Component::new(existing.id, ComponentType::Title, serde_json::json!("Test Document"));
    ComponentRepository::save(&store, &title_comp).await.unwrap();

    let candidate = Entity::new(EntityType::Article);
    let candidates = EntityResolver::find_candidates(&store, &candidate, "Test Document").await.unwrap();
    assert_eq!(candidates.len(), 1);
    assert_eq!(candidates[0].entity_id, existing.id);
    assert_eq!(candidates[0].confidence, 1.0);
}

#[tokio::test]
async fn test_entity_resolver_no_match_different_type() {
    let store = test_store();

    let existing = Entity::new(EntityType::Article);
    EntityRepository::save(&store, &existing).await.unwrap();
    let title_comp = Component::new(existing.id, ComponentType::Title, serde_json::json!("Test Document"));
    ComponentRepository::save(&store, &title_comp).await.unwrap();

    let candidate = Entity::new(EntityType::Concept);
    let candidates = EntityResolver::find_candidates(&store, &candidate, "Test Document").await.unwrap();
    assert!(candidates.is_empty());
}

#[tokio::test]
async fn test_entity_resolver_no_match_different_title() {
    let store = test_store();

    let existing = Entity::new(EntityType::Article);
    EntityRepository::save(&store, &existing).await.unwrap();
    let title_comp = Component::new(existing.id, ComponentType::Title, serde_json::json!("Existing Document"));
    ComponentRepository::save(&store, &title_comp).await.unwrap();

    let candidate = Entity::new(EntityType::Article);
    let candidates = EntityResolver::find_candidates(&store, &candidate, "Different Title").await.unwrap();
    assert!(candidates.is_empty());
}

#[tokio::test]
async fn test_entity_resolver_merge() {
    let store = test_store();

    let canonical = Entity::new(EntityType::Article);
    let duplicate = Entity::new(EntityType::Article);
    EntityRepository::save(&store, &canonical).await.unwrap();
    EntityRepository::save(&store, &duplicate).await.unwrap();

    let rel = Relationship::new(duplicate.id, canonical.id, RelationshipType::References);
    RelationshipRepository::save(&store, &rel).await.unwrap();

    let comp = Component::new(duplicate.id, ComponentType::Content, serde_json::json!("data"));
    ComponentRepository::save(&store, &comp).await.unwrap();

    EntityResolver::merge(&store, canonical.id, duplicate.id, 1.0).await.unwrap();

    let loaded = EntityRepository::get(&store, duplicate.id).await.unwrap();
    assert!(loaded.is_none());

    let comps = ComponentRepository::get(&store, canonical.id).await.unwrap();
    assert_eq!(comps.len(), 1);

    let rels = RelationshipRepository::by_source(&store, canonical.id).await.unwrap();
    assert_eq!(rels.len(), 0);
}

#[tokio::test]
async fn test_find_by_component_data() {
    let store = test_store();
    let entity = Entity::new(EntityType::Article);
    EntityRepository::save(&store, &entity).await.unwrap();

    let prov = Component::new(
        entity.id,
        ComponentType::Provenance,
        serde_json::json!({"source": "test.md", "imported_at": "2026-01-01T00:00:00Z"}),
    );
    ComponentRepository::save(&store, &prov).await.unwrap();

    let found = ComponentRepository::find_by_component_data(&store, "Provenance", "source", "test.md").await.unwrap();
    assert_eq!(found.len(), 1);
    assert_eq!(found[0].entity_id, entity.id);

    let not_found = ComponentRepository::find_by_component_data(&store, "Provenance", "source", "other.md").await.unwrap();
    assert!(not_found.is_empty());
}

#[tokio::test]
async fn test_delete_by_entity() {
    let store = test_store();
    let entity = Entity::new(EntityType::Article);
    EntityRepository::save(&store, &entity).await.unwrap();

    ComponentRepository::save(&store, &Component::new(entity.id, ComponentType::Title, serde_json::json!("t"))).await.unwrap();
    ComponentRepository::save(&store, &Component::new(entity.id, ComponentType::Content, serde_json::json!("c"))).await.unwrap();

    let comps = ComponentRepository::get(&store, entity.id).await.unwrap();
    assert_eq!(comps.len(), 2);

    ComponentRepository::delete_by_entity(&store, entity.id).await.unwrap();

    let comps = ComponentRepository::get(&store, entity.id).await.unwrap();
    assert!(comps.is_empty());
}
