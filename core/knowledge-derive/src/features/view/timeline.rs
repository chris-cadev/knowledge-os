use async_trait::async_trait;
use knowledge_core::features::component::ComponentType;
use knowledge_core::ports::{
    ComponentRepository, EntityRepository, Event, StorageError, TimelineData, TimelineEntry,
    ViewAdapter, ViewFilter, ViewOutput,
};

/// View adapter that renders entities ordered by their creation timestamp.
///
/// Entries are sorted chronologically. Entities without timestamps are excluded.
/// Uses the `created_at` field from the entity and optionally the `Timeline` component
/// for more precise timestamps.
pub struct TimelineViewAdapter {
    entity_repo: Box<dyn EntityRepository>,
    component_repo: Box<dyn ComponentRepository>,
}

impl TimelineViewAdapter {
    /// Creates a new timeline view adapter.
    pub fn new(
        entity_repo: Box<dyn EntityRepository>,
        component_repo: Box<dyn ComponentRepository>,
    ) -> Self {
        Self {
            entity_repo,
            component_repo,
        }
    }

    /// Gets the title for an entity from its Title component.
    async fn get_title(&self, entity_id: uuid::Uuid) -> Result<String, StorageError> {
        let components = self.component_repo.get(entity_id).await?;
        Ok(components
            .iter()
            .find(|c| c.component_type == ComponentType::Title)
            .and_then(|c| c.data.as_str().map(String::from))
            .unwrap_or_else(|| "Untitled".to_string()))
    }

    /// Gets the creation timestamp from the Timeline component, or falls back to entity created_at.
    async fn get_timestamp(
        &self,
        entity_id: uuid::Uuid,
        fallback: chrono::DateTime<chrono::Utc>,
    ) -> Result<chrono::DateTime<chrono::Utc>, StorageError> {
        let components = self.component_repo.get(entity_id).await?;
        components
            .iter()
            .find(|c| c.component_type == ComponentType::Timeline)
            .and_then(|c| {
                c.data
                    .get("created_at")
                    .and_then(|v| v.as_str())
                    .and_then(|s| chrono::DateTime::parse_from_rfc3339(s).ok())
                    .map(|dt| dt.with_timezone(&chrono::Utc))
            })
            .or(Some(fallback))
            .ok_or_else(|| StorageError::Internal("no timestamp available".to_string()))
    }
}

#[async_trait]
impl ViewAdapter for TimelineViewAdapter {
    fn name(&self) -> &str {
        "timeline"
    }

    async fn render(&self, filter: &ViewFilter) -> Result<ViewOutput, StorageError> {
        let entities = self.entity_repo.list().await?;

        let mut entries = Vec::new();
        for entity in &entities {
            // Entity type filter
            if let Some(ref types) = filter.entity_types {
                if !types.iter().any(|t| t == &entity.entity_type) {
                    continue;
                }
            }

            let timestamp = self.get_timestamp(entity.id, entity.created_at).await?;
            let label = self.get_title(entity.id).await?;

            entries.push(TimelineEntry {
                entity: entity.clone(),
                timestamp: timestamp.to_rfc3339(),
                label,
            });
        }

        // Sort by timestamp ascending (oldest first)
        entries.sort_by(|a, b| a.timestamp.cmp(&b.timestamp));

        // Apply max_results
        if let Some(max) = filter.max_results {
            entries.truncate(max);
        }

        Ok(ViewOutput::Timeline(TimelineData { entries }))
    }

