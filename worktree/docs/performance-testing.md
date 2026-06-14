---
hash: ef46db3751d8e999-6dae13caf54390d4
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

- [`gather_branch`](../../worktree/cli/src/commands/git_graph.rs) collects merge-base, context commits, and post-divergence commits for a single feature branch.
- [`gather_base_graph`](../../worktree/cli/src/commands/git_graph.rs) collects the same data for every active worktree branch concurrently, then sorts results deterministically before rendering.
- Graph data is only gathered when the terminal reports inline-image support **and** the parsed width fits the minimum terminal width threshold. On non-image terminals the entire graph-data path is skipped.
- The intended Criterion surface benchmarks both `gather_branch` (focused-branch scenario) and `gather_base_graph` (base-overview scenario), again using the Phase 1 `count-git` recorder to verify one `merge-base` per branch and zero `rev-parse --short` calls.

## Verbose Commit Details

The third owned cost center is the verbose commit block rendered by `wt list -v`.

- Verbose details are gathered independently of image support: they render as text on non-image terminals and accompany the graph on image-capable terminals.
- When both graph and verbose data are needed for the current branch, a single `gather_branch(..., verbose = true)` call populates both surfaces.
- The intended Criterion surface benchmarks the verbose-only gather path on a non-image terminal, asserting that exactly one `merge-base` and the expected number of `git log` calls are issued for the current branch.

## Excluded Surfaces

- **Mermaid → SVG → rasterized image rendering** is excluded from worktree-owned benchmarks. That pipeline lives in `biscuit-terminal` / `biscuit-visualized` and is tracked separately.
- Shell startup, argument parsing, and table rendering are not the dominant costs owned by this package and are not targeted here.

## Bench Recipe

The worktree package `justfile` does not define a `bench` recipe today. Adding Criterion benchmarks and wiring them through the shared `/just` bench pattern is a follow-up; until then, this document defines the surfaces a future benchmark suite should cover.

## Measurement Methodology

Until Criterion benchmarks are wired, the `perf_*` tests plus the `gather_branch_uses_one_merge_base_and_no_short_sha` fixture test provide reproducible subprocess-count and wall-clock measurements. Run the perf tests with:

```sh
cargo nextest run -p worktree-cli -E 'test(/perf_/)' --nocapture
```

For contention-free wall-clock measurement of the SLA, run perf tests serially via `just test-perf`.

### `perf_subprocess_counts_meet_sla` (unit test, `list.rs`)

Asserts the subprocess-count bounds the optimization guarantees. Runs in the ambient `rusty-biscuit` checkout so the counts reflect real worktree scale:

- `list_worktrees()` resolves the default branch exactly once (one `symbolic-ref` call). Wall-clock is printed for observability; the full-command SLA that subsumes this piece is asserted by the integration test below.
- `gather_base_graph()` issues exactly one `merge-base` per branch and one `git log` per branch plus one for main commits — no discarded default-context or default-after-base logs.
- `gather_branch()` wall-clock is printed for observability when the ambient checkout is a feature branch (skipped on main). The binding SLA + subprocess-count assertions for the image-terminal `wt list -v` data-gather path live in `gather_branch_uses_one_merge_base_and_no_short_sha` below, which runs on a controlled fixture so it always asserts regardless of the ambient checkout.

### `gather_branch_uses_one_merge_base_and_no_short_sha` (unit test, `git_graph.rs`)

Binding SLA guard for the image-terminal `wt list -v` data-gather path (graph IDs + verbose details). Runs `gather_branch(default, feature, verbose=true)` on a temporary non-main fixture so the path always executes — never skipped on a main checkout. Asserts exactly one `merge-base`, zero `rev-parse --short`, and completion under the 1-second SLA with rasterization excluded (this test never invokes `MermaidDiagram::render`).

### `perf_full_command_non_image_meets_sla` (integration test, `tests/perf_command_sla.rs`)

Spawns the built `wt` binary with image-capable env vars removed so the non-image fast path is taken, and asserts the full command (process startup, table rendering, and all git data gathering) meets the 1-second SLA on a warm cache. A warm-up primes caches, then the best of five timed runs is checked against the 1-second SLA; the best-of-5 minimum (rather than the mean) tolerates parallel-test-execution contention, and a true regression past the SLA fails this gate. Rasterization never runs on the non-image path, so it is excluded by construction.

The wall-clock and subprocess-count output is printed to stderr via `--nocapture`, providing a reproducible result trace. For a contention-free measurement, run perf tests serially via `just test-perf`.