---
ready: false
---

# Table Component — Implementation Review

**Review date:** 2026-05-20  
**Spec:** `Table-spec.md` (renderable/features/2026-05-19-pushing-toward-ir/components/Table-spec.md)  
**Implementation:** `biscuit-terminal/lib/src/components/table/table.rs` and related files  
**Reviewer:** Kimi Code CLI

---

## Summary

The Table component has been flipped to the canonical render-tree path following the Stage 2 recipe. `TreeRenderable`, `BrowserRenderable`, and `MarkdownRenderable` are all implemented. The CLI has the required `--md`, `--md-plus`, `--html`, and `--example` switches. The FR-2 table-title hint and FR-3 Markdown cell escaping are both wired and tested.

However, **one spec-required feature is broken on the default render path**, several parity-test variants from the spec are missing, and the error-fallback paths are untested. For these reasons the component is **not production-ready** in its current state.

---

## Findings

### 1. `uniform_alignment` is silently ignored by the tree renderer — **HIGH**

**Location:** `biscuit-terminal/lib/src/render_tree/render.rs`, `emit_table` (~line 1699)

The `TableColumnHints` carry `uniform_alignment: bool` and `build_table_column` seeds it onto the reconstructed `TableColumn` (line 1639). The bespoke renderer honours it by computing `max_content_widths` per column and passing them to `pad_cell` as `width_for_alignment` (`table.rs:1080`).

The tree renderer's `emit_table` never computes or uses these widths. Every call to `pad_cell` passes `None` for `width_for_alignment`:

- Header row: `emit_table` line ~1753: `pad_cell(line, width, alignment, None)`
- Data rows: `emit_table` line ~1856/1858: `pad_cell(..., width, alignment, None)`

**Impact:** A table built with `.with_uniform_alignment(true)` on a numeric column will right-align each cell individually in the bespoke path, but will **not** align them at a consistent position in the tree path. Because `TerminalRenderable::render` now routes through the tree by default, this is a user-visible regression for any caller relying on uniform alignment.

**Fix:** `emit_table` needs to pre-compute `max_content_widths` the same way the bespoke renderer does (`Table::max_content_widths_for_plan`) and thread them into `pad_cell` when `uniform_alignment` is enabled.

**Test gap:** There is no parity test asserting uniform alignment survives the tree path. The spec lists this as parity variant #19.

---

### 2. Missing dedicated parity tests for several spec variants — **MEDIUM**

The spec enumerates 22 parity-test variants. While many are covered by `table_parity.rs` or the extensive in-source unit-test module, the following variants have **no dedicated parity test in `table_parity.rs`** (the canonical Flow-B parity file):

| Variant | Description | Where covered (if anywhere) |
|---------|-------------|----------------------------|
| #8 | Word-wrapped cells (`WrapProse` triggers wrapping) | In-source only (`test_word_wrap_respects_column_width`) |
| #10 | Fixed-width columns | In-source only (`test_fixed_width_overrides_content_width`) |
| #11 | Min/max width constraints | In-source only (`test_calculate_column_widths_*`) |
| #14 | Left/center/right block alignment via `Layout` | In-source only (`test_table_with_left_margin`, cursor-alignment tests) |
| #19 | Uniform alignment | **Not covered anywhere** |
| #20 | Vertical alignment (`Top`/`Middle`/`Bottom`) | In-source only (`test_render_content_vertical_align_*`) |

The in-source tests exercise the **bespoke** `render_content` / `render_with_cursor_positioning` paths. After the flip, the user-facing default is the tree path. A parity test in `table_parity.rs` that renders the same table through `render(&term)` and `render_bespoke(&term)` and asserts semantic equivalence is the strongest guarantee that the tree path matches the legacy behaviour.

**Fix:** Add parity tests for variants #8, #10, #11, #14, #19, and #20 to `table_parity.rs`. Each should compare `strip_ansi(table.render(&term))` against `strip_ansi(table.render_bespoke(&term))` on structural invariants (token presence, alignment positions, row heights).

---

### 3. Error fallback paths are untested — **MEDIUM**

