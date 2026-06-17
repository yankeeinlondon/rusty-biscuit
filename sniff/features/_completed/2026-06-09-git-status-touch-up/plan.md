---
phases: 3
created: 2025-01-24
start_phase: 1
source_files_during_phase_1:
  - sniff/lib/src/request.rs
  - sniff/lib/src/filesystem/git/types.rs
  - sniff/lib/src/filesystem/git/remote_refresh.rs
docs_updated_during_phase_1: []
docs_created_during_phase_1: []
skills_files_updated_during_phase_1: []
source_files_during_phase_2:
  - sniff/cli/src/output/filesystem/mod.rs
  - sniff/lib/tests/git_parity.rs
docs_updated_during_phase_2: []
docs_created_during_phase_2: []
skills_files_updated_during_phase_2: []
source_files_during_phase_3:
  - sniff/cli/src/commands/mod.rs
  - sniff/cli/src/output/filesystem/mod.rs
  - sniff/cli/tests/cli.rs
docs_updated_during_phase_3:
  - sniff/docs/cli/repo_git-status.md
  - sniff/cli/CHANGELOG.md
docs_created_during_phase_3: []
skills_files_updated_during_phase_3: []
source_code:
  - sniff/lib/src/request.rs
  - sniff/lib/src/filesystem/git/types.rs
  - sniff/lib/src/filesystem/git/remote_refresh.rs
  - sniff/cli/src/output/filesystem/mod.rs
  - sniff/lib/tests/git_parity.rs
  - sniff/cli/src/commands/mod.rs
  - sniff/cli/tests/cli.rs
documentation:
  - sniff/docs/cli/repo_git-status.md
  - sniff/cli/CHANGELOG.md
packages:
  - sniff
---

# Execution Plan: `sniff repo git-status` Performance & UI Touch-up

This plan implements the performance optimizations and UI enhancements specified in `spec.md` for the `sniff repo git-status` subcommand.

## Phase 1: Library Optimization & Data Model

**Goal:** Eliminate expensive commit walks for non-current worktrees while preserving an opt-in for full detail.

- [x] **1.1** Add `full_worktree_details: bool` (default `false`) to `GitRequest` builder/config.
- [x] **1.2** Modify worktree enumeration so it distinguishes the *current* worktree from linked worktrees.
- [x] **1.3** Skip ahead/behind calculation for any linked worktree when `full_worktree_details` is `false`.
- [x] **1.4** Ensure ahead/behind is still computed for the **current** worktree relative to `main` when on a non-main branch.
- [x] **1.5** Update `GitRequest` default JSON serialization path to respect the new lazy-load behavior.
- [x] **1.6** Add unit tests for `GitRequest` covering:
  - Default (minimal) mode returns count-only for linked worktrees.
  - Full-detail mode returns ahead/behind for all worktrees.
  - Current worktree always has ahead/behind when not on `main`.

**Validation Checkpoint:** `cargo test` in `sniff/` passes; `GitRequest` tests demonstrate that commit walks are skipped for non-current worktrees in default mode.

---

## Phase 2: UI/Formatting Overhaul

**Goal:** Implement the new worktree section layouts and standardize vertical spacing/header styling.

- [x] **2.1** Implement `render_header(title)` helper that emits `<b><uu>{title}</uu></b>` with capability-aware degradation via `biscuit-terminal`.
- [x] **2.2** Implement `blank_line()` utility and wire it so every section produces exactly one blank line before and after its header.
- [x] **2.3** Implement **Case A** worktree renderer (current dir is NOT `main`):
  - Print `main:` worktree path (absolute href, relative label).
  - Print `Current Worktree:` with branch name and ahead/behind status.
  - Print `Other Worktrees:` with active count only.
- [x] **2.4** Implement **Case B** worktree renderer (current dir IS `main`):
  - Print `Current Worktree:` (main) with path.
  - Print `Other Worktrees:` with active count only.
- [x] **2.5** Apply new header styling and spacing to `Status` and `Meta` sections for consistency.
- [x] **2.6** Consolidate trailing/leading whitespace between sections to enforce exactly one blank row of separation.

