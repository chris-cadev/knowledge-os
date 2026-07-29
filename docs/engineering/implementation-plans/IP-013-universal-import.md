# IP-013: Universal Import with Database Connectors and Column Mapping

**Status:** Draft
**ADR(s):** [ADR-0027](../../architecture/adrs/adr-0027.md) (Universal Import with Database Connectors and Column Mapping)
**PRD(s):** [PRD-0007](../prds/prd-0007-knowledge-chat-and-universal-import.md) (F1.A–F1.J Universal Import, Import UX Redesign)
**Estimated effort:** ~6 days

---

## Context

ADR-0027 extends the import layer (ADR-0007) with ~30 new format adapters, a `DatabaseSource` trait for SQL connectivity, a `ColumnMapping` type for structured data, URL/clipboard acquisition, directory watching, recursive directory toggle, conflict detection, and undo. This is the largest single plan in the PRD-0007 implementation.

**Current state:**
- `core/knowledge-import/src/features/importer/mod.rs` registers `markdown`, `pdf`, `url` adapters. All other formats are new.
- `core/knowledge-import/src/features/importer/adapter.rs` defines the `ImportAdapter` trait. New adapters implement this trait.
- `core/knowledge-import/Cargo.toml` includes `pdf_oxide`, `pulldown-cmark`, `regex`, `reqwest`, `serde_yaml`, `tempfile`. New dependencies for office formats: `docx-rs`, `calamine`, `pptx-rs`. New dependency for databases: `sqlx` with `sqlite`, `postgres`, `mysql` features. New dependency for directory watching: `notify`.
- `desktop/src-tauri/src/commands/import.rs` has a single `import_files` command. New commands are added: `import_url`, `import_clipboard`, `import_database`, `undo_import`, `import_file_recursive`, `import_image`.
- `desktop/src/views/Import.svelte` is a single drop zone for .md and .pdf files. It is rewritten to a tabbed view (Files / URL / Clipboard / Database).

**Dependencies:**
- IP-012 (OCR backend) — image extraction from office files uses `OcrBackend` for embedded images
- This plan is otherwise independent

This plan is the largest in scope. It is delivered in 9 deliverables to allow incremental testing and committing.

---

## Deliverables

### D1: Microsoft Office Format Adapters (docx, xlsx, pptx, doc, xls, xlsm, ppt, pps)

**Purpose:** Add adapters for Microsoft Office formats using battle-tested Rust libraries.

**Files:**

| File | Action | Description |
|------|--------|-------------|
| `core/knowledge-import/Cargo.toml` | Modify | Add `docx-rs = "0.4"`, `calamine = "0.27"`, `pptx-rs = "0.2"` |
| `core/knowledge-import/src/features/importer/docx.rs` | Create | `DocxImporter` using `docx-rs` |
| `core/knowledge-import/src/features/importer/xlsx.rs` | Create | `XlsxImporter` using `calamine` (one entity per row by default) |
| `core/knowledge-import/src/features/importer/pptx.rs` | Create | `PptxImporter` using `pptx-rs` (one entity per slide) |
| `core/knowledge-import/src/features/importer/legacy_office.rs` | Create | `DocImporter`, `XlsImporter`, `PptImporter`, `PpsImporter` for legacy binary formats (use `calamine` for xls; legacy .doc/.ppt are out of scope for P0 — return `ImportError::UnsupportedFormat` with clear message) |
| `core/knowledge-import/src/features/importer/xlsm.rs` | Create | `XlsmImporter` reusing `XlsxImporter` logic (macro-enabled workbook) |
| `core/knowledge-import/src/features/importer/mod.rs` | Modify | Re-export all new adapters |
| `core/knowledge-import/src/features/importer/magic_bytes.rs` | Create | Format detection by magic bytes for files without extensions |

**Implementation outline for DocxImporter:**

