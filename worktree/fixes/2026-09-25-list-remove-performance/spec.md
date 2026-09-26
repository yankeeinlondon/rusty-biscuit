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
status: draft-spec
reviewed: false
review_iterations: 0
related:
    - 2026-09-24-ux-improvements
    - 2026-09-25-worktree-file
---

# `wt list` and `wt remove` performance

Performance problems found after `2026-09-24-ux-improvements` was implemented, measured on 2026-09-25 in this repository (3 worktrees, 13,013 tracked files, macOS on an APFS volume). Two need fixing; the third was investigated and needs no change, and is recorded so it is not re-investigated.

## 1. PR badges: stale-while-revalidate

**Problem.** `wt list` waits for the open-PR request (up to its 300 ms deadline) whenever the stored answer is older than the 60 s freshness window. Measured:

| Run | `list gather` | `pr gather` | Total |
|---|---|---|---|
| Stored answer older than 60 s | 66 ms | 315 ms | 338 ms |
| Stored answer fresh | 63 ms | 0.03 ms | 83 ms |

`wt list` is rarely run twice within a minute, so in practice most runs pay the request. The deadline also overshoots slightly (315 ms against 300 ms).

**Fix** (ruled 2026-09-25): `wt list` never waits for the network when it has a stored answer.

- **Stored answer, any age:** the table renders from it immediately. When the answer is older than the freshness window, `wt list` starts a background refresh and exits without waiting for it. The next run shows the refreshed badges.
- **No stored answer** (the first run in a repository, or after the cache is cleared): the request runs as today, under the 300 ms deadline, so badges can appear on the very first run. Only this run pays the request.
- **Age line:** the existing dim line under the legend ("PRs as of 12 min ago") appears whenever the shown badges are older than the freshness window, so a stale answer is never presented as current.
- **Background refresh:**
    - A detached `wt` process (a hidden internal subcommand) makes the request and atomically replaces the stored answer. It does not inherit the parent's standard handles: on Windows, inherited pipe handles make the parent wait for the child (see the `os` skill's `windows.md`, "Handle inheritance blocks `.output()`").
    - The Windows-safe detached-spawn helper exists today as `configure_detached_child` in `playa`'s `detached` module. It moves to `sniff`'s `process` module, which already owns how the repository spawns child processes (deadlines, process-tree termination, Windows Job Objects), and is exported from there. `playa` and `worktree` both already depend on `sniff`, so `playa` switches to the moved helper, `worktree` uses it, and no new dependency is added.
    - At most one refresh runs per repository at a time: a refresh marker in the user-cache store, with a start time, stops a second `wt list` from starting another. A marker older than the request deadline plus a margin is treated as abandoned.
    - A failed refresh (offline, no credentials, error) leaves the stored answer untouched; it is never replaced with an empty list. The existing rule that an authentication error is an unavailable answer, not "no open PRs", still applies.
- `wt remove`'s PR check is unchanged: removal needs a live answer and keeps its own deadline.

**Target:** a `wt list` with a stored answer of any age completes in about the same time as one with a fresh answer (about 80 ms here); a new performance gate asserts it.

## 2. `wt remove`'s move-first handoff repeats its network checks

