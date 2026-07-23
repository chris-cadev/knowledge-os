# PRD-0002: Rich Import and Entity Resolution

**Status:** Draft
**Date:** 2026-07-22
**Author:** Core maintainers
**Priority:** P0 — Quality Layer
**Depends on:** PRD-0001

---

## Purpose

This PRD extends the import pipeline from basic Markdown-only to multi-format with intelligent entity resolution. It adds PDF import, fuzzy duplicate detection, extended cross-reference patterns, and entity type inference. This is the quality layer — it makes the knowledge graph trustworthy.

---

## Problem Statement

PRD-0001 proves the entity model works with Markdown. But real knowledge is heterogeneous: PDFs sit alongside Markdown notes, the same concept appears under different names across sources, and cross-references use inconsistent formats. Without fuzzy resolution and multi-format import, the graph accumulates noise faster than signal.

The evidence from the 2026 landscape (see [Landscape 2026](../../research/landscape-2026.md)) confirms that entity resolution is where knowledge graphs succeed or fail:

- "Extraction is the part that demos, resolution is the part that matters."
- LLMs consistently produce duplicate entities in GraphRAG pipelines. Reducing graph size by 40% through resolution improved QA performance.
- Children's Medical Center Dallas found 22% of records were duplicates before proper resolution.

PRD-0002 addresses this directly. It makes the canonical model clean by construction, not correction.

---

## Scope

### In Scope

- PDF import (text extraction + metadata)
- Fuzzy entity resolution (Levenshtein / Jaro-Winkler distance)
- Confidence scoring for resolution candidates
- Configurable merge thresholds per entity type
- Auditable merge decisions with reason and confidence
- Merge undo capability
- Extended cross-reference patterns (wikilinks, URLs, @mentions, section anchors)
- Entity type inference from frontmatter `type` field
- Import from URL
- Batch import progress reporting (indicatif integration)

### Out of Scope

- AI-assisted extraction (Year 2)
- Embedding-based semantic resolution (deferred to PRD-0003)
- Plugin system (PRD-0003)
- View projections (PRD-0003)
- Multi-user collaboration (Year 3)

---

## Engineering Questions

Every feature proposed for the system must answer these 10 questions per [Engineering Principles](../../philosophy/engineering-principles.md):

### 1. Which canonical entities are introduced?

No new entity types are introduced. The system uses the core entity types from PRD-0001 and `docs/architecture/domain-model.md`.

### 2. Which relationships are introduced?

No new relationship types are introduced. The system uses the core relationship types. Extended cross-reference patterns produce additional `references` relationships using existing types.

### 3. Which components are introduced?

One new component type is introduced:

| Component       | Source                              |
| --------------- | ----------------------------------- |
| `BinaryContent` | PDF file reference, MIME type, size |

PDF import produces entities with `Content` (extracted text) and `BinaryContent` (reference to the original file) components.

### 4. Which events are emitted?

| Event               | Trigger                        | Consumers          |
| ------------------- | ------------------------------ | ------------------ |
| `EntityResolved`    | Duplicate detected and merged  | Audit log          |
| `MergeDecision`     | Resolution candidate evaluated | Audit log          |
| All PRD-0001 events | Existing pipeline events       | Existing consumers |

### 5. Which derived representations are generated?

| Derived Artifact      | Source                       | Regeneration           |
| --------------------- | ---------------------------- | ---------------------- |
| Resolution audit log  | Merge decisions + confidence | Rebuild from event log |
| Extended search index | Wikilinks, URLs, mentions    | Rebuild from canonical |

All derived data remains disposable and rebuildable from canonical data.

### 6. Which layer owns the feature?

| Feature               | Layer                                             |
| --------------------- | ------------------------------------------------- |
| PDF import            | Layer 1 (Import) + Layer 2 (Parsing)              |
| Entity type inference | Layer 2 (Parsing)                                 |
| Fuzzy resolution      | Layer 3 (Normalization)                           |
| Extended cross-refs   | Layer 2 (Parsing) + Layer 5 (Relationship Engine) |
| Resolution audit      | Layer 4 (Knowledge Model)                         |

### 7. Can every derived artifact be regenerated?

Yes. The resolution audit log is derived from canonical events. The extended search index is rebuilt from canonical entity data.

### 8. Does the feature violate storage independence?

