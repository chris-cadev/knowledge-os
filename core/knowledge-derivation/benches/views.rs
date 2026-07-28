//! Benchmarks for view rendering operations.
//!
//! Creates mock repositories with synthetic data and benchmarks
//! tree, graph, table, and timeline view rendering.

use async_trait::async_trait;
use criterion::{black_box, criterion_group, criterion_main, Criterion};
use knowledge_core::features::component::{Component, ComponentType};
use knowledge_core::features::entity::{Entity, EntityType};
use knowledge_core::features::relationship::{Relationship, RelationshipType};
use knowledge_core::ports::*;
use std::collections::HashMap;
use uuid::Uuid;

// ---------------------------------------------------------------------------
// Mock repositories for benchmarking
// ---------------------------------------------------------------------------

#[derive(Clone)]
struct BenchEntityRepo {
    entities: Vec<Entity>,
}

#[derive(Clone)]
struct BenchComponentRepo {
    components: HashMap<Uuid, Vec<Component>>,
}

#[derive(Clone)]
struct BenchRelationshipRepo {
    relationships: Vec<Relationship>,
}

#[derive(Clone)]
struct BenchTraversalPort;

#[async_trait]
impl EntityRepository for BenchEntityRepo {
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
impl ComponentRepository for BenchComponentRepo {
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

#[async_trait]
impl RelationshipRepository for BenchRelationshipRepo {
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
impl TraversalPort for BenchTraversalPort {
    async fn traverse(
        &self,
        _query: &TraversalQuery,
        _config: &TraversalConfig,
    ) -> Result<Vec<TraversalResult>, TraversalError> {
        Ok(vec![])
    }
}

// ---------------------------------------------------------------------------
// Setup helpers
// ---------------------------------------------------------------------------

fn setup_bench_data(
    entity_count: usize,
) -> (BenchEntityRepo, BenchComponentRepo, BenchRelationshipRepo) {
    let types = ["Concept", "Paper", "Article", "Person", "Tool"];
    let mut entities = Vec::with_capacity(entity_count);
    let mut components = HashMap::new();
    let mut relationships = Vec::new();

    let mut rng_state: u64 = 42;
    for i in 0..entity_count {
        rng_state = rng_state
            .wrapping_mul(6364136223846793005)
            .wrapping_add(1442695040888963407);
        let type_idx = (rng_state as usize) % types.len();
        let entity = Entity::new(EntityType::new(types[type_idx]));

        let title = format!("Entity {}", i);
        let comp = Component::new(entity.id, ComponentType::Title, serde_json::json!(title));
        components
            .entry(entity.id)
            .or_insert_with(Vec::new)
            .push(comp);

        entities.push(entity);
    }

    // Create some relationships
    for i in 0..entity_count.min(500) {
        rng_state = rng_state
            .wrapping_mul(6364136223846793005)
            .wrapping_add(1442695040888963407);
        let target_idx = (rng_state as usize) % entity_count;
        if target_idx != i {
            relationships.push(Relationship::new(
                entities[i].id,
                entities[target_idx].id,
                RelationshipType::References,
            ));
        }
    }

    (
        BenchEntityRepo { entities },
        BenchComponentRepo { components },
        BenchRelationshipRepo { relationships },
    )
}

// ---------------------------------------------------------------------------
// Benchmarks
// ---------------------------------------------------------------------------

fn bench_tree_view(c: &mut Criterion) {
    let (entity_repo, component_repo, _) = setup_bench_data(1000);
    let adapter = knowledge_derivation::features::view::tree::TreeViewAdapter::new(
        Box::new(entity_repo),
        Box::new(component_repo),
        None,
    );

    c.bench_function("tree_view_1000_entities", |b| {
        b.iter(|| {
            let rt = tokio::runtime::Runtime::new().unwrap();
            rt.block_on(adapter.render(black_box(&ViewFilter::default())))
                .unwrap()
        })
    });
}

fn bench_tree_view_filtered(c: &mut Criterion) {
    let (entity_repo, component_repo, _) = setup_bench_data(1000);
    let adapter = knowledge_derivation::features::view::tree::TreeViewAdapter::new(
        Box::new(entity_repo),
        Box::new(component_repo),
        None,
    );

    let filter = ViewFilter {
        entity_types: Some(vec![EntityType::new("Concept")]),
        ..Default::default()
    };

    c.bench_function("tree_view_1000_entities_filtered", |b| {
        b.iter(|| {
            let rt = tokio::runtime::Runtime::new().unwrap();
            rt.block_on(adapter.render(black_box(&filter))).unwrap()
        })
    });
}

fn bench_graph_view(c: &mut Criterion) {
    let (entity_repo, component_repo, relationship_repo) = setup_bench_data(1000);
    let traversal_port = BenchTraversalPort;
    let adapter = knowledge_derivation::features::view::graph::GraphViewAdapter::new(
        Box::new(entity_repo),
        Box::new(component_repo),
        Box::new(relationship_repo),
        Box::new(traversal_port),
    );

    c.bench_function("graph_view_1000_entities", |b| {
        b.iter(|| {
            let rt = tokio::runtime::Runtime::new().unwrap();
            rt.block_on(adapter.render(black_box(&ViewFilter::default())))
                .unwrap()
        })
    });
}

fn bench_table_view(c: &mut Criterion) {
    let (entity_repo, component_repo, _) = setup_bench_data(1000);
    let adapter = knowledge_derivation::features::view::table::TableViewAdapter::new(
        Box::new(entity_repo),
        Box::new(component_repo),
    );

    c.bench_function("table_view_1000_entities", |b| {
        b.iter(|| {
            let rt = tokio::runtime::Runtime::new().unwrap();
            rt.block_on(adapter.render(black_box(&ViewFilter::default())))
                .unwrap()
        })
    });
}

fn bench_table_view_sorted(c: &mut Criterion) {
    let (entity_repo, component_repo, _) = setup_bench_data(1000);
    let adapter = knowledge_derivation::features::view::table::TableViewAdapter::new(
        Box::new(entity_repo),
        Box::new(component_repo),
    );

    let filter = ViewFilter {
        sort_by: Some("Title".to_string()),
        sort_order: Some(SortOrder::Asc),
        ..Default::default()
    };

    c.bench_function("table_view_1000_entities_sorted", |b| {
        b.iter(|| {
            let rt = tokio::runtime::Runtime::new().unwrap();
            rt.block_on(adapter.render(black_box(&filter))).unwrap()
        })
    });
}

fn bench_timeline_view(c: &mut Criterion) {
    let (entity_repo, component_repo, _) = setup_bench_data(1000);
    let adapter = knowledge_derivation::features::view::timeline::TimelineViewAdapter::new(
        Box::new(entity_repo),
        Box::new(component_repo),
    );

    c.bench_function("timeline_view_1000_entities", |b| {
        b.iter(|| {
            let rt = tokio::runtime::Runtime::new().unwrap();
            rt.block_on(adapter.render(black_box(&ViewFilter::default())))
                .unwrap()
        })
    });
}

criterion_group!(
    benches,
    bench_tree_view,
    bench_tree_view_filtered,
    bench_graph_view,
    bench_table_view,
    bench_table_view_sorted,
    bench_timeline_view,
);
criterion_main!(benches);
