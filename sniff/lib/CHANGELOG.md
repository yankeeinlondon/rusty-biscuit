# Changelog

All notable changes to this project will be documented in this file.

## [Unreleased]

### Added

- `sniff::filesystem::git::list_worktrees` — returns all worktrees (main + linked) sorted alphabetically by name, with branch, path, current-directory flag, and detached-HEAD state.
- `sniff::filesystem::git::WorktreeEntry` — public struct representing a single worktree entry.
