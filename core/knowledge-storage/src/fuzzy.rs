use knowledge_core::features::entity::Entity;
use knowledge_core::ports::ResolutionCandidate;
use std::collections::HashMap;

/// Configuration for per-entity-type resolution behavior.
#[derive(Debug, Clone)]
pub struct ResolutionConfig {
    /// Default threshold when entity type has no specific override
    pub default_threshold: f64,
    /// Per-entity-type threshold overrides
    pub type_thresholds: HashMap<String, f64>,
    /// Default strategy pipeline when entity type has no specific override
    pub default_strategies: Vec<ResolutionStrategy>,
    /// Per-entity-type strategy pipeline overrides
    pub type_strategies: HashMap<String, Vec<ResolutionStrategy>>,
}

impl Default for ResolutionConfig {
    fn default() -> Self {
        Self {
            default_threshold: 0.95,
            type_thresholds: HashMap::new(),
            default_strategies: vec![
                ResolutionStrategy::Exact,
                ResolutionStrategy::Normalized,
                ResolutionStrategy::Fuzzy { threshold: 0.95 },
                // PONYTAIL: ContentMatch not in default pipeline. Ceiling: Jaccard word overlap is
                // naive — misses paraphrases, synonyms, restructured text. Upgrade: embedding-based
                // semantic similarity (PRD-0003) or TF-IDF cosine.
            ],
            type_strategies: HashMap::new(),
        }
    }
}

impl ResolutionConfig {
    /// Get the threshold for a specific entity type
    pub fn threshold_for(&self, entity_type: &str) -> f64 {
        self.type_thresholds
            .get(entity_type)
            .copied()
            .unwrap_or(self.default_threshold)
    }

    /// Get the strategies for a specific entity type
    pub fn strategies_for(&self, entity_type: &str) -> &[ResolutionStrategy] {
        self.type_strategies
            .get(entity_type)
            .map(|s| s.as_slice())
            .unwrap_or(&self.default_strategies)
    }
}

/// Normalize a title for comparison: lowercase, collapse whitespace, strip punctuation
pub fn normalize_title(title: &str) -> String {
    let mut result = String::new();
    let mut prev_was_space = false;

    for c in title.chars() {
        if c.is_alphanumeric() {
            result.push(c.to_ascii_lowercase());
            prev_was_space = false;
        } else if !prev_was_space && !result.is_empty() {
            result.push(' ');
            prev_was_space = true;
        }
    }

    result.trim().to_string()
}

/// Strategy for producing resolution candidates with confidence scores
#[derive(Debug, Clone)]
pub enum ResolutionStrategy {
    /// Exact match: title + entity type equality (confidence: 1.0)
    Exact,
    /// Normalized match: lowercase + whitespace normalize (confidence: 0.95)
    Normalized,
    /// Fuzzy match: Levenshtein / Jaro-Winkler distance-based (confidence: 0.0-1.0)
    Fuzzy { threshold: f64 },
    /// Content match: body text similarity via Jaccard word overlap (confidence: 0.0-1.0)
    ContentMatch { threshold: f64 },
}

impl ResolutionStrategy {
    /// Compute confidence for two titles (and optional body texts) based on this strategy
    pub fn compute_confidence(
        &self,
        title_a: &str,
        title_b: &str,
        content_a: Option<&str>,
        content_b: Option<&str>,
    ) -> Option<f64> {
        match self {
            ResolutionStrategy::Exact => {
                if title_a == title_b {
                    Some(1.0)
                } else {
                    None
                }
            }
            ResolutionStrategy::Normalized => {
                let norm_a = normalize_title(title_a);
                let norm_b = normalize_title(title_b);
                if norm_a == norm_b {
                    Some(0.95)
                } else {
                    None
                }
            }
            ResolutionStrategy::Fuzzy { threshold } => {
                let norm_a = normalize_title(title_a);
                let norm_b = normalize_title(title_b);

                // Skip fuzzy matching for short titles (< 10 chars) to avoid false positives
                if norm_a.len() < 10 || norm_b.len() < 10 {
                    return None;
                }

                // Use Jaro-Winkler for fuzzy matching (higher is more similar)
                let similarity = strsim::jaro_winkler(&norm_a, &norm_b);

                // Also compute Levenshtein distance for alternative comparison
                let max_len = std::cmp::max(norm_a.len(), norm_b.len());
                let lev_distance = strsim::levenshtein(&norm_a, &norm_b);
                let lev_similarity = if max_len > 0 {
                    1.0 - (lev_distance as f64 / max_len as f64)
                } else {
                    1.0
                };

                // Use the higher of the two similarity scores
                let best_similarity = similarity.max(lev_similarity);

                if best_similarity >= *threshold {
                    Some(best_similarity)
                } else {
                    None
                }
            }
            ResolutionStrategy::ContentMatch { threshold } => {
                // Both must have content for content matching
                let text_a = content_a?;
                let text_b = content_b?;

                if text_a.is_empty() || text_b.is_empty() {
                    return None;
                }

                let similarity = jaccard_similarity(text_a, text_b);

                if similarity >= *threshold {
                    Some(similarity)
                } else {
                    None
                }
            }
        }
    }
}

