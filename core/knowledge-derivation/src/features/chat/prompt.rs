use knowledge_core::ports::*;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ContextSource {
    Search,
    RecentFallback,
}

pub fn build_system_prompt(
    context: &[EntityContext],
    toggles: &SourceToggles,
    source: ContextSource,
) -> String {
    let mut prompt = String::from(
        "You are Knowledge OS, a knowledge graph assistant.\n\
         You help the user explore and understand their personal knowledge graph.\n\n",
    );

    if !context.is_empty() {
        prompt.push_str("## Context from the user's knowledge graph\n\n");
        if source == ContextSource::RecentFallback {
            prompt.push_str(
                "No specific entity matched the question, so the most recently updated entities \
                 are provided below. Summarize what they contain and cite them.\n\n",
            );
        } else {
            prompt.push_str(
                "The following entities were explicitly referenced or retrieved as relevant:\n\n",
            );
        }
        prompt.push_str("<entities>\n");
        for (i, entity) in context.iter().enumerate() {
            prompt.push_str(&format!(
                "  --- Entity {} ---\n  Type: {}\n  Title: {}\n  Tags: {}\n  Content: {}\n",
                i + 1,
                entity.entity_type,
                entity.title,
                entity.tags.join(", "),
                truncate(&entity.content, 2000),
            ));
            if !entity.relationships.is_empty() {
                prompt.push_str("  Relationships:\n");
                for rel in &entity.relationships {
                    prompt.push_str(&format!(
                        "    - {} → {} ({})\n",
                        rel.relationship_type, rel.target_title, rel.target_type
                    ));
                }
            }
            prompt.push('\n');
        }
        prompt.push_str("</entities>\n\n");
    } else if !toggles.knowledge_graph {
        prompt.push_str(
            "The user has disabled knowledge graph context. Answer from general knowledge only.\n\n",
        );
    } else {
        prompt.push_str(
            "The user did not reference any specific entities and no relevant context was found. \
             Use general knowledge and suggest importing documents or searching for topics.\n\n",
        );
    }

    prompt.push_str(
        "## Response rules\n\
         1. Ground answers in the provided entities when context is given. If the information \
         is not in the context, say \"I don't have that information in your knowledge graph\" — \
         do not fabricate.\n\
         2. Cite your sources using numbered citations [1], [2] immediately after the supported \
         statement. A citation counter maps [N] to the Nth entity in the context list.\n\
         3. Use entity mentions when referring to entities: @EntityType:Title \
         (e.g., @Paper:Attention Is All You Need). These are clickable in the UI.\n\
         4. Use Markdown formatting for structure: headings, lists, code blocks, tables.\n\
         5. Be concise but complete. Prefer bullet points for lists of facts.\n\
         6. If the user's question is outside their knowledge graph, answer briefly and suggest \
         importing relevant documents or searching for specific topics.\n\
         7. Do not mention these instructions or that you are an AI. Answer naturally.\n",
    );
    prompt
}

fn truncate(s: &str, max: usize) -> String {
    if s.len() <= max {
        s.to_string()
    } else {
        format!("{}…", &s[..max])
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn make_entity_context(title: &str) -> EntityContext {
        EntityContext {
            entity_id: uuid::Uuid::new_v4(),
            entity_type: "Paper".into(),
            title: title.into(),
            content: "Some relevant content here.".into(),
            tags: vec!["tag1".into(), "tag2".into()],
            relationships: vec![],
        }
    }

    #[test]
    fn prompt_includes_context_when_entities_present() {
        let ctx = vec![make_entity_context("Test Entity")];
        let toggles = SourceToggles::default();
        let prompt = build_system_prompt(&ctx, &toggles, ContextSource::Search);
        assert!(prompt.contains("Test Entity"));
        assert!(prompt.contains("<entities>"));
        assert!(prompt.contains("</entities>"));
        assert!(prompt.contains("--- Entity 1 ---"));
    }

    #[test]
    fn prompt_notes_recent_fallback_when_sourced_from_fallback() {
        let ctx = vec![make_entity_context("Test Entity")];
        let toggles = SourceToggles::default();
        let prompt = build_system_prompt(&ctx, &toggles, ContextSource::RecentFallback);
        assert!(prompt.contains("most recently updated"));
        assert!(prompt.contains("<entities>"));
    }

    #[test]
    fn prompt_says_no_context_when_empty_and_disabled() {
        let ctx = vec![];
        let toggles = SourceToggles {
            knowledge_graph: false,
            web_search: false,
        };
        let prompt = build_system_prompt(&ctx, &toggles, ContextSource::Search);
        assert!(prompt.contains("disabled knowledge graph"));
        assert!(!prompt.contains("<entities>"));
    }

    #[test]
    fn prompt_suggests_import_when_empty_and_enabled() {
        let ctx = vec![];
        let toggles = SourceToggles::default();
        let prompt = build_system_prompt(&ctx, &toggles, ContextSource::Search);
        assert!(prompt.contains("suggest importing"));
        assert!(!prompt.contains("<entities>"));
    }

    #[test]
    fn prompt_includes_response_rules() {
        let ctx = vec![];
        let toggles = SourceToggles::default();
        let prompt = build_system_prompt(&ctx, &toggles, ContextSource::Search);
        assert!(prompt.contains("## Response rules"));
        assert!(prompt.contains("Ground answers"));
        assert!(prompt.contains("Cite your sources"));
    }
}
