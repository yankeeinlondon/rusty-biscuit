---
agent: codex/
total_phases: 6
created: 2026-07-12
phase: 6
yolo: true
source_files_during_phase_1:
  - renderable/tests/style_features_baseline.rs
  - darkmatter/lib/tests/style_features_baseline.rs
docs_updated_during_phase_1: []
docs_created_during_phase_1:
  - darkmatter/features/2026-07-09-features/implementation-notes.md
skills_files_updated_during_phase_1: []
source_files_during_phase_2:
  - renderable/src/browser/feature.rs
  - renderable/src/browser/mod.rs
  - renderable/src/html/mod.rs
  - renderable/src/html/tag/link.rs
  - renderable/src/tree/error.rs
  - renderable/src/tree/render/browser.rs
  - renderable/src/tree/render/markdown.rs
  - biscuit-terminal/lib/src/render_tree/render.rs
docs_updated_during_phase_2:
  - darkmatter/features/2026-07-09-features/implementation-notes.md
docs_created_during_phase_2: []
skills_files_updated_during_phase_2: []
source_files_during_phase_3:
  - renderable/src/browser/fragment.rs
  - renderable/src/html/mod.rs
  - renderable/src/tree/render/browser.rs
  - renderable/tests/style_features_baseline.rs
docs_updated_during_phase_3:
  - darkmatter/features/2026-07-09-features/implementation-notes.md
docs_created_during_phase_3: []
skills_files_updated_during_phase_3: []
source_files_during_phase_4:
  - renderable/src/tree/render/browser.rs
  - renderable/src/browser/feature.rs
  - darkmatter/lib/src/render/link.rs
  - darkmatter/lib/src/markdown/mod.rs
  - darkmatter/lib/tests/style_features_baseline.rs
  - darkmatter/lib/tests/tree_features_characterization.rs
  - darkmatter/lib/tests/browser_render.rs
  - darkmatter/lib/tests/snapshots/tree_features_characterization__char_structured_link_attributes_html.snap
docs_updated_during_phase_4:
  - darkmatter/features/2026-07-09-features/implementation-notes.md
docs_created_during_phase_4: []
skills_files_updated_during_phase_4: []
source_files_during_phase_5:
  - renderable/src/tree/render/browser.rs
  - darkmatter/lib/src/mermaid/feature.rs
  - darkmatter/lib/src/mermaid/mod.rs
  - darkmatter/lib/src/markdown/render_tree/entrypoints.rs
  - darkmatter/lib/src/layout/page.rs
  - darkmatter/lib/src/layout/error.rs
  - darkmatter/lib/src/layout/page/tests.rs
  - darkmatter/lib/tests/style_features_baseline.rs
  - darkmatter/lib/tests/style_features_phase5.rs
docs_updated_during_phase_5:
  - darkmatter/features/2026-07-09-features/implementation-notes.md
  - darkmatter/lib/README.md
  - darkmatter/lib/src/mermaid/README.md
  - darkmatter/docs/rendering/mermaid.md
docs_created_during_phase_5: []
skills_files_updated_during_phase_5:
  - .claude/skills/darkmatter/terminal.md
source_files_during_phase_6: []
docs_updated_during_phase_6:
  - renderable/docs/tree-rendering.md
  - darkmatter/docs/rendering/popover.md
  - darkmatter/docs/structs/Link.md
  - darkmatter/docs/rendering/mermaid.md
  - darkmatter/lib/README.md
  - darkmatter/features/2026-07-09-features/implementation-notes.md
docs_created_during_phase_6: []
skills_files_updated_during_phase_6: []
packages_during_phase_6:
  - darkmatter
  - renderable
source_code:
  - renderable/src/browser/feature.rs
  - renderable/src/browser/fragment.rs
  - renderable/src/browser/mod.rs
  - renderable/src/html/mod.rs
  - renderable/src/html/tag/link.rs
  - renderable/src/tree/error.rs
  - renderable/src/tree/render/browser.rs
  - renderable/src/tree/render/markdown.rs
  - renderable/tests/style_features_baseline.rs
  - biscuit-terminal/lib/src/render_tree/render.rs
  - darkmatter/lib/src/mermaid/feature.rs
  - darkmatter/lib/src/mermaid/mod.rs
  - darkmatter/lib/src/render/link.rs
  - darkmatter/lib/src/markdown/mod.rs
  - darkmatter/lib/src/markdown/render_tree/entrypoints.rs
  - darkmatter/lib/src/layout/page.rs
  - darkmatter/lib/src/layout/error.rs
  - darkmatter/lib/src/layout/page/tests.rs
  - darkmatter/lib/tests/style_features_baseline.rs
  - darkmatter/lib/tests/style_features_phase5.rs
  - darkmatter/lib/tests/browser_render.rs
  - darkmatter/lib/tests/tree_features_characterization.rs
  - darkmatter/lib/tests/snapshots/tree_features_characterization__char_structured_link_attributes_html.snap
  - darkmatter/lib/tests/snapshots/style_features_phase5__mermaid_feature_assets.snap
