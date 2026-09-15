---
title: Phase 11 acceptance record
created: 2026-09-14
phase: 11
spec: fixes/2026-09-13-cicd-redundancies/spec.md
plan: fixes/2026-09-13-cicd-redundancies/plan.md
rulings: fixes/2026-09-13-cicd-redundancies/rulings.md
oracles: fixes/2026-09-13-cicd-redundancies/oracles.md
tree: 45d729dbc (branch `fix/cicd-improvements`), working tree
host: macOS 27.0.0, Apple Silicon; native Windows evidence from `$BUILD_WIN`
---

# Phase 11 — Acceptance Record

Every acceptance criterion in the specification, with the named, re-runnable
artifact that demonstrates it. Criteria whose artifact is a hosted GitHub
Actions run are marked **HOSTED-PENDING** and carry the local evidence that
does exist; see [Outstanding](#outstanding).

## Validation steps

| # | Validation step | State | Artifact |
|---|---|---|---|
| 1 | Pending fixtures fail before promotion | done (Phase 2) | `oracles.md`; no `@pending` / `pending_contract` decorator survives |
| 2 | Dependency-derived local scope | **done** | see [Validation 2](#validation-2) |
| 3 | Dual-directory workspace | **done** | see [Validation 3](#validation-3) |
| 4 | Documentation-only fixtures, three levels | **done** | see [Validation 4](#validation-4) |
| 5 | Hosted run + two forced-failure probes | **HOSTED-PENDING** | requires push; see [Outstanding](#outstanding) |
| 6 | Native `windows-latest` runtime evidence | **native evidence done**, hosted JUnit pending | see [Validation 6](#validation-6) |
| 7 | Documentation-only follow-up push | **HOSTED-PENDING** | requires push; see [Outstanding](#outstanding) |
| 8 | `.github/ci/README.md` + `docs/topics/ci-cd.md` in same change | done (Phase 10) | both modified in this tree |

### Validation 2

All eight gates run and green on this tree:

```sh
# 1. the ten compact Python CI suites (each exits 0, prints OK)
for s in test_affected_scope.py test_ci_local.py test_constraints.py \
         test_evidence_reuse.py test_local_evidence.py test_publish_gaps.py \
         test_resolved_plan.py test_reuse_validation.py test_runner_loss.py \
         test_schema.py; do python3 scripts/ci/suite_runner.py "$s"; done

just _test repo-deps        # 257 tests run: 257 passed, 0 skipped
just _test test-toolkit     # 179 tests run: 179 passed, 2 skipped
cd tools/test-audit && just check   # tsc clean; 189 passed | 29 skipped (218)
python3 scripts/ci/schema.py        # regenerates contract.json byte-identically
actionlint                          # exit 0
just ci-local --plan                # exit 0, renders inventory + 21 cells
```

No full-workspace test run was substituted.

The `ci_workflow_contracts` suite (`test-toolkit`) is the workflow-contract
gate; its 21 CI-contract fixtures are enumerated per criterion below.

### Validation 3

`cargo metadata` from the repository root and from `scripts/` returns the
identical `workspace_root` and `target_directory`, `repo-deps` is a member from
both, and `scripts/Cargo.lock` does not exist. Nextest profile resolution is
proved live rather than assumed: from `scripts/`, an unknown profile errors
with `known profiles: ci, default, default-miri, local-evidence` — exactly the
set defined in the root `.config/nextest.toml`.

### Validation 4

Three fixtures, one per ownership level, each via
`affected_scope.py --resolved-plan`:

| Fixture | packages | areas | cells | class |
|---|---|---|---|---|
| `docs/topics/ci-cd.md` | none | none | 0 | `documentation` |
| `darkmatter/docs/composition/index.md` | none | none | 0 | `documentation` |
| `biscuit-file/README.md` | none | none | 0 | `documentation` |

Each names its document in **both** renderers from one payload:

- terminal (`ci-plan`): `- documentation (1) — biscuit-file/README.md`
- Markdown (`ci-rollup summarize`): ``- **documentation** (1) — `biscuit-file/README.md` ``

Both also print `No package test is required: the resolved plan schedules no
package cell.`, and the Markdown report adds `The merge decision belongs to
ci-gate ...` (AC11).

Byte-equivalence is structural, not incidental: `scripts/ci-change-inventory.rs`
is `#[path]`-included by both binaries, so one `ChangeInventory` deserializer
serves both. It is also enforced by
`repo-deps::bin/ci-rollup the_markdown_report_projects_the_shared_inventory_payload_unchanged`.

### Validation 6

Native Windows runtime evidence on `$BUILD_WIN` via `just cross-check`, under
the package's declared `[package.metadata.ci.tests].features` and the **real
`ci` nextest profile** (the JUnit-writing profile):

```
Nextest run ID f5317a6d-51c1-411e-93ab-5cf06c570493 with nextest profile: ci
    Starting 1 test across 16 binaries (398 tests skipped)
        PASS [0.041s] (1/1) biscuit-tui-cli::windows_captured_stdout \
                            captured_stdout_receives_only_value_no_tui_bytes
```

This is native runtime evidence, not the macOS→Windows GNU cross-compile the
specification excludes.

**Negative probe.** With the console precondition deliberately broken
(`stderr_is_console` forced to `false` in `assert_console_precondition`), the
same native Windows cell fails:

```
    Summary [0.019s] 1 test run: 0 passed, 1 failed, 398 skipped
        FAIL [0.012s] (1/1) captured_stdout_receives_only_value_no_tui_bytes
cross-check summary for biscuit-tui-cli:
  windows  FAIL
```

The probe edit was reverted byte-identically (sha1
`ea8194ebcbaff019cbb7e02732b627e8631cd995`, `git diff` clean).

L1 classification is not asserted, it is derived: the L1 filterset is
`!(test(/(^|::)level2_/) + level3_ + browser_ + real_ + slow_)`, and neither the
binary `windows_captured_stdout` nor the test name carries any of those
prefixes. The earlier full-suite run that reported `398 tests run` excluded this
test only because `--features terminal-tests` was not passed; CI's L1 cell
declares that feature in `[package.metadata.ci.tests]`.

## Acceptance criteria

| AC | Criterion | Artifact |
|---|---|---|
| 1 | No suite in both `preflight` and another job | `grep -c 'scripts/ci/test_' .github/workflows/ci.yml` → `0`; `preflight` holds prerequisites only |
| 2 | `repo-deps` is a root member, no nested workspace/lockfile | `cargo metadata` from both dirs; `scripts/Cargo.lock` absent; no `[workspace]` in `scripts/Cargo.toml` |
| 3 | `test-toolkit` gates normally, exclusion removed | no `promotion-pending` in `tools/test-toolkit/Cargo.toml`; `just ci-local --plan` schedules 6 `test-toolkit` cells |
| 4 | Every suite has one owner/recipe/trigger/cell/outcome; bad suites fail a test | `SUITE_REGISTRY`; `test_an_unknown_suite_name_fails_validation`, `test_a_doubly_owned_suite_fails_validation`, `test_a_recipe_less_suite_fails_validation`, `test_an_unknown_suite_kind_fails_validation`, `test_unknown_companion_suite_is_rejected`, `the_producer_status_carries_one_record_per_companion_suite` |
| 5 | Tooling path-to-owner selection | `affected_scope.py scripts/ci/schema.py` → `['repo-deps']`; `... tools/test-audit/package.json` → `['test-toolkit']`; `... biscuit-file/README.md` → `[]`. Contract: `tooling_inputs_select_their_registered_suite_owner` |
| 6 | Doc-only follow-up reuses eligible cells, reports both outcomes | `a_reused_cell_reaches_its_area_summary_without_being_re_executed`, `the_grid_shows_a_reused_cell_with_its_origin_and_evidence`, `post_merge_reuse_preserves_the_gate_and_normal_ci_fallback`. **HOSTED-PENDING** for the live push |
| 7 | Retired jobs/workflow/recipe gone | `ci.yml` defines exactly `validation, scope, preflight, area-ci, ci-gate, ci-reporting`; `biscuit-tui-windows-captured-stdout.yml` absent; no `test-windows-captured-stdout` recipe. Contracts: `retired_specialized_workflows_and_jobs_are_absent`, `no_reader_facing_document_or_recipe_names_a_retired_ci_entity` |
| 8 | Windows test discovered/run by normal L1 cell, in JUnit, fails on broken precondition | [Validation 6](#validation-6) — native pass under `ci` profile + negative probe. Hosted JUnit **HOSTED-PENDING** |
| 9 | Bounded readiness observation, no fixed sleep, no window focus | `submit_and_wait` polls with `SUBMIT_DEADLINE` / `REINJECT_AFTER` and kills on deadline; the only `thread::sleep` is `POLL_INTERVAL` inside that loop. No `#[ignore]`. Console is allocated, never focused |
| 10 | Plan schema carries the inventory; validation, contracts, receipts, renderers agree | `contract.json` `resolved_plan.schema_version: 3` with `change_inventory`; regenerates byte-identically; `the_inventory_projection_names_every_changed_path`, `a_plan_written_before_the_inventory_existed_still_renders` |
| 11 | Doc-only change names documents, states no tests, zero executions, defers to `ci-gate` | [Validation 4](#validation-4) |
| 12 | `ci-reporting` renders deps, counts, durations, lint duration, companions, exact origins; applies no policy | `per_environment_rows_carry_counts_duration_and_the_plan_origin_vocabulary`, `a_lint_cell_is_reported_as_the_linux_only_ci_command_duration`, `a_measured_companion_reports_its_counts_and_duration`, `only_ci_gate_makes_a_run_level_claim`, `ci_reporting_aggregates_through_ci_rollup_rather_than_reparsing_artifacts`. **HOSTED-PENDING** for the live render |
| 13 | Missing measurements render unavailable with a reason, never `0`/`ci`/pass | `a_companion_with_no_counts_renders_not_recorded_never_zero`, `a_companion_with_no_recorded_counts_is_never_reported_as_zero`, `an_unmeasured_lint_command_renders_not_recorded_rather_than_zero`, `an_environment_no_producer_reported_renders_not_recorded_rather_than_zero`, `a_version_one_receipt_renders_its_measurements_as_unrecorded`, `a_lint_status_with_no_duration_records_none` |
| 14 | Zero-entry area matrix resolves skipped, never shows a raw expression | `preflight` and `area-ci` use **scalar** guards (`preflight_os != '[]'`, `has_packages == 'true'`) read before matrix expansion, and declare no `name:`. A doc-only plan yields `preflight_os=[]`, `scheduled_areas=[]`, `has_packages=false`. Contract: `the_fan_out_gate_derives_from_the_matrix_not_the_impacted_list` |
| 15 | `ci-gate` accepts only `success`/`skipped`, reads no plan/artifact/baseline, still required, no retired job in `needs` | job body folds `needs.*.result` only, `case success\|skipped`; `needs: [validation, scope, preflight, area-ci]`. Ruleset `protect-your-bacon` (id 19747338, active) requires exactly `ci-gate`, `updated_at 2026-09-13` — **not edited by this fix**. Contracts: `ci_gate_is_the_single_required_check`, `ci_gate_needs_exactly_the_four_surviving_blocking_jobs`, `every_top_level_job_is_either_folded_by_ci_gate_or_the_advisory_report`, `advisory_jobs_cannot_fail_the_run_and_gates_are_not_advisory` |
| 16 | All-reused areas still fan out far enough to produce result slices | `an_all_reused_area_still_produces_its_result_slice`, `a_reused_cell_reaches_its_area_summary_without_being_re_executed` |

## Pending-fixture sweep

No Phase 2 oracle survives. The only remaining `@pending` references are
`scripts/ci/pending_contracts.py` (the decorator's own definition and
docstring) and `scripts/ci/test_schema.py` (three locally-constructed
decorators that test the decorator's own three outcomes, including that a
landed contract raises `ContractLanded`). The Rust `pending_contract` wrapper is
documented as retired in `ci_workflow_contracts.rs`.

## Skipped and pre-existing conditions

- `test-toolkit`: 2 skipped — `nextest_config_verification::{cargo_nextest_flags_slow_test_in_output, slow_fixture_for_nextest_verification}`. Pre-existing `#[ignore]` fixtures that verify nextest's own slow-test reporting; unrelated to this fix.
- `tools/test-audit`: 29 skipped of 218 — the `*-claudine-compat` suites, gated on a Claudine fixture; pre-existing.
- `kache` prints `EXDEV / Cross-device link (os error 18)` advice during local `cargo nextest` runs. A host storage-layout warning, not a test result.

## Outstanding

Two validation steps require a commit and a push, which this phase was
explicitly instructed not to perform:

1. **Validation 5 — hosted run on this branch.** Job ownership, artifact
   collection, report aggregation, `ci-reporting`'s advisory failure behavior,
   prompt zero-matrix resolution, plus the two forced-failure probes (a failed
   package-owned tooling suite must block through `area-ci` → `ci-gate`; a
   forced `ci-reporting` failure must not block).
2. **Validation 7 — documentation-only follow-up push.** Eligible prior cells
   report as reused, no suite is silently dropped. One `scope-schema` receipt
   miss is the expected v2→v3 migration behavior and should be recorded rather
   than suppressed.

`AC8`'s hosted `biscuit-tui-cli/windows-latest/L1` JUnit artifact also lands
with Validation 5; the native runtime and negative-probe halves are already
proved above.
