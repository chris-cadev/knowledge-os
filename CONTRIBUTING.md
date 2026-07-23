# Contributing to Knowledge OS

Thank you for your interest in contributing to Knowledge OS. This document explains how to participate effectively.

---

## Before You Contribute

Every contributor must read the following documents before making any contribution:

1. [Seed Manifesto](docs/foundational-manifesto.md) -- The constitutional outline of the project
2. [Technical Foundation](docs/engineering-architecture.md) -- The engineering architecture constitution
3. [Philosophy](docs/philosophy/philosophy.md) -- Core philosophy and immutable principles
4. [Engineering Handbook](docs/engineering/engineering-handbook.md) -- Day-to-day engineering practices, Git workflow, code review, CI/CD
5. [Architectural Principles](docs/architecture/architectural-principles.md) -- Architectural invariants and the validation checklist

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
  philosophy/       Why the project exists and what it believes
  architecture/     How the system is designed and why
  reference/        Glossary, API specs, configuration
  engineering/      Testing, security, deployment, and practices
  guides/           How-to guides and tutorials
  research/         Landscape analysis and evidence
```

### Architecture Decision Records

Significant architectural decisions are recorded as ADRs in `docs/architecture/adrs/`. See the [ADR index](docs/architecture/adrs/README.md) for the template and existing records.

---

## Engineering Rules

### Feature Evaluation

Every feature must answer these questions before implementation. See [Engineering Principles](docs/philosophy/engineering-principles.md) for detailed explanations of each question.

1. **Which canonical entities are introduced?** If the feature creates new entity types, define them.
2. **Which relationships are introduced?** If the feature creates new relationship types, define them.
3. **Which components are introduced?** If the feature creates new component types, define their schema.
4. **Which events are emitted?** If the feature modifies canonical data, define the events.
5. **Which derived representations are generated?** If the feature affects search, embeddings, or caches, define the derivation.
6. **Which layer owns the feature?** Every feature belongs to exactly one pipeline layer.
7. **Can every derived artifact be regenerated?** If any derived artifact cannot be rebuilt, reclassify it as canonical.
8. **Does the feature violate storage independence?** If the feature requires a specific storage engine, redesign it.
9. **Does the feature introduce implementation leakage?** If the feature exposes storage details, AI model details, or infrastructure details to the domain model, redesign it.
10. **Does the feature preserve the canonical model?** If the feature bypasses the canonical model, redesign it.

Features that fail any question are redesigned before implementation.

### Architectural Validation

Every significant change must also pass the [architectural validation checklist](docs/architecture/architectural-principles.md#validation-checklist). This covers canonical model preservation, storage independence, derived data disposability, event emission, layer boundaries, composition over inheritance, determinism, scalability, extensibility, and AI treatment.

---

## Code Style

### Rust

- Follow `rustfmt` defaults. Do not override formatting.
- Follow `clippy` lint recommendations. Do not suppress warnings without justification.
- Use meaningful names. Abbreviations are avoided in public APIs.
- Prefer explicit over implicit. The reader must not guess what code does.
- All errors are typed. No string-based error messages in public APIs.
- Errors propagate with `?`. No `unwrap()` in production code.
- Every public function, struct, enum, and trait has a doc comment.

See [Engineering Principles](docs/philosophy/engineering-principles.md) for the complete coding standards.

### Documentation

- File names use `kebab-case.md`.
- Every document has a title and a one-line description.
- Every document links to related documents.
- Documentation uses Markdown with [CommonMark](https://commonmark.org/) formatting.
- Write as affirmation, not opinion. "The system uses events" not "I think events are better."
- Never use speculative language. Use declarative language.

---

## Questions?

Open a GitHub Issue with the `question` label.
