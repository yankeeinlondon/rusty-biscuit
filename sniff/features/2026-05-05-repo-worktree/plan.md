---
phases: 4
created: 2026-05-05
start_phase: 1
source_files_during_phase_1: []
docs_updated_during_phase_1: []
docs_created_during_phase_1: []
skills_files_updated_during_phase_1: []
source_files_during_phase_2:
  - sniff/lib/src/filesystem/git/worktree.rs
  - sniff/lib/src/filesystem/git/mod.rs
docs_updated_during_phase_2: []
docs_created_during_phase_2: []
skills_files_updated_during_phase_2: []
source_files_during_phase_3:
  - sniff/cli/src/args/repo.rs
  - sniff/cli/src/args/mod.rs
  - sniff/cli/src/commands/mod.rs
  - sniff/cli/src/output/repo_json.rs
  - sniff/cli/src/output/mod.rs
docs_updated_during_phase_3: []
docs_created_during_phase_3: []
skills_files_updated_during_phase_3:
  - .opencode/skill/sniff/SKILL.md
source_files_during_phase_4:
  - sniff/cli/src/args/mod.rs
  - sniff/cli/tests/cli.rs
docs_updated_during_phase_4: []
docs_created_during_phase_4: []
skills_files_updated_during_phase_4: []
packages:
  - sniff
---

# Execution Plan: `sniff repo worktree`

## Overview

Implement the `sniff repo worktree` subcommand that identifies if the current directory is inside a linked Git worktree and returns its base directory name. The command must treat the main/original worktree as "not in a worktree", support `--no-error` and `--on-error` switches, and respect the global `--json` flag.

## Phase 1: Discovery & Design

**Goal**: Verify existing worktree detection APIs and command patterns before writing implementation code.

| Step | Task | Observable Outcome |
|------|------|-------------------|
| 1.1 | Read `lib/src/filesystem/git/discovery.rs` to confirm `GitInfo` exposes `in_worktree`, `base_repo_root`, and `worktrees` fields. | Know exact struct fields and types available. |
| 1.2 | Read `lib/src/filesystem/git/mod.rs` to see re-exported helpers. | Know which library functions are public. |
| 1.3 | Read `cli/src/commands/mod.rs` to identify how early-return repo commands bypass the detection pipeline. | Can pattern-match the dispatch logic for a lightweight command. |
| 1.4 | Read `cli/src/output/repo_json.rs` to see how existing commands return `BuildOutcome { value, exit_code }`. | Know how to emit JSON while controlling exit code. |
| 1.5 | Read `cli/src/perf.rs` to confirm `handle_no_results()` signature and behavior. | Know how to wire `--no-error` and `--on-error`. |
| 1.6 | Read `cli/src/args/repo.rs` to see existing `RepoSubcommand` and `RepoAction` variants. | Know naming conventions and derive attributes to use. |
| 1.7 | Read `cli/tests/cli.rs` to find an existing fast-path repo command test to use as a template. | Have a copy-pasteable integration test skeleton. |

> **Parallelizable**: Steps 1.1–1.7 are independent reads and can be done in any order.

**Validation Checkpoint 1**: Document the exact field names and types from `GitInfo` / `WorktreeInfo` that will be used to extract the worktree name. Confirm `handle_no_results()` supports custom exit-code behavior.

---

## Phase 2: Library Helper (Early-Return Function)

**Goal**: Expose a clean, testable helper in `sniff-lib` that returns the worktree name or an error, without triggering full detection.

| Step | Task | Observable Outcome |
|------|------|-------------------|
| 2.1 | Add `get_current_worktree_name(cwd: &Path) -> Result<Option<String>, Box<dyn Error>>` to `lib/src/filesystem/git/worktree.rs` (create file if absent). | Function compiles and is re-exported in `lib/src/filesystem/git/mod.rs`. |
| 2.2 | Implement logic: call `git2::Repository::discover(cwd)`, check if worktree, iterate worktrees, match current path, return `Some(base_dir_name)`. Return `None` for main worktree or non-repo. | Unit tests pass (see 2.3). |
| 2.3 | Add unit tests in `lib/src/filesystem/git/worktree.rs` covering: (a) inside linked worktree, (b) inside main worktree, (c) outside any repo. | `cargo test -p sniff-lib worktree` passes. |

**Validation Checkpoint 2**: Run `cargo test -p sniff-lib worktree` and confirm all 3 test cases pass.

---

## Phase 3: CLI Implementation

**Goal**: Wire the library helper into the CLI argument parser, dispatcher, text renderer, and JSON builder.

### 3A – Argument Parsing

