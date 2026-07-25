# IP-NNN: <Title>

**Status:** Draft | In Progress | Complete
**ADR(s):** Links to related ADRs
**PRD(s):** Links to related PRDs
**Estimated effort:** <time>

---

## Context

Why this work exists. Link to ADR decisions, PRD user stories, and architecture docs.

---

## Deliverables

Each deliverable is a unit of work that can be completed, verified, and committed independently.

### D1: <Name>

**Purpose:** What this deliverable achieves
**Files:**

| File | Action | Description |
|------|--------|-------------|
| `path/to/file.rs` | Create | What goes in this file |
| `path/to/existing.rs` | Modify | What changes |

**Verification:**
- `cargo test -p <crate>` passes
- Specific test scenarios that must pass

**Exit criteria:** When this deliverable is done

---

## Execution Order

```
D1 (foundation) -> D2 (implementation) -> D3 (integration)
```

Deliverables must be implemented in this order. Each builds on the previous.

---

## Verification Strategy

| Level | Command | Coverage |
|-------|---------|----------|
| Unit | `cargo test -p <crate>` | Internal logic |
| Integration | `cargo test -p <crate> --test integration_test` | Cross-module |
| E2E | `cargo test --test cucumber -p knowledge-cli` | CLI behavior |
| Lint | `cargo clippy -- -D warnings && cargo fmt --check` | Code quality |

---

## Exit Criteria

- [ ] All deliverables complete
- [ ] All verification levels pass
- [ ] No new compiler warnings
- [ ] ADR(s) updated with Implementation Notes if any deviations

---

## Implementation Notes

*(Filled in during/after implementation -- records deviations, discoveries, decisions made during coding)*
