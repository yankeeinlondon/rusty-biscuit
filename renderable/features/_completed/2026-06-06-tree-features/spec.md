---
status: ready for planning and implementation
reviewed: true
date: 2026-06-06
owner: ken
parent: renderable/features/_completed/2026-06-04-css-box-architecture/spec.md
depends-on:
    - renderable/features/_completed/2026-06-04-style-vocabulary/spec.md
    - renderable/features/_completed/2026-06-04-tree-attrs/spec.md
    - renderable/features/_completed/2026-06-04-renderer-folds/spec.md
origin: renderable/features/_completed/2026-06-04-css-box-architecture/review-1.md
---

# Tree Features for Build-Time Policy and Typed Target Degradation

Small additions to the render tree that let producers build a complete styled
tree directly, while leaving target-dependent resolution to the renderer fold.
These features close the gaps identified by the first CSS Box Architecture
review without introducing another policy representation or generic styling
framework.

## Background

The render-tree cutover is complete at the output level: Darkmatter's public
terminal and browser paths build a `Document` and use the shared tree
renderers. The remaining architecture gap is that the first tree is not yet the
complete render input:

```text
Markdown
  -> Document
  -> decorate_document(LayoutContext)
  -> target fold
  -> browser-only opacity and attribute rewrites
```

Five limitations make those extra stages necessary:

1. `Style` carries `Color` but cannot preserve alpha. Darkmatter therefore
   keeps opacity in `StyleColor`, copies the opaque color into `Style`, writes
   the alpha-bearing CSS into `darkmatter.style` JSON hints, tags nodes with
   sentinel classes, and rewrites the rendered HTML.
2. Updating a sparse style or layout currently encourages a clone-modify-store
   sequence (`style()` then `set_style()`), which is awkward for a producer
   attaching policy while constructing many nodes.
3. Darkmatter's Markdown fold has no policy-aware construction context, so
   component policy is stored in `LayoutContext` and rediscovered through a
   second `NodeKind -> PageComponent -> HashMap` traversal.
4. Width-dependent hyperlink labels, image placeholders, and list-item
   alignment are represented by mutating content or writing package-local JSON
   hints before the renderer knows the target width.
5. Structured browser attributes on links and images are not complete typed
   tree input. Darkmatter tags nodes with sentinel classes, renders HTML, then
   rewrites opening tags to inject attributes such as `target`, `data-*`, and
   directive-derived metadata.

The problem is not that RGBA is inherently browser-specific. Alpha is
target-neutral paint intent with target-specific degradation. The complexity
arose because that intent was discarded at the canonical tree boundary and
had to be reconstructed after rendering.

## Thesis

> The tree stores complete, typed, unresolved presentation intent. Producers
> attach that intent while constructing nodes. Each renderer performs the
> target- and width-dependent resolution during its single fold.

The tree does not store terminal-sized output, browser CSS strings, resolved
padding, or pre-truncated labels. It stores enough typed information for each
target to make those decisions without consulting a component side-channel.

## Compatibility Contract

This feature is an intentional clean API cutover. The current tree-renderer
calling code is not live, so preserving the provisional Rust API or serialized
tree shape would add migration machinery without protecting a real user.

The implementation may therefore make direct breaking changes to:

- public Rust field types such as `Style::color`, `Style::background`, and
  `Border::color`;
- public component-hint structs and enums;
- render-tree serde shapes affected by those type changes;
- internal renderer and producer entry points;
- Darkmatter's internal layout and policy types.

Do not add deprecated aliases, duplicate `StyleV2`/`ColorV2` types, parallel
old/new fields, compatibility enum variants, or automatic migration code for
the previous tree JSON shape. Compiler errors should identify and drive the
finite in-repository call-site migration.

The compatibility requirements that remain are:

- Darkmatter `style:` v1 frontmatter input continues to parse with the same
  documented meaning.
- Opaque colors retain their intended rendered appearance, subject to reviewed
  reference changes from the broader CSS box cutover.
- Markdown's documented target degradation remains unchanged.
- Existing public CLI behavior is preserved unless a change is explicitly
  reviewed and documented.

