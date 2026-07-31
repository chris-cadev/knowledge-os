# UX Audit: Tree

**Date:** 2026-07-30
**Auditor:** UX Audit Agent
**View file:** `desktop/src/views/Tree.svelte`
**Overall score:** 5/11 passed, 4 partial, 2 fail

---

## Summary

The Tree view groups entities hierarchically by type with expand/collapse. It has a clean, simple layout with connector lines. Its main issues are: the type filter is free-text, there is no expand/collapse all control, keyboard navigation is absent, and nested items show unhelpful "Sub-items (N)" labels instead of meaningful content.

---

## Principle Audit

### 1. Goal-Oriented Design — Pass

The goal is to browse entities by category. Tree groups by type clearly. Clicking an entity opens detail.

### 2. Reduce Cognitive Load — Pass

Hierarchical layout is intuitive. Connector lines show structure. Expand/collapse reveals complexity progressively.

### 3. Predictability — Pass

Chevron icons indicate expand/collapse state. Clicking entity rows navigates to detail. Standard tree patterns.

### 4. Immediate Feedback — Partial

Loading shows "Loading..." text (line 65). No skeleton. Tree content appears after load without transition.

### 5. Consistency — Fail

Type badges use `var(--accent)` for all types (line 211). Design system defines per-type colors. Filter is free-text input (same issue as Browser).

### 6. Intelligent Defaults — Fail

Tree state (expanded/collapsed nodes) is not persisted. Every visit starts fully collapsed. No remembered filter.

### 7. Prefer Selection Over Input — Fail

Type filter is a free-text input (line 55: `placeholder="Filter by type..."`). Must be a dropdown.

### 8. Error Tolerance — Partial

Error shows status message (line 25). No retry button. "No entities found" is ambiguous.

### 9. Reversible Actions — Pass

Expand/collapse is reversible. Navigation back via sidebar.

### 10. Performance Perception — Partial

No skeleton loader. Loading text only. No progressive rendering.

### 11. User Confidence — Partial

"Sub-items (N)" label (line 108) is cryptic — it doesn't tell the user what's inside. No guidance for empty states.

---

## Design System Compliance

### Token Usage
Uses `--spacing-*`, `--font-size-*`, `--bg-*`, `--border`, `--radius-*` tokens.

### Entity Type Colors
**Non-compliant.** All type badges use `var(--accent)`.

### Accessibility
- Expand/collapse buttons are `<button>` elements — good
- No `aria-expanded` on toggle buttons
- Entity rows are `<button>` elements — good
- No keyboard navigation between tree nodes
- No ARIA tree role or treeitem roles

---

## Interaction Heuristics

| Heuristic | Status | Notes |
| --------- | ------ | ----- |
| One primary action | Pass | Click entity to view detail |
| Minimized clicks | Partial | Must expand each type separately |
| Reduced context switching | Pass | Self-contained view |
| No unnecessary complexity | Pass | Simple tree |
| System state visible | Partial | No entity count in header |

---

## Critical Issues (P0)

1. **Free-text type filter** — Must be a dropdown
2. **Type badges not using design system colors** — Uses `var(--accent)` for all

## Major Issues (P1)

3. **No expand/collapse all** — User must expand each type individually
4. **"Sub-items (N)" is unhelpful** — Should show the relationship type or child type
5. **No keyboard navigation** — Cannot navigate tree with arrow keys
6. **Loading state is plain text** — Needs skeleton
7. **No entity count** — Header should show total

## Minor Issues (P2)

8. **No search within tree** — Cannot filter by title
9. **Tree state not persisted** — Expanded nodes reset on revisit
10. **No drag-and-drop** — Cannot reorder or move entities

---

## Recommendations

1. Replace free-text filter with `<select>` dropdown
2. Use design system entity type color tokens
3. Add "Expand All" / "Collapse All" buttons
4. Show meaningful labels for nested items (relationship type or child entity type)
5. Add keyboard navigation (arrow keys between nodes, Enter to select, Right/Left to expand/collapse)
6. Add `aria-expanded` to toggle buttons
7. Add skeleton loader
8. Show entity count in header ("Tree View (42 entities)")
9. Persist expanded state in session storage
