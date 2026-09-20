---
kind: baseline-record
feature: 2026-09-19-direct-cell-execution
created: 2026-09-20
plan_phase: 1
tree_head: <recorded below>
status: complete
---

# S0 — Current-state baseline

Every number and name below was captured from this tree
(`feat-single-os` worktree) on 2026-09-20. Nothing here is a hosted
observation. Later phases compare against this inventory to know exactly what
they edit.

- tree head at capture: `1266e5fc98fef03535773ec7fe8a1519024c45e2`
  (`1266e5fc9`); production recipes in `plans/README.md` in this directory
- resolved-plan schema: 4 (`RESOLVED_PLAN_SCHEMA_VERSION`);
  `RECEIPT_SCHEMA_VERSION` 2; `SCOPE_RECEIPT_SCHEMA_VERSION` 1
- `MATRIX_LIMIT` = 256 (`scripts/ci/affected_scope.py:436`)

## 1. Producer job labels in the four reader-facing workflows

A called workflow's jobs render as `<caller job label> / <called job label>`,
so a producer cell is three or four segments deep
(`scripts/ci/runner_loss.py`):

```
area-ci (claudine) / claudine-cli / test (ubuntu-latest)
area-ci (claudine) / claudine-cli / wsl2 / test (wsl2-ubuntu)
```

| Workflow | Job id | Label when it runs | Label when skipped whole | Cell it owns |
|---|---|---|---|---|
| ci.yml | `validation` | `Check reusable PR validation` | (never skipped) | none |
| ci.yml | `scope` | `Determine affected scope` | (never skipped) | none |
| ci.yml | `preflight` | `preflight (ubuntu-latest)` | `preflight` | none |
| ci.yml | `build` | `build (ubuntu-latest)` etc. | `build` | none (build record) |
| ci.yml | `area-ci` | `area-ci (claudine)` etc. | `area-ci` | none (scheduling) |
| ci.yml | `area-drift` | `area-drift (planner areas match sniff)` | `area-drift` | none |
| ci.yml | `ci-gate` | `ci-gate` | (always runs) | none |
| ci.yml | `ci-reporting` | `ci-reporting (advisory)` | (always runs) | none |
| _area-ci.yml | `package-ci` | `${{ matrix.package }}` (one per package) | n/a (no guard) | none (delegation) |
| _area-ci.yml | `accepted-gaps` | `accepted-gaps` | `accepted-gaps` | none |
| _area-ci.yml | `coverage-audit` | `coverage-audit` | `coverage-audit` | none (verdict) |
| _package-ci.yml | `check` | `check (windows-latest)` etc. | `check` | check cell |
| _package-ci.yml | `test` | `test (macos-latest)` etc. | `test` | L1 cell |
| _package-ci.yml | `lint` | `lint (ubuntu-latest)` (static `name:`) | — (guard is gates-only) | lint cell |
| _package-ci.yml | `test-l2` | `test-l2 (macos-latest)` etc. | `test-l2` | L2 cell |
| _package-ci.yml | `test-browser` | `test-browser (ubuntu-latest)` etc. | `test-browser` | browser cell |
| _package-ci.yml | `wsl2` | `wsl2` (delegating matrix-less job) | `wsl2` | none (delegation) |
| _wsl-ci.yml | `wsl` | `test (wsl2-ubuntu)` (static `name:`) | — | wsl2 L1 cell |

Notes for the Phase 5 label work:

- `test`/`check`/`test-l2`/`test-browser` take their environment from the
  matrix values (`matrix.environment` / `matrix.os`); `lint` and _wsl-ci's
  `wsl` carry static `name:`s because they run on exactly one environment.
- `runner_loss.py`'s `JOB_KINDS` maps `test`→L1, `test-l2`→L2,
  `test-browser`→browser, `check`→check, `lint`→lint; `GATE_SEGMENT` parses
  `kind (detail)` from the last segment; `wsl2` delegation segments are
  stripped from the tail; `lint`'s environment is presentation-only.
- `build` labels use the producer environment (`build (ubuntu-latest)`);
  `BUILD_JOB_SEGMENT` parses them; a build owns no result cell.

