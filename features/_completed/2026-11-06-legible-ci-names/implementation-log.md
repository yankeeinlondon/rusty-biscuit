---
spec: /Volumes/coding/rusty-biscuit/features/2026-11-06-legible-ci-names/spec.md
plan: features/2026-11-06-legible-ci-names/plan.md
implemented_by: claude/opus
started_phase: "1"
source_files_during_phase_1:
  - tools/test-toolkit/tests/ci_workflow_contracts.rs
  - features/2026-11-06-legible-ci-names/workflow_structure.py
docs_updated_during_phase_1:
  - features/2026-11-06-legible-ci-names/plan.md
docs_created_during_phase_1:
  - features/2026-11-06-legible-ci-names/implementation.md
  - features/2026-11-06-legible-ci-names/implementation-log.md
  - features/2026-11-06-legible-ci-names/workflow-structure-baseline.json
skills_files_updated_during_phase_1: []
packages:
  - test-toolkit
  - repo-deps
source_files_during_phase_2:
  - .github/workflows/ci.yml
  - .github/workflows/_area-ci.yml
  - .github/workflows/_package-ci.yml
  - .github/workflows/_wsl-ci.yml
  - .github/workflows/ci-infra-retry.yml
  - .github/workflows/release-plz.yml
  - scripts/ci/runner_loss.py
  - scripts/ci/reuse_validation.py
  - scripts/ci/publish_gaps.py
  - scripts/ci/test_runner_loss.py
  - scripts/ci/test_reuse_validation.py
  - scripts/ci/test_publish_gaps.py
  - tools/test-toolkit/tests/ci_workflow_contracts.rs
  - scripts/ci/test_ci_local.py
docs_updated_during_phase_2:
  - .github/ci/README.md
  - features/2026-11-06-legible-ci-names/plan.md
  - features/2026-11-06-legible-ci-names/implementation-log.md
