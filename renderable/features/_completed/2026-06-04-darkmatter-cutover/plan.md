---
source_files_during_phase_1:
  - darkmatter/lib/tests/cutover_reference.rs
docs_updated_during_phase_1: []
docs_created_during_phase_1: []
skills_files_updated_during_phase_1: []
source_files_during_phase_2:
  - darkmatter/lib/src/style/apply.rs
  - darkmatter/lib/src/layout/page.rs
  - darkmatter/lib/src/layout/mod.rs
docs_updated_during_phase_2: []
docs_created_during_phase_2: []
skills_files_updated_during_phase_2: []
source_files_during_phase_3:
  - darkmatter/lib/src/markdown/render_tree/decorate.rs
  - darkmatter/lib/src/layout/context.rs
  - darkmatter/lib/src/layout/page.rs
  - darkmatter/lib/src/layout/mod.rs
  - darkmatter/lib/src/markdown/render_tree/entrypoints.rs
  - darkmatter/cli/tests/level2_layout.rs
  - darkmatter/lib/tests/snapshots/cutover_reference__ref_centered_table_terminal.snap
docs_updated_during_phase_3: []
docs_created_during_phase_3: []
skills_files_updated_during_phase_3: []
source_files_during_phase_7:
  - darkmatter/lib/src/layout/mod.rs
  - darkmatter/lib/src/style/apply.rs
  - darkmatter/lib/src/layout/error.rs
  - darkmatter/lib/src/markdown/output/code_block.rs
  - darkmatter/lib/src/style/schema/lists.rs
  - darkmatter/cli/tests/cli.rs
docs_updated_during_phase_7:
  - darkmatter/docs/rendering/style.md
docs_created_during_phase_7: []
skills_files_updated_during_phase_7:
  - .claude/skills/darkmatter/SKILL.md
packages:
  - darkmatter
---

# darkmatter Cutover Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan phase-by-phase. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Lower darkmatter's `style:` frontmatter **directly** into `Layout`/`Style` node attrs, delete the deprecated `Page*` vocabulary, `build_component_css`, the per-component `LayoutContext` math, and every `#![allow(deprecated)]` — keeping a slim renderable-typed page frame.

**Architecture:** `apply.rs` builds a per-`PageComponent` `ComponentPolicy { layout, style }` straight from `style:` values (no `map_alignment`/`lower_length_to_fill`); `decorate.rs` writes those onto nodes and lets the renderer folds do all width/padding/alignment/CSS math; `DarkmatterPage` survives as a slim page frame storing renderable types. Build the new path, prove parity against captured references, then delete the old path and the deprecated types last.

**Tech Stack:** Rust 2024, the monorepo `cargo`/`just` tooling, `insta` snapshots, `md hash`.

**Spec:** [`spec.md`](spec.md). **Depends on** [`renderer-folds`](../2026-06-04-renderer-folds/spec.md) (and transitively style-vocabulary + tree-attrs) being implemented — this plan assumes the terminal/browser folds already render `padding`/`width`/`border` from node attrs, `InheritedStyle` exists, and `NodeAttrs` is typed. Confirm with `cargo build -p darkmatter` before starting.

**Parity note:** Output is a *reference*, not a byte contract (architecture spec). The deleted `LayoutContext` math now happens in the fold; intended per-component diffs are re-baselined with rationale.

---

## File Structure

- `darkmatter/lib/src/style/apply.rs` — build `ComponentPolicy` directly; delete `map_alignment`, `lower_length_to_fill`, `Length→u16/WidthUnit` helpers.
- `darkmatter/lib/src/markdown/render_tree/decorate.rs` — write `Layout`/`Style` from `ComponentPolicy`; drop `LayoutContext` per-component queries; use `InheritedStyle`.
- `darkmatter/lib/src/layout/page.rs` — slim page frame on renderable types; delete `build_component_css` + `component_selectors`/`emit_component_*`.
- `darkmatter/lib/src/layout/context.rs` — reduce `LayoutContext` to page-frame residue.
- `darkmatter/lib/src/layout/types.rs` — delete `PageMargin`/`PagePadding`/`PageAlignment`/`PageFill`/`WidthUnit`/`PageComponent::Lists` + bridges.
- `darkmatter/lib/src/layout/mod.rs`, `darkmatter/lib/src/cli.rs`, `darkmatter/lib/tests/{layout_snapshots,style_frontmatter}.rs` — remove `#![allow(deprecated)]`.

