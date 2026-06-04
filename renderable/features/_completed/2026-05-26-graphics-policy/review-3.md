---
ready: true
agent: codex
model: ""
---

# Review 3

## Findings

### High -- Browser Mermaid code fallback drops code-block metadata and highlighting

The spec says Mermaid remains a `Code` node until promoted and that disabled or
capped promotion is lossless: `title="..."`, `line-numbering`, and `highlight`
metadata should still render as the full code-block presentation. The current
browser renderer short-circuits every Mermaid non-promotion path directly to
`render_plain_code_block` (`renderable/src/tree/render/browser.rs:756`,
`renderable/src/tree/render/browser.rs:786`,
`renderable/src/tree/render/browser.rs:793`). That bypasses the normal
`CodeRenderer::render_browser_code` hook (`renderable/src/tree/render/browser.rs:797`),
which is where darkmatter parses and renders title, line numbers, highlights,
and syntax-highlighted HTML (`darkmatter/lib/src/markdown/render_tree/code_renderer.rs:178`).

This is no longer necessary for safety: review-2 split Mermaid SVG promotion
into `render_browser_mermaid`, and darkmatter's `render_browser_code` no longer
promotes Mermaid. The current implementation therefore preserves the new
promotion guard but regresses the spec's lossless fallback contract.

Requirement verification level: Level 1 is appropriate for the metadata/fallback
policy itself, but there is no L1 regression covering Mermaid Off/Code/static
SVG failure with code-block metadata. Browser-rendered title/line-number/highlight
presentation should also have browser-tier DOM checks if treated as production
browser behavior.

Recommended fix: route Mermaid `Off`, `Code`, and static-SVG failure fallback
through the normal code fallback path that consults `render_browser_code` first,
then plain code only if the hook returns `None`. Add L1 tests with a darkmatter
code renderer and Mermaid metadata.

### High -- Browser-visible SVG behavior is still only string/unit verified

The spec requires browser HR at `Vector`/`Rich` to render the styled SVG and
browser Mermaid at `Vector`/`Rich` with promotion enabled to render static SVG.
The implementation has string-level tests for HR (`renderable/src/tree/render/browser.rs:1949`,
`renderable/src/tree/render/browser.rs:1970`) and in-process Mermaid policy
tests (`renderable/src/tree/render/browser.rs:1686`,
`renderable/src/tree/render/browser.rs:1783`,
`renderable/src/tree/render/browser.rs:1900`). The only browser harness test I
found exercises code-block background color, not HR SVG or Mermaid SVG
(`darkmatter/lib/tests/browser_render.rs:30`).

Per the requested rigor rubric, user-observable browser graphics need a real
browser/DOM verification tier, not just source-string assertions. String tests
can prove the renderer emitted `<svg>`, but they do not prove the SVG is valid
DOM, styled as intended, or rendered with the expected computed properties.

Requirement verification level: current strongest verification is Level 1 /
string-level. The appropriate level is browser-tier DOM/computed-style coverage.

Recommended fix: add browser harness tests that render a `style: waves` HR under
`Rich`/`Vector` and assert `.darkmatter-hr` exists with computed/displayed SVG
properties. Add a browser-tier static Mermaid test when the Mermaid renderer is
available, skipping cleanly when the host lacks the Mermaid toolchain.

### High -- Rich terminal image success path has no success coverage and no real-terminal coverage

The spec requires `TerminalImage`/image nodes to render alt text at
`Off`/`Vector` and attempt inline image protocol output at `Rich`. The tree
renderer now has a `render_terminal_image` implementation
(`biscuit-terminal/lib/src/render_tree/render.rs:1231`), but the tests I found
only cover missing-file fallback and `Off`/`Vector` alt text
(`biscuit-terminal/lib/src/render_tree/render.rs:2706`,
`biscuit-terminal/lib/src/render_tree/render.rs:2717`). There is no L1 test with
a real small image file on a Kitty-capable terminal asserting image escape
output, and no Level 2 capture for the user-visible Rich image path.

Requirement verification level: terminal inline image rendering is user-visible
terminal graphics. It needs at least L1 success/fallback policy coverage, and
production readiness needs Level 2 real-terminal capture for the protocol path.

Recommended fix: add a tiny fixture image and L1 tests for `Rich` success,
`Rich` no-capability fallback, remote/absolute/traversal rejection, and
`force_graphics`. Add a Level 2 real-terminal test for a Rich image node through
the tree path.

