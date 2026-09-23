---
$schema: feature-review.yaml
ready: false
findings:
  - title: Restore Darkmatter's all-target build after moving the performance fixtures
    priority: high
  - title: Keep ambient current repository facts on the request-owned observation
    priority: high
  - title: Exercise prompts/format.md at the Level 2 boundary required by AC28
    priority: high
  - title: Finish the explicit Level 1 acceptance guards
    priority: medium
human_review: false
human_review_items:
  - |-
      Confirm the execution-identifier choice already adopted by the specification: include a fresh random value in each execution's identifiers so two otherwise identical runs differ. The alternatives are repeatable identifiers that may collide between runs, or secret-key identifiers that also require key management. The adopted random-value option is recommended.
  - |-
      Confirm when live context values should refresh. The adopted choice keeps repeated reads of one value consistent within one expression, then refreshes it in the next expression. The alternatives are keeping the first value throughout the document or refreshing on every read. The adopted per-expression choice is recommended.
  - |-
      Confirm the persistent-cache boundary already adopted by the specification: save only raw downloaded responses and never save composed documents, operation results, or context snapshots. The alternative is to disable persistence of downloaded responses too until a separate freshness policy is designed.
  - |-
      Confirm how the obsolete `current.ctx.*` and `current.env.*` spellings should be guarded against returning. The recommended option is an automated allowlist that permits only documentation and tests which explicitly explain or reject the removed spellings, and fails for every new occurrence. The alternative is to remove even explanatory and negative-test occurrences so a literal repository search returns no matches.
reviewed_by: codex/gpt-5.6-sol
created: 2026-09-17T17:33:35-07:00
spec: 2026-09-09-more-context/spec.md
implemented: true
description: A **fix** review of `2026-09-09-more-context/spec.md`
fix: 2026-09-09-more-context/review-2.md
previous: 2026-09-09-more-context/review-1.md
next: 2026-09-09-more-context/review-3.md
---

# Review 2: More Context

## Verdict

The feature is **not production ready**. The three implementation failures from
review 1 are now substantially resolved, and the focused Level 1 and required
AC2/AC28 Level 2 tests pass. However, the Darkmatter package area's canonical
test and lint gates no longer compile all targets, the ambient `md compose`
implementation violates the request-owned repository observation contract for
`current.*`, and AC28 still lacks its explicitly required real-Claudine test of
`prompts/format.md`.

This verdict does not depend on missing cross-OS execution evidence or the
pending human confirmation of Q1-Q3.

## Human Review Rulings

Ken ruled on all four `human_review_items` on 2026-09-17; the rulings are
recorded in `spec.md` as R34-R37 and the items are closed.

- Execution identifiers: the adopted per-execution random nonce is confirmed
  (**R34**). No rework.
- Live-value refresh: the adopted per-expression, per-key memo scope is
  confirmed (**R35**). No rework.
- Persistent-cache boundary: raw remote-URL response bodies only, never
  composed documents, operation results, or snapshots (**R36**). No rework.
  The same ruling answers Q1 of `fixes/2026-09-16-content-policy-no-cache`.
- Obsolete-spelling guard: an automated allowlist test keyed by file path and
  expected occurrence count, with stale-entry detection (**R37**). AC29 was
  amended to match; the test is part of the "Finish the explicit Level 1
  acceptance guards" finding.

## Prior Review Closure

- **Complete the capture-group integration so Darkmatter compiles:** implemented.
  `ContextGroup` now owns the Document, GitHistory, and Network groups and the
  focused feature tests compile and pass. The new all-target compile failure is
  a different stale-fixture-path regression described below.
- **Implement the thirteen expression functions that still deliberately fail:**
  implemented. The pending dispatcher is gone and the repository, history,
  network, shell, agentic-CLI, and nested-composition handlers have positive and
  adversarial coverage.
- **Implement the lazy roots and replace Claudine's obsolete lifecycle shape:**
  implemented, subject to the ambient repository-observation defect below.
  Claudine installs its invocation provider, `current_env` is late-bound, and
  the obsolete nested paths are rejected.
- **Add the missing Level-1 acceptance matrix and required Level-2 Claudine
  coverage:** partially implemented. The focused Level 1 matrix is broad and
  five AC2/AC28 Level 2 tests pass, but `prompts/format.md` and the explicit
  Level 1 guards identified below remain uncovered.