Render-tree serde remains same-version serialization for debugging, inspection,
and transient persistence. It is not a durable cross-version interchange
format, and this feature does not deserialize the pre-`PaintColor` shape.

## Goals

- Preserve color alpha in first-class typed `Style` fields.
- Make incremental sparse-attribute construction ergonomic and allocation
  conscious.
- Give Darkmatter a policy-aware tree-build path that attaches `Layout`,
  `Style`, and component hints as nodes are created.
- Represent width-dependent inline and list behavior as typed unresolved
  intent rather than content mutation or JSON hints.
- Represent supported browser link/image attributes as validated typed attrs
  consumed directly by the browser fold.
- Keep target degradation explicit:
  - Browser and MarkdownPlus preserve alpha where they emit CSS.
  - Terminal uses the underlying color and intentionally discards alpha.
  - portable Markdown continues to ignore paint and geometry.
- Delete the opacity rewrite, component-policy decorate traversal, and
  first-class style/layout behavior carried in Darkmatter extension hints.

## Non-Goals

- A generic visitor, middleware, or component-policy engine in `renderable`.
- Moving Darkmatter's `PageComponent` taxonomy into `renderable`.
- Resolving terminal width, truncation, alignment padding, or alpha compositing
  during tree construction.
- Adding arbitrary CSS declarations to `Style`.
- Adding an unrestricted arbitrary HTML-attribute map.
- Making portable Markdown approximate terminal or browser layout.
- Migrating terminal-specific components such as image-protocol emitters solely
  because they implement `TerminalRenderable`.
- Folding the retained `DarkmatterPage` page frame into the root node. That
  remains a separate architecture decision.

## Design

### 1. Alpha-bearing paint color

Add a target-neutral paint value in `renderable::style` or
`renderable::color`:

```rust
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct PaintColor {
    pub color: Color,
    #[serde(default, skip_serializing_if = "Opacity::is_opaque")]
    pub opacity: Opacity,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct Opacity(u8);
```

`Opacity` uses an integer representation so it remains `Eq`, deterministic,
and free of floating-point serialization ambiguity. `0` is transparent and
`255` is opaque. Construction validates or clamps only at explicit parsing
boundaries; the stored type must not permit an out-of-range value.

> **Reader note:** `Opacity`'s `Default` must be `OPAQUE` (255), not the
> derived numeric default of `0`. The `PaintColor.opacity` field is
> `#[serde(default, skip_serializing_if = "Opacity::is_opaque")]`, so an opaque
> color elides the field on write and a missing field is filled by
> `Default::default()` on read. If `Opacity` used the derived `Default` of `0`,
> every elided (opaque) color would deserialize as fully transparent — a silent,
> catastrophic "everything invisible" regression. Implement `Default` manually
> (or with `#[serde(default = "Opacity::opaque")]`) and add a serde round-trip
> test that an opaque `PaintColor` survives elision as opaque.

Provide:

- `Opacity::TRANSPARENT` and `Opacity::OPAQUE`;
- `Opacity::from_u8` and a percentage conversion for Darkmatter's `/50`
  syntax. The percentage conversion maps `0% -> 0` and `100% -> 255` exactly
  and rounds intermediate values to the nearest `u8`
  (`(pct * 255 + 50) / 100`). The percentage round-trip is therefore lossy for
  most values; this is documented and tested, and the canonical stored form is
  the `u8`, not the percentage;
- `From<Color> for PaintColor`, producing an opaque value;
- `PaintColor::with_opacity`;
- accessors for the underlying color and normalized CSS alpha.

Change the color-bearing `Style` fields to:

```rust
pub color: Option<TargetValue<PerMode<PaintColor>>>,
pub background: Option<TargetValue<PerMode<PaintColor>>>,
```

Opaque construction should remain concise in the new API through `From<Color>`
helpers and updated constructors such as `Background::subtle()`. This is
ergonomic support for the replacement API, not source compatibility with old
`Style` struct literals. Do not add alpha-bearing variants to `Color`: alpha is
an orthogonal paint property and must apply uniformly to RGB, web, Tailwind,
basic, adaptive, and target-specific colors.

