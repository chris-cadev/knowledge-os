<script lang="ts">
  import { getState } from "../lib/state.svelte.js";
  import {
    importFiles,
    importUrl,
    importClipboard,
    importDatabase,
    importFileRecursive,
    importImage,
    undoImport,
    importDirectoryPreview,
    importStructuredPreview,
    importStructured,
  } from "../lib/api.js";
  import { setupDropZone } from "../lib/drag-drop.js";
  import { onMount } from "svelte";
  import type {
    ImportProgressItem,
    ImportProgressResult,
    DirectoryPreview,
    StructuredPreview,
  } from "../lib/types.js";

  const app = getState();

  let activeTab = $state<"files" | "url" | "clipboard" | "database">("files");

  // Shared state
  let importing = $state(false);
  let progressItems = $state<ImportProgressItem[]>([]);
  let showErrors = $state<Set<number>>(new Set());
  let lastImportResult = $state<ImportProgressResult | null>(null);

  // Files tab
  let dropZoneEl = $state<HTMLElement | null>(null);
  let recursive = $state(false);
  let directoryPreview = $state<DirectoryPreview | null>(null);
  let previewLoading = $state(false);

  // URL tab
  let urlInput = $state("");
  let urlFetching = $state(false);

  // Clipboard tab
  let clipboardContent = $state("");
  let clipboardFormat = $state<"text" | "html">("text");
  let clipboardImporting = $state(false);

  // Database tab
  let dbConnStr = $state("");
  let dbTablesInput = $state("");
  let dbImporting = $state(false);

  // Conflict detection
  let conflicts = $state<string[]>([]);

  onMount(() => {
    if (dropZoneEl) {
      setupDropZone(dropZoneEl, async (event) => {
        await handleFilesTab(event.paths);
      });
    }
  });

  function toggleError(i: number) {
    if (showErrors.has(i)) {
      showErrors.delete(i);
    } else {
      showErrors.add(i);
    }
    showErrors = new Set(showErrors);
  }

  function statusIcon(status: string): string {
    switch (status) {
      case "Imported": return "check_circle";
      case "Merged": return "merge";
      case "Failed": return "error";
      case "Processing": return "sync";
      default: return "help";
    }
  }

  function statusClass(status: string): string {
    switch (status) {
      case "Imported": return "status-created";
      case "Merged": return "status-merged";
      case "Failed": return "status-failed";
      default: return "status-processing";
    }
  }

  // ---- Files Tab ----
  async function handleFilesTab(paths: string[]) {
    if (paths.length === 0 || importing) return;
    importing = true;
    progressItems = [];
    lastImportResult = null;
    app.isImporting = true;

    try {
      const res = await importFiles(paths);
      lastImportResult = res;
      progressItems = res.items;
      refreshEntities(res);
    } catch (e) {
      app.statusMessage = `Import failed: ${e}`;
    } finally {
      importing = false;
      app.isImporting = false;
    }
  }

  async function openFilePicker() {
    try {
      const { open } = await import("@tauri-apps/plugin-dialog");
      const selected = await open({
        multiple: true,
        filters: [
          { name: "All Supported", extensions: [
            "md", "pdf", "html", "htm", "docx", "pptx", "xlsx", "xlsm",
            "csv", "json", "xml", "yaml", "yml",
            "png", "jpg", "jpeg", "gif", "bmp", "webp", "tiff", "tif",
            "eml", "msg", "ics", "vcf", "enex", "opml", "mbox",
            "doc", "ppt", "pps", "xls",
            "pages", "numbers", "key",
            "odt", "ods", "odp", "odg", "ott", "ots", "otp",
          ]},
        ],
      });
      if (selected) {
        const paths = Array.isArray(selected) ? selected : [selected];
        await handleFilesTab(paths);
      }
    } catch (e) {
      app.statusMessage = `File picker error: ${e}`;
    }
  }

  async function openDirectoryPicker() {
    try {
      const { open } = await import("@tauri-apps/plugin-dialog");
      const selected = await open({ directory: true, multiple: false });
      if (selected) {
        const path = Array.isArray(selected) ? selected[0] : selected;
        await handleFilesTab([path]);
      }
    } catch (e) {
      app.statusMessage = `Directory picker error: ${e}`;
    }
  }

  async function toggleRecursive() {
    recursive = !recursive;
    if (directoryPreview && directoryPreview.files.length > 0) {
      previewLoading = true;
      try {
        directoryPreview = await importDirectoryPreview(directoryPreview.files[0], recursive);
      } catch { }
      previewLoading = false;
    }
  }

  async function openDirectoryForPreview() {
    try {
      const { open } = await import("@tauri-apps/plugin-dialog");
      const selected = await open({ directory: true, multiple: false });
      if (selected) {
        const path = Array.isArray(selected) ? selected[0] : selected;
        previewLoading = true;
        directoryPreview = await importDirectoryPreview(path, recursive);
        previewLoading = false;
      }
    } catch (e) {
      app.statusMessage = `Preview error: ${e}`;
    }
  }

  async function handleUndo() {
    try {
      const result = await undoImport();
      if (result.removed_entities.length > 0) {
        app.statusMessage = `Undo: removed ${result.removed_entities.length} entities`;
      } else {
        app.statusMessage = "Nothing to undo";
      }
    } catch (e) {
      app.statusMessage = `Undo failed: ${e}`;
    }
  }

  async function refreshEntities(res: ImportProgressResult) {
    if (res.created > 0 || res.merged > 0) {
      app.statusMessage = `Import complete: ${res.created} created, ${res.merged} merged`;
      const { listEntities } = await import("../lib/api.js");
      const entities = await listEntities();
      app.entities = entities;
      app.entityCount = entities.length;
    }
  }

  function formatFileSize(bytes: number): string {
    if (bytes < 1024) return `${bytes} B`;
    if (bytes < 1024 * 1024) return `${(bytes / 1024).toFixed(1)} KB`;
    return `${(bytes / (1024 * 1024)).toFixed(1)} MB`;
  }

  async function handleRecursiveImport() {
    if (!directoryPreview || directoryPreview.files.length === 0) return;
    importing = true;
    progressItems = [];
    try {
      const res = await importFileRecursive(directoryPreview.files[0]);
      lastImportResult = res;
      progressItems = res.items;
      refreshEntities(res);
    } catch (e) {
      app.statusMessage = `Recursive import failed: ${e}`;
    } finally {
      importing = false;
    }
  }

  // ---- URL Tab ----
  async function handleUrlImport() {
    if (!urlInput.trim() || urlFetching) return;
    urlFetching = true;
    importing = true;
    progressItems = [];
    lastImportResult = null;
    try {
      const res = await importUrl(urlInput.trim());
      lastImportResult = res;
      progressItems = res.items;
      refreshEntities(res);
    } catch (e) {
      app.statusMessage = `URL import failed: ${e}`;
    } finally {
      urlFetching = false;
      importing = false;
    }
  }

  // ---- Clipboard Tab ----
  function detectClipboardFormat(text: string): "text" | "html" {
    if (text.trimStart().startsWith("<") && (text.includes("</") || text.includes("/>"))) {
      return "html";
    }
    return "text";
  }

  function onClipboardPaste(e: ClipboardEvent) {
    const html = e.clipboardData?.getData("text/html");
    if (html) {
      clipboardContent = html;
      clipboardFormat = "html";
    } else {
      const text = e.clipboardData?.getData("text") ?? "";
      clipboardContent = text;
      clipboardFormat = "text";
    }
  }

  async function handleClipboardImport() {
    if (!clipboardContent.trim() || clipboardImporting) return;
    clipboardImporting = true;
    importing = true;
    progressItems = [];
    lastImportResult = null;
    try {
      const format = clipboardFormat === "html" ? "html" : undefined;
      const res = await importClipboard(clipboardContent, format);
      lastImportResult = res;
      progressItems = res.items;
      refreshEntities(res);
    } catch (e) {
      app.statusMessage = `Clipboard import failed: ${e}`;
    } finally {
      clipboardImporting = false;
      importing = false;
    }
  }

  // ---- Database Tab ----
  async function handleDatabaseImport() {
    if (!dbConnStr.trim() || dbImporting) return;
    dbImporting = true;
    importing = true;
    progressItems = [];
    lastImportResult = null;
    try {
      const tables = dbTablesInput.trim()
        ? dbTablesInput.split(",").map(t => t.trim()).filter(Boolean)
        : [];
      const res = await importDatabase(dbConnStr.trim(), tables);
      lastImportResult = res;
      progressItems = res.items;
      refreshEntities(res);
    } catch (e) {
      app.statusMessage = `Database import failed: ${e}`;
    } finally {
      dbImporting = false;
      importing = false;
    }
  }
