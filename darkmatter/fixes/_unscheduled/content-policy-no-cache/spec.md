---
area: darkmatter
status: unscheduled
created: 2026-09-11
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
`ContentPolicy` sketch is ratified. Origin: [more-context spec](../../../features/2026-09-09-more-context/spec.md).
