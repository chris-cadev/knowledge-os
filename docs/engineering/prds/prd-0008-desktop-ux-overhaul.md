# PRD-0008: Desktop UX Overhaul

**Status:** Draft
**Date:** 2026-07-30
**Author:** Core maintainers
**Priority:** P0 — Experience Layer
**Depends on:** PRD-0001, PRD-0003, PRD-0004

---

## Purpose

This PRD fixes the UX deficits identified by auditing all 13 desktop views against the interaction design principles in `docs/architecture/interaction-design.md`. The audits found 5 critical (P0) patterns, 31 major (P1) issues, and 38 minor (P2) improvements. This PRD consolidates the P0 and P1 fixes into a single work plan. No new features are introduced — this is a quality and compliance pass.

---

## Problem Statement

The desktop application has a functional implementation of all 13 views, but the views violate the interaction design principles established in `docs/architecture/interaction-design.md`. The most severe violations are:

1. **Entity type colors are hardcoded** in 7 of 13 views, ignoring the design system tokens defined in `docs/engineering/ui-design-system.md`.
2. **Free-text inputs are used for entity type and tag filtering** in 4 views, violating the "Prefer Selection Over Input" principle.
3. **Graph view requires typing a raw UUID** to begin exploration, making the view unusable for any user who does not have entity IDs memorized.
4. **Skeleton loaders are missing** from 4 views, despite the Timeline view demonstrating the correct pattern.
5. **Keyboard navigation is absent** from 10 of 13 views, despite the UI Philosophy requiring it.

These issues make the application feel like a prototype rather than a polished product. The fixes are mechanical — they require no new backend capabilities, no new entity types, and no new pipeline layers.

---

## Scope

### In Scope

- Replace hardcoded entity type colors with design system tokens across all views
- Replace free-text type/tag filters with dropdowns/selects populated from available data
- Replace Graph view UUID input with an autocomplete entity picker
- Add skeleton loaders to all views that lack them (using Timeline as reference)
- Add keyboard navigation to all views (arrow keys, Enter, Escape, Tab)
- Add error retry buttons to all views that lack them (using Timeline as reference)
- Fix Table view empty type filter dropdown
- Change Table view from double-click to single-click for opening entities
- Remove raw BM25 score display from Search results
- Add search mode toggle (keyword / semantic / hybrid) to Search view
- Add collapsible sidebar (icon-only mode)
- Add ARIA attributes to all views (using Settings as reference)
- Add entity count to all view headers
- Replace StatusBar emoji theme indicator with accessible icon
- Add `aria-current="page"` to Sidebar active item

### Out of Scope

- New views or features (deferred to future PRDs)
- Major backend or core changes (moderate to minor backend changes are allowed; significant backend or core work is deferred to PRD-0009)
- New entity types, relationship types, or component types
- Real-time collaboration features
- Mobile or responsive layouts beyond sidebar collapse
- Plugin marketplace or managed service

---

## Engineering Questions

### 1. Which canonical entities are introduced?

None. This PRD introduces no new entity types.

### 2. Which relationships are introduced?

None.

### 3. Which components are introduced?

None.

### 4. Which events are emitted?

None. All events are already defined in PRD-0001.

### 5. Which derived representations are generated?

None. All derived data (search index, view projections) is already defined.

### 6. Which layer owns the feature?

| Feature | Layer |
| ------- | ----- |
| All changes | Layer 7 — Presentation |

Every change in this PRD is a presentation-layer modification. No other pipeline layer is affected.

### 7. Can every derived artifact be regenerated?

Yes. No derived artifacts are changed.

### 8. Does the feature violate storage independence?

No. No storage changes are made.

### 9. Does the feature introduce implementation leakage?

No. No implementation details are exposed.

### 10. Does the feature preserve the canonical model?

Yes. The canonical model is unchanged. All modifications are in the Svelte presentation layer.

---

## Pipeline Spine Analysis

This PRD does not modify the pipeline spine. All changes are confined to Layer 7 (Presentation).

