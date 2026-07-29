<script lang="ts">
  import { getState } from "../lib/state.svelte.js";
  import { onMount } from "svelte";
  import { getTableView } from "../lib/api.js";
  import { navigateTo } from "../lib/router.svelte.js";
  import type { TableRow } from "../lib/types.js";

  const app = getState();

  let rows = $state<TableRow[]>([]);
  let loading = $state(true);
  let sortColumn = $state<keyof TableRow | null>(null);
  let sortDirection = $state<"asc" | "desc">("asc");
  let entityTypeFilter = $state("");
  let searchQuery = $state("");

  const columnDefs: Array<{ key: keyof TableRow; label: string }> = [
    { key: "entity_type", label: "Type" },
    { key: "title", label: "Title" },
    { key: "tags", label: "Tags" },
    { key: "created_at", label: "Created" },
    { key: "updated_at", label: "Updated" },
  ];

  onMount(async () => {
    await loadTable();
  });

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

  function handleSort(col: keyof TableRow) {
    if (sortColumn === col) {
      sortDirection = sortDirection === "asc" ? "desc" : "asc";
    } else {
      sortColumn = col;
      sortDirection = "asc";
    }
    loadTable();
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

  function renderHeaderLabel(key: keyof TableRow): string {
    const def = columnDefs.find((d) => d.key === key);
    return def?.label ?? key;
  }

  let sorted = $derived(getSortedRows());
</script>

<div class="table-view">
  <div class="table-header">
    <h2>Table View</h2>
    <div class="controls">
      <div class="search-wrapper">
        <span class="material-symbols-outlined search-icon">search</span>
        <input
          type="text"
          placeholder="Search rows..."
          bind:value={searchQuery}
          class="search-input"
        />
      </div>
      <select
        bind:value={entityTypeFilter}
        onchange={loadTable}
        class="filter-select"
      >
        <option value="">All types</option>
        {#each columnDefs as col}
          {#if col.key === "entity_type"}
            <!-- placeholder; real filter fetched server-side -->
          {/if}
        {/each}
      </select>
    </div>
  </div>

  {#if loading}
    <p class="text-muted">Loading...</p>
  {:else if rows.length === 0}
    <p class="text-muted">No entities found.</p>
  {:else}
    <div class="table-container">
      <table class="entity-table">
        <thead>
          <tr>
            {#each columnDefs as col}
              <th
                class="sortable"
                onclick={() => handleSort(col.key)}
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
          {#each sorted as row (row.entity_id)}
            <tr
              class="entity-row"
              ondblclick={() => selectEntity(row.entity_id)}
            >
              {#each columnDefs as col}
                <td>{renderValue(row[col.key])}</td>
              {/each}
              <td>
                <button
                  class="btn btn-ghost btn-small"
                  onclick={() => selectEntity(row.entity_id)}
                  title="View details"
                >
                  <span class="material-symbols-outlined">open_in_new</span>
                </button>
              </td>
            </tr>
          {/each}
        </tbody>
      </table>
    </div>

    <div class="table-footer text-muted">
      Showing {sorted.length} of {rows.length} entities
    </div>
  {/if}
</div>

<style>
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

  .filter-select {
    padding: var(--spacing-sm) var(--spacing-md);
    border: 1px solid var(--border);
    border-radius: var(--radius-sm);
    background: var(--bg-card);
    color: var(--text-primary);
    font-size: var(--font-size-body-sm);
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

  .actions-col {
    width: 60px;
    text-align: center;
  }

  .table-footer {
    margin-top: var(--spacing-sm);
    font-size: var(--font-size-sm);
  }
</style>
