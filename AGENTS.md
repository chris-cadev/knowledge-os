# AGENTS.md

Instructions for AI coding agents working on Knowledge OS.

---

## Project Overview

Knowledge OS is a deterministic knowledge engine. It is documentation-first -- no source code exists yet. The entire architectural foundation is written before implementation begins.

**Primary language:** Rust (planned, not started)
**Current phase:** Documentation and architecture design

---

## Repository Structure

```
docs/
  foundational-manifesto.md   Foundational manifesto (read first)
  engineering-architecture.md Engineering architecture constitution
  philosophy/                 Why the project exists
    philosophy.md             Core philosophy and immutable principles
    vision.md                 The knowledge problem and the Knowledge OS vision
    boundaries.md             What we build and what we skip
    engineering-principles.md How software is developed
    product-vision.md         Long-term direction and ecosystem
    governance.md             How decisions are made
  architecture/               How the system is designed
    overview.md               Technical architecture overview
    mental-model.md           The canonical way of thinking
    domain-model.md           Entity, relationship, and component types
    pipeline.md               The seven-layer pipeline
    data-model.md             Canonical vs derived data
    storage.md                Polyglot persistence
    composition.md            Entity component model
    compilation.md            Knowledge as compilation
    events.md                 Event-driven architecture
    ai.md                     AI as a system component
    ui-philosophy.md          User interface philosophy
    extensibility.md          Plugin system and extension points
    scalability.md            Scaling strategies
    synchronization.md        Consistency and derived data updates
    adrs/                     Architecture Decision Records
      adr-0001.md through adr-0005.md
  reference/                  Glossary and specifications
    glossary.md               Every project term, defined once
  engineering/                Engineering practices
    testing-strategy.md       Test philosophy and pipeline testing
    security.md               Threat model and access control
    deployment.md             Deployment models and configuration
  guides/                     How-to guides
    plugin-development.md     How to build plugins
    ai-agent-guidelines.md    How AI agents work
README.md              Project entry point
CONTRIBUTING.md        How to participate
CHANGELOG.md           Release history
```

---

## Documentation Conventions

- Follow the [Diataxis framework](https://diataxis.fr/): Explanation, Reference, How-to, Tutorial
- File names: `kebab-case.md` (e.g., `design-principles.md`)
- No speculative language. Write as affirmations.
- No implementation-specific decisions unless they are architectural invariants.
- Every statement is a principle, not an opinion.

---

## What Agents Must Read Before Contributing

1. `docs/foundational-manifesto.md` -- The constitutional outline
2. `docs/engineering-architecture.md` -- The engineering architecture
3. `docs/philosophy/philosophy.md` -- Core principles
4. `docs/philosophy/vision.md` -- Why the project exists
5. `CONTRIBUTING.md` -- Engineering rules and the 10-question checklist

---

## Git Workflow

- Branch from `main`, prefix with `feat/`, `fix/`, or `docs/`
- Commit messages: `type: description` (e.g., `docs: add storage philosophy`)
- Conventional commits: `feat:`, `fix:`, `chore:`, `docs:`
- Squash merge PRs

---

## Do Not Touch

- `docs/foundational-manifesto.md` -- Immutable once established. Supersede, never edit.
- `docs/engineering-architecture.md` -- Engineering constitution. Changes require ADR.
- `LICENSE` -- MIT license. Do not modify.

---

## Verification

Before committing documentation changes:

1. Verify all internal links resolve (`grep -r '\](docs/' --include='*.md'`)
2. Verify no broken cross-references
3. Verify file naming follows `kebab-case.md`
4. Verify no speculative or opinionated language

---

## Architecture Rules

Every feature proposed for the system must answer these 10 questions (from `CONTRIBUTING.md`):

1. Which canonical entities are introduced?
2. Which relationships are introduced?
3. Which components are introduced?
4. Which events are emitted?
5. Which derived representations are generated?
6. Which layer owns the feature?
7. Can every derived artifact be regenerated?
8. Does the feature violate storage independence?
9. Does the feature introduce implementation leakage?
10. Does the feature preserve the canonical model?