`Border.color` and typed component color slots should use `PaintColor` where
their public semantics permit alpha. If a slot intentionally cannot preserve
alpha, that limitation must be documented and tested rather than silently
discarded.

#### Target lowering

- **Browser:** one shared helper lowers `PaintColor` directly to `rgb(...)`,
  `rgba(...)`, or the appropriate CSS keyword. No duplicate opaque declaration
  is emitted before an RGBA override.
- **MarkdownPlus:** uses the same CSS color lowering for supported inline
  styles.
- **Terminal:** resolves `PaintColor.color` through the existing capability
  degradation and ignores `opacity`. This is a documented degradation, not
  data loss during tree construction.
- **Markdown:** unchanged; paint is ignored.

The browser and MarkdownPlus lowering should share the color-to-CSS conversion
instead of maintaining separate RGB-only helpers.

> **Reader note:** the CSS alpha sink already exists. `renderable::stylesheet`
> provides `CssColor::rgba(r, g, b, alpha)` (`alpha` as `0.0..=1.0`). The shared
> `PaintColor`-to-CSS helper must build on `CssColor` rather than hand-format an
> `rgba(...)` string, so escaping, precision, and keyword selection stay in one
> place. `PaintColor`'s "normalized CSS alpha" accessor is the bridge: it
> converts the stored `u8` opacity to the `0.0..=1.0` value `CssColor::rgba`
> expects.

CSS keyword behavior must remain explicit. `transparent`, `currentColor`, and
`inherit` continue to lower as keywords; their stored opacity is ignored
because CSS cannot apply a color-channel alpha to `currentColor` or `inherit`
without changing semantics, and `transparent` is already fully transparent.
`DefaultForeground`, `DefaultBackground`, and `Reset` continue to produce no
browser color declaration. Do not substitute element-level `opacity`, which
would affect descendants and the whole box rather than one paint channel.

### 2. Mutable sparse-attribute access

Add direct mutation helpers to `NodeAttrs`:

```rust
pub fn style_mut_or_default(&mut self) -> &mut Style;
pub fn layout_mut_or_default(&mut self) -> &mut Layout;
```

The helpers allocate the existing `Box` only when the field is absent and then
return a mutable reference. Existing borrowed and owned accessors remain
available.

Also provide explicit cleanup helpers or document the caller contract for
restoring sparsity after mutation. The preferred API is:

```rust
pub fn retain_non_default_style(&mut self);
pub fn retain_non_default_layout(&mut self);
```

or an equivalent mutation guard that removes a newly created default value on
drop. The implementation should choose the simplest API that prevents routine
tree construction from serializing empty `Style` or `Layout` boxes.

These methods are convenience over typed fields, not a new attribute layer:
they must not access `NodeAttrs::data`, serialize values, or allocate lookup
keys.

The same lazy-allocate-then-retain-sparsity pattern applies to the new typed
sparse groups introduced in sections 4 and 5 (`text_layout` and `browser`).
Each of those fields should expose the equivalent `*_mut_or_default` accessor
and participate in the chosen sparsity-restoration mechanism, so the
policy-attach path in section 3 can construct every typed group through one
uniform idiom.

### 3. Policy-aware Darkmatter tree construction

Introduce a Darkmatter-owned build context passed into the Markdown-to-tree
fold:

```rust
struct TreeBuildContext<'a> {
    component_policies: &'a ComponentPolicies,
    hyperlink_policy: &'a HyperlinkPolicy,
    image_policy: &'a ImagePolicy,
}
```

The exact decomposition may follow existing Darkmatter types. The constraints
are:

- it is construction input, not renderer input;
- it contains unresolved policy expressed in renderable types;
- it is owned by Darkmatter rather than generalized into `renderable`;
- the ordinary no-style path can use an empty/default context cheaply.

When the fold creates or closes a node, it maps the semantic Markdown role to
the corresponding policy and writes attrs immediately:

```text
Table                 -> table policy
BlockQuote            -> block-quote policy
ordered List          -> ol policy
unordered List        -> ul policy
ListItem              -> li policy
Code                   -> code-block policy
Image                  -> image policy
Link                   -> hyperlink policy
ThematicBreak          -> hr policy
```

This mapping should live beside node construction and be expressed once.
Policy application copies typed values onto the node; it performs no target
width math and emits no CSS or ANSI.

Page-level inheriting text appearance is attached to the root. Normal renderer
traversal through `InheritedStyle` supplies fallback to descendants. Component
nodes should not receive copied page colors merely to simulate inheritance.

The resulting `Document` must be complete enough to pass directly to every
renderer. `LayoutContext` may survive only for the explicitly retained page
frame inputs that are outside the component tree.

### 4. Typed width-dependent text intent

Promote the remaining first-class width behavior out of Darkmatter JSON hints
and content mutation. Add the smallest typed hint vocabulary that describes
the unresolved operation, for example:

```rust
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct TextLayoutHints {
    pub width: Option<TargetValue<Length>>,
    pub max_width: Option<TargetValue<Length>>,
    pub alignment: Alignment,
    pub overflow: TextOverflow,
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum TextOverflow {
    #[default]
    Preserve,
    Truncate,
}
```

#### Node attribute placement

`TextLayoutHints` attaches to a node as a **new typed sparse field on
`NodeAttrs`**, mirroring the existing `style` / `layout` fields:

```rust
#[serde(default, skip_serializing_if = "Option::is_none")]
pub text_layout: Option<Box<TextLayoutHints>>,
```

with `set_text_layout` / `text_layout` (clone) / `text_layout_ref` (borrowed
hot path) accessors. It is **not** a `ComponentHints` variant and **not** a
`NodeAttrs::data` namespace.

> **Reader note:** `ComponentHints` is explicitly "at most one per node, matched
> to the node's `NodeKind`" (`attrs.rs:704-728`). Width-dependent text intent is
> orthogonal to the per-kind hint — a list-item node may carry both its list
> semantics and a text-layout hint — so folding it into `ComponentHints` would
> force a false mutual exclusion. `NodeAttrs::data` is rejected because the
> structural performance gate asserts zero `renderable.*` extension-bag accesses
> for first-class behavior (`attrs.rs:782-815`); a typed sparse field reads with
> no `data` access and no key allocation, which is exactly what the gate
> requires. Keeping it a peer of `style` / `layout` also lets it reuse the
> mutable-access and sparsity-retention idiom from section 2.

The final shape may use narrower per-component structs if one shared type would
permit invalid combinations. It must cover the existing supported behavior:

- hyperlink-label exact width, maximum width, alignment, padding, and
  truncation;
- image alt/placeholder exact width, maximum width, alignment, padding, and
  truncation without replacing the source alt text;
- list-item body alignment relative to the marker and resolved item width.

`width` and `max_width` are not interchangeable: `width` establishes an exact
resolved field width and pads shorter content according to `alignment`;
`max_width` only truncates content that exceeds the cap and leaves shorter
content unchanged. When both are present, `width` is the requested field width
subject to the `max_width` cap.

The tree retains semantic source content. For example, an image node keeps its
real `alt` value while a typed image render hint requests the terminal block
placeholder. A link keeps its child nodes while a text-layout hint controls
terminal presentation.

Renderer responsibilities:

- resolve percentages and available width from the current fold context;
- measure visible rendered content using target-appropriate measurement;
- apply alignment and overflow after measurement;
- keep list markers structurally separate from list-item body placement;
- degrade unsupported behavior without changing the stored tree.

Do not use `Extended`, raw HTML, or a generic string map for these first-class
semantics.

### 5. Typed browser attributes

Promote supported link/image browser behavior into typed renderer input. The
tree must contain every attribute the browser renderer is expected to emit
before the fold begins.

Use existing `NodeAttrs::classes` for CSS classes. Add a sparse browser
attribute group for standard behavior and validated extension attributes, for
example:

```rust
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct BrowserAttrs {
    pub link: Option<LinkBrowserAttrs>,
    pub image: Option<ImageBrowserAttrs>,
    pub inline_style: Option<CssStyle>,
    pub data_attrs: BTreeMap<DataAttrName, String>,
    pub aria_attrs: BTreeMap<AriaAttrName, String>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct LinkBrowserAttrs {
    pub target: Option<LinkTarget>,
    pub rel: Vec<LinkRelation>,
    pub download: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ImageBrowserAttrs {
    pub loading: Option<ImageLoading>,
    pub decoding: Option<ImageDecoding>,
}
```

Like `text_layout`, `BrowserAttrs` attaches as a new typed sparse field on
`NodeAttrs`:

```rust
#[serde(default, skip_serializing_if = "Option::is_none")]
pub browser: Option<Box<BrowserAttrs>>,
```

with the same accessor and mutable-helper family, and the same prohibition
against living in `NodeAttrs::data`.

> **Reader note:** the inner `data_attrs` / `aria_attrs` maps are deliberately
> **not** named `data`. `NodeAttrs::data` is the opaque extension bag the
> performance gate watches; an inner `BrowserAttrs::data` field would invite
> confusion between "HTML `data-*` attributes" (first-class, typed, gate-clean)
> and "the extension bag" (untyped package metadata). The `_attrs` suffix keeps
> the distinction unambiguous in both code and serialized output.

The final type names and exact fields should cover current supported behavior,
not speculative HTML completeness. Standard attributes with behavior or safety
semantics are typed. Extensible `data-*` and `aria-*` attributes use validated
name types with deterministic ordering.

`inline_style` is a validated `renderable::stylesheet::CssStyle`, not a raw CSS
string. It preserves the existing structured-link/image
`style='…'` behavior and the merged `CommonStyle` overlay without permitting
unparsed CSS injection. Add same-version serde for `CssStyle` as its canonical
declaration string (deserialize through `CssStyle::try_from`) or introduce an
equivalent validated serializable wrapper; do not expose its private declaration
storage merely to derive serde.

Do not expose a general `BTreeMap<String, String>` for arbitrary HTML
attributes. The API must prevent bypassing typed URL/style policy with fields
such as `onclick`, a raw `style`, or a replacement `href`/`src`.

Darkmatter lowers directives while constructing the tree:

- `class` appends to `NodeAttrs::classes`;
- `target`, `rel`, and other supported standard fields become typed link attrs;
- supported image fields become typed image attrs;
- structured per-link/image CSS and frontmatter `CommonStyle` overlays merge
  into `inline_style`, preserving the existing rule that per-node declarations
  win property-by-property over frontmatter defaults;
- `data-*` and `aria-*` become validated entries;
- `prompt` lowers to the validated `data-prompt` entry, matching the existing
  public browser contract.

An extension hint must not be read by the shared browser renderer. If the
renderer interprets a value, that value belongs in typed renderer input.

The browser renderer owns attribute-name validation, value escaping, stable
ordering, duplicate handling, and URL/raw-HTML safety policy. The streaming and
fragment browser paths must emit identical attributes from the same tree.

Target degradation is explicit:

- **Browser:** emits all supported typed attributes directly.
- **MarkdownPlus:** may emit safe HTML when its existing dialect policy permits
  retaining the attributes.
- **Terminal:** ignores browser-only attributes while preserving link/image
  semantic content.
- **Markdown:** emits portable link/image syntax and drops unrepresentable
  browser attributes.

### 6. Deletion of compatibility paths

Once the typed features and build path are active, delete:

- `darkmatter.style` opacity hints;
- opacity sentinel classes and `StyleMerge`;
- `inject_component_color_opacity`, `collect_style_merges`, and
  `apply_style_merges`;
- opacity-specific post-render HTML mutation;
- component-policy fields and lookup helpers on render-time `LayoutContext`;
- `decorate_document` and `component_for`;
- decorate-time hyperlink/image text replacement;
- `darkmatter.li` alignment hints when their typed replacement is active;
- link/image attribute sentinel classes and pending-injection records;
- `inject_link_image_attributes`, `apply_attribute_injections`, and associated
  post-render opening-tag mutation;
