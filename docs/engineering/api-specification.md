# API Specification

> The API is the contract between the core system and its consumers. Every operation is explicit. Every interface is typed.

---

## Overview

Knowledge OS exposes a RESTful API for programmatic access and an MCP (Model Context Protocol) interface for AI agent integration. The API operates on the canonical knowledge model: entities, relationships, components, and derived projections.

All API operations are idempotent. Retrying a request produces the same result. All API responses include entity versioning information. All API errors are structured and actionable.

---

## Base URL

```
https://{host}:{port}/v1
```

All endpoints are versioned under `/v1`. Breaking changes increment the major version. Non-breaking additions increment the minor version.

---

## Authentication

### API Key

```
Authorization: Bearer {api_key}
```

### OAuth 2.0 (Managed Service)

```
Authorization: Bearer {access_token}
```

### Local Deployment

No authentication required for local single-user deployments. Authentication is enforced for multi-user and managed service deployments.

---

## Entity Endpoints

### Create Entity

```
POST /v1/entities
```

**Request:**

```json
{
  "type": "Article",
  "components": [
    { "type": "Title", "payload": { "name": "On Knowledge Management" } },
    { "type": "Content", "payload": { "markdown": "..." } },
    { "type": "Tags", "payload": { "values": ["knowledge", "productivity"] } }
  ],
  "relationships": [
    { "type": "authored_by", "target_id": "per_01HXYZ..." }
  ]
}
```

**Response:**

```json
{
  "id": "ent_01HXYZ...",
  "type": "Article",
  "version": 1,
  "created_at": "2026-07-21T10:00:00Z",
  "components": [...],
  "relationships": [...]
}
```

**Rules:**

- Every entity has exactly one type.
- Components are optional. An entity may have zero or more components.
- No duplicate component types per entity.
- Relationships reference existing entity IDs.

### Get Entity

```
GET /v1/entities/{id}?components=true&relationships=true
```

**Query Parameters:**

| Parameter | Type | Default | Description |
|-----------|------|---------|-------------|
| `components` | bool | `true` | Include component data |
| `relationships` | bool | `true` | Include relationship data |
| `version` | int | latest | Retrieve a specific version |

**Response:**

```json
{
  "id": "ent_01HXYZ...",
  "type": "Article",
  "version": 3,
  "created_at": "2026-07-21T10:00:00Z",
  "modified_at": "2026-07-21T14:30:00Z",
  "archived": false,
  "components": [...],
  "relationships": [...]
}
```

### Update Entity

```
PATCH /v1/entities/{id}
```

**Request:**

```json
{
  "components": [
    { "type": "Title", "payload": { "name": "Updated Title" } }
  ]
}
```

**Rules:**

- Partial updates replace only specified components.
- Unspecified components remain unchanged.
- Version number increments on each update.
- Conflicting updates (concurrent modification) return `409 Conflict`.

### Archive Entity

```
DELETE /v1/entities/{id}
```

**Rules:**

- Archiving is soft-delete. The entity is marked as archived, not removed.
- Archived entities are excluded from default queries.
- Archived entities may be restored via `PATCH /v1/entities/{id}` with `{ "archived": false }`.
- All relationships involving the archived entity are marked inactive.

### List Entities

```
GET /v1/entities?type=Article&tag=knowledge&archived=false&limit=50&offset=0
```

**Query Parameters:**

| Parameter | Type | Default | Description |
|-----------|------|---------|-------------|
| `type` | string | all | Filter by entity type |
| `tag` | string | all | Filter by tag value |
| `archived` | bool | `false` | Include archived entities |
| `limit` | int | 50 | Maximum results per page |
| `offset` | int | 0 | Pagination offset |

**Response:**

```json
{
  "entities": [...],
  "total": 142,
  "limit": 50,
  "offset": 0
}
```

### Version History

```
GET /v1/entities/{id}/versions
```

**Response:**

```json
{
  "versions": [
    { "version": 3, "timestamp": "2026-07-21T14:30:00Z", "changes": ["ComponentUpdated:Title"] },
    { "version": 2, "timestamp": "2026-07-21T12:00:00Z", "changes": ["ComponentAdded:Tags"] },
    { "version": 1, "timestamp": "2026-07-21T10:00:00Z", "changes": ["EntityCreated"] }
  ]
}
```

---

## Component Endpoints

### Add Component

```
POST /v1/entities/{id}/components
```

**Request:**

```json
{
  "type": "Rating",
  "payload": { "score": 4.5, "scale": 5.0 }
}
```

**Rules:**

- A component type may appear at most once per entity.
- Adding a component of a type that already exists returns `409 Conflict`.

### Update Component

```
PATCH /v1/entities/{id}/components/{type}
```

**Request:**

```json
{
  "payload": { "score": 5.0, "scale": 5.0 }
}
```

### Remove Component

```
DELETE /v1/entities/{id}/components/{type}
```

**Rules:**

- Core components (Title, Content, Timeline) may be removed.
- Component removal increments the entity version.

---

## Relationship Endpoints

### Create Relationship

```
POST /v1/relationships
```

**Request:**

