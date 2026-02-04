# Graph-Based Caching Architecture

**Status:** Design - Ready for Implementation (Phase 1 Complete)
**Last Updated:** 2025-12-20
**Related:** [Caching Strategy](./caching-strategy.md), [Work Scheduling](../reference/work-scheduling.md), [Schema Evolution](../reference/schema-evolution.md)

## Executive Summary

This document specifies the comprehensive redesign of Composition's caching strategy to fully leverage SurrealDB's graph database capabilities. The current implementation builds dependency graphs in-memory and uses the database only for passive storage. The new design treats resource dependencies as a native graph problem, using SurrealDB to handle graph traversal, topological sorting, and cache invalidation through native graph queries.

**Key Outcomes:**

- **Unified resource model**: All cacheable entities (documents, images, audio, video, LLM operations, embeddings) as graph nodes
- **Database-driven scheduling**: SurrealDB handles work scheduling via iterative state queries
- **Automatic invalidation**: Graph edges enable cascade invalidation when resources change
- **Derivative tracking**: Image variants and optimized assets tracked as graph relationships
- **Resumable builds**: Database state persists, enabling interrupted builds to resume

## Motivation

### Current Limitations (v1)

**In-Memory Graph Approach:**

```rust
// All graph operations in Rust memory
let graph = HashMap<ResourceHash, Node>::new();
let edges = Vec<(ResourceHash, ResourceHash)>::new();

// Build entire graph before processing
build_graph_recursive(&resource, &mut graph, &mut edges)?;

// Kahn's algorithm in Rust
let layers = topological_sort(&graph, &edges)?;

// Process all layers
for layer in layers {
    rayon::scope(|s| {
        for resource in layer {
            s.spawn(|_| process(resource));
        }
    });
}
```

**Problems:**

1. **Graph logic duplication**: Topological sort implemented in Rust, not leveraging SurrealDB's graph capabilities
2. **No status tracking**: Can't resume interrupted builds - must start from scratch
3. **Separate cache tables**: `document`, `image_cache`, `audio_cache`, `llm_cache` with no relationships
4. **Cache invalidation requires full rebuild**: No incremental invalidation
5. **No derivative tracking**: Image variants not tracked as graph relationships

### Target State (v2)

**Database-Driven Graph:**

```rust
// Iterative state approach - ask DB for ready work
loop {
    // SurrealDB returns next batch via graph query
    let batch = scheduler.get_next_batch().await?;

    if batch.is_empty() {
        if scheduler.is_deadlocked().await? {
            return Err(Error::Deadlock);
        }
        break; // All work complete
    }

    // Process batch in parallel
    batch.par_iter().for_each(|r| process(r));

    // Update database
    scheduler.mark_batch_done(&batch).await?;
    // Loop continues - next layer automatically ready
}
```

**Benefits:**

1. **SurrealDB handles graph traversal**: Leverages database's core strength
2. **Unified resource model**: All types in single `resource` table
3. **Status tracking**: `PENDING → DONE → DIRTY → FAILED` state machine
4. **Incremental builds**: Resume from last successful state
5. **Cascade invalidation**: Graph edges enable automatic dependency invalidation
6. **Derivative tracking**: `has_variant` edges track image optimization relationships

## Unified Resource Model

### Core Principles

**Single Table for All Resources:**

All cacheable entities (documents, images, audio, video, LLM operations, embeddings) are stored in a single `resource` table. Type-specific fields are optional, allowing a polymorphic approach.

**Graph Edges as Relationships:**

- `depends_on`: Dependency relationship (document A transcl

udes document B)
- `has_variant`: Derivative relationship (original image → optimized WebP variant)

**Status-Driven Processing:**

Resources transition through states: `PENDING` (not yet processed) → `DONE` (successfully cached) → `DIRTY` (content or dependency changed) → `FAILED` (error occurred).

**Content Addressing:**

- `resource_hash`: Identity hash (location - path or URL)
- `content_hash`: Validation hash (actual content)
- `last_rendered_hash`: Cache validation (content hash at last successful render)

### Complete Schema Specification

