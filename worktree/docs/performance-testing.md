---
hash: ef46db3751d8e999-612971d5cae12dd4
last_updated: 2026-09-27
---

# Performance Testing — Worktree

This document defines the worktree-owned performance surfaces for `wt list` and related commands. It scopes what should be benchmarked, what assumptions the benchmarks rely on, and what is intentionally excluded.

## List Status Collection

The first owned cost center is [`list_worktrees`](../../worktree/lib/src/worktree.rs), which produces the status table.

- It runs `git worktree list --porcelain`, resolves the default branch once, and reads every branch tip with one `git for-each-ref refs/heads refs/remotes`.
- The caption compares the local default branch with its local tracking ref `origin/<default>` (cached like any other pair) and records the tracking tip it read. Its counts also choose the default-branch target, so a warm run makes no `merge-base` call. The library listing path never touches the network; the CLI reads it after its worker's check and fetch (see [Live Remote Head](#live-remote-head)).
- Per-worktree `git status --porcelain` and per-branch comparisons (`git rev-list --left-right --count` plus a speculative `git merge-tree --write-tree`, against the target and, for a branch whose fork parent is another branch, against the parent) are dispatched in parallel via `std::thread::scope`.
- `git status` is passed `-c core.untrackedCache=true`; benchmarks assume a warm untracked-cache so the measurement reflects steady-state behavior rather than the first cold walk.
- The intended Criterion surface benchmarks `list_worktrees()` end-to-end in the `rusty-biscuit` monorepo, using the Phase 1 `count-git` recorder to assert subprocess counts in addition to wall-clock time.

## Graph Data Collection

The second owned cost center is graph data collection in [`worktree/cli/src/commands/git_graph.rs`](../../worktree/cli/src/commands/git_graph.rs).

- [`gather`](../../worktree/cli/src/commands/git_graph.rs) reads whether the repository is shallow once, then works in three stages, each stage's branches concurrently: it classifies every drawn branch (one `merge-base --is-ancestor` per candidate lane, plus `rev-list --first-parent --parents` and `rev-list --ancestry-path` for a merged branch) and finds its fork (`merge-base`); it builds every branch lane (`log --first-parent`); then it builds the default lane. Each fork, merge, or label commit outside a lane's window costs one `rev-list --first-parent --count`, and a lane's anchors are verified together in one `log --no-walk`. See [git-graph.md](./git-graph.md).
- Graph data is only gathered when the terminal reports inline-image support. On non-image terminals the entire graph-data path is skipped.
- `GitGraph` measures each trimming candidate with the renderer (`GitGraph::plan`). That cost is in the `graph image render` stage, never in `list gather` or the table.

## PR Request

`wt list` makes at most one repository-wide open-PR request in the foreground, and only on a miss ([`pull_requests.rs`](../lib/src/pull_requests.rs)).

