<script lang="ts">
  import { getState } from "../lib/state.svelte.js";
  import { navigateTo } from "../lib/router.svelte.js";
  import {
    setProvider,
    getProvidersStatus,
    resetProvider,
    chatTestProvider,
    setOcrProvider,
    getOcrProviderStatus,
    resetOcrProvider,
    getIgnorePatterns,
    setIgnorePatterns,
    resetIgnorePatterns,
    openInDefaultApp,
  } from "../lib/api.js";
  import type {
    ProviderStatus,
    TestResult,
    OcrProviderStatus,
  } from "../lib/types.js";

  const app = getState();

  // ── Chat provider state ──────────────────────────────────────
  let chatProviderKind = $state("mock");
  let chatModel = $state("");
  let chatBaseUrl = $state("");
  let chatApiKey = $state("");
  let chatStatus = $state<ProviderStatus | null>(null);
  let chatTestResult = $state<TestResult | null>(null);
  let chatSaving = $state(false);
  let chatTesting = $state(false);
  let chatSaved = $state(false);
  let lastTestTime = $state<string | null>(null);

  // ── OCR provider state ───────────────────────────────────────
  let ocrBackend = $state("mock");
  let ocrModel = $state("");
  let ocrBaseUrl = $state("");
  let ocrApiKey = $state("");
  let ocrStatus = $state<OcrProviderStatus | null>(null);
  let ocrSaving = $state(false);
  let ocrSaved = $state(false);
  let showOcrAdvanced = $state(false);

  // ── Ignore patterns state ────────────────────────────────────
  let ignorePatterns = $state("");
  let ignoreSaving = $state(false);
  let ignoreSaved = $state(false);

  // ── UI state ─────────────────────────────────────────────────
  let showChatApiKey = $state(false);
  let showOcrApiKey = $state(false);
  let showResetConfirm = $state(false);
  let statusMessage = $state("");
  let statusLevel = $state<"info" | "success" | "error">("info");
  let showChatAdvanced = $state(false);
  let dirty = $state(false);

  // ── Constants ────────────────────────────────────────────────
  const chatProviderOptions = [
    { value: "mock", label: "Mock (offline, no setup)" },
    { value: "ollama", label: "Ollama (local, free)" },
    {
      value: "openai-compatible",
      label: "OpenAI-compatible (LM Studio, vLLM, etc.)",
    },
  ];

  const chatModelHints: Record<string, string> = {
    mock: "",
    ollama: "llama3.2, deepseek-r1:8b, qwen2.5:7b",
    "openai-compatible": "gpt-4o, local-model, deepseek-chat",
  };

  const chatUrlDefaults: Record<string, string> = {
    mock: "",
    ollama: "http://localhost:11434",
    "openai-compatible": "http://localhost:1234/v1",
  };

  const ocrBackendOptions = [
    { value: "mock", label: "Mock (test only)" },
    { value: "tesseract", label: "Tesseract (local, CPU)" },
    { value: "ollama", label: "Ollama vision (local, GPU)" },
    { value: "api", label: "API (OpenAI-compatible vision)" },
  ];

  const ocrModelHints: Record<string, string> = {
    mock: "",
    tesseract: "eng, eng+fra (language codes)",
    ollama: "deepseek-ocr, qwen3-vl, llama3.2-vision",
    api: "gpt-4o, gpt-4o-mini",
  };

  const ocrUrlDefaults: Record<string, string> = {
    mock: "",
    tesseract: "",
    ollama: "http://localhost:11434",
    api: "http://localhost:1234/v1",
  };

  // ── Load current config ──────────────────────────────────────
  async function loadAllStatus() {
    try {
      const cs = await getProvidersStatus();
      chatStatus = cs;
      chatProviderKind = cs.provider;
      chatModel = cs.model;
      chatBaseUrl = cs.base_url;
      app.providerName = labelForProvider(cs.provider);
      app.providerModel = cs.model;
    } catch (e) {
      statusMessage = `Could not load provider settings: ${e}`;
      statusLevel = "error";
    }
    try {
      const os = await getOcrProviderStatus();
      ocrStatus = os;
      ocrBackend = os.backend;
      ocrModel = os.model;
      ocrBaseUrl = os.base_url;
    } catch {
      // OCR config may not exist yet
    }
    try {
      ignorePatterns = await getIgnorePatterns();
    } catch {
      // ignore config may not exist yet
    }
  }

  function labelForProvider(kind: string): string {
    const found = chatProviderOptions.find((o) => o.value === kind);
    return found
      ? found.label.split(" ")[0]
      : kind.charAt(0).toUpperCase() + kind.slice(1);
  }

  function providerLabel(kind: string): string {
    const found = chatProviderOptions.find((o) => o.value === kind);
    return found ? found.label : kind;
  }

  function ocrLabel(backend: string): string {
    const found = ocrBackendOptions.find((o) => o.value === backend);
    return found ? found.label : backend;
  }

  // ── Validation ───────────────────────────────────────────────
  function isRemoteHttps(url: string): boolean {
    return /^https:\/\//i.test(url.trim());
  }

  function canSaveChat(): boolean {
    if (chatProviderKind === "mock") return true;
    if (!chatModel.trim()) return false;
    if (
      chatProviderKind === "openai-compatible" &&
      !chatApiKey.trim() &&
      (isRemoteHttps(chatBaseUrl) || !chatBaseUrl.trim())
    )
      return false;
    return true;
  }

  function canSaveOcr(): boolean {
    if (ocrBackend === "mock") return true;
    if (!ocrModel.trim()) return false;
    return true;
  }

  function canTestChat(): boolean {
    return canSaveChat() && chatProviderKind !== "mock";
  }

  // ── Save chat provider ───────────────────────────────────────
  async function handleSaveChat() {
    if (!canSaveChat()) return;
    chatSaving = true;
    try {
      const result = await setProvider(
        chatProviderKind,
        chatModel,
        chatBaseUrl || null,
        chatApiKey || null,
      );
      chatStatus = result;
      chatSaved = true;
      app.providerName = labelForProvider(chatProviderKind);
      app.providerModel = chatModel;
      dirty = false;
      statusMessage = `Chat provider saved: ${providerLabel(chatProviderKind)}`;
      statusLevel = "success";
      setTimeout(() => {
        chatSaved = false;
      }, 3000);
    } catch (e) {
      statusMessage = `Failed to save: ${e}`;
      statusLevel = "error";
    } finally {
      chatSaving = false;
    }
  }

  // ── Test chat provider ───────────────────────────────────────
  async function handleTestChat() {
    if (!canTestChat()) return;
    chatTesting = true;
    chatTestResult = null;
    try {
      chatTestResult = await chatTestProvider(
        chatProviderKind,
        chatModel,
        chatBaseUrl || null,
        chatApiKey || null,
      );
      lastTestTime = new Date().toLocaleTimeString();
      app.providerReachable = chatTestResult.success;
      if (chatTestResult.success) {
        statusMessage = `Connected in ${chatTestResult.latency_ms}ms`;
        statusLevel = "success";
      } else {
        statusMessage = `Connection failed: ${chatTestResult.error}`;
        statusLevel = "error";
      }
    } catch (e) {
      chatTestResult = { success: false, latency_ms: 0, error: String(e) };
      statusMessage = `Test error: ${e}`;
      statusLevel = "error";
    } finally {
      chatTesting = false;
    }
  }

  // ── Reset chat provider ──────────────────────────────────────
  async function handleResetChat() {
    try {
      const result = await resetProvider();
      chatProviderKind = result.provider;
      chatModel = "";
      chatBaseUrl = "";
      chatApiKey = "";
      chatTestResult = null;
      lastTestTime = null;
      app.providerName = "Mock";
      app.providerModel = "";
      showResetConfirm = false;
      dirty = false;
      statusMessage = "Reset to default (Mock) provider";
      statusLevel = "info";
    } catch (e) {
      statusMessage = `Reset failed: ${e}`;
      statusLevel = "error";
    }
  }

  // ── Save OCR provider ────────────────────────────────────────
  async function handleSaveOcr() {
    if (!canSaveOcr()) return;
    ocrSaving = true;
    try {
      ocrStatus = await setOcrProvider(
        ocrBackend,
        ocrModel,
        ocrBaseUrl || null,
        ocrApiKey || null,
      );
      ocrSaved = true;
      statusMessage = `OCR backend saved: ${ocrLabel(ocrBackend)}`;
      statusLevel = "success";
      setTimeout(() => {
        ocrSaved = false;
      }, 3000);
    } catch (e) {
      statusMessage = `Failed to save OCR config: ${e}`;
      statusLevel = "error";
    } finally {
      ocrSaving = false;
    }
  }

  // ── Reset OCR provider ───────────────────────────────────────
  async function handleResetOcr() {
    try {
      const result = await resetOcrProvider();
      ocrBackend = result.backend;
      ocrModel = "";
      ocrBaseUrl = "";
      ocrApiKey = "";
      statusMessage = "Reset to default (Mock) OCR backend";
      statusLevel = "info";
    } catch (e) {
      statusMessage = `OCR reset failed: ${e}`;
      statusLevel = "error";
    }
  }

  async function handleSaveIgnore() {
    ignoreSaving = true;
    ignoreSaved = false;
    try {
      await setIgnorePatterns(ignorePatterns);
      ignoreSaved = true;
      statusMessage = "Import exclusion patterns saved";
      statusLevel = "success";
    } catch (e) {
      statusMessage = `Failed to save exclusion patterns: ${e}`;
      statusLevel = "error";
    } finally {
      ignoreSaving = false;
    }
  }

  async function handleResetIgnore() {
    try {
      ignorePatterns = await resetIgnorePatterns();
      statusMessage = "Reset to default exclusion patterns";
      statusLevel = "info";
    } catch (e) {
      statusMessage = `Reset failed: ${e}`;
      statusLevel = "error";
    }
  }

  // ── Quick-start presets ──────────────────────────────────────
  function applyPreset(kind: string) {
    if (kind === "ollama") {
      chatProviderKind = "ollama";
      chatModel = "llama3.2";
      chatBaseUrl = "http://localhost:11434";
      chatApiKey = "";
    } else if (kind === "lm-studio") {
      chatProviderKind = "openai-compatible";
      chatModel = "local-model";
      chatBaseUrl = "http://localhost:1234/v1";
      chatApiKey = "";
    }
    dirty = true;
  }

  // ── Section navigation ─────────────────────────────────────
  const sections = [
    { id: "chat-provider", label: "Chat Provider" },
    { id: "ocr", label: "OCR Text Extraction" },
    { id: "import-exclusions", label: "Import Exclusions" },
    { id: "help", label: "Help" },
  ];

  let activeSection = $state("chat-provider");

  // ── Section flash highlight ─────────────────────────────────
  let flashSection = $state<string | null>(null);
  let flashNonce = $state(0);
  let flashTimer: ReturnType<typeof setTimeout> | null = null;

  function scrollToSection(id: string) {
    document.getElementById(id)?.scrollIntoView({ behavior: "smooth" });
    if (flashTimer) clearTimeout(flashTimer);
    flashSection = id;
    flashNonce += 1;
    flashTimer = setTimeout(() => {
      flashSection = null;
    }, 1000);
  }

  function openExternal(url: string) {
    openInDefaultApp(url);
  }

  // ── Mark dirty on change ─────────────────────────────────────
  function markDirty() {
    dirty = true;
  }

  // ── Load on mount ────────────────────────────────────────────
  $effect(() => {
    loadAllStatus();
  });

  $effect(() => {
    const observer = new IntersectionObserver(
      (entries) => {
        for (const entry of entries) {
          if (entry.isIntersecting) {
            activeSection = entry.target.id;
          }
        }
      },
      { rootMargin: "-20% 0px -70% 0px" },
    );

    for (const section of sections) {
      const el = document.getElementById(section.id);
      if (el) observer.observe(el);
    }

    return () => observer.disconnect();
  });