```surrealql
-- ============================================================================
-- UNIFIED RESOURCE TABLE
-- ============================================================================

DEFINE TABLE resource SCHEMAFULL;

-- Core fields (all resource types)
DEFINE FIELD type ON resource TYPE string
    ASSERT $value IN ['document', 'image', 'audio', 'video', 'llm_operation', 'embedding'];

DEFINE FIELD source ON resource TYPE string
    ASSERT $value IN ['local', 'remote'];

DEFINE FIELD status ON resource TYPE string
    ASSERT $value IN ['PENDING', 'DONE', 'DIRTY', 'FAILED']
    DEFAULT 'PENDING';

-- Content addressing
DEFINE FIELD resource_hash ON resource TYPE string;      -- xxh3_64 of location
DEFINE FIELD content_hash ON resource TYPE string;       -- xxh3_64 of content
DEFINE FIELD last_rendered_hash ON resource TYPE string; -- content_hash at last render

-- Source location (one of these will be populated)
DEFINE FIELD url ON resource TYPE option<string>;        -- Remote resources
DEFINE FIELD path ON resource TYPE option<string>;       -- Local resources

-- Time-based expiry (remote resources)
DEFINE FIELD last_checked ON resource TYPE datetime DEFAULT time::now();
DEFINE FIELD expiry_days ON resource TYPE int DEFAULT 1;

-- Timestamps
DEFINE FIELD created_at ON resource TYPE datetime DEFAULT time::now();
DEFINE FIELD updated_at ON resource TYPE datetime DEFAULT time::now();

-- Error tracking
DEFINE FIELD error_message ON resource TYPE option<string>;  -- For FAILED status

-- ============================================================================
-- TYPE-SPECIFIC FIELDS (optional based on type)
-- ============================================================================

-- Image-specific fields (when type='image')
DEFINE FIELD width ON resource TYPE option<int>;
DEFINE FIELD height ON resource TYPE option<int>;
DEFINE FIELD format ON resource TYPE option<string>
    ASSERT $value IN ['webp', 'avif', 'png', 'jpg', 'jpeg'] OR $value IS NONE;
DEFINE FIELD has_transparency ON resource TYPE option<bool>;

-- Audio-specific fields (when type='audio')
DEFINE FIELD duration_secs ON resource TYPE option<float>;
DEFINE FIELD bitrate ON resource TYPE option<int>;
DEFINE FIELD sample_rate ON resource TYPE option<int>;
DEFINE FIELD channels ON resource TYPE option<int>;

-- Video-specific fields (when type='video')
DEFINE FIELD video_codec ON resource TYPE option<string>;
DEFINE FIELD audio_codec ON resource TYPE option<string>;
DEFINE FIELD framerate ON resource TYPE option<float>;

-- LLM operation tracking (when type='llm_operation')
DEFINE FIELD operation ON resource TYPE option<string>
    ASSERT $value IN ['summarize', 'consolidate', 'topic'] OR $value IS NONE;
DEFINE FIELD model ON resource TYPE option<string>;
DEFINE FIELD input_hash ON resource TYPE option<string>;
DEFINE FIELD response ON resource TYPE option<string>;
DEFINE FIELD tokens_used ON resource TYPE option<int>;

-- Vector embeddings (when type='embedding')
DEFINE FIELD vector ON resource TYPE option<array<float>>;
DEFINE FIELD embedding_model ON resource TYPE option<string>;
DEFINE FIELD dimension ON resource TYPE option<int>;

-- ============================================================================
-- INDEXES
-- ============================================================================

-- Primary lookup by resource hash (location)
DEFINE INDEX idx_resource_hash ON resource FIELDS resource_hash UNIQUE;

-- Cache lookup by resource + content hash
DEFINE INDEX idx_content_lookup ON resource FIELDS resource_hash, content_hash;

-- Status filtering for work scheduling
DEFINE INDEX idx_status ON resource FIELDS status;

-- Type and source filtering
DEFINE INDEX idx_type_source ON resource FIELDS type, source;

-- Time-based queries (expiry checking)
DEFINE INDEX idx_last_checked ON resource FIELDS last_checked;

-- Covering index for cache validation (avoids table lookup)
DEFINE INDEX idx_cache_validation ON resource FIELDS resource_hash, content_hash, status, last_rendered_hash;

-- Vector index for embeddings (SurrealDB 2.4+)
-- MTREE for approximate nearest neighbor search
DEFINE INDEX idx_embedding_vector ON resource FIELDS vector
    MTREE DIMENSION 1536 DIST COSINE;
-- Note: DIMENSION must match your embedding model (1536 for OpenAI Ada-002)

-- ============================================================================
-- DEPENDENCY EDGE TABLE
-- ============================================================================

DEFINE TABLE depends_on SCHEMAFULL;

-- Edge direction: in (child) depends on out (parent)
-- Example: document A (in) depends on document B (out)
DEFINE FIELD in ON depends_on TYPE record<resource>;     -- Child resource
DEFINE FIELD out ON depends_on TYPE record<resource>;    -- Parent dependency

-- Dependency metadata
DEFINE FIELD reference_type ON depends_on TYPE string
    ASSERT $value IN ['transclusion', 'image', 'audio', 'video', 'data'];

DEFINE FIELD required ON depends_on TYPE bool DEFAULT false;  -- ! vs ?

-- Indexes for graph traversal
DEFINE INDEX idx_depends_in ON depends_on FIELDS in;
DEFINE INDEX idx_depends_out ON depends_on FIELDS out;

-- Composite index for filtered traversals
DEFINE INDEX idx_depends_in_out ON depends_on FIELDS in, out;

-- ============================================================================
-- DERIVATIVE/VARIANT EDGE TABLE
-- ============================================================================

DEFINE TABLE has_variant SCHEMAFULL;

-- Edge direction: in (source) has variant out (optimized)
-- Example: original.jpg (in) has variant optimized.webp (out)
DEFINE FIELD in ON has_variant TYPE record<resource>;   -- Source resource
DEFINE FIELD out ON has_variant TYPE record<resource>;  -- Optimized variant

-- Variant metadata
DEFINE FIELD width ON has_variant TYPE int;             -- Target width
DEFINE FIELD format ON has_variant TYPE string
    ASSERT $value IN ['webp', 'avif'];

-- Indexes for variant lookup
DEFINE INDEX idx_variant_in ON has_variant FIELDS in;
DEFINE INDEX idx_variant_out ON has_variant FIELDS out;

-- ============================================================================
-- SCHEMA VERSION TRACKING
-- ============================================================================

DEFINE TABLE schema_version SCHEMAFULL;
DEFINE FIELD version ON schema_version TYPE int;
DEFINE FIELD applied_at ON schema_version TYPE datetime DEFAULT time::now();
DEFINE FIELD description ON schema_version TYPE string;

-- ============================================================================
-- SURREALQL FUNCTIONS
-- ============================================================================

-- Check if a resource is stale and needs reprocessing
DEFINE FUNCTION fn::is_resource_stale($res: record<resource>) {
    -- Remote resources: check time-based expiry
    IF $res.source = 'remote' THEN {
        -- Convert expiry_days to seconds
        LET $expiry_secs = $res.expiry_days * 86400;
        LET $last_checked_unix = time::unix($res.last_checked);
        LET $now_unix = time::unix(time::now());

        -- Check if expired
        IF ($now_unix - $last_checked_unix) > $expiry_secs THEN {
            RETURN true;
        };
    };

    -- Local resources and non-expired remote: check content hash
    -- NULL handling for never-rendered resources
    IF $res.last_rendered_hash IS NONE THEN {
        RETURN true;
    };

    RETURN $res.content_hash != $res.last_rendered_hash;
};

-- Get ready work for a target document (returns next batch)
DEFINE FUNCTION fn::get_ready_work($target_doc: string) {
    -- Get dependency tree for target
    LET $tree = (
        SELECT VALUE out FROM (
            SELECT ->depends_on->resource AS out
            FROM type::thing('resource', $target_doc)
        ).out.*
    );

    -- Return resources that are ready to process
    RETURN SELECT * FROM resource
    WHERE id IN $tree
      AND (status = 'PENDING' OR status = 'DIRTY')
      AND count(->depends_on[WHERE out.status != 'DONE']) = 0
      AND count(<-has_variant[WHERE in.status != 'DONE']) = 0;
};

-- ============================================================================
-- EVENTS
-- ============================================================================

-- Cleanup variants when source resource is deleted
DEFINE EVENT cleanup_variants ON TABLE resource WHEN $event = "DELETE" THEN {
    -- Query variant IDs BEFORE deleting edges
    LET $variant_ids = (SELECT VALUE out FROM has_variant WHERE in = $before.id);

    -- Delete edges
    DELETE has_variant WHERE in = $before.id;
    DELETE depends_on WHERE in = $before.id OR out = $before.id;

    -- Delete variant resources
    DELETE resource WHERE id IN $variant_ids;
};

-- NOTE: cascade_dirty event removed - cascade invalidation handled in Rust
-- Reason: Event-based cascade can cause infinite loops or race conditions
-- Implementation: See lib/src/graph/invalidation.rs::cascade_invalidate()
```

