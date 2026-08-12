---
$schema: "@.claudine/schemas/review.yaml"
ready: false
implemented: true
agent: codex/default
created: 2026-07-13T06:45:56-07:00
---

# Review 1 — Style Features

## Verdict

Not ready for production. The feature identity, first-seen deduplication, typed classic/module
scripts, streaming/fragment parity, Markdown neutrality, resolver failures, and Mermaid API
retirement are implemented and have useful Level 1 coverage. The production surface still has
four functional defects: Darkmatter nests a complete HTML document inside a page `<div>`, the
public `HtmlPage` path can silently drop requested assets, Mermaid's generated theme values are
never passed to Mermaid, and popover IDs are unique only within one writer invocation. The most
important user-observable behavior also lacks the required browser/Level 3 verification.

## Findings

### High — The body-only path emits an invalid nested HTML document

`render_tree_html_page_body` calls `render_browser_document_html`, so its `body` value begins with
`<!DOCTYPE html><html>...` rather than being a body fragment
(`darkmatter/lib/src/markdown/render_tree/entrypoints.rs:253-283`). `wrap_browser_html` then writes
that value inside `<div class="darkmatter-page">`
(`darkmatter/lib/src/layout/page.rs:1742-1763`). The resulting shape is:

```html
<div class="darkmatter-page">
  <style>...</style>
  <!DOCTYPE html><html><head>...</head><body>...</body></html>
</div>
```

This is not valid HTML and does not implement acceptance criterion 5's body-only fragment. Browser
error recovery may ignore or relocate the nested document tokens, so the emitted bytes are not a
stable embedding contract. The focused snapshot and tests currently make the defect normative by
searching for the nested doctype and calling it the “embedded document body.”

Render a real fragment for this path, or assemble one valid full document with the page wrapper
inside its `<body>`. Add a browser-tier DOM assertion that the wrapper is a child of the real body,
contains the rendered Markdown and inline feature assets, and contains no nested `html`, `head`, or
`body` element. Replace the wrapper-prefix snapshot with full-page and true body-fragment
snapshots, as criterion 12 requires.

### High — Public `HtmlPage` rendering still silently drops feature requests

`HtmlPage::set_feature_resolver` is public, but resolution occurs only in the crate-private
`inject_resolved_features` method (`renderable/src/html/mod.rs:149-193`). Public
`HtmlPage::render()` merely renders the precomputed `feature_head`
(`renderable/src/html/mod.rs:341-345`). Therefore these documented public flows do not inject even
the default Popover CSS and cannot report `UnresolvedFeature`:

- `HtmlPage::from(feature_fragment).render()`;
- `HtmlPage::from_fragments(...).render()`; and
- `BrowserRenderable::render_html_page(...).render()`.

Only callers that pass through `render_browser_document` receive injection. This is the same
dead-end the specification set out to remove and contradicts the architecture ruling that direct
`HtmlPage` callers receive generic assets through `DefaultFeatureResolver`.

Make final page rendering fallible and resolve there (the codebase has no established users, so a
clean API correction is preferable), or provide a public fallible assembly/finalization API that
all constructors and `BrowserRenderable` promotion paths must use. Add tests for direct
`HtmlPage`, nested fragments, a custom resolver, Popover success, and an unresolved feature error;
do not preserve a public path that silently omits declared dependencies.

### High — Mermaid's theme-aware assets do not affect rendered diagrams

