# PRD-0006: Desktop MVP — Knowledge OS Desktop Application

**Status:** Draft
**Date:** 2026-07-28
**Author:** Core maintainers
**Priority:** P0 — Experience Layer
**Depends on:** PRD-0001, PRD-0002, PRD-0003, PRD-0005

---

## Purpose

Deliver a desktop application that demonstrates the Knowledge OS value proposition: import documents from your file system, explore them through an interactive graph, search across them, and manage your personal knowledge graph — all in a local desktop app with zero cloud dependency. This is the Year 1 experience layer.

---

## Problem Statement

Knowledge OS has a fully functional CLI (`kos`) and a seven-layer pipeline, but the only user interface is a terminal. The Year 1 vision (product-vision.md) calls for "a functional knowledge engine that demonstrates the core architecture" — which means a visual, interactive application.

The current desktop app skeleton (desktop/src/index.html) shows "Desktop app coming soon." The CLI proves the engine works but does not demonstrate the value proposition: that a knowledge graph is more useful than a folder of files. A desktop GUI with import, search, graph traversal, and view projections makes the abstract architecture tangible.

Without this, the system remains a library, not a product. Users cannot evaluate whether the knowledge graph thesis holds for their workflow.

---

## Scope

### In Scope

- **Desktop shell** — Tauri application window with menu bar, keyboard shortcuts, and native file dialogs
- **File import** — Drag-and-drop and file-picker import of Markdown and PDF files, reusing existing importers from `knowledge-import`
- **Entity browser** — List view of all entities with type-based filtering, search bar, and sort
- **Entity detail panel** — Side panel showing entity components, relationships (incoming/outgoing), event history, and version history
- **Graph view** — Interactive graph visualization rendering entities as nodes and relationships as edges, using the existing `GraphViewAdapter` from `knowledge-derivation`
- **Tree view** — Hierarchical tree of entities grouped by type, using the existing `TreeViewAdapter`
- **Table view** — Sortable table with columns for entity type, title, tags, created/updated dates, using the existing `TableViewAdapter`
- **Timeline view** — Chronological view of entities by creation time, using the existing `TimelineViewAdapter`
- **Search** — Keyword search with type and tag filters, reusing the existing `SearchIndex`
- **Local SQLite database** — All data stored locally in the user's data directory, no cloud sync

### Out of Scope

- **AI integration** — Semantic search, embeddings, conversational interface (deferred to Year 2, see product-vision.md)
- **Plugin management GUI** — CLI `plugin` commands remain the interface for now
- **Entity editing** — Create, update, archive/restore entities from within the app (deferred to PRD-0007)
- **Relationship editing** — No GUI for creating or modifying relationships
- **Collection management** — No GUI for collection CRUD or membership
- **Multi-user or collaboration** — Single-user desktop only
- **Preferences / settings UI** — No settings panel; configuration via CLI flags and environment variables
- **Cross-platform build infrastructure** — CI/CD for DMG, MSI, AppImage (separate work)

---

## Engineering Questions

1. **Which canonical entities are introduced?** None. The desktop app consumes canonical entities through the existing `SqliteStore`. No new entity types.

2. **Which relationships are introduced?** None. The app reads relationships from the existing store.

3. **Which components are introduced?** None. The app reads existing components.

4. **Which events are emitted?** None directly. Import operations emit events through the existing pipeline (EntityCreated via `save_entity_with_components`).

5. **Which derived representations are generated?** None new. The app renders existing derived representations: search index results, view adapter projections (tree, graph, table, timeline).

6. **Which layer owns the feature?** Layer 7 — Presentation Layer. The desktop app is a renderer of canonical and derived data.

7. **Can every derived artifact be regenerated?** Yes. The desktop app does not create new derived artifacts; it renders existing ones. If the user closes the app and reopens, all views are rebuilt from canonical data.

