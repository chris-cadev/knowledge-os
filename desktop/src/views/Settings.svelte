<script lang="ts">
  import { getState } from "../lib/state.svelte.js";
  import {
    setProvider,
    getProvidersStatus,
    chatTestProvider,
  } from "../lib/api.js";
  import type { ProviderStatus, TestResult } from "../lib/types.js";

  const app = getState();

  let providerKind = $state("mock");
  let model = $state("");
  let baseUrl = $state("");
  let apiKey = $state("");

  let status = $state<ProviderStatus | null>(null);
  let testResult = $state<TestResult | null>(null);
  let saving = $state(false);
  let testing = $state(false);
  let saved = $state(false);

  const providerOptions = [
    { value: "mock", label: "Mock (offline, no setup)" },
    { value: "ollama", label: "Ollama (local, free)" },
    { value: "openai-compatible", label: "OpenAI-compatible (LM Studio, vLLM, etc.)" },
  ];

  const modelHints: Record<string, string> = {
    mock: "",
    ollama: "e.g. llama3.2, deepseek-r1:8b, qwen2.5:7b",
    "openai-compatible": "e.g. gpt-4o, local-model, deepseek-chat",
  };

  const urlHints: Record<string, string> = {
    mock: "",
    ollama: "http://localhost:11434",
    "openai-compatible": "http://localhost:1234/v1",
  };

  let showApiKey = $state(false);
  let statusMessage = $state("");

  async function loadStatus() {
    try {
      status = await getProvidersStatus();
      providerKind = status.provider;
    } catch (e) {
      statusMessage = `Failed to load provider status: ${e}`;
    }
  }

  async function handleSave() {
    saving = true;
    saved = false;
    try {
      status = await setProvider(
        providerKind,
        model,
        baseUrl || null,
        apiKey || null,
      );
      saved = true;
      statusMessage = `Saved: ${providerKind} provider`;
      setTimeout(() => { saved = false; statusMessage = ""; }, 3000);
    } catch (e) {
      statusMessage = `Error saving provider: ${e}`;
    } finally {
      saving = false;
    }
  }

  async function handleTest() {
    testing = true;
    testResult = null;
    try {
      testResult = await chatTestProvider(
        providerKind,
        model,
        baseUrl || null,
        apiKey || null,
      );
    } catch (e) {
      testResult = { success: false, latency_ms: 0, error: String(e) };
    } finally {
      testing = false;
    }
  }

  $effect(() => { loadStatus(); });
</script>