No. All new storage (resolution audit log, PDF binary references) uses the existing adapter interface. The resolution system is accessed through a trait, not coupled to SQLite.

### 9. Does the feature introduce implementation leakage?

No. PDF parsing details are confined to the import adapter. Resolution strategies are trait-based and swappable. The domain model remains technology-independent.

### 10. Does the feature preserve the canonical model?

Yes. The canonical model is the source of truth. Resolution merges duplicates into canonical entities. PDF content becomes `Content` and `BinaryContent` components. Extended cross-references become `references` relationships.

---

## Pipeline Spine Analysis

The system spine remains: **Import → Extract → Resolve → Store → Connect → Search**

PRD-0002 enhances the Resolve step and extends the Import and Extract steps.

### Why Resolution Before Storage (Reinforced)

PRD-0001 established that resolution must happen before storage. PRD-0002 makes this more critical:

- Fuzzy resolution catches duplicates that exact matching misses
- PDF imports may reference the same paper as an existing Markdown note
- Wikilinks and @mentions create implicit connections that require resolution to resolve correctly

If resolution happens after storage, the graph degrades before correction. PRD-0002 ensures quality is maintained by construction.

### Extended Pipeline Flow

```
Markdown/PDF/URL Input
      |
  Import Layer         knowledge-cli: kos import command
      |
  Parsing Layer        pulldown-cmark (Markdown), lopdf (PDF)
                       Frontmatter type inference
      |
  Normalization Layer  Fuzzy resolution, confidence scoring
                       Extended cross-reference extraction
      |
  Knowledge Model      Entity + Components stored in SQLite
      |
  Relationship Engine  Cross-reference + wikilink + URL relationships
      |
  Derivation Layer     Search index update (extended patterns)
      |
  Event Log            Durable event emission (including merge decisions)
      |
  (Presentation)       CLI output
```

---

## Functional Requirements

### F1: PDF Import

| ID   | Requirement                          | Priority | Acceptance Criteria                                   |
| ---- | ------------------------------------ | -------- | ----------------------------------------------------- |
| F1.1 | Import PDF files                     | P0       | PDF file produces Entity with Content + BinaryContent |
| F1.2 | Extract text from PDF body           | P0       | Text extraction produces readable content             |
| F1.3 | Extract metadata from PDF properties | P0       | Title, author, date extracted from PDF metadata       |
| F1.4 | Handle scanned PDFs gracefully       | P1       | Scanned PDFs produce BinaryContent only, log warning  |
| F1.5 | Import PDF from URL                  | P1       | Remote PDFs are downloadable and importable           |

### F2: Entity Resolution

| ID   | Requirement                                  | Priority | Acceptance Criteria                                                |
| ---- | -------------------------------------------- | -------- | ------------------------------------------------------------------ |
| F2.1 | Fuzzy match resolution                       | P0       | Duplicate detection by title similarity (Levenshtein/Jaro-Winkler) |
| F2.2 | Confidence scoring per candidate             | P0       | Each resolution candidate has a confidence score (0.0–1.0)         |
| F2.3 | Configurable merge threshold per entity type | P0       | Different thresholds for Person vs. Concept vs. Article            |
| F2.4 | Auditable merge decisions                    | P0       | Every merge logged with reason, confidence, and source entities    |
| F2.5 | Merge undo capability                        | P1       | Merged entities can be split back via audit trail                  |
| F2.6 | Resolution strategy per entity type          | P0       | Different matching strategies for different entity types           |

### F3: Extended Import

| ID   | Requirement                               | Priority | Acceptance Criteria                                          |
| ---- | ----------------------------------------- | -------- | ------------------------------------------------------------ |
| F3.1 | Infer entity type from frontmatter `type` | P1       | Frontmatter `type` field overrides default Article           |
| F3.2 | Extract wikilinks as references           | P1       | `[[name]]` patterns produce `references` relationships       |
| F3.3 | Extract URLs as references                | P1       | HTTP/HTTPS links produce `references` relationships          |
| F3.4 | Extract @mentions as references           | P1       | `@name` patterns produce `references` relationships          |
| F3.5 | Extract section anchors from links        | P1       | `file.md#section` links preserve anchor metadata             |
| F3.6 | Import from URL                           | P1       | Remote Markdown files are downloadable and importable        |
| F3.7 | Batch import progress reporting           | P1       | Progress bar shows files processed, created, updated, errors |

---

