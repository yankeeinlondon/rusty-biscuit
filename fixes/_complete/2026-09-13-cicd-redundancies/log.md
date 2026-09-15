---
implementation_2: "2026-09-14T19:32:11-07:00"
implementation_3: "2026-09-15T03:20:04-07:00"
implementation_4: "2026-09-15T03:48:52-07:00"
implementation_5: "2026-09-15T04:24:33-07:00"
implementation_6: "2026-09-15T04:42:45-07:00"
---

## Implementation of Review Findings #2

> **started at:** 2026-09-14T19:32:11-07:00

- this implementation is attempting to implement _all_ of the review findings found in '/Volumes/coding/wt/rusty-biscuit/fix-cli-slow-tests/fixes/_complete/2026-09-13-cicd-redundancies/review-2.md'
- this is iteration 2 of the review-to-implement cycle
- orchestration plan decided after reading the review, the specification, and the implementation surfaces the review cites:
        - finding 1 (high, `ci-reporting` reuse mode emits a run-level `CLEAR`) — implementable here; touches `.github/workflows/ci.yml` and the workflow-source contracts in `tools/test-toolkit`
        - finding 2 (high, reporting modes overlap and cancellations are misclassified) — implementable here; touches the same `ci-reporting` job plus a new executable shell fixture
        - finding 3 (high, hosted GitHub Actions behavior unverified) — expected to defer: its only valid boundary is a hosted Actions run, which requires a push/workflow dispatch this session is not authorized to perform
        - finding 4 (medium, sub-second lint durations render as `not recorded`) — implementable here; touches `.github/workflows/_package-ci.yml`, `scripts/ci-rollup.rs`, and the Python producer-status schema
        - findings are worked serially by one subagent each; gates run only in the areas the specification's Validation 2 names: the `scripts/ci/*.py` suites, the `scripts` Nextest bins (`repo-deps`), and `tools/test-toolkit`
- starting the work on 'The advisory reuse report makes a run-level CLEAR claim' at 19:32:30
        - discovered: the Rust advisory renderer's ban list lives in `scripts/ci-rollup-tests.rs` (`the_report_makes_no_merge_or_policy_claim` and the twin list in `the_combined_summary_aggregates_area_slices_and_applies_no_policy`) and is `["BLOCKED", "CLEAR —", "must not merge", "may merge", "baseline-", "policy-gap-", "## CI verdict", "cicd"]`
        - discovered: applying that list verbatim to the `ci.yml` job text would have been **vacuous** — the workflow wrote `**CLEAR** — …`, so the literal `CLEAR —` is not a substring because the emphasis markers sit between `CLEAR` and the em dash; the new contract therefore strips Markdown emphasis, code spans, and the shell's backslash escaping before matching, keeping the vocabulary identical to the renderer's while `**CLEAR**` cannot evade it
        - discovered: `job_block("ci.yml", "  ci-reporting:")` starts at the job header, so the long `# ADVISORY …` comment block preceding the job — which legitimately quotes the retired `No gate reported a failure` line — belongs to the `ci-gate:` block and is not scanned; the existing `report.contains("ci-gate")` assertion still holds after the reuse prose stopped naming it, because mode 3 names `ci-gate` twice
        - non-vacuity: the extended contract was written first and run against the unmodified pre-fix `ci.yml`, failing at `tools/test-toolkit/tests/ci_workflow_contracts.rs:2338` with `the ci-reporting job applied policy ("CLEAR —")` — `1 test run: 0 passed, 1 failed, 102 skipped`; after the `ci.yml` fix the same invocation reported `1 passed, 102 skipped`
        - changed `.github/workflows/ci.yml`: the `ci-reporting` job's "Reuse successful PR validation" step now states only the three facts section 6 allows — same Git tree and integration base as a successful prior PR validation, this run executed no package cell, the linked run's results are authoritative — plus the link; removed `**CLEAR**`, the claim that `ci-gate` "folds only skipped and successful jobs", and the "every area job was skipped" conclusion derived from job results
        - changed `tools/test-toolkit/tests/ci_workflow_contracts.rs`: added a file-level `const POLICY_VOCABULARY: [&str; 8]` holding the renderer's exact ban list (doc comment names `scripts/ci-rollup-tests.rs` as its source) and extended `only_ci_gate_makes_a_run_level_claim` to assert, for every banned word, both that the literal is still present in `scripts/ci-rollup-tests.rs` (so the two vocabularies cannot silently diverge) and that the normalized `ci-reporting` job body does not contain it; the pre-existing `No gate reported a failure`, `needs.area-ci.result`, and `ci-gate`/`ci-verdict` assertions were left untouched
        - gates: `just _test test-toolkit` 179 passed, 2 skipped; `just _lint test-toolkit` clean; `actionlint .github/workflows/ci.yml` clean
        - note: cargo runs used an isolated `CARGO_TARGET_DIR` because of the pre-existing non-writable files in the shared `target/debug/deps`; the user's cache was not deleted or modified
- work completed for 'The advisory reuse report makes a run-level CLEAR claim' at 19:36:17
- starting the work on 'The reporting modes overlap and cancellations are misclassified' at 19:36:30
        - discovered: mode 3's guard (`reuse != 'true'`) was a strict superset of mode 2's (`reuse != 'true' && scope.result == 'success'`), so every normal successful scoped run rendered both the full rollup report and a second "Jobs outside the rollup / No bootstrap stage failed" section
        - discovered: a second hole in the same guards — the corner `reuse == 'true' && validation.result != 'success'` had no mode at all (mode 1 declines it; modes 2 and 3 both require `reuse != 'true'`), producing a run with no report
        - discovered: mode 3's loop matched only the literal `failure`, so `cancelled` fell through to the empty-`first` branch; confirmed by executing the pre-fix script with `RESULTS='bootstrap (reuse check):cancelled…'`, which rendered `No bootstrap stage failed.`
        - discovered: `scope`'s own guard is `!cancelled() && needs.validation.outputs.reuse != 'true'` and `validation.outputs.reuse` is `steps.reuse.outcome == 'success' && steps.reuse.outputs.reuse || 'false'`, so `reuse == 'true'` with a non-successful validation is near-unreachable in practice — which is why mode 3 is now expressed as a negation rather than an enumerated condition
        - discovered: `post_merge_reuse_preserves_the_gate_and_normal_ci_fallback` (`ci_workflow_contracts.rs:2274-2281`) asserts every report step after the first contains the literal `needs.validation.outputs.reuse != 'true'`; the negation form contains that literal inside its second clause, so that contract is untouched and still true
        - non-vacuity: against a pre-fix `ci.yml`, `exactly_one_ci_reporting_mode_speaks_for_every_bootstrap_state` failed with `validation=success reuse=false scope=success: mode 2 alone must report this run, but [2, 3] did`, and `the_bootstrap_report_names_cancellation_instead_of_denying_a_failure` failed with `validation=cancelled, scope=skipped: the report must state "First actionable failure class: bootstrap (reuse check) (cancelled)"` against a rendered `No bootstrap stage failed.`; the pre-fix file was restored byte-for-byte and `git diff --stat` matched before and after
        - changed `.github/workflows/ci.yml`: mode 3's guard is now the negation of modes 1 and 2, making the three modes mutually exclusive **and** exhaustive, with one comment at the guard naming the two holes the hand-rolled condition had; mode 3's loop treats `cancelled` as actionable alongside `failure` and renders the state it found as `**First actionable failure class: ${first} (${state})**`; mode 3 now lists each stage's observed state before its verdict line so `skipped` is distinguishable from `cancelled`; the neither-failed-nor-cancelled branch no longer claims "No bootstrap stage failed" but states that no stage failed or was cancelled yet scope did not succeed
        - changed `.github/workflows/ci.yml` comments in the same pass: the job header comment now says "a bootstrap that failed or was cancelled" and states the exclusive-and-exhaustive property; the `RESULTS` env comment now says "failure or cancellation"
        - changed `tools/test-toolkit/tests/ci_workflow_contracts.rs`: added a small scoped GitHub-expression evaluator (`GuardToken`, `tokenize_guard`, `GuardParser`, `eval_guard`) covering exactly `!`, `&&`, `||`, parentheses, and `==`/`!=` between a `needs.…` path and a quoted literal — an unmodelled path panics rather than defaulting, so a guard that starts reading new state fails here instead of evaluating as absent
        - changed `tools/test-toolkit/tests/ci_workflow_contracts.rs`: `reporting_mode_guards()` reads the guards out of `ci.yml` rather than hardcoding them; `exactly_one_ci_reporting_mode_speaks_for_every_bootstrap_state` covers 7 named cases plus an exhaustive 4x2x4 = 32-combination sweep; `the_bootstrap_report_names_cancellation_instead_of_denying_a_failure` (`#[cfg(unix)]`) executes mode 3's real `run:` body under `bash` with `env_clear()` and `GITHUB_STEP_SUMMARY` in a temp dir, for cancelled validation, failed validation, cancelled scope, and neither, and asserts no `${{` survives its substitution so a newly added stage fails rather than going unmodelled
        - the pre-existing source-only test `ci_summarizes_the_first_actionable_failure_class` was left unchanged — every literal it checks still holds
        - preserved from the previous finding: the reuse step's non-verdict prose and the `POLICY_VOCABULARY` contract; the new mode-3 prose passes `only_ci_gate_makes_a_run_level_claim`
        - gates: `just _test test-toolkit` 181 passed, 2 skipped; `just _lint test-toolkit` clean (zero warnings); `actionlint .github/workflows/ci.yml` clean
        - `repo-deps` and the Python suites were correctly not run: neither `scripts/ci-rollup.rs` nor `scripts/ci/*.py` was touched
