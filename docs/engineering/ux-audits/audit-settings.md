# UX Audit: Settings

**Date:** 2026-07-30
**Auditor:** UX Audit Agent
**View file:** `desktop/src/views/Settings.svelte`
**Overall score:** 7/11 passed, 4 partial, 0 fail

---

## Summary

Settings is a well-designed configuration view with progressive disclosure (collapsible "Advanced" sections), quick-start presets, validation feedback, dirty-state tracking, confirmation modals for destructive actions, and a help section with links. It uses ARIA attributes extensively (`aria-label`, `aria-live`, `aria-expanded`, `aria-describedby`, `aria-required`). Its main issues are: the settings are provider-focused with no UI customization options, the ignore patterns editor is a raw textarea with no syntax guidance, and section navigation requires scrolling.

---

## Principle Audit

### 1. Goal-Oriented Design — Pass

The goal (configure AI providers) is clear. Quick-start presets (Ollama, LM Studio) reduce setup friction. Status summary shows current state. Help section provides next steps.

### 2. Reduce Cognitive Load — Pass

Advanced sections are collapsed by default. Provider-specific fields appear only when relevant (e.g., API key only for OpenAI-compatible). Model hints show examples. Field hints explain each setting.

### 3. Predictability — Pass

Save/Test/Reset buttons behave consistently across sections. Validation prevents saving invalid configurations. Test results show clear success/failure.

### 4. Immediate Feedback — Pass

"Saved" badge appears after save (line 356). "Unsaved" badge shows dirty state (line 360). Status alerts with icons (line 319). Test result shows latency. Save buttons show "Saving..." during operation.

### 5. Consistency — Pass

All three sections (Chat, OCR, Ignore) follow the same pattern: label, input, hint, actions. Buttons are consistent. Modals are consistent.

### 6. Intelligent Defaults — Pass

Provider defaults are pre-filled. Model hints show common values. URL defaults shown as placeholders. Mock provider works offline.

### 7. Prefer Selection Over Input — Pass

Provider and backend selection use `<select>` dropdowns. Preset buttons for common configurations.

### 8. Error Tolerance — Pass

Validation prevents invalid saves. Reset confirmation modal (line 497). Test connection before committing. "Dirty" state warns about unsaved changes.

### 9. Reversible Actions — Pass

Reset to default with confirmation. Can change any setting without permanent consequences.

### 10. Performance Perception — Pass

Save/Test buttons show loading state. No long-running operations. Instant feedback.

### 11. User Confidence — Pass

Clear status indicators. Test connection validates setup. Help section provides guidance. Info cards explain mock mode.

---

## Design System Compliance

### Token Usage
Excellent token usage. Uses `--spacing-*`, `--font-size-*`, `--bg-*`, `--border`, `--radius-*`, `--accent`, `--text-*`, `--font-mono` tokens throughout.

### Entity Type Colors
Not applicable — Settings doesn't display entities.

### Accessibility
**Best-in-class accessibility:**
- `role="region"` and `aria-label` on main container (line 312)
- `aria-live="polite"` on status alerts (line 320)
- `aria-describedby` on inputs linking to hint text
- `aria-required="true"` on required fields
- `aria-expanded` on advanced toggles
- `aria-modal="true"` on confirmation modal
- `role="alert"` and `aria-live="polite"` on test results
- `aria-label` on icon buttons
- Field IDs match label `for` attributes

---

## Interaction Heuristics

| Heuristic | Status | Notes |
| --------- | ------ | ----- |
| One primary action | Partial | Multiple sections, each with Save |
| Minimized clicks | Pass | Presets reduce clicks |
| Reduced context switching | Pass | All settings on one page |
| No unnecessary complexity | Pass | Advanced sections hidden by default |
| System state visible | Pass | Saved/Unsaved/Testing states |

---

## Critical Issues (P0)

None.

## Major Issues (P1)

1. **No section navigation** — Must scroll through all sections. Needs a sticky section nav or anchor links.
2. **Ignore patterns editor is raw textarea** — No syntax highlighting, no examples, no validation feedback for patterns.
3. **No UI customization** — No theme selection, no layout options, no font size adjustment.
4. **No keyboard shortcuts for sections** — Cannot jump to a section via keyboard.

## Minor Issues (P2)

5. **No import/export of settings** — Cannot backup or transfer configuration.
6. **Provider status summary could be more detailed** — No latency history, no error count.
7. **Help section links may be broken** — Links to `/docs/guides/` paths that may not exist in desktop context.
8. **No search within settings** — Cannot find a specific setting quickly.

---

## Recommendations

1. Add section navigation (sticky sidebar or anchor links)
2. Add syntax hints or example patterns for the ignore patterns textarea
3. Add theme selection (light/dark/system)
4. Add keyboard shortcuts to jump to sections
5. Add import/export of settings as JSON
6. This view is the reference implementation for accessibility patterns — apply its ARIA usage to all other views
7. This view is the reference for progressive disclosure — apply its "Advanced" toggle pattern to Detail and other views