- the `*_with_layout` entry-point split where it exists only to inject
  component policy after tree construction;
- `ComponentPolicy`'s retention of `StyleColor` color slots for the sole
  purpose of surviving opacity to the HTML target (`layout/page.rs:35-43`).
  Those slots lower to `PaintColor` during construction.

> **Reader note:** the retention of darkmatter's `StyleColor` in
> `ComponentPolicy::color` / `bg_color` (and the page color slots) exists *only*
> because renderable `Style` could not carry alpha — its own docstring says so.
> `PaintColor` removes that reason, so this collapse is the concrete proof that
> the alpha representation is unified. `StyleColor` may persist *upstream* as a
> `style:` v1 parser/input representation, but it must lower to `PaintColor` at
> tree-build time and must never be retained on policy or component types as a
> post-construction opacity carrier. Leaving the `ComponentPolicy` slots as
> `StyleColor` would preserve exactly the dual representation this feature
> exists to delete.

Package-local extension hints may remain only for semantics that are genuinely
package-specific and not part of this spec's first-class presentation model.

## Performance and Ergonomics

The changes should improve both construction and rendering:

- `PaintColor` remains a small copyable value.
- Sparse attrs allocate only when present.
- Mutable access avoids clone-modify-store cycles.
- Policy lookup occurs once when the relevant node is constructed, not once in
  a second whole-tree traversal.
- Renderer hot paths use borrowed typed attrs.
- Browser output is produced directly, without sentinel searches or repeated
  `String::replace_range`.
- Link/image attributes are emitted during the browser fold, not patched into
  completed HTML.

Extend the structural performance gate to exercise styled Darkmatter production
entry points with:

- component foreground and background colors carrying alpha;
- table, block-quote, list, link, and image policy;
- width-dependent text behavior.

The test must prove that first-class style, layout, semantic attributes, and
width behavior perform no extension-bag access during construction or
rendering. Because `text_layout` and `browser` are typed sparse fields rather
than `data` namespaces, reading them increments neither slot of the
`(renderable_owned, extension)` counter, so the gate's `renderable_owned == 0`
assertion holds by construction. To keep the assertion from passing vacuously,
the expanded corpus must actually populate and fold each typed group (alpha
paint, component policy, text-layout, and browser attrs); a corpus-coverage
check should confirm the relevant typed fields are present before the fold runs.
Criterion cases should retain trend visibility for the same corpus, but timing
remains non-gating.

## Testing

### Renderable

- `PaintColor` serde and opaque-default elision, including that an opaque value
  omits `opacity` on write and that a missing `opacity` deserializes back to
  opaque (not transparent).
- Exact conversion of `0`, representative intermediate values, and `255`.
- Opaque `Color` construction is ergonomic in the replacement API.
- the pre-`PaintColor` serialized `Style` shape is not accepted; a test or
  fixture documents the clean serde break.
- Browser and MarkdownPlus emit direct alpha-bearing CSS.
- Markdown ignores alpha-bearing paint.
- `InheritedStyle` preserves foreground alpha and does not inherit background.
- mutable attr helpers allocate lazily and preserve sparse defaults.
- validation accepts typed text-layout hints only on compatible node kinds.
- browser attrs validate placement, standard enum values, and `data-*` /
  `aria-*` names.
- `CssStyle`/the validated inline-style wrapper round-trips through the tree's
  same-version serde shape, and raw invalid CSS cannot be constructed.
- fragment and streaming browser folds emit identical typed attributes.
- unsafe or reserved arbitrary attributes cannot be constructed through the
  validated extension maps.

### Biscuit Terminal

- foreground and background alpha degrade to the underlying color at every
  supported color depth.
- alpha does not change width, wrapping, padding, border, or inheritance.
- link, image, and list text-layout hints resolve against the actual terminal
  width without mutating the tree.
- repeated renders of one tree at different widths produce different layout
  while the input tree remains equal.

### Darkmatter

- a styled document's initial built tree already contains all component
  `Layout`, `Style`, and typed hints.
- no call to a decorate function is required before terminal or browser
  rendering.