## Test Rigor Notes

- Terminal HR `Vector` has both L1 and Level 2 coverage, which matches the
  requirement.
- Terminal Mermaid promotion has L1 policy/strictness coverage; I did not find a
  Level 2 test for promoted Mermaid image output through the tree path.
- No Level 3 requirement was identified; the spec does not define keyboard,
  mouse, paste, IME, or other OS-input behavior.

## Verification Performed

- Read `renderable/features/2026-05-26-graphics-policy/spec.md`,
  `review-1.md`, and `review-2.md`.
- Inspected the touched implementation in `renderable`, `darkmatter`, and
  `biscuit-terminal`.
- Ran `cargo test -p renderable --lib mermaid --color=never`: passed.
- Ran `cargo test -p darkmatter --lib browser_code_does_not_promote_mermaid --color=never`: passed.
- Ran `cargo test -p darkmatter --lib browser_options_mapping_maps_mermaid_image_to_static_svg --color=never`: passed.
- Ran `cargo test -p biscuit-terminal --lib render_tree_thematic_break_vector_mode_uses_text_tier --color=never`: passed.
- Two initial cargo invocations with multiple test filters failed with Cargo
  usage errors before running tests; I reran the relevant filters one at a time.

## Production Readiness

Not ready for production. The review-2 fixes are implemented, but the browser
Mermaid fallback path still violates the spec's lossless code-block fallback
contract, and browser/terminal graphics coverage is below the requested
verification level for production readiness.

## Resolution

All three High findings are addressed.

### Finding 1 — Browser Mermaid fallback now lossless

`render_code_block` no longer short-circuits the Mermaid non-promotion paths to
`render_plain_code_block`. A new `render_code_fallback` helper consults
`CodeRenderer::render_browser_code` first (which reproduces title / line-number /
highlight markup) and only falls back to the plain `<pre><code>` block when no
hook is installed or it declines. Every non-promotion outcome — `Off`, `Code`,
and a degraded `StaticSvg` failure under `Warn`/`Lossy` — routes through it.
`StaticSvg` promotion still uses the dedicated, fallible `render_browser_mermaid`
hook so failures remain observable to strictness; the generic hook's contract
forbids promoting `lang="mermaid"`, so the fallback cannot re-introduce an SVG.

- `renderable/src/tree/render/browser.rs` — `render_code_block`,
  `render_code_fallback`.
- L1 tests: `mermaid_non_promotion_preserves_code_block_metadata_via_hook`,
  `mermaid_non_promotion_falls_back_to_plain_code_without_hook`
  (`renderable`), and
  `render_tree_html_mermaid_off_preserves_code_block_metadata`
  (`darkmatter`, end-to-end with the real `TerminalCodeRenderer`).

### Finding 2 — Browser SVG now has DOM-tier coverage

Added real headless-Chromium tests in `darkmatter/lib/tests/browser_render.rs`:

- `browser_hr_waves_svg_computes_in_browser` — renders a `style: waves` HR at
  both `Vector` and `Rich` and asserts `.darkmatter-hr` exists, computes
  `display: block`, and that its waves `<path>` parsed into the DOM with a
  resolved `stroke-width`.
- `browser_mermaid_static_svg_computes_in_browser` — drives `as_html` with
  `MermaidMode::Image` and asserts the promoted `<svg>` exists as DOM, skipping
  cleanly when the host lacks the Mermaid toolchain.

### Finding 3 — Rich terminal image success + real-terminal coverage

- L1 tests (`biscuit-terminal/lib/src/render_tree/render.rs`):
  `render_tree_image_rich_emits_image_protocol`,
  `render_tree_image_rich_no_capability_falls_back_to_alt_text`,
  `render_tree_image_rich_rejects_remote_absolute_and_traversal`,
  `render_tree_image_force_graphics_emits_protocol_on_unsupported_terminal`
  (using a tiny generated PNG fixture).
- L2 test (`darkmatter/lib/tests/level2_render_tree_terminal.rs`):
  `level2_tree_rich_image_node_emits_protocol_and_renders_in_real_terminal` —
  renders a real image node through `render_terminal_document`, asserts the
  iTerm2 protocol bytes are emitted, and `cat`s them into a real WezTerm pane
  to prove the terminal consumes the image rather than showing the alt-text
  fallback.
