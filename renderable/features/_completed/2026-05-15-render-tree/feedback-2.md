# Render Tree Spec - Second-Pass Feedback

This second pass assumes the major direction is now sound. The revised spec
absorbed the big architectural concerns from the first review: vertical slicing,
terminal renderer placement, source provenance, explicit loss policy,
validation, and the `TreeRenderable` split all materially improve the design.

The remaining issues are more local, but they are still worth tightening before
implementation. Most are API-shape inconsistencies or places where the spec now
states the right concept but leaves the exact Rust surface ambiguous.

## Findings

### 1. Renderer return types are internally inconsistent

The "Options, context, and output" section shows these signatures:

```rust
pub fn render_markdown(node: &RenderNode, opts: &MarkdownOptions)
    -> Rendered<String>;
pub fn render_browser(node: &RenderNode, opts: &PageOptions)
    -> Rendered<BrowserFragment<Ready>>;
pub fn render_terminal(node: &RenderNode, ctx: &TerminalRenderContext)
    -> Rendered<String>;
```

Immediately after that, the strictness section says `render_*` returns
`Result<Rendered<T>, RenderError>`. The response file also says the final shape
is `Result<Rendered<T>, RenderError>`. The component integration example then
uses:

```rust
render_browser(&self.render_tree(), &PageOptions::default()).output
```

That only works if `render_browser` returns `Rendered<_>` directly, not if it
returns `Result<Rendered<_>, RenderError>`.

This is not just a documentation typo. The result shape determines whether
target trait impls can be one-line delegations, how strictness failures surface,
and whether existing infallible traits such as `BrowserRenderable` can delegate
without inventing a fallback.

Suggested fix: pick one canonical signature in the spec. Based on the rest of
the revision, it should probably be:

```rust
pub fn render_markdown(
    node: &RenderNode,
    opts: &MarkdownRenderOptions,
) -> Result<Rendered<String>, RenderError>;
```

Then update every example to handle the `Result`. If a target trait is
infallible today, the spec should explicitly describe the adapter policy:
strict mode is unavailable through that trait, or errors render as a diagnostic
fragment/string, or the trait API must become fallible in a later migration.

### 2. The one-line target-trait delegation claim is still too optimistic

The spec now correctly says there are no blanket impls and that target support
remains opt-in. However, it still says a component can opt into a target by
writing a one-line trait impl delegating to its tree. The example is for
`BrowserRenderable`, but even that example does not compile against the current
trait shape:

- `BrowserRenderable::render_html_fragment` returns `BrowserFragment<Ready>`,
  not `Result`.
- The example does not handle `RenderError`.
- The trait also requires `as_any`.

The problem is larger for terminal output. `TerminalRenderable` is not just
`fn render(&self, term: &Terminal) -> String`; it also requires `layout`,
`layout_mut`, and `as_any`, and it has layout semantics baked into the
component. A `TreeRenderable` subtree does not supply those accessors.

The spec should soften or qualify the "one-line" claim. A more accurate model:

- Tree rendering removes the need to hand-write the target's document-structure
  renderer.
- Infallible target traits still need adapter policy for errors.
- `TerminalRenderable` still needs layout ownership or a wrapper type that owns
  layout and delegates tree rendering.

One possible implementation pattern is a reusable adapter:

```rust
pub struct TreeComponent<T> {
    inner: T,
    layout: Layout,
    strictness: RenderStrictness,
}
```

That would make the ergonomics real without pretending every target trait can
be implemented as a literal one-liner on every component.

### 3. `PageOptions` and `MarkdownOptions` are probably the wrong renderer option types

The spec says renderers take target-specific options/context and proposes
reusing existing `MarkdownOptions` and `PageOptions`. That may be expedient, but
the current option types do not appear to be shaped for this new job.

`PageOptions` is page assembly configuration. A tree-to-browser renderer that
returns `BrowserFragment<Ready>` needs fragment rendering options: HTML
strictness, raw-HTML policy, class/style emission, diagnostics behavior, and
possibly component stylesheet collection. Page metadata, scripts, external
assets, and page-level CSS are related but not the same concern. Using
`PageOptions` for fragment rendering risks mixing page assembly with tree
serialization.

`MarkdownOptions` currently belongs to `MarkdownRenderable` and returns plain
strings. It does not yet encode the revised strictness model, Markdown versus
MarkdownPlus mode, or the loss/degrade policy described in the spec.

Suggested fix: introduce explicit render option types and allow them to embed
or convert from the older options:

```rust
pub struct MarkdownRenderOptions {
    pub dialect: MarkdownDialect,
    pub strictness: RenderStrictness,
    pub style: Option<MarkdownStyleOptions>,
}

pub struct BrowserRenderOptions {
    pub strictness: RenderStrictness,
    pub raw_html: RawHtmlPolicy,
    pub page: Option<PageOptions>,
}
```