## Status State Machine

Resources transition through the following states:

```
PENDING ──(process)──> DONE
   ↑                     │
   │                     │
   └──(invalidate)── DIRTY

Any ──(error)──> FAILED

FAILED ──(retry)──> PENDING
```

### State Descriptions

**PENDING:**

- Resource has not been processed yet
- New resources start in this state
- Dependencies must be DONE before processing can begin

**DONE:**

- Resource successfully processed and cached
- `last_rendered_hash` matches `content_hash`
- Ready to serve from cache

**DIRTY:**

- Resource content has changed (content_hash ≠ last_rendered_hash)
- Or a dependency was marked DIRTY (cascade invalidation)
- Needs reprocessing

**FAILED:**

- Error occurred during processing
- `error_message` field contains failure details
- Can be retried (transitions to PENDING)

### Valid Transitions

| From | To | Trigger | Validation |
|------|----|---------|-----------

|
| PENDING | DONE | Processing succeeded | All dependencies DONE |
| PENDING | FAILED | Processing error | Error message required |
| PENDING | DIRTY | Content changed before processing | Valid transition |
| DONE | DIRTY | Content changed or dependency invalidated | Common case |
| DIRTY | DONE | Reprocessing succeeded | All dependencies DONE |
| DIRTY | FAILED | Reprocessing error | Error message required |
| FAILED | PENDING | Retry requested | Valid recovery path |

