# Cache Design for Tree Hugger Symbol Schema v2

## Status

Draft design for the upcoming major refactor.

## Purpose

This document describes a caching architecture that improves performance for the new symbol schema defined in `symbol-schema-design.md`.

The goal is to make repeated analysis runs fast while preserving correctness when files, queries, grammar versions, or schema versions change.

## Current Baseline

Today, `tree-hugger` already has:

1. Query compilation cache via `OnceLock<Mutex<HashMap<...>>>` in `lib/src/queries/mod.rs`.
2. Per-file content hash generation in `TreeFile` (`hash` from file source).
3. Small in-memory package module memoization in `TreePackage`.

What is missing is persistent incremental caching for parsed file artifacts and enriched symbol metadata.

## Performance Goals

1. Warm-run speedup of at least 3x for unchanged repositories.
2. Re-analysis cost proportional to changed files, not total files.
3. Stable memory usage under large monorepos.
4. Zero stale results after schema/query/grammar/config changes.

## Design Principles

1. Cache by pipeline stage, not only final output.
2. Invalidate with deterministic fingerprints, not ad-hoc TTL.
3. Keep cache entries content-addressed where possible.
4. Separate local in-process cache (fast) from on-disk cache (persistent).
5. Allow partial reuse of symbol facets across passes.

## Pipeline and Cache Layers

The new symbol pipeline has four passes: parse, bind, semantic, docs.

Caching is applied per pass:

1. `L0 Query Cache` (existing): compiled Tree-sitter queries by `(language, query_kind)`.
2. `L1 Parse Cache`: file parse snapshot keyed by file hash and parser fingerprint.
3. `L2 Symbol Facet Cache`: per-file `Vec<SymbolRecord>` with pass-specific completeness.
4. `L3 Package Graph Cache`: cross-file binding edges and export/import resolution.
5. `L4 Render Cache`: final CLI JSON summaries for exact command/config inputs.

## Fingerprints and Cache Keys

Correct invalidation depends on precise fingerprints.

### Global Analyzer Fingerprint

`AnalyzerFingerprint` is included in every cache key:

1. `schema_version` (Symbol Schema v2 major/minor).
2. `tree_hugger_version` (crate version).
3. `grammar_fingerprint` (Tree-sitter grammar versions/checksums).
4. `query_fingerprint` (hash of all relevant query sources).
5. `config_fingerprint` (language override, ignore globs, feature flags, analysis options).

If any element changes, entries are treated as invalid.

### File-Level Key

`FileCacheKey`:

1. normalized absolute file path
2. language
3. file content hash
4. analyzer fingerprint

### Symbol-Level Key

Use `SymbolId` from schema v2. For secondary caches:

1. `symbol_id` + `type_view_kind` for canonical/resolved/expanded type views.
2. `symbol_id` + `doc_hash` for parsed docs.
3. `symbol_id` + `semantic_context_hash` for semantic inference.

## Cached Artifacts

### Parse Snapshot (L1)

Stores parse-layer output only:

1. root syntax diagnostics
2. raw symbol boundaries/spans
3. attached raw comments
4. declaration/signature source slices

This allows reuse when only later passes change.

### Symbol Snapshot (L2)

Stores per-file `SymbolRecord` values with pass completeness metadata.

```rust
pub struct SymbolSnapshot {
    pub key: FileCacheKey,
    pub completed_passes: Vec<AnalysisPass>, // Parse, Bind, Semantic, Docs
    pub symbols: Vec<SymbolRecord>,
    pub imports: Vec<ImportRecord>,
    pub exports: Vec<ExportRecord>,
    pub diagnostics: Vec<Diagnostic>,
    pub created_at_epoch_ms: u64,
}
```

`completed_passes` enables incremental upgrading of cache entries instead of full recompute.

### Package Graph Snapshot (L3)

Stores cross-file relationships:

1. symbol reference edges
2. import/export resolution map
3. dependency fan-out per file

This allows targeted invalidation when one file changes.

### Render Snapshot (L4)

Stores the final CLI output payload for exact command signatures:

1. command (`symbols`, `types`, `imports`, `exports`, `lint`, etc.)
2. glob set and ignore patterns
3. output mode (`json`, `plain`)
4. root dir + analyzer fingerprint + included file hashes

## Storage Architecture

Use a two-tier cache:

1. In-memory cache for current process.
2. Persistent disk cache for cross-process reuse.

### In-Memory Tier

Recommended:

1. `DashMap` or sharded `Mutex<HashMap<...>>` for low-lock read paths.
2. LRU bound by entry count and approximate bytes.
3. `Arc` values to avoid cloning large symbol vectors.

### Disk Tier

Recommended default: SQLite with WAL mode.

Reasons:

1. Single-file store.
2. Good concurrent read behavior.
3. Transactional updates.
4. Flexible indexing and metadata queries.

Alternative backends like `redb` are viable, but SQLite is the safest default for tooling workflows and debugging.

## Suggested SQLite Schema

