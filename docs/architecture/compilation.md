# Compilation

> The Knowledge Operating System is engineered as a deterministic knowledge compiler.

---

## The Analogy

A compiler transforms source code into executable artifacts through a series of well-defined stages. Each stage has one responsibility. Each stage communicates through explicit intermediate representations. The pipeline is deterministic: the same source always produces the same output.

Knowledge OS applies the same architecture to knowledge. Information enters as raw source material. It passes through import, parsing, normalization, and canonical representation. It is then compiled into derived artifacts optimized for specific access patterns: search indexes, embeddings, graph projections, and rendered views.

---

## Compiler Stages

| Compiler                    | Knowledge OS                                      |
| --------------------------- | ------------------------------------------------- |
| Source code                 | External resources (documents, APIs, media)       |
| Lexical analysis            | Import layer (format detection, extraction)       |
| Parsing                     | Parsing layer (structural analysis)               |
| Semantic analysis           | Normalization layer (entity resolution, identity) |
| Intermediate representation | Canonical entity model                            |
| Optimization                | Derivation layer (index generation, embedding)    |
| Code generation             | Presentation layer (view rendering)               |
| Executable                  | Rendered knowledge projection                     |

---

## Properties

### Deterministic

The same input always produces the same canonical output. There is no randomness, no non-deterministic side effects, and no hidden state that affects the pipeline.

This property enables:

- **Reproducibility.** Re-running the pipeline on the same data produces identical results.
- **Testing.** Pipeline stages can be tested with fixed inputs and expected outputs.
- **Debugging.** Failures can be reproduced by re-running the pipeline.
- **Recovery.** Derived data can be rebuilt by re-running the pipeline from canonical data.

### Composable

Each stage has a single responsibility. Stages are composed into a pipeline. The output of one stage is the input of the next.

```
Stage N  -->  [Intermediate]  -->  Stage N+1
```

This enables:

- **Independent replacement.** Any stage may be replaced without affecting others.
- **Per-stage testing.** Each stage can be tested in isolation.
- **Pipeline extension.** New stages can be inserted between existing ones.

### Incremental

The pipeline supports incremental processing. When canonical data changes, only the affected derived artifacts are rebuilt.

```
Canonical change
       |
  Event emitted
       |
  Affected derivation stages triggered
       |
  Derived artifacts updated
```

This avoids full rebuilds and enables near-real-time updates to derived data.

---

## Source Code as Metaphor

In compiler theory, source code is the human-readable representation of intent. The compiler transforms it into machine-executable form. The source code is the source of truth. Object files are derived.

In Knowledge OS, canonical knowledge is the human-curated representation of understanding. The pipeline transforms it into machine-optimized forms. The canonical model is the source of truth. Search indexes, embeddings, and graph projections are derived.

```
Compiler:                    Knowledge OS:

Source code                  Canonical knowledge
    |                            |
  Lexer                      Importer
    |                            |
  Parser                     Parser
    |                            |
  Semantic analyzer           Normalizer
    |                            |
  IR (intermediate repr.)     Canonical entity model
    |                            |
  Optimizer                  Derivation layer
    |                            |
  Code generator             Presentation layer
    |                            |
  Executable                 Rendered view
```

---

## Implications

### Derived artifacts are disposable

Just as object files can be deleted and recompiled, derived artifacts can be discarded and rebuilt. This is not a failure mode. It is a design property.

### The canonical model is the source of truth

Just as source code is the source of truth in compilation, the canonical knowledge model is the source of truth in Knowledge OS. If you lose the search index, rebuild it. If you lose the embedding store, recompute it. The canonical model is what you protect.

### The pipeline is the architecture

The pipeline is not an implementation detail. It is the architecture. Every feature fits into the pipeline. Every component has a stage. Every stage has a responsibility.

---

## Further Reading

- [Overview](overview.md) -- System-level architecture
- [Pipeline](pipeline.md) -- The seven-layer pipeline in detail
- [Data Model](data-model.md) -- Canonical vs derived data
- [Events](events.md) -- How events drive incremental compilation