### Transition Validation (Rust Implementation)

The Rust type system should encode these transition rules:

```rust
// In lib/src/types/resource_v2.rs
impl ResourceStatus {
    pub fn can_transition_to(&self, next: &ResourceStatus) -> bool {
        use ResourceStatus::*;
        matches!(
            (self, next),
            (Pending, Done) | (Pending, Failed) | (Pending, Dirty) |
            (Dirty, Done) | (Dirty, Failed) |
            (Done, Dirty) |
            (Failed, Pending) |
            // No-op transitions (same state)
            (Pending, Pending) | (Done, Done) | (Dirty, Dirty) | (Failed, Failed)
        )
    }

    pub fn transition_to(self, next: ResourceStatus) -> Result<ResourceStatus, InvalidTransition> {
        if self.can_transition_to(&next) {
            Ok(next)
        } else {
            Err(InvalidTransition { from: self, to: next })
        }
    }
}
```

## Graph Edges and Relationships

### Dependency Edge (depends_on)

**Semantic:** "Child resource depends on parent resource"

**Direction:** `child -> depends_on -> parent`

**Example:**

```surrealql
-- Document A transcludes document B
RELATE resource:doc_a -> depends_on -> resource:doc_b
SET reference_type = 'transclusion', required = true;

-- Document C includes optional image
RELATE resource:doc_c -> depends_on -> resource:img_1
SET reference_type = 'image', required = false;
```

**Properties:**

- `reference_type`: Type of dependency (transclusion, image, audio, video, data)
- `required`: Whether dependency is required (`!`) or optional (`?`)

**Traversal Patterns:**

