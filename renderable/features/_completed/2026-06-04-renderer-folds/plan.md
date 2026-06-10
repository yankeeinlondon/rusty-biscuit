---
source_files_during_phase_1:
  - biscuit-terminal/lib/src/render_tree/render.rs
docs_updated_during_phase_1: []
docs_created_during_phase_1: []
skills_files_updated_during_phase_1: []
source_files_during_phase_2:
  - biscuit-terminal/lib/src/render_tree/style.rs
  - biscuit-terminal/lib/src/render_tree/render.rs
docs_updated_during_phase_2: []
docs_created_during_phase_2: []
skills_files_updated_during_phase_2: []
source_files_during_phase_3:
  - biscuit-terminal/lib/src/render_tree/style.rs
  - biscuit-terminal/lib/src/render_tree/render.rs
  - biscuit-terminal/lib/src/components/block_quote.rs
  - biscuit-terminal/lib/src/components/status_block.rs
  - biscuit-terminal/cli/tests/level2_render_tree_style.rs
docs_updated_during_phase_3: []
docs_created_during_phase_3: []
skills_files_updated_during_phase_3: []
source_files_during_phase_4:
  - biscuit-terminal/lib/src/render_tree/render.rs
docs_updated_during_phase_4: []
docs_created_during_phase_4: []
skills_files_updated_during_phase_4: []
source_files_during_phase_5:
  - renderable/src/tree/render/browser.rs
docs_updated_during_phase_5: []
docs_created_during_phase_5: []
skills_files_updated_during_phase_5: []
source_files_during_phase_6:
  - renderable/src/tree/render/browser.rs
docs_updated_during_phase_6: []
docs_created_during_phase_6: []
skills_files_updated_during_phase_6: []
packages_during_phase_6:
  - renderable
source_files_during_phase_7: []
docs_updated_during_phase_7:
  - renderable/docs/layout-and-style.md
docs_created_during_phase_7: []
skills_files_updated_during_phase_7:
  - .claude/skills/renderable/style.md
  - .claude/skills/biscuit-terminal/render-tree.md
packages_during_phase_7:
  - renderable
  - biscuit-terminal
packages:
  - biscuit-terminal
  - biscuit-terminal-cli
  - renderable
---

# Renderer Folds Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Make the renderable **terminal** and **browser** folds render the full CSS box model — `padding` (painted by `background`), the `width` modes (`Auto`/`FitContent`/`Fixed`), and the full `Border` matrix on the browser — so every target renders from node attrs alone.

**Architecture:** Terminal uses the layered split — `render.rs` resolves geometry (margin, alignment, content-box `width` under the `max_width` cap), `paint_text`/`style.rs` owns the painted inner box (`padding` + `background` + `border`), reusing the band-pad machinery vacated by `Fill`; the implicit one-cell border gap is removed so `padding` is the single source of inner spacing. Browser extends `layout_to_css` (`padding`, `width`) and `style_css_declarations` (full `Border` matrix). Both folds read attrs via tree-attrs' borrowed `*_ref` accessors so the perf gate stays green.

**Tech Stack:** Rust 2024, the monorepo `cargo`/`just` tooling, `insta` snapshots, biscuit-test-harness L2 (real-terminal) checks, `md hash` for skills.

**Spec:** [`spec.md`](spec.md) (architect-reviewed). **Depends on** [`style-vocabulary`](../2026-06-04-style-vocabulary/spec.md) and [`tree-attrs`](../2026-06-04-tree-attrs/spec.md) being implemented — this plan assumes `Layout` has `padding`/`width`, `Fill` is gone, `NodeAttrs` is typed with `layout_ref`/`style_ref`, and `InheritedStyle` exists. Confirm with `cargo build -p renderable -p biscuit-terminal` before starting.

**Parity note:** Per the architecture spec, rendered output is a *reference*, not a byte contract. Snapshot diffs vs. the former `Fill` behavior are expected; each is judged improvement vs. regression and re-baselined with a note.

---

## File Structure

