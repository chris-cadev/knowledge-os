<script lang="ts">
  import { getState } from "../lib/state.svelte.js";
  import { onMount } from "svelte";
  import {
    getEntityDetail,
    getEntitySource,
    openInDefaultApp,
    openSourceFolder,
  } from "../lib/api.js";
  import { navigateTo } from "../lib/router.svelte.js";
  import type { EntityDetail, ComponentData } from "../lib/types.js";

  const app = getState();

  let loading = $state(true);
  let sourcePath = $state<string | null>(null);
  let errorMsg = $state<string | null>(null);
  let sourceActionLoading = $state<string | null>(null);

  onMount(async () => {
    if (!app.selectedEntityId) {
      loading = false;
      return;
    }

    loading = true;
    errorMsg = null;

    try {
      const [detail, source] = await Promise.all([
        getEntityDetail(app.selectedEntityId),
        getEntitySource(app.selectedEntityId),
      ]);
      app.selectedEntityDetail = detail;
      sourcePath = source;
    } catch (e) {
      errorMsg = `Failed to load entity: ${e}`;
      app.statusMessage = errorMsg;
    } finally {
      loading = false;
    }
  });

  function close() {
    app.selectedEntityId = null;
    app.selectedEntityDetail = null;
    navigateTo("browse");
  }

  function viewInGraph() {
    if (app.selectedEntityId) {
      navigateTo("graph", app.selectedEntityId);
    }
  }

  async function handleOpenFile() {
    if (!sourcePath || sourceActionLoading) return;

    sourceActionLoading = "open";
    app.statusMessage = `Opening ${sourcePath}...`;

    try {
      await openInDefaultApp(sourcePath);
      app.statusMessage = `Opened ${sourcePath}`;
    } catch (e) {
      const message = `Failed to open file: ${e}`;
      app.statusMessage = message;
      errorMsg = message;
    } finally {
      sourceActionLoading = null;
    }
  }

  async function handleShowInFolder() {
    if (!sourcePath || sourceActionLoading) return;

    sourceActionLoading = "folder";
    app.statusMessage = `Opening folder for ${sourcePath}...`;

    try {
      await openSourceFolder(sourcePath);
      app.statusMessage = `Revealed ${sourcePath} in folder`;
    } catch (e) {
      const message = `Failed to open folder: ${e}`;
      app.statusMessage = message;
      errorMsg = message;
    } finally {
      sourceActionLoading = null;
    }
  }

  function selectEntity(id: string) {
    app.selectedEntityId = id;
    // Re-trigger the onMount by navigating
    navigateTo("detail", id);
  }

  function formatComponentValue(comp: ComponentData): string {
    if (typeof comp.data === "string") return comp.data;
    if (comp.component_type === "Tags" && Array.isArray(comp.data)) {
      return comp.data.join(", ");
    }
    return JSON.stringify(comp.data, null, 2);
  }

  function isContentComponent(comp: ComponentData): boolean {
    return comp.component_type === "Content";
  }
</script>