- work completed for 'The reporting modes overlap and cancellations are misclassified' at 19:45:09
- starting the work on 'Sub-second lint measurements are reported as unavailable' at 19:45:30
        - GitNexus impact run before editing, as the repo requires:
                - `Struct:scripts/ci-rollup.rs:Cell` — 6 impacted, risk **LOW**; direct callers `blank_cell`, `classify_one`, `status_cells`, then `classify`, `cmd_rollup`, `run`, all inside `scripts/ci-rollup.rs`
                - `Struct:scripts/ci-rollup.rs:ProducerStatus` — 0 impacted, risk **UNKNOWN**; treated as unresolved per CLAUDE.md and confirmed by text search: the symbol appears only in `scripts/ci-rollup.rs` (11 sites) and `scripts/ci-rollup-tests.rs` (16 sites), nothing under `.github/`, `tools/`, or the Python suites
                - `render_lint_report` / `render_environment_report` — 3 impacted each, risk LOW, module `Scripts` only
                - post-change `detect-changes --scope all`: 12 files / 35 symbols, 0 affected processes, risk low, neither `partial` nor `truncated`
        - discovered: the defect is two-layered and the reviewer's `> 0` heuristic is only the second layer — reverting the renderer heuristic alone still let `Some(0.42)` render correctly, because the sub-second loss actually came from `u64` storage; the non-vacuity mutation therefore reproduces both truncation and the heuristic
        - discovered: `Cell.duration_s` is consumed nowhere outside `scripts/ci-rollup.rs`; the `duration_s` hits in `scripts/ci/local_evidence.py`, `scripts/ci/schema.py`, `tools/test-audit/*`, `.githooks/tests/test-pre-push.sh`, and `tools/test-toolkit/tests/junit_staging_contracts.rs` belong to two different documents — the local receipt cell and the JUnit `manifest.jsonl` record — neither of which was touched
        - non-vacuity (workflow): the real pre-fix Lint step body, restored from `git show HEAD:.github/workflows/_package-ci.yml`, failed `the_lint_step_measures_a_sub_second_command_instead_of_recording_zero` at `ci_workflow_contracts.rs:1938` with `the lint duration must be fractional — an integer clock reports a fast command as \`0\`, which the report renders as \`not recorded\`; wrote "0"` — the old body really does write `duration_s=0` for a sub-second command on this host's bash
        - non-vacuity (renderers): with pre-fix semantics emulated, `a_sub_second_lint_command_renders_its_measurement_rather_than_not_recorded` failed at `ci-rollup-tests.rs:3766`, `a_zero_lint_duration_is_a_measurement_not_an_absence` at `:3787`, and `a_sub_second_environment_duration_is_reported_as_a_measurement` at `:3868`
        - non-vacuity (type): with `ProducerStatus.duration_s` reverted to `Option<u64>`, `a_lint_producer_status_carries_presence_independently_of_the_number` failed at `:4830` with `Error("invalid type: floating point \`0.42\`, expected u64", line: 1, column: 75)`
        - all experiments used temp-dir copies restored and hash-verified with `shasum -c`; no `git stash` was used and the user's git state was not modified
        - changed `scripts/ci-rollup.rs`: `Cell.duration_s` `u64` -> `Option<f64>` with `#[serde(default, skip_serializing_if = "Option::is_none")]`; `ProducerStatus.duration_s` `Option<u64>` -> `Option<f64>`; `ReusedResult.duration_s` `u64` -> `Option<f64>` so a v1 receipt stays unmeasured rather than `unwrap_or(0.0)`; `classify_one` accumulates `Option<f64>`, `status_cells` passes the status value through unchanged, `blank_cell` is `None`
        - changed `scripts/ci-rollup.rs`: new `format_duration(f64)` renders whole seconds at or above 1s (preserving the existing spelling) and two decimals below 1s, so a measurement never prints `0s`; `render_lint_report` now matches on presence; `render_environment_report` sums only present values and drops the `> 0 || origin != Unproduced` heuristic
        - changed `scripts/ci-rollup.rs` comments in the same pass: the `ProducerStatus.duration_s` doc no longer claims durations are "never `0`" (the review correctly identified that clause as false), and `render_environment_report`'s unavailable text became `not recorded (no producer recorded a test duration for this environment)` because the old "no producer reported a result" wording became false once a v1-receipt cell with no duration reaches that branch
        - changed `.github/workflows/_package-ci.yml`: the Lint step times the command with `python3 -c 'import time; print(time.monotonic())'` at both ends, rounded to 3 decimals to match `companion_suites.py` and `suite_runner.py`; neither clock reading can fail the gate (`|| true`), so an unreadable clock yields an empty `duration_s` that the status step already omits and the report renders as `not recorded` — verified by hand with `python3` off `PATH` (step exits 0, writes `duration_s=`)
        - changed `.github/workflows/_package-ci.yml` comments: the status step no longer says "An unmeasured duration is ABSENT, never `0`"; it now states that absence is absence and a recorded value is emitted with its fraction
        - changed `scripts/ci-rollup-tests.rs`: migrated call sites to `Option<f64>`; `an_unmeasured_lint_command_…` now sets `None` (the genuinely-absent case) and gained a doc comment; added `a_sub_second_lint_command_renders_its_measurement_rather_than_not_recorded`, `a_zero_lint_duration_is_a_measurement_not_an_absence`, `a_lint_producer_status_carries_presence_independently_of_the_number` (parsed from real producer JSON, covering both the measured and the absent artifact), and `a_sub_second_environment_duration_is_reported_as_a_measurement`
        - changed `tools/test-toolkit/tests/ci_workflow_contracts.rs`: added `the_lint_step_measures_a_sub_second_command_instead_of_recording_zero` (`#[cfg(unix)]`), which executes the real Lint `run:` body under the host's bash with a stub `just` on `PATH` and asserts the published `duration_s` is fractional, greater than zero, and under 60; `the_producer_status_carries_one_record_per_companion_suite` now injects a lint `duration_s` of `0.42` instead of `37`, proving the jq fold carries a fraction through unrounded
        - backward-compatibility decision: an absent `duration_s` in a `ci-results-<area>` slice must still deserialize and now means *unmeasured* (`#[serde(default)]`), while `skip_serializing_if` stops the rollup emitting a fabricated `0` for a cell nothing produced; producer and consumer of a slice are the same binary within one run, so no cross-version slice is read, and a legacy slice carrying an explicit `0` now reads as "measured 0" — the safer of the two errors, because it never invents `not recorded` for a real result
        - backward-compatibility decision: `scripts/ci/schema.py` and `scripts/ci/local_evidence.py` were deliberately left untouched — their `duration_s` belongs to the local receipt cell (`RECEIPT_CELL_FIELDS`, required, already `(int, float)`) and to `manifest.jsonl`, which are different documents from the rollup `Cell`; receipts always record a duration, so making that field optional would weaken a contract this finding does not implicate. `python3 scripts/ci/schema.py` regenerated `.github/ci/schemas/contract.json` byte-identically (sha256 unchanged, and the file is absent from `git status`)
        - backward-compatibility decision: `ManifestRecord.duration_s` stays `u64`. It is written by `just/devops.just`'s integer `SECONDS` and read by `tools/test-audit` (TypeScript) and `junit_staging_contracts.rs`; changing it is a separate blast radius. Stated consequence: an L1 tier that stages `0` now renders `0.00s` — a measurement, because it did run — rather than being conflated with an absence. Making *test* durations fractional is a follow-up that was not taken
        - portability justification for the clock source: `EPOCHREALTIME` needs Bash 5 and macOS ships 3.2; `date +%N` is a GNU extension BSD `date` lacks. `python3` is already a hard dependency of these workflows (`scripts/ci/*.py` run in the same job), and `time.monotonic()` reads a system-wide clock on Linux (`CLOCK_MONOTONIC`), macOS (`mach_absolute_time`), and Windows (`GetTickCount64`), so two readings from separate processes are sound — noted in the workflow comment. The new contract test executes the step body under the dev Mac's own bash, so the macOS traps are asserted rather than assumed
        - gates (re-run and confirmed independently by the orchestrator, not only reported by the subagent): `just _test repo-deps` 261 passed, 0 skipped (was 257, +4 new); `just _test test-toolkit` 182 passed, 2 skipped (+1 new); `just _lint repo-deps` clean; `just _lint test-toolkit` clean; all ten `scripts/ci/test_*.py` suites 562 tests, all OK; `actionlint` clean on both `ci.yml` and `_package-ci.yml`
        - deliberately out of scope: companion `measurements` rendering (`CompanionResult::new`'s `duration.round() as u64`) is already presence-correct via `Option` + `reason`; `just/devops.just`'s integer `SECONDS` staging for test tiers; the reuse `measurements` string's whole-second receipt formatting, which stays unambiguous because the v1 case renders `not recorded (v1 receipt)`
- work completed for 'Sub-second lint measurements are reported as unavailable' at 20:04:32
- starting the work on 'Required hosted workflow behavior remains unverified' at 20:05:10
        - the finding asks for the specification's hosted Validation 5 and 7 probes: a controlled GitHub Actions run proving that package-owned tooling failures reach `area-ci` and `ci-gate`, that a `ci-reporting` failure stays advisory, that zero matrices resolve with the intended labels, that area artifacts aggregate, and that a documentation-only follow-up preserves and reports reused cells
        - the review is explicit that this is the correct and only boundary: "Terminal L2/L3 would not help; the required integration boundary is a controlled hosted Actions run", and that the L1 planner, workflow-source, and shell-fragment tests "manufacture inputs or parse YAML text" and cannot establish GitHub's matrix expansion, reusable-workflow result propagation, cross-job artifact availability, Checks-tab labels, or `continue-on-error` conclusions
        - **deferred.** A hosted run requires a commit and a push to the remote. This repo's `CLAUDE.md` forbids committing unless the prompt explicitly says so, and this prompt does not; the session is additionally non-interactive, so the outward-facing push could not be confirmed with the user. `fixes/_complete/2026-09-13-cicd-redundancies/acceptance.md` records that the immediately preceding phase was blocked for exactly the same reason and already marks Validation 5 and 7 `HOSTED-PENDING`
        - this is not a performance measurement that the host's CPU load prevented, so `deferred_perf_measurement` stays unset and no deferred-performance record was created
        - what was done instead, within the authorized boundary: findings 1, 2, and 4 moved the two `ci-reporting` defects the hosted run would otherwise have surfaced from "unverified YAML text" to executable local evidence — a `bash` fixture over the workflow's own `run:` bodies and a guard evaluator that reads the `if:` expressions out of `ci.yml` rather than a copy of them
        - changed `fixes/_complete/2026-09-13-cicd-redundancies/acceptance.md`: added a "Added by review iteration 2" subsection under `## Outstanding` recording that the machine-readable hosted record the review asks for still does not exist, and naming the three newly-introduced `ci-reporting` behaviors the hosted run must now also confirm (neutral reuse prose, exactly one mode on a successful scoped run, a cancelled bootstrap named as cancelled)
        - no source, test, or workflow change was made for this finding, so no gates were run for it
- work completed for 'Required hosted workflow behavior remains unverified' at 20:07:40 (deferred)
- cross-OS verification of the two impacted packages, using the `just cross-check` rigs rather than direct SSH:
        - macOS (this host): `repo-deps` 261 passed / 0 skipped, `test-toolkit` 182 passed / 2 skipped, both lints clean, all ten `scripts/ci/test_*.py` suites 562 tests OK, `actionlint` clean on `ci.yml` and `_package-ci.yml`
        - native Windows (`build-win-native`): `test-toolkit` 178 passed / 2 skipped and `repo-deps` 261 passed / 0 skipped, both exit 0. The 178-versus-184 test count is expected and correct — the new executable `bash` fixtures are `#[cfg(unix)]`, so Windows compiles the crate and runs every platform-neutral contract while skipping the shell fixtures by design
        - Linux (`build-linux`): **no evidence obtained.** `just cross-check repo-deps --os linux` never ran the suite; it waited 1800s for the rig lock and gave up. The lock is held by an unrelated run — `{"purpose": "nightly-reward-spike", "owner": "reward-20260914-c3e60d0", "branch": "feat-nightly-perf", "started": "2026-09-14T18:25:30Z"}` — which had been holding it for roughly nine hours by the time of the attempt. The recipe's own guidance is explicit ("if that run is dead its owner removes the lock by hand; never remove someone else's"), so the lock was left alone and no Linux result is claimed
        - residual Linux risk is assessed as low, and deliberately assessed rather than assumed: the Rust change is platform-neutral (`u64` -> `Option<f64>` with no `cfg` or path handling), and the one genuinely OS-sensitive change — the Lint step's `python3` monotonic clock — was executed for real under **macOS bash 3.2**, which is the harsher host. The `ubuntu-latest` runner the step actually targets has bash 5 and the same `python3` this repo already depends on

### Successful Completion

The implementation of review cycle 2 has completed successfully in 1 hour 13
minutes. During this implementation all 4 review findings were evaluated to see
if they could be fixed as a part of this implementation cycle: 3 were fixed, 1
was deferred (see reasons below):

- **High — Required hosted workflow behavior remains unverified.** Deferred
  because its only valid verification boundary is a controlled GitHub-hosted
  Actions run, which requires a commit and a push to the remote. This
  repository's `CLAUDE.md` forbids committing unless the prompt says so
  explicitly, and this prompt does not; the session is additionally
  non-interactive, so the outward-facing push could not be confirmed with the
  user. The review itself rules out every substitute — it states that terminal
  L2/L3 "would not help" and that the L1 planner, workflow-source, and
  shell-fragment tests "manufacture inputs or parse YAML text" and cannot
  establish GitHub's matrix expansion, reusable-workflow result propagation,
  cross-job artifact availability, Checks-tab labels, or `continue-on-error`
  conclusions. `acceptance.md` already recorded Validations 5 and 7 as
  `HOSTED-PENDING` for the identical reason in the preceding phase, and this
  cycle extended that record with the three newly-introduced `ci-reporting`
  behaviors the hosted run must now also confirm.

This deferral is not a deferred performance measurement — no metric was blocked
by host CPU load — so `deferred_perf_measurement` is not set on this log's
frontmatter and no deferred-performance record was created.

One verification gap that is **not** a deferred finding is recorded above for
completeness: the `build-linux` cross-check produced no result because the rig
lock was held by an unrelated nightly run, and the recipe forbids clearing
another owner's lock. macOS and native Windows evidence was obtained for both
impacted packages.

The files changed by this implementation cycle are:

- `.github/workflows/ci.yml` — the `ci-reporting` reuse step's neutral prose
  (finding 1) and mode 3's exclusive/exhaustive guard plus cancellation
  handling (finding 2)
- `.github/workflows/_package-ci.yml` — the Lint step's fractional monotonic
  clock and the corrected producer-status comment (finding 4)
- `scripts/ci-rollup.rs` — `Cell`, `ProducerStatus`, and `ReusedResult` carry
  duration as `Option<f64>`; new `format_duration`; presence-based rendering in
  `render_lint_report` and `render_environment_report` (finding 4)
- `scripts/ci-rollup-tests.rs` — four new regressions covering measured
  sub-second, recorded zero, absent duration, and the environment-presence
  heuristic (finding 4)
- `tools/test-toolkit/tests/ci_workflow_contracts.rs` — the `POLICY_VOCABULARY`
  contract (finding 1), the guard evaluator plus the three-mode exclusivity and
  cancellation fixtures (finding 2), and the executable Lint-duration fixture
  (finding 4)
- `fixes/_complete/2026-09-13-cicd-redundancies/acceptance.md` — the outstanding hosted
  work extended with what review iteration 2 added (finding 3)
- `fixes/_complete/2026-09-13-cicd-redundancies/log.md` — this record

## Implementation of Review Findings #3

> **started at:** 2026-09-15T03:20:04-07:00

- this implementation is attempting to implement _all_ of the review findings found in '/Volumes/coding/wt/rusty-biscuit/fix-cli-slow-tests/fixes/_complete/2026-09-13-cicd-redundancies/review-3.md'
- this is iteration 3 of the review-to-implement cycle
- orchestration plan decided after reading review 3, the specification, and the cited implementation surfaces:
        - finding 1 (high, lint timing compares separate-process clocks and fails the mandatory L1 suite) — implementable here; touches `.github/workflows/_package-ci.yml` and the executable fixture in `tools/test-toolkit/tests/ci_workflow_contracts.rs`
        - finding 2 (high, required hosted workflow behavior remains unverified) — expected to defer; review 3 itself files it under `## Blocked Findings` and states it is blocked on an explicitly authorized commit and push, which this session does not have
        - finding 3 (medium, environment durations silently omit unmeasured cells) — implementable here; touches `render_environment_report` in `scripts/ci-rollup.rs` and its regressions in `scripts/ci-rollup-tests.rs`
        - the two unblocked findings are worked serially by one subagent each; gates run only in the areas Validation 2 of the specification names and that each finding actually touches (`tools/test-toolkit` for finding 1, `repo-deps` for finding 3)
- starting the work on 'Lint timing compares separate-process clocks and fails the mandatory L1 suite' at 03:21:47
        - reproduction: the shipped regression `the_lint_step_measures_a_sub_second_command_instead_of_recording_zero` passed 25/25 consecutive `cargo nextest run -p test-toolkit` invocations on this macOS host, and 400 direct samples of the step's own two-process construct (`started=$(python3 -c 'print(time.monotonic())')` then a second `python3` subtracting it) never went below `+0.015`, so the review's `-0.001` is not a load- or contention-driven flake
        - root cause found by comparing interpreters rather than repeating runs: `/opt/homebrew/bin/python3` (CPython 3.14.7) reports `time.monotonic() = 215950.654443791` while `/usr/bin/python3` (CPython 3.9.6, the macOS system interpreter) reports `0.003695041` for the same instant — 3.9's reference point is PROCESS START, not boot, which is precisely the freedom Python documents ("the reference point of the returned value is undefined, so that only the difference between the results of two consecutive calls is valid")
        - with `/usr/bin/python3` first on `PATH`, 30 samples of the two-process construct produced `2 × -0.001`, `19 × -0.0`, `9 × 0.0` — the review's exact observed value, and the defect is deterministic per interpreter rather than random per run
        - confirmed end to end: `PATH="/usr/bin:/bin:/usr/sbin:/sbin:$PATH" cargo nextest run -p test-toolkit --test ci_workflow_contracts -E 'test(the_lint_step_measures...)'` failed at `tools/test-toolkit/tests/ci_workflow_contracts.rs:1946` with `a command that ran took a measurable amount of time; wrote "-0.002"`, matching review 3's report of the same assertion at the same line
        - changed `.github/workflows/_package-ci.yml`: the `Lint` step (`id: clippy`) now takes both endpoints inside ONE `python3` process that owns the `just _lint "<package>"` child via `subprocess.run(..., check=False)`, writes `duration_s=round(elapsed, 3)` to `$GITHUB_OUTPUT`, and `sys.exit(completed.returncode)` — the 3-decimal rounding matches `scripts/ci/companion_suites.py:128` and `scripts/ci/suite_runner.py:86`, which already time single-process this way
        - the interpreter-absent branch is kept explicit rather than implicit: `if ! command -v python3`, run `just _lint`, publish an EMPTY `duration_s`, propagate the child's status — an unavailable MEASUREMENT, never a gate verdict and never a fabricated `0`
        - the heredoc is `<<'PY'` with its body and its terminator written at the run-block's base column (10 spaces), so YAML's block-scalar dedent lands Python's top-level statements at column 0; verified by extracting the body exactly as `step_script` does (strip 10 columns) and EXECUTING it under `/bin/bash` 3.2.57 with `env -i`, not by reading it — four scenarios: success + python3 (`exit=0`, `duration_s=0.005`), `just` exiting 7 + python3 (`exit=7`, `duration_s=0.01`), success with no python3 on `PATH` (`exit=0`, `duration_s=`), and exit 7 with no python3 (`exit=7`, `duration_s=`)
        - rewrote the comment block above the step: deleted the falsified sentence "The two readings come from separate processes, which is sound because `time.monotonic` reads a system-wide clock on every platform this repository targets" and replaced it with the WHY for the single-process boundary plus the `/usr/bin/python3` counterexample that disproved it; the `SECONDS`/AC13 and never-fail-the-gate paragraphs were kept because they still hold
        - changed `tools/test-toolkit/tests/ci_workflow_contracts.rs`: extracted `host_bash()`, `run_lint_step(script, just_exit, python3_visible) -> LintStepRun`, and `lint_step_script()` so the step body can be executed under several stub and `PATH` policies from one place; `host_bash()` resolves `bash` absolutely because one case hands the script a `PATH` holding only the fixture's stubs
        - `the_lint_step_measures_a_sub_second_command_instead_of_recording_zero` now runs the success stub 8 times and a `exit 7` stub once; it asserts `exit_code == Some(just_exit)` (not merely `status.success()`), then parses `duration_s` and requires it to be FINITE, `> 0.0`, fractional, and `< 60.0` — the finite and non-negative checks are new, and the ordering puts `is_finite` ahead of `> 0.0` so a `NaN` fails with its own message instead of the negative-duration one
        - added `the_lint_step_publishes_an_empty_duration_when_it_cannot_time_the_command`, which runs the same body with `PATH` reduced to the fixture's stub directory (no `python3` anywhere) for both a passing and a failing `just`, asserting the gate status still propagates and `duration_s` is EMPTY rather than `0`; this pins the unavailable-measurement property the `|| true` chain used to provide incidentally
        - both tests stay `#[cfg(unix)]` and hermetic: per-case `tempfile::tempdir()`, `env_clear()`, an explicit `PATH`, and a per-case `GITHUB_OUTPUT` file
        - non-vacuity: `git show HEAD:.github/workflows/_package-ci.yml` is NOT the right baseline here — iteration 2's fractional-clock change is still uncommitted, so `HEAD` carries the older integer `$SECONDS` body. The pre-fix body for THIS finding (the two-process `python3` pair) was spliced into the workflow instead; under `/usr/bin/python3` the strengthened test FAILED at `ci_workflow_contracts.rs:2006` with ``a command that ran took a measurable, non-negative amount of time; two `time.monotonic` readings taken in separate interpreters subtract unrelated origins and can go backwards; `just` stub exiting 0, python3 visible; wrote "duration_s=0.0\n"``, and under Homebrew python3 it PASSED — which is exactly why the defect looked intermittent
        - after restoring the fixed body (`diff -q` against a pre-splice copy: identical), both lint-step tests pass under BOTH interpreter orderings, so the fix removes the interpreter dependence rather than hiding it
        - gates, run with `CARGO_TARGET_DIR=/tmp/cwc-target` because the shared `target/` is fed by a kache store on another volume: `just _test test-toolkit` → `183 tests run: 183 passed, 2 skipped`; `just _lint test-toolkit` → exit 0 with no clippy output; `actionlint .github/workflows/_package-ci.yml` → clean. `scripts/ci-rollup.rs` and `scripts/ci/*.py` were not touched, so `repo-deps` was not run
        - GitNexus: `impact "the_lint_step_measures_a_sub_second_command_instead_of_recording_zero" --direction upstream` returned `Target ... not found` with `risk: UNKNOWN` (the index predates the iteration-2 test); resolved by text search — the only references repo-wide are prose in `review-3.md` and this log, and a `#[test]` in an integration-test binary has no callers by construction. `detect-changes --scope all` reported `13 files, 35 symbols`, `Affected processes: 0`, `Risk level: low`, with no `partial`/`truncated` flag
        - not done: formatting. `rustfmt --check` (read-only) reports three cosmetic splits inside the new code, each the same style the surrounding file already uses and already reported at ~60 other pre-existing sites in the same file; `cargo fmt` was not run, per the session's standing instruction
- work completed for 'Lint timing compares separate-process clocks and fails the mandatory L1 suite' at 03:30:43
- starting the work on 'Environment durations silently omit unmeasured cells' at 03:32:23
        - read the surrounding surface first: `render_environment_report` (`scripts/ci-rollup.rs:4228`), `companion_tally` (`:4288`), `render_lint_report` (`:4316`), `format_duration` (`:4215`), `cell_text` (`:3123`), and the `UNRECORDED` / `NO_PACKAGE_TESTS` constants (`:80`, `:53`)
        - the defect confirmed as written: the row computed `measured_cells: Vec<f64>` by `filter_map(|cell| cell.duration_s)` and then printed `format_duration(sum)` whenever `!measured_cells.is_empty()`, so an environment holding a measured `L1` cell and an unmeasured `L2` cell printed the `L1` figure with no indication that a second cell contributed nothing
        - rendering chosen: the PARTIAL SUM, explicitly labeled and naming the unmeasured cells — `61s (partial; not recorded: claudine/L2)`. Rendering the whole row as `not recorded` satisfies the letter of Required Behavior 6 but discards a measurement the reader came for; the actual defect is the UNLABELED sum, not the sum. This is also `companion_tally`'s own shape (`{total} (+{UNRECORDED}: {names})`) — report what was measured, name what is missing from it — so the two aggregates in the same table now read the same way
        - cells are named `{package}/{tier}` rather than by the full `CellKey` Display (`{package}/{environment}/{tier}`): the environment is already the row's first column, and repeating it in every name would be noise
        - extracted the decision into `environment_duration(cells: &[&Cell]) -> String` (`scripts/ci-rollup.rs:4273`) rather than growing the row loop, because the three-way choice (complete / partial / absent) does not fit a `format!` argument and `render_lint_report`'s sibling `match` already reads that way. Three branches, in order: no unmeasured cell → `format_duration(sum)`, byte-identical to today; every cell unmeasured → the existing `not recorded (no producer recorded a test duration for this environment)`; otherwise the labeled partial sum
        - bounded the name list at `SHOWN = 5` with a `(+N more)` tail, copying `why`'s treatment of failing-test identities (`scripts/ci-rollup.rs:3361`). An environment row aggregates across every package in the run, so a whole-workspace fold could otherwise put dozens of names into one table cell. `companion_tally` is unbounded, but its list is per-cell declared companion suites (one or two), not a whole run's cells
        - the row value is now passed through `cell_text` like the other variable-content columns: the name list interpolates `Tier::Other(String)`, which is verbatim producer input and could carry a `|` or a newline that would break the GFM table. The previous code needed no escape because the column held only `61s` or a fixed sentence
        - comments: deleted the "Presence, never the number" comment at the old `measured_cells` line — it justified summing the present values, which is still true, but it was attached to the very expression that then reported the partial sum as complete. Its load-bearing half (`Some(0.0)` is a measurement, `None` is not) is restated at the new `filter_map`/`filter` pair, where the split between the sum and the missing list is the surprising line. `render_environment_report`'s `///` still describes the row and needed no change
        - checked the sibling renderers for the same class of defect, per Rule 3. `render_lint_report` renders ONE cell per row and has no aggregate — `match cell.duration_s` is already presence-correct, so it is NOT affected and was not touched. `companion_tally` already names its uncounted suites. The two remaining `.sum()` calls in the rendering region (`ci-rollup.rs:4251` and `:4404`) fold `cell.counts.total`, a non-optional `Counts`, and the shipped `an_environment_no_producer_reported_renders_not_recorded_rather_than_zero` deliberately pins `0` there for an unproduced cell — a different question from this finding, left alone
        - added four regressions in `scripts/ci-rollup-tests.rs` behind one shared `environment_cells` / `environment_report` fixture pair (`:3879`), each entry a distinct `{package, tier}` so every spec row survives into its own cell: `a_mixed_measured_and_unmeasured_environment_reports_a_partial_duration` (the regression review 3 asked for), `an_environment_whose_cells_all_measured_reports_one_complete_duration` (guards the normal path — asserts `| 68s |` and that neither `partial` nor `not recorded` appears), `an_environment_whose_cells_are_all_unmeasured_stays_not_recorded` (the absence keeps its reason with more than one cell, which the single-cell `an_environment_no_producer_reported_…` did not cover), and `a_partial_environment_duration_bounds_the_cells_it_names` (5 names + `(+1 more)` from 6 unmeasured cells)
        - non-vacuity, proved by splicing the pre-fix decision back in (`unmeasured.is_empty()` → `unmeasured.len() < cells.len()`, which is exactly the old "at least one measured ⇒ print the sum") and re-running: `a_mixed_measured_and_unmeasured_environment_reports_a_partial_duration` FAILED at `scripts/ci-rollup-tests.rs:3910` with `a partly measured environment must label its sum and name the unmeasured cell:` followed by the rendered table containing `| ubuntu-latest | 2 | 4 | none declared | 61s | ci |` — the two-cell environment reporting the one measured cell's 61s as its own, with the `claudine/L2` absence nowhere in the document. `a_partial_environment_duration_bounds_the_cells_it_names` failed alongside it
        - the same pre-fix run PASSED `an_environment_whose_cells_all_measured_reports_one_complete_duration`, `an_environment_whose_cells_are_all_unmeasured_stays_not_recorded`, `a_sub_second_environment_duration_is_reported_as_a_measurement`, and `an_environment_no_producer_reported_renders_not_recorded_rather_than_zero` — confirming the two boundary tests are guards on untouched paths rather than part of the regression, and that the fix changes only the mixed case
        - working tree restored from the pre-splice copy; `git diff --stat` back to 13 files and `grep -n "unmeasured.is_empty()"` back at `scripts/ci-rollup.rs:4293`
        - gates, in the one impacted area: `just _test repo-deps` → `265 tests run: 265 passed, 0 skipped` (261 before this change, +4 new); `just _lint repo-deps` → exit 0 with no clippy output. `tools/test-toolkit` was not re-run — nothing it contracts over was touched
        - GitNexus: `impact "render_environment_report" --direction upstream` returned `risk: LOW`, `epistemic: exact`, 3 upstream hits (`render_report` → `cmd_summarize` → `main`) and 0 affected processes; confirmed by text search that `scripts/`, `tools/`, `.github/`, and `just/` contain exactly two references, the definition and that one call. `environment_duration` is new and has the same single caller. `detect-changes --scope all` reported `13 files, 42 symbols`, `Affected processes: 0`, `Risk level: low`, with no `partial`/`truncated` flag
        - not done: formatting. `rustfmt --check` (read-only) reports 103 cosmetic diffs across `ci-rollup.rs` + `ci-rollup-tests.rs`, of which 2 fall inside the new code — a `format!` argument split and an `environment_report(&[…])` call-site join, both the same style the file already carries at ~100 pre-existing sites. `cargo fmt` was not run, per the session's standing instruction
- work completed for 'Environment durations silently omit unmeasured cells' at 03:39:10
- starting the work on 'Required hosted workflow behavior remains unverified' at 03:39:16
        - review 3 files this finding under its own `## Blocked Findings` heading and names the block itself: the finding "remains blocked on an explicitly authorized commit and push", and its previous-review disposition records it as "open and blocked" carried over from review 2
        - this implementation cycle received no authorization to commit or push, and the session is non-interactive so none can be requested; the only boundary that can answer the finding is a real GitHub Actions run, which requires exactly that push
        - the review rules out every local substitute in its own words: "Local planner, YAML, expression, Bash, and rollup tests cannot establish GitHub's matrix expansion, nested reusable-workflow result propagation, artifact handoff, Checks-tab labels, or `continue-on-error` conclusions"
        - deferred; no code change made for this finding. What was done instead is to keep the acceptance record honest: `fixes/_complete/2026-09-13-cicd-redundancies/acceptance.md` gained an `### Added by review iteration 3 (2026-09-15)` section restating the block and extending the hosted checklist with the two behaviors this iteration newly introduced
        - the two added hosted checks are: the `Lint` step's single-process `python3` boundary publishing a finite, non-negative, fractional `duration_s` on each hosted image's own interpreter (the defect fixed above was interpreter-dependent, so the hosted images pin a variable this host cannot), and an environment with an unmeasured cell rendering a labeled partial sum rather than one cell's measurement as the total
        - this is NOT a deferred performance measurement — no metric was blocked by host CPU load, so `deferred_perf_measurement` is not set on this log's frontmatter and no deferred-performance record was created
- work completed for 'Required hosted workflow behavior remains unverified' at 03:39:16
- orchestrator verification over the combined tree, after both subagents finished, so neither finding's fix is proved only in isolation:
        - `just _test test-toolkit` → `183 tests run: 183 passed, 2 skipped` (181 before this cycle, +2 new)
        - `just _test repo-deps` → `265 tests run: 265 passed, 0 skipped` (261 before this cycle, +4 new)
        - `actionlint .github/workflows/ci.yml .github/workflows/_package-ci.yml` → clean
        - the review's reported failure is gone: the suite it recorded as `181 passed, 1 failed, 2 skipped` with `duration_s=-0.001` now passes in full
- cross-OS consideration: neither change is OS-conditional in the way the `os` skill guards against. The Lint step is `shell: bash` on all three images and its fixture is `#[cfg(unix)]` because it asserts Bash semantics; the residual OS risk is which `python3` each hosted image resolves, which is recorded as a hosted check above rather than claimed as proved. `render_environment_report` is pure string rendering over in-memory cells with no path, clock, or process dependence. No `just cross-check` run was made, for that reason and because both changed packages' gates are environment-independent

### Successful Completion

The implementation of review cycle 3 has completed successfully in 22 minutes.
During this implementation all 3 review findings were evaluated to see if they
could be fixed as a part of this implementation cycle: 2 were fixed, 1 was
deferred (see reasons below):

- **High — Required hosted workflow behavior remains unverified.** Deferred
  because its only valid verification boundary is a controlled GitHub Actions
  run on this fix's branch, which requires a commit and a push that this cycle
  was not authorized to perform. Review 3 reaches the same conclusion itself:
  it files the finding under `## Blocked Findings` and states that it "remains
  blocked on an explicitly authorized commit and push". The review also rules
  out every local substitute, stating that local planner, YAML, expression,
  Bash, and rollup tests "cannot establish GitHub's matrix expansion, nested
  reusable-workflow result propagation, artifact handoff, Checks-tab labels, or
  `continue-on-error` conclusions". `acceptance.md` already carried Validations
  5 and 7 as `HOSTED-PENDING` for the identical reason across the two preceding
  cycles; this cycle extended that record with the two newly-introduced
  behaviors the hosted run must now also confirm.

This deferral is not a deferred performance measurement — no metric was blocked
by host CPU load — so `deferred_perf_measurement` is not set on this log's
frontmatter and no deferred-performance record was created.

The files changed by this implementation cycle are:

- `.github/workflows/_package-ci.yml` — the `Lint` step now times `just _lint`
  inside the single `python3` process that owns the child and propagates its
  exit status, replacing the two independent interpreters whose
  `time.monotonic()` origins are unrelated; the comment block above it lost the
  falsified claim that those readings were comparable and gained the real WHY,
  including the `/usr/bin/python3` counterexample that produced the review's
  `-0.001` (finding 1)
- `tools/test-toolkit/tests/ci_workflow_contracts.rs` — the kept regression now
  requires a finite, non-negative, fractional measurement, samples the success
  case repeatedly, and exercises a failing `just` child to prove timing never
  masks the gate status; a new companion test covers an absent `python3`
  publishing an empty `duration_s` while still propagating the gate (finding 1)
- `scripts/ci-rollup.rs` — new `environment_duration` renders an environment's
  duration three ways: the unchanged complete sum, the existing `not recorded`
  with its reason, and a labeled partial sum naming the unmeasured
  `{package}/{tier}` cells; the stale "Presence, never the number" comment was
  replaced at the line that is actually surprising (finding 3)
- `scripts/ci-rollup-tests.rs` — four regressions over one shared fixture pair
  covering mixed, fully measured, fully unmeasured, and bounded-name-list
  environments (finding 3)
- `fixes/_complete/2026-09-13-cicd-redundancies/acceptance.md` — the outstanding hosted
  work extended with what review iteration 3 added (finding 2)
- `fixes/_complete/2026-09-13-cicd-redundancies/log.md` — this record

## Implementation of Review Findings #4

> **started at:** 2026-09-15T03:48:52-07:00

- this implementation is attempting to implement _all_ of the review findings found in '/Volumes/coding/wt/rusty-biscuit/fix-cli-slow-tests/fixes/_complete/2026-09-13-cicd-redundancies/review-4.md'
- this is iteration 4 of the review-to-implement cycle
- orchestration plan decided after reading review 4, the specification, and the two renderer sites the review cites:
        - finding 1 (medium, unavailable Rust test counts are reported as numeric zero) — implementable here; touches `scripts/ci-rollup.rs` (`render_environment_report`, `render_combined_summary`) and `scripts/ci-rollup-tests.rs`, both owned by the `repo-deps` package
        - finding 2 (high, required hosted workflow behavior remains unverified) — filed by the review itself under `## Blocked Findings`; expected to defer because its only valid boundary is a hosted GitHub Actions run requiring a commit and push this session is not authorized to perform
        - findings are worked serially by one subagent each; gates run only in the package areas the specification's Validation 2 names and that this change actually touches — `repo-deps` (the `scripts` Nextest bins) and, for the contract vocabulary, `test-toolkit`
- reconnaissance completed by the orchestrator before dispatch, recorded here because it constrains the design handed to the subagent:
        - the defect is a **modeling** gap, not a rendering slip: `Cell.duration_s` is `Option<f64>` and is documented in-struct as "the absence of a measurement ... is the ONLY thing that renders as `not recorded`", while its sibling `Cell.counts` is a bare `Counts` whose `Default` is indistinguishable from a real zero
        - three producers hand a cell `Counts::default()` with no measurement behind it: a cell no producer reported (`indices.is_empty()`), a record whose JUnit could not be parsed (`has_unusable_record`), and a reused version-1 receipt (`evidence.and_then(|record| record.counts).unwrap_or_default()` at `scripts/ci-rollup.rs:1473`)
        - availability is **not** recoverable from an already-serialized `Cell`: a v1-reused cell carries a real `origin` and differs only by the `not recorded (v1 receipt)` string in its evidence link, so deriving it at render time would mean string-matching a human-readable label
        - `classify_state_from_evidence` already distinguishes a genuine zero-test run from an absent one via `counts.total == 0` against `indices.is_empty() || has_unusable_record`, so the review's "a genuine executed suite that selected zero tests must remain distinguishable" requirement has an existing seam to preserve rather than invent
- starting the work on 'Unavailable Rust test counts are reported as numeric zero' at 03:52:41
        - **incident — uncommitted test work was destroyed by the subagent and reconstructed.** Between non-vacuity probes the subagent ran `git checkout -- scripts/ci-rollup-tests.rs` to restore the file, which discarded that file's uncommitted review-2 and review-3 changes (the `duration_s` `u64 → Option<f64>` migration across its call sites, plus 8 regressions). `scripts/ci-rollup.rs` was never checked out, so the implementation half of both prior cycles survived intact. The subagent reconstructed the file from the still-warm compiled test binary (`--list` for the complete pre-loss test roster, `strings` for assertion literals), from verbatim regions already in its context, and from this log's own account of both cycles
        - orchestrator verification of the reconstruction, performed independently of the subagent's own claims:
                - no genuine recovery path existed: the file was never staged, so `git fsck --lost-found` holds no blob of it (the dangling blobs present are GitNexus index objects)
                - every test this log independently recorded as added to that file is present exactly once: review 2's `a_sub_second_lint_command_renders_its_measurement_rather_than_not_recorded`, `a_zero_lint_duration_is_a_measurement_not_an_absence`, `a_lint_producer_status_carries_presence_independently_of_the_number`, `a_sub_second_environment_duration_is_reported_as_a_measurement`; review 3's `a_mixed_measured_and_unmeasured_environment_reports_a_partial_duration`, `an_environment_whose_cells_all_measured_reports_one_complete_duration`, `an_environment_whose_cells_are_all_unmeasured_stays_not_recorded`, `a_partial_environment_duration_bounds_the_cells_it_names`
                - review 3's shared `environment_cells` / `environment_report` fixture pair is present, and the two assertion details this log had pinned by value survive verbatim: the complete-duration row `| ubuntu-latest | 2 | 4 | none declared | 68s | ci |` and the bounded list `5s (partial; not recorded: a/L1, b/L1, c/L1, d/L1, e/L1 (+1 more))`
                - the strongest corroboration is structural: `scripts/ci-rollup.rs` still carries the review-2/review-3 `Option<f64>` migration untouched, and the reconstructed test file compiles and passes against it. A reconstruction that had drifted from the prior cycles' contracts could not type-check against an implementation it did not touch
                - three files carry reconstructed rather than byte-faithful bodies — `a_sub_second_lint_command_renders_its_measurement_rather_than_not_recorded`, `a_zero_lint_duration_is_a_measurement_not_an_absence`, `a_lint_producer_status_carries_presence_independently_of_the_number` — with their original names, recovered assertion messages, and expected row strings but newly written bodies. **A reviewer should read those three as new code**, not as review-2 work that was merely restored
        - discovered, contradicting the orchestrator's own reconnaissance: availability is **not** decidable from `outcome` on the reused path. `local_evidence.py::_accept_version_one` writes `outcome: "pass"` with no `counts`, while `RECEIPT_CELL_FIELDS` makes `counts` required of every later receipt. So `counts: None` **is** the version-1 shape, and the pre-existing `measured = evidence.is_some_and(|record| record.outcome.is_some())` predicate at `ci-rollup.rs:1473` was wrong: given a plan that omitted `measurements`, a counts-less acceptance rendered `0 test(s), 0 failed, 0s` — the same fabricated zero the finding is about, at a site the review did not name. Re-keyed onto counts presence
        - discovered: `render_combined_summary` groups **all** cells by area, `lint` and `check` included, and those tiers have no cardinality and never will. Naming them as unmeasured would bury the test cell that genuinely failed to record one, so `area_tests` filters to `is_test_tier` first — behavior-preserving for the number, since a compile gate contributed `0` to the old sum
        - design decision: availability is modeled as `Cell.counts: Option<Counts>` rather than the parallel `bool` the orchestrator recommended. The subagent's rationale is accepted: the file already spells "counts may be absent" this way twice (`CompanionResult.counts`, `PlanEvidence.counts`), and a `bool` beside a populated `Counts` is a desyncable pair — the file's own fixture idiom `Cell { counts: X, ..blank_cell(k) }` would have silently desynced it on every existing fixture. Only four read sites existed, so the feared `.unwrap_or_default()` relocation did not materialize. `#[serde(default, skip_serializing_if = "Option::is_none")]` mirrors `duration_s`, so a legacy area slice omitting the field deserializes as unmeasured — the safe direction
        - because the `Option` model was chosen, `grid_text` and `why` were handled coherently rather than defaulted, as the orchestrator's brief required of that option: an unmeasured cell now renders `MISSING not recorded` instead of `0/0/0`. This closes the same fabricated zero in the grid for a version-1 reused cell, which the review did not name
        - availability is set truthfully at every construction site: `blank_cell` → `None`; `classify_one` → `!indices.is_empty() && !has_unusable_record`, reusing the exact seam `classify_state_from_evidence` uses to separate MISSING from NOTHING TO RUN; the reused path → `result.counts.is_some()`. A partly unreadable cell counts as unmeasured, matching its MISSING state
        - extracted `partial_measurement(measured, unmeasured, absent)` as the single owner of the complete/partial/`not recorded` spelling and of the bounded list (`SHOWN = 5`, `+N more`). `environment_duration` was rewritten onto it with its rendered strings unchanged — every pre-existing duration assertion still passes untouched — and `environment_tests` and `area_tests` are new callers, each with its own accurate reason string. Sums use `reduce` rather than `sum`, so an empty sum yields `None` instead of the `0` that started this finding
        - non-vacuity, probe 1 (the required one): the wholly-unmeasured environment and per-area regressions were run against the UNMODIFIED renderer and both FAILED. `an_environment_no_producer_reported_renders_not_recorded_rather_than_zero` panicked printing a report whose environment row read `| windows-latest | 1 | 0 | none declared | not recorded (no producer recorded a test duration for this environment) | none |` — the count column a bare `0` beside a duration that correctly said `not recorded`, which is the finding exactly — and whose area row read `| claudine | 1 | 0 | 0 | 1 | 0 | 0 | 0 |`. Counts line: `124/266 tests run: 122 passed, 2 failed, 0 skipped`
        - non-vacuity, probe 2 (renderer mutation): splicing `environment_tests` and `area_tests` back to the pre-fix unconditional sum gave `272 tests run: 267 passed, 5 failed, 0 skipped`, failing exactly the five count regressions and leaving the three guard tests green, as they must
        - non-vacuity, probe 3 (model mutation) **caught a real coverage gap and is why the fix is trustworthy**: forcing `counts_measured = true` in `classify_one` left all 272 tests green, because every count regression to that point exercised the renderer or the reused path and nothing guarded the `indices.is_empty() || has_unusable_record` seam. `a_cell_with_no_readable_report_measures_no_counts` was added and the same mutation then gave `273 tests run: 272 passed, 1 failed`, failing exactly that test
        - orchestrator-run gates, run fresh over the final tree rather than taken from the subagent's report: `just _test repo-deps` → `Summary [1.134s] 273 tests run: 273 passed, 0 skipped` (the review's 265 baseline + 8 added); `just _lint repo-deps` → clean, exit 0; `just _test test-toolkit` → `Summary [1.096s] 183 tests run: 183 passed, 2 skipped`, unchanged from its baseline and run because `ci_workflow_contracts.rs` asserts literals that live inside `scripts/ci-rollup-tests.rs`
        - `actionlint` was not re-run: no file under `.github/workflows/` was modified by this cycle
        - cross-OS assessment: no variance expected and none found. Everything changed is pure string formatting and arithmetic over in-memory cells — no path handling, no clock, no process, no filesystem, no `#[cfg]`. The only serialization movement is that `counts` is now omitted when absent, and producer and consumer are the same binary within one run. `just cross-check` was therefore not run
