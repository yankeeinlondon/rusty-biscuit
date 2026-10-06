---
kind: fix
name: archive-guard-full-tree
date: 2026-10-06
status: draft-spec
related:
  - 2026-09-19-less-brittle
  - 2026-10-05-worker-git-processes
reviewed: false
clarified: false
implemented: false
human_review: true
---

# The archive-path guard always scans the full tree

## Summary

The archive-path guard stops reading its scan scope from the resolved plan and
always scans the full eligible corpus. The planner keeps deciding **whether**
the guard runs (its selection rules are unchanged) but no longer tells it
**what** to scan, so the plan's `archive_guard` section, its Python emitter and
validator, its Rust reader, and the two cross-language contract tests that run
the whole planner to exercise that handoff are removed. The resolved plan's
schema version moves from 7 to 8.

This reverses one part of `2026-09-19-less-brittle`: the table that scoped a
pull request's guard scan to the changed eligible files. That spec's real goal,
reliable **selection** of the guard, is kept as it is.

## Background

### What the guard is

`tools/test-toolkit/src/archive_guard.rs` lexes Rust source and refuses
compile-time paths that do not survive an archived run:
`env!("CARGO_MANIFEST_DIR")`, `env!("CARGO_BIN_EXE_…")`, and literal
`/home/runner/work/…` paths, except through the runtime-first macros
(`biscuit_test_harness::manifest_dir!`, `bin_exe!`) or an `ALLOWED` entry with a
reason. Its driver is `tools/test-toolkit/tests/archive_path_guard.rs`, run by
`just archive-path-guard` in `tools/test-toolkit`. The driver has two tests:

| Test | Scope today |
| --- | --- |
| `no_archive_executed_target_bakes_in_a_producer_path` | the plan's scope (full tree, or a changed-file list) |
| `every_allowlist_entry_still_names_a_live_site` | **always the full tree** |

### Where it runs

- **CI (authoritative):** as the `archive-path-guard` companion suite of
  `test-toolkit`'s `lint` cell on `ubuntu-latest` (`_package-ci.yml`, step
  guarded by `contains(fromJSON(steps.cell.outputs.companion_suites),
  'archive-path-guard')`), with `BISCUIT_ARCHIVE_GUARD_PLAN` pointing at the
  downloaded resolved plan.
- **Locally (advisory):** `just/ci-local.just` runs it in the pre-push hook when
  the plan attaches it, and says a non-Ubuntu pass leaves the planned
  `ubuntu-latest` execution outstanding.

### What the plan carries today

`affected_scope.py::archive_guard_scope` returns exactly one of three shapes,
stored as the plan's required top-level `archive_guard` field (schema version 5
onward):

```json
{"selected": false, "reason": "…"}
{"selected": true, "mode": "full", "reason": "…"}
{"selected": true, "mode": "changed", "paths": ["…"], "reason": "…"}
```

The same function also decides **selection**: a changed path that is eligible
Rust source or one of `ARCHIVE_GUARD_OWN_INPUTS` triggers the guard; the guard
is not selected when the event schedules no `ubuntu-latest` cell; a full-scope
run selects it. When the owner (`test-toolkit`) is not otherwise impacted,
`archive_guard_only_selection` adds a `test-toolkit` lint cell that runs the
guard alone. Selection is already visible to every reader as that cell's
`companions` entry named `archive-path-guard`, independently of the
`archive_guard` section.

## Problem

### The scope handoff costs more than it saves

Measured on CI (PR #116, run 37537695506, `test-toolkit` lint on
`ubuntu-latest`):

| Test | Scope | Time |
| --- | --- | ---: |
| `every_allowlist_entry_still_names_a_live_site` | full tree, 4,038 files | 3.9 s |
| `no_archive_executed_target_bakes_in_a_producer_path` | `changed(1 listed)` | 0.03 s |

The cell already pays for a full-tree pass on every guard run, so scoping the
other test saves at most about 4 s. Recent full-tree runs report
`files-checked=4038 … violations=5` (all five allowed).

### The handoff's contract tests time out

Two tests in `tools/test-toolkit/tests/ci_workflow_contracts.rs` exist only to
prove that the Python emitter and the Rust reader agree:

- `the_shipped_planner_emits_plans_the_guards_reader_accepts` runs the shipped
  planner three times against the real workspace, once per shape
  (`-- README.md` → not selected, `--all` → full, `-- tools/test-toolkit/src/archive_guard.rs`
  → changed), and parses each result with `GuardPlan::from_plan_json`.
- `the_shipped_planner_omits_deletions_and_the_reader_refuses_an_unexpected_absence`
  checks that deleted paths are omitted from `changed` and that the reader
  refuses a listed path that does not exist.

Their cost, for the same two tests:

| Host and mode | Time |
| --- | ---: |
| macOS development host, native | ~4 s |
| `BUILD_LINUX`, native | 16 s |
| `BUILD_WSL`, native | 1.9 s + 5.9 s |
| `BUILD_WSL`, nextest archive mode (as CI runs it) | 51 s + 52 s |
| `ubuntu-latest` CI | 64–82 s each |
| `windows-latest` CI | **timed out at 90 s** (runs 37529082394, 37537695506) |

In archive mode the `-- README.md` plan alone took 30.1 s against about 3 s
from a checkout; `--all` and a `.rs` change took about 1.2 s and
`cargo metadata` about 1.1 s. The `README.md` plan is the expensive one
because a non-source change runs the planner's test-input scan
(`scripts/ci/test_inputs.py`) over every test-reachable Rust file, which the
guard's answer does not depend on. Why archive mode makes that scan about ten
times slower is unexplained (see Not in scope). These timeouts keep
`test-toolkit` L1 red on `windows-latest`, on `main` and on any `ci:all-os`
pull request.

### Why the original reason no longer holds

`2026-09-19-less-brittle` scoped a pull request's scan to its changed files.
With the allow-list keeping the full tree green, and pushes to `main` already
scanning the full tree, a pull request cannot inherit a violation from `main`:
one could not have reached `main` in the first place. Changed-only scanning
therefore buys nothing a full scan does not, at a measured 3.9 s.

## Decision

1. The guard always scans the full eligible corpus. It reads no plan and no
   scope variable.
2. The planner's selection rules are unchanged: what triggers the guard, the
   `ubuntu-latest`-only rule, full-scope runs, and the guard-only `test-toolkit`
   lint cell.
3. The resolved plan no longer carries the guard's scope. Selection remains
   expressed by the lint cell's `archive-path-guard` companion, and the
   guard-only cell keeps its human-readable `reason`.
4. The resolved plan's schema version moves from 7 to 8 under
   `docs/cicd/schema-versions.md`'s rules.

## Requirements

### R1. The guard (`tools/test-toolkit`)

- `scan` checks the full eligible corpus; the eligibility policy
  (`is_eligible`, `SKIPPED_DIRS`), `ALLOWED`, the matcher, and exemption
  maintenance are unchanged.
- Remove `GuardPlan`, `ScanMode` (or reduce it to the full-tree case if a
  remaining caller needs a mode type; prefer removal), plan parsing
  (`from_plan_json`, `from_env`), `PLAN_SCHEMA_VERSION`, and every reference to
  `BISCUIT_ARCHIVE_GUARD_PLAN`.
- `archive_path_guard.rs` keeps its two tests. The printed summary still states
  the mode (`full-tree`) and the file count, so a run that checked nothing
  cannot pass silently.
- `tools/test-toolkit/src/lib.rs` re-exports and `matcher_tests.rs` follow the
  removals.

### R2. The planner (`scripts/ci/affected_scope.py`)

- Keep the selection logic: `archive_guard_eligible`,
  `archive_guard_triggered`, `ARCHIVE_GUARD_OWN_INPUTS`, the
  `LINT_ENVIRONMENT` check, full-scope selection, and
  `archive_guard_only_selection` with its cell `reason`.
- Replace `archive_guard_scope`'s three-shape result with a selection decision
  (selected or not, with a reason used where a reason is shown today); remove
  `mode`, `paths`, and the top-level `archive_guard` plan field.
- Keep `SUITE_REGISTRY["archive-path-guard"]` and the
  `.github/ci/schemas/archive_guard_cases.json` test-input mapping only if
  still needed after `archive_guard_cases.json` is removed (R3); otherwise
  remove the mapping entry too.

### R3. The resolved plan schema (version 7 → 8)

Follow `docs/cicd/schema-versions.md`. In one change:

- `scripts/ci/schema.py`: drop `archive_guard` from the required fields and
  remove `_archive_guard`, `ARCHIVE_GUARD_FIELDS`, `ARCHIVE_GUARD_MODES`;
  bump `RESOLVED_PLAN_SCHEMA_VERSION` to 8.
- Regenerate `.github/ci/schemas/contract.json`; delete
  `.github/ci/schemas/archive_guard_cases.json` and its readers.
- Move every mirror the changelog lists: `ci-rollup.rs::PLAN_SCHEMA_VERSION`;
  `test_toolkit::archive_guard::PLAN_SCHEMA_VERSION` is removed with R1
  (record that in the changelog row); the `.githooks/tests/fixtures/plan-*.json`
  fixtures and `affected_scope_stub.py`; `scripts/ci/plan_fixtures.py`.
