# Engineering Principles

> Architecture before implementation. Features answer questions. Derived data is reproducible. Storage layers are replaceable.

---

## Coding Standards

### Language

The primary implementation language is Rust. All new code follows Rust conventions.

### Style

- Follow `rustfmt` defaults. Do not override formatting.
- Follow `clippy` lint recommendations. Do not suppress warnings without justification.
- Use meaningful names. Abbreviations are avoided in public APIs.
- Prefer explicit over implicit. The reader must not guess what code does.

### Structure

- One crate per pipeline layer.
- One crate per storage adapter.
- One crate per plugin type.
- Dependencies flow inward: plugins depend on core, never the reverse.

### Error Handling

- All errors are typed. No string-based error messages in public APIs.
- Errors propagate with `?`. No `unwrap()` in production code.
- Errors include context: what was attempted, what failed, why it failed.
- Recoverable errors are retryable. Unrecoverable errors halt the operation.

### Documentation

- Every public function, struct, enum, and trait has a doc comment.
- Doc comments explain intent, not implementation.
- Examples are provided for non-obvious APIs.
- `# Panics`, `# Errors`, and `# Safety` sections are required where applicable.

---

## Feature Evaluation

Every feature must answer ten engineering questions before implementation begins. This checklist is derived from the [Engineering Architecture Constitution](engineering-architecture.md).

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

---

## Architectural Review

Every significant change undergoes architectural review before merge.

### What Requires Review

- New entity types, relationship types, or component types
- New pipeline layers or modifications to existing layers
- New storage adapters
- New plugin types
- Changes to the event system
- Changes to the canonical model schema
- Changes to the derivation pipeline

### What Does Not Require Review

- Bug fixes that do not change the data model
- Documentation updates
- Dependency upgrades
- Refactoring that does not change public APIs
- New importer plugins that follow the existing plugin contract

### Review Criteria

1. **Does the change pass the ten-question checklist?**
2. **Does the change maintain backward compatibility?**
3. **Does the change preserve determinism?**
4. **Does the change keep derived data disposable?**
5. **Does the change keep storage replaceable?**

---

## Testing Philosophy

### Principles

1. **Tests are deterministic.** The same input produces the same output. No randomness, no external dependencies, no time-dependent behavior in unit tests.
2. **Tests are compositional.** Each layer is tested in isolation. Integration tests verify layer contracts. System tests verify end-to-end behavior.
3. **Tests are documentation.** Tests demonstrate how the system is intended to be used. Test names describe behavior, not implementation.
4. **Tests are fast.** Unit tests execute in milliseconds. Integration tests execute in seconds. Slow tests are flagged.

### Test Pyramid

```
         +-----------+
         |  System   |     End-to-end tests, full pipeline
         |   Tests   |
         +-----------+
        +-------------+
        | Integration |     Layer contract tests, adapter tests
        |    Tests    |
        +-------------+
       +---------------+
       |     Unit      |    Component tests, function tests
       |     Tests     |
       +---------------+
```

- **Unit tests** verify individual functions and components. They are fast, isolated, and numerous.
- **Integration tests** verify contracts between layers and adapters. They use real storage engines in test mode.
- **System tests** verify end-to-end behavior. They exercise the full pipeline from import to rendering.

### Pipeline Testing

The deterministic pipeline enables a specific testing strategy:

- **Canonical tests.** Given a fixed input, verify the canonical output. Re-run the pipeline on the same input and verify identical results.
- **Derivation tests.** Given canonical data, verify derived artifacts. Drop derived data, re-derive, and verify.
- **Event tests.** Given a sequence of events, verify the processing pipeline produces correct derived state.
- **Regression tests.** When a bug is found, add the failing input as a test case. The bug must never recur.

### Property-Based Testing

Where applicable, property-based testing verifies invariants across a range of inputs:

- Entity identifiers are unique across all generated entities.
- Component types are unique within an entity.
- Relationships always reference existing entities.
- Derived data is always reproducible from canonical data.

---

## Documentation Standards