**Baseline:**

- [ ] **Step 0: Confirm dependencies landed + green**

Run: `cargo build -p darkmatter && cargo test -p darkmatter --no-run`
Expected: clean; the renderable folds render `padding`/`width`/`border` (renderer-folds) and `InheritedStyle` resolves.

---

## Phase 1: Capture the parity reference (before any change)

The deleted `LayoutContext` math moves to the fold; capture the current `style:` output first so intended diffs are visible and deliberate.

**Files:**
- Create: `darkmatter/lib/tests/cutover_reference.rs`

- [x] **Step 1: Write reference snapshots of representative `style:` cases**

```rust
//! Pre-cutover reference output for representative `style:` per-component cases.
//! These snapshots are the parity *reference* (not a byte contract) the cutover
//! is diffed against; intended diffs are re-accepted with a note in Phase 6.
use darkmatter::markdown::Markdown;
// helper: render `md` with `style:` frontmatter to terminal and browser at width 80

#[test]
fn reference_centered_table() {
    let md = with_style("table:\n  align: center\n", TABLE_FIXTURE);
    insta::assert_snapshot!("ref_centered_table_terminal", render_terminal(&md, 80));
    insta::assert_snapshot!("ref_centered_table_browser", render_browser(&md));
}

// Repeat for: padded code block (`code-blocks: { fill: pad 4ch }`),
// indented block-quote (`block-quote: { fill: indent 6ch, align: left }`),
// list left-margin (`ul: { margin-left: 4 }`), page background pronounced,
// and a page margin+padding case.
```

- [x] **Step 2: Generate the snapshots**

Run: `cargo test -p darkmatter --test cutover_reference` then `cargo insta accept`
Expected: snapshots written. These are the baseline.

- [x] **Step 3: Commit**

```bash
git add darkmatter/lib/tests/cutover_reference.rs darkmatter/lib/tests/snapshots
git commit -m "test(darkmatter): capture pre-cutover style: parity reference"
```

---

## Phase 2: `ComponentPolicy` + lower `style:` directly (delete down-conversion)

**Files:**
- Modify: `darkmatter/lib/src/style/apply.rs`
- Modify: `darkmatter/lib/src/layout/page.rs` (store `HashMap<PageComponent, ComponentPolicy>`; replace the deprecated builder setters)

- [x] **Step 1: Write the failing test**

```rust
    #[test]
    fn apply_lowers_component_style_directly_to_layout() {
        // `table: { align: center, fill: max 60ch }` → ComponentPolicy with
        // Layout { alignment: Center, max_width: Some(60ch) }, no deprecated types.
        let page = apply_for_test("table:\n  align: center\n  fill: max 60ch\n");
        let policy = page.component_policy(PageComponent::Tables).unwrap();
        assert_eq!(policy.layout.alignment, renderable::layout::Alignment::Center);
        assert_eq!(
            policy.layout.max_width,
            Some(renderable::layout::TargetValue::universal(renderable::layout::Length::ch(60)))
        );
    }

    #[test]
    fn apply_lowers_pad_to_padding() {
        let page = apply_for_test("code-blocks:\n  fill: pad 4ch\n");
        let policy = page.component_policy(PageComponent::CodeBlocks).unwrap();
        assert_eq!(policy.layout.padding.left,
            renderable::layout::TargetValue::universal(renderable::layout::Length::ch(4)));
    }
```

- [x] **Step 2: Run to verify failure**

Run: `cargo test -p darkmatter apply_lowers`
Expected: FAIL — no `ComponentPolicy` / `component_policy`.

- [x] **Step 3: Add `ComponentPolicy` + the policy map; rewrite the apply functions**

In `page.rs`:

```rust
/// The renderable policy a `style:`-configured PageComponent contributes.
#[derive(Debug, Clone, Default)]
pub(crate) struct ComponentPolicy {
    pub layout: renderable::layout::Layout,
    pub style: Option<renderable::style::Style>,
}
```

Store `component_policies: HashMap<PageComponent, ComponentPolicy>` on `DarkmatterPage`; add `component_policy(&self, c) -> Option<&ComponentPolicy>` and builder helpers `with_component_layout` / `with_component_style` (or a single `with_component_policy`).

