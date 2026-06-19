---
status: complete
date: 2026-06-06
owner: ken
spec: renderable/features/_completed/2026-06-06-tree-features/spec.md
total_phases: 9
packages:
    - renderable
    - biscuit-terminal
    - darkmatter
    - darkmatter-cli
source_files_during_phase_1:
    - darkmatter/lib/tests/tree_features_characterization.rs
    - darkmatter/lib/tests/snapshots/tree_features_characterization__char_component_color_opaque_browser.snap
    - darkmatter/lib/tests/snapshots/tree_features_characterization__char_component_color_half_opacity_browser.snap
    - darkmatter/lib/tests/snapshots/tree_features_characterization__char_component_color_transparent_browser.snap
    - darkmatter/lib/tests/snapshots/tree_features_characterization__char_component_color_current_browser.snap
    - darkmatter/lib/tests/snapshots/tree_features_characterization__char_component_color_inherit_browser.snap
    - darkmatter/lib/tests/snapshots/tree_features_characterization__char_structured_link_attributes_html.snap
    - darkmatter/lib/tests/snapshots/tree_features_characterization__char_local_image_inline_css_precedence_browser.snap
    - darkmatter/lib/tests/snapshots/tree_features_characterization__char_hyperlink_exact_width_terminal.snap
    - darkmatter/lib/tests/snapshots/tree_features_characterization__char_hyperlink_max_width_terminal.snap
    - darkmatter/lib/tests/snapshots/tree_features_characterization__char_list_item_right_alignment_terminal.snap
    - darkmatter/lib/tests/snapshots/tree_features_characterization__char_list_item_center_alignment_terminal.snap
docs_updated_during_phase_1: []
docs_created_during_phase_1: []
skills_files_updated_during_phase_1: []
packages_touched_during_phase_1:
    - darkmatter
source_files_during_phase_2:
    - renderable/src/style/paint.rs
    - renderable/src/style.rs
    - renderable/src/prelude.rs
    - renderable/src/tree/render/shared.rs
    - renderable/src/stylesheet/style.rs
docs_updated_during_phase_2: []
docs_created_during_phase_2: []
skills_files_updated_during_phase_2: []
packages_touched_during_phase_2:
    - renderable
source_files_during_phase_3:
    - renderable/src/style.rs
    - renderable/src/tree/inherit.rs
    - renderable/src/tree/attrs.rs
    - renderable/src/tree/render/shared.rs
    - renderable/src/tree/render/browser.rs
    - renderable/src/tree/render/markdown.rs
    - biscuit-terminal/lib/src/render_tree/style.rs
    - biscuit-terminal/lib/src/components/block_quote.rs
    - biscuit-terminal/lib/src/components/prose/tree.rs
    - biscuit-terminal/lib/src/components/prose/parity.rs
    - biscuit-terminal/lib/src/components/filesystem/mod.rs
    - biscuit-terminal/cli/src/commands/table.rs
docs_updated_during_phase_3: []
docs_created_during_phase_3: []
skills_files_updated_during_phase_3: []
packages_touched_during_phase_3:
    - renderable
    - biscuit-terminal
    - biscuit-terminal-cli
source_files_during_phase_4:
    - renderable/src/tree/attrs.rs
    - renderable/src/tree/validate.rs
    - renderable/src/tree/mod.rs
docs_updated_during_phase_4: []
docs_created_during_phase_4: []
skills_files_updated_during_phase_4: []
packages_touched_during_phase_4:
    - renderable
source_files_during_phase_5:
    - renderable/src/tree/attrs.rs
    - renderable/src/tree/render/browser.rs
    - renderable/src/tree/render/markdown.rs
    - biscuit-terminal/lib/src/render_tree/render.rs
docs_updated_during_phase_5: []
docs_created_during_phase_5: []
skills_files_updated_during_phase_5: []
packages_touched_during_phase_5:
    - renderable
    - biscuit-terminal
