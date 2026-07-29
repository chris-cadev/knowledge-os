# IP-008: Desktop MVP — Tauri IPC Backend and Frontend Views

**Status:** Accepted
**ADR(s):** [ADR-0021](../../architecture/adrs/adr-0021.md) (Tauri as Desktop Application Framework), [ADR-0022](../../architecture/adrs/adr-0022.md) (Stateless Tauri IPC Bridge)
**PRD(s):** [PRD-0006](../prds/prd-0006-desktop-mvp.md)
**Estimated effort:** ~8 weeks

---

## Context

Knowledge OS has a fully functional CLI (`kos`) and seven-layer pipeline but no visual interface. PRD-0006 specifies a desktop MVP built with Tauri 2.x (ADR-0021) using stateless IPC commands to wrap existing port traits (ADR-0022). A visual design system exists at `design/knowledge_os/DESIGN.md` with HTML mockups for all views.

**Current state:**
- `desktop/src-tauri/` — minimal Tauri 2.11.3 skeleton (`lib.rs`, `main.rs`), no IPC commands
- `desktop/src-tauri/Cargo.toml` — depends on `knowledge-core`, `knowledge-storage`, but missing `knowledge-derivation`, `knowledge-import`, `knowledge-plugin`, `chrono`, `uuid`, `tauri-plugin-shell`
- `desktop/src/` — Svelte 5 + Vite 6 + TypeScript scaffolding exists with:
  - `App.svelte`, `Sidebar.svelte`, `Dashboard.svelte`, `Browser.svelte` (basic implementations)
  - `api.ts` (typed Tauri invoke wrappers — has function stubs for list, detail, graph, tree, table, timeline, import, search)
  - `types.ts` (all TypeScript interfaces mirroring Rust types)
  - `state.svelte.ts` (Svelte 5 runes state management)
  - `router.svelte.ts` (hash-based router)
  - `app.css` (placeholder theme with purple accent, not matching DESIGN.md)
- View components **not yet created**: `Detail.svelte`, `Graph.svelte`, `Tree.svelte`, `Table.svelte`, `Timeline.svelte`, `Import.svelte`, `Search.svelte`
- Library modules **not yet created**: `graph-layout.ts`, `theme.ts`
- `tauri.conf.json` — 800x600 default window, no dialog plugin capability

**Existing port traits** (in `knowledge-core/src/ports/`):
- `EntityRepository` — `get()`, `list()`, `find_by_type()`, `get_version_history()`
- `ComponentRepository` — `get()` by entity_id
- `RelationshipRepository` — `by_source()`, `by_target()` for incoming/outgoing
- `TraversalPort` — `traverse()` with configurable depth/direction
- `ViewAdapter` — `render()` producing TreeData, GraphData, TableData, TimelineData
- `SearchIndex` — `search()` with query/type/tag filters
- `EventLog` — `list_by_entity()` for event history

**View adapters** (in `knowledge-derivation/src/features/view/`):
- `GraphViewAdapter::new(entity_repo, component_repo, relationship_repo, traversal_port)`
- `TreeViewAdapter::new(entity_repo, component_repo, collection_repo)`
- `TableViewAdapter::new(entity_repo, component_repo)`
- `TimelineViewAdapter::new(entity_repo, component_repo)`

All adapters rebuild from canonical data on every `render()` call. The CLI pattern (`cli/src/main.rs` lines 30, 209–231) uses a `StoreWrapper(Arc<SqliteStore>)` that implements all port traits by delegating to the inner store. The desktop backend will use the same pattern.

---

## Deliverables

All features are delivered as a single consolidated scope — no phased rollout. The implementation covers: backend IPC commands (including source file access and file-opening), design system integration, import + browse + search views, entity detail panel with file actions, graph view, tree/table/timeline views, dashboard refinement, and polish.

### D1: Backend IPC Commands (Rust)

**Purpose:** Implement all Tauri IPC commands defined in PRD-0006 and ADR-0022, update Cargo.toml dependencies, and wire AppState.

**ADR-0021** specifies `AppState { store: Arc<Mutex<SqliteStore>> }`. **ADR-0022** specifies the stateless command pattern.

