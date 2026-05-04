---
feature: incorrect-json
review: review-1.md
created: 2026-04-28
phases: 7
---

# Implementation Plan — Address `review-1.md` for `sniff repo --json`

This plan turns every gap in `review-1.md` into concrete code work. The review's verdict is **not ready** because the central routing change (Phase 2 in `plan.md`) was never landed, leaving 16+ subcommands returning the full `RepoInfo` blob and 2 commit-family commands returning unfiltered `CommitDescSet`.

The plan is organized so that **Phase 1 unblocks Phases 2–5** (independent families that can be wired in any order), and **Phase 6/7** lock the contract behind tests + lint.

## Working assumptions

- Repo root: `/Users/ken/.claudine/worktrees/rusty-biscuit/sniff`
- Sniff lib crate: `sniff/lib/` (`-p sniff`)
- Sniff CLI crate: `sniff/cli/` (`-p sniff-cli`)
- All `cargo` invocations MUST be targeted (no bare `cargo build` / `cargo test` at repo root).
- All shape decisions match `spec.md`.

---

## Phase 1 — Repo-action-aware JSON routing (foundation)

**Goal.** Give the `--json` path access to the `RepoAction` so each subcommand can pick a focused serializer instead of falling through to `OutputFilter::Repo` → full `RepoInfo`.

**Files to touch.**
- `sniff/cli/src/output/mod.rs`
- `sniff/cli/src/commands.rs`

**Specific changes.**
1. In `sniff/cli/src/output/mod.rs`:
   - Extend `print_json` signature to:
     ```rust
     pub fn print_json(
         result: &SniffResult,
         filter: OutputFilter,
         docs_filter: &DocsFilter,
         files_filter: &FilesFilter,
         repo_action: Option<&RepoAction>,
         base_dir: Option<&std::path::Path>,
     ) -> serde_json::Result<()>
     ```
   - Extend `apply_filter_to_json` to receive `repo_action` and `base_dir` and to dispatch on `RepoAction` when `filter == OutputFilter::Repo`.
   - Add a new `repo_json::build(...)` module/function (new file `sniff/cli/src/output/repo_json.rs`) that returns `serde_json::Value` for any full-detection repo action. Default branch (`None`, `Structure { .. }`, anything not yet wired) preserves today's behavior — `serde_json::to_value(&fs.repo)`.
   - Re-export the new module via `mod.rs`.
2. In `sniff/cli/src/commands.rs`:
   - Update the single call site at line 844 to pass `repo_action.as_ref()` and `base_dir.as_deref()` to `output::print_json`.
   - Leave every existing early-return JSON path untouched (file lists, `packages`, `package-areas`, `hash`, `root`, `remote`, `pr`, unstaged/untracked-files, recent-commits handler).
3. Keep `attach_performance` unchanged — call it once on the final `serde_json::Value`.

**Tests to add/update.**
- New unit tests in `sniff/cli/src/output/repo_json.rs` (or `mod.rs`) that build a tiny `SniffResult` fixture and assert:
  - `repo_action = None` → JSON is the full `RepoInfo` (object containing `is_monorepo`).
  - `repo_action = Some(Structure { .. })` → same shape as `None` (regression guard for bare `repo` / `structure`).
- Update `test_git_status_subcommand_json_output` and `test_git_status_json_contains_repo_fields` in `sniff/cli/tests/cli.rs` to reflect the new `GitInfo`-shaped output (Phase 2 will land the body — set the assertions in this PR, ignore them with `#[ignore]` until Phase 2 turns them on, or land them together).

**Verification commands.**
```
cargo build -p sniff-cli
cargo test -p sniff-cli --test cli test_repo_subcommand_json_output
cargo test -p sniff-cli output::repo_json
```

---

## Phase 2 — Wire `git-status --json` to `GitInfo`

**Goal.** `sniff repo git-status --json` returns the `GitInfo` object directly, with package scoping already applied by `commands.rs`.

**Files to touch.**
- `sniff/cli/src/output/repo_json.rs`

**Specific changes.**
1. In the new `repo_json::build`, add a match arm for `RepoAction::GitStatus { .. }`:
   - If `result.filesystem.git` is `Some(git)`, return `serde_json::to_value(git)`.
   - Otherwise return `json!({})`.
2. Do **not** touch `RepoAction::GitStatus` in `render_text`; package scoping is performed in `commands.rs` between detection and serialization (lines ~723–784) and continues to apply.

