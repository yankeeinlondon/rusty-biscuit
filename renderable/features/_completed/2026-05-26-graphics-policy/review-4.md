---
ready: false
agent: codex
model: ""
---

# Review 4

## Findings

### High -- Browser Mermaid DOM coverage exercises the legacy renderer, not the tree graphics-policy path

The spec brings Mermaid into scope specifically as tree lowering through the
`CodeRenderer` extension point: `NodeKind::Code { lang: "mermaid", ... }` is
promoted by the browser tree renderer when `BrowserRenderOptions::mermaid_mode`
and `GraphicsMode` permit it. The new browser-tier test is valuable, but it
does not exercise that path. `browser_mermaid_static_svg_computes_in_browser`
calls `Markdown::as_html` (`darkmatter/lib/tests/browser_render.rs:139`), and
`Markdown::as_html` delegates to the legacy HTML renderer
(`darkmatter/lib/src/markdown/mod.rs:595`,
`darkmatter/lib/src/markdown/output/html.rs:153`). The internal tree entry
point remains separate (`darkmatter/lib/src/markdown/render_tree/entrypoints.rs:74`),
and the test never drives `render_browser_document`, `render_tree_html`, or
`BrowserRenderOptions { mermaid_mode: StaticSvg, ... }`.

Requirement verification level: browser Mermaid static SVG is user-observable
browser behavior. Current strongest tree-path verification is Level 1 string
coverage in `renderable/src/tree/render/browser.rs`; the new browser-tier DOM
test covers the legacy surface instead. This is a Level mismatch under the
requested rubric, so it blocks production readiness.

Recommended fix: add a browser-tier test that folds Markdown to a render tree
and renders through `render_browser_document` with `TerminalCodeRenderer` and
`BrowserMermaidMode::StaticSvg`, or expose/use the internal `render_tree_html`
path from a suitable darkmatter test module. Assert the promoted SVG exists as
DOM the same way the current legacy test does. Keep the existing legacy test if
public `Markdown::as_html` parity is also desired.

### High -- Rich terminal image Level-2 test does not verify that the image rendered

The spec maps `TerminalImage` at `GraphicsMode::Rich` to inline image protocol
output. The new test adds useful L1 proof that the tree path emits iTerm2 bytes
(`darkmatter/lib/tests/level2_render_tree_terminal.rs:1077`) and then sends
those bytes through WezTerm. But the real-terminal assertion is only that
captured pane text does not contain `[cat]`
(`darkmatter/lib/tests/level2_render_tree_terminal.rs:1096`). That proves the
alt-text fallback was absent from captured text; it does not prove WezTerm
decoded or displayed the image. A malformed, unsupported, ignored, or
zero-visible image payload can still satisfy this assertion because text capture
does not include rendered image pixels.

Requirement verification level: terminal image rendering is user-observable
terminal graphics. Current strongest verification of the tree path is L1
protocol-byte emission plus a Level-2 text-capture negative assertion. That is
not enough to verify the rendered graphical result.

Recommended fix: strengthen the Level-2 test with a terminal screenshot or
backend-specific image-state assertion that can distinguish "image decoded and
painted" from "bytes were ignored/stripped and no alt text appeared." If the
available harness cannot inspect image pixels, document that limitation and do
not count this test as production-ready verification for terminal image
rendering.

## Test Rigor Notes

- Browser HR at `Vector`/`Rich` now has browser-tier DOM/computed-style coverage
  in `browser_hr_waves_svg_computes_in_browser`; that matches the requested
  level for the styled SVG requirement.
- Mermaid fallback metadata now has focused Level 1 coverage, including the
  darkmatter tree entry point for `MermaidMode::Off`.
- Terminal image policy now has good Level 1 coverage for Rich success,
  no-capability fallback, path rejection, and `force_graphics`.
- No Level 3 requirement was identified; the spec does not define keyboard,
  mouse, paste, IME, or OS-input behavior.

## Verification Performed

- Read `renderable/features/2026-05-26-graphics-policy/spec.md` and
  `review-3.md`.
- Inspected the changed implementation in `renderable`, `darkmatter`, and
  `biscuit-terminal`.
- Ran `sniff repo`.
- Ran `cargo test -p renderable --lib mermaid --color=never`: passed.
- Ran `cargo test -p darkmatter --test browser_render --color=never`: passed.
- Ran `cargo test -p biscuit-terminal --lib render_tree_image_ --color=never`:
  passed.
- Ran `cargo test -p darkmatter --test level2_render_tree_terminal level2_tree_rich_image_node_emits_protocol_and_renders_in_real_terminal --color=never`:
  passed.

## Production Readiness

Not ready for production. The implementation addressed the previous functional
gaps, but the remaining graphics requirements are not verified at the required
level for the actual tree paths.
