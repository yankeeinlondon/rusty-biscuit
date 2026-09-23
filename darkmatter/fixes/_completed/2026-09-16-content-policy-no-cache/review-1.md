---
$schema: feature-review.yaml
ready: false
findings:
  - title: Split Cache-Control field lines can bypass no-store
    priority: high
human_review: true
human_review_items:
  - |-
      Decide how to close the split `Cache-Control` header gap before release. The recommended option is to update the shared HTTP fetch layer so it combines every `Cache-Control` field line, then add a test where `max-age=3600` and `no-store` arrive on separate lines. The alternative is to narrow the specification and public guarantee so this valid HTTP response shape is explicitly unsupported; that would accept persistence of a response whose server prohibited storage.
  - |-
      Confirm the two design defaults adopted without an owner decision:

      1. Keep the deleted semantic-result persistence machinery deleted until a future content-freshness policy is designed.
      2. Put the future shared content-freshness vocabulary in a small dependency-light library used by Research, Darkmatter, and Claudine.

      For each item, either approve the recorded default or select a different option from Q2/Q3 in the specification and require the implementation/design record to be updated.
reviewed_by: codex/gpt-5.6-sol
created: "2026-09-18T02:08:27-07:00"
spec: 2026-09-16-content-policy-no-cache/spec.md
log: darkmatter/fixes/2026-09-16-content-policy-no-cache/implementation-log.md
implemented: true
implemented_by: claude/opus
description: "A **fix** review of `2026-09-16-content-policy-no-cache/spec.md`"
fix: 2026-09-16-content-policy-no-cache/review-1.md
next: 2026-09-16-content-policy-no-cache/review-2.md
---

# Review 1: Content Policy No Cache

## Verdict

The fix is **not ready for production**. The semantic-result persistence path
has been removed, local-only work leaves configured cache roots untouched, host
authorization precedes cache access, manifest URL metadata is redacted, and the
single-line `no-store` / `no-cache` behavior is well exercised. However, a
standards-valid response with multiple `Cache-Control` field lines can still
persist and later reuse a body whose server sent `no-store`. That directly
violates the required transport-cache contract and acceptance criterion 4.

The required behavior is filesystem, HTTP/cache, API, and spawned-CLI behavior.
Level 1 is the correct verification boundary throughout; no requirement depends
on terminal-emulator rendering, terminal input encoding, or OS keyboard events,
so Level 2 and Level 3 are not applicable.

## Findings

### High — Split Cache-Control field lines can bypass no-store

`biscuit_file::file_reference::fetch::fetch` extracts `Cache-Control` with
`HeaderMap::get` (`biscuit-file/lib/src/file_reference/fetch.rs:221-225`), which
returns only one field value. The same issue exists in `post` at lines 292-296.
Darkmatter can correctly parse a complete comma-separated directive list, but
it cannot enforce a directive that the shared fetch layer discarded.

For example, an origin may validly send these as separate field lines:

```text
Cache-Control: max-age=3600
Cache-Control: no-store
```

Darkmatter sees only `max-age=3600`, writes the response body and manifest, and
may serve that body from the transport cache on a later run. This contradicts
the specification's requirement that `no-store` must never be persisted or
reused, even with a TTL override. It also makes the unconditional public claims
in `ComposeOptions::with_cache_root`, CLI help, and the caching documentation
too strong. The later “Known Gaps” disclosure does not make the earlier safety
guarantee true.

The Level-1 matrix does not cover this response shape. Both
`no_store_response_is_never_stored_under_any_mode_or_ttl` and the spawned-CLI
test pass a single combined field value such as `max-age=3600, no-store`.
The implementation plan records that a discarded throwaway spike reproduced
the split-field failure, but there is no retained regression test and the
production dependency remains unchanged.

