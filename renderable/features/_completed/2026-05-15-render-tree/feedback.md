# Render Tree Spec - Adversarial Feedback

This is an intentionally critical review of `spec.md`. I am not arguing against
the core direction. A canonical owned tree is a reasonable hub for multi-target
rendering, and keeping `pulldown-cmark` as the parser is likely the right
constraint. The main concern is that the current draft makes the hard parts look
smaller than they are. Several claims are directionally true only if the tree,
fold, walkers, options model, and migration plan are tightened together.

## Findings

### 1. The sequencing undercuts the main validation loop

The spec says the work lands in two units: first `renderable` gets the node type
and walkers, then a later `darkmatter` feature adds the `pulldown-cmark` events
to `RenderNode` fold. At the same time, the motivation and success story depend
on "parse once, fold once, walk per target", and Unit 1 explicitly names
`text -> tree -> Markdown` round-tripping as a cheap correctness test.

That validation is impossible if the fold is deferred. A handwritten test tree
can prove that the Markdown walker handles a few node shapes, but it cannot prove
that the proposed tree actually captures what `pulldown-cmark` emits, that byte
positions work, that tables and task lists fold correctly, or that the node
vocabulary is sufficient. The riskiest part is not allocating a `RenderNode`; it
is whether real parser events can be faithfully represented and then rendered
back with acceptable loss.

This creates a dangerous path where `renderable` can ship a polished but
untested abstraction, and the later darkmatter fold discovers that the shape is
wrong. At that point, the public API has already hardened around the wrong
center.

The sequencing should include a narrow vertical slice earlier: parse a small set
of real Markdown fixtures with `pulldown-cmark`, fold them into `RenderNode`,
and render them back to Markdown. That does not require migrating darkmatter's
public `as_html` or `for_terminal` paths. It does require implementing enough of
the fold to stress the model before the model becomes the committed spine.

### 2. The tree is called a render tree, but the node model is mostly a Markdown AST

The proposed `NodeKind` mirrors common MDAST vocabulary: root, paragraph,
heading, list, table, link, image, inline code, HTML, and so on. That is useful
for Markdown parity, but the architecture claims this is a general render-content
concept shared by parsed documents and components. The current shape does not
yet carry enough rendering semantics for that job.

Examples:

- There is no place for semantic styling or classes that a browser walker can
  turn into CSS and a terminal walker can turn into SGR.
- There is no place for layout hints, width behavior, fill behavior, alignment
  overrides, source-specific metadata, or microdata.
- The spec says custom inline tags such as `==mark==` and dim text become
  ordinary tree nodes, but the enum has no `Mark`, `Dim`, `Span`, `Styled`, or
  equivalent variant.
- The spec mentions horizontal rules with attributes becoming tree nodes, but
  `ThematicBreak` has no fields.
- Existing darkmatter link and image support appears richer than
  MDAST-compatible `url`, `title`, and `alt` fields. A plain `Link` node cannot
  preserve things like target, relation, structured props, or browser-only link
  metadata without an annotation mechanism.
- Existing terminal rendering uses component layout and terminal capabilities,
  while the tree has no expression for those concerns.

This is not just an extensibility nit. If components are expected to implement
`render_ast` once and get terminal, browser, and Markdown rendering "for free",
they need a way to express the non-Markdown parts of their intent. Otherwise
the tree is a good Markdown AST but a weak component IR. Components will keep
falling back to bespoke per-target implementations, which weakens the central
value proposition.

The spec defers a generic `data` slot until tree-rewrite passes need it. That may
be too late. The first browser and terminal walkers already need per-node
annotations for style, layout, source, feature flags, and target-specific
fallbacks. The envelope makes it cheap to add a field mechanically, but it does
not make the semantics cheap to design after public APIs and walker behavior
already depend on an unannotated tree.

### 3. The crate-boundary story for terminal walking is probably incomplete

The spec says generic `tree -> {Terminal, Browser, Markdown}` walkers live in
`renderable`, and also says `renderable` depends on neither `darkmatter` nor
`biscuit-terminal`. That boundary is clean for the tree type and probably clean
for Markdown and browser fragments. It is much less convincing for terminal
output.