```rust
// core/knowledge-import/src/features/importer/docx.rs
use docx_rs::Document;
use std::path::Path;
use super::adapter::*;

pub struct DocxImporter;

impl DocxImporter {
    pub fn new() -> Self { Self }
}

#[async_trait]
impl ImportAdapter for DocxImporter {
    fn can_import(&self, path: &Path) -> bool {
        path.extension().and_then(|e| e.to_str())
            .map(|e| e.eq_ignore_ascii_case("docx"))
            .unwrap_or(false)
    }

    async fn import(&self, path: &Path) -> Result<ImportResult, ImportError> {
        let bytes = std::fs::read(path)?;
        let doc = Document::from_read(&bytes[..])
            .map_err(|e| ImportError::Parse(e.to_string()))?;

        // Extract paragraphs
        let mut text = String::new();
        for child in doc.document.children {
            if let docx_rs::DocumentChild::Paragraph(p) = child {
                for run in p.children {
                    if let docx_rs::ParagraphChild::Run(r) = run {
                        for child in r.children {
                            if let docx_rs::RunChild::Text(t) = child {
                                text.push_str(&t.text);
                                text.push('\n');
                            }
                        }
                    }
                }
            }
        }

        // Extract images for OCR (delegated to OcrPipeline in IP-013 D8)
        let image_refs: Vec<ImageRef> = vec![];  // TODO: extract via docx-rs

        // Create entity
        let entity = Entity::new(EntityType::new("Article"));
        let title = path.file_stem().and_then(|s| s.to_str()).unwrap_or("Untitled").to_string();
        let components = vec![
            Component::new(entity.id, ComponentType::Title, serde_json::json!(title)),
            Component::new(entity.id, ComponentType::Content, serde_json::json!(text)),
            Component::new(entity.id, ComponentType::Provenance, serde_json::json!({
                "source": path.to_string_lossy(),
                "imported_at": chrono::Utc::now().to_rfc3339(),
                "format": "docx",
            })),
        ];

        Ok(ImportResult { entity, components, cross_references: vec![] })
    }

    fn supported_extensions(&self) -> &[&str] { &["docx"] }
}
```

**Implementation outline for XlsxImporter:**

```rust
// core/knowledge-import/src/features/importer/xlsx.rs
use calamine::{open_workbook_auto, DataType, Reader};
use std::path::Path;
use super::adapter::*;

pub struct XlsxImporter;

impl XlsxImporter {
    pub fn new() -> Self { Self }
}

#[async_trait]
impl ImportAdapter for XlsxImporter {
    fn can_import(&self, path: &Path) -> bool {
        let ext = path.extension().and_then(|e| e.to_str()).unwrap_or("");
        ext.eq_ignore_ascii_case("xlsx")
            || ext.eq_ignore_ascii_case("xls")
            || ext.eq_ignore_ascii_case("xlsm")
    }

    async fn import(&self, path: &Path) -> Result<ImportResult, ImportError> {
        let mut workbook = open_workbook_auto(path)
            .map_err(|e| ImportError::Parse(e.to_string()))?;

        // For P0: import the first sheet's rows as entities
        // (multi-sheet UI is a future enhancement)
        let sheet_name = workbook.sheet_names().first()
            .ok_or_else(|| ImportError::Parse("workbook has no sheets".into()))?
            .clone();
        let range = workbook.worksheet_range(&sheet_name)
            .map_err(|e| ImportError::Parse(e.to_string()))?;

        // ... convert rows to entities, return first as ImportResult
        // (multi-row imports return Err with multi-entity result via a different API)
        todo!()
    }

    fn supported_extensions(&self) -> &[&str] { &["xlsx", "xls", "xlsm"] }
}
```

**Note:** Structured data (XLSX) requires column mapping (D5). For P0, the importer returns a `MultiEntityImport` type that the import pipeline routes to the column mapping UI. This is a deviation from the single-entity `ImportResult` interface. The D5 deliverable formalizes this.

**Tests** (use `include_bytes!` for sample files in tests/ directory):
- `docx_imports_text_content` — verify text is extracted
- `docx_imports_title_from_filename` — verify Title component
- `docx_unsupported_format` — `.txt` returns `ImportError::UnsupportedFormat`
- `xlsx_imports_first_sheet` — verify cells are extracted
- `pptx_imports_slides` — verify slide titles are extracted
- `magic_bytes_detects_docx` — file without extension detected as docx by `PK\x03\x04`
- `magic_bytes_detects_pdf` — file without extension detected as pdf by `%PDF-`

**Verification:**
- `cargo test -p knowledge-import` passes with 7 new tests
- `cargo test --workspace` passes (no regressions)

**Exit criteria:** Office adapters implement the trait, format detection works.

---

### D2: OpenDocument Format Adapters (odt, ods, odp, odg, ott, ots, otp)

**Purpose:** Add adapters for OpenDocument formats. Most are ZIP-based XML.

