---
clarified: claude/fable
reviewed: true
review_iterations: 7
status: ready for planning and implementation
---

# Style Features

## Status

**Reviewed and ready for planning and implementation.** The feature-collection and
resolution architecture is approved. This review makes asset ownership, resolver
composition, failure behavior, output placement, and compatibility defaults explicit.
The exact popover trigger markup remains an implementation-planning decision; the
recommended progressive-enhancement design is recorded under [Open Questions](#open-questions).

When we designed the "style" system for Darkmatter we built a set of CSS-like configuration for certain block items like tables, code-blocks, block quotes, and more. This grammar allows for some useful configuration of stylistic rendering but one thing that was designed but not yet fully realized is the idea of a "feature" which a renderable component can declare. A feature is a declarable dependency bundle a component may request, covering:

- JavaScript
- CSS, including CSS variables

The type-safe identity for a feature already exists: the `PageFeature` enum in `renderable/src/browser/feature.rs` (a fieldless `Copy` enum with variants such as `CopyToClipboard`, `MermaidDiagram`, `DarkMode`, etc.). Components can already request features today — `BrowserFragment::add_feature` exists, and `HtmlPage::features()` (`renderable/src/html/mod.rs`) rolls up and dedups requested features in first-seen order — but the pipeline currently dead-ends there: `render_head` never consumes the rollup, so no assets are ever injected. This spec defines the missing half: mapping requested features to actual CSS/JS assets and injecting them exactly once per rendered page.

## Goals and Non-Goals

**Goals**

- Preserve `PageFeature` as the shared, type-safe declaration and deduplication key.
- Make both fragment-based and streaming browser renders collect the same feature set.
- Resolve collected features through an explicit host-provided policy, with deterministic
  ordering and actionable failures.
- Inject browser assets once in complete documents and retain the required assets in
  Darkmatter's body-only browser output.
- Make interactive Mermaid output functional and give prompted links an accessible,
  progressively enhanced presentation.

**Non-goals**

- Features are not a package manager, dependency graph, or arbitrary remote-code loader.
- V1 does not translate browser CSS into terminal styling.
- V1 does not add assets to Markdown or MarkdownPlus output.

## Multi Target Output

A feature is output-target aware. It must know that if the output target is Markdown (or MarkdownPlus) no JavaScript can be used and, in fact, features must never alter Markdown-family output at all. Terminal output can render some but not all CSS; the design intent is that where a `px`/`rem`/etc. unit is used in the CSS it would be downsampled to a unit that works in the terminal (e.g., `ch`).

> **Ruling (deferred):** terminal CSS downsampling is recorded design intent but is explicitly **out of v1 scope**. No v1 feature exercises it — mermaid renders as an image on the terminal and popover is browser-only. When it is needed, the existing `Length`/`TargetValue` machinery in `renderable` is the likely home.

## Architecture

The following structure was ratified as a hybrid between "renderable owns everything" and "each consumer owns its own assets":

1. **Identity** — `PageFeature` (`renderable/src/browser/feature.rs`) stays the type-safe identity and the deduplication key. New features (e.g. a popover feature) are added as variants there.
2. **Resolution seam** — a new object-safe `FeatureResolver` trait in `renderable` maps
   `(PageFeature, RenderTarget, &FeatureContext) → Result<Option<FeatureAssets>, FeatureResolveError>`.
   `FeatureContext` carries only renderable-owned values needed during resolution; v1 includes
   color mode and resolved semantic colors. It must not depend on Darkmatter types.
   This keeps the dependency direction `darkmatter → renderable` and lets Darkmatter derive
   theme-aware assets before calling the shared assembler.

   ```rust
   pub struct FeatureAssets {
       pub css: Option<Cow<'static, str>>,
       pub js: Option<FeatureScript>,
       pub links: Vec<LinkTag>,
   }

   pub enum FeatureScript {
       Classic(Cow<'static, str>),
       Module(Cow<'static, str>),
   }
   ```

   A typed script kind is required because wrapping all JavaScript in the existing classic
   `<script>` path would make Mermaid's ESM bootstrap invalid.

3. **Default resolver** — `renderable` ships a `DefaultFeatureResolver` for generic features.
   V1 resolves the popover feature there. Existing `PageFeature` variants that have no asset
   implementation remain unresolved; they must not silently claim to be active.
4. **Darkmatter resolver** — Darkmatter provides `DarkmatterFeatureResolver`, which resolves
   `MermaidDiagram` using colors derived from the document's resolved theme and delegates
   every other variant to `DefaultFeatureResolver`. Resolver composition is explicit
   delegation, not merging two independently returned asset bundles; one resolver owns a
   feature request, preventing ambiguous CSS/JS ordering.
5. **Host injection** — `HtmlPage` and `BrowserRenderOptions` accept a resolver and feature
   context. The default is `DefaultFeatureResolver`; Darkmatter installs its resolver on its
   browser entry points. Callers that construct `HtmlPage` directly therefore receive only
   generic assets and do not acquire a dependency on Darkmatter.

### Resolution failures and unsupported targets

- `RenderTarget::Markdown` and `RenderTarget::MarkdownPlus` bypass collection and resolution.
  Their output remains byte-for-byte neutral.
- A resolver returns `Ok(None)` only when a feature intentionally has no assets for the
  requested target. On the Browser target, a requested but unresolved feature produces a
  `FeatureResolveError::UnresolvedFeature`; silently dropping a browser dependency is forbidden.
- Resolver errors become a dedicated `RenderError::FeatureResolution` variant and include the
  feature identity and target.
- CSS, scripts, and links are emitted in first-seen feature order. Within one feature the order
  is links, CSS, then script. Page-authored links/styles/scripts retain their existing relative
  order, followed by feature assets, so feature code can rely on its own declarations without
  changing existing page output when no feature is requested.

### Feature flow and collection

`Rendered<T>` (`renderable/src/tree/error.rs`) grows a `features: Vec<PageFeature>` field
(first-seen order), mirroring the existing `diagnostics` side-channel. `Rendered::new` starts
with an empty set and `Rendered::map` preserves it. Every public renderer constructing
`Rendered` directly must be migrated so the new side channel is never accidentally discarded.

The streaming browser path (`render_browser_document_html` in
`renderable/src/tree/render/browser.rs`) currently writes through a Writer that creates no
fragments, so `add_feature` can never fire there. The Writer therefore accumulates features as
it emits nodes (for example, `write_mermaid_interactive` requests `MermaidDiagram`) and also
merges features from code-renderer hook fragments in document order. The fragment path keeps
using recursive `BrowserFragment` collection. The outermost document assembler resolves and
injects the deduplicated result exactly once.

Collection is request-only: a Mermaid fence rendered as code or static SVG must not request the
interactive Mermaid feature, and a link without prompt metadata must not request the popover
feature.

### Body-only renders

`DarkmatterPage::render_to_browser` (`darkmatter/lib/src/layout/page.rs`) returns a body-only HTML
fragment via `wrap_browser_html`, and can return a bare body with no wrapper when no decoration
is configured (for example, when Claudine embeds a render). **Ruling:** when a body-only render
requests features, inline `<style>` and `<script>` assets are emitted before the body inside a
wrapper that is forced into existence. A bare-body render never silently drops features.

`LinkTag` dependencies cannot be injected into a body-only fragment because their document-head
semantics cannot be guaranteed by an embedder. If a requested feature resolves to links in this
mode, rendering fails with `FeatureResolveError::HeadRequired` and identifies the feature. V1's
Mermaid and popover assets are inline, so this restriction does not block either implementation.
The wrapper receives a stable `data-darkmatter-features` attribute listing requested feature
names for debugging; it is not a configuration or runtime lookup surface.

## Mermaid Feature

Mermaid charts are really useful and so we support rendering them across targets. The approach differs per target; the following per-target behavior is ratified:

| Target | Behavior | Notes |
|---|---|---|
| **Terminal** | Code remains the library default; image is an explicit opt-in | Image rendering may launch `mmdc` or use `mermaid.ink`, so making it implicit would add process/network I/O to previously side-effect-free defaults. Today's image pipeline is retained. |
| **Browser** | Interactive is Darkmatter's full-page default; `renderable` keeps `BrowserMermaidMode::Code` as its low-level default | Static SVG and code remain opt-ins. `GraphicsMode::Vector` continues to cap `Interactive` to `StaticSvg`; `GraphicsMode::Off` renders code. |
| **Markdown** | Mermaid stays a fenced `mermaid` code block | Features never alter Markdown output. |
| **MarkdownPlus** | Keep the fence | Matches `TargetValue`'s MarkdownPlus-falls-back-to-Markdown philosophy; GitHub-class consumers render mermaid fences natively. |

This split is intentional compatibility policy. Darkmatter's browser-facing page API opts into
the feature-aware experience, while the shared low-level renderer and terminal API do not gain
implicit remote or subprocess behavior. JavaScript rendering reduces bytes embedded in the HTML
artifact, but transfers the Mermaid library at view time and therefore is not an offline mode.

### JS delivery (browser)

**Ruling:** the Mermaid bootstrap is an inline `<script type="module">` that dynamically imports
an exact, spec-owned Mermaid version from jsDelivr and retries the same exact version from unpkg
if the first import fails. The version must not use a floating major tag. Initialization occurs
after the module loads, targets only `.mermaid` elements, and reports a concise `console.error`
if both imports fail while leaving the escaped diagram source visible. The future `publish`
command may add a vendoring/sidecar option; that is out of scope for v1.

This delivery requires network access and a Content Security Policy that permits the chosen CDN
origins and inline modules. The generated markup must document that constraint. No diagram source
or document metadata is sent anywhere except as required by the browser's module fetch; Mermaid
runs locally in the browser.

Today, `BrowserMermaidMode::Interactive` emits `<pre class="mermaid">` with no script anywhere — the output is inert. This feature makes Interactive actually work by injecting the bootstrap through the feature pipeline.

### `MermaidHtml` retirement

The dead API `darkmatter/lib/src/mermaid/mod.rs::render_for_html` (`MermaidHtml`) builds a CDN-ESM head snippet that nothing consumes. **Ruling:** it is retired as part of this feature; its bootstrap logic is absorbed into darkmatter's `FeatureResolver`, which becomes the single owner of mermaid browser assets.

## Popover Feature

**Ruling (v1 scope):** prompted links use native HTML/CSS behavior with no custom JavaScript and
degrade to an ordinary navigable link when the enhancement is unavailable. Prompt content must
be reachable by keyboard, remain escaped, and be associated with its trigger through stable,
document-unique IDs and appropriate ARIA attributes. Multiple identical links in one document
must not generate duplicate IDs.

Grounding: `Link::to_html_with_popover` (`darkmatter/lib/src/render/link.rs`) already emits a
`popover="hint"` companion and an `interestfor` attribute. `interestfor` and the established
`popovertarget` invoker mechanism are different browser surfaces, and `popovertarget` is defined
for button-like controls rather than ordinary navigable anchors. The final trigger markup is
therefore resolved in [Open Questions](#open-questions), not assumed by the asset resolver.

The emitting link component requests `PageFeature::Popover` (a new variant; use this general name
rather than coupling the identity to links). The resolver supplies only shared CSS. Markup,
accessibility attributes, and unique-ID allocation remain the component/renderer's responsibility.

### Supported-engine contract (v1)

The viewport-safe edge-flip is **Chromium-verified**. Chrome 125+ ships CSS anchor positioning,
and automated headless-Chrome geometry tests assert the panel stays on-screen when the link is
pinned to either viewport edge.

On engines **without** CSS anchor positioning — Firefox and WebKit at the time of writing — the
panel falls back to a `max-width`-capped (`min(20rem, calc(100vw - 1rem))`), left-anchored layout.
Ordinary inline links stay on-screen and long prompts wrap inside the cap, but a link positioned
very near the **right** viewport edge can have its panel's right edge overflow. This is an
**accepted, documented v1 limitation, not a defect**: a portable pure-CSS edge-flip is not
achievable without CSS anchor positioning or JavaScript, and JavaScript is explicitly out of v1
scope (the popover is CSS-only in v1). A future vendored or JS-enhanced path could close the gap.

Firefox and WebKit are **not** part of the automated verification set in v1 — the repo has no
automation harness for those engines, and they are not installed. The `prefers-reduced-motion`
override applies on all engines.

## Deduplication

When a page is rendered, all the renderable components' requests for features are collected and de-duplicated so that a feature's assets are injected at most once.

**Rulings:**

- Deduplication identity is the `PageFeature` variant, preserved in first-seen order (matching the existing `HtmlPage::features()` behavior).
- **Divergent configuration for the same feature on one page is a hard error** — the render fails. It is not a warning, and it is not resolved by config-hash double-injection.
- This rule is **forward-looking**: v1 features are fieldless (`PageFeature` is a `Copy` enum) and per-page configuration lives in the resolver, so two requests cannot diverge yet. The rule binds any future feature that gains per-request configuration — such configuration must be comparable, and the assembler must fail the render on inequality.

Because fieldless `PageFeature` cannot represent divergent request configuration, v1 does not add
dead comparison machinery. Before the first config-bearing feature ships, identity must evolve to
a comparable request type (for example, `FeatureRequest { feature, config }`) and activate the
hard-error acceptance test. Resolver-level page context is deliberately not per-request config:
one resolver/context pair applies to the whole page.

## Implementation Targets

To ensure that our implementation of features has legs we will implement the following features:

- mermaid feature (JS-only: an inline ESM module; the palette is delivered via Mermaid `themeVariables`, not CSS)
- popover feature (CSS-only implementation)

## Acceptance Criteria

1. **Dedup** — a page containing two mermaid blocks emits exactly one injected mermaid `<script type="module">` block (deduplicated). The Mermaid feature is **script-only**: the resolved palette is delivered through Mermaid's own `themeVariables` (baked into the bootstrap's `mermaid.initialize` call), not through CSS — Mermaid does not read CSS custom properties, so no mermaid CSS block is emitted.
2. **Markdown neutrality** — (a) existing Markdown and MarkdownPlus snapshot tests are unchanged by this feature landing, and (b) a page whose components request features produces Markdown-family output containing no injected asset markup.
3. **Browser default** — rendering a Mermaid fence through Darkmatter's full-page browser path
   with default settings emits the Interactive markup (`<pre class="mermaid">`) plus the inline
   ESM bootstrap script.
4. **Compatibility defaults** — Darkmatter's full-page browser path defaults to Interactive;
   `BrowserRenderOptions::default()` and terminal defaults continue to render Mermaid as code and
   perform no network or subprocess I/O. Explicit terminal Image mode retains its current static
   image behavior.
5. **Body-only renders** — a body-only browser render (`DarkmatterPage::render_to_browser`) that requests features forces the wrapper div into existence and contains the inline `<style>`/`<script>` assets inside it.
6. **Divergent config** — two requests for the same feature with divergent configuration fail the render with a clear error. *(Activates with the first config-bearing feature; not testable against v1's fieldless features.)*
7. **Popover** — a page containing two prompted links gets popover CSS exactly once, unique IDs,
   keyboard-reachable prompts, and ordinary working links when popover/interest behavior is not
   supported; a page without a prompted link gets no popover CSS.
8. **Retirement** — `darkmatter::mermaid::render_for_html` (`MermaidHtml`) no longer exists.
9. **Resolver failures** — a requested unresolved browser feature fails with its feature and
   target in the error; a body-only render whose feature requires a head link fails with
   `HeadRequired`.
10. **Side-channel preservation** — `Rendered::map`, fragment hook collection, and the streaming
    writer preserve feature requests in deterministic first-seen order.
11. **Asset safety** — Mermaid source and prompt content remain HTML-escaped, the CDN version is
    exact and identical across primary/fallback imports, and a blocked/failed import leaves
    readable source rather than an empty diagram.
12. **Cross-platform and regression checks** — unit tests use no network, browser snapshots cover
    full-page and body-only output, and existing terminal tests remain deterministic on macOS,
    Windows, and Linux.
13. **Documentation cleanup** — Darkmatter's Mermaid README and public rustdoc no longer mention
    `MermaidHtml`; browser docs describe resolver installation, network/CSP requirements, and the
    low-level versus Darkmatter default split.

## Out of Scope (v1)

- Sidecar image files and `publish` command integration (the sidecar solution gains importance when the `publish` command is introduced, but not before).
- Terminal CSS downsampling (`px`/`rem` → `ch`).
- Vendored/offline mermaid JS delivery.
- A new general-purpose popover component beyond styling the native popover API (plus the invoker markup emitting components need).
- Dependency kinds beyond JS and CSS ("other?").
- CSP nonce/hash plumbing and Subresource Integrity. Dynamic ESM imports cannot use the existing
  `LinkTag` model to express SRI; a vendored/publish path should address this as one design.
- Changing low-level renderable or terminal Mermaid defaults to behavior that performs network or
  subprocess I/O.

## Open Questions

### Which declarative trigger should prompted links emit?

The current code emits `interestfor` on a navigable anchor, while the draft originally described
`popovertarget`, whose established invoker is a button. Replacing the anchor with a button would
damage link semantics; relying on only the newer interest-invoker behavior would make prompts
unavailable in clients that do not implement it.

1. **Progressive enhancement: anchor + CSS fallback + `interestfor` (recommended).** Wrap the
   anchor and prompt in a stable container, retain the real `href`, expose the prompt on
   `:hover` and `:focus-within`, and keep `interestfor`/`popover="hint"` as an enhancement where
   supported.
   - Pros: preserves navigation, works with mouse and keyboard without JavaScript, degrades
     gracefully, and builds on the markup already emitted.
   - Cons: requires wrapper markup and careful CSS to avoid hover/focus flicker; native top-layer
     positioning is not available in every browser.
2. **Dedicated `popovertarget` button beside the link.** Keep the anchor and add a separate button
   that opens the prompt.
   - Pros: uses the established Popover API explicitly and provides a clear keyboard control.
   - Cons: adds visual/control clutter, creates two focus stops, and changes compact inline-link
     layout.
3. **`interestfor` only.** Keep today's anchor/companion shape and style only supporting browsers.
   - Pros: smallest markup change and native interest-trigger behavior where implemented.
   - Cons: prompt access depends on a newer browser surface and does not satisfy graceful
     enhancement by itself.

**Recommendation:** choose option 1. It is the only CSS-only design that preserves an ordinary
link as the primary control while making prompt content available across a broad range of mouse
and keyboard clients. The final markup is validated in Chromium (automated headless-Chrome
geometry, focus, color-mode, and reduced-motion tests), and every transition carries a
reduced-motion override. Firefox and WebKit are **not** validated in v1 — no automation harness
exists for those engines and they fall back to the left-anchored, `max-width`-capped layout whose
supported-engine contract and accepted right-edge limitation are recorded in the
[Popover Feature](#popover-feature) section. A portable pure-CSS edge-flip across all three engines
is not achievable without CSS anchor positioning (Chromium-only) or JavaScript (out of v1 scope),
so the contract is honestly scoped to what ships rather than claiming three-engine parity.

Popover colors, spacing, maximum width, stacking, and motion should use existing renderable style
tokens where available. The implementation may tune those values without revisiting the
architecture. Snapshots and Chromium geometry tests cover long wrapped prompts, viewport edges,
focus, dark/light color modes, and `prefers-reduced-motion`.
