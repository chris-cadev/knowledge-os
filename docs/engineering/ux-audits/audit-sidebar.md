# UX Audit: Sidebar

**Date:** 2026-07-30
**Auditor:** UX Audit Agent
**View file:** `desktop/src/views/Sidebar.svelte`
**Overall score:** 4/11 passed, 4 partial, 3 fail

---

## Summary

The Sidebar provides navigation between all 10 views. It has a clean list layout with icons, active state indicator (left stripe), and entity count in the footer. Its critical issues are: it cannot be collapsed/hidden (wastes screen space on small screens), there are no badge counts on nav items, no keyboard shortcut indicators, and the footer only shows total entity count without breakdown.

---

## Principle Audit

### 1. Goal-Oriented Design — Pass

Navigation is the goal. All views are reachable. Icons provide visual recognition. Active state is clear (left stripe).

### 2. Reduce Cognitive Load — Pass

10 items is manageable. Icons + labels provide dual recognition. Visual hierarchy is clear.

### 3. Predictability — Pass

Clicking a nav item navigates to the view. Active state updates. Standard sidebar patterns.

### 4. Immediate Feedback — Partial

Active state changes on click (CSS). But no hover tooltip showing keyboard shortcut.

### 5. Consistency — Pass

Uses design system sidebar tokens (`--color-sidebar-*`). Icon + label pattern is consistent.

### 6. Intelligent Defaults — Fail

No remembered last view. No frecency-based ordering. No badge counts showing unread or pending items.

### 7. Prefer Selection Over Input — Pass

Navigation is all selection-based (click items). No free-text input.

### 8. Error Tolerance — Pass

Navigation is non-destructive. No errors possible.

### 9. Reversible Actions — Pass

Navigation is fully reversible. Clicking between views is instant.

### 10. Performance Perception — Pass

Navigation is instant. No loading states needed for sidebar.

### 11. User Confidence — Pass

Clear navigation. Active state is visible. Entity count provides context.

---

## Design System Compliance

### Token Usage
Uses sidebar-specific tokens (`--color-sidebar-*`) and general tokens (`--font-size-*`, `--space-*`).

### Entity Type Colors
Not applicable — Sidebar is navigation.

### Accessibility
- Nav items are `<button>` elements — good
- No `role="navigation"` on `<aside>` (implicit role is fine, but explicit is better)
- No `aria-current="page"` on active item
- No keyboard shortcut indicators
- No tooltip on hover

---

## Interaction Heuristics

| Heuristic | Status | Notes |
| --------- | ------ | ----- |
| One primary action | Pass | Navigate to a view |
| Minimized clicks | Pass | Single click |
| Reduced context switching | Pass | Always visible |
| No unnecessary complexity | Pass | Simple list |
| System state visible | Partial | Active view shown, entity count shown |

---

## Critical Issues (P0)

1. **Cannot be collapsed** — On small screens or when maximizing content area, the sidebar wastes ~200px permanently. Must support collapse to icon-only mode or hide entirely.

## Major Issues (P1)

2. **No badge counts** — Nav items should show entity count or pending items (e.g., "Chat (3)" or "Import (2 errors)")
3. **No keyboard shortcut indicators** — Should show shortcut hints on hover (e.g., "Dashboard (1)" or "Search (/)")
4. **No `aria-current="page"` on active item** — Accessibility gap
5. **No tooltips** — Hover should show full view name and shortcut
6. **Entity count footer is too minimal** — Could show type breakdown or be clickable

## Minor Issues (P2)

7. **No view reordering** — Cannot customize nav item order
8. **No view hiding** — Cannot hide unused views
9. **Logo subtitle "v0.1.0 SYSTEM CORE"** — Static, not useful

---

## Recommendations

1. Add collapse/expand toggle (icon-only mode)
2. Add badge counts on nav items
3. Add keyboard shortcut indicators (tooltips or inline hints)
4. Add `aria-current="page"` to active nav item
5. Add tooltips on hover showing view name and shortcut
6. Make entity count footer clickable (navigate to dashboard)
7. Add view customization (reorder, hide)
8. Remove or make dynamic the version subtitle
