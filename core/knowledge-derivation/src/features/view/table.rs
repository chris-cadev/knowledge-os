use async_trait::async_trait;
use knowledge_core::features::component::ComponentType;
use knowledge_core::ports::{
    ComponentRepository, EntityRepository, Event, SortOrder, StorageError, TableColumn, TableData,
    TableRow, ViewAdapter, ViewFilter, ViewOutput,
};

/// View adapter that renders entities as a sortable, filterable table.
///
/// Columns: ID, Type, Title, Tags, Created, Updated. Supports sorting by
/// any column and filtering by entity type, tags, and free-text search.
pub struct TableViewAdapter {
    entity_repo: Box<dyn EntityRepository>,
    component_repo: Box<dyn ComponentRepository>,
}

impl TableViewAdapter {
    /// Creates a new table view adapter.
    pub fn new(
        entity_repo: Box<dyn EntityRepository>,
        component_repo: Box<dyn ComponentRepository>,
    ) -> Self {
        Self {
            entity_repo,
            component_repo,
        }
    }

    /// Extracts title from entity components.
    async fn get_title(&self, entity_id: uuid::Uuid) -> Result<String, StorageError> {
        let components = self.component_repo.get(entity_id).await?;
        Ok(components
            .iter()
            .find(|c| c.component_type == ComponentType::Title)
            .and_then(|c| c.data.as_str().map(String::from))
            .unwrap_or_else(|| "Untitled".to_string()))
    }

    /// Extracts tags from entity components.
    async fn get_tags(&self, entity_id: uuid::Uuid) -> Result<String, StorageError> {
        let components = self.component_repo.get(entity_id).await?;
        Ok(components
            .iter()
            .filter(|c| c.component_type == ComponentType::Tags)
            .filter_map(|c| c.data.as_array())
            .flatten()
            .filter_map(|v| v.as_str())
            .collect::<Vec<_>>()
            .join(", "))
    }
}

#[async_trait]
impl ViewAdapter for TableViewAdapter {
    fn name(&self) -> &str {
        "table"
    }

    async fn render(&self, filter: &ViewFilter) -> Result<ViewOutput, StorageError> {
        let columns = vec![
            TableColumn {
                name: "ID".to_string(),
                sortable: true,
            },
            TableColumn {
                name: "Type".to_string(),
                sortable: true,
            },
            TableColumn {
                name: "Title".to_string(),
                sortable: true,
            },
            TableColumn {
                name: "Tags".to_string(),
                sortable: false,
            },
            TableColumn {
                name: "Created".to_string(),
                sortable: true,
            },
            TableColumn {
                name: "Updated".to_string(),
                sortable: true,
            },
        ];

        // Get entities
        let entities = self.entity_repo.list().await?;

        // Apply filters and build rows
        let mut rows = Vec::new();
        for entity in &entities {
            // Entity type filter
            if let Some(ref types) = filter.entity_types {
                if !types.iter().any(|t| t == &entity.entity_type) {
                    continue;
                }
            }

            let title = self.get_title(entity.id).await?;
            let tags = self.get_tags(entity.id).await?;

            // Tag filter
            if let Some(ref filter_tags) = filter.tags {
                let entity_tags: Vec<&str> = tags.split(", ").filter(|t| !t.is_empty()).collect();
                if !filter_tags
                    .iter()
                    .any(|ft| entity_tags.contains(&ft.as_str()))
                {
                    continue;
                }
            }

            // Text search filter
            if let Some(ref query) = filter.search_query {
                let query_lower = query.to_lowercase();
                let title_matches = title.to_lowercase().contains(&query_lower);
                let tags_matches = tags.to_lowercase().contains(&query_lower);
                let type_matches = entity
                    .entity_type
                    .to_string()
                    .to_lowercase()
                    .contains(&query_lower);
                if !title_matches && !tags_matches && !type_matches {
                    continue;
                }
            }

            rows.push(TableRow {
                cells: vec![
                    entity.id.to_string(),
                    entity.entity_type.to_string(),
                    title,
                    tags,
                    entity.created_at.to_rfc3339(),
                    entity.updated_at.to_rfc3339(),
                ],
            });
        }

        // Apply sorting
        if let Some(ref sort_by) = filter.sort_by {
            let sort_order = filter.sort_order.unwrap_or(SortOrder::Asc);
            let col_idx = columns.iter().position(|c| c.name == *sort_by);

            if let Some(idx) = col_idx {
                rows.sort_by(|a, b| {
                    let cmp = a.cells[idx].cmp(&b.cells[idx]);
                    match sort_order {
                        SortOrder::Asc => cmp,
                        SortOrder::Desc => cmp.reverse(),
                    }
                });
            }
        }

        // Apply max_results
        if let Some(max) = filter.max_results {
            rows.truncate(max);
        }

        Ok(ViewOutput::Table(TableData { columns, rows }))
    }

