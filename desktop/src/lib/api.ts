// === Knowledge OS — Tauri IPC API wrappers ===
// Each function invokes a Rust Tauri command on the backend.

import { invoke } from "@tauri-apps/api/core";
import type {
  EntitySummary,
  EntityDetail,
  ImportResult,
  SearchResult,
  GraphOutput,
  TreeOutput,
  TableOutput,
  TimelineOutput,
} from "./types.js";

export async function listEntities(
  entityType?: string
): Promise<EntitySummary[]> {
  return invoke("list_entities", { entityType: entityType ?? null });
}

export async function importFiles(paths: string[]): Promise<ImportResult> {
  return invoke("import_files", { paths });
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