/// Compute Jaccard similarity between two texts (word-level overlap)
fn jaccard_similarity(text_a: &str, text_b: &str) -> f64 {
    let words_a: std::collections::HashSet<&str> = text_a.split_whitespace().collect();
    let words_b: std::collections::HashSet<&str> = text_b.split_whitespace().collect();

    if words_a.is_empty() && words_b.is_empty() {
        return 1.0;
    }

    let intersection: usize = words_a.intersection(&words_b).count();
    let union = words_a.len() + words_b.len() - intersection;

    if union == 0 {
        0.0
    } else {
        intersection as f64 / union as f64
    }
}

/// Composable entity resolver with strategies in order of increasing cost
pub struct FuzzyEntityResolver {
    config: ResolutionConfig,
}

impl FuzzyEntityResolver {
    /// Create resolver with default config (threshold: 0.95)
    pub fn new() -> Self {
        Self {
            config: ResolutionConfig::default(),
        }
    }

    /// Create resolver with custom config
    pub fn with_config(config: ResolutionConfig) -> Self {
        Self { config }
    }

    /// Create resolver with custom default threshold
    pub fn with_threshold(threshold: f64) -> Self {
        let config = ResolutionConfig {
            default_threshold: threshold,
            default_strategies: vec![
                ResolutionStrategy::Exact,
                ResolutionStrategy::Normalized,
                ResolutionStrategy::Fuzzy { threshold },
            ],
            ..ResolutionConfig::default()
        };
        Self { config }
    }

    /// Find candidates using the first matching strategy (ordered by cost)
    pub fn find_candidates(
        &self,
        incoming_entity: &Entity,
        incoming_title: &str,
        incoming_content: Option<&str>,
        existing_entities: &[(Entity, String, Option<String>)], // (entity, title, content)
    ) -> Vec<ResolutionCandidate> {
        let entity_type_str = incoming_entity.entity_type.as_str();
        let strategies = self.config.strategies_for(entity_type_str);
        let threshold = self.config.threshold_for(entity_type_str);

        let mut candidates = Vec::new();

        for strategy in strategies {
            for (entity, title, content) in existing_entities {
                if entity.entity_type != incoming_entity.entity_type {
                    continue;
                }

                // For fuzzy strategy, use the per-type threshold
                let effective_strategy = match strategy {
                    ResolutionStrategy::Fuzzy { .. } => ResolutionStrategy::Fuzzy { threshold },
                    ResolutionStrategy::ContentMatch { .. } => {
                        ResolutionStrategy::ContentMatch { threshold }
                    }
                    other => other.clone(),
                };

                if let Some(confidence) = effective_strategy.compute_confidence(
                    incoming_title,
                    title,
                    incoming_content,
                    content.as_deref(),
                ) {
                    let reason = match strategy {
                        ResolutionStrategy::Exact => "Exact match: title + entity type".to_string(),
                        ResolutionStrategy::Normalized => {
                            "Normalized match: case/whitespace normalized".to_string()
                        }
                        ResolutionStrategy::Fuzzy { .. } => {
                            format!(
                                "Fuzzy match: Jaro-Winkler similarity >= {:.2}",
                                confidence
                            )
                        }
                        ResolutionStrategy::ContentMatch { .. } => {
                            format!(
                                "Content match: Jaccard similarity >= {:.2}",
                                confidence
                            )
                        }
                    };

                    candidates.push(ResolutionCandidate {
                        entity_id: entity.id,
                        confidence,
                        reason,
                    });
                }
            }

            // Stop at first strategy that produces candidates (ordered by cost)
            if !candidates.is_empty() {
                break;
            }
        }

        // Sort by confidence descending (highest first)
        candidates.sort_by(|a, b| b.confidence.partial_cmp(&a.confidence).unwrap());

        candidates
    }
}

impl Default for FuzzyEntityResolver {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::Utc;
    use uuid::Uuid;
    use knowledge_core::features::entity::EntityType;

    fn make_entity(title: &str, entity_type: EntityType) -> (Entity, String, Option<String>) {
        (
            Entity {
                id: Uuid::new_v4(),
                entity_type,
                is_active: true,
                created_at: Utc::now(),
                updated_at: Utc::now(),
                version: 1,
            },
            title.to_string(),
            None, // no content by default
        )
    }

