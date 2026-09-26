---
hash: ef46db3751d8e999-7b04bc36d9aed8f0
last_updated: 2026-08-12
---

# Performance Testing — Worktree

This document defines the worktree-owned performance surfaces for `wt list` and related commands. It scopes what should be benchmarked, what assumptions the benchmarks rely on, and what is intentionally excluded.

## List Status Collection

The first owned cost center is [`list_worktrees`](../../worktree/lib/src/worktree.rs), which produces the status table.

- It runs `git worktree list --porcelain`, resolves the default branch once, and reads every branch tip with one `git for-each-ref refs/heads refs/remotes`.
- The caption compares the local default branch with `origin/<default>` (cached like any other pair). Its counts also choose the default-branch target, so a warm run makes no `merge-base` call.
- Per-worktree `git status --porcelain` and per-branch comparisons (`git rev-list --left-right --count` plus a speculative `git merge-tree --write-tree`, against the target and, for a branch whose fork parent is another branch, against the parent) are dispatched in parallel via `std::thread::scope`.
- `git status` is passed `-c core.untrackedCache=true`; benchmarks assume a warm untracked-cache so the measurement reflects steady-state behavior rather than the first cold walk.
- The intended Criterion surface benchmarks `list_worktrees()` end-to-end in the `rusty-biscuit` monorepo, using the Phase 1 `count-git` recorder to assert subprocess counts in addition to wall-clock time.

## Graph Data Collection

The second owned cost center is graph data collection in [`worktree/cli/src/commands/git_graph.rs`](../../worktree/cli/src/commands/git_graph.rs).

- [`gather`](../../worktree/cli/src/commands/git_graph.rs) collects, for the focused view, the current branch (and its recorded fork parent) with one `merge-base` each, and, for the base view, one `merge-base` and one `git log` per worktree branch, concurrently. See [git-graph.md](./git-graph.md).
- Graph data is only gathered when the terminal reports inline-image support. On non-image terminals the entire graph-data path is skipped.
- `GitGraph` measures each trimming candidate with the renderer (`GitGraph::plan`). That cost is in the `graph image render` stage, never in `list gather` or the table.

## PR Request

`wt list` makes one repository-wide open-PR request on its own thread, beside the git work ([`pull_requests.rs`](../lib/src/pull_requests.rs)).

- Stored results younger than 60 s are used without a request (and without the `git remote get-url` call).
- Otherwise the request has a 300 ms deadline. On failure or timeout the stored results are shown with their age; a failure is never stored.
- The `--perf` stage is `pr gather`. It never adds to `list gather`, and it bounds how long the table can wait for the network.

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
- The base-view `gather` issues exactly one `merge-base` per branch and one unique-tip `git log` per branch plus one for the default lane.

### `graph_and_verbose_share_one_merge_base` (unit test, `git_graph/tests.rs`)

Subprocess-count guard for the image-terminal `wt list -v` data-gather path (graph facts + verbose details) on a controlled feature-branch fixture: exactly one `merge-base` and zero `rev-parse --short`. Rasterization is excluded (this test never renders).

### `perf_full_command_non_image_meets_sla` (integration test, `tests/perf_command_sla.rs`)

Spawns the built `wt` binary with image-capable env vars removed so the non-image fast path is taken, and asserts the full command (process startup, table rendering, and all git data gathering) meets the 1-second SLA on a warm cache. A warm-up primes caches, then the best of five timed runs is checked against the 1-second SLA; the best-of-5 minimum (rather than the mean) tolerates parallel-test-execution contention, and a true regression past the SLA fails this gate. Rasterization never runs on the non-image path, so it is excluded by construction.

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

The warm gate also asserts that warm `list gather` is below a cold reference measured in the same run, proving the cache collapses the divergent-branch recompute rather than the host merely being fast. Bounds are looser than the ratified measurements so ordinary host variance does not fail CI, yet tight enough to catch regressions that reintroduce serial branch comparison, skip the cache, or let the cold-path speculative `merge-tree` blow the budget. Deterministic subprocess-count assertions for cache hit/miss behavior live in the recorder-backed unit tests.

## Track 2 — Aspect Ratio (Parked)

Not active work. Recorded so the investigation is not lost if it recurs.

- **Observation history:** earlier `wt --perf` runs rendered the git-graph image squished or too tall in some worktrees but correct in others with the same binary. Current runs render correctly, so the issue appears intermittent or already resolved.
- **Confirmed rendering path:** `wt` -> `biscuit_terminal::components::mermaid::MermaidDiagram` (display: cells to pixels via `term.cell_size()`, terminal image protocol) -> `biscuit_visualized` (`MermaidDiagram` / `MermaidRenderer`) for SVG generation and rasterization. The CLI uses the existing biscuit-terminal component.
- **Likely origin if it recurs:** the terminal/raster path preserves aspect ratio. Mis-proportion would originate in `mermaid_rs_renderer::compute_layout` at `biscuit-visualized/src/src/mermaid/render.rs:223-225`, which can produce a near-square or padded canvas for small gitGraphs.
- **Measured clue:** broken cached gitGraph PNGs were near-square (`h/w` about `0.9-1.3`) and small; healthy graphs were wide and short (`h/w` about `0.25-0.45`). Branches further behind `main` get more horizontal commits, making them wider and less likely to show the issue.
- **Candidate fix:** in biscuit-visualized, re-fit the SVG canvas to its content bounding box before rasterizing at `raster/png.rs::render_tree_to_pixmap`. That covers both the at-width and scale rasterization entry points. A content-bbox crop normalizes dead space but does not widen an intrinsically near-square small graph; making small graphs always wide is a separate layout-level change.
- **Cache note:** bump the cache backend id (`MERMAID_BACKEND`) on any rendering-cache-affecting change.