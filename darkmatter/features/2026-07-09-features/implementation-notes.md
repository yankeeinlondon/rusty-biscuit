# Style Features — Implementation Notes

Working notes for the [execution plan](./plan.md). Records the Phase 1 inventory
and blast-radius data so no side channel or public surface is missed in later
phases.

## Phase 1 — Freeze Contracts and Baselines

### Symbol / side-channel inventory

Locations captured against the working tree at Phase 1 start. `renderable` is the
shared render crate; `darkmatter/lib` is the consumer that installs its own
resolver in later phases.

#### `Rendered<T>` (the side channel that grows `features` in Phase 2)

- Definition: `renderable/src/tree/error.rs:29` (struct), `Rendered::new` at
  `:39`, `Rendered::map` at `:48`. Fields today: `output`, `diagnostics` — **no
  `features` field yet**.
- Re-export: `renderable/src/tree/mod.rs:56`
  (`pub use error::{RenderError, RenderStrictness, Rendered}`).
- Live (non-test) constructors that Phase 2 must migrate so the new side channel
  is never discarded:
  - `renderable/src/tree/render/browser.rs:129` — `render_browser_node`
  - `renderable/src/tree/render/browser.rs:174` — `render_browser_document`
  - `renderable/src/tree/render/browser.rs:262` — `render_browser_document_html`
  - `renderable/src/tree/render/markdown.rs:131` — Markdown renderer
- No `Rendered` constructor exists in `darkmatter/` — it consumes them via
  `PipelineResult`.

#### Browser-render entry points

- `render_browser_node` — `renderable/src/tree/render/browser.rs:123`
  (`-> Result<Rendered<BrowserFragment<Ready>>, RenderError>`)
- `render_browser_document` — `:151` (`-> Result<Rendered<HtmlPage>, RenderError>`)
- `render_browser_document_html` — `:216` (streaming full-document path; builds
  the `StreamWriter`; `-> Result<Rendered<String>, RenderError>`)
- `render_tree_html_with_context` — `darkmatter/lib/src/markdown/render_tree/entrypoints.rs:193`
  (`pub(crate)`; calls `render_browser_document_html` at `:204`)
- All three renderable entries are public via
  `renderable/src/tree/mod.rs:59` and `renderable/src/tree/render/mod.rs:18`.

#### Feature-producing fragment hooks

- `BrowserFragment::add_feature` — `renderable/src/browser/fragment.rs:189`
  (pushes into `self.features` at `:190`).
- `BrowserFragment` `features` field — `:111`; typestate carry-over at `:494`,
  `:507`; accessor `features()` at `:469`.
- `HtmlPage.features` field — `renderable/src/html/mod.rs:40`; rollup accessor
  `HtmlPage::features()` — `:225` (first-seen dedup).
- **Dead-end confirmed:** `render_head` (`renderable/src/html/mod.rs:291`) never
  consumes the rollup, and `add_feature` has **zero production call sites** —
  only tests (`renderable/tests/render_pipeline.rs:116,155`). This is exactly the
  half the spec fills.

#### Mermaid promotion branches

- `BrowserMermaidMode` — `renderable/src/tree/mod.rs:83` (default `Code`);
  `mermaid_mode` field on `BrowserRenderOptions` — `browser.rs:84`.
- Fragment `Writer::render_code_block` — `browser.rs:892`; mermaid detect
  `:901`; graphics×mermaid resolution `:915`; dispatch `:921` (Interactive `:922`,
  StaticSvg `:923`, Code `:949`). `render_mermaid_interactive` — `:987` (emits
  `<pre class="mermaid">`, no script — inert today).
- Streaming `StreamWriter::write_code_block` — `browser.rs:1746`; detect `:1755`;
  resolution `:1764`; dispatch `:1770`. `write_mermaid_interactive` — `:1825`.
- Darkmatter static-SVG hook `CodeRenderer::render_browser_mermaid` — trait at
  `renderable/src/tree/render/mod.rs:125`; impl
  `darkmatter/lib/src/markdown/render_tree/code_renderer.rs:377`.

#### Prompted-link branch

- `Link::to_html_with_popover` — `darkmatter/lib/src/render/link.rs:606`
  (`interestfor` anchor `:615`, `<div … popover="hint">` `:647`); alias
  `to_browser_with_popover` — `:657`; `generate_popover_id` — `:1131`.
- `data-prompt` transport: emitted in Link's plain HTML at `link.rs:553`, parsed
  at `:934`. Structured-link lowering carries `prompt` → `data-prompt` on the
  anchor.
- Renderable tree link rendering (does **not** emit popover/`interestfor` today):
  `Writer::render_link` — `browser.rs:1189`; `StreamWriter::write_link` — `:1983`.
  `to_html_with_popover` has **no tree-pipeline caller** — it is a standalone
  shape Phase 4 reconciles.

#### `MermaidHtml` retirement targets (Phase 5)

- File `darkmatter/lib/src/mermaid/render_html.rs`: `struct MermaidHtml` `:22`,
  `MermaidHtml::new` `:30`, `detect_diagram_type` `:61`, `generate_css_variables`
  `:100`.