**Files:**

| File                                          | Action  | Description                                                                                                                      |
| --------------------------------------------- | ------- | -------------------------------------------------------------------------------------------------------------------------------- |
| `desktop/src-tauri/Cargo.toml`                | Modify  | Add `knowledge-derivation`, `knowledge-import`, `knowledge-plugin`, `chrono`, `uuid`, `serde`, `tauri-plugin-dialog`, `tauri-plugin-shell` |
| `desktop/src-tauri/src/lib.rs`                | Rewrite | Add `AppState`, import `commands` module, register commands with `tauri::Builder::invoke_handler()`, register dialog + shell plugins |
| `desktop/src-tauri/src/commands.rs`           | Create  | 11 IPC command functions + `StoreWrapper` + response types                                                                       |
| `desktop/src-tauri/src/commands/import.rs`    | Create  | `import_files` command delegating to import pipeline                                                                             |
| `desktop/src-tauri/src/commands/entity.rs`    | Create  | `list_entities`, `get_entity_detail`, `get_entity_source` commands                                                               |
| `desktop/src-tauri/src/commands/view.rs`      | Create  | `get_graph_view`, `get_tree_view`, `get_table_view`, `get_timeline_view` commands                                                |
| `desktop/src-tauri/src/commands/search.rs`    | Create  | `search_entities` command                                                                                                        |
| `desktop/src-tauri/src/commands/file.rs`      | Create  | `open_in_default_app`, `open_source_folder` commands via `tauri-plugin-shell`                                                    |
| `desktop/src-tauri/src/commands/mod.rs`       | Create  | Re-export module                                                                                                                 |
| `desktop/src-tauri/capabilities/default.json` | Modify  | Add `dialog:default`, `shell:default`, and `fs:default` permissions                                                              |

**New Rust types (in `commands.rs` or response module):**

```rust
// AppState shared across all commands
pub struct AppState {
    pub store: Arc<Mutex<SqliteStore>>,
}

// StoreWrapper follows same pattern as cli/src/main.rs
struct StoreWrapper(Arc<SqliteStore>);

#[async_trait]
impl EntityRepository for StoreWrapper {
    // ... delegate all EntityRepository methods
}
#[async_trait]
impl ComponentRepository for StoreWrapper { /* delegate */ }
#[async_trait]
impl RelationshipRepository for StoreWrapper { /* delegate */ }
#[async_trait]
impl TraversalPort for StoreWrapper { /* delegate */ }

// Response types (mirror existing TypeScript interfaces)
#[derive(Serialize)]
pub struct EntitySummary {
    pub id: Uuid,
    pub entity_type: String,
    pub title: String,
    pub is_active: bool,
    pub created_at: String,
    pub updated_at: String,
}

#[derive(Serialize)]
pub struct EntityDetailResponse {
    pub id: Uuid,
    pub entity_type: String,
    pub is_active: bool,
    pub created_at: String,
    pub updated_at: String,
    pub components: Vec<ComponentData>,
    pub outgoing_relationships: Vec<RelationshipInfo>,
    pub incoming_relationships: Vec<RelationshipInfo>,
    pub events: Vec<EventInfo>,
    pub versions: Vec<VersionInfo>,
}

#[derive(Serialize)]
pub struct ImportResultResponse {
    pub created: usize,
    pub merged: usize,
    pub errors: Vec<ImportErrorResponse>,
}
```

**Command signatures (from ADR-0022, PRD-0006 §Architecture):**

