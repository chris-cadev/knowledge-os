# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/), and this project adheres to [Semantic Versioning](https://semver.org/).

---

## [0.1.0] - 2026-07-21

### Added

- Foundational seed manifesto (`docs/foundational-manifesto.md`)
- Engineering architecture constitution (`docs/engineering-architecture.md`)
- Documentation structure following Diataxis framework
- README with architecture overview
- Contributing guidelines
- MIT License
- Architecture Decision Records framework
- Design principles documentation
- Goals and non-goals documentation
- System baseline architecture documentation
- Layered architecture deep dive
- Canonical vs derived data documentation
- Storage philosophy documentation
- Entity component model documentation
- Compiler perspective documentation
- Event-driven architecture documentation
- Project glossary

## [0.2.0] - 2026-07-21

### Added

- Vision document (`docs/philosophy/vision.md`) -- Part I of the manifesto
- Mental model document (`docs/architecture/mental-model.md`) -- Part III of the manifesto
- Domain model document (`docs/architecture/domain-model.md`) -- Part V of the manifesto
- AI architecture document (`docs/architecture/ai.md`) -- Part IX of the manifesto
- UI philosophy document (`docs/architecture/ui-philosophy.md`) -- Part VIII of the manifesto
- Engineering principles document (`docs/philosophy/engineering-principles.md`) -- Part X of the manifesto
- Extensibility document (`docs/architecture/extensibility.md`) -- Part XI of the manifesto
- Product vision document (`docs/philosophy/product-vision.md`) -- Part XII of the manifesto
- Governance document (`docs/philosophy/governance.md`) -- Part XIII of the manifesto
- Scalability document (`docs/architecture/scalability.md`)
- Synchronization document (`docs/architecture/synchronization.md`)
- Testing strategy document (`docs/engineering/testing-strategy.md`)
- Security architecture document (`docs/engineering/security.md`)
- Deployment architecture document (`docs/engineering/deployment.md`)
- Plugin development guide (`docs/guides/plugin-development.md`)
- AI agent guidelines (`docs/guides/ai-agent-guidelines.md`)
- 5 Architecture Decision Records (ADR-0001 through ADR-0005)

### Changed

- Expanded philosophy document with deeper analysis of principles, values, and anti-goals
- Expanded glossary with additional terms: Agent, Automation, Capability, Knowledge Graph, Metadata, Resource, Synchronization
- Updated ADR index with accepted status for all 5 ADRs
- Updated documentation README with complete reading order and new directory structure

## [0.3.0] - 2026-07-21

### Added

- Architectural principles document (`docs/architecture/architectural-principles.md`) -- Part VI consolidated invariants
- Appendices (`docs/appendices.md`) -- Part XV with diagrams, patterns, examples, model tables
- Expanded glossary (`docs/reference/glossary.md`) -- Part XIV canonical vocabulary with ~30 terms
- Expanded vision (`docs/philosophy/vision.md`) -- Part I deepened with concrete examples
- Expanded philosophy (`docs/philosophy/philosophy.md`) -- Part II deepened with implications and anti-goals
- Engineering handbook (`docs/engineering/engineering-handbook.md`) -- Git workflow, code review, CI/CD, debugging
- Operational runbooks (`docs/engineering/operational-runbooks.md`) -- 8 incident response procedures
- Product requirements (`docs/engineering/product-requirements.md`) -- Year 1 scope, FR/NFR, user stories
- UI design system (`docs/engineering/ui-design-system.md`) -- Design tokens, component specs, accessibility
- Tutorial: First Import (`docs/guides/tutorials/first-import.md`) -- Step-by-step walkthrough
- Tutorial: Build a Custom Importer (`docs/guides/tutorials/build-custom-importer.md`) -- Plugin development walkthrough
- API specification (`docs/engineering/api-specification.md`) -- REST and MCP API surfaces
- Infrastructure handbook (`docs/engineering/infrastructure-handbook.md`) -- Provisioning, scaling, monitoring, CI/CD

### Changed

- Converted all ASCII diagrams to Mermaid across 12+ architecture and engineering documents for consistent rendering
- Fixed Diataxis classification table in `docs/README.md` to accurately map each file to its actual content type instead of grouping all `engineering/` as How-to
- Expanded `docs/architecture/pipeline.md` with dedicated Indexing, Embedding, and Search subsections in Layer 6, plus Synchronization cross-reference
- Renamed "Core Belief" heading to "Core Philosophy" in `docs/philosophy/philosophy.md` for manifesto consistency
- Updated `docs/README.md` with new documents and tutorial reading order
- Updated root `README.md` with new status items and appendix documentation section
- Updated `AGENTS.md` with expanded repository structure
