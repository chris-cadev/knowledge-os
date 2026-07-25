---
description: Creates a PRD from a feature description, user problem, or set of requirements. Provide a description of what you want to build.
mode: primary
temperature: 0.2
permission:
  read: allow
  edit: allow
  glob: allow
  grep: allow
  list: allow
  bash:
    "*": deny
    "ls *": allow
  webfetch: deny
  websearch: deny
  task: deny
  todowrite: deny
---

Write a Product Requirements Document (PRD) for a Knowledge OS feature.

**Common rules:** Apply writing rules and self-correction protocol from `.opencode/COMMON.md`.

---

## Input

The user provides one of:
- A feature description ("I want to add graph traversal with bounded depth")
- A user problem ("Users can't explore relationships between entities")
- A set of requirements or a rough idea

Ask clarifying questions if the description is ambiguous. The PRD must be specific enough that an architect can extract ADRs from it.

---

## Before Writing

Read these references to understand the system:

1. `docs/architecture/domain-model.md` — entity, relationship, component types
2. `docs/architecture/pipeline.md` — seven-layer architecture
3. `docs/philosophy/engineering-principles.md` — 10-question checklist
4. `docs/engineering/prds/prd-0003-graph-exploration-and-plugins.md` — format and detail level reference

Read existing PRDs (`docs/engineering/prds/`) to ensure no duplication and to understand scope boundaries.

---

## Verify-First Gate

Before writing any PRD:

1. **Scope check:** The feature is not already covered by an existing PRD
2. **Dependency check:** If the feature depends on existing features, verify they are implemented or planned
3. **Type consistency:** Reference only entity types, component types, and relationship types that exist in `docs/architecture/domain-model.md` (or explicitly propose new ones)
4. **Pipeline consistency:** Place the feature correctly in the seven-layer pipeline

---

## PRD Format

Follow this structure exactly. Every section is required.

```markdown
# PRD-NNNN: <Title>

**Status:** Draft
**Date:** YYYY-MM-DD
**Author:** Core maintainers
**Priority:** P0 | P1 | P2
**Depends on:** PRD-XXXX, PRD-YYYY

---

## Purpose

One paragraph. What this feature does and why it matters.

---

## Problem Statement

What is the user's problem? What is the current gap? Why now?

---

## Scope

### In Scope

- Feature A
- Feature B

### Out of Scope

- Feature C (deferred to PRD-XXXX)
- Feature D (explicitly not building)

---

## Engineering Questions

Answer all 10 questions from [Engineering Principles](../../philosophy/engineering-principles.md):

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

---

## Pipeline Spine Analysis

How does this feature extend or modify the pipeline? Show the pipeline extension diagram.

---

## Functional Requirements

### F1: <Feature Name>

| ID   | Requirement | Priority | Acceptance Criteria |
| ---- | ----------- | -------- | ------------------- |
| F1.1 | ...         | P0       | ...                 |

---

## Non-Functional Requirements

### NF1: Performance

| ID    | Requirement | Target | Acceptable |
| ----- | ----------- | ------ | ---------- |
| NF1.1 | ...         | ...    | ...        |

### NF2: Scalability

| ID    | Requirement | Target |
| ----- | ----------- | ------ |
| NF2.1 | ...         | ...    |

---

## User Stories

### US1: <Story Name>

**As a** <role>,
**I want to** <action>,
**So that** <benefit>.

**Acceptance criteria:**
1. ...
2. ...

---

## Architecture

### Crate Changes

| Crate | Change |
| ----- | ------ |
| ...   | ...    |

### <Architecture Details>

Include: trait definitions, SQL patterns, type signatures, storage schema changes, algorithm descriptions.

---

## CLI Interface

### New Commands

```bash
kos <command> <args>
```

### Output Format

```
$ kos <command> <args>

<expected output>
```

---

## Acceptance Criteria

### Definition of Done

- [ ] Criterion 1
- [ ] Criterion 2

### Test Cases

1. Test case description
2. ...

---

## Testing Strategy

| Level | Scope | Framework |
| ----- | ----- | --------- |
| Unit  | ...   | ...       |

---

## Risks and Mitigations

| Risk | Impact | Likelihood | Mitigation |
| ---- | ------ | ---------- | ---------- |
| ...  | ...    | ...        | ...        |

---

## Dependencies

### External Crates

| Crate | Purpose | Justification |
| ----- | ------- | ------------- |
| ...   | ...     | ...           |

### Internal Dependencies

- `docs/architecture/domain-model.md` — ...
- PRD-XXXX — ...

---

## Timeline

| Phase | Duration | Deliverables |
| ----- | -------- | ------------ |
| ...   | ...      | ...          |

**Total: ~X weeks**
```

---

## Process

### Step 1: Clarify the problem
Read the user's input. If ambiguous, ask: What is the user's pain point? Who is the user? What does success look like?

### Step 2: Read the system
Read existing PRDs, architecture docs, and domain model. Understand what exists, what's planned, and where the new feature fits.

### Step 3: Answer the 10 questions
Work through the Engineering Questions systematically. Reference `docs/architecture/domain-model.md` for entity types. Be specific — include actual type definitions.

### Step 4: Write the PRD
Follow the template exactly. Every section is required. Be specific — include trait signatures, SQL patterns, type definitions, CLI examples, test cases.

### Step 5: Self-check
- Every entity type, component type, and relationship type exists in domain-model.md (or is explicitly proposed as new)
- The feature is correctly placed in the pipeline
- Functional requirements have acceptance criteria
- Non-functional requirements have measurable targets
- User stories have concrete acceptance criteria
- Architecture section includes actual type definitions, not hand-waving
- No speculative language ("might," "could," "should consider")
- Scope is bounded — "Out of Scope" is explicit

### Step 6: Write the file

Write to `docs/engineering/prds/prd-NNNN-<slug>.md` where `<slug>` is a kebab-case summary.

---

## Output

After writing the PRD, summarize: PRD number and title, scope summary, key architectural decisions required, dependencies on other PRDs, estimated complexity (simple/moderate/complex).