> **Parallelizable:** Tasks 2.1 and 2.2 (generic formatting helpers) can be worked on in parallel with 2.3 and 2.4 (section-specific renderers) once the data model from Phase 1 is finalized.

**Validation Checkpoint:** Manual/visual inspection of `sniff repo git-status` output matches the target format in `spec.md` for both Case A and Case B.

---

## Phase 3: Integration, Performance Validation, & Edge Cases

**Goal:** Confirm the <500ms performance target, validate JSON/text parity, and harden against edge cases.

- [x] **3.1** Run `hyperfine` or `cargo bench` (or manual `time`) against a large repository (>5 worktrees, >10k commits) to verify end-to-end latency is **<500ms** in default mode.
  - **Result (review-1 follow-up):** Timed on the rusty-biscuit monorepo (17 worktrees, ~7,284 commits). Default text mode now averages **~70–85 ms** (release build), comfortably under 500 ms. The earlier ~0.8 s figure was dominated by `RepoRequest::full()` per-package language scanning plus docs/formatting detection — none of which `git-status` renders. The detection plan for `git-status` now drops repo language scanning, docs, formatting, and the file inventory (keeping only `RepoRequest::structure()` when a `--package`/`--package-area` scope is given), and the per-worktree status scan is skipped for non-current worktrees.
- [x] **3.2** Verify `sniff repo git-status --json` also completes in **<500ms** and returns minimal worktree data by default.
  - **Result (review-1 follow-up):** JSON mode averages **~68–86 ms**, consistent with text mode. JSON payload contains minimal worktree data (count-only / ahead=0 for non-current worktrees) as expected; verified end-to-end by `test_git_status_json_worktree_ahead_is_lazy_by_default`.
- [x] **3.3** Verify `sniff repo git-status --refresh-remotes` is exempt from the 500ms assertion (expected slower due to network I/O).
  - **Result:** `--refresh-remotes` averaged ~2.9 s, significantly slower as expected because it fetches remote refs over the network and performs full detail probes on all worktrees.
- [x] **3.4** Test edge cases:
  - No linked worktrees (count = 0).
  - Single linked worktree.
  - Current directory is not inside any git worktree (graceful error).
  - **Result:** Added unit tests `git_status_no_worktrees_omits_worktrees_section`, `git_status_single_linked_worktree_counts_correctly`, and CLI integration test `test_repo_git_status_outside_git_repo_is_graceful`. All pass.
- [x] **3.5** Run `cargo test` and `cargo clippy` across affected crates; fix any regressions.
  - **Result:** All tests pass (`cargo test -p sniff -p sniff-cli`). Fixed a pre-existing clippy warning (`if_same_then_else` in `commands/mod.rs`). No regressions introduced.
- [x] **3.6** Update any inline documentation, CHANGELOG, or README snippets that reference the old `git-status` output format.
  - **Result:** Updated `sniff/docs/cli/repo_git-status.md` to describe the new Case A / Case B worktree layout. Updated `sniff/cli/CHANGELOG.md` with added/changed/performance notes for this release.

**Validation Checkpoint:** All tests pass, performance benchmark recorded in PR description, and output format matches spec for both text and JSON modes.

---

## Dependency Graph Summary

```
Phase 1 ──┬──► Phase 2 ──► Phase 3
          │       │
          │       └── 2.1/2.2 (helpers) ═══╗
          │       └── 2.3/2.4 (renderers) ═╝ (parallel once model is stable)
          │
          └── 1.6 (tests) can run after 1.1–1.5
```

## Success Criteria

1. `sniff repo git-status` on a large repo completes in **<500ms** without `--refresh-remotes`.
2. Non-current worktrees show **count only** (no ahead/behind) in default text and JSON output.
3. Current worktree on a non-main branch shows **ahead/behind relative to main**.
4. Section headers use **double-underline (`<uu>`)** styling with graceful degradation.
5. Exactly **one blank line** separates every section; no double blank lines anywhere.
6. All existing and new tests pass; no clippy warnings.
