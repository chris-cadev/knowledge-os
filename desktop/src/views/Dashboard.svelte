<script lang="ts">
  import { getState } from "../lib/state.svelte.js";
  import { onMount } from "svelte";
  import { listEntities } from "../lib/api.js";
  import { navigateTo } from "../lib/router.svelte.js";
  import { getEntityTypeColor } from "../lib/theme.svelte.js";
  import TypeBadge from "../components/TypeBadge.svelte";
  import type { EntitySummary } from "../lib/types.js";

  const app = getState();

  let loading = $state(true);
  let error = $state("");
  let typeDistribution = $state<Array<{ type: string; count: number }>>([]);
  let recentEntities = $state<EntitySummary[]>([]);

  onMount(async () => {
    await loadDashboard();
  });

  async function loadDashboard() {
    loading = true;
    error = "";
    try {
      const entities = await listEntities();
      app.entities = entities;
      app.entityCount = entities.length;

      const typeMap = new Map<string, number>();
      for (const e of entities) {
        typeMap.set(e.entity_type, (typeMap.get(e.entity_type) ?? 0) + 1);
      }
      typeDistribution = Array.from(typeMap.entries())
        .map(([type, count]) => ({ type, count }))
        .sort((a, b) => b.count - a.count);

      recentEntities = entities
        .slice()
        .sort((a, b) => new Date(b.created_at).getTime() - new Date(a.created_at).getTime())
        .slice(0, 5);
    } catch (e) {
      error = `Failed to load dashboard: ${e}`;
      app.statusMessage = error;
    } finally {
      loading = false;
    }
  }

  function openEntity(id: string) {
    navigateTo("detail", id);
  }

  function navigateToView(view: "import" | "search" | "graph" | "browse") {
    navigateTo(view);
  }

  function navigateToBrowse() {
    navigateTo("browse");
  }

  function getTotalEntityCount(): number {
    return typeDistribution.reduce((sum, t) => sum + t.count, 0);
  }

  function handleCardKeydown(event: KeyboardEvent) {
    if (event.key === "Enter" || event.key === " ") {
      event.preventDefault();
      (event.currentTarget as HTMLElement).click();
    }
  }
</script>