- Every stored answer is bound to a digest of the exact `git remote get-url origin` value, so every run pays that one git call, a cache hit included, before it may show stored badges.
- A matching answer younger than 60 s is used as is. An older one is still shown at once, with its age; the worker every listing launches (below) refreshes it and never delays the listing for it. The worker holds a lock beside the store from its freshness recheck to publication, so concurrent workers make at most one request, and its answer is shown by the next run.
- Only a miss (no store, another or no `origin`, an older format, a corrupt file, or a future fetch time) makes the request in the foreground, under a 300 ms deadline, and it settles *before* the worker is launched (the request never waits for the worker's lock; its answer is stored only if that lock is free afterwards and no other answer appeared meanwhile), so the worker finds the answer and never repeats the request. `-r`/`--ff` leave the miss to their forced worker, and a repository in `~/.wt.json` makes no request. A failure is never stored.
- The `--perf` stage is `pr gather`: the origin lookup and, on a miss, the request. It never adds to `list gather`.

## Live Remote Head

Every listing with an `origin` checks the remote and, when it differs, fetches the one tracking ref, through a detached worker it waits for up to 3 s ([`remote_update.rs`](../lib/src/remote_update.rs), [`list/wait.rs`](../cli/src/commands/list/wait.rs)).

- `wt list` launches (or adopts) one `wt internal-refresh <main checkout> --attempt <id>` and polls `<repo hash>.remote-head.json`'s `attempt` record every 25 ms until that attempt has an outcome or 3 s pass. Work still running at 3 s continues in the background, and the caption says so ("still checking" or "still pulling"). Refs, counts, and the graph are gathered *after* the wait, so they describe the post-fetch state.
- The worker's check asks the provider API, then `git ls-remote`, within one 10 s budget (`REMOTE_HEAD_REFRESH_DEADLINE`), and its fetch of `refs/remotes/origin/<default>` has its own 60 s deadline (`FETCH_DEADLINE`). Both kill the whole process tree at the deadline ([`live_remote.rs`](../lib/src/live_remote.rs)), so no worker holds `remote-head.lock` forever.
- `-r`/`--refresh` and `--ff` wait for the whole attempt and its completion receipt, bounded by `ATTEMPT_MAX_AGE` (75 s); in practice by the worker's own deadlines. Each forced attempt writes its own receipt file (`<repo hash>.refresh-receipt.<attempt id>.json`), so two overlapping `-r` runs each read their own; the run deletes it when its wait ends, and the next forced worker deletes any older than 75 s.
- The `--perf` stages are `remote wait` (launch through outcome or budget) and `fast-forward` (`--ff` only).

### The full-command contract

| Case | Bound | Gate |
| --- | --- | --- |
| Local gather plus render, worker answering at once | 1 s | `perf_command_sla::perf_full_command_non_image_meets_sla` |
| Ordinary listing, check held by `origin` | 3 s wait plus 1 s | `perf_pr_request::perf_a_held_live_head_check_costs_the_listing_only_its_wait` |
| Ordinary listing, fetch held by `origin` | 3 s wait plus 1 s | `perf_pr_request::perf_a_held_fetch_costs_the_listing_only_its_wait` |
| `-r`, check held by `origin` | 10 s check deadline plus 3 s | `perf_pr_request::perf_refresh_against_a_held_check_reports_within_the_check_deadline` |
| `--ff`, fetch held by `origin` | 60 s fetch deadline plus 3 s | `perf_pr_request::perf_fast_forward_against_a_held_fetch_reports_within_the_fetch_deadline` |

The 1 s bound is no longer the whole command's: spec decision 1 of `2026-09-27-list-freshness-ux` replaced it with a bounded remote wait. The held cases prove the listing never joins its worker: each asserts that the captured `.output()` returned while the worker was still held (the held-check case by the live-head lock, the held-fetch case by the worker process and the gate's run count). The deterministic proofs are in `tests/list_prs.rs` (`a_held_live_head_check_holds_the_listing_only_until_its_deadline`) and `tests/list_remote_head.rs` (`a_fetch_still_running_at_the_deadline_is_still_pulling_and_publishes_after_the_listing`).

## Ahead/Behind + Merge Result Cache

`list_worktrees()` caches the expensive deterministic branch-comparison result described in the feature spec's [Approach (decided: cache + concurrency)](../features/2026-06-16-two-problems/spec.md#approach-decided-cache--concurrency): `(target_tip_sha, branch_tip_sha, CACHE_FORMAT_VERSION) -> { ahead, behind, is_clean }`. One cache serves the `-> {default}` column, the `-> parent` column, and the caption (format version 2, since `2026-09-24-ux-improvements`).

- The SHA-pair key is deterministic and self-invalidating. If the target tip or the branch tip moves, the next lookup uses a different key and recomputes the result.
- Cache files live under `dirs::cache_dir()/worktree/<repo-root-hash>.json`, where the hash is derived from the canonical repo root path with `biscuit-hash` xxHash.
- Cache writes use write-temp-then-rename atomic replacement. Concurrent writers have last-rename-wins semantics, and readers never observe torn JSON.
- `CACHE_FORMAT_VERSION` is part of each key and the persisted file header; bump it when the on-disk shape or semantics change.
- Stale entries for superseded SHA pairs are tolerated opportunistically because they are unreachable until the same pair appears again.
- Working-tree dirtiness is never cached. The `git status --porcelain` walk still runs live for every listing.

## Verbose Commit Details

The third owned cost center is the verbose commit block rendered by `wt list -v`.

- Verbose details are gathered independently of image support: they render as text on non-image terminals and accompany the graph on image-capable terminals.
- When both graph and verbose data are needed for the current branch, one `gather(..., needs_graph, needs_verbose)` call populates both surfaces from a single `merge-base`.
- The intended Criterion surface benchmarks the verbose-only gather path on a non-image terminal, asserting that exactly one `merge-base` and the expected number of `git log` calls are issued for the current branch.

