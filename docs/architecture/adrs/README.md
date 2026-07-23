# Architecture Decision Records

> Significant architectural decisions are recorded here. Each decision is immutable once accepted.

---

## What Is an ADR?

An Architecture Decision Record (ADR) captures a significant architectural decision along with its context and consequences. ADRs are immutable once accepted -- they are superseded, not edited.

This practice is described by Michael Nygard in [Documenting Architecture Decisions](https://cognitect.com/blog/2011/11/15/documenting-architecture-decisions) and adopted widely in architecture-first projects.

---

## ADR Template

```markdown
# ADR-NNNN: [Short Title]

**Status:** Proposed | Accepted | Deprecated | Superseded by ADR-XXXX
**Date:** YYYY-MM-DD
**Deciders:** [people or team]

## Context

What is the issue motivating this decision or change?

## Decision

What is the change we are proposing and/or doing?

## Alternatives Considered

### Option 1: [Name]
- Pros: ...
- Cons: ...
- Why not chosen: ...

### Option 2: [Name]
- Pros: ...
- Cons: ...

## Consequences

### Positive
- ...

### Negative
- ...

### Risks
- ...

## Related Decisions
- ADR-XXXX: [related]
```

---

## ADR Lifecycle

```
Proposed --> Accepted --> [Deprecated | Superseded]
    |          |
 Rejected   Active (implemented)
```

- **Proposed.** Under discussion. Not yet decided.
- **Accepted.** Decision is final. Implementation may begin.
- **Deprecated.** Decision is no longer relevant. kept for historical reference.
- **Superseded.** Replaced by a newer ADR. The superseding ADR is referenced.

---

## ADR Index

| ADR      | Title                                                        | Status   | Date       |
| -------- | ------------------------------------------------------------ | -------- | ---------- |
| ADR-0001 | [Knowledge Model as Canonical Source of Truth](adr-0001.md)  | Accepted | 2026-07-21 |
| ADR-0002 | [Storage Independence via Adapter Pattern](adr-0002.md)      | Accepted | 2026-07-21 |
| ADR-0003 | [Entity Component Model for Knowledge Entities](adr-0003.md) | Accepted | 2026-07-21 |
| ADR-0004 | [Event-Driven Derivation Pipeline](adr-0004.md)              | Accepted | 2026-07-21 |
| ADR-0005 | [Compiler-Inspired Architecture](adr-0005.md)                | Accepted | 2026-07-21 |
| ADR-0006 | [Entity Resolution as Critical Layer](adr-0006.md)           | Accepted | 2026-07-22 |

---

## Rules

1. **One decision per ADR.** Do not combine multiple decisions in a single record.
2. **Immutability.** Once accepted, an ADR is never edited. Supersede it with a new one.
3. **Sequential numbering.** ADR numbers are never reused.
4. **Context is mandatory.** Every ADR must explain the problem it addresses.
5. **Consequences are mandatory.** Every ADR must describe positive, negative, and risk outcomes.