<div class="dashboard">
  <div class="dashboard-header">
    <div>
      <h2>Dashboard</h2>
      <p class="text-muted welcome">Welcome to Knowledge OS. Your personal knowledge graph.</p>
    </div>
    <div class="header-actions">
      <button class="btn btn-primary" onclick={() => navigateToView("import")}>
        <span class="material-symbols-outlined">file_upload</span>
        Import Documents
      </button>
    </div>
  </div>

  {#if error}
    <div class="state-banner error" role="alert">
      <span class="material-symbols-outlined">error</span>
      <span>{error}</span>
      <button class="btn btn-sm" onclick={loadDashboard}>Retry</button>
    </div>
  {/if}

  {#if loading}
    <div class="bento-grid skeleton-pulse" aria-label="Loading dashboard">
      <div class="bento-card skeleton-stat">
        <div class="skeleton-block skeleton-block-sm"></div>
        <div class="skeleton-block skeleton-block-xl"></div>
        <div class="skeleton-block skeleton-block-sm"></div>
      </div>
      <div class="bento-card skeleton-stat">
        <div class="skeleton-block skeleton-block-sm"></div>
        <div class="skeleton-block skeleton-block-xl"></div>
        <div class="skeleton-block skeleton-block-sm"></div>
      </div>
      <div class="bento-card skeleton-stat">
        <div class="skeleton-block skeleton-block-sm"></div>
        <div class="skeleton-block skeleton-block-xl"></div>
        <div class="skeleton-block skeleton-block-sm"></div>
      </div>
      <div class="bento-card skeleton-stat">
        <div class="skeleton-block skeleton-block-sm"></div>
        <div class="skeleton-block skeleton-block-md"></div>
        <div class="skeleton-block skeleton-block-md"></div>
      </div>
      <div class="bento-card skeleton-distribution">
        <div class="skeleton-block skeleton-block-md"></div>
        <div class="skeleton-block skeleton-block-lg"></div>
        <div class="skeleton-block skeleton-block-lg"></div>
        <div class="skeleton-block skeleton-block-lg"></div>
      </div>
      <div class="bento-card skeleton-recent">
        <div class="skeleton-block skeleton-block-md"></div>
        <div class="skeleton-block skeleton-block-lg"></div>
        <div class="skeleton-block skeleton-block-lg"></div>
        <div class="skeleton-block skeleton-block-lg"></div>
      </div>
    </div>
  {:else if !error}
    <div class="bento-grid">
      <div class="bento-card card-summary" tabindex="0" role="button" aria-label="Total entities: {getTotalEntityCount()}" onclick={navigateToBrowse} onkeydown={handleCardKeydown}>
        <div class="card-icon">
          <span class="material-symbols-outlined">database</span>
        </div>
        <div class="card-value">{getTotalEntityCount()}</div>
        <div class="card-label">Total Entities</div>
      </div>

      <div class="bento-card card-active" tabindex="0" role="button" aria-label="Active entities: {app.entities.filter((e) => e.is_active).length}" onclick={navigateToBrowse} onkeydown={handleCardKeydown}>
        <div class="card-icon">
          <span class="material-symbols-outlined">check_circle</span>
        </div>
        <div class="card-value">{app.entities.filter((e) => e.is_active).length}</div>
        <div class="card-label">Active</div>
      </div>

      <div class="bento-card card-types" tabindex="0" role="button" aria-label="Entity types: {typeDistribution.length}" onclick={navigateToBrowse} onkeydown={handleCardKeydown}>
        <div class="card-icon">
          <span class="material-symbols-outlined">category</span>
        </div>
        <div class="card-value">{typeDistribution.length}</div>
        <div class="card-label">Entity Types</div>
      </div>

      <div class="bento-card card-quick-actions">
        <div class="card-header">Quick Actions</div>
        <div class="action-buttons">
          <button class="action-btn" onclick={() => navigateToView("import")}>
            <span class="material-symbols-outlined">file_upload</span>
            <span>Import</span>
          </button>
          <button class="action-btn" onclick={() => navigateToView("search")}>
            <span class="material-symbols-outlined">search</span>
            <span>Search</span>
          </button>
          <button class="action-btn" onclick={() => navigateToView("graph")}>
            <span class="material-symbols-outlined">bubble_chart</span>
            <span>Graph</span>
          </button>
          <button class="action-btn" onclick={() => navigateToView("browse")}>
            <span class="material-symbols-outlined">explore</span>
            <span>Browse</span>
          </button>
        </div>
      </div>

      <div class="bento-card card-distribution" tabindex="0" aria-label="Entity distribution by type" onkeydown={handleCardKeydown}>
        <div class="card-header">Entity Distribution by Type</div>
        {#if typeDistribution.length === 0}
          <p class="text-muted empty-text">No entities found.</p>
        {:else}
          <div class="distribution-bars">
            {#each typeDistribution as item}
              <div class="dist-row">
                <span class="dist-type">{item.type}</span>
                <div class="dist-bar-track">
                  <div
                    class="dist-bar-fill"
                    style="width: {(item.count / getTotalEntityCount()) * 100}%; background: {getEntityTypeColor(item.type)}"
                  ></div>
                </div>
                <span class="dist-count">{item.count}</span>
              </div>
            {/each}
          </div>
        {/if}
      </div>

      <div class="bento-card card-recent" tabindex="0" aria-label="Recent entities" onkeydown={handleCardKeydown}>
        <div class="card-header">Recent Entities</div>
        {#if recentEntities.length === 0}
          <p class="text-muted empty-text">No entities yet. Import some documents to get started.</p>
        {:else}
          <div class="recent-list">
            {#each recentEntities as entity}
              <button class="recent-item" onclick={() => openEntity(entity.id)}>
                <TypeBadge type={entity.entity_type} />
                <span class="recent-title truncate">{entity.title}</span>
                <span class="recent-date text-muted">{entity.created_at.slice(0, 10)}</span>
              </button>
            {/each}
          </div>
        {/if}
      </div>
    </div>
  {/if}
</div>

<style>
  .dashboard-header {
    display: flex;
    align-items: flex-start;
    justify-content: space-between;
    margin-bottom: var(--spacing-lg);
  }

  .dashboard h2 {
    font-size: var(--font-size-2xl);
  }

  .welcome {
    font-size: var(--font-size-body-sm);
    margin-top: var(--spacing-xs);
  }

  .header-actions {
    display: flex;
    gap: var(--spacing-md);
  }

  /* ===== Error Banner ===== */
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

  .state-banner .material-symbols-outlined {
    color: #ef4444;
    flex-shrink: 0;
  }

  /* ===== Bento Grid ===== */
  .bento-grid {
    display: grid;
    grid-template-columns: 1fr 1fr 1fr 1fr;
    gap: var(--spacing-md);
    grid-auto-rows: minmax(100px, auto);
  }

  .bento-card {
    background: var(--bg-card);
    border: 1px solid var(--border);
    border-radius: var(--radius-lg);
    padding: var(--spacing-lg);
    display: flex;
    flex-direction: column;
    transition: border-color var(--transition-fast), box-shadow var(--transition-fast);
  }

  .bento-card:hover {
    border-color: var(--accent);
    box-shadow: var(--shadow-sm);
  }

  .bento-card:focus-visible {
    outline: 2px solid var(--accent);
    outline-offset: 2px;
    border-color: var(--accent);
  }

  .card-header {
    font-weight: 600;
    font-size: var(--font-size-body-md);
    margin-bottom: var(--spacing-md);
    color: var(--text-primary);
  }

  .card-icon {
    font-size: 24px;
    color: var(--accent);
    margin-bottom: var(--spacing-sm);
  }

  .card-value {
    font-size: 32px;
    font-weight: 700;
    line-height: 1.1;
  }

  .card-label {
    font-size: var(--font-size-sm);
    color: var(--text-secondary);
    margin-top: var(--spacing-xs);
  }

  .card-summary { grid-column: span 1; }
  .card-active { grid-column: span 1; }
  .card-types { grid-column: span 1; }
  .card-quick-actions { grid-column: span 1; }

  .card-distribution { grid-column: span 2; grid-row: span 1; }

  .card-recent { grid-column: span 2; grid-row: span 1; }

  /* ===== Skeleton Bento ===== */
  .skeleton-block {
    background: var(--border);
    border-radius: var(--radius-sm);
    height: 14px;
  }

  .skeleton-block-xs {
    width: 40px;
    height: 14px;
  }

  .skeleton-block-sm {
    width: 80px;
    height: 14px;
  }

  .skeleton-block-md {
    width: 60%;
    height: 14px;
  }

  .skeleton-block-lg {
    width: 100%;
    height: 14px;
  }

  .skeleton-block-xl {
    width: 100%;
    height: 28px;
  }

  .skeleton-stat {
    gap: var(--spacing-sm);
  }

  .skeleton-distribution,
  .skeleton-recent {
    gap: var(--spacing-sm);
  }

  /* ===== Quick Actions ===== */
  .action-buttons {
    display: grid;
    grid-template-columns: 1fr 1fr;
    gap: var(--spacing-sm);
    flex: 1;
  }

  .action-btn {
    display: flex;
    flex-direction: column;
    align-items: center;
    justify-content: center;
    gap: var(--spacing-xs);
    padding: var(--spacing-md);
    border-radius: var(--radius-md);
    background: var(--color-surface-container-low);
    border: 1px solid transparent;
    transition: border-color var(--transition-fast), background var(--transition-fast);
    font-size: var(--font-size-sm);
    color: var(--text-primary);
  }

  .action-btn:hover {
    border-color: var(--accent);
    background: var(--bg-secondary);
  }

  .action-btn:focus-visible {
    outline: 2px solid var(--accent);
    outline-offset: 2px;
    border-color: var(--accent);
  }

  .action-btn .material-symbols-outlined {
    font-size: 22px;
    color: var(--accent);
  }

  /* ===== Distribution ===== */
  .distribution-bars {
    display: flex;
    flex-direction: column;
    gap: var(--spacing-sm);
    flex: 1;
  }

  .dist-row {
    display: flex;
    align-items: center;
    gap: var(--spacing-sm);
    font-size: var(--font-size-body-sm);
  }

  .dist-type {
    width: 100px;
    font-weight: 500;
    text-overflow: ellipsis;
    overflow: hidden;
    white-space: nowrap;
  }

  .dist-bar-track {
    flex: 1;
    height: 8px;
    background: var(--color-surface-container-high);
    border-radius: 4px;
    overflow: hidden;
  }

  .dist-bar-fill {
    height: 100%;
    border-radius: 4px;
    transition: width var(--transition-normal);
    min-width: 2px;
  }

  .dist-count {
    width: 40px;
    text-align: right;
    font-variant-numeric: tabular-nums;
    color: var(--text-secondary);
  }

  /* ===== Recent ===== */
  .recent-list {
    display: flex;
    flex-direction: column;
    gap: 2px;
    flex: 1;
  }

  .recent-item {
    display: flex;
    align-items: center;
    gap: var(--spacing-md);
    padding: var(--spacing-sm) var(--spacing-md);
    border-radius: var(--radius-sm);
    transition: background var(--transition-fast);
    text-align: left;
    width: 100%;
    background: transparent;
    border: none;
    color: var(--text-primary);
    font-size: var(--font-size-body-sm);
  }

  .recent-item:hover {
    background: var(--bg-secondary);
  }

  .recent-item:focus-visible {
    outline: 2px solid var(--accent);
    outline-offset: -2px;
  }

  .recent-title {
    flex: 1;
    font-weight: 500;
  }

  .recent-date {
    font-size: var(--font-size-sm);
    flex-shrink: 0;
  }

  .empty-text {
    font-size: var(--font-size-sm);
  }
</style>
