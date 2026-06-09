---
date: 2026-06-08
agent: "${env.AGENT}"
---

## Problem Statement

The `sniff --perf` flag is supposed to decompose a command into the individual
computations that produced the answer, so a developer can see *which part* was
slow. In practice it reports a single useless line:

```text
## Performance

Total: 1256.49 ms

Stages:
- detect.total: 1256.49 ms total (1 call, max 1256.49 ms, last 1256.49 ms)
```

There is no decomposition. The flag is effectively a wall-clock stopwatch with
extra ceremony.

## Root Cause

There are **two independent defects**, both of which must be fixed.

### 1. Worker-thread metrics are silently dropped (data loss)

`detect_with_plan` (`sniff/lib/src/lib.rs`) runs the four domains in scoped
threads and records a stage per domain inside each worker:

- `detect.os`, `detect.hardware`, `detect.network`, `detect.filesystem`
- nested stages recorded deeper in each domain, e.g. `os.identity`,
  `os.locale`, `os.time`, `os.path_dirs`, `filesystem.repo`,
  `filesystem.shared_walk`, `filesystem.shared_walk.docs`, `hardware.core`

Stage recording writes to a **thread-local** `STAGE_BUFFER`
(`sniff/lib/src/performance.rs`). Buffers are only drained into the shared
`PerformanceCollector` by `merge_thread_local_buffers()`, which merges **the
calling thread's** buffer. The collector exposes `flush_thread_local()` for
exactly this purpose — but **the scoped worker threads never call it before they
exit.** When a worker thread terminates, its thread-local buffer is dropped,
unmerged.

Only `detect.total` is recorded on the **main** thread (inside the outer
`with_current_collector` closure), so it is the only stage `snapshot()` ever
sees. Every domain stage and every nested stage is lost. This is why the report
contains nothing but `detect.total`.

### 2. The report is rendered flat, not hierarchically

`render_performance_section` (`sniff/cli/src/output/render.rs`) emits a flat,
duration-sorted bullet list. Even if all stages survived, the parent/child
structure encoded in the dotted stage names would not be visible.

## Goals

1. Preserve every stage and counter recorded under an active collector,
   regardless of which thread recorded it.
2. Render the surviving timing data as a hierarchical, unit-aligned
   [`MetricsTree`](../../../biscuit-terminal/lib/src/components/metrics_tree.rs)
   that decomposes total time into its constituent stages.
3. Render counters as a separate `Count` block.

## Non-Goals (Out of Scope)

- **Non-detect command paths.** Commands that bypass the detection pipeline
  (`repo *`, `programs`, `just`, `docs`, …) call `perf.emit_*(None)` and emit
  only total wall-clock time. They are unchanged by this fix; most have little
  to instrument. A later effort may instrument them.
- **JSON output shape.** The `PerformanceReport` JSON serialization
  (`--json --perf`) is unchanged. Only the human-readable text rendering and the
  underlying data completeness change. (The richer stage set will naturally
  appear in the JSON too, because it is the same `PerformanceReport`; no schema
  change.)
- **New instrumentation.** No new `record_stage` call sites are added. This fix
  makes the *existing* instrumentation visible and correctly aggregated.

---

## Requirements

### R1 — No recorded metric may be lost when its recording thread exits

Any stage or counter recorded while a `PerformanceCollector` is the current
collector for a thread MUST be merged into that collector before the thread
exits.

**Required behavior**, with implementation latitude left to the plan. The
preferred mechanism is to make `with_current_collector`
(`sniff/lib/src/performance.rs`) flush the calling thread's buffers into the
collector being removed, on closure exit, before restoring the previous
collector:

- Drain is idempotent (`merge_thread_local_buffers` uses `buf.drain()`), so the
  subsequent `collector.snapshot()` merge on the main thread sees an empty
  buffer and does not double-count `detect.total`.
- The fix is general: it closes this class of bug for **any** future scoped /
  Rayon worker that records under a collector, not just the four domains in
  `detect_with_plan`.

A per-worker explicit `collector.flush_thread_local()` call inside each scoped
thread is an acceptable alternative, but the centralized fix is preferred for
durability.

