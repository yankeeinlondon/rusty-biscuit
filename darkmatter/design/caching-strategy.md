# Caching Strategy

**Status:** Phase 1 Complete - Aligned with Graph-Based Architecture
**Last Updated:** 2025-12-20
**Related:** [Graph Caching Architecture](./graph-caching-architecture.md), [Old Caching Strategy](./old-caching-strategy.md)

## Overview

This document defines the comprehensive caching strategy for Composition's graph-based caching system (v2). The strategy uses a unified resource model in SurrealDB with status-driven processing (PENDING → DONE → DIRTY → FAILED) and content-addressed caching.

**See also:** [Graph Caching Architecture](./graph-caching-architecture.md) for complete implementation details.

## Refresh Intervals by Resource Type

Refresh intervals determine how long cached results remain valid before revalidation.

| Resource Type | Interval | Trigger | Rationale |
|--------------|----------|---------|-----------|
| **Document (local)** | Immediate | Content hash change | Local files controlled by user; real-time change detection via `xxhash` |
| **Document (remote)** | 6 hours | Time-based expiry | Remote documents are slow-moving; balance freshness vs HTTP requests |
| **Image (local)** | Immediate | Content hash change | User controls source; detect edits immediately |
| **Image (remote)** | 1 day | Time-based expiry | Match typical CDN caching policies |
| **Audio (local)** | Immediate | Content hash change | Large files, infrequent changes |
| **Audio (remote)** | 1 day | Time-based expiry | Streaming URLs rarely change |
| **Video (local)** | Immediate | Content hash change | Very large files, infrequent changes |
| **Video (remote)** | 1 day | Time-based expiry | Streaming service stability |
| **LLM (summarize)** | 30 days | Time-based expiry | Balance cost vs freshness; summaries stable |
| **LLM (consolidate)** | 14 days | Time-based expiry | Consolidation may need refresh sooner |
| **LLM (topic)** | 60 days | Time-based expiry | Topic extraction is stable long-term |
| **Embeddings** | Never | Content hash change only | Deterministic for same content + model |

### Implementation

**Local resources:**

```rust
// Always check content hash - no time-based expiry
if resource.source == ResourceSource::Local {
    return resource.content_hash == resource.last_rendered_hash;
}
```

**Remote resources:**

```rust
// Check both content hash AND time-based expiry
if resource.source == ResourceSource::Remote {
    let now = time::unix(time::now());
    let last_checked = time::unix(resource.last_checked);
    let expiry_secs = resource.expiry_days * 86400;

    if (now - last_checked) > expiry_secs {
        return false; // Expired
    }

    return resource.content_hash == resource.last_rendered_hash;
}
```

**SurrealQL function:**

