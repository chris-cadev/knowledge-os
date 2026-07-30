use knowledge_core::ports::*;

pub fn extract_citations(response: &str, context: &[EntityContext]) -> Vec<CitationSource> {
    let mut citations = Vec::new();
    for (i, entity) in context.iter().enumerate() {
        let marker = format!("[{}]", i + 1);
        if response.contains(&marker) {
            citations.push(CitationSource {
                number: i + 1,
                entity_id: entity.entity_id,
                entity_type: entity.entity_type.clone(),
                title: entity.title.clone(),
                snippet: entity.content.chars().take(200).collect(),
            });
        }
    }
    citations
}

#[cfg(test)]
mod tests {
    use super::*;

    fn make_entity_context(title: &str) -> EntityContext {
        EntityContext {
            entity_id: uuid::Uuid::new_v4(),
            entity_type: "Paper".into(),
            title: title.into(),
            content: "This is the content of the entity that will be cited.".into(),
            tags: vec![],
            relationships: vec![],
        }
    }

    #[test]
    fn citations_extracts_all_marked_entities() {
        let ctx = vec![
            make_entity_context("Entity A"),
            make_entity_context("Entity B"),
        ];
        let response = "As shown in [1] and [2], this is correct.";
        let citations = extract_citations(response, &ctx);
        assert_eq!(citations.len(), 2);
        assert_eq!(citations[0].title, "Entity A");
        assert_eq!(citations[1].title, "Entity B");
    }

    #[test]
    fn citations_skips_unused_markers() {
        let ctx = vec![
            make_entity_context("Entity A"),
            make_entity_context("Entity B"),
            make_entity_context("Entity C"),
        ];
        let response = "Only [1] and [3] are cited.";
        let citations = extract_citations(response, &ctx);
        assert_eq!(citations.len(), 2);
        assert_eq!(citations[0].number, 1);
        assert_eq!(citations[1].number, 3);
    }

    #[test]
    fn citations_empty_for_no_markers() {
        let ctx = vec![make_entity_context("Entity A")];
        let response = "No citations here.";
        let citations = extract_citations(response, &ctx);
        assert!(citations.is_empty());
    }

    #[test]
    fn citations_respects_marker_order() {
        let ctx = vec![
            make_entity_context("Entity A"),
            make_entity_context("Entity B"),
            make_entity_context("Entity C"),
        ];
        let response = "See [3], then [1].";
        let citations = extract_citations(response, &ctx);
        assert_eq!(citations.len(), 2);
        assert_eq!(citations[0].number, 1);
        assert_eq!(citations[1].number, 3);
    }
}
