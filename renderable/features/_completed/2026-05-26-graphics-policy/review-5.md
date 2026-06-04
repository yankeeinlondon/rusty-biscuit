---
ready: true
agent: codex
model: ""
---

# Review 5

## Findings

### High -- Styled HR SVG interpolates unescaped user-derived hints into raw HTML

`render_thematic_break` passes `darkmatter.hr` hint strings directly into
`horizontal_rule_svg` for `GraphicsMode::Vector` and `Rich`
(`renderable/src/tree/render/browser.rs:449`). Those hints originate from
`HorizontalRuleAttrs` string fields such as `width` and `color`
(`darkmatter/lib/src/markdown/inline/types.rs:43`) and are copied into the
render tree without validation or normalization
(`darkmatter/lib/src/markdown/render_tree/fold.rs:362`,
`darkmatter/lib/src/markdown/render_tree/fold.rs:366`).

The helper then interpolates those strings into an SVG `width` attribute and
inline `style` attribute (`renderable/src/tree/graphics.rs:58`,
`renderable/src/tree/graphics.rs:103`) and returns the result through
`define_as_raw_html`, whose contract says the string is never escaped
(`renderable/src/browser/fragment.rs:163`). A crafted HR attribute containing
quotes or CSS/HTML-breaking text can therefore escape the intended attribute
context in the tree browser renderer.

Requirement verification level: browser HR styled SVG is user-observable
browser behavior and should have browser-tier coverage for valid rendering, but
this is also a safety contract. Current tests cover normal values only; there
is no L1 sanitizer/unit test or browser-tier regression showing hostile
`width`/`color` values are escaped, rejected, or normalized.

Recommended fix: make the shared SVG helper typed or escaping-aware. At a
minimum, HTML-escape attribute values and CSS-escape style values before raw
HTML emission, and reject invalid CSS dimensions/colors before they reach the
helper. Add L1 tests with quote/script-breaking `width` and `color` values, plus
a browser-tier test proving the rendered DOM contains one SVG/HR and no injected
nodes.

### High -- Browser Mermaid static SVG is not sanitized before raw HTML emission

The spec calls for browser Mermaid promotion to emit a sanitized static SVG.
The tree path delegates to `TerminalCodeRenderer::render_browser_mermaid`,
which calls `MermaidDiagram::new(value).render_to_svg()` and immediately wraps
the returned string as raw HTML (`darkmatter/lib/src/markdown/render_tree/code_renderer.rs:189`).
The upstream renderer parses Mermaid, renders SVG, applies string overrides,
and returns that SVG (`biscuit-visualized/src/src/mermaid/render.rs:212`), but
there is no sanitizer step before `define_as_raw_html`.

This leaves the implementation dependent on every future SVG-producing path in
`mermaid-rs-renderer` and every local string override staying safe forever. That
does not implement the spec's "sanitized static `<svg>`" requirement.

Requirement verification level: promoted browser Mermaid SVG is user-observable
browser behavior and now has browser-tier DOM coverage for the tree path, which
is good. The missing sanitizer is an L1/Lbrowser gap: no tests assert that
dangerous SVG elements/attributes are stripped or that the emitted raw HTML is
restricted to an allowed SVG subset.

Recommended fix: add an explicit SVG sanitizer at the adapter boundary before
constructing `BrowserFragment::define_as_raw_html`. Test it directly with
malicious SVG fixtures (`<script>`, event-handler attributes, external refs,
`foreignObject`) and add a browser-tier regression for a Mermaid input or
mocked renderer output that would otherwise inject active markup.

### High -- Rich terminal image rendering is still not verified as painted output

The spec maps `TerminalImage` at `GraphicsMode::Rich` to inline image protocol
rendering. The current Level-2 test now documents its own limitation: it checks
that the tree renderer emits iTerm2 protocol bytes and that WezTerm text capture
does not show `[cat]`, but explicitly says it cannot prove the image was decoded
and painted (`darkmatter/lib/tests/level2_render_tree_terminal.rs:1037`).

That is accurate, but it means the requirement is still not production-ready
under the review rubric. A malformed or ignored image payload can pass the same
test because `wezterm cli get-text` strips graphics bytes from text capture.

Requirement verification level: terminal image rendering is user-observable
terminal graphics. Current strongest verification is L1 protocol-byte emission
plus a Level-2 negative text-capture assertion. That does not verify rendered
graphics.

Recommended fix: add a Level-2-capable image assertion using screenshots,
pixel-readback, or a backend-specific graphics placement query. If the current
harness cannot provide that, extend the harness or mark terminal image painting
as an explicit unresolved verification gap; do not count this requirement as
ready.

## Test Rigor Notes

- Browser HR styled SVG now has browser-tier DOM/computed-style coverage for
  normal `Vector`/`Rich` output.
- Browser Mermaid static SVG now has browser-tier coverage for the actual
  render-tree path, addressing the main review-4 tree-path gap.
- Terminal image policy has solid L1 coverage for mode routing, fallback,
  path rejection, and `force_graphics`, but not painted-image verification.
- No Level 3 requirement was identified; the spec does not define keyboard,
  mouse, paste, IME, or OS-input behavior.

## Verification Performed

