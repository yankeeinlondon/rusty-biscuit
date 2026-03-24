# Caching

This document describes the caching system currently implemented for Darkmatter's compose pipeline.

It replaces the earlier proposal-oriented notes in this file with the behavior that exists in the code today. Where there is still a gap between the long-term design and the current implementation, that gap is called out explicitly.

## Overview

Darkmatter uses a two-layer cache for compose work:

1. A run-local in-memory cache owned by `PipelineRuntime`
2. An optional persistent file-backed cache stored under `.darkmatter/cache/v1/`

The cache is focused on transclusion-heavy compose work, especially:

- Recursive `::file` markdown transclusion
- `::code` source transclusion
- `::toc-linking` heading-link generation

The goals of the current implementation are:

- Deduplicate repeated work inside a single compose run
- Reuse stable artifacts across compose runs
- Invalidate parent entries when dependency state changes
- Keep the implementation simple enough to debug from manifests and blobs on disk

## Current Status

The implemented cache includes:

- Run-local single-flight deduplication for compose cores and operation results
- Persistent document snapshots
- Persistent composed document cores
- Persistent operation results for `::code` and `::toc-linking`
- Dependency-aware closure-hash validation for composed documents
- Freshness policy handling for persistent reads
- In-memory retention of loaded document snapshots to avoid repeated manifest reads
- Concurrent run-local maps for low-contention cache access

The implementation does not yet include:

- Remote artifact caching
- TTL-driven policies for remote or LLM-backed operations
- A special forced-mode empty-output fallback on generation failure
- Persisted report warnings reconstruction from cache

## Runtime Architecture

The runtime cache lives in `markdown/compose/cache/runtime.rs`.

`RunLocalCache` owns several in-memory maps backed by `DashMap`:

- `markdown_documents`: canonical-path markdown loads
- `toc_headings`: cached TOC heading extraction
- `document_snapshots`: cached snapshot manifests keyed by `source_id_hash`
- `compose_results`: single-flight slots for composed child markdown
- `operation_results`: single-flight slots for `::code` and `::toc-linking`

These maps are shared by cloning the runtime cache, so child compose branches see the same run-local cache.

### Single-flight behavior

For compose results and operation results:

- The first caller inserts an `InFlight` slot and computes
- Concurrent callers wait on the same slot
- Successful results are promoted to the slot
- Waiters receive the shared result
- A timeout falls back to duplicate computation to reduce Rayon deadlock risk

This is more than memoization. It suppresses duplicate work across concurrently evaluated sibling transclusions.

## Persistent Store

The persistent backend lives in `markdown/compose/cache/store.rs`.

The file layout is:

```text
.darkmatter/cache/v1/
  manifests/
    snapshot/
    composed/
    operation/
  blobs/
    md/
```

Files are fanned out by the first four hex digits of the key or blob hash:

```text
manifests/{class}/{ab}/{cd}/{hex}.json
blobs/{ext}/{ab}/{cd}/{hex}.{ext}
```

Writes are atomic:

- Blob is written first
- Manifest is written second
- Each write uses a temp file plus rename in the target directory

Cache roots are resolved as follows:

- Preferred: `<workspace>/.darkmatter/cache/v1/`
- Optional namespace: `<workspace>/.darkmatter/cache/v1/<namespace>/`
- Fallback: platform cache directory if no workspace root is available

## Artifact Classes

The cache currently persists three artifact classes.

### 1. `document_snapshot`

This is written during `load_markdown()`.

Purpose:

- Track the current state of a source document
- Provide the body hash needed to compute persistent compose keys
- Support freshness validation using fast file metadata checks

Stored manifest fields:

- `canonical_source`
- `source_id_hash`
- `raw_bytes_hash`
- `frontmatter_hash`
- `body_semantic_hash`
- `body_template_hash`
- `modified_at`
- `size_bytes`

Current source kinds:

- `local_file`

The enum also allows `remote_url`, but remote snapshot caching is not implemented yet.

### 2. `compose_document_core`

This is the main persistent artifact for `::file` transclusion.

It represents:

- The recursively composed child document
- Before parent-only transforms such as `exclude`, wrappers, and insertion-context releveling

Stored manifest fields:

- `entry_key`
- `source_id_hash`
- `source_body_semantic_hash`
- `self_hash`
- `closure_hash`
- `dependency_count`
- `dependencies`
- `payload_blob_hash`
- `warnings_hash`
- `created_at`
- `last_accessed_at`
- `expires_at`

### 3. `operation_result`

This persists expensive non-markdown-core operations.

Currently wired operations:

- `::code`
- `::toc-linking`

Stored manifest fields:

