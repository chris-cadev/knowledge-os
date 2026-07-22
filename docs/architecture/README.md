# Architecture

[Home](../../README.md) > [Documentation](../README.md) > Architecture

How the system is designed and why. These documents specify the canonical technical model that every implementation follows.

---

## Documents

### Core Architecture

| Document | Purpose |
|----------|---------|
| [overview.md](overview.md) | Technical architecture overview -- positions Knowledge OS as a deterministic knowledge compiler with the seven-layer pipeline |
| [mental-model.md](mental-model.md) | The canonical way of thinking -- replaces the filesystem mental model with entities, components, relationships, projections, and the knowledge graph |
| [domain-model.md](domain-model.md) | Entity, relationship, and component type definitions -- the authoritative reference for all domain types |
| [pipeline.md](pipeline.md) | The seven-layer pipeline -- detailed specification of each layer's responsibilities, rules, and contracts |
| [data-model.md](data-model.md) | Canonical vs derived data -- the fundamental distinction between what cannot be recreated and what is disposable |

### Subsystems

| Document | Purpose |
|----------|---------|
| [storage.md](storage.md) | Polyglot persistence -- six storage engine categories with purposes, properties, and candidate implementations |
| [composition.md](composition.md) | Entity component model -- why inheritance is avoided in favor of composable, flat component assemblies |
| [compilation.md](compilation.md) | Knowledge as compilation -- how pipeline stages map to compiler stages |
| [events.md](events.md) | Event-driven architecture -- canonical and derivation events, structure, and processing rules |
| [ai.md](ai.md) | AI as a system component -- principles, context construction, ranking, and operation patterns |
| [ui-philosophy.md](ui-philosophy.md) | User interface philosophy -- views as disposable projections, entity-centric navigation |
| [extensibility.md](extensibility.md) | Plugin system and extension points -- plugin lifecycle, manifest format, and adapter types |
| [scalability.md](scalability.md) | Scaling strategies -- data volume, throughput, and concurrency across all layers |
| [synchronization.md](synchronization.md) | Consistency and derived data updates -- eventual consistency, event propagation, and conflict resolution |
| [architectural-principles.md](architectural-principles.md) | Consolidated architectural invariants -- technology-independent principles governing every design decision |

### Decision Records

| Document | Purpose |
|----------|---------|
| [adrs/](adrs/) | Architecture Decision Records -- accepted decisions that shape the system (5 accepted) |

---

## Reading Order

Start with [overview.md](overview.md) and [mental-model.md](mental-model.md). Then read [domain-model.md](domain-model.md) and [pipeline.md](pipeline.md) for the technical foundation. The subsystem documents can be read in any order based on interest.