    async fn on_event(&self, _event: &Event) -> Result<(), StorageError> {
        // Table view rebuilds on every render — no cached state to invalidate.
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::Utc;
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
    async fn test_correct_columns_and_rows() {
        let entity = make_entity("Concept");
        let entity_repo = MockEntityRepo {
            entities: vec![entity.clone()],
        };

        let mut component_data = HashMap::new();
        component_data.insert(
            entity.id,
            vec![make_title_component(entity.id, "Transformer")],
        );
        let component_repo = MockComponentRepo {
            components: component_data,
        };

        let adapter = TableViewAdapter::new(Box::new(entity_repo), Box::new(component_repo));
        let output = adapter.render(&ViewFilter::default()).await.unwrap();

        match output {
            ViewOutput::Table(table) => {
                assert_eq!(table.columns.len(), 6);
                assert_eq!(table.rows.len(), 1);
                assert_eq!(table.rows[0].cells[2], "Transformer");
            }
            other => panic!("Expected Table output, got {:?}", other),
        }
    }

    #[tokio::test]
    async fn test_sort_by_title_ascending_and_descending() {
        let entity_a = make_entity("Concept");
        let entity_b = make_entity("Concept");

        let entity_repo = MockEntityRepo {
            entities: vec![entity_a.clone(), entity_b.clone()],
        };

        let mut component_data = HashMap::new();
        component_data.insert(
            entity_a.id,
            vec![make_title_component(entity_a.id, "Zebra")],
        );
        component_data.insert(
            entity_b.id,
            vec![make_title_component(entity_b.id, "Apple")],
        );
        let component_repo = MockComponentRepo {
            components: component_data,
        };

        let adapter = TableViewAdapter::new(Box::new(entity_repo), Box::new(component_repo));

        // Ascending
        let filter = ViewFilter {
            sort_by: Some("Title".to_string()),
            sort_order: Some(SortOrder::Asc),
            ..Default::default()
        };
        let output = adapter.render(&filter).await.unwrap();
        match output {
            ViewOutput::Table(table) => {
                assert_eq!(table.rows[0].cells[2], "Apple");
                assert_eq!(table.rows[1].cells[2], "Zebra");
            }
            other => panic!("Expected Table output, got {:?}", other),
        }

        // Descending
        let filter = ViewFilter {
            sort_by: Some("Title".to_string()),
            sort_order: Some(SortOrder::Desc),
            ..Default::default()
        };
        let output = adapter.render(&filter).await.unwrap();
        match output {
            ViewOutput::Table(table) => {
                assert_eq!(table.rows[0].cells[2], "Zebra");
                assert_eq!(table.rows[1].cells[2], "Apple");
            }
            other => panic!("Expected Table output, got {:?}", other),
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
        component_data.insert(concept.id, vec![make_title_component(concept.id, "C")]);
        component_data.insert(paper.id, vec![make_title_component(paper.id, "P")]);
        let component_repo = MockComponentRepo {
            components: component_data,
        };

        let adapter = TableViewAdapter::new(Box::new(entity_repo), Box::new(component_repo));

        let filter = ViewFilter {
            entity_types: Some(vec![EntityType::new("Concept")]),
            ..Default::default()
        };
        let output = adapter.render(&filter).await.unwrap();
        match output {
            ViewOutput::Table(table) => {
                assert_eq!(table.rows.len(), 1);
                assert_eq!(table.rows[0].cells[1], "Concept");
            }
            other => panic!("Expected Table output, got {:?}", other),
        }
    }

    #[tokio::test]
    async fn test_filter_by_search_query() {
        let entity = make_entity("Concept");
        let entity_repo = MockEntityRepo {
            entities: vec![entity.clone()],
        };

        let mut component_data = HashMap::new();
        component_data.insert(
            entity.id,
            vec![make_title_component(entity.id, "Machine Learning")],
        );
        let component_repo = MockComponentRepo {
            components: component_data,
        };

        let adapter = TableViewAdapter::new(Box::new(entity_repo), Box::new(component_repo));

        // Search matches
        let filter = ViewFilter {
            search_query: Some("machine".to_string()),
            ..Default::default()
        };
        let output = adapter.render(&filter).await.unwrap();
        match output {
            ViewOutput::Table(table) => {
                assert_eq!(table.rows.len(), 1);
            }
            other => panic!("Expected Table output, got {:?}", other),
        }

        // Search doesn't match
        let filter = ViewFilter {
            search_query: Some("quantum".to_string()),
            ..Default::default()
        };
        let output = adapter.render(&filter).await.unwrap();
        match output {
            ViewOutput::Table(table) => {
                assert!(table.rows.is_empty());
            }
            other => panic!("Expected Table output, got {:?}", other),
        }
    }

    #[tokio::test]
    async fn test_empty_data_produces_empty_table() {
        let entity_repo = MockEntityRepo { entities: vec![] };
        let component_repo = MockComponentRepo::default();

        let adapter = TableViewAdapter::new(Box::new(entity_repo), Box::new(component_repo));
        let output = adapter.render(&ViewFilter::default()).await.unwrap();
        match output {
            ViewOutput::Table(table) => {
                assert_eq!(table.columns.len(), 6);
                assert!(table.rows.is_empty());
            }
            other => panic!("Expected Table output, got {:?}", other),
        }
    }

    #[tokio::test]
    async fn test_view_name() {
        let adapter = TableViewAdapter::new(
            Box::new(MockEntityRepo::default()),
            Box::new(MockComponentRepo::default()),
        );
        assert_eq!(adapter.name(), "table");
    }
}
