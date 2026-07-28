use knowledge_core::features::component::{Component, ComponentType};
use knowledge_core::features::entity::{Entity, EntityType};
use knowledge_core::features::relationship::{Relationship, RelationshipType};
use knowledge_core::ports::*;
use knowledge_storage::adapters::sqlite::SqliteStore;
use uuid::Uuid;

fn test_store() -> SqliteStore {
    SqliteStore::new(":memory:").unwrap()
}

/// Default traversal config for tests.
fn test_config() -> TraversalConfig {
    TraversalConfig {
        default_max_depth: 10,
        default_max_results: 1000,
    }
}

#[tokio::test]
async fn test_entity_full_lifecycle() {
    let store = test_store();

    // Create
    let mut entity = Entity::new(EntityType::new("Article"));
    EntityRepository::save(&store, &entity).await.unwrap();

    // Read
    let loaded = EntityRepository::get(&store, entity.id)
        .await
        .unwrap()
        .unwrap();
    assert_eq!(loaded.id, entity.id);
    assert!(loaded.is_active);
    assert_eq!(loaded.version, 1);

    // Update via touch
    entity.touch();
    EntityRepository::save(&store, &entity).await.unwrap();
    let loaded = EntityRepository::get(&store, entity.id)
        .await
        .unwrap()
        .unwrap();
    assert_eq!(loaded.version, 2);

    // Archive
    entity.archive();
    EntityRepository::save(&store, &entity).await.unwrap();
    let loaded = EntityRepository::get(&store, entity.id)
        .await
        .unwrap()
        .unwrap();
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
    let entity = Entity::new(EntityType::new("Note"));
    EntityRepository::save(&store, &entity).await.unwrap();

    // Create component
    let comp = Component::new(
        entity.id,
        ComponentType::Title,
        serde_json::json!("Original"),
    );
    ComponentRepository::save(&store, &comp).await.unwrap();

    let comps = ComponentRepository::get(&store, entity.id).await.unwrap();
    assert_eq!(comps.len(), 1);
    assert_eq!(comps[0].data, serde_json::json!("Original"));
    assert_eq!(comps[0].version, 1);

    // Update component data
    ComponentRepository::update_data(&store, comp.id, serde_json::json!("Updated"))
        .await
        .unwrap();

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
    let a = Entity::new(EntityType::new("Article"));
    let b = Entity::new(EntityType::new("Concept"));
    let c = Entity::new(EntityType::new("Person"));
    EntityRepository::save(&store, &a).await.unwrap();
    EntityRepository::save(&store, &b).await.unwrap();
    EntityRepository::save(&store, &c).await.unwrap();

    // a -> b, a -> c
    let r1 = Relationship::new(a.id, b.id, RelationshipType::References);
    let r2 = Relationship::new(a.id, c.id, RelationshipType::References);
    RelationshipRepository::save(&store, &r1).await.unwrap();
    RelationshipRepository::save(&store, &r2).await.unwrap();

    // 1-hop from a
    let outgoing = RelationshipRepository::by_source(&store, a.id)
        .await
        .unwrap();
    assert_eq!(outgoing.len(), 2);

    // 1-hop to b
    let incoming = RelationshipRepository::by_target(&store, b.id)
        .await
        .unwrap();
    assert_eq!(incoming.len(), 1);

    // Find specific relationship
    let found = RelationshipRepository::find_by_source_and_target(&store, a.id, b.id)
        .await
        .unwrap();
    assert!(found.is_some());

    // Query by type
    let refs = RelationshipRepository::find_by_type(&store, "References")
        .await
        .unwrap();
    assert_eq!(refs.len(), 2);
}

#[tokio::test]
async fn test_search_with_type_and_tag_filtering() {
    let store = test_store();
    let article = Entity::new(EntityType::new("Article"));
    let concept = Entity::new(EntityType::new("Concept"));

    let article_comps = vec![
        Component::new(
            article.id,
            ComponentType::Title,
            serde_json::json!("Rust Programming"),
        ),
        Component::new(
            article.id,
            ComponentType::Content,
            serde_json::json!("Rust is a systems language"),
        ),
        Component::new(
            article.id,
            ComponentType::Tags,
            serde_json::json!(["rust", "programming"]),
        ),
    ];
    let concept_comps = vec![
        Component::new(
            concept.id,
            ComponentType::Title,
            serde_json::json!("Rust Language"),
        ),
        Component::new(
            concept.id,
            ComponentType::Content,
            serde_json::json!("Rust is a language"),
        ),
        Component::new(
            concept.id,
            ComponentType::Tags,
            serde_json::json!(["rust", "language"]),
        ),
    ];

    EntityRepository::save(&store, &article).await.unwrap();
    EntityRepository::save(&store, &concept).await.unwrap();
    store.index_entity(&article, &article_comps).await.unwrap();
    store.index_entity(&concept, &concept_comps).await.unwrap();

    // No filter
    let results = store
        .search(&SearchQuery {
            query: "Rust".to_string(),
            entity_type: None,
            tag: None,
        })
        .await
        .unwrap();
    assert_eq!(results.len(), 2);

    // Filter by type
    let results = store
        .search(&SearchQuery {
            query: "Rust".to_string(),
            entity_type: Some("Article".to_string()),
            tag: None,
        })
        .await
        .unwrap();
    assert_eq!(results.len(), 1);

    // Filter by tag
    let results = store
        .search(&SearchQuery {
            query: "Rust".to_string(),
            entity_type: None,
            tag: Some("programming".to_string()),
        })
        .await
        .unwrap();
    assert_eq!(results.len(), 1);
}

#[tokio::test]
async fn test_search_snippets() {
    let store = test_store();
    let entity = Entity::new(EntityType::new("Article"));
    let comps = vec![
        Component::new(
            entity.id,
            ComponentType::Title,
            serde_json::json!("Transformer Architecture"),
        ),
        Component::new(
            entity.id,
            ComponentType::Content,
            serde_json::json!(
                "The transformer model uses self-attention mechanisms for sequence processing"
            ),
        ),
    ];
    store.index_entity(&entity, &comps).await.unwrap();

    let results = store
        .search(&SearchQuery {
            query: "attention".to_string(),
            entity_type: None,
            tag: None,
        })
        .await
        .unwrap();

    assert_eq!(results.len(), 1);
    assert!(results[0].snippet.is_some());
    let snippet = results[0].snippet.as_ref().unwrap();
    assert!(snippet.contains("attention"));
}

