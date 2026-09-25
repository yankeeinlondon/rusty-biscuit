# Changelog

All notable changes to this project will be documented in this file.

## [Unreleased]

### Added

- `sniff::filesystem::git::list_worktrees` — returns all worktrees (main + linked) sorted alphabetically by name, with branch, path, current-directory flag, and detached-HEAD state.
- `sniff::filesystem::git::WorktreeEntry` — public struct representing a single worktree entry.
- **Breaking (source):** `InstallCapturedResult::timed_out` — the command was killed at its deadline rather than exiting on its own. Adding the field is a source break for struct-literal callers.
- **Breaking (source):** `SniffInstallationError::InstallationTimedOut { pkg, manager, timeout_secs }` — returned by `execute_install` and `execute_versioned_install` when the installer is killed at its deadline.
- **Breaking (source):** `InstallInterviewEvent::TimeoutWarning { prose }` — emitted after the failure status and before any retry prompt.
- **Breaking (source):** `InstallInterviewOutcome::TimedOut { attempted }` — every attempt failed and the last was killed at its deadline.
- `sniff::remote::blocking::{pull_request_for_branch, pull_request_for_branch_with}`: a deadline-bound, runtime-free lookup of the open or merged PR from one branch of one source repository, returning `PrEvidence` or a typed `PrUnavailable` (a list 404 is an error, never `Ok(None)`).
- **Breaking (source):** `PullRequestInfo::{source_repo, source_repo_is_target, source_head_sha}`, filled by every Stage-1 provider and the focused client. The head SHA is stored exactly as received (Bitbucket Cloud sends 12 characters). Adding the fields is a source break for struct-literal callers.

### Changed

- **Breaking:** `GitRepo::worktrees()` now returns `Result<HashMap<String, WorktreeInfo>>` instead of `HashMap<String, WorktreeInfo>`. Permission, I/O, and corruption failures during worktree enumeration are propagated as `SniffError::Git` rather than silently suppressed. This aligns with the library's error policy for fallible public APIs (spec §4.2).
- **Breaking:** Installation timeout is now a first-class outcome rather than an ordinary failure. `execute_install` and `execute_versioned_install` return `InstallationTimedOut` where they previously returned `PackageManagerFailed`, and the install interview returns `TimedOut` where it previously returned `Failed`. The meanings of `PackageManagerFailed` and `Failed` narrow to non-timeout failures only.

### Migration

`SniffInstallationError` and `InstallInterviewOutcome` are not `#[non_exhaustive]`, so exhaustive matchers must add arms for the new variants:

```rust
match outcome {
    InstallInterviewOutcome::Failed { attempted } => { /* non-timeout failure */ }
    InstallInterviewOutcome::TimedOut { attempted } => { /* killed at deadline */ }
    // ... existing arms
}
```

A caller that does not need to distinguish a timeout can treat `InstallationTimedOut` and `TimedOut` exactly as it previously treated `PackageManagerFailed` and `Failed`. A caller that reports installation state to a user should distinguish them: on Unix a timed-out install may have left a **partial install** behind, because process-tree termination there is best-effort (see the `process` module documentation).
