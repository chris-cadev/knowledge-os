<script lang="ts">
  import { getState } from "../lib/state.svelte.js";
  import { onMount } from "svelte";
  import { getTableView, listEntities } from "../lib/api.js";
  import { navigateTo } from "../lib/router.svelte.js";
  import type { TableRow } from "../lib/types.js";
  import TypeBadge from "../components/TypeBadge.svelte";
  import TypeFilterDropdown from "../components/TypeFilterDropdown.svelte";
  import SkeletonLoader from "../components/SkeletonLoader.svelte";

  const app = getState();

  let rows = $state<TableRow[]>([]);
  let loading = $state(true);
  let sortColumn = $state<keyof TableRow | null>(null);
  let sortDirection = $state<"asc" | "desc">("asc");
  let entityTypeFilter = $state("");
  let searchQuery = $state("");
  let typeOptions = $state<string[]>([]);
  let focusedRowIndex = $state(-1);

  let containerEl: HTMLDivElement;

  const columnDefs: Array<{ key: keyof TableRow; label: string }> = [
    { key: "entity_type", label: "Type" },
    { key: "title", label: "Title" },
    { key: "tags", label: "Tags" },
    { key: "created_at", label: "Created" },
    { key: "updated_at", label: "Updated" },
  ];

  onMount(async () => {
    await loadTypeOptions();
    await loadTable();
  });

  async function loadTypeOptions() {
    try {
      const entities = await listEntities();
      typeOptions = [...new Set(entities.map((e) => e.entity_type))].sort();
    } catch {
      typeOptions = [];
    }
  }

  async function loadTable() {
    loading = true;
    try {
      const data = await getTableView(sortColumn ?? undefined, entityTypeFilter || undefined);
      rows = data.rows;
    } catch (e) {
      app.statusMessage = `Failed to load table: ${e}`;
    } finally {
      loading = false;
    }
  }

  function handleFilterChange(value: string) {
    entityTypeFilter = value;
    loadTable();
  }

  function handleSort(col: keyof TableRow) {
    if (sortColumn === col) {
      sortDirection = sortDirection === "asc" ? "desc" : "asc";
    } else {
      sortColumn = col;
      sortDirection = "asc";
    }
    loadTable();
  }

  function getAriaSort(col: keyof TableRow): string | undefined {
    if (sortColumn !== col) return undefined;
    return sortDirection === "asc" ? "ascending" : "descending";
  }

  function getSortedRows(): TableRow[] {
    let filtered = rows;
    if (searchQuery) {
      const q = searchQuery.toLowerCase();
      filtered = rows.filter((row) =>
        Object.values(row).some((v) =>
          String(v ?? "").toLowerCase().includes(q)
        )
      );
    }
    return filtered;
  }

  function selectEntity(id: string) {
    navigateTo("detail", id);
  }

  function renderValue(val: unknown): string {
    if (val === null || val === undefined) return "—";
    if (Array.isArray(val)) return val.join(", ");
    if (typeof val === "object") return JSON.stringify(val);
    return String(val);
  }

  function handleKeydown(e: KeyboardEvent) {
    const sortedRows = sorted;
    if (sortedRows.length === 0) return;

    if (e.key === "ArrowDown") {
      e.preventDefault();
      focusedRowIndex = Math.min(focusedRowIndex + 1, sortedRows.length - 1);
      focusCurrentRow();
    } else if (e.key === "ArrowUp") {
      e.preventDefault();
      focusedRowIndex = Math.max(focusedRowIndex - 1, 0);
      focusCurrentRow();
    } else if (e.key === "Enter" && focusedRowIndex >= 0 && focusedRowIndex < sortedRows.length) {
      e.preventDefault();
      selectEntity(sortedRows[focusedRowIndex].entity_id);
    } else if (e.key === "Escape") {
      focusedRowIndex = -1;
    }
  }

  function focusCurrentRow() {
    const rowEls = containerEl?.querySelectorAll(".entity-row");
    if (rowEls && focusedRowIndex >= 0 && focusedRowIndex < rowEls.length) {
      (rowEls[focusedRowIndex] as HTMLElement).focus();
    }
  }

  function handleRetry() {
    loadTable();
  }

  let sorted = $derived(getSortedRows());
</script>

<div
  class="table-view"
  bind:this={containerEl}
  tabindex="0"
  role="region"
  aria-label="Table view"
  onkeydown={handleKeydown}