**Problem.** When removing the worktree the caller is standing in, the second run (`run_handoff` in `worktree-cli`'s `commands/remove/mod.rs`) calls `Facts::gather` again. That repeats the PR lookup (2 s deadline) and the live remote check (3 s deadline), which run in parallel, so offline or on a slow network the whole removal can take about 6 s: up to 3 s per run. The fingerprint also reads and hashes every dirty file's full content in both runs; a large untracked file is read twice.

**Fix** (ruled 2026-09-25):

- **Reuse the first run's network answers.** The handoff record stores the first run's PR answer and live remote heads alongside the state it already records. The second run reuses them instead of asking the network again. This is safe because the record already proves the branch tip and worktree state have not changed, and it expires after one minute. `--force-remote` deletion still uses a lease against the observed remote SHA, so a push between the runs still fails the deletion rather than deleting new work.
- **Cheaper fingerprint for dirty files.** Dirty files under 1 MiB are always hashed, so an edit that keeps a small file's size and modification time cannot slip through. Files of 1 MiB or more are fingerprinted by size and modification time and hashed only when either differs. The shortcut is acceptable here because the fingerprint only has to catch a change during the minute between the two runs. It does **not** apply to `.worktreeinclude` files: `2026-09-25-worktree-file` (its Decision 7) hashes every same-size included file in full, because those files live for days in a worktree. Hashing uses `biscuit_hash::blake3_hash_reader`, and both specs share one comparison contract.

**Target:** offline, removing the worktree the caller is standing in takes at most one network deadline (about 3 s) instead of two, and a large unchanged untracked file is not read by the second run.

## 3. `git status` cost in `wt list` (investigated, no change)

`wt list` runs one `git status` per worktree, in parallel; one `wt list` starts 6 git processes in total here. Each `git status` takes about 30 ms but about 0.22 s of system CPU:

| `git status` variant | Wall time | System CPU |
|---|---|---|
| `--untracked-files=all` (used by `wt remove`, which must list every file) | 80 ms | 0.25 s |
| default, untracked folders collapsed (used by `wt list`) | 30 ms | 0.22 s |
| `--untracked-files=no` | 20 ms | 0.19 s |
| default, with `core.preloadIndex=false` | 150 ms | 0.09 s |

- The system CPU is the kernel checking every tracked file for changes. Git spreads that across threads (`core.preloadIndex`), roughly doubling the CPU spent to cut the wait; turning it off trades 0.13 s of CPU for 120 ms of extra wall time, which is the wrong trade for an interactive command.
- The only way to avoid checking every file is a file-system watcher. Git's built-in one (`core.fsmonitor`) runs a background daemon per worktree and exists only on macOS and Windows: on the Linux build host (git 2.47.3), `git fsmonitor--daemon` reports "not supported on this platform", and Linux needs Watchman plus a hook instead.
- Ruled 2026-09-25: no watcher and no daemon. `wt list`'s wall time is already small; the CPU cost grows with the number of worktrees, but only a watcher would remove it, and that cost is not worth a daemon.

## Acceptance criteria and testing

Levels follow the `rust-testing` skill. Every criterion holds on macOS, Linux, native Windows, and WSL2.

1. **Stale-while-revalidate:**
    - With a stored answer older than the freshness window, `wt list` renders it without waiting, shows the age line, and starts exactly one background refresh; a second `wt list` while that refresh runs starts none (L1, with a stubbed PR source that blocks).
    - With no stored answer, `wt list` makes the request under the deadline as before (L1).
    - A failed refresh leaves the stored answer unchanged (L1).
    - The background process does not keep `wt list` waiting on any platform, including Windows' handle inheritance (L1 with a refresh that sleeps past the command's exit; run on Windows).
    - A new performance gate: `list gather` plus `pr gather` with a stale stored answer stays within the warm target (L1 performance gate, next to the existing ones in `worktree/docs/performance-testing.md`).
2. **Handoff reuse:** the second run of a move-first removal makes no PR request and no live remote query (L1, counting calls through stubs); a remote push between the runs makes `--force-remote` deletion fail its lease (L1 with a local bare remote).
3. **Fingerprint:** editing a small dirty file without changing its size or modification time is detected; a large unchanged file is not read by the second run; a large file whose size or modification time changed is hashed (L1).

## Packages

- `worktree` (library): the PR store's stale-while-revalidate rules and refresh marker (`pull_requests.rs`), the handoff record's stored network answers (`remove/handoff.rs`), and the fingerprint rule (`remove/inventory.rs`).
- `worktree-cli`: the hidden refresh subcommand, starting it detached from `wt list`, and `run_handoff` reusing the stored answers.
- `sniff`: `configure_detached_child` moves from `playa` into the `process` module and is exported (the module is `pub(crate)` today).
- `playa`: its `detached` module uses the helper from `sniff` instead of its own copy; behavior is unchanged.
- Docs: `worktree/docs/performance-testing.md` gains the stale-answer gate and the `git status` findings of section 3.

## Decisions

1. Ruled 2026-09-25: `wt list` renders stored PR badges immediately at any age and refreshes them in a detached background process; only a repository with no stored answer waits for the request.
2. Ruled 2026-09-25: the move-first handoff stores the first run's PR answer and live remote heads, and the second run reuses them.
3. Ruled 2026-09-25: the handoff fingerprint hashes dirty files under 1 MiB and uses size and modification time for larger ones. Included files are excluded from this shortcut and always hashed (`2026-09-25-worktree-file`, Decision 7).
4. Ruled 2026-09-25: no file-system watcher or daemon for `wt list`; the per-worktree `git status` cost is recorded, not changed.
5. Ruled 2026-09-25: the detached-spawn helper moves from `playa` to `sniff`'s `process` module, which both `playa` and `worktree` already depend on; `playa` and `sniff` are in this spec's scope.
