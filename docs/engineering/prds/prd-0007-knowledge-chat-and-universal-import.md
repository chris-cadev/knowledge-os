# PRD-0007: Knowledge Chat and Universal Import

**Status:** Draft
**Date:** 2026-07-29
**Author:** Core maintainers
**Priority:** P0 — Experience and Intelligence Layer
**Depends on:** PRD-0001, PRD-0002, PRD-0003, PRD-0006

---

## Purpose

Deliver two integrated capabilities that make Knowledge OS usable by non-technical knowledge workers: (1) a universal import system that accepts documents from any source (office files, databases, URLs, clipboard), and (2) a chat interface that lets users converse with their knowledge graph through natural language, entity references (`@`), and commands (`/`). Together these features create the "try at home, bring to work" value proposition.

---

## Problem Statement

Knowledge OS currently has a powerful engine with a CLI interface and a skeleton desktop app. Two gaps prevent adoption by office users:

**Import friction.** The desktop app imports only `.md` and `.pdf` files. Office users work across an entire ecosystem of formats: Microsoft Office (Word, Excel, PowerPoint, Outlook emails), OpenDocument (ODT, ODS, ODP), Apple iWork (Pages, Numbers, Keynote), email exports (EML, MSG, MBOX), structured data (CSV, JSON, XML, YAML), plain text (TXT, RTF), note-taking app exports (ENEX, OPML, Notion JSON), and contact/calendar data (VCF, ICS). They need drag-and-drop, URL import, clipboard paste, and directory watching — all working for the formats they actually use.

**No conversational interface.** The system holds structured knowledge but offers no way to interact with it conversationally. The product vision describes "Conversation" as a view type and "AI context" as a derived artifact. Without a chat interface, users cannot ask questions about their knowledge, reference specific entities, or iterate through dialogue. The `@`-reference pattern that coding assistants (opencode, claude code, codex) popularized must be adapted to knowledge entities for non-technical users through a sleek UI with proper HCI affordances.

---

## Scope

### In Scope

- **Universal import** — Support for all common office and document formats, organized into format families:
  - **Microsoft Office:** `.doc` / `.docx` (Word), `.xls` / `.xlsx` / `.xlsm` (Excel), `.ppt` / `.pps` / `.pptx` (PowerPoint), `.msg` / `.eml` (Outlook email)
  - **OpenDocument:** `.odt` / `.ott` (Writer), `.ods` / `.ots` (Calc), `.odp` / `.otp` (Impress), `.odg` (Draw)
  - **Apple iWork:** `.pages` (Pages), `.numbers` (Numbers), `.key` (Keynote)
  - **Email & communication:** `.eml`, `.msg`, `.mbox` (mailbox export), `.pst` / `.ost` (Outlook data file — metadata only), `.ics` (calendar), `.vcf` (contacts)
  - **Structured data:** `.csv`, `.json`, `.xml`, `.yaml` / `.yml`
  - **Documents:** `.rtf`, `.txt`, `.html` / `.htm`, `.md` (existing), `.pdf` (existing)
  - **Note-taking exports:** `.enex` (Evernote), `.opml` (outliner), Notion JSON export, Obsidian vault (bulk Markdown directory)
  - **Databases:** SQLite, PostgreSQL, MySQL via connection string
  - **Sources:** URL fetch, clipboard paste, directory watching with auto-import