```json
{
  "source_id": "ent_01HXYZ...",
  "target_id": "ent_02ABC...",
  "type": "references",
  "metadata": {
    "confidence": 0.95,
    "source": "ai-extraction"
  }
}
```

**Response:**

```json
{
  "id": "rel_01...",
  "source_id": "ent_01HXYZ...",
  "target_id": "ent_02ABC...",
  "type": "references",
  "version": 1,
  "created_at": "2026-07-21T10:00:00Z",
  "metadata": {...}
}
```

### Update Relationship

```
PATCH /v1/relationships/{id}
```

### Archive Relationship

```
DELETE /v1/relationships/{id}
```

### Query Relationships

```
GET /v1/relationships?source_id={id}&type=references&direction=outgoing
```

**Query Parameters:**

| Parameter | Type | Description |
|-----------|------|-------------|
| `source_id` | uuid | Filter by source entity |
| `target_id` | uuid | Filter by target entity |
| `type` | string | Filter by relationship type |
| `direction` | string | `outgoing`, `incoming`, or `both` |

---

## Search Endpoints

### Full-Text Search

```
GET /v1/search?q=machine+learning&type=Article&limit=20
```

**Query Parameters:**

| Parameter | Type | Description |
|-----------|------|-------------|
| `q` | string | Search query |
| `type` | string | Filter by entity type |
| `tag` | string | Filter by tag |
| `limit` | int | Maximum results |

**Response:**

```json
{
  "results": [
    {
      "entity_id": "ent_01HXYZ...",
      "score": 0.89,
      "highlights": ["...<em>machine learning</em>..."],
      "entity": {...}
    }
  ],
  "total": 23,
  "query_time_ms": 12
}
```

### Semantic Search

```
POST /v1/search/semantic
```

**Request:**

```json
{
  "query": "papers about attention mechanisms in neural networks",
  "limit": 10,
  "threshold": 0.7
}
```

**Response:**

```json
{
  "results": [
    {
      "entity_id": "ent_03DEF...",
      "similarity": 0.92,
      "entity": {...}
    }
  ],
  "total": 8
}
```

**Rules:**

- Semantic search uses vector embeddings generated by the AI adapter.
- Results are ranked by cosine similarity.
- Threshold filters results below the minimum similarity score.

---

## Graph Endpoints

### Traversal

```
GET /v1/graph/traverse/{entity_id}?depth=2&types=references,depends_on
```

**Query Parameters:**

| Parameter | Type | Default | Description |
|-----------|------|---------|-------------|
| `depth` | int | 1 | Maximum traversal depth |
| `types` | string | all | Comma-separated relationship types |
| `direction` | string | `both` | `outgoing`, `incoming`, or `both` |

**Response:**

```json
{
  "nodes": [
    { "id": "ent_01...", "type": "Article", "title": "..." },
    { "id": "ent_02...", "type": "Concept", "title": "..." }
  ],
  "edges": [
    { "source": "ent_01...", "target": "ent_02...", "type": "references" }
  ]
}
```

### Neighbors

```
GET /v1/graph/neighbors/{entity_id}?direction=both
```

**Response:**

```json
{
  "neighbors": [
    {
      "entity": { "id": "ent_02...", "type": "Concept", "title": "..." },
      "relationship": { "type": "references", "confidence": 0.95 }
    }
  ]
}
```

---

## Import Endpoints

### Import Document

```
POST /v1/import
```

**Request (multipart/form-data):**

```
file: {binary}
format: markdown
workspace_id: ws_01...
```

**Request (JSON with URL):**

```json
{
  "source": "https://example.com/article.html",
  "format": "html",
  "workspace_id": "ws_01..."
}
```

**Response:**

```json
{
  "job_id": "imp_01...",
  "status": "processing",
  "entities_created": 0
}
```

### Import Status

```
GET /v1/import/{job_id}
```

**Response:**

```json
{
  "job_id": "imp_01...",
  "status": "complete",
  "entities_created": 3,
  "entities": ["ent_01...", "ent_02...", "ent_03..."],
  "relationships_created": 2
}
```

### Batch Import

```
POST /v1/import/batch
```

**Request:**

```json
{
  "sources": [
    { "path": "/documents/paper1.pdf", "format": "pdf" },
    { "path": "/documents/paper2.pdf", "format": "pdf" }
  ],
  "workspace_id": "ws_01..."
}
```

---

## Context Endpoints

### Build Context

```
POST /v1/context
```

**Request:**

```json
{
  "entity_ids": ["ent_01...", "ent_02..."],
  "depth": 2,
  "max_tokens": 8000,
  "include": ["components", "relationships", "summaries"]
}
```

**Response:**

```json
{
  "context": {
    "entities": [...],
    "relationships": [...],
    "token_count": 6542
  }
}
```

**Rules:**

- Context assembly retrieves canonical data and derived summaries.
- Token count is estimated using the configured tokenizer.
- Context size is bounded by `max_tokens`.

### AI Question

```
POST /v1/ai/answer
```

**Request:**

```json
{
  "question": "What are the main themes across my collected papers?",
  "context_entity_ids": ["ent_01...", "ent_02..."],
  "provider": "openai"
}
```

