---
status: draft
---

# Browser Tree-Renderer Performance

## Status

**Draft — problem statement and measurement setup.** This spec captures the
state of play, how performance is measured, and the context a fresh session
needs to start the browser-perf investigation. It deliberately does **not**
root-cause or design the fix — that is the next phase. Treat this as the
handoff document: read it, then investigate.

The browser tree renderer is the **single remaining blocker** for the tree
cutover ([`../2026-06-02-tree-cutover/spec.md`](../2026-06-02-tree-cutover/spec.md)).
Every other phase (graphics-policy, prose-tree, non-structural, the perf-gate
suite) is implemented; the terminal tree path passes the perf gate decisively.
The browser tree path fails it broadly.

## 1. Where We Are

### The cutover, in one paragraph

Darkmatter's public renderers (`Markdown::as_html`, `Markdown::for_terminal`,
`DarkmatterPage::render` / `render_to_browser`) still run the **legacy
event-stream serializers**. The plan is to flip them to the **render-tree**
pipeline (fold Markdown → `Document` → per-target tree renderers) and then
delete the legacy serializers. The cutover's deletion gate requires "no
regressions" and a net-faster performance trend (the perf gate, §2). Terminal
clears it; **browser does not**.

### Terminal: passes (for context)

`migration_parity` tree ÷ legacy, terminal target: **geomean 0.122×** (tree
≈ 8× faster across the corpus). Only `mark_dim_hr` exceeds 1.0× (1.10×), within
the 1.5× per-fixture ceiling. The terminal cutover is unblocked on perf.

### Browser: fails, broadly — this is what must improve

`migration_parity` tree ÷ legacy, browser target: **geomean 3.80×**, with
**7 of 8 fixtures breaching the 1.5× ceiling**. From the 2026-06-03 provisional
baseline (numbers are load-contaminated — see §2 caveat — but the **ratios and
direction are robust**):

| Fixture | tree ÷ legacy | vs 1.5× ceiling |
|---|---|---|
| `small_prose` | 9.89× | breach |
| `large_prose` | 7.57× | breach |
| `large_code_block` | 0.37× | **pass** (both pay syntect) |
| `large_table` | 18.38× | breach (worst) |
| `deeply_nested_lists` | 3.53× | breach |
| `many_links_images` | 2.37× | breach |
| `mark_dim_hr` | 5.27× | breach |
| `image_heavy` | 1.96× | breach |

**What needs to improve:** the **whole browser tree render path**, not a single
hotspot. The cutover's Decision #9 originally framed this as a `large_table`
hotspot; the data shows it is every non-code fixture. The target is to bring the
browser path under the gate — **per-target geomean ≤ 1.0× and no fixture beyond
1.5× legacy** — or to document and accept specific exceptions.

### The bar is high (framing, not investigation)

