---
ready: false
---

# Progress Implementation Review

> Review date: 2026-05-19
> Reviewer: Code Review Agent
> Scope: `biscuit-terminal/lib/src/components/progress.rs`, `biscuit-terminal/cli/src/commands/progress.rs`, `renderable/src/tree/render/{browser,markdown,shared}.rs`, and associated tests.

## Summary

The Progress migration to the render-tree architecture is **functionally correct and well-architected**. The single private projection helper (`to_render_node`), canonical `TreeRenderable` impl, flipped `TerminalRenderable` path, and cross-target CLI switches (`--html`, `--md`, `--md-plus`) are all present and match the spec. The browser and MarkdownPlus renderers correctly handle `ProgressHints` (RT-PROGRESS-001/002), and portable Markdown degrades cleanly to label + percentage text.

All existing tests pass (L1 unit, L1 integration, L2 real-terminal). The bespoke renderer is retained as `#[doc(hidden)] pub fn render_bespoke` for parity testing, and the `render_comparison.rs` `KNOWN_DRIFT` ledger correctly records that Progress drift was retired.

However, **the component cannot be called production-ready** because several test-coverage gaps violate the spec's own test-strategy tables, and one user-visible CLI behavior (`NO_COLOR`) regresses relative to the bespoke path.

---

## Findings

### 1. Terminal layout parity is incomplete — right/top/bottom margins and center alignment untested

**Severity: HIGH**

The spec's Terminal Test Strategy explicitly requires:

> Layout left/right/top/bottom margins and center align → Tree layout matches bespoke layout semantics

The only layout dimension explicitly tested for Progress is **left margin** (`test_progress_uses_layout` in-source, `left_margin_is_honored_through_tree_path` in parity). No test covers:

- Right margin
- Top margin
- Bottom margin
- Center alignment
- Right alignment

The `layout_matrix_support` harness *does* exercise these scenarios (e.g., `right_margin_4`, `top_margin_2`, `align_center`), but after the flip both halves of the matrix call `progress.render(&term)` — which routes through the tree — making the comparison tautological. The true bespoke path (`render_bespoke`) is never compared against the tree path for any layout variant other than left margin.

**Recommended fix:** Add dedicated parity tests in `progress_parity.rs` that construct a Progress with each non-default layout variant, call both `render_bespoke` and `render` (the tree path), and assert ANSI-stripped equality. This is the only way to prove the tree renderer preserves bespoke layout semantics for Progress.

---

### 2. `NO_COLOR=1` is not honored by `bt progress`

**Severity: MEDIUM**

`bt progress` uses `detect_terminal_honoring_force_color()`, which checks `FORCE_COLOR` / `CLICOLOR_FORCE` but **does not inspect `NO_COLOR`**. If the underlying terminal reports truecolor (e.g., `COLORTERM=truecolor`), `bt progress 50 --fill-color green` will emit `\x1b[32m` even when `NO_COLOR=1` is set.

The integration test `test_progress_terminal_default` sets `NO_COLOR=1` but only asserts content presence (`Loading`, `60%`), not escape absence. The in-source unit test `progress_slot_colors_degrade_with_color_depth` tests `ColorDepth::None`, but that requires the caller to explicitly construct a terminal with no color support — it does not mirror the CLI environment.

By contrast, `bt prose` manually strips SGR sequences when `NO_COLOR` is present. `bt progress` does not.

**Recommended fix:** Either teach `detect_terminal_honoring_force_color()` to downgrade `color_depth` to `ColorDepth::None` when `NO_COLOR` is set (preferred — fixes all commands at once), or add a post-render strip step in `bt progress` like `bt prose` does. Add an integration test that asserts `NO_COLOR=1 bt progress 50 --fill-color green` emits no `\x1b` bytes.

---

### 3. Missing MarkdownPlus layout-no-op test

**Severity: MEDIUM**

The spec's Markdown Test Strategy requires:

