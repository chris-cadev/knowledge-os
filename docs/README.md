# Knowledge OS Documentation

This directory contains all project documentation, organized following the [Diataxis framework](https://diataxis.fr/).

---

## Structure

```
docs/
  foundational-manifesto.md   Foundational seed manifesto (Constitution Outline v1.0)
  engineering-architecture.md Engineering architecture constitution (v1.0)

  philosophy/                 Why this project exists
    philosophy.md             Core philosophy and immutable principles
    vision.md                 The knowledge problem and the Knowledge OS vision
    boundaries.md             What we build and what we intentionally skip
    engineering-principles.md How software is developed
    product-vision.md         Long-term direction and ecosystem
    governance.md             How decisions are made

  architecture/               How the system is designed
    overview.md               Current technical architecture overview
    mental-model.md           The canonical way of thinking about the system
    pipeline.md               The seven-layer pipeline
    data-model.md             The distinction between canonical and derived data
    storage.md                Polyglot persistence and storage independence
    composition.md            Composition over inheritance
    compilation.md            Knowledge as compilation
    events.md                 Event sourcing and processing
    domain-model.md           Entity types, relationship types, component types
    ai.md                     Artificial intelligence as a system component
    ui-philosophy.md          User interface philosophy and view types
    extensibility.md          Plugin system and extension points
    scalability.md            Scaling strategies and capacity planning
    synchronization.md        Event-driven consistency and derived data updates
    adrs/                     Architecture Decision Records
      README.md               ADR index and template
      adr-0001.md             Knowledge Model as Canonical Source of Truth
      adr-0002.md             Storage Independence via Adapter Pattern
      adr-0003.md             Entity Component Model for Knowledge Entities
      adr-0004.md             Event-Driven Derivation Pipeline
      adr-0005.md             Compiler-Inspired Architecture

  reference/                  Definitive specifications
    glossary.md               Every project term, defined once

  engineering/                Engineering practices
    testing-strategy.md       Test philosophy, pyramid, and pipeline testing
    security.md               Threat model, authentication, authorization
    deployment.md             Deployment models and storage configuration

  guides/                     How-to guides
    plugin-development.md     How to develop, test, and distribute plugins
    ai-agent-guidelines.md    How AI agents interact with the knowledge model
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

---

## Document Types

Following Diataxis:

| Type | Purpose | Location |
|------|---------|----------|
| **Explanation** | Understanding and context | `philosophy/`, `architecture/` |
| **Reference** | Factual specifications | `reference/` |
| **How-to** | Task-oriented guides | `guides/`, `engineering/` |
| **Tutorial** | Learning experiences | `guides/` (planned) |