```
Pipeline (unchanged): Import → Extract → Resolve → Store → Connect → Derive → Present
                                                                           |
                                                                    PRD-0008 changes
                                                                    (CSS, ARIA, components,
                                                                     keyboard handlers, filters)
```

---

## Functional Requirements

### F1: Design System Color Compliance

| ID   | Requirement | Priority | Acceptance Criteria |
| ---- | ----------- | -------- | ------------------- |
| F1.1 | Entity type colors use design system tokens | P0 | All views use `--color-entity-*` tokens instead of hardcoded colors |
| F1.2 | Type badges are colored per entity type | P0 | Concept = `--color-entity-concept`, Person = `--color-entity-person`, etc. |
| F1.3 | Graph node colors use design system tokens | P0 | Graph view node fill colors match design system entity type colors |
| F1.4 | Timeline type dot colors use design system tokens | P0 | Replace hardcoded `typeColors` array |
| F1.5 | StatusBar provider status uses design system tokens | P0 | Replace hardcoded `#e53e3e` with `--color-error` |

### F2: Structured Filter Controls

| ID   | Requirement | Priority | Acceptance Criteria |
| ---- | ----------- | -------- | ------------------- |
| F2.1 | Browser type filter is a dropdown | P0 | `<select>` populated from available entity types replaces free-text input |
| F2.2 | Graph type filter is a dropdown | P0 | `<select>` replaces free-text input |
| F2.3 | Tree type filter is a dropdown | P0 | `<select>` replaces free-text input |
| F2.4 | Search type filter is a dropdown | P0 | `<select>` populated from available entity types |
| F2.5 | Search tag filter is a searchable dropdown | P0 | Dropdown populated from available tags with search capability |
| F2.6 | Table type filter is populated | P0 | Dropdown contains actual entity types (currently empty) |

### F3: Graph View Entity Picker

| ID   | Requirement | Priority | Acceptance Criteria |
| ---- | ----------- | -------- | ------------------- |
| F3.1 | Graph view uses autocomplete entity picker | P0 | Replace UUID text input with searchable entity picker |
| F3.2 | Entity picker shows type badge and title | P0 | Each result shows entity type color, type name, and title |
| F3.3 | Entity picker supports keyboard navigation | P0 | Arrow keys navigate results, Enter selects, Escape closes |
| F3.4 | Graph defaults to showing all entities if no entity selected | P1 | When no `startId` is provided, show all entities or prompt picker |
| F3.5 | Breadcrumb shows entity title instead of UUID prefix | P1 | Replace `startId.slice(0, 8)` with entity title |

### F4: Skeleton Loaders

| ID   | Requirement | Priority | Acceptance Criteria |
| ---- | ----------- | -------- | ------------------- |
| F4.1 | Dashboard skeleton | P0 | Bento grid skeleton matches layout shape |
| F4.2 | Browser skeleton | P0 | Table skeleton with rows |
| F4.3 | Detail skeleton | P0 | Entity header + component card skeletons |
| F4.4 | Table skeleton | P0 | Table skeleton with rows |
| F4.5 | All skeletons use pulse animation | P1 | Consistent with Timeline skeleton pattern |

### F5: Keyboard Navigation

| ID   | Requirement | Priority | Acceptance Criteria |
| ---- | ----------- | -------- | ------------------- |
| F5.1 | Browser: arrow keys navigate rows, Enter opens entity | P0 | Full keyboard navigation in entity table |
| F5.2 | Table: arrow keys navigate rows, Enter opens entity | P0 | Replace double-click with single-click + keyboard |
| F5.3 | Tree: arrow keys navigate nodes, Right/Left expand/collapse | P0 | Full keyboard tree navigation |
| F5.4 | Search: arrow keys navigate results, Enter opens entity | P0 | Full keyboard result navigation |
| F5.5 | Graph: Tab between nodes, Enter to select | P1 | Keyboard graph navigation |
| F5.6 | Dashboard: Tab between cards and quick actions | P1 | Keyboard dashboard navigation |
| F5.7 | Import: keyboard tab switching (arrow keys or Ctrl+1-4) | P1 | Keyboard tab switching |
| F5.8 | Sidebar: keyboard shortcut hints visible on hover | P1 | Tooltips show shortcuts |

