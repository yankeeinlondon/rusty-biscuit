---
status: complete
date: 2026-06-06
owner: ken
spec: renderable/features/2026-06-06-tree-closeout/spec.md
phase: 1
---

# Extension-Hint Inventory

Audit of every production `NodeAttrs::data` namespace and every
`set_hint` / `get_hint` / `remove_hint` call against the closeout rule:

> If a shared renderer interprets the value or it changes first-class
> layout/style/semantic output, promote it to typed attrs. Keep extension
> hints only as opaque package metadata.

The audit covers the four Darkmatter production targets — **Terminal**,
**Browser**, **Markdown**, and **MarkdownPlus** — reached through the
`Markdown -> Document -> target fold` pipeline
(`render_tree_{terminal,html}_with_context`,
`render_tree_markdown{,_dialect}`).

## Method

1. `rg -n 'set_hint|get_hint|remove_hint'` across the workspace, then
   filtered to non-test, non-doc-comment production sites.
2. Cross-referenced producers (writers) against consumers (readers) for
   each namespace.
3. Confirmed whether each reader lives in a shared renderer
   (`renderable::tree::render::{browser,markdown}` or
   `biscuit_terminal::render_tree::render`) versus a construction-time or
   test-only caller.
4. Ran the negative-search list (see [Negative Searches](#negative-searches)).

## Inventory

| Namespace | Producer | Consumer | Supported node kinds | Shared renderer interprets it? | Affects first-class output? | Disposition |
|---|---|---|---|---|---|---|
| `darkmatter.hr.kind` | darkmatter `fold.rs:359` (`lower_hr_attrs_to_node`, inline `{:hr kind=…}` directive); darkmatter `build_context.rs:464` (`apply_hr_defaults` → `set_hint_if_absent`, frontmatter / `hr_defaults` option) | renderable browser renderer `browser.rs:546` (`GraphicsMode::Off` → `data-hr-kind` attr), `browser.rs:557` (`Vector`/`Rich` → SVG `style`); biscuit-terminal terminal renderer `render.rs:2027` (`horizontal_rule_from_attrs` → `RuleStyle`) | `NodeKind::ThematicBreak` only | **YES** (browser + terminal) | **YES** — selects the HR glyph / SVG primitive style | **PROMOTE to typed attrs** (Phase 2) |
| `darkmatter.hr.alignment` | `fold.rs:363` (inline directive); `build_context.rs:467` (defaults) | browser `browser.rs:546` (Off → `data-hr-alignment`); biscuit-terminal `render.rs:2039` (`RuleAlignment`) | `ThematicBreak` | **YES** (browser + terminal) | **YES** — first-class layout (rule horizontal placement) | **PROMOTE to typed attrs** (Phase 2) |
| `darkmatter.hr.weight` | `fold.rs:367` (inline); `build_context.rs:470` (defaults) | browser `browser.rs:558` (Vector/Rich SVG `--hr-weight`); `browser.rs:546` (Off → `data-hr-weight`) | `ThematicBreak` | **YES** (browser + terminal[1]) | **YES** — first-class paint (stroke thickness / row weight) | **PROMOTE to typed attrs** (Phase 2) |
| `darkmatter.hr.width` | `fold.rs:371` (inline); `build_context.rs:473` (defaults) | browser `browser.rs:559` (Vector/Rich SVG `--hr-width`); `browser.rs:546` (Off → `data-hr-width`) | `ThematicBreak` | **YES** (browser + terminal[1]) | **YES** — first-class layout (rule width) | **PROMOTE to typed attrs** (Phase 2) |
| `darkmatter.hr.color` | `fold.rs:375` (inline); `build_context.rs:476` (defaults) | browser `browser.rs:560` (Vector/Rich SVG `--hr-color`); `browser.rs:546` (Off → `data-hr-color`) | `ThematicBreak` | **YES** (browser + terminal[1]) | **YES** — first-class paint (rule color) | **PROMOTE to typed attrs** (Phase 2) |
| `renderable.layout.*`, `renderable.style.*`, `renderable.list.*`, `renderable.table.*`, `renderable.code.*`, `renderable.terminal.*`, `renderable.widget.{progress,columns,task}.*` | None in production — validator (`validate.rs:399`) rejects any `renderable.`-prefixed key in `NodeAttrs::data` as a stale renderable-owned hint. Used only in doc examples, serde round-trip tests (`attrs.rs:2460`+), and the `hint-access-counter` perf gate (`perf_gate.rs`) to exercise the namespace-classification path. | None in production — first-class behavior reads typed `NodeAttrs` fields (`layout()`, `style()`, `component()`, `text_layout()`, `browser()`). | n/a | **NO** — the validator makes a shared renderer read impossible | **NO** | **RETAIN** as the documented compatibility / testing surface. First-class behavior cannot regress: the validator rejects the keys and no accessor reads them. |
| `HintNamespace("myapp.custom")` (and any user-defined extension) | Example-only in `attrs.rs` rustdoc. | Example-only. | n/a | **NO** | **NO** | **RETAIN** — this is the intended opaque package-metadata escape hatch. |

[1] The terminal renderer reads `weight`, `width`, and `color` from the
    hint bag in `horizontal_rule_from_attrs`
    (`biscuit-terminal/lib/src/render_tree/render.rs:2017-2049+`); the
    whole `darkmatter.hr.*` family is consumed through one
    `horizontal_rule_from_attrs` helper, so weight/width/color are
    terminal-interpreted even though the off-mode browser path only
    surfaces them as `data-*` attributes. See the terminal renderer for
    the full read set.

## Markdown / MarkdownPlus reach

The Markdown renderer (`renderable/src/tree/render/markdown.rs:259`)
renders a `ThematicBreak` as the plain literal `---` and never reads any
`darkmatter.hr.*` hint. MarkdownPlus does not widen this. So the HR
extension hints affect only the Terminal and Browser targets; the two
Markdown dialects are clean today.

## Summary of findings

### Finding 1 — `darkmatter.hr.*` is the sole first-class extension hint

This is the **only** extension namespace a shared renderer interprets for
first-class output behavior. Every other `darkmatter.*` mechanism named
by the closeout spec is already deleted:

- `darkmatter.li` — deleted in `tree-features` Phase 7; only stale
  comment references remain (`entrypoints.rs:327`,
  `build_context.rs:198,334`, one characterization-test module doc).
- `darkmatter.style` — deleted in `tree-features` Phase 7; only a
  characterization-test module doc notes its removal.

The `darkmatter.hr.*` family is the single remaining blocker for
acceptance criterion 2 ("No shared renderer consumes extension data for
first-class style, layout, semantic attributes, or width behavior").

**Recommended resolution (Phase 2):** introduce a typed
`NodeAttrs::thematic_break` (or equivalent typed sparse group) modeled
on the existing `TextLayoutHints` / `BrowserAttrs` pattern, covering
`kind`, `alignment`, `weight`, `width`, and `color`. Convert the two
producers (`lower_hr_attrs_to_node`, `apply_hr_defaults`) and the two
shared-renderer consumers (`render_thematic_break`,
`horizontal_rule_from_attrs`) to the typed accessors, then delete the
`HintNamespace("darkmatter.hr")` usage from production. The validator
should gain a `darkmatter.` rejection (mirroring `renderable.`) or the
namespace simply falls out of production use.

**Resolution landed (Phase 2):** the typed
`renderable::tree::ThematicBreakAttrs` group (`kind` / `alignment` /
`weight` / `width` / `color`) was added on `NodeAttrs::thematic_break`,
with a validator placement rule restricting it to `ThematicBreak` nodes.
Both producers and **three** shared-renderer consumers (browser
`render_thematic_break` *and* the streaming `write_thematic_break`, plus
the terminal `horizontal_rule_from_attrs`) now read the typed field; no
production code calls `set_hint`/`get_hint` for `darkmatter.hr` anymore.
The namespace fell out of production use, so the validator's existing
`renderable.`-only rejection was left unchanged — `darkmatter.*` remains
a legitimate opaque-extension escape hatch. The darkmatter `structural_gate`
now asserts the styled production path performs **zero** extension-bag
round-trips (replacing the old "HR probe keeps the counter live" check
with an explicit post-render liveness probe).

### Finding 2 — `renderable.*` namespaces are correctly inert

The compatibility constants remain, but the validator rejects any
`renderable.`-prefixed `data` key at `Full` validation (which every
target fold runs). The `hint-access-counter` perf gate
(`biscuit-terminal/lib/tests/perf_gate.rs`,
`darkmatter/lib/src/markdown/render_tree/structural_gate.rs`) asserts
`renderable_owned == 0` on both the styled terminal and styled browser
production paths. No action required; documented here as a durable
record.

### Finding 3 — no other production extension namespaces exist

A workspace-wide search for `data.insert`, `HintNamespace("…")`, and
`get_hint` in non-test production code found no further extension
namespaces. The `ImageRef.data` / `LinkRef.data` fields in
`darkmatter/lib/src/render/{image_ref,link}.rs` are
`BTreeMap<String, String>` builder fields on parser helper structs, not
`NodeAttrs::data`; their contents are lowered to typed `BrowserAttrs`
during the `TreeBuildContext` construction fold
(`build_context.rs` `LinkDirective::apply_to_link_node` and the image
counterpart).

## Negative searches

Explicit `rg` checks for deleted mechanisms (run against the whole
workspace, `--type rust`):

| Mechanism | Command | Result |
|---|---|---|
| `decorate_document` | `rg -n 'decorate_document'` | One doc-comment in `build_context.rs:183` noting the post-fold walk was replaced by the construction-time `apply_node_policy`. No live function. |
| `component_for` (render-time) | `rg -n 'component_for'` | `build_context.rs:188,214` — a **construction-time** helper (`fn component_for(&NodeKind) -> Option<PageComponent>`) called inside the build-context fold, not a render-time traversal. This is the legitimate replacement; it maps a node kind to its policy component while the tree is being built. |
| `darkmatter.li` | `rg -n 'darkmatter\.li'` | Comment / doc references only (`entrypoints.rs:327`, `build_context.rs:198,334`, one test module doc). No live read or write. |
| `darkmatter.style` | `rg -n 'darkmatter\.style'` | Two characterization-test module doc lines noting its deletion. No live read or write. |
| Opacity / attribute sentinel injection | `rg -n 'sentinel' \| rg -i 'opacity\|inject\|link_image'` | No opacity or attribute sentinel injection. The `sentinel` hits are unrelated: U+FDD0 inline-envelope markers (`inline_extension.rs`, `markdown/inline/mod.rs`), `[render-tree error: …]` in-band error strings in biscuit-terminal components, and test `printf` completion sentinels. |
| Style/attribute merge functions | `rg -n 'apply_style_merges\|merge_style\|style_merge\|opacity_sentinel'` | One characterization-test doc reference (`tree_features_characterization.rs:10`) noting `apply_style_merges` is deleted. No live function. |
| Component-policy render context | `rg -n 'ComponentPolicy'` in `LayoutContext` | `LayoutContext` (`context.rs:24-48`) holds only viewport/page concerns: `terminal_width`, `page_margin`, `page_padding`, `content_width`, `effective_width`, `has_layout`, `background_color`, `render_color_mode`, `page_bg_color`. No component-policy map. |
| `inject_link_image_attributes` | `rg -n 'inject_link_image_attributes'` | One characterization-test doc line noting it is replaced by typed browser attrs. No live function. |
| Post-render HTML opening-tag mutation | `rg -n 'rewrite_html\|mutate_html\|post_render\|post_fold\|opening_tag_mutation'` | No matches. |
| Pre-render link/image text replacement | (covered) | Link children and image alt are attached as typed attrs / structural children during construction; the terminal renderer shapes its projection without mutating the tree (pinned by `render_tree_text_layout_does_not_mutate_tree_across_widths`). |
| Target-width-derived mutation of the source tree | (covered) | `rendering_does_not_mutate_the_tree_across_targets_and_widths` (darkmatter structural gate) renders one built tree at 40 and 100 cols plus the browser and asserts the input `Document` equals a pristine clone. |

## Scope note

This artifact satisfies acceptance criterion 1 ("Every production
extension hint is inventoried and classified") and supplies the
evidence for the criterion-2 finding (`darkmatter.hr.*`). The
**promotion** itself is Phase 2 work; Phase 1 only inventories and
dispositions.
