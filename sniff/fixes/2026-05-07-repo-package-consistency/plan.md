---
phases: 6
created: 2026-05-07
start_phase: 1
source_files_during_phase_1:
  - sniff/cli/src/commands/mod.rs
docs_updated_during_phase_1: []
docs_created_during_phase_1: []
skills_files_updated_during_phase_1: []
source_files_during_phase_2:
  - sniff/cli/src/args/mod.rs
  - sniff/cli/src/args/repo.rs
  - sniff/cli/src/commands/mod.rs
  - sniff/cli/src/output/mod.rs
  - sniff/cli/src/output/repo_json.rs
docs_updated_during_phase_2: []
docs_created_during_phase_2: []
skills_files_updated_during_phase_2: []
source_files_during_phase_3:
  - sniff/cli/src/commands/mod.rs
  - sniff/cli/src/commands/repo.rs
docs_updated_during_phase_3: []
docs_created_during_phase_3: []
skills_files_updated_during_phase_3: []
packages:
  - sniff-cli
---

# Plan: Consistent `--package` and `--package-area` Flags Across `sniff repo`

Source spec: [`spec.md`](./spec.md).

## Conventions

- Working directory: repo root (`/Users/ken/.claudine/worktrees/rusty-biscuit/sniff`).
- Build/test the `sniff` area only: `just -d sniff test` and `just -d sniff lint`. Avoid `cargo` at the repo root.
- Path prefixes from resolvers always include a trailing `/` so `starts_with` filtering matches whole-segment boundaries (matches today's behavior of `resolve_package_path`).

## Summary of Targeted Changes

- **Tier 1** (replace combined `--package`): `git-status`, `unstaged-files`, `untracked-files`.
- **Tier 2** (add both flags): `structure`, `deps`, `dirty-packages`, `staged-packages`, `unstaged-packages`, `dirty-package-areas`, `staged-package-areas`, `unstaged-package-areas`.
- **Tier 3** (add `--package`): `packages`, `package-areas`.
- **`-p` short**: add `short` to `--package` in `FileListArgs` (covers `staged-files`, `dirty-files`, `dirty-source-code`, `staged-source-code`, `unstaged-source-code`).
- **No-op**: `recent-commits`, `source-code-changes`, `documentation-changes`, `blast-radius`.

---

## Phase 1 — Resolver helpers and shared filter utility

Goal: introduce explicit name/area resolvers and a small intersection helper. No CLI behavior changes yet.

### Steps

1. In `sniff/cli/src/commands/mod.rs`:
   - Replace the combined `resolve_package_path` with two helpers:
     - `fn resolve_package_path(result: &SniffResult, name: &str) -> Result<String, Box<dyn std::error::Error>>` — exact case-insensitive match on `Package.name`. On failure, error lists valid package names only.
     - `fn resolve_package_area_path(result: &SniffResult, area: &str) -> Result<String, Box<dyn std::error::Error>>` — case-insensitive **prefix** match on `Package.package_area`. Returns the supplied area lowered to the canonical form found in the package list, with trailing `/`. On failure, error lists valid package areas only.
   - Add `fn resolve_package_and_area(result: &SniffResult, package: Option<&str>, area: Option<&str>) -> Result<Option<String>, Box<dyn std::error::Error>>` that:
     - Returns `Ok(None)` when both inputs are `None`.
     - Resolves whichever inputs are `Some`.
     - When **both** are `Some`, hard-errors when the resolved package path does **not** start with the resolved area path (e.g., `error: Package 'sniff-lib' is in area 'sniff', not 'homelab'`).
     - When both resolve and overlap, returns the **narrower** prefix (the package path).
     - Each path returned ends with a trailing `/`.

2. Update the only existing caller (`git-status` block around `commands/mod.rs:772`) to call `resolve_package_path` with the new exact-match semantics so the workspace still compiles. (Combined fallback behavior is removed here.)

### Validation

- `just -d sniff lint` clean.
- `just -d sniff test` passes (existing tests rely on combined behavior only via positional filter, not via `--package` on git-status with an area name).
- Manual: `cargo run -p sniff-cli -- repo git-status --package sniff-lib` produces a scoped result; passing a package-area name to `--package` now errors.

---

## Phase 2 — Arg surface updates (parallelizable across structs)

Goal: update clap definitions and `RepoAction` variants so all targeted commands carry both `package` and `package_area`. The crate must still build and all dispatch arms must wire the new fields through (defaulting to behaving as before when the new flags are `None`).

The four sub-tasks below modify disjoint regions and can be performed in parallel by independent agents, but **must all land before Phase 3 begins**:

### 2A — `FileListArgs` short flag

File: `sniff/cli/src/args/mod.rs`.

- Add `short` to the `--package` `#[arg(...)]` attribute on `FileListArgs::package` so all `FileListArgs`-based subcommands accept `-p`.

### 2B — `RepoSubcommand` (clap parse shape)

File: `sniff/cli/src/args/repo.rs`.

For each variant below, add `package: Option<String>` and/or `package_area: Option<String>` with the standard attribute pair (mirroring `RecentCommits`):

```rust
#[arg(short, long, value_name = "PKG",
      add = clap_complete::engine::ArgValueCandidates::new(repo_package_candidates))]
package: Option<String>,
#[arg(long, value_name = "AREA",
      add = clap_complete::engine::ArgValueCandidates::new(repo_package_area_candidates))]
package_area: Option<String>,
```

- Tier 1: `GitStatus` — keep `package` (already `short, long`), add `package_area`. Update doc comment to reflect "Scope to a specific package" only (no longer "or package area").
- Tier 1: `UnstagedFiles`, `UntrackedFiles` — keep existing `package` (already `short`), add `package_area`. Update doc comment.
- Tier 2: `Structure`, `Deps`, `DirtyPackages`, `StagedPackages`, `UnstagedPackages`, `DirtyPackageAreas`, `StagedPackageAreas`, `UnstagedPackageAreas` — add **both** flags.
- Tier 3: `Packages`, `PackageAreas` — keep existing `package_area` (do not duplicate), add `package` with `short, long`.

### 2C — `RepoAction` (normalized shape)

File: `sniff/cli/src/args/repo.rs`.

Mirror the new clap fields onto each affected variant of `RepoAction`. Variants needing both fields:

- `GitStatus` — add `package_area`.
- `UnstagedFiles`, `UntrackedFiles` — add `package_area` (rename current `package` semantics — it now means a true package name).
- `Structure`, `Deps`, `DirtyPackages`, `StagedPackages`, `UnstagedPackages`, `DirtyPackageAreas`, `StagedPackageAreas`, `UnstagedPackageAreas` — add `package` and `package_area`.
- `Packages`, `PackageAreas` — add `package` (already have `package_area`).

### 2D — Subcommand → `RepoAction` conversion

File: typically the `From<RepoSubcommand> for RepoAction` impl in `sniff/cli/src/args/repo.rs` (or wherever subcommand normalization happens — locate via grep `RepoAction::GitStatus` in `sniff/cli/src/args/`).

- Pass through the new fields from the parsed subcommand into the corresponding `RepoAction` variant. No semantic changes here — just wiring.

### Validation

- `just -d sniff lint` clean. Compilation must succeed even though dispatch hasn't started honoring the new fields.
- `cargo run -p sniff-cli -- repo dirty-packages --help` shows `-p, --package <PKG>` and `--package-area <AREA>`.
- `cargo run -p sniff-cli -- repo packages --help` shows both flags.
- Tab-completion candidates wired correctly (verified via `--help`).

---

## Phase 3 — Dispatch and filtering logic

Goal: the new flags actually scope output. Sequential within this phase because all changes share `commands/mod.rs` and `commands/repo.rs`.

### 3A — Tier 1: `git-status`

File: `sniff/cli/src/commands/mod.rs` (around the `package_for_git` block, ~line 772).

- Replace the single `package_for_git` extraction with extraction of both `package` and `package_area` from `RepoAction::GitStatus`.
- Use `resolve_package_and_area` (Phase 1.3) to compute a single most-restrictive `Option<String>` path prefix.
- Pass that single prefix into the existing `get_commits_for_path`, `file_changes` retain, `status.dirty` retain, `status.untracked` retain, and counts recompute. **Do not call `get_commits_for_path` twice.**
- Performance check: when both inputs are `None`, the code path must be byte-identical to today (early-return before any extra work).

### 3B — Tier 1: `unstaged-files`, `untracked-files`

Files: `sniff/cli/src/commands/mod.rs` (dispatch site that constructs `FileListArgs` for these two variants), `sniff/cli/src/commands/repo.rs` (`handle_file_list_command`).

- Stop hard-coding `package_area: None`. Pass the variant's `package` and `package_area` through into the `FileListArgs` value (or directly into `handle_file_list_command` if it can accept them another way).
- `handle_file_list_command` already uses `FileListArgs` flags, so as long as the values are wired in, behavior is consistent with the other `FileListArgs` commands.
- Confirm the existing intersection logic inside `handle_file_list_command` honors both flags. If not, plumb them through using `resolve_package_and_area` and apply `starts_with` on each retained file.

### 3C — Tier 2 & 3: package/area-list output dispatch

Files: `sniff/cli/src/commands/repo.rs` and the relevant output modules in `sniff/cli/src/output/filesystem/`.

- For each of: `Structure`, `Deps`, `DirtyPackages`, `StagedPackages`, `UnstagedPackages`, `DirtyPackageAreas`, `StagedPackageAreas`, `UnstagedPackageAreas`, `Packages`, `PackageAreas`:
  - In the dispatch site, resolve `package` and `package_area` via `resolve_package_and_area` (intersection error fires here too).
  - Pass the resolved `package` exact name and `package_area` resolved prefix into the relevant constructor on the output type.
- The existing positional `filter` path is left untouched — flags layer on top.

### Validation

- `just -d sniff test`.
- Manual smoke tests:
  - `sniff repo git-status -p sniff-lib --package-area sniff` succeeds.
  - `sniff repo git-status -p sniff-lib --package-area homelab` errors with the expected wording.
  - `sniff repo unstaged-files -p sniff-cli` lists only files under `sniff/cli/`.
  - `sniff repo dirty-packages --package-area homelab` (matches `homelab/server` etc. via prefix).

---

## Phase 4 — Output modules apply scoping (parallelizable)

Goal: each output module accepts the resolved `package` (exact `&str`) and `package_area` (prefix `&str`) and applies the AND-intersection with the positional `filter`. Each module is independent; agents may run in parallel.

### 4A — `output/filesystem/packages.rs`

- Update `DirtyPackages`, `StagedPackages`, `UnstagedPackages` constructors / from-result methods to accept `package: Option<&str>` and `package_area: Option<&str>` and retain only matching `Package` entries before formatting.
- Match `package` by exact case-insensitive `Package.name`; match `package_area` by case-insensitive prefix on `Package.package_area`.

### 4B — `output/filesystem/package_areas.rs`

- Same treatment for `DirtyPackageAreas`, `StagedPackageAreas`, `UnstagedPackageAreas`. Note that area-list outputs aggregate by area; apply `package` filter by the area of the resolved package, and `package_area` filter by prefix on the area key itself.

### 4C — `output/filesystem/repo.rs`

- Apply scoping inside the `Structure` rendering. Filter packages before grouping/rendering.

### 4D — `output/filesystem/deps.rs`

- Apply scoping to `Deps` rendering: prune packages and edges referencing pruned nodes.

### 4E — `output/repo_json.rs`

- Wire the new flags through any JSON path that emits package/area lists or structure. Same filtering rules.

### Validation

- `just -d sniff test`.
- `sniff repo structure --package-area homelab --json` returns only homelab packages.
- `sniff repo deps -p sniff-cli` produces deps view focused on that package.
- `sniff repo packages -p sniff-lib` returns exactly one package; `--package-area sniff` returns all sniff packages.

---

## Phase 5 — Integration tests

File: `sniff/cli/tests/cli.rs`.

Add `assert_cmd`-driven tests covering the matrix in spec §6:

1. `--package` returns exactly one package.
2. `--package-area homelab` matches `homelab` and `homelab/server` (prefix semantics).
3. `--package` AND `--package-area` overlap → success with intersection.
4. `--package` AND `--package-area` non-overlapping → exit non-zero with the canonical error message.
5. Unknown `--package` → error mentions valid package names.
6. Unknown `--package-area` → error mentions valid package areas.
7. Positional `filter` plus `-p` produces the AND of both.
8. `-p` short flag works on at least one `FileListArgs` command (e.g., `dirty-files -p sniff-cli`).
9. `git-status -p <area-name>` (where the value is a real area but **not** a package) → hard error (regression guard for the removed fallback).

### Validation

- `just -d sniff test` green.
- `cargo nextest run -p sniff-cli` green if available locally.

---

## Phase 6 — Documentation

Files under `sniff/docs/cli/`.

### Steps

1. Update `repo_git-status.md` to describe `--package` and `--package-area` separately, including the intersection error.
2. Create `repo_unstaged-files.md` and `repo_untracked-files.md` if missing; otherwise update them to document both flags.
3. Update `repo_packages.md` and `repo_package-areas.md` to mention the newly added `--package` flag.
4. Update Tier 2 docs (`repo_structure.md`, `repo_deps.md`, `repo_dirty-packages.md`, `repo_staged-packages.md`, `repo_unstaged-packages.md`, `repo_dirty-package-areas.md`, `repo_staged-package-areas.md`, `repo_unstaged-package-areas.md`) to mention the new flags. Skip the doc if the file does not exist and creation is out of scope; flag in the final report.
5. Cross-check the CLI README (`sniff/cli/README.md`) — refresh examples that show `--package <area-name>` against `git-status` since that pattern is now an error.

### Validation

- `just -d sniff doctest` if applicable.
- `sniff docs` lists the updated files.
- Manual `--help` output reads cleanly for each affected subcommand.

---

## Cross-cutting Validation Checkpoints

- After Phase 1: workspace builds with new helpers; existing behavior preserved.
- After Phase 2: workspace builds; `--help` shows new flags; no behavior change yet.
- After Phase 3: end-to-end scoping works for Tier 1 commands.
- After Phase 4: end-to-end scoping works for Tier 2 & 3 commands; JSON respects flags.
- After Phase 5: full integration coverage.
- After Phase 6: docs in sync.

## Out of Scope / Confirmed Excluded

- `repo hash`, `repo remote`, `repo package`, `repo package-area`, `repo package-root`, `repo package-area-root`, `repo root`, `repo is-current-package-area-dirty`, `repo package-area-has-source-code-changes`, `repo has-merge-conflict`, `repo pr`, `repo language`, `repo worktree` — package scoping has no meaning per spec §1.4.
- `recent-commits`, `source-code-changes`, `documentation-changes`, `blast-radius` — already consistent.

## Risk Notes

- **Breaking change**: `git-status --package <area>` no longer falls back to area matching. Mention prominently in commit message and any release notes.
- **Help-text drift**: doc comments on the previously combined `--package` flag say "package or package area" — they must change in lockstep with Phase 2 to avoid lying to users.
- **JSON consumers**: anyone scripting `repo_json` output should see no schema change, only narrower contents when flags are passed.
