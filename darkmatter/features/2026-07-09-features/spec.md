---
clarified: claude/fable
---

# Style Features

When we designed the "style" system for Darkmatter we built a set of CSS-like configuration for certain block items like tables, code-blocks, block quotes, and more. This grammar allows for some useful configuration of stylistic rendering but one thing that was designed but not yet fully realized is the idea of a "feature" which a renderable component can declare. A feature is a declarable dependency bundle a component may request, covering:

- JavaScript
- CSS, including CSS variables

The type-safe identity for a feature already exists: the `PageFeature` enum in `renderable/src/browser/feature.rs` (a fieldless `Copy` enum with variants such as `CopyToClipboard`, `MermaidDiagram`, `DarkMode`, etc.). Components can already request features today — `BrowserFragment::add_feature` exists, and `HtmlPage::features()` (`renderable/src/html/mod.rs`) rolls up and dedups requested features in first-seen order — but the pipeline currently dead-ends there: `render_head` never consumes the rollup, so no assets are ever injected. This spec defines the missing half: mapping requested features to actual CSS/JS assets and injecting them exactly once per rendered page.

## Multi Target Output

A feature is output-target aware. It must know that if the output target is Markdown (or MarkdownPlus) no JavaScript can be used and, in fact, features must never alter Markdown-family output at all. Terminal output can render some but not all CSS; the design intent is that where a `px`/`rem`/etc. unit is used in the CSS it would be downsampled to a unit that works in the terminal (e.g., `ch`).

> **Ruling (deferred):** terminal CSS downsampling is recorded design intent but is explicitly **out of v1 scope**. No v1 feature exercises it — mermaid renders as an image on the terminal and popover is browser-only. When it is needed, the existing `Length`/`TargetValue` machinery in `renderable` is the likely home.

## Architecture

The following structure was ratified as a hybrid between "renderable owns everything" and "each consumer owns its own assets":

1. **Identity** — `PageFeature` (`renderable/src/browser/feature.rs`) stays the type-safe identity and the deduplication key. New features (e.g. a popover feature) are added as variants there.
2. **Resolution seam** — a new `FeatureResolver` trait in `renderable` maps `(PageFeature, RenderTarget) → Option<FeatureAssets>`, where:

   ```rust
   struct FeatureAssets {
       css: Option<Cow<'static, str>>,
       js: Option<Cow<'static, str>>,
       links: Vec<LinkTag>,
   }
   ```

3. **Default resolver** — `renderable` ships a `DefaultFeatureResolver` that supplies assets for generic features (e.g. the popover CSS).
4. **Darkmatter resolver** — darkmatter provides its own resolver which overrides `MermaidDiagram`, supplying theme-aware CSS variables (derived from the document's syntect `ThemePair`/color mode) plus the mermaid ESM bootstrap script.

### Feature flow and collection

`Rendered<T>` (`renderable/src/tree/error.rs`) grows a `features: Vec<PageFeature>` field (first-seen order), mirroring the existing `diagnostics` side-channel. The streaming browser path (`render_browser_document_html` in `renderable/src/tree/render/browser.rs`) currently writes through a Writer that creates no fragments, so `add_feature` can never fire there; instead the Writer accumulates features as it emits nodes (e.g. `write_mermaid_interactive` pushes `MermaidDiagram`). The outermost assembler — whoever constructs the head or wrapper — resolves, dedups, and injects assets exactly once. This fixes both the streaming path and the existing fragment path in a single place.

### Body-only renders

`DarkmatterPage::render_to_browser` (`darkmatter/lib/src/layout/page.rs`) returns a body-only HTML fragment via `wrap_browser_html`, and can return a bare body with no wrapper when no decoration is configured (e.g. when claudine embeds a render). **Ruling:** when a body-only render requests features, inline `<style>`/`<script>` blocks are appended inside the wrapper div, and the wrapper is **forced into existence** when features are present — a bare-body render never silently drops features.

## Mermaid Feature

Mermaid charts are really useful and so we support rendering them across targets. The approach differs per target; the following per-target behavior is ratified:

| Target | Behavior | Notes |
|---|---|---|
| **Terminal** | Inline static image is the **default** — the only real option for a terminal | Changes the shipped default: `MermaidMode::Off` → image. Today's pipeline (biscuit-terminal's `MermaidDiagram`: mmdc/mermaid.ink → PNG → viuer) is retained. |
| **Browser** | JS-library rendering of the code block (Interactive) is the **default** | Changes the shipped default: `BrowserMermaidMode::Code` → `Interactive`. Static image / `StaticSvg` remain available as opt-ins. `GraphicsMode::Vector` continues to cap `Interactive` → `StaticSvg`. |
| **Markdown** | Mermaid stays a fenced `mermaid` code block | Features never alter Markdown output. |
| **MarkdownPlus** | Keep the fence | Matches `TargetValue`'s MarkdownPlus-falls-back-to-Markdown philosophy; GitHub-class consumers render mermaid fences natively. |

