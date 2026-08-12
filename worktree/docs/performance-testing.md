---
hash: ef46db3751d8e999-7b04bc36d9aed8f0
last_updated: 2026-08-12
---

# Performance Testing — Worktree

This document defines the worktree-owned performance surfaces for `wt list` and related commands. It scopes what should be benchmarked, what assumptions the benchmarks rely on, and what is intentionally excluded.

## List Status Collection

The first owned cost center is [`list_worktrees`](../../worktree/lib/src/worktree.rs), which produces the status table.

- It runs `git worktree list --porcelain` and resolves the default branch once.
- Per-worktree work (`git status --porcelain`, `git rev-list --count`, and `git merge-tree --write-tree` when histories diverge) is dispatched in parallel via `std::thread::scope`.
- `git status` is passed `-c core.untrackedCache=true`; benchmarks assume a warm untracked-cache so the measurement reflects steady-state behavior rather than the first cold walk.
- The intended Criterion surface benchmarks `list_worktrees()` end-to-end in the `rusty-biscuit` monorepo, using the Phase 1 `count-git` recorder to assert subprocess counts in addition to wall-clock time.

## Graph Data Collection

The second owned cost center is graph data collection in [`worktree/cli/src/commands/git_graph.rs`](../../worktree/cli/src/commands/git_graph.rs).

- [`gather_branch`](../../worktree/cli/src/commands/git_graph.rs) collects the selected merge-base, shared context commits, and commits unique to each tip for a single feature branch.
- [`gather_base_graph`](../../worktree/cli/src/commands/git_graph.rs) collects the selected merge-base and branch-tip-unique commits for every active worktree branch concurrently, then sorts results deterministically before rendering.
- Graph data is only gathered when the terminal reports inline-image support **and** the parsed width fits the minimum terminal width threshold. On non-image terminals the entire graph-data path is skipped.
- The intended Criterion surface benchmarks both `gather_branch` (focused-branch scenario) and `gather_base_graph` (base-overview scenario), again using the Phase 1 `count-git` recorder to verify one `merge-base` per branch and zero `rev-parse --short` calls.

## Ahead/Behind + Merge Result Cache

`list_worktrees()` caches the expensive deterministic branch-comparison result described in the feature spec's [Approach (decided: cache + concurrency)](../features/2026-06-16-two-problems/spec.md#approach-decided-cache--concurrency): `(default_tip_sha, branch_tip_sha, CACHE_FORMAT_VERSION) -> { ahead, behind, is_clean }`.

- The SHA-pair key is deterministic and self-invalidating. If the default branch tip or worktree branch tip moves, the next lookup uses a different key and recomputes the result.
- Cache files live under `dirs::cache_dir()/worktree/<repo-root-hash>.json`, where the hash is derived from the canonical repo root path with `biscuit-hash` xxHash.
- Cache writes use write-temp-then-rename atomic replacement. Concurrent writers have last-rename-wins semantics, and readers never observe torn JSON.
- `CACHE_FORMAT_VERSION` is part of each key and the persisted file header; bump it when the on-disk shape or semantics change.
- Stale entries for superseded SHA pairs are tolerated opportunistically because they are unreachable until the same pair appears again.
- Working-tree dirtiness is never cached. The `git status --porcelain` walk still runs live for every listing.

## Verbose Commit Details

The third owned cost center is the verbose commit block rendered by `wt list -v`.

- Verbose details are gathered independently of image support: they render as text on non-image terminals and accompany the graph on image-capable terminals.
- When both graph and verbose data are needed for the current branch, a single `gather_branch(..., verbose = true)` call populates both surfaces.
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

- `list_worktrees()` resolves the default branch exactly once (one `symbolic-ref` call). Wall-clock is printed for observability; the full-command SLA that subsumes this piece is asserted by the integration test below.
- `gather_base_graph()` issues exactly one `merge-base` per branch and one unique-tip `git log` per branch plus one for main commits — no discarded default-context or default-unique logs.
- `gather_branch()` wall-clock is printed for observability when the ambient checkout is a feature branch (skipped on main). The binding SLA + subprocess-count assertions for the image-terminal `wt list -v` data-gather path live in `gather_branch_uses_one_merge_base_and_no_short_sha` below, which runs on a controlled fixture so it always asserts regardless of the ambient checkout.

### `gather_branch_uses_one_merge_base_and_no_short_sha` (unit test, `git_graph.rs`)

Binding SLA guard for the image-terminal `wt list -v` data-gather path (graph IDs + verbose details). Runs `gather_branch(default, feature, verbose=true)` on a temporary non-main fixture so the path always executes — never skipped on a main checkout. Asserts exactly one `merge-base`, zero `rev-parse --short`, and completion under the 1-second SLA with rasterization excluded (this test never invokes `MermaidDiagram::render`).

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

The warm gate also asserts that warm `list gather` is below a cold reference measured in the same run, proving the cache collapses the divergent-branch recompute rather than the host merely being fast. Bounds are looser than the ratified measurements so ordinary host variance does not fail CI, yet tight enough to catch regressions that reintroduce serial branch comparison, skip the cache, or let the cold-path speculative `merge-tree` blow the budget. Deterministic subprocess-count assertions for cache hit/miss behavior live in the recorder-backed unit tests.

## Track 2 — Aspect Ratio (Parked)

Not active work. Recorded so the investigation is not lost if it recurs.

- **Observation history:** earlier `wt --perf` runs rendered the git-graph image squished or too tall in some worktrees but correct in others with the same binary. Current runs render correctly, so the issue appears intermittent or already resolved.
- **Confirmed rendering path:** `wt` -> `biscuit_terminal::components::mermaid::MermaidDiagram` (display: cells to pixels via `term.cell_size()`, terminal image protocol) -> `biscuit_visualized` (`MermaidDiagram` / `MermaidRenderer`) for SVG generation and rasterization. The CLI uses the existing biscuit-terminal component.
- **Likely origin if it recurs:** the terminal/raster path preserves aspect ratio. Mis-proportion would originate in `mermaid_rs_renderer::compute_layout` at `biscuit-visualized/src/src/mermaid/render.rs:223-225`, which can produce a near-square or padded canvas for small gitGraphs.
- **Measured clue:** broken cached gitGraph PNGs were near-square (`h/w` about `0.9-1.3`) and small; healthy graphs were wide and short (`h/w` about `0.25-0.45`). Branches further behind `main` get more horizontal commits, making them wider and less likely to show the issue.
- **Candidate fix:** in biscuit-visualized, re-fit the SVG canvas to its content bounding box before rasterizing at `raster/png.rs::render_tree_to_pixmap`. That covers both the at-width and scale rasterization entry points. A content-bbox crop normalizes dead space but does not widen an intrinsically near-square small graph; making small graphs always wide is a separate layout-level change.
- **Cache note:** bump the cache backend id (`MERMAID_BACKEND`) on any rendering-cache-affecting change.