<script lang="ts">
  import { getState } from "../lib/state.svelte.js";
  import { importFiles } from "../lib/api.js";
  import { setupDropZone, isSupportedFile } from "../lib/drag-drop.js";
  import { onMount } from "svelte";

  const app = getState();

  let importing = $state(false);
  let result = $state<{ created: number; merged: number; errors: { path: string; message: string }[] } | null>(null);
  let dropZoneEl = $state<HTMLElement | null>(null);

  onMount(() => {
    if (dropZoneEl) {
      setupDropZone(dropZoneEl, async (event) => {
        await handleFiles(event.paths);
      });
    }
  });

  async function handleFiles(paths: string[]) {
    if (paths.length === 0 || importing) return;

    importing = true;
    result = null;
    app.isImporting = true;

    try {
      const res = await importFiles(paths);
      result = res;

      if (res.created > 0 || res.merged > 0) {
        app.statusMessage = `Import complete: ${res.created} created, ${res.merged} merged`;
        // Refresh entity count
        const { listEntities } = await import("../lib/api.js");
        const entities = await listEntities();
        app.entities = entities;
        app.entityCount = entities.length;
      }
    } catch (e) {
      app.statusMessage = `Import failed: ${e}`;
      result = { created: 0, merged: 0, errors: [{ path: "", message: String(e) }] };
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
        filters: [{ name: "Documents", extensions: ["md", "pdf"] }],
      });
      if (selected) {
        // open() returns string | string[] | null in Tauri 2
        const paths = Array.isArray(selected) ? selected : [selected];
        await handleFiles(paths);
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
        // For directory import, the backend handles recursive discovery
        const path = Array.isArray(selected) ? selected[0] : selected;
        await handleFiles([path]);
      }
    } catch (e) {
      app.statusMessage = `Directory picker error: ${e}`;
    }
  }
</script>

<div class="import">
  <h2>Import Documents</h2>
  <p class="text-muted">Drag and drop files or use the file picker to import Markdown and PDF documents.</p>

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
        <p>Drag & drop files here</p>
        <p class="text-muted small">Supports .md and .pdf files</p>
        <div class="or-divider">
          <span>or</span>
        </div>
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

  <!-- Results -->
  {#if result}
    <div class="results">
      <h3>Import Results</h3>
      <div class="result-stats">
        <div class="stat">
          <span class="stat-value">{result.created}</span>
          <span class="stat-label">Created</span>
        </div>
        <div class="stat">
          <span class="stat-value">{result.merged}</span>
          <span class="stat-label">Merged</span>
        </div>
        <div class="stat">
          <span class="stat-value">{result.errors.length}</span>
          <span class="stat-label">Errors</span>
        </div>
      </div>
      {#if result.errors.length > 0}
        <div class="errors">
          <h4>Errors</h4>
          <ul>
            {#each result.errors as err}
              <li class="error-item">
                <span class="material-symbols-outlined error-icon">error</span>
                <span><strong>{err.path}</strong>: {err.message}</span>
              </li>
            {/each}
          </ul>
        </div>
      {/if}
    </div>
  {/if}
</div>

<style>
  .import {
    max-width: 640px;
  }

  .import h2 {
    font-size: var(--font-size-2xl);
    margin-bottom: var(--spacing-sm);
  }

  .drop-zone {
    margin-top: var(--spacing-lg);
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
    display: flex;
    align-items: center;
    gap: var(--spacing-sm);
    padding: var(--spacing-sm) var(--spacing-md);
    border-radius: var(--radius-sm);
    font-size: var(--font-size-body-sm);
    font-weight: 500;
    cursor: pointer;
    transition: background var(--transition-fast);
  }

  .btn-primary {
    background: var(--accent);
    color: white;
    border: none;
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
    font-size: 36px;
    width: 36px;
    height: 36px;
  }

  @keyframes spin {
    from { transform: rotate(0deg); }
    to { transform: rotate(360deg); }
  }

  .results {
    margin-top: var(--spacing-lg);
  }

  .results h3 {
    font-size: var(--font-size-title-sm);
    margin-bottom: var(--spacing-md);
  }

  .result-stats {
    display: flex;
    gap: var(--spacing-md);
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

  .errors {
    margin-top: var(--spacing-md);
  }

  .errors h4 {
    font-size: var(--font-size-body-md);
    margin-bottom: var(--spacing-sm);
    color: var(--danger);
  }

  .errors ul {
    list-style: none;
    padding: 0;
  }

  .error-item {
    display: flex;
    align-items: flex-start;
    gap: var(--spacing-sm);
    padding: var(--spacing-sm);
    background: rgba(186, 26, 26, 0.05);
    border-radius: var(--radius-sm);
    margin-bottom: var(--spacing-xs);
    font-size: var(--font-size-sm);
  }

  .error-icon {
    color: var(--danger);
    font-size: 18px;
    width: 18px;
    height: 18px;
    flex-shrink: 0;
  }
</style>
