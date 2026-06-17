# Changelog

All notable changes to this project will be documented in this file.

## [Unreleased]

### Added

- `sniff::filesystem::git::list_worktrees` — returns all worktrees (main + linked) sorted alphabetically by name, with branch, path, current-directory flag, and detached-HEAD state.
- `sniff::filesystem::git::WorktreeEntry` — public struct representing a single worktree entry.

### Changed

- **Breaking:** `GitRepo::worktrees()` now returns `Result<HashMap<String, WorktreeInfo>>` instead of `HashMap<String, WorktreeInfo>`. Permission, I/O, and corruption failures during worktree enumeration are propagated as `SniffError::Git` rather than silently suppressed. This aligns with the library's error policy for fallible public APIs (spec §4.2).