- `entry_key`
- `op_kind`
- `self_hash`
- `closure_hash`
- `payload_blob_hash`
- `canonical_source`
- `source_id_hash`
- `source_content_hash`
- `created_at`
- `last_accessed_at`
- `expires_at`

## Hashing Strategy

Darkmatter uses `biscuit-hash` for all cache identity and invalidation hashing.

### Source identity

`compose_cache_key(path)` canonicalizes the filesystem path and stringifies it.

That canonical string is then hashed with raw `xxHash` to produce `source_id_hash`.

### Raw bytes

`raw_bytes_hash()` is an exact byte-level hash.

It is used for:

- Snapshot raw file tracking
- Blob hashes
- Operation source-content validation

### Frontmatter

Frontmatter is converted to canonical JSON with recursively sorted keys and then hashed with raw `xxHash`.
The serializer writes the canonical form in one pass instead of recursively building intermediate strings.

This means:

- YAML key-order changes do not invalidate
- Real data changes do invalidate

### Body semantic hash

`body_semantic_hash()` uses:

- `HashVariant::BlockTrimming`
- `HashVariant::LeadingWhitespace`
- `HashVariant::TrailingWhitespace`

This is the correctness-oriented body hash used in persistent compose keys and snapshot validation.

Notably, the current implementation does not use `InteriorWhitespace` for `body_semantic_hash`, because that would be too aggressive for code blocks, tables, directives, and other structured markdown content.

### Body template hash

`body_template_hash()` uses:

- `HashVariant::BlockTrimming`
- `HashVariant::LeadingWhitespace`
- `HashVariant::TrailingWhitespace`
- `HashVariant::InteriorWhitespace`
- `HashVariant::BlankLine`

This is stored in the snapshot manifest for future structural reuse and diagnostics.

The current implementation stores it but does not yet use it as part of persistent cache validation.

### Effective state hash

`effective_state_hash()` hashes the fully materialized effective state map after merge behavior has been resolved.

This makes persistent compose reuse sensitive to inherited state that affects child output.

### Context hash

`context_hash()` hashes only stable output-relevant context:

- `today`
- `yesterday`
- `tomorrow`
- Sorted environment variables

It intentionally excludes volatile time fields that would destroy cache usefulness.

### Options hash

`options_hash()` includes only output-affecting compose options, including:

- Enabled operations
- Failure behavior flags that alter output
- Transclusion allow/deny flags
- `code_fallback_language`
- Cleanup settings
- Replace inheritance behavior
- One-off replace maps
- External state
- Set overrides

## Persistent Key Model

### Compose core entry key

Composed markdown cores use:

```text
compose_entry_key(
  source_id,
  source_snapshot.body_semantic_hash,
  state_hash,
  context_hash,
  options_hash
)
```

This means a persistent compose hit depends on:

- Which child source was transcluded
- The child's current snapshot body hash
- Effective inherited state
- Runtime context
- Output-affecting compose options

Parent-only cheap transforms are intentionally outside this persistent key:

- `exclude`
- quotation wrapping
- disclosure wrapping
- insertion-context heading releveling

### Operation entry key

Operation results use:

```text
operation_entry_key(op_kind, source_id, variant_hash)
```

Where `variant_hash` is derived from parameter buckets.

## Parameter Buckets

The parameter bucket model lives in `markdown/compose/cache/operation.rs`.

Each cacheable operation classifies inputs into:

- `conditional`
- `pre`
- `variant`
- `post`

Only `variant` parameters participate in the persistent key.

### `::file`

Current bucket model:

- `conditional`: `when`
- `variant`: `replace`
- `post`: `exclude`, `quotation`, `disclosure`

Important note: the `FileOperation` bucket model exists, but the current `::file` compose-core cache path is still keyed through the compose-core key model above rather than directly through `FileOperation::variant_cache_key()`.

### `::code`

Current bucket model:

- `conditional`: `when`
- `variant`: effective `replace` behavior plus inferred language
- `post`: quotation/disclosure wrappers

The persistent `::code` cache now uses this model.

### `::toc-linking`

Current bucket model:

- `variant`: heading levels, cleanup services, keep filters, reject filters, `empty_text`

`::toc-linking` does not use `BlockOptions`, so it has standalone helpers instead of implementing `CacheableOperation`.

The persistent `::toc-linking` cache now uses this model.

## Dependency-aware Invalidation

Dependency tracking is implemented for composed markdown cores.

Each composed document records direct dependencies as `DependencyRef` values:

- `artifact_class`
- `entry_key`
- `source_id_hash`
- `closure_hash`

These refs are collected per `PipelineRuntime` branch, so a child compose records only its direct dependencies, not every dependency seen elsewhere in the run.

### Closure hash

