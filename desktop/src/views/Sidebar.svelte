<script lang="ts">
  import { onMount } from "svelte";
  import { getState } from "../lib/state.svelte.js";
  import { navigateTo } from "../lib/router.svelte.js";
  import type { View } from "../lib/types.js";

  const state = getState();

  interface NavItem {
    view: View;
    label: string;
    icon: string;
  }

  const navItems: NavItem[] = [
    { view: "dashboard", label: "Dashboard", icon: "dashboard" },
    { view: "browse", label: "Browse", icon: "explore" },
    { view: "graph", label: "Graph", icon: "bubble_chart" },
    { view: "tree", label: "Tree", icon: "account_tree" },
    { view: "table", label: "Table", icon: "table_chart" },
    { view: "timeline", label: "Timeline", icon: "timeline" },
    { view: "chat", label: "Chat", icon: "chat" },
    { view: "import", label: "Import", icon: "file_upload" },
    { view: "search", label: "Search", icon: "search" },
    { view: "settings", label: "Settings", icon: "settings" },
  ];

  let collapsed = $state<boolean>(false);

  onMount(() => {
    collapsed = sessionStorage.getItem("sidebar-collapsed") === "true";
  });

  function toggleCollapse() {
    collapsed = !collapsed;
    sessionStorage.setItem("sidebar-collapsed", String(collapsed));
  }
</script>

<aside class="sidebar" class:collapsed>
  <div class="logo">
    <h1 class="logo-title">Knowledge OS</h1>
    <span class="logo-subtitle">v0.1.0 SYSTEM CORE</span>
  </div>

  <nav class="nav-list">
    {#each navItems as item}
      <button
        class="nav-item"
        class:active={state.currentView === item.view}
        onclick={() => navigateTo(item.view)}
        title={collapsed ? item.label : undefined}
        aria-current={state.currentView === item.view ? "page" : false}
      >
        <span class="nav-icon material-symbols-outlined">{item.icon}</span>
        <span class="nav-label">{item.label}</span>
      </button>
    {/each}
  </nav>

  <button class="collapse-toggle" onclick={toggleCollapse} title={collapsed ? "Expand sidebar" : "Collapse sidebar"}>
    <span class="collapse-icon material-symbols-outlined">chevron_left</span>
  </button>

  <div class="sidebar-footer">
    <div class="status-bar">
      <span class="status-text">{state.entityCount} entities</span>
    </div>
  </div>
</aside>

<style>
  .sidebar {
    width: var(--sidebar-width);
    min-width: var(--sidebar-width);
    background: var(--color-sidebar-bg);
    color: var(--color-sidebar-text);
    display: flex;
    flex-direction: column;
    padding: var(--space-8) 0;
    border-right: 1px solid var(--color-sidebar-border);
    user-select: none;
    transition: width var(--transition-fast), min-width var(--transition-fast);
  }

  .sidebar.collapsed {
    --sidebar-width: 48px;
  }

  .sidebar.collapsed .logo,
  .sidebar.collapsed .sidebar-footer {
    display: none;
  }

  .sidebar.collapsed .nav-item {
    justify-content: center;
    padding: 14px 0;
  }

  .sidebar.collapsed .nav-label {
    display: none;
  }

  .sidebar.collapsed .collapse-icon {
    transform: rotate(180deg);
  }

  .logo {
    padding: 0 var(--space-8);
    margin-bottom: var(--space-6);
  }

  .logo-title {
    font-size: var(--font-size-title-sm);
    font-weight: 700;
    color: var(--color-sidebar-logo);
    letter-spacing: -0.01em;
    line-height: 1.3;
  }

  .logo-subtitle {
    font-size: var(--font-size-label-caps);
    color: var(--color-sidebar-subtitle);
    letter-spacing: 0.05em;
    text-transform: uppercase;
    margin-top: var(--space-1);
    display: block;
  }

  .nav-list {
    display: flex;
    flex-direction: column;
    gap: 2px;
    flex: 1;
    padding: 0;
  }

  .nav-item {
    position: relative;
    display: flex;
    align-items: center;
    gap: var(--space-4);
    padding: 14px var(--space-8);
    color: var(--color-sidebar-text);
    font-size: var(--font-size-body-md);
    font-weight: 500;
    line-height: 1;
    transition: color var(--transition-fast), background var(--transition-fast);
    width: 100%;
    text-align: left;
    border-radius: 0;
  }

  .nav-item:hover {
    background: var(--color-sidebar-hover-bg);
    color: var(--color-sidebar-text-hover);
  }

  .nav-item.active {
    background: var(--color-sidebar-active-bg);
    color: var(--color-sidebar-text-active);
  }

  .nav-item.active::before {
    content: "";
    position: absolute;
    left: 0;
    top: 0;
    bottom: 0;
    width: 4px;
    background: var(--color-sidebar-active-stripe);
  }

  .nav-icon {
    font-size: 20px;
    width: 20px;
    height: 20px;
    display: flex;
    align-items: center;
    justify-content: center;
    flex-shrink: 0;
  }

  .nav-label {
    flex: 1;
  }

  .collapse-toggle {
    display: flex;
    align-items: center;
    justify-content: center;
    width: 100%;
    padding: var(--space-3) 0;
    color: var(--color-sidebar-subtitle);
    transition: color var(--transition-fast);
  }

  .collapse-toggle:hover {
    color: var(--color-sidebar-text-hover);
  }

  .collapse-icon {
    transition: transform var(--transition-fast);
  }

  .sidebar-footer {
    padding: var(--space-4) var(--space-8) 0;
    margin-top: var(--space-2);
  }

  .status-bar {
    padding-top: var(--space-4);
    border-top: 1px solid var(--color-sidebar-divider);
  }

  .status-text {
    font-size: var(--font-size-body-sm);
    color: var(--color-sidebar-subtitle);
  }
</style>
