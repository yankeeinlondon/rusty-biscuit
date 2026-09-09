---
area: claudine
status: unscheduled
created: 2026-09-08
origin: claudine/fixes/2026-09-07-faster-claudine-tests/results.md
packages:
    - claudine
    - claudine-cli
    - claudine-contract
    - claudine-gen
---

# Test-suite residuals deferred by the 2026-09-07 faster-tests fix

Eight findings the
[faster-claudine-tests fix](../../2026-09-07-faster-claudine-tests/results.md)
surfaced and could not close inside its scope. Each carries the evidence it
was found with, the reason it was deferred, and what closing it looks like.
None is a fixture migration: the L1 spawn allow-list is empty and that clause
of the fix's acceptance criteria is not what any of these defers.

Two have moved since. Residual 7's tooling half shipped on 2026-09-09 and its
evidence half remains open; it is rewritten in place rather than split, so the
entry still reads as one deferral. Residual 8 was added on 2026-09-09 when
review 1's closure criterion 1 was worked: its inventory half closed and its
measurement half could not.

## 1. Four zero-byte bench entry points

- **Evidence:** `claudine/lib/benches/{claude_parse,opencode_parse,pre_flight_checks,prompt_preparation}.rs`
  are 0 bytes, tracked since `0110a7a76`. Only `runtime_hot_paths.rs` is a
  declared `[[bench]]`; the four are auto-discovered, compile as empty libtest
  bench targets on every `just bench`, and contribute nothing
  (`inventory.md` § Benches).
- **Why deferred:** a hygiene change with no test-performance content;
  deleting them is behaviour-free and belongs with whoever owns the benchmark
  surface, in its own commit.
- **Closes when:** the four files are deleted or given real benchmarks, and
  `just bench` still exits 0.

## 2. `context` reports have no in-process render seam

- **Evidence:** `render_default_report` and its siblings write to `log::data`
  rather than returning text, so the two `context_command` width sweeps
  (12 and 7 launches) can only be proved at the CLI boundary. Phase 6 hoisted
  the fixture out of the loops (2.18 s → 0.90 s for the widest sweep) but
  every launch remains.
- **Why deferred:** adding a capture seam is a production change, which the
  fix's scope excludes.
- **Closes when:** a renderer returns (or accepts a sink for) the report text,
  the width × mode matrix moves to a library test, and one representative
  real-binary case per report mode stays in `context_command.rs`.

## 3. `a_failed_ownership_setup_kills_the_spawned_command` holds vacuously

- **Evidence:** in `composition::sequence::task::tests`, the injected failure
  fires on the statement after `spawn`, so the kill reaches the shell before
  its first command runs and nothing is ever backgrounded; the
  `!marker.exists()` assertion has always been true. Converting it to
  `BackgroundedDescendant` fails on exactly that (Phase 7, `plan.md`).
- **Why deferred:** strengthening it needs the task runner to expose the
  direct child's pid, a production change. The limit is now stated at the
  test rather than implied by its name.
- **Closes when:** the runner exposes the child pid (or an equivalent
  observation), and the test asserts that a spawned child is killed on
  ownership-setup failure with a non-vacuous witness.

## 4. `real_provider` fails `Unauthorized` where its contract says it skips

- **Evidence:** `just test-real` in `claudine/`: four
  `claudine-contract::real_provider` identities fail with `Unauthorized` on a
  host whose provider CLI is not authenticated, identically under the retired
  `cargo test` route (Phase 6). The file's module contract says each test
  "skips cleanly when its provider/model is unavailable".
- **Why deferred:** deciding whether an expired credential is "unavailable"
  or a real failure is a contract question for the adapter's owner; the fix
  recorded the tier as pending rather than changing what the tests assert.
- **Closes when:** the contract names the credential case explicitly and the
  tests either skip with a reason or fail with a diagnostic that says
  "authenticate", and `just test-real` on an authenticated host is 5 of 5.

## 5. `completion_perf::perf_enter_compose_partial_meets_target` fails on this host

- **Evidence:** the `#[ignore]`d harness reports
  `autocomplete requires an interactive terminal` from its PTY chooser, before
  and after the fixture migration (Phase 5, verified by restoring the
  pre-migration file).
- **Why deferred:** pre-existing, reachable only by `--ignored`, and a
  wall-clock budget assertion the fix's own measurement rules keep out of
  always-on gates.
- **Closes when:** the harness either drives the chooser through a PTY that
  satisfies the interactivity check or documents the host requirement and
  skips with that reason.

## 6. `claudine-cli-ci-l1` runs at `max-threads = 1`

- **Evidence:** the CI-profile test group serialises `claudine-cli`'s ~2,500
  L1 identities, so runner elapsed equals summed duration on every leg
  (`baseline/34173378609/`: 324.9 vs 324.7 s on Ubuntu). It is the single
  largest CI cost lever in the area (`inventory.md` § Runner override census,
  entry 11).
- **Why deferred:** relaxing it needs the candidate CI runs the fix could not
  produce (no push); a cap change without that evidence is the runner-limit
  change AC5/AC7 forbid.
- **Closes when:** three green candidate runs per leg exist, the group is
  raised one step at a time, and each step's four legs stay green with no
  new `LEAK`, timeout, or flake.

## 7. Budget derivation: aggregator built, runs still missing

**The aggregator half is done (2026-09-09). The evidence half is not.**

