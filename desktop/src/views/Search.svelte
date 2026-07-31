<script lang="ts">
  import { onMount } from "svelte";
  import { getState } from "../lib/state.svelte.js";
  import { searchEntities, listEntities, getTableView } from "../lib/api.js";
  import { navigateTo } from "../lib/router.svelte.js";
  import type { SearchResult } from "../lib/types.js";
  import TypeBadge from "../components/TypeBadge.svelte";
  import TypeFilterDropdown from "../components/TypeFilterDropdown.svelte";

  const app = getState();

  type SearchMode = "keyword" | "semantic" | "hybrid";

  let query = $state("");
  let results = $state<SearchResult[]>([]);
  let loading = $state(false);
  let debounceTimer: ReturnType<typeof setTimeout> | null = null;
  let entityTypeFilter = $state("");
  let tagFilter = $state("");
  let searchMode = $state<SearchMode>("keyword");
  let selectedIndex = $state(-1);
  let typeOptions = $state<string[]>([]);
  let tagOptions = $state<string[]>([]);
  let tagDropdownOpen = $state(false);
  let tagSearch = $state("");
  let searchInput: HTMLInputElement;
  let resultsContainer: HTMLDivElement;

  onMount(async () => {
    try {
      const [entities, tableData] = await Promise.all([
        listEntities(),
        getTableView(),
      ]);
      typeOptions = [...new Set(entities.map((e) => e.entity_type))].sort();
      tagOptions = [
        ...new Set(tableData.rows.flatMap((r) => r.tags)),
      ].sort();
    } catch {
      // filters will remain empty
    }
  });

  const filteredTagOptions = $derived(
    tagSearch
      ? tagOptions.filter((t) =>
          t.toLowerCase().includes(tagSearch.toLowerCase())
        )
      : tagOptions
  );

  function onQueryInput(e: Event) {
    const value = (e.target as HTMLInputElement).value;
    query = value;
    selectedIndex = -1;

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
    selectedIndex = -1;
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

  function handleKeydown(e: KeyboardEvent) {
    if (e.key === "Enter") {
      e.preventDefault();
      if (selectedIndex >= 0 && selectedIndex < results.length) {
        selectEntity(results[selectedIndex].entity_id);
      } else {
        performSearch();
      }
      return;
    }

    if (e.key === "ArrowDown") {
      e.preventDefault();
      selectedIndex = Math.min(selectedIndex + 1, results.length - 1);
      scrollToSelected();
      return;
    }

    if (e.key === "ArrowUp") {
      e.preventDefault();
      selectedIndex = Math.max(selectedIndex - 1, 0);
      scrollToSelected();
      return;
    }

    if (e.key === "Escape") {
      selectedIndex = -1;
      return;
    }
  }

  function scrollToSelected() {
    requestAnimationFrame(() => {
      const el = resultsContainer?.querySelector(
        `[data-index="${selectedIndex}"]`
      );
      el?.scrollIntoView({ block: "nearest" });
    });
  }

  function handleTypeChange(value: string) {
    entityTypeFilter = value;
    if (query.length >= 2) performSearch();
  }

  function selectTag(tag: string) {
    tagFilter = tagFilter === tag ? "" : tag;
    tagDropdownOpen = false;
    tagSearch = "";
    if (query.length >= 2) performSearch();
  }

  function clearTag() {
    tagFilter = "";
    if (query.length >= 2) performSearch();
  }

  const searchModes: { value: SearchMode; label: string }[] = [
    { value: "keyword", label: "Keyword" },
    { value: "semantic", label: "Semantic" },
    { value: "hybrid", label: "Hybrid" },
  ];
</script>

<div class="search" role="search" aria-label="Search knowledge graph">
  <h2>Search</h2>
  <p class="text-muted">Full-text search across your knowledge graph.</p>

  <div class="search-bar">
    <span class="material-symbols-outlined search-icon">search</span>
    <input
      type="text"
      placeholder="Type to search (min. 2 characters)..."
      value={query}
      oninput={onQueryInput}
      onkeydown={handleKeydown}
      class="search-input"
      aria-label="Search query"
      bind:this={searchInput}
    />
    {#if loading}
      <span class="material-symbols-outlined loading-icon">sync</span>
    {/if}
  </div>

  <div class="controls">
    <div class="filters">
      <label class="sr-only" for="type-filter">Filter by type</label>
      <TypeFilterDropdown
        value={entityTypeFilter}
        options={typeOptions}
        onchange={handleTypeChange}
      />
      <div class="tag-filter-wrapper">
        <button
          type="button"
          class="tag-filter-trigger"
          onclick={() => (tagDropdownOpen = !tagDropdownOpen)}
          aria-expanded={tagDropdownOpen}
          aria-haspopup="listbox"
          aria-label="Filter by tag"
        >
          {#if tagFilter}
            <span class="tag-chip">
              {tagFilter}
              <span
                class="material-symbols-outlined tag-chip-close"
                role="button"
                tabindex="0"
                onclick={(e) => {
                  e.stopPropagation();
                  clearTag();
                }}
                onkeydown={(e) => {
                  if (e.key === "Enter" || e.key === " ") {
                    e.stopPropagation();
                    clearTag();
                  }
                }}
                aria-label="Clear tag filter"
              >close</span>
            </span>
          {:else}
            <span class="tag-filter-placeholder">All tags</span>
          {/if}
          <span class="material-symbols-outlined tag-filter-arrow"
            >expand_more</span
          >
        </button>
        {#if tagDropdownOpen}
          <div class="tag-dropdown" role="listbox" aria-label="Tag options">
            <input
              type="text"
              class="tag-search"
              placeholder="Search tags..."
              bind:value={tagSearch}
              aria-label="Search tags"
            />
            <div class="tag-options">
              {#each filteredTagOptions as tag}
                <button
                  type="button"
                  class="tag-option"
                  class:tag-option-selected={tagFilter === tag}
                  role="option"
                  aria-selected={tagFilter === tag}
                  onclick={() => selectTag(tag)}
                >
                  {tag}
                </button>
              {/each}
              {#if filteredTagOptions.length === 0}
                <p class="tag-empty text-muted">No tags found</p>
              {/if}
            </div>
          </div>
        {/if}
      </div>
    </div>

    <div class="mode-toggle" role="radiogroup" aria-label="Search mode">
      {#each searchModes as mode}
        <button
          type="button"
          class="mode-btn"
          class:mode-btn-active={searchMode === mode.value}
          role="radio"
          aria-checked={searchMode === mode.value}
          onclick={() => {
            searchMode = mode.value;
            if (query.length >= 2) performSearch();
          }}
        >
          {mode.label}
        </button>
      {/each}
    </div>
  </div>

  <div
    class="results"
    role="listbox"
    aria-label="Search results"
    aria-live="polite"
    bind:this={resultsContainer}
  >
    {#if results.length > 0 && query.length >= 2 && !loading}
      <p class="result-count text-muted" aria-atomic="true">
        Found {results.length} result{results.length === 1 ? "" : "s"}
      </p>
    {/if}

    {#if results.length === 0 && query.length >= 2 && !loading}
      <p class="text-muted">No results found. Try broadening your search.</p>
    {/if}

    {#each results as result, i}
      <button
        class="result-item"
        class:result-item-selected={selectedIndex === i}
        onclick={() => selectEntity(result.entity_id)}
        role="option"
        aria-selected={selectedIndex === i}
        data-index={i}
        tabindex="-1"
      >
        <div class="result-header">
          <TypeBadge type={result.entity_type} />
          <span class="result-title">{result.title}</span>
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

  .sr-only {
    position: absolute;
    width: 1px;
    height: 1px;
    padding: 0;
    margin: -1px;
    overflow: hidden;
    clip: rect(0, 0, 0, 0);
    white-space: nowrap;
    border: 0;
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

  .controls {
    display: flex;
    align-items: flex-start;
    justify-content: space-between;
    gap: var(--spacing-md);
    margin-top: var(--spacing-md);
    flex-wrap: wrap;
  }

  .filters {
    display: flex;
    gap: var(--spacing-md);
    flex: 1;
  }

  .tag-filter-wrapper {
    position: relative;
  }

  .tag-filter-trigger {
    display: flex;
    align-items: center;
    gap: var(--spacing-xs);
    padding: var(--spacing-sm) var(--spacing-md);
    border: 1px solid var(--border);
    border-radius: var(--radius-sm);
    background: var(--bg-card);
    color: var(--text-primary);
    font-size: var(--font-size-body-sm);
    cursor: pointer;
    min-width: 140px;
    transition: border-color var(--transition-fast);
  }

  .tag-filter-trigger:hover,
  .tag-filter-trigger:focus {
    border-color: var(--accent);
    outline: none;
  }

  .tag-filter-placeholder {
    color: var(--text-secondary);
    flex: 1;
    text-align: left;
  }

  .tag-filter-arrow {
    font-size: 18px;
    color: var(--text-secondary);
    margin-left: auto;
  }

  .tag-chip {
    display: inline-flex;
    align-items: center;
    gap: var(--spacing-xs);
    background: var(--bg-secondary);
    padding: 2px 6px;
    border-radius: var(--radius-sm);
    font-size: var(--font-size-sm);
  }

  .tag-chip-close {
    font-size: 14px;
    cursor: pointer;
    color: var(--text-secondary);
  }

  .tag-chip-close:hover {
    color: var(--text-primary);
  }

  .tag-dropdown {
    position: absolute;
    top: 100%;
    left: 0;
    right: 0;
    margin-top: var(--spacing-xs);
    background: var(--bg-card);
    border: 1px solid var(--border);
    border-radius: var(--radius-sm);
    z-index: 10;
    box-shadow: 0 4px 12px rgba(0, 0, 0, 0.1);
    min-width: 200px;
  }

  .tag-search {
    width: 100%;
    padding: var(--spacing-sm) var(--spacing-md);
    border: none;
    border-bottom: 1px solid var(--border);
    background: transparent;
    color: var(--text-primary);
    font-size: var(--font-size-body-sm);
    outline: none;
  }

  .tag-options {
    max-height: 200px;
    overflow-y: auto;
    padding: var(--spacing-xs);
  }

  .tag-option {
    display: block;
    width: 100%;
    padding: var(--spacing-xs) var(--spacing-sm);
    border: none;
    background: transparent;
    color: var(--text-primary);
    font-size: var(--font-size-body-sm);
    cursor: pointer;
    text-align: left;
    border-radius: var(--radius-xs);
  }

  .tag-option:hover {
    background: var(--bg-secondary);
  }

  .tag-option-selected {
    background: var(--bg-secondary);
    font-weight: 500;
  }

  .tag-empty {
    padding: var(--spacing-sm) var(--spacing-md);
    font-size: var(--font-size-body-sm);
    text-align: center;
  }

  .mode-toggle {
    display: flex;
    border: 1px solid var(--border);
    border-radius: var(--radius-sm);
    overflow: hidden;
  }

  .mode-btn {
    padding: var(--spacing-sm) var(--spacing-md);
    border: none;
    background: var(--bg-card);
    color: var(--text-secondary);
    font-size: var(--font-size-body-sm);
    cursor: pointer;
    transition: background var(--transition-fast), color var(--transition-fast);
  }

  .mode-btn:not(:last-child) {
    border-right: 1px solid var(--border);
  }

  .mode-btn:hover {
    background: var(--bg-secondary);
  }

  .mode-btn-active {
    background: var(--accent);
    color: white;
  }

  .results {
    margin-top: var(--spacing-lg);
    display: flex;
    flex-direction: column;
    gap: var(--spacing-sm);
  }

  .result-count {
    font-size: var(--font-size-body-sm);
    padding-bottom: var(--spacing-xs);
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

  .result-item:hover,
  .result-item:focus-visible {
    background: var(--bg-secondary);
    border-color: var(--accent);
    outline: none;
  }

  .result-item-selected {
    background: var(--bg-secondary);
    border-color: var(--accent);
  }

  .result-header {
    display: flex;
    align-items: center;
    gap: var(--spacing-sm);
  }

  .result-title {
    font-weight: 600;
    font-size: var(--font-size-body-md);
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
