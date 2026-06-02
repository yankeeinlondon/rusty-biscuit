---
status: ready for planning and implementation
---

# Perf Gate: Benchmark Suite and Cutover Acceptance Criterion

## Status

**Architecture approved.** This spec defines the performance gate for the tree
cutover — both the **pass/fail criterion** and the **benchmark suite** that
instruments it. It resolves Decision #3 (and subsumes Decision #9) of
[`../2026-06-02-tree-cutover/spec.md`](../2026-06-02-tree-cutover/spec.md) and
supplies the concrete metric behind that spec's Acceptance Criteria #4
("overall performance trend toward faster").

The benchmark suite is permanent infrastructure: it outlives the cutover and
remains the regression-detection harness for the tree renderers.

Decision lineage:

| Question | Decision | Notes |
|---|---|---|
| Gate shape | **Two-part: bespoke comparison + baseline trend.** | Covers both "tree beats bespoke" and "tree doesn't drift," and works even where the bespoke arm is already gone. |
| Bespoke-compare threshold | **Per-target geomean ≤ 1.0×; hard per-fixture ceiling 1.5×.** | Geomean enforces the net-faster trend; the ceiling stops any single path from regressing badly under cover of a good average. |
| Baseline-trend threshold | **No bench regresses >10% vs its saved baseline.** | A ±noise band below that absorbs run-to-run jitter. |
| DarkmatterPage browser | **Build a minimal browser path, then bench it.** | Doubles as a cutover holdout advance (DarkmatterPage gains a browser render). |
| Bench organization | **Extend existing files; keep `migration_parity` as the comparison arm.** | Avoids duplicating the bespoke-vs-tree suite. |

## Background

The cutover's Acceptance Criteria #4 requires a "net performance trend toward
faster, mild localized regressions acceptable." That is a direction, not a
test. This spec makes it a test.

### Existing benchmark landscape

| Bench | Crate | Role today |
|---|---|---|
| `migration_parity.rs` | darkmatter | **Bespoke-vs-tree** comparison across a fixture corpus (legacy `as_html`/`for_terminal` vs fold + tree renderers). The live bespoke comparison. |
| `compose_pipeline.rs` | darkmatter | Per-stage compose isolation via `ComposeOptions::only` + `full_pipeline`. |
| `render_pipeline.rs` | darkmatter | Mode-level (`parse` / `terminal` / `html` / `highlight`) — **not** step-broken. |
| `render_tree.rs` | biscuit-terminal | Tree-renderer stress trees + a 6-component group + a now-degenerate `component_render_path_comparison` (both arms route through the tree). |
| `rendering.rs` | biscuit-terminal | Prose, word-wrap, layout, a partial component set. |

The gate reuses `migration_parity` as its comparison arm and fills the gaps
(step breakdown, full component coverage, the requested darkmatter component
benches) rather than duplicating.

## The Gate

The gate has two parts; **both** must pass.

### Part 1 — Bespoke comparison (where a bespoke arm survives)

Applies to paths that still have a legacy/bespoke renderer to compare against —
today, the darkmatter render pipeline measured by `migration_parity`.

- **Per-target geomean of tree ÷ bespoke ≤ 1.0** across the fixture corpus
  (terminal and browser computed separately).
- **Hard per-fixture ceiling: no fixture may exceed 1.5× bespoke** on any
  target. A fixture over the ceiling is either fixed before cutover or carries
  a **documented, signed-off exception** in `baselines.md` (fixture, ratio,
  reason, and the follow-up that will close it).

This subsumes cutover **Decision #9**: the browser `large_table` fixture is
≈ 11× bespoke today, so it fails the ceiling and must be fixed or explicitly
excepted before Phase 5.

> The terminal `mark_dim_hr` fixture must be re-measured **after** graphics-policy
> Phase 0 lands (raster gated to `Rich`). With `TerminalImageMode::Never → Off`
> in the bench's pinned options it should fall back under the ceiling; if it
> does not, the same fix-or-except rule applies.

### Part 2 — Baseline trend (the tree-only suite)

Applies to the comprehensive tree-only benchmark suite below, including paths
where no bespoke arm exists (most components, the pipeline-step breakdown,
DarkmatterPage/YamlBlock).

- A Criterion baseline is saved at cutover **Phase 1**
  (`--save-baseline pre-cutover-YYYY-MM-DD`).
- The gate forbids any bench **regressing more than 10%** versus its saved
  baseline. Movements within a ±noise band (set per the run's observed jitter,
  nominally ±5%) do not count.

Part 2 keeps the tree path honest as the cutover flips entry points and deletes
bespoke code in Phases 2–5, when Part 1's comparison arms start disappearing.

## Benchmark Inventory

### biscuit-terminal — every renderable component

One `component_render` suite with a bench per tree-rendering component, each
over a representative input, exercising the **tree** render path
(`render_terminal_node`):

`BlockQuote`, `Compose`, `FileSystem`, `OrderedList`, `UnorderedList`,
`Progress`, `Section`, `StatusBlock`, `Table`, `TextBlock`, `Todo`,
`TwoColumn`, and `Prose` (after its full collapse —
[`../2026-06-02-prose-tree/spec.md`](../2026-06-02-prose-tree/spec.md)).

This consolidates the partial coverage in `render_tree.rs` and `rendering.rs`
and **retires the degenerate `component_render_path_comparison` group** — its
"bespoke" arm routes through the tree just like its "tree" arm, so it no longer
measures a comparison (documented in the tree-cutover baseline note).