- work completed for 'Unavailable Rust test counts are reported as numeric zero' at 04:09:30
- starting the work on 'Required hosted workflow behavior remains unverified' at 04:10:05
        - no subagent was dispatched for this finding: review 4 files it under its own `## Blocked Findings` heading and names the unblocking condition itself — "after commit and push are explicitly authorized" — so there is no implementation to delegate, only the acceptance record to extend. The orchestrator did that directly rather than spending a round-trip on a documentation append it had full context for
        - confirmed the block is unchanged from cycles 2 and 3 rather than assumed: this cycle was given no commit or push authorization, and the repo's standing instruction is that commits happen only when explicitly prompted. The review's own `## Verification performed` records that it too performed no push, workflow dispatch, ruleset mutation, or commit
        - confirmed the review is not asking for something a local substitute could satisfy: it states that local planner, workflow-source, shell, and renderer fixtures "cannot establish GitHub's matrix expansion, nested reusable-workflow result propagation, artifact handoff, rendered check names, or `continue-on-error` conclusions", and closes by calling it "an integration-boundary gap, not a request for additional cross-OS proof or human design review"
        - extended `fixes/_complete/2026-09-13-cicd-redundancies/acceptance.md` with an `### Added by review iteration 4 (2026-09-15)` section, following the shape cycles 2 and 3 established: it restates why the block persists and names the two newly-introduced behaviors the hosted run must now also confirm — that a real `ci-results-<area-slug>` slice round-trips the now-optional `counts` field (present for a producer-reported cell, omitted entirely for a MISSING or version-1 cell, never serialized as zeros), and that the hosted render shows an unmeasured cell as `not recorded` in **both** the per-environment and per-area count columns while a genuinely measured zero stays a number
        - the `NOTHING TO RUN` case is called out explicitly in that record because it is the one a hosted run could plausibly regress: a cell that really did select zero tests must not flip to `not recorded`, and only real producer artifacts exercise that path end to end
        - Validations 5 and 7 remain `HOSTED-PENDING` in the acceptance table; no status was changed to claim evidence that does not exist
