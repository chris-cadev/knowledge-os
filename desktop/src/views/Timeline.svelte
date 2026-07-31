<script lang="ts">
  import { onMount, onDestroy } from "svelte";
  import { listEntities } from "../lib/api.js";
  import { navigateTo } from "../lib/router.svelte.js";
  import { getEntityTypeColor } from "../lib/theme.svelte.js";
  import type { EntitySummary } from "../lib/types.js";

  type LoadState = "loading" | "stalled" | "error" | "success";
  let loadState = $state<LoadState>("loading");
  let errorMsg = $state("");

  let allEntities = $state<EntitySummary[] | null>(null);
  let entityTypes = $state<string[]>([]);
  let entityTypeFilter = $state("");

  let items = $derived(
    (allEntities ?? [])
      .filter((e) => !entityTypeFilter || e.entity_type === entityTypeFilter)
      .sort((a, b) => new Date(a.created_at).getTime() - new Date(b.created_at).getTime())
  );

  let abort: AbortController | null = null;
  let stallTimer: ReturnType<typeof setTimeout> | null = null;

  onMount(() => {
    loadTimeline();
  });

  onDestroy(() => {
    abort?.abort();
    if (stallTimer) clearTimeout(stallTimer);
  });

  async function loadTimeline() {
    abort?.abort();
    abort = new AbortController();
    const signal = abort.signal;

    loadState = "loading";
    errorMsg = "";

    const timer = setTimeout(() => {
      if (loadState === "loading") loadState = "stalled";
    }, 5000);
    stallTimer = timer;

    try {
      const entities = await listEntities();
      if (signal.aborted) return;
      allEntities = entities;
      entityTypes = [...new Set(entities.map((e) => e.entity_type))].sort();
      loadState = "success";
    } catch (e) {
      if (signal.aborted) return;
      errorMsg = `${e}`;
      loadState = "error";
    } finally {
      clearTimeout(timer);
      stallTimer = null;
    }
  }

  function selectEntity(id: string) {
    navigateTo("detail", id);
  }

  function formatDate(dateStr: string): string {
    const date = new Date(dateStr);
    return date.toLocaleDateString(undefined, {
      year: "numeric",
      month: "long",
      day: "numeric",
    });
  }

  function getGroupLabel(dateStr: string): string {
    const date = new Date(dateStr);
    const now = new Date();
    const diffMs = now.getTime() - date.getTime();
    const diffDays = Math.floor(diffMs / 86400000);
    if (diffDays === 0) return "Today";
    if (diffDays === 1) return "Yesterday";
    if (diffDays < 7) return "This Week";
    if (diffDays < 30) return "This Month";
    if (diffDays < 90) return "Last 3 Months";
    if (diffDays < 365) return "This Year";
    return "Older";
  }


  let grouped = $derived.by(() => {
    const groups: Record<string, EntitySummary[]> = {};
    for (const item of items) {
      const label = getGroupLabel(item.created_at);
      if (!groups[label]) groups[label] = [];
      groups[label].push(item);
    }
    const order = ["Today", "Yesterday", "This Week", "This Month", "Last 3 Months", "This Year", "Older"];
    const sorted: Array<{ label: string; items: EntitySummary[] }> = [];
    for (const key of order) {
      if (groups[key]) sorted.push({ label: key, items: groups[key] });
    }
    return sorted;
  });
</script>

