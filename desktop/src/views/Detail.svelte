<script lang="ts">
  import { getState } from "../lib/state.svelte.js";
  import { onMount } from "svelte";
  import {
    getEntityDetail,
    getEntitySource,
    openInDefaultApp,
    openSourceFolder,
    toggleEntityActive,
  } from "../lib/api.js";
  import { navigateTo } from "../lib/router.svelte.js";
  import type { EntityDetail, ComponentData } from "../lib/types.js";
  import TypeBadge from "../components/TypeBadge.svelte";

  const app = getState();

  let loading = $state(true);
  let sourcePath = $state<string | null>(null);
  let errorMsg = $state<string | null>(null);
  let sourceActionLoading = $state<string | null>(null);
  let archiveLoading = $state(false);

  let showRelationships = $state(true);
  let showEvents = $state(false);
  let showVersions = $state(false);

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

  async function handleToggleArchive() {
    if (!app.selectedEntityId || archiveLoading) return;

    archiveLoading = true;
    const currentActive = app.selectedEntityDetail?.is_active;
    const action = currentActive ? "Archiving" : "Restoring";
    app.statusMessage = `${action} entity...`;

    try {
      const newActive = await toggleEntityActive(app.selectedEntityId);
      if (app.selectedEntityDetail) {
        app.selectedEntityDetail.is_active = newActive;
      }
      app.statusMessage = newActive ? "Entity restored" : "Entity archived";
    } catch (e) {
      const message = `Failed to toggle archive state: ${e}`;
      app.statusMessage = message;
      errorMsg = message;
    } finally {
      archiveLoading = false;
    }
  }

  function selectEntity(id: string) {
    app.selectedEntityId = id;
    navigateTo("detail", id);
  }

  function extractTitle(components: ComponentData[]): string {
    const titleComp = components.find((c) => c.component_type === "Title");
    if (titleComp && typeof titleComp.data === "string") return titleComp.data;
    if (titleComp && typeof titleComp.data === "object" && titleComp.data?.text)
      return titleComp.data.text;
    return "Untitled";
  }

  function extractContent(components: ComponentData[]): string | null {
    const contentComp = components.find((c) => c.component_type === "Content");
    if (!contentComp) return null;
    if (typeof contentComp.data === "string") return contentComp.data;
    if (typeof contentComp.data === "object" && contentComp.data?.text)
      return contentComp.data.text;
    return null;
  }

  function renderMarkdown(content: string): string {
    let html = content;

    html = html.replace(/&/g, "&amp;").replace(/</g, "&lt;").replace(/>/g, "&gt;");

    html = html.replace(/```([\s\S]*?)```/g, (_, code) => {
      const escaped = code
        .replace(/&/g, "&amp;")
        .replace(/</g, "&lt;")
        .replace(/>/g, "&gt;");
      return `<pre class="md-code-block"><code>${escaped}</code></pre>`;
    });

    html = html.replace(/`([^`]+)`/g, '<code class="md-inline-code">$1</code>');

    html = html.replace(
      /\[([^\]]+)\]\(([^)]+)\)/g,
      '<a href="$2" target="_blank" rel="noopener" class="md-link">$1</a>'
    );

    html = html.replace(/\*\*([^*]+)\*\*/g, "<strong>$1</strong>");
    html = html.replace(/\*([^*]+)\*/g, "<em>$1</em>");

    const lines = html.split("\n");
    const result: string[] = [];
    let inList = false;
    let listType: "ul" | "ol" | null = null;

    for (let i = 0; i < lines.length; i++) {
      const line = lines[i];
      const ulMatch = line.match(/^(\s*)[-*+]\s+(.*)$/);
      const olMatch = line.match(/^(\s*)\d+\.\s+(.*)$/);

      if (ulMatch) {
        if (!inList || listType !== "ul") {
          if (inList) result.push(`</${listType}>`);
          result.push("<ul>");
          inList = true;
          listType = "ul";
        }
        result.push(`<li>${ulMatch[2]}</li>`);
      } else if (olMatch) {
        if (!inList || listType !== "ol") {
          if (inList) result.push(`</${listType}>`);
          result.push("<ol>");
          inList = true;
          listType = "ol";
        }
        result.push(`<li>${olMatch[2]}</li>`);
      } else if (line.trim() === "") {
        if (inList) {
          result.push(`</${listType}>`);
          inList = false;
          listType = null;
        }
        result.push("");
      } else {
        if (inList) {
          result.push(`</${listType}>`);
          inList = false;
          listType = null;
        }
        result.push(line);
      }
    }

    if (inList) result.push(`</${listType}>`);

    return result.join("\n");
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

  function isTitleComponent(comp: ComponentData): boolean {
    return comp.component_type === "Title";
  }

  function toggleSection(section: "relationships" | "events" | "versions") {
    if (section === "relationships") showRelationships = !showRelationships;
    if (section === "events") showEvents = !showEvents;
    if (section === "versions") showVersions = !showVersions;
  }
</script>

<div class="detail" role="main">
  <nav class="breadcrumb" aria-label="Breadcrumb">
    <button class="breadcrumb-link" onclick={() => navigateTo("browse")}>
      Browse
    </button>
    {#if app.selectedEntityDetail}
      {@const detail = app.selectedEntityDetail}
      <span class="breadcrumb-sep" aria-hidden="true">/</span>
      <span class="breadcrumb-current">{detail.entity_type}</span>
      <span class="breadcrumb-sep" aria-hidden="true">/</span>
      <span class="breadcrumb-current">{extractTitle(detail.components)}</span>
    {/if}
  </nav>

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
    {@const entityTitle = extractTitle(detail.components)}

    <div class="entity-header">
      <TypeBadge type={detail.entity_type} />
      <h3 class="entity-title">{entityTitle}</h3>
      <div class="status-indicator" class:active={detail.is_active}>
        {detail.is_active ? "Active" : "Archived"}
      </div>
    </div>

    <div class="entity-meta">
      <span class="text-muted">Created: {detail.created_at.slice(0, 10)}</span>
      <span class="text-muted">Updated: {detail.updated_at.slice(0, 10)}</span>
      <span class="entity-id text-muted">{detail.id.slice(0, 8)}...</span>
    </div>

    <div class="header-actions">
      <button
        class="btn btn-small"
        class:btn-loading={archiveLoading}
        onclick={handleToggleArchive}
        disabled={archiveLoading || sourceActionLoading !== null}
        aria-busy={archiveLoading}
      >
        {#if archiveLoading}
          <span class="material-symbols-outlined spinning">sync</span>
        {:else if detail.is_active}
          <span class="material-symbols-outlined">archive</span>
          Archive
        {:else}
          <span class="material-symbols-outlined">unarchive</span>
          Restore
        {/if}
      </button>
      <button class="btn btn-small" onclick={viewInGraph} disabled={sourceActionLoading !== null}>
        <span class="material-symbols-outlined">bubble_chart</span>
        View in Graph
      </button>
    </div>

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
        </div>
      </div>
    {:else}
      <div class="source-actions-inline">
        <p class="text-muted">No source file attached.</p>
      </div>
    {/if}

    {@const contentMarkdown = extractContent(detail.components)}
    {#if contentMarkdown}
      <section class="section" aria-label="Content">
        <h3>Content</h3>
        <div class="content-rendered markdown-body">
          {@html renderMarkdown(contentMarkdown)}
        </div>
      </section>
    {/if}

    <section class="section">
      <h3>Components</h3>
      {#if detail.components.length === 0}
        <p class="text-muted">No components.</p>
      {:else}
        {#each detail.components as comp}
          {#if !isContentComponent(comp) && !isTitleComponent(comp)}
            <div class="component-card">
              <div class="component-header">
                <span class="component-type">{comp.component_type}</span>
              </div>
              <pre class="component-value">{formatComponentValue(comp)}</pre>
            </div>
          {/if}
        {/each}
      {/if}
    </section>

    <section class="section collapsible" aria-label="Relationships">
      <button class="section-toggle" onclick={() => toggleSection("relationships")}>
        <h3>Relationships ({detail.outgoing_relationships.length + detail.incoming_relationships.length})</h3>
        <span class="material-symbols-outlined toggle-icon" class:collapsed={!showRelationships}>
          expand_more
        </span>
      </button>
      {#if showRelationships}
        {#if detail.outgoing_relationships.length > 0}
          <h4 class="subheading">Outgoing ({detail.outgoing_relationships.length})</h4>
          <div class="relationship-list">
            {#each detail.outgoing_relationships as rel}
              <button class="relationship-item" onclick={() => selectEntity(rel.target_id)}>
                <span class="rel-type">{rel.relationship_type}</span>
                <TypeBadge type={rel.target_type || "Concept"} />
                <span class="rel-target">{rel.target_title || rel.target_id.slice(0, 8)}</span>
              </button>
            {/each}
          </div>
        {/if}
        {#if detail.incoming_relationships.length > 0}
          <h4 class="subheading">Incoming ({detail.incoming_relationships.length})</h4>
          <div class="relationship-list">
            {#each detail.incoming_relationships as rel}
              <button class="relationship-item" onclick={() => selectEntity(rel.source_id)}>
                <TypeBadge type={rel.source_type || "Concept"} />
                <span class="rel-source">{rel.source_title || rel.source_id.slice(0, 8)}</span>
                <span class="rel-type">{rel.relationship_type}</span>
              </button>
            {/each}
          </div>
        {/if}
        {#if detail.outgoing_relationships.length === 0 && detail.incoming_relationships.length === 0}
          <p class="text-muted">No relationships.</p>
        {/if}
      {/if}
    </section>

    <section class="section collapsible" aria-label="Events">
      <button class="section-toggle" onclick={() => toggleSection("events")}>
        <h3>Events ({detail.events.length})</h3>
        <span class="material-symbols-outlined toggle-icon" class:collapsed={!showEvents}>
          expand_more
        </span>
      </button>
      {#if showEvents}
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
      {/if}
    </section>

    <section class="section collapsible" aria-label="Version history">
      <button class="section-toggle" onclick={() => toggleSection("versions")}>
        <h3>Version History ({detail.versions.length})</h3>
        <span class="material-symbols-outlined toggle-icon" class:collapsed={!showVersions}>
          expand_more
        </span>
      </button>
      {#if showVersions}
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

  .breadcrumb {
    display: flex;
    align-items: center;
    gap: var(--spacing-xs);
    margin-bottom: var(--spacing-sm);
    font-size: var(--font-size-sm);
    color: var(--text-secondary);
  }

  .breadcrumb-link {
    background: none;
    border: none;
    color: var(--accent);
    cursor: pointer;
    padding: 0;
    font-size: var(--font-size-sm);
  }

  .breadcrumb-link:hover {
    text-decoration: underline;
  }

  .breadcrumb-sep {
    color: var(--text-muted);
  }

  .breadcrumb-current {
    color: var(--text-secondary);
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
    max-width: 200px;
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

  .entity-title {
    font-size: var(--font-size-title-sm);
    font-weight: 600;
    color: var(--text-primary);
    flex: 1;
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
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
    margin-bottom: var(--spacing-sm);
    font-size: var(--font-size-sm);
  }

  .entity-id {
    font-family: var(--font-mono);
  }

  .header-actions {
    display: flex;
    gap: var(--spacing-sm);
    margin-bottom: var(--spacing-md);
    flex-wrap: wrap;
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

  .source-actions-inline {
    margin-bottom: var(--spacing-md);
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

  .collapsible h3 {
    margin-bottom: 0;
  }

  .section-toggle {
    display: flex;
    align-items: center;
    justify-content: space-between;
    width: 100%;
    background: none;
    border: none;
    padding: 0;
    cursor: pointer;
    color: var(--text-primary);
  }

  .section-toggle:hover h3 {
    color: var(--accent);
  }

  .toggle-icon {
    font-size: 20px;
    transition: transform var(--transition-fast);
    color: var(--text-secondary);
  }

  .toggle-icon.collapsed {
    transform: rotate(-90deg);
  }

  .subheading {
    font-size: var(--font-size-sm);
    font-weight: 500;
    color: var(--text-secondary);
    margin: var(--spacing-sm) 0 var(--spacing-xs);
  }

  .content-rendered {
    padding: var(--spacing-md);
    background: var(--bg-primary);
    border-left: 3px solid var(--accent);
    border-radius: var(--radius-md);
  }

  .markdown-body :global(p) {
    margin: 0 0 var(--spacing-sm);
    line-height: 1.6;
  }

  .markdown-body :global(p:last-child) {
    margin-bottom: 0;
  }

  .markdown-body :global(strong) {
    font-weight: 600;
  }

  .markdown-body :global(em) {
    font-style: italic;
  }

  .markdown-body :global(code.md-inline-code) {
    background: var(--bg-secondary);
    padding: 1px 4px;
    border-radius: var(--radius-sm);
    font-family: var(--font-mono);
    font-size: var(--font-size-code-md);
  }

  .markdown-body :global(pre.md-code-block) {
    background: var(--bg-secondary);
    padding: var(--spacing-sm);
    border-radius: var(--radius-sm);
    overflow-x: auto;
    margin: var(--spacing-sm) 0;
  }

  .markdown-body :global(pre.md-code-block code) {
    font-family: var(--font-mono);
    font-size: var(--font-size-code-md);
  }

  .markdown-body :global(ul),
  .markdown-body :global(ol) {
    margin: var(--spacing-sm) 0;
    padding-left: var(--spacing-lg);
  }

  .markdown-body :global(li) {
    margin-bottom: var(--spacing-xs);
    line-height: 1.6;
  }

  .markdown-body :global(a.md-link) {
    color: var(--accent);
    text-decoration: none;
  }

  .markdown-body :global(a.md-link:hover) {
    text-decoration: underline;
  }

  .component-card {
    padding: var(--spacing-md);
    background: var(--bg-card);
    border: 1px solid var(--border);
    border-radius: var(--radius-md);
    margin-bottom: var(--spacing-sm);
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
    gap: var(--spacing-sm);
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
    white-space: nowrap;
  }

  .rel-target,
  .rel-source {
    color: var(--text-primary);
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
    flex: 1;
  }

  .event-list,
  .version-list {
    display: flex;
    flex-direction: column;
    gap: var(--spacing-xs);
    margin-top: var(--spacing-sm);
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
