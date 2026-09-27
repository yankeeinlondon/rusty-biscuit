---
spec: /Volumes/coding/wt/rusty-biscuit/fix-sniff/sniff/fixes/2026-09-25-recent-commits/spec.md
plan: sniff/fixes/2026-09-25-recent-commits/plan.md
implemented_by: claude/opus
started_phase: 1
source_files_during_phase_1: []
docs_updated_during_phase_1:
  - sniff/fixes/2026-09-25-recent-commits/plan.md
docs_created_during_phase_1:
  - sniff/fixes/2026-09-25-recent-commits/implementation-log.md
skills_files_updated_during_phase_1: []
source_files_during_phase_2:
  - sniff/lib/src/filesystem/git/recent_commits/collect.rs
  - sniff/lib/src/filesystem/repo/ownership.rs
  - sniff/lib/src/filesystem/repo/types.rs
  - sniff/lib/tests/l1/recent_commits.rs
  - sniff/cli/tests/l1/cli.rs
docs_updated_during_phase_2:
  - sniff/docs/topics/repo/recent-commits.md
  - sniff/docs/topics/repo/recent-commits-schema.md
  - sniff/docs/cli/repo_recent-commits.md
  - sniff/cli/README.md
  - sniff/lib/README.md
  - sniff/fixes/2026-09-25-recent-commits/plan.md
  - sniff/fixes/2026-09-25-recent-commits/implementation-log.md
docs_created_during_phase_2: []
skills_files_updated_during_phase_2:
  - .claude/skills/sniff/architecture.md
source_files_during_phase_3: []
docs_updated_during_phase_3:
  - sniff/fixes/2026-09-25-recent-commits/plan.md
  - sniff/fixes/2026-09-25-recent-commits/implementation-log.md
  - sniff/fixes/2026-09-25-recent-commits/spec.md
docs_created_during_phase_3: []
skills_files_updated_during_phase_3: []
source_code:
  - sniff/lib/src/filesystem/git/recent_commits/collect.rs
  - sniff/lib/src/filesystem/repo/ownership.rs
  - sniff/lib/src/filesystem/repo/types.rs
  - sniff/lib/tests/l1/recent_commits.rs
  - sniff/cli/tests/l1/cli.rs
documentation:
  - sniff/docs/topics/repo/recent-commits.md
  - sniff/docs/topics/repo/recent-commits-schema.md
  - sniff/docs/cli/repo_recent-commits.md
  - sniff/cli/README.md
  - sniff/lib/README.md
  - .claude/skills/sniff/architecture.md
  - sniff/fixes/2026-09-25-recent-commits/plan.md
  - sniff/fixes/2026-09-25-recent-commits/implementation-log.md
completed_phase: 3
implemented: true
packages:
  - sniff
  - sniff-cli
---

# Implementation Log for 2026-09-25-recent-commits (3 phases)

## Phase 1

Phase 1 is Wave 1 (boundary spike and regression blueprint). The plan forbids
production edits in this wave, so no source or test files were changed. Both
deliverables are recorded in the plan under "Wave 1 Decision Record" and
"Wave 1 Regression Blueprint".

### Reproduction

- `sniff repo recent-commits 200 --package-area worktree --json` against this
  repository: 136 commits, `1634e6e` absent. `--package worktree`: 49
  commits, `1634e6e` absent (correct). Unfiltered newest 60 contain
  `1634e6e` with `packages: []`, `package_areas: []`.

### Spike findings

- `PackageCatalog::matches` and `attribute` (`recent_commits/collect.rs`)
  derive areas only from the owning package, which is the root cause the
  spec names.
- `RepoInfo::package_area_for_dir_with_index` has an area-directory fallback,
  but it cannot be reused per file as-is: `HashSet` iteration makes nested
  areas nondeterministic (`claudine/rendezvous` vs `claudine`), it allocates
  per call, and `Path::starts_with` accepts a file whose path equals an area
  directory.
- `package_area` is always the parent of `relative` (`make_package_area`), so
  an area directory contains its packages.