- `change_inventory.deleted` was added in version 5 for the guard. Check every
  reader (`scripts/ci-rollup.rs` has a `deleted` symbol; `diff_scope.py` and
  the planner's own tests use it). Keep it if anything but the guard reads it;
  otherwise remove it in the same bump. Record which in the changelog row.
- Add a version 8 row to `docs/cicd/schema-versions.md` naming this fix by
  `2026-10-06-archive-guard-full-tree`, and state that a scope receipt from an
  earlier generation misses once as `scope-schema`.
- `scripts/ci-plan.rs` (the plan renderer) and its tests stop reading the
  section.

### R4. Workflows and the local runner

- `.github/workflows/_package-ci.yml`: remove `BISCUIT_ARCHIVE_GUARD_PLAN` and
  its comment from the companion step. The "a missing plan is an error"
  safeguard it describes no longer applies, because the guard's result no
  longer depends on a plan.
- `just/ci-local.just`: stop exporting `BISCUIT_ARCHIVE_GUARD_PLAN`; update the
  `jq` that reads `.archive_guard` (around line 351) to read selection from the
  cell companions, or remove it if it only reported scope. Keep the advisory
  "the planned ubuntu-latest execution remains OUTSTANDING" wording.
- `tools/test-toolkit/justfile`: update the `archive-path-guard` recipe's
  comment.

### R5. Tests

- Delete `the_shipped_planner_emits_plans_the_guards_reader_accepts` and
  `the_shipped_planner_omits_deletions_and_the_reader_refuses_an_unexpected_absence`,
  and any other contract in `ci_workflow_contracts.rs` that pins the removed
  section or variable (about 27 references today).
- Python suites under `scripts/ci/`: guard-scope tests in
  `test_affected_scope.py` become selection tests (a documentation change
  selects no guard; an eligible `.rs` change selects it; an owned input selects
  it; a deleted eligible file still selects it; a full-scope run selects it; an
  event without `ubuntu-latest` does not; the guard-only cell appears when
  `test-toolkit` is not otherwise impacted). Update `test_schema.py`,
  `test_resolved_plan.py`, `test_ci_local.py`, `test_evidence_reuse.py`, and
  `test_local_evidence.py` for the removed field and the version.
- `scripts/ci-plan-tests.rs` and `scripts/ci-rollup-tests.rs` follow R3.
- No timeout is raised anywhere.

### R6. Documentation

The `docs/` tree and these pages describe current behavior, so update them in
the same change: `.github/ci/README.md`, `.github/ci/schemas/README.md`,
`.claude/skills/rust-devops/ci-cd.md`, and the guard's mention in
`docs/dependencies.md`. A docs page states the full-tree behavior in its own
words and does not name this spec.

## Acceptance

1. Outside `docs/cicd/schema-versions.md`'s history rows and this spec, no file
   contains `BISCUIT_ARCHIVE_GUARD_PLAN`, `GuardPlan`, `archive_guard_cases`,
   or a reference to the plan's `archive_guard` field (verify with `rg`).
2. Selection behavior is unchanged, proven by the Python selection tests in R5.
3. These pass locally: `python3 -m unittest` in `scripts/ci`;
   `just test` and `just lint` in `tools/test-toolkit`; `repo-deps` L1
   (`just test repo-deps` from the root); the pre-push hook's own tests under
   `.githooks/tests`.
4. `just cross-check test-toolkit --os windows` passes with no timeout, and the
   guard reports `mode=full-tree` in CI's `test-toolkit` lint cell in under
   about 5 s.
5. The pull request carries `ci:all-os`, and its `test-toolkit` cells pass on
   every environment it schedules.

## Not in scope

These are open and recorded so they are not lost:

- **Archive-mode planner slowdown.** The `-- README.md` plan takes about 30 s
  in nextest archive mode against about 3 s from a checkout on the same WSL
  guest; `cargo metadata` and the other plan kinds are normal, and file reads
  are plain `read_text`. CI's own scope job plans from a normal checkout, so it
  is not affected; tests that run the planner are. Not yet explained.
- **Planner in Rust.** `scripts/ci/test_inputs.py` is a Rust lexer written in
  Python and the expensive part of planning; moving it into Rust, sharing the
  guard's lexer, is the proposed first step of an incremental migration. It
  needs its own spec.
- **`repo-deps` Windows MAX_PATH.** Seven `ci-build` archive tests fail on
  `windows-latest` with `LNK1104` because the nested fixture build reaches a
  266-character path; four more time out. Partial uncommitted work is on
  branch `fix/ci-build-fixture-max-path` (worktree
  `.claude/worktrees/agent-aaa3ce04badad1120`).
- **Fake-Gitea record-and-replay.** `list_prs::a_changed_origin_drops_the_old_head_checks_fallback_notice_and_caption`
  misses the 3 s wait on `windows-latest`; about half its transport time is the
  test fixture running `git http-backend` per request. Partial uncommitted work
  is on branch `test/gitea-replay` (worktree
  `.claude/worktrees/agent-ad15a63a110599e32`), with a probe test to delete.
- **Proving `2026-10-05-worker-git-processes` on Windows CI.** Re-run `main`'s
  run 37517419405 once the items above are merged; its `worktree` cell passed
  521/521 last time and failed only on the UTF-8 crash fixed by PR #116.
