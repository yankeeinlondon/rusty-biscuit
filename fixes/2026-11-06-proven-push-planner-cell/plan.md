---
total_phases: 3
created: 2026-11-08
phase: 4
agent: claude/sonnet
yolo: true
source_files_during_phase_1:
- scripts/ci/affected_scope.py
- scripts/ci/schema.py
- scripts/ci/test_affected_scope.py
- tools/test-toolkit/tests/ci_workflow_contracts.rs
- .github/workflows/_package-ci.yml
- .github/workflows/_wsl-ci.yml
docs_updated_during_phase_1: []
docs_created_during_phase_1: []
skills_files_updated_during_phase_1: []
packages:
- repo-deps
- test-toolkit
source_files_during_phase_2:
- scripts/ci/schema.py
docs_updated_during_phase_2:
- CLAUDE.md
- docs/topics/ci-cd.md
docs_created_during_phase_2: []
skills_files_updated_during_phase_2:
- .claude/skills/rust-devops/ci-cd.md
source_files_during_phase_3: []
docs_updated_during_phase_3: []
docs_created_during_phase_3: []
skills_files_updated_during_phase_3: []
source_code:
- scripts/ci/affected_scope.py
- scripts/ci/schema.py
- scripts/ci/test_affected_scope.py
- tools/test-toolkit/tests/ci_workflow_contracts.rs
- .github/workflows/_package-ci.yml
- .github/workflows/_wsl-ci.yml
documentation:
- CLAUDE.md
- docs/topics/ci-cd.md
- .claude/skills/rust-devops/ci-cd.md
completed_phase: 3
implemented: true
---

# Plan: a proven merge must not plan a cell for an environment it removed

Spec: `2026-11-06-proven-push-planner-cell`.

## Summary and Definition of Done

Seven of the eight latest failing pushes to `main` died in `Resolve this cell's execution contract` with
`cell-contract-mismatch: the plan knows no environment 'ubuntu-latest'`. Cause: `package_cells`
(`scripts/ci/affected_scope.py:3220`) has an `elif seam:` branch (around line 3388) that adds a dependents-only
`check` cell on `DEPENDENTS_ENVIRONMENT` unconditionally. `schedule_environments` had already moved
`ubuntu-latest` to `proven_environments`, so the plan carries a Linux cell with no Linux environment.

The work has three parts, one per phase:

1. **Planner and validator (Python).** Guard the dependents-only check on the scheduled environments. Make
   `schema.validate_resolved_plan` refuse any cell whose environment is not a scheduled cell-bearing
   environment, and run that validation before `calculate_scope` returns, so `--plan-out` and stdout are never
   written for an invalid plan.
2. **Workflow upload guards.** Skip every upload whose name comes from contract outputs when resolution failed
   and the output is empty, in `_package-ci.yml` (check, test, lint jobs) and `_wsl-ci.yml` (wsl job). Pin the
   guards in the toolkit contract tests.
3. **Docs and verification.** Qualify the "a dependent check always runs" wording in `CLAUDE.md`,
   `docs/topics/ci-cd.md`, and the `rust-devops` skill, then run every focused suite.

Definition of done:

- Replaying the `a816f1447` shape (a changed library, an unchanged direct consumer, no example/bench targets)
  as `event="push", proven_event="pull_request"` yields no cells, builds, rows, scheduled areas, or preflight
  OS; the same tree as a pull request or an unproven push yields exactly one Linux dependents check.
- A cell naming an absent or producer-only environment is refused by `validate_resolved_plan` with a
  `malformed-receipt:` problem, including through `apply_accepted_cells`. A legitimate producer-only plan
  (nightly, Linux retained for WSL2 archives) still validates.
- The planner CLI exits nonzero for an invalid calculated plan and writes neither `--plan-out` nor stdout.
- The four producer jobs skip contract-derived uploads when the contract did not resolve.
- `test_affected_scope.py`, `test_schema.py`, `test_resolved_plan.py` pass through `suite_runner.py`; so do
  the toolkit workflow contract tests; `actionlint` is clean when installed.
