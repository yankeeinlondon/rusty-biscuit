---
last_updated: 2026-03-23
sources:
  - darkmatter/features/2026-03-23. caching/alt-caching-design.md
  - darkmatter/docs/topics/caching.md
---

# Compose Pipeline Caching Design

This document defines the implementation-ready caching design for Darkmatter's compose pipeline. It merges the stronger architectural ideas from the alternative design with the stronger operation-modeling and cache-policy ideas from the caching topic document.

The result is a design with four core properties:

1. it treats document caching and transclusion caching as related but distinct concerns
2. it optimizes both repeated work inside one compose run and repeated work across compose runs
3. it caches the expensive core of a transclusion while leaving cheap parent-specific transforms outside the persistent key
4. it keeps v1 simple with a file-backed cache while preserving a clean path to SQLite later

## Purpose

Darkmatter's compose pipeline has three execution phases:

1. inline pre
2. transclusion
3. inline post

The transclusion phase is the part most likely to benefit from caching because it is recursive, concurrent, and increasingly responsible for expensive operations such as remote fetches and future LLM-backed transforms.

Caching therefore needs to solve two different problems:

1. avoid recomputing the same work within a single compose run
2. avoid recomputing stable expensive artifacts across compose runs

## Goals

1. Avoid recomposing the same child document multiple times in one compose run.
2. Avoid rerunning expensive transclusion operations across runs.
3. Use `biscuit-hash` for both identity hashing and context-aware content hashing.
4. Keep v1 debuggable and low-complexity.
5. Make invalidation dependency-aware without requiring a graph database.
6. Preserve a clean migration path to SQLite if the file backend later becomes limiting.

## Non-Goals

1. Building a shared remote cache service.
2. Solving every media or asset cache problem in the first implementation.
3. Persisting partial or failed compose output as durable hits.
4. Turning the cache into a full graph scheduler or provenance database.
5. Introducing a database backend before the key model and artifact boundaries are proven.

## Core Recommendation

Darkmatter should use a two-layer cache:

1. a run-local in-memory cache runtime with single-flight behavior
2. a persistent file-backed artifact cache stored by default at `<workspace-root>/.darkmatter/cache/`

If the persistent backend later needs to evolve, the next backend should be SQLite, not SurrealDB. The cache does not require graph-native queries in v1 because recursive invalidation can be represented with manifests and dependency closure hashes.

## Design Principles

### Separate document caching from transclusion caching

A raw document is not the same thing as a transclusion result.

A document depends on:

- source identity
- raw source bytes
- frontmatter
- body content

A transclusion result depends on all of the above plus:

- directive kind
- directive parameters
- effective inherited state
- compose context
- compose options
- downstream dependency state

Document snapshots are therefore building blocks. The primary expensive artifact is the transclusion core result.

### Cache expensive core work, not cheap insertion work

For `::file`, the expensive part is recursively composing the child document. The cheap part is parent-specific shaping after the child has already been composed.

That means the persistent cache should target the child document core first, then apply insertion-specific transforms after a hit, such as:

- `exclude`
- wrappers such as quotation or disclosure rendering
- heading releveling at the insertion point

The same principle should guide other transclusion operations.

### Favor correctness over maximum hit rate in v1

The first implementation should intentionally over-key some state rather than risk false cache hits. Once operation boundaries are stable, the key model can be narrowed.

## Architecture

### Layer 0: Run-Local Cache Runtime

This layer lives inside `PipelineRuntime` for a single compose execution.

Darkmatter already has a small in-memory runtime cache for:

- canonical-path markdown document loads
- TOC heading loads

That existing behavior should be preserved and generalized into a shared cache runtime owned by `PipelineRuntime` and carried into `clone_for_child()`.

### Responsibilities

1. Deduplicate repeated reads of the same local markdown source.
2. Deduplicate repeated TOC extraction work.
3. Deduplicate repeated child compose work.
4. Suppress repeated failures for the same missing or failing target during one run.
5. Collect cache statistics for the compose report.

### Required behavior

The runtime cache should be single-flight rather than just memoized:

1. the first caller for a key performs the work
2. concurrent callers for the same key wait on the same in-flight result
3. only successful results are eligible for promotion into the persistent cache

This is especially important because transclusion already runs concurrently across sibling branches.