**Files:**

| File | Action | Description |
|------|--------|-------------|
| `core/knowledge-import/src/features/importer/opendocument.rs` | Create | `OdtImporter`, `OdsImporter`, `OdpImporter`, `OdgImporter`, `OttImporter`, `OtsImporter`, `OtpImporter` (all use shared `extract_odf_text` helper) |
| `core/knowledge-import/Cargo.toml` | Modify | Add `zip = "2"` and `quick-xml = "0.36"` for ODF parsing |
| `core/knowledge-import/src/features/importer/mod.rs` | Modify | Re-export |

**Implementation outline:**

```rust
// core/knowledge-import/src/features/importer/opendocument.rs
fn extract_odf_text(path: &Path, expected_content_path: &str) -> Result<String, ImportError> {
    let file = std::fs::File::open(path)?;
    let mut archive = zip::ZipArchive::new(file)?;
    let mut content_file = archive.by_name(expected_content_path)
        .map_err(|_| ImportError::Parse(format!("missing {}", expected_content_path)))?;
    let mut content = String::new();
    use std::io::Read;
    content_file.read_to_string(&mut content)?;
    // Strip XML tags to get text
    let text = strip_xml_tags(&content);
    Ok(text)
}

pub struct OdtImporter;
#[async_trait]
impl ImportAdapter for OdtImporter {
    fn can_import(&self, path: &Path) -> bool {
        matches!(path.extension().and_then(|e| e.to_str()).map(|e| e.to_lowercase()).as_deref(),
            Some("odt") | Some("ott"))
    }
    async fn import(&self, path: &Path) -> Result<ImportResult, ImportError> {
        let text = extract_odf_text(path, "content.xml")?;
        // ... create entity with text
        todo!()
    }
    fn supported_extensions(&self) -> &[&str] { &["odt", "ott"] }
}

// Similar for OdsImporter (sheet content), OdpImporter (slide content), etc.
```

**Tests:**
- `odt_imports_text_content`
- `ods_imports_first_sheet_rows`
- `odp_imports_slide_text`
- `ott_treated_as_odt_template`
- `odf_unsupported_format`

**Verification:**
- `cargo test -p knowledge-import` passes with 5 new tests

**Exit criteria:** OpenDocument adapters work for common cases.

---

### D3: Email and Communication Adapters (eml, mbox, ics, vcf, msg)

**Purpose:** Add adapters for email and communication formats.

**Files:**

| File | Action | Description |
|------|--------|-------------|
| `core/knowledge-import/src/features/importer/email.rs` | Create | `EmlImporter` (parses RFC 5322 email), `MsgImporter` (Outlook .msg via `mailparse` crate) |
| `core/knowledge-import/src/features/importer/mbox.rs` | Create | `MboxImporter` (parses mbox archive) |
| `core/knowledge-import/src/features/importer/calendar_contact.rs` | Create | `IcsImporter` (parses iCalendar), `VcfImporter` (parses vCard) |
| `core/knowledge-import/Cargo.toml` | Modify | Add `mailparse = "0.15"` for EML/MSG parsing |
| `core/knowledge-import/src/features/importer/mod.rs` | Modify | Re-export |

**Tests:**
- `eml_imports_headers_and_body` — verify From, To, Subject, Date components
- `mbox_imports_multiple_messages` — verify per-message entity creation
- `ics_imports_events` — verify each VEVENT becomes entity
- `vcf_imports_contacts` — verify each VCARD becomes Person entity
- `msg_imports_outlook_email` — verify similar to EML

**Verification:**
- `cargo test -p knowledge-import` passes with 5 new tests

**Exit criteria:** Email adapters work for common cases.

---

### D4: Apple iWork Adapters (pages, numbers, key)

**Purpose:** Add adapters for Apple iWork formats. iWork files are bundles (directories) or ZIP archives.

**Files:**

| File | Action | Description |
|------|--------|-------------|
| `core/knowledge-import/src/features/importer/iwork.rs` | Create | `PagesImporter`, `NumbersImporter`, `KeynoteImporter` (handle both bundle and zip formats) |
| `core/knowledge-import/src/features/importer/mod.rs` | Modify | Re-export |

**Note:** iWork formats are reverse-engineered. Parsing is best-effort. The PRD marks these as P1/P2 with the risk of format breakage on new iWork versions.