```rust
#[tauri::command]
async fn list_entities(
    state: tauri::State<'_, AppState>,
    entity_type: Option<String>,
) -> Result<Vec<EntitySummary>, String> { /* ... */ }

#[tauri::command]
async fn import_files(
    state: tauri::State<'_, AppState>,
    paths: Vec<String>,
) -> Result<ImportResultResponse, String> { /* ... */ }

#[tauri::command]
async fn search_entities(
    state: tauri::State<'_, AppState>,
    query: String,
    entity_type: Option<String>,
    tag: Option<String>,
) -> Result<Vec<SearchResultResponse>, String> { /* ... */ }

#[tauri::command]
async fn get_entity_detail(
    state: tauri::State<'_, AppState>,
    id: String,
) -> Result<EntityDetailResponse, String> { /* ... */ }

#[tauri::command]
async fn get_entity_source(
    state: tauri::State<'_, AppState>,
    id: String,
) -> Result<Option<String>, String> {
    // Extract source file path from Provenance or BinaryContent component
    // Returns None if the entity has no source file
}

#[tauri::command]
async fn open_in_default_app(
    path: String,
) -> Result<(), String> {
    // Open file in OS default application via tauri-plugin-shell
}

#[tauri::command]
async fn open_source_folder(
    path: String,
) -> Result<(), String> {
    // Reveal file in OS file manager via tauri-plugin-shell
}

#[tauri::command]
async fn get_graph_view(
    state: tauri::State<'_, AppState>,
    start_id: Option<String>,
    depth: u32,
    entity_type: Option<String>,
) -> Result<ViewOutputData, String> { /* delegates to GraphViewAdapter::render */ }

#[tauri::command]
async fn get_tree_view(
    state: tauri::State<'_, AppState>,
    entity_type: Option<String>,
) -> Result<ViewOutputData, String> { /* delegates to TreeViewAdapter::render */ }

#[tauri::command]
async fn get_table_view(
    state: tauri::State<'_, AppState>,
    sort: Option<String>,
    filter: Option<String>,
    entity_type: Option<String>,
) -> Result<ViewOutputData, String> { /* delegates to TableViewAdapter::render */ }

#[tauri::command]
async fn get_timeline_view(
    state: tauri::State<'_, AppState>,
    entity_type: Option<String>,
) -> Result<ViewOutputData, String> { /* delegates to TimelineViewAdapter::render */ }
```

**Verification:**
- `cargo check -p knowledge-desktop` compiles
- `cargo test -p knowledge-desktop` passes (adding tests for StoreWrapper delegation and command parameter mapping)
- `cargo test -p knowledge-storage` still passes (81 tests, no regressions)
- `cargo test -p knowledge-derivation` still passes (view adapter tests)
- Verify `tauri dev` launches window without crash

**Exit criteria:** All 11 IPC commands compile, `cargo check` succeeds, existing tests pass across all workspace crates.

---

### D2: Design System Integration (app.css + Theme)

**Purpose:** Replace placeholder CSS custom properties with the design system tokens from `design/knowledge_os/DESIGN.md` and implement OS dark/light mode detection.

**Files:**

| File                               | Action  | Description                                                                                                                                          |
| ---------------------------------- | ------- | ---------------------------------------------------------------------------------------------------------------------------------------------------- |
| `desktop/src/app.css`              | Rewrite | Replace all CSS custom properties with DESIGN.md tokens (colors, typography (Inter + JetBrains Mono), spacing (4px grid), radius, elevation)         |
| `desktop/src/lib/theme.ts`         | Create  | OS dark/light mode detection via `prefers-color-scheme`, exports reactive `isDark` signal + `initTheme()`                                            |
| `desktop/src/App.svelte`           | Modify  | Import `initTheme()` on mount, apply `.dark` class to `<html>` when system theme is dark                                                             |
| `desktop/src/views/Sidebar.svelte` | Modify  | Replace text icons with Material Symbol icons, add active-state blue left border strip (per design mockup), update layout to match 260px design spec |
| `desktop/index.html`               | Modify  | Add Inter + JetBrains Mono font preload links, Material Symbols stylesheet link                                                                      |

**Design token mapping (from `design/knowledge_os/DESIGN.md`):**

```
Design Token            → CSS Custom Property
primary (#004ac6)       → --color-primary
primary-container       → --color-primary-container
surface (#f7f9fb)       → --color-surface
inverse-surface         → --color-inverse-surface (sidebar dark bg)
outline (#737686)       → --color-outline
on-surface (#191c1e)    → --color-on-surface
on-surface-variant      → --color-on-surface-variant
sidebar-width (260px)   → --layout-sidebar-width
spacing unit (4px)      → --spacing-unit
Inter font              → --font-sans
JetBrains Mono          → --font-mono
radius DEFAULT (0.125rem)→ --radius-sm
radius lg (0.25rem)     → --radius-md
radius xl (0.5rem)      → --radius-lg
```