documentation:
  - darkmatter/features/2026-07-09-features/implementation-notes.md
  - darkmatter/lib/README.md
  - darkmatter/lib/src/mermaid/README.md
  - darkmatter/docs/rendering/mermaid.md
  - darkmatter/docs/rendering/popover.md
  - darkmatter/docs/structs/Link.md
  - renderable/docs/tree-rendering.md
  - .claude/skills/darkmatter/terminal.md
packages:
  - renderable
  - darkmatter
  - biscuit-terminal
---

# Style Features Execution Plan

## Planning Decisions

- Use the specification's recommended progressive-enhancement popover design: preserve the anchor and `href`, wrap the anchor and prompt, expose the prompt through `:hover` and `:focus-within`, and retain `interestfor` plus `popover="hint"` where supported.
- Keep `PageFeature` fieldless in v1. Record the divergent-configuration contract in public documentation, but do not add unused request-comparison machinery until a configurable feature exists.
- Keep `BrowserRenderOptions::default()` and all terminal defaults non-interactive. Only Darkmatter's full-page browser entry points opt into interactive Mermaid rendering.
- Treat generated feature assets as assembler-owned output. Page-authored links, styles, and scripts remain in their current order; resolved feature assets follow in first-seen feature order and, per feature, in link, CSS, script order.
- Before editing any symbol below, run GitNexus upstream impact analysis and report its direct callers, affected processes, and risk. Stop for user review before editing any symbol reported HIGH or CRITICAL.

## Phase 1 — Freeze Contracts and Baselines

- [x] Inventory every direct `Rendered<T>` constructor, browser-render entry point, feature-producing fragment hook, Mermaid promotion branch, prompted-link branch, and relevant public re-export; record the file/symbol checklist in implementation notes so no side channel or API surface is missed.
- [x] Run GitNexus upstream impact analysis for `PageFeature`, `Rendered`, `HtmlPage::render_head`, `render_browser_node`, `render_browser_document`, `render_browser_document_html`, `Writer::render_link`, `StreamWriter::write_link`, `Writer::render_code_block`, `StreamWriter::write_code_block`, `DarkmatterPage::render_to_browser`, `render_tree_html_with_context`, `Link::to_html_with_popover`, and `Mermaid::render_for_html`; resolve ambiguous symbols with file paths and record the blast radius before edits.
- [x] Add or tighten characterization tests that pin current no-feature HTML head ordering, fragment-versus-streaming byte parity, `Rendered::map` diagnostics behavior, Markdown/MarkdownPlus output, low-level Mermaid code defaults, terminal Mermaid defaults, prompted-link escaping, and Darkmatter bare-body rendering.
- [x] Define unit-test fixtures for duplicate Mermaid requests, hook-fragment feature requests, unresolved features, a synthetic head-link feature, repeated identical prompted links, long/escaped prompts, dark/light semantic colors, and body-only output; keep all fixtures network-free and deterministic.
- [x] Validation checkpoint: run targeted baseline tests with `cargo nextest run -p renderable` and `cargo nextest run -p darkmatter --test <affected-test-binary>` filters, and confirm snapshots change only after the corresponding implementation task lands.

## Phase 2 — Build the Renderable Feature Resolution Model

- [x] Extend `renderable/src/browser/feature.rs` with `PageFeature::Popover`, stable feature names for diagnostics/debug attributes, `FeatureAssets`, typed `FeatureScript::{Classic, Module}`, `FeatureContext`, object-safe `FeatureResolver`, and `FeatureResolveError::{UnresolvedFeature, HeadRequired}` carrying feature and target context.
- [x] Implement `DefaultFeatureResolver` in `renderable` so Popover resolves to shared inline CSS for the Browser target, intentionally asset-free targets return `Ok(None)`, and every requested but unimplemented Browser feature returns `UnresolvedFeature`.
- [x] Add resolver/context ownership to `HtmlPage` and `BrowserRenderOptions` with `DefaultFeatureResolver` defaults; use shared ownership suitable for object-safe trait objects without adding a Darkmatter dependency or weakening cross-platform `Send`/`Sync` expectations already imposed by these APIs.
- [x] Add `RenderError::FeatureResolution` and lossless `From<FeatureResolveError>` conversion; update rustdoc and error tests to prove feature and target appear in actionable failures.
- [x] Extend `Rendered<T>` with `features: Vec<PageFeature>`; make `Rendered::new` initialize it and `Rendered::map` preserve diagnostics and features. Migrate every direct constructor in browser and Markdown renderers so compilation enforces complete side-channel handling.
- [x] Implement one shared first-seen deduplication and asset-serialization helper used by both `HtmlPage` and the streaming assembler; preserve authored head ordering, then serialize each feature's links, CSS, and typed script without wrapping ESM as classic JavaScript.
- [x] Add focused tests for default resolution, unresolved Browser features, intentional target bypass, classic/module script markup, deterministic first-seen order, authored-assets-before-feature-assets ordering, and `Rendered::map` preservation.
- [x] Validation checkpoint: run `cargo nextest run -p renderable`, then `cargo check -p darkmatter -p darkmatter-cli -p dmls` to catch downstream API construction failures before renderer integration.

