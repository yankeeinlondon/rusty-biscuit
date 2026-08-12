# Phase 4 gate evidence

All required commands ran on macOS from the repository or the Claudine package
area. Tests used nextest directly or through the canonical `just` recipes. No
write-mode formatter ran.

## Requirement-to-test execution

The complete mapping and representation analysis is in `phase4-test-map.md`.
Focused results were:

- Dispatch inventory regeneration and passive corpus: 12/12 passed; the
  regenerated shipped JSON was byte-for-byte unchanged.
- `claudine-gen check`: all ten providers plus catalog, signals, vocabulary,
  families, and roster wiring were clean.
- Generator drift corpus: 6/6 passed.
- Source/error and test-placement guards: 27/27 passed.
- Library boundary/source guard: 7/7 passed.
- Composition ownership seams: 17/17 passed.
- Catalog-derived provider boolean and root-menu checks: 3/3 passed.
- Real completion invocation through `claudine __complete`: 1/1 passed.
- Provider catalog serialization and adapter coverage: 2/2 passed.
- `scripts/check-lifecycle-doc-facets.sh`: passed.

No targeted tests were added because Phase 4 changes no behavior. The existing
catalog-derived and passive-corpus tests already exercise the public output and
all shipped artifacts reconciled by this phase.

## Required broader gates

The final `cd claudine && just test` run exited 0:

- `claudine-catalog-types`: 21 passed
- `claudine`: 3,952 passed, 7 tier-skipped
- `claudine-contract`: 47 passed, 5 tier-skipped
- `claudine-cli`: 2,313 passed, 246 tier-skipped
- `claudine-gen`: 152 passed, 4 tier-skipped

`cd claudine && just lint` exited 0, including all five crate clippy gates,
all 18 error guards, and the lifecycle documentation-facet guard.

Integrity gates passed:

- `git diff --check`
- tracked worktree conflict-marker scan
- untracked-inclusive conflict-marker scan
- index conflict-marker scan
- empty `git diff --cached --stat` (no staging performed)
- required cumulative and isolated GitNexus change detection, recorded in
  `impact/reconciliation-detect.md`

## Retries, failures, and skipped evidence

- The first compact `just test` attempt reached the CLI package, where the
  pre-existing timing-flaky
  `structured_verbosity_controls_stream_stderr_lines` exhausted its three
  30-second attempts. Fail-fast therefore left 1,734 CLI tests and the
  generator package unrun in that attempt.
- A canonical isolated `just test-cli --no-fail-fast` rerun passed all 2,313
  L1 tests; the same timing test passed on attempt 3 after two timeouts.
- The final required `just test` run passed every package. The same timing test
  again passed on attempt 3 after two timeouts. Phase 3 already records this
  test as timing-flaky; no assertion failure occurred.
- One focused error-guard run reported a leaked-handle retry for
  `from_code_projects_a_catalog_shaped_detail_for_every_registered_code`; it
  passed on attempt 2 and passed normally in the final lint gate.
- An exploratory raw `cargo nextest run -p claudine-cli --no-fail-fast` omitted
  the canonical L1 exclusion filter, unintentionally admitted L2 terminal
  tests without the `just test-l2` broker/serialization setup, and was
  interrupted with exit 130. It is not gate evidence; the canonical L1 command
  above is the required Phase 4 verification. Full L2 belongs to Phase 5.
- The skipped counts above are the canonical L2/L3/browser/real/slow tier
  exclusions. No required Phase 4 assertion or lint failure remains.
