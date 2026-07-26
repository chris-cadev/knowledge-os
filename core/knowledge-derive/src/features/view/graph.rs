use async_trait::async_trait;
use knowledge_core::features::component::ComponentType;
use knowledge_core::ports::{
    ComponentRepository, EntityRepository, Event, GraphData, GraphEdge, GraphNode,
    RelationshipRepository, StorageError, TraversalConfig, TraversalDirection, TraversalError,
    TraversalPort, TraversalQuery, ViewAdapter, ViewFilter, ViewOutput,
};

/// Default maximum results when no limit is specified in the filter.
const DEFAULT_MAX_RESULTS: usize = 100;

/// View adapter that renders entities as a nodes-and-edges graph.
///
/// When `start_entity_id` is provided in the filter, uses traversal to extract
/// a bounded subgraph. When no start entity is provided, returns all entities
/// and their direct relationships, bounded by `max_results`.
pub struct GraphViewAdapter {
    entity_repo: Box<dyn EntityRepository>,
    component_repo: Box<dyn ComponentRepository>,
    relationship_repo: Box<dyn RelationshipRepository>,
    traversal_port: Box<dyn TraversalPort>,
}

impl GraphViewAdapter {
    /// Creates a new graph view adapter.
    pub fn new(
        entity_repo: Box<dyn EntityRepository>,
        component_repo: Box<dyn ComponentRepository>,
        relationship_repo: Box<dyn RelationshipRepository>,
        traversal_port: Box<dyn TraversalPort>,
    ) -> Self {
        Self {
            entity_repo,
            component_repo,
            relationship_repo,
            traversal_port,
        }
    }

    /// Gets the title for an entity from its Title component.
    async fn get_entity_title(&self, entity_id: uuid::Uuid) -> Result<String, StorageError> {
        let components = self.component_repo.get(entity_id).await?;
        Ok(components
            .iter()
            .find(|c| c.component_type == ComponentType::Title)
            .and_then(|c| c.data.as_str().map(String::from))
            .unwrap_or_else(|| "Untitled".to_string()))
    }
}

#[async_trait]
impl ViewAdapter for GraphViewAdapter {
    fn name(&self) -> &str {
        "graph"
    }