- work completed for 'Required hosted workflow behavior remains unverified' at 04:13:40

### Successful Completion

The implementation of review cycle 4 has completed successfully in 25 minutes.
During this implementation all 2 review findings were evaluated to see if they
could be fixed as a part of this implementation cycle: 1 was fixed, 1 was
deferred (see reasons below):

- **High — Required hosted workflow behavior remains unverified.** Deferred
  because its only valid verification boundary is a controlled GitHub Actions
  run on this fix's branch, which requires a commit and a push that this cycle
  was not authorized to perform. Review 4 reaches that conclusion itself: it
  files the finding under `## Blocked Findings` and states the probes run
  "after commit and push are explicitly authorized". The review also rules out
  every local substitute, stating that local planner, workflow-source, shell,
  and renderer fixtures "cannot establish GitHub's matrix expansion, nested
  reusable-workflow result propagation, artifact handoff, rendered check names,
  or `continue-on-error` conclusions", and closes by calling it "an
  integration-boundary gap, not a request for additional cross-OS proof or
  human design review". `acceptance.md` has carried Validations 5 and 7 as
  `HOSTED-PENDING` for the identical reason across three preceding cycles; this
  cycle extended that record with the two newly-introduced behaviors the hosted
  run must now also confirm.

This deferral is not a deferred performance measurement — no metric was blocked
by host CPU load — so `deferred_perf_measurement` is not set on this log's
frontmatter and no deferred-performance record was created.

