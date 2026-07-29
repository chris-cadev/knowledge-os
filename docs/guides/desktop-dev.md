# Desktop App Guide

> How to set up, run, and develop the Knowledge OS desktop application.

---

## Prerequisites

- **Rust stable** -- installed via `mise install` or `rustup`
- **Node.js 26** -- installed via `mise install`
- **Tauri system dependencies** -- on Linux: `webkit2gtk-4.1`, `libgtk-3`, `libayatana-appindicator3`, and others. See [Tauri's prerequisites guide](https://v2.tauri.app/start/prerequisites/).
- **npm dependencies** -- run `npm install` in `desktop/`

```bash
mise install          # install Rust + Node.js managed by mise
cd desktop && npm install
```

---

## Quick Start

### Run in Development Mode

From the repository root:

```bash
mise run desktop
# or directly:
cargo tauri dev
```

This starts the Vite dev server (default `http://localhost:5173`) and launches the Tauri application window with hot-reload for both the Svelte frontend and Rust backend.

### Frontend-Only Dev (no Tauri window)

```bash
cd desktop && npm run dev
```

Useful for iterating on UI without the Rust backend. IPC calls to Tauri commands will fail, but layout, styling, and routing can be developed in isolation.

### Build the Rust Backend Only

```bash
cargo build -p knowledge-desktop
# or
mise run build-desktop
```

Output: `target/debug/knowledge-desktop`

### Build for Production

```bash
cd desktop && npm run build && cargo build -p knowledge-desktop --release
```

The Vite build outputs to `desktop/dist/`. The release binary is at `target/release/knowledge-desktop`. For a distributable bundle, use `cargo tauri build` (produces `.deb`, `.AppImage`, etc. on Linux).

---

## Project Structure

```
desktop/
├── src/                          # Svelte 5 Frontend
│   ├── main.ts                   # Entry point
│   ├── app.css                   # Global design system (light/dark)
│   ├── lib/
│   │   ├── types.ts              # TypeScript interfaces
│   │   ├── api.ts                # Tauri IPC wrappers
│   │   ├── state.svelte.ts       # Global reactive state
│   │   ├── router.svelte.ts      # Hash-based router
│   │   ├── theme.svelte.ts       # OS color scheme detection
│   │   ├── shortcuts.svelte.ts   # Keyboard shortcuts
│   │   ├── drag-drop.ts          # File drag-drop handling
│   │   └── graph-layout.ts       # D3 force simulation
│   └── views/                    # UI view components
│       ├── App.svelte            # Root shell layout
│       ├── Sidebar.svelte        # Navigation sidebar (8 views)
│       ├── StatusBar.svelte      # Bottom status bar
│       ├── Dashboard.svelte      # Home: stats, quick actions
│       ├── Browser.svelte        # Entity list browser
│       ├── Detail.svelte         # Entity detail view
│       ├── Graph.svelte          # Force-directed graph
│       ├── Tree.svelte           # Hierarchical tree
│       ├── Table.svelte          # Sortable table
│       ├── Timeline.svelte       # Chronological timeline
│       ├── Import.svelte         # File import UI
│       └── Search.svelte         # Full-text search
│
├── src-tauri/                    # Rust Tauri Backend
│   ├── Cargo.toml                # Crate: knowledge-desktop
│   ├── tauri.conf.json           # Window, build, bundle config
│   └── src/
│       ├── main.rs               # Entry point
│       ├── lib.rs                # Tauri builder, plugin setup, commands
│       └── commands/             # IPC command handlers
│           ├── mod.rs
│           ├── store.rs          # AppState + StoreWrapper
│           ├── response.rs       # Serializable response types
│           ├── entity.rs         # list_entities, get_entity_detail
│           ├── search.rs         # search_entities
│           ├── import.rs         # import_files
│           ├── file.rs           # open_in_default_app, open_source_folder
│           └── view.rs           # View adapters (graph, tree, table, timeline)
│
├── index.html                    # Vite entry HTML
├── vite.config.ts                # Vite configuration
├── svelte.config.js              # Svelte preprocessor config
├── tsconfig.json                 # TypeScript config
└── package.json                  # Node dependencies
```

---

## Architecture

### Two-Process Model

The desktop app uses **Tauri 2**, which separates the UI (webview) from the backend (Rust):

```
Svelte Frontend (webview)
    ── invoke("command", args) ──> Tauri IPC
                                         │
    <── serialized JSON response ───
                                         │
                              Rust Backend (Tauri core)
                                  │
                        StoreWrapper ──> SqliteStore
                        knowledge-import ──> MarkdownImporter, PdfImporter
                        knowledge-derivation ──> GraphViewAdapter, TreeViewAdapter, TableViewAdapter, TimelineViewAdapter
```

### Backend Dependencies

The Rust backend consumes the same Knowledge OS workspace crates as the CLI:

| Crate             | Role                                                      |
| ----------------- | --------------------------------------------------------- |
| `knowledge-core`  | Entity types, component types, port traits                |
| `knowledge-storage` | `SqliteStore` adapter (creates `knowledge.db`)          |
| `knowledge-import`  | `MarkdownImporter`, `PdfImporter` for file ingestion    |
| `knowledge-derivation` | View computation adapters (graph, tree, table, timeline) |
| `knowledge-plugin` | Plugin trait and loader                                  |

The desktop is a **first-class client** alongside the CLI -- it talks directly to the storage layer via Rust traits, not through the API server.

### IPC Commands

Eleven Tauri commands are registered in `lib.rs`:

| Command                 | Purpose                          |
| ----------------------- | -------------------------------- |
| `list_entities`         | List all entities with filters   |
| `get_entity_detail`     | Full entity with components/rels |
| `get_entity_source`     | Raw source file content          |
| `open_in_default_app`   | Open source in OS default app    |
| `open_source_folder`    | Reveal source in file manager    |
| `import_files`          | Import .md and .pdf files        |
| `search_entities`       | Full-text search                 |
| `get_graph_view`        | Force-directed graph data        |
| `get_tree_view`         | Hierarchical tree data           |
| `get_table_view`        | Sortable table data              |
| `get_timeline_view`     | Chronological timeline data      |

### Data Flow

1. User interacts with a Svelte view (e.g., clicks "Import")
2. View calls a function in `lib/api.ts` that invokes the Tauri command
3. Tauri serializes the arguments and calls the Rust handler
4. Handler uses `StoreWrapper` (wrapping `Arc<SqliteStore>`) or a derivation adapter
5. Response is serialized via `response.rs` types and sent back to the frontend

### Database

The app creates `knowledge.db` in its working directory on startup, managed by `SqliteStore::new("knowledge.db")`. This is the same SQLite schema used by the CLI -- data can be shared if both point to the same database file.

---

## Frontend Development

### Views

The app has 8 views navigated via hash-based routing (`#/browse`, `#/entity/<uuid>`, etc.):

| Route               | Component          | Description                              |
| ------------------- | ------------------ | ---------------------------------------- |
| `#/`                | Dashboard.svelte   | Summary stats, recent entities, actions  |
| `#/browse`          | Browser.svelte     | Filterable entity list                   |
| `#/entity/:id`      | Detail.svelte      | Entity detail with all data              |
| `#/graph`           | Graph.svelte       | D3 force-directed graph                  |
| `#/tree`            | Tree.svelte        | Hierarchical tree by type                |
| `#/table`           | Table.svelte       | Sortable table                           |
| `#/timeline`        | Timeline.svelte    | Chronological grouping                   |
| `#/search`          | Search.svelte      | Debounced full-text search               |
| `#/import`          | Import.svelte      | Drag-drop and file picker import         |

### State Management

Uses Svelte 5 runes (`$state`, `$derived`, `$effect`) in module-level stores managed by `state.svelte.ts`. Import `getState()` to access reactive state from any component.

### Theming

CSS custom properties in `app.css` define light and dark themes. The `theme.svelte.ts` module detects the OS color scheme preference. All colors, spacing, and typography use these variables.

### API Layer

All Tauri IPC calls are typed functions in `lib/api.ts`. Example:

```typescript
import { invoke } from "@tauri-apps/api/core";
import type { EntitySummary } from "./types";

export async function listEntities(): Promise<EntitySummary[]> {
  return invoke("list_entities");
}
```

### Keyboard Shortcuts

Defined in `lib/shortcuts.svelte.ts`:
- `Ctrl+1` through `Ctrl+8` -- Switch views
- `Ctrl+N` -- Navigate to Import view
- `Ctrl+F` -- Focus search

---

## Backend Development

### Adding a New Command

1. Create a handler function in `desktop/src-tauri/src/commands/` (or add to an existing file)
2. Register the function with `tauri::command` attribute:
   ```rust
   #[tauri::command]
   pub async fn my_command(state: State<'_, AppState>) -> Result<MyResponse, String> {
       // Implementation
   }
   ```
3. Add a corresponding function in `commands/mod.rs` and re-export it
4. Register the command in `lib.rs` via `.invoke_handler(tauri::generate_handler![..., my_command])`
5. Add a typed wrapper in `desktop/src/lib/api.ts`

### Accessing the Store

All command handlers receive `State<'_, AppState>`, which holds `Arc<SqliteStore>`. Use `store.store()` to get a `StoreWrapper` that implements all core port traits:

```rust
let store = state.store();
let entities = store.list_entities().await.map_err(|e| e.to_string())?;
```

### Using Derivation Adapters

View commands instantiate adapters from `knowledge-derivation` and pass the `StoreWrapper`:

```rust
let adapter = GraphViewAdapter::new(state.store());
let view = adapter.compute(&input).await.map_err(|e| e.to_string())?;
```

---

## Common Workflows

### Import Files

Open the Import view (`#/import`) to drag-and-drop `.md` and `.pdf` files, or use the file/directory picker buttons.

### Browse Entities

The Browse view (`#/browse`) lists all imported entities. Click any entity to open its Detail view showing components, relationships, events, and version history.

### Explore the Graph

The Graph view (`#/graph`) renders entities as a force-directed graph. Pan and zoom with mouse. Click a node to inspect it, or drag nodes to rearrange.

### Search

Press `Ctrl+F` or navigate to the Search view (`#/search`). Type a query -- results appear with debounced full-text search. Filter by entity type or tag.

---

## Further Reading

- [Tauri 2 Documentation](https://v2.tauri.app/) -- Framework reference
- [Svelte 5 Documentation](https://svelte.dev/docs/svelte/overview) -- Frontend framework
- [Architecture Overview](../architecture/overview.md) -- System architecture
- [Plugin Development](plugin-development.md) -- Extending Knowledge OS