**Locations:**
- `table.rs:1549-1562` (`render_via_tree`)
- `table.rs:1710-1721` (`render_markdown_for_dialect`)
- `table.rs:1761-1776` (`render_html_fragment`)

All three cross-target renderers follow the same pattern: log via `tracing::error!` and return an empty output on failure. This is the correct policy (better than the in-band `[render-tree error: …]` sentinel the spec originally suggested), but there is **no test that forces the error arm**.

A malformed tree (e.g. a `NodeKind::Table` whose first child is not a `TableRow`) would trigger this path. A regression that turned the empty fallback into a panic or into in-band text would not be caught.

**Fix:** Add a unit test that feeds a structurally-invalid `RenderNode` through `render_terminal_node` (or directly through `Table::render_via_tree` after monkey-patching a bad node) and asserts the result is empty and does not panic. Do the same for the Markdown and Browser fallbacks.

---

### 4. CLI `--example` does not append target flags — **LOW**

**Location:** `biscuit-terminal/cli/src/commands/table.rs:170-172`

The spec requires:
> "When `--html` is combined with `--example`, print the example with `--html` appended."

The current code always prints the constant `TABLE_EXAMPLE_CMD` regardless of which target flag is active. This is consistent with other flipped components (`prose`, `progress`, etc.) but inconsistent with the Table spec.

**Fix:** Build the example command string dynamically, appending `--html`, `--md`, or `--md-plus` when the corresponding flag is set. This is a one-line change.

---

### 5. `render_html_fragment` error fallback shape — **LOW (documented)**

The spec's sample implementation returns a `<div class="table-render-error">` on failure, but the actual code returns an empty text fragment (`BrowserFragment::new().define_as_text_fragment(String::new()).finalize()`). This is an intentional, documented divergence: the infallible `BrowserRenderable` contract should not pollute the HTML stream with error text. The empty fragment is the safer choice.

No action required; noting for completeness.

---

## What is working well

- **Single projection helper:** `Table::to_render_tree_node()` is the sole source of truth for both `TreeRenderable::render_tree()` and `TerminalRenderable::render_tree_node()`, preventing drift.
- **Cursor-positioning gate is correct:** `render` and `render_optimistic` both check `prefer_cursor_alignment && term.is_tty` before delegating to `render_bespoke`, matching the lessons-learned pattern.
- **Title hint (FR-2) is fully wired:** `set_table_title` / `table_title` accessors work; all three renderers (Terminal, Browser, Markdown) place the title correctly; empty/whitespace-only titles are ignored.
- **Markdown cell escaping (FR-3) is fully wired:** Literal pipes are escaped as `\|`, literal newlines become `<br>`, soft breaks collapse to spaces, and hard breaks become `<br>`. Comprehensive tests exist in `table_parity.rs` and the Markdown renderer's own test suite.
- **Strong Level-1 coverage:** ~160 in-source unit tests covering formatting, padding, striping, SGR-reset survival, multi-line cells, vertical alignment, conditional columns, and width planning.
- **Level-2 coverage:** `level2_render_tree_style.rs` exercises both striped tables and styled header/body slots in real terminals (WezTerm, Kitty, tmux).
- **CLI integration tests:** All target flags, mutual exclusion, title rendering, pipe escaping, and `--example` are covered in `integration_test.rs`.
- **KNOWN_DRIFT retired:** The `render_comparison.rs` ledger correctly contains no Table entries; the flip comment is present.

---

## Production Readiness

**Judgment: NOT production ready.**

The implementation is mature, well-tested, and follows the established Stage 2 recipe correctly. The only reason it is not ready is that **`uniform_alignment` is broken on the default tree render path** (Finding #1). This is a spec-required feature that silently degrades: a caller setting `.with_uniform_alignment(true)` will see different (worse) alignment in the tree-rendered output compared to the bespoke output, with no diagnostic.

Once `emit_table` is taught to compute and apply `max_content_widths` for uniform columns, and the missing parity tests (Finding #2) are added to guard against regressions, the component will be production ready. The remaining findings (#3, #4) are polish items that do not block shipping.