See `fn::is_resource_stale()` in [Graph Caching Architecture](./graph-caching-architecture.md#complete-schema-specification).

## Hash Strategy

### Hash Algorithm: xxHash (xxh3_64)

**Why xxHash?**

- **Speed**: 10-50x faster than cryptographic hashes (SHA-256)
- **Quality**: Excellent distribution, low collision rate
- **Non-cryptographic**: Acceptable for cache validation (not security-critical)
- **Compact**: 64-bit output (16 hex characters)

**Implementation:** `xxhash-rust` crate

```rust
use xxhash_rust::xxh3::xxh3_64;

let hash = format!("{:016x}", xxh3_64(bytes));
```

### Hash Types

**1. Resource Hash (Identity)**

**Purpose:** Identifies the resource location (where it is)

**Input:** Canonical resource path or URL

```rust
// Local file
let resource_hash = ResourceHash::new(Path::new("docs/article.md"));
// Hashes: "docs/article.md"

// Remote URL
let resource_hash = ResourceHash::new_from_url("https://example.com/data.json");
// Hashes: "https://example.com/data.json"
```

**Normalization:**

- Paths are canonicalized (resolve `..`, `.`, symlinks)
- URLs are lowercased, query parameters sorted
- Ensures same resource = same hash

**2. Content Hash (Validation)**

**Purpose:** Validates the resource content (what it contains)

**Input:** Raw file bytes or response body

```rust
let bytes = fs::read("docs/article.md")?;
let content_hash = ContentHash::new(&bytes);
```

**Whitespace handling:**

- **Documents**: Trim leading/trailing whitespace before hashing (minor edits don't invalidate)
- **Binary**: Hash raw bytes (no normalization)

**3. Last Rendered Hash (Cache Validation)**

**Purpose:** Content hash at time of last successful render

**Usage:**

```rust
// Cache hit condition
if resource.last_rendered_hash == resource.content_hash {
    // Content unchanged since last render - use cache
}
```

### Hash Collision Handling

**Probability:** xxHash collision rate is ~1 in 18 quintillion (2^64)

**Detection:**

```rust
// If collision detected (extremely rare)
if resource1.content_hash == resource2.content_hash && resource1.bytes != resource2.bytes {
    eprintln!("CRITICAL: xxHash collision detected!");
    eprintln!("Resource 1: {:?}", resource1.path);
    eprintln!("Resource 2: {:?}", resource2.path);

    // Fallback: use SHA-256 for these specific resources
    use sha2::{Sha256, Digest};
    let mut hasher = Sha256::new();
    hasher.update(&resource1.bytes);
    resource1.content_hash = format!("{:x}", hasher.finalize());
}
```

**Mitigation:** Log collision, fall back to cryptographic hash for affected resources.

## Cache Key Structure

### Composite Key Pattern

All cache lookups use a composite key: `(resource_hash, content_hash)`

**Example:**

```
resource_hash: "a3f2b1c9d4e5f6a7"  (location: docs/intro.md)
content_hash:  "1234567890abcdef"  (current content)
```

**Query:**

```surrealql
SELECT * FROM resource
WHERE resource_hash = 'a3f2b1c9d4e5f6a7'
  AND content_hash = '1234567890abcdef'
  AND status = 'DONE';
```

**Optimization:** Covering index `idx_cache_validation` includes all fields needed for validation without table lookup.

### RecordId Format

SurrealDB RecordIds use table + unique key:

```
resource:a3f2b1c9d4e5f6a7
```

**Rust representation:**

```rust
use surrealdb::RecordId;

let id = RecordId::from(("resource", resource_hash.as_str()));
```

## Expiry Policies

### Local vs Remote Resources

**Local resources (source = 'local'):**

- **Never expire** based on time
- Only invalidated on content hash change
- Rationale: User controls files, no external changes

**Remote resources (source = 'remote'):**

- **Time-based expiry** via `expiry_days` field
- Checked on every cache lookup
- Rationale: External content may change without notification

### Expiry Configuration

**Default expiry by type:**

| Resource Type | Default Expiry (days) | Configurable? |
|--------------|----------------------|---------------|
| Remote document | 0.25 (6 hours) | Yes (per-reference) |
| Remote image | 1 | Yes |
| Remote audio | 1 | Yes |
| Remote video | 1 | Yes |
| LLM summarize | 30 | Yes |
| LLM consolidate | 14 | Yes |
| LLM topic | 60 | Yes |

**Per-reference override:**

```markdown
<!-- Default: 6 hours -->
::file https://example.com/data.json

<!-- Override: 1 day -->
::file https://example.com/data.json?cache=1d

<!-- Override: No expiry (until content changes) -->
::file https://example.com/data.json?cache=never
```

**Implementation:**

```rust
// Parse cache override from query parameter
let expiry_days = url.query_pairs()
    .find(|(k, _)| k == "cache")
    .and_then(|(_, v)| parse_duration(&v))
    .unwrap_or(default_expiry_for_type(resource.type_));
```

### Expiry Checking

**Periodic cleanup:**

```bash
# Run hourly cron job
0 * * * * composition cache clean-expired
```

**Rust implementation:**

```rust
// lib/src/cache/expiry.rs
pub async fn mark_expired_dirty(db: &Surreal<Db>) -> Result<usize> {
    let count: usize = db.query(r#"
        UPDATE resource
        SET status = 'DIRTY', updated_at = time::now()
        WHERE source = 'remote'
          AND status = 'DONE'
          AND time::unix(time::now()) - time::unix(last_checked) > expiry_days * 86400
    "#)
    .await?
    .take(0)?;

    Ok(count)
}
```

## Status State Machine

Resources transition through states during their lifecycle. See [Graph Caching Architecture](./graph-caching-architecture.md#status-state-machine) for complete state machine documentation.

**Quick Reference:**

- **PENDING**: Not yet processed
- **DONE**: Successfully cached
- **DIRTY**: Content or dependency changed
- **FAILED**: Error during processing

**Transitions:**

```
PENDING → DONE (success)
PENDING → FAILED (error)
DONE → DIRTY (invalidation)
DIRTY → DONE (reprocessing success)
FAILED → PENDING (retry)
```

## Cache Invalidation

### Content-Based Invalidation

When a resource's content changes:

1. Compute new `content_hash`
2. Compare to `last_rendered_hash`
3. If different: Mark status as `DIRTY`

```rust
if resource.content_hash != resource.last_rendered_hash {
    resource.status = ResourceStatus::Dirty;
}
```

### Cascade Invalidation

When a resource is marked DIRTY, all dependents must also be marked DIRTY:

```surrealql
-- Get all dependents via graph traversal
SELECT * FROM resource:changed_doc.{1..20}(<-depends_on<-resource);

-- Mark all dependents dirty
UPDATE resource SET status = 'DIRTY' WHERE id IN $dependents;
```

**Implementation:** See `lib/src/graph/invalidation.rs::cascade_invalidate()` in [Graph Caching Architecture](./graph-caching-architecture.md#cache-invalidation-flows).

### Manual Invalidation

```bash
# CLI: Invalidate specific resource
composition cache invalidate docs/article.md

# CLI: Invalidate all of type
composition cache invalidate --type image

# CLI: Clear entire cache
composition cache clear
```

## Derivative Resources

### Image Variants

Original images spawn multiple optimized variants (WebP, AVIF at various widths).

**Example:**

```
Original: images/hero.jpg
Variants:
  - images/.composition/hero-640.webp
  - images/.composition/hero-1280.webp
  - images/.composition/hero-1920.webp
  - images/.composition/hero-640.avif
  - images/.composition/hero-1280.avif
  - images/.composition/hero-1920.avif
```

**Graph representation:**

```surrealql
-- Original has multiple variants
RELATE resource:hero_jpg -> has_variant -> resource:hero_640_webp
SET width = 640, format = 'webp';

RELATE resource:hero_jpg -> has_variant -> resource:hero_1280_webp
SET width = 1280, format = 'webp';

-- ... more variants
```

**Invalidation:** When original is invalidated, all variants are invalidated via graph traversal.

### Automatic Cleanup

When a source image is deleted, all variants are automatically cleaned up via SurrealDB event:

```surrealql
DEFINE EVENT cleanup_variants ON TABLE resource WHEN $event = "DELETE" THEN {
    LET $variant_ids = (SELECT VALUE out FROM has_variant WHERE in = $before.id);
    DELETE has_variant WHERE in = $before.id;
    DELETE resource WHERE id IN $variant_ids;
};
```

## Performance Characteristics

### Cache Hit Benefits

| Operation | Cold (No Cache) | Warm (Cache Hit) | Speedup |
|-----------|----------------|------------------|---------|
| Markdown parse | 1-10ms | <1ms | 10-100x |
| Image optimization | 50-500ms | <1ms | 50-500x |
| LLM summarization | 2-10s | <1ms | 2000-10000x |
| Embedding generation | 100-500ms | <1ms | 100-500x |
| Audio metadata extraction | 10-100ms | <1ms | 10-100x |

### Storage Overhead

**Per resource:**

- Metadata: ~200 bytes
- Embeddings: ~6KB (1536 dimensions × 4 bytes)
- LLM responses: 100-5000 bytes

**Typical project:**

- 100 documents → 20KB
- 50 images (with 6 variants each) → 300 × 200B = 60KB
- 10 audio files → 2KB
- 20 LLM calls → 10-50KB
- 50 embeddings → 300KB

**Total:** ~400KB (excluding LLM responses)

### Query Performance Targets

From Phase 1 review requirements:

- Work scheduling query: <10ms (100-node graph)
- Cache lookup: <5ms (with covering index)
- Cascade invalidation: <100ms (typical tree, 10-50 deps)
- Graph traversal: <50ms (up to 10 levels deep)

## Migration from v1

Existing projects with v1 caching will be migrated to v2 via transaction-safe migration.

See [Schema Evolution](../reference/schema-evolution.md) for complete migration procedures.

**High-level:**

1. Backup database
2. Apply v2 schema (additive)
3. Migrate resources from `document`, `image_cache`, etc. to unified `resource` table
4. Validate counts match
5. Commit transaction
6. Record schema version

**Backward compatibility:** v1 API wrappers provided during transition period.

## Configuration

### Cache Location

```bash
# Git repository (project-local cache)
.composition.db

# Non-git directory (global cache)
$HOME/.composition.db
```

### Environment Variables

```bash
# Override cache location
export COMPOSITION_CACHE_PATH=/custom/path/.composition.db

# Default expiry for remote resources (days)
export COMPOSITION_REMOTE_EXPIRY=1

# Default expiry for LLM operations (days)
export COMPOSITION_LLM_EXPIRY=30

# Enable cache debug logging
export COMPOSITION_CACHE_DEBUG=1
```

### Frontmatter Configuration

```yaml
---
cache:
  remote_expiry: 2d      # 2 days
  llm_expiry: 60d        # 60 days
  image_formats: [webp, avif]
  image_widths: [640, 1280, 1920]
---
```

## Related Documentation

- **[Graph Caching Architecture](./graph-caching-architecture.md)** - Complete v2 design
- **[Old Caching Strategy](./old-caching-strategy.md)** - v1 implementation (deprecated)
- **[SurrealQL Queries](../reference/surrealql-queries.md)** - All cache queries
- **[Schema Evolution](../reference/schema-evolution.md)** - Migration procedures
- **[Work Scheduling](../reference/work-scheduling.md)** - Scheduling algorithm

## Future Enhancements

**Phase 2-6 implementation:**

- Full graph reconstruction (recursive traversal)
- Image variant storage (persist processed bytes)
- Garbage collection (periodic cleanup of stale entries)
- Cache statistics (hit rate tracking, size monitoring)
- Remote cache (shared across team members)
- Cache compression (reduce storage size)

**v3 potential:**

- Status history tracking (debugging)
- Cache size limits (LRU eviction)
- Distributed caching (multi-machine support)
- Enhanced vector search (HNSW optimization)
