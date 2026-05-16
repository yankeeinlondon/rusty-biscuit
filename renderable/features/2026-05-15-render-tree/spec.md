# Render Tree — Spec

**Date:** 2026-05-15
**Updated:** 2026-05-16
**Status:** Draft / design discussion
**Areas:** `renderable` (tree + Markdown/Browser walkers), `darkmatter` (fold),
`biscuit-terminal` (Terminal walker)

## Summary

Introduce a canonical, owned **render tree** that sits between content sources
and render targets. Document-structural components render *into* this tree;
target renderers render *out of* it to Terminal, Browser, and Markdown.

The work splits across three crates along the existing dependency graph:

- **`renderable`** owns the `RenderNode` tree type, the `TreeRenderable` trait,
  validation/builders, and the Markdown and Browser renderers.
- **`darkmatter`** owns the **events → tree fold** that turns a
  `pulldown-cmark` event stream into a `RenderNode`.
- **`biscuit-terminal`** owns the **Terminal renderer**, because terminal
  output needs `Terminal`, `Layout`, and protocol detection that live there.

This replaces the current "parse text, walk events, emit strings — once per
target" model with "parse once, build one tree, walk it per target."

> This draft was revised across two adversarial architecture reviews. The
> per-item decisioning is recorded in [`response.md`](./response.md) (first
> pass) and [`response-2.md`](./response-2.md) (second pass).

## Motivation

### The problem today (darkmatter)

darkmatter has **two independent Markdown pipelines using two parser crates**:

| Output                    | Crate                | Shape                       |
|---------------------------|----------------------|-----------------------------|
| `as_html`, `for_terminal` | `pulldown-cmark` 0.13 | Streaming events — no tree  |
| `as_ast`                  | `markdown` 1.0-alpha | MDAST tree                  |

- `as_html` / `for_terminal` are hand-written **event → string** serializers.
  The source text is parsed *twice* (once per pipeline), and a third time for
  `as_ast`.
- The MDAST path (`as_ast`) is a **dead end**: a tree is produced and handed to
  the caller, but nothing renders *from* it.
- Document structure is therefore re-interpreted independently in each event
  loop — a latent divergence-bug class.
- Document transformations darkmatter already needs (transclusion,
  interpolation, TOC linking — the `compose/` module) are implemented as
  **string preprocessing** and **stream-mutating iterator adapters**
  (`InlineStyleProcessor`, `RuleProcessor`). These are workarounds for not
  having a tree.

### Why a tree, and why now

The focus is shifting from Terminal-first to **Terminal + Browser** as
first-class targets. The event model degrades exactly where `renderable`'s
goals live:

1. **Shared structure.** A tree decides document structure once; targets only
   decide presentation.
2. **Additive targets.** A new target is a new walk over a stable node set, not
   a full re-implementation of the parser-event handling.
3. **Transformable.** Tree-rewrite passes are composable and testable; stacked
   stream-mutating iterators are not.
4. **Component embedding.** A component that produces a tree node can be
   *spliced into the tree*. A component cannot be cleanly spliced into an event
   stream — it would have to emit each consumer's event dialect.

### Key realization: keep `pulldown-cmark`

`pulldown-cmark` is a **pull/event parser** by deliberate design — fast,
low-allocation, stable, and the de-facto standard. "Event parser" is not the
opposite of "tree." Events can be **folded into a tree** with a stack-based
pass.

Therefore: **keep `pulldown-cmark` as the parser.** Do not adopt the alpha
`markdown` crate as the rendering hub. Build the canonical tree from
`pulldown-cmark` events into a node type `renderable` owns.

## What the tree is — and is not

The render tree is a **document-structural intermediate representation**. It is
strong for document content (headings, prose, lists, tables, code, links,
images, styled inline spans) and carries enough annotation (`attrs`, `span`)
for cross-target styling and provenance.

It is **not** a universal component IR. Inherently visual or layout-driven
components — `TerminalImage`, `GraphExpression`, and anything whose intent is
not expressible as document structure — are **not** required to route through
the tree. They keep direct per-target trait implementations. The tree's value
proposition is scoped accordingly: *document-structural* components implement
one method and get all three targets; other components use the escape hatch.

## Goals

In `renderable`:

- Define the owned render-tree types (`RenderNode`, `NodeKind`, `NodeAttrs`,
  `SourceSpan`, `Document`, `SourceRegistry`, `DocumentMetadata`).
- Provide the `TreeRenderable` trait and tree-construction builders.
- Provide a structural `validate` pass.
- Provide the Markdown and Browser renderers, with explicit strictness/loss
  policy and diagnostics.

In `darkmatter`:

- Provide the **events → tree** fold from a `pulldown-cmark` event stream, with
  a defined disposition for *every* parser event.
- Parse source text **once**, not three times.

In `biscuit-terminal`:

- Provide the Terminal renderer over `RenderNode`.

## Non-goals