In `apply.rs`, rewrite `apply_page_style` / `apply_component_style` / `apply_list_style` / `apply_color_style` / `apply_hr_style` to build `ComponentPolicy` directly using the style-vocabulary mapping:
- `align` → `Layout.alignment` (no `map_alignment`),
- `fill: pad <len>` → `Layout.padding` (symmetric or aligned side),
- `fill: indent <len>` → `Layout.padding` on the aligned side,
- `fill: max <len>` → `Layout.max_width`,
- `width <len>` → `Layout.width = Width::Fixed`,
- `margin-*` → `Layout.margin` (`Edges`),
- `color`/`bg-color` → `ComponentPolicy.style` (`Style.color` / `Style.background`).

Delete `map_alignment`, `lower_length_to_fill`, `lower_length_to_width_unit`, and the `Length→u16`/`WidthUnit` helpers that fed the deprecated builder.

> Page-level `style.page.*` (margin/padding/background/max-width) feeds the **page frame** (Phase 5), not a `ComponentPolicy`.

- [x] **Step 4: Run to verify pass**

Run: `cargo test -p darkmatter apply_lowers`
Expected: PASS.

- [x] **Step 5: Build (deprecated builder setters now unused on the per-component path)**

Run: `cargo build -p darkmatter 2>&1 | rg "deprecated|unused" | head`
Expected: per-component `use_alignment`/`with_fill` calls are gone from `apply.rs`. Leave the deprecated *types* in place for now (deleted in Phase 6); they just have no per-component callers.

- [x] **Step 6: Commit**

```bash
git add darkmatter/lib/src/style/apply.rs darkmatter/lib/src/layout/page.rs
git commit -m "feat(darkmatter): lower style: per-component policy directly to renderable Layout/Style"
```

---

## Phase 3: `decorate.rs` writes attrs; drop `LayoutContext` per-component math

**Files:**
- Modify: `darkmatter/lib/src/markdown/render_tree/decorate.rs`
- Modify: `darkmatter/lib/src/layout/context.rs` (delete per-component methods)

- [x] **Step 1: Write the failing test**

```rust
    #[test]
    fn decorate_writes_component_layout_onto_nodes() {
        // A doc with a table + `table: { align: center }` → the Table node carries
        // Layout { alignment: Center } and decorate does NO width math itself.
        let doc = decorate_for_test(TABLE_MD, "table:\n  align: center\n");
        let table = find_node(&doc, |n| matches!(n.kind, NodeKind::Table { .. })).unwrap();
        assert_eq!(table.attrs.layout_ref().unwrap().alignment, Alignment::Center);
    }
```

- [x] **Step 2: Run to verify failure**

Run: `cargo test -p darkmatter decorate_writes_component_layout`
Expected: FAIL (decorate still queries `LayoutContext`).

- [x] **Step 3: Rewrite decorate + delete the per-component context math**

In `decorate.rs`: keep `component_for(NodeKind) → PageComponent`; for each block look up its `ComponentPolicy` and write `policy.layout` (and `policy.style` if `Some`) onto the node via `set_layout`/`set_style`. Remove all calls to `resolve_component_width`, `alignment_padding`, `component_side_padding`, `component_fill`, `component_alignment`, and the inline `cells()` math — the renderer fold does it. Replace darkmatter's bespoke inheritance push-down with `renderable::tree::InheritedStyle`.

In `context.rs`: delete `resolve_component_width`, `alignment_padding`, `component_side_padding`, `component_fill`, `component_alignment`, `list_left_margin`, the `alignments`/`fills`/`list_left_margins` fields, and `resolve_width_unit`. (Keep the page-frame fields for Phase 5.)

- [x] **Step 4: Run to verify pass + diff against the reference**

Run: `cargo test -p darkmatter decorate_writes_component_layout`
Expected: PASS.

Run: `cargo test -p darkmatter --test cutover_reference`
Expected: terminal snapshots may now differ (fold math vs the old `LayoutContext` math). Do **not** accept yet — inspect each diff; it should be the same layout intent. Real regressions are fixed here; intended diffs are accepted in Phase 6.

- [x] **Step 5: Commit**

```bash
git add darkmatter/lib/src/markdown/render_tree/decorate.rs darkmatter/lib/src/layout/context.rs
git commit -m "feat(darkmatter): decorate writes component Layout/Style; delete LayoutContext per-component math"
```

---

## Phase 4: Delete `build_component_css` (browser uses the fold)