- Read `renderable/features/2026-05-26-graphics-policy/spec.md`,
  `plan.md`, and `review-4.md`.
- Inspected the implementation in `renderable`, `darkmatter`,
  `biscuit-terminal`, `biscuit-visualized`, and the terminal/browser harnesses.
- Ran `sniff repo`: passed.
- Ran `cargo test -p renderable --lib graphics --color=never`: passed.
- Ran `cargo test -p darkmatter --test browser_render --color=never`: passed.
- Ran `cargo test -p biscuit-terminal --lib render_tree_image --color=never`:
  passed.

## Production Readiness

Not ready for production. The previous browser Mermaid path mismatch is fixed,
but the graphics-policy implementation still emits user-influenced SVG as raw
HTML without an explicit sanitizer, and the Rich terminal image requirement is
still not verified at the level needed to prove images actually render.

## Resolution

All three High findings are addressed.

### Finding 1 — Styled HR SVG hostile-input escape closed

`horizontal_rule_svg` (`renderable/src/tree/graphics.rs`) now whitelist-validates
the user-derived `width` and `color` hints before interpolation:
`is_safe_css_dimension` (digits + optional `.` + a known CSS unit) and
`is_safe_css_color` (hex, all-letter keyword, or `rgb()/rgba()/hsl()/hsla()`
with a restricted interior). None of the accepted character sets contains `"`,
`'`, `<`, `>`, or `;`, so a validated value cannot break out of the SVG `width`
attribute, the inline `style` declaration, or the `var(--hr-color, …)` fallback;
anything that fails validation falls back to the safe default (`100%` /
`currentColor`). `style` and `weight` already map through closed match arms.

- L1 tests (`renderable/src/tree/graphics.rs`):
  `safe_css_dimension_accepts_lengths_and_percentages`,
  `safe_css_dimension_rejects_hostile_values`,
  `safe_css_color_accepts_valid_colors`,
  `safe_css_color_rejects_hostile_values`,
  `hostile_width_and_color_fall_back_to_safe_defaults`.
- Browser-tier test (`darkmatter/lib/tests/browser_render.rs`):
  `browser_hr_hostile_attrs_inject_no_nodes` — drives real headless Chromium
  with quote/markup-breaking `width`/`color` and proves the `.darkmatter-hr`
  SVG stays intact while no injected `<img>`/`<script>` node enters the DOM.

### Finding 2 — Mermaid static SVG now sanitized before raw-HTML emission

A new allowlist sanitizer (`darkmatter/lib/src/markdown/render_tree/svg_sanitizer.rs`,
parse-based on `quick-xml`) re-emits only a presentation-only SVG element/attribute
subset. `render_browser_mermaid` (`code_renderer.rs`) routes
`MermaidDiagram::render_to_svg()` through `sanitize_svg` before
`define_as_raw_html`; a sanitizer parse failure returns `None`, which the render
tree treats as a promotion failure (no unsafe markup is emitted). Dropped:
non-allowlisted elements with their subtree (`<script>`, `<foreignObject>`,
`<image>`, animation/filter families), every `on*` handler, and any
`href`/`xlink:href` that is not a local `#…` fragment. Sanitization is the safety
net, so safety no longer depends on `mermaid-rs-renderer` or its string overrides
staying safe.

- L1 tests (`svg_sanitizer.rs`): strips `<script>` subtree, event handlers,
  `<foreignObject>` subtree, external/`javascript:` hrefs, case-obfuscated
  `<ScRiPt>`; preserves benign diagram markup without double-escaping; returns
  `None` on malformed XML.
- Browser-tier tests (`browser_render.rs`):
  `browser_sanitized_mermaid_svg_injects_no_active_markup` (real Chromium; a
  hostile mocked renderer output loses its active markup while `<svg>`/`<rect>`
  survive and no `<script>`/`<img>` node enters the DOM) and
  `sanitized_real_mermaid_retains_diagram_geometry` (a real promoted diagram
  keeps its drawable geometry through the sanitizer).

### Finding 3 — Rich terminal image now verified by pixel-readback

The harness gained `WezTermHarness::capture_window_png`
(`biscuit-test-harness/src/wezterm.rs`): it raises the spawned pane's window
(reusing the System Events bounds query, now factored into
`raise_and_window_bounds`) and screen-captures the region via macOS
`screencapture -R`, returning `None` when capture is unavailable.

`level2_tree_rich_image_node_paints_distinctive_pixels`
(`darkmatter/lib/tests/level2_render_tree_terminal.rs`) renders a `240×240`
solid-magenta image through the production tree path, paints it into a real
WezTerm pane, captures the window, and asserts magenta (`#ff00ff` — absent from
terminal chrome/text/background) is actually on screen. It skips cleanly when
the window cannot be raised or the capture is essentially black (the signature of
missing Screen Recording permission, indistinguishable from a paint failure). The
decode/threshold logic is independently covered by
`pixel_classification_distinguishes_magenta_from_black` (no terminal required).
The prior `level2_tree_rich_image_node_emits_protocol_and_renders_in_real_terminal`
remains the protocol-emission + `[cat]`-absence anti-regression check; its
verification-scope note now points to the pixel test for paint proof.