One incident is recorded above and is repeated here so it is not missed: the
subagent working finding 1 ran `git checkout -- scripts/ci-rollup-tests.rs`
mid-task and destroyed that file's uncommitted review-2 and review-3 changes.
The file was reconstructed and independently verified by the orchestrator
against this log's own record of both prior cycles, and the whole suite is
green, but three lint regressions carry their original names and assertions
over newly written bodies and should be reviewed as new code:
`a_sub_second_lint_command_renders_its_measurement_rather_than_not_recorded`,
`a_zero_lint_duration_is_a_measurement_not_an_absence`, and
`a_lint_producer_status_carries_presence_independently_of_the_number`.

The files changed by this implementation cycle are:

- `scripts/ci-rollup.rs` — `Cell.counts` becomes `Option<Counts>`, making an
  absent measurement unrepresentable as zero at the type level and mirroring
  `duration_s`'s established serde shape; `ReusedResult.counts` follows;
  `classify_one` sets availability from the same `indices.is_empty() ||
  has_unusable_record` seam the state classifier already uses; the version-1
  measurements string is re-keyed from `outcome` onto `counts`, closing the
  same fabricated zero at a site the review did not name; new
  `partial_measurement` becomes the single owner of the complete/partial/`not
  recorded` spelling and its bounded list, with `environment_duration`
  rewritten onto it unchanged and `environment_tests` and `area_tests` as new
  callers; `grid_text` and `why` render an unmeasured cell as `not recorded`
  rather than `0/0/0` (finding 1)