### F6: Error Recovery

| ID   | Requirement | Priority | Acceptance Criteria |
| ---- | ----------- | -------- | ------------------- |
| F6.1 | Dashboard: retry button on error | P1 | Error state includes a "Retry" button |
| F6.2 | Browser: retry button on error | P1 | Error state includes a "Retry" button |
| F6.3 | Tree: retry button on error | P1 | Error state includes a "Retry" button |
| F6.4 | Table: retry button on error | P1 | Error state includes a "Retry" button |
| F6.5 | All error messages are actionable | P1 | Error messages explain what happened and how to recover |

### F7: Search Improvements

| ID   | Requirement | Priority | Acceptance Criteria |
| ---- | ----------- | -------- | ------------------- |
| F7.1 | Remove raw BM25 score display | P1 | Score is not shown to users |
| F7.2 | Add search mode toggle | P1 | Toggle between Keyword / Semantic / Hybrid modes |
| F7.3 | Add search result count | P1 | "Found X results" displayed above results |
| F7.4 | Add `role="search"` and `aria-live` | P1 | Accessibility compliance |

### F8: Sidebar Collapse

| ID   | Requirement | Priority | Acceptance Criteria |
| ---- | ----------- | -------- | ------------------- |
| F8.1 | Sidebar collapses to icon-only mode | P1 | Toggle button collapses sidebar to ~48px width |
| F8.2 | Collapsed sidebar shows icons with tooltips | P1 | Hover shows view name tooltip |
| F8.3 | Collapse state is persisted | P1 | State persists across view changes |

### F9: Accessibility Compliance

| ID   | Requirement | Priority | Acceptance Criteria |
| ---- | ----------- | -------- | ------------------- |
| F9.1 | All views use ARIA landmarks | P0 | `role="navigation"`, `role="main"`, `role="region"` |
| F9.2 | Sidebar active item has `aria-current="page"` | P0 | Screen readers announce current page |
| F9.3 | StatusBar theme indicator uses icon + aria-label | P0 | Replace emoji with material icon |
| F9.4 | StatusBar has `aria-live` for status messages | P0 | Screen readers announce status changes |
| F9.5 | All filter inputs have `<label>` elements | P1 | Associated labels for all inputs |
| F9.6 | Entity count displayed in all view headers | P1 | "Browse (42)", "Table (42)", etc. |

---

## Non-Functional Requirements

### NF1: Performance

| ID    | Requirement | Target | Acceptable |
| ----- | ----------- | ------ | ---------- |
| NF1.1 | No view regression in render time | < 100ms | < 500ms |
| NF1.2 | Skeleton loaders render instantly | < 16ms | < 50ms |
| NF1.3 | Entity picker autocomplete responds | < 200ms | < 500ms |

### NF2: Accessibility

| ID    | Requirement | Target |
| ----- | ----------- | ------ |
| NF2.1 | WCAG 2.1 AA compliance | All views pass axe-core audit |
| NF2.2 | Keyboard navigation | All interactive elements reachable via keyboard |
| NF2.3 | Screen reader support | All views announce correctly with NVDA/VoiceOver |

---

## User Stories

### US1: Filter Entities by Type Without Typing

**As a** knowledge worker,
**I want to** select an entity type from a dropdown instead of typing it,
**So that** I don't need to remember exact type names.

**Acceptance criteria:**
1. Browser, Tree, Graph, Table, and Search views show a `<select>` dropdown for type filtering.
2. Dropdown is populated from entity types present in the database.
3. Selecting a type immediately filters the view.
4. "All types" is the default option.

### US2: Explore the Graph Without Knowing Entity IDs

**As a** knowledge worker,
**I want to** search for an entity by name to start graph exploration,
**So that** I don't need to know or copy entity UUIDs.

