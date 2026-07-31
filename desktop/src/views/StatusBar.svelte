<script lang="ts">
  import { getState } from "../lib/state.svelte.js";
  import { getTheme } from "../lib/theme.svelte.js";
  import { navigateTo } from "../lib/router.svelte.js";

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
      <span class="status-message" aria-live="polite">{app.statusMessage}</span>
    {:else}
      <span class="status-text">{app.entityCount} entit{app.entityCount === 1 ? "y" : "ies"}</span>
      <span class="status-separator">|</span>
      <button
        class="status-provider"
        class:reachable={app.providerReachable}
        class:unreachable={!app.providerReachable}
        onclick={() => navigateTo("settings")}
        aria-label={app.providerReachable ? `${app.providerName} connected, click to open settings` : `${app.providerName} disconnected, click to open settings`}
      >
        <span class="status-provider-label">{app.providerReachable ? "Connected" : "Disconnected"}</span>
        {app.providerName}
        {#if app.providerModel}
          <span class="provider-model">({app.providerModel})</span>
        {/if}
      </button>
    {/if}
  </div>
  <div class="status-right">
    <span class="status-view">{app.currentView}</span>
    <span class="status-separator">|</span>
    <span class="material-symbols-outlined status-theme-icon" aria-label={theme.isDark ? "Dark mode" : "Light mode"}>
      {theme.isDark ? "dark_mode" : "light_mode"}
    </span>
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

  .status-provider {
    all: unset;
    cursor: pointer;
    display: inline-flex;
    align-items: center;
    gap: var(--space-1);
    color: var(--text-secondary);
    border-radius: var(--radius-sm, 4px);
  }

  .status-provider:focus-visible {
    outline: 2px solid var(--accent);
    outline-offset: 2px;
  }

  .status-provider-label {
    font-size: 0.85em;
    opacity: 0.8;
  }

  .status-provider.reachable {
    color: var(--accent);
  }

  .status-provider.unreachable {
    color: var(--color-error);
  }

  .provider-model {
    font-size: 0.9em;
    opacity: 0.7;
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

  .status-theme-icon {
    font-size: 16px;
  }
</style>
