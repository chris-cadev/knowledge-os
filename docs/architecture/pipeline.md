# Pipeline

> The system is organized into independent layers. Each layer has one responsibility. Each layer communicates through explicit contracts.

---

## Overview

The Knowledge OS processes information through a seven-layer pipeline. This structure is inspired by compiler architecture, where source code passes through lexical analysis, parsing, semantic analysis, optimization, and code generation -- each stage with a single responsibility, communicating through well-defined intermediate representations.

In Knowledge OS, information enters as raw external data and exits as rendered knowledge projections. Between these endpoints, each layer transforms the data in a specific, isolated, deterministic way.

---

## The Seven Layers

```
  Layer 1  Import Layer          Receive information from external systems
  Layer 2  Parsing Layer         Extract structured information
  Layer 3  Normalization Layer   Convert to canonical representations
  Layer 4  Knowledge Model       Entity storage and lifecycle
  Layer 5  Relationship Engine   Connect entities through typed edges
  Layer 6  Derivation Layer      Generate indexes, embeddings, recommendations
  Layer 7  Presentation Layer    Render projections for human and machine interfaces
```

---

## Layer 1 -- Import Layer

**Responsibility:** Receive information from external systems.

The import layer is the system's boundary with the outside world. It accepts information in any format and transforms it into an internal representation that the parsing layer can consume.

**Supported sources:**

- Documents: Markdown, PDF, Word, HTML
- Code: Git repositories, source files
- Media: YouTube, podcasts, audio, images
- Feeds: RSS, email
- Structured: APIs, databases
- Visual: OCR, screen captures

**Rules:**

- The import layer never performs business logic.
- It only transforms external formats into internal representations.
- Each importer is a plugin. New formats are added without modifying the core.

---

## Layer 2 -- Parsing Layer

**Responsibility:** Extract structured information from imported data.

The parser produces normalized intermediate structures from raw input. It does not interpret meaning. It extracts structure.

**Operations:**

- Parsing (syntactic structure extraction)
- Metadata extraction (author, date, source, format)
- Structural analysis (headings, sections, code blocks)
- Language detection
- OCR (optical character recognition for images)
- Audio transcription (speech-to-text for audio/video)
- Content segmentation (breaking content into meaningful chunks)

**Rules:**

- The parser produces deterministic outputs.
- The parser does not make semantic judgments.
- Parsing errors are logged, not silently swallowed.

---

## Layer 3 -- Normalization Layer

**Responsibility:** Convert parsed information into canonical representations.

Normalization is where raw structure becomes meaningful knowledge. This layer identifies entities, resolves duplicates, assigns canonical identifiers, and normalizes metadata.

**Operations:**

- Entity identification (recognizing that "Dr. Smith" and "John Smith" are the same person)
- Duplicate detection (finding that two references describe the same paper)
- Identity resolution (merging duplicate entities under a single canonical identity)
- Canonical identifier assignment (generating stable, unique IDs)
- Metadata normalization (standardizing dates, names, taxonomies)
- Content normalization (consistent formatting, encoding, structure)

**Rules:**

- Normalization produces deterministic outputs.
- Entity resolution is auditable -- every merge decision is recorded.
- Canonical identifiers are immutable once assigned.

---

## Layer 4 -- Knowledge Model

**Responsibility:** Store and manage canonical entities, their components, and their lifecycle.

This layer is the heart of the system. Everything becomes a first-class entity. There is no distinction between documents and objects.

**Entity types include:**

Concept, Person, Organization, Project, Book, Research Paper, Video, Article, Tool, Technology, Question, Idea, Event, Skill, Location, Dataset, Collection, Workspace.

**Rules:**

- The knowledge model is the canonical source of truth.
- Every entity is composed of components (see [Composition](composition.md)).
- Entities are versioned and auditable.
- No storage engine defines the entity model.

---

## Layer 5 -- Relationship Engine

**Responsibility:** Connect entities through typed, versioned, queryable relationships.

Relationships are first-class citizens. They are not foreign keys. They are typed, directed, attributed edges with their own metadata.

**Relationship types include:**

`created_by`, `references`, `implements`, `depends_on`, `contradicts`, `contains`, `extends`, `belongs_to`, `teaches`, `requires`, `related_to`, `inspired_by`.

**Rules:**

- Every relationship has metadata (confidence, source, timestamp).
- Relationships are versioned.
- Relationships are queryable through graph traversal.
- Relationship extraction may be AI-assisted, but all outputs are reviewable.

---

## Layer 6 -- Derivation Layer

**Responsibility:** Generate derived representations from canonical data.

Everything generated by computation belongs in this layer. Derived data optimizes specific access patterns but never becomes authoritative.

**Derived artifacts include:**

- Search indexes (full-text retrieval)
- Embeddings (semantic similarity)
- Recommendations (suggested connections)
- Similarity graphs (computed proximity)
- Learning paths (ordered sequences)
- Relationship inference (AI-suggested connections)
- Knowledge summaries (condensed representations)
- AI context (retrieval-augmented generation payloads)
- Caches (performance optimization)

**Rules:**

- This layer contains no canonical information.
- Everything is disposable.
- All derived artifacts may be regenerated from canonical data.
- Derived data is never the source of truth.

---

## Layer 7 -- Presentation Layer

**Responsibility:** Render projections of knowledge for human and machine interfaces.

Views never own information. Views render knowledge. Every entity may appear in multiple projections simultaneously.

**View types include:**

- Tree View (hierarchical navigation)
- Graph View (relationship exploration)
- Timeline (temporal ordering)
- Table (structured comparison)
- Calendar (date-based organization)
- Kanban (status-based workflow)
- Gallery (visual browsing)
- Mind Map (conceptual mapping)
- Conversation (dialogue-based interaction)
- Dashboard (aggregated overview)
- Learning Path (ordered progression)

**Rules:**

- Every interface is a projection.
- No interface owns data.
- Views remain synchronized because they render canonical data.
- New view types are added as plugins.

---

## Layer Communication

Layers communicate through explicit contracts. A layer does not access the internal state of another layer. It receives input through a defined interface and produces output through a defined interface.

```
Layer N  --[Contract]-->  Layer N+1
```

This isolation means:

- Any layer may be independently replaced.
- No layer bypasses another.
- Testing is compositional -- each layer can be tested in isolation.

---

## Pipeline Properties

**Deterministic.** The same input always produces the same canonical output.

**Idempotent.** Processing the same input twice produces the same result.

**Auditable.** Every transformation is recorded. Every entity change is versioned.

**Extensible.** New importers, parsers, views, and storage engines are added as plugins.

**Independent.** No layer depends on a specific technology. Adapters isolate implementation details.

---

## Further Reading

- [Overview](overview.md) -- System-level architecture
- [Compilation](compilation.md) -- The compiler analogy in depth
- [Events](events.md) -- How events drive the pipeline
- [Data Model](data-model.md) -- Canonical vs derived data
