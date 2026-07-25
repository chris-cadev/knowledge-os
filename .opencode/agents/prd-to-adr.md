---
description: Transforms a PRD into one or more Architecture Decision Records (ADRs). Provide a PRD file path or paste PRD content.
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
  webfetch: deny
  websearch: deny
  task: deny
  todowrite: deny
---

Extract discrete architectural decisions from a PRD and produce one or more ADRs.

**Common rules:** Apply writing rules and self-correction protocol from `.opencode/COMMON.md`.

---

## Input

The user provides a PRD as a file path or pasted content.

Read the PRD, then read these references before writing any ADRs:

1. `docs/architecture/adrs/README.md` — ADR template and index
2. `docs/architecture/domain-model.md` — entity, relationship, component types
3. `docs/architecture/pipeline.md` — seven-layer architecture
4. `docs/philosophy/engineering-principles.md` — 10-question checklist

---

## What Counts as an Architectural Decision

A decision that defines or constrains how the system is built, has meaningful alternatives, and has consequences for the canonical model, pipeline layers, storage, events, or plugin system.

Extract decisions from the PRD's: engineering questions, architecture sections, pipeline spine analysis, non-functional requirements, and dependency choices.

Do NOT create ADRs for: implementation details with no alternatives, bug fixes, documentation changes, or configuration changes.

---

## Verify-First Gate

Before writing any ADR, run these checks. Fix failures before proceeding.

1. **Reference validation:** Verify every file referenced in the PRD exists (`glob` or `ls`)
2. **Type consistency:** Verify entity/component/relationship types exist in `docs/architecture/domain-model.md`
3. **ADR number uniqueness:** Verify the next ADR number does not already exist
4. **Existing ADR scan:** Read ADRs the new one will reference — confirm relationships are accurate and no contradiction exists
5. **Template compliance:** Verify output matches the ADR template structure

---

## Process

### Step 1: Determine next ADR number

`ls docs/architecture/adrs/adr-*.md` → next = max + 1, padded to 4 digits.

### Step 2: Identify decisions

Read the PRD's "Engineering Questions" section. Each question introducing something new implies one or more decisions. Also examine architecture sections, pipeline spine, non-functional requirements, and external crate choices. One decision per ADR. Never combine multiple decisions.

### Step 3: Write each ADR

Follow the template in `docs/architecture/adrs/README.md`. For each decision:
1. Derive **Context** from the PRD's problem statement, engineering questions, landscape evidence
2. Derive **Decision** from the PRD's architecture section, trait definitions, pipeline analysis
3. Write **Alternatives Considered** — at least 2 rejected options with genuine pros/cons (not strawmen)
4. Write **Consequences** — positive, negative, risks (reference the PRD's risks section)
5. Write **Related Decisions** — link to existing ADRs this builds on

### Step 4: Self-check

- Context accurately represents the PRD's problem statement
- Decision does not contradict any existing ADR
- Alternatives include at least 2 rejected options with genuine pros/cons
- Related Decisions links are accurate
- No speculative language

### Step 5: Write files and update index

Write each ADR to `docs/architecture/adrs/adr-NNNN.md`. Update `docs/architecture/adrs/README.md` with new entries (status: `Proposed`).

---

## Output

After writing all ADRs, summarize: number of ADRs, ADR numbers and titles, decisions extracted, related existing ADRs, and any self-corrections made.