### Layer 1: Persistent Artifact Cache

This layer survives across runs and stores manifests plus payload blobs.

The persistent cache should support three artifact classes:

1. `document_snapshot`
2. `compose_document_core`
3. `operation_result`

These classes have different invalidation rules and different key inputs, but they can share the same backend and manifest conventions.

## Cacheable Units

### 1. Document Snapshot Cache

This is the cheapest persistent unit. Its purpose is to avoid unnecessary rereads, support invalidation, and provide stable building blocks for higher-level caches.

The snapshot stores the document body without frontmatter plus document-derived hashes and metadata.

It should also preserve one important idea from the topic document: the snapshot should distinguish between the raw body and the body's transclusion structure.

### Why two body-derived hashes are needed

The two source documents disagree on how aggressive the body hash should be. The right merged design is to keep two hashes with two different purposes:

1. a conservative body semantic hash for correctness
2. a more structure-oriented template hash for document-level reuse and diagnostics

### Document snapshot fields

```rust
struct DocumentSnapshotManifest {
    cache_version: u16,
    source_kind: SourceKind,        // local_file | remote_url
    canonical_source: String,
    source_id_hash: u64,
    raw_bytes_hash: u64,
    frontmatter_hash: u64,
    body_semantic_hash: u64,
    body_template_hash: u64,
    modified_at: Option<SystemTime>,
    size_bytes: u64,
}
```

### `body_semantic_hash`

This hash is the correctness-oriented hash used for invalidation of composed artifacts. It should:

- exclude frontmatter
- retain directive text as written
- use `biscuit-hash` with conservative normalization

Recommended variants:

- `HashVariant::BlockTrimming`
- `HashVariant::LeadingWhitespace`
- `HashVariant::TrailingWhitespace`

This deliberately does not use `InteriorWhitespace` for whole-document invalidation because that is too aggressive for:

- inline code
- fenced code blocks
- tables
- directive arguments
- hard-break-sensitive prose

### `body_template_hash`

This hash keeps the "lean document cache" idea from the topic document.

It should be computed from the body after replacing transclusion directives with indexed placeholders that preserve:

- operation kind
- position/order
- inline vs block placement

Examples of the placeholder concept:

- block transclusion placeholder: `::transclusion:file:3`
- inline transclusion placeholder: `{{transclusion:toc-linking:1}}`

This hash is not the primary correctness hash. Its purpose is to capture document structure while removing sensitivity to large directive parameter variance.

Because it is a structure-oriented hash rather than a final-output invalidation hash, it may use more aggressive normalization, including `InteriorWhitespace`, when doing so is safe for the placeholder-normalized body.

This lets Darkmatter answer questions like:

- "is this document structurally the same shape as before?"
- "did the count or kind of transclusions change?"

without making the entire cache depend on literal directive argument strings.

### 2. Composed Document Core Cache

This is the main cache for markdown transclusion.

It represents the child document after its own compose pipeline completes, but before the parent applies cheap insertion-specific transforms.

For `render_markdown_transclusion()`, this means the method should be split conceptually into:

1. load or compute child composed core
2. apply parent insertion transforms

### Manifest shape

```rust
struct ComposedDocumentManifest {
    cache_version: u16,
    entry_key: u64,
    op_kind: ComposeOpKind,         // compose_document_core
    self_hash: u64,
    closure_hash: u64,
    dependency_count: usize,
    dependencies: Vec<DependencyRef>,
    payload_blob_hash: u64,
    warnings_hash: Option<u64>,
    created_at: SystemTime,
    last_accessed_at: SystemTime,
    expires_at: Option<SystemTime>,
}
```

### Key inputs

The composed document core key should include:

- source identity hash
- document snapshot identity
- document snapshot body/frontmatter hashes
- effective inherited state hash
- compose context hash
- compose options hash for output-affecting options

The key should not include cheap parent-only transforms such as:

- exclusion slicing
- wrappers
- heading releveling tied to insertion context

### 3. Operation Result Cache

This cache holds expensive transclusion results that are not simply "compose a child markdown document".

Examples:

- `::code`
- `::toc-linking`
- remote markdown fetch
- prompt expansion
- summarization
- consolidation

This artifact class should be designed around explicit operation modeling so the cache stays generic while operations remain responsible for defining what actually varies the expensive work.

