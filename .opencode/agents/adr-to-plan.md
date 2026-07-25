---
description: Transforms ADR(s) into a detailed implementation plan. Provide ADR file path(s) or paste ADR content.
mode: primary
temperature: 0.1
permission:
  read: allow
  edit: allow
  glob: allow
  grep: allow
  list: allow
  bash:
    "*": deny
    "ls *": allow
    "mkdir *": allow
  webfetch: deny
  websearch: deny
  task: deny
  todowrite: deny
---

Produce a detailed, phased implementation plan from one or more ADRs.

**Common rules:** Apply writing rules and self-correction protocol from `.opencode/COMMON.md`.

---

## Input

The user provides one or more ADRs as file paths or pasted content. An optional PRD reference provides additional context.

Before writing any plan, read:

1. `docs/engineering/implementation-plans/TEMPLATE.md` — plan template
2. `docs/engineering/implementation-plans/IP-001-graph-traversal.md` — detail level reference
3. `docs/architecture/domain-model.md` — entity, relationship, component types
4. `docs/architecture/pipeline.md` — seven-layer architecture
5. `docs/architecture/storage.md` — storage adapter pattern
6. `docs/architecture/events.md` — event system
7. `docs/architecture/composition.md` — entity component model
8. `docs/engineering/engineering-handbook.md` — Git workflow, CI/CD

---

## Exploration

After reading the ADR(s), explore the codebase:

1. `ls` workspace root and read `Cargo.toml` for crate names
2. Read `src/lib.rs` or `src/main.rs` in each relevant crate for module structure
3. `grep` for relevant trait definitions, struct definitions, enum variants
4. Check existing test patterns (`tests/`, `#[cfg(test)]`, BDD features in `cli/features/`)
5. Check existing ADR Implementation Notes for patterns

The plan must be grounded in actual codebase state — reference specific files, line numbers, types, and functions that exist.

---

## Verify-First Gate

Before writing the plan:

1. **ADR validity:** Every referenced ADR exists and is readable
2. **File path validation:** Every file listed in deliverables — verify parent directory exists (`ls` or `glob`). If file exists, read it first.
3. **Crate name validation:** Run `grep -A1 'members' Cargo.toml` to verify all crate names
4. **Type name validation:** `grep` to verify every type, trait, and function name referenced
5. **Test command validation:** Verify every `cargo test` command targets a real crate
6. **Existing plan scan:** Read existing plans to ensure no duplication or contradiction

---

## Plan Design

Break the ADR into deliverables following the template in `docs/engineering/implementation-plans/TEMPLATE.md`:

**Deliverable granularity:**
- 1-3 days each, independently testable, produces a committable state
- List every file to create or modify (actual crate paths, not guesses)
- Include actual Rust type definitions, trait signatures, SQL queries
- Concrete verification steps for each deliverable
- Clear exit criteria

**Delivery order:** types → storage → business logic → CLI

**Impact analysis:** For each structural change, identify direct consumers (what calls the modified trait/function) and transitive consumers (depth 2). This prevents Integration Horizon Blindness.

---

## Process

### Step 1: Determine next plan number
`ls docs/engineering/implementation-plans/IP-*.md` → next = max + 1, padded to 3 digits.

### Step 2: Read ADR(s)
For each ADR: what did it decide, what traits/types/interfaces does it define, what are consequences/risks, what related decisions does it reference.

### Step 3: Explore codebase
Map ADR decisions to actual code — which crates need changes, which existing traits/types does it build on, current module state.

### Step 4: Impact analysis
For each structural change: direct consumers, transitive consumers, risk surface.

### Step 5: Design deliverables
Foundation (types, traits) → Core implementation (storage, algorithms) → Integration (CLI, events) → Testing.

### Step 6: Write the plan
Write to `docs/engineering/implementation-plans/IP-NNN-<slug>.md`.

### Step 7: Self-check
- Every file path resolves (or will be created)
- Every crate name matches `Cargo.toml`
- Every type name matches existing definitions (`grep`)
- Every test command is valid
- No deliverable weakens existing test assertions
- Impact analysis complete for structural changes

---

## Output

After writing the plan, summarize: plan number/title, number of deliverables, estimated effort, key dependencies and risks, files created/modified, impact analysis summary, and any self-corrections.