Review 1 contained no `Blocked Findings` section, so no previously blocked
finding needed reclassification before this iteration.

## Unblocked Findings

### High — Restore Darkmatter's all-target build after moving the performance fixtures

The performance-followup feature was moved under `features/_completed`, but two
active tests still use its old path. `cli/tests/compose_transclusion.rs:70-78`
uses `include_str!` on the removed location, so both `just test` and `just lint`
fail while compiling `darkmatter-cli`. The library corpus test at
`lib/src/markdown/compose/directives_api.rs:703-709` also reads the removed
directory and fails at runtime.

This invalidates the earlier implementation-log claim that the canonical gates
pass and prevents the package area from compiling all declared test targets.
Point both consumers at the completed feature's durable location, or move the
fixtures into a non-lifecycle test-fixture directory owned by the tests. The
latter is preferable because completing a feature should never invalidate an
active test dependency.

### High — Keep ambient current repository facts on the request-owned observation

`CurrentProvider` explicitly prohibits rediscovering the repository root or
package topology (`context/current.rs:69-75`). Nevertheless, the default
`AnchoredRefresh::refresh` calls `capture_runtime_context_for_groups` for every
key (`context/current.rs:99-105`). For `current.repo`,
`current.repo_root`, `current.packages`, and the other Repo-group members, that
constructs a fresh `ContextCapture` and performs repository discovery again.
`ComposeOptions::current_authority` installs this provider for every
Darkmatter-owned request (`context/options.rs:1418-1426`).

The fixed anchor prevents CWD drift, but it does not preserve the request's
repository observation: adding/removing repository metadata or changing package
topology during a compose can make `current.*` switch observations, and every
read pays discovery work that D3 forbids. Claudine's provider correctly retains
its launch repository and its zero-rediscovery test does not exercise this
ambient provider. Retain the Darkmatter-owned request's repository observation
and resolution catalog in the provider, answer fixed repository facts from it,
and add a work-counter test that fails if ambient `current.repo_root` or a
package-topology key performs discovery.

### High — Exercise prompts/format.md at the Level 2 boundary required by AC28

AC28 requires every migrated shipped prompt, explicitly including
`prompts/format.md`, to compose through a real Claudine run. The new Level 2
shipped-prompt test reads only `prompts/plan.md`
(`level2_ac28_lifecycle_lazy_roots.rs:512-540`). No test reads or executes
`prompts/format.md`, whose lifecycle handler uses
`length(current.dirty_files)`.

The passing `plan.md` case proves DateTime refresh wiring, not the FileChanges
capability used by `format.md`; an omitted or broken FileChanges refresh path
could therefore ship while the Level 2 suite stays green. Add a headless tmux
case that runs the real `format.md` prompt against a fixture repository with a
known dirty-file set and distinguishes successful `current.dirty_files`
resolution from an empty/unsupported value. This is a missing Level 2 proof for
an explicitly user-observable requirement and is therefore a high-severity
readiness gap.

### Medium — Finish the explicit Level 1 acceptance guards

Several acceptance clauses remain asserted only indirectly or not at all:

- AC4 requires an entropy failure to surface as the typed
  `ExecutionNonceUnavailable` compose error. The injected-failure test stops at
  the internal `PartialRuntimeCapture` diagnostic
  (`capture/document.rs:281-297`) and never evaluates `ctx.id` or `ctx.sid`
  through a compose surface.
- AC29 requires an automated scoped migration check. No such test exists, and a
  literal zero-match grep is incompatible with the current intentional
  negative tests and documentation that explain the removed spelling. Adopt a
  reviewed allowlist with stale-entry detection, or amend the criterion and
  remove every explanatory occurrence; do not leave the clean-break guarantee
  as an unaudited manual search.
- AC30's URL-root “no extra fetch” clause has no request-count assertion.
- AC36 requires warm `--cache-root` runs to demonstrate that neither execution
  identity nor probe output is replayed. The current cache tests use only
  `ctx.timestamp_ms` (`persistent_cache_disabled.rs:25-35,77-115`). Their
  no-local-artifact assertions are strong general evidence, but the named
  identity/probe regression inputs are absent.

