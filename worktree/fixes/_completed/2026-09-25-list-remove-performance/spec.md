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
reviewed: true
reviewed_by: codex/gpt-6-sol
reviewed_on: 2026-09-26
review_iterations: 2
completed: true
related:
    - 2026-09-24-ux-improvements
    - 2026-09-25-worktree-file
human_review: false
implemented: true
message_to_agent: |-
    All 5 phases are implemented; read "## Phase 5" in implementation-log.md
    for the gates, the cross-OS evidence, and the acceptance-to-test map.
    For the reviewer:
    1. Run perf gates serially only. nextest runs each test in its own
       process, so #[serial] does not stop the three perf_pr_request gates
       from overlapping. The first native-Windows run of them overlapped and
       failed perf_list_meets_sla_when_the_pr_request_hits_its_deadline
       (warm list gather 120.7 ms > 120 ms); the serial rerun passed at
       95.3 ms. Windows' warm list gather (92-95 ms) has little headroom
       under the 120 ms bound. That margin predates this fix.
    2. Phase 4 has no log section of its own; its commits are ae03b5381,
       143bb2c50, 156978301, and c43a286d2. Phase 5 verified "Prove handoff
       safety" on all four OSes and ticked it.
    3. The spec status is still draft-spec; this phase did not change it.
---

# `wt list` and `wt remove` performance

Performance problems found after `2026-09-24-ux-improvements` was implemented, measured on 2026-09-25 in this repository (3 worktrees, 13,013 tracked files, macOS on an APFS volume). The first needs a fix. The removal handoff has a narrower safe optimization than the original draft proposed. The `git status` cost was investigated and needs no change.

## 1. PR badges: stale-while-revalidate

**Problem.** `wt list` waits for the open-PR request (up to its 300 ms deadline) whenever the stored answer is older than the 60 s freshness window. Measured:

| Run | `list gather` | `pr gather` | Total |
|---|---|---|---|
| Stored answer older than 60 s | 66 ms | 315 ms | 338 ms |
| Stored answer fresh | 63 ms | 0.03 ms | 83 ms |

`wt list` is rarely run twice within a minute, so in practice most runs pay the request. The deadline also overshoots slightly (315 ms against 300 ms).

**Fix:** `wt list` uses a stored answer immediately when that answer belongs to the current `origin` and refreshes an old answer in another process.

- **Stored answer, any age:** the table renders from it immediately. When the answer is older than the freshness window, `wt list` starts a background refresh and exits without waiting for it. The next run can show refreshed badges after that process finishes.
- **No stored answer** (the first run in a repository, or after the cache is cleared): the request runs as today, under the 300 ms deadline, so badges can appear on the very first run. Only this run pays the request.
- **Remote identity:** bind each stored answer to the exact `git remote get-url origin` value used for its request. Store only its `biscuit-hash` digest, since a URL can contain credentials. Check the current value before showing any stored badges, including a fresh answer. If `origin` is missing or different, ignore the old answer and take the no stored answer path. Bump the PR store format version; an older file is a cache miss. This deliberately adds one local Git call to cache hits, so measure it in the warm gate.
- **Age line:** the existing dim line under the legend ("PRs as of 12 min ago") appears whenever the shown badges are older than the freshness window, including while a refresh is in progress or after it fails. Compute age at render time; treat a stored fetch time in the future as a cache miss.
- **Background refresh:**
    - A detached `wt` process (a hidden internal subcommand) makes the request and atomically replaces the stored answer. Pass the main checkout path explicitly, and set the child's working directory there so it does not hold a linked worktree open. Disconnect stdin, stdout, and stderr. On Windows, inherited pipe handles can otherwise make the parent wait for the child (see the `os` skill's `windows.md`, "Handle inheritance blocks `.output()`"). A spawn failure leaves the stored answer visible and stale.
    - Move `configure_detached_child` from `playa`'s `detached` module to a public API in `sniff::process`; `playa` and `worktree-cli` use that API. Keep its existing platform behavior and the caller's obligation to set null standard streams. `playa` and `worktree-cli` already depend on `sniff`.
    - Use a nonblocking, cross process exclusive file lock beside the PR store. The worker holds it while checking freshness, fetching, and saving; a worker that cannot lock exits without a request. A process crash releases the lock, avoiding stale marker recovery races. The parent may start more than one worker during a race, but at most one makes the request. Use the repository's existing `fs4` crate in `worktree` for the lock and record the new dependency edge in the dependency docs.
    - After acquiring the lock, read the store and current `origin` again. Skip a request if another worker already refreshed it. Save only if `origin` still matches the identity used for the request; otherwise discard the result. A failed refresh (offline, no credentials, error) leaves the stored answer untouched; it is never replaced with an empty list. The existing rule that an authentication error is an unavailable answer, not "no open PRs", still applies.