Terminal rendering in this repo is not just "write ANSI strings". The
`TerminalRenderable` trait takes a `biscuit_terminal::terminal::Terminal`,
components own terminal `Layout`, and existing behavior includes wrapping,
alignment, row fill, OSC8 links, color-depth decisions, image protocol behavior,
tables, code highlighting, mermaid/image fallbacks, and rich error blocks. A
terminal walker inside `renderable` cannot name those types without introducing
the dependency the spec explicitly forbids.

There are only a few ways out:

- Put only a terminal-neutral IR or plain-string fallback in `renderable`, and
  let `biscuit-terminal` own the real terminal walker.
- Move enough terminal concepts into `renderable` that the terminal walker can
  be meaningful there, which is much larger than the current spec.
- Define a small trait-based terminal sink in `renderable` and implement it from
  `biscuit-terminal`, which needs to be specified carefully.

As written, "generic tree -> Terminal walker in renderable" hides a dependency
cycle or a degraded terminal implementation. The draft should pick the boundary
explicitly before implementation starts.

### 4. The `Visitor` default-method design risks silently losing semantics

The target walker section proposes one `visit_*` method per variant, each
defaulting to recurse into `children()`. That sounds ergonomic, but it is a bad
default for a canonical rendering layer unless there is a strict loss policy.

For many nodes, "just recurse" is not a sane rendering:

- `Link` recursion emits the label but drops the URL.
- `Image` has no children, so it disappears unless explicitly handled.
- `Code`, `InlineCode`, `ThematicBreak`, `Break`, and `Html` are leaves and
  could disappear under a generic recursion fallback.
- `List` with an `items` field does not fit a generic `children()` model unless
  `children()` treats `items` as children, which weakens the distinction between
  child content and structural slots.
- `Table` has `rows`, not `children`; again, the traversal abstraction has to
  blur the node shape.
- `Heading` recursion emits the heading text but loses depth.

Silent degradation is especially risky because the feature is motivated by
avoiding divergence and latent bugs. A visitor that compiles when a target has
not made a deliberate decision about `Image` or `Html` can create exactly the
kind of divergence the tree is meant to prevent.

Default recursion may still be useful for transform visitors, but render
visitors should probably be more explicit. At minimum, there should be a
`RenderLoss` or warning mechanism, a test that each target makes an explicit
decision for every node kind, and a documented fallback policy per target.

### 5. `position` is not as free or complete as the spec suggests

The draft says `position` is free because `pulldown-cmark` exposes byte ranges
through `into_offset_iter()`. That is only partly true.

Leaf events can carry ranges naturally, but container nodes are assembled from
start and end events. The final source range for a paragraph, list item,
blockquote, table row, or table cell is not obviously just the start event range
or end event range. It probably needs to span from the first meaningful child to
the end event, and that gets subtle around blank lines, indentation, and nested
structures.

Also, a byte range alone is not enough once darkmatter composition enters the
picture. The compose pipeline performs interpolation, transclusion, link
resolution, shell expansion, and other rewrites. A tree node may originate from
a different file, a shell command, generated TOC content, or a component. A plain
`Option<Position>` does not answer "which source?" or "was this synthetic?" or
"does this range refer to pre-compose or post-compose text?"

If position is included now, it should probably be a richer source span from the
start:

- source identity, not just byte offsets
- byte range and possibly line/column helpers
- generated/synthetic provenance
- enough room to represent transcluded content
- a clear statement about whether ranges refer to raw source, composed source,
  or parser input

Otherwise the field may give a false sense of diagnostic readiness and become
hard to correct later.

### 6. "Unhandled events become no-op" is too permissive for a canonical fold

The spec says the fold is total and any unhandled event becomes a no-op or a
documented fallback, never a panic. Avoiding panics is good. Allowing no-ops in
the canonical parser fold is not.

`pulldown-cmark` supports more than the enum currently models. Depending on the
enabled options, there are concerns such as footnotes, definition lists, metadata
blocks, reference definitions, task list markers, inline and block HTML, tables,
strikethrough, and smart punctuation. Some of these can be represented by the
proposed tree. Some cannot. Dropping the rest quietly means the new canonical
path can be less correct than the old event path while still passing broad
smoke tests.

The fold should be total in the Rust sense, but not quiet. A better rule is:
every event maps to a node, a known semantic side table, a documented lossy
fallback with a warning, or an explicit unsupported node. No-op should be
reserved for events proven to be structural noise.

