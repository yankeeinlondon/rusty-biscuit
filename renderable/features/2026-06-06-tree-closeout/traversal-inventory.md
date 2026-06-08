---
status: complete
date: 2026-06-06
owner: ken
spec: renderable/features/2026-06-06-tree-closeout/spec.md
phase: 1
---

# Production-Traversal Inventory

Audit of every recursive tree traversal in the Darkmatter production
render pipeline **after** the construction fold, classified against the
closeout taxonomy:

1. the target renderer fold;
2. validation / diagnostics explicitly requested by the caller or
   required at an API boundary;
3. a transformation required by a documented public API;
4. obsolete policy / output preparation.

The production pipeline is `Markdown -> Document -> target fold`, where
the `Document` is built by
`darkmatter::markdown::render_tree::fold::fold_markdown_spanned_with_context`
and consumed by one of the four target entry points: Terminal
(`render_tree_terminal_with_context`), Browser
(`render_tree_html_with_context`), Markdown
(`render_tree_markdown_dialect`). The page-decorated entry points
(`DarkmatterPage::render` / `render_to_browser`) build a
`TreeBuildContext` and delegate to the same folds.

## Method

1. Identified the construction fold and every traversal it invokes, to
   separate "construction" from "after construction".
2. Traced each target entry point (`render_tree_{html,terminal,markdown}*
   _with_context`) from fold-completion through final output to find
   every recursive walk of the `Document` root.
