<script lang="ts">
  import { getState } from "../lib/state.svelte.js";
  import { onMount } from "svelte";
  import { getTreeView } from "../lib/api.js";
  import { navigateTo } from "../lib/router.svelte.js";
  import type { TreeNode } from "../lib/types.js";

  const app = getState();

  let roots = $state<TreeNode[]>([]);
  let loading = $state(true);
  let expanded = $state<Set<string>>(new Set());
  let entityTypeFilter = $state("");

  onMount(async () => {
    await loadTree();
  });

  async function loadTree() {
    loading = true;
    try {
      const data = await getTreeView(entityTypeFilter || undefined);
      roots = data.roots;
    } catch (e) {
      app.statusMessage = `Failed to load tree: ${e}`;
    } finally {
      loading = false;
    }
  }

  function toggleExpand(label: string) {
    const next = new Set(expanded);
    if (next.has(label)) {
      next.delete(label);
    } else {
      next.add(label);
    }
    expanded = next;
  }

  function selectEntity(id: string) {
    navigateTo("detail", id);
  }

  function hasChildren(node: TreeNode): boolean {
    return !!node.children && node.children.length > 0;
  }
</script>

<div class="tree">
  <div class="tree-header">
    <h2>Tree View</h2>
    <div class="controls">
      <input
        type="text"
        placeholder="Filter by type..."
        bind:value={entityTypeFilter}
        oninput={loadTree}
        class="filter-input"
      />
    </div>
  </div>

  {#if loading}
    <p class="text-muted">Loading...</p>
  {:else if roots.length === 0}
    <p class="text-muted">No entities found.</p>
  {:else}
    <div class="tree-container">
      {#each roots as root}
        <div class="tree-node">
          <button
            class="node-header type-root"
            onclick={() => toggleExpand(root.label)}
          >
            <span class="material-symbols-outlined chevron" class:expanded={expanded.has(root.label)}>
              {expanded.has(root.label) ? "expand_more" : "chevron_right"}
            </span>
            <span class="type-badge">{root.label}</span>
            {#if root.count !== undefined}
              <span class="node-count text-muted">({root.count})</span>
            {/if}
          </button>

          {#if expanded.has(root.label) && root.children}
            <div class="children">
              {#each root.children as child}
                <div class="tree-node child">
                  <button
                    class="node-header"
                    onclick={() => selectEntity(child.entity_id!)}
                  >
                    <span class="connector-line"></span>
                    <span class="node-title">{child.label}</span>
                    {#if child.entity_type}
                      <span class="entity-type-tag">{child.entity_type}</span>
                    {/if}
                  </button>

                  {#if hasChildren(child)}
                    <button
                      class="node-header nested-toggle"
                      onclick={() => toggleExpand(child.label)}
                    >
                      <span class="material-symbols-outlined chevron" class:expanded={expanded.has(child.label)}>
                        {expanded.has(child.label) ? "expand_more" : "chevron_right"}
                      </span>
                      <span class="text-muted">Sub-items ({child.children?.length})</span>
                    </button>

                    {#if expanded.has(child.label) && child.children}
                      <div class="children nested">
                        {#each child.children as grandchild}
                          <button
                            class="node-header grandchild"
                            onclick={() => selectEntity(grandchild.entity_id!)}
                          >
                            <span class="connector-line double"></span>
                            <span class="node-title">{grandchild.label}</span>
                          </button>
                        {/each}
                      </div>
                    {/if}
                  {/if}
                </div>
              {/each}
            </div>
          {/if}
        </div>
      {/each}
    </div>
  {/if}
</div>

<style>
  .tree-header {
    display: flex;
    align-items: center;
    justify-content: space-between;
    margin-bottom: var(--spacing-lg);
  }

  .tree-header h2 {
    font-size: var(--font-size-2xl);
  }

  .controls {
    display: flex;
    gap: var(--spacing-md);
  }

  .filter-input {
    padding: var(--spacing-sm) var(--spacing-md);
    border: 1px solid var(--border);
    border-radius: var(--radius-sm);
    background: var(--bg-card);
    color: var(--text-primary);
    font-size: var(--font-size-body-sm);
    width: 200px;
  }

  .tree-container {
    display: flex;
    flex-direction: column;
    gap: var(--spacing-xs);
  }

  .tree-node {
    display: flex;
    flex-direction: column;
  }

  .node-header {
    display: flex;
    align-items: center;
    gap: var(--spacing-sm);
    padding: var(--spacing-sm) var(--spacing-md);
    border-radius: var(--radius-sm);
    cursor: pointer;
    transition: background var(--transition-fast);
    text-align: left;
    width: 100%;
    font-size: var(--font-size-body-sm);
    background: transparent;
    border: none;
    color: var(--text-primary);
  }

  .node-header:hover {
    background: var(--bg-secondary);
  }

  .type-root {
    font-weight: 600;
  }

  .chevron {
    font-size: 18px;
    width: 18px;
    height: 18px;
    transition: transform var(--transition-fast);
    color: var(--text-secondary);
  }

  .expanded {
    color: var(--accent);
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

  .node-count {
    font-size: var(--font-size-sm);
  }

  .children {
    margin-left: var(--spacing-lg);
    border-left: 1px solid var(--border);
    padding-left: var(--spacing-xs);
  }

  .children.nested {
    margin-left: var(--spacing-md);
  }

  .connector-line {
    display: inline-block;
    width: 12px;
    height: 1px;
    background: var(--border);
    margin-right: var(--spacing-xs);
  }

  .connector-line.double {
    width: 20px;
  }

  .node-title {
    font-weight: 500;
  }

  .entity-type-tag {
    font-size: 10px;
    color: var(--text-secondary);
    font-family: var(--font-mono);
    background: var(--bg-card);
    padding: 1px 4px;
    border-radius: 2px;
  }

  .nested-toggle {
    font-size: var(--font-size-sm);
    padding-left: var(--spacing-xl);
    color: var(--text-secondary);
  }

  .grandchild {
    padding-left: var(--spacing-xl);
  }
</style>
