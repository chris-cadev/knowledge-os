# Goals and Non-Goals

This document defines the boundaries of Knowledge OS. It clarifies what the system is designed to do and what it intentionally does not do.

---

## Goals

### G1. Unified Knowledge Model

Build a single canonical representation for all knowledge entities -- concepts, people, organizations, papers, code, decisions, events, and ideas. No distinction exists between documents and objects. Everything is an entity.

### G2. Storage-Agnostic Architecture

Decouple the knowledge model from any specific storage technology. The system supports relational, graph, search, vector, object, and cache storage engines. Any engine may be replaced without affecting the domain model.

### G3. Deterministic Knowledge Pipeline

Process all knowledge through a deterministic pipeline: import, parse, normalize, store canonically, derive projections, render views. The same input always produces the same canonical output.

### G4. First-Class Relationships

Represent relationships as first-class citizens with their own metadata, versioning, and queryability. Relationships are not foreign keys. They are typed, directed, attributed edges in a knowledge graph.

### G5. Disposable Derived Data

Ensure that all derived data -- search indexes, embeddings, recommendations, caches -- can be regenerated from canonical sources at any time. No derived artifact becomes authoritative.

### G6. AI as Infrastructure

Integrate AI as a component of the pipeline, not as the system itself. AI performs extraction, classification, summarization, and inference. Every AI output is reviewable, versioned, and replaceable.

### G7. Plugin Extensibility

Support extension through plugins for importers, exporters, views, storage adapters, relationship providers, search providers, embedding providers, AI providers, and automation agents.

### G8. Multiple Projections

Render the same knowledge through multiple interfaces: tree view, graph view, timeline, table, calendar, kanban, gallery, mind map, conversation, dashboard, and learning path.

---

## Non-Goals

### NG1. We Are Not a Database

We do not build a general-purpose database. We build a knowledge engine that uses databases as storage adapters. If you need a database, use PostgreSQL, SQLite, or DuckDB directly.

### NG2. We Are Not an AI Product

We do not build a chatbot, a writing assistant, or a code generator. AI is a pipeline component that assists in knowledge construction. The system's value is the knowledge model, not the AI model.

### NG3. We Are Not a Note-Taking App

We do not build a competitor to Notion, Obsidian, or Apple Notes. Those are document-centric tools. Knowledge OS is entity-centric. Documents are inputs. Entities are the output.

### NG4. We Are Not a Search Engine

We do not build a search product. Search is one projection of canonical data. We support full-text search, semantic search, and graph traversal as derived capabilities, not as the primary interface.

### NG5. We Are Not a Collaboration Platform

We do not build real-time collaborative editing, chat, or video conferencing. Knowledge OS is a knowledge engine. Collaboration tools may consume its API, but collaboration is not our domain.

### NG6. We Are Not a BI Tool

We do not build dashboards, reports, or analytics. Views are projections of canonical data. If a dashboard is needed, it consumes the knowledge API. The dashboard is a plugin, not the product.

### NG7. We Are Not a Workflow Engine

We do not build task management, project management, or process automation. Automation agents may interact with the knowledge model, but workflow orchestration is not our responsibility.

### NG8. We Are Not a Cloud Service

We do not build a SaaS product. Knowledge OS is an engine that can be deployed anywhere -- local machine, private cloud, or public cloud. We do not operate infrastructure for users.

---

## Decision Framework

When evaluating whether a proposed feature belongs in Knowledge OS, apply this test:

1. **Does it serve the canonical knowledge model?** If yes, it may belong.
2. **Does it introduce a storage dependency?** If yes, it must go through an adapter.
3. **Does it make AI the source of truth?** If yes, it is redesigned.
4. **Is it a projection of existing data?** If yes, it may be a plugin.
5. **Does it belong in a database, not a knowledge engine?** If yes, it is out of scope.