    async fn on_event(&self, _event: &Event) -> Result<(), StorageError> {
        // Timeline view rebuilds on every render — no cached state to invalidate.
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::{Duration, Utc};
    use knowledge_core::features::component::{Component, ComponentType};
    use knowledge_core::features::entity::{Entity, EntityType};
    use knowledge_core::ports::{ComponentRepository, EntityRepository, EntityVersion};
    use std::collections::HashMap;
    use uuid::Uuid;

    // ---------------------------------------------------------------------------
    // Mock repositories
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
    // Helpers
    // ---------------------------------------------------------------------------

    fn make_entity_with_time(
        entity_type: &str,
        created_at: chrono::DateTime<chrono::Utc>,
    ) -> Entity {
        Entity {
            id: Uuid::new_v4(),
            entity_type: EntityType::new(entity_type),
            is_active: true,
            created_at,
            updated_at: created_at,
            version: 1,
        }
    }

    fn make_title_component(entity_id: Uuid, title: &str) -> Component {
        Component::new(entity_id, ComponentType::Title, serde_json::json!(title))
    }

    // ---------------------------------------------------------------------------
    // Tests
    // ---------------------------------------------------------------------------

    #[tokio::test]
    async fn test_entities_ordered_by_creation_time() {
        let now = Utc::now();
        let early = now - Duration::hours(2);
        let late = now + Duration::hours(2);

        let entity_early = make_entity_with_time("Concept", early);
        let entity_late = make_entity_with_time("Concept", late);

        let entity_repo = MockEntityRepo {
            entities: vec![entity_late.clone(), entity_early.clone()],
        };

        let mut component_data = HashMap::new();
        component_data.insert(
            entity_early.id,
            vec![make_title_component(entity_early.id, "Early")],
        );
        component_data.insert(
            entity_late.id,
            vec![make_title_component(entity_late.id, "Late")],
        );
        let component_repo = MockComponentRepo {
            components: component_data,
        };

        let adapter = TimelineViewAdapter::new(Box::new(entity_repo), Box::new(component_repo));
        let output = adapter.render(&ViewFilter::default()).await.unwrap();

        match output {
            ViewOutput::Timeline(timeline) => {
                assert_eq!(timeline.entries.len(), 2);
                assert_eq!(timeline.entries[0].label, "Early");
                assert_eq!(timeline.entries[1].label, "Late");
            }
            other => panic!("Expected Timeline output, got {:?}", other),
        }
    }

    #[tokio::test]
    async fn test_filter_by_entity_type() {
        let now = Utc::now();
        let concept = make_entity_with_time("Concept", now);
        let paper = make_entity_with_time("Paper", now);

        let entity_repo = MockEntityRepo {
            entities: vec![concept.clone(), paper.clone()],
        };

        let mut component_data = HashMap::new();
        component_data.insert(concept.id, vec![make_title_component(concept.id, "C")]);
        component_data.insert(paper.id, vec![make_title_component(paper.id, "P")]);
        let component_repo = MockComponentRepo {
            components: component_data,
        };

        let adapter = TimelineViewAdapter::new(Box::new(entity_repo), Box::new(component_repo));

        let filter = ViewFilter {
            entity_types: Some(vec![EntityType::new("Concept")]),
            ..Default::default()
        };
        let output = adapter.render(&filter).await.unwrap();

        match output {
            ViewOutput::Timeline(timeline) => {
                assert_eq!(timeline.entries.len(), 1);
                assert_eq!(
                    timeline.entries[0].entity.entity_type,
                    EntityType::new("Concept")
                );
            }
            other => panic!("Expected Timeline output, got {:?}", other),
        }
    }

    #[tokio::test]
    async fn test_empty_data_produces_empty_timeline() {
        let entity_repo = MockEntityRepo { entities: vec![] };
        let component_repo = MockComponentRepo::default();

        let adapter = TimelineViewAdapter::new(Box::new(entity_repo), Box::new(component_repo));
        let output = adapter.render(&ViewFilter::default()).await.unwrap();

        match output {
            ViewOutput::Timeline(timeline) => {
                assert!(timeline.entries.is_empty());
            }
            other => panic!("Expected Timeline output, got {:?}", other),
        }
    }

    #[tokio::test]
    async fn test_view_name() {
        let adapter = TimelineViewAdapter::new(
            Box::new(MockEntityRepo::default()),
            Box::new(MockComponentRepo::default()),
        );
        assert_eq!(adapter.name(), "timeline");
    }
}