Letting mermaid be rendered by JavaScript in the browser enables dynamic features that a static image cannot support, and means "shipping less bytes."

### JS delivery (browser)

**Ruling:** the mermaid bootstrap is an inline `<script type="module">` that imports mermaid from a CDN (jsdelivr, with unpkg as fallback) at view time. The future `publish` command may later add a vendoring/sidecar option; that is out of scope for v1.

Today, `BrowserMermaidMode::Interactive` emits `<pre class="mermaid">` with no script anywhere — the output is inert. This feature makes Interactive actually work by injecting the bootstrap through the feature pipeline.

### `MermaidHtml` retirement

The dead API `darkmatter/lib/src/mermaid/mod.rs::render_for_html` (`MermaidHtml`) builds a CDN-ESM head snippet that nothing consumes. **Ruling:** it is retired as part of this feature; its bootstrap logic is absorbed into darkmatter's `FeatureResolver`, which becomes the single owner of mermaid browser assets.

## Popover Feature

**Ruling (v1 scope):** implement a variant of the modern native HTML popover API — modern browsers support modal popovers with capabilities that rival JS solutions. The v1 implementation is CSS-only (no custom JS). Iteration is expected to get the styling right.

Grounding: `Link::to_html_with_popover` (`darkmatter/lib/src/render/link.rs`) already emits `popover="hint"` companion divs today, but they are unstyled and trigger-less. Note that showing a `[popover]` element without JS requires `popovertarget` invoker buttons in the markup, so the feature covers the popover CSS plus whatever invoker markup the emitting components produce.

## Deduplication

When a page is rendered, all the renderable components' requests for features are collected and de-duplicated so that a feature's assets are injected at most once.

**Rulings:**

- Deduplication identity is the `PageFeature` variant, preserved in first-seen order (matching the existing `HtmlPage::features()` behavior).
- **Divergent configuration for the same feature on one page is a hard error** — the render fails. It is not a warning, and it is not resolved by config-hash double-injection.
- This rule is **forward-looking**: v1 features are fieldless (`PageFeature` is a `Copy` enum) and per-page configuration lives in the resolver, so two requests cannot diverge yet. The rule binds any future feature that gains per-request configuration — such configuration must be comparable, and the assembler must fail the render on inequality.

## Implementation Targets

To ensure that our implementation of features has legs we will implement the following features:

- mermaid feature (JS and CSS)
- popover feature (CSS-only implementation)

## Acceptance Criteria

1. **Dedup** — a page containing two mermaid blocks emits exactly one injected mermaid `<script type="module">` block and one mermaid CSS block.
2. **Markdown neutrality** — (a) existing Markdown and MarkdownPlus snapshot tests are unchanged by this feature landing, and (b) a page whose components request features produces Markdown-family output containing no injected asset markup.
3. **Browser default** — rendering a mermaid fence to the browser target with default settings emits the Interactive code-block markup (`<pre class="mermaid">`) plus the inline ESM bootstrap script.
4. **Terminal default** — rendering a mermaid fence to the terminal target with default settings embeds a static inline image.
5. **Body-only renders** — a body-only browser render (`DarkmatterPage::render_to_browser`) that requests features forces the wrapper div into existence and contains the inline `<style>`/`<script>` assets inside it.
6. **Divergent config** — two requests for the same feature with divergent configuration fail the render with a clear error. *(Activates with the first config-bearing feature; not testable against v1's fieldless features.)*
7. **Popover** — a page containing a link-with-prompt gets the popover CSS injected exactly once; a page without one gets no popover CSS.
8. **Retirement** — `darkmatter::mermaid::render_for_html` (`MermaidHtml`) no longer exists.

## Out of Scope (v1)

- Sidecar image files and `publish` command integration (the sidecar solution gains importance when the `publish` command is introduced, but not before).
- Terminal CSS downsampling (`px`/`rem` → `ch`).
- Vendored/offline mermaid JS delivery.
- A new general-purpose popover component beyond styling the native popover API (plus the invoker markup emitting components need).
- Dependency kinds beyond JS and CSS ("other?").

## Open Questions

- Popover styling specifics: the native-popover CSS is expected to take iteration to get right (positioning, transitions, hint vs. modal behavior). The v1 ruling fixes the approach (native popover API, CSS-only), not the final visual design.