8. **Does the feature violate storage independence?** No. The app uses `SqliteStore` through the same `EntityRepository` / `TraversalPort` / `ViewAdapter` traits. Any storage adapter can replace SQLite.

9. **Does the feature introduce implementation leakage?** No. The desktop app is a consumer of the existing port interfaces. No storage or pipeline details leak into the UI layer.

10. **Does the feature preserve the canonical model?** Yes. The app is read-mostly (import writes through existing canonical paths). It never bypasses the canonical model.

---

## Pipeline Spine Analysis

The desktop app sits at Layer 7 (Presentation Layer) and consumes data from all lower layers through existing port interfaces:

```
  Layer 1-6: Existing pipeline (import, parse, normalize, model, relate, derive)
       |
  Layer 7: Desktop MVP
       |
   Tauri IPC commands (Rust backend)
       |
   Web frontend (HTML/JS/CSS in Tauri webview)
       |
   Interactive views: graph, tree, table, timeline, entity detail
```

The pipeline extension introduces no new layers. It adds one new component — the Tauri IPC bridge — that maps UI actions to existing port calls.

---

## Functional Requirements

### F1: Desktop Shell

| ID   | Requirement                                                                           | Priority | Acceptance Criteria                                                                          |
| ---- | ------------------------------------------------------------------------------------- | -------- | -------------------------------------------------------------------------------------------- |
| F1.1 | App window opens to a dashboard view with navigation sidebar                          | P0       | On launch, user sees sidebar with tabs: Import, Browse, Search, Graph, Tree, Table, Timeline |
| F1.2 | App stores database at `$DATA_DIR/knowledge.db` (OS-appropriate default, overridable) | P0       | First launch creates database; subsequent launches reuse existing data                       |
| F1.3 | App shows a status bar with entity count and database path                            | P1       | Entity count updates after import operations                                                 |
| F1.4 | App supports macOS, Windows, and Linux                                                | P0       | `tauri build` produces valid artifacts for all three platforms                               |

### F2: File Import

| ID   | Requirement                                                                      | Priority | Acceptance Criteria                                         |
| ---- | -------------------------------------------------------------------------------- | -------- | ----------------------------------------------------------- |
| F2.1 | User can drag-and-drop Markdown and PDF files onto the app window to import them | P0       | Dropped files appear in the entity browser within 2 seconds |
| F2.2 | User can click an "Import" button to open a native file picker dialog            | P0       | Multi-file selection supported                              |
| F2.3 | User can import a directory; all `.md` and `.pdf` files are imported recursively | P1       | Directory picker selects all matching files                 |
| F2.4 | Import progress is shown (spinner or progress bar)                               | P1       | User sees which file is being processed during import       |
| F2.5 | Import results are shown: created vs merged count                                | P1       | Summary displayed after import completes                    |
| F2.6 | Import errors are surfaced (file name + error message)                           | P1       | Error list displayed with "Retry" option                    |

### F3: Entity Browser

| ID   | Requirement                                           | Priority | Acceptance Criteria                                                           |
| ---- | ----------------------------------------------------- | -------- | ----------------------------------------------------------------------------- |
| F3.1 | Browse tab shows a paginated list of all entities     | P0       | Each row shows: entity type badge, title (from Title component), created date |
| F3.2 | User can filter by entity type (dropdown)             | P0       | Selecting a type updates the list immediately                                 |
| F3.3 | User can sort by title, created date, or updated date | P1       | Click column header to sort ascending/descending                              |
| F3.4 | Clicking an entity row opens the entity detail panel  | P0       | Panel slides in from the right or navigates to detail view                    |

### F4: Entity Detail Panel