3. Classified each by the taxonomy above.
4. Cross-checked the negative-search list (see
   [Obsolete Mechanism Checks](#obsolete-mechanism-checks)) to confirm
   no deleted traversal survives.

## Inventory

| # | Traversal | Location | Classification | Output effect | Disposition |
|---|---|---|---|---|---|
| C1 | Construction fold: `fold_markdown_spanned_with_context` (`fold.rs:518`) drives a `pulldown-cmark` event stream into a `RenderNode` tree. Per node, `apply_node_policy` (`build_context.rs:187`) attaches typed layout, paint (`PaintColor`), `text_layout`, structured browser attrs, and HR defaults **while the node is being built**, before it is pushed onto its parent. | `darkmatter/lib/src/markdown/render_tree/fold.rs:518`; policy application at `fold.rs:177,457,557,734,742` and `build_context.rs:187`. | Construction (not a post-construction traversal). | Produces the complete typed `Document`. | **Retain** — this is the single construction pass the architecture calls for. Listed for boundary clarity. |
| C2 | `apply_page_colors` (`build_context.rs`) writes page foreground/background onto the root after the fold completes. | Single root node — **not recursive**. | Construction (root-only assignment). | Page color inherits to descendants via `InheritedStyle`. | **Retain.** |
| T1 | `resolve_node_spans` (`fold.rs:941`) — a full recursive walk that rewrites every node's `SourceSpan` byte range from rewritten-source offsets back to original-source offsets. | `fold.rs:583` (and `fold.rs:487` on the non-context path). Runs once, after construction, only when the inline-extension rewriter (`rewrite_inline_extensions`) rewrote the source. | **(3) Transformation required by a documented public API.** | Rewrites only `node.span.location.bytes`; touches no `kind`, no `children`, no `attrs`. Diagnostic spans must point at the user's original source, not the rewriter's intermediate bytes — the fold parses the *rewritten* string so `{{!…!}}` envelope tokens can resolve, but public diagnostics and source-location APIs are documented against the original bytes. | **Retain.** The traversal mutates only span metadata; it is structurally incapable of changing output behavior. One-line rationale: *diagnostic provenance must survive the inline-extension rewrite.* |
| T2 | `validate_code_directives` (`entrypoints.rs:304`) — a full recursive walk over the folded `Document` root, browser-path only. Re-parses every fenced code block's DSL directive and returns the first fatal parse error. | `entrypoints.rs:274` (inside `render_tree_html_with_context`). Runs once, before the browser fold, only on the browser path. | **(2) Validation required at an API boundary.** | No output effect; on success it returns `Ok(())`, on failure it returns `MarkdownError::InvalidLineRange` and the fold is abandoned. | **Retain.** This restores the legacy `output::as_html` fatal-directive contract: a malformed code-block directive (e.g. `highlight=1-2-3`) is a hard browser-path error, not a silent degrade. The render tree's `TerminalCodeRenderer` degrades malformed directives to language-only metadata (matching the legacy terminal renderer's `unwrap_or_default`), so the browser preflight is the one guard. One-line rationale: *the browser-path fatal-directive contract is an API boundary the tree renderer does not enforce.* |
| T3 | The shared validation gate: `renderable::tree::validate::validate` (`validate.rs:196`) → recursive `walk`. Runs once per target fold entry point. | Browser: `browser.rs:286` (`validate_and_collect_diagnostics`); Terminal: `render.rs:99`; Markdown: `markdown.rs:92`. Each target entry calls it once on the root before folding. | **(2) Validation / diagnostics explicitly requested at the renderer API boundary.** | Produces warning-severity `Diagnostic`s (or a fatal `ValidationError` on error-severity findings). Does not mutate the tree. | **Retain.** This is the structural-integrity gate every target renderer runs. It is not disguised policy decoration — it enforces the typed-attr placement rules (e.g. `text_layout` only on link/image/list-item, browser attrs only on links/images, no `renderable.*` keys in `data`). One-line rationale: *renderer API boundary integrity gate, not output preparation.* |
| T4 | The target renderer fold — the single recursive output-producing walk. Browser: `StreamWriter::write` (`browser.rs`); Terminal: `Writer::render` (`render.rs`); Markdown: `render_markdown_document` (`markdown.rs`). | Browser `render_browser_document_html` (`browser.rs:215`); Terminal `render_terminal_document` (`render.rs:152`); Markdown `render_markdown_document` (`markdown.rs:150`). | **(1) The target renderer fold.** | Produces the final output string for the target. | **Retain** — this is the architecture's "one target fold". |
| T5 | `DarkmatterPage` page-frame lowering — the terminal row/column decoration and browser page-wrapper assembly performed by the page frame around the folded target output. | `darkmatter/lib/src/layout/page.rs` (`render` at `790`, `render_to_browser` at `909`) and `darkmatter/lib/src/layout/context.rs`. Operates on the folded output string / wrapper, not on the `RenderNode` tree. | **Page-frame assembler** (outside the document component tree — see the page-frame decision in Phase 2). | Outer page margin/padding, full-page background rows/wrapper, max-width centering, `PageBackground::Pronounced` code-theme contrast, browser page-wrapper metadata and stylesheet assembly. | **Retain, constrained** (Phase 2 decision target). It owns no component policy, inspects no component node kinds, and mutates no component content. |

## Obsolete mechanism checks

Explicit `rg` checks for every deleted traversal named by the spec
(acceptance criterion 3). Workspace-wide, `--type rust`:

| Mechanism | Result |
|---|---|
| `decorate_document` (or an equivalent post-fold component-lookup walk) | **Absent.** The only match is `build_context.rs:183`, a doc-comment on `apply_node_policy` noting it *replaces* the deleted `decorate_document` walk. `apply_node_policy` is a construction-time helper called inside the fold, not a post-fold traversal. |
| Component-policy `LayoutContext` traversal | **Absent.** `LayoutContext` (`context.rs:24-48`) holds only viewport/page concerns; it carries no component-policy map and performs no recursive walk. The `max_width` cap is gated purely on page-frame geometry (`DarkmatterPage::has_frame_geometry`, review-2 finding 2), and the captured-terminal color depth is selected only when the page *actually paints* construction-time color (`DarkmatterPage::paints_construction_color`, review-3 finding) — component-policy *presence* has no bearing on either width or capability selection. An unmatched policy threads the build context but changes neither the content-box width nor the renderer-wide color depth. Neither gate performs a tree traversal. |
| Opacity or attribute sentinel injection | **Absent.** See the [extension-hint inventory](extension-hint-inventory.md#negative-searches). Alpha rides the typed `PaintColor` channel; no sentinel collection pass exists. |
| Post-render HTML opening-tag mutation | **Absent.** `rg 'rewrite_html\|mutate_html\|post_render\|post_fold\|opening_tag_mutation'` returns nothing. The browser fold emits each opening tag once, with typed attrs + validated `inline_style` in place; there is no second pass. |
| Pre-render link/image text replacement | **Absent.** Link children and image alt are attached as typed attrs / structural children during construction. The terminal renderer shapes its projection (label width/alignment, `▉ IMAGE[alt]` placeholder) without mutating the tree; pinned by `render_tree_text_layout_does_not_mutate_tree_across_widths`. |
| Target-width-derived mutation of the source tree | **Absent.** Pinned by `rendering_does_not_mutate_the_tree_across_targets_and_widths` (darkmatter structural gate): one built tree rendered at 40 and 100 cols plus the browser, input `Document` equals a pristine clone. |
| `component_for` (render-time) | **Absent as a render-time traversal.** The name survives only as a construction-time helper (`build_context.rs:214`) that maps a `NodeKind` to its `PageComponent` inside the fold. It is not invoked after construction. |

## Summary of findings

### Finding 1 — production rendering is one construction fold + one target fold, plus two explicitly-required non-output traversals

The complete post-construction traversal set on the production path is:

- **T1** `resolve_node_spans` — diagnostic-span provenance (required by
  the public source-location API; touches only span metadata).
- **T2** `validate_code_directives` — browser-path fatal-directive
  guard at the API boundary (browser path only; no output mutation).
- **T3** the shared validation gate — renderer API integrity (every
  target; no output mutation).
- **T4** the target renderer fold — the single output-producing walk.
- **T5** the page-frame assembler — operates on the folded output,
  outside the component tree.

There is **no** post-construction traversal that mutates component
content, applies component policy, or rewrites output. The architecture
is `complete typed Document -> one target fold`, satisfying acceptance
criterion 3.

### Finding 2 — no obsolete preparation traversal survives

Every deleted mechanism in the spec's negative list is confirmed absent
in production. The `decorate_document` / `component_for` names survive
only as doc-comments pointing at the construction-time replacement.

### Finding 3 — the page frame is a genuine assembler, not a traversal

T5 (`DarkmatterPage` lowering) does not walk the `RenderNode` tree; it
operates on the already-folded output. Its retained responsibility is
viewport/page-level only. The constrained Option A decision is recorded
here and tested in Phase 2; see [Page-Frame Decision](#page-frame-decision).

## Page-frame decision

**Signed off: Option A — retain the slim page frame.**

The Phase 1 audit confirms the prerequisites the spec sets for Option A:

- `LayoutContext` (`context.rs`) carries no component policy and performs
  no tree traversal; it holds only `terminal_width`, page margin/padding,
  content/effective width, `has_layout`, `background_color`,
  `render_color_mode`, and `page_bg_color`.
- The page frame operates on the folded output string / page wrapper, not
  on the `Document` root.
- It does not inspect component node kinds or mutate component content.

The retained frame owns exactly the viewport-level responsibilities the
spec lists for Option A:

- terminal/page viewport width (`effective_width`, `max_width` cap);
- outer page margin and padding (`page_margin`, `page_padding`);
- full-page background rows/wrapper (`background_color`,
  `page_bg_color`);
- max-width centering (`center_frame` in `page.rs`);
- `PageBackground::Pronounced` code-theme contrast mode
  (`render_color_mode`);
- browser page-wrapper metadata and stylesheet assembly (the
  `render_to_browser` path).

Phase 2 added the focused tests proving the frame carries no component
policy and does not traverse/mutate document components
(`darkmatter/lib/src/layout/page.rs`):

- `page_frame_chrome_ignores_component_policy_content` — two pages with
  identical frame geometry but different component-policy *content*,
  rendering a document none of those policies match, produce byte-identical
  output. This pins that the frame chrome is independent of which component a
  policy targets or what it sets.
- `terminal_unmatched_policy_does_not_cap_width_to_captured_terminal`
  (review-2 finding 2) — with the captured terminal *wider* than the ambient
  width and a wrapping document line, an unmatched component policy on an
  otherwise zero-geometry page produces byte-identical output to a no-policy
  page. This pins that the `max_width` content-box cap is gated on page-frame
  geometry alone (`has_frame_geometry`), never on component-policy presence:
  the policy still threads the build context (so a *matched* policy bakes its
  attrs), but an unmatched policy cannot widen the content box.
- `terminal_unmatched_policy_does_not_flip_color_depth_for_unrelated_content`
  (review-3 finding) — with the captured terminal at a no-color depth distinct
  from the ambient depth, an unmatched colored component policy on an otherwise
  zero-geometry page produces byte-identical output to a no-policy page; a
  fenced code block keeps its syntax-highlight color. This pins that
  renderer-wide color-depth selection is gated on whether the page *actually
  paints* construction-time color (`paints_construction_color`), never on
  component-policy presence — closing the capability analog of the review-2
  width leak. The real-terminal Level 2 test
  `level2_unmatched_policy_keeps_code_color_in_real_terminal`
  (`darkmatter/lib/tests/level2_render_tree_terminal.rs`) verifies the same
  color survives a WezTerm pane.
- `page_frame_vertical_margin_only_wraps_component_body` — adding top/bottom
  margin to an otherwise identical page only prepends/appends blank rows; the
  folded component body (heading, block quote, list, code) stays
  byte-identical. This pins that the frame is a pure wrapper that never
  traverses or rewrites component content.

These join the `LayoutContext::from_page` signature guarantee (it accepts only
viewport geometry and page colors — no component-policy input) and the
structural-gate immutability proof. The audit found nothing that forces a move
to Option B; **Option A stands as implemented.**

Promotion note: Phase 2 also promoted the `darkmatter.hr.*` extension hints
(the sole first-class extension reader identified by the
[extension-hint inventory](extension-hint-inventory.md)) to the typed
`renderable::tree::ThematicBreakAttrs` field on `NodeAttrs::thematic_break`.
No shared renderer reads `NodeAttrs::data` for first-class output anymore;
the styled production path now performs zero extension-bag round-trips
(pinned by the darkmatter `structural_gate`).

## Scope note

This artifact satisfies acceptance criterion 3 ("Production rendering is
one complete tree build followed by one target fold, excluding explicit
validation and the documented page frame") and supplies the evidence for
the page-frame decision (acceptance criterion 4). The Phase 2 focused
tests and the `darkmatter.hr.*` hint promotion are now complete (see the
Page-Frame Decision section above).