## Excluded Surfaces

- **Mermaid → SVG → rasterized image rendering** is excluded from worktree-owned benchmarks. That pipeline lives in `biscuit-terminal` / `biscuit-visualized` and is tracked separately.
- Shell startup, argument parsing, and table rendering are not the dominant costs owned by this package and are not targeted here.

## Bench Recipe

The library-owned gather surface is benchmarked with Criterion in [`worktree/lib/benches/list_status.rs`](../../worktree/lib/benches/list_status.rs). The bench runs end-to-end against `list_worktrees()` in the ambient `rusty-biscuit` checkout, using `iter_batched_ref` with a throughput of one element per iteration so the report shows calls/sec.

Run the benches from the package area with:

```sh
just -d worktree bench
```

The HTML report is written to `target/criterion/report/index.html`.

Use the shared baseline workflow to compare before and after a change:

```sh
just -d worktree bench-save   # capture this host's baseline
just -d worktree bench-compare  # compare the current run against it
```

The shared `_bench_preflight` recipe checks battery, memory, and load before running, and `_bench_id` produces a host-derived baseline ID so baselines do not accidentally migrate across machines.

## Measurement Methodology

Criterion benches, the `--perf` runtime diagnostic, and the `perf_*` tests together provide complementary measurement surfaces. The benches cover the library-owned gather stage, `--perf` covers the full CLI pipeline end-to-end, and the `perf_*` tests assert reproducible subprocess-count and wall-clock bounds.

### Runtime `--perf` flag

`wt list --perf` emits a per-stage timing report to stderr after the command completes. The report is rendered with `biscuit-terminal`'s `MetricsTree` inside a `BlockQuote` and shows only the stages that actually ran, plus an `unattributed` node so the tree reconciles to the wall-clock total. On a non-image terminal the graph-gather and graph-image-render stages are omitted, matching the package's exclusion of rasterization from worktree-owned benchmarks. Use this as a runtime diagnostic complement to the dev-time Criterion benches.

Run the perf tests with:

```sh
cargo nextest run -p worktree-cli -E 'test(/perf_/)' --nocapture
```

For contention-free wall-clock measurement of the SLA, run perf tests serially via `just test-perf`.

### `perf_subprocess_counts_meet_sla` (unit test, `list.rs`)

Asserts the subprocess-count bounds the optimization guarantees. Runs in the ambient `rusty-biscuit` checkout so the counts reflect real worktree scale:

- `list_worktrees()` resolves the default branch exactly once (one `symbolic-ref` call) and reads tips with one `for-each-ref`. Wall-clock is printed for observability; the full-command SLA that subsumes this piece is asserted by the integration test below.
- The base-view `gather` on that fixture, where every branch has only the default lane as a candidate, issues one `rev-parse` (the shallow check), one `merge-base --is-ancestor` and one fork `merge-base` per branch, and one `git log` per branch plus one for the default lane.

### `graph_and_verbose_share_one_merge_base` (unit test, `git_graph/tests.rs`)

Subprocess-count guard for the image-terminal `wt list -v` data-gather path (graph facts + verbose details) on a controlled feature-branch fixture: exactly one merge base, shared by the graph's fork and the verbose details, one `merge-base --is-ancestor` (the branch's classification), and zero `rev-parse --short`. Rasterization is excluded (this test never renders).

### `perf_full_command_non_image_meets_sla` (integration test, `tests/perf_command_sla.rs`)

