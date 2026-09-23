---
$schema: feature-review.yaml
ready: false
findings:
  - title: The canonical biscuit-file gate does not run the new cache-control regressions
    priority: high
human_review: true
human_review_items:
  - |-
      Confirm the two design defaults recorded by this fix before the specification is fully complete:

      1. Keep persistent storage of composed or derived Markdown results disabled until a future content-freshness policy is designed.
      2. Put the future shared content-freshness vocabulary in a small dependency-light library used by Research, Darkmatter, and Claudine.

      For each item, either approve the recorded default or choose a different option from Q2/Q3 in the specification and require the design record to be updated. This decision does not block the implementation from reaching production readiness; it is a separate human design review.
reviewed_by: codex/gpt-5.6-sol
created: 2026-09-18T15:51:19-07:00
spec: 2026-09-16-content-policy-no-cache/spec.md
implemented: true
implemented_by: claude/opus
log: darkmatter/fixes/2026-09-16-content-policy-no-cache/implementation-log.md
description: A **fix** review of `2026-09-16-content-policy-no-cache/spec.md`
fix: 2026-09-16-content-policy-no-cache/review-2.md
previous: 2026-09-16-content-policy-no-cache/review-1.md
next: 2026-09-16-content-policy-no-cache/review-3.md
---

# Review 2: Content Policy No Cache

## Verdict

The fix is **not ready for production**. Review 1's functional blocker is
correctly closed: `biscuit-file` now combines every `Cache-Control` field line
for both GET and POST responses, and Darkmatter proves that a split-line
`no-store` response is neither persisted nor reused under every freshness mode
and TTL combination. However, the new `biscuit-file` regressions are excluded
from that package's canonical Level-1 and CI test contract because the optional
`fetch` feature is not enabled there. The changed POST path therefore has no
regression test in a normal gate.

All requirements in this specification concern in-process parsing, HTTP,
filesystem state, public APIs, or spawned-CLI behavior. Level 1 is the correct
verification boundary; no requirement needs real-terminal rendering or OS
keyboard injection, so Levels 2 and 3 are not applicable.

## Prior Review Closure

- **High — Split Cache-Control field lines can bypass no-store:** functionally
  implemented. `combined_cache_control` uses `HeaderMap::get_all` and preserves
  every field line for both `fetch` and `post`. The retained GET and POST
  integration tests prove that the fixture emits two field lines and that the
  API exposes `max-age=3600, no-store`. Darkmatter's retained matrix proves the
  second-line directive prevents persistence and reuse in Strict, Fallback,
  and Optimistic modes, with and without a TTL override.
- Review 1 had no `Blocked Findings` section. Its split-header human-review
  decision was unblocked by choosing and implementing the recommended shared
  `biscuit-file` fix. The Q2/Q3 design confirmations remain human-review items,
  but do not change the implementation-readiness assessment.

## Findings

### High — The canonical biscuit-file gate does not run the new cache-control regressions

`biscuit-file/lib/tests/fetch_integration.rs` declares
`required-features = ["fetch"]`, and the helper unit tests in
`file_reference/fetch.rs` are behind the same optional feature. The
`biscuit-file` manifest has no `[package.metadata.ci.tests]` declaration that
enables `fetch`. Consequently, `biscuit-file`'s canonical `just test` run
executes 813 default-feature tests but does not build or run any of the five
new regressions:

- `combined_cache_control_joins_every_field_line_in_order`
- `combined_cache_control_is_none_without_the_field`
- `combined_cache_control_keeps_a_non_ascii_line_readable`
- `fetch_combines_split_cache_control_lines_in_wire_order`
- `post_combines_split_cache_control_lines_in_wire_order`

Darkmatter's canonical suite does exercise the changed GET behavior through
its transport-cache matrix, but it does not exercise `biscuit_file::post`.
The implementation therefore closes the previous functional defect while
leaving part of that fix outside the normal cross-platform regression gate.
An explicit local feature-enabled run is useful evidence, but it does not make
the tests reachable in routine CI.