> MarkdownPlus, with layout → Layout has no effect on output

No test creates a Progress with non-default layout and renders `render_markdown_plus()`, asserting that no layout CSS (e.g., `margin-left`, `text-align`) appears. The existing `markdown_renderable_drops_colors_glyphs_layout` only tests **portable Markdown**, not MarkdownPlus.

**Recommended fix:** Add a parity test that builds a Progress with `left_margin(4ch)` and `alignment(Center)`, renders `render_markdown_plus()`, and asserts the output contains no `margin-left`, `margin-right`, or `text-align` declarations.

---

### 4. No explicit `BrowserTreeComponent<Progress>` test

**Severity: MEDIUM**

The spec's acceptance criteria state:

> Progress can be rendered through `BrowserTreeComponent`

While `Progress: TreeRenderable` and `BrowserTreeComponent` is generic, there is **zero component-level test** verifying this pipeline. The adapter's own unit tests (`browser_adapter.rs`) use synthetic `Para`/`Broken` stubs. A regression in the adapter's error policy (e.g., `fallback_fragment` emitting `[render-tree error: …]`) would not be caught by any Progress-specific test.

**Recommended fix:** Add a test in `progress_parity.rs` that wraps a styled Progress in `BrowserTreeComponent`, renders `render_html_fragment()`, and asserts the HTML contains `role="progressbar"` and the correct `aria-valuenow`.

---

### 5. Small terminal widths are not stress-tested

**Severity: MEDIUM**

The spec's Terminal Test Strategy requires:

> Small terminal widths → Output remains deterministic and does not panic

`progress_renders_at_all_parity_widths` tests widths 40, 80, and 120. A default Progress bar with label is ~31 visible columns, so width 40 does not stress the renderer. There is **no test for widths smaller than the bar content** (e.g., 10 or 20 columns), and no bespoke-vs-tree parity at those widths.

This matters because the tree path routes the bar through `Prose::render`, which may wrap or trim differently than `apply_block_layout` at very small widths. The bespoke renderer never wraps a progress bar; the tree path's behavior is uncharacterized in this regime.

**Recommended fix:** Add parity tests for widths 10 and 20 that assert both `render_bespoke` and `render` produce deterministic output (no panic) and document any accepted divergence in a local `KNOWN_DRIFT` comment.

---

### 6. `BrowserTreeComponent` fallback emits in-band sentinel text

**Severity: LOW (cross-component inconsistency)**

`BrowserTreeComponent::fallback_fragment` returns a visible HTML fragment containing `[render-tree error: {error}]`. This is inconsistent with:

- The terminal adapter's policy (`tracing::error!` + empty string)
- The lessons-learned guidance: "the fallback policy must be uniform across targets (empty output + structured log)"
- Progress's own `BrowserRenderable` impl, which correctly returns an empty fragment and logs the error

Since the spec encourages rendering Progress through `BrowserTreeComponent`, a structural validation failure would result in visible error text in the HTML output rather than a clean empty fragment plus a log line.

**Recommended fix:** Change `fallback_fragment` to return an empty fragment and emit `tracing::error!`. This is an adapter change, not a Progress change, but it affects Progress's production contract.

---

### 7. Layout matrix is tautological for Progress

**Severity: INFORMATIONAL**

Since the `TerminalRenderable` flip, both halves of the layout-matrix `Progress` cell call the same tree renderer. The `render_comparison.rs` `KNOWN_DRIFT` ledger correctly documents this retirement. This is not a bug — it is the expected consequence of a successful flip — but it means the matrix no longer provides meaningful bespoke-vs-tree divergence signal for Progress. The bespoke fallback (`render_bespoke`) is exercised only by dedicated parity tests, not by the matrix.

No action required, but future reviewers should be aware that the matrix's silence on Progress is a sign of success, not neglect.

---

## Test Coverage Assessment

