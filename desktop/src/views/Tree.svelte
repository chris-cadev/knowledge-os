<script lang="ts">
  import { getState } from "../lib/state.svelte.js";
  import { onMount } from "svelte";
  import { getTreeView, listEntities } from "../lib/api.js";
  import { navigateTo } from "../lib/router.svelte.js";
  import type { TreeNode } from "../lib/types.js";
  import TypeBadge from "../components/TypeBadge.svelte";
  import TypeFilterDropdown from "../components/TypeFilterDropdown.svelte";
  import SkeletonLoader from "../components/SkeletonLoader.svelte";

  const app = getState();

  let roots = $state<TreeNode[]>([]);
  let loading = $state(true);
  let error = $state(false);
  let errorMsg = $state("");
  let expanded = $state<Set<string>>(new Set());
  let entityTypeFilter = $state("");
  let entityTypes = $state<string[]>([]);
  let focusedIndex = $state(-1);

  interface FlatNode {
    key: string;
    node: TreeNode;
    depth: number;
    isToggle: boolean;
    isEntity: boolean;
  }

  let flatNodes = $derived.by(() => {
    const result: FlatNode[] = [];
    for (const root of roots) {
      result.push({ key: `root:${root.label}`, node: root, depth: 0, isToggle: true, isEntity: false });
      if (expanded.has(root.label) && root.children) {
        for (const child of root.children) {
          const childKey = `${root.label}:${child.label}`;
          result.push({ key: childKey, node: child, depth: 1, isToggle: false, isEntity: true });
          if (hasChildren(child)) {
            result.push({ key: `nested:${childKey}`, node: child, depth: 1, isToggle: true, isEntity: false });
            if (expanded.has(child.label) && child.children) {
              for (const grandchild of child.children) {
                result.push({ key: `${childKey}:${grandchild.label}`, node: grandchild, depth: 2, isToggle: false, isEntity: true });
              }
            }
          }
        }
      }
    }
    return result;
  });

  let totalEntityCount = $derived(
    roots.reduce((sum, r) => sum + (r.count ?? r.children?.length ?? 0), 0)
  );

  onMount(async () => {
    await loadTypes();
    await loadTree();
  });

  async function loadTypes() {
    try {
      const entities = await listEntities();
      entityTypes = [...new Set(entities.map((e) => e.entity_type))].sort();
    } catch {
      entityTypes = [];
    }
  }

  async function loadTree() {
    loading = true;
    error = false;
    errorMsg = "";
    try {
      const data = await getTreeView(entityTypeFilter || undefined);
      roots = data.roots;
    } catch (e) {
      error = true;
      errorMsg = `Failed to load tree: ${e}`;
      app.statusMessage = `Failed to load tree: ${e}`;
    } finally {
      loading = false;
    }
  }

  function handleFilterChange(value: string) {
    entityTypeFilter = value;
    loadTree();
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

  function expandAll() {
    const next = new Set<string>();
    for (const root of roots) {
      next.add(root.label);
      if (root.children) {
        for (const child of root.children) {
          if (hasChildren(child)) {
            next.add(child.label);
          }
        }
      }
    }
    expanded = next;
  }

  function collapseAll() {
    expanded = new Set();
  }

  function selectEntity(id: string) {
    navigateTo("detail", id);
  }

  function hasChildren(node: TreeNode): boolean {
    return !!node.children && node.children.length > 0;
  }

  function getNestedLabel(node: TreeNode): string {
    if (!node.children || node.children.length === 0) return "";
    const types = [...new Set(node.children.map((c) => c.entity_type).filter(Boolean))];
    if (types.length === 1) return `${types[0]} (${node.children.length})`;
    if (types.length > 1) return `Related (${node.children.length})`;
    return `Sub-items (${node.children.length})`;
  }

  function handleKeydown(e: KeyboardEvent) {
    if (flatNodes.length === 0) return;

    switch (e.key) {
      case "ArrowDown":
        e.preventDefault();
        focusedIndex = Math.min(focusedIndex + 1, flatNodes.length - 1);
        break;
      case "ArrowUp":
        e.preventDefault();
        focusedIndex = Math.max(focusedIndex - 1, 0);
        break;
      case "ArrowRight": {
        e.preventDefault();
        const current = flatNodes[focusedIndex];
        if (current?.isToggle && !expanded.has(current.node.label)) {
          toggleExpand(current.node.label);
        }
        break;
      }
      case "ArrowLeft": {
        e.preventDefault();
        const current = flatNodes[focusedIndex];
        if (current?.isToggle && expanded.has(current.node.label)) {
          toggleExpand(current.node.label);
        }
        break;
      }
      case "Enter": {
        e.preventDefault();
        const current = flatNodes[focusedIndex];
        if (current?.isEntity && current.node.entity_id) {
          selectEntity(current.node.entity_id);
        } else if (current?.isToggle) {
          toggleExpand(current.node.label);
        }
        break;
      }
    }
  }
</script>

<div class="tree" role="tree" aria-label="Entity tree" onkeydown={handleKeydown}>
  <div class="tree-header">
    <h2>Tree View <span class="entity-count text-muted">({totalEntityCount})</span></h2>
    <div class="controls">
      <button class="ctrl-btn" onclick={expandAll} aria-label="Expand all">
        <span class="material-symbols-outlined icon-sm">unfold_more</span>
        <span>Expand All</span>
      </button>
      <button class="ctrl-btn" onclick={collapseAll} aria-label="Collapse all">
        <span class="material-symbols-outlined icon-sm">unfold_less</span>
        <span>Collapse All</span>
      </button>
      <TypeFilterDropdown
        value={entityTypeFilter}
        options={entityTypes}
        onchange={handleFilterChange}
      />
    </div>
  </div>

  {#if loading}
    <SkeletonLoader variant="list" count={6} />
  {:else if error}
    <div class="state-banner error">
      <span>{errorMsg || "Failed to load tree."}</span>
      <button class="retry-btn" onclick={loadTree}>Retry</button>
    </div>
  {:else if roots.length === 0}
    <p class="text-muted">No entities found. Import some documents to get started.</p>
  {:else}
    <div class="tree-container">
      {#each flatNodes as fn, i (fn.key)}
        {#if fn.isToggle && fn.depth === 0}
          <button
            class="node-header type-root"
            class:focused={focusedIndex === i}
            role="treeitem"
            aria-expanded={expanded.has(fn.node.label)}
            onclick={() => toggleExpand(fn.node.label)}
          >
            <span class="material-symbols-outlined chevron" class:expanded={expanded.has(fn.node.label)}>
              {expanded.has(fn.node.label) ? "expand_more" : "chevron_right"}
            </span>
            <TypeBadge type={fn.node.label} />
            {#if fn.node.count !== undefined}
              <span class="node-count text-muted">({fn.node.count})</span>
            {/if}
          </button>
        {:else if fn.isEntity && fn.depth === 1}
          <button
            class="node-header child-node"
            class:focused={focusedIndex === i}
            role="treeitem"
            onclick={() => selectEntity(fn.node.entity_id!)}
          >
            <span class="connector-line"></span>
            <span class="node-title">{fn.node.label}</span>
            {#if fn.node.entity_type}
              <TypeBadge type={fn.node.entity_type} />
            {/if}
          </button>
        {:else if fn.isToggle && fn.depth === 1}
          <button
            class="node-header nested-toggle"
            class:focused={focusedIndex === i}
            role="treeitem"
            aria-expanded={expanded.has(fn.node.label)}
            onclick={() => toggleExpand(fn.node.label)}
          >
            <span class="material-symbols-outlined chevron" class:expanded={expanded.has(fn.node.label)}>
              {expanded.has(fn.node.label) ? "expand_more" : "chevron_right"}
            </span>
            <span class="text-muted">{getNestedLabel(fn.node)}</span>
          </button>
        {:else if fn.isEntity && fn.depth === 2}
          <button
            class="node-header grandchild"
            class:focused={focusedIndex === i}
            role="treeitem"
            onclick={() => selectEntity(fn.node.entity_id!)}
          >
            <span class="connector-line double"></span>
            <span class="node-title">{fn.node.label}</span>
          </button>
        {/if}
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
    flex-wrap: wrap;
    gap: var(--spacing-md);
  }

  .tree-header h2 {
    font-size: var(--font-size-2xl);
  }

  .entity-count {
    font-size: var(--font-size-body-sm);
    font-weight: 400;
  }

  .controls {
    display: flex;
    gap: var(--spacing-md);
    align-items: center;
  }

  .ctrl-btn {
    display: inline-flex;
    align-items: center;
    gap: var(--spacing-xs);
    padding: var(--spacing-xs) var(--spacing-sm);
    border: 1px solid var(--border);
    border-radius: var(--radius-sm);
    background: var(--bg-card);
    color: var(--text-primary);
    font-size: var(--font-size-body-sm);
    cursor: pointer;
    transition: background var(--transition-fast), border-color var(--transition-fast);
  }

  .ctrl-btn:hover {
    background: var(--bg-secondary);
    border-color: var(--accent);
  }

  .icon-sm {
    font-size: 16px;
  }

  .tree-container {
    display: flex;
    flex-direction: column;
    gap: var(--spacing-xs);
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

  .node-header.focused {
    background: var(--bg-secondary);
    outline: 2px solid var(--accent);
    outline-offset: -2px;
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

  .chevron.expanded {
    color: var(--accent);
  }

  .node-count {
    font-size: var(--font-size-sm);
  }

  .child-node {
    margin-left: var(--spacing-lg);
    padding-left: var(--spacing-xs);
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

  .nested-toggle {
    margin-left: var(--spacing-lg);
    font-size: var(--font-size-sm);
    padding-left: var(--spacing-xl);
    color: var(--text-secondary);
  }

  .grandchild {
    margin-left: calc(var(--spacing-lg) + var(--spacing-md));
    padding-left: var(--spacing-xs);
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

  .retry-btn {
    padding: var(--spacing-xs) var(--spacing-sm);
    border: 1px solid var(--border);
    border-radius: var(--radius-sm);
    background: var(--bg-card);
    color: var(--text-primary);
    font-size: var(--font-size-body-sm);
    cursor: pointer;
  }

  .retry-btn:hover {
    border-color: var(--accent);
  }
</style>