### 7. The proposed node vocabulary is missing several Markdown constructs

The enum covers the common happy path, but it is not aligned with
`pulldown-cmark` with all options enabled. The spec should inventory the actual
event and tag set for the version in use (`pulldown-cmark = "0.13"`) and decide
where every construct lands.

Likely gaps include:

- Footnote definitions and footnote references.
- Reference link/image definitions and reference-style links.
- Definition lists, if enabled.
- Metadata blocks.
- Task list marker events and the relationship between task markers and
  `ListItem { checked }`.
- Soft breaks versus hard breaks. The enum has only `Break`, but Markdown and
  terminal output commonly need different behavior for soft and hard breaks.
- Inline HTML versus block HTML, if either target needs to preserve that
  distinction.
- Custom darkmatter inline styles such as mark and dim, which are mentioned in
  the spec but not represented.
- Horizontal-rule attributes from `RuleProcessor`, which are mentioned but not
  represented.

This matters because the spec wants the tree to become the single canonical
representation. A canonical representation can be intentionally lossy, but only
if the loss is named and accepted. Right now, the loss is implicit.

### 8. MDAST serialization compatibility is asserted but not specified

The draft says `serde` with `#[serde(tag = "type", rename_all = "camelCase")]`
keeps JSON compatible with MDAST. The proposed Rust shape will not automatically
produce MDAST-shaped JSON unless the serde design is spelled out.

For example, `RenderNode { kind, position }` with a tagged `NodeKind` will
naturally serialize with a `kind` object unless `kind` is flattened. MDAST nodes
normally have `type` at the same level as `children`, `value`, `position`, and
other fields. Similarly, list children are usually exposed as `children`, not
`items`, and table rows are usually children rather than a separate `rows`
field. The proposed `List { items }` and `Table { rows }` are fine Rust API
choices, but they are not MDAST JSON by default.

Position shape also matters. Unist position data is not just an arbitrary byte
range. If external consumers care about the existing `as_ast` output, "camelCase
tagging" is not enough to preserve compatibility.

The spec should either:

- define `RenderNode` JSON as its own format and stop calling it MDAST
  compatible, or
- specify custom serde behavior with fixtures proving compatibility for every
  supported node kind.

### 9. Component integration blurs opt-in target traits

The component section says document-structural components implement
`render_ast -> RenderNode`, and then generic tree walkers render terminal,
browser, and Markdown "for free". It also says `TerminalRenderable`,
`BrowserRenderable`, and `MarkdownRenderable` remain separate, opt-in traits.

Those claims need reconciliation. If a component implements `AstRenderable`,
does it automatically become renderable to browser, Markdown, and terminal via
blanket impls? If yes, then target support is no longer truly opt-in. If no,
then authors still need to implement target traits or call helper functions, and
the "for free" claim is overstated.

There is also an object-safety and API-shape concern. `BrowserRenderable`
currently returns a `BrowserFragment<Ready>`, `MarkdownRenderable` returns
strings with options, and `TerminalRenderable` lives in `biscuit-terminal` and
requires layout accessors plus terminal-aware rendering. A `RenderNode` alone
does not satisfy those trait contracts.

The design may need a separate trait, perhaps `TreeRenderable`, that means
"this component can produce a render tree". Target traits can then opt into
default implementations from that tree where the crate boundary allows it. That
would make the relationship explicit instead of overloading `AstRenderable`.

### 10. `Markdown` versus `MarkdownPlus` needs a loss policy

The spec says `MarkdownPlus` may emit `Html` nodes and plain `Markdown` emits
none. That raises immediate questions:

- What happens when the source document contains raw HTML and the caller asks
  for plain Markdown?
- Is the HTML escaped, dropped, converted, or reported as unsupported?
- How does the Markdown walker behave for tree nodes that have no Markdown
  syntax, such as styled spans, browser attributes, custom components, or future
  layout nodes?
- Is `text -> tree -> Markdown` intended to be byte-stable, semantically stable,
  or just parseable?

Without a policy, "plain Markdown emits no HTML" can mean silent data loss. The
round-trip tests could then pass only because they avoid difficult fixtures.

