---
status: ready for planning and implementation
reviewed: true
depends-on: ../2026-06-02-perf-gate/spec.md
---

# Browser Tree-Renderer Performance

## Status

**Complete — browser perf gate satisfied (2026-06-03).** Implementation landed
the direct document-string renderer and the `NodeAttrs::get_hint` structural fix,
dropping the full-corpus geomean from ≈ 4.15× to **1.58×** (non-exception geomean
**0.88× ≤ 1.0×**, 5/8 fixtures passing, two faster than legacy). The three
residual breaches — `small_prose` (9.74×), `deeply_nested_lists` (3.70×), and
`mark_dim_hr` (2.08×) — are documented **added-fidelity** exceptions (byte ratio
tracks time ratio in each case) and were **signed off one by one by the cutover
owner (Ken Snyder) on 2026-06-03**. See the accepted-exception table in
[`baselines.md`](../_completed/2026-05-20-darkmatter-tree/baselines.md)
("Post-Fix Browser Gate") and [`review-1.md`](./review-1.md) ("Resolution"). No
structural perf blocker remains; the tree cutover's browser gate is cleared.

**Ready for planning and implementation.** The review locks the design direction:
the production browser tree-document path should add a direct
`RenderNode` → full HTML string renderer instead of building a second
`BrowserFragment` tree and then serializing it. The existing
`BrowserRenderable` / `BrowserFragment<Ready>` composition contract remains
intact for component composition; the string path is a render-tree document
entry point used by the cutover path and benchmarks.

