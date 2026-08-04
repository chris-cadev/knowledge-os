# UX Audit: Chat

**Date:** 2026-08-03 (re-audit after `c9939a4` + compliance pass)
**Auditor:** UX Audit Agent
**View file:** `desktop/src/views/Chat.svelte`
**Related files:** `desktop/src/lib/api.ts`, `desktop/src/lib/chat-stream.ts`, `desktop/src/lib/command-palette.ts`, `desktop/src/lib/shortcuts.svelte.ts`, `desktop/src/lib/theme.svelte.ts`, `desktop/src/lib/types.ts`, `desktop/src/app.css`, `desktop/src-tauri/src/commands/chat.rs`, `core/knowledge-core/src/ports/chat.rs`, `core/knowledge-core/src/ports/conversation.rs`, `core/knowledge-storage/src/adapters/sqlite/conversation.rs`
**Overall score:** 11/11 passed, 0 partial, 0 fail

---

## Summary

Chat is the most sophisticated view in the app and the closest to interaction design excellence. It has streaming responses with real-time delta rendering, entity reference via @mentions with autocomplete, command palette via /, citation tooltips and collapsible source lists, message feedback (thumbs up/down with reason form), conversation management (rename, delete with confirmation modal), mode toggle (Fast/Thinking), and knowledge graph/web search toggles. All issues raised in the 2026-07-30 audit are now resolved, including entity refs reaching the backend, keyboard shortcuts help, conversation sidebar search, modal accessibility, working `/help` `/clear` `/export` commands, a collapsible input settings panel, message copy, in-conversation find, design-token colors, and citations/feedback surviving reload.

---

## Principle Audit

### 1. Goal-Oriented Design — Pass

The goal (ask questions about the knowledge graph) is immediately clear. Welcome screen provides tips. Input area is prominent. Entity reference and command palette are discoverable through placeholder text.

### 2. Reduce Cognitive Load — Pass

The view has many features (conversations, modes, toggles, citations, feedback) but the input area keeps the focus on asking questions. Mode and source toggles now live in a collapsible settings panel (`tune` button), so the default input area is a single clean row with just the textarea and send button.

### 3. Predictability — Pass

Streaming responses appear in real-time. Citations are clickable. Conversation switching is immediate. Standard chat patterns.

### 4. Immediate Feedback — Pass

Streaming delta rendering. Processing status text. Typing indicator. Error banner with retry. Loading spinner for conversations. Copy buttons confirm with a check icon. Excellent feedback throughout.

### 5. Consistency — Pass

Entity pills, citation type badges, citation tooltip, and entity dropdown badges all use per-type design system colors (`getEntityTypeColor()`). All on-accent rgba colors are replaced with design system tokens (`--text-on-accent-*`, `--surface-on-accent-subtle`, `--hover-on-accent`, `--border-on-accent`, `--overlay-*`, `--danger-soft`, `--danger-hover`). No hardcoded colors remain.

### 6. Intelligent Defaults — Pass

Mode defaults to "thinking" (correct for knowledge queries). Knowledge graph defaults to on. Web search defaults to off. Conversation sidebar is now searchable.

### 7. Prefer Selection Over Input — Pass

Entity reference uses autocomplete dropdown. Command palette uses autocomplete. Mode toggle is buttons. Source toggles are checkboxes. Conversation search is free-text but filters the selection.

### 8. Error Tolerance — Pass

Error banner with retry button. Streaming errors remove the empty assistant message. Fallback to non-streaming send. Delete and clear both require confirmation modals.

### 9. Reversible Actions — Pass

Conversations can be renamed. Delete requires confirmation. Streaming can be stopped. Messages can be retried. Copy gives a visual confirmation. `/clear` requires confirmation.

### 10. Performance Perception — Pass

Streaming provides real-time feedback. Processing status shows what the system is doing. Typing indicator during generation. Excellent.

### 11. User Confidence — Pass