| ID   | Requirement                                                                                   | Priority | Acceptance Criteria                                                                   |
| ---- | --------------------------------------------------------------------------------------------- | -------- | ------------------------------------------------------------------------------------- |
| F4.1 | Detail panel shows entity ID, type badge, title, active status                                | P0       | All fields populated from canonical data                                              |
| F4.2 | Detail panel shows all components grouped by component type                                   | P0       | Content component renders as readable text                                            |
| F4.3 | Detail panel shows outgoing and incoming relationships with entity titles                     | P0       | Relationships grouped by direction; clicking a related entity navigates to its detail |
| F4.4 | Detail panel shows event log for the entity                                                   | P1       | Events listed chronologically                                                         |
| F4.5 | Detail panel shows version history                                                            | P1       | Version list with timestamps                                                          |
| F4.6 | Detail panel has a "View in Graph" button that navigates to Graph tab centered on this entity | P1       | Graph tab opens with entity selected as start node                                    |

### F5: Graph View

| ID   | Requirement                                                                             | Priority | Acceptance Criteria                                                                     |
| ---- | --------------------------------------------------------------------------------------- | -------- | --------------------------------------------------------------------------------------- |
| F5.1 | Graph tab renders an interactive graph visualization using data from `GraphViewAdapter` | P0       | Nodes are entities (showing title), edges are relationships (showing relationship type) |
| F5.2 | User can pan and zoom the graph canvas                                                  | P0       | Mouse drag pans; scroll zooms                                                           |
| F5.3 | User can click a node to select it and view entity details                              | P0       | Selected node is highlighted; entity detail panel shows                                 |
| F5.4 | User can specify a start entity and traversal depth                                     | P0       | Input fields for entity ID and depth; "Explore" button triggers traversal               |
| F5.5 | Graph layout algorithm positions nodes to minimize overlap                              | P1       | Force-directed layout or similar                                                        |
| F5.6 | User can filter by entity type and relationship type                                    | P1       | Dropdown filters update the graph in place                                              |

### F6: Tree View

| ID   | Requirement                                                                          | Priority | Acceptance Criteria                                             |
| ---- | ------------------------------------------------------------------------------------ | -------- | --------------------------------------------------------------- |
| F6.1 | Tree tab renders a hierarchical tree grouped by entity type, using `TreeViewAdapter` | P0       | Root nodes are entity types; children are entities of that type |
| F6.2 | User can expand/collapse branches                                                    | P0       | Click toggles branch visibility                                 |
| F6.3 | User can filter by entity type (pre-select which types appear)                       | P1       | Filter checkboxes update tree in place                          |

### F7: Table View

| ID   | Requirement                                                 | Priority | Acceptance Criteria                                 |
| ---- | ----------------------------------------------------------- | -------- | --------------------------------------------------- |
| F7.1 | Table tab renders a sortable table using `TableViewAdapter` | P0       | Columns: entity type, title, tags, created, updated |
| F7.2 | User can sort by any column                                 | P0       | Click column header to sort                         |
| F7.3 | User can filter by search query                             | P1       | Text input filters rows in real-time                |
| F7.4 | Clicking a row opens the entity detail panel                | P1       | Same panel as F4                                    |

### F8: Timeline View

| ID   | Requirement                                                                                 | Priority | Acceptance Criteria                           |
| ---- | ------------------------------------------------------------------------------------------- | -------- | --------------------------------------------- |
| F8.1 | Timeline tab renders entities chronologically by creation date, using `TimelineViewAdapter` | P0       | Entities displayed as cards on a timeline     |
| F8.2 | User can filter by entity type                                                              | P1       | Dropdown filter updates timeline              |
| F8.3 | User can zoom the timeline (day/week/month/year groups)                                     | P2       | Grouping control changes timeline granularity |

### F9: Search

| ID   | Requirement                                                                | Priority | Acceptance Criteria                            |
| ---- | -------------------------------------------------------------------------- | -------- | ---------------------------------------------- |
| F9.1 | Global search bar in the sidebar executes keyword search via `SearchIndex` | P0       | Results appear as user types (debounced 300ms) |
| F9.2 | Search results show entity title, type, and snippet                        | P0       | Snippet from content component                 |
| F9.3 | User can filter search by entity type and tag                              | P1       | Filter controls next to search bar             |
| F9.4 | Clicking a search result opens the entity detail panel                     | P0       | Same panel as F4                               |

