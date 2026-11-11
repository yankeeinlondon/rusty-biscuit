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
message_to_agent: |-
  Phase 1 left the naming contract in tools/test-toolkit/tests/ci_workflow_contracts.rs
  (section "legible names: the frozen naming contract": NAMING_CONTRACT and
  STEP_RENAMES). Those tests accept either the old or the new names, so they
  pass before and after the rename. After applying the renames in Phase 2,
  tighten them: in reader_facing_jobs_follow_the_naming_contract accept only
  `after` and `title` (drop `title_before` and the `before` column), and in
  step_renames_name_real_steps require old_count == 0 and new_count == 1 per
  listed file. Then confirm no protected structure changed with
  `python3 features/2026-11-06-legible-ci-names/workflow_structure.py | diff features/2026-11-06-legible-ci-names/workflow-structure-baseline.json -`
  run from the repo root (an empty diff is the pass). The baseline was taken
  from the working tree including uncommitted _package-ci.yml edits that were
  already on this branch; if those change before Phase 2, regenerate the
  baseline first from the pre-rename files. preflight is a matrix job, so its
  new static name may render with "(ubuntu-latest)" appended; no reader
  depends on it. Existing step-name assertions elsewhere in the contracts
  file (GATE_COMMAND_STEPS, the WSL archive ordering test, "Tests" position
  tests) still use the old names and must be updated in Phase 2/Wave 3.
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
