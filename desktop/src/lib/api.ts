// === Knowledge OS — Tauri IPC API wrappers ===
// Each function invokes a Rust Tauri command on the backend.

import { invoke } from "@tauri-apps/api/core";
import type {
  ChatSendResult,
  ConversationSummary,
  EntitySearchResult,
  EntitySummary,
  EntityDetail,
  ImportResult,
  ImportProgressResult,
  DirectoryPreview,
  StructuredPreview,
  UndoResult,
  SearchResult,
  GraphOutput,
  TreeOutput,
  TableOutput,
  TimelineOutput,
  ProviderStatus,
  TestResult,
} from "./types.js";

export async function listEntities(
  entityType?: string
): Promise<EntitySummary[]> {
  return invoke("list_entities", { entityType: entityType ?? null });
}

export async function importFiles(paths: string[]): Promise<ImportProgressResult> {
  return invoke("import_files", { paths });
}

export async function importUrl(url: string): Promise<ImportProgressResult> {
  return invoke("import_url", { url });
}

export async function importClipboard(
  text: string,
  sourceFormat?: string
): Promise<ImportProgressResult> {
  return invoke("import_clipboard", { text, sourceFormat: sourceFormat ?? null });
}

export async function importDatabase(
  connectionString: string,
  tables?: string[]
): Promise<ImportProgressResult> {
  return invoke("import_database", {
    connectionString,
    tables: tables ?? [],
  });
}

export async function importFileRecursive(path: string): Promise<ImportProgressResult> {
  return invoke("import_file_recursive", { path });
}

export async function importImage(path: string): Promise<ImportProgressResult> {
  return invoke("import_image", { path });
}

export async function undoImport(
  importId?: string
): Promise<UndoResult> {
  return invoke("undo_import", { importId: importId ?? null });
}

export async function importDirectoryPreview(
  path: string,
  recursive?: boolean
): Promise<DirectoryPreview> {
  return invoke("import_directory_preview", { path, recursive: recursive ?? null });
}

export async function importStructuredPreview(
  path: string,
  format: string
): Promise<StructuredPreview> {
  return invoke("import_structured_preview", { path, format });
}

export async function importStructured(
  path: string,
  format: string,
  columnMapping?: string
): Promise<ImportProgressResult> {
  return invoke("import_structured", {
    path,
    format,
    columnMapping: columnMapping ?? null,
  });
}

export async function searchEntities(
  query: string,
  entityType?: string,
  tag?: string
): Promise<SearchResult[]> {
  return invoke("search_entities", {
    query,
    entityType: entityType ?? null,
    tag: tag ?? null,
  });
}

export async function getEntityDetail(id: string): Promise<EntityDetail> {
  return invoke("get_entity_detail", { id });
}

export async function getGraphView(
  startId?: string,
  depth: number = 2,
  entityType?: string
): Promise<GraphOutput> {
  return invoke("get_graph_view", {
    startId: startId ?? null,
    depth,
    entityType: entityType ?? null,
  });
}

export async function getTreeView(
  entityType?: string
): Promise<TreeOutput> {
  return invoke("get_tree_view", { entityType: entityType ?? null });
}

export async function getTableView(
  sort?: string,
  entityType?: string
): Promise<TableOutput> {
  return invoke("get_table_view", {
    sort: sort ?? null,
    entityType: entityType ?? null,
  });
}

export async function getTimelineView(
  entityType?: string
): Promise<TimelineOutput> {
  return invoke("get_timeline_view", { entityType: entityType ?? null });
}

export async function getEntitySource(
  id: string
): Promise<string | null> {
  return invoke("get_entity_source", { id });
}

export async function openInDefaultApp(path: string): Promise<void> {
  return invoke("open_in_default_app", { path });
}

export async function openSourceFolder(path: string): Promise<void> {
  return invoke("open_source_folder", { path });
}

// === Chat API ===

export async function chatSend(
  conversationId: string | null,
  message: string,
  entityRefs: string[],
  sourceToggles: { knowledge_graph: boolean; web_search: boolean },
  mode: "fast" | "thinking"
): Promise<ChatSendResult> {
  return invoke("chat_send", {
    conversationId,
    message,
    entityRefs,
    sourceToggles,
    mode,
  });
}

export async function chatStream(
  conversationId: string | null,
  message: string,
  entityRefs: string[],
  sourceToggles: { knowledge_graph: boolean; web_search: boolean },
  mode: "fast" | "thinking"
): Promise<string> {
  return invoke("chat_stream", {
    conversationId,
    message,
    entityRefs,
    sourceToggles,
    mode,
  });
}

export async function chatSearchEntities(
  prefix: string
): Promise<EntitySearchResult[]> {
  return invoke("chat_search_entities", { prefix });
}

export async function chatListConversations(): Promise<ConversationSummary[]> {
  return invoke("chat_list_conversations");
}

export async function chatDeleteConversation(
  conversationId: string
): Promise<void> {
  return invoke("chat_delete_conversation", { conversationId });
}

export async function chatRenameConversation(
  conversationId: string,
  title: string
): Promise<void> {
  return invoke("chat_rename_conversation", { conversationId, title });
}

export async function chatStopStream(
  conversationId: string
): Promise<void> {
  return invoke("chat_stop_stream", { conversationId });
}

export async function chatSendFeedback(feedback: {
  message_id: string;
  rating: string;
  reason?: string;
  comment?: string;
}): Promise<void> {
  return invoke("chat_send_feedback", { feedback });
}

// === Provider Configuration API ===

export async function setProvider(
  providerKind: string,
  model: string,
  baseUrl?: string | null,
  apiKey?: string | null
): Promise<ProviderStatus> {
  return invoke("set_provider", {
    providerKind,
    model,
    baseUrl: baseUrl ?? null,
    apiKey: apiKey ?? null,
  });
}

export async function getProvidersStatus(): Promise<ProviderStatus> {
  return invoke("get_providers_status");
}

export async function chatTestProvider(
  providerKind: string,
  model: string,
  baseUrl?: string | null,
  apiKey?: string | null
): Promise<TestResult> {
  return invoke("chat_test_provider", {
    providerKind,
    model,
    baseUrl: baseUrl ?? null,
    apiKey: apiKey ?? null,
  });
}