    #[test]
    fn test_normalize_title() {
        assert_eq!(normalize_title("Hello World"), "hello world");
        assert_eq!(normalize_title("  Hello   World  "), "hello world");
        assert_eq!(normalize_title("HELLO WORLD!"), "hello world");
        assert_eq!(normalize_title("Attention Is All You Need"), "attention is all you need");
        assert_eq!(
            normalize_title("Attention Is All You Need (2017)"),
            "attention is all you need 2017"
        );
    }

    #[test]
    fn test_exact_strategy() {
        let strategy = ResolutionStrategy::Exact;
        assert_eq!(strategy.compute_confidence("Hello", "Hello", None, None), Some(1.0));
        assert_eq!(strategy.compute_confidence("Hello", "hello", None, None), None);
        assert_eq!(strategy.compute_confidence("Hello", "World", None, None), None);
    }

    #[test]
    fn test_normalized_strategy() {
        let strategy = ResolutionStrategy::Normalized;
        assert_eq!(
            strategy.compute_confidence("Hello World", "hello world", None, None),
            Some(0.95)
        );
        assert_eq!(
            strategy.compute_confidence("Hello  World", "hello world", None, None),
            Some(0.95)
        );
        assert_eq!(strategy.compute_confidence("Hello", "World", None, None), None);
    }

    #[test]
    fn test_fuzzy_strategy() {
        let strategy = ResolutionStrategy::Fuzzy { threshold: 0.95 };

        // High similarity
        assert!(
            strategy
                .compute_confidence(
                    "Attention Is All You Need",
                    "Attention Is All You Need (2017)",
                    None,
                    None,
                )
                .unwrap()
                > 0.95
        );

        // Low similarity
        assert_eq!(
            strategy.compute_confidence("Hello World", "Completely Different", None, None),
            None
        );
    }

    #[test]
    fn test_content_match_strategy() {
        let strategy = ResolutionStrategy::ContentMatch { threshold: 0.5 };

        // Same content
        assert_eq!(
            strategy.compute_confidence(
                "Different Title",
                "Another Title",
                Some("the quick brown fox"),
                Some("the quick brown fox"),
            ),
            Some(1.0)
        );

        // Partially overlapping content
        let confidence = strategy.compute_confidence(
            "Title A",
            "Title B",
            Some("the quick brown fox jumps over the lazy dog"),
            Some("the quick brown fox jumps over the lazy cat"),
        );
        assert!(confidence.unwrap() > 0.5);

        // No content provided
        assert_eq!(
            strategy.compute_confidence("Title A", "Title B", None, None),
            None
        );

        // Empty content
        assert_eq!(
            strategy.compute_confidence("Title A", "Title B", Some(""), Some("")),
            None
        );
    }

    #[test]
    fn test_fuzzy_resolver_exact_match() {
        let resolver = FuzzyEntityResolver::new();
        let article = EntityType::new("Article");

        let existing = vec![
            make_entity("Hello World", article.clone()),
            make_entity("Another Document", article.clone()),
        ];

        let incoming = make_entity("Hello World", article.clone());
        let candidates = resolver.find_candidates(&incoming.0, "Hello World", None, &existing);

        assert_eq!(candidates.len(), 1);
        assert_eq!(candidates[0].confidence, 1.0);
        assert!(candidates[0].reason.contains("Exact"));
    }

    #[test]
    fn test_fuzzy_resolver_normalized_match() {
        let resolver = FuzzyEntityResolver::new();
        let article = EntityType::new("Article");

        let existing = vec![
            make_entity("Hello World", article.clone()),
            make_entity("Another Document", article.clone()),
        ];

        let incoming = make_entity("hello world", article.clone());
        let candidates = resolver.find_candidates(&incoming.0, "hello world", None, &existing);

        assert_eq!(candidates.len(), 1);
        assert_eq!(candidates[0].confidence, 0.95);
        assert!(candidates[0].reason.contains("Normalized"));
    }

    #[test]
    fn test_fuzzy_resolver_fuzzy_match() {
        let resolver = FuzzyEntityResolver::new();
        let article = EntityType::new("Article");

        let existing = vec![
            make_entity(
                "Attention Is All You Need",
                article.clone(),
            ),
            make_entity("Another Document", article.clone()),
        ];

        // Fuzzy variant with extra words
        let incoming = make_entity(
            "Attention Is All You Need (2017)",
            article.clone(),
        );
        let candidates = resolver.find_candidates(&incoming.0, "Attention Is All You Need (2017)", None, &existing);

        assert_eq!(candidates.len(), 1);
        assert!(candidates[0].confidence > 0.85);
        assert!(candidates[0].reason.contains("Fuzzy"));
    }

