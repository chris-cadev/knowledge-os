# UX Audit: Table

**Date:** 2026-07-30
**Auditor:** UX Audit Agent
**View file:** `desktop/src/views/Table.svelte`
**Overall score:** 4/11 passed, 4 partial, 3 fail

---

## Summary

The Table view provides sortable columns and client-side text search. It is the most structured list view. Its critical issues are: the type filter dropdown is empty (the loop on lines 101–105 produces no options), opening an entity requires double-click (not discoverable), and the search is client-side only (doesn't leverage server-side full-text search).

---

## Principle Audit

### 1. Goal-Oriented Design — Partial

Sortable columns and search support the goal of finding and comparing entities. But the type filter is broken (empty dropdown), and double-click to open is not discoverable.

### 2. Reduce Cognitive Load — Pass

Table layout is clean. Columns are limited to essential data. Search bar has icon indicator.

### 3. Predictability — Pass

Sort indicators show direction. Column headers are clickable. Standard table patterns.

### 4. Immediate Feedback — Partial

Loading shows "Loading..." text (line 112). No skeleton. Sort changes trigger a server reload without visual feedback.

### 5. Consistency — Partial

Search bar has an icon (consistent with Search view). But type filter is a `<select>` with no actual options — inconsistent with the working filter in Timeline view. Row open requires double-click while Browser uses single-click.

### 6. Intelligent Defaults — Fail

No remembered sort order. No remembered filter. No default column preferences. Client-side search text resets on view change.

### 7. Prefer Selection Over Input — Partial

Type filter is a `<select>` (correct pattern), but it's empty (lines 101–105 produce no options). The search is a text input (acceptable for free-text search).

### 8. Error Tolerance — Partial

Error shows status message (line 35). No retry button. Empty state is plain text.

### 9. Reversible Actions — Pass

Sort direction is reversible. Navigation back via sidebar.

### 10. Performance Perception — Fail

No skeleton. "Loading..." text (line 112). Sort changes trigger full reload with no indication.

### 11. User Confidence — Partial

Table footer shows "Showing X of Y entities" — helpful. But sort changes produce no visual feedback during loading.

---

## Design System Compliance

### Token Usage
Uses `--spacing-*`, `--font-size-*`, `--bg-*`, `--border`, `--radius-*` tokens correctly.

### Entity Type Colors
Not applicable — table doesn't render type badges with colors. The type column shows plain text.

### Accessibility
- Sort headers are clickable `<th>` elements — good but no `aria-sort` attribute
- Table rows use `ondblclick` — not keyboard accessible
- Search input has no `<label>` element
- No ARIA table roles
- Footer is not announced to screen readers

---

## Interaction Heuristics

| Heuristic | Status | Notes |
| --------- | ------ | ----- |
| One primary action | Partial | Double-click not discoverable |
| Minimized clicks | Fail | Double-click to open is two clicks |
| Reduced context switching | Pass | Self-contained |
| No unnecessary complexity | Pass | Clean table |
| System state visible | Partial | No loading skeleton, no sort feedback |

---

## Critical Issues (P0)

1. **Type filter dropdown is empty** — The loop on lines 101–105 produces no options. Must be populated with actual entity types.
2. **Double-click to open** — Not discoverable. Must be single-click like Browser view, or provide visual affordance.

## Major Issues (P1)

3. **No skeleton loader** — Plain text loading state
4. **Client-side search only** — Doesn't leverage server full-text search. For 100K entities, client-side filtering is impractical.
5. **Sort triggers full reload** — No visual feedback during sort
6. **No entity type color badges** — Type column shows plain text
7. **No `aria-sort` on sortable headers** — Accessibility gap

## Minor Issues (P2)

8. **No pagination** — All rows rendered at once
9. **No column resize** — Fixed column widths
10. **No column visibility toggle** — Cannot hide/show columns
11. **No export** — Cannot export table data

---

## Recommendations

1. Populate the type filter dropdown with entity types from the data
2. Change double-click to single-click for opening entities
3. Add skeleton loader
4. Add `aria-sort` attribute to sortable column headers
5. Consider server-side search integration
6. Add type badges with design system colors
7. Add visual feedback during sort (loading indicator on header)
8. Add pagination or virtual scrolling for large datasets
9. Add keyboard navigation (arrow keys between rows, Enter to open)