## 2. The contract-test edit set (Phase 2 oracles / Phase 5 promotion)

From `tools/test-toolkit/tests/ci_workflow_contracts.rs`, the tests that
assert an **environment-list input**, a **per-package call/matrix**, the
**`needs: test` staging order**, or a **producer job label**. These are the
tests Phase 5 rewrites; everything else in the file should survive untouched.

### Primary edit set (asserts the shape this feature changes)

| Test | Line | Asserts |
|---|---|---|
| `expensive_tiers_stage_behind_l1_but_lint_never_gates_it` | 381 | `needs: test` ×2 for `test-l2`/`test-browser` (R3 rewrites to the negative) |
| `the_package_matrix_is_scope_derived_not_static` | 431 | per-package `matrix` with `native_environments`/`l2_environments`/`browser_environments`/`node_environments`/`toolchain_environments` |
| `l2_runs_on_every_environment_with_a_provisioned_backend` | 1611 | `l2-environments` input drives the L2 matrix |
| `a_declared_toolchain_requirement_is_provisioned_once_before_the_suite_runs` | 1649 | `toolchain-environments` list gating |
| `a_declared_node_capability_is_provisioned_verified_and_hard_required` | 1697 | `node-environments` list gating |
| `lint_and_check_labels_identify_their_environment` | 4003 | `check-os` matrix + label shape (`check` label from matrix, `lint` static) |
| `the_area_is_the_top_level_identity_of_the_package_fan_out` | 4023 | per-package fan-out incl. `name: ${{ matrix.package }}` |
| `a_reused_cell_reaches_its_area_summary_without_being_re_executed` | 4092 | `native-environments` narrowing on reuse |
| `empty_execution_matrices_skip_before_expansion` | 4119 | the four environment-list inputs' empty-skip guards |
| `an_archive_consumer_verifies_first_and_never_reaches_a_compiler` | 5016 | archive steps + `toolchain-environments` |
| `every_consumer_resolves_its_bound_sidecars_under_their_windows_names` | 5457 | `toolchain-environments` in archive-consumer context |
| `no_test_tier_carries_a_compile_in_place_path` | 5499 | tier jobs incl. `toolchain-environments` references |

### Secondary set (asserts labels/job ids this feature renames or re-shapes)

| Test | Line | Asserts |
|---|---|---|
| `wsl_is_an_environment_and_never_a_runner_label` | 1396 | wsl2 delegation + `test (wsl2-ubuntu)` |
| `artifact_names_carry_the_environment_not_the_runner_label` | 1591 | artifact names vs labels |
| `every_producer_job_emits_an_explicit_status_artifact` | 1849 | producer job ids (`test`, `test-l2`, `test-browser`, `check`, `lint`, wsl) |
| `the_status_fold_preserves_every_failure_shape` | 2280 | status writer per producer kind |
| `no_skippable_job_is_labelled_with_an_unresolved_expression` | 3953 | skippable jobs carry no `name:` with expressions |
| `every_skippable_job_has_a_static_human_readable_identity` | 3971 | static labels for skippable jobs |
| `the_worker_policy_survives_the_area_restructure` | 4139 | worker policy inside the tier jobs |
| `only_recovery_and_diagnostic_steps_ignore_errors` | 2027 | step-level `continue-on-error` in tier jobs |
| `unchanged_dependents_are_compiled_inside_the_changed_packages_linux_check` | 527 | `check` job's dependent compile |
| `the_lint_and_check_gates_are_measured_as_their_own_configurations` | 5580 | per-gate build counters |
| `the_reusable_workflow_chain_stays_within_githubs_four_levels` | 4389 | four-level depth (must stay true) |
| `a_reused_cell_...` / `an_all_reused_area_still_produces_its_result_slice` | 4092 / 6121 | area survival on reuse/gap-only |

The D4 staging assertion (line 381) is the only test Phase 5 **rewrites to the
negative** rather than migrates; every other entry migrates onto the new
interfaces.

## 3. The twelve Python suites' current pass counts

All green on this tree, 2026-09-20 (`python3 scripts/ci/test_<name>.py`):