**Acceptance criteria:**
1. Graph view shows an autocomplete entity picker instead of a UUID text input.
2. Typing in the picker searches entities by title.
3. Results show entity type badge (colored) and title.
4. Selecting an entity starts graph exploration from that entity.
5. Keyboard navigation works (arrow keys, Enter, Escape).

### US3: Navigate Any View with the Keyboard

**As a** keyboard-focused user,
**I want to** navigate entities and interact with views using only the keyboard,
**So that** I don't need to switch between keyboard and mouse.

**Acceptance criteria:**
1. Arrow keys navigate between items in Browser, Table, Tree, and Search views.
2. Enter opens the selected entity.
3. Escape closes panels or returns to previous view.
4. Tab navigates between interactive elements.
5. Focus indicators are visible on all focused elements.

### US4: See Loading Skeletons Instead of Blank Screens

**As a** knowledge worker,
**I want to** see skeleton loaders while data is loading,
**So that** I know the view is loading and can anticipate the layout.

**Acceptance criteria:**
1. Dashboard, Browser, Detail, and Table views show skeleton loaders during data fetch.
2. Skeletons match the layout shape of the loaded content.
3. Skeletons use the pulse animation pattern from Timeline view.
4. Skeletons are replaced by content when data arrives.

### US5: Understand Error States and Recover

**As a** knowledge worker,
**I want to** see a clear error message with a retry button when something fails,
**So that** I can recover from errors without navigating away.

**Acceptance criteria:**
1. Dashboard, Browser, Tree, and Table views show a "Retry" button on error.
2. Error messages explain what happened.
3. Clicking "Retry" re-attempts the failed operation.
4. Loading state is shown during retry.

---

## Architecture

### Crate Changes

No Rust crate changes. All modifications are in the Svelte desktop application.

### Frontend Changes

| File | Change |
| ---- | ------ |
| `desktop/src/lib/theme.svelte.ts` | Add `getEntityTypeColor(type: string): string` function that maps entity types to design system CSS variables |
| `desktop/src/components/EntityPicker.svelte` | **New.** Autocomplete entity picker component (search by title, show type badge, keyboard nav) |
| `desktop/src/components/SkeletonLoader.svelte` | **New.** Configurable skeleton loader component (grid, table, list, card layouts) |
| `desktop/src/components/TypeFilterDropdown.svelte` | **New.** Reusable type filter dropdown populated from available entity types |
| `desktop/src/components/TypeBadge.svelte` | **New.** Entity type badge using design system colors |
| `desktop/src/views/Dashboard.svelte` | Add skeleton loader, use TypeBadge, add retry button, add keyboard nav |
| `desktop/src/views/Browser.svelte` | Replace text filter with TypeFilterDropdown, add keyboard nav, add skeleton, add entity count |
| `desktop/src/views/Detail.svelte` | Show entity title as heading, render Content as Markdown, use TypeBadge, add archive button |
| `desktop/src/views/Graph.svelte` | Replace UUID input with EntityPicker, replace type filter with TypeFilterDropdown, use design system node colors, show title in breadcrumb |
| `desktop/src/views/Tree.svelte` | Replace text filter with TypeFilterDropdown, add expand/collapse all, add keyboard nav, use TypeBadge |
| `desktop/src/views/Table.svelte` | Populate type filter, change to single-click, add skeleton, add keyboard nav, add aria-sort |
| `desktop/src/views/Timeline.svelte` | Replace hardcoded typeColors with design system tokens |
| `desktop/src/views/Import.svelte` | Add ARIA tab roles, add keyboard tab switching |
| `desktop/src/views/Search.svelte` | Replace text filters with TypeFilterDropdown, remove raw score, add search mode toggle, add keyboard nav, add aria-live |
| `desktop/src/views/Chat.svelte` | Use design system entity type colors for pills/citations, add aria-live on messages |
| `desktop/src/views/Settings.svelte` | Add section navigation anchors |
| `desktop/src/views/Sidebar.svelte` | Add collapse toggle, add badge counts, add aria-current, add tooltips |
| `desktop/src/views/StatusBar.svelte` | Replace hardcoded color with token, replace emoji with icon+aria-label, add aria-live |
| `desktop/src/app.css` | Add skeleton pulse animation, add focus-visible styles if missing |

