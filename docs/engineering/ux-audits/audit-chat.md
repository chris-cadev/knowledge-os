# UX Audit: Chat

**Date:** 2026-07-30
**Auditor:** UX Audit Agent
**View file:** `desktop/src/views/Chat.svelte`
**Overall score:** 7/11 passed, 4 partial, 0 fail

---

## Summary

Chat is the most sophisticated view in the app and the closest to interaction design excellence. It has streaming responses with real-time delta rendering, entity reference via @mentions with autocomplete, command palette via /, citation tooltips and collapsible source lists, message feedback (thumbs up/down with reason form), conversation management (rename, delete with confirmation modal), mode toggle (Fast/Thinking), and knowledge graph/web search toggles. Its remaining issues are minor: hardcoded colors in some elements, no keyboard shortcuts documentation, and the conversation sidebar could benefit from search.

---

## Principle Audit

### 1. Goal-Oriented Design — Pass

The goal (ask questions about the knowledge graph) is immediately clear. Welcome screen provides tips. Input area is prominent. Entity reference and command palette are discoverable through placeholder text.

### 2. Reduce Cognitive Load — Partial

The view has many features (conversations, modes, toggles, citations, feedback) but the input area keeps the focus on asking questions. The conversation sidebar is separate. Mode toggle and source toggles are above the input. However, the input area has many controls (mode toggle, knowledge graph toggle, web search toggle, entity icon hint) which adds visual complexity.

### 3. Predictability — Pass

Streaming responses appear in real-time. Citations are clickable. Conversation switching is immediate. Standard chat patterns.

### 4. Immediate Feedback — Pass

Streaming delta rendering (line 325). Processing status text (line 729). Typing indicator. Error banner with retry. Loading spinner for conversations. Excellent feedback throughout.

### 5. Consistency — Partial

Button and input patterns are consistent with other views. But some colors are hardcoded (e.g., `rgba(255, 255, 255, 0.15)` for user message code blocks, line 1667). Type badges in citations don't use design system colors.

### 6. Intelligent Defaults — Partial

Mode defaults to "thinking" (correct for knowledge queries). Knowledge graph defaults to on. Web search defaults to off. But conversation list is not searchable.

### 7. Prefer Selection Over Input — Pass

Entity reference uses autocomplete dropdown (line 220). Command palette uses autocomplete. Mode toggle is buttons. Source toggles are checkboxes.

### 8. Error Tolerance — Pass

Error banner with retry button (line 1188). Streaming errors remove the empty assistant message (line 358). Fallback to non-streaming send (line 369). Delete confirmation modal (line 998).

### 9. Reversible Actions — Pass

Conversations can be renamed. Delete requires confirmation. Streaming can be stopped. Messages can be retried.

### 10. Performance Perception — Pass

Streaming provides real-time feedback. Processing status shows what the system is doing. Typing indicator during generation. Excellent.

### 11. User Confidence — Pass

Welcome screen sets expectations. Feedback mechanism gives users a voice. Citations provide source transparency. Error states are recoverable.

---

## Design System Compliance

### Token Usage
Uses most tokens (`--spacing-*`, `--font-size-*`, `--bg-*`, `--border`, `--radius-*`, `--accent`, `--font-mono`).

### Entity Type Colors
**Partially non-compliant.** Entity pills use `var(--accent)` (line 1724). Citation type badges don't use per-type colors.

### Accessibility
- Conversation items have `role="button"`, `tabindex="0"`, Enter handler — good
- Context menu has backdrop — good
- Modal has `role="dialog"`, `aria-modal="true"` — good
- Input textarea has placeholder but no `<label>`
- No `aria-live` on message container for streaming updates
- Keyboard handler (line 770) handles arrow keys, Escape, Enter, Tab — good
- Feedback buttons have `title` attributes — good
- Sources toggle doesn't have `aria-expanded`

---

## Interaction Heuristics

| Heuristic | Status | Notes |
| --------- | ------ | ----- |
| One primary action | Pass | Type and send |
| Minimized clicks | Pass | @ for entity ref, / for commands |
| Reduced context switching | Pass | Chat + sources in one view |
| No unnecessary complexity | Partial | Many toggles in input area |
| System state visible | Pass | Streaming, status, errors all shown |

---

## Critical Issues (P0)

None.

## Major Issues (P1)

1. **Entity pills not using design system colors** — Use per-type colors
2. **No keyboard shortcuts help** — No `?` to show available shortcuts
3. **Conversation sidebar has no search** — Hard to find old conversations
4. **No `aria-live` on message container** — Screen readers don't announce streaming
5. **Sources toggle missing `aria-expanded`** — Accessibility gap

## Minor Issues (P2)

6. **Input area visual complexity** — Too many toggles above the textarea
7. **No message copy button** — Cannot easily copy assistant responses
8. **No conversation export** — Cannot export chat history
9. **No conversation search** — Cannot search within a conversation
10. **Hardcoded rgba colors** — Some colors are hardcoded instead of using tokens

---

## Recommendations

1. Use design system entity type colors for entity pills and citation badges
2. Add `?` keyboard shortcut to show available shortcuts
3. Add search to conversation sidebar
4. Add `aria-live="polite"` to message container
5. Add `aria-expanded` to sources toggle
6. Simplify input area — move toggles to a settings row or collapsible panel
7. Add copy button on assistant messages
8. Add conversation export
9. Add in-conversation search (Ctrl+F)
10. Replace hardcoded rgba colors with design system tokens
11. This view is the reference implementation for streaming, feedback, and error recovery patterns