- `wt remove` continues using its separate live PR lookup and deadline whenever that answer is needed for branch safety.

**Target:** a `wt list` with a stale, matching answer has no network wait and stays near the measured fresh-answer time on the same host. Assert a generous, contention-tolerant wall-clock bound as well as a deterministic no-wait check; do not use the 80 ms observation as a cross-platform limit.

## 2. `wt remove`'s move-first handoff can avoid unnecessary lookups

**Problem.** When removing the worktree the caller is standing in, the second run of [`run_handoff`](../../cli/src/commands/remove/mod.rs) in `worktree-cli` gathers all removal facts again. That repeats a PR lookup with a 2 s deadline and possibly live remote checks with a 3 s deadline. The two runs can therefore spend about 6 s waiting on a slow network. It also reads every dirty file twice across the handoff.

**Fix:** make the second run's network work depend on the approved action and on fresh local evidence.

- Always consume the one-use handoff record, verify the caller left the worktree, re-read the worktree, branch, include rules, copy baseline, and full dirty-file fingerprint, and refuse if they changed. Keep the existing 60 s expiry and exit behavior.
- If the caller chose to keep the local branch or explicitly approved its deletion, the second run does not need a PR answer or live remote evidence to decide local branch safety. Gather only the local facts needed to verify the handoff and carry out that choice. If the branch is being deleted automatically because it was judged safe, recompute its safety. A fresh local default branch, another local branch, or tag containing the tip can prove it without a PR request or live remote check. If only a PR or remote-tracking ref supplies the proof, repeat the current live lookup; on failure or lost proof, refuse before removing the worktree.
- With `--force-remote`, still recompute the destination and push endpoint, reject endpoint reinterpretation, and query the current live head before any mutation. The existing lease remains tied to the first run's approved SHA. Refuse before removing anything if the destination, endpoint, or live head differs from the approval; an unavailable live answer is not proof of no change. Recheck the endpoint immediately before pushing, as the current deletion code does. A lease failure after local removal still reports a partial result as today.
- Keep [`Inventory::fingerprint`](../../lib/src/remove/inventory.rs) in `worktree` content based for dirty files of every size, including files inside untracked nested repositories. Size and modification time cannot prove that a file's bytes stayed the same: an edit can preserve both, and timestamps can be restored or too coarse. The proposed 1 MiB shortcut would let the handoff discard unapproved work. The one minute lifetime does not close that gap. Keep the separate included-file observation contract in [`compare.rs`](../../lib/src/compare.rs), which `2026-09-25-worktree-file` requires.

**Reader's note on the revised design.** The original draft proposed storing and reusing all first-run network answers and using metadata for large dirty files. The worktree fingerprint verifies only local state. It cannot prove that a remote branch or PR still protects a commit, and the remote deletion lease protects only the branch being pushed to. Reusing those answers could delete the local branch after its last surviving remote copy disappeared. The metadata shortcut could discard a changed file. This revision keeps those safety checks and narrows the speedup to cases where the approved action or fresh local evidence makes network proof unnecessary.