## Non-Functional Requirements

### NF1: Performance

| ID    | Requirement                    | Target      | Acceptable  |
| ----- | ------------------------------ | ----------- | ----------- |
| NF1.1 | PDF import throughput          | 20 docs/sec | 10 docs/sec |
| NF1.2 | Fuzzy resolution latency       | < 50ms      | < 200ms     |
| NF1.3 | Resolution false positive rate | < 5%        | < 10%       |

### NF2: Scalability

| ID    | Requirement              | Target         |
| ----- | ------------------------ | -------------- |
| NF2.1 | Entity volume            | 100K entities  |
| NF2.2 | Resolution candidate set | 10K per import |
| NF2.3 | Batch import size        | 10K files      |

### NF3: Reliability

| ID    | Requirement             | Target                                      |
| ----- | ----------------------- | ------------------------------------------- |
| NF3.1 | Resolution auditability | Every merge decision logged with reason     |
| NF3.2 | Merge undo              | Merged entities splittable via audit trail  |
| NF3.3 | Pipeline idempotency    | Reimporting same input produces same output |

---

## User Stories

### US1: Import a PDF Paper

**As a** researcher,
**I want to** import a PDF paper into Knowledge OS,
**So that** it becomes a typed entity alongside my Markdown notes.

**Acceptance criteria:**
1. User runs `kos import paper.pdf`.
2. System extracts text from the PDF body.
3. System extracts title, authors, and date from PDF metadata.
4. System creates a `Paper` entity (if frontmatter `type` present) or `Article` (default).
5. System attaches `Content` (extracted text) and `BinaryContent` (file reference) components.
6. System checks for duplicate entities before storage.
7. Entity is stored in the canonical model.
8. Search index is updated.

### US2: Import a Directory with Mixed Formats

**As a** knowledge worker,
**I want to** import a folder containing both Markdown and PDF files,
**So that** each file becomes a properly typed entity.

**Acceptance criteria:**
1. User runs `kos import <directory>`.
2. System imports all `.md` and `.pdf` files.
3. Each file produces a separate entity with correct type.
4. Cross-references between files are extracted as relationships.
5. Duplicates are detected and resolved before storage.
6. Progress bar shows files processed.

### US3: Resolve Duplicates Across Imports

**As a** knowledge worker,
**I want to** have duplicate entities automatically detected and merged,
**So that** my knowledge graph stays clean without manual deduplication.

**Acceptance criteria:**
1. System detects fuzzy title matches (e.g., "Attention Is All You Need" vs. "Attention is All You Need").
2. System detects same-content entities from different sources (PDF + Markdown).
3. System presents resolution candidates with confidence scores.
4. High-confidence matches are merged automatically.
5. Low-confidence matches are flagged for review.
6. All merge decisions are logged with reason and confidence.
7. Merged entities can be undone.

### US4: Import a Paper from URL

**As a** researcher,
**I want to** import a Markdown file from a URL,
**So that** I can capture web content as entities.

**Acceptance criteria:**
1. User runs `kos import https://example.com/paper.md`.
2. System downloads the file.
3. System imports it as a normal Markdown entity.
4. Provenance records the URL as source.

---

## Architecture

### Crate Changes

