use std::collections::HashMap;

use knowledge_core::ports::{FusedResult, SearchResult, VectorResult};

/// Combine keyword (BM25) and semantic (cosine) search results using
/// Reciprocal Rank Fusion (RRF).
///
/// RRF operates on ranks rather than raw scores, which is important because
/// BM25 scores and cosine similarity scores are on different scales.
///
/// The constant `k` controls how much weight is given to lower-ranked results.
/// A typical value is 60 (used by Elasticsearch).
pub fn reciprocal_rank_fusion(
    keyword_results: &[SearchResult],
    semantic_results: &[VectorResult],
    k: usize,
) -> Vec<FusedResult> {
    let mut scores: HashMap<String, f64> = HashMap::new();

    for (rank, result) in keyword_results.iter().enumerate() {
        let id = result.entity_id.to_string();
        *scores.entry(id).or_insert(0.0) += 1.0 / (k + rank + 1) as f64;
    }
    for (rank, result) in semantic_results.iter().enumerate() {
        *scores.entry(result.entity_id.clone()).or_insert(0.0) += 1.0 / (k + rank + 1) as f64;
    }

    let mut fused: Vec<FusedResult> = scores
        .into_iter()
        .map(|(id, score)| FusedResult {
            entity_id: id,
            score,
        })
        .collect();
    fused.sort_by(|a, b| {
        b.score
            .partial_cmp(&a.score)
            .unwrap_or(std::cmp::Ordering::Equal)
    });
    fused
}

#[cfg(test)]
mod tests {
    use super::*;

    fn keyword_result(uuid: &str) -> SearchResult {
        SearchResult {
            entity_id: uuid.parse().unwrap(),
            score: 1.0,
            confidence: None,
            snippet: None,
        }
    }

    fn semantic_result(id: &str) -> VectorResult {
        VectorResult {
            entity_id: id.to_string(),
            score: 0.8,
            metadata: None,
        }
    }

    // Use valid UUIDs for tests
    const ID_A: &str = "aaaaaaaa-aaaa-aaaa-aaaa-aaaaaaaaaaaa";
    const ID_B: &str = "bbbbbbbb-bbbb-bbbb-bbbb-bbbbbbbbbbbb";
    const ID_C: &str = "cccccccc-cccc-cccc-cccc-cccccccccccc";

    #[test]
    fn fusion_merges_keyword_and_semantic_results() {
        let kw = vec![keyword_result(ID_A)];
        let sem = vec![semantic_result(ID_B)];
        let fused = reciprocal_rank_fusion(&kw, &sem, 60);
        assert_eq!(fused.len(), 2);
    }

    #[test]
    fn entity_in_both_lists_ranks_higher() {
        let kw = vec![keyword_result(ID_B), keyword_result(ID_A)];
        let sem = vec![semantic_result(ID_C), semantic_result(ID_A)];
        let fused = reciprocal_rank_fusion(&kw, &sem, 60);
        // Entity in both lists should have higher score than entities in only one
        let shared = fused.iter().find(|f| f.entity_id == ID_A).unwrap();
        let only_kw = fused.iter().find(|f| f.entity_id == ID_B).unwrap();
        assert!(shared.score > only_kw.score);
    }

    #[test]
    fn empty_input_lists_handled_gracefully() {
        let fused = reciprocal_rank_fusion(&[], &[], 60);
        assert!(fused.is_empty());
    }

    #[test]
    fn fusion_scores_in_valid_range() {
        let kw = vec![keyword_result(ID_A), keyword_result(ID_B)];
        let sem = vec![semantic_result(ID_A), semantic_result(ID_C)];
        let fused = reciprocal_rank_fusion(&kw, &sem, 60);
        for result in &fused {
            assert!(result.score >= 0.0, "score should be >= 0");
            assert!(result.score <= 1.0, "score should be <= 1");
        }
    }

    #[test]
    fn results_sorted_by_score_descending() {
        let kw = vec![keyword_result(ID_A), keyword_result(ID_B)];
        let sem = vec![semantic_result(ID_A), semantic_result(ID_C)];
        let fused = reciprocal_rank_fusion(&kw, &sem, 60);
        for window in fused.windows(2) {
            assert!(window[0].score >= window[1].score);
        }
    }

    #[test]
    fn only_keyword_results() {
        let kw = vec![keyword_result(ID_A), keyword_result(ID_B)];
        let fused = reciprocal_rank_fusion(&kw, &[], 60);
        assert_eq!(fused.len(), 2);
    }

    #[test]
    fn only_semantic_results() {
        let sem = vec![semantic_result(ID_A), semantic_result(ID_B)];
        let fused = reciprocal_rank_fusion(&[], &sem, 60);
        assert_eq!(fused.len(), 2);
    }
}
