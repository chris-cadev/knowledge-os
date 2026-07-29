<script lang="ts">
  import { getState } from "../lib/state.svelte.js";
  import { getTheme } from "../lib/theme.svelte.js";

  const app = getState();
  const theme = getTheme();

  $effect(() => {
    if (app.statusMessage) {
      const timer = setTimeout(() => {
        app.statusMessage = "";
      }, 5000);
      return () => clearTimeout(timer);
    }
  });
</script>

<div class="status-bar">
  <div class="status-left">
    {#if app.statusMessage}
      <span class="status-message">{app.statusMessage}</span>
    {:else}
      <span class="status-text">{app.entityCount} entit{app.entityCount === 1 ? "y" : "ies"}</span>
    {/if}
  </div>
  <div class="status-right">
    <span class="status-view">{app.currentView}</span>
    <span class="status-separator">|</span>
    <span class="status-theme">{theme.isDark ? "🌙" : "☀️"}</span>
  </div>
</div>

<style>
  .status-bar {
    display: flex;
    align-items: center;
    justify-content: space-between;
    padding: var(--space-1) var(--space-4);
    background: var(--color-surface-container-high);
    border-top: 1px solid var(--border);
    font-size: var(--font-size-body-sm);
    color: var(--text-secondary);
    user-select: none;
    min-height: 28px;
  }

  .status-left {
    display: flex;
    align-items: center;
    gap: var(--space-2);
  }

  .status-message {
    color: var(--accent);
    font-weight: 500;
  }

  .status-text {
    color: var(--text-secondary);
  }

  .status-right {
    display: flex;
    align-items: center;
    gap: var(--space-2);
  }

  .status-view {
    text-transform: capitalize;
  }

  .status-separator {
    color: var(--border);
  }

  .status-theme {
    font-size: 12px;
  }
</style>