### Shared Component: EntityPicker

```svelte
<script lang="ts">
  import { searchEntities } from "../lib/api.js";
  import type { EntitySummary } from "../lib/types.js";

  let { onSelect, placeholder = "Search entities..." }: {
    onSelect: (entity: EntitySummary) => void;
    placeholder?: string;
  } = $props();

  let query = $state("");
  let results = $state<EntitySummary[]>([]);
  let open = $state(false);
  let selectedIndex = $state(0);
  let debounceTimer: ReturnType<typeof setTimeout> | null = null;
</script>
```

### Shared Component: TypeFilterDropdown

```svelte
<script lang="ts">
  let { value, options, onchange }: {
    value: string;
    options: string[];
    onchange: (value: string) => void;
  } = $props();
</script>

<select bind:value onchange={() => onchange(value)}>
  <option value="">All types</option>
  {#each options as opt}
    <option value={opt}>{opt}</option>
  {/each}
</select>
```

### Shared Component: TypeBadge

```svelte
<script lang="ts">
  import { getEntityTypeColor } from "../lib/theme.svelte.js";

  let { type }: { type: string } = $props();
</script>

<span class="type-badge" style="background: {getEntityTypeColor(type)}">{type}</span>
```

### Entity Type Color Mapping

```typescript
// desktop/src/lib/theme.svelte.ts — add this function
const entityTypeColorTokens: Record<string, string> = {
  Concept:      "var(--color-entity-concept, #8B5CF6)",
  Person:       "var(--color-entity-person, #EC4899)",
  Organization: "var(--color-entity-organization, #F59E0B)",
  Project:      "var(--color-entity-project, #6366F1)",
  Paper:        "var(--color-entity-paper, #3B82F6)",
  Book:         "var(--color-entity-book, #06B6D4)",
  Article:      "var(--color-entity-default, #64748B)",
  Video:        "var(--color-entity-default, #64748B)",
  Tool:         "var(--color-entity-tool, #10B981)",
  Technology:   "var(--color-entity-technology, #6366F1)",
  Decision:     "var(--color-entity-decision, #EF4444)",
  Event:        "var(--color-entity-event, #F97316)",
  Collection:   "var(--color-entity-collection, #14B8A6)",
};

export function getEntityTypeColor(type: string): string {
  return entityTypeColorTokens[type] ?? "var(--color-entity-default, #64748B)";
}
```

---

## Acceptance Criteria

### Definition of Done

- [ ] All views use design system entity type color tokens (no hardcoded colors)
- [ ] All type/tag filters use dropdowns/selects (no free-text for types or tags)
- [ ] Graph view uses autocomplete entity picker instead of UUID input
- [ ] All views show skeleton loaders during data fetch
- [ ] All views support keyboard navigation (arrow keys, Enter, Escape)
- [ ] All views have error retry buttons
- [ ] Table view uses single-click to open entities
- [ ] Table view type filter is populated
- [ ] Search view does not show raw BM25 scores
- [ ] Search view has mode toggle (keyword / semantic / hybrid)
- [ ] Sidebar collapses to icon-only mode
- [ ] All views pass axe-core accessibility audit
- [ ] StatusBar uses design system tokens and accessible icons
- [ ] Entity count displayed in all view headers
- [ ] All shared components (EntityPicker, TypeFilterDropdown, TypeBadge, SkeletonLoader) are implemented and reused
- [ ] All tests pass
- [ ] No regression in view render performance

### Test Cases

