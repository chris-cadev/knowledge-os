//! Benchmarks for graph traversal operations.
//!
//! Creates a synthetic graph with entities and relationships, then benchmarks
//! traversal at various depths and configurations.

use criterion::{black_box, criterion_group, criterion_main, Criterion};
use knowledge_core::features::entity::{Entity, EntityType};
use knowledge_core::features::relationship::{Relationship, RelationshipType};
use knowledge_core::ports::*;
use knowledge_storage::adapters::sqlite::SqliteStore;

/// Create a test store with synthetic data for benchmarking.
async fn setup_store(entity_count: usize, avg_edges_per_entity: usize) -> SqliteStore {
    let store = SqliteStore::new(":memory:").expect("failed to create in-memory store");

    let mut entities = Vec::with_capacity(entity_count);
    for _ in 0..entity_count {
        let entity = Entity::new(EntityType::new("Concept"));
        EntityRepository::save(&store, &entity)
            .await
            .expect("failed to save entity");
        entities.push(entity);
    }

    // Create relationships: each entity connects to random downstream entities
    let mut rng_state: u64 = 42;
    for i in 0..entity_count {
        for _ in 0..avg_edges_per_entity {
            // Simple LCG for deterministic "random" indices
            rng_state = rng_state
                .wrapping_mul(6364136223846793005)
                .wrapping_add(1442695040888963407);
            let target_idx = (rng_state as usize) % entity_count;
            if target_idx != i {
                let rel = Relationship::new(
                    entities[i].id,
                    entities[target_idx].id,
                    RelationshipType::References,
                );
                RelationshipRepository::save(&store, &rel)
                    .await
                    .expect("failed to save relationship");
            }
        }
    }

    store
}

fn bench_traversal_2hop(c: &mut Criterion) {
    let rt = tokio::runtime::Runtime::new().unwrap();
    let store = rt.block_on(setup_store(1000, 10));

    // Get first entity as start node
    let entities = rt.block_on(EntityRepository::list(&store)).unwrap();
    let start_id = entities[0].id;

    let config = TraversalConfig {
        default_max_depth: 10,
        default_max_results: 1000,
    };

    c.bench_function("traversal_2hop_1000_entities", |b| {
        b.iter(|| {
            rt.block_on(async {
                let query = TraversalQuery {
                    start_id: black_box(start_id),
                    direction: TraversalDirection::Outgoing,
                    max_depth: Some(2),
                    max_results: None,
                    relationship_type: None,
                    entity_type_filter: None,
                };
                TraversalPort::traverse(&store, &query, &config)
                    .await
                    .unwrap()
            })
        })
    });
}

fn bench_traversal_3hop(c: &mut Criterion) {
    let rt = tokio::runtime::Runtime::new().unwrap();
    let store = rt.block_on(setup_store(1000, 10));

    let entities = rt.block_on(EntityRepository::list(&store)).unwrap();
    let start_id = entities[0].id;

    let config = TraversalConfig {
        default_max_depth: 10,
        default_max_results: 1000,
    };

    c.bench_function("traversal_3hop_1000_entities", |b| {
        b.iter(|| {
            rt.block_on(async {
                let query = TraversalQuery {
                    start_id: black_box(start_id),
                    direction: TraversalDirection::Outgoing,
                    max_depth: Some(3),
                    max_results: None,
                    relationship_type: None,
                    entity_type_filter: None,
                };
                TraversalPort::traverse(&store, &query, &config)
                    .await
                    .unwrap()
            })
        })
    });
}

fn bench_traversal_with_type_filter(c: &mut Criterion) {
    let rt = tokio::runtime::Runtime::new().unwrap();
    let store = rt.block_on(setup_store(1000, 10));

    let entities = rt.block_on(EntityRepository::list(&store)).unwrap();
    let start_id = entities[0].id;

    let config = TraversalConfig {
        default_max_depth: 10,
        default_max_results: 1000,
    };

    c.bench_function("traversal_2hop_with_type_filter", |b| {
        b.iter(|| {
            rt.block_on(async {
                let query = TraversalQuery {
                    start_id: black_box(start_id),
                    direction: TraversalDirection::Outgoing,
                    max_depth: Some(2),
                    max_results: None,
                    relationship_type: None,
                    entity_type_filter: Some(EntityType::new("Concept")),
                };
                TraversalPort::traverse(&store, &query, &config)
                    .await
                    .unwrap()
            })
        })
    });
}

fn bench_traversal_bidirectional(c: &mut Criterion) {
    let rt = tokio::runtime::Runtime::new().unwrap();
    let store = rt.block_on(setup_store(1000, 10));

    let entities = rt.block_on(EntityRepository::list(&store)).unwrap();
    let start_id = entities[0].id;

    let config = TraversalConfig {
        default_max_depth: 10,
        default_max_results: 1000,
    };

    c.bench_function("traversal_2hop_bidirectional", |b| {
        b.iter(|| {
            rt.block_on(async {
                let query = TraversalQuery {
                    start_id: black_box(start_id),
                    direction: TraversalDirection::Both,
                    max_depth: Some(2),
                    max_results: None,
                    relationship_type: None,
                    entity_type_filter: None,
                };
                TraversalPort::traverse(&store, &query, &config)
                    .await
                    .unwrap()
            })
        })
    });
}

fn bench_traversal_2hop_100k(c: &mut Criterion) {
    let rt = tokio::runtime::Runtime::new().unwrap();
    // 100K entities with ~1M relationships
    let store = rt.block_on(setup_store(100_000, 10));

    let entities = rt.block_on(EntityRepository::list(&store)).unwrap();
    let start_id = entities[0].id;

    let config = TraversalConfig {
        default_max_depth: 10,
        default_max_results: 1000,
    };

    c.bench_function("traversal_2hop_100k_entities", |b| {
        b.iter(|| {
            rt.block_on(async {
                TraversalPort::traverse(
                    &store,
                    &TraversalQuery {
                        start_id: black_box(start_id),
                        direction: TraversalDirection::Outgoing,
                        max_depth: Some(2),
                        max_results: None,
                        relationship_type: None,
                        entity_type_filter: None,
                    },
                    &config,
                )
                .await
                .unwrap()
            })
        })
    });
}

criterion_group!(
    benches,
    bench_traversal_2hop,
    bench_traversal_3hop,
    bench_traversal_with_type_filter,
    bench_traversal_bidirectional,
    bench_traversal_2hop_100k,
);
criterion_main!(benches);