</script>

<div class="settings" role="region" aria-label="Settings">
  <div class="top-patch"></div>
  <div class="settings-top">
    <header class="settings-header">
      <h2>Settings</h2>
      <p class="text-muted">
        Configure AI providers for chat and OCR text extraction.
      </p>
    </header>

    <nav class="section-nav" aria-label="Settings sections">
      {#each sections as section}
        <a
          href="#{section.id}"
          class="section-nav-link"
          class:active={activeSection === section.id}
          onclick={(e) => {
            e.preventDefault();
            scrollToSection(section.id);
          }}
        >
          {section.label}
        </a>
      {/each}
    </nav>
  </div>

  {#if statusMessage}
    <div
      class="status-alert"
      class:info={statusLevel === "info"}
      class:success={statusLevel === "success"}
      class:error={statusLevel === "error"}
      role="alert"
      aria-live="polite"
    >
      <span class="material-symbols-outlined">
        {statusLevel === "success"
          ? "check_circle"
          : statusLevel === "error"
            ? "error"
            : "info"}
      </span>
      <span>{statusMessage}</span>
    </div>
  {/if}

  <!-- ── Current Status Summary (F6.1, F6.11) ────────────── -->
  <section class="status-summary" aria-label="Current provider status">
    <div class="summary-card">
      <span class="summary-label">Chat</span>
      <span
        class="summary-value"
        class:reachable={chatTestResult?.success ?? true}
      >
        {app.providerName}
      </span>
      {#if app.providerModel}
        <span class="summary-detail">({app.providerModel})</span>
      {/if}
      {#if lastTestTime}
        <span class="summary-meta">Last tested: {lastTestTime}</span>
      {/if}
    </div>
    <div class="summary-card">
      <span class="summary-label">OCR</span>
      <span class="summary-value">{ocrLabel(ocrBackend)}</span>
      {#if ocrModel}
        <span class="summary-detail">({ocrModel})</span>
      {/if}
    </div>
  </section>

  <!-- ── Chat Provider Section (F6.6 Progressive disclosure) ── -->
  <section
    id="chat-provider"
    class="section"
    aria-label="Chat provider configuration"
  >
    {#key flashSection === "chat-provider" ? flashNonce : null}
      <div
        class="section-header"
        class:flash={flashSection === "chat-provider"}
      >
        <h3>Chat Provider</h3>
        {#if chatSaved}
          <span class="saved-badge" role="status" aria-live="polite">Saved</span
          >
        {/if}
        {#if dirty}
          <span class="dirty-badge" role="status">Unsaved</span>
        {/if}
      </div>
    {/key}

    <div class="quick-start" aria-label="Quick start presets">
      <span class="quick-start-label" id="presets-label">Quick start:</span>
      <div class="preset-buttons" role="group" aria-labelledby="presets-label">
        <button
          class="preset-btn"
          onclick={() => applyPreset("ollama")}
          aria-label="Use Ollama default configuration"
        >
          <span class="material-symbols-outlined">smart_toy</span>
          Ollama
        </button>
        <button
          class="preset-btn"
          onclick={() => applyPreset("lm-studio")}
          aria-label="Use LM Studio default configuration"
        >
          <span class="material-symbols-outlined">terminal</span>
          LM Studio
        </button>
      </div>
    </div>

    <div class="field">
      <label for="chat-provider-kind">
        Provider <span class="required" aria-hidden="true">*</span>
      </label>
      <select
        id="chat-provider-kind"
        bind:value={chatProviderKind}
        onchange={markDirty}
        aria-describedby="chat-provider-hint"
      >
        {#each chatProviderOptions as opt}
          <option value={opt.value}>{opt.label}</option>
        {/each}
      </select>
      <p id="chat-provider-hint" class="field-hint">
        Choose where AI responses come from. Mock works offline without setup.
      </p>
    </div>

    {#if chatProviderKind !== "mock"}
      <div class="field">
        <label for="chat-model">
          Model <span class="required" aria-hidden="true">*</span>
        </label>
        <input
          id="chat-model"
          type="text"
          bind:value={chatModel}
          oninput={markDirty}
          placeholder={chatModelHints[chatProviderKind]}
          aria-describedby="chat-model-hint"
          aria-required="true"
        />
        <p id="chat-model-hint" class="field-hint">
          {chatProviderKind === "ollama"
            ? "Model name you downloaded with `ollama pull`."
            : "Model name from your provider."}
          Examples: {chatModelHints[chatProviderKind]}
        </p>
      </div>

      <!-- Collapsible advanced (F6.6) -->
      <button
        class="advanced-toggle"
        onclick={() => {
          showChatAdvanced = !showChatAdvanced;
        }}
        aria-expanded={showChatAdvanced}
        aria-controls="chat-advanced-section"
      >
        <span class="material-symbols-outlined"
          >{showChatAdvanced ? "expand_less" : "expand_more"}</span
        >
        Advanced
      </button>

      {#if showChatAdvanced}
        <div
          id="chat-advanced-section"
          class="advanced-section"
          role="region"
          aria-label="Chat advanced settings"
        >
          <div class="field">
            <label for="chat-base-url">Base URL</label>
            <input
              id="chat-base-url"
              type="text"
              bind:value={chatBaseUrl}
              oninput={markDirty}
              placeholder={chatUrlDefaults[chatProviderKind]}
              aria-describedby="chat-url-hint"
            />
            <p id="chat-url-hint" class="field-hint">
              {chatProviderKind === "ollama"
                ? "Ollama server address. Default: http://localhost:11434"
                : "OpenAI-compatible endpoint URL. Default: https://api.openai.com/v1"}
            </p>
          </div>

          {#if chatProviderKind === "openai-compatible"}
            <div class="field">
              <label for="chat-api-key">API Key</label>
              <div class="api-key-row">
                <input
                  id="chat-api-key"
                  type={showChatApiKey ? "text" : "password"}
                  bind:value={chatApiKey}
                  oninput={markDirty}
                  placeholder="sk-..."
                  autocomplete="off"
                  aria-describedby="chat-key-hint"
                />
                <button
                  class="icon-btn"
                  onclick={() => {
                    showChatApiKey = !showChatApiKey;
                  }}
                  aria-label={showChatApiKey ? "Hide API key" : "Show API key"}
                >
                  <span class="material-symbols-outlined">
                    {showChatApiKey ? "visibility_off" : "visibility"}
                  </span>
                </button>
                {#if chatApiKey}
                  <button
                    class="icon-btn"
                    onclick={() => {
                      chatApiKey = "";
                      markDirty();
                    }}
                    aria-label="Clear API key"
                  >
                    <span class="material-symbols-outlined">close</span>
                  </button>
                {/if}
              </div>
              <p id="chat-key-hint" class="field-hint">
                Required for cloud providers. Leave blank for local (LM Studio,
                vLLM).
              </p>
            </div>
          {/if}
        </div>
      {/if}

      <div class="actions">
        <button
          class="btn btn-primary"
          onclick={handleSaveChat}
          disabled={chatSaving || !canSaveChat()}
          aria-busy={chatSaving}
        >
          {chatSaving ? "Saving..." : "Save"}
        </button>
        <button
          class="btn btn-secondary"
          onclick={handleTestChat}
          disabled={chatTesting || !canTestChat()}
          aria-busy={chatTesting}
        >
          {chatTesting ? "Testing..." : "Test Connection"}
        </button>
        <button
          class="btn btn-danger-outline"
          onclick={() => {
            showResetConfirm = true;
          }}
          aria-label="Reset to default provider"
        >
          Reset to Default
        </button>
      </div>

      {#if chatTestResult}
        <div
          class="test-result"
          class:success={chatTestResult.success}
          class:failure={!chatTestResult.success}
          role="alert"
          aria-live="polite"
        >
          <span class="material-symbols-outlined">
            {chatTestResult.success ? "check_circle" : "error"}
          </span>
          <div class="test-result-body">
            <span class="test-result-text">
              {#if chatTestResult.success}
                Connected
              {:else}
                Connection failed
              {/if}
            </span>
            {#if chatTestResult.success}
              <span
                class="test-latency"
                class:fast={chatTestResult.latency_ms < 500}
                class:slow={chatTestResult.latency_ms >= 2000}
              >
                {chatTestResult.latency_ms}ms
              </span>
            {:else}
              <span class="test-error-detail">{chatTestResult.error}</span>
            {/if}
          </div>
        </div>
      {/if}

      {#if showResetConfirm}
        <!-- svelte-ignore a11y_click_events_have_key_events, a11y_no_static_element_interactions, a11y_no_noninteractive_element_interactions -->
        <div
          class="modal-overlay"
          onclick={() => {
            showResetConfirm = false;
          }}
          role="presentation"
        >
          <div
            class="modal"
            role="dialog"
            aria-modal="true"
            aria-label="Confirm reset"
            tabindex="-1"
            onclick={(e) => e.stopPropagation()}
          >
            <h4>Reset Chat Provider?</h4>
            <p>
              This will switch back to the Mock provider. Your conversations
              will not be affected.
            </p>
            <div class="modal-actions">
              <button class="btn btn-primary" onclick={handleResetChat}
                >Reset</button
              >
              <button
                class="btn btn-secondary"
                onclick={() => {
                  showResetConfirm = false;
                }}>Cancel</button
              >
            </div>
          </div>
        </div>
      {/if}
    {/if}

    {#if chatProviderKind === "mock"}
      <div class="info-card" role="status">
        <span class="material-symbols-outlined info-icon">info</span>
        <div>
          <p>
            <strong>Mock provider is active.</strong> Responses are simulated — no
            real AI involved.
          </p>
          <p>
            To use a real AI provider, select Ollama or OpenAI-compatible above,
            or use a preset:
          </p>
          <ul class="info-links">
            <li>
              <a
                href="https://ollama.com"
                target="_blank"
                rel="noopener"
                onclick={(e) => {
                  e.preventDefault();
                  openExternal("https://ollama.com");
                }}
                >Download Ollama</a
              > — free, local, no API key needed
            </li>
            <li>
              <a
                href="https://lmstudio.ai"
                target="_blank"
                rel="noopener"
                onclick={(e) => {
                  e.preventDefault();
                  openExternal("https://lmstudio.ai");
                }}
                >Download LM Studio</a
              > — free, local, OpenAI-compatible
            </li>
          </ul>
        </div>
      </div>
    {/if}
  </section>

  <!-- ── OCR Provider Section (F6.4 Consistency) ──────────── -->
  <section id="ocr" class="section" aria-label="OCR provider configuration">
    {#key flashSection === "ocr" ? flashNonce : null}
      <div class="section-header" class:flash={flashSection === "ocr"}>
        <h3>OCR Text Extraction</h3>
        {#if ocrSaved}
          <span class="saved-badge" role="status" aria-live="polite">Saved</span
          >
        {/if}
      </div>
    {/key}

    <div class="field">
      <label for="ocr-backend">
        Backend <span class="required" aria-hidden="true">*</span>
      </label>
      <select
        id="ocr-backend"
        bind:value={ocrBackend}
        aria-describedby="ocr-backend-hint"
      >
        {#each ocrBackendOptions as opt}
          <option value={opt.value}>{opt.label}</option>
        {/each}
      </select>
      <p id="ocr-backend-hint" class="field-hint">
        {ocrBackend === "mock"
          ? "For testing only. No real text extraction."
          : ocrBackend === "tesseract"
            ? "Fast, CPU-only, works offline. Best for simple documents."
            : ocrBackend === "ollama"
              ? "Higher accuracy on complex layouts. Requires Ollama with a vision model."
              : "Highest accuracy. Requires an API key for cloud providers."}
      </p>
    </div>

    {#if ocrBackend !== "mock"}
      <div class="field">
        <label for="ocr-model">
          Model <span class="required" aria-hidden="true">*</span>
        </label>
        <input
          id="ocr-model"
          type="text"
          bind:value={ocrModel}
          placeholder={ocrModelHints[ocrBackend]}
          aria-describedby="ocr-model-hint"
        />
        <p id="ocr-model-hint" class="field-hint">
          Examples: {ocrModelHints[ocrBackend]}
        </p>
      </div>

      {#if ocrBackend !== "tesseract"}
        <button
          class="advanced-toggle"
          onclick={() => {
            showOcrAdvanced = !showOcrAdvanced;
          }}
          aria-expanded={showOcrAdvanced}
          aria-controls="ocr-advanced-section"
        >
          <span class="material-symbols-outlined"
            >{showOcrAdvanced ? "expand_less" : "expand_more"}</span
          >
          Advanced
        </button>

        {#if showOcrAdvanced}
          <div
            id="ocr-advanced-section"
            class="advanced-section"
            role="region"
            aria-label="OCR advanced settings"
          >
            <div class="field">
              <label for="ocr-base-url">Base URL</label>
              <input
                id="ocr-base-url"
                type="text"
                bind:value={ocrBaseUrl}
                placeholder={ocrUrlDefaults[ocrBackend]}
                aria-describedby="ocr-url-hint"
              />
              <p id="ocr-url-hint" class="field-hint">
                Default: {ocrUrlDefaults[ocrBackend]}
              </p>
            </div>

            {#if ocrBackend === "api"}
              <div class="field">
                <label for="ocr-api-key">API Key</label>
                <div class="api-key-row">
                  <input
                    id="ocr-api-key"
                    type={showOcrApiKey ? "text" : "password"}
                    bind:value={ocrApiKey}
                    autocomplete="off"
                  />
                  <button
                    class="icon-btn"
                    onclick={() => {
                      showOcrApiKey = !showOcrApiKey;
                    }}
                    aria-label={showOcrApiKey ? "Hide API key" : "Show API key"}
                  >
                    <span class="material-symbols-outlined">
                      {showOcrApiKey ? "visibility_off" : "visibility"}
                    </span>
                  </button>
                </div>
              </div>
            {/if}
          </div>
        {/if}
      {/if}

      <div class="actions">
        <button
          class="btn btn-primary"
          onclick={handleSaveOcr}
          disabled={ocrSaving || !canSaveOcr()}
        >
          {ocrSaving ? "Saving..." : "Save"}
        </button>
        <button class="btn btn-danger-outline" onclick={handleResetOcr}>
          Reset to Default
        </button>
      </div>
    {/if}

    {#if ocrBackend === "mock"}
      <div class="info-card" role="status">
        <span class="material-symbols-outlined info-icon">info</span>
        <div>
          <p>
            <strong>OCR is set to Mock.</strong> Text will not be extracted from
            images.
          </p>
          <p>
            For real OCR, select Tesseract (local, no setup), Ollama vision, or
            an API provider above.
          </p>
        </div>
      </div>
    {/if}
  </section>

  <!-- ── Import Exclusions ──────────────────────────────────── -->
  <section
    id="import-exclusions"
    class="section"
    aria-label="Import exclusion patterns"
  >
    {#key flashSection === "import-exclusions" ? flashNonce : null}
      <div
        class="section-header"
        class:flash={flashSection === "import-exclusions"}
      >
        <h3>Import Exclusions</h3>
      </div>
    {/key}
    <p class="section-desc">
      Patterns use gitignore syntax — one per line, lines starting with <code
        >#</code
      >
      are comments. If no patterns are configured and the import directory
      contains a <code>.gitignore</code>, it is used as fallback.
    </p>
    <textarea
      class="ignore-textarea"
      bind:value={ignorePatterns}
      rows="12"
      spellcheck="false"
      aria-label="Exclusion patterns"
    ></textarea>
    <div class="actions">
      <button
        class="btn btn-primary"
        onclick={handleSaveIgnore}
        disabled={ignoreSaving}
      >
        {ignoreSaving ? "Saving..." : "Save"}
      </button>
      <button class="btn btn-danger-outline" onclick={handleResetIgnore}>
        Reset to Defaults
      </button>
      {#if ignoreSaved}
        <span class="save-indicator" role="status">Saved</span>
      {/if}
    </div>
  </section>

  <!-- ── Help Section (Nielsen #10, F6.10) ──────────────────── -->
  <section
    id="help"
    class="section help-section"
    aria-label="Help and documentation"
  >
    {#key flashSection === "help" ? flashNonce : null}
      <div class="section-header" class:flash={flashSection === "help"}>
        <h3>Need Help?</h3>
      </div>
    {/key}
    <div class="help-grid">
      <a
        href="https://docs.ollama.com/quickstart"
        class="help-card"
        target="_blank"
        rel="noopener"
        onclick={(e) => {
          e.preventDefault();
          openExternal("https://docs.ollama.com/quickstart");
        }}
      >
        <span class="material-symbols-outlined">smart_toy</span>
        <div>
          <strong>Install Ollama</strong>
          <span class="help-desc">Step-by-step guide for local AI setup</span>
        </div>
      </a>
      <a
        href="https://lmstudio.ai/docs/app/basics"
        class="help-card"
        target="_blank"
        rel="noopener"
        onclick={(e) => {
          e.preventDefault();
          openExternal("https://lmstudio.ai/docs/app/basics");
        }}
      >
        <span class="material-symbols-outlined">terminal</span>
        <div>
          <strong>Install LM Studio</strong>
          <span class="help-desc"
            >Run local models with an OpenAI-compatible server</span
          >
        </div>
      </a>
      <button class="help-card" onclick={() => navigateTo("chat")}>
        <span class="material-symbols-outlined">chat</span>
        <div>
          <strong>Open Chat</strong>
          <span class="help-desc"
            >Start a conversation with your knowledge graph</span
          >
        </div>
      </button>
    </div>
  </section>
</div>

<style>
  .settings {
    max-width: 680px;
    padding-bottom: calc(100vh - 150px);
  }

  .settings-header {
    margin-bottom: var(--spacing-lg);
  }

  .settings h2 {
    font-size: var(--font-size-2xl);
    margin-bottom: var(--spacing-xs);
  }

  .top-patch {
    height: var(--space-6);
    top: 0;
    position: fixed;
    z-index: 10;
    width: 100%;
    background: var(--bg-primary, var(--bg));
  }

  .settings-top {
    position: sticky;
    top: 0;
    z-index: 10;
    background: var(--bg-primary, var(--bg));
    border-bottom: 1px solid var(--border);
    margin-bottom: var(--spacing-md);
  }

  .section-nav {
    display: flex;
    gap: var(--spacing-xs);
    padding: var(--spacing-sm) 0;
  }

  .section-nav-link {
    padding: var(--spacing-xs) var(--spacing-md);
    font-size: var(--font-size-sm);
    font-weight: 500;
    color: var(--text-secondary);
    text-decoration: none;
    border-radius: var(--radius-sm);
    transition: all var(--transition-fast);
    white-space: nowrap;
  }

  .section-nav-link:hover {
    color: var(--text-primary);
    background: var(--bg-secondary);
  }

  .section-nav-link.active {
    color: var(--accent);
    background: color-mix(in srgb, var(--accent) 12%, transparent);
    font-weight: 600;
  }

  .section {
    scroll-margin-top: 132px;
    margin-top: var(--spacing-xl);
    padding-top: var(--spacing-lg);
    border-top: 1px solid var(--border);
  }

  .section-header {
    display: flex;
    align-items: center;
    gap: var(--spacing-sm);
    margin-bottom: var(--spacing-md);
    border-radius: var(--radius-md);
    padding: 4px 8px;
    margin: -4px -8px;
    padding-bottom: 8px;
  }

  .section-header h3 {
    font-size: var(--font-size-lg);
    color: var(--text-primary);
    margin: 0;
  }

  .section-header.flash {
    animation: section-flash 1s ease-out;
  }

  @keyframes section-flash {
    0% {
      background-color: color-mix(in srgb, var(--accent) 25%, transparent);
    }
    100% {
      background-color: transparent;
    }
  }

  @media (prefers-reduced-motion: reduce) {
    .section-header.flash {
      animation: none;
    }
  }

  .section-desc {
    font-size: var(--font-size-sm);
    color: var(--text-secondary);
    margin: 0 0 var(--spacing-md) 0;
    line-height: 1.5;
  }

  .section-desc code {
    font-family: ui-monospace, SFMono-Regular, Menlo, monospace;
    font-size: 0.9em;
    background: var(--bg-tertiary);
    padding: 1px 5px;
    border-radius: var(--radius-sm);
  }

  .ignore-textarea {
    width: 100%;
    font-family: ui-monospace, SFMono-Regular, Menlo, monospace;
    font-size: var(--font-size-sm);
    line-height: 1.5;
    padding: var(--spacing-md);
    background: var(--bg-secondary);
    color: var(--text-primary);
    border: 1px solid var(--border);
    border-radius: var(--radius-md);
    resize: vertical;
    tab-size: 2;
  }

  .ignore-textarea:focus {
    outline: none;
    border-color: var(--accent);
    box-shadow: 0 0 0 2px color-mix(in srgb, var(--accent) 25%, transparent);
  }

  .save-indicator {
    font-size: var(--font-size-sm);
    color: var(--accent);
    font-weight: 600;
    padding: 1px 8px;
    background: color-mix(in srgb, var(--accent) 15%, transparent);
    border-radius: var(--radius-sm);
  }

  .saved-badge {
    font-size: var(--font-size-sm);
    color: var(--accent);
    font-weight: 600;
    padding: 1px 8px;
    background: color-mix(in srgb, var(--accent) 15%, transparent);
    border-radius: var(--radius-sm);
  }

  .dirty-badge {
    font-size: var(--font-size-sm);
    color: #dd6b20;
    font-weight: 600;
    padding: 1px 8px;
    background: color-mix(in srgb, #dd6b20 15%, transparent);
    border-radius: var(--radius-sm);
  }

  .status-alert {
    display: flex;
    align-items: center;
    gap: var(--spacing-sm);
    padding: var(--spacing-sm) var(--spacing-md);
    border-radius: var(--radius-sm);
    font-size: var(--font-size-sm);
    font-weight: 500;
    margin-bottom: var(--spacing-md);
  }

  .status-alert.info {
    background: color-mix(in srgb, var(--accent) 12%, transparent);
    color: var(--accent);
  }

  .status-alert.success {
    background: color-mix(in srgb, #38a169 12%, transparent);
    color: #38a169;
  }

  .status-alert.error {
    background: color-mix(in srgb, #e53e3e 12%, transparent);
    color: #e53e3e;
  }

  .status-summary {
    display: flex;
    gap: var(--spacing-md);
    margin-bottom: var(--spacing-lg);
  }

  .summary-card {
    display: flex;
    align-items: center;
    gap: var(--spacing-sm);
    flex: 1;
    padding: var(--spacing-md);
    background: var(--bg-card);
    border: 1px solid var(--border);
    border-radius: var(--radius-md);
  }

  .summary-label {
    font-size: var(--font-size-sm);
    font-weight: 600;
    color: var(--text-secondary);
    text-transform: uppercase;
    letter-spacing: 0.03em;
  }

  .summary-value {
    font-weight: 600;
    color: var(--text-primary);
  }

  .summary-value.reachable {
    color: var(--accent);
  }

  .summary-detail {
    font-size: var(--font-size-sm);
    color: var(--text-secondary);
  }

  .summary-meta {
    font-size: var(--font-size-xs);
    color: var(--text-secondary);
    opacity: 0.7;
    margin-left: auto;
  }

  .quick-start {
    display: flex;
    align-items: center;
    gap: var(--spacing-sm);
    margin-bottom: var(--spacing-md);
    padding: var(--spacing-sm) var(--spacing-md);
    background: var(--bg-card);
    border: 1px solid var(--border);
    border-radius: var(--radius-sm);
  }

  .quick-start-label {
    font-size: var(--font-size-sm);
    color: var(--text-secondary);
    font-weight: 600;
    flex-shrink: 0;
  }

  .preset-buttons {
    display: flex;
    gap: var(--spacing-xs);
  }

  .preset-btn {
    display: flex;
    align-items: center;
    gap: 4px;
    padding: 4px 12px;
    border: 1px solid var(--border);
    border-radius: var(--radius-sm);
    background: var(--bg-card);
    color: var(--text-primary);
    font-size: var(--font-size-sm);
    cursor: pointer;
    transition: all var(--transition-fast);
  }

  .preset-btn:hover {
    background: var(--accent);
    color: white;
    border-color: var(--accent);
  }

  .preset-btn .material-symbols-outlined {
    font-size: 16px;
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

  .required {
    color: #e53e3e;
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
    transition: border-color var(--transition-fast);
  }

  .field select {
    cursor: pointer;
  }

  .field input:focus,
  .field select:focus {
    outline: none;
    border-color: var(--accent);
    box-shadow: 0 0 0 2px color-mix(in srgb, var(--accent) 25%, transparent);
  }

  .field-hint {
    margin-top: 4px;
    font-size: var(--font-size-xs);
    color: var(--text-secondary);
    line-height: 1.4;
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
    transition: all var(--transition-fast);
  }

  .icon-btn:hover {
    background: var(--bg-secondary);
    color: var(--text-primary);
  }

  .advanced-toggle {
    display: flex;
    align-items: center;
    gap: 4px;
    padding: var(--spacing-xs) var(--spacing-sm);
    background: none;
    border: none;
    color: var(--accent);
    font-size: var(--font-size-sm);
    font-weight: 500;
    cursor: pointer;
    margin-bottom: var(--spacing-md);
  }

  .advanced-toggle:hover {
    opacity: 0.8;
  }

  .advanced-toggle .material-symbols-outlined {
    font-size: 16px;
  }

  .advanced-section {
    padding: var(--spacing-md);
    background: var(--bg-secondary);
    border-radius: var(--radius-sm);
    margin-bottom: var(--spacing-md);
  }

  .actions {
    display: flex;
    gap: var(--spacing-sm);
    margin-top: var(--spacing-lg);
    flex-wrap: wrap;
    padding-bottom: var(--spacing-md);
  }

  .btn {
    padding: var(--spacing-sm) var(--spacing-lg);
    border-radius: var(--radius-sm);
    font-size: var(--font-size-body-md);
    font-weight: 600;
    cursor: pointer;
    border: none;
    transition: all var(--transition-fast);
  }

  .btn:disabled {
    opacity: 0.45;
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

  .btn-danger-outline {
    background: transparent;
    color: #e53e3e;
    border: 1px solid #e53e3e;
  }

  .btn-danger-outline:hover:not(:disabled) {
    background: color-mix(in srgb, #e53e3e 10%, transparent);
  }

  .test-result {
    display: flex;
    align-items: flex-start;
    gap: var(--spacing-sm);
    margin-top: var(--spacing-md);
    padding: var(--spacing-md);
    border-radius: var(--radius-sm);
    font-size: var(--font-size-sm);
    font-weight: 500;
    line-height: 1.5;
  }

  .test-result.success {
    background: color-mix(in srgb, #38a169 12%, transparent);
    color: #38a169;
  }

  .test-result.failure {
    background: color-mix(in srgb, #e53e3e 12%, transparent);
    color: #e53e3e;
  }

  .test-result-body {
    display: flex;
    flex-direction: column;
    gap: 2px;
  }

  .test-result-text {
    font-weight: 600;
  }

  .test-latency {
    font-weight: 400;
    opacity: 0.8;
  }

  .test-latency.fast {
    color: #38a169;
  }

  .test-latency.slow {
    color: #dd6b20;
  }

  .test-error-detail {
    font-weight: 400;
    opacity: 0.9;
    word-break: break-word;
  }

  .info-card {
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
    line-height: 1.6;
  }

  .info-card p {
    margin: 0 0 var(--spacing-xs);
  }

  .info-card p:last-child {
    margin-bottom: 0;
  }

  .info-card strong {
    color: var(--text-primary);
  }

  .info-icon {
    font-size: 20px;
    flex-shrink: 0;
    margin-top: 1px;
  }

  .info-links {
    margin: var(--spacing-xs) 0 0;
    padding-left: 1.2em;
  }

  .info-links li {
    margin-bottom: 2px;
  }

  .info-links a {
    color: var(--accent);
    text-decoration: none;
  }

  .info-links a:hover {
    text-decoration: underline;
  }

  .modal-overlay {
    position: fixed;
    top: 0;
    left: 0;
    right: 0;
    bottom: 0;
    background: rgba(0, 0, 0, 0.5);
    display: flex;
    align-items: center;
    justify-content: center;
    z-index: 100;
  }

  .modal {
    background: var(--bg-card);
    border: 1px solid var(--border);
    border-radius: var(--radius-md);
    padding: var(--spacing-lg);
    max-width: 400px;
    width: 90%;
  }

  .modal h4 {
    margin: 0 0 var(--spacing-sm);
    font-size: var(--font-size-lg);
    color: var(--text-primary);
  }

  .modal p {
    font-size: var(--font-size-sm);
    color: var(--text-secondary);
    margin-bottom: var(--spacing-lg);
    line-height: 1.5;
  }

  .modal-actions {
    display: flex;
    gap: var(--spacing-sm);
    justify-content: flex-end;
  }

  .help-section {
    margin-top: var(--spacing-2xl);
  }

  .help-grid {
    display: flex;
    flex-direction: column;
    gap: var(--spacing-sm);
    margin-top: var(--spacing-md);
  }

  .help-card {
    display: flex;
    align-items: center;
    gap: var(--spacing-md);
    padding: var(--spacing-md);
    background: var(--bg-card);
    border: 1px solid var(--border);
    border-radius: var(--radius-sm);
    text-decoration: none;
    color: var(--text-primary);
    cursor: pointer;
    transition: all var(--transition-fast);
    text-align: left;
  }

  .help-card:hover {
    border-color: var(--accent);
    background: var(--bg-secondary);
  }

  .help-card .material-symbols-outlined {
    font-size: 24px;
    color: var(--accent);
  }

  .help-card strong {
    display: block;
    margin-bottom: 2px;
  }

  .help-desc {
    font-size: var(--font-size-sm);
    color: var(--text-secondary);
  }
</style>