<div class="timeline">
  <div class="timeline-header">
    <h2>Timeline View</h2>
    <div class="controls">
      <select
        bind:value={entityTypeFilter}
        class="filter-select"
        disabled={loadState === "loading" || loadState === "stalled"}
      >
        <option value="">All types</option>
        {#each entityTypes as type}
          <option value={type}>{type}</option>
        {/each}
      </select>
    </div>
  </div>

  {#if loadState === "error"}
    <div class="state-banner error">
      <span>{errorMsg || "Could not load timeline."}</span>
      <button class="btn btn-sm" onclick={loadTimeline}>Retry</button>
    </div>
    {#if items.length > 0}
      <div class="timeline-container stale">
        {#each grouped as group}
          <div class="timeline-group">
            <div class="group-header">
              <span class="group-label">{group.label}</span>
              <span class="group-count text-muted">{group.items.length} item{group.items.length !== 1 ? "s" : ""}</span>
            </div>
            {#each group.items as item (item.id + item.created_at)}
              <div class="timeline-entry">
                <div class="timeline-marker">
                  <div class="timeline-dot" style="background: {getEntityTypeColor(item.entity_type)}"></div>
                  {#if item !== group.items[group.items.length - 1]}
                    <div class="timeline-line"></div>
                  {/if}
                </div>
                <div class="entry-card" onclick={() => selectEntity(item.id)}
                  onkeydown={(e) => { if (e.key === 'Enter') selectEntity(item.id); }}
                  role="button" tabindex="0" aria-label={item.title}>
                  <div class="entry-header">
                    <span class="type-badge" style="background: {getEntityTypeColor(item.entity_type)}">{item.entity_type}</span>
                    <span class="timestamp">{formatDate(item.created_at)}</span>
                  </div>
                  <div class="entry-title">{item.title}</div>
                </div>
              </div>
            {/each}
          </div>
        {/each}
      </div>
    {/if}
  {:else if loadState === "loading" || loadState === "stalled"}
    {#if items.length === 0 && !allEntities}
      <div class="skeleton">
        {#each [0, 1, 2, 3] as _}
          <div class="skeleton-group">
            <div class="skeleton-header"></div>
            {#each [0, 1, 2] as _}
              <div class="skeleton-entry">
                <div class="skeleton-dot"></div>
                <div class="skeleton-card">
                  <div class="skeleton-title"></div>
                  <div class="skeleton-date"></div>
                </div>
              </div>
            {/each}
          </div>
        {/each}
      </div>
    {:else}
      <div class="timeline-container stale">
        {#each grouped as group}
          <div class="timeline-group">
            <div class="group-header">
              <span class="group-label">{group.label}</span>
              <span class="group-count text-muted">{group.items.length} item{group.items.length !== 1 ? "s" : ""}</span>
            </div>
            {#each group.items as item (item.id + item.created_at)}
              <div class="timeline-entry">
                <div class="timeline-marker">
                  <div class="timeline-dot" style="background: {getEntityTypeColor(item.entity_type)}"></div>
                  {#if item !== group.items[group.items.length - 1]}
                    <div class="timeline-line"></div>
                  {/if}
                </div>
                <div class="entry-card" onclick={() => selectEntity(item.id)}
                  onkeydown={(e) => { if (e.key === 'Enter') selectEntity(item.id); }}
                  role="button" tabindex="0" aria-label={item.title}>
                  <div class="entry-header">
                    <span class="type-badge" style="background: {getEntityTypeColor(item.entity_type)}">{item.entity_type}</span>
                    <span class="timestamp">{formatDate(item.created_at)}</span>
                  </div>
                  <div class="entry-title">{item.title}</div>
                </div>
              </div>
            {/each}
          </div>
        {/each}
      </div>
    {/if}
    {#if loadState === "stalled"}
      <div class="state-banner warning">
        <span>Still loading… taking longer than expected.</span>
        <button class="btn btn-sm" onclick={loadTimeline}>Retry</button>
      </div>
    {:else}
      <p class="loading-text">Loading timeline…</p>
    {/if}
  {:else if items.length === 0}
    <div class="state-banner empty">
      {#if allEntities && allEntities.length > 0}
        <span>No items match the selected type filter.</span>
      {:else}
        <span>No timeline items found. Import some documents to get started.</span>
        <button class="btn btn-primary btn-sm" onclick={() => navigateTo("import")}>Import Documents</button>
      {/if}
    </div>
  {:else}
    <div class="timeline-container">
      {#each grouped as group}
        <div class="timeline-group">
          <div class="group-header">
            <span class="group-label">{group.label}</span>
            <span class="group-count text-muted">{group.items.length} item{group.items.length !== 1 ? "s" : ""}</span>
          </div>
          {#each group.items as item (item.id + item.created_at)}
            <div class="timeline-entry">
              <div class="timeline-marker">
                <div class="timeline-dot" style="background: {getEntityTypeColor(item.entity_type)}"></div>
                {#if item !== group.items[group.items.length - 1]}
                  <div class="timeline-line"></div>
                {/if}
              </div>
              <div class="entry-card" onclick={() => selectEntity(item.id)}
                onkeydown={(e) => { if (e.key === 'Enter') selectEntity(item.id); }}
                role="button" tabindex="0" aria-label={item.title}>
                <div class="entry-header">
                  <span class="type-badge" style="background: {getEntityTypeColor(item.entity_type)}">{item.entity_type}</span>
                  <span class="timestamp">{formatDate(item.created_at)}</span>
                </div>
                <div class="entry-title">{item.title}</div>
              </div>
            </div>
          {/each}
        </div>
      {/each}
    </div>
  {/if}
</div>

<style>
  .timeline-header {
    display: flex;
    align-items: center;
    justify-content: space-between;
    margin-bottom: var(--spacing-lg);
    flex-wrap: wrap;
    gap: var(--spacing-md);
  }

  .timeline-header h2 {
    font-size: var(--font-size-2xl);
  }

  .controls {
    display: flex;
    gap: var(--spacing-md);
    align-items: center;
  }

  .filter-select {
    padding: var(--spacing-sm) var(--spacing-md);
    border: 1px solid var(--border);
    border-radius: var(--radius-sm);
    background: var(--bg-card);
    color: var(--text-primary);
    font-size: var(--font-size-body-sm);
  }

  .filter-select:disabled {
    opacity: 0.5;
    cursor: not-allowed;
  }

  .loading-text {
    color: var(--text-secondary);
    font-size: var(--font-size-sm);
    margin-top: var(--spacing-md);
  }

  .state-banner {
    display: flex;
    align-items: center;
    gap: var(--spacing-md);
    padding: var(--spacing-md);
    border-radius: var(--radius-md);
    font-size: var(--font-size-body-sm);
    margin-bottom: var(--spacing-md);
  }

  .state-banner.error {
    background: rgba(239, 68, 68, 0.1);
    border: 1px solid rgba(239, 68, 68, 0.3);
    color: var(--text-primary);
  }

  .state-banner.warning {
    background: rgba(245, 158, 11, 0.1);
    border: 1px solid rgba(245, 158, 11, 0.3);
    color: var(--text-primary);
    margin-top: var(--spacing-md);
  }

  .state-banner.empty {
    background: var(--bg-secondary);
    border: 1px solid var(--border);
    color: var(--text-primary);
  }

  .stale {
    opacity: 0.6;
    pointer-events: none;
  }

  .timeline-container {
    display: flex;
    flex-direction: column;
    gap: var(--spacing-xl);
  }

  .timeline-group {
    display: flex;
    flex-direction: column;
  }

  .group-header {
    display: flex;
    align-items: center;
    gap: var(--spacing-md);
    padding: var(--spacing-md) 0;
    border-bottom: 1px solid var(--border);
    margin-bottom: var(--spacing-md);
    position: sticky;
    top: 0;
    background: var(--bg);
    z-index: 1;
  }

  .group-label {
    font-weight: 600;
    font-size: var(--font-size-body-lg);
  }

  .group-count {
    font-size: var(--font-size-sm);
  }

  .timeline-entry {
    display: flex;
    gap: var(--spacing-md);
    position: relative;
  }

  .timeline-marker {
    display: flex;
    flex-direction: column;
    align-items: center;
    min-width: 24px;
    padding-top: 4px;
  }

  .timeline-dot {
    width: 12px;
    height: 12px;
    border-radius: 50%;
    flex-shrink: 0;
    box-shadow: 0 0 0 2px var(--bg);
  }

  .timeline-line {
    width: 2px;
    flex: 1;
    background: var(--border);
    min-height: 20px;
  }

  .entry-card {
    flex: 1;
    padding: var(--spacing-md);
    background: var(--bg-card);
    border: 1px solid var(--border);
    border-radius: var(--radius-md);
    cursor: pointer;
    transition: border-color var(--transition-fast), box-shadow var(--transition-fast);
    margin-bottom: var(--spacing-md);
  }

  .entry-card:hover {
    border-color: var(--accent);
    box-shadow: 0 2px 8px rgba(0, 0, 0, 0.08);
  }

  .entry-header {
    display: flex;
    align-items: center;
    gap: var(--spacing-md);
    margin-bottom: var(--spacing-xs);
  }

  .type-badge {
    display: inline-block;
    padding: 2px 8px;
    color: white;
    border-radius: var(--radius-sm);
    font-size: var(--font-size-sm);
    font-weight: 500;
  }

  .timestamp {
    font-size: var(--font-size-sm);
    color: var(--text-secondary);
  }

  .entry-title {
    font-weight: 500;
    font-size: var(--font-size-body-md);
  }

  .skeleton {
    display: flex;
    flex-direction: column;
    gap: var(--spacing-xl);
    animation: pulse 1.5s ease-in-out infinite;
  }

  @keyframes pulse {
    0%, 100% { opacity: 1; }
    50% { opacity: 0.4; }
  }

  .skeleton-group {
    display: flex;
    flex-direction: column;
    gap: var(--spacing-md);
  }

  .skeleton-header {
    height: 20px;
    width: 120px;
    background: var(--border);
    border-radius: var(--radius-sm);
    margin-bottom: var(--spacing-sm);
  }

  .skeleton-entry {
    display: flex;
    gap: var(--spacing-md);
    align-items: flex-start;
  }

  .skeleton-dot {
    width: 12px;
    height: 12px;
    border-radius: 50%;
    background: var(--border);
    flex-shrink: 0;
    margin-top: 4px;
  }

  .skeleton-card {
    flex: 1;
    padding: var(--spacing-md);
    border: 1px solid var(--border);
    border-radius: var(--radius-md);
    display: flex;
    flex-direction: column;
    gap: var(--spacing-sm);
  }

  .skeleton-title {
    height: 16px;
    width: 70%;
    background: var(--border);
    border-radius: var(--radius-sm);
  }

  .skeleton-date {
    height: 12px;
    width: 40%;
    background: var(--border);
    border-radius: var(--radius-sm);
  }
</style>