Spawns the built `wt` binary with image-capable env vars removed so the non-image fast path is taken, and asserts the full command (process startup, the worker's check, table rendering, and all git data gathering) meets the 1-second SLA on a warm cache. The fixture's `origin` is a local bare copy in sync with the tracking ref (`MixedFixture::with_local_origin`), so every run launches the worker, which answers from local Git at once, and each run must report "checked just now". A warm-up primes caches, then the best of five timed runs is checked against the 1-second SLA; the best-of-5 minimum (rather than the mean) tolerates parallel-test-execution contention, and a true regression past the SLA fails this gate. Rasterization never runs on the non-image path, so it is excluded by construction.

The wall-clock and subprocess-count output is printed to stderr via `--nocapture`, providing a reproducible result trace. For a contention-free measurement, run perf tests serially via `just test-perf`.

## Ratified SLA Targets

Run the binding performance gates from the package area with:

```sh
just -d worktree test-perf
```

In environments where the local `just` wrapper requires explicit paths, the equivalent command is:

```sh
just --justfile worktree/justfile --working-directory worktree test-perf
```

The warm and cold cache gates measure the **`list gather` stage** parsed from `wt list --perf`, not full-command wall-clock. `list gather` is the stage the cache targets (it dominates a cold `wt list`); a full-command bound could pass while `list gather` itself regresses. Both gates run on a shared *mixed* fixture (`tests/perf_support/mod.rs`): one `main` checkout plus several divergent branches and several fast-forward and behind-only branches. The mix is deliberate — the divergent branches show the warm-cache collapse, while the fast-forward and behind-only branches bound the cold-path tradeoff of the speculative `merge-tree` now issued for every non-main branch (the cache cannot help a cache miss).

Ratified on 2026-06-17 against the mixed fixture (4 divergent + 3 fast-forward + 3 behind-only worktrees):

| Surface | Test | Achieved best-of-5 | Asserted bound |
| --- | --- | ---: | ---: |
| Warm-cache `list gather` | `perf_cache_warm_list_gather_meets_sla` | 19 ms | 120 ms |
| Cold-cache `list gather` | `perf_cache_cold_list_gather_meets_sla` | 38 ms | 300 ms |
| Ambient checkout, non-image full `wt list` | `perf_full_command_non_image_meets_sla` | 254.20 ms | 1 s |

Re-measured on 2026-09-25 after the list redesign (`2026-09-24-ux-improvements`), same fixture, on the macOS development host, plus the two PR cases in `tests/perf_pr_request.rs`. Every PR request goes to a local proxy stub; nothing leaves the host.

| Surface | Test | Achieved best-of-5 | Asserted bound |
| --- | --- | ---: | ---: |
| Warm-cache `list gather` | `perf_cache_warm_list_gather_meets_sla` | 12.8 ms | 120 ms |
| Cold-cache `list gather` | `perf_cache_cold_list_gather_meets_sla` | 23.6 ms | 300 ms |
| Mixed fixture, non-image full `wt list` | `perf_full_command_non_image_meets_sla` | 55.7 ms | 1 s |
| Network down (connection refused): cold / warm `list gather`, full `wt list` | `perf_list_meets_sla_with_the_network_down` | 21.0 ms / 10.5 ms / 52.9 ms | 300 ms / 120 ms / 1 s |
| PR request stalled until its 300 ms deadline: warm `list gather`, full `wt list` | `perf_list_meets_sla_when_the_pr_request_hits_its_deadline` | 11.6 ms / 359.7 ms | 120 ms / 1 s |

In the stalled case every run's `pr gather` stage was 309–317 ms, and the test asserts it is at least the deadline, which proves the request was made and waited for. A fresh store making no request is an L1 behavior test (`tests/list_prs.rs`), not a timing gate. The Criterion `list_status/warm` bench measured 79 ms per `list_worktrees()` on the ambient `rusty-biscuit` checkout.

Re-measured on 2026-09-26 after stale answers stopped waiting for a request (`2026-09-25-list-remove-performance`), same fixture, on the macOS development host, with `just -d worktree test-perf` run on its own:

| Surface | Test | Achieved best-of-5 | Asserted bound |
| --- | --- | ---: | ---: |
| Warm-cache `list gather` | `perf_cache_warm_list_gather_meets_sla` | 12.5 ms | 120 ms |
| Cold-cache `list gather` | `perf_cache_cold_list_gather_meets_sla` | 25.3 ms | 300 ms |
| Mixed fixture, non-image full `wt list` | `perf_full_command_non_image_meets_sla` | 63.0 ms | 1 s |
| Network down (connection refused): cold / warm `list gather`, full `wt list` | `perf_list_meets_sla_with_the_network_down` | 25.8 ms / 12.8 ms / 63.5 ms | 300 ms / 120 ms / 1 s |
| No stored answer, request stalled until its 300 ms deadline: warm `list gather`, full `wt list` | `perf_list_meets_sla_when_the_pr_request_hits_its_deadline` | 12.7 ms / 366.2 ms | 120 ms / 1 s |
| Stale matching answer, refresh blocked: full `wt list`; every sample's `pr gather` | `perf_list_meets_sla_with_a_stale_answer_and_a_blocked_refresh` | 62.4 ms; 6.7–11.0 ms | 1 s; under 300 ms |

- **The stalled case is now the miss path.** It seeds no store, so its request is a foreground one; `pr gather` was 310–326 ms.
- **The stale gate** points `origin` at a hanging local proxy and seeds a 12-minute-old matching answer for every sample, then checks that the store is still stale afterwards, so a worker that succeeded cannot turn it into a fresh-cache measurement. Each sample must show the stored badge and `PRs as of 12 min ago`.
- **Timing alone cannot prove the parent does not wait.** A 1 s ceiling would still pass a reintroduced 300 ms wait, so the gate also asserts every sample's `pr gather` stays under the request deadline. The L1 tests in `tests/list_prs.rs` prove it deterministically: `a_stale_store_shows_its_badges_at_once_and_a_detached_worker_makes_the_request` and `a_detached_workers_answer_replaces_the_stale_one_on_the_next_list` capture `wt list`'s output, which returns only once every holder of its pipes has exited, while the worker's request is still held unanswered and its lock still taken.
- **Fresh and stale cost the same.** The same run measured a fresh answer at 63.3 ms full command and 6.9 ms `pr gather`, and the stale answer at 62.4 ms. The `pr gather` time is the added `git remote get-url origin` call that binds the stored answer to `origin`; before this change a fresh answer's `pr gather` took 0.03 ms. The spec's measurement before the change was 338 ms for a stale answer against 83 ms fresh on the ambient checkout.

Re-measured on 2026-09-27 after every listing began checking `origin` and waiting for it (`2026-09-27-list-freshness-ux`), same fixture, on the macOS development host, with `just -d worktree test-perf` run on its own (26 passed). The full-command bound is now **1 s for local gather plus render with a worker that answers at once, and 3 s plus that with a stalled worker**; `-r` and `--ff` are bounded by the worker's 10 s check and 60 s fetch deadlines (see [The full-command contract](#the-full-command-contract)).

| Surface | Test | Achieved | Asserted bound |
| --- | --- | ---: | ---: |
| Warm-cache `list gather` | `perf_cache_warm_list_gather_meets_sla` | 12.1 ms | 120 ms |
| Cold-cache `list gather` | `perf_cache_cold_list_gather_meets_sla` | 23.0 ms | 300 ms |
| Mixed fixture with a local `origin`, non-image full `wt list`, worker answering at once | `perf_full_command_non_image_meets_sla` | 143.4 ms | 1 s |
| Network down (connection refused): cold / warm `list gather`, full `wt list` | `perf_list_meets_sla_with_the_network_down` | 22.1 ms / 12.1 ms / 161.4 ms | 300 ms / 120 ms / 1 s |
| No stored answer, request held past its 300 ms deadline: warm `list gather`, full `wt list` | `perf_list_meets_sla_when_the_pr_request_hits_its_deadline` | 13.4 ms / 886.7 ms | 120 ms / 1 s + 300 ms |
| Fresh answer / stale answer with a failing refresh: full `wt list`; every stale sample's `pr gather` | `perf_list_meets_sla_with_a_stale_answer_and_a_failing_refresh` | 154.0 ms / 150.0 ms; 4.1–4.3 ms | 1 s; under 300 ms |
| Check held by `origin`: `remote wait`, full `wt list` (two listings, the second adopting) | `perf_a_held_live_head_check_costs_the_listing_only_its_wait` | 3.00 s, 3.11–3.13 s | [3 s, 3.3 s), 4 s |
| Fetch held by `origin`: `remote wait`, full `wt list` | `perf_a_held_fetch_costs_the_listing_only_its_wait` | 3.00 s, 3.10 s | [3 s, 3.3 s), 4 s |
| `wt list -r`, check held by `origin` | `perf_refresh_against_a_held_check_reports_within_the_check_deadline` | 10.19 s | [10 s, 13 s) |
| `wt --ff`, fetch held by `origin` | `perf_fast_forward_against_a_held_fetch_reports_within_the_fetch_deadline` | 60.20 s | [60 s, 63 s) |

- **Every full-command sample now includes the worker's launch and check.** Fixtures without a live `origin` answer in milliseconds (a local bare copy, or a refused or closed connection), which is the 100 ms rise over the 2026-09-26 figures: the worker is a second `wt` process whose check the listing waits for.
- **The PR-deadline case adds the miss request to the wait.** The foreground request (≤ 300 ms) settles before the worker is launched, so the worker finds the answer and never repeats it; the worker's check then fails at the stub's 400 ms close. Its bound is therefore 1 s plus the request deadline.
- **The held cases prove no join.** The captured `.output()` returns while the worker is still held: the live-head lock is still taken (held check), or the worker process still runs and `upload-pack` has started exactly twice (held fetch). A held check uses `HoldingOrigin` (loopback smart HTTP); a held fetch uses `remote_fixture::UploadPackGate` on a local bare `origin`.
- **`-r` and `--ff` wait for the whole attempt**, so their elapsed time is the worker's own deadline plus about 0.2 s: the transport's whole process tree is killed at the deadline, then the outcome and receipt are published and the listing renders. The `--ff` case has a 90 s nextest termination override in `.config/nextest.toml`, since the default ceiling is 30 s.

## Graph Stages

`graph gather` and `graph image render (biscuit-terminal)` appear in `wt list --perf` only when stderr is a terminal and `TERM_PROGRAM` names an image-capable emulator, so `tests/perf_graph_stages.rs` runs `wt list --perf` under `script` in a 120×40 pseudo-terminal, emulating Ghostty (other Kitty-protocol emulators wait out a 1 s cursor-position query there). The fixtures are `GraphFixture` in `tests/perf_support/graph.rs`, built through `git fast-import`. A second test, `perf_graph_stages_for_the_observed_sparse_lanes_at_200x60`, runs one more shape at 200×60 and prints each stage's median with its min–max spread. That shape (`GraphFixture::observed_sparse_lanes`) has three long branch lanes with recorded parents, and one of those branches is merged directly into `main` after its parent merged `main` back. The tests assert only that both stages are reported: the graph has no numeric budget, and the full command's 1 s bound (`perf_full_command_non_image_meets_sla`) still applies. Record medians with:

```sh
WT_GRAPH_PERF_SAMPLES=10 just test-perf perf_graph --cargo-profile release
```

Measured on the macOS development host (Apple M4 Max, git 2.55.0), release profile, 10 samples per fixture after one warm-up, medians. *Before* is the graph that drew merged branches as tags; *after* adds first-parent lanes, merge detection, anchors, and the tag-spacing layout (both 2026-09-27):

| Fixture | `graph gather` before | after | change | `graph image render` before | after | change |
|---|---:|---:|---:|---:|---:|---:|
| floor (one commit) | 5.9 ms | 10.5 ms | +4.6 ms | 352.4 ms | 347.0 ms | −1.5% |
| ordinary (3 worktrees, 50 commits, base view) | 20.2 ms | 39.1 ms | +18.9 ms | 351.2 ms | 356.1 ms | +1.4% |
| older essential connections (fork 5,000 and merge 3,000 first-parent commits back, focused view) | 69.5 ms | 126.3 ms | +56.8 ms | 353.3 ms | 351.8 ms | −0.4% |
| multiple selected branches (8 worktrees, 2 merged, base view) | 32.4 ms | 65.3 ms | +32.9 ms | 351.8 ms | 355.2 ms | +1.0% |

- **Gather roughly doubles, by added sequential git calls.** Each `git` spawn costs about 5 ms here. The floor's whole rise is the one added shallow check. Elsewhere the stages run one after another: shallow check, then per branch the `--is-ancestor` check, the two `rev-list` walks for a merged branch, and the fork `merge-base`; then the branch lanes' `log`; then the default lane's `log`, its anchors' `rev-list --count` calls (concurrent), and the one verifying `log --no-walk`. Before, a branch cost one `merge-base` and one `log`, concurrently. The older-connections fixture adds the most, because its merge's `rev-list` walks and its anchor counts cross thousands of commits.
- **Render is unchanged within noise.** It is dominated by about 350 ms of fixed cost that exists only in a pseudo-terminal: terminal detection waits for answers that never come (compare every row with the floor). A second layout pass runs only when two tags collide, and stays below that noise.
- **The full-command gates still pass** (`just -d worktree test-perf`, 2026-09-27), including `perf_full_command_non_image_meets_sla`, which never gathers a graph.

A later change (2026-09-28) made tag spacing collision-driven and let a later lane's direct merge win over an earlier lane's indirect containment. Same host, command, and profile, 10 samples, median (min–max):

| Fixture | Size | `graph gather` before | after | `graph image render` before | after |
|---|---|---:|---:|---:|---:|
| observed sparse lanes | 200×60 | 61.0 ms (58.8–70.3) | 75.2 ms (73.8–85.6) | 351.4 ms (350.0–357.8) | 348.4 ms (341.7–351.6) |
| floor | 120×40 | 10.9 ms | 10.9 ms | 341.8 ms | 344.4 ms |
| ordinary | 120×40 | 38.9 ms | 41.1 ms | 345.8 ms | 347.6 ms |
| older essential connections | 120×40 | 130.9 ms | 135.4 ms | 347.6 ms | 344.0 ms |
| multiple selected branches | 120×40 | 64.1 ms | 65.3 ms | 348.6 ms | 346.3 ms |

- **Gather rises 14 ms on the observed shape**, and the ranges do not overlap. The branch its parent contains only indirectly is now also checked against `main`, which merged it directly, and that merge's anchors are then placed on the default lane. The other fixtures moved within noise.
- **Render is unchanged.** None of these fixtures has colliding tags (the observed shape's `main` and `origin/main` share one commit), so each is laid out once.

## `git status` Cost (Investigated, Not Changed)

`wt list` runs one default `git status` per worktree, in parallel. On the ambient `rusty-biscuit` checkout (13,013 tracked files, APFS, 2026-09-25) each took about 30 ms of wall time but about 0.22 s of system CPU:

| `git status` variant | Wall time | System CPU |
| --- | ---: | ---: |
| `--untracked-files=all` (used by `wt remove`, which must list every file) | 80 ms | 0.25 s |
| default, untracked folders collapsed (used by `wt list`) | 30 ms | 0.22 s |
| `--untracked-files=no` | 20 ms | 0.19 s |
| default, with `core.preloadIndex=false` | 150 ms | 0.09 s |

- The system CPU is the kernel checking every tracked file for changes. `core.preloadIndex` spreads that over threads, roughly doubling the CPU to cut the wait; turning it off trades 0.13 s of CPU for 120 ms of wall time, the wrong trade for an interactive command.
- Only a file-system watcher avoids checking every file. Git's own (`core.fsmonitor`) runs a daemon per worktree and exists only on macOS and Windows; on Linux it needs Watchman plus a hook.
- Ruled 2026-09-25: no watcher and no daemon. The wall time is already small, and the status flags are unchanged.

The warm gate also asserts that warm `list gather` is below a cold reference measured in the same run, proving the cache collapses the divergent-branch recompute rather than the host merely being fast. Bounds are looser than the ratified measurements so ordinary host variance does not fail CI, yet tight enough to catch regressions that reintroduce serial branch comparison, skip the cache, or let the cold-path speculative `merge-tree` blow the budget. Deterministic subprocess-count assertions for cache hit/miss behavior live in the recorder-backed unit tests.

## Track 2 — Aspect Ratio (Parked)

Not active work. Recorded so the investigation is not lost if it recurs.

- **Observation history:** earlier `wt --perf` runs rendered the git-graph image squished or too tall in some worktrees but correct in others with the same binary. Current runs render correctly, so the issue appears intermittent or already resolved.
- **Confirmed rendering path:** `wt` -> `biscuit_terminal::components::mermaid::MermaidDiagram` (display: cells to pixels via `term.cell_size()`, terminal image protocol) -> `biscuit_visualized` (`MermaidDiagram` / `MermaidRenderer`) for SVG generation and rasterization. The CLI uses the existing biscuit-terminal component.
- **Likely origin if it recurs:** the terminal/raster path preserves aspect ratio. Mis-proportion would originate in `mermaid_rs_renderer::compute_layout` at `biscuit-visualized/src/src/mermaid/render.rs:223-225`, which can produce a near-square or padded canvas for small gitGraphs.
- **Measured clue:** broken cached gitGraph PNGs were near-square (`h/w` about `0.9-1.3`) and small; healthy graphs were wide and short (`h/w` about `0.25-0.45`). Branches further behind `main` get more horizontal commits, making them wider and less likely to show the issue.
- **Candidate fix:** in biscuit-visualized, re-fit the SVG canvas to its content bounding box before rasterizing at `raster/png.rs::render_tree_to_pixmap`. That covers both the at-width and scale rasterization entry points. A content-bbox crop normalizes dead space but does not widen an intrinsically near-square small graph; making small graphs always wide is a separate layout-level change.
- **Cache note:** bump the cache backend id (`MERMAID_BACKEND`) on any rendering-cache-affecting change.