**Tests to add/update.**
- In `sniff/cli/tests/cli.rs`:
  - Replace `test_git_status_subcommand_json_output` with assertions that:
    - top-level object has `repo_root`, `status`, `recent`, `branches`
    - top-level object does **not** have `is_monorepo` or `packages` (those are `RepoInfo` fields, not `GitInfo`)
  - Update `test_git_status_json_contains_repo_fields` (currently asserts `is_monorepo` / `packages`) to assert `repo_root` instead — rename to `test_git_status_json_is_git_info`.

**Verification commands.**
```
cargo test -p sniff-cli --test cli git_status
just lint     # run from sniff/ — area-level lint
```

---

## Phase 3 — Package- and area-family JSON shapes

**Goal.** Six subcommands return `{ scope, kind, names }` instead of full `RepoInfo`.

Subcommands: `dirty-packages`, `dirty-package-areas`, `staged-packages`, `staged-package-areas`, `unstaged-packages`, `unstaged-package-areas`.

**Files to touch.**
- `sniff/cli/src/output/filesystem.rs` — expose pure name selectors.
- `sniff/cli/src/output/repo_json.rs` — new builders.

**Specific changes.**
1. In `sniff/cli/src/output/filesystem.rs`, factor the name selection that today is buried in `render_dirty_*` / `render_staged_*` / `render_unstaged_*` into pure functions returning `Vec<String>`. Suggested public-in-crate signatures (no rendered text, no error strings):
   ```rust
   pub(crate) fn select_dirty_package_names(result: &SniffResult, repo_filter: &[String]) -> Vec<String>;
   pub(crate) fn select_dirty_package_area_names(result: &SniffResult, repo_filter: &[String]) -> Vec<String>;
   pub(crate) fn select_staged_package_names(result: &SniffResult, repo_filter: &[String]) -> Vec<String>;
   pub(crate) fn select_staged_package_area_names(result: &SniffResult, repo_filter: &[String]) -> Vec<String>;
   pub(crate) fn select_unstaged_package_names(result: &SniffResult, repo_filter: &[String]) -> Vec<String>;
   pub(crate) fn select_unstaged_package_area_names(result: &SniffResult, repo_filter: &[String]) -> Vec<String>;
   ```
   Reuse the existing private `dirty_package_names`, `staged_package_names`, `unstaged_package_names`, plus `filter_packages`. Refactor `render_dirty_packages` and friends to call the new selectors, then `join(", ")`. The non-monorepo error strings stay only in the `render_*` functions.
2. In `sniff/cli/src/output/repo_json.rs`, add arms that emit:
   ```json
   { "scope": "<dirty|staged|unstaged>", "kind": "<packages|package_areas>", "names": [...] }
   ```
   - `scope` and `kind` are string literals.
   - When the repo is not a monorepo, return `{ "scope": "...", "kind": "...", "names": [] }` (empty array) — JSON consumers should not see prose error strings.
3. Honor `--package-area` / repo `filter` exactly the way text mode does (use the new selectors, which already accept `repo_filter`).

**Tests to add/update.**
- Unit tests on each selector in `filesystem.rs` (mirror the existing `filter_packages_tests` pattern).
- CLI integration test in `sniff/cli/tests/cli.rs` using `create_test_repo` + `test_stage_file`/`test_commit_file`:
  - `test_dirty_packages_json_shape` — asserts top-level keys `scope == "dirty"`, `kind == "packages"`, `names` is an array.
  - `test_staged_package_areas_json_shape` — asserts `scope == "staged"`, `kind == "package_areas"`.
  - `test_unstaged_packages_json_shape`.
  - One negative test: a non-monorepo (the existing `create_test_repo` returns one) shows `names == []` in JSON, not the prose "only intended to be used in a monorepo" string.

**Verification commands.**
```
cargo test -p sniff-cli --test cli packages_json
cargo test -p sniff-cli output::filesystem::select
```

---

## Phase 4 — Locator + boolean families

**Goal.** Eight subcommands return small focused JSON objects and continue to honor exit-code semantics.

Locators: `package-root`, `package-area-root`, `package`, `package-area`.
Booleans: `is-current-package-area-dirty`, `package-area-has-source-code-changes`, `has-merge-conflict`.

**Files to touch.**
- `sniff/cli/src/output/filesystem.rs` — split boolean helpers from their `std::process::exit` calls.
- `sniff/cli/src/output/repo_json.rs` — locator + boolean builders.
- `sniff/cli/src/commands.rs` — for the early-return `Package` / `PackageArea` arms (lines ~789–807) and `HasMergeConflict` (line ~498), branch on `cli.json` and emit JSON before exiting.