- No doc says a dependent check runs regardless of event scheduling.
- Resolved-plan schema version stays 7. No commit is made by the implementation.

## Phase 1: Planner and validator

### Necessary Rules

- **R1. The guard goes in `package_cells`, keyed on the passed `environments` list.** The `elif seam:` branch
  becomes `elif seam and any(environment["name"] == DEPENDENTS_ENVIRONMENT for environment in environments):`.
  Evidence: `calculate_scope` passes `cell_environments` (the scheduled list, with proven, deferred and
  producer-only environments already removed) at the call near `affected_scope.py:4477`, and the second caller
  near 5265 passes the same kind of list. The `if unchecked:` branch already respects it through
  `native_environments(environments)` plus `name != CHECK_ENVIRONMENT`.
- **R2. Combined check stays one cell.** The `if unchecked:` branch already emits at most one check per
  environment and sets `compiled_dependents=seam if hosts_seam else ()`. It needs no edit. `elif` guarantees
  the dependents-only cell never duplicates it. Keep `reusable=False` on the dependents-only cell (spec reader's
  note).
- **R3. Do not touch `schedule_environments`, `producing_environments`, or the proven-event verifier.** The
  spec forbids re-adding Linux to a proven push or weakening proof. A producer-only Linux is not in
  `cell_environments`, so R1 already keeps it from enabling the check (evidence:
  `producing_environments` returns only environments absent from `scheduled`).
- **R4. Docstring drift fix in `package_cells`.** The `dependents` paragraph says the check cell exists "even
  when no target kind of its own needs one". Reword it to say the dependents-only cell exists only when the
  event schedules `DEPENDENTS_ENVIRONMENT`. Also change the comment on the `elif` branch accordingly. Code is
  authoritative.
- **R5. New validator rule lives inside `validate_resolved_plan` (`scripts/ci/schema.py`, after the
  `producing_environments` block, around line 1900).** It needs `scheduled` (all names in the plan's
  `environments`, already computed there) and a new set `producer_only` = names of `producing_environments`
  entries that are dicts with a string `name`. A cell's allowed environments = `scheduled - producer_only`.
  Walk every dict cell in `document["cells"]` that has string `package`, `environment` and `gate`
  (`reused`, `omitted`, accepted-gap and prohibited cells included, because the check is state-blind). Emit
  one problem per cell:
  - absent: `malformed-receipt: cell {package}/{environment}/{gate} names environment {environment!r}, which the plan's environment table does not carry`
  - producer-only: `malformed-receipt: cell {package}/{environment}/{gate} names environment {environment!r}, which the plan keeps only as a producer of archives for {guests}; a producer owns no cells`
  - `guests` is the producer's `for` list joined with `, ` when it is a list of strings.

  Evidence: the spec requires `malformed-receipt` and a diagnostic naming package, environment and gate with
  absent/producer-only distinguished.
- **R6. Existing `unknown-environment` problem for an environment outside `ENVIRONMENTS` stays.** A cell naming
  a wholly unknown name therefore yields both the existing `unknown-environment:` problem and the new absent
  problem. Acceptable: both are true and tests assert with `any(...)`.
