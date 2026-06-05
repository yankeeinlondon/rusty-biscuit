---
ready: false
agent: codex
model: ""
---

# Review: Darkmatter Code-Block Rendering Failures

## Findings

### High: Terminal UX requirements are only verified at Level 1, but the spec requires real-terminal evidence

The implementation adds useful invariant tests in `darkmatter/lib/tests/render_invariants.rs`, but the requirements that are explicitly user-observable in a terminal remain verified by in-process ANSI-string inspection only:

- code panel contrast / specific light-vs-dark background (`i7_code_block_inverts_theme_against_dark_terminal`, lines 405-426)
- code background rectangle, no right-margin gap, pill/body right-boundary coherence (`layout_invariants_hold_across_matrix`, lines 239-303)
- blank-line rhythm as seen in terminal output (`vertical_rhythm`, `margin_blanks`, and `blank_line_count_is_idempotent`)

Per the review rubric, those are Level 1 tests: they inspect bytes produced by the renderer, not what a real terminal emulator renders. The existing `darkmatter/lib/tests/level2_render_tree_terminal.rs` change is formatting-only and does not add a WezTerm/Kitty/tmux capture for the repro command or for the new color/background/right-edge assertions. Requirements like "`^X` badges with specific colors" were called out as needing Level 2; this feature has the same class of terminal-rendering assertions: SGR background, glyph widths, pane text, and row spacing through a real terminal. This should not be marked production-ready until there is at least a Level 2 test or documented env-gated real-terminal check that captures the rendered pane and verifies the contrast/background/right-boundary/blank-line behavior.

### High: Render-tree HTML code blocks still do not invert their theme

The Stage 4 docs and skill say inversion is applied at all code-highlighter construction sites, including the render-tree `CodeRenderer`, but the browser hook still constructs the highlighter with the non-inverted mode:

`darkmatter/lib/src/markdown/render_tree/code_renderer.rs:163-166`

```rust
let options = HtmlOptions::default();
// Browser code blocks do not invert (terminal-only contrast); see the
// note in `darkmatter::markdown::output::html::as_html`.
let highlighter = CodeHighlighter::new(options.code_theme, options.color_mode);
```

That leaves `render_tree_html` with dark code panels on dark pages for paired themes, while legacy `as_html` now inverts. The tests in `darkmatter/lib/tests/html_inversion.rs` exercise only `as_html`, not the render-tree browser path, so this regression is not caught. Fix by using `options.color_mode.inverted()` here as well, and add a render-tree HTML inversion test using a paired theme such as `github`.

### High: Mermaid code-block fallback keeps the old layout and color-mode behavior under page decoration

The normal fenced-code path now passes `code_color_mode` and a `body_width` whenever `layout_ctx` is present, but the Mermaid text/fallback paths still pass `options.color_mode` and `None`:

- `darkmatter/lib/src/markdown/output/terminal.rs:1097-1115`
- `darkmatter/lib/src/markdown/output/terminal.rs:1159-1177`

Under `MermaidMode::Text`, or `MermaidMode::Image` when rendering fails, this still lets `highlight_code` emit `\x1b[K` while page row decoration later appends margins. That is the same right-margin gap / clear-to-physical-edge defect fixed for ordinary code fences. The highlight-line background math also keys off the non-inverted page mode instead of the resolved code-panel mode. Add Mermaid text and image-fallback cases to the invariant matrix, and route them through the same `code_width`, `body_width`, `apply_component_layout`, and `code_color_mode` logic as ordinary code blocks.

## Test Rigor Classification

- Normal terminal code fences: strongest current verification is Level 1. Needs Level 2 for color/background/right-edge/pane text behavior.
- HTML legacy `as_html`: covered by in-process HTML string assertions; acceptable for non-terminal HTML, though a computed-style browser test would be stronger.
- HTML render-tree path: missing coverage for inversion.
- Mermaid code blocks under terminal layout: missing targeted coverage; current invariant matrix does not exercise `MermaidMode::Text` or image fallback.
- No Level 3 requirements found; the spec does not assert OS keyboard-event behavior.

## Verification

I attempted `cargo test -p darkmatter --test render_invariants -- --nocapture`, but stopped it after it was still compiling beyond the non-interactive timeout window. No green test result was obtained during this review.

## Production Readiness

Not ready for production. The ordinary code-fence path is much improved, but the feature still has spec-covered rendering paths that do not implement the new contract, and the terminal-facing behavior is not verified at the required Level 2.