**Target:** on a slow or offline network, a handoff that keeps the branch or explicitly approves its deletion and does not use `--force-remote` makes no second-run network request. Handoffs whose automatic deletion relies on remote evidence or whose action includes remote deletion may still pay a second network deadline. The second run still reads large dirty files in full.

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
    - With a stored answer older than the freshness window, `wt list` renders it without waiting, shows the age line, and starts a detached refresh. Concurrent list runs cause at most one request; the worker checks the store again after taking its lock. A worker crash allows a later refresh (L1, with a blocking local source and process synchronization rather than sleeps).
    - With no stored answer, or an answer bound to a different or missing `origin`, `wt list` makes a foreground request under the existing deadline when a source is available. A changed `origin` never shows the old badges; an old-format store is a miss (L1).
    - A failed refresh or one whose `origin` changes during the request leaves the stored answer unchanged. A stored timestamp in the future is not treated as fresh (L1).
    - The background process does not keep `wt list` waiting on any platform, including Windows' handle inheritance, and does not hold a linked worktree as its working directory (L1 subprocess test; run the Windows case on native Windows).
    - A new performance gate times the full non-image command with a stale matching answer and a blocked refresh, using the fixture and contention-tolerant method documented in `worktree/docs/performance-testing.md`. A deterministic test also proves the parent never joins the worker. `list gather` alone does not cover the network wait being removed.
2. **Handoff network work:** after moving out, keeping the branch or deleting it by explicit approval without `--force-remote` makes no PR or live remote request. Automatic deletion with fresh local branch or tag proof also makes none. Automatic deletion relying only on remote or PR evidence rechecks that evidence and refuses before worktree removal if it disappeared. A `--force-remote` handoff refuses before local removal when the live head changes or becomes unavailable; a push after its preflight fails the lease (L1 with counting stubs and a local bare remote).
3. **Fingerprint:** changing a large dirty file while preserving its size and modification time refuses the handoff. The same holds for a file inside an untracked nested repository. The second run reads dirty files fully, and a read error refuses deletion (L1).

## Packages

- `worktree` (library): the PR store's remote binding, freshness rules, and cross process lock in `pull_requests.rs`; a narrow local safety proof for the second removal run. Preserve the handoff record's existing remote approval and the full inventory fingerprint.
- `worktree-cli`: the hidden refresh subcommand and detached launch from `wt list`; `run_handoff` selects the minimum facts needed for the approved action and still verifies live evidence when required.
- `sniff`: move `configure_detached_child` from `playa` into a public `process` API and retain its Windows handle behavior.
- `playa`: use the moved helper with no behavior change.
- Docs: update `worktree/docs/performance-testing.md` with the stale-answer full-command gate and section 3's `git status` findings. Update `docs/dependencies.md` for the new `worktree` to `fs4` dependency edge, and update the worktree skill for the new PR cache and handoff behavior.

## Decisions

1. Revised 2026-09-26: `wt list` shows a stored answer immediately only when it belongs to the current `origin`; an old answer refreshes in a detached process. The first request for an origin remains foreground. A cross process file lock prevents concurrent duplicate refresh requests and recovers when a worker exits.
2. Revised 2026-09-26: the removal handoff skips second-run network checks only when the approved local branch action or fresh local proof makes them unnecessary. Automatic branch deletion that needs remote or PR evidence still checks it live. Remote deletion still checks the current head and uses the approved lease.
3. Revised 2026-09-26: the handoff fingerprint continues hashing dirty files of every size on both runs. Size and modification time are insufficient to prove that discarded content is unchanged. Included files retain their separate observation contract from `2026-09-25-worktree-file`.
4. Ruled 2026-09-25: no file-system watcher or daemon for `wt list`; the per-worktree `git status` cost is recorded, not changed.
5. Ruled 2026-09-25: the detached-spawn helper moves from `playa` to `sniff`'s `process` module, which both `playa` and `worktree-cli` already depend on; `playa` and `sniff` are in this spec's scope.