**Tests:**
- `pages_imports_text_from_index_xml` — extract text from `index.xml` in the bundle
- `numbers_imports_rows` — extract rows from Numbers sheet XML
- `keynote_imports_slides` — extract text from Keynote slide XML
- `iwork_unsupported_format` — return clear error for malformed bundles

**Verification:**
- `cargo test -p knowledge-import` passes with 4 new tests

**Exit criteria:** iWork adapters handle common cases.

---

### D5: Structured Data Adapters and Column Mapping (csv, json, xml, yaml)

**Purpose:** Add adapters for structured data with the column mapping pattern from ADR-0027.

**Files:**

| File | Action | Description |
|------|--------|-------------|
| `core/knowledge-core/src/ports/import_structured.rs` | Create | `ColumnMapping`, `FieldMapping`, `ImportPreview` types |
| `core/knowledge-core/src/ports/mod.rs` | Modify | Re-export |
| `core/knowledge-import/src/features/importer/structured.rs` | Create | `CsvImporter`, `JsonImporter`, `XmlImporter`, `YamlImporter` with column mapping support |

**New types:**

```rust
// core/knowledge-core/src/ports/import_structured.rs
use serde::{Deserialize, Serialize};
use std::collections::{HashMap, HashSet};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ColumnMapping {
    pub field_mappings: HashMap<String, FieldMapping>,
    pub skip_columns: HashSet<String>,
    pub entity_type_override: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum FieldMapping {
    Title,
    Description,
    Content,
    Tags { separator: String },
    CustomComponent { component_name: String },
    TimelineDate,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ImportPreview {
    pub columns: Vec<ColumnInfo>,
    pub sample_rows: Vec<Vec<ColumnValue>>,
    pub row_count: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ColumnInfo {
    pub name: String,
    pub data_type: String,
    pub nullable: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ColumnValue {
    Text(String),
    Integer(i64),
    Float(f64),
    Boolean(bool),
    Null,
}
```

**CsvImporter implementation:**

```rust
// core/knowledge-import/src/features/importer/structured.rs
use csv::ReaderBuilder;

pub struct CsvImporter;

#[async_trait]
impl ImportAdapter for CsvImporter {
    fn can_import(&self, path: &Path) -> bool {
        path.extension().and_then(|e| e.to_str())
            .map(|e| e.eq_ignore_ascii_case("csv")).unwrap_or(false)
    }

    async fn import(&self, _path: &Path) -> Result<ImportResult, ImportError> {
        // Structured data imports require column mapping
        // This method returns an error directing the caller to use preview() then import_with_mapping()
        Err(ImportError::Parse(
            "CSV import requires column mapping. Use preview() then import_with_mapping().".into()
        ))
    }

    fn supported_extensions(&self) -> &[&str] { &["csv"] }
}

impl CsvImporter {
    pub async fn preview(&self, path: &Path, sample_size: usize) -> Result<ImportPreview, ImportError> {
        let mut reader = ReaderBuilder::new().from_path(path)?;
        let headers: Vec<String> = reader.headers()?.iter().map(|s| s.to_string()).collect();
        let mut rows = Vec::new();
        for (i, record) in reader.records().enumerate() {
            if i >= sample_size { break; }
            let record = record?;
            rows.push(record.iter().map(|s| ColumnValue::Text(s.to_string())).collect());
        }
        Ok(ImportPreview {
            columns: headers.iter().map(|n| ColumnInfo {
                name: n.clone(),
                data_type: "text".to_string(),
                nullable: true,
            }).collect(),
            sample_rows: rows,
            row_count: reader.records().count() as u64,  // simplified
        })
    }

    pub async fn import_with_mapping(
        &self,
        path: &Path,
        mapping: &ColumnMapping,
    ) -> Result<Vec<ImportResult>, ImportError> {
        // Apply mapping to each row, create entity per row
        todo!()
    }
}
```

**Tests:**
- `csv_preview_returns_columns_and_sample`
- `csv_import_with_mapping_creates_entities` — map "Name"→Title, "Description"→Content
- `csv_skip_columns_excluded`
- `csv_tags_with_separator_splits_correctly`
- `json_array_imports_objects`
- `json_object_imports_single_entity`
- `xml_imports_elements`
- `yaml_imports_array`
- `yaml_imports_mapping`