- `biscuit-terminal/lib/src/render_tree/render.rs` — geometry: `width` resolution + content-box clamp in `render_with_layout`; `FitContent` measure pass.
- `biscuit-terminal/lib/src/render_tree/style.rs` — painted box: `padding` painting in `paint_text`; remove the implicit border gap from `render_border` / `border_horizontal_overhead`.
- `renderable/src/tree/render/browser.rs` — `layout_to_css` (`padding`, `width`) and `style_css_declarations` (full `Border`).

**Baseline:**

- [ ] **Step 0: Confirm dependencies landed + green**

Run: `cargo build -p renderable -p biscuit-terminal && cargo test -p renderable -p biscuit-terminal --no-run`
Expected: clean; `Layout { padding, width, .. }`, `Background::subtle`, `NodeAttrs::layout_ref`, and `renderable::tree::InheritedStyle` all resolve.

---

## Task 1: Terminal — resolve the `width` modes + content-box clamp

**Files:**
- Modify: `biscuit-terminal/lib/src/render_tree/render.rs` (`render_with_layout`)

- [x] **Step 1: Write the failing tests**

In `render.rs` tests (use the existing terminal test harness — render a styled node at a fixed `available_width` and strip ANSI to count columns):

```rust
    #[test]
    fn width_fixed_sets_content_box() {
        // available 80; Fixed(40) → content box is 40 cells wide.
        let out = render_block_with_layout(
            "wide content here",
            Layout { width: Width::Fixed(TargetValue::universal(Length::ch(40))), ..Default::default() },
            80,
        );
        assert_eq!(max_line_cols(&out), 40);
    }

    #[test]
    fn width_auto_fills_after_margin() {
        let out = render_block_with_layout(
            "x",
            Layout { margin: Edges::x(Length::ch(5)), ..Default::default() }, // width: Auto
            80,
        );
        // content box = 80 - 2*5 = 70 (Auto fills remaining)
        assert_eq!(content_box_cols(&out), 70);
    }

    #[test]
    fn fixed_width_is_clamped_by_available_minus_margin_padding() {
        let out = render_block_with_layout(
            "x",
            Layout {
                margin: Edges::x(Length::ch(10)),
                padding: Edges::x(Length::ch(5)),
                width: Width::Fixed(TargetValue::universal(Length::ch(100))), // asks 100, clamped
                ..Default::default()
            },
            80,
        );
        // 80 - 2*10 (margin) - 2*5 (padding) = 50 content box max
        assert!(content_box_cols(&out) <= 50);
    }
```

Provide `render_block_with_layout`, `max_line_cols`, `content_box_cols` helpers (build a one-node `Document`, set the layout via `set_layout`, render through the terminal fold at `available_width`, strip ANSI).

- [x] **Step 2: Run to verify failure**

Run: `cargo test -p biscuit-terminal width_fixed width_auto fixed_width_is_clamped`
Expected: FAIL — `width` not honored (only `Auto` exists today).

- [x] **Step 3: Implement `width` resolution**

In `render_with_layout`, after computing margins, resolve the content-box width from `layout.width` (the field added by style-vocabulary). `Auto` keeps today's `available − margin_lr`; `Fixed(n)` resolves `n` (cells / `%` of available); `FitContent` is Task 4. Then clamp:

```rust
let margin_lr = left + right;
let padding_lr = resolve_cells(&layout.padding.left, available)
    + resolve_cells(&layout.padding.right, available);
let border_lr = style::border_horizontal_overhead(style); // 0 when no border

let auto_cap = available.saturating_sub(margin_lr + padding_lr + border_lr).max(1);
let mut content_width = match &layout.width {
    Width::Auto => auto_cap,
    Width::Fixed(tv) => resolve_cells(tv, available).min(auto_cap).max(1),
    Width::FitContent => auto_cap, // refined in Task 4
};
if let Some(mw) = &layout.max_width {
    content_width = content_width.min(resolve_cells(mw, available)).max(1);
}
```