    #[test]
    fn test_fuzzy_resolver_no_match() {
        let resolver = FuzzyEntityResolver::new();
        let article = EntityType::new("Article");

        let existing = vec![make_entity(
            "Completely Different",
            article.clone(),
        )];

        let incoming = make_entity("Hello World", article.clone());
        let candidates = resolver.find_candidates(&incoming.0, "Hello World", None, &existing);

        assert!(candidates.is_empty());
    }

    #[test]
    fn test_fuzzy_resolver_different_types_no_match() {
        let resolver = FuzzyEntityResolver::new();
        let article = EntityType::new("Article");
        let person = EntityType::new("Person");

        let existing = vec![make_entity("John Smith", person.clone())];

        let incoming = make_entity("John Smith", article.clone());
        let candidates = resolver.find_candidates(&incoming.0, "John Smith", None, &existing);

        assert!(candidates.is_empty());
    }

    #[test]
    fn test_fuzzy_resolver_short_title_no_fuzzy_match() {
        let resolver = FuzzyEntityResolver::new();
        let article = EntityType::new("Article");

        let existing = vec![make_entity("Doc A", article.clone())];

        // "Doc B" is short (< 10 chars) — should NOT fuzzy match
        let incoming = make_entity("Doc B", article.clone());
        let candidates = resolver.find_candidates(&incoming.0, "Doc B", None, &existing);

        assert!(candidates.is_empty(), "Short titles should not fuzzy match");
    }

    #[test]
    fn test_fuzzy_resolver_sorted_by_confidence() {
        let resolver = FuzzyEntityResolver::new();
        let article = EntityType::new("Article");

        let existing = vec![
            make_entity("Hello World", article.clone()),
            make_entity("Hello World Again", article.clone()),
        ];

        let incoming = make_entity("Hello World", article.clone());
        let candidates = resolver.find_candidates(&incoming.0, "Hello World", None, &existing);

        // Should have candidates from both strategies
        assert!(!candidates.is_empty());
        // Highest confidence first
        assert!(candidates[0].confidence >= candidates.last().unwrap().confidence);
    }

    #[test]
    fn test_resolution_config_per_type_threshold() {
        let mut type_thresholds = HashMap::new();
        type_thresholds.insert("Person".to_string(), 0.80);
        type_thresholds.insert("Article".to_string(), 0.99);

        let config = ResolutionConfig {
            default_threshold: 0.95,
            type_thresholds,
            ..Default::default()
        };

        assert_eq!(config.threshold_for("Person"), 0.80);
        assert_eq!(config.threshold_for("Article"), 0.99);
        assert_eq!(config.threshold_for("Concept"), 0.95); // falls back to default
    }

    #[test]
    fn test_resolution_config_per_type_strategies() {
        let mut type_strategies = HashMap::new();
        // Person: only exact and normalized (no fuzzy)
        type_strategies.insert(
            "Person".to_string(),
            vec![
                ResolutionStrategy::Exact,
                ResolutionStrategy::Normalized,
            ],
        );

        let config = ResolutionConfig {
            type_strategies,
            ..Default::default()
        };

        let person_strategies = config.strategies_for("Person");
        assert_eq!(person_strategies.len(), 2);
        assert!(matches!(person_strategies[0], ResolutionStrategy::Exact));
        assert!(matches!(person_strategies[1], ResolutionStrategy::Normalized));

        // Article uses defaults (3 strategies)
        let article_strategies = config.strategies_for("Article");
        assert_eq!(article_strategies.len(), 3);
    }

    #[test]
    fn test_fuzzy_resolver_per_type_threshold() {
        let mut type_thresholds = HashMap::new();
        type_thresholds.insert("Person".to_string(), 0.80); // lower threshold for Person

        let config = ResolutionConfig {
            default_threshold: 0.99,
            type_thresholds,
            ..Default::default()
        };

        let resolver = FuzzyEntityResolver::with_config(config);
        let person = EntityType::new("Person");
        let article = EntityType::new("Article");

        let existing = vec![
            make_entity("Johnathan Smith", person.clone()),
            make_entity("Attention Is All You Need", article.clone()),
        ];

        // Person with lower threshold should match at 0.80
        let incoming_person = make_entity("Jonathan Smith", person.clone());
        let candidates = resolver.find_candidates(&incoming_person.0, "Jonathan Smith", None, &existing);
        // Should match "Johnathan Smith" via fuzzy at 0.80 threshold
        assert!(!candidates.is_empty(), "Person should match with lower threshold");

        // Article with 0.99 threshold should NOT fuzzy match
        let incoming_article = make_entity(
            "Attention Is All You Need (2017)",
            article.clone(),
        );
        let candidates = resolver.find_candidates(
            &incoming_article.0,
            "Attention Is All You Need (2017)",
            None,
            &existing,
        );
        // Should not fuzzy match because threshold is 0.99
        assert!(candidates.is_empty(), "Article should not match with 0.99 threshold");
    }
}