- Real overlaps in this repository: `darkmatter/dmls` and `tabby` are both
  package directories and area directories. The existing library fixture has
  the same shape (`crates/alpha`).
- A root `[package]` next to `[workspace]` is not a catalog entry (checked in
  a disposable repository), so package-first attribution cannot let a root
  package absorb shared area files.
- Windows: `PathBuf::push` onto a `\\?\` verbatim root re-parses a
  `/`-separated Git path, which is why ownership lookups already work there.
  An area index built the same way inherits that. Phase 3 must still get
  runtime evidence on native Windows.

### Ruling

The plan's proposed ruling is confirmed: package-first area for owned files,
otherwise the deepest recognized non-empty area directory strictly containing
the path. One resolver serves both filtering (`area == A || area starts with
A/`) and attribution, so they cannot disagree. The seam is a crate-private
area index in `repo/ownership.rs`, used by `PackageCatalog` and by
`package_area_for_dir_with_index` in place of its `HashSet` fallback. The one
remaining ambiguity is non-blocking: files of a package whose directory is
also an area stay in the package's declared parent area.

### Gates

- `just test` (sniff): 2894 passed, 32 skipped.
- `just lint` (sniff): clean.
- These runs are a baseline only. No code changed, so there is no
  requirement-to-test mapping to run yet. The blueprint maps each acceptance
  criterion to planned Phase 2 tests.

## Phase 2

Phase 2 is Wave 2 (library and CLI regressions) and Wave 3 (documentation and
integration).

### Library correction

- Regressions written first and run against the unchanged implementation:
  10 of 16 targeted `recent_commits::{attribution,area_membership}` tests
  failed (for example the `1634e6e`-shaped C3 was absent from
  `--package-area worktree`, and C3 reported `package_areas: []`).
- `repo/ownership.rs`: new crate-private `PackageAreaIndex` (non-empty area
  directories keyed as `normalize_path(root.join(area))`, lookup walks
  `ancestors()` of a directory, so the first hit is the deepest), and
  `PackageOwnershipIndex::normalize_relative` so a path is normalized once
  for both lookups.
- `recent_commits/collect.rs`: `PackageCatalog` holds both indices and the
  lower-cased area selector; one `resolve(path) -> (owner, area)` feeds
  `matches` and `attribute`. Owned paths take the owner's `package_area`;
  unowned paths take the deepest area strictly containing them (lookup
  starts at the path's parent). `area_selects` preserves the old
  `area == A || area starts with "A/"` ASCII case-insensitive rule without
  allocating. `UnknownPackageArea` validation is unchanged.
- `repo/types.rs`: `package_area_for_dir_with_index` replaces its `HashSet`
  fallback with `PackageAreaIndex` (inclusive lookup, since callers pass
  directories); public signature unchanged. Its rustdoc now says "deepest".
- Old-contract test renamed to
  `monorepo_commits_carry_deepest_package_arrays_and_shared_area_files_report_only_their_area`
  and re-expected (`crates/README.md` → `[]`/`["crates"]`); the filter test
  was renamed to
  `package_filters_use_the_deepest_owner_and_area_filters_use_the_attributed_area`
  and `package_area("crates")` now includes "area root file".

### CLI regression

- `sniff/cli/tests/l1/cli.rs`: `create_area_membership_repo` (C0–C8 plus the
  mixed commit as C9, explicit times through the existing `commit_files_at`)
  and `test_recent_commits_package_area_selects_shared_area_files_by_location`.
  Verified red against the pre-fix `collect.rs` (count-5 area selection
  returned 3 hashes instead of C9, C8, C7, C6, C3), then green with the fix.
- No CLI source changed: the CLI only reports what the library resolves.

### Documentation

- `docs/topics/repo/recent-commits.md` (Package Attribution rewritten: package
  vs area directories, deepest nested area, root area, scope/look-alike
  negatives, lexical historical paths, count-of-matches, full file lists).
- `docs/topics/repo/recent-commits-schema.md` (`package_areas` field text,
  attribution rule, shared-file JSON example).
- `docs/cli/repo_recent-commits.md` (Package Scoping bullets).
- `cli/README.md`, `lib/README.md` (short behavior notes), and
  `.claude/skills/sniff/architecture.md` (one resolver, `PackageAreaIndex`).

### Integration safeguards

- `test_repo_aggregate_attributes_shared_area_files_like_the_focused_command`
  (CLI): aggregate `recent_commits` equals focused `recent-commits --json` on
  the area fixture (the aggregate uses `collect_observed`), the planning
  commit reports `[]`/`["worktree"]` there, and the `documentation_changes`
  projection keeps that attribution.
- The count-5 library test now asserts `git.file_diffs == 5`: survivors
  change six files, but a content-identical move needs no content diff, and
  the walked nonmatching C13/C9 are never diffed. Found while writing the
  test: an earlier "one diff per survivor file" assertion was wrong for that
  reason.
- Existing counter tests unchanged and passing:
  `attribution_uses_the_structure_tier_without_inventory_language_or_doc_walks`
  (now exercising the new area path),
  `aggregate_view` counter test (no extra manifest parses), and the new
  `area_filtering_adds_no_per_path_canonicalization`.

### Requirement-to-test mapping

| Requirement | Test(s) |
|---|---|
| Planning-only commit (`1634e6e` shape) matches `--package-area`, not `--package` | lib `area_membership::planning_only_commit_matches_its_area_but_not_its_package`; CLI `test_recent_commits_package_area_selects_shared_area_files_by_location` |
| README, reviews, features, moved and deleted shared files match | lib `count_is_filled_with_area_matches_across_nonmatching_commits` (C6, C7, C10, C11 via source path, C12 deleted) |
| Library vs CLI package distinction | lib `planning_only...` (`worktree` → C1, C0; `worktree-cli` → C8, C0); CLI test |
| Scope and look-alike paths confer nothing | lib `commit_scopes_and_path_name_matches_confer_no_area_membership`, `attribution::a_file_named_like_an_area_directory_or_a_prefix_sibling_is_outside_the_area` |
| Shared file attribution `[]`/`[area]`; filtered == unfiltered records | lib `shared_area_files_are_attributed_to_the_area_and_filtering_keeps_whole_records`; CLI tests (unfiltered and aggregate) |
| Count means matches; exhaustion; exact order | lib count test (count 5 and count 50 with `WORKTREE`); CLI count 5 / 50 |
| Mixed-area survivor keeps foreign files and attribution | lib shared-attribution test (C10); CLI test (C9) |
| AND semantics with different files | lib `package_and_area_filters_and_together_even_when_different_files_satisfy_them` |
| Nested areas deterministic, parent selects nested | lib `attribution::unowned_files_in_nested_areas_resolve_to_the_deepest_area_deterministically`; unit `types::an_unowned_directory_in_nested_areas_resolves_to_the_deepest_area`, `ownership::area_lookup_chooses_the_deepest_area_and_never_the_root_area` |
| Package-owned file keeps declared area | lib `attribution::a_package_owned_file_keeps_its_package_area_even_inside_another_area_directory` |
| Empty root area never a fallback | lib `attribution::the_empty_root_area_is_reached_only_through_a_top_level_package`; unit ownership test |
| Old contract re-expected | lib `monorepo_commits_carry_deepest_package_arrays_and_shared_area_files_report_only_their_area`, `package_filters_use_the_deepest_owner_and_area_filters_use_the_attributed_area` |
| No per-path canonicalization / survivors-only diffs | lib `area_filtering_adds_no_per_path_canonicalization`, count test `git.file_diffs == 5` |
| Aggregate agrees with focused | CLI `test_repo_aggregate_attributes_shared_area_files_like_the_focused_command` |

All new tests are L1 (no tier markers), in declared targets (`sniff` `l1`
binary via `tests/l1/recent_commits.rs`, `sniff-cli` `l1` via
`tests/l1/cli.rs`, and `#[cfg(test)]` unit modules). `just
check-tier-coverage sniff`: 0 stranded.

### Gates

- `just test` (sniff): 2908 passed, 32 skipped (baseline 2894; +14 tests).
- `just lint` (sniff): clean. `cargo clippy -p sniff --all-targets -- -D
  warnings`, the same with `--features remote`, and `cargo clippy -p
  sniff-cli --all-targets -- -D warnings`: clean (one `collapsible_if` fixed).
- No pre-existing failures. No cross-OS run in this phase: runtime evidence on
  Linux, native Windows, and WSL2 is Phase 3 Wave 4 work. The new index uses
  the same `normalize_path(root.join(..))` keying as the ownership index,
  which is why Windows risk is expected to be low, but that is unverified.

## Phase 3

Phase 3 is Wave 4 (platform validation) and Wave 5 (acceptance audit). No
source, test, or product documentation changed. The only edits are to this
log, the plan, and the spec frontmatter.

### Local verification (macOS)

- Focused nextest (`-p sniff -p sniff-cli --features sniff/remote`, filter
  `recent_commits|area_lookup|nested_areas|area_membership|shared_area`):
  154 passed.
- `just test` (sniff): 2908 passed, 32 skipped. Same as the Phase 2 close.
- `just lint` (sniff): clean.
- `cargo clippy -p sniff --all-targets -- -D warnings`, the same with
  `--features remote`, and `cargo clippy -p sniff-cli --all-targets -- -D
  warnings`: clean.
- Doctests (`cargo test -p sniff --features remote --doc`): 96 passed, 22
  ignored.
- `just check-tier-coverage sniff`: 0 stranded.
- `cargo fmt --check -p sniff -p sniff-cli` (check only; nothing
  reformatted, per the no-formatting rule): the check reports drift in 42 files
  across both packages. Most of those files were never touched by this
  fix. In the five changed files, rustfmt reports this many diffs against
  pre-fix `012858ffc`:
  `collect.rs` 1→2, `ownership.rs` 0→2, `types.rs` 2→2,
  `cli/tests/l1/cli.rs` 34→42, `lib/tests/l1/recent_commits.rs` 48→77.
  This belongs to the separate formatting pass.
- No rendering changed, so `just test-l2` was not required and was not run.

### Platform evidence

`just cross-check sniff --os all` then `just cross-check sniff-cli --os all`,
archive mode (the CI producer/consumer path), no extra features or filters.
Tested tree `2ae698bd0` is HEAD `d6ed07b48` plus the uncommitted plan
checkbox edit, so its source is identical to HEAD. Because the tree was not
HEAD's, no `wsl2-ubuntu` receipt was published.

| Host | sniff | sniff-cli |
|---|---|---|
| build-linux (ubuntu-latest key `1c3d4c912a977b09` / `cd19fd4f8980daa3`) | 2023 passed, 30 skipped | 862 passed, 7 skipped |
| build-win-native (windows-latest `900b45cbd9adc7fb` / `bf1d2aea9325f33f`) | 2012 passed, 23 skipped | 858 passed, 7 skipped |
| build-win WSL2 (consumed as wsl2-ubuntu) | 2023 passed, 30 skipped | 862 passed, 7 skipped |

On each host, every targeted test ran and passed: 19 library tests
(`recent_commits::area_membership::*`, `recent_commits::attribution::*`, and
the ownership/types area unit tests) and both CLI regressions. None were
skipped. Native Windows therefore exercised `\\?\` verbatim-root joins,
deleted historical paths (C12), moved paths (C11), and the sibling-prefix
exclusion (C5). The Windows build also emitted two unused-code warnings
(`programs/windows_apps.rs`, `executable_index.rs`). Neither file is touched
by this fix.

### Acceptance audit

- The criterion-to-test map is recorded in the plan as the "Wave 5
  Acceptance Record".
- A stale-wording scan of `sniff/lib/src`, `sniff/cli/src`, `sniff/docs`,
  both READMEs, and the sniff skill found no old-contract claims. The one
  remaining "unattributed" line (a repository-root `README.md`) matches the
  new contract.
- `012858ffc..HEAD` changes no `Cargo.toml` or `Cargo.lock`, no CLI source,
  and adds no network work.
- The sniff skill needs no update beyond Phase 2's `architecture.md` edit.
- State: implementation complete, ready for review. The spec stays in its
  current lifecycle location.