## Phase 3 — Collect and Inject Features on Both Browser Pipelines

- [x] Update fragment-based `Writer` rendering so interactive Mermaid fragments request `PageFeature::MermaidDiagram`, prompted-link fragments request `PageFeature::Popover`, and code/static Mermaid/plain links request no feature.
- [x] Add a first-seen feature accumulator to `StreamWriter`; request features at the same semantic branches as the fragment writer and merge feature requests from code-renderer hook fragments at their document position.
- [x] Return collected features from `render_browser_node`, `render_browser_document`, and `render_browser_document_html`; ensure document-level collection includes nested fragment features and cannot be discarded when converting `Rendered<BrowserFragment>` to `Rendered<HtmlPage>` or final strings.
- [x] Resolve and inject assets exactly once in `HtmlPage::render_head` and the streaming full-document assembler using their configured resolver/context; bypass feature collection/resolution entirely on Markdown and MarkdownPlus paths.
- [x] Keep `HtmlPage::features()` as the observable first-seen rollup and add parity tests proving fragment and streaming paths emit byte-identical head/body output and feature order, including features contributed by nested and hook fragments.
- [x] Add negative tests proving `GraphicsMode::Off`, `GraphicsMode::Vector` static fallback, `BrowserMermaidMode::Code`, static SVG success/fallback, and unprompted links do not request or inject interactive assets.
- [x] Validation checkpoint: run `cargo nextest run -p renderable`, `cargo fmt --check -p renderable` in read-only mode, and `just lint` from `renderable/`.

## Phase 4 — Implement Accessible Prompted-Link Markup

- [x] Introduce a document-render-scoped deterministic ID allocator shared by fragment and streaming writers; derive readable stable bases from link data and add occurrence suffixes so repeated identical prompted links never duplicate IDs.
- [x] Update the shared render-tree link branches in `renderable/src/tree/render/browser.rs` to consume the Darkmatter-lowered `data-prompt` value, omit that internal transport attribute from emitted anchor markup, preserve all existing link attributes and real `href`, and emit the approved wrapper/anchor/prompt structure with escaped prompt content, `interestfor`, `popover="hint"`, and appropriate ARIA association.
- [x] Ensure the wrapper and prompt remain keyboard reachable through anchor focus and CSS `:focus-within`, while browsers lacking native interest/popover behavior still expose the prompt and leave the anchor fully navigable.
- [x] Implement Popover CSS through `DefaultFeatureResolver` using existing semantic color tokens where available, constrained width/stacking/viewport-safe placement, dark/light compatibility, and a `prefers-reduced-motion` override.
- [x] Reconcile `darkmatter/lib/src/render/link.rs::to_html_with_popover` with the canonical render-tree markup: either delegate to a shared pure markup helper or update/remove the stale standalone shape so public docs and tests do not advertise behavior different from production rendering.
- [x] Add unit and snapshots covering no-prompt/no-CSS output, two prompted links/one CSS block, repeated identical links/unique IDs, long wrapping prompts, hostile HTML characters, focus associations, ordinary navigation attributes, light/dark contexts, and reduced-motion CSS.
- [x] Add headless-browser coverage for Chromium behavior and portable markup assertions for Firefox/WebKit semantics; if only Chromium automation is available, document the manual Firefox and WebKit verification commands/checklist rather than adding platform-specific test assumptions.
- [x] Validation checkpoint: run targeted prompted-link unit/snapshot tests and `just test-browser` from `darkmatter/`; require zero network access from tests.

## Phase 5 — Install Darkmatter Mermaid Assets and Body-Only Injection