1. **Type badge colors** — Each entity type displays the correct design system color across all views
2. **Type filter dropdown** — Selecting a type filters correctly in Browser, Tree, Graph, Table, Search
3. **Entity picker** — Typing a title fragment finds entities; Enter selects; Escape closes
4. **Skeleton loader** — Each view shows skeleton during loading, replaced by content on load
5. **Keyboard navigation** — Arrow keys move between items in Browser, Table, Tree, Search
6. **Keyboard entity open** — Enter key opens entity detail from any list view
7. **Error retry** — Clicking retry on any view re-fetches data
8. **Table single-click** — Single click opens entity (no double-click required)
9. **Search mode toggle** — Switching modes changes search behavior
10. **Sidebar collapse** — Collapse/expand toggle works; state persists
11. **Accessibility audit** — axe-core reports zero critical violations on all views
12. **StatusBar accessibility** — Theme icon has aria-label; status messages use aria-live
13. **Entity count** — Each view header shows entity count

---

## Testing Strategy

| Level | Scope | Framework |
| ----- | ----- | --------- |
| Unit | Shared components (EntityPicker, TypeFilterDropdown, TypeBadge, SkeletonLoader) | Vitest + Svelte Testing Library |
| Integration | View + component integration, keyboard event handling | Vitest + Svelte Testing Library |
| Accessibility | Automated axe-core audit on all views | axe-core + Vitest |
| E2E | Full user flows: filter, search, graph explore, keyboard navigation | Playwright |

---

## Risks and Mitigations

| Risk | Impact | Likelihood | Mitigation |
| ---- | ------ | ---------- | ---------- |
| Shared component refactoring breaks existing views | High | Medium | Write components first with tests, then integrate one view at a time |
| Keyboard navigation conflicts with existing shortcuts | Medium | Medium | Audit existing shortcuts in `shortcuts.svelte.ts` before implementing |
| Entity picker performance with 100K entities | Medium | Low | Use debounced search (300ms) with server-side filtering |
| Skeleton loader flash on fast connections | Low | Low | Only show skeleton if load takes > 100ms |
| Sidebar collapse breaks layout on small screens | Medium | Low | Test at minimum viewport width (800px) |

---

## Dependencies

### External Crates

No new Rust crate dependencies. Frontend dependencies (if needed):

| Package | Purpose | Justification |
| ------- | ------- | ------------- |
| `axe-core` | Accessibility testing | Automated WCAG audit |

### Internal Dependencies

- `docs/architecture/interaction-design.md` — The 11 interaction principles
- `docs/engineering/ui-design-system.md` — Design tokens and component specs
- `docs/architecture/ui-philosophy.md` — View properties and navigation rules
- PRD-0001 — Core entity model (entity types, component types)
- PRD-0003 — View projections and plugin system
- PRD-0004 — Implementation gaps (some overlap on view synchronization)
- `docs/engineering/ux-audits/audit-*.md` — All 13 view audit reports

---

## Timeline

| Phase | Duration | Deliverables |
| ----- | -------- | ------------ |
| Phase 1: Shared components | 3 days | EntityPicker, TypeFilterDropdown, TypeBadge, SkeletonLoader, getEntityTypeColor() |
| Phase 2: Color compliance | 2 days | Replace all hardcoded colors in all 13 views |
| Phase 3: Filter controls | 2 days | Replace all free-text filters with dropdowns |
| Phase 4: Graph entity picker | 2 days | Replace UUID input with autocomplete picker |
| Phase 5: Skeleton loaders | 2 days | Add skeletons to Dashboard, Browser, Detail, Table |
| Phase 6: Keyboard navigation | 3 days | Add keyboard nav to all 13 views |
| Phase 7: Error recovery | 1 day | Add retry buttons to all views |
| Phase 8: Search improvements | 1 day | Remove score, add mode toggle, add result count |
| Phase 9: Sidebar collapse | 1 day | Collapse toggle, tooltips, badge counts |
| Phase 10: Accessibility | 2 days | ARIA attributes, labels, aria-live, aria-current |
| Phase 11: Testing + audit | 2 days | axe-core audit, integration tests, E2E tests |

**Total: ~3 weeks**
