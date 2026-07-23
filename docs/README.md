# Knowledge OS Documentation

[Home](../README.md) > Documentation

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
│   ├── open-infrastructure.md
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
│   ├── architectural-principles.md
│   └── adrs/
│
├── reference/
│   └── glossary.md
│
├── research/
│   └── landscape-2026.md
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
│   ├── infrastructure-handbook.md
│   └── prds/
│       ├── prd-0001-core-entity-model.md
│       ├── prd-0002-rich-import-and-resolution.md
│       └── prd-0003-graph-exploration-and-plugins.md
│
├── guides/
│   ├── plugin-development.md
│   ├── ai-agent-guidelines.md
│   └── tutorials/
│       ├── first-import.md
│       └── build-custom-importer.md
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
6. [Open Infrastructure](philosophy/open-infrastructure.md) -- Why Knowledge OS is available to everyone
7. [System Overview](architecture/overview.md) -- Technical overview

For **architects and engineers**, also read:

8. [Mental Model](architecture/mental-model.md) -- The conceptual foundation
9. [Domain Model](architecture/domain-model.md) -- Entity, relationship, and component types
10. [Pipeline](architecture/pipeline.md) -- The seven-layer pipeline
11. [Data Model](architecture/data-model.md) -- Canonical vs derived data
12. [Storage](architecture/storage.md) -- Persistence strategy
13. [Composition](architecture/composition.md) -- Entity component model
14. [Compilation](architecture/compilation.md) -- Pipeline model
15. [Events](architecture/events.md) -- Event system
16. [AI](architecture/ai.md) -- AI integration
17. [Scalability](architecture/scalability.md) -- Scaling strategies
18. [Synchronization](architecture/synchronization.md) -- Consistency model
19. [ADR-0006: Entity Resolution](architecture/adrs/adr-0006.md) -- Critical quality layer
20. [ADR-0007: Multi-Format Import](architecture/adrs/adr-0007.md) -- ImportAdapter trait pattern
21. [ADR-0008: Fuzzy Entity Resolution](architecture/adrs/adr-0008.md) -- Confidence scoring and merge strategies
22. [ADR-0009: Extended Cross-References](architecture/adrs/adr-0009.md) -- Wikilinks, URLs, @mentions
23. [ADR-0010: Entity Type Inference](architecture/adrs/adr-0010.md) -- Frontmatter type field
24. [ADR-0011: BinaryContent Component](architecture/adrs/adr-0011.md) -- Binary data references

For **product managers and stakeholders**, also read:

25. [Product Vision](philosophy/product-vision.md) -- Long-term direction
26. [Governance](philosophy/governance.md) -- Decision-making process
27. [Engineering Principles](philosophy/engineering-principles.md) -- How code is developed
28. [Landscape 2026](research/landscape-2026.md) -- 2026 knowledge management landscape

For **plugin developers**, also read:

29. [Extensibility](architecture/extensibility.md) -- Plugin system
30. [Plugin Development Guide](guides/plugin-development.md) -- How to build plugins
31. [AI Agent Guidelines](guides/ai-agent-guidelines.md) -- How AI agents work

For **learners**, start with:

32. [Tutorial: First Import](guides/tutorials/first-import.md) -- Import your first document
33. [Tutorial: Build a Custom Importer](guides/tutorials/build-custom-importer.md) -- Extend the system

---

## Document Types

Following [Diataxis](https://diataxis.fr/):

| Type            | Purpose                         | Locations                                                                                                                                                                                                                          |
| --------------- | ------------------------------- | ---------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| **Explanation** | Understanding and context       | `philosophy/`, `architecture/`, `engineering/deployment.md`, `engineering/testing-strategy.md`                                                                                                                                     |
| **Reference**   | Factual specifications          | `reference/`, `engineering/security.md`, `engineering/product-requirements.md`, `engineering/ui-design-system.md`, `engineering/api-specification.md`, `engineering/infrastructure-handbook.md`, `engineering/testing-strategy.md` |
| **How-to**      | Task-oriented guides            | `guides/`, `engineering/operational-runbooks.md`, `engineering/engineering-handbook.md`                                                                                                                                            |
| **Tutorial**    | Learning experiences            | `guides/tutorials/`                                                                                                                                                                                                                |
| **Research**    | Landscape analysis and evidence | `research/`                                                                                                                                                                                                                        |

> **Note:** The `engineering/` directory contains mixed Diataxis types. Each file's type is determined by its content, not its location.
