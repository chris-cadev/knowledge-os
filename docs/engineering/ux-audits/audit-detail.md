# UX Audit: Detail

**Date:** 2026-07-30
**Auditor:** UX Audit Agent
**View file:** `desktop/src/views/Detail.svelte`
**Overall score:** 4/11 passed, 5 partial, 2 fail

---

## Summary

The Detail view displays comprehensive entity information: type badge, components, relationships (incoming/outgoing), events, and version history. Its strength is the thorough data display. Its critical weaknesses are: the page title shows "Entity Detail" instead of the entity's actual title, components render as raw JSON (including Markdown content), there is no archive/restore action, and entity type badge colors are not from the design system.

---

## Principle Audit

### 1. Goal-Oriented Design — Partial

The goal is to inspect an entity. The view shows all data, but the most important information (entity title) is displayed as a truncated ID hash (line 137: `{detail.id.slice(0, 8)}...`) rather than the entity's actual title from its Title component. The user must scan through components to find the title.

### 2. Reduce Cognitive Load — Partial

Components are listed sequentially. Content component renders as preformatted text (line 215). No progressive disclosure — all sections (components, relationships, events, versions) are expanded at once. For entities with many relationships, this creates information overload.

### 3. Predictability — Pass

Clicking a relationship navigates to the target entity. "View in Graph" navigates to graph view. "Close" returns to browse. Standard patterns.

### 4. Immediate Feedback — Pass

Loading state shows a spinning icon (line 122). Error state shows error icon and message. Source file actions show loading spinners during operations.

### 5. Consistency — Fail

- Type badge uses `var(--accent)` for all types (line 367). Design system defines per-type colors.
- Entity header shows "Entity Detail" as h2 (line 117) — not the entity's actual title.
- Relationship items use different styling than other views.
- Component cards render all data as `<pre>` JSON — inconsistent with how content should be rendered (Markdown should be rendered as HTML).

### 6. Intelligent Defaults — Partial

If navigated from another view, the selected entity is pre-loaded. But no scroll position memory or section collapse state.

### 7. Prefer Selection Over Input — Pass

No free-text inputs. Navigation is through buttons and clickable items.

### 8. Error Tolerance — Pass

Error state has a clear message and "Go Back" button (line 129). Source file operations show loading state and error messages.

### 9. Reversible Actions — Partial

"Close" navigates back. But there is no archive/restore button. The user cannot change entity state from this view. No undo for any action.

### 10. Performance Perception — Pass

Loading spinner with text. Source actions show loading state. Good feedback during async operations.

### 11. User Confidence — Partial

Error state is clear. But the entity title is not visible — showing a truncated UUID hash (line 137) makes the user feel like they're looking at a database record, not a knowledge entity.

---

## Design System Compliance

### Token Usage
Uses `--spacing-*`, `--font-size-*`, `--bg-*`, `--border`, `--radius-*` tokens. Uses `--font-mono` for entity ID.

### Entity Type Colors
**Non-compliant.** Type badge uses `var(--accent)` for all types.

### Accessibility
- Close button has `aria-label="Close detail panel"` — good
- Source action buttons have `aria-busy` during loading — good
- Entity ID heading is an `<h3>` but should be the entity title
- Relationship buttons are `<button>` elements — good
- No ARIA landmarks for sections
- Component cards use `<pre>` which is not screen-reader-friendly for Markdown content

---

## Interaction Heuristics

| Heuristic | Status | Notes |
| --------- | ------ | ----- |
| One primary action | Partial | Multiple actions but none clearly primary |
| Minimized clicks | Pass | Direct navigation from relationships |
| Reduced context switching | Pass | All entity info on one page |
| No unnecessary complexity | Partial | All sections expanded at once |
| System state visible | Pass | Loading, error, and active states shown |

---

## Critical Issues (P0)

1. **Entity title not displayed** — Shows truncated UUID hash instead of Title component value
2. **Markdown content rendered as raw text** — Content component should render Markdown as HTML
3. **Type badge not using design system colors** — Uses `var(--accent)` for all types

## Major Issues (P1)

4. **No archive/restore button** — User cannot change entity state
5. **No progressive disclosure** — All sections expanded; should collapse long lists
6. **No breadcrumb** — No indication of navigation path
7. **No edit capability** — Cannot modify components from this view
8. **Relationship items don't show entity type badges** — Only shows type label for relationship, not target entity type

## Minor Issues (P2)

9. **Version history shows only version number and timestamp** — No diff or change description
10. **Event list shows raw event types** — Could be more human-readable
11. **No keyboard shortcuts** — No way to navigate sections via keyboard

---

## Recommendations

1. Extract entity title from Title component and display as the primary heading
2. Render Content component as Markdown HTML (use a Markdown renderer)
3. Use per-entity-type colors from design system for type badge
4. Add archive/restore button
5. Add collapsible sections for relationships, events, versions (especially for entities with many)
6. Add breadcrumb: "Browse > [Type] > [Title]"
7. Show entity type badge on relationship target items
8. Add keyboard navigation between sections (Tab/Shift+Tab)
9. Show relationship count in section headers (already done — good)
10. Add "Copy ID" button for the entity UUID
