---
$schema:
    status: |-
        enum(
            draft-spec,
            finalized-spec,
            planned,
            implemented,
            review-findings,
            human-in-the-loop,
            completed,
            on-hold,
            abandoned
        ) -> an indicator of progress for this specification
    reviewed: boolean -> indicates whether the specification file has been reviewed by another agent from the one which created the spec
    reviewed_by: string -> the agent and model used in the spec review
    reviewed_on: date -> the date the spec was reviewed
    review_iterations: number -> the number of implementation reviews have taken place in the review/fix cycle
    clarified: boolean -> indicates whether the specification was built -- _in part_ -- with the 'clarify.md' prompt
    implemented: boolean -> indicates whether this spec's plan has been implemented
    implemented_by: string -> the agent who implemented the plan
status: planned
implemented: false
implemented_by: claude/opus
review_iterations: 0
human_review: false
message_to_agent: |-
    Phase 1 changed no source. Read "Wave 1 Decision Record" and "Wave 1
    Regression Blueprint" in plan.md before coding. Key points: (1) do not
    reuse RepoInfo::package_area_for_dir_with_index as-is per file -- its
    HashSet fallback is nondeterministic for nested areas, allocates per call,
    and treats a file whose path equals an area directory as inside it;
    (2) add one crate-private area index in repo/ownership.rs built from the
    ownership index's already-canonical root, and route both
    PackageCatalog::matches and ::attribute through a single per-path resolver
    (package-first area for owned files, else deepest strictly-containing
    non-empty area); (3) switch package_area_for_dir_with_index's fallback to
    the same index (inclusive lookup for directories); (4) the existing test
    monorepo_commits_carry_deepest_package_arrays_and_area_files_stay_unattributed
    encodes the old contract and must be renamed and re-expected, not deleted.
    Baseline before Phase 2: sniff `just test` 2894 passed / 32 skipped,
    `just lint` clean. The working tree carries unrelated uncommitted edits
    from 2026-09-21-lockfile-provenance-cost; preserve them.
---

# Recent Commits: Package-Area Filtering

## Problem

`sniff repo recent-commits --package-area worktree` excludes commits that
change files inside `worktree/` but outside its individual package directories.
Planning files, reviews, and shared documentation belong to the package area
and must appear in its history.

For example, commit `1634e6e` (`planning(worktree): record execution plan for
2026-09-25-worktree-file`) changes only the plan and spec for
2026-09-25-worktree-file, under `worktree/fixes/`. It appears in unfiltered
recent commits but is omitted by `--package-area worktree`.

The area filter must include that commit.

## Required Behavior

Package and package-area filters have different boundaries:

- `--package worktree` selects commits touching the library package under
  `worktree/lib/`. A commit touching only shared planning files under
  `worktree/fixes/` correctly does not match.
- `--package-area worktree` selects commits touching any file under the
  `worktree/` area, including its library, CLI, fixes, features, README, and
  other shared files.

Area membership is determined by file location. A conventional-commit scope
such as `planning(worktree)` does not establish membership, and a file outside
`worktree/`, such as `.claude/skills/worktree/SKILL.md`, does not match merely
because its path mentions worktree.

The count means the number of matching commits: `recent-commits 5
--package-area worktree` walks history until it finds five matches or exhausts
history. A matching commit retains its full changed-file list.

## Root Cause

Recent-commit filtering first asks which individual package contains each
changed file. It then derives the package area from that package. A file
outside every package directory gets no area, even when it is clearly inside
an area directory.

The same restriction affects the reported `package_areas` values. Area
membership must be determined independently of whether a package contains the
file. A commit touching only `worktree/README.md`, for example, should report
`packages: []` and `package_areas: ["worktree"]`.

Sniff already recognizes area directories in `RepoInfo::package_area_for_dir`.
The recent-commit library should use consistent area boundaries for filtering
and attribution, while retaining package directory boundaries for package
filtering. This logic belongs in the library.

## Acceptance Criteria

- A commit changing only files under `worktree/fixes/`, as in `1634e6e`, appears
  in `--package-area worktree` results and is excluded by `--package worktree`.
- Area filtering includes commits changing only an area-root README, planning
  files, reviews, or other files outside individual package directories.
- Commits touching library or CLI files match their package area;
  package filters distinguish the individual packages.
- Files outside the selected area do not match, including similarly named
  sibling directories and external skill files.
- JSON attribution includes the area for shared area files without inventing
  an owning package. Filtering and attribution agree.
- Library and CLI regression tests cover shared area files and package files
  in the same repository, verify commit identities and ordering, and prove
  that the requested count is filled with matching commits.
- Tests and recent-commit documentation reflect these filtering and
  attribution rules.
- Area matching works on macOS, Linux, native Windows, and WSL2.