**Verification:**
- `npm run check` passes (no TypeScript/Svelte errors)
- `cargo tauri dev` shows sidebar with Material Symbols, correct colors
- Toggle OS theme between light/dark — app theme switches correctly
- Design tokens visually match `design/knowledge_os_dashboard/` mockup sidebar

**Exit criteria:** Design tokens match `DESIGN.md` spec, dark/light mode works, sidebar renders with Material Symbols.

---

### D3: Import + Browse + Search Views (Frontend)

**Purpose:** Implement the Import view (drag-drop, file picker, directory picker, progress), Browser view (type filter, sort, pagination), and Search view (debounced search, filters).

**Files:**

| File                               | Action  | Description                                                                                                                                                                                             |
| ---------------------------------- | ------- | ------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| `desktop/src/views/Import.svelte`  | Create  | Drag-drop zone, file picker button (invoking Tauri dialog), directory picker, progress indicator, import results summary (created/merged/errors)                                                         |
| `desktop/src/views/Browser.svelte` | Rewrite | Type filter dropdown, sort button, entity count label, table with type badge / title / created date columns, pagination controls                                                                        |
| `desktop/src/views/Search.svelte`  | Create  | Search bar with debounced input (300ms), results list showing entity title/type/snippet, type and tag filter controls                                                                                   |
| `desktop/src/lib/drag-drop.ts`     | Create  | Utility for handling file drop events, extracting file paths, filtering for .md/.pdf                                                                                                                    |

**Key behaviors:**
- Import: drag-drop fires `importFiles([...paths])`, also supports directory import via recursive file discovery. Shows spinner during import, shows result summary.
- Browser: type filter dropdown calls `listEntities(type)` immediately on selection, sort click toggles asc/desc, clicking row navigates to detail view.
- Search: typeahead debounced at 300ms, results clickable to open entity detail.

**Verification:**
- `npm run check` passes
- Manual: drag file onto import zone → file appears in entity list within 2s
- Manual: drag directory onto import zone → all .md/.pdf files imported recursively
- Manual: type filter dropdown updates list immediately
- Manual: search typeahead returns results within 300ms of stopping typing
- `cargo test -p knowledge-storage` still passes (import pipeline tests)

**Exit criteria:** Import, Browser, and Search views render, interact, and communicate with backend via Tauri IPC.

---

### D4: Entity Detail Panel with File Actions

**Purpose:** Implement the entity detail panel per `design/entity_browser/` mockup (400px side panel showing components, relationships, events, versions) with source file actions.

**Files:**

| File                              | Action | Description                                                                                                                                                                                                                                    |
| --------------------------------- | ------ | ---------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| `desktop/src/views/Detail.svelte` | Create | Detail panel with: entity header (type badge, ID, title, active status), components grouped by type (Content rendered as readable text), outgoing/incoming relationships lists grouped by direction with clickable targets, event log, version history, source file path display, "Open File" button, "Show in Folder" button, "View in Graph" button, close/slide-over behavior |
| `desktop/src/lib/api.ts`          | Modify | Add `getEntitySource`, `openInDefaultApp`, `openSourceFolder` function wrappers                                                                                                                                                                |

**Key behaviors:**
- Panel opens via `router.svelte.ts` hash routing: `#/entity/{id}` → sets `selectedEntityId` and switches to `detail` view
- Source file path extracted via `getEntitySource(id)` IPC call — looks up Provenance component (Markdown) or BinaryContent component (PDF)
- "Open File" button calls `openInDefaultApp(path)` IPC → `tauri-plugin-shell` opens file in OS default app
- "Show in Folder" button calls `openSourceFolder(path)` IPC → `tauri-plugin-shell` reveals file in OS file manager
- "View in Graph" button: navigates to graph tab with `selectedEntityId` as start entity
- Clicking relationship target: navigates to that entity's detail panel (same component, new data)
- Loading state while `getEntityDetail()` is in flight
- Error state if entity not found or backend error
- File action buttons disabled/hidden when entity has no source file

