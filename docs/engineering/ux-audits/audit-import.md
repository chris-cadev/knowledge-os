# UX Audit: Import

**Date:** 2026-07-30
**Auditor:** UX Audit Agent
**View file:** `desktop/src/views/Import.svelte`
**Overall score:** 5/11 passed, 5 partial, 1 fail

---

## Summary

The Import view is the most feature-rich workflow in the app — it supports files, URLs, clipboard, and database imports with drag-and-drop, directory preview, undo, and per-item progress. Its tabbed interface provides good progressive disclosure. Its main issues are: no keyboard shortcuts for tab switching, no connection testing for database imports, and the import progress is only shown after completion (no real-time progress during import).

---

## Principle Audit

### 1. Goal-Oriented Design — Pass

The primary goal (importing content) is clearly supported. Tab-based organization separates import sources. Drag-and-drop is the most direct path. File picker and directory picker provide alternatives.

### 2. Reduce Cognitive Load — Partial

Four tabs is the upper limit of cognitive load. Each tab has a clear purpose. But the files tab has many options (recursive checkbox, preview button, undo button, drop zone, file picker, directory picker) which creates decision fatigue.

### 3. Predictability — Pass

Tab switching is instant. Import results are consistent. Undo removes last import. Standard patterns.

### 4. Immediate Feedback — Partial

Drop zone shows importing state with spinner (line 377). But there is no per-file progress during import — all results appear after completion. For large directories, the user has no visibility into progress.

### 5. Consistency — Pass

Uses design system tokens. Buttons and inputs follow patterns from other views. Tab styling is consistent.

### 6. Intelligent Defaults — Partial

Drop zone is the default interaction (good). But "recursive" defaults to false, which may not match user expectations for directory imports.

### 7. Prefer Selection Over Input — Pass

File picker uses native OS dialog. No free-text input for file paths. URL tab has a URL input (acceptable for URLs).

### 8. Error Tolerance — Pass

Per-item error display with expandable details (lines 556–566). Error toggle button. Undo button for recovery. Status messages for failures.

### 9. Reversible Actions — Pass

Undo button (line 363, 573) reverses the last import. This is critical for an import workflow.

### 10. Performance Perception — Partial

Importing state shows spinner (line 377). But no per-file progress, no progress bar, no count of files processed. For large imports, the user has no visibility.

### 11. User Confidence — Pass

Undo provides safety. Error details are expandable. Success stats (created, merged, errors) are clear. Format breakdown in directory preview helps users understand what will be imported.

---

## Design System Compliance

### Token Usage
Uses `--spacing-*`, `--font-size-*`, `--bg-*`, `--border`, `--radius-*`, `--accent` tokens.

### Entity Type Colors
Not applicable — Import doesn't display entities.

### Accessibility
- Tabs are `<button>` elements — good
- Drop zone has visual feedback — good
- But no `role="tablist"`, `role="tab"`, `aria-selected` on tabs
- No keyboard shortcut for tab switching (Ctrl+Tab or arrow keys)
- File picker button has icon + text — good
- Progress items have status icons — good

---

## Interaction Heuristics

| Heuristic | Status | Notes |
| --------- | ------ | ----- |
| One primary action | Pass | Import (drop or pick) |
| Minimized clicks | Partial | Tab + pick + confirm is 3 steps |
| Reduced context switching | Pass | All import sources in one view |
| No unnecessary complexity | Partial | Files tab has many options |
| System state visible | Partial | No real-time progress |

---

## Critical Issues (P0)

None. This is the strongest view.

## Major Issues (P1)

1. **No real-time import progress** — No per-file progress or progress bar during import
2. **No keyboard shortcuts for tabs** — Cannot switch tabs with keyboard
3. **No database connection test** — Database tab has no "Test Connection" button
4. **Tabs missing ARIA roles** — No `role="tablist"`, `role="tab"`, `aria-selected`
5. **Recursive defaults to false** — Should remember user preference

## Minor Issues (P2)

6. **No drag-and-drop for URL tab** — Could support dropping URLs
7. **Clipboard format is auto-detected but not changeable** — User cannot override format
8. **No import history** — Cannot see past imports
9. **No batch import from multiple directories** — Only one directory at a time

---

## Recommendations

Status of the audit's recommendations as of the Import UX pass (`feat/ux-import`, PRD-0008):

1. ~~Add real-time per-file progress during import (streaming progress events)~~ — Deferred to [PRD-0009](../prds/prd-0009-import-workflow.md) (F1)
2. ~~Add keyboard shortcuts for tab switching (arrow keys or Ctrl+1/2/3/4)~~ — Resolved and reversed: removed. App-level `Ctrl+1-8` view navigation in `shortcuts.svelte.ts` already covers section switching; per-tab shortcuts conflicted with it.
3. ~~Add "Test Connection" button for database tab~~ — Done (Import view, database tab)
4. ~~Add ARIA tab roles (`role="tablist"`, `role="tab"`, `aria-selected`)~~ — Done (Import view)
5. ~~Remember recursive preference in session storage~~ — Done (localStorage, Import view)
6. Add import history view — Planned in [PRD-0009](../prds/prd-0009-import-workflow.md) (F2)
7. This view is the reference for undo/redo patterns — apply to other views
8. Add batch import from multiple directories — Planned in [PRD-0009](../prds/prd-0009-import-workflow.md) (F3)
9. Add drag-and-drop for URL tab — Planned in [PRD-0009](../prds/prd-0009-import-workflow.md) (F4)
10. Make clipboard format overrideable — Done (Text/HTML toggle, Import view)