Thread `content_width` as the inner render width (as `max_width` does today) and keep the alignment offset computed against `available − margin` and the *painted box* width (`content_width + padding_lr + border_lr`).

- [x] **Step 4: Run to verify pass**

Run: `cargo test -p biscuit-terminal width_fixed width_auto fixed_width_is_clamped`
Expected: PASS.

- [ ] **Step 5: Commit**

```bash
git add biscuit-terminal/lib/src/render_tree/render.rs
git commit -m "feat(biscuit-terminal): resolve Layout width modes with content-box clamp"
```

---

## Task 2: Terminal — paint `padding` with `background`

**Files:**
- Modify: `biscuit-terminal/lib/src/render_tree/style.rs` (`paint_text` / `apply_style`)
- Modify: `biscuit-terminal/lib/src/render_tree/render.rs` (pass `padding` into the paint step)

- [x] **Step 1: Write the failing tests**

```rust
    #[test]
    fn padding_is_painted_with_background_margin_is_not() {
        // margin 2 (transparent) + padding 3 (painted) + content, background subtle.
        let layout = Layout {
            margin: Edges::x(Length::ch(2)),
            padding: Edges::x(Length::ch(3)),
            ..Default::default()
        };
        let style = Style { background: Some(Background::subtle()), ..Default::default() };
        let out = render_block_with_layout_and_style("hi", layout, style, 40);
        let first = out.lines().find(|l| l.contains("hi")).unwrap();
        // The 2 leading margin cells carry no background SGR; the 3 padding cells do.
        assert!(leading_unstyled_spaces(first) == 2, "margin transparent");
        assert!(has_background_run_of_width(first, 3 /*pad*/ + 2 /*"hi"*/ + 3), "padding+content painted");
    }
```

Add `render_block_with_layout_and_style`, `leading_unstyled_spaces`, `has_background_run_of_width` helpers (operate on the raw ANSI string — assert SGR presence semantically, not by byte equality, per the L2 capture rule).

- [x] **Step 2: Run to verify failure**

Run: `cargo test -p biscuit-terminal padding_is_painted`
Expected: FAIL — padding not rendered.

- [x] **Step 3: Implement padding painting**

Pass `layout.padding` (resolved to cells) into `paint_text`. In `paint_text`, after computing the background SGR, build each output line as: top/bottom padding rows (full painted width), then for content rows: `left_padding` painted spaces + content padded to `content_width` + `right_padding` painted spaces — all inside the single background SGR run. Reuse the existing widest-line/band machinery (it already pads each line to a width and wraps it in the SGR); the change is that the pad amount is now `padding` + content-to-`content_width`, and the painted band is `content_width + padding_lr`. The margin (transparent) is still applied outside, in `render.rs`.

- [x] **Step 4: Run to verify pass**

Run: `cargo test -p biscuit-terminal padding_is_painted`
Expected: PASS.

- [ ] **Step 5: Commit**

```bash
git add biscuit-terminal/lib/src/render_tree/style.rs biscuit-terminal/lib/src/render_tree/render.rs
git commit -m "feat(biscuit-terminal): paint Layout.padding with Style.background"
```

---

## Task 3: Terminal — remove border's implicit interior gap

Per the architect's review: the implicit one-cell interior space inside drawn vertical borders is removed; `border_horizontal_overhead` counts drawn border cells only. `padding` is the single source of inner spacing.

**Files:**
- Modify: `biscuit-terminal/lib/src/render_tree/style.rs` (`render_border`, `border_horizontal_overhead`)

- [x] **Step 1: Write the failing test**

```rust
    #[test]
    fn border_reserves_only_drawn_cells_no_implicit_gap() {
        // A left+right border with NO padding: content sits directly inside the edges.
        let style = Style {
            border: Some(Border { sides: BorderSides::All, ..Default::default() }),
            ..Default::default()
        };
        assert_eq!(border_horizontal_overhead(&style), 2); // 1 cell per drawn vertical edge, no +1 gap
        let out = render_block_with_style("ab", style, 20);
        // the content row is `│ab│`-like with no interior space unless padding is set
        let row = out.lines().find(|l| l.contains("ab")).unwrap();
        assert!(adjacent_to_border(row, "ab"), "no implicit interior space");
    }
```