---

## Non-Functional Requirements

### NF1: Performance

| ID    | Requirement                                      | Target  | Acceptable |
| ----- | ------------------------------------------------ | ------- | ---------- |
| NF1.1 | App cold start (first launch or after OS reboot) | < 3s    | < 5s       |
| NF1.2 | Import single Markdown file (>100KB)             | < 500ms | < 1s       |
| NF1.3 | Import batch of 100 files (10KB avg)             | < 30s   | < 60s      |
| NF1.4 | Graph view load at 1000 entities                 | < 1s    | < 3s       |
| NF1.5 | Search response (100K entities indexed)          | < 100ms | < 500ms    |
| NF1.6 | Entity detail panel load                         | < 200ms | < 500ms    |
| NF1.7 | App memory usage at idle                         | < 50MB  | < 100MB    |
| NF1.8 | App memory usage after importing 10K entities    | < 200MB | < 500MB    |

### NF2: UX

| ID    | Requirement                                                 | Target                                  |
| ----- | ----------------------------------------------------------- | --------------------------------------- |
| NF2.1 | All user-facing strings are in English (i18n-ready pattern) | Messages in constants, not inline       |
| NF2.2 | App respects OS dark/light mode preference                  | System theme detection                  |
| NF2.3 | Keyboard navigation for all primary actions                 | Tab, Enter, Escape per ui-philosophy.md |
| NF2.4 | Accessible label on all interactive elements                | aria-label or equivalent                |
| NF2.5 | App window remembers size and position between sessions     | Window state persisted to config file   |

---

## User Stories

### US1: First-Time User Import

**As a** new user,
**I want to** drag a folder of Markdown notes onto the app,
**So that** my notes appear in the knowledge graph immediately.

**Acceptance criteria:**
1. Drag-and-drop a directory onto the app window triggers import
2. CLI-equivalent import pipeline runs (resolution, cross-references, indexing)
3. Entity browser populates with imported entities within 30s for 100 files
4. Success/failure summary displayed

### US2: Graph Exploration

**As a** researcher,
**I want to** start from a paper and traverse its references,
**So that** I discover related work I was not aware of.

**Acceptance criteria:**
1. Navigate to the paper entity detail panel
2. Click "View in Graph" button
3. Graph tab opens centered on the paper entity
4. Set traversal depth to 2 and click "Explore"
5. Graph shows the paper, its direct references, and their references
6. Clicking any node opens its detail panel

### US3: Daily Knowledge Review

**As a** knowledge worker,
**I want to** see a timeline of everything I added this week,
**So that** I can review and organize my recent learning.

**Acceptance criteria:**
1. Navigate to Timeline tab
2. Set group to "week"
3. See entities grouped by week
4. Click any entity to view details
5. Filter by entity type to see only concepts or only papers

### US4: Find What I Need

**As a** user with 1000+ entities,
**I want to** search by keyword and filter by type,
**So that** I find relevant entities quickly.

**Acceptance criteria:**
1. Type in the global search bar
2. Results appear within 300ms of stopping typing
3. Filter results by entity type via dropdown
4. Click a result to open entity detail
5. Search finds entities by title and content

---

## Architecture

### Crate / Package Changes