</script>

<div class="import">
  <h2>Import Knowledge</h2>

  <!-- Tabs -->
  <div class="tabs">
    <button
      class="tab"
      class:active={activeTab === "files"}
      onclick={() => (activeTab = "files")}
    >
      <span class="material-symbols-outlined">folder</span>
      Files
    </button>
    <button
      class="tab"
      class:active={activeTab === "url"}
      onclick={() => (activeTab = "url")}
    >
      <span class="material-symbols-outlined">language</span>
      URL
    </button>
    <button
      class="tab"
      class:active={activeTab === "clipboard"}
      onclick={() => (activeTab = "clipboard")}
    >
      <span class="material-symbols-outlined">content_paste</span>
      Clipboard
    </button>
    <button
      class="tab"
      class:active={activeTab === "database"}
      onclick={() => (activeTab = "database")}
    >
      <span class="material-symbols-outlined">storage</span>
      Database
    </button>
  </div>

  <!-- Tab Content -->
  <div class="tab-content">
    {#if activeTab === "files"}
      <div class="tab-pane">
        <div class="import-options">
          <label class="checkbox-label">
            <input type="checkbox" checked={recursive} onchange={toggleRecursive} />
            Recursive (include subdirectories)
            {#if recursive && directoryPreview}
              <span class="badge">{directoryPreview.file_count} files</span>
            {/if}
          </label>
          <button class="btn btn-sm btn-secondary" onclick={openDirectoryForPreview}>
            <span class="material-symbols-outlined">preview</span>
            Preview Directory
          </button>
          <button class="btn btn-sm btn-secondary" onclick={handleUndo}>
            <span class="material-symbols-outlined">undo</span>
            Undo Last Import
          </button>
        </div>

        <!-- Drop Zone -->
        <div
          class="drop-zone"
          class:importing
          bind:this={dropZoneEl}
        >
          {#if importing}
            <div class="importing-indicator">
              <span class="material-symbols-outlined spinning">sync</span>
              <p>Importing...</p>
            </div>
          {:else}
            <div class="drop-content">
              <span class="material-symbols-outlined drop-icon">cloud_upload</span>
              <p>Drag & drop files or directories</p>
              <p class="text-muted small">Supports all formats (Markdown, PDF, Office, images, structured data, email, and more)</p>
              <div class="or-divider"><span>or</span></div>
              <div class="button-group">
                <button class="btn btn-primary" onclick={openFilePicker}>
                  <span class="material-symbols-outlined">description</span>
                  Select Files
                </button>
                <button class="btn btn-secondary" onclick={openDirectoryPicker}>
                  <span class="material-symbols-outlined">folder_open</span>
                  Select Directory
                </button>
              </div>
            </div>
          {/if}
        </div>

        <!-- Directory Preview -->
        {#if directoryPreview && !importing}
          <div class="preview-panel">
            <h3>Directory Preview</h3>
            <div class="preview-stats">
              <span class="stat-chip">{directoryPreview.file_count} files</span>
              <span class="stat-chip">{formatFileSize(directoryPreview.total_size_bytes)}</span>
            </div>
            <div class="format-breakdown">
              {#each Object.entries(directoryPreview.formats) as [fmt, count]}
                <span class="format-badge">{fmt}: {count}</span>
              {/each}
            </div>
            <button class="btn btn-primary" onclick={handleRecursiveImport}>
              <span class="material-symbols-outlined">download</span>
              Import All
            </button>
          </div>
        {/if}
      </div>

    {:else if activeTab === "url"}
      <div class="tab-pane">
        <p class="text-muted">Fetch content from a web URL. Supports HTML, text, and PDF URLs.</p>
        <div class="url-input-group">
          <input
            type="url"
            class="input"
            placeholder="https://example.com/article"
            bind:value={urlInput}
            disabled={urlFetching}
          />
          <button
            class="btn btn-primary"
            onclick={handleUrlImport}
            disabled={!urlInput.trim() || urlFetching}
          >
            {#if urlFetching}
              <span class="material-symbols-outlined spinning">sync</span>
              Fetching...
            {:else}
              <span class="material-symbols-outlined">download</span>
              Fetch & Import
            {/if}
          </button>
        </div>
      </div>

    {:else if activeTab === "clipboard"}
      <div class="tab-pane">
        <p class="text-muted">
          Paste text or HTML content from your clipboard.
          {#if clipboardFormat === "html"}
            <span class="badge">HTML detected</span>
          {/if}
        </p>
        <textarea
          class="clipboard-textarea"
          placeholder="Paste text or HTML here..."
          bind:value={clipboardContent}
          disabled={clipboardImporting}
          onpaste={onClipboardPaste}
        ></textarea>
        <div class="clipboard-info">
          <span class="text-muted small">
            {clipboardContent.length} characters detected as {clipboardFormat}
          </span>
        </div>
        <button
          class="btn btn-primary"
          onclick={handleClipboardImport}
          disabled={!clipboardContent.trim() || clipboardImporting}
        >
          {#if clipboardImporting}
            <span class="material-symbols-outlined spinning">sync</span>
            Importing...
          {:else}
            <span class="material-symbols-outlined">content_paste</span>
            Import from Clipboard
          {/if}
        </button>
      </div>

    {:else if activeTab === "database"}
      <div class="tab-pane">
        <p class="text-muted">Connect to a database and import tables. Supports SQLite, PostgreSQL, and MySQL.</p>
        <div class="db-form">
          <label>
            Connection String
            <input
              type="text"
              class="input"
              placeholder="sqlite:///path/to/db.db or postgres://user:pass@host/db"
              bind:value={dbConnStr}
              disabled={dbImporting}
            />
          </label>
          <label>
            Tables (comma-separated, leave empty for all)
            <input
              type="text"
              class="input"
              placeholder="users, posts, comments"
              bind:value={dbTablesInput}
              disabled={dbImporting}
            />
          </label>
          <button
            class="btn btn-primary"
            onclick={handleDatabaseImport}
            disabled={!dbConnStr.trim() || dbImporting}
          >
            {#if dbImporting}
              <span class="material-symbols-outlined spinning">sync</span>
              Importing...
            {:else}
              <span class="material-symbols-outlined">storage</span>
              Import from Database
            {/if}
          </button>
        </div>
      </div>
    {/if}
  </div>

  <!-- Progress Items -->
  {#if progressItems.length > 0}
    <div class="progress-list">
      <h3>Import Results</h3>
      <div class="result-stats">
        <div class="stat">
          <span class="stat-value">{lastImportResult?.created ?? 0}</span>
          <span class="stat-label">Created</span>
        </div>
        <div class="stat">
          <span class="stat-value">{lastImportResult?.merged ?? 0}</span>
          <span class="stat-label">Merged</span>
        </div>
        <div class="stat">
          <span class="stat-value">{lastImportResult?.errors.length ?? 0}</span>
          <span class="stat-label">Errors</span>
        </div>
      </div>

      <div class="items">
        {#each progressItems as item, i}
          <div class="import-item {statusClass(item.status)}">
            <span class="material-symbols-outlined item-icon">{statusIcon(item.status)}</span>
            <div class="item-details">
              <span class="item-path">{item.path}</span>
              <span class="item-status">{item.status}</span>
              {#if item.action}
                <span class="item-action">({item.action})</span>
              {/if}
            </div>
            {#if item.error}
              <button class="error-toggle" onclick={() => toggleError(i)}>
                <span class="material-symbols-outlined">expand_more</span>
              </button>
            {/if}
          </div>
          {#if item.error && showErrors.has(i)}
            <div class="error-detail">
              <span class="material-symbols-outlined error-icon">error</span>
              <span>{item.error}</span>
            </div>
          {/if}
        {/each}
      </div>

      <!-- Undo button -->
      {#if lastImportResult && (lastImportResult.created > 0 || lastImportResult.merged > 0)}
        <div class="post-import-actions">
          <button class="btn btn-secondary" onclick={handleUndo}>
            <span class="material-symbols-outlined">undo</span>
            Undo This Import
          </button>
        </div>
      {/if}
    </div>
  {/if}
</div>

<style>
  .import {
    max-width: 800px;
  }

  .import h2 {
    font-size: var(--font-size-2xl);
    margin-bottom: var(--spacing-md);
  }

  .tabs {
    display: flex;
    gap: 0;
    border-bottom: 1px solid var(--border);
    margin-bottom: var(--spacing-lg);
  }

  .tab {
    display: flex;
    align-items: center;
    gap: var(--spacing-xs);
    padding: var(--spacing-sm) var(--spacing-md);
    background: transparent;
    border: none;
    border-bottom: 2px solid transparent;
    cursor: pointer;
    font-size: var(--font-size-body-sm);
    color: var(--text-secondary);
    transition: color var(--transition-fast), border-color var(--transition-fast);
  }

  .tab:hover {
    color: var(--text-primary);
  }

  .tab.active {
    color: var(--accent);
    border-bottom-color: var(--accent);
  }

  .tab-content {
    min-height: 200px;
  }

  .tab-pane {
    display: flex;
    flex-direction: column;
    gap: var(--spacing-md);
  }

  .import-options {
    display: flex;
    align-items: center;
    gap: var(--spacing-md);
    flex-wrap: wrap;
  }

  .checkbox-label {
    display: flex;
    align-items: center;
    gap: var(--spacing-xs);
    font-size: var(--font-size-sm);
    cursor: pointer;
  }

  .badge {
    display: inline-block;
    padding: 2px 6px;
    background: var(--accent);
    color: white;
    border-radius: var(--radius-sm);
    font-size: var(--font-size-xs);
  }

  .btn-sm {
    padding: var(--spacing-xs) var(--spacing-sm);
    font-size: var(--font-size-xs);
  }

  .drop-zone {
    border: 2px dashed var(--border);
    border-radius: var(--radius-lg);
    padding: var(--spacing-xl);
    text-align: center;
    transition: border-color var(--transition-fast), background var(--transition-fast);
    min-height: 200px;
    display: flex;
    align-items: center;
    justify-content: center;
  }

  .drop-zone.importing {
    border-style: solid;
    border-color: var(--accent);
    background: var(--bg-secondary);
    cursor: default;
  }

  :global(.drop-zone.drag-over) {
    border-color: var(--accent);
    background: var(--bg-secondary);
  }

  .drop-content {
    display: flex;
    flex-direction: column;
    align-items: center;
    gap: var(--spacing-sm);
  }

  .drop-icon {
    font-size: 48px;
    color: var(--accent);
    width: 48px;
    height: 48px;
  }

  .small {
    font-size: var(--font-size-sm);
  }

  .text-muted {
    color: var(--text-secondary);
  }

  .or-divider {
    display: flex;
    align-items: center;
    gap: var(--spacing-md);
    width: 100%;
    margin: var(--spacing-sm) 0;
    color: var(--text-secondary);
    font-size: var(--font-size-sm);
  }

  .or-divider::before,
  .or-divider::after {
    content: "";
    flex: 1;
    height: 1px;
    background: var(--border);
  }

  .button-group {
    display: flex;
    gap: var(--spacing-md);
    margin-top: var(--spacing-sm);
  }

  .btn {
    display: inline-flex;
    align-items: center;
    gap: var(--spacing-sm);
    padding: var(--spacing-sm) var(--spacing-md);
    border-radius: var(--radius-sm);
    font-size: var(--font-size-body-sm);
    font-weight: 500;
    cursor: pointer;
    transition: background var(--transition-fast);
    border: none;
  }

  .btn-primary {
    background: var(--accent);
    color: white;
  }

  .btn-primary:hover {
    background: var(--accent-hover);
  }

  .btn-secondary {
    background: var(--bg-card);
    color: var(--text-primary);
    border: 1px solid var(--border);
  }

  .btn-secondary:hover {
    background: var(--bg-secondary);
  }

  .importing-indicator {
    display: flex;
    flex-direction: column;
    align-items: center;
    gap: var(--spacing-md);
  }

  .spinning {
    animation: spin 1s linear infinite;
  }

  @keyframes spin {
    from { transform: rotate(0deg); }
    to { transform: rotate(360deg); }
  }

  .preview-panel {
    background: var(--bg-card);
    border: 1px solid var(--border);
    border-radius: var(--radius-md);
    padding: var(--spacing-md);
  }

  .preview-stats {
    display: flex;
    gap: var(--spacing-sm);
    margin: var(--spacing-sm) 0;
  }

  .stat-chip {
    padding: 2px 8px;
    background: var(--bg-secondary);
    border-radius: var(--radius-sm);
    font-size: var(--font-size-sm);
  }

  .format-breakdown {
    display: flex;
    flex-wrap: wrap;
    gap: var(--spacing-xs);
    margin-bottom: var(--spacing-md);
  }

  .format-badge {
    padding: 2px 6px;
    background: var(--bg-secondary);
    border-radius: var(--radius-sm);
    font-size: var(--font-size-xs);
  }

  .url-input-group {
    display: flex;
    gap: var(--spacing-sm);
    align-items: flex-start;
  }

  .input {
    flex: 1;
    padding: var(--spacing-sm) var(--spacing-md);
    border: 1px solid var(--border);
    border-radius: var(--radius-sm);
    background: var(--bg-card);
    color: var(--text-primary);
    font-size: var(--font-size-body-sm);
  }

  .clipboard-textarea {
    width: 100%;
    min-height: 200px;
    padding: var(--spacing-sm);
    border: 1px solid var(--border);
    border-radius: var(--radius-sm);
    background: var(--bg-card);
    color: var(--text-primary);
    font-family: monospace;
    font-size: var(--font-size-body-sm);
    resize: vertical;
  }

  .clipboard-info {
    display: flex;
    justify-content: space-between;
  }

  .db-form {
    display: flex;
    flex-direction: column;
    gap: var(--spacing-md);
  }

  .db-form label {
    display: flex;
    flex-direction: column;
    gap: var(--spacing-xs);
    font-size: var(--font-size-sm);
    color: var(--text-secondary);
  }

  /* Progress List */
  .progress-list {
    margin-top: var(--spacing-lg);
  }

  .result-stats {
    display: flex;
    gap: var(--spacing-md);
    margin-bottom: var(--spacing-md);
  }

  .stat {
    display: flex;
    flex-direction: column;
    align-items: center;
    padding: var(--spacing-md);
    background: var(--bg-card);
    border: 1px solid var(--border);
    border-radius: var(--radius-md);
    min-width: 100px;
  }

  .stat-value {
    font-size: var(--font-size-2xl);
    font-weight: 700;
  }

  .stat-label {
    font-size: var(--font-size-sm);
    color: var(--text-secondary);
  }

  .items {
    display: flex;
    flex-direction: column;
    gap: var(--spacing-xs);
  }

  .import-item {
    display: flex;
    align-items: center;
    gap: var(--spacing-sm);
    padding: var(--spacing-sm) var(--spacing-md);
    border-radius: var(--radius-sm);
    background: var(--bg-card);
    border: 1px solid var(--border);
  }

  .item-icon {
    font-size: 20px;
    width: 20px;
    height: 20px;
  }

  .status-created .item-icon { color: var(--success, #2e7d32); }
  .status-merged .item-icon { color: var(--accent); }
  .status-failed .item-icon { color: var(--danger); }
  .status-processing .item-icon { color: var(--text-secondary); }

  .item-details {
    display: flex;
    align-items: center;
    gap: var(--spacing-sm);
    flex: 1;
    font-size: var(--font-size-sm);
  }

  .item-path {
    font-weight: 500;
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
    max-width: 300px;
  }

  .item-status {
    font-size: var(--font-size-xs);
    color: var(--text-secondary);
  }

  .item-action {
    font-size: var(--font-size-xs);
    color: var(--text-secondary);
    font-style: italic;
  }

  .error-toggle {
    background: transparent;
    border: none;
    cursor: pointer;
    color: var(--text-secondary);
    padding: 0;
  }

  .error-detail {
    display: flex;
    align-items: flex-start;
    gap: var(--spacing-sm);
    padding: var(--spacing-sm) var(--spacing-md);
    margin-left: var(--spacing-xl);
    background: rgba(186, 26, 26, 0.05);
    border-radius: var(--radius-sm);
    font-size: var(--font-size-sm);
    color: var(--danger);
  }

  .error-icon {
    font-size: 16px;
    width: 16px;
    height: 16px;
    flex-shrink: 0;
    color: var(--danger);
  }

  .post-import-actions {
    margin-top: var(--spacing-md);
    display: flex;
    gap: var(--spacing-sm);
  }
</style>