    async fn render(&self, filter: &ViewFilter) -> Result<ViewOutput, StorageError> {
        let max_results = filter.max_results.unwrap_or(DEFAULT_MAX_RESULTS);

        // If start_entity_id is provided, use traversal to extract a subgraph
        if let Some(ref start_id_str) = filter.start_entity_id {
            let start_id = uuid::Uuid::parse_str(start_id_str)
                .map_err(|e| StorageError::Internal(format!("invalid entity ID: {}", e)))?;

            let max_depth = filter.max_depth.unwrap_or(3);

            let relationship_type = filter
                .relationship_types
                .as_ref()
                .and_then(|types| types.first())
                .cloned();

            let entity_type_filter = filter
                .entity_types
                .as_ref()
                .and_then(|types| types.first())
                .cloned();

            let query = TraversalQuery {
                start_id,
                direction: TraversalDirection::Both,
                max_depth: Some(max_depth),
                max_results: Some(max_results),
                relationship_type,
                entity_type_filter,
            };

            let config = TraversalConfig::default();
            let results = match self.traversal_port.traverse(&query, &config).await {
                Ok(r) => r,
                Err(TraversalError::Storage(e)) => return Err(e),
                Err(TraversalError::StartNotFound(id)) => {
                    return Err(StorageError::Internal(format!(
                        "start entity not found: {}",
                        id
                    )));
                }
                Err(TraversalError::LimitExceeded { limit }) => {
                    return Err(StorageError::Internal(format!(
                        "traversal limit exceeded: {}",
                        limit
                    )));
                }
            };

            // Collect all unique entity IDs from traversal results
            let mut entity_ids = std::collections::HashSet::new();
            entity_ids.insert(start_id);
            for result in &results {
                for &id in &result.path {
                    entity_ids.insert(id);
                }
            }

            // Build nodes
            let mut nodes = Vec::new();
            for &id in &entity_ids {
                if let Some(entity) = self.entity_repo.get(id).await? {
                    let label = self.get_entity_title(id).await?;
                    let node_type = entity.entity_type.to_string();
                    nodes.push(GraphNode {
                        entity,
                        label,
                        node_type,
                    });
                }
            }

            // Build edges from traversal results (deduplicated)
            let mut seen_edges = std::collections::HashSet::new();
            let mut edges = Vec::new();
            for result in &results {
                for edge in &result.edges {
                    let rel_type_str = format!("{:?}", edge.relationship_type);
                    let edge_key =
                        format!("{}->{}:{}", edge.source_id, edge.target_id, rel_type_str);
                    if seen_edges.insert(edge_key) {
                        let rel_type = edge.relationship_type.clone();
                        edges.push(GraphEdge {
                            source_id: edge.source_id.to_string(),
                            target_id: edge.target_id.to_string(),
                            relationship_type: format!("{:?}", rel_type),
                            label: format!("{:?}", rel_type),
                        });
                    }
                }
            }

            return Ok(ViewOutput::Graph(GraphData { nodes, edges }));
        }

        // No start entity: return all entities with direct relationships
        let entities = self.entity_repo.list().await?;

        // Apply entity type filter
        let filtered: Vec<_> = match &filter.entity_types {
            Some(types) => entities
                .into_iter()
                .filter(|e| types.iter().any(|t| t == &e.entity_type))
                .collect(),
            None => entities,
        };

        let mut nodes = Vec::new();
        for entity in filtered.iter().take(max_results) {
            let label = self.get_entity_title(entity.id).await?;
            let node_type = entity.entity_type.to_string();
            nodes.push(GraphNode {
                entity: entity.clone(),
                label,
                node_type,
            });
        }

        // Collect direct relationships between visible nodes
        let visible_ids: std::collections::HashSet<_> = nodes.iter().map(|n| n.entity.id).collect();
        let mut edges = Vec::new();
        let mut seen_edges = std::collections::HashSet::new();

        for node in &nodes {
            let relationships = self.relationship_repo.by_source(node.entity.id).await?;

            for rel in &relationships {
                if !rel.is_active {
                    continue;
                }
                if !visible_ids.contains(&rel.target_id) {
                    continue;
                }
                let rel_type_str = format!("{:?}", rel.relationship_type);
                let edge_key = format!("{}->{}:{}", rel.source_id, rel.target_id, rel_type_str);
                if seen_edges.insert(edge_key) {
                    let rel_type = rel.relationship_type.clone();
                    edges.push(GraphEdge {
                        source_id: rel.source_id.to_string(),
                        target_id: rel.target_id.to_string(),
                        relationship_type: format!("{:?}", rel_type),
                        label: format!("{:?}", rel_type),
                    });
                }
            }
        }

        // Apply max_results to edges as well
        edges.truncate(max_results);

        Ok(ViewOutput::Graph(GraphData { nodes, edges }))
    }

