---
kind: review
component: TextBlock
reviewer: kimi
date: 2026-05-20
ready: false
---

# TextBlock Implementation Review

## Scope

Review of the `TextBlock` IR migration in `biscuit-terminal/lib/src/components/text_block.rs`, its CLI command, parity tests, and documentation against the spec at `components/TextBlock-spec.md`.

---

## Summary

The **implementation code is correct, complete, and well-structured**. `TextBlock` properly implements `TreeRenderable`, `TerminalRenderable`, `BrowserRenderable`, and `MarkdownRenderable`. The tree projection (`Paragraph(Text)` with `Style` and optional `Layout`) matches the spec exactly. RT-TEXTBLOCK-001 (Browser `Style` lowering) is consumed correctly. The `bt text-block` CLI is fully implemented. Documentation is updated.

However, **test coverage has material gaps** against the spec's own test-strategy tables, and **TextBlock is the only flipped component absent from the layout-matrix snapshot harness** — a regression-safety net that every other migrated component (BlockQuote, Progress, Section, Table, TwoColumn, UnorderedList) participates in.

---

## Findings

### 1. Missing layout-matrix coverage — **high**

`TextBlock` is not included in `biscuit-terminal/lib/tests/layout_matrix_support/mod.rs`'s `component_cases()`. Every other component whose `TerminalRenderable::render` was flipped to the tree path appears in the matrix:

- BlockQuote, Progress, Section, Table, TwoColumn, UnorderedList — all present.
- TextBlock — absent.

The layout matrix is the primary snapshot harness that catches layout regressions (margins, alignment, width, word-wrap) across 12 scenarios. Omitting TextBlock means:

