<script lang="ts">
  import { getState } from "../lib/state.svelte.js";
  import { onMount } from "svelte";
  import { listEntities } from "../lib/api.js";
  import { navigateTo } from "../lib/router.svelte.js";
  import type { EntitySummary } from "../lib/types.js";
  import TypeBadge from "../components/TypeBadge.svelte";
  import TypeFilterDropdown from "../components/TypeFilterDropdown.svelte";
  import SkeletonLoader from "../components/SkeletonLoader.svelte";

  const app = getState();

  let loading = $state(true);
  let error = $state<string | null>(null);
  let allEntities = $state<EntitySummary[]>([]);
  let typeFilter = $state("");
  let sortColumn = $state<keyof EntitySummary | null>(null);
  let sortDirection = $state<"asc" | "desc">("asc");

  const columnDefs: Array<{ key: keyof EntitySummary; label: string }> = [
    { key: "entity_type", label: "Type" },
    { key: "title", label: "Title" },
    { key: "created_at", label: "Created" },
    { key: "updated_at", label: "Updated" },
  ];

  let availableTypes = $derived(
    [...new Set(allEntities.map((e) => e.entity_type))].sort()
  );

  let filteredEntities = $derived(
    typeFilter
      ? allEntities.filter((e) => e.entity_type === typeFilter)
      : allEntities
  );

  let sortedEntities = $derived.by(() => {
    if (!sortColumn) return filteredEntities;
    const col = sortColumn;
    return [...filteredEntities].sort((a, b) => {
      const aVal = String(a[col] ?? "");
      const bVal = String(b[col] ?? "");
      const cmp = aVal.localeCompare(bVal);
      return sortDirection === "asc" ? cmp : -cmp;
    });
  });

  onMount(async () => {
    await loadEntities();
  });

  async function loadEntities() {
    loading = true;
    error = null;
    try {
      const entities = await listEntities();
      allEntities = entities;
      app.entities = entities;
      app.entityCount = entities.length;
    } catch (e) {
      error = `Failed to load entities: ${e}`;
      app.statusMessage = `Failed to load entities: ${e}`;
    } finally {
      loading = false;
    }
  }

  function handleTypeFilter(value: string) {
    typeFilter = value;
    app.entityTypeFilter = value || null;
  }

  function handleSort(col: keyof EntitySummary) {
    if (sortColumn === col) {
      sortDirection = sortDirection === "asc" ? "desc" : "asc";
    } else {
      sortColumn = col;
      sortDirection = "asc";
    }
  }

  function openEntity(id: string) {
    navigateTo("detail", id);
  }

  function handleRowKeydown(event: KeyboardEvent, id: string) {
    if (event.key === "Enter" || event.key === " ") {
      event.preventDefault();
      openEntity(id);
    }
    if (event.key === "ArrowDown") {
      event.preventDefault();
      const row = event.currentTarget as HTMLElement;
      const next = row.nextElementSibling as HTMLElement | null;
      next?.focus();
    }
    if (event.key === "ArrowUp") {
      event.preventDefault();
      const row = event.currentTarget as HTMLElement;
      const prev = row.previousElementSibling as HTMLElement | null;
      prev?.focus();
    }
  }

  function getSortIcon(col: keyof EntitySummary): string {
    if (sortColumn !== col) return "";
    return sortDirection === "asc" ? "arrow_upward" : "arrow_downward";
  }

  function getAriaSort(col: keyof EntitySummary): string | undefined {
    if (sortColumn !== col) return undefined;
    return sortDirection === "asc" ? "ascending" : "descending";
  }
</script>

