# Operational Runbooks

> Incident response procedures, recovery workflows, and operational procedures for Knowledge OS deployments.

---

## Purpose

This document provides step-by-step procedures for common operational tasks. These runbooks are written for engineers operating Knowledge OS deployments, not for end users.

---

## Runbook 1: Derived Data Rebuild

### Symptom

Search results are stale, embeddings are outdated, or graph projections are inconsistent with canonical data.

### Diagnosis

1. Check event log for unprocessed events.
2. Check derived data stores for consistency.
3. Compare entity counts across canonical and derived stores.

### Procedure

**Full rebuild:**

```bash
# Stop the pipeline
knowledge-os pipeline stop

# Drop all derived data
knowledge-os derive drop --all

# Rebuild from canonical data
knowledge-os derive rebuild --all

# Verify consistency
knowledge-os verify --derived

# Restart the pipeline
knowledge-os pipeline start
```

**Partial rebuild (search index only):**

```bash
# Drop search index
knowledge-os derive drop --type search

# Rebuild search index
knowledge-os derive rebuild --type search

# Verify search index
knowledge-os verify --type search
```

**Partial rebuild (embeddings only):**

```bash
# Drop embeddings
knowledge-os derive drop --type embedding

# Rebuild embeddings (may take minutes to hours depending on entity count)
knowledge-os derive rebuild --type embedding

# Verify embeddings
knowledge-os verify --type embedding
```

### Verification

1. Query search index for known entities. Verify results match canonical data.
2. Query vector store for known entities. Verify embeddings exist.
3. Query graph store for known relationships. Verify edges exist.
4. Run the full verification suite: `knowledge-os verify --all`.

---

## Runbook 2: Storage Engine Failure

### Symptom

One or more storage engines are reporting errors. Pipeline events are failing. Derived data may be inconsistent.

### Diagnosis

1. Check storage adapter health: `knowledge-os health`.
2. Identify the failed engine from the health report.
3. Check storage engine logs for error details.

### Procedure

**SQLite failure (local deployment):**

```bash
# Check SQLite file integrity
sqlite3 ./data/knowledge.db "PRAGMA integrity_check;"

# If integrity check fails, restore from backup
cp ./backups/knowledge.db ./data/knowledge.db

# Rebuild derived data from restored canonical data
knowledge-os derive rebuild --all
```

**PostgreSQL failure (production deployment):**

```bash
# Check PostgreSQL connection
psql -h localhost -U knowledge_os -c "SELECT 1;"

# If connection fails, check PostgreSQL service
systemctl status postgresql

# If data is corrupted, restore from WAL backup
pg_restore -h localhost -U knowledge_os -d knowledge_os ./backups/knowledge.dump

# Rebuild derived data
knowledge-os derive rebuild --all
```

**Search engine failure (Tantivy):**

```bash
# Tantivy data is derived. Drop and rebuild.
knowledge-os derive drop --type search
knowledge-os derive rebuild --type search
```

**Vector store failure (Qdrant):**

```bash
# Vector data is derived. Drop and rebuild.
knowledge-os derive drop --type embedding
knowledge-os derive rebuild --type embedding
```

**Cache failure (Redis):**

```bash
# Cache is derived. Clear and repopulate.
knowledge-os derive drop --type cache
# Cache repopulates on demand. No rebuild needed.
```

### Verification

1. Run `knowledge-os health` to verify all engines are healthy.
2. Run `knowledge-os verify --canonical` to verify canonical data integrity.
3. Run `knowledge-os verify --derived` to verify derived data consistency.
4. Test a search query to verify search functionality.
5. Test an entity retrieval to verify relational storage.

---

## Runbook 3: Event Processing Failure

### Symptom

Derived data is stale. Events are accumulating in the dead letter queue. Pipeline throughput has dropped.

### Diagnosis

1. Check dead letter queue: `knowledge-os events dlq list`.
2. Check event processing metrics: `knowledge-os metrics events`.
3. Identify failed events and their error messages.

### Procedure