The spec should define loss modes up front: strict error, warn and degrade,
escape, drop, or MarkdownPlus fallback. The default should probably not be
silent drop.

### 11. Target walkers need option and context types, not just visitors

The walker section describes a visitor trait but not the state required to
render correctly. Real target output needs context:

- Terminal width, color mode, hyperlink mode, image protocol support, wrapping,
  layout, theme, and syntax-highlighting settings.
- Browser page options, stylesheet collection, CSS variables, assets, feature
  flags, safe raw HTML handling, and metadata.
- Markdown style options, whether HTML is allowed, line-width/wrapping policy,
  table formatting policy, and link normalization.

If the visitor owns this state, the trait signature needs to show it. If the
state is passed as options to top-level helper functions, those option types
need to be part of the design. Without this, the "generic walkers" are only
abstractly specified and may not compose with existing `MarkdownOptions`,
`PageOptions`, `TerminalRenderable`, and `DarkmatterPage` behavior.

### 12. Tree transforms are used as motivation but not designed enough to shape the tree

The draft argues that tree-rewrite passes will be better than string
preprocessing and stream-mutating iterators. That is plausible, but the concrete
tree shape does not yet reflect the needs of the existing compose pipeline.

For example, transclusion needs source identity and provenance. Link resolution
needs to distinguish original and resolved URLs, or at least preserve enough
information to normalize back to portable paths. TOC linking needs heading IDs
or a reliable heading extraction story. Shell expansion and generated blocks
need generated-source diagnostics. Conditional page blocks may need annotations
about skipped or retained content. Frontmatter interpolation may need a document
metadata model separate from root children.

If tree transforms are a major reason for doing this, it is worth designing just
enough transform infrastructure now: pass ordering, mutation API, diagnostics,
metadata/provenance, and validation. Otherwise the first transform migration may
force retrofitting fields that should have informed `RenderNode` from the start.

### 13. Structural validity is deferred, but invalid trees will be easy to create

The spec intentionally avoids a type-level inline/block split, and that is a
reasonable simplification. However, it then allows any `RenderNode` to appear
anywhere. Components can emit a `Root` inside a `Paragraph`, a `TableCell`
outside a `TableRow`, a `Heading` inside a `Link`, or a `ListItem` directly
under `Root`.

If target walkers are expected to be generic and robust, they need either a
validation pass or strong builder APIs that make common invalid states hard to
construct. Otherwise every walker must either handle malformed trees defensively
or produce broken output.

The spec says structural validity can be a separate validation pass "if needed".
It is needed as soon as components can splice their own subtrees into parsed
documents. A minimal validator should probably be part of the spine, even if the
type system remains simple.

### 14. Error handling is missing from the public shape

The examples and prose imply infallible rendering: visitors walk nodes and emit
strings/fragments. Existing rendering is not purely infallible in practice. HTML
asset paths can be validated earlier, but Markdown conversion can be lossy,
terminal rendering may encounter unsupported nodes, syntax highlighting may have
missing languages, source spans may be invalid after transforms, and strict mode
should be able to reject unsupported structures.

Returning `String` everywhere would force silent degradation or panics. Returning
`Result` everywhere has API cost but makes loss and unsupported features visible.
The spec should choose, probably with target-specific error/report types. Given
darkmatter already has rich diagnostic conventions, the tree and walkers should
not flatten those concerns too early.

### 15. The memory risk is probably broader than "document sizes are small"

The risk section notes that a tree holds the whole document and says that is
negligible at darkmatter document sizes. That may be true for most authored
documents, but the new design also encourages component subtrees, transcluded
documents, generated shell output, TOC content, and possibly embedded HTML/SVG.
Owned strings amplify the cost because text is copied out of `pulldown-cmark`
events.

This is not a reason to avoid an owned tree. It is a reason to define expected
usage boundaries and performance tests. A few large-fixture benchmarks would
keep the design honest: large code blocks, large tables, deeply nested lists,
many links/images, transcluded generated content, and repeated component
subtrees.

## Quick Improvement Pass

### Make the first milestone a vertical slice

Instead of shipping all `renderable` walkers before the fold exists, make the
first milestone:

1. `RenderNode` and supporting types.
2. A minimal darkmatter fold for common `pulldown-cmark` events.
3. A Markdown walker with strict and lossy modes.
4. Golden fixtures for `text -> tree -> Markdown`.