| Crate / Package                     | Change                                                                                                      |
| ----------------------------------- | ----------------------------------------------------------------------------------------------------------- |
| `desktop/src-tauri/Cargo.toml`      | Add dependencies: `knowledge-derivation`, `knowledge-import`, `knowledge-plugin`, `chrono`, `uuid`, `serde` |
| `desktop/src-tauri/src/lib.rs`      | Rewrite — add Tauri commands exposing core operations                                                       |
| `desktop/src-tauri/src/commands.rs` | New — Tauri IPC command handlers                                                                            |
| `desktop/package.json`              | New — Svelte 5 + Vite 6 + TypeScript project definition                                                     |
| `desktop/vite.config.ts`            | New — Vite config with `@sveltejs/vite-plugin-svelte`                                                       |
| `desktop/svelte.config.js`          | New — Svelte 5 compiler config (runes mode)                                                                 |
| `desktop/tsconfig.json`             | New — TypeScript config                                                                                     |
| `desktop/src/main.ts`               | New — Svelte app entry point (mounts App.svelte)                                                            |
| `desktop/src/App.svelte`            | New — Root Svelte component (router + sidebar)                                                              |
| `desktop/src/app.css`               | New — Global styles, CSS custom properties for theming                                                      |
| `desktop/src/views/`                | New — Svelte view components (`.svelte` files)                                                              |
| `desktop/src/lib/`                  | New — TypeScript modules (api.ts, types.ts, state.svelte.ts, router.svelte.ts)                              |

### Tauri Command Architecture

The Rust backend exposes Tauri IPC commands that wrap existing core port calls. Each command is stateless — it acquires a connection to `SqliteStore` and dispatches to the appropriate trait method.

```rust
// lib.rs — command handlers

#[tauri::command]
async fn list_entities(
    state: tauri::State<'_, AppState>,
    entity_type: Option<String>,
) -> Result<Vec<EntitySummary>, String> {
    let store = state.store.lock().map_err(|e| e.to_string())?;
    let entities = match entity_type {
        Some(t) => EntityRepository::find_by_type(&*store, &t).await.map_err(|e| e.to_string())?,
        None => EntityRepository::list(&*store).await.map_err(|e| e.to_string())?,
    };
    // Map to summary type with title component
    let mut summaries = Vec::new();
    for entity in &entities {
        let components = ComponentRepository::get(&*store, entity.id).await.map_err(|e| e.to_string())?;
        let title = components.iter()
            .find(|c| c.component_type == ComponentType::Title)
            .and_then(|c| c.data.as_str().map(String::from))
            .unwrap_or_default();
        summaries.push(EntitySummary {
            id: entity.id,
            entity_type: entity.entity_type.as_str().to_string(),
            title,
            is_active: entity.is_active,
            created_at: entity.created_at.to_rfc3339(),
            updated_at: entity.updated_at.to_rfc3339(),
        });
    }
    Ok(summaries)
}

#[tauri::command]
async fn import_files(
    state: tauri::State<'_, AppState>,
    paths: Vec<String>,
) -> Result<ImportResult, String> {
    // Reuse existing import pipeline from knowledge-import
}

#[tauri::command]
async fn search_entities(
    state: tauri::State<'_, AppState>,
    query: String,
    entity_type: Option<String>,
    tag: Option<String>,
) -> Result<Vec<SearchResult>, String> {
    // Delegate to SearchIndex::search
}

#[tauri::command]
async fn get_entity_detail(
    state: tauri::State<'_, AppState>,
    id: String,
) -> Result<EntityDetail, String> {
    // Aggregate entity + components + relationships + events + versions
}

#[tauri::command]
async fn get_graph_view(
    state: tauri::State<'_, AppState>,
    start_id: Option<String>,
    depth: u32,
    entity_type: Option<String>,
) -> Result<GraphOutput, String> {
    // Delegate to GraphViewAdapter::render
}

#[tauri::command]
async fn get_tree_view(
    state: tauri::State<'_, AppState>,
    entity_type: Option<String>,
) -> Result<TreeOutput, String> {
    // Delegate to TreeViewAdapter::render
}

#[tauri::command]
async fn get_table_view(
    state: tauri::State<'_, AppState>,
    sort: Option<String>,
    filter: Option<String>,
    entity_type: Option<String>,
) -> Result<TableOutput, String> {
    // Delegate to TableViewAdapter::render
}

#[tauri::command]
async fn get_timeline_view(
    state: tauri::State<'_, AppState>,
    entity_type: Option<String>,
) -> Result<TimelineOutput, String> {
    // Delegate to TimelineViewAdapter::render
}
```