`DarkmatterFeatureResolver` emits `--mermaid-*` CSS custom properties, but no production CSS or
JavaScript reads them. The bootstrap initializes Mermaid with only `startOnLoad: false`
(`darkmatter/lib/src/mermaid/feature.rs:174-182`). Mermaid's supported customization contract
requires `theme: 'base'` and a `themeVariables` object passed to `mermaid.initialize`; arbitrary
`--mermaid-*` properties are not an input to its theming engine. See Mermaid's authoritative
[theme configuration](https://mermaid.js.org/config/theming.html).

The page therefore renders Mermaid's default theme while tests merely count unused variable names.
Pass the resolved palette through Mermaid's configuration. If the diagram must follow
`prefers-color-scheme`, select the light/dark `themeVariables` in the bootstrap (and define how a
live mode change rerenders), or resolve one palette from `FeatureContext::color_mode` and document
that fixed behavior. Add a browser-tier test using a local/pinned Mermaid module fixture and assert
browser-computed SVG fill/stroke/text colors in both light and dark modes.

### High — Popover IDs are not document-unique across composed fragments

`PopoverIdAllocator` is owned by each fragment `Writer` and streaming `StreamWriter`
(`renderable/src/tree/render/browser.rs:2724-2750`). It guarantees uniqueness only while one writer
walks one tree. Rendering two prompted-link nodes separately with `render_browser_node` and then
composing their fragments into one `HtmlPage` resets the allocator and emits the same ID twice.
The standalone `Link::to_html_with_popover` helper has the same limitation because it derives one
deterministic ID with no document-scoped occurrence state.

This violates criterion 7's document-unique association and can make `aria-describedby` resolve to
the wrong prompt. Move ID allocation to a true final-document assembly context, or carry a typed
popover request until final rendering so composed fragments can be renumbered safely. Add a test
that independently renders two identical prompted-link fragments, combines them with
`HtmlPage::from_fragments`, and asserts unique IDs and correct associations.

### High — Interactive Mermaid and body placement have no real-browser verification

`full_page_browser_mermaid_defaults_to_interactive` is selected by `just test-browser` only because
its name contains `browser`; it is a synchronous string-assertion test and never launches Chrome.
No browser test loads the module, observes Mermaid replace the `<pre>` source with an SVG, checks
the primary-to-fallback import behavior, verifies that blocked imports leave readable source, or
checks the light/dark theme. The body-only tests likewise assert the invalid source shape rather
than the parsed DOM.

Under the repository test taxonomy these requirements need the Browser tier, not Level 1 source
matching. Keep tests network-free by serving a deterministic local module or intercepting the CDN
requests in the harness. Exercise successful rendering, primary failure plus fallback success,
total failure with readable source, deduplication in the live DOM, valid wrapper placement, and
theme application.

### High — Popover interaction verification does not cover the specified user actions

The one real-Chrome test calls `a.focus()` from JavaScript
(`darkmatter/lib/tests/browser_render.rs:1351-1363`). It proves the CSS result after focus, but it
does not prove that Tab reaches the anchor, that hover works, or that Enter keeps ordinary link
navigation. Because keyboard/mouse injection is required for those user actions, the strongest
appropriate evidence is Level 3; none exists. There is also no browser coverage for viewport-edge
placement, long prompt wrapping, dark/light colors, or reduced motion. The implementation's
`left: 0; right: auto; width: max-content` rule can overflow when the triggering link is near the
right edge, despite the comment claiming viewport safety. Firefox and WebKit are represented only
by an unexecuted manual checklist.

Add Level 3 browser-window tests for Tab/Shift-Tab, Enter, and pointer hover, gated with
`RUN_LEVEL3=1`. Add ordinary browser-tier computed-style/geometry tests for right and left viewport
edges, long prompts, both color modes, and `prefers-reduced-motion`. Record executed Firefox and
WebKit checks, or narrow the supported-browser claim until those engines are verified.

### Medium — Darkmatter erases typed feature failures into strings

`PageRenderError::FeatureResolution(String)` stringifies `FeatureResolveError`, and the
`HeadRequired` test can only search the message. The specification says body-only placement fails
with `FeatureResolveError::HeadRequired`; callers should be able to match that variant rather than
parse prose. Store the source error directly with a transparent `#[from]` variant and test the
typed match for both `HeadRequired` and `UnresolvedFeature`.

## Requirement-to-verification assessment

| Requirement | Strongest evidence | Assessment |
|---|---|---|
| 1. Mermaid asset dedup | Level 1 unit/integration assertions on both render paths | Appropriate for byte/order contract; passed |
| 2. Markdown neutrality | Level 1 render/snapshot tests | Appropriate; passed |
| 3. Interactive browser default | Level 1 source assertions, mislabeled into Browser tier by test name | Insufficient: no live Mermaid execution |
| 4. Compatibility defaults | Level 1 mode-map and terminal regression tests | Appropriate for default/no-I/O selection; explicit Image regression is inherited rather than feature-focused |
| 5. Body-only placement | Level 1 snapshot/assertions that freeze a nested doctype | Wrong behavior and wrong level; needs valid DOM verification in Browser tier |
| 6. Divergent request config | Deferred by the specification until a config-bearing feature exists | Not applicable to fieldless v1 |
| 7. Popover behavior | Level 1 markup tests plus Browser-tier programmatic `.focus()` | Insufficient for keyboard/mouse behavior; no Level 3, viewport, hover, reduced-motion, Firefox, or WebKit evidence |
| 8. `MermaidHtml` retirement | Source audit and compilation | Appropriate; passed |
| 9. Resolver failures | Level 1 typed tests in `renderable`; string-only Darkmatter test | Mostly appropriate; Darkmatter loses the typed error |
| 10. Side-channel preservation | Level 1 map/hook/parity/order tests | Appropriate; passed for the audited constructors |
| 11. Asset safety/failure fallback | Level 1 escaping/version/source assertions | Escaping and pinning passed; import-failure/readable-source behavior lacks Browser-tier verification |
| 12. Cross-platform/regression | macOS Level 1 and Browser recipes; no full valid body-only snapshot | Current host gates pass, but the specified full-page/body-only coverage is incomplete |
| 13. Documentation cleanup | Source/reference audit and updated docs | `MermaidHtml` cleanup passed; body-only and theme docs currently describe behavior the implementation does not provide |

## Verification performed for this review

- `just test` from `renderable/`: 520 passed, 15 skipped.
- `cargo nextest run -p darkmatter -E 'binary(style_features_baseline) + binary(style_features_phase5)'`: 12 passed.
- `just test-browser` from `darkmatter/`: 84 passed. Only the popover focus test launched Chrome for this feature; the Mermaid default test was an in-process string test selected by naming.
- Began `just test` from `darkmatter/` twice. After the dependency build, 2,110 of 5,506 tests passed before interruption at the non-interactive duration limit; one unrelated timeout test failed its first attempt and passed its configured retry. This is not a completed broad-suite result.
- Inspected the feature resolver/serializers, both browser writers, `HtmlPage`, Darkmatter page assembly, Mermaid bootstrap/theme code, popover markup/CSS, feature-specific tests/snapshots, documentation, plan, and implementation notes.
- GitNexus was 30 commits stale. The mandated refresh fallback was attempted but did not complete inside the safe non-interactive command window, so findings were verified directly against the current working-tree source and tests rather than relying on the stale graph.
