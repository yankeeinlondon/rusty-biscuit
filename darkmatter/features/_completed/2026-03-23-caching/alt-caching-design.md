# Alternative Caching Design For Documents And Transclusions

## Recommendation

Darkmatter should use a **two-layer cache**:

1. a **run-local in-memory memoization layer** that deduplicates repeated work inside one compose run
2. a **persistent file-backed cache** stored by default at **`<workspace-root>/.darkmatter/cache/`**

If the persistent cache later outgrows a file-based implementation, the next backend should be **SQLite**, not SurrealDB. The recursive graph shape of transclusion can be represented with manifests and dependency hashes without requiring a graph database in v1.

## Why This Fits Darkmatter

Darkmatter's composition pipeline is:

1. inline pre
2. transclusion
3. inline post
4. rendering

The expensive part is the transclusion stage because it is recursive, can fan out concurrently, and will soon include LLM-backed operations. That means the cache needs to optimize two different problems:

- repeated work **within the same compose run**
- repeated work **across compose runs**

It also means "document cache" and "transclusion cache" cannot be treated as the same thing:

- a **document** has its own source content and frontmatter
- a **transclusion result** also depends on inherited state, compose options, directive options, context, and downstream dependencies

The cache should therefore treat the document as a building block and the transclusion result as the primary expensive artifact.

## Goals

- Avoid recomposing the same child document multiple times in one parent compose.
- Avoid rerunning expensive transclusion operations across compose runs.
- Use `biscuit-hash` for fast identity and invalidation hashes.
- Keep the first implementation simple enough to land without a database.
- Preserve a clean migration path to SQLite.
- Make invalidation dependency-aware without requiring a graph database.

## Non-Goals

- Full graph scheduling in the cache layer.
- Persisting partial or failed compose output as durable cache hits.
- Solving every asset cache problem at once.
- Building a shared remote cache service.

## Proposed Architecture

### Layer 0: Run-Local Memoization

This layer lives only for a single compose run and sits inside the pipeline runtime.

It should deduplicate work keyed by a stable operation key, for example:

- compose of the same child markdown document with the same inherited state
- TOC extraction of the same target
- future LLM summarization/consolidation of the same input

This is the highest-value first step because the current compose pipeline can revisit the same target multiple times inside one parent document or across sibling branches.

The runtime behavior should be **single-flight**, not just a plain map:

- first caller for a key performs the work
- concurrent callers for the same key wait on the same in-flight result
- only successful results are promoted into the persistent cache

### Layer 1: Persistent Artifact Cache

This layer survives across runs and stores cache manifests plus payload blobs.

The persistent cache should store three artifact classes:

| Artifact | Purpose | Persistent? |
|----------|---------|-------------|
| Document snapshot | Parsed source identity and hashes for a raw document | Yes |
| Composed document core | The fully composed child markdown before cheap parent-specific wrapping/releveling | Yes |
| Operation result | Expensive transclusion outputs such as TOC extraction, prompt expansion, summarization, consolidation | Yes |

The important split is that `::file` should primarily hit the **composed document core** cache, then apply cheap parent-specific transforms after the hit:

- `exclude`
- quotation/disclosure wrappers
- heading releveling based on insertion point

That keeps key cardinality down while still caching the expensive recursive child compose.

## Cacheable Units

### 1. Document Snapshot Cache

This is the cheapest durable unit and should be used to avoid unnecessary rereads/reparses.

Suggested payload:

```rust
struct DocumentSnapshotManifest {
    cache_version: u16,
    source_id_hash: u64,
    source_kind: SourceKind,      // local_file | remote_url
    canonical_source: String,
    raw_bytes_hash: u64,
    frontmatter_hash: u64,
    body_semantic_hash: u64,
    directive_hash: u64,
    modified_at: Option<SystemTime>,
    size_bytes: u64,
}
```

This cache does **not** need to store a serialized AST in v1. A string-first design is simpler and is enough to support invalidation.
The snapshot hash itself should be derived from the stable fields in this manifest.

### 2. Composed Document Core Cache

This is the main cache for markdown transclusion.

It should represent:

- the child document after its own compose pipeline finishes
- before parent insertion-specific transforms are applied

Suggested key inputs:

- source identity hash
- document snapshot hash
- effective inherited state hash
- compose context hash
- compose options hash for options that materially affect output

Suggested payload:

```rust
struct ComposedDocumentManifest {
    cache_version: u16,
    entry_key: u64,
    op_kind: ComposeOpKind,       // compose_document_core
    self_hash: u64,
    closure_hash: u64,
    dependency_count: usize,
    dependencies: Vec<DependencyRef>,
    payload_blob_hash: u64,
    warnings_hash: Option<u64>,
    created_at: SystemTime,
    last_accessed_at: SystemTime,
}
```