Welcome screen sets expectations. Feedback mechanism gives users a voice. Citations provide source transparency. Error states are recoverable. Keyboard shortcuts are discoverable via `?`.

---

## Design System Compliance

### Token Usage
Uses most tokens (`--spacing-*`, `--font-size-*`, `--bg-*`, `--border`, `--radius-*`, `--accent`, `--font-mono`, `--color-surface-*`) plus the new on-accent/overlay tokens defined in `app.css`.

### Entity Type Colors
**Compliant.** Entity pills, citation type badges, citation tooltip, and entity dropdown badges all use `getEntityTypeColor()` mapped to `--color-entity-*` tokens.

### Accessibility
- Conversation items have `role="button"`, `tabindex="0"`, Enter handler — good
- Context menu has backdrop — good
- Delete and clear confirmation modals have `role="dialog"`, `aria-modal="true"`, and `aria-labelledby` — good
- Message textarea has `aria-label` — good
- Feedback comment textarea has `aria-label` — good
- `aria-live="polite"` on message container for streaming updates — good
- Keyboard handler handles arrow keys, Escape, Enter, Tab — good
- Feedback and copy buttons have `title` attributes — good
- Sources toggle has `aria-expanded` — good
- Input settings toggle has `aria-expanded` and `aria-label` — good
- Find bar input has `aria-label` — good
- Global `?` shortcut is non-interfering with typing (ignored when focus is in an input)

---

## Interaction Heuristics

| Heuristic | Status | Notes |
| --------- | ------ | ----- |
| One primary action | Pass | Type and send |
| Minimized clicks | Pass | @ for entity ref, / for commands |
| Reduced context switching | Pass | Chat + sources in one view |
| No unnecessary complexity | Pass | Toggles hidden behind a settings panel |
| System state visible | Pass | Streaming, status, errors all shown |

---

## Issues

### Critical Issues (P0)

None.

### Major Issues (P1) — All Resolved

1. **Entity refs discarded before send** — `getEntityRefIds()` is now called before `selectedEntityRefs` is cleared, and the ids are threaded into `fallbackSend` (Chat.svelte `sendMessage`).
2. **No keyboard shortcuts help** — `?` opens a shortcuts help modal (ignored while typing); `/help` command opens it too; global shortcuts (Ctrl+N, Ctrl+F, Ctrl+1-8) are documented in the modal.
3. **Conversation sidebar has no search** — Search input added above the list; filters by title and preview with an empty-state message.
4. **Modal missing `role="dialog"` / `aria-modal`** — Both delete and clear modals now expose `role="dialog"`, `aria-modal="true"`, and `aria-labelledby`.
5. **No-op commands** — `/help`, `/clear`, and `/export` now work: `/help` opens shortcuts help, `/clear` resets the conversation after confirmation, `/export` downloads the conversation as Markdown.

### Minor Issues (P2) — All Resolved

6. **Input area visual complexity** — Mode and source toggles moved into a collapsible settings panel toggled by a `tune` button.
7. **No message copy button** — Copy button added to every message with a check-icon confirmation.
8. **No conversation export** — `/export` downloads the current conversation as a `.md` file.
9. **No in-conversation search** — `Ctrl+F` opens an in-conversation find bar with live highlighting, match count, and prev/next navigation; the global Ctrl+F handler now defers to Chat.
10. **Hardcoded rgba colors** — All rgba/hex overrides replaced with design system tokens (`--text-on-accent-*`, `--surface-on-accent-subtle`, `--hover-on-accent`, `--border-on-accent`, `--overlay-*`, `--danger-soft`, `--danger-hover`).
11. **Citations and feedback lost on reload** — Backend now surfaces `citations` and `feedback` from conversation history; `selectConversation` hydrates both. Feedback rating/reason serialization fixed (`snake_case`) so feedback actually persists.

---

## Recommendations

1. This view is the reference implementation for streaming, feedback, and error recovery patterns.
2. Future work: consider persisting the input settings (mode, toggles) across sessions, and exporting to additional formats (JSON) alongside Markdown.
