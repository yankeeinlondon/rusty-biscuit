---
$schema: feature-review.yaml
ready: true
human_review: true
human_review_items:
  - |-
      Confirm the two design defaults recorded by this fix before the specification is fully complete:

      1. Keep persistent storage of composed or derived Markdown results disabled until a future content-freshness policy is designed.
      2. Put the future shared content-freshness vocabulary in a small dependency-light library used by Research, Darkmatter, and Claudine.

      For each item, either approve the recorded default or choose a different option from Q2/Q3 in the specification and require the design record to be updated. This decision does not block production readiness; it is a separate human design review.
reviewed_by: codex/gpt-5.6-sol
created: 2026-09-18T18:49:19-07:00
spec: 2026-09-16-content-policy-no-cache/spec.md
implemented: false
description: A **fix** review of `2026-09-16-content-policy-no-cache/spec.md`
fix: 2026-09-16-content-policy-no-cache/review-3.md
previous: 2026-09-16-content-policy-no-cache/review-2.md
---

# Review 3: Content Policy No Cache

## Verdict

The fix is **ready for production**. Review 2's only unblocked finding is
closed: Biscuit File's canonical local Level-1 recipe enables the `fetch`
feature, and its CI metadata declares the same feature. Both paths therefore
select the helper unit tests and `fetch_integration` target that protect the
shared GET and POST `Cache-Control` behavior.

The implementation continues to satisfy the specification's semantic-result
and transport-cache boundary. No new correctness, ergonomics, performance, or
test-coverage finding was identified.

All requirements in this specification concern in-process parsing, HTTP,
filesystem state, public APIs, or spawned-CLI behavior. Level 1 is the correct
verification boundary; no requirement needs real-terminal rendering or OS
keyboard injection, so Levels 2 and 3 are not applicable.

## Prior Review Closure

- **High — The canonical biscuit-file gate does not run the new cache-control
  regressions:** implemented. `biscuit-file/justfile` now passes `--features
  fetch` for the `biscuit-file` library in the package area's canonical
  `just test` recipe. `biscuit-file/lib/Cargo.toml` now declares
  `features = ["fetch"]` under `[package.metadata.ci.tests]`; the canonical CI
  planner forwards that metadata into both check and test arguments. The local
  recipe selected 838 tests rather than the earlier 813, including all three
  `combined_cache_control` unit tests and both split-header GET/POST integration
  tests.
- Review 2 had no `Blocked Findings` section, so no blocked implementation
  finding could have become actionable between iterations. Its Q2/Q3 items
  remain explicit human-review decisions and do not block production
  readiness.

## Findings

None.

## Requirement Verification Levels

| Acceptance criterion | Strongest verification present | Assessment |
| --- | --- | --- |
| 1. Local-only work does not create or mutate a cache root | Level 1 library and spawned-CLI filesystem tests | Pass; correct level |
| 2. Semantic results are recomputed and never persisted across controls and entry points | Level 1 library, reference-graph, structural, and spawned-CLI tests | Pass; correct level |
| 3. The CLI may reuse raw remote bytes while recomposing local content | Level 1 spawned-CLI test with a local HTTP server | Pass; correct level |
| 4. `no-cache` always revalidates and `no-store` is never persisted or reused, including split field lines | Level 1 Darkmatter storage matrix plus canonically selected Biscuit File GET/POST tests | Pass; correct level |
| 5. Host policy is checked before cache access and a cache root grants no network authority | Level 1 runtime and spawned-CLI tests, including seeded entries | Pass; correct level |
| 6. Persistent manifests omit cleartext URL credentials and query values | Level 1 library and spawned-CLI persisted-byte inspection | Pass; correct level |
| 7. Help, builders, docs, and the Darkmatter skill describe the resolved boundary | Level 1 help test plus source review | Pass; correct level |
| 8. Production code cannot attach a semantic-result store to `RunLocalCache` | Level 1 structural source guard with a planted-violation self-test | Pass; correct level |

## Implementation Assessment

The two gate changes are minimal and align local and CI coverage without
changing Biscuit File's default feature set for downstream consumers. Cargo
metadata exposes `ci.tests.features = ["fetch"]`, and the planner's regression
test proves declared features become both qualified check arguments and test
arguments. The package-area recipe independently makes the same selection for
local development.

The underlying split-header implementation remains correctly owned by the
shared HTTP layer. `combined_cache_control` consumes every field value with
`HeaderMap::get_all`, preserves order, and is used by both `fetch` and `post`.
Darkmatter parses the combined directive list and retains its type-level
barrier preventing a `no-store` response from reaching persistence.

GitNexus was bound to the `feat-dark-fixes` worktree and current at its HEAD.
Indexed upstream impact for `fetch` and `post` was LOW and exact; `post`
reaches Darkmatter's `EffectEngine::http_post`, confirming that keeping the POST
regression in the normal gate is material rather than redundant.

## Verification

- `biscuit-file/just test` selected 838 Level-1 tests, including all five
  cache-control regressions. It passed 825 non-HTTP tests; all 13 Wiremock tests
  reached the repository's 30-second local timeout together under concurrent
  host load.
- The documented contention rerun,
  `cargo nextest run -p biscuit-file --features fetch --test fetch_integration
  --test-threads 2`, passed 13/13, including
  `fetch_combines_split_cache_control_lines_in_wire_order` and
  `post_combines_split_cache_control_lines_in_wire_order`.
- `biscuit-file/just lint` passed for the library and CLI.
- The CI planner regression
  `MatrixRecordTests.test_features_become_qualified_check_and_test_args`
  passed, and `cargo metadata` reported the Biscuit File CI feature contract as
  `["fetch"]`.

Cross-OS execution evidence is intentionally not a readiness finding; CI/CD
owns that evidence. The implementation and tests use portable Cargo/Just
feature selection and introduce no OS-specific behavior.