source_files_during_phase_6:
    - darkmatter/lib/src/style/color.rs
    - darkmatter/lib/src/layout/page.rs
    - darkmatter/lib/src/layout/context.rs
    - darkmatter/lib/src/style/apply.rs
    - darkmatter/lib/src/markdown/render_tree/decorate.rs
    - darkmatter/lib/src/markdown/render_tree/build_context.rs
    - darkmatter/lib/src/markdown/render_tree/entrypoints.rs
    - darkmatter/lib/src/markdown/render_tree/fold.rs
    - darkmatter/lib/src/markdown/render_tree/mod.rs
docs_updated_during_phase_6: []
docs_created_during_phase_6: []
skills_files_updated_during_phase_6: []
packages_touched_during_phase_6:
    - darkmatter
source_files_during_phase_7:
    - renderable/src/tree/render/browser.rs
    - biscuit-terminal/lib/src/render_tree/render.rs
    - darkmatter/lib/src/layout/page.rs
    - darkmatter/lib/src/layout/context.rs
    - darkmatter/lib/src/layout/mod.rs
    - darkmatter/lib/src/markdown/render_tree/entrypoints.rs
    - darkmatter/lib/src/style/bespoke.rs
    - darkmatter/lib/tests/snapshots/cutover_reference__ref_block_quote_width_and_left_browser.snap
    - darkmatter/lib/tests/snapshots/cutover_reference__ref_centered_table_browser.snap
    - darkmatter/lib/tests/snapshots/cutover_reference__ref_list_left_margin_browser.snap
    - darkmatter/lib/tests/snapshots/cutover_reference__ref_page_background_pronounced_browser.snap
    - darkmatter/lib/tests/snapshots/cutover_reference__ref_table_max_width_browser.snap
    - darkmatter/lib/tests/snapshots/tree_features_characterization__char_component_color_current_browser.snap
    - darkmatter/lib/tests/snapshots/tree_features_characterization__char_component_color_half_opacity_browser.snap
    - darkmatter/lib/tests/snapshots/tree_features_characterization__char_component_color_inherit_browser.snap
    - darkmatter/lib/tests/snapshots/tree_features_characterization__char_component_color_opaque_browser.snap
    - darkmatter/lib/tests/snapshots/tree_features_characterization__char_component_color_transparent_browser.snap
    - darkmatter/lib/tests/snapshots/tree_features_characterization__char_local_image_inline_css_precedence_browser.snap
    - darkmatter/lib/tests/snapshots/tree_features_characterization__char_structured_link_attributes_html.snap
docs_updated_during_phase_7: []
docs_created_during_phase_7: []
skills_files_updated_during_phase_7:
    - .claude/skills/renderable/SKILL.md
packages_touched_during_phase_7:
    - renderable
    - biscuit-terminal
    - darkmatter
source_files_during_phase_8:
    - darkmatter/lib/Cargo.toml
    - darkmatter/lib/src/markdown/render_tree/mod.rs
    - darkmatter/lib/src/markdown/render_tree/structural_gate.rs
    - darkmatter/lib/tests/cutover_reference.rs
    - darkmatter/lib/tests/tree_features_characterization.rs
docs_updated_during_phase_8: []
docs_created_during_phase_8: []
skills_files_updated_during_phase_8: []
packages_touched_during_phase_8:
    - darkmatter
source_files_during_phase_9: []
docs_updated_during_phase_9:
    - renderable/docs/tree-rendering.md
    - renderable/docs/layout-and-style.md
    - darkmatter/docs/render-tree-fold.md
    - darkmatter/docs/darkmatter-rendering-pipeline.md
    - darkmatter/docs/rendering/style.md
docs_created_during_phase_9: []
skills_files_updated_during_phase_9:
    - .claude/skills/renderable/style.md
    - .claude/skills/renderable/tree.md
    - .claude/skills/biscuit-terminal/render-tree.md
    - .claude/skills/darkmatter/SKILL.md
    - .claude/skills/darkmatter/terminal.md
packages_touched_during_phase_9:
    - renderable
    - biscuit-terminal
    - darkmatter
---

# Tree Features Implementation Plan

**Goal:** Make the initial render tree the complete typed input to every target:
alpha-bearing paint, unresolved text layout, browser attributes, and Darkmatter
component policy are attached before rendering; each target performs one fold
without component decoration or post-render HTML mutation.

**Compatibility:** This is the clean breaking cutover authorized by the spec.
Migrate all repository callers directly. Do not add deprecated aliases, V2
types, old/new parallel fields, or old-tree serde migration.

