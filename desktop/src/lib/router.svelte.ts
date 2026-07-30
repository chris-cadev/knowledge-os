// === Knowledge OS — Simple hash-based router ===
// Maps URL hash fragments to view names.
// Example: #/browse -> "browse", #/entity/uuid -> "detail"

import { getState } from "./state.svelte.js";
import type { View } from "./types.js";

const state = getState();

export function initRouter() {
  function handleHash() {
    const hash = window.location.hash.slice(1) || "/";

    if (hash.startsWith("/entity/")) {
      const id = hash.split("/")[2];
      state.selectedEntityId = id;
      state.currentView = "detail";
    } else {
      const viewMap: Record<string, View> = {
        "/": "dashboard",
        "/browse": "browse",
        "/graph": "graph",
        "/tree": "tree",
        "/table": "table",
        "/timeline": "timeline",
        "/import": "import",
        "/search": "search",
        "/chat": "chat",
      };
      state.currentView = viewMap[hash] ?? "dashboard";
    }
  }

  window.addEventListener("hashchange", handleHash);
  handleHash(); // Handle on load
}

export function navigateTo(view: View, entityId?: string) {
  if (view === "detail" && entityId) {
    window.location.hash = `/entity/${entityId}`;
  } else {
    const hashMap: Record<View, string> = {
      dashboard: "/",
      browse: "/browse",
      detail: "/",
      graph: "/graph",
      tree: "/tree",
      table: "/table",
      timeline: "/timeline",
      import: "/import",
      search: "/search",
      chat: "/chat",
    };
    window.location.hash = hashMap[view] ?? "/";
  }
}