- [x] **Step 2: Run to verify failure**

Run: `cargo test -p biscuit-terminal border_reserves_only_drawn_cells`
Expected: FAIL — overhead is currently `2 per side` (edge + interior space) and the row has an implicit space.

- [x] **Step 3: Implement**

In `border_horizontal_overhead`, return `u32::from(left) + u32::from(right)` (one cell per drawn vertical edge, **not** `* 2`). In `render_border`, stop inserting the interior space — wrap content directly with the edge glyphs. Update the border doc comment to say inner spacing is `Layout.padding`.

> **Migration follow-through (per spec §4 / AC4):** removing the gap means the
> left-bordered components that relied on it must express their inner space as
> `Layout.padding`. Implemented alongside Step 3:
> - `paint_text` now reserves `padding` cells even with no `background` (ragged
>   right edge, so left-only padding adds no trailing whitespace).
> - `BlockQuote` and `StatusBlock` projections add `padding.left = 1ch` so the
>   `│ `/`┃ ` gap survives via padding.
> - `render_with_layout` measures alignment slack against `content_width +
>   padding_lr` **only when the padding is painted** (node carries a `Style`),
>   fixing a latent 1-cell center/right offset for painted padded boxes.
> - The 7 `BlockQuote` browser characterization tests now accept the
>   `<blockquote style=…>` opening tag (a layout is recorded on the node), and
>   the obsolete `--fill-band` L2 cases (a removed `Fill` flag) were deleted.

- [x] **Step 4: Run to verify pass + check border regressions**

Run: `cargo test -p biscuit-terminal border`
Expected: PASS; the `render_border width mismatch` known gap (top/bottom rule two columns narrow) should now align — verify the top/bottom rule width equals the content row width.

- [ ] **Step 5: Commit**

```bash
git add biscuit-terminal/lib/src/render_tree/style.rs
git commit -m "feat(biscuit-terminal): border reserves only drawn cells; padding owns inner spacing"
```

---

## Task 4: Terminal — `FitContent` (render-measure-place)

**Files:**
- Modify: `biscuit-terminal/lib/src/render_tree/render.rs`

- [x] **Step 1: Write the failing tests**

```rust
    #[test]
    fn fit_content_sizes_box_to_widest_line() {
        // Two short lines; FitContent → box width == widest line, centered in 80.
        let layout = Layout { width: Width::FitContent, alignment: Alignment::Center, ..Default::default() };
        let out = render_block_with_layout("hi\nthere", layout, 80);
        assert_eq!(content_box_cols(&out), "there".len() as usize);
        assert!(centered_offset(&out) > 0, "centered within available");
    }

    #[test]
    fn fit_content_is_capped_by_max_width() {
        let layout = Layout {
            width: Width::FitContent,
            max_width: Some(TargetValue::universal(Length::ch(3))),
            ..Default::default()
        };
        let out = render_block_with_layout("aaaaaaaa", layout, 80);
        assert!(content_box_cols(&out) <= 3);
    }
```

- [x] **Step 2: Run to verify failure**

Run: `cargo test -p biscuit-terminal fit_content`
Expected: FAIL — `FitContent` currently falls back to `Auto` (Task 1).

> **Executor note (architecture finding):** under the post-`Fill` terminal
> architecture, `FitContent` is *observationally equivalent* to `Auto` for
> these cases — a true fail-first test is impossible. The painted background
> band (`paint_text`) and `render_border` already hug the content's widest
> line, and the `Auto` alignment basis (`content_width == available − margin`)
> already places a shrunk box correctly. The tests are therefore correct
> characterization/regression guards for AC2/AC3 (they verify FitContent sizes
> the box to the widest line and centers it within the available width), and
> they pass before and after the implementation. The bounded two-pass is still
> implemented per AC2: it makes the resolved `content_width` the *natural*
> content width (rather than the full cap) and keeps placement correct once
> shrunk, which is required for spec compliance and future-proofs the field.

