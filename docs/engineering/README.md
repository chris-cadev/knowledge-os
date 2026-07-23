# Engineering

[Home](../../README.md) > [Documentation](../README.md) > Engineering

Testing, security, deployment, and practices. These documents define how the system is built, tested, operated, and shipped.

---

## Documents

### Development Practices

| Document                                           | Purpose                                                                                                                   |
| -------------------------------------------------- | ------------------------------------------------------------------------------------------------------------------------- |
| [testing-strategy.md](testing-strategy.md)         | Test philosophy and pipeline testing -- test pyramid, canonical tests, derivation tests, event tests, and plugin tests    |
| [engineering-handbook.md](engineering-handbook.md) | Git workflow, code review, CI/CD, debugging -- branch conventions, commit format, PR process, and local development setup |
| [product-requirements.md](product-requirements.md) | Product scope and functional/non-functional requirements -- Year 1 scope with priority and acceptance criteria            |

### Operations

| Document                                                 | Purpose                                                                                                             |
| -------------------------------------------------------- | ------------------------------------------------------------------------------------------------------------------- |
| [security.md](security.md)                               | Threat model and access control -- defense-in-depth, threat categories, authentication, and plugin signing          |
| [deployment.md](deployment.md)                           | Deployment models and configuration -- local, private cloud, and public cloud with configuration management         |
| [operational-runbooks.md](operational-runbooks.md)       | Operational procedures and incident response -- rebuild, migration, backup, health checks, and failure scenarios    |
| [infrastructure-handbook.md](infrastructure-handbook.md) | Provisioning, scaling, monitoring, CI/CD -- infrastructure specs, autoscaling, observability, and disaster recovery |

### Interfaces

| Document                                     | Purpose                                                                                                              |
| -------------------------------------------- | -------------------------------------------------------------------------------------------------------------------- |
| [api-specification.md](api-specification.md) | REST and MCP API surfaces -- entity and relationship CRUD, search, rendering, and AI agent integration               |
| [ui-design-system.md](ui-design-system.md)   | Design tokens, component specs, accessibility -- colors, typography, spacing, components, and WCAG 2.1 AA compliance |

---

## Note

The `engineering/` directory contains mixed Diataxis types. Each file's type is determined by its content, not its location. Some files are reference specifications, others are how-to guides, and others are explanations.