**Verification:**
- `cargo test -p knowledge-import` passes with 9 new tests
- `cargo test -p knowledge-core` still passes

**Exit criteria:** Structured data adapters with column mapping work end-to-end.

---

### D6: Database Connectors (SQLite, PostgreSQL, MySQL)

**Purpose:** Add the `DatabaseSource` trait and three concrete implementations.

**Files:**

| File | Action | Description |
|------|--------|-------------|
| `core/knowledge-core/src/ports/database.rs` | Create | `DatabaseSource` trait, `ConnectionInfo`, `TableInfo`, `ColumnInfo`, `TablePreview`, `ColumnValue`, `DatabaseError` |
| `core/knowledge-core/src/ports/mod.rs` | Modify | Re-export |
| `core/knowledge-import/Cargo.toml` | Modify | Add `sqlx = { version = "0.8", features = ["runtime-tokio", "sqlite", "postgres", "mysql", "any"] }` |
| `core/knowledge-import/src/features/importer/database.rs` | Create | `SqliteDatabaseSource`, `PostgresDatabaseSource`, `MySqlDatabaseSource` |

**New trait:**

```rust
// core/knowledge-core/src/ports/database.rs
#[async_trait]
pub trait DatabaseSource: Send + Sync {
    async fn test_connection(&self) -> Result<ConnectionInfo, DatabaseError>;
    async fn list_tables(&self) -> Result<Vec<TableInfo>, DatabaseError>;
    async fn preview_table(&self, table: &str, limit: usize) -> Result<TablePreview, DatabaseError>;
    async fn stream_rows(
        &self,
        table: &str,
    ) -> Result<Box<dyn Stream<Item = Result<Vec<ColumnValue>, DatabaseError>> + Send + Unpin>, DatabaseError>;
}
```

**SQLite implementation:**

```rust
// core/knowledge-import/src/features/importer/database.rs
pub struct SqliteDatabaseSource {
    path: PathBuf,
}

#[async_trait]
impl DatabaseSource for SqliteDatabaseSource {
    async fn test_connection(&self) -> Result<ConnectionInfo, DatabaseError> {
        let start = std::time::Instant::now();
        let pool = sqlx::SqlitePool::connect(&format!("sqlite://{}", self.path.display())).await
            .map_err(|e| DatabaseError::Connection(e.to_string()))?;
        let version: String = sqlx::query_scalar("SELECT sqlite_version()")
            .fetch_one(&pool).await
            .map_err(|e| DatabaseError::Query(e.to_string()))?;
        let latency_ms = start.elapsed().as_millis() as u32;
        pool.close().await;
        Ok(ConnectionInfo {
            server_version: version,
            database_name: self.path.file_name().and_then(|n| n.to_str()).unwrap_or("").to_string(),
            reachable: true,
            latency_ms,
        })
    }

    async fn list_tables(&self) -> Result<Vec<TableInfo>, DatabaseError> {
        let pool = sqlx::SqlitePool::connect(&format!("sqlite://{}", self.path.display())).await
            .map_err(|e| DatabaseError::Connection(e.to_string()))?;
        // SELECT name FROM sqlite_master WHERE type='table'
        // For each table: PRAGMA table_info(name) for columns
        // COUNT(*) for row count
        todo!()
    }

    // ... preview_table, stream_rows similar
}
```

**PostgreSQL and MySQL implementations** follow the same pattern using `sqlx::PgPool` and `sqlx::MySqlPool` with their respective SQL dialects.

**Tests** (use `sqlx` in-memory SQLite for testing):
- `sqlite_test_connection_succeeds` — open in-memory DB, return version
- `sqlite_list_tables_empty` — empty DB returns empty list
- `sqlite_list_tables_populated` — created tables are listed
- `sqlite_preview_table_returns_columns_and_sample`
- `sqlite_stream_rows_yields_all_rows`
- `postgres_test_connection_failure` — invalid host returns Connection error
- `mysql_test_connection_failure` — invalid host returns Connection error

**Verification:**
- `cargo test -p knowledge-import` passes with 7 new tests
- Manual test: connect to a real PostgreSQL/MySQL instance

**Exit criteria:** Database trait and three implementations work.

---

### D7: Note-Taking App Adapters (enex, opml, Notion JSON)

**Purpose:** Add adapters for note-taking app exports.

**Files:**

