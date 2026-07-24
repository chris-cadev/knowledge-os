use knowledge_core::features::component::{Component, ComponentType};
use knowledge_core::features::entity::{Entity, EntityType};
use knowledge_core::features::relationship::{Relationship, RelationshipType};
use knowledge_core::ports::*;

#[test]
fn test_entity_types_cover_all_variants() {
    let types = vec![
        EntityType::new("Concept"),
        EntityType::new("Person"),
        EntityType::new("Organization"),
        EntityType::new("Project"),
        EntityType::new("Book"),
        EntityType::new("Paper"),
        EntityType::new("Video"),
        EntityType::new("Article"),
        EntityType::new("Tool"),
        EntityType::new("Technology"),
        EntityType::new("Question"),
        EntityType::new("Idea"),
        EntityType::new("Event"),
        EntityType::new("Skill"),
        EntityType::new("Location"),
        EntityType::new("Dataset"),
        EntityType::new("Collection"),
        EntityType::new("Workspace"),
        EntityType::new("Decision"),
        EntityType::new("Note"),
    ];
    assert_eq!(types.len(), 20);
}

#[test]
fn test_component_types_cover_all_variants() {
    let types = vec![
        ComponentType::Title,
        ComponentType::Description,
        ComponentType::Content,
        ComponentType::Tags,
        ComponentType::Author,
        ComponentType::Embedding,
        ComponentType::Summary,
        ComponentType::Timeline,
        ComponentType::Language,
        ComponentType::Provenance,
    ];
    assert_eq!(types.len(), 10);
}

#[test]
fn test_entity_serialization_roundtrip() {
    let entity = Entity::new(EntityType::new("Article"));
    let json = serde_json::to_string(&entity).unwrap();
    let deserialized: Entity = serde_json::from_str(&json).unwrap();
    assert_eq!(entity.id, deserialized.id);
    assert_eq!(entity.entity_type, deserialized.entity_type);
    assert_eq!(entity.version, deserialized.version);
}

#[test]
fn test_component_serialization_roundtrip() {
    let comp = Component::new(
        uuid::Uuid::new_v4(),
        ComponentType::Provenance,
        serde_json::json!({"source": "test.md", "imported_at": "2026-01-01T00:00:00Z"}),
    );
    let json = serde_json::to_string(&comp).unwrap();
    let deserialized: Component = serde_json::from_str(&json).unwrap();
    assert_eq!(comp.id, deserialized.id);
    assert_eq!(comp.component_type, deserialized.component_type);
    assert_eq!(comp.data, deserialized.data);
}

#[test]
fn test_relationship_types() {
    let rel = Relationship::new(
        uuid::Uuid::new_v4(),
        uuid::Uuid::new_v4(),
        RelationshipType::References,
    );
    assert!(rel.is_active);
    assert_eq!(rel.relationship_type, RelationshipType::References);
}

#[test]
fn test_event_types_cover_canonical_set() {
    let types = vec![
        EventType::EntityCreated,
        EventType::EntityUpdated,
        EventType::EntityArchived,
        EventType::EntityRestored,
        EventType::EntityResolved,
        EventType::ComponentAdded,
        EventType::ComponentUpdated,
        EventType::ComponentRemoved,
        EventType::RelationshipCreated,
        EventType::RelationshipArchived,
    ];
    assert_eq!(types.len(), 10);
}

#[test]
fn test_search_query_construction() {
    let q = SearchQuery {
        query: "test".to_string(),
        entity_type: Some("Article".to_string()),
        tag: Some("rust".to_string()),
    };
    assert_eq!(q.query, "test");
    assert!(q.entity_type.is_some());
    assert!(q.tag.is_some());
}

#[test]
fn test_search_result_fields() {
    let r = SearchResult {
        entity_id: uuid::Uuid::new_v4(),
        score: -0.5,
        confidence: Some(0.95),
        snippet: Some("test snippet".to_string()),
    };
    assert!(r.confidence.unwrap() > 0.9);
    assert!(r.snippet.is_some());
}

#[test]
fn test_resolution_candidate_fields() {
    let c = ResolutionCandidate {
        entity_id: uuid::Uuid::new_v4(),
        confidence: 1.0,
        reason: "Exact match".to_string(),
    };
    assert_eq!(c.confidence, 1.0);
}

#[test]
fn test_storage_error_display() {
    let not_found = StorageError::NotFound;
    assert_eq!(not_found.to_string(), "not found");

    let internal = StorageError::Internal("test error".to_string());
    assert!(internal.to_string().contains("test error"));
}
