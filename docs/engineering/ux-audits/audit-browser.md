# UX Audit: Browser

**Date:** 2026-07-30
**Auditor:** UX Audit Agent
**View file:** `desktop/src/views/Browser.svelte`
**Overall score:** 3/11 passed, 4 partial, 4 fail

---

## Summary

The Browser view is the simplest entity listing but has fundamental UX problems: the type filter is a free-text input that requires the user to know and type the exact entity type string, there is no entity count, no sorting, no pagination, and type badges use a single accent color for all types. It is the most design-system-noncompliant view in the application.

---

## Principle Audit

### 1. Goal-Oriented Design — Partial

The goal is to browse and find entities. The table shows entities, but without sorting, pagination, or entity count, the user cannot efficiently locate what they need in large datasets.

### 2. Reduce Cognitive Load — Pass

Simple table layout. Columns are clear. No unnecessary information.

### 3. Predictability — Pass

Clicking a row opens entity detail. Standard table interaction.

### 4. Immediate Feedback — Partial

Loading state shows plain text "Loading..." (line 51). No skeleton.

### 5. Consistency — Fail

- Type badge uses `var(--accent)` for ALL entity types (line 127). The design system defines per-type colors.
- Filter is a free-text `<input>` (line 43) while Table view uses a `<select>` dropdown. Inconsistent filter patterns across views.
- Different styling from Table view (which has sortable headers, search icon, etc.)

### 6. Intelligent Defaults — Fail

No remembered sort order. No remembered filter. No default view preferences. Every visit starts from scratch.

### 7. Prefer Selection Over Input — Fail

The type filter is a free-text input (line 43: `placeholder="Filter by type..."`). The user must type the exact type string (e.g., "Paper", "Concept"). This violates the principle directly. A dropdown/select with all available entity types is the correct pattern.

### 8. Error Tolerance — Partial

If the user types an incorrect type, the table shows "No entities found" with no suggestion of valid types. No validation feedback.

### 9. Reversible Actions — Pass

Read-only view. Navigation is reversible.

### 10. Performance Perception — Fail

No skeleton loader. "Loading..." text (line 51). No optimistic display of previously loaded entities.

### 11. User Confidence — Partial

"No entities found" message (line 53) is unhelpful — it could mean no entities exist, or the filter is wrong. No guidance on what to do next.

---

## Design System Compliance

### Token Usage
Uses `--spacing-*`, `--font-size-*`, `--bg-*`, `--border`, `--radius-*` tokens.

### Entity Type Colors
**Non-compliant.** All type badges use `var(--accent)` regardless of entity type. Must use per-type tokens.

### Accessibility
- Table rows are clickable but use `<tr>` with `onclick` — not keyboard accessible (no `tabindex`, no `role="button"`, no Enter key handler)
- No ARIA labels on table
- Filter input has no `<label>` element
- No column sort indicators

---

## Interaction Heuristics

| Heuristic | Status | Notes |
| --------- | ------ | ----- |
| One primary action | Pass | Click row to view detail |
| Minimized clicks | Pass | Single click opens detail |
| Reduced context switching | Pass | Browse is self-contained |
| No unnecessary complexity | Pass | Simple table |
| System state visible | Fail | No entity count, no loading skeleton |

---

## Critical Issues (P0)

1. **Free-text type filter** — Must be replaced with a dropdown/select populated from available entity types
2. **Single accent color for all type badges** — Must use design system entity type colors
3. **Table rows not keyboard accessible** — Must add `tabindex`, `role="button"`, and Enter key handler

## Major Issues (P1)

4. **No entity count displayed** — User doesn't know how many entities exist
5. **No sorting** — Table has no sortable columns
6. **No pagination or virtualization** — Large entity sets will render all rows
7. **Plain text loading state** — Needs skeleton loader
8. **No error retry** — Same issue as Dashboard

## Minor Issues (P2)

9. **No search integration** — Cannot search within browser
10. **No multi-select or batch actions** — Cannot select multiple entities
11. **"No entities found" is ambiguous** — Could mean filter mismatch or empty database

---

## Recommendations

1. Replace free-text filter with `<select>` dropdown populated from entity types
2. Use per-entity-type colors from design system for type badges
3. Add `tabindex="0"`, `role="button"`, `onkeydown` Enter handler to table rows
4. Add entity count to header ("Browse Entities (42)")
5. Add sortable column headers
6. Add skeleton loader for loading state
7. Improve empty state with guidance ("No entities found. Try clearing the filter or import documents.")
8. Add keyboard navigation (arrow keys between rows)