The problem statement and measurement setup (below) stand. The investigation
phase is complete; its results and the chosen fix direction are recorded in
[§4 Investigation Findings](#4-investigation-findings-2026-06-03). One outcome
of the investigation: the sibling `2026-05-21-isolated-perf` spec was removed —
its terminal/fold items did not bear on the browser gate (see §4). Treat §1–§3
as the original handoff context and §4 plus the reviewed sections that follow
as the implementation contract.

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
- `2026-05-21-isolated-perf` (removed) — was a tree-pipeline perf
  investigation. **Decided (see §4): it did not bear on the browser gate**
  (terminal/fold work measured against a different baseline), so it was removed;
  its one surviving idea (the accepted terminal no-color exception) moved to the
  perf-gate spec, and its fold-hygiene item is captured in §4 here.
- [`../2026-05-26-graphics-policy/spec.md`](../2026-05-26-graphics-policy/spec.md) —
  the styled-HR-SVG browser cost (relevant to `mark_dim_hr`).

## 4. Investigation Findings (2026-06-03)

### Root cause: an architecture cost, not a hotspot

The §1 framing holds — it is the whole non-code browser path — and the reason
is structural, confirmed by reading both render paths:

- **Legacy** (`html.rs` `as_html`) walks the pulldown-cmark event stream and
  `push_str`s straight into one `String`. One pass, one buffer.
- **Tree** folds Markdown → an owned `RenderNode` tree (**tree #1**), then
  `render_browser_document` walks tree #1 and **builds a second owned tree** of
  `BrowserFragment<Ready>` / `ComposableNode` (**tree #2**,
  `renderable/src/tree/render/browser.rs`), then serializes tree #2 to the
  `String` (`renderable/src/browser/fragment.rs`).

Tree #2 is overhead legacy never pays, and it is expensive *per node*:

- every child is a heap-allocated `Box<BrowserFragment>` (`fragment.rs:89`);
- every `BrowserFragment` carries a `HashMap` + two `Vec`s
  (`fragment.rs:108-115`), constructed per node;
- `node_attributes` (`browser.rs:1265`) allocates a fresh `Vec<HtmlAttribute>`
  + per-attribute `String`s + a `join` for **every** node, including
  attribute-less text cells;
- `text_fragment` (`browser.rs:1471`) clones each text run into a `String` to
  wrap it, which is escaped into *another* `String` at serialization.

`large_table` is the 18× worst purely because it has the most nodes (200×8 =
1600 cells → ~3000+ fragments); the per-node overhead multiplies. The browser
`render_table` (`browser.rs:892`) does **no** column-width / alignment planning
— it emits `<thead>`/`<tbody>` directly with a per-cell `text-align`. So the
spec's open question is settled: **the 18× is structural per-node overhead
scaled by node count, not a distinct table algorithm.**

### Key finding: the browser gate under-measures the tree path

`migration_parity.rs:461-467` measures the tree side as
`fold + render_browser_document` but **never calls `.output.render()`** — it
stops at the built `HtmlPage` (tree #2). Production *does* serialize
(`render_tree/entrypoints.rs:81-83`; `DarkmatterPage::render_to_browser`,
`layout/page.rs:910`). Consequences:

1. The reported 3.80× / 18× is dominated by **building tree #2**; serialization
   is not even in the number.
2. `HtmlPage::render()` (`html/mod.rs:250`) adds cost production pays but the
   gate hides: `merged_metadata()`, `stylesheet()`, `first_h1_text()`, and
   `collect_dedup_links()` each call `all_fragments()`, a **separate full
   recursive walk building a `Vec<&BrowserFragment>` of every node** — 3-4 whole-
   tree traversals before one body byte is written — plus return-string-and-
   concat serialization (`fragment.rs:303-324`) that re-copies all descendant
   bytes at each nesting level.

The browser gate therefore compares "legacy full string" against "tree without
serialization." **Fix the bench (add `.output.render()` to the tree side)
before re-baselining**, or the numbers understate the real end-to-end gap.

### Opportunities, ranked by leverage

1. **Bypass tree #2 for the document path** — render `RenderNode` → `String`
   directly via a streaming writer, keeping the `BrowserFragment` API only for
   external composition consumers. Removes the largest measured cost and the
   biggest table multiplier. **This is the chosen direction.**
2. **Streaming writer instead of return-and-concat** — even if tree #2 stays,
   write into `&mut String` so deep/wide trees stop re-copying descendant bytes
   per level. Disproportionately helps `large_table` / `deeply_nested_lists`.
3. **Collapse the `HtmlPage::render` rollup walks** — one combined traversal, or
   short-circuit when metadata / features / stylesheets are empty (the norm for
   parsed Markdown). Removes 3-4 full-tree `Vec` allocations.
4. **Per-node allocation hygiene** — skip the `node_attributes` `Vec` +
   `format!` temporaries when a node has no attrs; `write!` attributes directly
   (`render_attributes` / `push_pair`).
5. **Stream-escape text runs** — write escaped bytes straight to the buffer
   instead of `to_string()` → escape-into-new-`String`.

All preserve byte-output, guarded by the `render_tree_parity` / HR-snapshot
tests.

### Reviewed design decision: add a document-string renderer

Implement a public render-tree document entry point that returns the final HTML
string, tentatively named:

```rust
render_browser_document_html(
    doc: &Document,
    opts: &BrowserRenderOptions,
) -> Result<Rendered<String>, RenderError>
```

The exact name can follow local API naming during implementation, but the
behavioral contract is fixed:

- It validates the `Document` with the same `gate` / strictness / diagnostics
  semantics as `render_browser_document`.
- It emits the same full-page bytes as
  `render_browser_document(doc, opts)?.output.render()` for supported tree
  inputs, including `RawHtmlPolicy`, `GraphicsMode`, Mermaid mode, styled HR,
  page options, escaping, attributes, and semantic wrapping.
- It streams HTML into one output buffer and does not construct a full
  `BrowserFragment` tree for every render-tree node.
- It keeps `render_browser_document` and `render_browser_node` available and
  behavior-compatible for callers that need `HtmlPage` or
  `BrowserFragment<Ready>` composition.
- If `BrowserRenderOptions::code_renderer` returns a `BrowserFragment`, the new
  writer may serialize that isolated hook result into the output buffer. That is
  acceptable because the hook is an extension island; it must not reintroduce a
  second fragment tree for the whole document.

Reader note: this is an intentional addition to the renderable browser contract,
not a reversal of the existing "no legacy string surface for
`BrowserRenderable`" decision. Components still compose through
`BrowserFragment<Ready>`. The new string surface is for the shared render-tree
document renderer, where the caller already has an owned `Document` and needs
the final browser output.

Rejected alternatives:

| Alternative | Pros | Cons |
|---|---|---|
| Optimize `BrowserFragment` construction only | Lowest API churn; improves all fragment users somewhat | Leaves tree #2 in the document hot path and keeps the largest measured cost, especially for tables |
| Change `BrowserRenderable` to return strings | Direct and fast for simple components | Breaks established composition semantics for stylesheets, metadata, links, and features |
| Keep the current API and only collapse `HtmlPage::render` traversals | Small, localized patch | Fixes hidden serialization overhead but not the measured gate failure, which is dominated by building tree #2 |

### The removed `isolated-perf` spec — why it didn't bear on this gate

Every item of the former `2026-05-21-isolated-perf` spec was checked against the
browser problem:

| Item | Target | Helps browser? | Disposition |
|---|---|---|---|
| QW-1 raster memo | Terminal/graphics | No | Owned by graphics-policy |
| QW-2 SVG string | Terminal HR | No | Owned by graphics-policy |
| QW-3 resvg Options | Terminal | No | Owned by graphics-policy |
| QW-4 fold hygiene | All targets | Partially — fold is ~11% of browser cost | Captured below |
| FC-2 no-color fast path | Terminal | No | Accepted exception in perf-gate spec |

`isolated-perf` was almost entirely terminal / fold work, measured against the
*slow* legacy terminal renderer. Browser is the render step (~89% of cost)
measured against the *fast* legacy HTML streamer. Only QW-4 touched the browser,
and only the fold (~29 µs), not the render step where the gate fails. **Running
`isolated-perf` first would not have moved the browser gate**, and none of its
items were approved work — so the spec was removed. Its substance is preserved:
the terminal-rasterization items (QW-1/2/3) are owned by graphics-policy, the
no-color observation (FC-2) is recorded as an accepted exception in the perf-gate
spec, and the fold-hygiene item (QW-4) is folded into the sequence below — it may
be pulled forward opportunistically (shared fold) but is secondary to the
render-step / tree-#2 fixes this spec owns.

### Recommended implementation sequence

1. Fix the gate measurement gap (`.output.render()` on the tree side); run the
   two cheap diagnostics already listed in §"To Investigate" (per-fixture byte-
   size parity; re-capture the baseline on a quiescent host) for honest numbers.
2. Add the direct document-string renderer and switch the production darkmatter
   browser cutover path plus the browser gate benchmark to measure that final
   string. Keep `render_browser_document` intact for composition callers and
   tests that need `HtmlPage`.
3. Collapse `HtmlPage::render` rollup walks for the fragment-composition API so
   non-tree component users also avoid avoidable full-tree traversals.
4. Apply per-node allocation hygiene in the shared browser writer: stream
   attributes and text escaping into the destination buffer, and skip temporary
   vectors/strings for attribute-less nodes.
5. Pull QW-4 forward opportunistically.

## Goals

- Bring the browser tree path under the perf gate: per-target geomean ≤ 1.0×,
  no fixture beyond 1.5× legacy — or documented, signed-off exceptions.
- Preserve browser output fidelity (parity or intended improvements only).
- Keep fixes in the shared `renderable` browser renderer where possible.
- Preserve the existing `BrowserRenderable` / `BrowserFragment<Ready>` /
  `HtmlPage` public composition contract while adding the faster document-string
  path.

## Non-Goals

- Terminal perf (already passes).
- Changing the perf-gate criterion. Updating `migration_parity` to measure the
  final tree HTML string is a benchmark correctness fix, not a criterion change.
- Replacing `BrowserFragment<Ready>` as the component composition currency.
- Re-litigating the cutover sequencing or any resolved cutover decision.

## Acceptance Criteria

1. `migration_parity` browser tree benches measure the same production surface
   as legacy: a final HTML `String`. The tree side must not stop at
   `Rendered<HtmlPage>`.
2. A fresh quiescent-host baseline is captured after the measurement fix and
   again after the renderer work; `baselines.md` records the ratios and any
   signed-off exceptions.
3. Browser parity tests compare final HTML strings for legacy vs tree, including
   at least prose, table, list, links/images, code, raw HTML policy, `mark`,
   and graphics-policy HR fixtures.
4. The direct document-string renderer and the existing
   `render_browser_document(...).output.render()` path produce identical bytes
   for the shared fixture corpus unless a deliberate fidelity improvement is
   documented in this spec or the dependency specs.
5. Existing public fragment/page composition tests still pass. Add at least one
   regression test proving `BrowserFragment` metadata, dependency links,
   stylesheets, and features still roll up through `HtmlPage::render`.
6. Rustdoc and skill docs are updated where public browser-renderer surfaces are
   added or behavior is clarified.

## To Investigate (largely resolved in §4)

This checklist drove the investigation. **Resolved:** the table 18× is
structural overhead scaled by node count, not a table algorithm (no width
planning exists); the cost is concentrated in building/serializing the
intermediate fragment tree (#4); the `isolated-perf` spec was removed as not
bearing on this gate.
**Still open / first implementation steps:** the byte-size fidelity-vs-overhead
diagnostic and the quiescent-host re-baseline (both deferred to the gate-fix
step in §4's sequence).

The original checklist, for reference:

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

## Open Questions

No architecture-level question remains open after review. Implementation should
settle these small choices locally:

- **Public function name.** Recommended:
  `render_browser_document_html`, because it mirrors
  `render_browser_document` while making the final string surface explicit.
  `render_browser_document_to_string` is acceptable if the surrounding module
  strongly prefers verb-style names.
- **Head streaming implementation.** Recommended: factor shared head/body
  helpers so the string renderer and `HtmlPage::render` cannot drift. Duplicating
  head construction would be faster to write, but it makes title, metadata,
  stylesheet, script, and link ordering harder to keep compatible.
- **Exception policy after honest measurement.** Recommended: no new browser
  exceptions unless a fixture's remaining gap is caused by documented added
  fidelity, not by structural overhead.

## Related Specs

- [`../2026-06-02-tree-cutover/spec.md`](../2026-06-02-tree-cutover/spec.md)
- [`../2026-06-02-perf-gate/spec.md`](../2026-06-02-perf-gate/spec.md)
- [`../2026-05-26-graphics-policy/spec.md`](../2026-05-26-graphics-policy/spec.md)
- [`../_completed/2026-05-20-darkmatter-tree/baselines.md`](../_completed/2026-05-20-darkmatter-tree/baselines.md)