| Crate               | Change                                                                                             |
| ------------------- | -------------------------------------------------------------------------------------------------- |
| `knowledge-core`    | String-based `EntityType` (Upgrade #1); `ResolutionCandidate` gains confidence + reason            |
| `knowledge-import`  | PDF importer via `ImportAdapter` trait; fuzzy resolution strategies; extended cross-ref extraction |
| `knowledge-storage` | Resolution audit log table; merge operations; PDF binary reference storage                         |

### PDF Importer

The PDF importer uses `lopdf` for text extraction and metadata parsing. It implements the `ImportAdapter` trait:

```rust
#[async_trait]
pub trait ImportAdapter: Send + Sync {
    fn can_import(&self, path: &Path) -> bool;
    async fn import(&self, path: &Path) -> Result<ImportResult, ImportError>;
    fn supported_extensions(&self) -> &[&str];
}
```

The Markdown importer is refactored to implement the same trait. The CLI selects the adapter based on file extension.

### Entity Resolution System

Resolution is accessed through the `EntityResolver` trait (defined in PRD-0001):

```rust
#[async_trait]
pub trait EntityResolver: Send + Sync {
    async fn find_candidates(&self, entity: &Entity, title: &str) -> Result<Vec<ResolutionCandidate>>;
    async fn merge(&self, canonical_id: Uuid, duplicate_id: Uuid, confidence: f64) -> Result<()>;
}
```

Resolution strategies:

| Strategy         | Method                           | Use Case                       |
| ---------------- | -------------------------------- | ------------------------------ |
| Exact match      | Title + entity type equality     | Fast, high confidence          |
| Normalized match | Lowercase + whitespace normalize | Catches casing differences     |
| Fuzzy match      | Levenshtein / Jaro-Winkler       | Catches typos, abbreviations   |
| Content match    | Body text similarity             | Same content, different titles |

Each strategy produces a confidence score. Strategies are composable — the resolver tries exact first, then falls back to fuzzy.

### Extended Cross-Reference Extraction

The `CrossReference` type evolves to support multiple reference patterns:

```rust
pub enum CrossReference {
    FileRef { target_path: PathBuf, link_text: String },
    UrlRef { url: String, link_text: String },
    WikilinkRef { target_name: String, link_text: String },
    MentionRef { target_name: String },
    SectionRef { target_path: PathBuf, section: String, link_text: String },
}
```

Wikilinks (`[[name]]`) require a pre-processing pass before pulldown-cmark, since they are not standard Markdown syntax. The extractor uses regex to find `[[...]]` patterns and converts them to entity name lookups.

### Entity Type Inference

Frontmatter `type` field is parsed as a string and matched against `EntityType` variants (case-insensitive). If not present or invalid, the default type (`Article`) is used.

```yaml
---
title: "Attention Is All You Need"
type: paper
tags: [transformer, attention]
---
```

This is a simple extension of the existing frontmatter extraction pattern. The importer already extracts `title`, `tags`, `author`, `language` — adding `type` is consistent.

### Storage Changes

| Table                   | Purpose                                |
| ----------------------- | -------------------------------------- |
| `resolution_log`        | Audit trail for merge decisions        |
| `resolution_candidates` | Candidates evaluated during resolution |

Schema additions:

```sql
CREATE TABLE resolution_log (
    id TEXT PRIMARY KEY,
    canonical_id TEXT NOT NULL,
    merged_id TEXT NOT NULL,
    confidence REAL NOT NULL,
    reason TEXT NOT NULL,
    strategy TEXT NOT NULL,
    created_at TEXT NOT NULL
);

CREATE TABLE resolution_candidates (
    id TEXT PRIMARY KEY,
    source_entity_id TEXT NOT NULL,
    candidate_entity_id TEXT NOT NULL,
    confidence REAL NOT NULL,
    strategy TEXT NOT NULL,
    evaluated_at TEXT NOT NULL
);
```

---

## CLI Interface

### Updated Commands

```bash
# Import PDF
kos import paper.pdf

# Import from URL
kos import https://example.com/paper.md

# Import directory (mixed formats)
kos import <directory>

# Search (unchanged)
kos search "query"
kos search "query" --type paper

# Get entity details (unchanged)
kos get <entity-id>

# List entities (unchanged)
kos list --type article

# Resolution audit (new)
kos resolution log
kos resolution log --entity <entity-id>
kos resolution undo <merge-id>
```

### Output Format

```
$ kos import paper.pdf
Imported: Entity abc123 (Paper)
  Title: "Attention Is All You Need"
  Authors: Vaswani, Shazeer, ...
  Tags: transformer, attention, NLP
  Content: 2847 words (extracted from PDF)
  BinaryContent: paper.pdf (2.3 MB)
  Search index: updated
  Resolution: no duplicates found
```

```
$ kos import mixed-dir/
[12/24] ===================> attention.pdf

--- Import Summary ---
Total files: 24
Created: 18
Updated: 3
Duplicates resolved: 2
Errors: 1

Resolution log:
  Merged "Attention Is All You Need" (pdf) into abc123 (confidence: 0.95)
  Merged "Self-Attention Mechanism" (md) into def456 (confidence: 0.82)
```

---

## Acceptance Criteria

### Definition of Done

- [ ] PDF import produces entities with Content + BinaryContent components
- [ ] PDF metadata extraction (title, authors, date) works
- [ ] Fuzzy resolution detects duplicates by title similarity
- [ ] Confidence scoring produces 0.0–1.0 scores per candidate
- [ ] Configurable merge thresholds per entity type
- [ ] Every merge decision is logged with reason and confidence
- [ ] Merged entities can be undone via audit trail
- [ ] Wikilinks extracted as references
- [ ] URLs extracted as references
- [ ] @mentions extracted as references
- [ ] Section anchors preserved in link metadata
- [ ] Entity type inferred from frontmatter `type` field
- [ ] Import from URL works
- [ ] Batch import progress bar works
- [ ] All tests pass
- [ ] No canonical data loss on any failure path

### Test Cases

1. **Import single PDF** — Creates entity with correct components
2. **Import PDF with metadata** — Extracts title, authors, date
3. **Import scanned PDF** — Produces BinaryContent only, logs warning
4. **Fuzzy duplicate detection** — Similar titles flagged as candidates
5. **Exact duplicate detection** — Same title + type triggers resolution
6. **Content-based duplicate detection** — Same body, different title
7. **Confidence scoring** — Candidates have confidence scores
8. **Merge threshold** — Configurable per entity type
9. **Merge undo** — Merged entities can be split
10. **Wikilink extraction** — `[[name]]` produces references relationship
11. **URL extraction** — HTTP links produce references relationship
12. **@mention extraction** — `@name` produces references relationship
13. **Frontmatter type inference** — `type: paper` creates Paper entity
14. **Import from URL** — Remote Markdown file imported correctly
15. **Batch import progress** — Progress bar shows correct counts

---

## Testing Strategy

### Test Levels

| Level             | Scope                                                    | Framework                       |
| ----------------- | -------------------------------------------------------- | ------------------------------- |
| Unit tests        | Resolution strategies, cross-ref extraction, PDF parsing | `#[cfg(test)]` modules          |
| Integration tests | PDF import pipeline, resolution pipeline                 | `tests/` directories            |
| End-to-end tests  | CLI import commands, full import→resolve→search flow     | `tests/` with process execution |

### Test Data

- PDF files with varying metadata quality
- PDF files with scanned content (no extractable text)
- Markdown files with wikilinks, @mentions, URLs
- Duplicate entity sets (exact, fuzzy, content-based)
- Mixed directories (Markdown + PDF)

---

## Risks and Mitigations

| Risk                             | Impact | Likelihood | Mitigation                                                |
| -------------------------------- | ------ | ---------- | --------------------------------------------------------- |
| PDF text extraction quality      | High   | Medium     | Use battle-tested `lopdf`; fallback to BinaryContent-only |
| Fuzzy resolution false positives | High   | Medium     | Confidence thresholds, human review for low-confidence    |
| Wikilink syntax variation        | Medium | High       | Support common patterns; document limitations             |
| Resolution performance at scale  | Medium | Low        | Index-based lookup; bounded candidate sets                |
| Merge undo complexity            | Medium | Low        | Event-sourced merge history; replay for undo              |

---

## Dependencies

### External Crates

| Crate       | Purpose             | Justification                                         |
| ----------- | ------------------- | ----------------------------------------------------- |
| `lopdf`     | PDF text extraction | Battle-tested PDF parsing library                     |
| `strsim`    | String similarity   | Levenshtein, Jaro-Winkler distance functions          |
| `indicatif` | Progress bars       | Batch import progress reporting (already in PRD-0001) |

### Internal Dependencies

- `docs/architecture/domain-model.md` — Entity, component, relationship types
- `docs/architecture/composition.md` — Entity component model
- `docs/architecture/adrs/adr-0006.md` — Entity resolution as critical layer

---

## Timeline

| Phase                          | Duration | Deliverables                                                             |
| ------------------------------ | -------- | ------------------------------------------------------------------------ |
| Phase 1: PDF importer          | 3 days   | ImportAdapter trait, PDF text extraction, metadata parsing               |
| Phase 2: Fuzzy resolution      | 1 week   | Resolution strategies, confidence scoring, merge thresholds, audit trail |
| Phase 3: Extended cross-refs   | 3 days   | Wikilink, URL, @mention, section anchor extraction                       |
| Phase 4: Frontmatter type      | 1 day    | Entity type inference from frontmatter                                   |
| Phase 5: URL import            | 2 days   | Remote file download and import                                          |
| Phase 6: Integration + testing | 3 days   | End-to-end tests, performance benchmarks                                 |

**Total: ~3 weeks**
