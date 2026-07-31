# UX Audit: Dashboard

**Date:** 2026-07-30
**Auditor:** UX Audit Agent
**View file:** `desktop/src/views/Dashboard.svelte`
**Overall score:** 4/11 passed, 5 partial, 2 fail

---

## Summary

The Dashboard provides a useful overview with bento-grid stats, quick actions, type distribution, and recent entities. Its primary strength is the clear information hierarchy and quick-action shortcuts. Its critical weakness is the reliance on a plain-text loading state instead of skeletons, hardcoded type colors that violate the design system, and lack of keyboard navigation — leaving the user without guidance on how to proceed beyond import.

---

## Principle Audit

### 1. Goal-Oriented Design — Pass

The primary action ("Import Documents") is prominent. Quick actions provide navigation shortcuts. The recent entities list makes it easy to resume work.

### 2. Reduce Cognitive Load — Pass

Information is organized into clear cards. The bento grid layout prevents overload. Stats are simple numbers.

### 3. Predictability — Pass

Clicking a recent entity navigates to detail. Quick action buttons navigate to expected views. Standard button patterns.

### 4. Immediate Feedback — Partial

Loading state shows plain text "Loading dashboard..." (line 81). No skeleton or shimmer. The user sees an empty screen during load.

**Issues:**
- No skeleton loader — plain text "Loading dashboard..." (line 81)
- No progress indicator during data fetch

### 5. Consistency — Fail

Type colors are hardcoded in a local array (line 59: `["#3b82f6", "#10b981", ...]`) instead of using the design system entity type colors (`--color-entity-concept`, `--color-entity-person`, etc.). The type badge in recent entities uses a single `--accent` color (line 375) for all types.

**Issues:**
- Hardcoded color array (line 59) ignores `ui-design-system.md` entity type tokens
- All type badges use `var(--accent)` regardless of entity type
- Inconsistent with Graph view, which also has its own hardcoded color map

### 6. Intelligent Defaults — Pass

Dashboard auto-loads on mount. Recent entities are pre-sorted by creation date. Type distribution is pre-computed.

### 7. Prefer Selection Over Input — Pass

No free-text inputs on this screen. Navigation is through buttons and clickable items.

### 8. Error Tolerance — Partial

Error is captured and displayed in `app.statusMessage` (line 40), but the error message is shown at the bottom of the page (line 176) with no retry button. The user must navigate away and back to retry.

**Issues:**
- No retry button on error
- Error shown as status message at bottom, easy to miss

### 9. Reversible Actions — Pass

Dashboard is read-only. Navigation is reversible via sidebar.

### 10. Performance Perception — Fail

No skeleton loader. During data fetch, the user sees "Loading dashboard..." text (line 81). No partial rendering or optimistic display.

### 11. User Confidence — Pass

Clear stats, clear actions, no confusing states. Empty state provides guidance ("No entities yet. Import some documents to get started.").

---

## Design System Compliance

### Token Usage
Uses `--spacing-*`, `--font-size-*`, `--bg-*`, `--border`, `--radius-*` tokens correctly.

### Entity Type Colors
**Non-compliant.** Uses a hardcoded local `typeColors` array instead of design system entity type tokens (`--color-entity-concept`, `--color-entity-paper`, etc.).

### Accessibility
- No keyboard navigation between bento cards
- No ARIA labels on stat cards
- Recent entity buttons are accessible (`<button>` elements)
- Missing focus indicators on action buttons

---

## Interaction Heuristics

| Heuristic | Status | Notes |
| --------- | ------ | ----- |
| One primary action | Pass | "Import Documents" is clear |
| Minimized clicks | Pass | Quick actions reduce clicks |
| Reduced context switching | Pass | All on one screen |
| No unnecessary complexity | Pass | Simple, focused |
| System state visible | Partial | No loading skeleton |

---

## Critical Issues (P0)

1. **Hardcoded entity type colors** — Must use design system tokens
2. **No skeleton loader** — Plain text loading state violates performance perception

## Major Issues (P1)

3. **No error retry button** — User must navigate away to retry
4. **No keyboard navigation** — Cards and quick actions not keyboard-navigable
5. **Status message at bottom** — Easy to miss, not prominent enough

## Minor Issues (P2)

6. **No "see all" link for recent entities** — Limited to 5, no way to see more
7. **No search access from dashboard** — User must navigate to search view
8. **No entity count on type distribution** — Only shows counts per type, not total context

---

## Recommendations

1. Replace hardcoded `typeColors` array with design system entity type color tokens
2. Add skeleton loader matching the bento grid layout
3. Add a retry button in the error state
4. Add `tabindex` and keyboard handlers to bento cards
5. Move status message to a toast/notification pattern or add inline error state
6. Add a "View all recent" link that navigates to browse view sorted by date