### 3. Operation Result Cache

This is for non-trivial transclusion operations whose result is not just "compose a child markdown file".

Examples:

- `::code` when replace maps are applied
- TOC extraction
- prompt expansion
- summarization
- consolidation
- future directory-style transclusion operations

Suggested operation kinds:

- `code_transclusion`
- `toc_transclusion`
- `prompt_expansion`
- `summarization`
- `consolidation`

## Hashing Strategy

### Identity Hashes

Use raw `xx_hash` for **identity**, not semantic equivalence:

- canonical absolute path for local files
- normalized URL string for remote references

This is the stable "what resource is this?" key.

### Content Hashes

Use different hashes for different responsibilities.

#### Raw Bytes Hash

Use raw `xx_hash_bytes` of the source bytes for exact change detection and diagnostics.

#### Frontmatter Hash

Frontmatter should be normalized into canonical JSON with recursively sorted keys, then hashed with raw `xx_hash`.

That gives these properties:

- key order changes do not invalidate
- YAML formatting differences do not invalidate
- actual value changes do invalidate

#### Markdown Body Semantic Hash

For the body content, use `xx_hash_variant` with:

- `HashVariant::BlockTrimming`
- `HashVariant::LeadingWhitespace`
- `HashVariant::TrailingWhitespace`
- `HashVariant::BlankLine`

This is the right starting point for markdown-level cache invalidation because it ignores the most common meaningless edits while staying conservative.

#### Do Not Use `InteriorWhitespace` For Whole Markdown Bodies

`InteriorWhitespace` is too aggressive for a document-wide hash because it can erase meaningful changes in:

- inline code
- fenced code blocks
- tables
- directive arguments
- hard-break-sensitive prose

It is still useful for narrower operation-specific inputs when the syntax guarantees that collapsing interior whitespace is safe, but it should not be part of the default whole-document body hash.

### State Hashes

The output of a child document depends on more than the child's own frontmatter. It depends on the **effective state** passed into that child:

- inherited parent state
- child frontmatter
- `set_overrides`
- replace semantics
- captured compose context

For correctness, v1 should hash the **entire effective state** and **entire compose context** after canonical JSON serialization.

That is slightly over-broad, but it is safe. Later we can narrow the state hash to only the keys actually referenced by interpolation, conditions, and operation options.

### Options Hashes

Only options that materially affect output should enter the key. Examples:

- enabled compose operations
- `allow_remote_transclusion`
- `allow_local_markdown`
- `allow_local_code`
- `ignore_invalid_references`
- `resolve_repo_root`
- `code_fallback_language`
- cleanup settings that affect final markdown shape

Options that only change control flow or diagnostics should stay out of the key unless they can alter emitted output.

### Merkle-Style Closure Hash

To support dependency-aware invalidation without a database, each cached entry should store:

- `self_hash`: hash of this operation's direct inputs
- `closure_hash`: hash of `self_hash` plus sorted child dependency closure hashes

Conceptually:

```text
closure_hash = xx_hash(
  canonical_json({
    "self": self_hash,
    "deps": sort_by_key(dependency_key, dependency_closure_hashes)
  })
)
```

This gives a file-based equivalent of graph invalidation:

- child changes
- child closure hash changes
- parent sees mismatch and becomes invalid

That is enough for v1 and v2 without requiring SurrealDB graph traversal.

## Cache Validation Rules

### Local Documents

Local document cache entries should have **no time-based TTL**.

They remain valid if:

- the source identity still resolves
- the document snapshot still matches
- the operation `self_hash` still matches
- all dependency closure hashes still match

### Remote References

Remote references should use both content validation and time-based revalidation.

Suggested defaults:

- remote markdown or data transclusion: `6h`
- remote media metadata: `24h`

If later implemented, HTTP validators such as `ETag` and `Last-Modified` should be stored in the manifest.

### LLM-Backed Operations

LLM-backed transclusions should be cached aggressively because they are the most expensive operations in the pipeline.

Their key should include:

- operation kind
- input text hash
- prompt template hash
- provider hash
- model identifier
- model revision or digest if available
- generation parameters that affect output
- effective state hash if interpolation contributes to the prompt

Suggested freshness rule:

- **pinned model/version**: content-addressed only
- **floating model alias**: apply a TTL, default `7d`

### Negative Caching

Persistent negative caching should be avoided in v1 for local resources. A missing file today may exist seconds later.

If needed, use **in-memory-only negative caching** during a single compose run to suppress repeated failures for the same missing target.

## Storage Layout

### Default Location

The persistent cache should be **workspace-local by default**:

```text
<workspace-root>/.darkmatter/cache/
```

Why workspace-local is the right default:

- relative transclusion resolution is workspace-sensitive
- repo-root resolution is workspace-sensitive
- shell policy and future local execution policies are workspace-sensitive
- LLM outputs tied to a document graph are usually more valuable inside the project that produced them
- cleanup is easy: remove one directory

For ad hoc compose runs outside a repository, fall back to the OS cache directory:

- Linux: `$XDG_CACHE_HOME/darkmatter/...` or `~/.cache/darkmatter/...`
- macOS: `~/Library/Caches/darkmatter/...`
- Windows: `%LOCALAPPDATA%\\darkmatter\\...`

That fallback should be resolved with `dirs::cache_dir()`.

### On-Disk Layout

Suggested layout:

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
    reverse/
    locks/
    gc/
```

More concretely:

```text
.darkmatter/cache/v1/
  manifests/compose_core/ab/cd/abcdef0123456789.json
  manifests/operation/12/34/1234567890abcdef.json
  blobs/md/fe/dc/fedcba9876543210.md
  blobs/json/98/76/9876543210abcdef.json
  locks/abcdef0123456789.lock
```

Notes:

- Use hash fanout directories to avoid huge single-directory counts.
- Store metadata and payload separately.
- Large payloads can be compressed later without changing key design.

### File Format

Use:

- JSON for manifests
- raw `.md`, `.txt`, or `.json` payload blobs for readability in v1

This keeps the first implementation debuggable. If size becomes a problem, add compression only to blob files, not to manifest files.

### Concurrency And Atomicity

The file backend should be safe for concurrent readers and conservative concurrent writers.

Recommended write path:

1. acquire a per-entry lock file
2. write manifest and blob to temporary files
3. `rename()` into place atomically
4. release lock

That is enough for the expected local workflow and is much simpler than introducing SQLite or SurrealDB immediately.

### Eviction And Garbage Collection

V1 can use a simple best-effort policy:

- optional total size cap
- optional max age for operation classes with TTL
- periodic orphan-blob cleanup

Manifest fields should therefore include:

- `created_at`
- `last_accessed_at`
- payload size
- optional `expires_at`

## Integration Points In Darkmatter

### Runtime

`PipelineRuntime` should gain a cache runtime alongside transclusion and shell state.

That cache runtime should own:

- in-memory single-flight map
- persistent backend handle
- cache statistics

### Compose Options

The public compose API has recently moved toward flatter options. The cache controls should follow that direction.

Suggested public fields:

- `cache_mode`
- `cache_root`
- `cache_llm_ttl`
- `cache_namespace`

Suggested modes:

- `Off`
- `ReadOnly`
- `ReadWrite`
- `Refresh`

### Compose Report

Add cache observability so callers can understand behavior:

- `cache_hits`
- `cache_misses`
- `cache_writes`
- `cache_revalidations`
- `cache_skipped`

### Pipeline Hooks

The highest-value hook points are:

1. child markdown compose in `render_markdown_transclusion()`
2. TOC extraction
3. future LLM-backed transclusion operations

`render_markdown_transclusion()` should be split conceptually into:

1. load or compute composed child core
2. apply cheap parent-specific transforms

That split is what makes the cache broadly reusable.

## Why File-Based First

The file backend is the right first move because:

- the key/value access pattern is simple
- manifests plus closure hashes already cover dependency invalidation
- debugging is trivial
- failure modes are easy to reason about
- implementation cost is much lower than a database-backed cache

This matters because Darkmatter is still actively shaping its compose and transclusion architecture. Locking into a database too early would add migration cost before the cache key model stabilizes.

## Why SQLite Later, Not SurrealDB

If the file cache becomes limiting, the next backend should be **SQLite**.

SQLite wins on the axes that matter most for this cache layer:

- single embedded file
- low operational complexity
- strong tooling
- straightforward local concurrency story
- good enough indexing for manifests and reverse-dependency lookups
- better fit for "local artifact cache" than a graph database

SurrealDB's graph model is interesting, but the cache does not need graph-native querying to be correct. A Merkle-style manifest design already gives us recursive invalidation, and SQLite can store the same manifests if and when we need:

- faster metadata queries
- better eviction queries
- fewer small files
- more concurrent access

SurrealDB should only be reconsidered if the cache evolves into a broader provenance or query system rather than staying a local artifact cache.

## Final Recommendation

Implement the cache in this order:

1. add run-local single-flight memoization
2. add a persistent file-backed cache at `<workspace-root>/.darkmatter/cache/`
3. key entries with `biscuit-hash` identity hashes, semantic content hashes, state hashes, and Merkle-style dependency closure hashes
4. promote only successful results into the persistent cache
5. revisit the backend only after the key model and operation coverage are proven

When that backend decision does need to be revisited, prefer **SQLite** over SurrealDB.
