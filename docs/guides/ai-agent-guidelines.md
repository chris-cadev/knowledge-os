# AI Agent Guidelines

> AI agents suggest. Humans decide. Every agent output is reviewable, versioned, and replaceable.

---

## Agent Model

AI agents in Knowledge OS are autonomous components that interact with the knowledge graph. They differ from automation in that agents make decisions, while automation follows rules.

### Agent Principles

1. **Agents suggest.** Agent outputs are proposals. A human approves or rejects each proposal.
2. **Agents are scoped.** Each agent operates within a defined scope: specific entity types, relationship types, or graph regions.
3. **Agents are auditable.** Every agent action is logged: what was proposed, what was approved, what was rejected.
4. **Agents are replaceable.** Agent implementations are adapters. Replacing an agent never changes the knowledge model.
5. **Agents have budgets.** Each agent has resource limits: maximum entities processed, maximum context size, maximum execution time.

---

## Agent Types

### Research Agent

Given a topic, the research agent:

1. Retrieves relevant entities from the knowledge graph.
2. Identifies gaps in knowledge (entities that are referenced but not fully described).
3. Suggests entities to import from external sources.
4. Proposes relationships between discovered entities.

**Scope:** Read access to the knowledge graph. Write access to draft entities.

### Organization Agent

The organization agent maintains the structure of the knowledge graph:

1. Suggests entity classifications (type assignments).
2. Suggests tag assignments for entities.
3. Identifies entities that may be misclassified.
4. Proposes collection groupings.

**Scope:** Read access to the knowledge graph. Write access to entity metadata.

### Curation Agent

The curation agent maintains knowledge quality:

1. Identifies outdated entities (content that may have changed).
2. Suggests updates to entity content.
3. Recommends archiving of stale entities.
4. Detects conflicting information between entities.

**Scope:** Read access to the knowledge graph. Write access to entity status.

### Discovery Agent

The discovery agent finds connections:

1. Identifies unexpected relationships between distant parts of the graph.
2. Suggests relationship creation between semantically related entities.
3. Discovers clusters of related entities.
4. Highlights knowledge gaps in specific domains.

**Scope:** Read access to the knowledge graph. Write access to draft relationships.

---

## Agent Interaction Patterns

### Pattern 1: Human-Initiated, Agent-Responded

The user asks a question. The agent retrieves context, reasons over the knowledge graph, and generates a response.

```
User: "What papers influenced the transformer architecture?"
Agent: [retrieves entities, traverses relationships, synthesizes answer]
Agent: "Based on the knowledge graph, the transformer architecture was influenced by:
  1. 'Attention Is All You Need' (Vaswani et al., 2017) - the original paper
  2. 'Neural Machine Translation by Jointly Learning to Align and Translate' (Bahdanau et al., 2014) - attention mechanism
  3. 'Long Short-Term Memory' (Hochreiter & Schmidhuber, 1997) - sequence modeling
  ...
  Would you like me to create relationships between these entities?"
```

### Pattern 2: Agent-Initiated, Human-Decided

The agent monitors the knowledge graph and proposes actions.

```
Agent: "I noticed 3 entities that may be misclassified:
  1. 'PyTorch' is classified as Technology but may be better classified as Tool
  2. 'Deep Learning' is classified as Concept but may be better classified as Technology
  3. 'BERT' is classified as Concept but may be better classified as Tool
  
  Would you like me to reclassify these entities?"
User: "Reclassify PyTorch as Tool. Keep the others as-is."
Agent: "Done. PyTorch is now classified as Tool."
```

### Pattern 3: Agent-Monitored, Human-Reviewed

The agent continuously monitors the knowledge graph and generates periodic reports.

```
Agent Report (weekly):
  - 5 entities added this week
  - 12 relationships created
  - 3 entities flagged as outdated
  - 7 entities missing summaries
  - 2 conflicting classifications detected
  
  Recommended actions:
  1. Update 'TensorFlow 2.0' content (outdated)
  2. Add summary to 'GANs' concept
  3. Resolve conflict between 'Python' and 'Python 3' entities
```

---

## Agent Configuration

```toml
[agent.research]
enabled = true
model = "gpt-4"
max_context_entities = 50
max_traversal_depth = 3
budget = { max_tokens = 10000, max_entities = 100 }

[agent.organization]
enabled = true
model = "gpt-4"
auto_classify = false  # always require human approval
max_batch_size = 20

[agent.curation]
enabled = true
model = "gpt-4"
staleness_threshold_days = 90
check_interval_hours = 24

[agent.discovery]
enabled = true
model = "gpt-4"
min_semantic_similarity = 0.7
max_results = 20
```

---

## Agent Security

1. **Agents cannot create canonical entities without approval.** Agent-created entities are drafts, flagged for review.
2. **Agents cannot modify existing entities without approval.** Agent-suggested modifications are presented as proposals.
3. **Agents have read-only access by default.** Write access is explicitly granted per agent type.
4. **Agent actions are rate-limited.** Agents cannot process unlimited entities in a single session.
5. **Agent outputs are versioned.** Every agent output is tracked with its model version and timestamp.

---

## Agent Development

### Creating a Custom Agent

```rust
use knowledge_os::agent::{Agent, AgentContext, AgentOutput};

pub struct MyCustomAgent {
    config: MyAgentConfig,
}

#[async_trait]
impl Agent for MyCustomAgent {
    async fn execute(&self, context: &AgentContext) -> Result<AgentOutput> {
        // 1. Analyze the knowledge graph
        let entities = context.query(&self.config.scope).await?;
        
        // 2. Apply agent logic
        let suggestions = self.analyze(&entities).await?;
        
        // 3. Return suggestions for human review
        Ok(AgentOutput::Suggestions(suggestions))
    }
}
```

### Agent Testing

Agent behavior is verified through three test layers:

1. **Unit tests** validate agent logic in isolation.
2. **BDD tests** exercise the CLI end-to-end, verifying that agent-related commands produce expected output. Run with `cargo test --test cucumber -p knowledge-cli`.
3. **Integration tests** verify agent interactions with the knowledge graph.

```rust
#[tokio::test]
async fn test_my_agent() {
    let agent = MyCustomAgent::new(test_config());
    let context = create_test_context();
    let output = agent.execute(&context).await.unwrap();
    
    match output {
        AgentOutput::Suggestions(suggestions) => {
            assert!(!suggestions.is_empty());
        }
        _ => panic!("Expected suggestions"),
    }
}
```

---

## Further Reading

- [AI Architecture](../architecture/ai.md) -- AI integration in the pipeline
- [Extensibility](../architecture/extensibility.md) -- Plugin system for agents
- [Governance](../philosophy/governance.md) -- How agent decisions are governed
