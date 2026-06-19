---
ready: false
agent: codex
model: ""
---

# Review

## Findings

### High: Fallible revision entry points still classify every lookup failure as absence

The new fallible helpers propagate failures after a walk starts, but still
discard failures while resolving the starting revision:

- `get_commit_by_sha_fallible()` maps every `rev_parse_single()` error to
  `Ok(None)` (`discovery.rs:311-315`).
- `get_commits_for_path_fallible()` maps every `head_id()` error to an empty
  history (`discovery.rs:519-522`).
- `get_commits_for_branch_fallible()` uses `.ok()` for local-ref lookup, ref
  peeling, and fallback rev parsing (`discovery.rs:611-619`).

These are fallible public-API paths, so the specification requires only genuine
not-found, unborn-HEAD, or detached/optional cases to become `None`/empty.
Permission, I/O, malformed-ref, and corruption errors must remain
`SniffError::Git` (`spec.md:349-365`). In particular, a corrupt `refs/heads/main`
can still make `commits_for_branch_at()` return a valid-looking empty history.

Match the specific gix not-found/unborn variants and propagate all other
lookup/peel errors. Add L1 fixtures for malformed `HEAD`, a malformed requested
branch ref, a branch ref targeting a missing object, and a revision lookup that
fails for a reason other than no match. The new ancestor-corruption tests
(`git_parity.rs:697-723`) begin after successful revision resolution and do not
cover these branches.

### High: Primary detection and recent-commit Result APIs still suppress corrupt history

`GitRepo::detect_with_request() -> Result<GitInfo>` still obtains history through
the infallible `get_recent_commits()` helper (`types.rs:768-773`). That helper
converts HEAD, revwalk creation/items, object decode, author, timestamp, message,
and ref-decoration failures into empty or partial history
(`discovery.rs:131-190`). The cached `ref_decorations()` path also converts any
ref failure into an empty map (`types.rs:512-524`).

The separate public recent-commit queries have the same problem:
`get_recent_commits_by_duration`, `get_recent_commits_in_range`,
`get_recent_commits_by_date`, `get_recent_commits_by_hash`, and
`get_recent_commits_by_count` return `Result`, but their collectors silently
skip revwalk/object/message failures and use the infallible commit-diff helper
(`recent_commits.rs:360-425`, `474-526`, `577-632`). A corrupt repository can
therefore produce successful but incomplete `git-status`, `recent-commits`, and
source-change output.

Add fallible history/decorations collectors and use them from every public
`Result` API, retaining suppression only in explicitly documented infallible
convenience methods. Add L1 corruption tests through
`detect_git_with_request()` and each recent-commit query, plus CLI tests for
`repo git-status`, `repo recent-commits`, and the source-change commands. The
current CLI additions cover only `repo hash` and `has-merge-conflict`
(`cli/tests/cli.rs:1591-1630`).

## Verification Levels

All user-observable requirements are repository/CLI data behavior. Level 1 is
the appropriate level; no requirement needs Level 2 terminal rendering or Level
3 keyboard injection.

| Requirement | Required | Strongest evidence | Result |
|---|---|---|---|
| Discovery, status, diff, history, refs | L1 on macOS/Linux/Windows | Cross-platform workflow and parity fixtures | Gap for corruption paths above |
| Branch/tracking iterator and peel errors | L1 corrupt-ref/object fixtures | New per-item and peel tests | Present |
| Path-based commit/conflict APIs | L1 corrupt-object/index fixtures | New library and CLI tests | Present after revision resolution |
| Full detection and recent-commit commands | L1 corrupt-history/ref fixtures | Happy-path parity tests | Gap |
| Performance | Same-host Criterion comparison | All 16 recorded IDs improved | Passes |

## Verification

- Reviewed the specification, review 6, staged iteration-7 changes, production
  Git paths, parity tests, CLI tests, benchmark records, and cross-platform CI.
- `git diff --cached --check` passes.
- macOS, Linux, and Windows jobs run `just test`; the workflow also invokes the
  L2 recipe separately.
- The same-host Criterion record reports no regression across all 16 `git_ops`
  IDs.
- Rust tests, Clippy, and doctests could not run because this session has no
  rustup default toolchain configured.

## Decision

Not ready for production. Iteration 7 fixes the review-6 iterator, peel,
post-resolution object, and index error paths, but fallible revision resolution
and the primary history APIs can still convert repository corruption into
successful empty or partial output.