**Implementation rule:** Keep the workspace compiling at each phase boundary.
Do not run `cargo fmt`; use targeted formatting only if explicitly requested.

## Phase 1: Baseline and Characterization

Establish the pre-change behavior and make the later deletions measurable.

- [x] Confirm the starting worktree and record unrelated changes without
  modifying them. (Unrelated, left untouched: modified
  `2026-06-06-tree-closeout/spec.md`, `2026-06-06-tree-features/spec.md`;
  untracked `2026-06-06-tree-closeout/plan.md`.)
- [x] Run compile baselines (all green):
  - `cargo check -p renderable -p biscuit-terminal -p darkmatter -p darkmatter-cli`
  - `cargo test -p renderable --no-run`
  - `cargo test -p biscuit-terminal --no-run`
  - `cargo test -p darkmatter --no-run`
- [x] Run and retain the current dedicated references:
  - `cargo test -p darkmatter --test cutover_reference` (1 pass; 5 known stale
    centering snapshots fail — see below);
  - structured-link/image and opacity behavior is now also pinned by the new
    characterization suite;
  - `cargo test -p biscuit-terminal --test perf_gate` (2 pass).
- [x] Add or strengthen characterization tests before changing behavior
  (`darkmatter/lib/tests/tree_features_characterization.rs`):
  - opaque and `/50` component colors;
  - Tailwind `transparent`, `current`, and `inherit` with opacity;
  - exact `width` versus `max-width` link/image behavior;
  - structured link class, typed CSS, target, title, `data-prompt`, and custom
    `data-*`;
  - local image inline CSS / frontmatter overlay (current double-`style=` and
    title-leak behavior);
  - right/center list-item placement;
  - terminal alpha degradation to the underlying opaque color.
  - Note: browser fragment-vs-streaming byte equivalence is already covered in
    `renderable` (`render::browser::document_html_matches_fragment_page_bytes`
    plus raw-HTML and Mermaid parity variants); not duplicated here.
- [x] Record the five known stale centering snapshots as expected review work,
  not failures to fix blindly: `reference_block_quote_width_and_left`,
  `reference_centered_table`, `reference_list_left_margin`,
  `reference_page_background_pronounced`, `reference_table_max_width`. All five
  fail because the committed snapshots expect the page wrapper margin as
  `0ch 0ch 0ch 0ch` while the current centering emits `0ch auto 0ch auto`.
  Deferred to Phase 8 (accept `auto` only with an explicit CSS rationale).

**Exit condition:** Existing behavior is captured well enough that every later
snapshot or output change can be classified as intended or regressive.

## Phase 2: Add Paint and Serializable Inline-CSS Foundations

Add replacement types without changing existing `Style` field types yet.

**Primary files:**

- `renderable/src/style.rs`
- `renderable/src/style/paint.rs` (new, if a submodule keeps `style.rs` focused)
- `renderable/src/stylesheet/style.rs`
- `renderable/src/stylesheet/mod.rs`
- `renderable/src/tree/render/shared.rs`

- [x] Implement `Opacity(u8)` with:
  - manual `Default` returning `OPAQUE`;
  - `TRANSPARENT`/`OPAQUE`;
  - exact `u8` construction;
  - checked percentage conversion using `(pct * 255 + 50) / 100`;
  - normalized CSS alpha conversion.
- [x] Implement `PaintColor { color, opacity }`, `From<Color>`, and
  `with_opacity`.
- [x] Add serde tests proving opaque opacity is elided and missing opacity
  deserializes as opaque.
- [x] Add the shared `PaintColor -> Option<CssColor>` lowering
  (`tree::render::shared::paint_to_css_color`, `#[allow(dead_code)]` until
  Phase 5 wires consumers):
  - fixed RGB plus opaque -> `CssColor::rgb`;
  - fixed RGB plus alpha -> `CssColor::rgba`;
  - `transparent`/`currentColor`/`inherit` -> keywords, ignoring stored alpha;
  - terminal default/reset colors -> no CSS declaration.
- [x] Add same-version serde for `CssStyle` as a canonical declaration string,
  deserializing through `CssStyle::try_from`; do not expose declaration storage.