This keeps the API honest. Browser and terminal walkers can follow once the
tree has survived real parser input.

### Split the concepts more explicitly

Consider these names and responsibilities:

- `RenderNode`: the owned tree node.
- `TreeRenderable`: a component can produce a `RenderNode`.
- `MarkdownTreeRenderer`: tree to Markdown/MarkdownPlus.
- `BrowserTreeRenderer`: tree to `BrowserFragment<Ready>` or `HtmlPage`.
- `TerminalTreeRenderer`: likely owned by `biscuit-terminal`, or parameterized
  over a terminal sink defined in `renderable`.

This would avoid making `AstRenderable` carry too much meaning. `AstRenderable`
can then either become a compatibility alias, a serializer-oriented trait, or be
reworked deliberately.

### Add an annotation and provenance model sooner

Do not wait for compose migration to add all metadata. A small envelope can
remain conservative while still covering known needs:

```rust
pub struct RenderNode {
    pub kind: NodeKind,
    pub span: Option<SourceSpan>,
    pub attrs: NodeAttrs,
}
```

`NodeAttrs` does not need to become an untyped dumping ground. It could start
with stable fields such as `id`, `classes`, `style`, `data`, and maybe
target-specific extension maps. `SourceSpan` should include source identity and
synthetic provenance, not just offsets.

### Inventory every parser event before finalizing `NodeKind`

Add a table to the spec with every relevant `pulldown-cmark 0.13` event/tag and
one of:

- represented directly by `NodeKind`
- represented by node attributes
- stored in a side table or document metadata
- converted lossily with warning
- unsupported in strict mode

This table will reveal whether the tree is actually canonical or only covers the
happy path.

### Define loss and strictness policies per target

Each target walker should have a policy for unsupported or non-native nodes:

- `Strict`: return an error/report.
- `Warn`: emit best-effort output and diagnostics.
- `Lossy`: silently degrade only for explicitly documented cases.
- `MarkdownPlus`: permit HTML where plain Markdown cannot express structure.

Plain Markdown should not silently drop raw HTML or custom nodes.

### Rework visitor defaults

Use different visitor traits for different jobs:

- Transform visitors can default to recursive traversal.
- Render visitors should require explicit handling for leaf/semantic nodes, or
  at least produce diagnostics when a default fallback loses information.

A compile-time or test-time exhaustiveness check would help every target stay in
sync when new node kinds are added.

### Put terminal rendering at the right layer

Do not force a terminal walker into `renderable` unless it can be meaningful
without depending on `biscuit-terminal`. The cleaner design may be:

- `renderable` owns tree, Markdown walker, browser walker, and target-neutral
  traversal helpers.
- `biscuit-terminal` owns `RenderNode -> terminal string` because it owns
  `Terminal`, `Layout`, protocol detection, terminal styles, and terminal
  component behavior.

If `renderable` must expose a terminal walker, define a small terminal rendering
backend trait and prove that `biscuit-terminal` can implement it without awkward
adapter leakage.

### Add validation and builder APIs

Keep `RenderNode` simple, but provide constructors and a validator:

- `RenderNode::root(children)`
- `RenderNode::paragraph(inline_children)`
- `HeadingDepth::new`
- `ColumnAlign`
- `validate_tree(&RenderNode) -> ValidationReport`

The validator should catch misplaced table/list nodes, invalid heading depths,
block nodes inside phrasing-only containers, and other structural mistakes.

### Specify serde with fixtures

If MDAST compatibility is a real goal, add JSON fixtures for root, paragraph,
heading, list, table, link, image, code, HTML, and position. Use custom serde if
needed. If exact compatibility is not a goal, rename the claim to "MDAST-inspired"
and treat `as_ast` compatibility as a separate adapter.

### Add performance and parity gates

Before any darkmatter renderer migrates, require:

- fixture parity for existing tricky Markdown cases
- explicit tests for links, images, code fences, tables, task lists, raw HTML,
  custom inline styles, and horizontal-rule attributes
- benchmarks for large documents and large generated blocks
- diagnostics tests for unsupported/lossy conversions

This gives the project a concrete definition of "ready to cut over" instead of
relying on the existence of a tree as proof that the architecture is complete.