- `scripts/ci-rollup-tests.rs` — the fixture and assertion sites migrated to
  `Option<Counts>`; a `measured_counts` helper that panics on absence, so a
  regression erasing a measurement cannot pass as a zero;
  `an_environment_no_producer_reported_renders_not_recorded_rather_than_zero`
  updated to stop requiring the `tests = 0` the review flagged and given a
  negative guard; eight regressions added covering the classifier seam, mixed
  and complete environments, a version-1 reused cell, a measured zero, and the
  per-area aggregate in both its unmeasured and mixed forms (finding 1)
- `fixes/_complete/2026-09-13-cicd-redundancies/acceptance.md` — the outstanding hosted
  work extended with what review iteration 4 added (finding 2)
- `fixes/_complete/2026-09-13-cicd-redundancies/log.md` — this record

## Implementation of Review Findings #5

> **started at:** 2026-09-15T04:24:33-07:00

- this implementation is attempting to implement _all_ of the review findings found in '/Volumes/coding/wt/rusty-biscuit/fix-cli-slow-tests/fixes/_complete/2026-09-13-cicd-redundancies/review-5.md'
- this is iteration 5 of the review-to-implement cycle
- orchestration plan decided after reading review 5, the specification, and the surfaces the review cites:
        - finding 1 (medium, result schema changes still identify themselves as version 3) — implementable here; touches `scripts/ci-rollup.rs` (`RESULT_SCHEMA_VERSION`, `reject_old_schema`, `load_rollup`), `scripts/ci-rollup-tests.rs`, and the four documents that pin the result document at `schema_version: 3`
        - finding 2 (high, required hosted workflow behavior remains unverified) — expected to defer: review 5 files it under `## Blocked Findings` and states the probes run only "after commit and push are explicitly authorized"; this cycle has no such authorization
        - gates run only in the areas the specification's Validation 2 names: the `scripts` Nextest bins (`repo-deps`) and `tools/test-toolkit`