**Files:**
- Modify: `darkmatter/lib/src/layout/page.rs`

- [ ] **Step 1: Write the failing test**

```rust
    #[test]
    fn browser_per_component_css_comes_from_the_fold_not_build_component_css() {
        // `table: { align: center }` → the table node's Layout lowers to margin:auto
        // via the renderable browser fold; no `.darkmatter-page table { ... }` block.
        let html = render_browser_with_style(TABLE_MD, "table:\n  align: center\n");
        assert!(html.contains("margin-left:auto") || html.contains("margin: 0 auto"));
        assert!(!html.contains(".darkmatter-page table {"), "no bespoke component CSS block");
    }
```

- [ ] **Step 2: Run to verify failure**

Run: `cargo test -p darkmatter browser_per_component_css`
Expected: FAIL — `build_component_css` still emits `.darkmatter-page table {…}`.

- [ ] **Step 3: Delete the bespoke browser component CSS**

In `page.rs` `render_to_browser`: stop calling `build_component_css`; the per-component nodes now carry `Layout`/`Style` lowered by the renderable browser fold. Delete `build_component_css`, `component_selectors`, `emit_component_css_rules`, `emit_component_color_rules`, and `resolve_width_unit_for_browser`. Keep the `.darkmatter-page` wrapper `<div>` (page frame, Phase 5).

- [ ] **Step 4: Run to verify pass**

Run: `cargo test -p darkmatter browser_per_component_css`
Expected: PASS.

- [ ] **Step 5: Commit**

```bash
git add darkmatter/lib/src/layout/page.rs
git commit -m "feat(darkmatter): delete build_component_css; per-component browser CSS via the fold"
```

---

## Phase 5: Slim, renderable-typed page frame

**Files:**
- Modify: `darkmatter/lib/src/layout/page.rs` (page-frame fields + `apply_row_decoration` + wrapper)
- Modify: `darkmatter/lib/src/layout/context.rs` (page-frame residue only)

- [ ] **Step 1: Write the failing tests**

```rust
    #[test]
    fn page_frame_stores_renderable_types() {
        let page = DarkmatterPage::new(&term()).with_margin(2).with_padding(3);
        // page-frame margin/padding are renderable Edges, not PageMargin/PagePadding
        let _: &renderable::layout::Edges = page.page_margin();   // accessor returns Edges
        let _: &renderable::layout::Edges = page.page_padding();
    }

    #[test]
    fn pronounced_still_flips_render_mode() {
        let page = DarkmatterPage::new(&term()).with_page_background(PageBackground::Pronounced);
        // existing guard: the code theme mode inverts; reuse the existing snapshot
        let html = page.render_to_browser(&"```rust\nfn x(){}\n```".into()).unwrap();
        assert!(html.contains("darkmatter-page"));
        insta::assert_snapshot!("pronounced_background_snapshot", html);
    }
```

- [ ] **Step 2: Run to verify failure**

Run: `cargo test -p darkmatter page_frame_stores_renderable`
Expected: FAIL — page-frame fields still `PageMargin`/`PagePadding`.

- [ ] **Step 3: Convert the page frame to renderable types**

In `page.rs`: change `DarkmatterPage`'s page-frame fields to `Edges` (margin/padding), `Option<TargetValue<Length>>` (max-width), and a `Background`/`PageBackground` knob; the granular builder setters (`with_margin_left`, …) now write `Edges` cells. `apply_row_decoration` (terminal) and the wrapper `<div>` styles read these renderable values (resolve to cells/CSS directly). `rebuild_layout` exports the page-frame `Layout` from these fields.

In `context.rs`: `LayoutContext` keeps only page-frame residue — `effective_width`, `background_color`, `render_color_mode` (the `PageBackground::Pronounced` flip), `terminal_width`. Drop everything per-component (already removed in Phase 3). Update `from_page` to take only page-frame inputs.

- [ ] **Step 4: Run to verify pass**

Run: `cargo test -p darkmatter page_frame pronounced`
Expected: PASS (the `pronounced_background_snapshot` guard holds).

- [ ] **Step 5: Commit**

```bash
git add darkmatter/lib/src/layout/page.rs darkmatter/lib/src/layout/context.rs
git commit -m "feat(darkmatter): slim page frame on renderable types; LayoutContext = page-frame residue"
```

---