Documentation follows the [Diataxis framework](https://diataxis.fr/): four types of content for four different needs.

### Documentation Types

| Type | Purpose | Audience |
|------|---------|----------|
| **Explanation** | Understanding and context | Anyone learning the system |
| **Reference** | Factual specifications | Engineers building on the system |
| **How-to** | Task-oriented guides | Engineers performing specific tasks |
| **Tutorial** | Learning experiences | New contributors |

### File Conventions

- File names use `kebab-case.md`.
- Every document has a title and a one-line description.
- Every document links to related documents.
- No document stands alone. Every document is part of the reading order.

### Writing Style

- Write as affirmation, not opinion.
- Write for the reader, not the author.
- Prefer concrete examples over abstract descriptions.
- Prefer tables over paragraphs for structured information.
- Never use speculative language ("might," "could," "should consider"). Use declarative language ("is," "does," "provides").

---

## Observability

The system is designed to be observable. Every operation produces signals that can be monitored, logged, and analyzed.

### Logging

- Every pipeline stage logs its inputs, outputs, and duration.
- Every error is logged with full context: what failed, why, and what the inputs were.
- Logs are structured (JSON) and machine-parseable.
- Log levels: `error` (system failure), `warn` (degraded operation), `info` (significant event), `debug` (detailed tracing).

### Metrics

- Pipeline throughput: entities processed per second per layer.
- Pipeline latency: time from import to canonical persistence, time from canonical change to derived update.
- Storage health: connection pool utilization, query latency, error rates.
- AI operation metrics: model invocation count, latency, confidence distribution.

### Tracing

- Every request is assigned a correlation ID.
- All events and logs within a request share the correlation ID.
- Distributed tracing spans cover the full pipeline path.
- Traces are exportable to standard observability platforms.

### Health Checks

- Each storage adapter exposes a health check endpoint.
- Each pipeline stage exposes a readiness indicator.
- The system exposes an overall health status: healthy, degraded, unhealthy.

---

## Versioning

### Semantic Versioning

Knowledge OS follows [Semantic Versioning](https://semver.org/) (SemVer):

- **Major version.** Breaking changes to the canonical model, the event schema, or the plugin API.
- **Minor version.** New features that are backward-compatible: new entity types, new relationship types, new component types, new plugin types.
- **Patch version.** Bug fixes that do not change the data model or public API.

### Canonical Model Versioning

The canonical model is versioned independently of the software version:

- **Schema version.** The version of the entity, relationship, and component type definitions.
- **Data version.** The version of each individual entity and relationship.

Schema changes are backward-compatible: new types are added, existing types are never modified or removed.

### Plugin API Versioning

The plugin API is versioned independently:

- **API version.** The version of the plugin contract.
- **Plugin compatibility.** Plugins declare which API versions they support.
- **Deprecation cycle.** Deprecated API versions are supported for at least two minor versions before removal.

---

## Backward Compatibility

The system preserves backward compatibility across versions.

### Compatibility Rules

1. **Canonical data is never lost.** Schema migrations add fields, never remove them. Old data remains readable.
2. **Events are never restructured.** New event types are added. Existing event types are never modified.
3. **Plugin APIs are deprecated before removal.** Deprecated APIs remain functional for a defined deprecation period.
4. **Derived data may be rebuilt.** When derived data format changes, a full rebuild is triggered. This is acceptable because derived data is disposable.

### Migration Strategy

- **Additive migrations.** Adding a new field, type, or relationship. No user action required.
- **Transformative migrations.** Changing the structure of existing data. The system runs a migration that preserves all canonical data.
- **Rebuild migrations.** Changing derived data format. The system drops derived data and rebuilds it from canonical sources.

### Version Compatibility Matrix

| Component | Policy |
|-----------|--------|
| Canonical data | Forward-compatible (old readers can read new data) |
| Events | Forward-compatible (old consumers can process new events) |
| Plugin API | Backward-compatible (new plugins work with old APIs) |
| Derived data | Disposable (rebuilt on version change) |

---

## Further Reading

- [Philosophy](../philosophy/philosophy.md) -- Engineering values and immutable principles
- [Engineering Architecture](../engineering-architecture.md) -- The engineering constitution
- [CONTRIBUTING.md](../../CONTRIBUTING.md) -- How to participate
- [Governance](governance.md) -- How decisions are made