**Data flow:**
```
Detail.svelte mounts
  → reads state.selectedEntityId (from URL hash)
  → calls getEntityDetail(id) AND getEntitySource(id) in parallel
  → displays EntityDetailResponse (components, relationships, events, versions)
  → displays source path if present
  → "Open File" → calls openInDefaultApp(path)
  → "Show in Folder" → calls openSourceFolder(path)
```

**Verification:**
- `npm run check` passes
- Manual: click entity in Browser → detail panel opens with correct data
- Manual: click relationship target → detail switches to that entity
- Manual: click "View in Graph" → navigates to graph view with entity selected
- Manual: click "Open File" → file opens in OS default application
- Manual: click "Show in Folder" → file manager opens to containing directory
- Manual: entity imported via CLI shows source path; entity created without file has no actions

**Exit criteria:** Entity detail panel shows all components, relationships (grouped by direction), events, versions; source file actions work correctly; "View in Graph" navigates to Graph tab.

---

### D5: Graph View (SVG + D3-force)

**Purpose:** Implement interactive graph visualization per `design/interactive_graph_view/` mockup — SVG canvas with D3-force layout, traversal controls, entity inspector panel, pan/zoom.

**Files:**

| File                              | Action | Description                                                                                                                                                                                                             |
| --------------------------------- | ------ | ----------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| `desktop/src/views/Graph.svelte`  | Create | SVG canvas with dark dot-grid background (`canvas-bg` class), D3-force simulation, pan/zoom via SVG transform + D3-zoom, node selection, edge labels, traversal controls panel, entity inspector sidebar, zoom controls |
| `desktop/src/lib/graph-layout.ts` | Create | D3-force simulation factory: `forceSimulation` with `forceLink`, `forceManyBody`, `forceCenter`, `forceCollide`. Exports `startSimulation(nodes, edges)` returning `{ simulation, updatePositions }`                    |

**Graph.svelte component structure:**
```
<main class="graph-canvas">           <!-- dark dot-grid background -->
  <svg>                               <!-- SVG viewport -->
    <g class="edges">                  <!-- <line> elements with arrowheads -->
    <g class="edge-labels">            <!-- <text> labels for relationship types -->
    <g class="nodes">                  <!-- <g> groups: <circle> + <text> -->
  </svg>
  <!-- Overlay panels -->
  <div class="traversal-controls">     <!-- Entity ID input, depth slider, Explore button -->
  <div class="entity-inspector">       <!-- Entity header, description, relationships -->
  <div class="zoom-controls">          <!-- Zoom in/out, reset buttons -->
  <div class="breadcrumb">             <!-- Current focus + depth indicator -->
  <div class="legend">                 <!-- Traversal legend (colors for entity types) -->
</main>
```

**Key behaviors:**
- Pan: mouse drag on empty canvas area
- Zoom: scroll wheel, +/- buttons, reset button
- Node interaction: hover (tooltip), click (select → highlight → update entity inspector panel)
- Traversal controls: enter entity ID + depth (slider 1-5) + Explore button → calls `getGraphView(startId, depth)`
- Force layout: runs at `requestAnimationFrame` with tick updates. For >500 nodes, computation moves to Web Worker
- Arrow markers on edges via SVG `<defs><marker>`

**Verification:**
- `npm run check` passes
- Manual: graph tab loads with initial data
- Manual: enter entity ID and depth → Explore button fetches traversal
- Manual: pan and zoom via mouse
- Manual: click node → entity inspector panel updates

**Exit criteria:** Graph view renders SVG nodes/edges, D3-force layout runs, pan/zoom work, traversal explorer fetches and renders subgraph, entity inspector shows node details.

---

### D6: Tree, Table, and Timeline Views

**Purpose:** Implement the remaining view projection components per their `design/` mockups.

**Files:**

| File                                | Action | Description                                                                                                                                                                                         |
| ----------------------------------- | ------ | --------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| `desktop/src/views/Tree.svelte`     | Create | Hierarchical tree per `design/tree_view/` mockup: entity type root nodes, entity children, collapsible branches with connector lines, type filter, active entity highlight                          |
| `desktop/src/views/Table.svelte`    | Create | Sortable table per `design/table_view/` mockup: columns (entity type badge, title, tags, created, updated), sortable column headers (asc/desc indicators), search filter, click row → entity detail |
| `desktop/src/views/Timeline.svelte` | Create | Chronological timeline per `design/timeline_view/` mockup: vertical timeline line, entity cards with dates and type badges, type filter, zoom grouping (day/week/month)                             |

