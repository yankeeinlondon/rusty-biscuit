---
ready: false
agent: codex
model: ""
---

# Review

## Findings

### High: The production migration is incomplete

The feature does not satisfy its primary exit criterion. `sniff/lib` still has
`git2` as a normal dependency, `GitRepo` retains both backend handles, and
production modules still use `git2` for refs, remotes, config, repository
identity, docs, justfile scope, and worktrees.

Examples include:

- `sniff/lib/Cargo.toml:21`
- `sniff/lib/src/filesystem/git/types.rs:479`
- `sniff/lib/src/filesystem/git/discovery.rs:8`
- `sniff/lib/src/filesystem/git/remote_refresh.rs:8`
- `sniff/lib/src/filesystem/git/worktree.rs:47`
- `sniff/lib/src/filesystem/repo/identity.rs:72`

The execution plan also leaves Phases 4 through 8 unchecked. In particular,
the required ref/config migration, `ThreadSafeRepository` worktree fan-out,
production `git2` removal, cross-platform validation, documentation updates,
and final benchmark sweep have not been completed.

### High: CLI-facing helpers suppress or bypass trusted-open failures

The specification requires trust, permission, I/O, and corruption failures to
surface distinctly from repository absence. Several new library APIs erase
those failures:

- `open_gix()` converts every `trusted_discover()` error to `None`
  (`sniff/lib/src/filesystem/git/api.rs:37`).
- Remote helpers use an unchecked `git2::Repository::discover(...).ok()`
  (`api.rs:42`).
- `merge_conflicts_at()` calls `gix::open()` directly instead of the trusted
  opener (`api.rs:82`).
- The package and package-area CLI paths discard `repo_root()` errors with
  `.ok().flatten()` (`sniff/cli/src/commands/repo.rs:46` and `:134`).

An untrusted or corrupt repository can therefore be reported as missing, as
having no remote, or as having no conflicts. This violates the specified error
contract and makes behavior depend on which command is used.

### High: Deep status output is not behavior-parity compatible

`get_repo_status_with_changes()` now hard-codes `origin_commit` to `None`
(`sniff/lib/src/filesystem/git/status.rs:259`). The previous implementation
populated it through `get_commit_refs()`, so serialized `DirtyFile` output has
changed for repositories with an upstream.

The new `unified_diff()` also emits only hunk headers and content
(`status.rs:556`). The prior `git2::DiffFormat::Patch` output included file-level
patch headers such as the diff paths and old/new file markers. This breaks the
specification's byte-identical-output goal and can also change binary-file
output. The current integration test only checks that the diff string is
non-empty, so neither regression is detected.

### High: The mandatory performance gate has not been met

The plan explicitly records that no same-host comparison was performed and
defers it to Phase 8. The committed baseline also says its reduced-sampling,
high-variance measurements must be recaptured before a real no-regression gate.
Production readiness cannot be established without the required Criterion
comparison.

The summary implementation additionally contradicts the intended optimization:
`GitRequest::summary()` calls `get_repo_status_counts()`
(`sniff/lib/src/filesystem/git/types.rs:713`), which delegates to the detailed
counter and consumes the entire status iterator
(`sniff/lib/src/filesystem/git/status.rs:681`). It does not use
`Repository::is_dirty()` or otherwise stop after the first change as required.

### Medium: Non-UTF-8 paths are converted too early for correct diff processing

Status paths are converted lossily to `PathBuf` immediately
(`sniff/lib/src/filesystem/git/status.rs:88`), then the lossy value is converted
back to bytes for index/tree lookup and used for worktree file reads
(`status.rs:381`, `:470`). For a genuinely non-UTF-8 path, those lookups target a
different path, producing zero line statistics or empty patches. Distinct byte
paths can also collapse to the same replacement-character path.

The specification requires byte-native paths internally and lossy conversion
only at the public boundary. The existing test asserts only that handling does
not panic and that the displayed path is non-empty; it does not verify stats,
patches, or collision behavior.

### Medium: Required parity assertions are too weak or absent

Several required contracts are not actually verified:

- The SHA-256 test accepts `Ok(Some)`, `Ok(None)`, or `Err`; it therefore does
  not assert the documented unsupported/error outcome
  (`sniff/lib/tests/git_parity.rs:437`).
- There is no effective ownership/trust-failure test for the CLI-facing APIs.
- Working-tree patch bytes and `origin_commit` parity are untested.
- Config layering has not been tested on both macOS and Windows.
- Worktree fan-out still uses per-worker `git2` opens, so the required gix
  implementation and its parity tests do not exist.

All Git and CLI requirements here are correctly Level 1 concerns; no Level 2 or
Level 3 terminal verification is required. The gap is missing or insufficient
Level 1 assertions, not incorrect test-level placement.

## Verification

- Inspected the specification, execution plan, production diff, parity tests,
  benchmark harness, and baseline record.
- `git diff --check` fails because
  `sniff/cli/src/commands/remote.rs:183` has an extra blank line at EOF.
- Rust tests, Clippy, and rustfmt could not be rerun in this session because
  rustup has no installed/default toolchain.

## Decision

Not ready for production. The implementation is an intermediate migration
state and fails required correctness, trust-handling, performance, dependency,
and cross-platform exit criteria.
