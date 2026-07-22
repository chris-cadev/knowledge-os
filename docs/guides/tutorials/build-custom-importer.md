# Tutorial: Build a Custom Importer

> This tutorial walks through building a custom importer plugin that parses a fictional `.knowledge` format and emits canonical entities.

---

## Prerequisites

- Rust toolchain installed (`rustup`)
- Knowledge OS SDK (`knowledge-os-sdk`)
- Familiarity with Rust traits and error handling
- Understanding of the [Plugin Development Guide](../plugin-development.md)

---

## The Format

The `.knowledge` file format is a fictional structured format used in this tutorial:

```
% @title: Quantum Computing Overview
% @author: Alice Johnson
% @tags: quantum-computing, physics, computing
% @date: 2026-03-01

Quantum computing leverages quantum mechanical phenomena such as
superposition and entanglement to process information.

## Qubits

A qubit is the basic unit of quantum information. Unlike classical
bits, qubits can exist in superposition.

## Applications

- Cryptography
- Drug discovery
- Optimization problems

% @reference: Nielsen & Chuang, "Quantum Computation and Quantum Information"
```

The format uses `% @key: value` lines for metadata and standard Markdown for body content.

---

## Step 1: Scaffold the Plugin

Create a new Rust library:

```bash
cargo new knowledge-os-importer-knowledge --lib
cd knowledge-os-importer-knowledge
```

Add dependencies to `Cargo.toml`:

```toml
[package]
name = "knowledge-os-importer-knowledge"
version = "0.1.0"
edition = "2021"

[lib]
crate-type = ["cdylib"]

[dependencies]
knowledge-os-sdk = "0.1"
serde = { version = "1", features = ["derive"] }
thiserror = "1"
```

---

## Step 2: Define the Manifest

Create `knowledge-plugin.toml` in the project root:

```toml
[plugin]
name = "knowledge-importer"
version = "0.1.0"
description = "Import .knowledge files as knowledge entities"
author = "Your Name"
license = "MIT"

[plugin.capabilities]
importers = ["knowledge"]

[plugin.dependencies]
knowledge-os = ">=0.1.0"

[plugin.permissions]
files = ["read"]
network = []
```

---

## Step 3: Implement the Parser

The parser converts raw `.knowledge` content into an intermediate structure. It performs no semantic judgment -- it only extracts structure.

Create `src/parser.rs`:

```rust
use serde::{Deserialize, Serialize};

#[derive(Debug, Serialize, Deserialize)]
pub struct ParsedKnowledge {
    pub metadata: Metadata,
    pub body: String,
    pub references: Vec<String>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct Metadata {
    pub title: String,
    pub author: String,
    pub tags: Vec<String>,
    pub date: Option<String>,
}

#[derive(Debug, thiserror::enum)]
pub enum ParseError {
    MissingField(&'static str),
    InvalidFormat(String),
}

pub fn parse(content: &str) -> Result<ParsedKnowledge, ParseError> {
    let mut metadata = Vec::new();
    let mut body_lines = Vec::new();
    let mut references = Vec::new();

    for line in content.lines() {
        if let Some(rest) = line.strip_prefix("% @") {
            if let Some((key, value)) = rest.split_once(':') {
                metadata.push((key.trim().to_string(), value.trim().to_string()));
            }
        } else if line.starts_with("% @reference:") {
            let reference = line
                .strip_prefix("% @reference:")
                .unwrap()
                .trim()
                .to_string();
            references.push(reference);
        } else {
            body_lines.push(line);
        }
    }

    let get = |key: &str| -> Result<String, ParseError> {
        metadata
            .iter()
            .find(|(k, _)| k == key)
            .map(|(_, v)| v.clone())
            .ok_or(ParseError::MissingField(key))
    };

    Ok(ParsedKnowledge {
        metadata: Metadata {
            title: get("title")?,
            author: get("author")?,
            tags: get("tags")?
                .split(',')
                .map(|s| s.trim().to_string())
                .collect(),
            date: metadata
                .iter()
                .find(|(k, _)| k == "date")
                .map(|(_, v)| v.clone()),
        },
        body: body_lines.join("\n"),
        references,
    })
}
```

This parser is pure. It takes a string and returns a structured result. No I/O, no side effects.

---

## Step 4: Implement the Import Adapter

The adapter bridges the parser and the Knowledge OS SDK. It implements the `ImportAdapter` trait.

Create `src/lib.rs`:

```rust
mod parser;

use knowledge_os::plugin::{
    ExternalIdentifier, ImportAdapter, ImportSource, ImportedEntity, Relationship,
};
use parser::{parse, ParsedKnowledge};

pub struct KnowledgeImporter;

impl ImportAdapter for KnowledgeImporter {
    fn can_import(&self, source: &ImportSource) -> bool {
        source.path.ends_with(".knowledge")
    }

    fn import(&self, source: &ImportSource) -> Result<Vec<ImportedEntity>, ImportError> {
        let content = std::fs::read_to_string(&source.path)
            .map_err(|e| ImportError::Io(e.to_string()))?;

        let parsed = parse(&content)
            .map_err(|e| ImportError::Parse(e.to_string()))?;

        let mut entities = Vec::new();
        let mut relationships = Vec::new();

        // Create the main article entity
        let article = ImportedEntity {
            entity_type: "Article".into(),
            components: vec![
                ("Title".into(), serde_json::json!({ "name": parsed.metadata.title })),
                ("Content".into(), serde_json::json!({ "markdown": parsed.body })),
                ("Tags".into(), serde_json::json!({ "values": parsed.metadata.tags })),
                ("Language".into(), serde_json::json!({ "code": "en" })),
            ],
            relationships: Vec::new(),
            external_ids: vec![ExternalIdentifier {
                system: "file".into(),
                id: source.path.clone(),
            }],
        };
        entities.push(article);

        // Create the author entity
        let author = ImportedEntity {
            entity_type: "Person".into(),
            components: vec![
                ("Title".into(), serde_json::json!({ "name": parsed.metadata.author })),
            ],
            relationships: Vec::new(),
            external_ids: Vec::new(),
        };

        // Create authored_by relationship
        relationships.push(Relationship {
            rel_type: "authored_by".into(),
            source_index: 0,
            target_index: 1,
        });

        entities.push(author);

        // Create reference entities
        for reference in &parsed.references {
            let ref_entity = ImportedEntity {
                entity_type: "Note".into(),
                components: vec![
                    ("Content".into(), serde_json::json!({ "markdown": reference.clone() })),
                ],
                relationships: Vec::new(),
                external_ids: Vec::new(),
            };

            relationships.push(Relationship {
                rel_type: "references".into(),
                source_index: 0,
                target_index: entities.len(),
            });

            entities.push(ref_entity);
        }

        // Attach relationships to the article
        entities[0].relationships = relationships;

        Ok(entities)
    }

    fn supported_types(&self) -> &[&str] {
        &["knowledge"]
    }
}

#[derive(Debug, thiserror::Error)]
pub enum ImportError {
    #[error("IO error: {0}")]
    Io(String),

    #[error("Parse error: {0}")]
    Parse(String),
}
```

---

## Step 5: Register the Plugin

At the bottom of `src/lib.rs`, add the registration entry point:

```rust
use knowledge_os::plugin::register;

#[no_mangle]
pub extern "C" fn knowledge_os_plugin_init() {
    register(KnowledgeImporter);
}
```

---

## Step 6: Write Tests

Create `tests/import_test.rs`:

```rust
use knowledge_os_importer_knowledge::KnowledgeImporter;
use knowledge_os::plugin::{ImportSource, ImportAdapter};

#[test]
fn test_can_import() {
    let importer = KnowledgeImporter;
    let source = ImportSource {
        path: "test.knowledge".into(),
    };
    assert!(importer.can_import(&source));
}

#[test]
fn test_cannot_import_wrong_extension() {
    let importer = KnowledgeImporter;
    let source = ImportSource {
        path: "test.md".into(),
    };
    assert!(!importer.can_import(&source));
}

#[test]
fn test_import() {
    let importer = KnowledgeImporter;
    let source = ImportSource {
        path: "fixtures/test.knowledge".into(),
    };
    let entities = importer.import(&source).unwrap();
    assert_eq!(entities.len(), 4); // article + author + 2 references
    assert_eq!(entities[0].entity_type, "Article");
    assert_eq!(entities[1].entity_type, "Person");
}
```

Create `fixtures/test.knowledge`:

```
% @title: Test Document
% @author: Test Author
% @tags: test, example
% @date: 2026-01-01

This is a test document.

% @reference: Reference One
% @reference: Reference Two
```

Run the tests:

```bash
cargo test
```

---

## Step 7: Build and Install

Build the plugin:

```bash
cargo build --release
```

Install it:

```bash
knowledge-os plugin install ./target/release/libknowledge_os_importer_knowledge.so
```

Verify the installation:

```bash
knowledge-os plugin list
```

Output:

```
Installed plugins:
  knowledge-importer v0.1.0 (importer: knowledge)
```

---

## Step 8: Import a .knowledge File

Create a test file `example.knowledge`:

```
% @title: Learning Rust
% @author: Chris
% @tags: rust, programming, systems
% @date: 2026-04-01

Rust is a systems programming language focused on safety, speed,
and concurrency.

## Ownership

Rust's ownership system ensures memory safety without a garbage collector.

% @reference: "The Rust Programming Language" by Steve Klabnik
```

Import it:

```bash
knowledge-os import example.knowledge
```

Output:

```
Importing example.knowledge...
  Format detected: knowledge (knowledge-importer)
  Entity created: Article (id: ent_02...)
  Components attached: Title, Content, Tags, Language, Timeline
  Relationships created: authored_by, references
  Import complete.
```

---

## What You Learned

- The `.knowledge` format is parsed in two stages: raw parsing (syntactic structure) and adapter processing (semantic structure).
- The parser is pure -- no I/O, no side effects.
- The adapter implements the `ImportAdapter` trait and produces `ImportedEntity` values.
- Entities and relationships are created declaratively.
- The plugin is registered, built, installed, and invoked through the CLI.
- All derived data (search indexes, embeddings) regenerates automatically after import.

---

## Next Steps

- [Plugin Development Guide](../plugin-development.md) -- Full reference for all plugin types.
- [Extensibility](../../architecture/extensibility.md) -- Plugin system architecture.
- [Storage](../../architecture/storage.md) -- Where imported data is persisted.
- [Events](../../architecture/events.md) -- How import events propagate through the system.

---

## Further Reading

- [Pipeline](../../architecture/pipeline.md) -- The seven-layer architecture
- [Data Model](../../architecture/data-model.md) -- Canonical vs derived data
- [Composition](../../architecture/composition.md) -- Entity component model
- [Domain Model](../../architecture/domain-model.md) -- Entity, relationship, and component types