<div class="settings">
  <h2>Settings</h2>
  <p class="text-muted">Configure AI providers for chat and OCR.</p>

  <section class="section">
    <h3>Chat Provider</h3>

    <div class="field">
      <label for="provider-kind">Provider</label>
      <select id="provider-kind" bind:value={providerKind}>
        {#each providerOptions as opt}
          <option value={opt.value}>{opt.label}</option>
        {/each}
      </select>
    </div>

    {#if providerKind !== "mock"}
      <div class="field">
        <label for="model">Model</label>
        <input
          id="model"
          type="text"
          bind:value={model}
          placeholder={modelHints[providerKind]}
        />
      </div>

      <div class="field">
        <label for="base-url">Base URL</label>
        <input
          id="base-url"
          type="text"
          bind:value={baseUrl}
          placeholder={urlHints[providerKind]}
        />
      </div>

      {#if providerKind === "openai-compatible"}
        <div class="field">
          <label for="api-key">API Key</label>
          <div class="api-key-row">
            <input
              id="api-key"
              type={showApiKey ? "text" : "password"}
              bind:value={apiKey}
              placeholder="sk-..."
            />
            <button class="icon-btn" onclick={() => { showApiKey = !showApiKey; }}
              aria-label={showApiKey ? "Hide API key" : "Show API key"}>
              <span class="material-symbols-outlined">
                {showApiKey ? "visibility_off" : "visibility"}
              </span>
            </button>
          </div>
        </div>
      {/if}

      <div class="actions">
        <button class="btn btn-primary" onclick={handleSave} disabled={saving}>
          {saving ? "Saving..." : "Save"}
        </button>
        <button class="btn btn-secondary" onclick={handleTest} disabled={testing}>
          {testing ? "Testing..." : "Test Connection"}
        </button>
      </div>

      {#if testResult}
        <div class="test-result" class:success={testResult.success} class:failure={!testResult.success}>
          {#if testResult.success}
            Connected ({testResult.latency_ms}ms)
          {:else}
            Connection failed: {testResult.error}
          {/if}
        </div>
      {/if}
    {/if}

    {#if providerKind === "mock"}
      <div class="info-box">
        <span class="material-symbols-outlined info-icon">info</span>
        <span>Mock provider works offline without any setup. Switch to Ollama or an OpenAI-compatible provider for real AI responses.</span>
      </div>

      <div class="info-box">
        <span class="material-symbols-outlined info-icon">info</span>
        <span>To install Ollama: download from <a href="https://ollama.com" target="_blank" rel="noopener">ollama.com</a>, then run <code>ollama pull llama3.2</code>.</span>
      </div>

      <div class="info-box">
        <span class="material-symbols-outlined info-icon">info</span>
        <span>To install LM Studio: download from <a href="https://lmstudio.ai" target="_blank" rel="noopener">lmstudio.ai</a>, start a local server at port 1234, then select "OpenAI-compatible" above and set base URL to <code>http://localhost:1234/v1</code>.</span>
      </div>

      <h4 style="margin-top: var(--space-6);">Quick Start</h4>
      <div class="info-box quick-start">
        <ol>
          <li><strong>Ollama:</strong> Install Ollama, run <code>ollama pull llama3.2</code>, select "Ollama" above, enter <code>llama3.2</code>, click Save then Test Connection.</li>
          <li><strong>LM Studio:</strong> Install LM Studio, load a model, start server on port 1234, select "OpenAI-compatible", set base URL to <code>http://localhost:1234/v1</code>, set model name, click Save then Test Connection.</li>
        </ol>
      </div>
    {/if}

    {#if statusMessage}
      <div class="status-msg">{statusMessage}</div>
    {/if}
  </section>
</div>

<style>
  .settings {
    max-width: 640px;
  }

  .settings h2 {
    font-size: var(--font-size-2xl);
    margin-bottom: var(--spacing-sm);
  }

  .section {
    margin-top: var(--spacing-lg);
  }

  .section h3 {
    font-size: var(--font-size-lg);
    margin-bottom: var(--spacing-md);
    color: var(--text-primary);
  }

  .field {
    margin-bottom: var(--spacing-md);
  }

  .field label {
    display: block;
    font-size: var(--font-size-sm);
    font-weight: 600;
    color: var(--text-secondary);
    margin-bottom: var(--spacing-xs);
  }

  .field input,
  .field select {
    width: 100%;
    padding: var(--spacing-sm) var(--spacing-md);
    border: 1px solid var(--border);
    border-radius: var(--radius-sm);
    background: var(--bg-card);
    color: var(--text-primary);
    font-size: var(--font-size-body-md);
  }

  .field select {
    cursor: pointer;
  }

  .field input:focus,
  .field select:focus {
    outline: none;
    border-color: var(--accent);
  }

  .api-key-row {
    display: flex;
    gap: var(--spacing-xs);
  }

  .api-key-row input {
    flex: 1;
  }

  .icon-btn {
    display: flex;
    align-items: center;
    justify-content: center;
    padding: var(--spacing-sm);
    border: 1px solid var(--border);
    border-radius: var(--radius-sm);
    background: var(--bg-card);
    color: var(--text-secondary);
    cursor: pointer;
  }

  .icon-btn:hover {
    background: var(--bg-secondary);
    color: var(--text-primary);
  }

  .actions {
    display: flex;
    gap: var(--spacing-md);
    margin-top: var(--spacing-lg);
  }

  .btn {
    padding: var(--spacing-sm) var(--spacing-lg);
    border-radius: var(--radius-sm);
    font-size: var(--font-size-body-md);
    font-weight: 600;
    cursor: pointer;
    border: none;
    transition: background var(--transition-fast);
  }

  .btn:disabled {
    opacity: 0.5;
    cursor: not-allowed;
  }

  .btn-primary {
    background: var(--accent);
    color: white;
  }

  .btn-primary:hover:not(:disabled) {
    filter: brightness(1.1);
  }

  .btn-secondary {
    background: var(--bg-card);
    color: var(--text-primary);
    border: 1px solid var(--border);
  }

  .btn-secondary:hover:not(:disabled) {
    background: var(--bg-secondary);
  }

  .test-result {
    margin-top: var(--spacing-md);
    padding: var(--spacing-sm) var(--spacing-md);
    border-radius: var(--radius-sm);
    font-size: var(--font-size-sm);
    font-weight: 600;
  }

  .test-result.success {
    background: color-mix(in srgb, var(--accent) 15%, transparent);
    color: var(--accent);
  }

  .test-result.failure {
    background: color-mix(in srgb, #e53e3e 15%, transparent);
    color: #e53e3e;
  }

  .info-box {
    display: flex;
    align-items: flex-start;
    gap: var(--spacing-sm);
    margin-top: var(--spacing-md);
    padding: var(--spacing-md);
    background: var(--bg-card);
    border: 1px solid var(--border);
    border-radius: var(--radius-sm);
    font-size: var(--font-size-sm);
    color: var(--text-secondary);
    line-height: 1.5;
  }

  .info-box code {
    background: var(--bg-secondary);
    padding: 1px 6px;
    border-radius: 3px;
    font-size: 0.9em;
  }

  .info-box a {
    color: var(--accent);
    text-decoration: none;
  }

  .info-box a:hover {
    text-decoration: underline;
  }

  .info-icon {
    font-size: 20px;
    flex-shrink: 0;
    margin-top: 1px;
  }

  .quick-start ol {
    margin: 0;
    padding-left: 1.2em;
  }

  .quick-start li {
    margin-bottom: var(--spacing-xs);
  }

  .status-msg {
    margin-top: var(--spacing-md);
    padding: var(--spacing-sm) var(--spacing-md);
    border-radius: var(--radius-sm);
    background: color-mix(in srgb, var(--accent) 15%, transparent);
    color: var(--accent);
    font-size: var(--font-size-sm);
    font-weight: 600;
  }
</style>