**Reprocess dead letter queue:**

```bash
# List failed events
knowledge-os events dlq list

# Reprocess all failed events
knowledge-os events dlq reprocess --all

# Or reprocess specific events
knowledge-os events dlq reprocess --event-id <uuid>
```

**Reset event processing (nuclear option):**

```bash
# Stop the pipeline
knowledge-os pipeline stop

# Clear the event log (WARNING: this loses event history)
knowledge-os events clear

# Rebuild all derived data from canonical data
knowledge-os derive rebuild --all

# Restart the pipeline
knowledge-os pipeline start
```

### Verification

1. Check dead letter queue is empty: `knowledge-os events dlq list`.
2. Check event processing metrics show normal throughput.
3. Verify derived data consistency: `knowledge-os verify --derived`.

---

## Runbook 4: Storage Engine Migration

### Symptom

Need to migrate from one storage engine to another (e.g., SQLite to PostgreSQL, Tantivy to Elasticsearch).

### Preparation

1. Back up all canonical data.
2. Back up all derived data (optional, since derived data is rebuildable).
3. Verify the new storage engine is running and accessible.

### Procedure

**Relational migration (SQLite to PostgreSQL):**

```bash
# Export canonical data from SQLite
knowledge-os export --format canonical --output ./migration/canonical.json

# Stop the pipeline
knowledge-os pipeline stop

# Update storage configuration
# Edit config.toml to point to PostgreSQL

# Import canonical data into PostgreSQL
knowledge-os import --format canonical --input ./migration/canonical.json

# Rebuild all derived data
knowledge-os derive rebuild --all

# Verify consistency
knowledge-os verify --all

# Restart the pipeline
knowledge-os pipeline start
```

**Search engine migration (Tantivy to Elasticsearch):**

```bash
# Search data is derived. Drop and rebuild.
knowledge-os derive drop --type search

# Update storage configuration
# Edit config.toml to point to Elasticsearch

# Rebuild search index
knowledge-os derive rebuild --type search

# Verify search functionality
knowledge-os search --query "test" --limit 10
```

### Verification

1. Run `knowledge-os health` to verify all engines are healthy.
2. Run `knowledge-os verify --canonical` to verify canonical data integrity.
3. Run `knowledge-os verify --derived` to verify derived data consistency.
4. Test all critical operations: import, search, entity retrieval, graph traversal.

---

## Runbook 5: Backup and Restore

### Backup Procedure

**Canonical data backup:**

```bash
# SQLite backup
cp ./data/knowledge.db ./backups/knowledge-$(date +%Y%m%d).db

# PostgreSQL backup
pg_dump -h localhost -U knowledge_os -d knowledge_os > ./backups/knowledge-$(date +%Y%m%d).sql

# Full backup (canonical + configuration)
knowledge-os backup --output ./backups/full-$(date +%Y%m%d)
```

**Automated backup (cron):**

```bash
# Daily backup at 2 AM
0 2 * * * knowledge-os backup --output /backups/knowledge-$(date +\%Y\%m\%d)
```

### Restore Procedure

**From SQLite backup:**

```bash
# Stop the pipeline
knowledge-os pipeline stop

# Restore SQLite file
cp ./backups/knowledge-20260721.db ./data/knowledge.db

# Rebuild derived data
knowledge-os derive rebuild --all

# Restart the pipeline
knowledge-os pipeline start
```

**From PostgreSQL backup:**

```bash
# Stop the pipeline
knowledge-os pipeline stop

# Restore PostgreSQL database
psql -h localhost -U knowledge_os -d knowledge_os < ./backups/knowledge-20260721.sql

# Rebuild derived data
knowledge-os derive rebuild --all

# Restart the pipeline
knowledge-os pipeline start
```

**From full backup:**

```bash
# Stop the pipeline
knowledge-os pipeline stop

# Restore from full backup
knowledge-os restore --input ./backups/full-20260721

# Restart the pipeline
knowledge-os pipeline start
```

---

## Runbook 6: Scaling Operations