**Ordering note.** `std::thread::scope` joins all workers before the outer
`with_current_collector` closure returns, so by snapshot time every worker has
already flushed. No additional synchronization is required.

### R2 — Render timing as a hierarchical MetricsTree

`render_performance_section` MUST build a `MetricNode` tree from the report's
`stages` and render it with `MetricsTree`, replacing the flat bullet list.

#### R2.1 — Hierarchy construction (auto-parse dotted names)

The tree is derived from the dotted stage names, not from a hand-maintained
mapping, so newly instrumented stages appear automatically.

Construction rules:

1. **Root.** `detect.total` is the synthetic root, labeled `Total`, rendered
   bold (`.emphasized()`) with `MetricShare::Full`.
2. **Domain re-parenting (the one curated piece).** The domain prefixes
   `os`, `hardware`, `network`, `filesystem` are aliased under their
   `detect.<domain>` branch. Concretely:
   - `detect.os` / `detect.hardware` / `detect.network` / `detect.filesystem`
     are the first-level branches under the root.
   - A stage named `<domain>.<rest>` (e.g. `filesystem.shared_walk.docs`,
     `os.identity`) is re-parented under `detect.<domain>` with the leading
     `<domain>.` stripped for display (→ `shared_walk.docs`, `identity`).

   This four-entry alias map is the only domain-specific knowledge; the domain
   set is stable.
3. **Remaining segments nest by `.`.** After re-parenting, each remaining stage
   name is split on `.` and nested. Intermediate path segments that have no
   stage of their own become synthetic branch nodes.
4. **Node value.**
   - A node backed by a measured stage uses that stage's `total_duration_ms`
     as `MetricValue::Duration`.
   - A synthetic intermediate node (no own measurement) uses the **sum of its
     children's durations**.
5. **Calls.** A node backed by a stage with `calls > 1` sets `.with_calls(n)`
   so the tree surfaces `×N` (e.g. repeated per-file classification stages).
6. **Ordering.** Siblings are sorted by duration descending, then by label
   ascending (matching the current flat-list tiebreak).

Target rendering (illustrative — connectors, alignment, and the `▇ HOT` marker
are owned by `MetricsTree`):

```text
Total                1256.49ms  100%
├─ detect.filesystem  640.3ms   51%
│  └─ shared_walk     590.1ms   47%
│     └─ docs          12.4ms    1%
├─ detect.os          412.0ms   33%
│  ├─ path            301.2ms   24%  ▇ HOT
│  └─ identity         88.1ms    7%
└─ detect.hardware    180.0ms   14%
```

#### R2.2 — Share semantics

Every node's `MetricShare::Of(fraction)` is computed as
`node_duration / detect.total_duration` (the grand total), **not** relative to
its parent. The root is `MetricShare::Full`.

Because the four domains execute concurrently in scoped threads, sibling
durations can sum to **more** than their parent's wall-clock time. To prevent
this from reading as a bug, attach an italic trailing note via
`MetricsTree::with_notes`:

> *Domains run concurrently; sibling shares may exceed their parent's wall-clock
> time.*

`MetricsTree`'s existing share rendering handles the edge cases: sub-1% slivers
render `<1%`, measured shares cap at `99%`, and `100%` is reserved for the root.

#### R2.3 — HOT marker

Exactly one node carries `MetricMarker::Highlight`: the non-root node with the
greatest `total_duration_ms`. (`MetricsTree` enforces a single marker visually;
the builder must set it on a single node.)

### R3 — Render counters as a separate Count block

Counters (`cache_hits`, `cache_misses`, `files_scanned`, `files_classified`,
…) are not durations and MUST NOT appear in the timing tree.

When `report.counters` is non-empty, render a **second** `MetricsTree` below the
timing tree:

- Root labeled `Counters`, `.emphasized()`.
- Counter names auto-parse by `.` into a tree the same way stages do
  (e.g. `network.wan_ip.cache_hits` → `network` › `wan_ip` › `cache_hits`).
- Each node uses `MetricValue::Count`. A synthetic intermediate node's count is
  the sum of its children.