**Key behaviors for Tree:**
- Calls `getTreeView(entityType)` on mount and when filter changes
- Renders `TreeNode` as nested collapsible list
- Connector lines via CSS `::before` pseudo-elements (per `design/tree_view/code.html`)

**Key behaviors for Table:**
- Calls `getTableView(sort, filter, entityType)` on mount
- Sort indicators on clickable column headers
- Rows clickable → navigate to entity detail

**Key behaviors for Timeline:**
- Calls `getTimelineView(entityType)` on mount
- Vertical line via CSS (per `design/timeline_view/code.html`)
- Grouping control (day/week/month/year) as buttons or dropdown
- Cards show title, type badge, date

**Verification:**
- `npm run check` passes
- Tree: entity types appear as root nodes, expand/collapse works, filtering updates tree
- Table: columns sort correctly by clicking headers, search filter narrows rows, clicking row opens detail
- Timeline: entities ordered by date, grouping control changes granularity, filter works

**Exit criteria:** All three views render data from backend via their IPC commands, interactive behaviors (sort, filter, expand, group) work correctly.

---

### D7: Dashboard Refinement and Polish

**Purpose:** Refine the Dashboard to match `design/knowledge_os_dashboard/` bento-grid mockup, add window state persistence, keyboard shortcuts, error handling, and status bar.

**Files:**

| File                                 | Action  | Description                                                                                                                                                                                                                               |
| ------------------------------------ | ------- | ----------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| `desktop/src/views/Dashboard.svelte` | Rewrite | Bento-grid layout with: welcome message, stats cards (Total Entities, Indexed Pages, Relationships), Recent Activity feed (last 10 events), Quick Actions cards (Import, New Entry, Explore Graph), System Health card |
| `desktop/src/views/StatusBar.svelte` | Create  | Fixed bottom footer bar: system status indicator (green dot + "Operational"), entity count, database path                                                                                                                                |
| `desktop/src/App.svelte`             | Modify  | Integrate StatusBar, add keyboard shortcut handler (Ctrl+K → focus search, Escape → close detail)                                                                                                                                         |
| `desktop/src-tauri/tauri.conf.json`  | Modify  | Update window config: default size 1280x800, min size 900x600, title "Knowledge OS"                                                                                                                                                       |

**Keyboard shortcuts (from PRD-0006 NF2.3 and `design/knowledge_os_dashboard/code.html`):**
- `Ctrl+K` or `Cmd+K` — focus global search bar
- `Escape` — close detail panel / deselect node
- `Tab` / `Shift+Tab` — navigate between interactive elements

**Window state persistence (PRD-0006 NF2.5):**
- Tauri's window state plugin or manual `localStorage` for window size/position
- Restore window geometry on next launch

**Error handling:**
- All IPC commands return `Result<T, String>` — frontend shows error messages in a toast or inline banner
- Network-level errors (Tauri IPC failure) caught in `api.ts` wrapper and surfaced as status messages
- Loading spinners for all async operations

**Verification:**
- `npm run check` passes
- `cargo tauri build` produces valid binary (manual verification)
- Dashboard shows entity count matching backend data
- Status bar shows correct entity count and "Operational" indicator
- Ctrl+K focuses search bar in all views
- Window size/position remembered between sessions
- Import errors display correctly in Import view (per F2.6)

**Exit criteria:** Dashboard matches design mockup, status bar renders, keyboard shortcuts work, window state persists, error messages surface correctly.

---

## Verification Strategy