Legacy `as_html` is a tight event→string streamer and is genuinely fast
(`small_prose` ≈ 3.9 µs; `large_table` ≈ 1.5 ms). The tree path pays for folding
to an owned `Document`, then walking it and building strings per node. That
overhead was **hidden** against the heavy legacy *terminal* renderer (which is
milliseconds-slow, so the tree wins easily) but is **exposed** against the fast
legacy *HTML* renderer. `large_code_block` is the lone browser pass because both
sides pay the dominant syntect highlighting cost, which swamps the structural
overhead. So the investigation is about **structural/allocation overhead in the
browser node walk**, measured against a fast baseline — not about an algorithmic
blowup (except possibly `large_table`'s 18×, which may have its own issue).

## 2. How We Measure Performance

### The perf gate (the acceptance bar)

Defined in [`../2026-06-02-perf-gate/spec.md`](../2026-06-02-perf-gate/spec.md).
Two parts, both must pass before bespoke deletion:

1. **Bespoke comparison** (where a legacy arm survives — the browser render
   pipeline, via `migration_parity`): per-target **geomean of tree ÷ legacy
   ≤ 1.0×**, with a **hard per-fixture ceiling of 1.5× legacy**. A fixture over
   the ceiling is fixed or carries a documented, signed-off exception.
2. **Baseline trend** (tree-only suites): no bench regresses **> 10%** vs its
   saved Criterion baseline (`pre-cutover-2026-06-03`), outside a ±noise band.

This spec's success criterion = the **browser** half of Part 1 passing.

### The benchmark suite (the instrument)

| Bench | What it measures | Use for this work |
|---|---|---|
| `darkmatter/lib/benches/migration_parity.rs` | legacy vs tree, full render, per fixture, per target | **The gate metric.** Browser `/legacy` vs `/tree` is the number to move. |
| `darkmatter/lib/benches/render_pipeline_steps.rs` | tree-only `parse`/`fold`/`render`/`full`, terminal + browser | **Localize the cost** — is it in the fold or the render step? |
| `biscuit-terminal/lib/benches/render_tree.rs` | per-component tree render (terminal) | Component-level signal (terminal). |
| `darkmatter/lib/benches/compose_pipeline.rs` | compose stages | Unrelated to browser render; baseline-tracked. |

Run (reduced flags = the baseline convention; in-code `sample_size(20)` wins
over `--sample-size`):

```bash
# The gate number for browser:
cargo bench -p darkmatter --bench migration_parity -- migration/browser \
    --warm-up-time 1 --measurement-time 3 --sample-size 10

# Localize fold vs render:
cargo bench -p darkmatter --bench render_pipeline_steps -- render_pipeline_browser \
    --warm-up-time 1 --measurement-time 1 --sample-size 10

# Compare against the saved baseline after a change:
cargo bench -p darkmatter --bench migration_parity -- migration/browser --baseline pre-cutover-2026-06-03
```

### Already-measured localization (no new investigation)

From `render_pipeline_browser` on its mixed corpus: `parse` ≈ 1.2 µs, `fold`
≈ 29 µs, `render` ≈ 230 µs, `full` ≈ 258 µs. **The render step (node tree →
HTML) dominates** — ~8× the fold. So the browser cost is concentrated in the
node walk / string building, not the fold. (Terminal is similar in shape —
`render` ≈ 378 µs vs `fold` ≈ 29 µs — yet terminal passes the gate, because the
*legacy* terminal renderer it is compared against is far slower than legacy
`as_html`.)

### Baseline caveat (must address early)

The `pre-cutover-2026-06-03` baseline was captured while the host was under load:
`migration/browser/large_code_block/legacy` read 164 ms vs ≈ 16 ms in the
2026-06-02 full run (~10× inflation). **Re-capture both Part-1 ratios and the
Part-2 baseline on a quiescent host** before treating absolute numbers as
authoritative. The ratios/direction in §1 are robust to this (legacy and tree
are measured back-to-back within one run); the absolute µs/ms values are not.
Recorded in
[`../_completed/2026-05-20-darkmatter-tree/baselines.md`](../_completed/2026-05-20-darkmatter-tree/baselines.md)
("Perf-Gate Baseline (2026-06-03 … PROVISIONAL)").

## 3. Context for a Full Understanding

### The two code paths

- **Legacy (fast baseline):** `darkmatter/lib/src/markdown/output/html.rs`
  (`as_html`, line ~153) — walks the `pulldown-cmark` event stream and writes
  HTML straight to a `String`. One pass, minimal allocation.
- **Tree (the path to optimize):** fold via
  `darkmatter::markdown::render_tree::fold_markdown_spanned_with_frontmatter`
  → `renderable::tree::Document` → `renderable/src/tree/render/browser.rs`
  (`render_browser_document`, line ~150; `render_span`; `render_thematic_break`
  ~line 425 with the `graphics_mode` HR/SVG logic). Builds an owned node tree,
  then recursively renders each node to a `String`.

### Constraints (do not regress these)

- **No fidelity regressions.** Output parity with legacy, except deliberate
  documented improvements (the `<mark>` recovery from the inline-span spec;
  styled HR `<svg>` at `GraphicsMode::Vector`+ from graphics-policy). Any perf
  fix must preserve byte-output (the `render_tree_parity` / HR-snapshot tests
  guard this).
- **`mark_dim_hr` browser (5.27×) includes graphics-policy's styled-SVG HR**,
  which is *new, intended* browser work (Vector-tier fidelity). Some of that
  fixture's cost is fidelity we chose to add, not pure overhead — separate the
  two when judging it against the ceiling.
- The fix is structural/allocation hygiene in the shared
  `renderable::tree::render::browser` walk, which is used by every browser-tree
  consumer — keep it general, not darkmatter-specific.

### Relationship to other specs

- [`../2026-06-02-tree-cutover/spec.md`](../2026-06-02-tree-cutover/spec.md) —
  this work unblocks its browser cutover (Phase 4 gate / Phase 5 deletion).
- [`../2026-06-02-perf-gate/spec.md`](../2026-06-02-perf-gate/spec.md) — defines
  the acceptance bar and the bench suite used here.
- [`../2026-05-21-isolated-perf/spec.md`](../2026-05-21-isolated-perf/spec.md) —
  owns tree-pipeline perf hotspots and "fold hygiene." This browser work is its
  natural continuation; the investigation may extend that spec or supersede its
  browser items. Decide during the investigation.
- [`../2026-05-26-graphics-policy/spec.md`](../2026-05-26-graphics-policy/spec.md) —
  the styled-HR-SVG browser cost (relevant to `mark_dim_hr`).

## Goals

- Bring the browser tree path under the perf gate: per-target geomean ≤ 1.0×,
  no fixture beyond 1.5× legacy — or documented, signed-off exceptions.
- Preserve browser output fidelity (parity or intended improvements only).
- Keep fixes in the shared `renderable` browser renderer where possible.

## Non-Goals

- Root-causing or designing the fix — that is the investigation phase this spec
  hands off to.
- Terminal perf (already passes).
- Changing the gate criterion or the bench suite.
- Re-litigating the cutover sequencing or any resolved cutover decision.

## To Investigate (next phase — not done here)

A checklist for the investigation, deliberately not pre-judged:

- **Separate "added fidelity" from "structural overhead."** Some of the regression
  is real work the tree path *should* do: `mark_dim_hr` now emits styled HR
  `<svg>` (graphics-policy Vector tier) and `<mark>`, which legacy did not on the
  tree path. But the 6 plain-CommonMark fixtures (prose/table/list/links/images)
  emit equivalent HTML on both sides, so their 2–18× is overhead, not fidelity.
  **Cheap first diagnostic:** compare legacy vs tree output **byte-length** per
  fixture — equal size ⇒ pure overhead (same output, slower path); tree larger
  ⇒ it emits extra markup (classes / `data-*` / provenance) that is heavier and
  arguably higher-fidelity. This directly settles "was the bespoke renderer just
  doing less / lower-fidelity work?" before any profiling.
- For `large_table` (18×), check whether the tree table renderer does column
  width / alignment **planning** that HTML auto-layout makes unnecessary —
  wasted work rather than fidelity.
- Re-capture the baseline on a quiescent host; confirm the ratios hold.
- Use `render_pipeline_browser` (and a per-fixture variant if needed) to confirm
  the cost is in `render`, and break the render step down by node kind.
- Determine why `large_table` is the worst (18×) — is it the same structural
  overhead scaled by node count, or a distinct algorithmic issue in table
  rendering?
- Profile the browser node walk for allocation/string-building overhead vs the
  legacy streamer; identify the highest-leverage reductions.
- Decide whether any fixture's residual gap is an accepted, documented exception
  vs a must-fix.
- Decide whether this folds into `2026-05-21-isolated-perf` or stands alone.

## Related Specs

- [`../2026-06-02-tree-cutover/spec.md`](../2026-06-02-tree-cutover/spec.md)
- [`../2026-06-02-perf-gate/spec.md`](../2026-06-02-perf-gate/spec.md)
- [`../2026-05-21-isolated-perf/spec.md`](../2026-05-21-isolated-perf/spec.md)
- [`../2026-05-26-graphics-policy/spec.md`](../2026-05-26-graphics-policy/spec.md)
- [`../_completed/2026-05-20-darkmatter-tree/baselines.md`](../_completed/2026-05-20-darkmatter-tree/baselines.md)