| File | Action | Description |
|------|--------|-------------|
| `core/knowledge-import/src/features/importer/note_apps.rs` | Create | `EnexImporter`, `OpmlImporter`, `NotionJsonImporter` |
| `core/knowledge-import/src/features/importer/obsidian.rs` | Create | `ObsidianVaultImporter` (reuses `MarkdownImporter` for each .md file in a directory) |
| `core/knowledge-import/src/features/importer/mod.rs` | Modify | Re-export |

**Tests:**
- `enex_imports_notes` — verify each `<note>` becomes entity with Title, Content, Tags
- `opml_imports_outline_hierarchy` — verify `<outline>` nodes become entities with parent-child
- `notion_imports_pages` — verify each page becomes entity
- `obsidian_imports_directory` — verify all .md files in vault are imported

**Verification:**
- `cargo test -p knowledge-import` passes with 4 new tests

**Exit criteria:** Note-taking adapters work.

---

### D8: URL, Clipboard, Image Extraction from Office Files

**Purpose:** Add URL fetch (extending existing `UrlImporter`), clipboard acquisition, and image extraction from office files for OCR.

**Files:**

| File | Action | Description |
|------|--------|-------------|
| `core/knowledge-import/src/features/importer/url.rs` | Modify | Extend `UrlImporter` to handle content types beyond PDF/HTML (CSV, JSON, XML detected by content-type) |
| `core/knowledge-import/src/features/importer/clipboard.rs` | Create | `ClipboardImporter` accepting text or HTML content |
| `core/knowledge-import/src/features/importer/image.rs` | Create | `ImageImporter` for standalone image files; integrates with `OcrPipeline` (IP-012) |
| `core/knowledge-import/src/features/importer/docx.rs` | Modify | Extract embedded images, pass to OCR pipeline |
| `core/knowledge-import/src/features/importer/pptx.rs` | Modify | Extract embedded images, pass to OCR pipeline |
| `core/knowledge-import/src/features/importer/pdf.rs` | Modify | Extract embedded images, pass to OCR pipeline |

**Tests:**
- `url_imports_csv_from_url`
- `url_imports_json_from_url`
- `clipboard_imports_text`
- `clipboard_imports_html`
- `image_imports_png_with_ocr` — verify BinaryContent is set, OCR runs, Content is populated
- `image_imports_jpeg_with_ocr`
- `docx_extracts_embedded_images_for_ocr` — verify images are extracted and OCR'd

**Verification:**
- `cargo test -p knowledge-import` passes with 7 new tests

**Exit criteria:** URL/clipboard/image/embedded-image extraction works.

---

### D9: Directory Watching, Recursive Toggle, Conflict Detection, Undo

**Purpose:** Add directory watching, the recursive directory toggle, conflict detection on re-import, and undo of the last import.

**Files:**

| File | Action | Description |
|------|--------|-------------|
| `core/knowledge-import/src/features/importer/directory.rs` | Create | `DirectoryImporter` with recursive option, conflict detection |
| `core/knowledge-import/src/features/importer/watch.rs` | Create | `DirectoryWatcher` using `notify` crate |
| `core/knowledge-import/Cargo.toml` | Modify | Add `notify = "6"` |
| `core/knowledge-import/src/features/importer/undo.rs` | Create | `ImportRecord` type and `undo_last_import()` function |
| `core/knowledge-core/src/ports/import_record.rs` | Create | `ImportRecord` port trait (or store as Provenance component) |
| `core/knowledge-storage/src/adapters/sqlite/import_record.rs` | Create | SQLite storage for import records |

**Implementation outline for DirectoryImporter:**

```rust
pub struct DirectoryImporter {
    recursive: bool,
}

impl DirectoryImporter {
    pub fn new(recursive: bool) -> Self { Self { recursive } }

    pub fn list_files(&self, path: &Path) -> Result<Vec<PathBuf>, ImportError> {
        let mut files = Vec::new();
        if self.recursive {
            for entry in walkdir::WalkDir::new(path) {
                let entry = entry.map_err(|e| ImportError::Io(e.into()))?;
                if entry.file_type().is_file() {
                    files.push(entry.path().to_path_buf());
                }
            }
        } else {
            for entry in std::fs::read_dir(path)? {
                let entry = entry?;
                if entry.file_type()?.is_file() {
                    files.push(entry.path());
                }
            }
        }
        Ok(files)
    }
}
```

