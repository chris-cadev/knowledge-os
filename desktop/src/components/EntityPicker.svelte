<script lang="ts">
  import { searchEntities } from "../lib/api.js";
  import { getEntityTypeColor } from "../lib/theme.svelte.js";
  import type { EntitySummary } from "../lib/types.js";

  let { onSelect, placeholder = "Search entities..." }: {
    onSelect: (entity: EntitySummary) => void;
    placeholder?: string;
  } = $props();

  let query = $state("");
  let results = $state<EntitySummary[]>([]);
  let open = $state(false);
  let selectedIndex = $state(-1);
  let searching = $state(false);
  let debounceTimer: ReturnType<typeof setTimeout> | null = null;
  let containerEl: HTMLDivElement | undefined = $state();
  let inputEl: HTMLInputElement | undefined = $state();
  let dropdownAbove = $state(false);

  function handleInput(e: Event) {
    query = (e.target as HTMLInputElement).value;
    selectedIndex = -1;

    if (debounceTimer) clearTimeout(debounceTimer);

    if (query.length < 2) {
      results = [];
      open = false;
      return;
    }

    searching = true;
    debounceTimer = setTimeout(() => performSearch(), 300);
  }

  async function performSearch() {
    if (query.length < 2) return;

    try {
      const searchResults = await searchEntities(query);
      results = searchResults.map((r) => ({
        id: r.entity_id,
        entity_type: r.entity_type,
        title: r.title,
        is_active: true,
        created_at: "",
        updated_at: "",
      }));
      open = results.length > 0;
      selectedIndex = results.length > 0 ? 0 : -1;
      updateDropdownPosition();
    } catch {
      results = [];
      open = false;
    } finally {
      searching = false;
    }
  }

  function handleKeyDown(e: KeyboardEvent) {
    if (!open) return;

    switch (e.key) {
      case "ArrowDown":
        e.preventDefault();
        selectedIndex = Math.min(selectedIndex + 1, results.length - 1);
        scrollSelectedIntoView();
        break;
      case "ArrowUp":
        e.preventDefault();
        selectedIndex = Math.max(selectedIndex - 1, 0);
        scrollSelectedIntoView();
        break;
      case "Enter":
        e.preventDefault();
        if (selectedIndex >= 0 && selectedIndex < results.length) {
          selectResult(results[selectedIndex]);
        }
        break;
      case "Escape":
        e.preventDefault();
        open = false;
        selectedIndex = -1;
        break;
    }
  }

  function selectResult(entity: EntitySummary) {
    open = false;
    query = "";
    results = [];
    selectedIndex = -1;
    onSelect(entity);
  }

  function scrollSelectedIntoView() {
    if (!containerEl) return;
    const selected = containerEl.querySelector(`[data-index="${selectedIndex}"]`);
    selected?.scrollIntoView({ block: "nearest" });
  }

  function updateDropdownPosition() {
    if (!containerEl || !inputEl) return;
    const rect = inputEl.getBoundingClientRect();
    const spaceBelow = window.innerHeight - rect.bottom;
    dropdownAbove = spaceBelow < 250;
  }

  function handleBlur(e: FocusEvent) {
    const related = e.relatedTarget as HTMLElement | null;
    if (containerEl && related && containerEl.contains(related)) return;
    setTimeout(() => {
      open = false;
      selectedIndex = -1;
    }, 150);
  }
</script>

<!-- svelte-ignore a11y_no_static_element_interactions -->
<div
  class="entity-picker"
  bind:this={containerEl}
  onkeydown={handleKeyDown}
>
  <div class="picker-input-wrapper">
    <span class="material-symbols-outlined picker-icon">search</span>
    <input
      bind:this={inputEl}
      type="text"
      {placeholder}
      value={query}
      oninput={handleInput}
      onfocus={() => { if (results.length > 0) { open = true; updateDropdownPosition(); } }}
      onblur={handleBlur}
      class="picker-input"
      role="combobox"
      aria-expanded={open}
      aria-autocomplete="list"
      aria-activedescendant={selectedIndex >= 0 ? `picker-option-${selectedIndex}` : undefined}
    />
    {#if searching}
      <span class="material-symbols-outlined picker-loading">sync</span>
    {/if}
  </div>

  {#if open && results.length > 0}
    <div class="picker-dropdown" class:above={dropdownAbove} role="listbox">
      {#each results as entity, i}
        <button
          data-index={i}
          id={`picker-option-${i}`}
          class="picker-option"
          class:selected={i === selectedIndex}
          role="option"
          aria-selected={i === selectedIndex}
          onmousedown|preventDefault={() => selectResult(entity)}
          onmouseenter={() => { selectedIndex = i; }}
        >
          <span class="picker-type-badge" style="background: {getEntityTypeColor(entity.entity_type)}">
            {entity.entity_type}
          </span>
          <span class="picker-title">{entity.title}</span>
        </button>
      {/each}
    </div>
  {/if}
</div>

<style>
  .entity-picker {
    position: relative;
    width: 100%;
  }

  .picker-input-wrapper {
    display: flex;
    align-items: center;
    gap: var(--spacing-sm);
    padding: var(--spacing-xs) var(--spacing-sm);
    border: 1px solid var(--border);
    border-radius: var(--radius-sm);
    background: var(--bg-card);
    transition: border-color var(--transition-fast);
  }

  .picker-input-wrapper:focus-within {
    border-color: var(--accent);
  }

  .picker-icon {
    color: var(--text-secondary);
    font-size: 18px;
    width: 18px;
    height: 18px;
    flex-shrink: 0;
  }

  .picker-input {
    flex: 1;
    border: none;
    background: transparent;
    color: var(--text-primary);
    font-size: var(--font-size-body-sm);
    outline: none;
    padding: var(--spacing-xs) 0;
    font-family: inherit;
  }

  .picker-input::placeholder {
    color: var(--text-tertiary, var(--color-on-surface-variant));
  }

  .picker-loading {
    color: var(--accent);
    font-size: 18px;
    width: 18px;
    height: 18px;
    animation: spin 1s linear infinite;
    flex-shrink: 0;
  }

  @keyframes spin {
    from { transform: rotate(0deg); }
    to { transform: rotate(360deg); }
  }

  .picker-dropdown {
    position: absolute;
    top: 100%;
    left: 0;
    right: 0;
    margin-top: 4px;
    max-height: 240px;
    overflow-y: auto;
    background: var(--bg-card);
    border: 1px solid var(--border);
    border-radius: var(--radius-md);
    box-shadow: var(--shadow-md);
    z-index: 50;
  }

  .picker-dropdown.above {
    top: auto;
    bottom: 100%;
    margin-top: 0;
    margin-bottom: 4px;
  }

  .picker-option {
    display: flex;
    align-items: center;
    gap: var(--spacing-sm);
    width: 100%;
    padding: var(--spacing-sm) var(--spacing-md);
    border: none;
    background: transparent;
    color: var(--text-primary);
    font-size: var(--font-size-body-sm);
    text-align: left;
    cursor: pointer;
    transition: background var(--transition-fast);
    font-family: inherit;
  }

  .picker-option:hover,
  .picker-option.selected {
    background: var(--bg-secondary);
  }

  .picker-option:focus {
    outline: none;
    background: var(--bg-secondary);
  }

  .picker-type-badge {
    display: inline-block;
    padding: 1px 6px;
    color: white;
    border-radius: var(--radius-sm);
    font-size: 10px;
    font-weight: 500;
    flex-shrink: 0;
  }

  .picker-title {
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }
</style>
