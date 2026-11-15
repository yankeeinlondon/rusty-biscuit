---
spec: "/Volumes/coding/rusty-biscuit/fixes/2026-11-06-proven-push-planner-cell/spec.md"
plan: "fixes/2026-11-06-proven-push-planner-cell/plan.md"
implemented_by: "claude/opus"
started_phase: "1"
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
decisions_for_review:
  - |-
    Three existing workflow contract tests pinned the old upload conditions
    as exact text (every_producer_job_emits_an_explicit_status_artifact,
    every_producer_uploads_a_completion_artifact,
    every_hosted_producer_records_and_uploads_its_cell_on_the_nightly_only).
    The plan did not mention them. I updated each to expect the new guarded
    condition rather than loosening them to a prefix match, so they still pin
    the exact expression. Alternative: match only the leading "always()" or
    "!cancelled()" part, which would tolerate future guard changes but pin
    less. Redo if reversed: edit those three assertions in
    tools/test-toolkit/tests/ci_workflow_contracts.rs.
  - |-
    The new contract test also sweeps every upload step in both workflow
    files and fails if one is named from the cell contract's outputs, runs
    after a failure, and has no nonempty check. The plan asked only for the
    seven named steps. I added the sweep so a future upload cannot repeat the
    defect. It skips the WSL2 diagnostics upload because that step runs only
    when a later step failed, and those later steps are skipped, never
    failed, when the contract did not resolve. Alternative: pin only the
    seven steps (simpler, misses new uploads). Redo if reversed: delete the
    sweep loop from that test.
  - |-
    The CLI refusal test does not patch workspace_packages as the plan's R14
    listed, because calculate_scope itself calls workspace_packages on the
    same metadata, and returning an empty map would break planning before
    the validator ran. It also misplaces only the check cell rather than
    every cell: moving the L1 cells too makes build-producer lookup fail
    first, with a different error. The test asserts the exact validator
    message, so it cannot pass through another failure. No redo needed
    unless the reviewer wants a different defect injected.
  - |-
    I added a three-line comment above each guarded "Upload producer status"
    step explaining why it skips when the contract did not resolve. The
    check job's existing comment says "always(), so a FAILED job still
    reports itself", which would otherwise read as contradicting the new
    guard. Alternative: no comment (the plan did not ask for one). Redo:
    delete the four comments.
  - |-
    Comment drift outside the plan's list: the schema comment on the package
    record's dependent_seam field said the seam's dependents are "compiled
    inside this package's own ubuntu-latest check cell". On a push its pull
    request proved, the record still carries the seam (it reports which
    consumers would be compiled) but no check cell exists. I reworded the
    comment to say the field is present even when no such cell is planned,
    and left the behavior alone. Alternative: drop dependent_seam from the
    record when the event schedules no ubuntu-latest (a behavior change the
    spec did not ask for, which would also affect the reporting of unchanged
    dependents). Redo if reversed: revert the comment in scripts/ci/schema.py
    and change the record assembly in calculate_scope.
  - |-
    Phase 3 recorded implemented_by as "claude/opus" on the spec, because
    the run's instructions asked for that value; the plan's spec-hygiene
    task said "claude/sonnet". The value only names the agent and changes no
    behavior. Redo if reversed: change implemented_by in spec.md.
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

# Implementation Log for 2026-11-06-proven-push-planner-cell (3 phases)

## Phase 1

### Wave 1: planner and validator

