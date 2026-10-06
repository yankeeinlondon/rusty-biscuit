---
status: implemented
reviewed: false
clarified: false
implemented: true
implemented_by: claude-code/claude-opus-5-5
human_review: true
---

# The refresh worker starts too many git processes

## Problem

`wt list` waits up to 3 s for its refresh worker (`wt internal-refresh`). On
native Windows the worker misses that wait under load, and about 36
`worktree-cli` L1 tests fail on `windows-latest` (and some on the
`build-win-native` host) with "origin hasn't answered yet; still checking in
the background".

The worker is not slow at its real work. Measured on 2026-10-05 with
`GIT_TRACE` and `--perf=json` against a local bare `origin`:

| | macOS | Windows |
|---|---|---|
| one trivial git process | ~5 ms | ~47 ms |
| `ls-remote` (the check) | 30 ms | 146 ms |
| fetch | 46 ms | 280 ms |
| whole worker | 140 ms | 1020 ms |

The head half's critical path runs **14 git processes in sequence**; only
`ls-remote` and `fetch` talk to `origin`. The other 12 read local metadata,
several of them repeatedly: `remote get-url origin` (6 times per worker,
counting the PR half), `symbolic-ref refs/remotes/origin/HEAD` (4),
`check-ref-format --branch` (2), `config --get core.sshCommand` (2), and
`rev-parse --verify refs/remotes/origin/<branch>` (2). About 75% of the
Windows worker is process start-up. The same reads run in `wt list`'s
foreground path.

## Decision

Read repository metadata in-process with `gix` (already in the build through
`sniff`, pinned `=0.84.0`), keeping every function's signature and meaning:

| Function | Today | After |
|---|---|---|
| `pull_requests::origin_url` | `git remote get-url origin` | `remote.origin.url` (first value) with `url.<base>.insteadOf` applied by longest prefix |
| `worktree::default_branch_in` | `symbolic-ref`, then `rev-parse --verify main/master` | `refs/remotes/origin/HEAD`'s symbolic target, then `main`/`master` resolved as revisions |
| `live_remote::is_valid_branch_name` | `check-ref-format --branch` | `gix` reference-name validation of `refs/heads/<branch>`; `HEAD` and a leading `-` refused |
| `live_remote::batch_ssh_command` | `config --get core.sshCommand` | `core.sshCommand` from the config snapshot |
| `remote_update::tracking_tip` | `rev-parse --verify --quiet` | the reference's object ID, following symbolic refs, never peeling |

- Every read opens the repository afresh, so the deliberate re-reads in
  `remote_update::changed` still see a configuration that changed during an
  attempt.
- When `gix` cannot open or read the repository (for example a reftable
  repository), the function falls back to the git command it runs today.
  Correctness never depends on `gix` supporting a layout.
- Only `ls-remote` and `fetch` (and the worker's one `rev-parse` locating the
  checkout) still start git.

## Acceptance

1. Parity tests compare each in-process read with the git command it
   replaces over local paths, scp-like and HTTPS URLs, `insteadOf` (including
   overlapping prefixes), a multi-valued `remote.origin.url`, a missing
   origin, a missing `origin/HEAD`, and valid and invalid branch names.
2. A test pins the worker's git-process count for a fetch attempt (with the
   `count-git` counter) to `ls-remote`, `fetch`, and the checkout lookup.
3. `worktree` and `worktree-cli` L1 pass on macOS, Linux, and native Windows
   (`just cross-check <pkg> --os windows`) with the ordinary 3 s wait
   unchanged.
4. `docs/` pages that describe how these values are read are updated.

## Not in scope

- Replacing `ls-remote` or `fetch` with `gix`'s transport.
- Changing the 3 s wait.