- `MetricShare::Unknown` for every counter row (percentages across
  heterogeneous counters are meaningless; the share column renders an em dash).
- No HOT marker.

If `report.counters` is empty, the counters block is omitted entirely.

### R4 — Output plumbing unchanged

`render_performance_section` continues to return a `String` and is still emitted
through the existing `emit_text` / `emit_stderr` / `emit_for_json` seam
(`sniff/cli/src/perf.rs`). Render with the established sniff CLI pattern:

```rust
MetricsTree::new(root).render(&Terminal::default())
```

`Terminal::default()` supplies real detected width and capabilities. The
existing `plain` handling in `emit_text` / `emit_stderr`
(`strip_escape_codes`) continues to strip ANSI for `--plain`, and `MetricsTree`
already folds Unicode connectors/glyphs to ASCII on non-Unicode terminals.
Stdout-vs-stderr routing (rich → stdout, scriptable → stderr) is unchanged.

The leading `\n## Performance\n` header is preserved for continuity with
existing output and snapshot expectations.

---

## Affected Code

| File | Change |
|------|--------|
| `sniff/lib/src/performance.rs` | R1: flush the calling thread's buffers into the collector on `with_current_collector` exit (preferred), or document the per-worker flush requirement. |
| `sniff/lib/src/lib.rs` | If per-worker flush is chosen instead of R1's centralized fix, each scoped thread calls `collector.flush_thread_local()` before returning. No change if R1 centralizes. |
| `sniff/cli/src/output/render.rs` | R2/R3: replace the flat bullet rendering in `render_performance_section` with `MetricsTree` builders for timing and counters. |

No dependency change is required: `biscuit-terminal` is already a path
dependency of `sniff/cli` and `components::metrics_tree` is a public module.

---

## Testing

### Library (`sniff/lib`)

- **Worker-thread metrics survive (regression test for the real bug).** With a
  collector installed, spawn a scoped/std thread that records a stage, let it
  exit, then `snapshot()` on the main thread. Assert the worker's stage is
  present. This test FAILS on the current code and PASSES after R1.
- **No double-count.** A stage recorded on the main thread (e.g. `detect.total`)
  appears with `calls == 1` after both the on-exit flush and the snapshot merge.
- **End-to-end `detect_with_plan(... .performance(true))`** on the repo returns
  a report whose `stages` contains `detect.os`, `detect.hardware`,
  `detect.filesystem`, and at least one nested domain stage — not just
  `detect.total`.

### CLI (`sniff/cli`)

- **Hierarchy builder unit tests** (pure function over a synthetic
  `PerformanceReport`):
  - `detect.total` becomes the bold root with `100%`.
  - `filesystem.shared_walk.docs` nests under `detect.filesystem` › `shared_walk`
    › `docs`.
  - `os.identity` nests under `detect.os` › `identity`.
  - Siblings are duration-sorted descending.
  - Exactly one node carries the HOT marker, and it is the max-duration
    non-root node.
  - A stage with `calls > 1` renders `×N`.
- **Counters block**: present only when counters are non-empty; counter rows
  render em-dash shares; empty counters omit the block.
- **Plain mode**: `--perf --plain` output contains no ANSI escape sequences and
  no Unicode box-drawing/marker glyphs (ASCII connectors `+-`, `# HOT`).

### Manual verification

```bash
sniff --perf                     # rich tree on stdout
sniff repo git-status --perf     # tree on stderr (scriptable), clean stdout
sniff --perf --plain             # ASCII-folded, no ANSI
sniff --json --perf              # JSON on stdout, perf text on stderr
```

---

## Success Criteria

1. `sniff --perf` shows a decomposed tree with the four domains and their nested
   stages — not a single `detect.total` line.
2. The slowest stage is flagged with the `▇ HOT` marker.
3. Removing the R1 flush fix makes the library regression test fail (the fix is
   load-bearing, not cosmetic).
4. `--plain` output is ANSI-free and ASCII-folded; `--json` stdout stays
   machine-parseable with perf on stderr.
5. Counters render in their own block (or are omitted when empty) and never
   pollute the timing tree.