## Operation Model and Parameter Buckets

The strongest idea from the topic document is that transclusion parameters should not all be treated the same.

Every cacheable transclusion should classify its parameters into four buckets:

1. `conditional`
2. `pre`
3. `variant`
4. `post`

### Conditional params

These decide whether the transclusion renders at all.

Examples:

- `when="..."`

They are evaluated before the cache lookup and are not part of the persistent cache key.

### Pre params

These affect a cheap preprocessing step that occurs before the expensive core work but do not need their own persistent artifact.

They are allowed to shape the prepared input to the core operation, but the cache only cares about the normalized prepared input hash, not the original parameter spelling.

### Variant params

These change the expensive core artifact itself. They must be included in the persistent cache key.

### Post params

These are cheap transforms applied after the core artifact is loaded or computed. They should stay out of the persistent key whenever correctness allows.

### Recommended abstraction

Darkmatter should formalize this with an internal transclusion trait or equivalent internal abstraction:

```rust
trait CacheableTransclusion {
    fn split_params(&self) -> ParamBuckets;
    fn evaluate_conditions(&self, state: &EffectiveState, ctx: &ComposeContext) -> bool;
    fn prepare(&self, state: &EffectiveState, options: &ComposeOptions) -> PreparedInput;
    fn core_cache_key(&self, prepared: &PreparedInput) -> CacheKey;
    fn compute_core(&self, prepared: &PreparedInput) -> Result<CoreArtifact>;
    fn postprocess(&self, core: CoreArtifact, post: &PostParams) -> Result<String>;
}
```

The point is not the exact trait shape. The point is to force every transclusion operation to state:

- what is cheap
- what is expensive
- what actually varies the expensive artifact

### Operation examples

`::file`

- conditional: `when`
- variant: referenced source, inherited state, replace behavior that changes child composition
- post: `exclude`, wrappers, insertion-based releveling

`::toc-linking`

- conditional: `when`
- variant: referenced source plus TOC formatting options that materially change emitted links
- post: wrappers

`::code`

- conditional: `when`
- variant: referenced source, effective replace map when replacement changes emitted code text
- post: wrappers and spacing normalization

Future LLM-backed operations

- conditional: `when`
- variant: input text hash, prompt template hash, provider, model, revision, generation params, relevant effective state
- post: wrappers and presentation formatting

## Hashing Strategy

Darkmatter should use `biscuit-hash` for both exact identity hashing and context-aware semantic hashing.

### Identity hashes

Use raw `xxHash` for source identity:

- canonical absolute path for local files
- normalized URL string for remote references

This answers "what resource is this?"

### Content hashes

### Raw bytes hash

Use raw `xxHash` over the exact source bytes for exact-change detection and diagnostics.

### Frontmatter hash

Frontmatter should be serialized into canonical JSON with recursively sorted keys and then hashed with raw `xxHash`.

That ensures:

- YAML key order changes do not invalidate
- formatting-only changes do not invalidate
- actual data changes do invalidate

### Body semantic hash

Use the conservative `biscuit-hash` variants described earlier.

### Body template hash

Use the placeholder-normalized body described earlier.

### State and context hashes

The output of a child document depends on more than the child file itself. For correctness, v1 should hash the fully materialized effective inputs:

- effective inherited state
- child frontmatter after merge/override rules
- compose context

These should be canonicalized through JSON serialization and then hashed.

This is intentionally broad in v1. Later Darkmatter can narrow the key to only the state actually referenced by interpolation, conditions, or operation config.

### Options hash

Only options that materially affect emitted output should enter the key.

Examples:

- enabled compose operations
- local vs remote transclusion allow/deny settings
- `ignore_invalid_references` when it affects emitted fallback content
- cleanup settings that alter final markdown shape
- `code_fallback_language`

Purely diagnostic or control-flow options should stay out of the persistent key unless they can change emitted output.

## Dependency-aware invalidation with closure hashes

Every persistent artifact should store:

- `self_hash`
- `closure_hash`

`self_hash` captures the artifact's direct inputs.

`closure_hash` captures the artifact plus the current closure hashes of its direct dependencies:

```text
closure_hash = xx_hash(
  canonical_json({
    "self": self_hash,
    "deps": sort_by_key(dependency_key, dependency_closure_hashes)
  })
)
```

