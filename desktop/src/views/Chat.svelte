<script lang="ts">
  import { onMount, onDestroy, tick } from "svelte";
  import { getState } from "../lib/state.svelte.js";
  import { navigateTo } from "../lib/router.svelte.js";
  import {
    chatSend,
    chatSearchEntities,
    chatListConversations,
    chatDeleteConversation,
    chatRenameConversation,
    chatStopStream,
    chatSendFeedback,
  } from "../lib/api.js";
  import { ChatStreamSession } from "../lib/chat-stream.js";
  import type {
    ChatMessage,
    Citation,
    ChatDelta,
    ProcessingStatus,
    ConversationSummary,
    EntitySearchResult,
    MessageDisplay,
    FeedbackRating,
  } from "../lib/types.js";

  const app = getState();

  // --- Conversation state ---
  let conversations = $state<ConversationSummary[]>([]);
  let currentConversationId = $state<string | null>(null);
  let messages = $state<MessageDisplay[]>([]);
  let loadingConversations = $state(false);
  let loadingMessages = $state(false);

  // --- Input state ---
  let inputText = $state("");
  let mode = $state<"fast" | "thinking">("thinking");
  let knowledgeGraph = $state(true);
  let webSearch = $state(false);
  let streaming = $state(false);
  let processingStatus = $state<ProcessingStatus | null>(null);
  let errorMessage = $state<string | null>(null);

  // --- Entity ref picker ---
  let entityRefs = $state<string[]>([]);
  let entityRefChips = $state<EntitySearchResult[]>([]);
  let showEntityDropdown = $state(false);
  let entitySearchResults = $state<EntitySearchResult[]>([]);
  let entitySearchPrefix = $state("");
  let entityInputValue = $state("");

  // --- Context menus ---
  let contextMenuConvId = $state<string | null>(null);
  let contextMenuX = $state(0);
  let contextMenuY = $state(0);
  let renamingConvId = $state<string | null>(null);
  let renameValue = $state("");
  let deletingConvId = $state<string | null>(null);

  // --- Feedback ---
  let feedbackForm = $state<{
    messageId: string;
    showForm: boolean;
    reason: string;
    comment: string;
  } | null>(null);

  // --- Streaming ---
  let streamSession = $state<ChatStreamSession | null>(null);
  let messagesContainer: HTMLDivElement | undefined = $state();
  let sendCounter = 0;

  // --- Timers ---
  let statusTimer: ReturnType<typeof setTimeout> | null = null;

  onMount(() => {
    loadConversations();
  });

  onDestroy(() => {
    streamSession?.stop();
    if (statusTimer) clearTimeout(statusTimer);
  });

  // --- Conversation management ---

  async function loadConversations() {
    loadingConversations = true;
    try {
      conversations = await chatListConversations();
    } catch (e) {
      app.statusMessage = `Failed to load conversations: ${e}`;
    } finally {
      loadingConversations = false;
    }
  }

  async function selectConversation(id: string) {
    if (streaming) return;
    currentConversationId = id;
    loadingMessages = true;
    messages = [];
    errorMessage = null;
    try {
    } catch (e) {
      errorMessage = `Failed to load conversation: ${e}`;
    } finally {
      loadingMessages = false;
    }
  }

  async function newConversation() {
    if (streaming) return;
    currentConversationId = null;
    messages = [];
    inputText = "";
    entityRefs = [];
    entityRefChips = [];
    errorMessage = null;
    processingStatus = null;
  }

  function openContextMenu(e: MouseEvent, convId: string) {
    e.preventDefault();
    contextMenuConvId = convId;
    contextMenuX = e.clientX;
    contextMenuY = e.clientY;
  }

  function closeContextMenu() {
    contextMenuConvId = null;
    renamingConvId = null;
    deletingConvId = null;
  }

  function startRename(convId: string) {
    const conv = conversations.find((c) => c.id === convId);
    if (!conv) return;
    renamingConvId = convId;
    renameValue = conv.title;
    contextMenuConvId = null;
  }

  async function confirmRename() {
    if (!renamingConvId || !renameValue.trim()) return;
    try {
      await chatRenameConversation(renamingConvId, renameValue.trim());
      await loadConversations();
    } catch (e) {
      app.statusMessage = `Failed to rename: ${e}`;
    } finally {
      renamingConvId = null;
    }
  }

  function confirmDelete(convId: string) {
    deletingConvId = convId;
    contextMenuConvId = null;
  }

  async function doDelete() {
    if (!deletingConvId) return;
    try {
      await chatDeleteConversation(deletingConvId);
      if (currentConversationId === deletingConvId) {
        currentConversationId = null;
        messages = [];
      }
      await loadConversations();
    } catch (e) {
      app.statusMessage = `Failed to delete: ${e}`;
    } finally {
      deletingConvId = null;
    }
  }

  // --- Entity ref picker ---

  async function onEntityInput(e: Event) {
    const value = (e.target as HTMLInputElement).value;
    entityInputValue = value;

    const atIndex = value.lastIndexOf("@");
    if (atIndex >= 0) {
      const prefix = value.slice(atIndex + 1);
      entitySearchPrefix = prefix;
      if (prefix.length >= 1) {
        try {
          entitySearchResults = await chatSearchEntities(prefix);
          showEntityDropdown = entitySearchResults.length > 0;
        } catch {
          showEntityDropdown = false;
        }
      } else {
        showEntityDropdown = false;
      }
    } else {
      showEntityDropdown = false;
    }
  }

  function selectEntityRef(entity: EntitySearchResult) {
    if (entityRefs.includes(entity.id)) return;
    entityRefs = [...entityRefs, entity.id];
    entityRefChips = [...entityRefChips, entity];

    const atIndex = entityInputValue.lastIndexOf("@");
    if (atIndex >= 0) {
      entityInputValue = entityInputValue.slice(0, atIndex);
    }
    showEntityDropdown = false;
  }

  function removeEntityRef(id: string) {
    entityRefs = entityRefs.filter((r) => r !== id);
    entityRefChips = entityRefChips.filter((c) => c.id !== id);
  }

  // --- Send / Stream ---

  async function sendMessage() {
    const text = inputText.trim();
    if (!text) return;
    if (streaming) return;

    errorMessage = null;
    processingStatus = null;

    const msgId = ++sendCounter;
    const userMsg: MessageDisplay = {
      id: `user-${msgId}`,
      role: "user",
      content: text,
      timestamp: new Date().toISOString(),
    };
    messages = [...messages, userMsg];
    inputText = "";
    entityInputValue = "";

    const assistantMsg: MessageDisplay = {
      id: `assistant-${msgId}`,
      role: "assistant",
      content: "",
      timestamp: new Date().toISOString(),
    };
    messages = [...messages, assistantMsg];
    streaming = true;

    scrollToBottom();

    try {
      const session = new ChatStreamSession();
      streamSession = session;

      await session.start(
        currentConversationId,
        text,
        entityRefs,
        { knowledge_graph: knowledgeGraph, web_search: webSearch },
        mode,
        {
          onStatus: (status) => {
            processingStatus = status;
            clearStatusTimer();
          },
          onDelta: (delta: ChatDelta) => {
            if (delta.status) {
              processingStatus = delta.status;
              clearStatusTimer();
            }
            if (delta.delta) {
              messages = messages.map((m, i) =>
                i === messages.length - 1
                  ? { ...m, content: m.content + delta.delta }
                  : m
              );
              scrollToBottom();
            }
            if (delta.finished) {
              messages = messages.map((m, i) =>
                i === messages.length - 1
                  ? { ...m, citations: [] }
                  : m
              );
            }
          },
          onDone: (info) => {
            messages = messages.map((m, i) =>
              i === messages.length - 1
                ? { ...m, id: info.assistantMessageId, citations: info.citations }
                : m
            );
            streaming = false;
            streamSession = null;
            processingStatus = null;
            entityRefs = [];
            entityRefChips = [];
            currentConversationId = session.getConversationId();
            loadConversations();
            scrollToBottom();
          },
          onError: (err) => {
            errorMessage = err;
            streaming = false;
            streamSession = null;
            processingStatus = null;
            if (messages.length > 0 && messages[messages.length - 1].role === "assistant" && !messages[messages.length - 1].content) {
              messages = messages.slice(0, -1);
            }
          },
        }
      );
    } catch (e) {
      await fallbackSend(text);
    }
  }

  async function fallbackSend(text: string) {
    if (messages.length > 0 && messages[messages.length - 1].role === "assistant" && !messages[messages.length - 1].content) {
      messages = messages.slice(0, -1);
    }
    try {
      const result = await chatSend(
        currentConversationId,
        text,
        entityRefs,
        { knowledge_graph: knowledgeGraph, web_search: webSearch },
        mode
      );
      const assistantMsg: MessageDisplay = {
        id: result.message_id,
        role: "assistant",
        content: result.message,
        citations: result.citations,
        referencedEntities: result.referenced_entities,
        timestamp: new Date().toISOString(),
      };
      messages = [...messages, assistantMsg];
      currentConversationId = result.conversation_id;
      entityRefs = [];
      entityRefChips = [];
      loadConversations();
      scrollToBottom();
    } catch (e) {
      errorMessage = `Send failed: ${e}`;
      if (messages.length > 0 && messages[messages.length - 1].role === "user") {
        messages = messages.slice(0, -1);
      }
    } finally {
      streaming = false;
      streamSession = null;
      processingStatus = null;
    }
  }

  async function stopStreaming() {
    if (streamSession) {
      await streamSession.stop();
      streamSession = null;
    }
    streaming = false;
    processingStatus = null;
    if (messages.length > 0 && messages[messages.length - 1].role === "assistant" && !messages[messages.length - 1].content) {
      messages = messages.slice(0, -1);
    }
  }

  function retryLastMessage() {
    const lastUserMsg = [...messages].reverse().find((m) => m.role === "user");
    if (lastUserMsg) {
      inputText = lastUserMsg.content;
      messages = messages.filter((m) => m.id !== lastUserMsg.id);
      sendMessage();
    }
  }

  function clearStatusTimer() {
    if (statusTimer) clearTimeout(statusTimer);
    statusTimer = setTimeout(() => {
      processingStatus = null;
    }, 10000);
  }

  // --- Feedback ---

  async function sendFeedback(messageId: string, rating: FeedbackRating, reason?: string, comment?: string) {
    try {
      await chatSendFeedback({
        message_id: messageId,
        rating,
        reason,
        comment: comment || undefined,
      });
      messages = messages.map((m) =>
        m.id === messageId ? { ...m, feedback: rating } : m
      );
      feedbackForm = null;
    } catch (e) {
      app.statusMessage = `Failed to send feedback: ${e}`;
    }
  }

  function openFeedbackForm(messageId: string) {
    feedbackForm = { messageId, showForm: true, reason: "", comment: "" };
  }

  function closeFeedbackForm() {
    feedbackForm = null;
  }

  async function submitFeedbackForm() {
    if (!feedbackForm) return;
    await sendFeedback(
      feedbackForm.messageId,
      "thumbs_down",
      feedbackForm.reason || undefined,
      feedbackForm.comment || undefined
    );
  }

  // --- Scroll ---

  function scrollToBottom() {
    tick().then(() => {
      if (messagesContainer) {
        messagesContainer.scrollTop = messagesContainer.scrollHeight;
      }
    });
  }

  // --- Markdown rendering ---

  function renderMarkdown(content: string): string {
    let html = content;

    html = html.replace(/&/g, "&amp;").replace(/</g, "&lt;").replace(/>/g, "&gt;");

    html = html.replace(/```([\s\S]*?)```/g, (_, code) => {
      const escaped = code
        .replace(/&/g, "&amp;")
        .replace(/</g, "&lt;")
        .replace(/>/g, "&gt;");
      return `<pre class="chat-code-block"><code>${escaped}</code></pre>`;
    });

    html = html.replace(/`([^`]+)`/g, '<code class="chat-inline-code">$1</code>');

    html = html.replace(/\[([^\]]+)\]\(([^)]+)\)/g, '<a href="$2" target="_blank" rel="noopener" class="chat-link">$1</a>');

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
        result.push(`<p>${line}</p>`);
      }
    }
    if (inList) {
      result.push(`</${listType}>`);
    }

    return result.join("\n");
  }

  function renderWithCitations(content: string, citations?: Citation[]): string {
    if (!citations || citations.length === 0) return renderMarkdown(content);

    let html = content;
    html = html.replace(/&/g, "&amp;").replace(/</g, "&lt;").replace(/>/g, "&gt;");

    html = html.replace(/\[(\d+)\]/g, (_, num) => {
      const citation = citations.find((c) => c.number === parseInt(num));
      if (citation) {
        return `<sup class="chat-citation" data-entity-id="${citation.entity_id}" onclick="window.__chatCitationClick('${citation.entity_id}')">[${num}]</sup>`;
      }
      return `<sup class="chat-citation">[${num}]</sup>`;
    });

    html = html.replace(/```([\s\S]*?)```/g, (_, code) => {
      const escaped = code
        .replace(/&/g, "&amp;")
        .replace(/</g, "&lt;")
        .replace(/>/g, "&gt;");
      return `<pre class="chat-code-block"><code>${escaped}</code></pre>`;
    });

    html = html.replace(/`([^`]+)`/g, '<code class="chat-inline-code">$1</code>');

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
        result.push(`<p>${line}</p>`);
      }
    }
    if (inList) {
      result.push(`</${listType}>`);
    }

    return result.join("\n");
  }

  function statusText(status: ProcessingStatus): string {
    if (status.Searching) return `Searching: ${status.Searching.detail}`;
    if (status.ReadingEntities) return `Reading entities (${status.ReadingEntities.count})`;
    if (status.Generating !== undefined) return "Generating...";
    return "";
  }

  function formatTime(ts: string): string {
    try {
      const d = new Date(ts);
      return d.toLocaleTimeString(undefined, { hour: "2-digit", minute: "2-digit" });
    } catch {
      return "";
    }
  }

  function formatDate(ts: string): string {
    try {
      const d = new Date(ts);
      const now = new Date();
      const isToday = d.toDateString() === now.toDateString();
      if (isToday) return formatTime(ts);
      return d.toLocaleDateString(undefined, { month: "short", day: "numeric" }) + " " + formatTime(ts);
    } catch {
      return "";
    }
  }

  function openEntityDetail(entityId: string) {
    navigateTo("detail", entityId);
  }

  function handleKeyDown(e: KeyboardEvent) {
    if (e.key === "Enter" && !e.shiftKey) {
      e.preventDefault();
      sendMessage();
    }
  }

  // --- Actions ---
  function focusOnMount(node: HTMLInputElement) {
    node.focus();
    node.select();
  }