## Phase 6: Delete the deprecated vocabulary + every `#![allow(deprecated)]`

**Files:**
- Modify: `darkmatter/lib/src/layout/types.rs`, `mod.rs`, `page.rs`, `context.rs`, `cli.rs`, `darkmatter/lib/tests/{layout_snapshots,style_frontmatter}.rs`

- [ ] **Step 1: Delete the types + bridges**

In `types.rs`, delete `PageMargin`, `PagePadding`, `PageAlignment`, `PageFill`, `WidthUnit`, the `PageComponent::Lists` variant (and `LISTS`/`ALL` references to it), and the `From`/`TryFrom` bridge impls + their tests. Keep `PageComponent` (sans `Lists`), `PageBackground`, `StyleColor`.

- [ ] **Step 2: Remove the allows and fix what they hid**

Delete every `#![allow(deprecated)]` / `#[allow(deprecated)]` in `layout/{mod,page,context,types}.rs`, `cli.rs`, and the two test files. Then:

Run: `cargo build -p darkmatter 2>&1 | rg "deprecated|cannot find|E0" | head -40`
Fix each: any remaining reference to a deleted type is dead code from the old path — remove it. `cli.rs` flag handling that built `PageFill`/`PageAlignment` now builds `ComponentPolicy`/page-frame `Edges` directly.

Run: `cargo build -p darkmatter`
Expected: clean, no deprecation warnings for these types.

- [ ] **Step 3: Acceptance greps**

Run: `rg -n 'PageMargin|PagePadding|PageAlignment|PageFill|WidthUnit|PageComponent::Lists' darkmatter/ --type rust`
Expected: nothing.

Run: `rg -n 'build_component_css|resolve_component_width|component_side_padding|map_alignment|lower_length_to_fill' darkmatter/ --type rust`
Expected: nothing.

Run: `rg -n 'allow\(deprecated\)' darkmatter/lib/src darkmatter/lib/tests`
Expected: nothing (or only allows re-justified by an unrelated deprecation, each commented).

- [ ] **Step 4: Run the suites + re-baseline intended diffs**

Run: `cargo test -p darkmatter`
Then `cargo insta review`: accept the `cutover_reference` and `layout_snapshots` diffs that are deliberate fold-vs-old-math artifacts (document the rationale in the commit); investigate any unexpected diff as a regression. The `style:` suite (AC5) must pass with the **same frontmatter input** and unchanged `--strict-style` surface.

- [ ] **Step 5: Commit**

```bash
git add -A
git commit -m "refactor(darkmatter): delete deprecated Page* vocabulary and all #![allow(deprecated)]"
```

---

## Phase 7: Docs + final verification

**Files:**
- Modify: `darkmatter/lib/src/layout/mod.rs` (the "Migration deferral (Spec A)" section), `.claude/skills/darkmatter/*` (deferral note), `darkmatter/docs/rendering/style.md`

- [x] **Step 1: Update docs**

- `layout/mod.rs`: rewrite the "Migration deferral (Spec A)" module doc — the deferral is **done**; describe the direct `style:` → `Layout`/`Style` lowering and the slim page frame.
- darkmatter skill: replace the "Migration deferral" note; state `Page*` are gone and `style:` lowers to attrs.
- `style.md`: update fill/alignment representation to the CSS box model; remove `PageFill`/`PageAlignment` references.

Run: `rg -n 'PageFill|PageAlignment|Migration deferral|deprecated layout' darkmatter/docs .claude/skills/darkmatter darkmatter/lib/src/layout/mod.rs`
Expected: no stale claims (the deferral note now reads as done).

- [x] **Step 2: Regenerate skill hashes**

`md hash` each edited darkmatter skill file; update its `hash:` frontmatter.

- [x] **Step 3: Whole-crate verification**

Run: `cargo build -p darkmatter -p biscuit-terminal -p renderable && cargo test -p darkmatter`
Expected: clean build, suites green (with the Phase 6 re-baselined references).

Run the tree-attrs perf gate to confirm darkmatter's fold still does zero renderable-owned hint round-trips:
Run: `cargo test -p renderable fold_does_zero`
Expected: PASS.

- [x] **Step 4: Commit**

```bash
git add darkmatter/lib/src/layout/mod.rs .claude/skills/darkmatter darkmatter/docs
git commit -m "docs(darkmatter): mark Migration deferral done; document direct style: lowering"
```

---

