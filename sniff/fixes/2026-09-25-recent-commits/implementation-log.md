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
packages: []
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