- **R7. Validation runs inside `calculate_scope` at its end.** Replace the tail
  `plan["rows"] = attach_rows(plan); return plan` with: after `attach_rows`, call
  `problems = schema.validate_resolved_plan(plan)`; if `problems`, `raise RuntimeError("the calculated plan is invalid: " + problems[0])`
  (first problem only, matching `apply_accepted_cells`' message style at line ~4850); then return. Evidence:
  `main()` calls `calculate_scope` before `--plan-out` write (line ~6054) and before `print`, and nothing
  catches `RuntimeError` in `main`, so the uncaught exception exits nonzero (traceback on stderr) with no
  publication. All other planner CLI error paths (`validate_no_shadow_workspaces`, unknown events) behave the
  same way, so this is the established convention; no `main()` change.
  Cost risk: validating the full workspace plan once per planner run is acceptable (apply path already does it).
- **R8. `apply_accepted_cells` needs no code change.** It already calls `schema.validate_resolved_plan(plan)` at
  entry (line ~4850), so R5 covers "carried plans entering the evidence overlay". The spec's test row is
  satisfied by a test, not code.
- **R9. `cell_contract.py` stays unchanged.** Its `cell-contract-mismatch` refusal (line ~149) is the retained
  defense for external or older plans.
- **R10. Schema version stays 7** (`RESOLVED_PLAN_SCHEMA_VERSION`). No field added; no `docs/cicd/schema-versions.md`
  edit.
- **R11. Test placement and fixtures.**
  - Planner regressions go in `EventSchedulingTests` in `scripts/ci/test_affected_scope.py` (line ~2883) as a
    new fixture class `DependentCheckSchedulingTests` placed directly after it, copying its `setUp` shape but with
    two packages: `alpha-core` (`targets=["lib"]`, manifest `alpha/lib/Cargo.toml`) and `beta-app`
    (`targets=["bin"]`, `beta/app/Cargo.toml`), then `declare_dependencies(self.metadata, {"beta-app": ["alpha-core"]})`
    (helper at line 216). Reuse `EventSchedulingTests.shipped_events()` by calling it as a static method. The
    changed file is `alpha/lib/src/lib.rs`. Neither package has example or bench targets, so `uncovered_target_kinds`
    is empty and the dependents-only branch is the one exercised. A second variant gives `alpha-core`
    `targets=["lib", "example"]` for the combined-check row. No network, no git history.
  - Validator, overlay and CLI tests all go in that same class (see R12); `test_schema.py` and
    `test_resolved_plan.py` are regression runs only.
- **R12. Where the validator tests get a valid plan.** `test_schema.py` has no planner dependency, so the
  validator matrix lives in `test_affected_scope.py` beside the planner fixture
  (`DependentCheckSchedulingTests`), built from a plan the planner just calculated (an
  `event="schedule"`, `force_all=True` plan for the producer-only case, mirroring
  `test_a_full_scope_run_preflights_only_the_scheduled_runners`, and a pull-request plan for the control case).
  One small addition to `test_schema.py` is not made. Evidence: `test_affected_scope.py:3125-3200` already
  tests `schema.validate_resolved_plan` against hand-edited planner plans (`test_the_schema_refuses_a_deferred_environment_the_plan_also_schedules`).
  The "run `test_schema.py`" step is a regression run only.
- **R13. Input robustness matrix: not applicable.** The change adds a relational check over an already-validated
  list, not a new parser or loader. The default outcome stays "located `malformed-receipt:` problem, nothing
  published". The only shapes covered are the two the spec names (absent, producer-only) plus a control and a
  legitimate producer-only plan.
- **R14. CLI refusal test mechanism.** Add one test to `DependentCheckSchedulingTests` that runs
  `affected_scope.main()` in-process (no subprocess, no `cargo metadata`, no network) under `mock.patch.object`:
  `sys.argv = ["affected_scope.py", "--resolved-plan", "--event", "pull_request", "--plan-out", <tmp>/plan.json, "alpha/lib/src/lib.rs"]`
  (the implementer reads `parse_args` near line 5790 for the exact flag and positional names and adjusts the
  spelling only),
  `affected_scope.ROOT = self.root`, `load_metadata -> self.metadata`, `load_environments -> self.shipped_events()`,
  `package_ci_policy -> self.policy`, `workspace_packages -> {}`, `validate_no_shadow_workspaces -> None`,
  and `package_cells` wrapped so every cell it returns has `environment` rewritten to `"windows-latest"`
  (an environment the pull request event defers, so absent from the plan). Assert: `RuntimeError` is raised with `"the calculated plan is invalid"`
  in its message, `plan.json` does not exist, and captured stdout (`contextlib.redirect_stdout`) is empty.
  Evidence: `test_affected_scope.py` already patches module-level planner functions elsewhere and builds the
  synthetic root with `seed_build_inputs`.
- **R15. Workflow guard form.** Contract-derived upload steps whose names can be empty after a failed
  resolution get the guard `steps.cell.outputs.<name> != ''` ANDed into their existing `if`. Exact edits:
  - `Upload producer status` (4 steps): `if: ${{ always() && steps.cell.outputs.status_artifact != '' }}`
    in `_package-ci.yml` check (~line 344), test (~1018), lint (~1470) and `_wsl-ci.yml` (~845).
  - `Upload JUnit` in `_package-ci.yml` test (~930): `if: ${{ !cancelled() && steps.cell.outputs.junit_artifact != '' }}`.
  - `Upload wsl2-ubuntu L1 JUnit` in `_wsl-ci.yml` (~663): same form with `junit_artifact`.
  - `Upload this cell's observation` in `_wsl-ci.yml` (~915): `if: ${{ !cancelled() && github.event_name == 'schedule' && steps.cell.outputs.package != '' }}`
    (its name embeds `package` and `gate`, both emitted together by the contract).
  - Not edited, with reason: the two `Upload this cell's completion record` steps and every other
    `completion_artifact` upload have no `always()`/`!cancelled()`, so a failed resolution already skips them
    ("no completion record is created" stays true by default `success()`). `Upload WSL2 diagnostics` is gated on
    `steps.l1.outcome == 'failure' || steps.verified.outcome == 'failure'`; those steps are skipped, never
    failed, when resolution failed, so it cannot run. Matrix-named uploads (`measurement-*`, the native
    observation) use `matrix.*`, never contract outputs, so they are never empty. Do not set
    `continue-on-error` or fallback names anywhere.
- **R16. Contract tests.** Add one test to `tools/test-toolkit/tests/ci_workflow_contracts.rs` named
  `uploads_named_by_the_cell_contract_skip_when_it_failed_to_resolve`. It uses `workflow()`/`jobs()`/`job_block()`
  to assert, for `_package-ci.yml` jobs `  check:`, `  test:`, `  lint:` and `_wsl-ci.yml` job `  wsl:`, that the
  `Upload producer status` step's `if:` line contains both `always()` and `steps.cell.outputs.status_artifact != ''`;
  for the JUnit steps and the WSL observation step the guards from R15. It also asserts the status
  artifact upload still reads `name: ${{ steps.cell.outputs.status_artifact }}` (the existing
  `retired_packages_reach_verdict_through_package_evidence` test at line ~1287 matches that exact text, so the
  `name:` lines must not change). Extract steps by splitting each job block on `      - name:` and finding the
  step whose text contains the step name.
- **R17. Out of scope and decided:** the Oct 26 `coverage-audit` failure, the Rust planner port, and any
  change to `.github/ci/environments.json`.

### Wave 1 (sequential; the later waves depend on it)

- [x] **Guard dependents check**
    - Apply R1, R2, R4 to `package_cells` in `scripts/ci/affected_scope.py`.
    - Apply the guard only inside `package_cells`; the function owns the invariant, so the caller near
      line 5265 needs no edit.
- [x] **Validator rule**
    - Implement R5 and R6 in `scripts/ci/schema.py` `validate_resolved_plan`. Update that function's docstring
      with one sentence naming the cell-environment relation.
    - Prerequisite: none. Risk: a plan whose `producing_environments` is malformed (non-list, non-dict
      entries). Build `producer_only` defensively with `isinstance` checks; the existing producing-environments
      block already reports those malformations, so this rule must not raise.
- [x] **Validate at calculation**
    - Implement R7 at the end of `calculate_scope`. Update `calculate_scope`'s docstring line saying
      `schema.validate_resolved_plan` "is its contract" to say the plan is validated before being returned.
- [x] **Run existing suites for fallout**
    - `python3 scripts/ci/suite_runner.py test_affected_scope.py`, `test_schema.py`, `test_resolved_plan.py`
      from the repo root as the spec names. R7 means every test that calls `calculate_scope` now validates;
      any failure from a fixture plan that was previously invalid is a test fixture defect to fix in the
      fixture (correct the fixture's environment/producer data), never a reason to weaken the validator.

### Wave 2 (parallel after Wave 1; edits are in separate test classes/files)

- [x] **Planner regression tests**
    - Create `DependentCheckSchedulingTests` per R11 in `scripts/ci/test_affected_scope.py` with these tests,
      each asserting through `calculate_scope` results:
      1. proven push (`event="push", proven_event="pull_request"`): `cells`, `builds`, `preflight_os` empty,
         `legacy_scope_document(plan)["scheduled_areas"] == []`, `plan["proven_environments"]` lists Linux and
         macOS, `reverse_dependencies` still names `beta-app`, and `validate_resolved_plan(plan) == []`.
      2. pull request and unproven push: exactly one cell `(alpha-core, ubuntu-latest, check)` with
         `dependents == ["beta-app"]`, `reusable is False`; no cell and no `packages` gate entry for `beta-app`;
         no `beta-app` in `legacy_scope_document(plan)["scheduled_areas"]` packages.
      3. combined: `alpha-core` with an `example` target and the same edge gives one check cell on
         `ubuntu-latest` with `dependents` retained and `target_kinds` containing `example`; with
         `proven_event` set it gives none.
      4. nightly (`event="schedule"`, `force_all=True` is not used because that drops the seam; use
         `event="schedule"` with the changed file): `ubuntu-latest` is in `producing_environments`, no cell has
         `environment == "ubuntu-latest"`, the Windows and WSL2 cells and the Linux build record remain.
         Note `force_all` attributes no dependents (existing `test_a_full_scope_run_attributes_no_dependents`),
         so the dependents case must use a changed-file diff.
      5. local L1 evidence: pass `accepted_cells` with a passing `(alpha-core, ubuntu-latest, L1)` result in the
         shape `apply_accepted_cells` consumes (copy the shape from `ApplyFixture` in the same file near
         line 3800); the `check` cell must still be `execution == "execute"`, `reusable is False`, `state == "pending"`.
- [x] **Validator tests**
    - In the same new class add (per R12): control plan validates `[]`; a cell edited to an environment absent
      from the plan (pull-request plan, cell environment `windows-latest`) yields a problem starting
      `malformed-receipt:` containing the package, the environment and the gate and "does not carry"; a cell
      edited to the producer-only `ubuntu-latest` on the nightly plan yields a problem containing "producer" ;
      the unedited nightly plan validates `[]` (legitimate producer-only build records); the edited absent-env
      plan passed to `apply_accepted_cells(plan, [], [])` raises `RuntimeError` mentioning `malformed-receipt`;
      a reused cell (use `mark_reused` through an accepted-cell plan) renamed to an absent environment is also
      refused (state-blind).
- [x] **CLI refusal test**
    - Add the R14 test to the same class.
- [x] **Workflow upload guards**
    - Apply R15 to `.github/workflows/_package-ci.yml` and `.github/workflows/_wsl-ci.yml`; add the R16 test
      to `tools/test-toolkit/tests/ci_workflow_contracts.rs`.

### Phase 1 validation checkpoint

- [x] **Python suites**: from the repo root run `python3 scripts/ci/suite_runner.py test_affected_scope.py`,
      `... test_schema.py`, `... test_resolved_plan.py`. There is no separate cell-contract suite; its failure
      tests live in `test_resolved_plan.py` (`assert_refused("cell-contract-mismatch", ...)` near line 2278).
- [x] **Toolkit contracts**: `just test test-toolkit` from the repo root (nextest recipe); expect the new test
      and `retired_packages_reach_verdict_through_package_evidence` to pass.
- [x] **actionlint**: run `command -v actionlint`; when present run `actionlint .github/workflows/_package-ci.yml .github/workflows/_wsl-ci.yml`
      and require a clean exit. When absent on this macOS host, do not install anything; record "actionlint
      unavailable" in the implementation log.
- [x] **Replay evidence**: run `python3 scripts/ci/affected_scope.py --help` to read real flag names, then write
      one sentence in the log that the proven-push fixture test reproduces the `messenger`/`test-toolkit`
      failure shape before the fix (confirm by stashing the R1 edit once: test 1 must fail without it).

## Phase 2: Documentation drift

### Wave 3 (parallel; independent files)

- [x] **CLAUDE.md wording**
    - In `CLAUDE.md` ("Compile coverage is per target kind" bullet, ends "a check cell that compiles dependents
      always runs."), change the final clause to: "a *scheduled* check cell that compiles dependents always
      runs, because no local L1 receipt compiles the unchanged consumers; an environment the event does not
      schedule (a push proven by its pull request) plans no such cell." `AGENTS.md` is a symlink; edit only
      `CLAUDE.md`.
- [x] **ci-cd topic**
    - In `docs/topics/ci-cd.md` (line ~122) replace "A check that compiles unchanged dependents always
      executes." with: "A scheduled check that compiles unchanged dependents always executes and never reuses
      a local receipt. A push whose pull request already proved Linux plans no Linux cells at all, dependents
      check included, and the planner refuses any plan whose cell names an environment it did not schedule."
      Add the rule for the reader (a short example: changed lib, unchanged consumer, push with proof → no cells)
      in one or two sentences. The page must not link to or name this fix.
- [x] **rust-devops skill**
    - In `.claude/skills/rust-devops/ci-cd.md` (~lines 98-109) amend "the cell that compiles unchanged
      dependents is never reusable" to add that the cell exists only when the event schedules
      `DEPENDENTS_ENVIRONMENT`, and add one bullet that `validate_resolved_plan` refuses a cell naming an
      environment outside the plan's cell-bearing environments (absent, or a producer-only entry). Also add to
      the same file's workflow-guard guidance one sentence: uploads named by `steps.cell.outputs.*` are guarded
      with a nonempty check so a refused contract shows only its own error.
- [x] **Comment drift pass**
    - Re-read the docstrings and comments of `package_cells`, `calculate_scope`, `validate_resolved_plan`,
      and the edited workflow steps; fix any remaining "always"/"even when no target kind needs one" wording
      (R4 covers `package_cells`). Run `grep -rn "always runs\|always executes\|even when no target kind" CLAUDE.md docs .claude/skills/rust-devops scripts/ci/affected_scope.py`
      and resolve each hit that describes the dependents check as unconditional; hits about `lint always runs`
      are correct and stay.

## Phase 3: Final verification and handoff

### Wave 4 (sequential)

- [x] **Full focused rerun**
    - Rerun the three Python suites through `suite_runner.py`, `just test test-toolkit`, and `actionlint`
      (same rules as the Phase 1 checkpoint). All must pass.
- [x] **Lint**
    - `just lint test-toolkit` from the repo root. Do not run `cargo fmt`.
- [x] **Spec hygiene**
    - Set `implemented: true` and `implemented_by: claude/sonnet` in `spec.md` frontmatter and
      `status: implemented`. Do not move the spec to `_completed` and do not run `just complete`. Do not commit.
- [x] **Implementation log**
    - Append `## Implementation log` to this plan listing: the files changed, any departure from the spec
      (none expected), whether `actionlint` was available, and the pre-fix reproduction result.
## Implementation log

Full log: `implementation-log.md` in this directory.

- **Files changed (source):** `scripts/ci/affected_scope.py`, `scripts/ci/schema.py`,
  `scripts/ci/test_affected_scope.py`, `tools/test-toolkit/tests/ci_workflow_contracts.rs`,
  `.github/workflows/_package-ci.yml`, `.github/workflows/_wsl-ci.yml`.
- **Files changed (docs and skill):** `CLAUDE.md`, `docs/topics/ci-cd.md`, `.claude/skills/rust-devops/ci-cd.md`.
- **Departures from the spec:** none in behavior. Additions beyond the plan are listed in the log's
  `decisions_for_review` (three pinned contract assertions updated, an upload sweep in the new contract
  test, WHY comments over the guarded status uploads, a drifted `dependent_seam` schema comment
  reworded, and `implemented_by` recorded as `claude/opus` instead of the plan's `claude/sonnet`).
- **actionlint:** unavailable on this macOS host in every phase; nothing was installed.
- **Pre-fix reproduction:** with the `package_cells` guard reverted, the proven-push fixture test failed with
  `the calculated plan is invalid: malformed-receipt: cell alpha-core/ubuntu-latest/check names environment
  'ubuntu-latest', which the plan's environment table does not carry`, the `a816f1447` shape; 4 of the 10
  new tests failed without the guard and all pass with it.