## Self-Review Notes (for the executor)

- **Spec AC coverage:** AC1 (Phase 6 grep), AC2 (Phase 6), AC3 (Phase 4 + Phase 3 + Phase 6 grep), AC4 (Phase 2), AC5 (Phase 6 step 4 — same input, unchanged strict-style), AC6 (Phases 1/3/6 reference snapshots), AC7 (Phase 5 page frame + pronounced guard), AC8 (Phase 7).
- **Order is load-bearing:** build the new path (Phases 2–5) and prove parity before deleting the old types and allows (Phase 6). The reference snapshots (Phase 1) make every intended diff visible.
- **Parity is a reference:** the `LayoutContext` math moving into the fold will shift some cells/CSS; re-baseline those deliberately (Phase 6 step 4), don't force byte equality.
- **Keep:** `DarkmatterPage`, `PageComponent` (minus `Lists`), `PageBackground`, `StyleColor`, and the sub-spec-#7 bespoke knobs (`page.stylesheet`/`meta`/`code.theme`, hyperlink/image local-style) — none are deprecated layout types.

---

## Review 1 Resolutions (2026-06-06)

Addresses [`review-1.md`](./review-1.md). Decisions taken with the user: **darkmatter
opacity channel** (not extending the renderable IR) and **retain `Length`
per-target** on the page frame.