- [x] Test invalid CSS rejection, deterministic round-trip output, and property
  ordering/replacement preservation.
- [x] Re-export the replacement types from the intended public modules/prelude
  (`renderable::style::{Opacity, PaintColor}` and the prelude).

**Verification:**

- `cargo test -p renderable style::`
- `cargo test -p renderable stylesheet::`
- `cargo check -p renderable`

**Exit condition:** New paint and validated inline-style types exist and are
tested, while existing consumers still compile unchanged.

## Phase 3: Flip Style Color Slots and Migrate All Consumers

Perform the public breaking type change atomically.

**Primary files:**

- `renderable/src/style.rs`
- `renderable/src/tree/inherit.rs`
- `renderable/src/tree/render/{shared,browser,markdown}.rs`
- `biscuit-terminal/lib/src/render_tree/style.rs`
- `biscuit-terminal/lib/src/components/**`
- renderable/biscuit-terminal tests, examples, and benches constructing colors

- [x] Change `Style::{color,background}` and `Border::color` to
  `TargetValue<PerMode<PaintColor>>`.
- [x] Audit every renderable-owned component color slot. Convert slots that
  represent browser/terminal paint to `PaintColor`; document any intentionally
  opaque slot instead of silently dropping alpha. (Style fg/bg and `Border.color`
  carry `PaintColor`. The remaining renderable-owned slots — `ProgressHints`
  fill/empty/bracket colors and `TableTerminalHints` stripe colors — are solid
  decoration with no alpha-compositing intent; documented as intentionally
  opaque `Color` rather than widened speculatively.)