### Horizontal Scaling (Adding Pipeline Workers)

```bash
# Start additional pipeline workers
knowledge-os pipeline start --workers 4

# Verify workers are running
knowledge-os pipeline status
```

### Vertical Scaling (Increasing Resource Limits)

Update the configuration:

```toml
[pipeline]
max_concurrent_imports = 20
max_concurrent_derivations = 50
max_memory_mb = 8192
```

### Database Scaling (Adding Read Replicas)

1. Set up PostgreSQL read replicas.
2. Update configuration to point read operations to replicas.
3. Verify read operations use replicas.
4. Monitor replication lag.

### Search Index Sharding

1. Configure Elasticsearch with multiple shards.
2. Rebuild search index with sharding enabled:
   ```bash
   knowledge-os derive drop --type search
   knowledge-os derive rebuild --type search --shards 4
   ```

---

## Runbook 7: Health Check Interpretation

### Health Check Command

```bash
knowledge-os health
```

### Output Format

```
Overall Status: healthy | degraded | unhealthy

Storage Engines:
  Relational (SQLite):    healthy    (latency: 2ms)
  Search (Tantivy):       healthy    (latency: 5ms)
  Vector (Qdrant):        degraded   (latency: 150ms, threshold: 100ms)
  Cache (Redis):          healthy    (latency: 1ms)
  Object (Local):         healthy    (latency: 1ms)

Pipeline:
  Import:                 healthy    (throughput: 150 docs/min)
  Parsing:                healthy    (throughput: 200 docs/min)
  Normalization:          healthy    (throughput: 180 docs/min)
  Knowledge Model:        healthy    (throughput: 200 ops/sec)
  Relationship Engine:    healthy    (throughput: 150 ops/sec)
  Derivation:             degraded   (throughput: 80 ops/sec, threshold: 100)
  Presentation:           healthy    (throughput: 500 ops/sec)

Events:
  Queue depth:            12
  Dead letter queue:      0
  Processing rate:        1000 events/sec
```

### Status Meanings

| Status | Meaning |
|--------|---------|
| `healthy` | Component is operating within normal parameters |
| `degraded` | Component is operational but outside normal parameters. Investigate. |
| `unhealthy` | Component has failed. Immediate attention required. |

### Common Issues

| Issue | Likely Cause | Resolution |
|-------|-------------|------------|
| Vector latency high | Qdrant overloaded | Scale Qdrant horizontally or reduce embedding batch size |
| Derivation throughput low | AI provider rate limited | Check AI provider limits, implement backoff |
| Event queue depth growing | Pipeline saturation | Add pipeline workers or optimize slow handlers |
| Dead letter queue non-empty | Event processing failures | Run Runbook 3 (Event Processing Failure) |

---

## Runbook 8: Plugin Management

### Installing a Plugin

```bash
# Install from local file
knowledge-os plugin install ./path/to/plugin.so

# Install from marketplace
knowledge-os plugin install <plugin-name>

# Verify installation
knowledge-os plugin list
```

### Removing a Plugin

```bash
# Deactivate the plugin
knowledge-os plugin deactivate <plugin-name>

# Remove the plugin
knowledge-os plugin remove <plugin-name>

# Rebuild derived data if the plugin affected derived artifacts
knowledge-os derive rebuild --all
```

### Debugging Plugin Issues

```bash
# Check plugin health
knowledge-os plugin health <plugin-name>

# View plugin logs
knowledge-os plugin logs <plugin-name>

# Reinstall the plugin
knowledge-os plugin remove <plugin-name>
knowledge-os plugin install <plugin-path>
```

---

## Further Reading

- [Deployment Architecture](deployment.md) -- Deployment models and configuration
- [Security Architecture](security.md) -- Threat model and access control
- [Testing Strategy](testing-strategy.md) -- Test philosophy and pipeline testing
- [Storage](../architecture/storage.md) -- Storage engine details
- [Events](../architecture/events.md) -- Event system architecture
- [Synchronization](../architecture/synchronization.md) -- Consistency model