This provides Merkle-style invalidation without requiring a graph database:

1. a child changes
2. the child's closure hash changes
3. the parent dependency check sees a mismatch
4. the parent entry becomes stale

## Validation, Expiry, and Freshness

The merged design should distinguish three concerns that were mixed together in the source documents:

1. whether the cache is enabled for reads and writes
2. whether an entry is considered fresh
3. what to do when a stale or missing entry cannot be regenerated

### Access mode

This controls read/write behavior:

```rust
enum CacheAccessMode {
    Off,
    ReadOnly,
    ReadWrite,
    Refresh,
}
```

Recommended meanings:

- `Off`: skip both run-local and persistent caching except for any minimal internal correctness caches that already exist
- `ReadOnly`: read caches but do not write new entries
- `ReadWrite`: normal mode
- `Refresh`: bypass persistent hits, recompute, and write back fresh entries

### Freshness / failure mode

This keeps the stronger operational policy model from the topic document:

```rust
enum CacheFreshnessMode {
    Strict,
    Fallback,
    Optimistic,
    Forced,
}
```

Recommended meanings:

- `Strict`: expired entries must be regenerated; failure is an error
- `Fallback`: try to regenerate expired entries, but fall back to stale cached content when regeneration fails; missing uncached content is still an error
- `Optimistic`: treat any present cache entry as a hit regardless of expiry; generate only for cache misses
- `Forced`: same as optimistic for hits, but if an uncached generation fails, warn and replace the directive with empty output; this should never be used for production content

These modes should be orthogonal to access mode.

### Lifetime defaults

Not every artifact class should use the same TTL policy.

### Local documents

Local documents should have no time-based TTL.

They are valid while:

- the source identity still resolves
- the snapshot still matches
- the `self_hash` still matches
- all dependency closure hashes still match

### Remote references

Remote references should use both content validation and time-based revalidation.

Recommended defaults:

- remote markdown or data transclusion: `6h`
- remote media metadata: `24h`

If available later, HTTP validators such as `ETag` and `Last-Modified` should be stored in the manifest and used during revalidation.

### LLM-backed operations

LLM-backed operations should be cached aggressively because they are the most expensive transclusions in the pipeline.

Recommended rules:

- pinned model/version: content-addressed only, no TTL required
- floating model alias: use a TTL, default `7d`

The key should include:

- operation kind
- input text hash
- prompt template hash
- provider hash
- model identifier
- model revision or digest when available
- generation parameters that affect output
- relevant effective state when interpolation changes the prompt

### Per-operation overrides

The default freshness lifetime should be derived from the operation kind and the reliability/cost of its environment. Individual operations should be allowed to override that default through configuration.

### Negative caching

Persistent negative caching should be avoided in v1 for local resources. A missing file now may exist seconds later.

If needed, negative caching should be in-memory only and scoped to the current compose run.

## Storage Layout

### Default location

The persistent cache should be workspace-local by default:

```text
<workspace-root>/.darkmatter/cache/
```

This is the right default because:

- relative transclusion is workspace-sensitive
- repo-root resolution is workspace-sensitive
- local shell and security policy is workspace-sensitive
- LLM outputs are usually most valuable inside the project that produced them
- cleanup is straightforward

For compose runs outside a repository, Darkmatter should fall back to the OS cache directory via `dirs::cache_dir()`.

### On-disk layout

```text
.darkmatter/cache/
  v1/
    manifests/
      document_snapshot/
      compose_core/
      operation/
    blobs/
      md/
      txt/
      json/
    locks/
    gc/
```

Example paths:

```text
.darkmatter/cache/v1/manifests/compose_core/ab/cd/abcdef0123456789.json
.darkmatter/cache/v1/manifests/operation/12/34/1234567890abcdef.json
.darkmatter/cache/v1/blobs/md/fe/dc/fedcba9876543210.md
.darkmatter/cache/v1/blobs/json/98/76/9876543210abcdef.json
.darkmatter/cache/v1/locks/abcdef0123456789.lock
```

Recommended rules:

- use hash fanout directories
- store manifests separately from payload blobs
- keep manifests human-readable JSON
- keep blobs readable in v1 (`.md`, `.txt`, `.json`)
- add compression later only if needed

### Concurrency and atomicity

