---
ready: false
agent: codex
model: ""
---

# Review

## Findings

### High: The mandatory performance gate fails

The recorded comparison reports statistically significant regressions for
discovery, three revwalk variants, commit-file diffs, and eight-worktree fan-out
(`sniff/lib/baselines/gix.md:57-74`). The largest is
`git_ops/diff_commit_files` at `+235.15%`; graph-absent revwalks regress by
roughly 54-57%. The record itself defers the diff regression to a future phase
(`sniff/lib/baselines/gix.md:90-94`), but the specification makes
equal-or-better performance a hard release gate and requires every Criterion ID
to avoid a regression (`spec.md:496-511`).

The comparison is also not the required final proof: the git2 baseline used
reduced sampling and explicitly says it must be recaptured with default sampling
before a real no-regression decision (`sniff/lib/baselines/git2.md:29-34`).
Only the gix side was recaptured with default sampling.

Recapture both backends under identical default-sampling conditions, then
optimize or explicitly revise the specification before release. The documented
levers that remain unused include repository object-cache sizing for revwalk,
diff, and ref enumeration (`spec.md:519-525`).

### High: Fallible worktree enumeration still returns valid-looking metadata after real errors

`GitRepo::worktrees()` is now fallible, but only registry, path-resolution, and
open errors propagate. Once a worktree is open, failures are still converted to
normal values:

- revwalk failures become ahead/behind `0` (`remote_refresh.rs:35-44`);
- merge-base failures become `merged = false` (`remote_refresh.rs:47-52`);
- merge failures become `has_conflicts = false` (`remote_refresh.rs:655-662`);
- status failures become `dirty = false, changed_files = 0`
  (`status.rs:849-857`, consumed at `remote_refresh.rs:629`).

These are permission, I/O, or corruption failures on a fallible public API, not
legitimately optional HEAD/upstream cases. They violate the error policy in
`spec.md:358-365` and can report a damaged worktree as clean, conflict-free, and
fully synchronized.

Make the worktree-specific ahead/behind, ancestry, merge-conflict, and status
helpers return `Result` and propagate their operation-tagged errors. Add L1
fixtures for a corrupt index/object database and a failed ancestry or merge
read.

### High: Linux and Windows release verification is still absent

The feature targets macOS, Linux, and Windows and requires all three to build and
pass parity tests. The review artifact records only macOS execution and says
Linux and Windows still require CI confirmation
(`review-3.md:30-36`, `review-3.md:131-135`). No checked-in result closes that
gate.

The platform-config tests also do not prove the required behavior. The macOS
test permits the Command Line Tools config to be absent and only proves that a
global value wins (`git_parity.rs:1696-1732`); the Windows unit test similarly
accepts an absent Git-for-Windows config and checks only for no panic
(`remote_refresh.rs:1079-1095`). Neither verifies all 12 keys through the actual
platform fallback.

Run the effective tree's build, L1 library tests, and CLI integration tests on
Linux and Windows. On macOS and Windows, use a controlled platform-system config
fixture or injectable path to verify all 12 keys and precedence through the
production fallback path.

### Medium: `GitRepo::worktrees()` introduces an undocumented source-breaking API change

The public method changed from
`HashMap<String, WorktreeInfo>` to
`Result<HashMap<String, WorktreeInfo>>`
(`types.rs:629-639`). Propagating errors is appropriate, but this migration is
specified as behavior-preserving and the public README does not document this
signature change or a migration path.

Either retain `worktrees()` as the documented best-effort compatibility API and
add `try_worktrees()`, or explicitly classify and document the breaking change.

### Medium: Final dependency documentation is stale

The final phase requires dependency documentation to be updated from git2 to
gix (`spec.md:740-751`), but `docs/dependencies.md:545-551` still lists git2 as
the Git production dependency and contains no gix entry. The staged module docs
also contain the stray text `TEST EDIT` at
`remote_refresh.rs:2`.

Update the generated dependency documentation and remove the accidental module
comment before release.

## Verification Levels

All user-observable requirements here are repository and CLI data behavior.
Level 1 is the appropriate tier; there are no terminal-rendering or input
requirements needing Level 2 or Level 3.

| Requirement area | Required level | Strongest evidence | Result |
|---|---|---|---|
| Discovery, status, diffs, history, refs | L1 | Unit/integration parity on macOS | Present on macOS |
| Worktree metadata and failure semantics | L1 | Success and open/registry failure tests | Gap for post-open operation failures |
| Platform config layering | L1 on macOS and Windows | Partial macOS tests; no recorded Windows run | Gap |
| CLI output parity | L1 integration on all targets | macOS recorded | Linux/Windows gap |
| Performance | Criterion, outside L1-L3 | Same-host comparison records regressions | Failed |

## Verification

- Reviewed the specification, plan, prior reviews, staged iteration-4 changes,
  production git paths, parity tests, benchmark harness, and baseline records.
- `git diff --cached --check` passes.
- Production `git2` use remains test-only; CLI production source has no `git2`
  or `gix` imports.
- Tests, Clippy, doctests, and metadata could not be rerun because rustup has no
  installed/configured default toolchain in this session.

## Implementation Notes (Post-Review)

### Completed

- **Worktree metadata error suppression** — `ahead_behind`, `count_reachable_excluding`,
  `is_ancestor`, `has_merge_conflicts`, `push_relevant_ahead`, and
  `get_repo_status_counts` now return `Result` and propagate operation-tagged
  `SniffError::Git` errors. `get_worktrees` propagates all post-open failures
  rather than converting them to normal values.
- **L1 corrupt-fixture tests** — Added tests for corrupt index (status error),
  corrupt object (revwalk and ancestry errors), and corrupt merge tip
  (merge-conflict error).
- **API breaking change documented** — `GitRepo::worktrees()` signature change
  is recorded in `sniff/lib/CHANGELOG.md` as a breaking change.
- **Dependency documentation updated** — `docs/dependencies.md` now lists `gix`
  instead of `git2`.
- **Stray text removed** — `TEST EDIT` removed from `remote_refresh.rs:2`.
- **Platform config tests improved** — `get_git_config` refactored to
  `get_git_config_with_extra` with an injectable fallback path. Added
  `extra_system_config_fallback_reads_all_12_keys` test that isolates the
  environment and verifies every key reads through the fallback.
- **Object-cache sizing applied** — `configure_cache` helper added to
  `open.rs`; `trusted_discover` and `trusted_open` size the object cache
  proportionally to the index entry count via
  `compute_object_cache_size_for_tree_diffs`.

### Remaining (requires infrastructure / platform access)

- **Performance gate** — Baselines must be recaptured under identical
  default-sampling conditions on the same host; then the comparison rerun.
- **Linux and Windows verification** — Build, L1 library tests, and CLI
  integration tests must be executed on Linux and Windows runners.

## Decision

Not ready for production. Iteration 4 resolves the earlier worktree-open,
timestamp-pruning, ref, and basic config-test findings, but the specification's
performance and three-platform gates remain unmet, and worktree metadata still
suppresses real repository failures.
