---
date: 2026-06-08
agent: "${env.AGENT}"
status: active
reviewed_on: 2026-09-14
implemented: true
implemented_by: "claude/opus"
review_iterations: 3
---

## Problem Statement

`sniff --perf` now collects a useful request-wide set of stages and work
counters, but its human-readable report renders them as two flat bullet lists.
On a full detection the output can contain thousands of repeated stage calls
and dozens of dotted names such as:

```text
- detect.filesystem: 643.14 ms total (1 call, ...)
- filesystem.shared_walk.docs: 1669.36 ms total (4238 calls, ...)
- filesystem.file_inventory.classify.extension: 12.33 ms total (8156 calls, ...)
```

The data is present, but the flat presentation obscures the relationships
encoded by those dotted names and makes a large report hard to scan. Counters
are likewise useful but visually mixed into a long unstructured list.

## Current State

The original version of this specification described two defects. One has
already been fixed:

- `performance::with_current_collector` flushes the calling thread's buffered
  stages and counters into the installed collector before normal return.
- `WorkerCollector` and `pooled_worker` carry and flush request attribution for
  parallel walkers and Rayon workers.
- Regression coverage in `sniff/lib/tests/integration.rs` and
  `sniff/lib/src/performance.rs` verifies worker survival, request isolation,
  and pooled-worker attribution.
- A current full `--json --perf` run contains `detect.total`, all four domain
  stages, nested stages, and stable work counters. Worker data is no longer
  missing.

The remaining defect is entirely in
`sniff/cli/src/output/render.rs::render_performance_section`, which still sorts
and prints flat lists. This refreshed fix must not reopen the CRITICAL-risk
collector seam without new failing evidence.

## Outcome

Human-readable `--perf` output presents timings as a hierarchical,
unit-aligned [`MetricsTree`](../../../biscuit-terminal/lib/src/components/metrics_tree.rs)
and presents work counters in a separate count tree. The existing structured
`PerformanceReport`, collection behavior, JSON payload, and command routing
remain intact.

## Non-Goals

- Changing performance collection, worker propagation, counter names, or stage
  instrumentation.
- Making stage totals reconcile to wall-clock time. Concurrent domains,
  inclusive parent/child timings, and repeated per-item stages legitimately
  overlap.
- Changing the `PerformanceReport` serialization schema.
- Adding process-global caches or using wall-clock thresholds as optimization
  evidence.
- Instrumenting commands that do not yet expose useful substages. The renderer
  must still handle their total-only reports cleanly.

## Requirements

### R1 — Preserve the current collection contract

No production change is required in `sniff/lib/src/performance.rs` or
`sniff/lib/src/lib.rs`. Existing tests that prove worker stages and counters
survive normal completion must remain green.

If implementation uncovers a distinct collection failure, stop and revise the
scope before changing `with_current_collector`: GitNexus reports 21 direct
callers and CRITICAL upstream impact across Sniff and Claudine.

### R2 — Render timing stages as a `MetricsTree`

`render_performance_section` MUST replace the flat timing bullet list with a
`MetricsTree` rendered through `TerminalRenderable` and `Terminal::default()`.
Sniff owns the report-to-tree projection; `biscuit-terminal` owns width,
styling, connectors, unit alignment, glyph fallback, and terminal capability
degradation.

#### R2.1 — Root and hierarchy construction

1. Build a synthetic root labeled `Total` from
   `PerformanceReport::total_duration_ms`. It uses
   `MetricValue::Duration`, `MetricShare::Full`, and `.emphasized()`.
2. Do not render `detect.total` as a duplicate child of the synthetic root.
3. Parse all other stage names by `.` so new instrumentation appears without a
   maintained stage catalog.
4. Re-parent `<domain>.<rest>` below an existing `detect.<domain>` branch for
   the four detection domains `os`, `hardware`, `network`, and `filesystem`.
   This alias set is the only detection-specific mapping.
5. Reports without a `detect.total` stage, including focused command reports,
   use the same generic dotted-name parsing from the synthetic root.
6. A measured node uses its stage's `total_duration_ms`. A synthetic
   intermediate node uses the sum of its immediate children.
7. A measured stage with `calls > 1` uses `.with_calls(n)`.
8. Sort siblings by duration descending and label ascending.

The projection should be a small pure helper over `PerformanceReport`; it must
not move report interpretation into `biscuit-terminal`.

#### R2.2 — Shares and overlap note

