<script lang="ts">
  import { getState } from "./lib/state.svelte.js";
  import { initTheme } from "./lib/theme.svelte.js";
  import { initRouter } from "./lib/router.svelte.js";
  import { initShortcuts } from "./lib/shortcuts.svelte.js";
  import Sidebar from "./views/Sidebar.svelte";
  import StatusBar from "./views/StatusBar.svelte";
  import Dashboard from "./views/Dashboard.svelte";
  import Browser from "./views/Browser.svelte";
  import Detail from "./views/Detail.svelte";
  import Graph from "./views/Graph.svelte";
  import Tree from "./views/Tree.svelte";
  import Table from "./views/Table.svelte";
  import Timeline from "./views/Timeline.svelte";
  import Import from "./views/Import.svelte";
  import Search from "./views/Search.svelte";
  import Chat from "./views/Chat.svelte";

  initTheme();
  initRouter();
  initShortcuts();

  const state = getState();

  const views: Record<string, any> = {
    dashboard: Dashboard,
    browse: Browser,
    detail: Detail,
    graph: Graph,
    tree: Tree,
    table: Table,
    timeline: Timeline,
    import: Import,
    search: Search,
    chat: Chat,
  };

  let CurrentView = $derived(views[state.currentView] ?? Dashboard);
</script>

<div class="app-shell">
  <Sidebar />
  <div class="main-area">
    <main class="content">
      <CurrentView />
    </main>
    <StatusBar />
  </div>
</div>

<style>
  .app-shell {
    display: flex;
    height: 100vh;
    overflow: hidden;
  }

  .main-area {
    flex: 1;
    display: flex;
    flex-direction: column;
    min-width: 0;
  }

  .content {
    flex: 1;
    overflow-y: auto;
    padding: var(--space-6);
    background: var(--color-surface);
  }
</style>
