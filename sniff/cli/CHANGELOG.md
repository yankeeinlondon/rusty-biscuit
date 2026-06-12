# Changelog

All notable changes to this project will be documented in this file.

## [Unreleased]

### Added

- `sniff repo worktrees` — list all worktrees in the current repository, including the main worktree. Supports `--list`, `--csv`, `--verbose`, `--json`, and `--plain` output modes. The current worktree is marked with `*` in text output.

### Changed

- `sniff repo git-status` worktree rendering now selects its layout by the **physical location** of the running process rather than the spelling of the current branch:
  - **Case A** (running inside a linked worktree): shows the `main` worktree path, the current worktree with ahead/behind relative to `main`, and a count-only summary of other worktrees.
  - **Case B** (running inside the main worktree): shows the current worktree and a count-only summary of other worktrees.
  - The Worktrees section is now **always rendered**, including the `there are 0 other active worktrees in this repo` summary for repositories with no linked worktrees.
  - This corrects empty/blank output for valid states the old branch-name check misclassified: a non-main branch checked out in the main worktree, detached HEAD, and repositories whose primary branch is named `master`.
- Worktree locations now render with a home-abbreviated label (a leading home directory collapses to `~`) over an absolute-path OSC8 hyperlink, and the current worktree is named by its directory basename rather than its branch.
- Section headers (`Status`, `Worktrees`, `Meta`) now use double-underline (`<uu>`) styling with capability-aware graceful degradation.
- Exactly one blank line separates every section; no double blank lines anywhere.

### Performance

- `sniff repo git-status` now completes in well under 500 ms (≈70–85 ms on a 17-worktree, ~7,300-commit monorepo, down from ≈800 ms). The command only renders the git section, so its detection plan no longer runs per-package repo language scanning, docs, or formatting detection — the dominant fixed costs. `--refresh-remotes` remains exempt (network I/O).
- Default `git-status` detection now skips expensive commit-graph walks **and the working-tree status scan** for non-current linked worktrees. Only the current worktree receives full ahead/behind, merge-conflict, merged, and dirty status; all other worktrees are enumerated but not probed. Use `--refresh-remotes` (or `GitRequest::deep()`) to force full detail for every worktree.
