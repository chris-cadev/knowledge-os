// === Knowledge OS — Keyboard Shortcuts ===
// Browser-level keyboard navigation for the desktop app.

import { navigateTo } from "./router.svelte.js";
import type { View } from "./types.js";

const viewShortcuts: Record<string, View> = {
  "1": "dashboard",
  "2": "browse",
  "3": "graph",
  "4": "tree",
  "5": "table",
  "6": "timeline",
  "7": "import",
  "8": "search",
};

export function initShortcuts() {
  function handleKeyDown(e: KeyboardEvent) {
    // Don't trigger shortcuts when typing in input fields
    const target = e.target as HTMLElement;
    if (
      target.tagName === "INPUT" ||
      target.tagName === "TEXTAREA" ||
      target.tagName === "SELECT" ||
      target.isContentEditable
    ) {
      return;
    }

    // Ctrl+N → Import
    if ((e.ctrlKey || e.metaKey) && e.key === "n") {
      e.preventDefault();
      navigateTo("import");
      return;
    }

    // Ctrl+F → Search
    if ((e.ctrlKey || e.metaKey) && e.key === "f") {
      e.preventDefault();
      navigateTo("search");
      return;
    }

    // Ctrl+1-8 → Navigate to view
    if ((e.ctrlKey || e.metaKey) && viewShortcuts[e.key]) {
      e.preventDefault();
      navigateTo(viewShortcuts[e.key]);
      return;
    }
  }

  window.addEventListener("keydown", handleKeyDown);
}