Add focused Level 1 tests through normal composition boundaries. These do not
need a real terminal, browser, network, or OS input injection.

## Blocked Findings

None. The findings above are implementable without external access or cross-OS
execution. The human decisions remain post-readiness review items and do not
block the implementation/fix cycle.

## Requirement Verification Levels

No requirement involves terminal glyph geometry, SGR styling, keyboard input,
paste/IME, mouse behavior, or scrolling. Level 3 is therefore not applicable.
Level 2 is required only where AC2 and AC28 explicitly require a real Claudine
run; all other deterministic behavior belongs at Level 1, while AC14 is a real
network-resource test rather than a terminal tier.

| Requirements | Required level | Strongest evidence and assessment |
| --- | --- | --- |
| AC1, AC19, AC26 | Level 1 passive corpus, generator drift, and listing CLI | Implemented, but the canonical Darkmatter all-target gate cannot currently compile. |
| AC2 | Level 2 real Claudine | `level2_ac2_worktree_compose` passed for linked and main worktrees. Correct level. |
| AC3-AC5, AC30, identity parts of AC36 | Level 1 fixed vectors and normal compose/CLI paths | Hash parity, retained identity, transclusion, uniqueness, and vectors are covered. Entropy-to-compose-error and URL request-count clauses remain gaps. |
| AC6-AC8, AC33 | Level 1 deterministic ICMP transport/policy fixtures | Implemented at the correct level; focused tests passed. |
| AC9, AC13 | Level 1 interface/route fixtures and projection | CGNAT range and six OS route-parser fixture families are covered at Level 1. |
| AC10-AC12, AC34 | Level 1 controlled shell/profile fixtures | Implemented at the correct level; no real user profile or focused window is required. |
| AC14 | Real network resource on each required OS | The real-resource tests exist. Cross-OS result collection belongs to CI and does not affect this verdict. |
| AC15-AC18, AC20, AC32, AC35 | Level 1 compose/function fixtures | Implemented at the correct level; focused nested, repository, agent, address, preflight, and path tests passed. |
| AC21-AC23 | Level 1 ambient and supplied-evidence fixtures | Both scope paths and conditional truthiness are covered at Level 1. |
| AC24-AC25, AC37 | Level 1 fixed repositories and CLI-format parity | Eager/lazy history, mutations, empty commits, invalid counts, and formatter parity are covered at Level 1. |
| AC27, AC31 | Level 1 scripted provider, pipeline, and work counters | Lazy-root surfaces, memo scope, fail-closed behavior, and Claudine no-rediscovery pass. Ambient repository refresh violates the fixed-observation contract and lacks a counter test. |
| AC28 | Level 2 real Claudine, supported by Level 1 executor/provider tests | All five new Level 2 tests passed, covering every lifecycle event, environment and branch freshness, AC2, and `plan.md`. `format.md` is untested at Level 2. **Wrong/missing level; high gap.** |
| AC29 | Level 1 passive documentation/migration guard | Documentation is substantially migrated, but the required automated drift guard is absent. |
| Cache portion of AC36 | Level 1 cold/warm cache fixture | General no-local-artifact tests exist; the criterion's identity and probe-output cases are not explicit. |

## Verification

- `cargo nextest run -p darkmatter --features effects-instrumentation` reached
  the moved-fixture corpus test: 935 passed, 1 failed, then 5,730 were canceled.
- A focused Darkmatter feature filter passed 118 of 118 Level 1 tests.
- A focused Claudine filter passed 43 of 43 Level 1 tests.
- The AC2/AC28 tmux binaries passed 5 of 5 Level 2 tests with
  `BISCUIT_TEST_REQUIRED_BACKENDS=tmux`; none skipped and no window was opened.
- `cd darkmatter && just test` and `just lint` both fail on the two stale
  `include_str!` paths in `compose_transclusion.rs`.
- GitNexus was bound to `rusty-biscuit`, but its index was stale. The required
  `just gitnexus` refresh could not acquire the index because a pre-existing
  `gitnexus analyze --watch` process had held it for more than 14 hours. The
  graph returned UNKNOWN for `AnchoredRefresh`, so direct call-site search was
  used as required for an unresolved graph result.