- `darkmatter/lib/src/mermaid/mod.rs`: `pub use render_html::MermaidHtml` `:17`;
  `Mermaid::render_for_html` `:287`; tests `:544,552,560,572,582,589`.
- Doc references to sweep: `darkmatter/lib/README.md:681`,
  `darkmatter/lib/src/mermaid/README.md:9,25,42`,
  `darkmatter/docs/rendering/mermaid.md:23,61,67,71,74` (stale),
  `.claude/skills/darkmatter/terminal.md`.
- No live pipeline caller of `render_for_html`/`MermaidHtml` outside the
  `mermaid` module itself.

#### Public re-exports touched

- `renderable::tree::{Rendered, RenderError, RenderStrictness}` — `tree/mod.rs:56`.
- `PageFeature` — defined `renderable/src/browser/feature.rs:8`; reached as
  `renderable::browser::feature::PageFeature` (no dedicated `pub use`).
- `FeatureResolver` / `FeatureAssets` / `FeatureScript` / `FeatureContext` —
  **do not exist yet** (created in Phase 2).

#### Test binaries touched by later phases (`--test <name>` filters)

- `renderable`: `render_pipeline` (only integration binary; exercises
  `add_feature` / `HtmlPage::features()`), plus new `style_features_baseline`.
- `darkmatter/lib`: `browser_render`, `render_invariants`, `render_comparison`,
  `render_tree_roundtrip`, `link_interpolation_integration`,
  `disclosure_render_targets`, `prelude_exports`, plus new
  `style_features_baseline`.

### GitNexus upstream impact (blast radius)

Recorded via `impact({direction: "upstream", summaryOnly: true})` against the
darkmatter worktree index. **Phase 1 makes no production edits — only tests and
docs — so none of these gates fire in Phase 1.** They are recorded for the
phases that do edit each symbol; per the plan, stop for user review before
editing any HIGH/CRITICAL symbol.

| Symbol | File | Impacted | Direct | Risk |
|--------|------|---------:|-------:|------|
| `PageFeature` | `renderable/src/browser/feature.rs` | 0 | 0 | LOW |
| `Rendered` | `renderable/src/tree/error.rs` | 0 | 0 | LOW |
| `HtmlPage::render_head` | `renderable/src/html/mod.rs` | 23 | 2 | **CRITICAL** |
| `render_browser_node` | `renderable/src/tree/render/browser.rs` | 139 | 47 | **CRITICAL** |
| `render_browser_document` | `renderable/src/tree/render/browser.rs` | 18 | 18 | **HIGH** |
| `render_browser_document_html` | `renderable/src/tree/render/browser.rs` | 59 | 16 | **CRITICAL** |
| `render_tree_html_with_context` | `darkmatter/.../entrypoints.rs` | 44 | 3 | **CRITICAL** |
| `DarkmatterPage::render_to_browser` | `darkmatter/lib/src/layout/page.rs` | 33 | 29 | **HIGH** |
| `Link::to_html_with_popover` | `darkmatter/lib/src/render/link.rs` | 2 | 2 | LOW |
| `Mermaid::render_for_html` | `darkmatter/lib/src/mermaid/mod.rs` | 6 | 6 | MEDIUM |

`Writer::render_link`, `StreamWriter::write_link`, `Writer::render_code_block`,
and `StreamWriter::write_code_block` are private methods on `browser.rs`'s
`Writer`/`StreamWriter`; their blast radius is bounded by their public wrappers
(`render_browser_node` / `render_browser_document*`), already covered above.

Notes for later phases:

- The `render_browser_*` family is the single hottest cluster (CRITICAL). Adding
  a `features` field to `Rendered` and threading collection through these is
  additive at the type level (new field, preserved by `map`), which is why the
  `Rendered`/`PageFeature` targets themselves score LOW — the risk lives in the
  many callers that construct `Rendered { … }` literally and must be migrated so
  the field is initialized (compiler-enforced once the field is non-defaulted).
- `render_for_html` (MEDIUM, 6 direct, all inside the `Mermaid` module) confirms
  the retirement is contained — no external consumer.

### Characterization baselines added (Phase 1)

New network-free, deterministic baseline suites pin the current pre-implementation
behavior. Where existing tests already lock a behavior, they are cited rather than
duplicated.

- `renderable/tests/style_features_baseline.rs`:
  - low-level default browser Mermaid mode is `Code` (no `<pre class="mermaid">`,
    no `<script>`);
  - `Interactive` Mermaid emits `<pre class="mermaid">` **with no script anywhere**
    (the inert dead-end Phase 3/5 fixes);
  - a no-feature page head injects no feature assets;
  - `PageFeature` first-seen dedup rollup (fixture for the dedup criterion);
  - `Rendered::map` preserves the diagnostics side channel (guards the Phase 2
    `features` addition);
  - deterministic two-Mermaid-block `Document` fixture.
