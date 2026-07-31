# UX Audit: Timeline

**Date:** 2026-07-30
**Auditor:** UX Audit Agent
**View file:** `desktop/src/views/Timeline.svelte`
**Overall score:** 6/11 passed, 4 partial, 1 fail

---

## Summary

The Timeline is the best-implemented view in the desktop app. It has skeleton loading, stalled-load detection with warning, error retry, relative date grouping ("Today", "Yesterday", "This Week"), abort controller for cancellation, and proper accessibility attributes on entry cards. Its only critical issue is that type colors are hardcoded. The type filter uses a proper `<select>` dropdown (unlike Browser and Tree).

---

## Principle Audit

### 1. Goal-Oriented Design — Pass

The goal is to browse entities chronologically. Groups are clearly labeled with relative dates. Entry cards are clickable. Empty state guides the user to import.

### 2. Reduce Cognitive Load — Pass

Relative date grouping reduces cognitive effort. Sticky group headers maintain context while scrolling. Entry cards show only essential info (type, title, date).

### 3. Predictability — Pass

Clicking an entry opens detail. Filter changes are immediate. Standard timeline patterns.

### 4. Immediate Feedback — Pass

Skeleton loading is well-implemented (lines 172–188). Stalled loading shows a warning after 5 seconds (line 221). Error state has retry button. Loading state disables the filter.

### 5. Consistency — Fail

Type colors are hardcoded (lines 90–93: `typeColors = ["#3b82f6", "#10b981", ...]`). Design system defines per-type tokens. The type badge uses the hardcoded color (line 257).

### 6. Intelligent Defaults — Partial

Filter defaults to "All types" (correct). But filter state is not persisted across view changes.

### 7. Prefer Selection Over Input — Pass

Type filter is a `<select>` dropdown (line 122) populated from actual entity types. Correct pattern.

### 8. Error Tolerance — Pass

Error state has retry button (line 138). Stalled state has retry button (line 223). Empty state for filter mismatch is separate from empty database state (lines 230–234).

### 9. Reversible Actions — Pass

Filter is reversible. Navigation back via sidebar.

### 10. Performance Perception — Pass

Skeleton loader with pulse animation (line 452). Abort controller cancels in-flight requests. Stalled detection warns the user. Best-in-class loading UX.

### 11. User Confidence — Pass

Clear group labels. Entity count per group. Empty state provides guidance. Error states are recoverable.

---

## Design System Compliance

### Token Usage
Uses `--spacing-*`, `--font-size-*`, `--bg-*`, `--border`, `--radius-*` tokens correctly. Uses `--accent` for hover states.

### Entity Type Colors
**Non-compliant.** Hardcoded `typeColors` array. Must use design system entity type tokens.

### Accessibility
- Entry cards have `role="button"`, `tabindex="0"`, `aria-label`, and `onkeydown` Enter handler — excellent
- Filter select is disabled during loading — good
- Sticky group headers may cause issues for screen readers
- Group header has `position: sticky` but no ARIA indication

---

## Interaction Heuristics

| Heuristic | Status | Notes |
| --------- | ------ | ----- |
| One primary action | Pass | Click entry to view detail |
| Minimized clicks | Pass | Single click |
| Reduced context switching | Pass | Self-contained |
| No unnecessary complexity | Pass | Clean timeline |
| System state visible | Pass | Loading, stalled, error, empty all handled |

---

## Critical Issues (P0)

1. **Hardcoded type colors** — Must use design system entity type color tokens

## Major Issues (P1)

2. **No search** — Cannot search within timeline
3. **No "jump to date"** — Cannot navigate to a specific date range
4. **Filter state not persisted** — Resets on view change
5. **No entity count in header** — Should show total count

## Minor Issues (P2)

6. **No infinite scroll** — All entries rendered at once
7. **Sticky headers may overlap** — On small screens, sticky group headers could overlap content
8. **No export** — Cannot export timeline data

---

## Recommendations

1. Replace hardcoded `typeColors` with design system entity type color tokens
2. Add a search input to filter by title within timeline
3. Add date range picker for "jump to date" navigation
4. Persist filter state in session storage
5. Show entity count in header ("Timeline (42 entities)")
6. Add virtual scrolling for large entity sets
7. This view is the reference implementation — use its patterns (skeleton, stalled detection, abort controller, retry) as the standard for all other views