- **Finding 1 — component color opacity lost + retired side-channel remained.**
  - `ComponentPolicy` now carries `color` / `bg_color` as `StyleColor` (opacity
    preserved) — the single source of truth. Replaced the `style: Option<Style>`
    field; deleted the duplicate `component_colors` / `component_bg_colors` maps
    from `DarkmatterPage` and `LayoutContext` (plus their accessors and
    `from_page` params).
  - `decorate.rs` records a `darkmatter.style` `rgba(...)` hint for any
    opacity-bearing component color; the browser entry point
    (`render_tree_html_with_layout`) splices it into the rendered element's inline
    `style` (CSS source order wins over the fold's opaque `rgb(...)`). Terminal
    still drops opacity (documented). Verified in a real browser
    (`browser_render::browser_component_blockquote_bg_opacity_computes_rgba` →
    `rgba(255, 0, 0, 0.5)`).
- **Finding 2 — browser component layout/style only source-tested.** Added
  `ChromeHarness` computed-style tests in `browser_render.rs`: blockquote bg
  opacity (rgba), table fg color (rgb), percentage page `max-width` resolved by
  the browser, centered-table equal auto margins, and list `left-margin` px.
- **Finding 3 — page frame stored renderable types but collapsed to cells.** The
  frame now retains the authored `Length`; the terminal resolves it
  (`length_to_cells`, percent base = terminal/content width) and the browser
  wrapper emits the native unit (`length_to_css_frame` → `%` / `ch`). `apply.rs`
  stores `Length` instead of pre-resolving to `u16`; `max-width = 0` validation
  is preserved at apply time.

Status: `darkmatter` lib (3495) + integration (`layout_snapshots`,
`style_frontmatter`, `cutover_reference`, `browser_render`) and `darkmatter-cli`
suites green; clippy clean. `docs/rendering/style.md` updated (Length retention,
`ComponentPolicy` single-source-of-truth color storage).

## Review 3 Resolutions (2026-06-06)

Addresses [`review-3.md`](./review-3.md), the single High finding: the
browser-tier percentage test checked the `max-width` *declaration*, not used
geometry.

- **Finding 1 — percentage component test verified serialization, not the used
  width.** Chrome reports `getComputedStyle(table).max-width` verbatim as `50%`,
  so the old assertion only proved the percent round tripped. Two design
  realities make a *binding* percentage `max-width` unobservable on a table:
  the component lowers to `white-space: nowrap` (so min-content == max-content
  and `max-width` can never shrink it), and supplying an explicit `width`
  alongside `max-width` is rejected as a `ComponentWidthConflict`. So the test
  now uses a percentage **`width`** — exactly what review-2 named ("a
  percentage-width table … asserting its used width relative to its containing
  block") — which the browser resolves to a px used value.
  - `browser_render::browser_component_table_width_percent_resolves_against_container`
    renders `table: width: 50%` at two different containing-block sizes (page
    `max-width` `40ch` then `80ch`) and asserts the table's used pixel width is
    ~50% of `.darkmatter-page` in both. Holding at 50% across two containers
    proves the authored `Length` was carried onto the node and resolved live
    against the containing block, not pre-resolved to a fixed cell count. The
    used-geometry read is centralized in a `used_table_ratio` helper.

Status: `BISCUIT_BROWSER_REQUIRED=1 cargo test -p darkmatter --test
browser_render` — 14 passed.

## Review 4 Resolutions (2026-06-06)

Addresses [`review-4.md`](./review-4.md), the single High finding: the browser
page frame no longer centered when `max-width` constrained the wrapper.

- **Finding 1 — browser page `max-width` did not center the page frame.**
  `wrap_browser_html` emitted the authored page margins verbatim, so the
  default (zero) side margins left a `max-width`-capped `.darkmatter-page`
  left-aligned in the viewport. The spec retains page-frame max-width
  centering, so the wrapper now emits `auto` left/right margins when both side
  margins are at their default. Explicitly authored side margins
  (`with_margin`/`with_margin_x`/`page.margin`) are emitted verbatim and
  suppress auto-centering, mirroring the terminal frame's left/right margin
  placement so the two targets agree (`darkmatter/lib/src/layout/page.rs`,
  `wrap_browser_html`).
  - **Browser-tier used geometry.**
    `browser_render::browser_page_max_width_centers_frame` renders
    `page: max-width: 40ch` with default margins and asserts the wrapper's
    computed `margin-left` == `margin-right` (equal, non-zero auto-resolved px
    offsets from the viewport edges) and a positive used `max-width` px — the
    centering the review flagged as having only declaration-level coverage.
  - **L1 guards.** `browser_render_with_max_width` now asserts the
    `margin: 0ch auto 0ch auto` centering string;
    `browser_render_authored_side_margins_suppress_centering` proves authored
    side margins are preserved and emit no `auto`.
  - The `pronounced_background_snapshot` insta snapshot was updated for the
    default-margin page frame's new `margin: 0ch auto 0ch auto` (centering)
    line; only that margin declaration changed.

Status: `BISCUIT_BROWSER_REQUIRED=1 cargo test -p darkmatter --test
browser_render` — 15 passed; `cargo test -p darkmatter --lib` — all passed.

## Review 5 Resolutions (2026-06-06)

Addresses [`review-5.md`](./review-5.md), the single Medium finding:
authoritative docs described a `ComponentPolicy.style` field that does not
exist, hiding the implemented `StyleColor`-retention / opacity-hint path.

- **Finding 1 — docs named a non-existent `ComponentPolicy.style` field.** The
  implementation stores component colors as `ComponentPolicy.color` /
  `ComponentPolicy.bg_color`, kept as `StyleColor` (not lowered into
  `renderable::style::Style`) so Tailwind/hex **opacity** survives to the HTML
  target; the `decorate` pass projects the `Color` onto each node's `Style` and
  records opacity separately as a `darkmatter.style` render hint the browser
  lowers to `rgba(...)` (`darkmatter/lib/src/layout/page.rs:31-44`,
  `darkmatter/lib/src/markdown/render_tree/decorate.rs:151-202`). The
  `page.rs` docstring already described this accurately; three other locations
  contradicted it and were corrected:
  - `darkmatter/lib/src/layout/mod.rs` — the `ComponentPolicy` map is now
    described as `layout` plus optional `color` / `bg_color`, and the `style:`
    lowering table maps `color`/`bg-color` to `ComponentPolicy.color` /
    `ComponentPolicy.bg_color` (retained as `StyleColor`), with a note that the
    `decorate` pass projects layout and color onto each node's `Style` and the
    opacity rides the browser-only `darkmatter.style` hint.
  - `renderable/docs/layout-and-style.md` (§7) — replaced the
    "lowers straight into `renderable::style::Style` via `ComponentPolicy`"
    claim with the `StyleColor`-retention + `decorate`-projection +
    `darkmatter.style` opacity-hint description.
  - `.claude/skills/darkmatter/SKILL.md` — same correction; `hash:` frontmatter
    regenerated with `md hash`.

Status: docs-only change. `cargo doc -p darkmatter --no-deps --lib` builds; the
new intra-doc links (`crate::style::StyleColor`, `renderable::style::Style`)
resolve. The two pre-existing `-D warnings` doc errors live in
`markdown/mod.rs` and `schemas/mod.rs`, not in any file touched here.