**Specific changes.**
1. In `sniff/cli/src/output/filesystem.rs`, refactor:
   - `print_current_package_area_dirty` → split into `pub(crate) fn current_package_area_is_dirty(result, base_dir) -> Option<bool>` (returns `None` when the area cannot be resolved). The existing `print_*` keeps its `std::process::exit(1)` behavior by mapping `None → exit(1)` and `Some(true|false) → exit(0|1)`.
   - `print_package_area_has_source_code_changes` → analogous `package_area_has_source_code_changes(...) -> Option<(bool, usize)>` so verbose output can keep its count message while the JSON path only consumes the `bool`.
2. In `sniff/cli/src/commands.rs`:
   - For `RepoAction::HasMergeConflict` (early-return, line ~498): when `cli.json` is true, print `{ "has_merge_conflict": <bool> }`, then `std::process::exit(if has { 0 } else { 1 })`. When `cli.verbose > 0`, keep printing conflicted paths to stderr in both JSON and text modes.
   - For `RepoAction::Package { no_error, on_error }` and `RepoAction::PackageArea { .. }` (lines ~789–807): if `cli.json` is true, build `{ "name": resolved }` (or run `handle_no_results` when unresolved). The existing `handle_no_results` semantics remain authoritative for missing values.
3. In `sniff/cli/src/output/repo_json.rs` add builders for:
   - `RepoAction::PackageRoot` → `{ "root": render_repo_package_root(result, base_dir) }` (omit when empty? — emit `{ "root": "" }` is fine; downstream scripts can check). Match text-mode behavior: emit JSON even when path is empty, but the orchestration in `commands.rs` should still `exit(1)` on empty (mirror the text path at line ~512–519). Implement by returning `Option<Value>` from the builder; `commands.rs`'s JSON path falls back to `handle_no_results`-style exit when `None`.
   - `RepoAction::PackageAreaRoot` → analogous.
   - `RepoAction::IsCurrentPackageAreaDirty` → `{ "dirty": bool }`. Use the new pure helper. JSON path: print, then `exit(if dirty { 0 } else { 1 })`. Wire from `render_text`'s `Repo` arm so we don't break text mode, but JSON dispatch happens via the new repo-aware print path.
   - `RepoAction::PackageAreaHasSourceCodeChanges` → `{ "has_source_code_changes": bool }`.

   Because boolean and locator commands need to call `std::process::exit` after emitting JSON, the cleanest path is:
   - Have `repo_json::build` return `BuildOutcome { value: Value, exit_code: Option<i32> }` for these arms.
   - `commands.rs`'s JSON branch checks `exit_code` and exits after `attach_performance` + `println!`.

   Alternatively, keep `print_json` returning `Result<()>` and let it call `exit` itself for these specific arms. Pick whichever yields the smallest diff; document the choice in code comments.

**Tests to add/update.**
- Unit tests on the new pure helpers (`current_package_area_is_dirty`, `package_area_has_source_code_changes`).
- CLI integration tests in `sniff/cli/tests/cli.rs`:
  - `test_has_merge_conflict_json_false` — clean repo → exit 1, stdout has `{"has_merge_conflict": false}`.
  - `test_is_current_package_area_dirty_json` — modify a file → exit 0, stdout has `{"dirty": true}`.
  - `test_package_root_json_when_present` — assert top-level `root` key matching `path`.
  - `test_package_root_json_when_absent` — outside any package, exit 1, no JSON to stdout (matches existing `handle_no_results` behavior).
  - `test_package_name_json` — `{"name": "<pkg>"}`.

**Verification commands.**
```
cargo test -p sniff-cli --test cli locator
cargo test -p sniff-cli --test cli merge_conflict
cargo test -p sniff-cli --test cli is_current_package_area_dirty
```

---

## Phase 5 — `deps --json` builder

**Goal.** `sniff repo deps --json` returns `{ "packages": [ { name, depends_on, used_by, dependencies, dev_dependencies, peer_dependencies?, optional_dependencies? } ] }`.

**Files to touch.**
- `sniff/cli/src/output/repo_json.rs`
- (Possibly read-only) `sniff/cli/src/output/filesystem.rs` to reuse `filter_packages` and the existing `render_repo_deps_text` selection logic — but do NOT parse text.