Required fix: add `features = ["fetch"]` under
`[package.metadata.ci.tests]` for `biscuit-file` so CI builds the feature, and
pass `--features fetch` for the library entry in the package area's `just test`
specification so the canonical local recipe selects the same tests. Verify
that both paths run the fetch unit and integration tests. Keep the existing
Darkmatter matrix; it proves the security-relevant storage outcome rather than
only the shared header representation.

## Requirement Verification Levels

| Acceptance criterion | Strongest verification present | Assessment |
| --- | --- | --- |
| 1. Local-only work does not create or mutate a cache root | Level 1 library and spawned-CLI filesystem tests | Pass; correct level |
| 2. Semantic results are recomputed and never persisted across controls and entry points | Level 1 library, reference-graph, structural, and spawned-CLI tests | Pass; correct level |
| 3. The CLI may reuse raw remote bytes while recomposing local content | Level 1 spawned-CLI test with a local HTTP server | Pass; correct level |
| 4. `no-cache` always revalidates and `no-store` is never persisted or reused, including split field lines | Level 1 Darkmatter matrix for GET; feature-gated `biscuit-file` GET/POST tests | Functional behavior passes, but the shared-layer regressions are absent from the canonical package gate |
| 5. Host policy is checked before cache access and a cache root grants no network authority | Level 1 runtime and spawned-CLI tests, including seeded entries | Pass; correct level |
| 6. Persistent manifests omit cleartext URL credentials and query values | Level 1 library and spawned-CLI persisted-byte inspection | Pass; correct level |
| 7. Help, builders, docs, and the Darkmatter skill describe the resolved boundary | Level 1 help test plus source review | Pass; correct level |
| 8. Production code cannot attach a semantic-result store to `RunLocalCache` | Level 1 structural source guard with a planted-violation self-test | Pass; correct level |

## Implementation Assessment

The split-header implementation is small and correctly owned by the shared
HTTP layer. Joining with `", "` preserves the RFC list-field representation,
and lossy decoding is fail-safer than dropping an otherwise readable line that
also contains `no-store`. Darkmatter still parses the combined value once and
keeps `StorableDirectives` as the type-level barrier to persistence. No further
ergonomics or performance change is warranted for this finding.

The implementation and documentation otherwise continue to satisfy the spec:
semantic-result persistence remains deleted, cache-root setup is lazy, host
authorization precedes cache access, transport failures surface as coded
warnings, and manifests redact credentials and query text.

## Verification

- `biscuit-file/just test`: passed, 813/813 default-feature Level-1 tests and
  6/6 no-default-features tests. This run also demonstrated the finding: none
  of the five new `fetch`-feature tests was selected.
- An explicit feature-enabled Nextest run selected all five new `biscuit-file`
  regressions: 5/5 passed. The implementation record also reports a mutation
  check in which keeping only the first header line made the boundary tests
  fail.
- A narrow Nextest run of Darkmatter's
  `split_line_no_store_is_never_stored_under_any_mode_or_ttl` passed (1/1),
  covering all six freshness/TTL combinations inside that test.
- `darkmatter/just test` compiled successfully, then fail-fast stopped after
  the known HTTP-client timeout cluster: 2,328 tests passed, 12 timed out, 14
  were skipped, and 5,961 were not run. The timeouts were in the pre-existing
  remote HTTP group documented by the implementation log; the changed
  split-line matrix passed separately with two test threads.
- GitNexus was bound explicitly to the `feat-dark-fixes` worktree and was
  current at `HEAD`. Indexed upstream impact for `fetch` and `post` was LOW and
  exact; the uncommitted helper itself was reviewed directly from source.

Cross-OS execution evidence is intentionally not a readiness finding; CI/CD
owns that evidence. The finding is instead that the relevant tests must first
be included in the package's CI contract so those OS legs can execute them.
