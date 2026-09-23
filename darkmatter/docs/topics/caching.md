# Caching

This document describes how Darkmatter's compose pipeline caches work today and
the boundary that decides what may ever reach disk.

## Overview

Darkmatter separates cached artifacts into two categories. Each has its own
cache, and the two never share a store.

| Category | Examples | Where it lives | Governed by |
|---|---|---|---|
| **Semantic-result artifacts** | composed documents, `::file` children, `::code` / `::toc-linking` results, document snapshots, shell output | run-local memory only | `CacheAccessMode`, within one compose run |
| **Transport artifacts** | raw HTTP(S) response bodies | the remote transport cache under an explicit cache root | HTTP validators, response `Cache-Control`, and `RemoteReadConfig` |

> **Semantic results are never persisted.** Until a `ContentPolicy` defines
> when cached content becomes stale, no composed or derived result is read
> from or written to disk (more-context rulings **R18** and **R36**,
> `darkmatter/fixes/2026-09-16-content-policy-no-cache`). A warm cache cannot
> replay an earlier run's composed output, warnings, runtime context, or
> shell/probe output. The persistent semantic-result implementation was
> deleted, not disabled; see [ContentPolicy](#contentpolicy-prerequisite-for-persisting-semantic-results).

A transport artifact is a lower-level exception. A cache hit returns raw bytes
to the normal composition pipeline, which composes them again under the
current request. It never returns a previously composed document.

## Run-local Cache

The run-local cache (`RunLocalCache`, `markdown/compose/cache/runtime.rs`) is
owned by `PipelineRuntime` and lives for one compose invocation. Cloning it
shares the same maps, so child compose branches see one cache. Its concurrent
`DashMap` maps hold:

- `markdown_documents`: loaded Markdown, keyed by canonical path
- `toc_headings`: TOC heading extraction
- `compose_results`: single-flight slots for composed `::file` children
- `operation_results`: single-flight slots for `::code` and `::toc-linking`

`RunLocalCache` has no file-backed store and cannot acquire one.
`lib/tests/semantic_results_never_persist.rs` fails the build's tests if it
names `FileStore` or `RemoteFetchRuntime`, or if the deleted persistence
symbols reappear.

Local file transclusion stays run-local permanently. A future `ContentPolicy`
does not make it persistable without a new ruling.

### Single-flight behavior

For compose results and operation results:

- The first caller inserts an `InFlight` slot and computes.
- Concurrent callers wait on the same slot.
- Successful results are promoted to the slot, and waiters receive the shared
  result.
- A timeout falls back to duplicate computation to reduce Rayon deadlock risk.

This suppresses duplicate work across concurrently evaluated sibling
transclusions, not only repeated sequential work.

### Access mode

`CacheAccessMode` (`ComposeOptions::with_cache_access_mode`) controls run-local
reuse. Nothing it selects is written to disk.

- `Off`: no cache use; every request computes fresh
- `ReadOnly`: read existing entries, never write
- `ReadWrite` (default): read hits, write misses
- `Refresh`: ignore existing entries, recompute, then write fresh results

### Run-local keys

A composed `::file` child is keyed on:

```text
compose:{source_id}:{state_hash}:{context_hash}:{options_hash}:{overlay_hash}
```

- `source_id`: `source_id_hash` of the canonicalized source path
- `state_hash`: `effective_state_hash` of the fully merged effective state
- `context_hash`: see [Context hash](#context-hash)
- `options_hash`: see [Options hash](#options-hash)
- `overlay_hash`: the directive's `set` overlay, combined with the options hash

Parent-only cheap transforms are applied after the lookup and stay outside the
key: `exclude`, quotation wrapping, disclosure wrapping, and insertion-context
heading releveling.

`::code` results are keyed on the canonical source plus a variant hash from
[parameter buckets](#parameter-buckets). `::toc-linking` results are keyed on
the canonical source plus its variant options.

`ComposeResult` also records the subtree's context-group closure, so the parent
runtime can `record_context_groups` on a run-local hit.

### Context hash

`context_hash()` hashes the request context a composed source was rendered
from:

- every captured context value, excluding volatile per-second clock fields
  (`now`, `now_utc`, `utc`, `time`, `time_military`, `timestamp`,
  `timestamp_ms`) and volatile system state (`memory_used`, `memory_avail`)
- sorted environment variables

A transcluded child's context is finalized before its key is computed. The
child's referenced `ctx.*` groups are added to the request context first (see
[Context Variables](./context-variables.md#the-request-context-and-its-authority)),
and the lookup and the write both use that one post-extension hash. When a
child names no group its parent lacks, the parent's phase-wide hash is reused.
Because one request has one context, a run-local hit cannot hide a missing
capture.

### Options hash

`options_hash()` includes only output-affecting compose options, including
enabled operations, failure-behavior flags that alter output,
transclusion allow/deny flags, `code_fallback_language`, cleanup settings,
replace inheritance, one-off replace maps, external state, and set overrides.
It delegates to `ComposeOptions::compose_cache_fingerprint`, which shares one
field classification with the reference-graph options identity.

### Parameter buckets

`markdown/compose/cache/operation.rs` classifies directive parameters into
buckets:

- `conditional`: controls whether the operation runs (`when`)
- `variant`: determines the cached result and participates in the key
- `post`: applied after the lookup (quotation and disclosure wrappers)

For `::code`, the variant bucket holds the effective `replace` behavior and the
inferred language. `::toc-linking` does not use `BlockOptions`; its variant
options (heading levels, cleanup services, keep and reject filters,
`empty_text`) feed `TocLinkingOperation::cache_key_string` directly.

### Shell command memoization

Shell command memoization is a separate run-local cache in
`ShellExpansionRuntime`, not in `RunLocalCache`. The shell family's `no-cache`
spelling (`::shell --no-cache`, `$(<cmd>)::no-cache`,
`::shell-block no_cache=true`) bypasses that memoization only. It is unrelated
to the HTTP `Cache-Control: no-cache` directive described below. See
[Shell Expansion](../inline/shell-expansion.md).

### Reference analysis

`reference_graph()`, `validate_references()`, `composed_references()`, and
related methods build their own memory-only `RunLocalCache` via `make_cache()`
in `markdown/reference/graph.rs`. Every graph build reads its documents from
disk, and `load_markdown()` and TOC heading extraction are deduplicated within
that build. When child composition fetches remote URLs, those bodies use the
same remote transport cache as the compose pipeline, including
`cache_namespace`.

```rust
let mut options = ReferenceGraphOptions::default();
options.compose = options.compose
    .with_cache_root(workspace_root)
    .with_cache_namespace("feature-branch");

let graph = md.reference_graph(options)?;
```

## Remote Transport Cache

The remote transport cache (`markdown/compose/cache/remote_cache.rs`, reached
through `RemoteFetchRuntime` in `remote_fetch.rs`) stores raw HTTP(S) response
bodies. It is the only persistent cache a compose run uses.

### Enabling it

Persistence is explicit opt-in:

- CLI: `md compose FILE --allow-host HOST --cache-root DIR`
- Library: `ComposeOptions::with_cache_root(dir)`, optionally
  `with_cache_namespace(name)` for branch or profile isolation

Without a cache root, every remote read goes to the network and nothing is
stored. There is no platform-cache fallback.

The store root resolves to `<cache-root>/.darkmatter/cache/v1/`, or
`<cache-root>/.darkmatter/cache/v1/<namespace>/` with a namespace. Pass the
directory that should *contain* `.darkmatter/`, not the `v1` directory itself.

**Configuring a cache root mutates nothing.** The store records the path and
creates directories only inside the write that needs them. A run that writes no
transport artifact, including every local-only compose, leaves a missing root
missing and an existing root byte-for-byte unchanged.

### On-disk layout

```text
<cache-root>/.darkmatter/cache/v1[/<namespace>]/
  manifests/remote/{ab}/{cd}/{hex}.json
  blobs/remote/{ab}/{cd}/{hex}.remote
```

Paths fan out by the first four hex digits of the key or blob hash. The
manifest key is `xx_hash` of the full request URL; the blob is keyed by
`xx_hash` of the body. The blob is written before the manifest, and each write
is atomic (temp file plus rename in the target directory). Concurrent processes
rely on that rename, not on lock files.

### Host policy comes first

A cache root never authorizes a host. `RemoteFetchRuntime::register_and_fetch`
checks the exact-host `FetchPolicy` before any fetch task is created, and
therefore before the transport cache is read. A denied host fails without a
cache read or a network request, even when a fresh entry for that URL is on
disk. The default policy is deny-all; the CLI grants hosts with `--allow-host`.

### Freshness

Freshness applies only to an entry that is eligible for storage and reuse (see
[Cache-Control precedence](#cache-control-precedence)). A stored entry's
lifetime is:

1. no lifetime for a `no-cache` response (always revalidate);
2. otherwise `--remote-ttl` / `RemoteReadConfig` TTL, when set;
3. otherwise the response's `max-age`;
4. otherwise no lifetime (stale immediately).

`RemoteFreshnessMode` (`--remote-freshness`) then decides what to do with a
found entry:

- `Fallback` (default): serve within the lifetime; past it, revalidate with a
  conditional GET, and serve the stale body if revalidation fails on the
  network.
- `Strict`: serve within the lifetime; past it, revalidate, and fail if
  revalidation fails.
- `Optimistic`: serve the cached body without revalidation, even when stale.

`--remote-refresh` revalidates every found entry, even a fresh one.
Revalidation sends `If-None-Match` and `If-Modified-Since` when the server
previously returned `ETag` or `Last-Modified`. A `304` keeps the cached body
and refreshes the manifest from the `304`'s headers.

Serving stale data is never an implicit consequence of configuring a cache
root: it happens only under `Fallback`, only after a failed revalidation, and
never for `no-cache`.

### Cache-Control precedence

Response `Cache-Control` outranks every freshness mode and the TTL override
(RFC 9111 §5.2.2.4 and §5.2.2.5). Directive names are matched
case-insensitively, the whole directive list is parsed rather than the first
match, and `no-cache="field"` is treated as `no-cache`.

- **`no-store`** is never written and never reused. A new `no-store` response
  is served for the current read only. An existing entry that records
  `no-store` (including a revalidation whose `304` now says `no-store`) is
  ignored, and its manifest and body are removed best-effort. Blobs are
  content-addressed, so another entry could share the removed body; that entry
  simply re-fetches.
- **`no-cache`** may be stored but is revalidated before every reuse, including
  under `Optimistic`. If that revalidation fails, the read fails; `Fallback`
  never serves a `no-cache` body stale.
- **A TTL override** controls freshness only where HTTP permits storage. It
  never makes a `no-store` response storable or a `no-cache` response fresh. A
  zero TTL is *not* equivalent to `no-store`: a zero-TTL entry is still
  written and revalidated on reuse.

### Manifest privacy and versioning

A `RemoteUrlManifest` records the status, `ETag`, `Last-Modified`,
`Cache-Control`, fetch time, expiry, content hash, blob hash, and size. It never
records the raw request URL:

- `redacted_url` is `scheme://host[:port]/path`. Userinfo and the fragment are
  removed, and any query is replaced by the literal marker `?<redacted>`.
- `source_id_hash` is `xx_hash` of the full, unredacted URL, so URLs that
  differ only in userinfo or query never share an entry.

Warnings name only the redacted form. A path segment can still carry a token,
which redaction does not hide. Cached bodies remain potentially sensitive,
which is why persistence stays explicit opt-in.

Two versions are tracked separately:

- `CACHE_VERSION` (currently `2`) is the manifest schema version. Version `1`
  stored the cleartext URL. An entry from any other version is a miss and is
  left on disk, except that one recording `no-store` is still purged.
- `STORE_LAYOUT_VERSION` (currently `1`) is the `v{N}` directory. It changes
  only when the path scheme changes, so old manifests stay findable.

Darkmatter never deletes a cache tree automatically beyond the `no-store`
removal above.

### Failures are warnings

A failed transport-cache write or `no-store` removal never fails the compose or
changes a fetch's outcome. It is recorded in `RemoteFetchStats::cache_warnings`
and surfaced as a compose warning with stage `remote_cache` and code
`dm.remote_cache.io_failure`. An unusable cache root (unwritable, or a regular
file) first fails at the write that needs it, and the fetch stays network-only.

## Cache Statistics

Run-local and transport activity are reported separately:

- `ComposeReport.cache_stats` (`CacheStats`, run-local): `hits`, `misses`,
  `writes`, `inflight_waits`, `errors`. These merge upward through child
  compose reports and render as the `cache:` summary segment.
- `ComposeReport.remote_fetch_stats` (`RemoteFetchStats`, transport):
  `fetched`, `waits`, `policy_denials`, `failures`, `cache_hits`,
  `revalidations`, `not_modified`, `stale_served`, and `cache_warnings`. These
  render as the `remote:` summary segment.

No statistic counts directory creation as a write.

## ContentPolicy: Prerequisite for Persisting Semantic Results

Persisting an expensive or agentic semantic result (for example, a website
summary) requires a `ContentPolicy` that defines when the content becomes
stale. It does not exist yet. The contract a future implementation must meet is
recorded in the
[content-policy-no-cache specification](../../fixes/2026-09-16-content-policy-no-cache/spec.md#contentpolicy-design-constraints).
In brief:

- `ContentPolicy` governs content freshness only. It does not grant network
  access, define artifact identity, choose file placement, or permit persisting
  sensitive data, and it does not enable a cache root by itself.
- It needs a versioned, serializable identity; explicit expiry predicates with
  "any predicate expires" semantics; an evaluator for each predicate; an
  explicit stale action (recompute, serve with a warning, or fail); stored
  generation evidence; deterministic evaluation under an injected clock; and
  fail-closed handling of absent, unknown, malformed, or newer policies.
- Calendar months and years must either define exact calendar arithmetic and
  timezone or be replaced by unambiguous durations.
- A persistent producer must opt in at its own boundary and key its artifact
  on producer kind and version, inputs, dependency closure, and policy
  identity. Shell execution, ICMP probes, and current-value lookups are not
  persistable merely because their document has a policy.

The shared `ContentPolicy` vocabulary is to live in a new dependency-light
library consumed by Research, Darkmatter, and Claudine (Q3). Darkmatter must not
define its own unqualified `ContentPolicy`.

## Known Gaps

- `redacted_url` keeps the URL path, which can itself carry a token.

## Source Files

- `darkmatter/lib/src/markdown/compose/cache/types.rs`
- `darkmatter/lib/src/markdown/compose/cache/runtime.rs`
- `darkmatter/lib/src/markdown/compose/cache/hashing.rs`
- `darkmatter/lib/src/markdown/compose/cache/operation.rs`
- `darkmatter/lib/src/markdown/compose/cache/remote_cache.rs`
- `darkmatter/lib/src/markdown/compose/cache/manifest.rs`
- `darkmatter/lib/src/markdown/compose/cache/store.rs`
- `darkmatter/lib/src/markdown/compose/remote_fetch.rs`
- `darkmatter/lib/src/markdown/compose/shell_expansion/types.rs`
- `darkmatter/lib/src/markdown/reference/graph.rs`
- `darkmatter/lib/tests/semantic_results_never_persist.rs`