**Specific changes.**
1. Add `pub(crate) fn build_deps_value(repo: &RepoInfo, repo_filter: &[String]) -> serde_json::Value`:
   - `let packages = repo.packages.as_deref().unwrap_or(&[]);`
   - `let filtered = filter_packages(packages, repo_filter);`
   - For each filtered `Package`, build a `serde_json::Map` with at minimum:
     - `name`: `pkg.name`
     - `depends_on`: `pkg.depends_on`
     - `used_by`: `pkg.used_by`
     - `dependencies`: `pkg.dependencies` (Vec<DependencyEntry> already serializes correctly; existing `#[serde(skip_serializing_if = "Option::is_none")]` and `Vec::is_empty` semantics on `DependencyEntry` are preserved).
     - `dev_dependencies`: `pkg.dev_dependencies`
     - `peer_dependencies`: include only when non-empty.
     - `optional_dependencies`: include only when non-empty.
   - Wrap in `{ "packages": [...] }` so the top-level shape is an object (lets `attach_performance` add `performance` without wrapping under `data`).
2. Wire into `repo_json::build` for `RepoAction::Deps { ui: _, filter }` — the `ui` flag is text-only and is ignored in JSON mode.
3. Confirm we serialize `Package` fields by hand (not via `serde_json::to_value(pkg)`), so we don't accidentally leak unrelated fields like `path`, `documentation`, `languages`. Reference `Package` struct in `sniff/lib/src/filesystem/repo/types.rs` lines 171–230 to make sure the allowlist is correct.

**Tests to add/update.**
- Unit test in `repo_json.rs` constructing a tiny `RepoInfo` fixture with two packages, asserting:
  - top-level `packages` array length == 2
  - each entry has `name`, `depends_on`, `used_by`, `dependencies`, `dev_dependencies`
  - entries with empty `peer_dependencies` do NOT include the key
  - `path`, `languages`, `documentation` are NOT present
- CLI test `test_repo_deps_json_shape` using the temp repo helper to assert top-level `packages` exists.

**Verification commands.**
```
cargo test -p sniff-cli output::repo_json::deps
cargo test -p sniff-cli --test cli deps_json
```

---

## Phase 6 — Filter commit-family JSON (`source-code-changes`, `documentation-changes`)

**Goal.** `source-code-changes --json` and `documentation-changes --json` apply the same per-commit / per-file filtering that styled and plain text apply. `recent-commits --json` is unchanged.

**Files to touch.**
- `sniff/cli/src/output/commit_blocks.rs` — promote / expose a pure filter helper.
- `sniff/cli/src/output/recent_commits.rs` — apply the filter before `serde_json::to_string_pretty`.

**Specific changes.**
1. In `sniff/cli/src/output/commit_blocks.rs`, expose:
   ```rust
   pub(crate) fn filter_commit_set(set: &CommitDescSet, filter: CommitCentricFilter) -> CommitDescSet
   ```
   - Iterate `set.commits`, keep each commit whose files have at least one match per `filter.file_matches`.
   - For each kept commit, replace `commit.files` with the filtered subset.
   - Preserve `set.period_label` and `set.repo_root`.
