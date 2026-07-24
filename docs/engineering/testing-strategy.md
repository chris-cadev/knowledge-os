# Testing Strategy

> Tests are deterministic. Tests are compositional. Tests are documentation. Tests are fast.

---

## Test Pyramid

```mermaid
graph TD
    ST[System Tests<br/>End-to-end tests, full pipeline]
    IT[Integration Tests<br/>Layer contract tests, adapter tests]
    UT[Unit Tests<br/>Component tests, function tests]
    
    ST --> IT
    IT --> UT
```

- **Unit tests** verify individual functions and components. They are fast, isolated, and numerous.
- **Integration tests** verify contracts between layers and adapters. They use real storage engines in test mode.
- **System tests** verify end-to-end behavior. They exercise the full pipeline from import to rendering.
- **BDD tests** verify user-facing behavior through Gherkin scenarios. They exercise the CLI against real binaries and temporary databases.

---

## BDD Testing

Behavior-Driven Development tests describe user stories in Gherkin syntax and execute them against the compiled `kos` binary. The `kos` CLI crate uses [cucumber-rs](https://github.com/cucumber-rs/cucumber) for BDD.

### Structure

```
cli/
  features/
    prd-0001/                  Tests for PRD-0001 user stories
      import.feature           Markdown import pipeline
      search.feature           Search, retrieval, entity lifecycle
      entity-management.feature CRUD, versioning, relationships, tags
    prd-0002/                  Tests for PRD-0002 user stories
      pdf-import.feature       PDF import (error handling for invalid files)
      resolution.feature       Duplicate detection, merge, undo
      extended-import.feature  Type inference, cross-refs, batch import
  tests/
    cucumber.rs                Step definitions and CliWorld
    integration_test.rs        assert_cmd integration tests
```

### Running BDD Tests

```bash
cargo test --test cucumber -p knowledge-cli    # All BDD scenarios
mise run test-bdd                               # Same, via mise
```

### Conventions

- Each `.feature` file maps to a PRD user story. Feature file names use `kebab-case`.
- Background steps create a fresh `TempDir` per scenario. No shared state between scenarios.
- The `CliWorld` struct holds per-scenario state: temp directory, last output, entity IDs, merge IDs.
- Step definitions invoke the `kos` binary via `assert_cmd::Command::cargo_bin("kos")`.
- Each scenario gets its own `--db` path inside the temp directory.
- Assertions match actual CLI output text. When the CLI output changes, update the feature files.
- `max_concurrent_scenarios(1)` is required because `TempDir` isolation prevents concurrent scenarios from sharing state.

### Writing New Scenarios

1. Identify the PRD user story the scenario covers.
2. Create or edit the `.feature` file under `cli/features/prd-NNNN/`.
3. Use existing step definitions where possible. Add new steps in `cucumber.rs` only when no existing step matches.
4. Run `cargo test --test cucumber -p knowledge-cli` to verify.
5. See [BDD Testing Guide](../guides/bdd-testing.md) for detailed instructions.

---

## Pipeline Testing

The deterministic pipeline enables a specific testing strategy:

### Canonical Tests

Given a fixed input, verify the canonical output. Re-run the pipeline on the same input and verify identical results.

```
Input: "test-paper.md"
Expected: Entity { type: Paper, title: "Test Paper", ... }
Assert: pipeline.run(input) == expected
```

### Derivation Tests

Given canonical data, verify derived artifacts. Drop derived data, re-derive, and verify.

```
Input: Entity { type: Paper, content: "..." }
Expected: SearchIndexEntry { terms: [...], ... }
Assert: derive(entity) == expected
```

### Event Tests

Given a sequence of events, verify the processing pipeline produces correct state.

```
Events: [EntityCreated, ComponentAdded, RelationshipCreated]
Expected: Derived state matches canonical state
Assert: process(events) == expected_derived_state
```

### Regression Tests

When a bug is found, add the failing input as a test case. The bug must never recur.

---

## Property-Based Testing

Where applicable, property-based testing verifies invariants across a range of inputs:

- Entity identifiers are unique across all generated entities.
- Component types are unique within an entity.
- Relationships always reference existing entities.
- Derived data is always reproducible from canonical data.
- Events are always ordered by entity version.

---

## Test Organization

```
tests/
  unit/               Unit tests (co-located with source)
  integration/        Integration tests
    pipeline/         Pipeline layer tests
    storage/          Storage adapter tests
    plugins/          Plugin contract tests
  system/             End-to-end tests
    import/           Import pipeline tests
    query/            Query and rendering tests
    ai/               AI integration tests
  properties/         Property-based tests
  fixtures/           Test data and fixtures
  cucumber/           BDD feature files (per-crate)
    features/         Gherkin scenarios organized by PRD
```

---

## Test Data

- **Fixtures.** Pre-defined entities, components, and relationships for deterministic tests.
- **Generators.** Random entity and relationship generators for property-based tests.
- **Snapshots.** Captured pipeline outputs for regression tests.
- **TempDir.** BDD tests create isolated temporary directories per scenario. Each scenario gets its own database and files.

---

## Test Execution

- **Unit tests:** `cargo test` (milliseconds)
- **Integration tests:** `cargo test --test integration` (seconds)
- **BDD tests:** `cargo test --test cucumber -p knowledge-cli` (seconds)
- **System tests:** `cargo test --test system` (seconds to minutes)
- **All tests:** `cargo test` (comprehensive)
- **Via mise:** `mise run test`, `mise run test-bdd`

---

## Coverage

- Unit tests target > 90% code coverage.
- Integration tests target all storage adapters and pipeline layers.
- System tests target all import formats and view types.

---

## Further Reading

- [Engineering Principles](../philosophy/engineering-principles.md) -- Testing philosophy
- [Pipeline](../architecture/pipeline.md) -- Pipeline architecture
- [Events](../architecture/events.md) -- Event-driven testing
