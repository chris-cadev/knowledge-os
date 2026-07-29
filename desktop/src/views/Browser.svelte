<script lang="ts">
  import { getState } from "../lib/state.svelte.js";
  import { onMount } from "svelte";
  import { listEntities } from "../lib/api.js";
  import { navigateTo } from "../lib/router.svelte.js";

  const app = getState();

  let loading = $state(true);

  onMount(async () => {
    try {
      const entities = await listEntities(app.entityTypeFilter ?? undefined);
      app.entities = entities;
      app.entityCount = entities.length;
    } catch (e) {
      app.statusMessage = `Failed to load entities: ${e}`;
    } finally {
      loading = false;
    }
  });

  async function filterByType(type: string | null) {
    app.entityTypeFilter = type;
    loading = true;
    try {
      const entities = await listEntities(type ?? undefined);
      app.entities = entities;
    } finally {
      loading = false;
    }
  }

  function openEntity(id: string) {
    navigateTo("detail", id);
  }
</script>

<div class="browser">
  <div class="browser-header">
    <h2>Browse Entities</h2>
    <input
      type="text"
      placeholder="Filter by type..."
      oninput={(e) => filterByType((e.target as HTMLInputElement).value || null)}
      class="filter-input"
    />
  </div>

  {#if loading}
    <p>Loading...</p>
  {:else if app.entities.length === 0}
    <p class="text-muted">No entities found. Import some documents to get started.</p>
  {:else}
    <table class="entity-table">
      <thead>
        <tr>
          <th>Type</th>
          <th>Title</th>
          <th>Created</th>
          <th>Updated</th>
        </tr>
      </thead>
      <tbody>
        {#each app.entities as entity}
          <tr onclick={() => openEntity(entity.id)} class="entity-row">
            <td><span class="type-badge">{entity.entity_type}</span></td>
            <td class="truncate">{entity.title}</td>
            <td class="text-muted">{entity.created_at.slice(0, 10)}</td>
            <td class="text-muted">{entity.updated_at.slice(0, 10)}</td>
          </tr>
        {/each}
      </tbody>
    </table>
  {/if}
</div>

<style>
  .browser-header {
    display: flex;
    align-items: center;
    justify-content: space-between;
    margin-bottom: var(--spacing-lg);
  }

  .filter-input {
    padding: var(--spacing-sm) var(--spacing-md);
    border: 1px solid var(--border);
    border-radius: var(--radius-sm);
    background: var(--bg-card);
    color: var(--text-primary);
    width: 200px;
  }

  .entity-table {
    width: 100%;
    border-collapse: collapse;
  }

  .entity-table th {
    text-align: left;
    padding: var(--spacing-sm) var(--spacing-md);
    font-size: var(--font-size-sm);
    color: var(--text-secondary);
    border-bottom: 1px solid var(--border);
    text-transform: uppercase;
    letter-spacing: 0.05em;
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

  .type-badge {
    display: inline-block;
    padding: 2px 8px;
    background: var(--accent);
    color: white;
    border-radius: var(--radius-sm);
    font-size: var(--font-size-sm);
    font-weight: 500;
  }
</style>