### AppState

```rust
struct AppState {
    store: Arc<Mutex<SqliteStore>>,
}
```

### Frontend Architecture (Svelte 5 + Vite + TypeScript)

The frontend is a single-page application built with **Svelte 5** (runes mode), **Vite 6**, and **TypeScript 5**. Vite provides HMR during development and optimized builds for production. Svelte 5's runes (`$state`, `$derived`, `$effect`) provide fine-grained reactivity without a virtual DOM.

```
desktop/src/
  main.ts                     — Svelte app entry point (mounts App.svelte)
  App.svelte                  — Root component: sidebar navigation, router outlet
  app.css                     — Global styles, CSS custom properties, theme tokens
  vite-env.d.ts               — Vite client type declarations
  views/
    Dashboard.svelte          — Landing page with entity stats and recent activity
    Import.svelte             — Drag-drop zone, file picker, progress indicator
    Browser.svelte            — Entity list with type filter, sort, pagination
    Detail.svelte             — Entity detail panel (split or slide-over)
    Graph.svelte              — Interactive graph (SVG + D3-force layout)
    Tree.svelte               — Hierarchical tree (collapsible)
    Table.svelte              — Sortable data table
    Timeline.svelte           — Chronological card timeline
    Search.svelte             — Search bar + results panel
  lib/
    api.ts                    — Typed Tauri invoke wrappers
    types.ts                  — TypeScript interfaces mirroring Rust types
    state.svelte.ts           — Global reactive state using Svelte 5 runes
    router.svelte.ts          — Hash-based router using $state
    graph-layout.ts           — D3-force layout worker (via comlink or inline)
    theme.ts                  — OS theme detection, dark/light mode toggle
```

#### State Management

Application state uses Svelte 5 runes in a module-level store, avoiding the old Svelte 4 `writable` store pattern:

```typescript
// lib/state.svelte.ts
let currentView = $state<View>('dashboard');
let selectedEntityId = $state<string | null>(null);
let entities = $state<EntitySummary[]>([]);
let searchQuery = $state('');
let isImporting = $state(false);

export function getState() {
  return {
    get currentView() { return currentView; },
    set currentView(v: View) { currentView = v; },
    get selectedEntityId() { return selectedEntityId; },
    set selectedEntityId(id: string | null) { selectedEntityId = id; },
    // ... etc
  };
}
```

#### Routing

A lightweight hash-based router implemented as a Svelte 5 component:

```typescript
// lib/router.svelte.ts
const routes: Record<string, Component> = {
  '/': Dashboard,
  '/browse': Browser,
  '/graph': Graph,
  '/tree': Tree,
  '/table': Table,
  '/timeline': Timeline,
  '/import': Import,
  '/entity/:id': Detail,
};
```

### Graph Visualization Strategy

The graph view uses **D3-force** for layout computation and **SVG** for rendering, embedded in a Svelte 5 component:

- **Layout**: `d3-force-3d` (forceSimulation with link, charge, center forces) runs in the main thread with `requestAnimationFrame` throttling. For >500 nodes, computation moves to a Web Worker via `comlink`.
- **Rendering**: SVG elements (circles for nodes with `<text>` labels, `<line>` or `<path>` for edges with arrowheads). SVG enables native CSS hover states, click handlers, and accessible labels.
- **Interaction**: Pan/zoom via SVG `transform` + D3-zoom; click to select entity (updates `selectedEntityId` in state); hover shows tooltip with title and type.
- **Data source**: `GraphViewAdapter::render()` returns nodes with positions computed on the Rust side using a simple initial layout (Kamada-Kawai or circle), then D3-force refines positions client-side.