- No snapshot catches a future regression in `apply_layout` for this component.
- No `render_comparison` drift ledger exists to retire (the spec acceptance criterion #2 is only half-met: parity tests exist in `text_block_parity.rs`, but the cross-component matrix does not).

**Remediation:** Add a `ComponentCase` for `TextBlock` to `layout_matrix_support/mod.rs`, generate snapshots, and verify no drift entries are needed (the tree path should agree with itself by construction, but the harness still guards the public `render()` surface).

### 2. Missing legacy parity test: center alignment — **medium**

The spec's "Legacy parity tests" table lists `#7: Layout with center alignment` as a required parity test. The `text_block_parity.rs` suite covers left margin (`layout_left_margin_applied_through_both_paths`) but **not center alignment**.

Center alignment is a distinct code path from left-margin padding: it requires width calculation and symmetric whitespace insertion. The bespoke `LayoutTerminalExt::apply_layout` and the tree renderer's `render_with_layout` could diverge here.

**Remediation:** Add `layout_center_alignment_applied_through_both_paths` to `text_block_parity.rs`.

### 3. Incomplete browser underline-variant coverage — **medium**

The spec's "Browser Test Strategy" table (#7) says: "Tests must cover … each underline variant." The current suite tests:

- `browser_underline_lowers_to_text_decoration` — Straight only.
- `browser_curly_underline_uses_wavy_decoration_style` — Curly only.

Missing: Double, Dotted, Dashed.

While the projection plumbing (`underline_style_from_request`) is the same for all variants, the browser renderer's CSS lowering for `text-decoration-style` is not — each variant maps to a different CSS keyword (`double`, `dotted`, `dashed`). Those paths are unverified.

**Remediation:** Add three tests:
- `browser_double_underline_lowers_to_text_decoration_double`
- `browser_dotted_underline_lowers_to_text_decoration_dotted`
- `browser_dashed_underline_lowers_to_text_decoration_dashed`

### 4. Missing markdown coverage for dim, blink, and HTML-sensitive content — **low–medium**

The spec's "Markdown Test Strategy" requires:

- `#8: Dim` — tested in `markdown_plus_renders_plain_text_regardless_of_style`? No, that test only uses bold.
- `#9: Blink` — no dedicated test.
- `#11: HTML-sensitive content` — no test.

The existing `markdown_renders_plain_text_regardless_of_style` covers bold, italic, strikethrough, fg, bg, and underline, but **not dim or blink**. The contract is "Markdown ignores Style entirely," so a single comprehensive test would suffice, but the current one is not comprehensive.

**Remediation:** Extend `markdown_renders_plain_text_regardless_of_style` to set dim and blink, and add `markdown_html_sensitive_content_is_escaped`.

### 5. No Level-2 (real-terminal) verification — **medium**

Per `prompts/snippets/test-rigor.md`, user-observable SGR behavior should have at minimum Level-1 tests, with Level-2 preferred for new terminal-facing features. `TextBlock` activates five previously inert fields (fg, bg, underline, strikethrough, blink). While BlockQuote has a dedicated `level2_render_tree_style.rs` suite verifying declared `Style` survives to real terminal cells, `TextBlock` has no equivalent.

This is especially relevant because:
- Color degradation (truecolor → 256 → 16) is environment-dependent.
- Underline variant support varies by terminal (e.g., curly may degrade to straight).
- Blink SGR is explicitly called out as "rarely supported."

**Remediation:** Add a `level2_text_block_style.rs` that drives `bt text-block` through WezTerm/Kitty/tmux and verifies that bold, fg color, and underline SGR sequences appear in captured pane text.

### 6. Dead code in bespoke renderer — **low**

`TextBlock::to_terminal()` contains:

```rust
let _underline = term.underline_support;
```

This binding is never read. It appears to be a leftover from an earlier iteration. It does not produce a compiler warning (the `let _underline` prefix suppresses the unused-binding lint), but it is misleading — a reader might assume underline logic was intended here.

**Remediation:** Remove the line.

### 7. `render_bespoke` visibility diverges from spec — **low**

The spec says: "Retain the old implementation as a private `bespoke_render()` fallback during the transition." The actual code uses `#[doc(hidden)] pub fn render_bespoke()`. This matches the pattern used by other flipped components (Progress, Section, etc.), so the divergence is consistent with repo conventions, but the spec text should be updated if `pub` is the intended visibility.

**Remediation:** Either make the method `pub(crate)` (truly private to the crate, sufficient for parity tests in `biscuit-terminal/lib/tests/`) or update the spec to match the `#[doc(hidden)] pub` convention.

---

## Ergonomics and Performance

### Positive

- **Single projection helper (`to_render_node`)** is the right pattern. Both `TreeRenderable::render_tree` and `TerminalRenderable::render_tree_node` delegate to it, preventing the drift called out in `lessons-learned.md` for OrderedList and Table.
- **Error handling is uniform** with other flipped components: `tracing::error!` + empty output, never an in-band sentinel. This follows the lesson from `BlockQuote` and `Progress`.
- **Builder API is preserved** and all stored fields now actually render — a genuine bug-fix, not a regression.
- **`build_style()` is private** and well-documented. No unnecessary API surface is exposed.

### Neutral / Notes

- `render_optimistic()` allocates a fresh `Terminal::new_optimistic()` on every call. This is the accepted pattern across the crate and is not a performance concern for `TextBlock`.
- The tree path is heavier than the old bespoke path (tree construction + validation + renderer walk vs. direct string formatting). This is the accepted architecture-wide trade-off and is not a `TextBlock`-specific issue.

---

## Production Readiness

**Judgment: `ready: false`**

The implementation code is solid, but the component cannot be called production-ready while it is **the only flipped component missing from the layout-matrix snapshot harness** and while **multiple spec-mandated tests are absent**.

The specific blockers are:

1. **Layout matrix omission** (high) — TextBlock is the outlier among all flipped components. The matrix is the primary regression-safety net for layout-bearing components.
2. **Missing center alignment parity test** (medium) — A spec-required test for a distinct layout code path is unimplemented.
3. **Incomplete browser underline coverage** (medium) — Three of five underline variants are untested in browser output.
4. **Missing markdown dim/blink/HTML-sensitive tests** (low–medium) — The spec's Markdown test strategy is not fully realized.
5. **No Level-2 verification** (medium) — Newly activated SGR fields (fg, bg, underline, strikethrough, blink) have only Level-1 verification. Given that these fields were previously inert bugs, real-terminal confirmation is appropriate before calling the fix shipped.

All of these are test-coverage gaps, not implementation defects. Once the layout matrix is wired, the missing parity/browser/markdown tests are added, and a minimal Level-2 smoke test is in place, `TextBlock` will be production-ready.
