# Security Architecture

> Security is a property of the architecture, not an afterthought.

---

## Principles

1. **Defense in depth.** Multiple layers of security. No single point of failure.
2. **Least privilege.** Components access only what they need. Plugins are sandboxed.
3. **Data at rest is encrypted.** Canonical data is encrypted in storage.
4. **Data in transit is encrypted.** All network communication uses TLS.
5. **Auditability.** Every access and modification is logged.

---

## Threat Model

### In Scope

- Unauthorized access to the knowledge graph
- Data exfiltration from storage engines
- Plugin-mediated attacks
- AI model manipulation
- Import-mediated attacks (malicious documents)

### Out of Scope

- Physical security of deployment infrastructure
- Social engineering attacks
- Denial of service at the infrastructure level

---

## Authentication

### User Authentication

- Local deployment: system-level authentication (Unix, Windows)
- Network deployment: token-based authentication (JWT, API keys)
- Enterprise deployment: SSO integration (OIDC, SAML)

### Plugin Authentication

- Plugins are signed. Unsigned plugins are rejected.
- Plugin manifests declare required permissions.
- The system grants permissions based on plugin capabilities.

### AI Provider Authentication

- AI providers are authenticated through API keys.
- API keys are stored in encrypted configuration.
- API keys are never logged or exposed in error messages.

---

## Authorization

### Entity-Level Access Control

- Entities may have permission components.
- Permission rules specify who can read, write, or delete an entity.
- Permission evaluation is performed at the query layer.

### Workspace-Level Access Control

- Workspaces scope access to a set of entities.
- Users are assigned roles: viewer, editor, curator, admin.
- Role permissions are hierarchical: admin > curator > editor > viewer.

### Plugin-Level Access Control

- Plugins declare required permissions in their manifest.
- The system grants or denies permissions at plugin activation.
- Plugins cannot escalate their own permissions.

---

## Data Protection

### Encryption at Rest

- Canonical data is encrypted using AES-256.
- Encryption keys are managed through the deployment environment.
- Object storage uses server-side encryption (SSE-S3, SSE-KMS).

### Encryption in Transit

- All network communication uses TLS 1.2+.
- Internal service communication uses mTLS where supported.
- API endpoints require HTTPS.

### Data Classification

| Data Type | Classification | Encryption | Retention |
|-----------|---------------|------------|-----------|
| Canonical entities | Confidential | At rest + transit | Indefinite |
| Relationships | Confidential | At rest + transit | Indefinite |
| Artifacts (content) | Confidential | At rest + transit | Indefinite |
| Search indexes | Internal | At rest | Rebuildable |
| Embeddings | Internal | At rest | Rebuildable |
| Caches | Internal | In transit | Ephemeral |
| Logs | Internal | At rest | Configurable |
| Events | Internal | At rest | Configurable |

---

## Import Security

Imported content is untrusted. The system applies these safeguards:

1. **Sandboxed parsing.** Parsers run in isolated contexts. Malformed input cannot escape the parser.
2. **Content validation.** Imported content is validated against expected schemas.
3. **Size limits.** Import size is bounded to prevent resource exhaustion.
4. **Format validation.** File formats are validated by magic bytes, not file extensions.
5. **OCR and media processing.** Media processing runs in isolated workers with resource limits.

---

## Plugin Security

1. **Signed plugins.** Plugins are cryptographically signed. Unsigned plugins are rejected.
2. **Permission declaration.** Plugins declare required permissions. Undeclared permissions are denied.
3. **Sandboxed execution.** Plugins run in isolated contexts with limited system access.
4. **Resource limits.** Plugins have CPU, memory, and I/O limits.
5. **Audit logging.** All plugin actions are logged.

---

## Audit Logging

Every significant action is logged:

- Entity creation, modification, and archival
- Relationship creation and modification
- User authentication and authorization changes
- Plugin activation and deactivation
- AI operation invocations
- Import operations
- Export operations

Audit logs are:

- Immutable once written.
- Stored in append-only storage.
- Retained for a configurable retention period.
- Queryable for compliance and forensics.

---

## Deployment Security

### Local Deployment

- The system runs on the user's machine.
- No network exposure unless explicitly configured.
- Data stays on the user's machine.

### Private Cloud Deployment

- The system runs in a private network.
- Access is controlled through network policies and authentication.
- Data is encrypted at rest and in transit.

### Managed Service Deployment

- The system runs in a managed environment.
- Multi-tenant isolation through workspace separation.
- Data is encrypted and access-controlled.

---

## Further Reading

- [Storage](../architecture/storage.md) -- How data is persisted
- [Extensibility](../architecture/extensibility.md) -- How plugins are sandboxed
- [Governance](../philosophy/governance.md) -- How security decisions are governed
