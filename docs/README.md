# Knowledge OS Documentation

This directory contains all project documentation, organized following the [Diataxis framework](https://diataxis.fr/).

---

## Structure

```
docs/
├── foundational-manifesto.md
├── engineering-architecture.md
│
├── philosophy/
│   ├── philosophy.md
│   ├── vision.md
│   ├── boundaries.md
│   ├── engineering-principles.md
│   ├── product-vision.md
│   └── governance.md
│
├── architecture/
│   ├── overview.md
│   ├── mental-model.md
│   ├── pipeline.md
│   ├── data-model.md
│   ├── storage.md
│   ├── composition.md
│   ├── compilation.md
│   ├── events.md
│   ├── domain-model.md
│   ├── ai.md
│   ├── ui-philosophy.md
│   ├── extensibility.md
│   ├── scalability.md
│   ├── synchronization.md
│   └── adrs/
│
├── reference/
│   └── glossary.md
│
├── engineering/
│   ├── testing-strategy.md
│   ├── security.md
│   ├── deployment.md
│   ├── engineering-handbook.md
│   ├── operational-runbooks.md
│   ├── product-requirements.md
│   ├── ui-design-system.md
│   ├── api-specification.md
│   └── infrastructure-handbook.md
│
├── guides/
│   ├── plugin-development.md
│   ├── ai-agent-guidelines.md
│   └── tutorials/
│
└── appendices.md
```

---

## Reading Order

For **new contributors**, read in this order:

1. [Seed Manifesto](foundational-manifesto.md) -- The constitutional foundation
2. [Technical Foundation](engineering-architecture.md) -- The engineering architecture
3. [Philosophy](philosophy/philosophy.md) -- Core philosophy
4. [Vision](philosophy/vision.md) -- Why Knowledge OS exists
5. [Boundaries](philosophy/boundaries.md) -- What we build and what we skip
6. [System Overview](architecture/overview.md) -- Technical overview

For **architects and engineers**, also read:

7. [Mental Model](architecture/mental-model.md) -- The conceptual foundation
8. [Domain Model](architecture/domain-model.md) -- Entity, relationship, and component types
9. [Pipeline](architecture/pipeline.md) -- The seven-layer pipeline
10. [Data Model](architecture/data-model.md) -- Canonical vs derived data
11. [Storage](architecture/storage.md) -- Persistence strategy
12. [Composition](architecture/composition.md) -- Entity component model
13. [Compilation](architecture/compilation.md) -- Pipeline model
14. [Events](architecture/events.md) -- Event system
15. [AI](architecture/ai.md) -- AI integration
16. [Scalability](architecture/scalability.md) -- Scaling strategies
17. [Synchronization](architecture/synchronization.md) -- Consistency model

For **product managers and stakeholders**, also read:

18. [Product Vision](philosophy/product-vision.md) -- Long-term direction
19. [Governance](philosophy/governance.md) -- Decision-making process
20. [Engineering Principles](philosophy/engineering-principles.md) -- How code is developed

For **plugin developers**, also read:

21. [Extensibility](architecture/extensibility.md) -- Plugin system
22. [Plugin Development Guide](guides/plugin-development.md) -- How to build plugins
23. [AI Agent Guidelines](guides/ai-agent-guidelines.md) -- How AI agents work

For **learners**, start with:

24. [Tutorial: First Import](guides/tutorials/first-import.md) -- Import your first document
25. [Tutorial: Build a Custom Importer](guides/tutorials/build-custom-importer.md) -- Extend the system

---

## Document Types

Following [Diataxis](https://diataxis.fr/):

| Type | Purpose | Locations |
|------|---------|-----------|
| **Explanation** | Understanding and context | `philosophy/`, `architecture/`, `engineering/deployment.md`, `engineering/testing-strategy.md` |
| **Reference** | Factual specifications | `reference/`, `engineering/security.md`, `engineering/product-requirements.md`, `engineering/ui-design-system.md`, `engineering/api-specification.md`, `engineering/infrastructure-handbook.md`, `engineering/testing-strategy.md` |
| **How-to** | Task-oriented guides | `guides/`, `engineering/operational-runbooks.md`, `engineering/engineering-handbook.md` |
| **Tutorial** | Learning experiences | `guides/tutorials/` |

> **Note:** The `engineering/` directory contains mixed Diataxis types. Each file's type is determined by its content, not its location.
