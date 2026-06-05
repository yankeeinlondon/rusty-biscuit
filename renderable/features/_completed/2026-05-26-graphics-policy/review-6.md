---
ready: false
agent: codex
model: ""
---

# Review 6

## Findings

### High -- Mermaid SVG sanitizer still permits CSS/external-reference payloads

The new Mermaid static-SVG path now routes `MermaidDiagram::render_to_svg()`
through `sanitize_svg` before raw HTML emission, which closes the direct
`<script>`, event-handler, `<foreignObject>`, and `href` cases from review 5
(`darkmatter/lib/src/markdown/render_tree/code_renderer.rs:198-205`). However,
the sanitizer still preserves `<style>` elements verbatim and preserves every
non-`href` attribute after XML escaping (`darkmatter/lib/src/markdown/render_tree/svg_sanitizer.rs:53`,
`darkmatter/lib/src/markdown/render_tree/svg_sanitizer.rs:107-114`,
`darkmatter/lib/src/markdown/render_tree/svg_sanitizer.rs:156-167`).

That leaves URL-bearing SVG/CSS surfaces in sanitized raw HTML, for example:

- `<style>@import url(https://attacker.example/x.css)</style>`
- `<style>rect { fill: url(https://attacker.example/p.svg#x) }</style>`
- `<rect style="fill:url(https://attacker.example/p.svg#x)">`
- `<rect filter="url(https://attacker.example/filter.svg#f)">`

This contradicts the sanitizer's own presentation-only/no-external-resource
contract (`svg_sanitizer.rs:20-27`) and leaves the implementation short of the
spec's "sanitized static `<svg>`" requirement. The current browser regression
only exercises `<script>`, `foreignObject`, event handlers, and
`javascript:`/external `href` (`darkmatter/lib/tests/browser_render.rs:324-350`);
it does not cover CSS `url(...)`, `@import`, `style` attributes, or other
fragment-reference attributes.

Requirement verification level: browser Mermaid static SVG is user-observable
browser behavior and the safety boundary is raw-HTML emission. Current strongest
coverage is L1 plus browser-tier tests for a subset of active markup. The
missing cases need L1 sanitizer fixtures and a browser-tier regression proving
no external/CSS reference payload survives into the DOM.

Recommended fix: either drop `<style>` entirely, parse and allowlist CSS
declarations, or strip any declaration/value containing `url(` or `@import`.
For attributes, use an allowlist of SVG presentation/geometry attributes and
reject URL-capable attributes unless the value is a local `#...` fragment.
Add tests for `<style>@import`, CSS `url(...)`, `style="...url(...)"`,
`filter="url(https://...)"`, and local fragment values that should remain.

## Test Rigor Notes

- HR hostile hint handling now has L1 validation coverage and browser-tier DOM
  coverage for injected nodes.
- Browser Mermaid static SVG now has L1 sanitizer coverage and browser-tier
  coverage for direct active markup, but not for the remaining CSS/external
  reference surfaces listed above.
- Rich terminal image rendering now has a Level-2 pixel-readback test in WezTerm
  (`level2_tree_rich_image_node_paints_distinctive_pixels`). That is the right
  verification level for painted terminal graphics; it still self-skips when
  screen capture is unavailable or blocked, which is acceptable as a harness
  availability condition but should be documented in release/test sign-off.
- No Level-3 requirement was identified; the spec does not define keyboard,
  mouse, paste, IME, or OS-input behavior.

## Verification Performed

- Read `renderable/features/2026-05-26-graphics-policy/spec.md`, `plan.md`,
  and `review-5.md`.
- Inspected the staged implementation changes in `renderable`, `darkmatter`,
  `biscuit-terminal`, and `biscuit-test-harness`.
- Ran `cargo test -p renderable --lib graphics --color=never`: passed.
- Ran `cargo test -p darkmatter --test browser_render sanitized --color=never`:
  passed.

## Production Readiness

Not ready for production. The review-5 findings are substantially addressed,
but the Mermaid sanitizer still allows CSS/external-reference payloads through
the raw-HTML SVG boundary, so the "sanitized static `<svg>`" requirement is not
complete.
