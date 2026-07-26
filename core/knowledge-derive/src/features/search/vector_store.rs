use std::collections::HashMap;
use std::sync::RwLock;

use async_trait::async_trait;
use knowledge_core::ports::{VectorError, VectorFilter, VectorMetadata, VectorResult, VectorStore};

/// In-memory vector store using brute-force cosine similarity.
///
/// Works well for fewer than 100K vectors. For larger datasets, swap
/// in an HNSW or sqlite-vec implementation behind the same `VectorStore` trait.
pub struct InMemoryVectorStore {
    vectors: RwLock<HashMap<String, Vec<f32>>>,
    metadata: RwLock<HashMap<String, VectorMetadata>>,
    dimensions: usize,
}

impl InMemoryVectorStore {
    /// Create a new in-memory vector store with the given dimensionality.
    pub fn new(dimensions: usize) -> Self {
        Self {
            vectors: RwLock::new(HashMap::new()),
            metadata: RwLock::new(HashMap::new()),
            dimensions,
        }
    }

    /// Return the number of vectors currently stored.
    pub fn len(&self) -> usize {
        self.vectors.read().map(|v| v.len()).unwrap_or(0)
    }

    /// Return `true` if no vectors are stored.
    pub fn is_empty(&self) -> bool {
        self.len() == 0
    }
}

/// Compute cosine similarity between two vectors.
///
/// Returns 0.0 if either vector has zero magnitude.
pub fn cosine_similarity(a: &[f32], b: &[f32]) -> f64 {
    let dot: f64 = a
        .iter()
        .zip(b.iter())
        .map(|(x, y)| (*x as f64) * (*y as f64))
        .sum();
    let norm_a: f64 = a.iter().map(|x| (*x as f64).powi(2)).sum::<f64>().sqrt();
    let norm_b: f64 = b.iter().map(|x| (*x as f64).powi(2)).sum::<f64>().sqrt();
    if norm_a == 0.0 || norm_b == 0.0 {
        0.0
    } else {
        dot / (norm_a * norm_b)
    }
}

#[async_trait]
impl VectorStore for InMemoryVectorStore {
    async fn upsert(
        &self,
        entity_id: &str,
        vector: &[f32],
        metadata: Option<VectorMetadata>,
    ) -> Result<(), VectorError> {
        if vector.len() != self.dimensions {
            return Err(VectorError::DimensionMismatch {
                expected: self.dimensions,
                actual: vector.len(),
            });
        }

        self.vectors
            .write()
            .map_err(|e| VectorError::Storage(e.to_string()))?
            .insert(entity_id.to_string(), vector.to_vec());

        if let Some(meta) = metadata {
            self.metadata
                .write()
                .map_err(|e| VectorError::Storage(e.to_string()))?
                .insert(entity_id.to_string(), meta);
        }

        Ok(())
    }

    async fn search(
        &self,
        query: &[f32],
        k: usize,
        filter: Option<VectorFilter>,
    ) -> Result<Vec<VectorResult>, VectorError> {
        if query.len() != self.dimensions {
            return Err(VectorError::DimensionMismatch {
                expected: self.dimensions,
                actual: query.len(),
            });
        }

        let vectors = self
            .vectors
            .read()
            .map_err(|e| VectorError::Storage(e.to_string()))?;
        let metadata_map = self
            .metadata
            .read()
            .map_err(|e| VectorError::Storage(e.to_string()))?;

        let min_score = filter.as_ref().and_then(|f| f.min_score);
        let filter_entity_types = filter.as_ref().and_then(|f| f.entity_types.as_ref());
        let filter_tags = filter.as_ref().and_then(|f| f.tags.as_ref());

        let mut scored: Vec<(String, f64)> = vectors
            .iter()
            .map(|(id, vec)| {
                let score = cosine_similarity(query, vec);
                (id.clone(), score)
            })
            .filter(|(_, score)| min_score.is_none() || *score >= min_score.unwrap())
            .filter(|(id, _)| {
                // Apply entity type filter
                if let Some(types) = filter_entity_types {
                    if let Some(meta) = metadata_map.get(id) {
                        let entity_type_str = meta.entity_type.as_str();
                        if !types.iter().any(|t| t.as_str() == entity_type_str) {
                            return false;
                        }
                    } else {
                        return false;
                    }
                }
                // Apply tag filter (tags not yet stored in VectorMetadata, skip if filter present but no tags)
                if filter_tags.is_some() {
                    // Tags are not stored in VectorMetadata currently; allow through
                    // This filter is a placeholder for future tag-based filtering
                }
                true
            })
            .collect();

        scored.sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap_or(std::cmp::Ordering::Equal));

        let results: Vec<VectorResult> = scored
            .into_iter()
            .take(k)
            .map(|(id, score)| VectorResult {
                entity_id: id.clone(),
                score,
                metadata: metadata_map.get(&id).cloned(),
            })
            .collect();