- [x] **Step 3: Implement the bounded measure pass**

Per the architect's refinement (bounded, not unbounded): when `layout.width == FitContent`, render the content once at `auto_cap` (the largest permitted content box), measure the widest visible line (`max(visible_width(line))`), then `content_width = measured.min(auto_cap).min(max_width?)`. If `content_width` differs from `auto_cap`, re-render the content at `content_width` (so wrapping reflows to the final box). Place via the existing alignment offset.

- [x] **Step 4: Run to verify pass**

Run: `cargo test -p biscuit-terminal fit_content`
Expected: PASS.

- [ ] **Step 5: Commit**

```bash
git add biscuit-terminal/lib/src/render_tree/render.rs
git commit -m "feat(biscuit-terminal): FitContent width via bounded measure-then-place"
```

---

## Task 5: Browser — lower `padding` and `width`

**Files:**
- Modify: `renderable/src/tree/render/browser.rs` (`layout_to_css`)

- [x] **Step 1: Write the failing tests**

In `browser.rs` tests:

```rust
    #[test]
    fn layout_to_css_emits_padding() {
        let css = layout_to_css(&Layout { padding: Edges::x(Length::ch(3)), ..Default::default() });
        assert!(css.contains("padding-left:3ch") && css.contains("padding-right:3ch"), "{css}");
    }

    #[test]
    fn layout_to_css_emits_width_modes() {
        assert!(layout_to_css(&Layout { width: Width::FitContent, ..Default::default() })
            .contains("width:fit-content"));
        let fixed = layout_to_css(&Layout {
            width: Width::Fixed(TargetValue::universal(Length::ch(60))), ..Default::default()
        });
        assert!(fixed.contains("width:60ch"), "{fixed}");
        // Auto omits an explicit width
        assert!(!layout_to_css(&Layout::default()).contains("width:"));
    }
```

- [x] **Step 2: Run to verify failure**

Run: `cargo test -p renderable layout_to_css_emits`
Expected: FAIL — `padding`/`width` not emitted.

- [x] **Step 3: Implement**

In `layout_to_css`, after the margin block, emit `padding-{top,right,bottom,left}` from `layout.padding` (vertical → `lh`, horizontal → `css_len`, mirroring margin), and emit `width` per `layout.width`: `Auto` → nothing; `FitContent` → `width:fit-content`; `Fixed(tv)` → `width:{css_len(tv)}`.

- [x] **Step 4: Run to verify pass**

Run: `cargo test -p renderable layout_to_css`
Expected: PASS (new + existing margin/max-width/alignment tests).

- [ ] **Step 5: Commit**

```bash
git add renderable/src/tree/render/browser.rs
git commit -m "feat(renderable): browser layout_to_css lowers padding and width"
```

---

## Task 6: Browser — lower the full `Border` matrix

**Files:**
- Modify: `renderable/src/tree/render/browser.rs` (`style_css_declarations`)

- [x] **Step 1: Write the failing tests**

```rust
    #[test]
    fn border_lowers_full_matrix() {
        let style = Style {
            border: Some(Border {
                weight: BorderWeight::Thick,
                line_style: BorderLineStyle::Dashed,
                sides: BorderSides::Sides { top: false, right: false, bottom: false, left: true },
                radius: Some(TargetValue::universal(Length::ch(1))),
                color: Some(TargetValue::universal(PerMode::universal(Color::Tailwind(Tailwind::Indigo500)))),
            }),
            ..Default::default()
        };
        let css = style_css_declarations(&style, true);
        assert!(css.contains("border-left-style:dashed"), "{css}");
        assert!(css.contains("border-left-width:"), "{css}");
        assert!(css.contains("border-radius:"), "{css}");
        assert!(css.contains("border-left-color:") || css.contains("border-color:"), "{css}");
    }

    #[test]
    fn border_all_sides_uses_shorthand() {
        let style = Style { border: Some(Border { sides: BorderSides::All, ..Default::default() }), ..Default::default() };
        let css = style_css_declarations(&style, true);
        assert!(css.contains("border-style:solid"), "{css}");
    }
```