`closure_hash(self_hash, deps)` hashes:

- The artifact's `self_hash`
- Each dependency's `source_id_hash`
- Each dependency's `closure_hash`

This gives Merkle-style invalidation:

1. A child source changes
2. The child snapshot or operation source content changes
3. The child artifact's closure hash changes or becomes invalid
4. The parent's stored dependency ref no longer matches current dependency state
5. The parent is treated as stale

### Revalidation behavior

When Darkmatter reads a composed-document manifest in strict or fallback freshness modes, it checks:

1. `expires_at`
2. The source document's current snapshot body semantic hash
3. Every dependency ref by loading that dependency artifact and validating its current closure state

If any of those checks fail, the compose core is stale.

### Operation validation

Operation artifacts do not currently recurse through dependency graphs.

Instead, operation freshness is validated by:

- `expires_at`
- Re-reading the source file bytes
- Comparing the current `raw_bytes_hash` to the stored `source_content_hash`

For current `::code` and `::toc-linking`, that is sufficient because the expensive core depends on a single local source file.

## Freshness and Access Modes

The cache has two orthogonal policy knobs.

### Access mode

`CacheAccessMode` controls read/write behavior:

- `Off`: no cache use
- `ReadOnly`: read existing entries, never write
- `ReadWrite`: normal mode
- `Refresh`: bypass existing run-local hits and recompute, then write back

### Freshness mode

`CacheFreshnessMode` controls how persistent reads treat stale entries:

- `Strict`: revalidate and reject stale entries
- `Fallback`: revalidate, but keep a stale entry available if recomputation fails
- `Optimistic`: accept any present persistent entry without revalidation
- `Forced`: currently behaves the same as optimistic for reads

### Important note about `Forced`

The original design intent for `Forced` was:

- Treat stale hits as usable
- If generation misses and recomputation fails, replace the directive with empty output instead of erroring

That final "empty output on miss + failure" behavior is not implemented yet.

Today, `Forced` only affects persistent read validation. It does not add special error suppression at the compose-call-site layer.

## Read and Write Flow

### Compose core

For `::file`:

1. Build a run-local key from source/state/context/options
2. Check run-local single-flight slots
3. If persistent caching is enabled, resolve the compose entry key from the current snapshot
4. Read and validate the composed manifest and blob
5. If fresh, return the cached core
6. If stale and mode is `Fallback`, remember the stale payload as backup
7. Compute a fresh child compose if needed
8. Persist the new core manifest and blob
9. Apply parent-only transforms after the cache hit/miss

### Operation result

For `::code` and `::toc-linking`:

1. Build a run-local key from canonical source plus variant parameters
2. Check run-local single-flight slots
3. Resolve the operation entry key
4. Read and validate the persistent manifest and blob
5. If fresh, return the cached core
6. If stale and mode is `Fallback`, keep the stale payload as backup
7. Compute the fresh operation output if needed
8. Persist the new operation manifest and blob
9. Apply cheap post-cache transforms such as wrappers

## Snapshot Caching in Memory

One implementation detail worth keeping:

- `DocumentSnapshotManifest` values are cached in-memory inside `RunLocalCache`

This avoids repeated manifest reads from disk when multiple compose-core lookups need the same source snapshot during one run.

## Cache Statistics

Compose reports can include cache stats gathered during the run:

- `hits`
- `misses`
- `writes`
- `inflight_waits`
- `errors`
- `persistent_hits`
- `persistent_writes`
- `revalidations`
- `stale_hits`

These are merged upward through child compose reports in the normal compose-report flow.

## Current Limitations and Future Work

The following are still reasonable future improvements:

- Use `FileOperation` directly for `::file` variant-key generation once that abstraction is pushed further into the compose-core path
- Persist and reconstruct warnings from cache instead of dropping them on cache hits
- Add explicit TTL policies for remote and time-sensitive operations
- Implement the original forced-mode empty-output fallback semantics
- Decide whether `body_template_hash` should drive future structural caching or diagnostics
- Expand operation caching beyond local-file-based `::code` and `::toc-linking`

## Source Files

The current implementation is primarily defined in:

- `darkmatter/lib/src/markdown/compose/cache/types.rs`
- `darkmatter/lib/src/markdown/compose/cache/hashing.rs`
- `darkmatter/lib/src/markdown/compose/cache/manifest.rs`
- `darkmatter/lib/src/markdown/compose/cache/store.rs`
- `darkmatter/lib/src/markdown/compose/cache/runtime.rs`
- `darkmatter/lib/src/markdown/compose/cache/operation.rs`
- `darkmatter/lib/src/markdown/compose/mod.rs`
- `darkmatter/lib/src/markdown/compose/shell_expansion/types.rs`
