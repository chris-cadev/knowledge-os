# BDD Testing Guide

> How to write, maintain, and debug Behavior-Driven Development tests for the `kos` CLI.

---

## Prerequisites

- Rust stable toolchain installed (`mise install` or `rustup`)
- Familiarity with [Gherkin syntax](https://cucumber.io/docs/gherkin/)
- Read [Testing Strategy](../engineering/testing-strategy.md) for the full test pyramid

---

## Quick Start

### Run All BDD Tests

```bash
cargo test --test cucumber -p knowledge-cli
# or
mise run test-bdd
```

### Run a Single Feature

```bash
cargo test --test cucumber -p knowledge-cli -- --include prd-0001/import
```

### Run a Single Scenario

```bash
cargo test --test cucumber -p knowledge-cli -- --include "Import a single Markdown file"
```

---

## Architecture

BDD tests exercise the compiled `kos` binary against real temporary databases. Each scenario:

1. Creates a fresh `TempDir` (via `tempfile::TempDir`)
2. Writes test files into that directory
3. Runs `kos` commands with `--db <tempdir>/test.db`
4. Asserts on stdout/stderr output
5. Drops the `TempDir`, cleaning up all files and the database

No shared state exists between scenarios. This makes scenarios safe to run in any order.

### Key Files

| File                              | Purpose                                                |
| --------------------------------- | ------------------------------------------------------ |
| `cli/tests/cucumber.rs`           | Step definitions, `CliWorld` struct, assertion helpers |
| `cli/features/prd-0001/*.feature` | PRD-0001 user story scenarios                          |
| `cli/features/prd-0002/*.feature` | PRD-0002 user story scenarios                          |
| `cli/Cargo.toml`                  | `cucumber` and `tokio` dev-dependencies                |

---

## CliWorld

The `CliWorld` struct holds per-scenario state:

```rust
#[derive(Debug, Default, World)]
pub struct CliWorld {
    temp_dir: Option<TempDir>,
    last_output: Option<std::process::Output>,
    last_entity_id: Option<String>,
    last_merge_id: Option<String>,
    files: HashMap<String, String>,
    entity_ids: HashMap<String, String>,
}
```

- `temp_dir` — Isolated directory for this scenario. Created by the `Given an empty database` step.
- `last_output` — The most recent `kos` command output. Used by assertion steps.
- `last_entity_id` — Extracted from import or list commands. Used by `<entity-id>` placeholder replacement.
- `last_merge_id` — Extracted from resolution log. Used by `<merge-id>` placeholder replacement.
- `files` — Map of filename to content for files created in this scenario.
- `entity_ids` — Map of logical names to UUIDs for entities created in this scenario.

### Important: State Belongs on CliWorld

Do not use `static Mutex<LazyLock<T>>` for scenario state. The `World` trait creates a fresh instance per scenario. State that must persist across steps within a scenario belongs on `CliWorld` fields. State that uses `static` globals persists across scenarios and causes cross-contamination.

---

## Step Definitions

### Background Steps

| Step                      | Definition                                            |
| ------------------------- | ----------------------------------------------------- |
| `Given an empty database` | Creates a new `TempDir`, resets all `CliWorld` fields |

### File Setup Steps

| Step                                               | Definition                                  |
| -------------------------------------------------- | ------------------------------------------- |
| `Given a file {string} with content:`              | Creates a file in the temp directory        |
| `Given the file {string} is updated with content:` | Overwrites a file in the temp directory     |
| `Given a directory with files:`                    | Creates multiple files from a Gherkin table |
| `Given I import a file {string} with content:`     | Creates a file and runs `kos import` on it  |
| `Given an empty file {string}`                     | Creates an empty file                       |
| `Given I import files with varying relevance:`     | Imports multiple files from a table         |
| `Given I import files:`                            | Imports multiple files from a table         |

### Command Execution Steps

| Step                                                  | Definition                                                                                                                              |
| ----------------------------------------------------- | --------------------------------------------------------------------------------------------------------------------------------------- |
| `When I run {string}`                                 | Runs `kos` with the given arguments. Replaces `<directory>`, `<entity-id>`, `<merge-id>` placeholders. Strips leading `kos` if present. |
| `When I extract the entity ID from the last import`   | Parses `Created:` line from last output, sets `last_entity_id`                                                                          |
| `When I extract the entity ID for {string}`           | Runs `kos list`, finds the entity by title, sets `last_entity_id`                                                                       |
| `When I run kos get <entity-id>`                      | Runs `kos get` with `last_entity_id`                                                                                                    |
| `When I run kos archive <entity-id>`                  | Runs `kos archive` with `last_entity_id`                                                                                                |
| `When I run kos restore <entity-id>`                  | Runs `kos restore` with `last_entity_id`                                                                                                |
| `When I run kos resolution log`                       | Runs `kos resolution log`                                                                                                               |
| `When I extract the merge ID from the resolution log` | Parses `Merge ID:` line, sets `last_merge_id`                                                                                           |
| `When I run kos resolution undo <merge-id>`           | Runs `kos resolution undo` with `last_merge_id`                                                                                         |

### Assertion Steps

| Step                                                               | Definition                                              |
| ------------------------------------------------------------------ | ------------------------------------------------------- |
| `Then the output contains {string}`                                | Asserts stdout contains the string                      |
| `Then the error output contains {string}`                          | Asserts stderr contains the string                      |
| `Then the warning output contains {string}`                        | Asserts stderr or stdout contains the string            |
| `Then {int} relationships of type {string} exist`                  | Counts relationships of a type in `kos list` output     |
| `Then relationships of type {string} exist`                        | Asserts at least one relationship of a type exists      |
| `Then the entity has a Provenance component with source {string}`  | Asserts Provenance source in list output                |
| `Then the entity has a Content component with extracted text`      | Asserts Content component or word count in output       |
| `Then the entity has a BinaryContent component with file {string}` | Asserts BinaryContent in output                         |
| `Then the entity has components:`                                  | Asserts each component type in a table exists in output |
| `Then the entity version is incremented`                           | Asserts version is 2                                    |
| `Then the relationship preserves the section anchor`               | Asserts section/anchor in output                        |
| `Then no manual review is required`                                | Asserts no review-related text in output                |
| `Then the resolution log contains a merge with confidence`         | Asserts `confidence:` in resolution log                 |
| `Then the resolution respects the configured threshold`            | Asserts threshold-related text in output                |
| `Then {string} appears before {string} in results`                 | Asserts ordering in stdout                              |

---

## Writing a New Scenario

### Step 1: Identify the PRD User Story

Every scenario maps to a specific PRD user story. Check which PRD your feature belongs to:
- **PRD-0001** — Core entity model and markdown import pipeline
- **PRD-0002** — Rich import and resolution

### Step 2: Create or Edit the Feature File

Feature files live under `cli/features/prd-NNNN/`. Create a new file or add to an existing one.

```gherkin
@prd-0001 @search
Feature: Search and Retrieval

  As a knowledge worker
  I want to search for entities by topic
  So that I can find relevant information quickly

  Background:
    Given an empty database

  @us3
  Scenario: Search by keyword returns matching entities
    Given I import a file "transformer.md" with content:
      """
      ---
      title: "Transformer Architecture"
      tags:
        - transformer
        - attention
      ---
      
      # Transformer Architecture
      
      The transformer relies on self-attention mechanisms.
      """
    When I run "kos search transformer"
    Then the output contains "Found 1"
    And the output contains "Transformer Architecture"
    And the output contains "Snippet:"
```

### Step 3: Run the Scenario

```bash
cargo test --test cucumber -p knowledge-cli -- --include "Search by keyword returns matching entities"
```

### Step 4: Add New Steps (if needed)

If no existing step matches your scenario, add a new step in `cli/tests/cucumber.rs`:

```rust
#[given(expr = "a custom setup for {string}")]
async fn custom_setup(world: &mut CliWorld, name: String) {
    // Implementation
}

#[then(expr = "the output has custom behavior")]
async fn custom_assertion(world: &mut CliWorld) {
    let stdout = world.stdout();
    assert!(stdout.contains("expected"), "Custom assertion failed");
}
```

Use `#[given]`, `#[when]`, or `#[then]` macros. The `expr` parameter uses cucumber expression syntax:
- `{string}` — Matches a quoted string argument
- `{int}` — Matches an integer argument
- `{float}` — Matches a float argument
- Plain text — Matches literal step text

---

## Common Patterns

### Importing a File and Checking Output

```gherkin
Given I import a file "test.md" with content:
  """
  ---
  title: "Test Entity"
  ---
  
  # Test Entity
  
  Some content.
  """
When I run "kos search test"
Then the output contains "Found 1"
And the output contains "Test Entity"
```

### Using Entity ID Placeholders

```gherkin
Given I import a file "entity.md" with content:
  """
  # Entity
  
  Content.
  """
When I extract the entity ID from the last import
And I run "kos get <entity-id>"
Then the output contains "Entity:"
And the output contains "Version: 1"
```

### Testing Duplicate Detection

```gherkin
Given I import a file "original.md" with content:
  """
  ---
  title: "My Document"
  ---
  
  # My Document
  
  Original content.
  """
When I run "kos import original.md"
Then the output contains "Duplicates resolved: 1"
```

### Testing Archive and Restore Lifecycle

```gherkin
Given I import a file "archive-me.md" with content:
  """
  # Archive Me
  
  Content to archive.
  """
When I extract the entity ID from the last import
And I run "kos archive <entity-id>"
Then the output contains "Archived"
When I run "kos search Archive"
Then the output contains "No entities found."
When I run "kos restore <entity-id>"
Then the output contains "Restored"
When I run "kos search Archive"
Then the output contains "Found 1"
```

### Testing Batch Import

```gherkin
Given a directory with files:
  | filename | content   |
  | doc-a.md | # Doc A   |
  | doc-b.md | # Doc B   |
  | doc-c.md | # Doc C   |
When I run "kos import <directory>"
Then the output contains "Total files: 3"
And the output contains "Created: 3"
```

---

## Debugging Failing Scenarios

### Check the Actual Output

When a step fails, cucumber prints the actual stdout and stderr. Use this to update your assertions:

```
Step panicked. Captured output: Expected 'Found 1' in stdout, got:
stdout: No entities found.

stderr: 
```

### Run with Backtrace

```bash
RUST_BACKTRACE=1 cargo test --test cucumber -p knowledge-cli -- --include "scenario name"
```

### Inspect the Temp Directory

Add a `println!` or `eprintln!` to your step to print the temp directory path, then inspect it manually:

```rust
#[when(expr = "I debug the temp directory")]
async fn debug_temp_dir(world: &mut CliWorld) {
    eprintln!("Temp dir: {}", world.temp_path().display());
    // Run a command to list files
    let output = world.run_kos_direct(&["list"]);
    eprintln!("List output: {}", String::from_utf8_lossy(&output.stdout));
}
```

### Common Failure Modes

| Symptom                                  | Cause                                                                    | Fix                                                                  |
| ---------------------------------------- | ------------------------------------------------------------------------ | -------------------------------------------------------------------- |
| `ParseChar { character: '[', index: 0 }` | Entity ID starts with `[` — extraction parsed wrong field                | Check `extract_entity_id_for_title` parses `parts[1]` not `parts[0]` |
| `ParseChar { character: '<', index: 0 }` | Placeholder not replaced — `last_entity_id` or `last_merge_id` is `None` | Check the extraction step ran before the command step                |
| `Expected 'X' in stdout, got: ...`       | CLI output format changed                                                | Update the assertion string to match actual output                   |
| Scenario passes intermittently           | State leaking between scenarios                                          | Ensure state is on `CliWorld`, not `static Mutex`                    |

---

## Rules

1. **Each scenario is isolated.** Never depend on state from a previous scenario.
2. **Assertions match actual output.** When the CLI output changes, update the feature files.
3. **No speculative scenarios.** Every scenario tests a defined PRD user story.
4. **Keep scenarios simple.** One behavior per scenario. Complex flows use `And` steps.
5. **Use existing steps first.** Only add new steps when no existing step fits.

---

## Further Reading

- [Testing Strategy](../engineering/testing-strategy.md) -- Test philosophy and pyramid
- [PRD-0001](../engineering/prds/prd-0001-core-entity-model.md) -- Core entity model requirements
- [PRD-0002](../engineering/prds/prd-0002-rich-import-and-resolution.md) -- Rich import requirements
- [cucumber-rs documentation](https://cucumber-rs.github.io/cucumber/) -- Framework reference
