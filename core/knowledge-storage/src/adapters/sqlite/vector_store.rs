use async_trait::async_trait;
use knowledge_core::ports::{VectorError, VectorFilter, VectorMetadata, VectorResult, VectorStore};
use rusqlite::params;

use super::store::SqliteStore;

/// SQLite-backed persistent vector store.
///
/// Stores embeddings as binary BLOBs in the `embeddings` table.
/// Uses brute-force cosine similarity for search (fine for <100K vectors).
pub struct SqliteVectorStore {
    store: SqliteStore,
    dimensions: usize,
}

impl SqliteVectorStore {
    /// Create a new SQLite-backed vector store with the given dimensionality.
    pub fn new(store: SqliteStore, dimensions: usize) -> Self {
        Self { store, dimensions }
    }

    /// Encode a float vector to a binary blob for storage.
    fn encode(vector: &[f32]) -> Vec<u8> {
        let mut bytes = Vec::with_capacity(vector.len() * 4);
        for &val in vector {
            bytes.extend_from_slice(&val.to_le_bytes());
        }
        bytes
    }

    /// Decode a binary blob back into a float vector.
    fn decode(bytes: &[u8], dimensions: usize) -> Vec<f32> {
        bytes
            .chunks_exact(4)
            .take(dimensions)
            .map(|chunk| f32::from_le_bytes([chunk[0], chunk[1], chunk[2], chunk[3]]))
            .collect()
    }

    /// Compute cosine similarity between two vectors.
    fn cosine_similarity(a: &[f32], b: &[f32]) -> f64 {
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
}

#[async_trait]
impl VectorStore for SqliteVectorStore {
    async fn upsert(
        &self,
        entity_id: &str,
        vector: &[f32],
        metadata: Option<VectorMetadata>,
    ) -> Result<(), VectorError> {
        let bytes = Self::encode(vector);
        let timestamp = chrono::Utc::now().to_rfc3339();

        let model = metadata.as_ref().map(|m| &*m.model).unwrap_or("default");

        let conn = self
            .store
            .conn
            .lock()
            .map_err(|e| VectorError::Storage(format!("lock error: {}", e)))?;

        conn.execute(
            "INSERT OR REPLACE INTO embeddings (entity_id, model, vector, dimensions, created_at)
             VALUES (?1, ?2, ?3, ?4, ?5)",
            params![entity_id, model, bytes, self.dimensions as i64, timestamp],
        )
        .map_err(|e| VectorError::Storage(format!("insert failed: {}", e)))?;

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

        let conn = self
            .store
            .conn
            .lock()
            .map_err(|e| VectorError::Storage(format!("lock error: {}", e)))?;

        let min_score = filter.as_ref().and_then(|f| f.min_score);
        let filter_entity_types = filter.as_ref().and_then(|f| f.entity_types.as_ref());

        // Get entity type for each entity ID via join
        let mut stmt = conn
            .prepare(
                "SELECT e.id, e.entity_type, em.vector, em.model
                 FROM embeddings em
                 JOIN entities e ON e.id = em.entity_id
                 WHERE em.dimensions = ?1",
            )
            .map_err(|e| VectorError::Storage(format!("prepare failed: {}", e)))?;

        let rows = stmt
            .query_map(params![self.dimensions as i64], |row| {
                let id: String = row.get(0)?;
                let entity_type: String = row.get(1)?;
                let bytes: Vec<u8> = row.get(2)?;
                let model: String = row.get(3)?;
                Ok((id, entity_type, bytes, model))
            })
            .map_err(|e| VectorError::Storage(format!("query failed: {}", e)))?;

        let mut scored: Vec<VectorResult> = Vec::new();

        for row in rows.flatten() {
            let (id, entity_type, bytes, model) = row;

            // Apply entity type filter
            if let Some(types) = &filter_entity_types {
                if !types.iter().any(|t| t.as_str() == entity_type) {
                    continue;
                }
            }

            let vec = Self::decode(&bytes, self.dimensions);
            let score = Self::cosine_similarity(query, &vec);

            // Apply min_score filter
            if let Some(min) = min_score {
                if score < min {
                    continue;
                }
            }

            scored.push(VectorResult {
                entity_id: id,
                score,
                metadata: Some(VectorMetadata {
                    model,
                    entity_type,
                    title: String::new(),
                }),
            });
        }

        scored.sort_by(|a, b| {
            b.score
                .partial_cmp(&a.score)
                .unwrap_or(std::cmp::Ordering::Equal)
        });
        scored.truncate(k);

        Ok(scored)
    }

    async fn delete(&self, entity_id: &str) -> Result<(), VectorError> {
        let conn = self
            .store
            .conn
            .lock()
            .map_err(|e| VectorError::Storage(format!("lock error: {}", e)))?;

        conn.execute(
            "DELETE FROM embeddings WHERE entity_id = ?1",
            params![entity_id],
        )
        .map_err(|e| VectorError::Storage(format!("delete failed: {}", e)))?;

        Ok(())
    }

    async fn rebuild(&self) -> Result<(), VectorError> {
        let conn = self
            .store
            .conn
            .lock()
            .map_err(|e| VectorError::Storage(format!("lock error: {}", e)))?;

        conn.execute("DELETE FROM embeddings", [])
            .map_err(|e| VectorError::Storage(format!("rebuild failed: {}", e)))?;

        Ok(())
    }
}