**Why SVG over Canvas for MVP:**
- Easier hit testing (native DOM events vs manual coordinate math)
- Crisp text at any zoom level
- Accessible (SVG elements can have `aria-label`)
- Faster to iterate during development
- D3-force integrates naturally with SVG selection/transition

---

## CLI Interface

No new CLI commands. The desktop app consumes the same backend. The existing `kos` CLI continues to work alongside the desktop app (they share the same SQLite database).

### New Tauri IPC Commands (Internal)

These are not user-facing CLI commands but Rust functions exposed to the frontend via Tauri's IPC:

```
invoke('list_entities', { entity_type: "Paper" })       -> EntitySummary[]
invoke('import_files', { paths: ["/path/to/file.md"] }) -> ImportResult
invoke('search_entities', { query: "machine learning" }) -> SearchResult[]
invoke('get_entity_detail', { id: "uuid" })             -> EntityDetail
invoke('get_graph_view', { start_id: "uuid", depth: 2 }) -> GraphOutput
invoke('get_tree_view', { entity_type: null })           -> TreeOutput
invoke('get_table_view', { sort: "title" })              -> TableOutput
invoke('get_timeline_view', { entity_type: null })       -> TimelineOutput
```

---

## Acceptance Criteria

### Definition of Done

- [ ] User can launch the desktop app on macOS, Windows, and Linux
- [ ] User can import a Markdown file via drag-and-drop and see it in the entity browser
- [ ] User can search entities by keyword and filter by type
- [ ] User can click an entity to view its details (components, relationships, events)
- [ ] User can explore the graph view starting from an entity
- [ ] User can view entities in tree, table, and timeline projections
- [ ] All view adapters render correctly through the frontend
- [ ] Existing CLI commands continue to work with the same database
- [ ] All 81 existing `knowledge-storage` tests pass
- [ ] All integration tests pass

### Test Cases

1. **Cold start**: Launch app, verify dashboard shows empty state. Import a file. Verify entity appears. Close and reopen app. Verify entity persists.
2. **Import round-trip**: Drop 3 Markdown files. Verify all 3 appear in entity browser. Verify auto-merge works if same file is dropped again.
3. **Search**: Import 10 files with varied titles. Search for a partial title match. Verify correct entities appear in results.
4. **Graph navigation**: Import files with cross-references. Open graph view from entity A at depth 2. Verify referenced entities appear as connected nodes.
5. **View switching**: Navigate between all 5 view tabs without error. Verify each view renders data without crashing.
6. **Entity detail**: Click entity in browser. Verify detail panel shows title, type, components. Click a relationship target. Verify detail panel switches to that entity.
7. **Cross-database compatibility**: Run `kos import` CLI commands on the same database. Verify entities appear in desktop app on relaunch.

---

## Testing Strategy

| Level       | Scope                             | Framework                                    |
| ----------- | --------------------------------- | -------------------------------------------- |
| Unit        | Tauri command handlers (Rust)     | `cargo test` (existing)                      |
| Integration | Import pipeline + store           | `cargo test -p knowledge-storage` (existing) |
| E2E         | Desktop app boot + UI interaction | Tauri WebDriver + Playwright/Selenium        |
| Manual      | Visual verification of all views  | QA checklist                                 |

E2E tests are deferred post-MVP. The initial testing strategy relies on:
1. Reusing existing Rust unit/integration tests for the backend
2. Manual testing of the frontend
3. The Tauri command handlers being thin wrappers around already-tested core logic

---

## Risks and Mitigations

