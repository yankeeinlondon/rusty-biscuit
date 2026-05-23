---
ready: false
---

# TwoColumn Implementation Review

## Executive Summary

The `TwoColumn` component has been successfully migrated to the canonical render-tree architecture. All nine acceptance criteria from the specification are functionally satisfied: `TreeRenderable` is implemented with a shared private projection helper, the terminal `render()` path routes through the tree with a bespoke fallback for image-overlay scenarios, `BrowserRenderable` and `MarkdownRenderable` delegate to the tree renderers, the `bt columns` CLI exposes `--html`, `--md`, and `--md-plus`, and both approved render-tree work items (RT-TWOCOLUMN-001 and RT-TWOCOLUMN-002) are implemented and tested.

The implementation follows the established Stage-2 migration recipe exactly: one private projection helper (`to_render_node`), `render_via_tree` with `tracing::error!` + bespoke fallback, `#[doc(hidden)] pub fn render_bespoke()` retained for parity, direct cross-target trait impls, and CLI flags with `conflicts_with_all`.

However, one test-coverage gap prevents a clean "production ready" verdict: the Prose-styling guard test is missing at the byte level. A future regression that silently flattens Prose emphasis inside a column would pass every existing test. This gap is flagged as **high severity** because it was explicitly called out in `lessons-learned.md` as the exact regression mode that the `BlockQuote` guard test was added to prevent.

---

## Specification Compliance

| # | Criterion | Status | Notes |
|---|-----------|--------|-------|
| 1 | `TreeRenderable` implemented; `TerminalRenderable::render_tree_node()` delegates to same helper | ✅ | `to_render_node()` is the single source of truth. Parity test 25 (`tree_renderable_matches_terminal_render_tree_node`) serializes both paths and asserts equality. |
| 2 | `BrowserRenderable` via tree renderer | ✅ | `render_html_fragment()` delegates to `render_browser_node`. RT-TWOCOLUMN-001 is implemented in `renderable/src/tree/render/browser.rs`. |
| 3 | `MarkdownRenderable` with `render_markdown()` and `render_markdown_plus()` | ✅ | `render_markdown()` uses `MarkdownDialect::Markdown`; `render_markdown_plus()` uses `MarkdownDialect::MarkdownPlus`. RT-TWOCOLUMN-002 is implemented in `renderable/src/tree/render/markdown.rs`. |
| 4 | Bespoke `TerminalRenderable` flipped to tree; old path retained as fallback | ✅ | `render()` → `render_via_tree()` → `render_terminal_node()`. Falls back to `render_bespoke()` on `Unsupported` or `Err`. |
| 5 | `bt columns` CLI updated with `--html`, `--md`, `--md-plus` | ✅ | All three flags present with `conflicts_with_all`. Integration tests verify mutual exclusion and cross-target rendering. |
| 6 | Parity tests compare bespoke-vs-tree terminal output | ✅ | `two_column_parity.rs` covers variants 1–12 and 15–17 from the spec. |
| 7 | Cross-target tests cover Browser HTML and Markdown/MarkdownPlus output | ✅ | Browser CSS lowering, Markdown sequential fallback, and MarkdownPlus flex HTML are all tested. |
| 8 | Component table updated | ✅ | `renderable/docs/components.md` shows TwoColumn as `both avail, tree renders` with Browser ✅ and Markdown ✅. |
| 9 | RT-TWOCOLUMN-001 and RT-TWOCOLUMN-002 completed | ✅ | Browser flex CSS and MarkdownPlus flex HTML are both implemented and tested at the renderer level. |

---

## Implementation Quality

### What is well done

- **Single-source projection.** `to_render_node()` is the only place that builds the `BlockQuote` carrier with `ColumnsHints`. Both `TreeRenderable::render_tree()` and `TerminalRenderable::render_tree_node()` delegate to it, preventing drift.
- **Correct image-overlay fallback.** `render_via_tree` checks `matches!(node.kind, NodeKind::Unsupported { .. })` *before* calling `render_terminal_node`, so a `TerminalImage` column never surfaces the unsupported placeholder in-band. This matches the pattern established in `lessons-learned.md`.
- **Uniform error handling.** All three target impls (`TerminalRenderable`, `BrowserRenderable`, `MarkdownRenderable`) use the same policy: log via `tracing::error!` and return empty output / empty fragment. No in-band sentinels.
- **Shared CSS helpers.** `columns_container_css`, `left_column_css`, and `right_column_css` live in `renderable/src/tree/render/shared.rs` and are consumed by both the Browser and MarkdownPlus renderers. This prevents the two targets from drifting.
- **Layout merge without overwrite.** The browser `render_columns` path merges the literal `columns` class and the flex CSS with any user-supplied classes and `Layout`-derived CSS, so `node_attrs.set_layout()` is not shadowed.
- **CLI follows the established pattern.** `ColumnsArgs` uses `conflicts_with_all`, `--example` composes with each target, and the terminal branch threads through `emit_vertical_margins` correctly.