This keeps page assembly options from becoming the catch-all browser render
context and gives strictness a clear home.

### 4. `Document` is introduced, but renderers only accept `RenderNode`

The fold now returns `Document { metadata, root }`, which is the right shape for
frontmatter and document-level state. But all renderer signatures take
`&RenderNode`, not `&Document`.

That leaves unclear how document metadata affects rendering:

- Browser output may need title, metadata, microdata, feature flags, or page
  stylesheet decisions.
- Markdown output may need frontmatter preservation or omission policy.
- Diagnostics may need the document source registry behind `SourceId`.
- TOC/heading IDs may need document-level metadata.

Components producing a bare subtree should still be renderable, but full
documents need first-class rendering too. The spec should probably define both
layers:

```rust
pub fn render_markdown_node(node: &RenderNode, opts: &MarkdownRenderOptions)
    -> Result<Rendered<String>, RenderError>;

pub fn render_markdown_document(doc: &Document, opts: &MarkdownRenderOptions)
    -> Result<Rendered<String>, RenderError>;
```

The document-level function can call the node-level one internally, but the API
should not force callers to drop metadata manually.

### 5. `SourceSpan` cannot represent synthetic nodes as currently sketched

The `RenderNode` comment says builders default `span` to a `Synthetic` span.
But `SourceSpan` requires:

```rust
pub source: SourceId,
pub bytes: Range<usize>,
pub provenance: Provenance,
```

That shape is awkward for synthetic nodes because there is no backing source
text and therefore no meaningful byte range. The prose says `None` is only for
APIs that do not supply a span, but builders default to `Synthetic`, so
synthetic nodes still need a fake `SourceId` and fake `0..0` bytes.

That will leak into diagnostics unless every consumer learns that `Synthetic +
0..0` means "no source". It also makes `SourceSpan` less honest than the revised
spec wants it to be.

Suggested fix: split source location from provenance, or make location optional
inside the span:

```rust
pub struct SourceSpan {
    pub provenance: Provenance,
    pub location: Option<SourceLocation>,
}

pub struct SourceLocation {
    pub source: SourceId,
    pub bytes: Range<usize>,
}
```

Then parsed and transcluded nodes can carry real locations, while synthetic and
generated nodes can carry provenance without inventing byte ranges.

### 6. `SourceId` is too central to leave open much longer

The spec leaves `SourceId` representation as an open Milestone 1 question. That
is reasonable as an implementation detail, but `SourceId` affects public
serialization, diagnostics, test fixtures, transclusion provenance, and whether
`Document` must carry a source registry.

If `SourceId` is just an opaque interned handle, a serialized `RenderNode`
cannot be interpreted without a side table. If it is a path/URL enum, it can be
serialized directly but may be heavier and leak filesystem-specific details. If
it is a component-origin enum, it needs stable naming conventions.

Suggested fix: move the source-registry decision into the spec now:

```rust
pub struct Document {
    pub sources: SourceRegistry,
    pub metadata: DocumentMetadata,
    pub root: RenderNode,
}
```

`SourceSpan` can then carry a small `SourceId`, and serialization fixtures can
include the registry. Without this, the span model remains underspecified.

### 7. `NodeAttrs::style: CssStyle` may overfit browser semantics

The revision adds `NodeAttrs { id, classes, style, data }` and says browser maps
classes to CSS while terminal maps them to SGR. The `style` field, however, is
specifically `crate::stylesheet::CssStyle`.

That is practical because `CssStyle` already exists, but it makes the
cross-target tree carry CSS as the typed style intent. Terminal rendering then
has to downsample CSS into terminal styling, likely through a lossy and partial
mapping. Some CSS properties are irrelevant to terminal output; some terminal
styles have no clean CSS equivalent; and using `CssStyle` in the core attrs may
make future non-browser targets inherit browser vocabulary by default.

This may still be the right trade-off, but the spec should call it out as a
deliberate downsampling contract. If the intent is truly target-neutral styling,
consider a smaller semantic style layer:

```rust
pub enum InlineSemanticStyle {
    Mark,
    Dim,
    Superscript,
    Subscript,
    CustomClass(String),
}
```

`CssStyle` could remain available in `attrs.data` or a browser-specific
extension. If the design keeps `CssStyle`, define which CSS properties terminal
renderers are expected to honor and how unsupported properties produce
diagnostics.

### 8. `NodeAttrs::data: BTreeMap<String, String>` is a weak carrier for structured metadata

The first review warned against an untyped dumping ground. The revised spec
correctly adds typed `id`, `classes`, and `style`, but `data:
BTreeMap<String, String>` is still asked to carry several important semantics:
horizontal-rule attributes, structured link metadata, and target hints.

