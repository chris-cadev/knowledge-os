# UX Audit: StatusBar

**Date:** 2026-07-30
**Auditor:** UX Audit Agent
**View file:** `desktop/src/views/StatusBar.svelte`
**Overall score:** 5/11 passed, 4 partial, 2 fail

---

## Summary

The StatusBar is a compact bottom bar showing entity count, provider status, current view name, and theme indicator. It auto-dismisses status messages after 5 seconds. It's functional but minimal. Its issues are: the provider status color uses a hardcoded red (`#e53e3e`, line 78) instead of a design system token, the theme indicator is an emoji (not accessible), and the status message area is not clickable or actionable.

---

## Principle Audit

### 1. Goal-Oriented Design — Pass

System status is the goal. Entity count, provider reachability, and current view are all visible at a glance.

### 2. Reduce Cognitive Load — Pass

Compact single-line display. Information is secondary — doesn't compete with main content.

### 3. Predictability — Pass

Status messages auto-dismiss after 5 seconds. Provider status shows reachable/unreachable. Consistent layout.

### 4. Immediate Feedback — Pass

Status messages appear immediately when set. Provider status updates in real-time.

### 5. Consistency — Fail

Provider unreachable color is hardcoded `#e53e3e` (line 78). Theme indicator uses emoji `🌙`/`☀️` (line 36) — inconsistent with the icon-based patterns used elsewhere. Should use design system tokens and material icons.

### 6. Intelligent Defaults — Pass

Shows relevant context (entity count, provider, view name) without user configuration.

### 7. Prefer Selection Over Input — Pass

Read-only display. No input required.

### 8. Error Tolerance — Pass

Status messages are informational. Provider unreachable is indicated but doesn't block interaction.

### 9. Reversible Actions — Pass

Read-only. No actions to reverse.

### 10. Performance Perception — Pass

Instant display. Auto-dismiss prevents clutter.

### 11. User Confidence — Partial

Provider status gives confidence about AI availability. But status messages are transient — user may miss important messages. No status history.

---

## Design System Compliance

### Token Usage
Uses `--space-*`, `--font-size-*`, `--border`, `--text-secondary`, `--accent`, `--color-surface-container-high` tokens.

### Entity Type Colors
Not applicable.

### Accessibility
- No ARIA attributes
- Theme emoji is not announced meaningfully by screen readers
- Status message area has no `aria-live`
- Not keyboard accessible (no interactive elements, which is acceptable for a status bar)
- Provider status color is not conveyed by text — color-only indication

---

## Interaction Heuristics

| Heuristic | Status | Notes |
| --------- | ------ | ----- |
| One primary action | Pass | Information display |
| Minimized clicks | Pass | No interaction needed |
| Reduced context switching | Pass | Always visible |
| No unnecessary complexity | Pass | Single line |
| System state visible | Pass | Entity count, provider, view, theme |

---

## Critical Issues (P0)

None.

## Major Issues (P1)

1. **Hardcoded color** — `#e53e3e` must use design system `--color-error` token
2. **Theme emoji not accessible** — Replace with material icon and `aria-label`
3. **No `aria-live` on status message** — Screen readers won't announce status changes
4. **Status messages are transient** — Important messages may be missed. Add a status history or notification center.
5. **Provider status is color-only** — Add text label ("Connected" / "Disconnected")

## Minor Issues (P2)

6. **Not clickable** — Status bar items could be clickable (e.g., click provider to go to Settings)
7. **No connection latency** — Could show last-known latency
8. **No version number** — Could show app version

---

## Recommendations

1. Replace hardcoded `#e53e3e` with `--color-error` or `var(--danger)` token
2. Replace theme emoji with material icon (`dark_mode`/`light_mode`) and `aria-label`
3. Add `aria-live="polite"` to status message area
4. Add text label for provider status ("Connected" / "Disconnected")
5. Add a notification/status history panel (click status bar to expand)
6. Make provider status clickable (navigate to Settings)
7. Add app version display