### Minor code notes

- `ColumnWidth::Percent` is clamped three times: once in `with_left_percent`, once in `with_left_width`, and once in `render_columns`. This is harmless but slightly redundant.
- `render_column_block` and `render_overlay_with_cursor_reset` are bespoke-only helpers that remain `private` and are untouched by the migration — correct.

---

## Test Coverage Assessment

### Level 1 (in-process / unit)

**In-source unit tests** (`biscuit-terminal/lib/src/components/two_column.rs`):
- `renders_side_by_side_balanced` — basic layout
- `respects_custom_ratio_and_height_padding` — multi-line, ratio
- `stacks_when_not_enough_space` — narrow terminal
- `is_block_level_component`
- `two_column_render_tree_node_carries_layout_when_margins_set` — layout attrs

These are adequate for the component's happy path, but they exercise `render_optimistic` (now tree-routed) without comparing against `render_bespoke_optimistic`.

**Parity tests** (`biscuit-terminal/lib/tests/two_column_parity.rs` — 867 lines):
- Structural tests (variants 18–26): root kind, `left_count`, gap, left width, layout presence, default layout omission, round-trip validation, canonical trait parity.
- Semantic parity (variants 1–12, 15–17): side-by-side, custom ratio, fixed width, custom gap, stacked narrow, alignment center, left/right margins, nested block content (relaxed), empty columns, unicode, Prose content.
- Image fallback (variants 13–14): left-image and right-image both fall back to bespoke, no unsupported placeholder.
- Cross-target: Browser HTML, Markdown sequential, MarkdownPlus flex HTML, empty columns, fixed/percent/gap in MarkdownPlus.
- Strictness: terminal Warn/Strict for unsupported image; MarkdownPlus Strict for unsupported image.

**Renderer-level tests** (`renderable/src/tree/render/browser.rs` and `markdown.rs`):
- RT-TWOCOLUMN-001: default columns, fixed width, percent width (with clamping), custom gap, layout + column CSS coexistence, empty columns, user classes preserved, plain block quote unchanged.
- RT-TWOCOLUMN-002: portable Markdown sequential, MarkdownPlus flex container, fixed width, percent width, empty columns, emphasis/prose content, nested block content, image column under Warn/Strict, multiple blocks per column (parser constraint documented), plain block quote unchanged.

**Layout matrix** (`biscuit-terminal/lib/tests/layout_matrix.rs`):
- 12 snapshots for TwoColumn covering baseline, alignment, margins, max-width, word-wrap, and width variants. The `render_comparison` drift ledger is empty for TwoColumn.

**CLI integration tests** (`biscuit-terminal/cli/tests/integration_test.rs`):
- `test_columns_help`
- `test_columns_snapshot` (insta snapshot)
- `test_columns_html_emits_columns_flex_container`
- `test_columns_md_collapses_to_sequential_blocks`
- `test_columns_md_plus_emits_flex_html_container`
- Mutual-exclusion tests for `--md`, `--md-plus`, `--html`
- `--example` combined with each target
- `test_every_subcommand_help_exposes_example_flag` includes `"columns"`

### Level 2 (real-terminal)

**`biscuit-terminal/cli/tests/level2_layout.rs`**:
- `level2_columns_long_content_wraps_within_columns` — wrapping + side-by-side
- `level2_columns_left_margin_shifts_block` — margin indentation
- `level2_columns_gap_separates_columns` — gutter width

These exercise the actual terminal display path for the tree-routed `bt columns` command.

---

## Findings

### High severity

#### 1. Missing byte-level Prose SGR guard test

**Location:** `biscuit-terminal/lib/tests/two_column_parity.rs`, spec variant 11  
**Issue:** `prose_content_in_columns_survives_tree_rendering` asserts token presence (`tree.contains("bold")`) but does **not** assert that bold SGR bytes (`\x1b[1m`) survive the tree path. If the `Prose` downcast in `project_renderable_content` broke and fell back to ANSI-stripped plain text, both the bespoke and tree paths would emit the same words — the semantic parity test would stay green while styling silently disappeared.

**Why this matters:** `lessons-learned.md` records this exact regression mode for `BlockQuote`:

> "The semantic parity-token comparison in `render_tree_component_parity.rs` is now too coarse to catch this regression on its own — both paths emit the same *words* — which is why an explicit byte-level positive pin matters."

The fix is a guard test equivalent to `test_prose_bold_inline_styling_survives_terminal_tree_render` for BlockQuote:

```rust
let left = Prose::new("**bold** left");
let right = Prose::new("plain right");
let cols = TwoColumn::new(left, right);
let term = test_terminal(80);
let out = cols.render(&term);
assert!(out.contains("\x1b[1m"), "bold SGR must survive tree path: {out:?}");
```

This should be added before the component is marked production-ready.

### Medium severity

#### 2. No `render_optimistic` vs `render_bespoke_optimistic` parity test