- [x] Add `DarkmatterFeatureResolver` in the Darkmatter library; resolve `MermaidDiagram` from the captured `ThemePair` (the resolved code theme plus `FeatureContext.color_mode`) — the page supplies an empty `semantic_colors` vector, so the palette derives from the `ThemePair`, not from `FeatureContext` semantic colors — delegate every other feature to `DefaultFeatureResolver`, and keep resolver composition single-owner rather than merging duplicate bundles.
- [x] Move the Mermaid browser **bootstrap script** ownership into `DarkmatterFeatureResolver` (script-only: the resolver sets `css: None` and emits no Mermaid CSS block; the palette rides Mermaid's own `themeVariables`, baked into the bootstrap's `mermaid.initialize` call); use one exact spec-owned Mermaid version for jsDelivr primary and unpkg fallback dynamic ESM imports, initialize only `.mermaid` elements, escape diagram source, and log a concise failure while leaving readable source visible.
- [x] Install `DarkmatterFeatureResolver` plus the page's resolved color mode (carried on a `FeatureContext { color_mode, semantic_colors: Vec::new() }`) on all Darkmatter browser entry points, and make Darkmatter's full-page browser path default to `BrowserMermaidMode::Interactive` while leaving `BrowserRenderOptions::default()`, Markdown-family output, and terminal behavior unchanged.
- [x] Preserve explicit browser Mermaid controls: `GraphicsMode::Vector` caps Interactive to static SVG, `GraphicsMode::Off` renders code, and explicit static/code modes remain opt-ins with their existing fallback and no-I/O behavior.
- [x] Carry resolved feature assets out of the Darkmatter browser render result so `DarkmatterPage::render_to_browser` can distinguish body HTML from inline assets without parsing generated HTML.
- [x] Extend `wrap_browser_html` and its caller so any requested feature forces the `.darkmatter-page` wrapper, adds stable `data-darkmatter-features="..."`, and emits inline feature styles/scripts before the body; reject any resolved `LinkTag` with `HeadRequired` rather than placing a head dependency in an embeddable fragment.
- [x] Retire `darkmatter/lib/src/mermaid/render_html.rs`, `MermaidHtml`, `Mermaid::render_for_html`, their re-exports, tests, and stale references after the resolver path has equivalent escaping, exact-version, fallback, theme, and readable-failure coverage.
- [x] Add full-page and body-only snapshots proving two Mermaid blocks inject exactly one module script and NO Mermaid CSS block (script-only: no `--mermaid-*` custom properties, no Mermaid stylesheet), default Darkmatter output is interactive, bare-body output is wrapped when features exist, `data-darkmatter-features` is stable, no-feature output retains prior bytes, and synthetic link assets fail with `HeadRequired`.
- [x] Add CSP/network contract assertions that generated markup names both required CDN origins and uses no floating version, while tests inspect strings only and never fetch remote assets.
- [x] Validation checkpoint: run `cargo nextest run -p darkmatter`, `just test-browser`, `cargo fmt --check -p darkmatter`, and `just lint` from `darkmatter/`; verify explicit terminal Image mode through existing deterministic tests without initiating network or subprocess work in default modes.

## Phase 6 — Regression, Documentation, and Handoff

- [x] Update Renderable browser-rendering rustdoc/docs with resolver installation, feature ordering, unresolved/head-required failures, fieldless-v1 dedup semantics, and the deferred activation point for divergent per-request configuration.
- [x] Update Darkmatter Mermaid README, browser/rendering docs, public rustdoc, and package skill guidance to remove `MermaidHtml`, explain the low-level Code versus Darkmatter Interactive defaults, identify exact CDN delivery and CSP/network requirements, and describe body-only asset placement.
- [x] Update prompted-link documentation with the wrapper/ARIA/keyboard contract and progressive fallback; explicitly state that Markdown, MarkdownPlus, and terminal outputs receive no feature assets.
- [x] Audit behavior-changing comments and docs adjacent to every edited symbol; remove or correct stale claims while keeping cleanup limited to directly affected surfaces.
- [x] Run repository searches proving `MermaidHtml` and `render_for_html` have zero remaining code/doc references and that every `Rendered { ... }` constructor initializes or propagates features.
- [x] Run the complete package gates: `just all` from `renderable/`, then `just all` from `darkmatter/`. Record any environment-skipped browser harness separately and rerun with `BISCUIT_BROWSER_REQUIRED=1` in CI.
- [x] Run Markdown/MarkdownPlus snapshot suites and terminal regression suites explicitly; confirm feature work changes no Markdown-family bytes and introduces no OS-specific paths, commands, line endings, or runtime assumptions across macOS, Windows, and Linux.
- [x] Run GitNexus `detect_changes({ scope: "compare", base_ref: "main" })`; review affected symbols/processes against the Phase 1 checklist and investigate every unexpected flow before declaring completion.
- [x] Final validation checkpoint: map each of the 13 acceptance criteria to a passing automated test, snapshot, source audit, or documented cross-browser manual check; leave criterion 6 marked deferred until a config-bearing feature exists, as required by the specification.