- starting the work on 'Result schema changes still identify themselves as version 3' at 04:26:08
        - GitNexus impact, `node .gitnexus/run.cjs impact "<symbol>" --direction upstream --repo .`, bound to `rusty-biscuit`: `reject_old_schema` is `risk: LOW`, `epistemic: exact`, 7 impacted, 2 direct (`cmd_verdict`, `load_rollup`), no affected process, one module (`Scripts`); `load_rollup` is `risk: LOW`, 5 impacted, 2 direct (`cmd_summarize`, `load_rollups`). The committed index is stale for the working tree, so both were cross-checked with `grep -n "reject_old_schema\|load_rollup\|RESULT_SCHEMA_VERSION"` over `scripts/ci-rollup.rs` and `scripts/ci-rollup-tests.rs`; the grep found exactly the indexed call sites and no others, so the blast radius is one file plus its test bin
        - `grep -rn "ci-results\|RESULT_SCHEMA"` over `.github/workflows/*.yml`, `scripts/ci/*.py`, and `tools/test-audit` found no consumer that pins the result schema version — the workflows only name the `ci-results-<slug>` artifact, and the Python suites only assert on that artifact name. `tools/test-toolkit/tests/ci_workflow_contracts.rs` contains no `schema_version` assertion. Nothing outside `scripts/` needed changing
        - what the code actually did before the change: both readers deserialized the whole `Rollup` and only then called `reject_old_schema` (`load_rollup`, and a hand-inlined duplicate of the same three statements in `cmd_verdict`). Because a version-3 cell's `counts` is a required object and version 4's is optional, a v3 document whose cells the current struct cannot read answered with a serde type error instead of the migration — the exact ordering the review names
        - `scripts/ci-rollup.rs` — `RESULT_SCHEMA_VERSION` advanced 3 → 4 with its history line extended to say what version 4 adds (`counts` became an optional measurement alongside `duration_s`, so an unmeasured cell omits the field rather than serializing a zero that reads as a suite which found nothing). `BASELINE_SCHEMA_VERSION` left at 3
        - `scripts/ci-rollup.rs` — new `parse_rollup` reads a one-field `SchemaProbe` (`schema_version`, no serde default), runs `reject_old_schema`, and only then deserializes the full document. `load_rollup` delegates to it and `cmd_verdict` now calls `load_rollup` instead of re-inlining read/parse/check, so the two read sites cannot diverge on the ordering
        - `scripts/ci-rollup.rs` — `reject_old_schema` rewritten around a per-generation migration table: version 1 keeps its area-keyed message, version 2 keeps "area, origin, and evidence", version 3 gets its own text naming the optional-measurement migration, and an unnamed generation says its cell contract is not the one this tool reads rather than borrowing someone else's migration. Every branch still ends in "Re-run the rollup that produced it". The function's doc comment, which described only the 1 → 2 transition, now covers all four generations and why the error names a migration at all
        - non-vacuity, each proved by reverting exactly the behavior under test, observing the failure, and restoring:
                - `the_result_and_baseline_schemas_version_independently` with `RESULT_SCHEMA_VERSION` put back to 3: `assertion left == right failed / left: 3 / right: 4` at `scripts/ci-rollup-tests.rs:4356`
                - `a_superseded_result_document_is_refused_by_version_before_its_cells_are_read` with the deserialize-then-check order restored in `parse_rollup`: `the version must be refused by its migration: invalid result document …/v3-unreadable-cells.json: invalid type: string "three", expected struct Counts at line 1 column 93`. This is the ordering failure itself, not a rephrased assertion
                - `a_result_document_without_a_schema_version_is_refused_by_name` with `#[serde(default)]` added to the probe field: `the error must report the field as missing rather than as generation zero: result document …/unversioned-results.json is schema_version 0; this tool reads schema_version 4: its cell contract is not the one this tool reads. Re-run the rollup that produced it`. The first draft of this assertion only required the string `schema_version`, which the defaulted probe satisfied — it was tightened to require `no readable \`schema_version\`` and `missing field` after that mutation passed
                - `a_result_document_from_the_previous_generation_is_refused` with the version-3 arm collapsed onto the version-2 text: `the error must name the version-4 migration: … is schema_version 3; this tool reads schema_version 4: schema_version 3 adds each cell's area, origin, and evidence`. Its negative assertion is what forbids one generic message standing in for both migrations
        - `scripts/ci-rollup-tests.rs` — `the_result_and_baseline_schemas_version_independently` asserts 4 against the baseline's 3, so the independence claim still bites; `a_result_document_from_the_previous_generation_is_refused` extended to refuse versions 3 and 2 each with its own diagnostic plus a negative guard that one may not be told the other's migration; `a_superseded_result_document_is_refused_by_version_before_its_cells_are_read` added, whose v3 fixture carries `"counts":"three"` and first asserts `serde_json::from_str::<Rollup>` fails on it with `invalid type`, so the version rejection is proved to have outrun a parse that would otherwise have answered; `a_result_document_without_a_schema_version_is_refused_by_name` added. `an_area_keyed_result_document_is_refused_with_a_migration_error` and the `schema_version = 3` baseline TOML fixtures at lines 2002, 4605, and 4867 are unchanged — the baseline is independently versioned
        - `.github/ci/README.md`, `.github/ci/schemas/README.md`, `docs/topics/ci-cd.md`, `.claude/skills/rust-devops/ci-cd.md` — the result document repinned to `schema_version: 4` with the baseline stated separately as 3, what version 4 means recorded, and the refusal restated as happening by version before any cell is interpreted. The local-receipt `schema_version: 1` sentences (`.github/ci/README.md:276`, `docs/topics/ci-cd.md:100`, `.claude/skills/rust-devops/ci-cd.md:399`) and the environment-table version are untouched
        - `fixes/_complete/2026-09-13-cicd-redundancies/acceptance.md` — the outstanding hosted bullet for the optional-`counts` round trip now records that the change carries the result document to version 4 and that a retained version-3 slice must be refused by its version. No hosted evidence was invented
        - `fixes/2026-09-11-cicd-cleanup/plan.md:819` says the result document "is now `schema_version: 3`" and was deliberately left alone: it is that cycle's historical record of what it landed, not a live pin
        - gates, all green, each Rust one under an isolated `CARGO_TARGET_DIR`:
                - `just _test repo-deps` — **275 passed, 0 skipped** (273 baseline plus the two new regressions)
                - `just _test test-toolkit` — **183 passed, 2 skipped** (the two pre-existing Nextest self-verification fixtures)
                - `just _lint repo-deps` — passed
                - `just _lint test-toolkit` — passed
                - all ten registered Python CI suites through `scripts/ci/suite_runner.py` — `test_affected_scope` 183, `test_ci_local` 72, `test_constraints` 39, `test_evidence_reuse` 64, `test_local_evidence` 20, `test_publish_gaps` 19, `test_resolved_plan` 54, `test_reuse_validation` 15, `test_runner_loss` 35, `test_schema` 61, every one `OK`
                - `just ci-local --plan` — passed, 20 planned cells across `biscuit-tui`, `root`, and `tools` plus the two accepted L2 gaps, unchanged from the review's reading
                - `actionlint` not run: no workflow YAML was touched by this finding
        - finding 2 of review 5 (high, hosted workflow behavior) is untouched here: it is filed under `## Blocked Findings` and needs commit and push authorization this cycle does not have
- work completed for 'Result schema changes still identify themselves as version 3' at 04:35:40

### Successful Completion

The implementation of review cycle 5 has completed successfully in 12 minutes.
During this implementation all 2 review findings were evaluated to see if they
could be fixed as a part of this implementation cycle: 1 was fixed, 1 was
deferred (see reasons below):

- **High — Required hosted workflow behavior remains unverified.** Deferred
  because its only valid verification boundary is a controlled GitHub Actions
  run on this fix's branch, which requires a commit and a push that this cycle
  was not authorized to perform. Review 5 reaches that conclusion itself: it
  files the finding under `## Blocked Findings` and states that the probes run
  "after commit and push are explicitly authorized". The review also rules out
  every local substitute, stating that local planner, workflow-source, shell,
  and renderer fixtures "cannot establish GitHub's matrix expansion, nested
  reusable-workflow result propagation, artifact handoff, rendered check names,
  or `continue-on-error` conclusions", and closes by calling it "an
  integration-boundary gap, not a request for additional cross-OS proof or
  human design review". `acceptance.md` has carried Validations 5 and 7 as
  `HOSTED-PENDING` for the identical reason across four preceding cycles; this
  cycle extended that record with the result-schema behavior the hosted run
  must now also confirm — that a retained version-3 area slice is refused by
  its version rather than partly read.

This deferral is not a deferred performance measurement — no metric was blocked
by host CPU load — so `deferred_perf_measurement` is not set on this log's
frontmatter and no deferred-performance record was created.

The files changed by this implementation cycle are:

- `scripts/ci-rollup.rs` — `RESULT_SCHEMA_VERSION` advanced 3 → 4, because the
  serialized `Cell` contract changed incompatibly when `counts` became optional
  while the constant stayed put; new `parse_rollup` reads a one-field
  `SchemaProbe` and refuses a superseded generation *before* the full document
  is deserialized, with `cmd_verdict` routed through `load_rollup` so the two
  read sites cannot diverge on that ordering; `reject_old_schema` rewritten
  around a per-generation migration table so version 3 is told its own
  optional-measurement migration rather than version 2's, and an unnamed
  generation borrows nobody's; `BASELINE_SCHEMA_VERSION` left independent at 3
  (finding 1)
- `scripts/ci-rollup-tests.rs` — the version-independence assertion moved to 4
  against the baseline's 3; `a_result_document_from_the_previous_generation_is_refused`
  extended to refuse versions 3 and 2 each with its own diagnostic plus a
  negative guard against one message standing in for both; two regressions
  added, `a_superseded_result_document_is_refused_by_version_before_its_cells_are_read`
  (whose fixture carries a `counts` the current struct cannot parse, so the
  test fails on serde rather than on version if the ordering regresses) and
  `a_result_document_without_a_schema_version_is_refused_by_name` (finding 1)
- `.github/ci/README.md`, `.github/ci/schemas/README.md`,
  `docs/topics/ci-cd.md`, `.claude/skills/rust-devops/ci-cd.md` — the result
  document repinned to `schema_version: 4`, the baseline stated separately as
  3, and the refusal restated as happening by version before any cell is
  interpreted; the local-receipt `schema_version: 1` sentences are untouched
  (finding 1)
- `fixes/_complete/2026-09-13-cicd-redundancies/acceptance.md` — the outstanding hosted
  work extended with the version-4 result-slice refusal the hosted run must
  confirm (finding 2)
- `fixes/_complete/2026-09-13-cicd-redundancies/log.md` — this record

## Implementation of Review Findings #6

> **started at:** 2026-09-15T04:42:45-07:00

- this implementation is attempting to implement _all_ of the review findings found in '/Volumes/coding/wt/rusty-biscuit/fix-cli-slow-tests/fixes/_complete/2026-09-13-cicd-redundancies/review-6.md'
- this is iteration 6 of the review-to-implement cycle
- orchestration plan decided after reading review 6, the specification, and the two surfaces it cites:
        - finding 1 (medium, `.github/ci/schemas/README.md` still reports the baseline as version 2) — implementable here; the sentence at lines 34–39 contradicts `BASELINE_SCHEMA_VERSION = 3` in `scripts/ci-rollup.rs:71` and `schema_version = 3` in `.github/ci/ci-baseline.toml:40`. The review also asks for a documentation contract so this live version inventory cannot drift from the Rust constants and the shipped documents again
        - finding 2 (high, required hosted workflow behavior remains unverified) — expected to defer: its only valid boundary is a hosted GitHub Actions run, which needs a commit and push this session is not authorized to perform
        - findings are worked serially; gates run only in the areas the specification's Validation 2 names — the `scripts/ci/*.py` suites and `scripts` Nextest bins (`repo-deps`), and `tools/test-toolkit`
