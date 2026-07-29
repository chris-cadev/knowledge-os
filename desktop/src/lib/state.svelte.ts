// === Knowledge OS — Global reactive state (Svelte 5 runes) ===
import type { View, EntitySummary, EntityDetail } from "./types.js";

let currentView = $state<View>("dashboard");
let selectedEntityId = $state<string | null>(null);
let selectedEntityDetail = $state<EntityDetail | null>(null);
let entities = $state<EntitySummary[]>([]);
let entityTypeFilter = $state<string | null>(null);
let searchQuery = $state("");
let isImporting = $state(false);
let importProgress = $state<string | null>(null);
let statusMessage = $state<string>("");
let entityCount = $state<number>(0);

function navigate(view: View, entityId?: string) {
  currentView = view;
  if (entityId) {
    selectedEntityId = entityId;
  }
}

export function getState() {
  return {
    get currentView() {
      return currentView;
    },
    set currentView(v: View) {
      currentView = v;
    },
    get selectedEntityId() {
      return selectedEntityId;
    },
    set selectedEntityId(id: string | null) {
      selectedEntityId = id;
    },
    get selectedEntityDetail() {
      return selectedEntityDetail;
    },
    set selectedEntityDetail(d: EntityDetail | null) {
      selectedEntityDetail = d;
    },
    get entities() {
      return entities;
    },
    set entities(e: EntitySummary[]) {
      entities = e;
    },
    get entityTypeFilter() {
      return entityTypeFilter;
    },
    set entityTypeFilter(t: string | null) {
      entityTypeFilter = t;
    },
    get searchQuery() {
      return searchQuery;
    },
    set searchQuery(q: string) {
      searchQuery = q;
    },
    get isImporting() {
      return isImporting;
    },
    set isImporting(v: boolean) {
      isImporting = v;
    },
    get importProgress() {
      return importProgress;
    },
    set importProgress(p: string | null) {
      importProgress = p;
    },
    get statusMessage() {
      return statusMessage;
    },
    set statusMessage(m: string) {
      statusMessage = m;
    },
    get entityCount() {
      return entityCount;
    },
    set entityCount(c: number) {
      entityCount = c;
    },
    navigate,
  };
}
