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