docs_created_during_phase_2: []
skills_files_updated_during_phase_2: []
decisions_for_review:
  - |-
    Phase 1's new naming assertions accept either the recorded old name or
    the new name, so they pass before the rename. The plan allowed them to
    be red until Phase 2, but the phase's completion gate requires every
    test to pass. Each contract row carries the name before and after the
    rename; the test fails on any other name, so the old names are proved
    real and a typo in the map fails now. Phase 2 deletes the `before`
    values (and the step test's "old" side) to make only the new names
    acceptable. Alternative: red tests in Phase 1, which would block the
    phase gate. Nothing needs redoing if the reviewer prefers that; it is a
    one-line tightening either way.
  - |-
    The "baseline of non-presentation workflow structure" is a small script
    plus its saved output in the feature directory
    (`workflow_structure.py`, `workflow-structure-baseline.json`), not a
    permanent Rust snapshot test. A permanent snapshot would fail on every
    future legitimate workflow edit, long after this feature closes. The
    script removes only the workflow title and job and step `name:` keys
    (artifact `name` fields under `with:` stay) and Phase 2/3 diff its
    output against the baseline. If the reviewer wants a permanent guard,
    the script's logic would move into `ci_workflow_contracts.rs`, which
    has no YAML parser today (it would need a dev-dependency).
  - |-
    The evidence file is `implementation.md`, as the plan names it, next to
    this log. The log records progress; `implementation.md` holds the
    evidence, the frozen naming contract, and projected names for review.
  - |-
    Phase 2 renamed the `ci.yml` workflow title to "Continuous integration"
    as the plan says, and ALSO changed one line in each of two workflows the
    plan put out of scope: `release-plz.yml` and `ci-infra-retry.yml`. Both
    start when a workflow called `ci` finishes, and GitHub matches that by
    the title. With the title changed and those lines left alone, releases
    and the automatic retry after a lost runner would silently stop. A
    contract test now checks both triggers against the actual title.
    Alternative: keep the title `ci` and touch nothing else, which leaves the
    one reader-facing title the plan wanted changed unchanged. To redo:
    restore `name: ci` in ci.yml, revert the two trigger lines, and set the
    `title` of ci.yml back to "ci" in the test file's NAMING_CONTRACT.
  - |-
    Two renamed steps now fall into a different category in the runner-loss
    report message (the wording that says which stage failed). The old
    `cargo check (declared kinds no test gate compiles)` was classed as a
    test step only because it contained the words "no test"; the new
    `Check compilation of declared targets` is classed as a compile check.
    The old `Companion suites` matched no category; `Run companion test
    suites` is a test step. The plan asked to keep the old categories but
    also gave these exact new names; the new categories are the accurate
    ones, and the category never affects whether a retry happens. To redo:
    pick other step names, or add exceptions to `STAGE_MARKERS`.
  - |-
    The accepted-gap check's summary text still says the area's
    `coverage-audit` job owns completeness. That is the job's identifier,
    which is unchanged; its displayed name is now "Check required results".
    The plan says to keep every field other than the check name the same,
    so the text was left alone. To change it: one sentence in
    `scripts/ci/publish_gaps.py` and its exact-match test.
  - |-
    Phase 3 left one failing Python test red:
    `test_ci_local.py` NativeProvisioningTests
    `test_the_build_owner_installs_only_its_own_records_closure`. Six steps
    in `ci.yml`'s `build` job use `run: >-` (folded text), and
    `scripts/ci/workflow_reading.py` refuses that form on purpose. Those steps
    came from commit 212070aba (build-storage diagnostics), not from this
    feature. There are two fixes with different results: rewrite the six steps
    as `run: |` with line continuations (changes `ci.yml` commands, so this
    feature's "commands unchanged" check would no longer be empty), or teach
    the reader folded text (widens a reader that is deliberately narrow). That
    choice belongs to the diagnostics change's owner, so neither was made here.
    Nothing in this feature needs redoing either way.
  - |-
    The new name reference lives in one place, a section of
    `docs/topics/ci-cd.md` called "Reading job and check names". The two
    older paragraphs there that described label shapes were replaced by a
    link to it, and `.github/ci/README.md` and the `rust-devops` skill point
    to it instead of copying the table. Alternative: a separate page under
    `docs/cicd/`. Moving it would mean changing three links.
source_files_during_phase_3: []
docs_updated_during_phase_3:
  - docs/topics/ci-cd.md
  - .github/ci/README.md
  - features/2026-11-06-legible-ci-names/implementation.md
  - features/2026-11-06-legible-ci-names/plan.md
  - features/2026-11-06-legible-ci-names/implementation-log.md
docs_created_during_phase_3: []
skills_files_updated_during_phase_3:
  - .claude/skills/rust-devops/ci-cd.md
  - .claude/skills/os/wsl.md
source_code:
  - .github/workflows/ci.yml
  - .github/workflows/_area-ci.yml
  - .github/workflows/_package-ci.yml
  - .github/workflows/_wsl-ci.yml
  - .github/workflows/ci-infra-retry.yml
  - .github/workflows/release-plz.yml
  - scripts/ci/runner_loss.py
  - scripts/ci/reuse_validation.py
  - scripts/ci/publish_gaps.py
  - scripts/ci/test_runner_loss.py
  - scripts/ci/test_reuse_validation.py
  - scripts/ci/test_publish_gaps.py
  - scripts/ci/test_ci_local.py
  - tools/test-toolkit/tests/ci_workflow_contracts.rs
  - features/2026-11-06-legible-ci-names/workflow_structure.py
documentation:
  - docs/topics/ci-cd.md
  - .github/ci/README.md
  - .claude/skills/rust-devops/ci-cd.md
  - .claude/skills/os/wsl.md
  - features/2026-11-06-legible-ci-names/implementation.md
  - features/2026-11-06-legible-ci-names/implementation-log.md
  - features/2026-11-06-legible-ci-names/plan.md
  - features/2026-11-06-legible-ci-names/workflow-structure-baseline.json
completed_phase: 3
implemented: true
---

# Implementation Log for 2026-11-06-legible-ci-names (3 phases)

## Phase 1

### Evidence capture

- Read-only `gh api` calls, both succeeded first time (no retries, no new runs):
    - `actions/runs/42491863852/jobs`: nightly schedule run, 222 jobs.
    - `commits/5a2760470df3cec36fe8668859efd5549ca4dd12/check-runs`: PR head, 93 check runs.
- The PR check list gives observed examples for two levels of one package
  (`claudine-cli` L1 and L2), a nested area with an empty selection
  (`area-ci (darkmatter/dmls) / package-ci`, skipped), and the current
  accepted-gap names (`accepted gap: darkmatter / darkmatter / L2 (macos-latest)`).
  The nightly gives native Windows, WSL2 nesting, and `claudine/rendezvous`.
- No skipped name in either source contains `${{` within scope. The one
  unexpanded name observed (`Work counts (${{ matrix.os }})`) is from a
  workflow outside this feature.
- `preflight` is a matrix job (`preflight (ubuntu-latest)` today). It is not
  yet known whether GitHub appends matrix values to its new static name; it
  owns no result cell, so no reader depends on it. Recorded in
  `implementation.md` as an uncertain projection.
- Every old step name in the plan's Rule 7 table was confirmed present in
  its workflow.

### Structure baseline

- `features/2026-11-06-legible-ci-names/workflow_structure.py` prints the four
  workflows as canonical JSON without workflow titles or job/step `name:`.
  Its output was saved as `workflow-structure-baseline.json` (125 KB, 38
  remaining `name` keys, all artifact/input names). Re-running it
  immediately produced an identical file.
- Captured from the working tree, which already carried uncommitted
  `_package-ci.yml` edits from other work on this branch.

### Contract tests (`tools/test-toolkit/tests/ci_workflow_contracts.rs`)

New section "legible names: the frozen naming contract":

| Requirement | Test |
| --- | --- |
| Ten `ci.yml` identifiers and every job of the four workflows stay, in order; each job's name is the contract name (or the recorded old one) | `reader_facing_jobs_follow_the_naming_contract` |
| New titles and job/step names are literal: no `${{`, no matrix-style suffix, not empty or punctuation-only | `the_naming_contract_uses_only_literal_descriptive_names` |
| `ci-gate` stays the exact displayed context; reporting stays `(advisory)` | `the_rename_keeps_the_required_context_and_the_advisory_report` |
| `wsl2`, `area-ci`, `build` keep automatic labels from their matrices (`check`/`test`/`lint` are already covered by `lint_and_check_labels_identify_their_environment`) | `delegation_area_and_build_jobs_keep_their_automatic_labels` |
| Each step rename names a real step under exactly one of its names; `Compile unchanged dependents` stays | `step_renames_name_real_steps` |

Boundary: workflow source text (F1, L1). These tests need no real Actions run.
No expected-red test was added (see `decisions_for_review`).

Focused run: `cargo nextest run -p test-toolkit --test ci_workflow_contracts -E 'test(naming_contract) | test(advisory_report) | test(automatic_labels) | test(step_renames) | test(lint_and_check_labels)'`: 8 passed.

Input Robustness Matrix: not applicable. No configuration reader or field
validator was added; the structure script is a review aid, not shipped code.

### Gates

Scope calculator (all Phase 1 paths): `packages: ['test-toolkit']`,
`change_class: package`, so both gates run.

- `just test` (in `tools/`): 531 run, 531 passed, 4 skipped. Four tests
  went over 5 s on this host: `archive_path_guard::no_archive_executed_target_bakes_in_a_producer_path`,
  `archive_path_guard::every_allowlist_entry_still_names_a_live_site`,
  `ci_workflow_contracts::the_shipped_planner_emits_plans_the_guards_reader_accepts`,
  and `kache_host_contracts::report_resolves_the_worktree_base_by_the_wt_contract`.
  These were already slow before this change; none is a Phase 1 test (the
  new tests take about 0.03 s each).
- `just lint` (in `tools/`): clean.
- Dependent packages: not run. Phase 1 changed only an integration-test file
  of `test-toolkit`, which no other package compiles.
- Cross-OS check: not run. The new tests only read workflow text, through
  the existing `read()` helper that already normalizes CRLF.

Phase 1 status: both tasks are done and checked off in the plan.

## Phase 2

### Baseline precondition

- The working tree was clean at the start (the `_package-ci.yml` edits noted in
  Phase 1 had been committed), and
  `python3 features/2026-11-06-legible-ci-names/workflow_structure.py | diff features/2026-11-06-legible-ci-names/workflow-structure-baseline.json -`
  was empty, so the Phase 1 baseline was reused without regeneration.

### Wave 2: workflow surfaces (done directly, not delegated)

- Applied Rule 3's titles and job names and Rule 7's eleven step renames in
  the four workflows through a script that asserted each old string's exact
  occurrence count (`Certify this cell's completeness` occurs three times in
  `_package-ci.yml`, once per producer job). `area-ci`, `build`, `check`,
  `test`, `lint`, `wsl2` keep their automatic labels.
- Comments that described rendered names were corrected in all four files: the
  stale pre-row example and the "no display-name parsing anywhere" claim at the
  top of `_area-ci.yml` (now names the two grandfathered readers), and every
  "No `name:`" comment on a job that now carries a literal name.
- Run-script `echo` banners (`guest post-mortem`, `host post-mortem`) are
  execution text and were left unchanged.
- **Workflow title coupling found.** `workflow_run` triggers match a workflow
  by its title, and both `release-plz.yml` and `ci-infra-retry.yml` listened
  for `ci`. Renaming `ci.yml`'s title alone would have silently stopped
  release automation and the runner-loss retry. Both triggers now name
  `Continuous integration`, and the contract
  `every_downstream_consumer_reads_the_same_ci_run_conclusion` now compares
  each trigger against `ci.yml`'s actual title, so a future rename fails the
  suite instead of production (see `decisions_for_review`).
- Structure check after the edits: empty diff against the baseline (run twice:
  after the renames and after the comment edits).

### Wave 2: readers (subagent) and gap checks (subagent), run in parallel

- `runner_loss.py`: `WSL_GUEST_SEGMENT = "Run guest tests"`; when it is the
  last segment, the cell comes only from the immediately enclosing
  `wsl2 (<row>)` segment through `row_cell`, else `None`. Every earlier branch
  is unchanged. `Report CI results (advisory)` added to `NON_PRODUCER_JOBS`;
  the audit is deliberately not a non-producer.
- `reuse_validation.py`: `AUDIT_JOB` accepts `coverage-audit` and
  `Check required results`; `legacy_audits_passed` and `count_audits` still
  share it.
- `publish_gaps.py`: `GATE_ACTIONS` and `ENVIRONMENT_NAMES`; name is
  `ACCEPTED GAP: <package>: <action> on <environment>`; unknown values raise
  `Refusal` before any post. `NAME_LIMIT` still applies.
- Stage classification (`failure_stage`) of renamed steps: nine keep their
  stage; two change (see `decisions_for_review`):
  `cargo check (declared kinds no test gate compiles)` was `test` (the words
  "no test" matched) and is now `check`; `Companion suites` was `other` and is
  now `test`. The stage only words the retry-refusal message; the retry
  decision itself never reads it. `STAGE_MARKERS` is unchanged.

### Wave 3: contracts

- Phase 1's contract tightened: `NAMING_CONTRACT` keeps only the new names
  (`title_before` and the `before` column removed) and is compared with
  `assert_eq!`; `step_renames_name_real_steps` now requires every old name gone
  and each new name present, at most once per job.
- Old step names updated in every other assertion of
  `ci_workflow_contracts.rs` (GATE_COMMAND_STEPS, the WSL archive/census
  ordering, the `Run planned tests` position test, the WSL ignore list,
  certification lookups, JUnit upload lookups, the clippy `if:` literal, the
  coverage-audit enforcement lookups), and
  `the_gap_publisher_is_the_only_job_holding_checks_write` now requires a
  literal publisher name instead of none. Historical parser fixtures in the
  Python suites were kept on purpose.
- New `projected_full_names_compose_from_the_declared_names` builds the four
  plan examples from the declared names (research L1 and L2, the schematic
  WSL2 guest, `claudine/rendezvous`'s audit) plus four skipped shapes with no
  `${{`. `test_runner_loss.py::RenamedJobNameTests::test_the_documented_projected_names_parse_to_their_cells`
  parses the same strings. These are projections, not observed run records.
- `RowAdapterOracleTests` and row budgets untouched; `test_resolved_plan.py`
  and `test_schema.py` pass unchanged, so no matrix field was added.
- `.github/ci/README.md` described the old producer label shape and the old
  gap check name; both passages corrected (drift found by the gap subagent).

### Requirement-to-test map

| Behavior | Test(s) | Boundary / rung |
| --- | --- | --- |
| Titles, job names, identifiers per the map | `reader_facing_jobs_follow_the_naming_contract`, `the_naming_contract_uses_only_literal_descriptive_names` | workflow text, F1, L1 |
| Step renames landed | `step_renames_name_real_steps` + every updated step lookup | workflow text, F1, L1 |
| `ci-gate` exact; report advisory | `the_rename_keeps_the_required_context_and_the_advisory_report` | F1, L1 |
| Downstream `workflow_run` triggers follow the title | `every_downstream_consumer_reads_the_same_ci_run_conclusion`, `release_automation_follows_successful_ci` | F1, L1 |
| Full projected names, skipped names literal | `projected_full_names_compose_from_the_declared_names` | F1, L1 |
| Guest leaf, malformed rows, historical names, build, advisory, audit veto, stages | `test_runner_loss.py::RenamedJobNameTests` (12), updated `JobNameCorpusTests` workflow walk | public `parse_job_name`/`classify`/`should_rerun`, in-process |
| Audit discovery, both spellings, lookalikes | `test_reuse_validation.py::AuditJobNameTests` (5) | public `count_audits`/`legacy_audits_passed` |
| Gap check names, mappings, refusals, limit, unchanged other fields, nothing posted | `test_publish_gaps.py::CheckNameTests` (7), two `RefusalTests`, `GhInvocationTests::test_an_unknown_gate_or_environment_posts_nothing` | `payloads`/`check_run`, plus the CLI with a stub `gh` |
| Protected structure unchanged | `workflow_structure.py` diff against the baseline | review aid |

Input Robustness Matrix: not applicable. No configuration reader or field
validator was added; display-name parsing is not a file-format loader. The
gap-name maps refuse unknown values (tested), which is the only new input
handling.

### Gates

Scope calculator (all Phase 2 paths, after every edit):
`packages: ['repo-deps', 'test-toolkit']`, `change_class: package`, so the
gates ran.

- `just test repo-deps test-toolkit` (repo root): 1045 run, 1045 passed,
  5 skipped.
- `just lint` (in `tools/`): clean. `repo-deps` has no area justfile;
  `cargo clippy -p repo-deps --all-targets -- -D warnings`: clean.
- `just check-tier-coverage tools`: nothing stranded.
- `ci_workflow_contracts` binary alone: 227 of 227 passed.
- Python suites: `test_runner_loss.py` 72 OK, `test_reuse_validation.py` 52 OK,
  `test_publish_gaps.py` 29 OK, `test_resolved_plan.py` 103 OK,
  `test_schema.py` 203 OK.
- `test_ci_local.py`: `EmptyMatrixGuardTests` failed on the new literal
  `preflight` name. The test said both matrix jobs must have no name at all.
  It now requires the literal `Verify build prerequisites` for `preflight`
  and no name for `area-ci`, and forbids `${{` in either; the class passes.
  **Pre-existing failure, not fixed:**
  `NativeProvisioningTests.test_the_build_owner_installs_only_its_own_records_closure`
  fails because `scripts/ci/workflow_reading.py` cannot read the `run: >-`
  block of `ci.yml`'s `Sample storage before the cache restore` step. That
  step was added by commit `212070aba` and is identical at HEAD; this phase
  did not touch it. It also fails when run alone.
- Structure check: empty diff against the Phase 1 baseline. No job or step
  name contains `${{`.
- Dependent packages: none to run. The Python scripts are not compiled by
  any crate, and the Rust change is an integration test of `test-toolkit`.
- Cross-OS check: not run. The changes are workflow text, Python string
  handling, and a test that reads workflow text through the existing
  CRLF-normalizing helper; there is no path or OS-specific code.

Phase 2 status: all three tasks are done and checked off in the plan.

## Phase 3

### Documentation

- `docs/topics/ci-cd.md`: new section "Reading job and check names" before
  "How the planner reads the workspace". It covers the four workflow titles
  and the `workflow_run` title coupling (`release-plz.yml` and
  `ci-infra-retry.yml` wait for `Continuous integration`); the
  infrastructure job map; `build (<producer>)` as archive preparation,
  including the shared Linux archive the WSL2 guest consumes; how a package
  job's automatic suffix is composed and why it keeps stored values; gate and
  environment tables (Level 1, Level 2, browser, WSL2 defined); the projected
  examples from `projected_full_names_compose_from_the_declared_names`, called
  projected, plus one observed pre-rename name for contrast; the
  accepted-gap check name; a table of skipped, reused, deferred, gap, and
  blocking states with examples; a Mermaid flow from selection through the
  audit to `ci-gate`; and the two grandfathered name readers. The old
  label-shape paragraph (which still showed `package-ci`) and the old
  "Skippable jobs carry no `name:`" paragraph were replaced by a link to the
  section and by the section itself. No dated feature is named on the page.
- `.github/ci/README.md`: its "Skippable jobs carry no `name:`" paragraph
  contradicted the literal `preflight` name; rewritten to separate literal
  infrastructure names from automatic cell-carrying names, with a pointer to
  the topic page. The Phase 2 passages (producer label, gap name) were
  rechecked against the topic page and agree.
- `.claude/skills/rust-devops/ci-cd.md`: `preflight`/`area-ci` note, the
  label example, the WSL2 guest leaf, and the presentation rule now
  distinguish static infrastructure names from automatic matrix labels, keep
  the no-expression rule, and name both readers and the title coupling.
- Drift found while searching for old step names and fixed in the same pass:
  `Companion suites` in `docs/topics/ci-cd.md` and twice in the skill,
  `Companion suites (lint)` in the skill and `.github/ci/README.md`, and
  `"Certify this cell's completeness"` in `.claude/skills/os/wsl.md`.
- `implementation.md`: added test outcomes, the departure from the `label`
  design, preserved properties, and the observed-versus-projected status.

### Verification

| Check | Result |
| --- | --- |
| `test_runner_loss.py` | 72 OK |
| `test_reuse_validation.py` | 52 OK |
| `test_publish_gaps.py` | 29 OK |
| `test_resolved_plan.py` | 103 OK |
| `test_schema.py` | 203 OK |
| `test_ci_local.py` | 104 run, 1 failure that predates this feature (see below) |
| `workflow_structure.py` diff | empty |
| `just test repo-deps test-toolkit` | 1045 passed, 5 skipped |
| `just lint` (tools), `cargo clippy -p repo-deps --all-targets -D warnings`, `just lint` (darkmatter) | clean |
| `just check-tier-coverage tools` | nothing stranded |

- Scope calculator over the whole uncommitted tree:
  `packages: ['repo-deps', 'test-toolkit', 'darkmatter']`,
  `change_class: package`, so the gates ran. `darkmatter` is selected because
  its tests read `docs/topics/` and `.claude/skills/`: those are
  `shipped_schema_corpus::shipped_markdown_inline_schemas_parse_passively`
  and `shipped_markdown_named_type_references_resolve`, which both passed
  (2 of 2). The full darkmatter suite (over 11,000 tests) was not run; no
  darkmatter code changed.
- **Pre-existing failure, still red, not fixed:**
  `NativeProvisioningTests.test_the_build_owner_installs_only_its_own_records_closure`
  (also red alone). `workflow_reading.py` refuses the `run: >-` steps that
  commit `212070aba` added to `ci.yml`'s `build` job. See
  `decisions_for_review`.
- No job or step name contains `${{`; `ci-gate` unchanged; this feature
  touches neither `schema.py` nor `.github/ci/schemas/`.
- Cross-OS check: not run. Phase 3 changed only Markdown.
- Requirement-to-test map: Phase 3 added no behavior. The documented
  behavior is covered by the Phase 2 tests listed in its map, all of which
  passed again here.

Phase 3 status: both tasks done and checked off; the feature is ready for
review and stays in its active directory.
