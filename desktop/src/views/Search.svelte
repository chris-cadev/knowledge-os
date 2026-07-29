<script lang="ts">
  import { getState } from "../lib/state.svelte.js";
  import { searchEntities } from "../lib/api.js";
  import { navigateTo } from "../lib/router.svelte.js";
  import type { SearchResult } from "../lib/types.js";

  const app = getState();

  let query = $state("");
  let results = $state<SearchResult[]>([]);
  let loading = $state(false);
  let debounceTimer: ReturnType<typeof setTimeout> | null = null;
  let entityTypeFilter = $state("");
  let tagFilter = $state("");

  function onQueryInput(e: Event) {
    const value = (e.target as HTMLInputElement).value;
    query = value;

    if (debounceTimer) clearTimeout(debounceTimer);

    if (value.length < 2) {
      results = [];
      return;
    }

    debounceTimer = setTimeout(() => performSearch(), 300);
  }

  async function performSearch() {
    if (query.length < 2) return;

    loading = true;
    try {
      results = await searchEntities(
        query,
        entityTypeFilter || undefined,
        tagFilter || undefined
      );
    } catch (e) {
      app.statusMessage = `Search failed: ${e}`;
      results = [];
    } finally {
      loading = false;
    }
  }

  function selectEntity(id: string) {
    navigateTo("detail", id);
  }

  function handleKeyDown(e: KeyboardEvent) {
    if (e.key === "Enter") {
      performSearch();
    }
  }
</script>

<div class="search">
  <h2>Search</h2>
  <p class="text-muted">Full-text search across your knowledge graph.</p>

  <div class="search-bar">
    <span class="material-symbols-outlined search-icon">search</span>
    <input
      type="text"
      placeholder="Type to search (min. 2 characters)..."
      value={query}
      oninput={onQueryInput}
      onkeydown={handleKeyDown}
      class="search-input"
    />
    {#if loading}
      <span class="material-symbols-outlined loading-icon">sync</span>
    {/if}
  </div>

  <div class="filters">
    <input
      type="text"
      placeholder="Filter by type..."
      bind:value={entityTypeFilter}
      oninput={() => { if (query.length >= 2) performSearch(); }}
      class="filter-input"
    />
    <input
      type="text"
      placeholder="Filter by tag..."
      bind:value={tagFilter}
      oninput={() => { if (query.length >= 2) performSearch(); }}
      class="filter-input"
    />
  </div>

  <div class="results">
    {#if results.length === 0 && query.length >= 2 && !loading}
      <p class="text-muted">No results found.</p>
    {/if}

    {#each results as result}
      <button class="result-item" onclick={() => selectEntity(result.entity_id)}>
        <div class="result-header">
          <span class="type-badge">{result.entity_type}</span>
          <span class="result-title">{result.title}</span>
        </div>
        <div class="result-score">
          Score: {result.score.toFixed(2)}
        </div>
        {#if result.snippet}
          <p class="result-snippet">{result.snippet}</p>
        {/if}
      </button>
    {/each}
  </div>
</div>

<style>
  .search {
    max-width: 720px;
  }

  .search h2 {
    font-size: var(--font-size-2xl);
    margin-bottom: var(--spacing-sm);
  }

  .search-bar {
    display: flex;
    align-items: center;
    gap: var(--spacing-sm);
    margin-top: var(--spacing-lg);
    padding: var(--spacing-sm) var(--spacing-md);
    background: var(--bg-card);
    border: 1px solid var(--border);
    border-radius: var(--radius-md);
    transition: border-color var(--transition-fast);
  }

  .search-bar:focus-within {
    border-color: var(--accent);
  }

  .search-icon {
    color: var(--text-secondary);
    font-size: 20px;
    width: 20px;
    height: 20px;
  }

  .search-input {
    flex: 1;
    border: none;
    background: transparent;
    color: var(--text-primary);
    font-size: var(--font-size-body-md);
    outline: none;
    padding: var(--spacing-xs) 0;
  }

  .loading-icon {
    animation: spin 1s linear infinite;
    color: var(--accent);
    font-size: 20px;
    width: 20px;
    height: 20px;
  }

  @keyframes spin {
    from { transform: rotate(0deg); }
    to { transform: rotate(360deg); }
  }

  .filters {
    display: flex;
    gap: var(--spacing-md);
    margin-top: var(--spacing-md);
  }

  .filter-input {
    flex: 1;
    padding: var(--spacing-sm) var(--spacing-md);
    border: 1px solid var(--border);
    border-radius: var(--radius-sm);
    background: var(--bg-card);
    color: var(--text-primary);
    font-size: var(--font-size-body-sm);
  }

  .results {
    margin-top: var(--spacing-lg);
    display: flex;
    flex-direction: column;
    gap: var(--spacing-sm);
  }

  .result-item {
    display: flex;
    flex-direction: column;
    gap: var(--spacing-xs);
    padding: var(--spacing-md);
    background: var(--bg-card);
    border: 1px solid var(--border);
    border-radius: var(--radius-md);
    text-align: left;
    cursor: pointer;
    transition: background var(--transition-fast), border-color var(--transition-fast);
    width: 100%;
  }

  .result-item:hover {
    background: var(--bg-secondary);
    border-color: var(--accent);
  }

  .result-header {
    display: flex;
    align-items: center;
    gap: var(--spacing-sm);
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

  .result-title {
    font-weight: 600;
    font-size: var(--font-size-body-md);
  }

  .result-score {
    font-size: var(--font-size-sm);
    color: var(--text-secondary);
  }

  .result-snippet {
    font-size: var(--font-size-sm);
    color: var(--text-secondary);
    line-height: 1.4;
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }
</style>
