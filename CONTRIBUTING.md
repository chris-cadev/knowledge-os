# Contributing to Knowledge OS

Thank you for your interest in contributing to Knowledge OS. This document explains how to participate effectively.

---

## Before You Contribute

Every contributor must read the following documents before making any contribution:

1. [Seed Manifesto](docs/foundational-manifesto.md) -- The constitutional outline of the project
2. [Technical Foundation](docs/engineering-architecture.md) -- The engineering architecture constitution
3. [Philosophy](docs/philosophy/philosophy.md) -- Core philosophy and immutable principles

If a proposed feature contradicts the philosophy described in these documents, the feature must be redesigned before implementation.

---

## How to Contribute

### Reporting Issues

Use GitHub Issues to report bugs, suggest features, or ask questions. Before opening a new issue, search existing issues to avoid duplicates.

### Proposing Changes

1. Fork the repository
2. Create a feature branch from `main`
3. Make your changes
4. Ensure all documentation is updated
5. Submit a pull request

### Writing Documentation

Documentation follows the [Diataxis framework](https://diataxis.fr/). Before writing, determine which category your content falls into:

- **Tutorial** -- A learning-oriented guided experience
- **How-to guide** -- A task-oriented recipe for practitioners
- **Reference** -- Factual, complete specifications
- **Explanation** -- Deep understanding and context

Place your content in the appropriate `docs/` subdirectory:

```
docs/
  philosophy/       Design principles, goals, non-goals
  architecture/     System design, ADRs, technical decisions
  reference/        Glossary, API specs, configuration
```

### Architecture Decision Records

Significant architectural decisions are recorded as ADRs in `docs/architecture/adrs/`. See the [ADR index](docs/architecture/adrs/README.md) for the template and existing records.

---

## Engineering Rules

Every feature must answer these questions before implementation:

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

Features that fail these questions are redesigned before implementation.

---

## Code Style

Rust code will follow standard `rustfmt` conventions. Documentation uses Markdown with [CommonMark](https://commonmark.org/) formatting. All documentation files use `kebab-case` naming.

---

## Questions?

Open a GitHub Issue with the `question` label.