<div class="detail">
  <!-- Header -->
  <div class="detail-header">
    <button class="close-btn" onclick={close} aria-label="Close detail panel">
      <span class="material-symbols-outlined">close</span>
    </button>
    <h2>Entity Detail</h2>
  </div>

  {#if loading}
    <div class="loading-state">
      <span class="material-symbols-outlined spinning">sync</span>
      <p>Loading entity...</p>
    </div>
  {:else if errorMsg}
    <div class="error-state">
      <span class="material-symbols-outlined error-icon">error</span>
      <p>{errorMsg}</p>
      <button class="btn btn-primary" onclick={close}>Go Back</button>
    </div>
  {:else if app.selectedEntityDetail}
    {@const detail = app.selectedEntityDetail}

    <!-- Entity Header -->
    <div class="entity-header">
      <span class="type-badge">{detail.entity_type}</span>
      <h3 class="entity-id">{detail.id.slice(0, 8)}...</h3>
      <div class="status-indicator" class:active={detail.is_active}>
        {detail.is_active ? "Active" : "Archived"}
      </div>
    </div>

    <div class="entity-meta">
      <span class="text-muted">Created: {detail.created_at.slice(0, 10)}</span>
      <span class="text-muted">Updated: {detail.updated_at.slice(0, 10)}</span>
    </div>

    <!-- Source File Actions -->
    {#if sourcePath}
      <div class="source-actions">
        <span class="source-path text-muted" title={sourcePath}>
          <span class="material-symbols-outlined">link</span>
          {sourcePath}
        </span>
        <div class="action-buttons">
          <button
            class="btn btn-small"
            class:btn-loading={sourceActionLoading === "open"}
            onclick={handleOpenFile}
            disabled={sourceActionLoading !== null}
            aria-busy={sourceActionLoading === "open"}
          >
            {#if sourceActionLoading === "open"}
              <span class="material-symbols-outlined spinning">sync</span>
            {:else}
              <span class="material-symbols-outlined">open_in_new</span>
            {/if}
            Open File
          </button>
          <button
            class="btn btn-small"
            class:btn-loading={sourceActionLoading === "folder"}
            onclick={handleShowInFolder}
            disabled={sourceActionLoading !== null}
            aria-busy={sourceActionLoading === "folder"}
          >
            {#if sourceActionLoading === "folder"}
              <span class="material-symbols-outlined spinning">sync</span>
            {:else}
              <span class="material-symbols-outlined">folder_open</span>
            {/if}
            Show in Folder
          </button>
          <button
            class="btn btn-small"
            onclick={viewInGraph}
            disabled={sourceActionLoading !== null}
          >
            <span class="material-symbols-outlined">bubble_chart</span>
            View in Graph
          </button>
        </div>
      </div>
    {:else}
      <div class="source-actions">
        <p class="text-muted">No source file attached.</p>
        <button class="btn btn-small" onclick={viewInGraph}>
          <span class="material-symbols-outlined">bubble_chart</span>
          View in Graph
        </button>
      </div>
    {/if}

    <!-- Components -->
    <section class="section">
      <h3>Components</h3>
      {#if detail.components.length === 0}
        <p class="text-muted">No components.</p>
      {:else}
        {#each detail.components as comp}
          <div class="component-card" class:content-card={isContentComponent(comp)}>
            <div class="component-header">
              <span class="component-type">{comp.component_type}</span>
            </div>
            <pre class="component-value">{formatComponentValue(comp)}</pre>
          </div>
        {/each}
      {/if}
    </section>

    <!-- Outgoing Relationships -->
    <section class="section">
      <h3>Outgoing Relationships ({detail.outgoing_relationships.length})</h3>
      {#if detail.outgoing_relationships.length === 0}
        <p class="text-muted">No outgoing relationships.</p>
      {:else}
        <div class="relationship-list">
          {#each detail.outgoing_relationships as rel}
            <button class="relationship-item" onclick={() => selectEntity(rel.target_id)}>
              <span class="rel-type">{rel.relationship_type}</span>
              <span class="rel-target">→ {rel.target_title || rel.target_id.slice(0, 8)}</span>
            </button>
          {/each}
        </div>
      {/if}
    </section>

    <!-- Incoming Relationships -->
    <section class="section">
      <h3>Incoming Relationships ({detail.incoming_relationships.length})</h3>
      {#if detail.incoming_relationships.length === 0}
        <p class="text-muted">No incoming relationships.</p>
      {:else}
        <div class="relationship-list">
          {#each detail.incoming_relationships as rel}
            <button class="relationship-item" onclick={() => selectEntity(rel.source_id)}>
              <span class="rel-source">{rel.source_title || rel.source_id.slice(0, 8)}</span>
              <span class="rel-type">→ {rel.relationship_type}</span>
            </button>
          {/each}
        </div>
      {/if}
    </section>

    <!-- Events -->
    <section class="section">
      <h3>Events ({detail.events.length})</h3>
      {#if detail.events.length === 0}
        <p class="text-muted">No events.</p>
      {:else}
        <div class="event-list">
          {#each detail.events as event}
            <div class="event-item">
              <span class="event-type">{event.event_type}</span>
              <span class="event-time text-muted">{event.timestamp.slice(0, 19).replace("T", " ")}</span>
            </div>
          {/each}
        </div>
      {/if}
    </section>

    <!-- Version History -->
    <section class="section">
      <h3>Version History ({detail.versions.length})</h3>
      {#if detail.versions.length === 0}
        <p class="text-muted">No version history.</p>
      {:else}
        <div class="version-list">
          {#each detail.versions as ver}
            <div class="version-item">
              <span class="version-number">v{ver.version}</span>
              <span class="version-time text-muted">{ver.created_at.slice(0, 19).replace("T", " ")}</span>
            </div>
          {/each}
        </div>
      {/if}
    </section>
  {:else}
    <div class="empty-state">
      <p class="text-muted">No entity selected.</p>
    </div>
  {/if}
</div>

<style>
  .detail {
    max-width: 720px;
  }

  .detail-header {
    display: flex;
    align-items: center;
    gap: var(--spacing-md);
    margin-bottom: var(--spacing-lg);
  }

  .detail-header h2 {
    font-size: var(--font-size-title-sm);
  }

  .close-btn {
    display: flex;
    align-items: center;
    justify-content: center;
    width: 32px;
    height: 32px;
    border-radius: var(--radius-sm);
    color: var(--text-secondary);
    transition: background var(--transition-fast), color var(--transition-fast);
  }

  .close-btn:hover {
    background: var(--bg-secondary);
    color: var(--text-primary);
  }

  .loading-state,
  .error-state,
  .empty-state {
    display: flex;
    flex-direction: column;
    align-items: center;
    gap: var(--spacing-md);
    padding: var(--spacing-xl);
    text-align: center;
  }

  .spinning {
    animation: spin 1s linear infinite;
    font-size: 36px;
    width: 36px;
    height: 36px;
  }

  @keyframes spin {
    from { transform: rotate(0deg); }
    to { transform: rotate(360deg); }
  }

  .error-icon {
    color: var(--danger);
    font-size: 36px;
    width: 36px;
    height: 36px;
  }

  .entity-header {
    display: flex;
    align-items: center;
    gap: var(--spacing-md);
    margin-bottom: var(--spacing-sm);
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

  .entity-id {
    font-size: var(--font-size-body-sm);
    font-family: var(--font-mono);
    color: var(--text-secondary);
    font-weight: 400;
  }

  .status-indicator {
    font-size: var(--font-size-sm);
    color: var(--text-secondary);
  }

  .status-indicator.active {
    color: var(--success);
  }

  .entity-meta {
    display: flex;
    gap: var(--spacing-lg);
    margin-bottom: var(--spacing-md);
    font-size: var(--font-size-sm);
  }

  .source-actions {
    display: flex;
    flex-direction: column;
    gap: var(--spacing-md);
    padding: var(--spacing-md);
    background: var(--bg-card);
    border: 1px solid var(--border);
    border-radius: var(--radius-md);
    margin-bottom: var(--spacing-lg);
  }

  .source-path {
    display: flex;
    align-items: center;
    gap: var(--spacing-sm);
    font-size: var(--font-size-sm);
    font-family: var(--font-mono);
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }

  .action-buttons {
    display: flex;
    gap: var(--spacing-sm);
    flex-wrap: wrap;
  }

  .btn {
    display: flex;
    align-items: center;
    gap: var(--spacing-xs);
    padding: var(--spacing-sm) var(--spacing-md);
    border-radius: var(--radius-sm);
    font-size: var(--font-size-body-sm);
    font-weight: 500;
    cursor: pointer;
    transition: background var(--transition-fast);
  }

  .btn-small {
    padding: var(--spacing-xs) var(--spacing-sm);
    background: var(--bg-card);
    border: 1px solid var(--border);
    color: var(--text-primary);
  }

  .btn-small:hover {
    background: var(--bg-secondary);
  }

  .btn-small:disabled {
    opacity: 0.6;
    cursor: not-allowed;
  }

  .btn-loading {
    cursor: wait;
  }

  .btn-primary {
    background: var(--accent);
    color: white;
    border: none;
  }

  .btn-primary:hover {
    background: var(--accent-hover);
  }

  .section {
    margin-bottom: var(--spacing-lg);
  }

  .section h3 {
    font-size: var(--font-size-body-md);
    font-weight: 600;
    margin-bottom: var(--spacing-sm);
    color: var(--text-primary);
  }

  .component-card {
    padding: var(--spacing-md);
    background: var(--bg-card);
    border: 1px solid var(--border);
    border-radius: var(--radius-md);
    margin-bottom: var(--spacing-sm);
  }

  .content-card {
    background: var(--bg-primary);
    border-left: 3px solid var(--accent);
  }

  .component-header {
    display: flex;
    align-items: center;
    gap: var(--spacing-sm);
    margin-bottom: var(--spacing-xs);
  }

  .component-type {
    font-size: var(--font-size-sm);
    font-weight: 600;
    color: var(--accent);
    text-transform: uppercase;
    letter-spacing: 0.03em;
  }

  .component-value {
    font-size: var(--font-size-body-sm);
    color: var(--text-primary);
    white-space: pre-wrap;
    word-break: break-word;
    font-family: inherit;
    margin: 0;
    line-height: 1.6;
  }

  .relationship-list {
    display: flex;
    flex-direction: column;
    gap: var(--spacing-xs);
  }

  .relationship-item {
    display: flex;
    align-items: center;
    gap: var(--spacing-md);
    padding: var(--spacing-sm) var(--spacing-md);
    background: var(--bg-card);
    border: 1px solid var(--border);
    border-radius: var(--radius-sm);
    cursor: pointer;
    transition: background var(--transition-fast), border-color var(--transition-fast);
    text-align: left;
    width: 100%;
    font-size: var(--font-size-body-sm);
  }

  .relationship-item:hover {
    background: var(--bg-secondary);
    border-color: var(--accent);
  }

  .rel-type {
    font-weight: 500;
    color: var(--accent);
  }

  .rel-target,
  .rel-source {
    color: var(--text-primary);
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }

  .event-list,
  .version-list {
    display: flex;
    flex-direction: column;
    gap: var(--spacing-xs);
  }

  .event-item,
  .version-item {
    display: flex;
    align-items: center;
    justify-content: space-between;
    padding: var(--spacing-xs) var(--spacing-md);
    background: var(--bg-card);
    border: 1px solid var(--border);
    border-radius: var(--radius-sm);
    font-size: var(--font-size-sm);
  }

  .event-type,
  .version-number {
    font-weight: 500;
    font-family: var(--font-mono);
    font-size: var(--font-size-code-md);
  }

  .event-time,
  .version-time {
    font-size: var(--font-size-sm);
  }
</style>