| Suite | Tests | Result |
|---|---:|---|
| test_affected_scope.py | 251 | OK |
| test_schema.py | 112 | OK |
| test_ci_local.py | 87 | OK |
| test_evidence_reuse.py | 76 | OK |
| test_resolved_plan.py | 75 | OK |
| test_runner_loss.py | 44 | OK |
| test_constraints.py | 39 | OK |
| test_local_evidence.py | 20 | OK |
| test_publish_gaps.py | 19 | OK |
| test_reuse_validation.py | 21 | OK |
| test_cross_check.py | 13 | OK |
| test_build_key.py | 12 | OK |
| **total** | **769** | **12/12 OK** |

(`test_completion.py` does not exist yet; Phase 2 Wave 2 creates it and the
self-test list is spelled in four coupled files — `just/ci-local.just`,
`scripts/ci/test_ci_local.py` twice, `.githooks/tests/test-pre-push.sh`.)

## 4. `affected_scope.py --all` statistics (this tree)

| Quantity | Value |
|---|---:|
| Cells / executing cells | 412 / 374 |
| Areas (plan `areas`) | 31 (29 carry executing cells) |
| Executing gates | L1 270, lint 68, L2 22, check 12, browser 2 |
| Executing environments | ubuntu 161, macos 79, windows 68, wsl2 66 |
| Accepted-gap cells | 38, across 11 areas |
| Build records / build owners | 204 / 3 |
| `job_estimate` | 440 |
| Largest area by executing cells | `homelab` (35) |
| Largest single row set (native test, `homelab`) | **21 rows** |
| Serialized `area_matrix` (all areas) | 79,929 B |
| Largest single area matrix (`homelab`) | 7,908 B |
| Largest per-area row-set payload (new design) | 2,140 B |
| Total serialized scope.json | 323,063 B |

The plan's own table (measured on `fix/archive-path-sites`) reported 85,956 B
`area_matrix` and 8,504 B largest; this tree has moved since — every number
above is the one Phase 3's equality proof compares against.

## 5. The plan corpus (Phase 3's comparison set)

Saved under `spikes/plans/` in this directory; `README.md` there records the
exact commands and inputs that produced each file. All validate against
`schema.validate_resolved_plan` at schema version 4.

| File | Shape | Cells / executing | Areas | Builds | job_estimate |
|---|---|---:|---:|---:|---:|
| `all.json` | `--all`, every environment | 412 / 374 | 31 | 204 | 440 |
| `dispatch.json` | `--all --event workflow_dispatch` | 412 / 374 | 31 | 204 | 440 |
| `nightly.json` | `--all --event schedule` (wsl2-only) | 83 / 66 | 31 | 66 | 132 |
| `pr.json` | `--event pull_request`, one source file (`biscuit-hash/lib/src/lib.rs`) | 4 / 4 | 1 | 2 | 4 |
| `all-reused.json` | `--all` + every executing cell accepted | 412 / 71 | 31 | 3 | 71 |
| `mixed.json` | `--all` + the 66 wsl2 L1 cells accepted | 412 / 308 | 31 | 204 | 308 |
| `prohibited.json` | `--all` + a wsl2-ubuntu constraint store | 412 / 308 | 31 | 204 | 308 |
| `gap-only.json` | derived from `all.json`: gap cells only | 38 / 0 | 11 | 0 | 22 |

Notes the adapter phase must respect:

- **No area is gap-only or fully reused in a real plan today.** Every area
  carries a `lint` cell, and lint has no evidence-reuse eligibility, so every
  area always has at least one executing cell. `gap-only.json` is therefore
  **synthesized** (real gap cells, real area records, packages trimmed to the
  gap-owning packages, `builds` emptied) and validated by the schema; it is
  the only corpus file not produced by a single planner invocation. Its
  recipe is in `plans/README.md`.
- `all-reused.json` still executes 71 cells — the 68 lint cells (never
  reusable) and 3 companion-owning L1 cells (a companion detaches its cell
  from reuse). That is the planner's real behavior, not a fixture defect.
- `nightly.json` carries wsl2 cells only; its Linux producer joins through
  build records, cell-less, exactly as the environment table's `events`
  decide.