Every non-root timing node uses its share of the report wall-clock total, not
its share of its parent. A zero-duration report uses `MetricShare::Unknown` for
children and must not divide by zero.

Repeated and concurrent stages may accumulate more duration than wall-clock,
so a node or a set of siblings may exceed its parent. Attach this italic note
with `MetricsTree::with_notes`:

> Concurrent, nested, and repeated stages may overlap; their durations do not
> sum to wall-clock time.

`MetricsTree` retains responsibility for its display rules, including
sub-one-percent shares and reserving `100%` for the root.

#### R2.3 — HOT marker

Exactly one measured, non-root timing stage carries
`MetricMarker::Highlight`: the stage with the greatest total duration. Break
equal-duration ties by stage name so selection is deterministic. Synthetic
intermediate nodes are not candidates.

If the report has no non-root measured stages, render only the `Total` root and
do not add a marker.

### R3 — Render counters as a separate count tree

When `report.counters` is non-empty, render a second `MetricsTree` below the
timing tree:

- a bold synthetic root labeled `Counters`;
- dotted counter names parsed into a generic hierarchy;
- measured leaves represented by `MetricValue::Count`;
- synthetic intermediate values equal to the sum of their immediate children;
- `MetricShare::Unknown` on every row because heterogeneous work counters do
  not have a meaningful common denominator;
- siblings ordered by count descending and label ascending; and
- no HOT marker.

Omit the counter tree when the report has no counters.

### R4 — Preserve output and JSON contracts

The leading `## Performance` heading remains for continuity. The rendered tree
continues through the existing `CliPerf` emit seams:

- rich terminal commands emit their human-readable report to stdout;
- scriptable text commands emit it to stderr so their stdout data stays clean;
- `--json --perf` keeps one valid JSON document on stdout with the structured
  `performance` field attached, and the current human-readable copy on stderr;
  and
- `--plain` strips ANSI styling but does not itself force ASCII. Connector and
  glyph fallback follows the detected terminal's Unicode capability.

Do not hand-author ANSI escapes or duplicate `MetricsTree`'s width and glyph
logic in Sniff.

## Affected Code

| File | Change |
|---|---|
| `sniff/cli/src/output/render.rs` | Add the pure timing/counter tree projection and render both trees with `MetricsTree`. |
| `sniff/cli/src/output/render.rs` tests | Cover hierarchy, ordering, markers, calls, zero totals, counters, and rendering modes. |
| `sniff/cli/tests/cli.rs` | Pin end-to-end routing and JSON/stdout validity where current coverage is insufficient. |

No dependency change is required: `sniff-cli` already depends on
`biscuit-terminal`, and `components::metrics_tree` is public.

## Testing

### Projection and rendering tests

- `detect.total` supplies no duplicate child below `Total`.
- `filesystem.shared_walk.docs` nests below `detect.filesystem` then
  `shared_walk` then `docs`.
- Non-detection dotted names form an equivalent generic hierarchy.
- Measured parents retain their measured duration; synthetic parents sum their
  immediate children.
- Siblings sort by duration/count descending and label ascending.
- The maximum measured non-root timing stage is the sole HOT node, with a
  deterministic tie case.
- Calls render for `calls > 1` and stay absent for one call.
- A zero-duration report renders without non-finite shares.
- Counters use count values and unknown shares, and an empty counter map omits
  the second tree.

### CLI contract tests

- A representative `--perf --plain` command is ANSI-free and contains the
  timing hierarchy.
- A non-Unicode terminal projection uses the component's ASCII connectors and
  marker fallback. This is a component/projection test, not an assumption about
  `--plain`.
- A scriptable text command keeps its data on stdout and performance text on
  stderr.
- `--json --perf` stdout parses as exactly one JSON value containing
  `performance`; any stderr text does not contaminate stdout.

### Verification

Run the Sniff package-area gates:

```bash
cd sniff
just test
just lint
```

The projection is platform-independent. Existing macOS, Linux, native Windows,
and WSL2 CI compile/test coverage remains required; do not introduce
platform-specific rendering code.

## Success Criteria

1. Human-readable `--perf` output is a scan-friendly timing tree rather than a
   flat stage list.
2. Counters appear in a separate count tree and never enter the timing tree.
3. One deterministic measured stage is marked HOT when stage data exists.
4. Current collection-completeness tests remain green without modifications to
   the collector seam.
5. Plain output is ANSI-free, terminal capability fallback remains owned by
   `biscuit-terminal`, and JSON stdout remains a single valid document with its
   structured performance field.