Stringly typed extension data is easy to add but hard to validate, serialize
consistently, or round-trip without convention drift. It also pushes parsing
burden into every renderer. Link metadata is a good example: `target`, `rel`,
and structured props have known domains and should not be arbitrary strings if
they are important enough for parity with existing darkmatter `Link` behavior.

Suggested fix: keep `data` for genuinely unknown extension data, but promote
known metadata now or require namespaced keys and JSON values:

```rust
pub data: BTreeMap<String, serde_json::Value>
```

or:

```rust
pub extensions: BTreeMap<ExtensionKey, ExtensionValue>
```

At minimum, specify key namespaces such as `darkmatter.link.target`,
`darkmatter.hr.width`, or `renderable.target.browser`.

### 9. The parser inventory table still has a few suspicious mappings

The spec says the inventory must be verified, which is good. A few entries
already look under-specified enough to call out:

- `TableHead` is listed as `Node`, but `NodeKind` has no `TableHead`. If header
  rows are represented as ordinary `TableRow`, the fold needs an attribute or
  position convention so renderers can emit `<thead>` in browser output and
  format Markdown tables correctly.
- `TaskListMarker(bool)` is listed as an attr that sets enclosing
  `ListItem.checked`. That requires the fold to know the current enclosing
  item and to handle malformed or unexpected marker placement. The disposition
  should define the error path.
- `MetadataBlock` is mapped to `DocumentMetadata`, but the open questions still
  leave `DocumentMetadata` undefined. If this is frontmatter, the fold needs to
  state whether it parses YAML/TOML/JSON or stores raw text.
- `DefinitionList*`, `InlineMath`, and `DisplayMath` are marked verify. If
  unsupported for v1, tests should assert strict diagnostics for fixtures using
  those constructs so they do not become accidental silent losses.

The inventory table is now a required Milestone 1 deliverable, so this is not a
blocker. The suggestion is to make the table more precise before coding the
fold, especially around table header semantics.

### 10. Validation needs a mode and probably should return `Result`

The spec defines:

```rust
pub fn validate(node: &RenderNode) -> ValidationReport;
```

That is fine for inspection, but rendering in strict mode should probably be
able to require a valid tree and fail directly. Also, some validation findings
may be warnings rather than hard errors: unsupported nodes are structurally
valid but semantically unsupported for a target; block-in-inline is structurally
invalid; a table with mismatched cell counts may be renderable with warnings.

Suggested fix: give validation severities and an ergonomic strict API:

```rust
pub fn validate(node: &RenderNode, mode: ValidationMode) -> ValidationReport;
pub fn ensure_valid(node: &RenderNode) -> Result<(), ValidationError>;
```

Then each renderer can state whether it calls validation internally, requires
prevalidated input, or validates only in debug/strict mode.

### 11. Serialization fixtures need to include diagnostics and source registry assumptions

The testing section requires JSON fixtures for every `NodeKind`, `SourceSpan`,
and `NodeAttrs`. That covers node shape, but the revised design also depends on
diagnostics and source identity.

If `Rendered<T>` carries diagnostics and `Document` carries source/metadata
state, fixture coverage should include:

- `Document` serialization, not only `RenderNode`.
- `Diagnostic` serialization or snapshot shape, if diagnostics are public.
- `SourceId` / source registry representation.
- `Unsupported` nodes with diagnostics.
- `Synthetic`, `Generated`, and `Transcluded` provenance.

Otherwise the public JSON format could stabilize around nodes while leaving the
supporting structures underspecified.

## Quick Improvement Pass

1. Update all renderer signatures and examples to use
   `Result<Rendered<T>, RenderError>` consistently.
2. Add explicit adapter policy for infallible target traits, especially
   `BrowserRenderable` and `TerminalRenderable`.
3. Introduce dedicated `MarkdownRenderOptions` and `BrowserRenderOptions`
   instead of overloading existing `MarkdownOptions` / `PageOptions`.
4. Add document-level render functions alongside node-level render functions.
5. Redesign `SourceSpan` so synthetic/generated nodes do not need fake byte
   ranges.
6. Decide whether `Document` owns a `SourceRegistry`; if yes, include it in the
   type sketch and serialization fixtures.
7. Clarify whether `CssStyle` is the canonical cross-target style model or a
   browser-biased style payload with a documented terminal downsampling policy.
8. Promote known `attrs.data` fields or require typed/namespaced extension
   values.
9. Tighten the parser inventory around table headers, task-list marker
   placement, metadata block parsing, and unsupported math/definition-list
   diagnostics.
10. Define validation severities and whether renderers validate internally.