| Step | Task | Observable Outcome |
|------|------|-------------------|
| 3.1 | Add `Worktree { no_error: bool, on_error: Option<String> }` variant to `RepoSubcommand` in `cli/src/args/repo.rs`. | `cargo check -p sniff-cli` passes. |
| 3.2 | Add `Worktree { no_error: bool, on_error: Option<String> }` variant to `RepoAction` in `cli/src/args/repo.rs`. | Compiles. |
| 3.3 | Wire the conversion from `RepoSubcommand::Worktree` to `RepoAction::Worktree` in `cli/src/args/mod.rs` (`Commands::to_repo_action()`). | Compiles. |

### 3B – Dispatch & Output

| Step | Task | Observable Outcome |
|------|------|-------------------|
| 3.4 | Add `RepoAction::Worktree` arm in `cli/src/commands/mod.rs`. Call `sniff::filesystem::git::get_current_worktree_name(&cwd)?`. On `Some(name)`, print name (or JSON) and exit 0. On `None`, call `perf::handle_no_results(no_error, on_error, plain, perf)` and return. | `cargo check -p sniff-cli` passes. |
| 3.5 | Add `build_worktree(name: Option<&str>) -> BuildOutcome` in `cli/src/output/repo_json.rs` returning `{ "worktree": "name" }` or `{ "worktree": null }`. | Compiles. |
| 3.6 | Wire `build_worktree` into `build_with_outcome()` match arm in `cli/src/output/repo_json.rs`. | Compiles. |
| 3.7 | Verify `cli/src/output/mod.rs` routes `OutputFilter::Repo` to the correct text renderer (plain string emission is sufficient; no special table/list needed). | `cargo check -p sniff-cli` passes. |

> **Parallelizable**: 3A (argument parsing) and 3B (output builders) can be drafted concurrently, but 3B depends on 3A for the `RepoAction` variant to exist.

**Validation Checkpoint 3**: Run `cargo check -p sniff-cli` with zero errors.

---

## Phase 4: Testing & Final Validation

**Goal**: Prove correctness via automated tests and manual edge-case verification.

| Step | Task | Observable Outcome |
|------|------|-------------------|
| 4.1 | Add arg-parsing tests in `cli/src/args/mod.rs` for `repo worktree`, `repo worktree --no-error`, and `repo worktree --on-error "msg"`. | Tests pass. |
| 4.2 | Add CLI integration tests in `cli/tests/cli.rs`: (a) inside linked worktree → stdout contains worktree name, exit 0; (b) inside main worktree → exit 1, no stdout; (c) `--no-error` → exit 0; (d) `--on-error` → stderr contains message, exit 1; (e) `--json` success → valid JSON with worktree field; (f) `--json --no-error` failure → `{ "worktree": null }`, exit 0. | `cargo test -p sniff-cli` passes. |
| 4.3 | Run `cargo test -p sniff-lib -p sniff-cli` to ensure no regressions in existing tests. | All existing tests pass. |
| 4.4 | Manual verification: run `sniff repo worktree` in the current repo (if in a worktree) and confirm output matches `pwd` basename. | Human confirms correct name. |
| 4.5 | Manual verification: run `sniff repo worktree` in the repo root (main worktree) and confirm exit code 1 with no output. | Human confirms failure behavior. |
| 4.6 | Run `cargo clippy -p sniff-cli -p sniff-lib` and fix any warnings. | Zero clippy warnings. |

**Validation Checkpoint 4**: All automated tests pass and manual checks confirm spec compliance.

---

## Dependency Graph

```
Phase 1 (Discovery)
    │
    ▼
Phase 2 (Library Helper)
    │
    ▼
Phase 3A (Argument Parsing) ──► Phase 3B (Dispatch & Output)
    │                                    │
    └────────────────────────────────────┘
                    │
                    ▼
            Phase 4 (Testing)
```

## Parallelizable Work Summary

- **Phase 1**: All reading steps (1.1–1.7) are independent.
- **Phase 3**: 3A (parsing) and 3B (output builders) can be drafted in parallel branches, but must be integrated before 3B compiles.

## Exit Criteria

- [ ] `sniff repo worktree` prints the linked worktree base directory name and exits 0 when inside a linked worktree.
- [ ] `sniff repo worktree` exits 1 with no output when in the main worktree or outside a Git repo.
- [ ] `--no-error` flips exit code to 0 in failure cases.
- [ ] `--on-error <msg>` prints `<msg>` to stderr in failure cases.
- [ ] `--json` produces `{ "worktree": "name" }` on success and `{ "worktree": null }` on failure.
- [ ] All new and existing tests pass.
- [ ] `cargo clippy` is clean.