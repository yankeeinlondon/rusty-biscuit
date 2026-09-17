---
area: darkmatter
status: active
created: 2026-09-11
activated: 2026-09-16
activated_by: darkmatter/features/2026-09-09-more-context (plan Phase 1, Q3)
owner: Ken Snyder <ken@ken.net>
origin: darkmatter/features/2026-09-09-more-context/spec.md
packages:
    - darkmatter
---

# Disable persistent compose cache until ContentPolicy exists

**Ruling (Ken, 2026-09-11, R18 of the more-context feature):** local file
transclusion does not need caching. Persistent caching is for content that
arrives through an expensive or agentic operation (for example, summarizing a
website). Invalidating such content requires a `ContentPolicy` struct that
defines the freshness policy of the cached content. Until `ContentPolicy`
exists, nothing is cached persistently.

## What exists today

`md compose --cache-root <dir>` enables a persistent file store
(`compose/cache/`) holding four artifact classes: document snapshots, composed
`::file` children, individual operation results (`::code`, `::toc-linking`),
and remote-URL artifacts. Entry keys include a hash of the ctx snapshot minus a
volatile list in `cache/hashing.rs`. A hit replays the stored text verbatim.
Claudine never enables the store; the run-local cache is unaffected.

## What the fix must do

- Stop persisting `::file` children, `::code`, `::toc-linking`, and snapshots.
- Decide the fate of `--cache-root` and the remote-artifact store, the one
  legitimate current use: remove the flag, or keep it scoped to remote
  artifacts only until `ContentPolicy` lands. Needs Ken's ruling.
- Sketch `ContentPolicy`, the freshness-policy struct every future agentic or
  remote transclusion operation must carry before persistent caching returns.

Closes when a local-only compose performs no persistent write and the
`ContentPolicy` sketch is ratified. Origin: [more-context spec](../../features/2026-09-09-more-context/spec.md).

## Implementation status (2026-09-16)

The cache-disable portion is implemented, as a hard prerequisite of the
more-context feature (its Q3):

- `PipelineRuntime::with_remote_fetch` no longer takes a cache root, and
  `reference::graph::make_cache` attaches none; `RunLocalCache::with_persistent`
  is test-only. No production path persists a document snapshot, composed
  `::file` child, or `::code` / `::toc-linking` result.
- `--cache-root` stays scoped to raw remote-URL bodies (the second option
  above) **pending Ken's ruling**; it is the least destructive choice and can
  be narrowed or removed without migrating any data. Those bodies carry no
  composed context, so they cannot replay identity or probe output.
- The persistent compose/operation/snapshot machinery (`cache/runtime.rs`
  read/write paths, `manifest.rs`, `hashing.rs` keys) is retained and still
  unit-tested for reuse by `ContentPolicy`. Whether to delete it instead is
  also left to Ken.
- Regression coverage: `lib/tests/persistent_cache_disabled.rs` (warm-cache
  replay across all four freshness modes; reference graph) and
  `cli/tests/compose_remote_caching.rs`
  `test_compose_cache_root_never_replays_composed_local_output` (end-to-end:
  remote body still cached, local output recomposed, no local manifests).

Still open: Ken's ruling on `--cache-root` and the remote-artifact store, and
the `ContentPolicy` sketch.