| Level             | Command                                            | Coverage                                                                          |
| ----------------- | -------------------------------------------------- | --------------------------------------------------------------------------------- |
| Rust unit         | `cargo test -p knowledge-desktop`                  | StoreWrapper delegation, command parameter mapping                                |
| Rust integration  | `cargo test -p knowledge-storage`                  | 81 existing tests, no regressions                                                 |
| Rust integration  | `cargo test -p knowledge-derivation`               | View adapter tests (22 unit + 5 integration)                                      |
| BDD               | `cargo test --test cucumber -p knowledge-cli`      | 68 BDD scenarios, CLI commands still work                                         |
| Svelte type-check | `npm run check`                                    | No TypeScript / Svelte errors                                                     |
| Lint              | `cargo clippy -- -D warnings && cargo fmt --check` | Code quality                                                                      |
| Manual (MVP)      | See PRD-0006 §Test Cases                           | 8 test cases for cold start, import, search, graph, views, detail, file-open, cross-database |

---

## Exit Criteria

- [ ] All 11 IPC commands implemented and tested (list, import, search, detail, source, open_in_app, open_folder, graph, tree, table, timeline)
- [ ] All view components render and interact correctly (Dashboard, Import, Browser, Detail, Graph, Tree, Table, Timeline, Search)
- [ ] Entity detail panel shows source file path and provides "Open File" / "Show in Folder" actions
- [ ] Design system tokens match `design/knowledge_os/DESIGN.md`
- [ ] All existing Rust tests pass (81 storage, 27 derivation, 68 BDD)
- [ ] `npm run check` passes with no type errors
- [ ] `cargo clippy` passes with no warnings
- [ ] `cargo tauri build` produces valid binary on target platform
- [ ] PRD-0006 acceptance criteria met: launch, import, search, detail, graph, views, file opening, CLI compatibility

---

## Impact Analysis

### Structural Changes and Consumers

| Change                             | Direct Consumers                | Transitive Consumers                     |
| ---------------------------------- | ------------------------------- | ---------------------------------------- |
| `commands.rs` (new)                | `lib.rs` (registers handlers)   | Frontend `api.ts` (invoke calls)         |
| `file.rs` (new)                    | `commands/mod.rs`               | `tauri-plugin-shell` (open in app/folder)|
| `StoreWrapper` in commands         | All 11 command functions        | N/A (encapsulated in commands module)    |
| `app.css` rewrite                  | All Svelte components           | All view styles                          |
| `Graph.svelte` (new)               | `App.svelte` (route mapping)    | `graph-layout.ts` (D3-force worker)      |
| `Detail.svelte` (new)              | `App.svelte` + `Browser.svelte` | `state.svelte.ts` (selectedEntityDetail) |
| `Import.svelte` (new)              | `App.svelte`                    | `api.ts` importFiles call                |
| `capabilities/default.json` modify | Tauri security config           | Window permissions for dialog + shell    |

### Risk Surface

1. **StoreWrapper boilerplate:** 7 port traits × ~5 methods each = ~35 delegation methods. Risk of copy-paste errors. **Mitigation:** Follow existing CLI `StoreWrapper` pattern exactly; test with a single entity listing command first.

2. **ViewOutput serialization:** `ViewOutput` enum contains full `Entity` structs from `knowledge-core`. Serializing these to JSON for Tauri IPC may include fields the frontend doesn't need. **Mitigation:** Define view-specific response types (already planned in `commands.rs`) that map only required fields.

3. **Graph performance at >500 nodes:** D3-force on main thread causes UI jank. **Mitigation:** Deferred to Web Worker for larger graphs (using comlink). The initial implementation runs on main thread with `requestAnimationFrame` throttling.

4. **File path resolution:** Source file paths stored in Provenance/BinaryContent components may refer to files that have been moved or deleted since import. **Mitigation:** `get_entity_source` returns the stored path as-is; frontend handles file-not-found errors gracefully from `open_in_default_app`.

5. **Cross-platform file opening:** Opening files and revealing in folder behaves differently per OS. **Mitigation:** Use `tauri-plugin-shell` which provides consistent `open` and `reveal` APIs across platforms.

6. **Cross-database compatibility:** Desktop app and CLI sharing `SqliteStore` requires identical schema. **Mitigation:** D1 tests verify `cargo test -p knowledge-storage` passes unchanged.

---

## Implementation Notes

*(Filled in during/after implementation — records deviations, discoveries, decisions made during coding)*
