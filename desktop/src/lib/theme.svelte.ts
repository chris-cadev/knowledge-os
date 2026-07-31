// === Knowledge OS — Reactive Theme (DESIGN.md) ===
// Detects OS color scheme preference and exposes a reactive `isDark` flag.
// Future: support manual toggle via data-theme attribute.

let isDark = $state(false);
let mediaQuery: MediaQueryList | null = null;
let listener: ((e: MediaQueryListEvent) => void) | null = null;

export function getTheme() {
  return {
    get isDark() {
      return isDark;
    },
  };
}

export function initTheme() {
  if (listener) return;

  mediaQuery = window.matchMedia("(prefers-color-scheme: dark)");
  isDark = mediaQuery.matches;

  listener = (e: MediaQueryListEvent) => {
    isDark = e.matches;
  };

  mediaQuery.addEventListener("change", listener);
}

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