- `darkmatter/lib/tests/style_features_baseline.rs`:
  - `DarkmatterPage::render_to_browser` today renders a Mermaid fence as **code**,
    not interactive (pins the pre-Phase-5 default that flips to `Interactive`);
  - bare-body render carries no `.darkmatter-page` wrapper and no feature assets;
  - a `prompt='…'` structured link lowers to a plain `<a>` carrying an escaped
    `data-prompt` and **no popover markup / CSS** in the tree path (pins the
    pre-Phase-4 shape);
  - `Link::to_html_with_popover` escapes hostile prompt content;
  - terminal Mermaid default keeps the ` ```mermaid ` fence;
  - deterministic two-prompted-link and long/escaped-prompt fixtures.

Existing coverage relied on (not re-created):

- head order start + `features()` rollup: `renderable/tests/render_pipeline.rs`
  (`page_render_emits_doctype_charset_and_title`,
  `html_page_render_rolls_up_every_composition_channel`).
- fragment-vs-streaming byte parity: `renderable/src/tree/render/browser.rs`
  inline `document_html_*_parity` tests over `parity_corpus`.
- Mermaid mode matrix + escaping + Vector-degrade: `browser.rs` inline
  `mermaid_*` tests.
- Markdown / MarkdownPlus neutrality: existing snapshot suites
  (`render_comparison`, `disclosure_render_targets`, terminal snapshot suites).

### Deferred fixtures (require Phase 2 types)

The plan lists fixtures for "unresolved features" and "a synthetic head-link
feature". Both need `FeatureResolver` / `FeatureAssets` / `FeatureScript`, which
do not exist until Phase 2. They are intentionally **not** created in Phase 1 (a
test cannot reference an unbuilt type); they land with the Phase 2 resolver in
`renderable`. All Phase 1 fixtures are self-contained and compile against the
current tree.

## Phase 2 — Renderable Feature Resolution Model

The resolution model landed entirely in `renderable` (plus a downstream
constructor migration). No Darkmatter or `dmls` source changed.

### Public surface added (`renderable::browser::feature`)

- `PageFeature::Popover` variant + `PageFeature::name(self) -> &'static str`
  (stable kebab-case identity for diagnostics and the future
  `data-darkmatter-features` attribute).
- `FeatureAssets { css: Option<Cow<'static,str>>, js: Option<FeatureScript>,
  links: Vec<LinkTag> }` with `FeatureAssets::css(..)`. Intentionally **not**
  `Debug` — it carries a non-`Debug` `LinkTag`, so error tests match rather than
  `expect_err`.
- `FeatureScript::{Classic, Module}` + `render()` — `Module` emits
  `<script type="module">` so the Phase-5 ESM bootstrap is never mis-wrapped.
- `FeatureContext { color_mode, semantic_colors }` (`Default` = dark, empty).
- Object-safe `FeatureResolver` trait + `DefaultFeatureResolver` (Popover→CSS on
  Browser; `Ok(None)` for Markdown/MarkdownPlus/Terminal; `UnresolvedFeature`
  for any other Browser feature — Mermaid stays unowned here, Darkmatter's Phase-5
  resolver owns it).
- `FeatureResolveError::{UnresolvedFeature, HeadRequired}` — both name feature +
  target.
- Shared helpers: `dedup_features`, `resolve_features` (dedup→resolve, drops
  `Ok(None)`), `serialize_features_head` (per feature: link, `<style>`, script),
  and `serialize_features_body` (inline only; `HeadRequired` if any links). These
  are the single shared assembler Phase 3 wires into `render_head` and the
  streaming full-document path.

Re-exported from `renderable::browser`. `LinkTag::new(rel, href)` was added
(previously unconstructable) so a head-linked feature / test resolver can exist.

### Side channel + ownership

- `Rendered<T>` grew `features: Vec<PageFeature>`; `new` inits empty, `map`
  preserves it.
- `HtmlPage` and `BrowserRenderOptions` own an `Rc<dyn FeatureResolver>` +
  `FeatureContext`, defaulting to `DefaultFeatureResolver`. `Rc` matches the
  existing `Rc<dyn CodeRenderer>` — these APIs are already not `Send`/`Sync`, so
  no expectation is weakened and no Darkmatter dependency is added. `HtmlPage`
  fields are stored-only in Phase 2 (`#[allow(dead_code)]`) with public
  `set_feature_resolver` / `set_feature_context`; Phase 3 consumes them.
- `RenderError::FeatureResolution(#[from] FeatureResolveError)` (`transparent`,
  so feature + target surface in the message).

### Constructor migration — Phase 1 inventory was incomplete

The Phase 1 side-channel inventory listed four direct `Rendered { .. }`
constructors (3 in `browser.rs`, 1 in `markdown.rs`). A workspace-wide grep
found a **fifth**: `biscuit-terminal/lib/src/render_tree/render.rs` (the terminal
node renderer). All five now initialize `features: Vec::new()`; only the browser
trio is populated in Phase 3 (Markdown and terminal stay empty by design).

## Phase 3 — Collect and Inject Features on Both Browser Pipelines

All source changes landed in `renderable` (no Darkmatter/`dmls` source changed).

### Request sites (collection)