- opacity survives frontmatter parsing into the tree and direct browser output.
- link and image source content is unchanged in the tree.
- local/remote hyperlink and image policy is attached during construction.
- structured link/image directives are present as typed browser attrs on the
  initial tree.
- structured and frontmatter inline CSS is present as validated `inline_style`
  with per-node declarations winning over frontmatter defaults.
- list-item alignment uses typed hints rather than `darkmatter.li`.
- production entry points do not use component-policy `LayoutContext` fields,
  opacity hints, sentinel classes, or post-render style/attribute mutation.
- component policy and component types no longer retain `StyleColor` color
  slots; `style:` color/bg-color (including opacity) lowers to `PaintColor` at
  construction.
- existing `style:` v1 input remains accepted.

### Reference corpus

Review and re-baseline intentional output changes, including the five stale
browser-centering snapshots identified in `review-1.md`. Run the complete
Level 1 suites after re-baselining; the cutover reference suite must be green.

## Acceptance Criteria

1. `Style` preserves alpha through a first-class typed paint value.
2. Browser and MarkdownPlus lower alpha directly; terminal degradation is
   documented and tested.
3. `NodeAttrs` supports direct sparse style/layout mutation without JSON access
   or clone-modify-store boilerplate.
4. Darkmatter attaches component policy while constructing the tree.
5. A styled `Document` can be rendered directly by every target without a
   component-policy decorate traversal.
6. Hyperlink, image, and list width behavior is typed, unresolved tree intent;
   no source text is pre-rendered or replaced during construction.
7. `darkmatter.style`, opacity sentinels, HTML style rewriting, and
   `darkmatter.li` alignment hints are absent from production code.
8. Supported link/image browser attributes are typed tree input and are emitted
   directly by both browser folds.
9. Link/image attribute sentinels and post-render attribute injection are absent
   from production code.
10. `LayoutContext` carries only documented page-frame residue and no component
   policy map.
11. The structural performance gate covers the real styled Darkmatter path and
    reports zero extension-bag accesses for first-class style, layout,
    semantic-attribute, and width behavior.
12. The complete Level 1 suites and cutover reference suite pass.
13. No deprecated aliases, version-suffixed replacement types, parallel legacy
    fields, or old-tree serde migration code are introduced for this cutover.
14. Typed width-dependent text intent and browser attributes are stored as
    typed sparse `NodeAttrs` fields (peers of `style` / `layout`), not as
    `NodeAttrs::data` namespaces or `ComponentHints` variants, and elide to
    nothing when absent.
15. No `StyleColor` (or other non-`PaintColor`) color slot is retained on
    component policy or component types for the purpose of carrying opacity past
    tree construction.

## Sequencing

Land in slices that keep the workspace compiling:

1. Add `PaintColor` / `Opacity`, conversions, serde, and target lowering.
2. Change `Style`, border/component color slots, and all in-repository callers
   directly to the alpha-bearing type; do not retain a legacy API path.
3. Add mutable sparse-attribute helpers.
4. Add typed text-layout hints and renderer support.
5. Add typed browser attrs and direct browser/MarkdownPlus lowering.
6. Introduce the Darkmatter tree-build context and attach policy and directives
   during folding.
7. Switch production entry points to the completed tree.
8. Delete decorate-time policy, extension hints, opacity/attribute rewrites,
   and obsolete entry-point variants.
9. Re-baseline reviewed references, extend the structural gate, and run all
   Level 1 verification.

## Open Questions

1. Should `PaintColor` live in `renderable::color` as a general composited color
   value, or in `renderable::style` because alpha is paint intent rather than a
   terminal color identity? Prefer `style` unless non-style consumers need it.
2. Should `Opacity` store `u8` alpha or basis points? Prefer `u8`: it exactly
   represents hex alpha, stays compact, and is sufficient for CSS output.
3. Should sparse mutable attrs use explicit cleanup methods or a mutation guard?
   Prefer explicit helpers unless empty-box retention proves easy to misuse.
4. Can one `TextLayoutHints` type describe links, images, and list bodies
   without invalid states? If not, use narrow typed component hints rather than
   a permissive shared struct.