| Risk                                                    | Impact                   | Likelihood | Mitigation                                                                           |
| ------------------------------------------------------- | ------------------------ | ---------- | ------------------------------------------------------------------------------------ |
| **Tauri API changes between versions**                  | Build breakage           | Medium     | Pin Tauri version in Cargo.toml; upgrade on a defined schedule                       |
| **Frontend bundle size grows large with graph library** | Slow startup             | Low        | Use D3-force + SVG (lightweight); defer heavy layout to Web Worker                   |
| **Cross-platform file dialog differences**              | UX inconsistency         | Medium     | Use Tauri's built-in dialog API (handles platform differences)                       |
| **Database locked across CLI and desktop**              | Concurrent access errors | Medium     | Single-process app (no concurrent CLI + desktop); document limitation                |
| **Graph layout performance at >1000 nodes**             | UI jank                  | Medium     | Limit graph to depth 3 by default; show loading indicator; use Web Worker for layout |
| **Import of 10K+ files blocks UI**                      | App becomes unresponsive | Medium     | Run import in async Tauri command; show progress; allow cancellation                 |

---

## Dependencies

### External Crates / Packages

#### Rust (Cargo)

| Crate                 | Version | Purpose                       | Justification                                           |
| --------------------- | ------- | ----------------------------- | ------------------------------------------------------- |
| `tauri`               | 2.11.x  | Desktop application framework | Cross-platform native window with webview; Rust backend |
| `tauri-plugin-dialog` | 2.x     | Native file picker dialogs    | Cross-platform open/save dialogs                        |

#### Frontend (npm)

| Package                        | Version | Purpose                     | Justification                                                                         |
| ------------------------------ | ------- | --------------------------- | ------------------------------------------------------------------------------------- |
| `svelte`                       | ^5.x    | UI framework                | Reactive component model; runes-based reactivity; smallest bundle of major frameworks |
| `@sveltejs/vite-plugin-svelte` | ^5.x    | Svelte + Vite integration   | Official plugin; handles Svelte compilation in Vite pipeline                          |
| `vite`                         | ^6.x    | Build tool and dev server   | Fast HMR; native ESM dev server; Tauri's default frontend bundler                     |
| `typescript`                   | ^5.x    | Type system                 | Type-safe IPC calls; catches API mismatches at build time                             |
| `d3-force`                     | ^3.x    | Force-directed graph layout | De facto standard for graph layout; integrates cleanly with SVG                       |
| `@tauri-apps/api`              | ^2.x    | Tauri JS bindings           | `invoke`, `event`, `window` APIs for frontend-backend communication                   |

### Internal Dependencies

- `docs/architecture/ui-philosophy.md` — View properties, navigation patterns, keyboard navigation
- `docs/architecture/pipeline.md` — Seven-layer architecture; desktop app is Layer 7
- `docs/philosophy/product-vision.md` — Year 1 vision drives MVP scope
- PRD-0001 — Core entity model (entities, components, relationships already exist)
- PRD-0003 — View adapters (TreeViewAdapter, GraphViewAdapter, TableViewAdapter, TimelineViewAdapter)
- PRD-0005 — Traversal performance (BFS graph traversal used by graph view)

---

## Timeline

| Phase                     | Duration | Deliverables                                                                             |
| ------------------------- | -------- | ---------------------------------------------------------------------------------------- |
| Phase 1: Backend IPC      | 1 week   | Tauri commands for list, get, search, import; AppState wiring; database path resolution  |
| Phase 2: Frontend shell   | 1 week   | SPA router; navigation sidebar; theme support; global search bar                         |
| Phase 3: Import + Browse  | 1 week   | Import view (drag-drop, file picker, progress); entity browser view (list, filter, sort) |
| Phase 4: Entity Detail    | 1 week   | Detail panel (components, relationships, events, versions); relationship navigation      |
| Phase 5: Graph View       | 2 weeks  | SVG + D3-force graph renderer; interactive layout; traversal controls; node selection    |
| Phase 6: View Projections | 1 week   | Tree, table, and timeline views wired to existing ViewAdapters                           |
| Phase 7: Polish           | 1 week   | Error handling, loading states, keyboard shortcuts, window state persistence, README     |

**Total: ~8 weeks**