- **Fragment `Writer`** (`tree/render/browser.rs`): `render_mermaid_interactive`
  calls `add_feature(PageFeature::MermaidDiagram)` on the `<pre class="mermaid">`
  fragment; `render_link` calls `add_feature(PageFeature::Popover)` when the link
  carries a Darkmatter-lowered `data-prompt` (detected by the new
  `link_requests_popover` helper — presence, not value). Code / static-SVG /
  plain-link branches request nothing.
- **`StreamWriter`** grew a first-seen `features: Vec<PageFeature>` accumulator.
  `write_mermaid_interactive` and `write_link` push at the same semantic branches
  in document order; `push_hook_fragment` merges a hook fragment's
  `collect_features()` at its document position, so nested/hook features are
  collected the same way the fragment path's page rollup does.
- **`BrowserFragment<Ready>::collect_features`** (new, `browser/fragment.rs`) is
  the single-fragment recursive rollup (own features + nested `Component`
  fragments, first-seen, deduped) — the analogue of `HtmlPage::features` used by
  `render_browser_node`.

### Side-channel population

- `render_browser_node` → `output.collect_features()`.
- `render_browser_document` → `page.features()` (unchanged rollup).
- `render_browser_document_html` → `dedup_features(&accumulator)`.

### Resolution + injection