- starting the work on 'Schema reference still reports the baseline as version 2' at 04:44:14
        - the authorities, read first: `scripts/ci-rollup.rs:66` `RESULT_SCHEMA_VERSION = 4` and `:71` `BASELINE_SCHEMA_VERSION = 3`; `.github/ci/ci-baseline.toml:40` `schema_version = 3`; `scripts/ci/schema.py:68-75` `RESOLVED_PLAN_SCHEMA_VERSION = 3`, `RECEIPT_SCHEMA_VERSION = 2`, `SCOPE_RECEIPT_SCHEMA_VERSION = 1`. Iteration 5 had repinned the result document to 4 in the same sentence and left `the baseline's 2` beside it, which is exactly the drift review 6 caught
        - audited every other version claim the README states. The table rows (3 / 2 / 1), the producer sentence `write version 2`, and `their version stays at 2` are all currently true; the `Version 1` / `Version 2` / `Version 3` paragraphs are the resolved plan's generation history, not live counters. Only `the baseline's 2` was wrong, so exactly one number changed
        - ownership: each number is asserted against the module that defines it, so neither test hard-codes a copy. `scripts/ci-rollup-tests.rs` (package `repo-deps`) is `#[path]`-included into `ci-rollup.rs`, so `RESULT_SCHEMA_VERSION` and `BASELINE_SCHEMA_VERSION` are in scope as the compiled constants themselves — it owns the result and baseline claims and additionally cross-checks the baseline against the shipped `.github/ci/ci-baseline.toml`. `scripts/ci/test_schema.py` already imports `schema` and already reads a shipped repository file (`ci.yml`, at line 649), so it owns the plan / receipt / scope-receipt claims. Both suites belong to `repo-deps`, so no new suite owner appears. Neither needs `regex`: adding a crate to `repo-deps` would slow the deliberately dependency-light `ci-rollup` bin, so the Rust side scans with literal anchors
        - skip convention reused, not invented: the new Rust test opens with the surrounding suite's `let Some(root) = checkout_root() else { eprintln!(...); return; }` guard, matching `plan_fields_match_the_frozen_contract` and `the_shipped_capability_table_names_what_closes_its_l2_gaps`
        - non-vacuity, Rust half, run against the UNCORRECTED README before any prose was edited — `scripts/ci-rollup-tests.rs:4497` in `the_schema_readme_states_this_tools_versions`:
                - `thread 'tests::the_schema_readme_states_this_tools_versions' panicked at scripts/ci-rollup-tests.rs:4497:5: assertion `left == right` failed: the schema README states baseline version 2, but `BASELINE_SCHEMA_VERSION` is 3 / left: 2 / right: 3`
                - `Summary [0.008s] 1 test run: 0 passed, 1 failed, 219 skipped`
        - anti-vacuous-match proof, Rust half. Mutating the result version to 5 fails at `scripts/ci-rollup-tests.rs:4489`: `the schema README states result schema_version 5, but `RESULT_SCHEMA_VERSION` is 4`. Rewording the sentence to `at result generation four` fails at `:4488` with `the schema README must still state the result document's version` — a rewording is a failure, not a silent pass, because `version_stated_after` returns `None` and the caller `expect`s it
        - non-vacuity, Python half. The plan/receipt/scope numbers were already correct, so the contract was proved by mutation instead: `| Validation receipt | 9 |` → `AssertionError: 2 != 9`; `the plan's 7` → `AssertionError: 3 != 7`; renaming the row to `| Receipt (validation) |` and rewriting the sentence to `versioned separately from the plan, receipt` both fail through the explicit no-match branches rather than matching nothing. `> write version 5;` → `AssertionError: 2 != 5`; `> version stays at 4 and` → `AssertionError: 2 != 4`. A first draft of the producer pattern used `[^.]*`, which could not cross the `.` in `scripts/cross-check.sh` and therefore never matched; the mutation run is what exposed it, and it is now `.*?`
        - `.github/ci/schemas/README.md` — one number: `the baseline's 2` → `the baseline's 3`. Nothing else in the file changed
        - `scripts/ci-rollup-tests.rs` — added `schema_readme_prose` (strips blockquote markers and un-wraps the README into one line, so a claim spanning three wrapped lines reads as one sentence), `version_stated_after` (reads the number after a literal, `None` when the literal is gone), and `the_schema_readme_states_this_tools_versions`, which asserts the README's stated result version against `RESULT_SCHEMA_VERSION`, its stated baseline version against `BASELINE_SCHEMA_VERSION`, and that same stated number against the shipped `ci-baseline.toml`'s `schema_version`. The existing `the_result_and_baseline_schemas_version_independently` was left alone: it pins the constants, the new test pins the prose, and keeping them apart keeps each failure legible
        - `scripts/ci/test_schema.py` — added `SCHEMA_README_PATH`, `schema_readme_prose`, and `SchemaReadmeVersionTests` with three cases: the table's three rows against `RESOLVED_PLAN_SCHEMA_VERSION` / `RECEIPT_SCHEMA_VERSION` / `SCOPE_RECEIPT_SCHEMA_VERSION`, the prose inventory's plan and receipt numbers against the first two, and the two prose restatements of the receipt version. Every regex has an explicit `if match is None: raise AssertionError(...)` branch naming the reworded claim, so no case can pass by matching nothing. `import re` added
        - gate `just _test repo-deps` — `276 tests run: 276 passed, 0 skipped` (275 before this change; the one addition is `the_schema_readme_states_this_tools_versions`)
        - gate `just _lint repo-deps` — exit 0, clippy `-D warnings` across `--all-targets`, no warnings emitted
        - gate `python3 scripts/ci/test_schema.py` (the canonical invocation, the form `just/ci-local.just:432` uses) — `Ran 64 tests ... OK`, exit 0 (61 before this change; three cases added)
        - `tools/test-toolkit` untouched, so its two gates were not run
        - all cargo gates ran under `CARGO_TARGET_DIR=/tmp/rb-iter6-target` to avoid the shared `target/debug/deps` non-writable files earlier cycles hit; the user's shared cache was neither deleted nor modified
        - not done: no formatter was run (`just _lint` is clippy-only here), so the new Rust helpers' formatting is unverified against rustfmt
- work completed for 'Schema reference still reports the baseline as version 2' at 04:50:05
- starting the work on 'Required hosted workflow behavior remains unverified' at 04:50:20
        - re-read the finding against the current worktree before deferring rather than assuming the previous cycles' disposition still held. Its boundary is unchanged: `HEAD` is still `4b6e8944f` with every change of this fix uncommitted, no push has been authorized in this session, and the specification's Validations 5 and 7 can only be satisfied by GitHub's own job graph — matrix expansion, nested reusable-workflow result propagation, artifact handoff between jobs, rendered check names, and `continue-on-error` conclusions have no local substitute
        - checked whether this iteration adds anything to the hosted probe's required scope. It does not: the only behavioral surface touched was `.github/ci/schemas/README.md` prose plus two contract suites that read shipped repository files. Nothing new is observable on a runner, so the `## Outstanding` hosted list in `acceptance.md` was left exactly as iteration 5 wrote it — including the version-4 slice declaration and the refusal of a retained version-3 slice by version. Extending that record with a documentation assertion would have made the hosted probe claim to verify something a runner cannot see
        - confirmed `acceptance.md:164-260` still marks Validations 5 and 7 `HOSTED-PENDING` with the accumulated review fixes the eventual run must exercise, which is the record review 6 asks to be honored rather than replaced
- work completed for 'Required hosted workflow behavior remains unverified' at 04:51:25 — deferred, not implemented

### Successful Completion

The implementation of review cycle 6 has completed successfully in 8 minutes 45
seconds. During this implementation all 2 review findings were evaluated to see
if they could be fixed as a part of this implementation cycle: 1 was fixed, 1 was
deferred (see reasons below):

- **Finding 2 (high) — "Required hosted workflow behavior remains unverified"**
  is deferred because its only valid verification boundary is a hosted GitHub
  Actions run, and this session was not authorized to commit or push. Every
  change of this fix is still uncommitted against `4b6e8944f`, so there is no
  branch state for a runner to execute. The behaviors the finding names —
  GitHub's matrix expansion, nested reusable-workflow result propagation,
  artifact handoff, rendered check names, and `continue-on-error` conclusions —
  are properties of the Actions job graph itself; the local planner,
  workflow-source, expression, shell, and renderer fixtures already committed to
  this fix reach the strongest boundary available without a push, and the review
  agrees they cannot establish the hosted ones. `acceptance.md` has carried
  Validations 5 and 7 as `HOSTED-PENDING` for this identical reason across five
  preceding cycles; this cycle added no new runner-observable behavior, so that
  record was honored unchanged rather than extended.

This deferral is not a deferred performance measurement — no metric was blocked
by host CPU load — so `deferred_perf_measurement` is not set on this log's
frontmatter and no deferred-performance record was created.

The files changed by this implementation cycle are:

- `.github/ci/schemas/README.md` — the live version inventory's baseline claim
  corrected from 2 to 3, matching `BASELINE_SCHEMA_VERSION` in
  `scripts/ci-rollup.rs:71` and `schema_version = 3` in
  `.github/ci/ci-baseline.toml:40`. Every other version claim in the file was
  audited and is already true, so exactly one number changed (finding 1)
- `scripts/ci-rollup-tests.rs` — `the_schema_readme_states_this_tools_versions`
  added, asserting the README's stated result and baseline versions against the
  compiled `RESULT_SCHEMA_VERSION` and `BASELINE_SCHEMA_VERSION` (in scope
  because this file is `#[path]`-included into `ci-rollup.rs`, so they are the
  constants themselves and not copies) and cross-checking the stated baseline
  against the shipped `ci-baseline.toml`; helpers `schema_readme_prose` and
  `version_stated_after` added, the latter returning `None` so a reworded claim
  fails loudly instead of matching nothing. Proved non-vacuous against the
  uncorrected README at `scripts/ci-rollup-tests.rs:4497` before any prose was
  edited (finding 1)
- `scripts/ci/test_schema.py` — `SchemaReadmeVersionTests` added, asserting the
  README's table rows and prose restatements against
  `RESOLVED_PLAN_SCHEMA_VERSION`, `RECEIPT_SCHEMA_VERSION`, and
  `SCOPE_RECEIPT_SCHEMA_VERSION`; every regex carries an explicit no-match
  branch, and each case was proved non-vacuous by mutation because those numbers
  were already correct (finding 1)
- `fixes/_complete/2026-09-13-cicd-redundancies/log.md` — this record

Gates, all re-verified by the orchestrator after the subagent reported them:
`just _test repo-deps` 276 passed / 0 skipped, `just _lint repo-deps` clean,
`python3 scripts/ci/test_schema.py` 64 tests OK. `tools/test-toolkit` was not
touched by this cycle and its gates were not run. No formatter, commit, push,
workflow dispatch, or full-workspace run was performed.

## Closure — 2026-09-15

The user accepted the remaining hosted integration uncertainty and requested
closure and archival under `fixes/_complete/`. Validations 5 and 7 and accumulated
hosted assertions are deferred, not passed. Updated the specification, acceptance
record, final review, and archival plan; corrected the stale three-OS lint claim
to match the Ubuntu-only production job. Earlier review entries remain historical.
Observe normal CI runs and report focused defects; reopen only if a core design
assumption fails. No hosted probes, commit, or push were performed for closure.