    async fn on_event(&self, _event: &Event) -> Result<(), StorageError> {
        // Graph view rebuilds on every render — no cached state to invalidate.
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::Utc;
    use knowledge_core::features::component::{Component, ComponentType};
    use knowledge_core::features::entity::{Entity, EntityType};
    use knowledge_core::features::relationship::{Relationship, RelationshipType};
    use knowledge_core::ports::{
        ComponentRepository, EntityRepository, EntityVersion, RelationshipRepository, StorageError,
        TraversalConfig, TraversalError, TraversalResult,
    };
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

    #[derive(Default)]
    struct MockRelationshipRepo {
        relationships: Vec<Relationship>,
    }

    #[derive(Default)]
    struct MockTraversalPort {
        results: Vec<TraversalResult>,
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

    #[async_trait]
    impl RelationshipRepository for MockRelationshipRepo {
        async fn get(&self, _id: Uuid) -> Result<Option<Relationship>, StorageError> {
            Ok(None)
        }
        async fn save(&self, _relationship: &Relationship) -> Result<(), StorageError> {
            Ok(())
        }
        async fn update(&self, _relationship: &Relationship) -> Result<(), StorageError> {
            Ok(())
        }
        async fn delete(&self, _id: Uuid) -> Result<(), StorageError> {
            Ok(())
        }
        async fn by_source(&self, source_id: Uuid) -> Result<Vec<Relationship>, StorageError> {
            Ok(self
                .relationships
                .iter()
                .filter(|r| r.source_id == source_id)
                .cloned()
                .collect())
        }
        async fn by_target(&self, _target_id: Uuid) -> Result<Vec<Relationship>, StorageError> {
            Ok(vec![])
        }
        async fn find_by_source_and_target(
            &self,
            _source_id: Uuid,
            _target_id: Uuid,
        ) -> Result<Option<Relationship>, StorageError> {
            Ok(None)
        }
        async fn find_by_type(
            &self,
            _relationship_type: &str,
        ) -> Result<Vec<Relationship>, StorageError> {
            Ok(vec![])
        }
    }

    #[async_trait]
    impl TraversalPort for MockTraversalPort {
        async fn traverse(
            &self,
            _query: &TraversalQuery,
            _config: &TraversalConfig,
        ) -> Result<Vec<TraversalResult>, TraversalError> {
            Ok(self.results.clone())
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
    async fn test_nodes_and_edges_from_entities_and_relationships() {
        let entity_a = make_entity("Concept");
        let entity_b = make_entity("Concept");

        let relationship =
            Relationship::new(entity_a.id, entity_b.id, RelationshipType::References);

        let entity_repo = MockEntityRepo {
            entities: vec![entity_a.clone(), entity_b.clone()],
        };

        let mut component_data = HashMap::new();
        component_data.insert(
            entity_a.id,
            vec![make_title_component(entity_a.id, "Entity A")],
        );
        component_data.insert(
            entity_b.id,
            vec![make_title_component(entity_b.id, "Entity B")],
        );
        let component_repo = MockComponentRepo {
            components: component_data,
        };

        let relationship_repo = MockRelationshipRepo {
            relationships: vec![relationship],
        };

        let traversal_port = MockTraversalPort::default();

        let adapter = GraphViewAdapter::new(
            Box::new(entity_repo),
            Box::new(component_repo),
            Box::new(relationship_repo),
            Box::new(traversal_port),
        );

        let output = adapter.render(&ViewFilter::default()).await.unwrap();
        match output {
            ViewOutput::Graph(graph) => {
                assert_eq!(graph.nodes.len(), 2);
                assert_eq!(graph.edges.len(), 1);
                assert_eq!(graph.edges[0].source_id, entity_a.id.to_string());
                assert_eq!(graph.edges[0].target_id, entity_b.id.to_string());
            }
            other => panic!("Expected Graph output, got {:?}", other),
        }
    }

    #[tokio::test]
    async fn test_subgraph_from_start_entity() {
        let entity_a = make_entity("Concept");
        let entity_b = make_entity("Concept");
        let entity_c = make_entity("Concept");

        let entity_repo = MockEntityRepo {
            entities: vec![entity_a.clone(), entity_b.clone(), entity_c.clone()],
        };

        let mut component_data = HashMap::new();
        component_data.insert(entity_a.id, vec![make_title_component(entity_a.id, "A")]);
        component_data.insert(entity_b.id, vec![make_title_component(entity_b.id, "B")]);
        component_data.insert(entity_c.id, vec![make_title_component(entity_c.id, "C")]);
        let component_repo = MockComponentRepo {
            components: component_data,
        };

        let relationship_repo = MockRelationshipRepo::default();

        // Traversal returns A -> B -> C
        let traversal_port = MockTraversalPort {
            results: vec![
                TraversalResult {
                    path: vec![entity_a.id, entity_b.id],
                    edges: vec![knowledge_core::ports::TraversalEdge {
                        source_id: entity_a.id,
                        target_id: entity_b.id,
                        relationship_type: RelationshipType::References,
                    }],
                    depth: 1,
                },
                TraversalResult {
                    path: vec![entity_a.id, entity_b.id, entity_c.id],
                    edges: vec![
                        knowledge_core::ports::TraversalEdge {
                            source_id: entity_a.id,
                            target_id: entity_b.id,
                            relationship_type: RelationshipType::References,
                        },
                        knowledge_core::ports::TraversalEdge {
                            source_id: entity_b.id,
                            target_id: entity_c.id,
                            relationship_type: RelationshipType::References,
                        },
                    ],
                    depth: 2,
                },
            ],
        };

        let adapter = GraphViewAdapter::new(
            Box::new(entity_repo),
            Box::new(component_repo),
            Box::new(relationship_repo),
            Box::new(traversal_port),
        );

        let filter = ViewFilter {
            start_entity_id: Some(entity_a.id.to_string()),
            max_depth: Some(2),
            ..Default::default()
        };

        let output = adapter.render(&filter).await.unwrap();
        match output {
            ViewOutput::Graph(graph) => {
                // All 3 entities should appear as nodes
                assert_eq!(graph.nodes.len(), 3);
                // Edges from traversal
                assert!(!graph.edges.is_empty());
            }
            other => panic!("Expected Graph output, got {:?}", other),
        }
    }

    #[tokio::test]
    async fn test_no_start_entity_returns_all_entities_with_direct_relationships() {
        let entity_a = make_entity("Concept");
        let entity_b = make_entity("Concept");

        let relationship =
            Relationship::new(entity_a.id, entity_b.id, RelationshipType::References);

        let entity_repo = MockEntityRepo {
            entities: vec![entity_a.clone(), entity_b.clone()],
        };

        let mut component_data = HashMap::new();
        component_data.insert(entity_a.id, vec![make_title_component(entity_a.id, "A")]);
        component_data.insert(entity_b.id, vec![make_title_component(entity_b.id, "B")]);
        let component_repo = MockComponentRepo {
            components: component_data,
        };

        let relationship_repo = MockRelationshipRepo {
            relationships: vec![relationship],
        };

        let traversal_port = MockTraversalPort::default();

        let adapter = GraphViewAdapter::new(
            Box::new(entity_repo),
            Box::new(component_repo),
            Box::new(relationship_repo),
            Box::new(traversal_port),
        );

        let output = adapter.render(&ViewFilter::default()).await.unwrap();
        match output {
            ViewOutput::Graph(graph) => {
                assert_eq!(graph.nodes.len(), 2);
                assert_eq!(graph.edges.len(), 1);
            }
            other => panic!("Expected Graph output, got {:?}", other),
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

        let relationship_repo = MockRelationshipRepo::default();
        let traversal_port = MockTraversalPort::default();

        let adapter = GraphViewAdapter::new(
            Box::new(entity_repo),
            Box::new(component_repo),
            Box::new(relationship_repo),
            Box::new(traversal_port),
        );

        let filter = ViewFilter {
            entity_types: Some(vec![EntityType::new("Concept")]),
            ..Default::default()
        };

        let output = adapter.render(&filter).await.unwrap();
        match output {
            ViewOutput::Graph(graph) => {
                assert_eq!(graph.nodes.len(), 1);
                assert_eq!(graph.nodes[0].node_type, "Concept");
            }
            other => panic!("Expected Graph output, got {:?}", other),
        }
    }

    #[tokio::test]
    async fn test_empty_graph_produces_empty_output() {
        let entity_repo = MockEntityRepo::default();
        let component_repo = MockComponentRepo::default();
        let relationship_repo = MockRelationshipRepo::default();
        let traversal_port = MockTraversalPort::default();

        let adapter = GraphViewAdapter::new(
            Box::new(entity_repo),
            Box::new(component_repo),
            Box::new(relationship_repo),
            Box::new(traversal_port),
        );

        let output = adapter.render(&ViewFilter::default()).await.unwrap();
        match output {
            ViewOutput::Graph(graph) => {
                assert!(graph.nodes.is_empty());
                assert!(graph.edges.is_empty());
            }
            other => panic!("Expected Graph output, got {:?}", other),
        }
    }

    #[tokio::test]
    async fn test_view_name() {
        let adapter = GraphViewAdapter::new(
            Box::new(MockEntityRepo::default()),
            Box::new(MockComponentRepo::default()),
            Box::new(MockRelationshipRepo::default()),
            Box::new(MockTraversalPort::default()),
        );
        assert_eq!(adapter.name(), "graph");
    }
}
