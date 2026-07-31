# UX Audit: Search

**Date:** 2026-07-30
**Auditor:** UX Audit Agent
**View file:** `desktop/src/views/Search.svelte`
**Overall score:** 3/11 passed, 4 partial, 4 fail

---

## Summary

The Search view provides full-text search with debounced input and type/tag filtering. Its critical issues are: type and tag filters are free-text inputs (violating "Prefer Selection Over Input"), the raw BM25 score is displayed to users (not meaningful), there is no search mode toggle (keyword/semantic/hybrid), no recent searches, no autocomplete, and no keyboard navigation of results.

---

## Principle Audit

### 1. Goal-Oriented Design — Partial

The goal is to find entities. The search bar is prominent. But results show raw scores (line 107: `Score: {result.score.toFixed(2)}`) which is not meaningful to users. No guidance on how to refine searches.

### 2. Reduce Cognitive Load — Partial

Search bar is simple. But the two filter inputs below add complexity. Filter labels ("Filter by type...", "Filter by tag...") require the user to know exact type/tag strings.

### 3. Predictability — Pass

Debounced search triggers automatically. Results appear as a list. Clicking a result opens detail. Standard search patterns.

### 4. Immediate Feedback — Partial

Loading spinner in search bar (line 74). But no skeleton for results. Results appear/disappear instantly when loading completes.

### 5. Consistency — Partial

Search bar styling matches other views. But the filter inputs are free-text while other views (Timeline) use dropdowns. Result cards use the same accent-colored type badge as other views.

### 6. Intelligent Defaults — Fail

No recent searches. No suggested searches. No search history. Every search starts from scratch.

### 7. Prefer Selection Over Input — Fail

Both filters are free-text inputs (lines 79–91). The user must type exact type strings ("Paper", "Concept") and exact tag strings. These must be dropdowns/selects populated from available types and tags.

### 8. Error Tolerance — Partial

"No results found" (line 97) is shown but with no suggestions. No "did you mean?" or alternative search terms. If search fails, status message is shown (line 41).

### 9. Reversible Actions — Pass

Clearing the search box returns to empty state. Navigation back via sidebar.

### 10. Performance Perception — Partial

Loading spinner in search bar (line 74). But no skeleton for results area. Results flash in after loading.

### 11. User Confidence — Partial

"No results found" is clear but unhelpful. No guidance on how to broaden the search.

---

## Design System Compliance

### Token Usage
Uses `--spacing-*`, `--font-size-*`, `--bg-*`, `--border`, `--radius-*` tokens.

### Entity Type Colors
Type badge in results uses `var(--accent)` for all types (line 224). Non-compliant.

### Accessibility
- Search input has no `<label>` element
- Filter inputs have no `<label>` elements
- Results are `<button>` elements — good
- No keyboard navigation between results (arrow keys)
- No `aria-live` for search results updates
- No `role="search"` on the search container

---

## Interaction Heuristics

| Heuristic | Status | Notes |
| --------- | ------ | ----- |
| One primary action | Pass | Type to search |
| Minimized clicks | Partial | Filter requires typing + clicking result |
| Reduced context switching | Pass | Self-contained |
| No unnecessary complexity | Partial | Raw score display adds confusion |
| System state visible | Partial | Loading spinner but no result skeleton |

---

## Critical Issues (P0)

1. **Free-text type filter** — Must be a dropdown populated from available entity types
2. **Free-text tag filter** — Must be a dropdown/autocomplete populated from available tags
3. **Type badge not using design system colors** — Uses `var(--accent)` for all types

## Major Issues (P1)

4. **Raw score displayed** — BM25 score (0.00–N.NN) is meaningless to users. Remove or replace with relevance indicator (high/medium/low)
5. **No search mode toggle** — Cannot switch between keyword/semantic/hybrid search
6. **No keyboard navigation of results** — Arrow keys should navigate between results
7. **No recent searches** — Every search starts from scratch
8. **No autocomplete** — Search bar doesn't suggest entity titles or tags
9. **Missing ARIA roles** — No `role="search"`, no `aria-live` for results

## Minor Issues (P2)

10. **No search result count** — Should show "Found X results"
11. **No "clear search" button** — Must manually delete text
12. **No search in other views** — Search is isolated to this view

---

## Recommendations

1. Replace type filter with `<select>` dropdown
2. Replace tag filter with `<select>` or searchable dropdown
3. Use design system entity type colors for result type badges
4. Remove raw score display or replace with relevance indicator
5. Add search mode toggle (keyword / semantic / hybrid)
6. Add keyboard navigation (arrow keys between results, Enter to open)
7. Add recent searches (stored in session/local storage)
8. Add autocomplete for entity titles
9. Add `role="search"`, `aria-live="polite"` for results
10. Add search result count ("Found 12 results")