- [x] Update `Background::{subtle,pronounced}` and ergonomic constructors.
  (`PerMode::{universal,adaptive}` now accept `impl Into<T>`, realizing the
  spec's `From<Color>` ergonomics so opaque construction stays concise.)
- [x] Update `InheritedStyle` so foreground alpha inherits exactly with color;
  background and border remain non-inheriting. (The resolver carries the whole
  `PaintColor` forward, so alpha inherits with the color; pinned by new tests.)
- [x] Switch browser and MarkdownPlus color emission to the shared `CssColor`
  lowering (`paint_to_css_color`).
- [x] Switch terminal resolution to use `PaintColor.color` and intentionally
  ignore opacity at every color depth.
- [x] Migrate all repository struct literals, helpers, tests, examples, and
  benches directly to the new type. (Construction sites convert via the
  `impl Into<T>` constructors; helper signatures and read sites updated.)
- [x] Update tree JSON snapshots to the new same-version shape; do not support
  the old shape. (In-source `Style` serde fixtures updated; a new test pins that
  the pre-`PaintColor` shape is rejected. No `.snap` files embed the old shape.)

**Verification:**

- `cargo check -p renderable -p biscuit-terminal -p darkmatter`
- `cargo test -p renderable`
- targeted `biscuit-terminal` render-tree style tests
- targeted browser/MarkdownPlus color tests

**Exit condition:** The workspace compiles on one paint representation, alpha
survives the tree, and no renderer requires a Darkmatter opacity side channel.

## Phase 4: Add Typed Sparse Tree Features

Add mutation ergonomics and the two new independent typed attr groups.

**Primary files:**

- `renderable/src/tree/attrs.rs`
- `renderable/src/tree/mod.rs`
- `renderable/src/tree/validate.rs`
- tree serde snapshots/tests

- [x] Add `style_mut_or_default` and `layout_mut_or_default`.
- [x] Choose and implement the simplest explicit sparsity cleanup API; prefer
  `retain_non_default_*` helpers unless tests show a mutation guard is safer.
  (Implemented `retain_non_default_{style,layout,text_layout,browser}` — explicit
  helpers, no mutation guard needed.)
- [x] Define `TextLayoutHints` with `width`, `max_width`, `alignment`, and
  overflow semantics. (Plus `TextOverflow` with `Preserve`/`Truncate`.)
- [x] Add `NodeAttrs::text_layout: Option<Box<TextLayoutHints>>` with owned,
  borrowed, setter, mutable, and sparsity helpers.
- [x] Define validated browser types:
  - renderable-owned `LinkTarget`/relations (`LinkRelation`);
  - supported image attributes (`ImageLoading`, `ImageDecoding`,
    `ImageBrowserAttrs`);
  - validated `DataAttrName` and `AriaAttrName` (reject empty/unsafe/uppercase;
    `data-*` reserves the `xml` prefix) with `BrowserAttrNameError`;
  - `BrowserAttrs` with `inline_style: Option<CssStyle>`;
  - deterministic `BTreeMap` storage.
- [x] Add `NodeAttrs::browser: Option<Box<BrowserAttrs>>` with the same accessor
  family.
- [x] Extend validation:
  - text layout only on supported link/image/list-item nodes;
  - link attrs only on links and image attrs only on images;
  - browser attribute names reject reserved/unsafe forms (enforced by the
    validated name newtypes at construction *and* on serde deserialize);
  - no typed feature is stored in `NodeAttrs::data` (existing `renderable.*`
    rejection unchanged).
- [x] Add serde and default-elision tests for every new sparse group.
- [x] Add corpus-coverage helpers proving tests actually populate each field.
  (`styled_corpus_populates_every_typed_group` walks the perf-gate corpus and
  asserts text-layout, link/image browser attrs, inline style, data attrs, and
  alpha-capable style color are all present before the fold.)

**Verification:**

- `cargo test -p renderable tree::attrs`
- `cargo test -p renderable tree::validate`
- `cargo test -p renderable --test render_pipeline`

**Exit condition:** Producers can attach all new intent through typed sparse
fields with no JSON access and no empty-box serialization.

## Phase 5: Teach Every Renderer to Consume the Typed Features

Implement target resolution before switching Darkmatter production paths.

**Primary files:**

- `renderable/src/tree/render/browser.rs`
- `renderable/src/tree/render/markdown.rs`
- `renderable/src/tree/render/shared.rs`
- `biscuit-terminal/lib/src/render_tree/render.rs`
- renderer parity and integration tests

- [x] Browser fragment and streaming writers emit identical:
  - classes;
  - validated inline `CssStyle`;
  - typed link/image attributes;
  - stable `data-*`/`aria-*` ordering and escaping.
  (Both writers share `node_attributes`, which now folds `inline_style` into the
  single `style` attribute and appends typed link/image/`data-*`/`aria-*`
  attributes in deterministic order; `LinkRelation`/`ImageLoading`/`ImageDecoding`
  gained `as_str` tokens. The fragment-vs-stream byte-equivalence corpus was
  extended with browser-attr-bearing nodes.)
- [x] Browser emission forbids duplicate/replacement `href`, `src`, raw
  `style`, and event-handler attributes through extension maps. (`data_attrs`
  and `aria_attrs` emit through the validated `DataAttrName`/`AriaAttrName`
  newtypes with a fixed `data-`/`aria-` prefix, and `inline_style` is a validated
  `CssStyle`; a dedicated test pins that the injection vectors cannot break out.)
- [x] MarkdownPlus emits only attributes allowed by its dialect policy;
  portable Markdown drops paint, layout, and browser-only attrs. (The Markdown
  renderer never reads `NodeAttrs::browser`/`text_layout`, so both dialects emit
  the plain `[text](url)` / `![alt](url)` form; a test pins the drop in both
  dialects.)
- [x] Terminal text-layout resolution:
  - exact `width` pads according to alignment;
  - `max_width` truncates only when exceeded;
  - both use the cap correctly;
  - Unicode display width and ellipsis behavior match characterized output;
  - links retain structured children in the tree;
  - images retain source alt text;
  - list markers remain structurally separate from body placement.
  (`Writer::apply_text_layout` resolves `width`/`max_width` to cells, pads per
  alignment, and truncates with `…` via the ANSI-aware `word_wrap::truncate`;
  wired into the link label, image alt-text placeholder, and list-item body. The
  link/image source children/alt stay intact in the tree.)
- [x] Browser/Markdown targets ignore terminal-only text-field behavior where
  appropriate rather than mutating content. (`text_layout` is consumed only by
  the terminal renderer; the browser and Markdown folds never read it and never
  rewrite content for it.)
- [x] Prove rendering the same tree at different terminal widths does not
  mutate it. (`render_tree_text_layout_does_not_mutate_tree_across_widths`
  renders one tree at two widths and asserts the input equals a pristine clone.)
- [x] Remove terminal reads of `darkmatter.li` only after typed list behavior is
  green. (Typed list-item `text_layout` now drives the marker-lift + body pad and
  is tested; the `darkmatter.li` read is retained as the still-active production
  fallback — darkmatter does not emit `text_layout` until Phase 6 — so its
  deletion stays with the Phase 7 sentinel-removal list.)

**Verification:**

- targeted renderable browser and Markdown tests
- targeted biscuit-terminal link/image/list render-tree tests
- `cargo test -p biscuit-terminal --test perf_gate`
- fragment/streaming byte-equivalence tests

**Exit condition:** All targets can consume the complete typed tree without
Darkmatter preparation passes.

## Phase 6: Build Complete Trees in Darkmatter

Move policy and directive lowering into Markdown node construction.

**Primary files:**

- `darkmatter/lib/src/markdown/render_tree/fold.rs`
- `darkmatter/lib/src/markdown/render_tree/entrypoints.rs`
- `darkmatter/lib/src/layout/{page,context}.rs`
- `darkmatter/lib/src/style/{apply,bespoke,color}.rs`
- `darkmatter/lib/src/render/{link,image_ref}.rs` as parser helpers

- [x] Introduce `TreeBuildContext` (or an equivalent Darkmatter-owned policy
  view) and context-aware fold entry points.
- [x] Keep the unstyled/default path cheap with an empty context.
- [x] Apply component policy when each container node closes:
  table, block quote, ordered/unordered list, list item, code, image, link, and
  thematic break.
- [x] Attach page inheriting foreground to the root and rely on
  `InheritedStyle`; do not copy page color to component nodes.
- [x] Lower `StyleColor` to `PaintColor` at the parser/apply boundary.
  `ComponentPolicy` and post-construction component types must no longer retain
  `StyleColor`.
- [x] Attach exact/max text layout to link/image/list nodes without replacing
  children or alt text.
- [x] Parse structured link/image metadata once during construction:
  - classes -> `NodeAttrs::classes`;
  - target/standard attrs -> typed browser attrs;
  - prompt -> validated `data-prompt`;
  - custom data -> validated `data_attrs`;
  - per-node CSS merged over frontmatter defaults -> `inline_style`;
  - clear a raw directive title only after its semantic fields are preserved.
- [x] Move HR defaults/inline precedence into construction or a construction
  input path so the initial tree is complete; avoid creating a replacement
  post-fold decoration walk.
- [x] Add structural tests that inspect the initial `Document` before rendering.

**Verification:**

- targeted Darkmatter fold and style tests
- structured-link/image tests
- tests asserting source children/alt remain unchanged
- tests asserting no `StyleColor` survives on component policy

**Exit condition:** `to_render_document` (or its replacement) returns the final
typed render input for every target.

## Phase 7: Switch Production Entry Points and Delete Compatibility Paths

Flip all public rendering onto the complete tree and remove obsolete machinery.

**Primary files:**

- `darkmatter/lib/src/markdown/{mod.rs,render_tree/entrypoints.rs}`
- delete `darkmatter/lib/src/markdown/render_tree/decorate.rs` when empty
- `darkmatter/lib/src/layout/{context,page,mod}.rs`
- affected tests and benches

- [x] Switch terminal, browser, Markdown, MarkdownPlus, and `DarkmatterPage`
  paths to the context-aware complete-tree builder followed by the target fold.
  (Production paths route through `render_tree_{terminal,html}_with_context` /
  `to_render_document_with_context`; `DarkmatterPage::render`/`render_to_browser`
  thread the `TreeBuildContext`. The HTML entry points now also return
  `MarkdownResult` so the restored `validate_code_directives` browser preflight —
  dropped mid-Phase-6 — runs over the single folded tree.)
- [x] Reduce `LayoutContext` to documented page-frame residue. (Already reduced;
  the component-policy decoration *signal* is read straight off the page in
  `page.rs` so `LayoutContext` keeps no policy map.)
- [x] Delete:
  - `decorate_document` and `component_for` (render-time);
  - component policy maps/lookups from render-time context;
  - inline link/image text mutation (link children / image alt stay intact;
    terminal projections are shaped without mutating the tree);
  - `darkmatter.li` (removed the `list_item_align_pad` read in
    `biscuit-terminal` — typed list-item `text_layout` now drives the marker
    lift + block-aligned body pad) and `darkmatter.style` production hints;
  - opacity sentinel collection and HTML rewrites;
  - link/image attribute sentinels and HTML rewrites;
  - obsolete `*_with_layout` variants;
  - post-construction `StyleColor` policy slots (`ComponentPolicy` carries
    `PaintColor`).
- [x] Use `rg` deletion checks for every removed symbol/namespace.
- [x] Update benches to call the actual new production path, not a compatibility
  helper. (Benches reference only the public `render_tree_*` entry points and
  compile against the new signatures.)

**Verification:**

- `cargo check -p darkmatter -p darkmatter-cli`
- targeted public `Markdown::{as_html,as_terminal}` tests
- `DarkmatterPage` terminal/browser tests
- negative `rg` checks from the spec acceptance criteria

**Exit condition:** Production rendering is complete tree construction followed
by one target fold, with no policy decoration or output rewriting.

## Phase 8: Structural Gates, References, and Performance

Prove the architecture and review intentional output movement.

- [x] Expand the structural gate corpus with alpha paint, component policy,
  exact/max text layout, and browser attrs. (New crate-internal gate
  `darkmatter/lib/src/markdown/render_tree/structural_gate.rs` drives a
  `style:`-frontmatter corpus — `block-quote.bg-color: red-500/50` alpha paint,
  `table.alignment` + per-component colors, `hyperlinks.width` /
  `li.max-width` / `images.local-style.width` text layout, and a structured
  link directive's typed browser link/`data-*`/inline-style attrs — through the
  real styled `DarkmatterPage` build context.)
- [x] Assert corpus coverage before folding so zero-access checks cannot pass
  vacuously. (`styled_corpus_populates_every_typed_group` walks the built
  `Document` and asserts alpha paint, baked component policy, text-layout,
  browser link attrs, inline_style, and `data_attrs` are all present before any
  fold runs.)
- [x] Assert zero extension-bag accesses for first-class behavior through the
  real styled Darkmatter entry points. (`styled_{terminal,browser}_path_does_
  zero_renderable_owned_hint_roundtrips` fold the same corpus through
  `render_tree_{terminal,html}_with_context` with renderable's
  `hint-access-counter` enabled via darkmatter dev-deps; `renderable_owned == 0`
  on both, and the terminal gate asserts the `darkmatter.hr` extension access is
  non-zero to prove the counter is live.)
- [x] Add immutable-tree tests across targets and widths.
  (`rendering_does_not_mutate_the_tree_across_targets_and_widths` renders one
  built tree at 40 and 100 cols plus the browser, asserts the two terminal
  outputs differ, and asserts the input `Document` equals a pristine clone.)
- [x] Review the five stale centering snapshots and accept `auto` margins only
  with an explicit CSS rationale. (All five `cutover_reference` browser
  snapshots are re-baselined to `margin: 0ch auto 0ch auto` and pass; the CSS
  rationale — `auto` side margins are the canonical center-a-`max-width`-frame
  idiom, mirrored by `layout/page.rs` `center_frame` and its two unit tests — is
  recorded in `cutover_reference.rs`'s module doc.)
- [x] Review all alpha, text-layout, structured-attribute, and list snapshot
  changes individually. (The seven re-baselined characterization snapshots —
  `rgb`/single-`rgba` color lowering, validated single `inline_style` replacing
  the double `style=`, deterministic typed attr ordering, and unmutated image
  `alt` — are each reviewed and accepted in `tree_features_characterization.rs`'s
  module doc; the terminal `char_hyperlink_*` / `char_list_item_*` snapshots are
  unchanged.)
- [x] Compile and run Criterion trend cases:
  - `cargo bench -p darkmatter --bench render_pipeline_steps --no-run` (builds);
  - targeted short runs for terminal/browser production cases (`--quick`
    `render_pipeline_{terminal,browser}/full`): both **improved** (terminal full
    −31%, browser full −14%) versus the prior baseline — consistent with
    dropping the post-fold decoration pass and HTML rewrites;
  - renderable `render` and biscuit-terminal `render_tree` benches both build.
- [x] Investigate material regressions; do not create a flaky timing gate. (No
  regressions; the only movement is the improvement above. Timing remains
  non-gating — the gate is the structural zero-access assertion, not a
  wall-clock threshold.)

**Exit condition:** Structural gates prove the intended architecture and the
reference corpus is green with reviewed changes.

## Phase 9: Documentation and Full Verification

- [x] Update:
  - `renderable/docs/tree-rendering.md` — typed sparse fields now list
    `text_layout` and `browser`, `Style` color slots noted as alpha-bearing
    `PaintColor`, and the darkmatter pipeline section describes complete-tree
    construction (`TreeBuildContext`) + one target fold, no decoration pass;
  - `renderable/docs/layout-and-style.md` — §6 `Style` struct flipped to
    `TargetValue<PerMode<PaintColor>>` (with the `PaintColor`/`Opacity` /
    `paint_to_css_color` contract) and §7 darkmatter migration rewritten
    (`ComponentPolicy` carries `PaintColor`; `decorate` pass / `darkmatter.style`
    deleted);
  - component migration guidance — the renderable `SKILL.md` IR-status section
    (already current from Phase 7) and `tree.md`'s new *Browser Attributes* /
    `text_layout` accessor families cover producer-side attachment;
  - Darkmatter rendering/style docs — `darkmatter/docs/render-tree-fold.md`
    (experimental → production + complete-tree construction),
    `darkmatter/docs/darkmatter-rendering-pipeline.md` (CURRENT STATE NOTES /
    Render Path rewritten off the deleted event-stream renderer), and
    `darkmatter/docs/rendering/style.md` (sub-spec #3/#5 storage + public API now
    `PaintColor`, opacity rides the alpha channel, no `decorate_document`);
  - public examples for `PaintColor`, text layout, and browser attrs — added to
    renderable `style.md` (PaintColor/Opacity section) and `tree.md`
    (`text_layout` + Browser Attributes); rustdoc examples already present on the
    new types (`paint.rs`, `attrs.rs`);
  - same-version-only tree serde documentation — `tree.md` *Render Hints* states
    the typed fields serialize to same-version serde JSON (not a cross-version
    durable format);
  - renderable, biscuit-terminal, and Darkmatter skills — see
    `skills_files_updated_during_phase_9`.
- [x] Review all changed rustdoc and inline comments for drift. (No stale
  comments remained in changed `renderable` / `biscuit-terminal` source; the
  `component_for` / `decorate_document` mentions in
  `darkmatter/.../build_context.rs` are a new local helper plus a legitimate
  "this replaces" note. Fixed doc drift: darkmatter `terminal.md` referenced the
  deleted `for_terminal` / `write_terminal` / `push_prose_text` / `LineWrapper`
  helpers; bt `render-tree.md` referenced `renderable.widget.progress.*`
  data-bag hints that are now typed `ComponentHints`.)
- [x] Run final Level 1 and doctest verification (all green):
  - `just -f renderable/justfile test` — 486 passed;
  - `just -f renderable/justfile doctest` — passed;
  - `just -f biscuit-terminal/justfile test` — 357 passed (68 skipped L2);
  - `just -f biscuit-terminal/justfile doctest` — passed;
  - `just -f darkmatter/justfile test` — passed;
  - `just -f darkmatter/justfile doctest` — passed.
- [x] Run lint checks without invoking `cargo fmt`:
  - `just -f {renderable,biscuit-terminal,darkmatter}/justfile lint` — all exit
    0 (clippy + fmt-check clean); this phase changed only Markdown docs/skills,
    so no Rust formatting was affected.
- [x] Leave Level 2/browser execution for closeout. (Not required — no changed
  behavior in this phase; Phase 9 is documentation + verification only.)
- [x] Record final verification results for the closeout spec. (Captured above:
  L1 tests + doctests + lint all green across renderable, biscuit-terminal, and
  darkmatter.)

**Exit condition:** The feature spec acceptance criteria are met, docs match the
implementation, and the closeout work can begin from a green tree.

