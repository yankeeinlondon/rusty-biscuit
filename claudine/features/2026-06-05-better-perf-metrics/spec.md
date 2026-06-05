---
title: Better --perf metrics
date: 2026-06-05
status: ready for planning and implementation
reviewed: true
area: claudine
scope: claudine-cli performance reporting
---

# Better `--perf` metrics

The `--perf` flag for claudine compose / inline-compose / sequence / wrapper commands is meant to be the
first tool a developer reaches for when startup or composition feels too slow. Today it is the opposite:
its headline number disagrees with its own sections by **one to two orders of magnitude**, its itemized
rows do not add up to the totals shown next to them, and the same work is counted in two different
sections without any indication that they overlap. The numbers are not _wrong_ in isolation — each one is
a real measurement — but they are measured over **different, overlapping time windows** and then stacked
on top of each other as if they were disjoint. The result is a report you cannot reason about.

This feature re-grounds the entire report on a **single timeline** with **true wall-clock** as the only
100% reference, presents it as a **unified hierarchical tree** where every child rolls up into its parent,
and enforces — by test — that the disjoint top-level buckets sum to the headline. Imbalance (one stage
eating 99% of the time) stops being a confusion and becomes the report's single most useful signal.

## Motivating example (today's broken output)

```
▌ Performance (elapsed 78.6ms)
▌
▌ CLI Overhead
▌   pre-dispatch:                   7.3ms
▌   prep phase:                     1.50s
▌   arg parsing:                    6.1ms
▌   config loading:                 6.8ms
▌   tracing init:                   403µs
▌   environment setup:             65.7ms
▌     target resolution:               32µs
▌     header env plan:                275µs
▌     child env build:                300µs
▌     mcp composition:                  0µs
▌     argv assembly:                  360µs
▌     system prompt:                 64.7ms
▌     stream + prompt delivery:         2µs
▌   ═════════════════════════════════════
▌   TOTAL:                   1.57s
▌
▌ Composition Report
▌   …
▌   shell expansion:                970.5ms
▌   …
▌   ═══════════════════════════════════════
▌   TOTAL:                   981.6ms
▌
▌ Agent execution skipped (dry run)
```

Three independent defects are visible in this one capture:

1. **The headline measures a different window than the sections.** `(elapsed 78.6ms)` is `78.6ms`, yet the
   first section alone totals `1.57s`. That is not rounding — it is a different clock entirely.
2. **The itemized rows do not visibly add to their TOTAL.** `7.3 + 1.50s + 6.1 + 6.8 + 0.403 + 65.7` is
   nowhere near `1.57s` by eye, because three of those rows are diagnostic sub-buckets that are silently
   excluded from the sum while looking identical to the rows that _are_ summed.
3. **`prep phase` and `Composition Report` count the same work twice.** The 970ms shell-expansion cost lives
   _inside_ `prep phase` and is _also_ itemized as its own section with its own TOTAL, with nothing telling
   the reader these overlap. Summing the two section TOTALs would double-count ~1 second.

## Root-cause analysis

These are not cosmetic. They come from how the timers are wired.

### RC-1 — The headline is a post-prep sub-window, not wall-clock

`process_start` is captured at the very top of `run()` (`claudine/cli/src/main.rs:191`) and `pre_dispatch`
is `process_start.elapsed()` (`main.rs:254`). But the value printed as `(elapsed …)` comes from a **fresh**
`total_start = Instant::now()` created _inside_ `execute_composition_request_inner`
(`claudine/cli/src/commands/wrap/composition/mod.rs:790`) and read at the dry-run return
(`mod.rs:848`). By the time that timer starts, argv normalization, config load, preflight shell approval,
and the **entire darkmatter compose pass** (the 970ms) have already run. So the headline can only ever see
the small post-prep execute window. The headline and the sections are measuring two disjoint spans of the
same process.

### RC-2 — `prep phase` contains the compose pass, which is then itemized again

`prep_phase` is `compose_entry.elapsed()` (`claudine/cli/src/commands/compose.rs:328` → `:670`; inline at
`:681` → `:1077`). `compose_entry` is set at the top of `run_compose_inner`, and the window it measures
includes `composition::prepare_direct_with_schema(...)`, which runs darkmatter's compose pipeline and
produces `prepared.compose_perf` — the very `ComposePerfReport` later rendered as the "Composition Report"
section. So `prep phase` ⊇ `compose_perf.total` **by construction**. The report shows both as peers.

### RC-3 — `environment setup` and its sub-stages are measured on different clocks