- **Original evidence:** `deriveBudgets` refused (`missing-leg`) while
  `perLegFamilySummed` was empty, because `attribute` reads nextest logs and
  the CI evidence is JUnit XML. `reconcile`'s matcher already mapped an
  identity to a family, so the missing piece was a join, not a classifier.
- **Done:** `test-audit attribute aggregate <run-dir>... --config <cfg>`
  (`tools/test-audit/src/attribute/aggregate.ts`) walks the stored
  `<run-id>/<env>/<tier>/<package>.xml` staging trees, projects every cell
  through `invocationFromJunit`, resolves each `<testcase>` through
  `familiesMatching`, and emits exactly the `BudgetInput` that
  `attribute budgets --runs` reads. It is a gate: 0 clean, 1 on violation,
  2 on usage. A leg contributes a sample only when its evidence is clean —
  a red run, a malformed or missing report, a manifest naming another
  environment, a declared leg absent from the tree, or an identity that zero
  or two families claim disqualifies that leg for that run.
  36 tests (`tests/aggregate.test.ts`, `tests/aggregate-claudine-compat.test.ts`),
  the second reading the real shipped baseline.
- **Demonstrated:** against `baseline/34173378609/` the aggregator exits 0 with
  18 families on each Unix leg and 15 on `windows-latest`, reproducing the
  summed-duration column already recorded in `results.md`; feeding its output
  to `attribute budgets` still refuses, now with `insufficient-runs`
  (`1 green run(s); 3 consecutive are required`) instead of `missing-leg`.
  That is the correct end state, and it is a smaller refusal than before.
- **Still open, and operator-gated:** two more consecutive green `main` runs per
  leg at a comparable source state, plus the candidate tranche. Neither can be
  produced without merging `main`, signing, pushing, and waiting on GitHub
  Actions. `attribution/budgets-pending.json` deliberately still declares
  `runsPerLeg` 1 and an empty `perLegFamilySummed`; raising either by hand
  would be fabricating evidence.
- **Closes when:** three consecutive green runs per leg exist, the aggregator's
  output replaces `budgets-pending.json`'s hand-written provenance,
  `deriveBudgets` exits 0, and the resulting table lands beside the baseline in
  `inventory.md` § Budgets.

## 8. Local cost measurement cannot be re-run against a red suite

- **Evidence (2026-09-09, gates run rather than assumed):** `just test-cli
  --no-fail-fast` in `claudine/` — 2523 run, 2515 passed, **7 failed, 1 timed
  out**, 9 skipped. `just _test claudine-gen --no-fail-fast` — 155 run, 145
  passed, **10 failed**. Eighteen failing identities in two packages:

  | Failing identity | Count | Cause |
  |---|---:|---|
  | `claudine-cli::bin/claudine …commands::wrap::harness_orch::loop_control::target_launch::tests::*` | 5 | commit `f0aaa4832` |
  | `claudine-cli::propagated_context_fixtures::isolated_fixture_can_opt_in_to_provider_memory_discovery` | 1 | — |
  | `claudine-cli::spawn_inventory::production_spawn_inventory_is_complete_and_governed` | 1 | line-number drift from `f0aaa4832` |
  | `claudine-cli::wrap_sigint::compose_sigint_during_prep_exits_130_with_notice` | 1 | 30 s timeout |
  | `claudine-gen::drift::committed_{generated_artifacts_match_phase_1_byte_baseline,signals,data,vocabulary,families}` | 5 | archived-baseline break |
  | `claudine-gen::generate_ux::{non_tty_check_output_has_no_ansi,force_color_check_output_carries_ansi,no_color_check_output_has_no_ansi,clean_area_generates_nothing_and_exits_zero,clean_check_report_summary_matches_phase_1_snapshot}` | 5 | same |

  All eighteen predate the review-1 remediation cycle and none is in a file it
  touched; the two counts match the recorded pre-existing baseline exactly.
- **Why it is deferred, not skipped:** the measurement protocol is
  [`spec.md` § 5](../../2026-09-07-faster-claudine-tests/spec.md)'s five
  alternating warm runs per cohort, and `test-audit attribute` **refuses a red
  run by design** — a log carrying failures, or whose result lines disagree
  with its own `Summary` line, is rejected rather than averaged in. Running the
  protocol anyway would produce either a refusal or, if the refusal were
  weakened, a number that silently excluded eighteen tests. Repairing the
  eighteen is production and generated-artifact work outside the faster-tests
  fix's scope.
- **What is *not* affected:** the runner-identity inventory. Its captures were
  regenerated on 2026-09-09 (`cargo nextest list` is a listing, not a run, so a
  red suite does not block it), the superseded `9fc5151a0` listings are
  preserved under `enumeration/9fc5151a0/`, and `inventory-reconciler.ts` exits
  0 over 7,417 identities.
- **Consequence, stated wherever a number is affected:** every **Executed** and
  **Summed cost** figure in `inventory.md` and every figure in
  `attribution.md` remains the `9fc5151a0` measurement and is labelled as such.
  Six families' identity counts have since moved, so for those six the recorded
  cost is a lower bound on the family rather than a measurement of it.
- **Repairability note for the scheduler:** `spawn_inventory` is line-number
  drift and may be a one-line update; the other seventeen were not diagnosed.
  This entry deliberately does not repair any of them — a documentation and
  evidence work unit must not edit production code — and takes no position on
  whether they belong here or in their own fix.
- **Closes when:** the eighteen are green, the § 5 protocol is re-run, and
  `attribution.md`, `inventory.md`'s **Executed** / **Summed cost** columns and
  `measurement/` are regenerated together from that run.