- Rewriting darkmatter's existing `as_html` / `for_terminal` in this feature.
- Replacing `pulldown-cmark`.
- Forcing every component to be multi-target. Target traits stay separate and
  opt-in (see [Component integration](#component-integration)).
- A type-level inline/block split (see [Validation](#validation--builders)).
- Designing the `compose/` transform *pipeline*. The node model reserves the
  hooks transforms need (`SourceSpan` provenance, `NodeAttrs`, document
  metadata) but the pass framework is a later feature.
- MDAST JSON wire-compatibility. The tree serializes to *its own* format; see
  [Serialization](#serialization).

## Architecture

```text
   darkmatter                          renderable
  ┌───────────────────────────┐   ┌───────────────────────────────┐
  │  source ─▶ pulldown-cmark │   │   ┌──────────────┐             │
  │  text      │ events       │   │   │  RenderNode  │             │
  │            ▼              │   │   │  + Document  │             │
  │   events → tree fold ─────┼───┼─▶ └──────────────┘             │
  │   (total; every event     │   │          │                     │
  │    has a disposition)     │   │   ┌──────┴───────┐             │
  └───────────────────────────┘   │   ▼              ▼             │
                                  │ Markdown      Browser           │
   components (biscuit-terminal)  │ renderer      renderer           │
  ┌───────────────────────────┐  │ (MD / MD+)    (Fragment/Page)    │
  │ Table, BlockQuote, … ─────┼──┼▶      │                          │
  │   impl TreeRenderable     │  └───────┼──────────────────────────┘
  └───────────────────────────┘          │ RenderNode
                                          ▼
                            biscuit-terminal: Terminal renderer
                            (owns Terminal, Layout, protocols)
```

- Parse once. Fold once. Walk per target.
- **`RenderNode` producers** are symmetric: a parsed document (via the fold)
  and a component (via `TreeRenderable`) yield the *same* tree.
- The Terminal renderer lives in `biscuit-terminal`, not `renderable` — see
  [Crate boundaries](#crate-boundaries).

## Crate boundaries

The split follows the dependency graph
(`darkmatter → biscuit-terminal → renderable`; `renderable` depends on
neither):

| Piece                                  | Crate             | Why                                                                                              |
|-----------------------------------------|-------------------|--------------------------------------------------------------------------------------------------|
| `RenderNode` + supporting types         | `renderable`      | Components must produce it; placing it in `darkmatter` would cycle.                              |
| `TreeRenderable`, builders, `validate`  | `renderable`      | Foundation API the tree is constructed and checked through.                                      |
| Markdown renderer                       | `renderable`      | Emits plain strings; no terminal/Markdown-parser dependency.                                     |
| Browser renderer                        | `renderable`      | Emits `BrowserFragment<Ready>` / `HtmlPage`, both already in `renderable`.                       |
| **Terminal renderer**                   | `biscuit-terminal`| Needs `Terminal`, `Layout`, color-depth, OSC8, image protocols — all owned by `biscuit-terminal`. |
| `pulldown-cmark` events → tree fold     | `darkmatter`      | Markdown- and `pulldown-cmark`-specific; that crate owns the parser + processors.                |

A "generic Terminal walker in `renderable`" was rejected: a meaningful terminal
renderer must name `biscuit-terminal` types, which `renderable` cannot depend
on. Since `biscuit-terminal` already depends on `renderable`, the Terminal
renderer simply lives there as a consumer of `RenderNode`. `renderable` gains
**no** `pulldown-cmark` or `biscuit-terminal` dependency.

## The render tree type

`RenderNode` lives in `renderable` (replacing the `AstRenderable` placeholder in
`ast.rs`, filling `ast_utils.rs`).

### Envelope, attributes, span

```rust
/// A node in the canonical render tree.
pub struct RenderNode {
    pub kind: NodeKind,
    /// Provenance, and source location when one exists.
    pub span: SourceSpan,
    /// Cross-target presentational and identity annotations.
    pub attrs: NodeAttrs,
}

/// Cross-cutting annotations every node may carry. Default is empty.
#[derive(Default)]
pub struct NodeAttrs {
    /// Stable identifier (e.g. heading slug for TOC linking, anchor targets).
    pub id: Option<String>,
    /// Target-neutral *semantic* classes. A browser renderer maps these to
    /// CSS classes; a terminal renderer maps the documented vocabulary
    /// (`mark`, `dim`, `sup`, `sub`, …) to SGR and ignores unknown classes.
    pub classes: Vec<String>,
    /// Namespaced extension data for annotations with no first-class field
    /// (HR attributes, structured link metadata, target hints). Keys are
    /// namespaced (`darkmatter.hr.width`, `darkmatter.link.target`,
    /// `renderable.target.browser.style`); values are structured JSON.
    pub data: BTreeMap<String, serde_json::Value>,
}
```

`NodeAttrs` is included from the start, not deferred: the first Browser and
Terminal renderers already need per-node identity/class intent (`id` for TOC
anchors, `classes` for styled spans).

**No `CssStyle` in the core.** An earlier draft gave `NodeAttrs` a
`style: Option<CssStyle>` field. That is withdrawn: CSS is browser vocabulary,
and putting it in the core tree would force every other target to downsample
CSS and let future non-browser targets inherit browser bias. Styling intent in
the core is **semantic** — carried by `classes`, a documented vocabulary each
renderer interprets its own way. Raw CSS, when a browser-specific component
genuinely needs it, goes in namespaced `attrs.data`
(`renderable.target.browser.style`), not the core.

**`data` is structured and namespaced, not stringly typed.** Values are
`serde_json::Value`, so structured metadata (a link's `target`/`rel`, an HR's
width/color) round-trips without ad-hoc string parsing in every renderer. Keys
must be namespaced to prevent convention drift. `data` is the escape hatch for
*not-yet-promoted* fields; if parity testing shows a construct (e.g. link
metadata) is load-bearing, it is promoted to a typed field in a follow-up
rather than living in `data` forever.

### Source span and provenance

A node's *provenance* always exists; its *source location* does not — a
synthetic node emitted by a component has no backing text and therefore no byte
range. The two are split so synthetic/generated nodes never have to carry a
fake `SourceId` or `0..0` range:

```rust
pub struct SourceSpan {
    pub provenance: Provenance,
    /// Byte location in a real source. `None` for synthetic/generated nodes.
    pub location: Option<SourceLocation>,
}

pub struct SourceLocation {
    pub source: SourceId,
    pub bytes: std::ops::Range<usize>,
}

pub enum Provenance {
    /// Parsed directly from a source's original text.
    Parsed,
    /// Emitted by a component or builder — no backing source text.
    Synthetic,
    /// Produced by a transform pass (TOC, interpolation, shell expansion).
    Generated,
    /// Folded in from another document during transclusion.
    Transcluded { origin: SourceId },
}
```

`SourceId` is a small interned handle into the `Document`'s `SourceRegistry`
(see [Document wrapper](#document-wrapper)) — not a path or URL embedded per
node. That keeps nodes cheap and uniform; the registry resolves a handle to a
path, virtual name, or component origin, and is the single place serialization
carries the mapping.

**Container-node span computation is a defined fold responsibility** — a
container's `location` spans from its first meaningful child to its end event;
the fold spec documents the rules around blank lines and nesting.

### Node kinds

```rust
pub enum NodeKind {
    Root { children: Vec<RenderNode> },

    // ── Block content ────────────────────────────────────────────
    Heading { depth: HeadingDepth, children: Vec<RenderNode> },
    Paragraph { children: Vec<RenderNode> },
    BlockQuote { children: Vec<RenderNode> },
    List { ordered: bool, start: Option<u64>, children: Vec<RenderNode> },
    ListItem { checked: Option<bool>, children: Vec<RenderNode> },
    Code { lang: Option<String>, meta: Option<String>, value: String },
    ThematicBreak,                                  // attributes carried in `attrs`
    Table { align: Vec<ColumnAlign>, children: Vec<RenderNode> },
    TableRow { children: Vec<RenderNode> },
    TableCell { children: Vec<RenderNode> },
    FootnoteDefinition { identifier: String, children: Vec<RenderNode> },

    // ── Inline content ───────────────────────────────────────────
    Text { value: String },
    Emphasis { children: Vec<RenderNode> },
    Strong { children: Vec<RenderNode> },
    Delete { children: Vec<RenderNode> },
    /// Styled inline span — the carrier for `==mark==`, dim text,
    /// superscript/subscript, and class-driven inline styling. The styling
    /// itself is semantic, carried by `attrs.classes`.
    Span { children: Vec<RenderNode> },
    InlineCode { value: String },
    Link { url: String, title: Option<String>, children: Vec<RenderNode> },
    Image { url: String, title: Option<String>, alt: String },
    FootnoteReference { identifier: String },
    SoftBreak,
    HardBreak,

    // ── Raw / escape hatch ───────────────────────────────────────
    /// Raw HTML. `block` distinguishes an HTML block from inline HTML.
    Html { value: String, block: bool },

    // ── Explicit unsupported ─────────────────────────────────────
    /// A parser event with no faithful representation. Never produced
    /// silently — see the fold's disposition rules. Carries a label for
    /// diagnostics and strict-mode rejection.
    Unsupported { label: String },
}

impl RenderNode {
    /// Children of any node; `&[]` for leaves.
    pub fn children(&self) -> &[RenderNode] { /* match on kind */ }
    pub fn children_mut(&mut self) -> Option<&mut Vec<RenderNode>> { /* … */ }
}
```

### Rationale for the shape

- **Envelope struct, not a flat enum.** `span` and `attrs` are cross-cutting —
  every node carries them. On the envelope they are declared once and read
  without a `match`; on a flat enum they would be repeated across ~25 variants.
- **Children field is always named `children`** (except the structurally
  distinct `Table.align`). Earlier drafts used `items`/`rows`; uniform naming
  keeps the `children()` accessor trivial and removes per-variant special
  cases. `Table` keeps `align` because column alignment is genuinely
  table-level data, not a child.
- **`Span` instead of per-style variants.** `==mark==`, dim, superscript, and
  subscript are all styled inline spans; one `Span` variant carrying styling in
  `attrs` covers them without inflating the enum. This is also the variant the
  fold's custom `InlineStyleProcessor` tags map onto.
- **`ThematicBreak` stays fieldless.** Horizontal-rule attributes (from
  `RuleProcessor`) ride in namespaced `attrs.data` — the reason the envelope
  carries `attrs`.
- **`SoftBreak` and `HardBreak` are distinct.** They render differently
  (Markdown: trailing spaces / line; terminal: space vs. newline). A single
  `Break` would have erased that.
- **`Link` keeps three fields; richer link metadata** (target, relation,
  structured props from darkmatter's `Link`) rides in namespaced, structured
  `attrs.data` rather than inflating the variant — promoted to a typed field
  later if parity testing shows it is load-bearing.
- **`Unsupported` is a real variant.** The fold never silently drops an event;
  anything without a faithful node becomes `Unsupported`, which strict mode
  rejects and other modes report (see [the fold](#the-events--tree-fold)).
- **Struct variants everywhere**, even single-field (`Text { value }`) — named
  fields read better at match sites and are forward-compatible.
- **Constrained newtypes.** `HeadingDepth` (1–6), `ColumnAlign` — not raw
  integers.
- **One node type — no inline/block split.** A type-level split would double
  the type surface and complicate the fold, renderers, and components. MDAST
  does not split. Structural validity is enforced by the `validate` pass
  instead (see [Validation](#validation--builders)).
- **Owned strings** (`String`, not `CowStr`). The fold converts at the
  boundary; an owned tree carries no lifetime parameter.

### Document wrapper

Frontmatter and document-level state do not belong inside `Root`'s children.
The fold's output is a `Document`:

```rust
pub struct Document {
    /// Resolves every `SourceId` used by spans in this document's tree.
    pub sources: SourceRegistry,
    pub metadata: DocumentMetadata,
    pub root: RenderNode,            // a NodeKind::Root
}

pub struct DocumentMetadata {
    /// Raw frontmatter block, if present. Structured parsing
    /// (YAML/TOML/JSON) stays darkmatter's existing concern — the fold
    /// stores the verbatim block and its detected format.
    pub frontmatter: Option<Frontmatter>,
}

pub struct Frontmatter {
    pub format: FrontmatterFormat,   // Yaml | Toml | Json
    pub raw: String,
}
```

`SourceRegistry` maps each `SourceId` to a `SourceDescriptor` (a file path, a
virtual name, or a component origin). It lives on `Document` so a serialized
document is self-contained: spans carry small handles, the registry carries the
mapping.

Components produce a bare `RenderNode` subtree (via `TreeRenderable`); only the
fold produces a full `Document`. This keeps frontmatter separate from node
content and gives interpolation/TOC transforms a metadata home.

## Serialization

Earlier drafts claimed serde `#[serde(tag = "type", rename_all = "camelCase")]`
would keep the JSON **MDAST-compatible**. That claim is withdrawn:

- An envelope struct (`RenderNode { kind, span, attrs }`) serializes with a
  nested `kind` object; MDAST is flat (`type` beside `children`/`value`).
- `Table.align`, `NodeAttrs`, and `SourceSpan` have no MDAST equivalent.

Decision: **`RenderNode` serializes to its own documented format.** The
vocabulary is *MDAST-inspired*, not MDAST. If `darkmatter::as_ast` must keep
emitting MDAST-shaped JSON for external consumers, that is a **separate
adapter** (`Document → mdast JSON`), specified and fixture-tested on its own —
not an implicit consequence of deriving `Serialize`. The serialized surface is
the whole public format — `RenderNode`, `Document`, `SourceRegistry`,
`SourceSpan`, `NodeAttrs`, and `Diagnostic` — and fixtures are required for all
of it (see [Testing](#testing--parity-gates)).

## Parser event inventory

The fold must give **every** `pulldown-cmark` 0.13 event a disposition. The
table below is the working inventory; it **must be verified against the exact
event/tag set of the pinned `pulldown-cmark` 0.13** (and the enabled `Options`)
before the fold is implemented — entries marked *(verify)* are expected but not
yet confirmed.

Dispositions: **Node** (maps to a `NodeKind`), **Attr** (folded into a parent's
`attrs`), **Meta** (folded into `DocumentMetadata`), **Lossy** (best-effort
node + diagnostic), **Unsupported** (`NodeKind::Unsupported` + diagnostic),
**Noise** (proven structural-only; dropped).

| Event / Tag                          | Disposition | Target / note                                  |
|---------------------------------------|-------------|------------------------------------------------|
| `Paragraph`                           | Node        | `Paragraph`                                    |
| `Heading`                             | Node        | `Heading` (+ `attrs.id` slug)                  |
| `BlockQuote`                          | Node        | `BlockQuote`                                   |
| `CodeBlock`                           | Node        | `Code`                                         |
| `HtmlBlock` / `Html`                  | Node        | `Html { block: true }`                         |
| `InlineHtml`                          | Node        | `Html { block: false }`                        |
| `List` / `Item`                       | Node        | `List` / `ListItem`                            |
| `TaskListMarker(bool)`                | Attr        | sets enclosing `ListItem.checked` (see notes)  |
| `Table` / `TableRow` / `TableCell`    | Node        | `Table` / `TableRow` / `TableCell`             |
| `TableHead`                           | Node        | folds to a leading `TableRow` (see notes)      |
| `Emphasis` / `Strong` / `Strikethrough` | Node      | `Emphasis` / `Strong` / `Delete`               |
| `Superscript` / `Subscript` *(verify)* | Node       | `Span` + class                                 |
| `Link` / `Image`                      | Node        | `Link` / `Image` (extras → `attrs.data`)       |
| `Text` / `Code`                       | Node        | `Text` / `InlineCode`                          |
| `SoftBreak` / `HardBreak`             | Node        | `SoftBreak` / `HardBreak`                      |
| `Rule`                                | Node        | `ThematicBreak`                                |
| `FootnoteReference` / `FootnoteDefinition` | Node   | `FootnoteReference` / `FootnoteDefinition`     |
| `MetadataBlock` *(verify)*            | Meta        | `DocumentMetadata` (frontmatter)               |
| `DefinitionList*` *(verify)*          | Unsupported | not enabled for v1; revisit if darkmatter needs it |
| `InlineMath` / `DisplayMath` *(verify)* | Unsupported | revisit; `Code`-style fallback under Lossy   |
| darkmatter `==mark==`, dim (`InlineStyleProcessor`) | Node | `Span` + class                          |
| darkmatter HR-with-attributes (`RuleProcessor`) | Node + Attr | `ThematicBreak` + `attrs`               |

Producing the *verified, exhaustive* table is a required first step of the fold
work (Milestone 1).

### Inventory notes

- **Table header.** `NodeKind` has no `TableHead`. `pulldown-cmark`'s
  `TableHead` folds into an ordinary `TableRow` placed **first** under `Table`;
  renderers and the validator treat `Table.children[0]` as the header row
  (browser `<thead>`, Markdown delimiter row). This positional convention
  matches MDAST and is documented so every consumer agrees.
- **Task-list markers.** `TaskListMarker(bool)` sets `checked` on the enclosing
  `ListItem`. A marker with no enclosing list item is malformed input: the fold
  drops it and emits a diagnostic — never a silent drop.
- **Metadata block.** `MetadataBlock` is stored verbatim into
  `DocumentMetadata.frontmatter` with its detected `FrontmatterFormat`. The
  fold does **not** parse YAML/TOML/JSON; structured parsing stays darkmatter's
  existing concern.
- **Math & definition lists.** If unsupported for v1 they fold to
  `Unsupported`. Fixtures using them **must** assert a `Strict`-mode diagnostic,
  so an unsupported construct can never become an accidental silent loss.

## The events → tree fold

Implemented in **`darkmatter`** (it owns `pulldown-cmark` and the wrapping
processors). A stack-based pass converts the event stream into a `Document`:

- Maintain a stack of in-progress nodes.
- `Start(tag)` → push a partially-built node.
- `End(tag)` → pop, compute its `SourceSpan`, append to the new stack top.
- Leaf events (`Text`, `Code`, `SoftBreak`, `HardBreak`, `Rule`, …) → append a
  leaf to the top.
- The fold consumes the **same wrapped event stream** darkmatter already builds
  (`pulldown-cmark` → `InlineStyleProcessor` → `RuleProcessor`), so the custom
  inline tags and HR-with-attributes are folded as ordinary nodes per the
  inventory.

**Totality policy.** The fold is total in the Rust sense *and loud*:

- Every event resolves to one of the inventory dispositions.
- **`Noise` is reserved for events proven to be purely structural.** No event
  is dropped merely because the enum lacks a variant — that becomes
  `Unsupported` plus a diagnostic.
- The fold returns `(Document, Vec<Diagnostic>)`. `Unsupported` and `Lossy`
  dispositions emit diagnostics so the canonical path can never be *quietly*
  less correct than the legacy event path.
- `into_offset_iter()` supplies byte ranges; the fold assigns `provenance =
  Parsed` and the document's `SourceId`.

## Target renderers

Each target is a renderer over `RenderNode`. **They are not built on a
default-recursing `Visitor`.** A default "recurse into children" is unsafe for a
rendering layer: it would silently emit a `Link`'s label while dropping its URL,
make `Image` (a leaf) vanish, and erase `Heading` depth — re-introducing exactly
the divergence this feature exists to remove.

Instead:

- **Render renderers use an exhaustive `match` on `NodeKind`.** The compiler is
  the exhaustiveness gate: adding a `NodeKind` variant breaks every renderer
  until it makes a deliberate decision. No silent fallback exists.
- A default-recursing traversal helper *may* be provided separately for
  **transform** passes, where "recurse by default, touch a few kinds" is the
  correct shape. Render and transform traversal are different jobs.

### Options, context, and output

Renderers do **not** reuse `MarkdownOptions` / `PageOptions` directly:
`PageOptions` is *page-assembly* configuration, and `MarkdownOptions` predates
the strictness model. Each target gets a dedicated render-options type, which
may embed the older type where page assembly is genuinely needed:

```rust
pub struct MarkdownRenderOptions {
    pub dialect: MarkdownDialect,        // Markdown | MarkdownPlus
    pub strictness: RenderStrictness,
    pub style: Option<MarkdownStyleOptions>,
}

pub struct BrowserRenderOptions {
    pub strictness: RenderStrictness,
    pub raw_html: RawHtmlPolicy,
    pub page: Option<PageOptions>,       // page assembly, when rendering a full page
}
```

`biscuit-terminal` likewise defines `TerminalRenderOptions`, wrapping a
`TerminalRenderContext` (terminal width, color depth, hyperlink mode,
image-protocol support, layout, theme — the state the existing
`TerminalRenderable` path depends on) plus `strictness`.

Renderers come in **two layers** — node-level and document-level. The fold
produces a `Document` (metadata + source registry); components produce a bare
`RenderNode`. The document-level functions exist so document metadata
(frontmatter, the `SourceRegistry` behind diagnostics) reaches the renderer
instead of being silently dropped:

```rust
// renderable — node level
pub fn render_markdown_node(node: &RenderNode, opts: &MarkdownRenderOptions)
    -> Result<Rendered<String>, RenderError>;
pub fn render_browser_node(node: &RenderNode, opts: &BrowserRenderOptions)
    -> Result<Rendered<BrowserFragment<Ready>>, RenderError>;

// renderable — document level (calls the node-level fn internally)
pub fn render_markdown_document(doc: &Document, opts: &MarkdownRenderOptions)
    -> Result<Rendered<String>, RenderError>;
pub fn render_browser_document(doc: &Document, opts: &BrowserRenderOptions)
    -> Result<Rendered<HtmlPage>, RenderError>;

// biscuit-terminal
pub fn render_terminal_node(node: &RenderNode, opts: &TerminalRenderOptions)
    -> Result<Rendered<String>, RenderError>;
pub fn render_terminal_document(doc: &Document, opts: &TerminalRenderOptions)
    -> Result<Rendered<String>, RenderError>;
```

```rust
/// A successful render plus any non-fatal diagnostics raised along the way.
pub struct Rendered<T> {
    pub output: T,
    pub diagnostics: Vec<Diagnostic>,
}
```

**Every `render_*` returns `Result<Rendered<T>, RenderError>`.** This is the
single canonical signature; earlier drafts inconsistently showed a bare
`Rendered<T>` — the `Result` form is authoritative, and every example obeys it.

### Strictness and loss policy

Every renderer takes a `RenderStrictness` mode (carried in its options type):

| Mode      | Behavior                                                            |
|-----------|---------------------------------------------------------------------|
| `Strict`  | Any `Unsupported` node or lossy conversion → `Err`.                 |
| `Warn`    | Best-effort output; loss recorded as `Diagnostic`s.                 |
| `Lossy`   | Best-effort output; silent degradation only for *documented* cases. |

`Strict` yields `Err`; `Warn`/`Lossy` yield `Ok` with diagnostics. Rendering is
**not** assumed infallible: missing highlight languages, unsupported nodes, and
invalid spans are real conditions that must be visible, not panicked or
swallowed.

### Markdown vs. MarkdownPlus loss policy

- **`MarkdownPlus`** may emit `Html` nodes and inline HTML for structure GFM
  cannot express (styled spans, colspan, etc.).
- **Plain `Markdown`** cannot. Encountering an `Html` node or a `Span` with no
  Markdown equivalent is governed by strictness: `Strict` → error; `Warn` →
  degrade (escape or drop) with a diagnostic; `Lossy` → documented degrade.
  **Silent drop is never the default.**
- The `text → tree → Markdown` round-trip target is **semantic stability**, not
  byte-stability — fixtures assert meaning is preserved, not exact bytes.

## Validation & builders

The tree deliberately has no type-level inline/block split, so invalid trees
are constructible (a `Root` inside a `Paragraph`, a `TableCell` outside a
`TableRow`). This becomes a live risk the moment components splice their own
subtrees into parsed documents — so validation is **part of the spine**.

Findings carry a **severity**, because not every structural anomaly is fatal:

```rust
pub enum Severity {
    /// Structurally invalid — a block node inside a phrasing-only
    /// container, an orphaned `TableCell`, an out-of-range heading depth.
    Error,
    /// Renderable but suspect — a table row with a mismatched cell count,
    /// an `Unsupported` node present.
    Warning,
}

pub enum ValidationMode {
    /// Collect every finding.
    Full,
    /// Stop at the first `Error`.
    FailFast,
}

pub fn validate(node: &RenderNode, mode: ValidationMode) -> ValidationReport;

/// Convenience: `Ok` iff `validate` finds no `Error`-severity finding.
pub fn ensure_valid(node: &RenderNode) -> Result<(), ValidationError>;
```

**Renderer policy:** each `render_*` calls `ensure_valid` internally and folds
a structural `Error` into `RenderError` *regardless of strictness* — a
structurally invalid tree is never rendered. `Warning`-severity findings follow
the strictness model (surfaced as `Diagnostic`s, escalated to `Err` under
`Strict`).

Builders make valid construction the easy path and keep `RenderNode`'s public
fields from being assembled by hand:

```rust
RenderNode::root(children)
RenderNode::paragraph(inline_children)
RenderNode::text("…")
RenderNode::heading(HeadingDepth::new(2)?, children)
// builders set `span` to `SourceSpan { provenance: Synthetic, location: None }`
// and `attrs` to empty.
```

## Component integration

A component produces a tree by implementing **`TreeRenderable`** (this
supersedes the placeholder `AstRenderable`):

```rust
pub trait TreeRenderable {
    /// Produces this component's document-structural subtree.
    fn render_tree(&self) -> RenderNode;
}
```

Multi-target support is **opt-in and explicit** — there are **no blanket
impls**. Implementing `TreeRenderable` does not confer `BrowserRenderable` /
`MarkdownRenderable` / `TerminalRenderable`; a component exposes only the
targets it implements.

What tree rendering removes is the need to *hand-write a target's
document-structure renderer*. It does **not** make every target trait a literal
one-liner — the trait contracts differ, and the earlier draft's one-line
`BrowserRenderable` example did not even compile:

- **Markdown** has no trait friction: call `render_markdown_node` and handle
  the `Result`.
- **`BrowserRenderable::render_html_fragment` is infallible** — it returns
  `BrowserFragment<Ready>`, no `Result`, and the trait also requires `as_any`.
  A delegating impl must therefore define an **error policy**: render in
  `Warn`/`Lossy` mode (which yields `Ok`) and surface diagnostics as a fallback
  fragment. `Strict` mode is unreachable through an infallible trait — a
  documented limitation, not a one-liner.
- **`TerminalRenderable` needs layout.** It requires `layout` / `layout_mut` /
  `as_any` and owns layout state a bare `RenderNode` does not supply.

For the layout-ownership and error-policy boilerplate, a reusable adapter is
provided rather than re-derived per component. Because it carries the
`TerminalRenderable` impl, the adapter lives in **`biscuit-terminal`** (which
owns that trait and the layout type), consistent with the Terminal renderer's
placement:

```rust
/// biscuit-terminal — wraps a `TreeRenderable` component, owning the layout
/// and strictness state the terminal trait needs.
pub struct TreeComponent<T: TreeRenderable> {
    pub inner: T,
    pub layout: Layout,
    pub strictness: RenderStrictness,
}
```

`TreeComponent<T>` carries the target-trait impls; an author either wraps their
type in it or copies the small, mechanical delegation. The honest claim: tree
rendering makes multi-target support **cheap and uniform**, not literally one
line for traits whose contracts predate it.

`AstRenderable` (current placeholder, `render_ast -> String`, no implementors)
is **removed and replaced** by `TreeRenderable`. String serialization of a tree
is `serde` on a `RenderNode`, not a trait method.

## Scope & sequencing

The earlier "all `renderable` walkers first, fold later" plan is **rejected**:
it would harden the public tree API before any real parser event had touched
it, risking a polished-but-wrong center. The fold is the part that proves the
vocabulary is sufficient — so every milestone is a **vertical slice** that
exercises the tree end-to-end. Milestones cross crate boundaries; that is
expected and fine (the crate split is about *where code lives*, not *when it
ships*).

**Milestone 1 — vertical slice (the riskiest unknowns first):**

1. `RenderNode`, `NodeKind`, `NodeAttrs`, `SourceSpan`, `Document`,
   `HeadingDepth`, `ColumnAlign`.
2. The **verified, exhaustive** `pulldown-cmark` 0.13 event inventory table.
3. A minimal darkmatter fold covering the common-event subset.
4. The Markdown renderer with `Strict` and `Warn`/`Lossy` modes.
5. Golden `text → tree → Markdown` round-trip fixtures (semantic stability).
6. `validate` and the core builders.

**Milestone 2 — completeness + Browser:**

7. Fold coverage for *every* inventory event (footnotes, HTML block/inline,
   task lists, custom inline styles, HR attributes).
8. The Browser renderer.
9. `TreeRenderable` + delegation pattern; first component adoption
   (`BlockQuote`).

**Milestone 3 — Terminal:**

10. The Terminal renderer in `biscuit-terminal` over `RenderNode`.
11. Parity fixtures against the existing `for_terminal` output.

**Deferred to later features:**

- Migrating darkmatter's `as_html` / `for_terminal` onto the tree.
- Re-homing `compose/` transformations as tree-rewrite passes (the node model
  reserves the hooks: `SourceSpan` provenance, `NodeAttrs`, `DocumentMetadata`).
- An MDAST-JSON adapter for `as_ast`, if external consumers need it.

## Testing & parity gates

Before any darkmatter renderer migrates onto the tree:

- **Round-trip fixtures:** `text → tree → Markdown`, semantic stability.
- **Serialization fixtures:** the whole public JSON surface, not just node
  shapes — every `NodeKind`; `SourceSpan` across each `Provenance` (`Parsed`,
  `Synthetic`, `Generated`, `Transcluded`) and `SourceLocation`; `NodeAttrs`;
  `Document` including `SourceRegistry` and `DocumentMetadata`; `Diagnostic`;
  and `Unsupported` nodes.
- **Coverage tests:** every inventory event exercised — links, images, code
  fences, tables (including the header-row convention), task lists, footnotes,
  raw HTML (block + inline), custom inline styles, HR attributes.
- **Unsupported-construct tests:** fixtures using math, definition lists, or any
  `Unsupported` disposition must assert a `Strict`-mode diagnostic — a construct
  can never silently disappear.
- **Diagnostics tests:** `Unsupported` / `Lossy` dispositions raise diagnostics;
  `Strict` mode converts them to errors; a structural `Error` fails rendering
  regardless of strictness.
- **Benchmarks:** large code blocks, large tables, deeply nested lists, many
  links/images, transcluded/generated content, repeated component subtrees —
  to keep the owned-tree memory cost honest (see Risks).

## Risks & open questions

- **`markdown` (alpha) crate.** Decision: do not build the hub on it.
  `RenderNode` serializes to its own format; `as_ast` MDAST compatibility, if
  required, is a separate adapter.
- **Memory.** An owned tree holds the whole document, and owned strings copy
  text out of `pulldown-cmark` events. The new design also encourages component
  subtrees, transcluded documents, and generated content — so the cost is
  broader than "documents are small." Mitigation: the benchmark fixtures above;
  expected-usage boundaries documented alongside them.
- **Correctness parity.** The legacy event HTML path has accumulated behavior
  (code-fence DSL, `ImageRef`/`Link` structured parsing, mermaid, syntect).
  Tree renderers must reach fixture parity before darkmatter cuts over — hence
  the staged, non-rewrite sequencing.
- **Container-span computation.** Assigning a faithful `SourceSpan` to
  container nodes (range from first child to end event, around blank
  lines/nesting) is subtle — a defined fold responsibility, validated by
  fixtures.

### Open questions

- Exact `SourceDescriptor` contents — how a component origin is named, whether
  file paths are absolute or workspace-relative. Resolve during Milestone 1.
- Whether `RenderError` is one type with a kind enum or per-target error types.
  Resolve once the first two renderers exist.

### Resolved

- **Node model shape.** Envelope struct (`RenderNode`) + payload enum
  (`NodeKind`); `span` and `attrs` on the envelope; children in variants,
  uniformly named `children`. See [The render tree type](#the-render-tree-type).
- **`render_ast` signature.** `AstRenderable` is removed; replaced by
  `TreeRenderable { fn render_tree(&self) -> RenderNode }`.
- **Terminal walker placement.** Lives in `biscuit-terminal`, not `renderable`.
- **MDAST compatibility.** Not a goal; `RenderNode` has its own serialization
  format.
- **`SourceId` representation.** A small interned handle into a `SourceRegistry`
  owned by `Document` — not a path/URL embedded per node.
- **`SourceSpan` shape.** Provenance always present; `SourceLocation` optional,
  so synthetic/generated nodes carry no fake byte range.
- **`DocumentMetadata`.** A typed struct holding raw frontmatter
  (`Frontmatter { format, raw }`); structured YAML/TOML/JSON parsing stays in
  darkmatter.
- **Renderer signatures.** `Result<Rendered<T>, RenderError>`, in node-level and
  document-level layers, with dedicated `*RenderOptions` types — not the
  pre-existing `MarkdownOptions` / `PageOptions`.
- **`NodeAttrs` styling.** No `CssStyle` in the core; styling is semantic via
  `classes`; raw CSS lives in namespaced `attrs.data`.