- **Import UX redesign** — Tabbed import view (Files / URL / Clipboard / Database), per-file progress, import preview (show what will be created before confirming), format-specific column mapping (CSV, database), conflict detection for re-imported files, collapsible errors, undo last import.
- **Post-import onboarding** — After import, suggest next actions: "Try asking about this in Chat", "Explore related entities in Graph view".
- **Chat port trait** — New `ChatCompletion` trait in `knowledge-core` with `chat()` and `chat_stream()` methods.
- **Chat pipeline** — RAG pipeline in `knowledge-derivation` that retrieves relevant entities, builds structured context, and calls a chat provider.
- **Chat adapters** — OpenAI (GPT-4o), OpenAI-compatible (LM Studio, vLLM, llama.cpp, any endpoint), Ollama (local models), Mock (testing).
- **Chat view in desktop app** — Full conversation UI with streaming responses, message history, conversation sidebar.
- **Inline citation system** — AI responses include numbered citations `[1]`, `[2]` linked to entity sources. Hover shows entity preview. Click navigates to entity detail.
- **Source type toggles** — Toggle buttons in chat composer: "Search knowledge graph" (on by default), "Search web" (optional integration), "LLM knowledge only" — matching Glean's source toggle pattern.
- **@-mention entity referencing** — Dual-mode autocomplete (quick `@` + type to filter, and a visual entity browser panel). Selected entities become pinned context chips. Autocomplete boosts recently viewed and frequently referenced entities.
- **Intermediate processing states** — Chat shows live status: "Searching entities...", "Reading 3 entities...", "Generating response..." — matching Claude.ai and Glean's tool-use visibility.
- **Feedback mechanism** — Thumbs up/down on AI responses with reason selection ("Used incorrect entity", "Missing information", "Other"). Feedback stored as derived data for quality improvement.
- **Context limit handling** — When approaching LLM context window, oldest messages are truncated with a "Continue conversation" button (matching Claude.ai's pattern).
- **Adaptive response mode** — Toggle between "Fast" (optimized for speed, fewer context retrievals) and "Thinking" (deeper retrieval, full context, stronger reasoning) — matching Glean's Fast/Thinking mode.
- **OCR for images and scanned documents** — Text extraction from embedded images in office files, scanned PDFs, and standalone image files via a pluggable OCR backend. Supports Tesseract (local, fast, CPU-only), Ollama multimodal models (DeepSeek-OCR, Qwen3-VL), API-based (GPT-4o vision via OpenAI or any OpenAI-compatible vision endpoint like LM Studio), and Mock. OCR is a derived artifact — image blobs are canonical, text output is regenerable.
- **`/` slash commands** — `/import`, `/search`, `/graph`, `/help`, `/clear`, `/export` commands in the chat input.
- **Conversation persistence** — Conversations and messages stored as canonical entities in the knowledge graph.
- **HCI design principles** — Progressive disclosure, forgiveness (undo), consistency, feedback, affordances, accessibility.
- **MCP server path** — Architecture must support exposing chat and entity retrieval via Model Context Protocol in a future phase. Chat completion trait and entity retrieval pipeline are designed to be MCP-adaptable.

### Out of Scope

- **Real-time collaboration** — Multi-user chat (deferred to Year 3, see product-vision.md)
- **Voice input** — Speech-to-text for chat (no current requirement)
- **Automated knowledge gap detection** — AI suggesting what's missing (Year 2)
- **Plugin-based chat providers** — Chat adapters are built-in at launch; plugin API deferred
- **Export formats** — Exporting conversations as documents (future enhancement)
- **Image/video import** — Standalone image files (.png, .jpg, .gif) can be imported. OCR is derived text. No video processing.
- **Office binary content preservation** — Embedded images, charts, and media within office files are OCR'd for text but the original binary blobs are not preserved as entities. Binary media preservation is a future enhancement.
- **MCP server implementation** — Architectural compatibility only; MCP server is a follow-up PRD

---

## Engineering Questions

### 1. Which canonical entities are introduced?

| Entity Type    | Description                              | Example                                 |
| -------------- | ---------------------------------------- | --------------------------------------- |
| `Conversation` | A series of messages between user and AI | "Q3 research review", "Onboarding chat" |
| `Message`      | A single turn in a conversation          | A user question or an AI response       |

Both types follow the existing entity rules: exactly one type, type never changes, extensible through configuration.

### 2. Which relationships are introduced?

| Relationship Type | Source       | Target       | Description                               |
| ----------------- | ------------ | ------------ | ----------------------------------------- |
| `has_message`     | Conversation | Message      | A conversation contains this message      |
| `part_of`         | Message      | Conversation | This message belongs to the conversation  |
| `references`      | Message      | Any entity   | The message references a knowledge entity |
| `referenced_by`   | Any entity   | Message      | The entity is referenced by this message  |

These relationship types are additive. They extend the existing relationship catalog without modifying existing types.

### 3. Which components are introduced?

| Component Type   | Payload                                                     | Description                                          |
| ---------------- | ----------------------------------------------------------- | ---------------------------------------------------- |
| `MessageContent` | `{ role: "user" \| "assistant" \| "system", text: string }` | The text content of a message                        |
| `EntityRefs`     | `{ entity_ids: Uuid[] }`                                    | Entity references pinned as context for this message |

### 4. Which events are emitted?

| Event                  | Trigger                      | Description                                     |
| ---------------------- | ---------------------------- | ----------------------------------------------- |
| `ConversationCreated`  | New conversation started     | A conversation entity is created                |
| `MessageCreated`       | New message in conversation  | A user or AI message is persisted               |
| `EntityReferenced`     | Entity referenced in message | An entity was mentioned via `@`                 |
| `ChatContextRetrieved` | Context retrieved for LLM    | Entities were fetched for RAG context (derived) |

The `ConversationCreated`, `MessageCreated`, and `EntityReferenced` events follow the existing canonical event structure. `ChatContextRetrieved` is a derivation event (derived data).

### 5. Which derived representations are generated?

- **Chat context payload** — Assembled from referenced entities' components and relationships, structured into a system prompt for the LLM.
- **Search results** — Used to find entities for `@`-mention autocomplete (reuses existing `SearchIndex`).
- **Semantic retrieval results** — Used to find context entities when no explicit `@` references are provided (reuses existing `VectorStore`).

All derived artifacts are regenerable from canonical data (entities, components, relationships).

### 6. Which layer owns the feature?

| Feature               | Layer | Rationale                                          |
| --------------------- | ----- | -------------------------------------------------- |
| Universal import      | 1     | Import layer — receives external data              |
| Chat completion trait | 4     | Knowledge model — defines the capability interface |
| Chat pipeline (RAG)   | 6     | Derivation layer — assembles AI context            |
| Chat view + UI        | 7     | Presentation layer — renders the conversation      |
| Conversation storage  | 4     | Knowledge model — persists conversations           |
| Commands (`/`)        | 7     | Presentation layer — CLI-like affordance in UI     |

### 7. Can every derived artifact be regenerated?

Yes:
- Chat context payloads are assembled on-demand from canonical entities. No caching required.
- Search results are regenerated from the search index, which is derived from canonical entities.
- Semantic retrieval results are regenerated from embeddings, which are derived from canonical entities' Content components.

### 8. Does the feature violate storage independence?

No. All storage goes through existing port traits (`EntityRepository`, `ComponentRepository`, `RelationshipRepository`, `SearchIndex`, `VectorStore`). The `ChatCompletion` trait is a new port that is implementation-agnostic. No storage engine is assumed.

### 9. Does the feature introduce implementation leakage?

No. The chat adapters (OpenAI, Ollama, Mock) are behind the `ChatCompletion` port trait. No AI provider details leak into the domain model. The importers are behind the `ImportAdapter` port. All storage, AI, and import details are adapter-isolated.

### 10. Does the feature preserve the canonical model?

Yes. Conversations and messages are canonical entities with typed components and relationships. They follow the same invariants as all other entities: unique IDs, versioned, auditable, never hard-deleted. The chat is built *on top* of the canonical model, never bypassing it.

---

## Pipeline Spine Analysis

```mermaid
flowchart TD
    subgraph L1["Layer 1 — Import Layer"]
        direction LR
        DOCX[".docx / .doc"]
        XLSX[".xlsx / .xls"]
        PPTX[".pptx / .ppt"]
        CSV[".csv"]
        ODT[".odt / .ods"]
        PAGES[".pages / .numbers"]
        EML[".eml / .msg / .mbox"]
        HTML[".html / .htm"]
        DB[("SQLite / PG / MySQL")]
        URL["URL fetch"]
        CLIP["Clipboard"]
        ENEX[".enex / .opml"]
    end

    L1 --> L2

    subgraph L2["Layer 2 — Parsing Layer"]
        TXT["Extract text, metadata, structure\nConvert tables to structured data\nParse headers, formatting, links"]
    end

    L2 --> L3

    subgraph L3["Layer 3 — Normalization Layer"]
        NORM["Entity identification\nFormat normalization\nCross-reference resolution\nCanonical ID assignment"]
    end

    L3 --> L4

    subgraph L4["Layer 4 — Knowledge Model\n(canonical)"]
        direction LR
        ENT["Entities\n(Conversation, Message,\n+ all existing types)"]
        CMP["Components\n(MessageContent,\nEntityRefs, + existing)"]
        REL["Relationships\n(has_message, references,\npart_of, + existing)"]
    end

    L4 --> L5

    subgraph L5["Layer 5 — Relationship Engine"]
        RE["Typed, directed, versioned relationships\nhas_message · references · part_of · referenced_by"]
    end

    L5 --> L6

    subgraph L6["Layer 6 — Derivation Layer"]
        direction LR
        subgraph CP["Chat Pipeline (new)"]
            C1["1. Receive query + @refs"]
            C2["2. Retrieve entity context"]
            C3["3. Build system prompt\nwith citations"]
            C4["4. Call ChatCompletion\n(streaming)"]
            C5["5. Persist conversation"]
        end
        SR["Search Index (existing)"]
        VR["Vector Store / Embeddings (existing)"]
    end

    L6 --> L7

    subgraph L7["Layer 7 — Presentation Layer"]
        direction LR
        CV["Chat view\n@-mentions · / commands\ncitations · source toggles\nstreaming · feedback"]
        IV["Import view\nTabbed: Files / URL / Clipboard / DB\nProgress · Preview · Undo"]
        EV["Existing views\nDashboard · Browse · Detail\nGraph · Tree · Table · Timeline"]
    end
```

The chat pipeline introduces a new derivation path at Layer 6. Import expands Layer 1 with new adapters covering the full office ecosystem: Microsoft Office, OpenDocument, Apple iWork, email/communication formats, structured data, plain text, note-taking exports, and databases.

---

## Functional Requirements

### F1: Universal Import

#### F1.A — Microsoft Office

| ID     | Requirement                                                                    | Priority | Acceptance Criteria                                                                                                                             |
| ------ | ------------------------------------------------------------------------------ | -------- | ----------------------------------------------------------------------------------------------------------------------------------------------- |
| F1.A.1 | Import `.docx` / `.doc` — extract text, headings, metadata                     | P0       | Entity created with Content (full text), Title, Timeline, Author components                                                                     |
| F1.A.2 | Import `.xlsx` / `.xls` / `.xlsm` — extract rows as entities                   | P0       | Each row becomes an entity with column data mapped to components. Sheet selection UI.                                                           |
| F1.A.3 | Import `.pptx` / `.ppt` / `.pps` — extract text from slides, notes             | P1       | Each slide becomes an entity. Slide content, speaker notes, and slide titles extracted.                                                         |
| F1.A.4 | Import `.msg` (Outlook message) — extract sender, recipient, body, attachments | P1       | Email entity created with From, To, Subject, Body, Date components. Attachments listed.                                                         |
| F1.A.5 | Import `.pst` / `.ost` (Outlook data file) — enumerate folders, extract emails | P2       | Browse PST folder structure. Select folders to import. Each email becomes a Message entity. Metadata only if full MSG extraction not available. |

#### F1.B — OpenDocument Format

| ID     | Requirement                                               | Priority | Acceptance Criteria                                                                        |
| ------ | --------------------------------------------------------- | -------- | ------------------------------------------------------------------------------------------ |
| F1.B.1 | Import `.odt` / `.ott` — extract text, headings, metadata | P1       | Entity created with Content, Title, Author components. Headings preserved as structure.    |
| F1.B.2 | Import `.ods` / `.ots` — extract rows as entities         | P1       | Same column mapping UI as `.xlsx`. Each row becomes entity with structured components.     |
| F1.B.3 | Import `.odp` / `.otp` — extract text from slides, notes  | P2       | Each slide becomes an entity. Content and slide notes extracted.                           |
| F1.B.4 | Import `.odg` — extract text labels from vector drawings  | P2       | Text content extracted from `<text>` elements in the ODG XML. Position metadata preserved. |

#### F1.C — Apple iWork

| ID     | Requirement                                               | Priority | Acceptance Criteria                                                                          |
| ------ | --------------------------------------------------------- | -------- | -------------------------------------------------------------------------------------------- |
| F1.C.1 | Import `.pages` — extract text, headings, metadata        | P1       | Entity created with Content, Title, Author. Handles both old (bundle) and new (zip) formats. |
| F1.C.2 | Import `.numbers` — extract rows as entities              | P2       | Same column mapping as `.xlsx` / `.ods`. Sheet selection. Each row = entity.                 |
| F1.C.3 | Import `.key` — extract text from slides, presenter notes | P2       | Each slide becomes an entity. Content extracted from slide XML within the .key bundle.       |

#### F1.D — Email and Communication

| ID     | Requirement                                                         | Priority | Acceptance Criteria                                                                                                       |
| ------ | ------------------------------------------------------------------- | -------- | ------------------------------------------------------------------------------------------------------------------------- |
| F1.D.1 | Import `.eml` (standard email) — extract headers, body, attachments | P0       | Email entity with From, To, Subject, Date, Body (HTML/text parsed to Markdown). Attachments listed as BinaryContent refs. |
| F1.D.2 | Import `.mbox` (mailbox archive) — enumerate and extract messages   | P1       | Multi-message .mbox file parsed. Each message becomes an entity. Progress shown per-message.                              |
| F1.D.3 | Import `.ics` (calendar export) — extract events                    | P2       | Each event becomes an entity with Title, Timeline (date/time), Location, Description.                                     |
| F1.D.4 | Import `.vcf` (contacts export) — extract people                    | P2       | Each contact becomes a Person entity with name, email, phone, organization components.                                    |

#### F1.E — Structured Data

| ID     | Requirement                                                              | Priority | Acceptance Criteria                                                                                                 |
| ------ | ------------------------------------------------------------------------ | -------- | ------------------------------------------------------------------------------------------------------------------- |
| F1.E.1 | Import `.csv` — parse rows, show column mapping UI before importing      | P0       | User selects which CSV columns map to Title, Tags, Content, Type. Preview before import. Each row = entity.         |
| F1.E.2 | Import `.json` — parse JSON arrays as entities, objects as single entity | P1       | JSON array of objects → each object becomes entity with keys mapped to components. Single JSON object → one entity. |
| F1.E.3 | Import `.xml` — parse XML document structure into entities               | P1       | XML elements become entities with child elements as nested components or relationships.                             |
| F1.E.4 | Import `.yaml` / `.yml` — parse YAML structured data                     | P1       | YAML arrays → entities. YAML mappings → component data. Same mapping logic as JSON.                                 |

#### F1.F — Documents and Plain Text

| ID     | Requirement                                                         | Priority | Acceptance Criteria                                                                                  |
| ------ | ------------------------------------------------------------------- | -------- | ---------------------------------------------------------------------------------------------------- |
| F1.F.1 | Import `.rtf` — extract formatted text, metadata                    | P1       | RTF parsed to Markdown. Entity with Content, Title. Formatting preserved where possible.             |
| F1.F.2 | Import `.txt` — import as plain text entity                         | P0       | File name becomes Title. Full content in Content component. Encoding auto-detected (UTF-8, Latin-1). |
| F1.F.3 | Import `.html` / `.htm` — extract text, metadata, links             | P0       | HTML parsed to Markdown. Title from `<title>`. Links extracted as `references` relationships.        |
| F1.F.4 | Import from URL — paste URL, fetch and import (auto-detects format) | P0       | URL field fetches content. Content-Type header determines parser (HTML, PDF, image, text).           |
| F1.F.5 | Import from clipboard — paste text or HTML                          | P0       | `Ctrl+V` in import tab or chat creates entity from clipboard content. HTML pasted as Markdown.       |

#### F1.G — Note-Taking App Exports

| ID     | Requirement                                                                    | Priority | Acceptance Criteria                                                                                                                                         |
| ------ | ------------------------------------------------------------------------------ | -------- | ----------------------------------------------------------------------------------------------------------------------------------------------------------- |
| F1.G.1 | Import `.enex` (Evernote export) — extract notes, tags, notebooks              | P1       | Evernote ENEX XML parsed. Each note becomes entity with Title, Content, Tags, Timeline. Notebooks become Collection entities with `contains` relationships. |
| F1.G.2 | Import `.opml` (outliner export) — extract outline hierarchy                   | P2       | OPML parsed. Each outline node becomes entity. Hierarchy preserved via `contains` / `part_of` relationships.                                                |
| F1.G.3 | Import Notion JSON export — extract pages, databases                           | P2       | Notion export format parsed. Pages become entities. Database rows become entities with column data.                                                         |
| F1.G.4 | Import Obsidian vault directory — bulk import `.md` files with `[[wikilinks]]` | P1       | Directory selected. All `.md` files imported. `[[wikilinks]]` resolved to `references` relationships. Tags frontmatter → Tags component.                    |

#### F1.H — Databases

| ID     | Requirement                                                             | Priority | Acceptance Criteria                                                                                                               |
| ------ | ----------------------------------------------------------------------- | -------- | --------------------------------------------------------------------------------------------------------------------------------- |
| F1.H.1 | Connect to SQLite database — import tables as entity collections        | P1       | Database file selected. Tables listed. Column mapping UI. Each row → entity.                                                      |
| F1.H.2 | Connect to PostgreSQL — connection string input, table selection        | P1       | `postgresql://user:pass@host/db` connection. Table list. Column mapping. Each row → entity.                                       |
| F1.H.3 | Connect to MySQL — connection string input, table selection             | P1       | `mysql://user:pass@host/db` connection. Same flow as PostgreSQL.                                                                  |
| F1.H.4 | Connection test — validate database connection before import            | P1       | "Test Connection" button shows success/failure with error details.                                                                |
| F1.H.5 | Column mapping UI — preview and map source columns to entity components | P1       | For all structured imports (CSV, JSON, databases): mapping step shown. User assigns Title, Tags, Content, Type, or skips columns. |

#### F1.I — OCR for Images and Scanned Documents

| ID     | Requirement                                                                                   | Priority | Acceptance Criteria                                                                                                                                                                    |
| ------ | --------------------------------------------------------------------------------------------- | -------- | -------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| F1.I.1 | Extract text from embedded images in `.docx`, `.pptx`, `.pdf` files via OCR                   | P1       | Images extracted from office files, passed through OCR backend, text merged into entity's Content                                                                                      |
| F1.I.2 | Import standalone image files (.png, .jpg, .gif, .bmp) with OCR                               | P1       | Image file imported as entity with `BinaryContent` + OCR text in `Content`. Title from filename.                                                                                       |
| F1.I.3 | Pluggable OCR backend trait — switchable between Tesseract, Ollama, and API                   | P1       | `OcrBackend` trait with `TesseractBackend`, `OllamaBackend`, `ApiBackend` implementations                                                                                              |
| F1.I.4 | Tesseract OCR backend — local CPU-based OCR via `tesseract-rs`                                | P1       | `tesseract-rs` 0.3 with `build-tesseract` feature. No system dependency needed.                                                                                                        |
| F1.I.5 | Ollama OCR backend — local LLM-based OCR via DeepSeek-OCR or Qwen3-VL                         | P1       | HTTP call to Ollama API. Configurable model name. Higher accuracy on complex layouts.                                                                                                  |
| F1.I.6 | API OCR backend — GPT-4o vision or any OpenAI-compatible vision endpoint for highest accuracy | P2       | HTTP call to `{base_url}/chat/completions` with vision model. Supports OpenAI, LM Studio, vLLM, etc. Configurable via `KOS_OPENAI_BASE_URL` (defaults to `https://api.openai.com/v1`). |
| F1.I.7 | OCR configuration in desktop app UI — dropdown to select backend                              | P2       | Settings section: "Text extraction" with Tesseract (default) / Ollama / API.                                                                                                           |
| F1.I.8 | Async OCR processing — images queued, results arrive as events                                | P1       | OCR runs in background. Status shown in import progress. Entity `Content` updated when OCR completes.                                                                                  |
| F1.I.9 | OCR is derived data — image blobs are canonical, OCR output can be regenerated                | P1       | Re-running OCR with a different backend produces new Content without losing the original image.                                                                                        |

#### F1.J — Cross-Cutting Import UX

| ID      | Requirement                                                                                                                                                      | Priority | Acceptance Criteria                                                                                                                                                         |
| ------- | ---------------------------------------------------------------------------------------------------------------------------------------------------------------- | -------- | --------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| F1.J.1  | Import preview — show entities that will be created before confirming                                                                                            | P1       | Preview panel lists entity type, title, component summary. User can cancel before write.                                                                                    |
| F1.J.2  | Conflict detection — detect re-imported files by path and content hash                                                                                           | P1       | Same file re-imported → "Already imported" shown. Options: create new version, skip, overwrite.                                                                             |
| F1.J.3  | Post-import suggested actions — contextual next steps after import                                                                                               | P1       | "150 entities from 3 files. Try asking about them in Chat" with link to Chat.                                                                                               |
| F1.J.4  | Undo last import — revert the last import operation                                                                                                              | P1       | Undo button restores state before import. Works for any format.                                                                                                             |
| F1.J.5  | Watch directory — auto-import new files in a watched folder                                                                                                      | P2       | Directory selected. New files detected within 5 seconds. Auto-imported with notification.                                                                                   |
| F1.J.6  | Per-file progress — real-time status per file during batch import                                                                                                | P0       | Each file shows icon + file name + status: Processing / Imported / Merged / Failed.                                                                                         |
| F1.J.7  | Collapsible errors — errors grouped by file, expandable detail                                                                                                   | P1       | Error count shown. Expand to see per-file error messages with retry buttons.                                                                                                |
| F1.J.8  | Format auto-detection — detect format by file extension and magic bytes                                                                                          | P0       | Files without recognized extension are inspected by magic bytes. Unknown formats warned.                                                                                    |
| F1.J.9  | Recursive toggle for directory imports — when importing a directory, a toggle controls whether to recurse into subdirectories or import only the top-level files | P0       | Toggle appears when a directory is selected. Label: "Include subdirectories" with ON/OFF. Default: ON. Affects only directory imports (single file imports are unaffected). |
| F1.J.10 | Recursive depth preview — when a directory is selected with recursive ON, show the total file count and depth                                                    | P1       | After selecting a directory, shows: "42 files found (3 subdirectories)" when recursive ON. "12 files found (top-level only)" when recursive OFF.                            |

### F2: Chat View

| ID    | Requirement                                                                                                | Priority | Acceptance Criteria                                                                                                                                                                           |
| ----- | ---------------------------------------------------------------------------------------------------------- | -------- | --------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| F2.1  | Chat view appears as a navigation tab in the sidebar                                                       | P0       | Sidebar shows "Chat" with `chat` icon; clicking it opens the chat view                                                                                                                        |
| F2.2  | Chat view shows conversation history in a sidebar panel                                                    | P0       | Left panel lists conversations by title with last message preview                                                                                                                             |
| F2.3  | User can start a new conversation                                                                          | P0       | "New Chat" button creates empty conversation, scrolls to input                                                                                                                                |
| F2.4  | User can rename a conversation                                                                             | P1       | Double-click title in sidebar to rename; persists to Conversation entity                                                                                                                      |
| F2.5  | User can delete a conversation                                                                             | P1       | Delete button archives the Conversation and all Message entities                                                                                                                              |
| F2.6  | Messages stream in real-time as the AI generates them                                                      | P0       | AI response appears token-by-token, not all at once                                                                                                                                           |
| F2.7  | Messages support Markdown rendering (code blocks, lists, headings)                                         | P0       | AI responses with Markdown are rendered, not shown as raw text                                                                                                                                |
| F2.8  | User can scroll back through conversation history                                                          | P0       | Scrollable message area with auto-scroll to bottom on new messages                                                                                                                            |
| F2.9  | Chat input supports multi-line text (Shift+Enter for newline, Enter to send)                               | P0       | Enter sends, Shift+Enter inserts newline                                                                                                                                                      |
| F2.10 | Chat input shows a character count for long messages                                                       | P2       | Counter appears after 2000 characters                                                                                                                                                         |
| F2.11 | Chat view works offline with MockChatAdapter when no AI provider is configured                             | P0       | Chat is usable even without API keys; Mock adapter returns deterministic responses                                                                                                            |
| F2.12 | **Inline citations** — AI responses include numbered citations `[1]`, `[2]` linked to entity sources       | P0       | Citation appears as superscript number after the supported statement. Hover shows entity preview card with title, type, snippet. Click navigates to entity detail.                            |
| F2.13 | **Source toggles** — Chat composer shows toggle buttons for knowledge sources                              | P0       | "Knowledge graph" toggle (default on). When off, LLM answers from general knowledge only. Clear visual indicator when a source is active.                                                     |
| F2.14 | **Intermediate processing states** — Chat shows live status during response generation                     | P0       | Status shown as animated text in the response area: "Searching knowledge graph...", "Reading 3 entities...", "Generating response..."                                                         |
| F2.15 | **Feedback mechanism** — Each AI response has thumbs up/down buttons                                       | P1       | Clicking thumbs down shows reason selection: "Used incorrect entity", "Missing information", "Wrong citation", "Other". Feedback stored for improvement.                                      |
| F2.16 | **Context limit handling** — When approaching LLM context window, oldest messages are truncated gracefully | P1       | A "Continue conversation" button appears at the truncation point. Earlier messages are summarized, not silently dropped. User is notified.                                                    |
| F2.17 | **Adaptive response mode** — Toggle between Fast and Thinking mode                                         | P2       | "Fast" mode: fewer context retrievals, faster response. "Thinking" mode: full entity context, deeper reasoning, full citations. Visual indicator of current mode.                             |
| F2.18 | **"View sources" section** — Each response has a collapsible "View sources" footer                         | P1       | Footer lists all entities referenced in the response with type badge and title. Click navigates to entity detail.                                                                             |
| F2.19 | **Stop generation** — User can interrupt the AI mid-response                                               | P0       | Stop button appears during streaming generation. Partial response is preserved.                                                                                                               |
| F2.20 | **Empty state** — First-time chat view shows suggested questions based on imported entities                | P0       | If entities exist: "Ask me about your knowledge. Try: 'What are my most recent papers?' or '@ summarize @". If no entities: "Import some documents to get started!" with link to Import view. |

### F3: @-Mention Entity Referencing

| ID    | Requirement                                                                                                              | Priority | Acceptance Criteria                                                                                                                     |
| ----- | ------------------------------------------------------------------------------------------------------------------------ | -------- | --------------------------------------------------------------------------------------------------------------------------------------- |
| F3.1  | Typing `@` in chat input triggers entity autocomplete dropdown                                                           | P0       | After `@`, a dropdown appears within 150ms showing entity search results                                                                |
| F3.2  | Autocomplete searches by entity title, type, and tags                                                                    | P0       | `@transformer` returns Concept "Transformer", Paper "Attention Is All You Need"                                                         |
| F3.3  | Autocomplete shows entity type badge, title, and subtitle (first 60 chars of Description or Content)                     | P0       | Each result row shows type badge + title + preview text                                                                                 |
| F3.4  | Autocomplete boosts recently viewed and frequently referenced entities in ranking                                        | P1       | Entities viewed within the last 24 hours appear first. Entities referenced in 3+ conversations rank higher.                             |
| F3.5  | User can navigate autocomplete with arrow keys and select with Enter or click                                            | P0       | Keyboard and mouse selection both work                                                                                                  |
| F3.6  | Selected entity appears as an inline `@EntityType:Title` pill in the message text, not as a separate chip area           | P0       | EntRef becomes a styled inline pill with entity type + title, replacing the trigger text. Matching Notion and Linear patterns.          |
| F3.7  | Clicking the `[@]` button next to the chat input opens the entity browser panel                                          | P0       | Slide-over panel shows search, type filter, recent entities, select multiple                                                            |
| F3.8  | Entity browser panel shows entity type distribution and allows multi-select                                              | P1       | User can select multiple entities to pin as context                                                                                     |
| F3.9  | Pinned entities are visible in the context strip above the input, showing count, type distribution, and an "Edit" button | P1       | Context strip shows "3 entities: 2 Papers, 1 Concept · Edit" where Edit reopens the entity browser panel                                |
| F3.10 | AI responses include `@EntityType:Title` mentions rendered as clickable links                                            | P0       | AI-generated `@` mentions in responses are rendered as styled links. Click navigates to entity detail panel.                            |
| F3.11 | `@` mention is the primary way to add context. No separate "attach file" input needed                                    | P1       | All entity referencing uses `@`. There is no separate "Attach" button or paperclip icon. Chat input shows subtle `@` hint text instead. |

### F4: `/` Slash Commands

| ID    | Requirement                                                 | Priority | Acceptance Criteria                                                                                                                                  |
| ----- | ----------------------------------------------------------- | -------- | ---------------------------------------------------------------------------------------------------------------------------------------------------- |
| F4.1  | Typing `/` in chat input shows a searchable command palette | P0       | Typing `/` opens an inline dropdown. Typing more characters filters commands by prefix match (Notion/Discord pattern).                               |
| F4.2  | Command palette shows per-command details                   | P0       | Each entry shows: icon (emoji), command name, brief description, argument types in `[brackets]`, optional vs required distinction (Discord pattern). |
| F4.3  | Command palette is keyboard-navigable                       | P0       | Arrow keys to move between commands. Enter to select. Tab moves to next argument. Escape closes palette.                                             |
| F4.4  | `/search <query>` runs a search and returns results inline  | P0       | Search results appear in chat as formatted cards with entity type badge, title, preview. Rich inline results (not a navigation event).               |
| F4.5  | `/import` opens the import view                             | P0       | Navigates to Import tab. If a URL/path argument is provided, pre-fills the appropriate field.                                                        |
| F4.6  | `/graph <entity>` opens the graph centered on entity        | P0       | Navigates to Graph tab; starts traversal from specified entity. Entity can be name or ID.                                                            |
| F4.7  | `/help [command]` shows available commands                  | P0       | Without argument: formatted list of all commands with examples. With command name: detailed help for that specific command.                          |
| F4.8  | `/clear` clears the current conversation                    | P1       | Removes all messages from current conversation (with confirmation dialog matching F6.5 forgiveness).                                                 |
| F4.9  | `/export <format>` exports conversation                     | P2       | Exports as Markdown or JSON. File save dialog opens.                                                                                                 |
| F4.10 | Commands are extensible through plugin configuration        | P2       | Plugin manifest can register new slash commands with name, description, argument spec, handler.                                                      |

### F5: MCP Server Compatibility (Architectural Path)

The chat and entity retrieval infrastructure must support future exposure through the Model Context Protocol. This PRD does not implement the MCP server but requires architectural compatibility:

| ID   | Requirement                                                                                                                                          | Priority | Acceptance Criteria                                                                                            |
| ---- | ---------------------------------------------------------------------------------------------------------------------------------------------------- | -------- | -------------------------------------------------------------------------------------------------------------- |
| F5.1 | `ChatCompletion` trait must be exposed as a reusable service, not coupled to Tauri IPC                                                               | P0       | `ChatPipeline` is a standalone struct usable from any Rust context (Tauri command, API server, MCP server)     |
| F5.2 | Entity retrieval and search must be callable from a shared service layer                                                                             | P0       | `EntityRetrievalService` aggregates entity + component + relationship loading. Reused by ChatPipeline and MCP. |
| F5.3 | Chat request/response types must be serializable (JSON) for MCP transport                                                                            | P0       | All `ChatRequest`, `ChatResponse`, `ChatDelta`, `EntityContext` types derive `Serialize`/`Deserialize`         |
| F5.4 | The MCP server (future) will expose: `search_knowledge_graph(query, filters)` and `get_entity(id)` as tools, and `kos://entities/{id}` as a resource | P2       | Architecture allows registering tools and resources without modifying core                                     |

### F6: HCI Design Principles in Chat

| ID    | Principle              | Implementation                                                                                                                                                                                                                     |
| ----- | ---------------------- | ---------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| F6.1  | Visibility             | All available actions are visible: `@` and `/` triggers show hint text in the input placeholder. Source toggles visible. Processing states visible.                                                                                |
| F6.2  | Affordance             | Input area has clear visual states: empty, typing, `@`-mode indicator, `/`-mode indicator, source toggle states, sending, receiving streaming.                                                                                     |
| F6.3  | Feedback               | Every user action has immediate feedback: character echo, autocomplete response, send animation, streaming tokens, inline citations, error messages with retry.                                                                    |
| F6.4  | Consistency            | `@`-mention and `/` command patterns follow platform conventions (similar to Slack, Discord, Notion). Inline citation format matches scholarly convention `[N]`.                                                                   |
| F6.5  | Forgiveness            | All destructive actions (delete conversation, clear messages) have confirmation dialogs. Undo for import. Stop generation preserves partial response.                                                                              |
| F6.6  | Progressive disclosure | Simple chat for basic use (type a question). Source toggles and `/` commands for power users. `@`-mention entity browser for discovery. Fast/Thinking mode for advanced users.                                                     |
| F6.7  | Error prevention       | Send button disabled when input is empty or only whitespace. AI provider connection tested before first message. Context limit handled gracefully with user notification.                                                          |
| F6.8  | User control           | User can stop AI response mid-stream. Can toggle sources on/off. Can choose Fast or Thinking mode. Can remove pinned entities. Can undo import.                                                                                    |
| F6.9  | Accessibility          | All interactive elements have aria-labels. Keyboard navigable. Focus management between input, autocomplete, command palette, and entity browser panel.                                                                            |
| F6.10 | Cognitive load         | Entity entities rendered as inline pills (Notion/Linear pattern). Context strip shows aggregate count only. Empty state shows relevant suggestions. Error messages use plain language.                                             |
| F6.11 | Transparency           | Users can see what the AI is doing (intermediate processing states), which entities were used (citations + "View sources"), and can inspect entity details by clicking citations. System prompt is inspectable via `/help system`. |

---

## Non-Functional Requirements

### NF1: Performance

| ID    | Requirement                                             | Target        | Acceptable    |
| ----- | ------------------------------------------------------- | ------------- | ------------- |
| NF1.1 | `@`-mention autocomplete response (first keystroke)     | < 100ms       | < 300ms       |
| NF1.2 | Chat view cold load (conversation list + first message) | < 500ms       | < 1s          |
| NF1.3 | Message send → first response token displayed           | < 2s          | < 5s          |
| NF1.4 | Streaming response throughput                           | ≥ 30 char/sec | ≥ 10 char/sec |
| NF1.5 | Import single `.docx` file                              | < 1s          | < 3s          |
| NF1.6 | Import `.xlsx` with 1000 rows                           | < 10s         | < 30s         |

### NF2: UX

| ID    | Requirement                                          | Target                                                                    |
| ----- | ---------------------------------------------------- | ------------------------------------------------------------------------- |
| NF2.1 | Chat input placeholder hints at `@` and `/` features | "Ask anything... Use @ to reference entities, / for commands"             |
| NF2.2 | Empty state shows suggested questions                | 3-5 example questions based on entity count                               |
| NF2.3 | Message loading state shows shimmer skeleton         | Typing indicator with entity context                                      |
| NF2.4 | Error messages use plain language, not error codes   | "I couldn't find any entities matching '@tra...' Try a different search." |
| NF2.5 | All chat UI is keyboard-navigable                    | Tab, Enter, Escape, Arrow keys                                            |
| NF2.6 | Dark and light mode support                          | Follows OS theme preference                                               |

---

## User Stories

### US1: Import Research Papers for Chat

**As a** researcher,
**I want to** drag a Word document onto the app and then ask questions about it,
**So that** I can extract insights from papers without reading them fully.

**Acceptance criteria:**
1. Drag a `.docx` file onto the import view
2. Progress bar shows processing status with file name
3. Entity appears in browser with Title, Content, Tags components
4. Post-import suggests: "Try asking about this in Chat" with clickable link
5. Navigate to Chat, type `@` followed by the paper title
6. Select the paper from autocomplete (appears as inline pill in the message)
7. Ask "summarize the key findings"
8. Processing states show: "Searching knowledge graph...", "Reading 1 entity...", "Generating response..."
9. AI responds with paper-grounded summary and numbered citations `[1]`
10. Hover citation → preview card shows entity title, type, and content snippet
11. Click citation → navigates to entity detail panel

### US2: Database Import for Business Users

**As a** business analyst,
**I want to** connect to a SQL database and import tables as entities,
**So that** I can ask questions about business data in natural language.

**Acceptance criteria:**
1. Open import view, click "Database" tab
2. Enter connection string for SQLite/PostgreSQL/MySQL
3. Connection test succeeds with visual confirmation
4. Select tables to import
5. Column mapping step: map database columns to entity components (Title, Tags, Content)
6. Each row imports as an entity with column data as components
7. Post-import summary shows "150 entities from 3 tables" with "Explore in Chat" button
8. Navigate to Chat, type `@` and select the imported collection
9. Ask "which customers had the highest revenue?" → AI searches and answers from imported data with citations

### US3: Chat with Entity References and Citations

**As a** knowledge worker,
**I want to** reference specific entities in my conversation with the AI,
**So that** answers are grounded in my knowledge graph and I can verify the sources.

**Acceptance criteria:**
1. Type `@` in chat input
2. Autocomplete shows matching entities with type badges. Recently viewed entities appear first.
3. Select "Attention Is All You Need (Paper)" — it appears as an inline `@Paper:Attention Is All You Need` pill in the message text
4. Ask "what architecture does this paper propose?"
5. Processing states: "Searching...", "Reading 1 entity...", "Generating response..."
6. AI responds describing the Transformer architecture
7. Response includes `[1]` citation after the supported statement
8. "View sources" footer shows: `[1] @Paper:Attention Is All You Need`
9. Thumbs up/down buttons on AI response
10. Click thumbs down → reason selection popup: "Used incorrect entity", "Missing information", "Wrong citation", "Other"

### US4: Slash Commands for Power Users

**As a** power user,
**I want to** use slash commands to control the app from the chat input,
**So that** I don't need to switch between views.

**Acceptance criteria:**
1. Type `/` in chat input
2. Searchable command palette appears. Shows: icon + command name + description + argument types in `[brackets]`
3. Typing more characters filters commands by prefix match
4. Arrow keys navigate, Enter selects, Escape closes
5. `/search transformer` → search results appear inline in chat as formatted entity cards with type badge, title, score
6. `/graph Attention` → app navigates to graph view centered on the entity
7. `/help` → formatted list of all commands with examples shown in chat
8. `/help search` → detailed help for the `/search` command

### US5: New User Onboarding with Adaptive Modes

**As a** first-time user,
**I want to** import my documents and start chatting immediately,
**So that** I see value without reading documentation.

**Acceptance criteria:**
1. Fresh install: import view shown as first experience (not dashboard)
2. Hint text guides: "Drag your documents here, or paste a URL"
3. After first import, app suggests: "3 entities created. Try asking about them in Chat"
4. Chat opens in "Fast" mode by default (new users get faster responses)
5. Empty state shows sample questions relevant to imported entities:
   - If papers: "Ask 'what are my papers about?'"
   - If general: "Try typing @ to reference an entity"
6. Input placeholder hints: "Ask anything... Use @ to reference entities"
7. First `@` attempt: autocomplete appears naturally (no tooltip needed — it just works)
8. Source toggles default: "Knowledge graph" ON, "Web search" OFF
9. Users can switch to "Thinking" mode later for deeper analysis

---

## Architecture

### Crate Changes

| Crate                  | Change                                                                                                                                                                                                                                                                                                                                                                                           |
| ---------------------- | ------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------ |
| `knowledge-core`       | Add `ChatCompletion` port trait. Add `MessageContent`, `EntityRefs` component types                                                                                                                                                                                                                                                                                                              |
| `knowledge-derivation` | Add `features/chat/` module with `ChatPipeline`, providers (OpenAI, Ollama, Mock)                                                                                                                                                                                                                                                                                                                |
| `knowledge-import`     | Add importers for Microsoft Office (doc/docx, xls/xlsx/xlsm, ppt/pps/pptx, msg, pst/ost), OpenDocument (odt/ott, ods/ots, odp/otp, odg), Apple iWork (pages, numbers, key), email (eml, mbox, ics, vcf), structured data (csv, json, xml, yaml), plain text (rtf, txt), note-taking exports (enex, opml, Notion JSON), URL fetch, clipboard, and database connectors (SQLite, PostgreSQL, MySQL) |
| `knowledge-storage`    | No changes (existing port traits support new entity types)                                                                                                                                                                                                                                                                                                                                       |
| `desktop`              | Add `Chat.svelte`, `Import.svelte` redesign, `@-mention` component, `/`-command component                                                                                                                                                                                                                                                                                                        |

### New ChatCompletion Trait

```rust
// knowledge-core/src/ports/chat.rs

use async_trait::async_trait;
use futures::Stream;

#[async_trait]
pub trait ChatCompletion: Send + Sync {
    /// Send a chat request and receive a complete response.
    async fn chat(&self, request: ChatRequest) -> Result<ChatResponse, ChatError>;

    /// Send a chat request and receive a streaming response.
    async fn chat_stream(
        &self,
        request: ChatRequest,
    ) -> Result<Box<dyn Stream<Item = ChatDelta> + Send + Unpin>, ChatError>;
}

/// A single message in a conversation.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Message {
    pub role: MessageRole,
    pub content: String,
    pub entity_refs: Vec<Uuid>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum MessageRole {
    System,
    User,
    Assistant,
}

/// Structured context for an entity referenced in a message.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EntityContext {
    pub entity_id: Uuid,
    pub entity_type: String,
    pub title: String,
    pub content: String,
    pub tags: Vec<String>,
    pub relationships: Vec<RelationshipSummary>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RelationshipSummary {
    pub relationship_type: String,
    pub target_id: Uuid,
    pub target_title: String,
    pub target_type: String,
}

/// A citation linking a statement in the response to an entity source.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CitationSource {
    pub number: usize,
    pub entity_id: Uuid,
    pub entity_type: String,
    pub title: String,
    pub snippet: String,
}

/// Response mode — determines depth of context retrieval and reasoning.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ResponseMode {
    Fast,     // Fewer context retrievals, faster response
    Thinking, // Full entity context, deeper reasoning, full citations
}

/// Which knowledge sources the LLM should use.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SourceToggles {
    pub knowledge_graph: bool, // Search and use entities from the user's knowledge graph
    pub web_search: bool,      // Search the web for additional context
}

/// Feedback from the user on a response.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ResponseFeedback {
    pub message_id: Uuid,
    pub rating: FeedbackRating,
    pub reason: Option<FeedbackReason>,
    pub comment: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum FeedbackRating {
    ThumbsUp,
    ThumbsDown,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum FeedbackReason {
    WrongEntity,     // Referenced the wrong entity
    MissingInfo,     // Response lacked needed information
    WrongCitation,   // Citation pointed to incorrect source
    Other,
}

/// A request to the chat completion provider.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChatRequest {
    /// System prompt (assembled from entity context + knowledge graph instructions).
    pub system_prompt: String,
    /// Conversation history (user + assistant messages).
    pub messages: Vec<Message>,
    /// Entity context to inject into the system prompt.
    pub context_entities: Vec<EntityContext>,
    /// Response mode (Fast or Thinking).
    pub mode: ResponseMode,
    /// Which knowledge sources to use.
    pub source_toggles: SourceToggles,
    /// Configuration for the provider.
    pub config: ChatConfig,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChatConfig {
    pub temperature: f64,
    pub max_tokens: u32,
    pub model: Option<String>,
}

/// A complete chat response.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChatResponse {
    pub message: String,
    pub citations: Vec<CitationSource>,
    pub referenced_entities: Vec<Uuid>,
}

/// Processing status updates emitted during response generation.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ProcessingStatus {
    Searching { detail: String },
    ReadingEntities { count: u32 },
    Generating,
}

/// A delta chunk in a streaming response.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChatDelta {
    pub delta: String,
    pub citation: Option<usize>,  // Citation number, if this delta starts a new citation
    pub status: Option<ProcessingStatus>,
    pub finished: bool,
}

#[derive(Debug, thiserror::Error)]
pub enum ChatError {
    #[error("Provider error: {0}")]
    Provider(String),
    #[error("Network error: {0}")]
    Network(String),
    #[error("Rate limited: {0}")]
    RateLimited(String),
    #[error("Context too long: {0}")]
    ContextTooLong(String),
}
```

### Chat Pipeline

```rust
// knowledge-derivation/src/features/chat/pipeline.rs

pub struct ChatPipeline {
    chat_provider: Box<dyn ChatCompletion>,
    entity_repo: Arc<dyn EntityRepository>,
    component_repo: Arc<dyn ComponentRepository>,
    relationship_repo: Arc<dyn RelationshipRepository>,
    search_index: Arc<dyn SearchIndex>,
    vector_store: Arc<dyn VectorStore>,
}

impl ChatPipeline {
    /// Send a chat message with optional entity references.
    /// Returns the AI response and persists the conversation.
    pub async fn chat(
        &self,
        conversation_id: Option<Uuid>,
        user_message: &str,
        entity_refs: &[Uuid],
    ) -> Result<ChatResult, ChatError> {
        // 1. Load or create conversation
        // 2. Load entity context for all referenced entities
        // 3. If no explicit refs, run search to find relevant entities
        // 4. Build system prompt with entity context
        // 5. Call chat_provider.chat() or chat_stream()
        // 6. Save user message + AI response as Message entities
        // 7. Create has_message relationships
        // 8. Return result
    }

    /// Retrieve entity context for a set of entity IDs.
    async fn build_entity_context(
        &self,
        entity_ids: &[Uuid],
    ) -> Result<Vec<EntityContext>, ChatError> {
        // For each entity: load components, load relationships
        // Build EntityContext with title, content, tags, relationships
    }

    /// Search for relevant entities when no explicit refs are given.
    async fn search_entities(
        &self,
        query: &str,
        limit: usize,
    ) -> Result<Vec<EntityContext>, ChatError> {
        // Hybrid search: keyword + semantic
        // Return top-k entities with context
    }

    /// Build system prompt from entity contexts.
    fn build_system_prompt(&self, contexts: &[EntityContext]) -> String {
        // Instruct the LLM about Knowledge OS capabilities
        // Include entity context with type, title, content, relationships
        // Instruct to use @EntityType:Title format for references in responses
    }
}
```

### Chat Provider Adapters

```rust
// knowledge-derivation/src/features/chat/providers/mod.rs

pub fn create_chat_provider(config: &str) -> Result<Box<dyn ChatCompletion>, ChatError> {
    // "mock://" → MockChatAdapter
    // "openai://gpt-4o?api_key=KEY" → OpenAiChatAdapter (uses api.openai.com)
    // "openai://gpt-4o?api_key=KEY&base_url=http://localhost:1234/v1" → LM Studio, vLLM, llama.cpp, etc.
    // "ollama://llama3.2" → OllamaChatAdapter (uses localhost:11434)
    // "ollama://llama3.2?url=http://192.168.1.50:11434" → remote Ollama instance
    // Environment variable fallbacks:
    //   KOS_CHAT_PROVIDER, OPENAI_API_KEY, KOS_OPENAI_BASE_URL, OLLAMA_HOST
}

// knowledge-derivation/src/features/chat/providers/openai.rs
pub struct OpenAiChatAdapter {
    client: reqwest::Client,
    model: String,
    api_key: String,
    base_url: String,   // "https://api.openai.com/v1" (default) or any OpenAI-compatible endpoint
}
impl ChatCompletion for OpenAiChatAdapter {
    /* calls POST {base_url}/chat/completions
       Works with: OpenAI API, LM Studio (localhost:1234/v1),
       vLLM, llama.cpp server, Together AI, Groq, etc. */
}

// knowledge-derivation/src/features/chat/providers/ollama.rs
pub struct OllamaChatAdapter {
    client: reqwest::Client,
    model: String,
    endpoint: String,   // "http://localhost:11434" (default) or any Ollama host
}
impl ChatCompletion for OllamaChatAdapter { /* calls POST {endpoint}/api/chat */ }

// knowledge-derivation/src/features/chat/providers/mock.rs
pub struct MockChatAdapter { /* deterministic echo for testing */ }
impl ChatCompletion for MockChatAdapter {
    // Returns a canned response referencing the provided entity context
}
```

### Conversation Persistence

Conversations and messages are stored as canonical entities:

```sql
-- Entity table already exists; Conversations and Messages use it:
-- entity_type = "Conversation" or "Message"

-- Component storage uses existing Component table:
-- MessageContent component: { role: "user"|"assistant"|"system", text: "..." }
-- EntityRefs component: { entity_ids: ["uuid1", "uuid2"] }
-- Title component: conversation title

-- Relationship storage uses existing Relationship table:
-- (conversation) --has_message--> (message)
-- (message) --part_of--> (conversation)
-- (message) --references--> (any entity)
```

### Tauri Commands

```rust
// desktop/src-tauri/src/commands/chat.rs

#[tauri::command]
async fn chat_send(
    state: State<'_, AppState>,
    conversation_id: Option<String>,
    message: String,
    entity_refs: Vec<String>,
) -> Result<ChatResult, String> { /* delegates to ChatPipeline::chat() */ }

#[tauri::command]
async fn chat_stream(
    state: State<'_, AppState>,
    conversation_id: Option<String>,
    message: String,
    entity_refs: Vec<String>,
) -> Result<(), String> { /* emits events via Tauri event system for streaming */ }

#[tauri::command]
async fn chat_search_entities(
    state: State<'_, AppState>,
    prefix: String,
) -> Result<Vec<EntitySearchResult>, String> {
    // Lightweight search for @-mention autocomplete
    // Searches by title prefix + type
    // Returns results fast (< 100ms)
}

#[tauri::command]
async fn chat_list_conversations(
    state: State<'_, AppState>,
) -> Result<Vec<ConversationSummary>, String> {
    // Lists all Conversation entities with last message preview
}

#[tauri::command]
async fn chat_delete_conversation(
    state: State<'_, AppState>,
    conversation_id: String,
) -> Result<(), String> {
    // Archives conversation and all its messages
}

#[tauri::command]
async fn chat_rename_conversation(
    state: State<'_, AppState>,
    conversation_id: String,
    title: String,
) -> Result<(), String> {
    // Updates the Title component of the conversation entity
}

#[tauri::command]
async fn chat_stop_stream(
    state: State<'_, AppState>,
    conversation_id: String,
) -> Result<(), String> {
    // Cancels the active streaming response
}
```

```rust
// desktop/src-tauri/src/commands/import.rs (additions)

#[tauri::command]
async fn import_url(
    state: State<'_, AppState>,
    url: String,
) -> Result<ImportResult, String> {
    // Fetches URL, detects content type, routes to appropriate importer
}

#[tauri::command]
async fn import_clipboard(
    state: State<'_, AppState>,
    text: String,
    source_format: String, // "text" | "html"
) -> Result<ImportResult, String> {
    // Creates entity from clipboard content
}

#[tauri::command]
async fn import_database(
    state: State<'_, AppState>,
    connection_string: String,
    tables: Vec<String>,
) -> Result<ImportResult, String> {
    // Connects to database, imports selected tables
}

#[tauri::command]
async fn undo_import(
    state: State<'_, AppState>,
    import_id: String,
) -> Result<(), String> {
    // Reverts the entities created in the last import
}
```

### Frontend Architecture

```
desktop/src/
  views/
    Chat.svelte              ← NEW: Full chat view with messages, citations, source toggles, feedback
    Import.svelte            ← MODIFIED: Tabbed import (files, URL, clipboard, database, CSV)
  lib/
    api.ts                   ← MODIFIED: Add chat + new import functions
    types.ts                 ← MODIFIED: Add chat types, citation types, feedback types
    state.svelte.ts          ← MODIFIED: Add chat state, mode state, source toggle state
    router.svelte.ts         ← MODIFIED: Add /chat route
    mention-autocomplete.ts  ← NEW: @-mention autocomplete with recency boost
    command-palette.ts       ← NEW: / command palette with search, arg hints, keyboard navigation
    chat-stream.ts           ← NEW: Streaming response handler with token accumulation
    citations.ts             ← NEW: Citation rendering, hover preview cards, source listing
    feedback.ts              ← NEW: Thumbs up/down UI, reason selection, submission
    processing-states.ts     ← NEW: Animated status indicators for intermediate pipeline states
    source-toggles.ts        ← NEW: Knowledge source toggle controls
```

#### Chat View Component Layout

```
┌────────────────────────────────────────────────────────────────────────────┐
│ Chat                                                               [New]   │
├──────────────────────────┬─────────────────────────────────────────────────┤
│ Conversations            │ Messages                                        │
│                          │                                                 │
│ ⇤ Today                 │ [AI] 👋 I found 3 papers                        │
│   Q3 review      ••      │      about transformers.                        │
│   Onboarding             │      The Transformer arch[1]                    │
│ ⇤ Yesterday             │       uses self-attention...                    │
│   Research notes         │                                                 │
│                          │ ┌────────────────────────────┐                  │
│                          │ │ [1] @Paper: Attention      │                  │
│                          │ │ Is All You Need            │                  │
│                          │ │ ════════════════════════   │                  │
│                          │ │ Type: Paper · 2017         │                  │
│                          │ │ Score: 0.94                │                  │
│                          │ └────────────────────────────┘                  │
│                          │                                                 │
│                          │ [You] @Attention paper                          │
│                          │       what architecture?                        │
│                          │                                                 │
│                          │ ┌────────────────────────────┐                  │
│                          │ │ ⚡ Thinking mode           │                  │
│                          │ │ ✓ Searched (3 entities)    │                  │
│                          │ │ ✓ Reading entities...      │                  │
│                          │ │ ⟳ Generating response     │                  │
│                          │ └────────────────────────────┘                  │
│                          │                                                 │
│                          │ ┌────────────────────────────┐                  │
│                          │ │ 1 entity pinned via @      │                  │
│                          │ │ [Paper] Attention...   [×] │                  │
│                          │ ├────────────────────────────┤                  │
│                          │ │ ⚡ Thinking  [📖][🌐]     │                  │
│                          │ │ Ask anything...  [@] [/]   │                  │
│                          │ └────────────────────────────┘                  │
├──────────────────────────┴─────────────────────────────────────────────────┤
│ 3 conversations · Thinking mode · GPT-4o · 12 entities                     │
└────────────────────────────────────────────────────────────────────────────┘
```

### SQL Patterns for Entity Autocomplete

```sql
-- @-mention autocomplete query (optimized for prefix search)
SELECT e.id, e.entity_type,
       c.data->>'name' AS title,
       c2.data->>'text' AS preview
FROM entities e
JOIN components c ON c.entity_id = e.id AND c.component_type = 'Title'
LEFT JOIN components c2 ON c2.entity_id = e.id AND c2.component_type = 'Description'
WHERE e.is_active = 1
  AND (c.data->>'name' LIKE ?1 || '%'            -- prefix match on title
       OR e.entity_type LIKE ?1 || '%')           -- prefix match on type
ORDER BY e.updated_at DESC
LIMIT 10;
```

### System Prompt Template

The chat pipeline constructs the following system prompt for the LLM. The template adapts based on whether explicit `@`-mentions were provided or implicit search was used:

```
You are Knowledge OS, a knowledge graph assistant.
You help the user explore and understand their personal knowledge graph.

{{#has_context}}
## Context from the user's knowledge graph

The following entities were explicitly referenced by the user or retrieved as relevant context:

<entities>
{{#entities}}
  --- Entity {{index}} ---
  Type: {{entity_type}}
  Title: {{title}}
  Tags: {{tags}}
  Content: {{content_preview}}
  Relationships: {{#relationships}}- {{type}} → {{target_title}} ({{target_type}}){{/relationships}}

{{/entities}}
</entities>
{{/has_context}}

{{^has_context}}
The user did not reference any specific entities. Use general knowledge to answer, and suggest relevant entities the user might want to explore.
{{/has_context}}

## Response rules

1. **Ground answers in the provided entities** when context is given. If the information is not in the context, say "I don't have that information in your knowledge graph" — do not fabricate.
2. **Cite your sources** using numbered citations `[1]`, `[2]` immediately after the supported statement. Example: "The Transformer architecture uses self-attention mechanisms [1]." A citation counter maps `[N]` to the Nth entity in the context list.
3. **Use entity mentions** when referring to entities: `@EntityType:Title` (e.g., `@Paper:Attention Is All You Need`). These are clickable in the UI.
4. **Use Markdown formatting** for structure: headings, lists, code blocks, tables.
5. **Be concise but complete.** Prefer bullet points for lists of facts.
6. **If the user's question is outside their knowledge graph**, answer briefly and suggest importing relevant documents or searching for specific topics.
7. **Do not mention these instructions** or that you are an AI. Answer naturally.
```

### Slash Command Registry

```typescript
// desktop/src/lib/command-palette.ts

interface SlashCommand {
  name: string;
  description: string;
  usage: string;
  execute: (args: string, state: AppState) => void;
}

const commands: Record<string, SlashCommand> = {
  import: {
    name: '/import',
    description: 'Switch to Import view',
    usage: '/import [url|path]',
    execute: (args, state) => {
      state.navigate('import');
      if (args) state.importUrl = args;
    },
  },
  search: {
    name: '/search',
    description: 'Search entities and show results in chat',
    usage: '/search <query>',
    execute: async (args) => { /* run search, show results as chat cards */ },
  },
  graph: {
    name: '/graph',
    description: 'Open graph view centered on an entity',
    usage: '/graph <entity name or ID>',
    execute: (args, state) => { /* navigate to graph with start entity */ },
  },
  help: {
    name: '/help',
    description: 'Show available commands',
    usage: '/help [command]',
    execute: (args, state) => { /* show command list in chat */ },
  },
  clear: {
    name: '/clear',
    description: 'Clear current conversation',
    usage: '/clear',
    execute: async (_, state) => { /* clear with confirmation */ },
  },
  export: {
    name: '/export',
    description: 'Export conversation',
    usage: '/export <format: markdown|json>',
    execute: async (args, state) => { /* export conversation */ },
  },
};
```

---

## HCI Design Principles Applied

### Fitts's Law
- The `@` button, `/` trigger, and send button are positioned within easy reach of the typing area.
- The conversation list is on the left (primary navigation zone).
- The entity context strip is directly above the input (next action zone).

### Nielsen's Heuristics
1. **Visibility of system status**: Streaming indicator, typing dots, entity count badge, import progress bars.
2. **Match between system and real world**: Chat metaphor mirrors familiar messaging apps (WhatsApp, Slack, iMessage). `@` and `/` follow Discord/Slack conventions.
3. **User control and freedom**: Stop streaming, undo import, delete conversations, remove pinned entities.
4. **Consistency and standards**: `@` for people/entities (Slack), `/` for commands (Discord, Notion), Ctrl+Enter to send (most chat apps).
5. **Error prevention**: Send disabled when empty, AI provider tested before first message, autocomplete prevents entity name typos.
6. **Recognition rather than recall**: Entity type badges in autocomplete, preview in search, entity chips show context.
7. **Flexibility and efficiency of use**: `@` quick mode for power users, entity browser panel for discovery. Keyboard shortcuts for all actions.
8. **Aesthetic and minimalist design**: Chat view focuses on messages. Entity context is a thin strip, not a panel. Conversations sidebar collapsible.
9. **Help users recognize, diagnose, and recover from errors**: Plain language errors ("I couldn't find any entities matching..."), undo actions, retry buttons.
10. **Help and documentation**: Slash `/help` command, empty state suggestions, tooltips on `@` and `/`.

### Shneiderman's Eight Golden Rules
1. **Strive for consistency**: `@` and `/` patterns are consistent across all chat interactions.
2. **Enable frequent users to use shortcuts**: Power users type `@` directly; `/` commands skip UI navigation.
3. **Offer informative feedback**: Every action has immediate visual feedback (streaming, progress bars, animations).
4. **Design dialog to yield closure**: Import completes with summary; message stream ends with completion indicator.
5. **Offer error prevention and simple error handling**: Autocomplete prevents entity lookup errors; undo handles mistakes.
6. **Permit easy reversal of actions**: Undo import, delete conversation (with archive, not hard delete).
7. **Keep users in control**: Stop streaming, remove entity chips, edit messages, cancel operations.
8. **Reduce short-term memory load**: Entity chips stay visible, context strip shows pinned items, conversation history persists.

---

## CLI Interface

### New Commands

```bash
# Import office files
kos import document.docx

# Import from URL
kos import https://example.com/article

# Import from database (matches existing kos import pattern)
kos import --db sqlite:///path/to/db.db --tables customers,orders

# Import Evernote export
kos import notebook.enex

# List conversations
kos conversation list

# Show conversation
kos conversation get <id>

# Delete conversation
kos conversation delete <id>
```

### Desktop IPC Commands (Internal)

```
invoke('chat_send', {
        conversation_id: null,
        message: "hello",
        entity_refs: [],           # UUIDs of @-mentioned entities
        mode: "thinking"|"fast",   # Default: "thinking"
        source_toggles: { graph: true, web: false }
    })
    -> {
        conversation_id: "uuid",
        message_id: "uuid",
        message: "...",
        citations: [{ number: 1, entity_id: "uuid", entity_type: "Paper", title: "..." }],
        referenced_entities: ["uuid"]
    }

invoke('chat_stream', { ... })           # Same params as chat_send, returns via Tauri events
    events:
        'chat:status'  -> { status: "searching|reading|generating", detail: "3 entities" }
        'chat:delta'   -> { delta: "text", citation: null|{number, entity_id} }
        'chat:done'    -> { message_id, citations, referenced_entities }
        'chat:error'   -> { error: "context_limit|rate_limit|provider_error", message: "..." }

invoke('chat_search_entities', { prefix: "transfor" })
    -> [{ entity_id, entity_type, title, preview, view_count, last_viewed_at }]

invoke('chat_list_conversations')
    -> [{ id, title, last_message_preview, message_count, updated_at }]

invoke('chat_delete_conversation', { conversation_id: "uuid" }) -> ()

invoke('chat_rename_conversation', { conversation_id: "uuid", title: "New Title" }) -> ()

invoke('chat_send_feedback', {
        message_id: "uuid",
        rating: "up"|"down",
        reason: null|"wrong_entity"|"missing_info"|"wrong_citation"|"other",
        comment: null|"..."
    }) -> ()

invoke('chat_stop_stream', { conversation_id: "uuid" }) -> ()

invoke('import_url', { url: "https://..." }) -> { created, merged, errors }

invoke('import_clipboard', { text: "...", source_format: "text"|"html" }) -> { created, merged, errors }

invoke('import_file', { path: "...", recursive: true|false }) -> { created, merged, errors }
    # Auto-detects format by extension + magic bytes. Routes to correct importer.
    # When path is a directory: if recursive=true, scans all subdirectories;
    #   if recursive=false, imports only files directly in that directory.

invoke('import_directory_preview', { path: "...", recursive: true|false })
    -> { total_files: 42, depth: 3, formats: { "docx": 10, "pdf": 20, "md": 12 } }
    # Shows file count and format breakdown before import starts.

invoke('import_image', { path: "..." }) -> { created, merged, errors, ocr_status: "pending"|"done"|"failed" }
    # Imports image files. OCR runs async if backend configured.

invoke('set_provider', {
        kind: "chat"|"ocr",
        provider: "openai"|"openai-compatible"|"ollama"|"tesseract"|"mock",
        model: "gpt-4o"|"deepseek-ocr"|"llama3.2-vision",
        base_url: null|"http://localhost:1234/v1",
        api_key: null|"..."
    }) -> { success: bool }

invoke('get_providers_status')
    -> {
        chat: { provider: string, model: string, base_url: string, reachable: bool, latency_ms: u32 },
        ocr: { backend: string, model: string, base_url: string, reachable: bool },
    }

invoke('chat_test_provider', {
        provider: "openai"|"openai-compatible"|"ollama",
        model: "gpt-4o"|"llama3.2-vision",
        base_url: null|"http://localhost:1234/v1",
        api_key: null|"..."
    })
    -> { success: bool, latency_ms: u32, error: null|string }

invoke('import_database', { connection_string: "...", tables: ["..."] }) -> { created, merged, errors }

invoke('import_structured_preview', { path: "...", format: "csv"|"json"|"xml"|"yaml" })
    -> { columns: ["Name", "Date", "..."], sample_rows: [{...}, {...}] }

invoke('import_structured', { path: "...", format: "csv"|"json"|"xml"|"yaml", column_mapping: { title: "Name", tags: "Category", content: "Description", entity_type: "Person" } })
    -> { created, merged, errors }

invoke('import_enex', { path: "..." }) -> { created, merged, errors, notebooks: ["..."] }

invoke('import_mbox', { path: "..." }) -> { created, merged, errors, total_messages }

invoke('undo_import', { import_id: "uuid" }) -> ()
```

---

## Acceptance Criteria

### Definition of Done

- [ ] User can import Microsoft Office files (`.doc`/`.docx`, `.xls`/`.xlsx`/`.xlsm`, `.ppt`/`.pps`/`.pptx`, `.msg`) via drag-and-drop
- [ ] User can import OpenDocument files (`.odt`/`.ott`, `.ods`/`.ots`, `.odp`/`.otp`, `.odg`)
- [ ] User can import Apple iWork files (`.pages`, `.numbers`, `.key`)
- [ ] User can import email and communication files (`.eml`, `.mbox`, `.ics`, `.vcf`)
- [ ] User can import structured data files (`.csv`, `.json`, `.xml`, `.yaml`) with column mapping preview
- [ ] User can import plain text documents (`.rtf`, `.txt`)
- [ ] User can import note-taking exports (`.enex`, `.opml`, Notion JSON, Obsidian vault)
- [ ] User can import from URL and clipboard
- [ ] User can import from SQL databases (SQLite, PostgreSQL, MySQL) with column mapping preview
- [ ] Import shows per-file progress, results summary, column mapping UI (CSV/database), conflict detection, undo button, post-import suggested actions
- [ ] Chat view exists as a sidebar tab with conversation history sidebar
- [ ] User can send messages and receive streaming AI responses with inline numbered citations `[1]`, `[2]`
- [ ] Citation hover shows entity preview card; click navigates to entity detail
- [ ] "View sources" footer lists all referenced entities
- [ ] Source toggles (knowledge graph, web) visible in chat composer and functional
- [ ] `@` triggers entity autocomplete (both quick-type and browser panel) with recency boost
- [ ] `/` triggers searchable command palette with argument hints and keyboard navigation
- [ ] Commands work: `/search`, `/graph`, `/import`, `/help`, `/clear`, `/export`
- [ ] Intermediate processing states visible: "Searching...", "Reading N entities...", "Generating..."
- [ ] Fast/Thinking mode toggle works and affects retrieval depth
- [ ] Feedback mechanism: thumbs up/down with reason selection, persisted
- [ ] Context limit handling: "Continue conversation" button with graceful truncation
- [ ] Stop generation button works during streaming
- [ ] Conversations are persisted as entities and survive app restart
- [ ] MockChatAdapter works without any API key for offline/demo use
- [ ] Chat works with OpenAI, OpenAI-compatible (LM Studio, vLLM, llama.cpp, etc.), Ollama, and Mock providers
- [ ] User can configure chat provider via UI (provider selection + optional base URL + optional API key)
- [ ] Provider connection test (`chat_test_provider`) verifies reachability and returns latency
- [ ] Chat provider and OCR backend configuration persisted across app restarts
- [ ] OCR works with Tesseract (CPU, local), Ollama vision models (DeepSeek-OCR, Qwen3-VL), and OpenAI-compatible vision endpoints
- [ ] OCR backend configurable via UI
- [ ] OCR runs asynchronously; entity `Content` component updates when OCR completes
- [ ] Embedded images in office files are OCR'd; text merges into the entity's `Content`
- [ ] Chat follows HCI principles: progressive disclosure, undo, keyboard navigation, accessibility labels, error messages in plain language
- [ ] MCP architectural compatibility: `ChatCompletion` trait, `EntityRetrievalService`, and all request/response types are serializable and framework-agnostic
- [ ] All existing tests pass; new tests for chat pipeline, importers, and UI components
- [ ] Desktop app builds on macOS, Windows, and Linux

### Test Cases

1. **Round-trip chat with @-mention**: Import a document. Open chat. Type `@`, select entity from autocomplete (appears as inline pill). Ask a question. Verify AI response references the entity with numbered citation `[1]`. Verify "View sources" shows the entity. Click citation → navigates to entity detail.
2. **Citation hover and click**: AI response with `[1]` citation. Hover over `[1]` → preview card appears with entity title, type, and snippet. Click `[1]` → navigates to entity detail panel.
3. **Streaming with processing states**: Send message. Verify states appear in sequence: "Searching knowledge graph...", "Reading N entities...", "Generating response...". Verify response streams token-by-token.
4. **Source toggles**: Turn off "Knowledge graph" source toggle. Send message. Verify AI responds from general knowledge only with no citations. Turn "Knowledge graph" back on. Verify citations reappear.
5. **Feedback mechanism**: Send message. Click thumbs up → visual confirmation. Send another message. Click thumbs down → reason selection appears. Select "Wrong citation" → feedback recorded. Verify feedback persists across restart.
6. **Fast vs Thinking mode**: Switch to "Fast" mode. Send message. Verify response appears faster but with fewer citations. Switch to "Thinking" mode. Send same message. Verify response has more citations and detail.
7. **Context limit handling**: Send enough messages to approach LLM context limit. Verify "Continue conversation" button appears. Click it. Verify oldest messages are truncated gracefully with a summary. Chat continues normally.
8. **Stop generation**: Send a long message. Click stop button during streaming. Verify stream halts. Verify partial response is preserved and shown. Verify user can send another message.
9. **Import office file**: Drag `.docx` onto import. Verify entity created with Content component containing extracted text. Verify post-import suggestion appears: "Try asking about this in Chat".
10. **CSV import with column mapping**: Import `.csv` file. Verify column mapping UI shows columns and sample rows. Map "Name" → Title, "Category" → Tags. Verify entities created with correct component data.
11. **Database import with preview**: Connect to test SQLite database. Verify connection test succeeds. Select table. Verify column mapping step. Import. Verify entities created with correct components.
12. **Conflict detection**: Import a file. Import the same file again. Verify "Already imported" message with skip option. Verify no duplicate entities created unless user chooses re-import.
13. **Slash command palette**: Type `/` in chat input. Verify searchable palette opens with command name, description, argument hints. Type `/search test`. Verify search results appear inline as formatted entity cards. Type `/help`. Verify command list shown in chat.
14. **Conversation persistence**: Send messages across multiple conversations. Restart app. Verify all conversations appear in sidebar with correct titles and last message previews. Verify clicking a conversation loads its history.
15. **Keyboard navigation**: Tab through: chat input → source toggles → send button. Arrow keys in `@` autocomplete. Arrow keys in `/` command palette. Enter to select. Escape to close palettes. Tab through "View sources" items.
16. **Empty state (new user)**: Fresh install with no entities. Open chat. Verify empty state shows: "Import some documents to get started!" with link to Import view. No hallucinated suggestions.
17. **Empty state (has entities)**: Import 3 documents. Open chat. Verify empty state shows entity-aware suggestions: "Ask 'what are my latest papers?'". Verify these work when sent.
18. **Chat works offline (Mock adapter)**: No AI provider configured. Open chat. Verify it works. Send message. Verify MockAdapter returns deterministic response. Verify no error modals.
19. **OCR from embedded image in .docx**: Import a `.docx` file containing an embedded image with text. Verify OCR backend runs. Verify entity `Content` component contains extracted text from the image. Verify image blob preserved (OCR is derived).
20. **OCR backend switching**: Configure Tesseract backend. OCR an image. Verify text extracted. Switch to Ollama backend (DeepSeek-OCR mock). Re-run OCR on same image. Verify different (or better) text extracted. Verify no data loss.
21. **LM Studio / OpenAI-compatible endpoint**: Point chat provider to `http://localhost:1234/v1` with model `local-model`. Send message. Verify streaming response works. Verify connection test returns success with latency.
22. **Provider configuration persistence**: Configure OpenAI chat provider. Restart app. Verify provider configuration is loaded. Verify chat works without reconfiguration.
23. **Provider connection test**: Go to settings. Click "Test Connection" for Ollama with wrong URL. Verify error shown. Fix URL. Verify success shown with latency.

---

## Testing Strategy

| Level       | Scope                                                                                            | Framework                            |
| ----------- | ------------------------------------------------------------------------------------------------ | ------------------------------------ |
| Unit        | `ChatCompletion` trait + adapters                                                                | `cargo test`                         |
| Unit        | `ChatPipeline` logic                                                                             | `cargo test -p knowledge-derivation` |
| Unit        | Importers (Microsoft Office, OpenDocument, iWork, email, structured data, plain text, databases) | `cargo test -p knowledge-import`     |
| Unit        | OCR backends (Tesseract, Ollama, API, Mock)                                                      | `cargo test -p knowledge-import`     |
| Unit        | Image extraction from office files                                                               | `cargo test -p knowledge-import`     |
| Integration | Chat pipeline + store round-trip                                                                 | `cargo test --test integration`      |
| Integration | Import pipeline + store + chat pipeline                                                          | `cargo test --test integration`      |
| Integration | OCR pipeline: image → OCR → content update                                                       | `cargo test --test integration`      |
| E2E         | Desktop app: import → chat → @-mention                                                           | Manual + Tauri WebDriver             |
| E2E         | Desktop app: `/` commands → view navigation                                                      | Manual                               |
| BDD         | Chat + import scenarios                                                                          | `cargo test --test cucumber`         |

---

## Risks and Mitigations

| Risk                                                                                                                          | Impact | Likelihood | Mitigation                                                                                                                        |
| ----------------------------------------------------------------------------------------------------------------------------- | ------ | ---------- | --------------------------------------------------------------------------------------------------------------------------------- |
| **Office file parsing quality** — `.docx`/`.xlsx` extraction may miss complex formatting                                      | Medium | Medium     | Use `calamine` for Excel (battle-tested), `docx-rs` for Word. Fallback to basic text extraction.                                  |
| **Database import schema mismatch** — Tables may not map cleanly to entities                                                  | Medium | Medium     | User selects which columns map to Title, Tags, Content. Preview before import.                                                    |
| **Chat provider API changes** — OpenAI/Anthropic API may change                                                               | Medium | Low        | Adapter pattern isolates provider code. Version-pin API clients.                                                                  |
| **LLM hallucination from entity context** — AI fabricates facts not in provided context                                       | High   | Medium     | System prompt strictly constrains: "If the answer is not in the context, say so." Confidence scores in future.                    |
| **Chat streaming performance** — Slow model makes chat feel unresponsive                                                      | Medium | Low        | Streaming required by spec (30 char/sec). Provide MockChatAdapter for instant responses.                                          |
| **User types sensitive data in chat** — LLM provider sees user's knowledge                                                    | High   | Medium     | Document in security model. Ollama adapter for local-only. Warning when using cloud provider.                                     |
| **OCR quality varies by backend** — Tesseract is fast but less accurate on complex layouts; Ollama/API is accurate but slower | Medium | Medium     | Default to Tesseract for speed, allow switching to Ollama/API for quality. OCR is derived — can be re-run with different backend. |
| **External provider unreachable** — LM Studio, Ollama, or OpenAI API may be down or not installed                             | Medium | Medium     | Provider connection test before first use. Graceful fallback to Mock adapter. Clear error messages with setup instructions.       |
| **OCR GPU requirement** — Ollama multimodal models need GPU for acceptable speed                                              | Low    | Medium     | Tesseract backend works on CPU. Ollama backend documented as GPU-recommended with CPU fallback (slower).                          |

---

## Dependencies

### External Crates

| Crate          | Version | Purpose                         | Justification                                                                                                             |
| -------------- | ------- | ------------------------------- | ------------------------------------------------------------------------------------------------------------------------- |
| `docx-rs`      | 0.4.x   | Parse `.docx` files             | Pure Rust, extracts text + formatting from Word files                                                                     |
| `calamine`     | 0.27.x  | Parse `.xlsx` files             | Battle-tested Excel parser in Rust                                                                                        |
| `pptx-rs`      | 0.2.x   | Parse `.pptx` files             | Extract text from PowerPoint slides                                                                                       |
| `csv`          | 1.3.x   | Parse `.csv` files              | Robust CSV parsing with flexible quoting/escaping options                                                                 |
| `sqlx`         | 0.8.x   | Database connectivity           | Async, supports PostgreSQL, MySQL, SQLite                                                                                 |
| `tokio-stream` | 0.1.x   | Streaming chat responses        | Async stream adapter for SSE                                                                                              |
| `serde`        | 1.x     | Serialization                   | Already a dependency; used for component serialization                                                                    |
| `reqwest`      | 0.12.x  | HTTP client for URL import      | Already a dependency                                                                                                      |
| `futures`      | 0.3.x   | Async stream combinators        | Already a dependency; used for ChatCompletion stream type                                                                 |
| `tesseract-rs` | 0.3.x   | OCR text extraction             | Pure Rust bindings with optional built-in Tesseract compilation (`build-tesseract` feature). No system dependency needed. |
| `image`        | 0.25.x  | Image loading and preprocessing | Load images from disk and memory for OCR. Already a transitive dependency.                                                |

### Internal Dependencies

- `docs/architecture/domain-model.md` — **Must be updated** to add `Conversation`, `Message` entity types, `MessageContent`, `EntityRefs` component types, and `has_message`, `part_of`, `referenced_by` relationship types.
- `docs/architecture/pipeline.md` — Chat pipeline at Layer 6 (Derivation). Import expands Layer 1. Chat UI is Layer 7.
- `docs/architecture/data-model.md` — Conversations and Messages are canonical data (cannot be regenerated). Chat context payloads are derived (disposable).
- `docs/architecture/events.md` — `ConversationCreated`, `MessageCreated`, `EntityReferenced` events added.
- `docs/architecture/data-model.md` — Chat context payloads are derived data.
- `PRD-0006` — Desktop MVP provides the base app shell, sidebar, navigation, and existing views that this PRD extends.
- `PRD-0001` — Core entity model provides the storage foundation for conversations.
- `PRD-0003` — Search index and vector store are reused for `@`-mention autocomplete and RAG context retrieval.

---

## Timeline

| Phase                                    | Duration | Deliverables                                                                                                                                                                                                                                                                                                                                                                                                                                                          |
| ---------------------------------------- | -------- | --------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| Phase 1: Chat infrastructure             | 4 days   | `ChatCompletion` trait, `CitationSource`, `ResponseMode`, `SourceToggles`, `ResponseFeedback`, `ProcessingStatus` types. Mock/OpenAI/Ollama adapters. `ChatPipeline` with RAG, citation tracking, mode dispatch. Tauri commands.                                                                                                                                                                                                                                      |
| Phase 2: Chat UI                         | 5 days   | `Chat.svelte` with streaming, inline citations `[N]` with hover preview, "View sources" footer, source toggles, Fast/Thinking mode toggle, intermediate processing states, stop generation, feedback (thumbs up/down + reason), `@`-mention autocomplete (quick + browser panel) with recency boost, `/` searchable command palette with arg hints.                                                                                                                   |
| Phase 3: Conversation persistence        | 2 days   | Conversation/Message entity storage, sidebar history with last message preview, rename/delete, EntityRefs component.                                                                                                                                                                                                                                                                                                                                                  |
| Phase 4: Universal import                | 6 days   | Importers for Microsoft Office (doc/docx, xls/xlsx/xlsm, ppt/pps/pptx, msg, pst/ost), OpenDocument (odt/ott, ods/ots, odp/otp, odg), Apple iWork (pages, numbers, key), email/communication (eml, mbox, ics, vcf), structured data (csv, json, xml, yaml), plain text (rtf, txt), note-taking exports (enex, opml, Notion JSON). Database connectors (SQLite/PostgreSQL/MySQL). URL fetch, clipboard import, column mapping UI, format auto-detection by magic bytes. |
| Phase 5: OCR pipeline                    | 3 days   | `OcrBackend` trait, Tesseract backend (`tesseract-rs`), Ollama vision backend (DeepSeek-OCR/Qwen3-VL via HTTP), OpenAI-compatible vision backend, Mock backend. Async OCR processing queue. Image extraction from office files. OCR configuration UI. Provider connection test.                                                                                                                                                                                       |
| Phase 6: Import UX redesign              | 3 days   | Tabbed import view (Files/URL/Clipboard/Database), per-file progress bars, post-import suggested actions, undo, watch directory, collapsible errors.                                                                                                                                                                                                                                                                                                                  |
| Phase 7: MCP architectural compatibility | 1 day    | `EntityRetrievalService` extraction, Serialize derives on all types, framework-agnostic separation.                                                                                                                                                                                                                                                                                                                                                                   |
| Phase 8: Provider configuration UI       | 2 days   | Settings panel for chat provider (OpenAI/Ollama/OpenAI-compatible/Mock) and OCR backend (Tesseract/Ollama/API/Mock). Connection test UI. Persistence.                                                                                                                                                                                                                                                                                                                 |
| Phase 9: Polish and testing              | 3 days   | HCI audit, empty states, error messages in plain language, context limit handling, accessibility labels, OCR quality tests, E2E tests, documentation.                                                                                                                                                                                                                                                                                                                 |

**Total: ~4.5 weeks**

---

## References

- Shneiderman, B. *Designing the User Interface* (6th ed., 2016) — Eight Golden Rules
- Nielsen, J. *10 Usability Heuristics for User Interface Design* (1994)
- Fitts, P. M. "The information capacity of the human motor system" (1954)
- Norman, D. *The Design of Everyday Things* (2013) — Affordances, signifiers, feedback
- Krug, S. *Don't Make Me Think* (2014) — Usability principles
- Anthropic. "Model Context Protocol (MCP)" (2025) — Reference for future tool integration
