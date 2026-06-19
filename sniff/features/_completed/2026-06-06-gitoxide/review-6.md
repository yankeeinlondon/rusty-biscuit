---
ready: false
agent: codex
model: ""
---

# Review

## Findings

### High: Fallible branch and tracking queries still discard ref failures

The new `try_branches()` and `try_tracking_status()` APIs propagate failures
that occur while creating an iterator, but not failures yielded by that
iterator:

- `get_local_branches_fallible()` uses `iter.flatten()`, which silently drops
  each `Err` item (`remote_refresh.rs:173`).
- `push_relevant_ahead()` uses `flatten()` and an `.ok()` peel, then converts
  failures from `references()` or `prefixed()` into an empty hidden-ref set
  (`remote_refresh.rs:273-282`).

The methods' documentation says ref-iteration and peel failures propagate
(`remote_refresh.rs:153-156`, `210-213`), and the specification requires
permission, I/O, and corruption errors to surface rather than produce
valid-looking metadata (`spec.md:358-365`). The corruption tests only make
iterator construction fail through a malformed `packed-refs`; they do not
exercise an error yielded after iteration begins or a failed remote-ref peel
(`remote_refresh.rs:1507-1578`).

Iterate explicitly with `for reference in iter { let reference =
reference.map_err(...)?; }`, propagate prefixed-ref and peel failures from
`push_relevant_ahead()`, and add fixtures that fail on an individual ref item
and remote-tracking object.

### High: Public commit and conflict APIs do not honor their error contract

The backend-neutral APIs document that repository corruption propagates as
`SniffError::Git` (`api.rs:48-51`, `62-65`, `78-81`, `94-97`, `106-109`), but
after a repository opens they call helpers that cannot return an error:

- `commit_by_sha_at()` maps object/decode failures to `None`
  (`discovery.rs:275-299`).
- `commit_files_at()` maps missing/corrupt commits, trees, parents, and diff
  failures to an empty or partial list (`discovery.rs:328-420`).
- `commits_for_path_at()` and `commits_for_branch_at()` skip revwalk, object,
  author, time, message, and ref-decoration failures
  (`discovery.rs:465-526`, `553-608`).
- `merge_conflicts_at()` maps index-read failure to no conflicts
  (`status.rs:953-957`).

The existing API tests only verify failures that prevent repository opening;
there is no post-open corrupt-object/index coverage for these public functions
(`git_parity.rs:590-607`). A corrupt repository can therefore be reported as
"commit not found", "no changed files", "no history", or "no conflicts".

Add fallible internal helpers and make these public `Result` APIs use them.
Retain explicitly documented infallible convenience helpers only where API
compatibility requires them. Add corrupt commit, tree, parent, ref, and index
fixtures through each backend-neutral API and the corresponding CLI command.

## Verification Levels

All observable behavior in this migration is repository and CLI data behavior,
so Level 1 is the appropriate verification level. No requirement needs Level 2
terminal rendering or Level 3 keyboard injection.

| Requirement | Required | Strongest evidence | Result |
|---|---|---|---|
| Discovery, status, diff, history, refs | L1 on macOS/Linux/Windows | Cross-platform workflow plus parity tests | Present, except corruption paths above |
| Branch/tracking error propagation | L1 corrupt-ref/object fixtures | Iterator-construction corruption tests | Gap |
| Backend-neutral commit API error propagation | L1 corrupt-object/index fixtures | Open-time failure tests only | Gap |
| CLI parity | L1 integration on all targets | `just test` in cross-platform matrix | Configured |
| Performance | Same-host Criterion comparison | Identical-settings record; all 16 IDs improved | Passes |

## Verification

- Reviewed the specification, iteration-6 changes, prior review, production Git
  paths, parity tests, benchmark records, and CI workflows.
- `git diff --check` passes.
- The repository now configures macOS, Linux, and Windows sniff test jobs.
- The updated same-host Criterion record reports no regression across all 16
  `git_ops` IDs.
- Rust tests, Clippy, doctests, and `cargo metadata` could not run because this
  session has no rustup default toolchain installed.

## Decision

Not ready for production. The previous performance, cache-placement, test
isolation, and cross-platform workflow findings are addressed, but corruption
can still be silently converted into valid-looking branch, tracking, commit,
and conflict results.