```sql
CREATE TABLE cache_meta (
  key TEXT PRIMARY KEY,
  value TEXT NOT NULL
);

CREATE TABLE file_snapshots (
  file_key TEXT PRIMARY KEY,
  file_path TEXT NOT NULL,
  language TEXT NOT NULL,
  file_hash TEXT NOT NULL,
  analyzer_fingerprint TEXT NOT NULL,
  completed_passes TEXT NOT NULL, -- JSON array
  payload BLOB NOT NULL,          -- compressed serde payload
  created_at_ms INTEGER NOT NULL,
  updated_at_ms INTEGER NOT NULL
);

CREATE INDEX idx_file_snapshots_path ON file_snapshots(file_path);
CREATE INDEX idx_file_snapshots_hash ON file_snapshots(file_hash);

CREATE TABLE package_graph_snapshots (
  graph_key TEXT PRIMARY KEY,
  root_dir TEXT NOT NULL,
  analyzer_fingerprint TEXT NOT NULL,
  included_file_hashes TEXT NOT NULL, -- sorted + hashed
  payload BLOB NOT NULL,
  created_at_ms INTEGER NOT NULL,
  updated_at_ms INTEGER NOT NULL
);

CREATE TABLE render_snapshots (
  render_key TEXT PRIMARY KEY,
  command_sig TEXT NOT NULL,
  analyzer_fingerprint TEXT NOT NULL,
  input_hash TEXT NOT NULL,
  payload BLOB NOT NULL,
  created_at_ms INTEGER NOT NULL,
  updated_at_ms INTEGER NOT NULL
);
```

Payloads should be compressed (`zstd`) to reduce disk footprint.

## Invalidation Strategy

### Hard Invalidation Triggers

1. Schema major version change.
2. Analyzer crate version change (configurable, usually strict).
3. Query fingerprint change.
4. Tree-sitter grammar fingerprint change.

### File-Level Invalidation

1. File content hash change invalidates L1/L2 for that file.
2. Deleted files remove entries.
3. Renames move key by path update but preserve reuse if hash and content stay identical.

### Dependency-Aware Invalidation

Use package graph snapshot edges:

1. If file `A` exports symbols consumed by file `B`, changing `A` invalidates bind/semantic results in `B`.
2. Parse-layer artifacts for `B` remain valid unless `B` content changed.

### Time-Based Eviction

No TTL for correctness-sensitive entries.
Optional TTL only for render snapshots (L4), because they are pure acceleration.

## Incremental Update Algorithm

On each run:

1. Build analyzer fingerprint.
2. Collect candidate files.
3. Compute file hashes.
4. For each file:
   1. Try L2 `SymbolSnapshot` hit for required pass level.
   2. If miss, try L1 parse snapshot.
   3. Recompute only missing passes.
   4. Persist upgraded snapshot.
5. Rebuild or reuse L3 package graph based on included file hash set.
6. Materialize output and optionally cache L4.

This gives cheap warm runs and cheap partial reruns.

## Facet-Level Reuse Opportunities

The new schema facets allow narrow recomputation:

1. `docs` facet:
   1. key by raw doc hash.
   2. parse once, reuse across unchanged comments.
2. `type_info.views.canonical`:
   1. key by declared type text hash.
   2. avoids repeated normalization work.
3. `semantics` facet:
   1. key by `symbol_id` + local body hash + dependency summary hash.
   2. only recompute when body or relevant callees changed.

## CLI and UX Additions

Recommended flags:

1. `--cache` / `--no-cache` (default on).
2. `--cache-dir <PATH>`.
3. `--cache-reset` (clear all cache before run).
4. `--cache-stats` (print hit/miss and timings).

Recommended environment variables:

1. `TREE_HUGGER_CACHE_DIR`
2. `TREE_HUGGER_CACHE_DISABLE=1`

## Concurrency Model

1. Read-mostly operations should be lock-light.
2. Use write transactions for snapshot updates.
3. Apply single-flight deduplication so concurrent workers do not recompute the same file.
4. Prefer atomic upsert semantics for snapshot writes.

## Observability

Emit metrics for:

1. hit/miss rate by layer (`L1`, `L2`, `L3`, `L4`)
2. parse/bind/semantic/docs pass durations
3. bytes read/written and compression ratio
4. invalidation reason counts

Expose these metrics in debug logs and `--cache-stats` output.

## Risks and Mitigations

1. Risk: stale cache from incomplete fingerprinting.
   1. Mitigation: include query, grammar, schema, and config fingerprints in all keys.
2. Risk: disk bloat in large monorepos.
   1. Mitigation: size-based eviction and optional retention windows.
3. Risk: lock contention in parallel runs.
   1. Mitigation: two-tier cache + sharded memory map + SQLite WAL.
4. Risk: migration complexity from v1 schema.
   1. Mitigation: dual-write mode during rollout and snapshot version tagging.

## Rollout Plan

1. Phase 1:
   1. Keep current query cache.
   2. Add disk-backed L2 file snapshots for parse pass only.
2. Phase 2:
   1. Add pass completeness tracking.
   2. Enable bind/docs facet reuse.
3. Phase 3:
   1. Add package graph cache and dependency-aware invalidation.
   2. Add render cache and CLI stats.
4. Phase 4:
   1. Tune eviction.
   2. Stabilize public cache schema and document compatibility policy.

## Testing Strategy

1. Deterministic cache key tests for all fingerprint inputs.
2. Golden tests for cache hit behavior with unchanged files.
3. Mutation tests that verify correct invalidation on:
   1. file content changes
   2. query updates
   3. schema version bumps
4. Concurrency tests for duplicate-work suppression.
5. Corruption recovery tests (bad payload, partial write, stale index).

## Recommended Initial Implementation

Start with the smallest high-impact slice:

1. Add `AnalyzerFingerprint`.
2. Add per-file `SymbolSnapshot` cache keyed by file hash and fingerprint.
3. Cache parse + docs passes first.
4. Add `--cache-stats` to prove speedup.

Then expand to bind/semantic and package graph caching.