`environment setup` is measured by `CommandPerfCollector`'s own timer (`env_setup_started_at` set in
`CommandPerfCollector::new`, closed by `mark_env_setup_complete`, `mod.rs:1524`). The seven sub-stage rows
(`target resolution` … `stream + prompt delivery`) are measured by a _separate_ checkpoint chain inside
`execute_composition_request_inner` (`mod.rs:804`-`814`, `:1230`). Nothing guarantees the sub-stages sum to
their parent. They are presented as a nested breakdown of `environment setup` but are not actually carved
out of the same measured window.

### RC-4 — The renderer hides which rows are structural and which are diagnostic

`perf.rs` already distinguishes summed rows (`push_sum`) from diagnostic rows (`push_diagnostic` /
`push_indented`) internally via `Row::sum_contribution`, and `TotalKind::Sum` correctly excludes the
diagnostics. But that distinction is **invisible in the output** — every row is `label … value`. A reader
has no way to know that `arg parsing` is a sub-bucket of `pre-dispatch` while `prep phase` is a true
top-level cost. The data model knows; the presentation throws it away.

## Goals

- **G-1** The headline `Performance NNN` is **true wall-clock** — process entry to report emission — and is
  the report's only 100% reference.
- **G-2** The report is a **single hierarchical tree**. Every node is a fraction of its parent; the parent
  is a fraction of wall-clock. Children never exceed their parent.