### darkmatter

1. **Compose pipeline** — extend `compose_pipeline.rs`: audit the per-stage
   `ComposeOptions::only` coverage against the full stage list (note where shell
   / transclusion stages remain deliberately excluded and why) and keep the
   `full_pipeline` end-to-end. One group, every stage visible together.
2. **Render pipeline (terminal)** — new group `render_pipeline_terminal`,
   step-broken over the shared corpus: `parse` → `fold` → `render` (each
   isolated) plus `full` end-to-end. The fold sub-split (rewrite-scan vs
   event-fold) is already covered by `migration_parity`'s
   `fold_only`/`fold_production`, so it is not duplicated here.
3. **Render pipeline (browser)** — group `render_pipeline_browser`: the same
   `parse` / `fold` / `render` / `full` steps with the browser renderer.
4. **DarkmatterPage (terminal)** — a component bench over a representative page
   (margins, padding, background, max-width) rendering a mixed-content body.
5. **DarkmatterPage (browser)** — requires building a minimal browser render
   first (see [DarkmatterPage Browser Path](#darkmatterpage-browser-path)), then
   the equivalent bench.
6. **YamlBlock (terminal)** — component bench (validated YAML body, highlighted).
7. **YamlBlock (browser)** — component bench.

## DarkmatterPage Browser Path

`DarkmatterPage` currently renders only to the terminal. Item #5 requires a
minimal browser render: the page's layout policy (margin, padding, background,
max-width, alignment) lowered to a styled container wrapping the body's browser
output — e.g. a `<div>` carrying the corresponding CSS (max-width, padding,
margin, background color). Scope is deliberately minimal: enough to render a
faithful page wrapper and be benchmarked, not a full browser layout engine. It
advances DarkmatterPage as a cutover holdout at the same time.

## Mechanics

- **Frozen corpus per crate.** Inputs generated deterministically in-process so
  they do not drift across runs (matching the existing benches' approach).
- **Pinned options.** Fixed terminal width and color depth; browser options
  pinned; `TerminalImageMode::Never → GraphicsMode::Off` where a no-raster
  measurement is intended, so a baseline is reproducible across hosts.
- **Baseline capture.** At Phase 1, run every bench and
  `--save-baseline pre-cutover-YYYY-MM-DD`; record middle estimates and the
  Part-1 ratios in `baselines.md`.
- **When the gate runs.** At cutover **Phase 4** (validate) and re-checked
  immediately before **Phase 5** (delete). Any Part-1 ceiling breach or Part-2
  >10% regression blocks the phase until fixed or excepted.

## Goals

- A single, concrete pass/fail criterion for cutover Acceptance Criteria #4.
- Comprehensive, permanent benchmark coverage: every renderable component, the
  compose pipeline by stage, the render pipeline by step (both targets), and the
  requested DarkmatterPage / YamlBlock component benches.
- Reuse `migration_parity` as the bespoke comparison rather than duplicating it.

## Non-Goals

- Per-component rasterization micro-optimization (perf spec).
- A full browser layout engine for DarkmatterPage (only a minimal wrapper).
- Changing the gate into CI enforcement wiring — this spec defines the gate; how
  it is automated (manual Phase-4 check vs CI job) is an operational follow-up.
- Designing fixtures for components that never reach the tree (the exempt set —
  cutover Decision #5).

## Build Plan

Ordered so each step lands on a green tree:

1. **biscuit-terminal component suite** — add the full `component_render` group;
   retire `component_render_path_comparison`.
2. **darkmatter render-pipeline step benches** — add `render_pipeline_terminal`
   and `render_pipeline_browser` with `parse`/`fold`/`render`/`full`.
3. **Compose-pipeline coverage audit** — confirm/extend `compose_pipeline.rs`.
4. **YamlBlock terminal + browser benches.**
5. **DarkmatterPage terminal bench** + **minimal browser path** + browser bench.
6. **Wire the gate** — document Part-1 ratios and Part-2 baseline in
   `baselines.md`; define the exception-record format.

## Open Questions

Implementation-level only:

- **Exact noise band** for Part 2 (nominal ±5%) — calibrate from observed
  run-to-run jitter on the dev host during Phase 1 capture.
- **Gate automation** — manual Phase-4 check vs a CI job comparing against the
  saved baseline. Default: manual at the gate, automate later.
- **Per-component representative inputs** — small/medium sizing per component so
  no single bench dominates wall-clock; settle during step 1.

## Related Specs

- [`../2026-06-02-tree-cutover/spec.md`](../2026-06-02-tree-cutover/spec.md) —
  resolves its Decision #3, subsumes Decision #9, supplies Acceptance Criteria
  #4's metric; the gate runs at its Phase 4/5.
- [`../2026-06-02-prose-tree/spec.md`](../2026-06-02-prose-tree/spec.md) —
  `Prose` joins the component suite after its collapse.
- [`../2026-05-26-graphics-policy/spec.md`](../2026-05-26-graphics-policy/spec.md) —
  `mark_dim_hr` / HR fixtures must be re-measured after its Phase 0.
- [`../2026-05-21-isolated-perf/spec.md`](../2026-05-21-isolated-perf/spec.md) —
  owns per-component rasterization micro-optimizations the gate may motivate.
- [`../_completed/2026-05-20-darkmatter-tree/baselines.md`](../_completed/2026-05-20-darkmatter-tree/baselines.md) —
  where Part-1 ratios and the Part-2 baseline are recorded.