The file backend should be conservative and robust:

1. acquire a per-entry lock file
2. write manifest and payload to temporary files
3. `rename()` them into place atomically
4. release the lock

This is sufficient for the expected local workflow and keeps the failure model easy to reason about.

### Eviction and garbage collection

V1 can use best-effort GC rather than a complex eviction subsystem.

Recommended metadata:

- `created_at`
- `last_accessed_at`
- `payload_size`
- optional `expires_at`

Recommended policies:

- optional total size cap
- optional max age for expirable artifact classes
- orphan blob cleanup

## Integration Points in Darkmatter

### `PipelineRuntime`

`PipelineRuntime` should gain a dedicated cache runtime alongside transclusion and shell state.

That runtime should own:

- existing run-local markdown and TOC caches
- the new in-flight single-flight registry
- optional in-memory negative cache
- persistent backend handle
- cache statistics

The shared cache runtime should survive `clone_for_child()` so sibling branches benefit from the same deduplication.

### `render_markdown_transclusion()`

This function is the highest-value first integration point.

It should be refactored conceptually into:

1. resolve child source and build effective child state
2. load or compute the composed child core through the cache runtime
3. apply cheap parent-specific transforms
4. merge cache statistics into `ComposeReport`

This is the most direct way to keep key cardinality low while still caching the expensive recursive work.

### Other initial hook points

After markdown transclusion, the next most valuable hook points are:

1. TOC extraction
2. code transclusion
3. future prompt expansion
4. future summarization
5. future consolidation

### Compose API surface

The public compose API has already moved toward flatter options. Cache configuration should follow that direction.

Recommended new compose options:

- `cache_access_mode`
- `cache_freshness_mode`
- `cache_root`
- `cache_namespace`
- `cache_llm_ttl`

`cache_namespace` gives callers a clean way to isolate incompatible cache populations for testing, experiments, or future prompt/model changes.

### Compose report

`ComposeReport` should expose cache observability:

- `cache_hits`
- `cache_misses`
- `cache_writes`
- `cache_revalidations`
- `cache_stale_hits`
- `cache_skipped`

This makes it much easier to reason about whether the cache is helping and why a run behaved the way it did.

## Recommended Implementation Order

### Phase 1: Generalize the existing run-local cache

1. Keep the current path-based markdown and TOC caches.
2. Move them under a dedicated cache runtime.
3. Add single-flight behavior for child compose work.
4. Add cache stats to `ComposeReport`.

This phase delivers immediate value and aligns with the current `PipelineRuntime` shape.

### Phase 2: Add the persistent file-backed cache

1. Add workspace-local cache root resolution.
2. Add document snapshot manifests.
3. Add composed document core manifests and blobs.
4. Add Merkle-style dependency tracking and validation.
5. Promote only successful results into persistent storage.

This phase should begin with `::file` and `::toc-linking`.

### Phase 3: Formalize operation-level caching

1. Introduce the transclusion parameter buckets.
2. Add operation-result cache support for `::code`.
3. Refactor transclusion implementations around pre/core/post boundaries.

This phase preserves the "cache the expensive core" idea consistently across operations.

### Phase 4: Add remote and LLM cache policies

1. Add TTL-based revalidation for remote sources.
2. Store HTTP validators where available.
3. Add pinned-vs-floating model rules for LLM operations.
4. Add the access and freshness modes to the public API.

### Phase 5: Revisit backend only if the file backend proves limiting

If the file backend later becomes a bottleneck due to metadata query volume, small-file overhead, or concurrent access patterns, the next backend should be SQLite.

SQLite is a better fit than SurrealDB because it provides:

- one embedded file
- low operational complexity
- strong local tooling
- sufficient indexing for manifests and eviction queries
- a natural upgrade path from the same manifest model

## Final Recommendation

Darkmatter should land caching as a layered system:

1. preserve and extend the current run-local runtime cache
2. add a persistent workspace-local artifact cache
3. separate document snapshots from transclusion core artifacts
4. model operation parameters as conditional, pre, variant, and post
5. use both a conservative body semantic hash and a structure-oriented template hash
6. use Merkle-style closure hashes for dependency-aware invalidation
7. separate cache access mode from freshness/failure mode
8. keep v1 file-based and revisit the backend only after the key model is proven