>
  <div class="table-header">
    <h2>Table View <span class="entity-count text-muted">({rows.length})</span></h2>
    <div class="controls">
      <div class="search-wrapper">
        <label for="table-search" class="sr-only">Search rows</label>
        <span class="material-symbols-outlined search-icon">search</span>
        <input
          id="table-search"
          type="text"
          placeholder="Search rows..."
          bind:value={searchQuery}
          class="search-input"
        />
      </div>
      <TypeFilterDropdown
        value={entityTypeFilter}
        options={typeOptions}
        onchange={handleFilterChange}
      />
    </div>
  </div>

  {#if loading}
    <SkeletonLoader variant="table" count={8} />
  {:else if rows.length === 0}
    <div class="empty-state">
      <p class="text-muted">No entities found.</p>
      <button class="btn btn-primary" onclick={handleRetry}>Retry</button>
    </div>
  {:else}
    <div class="table-container">
      <table class="entity-table" role="table">
        <thead>
          <tr>
            {#each columnDefs as col}
              <th
                class="sortable"
                role="columnheader"
                aria-sort={getAriaSort(col.key)}
                onclick={() => handleSort(col.key)}
                onkeydown={(e) => { if (e.key === "Enter" || e.key === " ") { e.preventDefault(); handleSort(col.key); } }}
                tabindex="0"
              >
                <span>{col.label}</span>
                {#if sortColumn === col.key}
                  <span class="material-symbols-outlined sort-icon">
                    {sortDirection === "asc" ? "arrow_upward" : "arrow_downward"}
                  </span>
                {/if}
              </th>
            {/each}
            <th class="actions-col">Actions</th>
          </tr>
        </thead>
        <tbody>
          {#each sorted as row, i (row.entity_id)}
            <tr
              class="entity-row"
              class:focused={focusedRowIndex === i}
              role="row"
              tabindex="-1"
              onclick={() => selectEntity(row.entity_id)}
              onkeydown={(e) => { if (e.key === "Enter") { e.preventDefault(); selectEntity(row.entity_id); } }}
            >
              {#each columnDefs as col}
                <td>
                  {#if col.key === "entity_type"}
                    <TypeBadge type={row.entity_type} />
                  {:else}
                    {renderValue(row[col.key])}
                  {/if}
                </td>
              {/each}
              <td>
                <button
                  class="btn btn-ghost btn-small"
                  onclick={(e) => { e.stopPropagation(); selectEntity(row.entity_id); }}
                  title="View details"
                  aria-label="View details for {row.title}"
                >
                  <span class="material-symbols-outlined">open_in_new</span>
                </button>
              </td>
            </tr>
          {/each}
        </tbody>
      </table>
    </div>

    <div class="table-footer text-muted" role="status" aria-live="polite">
      Showing {sorted.length} of {rows.length} entities
    </div>
  {/if}
</div>

<style>
  .table-view {
    outline: none;
  }

  .table-view:focus-visible {
    outline: 2px solid var(--accent);
    outline-offset: 2px;
    border-radius: var(--radius-sm);
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

  .entity-count {
    font-size: var(--font-size-body-sm);
    font-weight: normal;
  }

  .table-header {
    display: flex;
    align-items: center;
    justify-content: space-between;
    margin-bottom: var(--spacing-lg);
    flex-wrap: wrap;
    gap: var(--spacing-md);
  }

  .table-header h2 {
    font-size: var(--font-size-2xl);
  }

  .controls {
    display: flex;
    gap: var(--spacing-md);
    align-items: center;
  }

  .search-wrapper {
    position: relative;
    display: flex;
    align-items: center;
  }

  .search-icon {
    position: absolute;
    left: var(--spacing-sm);
    font-size: 18px;
    color: var(--text-secondary);
    pointer-events: none;
  }

  .search-input {
    padding: var(--spacing-sm) var(--spacing-md) var(--spacing-sm) calc(var(--spacing-md) + 20px);
    border: 1px solid var(--border);
    border-radius: var(--radius-sm);
    background: var(--bg-card);
    color: var(--text-primary);
    font-size: var(--font-size-body-sm);
    width: 220px;
  }

  .search-input:focus {
    outline: none;
    border-color: var(--accent);
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
    border-radius: var(--radius-sm);
  }

  .sort-icon {
    font-size: 16px;
    vertical-align: middle;
    margin-left: 2px;
  }

  .entity-table td {
    padding: var(--spacing-sm) var(--spacing-md);
    border-bottom: 1px solid var(--border);
    max-width: 250px;
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }

  .entity-row {
    transition: background var(--transition-fast);
    cursor: pointer;
  }

  .entity-row:hover {
    background: var(--bg-secondary);
  }

  .entity-row:focus-visible {
    outline: 2px solid var(--accent);
    outline-offset: -2px;
    background: var(--bg-secondary);
  }

  .entity-row.focused {
    background: var(--bg-secondary);
  }

  .actions-col {
    width: 60px;
    text-align: center;
  }

  .empty-state {
    display: flex;
    flex-direction: column;
    align-items: center;
    gap: var(--spacing-md);
    padding: var(--spacing-2xl) 0;
  }

  .table-footer {
    margin-top: var(--spacing-sm);
    font-size: var(--font-size-sm);
  }
</style>