- `scripts/ci/affected_scope.py` `package_cells`: the dependents-only check
  branch is now `elif seam and any(environment["name"] == DEPENDENTS_ENVIRONMENT ...)`
  (R1). The combined `if unchecked:` branch is unchanged (R2). The docstring's
  `dependents` paragraph and the branch comment now say the cell exists only
  when the event schedules `DEPENDENTS_ENVIRONMENT` (R4, comment drift fixed:
  the old text said the cell exists "even when no target kind of its own needs
  one", unconditionally).
- `scripts/ci/schema.py` `validate_resolved_plan`: new state-blind rule after
  the `producing_environments` block. Every cell with string
  `package`/`environment`/`gate` must name an environment in the plan's
  `environments` that is not a producer. Two diagnostics, both
  `malformed-receipt:` and naming `package/environment/gate`: "...which the
  plan's environment table does not carry" (absent) and "...which the plan
  keeps only as a producer of archives for {guests}; a producer owns no cells"
  (producer-only). Built defensively; a malformed `producing_environments`
  is still reported by the existing block and never raises here. Docstring
  gained one sentence naming the relation. The existing
  `unknown-environment` member check is kept (R6).
- `calculate_scope` now validates the plan after `attach_rows` and raises
  `RuntimeError("the calculated plan is invalid: <first problem>")` (R7).
  `main()` is unchanged: the exception propagates before `--plan-out` is
  written or stdout is printed. Docstring updated.
- `apply_accepted_cells` and `cell_contract.py` unchanged (R8, R9). Schema
  version stays 7 (R10).

### Wave 2: tests and workflow guards

- New `DependentCheckSchedulingTests` in `scripts/ci/test_affected_scope.py`
  (directly after `EventSchedulingTests`; 10 tests, all L1, in-process, no
  network/git history). Fixture: `alpha-core` (`lib`) with unchanged consumer
  `beta-app` (`bin`); changed file `alpha/lib/src/lib.rs`.
- Workflow guards (R15) applied to the seven steps the plan lists. I audited
  every `actions/upload-artifact` step in both workflows: the list in R15 is
  complete. Completion-record uploads (default `success()`), the WSL2
  diagnostics upload (gated on a later step's `outcome == 'failure'`), and
  matrix-named uploads stay unguarded. Added a three-line WHY comment above
  each guarded `Upload producer status` step, because the check job's comment
  block says "`always()`, so a FAILED job still reports itself" and the upload
  now deliberately skips when the contract never resolved.
- New contract test
  `uploads_named_by_the_cell_contract_skip_when_it_failed_to_resolve` in
  `tools/test-toolkit/tests/ci_workflow_contracts.rs` (autotests on, so the
  file's binary compiles it; L1 name). It pins the seven guards, pins that the
  status upload keeps `name: ${{ steps.cell.outputs.status_artifact }}`, and
  sweeps every upload in both files so a future contract-named upload that runs
  after failure without a nonempty guard fails. Negative check: removing the
  WSL JUnit guard made both the pinned and sweep halves fail; restored.

### Requirement-to-test mapping

| Spec case | Test |
| --- | --- |
| Proven push: no cells, builds, rows, scheduled areas, preflight; Linux+macOS proven; facts kept | `test_a_push_its_pull_request_proved_plans_no_dependents_check` |
| Pull request / unproven push: one Linux dependents check, consumer gets no cell or area | `test_an_unproven_event_plans_exactly_one_linux_dependents_check` (subtests for both events) |
| Example target: one combined check with dependents; none when proven | `test_an_example_target_folds_the_dependents_into_one_check` |
| Nightly with Linux as producer only: no Linux cell, builds and Windows/WSL2 cells remain | `test_a_nightly_keeping_linux_only_as_a_producer_plans_no_linux_cell` |
| Accepted local L1 evidence: check still executes, non-reusable (planner and overlay paths) | `test_local_l1_evidence_leaves_the_dependents_check_executing` |
| Control: legitimate plans, incl. producer-only build records, validate | `test_the_validator_accepts_the_planned_documents` |
| Absent environment refused, incl. via `apply_accepted_cells` | `test_the_validator_refuses_a_cell_on_an_environment_the_plan_lacks` |
| Producer-only environment refused, distinguished from absent | `test_the_validator_refuses_a_cell_on_a_producer_only_environment` |
| Reused cell refused too (state-blind) | `test_the_validator_refuses_a_reused_cell_on_an_absent_environment` |
| CLI writes neither `--plan-out` nor stdout for an invalid calculated plan | `test_the_cli_publishes_nothing_for_an_invalid_calculated_plan` |
| Contract-derived uploads skip when resolution failed; status keeps its name after successful resolution | `uploads_named_by_the_cell_contract_skip_when_it_failed_to_resolve`, `retired_packages_reach_verdict_through_package_evidence` |

The "gate fails after successful resolution" row is covered by the guard form:
the outputs are nonempty once resolution succeeded, so `always()` still holds
and the status uploads under its existing name; the contract test pins both
halves of the condition and the unchanged name.

### Replay evidence

Real flags confirmed with `affected_scope.py --help` (`--event`,
`--proven-event`, `--resolved-plan`, `--plan-out`). With the R1 guard
temporarily reverted, the proven-push fixture test failed with
`the calculated plan is invalid: malformed-receipt: cell alpha-core/ubuntu-latest/check names environment 'ubuntu-latest', which the plan's environment table does not carry`,
the same shape as the `messenger`/`test-toolkit` cells of `a816f1447`. The
nightly fixture test also failed without the guard (the dependents check named
the producer-only Linux), confirming the spec's note that the unguarded branch
affected other events too. 4 of 10 new tests failed without the guard; all 10
pass with it. Guard restored and verified.

### actionlint

actionlint unavailable: `command -v actionlint` found nothing on this macOS
host; nothing was installed.

### Local OS rigs

No local OS testing rigs are configured (`BUILD_LINUX`, `BUILD_WIN`,
`BUILD_WSL`, `BUILD_MACOS` unset). The change is Python plus workflow YAML
and test text; only macOS was exercised here.

### Gates

- Scope calculator (`git status --porcelain --untracked-files=all | awk ... | xargs python3 scripts/ci/affected_scope.py`):
  `packages: ["repo-deps", "test-toolkit"]`, `change_class: package`. Gates run, not skipped.
- `python3 scripts/ci/suite_runner.py test_affected_scope.py`: 478 OK (run with
  final production code, before the new class was added; the new class's 10
  tests pass on their own under unittest). A final run including the new class
  is recorded below.
- `test_schema.py`: 203 OK. `test_resolved_plan.py`: 103 OK (it holds the
  existing `cell-contract-mismatch` refusal tests; they pass unchanged).
- `just test test-toolkit`: first run had 3 failures, all existing contract
  tests that pinned the old `if:` text exactly
  (`every_producer_job_emits_an_explicit_status_artifact`,
  `every_producer_uploads_a_completion_artifact`,
  `every_hosted_producer_records_and_uploads_its_cell_on_the_nightly_only`).
  I updated them to pin the new guarded expressions, and refactored my new
  test to reuse the file's existing `steps`/`step_named`/`step_name` helpers.
  Rerun: 484 passed, 4 skipped.
- Lint: test-toolkit has no area `lint` recipe (`just lint test-toolkit` is not
  a recipe), so I ran `cargo clippy -p test-toolkit --all-targets -- -D warnings`: clean.
- `just test repo-deps` (Rust L1): 507 passed, 1 skipped.
- repo-deps companion suites that use the changed functions: `test_completion.py`
  75 OK, `test_cross_check.py` 55 OK, `test_reuse_validation.py` 47 OK,
  `test_ci_local.py` 104 OK. `test_local_evidence.py`, `test_evidence_reuse.py`,
  `test_hosted_observations.py` failed every test in setUp with fixture
  `git commit` exit 128: this host's global git signing key is expired
  (`gpg: ... Unusable secret key`, `KEYEXPIRED`) and those fixtures do not set
  `commit.gpgsign=false`. This is a host problem and predates this change. Rerun
  with environment-scoped `commit.gpgsign=false` (temp fixture repos only):
  23 / 44 / 180 OK.
- An outside process (not this session) staged the whole working tree,
  including spec files of other features. I left the index as it was.

- Final `suite_runner.py test_affected_scope.py` including the new class: 488 OK (478 + 10 new).

## Phase 2

- Edited `CLAUDE.md` ("Compile coverage is per target kind" bullet): a
  *scheduled* check cell that compiles dependents always runs; an environment
  the event does not schedule (a push proven by its pull request) plans no
  such cell. `AGENTS.md` is a symlink and was not touched.
- Edited `docs/topics/ci-cd.md`: replaced "A check that compiles unchanged
  dependents always executes." with the scheduled-only rule, the planner
  refusal, and a one-sentence example (changed library + unchanged consumer:
  one Linux dependents check on the pull request, no cells on the proven
  push). The page names no fix.
- Edited `.claude/skills/rust-devops/ci-cd.md`: the dependents-only cell exists
  only when the event schedules `DEPENDENTS_ENVIRONMENT`; new bullet on
  `validate_resolved_plan` refusing a cell outside the cell-bearing
  environments and `calculate_scope` validating before the CLI writes; one
  sentence in the cell-contract paragraph that uploads named by
  `steps.cell.outputs.*` carry a `!= ''` check.
- Comment drift pass: `grep -rn "always runs\|always executes\|even when no
  target kind" CLAUDE.md docs .claude/skills/rust-devops scripts/ci/affected_scope.py`.
  Remaining hits are correct and unrelated (`lint always runs`; a producer
  "always executes what it compiles"; "a gating package always runs L1").
  `package_cells` docstring/branch comment were already right (Phase 1).
  `validate_resolved_plan` docstring already states the rule. The workflow
  check-job comment ("on `ubuntu-latest` alone for a package whose only
  reason is its dependents") describes the row the planner gives it and is
  still accurate. Found and fixed one drifted comment: the `dependent_seam`
  field comment in `scripts/ci/schema.py` (see decisions_for_review).
- Scope check: `git status --porcelain --untracked-files=all | awk '{print $NF}' | xargs python3 scripts/ci/affected_scope.py`
  → `packages: ['repo-deps', 'test-toolkit', 'darkmatter']`, `change_class: package`
  (the tree still holds Phase 1's uncommitted changes). Gates were run.
- Gates:
    - darkmatter, narrowed to the two tests the plan's cell names
      (`shipped_schema_corpus::shipped_markdown_inline_schemas_parse_passively`,
      `shipped_schema_corpus::shipped_markdown_named_type_references_resolve`): 2 passed.
    - `suite_runner.py test_schema.py`: 203 OK; `test_resolved_plan.py`: 103 OK.
    - `just test test-toolkit`: 484 passed, 4 skipped.
    - `cargo clippy -p test-toolkit --all-targets -- -D warnings`: clean (exit 0;
      only a cargo "failed to auto-clean cache data" registry permission warning).
    - `test_affected_scope.py` not rerun: `affected_scope.py` was not changed this
      phase, and Phase 3 reruns it.
    - actionlint unavailable on this host.
- Requirement-to-test mapping: this phase changes documentation and one
  comment, no behavior; no new tests. The documented behavior is pinned by
  Phase 1's tests (proven-push fixture class in `test_affected_scope.py`,
  validator cases, CLI refusal test, workflow upload-guard contract test).
- No local OS test rigs (BUILD_LINUX, BUILD_WIN, BUILD_WSL, BUILD_MACOS unset).

## Phase 3

- Full focused rerun, all from the repo root on macOS:
    - `python3 scripts/ci/suite_runner.py test_affected_scope.py`: 488 OK (678 s).
    - `... test_schema.py`: 203 OK. `... test_resolved_plan.py`: 103 OK.
    - `just test test-toolkit`: 484 passed, 4 skipped (includes
      `uploads_named_by_the_cell_contract_skip_when_it_failed_to_resolve` and
      `retired_packages_reach_verdict_through_package_evidence`).
    - actionlint unavailable (`command -v actionlint` found nothing); nothing installed.
- Lint: `just lint test-toolkit` is not a recipe; ran
  `cargo clippy -p test-toolkit --all-targets -- -D warnings`: clean (exit 0;
  only the cargo registry "failed to auto-clean cache data" permission warning).
- Scope check after this phase's edits:
  `git status --porcelain --untracked-files=all | awk '{print $NF}' | xargs python3 scripts/ci/affected_scope.py`
  → `packages: ['repo-deps', 'test-toolkit', 'darkmatter']`, `change_class: package`
  (the tree still holds Phases 1–2 uncommitted). Gates were run, not skipped:
    - `just test repo-deps`: 507 passed, 1 skipped.
    - darkmatter, narrowed to the planned cell's two tests
      (`shipped_schema_corpus::shipped_markdown_inline_schemas_parse_passively`,
      `shipped_schema_corpus::shipped_markdown_named_type_references_resolve`): 2 passed.
- Spec hygiene: `spec.md` now has `status: implemented`, `implemented: true`,
  `implemented_by: "claude/opus"` (see decisions_for_review). Not moved, not committed.
- Appended `## Implementation log` to `plan.md` (files changed, departures,
  actionlint availability, pre-fix reproduction).
- This phase changed no source, docs, or skill files; only spec/plan/log
  frontmatter and text. No new tests (verification-only phase); the
  requirement-to-test mapping is in Phase 1.
- No failures, skipped tests beyond the suites' own skips, or load-sensitive reruns.
- No local OS test rigs (BUILD_LINUX, BUILD_WIN, BUILD_WSL, BUILD_MACOS unset).