<div class="browser">
  <div class="browser-header">
    <h2>
      Browse Entities
      {#if !loading && !error}
        <span class="entity-count">({sortedEntities.length})</span>
      {/if}
    </h2>
    <div class="controls">
      <label for="browser-type-filter" class="sr-only">Filter by entity type</label>
      <TypeFilterDropdown
        value={typeFilter}
        options={availableTypes}
        onchange={handleTypeFilter}
      />
    </div>
  </div>

  {#if loading}
    <SkeletonLoader variant="table" count={8} />
  {:else if error}
    <div class="error-state">
      <span class="material-symbols-outlined error-icon">error_outline</span>
      <p>{error}</p>
      <button class="btn btn-primary" onclick={loadEntities}>
        <span class="material-symbols-outlined">refresh</span>
        Retry
      </button>
    </div>
  {:else if sortedEntities.length === 0}
    <div class="empty-state">
      <span class="material-symbols-outlined empty-icon">inbox</span>
      {#if typeFilter}
        <p>No entities found matching type "{typeFilter}".</p>
        <button class="btn btn-ghost" onclick={() => handleTypeFilter("")}>
          Clear filter
        </button>
      {:else}
        <p>No entities found. Import some documents to get started.</p>
        <button class="btn btn-primary" onclick={() => navigateTo("import")}>
          <span class="material-symbols-outlined">file_upload</span>
          Import Documents
        </button>
      {/if}
    </div>
  {:else}
    <div class="table-container">
      <table class="entity-table" role="grid" aria-label="Entity browser">
        <thead>
          <tr>
            {#each columnDefs as col}
              <th
                class="sortable"
                onclick={() => handleSort(col.key)}
                onkeydown={(e) => { if (e.key === "Enter" || e.key === " ") { e.preventDefault(); handleSort(col.key); } }}
                role="columnheader"
                tabindex="0"
                aria-sort={getAriaSort(col.key)}
              >
                <span>{col.label}</span>
                {#if sortColumn === col.key}
                  <span class="material-symbols-outlined sort-icon">
                    {getSortIcon(col.key)}
                  </span>
                {/if}
              </th>
            {/each}
          </tr>
        </thead>
        <tbody>
          {#each sortedEntities as entity (entity.id)}
            <tr
              tabindex="0"
              role="button"
              class="entity-row"
              onclick={() => openEntity(entity.id)}
              onkeydown={(e) => handleRowKeydown(e, entity.id)}
              aria-label="View {entity.title}"
            >
              <td><TypeBadge type={entity.entity_type} /></td>
              <td class="truncate">{entity.title}</td>
              <td class="text-muted">{entity.created_at.slice(0, 10)}</td>
              <td class="text-muted">{entity.updated_at.slice(0, 10)}</td>
            </tr>
          {/each}
        </tbody>
      </table>
    </div>
  {/if}
</div>

<style>
  .browser-header {
    display: flex;
    align-items: center;
    justify-content: space-between;
    margin-bottom: var(--spacing-lg);
    flex-wrap: wrap;
    gap: var(--spacing-md);
  }

  .browser-header h2 {
    font-size: var(--font-size-2xl);
    display: flex;
    align-items: center;
    gap: var(--spacing-sm);
  }

  .entity-count {
    font-size: var(--font-size-body-md);
    font-weight: 400;
    color: var(--text-secondary);
  }

  .controls {
    display: flex;
    align-items: center;
    gap: var(--spacing-md);
  }

  .sr-only {
    position: absolute;
    width: 1px;
    height: 1px;
    padding: 0;
    margin: -1px;
    overflow: hidden;
    clip: rect(0, 0, 0, 0);
    white-space: nowrap;
    border-width: 0;
  }

  .table-container {
    overflow-x: auto;
    border: 1px solid var(--border);
    border-radius: var(--radius-md);
    background: var(--bg-card);
  }

  .entity-table {
    width: 100%;
    border-collapse: collapse;
    font-size: var(--font-size-body-sm);
  }

  .entity-table th {
    text-align: left;
    padding: var(--spacing-md);
    font-weight: 600;
    border-bottom: 2px solid var(--border);
    background: var(--bg-secondary);
    user-select: none;
  }

  .sortable {
    cursor: pointer;
    transition: color var(--transition-fast);
  }

  .sortable:hover {
    color: var(--accent);
  }

  .sortable:focus-visible {
    outline: 2px solid var(--accent);
    outline-offset: -2px;
  }

  .sort-icon {
    font-size: 16px;
    vertical-align: middle;
    margin-left: 2px;
  }

  .entity-table td {
    padding: var(--spacing-sm) var(--spacing-md);
    border-bottom: 1px solid var(--border);
  }

  .entity-row {
    cursor: pointer;
    transition: background var(--transition-fast);
  }

  .entity-row:hover {
    background: var(--bg-secondary);
  }

  .entity-row:focus-visible {
    outline: 2px solid var(--accent);
    outline-offset: -2px;
    background: var(--bg-secondary);
  }

  .truncate {
    max-width: 300px;
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }

  .error-state,
  .empty-state {
    display: flex;
    flex-direction: column;
    align-items: center;
    justify-content: center;
    padding: var(--spacing-xl) var(--spacing-lg);
    text-align: center;
    gap: var(--spacing-md);
    color: var(--text-secondary);
  }

  .error-icon {
    font-size: 40px;
    color: var(--color-error, #ef4444);
  }

  .empty-icon {
    font-size: 40px;
    color: var(--text-secondary);
  }

  .btn {
    display: inline-flex;
    align-items: center;
    gap: var(--spacing-xs);
    padding: var(--spacing-sm) var(--spacing-md);
    border-radius: var(--radius-sm);
    font-size: var(--font-size-body-sm);
    font-weight: 500;
    cursor: pointer;
    transition: background var(--transition-fast);
    border: none;
  }

  .btn-primary {
    background: var(--accent);
    color: white;
  }

  .btn-primary:hover {
    background: var(--accent-hover);
  }

  .btn-ghost {
    background: transparent;
    color: var(--text-primary);
    border: 1px solid var(--border);
  }

  .btn-ghost:hover {
    background: var(--bg-secondary);
  }
</style>