| Level | Present? | Assessment |
|-------|----------|------------|
| **L1 — Unit** | ✅ | Strong. 18 in-source tests covering construction, clamping, custom glyphs, custom width, percentage alignment, layout (left margin only), color degradation, serde round-trip, tree node structure, and slot color builders. |
| **L1 — Integration (in-process)** | ✅ | Good. `progress_parity.rs` (37 tests) guards projection shape, validation, terminal semantic parity, bespoke-vs-tree parity at default style, Markdown fallback, MarkdownPlus HTML, browser semantic output, and color depth none. CLI integration tests cover default terminal, `--example`, all cross-target flags, mutual exclusion, invalid percentages, and color flag parsing. |
| **L2 — Real terminal** | ✅ | Good. `level2_render_tree_style.rs` exercises Progress slot colors in WezTerm, Kitty, and tmux. |
| **L3 — OS keyboard** | N/A | Not applicable for a non-interactive rendering component. |

**Gaps in test coverage:**
- No L1/L2 test verifying `NO_COLOR=1` produces zero SGR escapes for `bt progress` with slot colors.
- No parity test for right/top/bottom margins or center/right alignment (only left margin is covered).
- No MarkdownPlus test asserting layout is ignored.
- No explicit `BrowserTreeComponent<Progress>` rendering test.
- No stress test for terminal widths smaller than the bar content.

---

## Ergonomic / Performance Observations

1. **`render_via_tree` is the single terminal path** — After the flip, `render(&term)` and `render_optimistic()` both call `render_via_tree`, which constructs a `RenderNode` and invokes `render_terminal_node`. This is slightly heavier than the old bespoke path, but it is the intended architecture and is already parity-gated. The overhead is negligible for a single-line widget.

2. **`paint_fg` allocates unconditionally** — When no color is set, `paint_fg` still does `text.to_string()`, allocating a new `String` identical to the input. A `Cow<'_, str>` return type would avoid this, but the allocation is tiny (one progress bar segment) and not worth complicating the signature for.

3. **`progress_html` in `shared.rs` is a single source of truth** — Browser and MarkdownPlus both call the same `shared::progress_html` helper. This prevents the two targets from drifting, which is exactly the pattern the lessons-learned document recommends. Good.

4. **Error handling is uniform across Progress's own trait impls** — Terminal, Browser, and Markdown paths all log via `tracing::error!` and fall back to empty output. This follows the lessons-learned policy and is a positive exception to the generic `BrowserTreeComponent` adapter's behavior.

---

## Production Readiness

**Judgment: NOT production-ready.**

The implementation is **architecturally sound, functionally correct, and well-tested for its happy path**, but four gaps block production readiness:

1. **Terminal layout parity is incomplete.** The spec requires bespoke-vs-tree verification for left/right/top/bottom margins and center alignment. Only left margin is explicitly parity-tested. Without these tests, a regression in the tree renderer's layout lowering for Progress would not be caught by the current suite.

2. **`NO_COLOR` is not honored by `bt progress`.** This is a user-visible accessibility regression relative to standard CLI conventions and relative to `bt prose`, which manually strips SGR. A command that respects `NO_COLOR` for prose but emits color for progress violates the principle of least surprise.

3. **MarkdownPlus layout behavior is unverified.** The spec requires that layout has no effect on MarkdownPlus output, but no test asserts this. A future change to the MarkdownPlus renderer that accidentally pipes layout CSS into the inline HTML would land green.

4. **`BrowserTreeComponent<Progress>` is untested at the component level.** The spec lists this as an acceptance criterion, yet the only adapter tests use synthetic stubs. A regression in the adapter's error policy or strictness clamping would not be caught by Progress's own suite.

Once these four items are addressed — plus the small-width stress test and the adapter-level `NO_COLOR` fix — the component will be production-ready. The core tree projection, cross-target rendering, CLI surface, and Level-2 real-terminal verification are all in excellent shape.