- `HtmlPage` grew a `feature_head: String` plus `inject_resolved_features()`
  (`pub(crate)`), which resolves `self.features()` through the installed
  `feature_resolver`/`feature_context` for `RenderTarget::Browser` and stores the
  serialized head assets. `render_head` appends `feature_head` **after** the
  authored links/styles/scripts (step 8), so a no-feature page is byte-for-byte
  unchanged and `HtmlPage::render()` stays **infallible** (no blast radius into
  the component ecosystem's `render_html_page(...).render()` callers).
- `render_browser_document` installs `opts.feature_resolver`/`feature_context`
  on the page and calls `inject_resolved_features()?` — resolution failures
  surface at this fallible entry point (spec criterion 9), not in `render()`.
- `render_browser_document_html` resolves its accumulator directly with the
  render's resolver/context and appends the serialized assets after the shared
  `HtmlPage::render_head` output — the same helper and position as the fragment
  path, so the two paths stay byte-identical.
- Markdown / MarkdownPlus and terminal renderers request nothing and never
  resolve; their `features` stay empty.

### Key design ruling

The low-level `DefaultFeatureResolver` does **not** own `MermaidDiagram`
(Darkmatter's Phase-5 resolver does). Consequently, rendering an *interactive*
Mermaid diagram through a full-document path with the default resolver now
returns `RenderError::FeatureResolution(UnresolvedFeature { MermaidDiagram,
Browser })` instead of the pre-Phase-3 inert `<pre class="mermaid">`. This is
the intended criterion-9 behavior — a requested-but-unowned browser feature is a
hard error, not a silent inert element. The Phase 1 renderable baseline
`two_mermaid_document_injects_no_assets_on_either_path` was replaced by
`two_interactive_mermaid_are_unresolved_by_default_resolver`; the inline
`document_html_mermaid_mode_parity` gained a test-local `SyntheticFeatureResolver`.
`render_browser_node` does *not* resolve (fragment-level), so the many
`render_browser_node(...).output.render()` interactive-mermaid unit tests are
unaffected.

### Tests added (`browser.rs` inline)

- `two_feature_requests_inject_one_css_one_script_on_both_paths` (generic
  feature-asset dedup + byte parity via a synthetic CSS+JS probe resolver,
  criterion 10 — *not* production Mermaid CSS, which is script-only),
- `interactive_mermaid_unresolved_by_default_resolver_errors` (criterion 9),
- `non_interactive_mermaid_requests_no_feature` (Off / Code / Vector-static /
  Vector+Interactive degrade — criterion 4),
- `prompted_link_requests_popover_plain_link_does_not` (criterion 7 request +
  single-CSS injection + parity).

### Validation

`cargo nextest run -p renderable` → 531 passed. `cargo nextest run -p darkmatter`
→ 5625 passed (Darkmatter prompted-link baselines unaffected: their assertions
check body content / absence of `popover="hint"`/`interestfor=`, and injected
Popover CSS carries neither). `just lint` clean for `renderable` and
`darkmatter`. `cargo fmt --check -p renderable` shows only the pre-existing
local-rustfmt-vs-`main` drift in files this phase did not touch.

## Phase 4 — Accessible Prompted-Link Markup

All production source changes landed in `renderable`
(`tree/render/browser.rs` + a CSS refinement in `browser/feature.rs`); Darkmatter
changed only `render/link.rs` (standalone helper reconciliation) plus test flips.

### Emitted markup contract

A prompted link (a `NodeKind::Link` carrying the Darkmatter-lowered
`data-prompt` browser data attribute) now lowers to:

```html
<span class="dm-popover-wrapper">
  <a {existing attrs} href="…" [title="…"] interestfor="ID" aria-describedby="ID">…</a>
  <span id="ID" class="dm-popover-prompt" popover="hint" role="note">{escaped prompt}</span>
</span>
```

- The internal `data-prompt` transport is **consumed** — filtered out of the
  emitted anchor attributes (`prompted_anchor_attributes`) and never re-emitted.
- Every other existing link attribute (id, class, style, target, rel, download,
  other `data-*` / `aria-*`) and the real `href`/`title` are preserved.
- `interestfor` is the progressive-enhancement invoker where supported;
  `aria-describedby` is the always-on accessible association. Both name the same
  document-unique `id`.
- A plain link (no prompt) renders byte-identically to before and requests no
  feature.

### ID allocation

`PopoverIdAllocator` (private to `browser.rs`) is owned once per document render
by the fragment `Writer` and the streaming `StreamWriter`. The base is a readable
slug from the link target (`popover_id_base`: ASCII-alnum lowercased, other runs
collapse to `-`, bounded to 40 chars, `link` fallback); the first occurrence of a
base uses it verbatim, later occurrences append `-N`. Both writers walk links in
document order, so they derive identical id sequences and stay byte-parity.

### Popover CSS (`DefaultFeatureResolver`)

`POPOVER_CSS` is a CSS-only progressive enhancement (no JS): the prompt is
`visibility:hidden;opacity:0` and revealed on
`.dm-popover-wrapper:hover` / `:focus-within`, with a
`@media(prefers-reduced-motion:reduce)` override. Colors use the shared
`--color-bg`/`--color-fg`/`--color-border` semantic tokens with dark-safe literal
fallbacks. The prompt sets an explicit `display:block` so the popover-supporting
UA `[popover]{display:none}` rule cannot defeat the `:hover`/`:focus-within`
fallback (author rules beat the UA sheet). `max-width:20rem` + `left:0;right:auto`
keeps the panel viewport-safe for inline links.

### Standalone helper reconciliation

`Link::to_html_with_popover` (and its `to_browser_with_popover` alias) had **no
tree-pipeline caller** — a stale standalone shape. It now returns
`Option<String>` producing the same canonical wrapper/anchor/prompt structure the
render-tree path emits (wrapper span, `interestfor` + `aria-describedby`,
`popover="hint"` prompt span), so public docs/tests no longer advertise a shape
divergent from production. Signature changed from `Option<(String, String)>`;
only its own module + self-test referenced it.

### Tests

- `renderable` inline (`browser.rs`): `prompted_link_emits_accessible_popover_markup`,
  `repeated_prompted_links_get_unique_ids` (byte parity + unique ids),
  `prompted_link_escapes_hostile_prompt`,
  `prompted_link_preserves_navigation_attributes`; the pre-existing
  `prompted_link_requests_popover_plain_link_does_not` (no-prompt/no-CSS, single
  CSS block, CSS-only) still holds.
- `darkmatter` baseline flips: `prompted_link_tree_path_emits_accessible_popover`
  and `repeated_prompted_links_get_unique_ids` replace the pre-Phase-4 "no
  popover today" freezes; the `char_structured_link_attributes` characterization
  snapshot and `as_html_preserves_structured_link_metadata` were updated to the
  consumed-prompt contract.
- `darkmatter` browser tier: `browser_prompted_link_popover_reveals_on_focus`
  (real headless Chrome, see Cross-browser verification below).

### Cross-browser verification

**Chromium (headless, automated).**
`browser_render::browser_prompted_link_popover_reveals_on_focus` drives real
headless Chrome through the `biscuit-browser-harness` (browser tier — skips
cleanly when no Chrome is present, network-free) and asserts against *computed*
styles / live focus, not HTML-source substrings:

- the prompt's computed `display` is `block` — our author rule overrides the
  popover-supporting UA `[popover]{display:none}` rule, so the CSS fallback is
  never defeated;
- the prompt is `visibility:hidden` by default and becomes `visible` once the
  anchor is keyboard-focused (`:focus-within`, no JS);
- the anchor keeps its real `href` and the `aria-describedby` association names
  the prompt element's `id`.

Portable markup unit assertions in `renderable` additionally pin the exact
attributes/classes Firefox/WebKit rely on (`:hover`/`:focus-within` CSS
fallback, `interestfor`, `aria-describedby`, `popover="hint"`, escaping).

**Firefox / WebKit (manual checklist)** — no automation harness for these engines
exists in the repo; the CSS-only fallback is engine-portable, so verify manually:

1. Render any doc with a `prompt='…'` structured link to HTML
   (`md compose … | md render --html`, or `Markdown::as_html`).
2. Hover the link: prompt appears; Tab to the link: prompt appears
   (`:focus-within`); the anchor still navigates on click/Enter; with reduced
   motion enabled the fade is disabled.
3. Confirm the anchor stays navigable and the prompt is not clipped at a viewport
   edge for a long wrapping prompt.
4. Confirm no network request is made (the feature is inline CSS only).

## Phase 5 — Install Darkmatter Mermaid Assets and Body-Only Injection

Darkmatter's page browser path is now feature-aware: interactive Mermaid is
the default. On the body-only fragment path (`render_to_browser`) feature assets
are injected **into the page wrapper body**, not a document `<head>` — the shape
an embeddable fragment requires; the standalone document path
(`render_to_browser_document`) injects them into a real `<head>`.

### Body-only rendering model

The two public browser methods now have content-independent return shapes.
`DarkmatterPage::render_to_browser` **always** returns a body-only HTML fragment
(no `<!DOCTYPE>`/`<html>`/`<head>`/`<body>` scaffold): the bare body when the
page is undecorated and feature-free, or a forced single-element page wrapper
(`<div class="darkmatter-page">…</div>`) carrying the inline `<style>`/`<script>`
feature assets when decoration or a requested feature is present. It is the
method used to *embed* a render into a host document.
`DarkmatterPage::render_to_browser_document` **always** returns a complete
standalone `<!DOCTYPE html>` document with a real `<head>` (charset, viewport,
title, design-token/panel CSS, page metadata/stylesheets, and head-serialized
feature assets) around a wrapper-only `<body>`; the `md` CLI
(`darkmatter/cli/src/artifact.rs`) uses this method for HTML artifact output. In
both methods the page collects feature *requests* and resolves them at this outer
boundary — the body-only path places the resulting inline `<style>`/`<script>`
before the body fragment so the wrapper stays self-contained, while the
standalone path serializes them into the real `<head>` (where a head-only
`<link>` is legal).

### Renderable change (single, additive)

- `BrowserRenderOptions::defer_feature_injection: bool` (default `false`). When
  set, `render_browser_document_html` collects features into `Rendered::features`
  but does **not** resolve/inject them into `<head>`. Darkmatter sets it so the
  page owns placement; `Markdown::as_html` and all other renderable consumers keep
  head injection unchanged. This is the only renderable edit.

### DarkmatterFeatureResolver (`mermaid/feature.rs`, new)

- Owns `PageFeature::MermaidDiagram` on Browser → a script-only inline
  `<script type="module">` bootstrap. The palette (derived from the page's
  resolved `ThemePair` and color mode) rides Mermaid `themeVariables` baked into
  `mermaid.initialize`, not CSS — Mermaid does not read CSS custom properties, so
  a single fixed palette is emitted with no `prefers-color-scheme` switch.
  Delegates every
  other feature (Popover) and every non-Browser target to
  `DefaultFeatureResolver` (single-owner delegation, never a merged bundle).
- `MERMAID_VERSION = "11.6.0"` (exact, never a floating major tag). Bootstrap
  dynamically imports from `cdn.jsdelivr.net` (primary) then retries the identical
  version from `unpkg.com` (fallback), calls `mermaid.run({querySelector:
  '.mermaid'})` (initializes only `.mermaid`), and `console.error`s once on total
  failure while leaving the escaped `<pre class="mermaid">` source visible.
  `MERMAID_CDN_PRIMARY_ORIGIN` / `MERMAID_CDN_FALLBACK_ORIGIN` are public so the
  CSP/network contract is nameable.

### Entry-point wiring

- New `entrypoints::render_tree_html_page_body` → `BrowserPageRender { body,
  features }`. It maps Mermaid mode for the page path (**`Off` → Interactive**,
  `Image` → StaticSvg, `Text` → Code), threads `graphics_mode` (page
  `image_mode == Never` → `GraphicsMode::Off` → code; the streaming writer still
  caps `Vector` → static SVG), installs the resolver + `FeatureContext`, and sets
  `defer_feature_injection`.
- `render_to_browser` now routes **both** the default-layout and decorated paths
  through this feature-aware entry point (the old `is_default_layout` →
  `md.as_html` branch is gone), so a bare `mermaid` fence is interactive with
  default settings. For a feature-free doc the body is byte-identical to the old
  `md.as_html` output (no features → deferral and resolver are no-ops).

### Body-only injection (`page.rs`)

- `wrap_browser_html` gained `features` + `feature_assets` params: a
  feature-bearing render stamps `data-darkmatter-features="<space-separated
  names>"` on the wrapper and emits the inline assets before the body. A
  requested feature **forces** the wrapper into existence even when no decoration
  is configured.
- `resolve_feature_body_assets` resolves the collected features via
  `serialize_features_body`, so a feature resolving to a `<head>` `<link>` fails
  with `PageRenderError::FeatureResolution` wrapping `HeadRequired` (new error
  variant).

### Retirement

- Deleted `mermaid/render_html.rs` (the dead `MermaidHtml` struct +
  `generate_css_variables`), `Mermaid::render_for_html`, the `MermaidHtml`
  re-export, and their tests + `insta` snapshots. `detect_diagram_type` (still
  used by `Mermaid::alt_text`) was relocated into `mermaid/mod.rs` and kept
  public. Doc references swept: `lib/README.md`, `mermaid/README.md`,
  `docs/rendering/mermaid.md`, and the `darkmatter` skill's `terminal.md`.

### Tests

- `tests/style_features_baseline.rs`: the pre-Phase-5 "mermaid defaults to code"
  freeze flipped to `full_page_browser_mermaid_defaults_to_interactive`.
- `tests/style_features_phase5.rs` (new): dedup (2 fences → 1 module script,
  no Mermaid CSS, with the palette carried by `themeVariables`), body-only
  popover wrapper (CSS before body, injected once), feature-free no-wrapper,
  MarkdownPlus neutrality, explicit `Text` → code, `image_mode=Never` → code,
  and a durable `insta` snapshot of the exact injected Mermaid assets (CSP
  origins + exact version). Body-only `HeadRequired` and the empty-feature case
  are unit tests in `layout/page/tests.rs` (they need `pub(crate)` access).
- `mermaid/feature.rs` unit tests pin the resolver (mermaid → Module only, no
  CSS; palette via `themeVariables`; delegate Popover, asset-free off-Browser,
  CDN/version contract).

### Deviation

- The plan says "snapshots"; most Phase-5 checks are precise assertions because a
  full-document `insta` snapshot would bind the large Tailwind `:root` token
  preamble and be brittle. One focused snapshot pins the exact injected-asset
  wrapper prefix (the meaningful surface). All Phase-5 tests are network-free.

### Gates

`cargo nextest run -p darkmatter` (5609 + new pass; one pre-existing unrelated
flaky compose test), the browser tier (`-E test(/browser_/)`, 84 real-Chrome
tests pass), doctests, and `cargo clippy -D warnings` clean for
`darkmatter` / `darkmatter-cli` / `dmls` / `renderable`. `cargo fmt --check`
shows only the documented local-rustfmt-vs-`main` drift (present in untouched
code); `cargo fmt` was **not** run, per repo policy.

## Phase 6 — Regression, Documentation, and Handoff

Phase 6 is **documentation + verification only** — no production or test source
changed (`git status` shows only `.md` edits + the plan). Because the `README.md`
files are not `include_str!`-embedded into any crate, the doc edits cannot affect
any doctest, and every gate holds at the Phase-5 green state.

### Documentation updated

- `renderable/docs/tree-rendering.md` — new "Page features" section: request →
  collect → resolve/inject flow, resolver installation on
  `HtmlPage`/`BrowserRenderOptions`, first-seen link→CSS→script ordering,
  `Ok(None)` bypass vs. `UnresolvedFeature`/`HeadRequired` failures, and the
  fieldless-v1 dedup semantics + the forward-looking divergent-config activation
  point. (The `feature.rs` module rustdoc already documents the same contracts
  at the code level — added in Phases 2-5, unchanged here.)
- `darkmatter/docs/rendering/popover.md` — replaced the `Future` stub with the
  full prompted-link contract: emitted wrapper/anchor/prompt markup,
  `interestfor` + `aria-describedby` association, keyboard `:focus-within`
  fallback, `prefers-reduced-motion`, unique-ID allocation, the standalone
  `to_html_with_popover()` helper's `Option<String>` shape, a target-support
  table (browser-only; terminal/Markdown/MarkdownPlus receive no feature
  assets), and the Chromium-automated / Firefox-WebKit-manual verification.
- `darkmatter/docs/structs/Link.md` — corrected the stale `to_html_with_popover`
  example (was the pre-Phase-4 `(anchor, popover)` tuple; now `Option<String>`)
  and linked to `popover.md`.
- `darkmatter/docs/rendering/mermaid.md` — `blast_radius` swept (dropped deleted
  `render_html.rs`, added `mermaid/feature.rs` + `render_tree/entrypoints.rs`,
  repointed the removed CLI god-files to the current `args/`, `render.rs`,
  `commands/` layout), `last_updated` bumped, and the `MermaidMode` overview
  table split into Terminal vs. Browser columns to reflect the page-path remap
  (`Off → Interactive`, `Image → StaticSvg`, `Text → Code`) plus the low-level
  `renderable` `Code` default and the `as_html` `Off → Code` mapping.
- `darkmatter/lib/README.md` — un-"(future)"-ed Popovers with a real description
  + link, fixed the stale `mermaid-rendering.md`/`popovers.md` links, and
  corrected the Mermaid feature blurb ("Markdown as inline images" → unchanged
  `mermaid` fence; browser interactive client-side).

Darkmatter's Mermaid README (`lib/src/mermaid/README.md`) and the package skill
(`.claude/skills/darkmatter/terminal.md`) were already brought current in Phase 5.

### Source audits

- `grep MermaidHtml` → **zero** references anywhere.
- `grep render_for_html` → **zero** live references. One reference survives in
  `renderable/features/_completed/2026-06-02-non-structural/phase-3-notes.md`,
  an immutable historical archive of an unrelated completed feature (it
  documents `output/html.rs`, itself long deleted). Rewriting a `_completed`
  record would falsify history, so it is intentionally left as-is.
- All six live `Rendered { .. }` constructors initialize/propagate `features`:
  `error.rs` (`map`), `render/markdown.rs` (empty — Markdown neutrality),
  `render/browser.rs` ×3 (populated), `biscuit-terminal/.../render.rs` (empty —
  terminal neutrality).

### Gates re-run (Phase 6)

All `just all` tiers green for both packages (run tier-by-tier; no source
changed so this reconfirms the Phase-5 state):

- `renderable`: `just lint` ✓, `just doctest` (98) ✓, `just test` (520 passed,
  15 skipped) ✓. `test-l2`/`test-browser` are "not applicable" for renderable.
- `darkmatter`: `just lint` (lib+cli+dmls) ✓, `just doctest` (lib 177) ✓,
  `just test` (lib+cli green; dmls 502) ✓, `just test-l2` (3) ✓,
  `just test-browser` (84 real-Chrome, incl.
  `browser_prompted_link_popover_reveals_on_focus` and
  `full_page_browser_mermaid_defaults_to_interactive`) ✓.

Markdown/MarkdownPlus neutrality and terminal regression are covered by the
above suites (`render_comparison`, `disclosure_render_targets`, terminal
snapshot suites, `style_features_*`). No OS-specific paths/commands/line-endings
introduced (Phase 6 touched no code).

### GitNexus `detect_changes(compare, base_ref=main)`

`changed_count: 650, affected_count: 4, changed_files: 99, risk: medium`. The
compare is branch-wide (main predates several already-committed features), so
most changed `.rs` files (dmls semantic-tokens, schemas grammar, etc.) are
unrelated prior work, not Style Features. The Style-Features `.rs` set maps
**exactly** to the Phase 1 inventory — `renderable` {`feature.rs`,
`fragment.rs`, `html/mod.rs`, `html/tag/link.rs`, `tree/error.rs`,
`tree/render/browser.rs`, `tree/render/markdown.rs`}; `darkmatter`
{`mermaid/mod.rs`, `render/link.rs`, `layout/page.rs`, `layout/error.rs`,
`markdown/render_tree/entrypoints.rs`, `markdown/mod.rs`}; and
`biscuit-terminal/.../render.rs`. The 4 affected processes are all
`Run → Errors` / `Run → Has_errors` — the expected
`PageRenderError::FeatureResolution`/`HeadRequired` error wiring from Phase 5.
No unexpected flow.

### Acceptance-criteria → verification map

| # | Criterion | Verification |
|---|-----------|--------------|
| 1 | Dedup: two mermaid blocks → one injected module script (script-only; palette via `themeVariables`, no CSS) | `darkmatter` `style_features_phase5::two_mermaid_blocks_inject_one_module_script` (authoritative production dedup); generic CSS+JS dedup is covered separately by `renderable` inline `two_feature_requests_inject_one_css_one_script_on_both_paths` (criterion 10, synthetic probe) |
| 2 | Markdown neutrality (a snapshots unchanged, b no injected markup) | `render_comparison`/`disclosure_render_targets` snapshots unchanged; `style_features_phase5` MarkdownPlus-neutrality test; `render/markdown.rs` `features: Vec::new()` audit |
| 3 | Browser default = Interactive `<pre class="mermaid">` + ESM bootstrap | `full_page_browser_mermaid_defaults_to_interactive` (browser tier) + `style_features_phase5` asset snapshot |
| 4 | Compatibility defaults (default/terminal stay code, no I/O) | `non_interactive_mermaid_requests_no_feature` (renderable); `entrypoints.rs` mode-map unit tests; `image_mode=Never → code` test |
| 5 | Body-only render forces wrapper + inline assets | `style_features_phase5` body-only popover wrapper test; `layout/page/tests.rs` |
| 6 | Divergent config → hard error | **Deferred** — fieldless v1 cannot represent divergent config; activates with the first config-bearing feature (documented in `tree-rendering.md` + spec) |
| 7 | Popover: two links → one CSS, unique IDs, keyboard-reachable, plain fallback; no prompt → no CSS | `prompted_link_requests_popover_plain_link_does_not`, `repeated_prompted_links_get_unique_ids`, `prompted_link_emits_accessible_popover_markup` (renderable); `browser_prompted_link_popover_reveals_on_focus` (Chrome) |
| 8 | `MermaidHtml`/`render_for_html` retired | `grep` audit → zero live references |
| 9 | Resolver failures name feature+target; body-only link → `HeadRequired` | `feature.rs` `default_resolver_errors_on_unimplemented_browser_feature`, `body_serialization_rejects_head_only_links`; `layout/page/tests.rs` `HeadRequired` |
| 10 | Side-channel preservation (map, hook, streaming, first-seen) | `style_features_baseline` `Rendered::map` test; streaming/fragment parity tests; `collect_features` |
| 11 | Asset safety: escaped source/prompt, exact identical CDN version, readable fallback | `feature.rs` CDN/version unit tests; `prompted_link_escapes_hostile_prompt`; `style_features_phase5` CSP/version snapshot |
| 12 | Cross-platform + regression (no network, full-page + body-only snapshots, deterministic terminal) | All suites network-free; browser tier full-page + body-only; terminal snapshot suites deterministic |
| 13 | Docs cleanup (no `MermaidHtml`, resolver/CSP/default-split described) | This phase: `mermaid.md`, Mermaid README, `tree-rendering.md`, `popover.md`, `Link.md`, `lib/README.md` |
