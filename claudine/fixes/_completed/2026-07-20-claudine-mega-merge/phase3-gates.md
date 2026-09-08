# Phase 3 Gate Evidence

All commands ran on macOS from the repository or named package area. Tests use
nextest through the repository `just` recipes. No write-mode formatter ran.

## Requirement-to-test execution

The complete behavior/variant mapping is in `phase3-test-map.md`. Focused exit
evidence included:

- 45 sequence JIT/preflight/guard tests passed.
- Both fail-closed Sequence Plus lifecycle containment L2 rows passed at `-j1`.
- All 17 `composition_seams` responsibility/duplicate-path guards passed.
- All 21 overlay-layering tests passed.
- Darkmatter caller-file provenance tests passed 3/3: eager scalar, eager
  array, and ordinary-string negative behavior.
- Launch-plan typed producer transport tests passed 2/2.
- All 18 typed-error/source-chain guards passed.
- All 5 effective-diagnostic rendering tests passed.
- The shipped implement-route corpus/fixture guard passed 3/3.
- The regenerated dispatch inventory passed all 12 owning tests.

The original failing overlay input was `spec.md` supplied through
`proxy.with` to a target declaring `spec: 'file(required;eager)'`. Its effective
frontmatter and rendered body now both contain the launch-area-resolved path.

## Broader gates

`cd claudine && just test` passed the complete area L1 set:

- `claudine-catalog-types`: 21 passed
- `claudine`: 3,952 passed, 7 skipped
- `claudine-contract`: 47 passed, 5 skipped
- `claudine-cli`: 2,313 passed, 246 skipped
- `claudine-gen`: 152 passed, 4 skipped

`cd claudine && just lint` passed, including all five crate clippy gates, all
18 error guards, and the lifecycle documentation-facet guard.

Integrity gates also passed:

- `git diff --check`
- tracked worktree conflict-marker scan
- untracked-inclusive conflict-marker scan
- index conflict-marker scan
- `git diff --cached --stat` (empty; no staging performed)
- GitNexus required comparison and isolated Phase 3 review, recorded in
  `impact/proxy-merge-detect.md`

## Retries, interim failures, and intentionally deferred evidence

- The final CLI run reported
  `structured_verbosity_controls_stream_stderr_lines` as timing-flaky; it
  passed on attempt 3. No assertion failure remained.
- An earlier run reported one nextest leaked-handle retry that passed on its
  second attempt.
- Broad-gate iterations exposed stale merged assertions for the coordinator
  boundary and the old wrapper-first error chain. Each was updated to the
  merged public contract, verified by focused regressions, and included in the
  final passing L1 run.
- The first lint iteration found `LaunchPlanError`'s restored diagnostic stored
  inline. Boxing that projection removed the size warning without changing its
  error chain; focused transport tests passed afterward and the full lint gate
  is clean.
- Full L2, L3, Linux CI, and native Windows runtime evidence belong to Phase 5
  of this plan. Phase 3 ran the two mandatory containment L2 regressions and
  does not represent later platform gates as complete.

There are no known pre-existing assertion or lint failures in the required
Phase 3 gates.