Required fix: make `biscuit-file` combine all `Cache-Control` field values in
wire order (for both GET and POST response construction), expose the combined
value to Darkmatter, and retain a `biscuit-file` integration test plus a
Darkmatter transport-cache test proving that a split-line `no-store` response
writes no artifact under every relevant freshness/TTL mode. If the project
instead chooses not to support valid repeated field lines, the specification,
acceptance criterion, help, builder docs, and topic docs must all be narrowed;
that choice knowingly weakens the promised `no-store` boundary.

## Requirement Verification Levels

| Acceptance criterion | Strongest verification present | Assessment |
| --- | --- | --- |
| 1. Local-only work does not create or mutate a cache root | Level 1 library and spawned-CLI filesystem tests | Pass; correct level |
| 2. Semantic results are recomputed and never persisted across cache controls and entry points | Level 1 library, reference-graph, structural, and spawned-CLI tests | Pass; correct level |
| 3. The CLI may reuse raw remote bytes while recomposing local content | Level 1 spawned-CLI test with a real local HTTP server | Pass; correct level |
| 4. `no-cache` always revalidates and `no-store` is never persisted/reused for every valid response shape | Level 1 unit/integration and spawned-CLI tests for single field lines only | **Fail; wrong coverage breadth. Repeated `Cache-Control` field lines bypass the guarantee.** |
| 5. Host policy is checked before cache access and a cache root grants no network authority | Level 1 runtime and spawned-CLI tests, including seeded entries | Pass; correct level |
| 6. Persistent manifests omit cleartext URL credentials and query values | Level 1 library and spawned-CLI persisted-byte inspection | Pass; correct level |
| 7. Help, builders, docs, and the Darkmatter skill describe the resolved boundary | Level 1 help test plus source review | Partial; terminology aligns, but the unconditional `no-store` claim conflicts with the documented split-header gap |
| 8. Production code cannot attach a semantic-result store to `RunLocalCache` | Level 1 structural source guard with planted-violation self-test | Pass; appropriate structural verification |

## Implementation Assessment

- `RunLocalCache` is memory-only, the deleted persistent semantic-result
  symbols are guarded against reintroduction, and `FileStore` use is restricted
  to the remote transport path.
- `FileStore::at` performs no filesystem I/O. Its first eligible write creates
  only the required fan-out directories, while local-only library, graph, and
  CLI tests cover missing, existing, partially populated, namespaced, and
  regular-file roots.
- `ResponseDirectives` distinguishes `no-store`, `no-cache`, and zero
  `max-age`; parses case-insensitive and quoted single-field directive lists;
  and uses `StorableDirectives` to keep a recognized `no-store` value out of the
  write path.
- Cache write and purge failures remain non-fatal and surface through coded
  `remote_cache` compose warnings. Host policy is checked before the cache-read
  path.
- Manifest schema and store-layout versions are separated. Version-1
  `no-store` entries remain discoverable for purging, while newly written
  manifests persist a redacted diagnostic URL and hash the complete URL for
  identity.
- No performance or ergonomics change beyond the split-header fix is necessary
  for production readiness. Combining a small number of header values is
  negligible beside the network request and avoids duplicating HTTP header
  semantics inside Darkmatter.

## Verification

- `just test`: 8,118 Level-1 tests passed; 14 tests skipped.
- `just lint`: passed for `darkmatter`, `darkmatter-cli`, `dmls`, and
  `zed-dmls-cli`; the `zed-dmls` `wasm32-wasip2` compile check also passed.
- `git diff --check`: passed before the review metadata edit and was rerun after
  writing this review.
- GitNexus was bound to the `feat-dark-fixes` worktree and current at `HEAD`.
  The cache completion is uncommitted and therefore absent from that index;
  the new `fetch_with_cache` symbol could not be resolved, so the review used
  direct source and call-site inspection for the working-tree implementation.

Cross-OS execution evidence is intentionally not a readiness finding; CI/CD
owns that evidence. The reviewed code and tests use `Path` operations rather
than native separator strings for the changed filesystem behavior.