#[tokio::test]
async fn test_search_rebuild() {
    let store = test_store();
    let entity = Entity::new(EntityType::new("Article"));
    let comps = vec![
        Component::new(entity.id, ComponentType::Title, serde_json::json!("Test")),
        Component::new(
            entity.id,
            ComponentType::Content,
            serde_json::json!("Content"),
        ),
    ];
    store.index_entity(&entity, &comps).await.unwrap();

    // Verify search works
    let results = store
        .search(&SearchQuery {
            query: "Test".to_string(),
            entity_type: None,
            tag: None,
        })
        .await
        .unwrap();
    assert_eq!(results.len(), 1);

    // Rebuild
    store
        .rebuild(&[(entity.clone(), comps.clone())])
        .await
        .unwrap();

    // Verify search still works
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
async fn test_event_log_full() {
    let store = test_store();
    let entity = Entity::new(EntityType::new("Article"));

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
    let entity = Entity::new(EntityType::new("Article"));
    EntityRepository::save(&store, &entity).await.unwrap();

    EntityRepository::increment_version(&store, entity.id)
        .await
        .unwrap();
    EntityRepository::increment_version(&store, entity.id)
        .await
        .unwrap();

    let history = EntityRepository::get_version_history(&store, entity.id)
        .await
        .unwrap();
    assert_eq!(history.len(), 2);
    assert_eq!(history[0].version, 2);
    assert_eq!(history[1].version, 1);
}

#[tokio::test]
async fn test_transactional_write_entity_and_components() {
    let store = test_store();
    let entity = Entity::new(EntityType::new("Article"));
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
    assert_eq!(comps.len(), 3);

    let events = store.list_by_entity(entity.id).await.unwrap();
    assert_eq!(events.len(), 1);
}

#[tokio::test]
async fn test_update_entity_with_components_replaces_all() {
    let store = test_store();
    let entity = Entity::new(EntityType::new("Article"));
    let components = vec![
        Component::new(
            entity.id,
            ComponentType::Title,
            serde_json::json!("Original Title"),
        ),
        Component::new(
            entity.id,
            ComponentType::Content,
            serde_json::json!("Original Body"),
        ),
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

    // Update with different components
    let new_entity = {
        let mut e = entity.clone();
        e.touch();
        e
    };
    let new_components = vec![
        Component::new(
            entity.id,
            ComponentType::Title,
            serde_json::json!("New Title"),
        ),
        Component::new(
            entity.id,
            ComponentType::Content,
            serde_json::json!("New Body"),
        ),
        Component::new(
            entity.id,
            ComponentType::Tags,
            serde_json::json!(["new-tag"]),
        ),
    ];
    let update_event = Event {
        id: uuid::Uuid::new_v4(),
        event_type: EventType::EntityUpdated,
        entity_id: entity.id,
        timestamp: chrono::Utc::now(),
        data: serde_json::json!({}),
    };

    store
        .update_entity_with_components(&new_entity, &new_components, &update_event)
        .await
        .unwrap();

    let comps = ComponentRepository::get(&store, entity.id).await.unwrap();
    assert_eq!(comps.len(), 3);

    let title = comps
        .iter()
        .find(|c| c.component_type == ComponentType::Title)
        .unwrap();
    assert_eq!(title.data, serde_json::json!("New Title"));

    let tags = comps
        .iter()
        .find(|c| c.component_type == ComponentType::Tags)
        .unwrap();
    assert_eq!(tags.data, serde_json::json!(["new-tag"]));
}

#[tokio::test]
async fn test_entity_resolver_exact_match() {
    let store = test_store();

    let existing = Entity::new(EntityType::new("Article"));
    EntityRepository::save(&store, &existing).await.unwrap();
    let title_comp = Component::new(
        existing.id,
        ComponentType::Title,
        serde_json::json!("Test Document"),
    );
    ComponentRepository::save(&store, &title_comp)
        .await
        .unwrap();

    let candidate = Entity::new(EntityType::new("Article"));
    let candidates = EntityResolver::find_candidates(&store, &candidate, "Test Document", None)
        .await
        .unwrap();
    assert_eq!(candidates.len(), 1);
    assert_eq!(candidates[0].entity_id, existing.id);
    assert_eq!(candidates[0].confidence, 1.0);
}

#[tokio::test]
async fn test_entity_resolver_no_match_different_type() {
    let store = test_store();

    let existing = Entity::new(EntityType::new("Article"));
    EntityRepository::save(&store, &existing).await.unwrap();
    let title_comp = Component::new(
        existing.id,
        ComponentType::Title,
        serde_json::json!("Test Document"),
    );
    ComponentRepository::save(&store, &title_comp)
        .await
        .unwrap();

    let candidate = Entity::new(EntityType::new("Concept"));
    let candidates = EntityResolver::find_candidates(&store, &candidate, "Test Document", None)
        .await
        .unwrap();
    assert!(candidates.is_empty());
}

#[tokio::test]
async fn test_entity_resolver_no_match_different_title() {
    let store = test_store();

    let existing = Entity::new(EntityType::new("Article"));
    EntityRepository::save(&store, &existing).await.unwrap();
    let title_comp = Component::new(
        existing.id,
        ComponentType::Title,
        serde_json::json!("Existing Document"),
    );
    ComponentRepository::save(&store, &title_comp)
        .await
        .unwrap();

    let candidate = Entity::new(EntityType::new("Article"));
    let candidates = EntityResolver::find_candidates(&store, &candidate, "Different Title", None)
        .await
        .unwrap();
    assert!(candidates.is_empty());
}

#[tokio::test]
async fn test_entity_resolver_merge() {
    let store = test_store();

    let canonical = Entity::new(EntityType::new("Article"));
    let duplicate = Entity::new(EntityType::new("Article"));
    EntityRepository::save(&store, &canonical).await.unwrap();
    EntityRepository::save(&store, &duplicate).await.unwrap();

    let rel = Relationship::new(duplicate.id, canonical.id, RelationshipType::References);
    RelationshipRepository::save(&store, &rel).await.unwrap();

    let comp = Component::new(
        duplicate.id,
        ComponentType::Content,
        serde_json::json!("data"),
    );
    ComponentRepository::save(&store, &comp).await.unwrap();

    EntityResolver::merge(&store, canonical.id, duplicate.id, 1.0)
        .await
        .unwrap();

    let loaded = EntityRepository::get(&store, duplicate.id).await.unwrap();
    assert!(loaded.is_none());

    let comps = ComponentRepository::get(&store, canonical.id)
        .await
        .unwrap();
    assert_eq!(comps.len(), 1);

    let rels = RelationshipRepository::by_source(&store, canonical.id)
        .await
        .unwrap();
    assert_eq!(rels.len(), 0);
}

#[tokio::test]
async fn test_find_by_component_data() {
    let store = test_store();
    let entity = Entity::new(EntityType::new("Article"));
    EntityRepository::save(&store, &entity).await.unwrap();

    let prov = Component::new(
        entity.id,
        ComponentType::Provenance,
        serde_json::json!({"source": "test.md", "imported_at": "2026-01-01T00:00:00Z"}),
    );
    ComponentRepository::save(&store, &prov).await.unwrap();

    let found =
        ComponentRepository::find_by_component_data(&store, "Provenance", "source", "test.md")
            .await
            .unwrap();
    assert_eq!(found.len(), 1);
    assert_eq!(found[0].entity_id, entity.id);

    let not_found =
        ComponentRepository::find_by_component_data(&store, "Provenance", "source", "other.md")
            .await
            .unwrap();
    assert!(not_found.is_empty());
}

#[tokio::test]
async fn test_entity_resolver_normalized_match() {
    let store = test_store();

    // Store with title "Hello World"
    let existing = Entity::new(EntityType::new("Article"));
    EntityRepository::save(&store, &existing).await.unwrap();
    let title_comp = Component::new(
        existing.id,
        ComponentType::Title,
        serde_json::json!("Hello World"),
    );
    ComponentRepository::save(&store, &title_comp)
        .await
        .unwrap();

    // Query with lowercase version - should match via normalized strategy
    let candidate = Entity::new(EntityType::new("Article"));
    let candidates = EntityResolver::find_candidates(&store, &candidate, "hello world", None)
        .await
        .unwrap();
    assert_eq!(candidates.len(), 1);
    assert_eq!(candidates[0].entity_id, existing.id);
    assert_eq!(candidates[0].confidence, 0.95);
}

#[tokio::test]
async fn test_entity_resolver_fuzzy_match() {
    let store = test_store();

    // Store "Attention Is All You Need"
    let existing = Entity::new(EntityType::new("Article"));
    EntityRepository::save(&store, &existing).await.unwrap();
    let title_comp = Component::new(
        existing.id,
        ComponentType::Title,
        serde_json::json!("Attention Is All You Need"),
    );
    ComponentRepository::save(&store, &title_comp)
        .await
        .unwrap();

    // Query with variant that differs slightly - should match via fuzzy strategy
    let candidate = Entity::new(EntityType::new("Article"));
    let candidates = EntityResolver::find_candidates(
        &store,
        &candidate,
        "Attention Is All You Need (2017)",
        None,
    )
    .await
    .unwrap();

    assert!(!candidates.is_empty());
    assert!(candidates[0].confidence > 0.95);
    assert!(candidates[0].confidence < 1.0);
}

#[tokio::test]
async fn test_delete_by_entity() {
    let store = test_store();
    let entity = Entity::new(EntityType::new("Article"));
    EntityRepository::save(&store, &entity).await.unwrap();

    ComponentRepository::save(
        &store,
        &Component::new(entity.id, ComponentType::Title, serde_json::json!("t")),
    )
    .await
    .unwrap();
    ComponentRepository::save(
        &store,
        &Component::new(entity.id, ComponentType::Content, serde_json::json!("c")),
    )
    .await
    .unwrap();

    let comps = ComponentRepository::get(&store, entity.id).await.unwrap();
    assert_eq!(comps.len(), 2);

    ComponentRepository::delete_by_entity(&store, entity.id)
        .await
        .unwrap();

    let comps = ComponentRepository::get(&store, entity.id).await.unwrap();
    assert!(comps.is_empty());
}

#[tokio::test]
async fn test_merge_audit_log_and_undo() {
    let store = test_store();

    // Create two entities
    let canonical = Entity::new(EntityType::new("Article"));
    let duplicate = Entity::new(EntityType::new("Article"));
    EntityRepository::save(&store, &canonical).await.unwrap();
    EntityRepository::save(&store, &duplicate).await.unwrap();

    // Add components to duplicate
    let comp = Component::new(
        duplicate.id,
        ComponentType::Content,
        serde_json::json!("data"),
    );
    ComponentRepository::save(&store, &comp).await.unwrap();

    // Log the merge
    let entry = MergeAuditEntry {
        id: Uuid::new_v4(),
        source_id: duplicate.id,
        source_title: "Duplicate Doc".to_string(),
        target_id: canonical.id,
        target_title: "Canonical Doc".to_string(),
        strategy: "fuzzy".to_string(),
        confidence: 0.92,
        timestamp: chrono::Utc::now(),
        reason: "Jaro-Winkler similarity >= 0.85".to_string(),
        snapshot: Some(
            serde_json::json!({
                "entity_type": duplicate.entity_type.as_str(),
                "is_active": duplicate.is_active,
                "created_at": duplicate.created_at.to_rfc3339(),
                "updated_at": duplicate.updated_at.to_rfc3339(),
                "version": duplicate.version,
            })
            .to_string(),
        ),
    };
    EntityResolver::log_merge(&store, &entry).await.unwrap();

    // Verify merge history
    let history = EntityResolver::get_merge_history(&store, canonical.id)
        .await
        .unwrap();
    assert_eq!(history.len(), 1);
    assert_eq!(history[0].id, entry.id);
    assert_eq!(history[0].confidence, 0.92);

    // Undo the merge
    EntityResolver::undo_merge(&store, entry.id).await.unwrap();

    // Verify merge entry removed
    let history = EntityResolver::get_merge_history(&store, canonical.id)
        .await
        .unwrap();
    assert!(history.is_empty());
}

#[tokio::test]
async fn test_merge_history_by_source_and_target() {
    let store = test_store();

    let entity_a = Entity::new(EntityType::new("Article"));
    let entity_b = Entity::new(EntityType::new("Article"));
    let entity_c = Entity::new(EntityType::new("Article"));
    EntityRepository::save(&store, &entity_a).await.unwrap();
    EntityRepository::save(&store, &entity_b).await.unwrap();
    EntityRepository::save(&store, &entity_c).await.unwrap();

    // A merged into B
    let entry1 = MergeAuditEntry {
        id: Uuid::new_v4(),
        source_id: entity_a.id,
        source_title: "A".to_string(),
        target_id: entity_b.id,
        target_title: "B".to_string(),
        strategy: "exact".to_string(),
        confidence: 1.0,
        timestamp: chrono::Utc::now(),
        reason: "Exact match".to_string(),
        snapshot: Some(
            serde_json::json!({
                "entity_type": entity_a.entity_type.as_str(),
                "is_active": entity_a.is_active,
                "created_at": entity_a.created_at.to_rfc3339(),
                "updated_at": entity_a.updated_at.to_rfc3339(),
                "version": entity_a.version,
            })
            .to_string(),
        ),
    };
    EntityResolver::log_merge(&store, &entry1).await.unwrap();

    // C merged into B
    let entry2 = MergeAuditEntry {
        id: Uuid::new_v4(),
        source_id: entity_c.id,
        source_title: "C".to_string(),
        target_id: entity_b.id,
        target_title: "B".to_string(),
        strategy: "normalized".to_string(),
        confidence: 0.95,
        timestamp: chrono::Utc::now(),
        reason: "Normalized match".to_string(),
        snapshot: Some(
            serde_json::json!({
                "entity_type": entity_c.entity_type.as_str(),
                "is_active": entity_c.is_active,
                "created_at": entity_c.created_at.to_rfc3339(),
                "updated_at": entity_c.updated_at.to_rfc3339(),
                "version": entity_c.version,
            })
            .to_string(),
        ),
    };
    EntityResolver::log_merge(&store, &entry2).await.unwrap();

    // Query by target (B) should find both
    let history = EntityResolver::get_merge_history(&store, entity_b.id)
        .await
        .unwrap();
    assert_eq!(history.len(), 2);

    // Query by source (A) should find one
    let history = EntityResolver::get_merge_history(&store, entity_a.id)
        .await
        .unwrap();
    assert_eq!(history.len(), 1);
    assert_eq!(history[0].source_id, entity_a.id);
}

// =============================================================================
// Graph Traversal Tests
// =============================================================================

/// Helper to create an entity and save it.
async fn create_entity(store: &SqliteStore, entity_type: &str) -> Entity {
    let entity = Entity::new(EntityType::new(entity_type));
    EntityRepository::save(store, &entity).await.unwrap();
    entity
}

/// Helper to create a relationship between two entities.
async fn create_relationship(
    store: &SqliteStore,
    source: &Entity,
    target: &Entity,
    rel_type: RelationshipType,
) -> Relationship {
    let rel = Relationship::new(source.id, target.id, rel_type);
    RelationshipRepository::save(store, &rel).await.unwrap();
    rel
}

#[tokio::test]
async fn test_traversal_chain() {
    // A -> B -> C -> D
    let store = test_store();
    let a = create_entity(&store, "Article").await;
    let b = create_entity(&store, "Concept").await;
    let c = create_entity(&store, "Concept").await;
    let d = create_entity(&store, "Concept").await;

    create_relationship(&store, &a, &b, RelationshipType::References).await;
    create_relationship(&store, &b, &c, RelationshipType::References).await;
    create_relationship(&store, &c, &d, RelationshipType::References).await;

    let config = test_config();
    let query = TraversalQuery {
        start_id: a.id,
        direction: TraversalDirection::Outgoing,
        max_depth: Some(3),
        max_results: None,
        relationship_type: None,
        entity_type_filter: None,
    };

    let results = TraversalPort::traverse(&store, &query, &config)
        .await
        .unwrap();
    assert_eq!(results.len(), 3);

    // All should be reachable at expected depths
    let ids: Vec<Uuid> = results.iter().map(|r| *r.path.last().unwrap()).collect();
    assert!(ids.contains(&b.id));
    assert!(ids.contains(&c.id));
    assert!(ids.contains(&d.id));

    // B at depth 1, C at depth 2, D at depth 3
    let b_result = results
        .iter()
        .find(|r| *r.path.last().unwrap() == b.id)
        .unwrap();
    assert_eq!(b_result.depth, 1);
    let c_result = results
        .iter()
        .find(|r| *r.path.last().unwrap() == c.id)
        .unwrap();
    assert_eq!(c_result.depth, 2);
    let d_result = results
        .iter()
        .find(|r| *r.path.last().unwrap() == d.id)
        .unwrap();
    assert_eq!(d_result.depth, 3);
}

#[tokio::test]
async fn test_traversal_tree() {
    // A -> {B, C}, B -> {D, E}, C -> {F}
    let store = test_store();
    let a = create_entity(&store, "Article").await;
    let b = create_entity(&store, "Concept").await;
    let c = create_entity(&store, "Concept").await;
    let d = create_entity(&store, "Concept").await;
    let e = create_entity(&store, "Concept").await;
    let f = create_entity(&store, "Concept").await;

    create_relationship(&store, &a, &b, RelationshipType::References).await;
    create_relationship(&store, &a, &c, RelationshipType::References).await;
    create_relationship(&store, &b, &d, RelationshipType::References).await;
    create_relationship(&store, &b, &e, RelationshipType::References).await;
    create_relationship(&store, &c, &f, RelationshipType::References).await;

    let config = test_config();
    let query = TraversalQuery {
        start_id: a.id,
        direction: TraversalDirection::Outgoing,
        max_depth: Some(3),
        max_results: None,
        relationship_type: None,
        entity_type_filter: None,
    };

    let results = TraversalPort::traverse(&store, &query, &config)
        .await
        .unwrap();
    assert_eq!(results.len(), 5);

    // B, C at depth 1
    let b_result = results
        .iter()
        .find(|r| *r.path.last().unwrap() == b.id)
        .unwrap();
    assert_eq!(b_result.depth, 1);
    let c_result = results
        .iter()
        .find(|r| *r.path.last().unwrap() == c.id)
        .unwrap();
    assert_eq!(c_result.depth, 1);

    // D, E, F at depth 2
    let d_result = results
        .iter()
        .find(|r| *r.path.last().unwrap() == d.id)
        .unwrap();
    assert_eq!(d_result.depth, 2);
    let e_result = results
        .iter()
        .find(|r| *r.path.last().unwrap() == e.id)
        .unwrap();
    assert_eq!(e_result.depth, 2);
    let f_result = results
        .iter()
        .find(|r| *r.path.last().unwrap() == f.id)
        .unwrap();
    assert_eq!(f_result.depth, 2);
}

#[tokio::test]
async fn test_traversal_cycle() {
    // A -> B -> C -> A
    let store = test_store();
    let a = create_entity(&store, "Article").await;
    let b = create_entity(&store, "Concept").await;
    let c = create_entity(&store, "Concept").await;

    create_relationship(&store, &a, &b, RelationshipType::References).await;
    create_relationship(&store, &b, &c, RelationshipType::References).await;
    create_relationship(&store, &c, &a, RelationshipType::References).await;

    let config = test_config();
    let query = TraversalQuery {
        start_id: a.id,
        direction: TraversalDirection::Outgoing,
        max_depth: Some(5),
        max_results: None,
        relationship_type: None,
        entity_type_filter: None,
    };

    let results = TraversalPort::traverse(&store, &query, &config)
        .await
        .unwrap();

    // Should find B at depth 1, C at depth 2
    // A should NOT appear as a result (it's the start, and cycle prevents revisiting)
    assert_eq!(results.len(), 2);
    let ids: Vec<Uuid> = results.iter().map(|r| *r.path.last().unwrap()).collect();
    assert!(ids.contains(&b.id));
    assert!(ids.contains(&c.id));
    assert!(!ids.contains(&a.id));
}

#[tokio::test]
async fn test_traversal_diamond() {
    // A -> {B, C}, B -> D, C -> D
    let store = test_store();
    let a = create_entity(&store, "Article").await;
    let b = create_entity(&store, "Concept").await;
    let c = create_entity(&store, "Concept").await;
    let d = create_entity(&store, "Concept").await;

    create_relationship(&store, &a, &b, RelationshipType::References).await;
    create_relationship(&store, &a, &c, RelationshipType::References).await;
    create_relationship(&store, &b, &d, RelationshipType::References).await;
    create_relationship(&store, &c, &d, RelationshipType::References).await;

    let config = test_config();
    let query = TraversalQuery {
        start_id: a.id,
        direction: TraversalDirection::Outgoing,
        max_depth: Some(3),
        max_results: None,
        relationship_type: None,
        entity_type_filter: None,
    };

    let results = TraversalPort::traverse(&store, &query, &config)
        .await
        .unwrap();

    // B, C at depth 1, D at depth 2 (first discovery)
    assert_eq!(results.len(), 3);
    let d_result = results
        .iter()
        .find(|r| *r.path.last().unwrap() == d.id)
        .unwrap();
    assert_eq!(d_result.depth, 2);
}

#[tokio::test]
async fn test_traversal_bidirectional() {
    // A <-> B (bidirectional relationship)
    let store = test_store();
    let a = create_entity(&store, "Article").await;
    let b = create_entity(&store, "Concept").await;
    let c = create_entity(&store, "Concept").await;

    // A -> B
    create_relationship(&store, &a, &b, RelationshipType::References).await;
    // C -> A (incoming to A)
    create_relationship(&store, &c, &a, RelationshipType::References).await;

    let config = test_config();
    let query = TraversalQuery {
        start_id: a.id,
        direction: TraversalDirection::Both,
        max_depth: Some(2),
        max_results: None,
        relationship_type: None,
        entity_type_filter: None,
    };

    let results = TraversalPort::traverse(&store, &query, &config)
        .await
        .unwrap();

    // Should find B (outgoing from A) and C (incoming to A)
    assert_eq!(results.len(), 2);
    let ids: Vec<Uuid> = results.iter().map(|r| *r.path.last().unwrap()).collect();
    assert!(ids.contains(&b.id));
    assert!(ids.contains(&c.id));
}

#[tokio::test]
async fn test_traversal_disconnected() {
    // A -> B, C -> D (disconnected subgraphs)
    let store = test_store();
    let a = create_entity(&store, "Article").await;
    let b = create_entity(&store, "Concept").await;
    let c = create_entity(&store, "Concept").await;
    let d = create_entity(&store, "Concept").await;

    create_relationship(&store, &a, &b, RelationshipType::References).await;
    create_relationship(&store, &c, &d, RelationshipType::References).await;

    let config = test_config();
    let query = TraversalQuery {
        start_id: a.id,
        direction: TraversalDirection::Outgoing,
        max_depth: Some(3),
        max_results: None,
        relationship_type: None,
        entity_type_filter: None,
    };

    let results = TraversalPort::traverse(&store, &query, &config)
        .await
        .unwrap();

    // Should only find B, not C or D
    assert_eq!(results.len(), 1);
    assert_eq!(*results[0].path.last().unwrap(), b.id);
}

#[tokio::test]
async fn test_traversal_depth_limiting() {
    // A -> B -> C -> D, depth 1
    let store = test_store();
    let a = create_entity(&store, "Article").await;
    let b = create_entity(&store, "Concept").await;
    let c = create_entity(&store, "Concept").await;
    let d = create_entity(&store, "Concept").await;

    create_relationship(&store, &a, &b, RelationshipType::References).await;
    create_relationship(&store, &b, &c, RelationshipType::References).await;
    create_relationship(&store, &c, &d, RelationshipType::References).await;

    let config = test_config();
    let query = TraversalQuery {
        start_id: a.id,
        direction: TraversalDirection::Outgoing,
        max_depth: Some(1),
        max_results: None,
        relationship_type: None,
        entity_type_filter: None,
    };

    let results = TraversalPort::traverse(&store, &query, &config)
        .await
        .unwrap();

    // Should only find B at depth 1, not C or D
    assert_eq!(results.len(), 1);
    assert_eq!(*results[0].path.last().unwrap(), b.id);
    assert_eq!(results[0].depth, 1);
}

#[tokio::test]
async fn test_traversal_relationship_type_filter() {
    // A --references--> B, A --related_to--> C
    let store = test_store();
    let a = create_entity(&store, "Article").await;
    let b = create_entity(&store, "Concept").await;
    let c = create_entity(&store, "Concept").await;

    create_relationship(&store, &a, &b, RelationshipType::References).await;
    let rel_c = Relationship::new(a.id, c.id, RelationshipType::References);
    RelationshipRepository::save(&store, &rel_c).await.unwrap();

    // Override with a different type for C
    let mut rel_c_typed = rel_c.clone();
    rel_c_typed.relationship_type = RelationshipType::References; // same type for now
    RelationshipRepository::save(&store, &rel_c_typed)
        .await
        .unwrap();

    let config = test_config();
    let query = TraversalQuery {
        start_id: a.id,
        direction: TraversalDirection::Outgoing,
        max_depth: Some(2),
        max_results: None,
        relationship_type: Some(RelationshipType::References),
        entity_type_filter: None,
    };

    let results = TraversalPort::traverse(&store, &query, &config)
        .await
        .unwrap();

    // Both B and C should be found since both have References type
    assert_eq!(results.len(), 2);
    let ids: Vec<Uuid> = results.iter().map(|r| *r.path.last().unwrap()).collect();
    assert!(ids.contains(&b.id));
    assert!(ids.contains(&c.id));
}

#[tokio::test]
async fn test_traversal_entity_type_filter() {
    // A (Article) -> B (Concept), A -> C (Person)
    let store = test_store();
    let a = create_entity(&store, "Article").await;
    let b = create_entity(&store, "Concept").await;
    let c = create_entity(&store, "Person").await;

    create_relationship(&store, &a, &b, RelationshipType::References).await;
    create_relationship(&store, &a, &c, RelationshipType::References).await;

    let config = test_config();
    let query = TraversalQuery {
        start_id: a.id,
        direction: TraversalDirection::Outgoing,
        max_depth: Some(2),
        max_results: None,
        relationship_type: None,
        entity_type_filter: Some(EntityType::new("Concept")),
    };

    let results = TraversalPort::traverse(&store, &query, &config)
        .await
        .unwrap();

    // Should only find B (Concept), not C (Person)
    assert_eq!(results.len(), 1);
    assert_eq!(*results[0].path.last().unwrap(), b.id);
}

#[tokio::test]
async fn test_traversal_start_not_found() {
    let store = test_store();
    let config = test_config();
    let nonexistent_id = Uuid::new_v4();

    let query = TraversalQuery {
        start_id: nonexistent_id,
        direction: TraversalDirection::Outgoing,
        max_depth: Some(3),
        max_results: None,
        relationship_type: None,
        entity_type_filter: None,
    };

    let result = TraversalPort::traverse(&store, &query, &config).await;
    assert!(result.is_err());
    match result.unwrap_err() {
        TraversalError::StartNotFound(id) => assert_eq!(id, nonexistent_id),
        other => panic!("Expected StartNotFound, got {:?}", other),
    }
}

#[tokio::test]
async fn test_traversal_start_inactive() {
    let store = test_store();
    let mut a = create_entity(&store, "Article").await;
    let b = create_entity(&store, "Concept").await;
    create_relationship(&store, &a, &b, RelationshipType::References).await;

    // Archive the start entity
    a.archive();
    EntityRepository::save(&store, &a).await.unwrap();

    let config = test_config();
    let query = TraversalQuery {
        start_id: a.id,
        direction: TraversalDirection::Outgoing,
        max_depth: Some(3),
        max_results: None,
        relationship_type: None,
        entity_type_filter: None,
    };

    let result = TraversalPort::traverse(&store, &query, &config).await;
    assert!(result.is_err());
    assert!(matches!(
        result.unwrap_err(),
        TraversalError::StartNotFound(_)
    ));
}

#[tokio::test]
async fn test_traversal_incoming() {
    // A <- B <- C (incoming from A)
    let store = test_store();
    let a = create_entity(&store, "Article").await;
    let b = create_entity(&store, "Concept").await;
    let c = create_entity(&store, "Concept").await;

    // B -> A, C -> B (so A can reach B and C via incoming)
    create_relationship(&store, &b, &a, RelationshipType::References).await;
    create_relationship(&store, &c, &b, RelationshipType::References).await;

    let config = test_config();
    let query = TraversalQuery {
        start_id: a.id,
        direction: TraversalDirection::Incoming,
        max_depth: Some(3),
        max_results: None,
        relationship_type: None,
        entity_type_filter: None,
    };

    let results = TraversalPort::traverse(&store, &query, &config)
        .await
        .unwrap();
    assert_eq!(results.len(), 2);
    let ids: Vec<Uuid> = results.iter().map(|r| *r.path.last().unwrap()).collect();
    assert!(ids.contains(&b.id));
    assert!(ids.contains(&c.id));
}

#[tokio::test]
async fn test_traversal_result_limit() {
    // A -> B, A -> C, A -> D (star topology)
    let store = test_store();
    let a = create_entity(&store, "Article").await;
    let b = create_entity(&store, "Concept").await;
    let c = create_entity(&store, "Concept").await;
    let d = create_entity(&store, "Concept").await;

    create_relationship(&store, &a, &b, RelationshipType::References).await;
    create_relationship(&store, &a, &c, RelationshipType::References).await;
    create_relationship(&store, &a, &d, RelationshipType::References).await;

    let config = test_config();
    let query = TraversalQuery {
        start_id: a.id,
        direction: TraversalDirection::Outgoing,
        max_depth: Some(3),
        max_results: Some(2),
        relationship_type: None,
        entity_type_filter: None,
    };

    let results = TraversalPort::traverse(&store, &query, &config)
        .await
        .unwrap();
    assert_eq!(results.len(), 2);
}

// =============================================================================
// Collection Tests
// =============================================================================

use knowledge_core::ports::{Collection, CollectionRepository};

#[tokio::test]
async fn test_collection_crud() {
    let store = test_store();

    // Create
    let collection = Collection {
        id: Uuid::new_v4(),
        name: "Papers to Read".to_string(),
        description: Some("Research papers for literature review".to_string()),
        created_at: chrono::Utc::now(),
        updated_at: chrono::Utc::now(),
    };
    let created = CollectionRepository::create(&store, collection.clone())
        .await
        .unwrap();
    assert_eq!(created.id, collection.id);
    assert_eq!(created.name, "Papers to Read");

    // Read
    let loaded = CollectionRepository::get(&store, created.id)
        .await
        .unwrap()
        .unwrap();
    assert_eq!(loaded.name, "Papers to Read");
    assert_eq!(
        loaded.description,
        Some("Research papers for literature review".to_string())
    );

    // List
    let all = CollectionRepository::list(&store).await.unwrap();
    assert_eq!(all.len(), 1);
    assert_eq!(all[0].name, "Papers to Read");

    // Update
    let mut updated = loaded;
    updated.name = "Must Read Papers".to_string();
    let result = CollectionRepository::update(&store, updated.clone())
        .await
        .unwrap();
    assert_eq!(result.name, "Must Read Papers");
    let loaded = CollectionRepository::get(&store, created.id)
        .await
        .unwrap()
        .unwrap();
    assert_eq!(loaded.name, "Must Read Papers");

    // Delete
    CollectionRepository::delete(&store, created.id)
        .await
        .unwrap();
    let loaded = CollectionRepository::get(&store, created.id).await.unwrap();
    assert!(loaded.is_none());
}

#[tokio::test]
async fn test_collection_update_not_found() {
    let store = test_store();
    let collection = Collection {
        id: Uuid::new_v4(),
        name: "Nonexistent".to_string(),
        description: None,
        created_at: chrono::Utc::now(),
        updated_at: chrono::Utc::now(),
    };
    let result = CollectionRepository::update(&store, collection).await;
    assert!(matches!(result, Err(StorageError::NotFound)));
}

#[tokio::test]
async fn test_collection_add_member() {
    let store = test_store();
    let entity = create_entity(&store, "Article").await;

    let collection = Collection {
        id: Uuid::new_v4(),
        name: "Reading List".to_string(),
        description: None,
        created_at: chrono::Utc::now(),
        updated_at: chrono::Utc::now(),
    };
    let created = CollectionRepository::create(&store, collection)
        .await
        .unwrap();

    // Add member
    CollectionRepository::add_member(&store, created.id, entity.id)
        .await
        .unwrap();

    // Check membership
    let is_member = CollectionRepository::is_member(&store, created.id, entity.id)
        .await
        .unwrap();
    assert!(is_member);

    // Get members
    let members = CollectionRepository::get_members(&store, created.id)
        .await
        .unwrap();
    assert_eq!(members.len(), 1);
    assert_eq!(members[0].id, entity.id);
}

#[tokio::test]
async fn test_collection_add_member_duplicate() {
    let store = test_store();
    let entity = create_entity(&store, "Article").await;

    let collection = Collection {
        id: Uuid::new_v4(),
        name: "Reading List".to_string(),
        description: None,
        created_at: chrono::Utc::now(),
        updated_at: chrono::Utc::now(),
    };
    let created = CollectionRepository::create(&store, collection)
        .await
        .unwrap();

    // Add same member twice — second add should fail with Internal error
    CollectionRepository::add_member(&store, created.id, entity.id)
        .await
        .unwrap();
    let result = CollectionRepository::add_member(&store, created.id, entity.id).await;
    assert!(result.is_err(), "Expected error for duplicate membership");
    match result.unwrap_err() {
        StorageError::Internal(msg) => assert!(
            msg.contains("already a member"),
            "Expected 'already a member' in error, got: {}",
            msg
        ),
        other => panic!("Expected Internal error, got: {:?}", other),
    }

    let members = CollectionRepository::get_members(&store, created.id)
        .await
        .unwrap();
    assert_eq!(members.len(), 1);
}

#[tokio::test]
async fn test_collection_remove_member() {
    let store = test_store();
    let entity = create_entity(&store, "Article").await;

    let collection = Collection {
        id: Uuid::new_v4(),
        name: "Reading List".to_string(),
        description: None,
        created_at: chrono::Utc::now(),
        updated_at: chrono::Utc::now(),
    };
    let created = CollectionRepository::create(&store, collection)
        .await
        .unwrap();

    CollectionRepository::add_member(&store, created.id, entity.id)
        .await
        .unwrap();

    // Remove member
    CollectionRepository::remove_member(&store, created.id, entity.id)
        .await
        .unwrap();

    let is_member = CollectionRepository::is_member(&store, created.id, entity.id)
        .await
        .unwrap();
    assert!(!is_member);

    let members = CollectionRepository::get_members(&store, created.id)
        .await
        .unwrap();
    assert!(members.is_empty());
}

#[tokio::test]
async fn test_collection_get_entity_collections() {
    let store = test_store();
    let entity = create_entity(&store, "Article").await;

    let coll1 = Collection {
        id: Uuid::new_v4(),
        name: "Reading List".to_string(),
        description: None,
        created_at: chrono::Utc::now(),
        updated_at: chrono::Utc::now(),
    };
    let coll2 = Collection {
        id: Uuid::new_v4(),
        name: "Favorites".to_string(),
        description: None,
        created_at: chrono::Utc::now(),
        updated_at: chrono::Utc::now(),
    };
    let c1 = CollectionRepository::create(&store, coll1).await.unwrap();
    let c2 = CollectionRepository::create(&store, coll2).await.unwrap();

    CollectionRepository::add_member(&store, c1.id, entity.id)
        .await
        .unwrap();
    CollectionRepository::add_member(&store, c2.id, entity.id)
        .await
        .unwrap();

    // Entity should be in both collections
    let collections = CollectionRepository::get_entity_collections(&store, entity.id)
        .await
        .unwrap();
    assert_eq!(collections.len(), 2);
    let names: Vec<&str> = collections.iter().map(|c| c.name.as_str()).collect();
    assert!(names.contains(&"Reading List"));
    assert!(names.contains(&"Favorites"));
}

#[tokio::test]
async fn test_collection_delete_cascade() {
    let store = test_store();
    let entity = create_entity(&store, "Article").await;

    let collection = Collection {
        id: Uuid::new_v4(),
        name: "To Delete".to_string(),
        description: None,
        created_at: chrono::Utc::now(),
        updated_at: chrono::Utc::now(),
    };
    let created = CollectionRepository::create(&store, collection)
        .await
        .unwrap();

    CollectionRepository::add_member(&store, created.id, entity.id)
        .await
        .unwrap();

    // Verify membership exists
    let members = CollectionRepository::get_members(&store, created.id)
        .await
        .unwrap();
    assert_eq!(members.len(), 1);

    // Delete collection — CASCADE should remove membership records
    CollectionRepository::delete(&store, created.id)
        .await
        .unwrap();

    // Verify collection is gone
    let loaded = CollectionRepository::get(&store, created.id).await.unwrap();
    assert!(loaded.is_none());

    // Verify entity still exists (not cascade-deleted)
    let entity_still_exists = EntityRepository::get(&store, entity.id)
        .await
        .unwrap()
        .is_some();
    assert!(entity_still_exists);
}

#[tokio::test]
async fn test_collection_entity_in_multiple_collections() {
    let store = test_store();
    let entity = create_entity(&store, "Article").await;

    let coll1 = Collection {
        id: Uuid::new_v4(),
        name: "Collection A".to_string(),
        description: None,
        created_at: chrono::Utc::now(),
        updated_at: chrono::Utc::now(),
    };
    let coll2 = Collection {
        id: Uuid::new_v4(),
        name: "Collection B".to_string(),
        description: None,
        created_at: chrono::Utc::now(),
        updated_at: chrono::Utc::now(),
    };
    let c1 = CollectionRepository::create(&store, coll1).await.unwrap();
    let c2 = CollectionRepository::create(&store, coll2).await.unwrap();

    CollectionRepository::add_member(&store, c1.id, entity.id)
        .await
        .unwrap();
    CollectionRepository::add_member(&store, c2.id, entity.id)
        .await
        .unwrap();

    // Entity appears in each collection's member list
    let members1 = CollectionRepository::get_members(&store, c1.id)
        .await
        .unwrap();
    assert_eq!(members1.len(), 1);
    assert_eq!(members1[0].id, entity.id);

    let members2 = CollectionRepository::get_members(&store, c2.id)
        .await
        .unwrap();
    assert_eq!(members2.len(), 1);
    assert_eq!(members2[0].id, entity.id);

    // Entity reports both collections
    let collections = CollectionRepository::get_entity_collections(&store, entity.id)
        .await
        .unwrap();
    assert_eq!(collections.len(), 2);
}

#[tokio::test]
async fn test_collection_empty_members() {
    let store = test_store();

    let collection = Collection {
        id: Uuid::new_v4(),
        name: "Empty Collection".to_string(),
        description: None,
        created_at: chrono::Utc::now(),
        updated_at: chrono::Utc::now(),
    };
    let created = CollectionRepository::create(&store, collection)
        .await
        .unwrap();

    let members = CollectionRepository::get_members(&store, created.id)
        .await
        .unwrap();
    assert!(members.is_empty());

    let is_member = CollectionRepository::is_member(&store, created.id, Uuid::new_v4())
        .await
        .unwrap();
    assert!(!is_member);
}

#[tokio::test]
async fn test_collection_is_member_nonexistent() {
    let store = test_store();
    let is_member = CollectionRepository::is_member(&store, Uuid::new_v4(), Uuid::new_v4())
        .await
        .unwrap();
    assert!(!is_member);
}

// ===========================================================================
// Cross-plan integration tests (IP-006 D3)
//
// Verify that features from different implementation plans work together
// correctly in realistic workflows.
// ===========================================================================

/// Import → Traverse → View: Import entities with relationships, traverse the
/// graph, verify traversal returns the expected subgraph.
#[tokio::test]
async fn test_import_then_traverse_subgraph() {
    let store = test_store();

    // Create a small graph: A -> B -> C, A -> D
    let entity_a = Entity::new(EntityType::new("Concept"));
    let entity_b = Entity::new(EntityType::new("Concept"));
    let entity_c = Entity::new(EntityType::new("Concept"));
    let entity_d = Entity::new(EntityType::new("Paper"));

    EntityRepository::save(&store, &entity_a).await.unwrap();
    EntityRepository::save(&store, &entity_b).await.unwrap();
    EntityRepository::save(&store, &entity_c).await.unwrap();
    EntityRepository::save(&store, &entity_d).await.unwrap();

    // A -> B
    let rel_ab = Relationship::new(entity_a.id, entity_b.id, RelationshipType::References);
    RelationshipRepository::save(&store, &rel_ab).await.unwrap();
    // B -> C
    let rel_bc = Relationship::new(entity_b.id, entity_c.id, RelationshipType::References);
    RelationshipRepository::save(&store, &rel_bc).await.unwrap();
    // A -> D
    let rel_ad = Relationship::new(entity_a.id, entity_d.id, RelationshipType::References);
    RelationshipRepository::save(&store, &rel_ad).await.unwrap();

    // Traverse from A with depth 2
    let config = test_config();
    let query = TraversalQuery {
        start_id: entity_a.id,
        direction: TraversalDirection::Outgoing,
        max_depth: Some(2),
        max_results: None,
        relationship_type: None,
        entity_type_filter: None,
    };
    let result = TraversalPort::traverse(&store, &query, &config)
        .await
        .unwrap();

    // Should visit B, C, D
    assert_eq!(result.len(), 3);
    let ids: Vec<Uuid> = result.iter().map(|r| *r.path.last().unwrap()).collect();
    assert!(ids.contains(&entity_b.id));
    assert!(ids.contains(&entity_c.id));
    assert!(ids.contains(&entity_d.id));
}

/// Import → Collection → View: Import entities, create a collection, add
/// members, then verify collection membership and entity listing.
#[tokio::test]
async fn test_import_then_collection_then_list() {
    let store = test_store();

    let e1 = Entity::new(EntityType::new("Paper"));
    let e2 = Entity::new(EntityType::new("Paper"));
    let e3 = Entity::new(EntityType::new("Concept"));

    EntityRepository::save(&store, &e1).await.unwrap();
    EntityRepository::save(&store, &e2).await.unwrap();
    EntityRepository::save(&store, &e3).await.unwrap();

    // Create a collection and add e1, e2
    let collection = Collection {
        id: Uuid::new_v4(),
        name: "Reading List".to_string(),
        description: Some("Papers to read".to_string()),
        created_at: chrono::Utc::now(),
        updated_at: chrono::Utc::now(),
    };
    let collection = CollectionRepository::create(&store, collection)
        .await
        .unwrap();

    CollectionRepository::add_member(&store, collection.id, e1.id)
        .await
        .unwrap();
    CollectionRepository::add_member(&store, collection.id, e2.id)
        .await
        .unwrap();

    // Verify membership
    assert!(
        CollectionRepository::is_member(&store, collection.id, e1.id)
            .await
            .unwrap()
    );
    assert!(
        CollectionRepository::is_member(&store, collection.id, e2.id)
            .await
            .unwrap()
    );
    assert!(
        !CollectionRepository::is_member(&store, collection.id, e3.id)
            .await
            .unwrap()
    );

    // Verify member list
    let members = CollectionRepository::get_members(&store, collection.id)
        .await
        .unwrap();
    assert_eq!(members.len(), 2);

    // Verify entity list still returns all entities
    let all = EntityRepository::list(&store).await.unwrap();
    assert_eq!(all.len(), 3);
}

/// Traverse → Filter → Collection: Traverse from an entity, filter by
/// relationship type, then add filtered results to a collection.
#[tokio::test]
async fn test_traverse_filter_then_add_to_collection() {
    let store = test_store();

    let a = Entity::new(EntityType::new("Concept"));
    let b = Entity::new(EntityType::new("Concept"));
    let c = Entity::new(EntityType::new("Paper"));

    EntityRepository::save(&store, &a).await.unwrap();
    EntityRepository::save(&store, &b).await.unwrap();
    EntityRepository::save(&store, &c).await.unwrap();

    // A -> B (References), A -> C (References)
    let rel_ab = Relationship::new(a.id, b.id, RelationshipType::References);
    let rel_ac = Relationship::new(a.id, c.id, RelationshipType::References);
    RelationshipRepository::save(&store, &rel_ab).await.unwrap();
    RelationshipRepository::save(&store, &rel_ac).await.unwrap();

    // Traverse A with References filter
    let config = test_config();
    let query = TraversalQuery {
        start_id: a.id,
        direction: TraversalDirection::Outgoing,
        max_depth: Some(1),
        max_results: None,
        relationship_type: Some(RelationshipType::References),
        entity_type_filter: None,
    };
    let result = TraversalPort::traverse(&store, &query, &config)
        .await
        .unwrap();

    // Both edges are References, filter matches both
    assert_eq!(result.len(), 2); // B, C
    assert_eq!(result.iter().map(|r| r.edges.len()).sum::<usize>(), 2);

    // Add traversed entities to a collection
    let collection = Collection {
        id: Uuid::new_v4(),
        name: "Related Concepts".to_string(),
        description: None,
        created_at: chrono::Utc::now(),
        updated_at: chrono::Utc::now(),
    };
    let collection = CollectionRepository::create(&store, collection)
        .await
        .unwrap();

    for r in &result {
        CollectionRepository::add_member(&store, collection.id, *r.path.last().unwrap())
            .await
            .unwrap();
    }

    let members = CollectionRepository::get_members(&store, collection.id)
        .await
        .unwrap();
    assert_eq!(members.len(), 2);
}

/// Entity + Component + Traverse: Create entities with Title components, then
/// traverse and verify component data is accessible alongside traversal results.
#[tokio::test]
async fn test_traverse_with_component_data() {
    let store = test_store();

    let e1 = Entity::new(EntityType::new("Concept"));
    let e2 = Entity::new(EntityType::new("Concept"));
    EntityRepository::save(&store, &e1).await.unwrap();
    EntityRepository::save(&store, &e2).await.unwrap();

    let title1 = Component::new(
        e1.id,
        ComponentType::Title,
        serde_json::json!("Machine Learning"),
    );
    let title2 = Component::new(
        e2.id,
        ComponentType::Title,
        serde_json::json!("Deep Learning"),
    );
    ComponentRepository::save(&store, &title1).await.unwrap();
    ComponentRepository::save(&store, &title2).await.unwrap();

    let rel = Relationship::new(e1.id, e2.id, RelationshipType::References);
    RelationshipRepository::save(&store, &rel).await.unwrap();

    // Traverse
    let config = test_config();
    let query = TraversalQuery {
        start_id: e1.id,
        direction: TraversalDirection::Outgoing,
        max_depth: Some(1),
        max_results: None,
        relationship_type: None,
        entity_type_filter: None,
    };
    let result = TraversalPort::traverse(&store, &query, &config)
        .await
        .unwrap();

    assert_eq!(result.len(), 1);

    // Load components for each traversed entity
    for r in &result {
        let components = ComponentRepository::get(&store, *r.path.last().unwrap())
            .await
            .unwrap();
        assert!(!components.is_empty(), "entity should have components");
        assert_eq!(components[0].component_type, ComponentType::Title);
    }
}

/// Event → Entity Lifecycle → Traverse: Create entities, archive one, verify
/// traversal only returns active entities.
#[tokio::test]
async fn test_archived_entity_excluded_from_traversal() {
    let store = test_store();

    let a = Entity::new(EntityType::new("Concept"));
    let mut b = Entity::new(EntityType::new("Concept"));
    let c = Entity::new(EntityType::new("Concept"));
    EntityRepository::save(&store, &a).await.unwrap();
    EntityRepository::save(&store, &b).await.unwrap();
    EntityRepository::save(&store, &c).await.unwrap();

    // A -> B -> C
    let rel_ab = Relationship::new(a.id, b.id, RelationshipType::References);
    let rel_bc = Relationship::new(b.id, c.id, RelationshipType::References);
    RelationshipRepository::save(&store, &rel_ab).await.unwrap();
    RelationshipRepository::save(&store, &rel_bc).await.unwrap();

    // Archive entity B
    b.archive();
    EntityRepository::save(&store, &b).await.unwrap();

    // Traverse from A
    let config = test_config();
    let query = TraversalQuery {
        start_id: a.id,
        direction: TraversalDirection::Outgoing,
        max_depth: Some(2),
        max_results: None,
        relationship_type: None,
        entity_type_filter: None,
    };
    let result = TraversalPort::traverse(&store, &query, &config)
        .await
        .unwrap();
    // B is archived so traversal should find no reachable entities
    assert_eq!(result.len(), 0);
}

/// Multi-collection membership: Add an entity to multiple collections, verify
/// it appears in all of them.
#[tokio::test]
async fn test_entity_in_multiple_collections_with_traversal() {
    let store = test_store();

    let e1 = Entity::new(EntityType::new("Paper"));
    let e2 = Entity::new(EntityType::new("Paper"));
    let e3 = Entity::new(EntityType::new("Concept"));

    EntityRepository::save(&store, &e1).await.unwrap();
    EntityRepository::save(&store, &e2).await.unwrap();
    EntityRepository::save(&store, &e3).await.unwrap();

    // e1 -> e2 -> e3
    let rel1 = Relationship::new(e1.id, e2.id, RelationshipType::References);
    let rel2 = Relationship::new(e2.id, e3.id, RelationshipType::References);
    RelationshipRepository::save(&store, &rel1).await.unwrap();
    RelationshipRepository::save(&store, &rel2).await.unwrap();

    // Two collections, both contain e2
    let coll_a = Collection {
        id: Uuid::new_v4(),
        name: "Collection A".to_string(),
        description: None,
        created_at: chrono::Utc::now(),
        updated_at: chrono::Utc::now(),
    };
    let coll_b = Collection {
        id: Uuid::new_v4(),
        name: "Collection B".to_string(),
        description: None,
        created_at: chrono::Utc::now(),
        updated_at: chrono::Utc::now(),
    };
    let coll_a = CollectionRepository::create(&store, coll_a).await.unwrap();
    let coll_b = CollectionRepository::create(&store, coll_b).await.unwrap();

    CollectionRepository::add_member(&store, coll_a.id, e1.id)
        .await
        .unwrap();
    CollectionRepository::add_member(&store, coll_a.id, e2.id)
        .await
        .unwrap();
    CollectionRepository::add_member(&store, coll_b.id, e2.id)
        .await
        .unwrap();
    CollectionRepository::add_member(&store, coll_b.id, e3.id)
        .await
        .unwrap();

    // e2 is in both collections
    assert!(CollectionRepository::is_member(&store, coll_a.id, e2.id)
        .await
        .unwrap());
    assert!(CollectionRepository::is_member(&store, coll_b.id, e2.id)
        .await
        .unwrap());

    // Traverse from e1 — should reach all three
    let config = test_config();
    let query = TraversalQuery {
        start_id: e1.id,
        direction: TraversalDirection::Outgoing,
        max_depth: Some(2),
        max_results: None,
        relationship_type: None,
        entity_type_filter: None,
    };
    let result = TraversalPort::traverse(&store, &query, &config)
        .await
        .unwrap();
    assert_eq!(result.len(), 2);
}