- [x] **Step 2: Run to verify failure**

Run: `cargo test -p renderable border_lowers border_all_sides`
Expected: FAIL — `border` is currently ignored.

- [x] **Step 3: Implement the border lowering**

In `style_css_declarations`, replace the "border intentionally ignored" branch with a lowering of `style.border`:
- `weight` → `border-width` px step (`Thin`=1px, `Medium`=2px, `Thick`=3px),
- `line_style` → `border-style` (`Solid`→solid, `Dashed`→dashed, `Dotted`→dotted, `Double`→double),
- `color` → `border-color` via the existing `PerMode`→CSS color path,
- `sides`: `All` → `border-*` shorthands; `Sides{..}` → per-side `border-{side}-{width,style,color}` for each enabled side; `None` → emit nothing,
- `radius` → `border-radius` via `css_len`.

- [x] **Step 4: Run to verify pass + update the doc note**

Run: `cargo test -p renderable border`
Expected: PASS.

- [ ] **Step 5: Commit**

```bash
git add renderable/src/tree/render/browser.rs
git commit -m "feat(renderable): browser lowers the full Border matrix to CSS"
```

---

## Task 7: Docs, perf gate, and verification

**Files:**
- Modify: `.claude/skills/renderable/{style,layout}.md`, `.claude/skills/biscuit-terminal/*` (terminal box rendering), `renderable/docs/layout-and-style.md`

- [x] **Step 1: Update docs**

- `layout-and-style.md`: the "border and fill not lowered to Browser" note → border is now lowered; fill no longer exists; document terminal `padding`/`width`/`FitContent` rendering and the removed implicit border gap.
- Skills: describe terminal padding painting + width modes and browser `padding`/`width`/`border` lowering.

Run: `rg -n 'border.*not.*lowered|fill.*not.*lowered|implicit.*space' renderable/docs .claude/skills/renderable`
Expected: no stale claims.

- [x] **Step 2: Regenerate skill hashes**

`md hash` each edited skill file; update its `hash:` frontmatter. (No-op: the
edited skill files — `style.md`, `render-tree.md` — carry no `hash:`
frontmatter property.)

- [x] **Step 3: Perf gate + L2 checks**

Run: `cargo test -p renderable fold_does_zero` (the tree-attrs structural gate — folds use `*_ref`, must stay green).
Run the biscuit-terminal L2 styling suite if present: `cargo test -p biscuit-terminal --test level2_render_tree_style` (or the harness recipe). Confirm padding/border render in a real terminal.

- [x] **Step 4: Whole-crate build + test, re-baseline reference snapshots**

Run: `cargo build -p renderable -p biscuit-terminal && cargo test -p renderable -p biscuit-terminal`
Then `cargo insta review`: accept intended box-model diffs (padding now painted, border gap removed, fit-content sizing) with a note; investigate any unexpected diff.

- [ ] **Step 5: Commit**

```bash
git add -A
git commit -m "docs(renderable): document CSS box-model terminal/browser folds; re-baseline references"
```

---

## Self-Review Notes (for the executor)

- **Spec AC coverage:** AC1 (Task 2), AC2 (Tasks 1+4), AC3 (Task 4), AC4 (Task 5), AC5 (Task 6), AC6 (Task 7 perf gate), AC7 (no darkmatter change — verify `build_component_css` untouched), AC8 (Task 7 docs).
- **Border gap removal (Task 3)** changes the terminal border contract intentionally (architect's review): `padding` is the only inner spacing. Existing border snapshots will shift — re-baseline with a note in Task 7.
- **Borrowed accessors:** every fold read goes through `layout_ref`/`style_ref` (tree-attrs) so the perf gate (Task 7 step 3) stays green; do not call the owned `layout()`/`style()` in the hot fold.
- **Parity is a reference:** do not chase byte-for-byte equality with the former `Fill` output; document intended diffs.