**Location:** `biscuit-terminal/lib/tests/two_column_parity.rs`  
**Issue:** The spec says the public `render()` and `render_optimistic()` paths both route through the tree. The parity suite compares `render_bespoke()` against `render()` but never compares `render_bespoke_optimistic()` against `render_optimistic()`. The layout matrix exercises `render_optimistic` indirectly, but a dedicated parity test would make the optimistic-path contract explicit.

#### 3. Missing component-level MarkdownPlus emphasis test

**Location:** `biscuit-terminal/lib/tests/two_column_parity.rs`  
**Issue:** The spec's MarkdownPlus test variant 5 ("Prose content") is covered at the renderer level (`columns_markdown_plus_emphasis_and_prose_content` in `markdown.rs`), but `two_column_parity.rs` has no component-level assertion that `TwoColumn::render_markdown_plus()` preserves `**bold**` or `_italic_` inside columns. Low severity because the renderer test is authoritative, but a component-level pin would close the loop.

### Low severity

#### 4. Stale doc comment in Level-2 test file

**Location:** `biscuit-terminal/cli/tests/level2_layout.rs`, line 8  
**Issue:** The module doc comment states:

> "`bt columns` renders through the bespoke `TwoColumn` renderer."

This is incorrect after the IR flip. `bt columns` now routes through the canonical tree renderer (`TwoColumn::render()` calls `render_via_tree()`). The comment should be updated to say the tree renderer is exercised, and that the `render_comparison` suite proves the tree path is byte-equivalent to the legacy bespoke path.

#### 5. `render_html_page` is implemented but has no direct test

**Location:** `biscuit-terminal/lib/src/components/two_column.rs`, line 757  
**Issue:** `BrowserRenderable::render_html_page` delegates to `render_html_fragment`, so the risk is low. A minimal test that asserts the returned `HtmlPage` contains the fragment content would remove the untested-surface gap.

#### 6. `render_via_tree_optimistic` error fallback is untested

**Location:** `biscuit-terminal/lib/src/components/two_column.rs`, line 530  
**Issue:** The `Err` arm that falls back to `render_bespoke_optimistic` has no test forcing a tree-render failure. Hard to trigger in practice (it requires `render_terminal_node` to fail on a structurally valid non-image tree), but the path is present and unexercised.

---

## Ergonomic and Performance Observations

- **Ergonomics:** The `TwoColumn` builder API (`new`, `with_left_percent`, `with_left_width`, `with_gap`) is unchanged, so existing callers require no migration. The cross-target CLI flags (`--html`, `--md`, `--md-plus`) follow the exact same shape as the other 11 flipped components, which is good for user consistency.
- **Performance:** The tree projection allocates a `Vec<RenderNode>` for each column, then a merged `Vec` for the carrier. For typical two-column usage (short text, single paragraph per side) this is negligible. No unnecessary clones were introduced compared to the bespoke path. The `render_via_tree` path does pay the tree-validation cost, but that is the accepted cost of the render-tree architecture and is already amortized across all flipped components.
- **Memory:** The `project_column` helper uses `std::mem::take` to drain the inline-run buffer into paragraphs. This is efficient and follows the pattern used in `BlockQuote` and `Compose`.

---

## Production Readiness

**Judgment: Not production ready.**

The implementation is architecturally sound, functionally complete, and strongly tested across Level 1 and Level 2. All nine spec acceptance criteria are met. RT-TWOCOLUMN-001 and RT-TWOCOLUMN-002 are fully implemented. The `render_comparison` drift ledger is clean.

However, the **missing byte-level Prose SGR guard test** (Finding 1) is a user-observable test gap that the project's own `lessons-learned.md` explicitly flags as a regression vector. The current parity test for Prose content (`prose_content_in_columns_survives_tree_rendering`) is token-level only; a future change that breaks the `Prose` downcast in `project_renderable_content` would flatten styled text to plain text and still pass every existing test. This is the exact failure mode the BlockQuote migration added `test_prose_bold_inline_styling_survives_terminal_tree_render` to prevent.

Per the `test-rigor.md` standard:

> "A feature MAY be marked production-ready only when each user-observable requirement has at minimum the level of verification appropriate for it."

"Styled text content preserved in both paths" (spec variant 11) is a user-observable requirement. The appropriate verification is a positive byte-level assertion that SGR bytes survive the tree path, not merely that the words survive. Until that guard test is added, the component does not meet the production-readiness bar.

**Recommended fix:** Add one test to `two_column_parity.rs`:

```rust
#[test]
fn prose_bold_inline_styling_survives_terminal_tree_render() {
    let left = Prose::new("**bold** left");
    let right = Prose::new("plain right");
    let cols = TwoColumn::new(left, right);
    let term = test_terminal(80);
    let out = cols.render(&term);
    assert!(out.contains("\x1b[1m"), "bold SGR must survive tree path: {out:?}");
    assert!(out.contains("\x1b[22m"), "bold reset must survive tree path: {out:?}");
}
```

Once this test is in place, the component can be marked production ready with no further changes required.