**Conflict detection** — uses content hash (SHA-256) of file bytes stored in `Provenance.content_hash` component. Re-import compares hashes. If match, surface as `AlreadyImported` and offer options.

**Tests:**
- `directory_lists_files_top_level_only` — recursive=false returns only top-level
- `directory_lists_files_recursive` — recursive=true includes subdirectories
- `directory_detects_format_by_magic_bytes`
- `conflict_detected_by_content_hash`
- `undo_archives_created_entities`
- `undo_removes_relationships`
- `watch_detects_new_file` — uses `notify` recommended test pattern with debounced events

**Verification:**
- `cargo test -p knowledge-import` passes with 7 new tests

**Exit criteria:** Directory import, watching, conflict, undo all work.

---

## Execution Order

```
D1 (Office) -> D2 (OpenDocument) -> D3 (Email) -> D4 (iWork)
  -> D5 (Structured + Column Mapping) -> D6 (Database)
  -> D7 (Note-taking) -> D8 (URL/Clipboard/Image)
  -> D9 (Directory/Watch/Conflict/Undo)
```

D1–D4 are format adapters (independent). D5 introduces the column mapping pattern (used by D6 and the desktop UI). D6 adds database connectivity. D7–D8 add remaining formats. D9 adds the UX-level features (recursive, watch, conflict, undo).

---

## Verification Strategy

| Level | Command | Coverage |
|-------|---------|----------|
| Unit | `cargo test -p knowledge-import` | All 55+ new tests across 9 deliverables |
| Integration | `cargo test -p knowledge-storage` | Import record storage |
| Lint | `cargo clippy --all-targets --all-features -- -D warnings && cargo fmt --check` | Code quality |
| Manual | `cargo tauri dev` | Desktop app import tab |

---

## Exit Criteria

- [ ] 30+ new `ImportAdapter` implementations across 9 files
- [ ] `DatabaseSource` trait with SQLite, PostgreSQL, MySQL implementations
- [ ] `ColumnMapping` and `FieldMapping` types
- [ ] `DirectoryImporter` with recursive toggle
- [ ] `DirectoryWatcher` for continuous import
- [ ] Conflict detection by content hash
- [ ] Undo last import
- [ ] 55+ new tests pass
- [ ] All existing tests pass (no regressions)
- [ ] `cargo clippy` and `cargo fmt` clean
- [ ] ADR-0027 updated with Implementation Notes

---

## Impact Analysis

### Structural Changes and Consumers

| Change | Direct Consumers | Transitive Consumers |
|--------|------------------|---------------------|
| `DatabaseSource` trait (new) | Database import in desktop UI | Future database sources (MSSQL, Oracle) |
| `ColumnMapping` types (new) | Structured data imports | Desktop column mapping UI |
| `ImportRecord` (new) | Undo functionality | Future import history view |
| `DirectoryWatcher` (new) | Future "watch folder" feature | Auto-import on file change |
| `knowledge-import` crate growth | All format adapters | Workspace rebuild |

### Risk Surface

1. **Office format parsing quality:** Some formats (legacy .doc, .ppt) have limited support. **Mitigation:** Clear `UnsupportedFormat` error messages with links to format conversion tools.

2. **Database schema mismatch:** Imported tables may not map cleanly to entities. **Mitigation:** Column mapping UI is mandatory before import. Preview shows the first 10 rows.

3. **OCR dependency for office images:** If `OcrPipeline` is not configured, embedded images are imported as `BinaryContent` only. **Mitigation:** The `Content` component is added when OCR completes. Without OCR, the document text is still imported.

4. **Recursive directory depth:** A deeply nested directory can produce thousands of files. **Mitigation:** The `Include subdirectories` toggle defaults to ON but the user can disable. The frontend shows total file count and depth before import starts (F1.J.10).

5. **Conflict detection by content hash:** Hashing large files is expensive. **Mitigation:** Hash is computed once on first import and stored in `Provenance.content_hash`. Re-import reads the stored hash first, then compares hashes of new files (avoid re-hashing if path matches).

6. **Notify crate platform support:** Directory watching on macOS uses FSEvents, Linux uses inotify, Windows uses ReadDirectoryChangesW. All three are supported by `notify`. **Mitigation:** The `notify` crate abstracts platform differences.

---

## Implementation Notes

*(Filled in during/after implementation — records deviations, discoveries, decisions made during coding)*