</script>

<div class="chat-view">
  <!-- Left pane: conversation list -->
  <aside class="conv-sidebar">
    <button class="new-chat-btn" onclick={newConversation} disabled={streaming}>
      <span class="material-symbols-outlined">add</span>
      New Chat
    </button>

    <div class="conv-list">
      {#if loadingConversations}
        <div class="conv-list-loading">
          <span class="material-symbols-outlined loading-spinner">sync</span>
        </div>
      {:else if conversations.length === 0}
        <p class="conv-list-empty">No conversations yet.<br />Start a new chat!</p>
      {:else}
        {#each conversations as conv}
          <div
            class="conv-item"
            class:active={conv.id === currentConversationId}
            class:conv-disabled={streaming}
            onclick={() => { if (!streaming) selectConversation(conv.id); }}
            oncontextmenu={(e) => openContextMenu(e, conv.id)}
            role="button"
            tabindex="0"
            onkeydown={(e) => { if (e.key === "Enter" && !streaming) selectConversation(conv.id); }}
          >
            <div class="conv-item-content">
              <span class="conv-title truncate">
                {#if renamingConvId === conv.id}
                  <input
                    type="text"
                    bind:value={renameValue}
                    onkeydown={(e) => { if (e.key === "Enter") confirmRename(); if (e.key === "Escape") renamingConvId = null; }}
                    onblur={confirmRename}
                    use:focusOnMount
                    class="rename-input"
                  />
                {:else}
                  {conv.title || "Untitled"}
                {/if}
              </span>
              {#if conv.last_message_preview}
                <span class="conv-preview truncate">{conv.last_message_preview}</span>
              {/if}
              <span class="conv-meta">
                {conv.message_count} msg &middot; {formatDate(conv.last_message_at || conv.updated_at)}
              </span>
            </div>
            <button
              class="conv-menu-btn"
              onclick={(e) => { e.stopPropagation(); openContextMenu(e, conv.id); }}
            >
              <span class="material-symbols-outlined">more_vert</span>
            </button>
          </div>
        {/each}
      {/if}
    </div>

    <!-- Context menu -->
    {#if contextMenuConvId}
      <!-- svelte-ignore a11y_click_events_have_key_events -->
      <div class="context-menu-backdrop" onclick={closeContextMenu} oncontextmenu={(e) => e.preventDefault()} role="presentation"></div>
      <div
        class="context-menu"
        style="left: {contextMenuX}px; top: {contextMenuY}px;"
      >
        <button class="context-menu-item" onclick={() => startRename(contextMenuConvId!)}>
          <span class="material-symbols-outlined">edit</span>
          Rename
        </button>
        <button class="context-menu-item danger" onclick={() => confirmDelete(contextMenuConvId!)}>
          <span class="material-symbols-outlined">delete</span>
          Delete
        </button>
      </div>
    {/if}

    <!-- Delete confirmation -->
    {#if deletingConvId}
      <!-- svelte-ignore a11y_click_events_have_key_events -->
      <div class="modal-backdrop" onclick={() => deletingConvId = null} onkeydown={(e) => { if (e.key === 'Escape') deletingConvId = null; }} role="presentation"></div>
      <div class="modal">
        <h3>Delete conversation?</h3>
        <p>This action cannot be undone.</p>
        <div class="modal-actions">
          <button class="btn btn-secondary" onclick={() => deletingConvId = null}>Cancel</button>
          <button class="btn btn-danger" onclick={doDelete}>Delete</button>
        </div>
      </div>
    {/if}
  </aside>

  <!-- Right pane: message thread -->
  <div class="chat-main">
    {#if !currentConversationId && messages.length === 0}
      <div class="chat-welcome">
        <div class="welcome-icon">
          <span class="material-symbols-outlined" style="font-size: 48px;">chat</span>
        </div>
        <h2>Knowledge OS Chat</h2>
        <p>Ask questions about your knowledge graph, reference entities with @, and get AI-powered answers.</p>
        <div class="welcome-tips">
          <div class="tip">
            <span class="material-symbols-outlined">travel_explore</span>
            <span>Search your knowledge graph with AI</span>
          </div>
          <div class="tip">
            <span class="material-symbols-outlined">link</span>
            <span>Reference entities using @ mentions</span>
          </div>
          <div class="tip">
            <span class="material-symbols-outlined">psychology</span>
            <span>Toggle between Fast and Thinking modes</span>
          </div>
        </div>
      </div>
    {:else}
      <div class="messages-container" bind:this={messagesContainer}>
        {#each messages as msg}
          <div class="message" class:message-user={msg.role === "user"} class:message-assistant={msg.role === "assistant" || msg.role === "system"}>
            <div class="message-bubble">
              {#if msg.role === "user"}
                <div class="message-content">
                  {msg.content}
                </div>
              {:else}
                <div class="message-content chat-markdown">
                  {@html renderWithCitations(msg.content, msg.citations)}
                </div>
              {/if}

              <div class="message-timestamp">{formatTime(msg.timestamp)}</div>

              <!-- Citations section -->
              {#if msg.citations && msg.citations.length > 0}
                <div class="message-citations">
                  <div class="citations-title">Sources</div>
                  {#each msg.citations as citation}
                    <button
                      class="citation-item"
                      onclick={() => openEntityDetail(citation.entity_id)}
                    >
                      <span class="citation-number">[{citation.number}]</span>
                      <span class="citation-type">{citation.entity_type}</span>
                      <span class="citation-title">{citation.title}</span>
                    </button>
                  {/each}
                </div>
              {/if}

              <!-- Feedback buttons -->
              {#if msg.role === "assistant" && msg.content}
                <div class="message-feedback">
                  <button
                    class="feedback-btn"
                    class:feedback-active={msg.feedback === "thumbs_up"}
                    onclick={() => {
                      if (msg.feedback === "thumbs_up") return;
                      sendFeedback(msg.id, "thumbs_up");
                    }}
                    title="Thumbs up"
                  >
                    <span class="material-symbols-outlined">thumb_up</span>
                  </button>
                  <button
                    class="feedback-btn"
                    class:feedback-active={msg.feedback === "thumbs_down"}
                    onclick={() => {
                      if (msg.feedback === "thumbs_down") return;
                      if (feedbackForm?.messageId === msg.id) {
                        closeFeedbackForm();
                      } else {
                        openFeedbackForm(msg.id);
                      }
                    }}
                    title="Thumbs down"
                  >
                    <span class="material-symbols-outlined">thumb_down</span>
                  </button>
                </div>
              {/if}

              <!-- Feedback form (thumbs down) -->
              {#if feedbackForm && feedbackForm.messageId === msg.id}
                <div class="feedback-form">
                  <p class="feedback-form-title">What went wrong?</p>
                  <div class="feedback-reasons">
                    {#each ["Wrong Entity", "Missing Info", "Wrong Citation", "Other"] as reason}
                      <button
                        class="feedback-reason-btn"
                        class:selected={feedbackForm.reason === reason}
                        onclick={() => { if (feedbackForm) feedbackForm = { ...feedbackForm, reason }; }}
                      >
                        {reason}
                      </button>
                    {/each}
                  </div>
                  <textarea
                    class="feedback-comment"
                    placeholder="Additional comment (optional)..."
                    bind:value={feedbackForm.comment}
                    rows={2}
                  ></textarea>
                  <div class="feedback-actions">
                    <button class="btn btn-sm" onclick={closeFeedbackForm}>Cancel</button>
                    <button class="btn btn-sm btn-primary" onclick={submitFeedbackForm}>Submit</button>
                  </div>
                </div>
              {/if}
            </div>
          </div>
        {/each}

        <!-- Typing / status indicator -->
        {#if streaming}
          <div class="message message-assistant">
            <div class="message-bubble">
              <div class="typing-indicator">
                {#if processingStatus}
                  <span class="status-text">{statusText(processingStatus)}</span>
                {:else}
                  <span class="typing-dots">
                    <span class="dot">.</span>
                    <span class="dot">.</span>
                    <span class="dot">.</span>
                  </span>
                {/if}
              </div>
            </div>
          </div>
        {/if}
      </div>
    {/if}

    <!-- Error message -->
    {#if errorMessage}
      <div class="error-banner">
        <span class="material-symbols-outlined">error</span>
        <span>{errorMessage}</span>
        {#if messages.some((m) => m.role === "user")}
          <button class="btn btn-sm" onclick={retryLastMessage}>Retry</button>
        {/if}
        <button class="btn btn-sm" onclick={() => errorMessage = null}>Dismiss</button>
      </div>
    {/if}

    <!-- Input area -->
    <div class="input-area" class:input-area-streaming={streaming}>
      <!-- Entity ref chips -->
      {#if entityRefChips.length > 0}
        <div class="entity-chips">
          {#each entityRefChips as chip}
            <span class="entity-chip">
              <span class="chip-type">{chip.entity_type}</span>
              <span class="chip-title">{chip.title}</span>
              <button class="chip-remove" onclick={() => removeEntityRef(chip.id)}>
                <span class="material-symbols-outlined" style="font-size: 14px;">close</span>
              </button>
            </span>
          {/each}
        </div>
      {/if}

      <!-- Entity dropdown -->
      <div class="entity-dropdown-container">
        {#if showEntityDropdown}
          <div class="entity-dropdown">
            {#each entitySearchResults as entity}
              <button
                class="entity-dropdown-item"
                onclick={() => selectEntityRef(entity)}
              >
                <span class="dropdown-item-type">{entity.entity_type}</span>
                <span class="dropdown-item-title">{entity.title}</span>
                <span class="dropdown-item-preview truncate">{entity.preview}</span>
              </button>
            {/each}
          </div>
        {/if}
      </div>

      <!-- Input row: toggles -->
      <div class="input-toggles">
        <div class="mode-toggle">
          <button
            class="mode-btn"
            class:mode-active={mode === "fast"}
            onclick={() => mode = "fast"}
            disabled={streaming}
          >
            <span class="material-symbols-outlined">bolt</span>
            Fast
          </button>
          <button
            class="mode-btn"
            class:mode-active={mode === "thinking"}
            onclick={() => mode = "thinking"}
            disabled={streaming}
          >
            <span class="material-symbols-outlined">psychology</span>
            Thinking
          </button>
        </div>
        <div class="source-toggles">
          <label class="toggle-label" class:toggle-disabled={streaming}>
            <input type="checkbox" bind:checked={knowledgeGraph} disabled={streaming} />
            <span>Knowledge Graph</span>
          </label>
          <label class="toggle-label" class:toggle-disabled={streaming}>
            <input type="checkbox" bind:checked={webSearch} disabled={streaming} />
            <span>Web Search</span>
          </label>
        </div>
      </div>

      <!-- Input row: text + button -->
      <div class="input-row">
        <div class="text-input-wrapper">
          <span class="material-symbols-outlined input-entity-icon">alternate_email</span>
          <input
            type="text"
            class="text-input"
            placeholder="Ask a question... (use @ to reference entities)"
            bind:value={entityInputValue}
            oninput={onEntityInput}
            onkeydown={handleKeyDown}
            disabled={streaming}
          />
        </div>
        {#if streaming}
          <button class="stop-btn" onclick={stopStreaming} title="Stop generating">
            <span class="material-symbols-outlined">stop</span>
          </button>
        {:else}
          <button
            class="send-btn"
            onclick={sendMessage}
            disabled={streaming || !entityInputValue.trim()}
            title="Send message"
          >
            <span class="material-symbols-outlined">send</span>
          </button>
        {/if}
      </div>
    </div>
  </div>
</div>

<style>
  /* ===== Layout ===== */
  .chat-view {
    display: flex;
    height: 100%;
    gap: 0;
    position: relative;
  }

  /* ===== Left sidebar ===== */
  .conv-sidebar {
    width: 280px;
    min-width: 280px;
    display: flex;
    flex-direction: column;
    border-right: 1px solid var(--border);
    background: var(--color-surface-container-low);
    overflow: hidden;
  }

  .new-chat-btn {
    display: flex;
    align-items: center;
    gap: var(--spacing-sm);
    padding: var(--spacing-md);
    margin: var(--spacing-md);
    background: var(--accent);
    color: white;
    border-radius: var(--radius-md);
    font-weight: 600;
    font-size: var(--font-size-md);
    transition: background var(--transition-fast);
  }

  .new-chat-btn:hover:not(:disabled) {
    background: var(--accent-hover);
  }

  .new-chat-btn:disabled {
    opacity: 0.5;
    cursor: not-allowed;
  }

  .conv-list {
    flex: 1;
    overflow-y: auto;
    padding: 0 var(--spacing-sm);
  }

  .conv-list-loading {
    display: flex;
    justify-content: center;
    padding: var(--spacing-xl);
  }

  .conv-list-empty {
    color: var(--text-secondary);
    text-align: center;
    padding: var(--spacing-xl);
    font-size: var(--font-size-sm);
    line-height: 1.6;
  }

  .conv-item {
    display: flex;
    align-items: flex-start;
    gap: var(--spacing-xs);
    width: 100%;
    padding: var(--spacing-sm) var(--spacing-sm);
    border-radius: var(--radius-md);
    text-align: left;
    color: var(--text-primary);
    transition: background var(--transition-fast);
    margin-bottom: 2px;
  }

  .conv-item:hover:not(:disabled) {
    background: var(--color-surface-container-high);
  }

  .conv-item.active {
    background: var(--accent);
    color: white;
  }

  .conv-item.conv-disabled {
    opacity: 0.5;
    cursor: not-allowed;
    pointer-events: none;
  }

  .conv-item-content {
    flex: 1;
    min-width: 0;
    display: flex;
    flex-direction: column;
    gap: 2px;
  }

  .conv-title {
    font-weight: 600;
    font-size: var(--font-size-md);
    line-height: 1.3;
  }

  .conv-preview {
    font-size: var(--font-size-sm);
    color: var(--text-secondary);
    line-height: 1.3;
  }

  .conv-item.active .conv-preview {
    color: rgba(255, 255, 255, 0.8);
  }

  .conv-item.active .conv-meta {
    color: rgba(255, 255, 255, 0.7);
  }

  .conv-meta {
    font-size: 11px;
    color: var(--text-secondary);
    margin-top: 2px;
  }

  .conv-menu-btn {
    display: flex;
    align-items: center;
    justify-content: center;
    padding: 4px;
    border-radius: var(--radius-sm);
    opacity: 0;
    transition: opacity var(--transition-fast);
    flex-shrink: 0;
    color: inherit;
  }

  .conv-item:hover .conv-menu-btn {
    opacity: 1;
  }

  .conv-item.active .conv-menu-btn {
    opacity: 1;
  }

  .conv-menu-btn:hover {
    background: rgba(0, 0, 0, 0.1);
  }

  .rename-input {
    width: 100%;
    padding: 2px 4px;
    border: 1px solid var(--accent);
    border-radius: var(--radius-sm);
    background: var(--bg-card);
    color: var(--text-primary);
    font-size: var(--font-size-md);
    outline: none;
  }

  /* ===== Context menu ===== */
  .context-menu-backdrop {
    position: fixed;
    inset: 0;
    z-index: 99;
  }

  .context-menu {
    position: fixed;
    z-index: 100;
    background: var(--color-surface-container-high);
    border: 1px solid var(--border);
    border-radius: var(--radius-md);
    box-shadow: var(--shadow-lg);
    padding: var(--spacing-xs);
    min-width: 160px;
  }

  .context-menu-item {
    display: flex;
    align-items: center;
    gap: var(--spacing-sm);
    width: 100%;
    padding: var(--spacing-sm) var(--spacing-md);
    border-radius: var(--radius-sm);
    color: var(--text-primary);
    font-size: var(--font-size-md);
    transition: background var(--transition-fast);
  }

  .context-menu-item:hover {
    background: var(--color-surface-container-highest);
  }

  .context-menu-item.danger {
    color: var(--danger);
  }

  /* ===== Modal ===== */
  .modal-backdrop {
    position: fixed;
    inset: 0;
    background: rgba(0, 0, 0, 0.4);
    z-index: 200;
  }

  .modal {
    position: fixed;
    top: 50%;
    left: 50%;
    transform: translate(-50%, -50%);
    background: var(--color-surface);
    border: 1px solid var(--border);
    border-radius: var(--radius-lg);
    padding: var(--spacing-lg);
    z-index: 201;
    min-width: 320px;
    box-shadow: var(--shadow-lg);
  }

  .modal h3 {
    margin-bottom: var(--spacing-sm);
  }

  .modal p {
    color: var(--text-secondary);
    margin-bottom: var(--spacing-lg);
    font-size: var(--font-size-sm);
  }

  .modal-actions {
    display: flex;
    gap: var(--spacing-sm);
    justify-content: flex-end;
  }

  /* ===== Chat main area ===== */
  .chat-main {
    flex: 1;
    display: flex;
    flex-direction: column;
    min-width: 0;
    overflow: hidden;
  }

  /* ===== Welcome screen ===== */
  .chat-welcome {
    flex: 1;
    display: flex;
    flex-direction: column;
    align-items: center;
    justify-content: center;
    padding: var(--spacing-xl);
    text-align: center;
    gap: var(--spacing-md);
  }

  .welcome-icon {
    color: var(--accent);
    margin-bottom: var(--spacing-sm);
  }

  .chat-welcome h2 {
    font-size: var(--font-size-2xl);
  }

  .chat-welcome > p {
    color: var(--text-secondary);
    max-width: 480px;
    line-height: 1.6;
  }

  .welcome-tips {
    display: flex;
    flex-direction: column;
    gap: var(--spacing-md);
    margin-top: var(--spacing-lg);
  }

  .tip {
    display: flex;
    align-items: center;
    gap: var(--spacing-md);
    color: var(--text-secondary);
    font-size: var(--font-size-md);
  }

  /* ===== Messages container ===== */
  .messages-container {
    flex: 1;
    overflow-y: auto;
    padding: var(--spacing-md);
    display: flex;
    flex-direction: column;
    gap: var(--spacing-md);
  }

  /* ===== Message bubbles ===== */
  .message {
    display: flex;
    max-width: 70%;
  }

  .message-user {
    align-self: flex-end;
  }

  .message-assistant {
    align-self: flex-start;
  }

  .message-bubble {
    padding: var(--spacing-md);
    border-radius: var(--radius-lg);
    background: var(--bg-card);
    border: 1px solid var(--border);
    font-size: var(--font-size-md);
    line-height: 1.6;
  }

  .message-user .message-bubble {
    background: var(--accent);
    color: white;
    border-color: var(--accent);
    border-bottom-right-radius: var(--radius-sm);
  }

  .message-assistant .message-bubble {
    border-bottom-left-radius: var(--radius-sm);
  }

  .message-content {
    white-space: pre-wrap;
    word-break: break-word;
  }

  .message-timestamp {
    font-size: 11px;
    color: var(--text-secondary);
    margin-top: var(--spacing-xs);
    opacity: 0.7;
  }

  .message-user .message-timestamp {
    color: rgba(255, 255, 255, 0.7);
  }

  /* ===== Markdown ===== */
  .chat-markdown :global(p) {
    margin-bottom: var(--spacing-sm);
  }

  .chat-markdown :global(p:last-child) {
    margin-bottom: 0;
  }

  .chat-markdown :global(strong) {
    font-weight: 700;
  }

  .chat-markdown :global(em) {
    font-style: italic;
  }

  .chat-markdown :global(code.chat-inline-code) {
    font-family: var(--font-mono);
    font-size: var(--font-size-code-md);
    background: var(--color-surface-container-high);
    padding: 1px 5px;
    border-radius: var(--radius-sm);
  }

  .message-user .chat-markdown :global(code.chat-inline-code) {
    background: rgba(255, 255, 255, 0.15);
  }

  .chat-markdown :global(pre.chat-code-block) {
    background: var(--color-surface-container-high);
    border-radius: var(--radius-md);
    padding: var(--spacing-md);
    overflow-x: auto;
    margin: var(--spacing-sm) 0;
  }

  .chat-markdown :global(pre.chat-code-block code) {
    font-family: var(--font-mono);
    font-size: var(--font-size-code-md);
    line-height: 1.5;
  }

  .chat-markdown :global(ul),
  .chat-markdown :global(ol) {
    padding-left: var(--spacing-lg);
    margin: var(--spacing-sm) 0;
  }

  .chat-markdown :global(li) {
    margin-bottom: 4px;
  }

  .chat-markdown :global(a.chat-link) {
    color: var(--accent);
    text-decoration: underline;
  }

  .message-user .chat-markdown :global(a.chat-link) {
    color: rgba(255, 255, 255, 0.9);
    text-decoration: underline;
  }

  /* ===== Citations ===== */
  .chat-markdown :global(sup.chat-citation) {
    color: var(--accent);
    font-weight: 700;
    cursor: pointer;
    font-size: 12px;
    padding: 0 2px;
    user-select: none;
  }

  .chat-markdown :global(sup.chat-citation:hover) {
    text-decoration: underline;
  }

  .message-citations {
    margin-top: var(--spacing-sm);
    padding-top: var(--spacing-sm);
    border-top: 1px solid var(--border);
  }

  .message-user .message-citations {
    border-top-color: rgba(255, 255, 255, 0.2);
  }

  .citations-title {
    font-size: var(--font-size-sm);
    font-weight: 600;
    color: var(--text-secondary);
    margin-bottom: var(--spacing-xs);
  }

  .message-user .citations-title {
    color: rgba(255, 255, 255, 0.7);
  }

  .citation-item {
    display: flex;
    align-items: center;
    gap: var(--spacing-sm);
    width: 100%;
    padding: var(--spacing-xs) var(--spacing-sm);
    border-radius: var(--radius-sm);
    font-size: var(--font-size-sm);
    color: var(--text-primary);
    transition: background var(--transition-fast);
  }

  .citation-item:hover {
    background: var(--color-surface-container-high);
  }

  .message-user .citation-item {
    color: rgba(255, 255, 255, 0.9);
  }

  .message-user .citation-item:hover {
    background: rgba(255, 255, 255, 0.1);
  }

  .citation-number {
    font-weight: 700;
    color: var(--accent);
    min-width: 24px;
  }

  .message-user .citation-number {
    color: rgba(255, 255, 255, 0.9);
  }

  .citation-type {
    font-weight: 500;
    padding: 1px 6px;
    background: var(--color-surface-container-high);
    border-radius: var(--radius-sm);
    font-size: 11px;
  }

  .message-user .citation-type {
    background: rgba(255, 255, 255, 0.15);
  }

  .citation-title {
    flex: 1;
    min-width: 0;
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }

  /* ===== Feedback ===== */
  .message-feedback {
    display: flex;
    gap: var(--spacing-xs);
    margin-top: var(--spacing-sm);
    opacity: 0;
    transition: opacity var(--transition-fast);
  }

  .message-bubble:hover .message-feedback {
    opacity: 1;
  }

  .feedback-btn {
    display: flex;
    align-items: center;
    justify-content: center;
    width: 28px;
    height: 28px;
    border-radius: var(--radius-sm);
    color: var(--text-secondary);
    transition: background var(--transition-fast), color var(--transition-fast);
  }

  .feedback-btn:hover {
    background: var(--color-surface-container-high);
    color: var(--text-primary);
  }

  .feedback-btn.feedback-active {
    color: var(--accent);
    opacity: 1;
  }

  .message-user .feedback-btn {
    color: rgba(255, 255, 255, 0.6);
  }

  .message-user .feedback-btn:hover {
    background: rgba(255, 255, 255, 0.1);
    color: white;
  }

  .message-user .feedback-btn.feedback-active {
    color: white;
  }

  .feedback-form {
    margin-top: var(--spacing-sm);
    padding-top: var(--spacing-sm);
    border-top: 1px solid var(--border);
  }

  .feedback-form-title {
    font-size: var(--font-size-sm);
    font-weight: 600;
    margin-bottom: var(--spacing-xs);
  }

  .feedback-reasons {
    display: flex;
    flex-wrap: wrap;
    gap: var(--spacing-xs);
    margin-top: var(--spacing-xs);
  }

  .feedback-reason-btn {
    padding: 4px 10px;
    border-radius: var(--radius-sm);
    font-size: var(--font-size-sm);
    border: 1px solid var(--border);
    color: var(--text-primary);
    transition: background var(--transition-fast), border-color var(--transition-fast);
  }

  .feedback-reason-btn:hover {
    border-color: var(--accent);
  }

  .feedback-reason-btn.selected {
    background: var(--accent);
    color: white;
    border-color: var(--accent);
  }

  .feedback-comment {
    width: 100%;
    margin-top: var(--spacing-sm);
    padding: var(--spacing-sm);
    border: 1px solid var(--border);
    border-radius: var(--radius-sm);
    background: var(--bg-card);
    color: var(--text-primary);
    font-size: var(--font-size-sm);
    resize: vertical;
    font-family: inherit;
  }

  .feedback-actions {
    display: flex;
    gap: var(--spacing-sm);
    justify-content: flex-end;
    margin-top: var(--spacing-sm);
  }

  /* ===== Typing indicator ===== */
  .typing-indicator {
    display: flex;
    align-items: center;
    gap: var(--spacing-sm);
    padding: var(--spacing-xs) 0;
  }

  .status-text {
    font-size: var(--font-size-sm);
    color: var(--text-secondary);
    font-style: italic;
  }

  .typing-dots {
    display: flex;
    gap: 2px;
  }

  .dot {
    animation: typing-bounce 1.4s infinite;
    font-size: 24px;
    line-height: 1;
    color: var(--text-secondary);
  }

  .dot:nth-child(2) {
    animation-delay: 0.2s;
  }

  .dot:nth-child(3) {
    animation-delay: 0.4s;
  }

  @keyframes typing-bounce {
    0%, 60%, 100% { opacity: 0.3; }
    30% { opacity: 1; }
  }

  /* ===== Error banner ===== */
  .error-banner {
    display: flex;
    align-items: center;
    gap: var(--spacing-sm);
    padding: var(--spacing-sm) var(--spacing-md);
    background: rgba(239, 68, 68, 0.1);
    border-top: 1px solid rgba(239, 68, 68, 0.3);
    color: var(--danger);
    font-size: var(--font-size-sm);
  }

  .error-banner span:nth-child(2) {
    flex: 1;
  }

  /* ===== Input area ===== */
  .input-area {
    border-top: 1px solid var(--border);
    padding: var(--spacing-md);
    background: var(--color-surface-container-low);
    display: flex;
    flex-direction: column;
    gap: var(--spacing-sm);
  }

  .input-area-streaming {
    opacity: 0.7;
  }

  /* ===== Entity chips ===== */
  .entity-chips {
    display: flex;
    flex-wrap: wrap;
    gap: var(--spacing-xs);
  }

  .entity-chip {
    display: inline-flex;
    align-items: center;
    gap: 4px;
    padding: 2px 8px;
    background: var(--accent);
    color: white;
    border-radius: var(--radius-sm);
    font-size: var(--font-size-sm);
    line-height: 1.4;
  }

  .chip-type {
    font-weight: 600;
    font-size: 11px;
    opacity: 0.9;
  }

  .chip-title {
    font-weight: 500;
  }

  .chip-remove {
    display: flex;
    align-items: center;
    justify-content: center;
    padding: 0;
    color: white;
    opacity: 0.8;
    transition: opacity var(--transition-fast);
  }

  .chip-remove:hover {
    opacity: 1;
  }

  /* ===== Entity dropdown ===== */
  .entity-dropdown-container {
    position: relative;
  }

  .entity-dropdown {
    position: absolute;
    bottom: 100%;
    left: 0;
    right: 0;
    max-height: 200px;
    overflow-y: auto;
    background: var(--color-surface);
    border: 1px solid var(--border);
    border-radius: var(--radius-md);
    box-shadow: var(--shadow-lg);
    z-index: 50;
    margin-bottom: 4px;
  }

  .entity-dropdown-item {
    display: flex;
    align-items: center;
    gap: var(--spacing-sm);
    width: 100%;
    padding: var(--spacing-sm) var(--spacing-md);
    text-align: left;
    color: var(--text-primary);
    font-size: var(--font-size-sm);
    transition: background var(--transition-fast);
  }

  .entity-dropdown-item:hover {
    background: var(--color-surface-container-high);
  }

  .dropdown-item-type {
    font-weight: 600;
    padding: 1px 6px;
    background: var(--color-surface-container-high);
    border-radius: var(--radius-sm);
    font-size: 11px;
    flex-shrink: 0;
  }

  .dropdown-item-title {
    font-weight: 500;
    flex-shrink: 0;
  }

  .dropdown-item-preview {
    color: var(--text-secondary);
    flex: 1;
    min-width: 0;
  }

  /* ===== Input toggles ===== */
  .input-toggles {
    display: flex;
    align-items: center;
    gap: var(--spacing-md);
    flex-wrap: wrap;
  }

  .mode-toggle {
    display: flex;
    gap: 0;
    border: 1px solid var(--border);
    border-radius: var(--radius-md);
    overflow: hidden;
  }

  .mode-btn {
    display: flex;
    align-items: center;
    gap: 4px;
    padding: 4px 12px;
    font-size: var(--font-size-sm);
    font-weight: 500;
    color: var(--text-secondary);
    transition: background var(--transition-fast), color var(--transition-fast);
  }

  .mode-btn:hover:not(:disabled) {
    background: var(--color-surface-container-high);
  }

  .mode-btn:disabled {
    opacity: 0.5;
    cursor: not-allowed;
  }

  .mode-btn.mode-active {
    background: var(--accent);
    color: white;
  }

  .mode-btn.mode-active:hover {
    background: var(--accent-hover);
  }

  .mode-btn .material-symbols-outlined {
    font-size: 16px;
  }

  .source-toggles {
    display: flex;
    gap: var(--spacing-md);
  }

  .toggle-label {
    display: flex;
    align-items: center;
    gap: 4px;
    font-size: var(--font-size-sm);
    color: var(--text-secondary);
    cursor: pointer;
    user-select: none;
  }

  .toggle-label.toggle-disabled {
    opacity: 0.5;
    cursor: not-allowed;
  }

  .toggle-label input[type="checkbox"] {
    accent-color: var(--accent);
  }

  /* ===== Input row ===== */
  .input-row {
    display: flex;
    gap: var(--spacing-sm);
    align-items: stretch;
  }

  .text-input-wrapper {
    flex: 1;
    display: flex;
    align-items: center;
    gap: var(--spacing-sm);
    padding: 0 var(--spacing-md);
    background: var(--bg-card);
    border: 1px solid var(--border);
    border-radius: var(--radius-md);
    transition: border-color var(--transition-fast);
  }

  .text-input-wrapper:focus-within {
    border-color: var(--accent);
  }

  .input-entity-icon {
    color: var(--text-secondary);
    font-size: 20px;
    flex-shrink: 0;
  }

  .text-input {
    flex: 1;
    border: none;
    background: transparent;
    color: var(--text-primary);
    font-size: var(--font-size-md);
    outline: none;
    padding: 10px 0;
  }

  .text-input:disabled {
    opacity: 0.5;
  }

  .text-input::placeholder {
    color: var(--text-secondary);
    opacity: 0.6;
  }

  .send-btn,
  .stop-btn {
    display: flex;
    align-items: center;
    justify-content: center;
    width: 40px;
    height: 40px;
    border-radius: var(--radius-md);
    color: white;
    flex-shrink: 0;
    transition: background var(--transition-fast);
  }

  .send-btn {
    background: var(--accent);
  }

  .send-btn:hover:not(:disabled) {
    background: var(--accent-hover);
  }

  .send-btn:disabled {
    opacity: 0.5;
    cursor: not-allowed;
  }

  .stop-btn {
    background: var(--danger);
  }

  .stop-btn:hover {
    background: #dc2626;
  }

  /* ===== Shared buttons ===== */
  :global(.btn) {
    display: inline-flex;
    align-items: center;
    justify-content: center;
    padding: var(--spacing-sm) var(--spacing-md);
    border-radius: var(--radius-md);
    font-size: var(--font-size-sm);
    font-weight: 500;
    transition: background var(--transition-fast), color var(--transition-fast);
  }

  :global(.btn-sm) {
    padding: 4px 10px;
    font-size: var(--font-size-sm);
  }

  :global(.btn-primary) {
    background: var(--accent);
    color: white;
  }

  :global(.btn-primary:hover) {
    background: var(--accent-hover);
  }

  :global(.btn-secondary) {
    background: var(--color-surface-container-high);
    color: var(--text-primary);
    border: 1px solid var(--border);
  }

  :global(.btn-secondary:hover) {
    background: var(--color-surface-container-highest);
  }

  :global(.btn-danger) {
    background: var(--danger);
    color: white;
  }

  :global(.btn-danger:hover) {
    background: #dc2626;
  }

  /* ===== Utilities ===== */
  .truncate {
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }

  .loading-spinner {
    animation: spin 1s linear infinite;
  }

  @keyframes spin {
    from { transform: rotate(0deg); }
    to { transform: rotate(360deg); }
  }
</style>
