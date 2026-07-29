# Engineering

[Home](../../README.md) > [Documentation](../README.md) > Engineering

Testing, security, deployment, and practices. These documents define how the system is built, tested, operated, and shipped.

---

## Documents

### Development Practices

| Document                                           | Purpose                                                                                                                   |
| -------------------------------------------------- | ------------------------------------------------------------------------------------------------------------------------- |
| [testing-strategy.md](testing-strategy.md)         | Test philosophy and pipeline testing -- test pyramid, canonical tests, derivation tests, event tests, and plugin tests    |
| [engineering-handbook.md](engineering-handbook.md) | Git workflow, code review, CI/CD, debugging -- branch conventions, commit format, PR process, and local development setup |
| [product-requirements.md](product-requirements.md) | Product scope and functional/non-functional requirements -- Year 1 scope with priority and acceptance criteria            |

### Operations

| Document                                                 | Purpose                                                                                                             |
| -------------------------------------------------------- | ------------------------------------------------------------------------------------------------------------------- |
| [security.md](security.md)                               | Threat model and access control -- defense-in-depth, threat categories, authentication, and plugin signing          |
| [deployment.md](deployment.md)                           | Deployment models and configuration -- local, private cloud, and public cloud with configuration management         |
| [operational-runbooks.md](operational-runbooks.md)       | Operational procedures and incident response -- rebuild, migration, backup, health checks, and failure scenarios    |
| [infrastructure-handbook.md](infrastructure-handbook.md) | Provisioning, scaling, monitoring, CI/CD -- infrastructure specs, autoscaling, observability, and disaster recovery |

### Interfaces

| Document                                     | Purpose                                                                                                              |
| -------------------------------------------- | -------------------------------------------------------------------------------------------------------------------- |
| [api-specification.md](api-specification.md) | REST and MCP API surfaces -- entity and relationship CRUD, search, rendering, and AI agent integration               |
| [ui-design-system.md](ui-design-system.md)   | Design tokens, component specs, accessibility -- colors, typography, spacing, components, and WCAG 2.1 AA compliance |

### Implementation Plans

| Plan                                                         | Phase | ADR(s)                                 | Purpose                                                                  |
| ------------------------------------------------------------ | ----- | -------------------------------------- | ------------------------------------------------------------------------ |
| [IP-001](implementation-plans/IP-001-graph-traversal.md)    | 1     | ADR-0014                              | Graph traversal via recursive CTE, bounded depth, cycle detection        |
| [IP-002](implementation-plans/IP-002-view-projections.md)   | 2     | ADR-0015                              | Tree, graph, table, timeline views; synchronization; filtering           |
| [IP-003](implementation-plans/IP-003-plugin-system.md)      | 3     | ADR-0016                              | Plugin infrastructure, manifest parsing, capability registry, sandboxing |
| [IP-004](implementation-plans/IP-004-semantic-search.md)    | 4     | ADR-0017                              | Embedding generation, in-memory vector store, hybrid search via RRF      |
| [IP-005](implementation-plans/IP-005-collections.md)        | 5     | ADR-0018                              | Collection entity, membership, tree view integration                     |
| [IP-006](implementation-plans/IP-006-integration-and-testing.md) | 6 | ADR-0014–0018                     | Cross-plan integration tests and BDD scenarios                           |
| [IP-007](implementation-plans/IP-007-traversal-performance-optimization.md) | — | ADR-0019–0020            | Composite indexes and BFS optimization for graph traversal               |
| [IP-008](implementation-plans/IP-008-desktop-mvp.md)        | 7     | ADR-0021–0022                         | Tauri desktop MVP: IPC backend, 11 commands, 9 views, design system      |
| [IP-009](implementation-plans/IP-009-chat-completion-port.md) | 8   | ADR-0023                              | ChatCompletion port trait and built-in LLM adapters (Mock, OpenAI, Ollama) |
| [IP-010](implementation-plans/IP-010-conversation-message-entities.md) | 8 | ADR-0024                          | Conversation and Message as canonical entities with typed components     |
| [IP-011](implementation-plans/IP-011-chat-pipeline.md)      | 8     | ADR-0023, ADR-0025                   | Chat pipeline with RAG, streaming, citations, status emission           |
| [IP-012](implementation-plans/IP-012-ocr-backend.md)        | 8     | ADR-0026                              | Pluggable OCR backend (Tesseract, Ollama, API, Mock)                     |
| [IP-013](implementation-plans/IP-013-universal-import.md)   | 8     | ADR-0027                              | Universal import: office, email, databases, URL, clipboard, column mapping |
| [IP-014](implementation-plans/IP-014-mcp-service-architecture.md) | 8 | ADR-0028                          | MCP-compatible EntityRetrievalService and framework-agnostic chat pipeline |

---

## Note

The `engineering/` directory contains mixed Diataxis types. Each file's type is determined by its content, not its location. Some files are reference specifications, others are how-to guides, and others are explanations.
