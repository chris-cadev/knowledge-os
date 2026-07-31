# UX Audit: Graph

**Date:** 2026-07-30
**Auditor:** UX Audit Agent
**View file:** `desktop/src/views/Graph.svelte`
**Overall score:** 3/11 passed, 4 partial, 4 fail

---

## Summary

The Graph view is the visual centerpiece of the desktop app — an SVG-based force-directed graph with pan/zoom, node selection, and an entity inspector panel. Its critical UX failure is requiring the user to type a raw entity UUID into a text field to begin exploration (line 199: `placeholder="Entity ID..."`). This makes the view unusable for any user who doesn't have entity IDs memorized. The node color map is also hardcoded and inconsistent with the design system.

---

## Principle Audit

### 1. Goal-Oriented Design — Fail

The goal is to explore the knowledge graph visually. But the entry point requires typing a UUID (line 199). Users do not know entity UUIDs. The view is effectively locked behind a barrier that prevents the primary goal. The empty state (line 237) says "Enter an entity ID and click Explore" — this assumes knowledge the user does not have.

### 2. Reduce Cognitive Load — Partial

The canvas is clean once loaded. The inspector panel provides focused detail. But the traversal controls bar (lines 195–223) presents three inputs plus a button simultaneously, and the depth slider has no explanation of what "depth" means.

### 3. Predictability — Pass

Pan/zoom is standard (mouse drag to pan, scroll wheel to zoom). Click selects a node. Double-click re-centers the graph on that node. Zoom buttons work as expected.

### 4. Immediate Feedback — Pass

Loading overlay shows spinner (line 229). Node hover shows visual feedback (CSS line 507). Node selection shows highlight ring. Zoom controls respond immediately.

### 5. Consistency — Fail

Node colors use a hardcoded `colorMap` (line 177) with different values than the design system entity type colors. For example:
- Design system: Concept = `#8B5CF6`, Graph: Concept = `#004ac6`
- Design system: Person = `#EC4899`, Graph: Person = `#22c55e`
- Design system: Paper = `#3B82F6`, Graph: Paper = `#ec4899`

These are completely different from the design system tokens.

### 6. Intelligent Defaults — Fail

No default entity pre-selected. The `startId` defaults to `app.selectedEntityId` (line 17) but this is only set if the user navigated from another view. A new user sees an empty canvas with no guidance.

### 7. Prefer Selection Over Input — Fail

Entity ID input is a raw text field (line 199). Must be replaced with an autocomplete/searchable entity picker. Type filter is also a free-text input (line 213).

### 8. Error Tolerance — Partial

Graph load error shows status message (line 91) but no retry button on the canvas. Invalid entity ID silently produces an empty graph.

### 9. Reversible Actions — Pass

Zoom reset button (line 161). Inspector panel can be closed. Navigation back via sidebar.

### 10. Performance Perception — Pass

Loading overlay with spinner. Force simulation runs with rAF batching (line 82). Smooth animation during layout.

### 11. User Confidence — Partial

The breadcrumb shows focus ID as truncated UUID (line 329: `startId.slice(0, 8)...`). This is not helpful — the user doesn't know which entity this refers to by UUID prefix.

---

## Design System Compliance

### Token Usage
Uses some tokens (`--spacing-*`, `--bg-*`, `--border`, `--radius-*`, `--font-*`).

### Entity Type Colors
**Non-compliant.** Hardcoded `colorMap` with values that conflict with the design system. Must use `--color-entity-*` tokens.

### Accessibility
- SVG has `role="img"` and `aria-label` — good
- Nodes have `role="button"` and `tabindex="0"` — good
- Node click handler has `onkeydown` for Enter — good
- No ARIA labels on zoom controls
- No `aria-label` on depth slider
- Inspector panel not focus-trapped
- Legend is not keyboard accessible

---

## Interaction Heuristics

| Heuristic | Status | Notes |
| --------- | ------ | ----- |
| One primary action | Fail | No clear entry point without UUID |
| Minimized clicks | Partial | Double-click to re-center is good |
| Reduced context switching | Pass | Inspector panel keeps context |
| No unnecessary complexity | Partial | Depth slider unexplained |
| System state visible | Pass | Loading, breadcrumb, legend visible |

---

## Critical Issues (P0)

1. **Entity ID text input** — Must be replaced with autocomplete/searchable entity picker
2. **Hardcoded node colors** — Must use design system entity type color tokens
3. **No default entity** — Must show a default graph or prompt entity selection via picker

## Major Issues (P1)

4. **Type filter is free-text** — Must be a dropdown/select
5. **Breadcrumb shows UUID prefix** — Must show entity title
6. **No edge labels** — Relationship types not visible on edges
7. **Empty state is unhelpful** — Must guide the user to select an entity
8. **No keyboard navigation between nodes** — Arrow keys should traverse edges

## Minor Issues (P2)

9. **Legend is static** — Only shows types present in the graph
10. **No fit-to-screen button** — User must manually reset zoom
11. **Inspector panel position** — Fixed bottom-left may overlap with graph content

---

## Recommendations

1. Replace entity ID text input with an autocomplete entity picker (search by title, show type badge)
2. Replace hardcoded `colorMap` with design system entity type color tokens
3. If no `startId`, show all entities or prompt with the entity picker
4. Replace type filter text input with `<select>` dropdown
5. Show entity title in breadcrumb instead of UUID prefix
6. Add edge labels for relationship types (or show on hover)
7. Improve empty state with entity picker and guidance text
8. Add keyboard navigation: Tab between nodes, Enter to select, arrow keys to traverse
9. Add "fit to screen" button
10. Make legend dynamic — only show types present in current graph