        Ok(results)
    }

    async fn delete(&self, entity_id: &str) -> Result<(), VectorError> {
        self.vectors
            .write()
            .map_err(|e| VectorError::Storage(e.to_string()))?
            .remove(entity_id);

        self.metadata
            .write()
            .map_err(|e| VectorError::Storage(e.to_string()))?
            .remove(entity_id);

        Ok(())
    }

    async fn rebuild(&self) -> Result<(), VectorError> {
        self.vectors
            .write()
            .map_err(|e| VectorError::Storage(e.to_string()))?
            .clear();

        self.metadata
            .write()
            .map_err(|e| VectorError::Storage(e.to_string()))?
            .clear();

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use knowledge_core::features::entity::EntityType;

    fn test_store(dimensions: usize) -> InMemoryVectorStore {
        InMemoryVectorStore::new(dimensions)
    }

    #[test]
    fn cosine_similarity_returns_correct_values() {
        let a = vec![1.0, 0.0, 0.0];
        let b = vec![1.0, 0.0, 0.0];
        assert!((cosine_similarity(&a, &b) - 1.0).abs() < 1e-10);
    }

    #[test]
    fn cosine_similarity_returns_zero_for_zero_vectors() {
        let a = vec![0.0, 0.0, 0.0];
        let b = vec![1.0, 0.0, 0.0];
        assert!((cosine_similarity(&a, &b) - 0.0).abs() < 1e-10);
    }

    #[test]
    fn cosine_similarity_returns_one_for_identical_vectors() {
        let a = vec![1.0, 2.0, 3.0];
        let b = vec![1.0, 2.0, 3.0];
        assert!((cosine_similarity(&a, &b) - 1.0).abs() < 1e-10);
    }

    #[test]
    fn cosine_similarity_returns_zero_for_orthogonal_vectors() {
        let a = vec![1.0, 0.0];
        let b = vec![0.0, 1.0];
        assert!((cosine_similarity(&a, &b) - 0.0).abs() < 1e-10);
    }

    #[tokio::test]
    async fn upsert_stores_vector_correctly() {
        let store = test_store(3);
        let vector = vec![1.0, 2.0, 3.0];
        store.upsert("e1", &vector, None).await.unwrap();
        assert_eq!(store.len(), 1);
    }

    #[tokio::test]
    async fn search_returns_top_k_sorted_by_score() {
        let store = test_store(3);
        store.upsert("e1", &[1.0, 0.0, 0.0], None).await.unwrap();
        store.upsert("e2", &[0.0, 1.0, 0.0], None).await.unwrap();
        store.upsert("e3", &[1.0, 0.0, 0.0], None).await.unwrap();

        let results = store.search(&[1.0, 0.0, 0.0], 2, None).await.unwrap();
        assert_eq!(results.len(), 2);
        // e1 and e3 should be at the top (score 1.0)
        assert!((results[0].score - 1.0).abs() < 1e-10);
        assert!((results[1].score - 1.0).abs() < 1e-10);
    }

    #[tokio::test]
    async fn search_with_filter_returns_only_matching_results() {
        let store = test_store(3);
        store
            .upsert(
                "e1",
                &[1.0, 0.0, 0.0],
                Some(VectorMetadata {
                    model: "test".into(),
                    entity_type: "Concept".into(),
                    title: "A".into(),
                }),
            )
            .await
            .unwrap();
        store
            .upsert(
                "e2",
                &[0.9, 0.1, 0.0],
                Some(VectorMetadata {
                    model: "test".into(),
                    entity_type: "Person".into(),
                    title: "B".into(),
                }),
            )
            .await
            .unwrap();

        let filter = VectorFilter {
            entity_types: Some(vec![EntityType::new("Concept")]),
            tags: None,
            min_score: None,
        };

        let results = store
            .search(&[1.0, 0.0, 0.0], 10, Some(filter))
            .await
            .unwrap();
        assert_eq!(results.len(), 1);
        assert_eq!(results[0].entity_id, "e1");
    }

    #[tokio::test]
    async fn delete_removes_vector() {
        let store = test_store(3);
        store.upsert("e1", &[1.0, 0.0, 0.0], None).await.unwrap();
        assert_eq!(store.len(), 1);

        store.delete("e1").await.unwrap();
        assert_eq!(store.len(), 0);
    }

    #[tokio::test]
    async fn rebuild_clears_and_repopulates() {
        let store = test_store(3);
        store.upsert("e1", &[1.0, 0.0, 0.0], None).await.unwrap();
        store.upsert("e2", &[0.0, 1.0, 0.0], None).await.unwrap();
        assert_eq!(store.len(), 2);

        store.rebuild().await.unwrap();
        assert_eq!(store.len(), 0);

        // Can re-populate after rebuild
        store.upsert("e3", &[0.0, 0.0, 1.0], None).await.unwrap();
        assert_eq!(store.len(), 1);
    }

    #[tokio::test]
    async fn dimension_mismatch_returns_error() {
        let store = test_store(3);
        let result = store.upsert("e1", &[1.0, 0.0], None).await;
        assert!(result.is_err());
        match result.unwrap_err() {
            VectorError::DimensionMismatch { expected, actual } => {
                assert_eq!(expected, 3);
                assert_eq!(actual, 2);
            }
            _ => panic!("Expected DimensionMismatch"),
        }
    }

    #[tokio::test]
    async fn search_dimension_mismatch_returns_error() {
        let store = test_store(3);
        let result = store.search(&[1.0, 0.0], 10, None).await;
        assert!(result.is_err());
    }

    #[tokio::test]
    async fn search_with_min_score_filter() {
        let store = test_store(3);
        store.upsert("e1", &[1.0, 0.0, 0.0], None).await.unwrap();
        store.upsert("e2", &[0.0, 1.0, 0.0], None).await.unwrap();

        let filter = VectorFilter {
            entity_types: None,
            tags: None,
            min_score: Some(0.9),
        };

        let results = store
            .search(&[1.0, 0.0, 0.0], 10, Some(filter))
            .await
            .unwrap();
        assert_eq!(results.len(), 1);
        assert_eq!(results[0].entity_id, "e1");
    }
}