- **G-3** The disjoint top-level buckets **sum to wall-clock** within a fixed rounding tolerance, enforced
  by a reconciliation **unit test** and a **debug-build runtime assertion** (the user-selected "hard
  invariant + test").
- **G-4** No measurement appears in two places as if it were two costs. Overlapping work is expressed as
  **parent → child nesting**, never as sibling sections.
- **G-5** Each node shows its **share of wall-clock** as a percentage, and the single **dominant** leaf is
  visually flagged, so a 99%-in-one-stage profile reads as a clear pointer rather than noise.
- **G-6** Values are **column-aligned at the unit boundary** so durations are scannable.
- **G-7** Coverage is uniform across `compose`, `inline-compose`, `sequence`, and the direct provider
  wrappers — including the dry-run path that produced the example above.
- **G-8** The output contract stays intact: `--perf` remains a human-facing **stderr-only** report, and
  stdout remains reserved for composed content or provider output. The tree renderer must not introduce raw
  escape-code writes; it must keep using biscuit-terminal renderables / markup.

### G-9 — Upstream spans and a reusable renderer (in scope)

Darkmatter and biscuit-terminal are first-class dependencies of claudine, not black boxes. Where the missing
signal is _coarse instrumentation_ (darkmatter's perf is flatter than its own pipeline) or a _missing
presentation primitive_ (biscuit-terminal has no hierarchical-metrics renderable), the right fix lives
**upstream**, where it also improves those crates' own tooling (`dm` compose perf, sniff, model-citizen,
any future timing/size report). This feature explicitly **opens the door** to additive, general-purpose
changes in both crates — see [Upstream dependency changes](#upstream-dependency-changes). The guard rails:
upstream changes must be (a) **additive** (no behavior change to compose output or existing component APIs),
(b) **generally useful** beyond claudine, and (c) **phased** so claudine's report improvement does not
hard-block on every upstream landing (see [Phasing](#phasing)).

## Non-goals

- **NG-1** No change to the _semantics_ of the darkmatter compose pipeline — what it produces, the order of
  operations, caching behavior, or error handling. Upstream work is limited to **observability**: new spans,
  richer metric shapes, and surfacing already-collected timings. Composed output must be byte-identical
  whether or not the new instrumentation is read.
- **NG-2** No new sub-millisecond instrumentation _purely_ to shrink the unattributed remainder. The
  remainder is made **explicit** (see TR-3), not engineered to zero. (This does not forbid the upstream
  spans in G-9, which exist to attribute _material_ cost like per-command shell expansion.)
- **NG-3** No change to `--perf` gating / bootstrap detection (`scan_perf_bootstrap`, `PerfBootstrap`). The
  set of commands that support `--perf` is unchanged.
- **NG-4** Not a machine-readable / JSON perf export. This is the human-facing terminal report only. A
  `--perf --json` mode may be a future feature; it is out of scope here.
- **NG-5** No change to the default behavior of commands that do not pass `--perf`; instrumentation may
  collect cheap timestamps already needed for the command path, but rendering and expensive upstream
  per-directive spans remain gated by the existing `perf_enabled` flow.

## The model: one timeline, one tree

### TM-1 — A single wall-clock baseline threaded end to end

`process_start: Instant` captured in `run()` (`main.rs:191`) is the **only** zero point. It must be threaded
to the report-emission site so wall-clock can be computed there as `process_start.elapsed()`. Today
`process_start` reaches `pre_dispatch` but is dropped before the report is built; each report-emission site
instead invents its own mid-flight timer. There are **six** such sites — composition
(`mod.rs:848/1693/1900/2000`), wrapper (`wrap/mod.rs:334`), and sequence (`wrap/sequence.rs:442/757`) — every
one computing `<timer>.elapsed()` as the headline. All six invented timers are removed and replaced by the
single threaded baseline, ideally via one shared emit helper so they cannot drift back apart.

Concretely, `StartupTimings` (`perf.rs:37`) carries the **baseline instant** (or a pre-measured
`pre_dispatch` plus the `process_start` instant) down through `run_provider_wrapper` /
`run_compose_inner` / `run_inline_compose_inner` / `run_sequence` into the collector, and the collector
computes the headline at `into_report` time from that baseline — not from a fresh timer started mid-flight.

> The exact carrier (pass the `Instant` vs. pass an already-elapsed `Duration` and add later deltas) is an
> implementation detail for the plan. The **invariant** is: the headline equals `process_start.elapsed()`
> sampled at report build, and every top-level bucket is a measured sub-span of that same `[process_start,
> report_build]` interval.

### TM-2 — The node tree

The report is a tree of `PerfNode { label, self_or_total: Duration, children: Vec<PerfNode>, role }`,
rooted at the wall-clock node. `role` distinguishes presentation/aggregation behavior:

- `Structural` — a disjoint bucket that **contributes to its parent's reconciliation sum** (e.g.
  `pre-dispatch`, `prep phase`, `environment setup`, `agent`).
- `Breakdown` — a child that itemizes its parent's cost but whose siblings may **overlap or under-cover**
  the parent (e.g. darkmatter compose stages, where transclusion recursively re-enters compose). Breakdown
  children are displayed and percentaged but are **not** required to sum to the parent; the parent keeps its
  own authoritative measured total.
- `Unattributed` — a synthetic remainder node (see TR-3).

The canonical top-level structure for a compose/inline-compose/wrapper run:

```
Performance                      <wall-clock>            100%
├─ pre-dispatch                  <process_start→dispatch>  …%   (Structural)
│  ├─ arg parsing                …                              (Breakdown)
│  ├─ tracing init               …                              (Breakdown)
│  └─ config loading             …                              (Breakdown)
├─ prep phase                    <compose_entry→execute>   …%   (Structural)
│  └─ composition                <compose_perf.total>      …%   (Structural)
│     ├─ shell expansion         970.5ms             ▇ HOT      (Breakdown)
│     ├─ link resolve            5.9ms                          (Breakdown)
│     ├─ interpolation           58µs                           (Breakdown)
│     └─ …                       …                              (Breakdown)
├─ environment setup             <env window>              …%   (Structural)
│  ├─ target resolution          …                              (Structural*)
│  ├─ system prompt              64.7ms                         (Structural*)
│  └─ …                          …                              (Structural*)
├─ agent execution               <total_elapsed | — dry run>  …%
│  ├─ first response             …
│  └─ provider api duration      …
└─ unattributed                  <wall − Σ structural>     …%   (Unattributed)
```

Key consequences versus today:

- **`composition` is a child of `prep phase`, not a sibling section.** RC-2 dissolves: the 970ms appears
  exactly once, nested where it is actually spent. `prep phase` keeps its measured total; `composition`
  shows `compose_perf.total`; the compose stages are `Breakdown` leaves under it. With the upstream
  enrichment in [DM-2 / DM-3](#upstream-dependency-changes), these stages gain a phase grouping and a
  per-`::shell`-command breakdown, and `composition` can be promoted from authoritative-total + `Breakdown`
  to a true reconciling node — but Phase 1 ships correctly against today's flat metrics.
- **`arg parsing` / `tracing init` / `config loading` become `Breakdown` children of `pre-dispatch`**
  instead of free-floating peers of `prep phase`. RC-4 dissolves: nesting _is_ the structural/diagnostic
  signal.
- **The `environment setup` sub-stages must be measured on the same clock as their parent** (`Structural*`).
  This requires TR-2 below; until they reconcile, they are emitted as `Breakdown` and an `unattributed`
  child of `environment setup` absorbs the gap, rather than silently misrepresenting them as an exact carve.

### TM-3 — Sequence aggregation

`sequence` aggregates N steps. The tree gains a `steps` Structural node whose children are per-step subtrees
(reusing `SequencePerfAccumulator`, `perf.rs:122`). The merge of `compose_perf` across steps
(`into_report`, `perf.rs:177`-) is preserved, but the merged composition becomes a `Breakdown` subtree under
the aggregate, and the headline remains the single wall-clock of the whole sequence run. Per-step wall-clocks
are Structural children that reconcile to the sequence wall-clock (plus the sequence-level `unattributed`
remainder).

Design decision from review: the sequence headline is the wall-clock of the entire sequence invocation,
including inter-step orchestration, shared shell-approval-cache work, and fail-fast handling. If
`SequencePerfAccumulator` can only sum per-step windows during Phase 1, orchestration time must appear as a
named `sequence orchestration` Structural child when it is measured directly, or as sequence-level
`unattributed` when it is not yet measured. Do not redefine the headline as "sum of completed steps"; that
would repeat RC-1 at sequence scale.

## Reconciliation invariant (hard)

### TR-1 — Definition

For every node with `role == Structural` children, the following must hold at report-build time:

```
node.total  ==  Σ(child.total for Structural children)  +  unattributed_child.total
```

with the headline as the special case `wall_clock == Σ(top-level Structural) + unattributed`. `Breakdown`
children are excluded from this sum (they may overlap or under-cover by design).

### TR-2 — Same-clock measurement for `environment setup` sub-stages

To let the `environment setup` sub-stages be `Structural` (carve the parent exactly rather than estimate),
their checkpoint chain (`mod.rs:804`-`814`, `:1230`) and the parent `environment setup` window
(`env_setup_started_at` … `mark_env_setup_complete`) must be derived from **one** timer. The plan must either
(a) make `mark_env_setup_complete` the close of the same checkpoint chain that records the sub-stages, or
(b) keep them on separate clocks and demote the sub-stages to `Breakdown` with an explicit `unattributed`
child. (a) is preferred; (b) is the acceptable fallback. Either way the parent total stays authoritative.

### TR-3 — Explicit `unattributed` remainder

Rather than instrument every nanosecond, each reconciling node carries a synthetic `unattributed` child:

```
unattributed.total = max(ZERO, node.total − Σ Structural children)
```

This makes TR-1 hold **exactly** by construction and surfaces, honestly, how much of a window is not yet
broken down. Rendering rule: omit the `unattributed` row when it is below a `1ms` _and_ below `1%`
threshold (it is noise); always show it when it is material. A large `unattributed` is itself a useful
signal ("most of prep is uninstrumented") rather than a hidden discrepancy.

### TR-4 — Enforcement

- **Unit test (required):** a `perf.rs` test builds a representative `CommandPerfReport`, then walks the
  tree asserting TR-1 at every reconciling node within a tolerance of **the larger of 1ms or the number of
  summed children × the duration-formatting granularity** (formatting rounds each row; the tolerance must
  absorb that without masking real drift). The test must include the dry-run shape that produced the
  motivating example, asserting the headline equals the sum of top-level Structural buckets + remainder —
  i.e. the exact bug class (`78.6ms` headline vs `1.57s` body) cannot recur.
- **Debug runtime assertion (required):** in `debug_assertions` builds, `into_report` (or the renderer
  entry) `debug_assert!`s TR-1 on the assembled tree. Release builds skip it (zero cost, no panic risk in
  the field).

## Presentation

The renderer continues to go through biscuit-terminal `Prose` / `BlockQuote` (`perf.rs:547`-) — **no raw
escape codes**, per repo convention. The existing yellow `▌ ` block-quote frame and `<b>`/`<dim>`/`<i>`
markup styling are retained. Changes are to layout and content.

### P-1 — Tree glyphs

Render the hierarchy with box-drawing connectors (`├─`, `│`, `└─`) computed from tree depth, replacing the
current fixed `indent: 2 / 4` scheme. Depth is unbounded in principle (compose nests under prep nests under
wall-clock); the renderer walks the tree generically rather than hard-coding two levels.

### P-2 — Share-of-wall-clock column

Every node renders a third column: its percentage of **wall-clock** (not of its parent — wall-clock keeps
all percentages on one comparable scale). Format: integer percent for ≥1% (`97%`), `<1%` for anything below
1% (avoid a column full of `0%`). The root shows `100%`.

### P-3 — Dominant-leaf highlight

The single leaf node with the largest `total` across the **entire** tree is flagged `HOT` (or an
equivalent single marker — a small bar `▇` and/or `<b>` emphasis chosen to read well in the yellow frame).
Exactly one leaf is flagged per report. This is what turns "99% in shell expansion" from a confusing
imbalance into the headline finding. If the dominant leaf is below a materiality floor (e.g. wall-clock is
tiny and no leaf exceeds ~20%), suppress the marker — there is no hot spot worth pointing at.

### P-4 — Alignment

Durations align at the **unit boundary** so a column of `32µs / 275µs / 64.7ms / 1.50s` lines up by
magnitude rather than ragged right-edge. Concretely: right-align the numeric mantissa and left-align the
unit suffix in a fixed unit column, or pad to a common decimal layout. The percentage column is
right-aligned in its own fixed width. Column widths are computed once for the whole tree (not per-section as
today, `render_section` `perf.rs:498`-), so all rows share one grid.

### P-5 — Dry-run and partial states

- Dry run: the `agent execution` node renders as a single `—` leaf labeled dry run (today's
  `"Agent execution skipped (dry run)"` note becomes the node's value). The wall-clock headline still
  reflects real elapsed time through report emission — crucially it is **no longer** the tiny post-prep
  window; for the motivating example it would read ≈ the real `~1.6s`, matching the body.
- Partial sequence (`set_partial`, `perf.rs:171`): retained as a node annotation / note; reconciliation
  still holds over whatever steps ran, with the remainder absorbing interrupted work.

### P-5a — Prep-phase named children

Review gap resolved: `prep phase` must not become a catch-all parent with a permanently large remainder if
the implementation already has material prep work outside darkmatter composition. During planning, audit the
window from `compose_entry` to `execute_composition_request_inner`. Any material, non-overlapping work in
that window must become a named Structural child under `prep phase`; likely candidates include schema
validation, shell-approval prompting, dry-run metadata preparation, and file/frontmatter loading. Work that
is small or not yet cleanly separable may remain in `prep phase → unattributed`, but the plan should call
out any remainder expected to exceed the TR-3 display threshold.

### P-6 — `fmt_duration` is reused, not reinvented

`fmt_duration` (`perf.rs:376`) stays the single duration formatter. P-4 alignment is a layout concern around
its output, not a new number format. (`µs` / `ms` / `s` thresholds unchanged.)

## Upstream dependency changes

The investigation found that the report's coarseness is, in part, **upstream coarseness** — darkmatter
collects more than its perf report exposes, and biscuit-terminal lacks the one component this report needs.
Fixing these upstream is strictly additive and pays off in those crates' own tooling. None of these are
required for Phase 1; they are what makes Phases 2–3 (see [Phasing](#phasing)) land cleanly.

### Darkmatter (`darkmatter/lib/src/markdown/compose`)

Today `ComposePerfReport { total, metrics: Vec<ComposePerfMetric { stage, elapsed, calls }> }`
(`compose/types.rs:1576`-) is **flat**: 17 aggregate `ComposeStage`s, no nesting, no per-command detail —
even though the pipeline itself is richly structured (`ComposeOperation`, `ComposePhase` =
`InlinePre`/`Transclusion`/`InlinePost`/`Finalization`, `compose/types.rs:156`-) and already tracks data the
report drops on the floor.

- **DM-1 — Surface `calls` (claudine-side, zero upstream change).** `ComposePerfMetric.calls` already exists
  and claudine's renderer ignores it. A stage that ran 40 times for 5ms reads very differently from one that
  ran once for 5ms. Show `calls` on `Breakdown` rows where `> 1`. _(No darkmatter change — listed here so it
  isn't missed.)_
- **DM-2 — Per-`ComposePhase` grouping.** Add a phase tag to each metric (or group metrics by
  `ComposeOperation::phase()`) so the 17 stages nest under their four phases. This directly feeds the tree
  model: `composition → {InlinePre, Transclusion, InlinePost, Finalization} → stages`. Benefits darkmatter's
  own `dm` perf output identically. Additive: the flat `metrics` vec can remain, with phase as an added
  field or a parallel grouped view.
- **DM-3 — Per-command shell-expansion spans.** This is the single highest-value span. When `ShellExpansion`
  is 970ms / 99%, the report cannot say **which** `::shell` directive caused it. Emit a per-directive span
  carrying `{ command_display, command_hash, elapsed, cached: bool, exit_status }`. `command_display` is the
  redacted and elided rendering chosen by [OQ-2](#open-questions); `command_hash` uses the repo-standard
  non-crypto xxHash path from biscuit-hash for stable local correlation without exposing the full command.
  Turns
  `shell expansion 970.5ms` into `shell · $(curl …) 960ms ▇ HOT` — actionable instead of merely alarming.
  `ComposeReport` already counts `shell_expansions_applied` / `shell_approvals_used` (`types.rs:1701`-),
  so the directive set is known; this adds timing per directive. Hugely useful to anyone debugging a slow
  `dm compose`, not just claudine.
- **DM-4 — Surface `ComposeContext::capture_timings`.** `ComposeContext` already records per-group
  capture timings — git, repo, OS, hardware via sniff — as `Vec<(String, Duration)>`
  (`compose/types.rs:1505`, `1312`). This context-capture cost (potentially tens of ms of `sniff` work)
  is **completely invisible** in today's perf report. Thread it into `ComposePerfReport` (or expose it on
  the report claudine already holds) so it becomes a `Breakdown` subtree under composition. This is a real
  hidden bucket, valuable to darkmatter's own profiling.
- **DM-5 — Optional: nested sub-reports for recursive transclusion.** Recursive `::file`/`::url`
  transclusion re-enters compose; `ComposePerfReport::merge` (`types.rs:1656`) flattens child runs into the
  parent's aggregate, which is exactly why stages can't be made to reconcile to `total`. Optionally retain
  child sub-reports as a nested structure so a `composition` node can become a true reconciling node (TR-1)
  and so per-included-document cost is visible. Lower priority than DM-3; flagged because it removes the last
  reason `composition` is `Breakdown`-only.

All of the above keep composed **output** byte-identical (NG-1) and are gated by the existing
`perf_enabled` flag, so they cost nothing when `--perf` is off.

### biscuit-terminal (`biscuit-terminal/lib/src/components`)

biscuit-terminal today has `table/`, `two_column.rs`, `list.rs`, `progress.rs`, `section.rs` — but **no
hierarchical-metrics / tree renderable** and no aligned-numeric-with-share-bar primitive. The renderer this
report needs (box-drawing tree connectors + unit-boundary-aligned duration column + share-of-total percent +
single dominant-leaf highlight) is **generic** and wanted by other crates the moment it exists.

- **BT-1 — A `MetricsTree` (working name) renderable.** A `TerminalRenderable` that takes a tree of
  `{ label, value: Duration | bytes | count, share: f32, marker: Option<Marker>, children }` and renders it
  with depth-derived connectors (`├─ │ └─`), a value column aligned at the unit boundary, a right-aligned
  share column, and an optional single highlight marker. Capability-aware (degrades connectors/markers under
  `NO_COLOR` / ASCII-only) via the existing `Terminal` plumbing, consistent with `Prose`/`BlockQuote`.
- **Who else benefits:** darkmatter's `dm compose --perf` and document-size breakdowns, `sniff`'s
  per-group detection timings, `model-citizen`'s scan timings, and any future "where did the time/bytes go"
  report. This is precisely the kind of cross-cutting primitive that belongs in biscuit-terminal rather than
  re-implemented per CLI.
- **Promotion path.** If the component API is clear enough up front, build it in biscuit-terminal from the
  start. If not, **prototype the tree renderer inside `claudine/cli/src/perf.rs` in Phase 1, then promote**
  the stabilized API to biscuit-terminal in Phase 3 and have claudine consume it. Either way the unit-aligned
  formatting reuses claudine's `fmt_duration` semantics (P-6) until/unless biscuit-terminal grows its own
  duration formatter.

## Phasing

The phases are independently shippable; each leaves `--perf` better than before. Phase 1 alone fixes every
defect in the [motivating example](#motivating-example-todays-broken-output).

- **Phase 1 — claudine-only (no upstream dependency).** True wall-clock headline (TM-1), the `PerfNode` tree
  (TM-2), reconciliation invariant + remainder + tests (TR-1…TR-4), tree rendering / percent / HOT /
  alignment (P-1…P-6) built inline in `perf.rs`, DM-1 (`calls`). Compose stages render as `Breakdown` under
  `composition`. **This is the minimum viable feature** and resolves RC-1…RC-4.
- **Phase 2 — darkmatter enrichment.** DM-2 (phase grouping), DM-3 (per-command shell spans — highest
  value), DM-4 (capture timings), DM-5 (optional nested transclusion sub-reports). Each lands in darkmatter
  with its own tests, then claudine consumes it: phases/commands/capture become nested `Breakdown` (or
  reconciling, post-DM-5) subtrees. No claudine-side schema churn beyond reading richer data.
- **Phase 3 — biscuit-terminal promotion.** Extract the Phase-1 inline tree renderer into the BT-1
  `MetricsTree` component; claudine consumes it; darkmatter/sniff/model-citizen become candidate consumers.

## Affected code (orienting map, not a change list)

| Area | File / anchor | Nature of change |
| --- | --- | --- |
| Baseline threading | `cli/src/main.rs:191,254,263-273,301-311` | Carry `process_start` baseline into the collector instead of dropping it |
| Headline source (6 emit sites) | composition `mod.rs:790,848,1693,1900,2000`; wrapper `wrap/mod.rs:313,334`; sequence `wrap/sequence.rs:442,757` | Remove the per-site mid-flight timers (`total_start`/`wrapper_start`/`sequence_start`); headline = threaded `process_start` baseline elapsed at build. **Centralize** so the six sites cannot drift |
| Prep window | `cli/src/commands/compose.rs:328,670,681,1077` | Keep measuring prep; expose compose as a **child** of prep, not a peer |
| Env sub-stage clock | `mod.rs:804-814,1230,1524` | TR-2: one timer for `environment setup` + its sub-stages |
| Report model | `cli/src/perf.rs:20-117,277-373` | Replace flat `CliOverheadReport` + separate sections with the `PerfNode` tree |
| Renderer | `cli/src/perf.rs:388-622` | Tree walk, glyphs (P-1), percent column (P-2), HOT (P-3), unified alignment (P-4) |
| Sequence merge | `cli/src/perf.rs:122-270` | Produce a `steps` Structural subtree feeding the same tree model |
| Tests | `cli/src/perf.rs` `#[cfg(test)]`; existing snapshot `render_perf_report_snapshot_locks_totals_and_alignment` | Add TR-4 reconciliation test; update snapshot to tree layout |
| _Phase 2_ — compose perf shape | `darkmatter/lib/src/markdown/compose/{types.rs,perf.rs}` | DM-2 phase tag, DM-3 per-command shell spans, DM-4 expose `capture_timings`, DM-5 optional nested sub-reports |
| _Phase 2_ — context timings | `darkmatter/.../compose/types.rs:1505,1312` | Thread `ComposeContext::capture_timings` into `ComposePerfReport` |
| _Phase 3_ — tree component | `biscuit-terminal/lib/src/components/` (new `metrics_tree.rs` + `mod.rs`) | BT-1 `MetricsTree` renderable; claudine consumes it, drops inline renderer |

The existing snapshot test
(`render_perf_report_snapshot_locks_totals_and_alignment`, `perf.rs:1158`) already locks several correct
behaviors (microsecond rows show their value; long labels keep a gutter; CLI TOTAL excludes overlapping
rows; Composition TOTAL mirrors `compose.total`). Those guarantees must be **preserved** under the new tree
layout — the snapshot is rewritten to the tree shape, not deleted, and its assertions re-expressed against
the nested structure.

## Success criteria

A run of `claudine compose <file> --perf --dry-run` (and the non-dry, inline, sequence, and wrapper variants)
produces a report where:

1. The headline `Performance NNN` equals, within rounding, the sum of the top-level Structural buckets shown
   beneath it. The `78.6ms`-vs-`1.57s` class of contradiction is impossible (enforced by TR-4).
2. No duration appears as two sibling costs. The compose / shell-expansion time appears once, nested under
   `prep phase`.
3. The reader can see, at a glance, that one leaf (`shell expansion`) is the dominant cost, via the percent
   column and the HOT marker — turning the imbalance into the report's primary signal.
4. Every value column lines up at the unit boundary; the tree connectors make parent/child relationships
   unambiguous.
5. The claudine CLI tests pass, including the new reconciliation test and the rewritten snapshot. At minimum
   this means the area-local test recipe or `cargo test -p claudine-cli perf --color=never`; broader
   `just test` coverage is expected when the implementation touches shared darkmatter or biscuit-terminal
   code. Debug builds carry the TR-1 `debug_assert!`.
6. _(Phase 2)_ A slow run names the **specific** `::shell` directive responsible (DM-3), not just a
   `shell expansion` aggregate, and previously-invisible `ctx.*` capture cost (DM-4) appears as its own
   subtree. Darkmatter's own `dm compose --perf` benefits from the same spans, with darkmatter tests added
   alongside.
7. _(Phase 3)_ The hierarchical tree is rendered by a reusable biscuit-terminal `MetricsTree` component
   (BT-1) that claudine consumes; the inline Phase-1 renderer is removed. biscuit-terminal carries the
   component's own tests.

## Open questions

- **OQ-1 — Should recursive transclusion keep nested sub-reports?**

  Today `ComposePerfReport::merge` (`types.rs:1656`) flattens recursive-transclusion child runs into the
  parent aggregate, so stage rows can overlap and `composition` cannot reconcile to `total`. Phase 2 must
  choose one of these designs:

  - **Option A: keep the flat aggregate indefinitely.** Pros: smallest darkmatter API change; low migration
    risk; enough for claudine Phase 1 because `composition` can remain authoritative-total + `Breakdown`.
    Cons: no per-included-document visibility; composition can never become a fully reconciling subtree;
    repeated shell stages remain harder to localize.
  - **Option B: add optional nested child reports while preserving the flat aggregate.** Pros: additive;
    keeps current consumers working; enables per-document drill-down and future reconciliation; lets `dm`
    and claudine render both summary and detail. Cons: larger report shape; more tests; recursive displays
    need depth and elision limits.
  - **Option C: replace the flat aggregate with only nested reports.** Pros: cleanest conceptual model.
    Cons: breaking API change and unnecessary churn for current consumers.

  **Recommendation:** Option B. It achieves the design goal without breaking darkmatter's existing flat
  `metrics` consumers, and it lets claudine promote `composition` to a true reconciling subtree later.

- **OQ-2 — What is the command-display policy for per-shell spans?**

  Shell command strings can contain secrets, file paths, URLs, or very long inline scripts. Phase 2 must
  choose a display policy before DM-3 surfaces per-command spans:

  - **Option A: render raw command text with a length cap.** Pros: most immediately actionable. Cons:
    unacceptable secret-leak risk; inconsistent with Claudine's webhook/error redaction posture.
  - **Option B: render redacted, whitespace-normalized, length-capped text plus a stable non-crypto hash.**
    Pros: actionable enough for humans; secrets can be masked; long scripts stay readable; the hash lets a
    developer correlate repeated commands locally without exposing the full text. Cons: requires a shared
    redaction helper and tests for common token/URL patterns.
  - **Option C: render only a hash and timing.** Pros: safest. Cons: not actionable from the report alone;
    forces users to cross-reference source files manually.

  **Recommendation:** Option B. It matches the repo's existing redaction posture while preserving the core
  value of DM-3. Use biscuit-hash xxHash for the display hash unless darkmatter already has a local hashing
  helper available in that layer.

- **OQ-3 — Where should context-capture timings attach?**

  DM-4 exposes `ComposeContext::capture_timings`, but the tree must attach them to the real timeline:

  - **Option A: always attach under `composition`.** Pros: simple; mirrors where darkmatter owns the data.
    Cons: wrong if the capture happens before the measured compose total starts.
  - **Option B: always attach under `prep phase` as `context capture`.** Pros: timeline-safe if capture is
    setup work. Cons: splits darkmatter-owned detail away from the rest of the compose subtree when capture
    is actually part of `compose_perf.total`.
  - **Option C: attach based on the measured window.** If capture is inside `compose_perf.total`, render it
    under `composition`; otherwise render it as a `prep phase` Structural child.

  **Recommendation:** Option C. The tree's main contract is timeline truth, so ownership should not override
  where the time was actually spent.

- **OQ-4 — How generic should biscuit-terminal `MetricsTree` be in Phase 3?**

  - **Option A: `Duration`-only.** Pros: smallest API; directly matches claudine's immediate need. Cons:
    likely forces another API revision for byte/count reports.
  - **Option B: generic value enum from the start (`Duration`, `Bytes`, `Count`).** Pros: reusable by
    darkmatter size reports, sniff timings/counts, and model-citizen scans. Cons: larger initial API and
    more formatting policy to stabilize.
  - **Option C: generic over a caller-provided formatted value plus optional numeric share.** Pros: maximum
    flexibility. Cons: weaker consistency; callers can bypass unit-boundary alignment semantics.

  **Recommendation:** Option B, but keep Phase 1's claudine-local renderer `Duration`-only. By Phase 3 the
  component is explicitly a shared primitive, and the broader value enum prevents a near-term follow-up
  breaking change.