2. In `sniff/cli/src/output/recent_commits.rs::handle_recent_commits_command`, in the `if json { ... }` branch (line 83):
   - For `RecentCommitsMode::RecentCommits`: serialize `commit_set` directly (today's behavior).
   - For `SourceCodeChanges` / `DocumentationChanges`:
     - Run `let filtered = filter_commit_set(&commit_set, CommitCentricFilter::SourceCode|Documentation);`
     - If `filtered.commits.is_empty()`, route through `handle_no_results` like the text path does.
     - Build the final JSON manually so we can add the `"filter"` field:
       ```rust
       let mut value = serde_json::to_value(&filtered)?;
       if let Some(obj) = value.as_object_mut() {
           obj.insert("filter".into(), serde_json::json!(filter_label));
       }
       println!("{}", serde_json::to_string_pretty(&value)?);
       ```
     - `filter_label` is `"source_code"` or `"documentation"`.
3. Apply `--package`, `--package-area`, `--action` filters BEFORE the source/docs filter (today's `commit_set.filter_by_*` calls already run before the JSON branch — keep them where they are).

**Tests to add/update.**
- New unit test in `commit_blocks.rs` (or a new test module): construct a `CommitDescSet` with one commit touching `["src/main.rs", "README.md"]` and one touching only `["README.md"]`, then:
  - `filter_commit_set(_, SourceCode)` → 1 commit, `files == ["src/main.rs"]`.
  - `filter_commit_set(_, Documentation)` → 2 commits, all files are `README.md`.
  - `filter_commit_set(_, All)` → identical to input.
- CLI integration test using a temp repo with two commits (one touches `src/main.rs`, one touches `README.md`):
  - `test_source_code_changes_json_filters_commits_and_files`
  - `test_documentation_changes_json_filters_commits_and_files`
  - `test_recent_commits_json_unchanged` (regression — ensures `--filter` field is NOT added).

**Verification commands.**
```
cargo test -p sniff-cli --test cli source_code_changes_json
cargo test -p sniff-cli --test cli documentation_changes_json
cargo test -p sniff-cli output::commit_blocks::filter_commit_set
```

---

## Phase 7 — End-to-end regression guard, lint, and docs

**Goal.** Lock the contract from `spec.md` so future changes can't silently regress to the "all commands return the same blob" failure mode.

**Files to touch.**
- `sniff/cli/tests/cli.rs` — add the regression matrix.
- `sniff/cli/README.md` and any `sniff/docs/cli/repo*.md` pages whose JSON examples are now wrong.

**Specific changes.**
1. Add `test_repo_subcommand_json_shapes_are_distinct` to `sniff/cli/tests/cli.rs`:
   - Build a temp repo (use `create_test_repo` + a couple commits).
   - For each subcommand in:
     ```
     git-status, deps, dirty-packages, dirty-package-areas, staged-packages,
     staged-package-areas, unstaged-packages, unstaged-package-areas,
     package-root, package-area-root, package, package-area,
     is-current-package-area-dirty, package-area-has-source-code-changes,
     has-merge-conflict, source-code-changes, documentation-changes
     ```
     run `sniff --base <path> repo <sub> --json`, capture stdout, hash with `xxhash` or just store the raw string.
     - Some commands exit 1 on no results — accept either exit code; capture stdout.
   - Assert no two stdout payloads are equal.
   - Allow `structure` and bare `repo` to match (skip those from the distinctness assertion).
2. Add a `--perf` smoke test for one object output (`git-status --json --perf`) and one numeric/scalar-bearing output (`is-current-package-area-dirty --json --perf`), asserting `performance` shows up at the top level.
3. Update `sniff/cli/README.md` JSON examples for `git-status`, `deps`, `dirty-packages`, etc. (only the JSON snippets — text examples are unchanged).
4. Spot-check `sniff/docs/cli/repo_*.md` — add or replace JSON example blocks to match the spec shapes.

**Verification commands.**
```
# focused
cargo test -p sniff-cli --test cli json_shapes_are_distinct
cargo test -p sniff-cli --test cli perf

# full sniff area
cargo test -p sniff -p sniff-cli
just lint    # run from sniff/

# snapshot check (only if any insta snapshots flipped)
cargo test -p sniff-cli --test snapshots
```

---

## Cross-cutting acceptance gate

After Phase 7, all of the following must hold:

1. **All sniff tests pass.**
   ```
   cargo test -p sniff -p sniff-cli
   ```
2. **Zero lint warnings/errors in the sniff area.**
   ```
   cd sniff && just lint
   ```
   (Equivalent direct call: `cargo clippy -p sniff -p sniff-cli --all-targets -- -D warnings && cargo fmt -p sniff -p sniff-cli --check`.)
3. **Every spec acceptance criterion** in `sniff/features/2026-04-28-incorrect-json/spec.md` (numbered 1–9) is verifiable by an automated test added in Phases 2–7.
4. **No regression** in already-correct subcommands: file-list, `packages`, `package-areas`, `hash`, `root`, `remote`, `pr`, `unstaged-files`, `untracked-files`, `recent-commits`, bare `repo`, `structure`. The Phase 7 distinctness test plus existing `test_repo_staged_files_json_uses_new_shape` cover this.

---

## Risks and unknowns

- **Boolean commands and `std::process::exit` after JSON.** `print_json` currently doesn't expect to exit. The Phase 4 `BuildOutcome` shape (or letting the JSON branch in `commands.rs` exit after `println!`) needs a concrete decision when implementing — both work, the former is more testable.
- **`Package` field allowlist for `deps`.** The struct has many fields. Phase 5 deliberately hand-builds the JSON to keep the contract narrow; this means future fields on `Package` won't auto-leak into `deps --json`. This is a feature, not a bug, but is worth a code comment.
- **Insta snapshots.** Phase 7 may flip some text-mode snapshots if the author of the original feature embedded JSON snippets in markdown snapshots. Re-run `cargo insta review` only for snapshots that are clearly the new expected JSON; otherwise treat snapshot changes as a regression to investigate.
- **Plain mode interaction.** None of the new JSON shapes should depend on terminal/`Prose` rendering. Phase 3's selectors ensure this — re-confirm during code review.
- **Performance attachment with array outputs.** All new shapes are objects, so `attach_performance` no longer needs the `data` wrapper for the affected commands. Existing tests on `attach_performance` continue to cover the array-wrapping case via the file-list early return paths.