**Response:**

```json
{
  "answer": "The main themes are...",
  "sources": [
    { "entity_id": "ent_01...", "relevance": 0.92 }
  ],
  "provider": "openai",
  "model": "gpt-4"
}
```

---

## Workspace Endpoints

### Create Workspace

```
POST /v1/workspaces
```

**Request:**

```json
{
  "name": "Research Team",
  "description": "Shared knowledge for the research team"
}
```

### List Workspaces

```
GET /v1/workspaces
```

### Add Member

```
POST /v1/workspaces/{id}/members
```

**Request:**

```json
{
  "user_id": "usr_01...",
  "role": "editor"
}
```

### List Entities in Workspace

```
GET /v1/workspaces/{id}/entities?type=Article&limit=50
```

---

## Plugin Endpoints

### List Plugins

```
GET /v1/plugins
```

### Install Plugin

```
POST /v1/plugins/install
```

**Request:**

```json
{
  "source": "knowledge-os-importer-csv",
  "version": "0.1.0"
}
```

### Plugin Health

```
GET /v1/plugins/{name}/health
```

---

## Derived Data Endpoints

### Derived Data Status

```
GET /v1/derived/{entity_id}
```

**Response:**

```json
{
  "entity_id": "ent_01...",
  "search_indexed": true,
  "embedded": true,
  "graph_projected": true,
  "last_indexed_at": "2026-07-21T10:05:00Z",
  "last_embedded_at": "2026-07-21T10:05:02Z"
}
```

### Rebuild Derived Data

```
POST /v1/derived/{entity_id}/rebuild
```

**Request:**

```json
{
  "artifacts": ["search_index", "embedding", "graph"]
}
```

### Rebuild All Derived Data

```
POST /v1/derived/rebuild
```

**Request:**

```json
{
  "artifacts": ["search_index", "embedding", "graph"],
  "filter": { "type": "Article" }
}
```

---

## Event Endpoints

### List Events

```
GET /v1/events?entity_id={id}&kind=EntityUpdated&limit=50
```

### Event Stream (SSE)

```
GET /v1/events/stream?types=EntityCreated,EntityUpdated
```

**Response (Server-Sent Events):**

```
event: EntityCreated
data: {"id":"evt_01...","kind":"EntityCreated","entity_id":"ent_04...","timestamp":"..."}

event: EntityUpdated
data: {"id":"evt_02...","kind":"EntityUpdated","entity_id":"ent_01...","timestamp":"..."}
```

---

## Error Responses

All errors follow a consistent structure:

```json
{
  "error": {
    "code": "CONFLICT",
    "message": "Entity version mismatch",
    "details": {
      "expected_version": 3,
      "actual_version": 4
    }
  }
}
```

### Error Codes

| Code | HTTP Status | Description |
|------|-------------|-------------|
| `NOT_FOUND` | 404 | Entity, relationship, or resource not found |
| `CONFLICT` | 409 | Version mismatch or duplicate component type |
| `VALIDATION_ERROR` | 400 | Invalid request payload |
| `UNAUTHORIZED` | 401 | Missing or invalid authentication |
| `FORBIDDEN` | 403 | Insufficient permissions |
| `UNSUPPORTED_FORMAT` | 400 | Import format not supported by any importer |
| `PLUGIN_ERROR` | 500 | Plugin execution failure |
| `STORAGE_ERROR` | 502 | Storage engine unreachable |
| `RATE_LIMITED` | 429 | Too many requests |

---

## MCP Interface

The MCP (Model Context Protocol) interface exposes the same operations through a protocol designed for AI agent integration.

### MCP Resources

| Resource | Description |
|----------|-------------|
| `entity://{id}` | Access a specific entity |
| `search://{query}` | Search for entities |
| `graph://{id}` | Access entity relationships |
| `context://{id}` | Build AI context for an entity |

### MCP Tools

| Tool | Description |
|------|-------------|
| `create_entity` | Create a new entity |
| `update_entity` | Update entity components |
| `search_entities` | Search by text or semantics |
| `traverse_graph` | Walk entity relationships |
| `build_context` | Assemble AI context |
| `import_document` | Import from external source |
| `answer_question` | Ask a question with context |

---

## Rate Limits

| Endpoint | Limit | Window |
|----------|-------|--------|
| Entity CRUD | 1000 req/min | Rolling |
| Search | 200 req/min | Rolling |
| Import | 10 req/min | Rolling |
| AI operations | 30 req/min | Rolling |
| Plugin operations | 20 req/min | Rolling |

Rate limit headers are included in every response:

```
X-RateLimit-Limit: 1000
X-RateLimit-Remaining: 997
X-RateLimit-Reset: 1692633600
```

---

## Further Reading

- [Pipeline](../architecture/pipeline.md) -- How API operations flow through the system
- [Events](../architecture/events.md) -- The event system behind API operations
- [Security](security.md) -- Authentication and authorization details
- [Plugin Development](../guides/plugin-development.md) -- Extending the API with plugins
- [AI Agent Guidelines](../guides/ai-agent-guidelines.md) -- MCP integration for AI agents
