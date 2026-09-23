---
$schema: feature-review.yaml
ready: true
human_review: false
reviewed_by: codex/gpt-5.6-sol
created: 2026-09-17T21:26:50-07:00
spec: 2026-09-09-more-context/spec.md
implemented: false
description: A **fix** review of `2026-09-09-more-context/spec.md`
fix: 2026-09-09-more-context/review-5.md
previous: 2026-09-09-more-context/review-4.md
---

# Review 5: More Context

## Verdict

The feature is **ready for production**. Both unblocked findings from review 4
are fully implemented and directly regression-tested. Review 4 had no blocked
findings, so none were available to become unblocked before this iteration.

This verdict does not depend on cross-OS execution evidence or human review.
No new product decision or activity that only a human can perform remains.

## Prior Review Closure

- **Capture the ambient repository observation at the actual request
  boundary:** implemented. The public `ComposeOptions::for_document` constructor
  captures the root document's demand-driven context and fixes the request's
  repository observation before validation, preflight, or composition. The
  `md compose` path uses that constructor. Older constructors establish the
  observation unconditionally at the root pipeline entry, before any stage;
  child pipelines no longer establish request state. Regressions create the
  request through the public constructor, mutate repository metadata and
  topology, and prove later phases retain the creation-time values without
  another discovery. A separate descendant-only case mutates the repository
  during an earlier root shell stage and proves the transcluded child reads the
  already-fixed observation.
- **Finish the AC29 documentation contract and correct repository freshness
  claims:** implemented. The required Claudine context-variable page now
  explains eager `ctx`, lazy `current`, lazy `current_env`, the
  `ctx.recent_commits` / `recent_commits(count)` pair, direct-root spelling,
  and the fixed-versus-refreshable distinction. The stale nonexistent
  `repo_name` example and repository-rename claim were replaced with the real
  `branch` example and the request-fixed repository contract. A positive,
  non-vacuous Level 1 documentation contract covers every page named by AC29,
  complementing the existing removed-spelling migration guard.

## Findings

None.

## Blocked Findings

None.

## Requirement Verification Levels

No requirement asserts terminal glyph geometry, SGR styling, scrolling,
keyboard input, paste/IME, or mouse behavior. Level 3 is not applicable. Level
2 is required only by AC2 and AC28's explicit real-Claudine boundary; AC14 is a
real network-resource gate rather than a terminal-verification tier.

| Requirements | Required level | Strongest evidence and assessment |
| --- | --- | --- |
| AC1, AC19, AC26 | Level 1 passive corpus, generator drift, and listing CLI | Present at the correct level. |
| AC2 | Level 2 real Claudine | Linked/main-worktree coverage was accepted in review 2; this iteration did not change that behavior. Correct level. |
| AC3-AC5, identity part of AC36 | Level 1 fixed vectors and normal compose/CLI paths | Hash parity, root retention, uniqueness, fixed vectors, and typed entropy failure are present. Correct level. |
| AC6-AC8, AC33 | Level 1 deterministic ICMP transport/policy fixtures | Present at the correct level. |
| AC9, AC13 | Level 1 interface/route fixtures and projection | Present at Level 1; cross-OS execution belongs to CI. Correct level. |
| AC10-AC12, AC34 | Level 1 controlled shell/profile fixtures | Present at the correct level; no terminal window or real user profile is required. |
| AC14 | Real network resource on each required OS | The bounded real-resource tests exist; cross-OS results do not gate this review's readiness decision. |
| AC15-AC18, AC20, AC32, AC35 | Level 1 compose/function fixtures | Present at the correct level. |
| AC21-AC23 | Level 1 ambient and supplied-evidence fixtures | Present at the correct level. |
| AC24-AC25, AC37 | Level 1 fixed repositories and CLI-format parity | Present at the correct level. |
| AC27, AC31 | Level 1 scripted provider, public request boundary, pipeline, and work counters | Public-boundary and descendant-only mutation regressions now prove request-time repository fixation and zero downstream rediscovery. Correct level. |
| AC28 | Level 2 real Claudine, supported by Level 1 executor/provider tests | The previously accepted focus-safe tmux suite covers the shipped `plan.md` and `format.md` lifecycle behavior. This iteration did not change that boundary. Correct level. |
| AC29 | Level 1 passive documentation and migration guards | The required user page and skills now describe the binding model accurately; the positive phrase contract is non-vacuous and the removed-spelling guard remains present. Correct level. |
| AC30 | Level 1 source-kind and request-count fixtures | URL-root request counting proves one caller fetch, one child fetch, and no root refetch. Correct level. |
| Cache portion of AC36 | Level 1 cold/warm cache fixture | CLI and library tests prove fresh identity/probe output and no composed, operation, or snapshot persistence. Correct level. |

## Verification

- Focused request-boundary run passed: 3 tests executed, 3 passed, with 5,876
  unrelated library tests filtered out.
- Focused AC29 guards passed: 4 tests executed, 4 passed, 0 skipped.
- `cd darkmatter && just test --color=never` passed: 8,098 tests executed,
  8,098 passed, 14 higher-tier tests skipped.
- `cd darkmatter && just lint` passed for `darkmatter`, `darkmatter-cli`,
  `dmls`, `zed-dmls-cli`, and the `wasm32-wasip2` extension check.
- The review did not rerun Level 2 because this iteration changed only
  request-boundary timing and passive documentation. Review 4 recorded the
  applicable tmux suite passing 5/5 with no skips, and the Level 2 behavior was
  not modified.
- GitNexus was queried for the request-owned repository-observation flow. Its
  branch index found the new types but no execution process, and impact lookup
  for both changed symbols returned `UNKNOWN`; direct source and call-site
  inspection therefore supplied the review evidence rather than treating an
  unresolved graph walk as an all-clear.
- `git diff --check` is not clean because the pre-existing, unrelated
  `prompts/_implement/implement-plan.md` changes contain trailing whitespace at
  lines 40 and 57. The three review/spec files changed by this review have no
  whitespace errors.

## Production Readiness

Ready for production. The final two findings are closed, every user-observable
requirement retains verification at the appropriate level, the complete
Darkmatter Level 1 and lint gates pass, and no human review is required by this
specification.