```surrealql
-- Find all dependencies of a document (outgoing)
SELECT * FROM resource:doc_a -> depends_on -> resource;

-- Find all dependents of a document (incoming)
SELECT * FROM resource <- depends_on <- resource:doc_b;

-- Recursive dependency tree (up to 20 levels)
SELECT * FROM resource:doc_a.{1..20}(-> depends_on -> resource);
```

### Derivative Edge (has_variant)

**Semantic:** "Source resource has an optimized variant"

**Direction:** `source -> has_variant -> variant`

**Example:**

```surrealql
-- Original image has WebP variant at 640px width
RELATE resource:img_original -> has_variant -> resource:img_640_webp
SET width = 640, format = 'webp';

-- Original image has AVIF variant at 1280px width
RELATE resource:img_original -> has_variant -> resource:img_1280_avif
SET width = 1280, format = 'avif';
```

**Properties:**

- `width`: Target width for responsive image
- `format`: Output format (webp, avif)

**Traversal Patterns:**

```surrealql
-- Find all variants of an image (outgoing)
SELECT * FROM resource:img_original -> has_variant -> resource;

-- Find source of a variant (incoming)
SELECT * FROM resource <- has_variant <- resource:img_640_webp;
```

## Work Scheduling Algorithm

### Iterative State Approach

