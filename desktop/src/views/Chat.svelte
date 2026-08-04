<script lang="ts">
  import { onMount, onDestroy, tick } from "svelte";
  import { getState } from "../lib/state.svelte.js";
  import { navigateTo } from "../lib/router.svelte.js";
  import {
    chatSend,
    chatSearchEntities,
    chatListConversations,
    chatGetConversation,
    chatDeleteConversation,
    chatRenameConversation,
    chatStopStream,
    chatSendFeedback,
    getEntitySources,
    resolveEntityMention,
  } from "../lib/api.js";
  import type {
    EntitySourceEntry,
    MentionResolution,
  } from "../lib/api.js";
  import { ChatStreamSession } from "../lib/chat-stream.js";
  import { getEntityTypeColor } from "../lib/theme.svelte.js";
  import { builtinCommands, matchCommands } from "../lib/command-palette.js";
  import type {
    CommandDef,
  } from "../lib/command-palette.js";
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
  let selectedEntityRefs = $state<Map<string, EntitySearchResult>>(new Map());
  let showEntityDropdown = $state(false);
  let entitySearchResults = $state<EntitySearchResult[]>([]);
  let entitySearchPrefix = $state("");
  let entityDropdownIndex = $state(0);

  // --- Command palette ---
  let showCommandPalette = $state(false);
  let commandMatches = $state<CommandDef[]>([]);
  let commandPaletteIndex = $state(0);

  // --- Citation state ---
  let citationTooltip = $state<{
    x: number;
    y: number;
    citation: Citation;
  } | null>(null);

  // --- Collapsible sources ---
  let expandedSources = $state<Set<string>>(new Set());

  // --- Source enrichment cache ---
  let sourceCache = $state<Map<string, string | null>>(new Map());
  let sourceLoading = $state<Set<string>>(new Set());
  let sourceError = $state<Map<string, string>>(new Map());

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

  // --- Sidebar search ---
  let conversationSearch = $state("");

  // --- Input settings panel ---
  let showInputSettings = $state(false);

  // --- Shortcuts help ---
  let showShortcutsHelp = $state(false);

  // --- Clear confirmation ---
  let clearingConversation = $state(false);

  // --- In-conversation search ---
  let showFindBar = $state(false);
  let findQuery = $state("");
  let findActiveIndex = $state(0);

  // --- Copy feedback ---
  let copiedMessageId = $state<string | null>(null);

  onMount(() => {
    loadConversations();
    window.addEventListener("keydown", handleWindowKeyDown);
  });

  onDestroy(() => {
    streamSession?.stop();
    if (statusTimer) clearTimeout(statusTimer);
    window.removeEventListener("keydown", handleWindowKeyDown);
  });

  function handleWindowKeyDown(e: KeyboardEvent) {
    const target = e.target as HTMLElement;
    const isTyping =
      target.tagName === "INPUT" ||
      target.tagName === "TEXTAREA" ||
      target.isContentEditable;

    if ((e.ctrlKey || e.metaKey) && e.key.toLowerCase() === "f") {
      e.preventDefault();
      openFindBar();
      return;
    }

    if (e.key === "?" && !isTyping && !e.ctrlKey && !e.metaKey && !e.altKey) {
      e.preventDefault();
      showShortcutsHelp = true;
      return;
    }
  }

  let findState = $derived.by(() => {
    const counter = { n: 0 };
    const items = messages.map((m) => {
      const base =
        m.role === "user"
          ? renderEntityPills(m.content)
          : renderWithCitations(m.content, m.citations);
      return { ...m, html: highlightFind(base, findQuery, counter) };
    });
    return { items, count: counter.n };
  });

  let filteredConversations = $derived(
    conversations.filter((c) => {
      const q = conversationSearch.trim().toLowerCase();
      if (!q) return true;
      return (
        (c.title || "").toLowerCase().includes(q) ||
        (c.last_message_preview || "").toLowerCase().includes(q)
      );
    })
  );

  function openFindBar() {
    showFindBar = true;
    tick().then(() => {
      const input = document.querySelector<HTMLInputElement>(".find-input");
      input?.focus();
      input?.select();
    });
  }

  function closeFindBar() {
    showFindBar = false;
    findQuery = "";
    findActiveIndex = 0;
  }

  function updateFindQuery(value: string) {
    findQuery = value;
    findActiveIndex = 0;
    if (value.trim()) scrollToFind(0);
  }

  function findNext() {
    if (findState.count === 0) return;
    findActiveIndex = (findActiveIndex + 1) % findState.count;
    scrollToFind(findActiveIndex);
  }

  function findPrev() {
    if (findState.count === 0) return;
    findActiveIndex = (findActiveIndex - 1 + findState.count) % findState.count;
    scrollToFind(findActiveIndex);
  }

  function scrollToFind(idx: number) {
    tick().then(() => {
      const el = document.querySelector(`[data-find-idx="${idx}"]`);
      el?.scrollIntoView({ block: "center", behavior: "smooth" });
    });
  }

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
      const detail = await chatGetConversation(id);
        if (detail) {
          messages = detail.messages.map((m) => {
            const display: MessageDisplay = {
              id: m.id,
              role: m.role as "user" | "assistant" | "system",
              content: m.text,
              citations: m.citations,
              feedback: m.feedback?.rating,
              timestamp: m.created_at,
            };
            return display;
          });
          scrollToBottom();
          // Enrich sources for all assistant messages
          for (const m of messages) {
            if (m.role === "assistant" && m.citations && m.citations.length > 0) {
              enrichMessageSources(m.id, m.citations);
            }
          }
        }
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
    selectedEntityRefs = new Map();
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

  async function updateEntitySearch(value: string) {
    const cursorPos = getCursorPosition();
    const textBeforeCursor = value.slice(0, cursorPos);
    const atIndex = textBeforeCursor.lastIndexOf("@");

    if (atIndex >= 0 && (atIndex === 0 || textBeforeCursor[atIndex - 1] === " ")) {
      const prefix = textBeforeCursor.slice(atIndex + 1);
      entitySearchPrefix = prefix;
      if (prefix.length >= 1 && !prefix.includes(" ")) {
        try {
          entitySearchResults = await chatSearchEntities(prefix);
          showEntityDropdown = entitySearchResults.length > 0;
          entityDropdownIndex = 0;
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

  function getCursorPosition(): number {
    const textarea = document.querySelector(".text-input") as HTMLTextAreaElement;
    if (!textarea) return inputText.length;
    return textarea.selectionStart ?? inputText.length;
  }

  function selectEntityRef(entity: EntitySearchResult) {
    if (selectedEntityRefs.has(entity.id)) return;
    selectedEntityRefs = new Map(selectedEntityRefs).set(entity.id, entity);

    const cursorPos = getCursorPosition();
    const textBeforeCursor = inputText.slice(0, cursorPos);
    const atIndex = textBeforeCursor.lastIndexOf("@");
    if (atIndex >= 0) {
      const before = inputText.slice(0, atIndex);
      const after = inputText.slice(cursorPos);
      const pill = `@${entity.entity_type}:${entity.title} `;
      inputText = before + pill + after;
    }
    showEntityDropdown = false;
  }

  function getEntityRefIds(): string[] {
    return Array.from(selectedEntityRefs.keys());
  }

  function isCommandInput(text: string): boolean {
    return text.startsWith("/") && !text.startsWith("//");
  }

  // --- Send / Stream ---

  async function sendMessage() {
    const text = inputText.trim();
    if (!text) return;
    if (streaming) return;

    if (isCommandInput(text)) {
      executeCommandText(text);
      return;
    }

    errorMessage = null;
    processingStatus = null;

    const msgId = ++sendCounter;
    const entityIds = getEntityRefIds();
    const userMsg: MessageDisplay = {
      id: `user-${msgId}`,
      role: "user",
      content: text,
      timestamp: new Date().toISOString(),
    };
    messages = [...messages, userMsg];
    inputText = "";
    selectedEntityRefs = new Map();
    resetTextareaHeight();

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
        entityIds,
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
            selectedEntityRefs = new Map();
            currentConversationId = session.getConversationId();
            loadConversations();
            scrollToBottom();
            // Enrich sources for the completed message
            if (info.citations && info.citations.length > 0) {
              enrichMessageSources(info.assistantMessageId, info.citations);
            }
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
      await fallbackSend(text, entityIds);
    }
  }

  async function fallbackSend(text: string, entityIds: string[]) {
    if (messages.length > 0 && messages[messages.length - 1].role === "assistant" && !messages[messages.length - 1].content) {
      messages = messages.slice(0, -1);
    }
    try {
      const result = await chatSend(
        currentConversationId,
        text,
        entityIds,
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
      selectedEntityRefs = new Map();
      loadConversations();
      scrollToBottom();
      // Enrich sources for the completed message
      if (result.citations && result.citations.length > 0) {
        enrichMessageSources(result.message_id, result.citations);
      }
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

  // --- Command palette ---

  function runCommand(cmd: CommandDef, arg?: string) {
    switch (cmd.id) {
      case "help":
        showShortcutsHelp = true;
        break;
      case "clear":
        confirmClearConversation();
        break;
      case "export":
        exportConversation();
        break;
      default:
        cmd.action(arg);
    }
  }

  function executeCommandText(text: string) {
    const [cmd, ...args] = text.slice(1).split(/\s+/);
    const match = builtinCommands.find((c) => c.name === `/${cmd}`);
    if (match) {
      runCommand(match, args.join(" "));
    }
  }

  function executeCommand(cmd: CommandDef) {
    runCommand(cmd);
    showCommandPalette = false;
    inputText = "";
    resetTextareaHeight();
  }

  function updateCommandPalette(value: string) {
    if (value.startsWith("/") && !value.startsWith("//")) {
      const afterSlash = value.slice(1);
      if (afterSlash.length === 0 || !afterSlash.includes(" ")) {
        commandMatches = matchCommands(`/${afterSlash}`);
        showCommandPalette = commandMatches.length > 0;
        commandPaletteIndex = 0;
        return;
      }
    }
    showCommandPalette = false;
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

  function renderEntityPills(content: string): string {
    let html = content;
    html = html.replace(/&/g, "&amp;").replace(/</g, "&lt;").replace(/>/g, "&gt;");

    // P1: Extract code blocks FIRST to protect their content from mention regex
    const codeBlocks: string[] = [];
    html = html.replace(/```([\s\S]*?)```/g, (_, code) => {
      const escaped = code
        .replace(/&/g, "&amp;")
        .replace(/</g, "&lt;")
        .replace(/>/g, "&gt;");
      const placeholder = `\x00CODEBLOCK_${codeBlocks.length}\x00`;
      codeBlocks.push(`<pre class="chat-code-block"><code>${escaped}</code></pre>`);
      return placeholder;
    });

    html = html.replace(/`([^`]+)`/g, '<code class="chat-inline-code">$1</code>');

    // Convert @Type:Title to clickable pill buttons
    html = html.replace(/@(\w+):([^\s]+)/g, (_, type, title) => {
      const color = getEntityTypeColor(type);
      return `<button class="entity-pill entity-pill-clickable" style="background: ${color}" data-entity-type="${type}" data-entity-title="${title}" type="button"><span class="entity-pill-type">${type}</span><span class="entity-pill-title">${title}</span></button>`;
    });

    // Restore code blocks
    html = html.replace(/\x00CODEBLOCK_(\d+)\x00/g, (_, i) => codeBlocks[parseInt(i)]);

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

    // P1: Extract code blocks FIRST to protect citations inside code
    const codeBlocks: string[] = [];
    html = html.replace(/```([\s\S]*?)```/g, (_, code) => {
      const escaped = code
        .replace(/&/g, "&amp;")
        .replace(/</g, "&lt;")
        .replace(/>/g, "&gt;");
      const placeholder = `\x00CODEBLOCK_${codeBlocks.length}\x00`;
      codeBlocks.push(`<pre class="chat-code-block"><code>${escaped}</code></pre>`);
      return placeholder;
    });

    // Convert [N] citations to real <a> anchors with href routing
    html = html.replace(/\[(\d+)\]/g, (_, num) => {
      const citation = citations.find((c) => c.number === parseInt(num));
      if (citation) {
        const sourceInfo = citation.source
          ? ` data-entity-source="${citation.source}"`
          : "";
        return `<a class="chat-citation" href="#/entity/${citation.entity_id}" data-entity-id="${citation.entity_id}" data-citation-number="${citation.number}"${sourceInfo}>[${num}]</a>`;
      }
      return `<span class="chat-citation">[${num}]</span>`;
    });

    html = html.replace(/`([^`]+)`/g, '<code class="chat-inline-code">$1</code>');

    // Restore code blocks
    html = html.replace(/\x00CODEBLOCK_(\d+)\x00/g, (_, i) => codeBlocks[parseInt(i)]);

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

  async function onInput(e: Event) {
    const value = (e.target as HTMLTextAreaElement).value;
    inputText = value;
    autoResizeTextarea(e.target as HTMLTextAreaElement);

    showCommandPalette = false;
    showEntityDropdown = false;

    updateCommandPalette(value);
    if (!showCommandPalette) {
      await updateEntitySearch(value);
    }
  }

  function handleKeyDown(e: KeyboardEvent) {
    if (showCommandPalette && (e.key === "ArrowDown" || e.key === "ArrowUp")) {
      e.preventDefault();
      const max = commandMatches.length;
      if (e.key === "ArrowDown") {
        commandPaletteIndex = (commandPaletteIndex + 1) % max;
      } else {
        commandPaletteIndex = (commandPaletteIndex - 1 + max) % max;
      }
      return;
    }

    if (showEntityDropdown && (e.key === "ArrowDown" || e.key === "ArrowUp")) {
      e.preventDefault();
      const max = entitySearchResults.length;
      if (e.key === "ArrowDown") {
        entityDropdownIndex = (entityDropdownIndex + 1) % max;
      } else {
        entityDropdownIndex = (entityDropdownIndex - 1 + max) % max;
      }
      return;
    }

    if (e.key === "Escape") {
      if (showCommandPalette) {
        showCommandPalette = false;
        return;
      }
      if (showEntityDropdown) {
        showEntityDropdown = false;
        return;
      }
      return;
    }

    if (e.key === "Enter") {
      if (showCommandPalette) {
        e.preventDefault();
        const cmd = commandMatches[commandPaletteIndex];
        if (cmd) executeCommand(cmd);
        return;
      }
      if (showEntityDropdown) {
        e.preventDefault();
        const entity = entitySearchResults[entityDropdownIndex];
        if (entity) selectEntityRef(entity);
        return;
      }
      if (!e.shiftKey) {
        e.preventDefault();
        sendMessage();
        return;
      }
      return;
    }

    if (e.key === "Tab") {
      if (showCommandPalette) {
        e.preventDefault();
        showCommandPalette = false;
        return;
      }
      if (showEntityDropdown) {
        e.preventDefault();
        showEntityDropdown = false;
        return;
      }
    }
  }

  function autoResizeTextarea(textarea: HTMLTextAreaElement) {
    textarea.style.height = "auto";
    textarea.style.height = `${Math.min(textarea.scrollHeight, 200)}px`;
  }

  function resetTextareaHeight() {
    const textarea = document.querySelector(".text-input") as HTMLTextAreaElement | null;
    if (textarea) {
      textarea.style.height = "auto";
    }
  }

  // --- Citation click / hover ---

  function handleMessageContentClick(e: MouseEvent) {
    const target = e.target as HTMLElement;

    // Citation [N] click
    const citation = target.closest(".chat-citation") as HTMLElement | null;
    if (citation) {
      const entityId = citation.dataset.entityId;
      if (entityId) {
        navigateTo("detail", entityId);
      }
      return;
    }

    // Entity pill @Type:Title click
    const pill = target.closest(".entity-pill-clickable") as HTMLElement | null;
    if (pill) {
      const entityType = pill.dataset.entityType;
      const title = pill.dataset.entityTitle;
      if (entityType && title) {
        handlePillClick(entityType, title);
      }
      return;
    }
  }

  async function handlePillClick(entityType: string, title: string) {
    try {
      const resolution = await resolveEntityMention(entityType, title);
      if (resolution) {
        navigateTo("detail", resolution.entity_id);
      } else {
        app.statusMessage = `No entity found for @${entityType}:${title}`;
      }
    } catch (err) {
      app.statusMessage = `Failed to resolve @${entityType}:${title}: ${err}`;
    }
  }

  function handleMessageContentMouseEnter(e: MouseEvent) {
    const target = e.target as HTMLElement;
    const el = target.closest(".chat-citation") as HTMLElement | null;
    if (el) {
      const entityId = el.dataset.entityId;
      const num = el.dataset.citationNumber;
      if (entityId && num) {
        const msg = messages.find((m) =>
          m.citations?.some((c) => c.entity_id === entityId && c.number === parseInt(num))
        );
        if (msg) {
          const c = msg.citations!.find((c) => c.entity_id === entityId && c.number === parseInt(num));
          if (c) {
            citationTooltip = {
              x: e.clientX,
              y: e.clientY,
              citation: c,
            };
          }
        }
      }
    }
  }

  function handleMessageContentMouseLeave() {
    citationTooltip = null;
  }

  // --- Collapsible sources ---

  function toggleSources(id: string) {
    const next = new Set(expandedSources);
    if (next.has(id)) {
      next.delete(id);
    } else {
      next.add(id);
    }
    expandedSources = next;
  }

  function openEntityDetail(entityId: string) {
    navigateTo("detail", entityId);
  }

  // --- Source enrichment ---

  async function enrichMessageSources(msgId: string, citations: Citation[]) {
    const uncachedIds = citations
      .filter((c) => !sourceCache.has(c.entity_id) && !sourceLoading.has(c.entity_id))
      .map((c) => c.entity_id);

    if (uncachedIds.length === 0) return;

    const loadingSet = new Set(sourceLoading);
    uncachedIds.forEach((id) => loadingSet.add(id));
    sourceLoading = loadingSet;

    try {
      const entries = await getEntitySources(uncachedIds);
      const cache = new Map(sourceCache);
      for (const entry of entries) {
        cache.set(entry.entity_id, entry.source);
      }
      sourceCache = cache;
    } catch (err) {
      const errMap = new Map(sourceError);
      for (const id of uncachedIds) {
        errMap.set(id, `Failed to fetch source: ${err}`);
      }
      sourceError = errMap;
    } finally {
      const loadedSet = new Set(sourceLoading);
      uncachedIds.forEach((id) => loadedSet.delete(id));
      sourceLoading = loadedSet;
    }
  }

  async function openEntitySource(entityId: string) {
    const source = sourceCache.get(entityId);
    if (source === undefined) {
      app.statusMessage = "Source not available for this entity.";
      return;
    }
    if (source === null) {
      app.statusMessage = "This entity has no associated source file.";
      return;
    }
    try {
      const { openInDefaultApp } = await import("../lib/api.js");
      await openInDefaultApp(source);
    } catch (err) {
      const errMap = new Map(sourceError);
      errMap.set(entityId, `Failed to open: ${err}`);
      sourceError = errMap;
      app.statusMessage = `Failed to open source: ${err}`;
    }
  }

  // --- Actions ---
  function focusOnMount(node: HTMLInputElement) {
    node.focus();
    node.select();
  }

  // --- Clear conversation ---

  function confirmClearConversation() {
    if (messages.length === 0) {
      app.statusMessage = "Nothing to clear.";
      return;
    }
    clearingConversation = true;
  }

  function doClear() {
    currentConversationId = null;
    messages = [];
    inputText = "";
    selectedEntityRefs = new Map();
    errorMessage = null;
    processingStatus = null;
    showFindBar = false;
    findQuery = "";
    clearingConversation = false;
  }

  // --- Export conversation ---

  function exportConversation() {
    if (messages.length === 0) {
      app.statusMessage = "Nothing to export yet.";
      return;
    }
    const conv = conversations.find((c) => c.id === currentConversationId);
    const title = (conv?.title || "conversation")
      .replace(/[^\w\- ]+/g, "")
      .trim()
      .slice(0, 60) || "conversation";

    const md = messages
      .map((m) => {
        const who = m.role === "user" ? "You" : "Assistant";
        return `**${who}** (${formatDate(m.timestamp)}):\n\n${m.content}`;
      })
      .join("\n\n---\n\n");

    const blob = new Blob([md], { type: "text/markdown;charset=utf-8" });
    const url = URL.createObjectURL(blob);
    const a = document.createElement("a");
    a.href = url;
    a.download = `${title}.md`;
    document.body.appendChild(a);
    a.click();
    document.body.removeChild(a);
    URL.revokeObjectURL(url);
    app.statusMessage = `Conversation exported as ${title}.md`;
  }

  // --- Copy message ---

  async function copyMessage(msg: MessageDisplay) {
    try {
      await navigator.clipboard.writeText(msg.content);
      copiedMessageId = msg.id;
      setTimeout(() => {
        if (copiedMessageId === msg.id) copiedMessageId = null;
      }, 1500);
    } catch {
      app.statusMessage = "Failed to copy message.";
    }
  }

  // --- Find highlighting ---

  function escapeRegex(s: string): string {
    return s.replace(/[.*+?^${}()|[\]\\]/g, "\\$&");
  }

  function highlightFind(html: string, term: string, counter: { n: number }): string {
    if (!term.trim()) return html;
    const escapedTerm = term.replace(/&/g, "&amp;").replace(/</g, "&lt;").replace(/>/g, "&gt;");
    const re = new RegExp(escapeRegex(escapedTerm), "gi");
    return html.replace(/(<[^>]+>)|([^<]+)/g, (match, tag, text) => {
      if (tag) return tag;
      return text.replace(re, (m: string) => `<mark class="find-match" data-find-idx="${counter.n++}">${m}</mark>`);
    });
  }
</script>

<div class="chat-view">
  <!-- Left pane: conversation list -->
  <aside class="conv-sidebar">
    <button class="new-chat-btn" onclick={newConversation} disabled={streaming}>
      <span class="material-symbols-outlined">add</span>
      New Chat
    </button>

    <div class="conv-search">
      <span class="material-symbols-outlined conv-search-icon">search</span>
      <input
        class="conv-search-input"
        type="text"
        placeholder="Search conversations..."
        bind:value={conversationSearch}
        aria-label="Search conversations"
      />
    </div>

    <div class="conv-list">
      {#if loadingConversations}
        <div class="conv-list-loading">
          <span class="material-symbols-outlined loading-spinner">sync</span>
        </div>
      {:else if filteredConversations.length === 0}
        <p class="conv-list-empty">
          {#if conversations.length === 0}
            No conversations yet.<br />Start a new chat!
          {:else}
            No conversations match "{conversationSearch}".
          {/if}
        </p>
      {:else}
        {#each filteredConversations as conv}
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
      <div class="modal" role="dialog" aria-modal="true" aria-labelledby="delete-conv-title">
        <h3 id="delete-conv-title">Delete conversation?</h3>
        <p>This action cannot be undone.</p>
        <div class="modal-actions">
          <button class="btn btn-secondary" onclick={() => deletingConvId = null}>Cancel</button>
          <button class="btn btn-danger" onclick={doDelete}>Delete</button>
        </div>
      </div>
    {/if}

    <!-- Clear confirmation -->
    {#if clearingConversation}
      <!-- svelte-ignore a11y_click_events_have_key_events -->
      <div class="modal-backdrop" onclick={() => clearingConversation = false} onkeydown={(e) => { if (e.key === 'Escape') clearingConversation = false; }} role="presentation"></div>
      <div class="modal" role="dialog" aria-modal="true" aria-labelledby="clear-conv-title">
        <h3 id="clear-conv-title">Clear conversation?</h3>
        <p>This clears the current conversation and starts a new chat.</p>
        <div class="modal-actions">
          <button class="btn btn-secondary" onclick={() => clearingConversation = false}>Cancel</button>
          <button class="btn btn-danger" onclick={doClear}>Clear</button>
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
      {#if showFindBar}
        <div class="find-bar">
          <span class="material-symbols-outlined find-bar-icon">search</span>
          <input
            class="find-input"
            type="text"
            placeholder="Find in conversation..."
            bind:value={findQuery}
            oninput={(e) => updateFindQuery((e.target as HTMLInputElement).value)}
            onkeydown={(e) => {
              if (e.key === "Enter") { e.preventDefault(); findNext(); }
              if (e.key === "Escape") closeFindBar();
            }}
            aria-label="Find in conversation"
          />
          <span class="find-count">
            {#if findQuery.trim() && findState.count === 0}
              No matches
            {:else if findQuery.trim()}
              {findActiveIndex + 1}/{findState.count}
            {/if}
          </span>
          <button class="find-nav" onclick={findPrev} title="Previous match (Shift+Enter)">
            <span class="material-symbols-outlined">chevron_left</span>
          </button>
          <button class="find-nav" onclick={findNext} title="Next match (Enter)">
            <span class="material-symbols-outlined">chevron_right</span>
          </button>
          <button class="find-close" onclick={closeFindBar} title="Close search">
            <span class="material-symbols-outlined">close</span>
          </button>
        </div>
      {/if}
      <div class="messages-container" bind:this={messagesContainer} aria-live="polite">
        {#each findState.items as msg}
          <div class="message" class:message-user={msg.role === "user"} class:message-assistant={msg.role === "assistant" || msg.role === "system"}>
            <div class="message-bubble">
              {#if msg.role === "user"}
                <div class="message-content chat-markdown">
                  {@html msg.html}
                </div>
              {:else}
                <div
                  class="message-content chat-markdown"
                  onclick={handleMessageContentClick}
                  onmouseenter={handleMessageContentMouseEnter}
                  onmouseleave={handleMessageContentMouseLeave}
                  role="document"
                >
                  {@html msg.html}
                </div>
              {/if}

              <div class="message-timestamp">{formatTime(msg.timestamp)}</div>

              <!-- Collapsible sources footer -->
              {#if msg.citations && msg.citations.length > 0}
                <div class="sources-footer">
                  <button
                    class="sources-toggle"
                    onclick={() => toggleSources(msg.id)}
                    aria-expanded={expandedSources.has(msg.id)}
                  >
                    <span class="material-symbols-outlined sources-chevron" class:expanded={expandedSources.has(msg.id)}>chevron_right</span>
                    View sources ({msg.citations.length})
                  </button>
                  {#if expandedSources.has(msg.id)}
                    <div class="sources-list">
                      {#each msg.citations as citation}
                        <!-- P1 fix: single interactive element per source row -->
                        <div
                          class="source-item"
                          role="button"
                          tabindex="0"
                          onclick={() => openEntityDetail(citation.entity_id)}
                          onkeydown={(e) => { if (e.key === 'Enter') openEntityDetail(citation.entity_id); }}
                        >
                          <span class="source-number">[{citation.number}]</span>
                          <span class="source-type" style="background: {getEntityTypeColor(citation.entity_type)}; color: white">{citation.entity_type}</span>
                          <span class="source-title">{citation.title}</span>
                          {#if sourceLoading.has(citation.entity_id)}
                            <span class="source-status">
                              <span class="material-symbols-outlined loading-spinner" style="font-size: 14px;">sync</span>
                            </span>
                          {:else if sourceError.has(citation.entity_id)}
                            <button
                              class="source-open-btn source-error"
                              onclick={(e) => { e.stopPropagation(); app.statusMessage = sourceError.get(citation.entity_id) ?? "Unknown error"; }}
                              title="Source error"
                              type="button"
                            >
                              <span class="material-symbols-outlined" style="font-size: 14px;">error</span>
                            </button>
                          {:else if sourceCache.get(citation.entity_id) !== undefined}
                            <button
                              class="source-open-btn"
                              onclick={(e) => { e.stopPropagation(); openEntitySource(citation.entity_id); }}
                              title={sourceCache.get(citation.entity_id) || "No source file"}
                              type="button"
                            >
                              <span class="material-symbols-outlined" style="font-size: 14px;">open_in_new</span>
                            </button>
                          {/if}
                        </div>
                      {/each}
                    </div>
                  {/if}
                </div>
              {/if}

              <!-- Feedback buttons -->
              {#if (msg.role === "assistant" || msg.role === "user") && msg.content}
                <div class="message-feedback">
                  <button
                    class="feedback-btn"
                    class:feedback-active={copiedMessageId === msg.id}
                    onclick={() => copyMessage(msg)}
                    title="Copy message"
                  >
                    <span class="material-symbols-outlined">
                      {copiedMessageId === msg.id ? "check" : "content_copy"}
                    </span>
                  </button>
                  {#if msg.role === "assistant"}
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
                  {/if}
                </div>
              {/if}

              <!-- Feedback form (thumbs down) -->
              {#if feedbackForm && feedbackForm.messageId === msg.id}
                <div class="feedback-form">
                  <p class="feedback-form-title">What went wrong?</p>
                  <div class="feedback-reasons">
                    {#each [
                      { value: "wrong_entity", label: "Wrong Entity" },
                      { value: "missing_info", label: "Missing Info" },
                      { value: "wrong_citation", label: "Wrong Citation" },
                      { value: "other", label: "Other" },
                    ] as reasonOption}
                      <button
                        class="feedback-reason-btn"
                        class:selected={feedbackForm.reason === reasonOption.value}
                        onclick={() => { if (feedbackForm) feedbackForm = { ...feedbackForm, reason: reasonOption.value }; }}
                      >
                        {reasonOption.label}
                      </button>
                    {/each}
                  </div>
                  <textarea
                    class="feedback-comment"
                    placeholder="Additional comment (optional)..."
                    bind:value={feedbackForm.comment}
                    rows={2}
                    aria-label="Feedback comment"
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

    <!-- Citation tooltip -->
    {#if citationTooltip}
      <div
        class="citation-tooltip"
        style="left: {citationTooltip.x + 12}px; top: {citationTooltip.y - 10}px;"
      >
        <span class="tooltip-type" style="color: {getEntityTypeColor(citationTooltip.citation.entity_type)}">{citationTooltip.citation.entity_type}</span>
        <span class="tooltip-title">{citationTooltip.citation.title}</span>
        <span class="tooltip-snippet">{citationTooltip.citation.snippet}</span>
        {#if citationTooltip.citation.source}
          <span class="tooltip-source">
            <span class="material-symbols-outlined" style="font-size: 12px;">link</span>
            {citationTooltip.citation.source}
          </span>
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
      <!-- Command palette dropdown -->
      {#if showCommandPalette}
        <div class="dropdown command-dropdown">
          {#each commandMatches as cmd, i}
            <button
              class="dropdown-item"
              class:dropdown-item-selected={i === commandPaletteIndex}
              onclick={() => executeCommand(cmd)}
            >
              <span class="dropdown-item-name">{cmd.name}</span>
              <span class="dropdown-item-args">{cmd.args}</span>
              <span class="dropdown-item-desc">{cmd.description}</span>
            </button>
          {/each}
        </div>
      {:else if showEntityDropdown}
        <div class="dropdown entity-dropdown">
          {#each entitySearchResults as entity, i}
            <button
              class="dropdown-item"
              class:dropdown-item-selected={i === entityDropdownIndex}
              onclick={() => selectEntityRef(entity)}
            >
              <span class="dropdown-item-type" style="background: {getEntityTypeColor(entity.entity_type)}; color: white">{entity.entity_type}</span>
              <span class="dropdown-item-title">{entity.title}</span>
              <span class="dropdown-item-preview truncate">{entity.preview}</span>
            </button>
          {/each}
        </div>
      {/if}

      <!-- Input settings panel (collapsible) -->
      {#if showInputSettings}
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
      {/if}

      <!-- Input row: textarea + button -->
      <div class="input-row">
        <div class="text-input-wrapper">
          <span class="material-symbols-outlined input-entity-icon">alternate_email</span>
          <textarea
            class="text-input"
            placeholder="Ask a question... (use @ to reference entities, / for commands)"
            bind:value={inputText}
            oninput={onInput}
            onkeydown={handleKeyDown}
            disabled={streaming}
            rows={1}
            aria-label="Message"
          ></textarea>
        </div>
        <div class="input-controls">
          <button
            class="icon-btn"
            class:icon-btn-active={showInputSettings}
            onclick={() => showInputSettings = !showInputSettings}
            disabled={streaming}
            title="Input settings"
            aria-expanded={showInputSettings}
            aria-label="Toggle input settings"
          >
            <span class="material-symbols-outlined">tune</span>
          </button>
          <button
            class="icon-btn"
            onclick={() => showShortcutsHelp = true}
            title="Keyboard shortcuts (?)"
            aria-label="Show keyboard shortcuts"
          >
            <span class="material-symbols-outlined">help</span>
          </button>
        </div>
        {#if streaming}
          <button class="stop-btn" onclick={stopStreaming} title="Stop generating">
            <span class="material-symbols-outlined">stop</span>
          </button>
        {:else}
          <button
            class="send-btn"
            onclick={sendMessage}
            disabled={streaming || !inputText.trim()}
            title="Send message"
          >
            <span class="material-symbols-outlined">send</span>
          </button>
        {/if}
      </div>
    </div>

    <!-- Shortcuts help modal -->
    {#if showShortcutsHelp}
      <!-- svelte-ignore a11y_click_events_have_key_events -->
      <div class="modal-backdrop" onclick={() => showShortcutsHelp = false} onkeydown={(e) => { if (e.key === 'Escape') showShortcutsHelp = false; }} role="presentation"></div>
      <div class="modal" role="dialog" aria-modal="true" aria-labelledby="shortcuts-title">
        <h3 id="shortcuts-title">Keyboard Shortcuts</h3>
        <div class="shortcuts-grid">
          <div class="shortcut-row"><span class="shortcut-keys">Enter</span><span>Send message</span></div>
          <div class="shortcut-row"><span class="shortcut-keys">Shift + Enter</span><span>New line</span></div>
          <div class="shortcut-row"><span class="shortcut-keys">@</span><span>Reference an entity</span></div>
          <div class="shortcut-row"><span class="shortcut-keys">/</span><span>Commands (/help, /clear, /export)</span></div>
          <div class="shortcut-row"><span class="shortcut-keys">?</span><span>Show this help</span></div>
          <div class="shortcut-row"><span class="shortcut-keys">Ctrl + F</span><span>Find in conversation</span></div>
          <div class="shortcut-row"><span class="shortcut-keys">Ctrl + N</span><span>New import</span></div>
          <div class="shortcut-row"><span class="shortcut-keys">Ctrl + 1&ndash;8</span><span>Switch views</span></div>
          <div class="shortcut-row"><span class="shortcut-keys">Esc</span><span>Close popups / menus</span></div>
        </div>
        <div class="modal-actions">
          <button class="btn btn-secondary" onclick={() => showShortcutsHelp = false}>Close</button>
        </div>
      </div>
    {/if}
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

  .conv-search {
    display: flex;
    align-items: center;
    gap: var(--spacing-xs);
    margin: 0 var(--spacing-md) var(--spacing-sm);
    padding: 0 var(--spacing-sm);
    background: var(--bg-card);
    border: 1px solid var(--border);
    border-radius: var(--radius-md);
    transition: border-color var(--transition-fast);
  }

  .conv-search:focus-within {
    border-color: var(--accent);
  }

  .conv-search-icon {
    font-size: 18px;
    color: var(--text-secondary);
    flex-shrink: 0;
  }

  .conv-search-input {
    flex: 1;
    min-width: 0;
    border: none;
    background: transparent;
    color: var(--text-primary);
    font-size: var(--font-size-sm);
    outline: none;
    padding: 8px 0;
  }

  .conv-search-input::placeholder {
    color: var(--text-secondary);
    opacity: 0.6;
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
    color: var(--text-on-accent-secondary);
  }

  .conv-item.active .conv-meta {
    color: var(--text-on-accent-muted);
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
    background: var(--overlay-hover);
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
    background: var(--overlay-scrim);
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

  /* ===== Shortcuts help ===== */
  .shortcuts-grid {
    display: flex;
    flex-direction: column;
    gap: var(--spacing-xs);
    margin: var(--spacing-md) 0 var(--spacing-lg);
  }

  .shortcut-row {
    display: flex;
    align-items: center;
    gap: var(--spacing-md);
    font-size: var(--font-size-sm);
  }

  .shortcut-keys {
    min-width: 120px;
    font-family: var(--font-mono);
    font-size: var(--font-size-code-md);
    background: var(--color-surface-container-high);
    border-radius: var(--radius-sm);
    padding: 2px 8px;
    text-align: center;
    flex-shrink: 0;
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

  /* ===== Find bar ===== */
  .find-bar {
    display: flex;
    align-items: center;
    gap: var(--spacing-xs);
    margin: var(--spacing-sm) var(--spacing-md) 0;
    padding: 0 var(--spacing-sm);
    background: var(--bg-card);
    border: 1px solid var(--border);
    border-radius: var(--radius-md);
    flex-shrink: 0;
  }

  .find-bar:focus-within {
    border-color: var(--accent);
  }

  .find-bar-icon {
    font-size: 18px;
    color: var(--text-secondary);
    flex-shrink: 0;
  }

  .find-input {
    flex: 1;
    min-width: 0;
    border: none;
    background: transparent;
    color: var(--text-primary);
    font-size: var(--font-size-sm);
    outline: none;
    padding: 8px 0;
  }

  .find-count {
    font-size: var(--font-size-sm);
    color: var(--text-secondary);
    white-space: nowrap;
    flex-shrink: 0;
  }

  .find-nav,
  .find-close {
    display: flex;
    align-items: center;
    justify-content: center;
    width: 28px;
    height: 28px;
    border-radius: var(--radius-sm);
    color: var(--text-secondary);
    flex-shrink: 0;
    transition: background var(--transition-fast), color var(--transition-fast);
  }

  .find-nav:hover,
  .find-close:hover {
    background: var(--color-surface-container-high);
    color: var(--text-primary);
  }

  .find-nav .material-symbols-outlined,
  .find-close .material-symbols-outlined {
    font-size: 18px;
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
    color: var(--text-on-accent-muted);
  }

  /* ===== Markdown ===== */
  .chat-markdown :global(p) {
    margin-bottom: var(--spacing-sm);
  }

  .chat-markdown :global(mark.find-match) {
    background: var(--warning);
    color: var(--color-on-surface);
    border-radius: 2px;
    padding: 0 1px;
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
    background: var(--surface-on-accent-subtle);
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
    color: var(--text-on-accent-high);
    text-decoration: underline;
  }

  /* ===== Citations ===== */
  .chat-markdown :global(a.chat-citation) {
    color: var(--accent);
    font-weight: 700;
    cursor: pointer;
    font-size: 12px;
    padding: 0 2px;
    user-select: none;
    text-decoration: none;
  }

  .chat-markdown :global(a.chat-citation:hover) {
    text-decoration: underline;
  }

  /* ===== Entity pills ===== */
  .chat-markdown :global(.entity-pill) {
    display: inline-flex;
    align-items: center;
    gap: 3px;
    padding: 1px 6px;
    color: white;
    border-radius: var(--radius-sm);
    font-size: var(--font-size-sm);
    line-height: 1.4;
    vertical-align: middle;
  }

  .chat-markdown :global(.entity-pill-clickable) {
    border: none;
    cursor: pointer;
    font-family: inherit;
    transition: opacity var(--transition-fast);
  }

  .chat-markdown :global(.entity-pill-clickable:hover) {
    opacity: 0.85;
  }

  .message-user .chat-markdown :global(.entity-pill) {
    opacity: 0.85;
  }

  .chat-markdown :global(.entity-pill-type) {
    font-weight: 700;
    font-size: 10px;
    opacity: 0.9;
  }

  .chat-markdown :global(.entity-pill-title) {
    font-weight: 500;
  }

  /* ===== Sources footer ===== */
  .sources-footer {
    margin-top: var(--spacing-sm);
    padding-top: var(--spacing-sm);
    border-top: 1px solid var(--border);
  }

  .message-user .sources-footer {
    border-top-color: var(--border-on-accent);
  }

  .sources-toggle {
    display: flex;
    align-items: center;
    gap: var(--spacing-xs);
    font-size: var(--font-size-sm);
    color: var(--text-secondary);
    padding: 2px 0;
    transition: color var(--transition-fast);
  }

  .sources-toggle:hover {
    color: var(--text-primary);
  }

  .message-user .sources-toggle {
    color: var(--text-on-accent-muted);
  }

  .message-user .sources-toggle:hover {
    color: white;
  }

  .sources-chevron {
    font-size: 16px;
    transition: transform var(--transition-fast);
  }

  .sources-chevron.expanded {
    transform: rotate(90deg);
  }

  .sources-list {
    margin-top: var(--spacing-xs);
    display: flex;
    flex-direction: column;
    gap: 2px;
  }

  .source-item {
    display: flex;
    align-items: center;
    gap: var(--spacing-sm);
    width: 100%;
    padding: var(--spacing-xs) var(--spacing-sm);
    border-radius: var(--radius-sm);
    font-size: var(--font-size-sm);
    color: var(--text-primary);
    text-align: left;
    transition: background var(--transition-fast);
    cursor: pointer;
  }

  .source-item:hover {
    background: var(--color-surface-container-high);
  }

  .message-user .source-item {
    color: var(--text-on-accent-high);
  }

  .message-user .source-item:hover {
    background: var(--hover-on-accent);
  }

  .source-number {
    font-weight: 700;
    color: var(--accent);
    min-width: 24px;
    flex-shrink: 0;
  }

  .message-user .source-number {
    color: var(--text-on-accent-high);
  }

  .source-type {
    font-weight: 600;
    padding: 1px 6px;
    border-radius: var(--radius-sm);
    font-size: 11px;
    flex-shrink: 0;
  }

  .source-title {
    font-weight: 500;
    flex-shrink: 0;
  }

  .source-open-btn {
    display: flex;
    align-items: center;
    justify-content: center;
    width: 24px;
    height: 24px;
    border-radius: var(--radius-sm);
    color: var(--text-secondary);
    flex-shrink: 0;
    transition: background var(--transition-fast), color var(--transition-fast);
  }

  .source-open-btn:hover {
    background: var(--color-surface-container-high);
    color: var(--accent);
  }

  .source-open-btn.source-error {
    color: var(--danger);
  }

  .source-open-btn.source-error:hover {
    background: var(--danger-soft);
    color: var(--danger);
  }

  .source-status {
    display: flex;
    align-items: center;
    justify-content: center;
    width: 24px;
    height: 24px;
    flex-shrink: 0;
    color: var(--text-secondary);
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
    color: var(--text-on-accent-faint);
  }

  .message-user .feedback-btn:hover {
    background: var(--hover-on-accent);
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
    background: var(--danger-soft);
    border-top: 1px solid var(--danger-soft-border);
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

  /* ===== Dropdown (shared between command palette and entity) ===== */
  .dropdown {
    max-height: 240px;
    overflow-y: auto;
    background: var(--color-surface);
    border: 1px solid var(--border);
    border-radius: var(--radius-md);
    box-shadow: var(--shadow-lg);
    z-index: 50;
  }

  .dropdown-item {
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

  .dropdown-item:hover,
  .dropdown-item-selected {
    background: var(--color-surface-container-high);
  }

  .dropdown-item-name {
    font-weight: 600;
    color: var(--accent);
    flex-shrink: 0;
  }

  .dropdown-item-args {
    font-weight: 500;
    color: var(--text-secondary);
    font-size: 11px;
    flex-shrink: 0;
  }

  .dropdown-item-desc {
    color: var(--text-secondary);
    flex: 1;
    min-width: 0;
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }

  .dropdown-item-type {
    font-weight: 600;
    padding: 1px 6px;
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

  /* ===== Citation tooltip ===== */
  .citation-tooltip {
    position: fixed;
    z-index: 300;
    background: var(--color-surface-container-high);
    border: 1px solid var(--border);
    border-radius: var(--radius-md);
    padding: var(--spacing-sm) var(--spacing-md);
    box-shadow: var(--shadow-lg);
    max-width: 300px;
    display: flex;
    flex-direction: column;
    gap: 2px;
    pointer-events: none;
  }

  .tooltip-type {
    font-weight: 600;
    font-size: 11px;
    color: var(--text-secondary);
  }

  .tooltip-title {
    font-weight: 500;
    font-size: var(--font-size-sm);
    color: var(--text-primary);
  }

  .tooltip-snippet {
    font-size: var(--font-size-sm);
    color: var(--text-secondary);
    line-height: 1.3;
  }

  .tooltip-source {
    display: flex;
    align-items: center;
    gap: 4px;
    font-size: 11px;
    color: var(--accent);
    margin-top: 2px;
    word-break: break-all;
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

  .input-controls {
    display: flex;
    gap: var(--spacing-xs);
    align-items: stretch;
    flex-shrink: 0;
  }

  .icon-btn {
    display: flex;
    align-items: center;
    justify-content: center;
    width: 40px;
    border-radius: var(--radius-md);
    color: var(--text-secondary);
    flex-shrink: 0;
    transition: background var(--transition-fast), color var(--transition-fast);
  }

  .icon-btn:hover:not(:disabled) {
    background: var(--color-surface-container-high);
    color: var(--text-primary);
  }

  .icon-btn:disabled {
    opacity: 0.5;
    cursor: not-allowed;
  }

  .icon-btn-active {
    background: var(--color-surface-container-high);
    color: var(--accent);
  }

  .text-input-wrapper {
    flex: 1;
    display: flex;
    align-items: flex-start;
    gap: var(--spacing-sm);
    padding: var(--spacing-xs) var(--spacing-md);
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
    margin-top: 10px;
  }

  .text-input {
    flex: 1;
    border: none;
    background: transparent;
    color: var(--text-primary);
    font-size: var(--font-size-md);
    outline: none;
    padding: 10px 0;
    font-family: inherit;
    line-height: 1.5;
    resize: none;
    min-height: 24px;
    max-height: 200px;
  }

  .text-input:disabled {
    opacity: 0.5;
  }

  .text-input::placeholder {
    color: var(--text-secondary);
    opacity: 0.6;
  }

  .text-input::-webkit-scrollbar {
    width: 4px;
  }

  .text-input::-webkit-scrollbar-thumb {
    background: var(--border);
    border-radius: 2px;
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
    background: var(--danger-hover);
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
    background: var(--danger-hover);
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