Instead of calculating the entire work schedule upfront (Kahn's algorithm in memory), we use an **iterative state** approach where the database returns the next batch of ready work.

**Core Query:**

```surrealql
-- Get next batch of ready work
SELECT * FROM resource
WHERE (status = 'PENDING' OR status = 'DIRTY')
  AND count(->depends_on[WHERE out.status != 'DONE']) = 0
  AND count(<-has_variant[WHERE in.status != 'DONE']) = 0;
```

**How it works:**

1. **Iteration 1**: Returns leaf nodes (no dependencies, or all dependencies DONE)
2. Process batch in parallel with rayon
3. Mark batch as DONE
4. **Iteration 2**: Returns next layer (dependencies now satisfied)
5. Repeat until query returns empty

**Advantages over in-memory graph:**

- **Resumable**: Database state persists - can resume from interruption
- **Incremental**: Only processes dirty subgraphs, not entire tree
- **Memory efficient**: No need to load entire graph into RAM
- **Database-native**: Leverages SurrealDB's graph traversal optimization

### Rust Implementation Outline

```rust
// In lib/src/graph/scheduler.rs
pub struct WorkScheduler {
    db: Surreal<Db>,
}

impl WorkScheduler {
    pub async fn execute_work_plan<F>(
        &self,
        processor: F
    ) -> Result<WorkStats>
    where
        F: Fn(&ResourceV2) -> Result<()> + Sync + Send,
    {
        let mut stats = WorkStats::default();

        loop {
            // Get next batch from database
            let batch = self.get_next_batch().await?;

            if batch.is_empty() {
                // Check for deadlock
                if self.is_deadlocked().await? {
                    return Err(GraphError::Deadlock {
                        remaining: self.count_pending().await?
                    });
                }
                break; // All work complete
            }

            // Process batch in parallel with rayon
            let results: Vec<_> = batch.par_iter()
                .map(|resource| {
                    processor(resource)
                        .map(|_| resource.resource_hash.clone())
                        .map_err(|e| (resource.resource_hash.clone(), e))
                })
                .collect();

            // Separate successes and failures
            let (successes, failures): (Vec<_>, Vec<_>) =
                results.into_iter().partition_result();

            // Update database
            if !successes.is_empty() {
                self.mark_batch_done(&successes).await?;
                stats.successful += successes.len();
            }

            if !failures.is_empty() {
                self.mark_batch_failed(&failures).await?;
                stats.failed += failures.len();
            }
        }

        Ok(stats)
    }

    async fn get_next_batch(&self) -> Result<Vec<ResourceV2>> {
        self.db
            .query(QUERY_GET_READY_WORK)
            .await?
            .take(0)
    }

    async fn mark_batch_done(&self, hashes: &[ResourceHash]) -> Result<()> {
        self.db
            .query("UPDATE resource SET status = 'DONE', last_rendered_hash = content_hash WHERE resource_hash IN $hashes")
            .bind(("hashes", hashes))
            .await?;
        Ok(())
    }

    async fn is_deadlocked(&self) -> Result<bool> {
        let count: i64 = self.db
            .query("SELECT count() FROM resource WHERE status IN ['PENDING', 'DIRTY']")
            .await?
            .take(0)?;
        Ok(count > 0)
    }
}
```

## Cache Invalidation Flows

### Content-Based Invalidation

When a resource's content changes, its `content_hash` will differ from `last_rendered_hash`:

```rust
// In lib/src/graph/invalidation.rs
pub async fn invalidate_resource(db: &Surreal<Db>, resource_hash: &ResourceHash) -> Result<()> {
    // Mark resource as DIRTY
    db.query("UPDATE resource SET status = 'DIRTY' WHERE resource_hash = $hash")
        .bind(("hash", resource_hash.as_str()))
        .await?;

    Ok(())
}
```

### Cascade Invalidation

When a resource is invalidated, all resources that depend on it must also be invalidated:

```rust
// In lib/src/graph/invalidation.rs
pub async fn cascade_invalidate(db: &Surreal<Db>, resource_hash: &ResourceHash) -> Result<()> {
    // Begin transaction for atomicity
    let mut tx = db.begin_transaction().await?;

    // Mark resource dirty
    tx.query("UPDATE resource SET status = 'DIRTY' WHERE resource_hash = $hash")
        .bind(("hash", resource_hash.as_str()))
        .await?;

    // Get all dependents via graph traversal (up to 20 levels)
    let dependents: Vec<RecordId> = tx.query(r#"
        SELECT VALUE id FROM (
            SELECT * FROM resource WHERE resource_hash = $hash
        ).{1..20}(<-depends_on<-resource)
    "#)
    .bind(("hash", resource_hash.as_str()))
    .await?
    .take(0)?;

    // Mark all dependents dirty
    if !dependents.is_empty() {
        tx.query("UPDATE resource SET status = 'DIRTY' WHERE id IN $ids")
            .bind(("ids", dependents))
            .await?;
    }

    // Commit transaction
    tx.commit().await?;
    Ok(())
}
```

**Why in Rust, not SurrealQL event?**

The plan originally included a `cascade_dirty` event, but review feedback identified this as a **critical issue**:

- Event triggers on UPDATE
- Which triggers more UPDATEs
- Potential for infinite loops
- Race conditions in concurrent scenarios

**Solution:** Explicit transaction-based cascade in Rust with depth limiting.

### Time-Based Expiry (Remote Resources)

Remote resources (HTTP-fetched) have time-based expiry:

```rust
// In lib/src/cache/expiry.rs
pub async fn mark_expired_dirty(db: &Surreal<Db>) -> Result<usize> {
    let result = db.query(r#"
        UPDATE resource
        SET status = 'DIRTY'
        WHERE source = 'remote'
          AND time::unix(time::now()) - time::unix(last_checked) > expiry_days * 86400
          AND status = 'DONE'
    "#)
    .await?;

    let count: usize = result.take(0)?;
    Ok(count)
}
```

## Type Safety and Validation

### Newtype Pattern for Identifiers

To prevent mixing up hash types, use newtypes:

```rust
// In lib/src/types/resource_v2.rs

#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(transparent)]
pub struct ResourceHash(String);

#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(transparent)]
pub struct ContentHash(String);

impl ResourceHash {
    pub fn new(path: &Path) -> Self {
        Self(format!("{:016x}", xxh3_64(path.to_string_lossy().as_bytes())))
    }

    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl ContentHash {
    pub fn new(bytes: &[u8]) -> Self {
        Self(format!("{:016x}", xxh3_64(bytes)))
    }

    pub fn as_str(&self) -> &str {
        &self.0
    }
}
```

**Benefits:**

- Compiler prevents mixing resource_hash and content_hash
- Type-safe API signatures
- Self-documenting code

### Multi-Layer Validation Strategy

**Layer 1: Construction** - Newtypes prevent invalid construction

**Layer 2: Database** - ASSERT constraints prevent invalid data

```surrealql
DEFINE FIELD status ON resource TYPE string
    ASSERT $value IN ['PENDING', 'DONE', 'DIRTY', 'FAILED']
    DEFAULT 'PENDING';
```

**Layer 3: Deserialization** - Serde validates on load

```rust
#[derive(Debug, Serialize, Deserialize)]
pub enum ResourceStatus {
    #[serde(rename = "PENDING")]
    Pending,
    #[serde(rename = "DONE")]
    Done,
    #[serde(rename = "DIRTY")]
    Dirty,
    #[serde(rename = "FAILED")]
    Failed,
}
```

## Integration Plan

### Initialization

```rust
// In lib/src/cache/mod.rs
pub async fn init() -> Result<Surreal<Db>> {
    let db = Surreal::new::<RocksDb>("path/to/.composition.db").await?;
    db.use_ns("composition").use_db("cache").await?;

    // Apply schema
    apply_schema_v2(&db).await?;

    // Check schema version
    let version: Option<SchemaVersion> = db.select(("schema_version", "v2")).await?;
    if version.is_none() {
        // Run migration if needed
        migrate_v1_to_v2(&db).await?;
    }

    Ok(db)
}
```

### Graph Building

```rust
// In lib/src/graph/builder_v2.rs
pub async fn build_graph_v2(root: Resource, db: &Surreal<Db>) -> Result<()> {
    // Insert root resource
    insert_resource(db, &root).await?;

    // Recursively process dependencies
    for dep in parse_dependencies(&root)? {
        // Insert dependency resource
        insert_resource(db, &dep).await?;

        // Create edge
        insert_dependency(db, &root, &dep, dep.ref_type, dep.required).await?;

        // Recurse
        build_graph_v2(dep, db).await?;
    }

    Ok(())
}

async fn insert_dependency(
    db: &Surreal<Db>,
    child: &Resource,
    parent: &Resource,
    ref_type: ReferenceType,
    required: bool,
) -> Result<()> {
    db.query("RELATE $child -> depends_on -> $parent SET reference_type = $ref_type, required = $required")
        .bind(("child", child.id()))
        .bind(("parent", parent.id()))
        .bind(("ref_type", ref_type.to_string()))
        .bind(("required", required))
        .await?;
    Ok(())
}
```

### Work Scheduling

```rust
// In lib/src/render/v2.rs
pub async fn render_v2(target: Resource, db: Arc<Surreal<Db>>) -> Result<String> {
    // Build graph
    build_graph_v2(target.clone(), &db).await?;

    // Create scheduler
    let scheduler = WorkScheduler::new(db.clone());

    // Execute work plan
    let stats = scheduler.execute_work_plan(|resource| {
        match resource.type_ {
            ResourceType::Document => process_document(resource, &db),
            ResourceType::Image => process_image(resource, &db),
            ResourceType::Audio => process_audio(resource, &db),
            // ... other types
        }
    }).await?;

    println!("Processed: {} successful, {} failed", stats.successful, stats.failed);

    // Render final document
    render_document(&target, &db).await
}
```

## Performance Considerations

### Query Optimization

**Indexes Required:**

All queries in the work scheduling and invalidation flows rely on these indexes:

- `idx_resource_hash` - O(1) lookup by location
- `idx_status` - Filter by status (PENDING/DIRTY)
- `idx_depends_in`, `idx_depends_out` - Graph traversal
- `idx_cache_validation` - Covering index avoids table lookup

**Query Timeout:**

```rust
const QUERY_TIMEOUT: Duration = Duration::from_secs(30);
const GRAPH_TRAVERSAL_TIMEOUT: Duration = Duration::from_secs(60);

pub async fn get_next_batch(&self) -> Result<Vec<ResourceV2>> {
    timeout(QUERY_TIMEOUT, async {
        self.db
            .query(QUERY_GET_READY_WORK)
            .await?
            .take(0)
    }).await?
}
```

### Graph Traversal Depth Limiting

```surrealql
-- Limit recursive traversal to 20 levels (prevent infinite loops on cycles)
SELECT * FROM resource:root.{1..20}(-> depends_on -> resource);
```

**Cycle Detection:**

Cycles should be detected during graph building, not during traversal:

```rust
// In lib/src/graph/builder_v2.rs
pub async fn detect_cycles(db: &Surreal<Db>, resource: &Resource) -> Result<bool> {
    let result: Vec<RecordId> = db.query(r#"
        SELECT VALUE id FROM $root.{1..20}(-> depends_on -> resource)
        WHERE id = $root
    "#)
    .bind(("root", resource.id()))
    .await?
    .take(0)?;

    Ok(!result.is_empty())
}
```

### Batch Processing

For large graphs (1000+ resources), batch status updates:

```rust
const BATCH_SIZE: usize = 100;

pub async fn mark_batch_done(&self, hashes: &[ResourceHash]) -> Result<()> {
    for chunk in hashes.chunks(BATCH_SIZE) {
        self.db
            .query("UPDATE resource SET status = 'DONE', last_rendered_hash = content_hash WHERE resource_hash IN $hashes")
            .bind(("hashes", chunk))
            .await?;
    }
    Ok(())
}
```

## Migration Strategy

See [Schema Evolution](../reference/schema-evolution.md) for complete migration details.

**High-level steps:**

1. Apply v2 schema (additive - doesn't break v1)
2. Migrate documents from `document` table to `resource` table
3. Migrate images from `image_cache` to `resource`
4. Migrate audio, LLM, embeddings
5. Validate counts match
6. Optionally drop v1 tables (or keep for rollback)

**Migration is transactional:**

```rust
let mut tx = db.begin_transaction().await?;

// Apply all migrations
migrate_documents(&tx).await?;
migrate_images(&tx).await?;
// ... other migrations

// Validate
if !validate_migration(&tx).await? {
    tx.cancel().await?;
    return Err(MigrationError::ValidationFailed);
}

// Commit all changes
tx.commit().await?;
```

## Open Design Questions

**Resolved in Plan Review:**

1. ✅ **Schema version tracking** - YES, add schema_version table
2. ✅ **HNSW index inclusion** - YES, include MTREE vector index in v2
3. ✅ **Transaction granularity** - Single transaction with timeout, batch if >100 resources
4. ✅ **Status history** - NO for v2, add in v3 if debugging needs arise
5. ✅ **Resource retention** - Soft delete with 30-day TTL
6. ✅ **Variant table strategy** - Unified resource table
7. ✅ **LLM expiry configuration** - Per-operation defaults (summarize: 30d, consolidate: 14d, topic: 60d)

**Remaining for Implementation:**

1. **Garbage collection frequency** - How often to run cleanup of soft-deleted resources?
2. **Cache size limits** - Should we enforce max database size? How to handle?
3. **Resumable build UI** - How to surface "resuming from step X" to user?
4. **Deadlock recovery** - Manual intervention? Automatic cycle-breaking?

## Conclusion

The graph-based caching architecture fully leverages SurrealDB's multi-model capabilities to create a unified, persistent, and resumable caching system. By treating all resources as graph nodes and dependencies as edges, we enable:

- **Database-native graph operations** instead of in-memory graph algorithms
- **Incremental builds** via status tracking (PENDING/DONE/DIRTY/FAILED)
- **Automatic cascade invalidation** through graph traversal
- **Derivative tracking** for optimized assets
- **Type safety** through Rust newtypes and database constraints

The design is ready for implementation starting with Phase 2: Core Schema Implementation.

**Next Steps:**

1. Implement Phase 2: `lib/src/cache/schema_v2.rs`, `migration.rs`, `resource_ops.rs`, `resource_v2.rs`
2. Implement Phase 3: `lib/src/graph/builder_v2.rs`, `scheduler.rs`, `invalidation.rs`
3. Implement Phase 4: `lib/src/cache/lookup.rs`, `storage.rs`, `expiry.rs`
4. Implement Phase 5: Resource handlers (document, image, audio, LLM)
5. Implement Phase 6: Integration tests and performance validation

**Related Documentation:**

- [SurrealQL Query Reference](../reference/surrealql-queries.md)
- [Schema Evolution Guide](../reference/schema-evolution.md)
- [Caching Strategy](./caching-strategy.md)
- [Work Scheduling](../reference/work-scheduling.md)
