# Render Tree — Spec

**Date:** 2026-05-15
**Updated:** 2026-05-16
**Status:** Draft / design discussion
**Areas:** `renderable` (tree + walkers), `darkmatter` (Markdown→tree fold)

## Summary

Introduce a canonical, owned **render tree** that sits between content sources
and render targets. Document-structural components render *into* this tree;
generic walkers render *out of* it to Terminal, Browser, and Markdown.

The work splits across two crates along the existing dependency graph:

- **`renderable`** owns the `RenderNode` tree type and the generic
  `tree → {Terminal, Browser, Markdown}` walkers.
- **`darkmatter`** owns the **events → tree fold** that turns a
  `pulldown-cmark` event stream into a `RenderNode`.

This replaces the current "parse text, walk events, emit strings — once per
target" model with "parse once, build one tree, walk it per target."

## Motivation

### The problem today (darkmatter)

darkmatter has **two independent Markdown pipelines using two parser crates**:

| Output                | Crate                  | Shape                          |
|-----------------------|------------------------|--------------------------------|
| `as_html`, `for_terminal` | `pulldown-cmark` 0.13 | Streaming events — no tree     |
| `as_ast`              | `markdown` 1.0-alpha   | MDAST tree                     |

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
   decide presentation. Required for "one component, faithful on many targets."
2. **Additive targets.** A new target is a new visitor with a generic fallback,
   not a full re-implementation of every node type.
3. **Transformable.** Tree-rewrite passes are composable and testable; stacked
   stream-mutating iterators are not.
4. **Component embedding.** A component that produces a tree node can be
   *spliced into the tree*. A component cannot be cleanly spliced into an event
   stream — it would have to emit each consumer's event dialect.

### Key realization: keep `pulldown-cmark`

`pulldown-cmark` is a **pull/event parser** by deliberate design — fast,
low-allocation, stable, and the de-facto standard. "Event parser" is not the
opposite of "tree." Events can be **folded into a tree** with a simple
stack-based pass.

Therefore: **keep `pulldown-cmark` as the parser.** Do not adopt the alpha
`markdown` crate as the rendering hub. Build the canonical tree from
`pulldown-cmark` events into a node type `renderable` owns.

## Goals

In `renderable`:

- Define an owned render-tree node type (`RenderNode`).
- Provide the generic `tree → {Terminal, Browser, Markdown}` walkers.
- Establish the tree as the single canonical representation every target
  renderer walks.
- Make multi-target support for document-structural components nearly free:
  implement once against the tree, derive the rest generically.

In `darkmatter`:

- Provide the **events → tree** fold from a `pulldown-cmark` event stream.
- Parse source text **once**, not three times.

## Non-goals

- Rewriting darkmatter's existing renderers in this feature. The tree and fold
  land first; darkmatter migration is a separate, later, incremental effort.
- Replacing `pulldown-cmark`.
- Forcing every component to be multi-target. Target traits stay separate and
  opt-in.
- Bespoke/visual components (`TerminalImage`, `GraphExpression`) are **not**
  required to route through the tree — direct per-target trait impls remain a
  supported escape hatch.

## Architecture

```text
   darkmatter                          renderable
  ┌───────────────────────────┐   ┌───────────────────────────────┐
  │  source ─▶ pulldown-cmark │   │                               │
  │  text      │              │   │       ┌──────────────┐         │
  │            ▼ events       │   │       │  RenderNode  │         │
  │   events → tree fold ─────┼───┼─────▶ │  (the tree)  │         │
  │   (stack-based)           │   │       └──────────────┘         │
  └───────────────────────────┘   │              │                │
                                  │   ┌──────────┼──────────┐      │
   components (biscuit-terminal)  │   ▼          ▼          ▼      │
  ┌───────────────────────────┐  │ tree→Term  tree→Browser tree→MD │
  │ Table, BlockQuote, … ─────┼──┼▶  (ANSI)    (HTML)   (MD / MD+) │
  │   render_ast → RenderNode │  └───────────────────────────────┘
  └───────────────────────────┘
```

- Parse once. Fold once. Walk per target.
- **`RenderNode` producers** are symmetric: a parsed Markdown document (via the
  darkmatter fold) and a component (via `render_ast`) both yield the *same*
  tree. The Markdown document is not privileged — it is just one tree source.
- **Target renderers** are the tree's consumers and live entirely in
  `renderable`.
- `MarkdownPlus` = a tree walk to Markdown that is permitted to emit `Html`
  nodes for structure GFM cannot express; plain `Markdown` emits none.

## Crate boundaries

The split follows the dependency graph
(`darkmatter → biscuit-terminal → renderable`; `renderable` depends on
neither):

| Piece                              | Crate              | Why                                                                                       |
|------------------------------------|--------------------|-------------------------------------------------------------------------------------------|
| `RenderNode` type + supporting types | `renderable`     | Components in `biscuit-terminal` must produce it; placing it in `darkmatter` would cycle. |
| Generic `tree → target` walkers    | `renderable`       | Belong with the target traits they serve (`BrowserRenderable`, etc.), already here.       |
| `pulldown-cmark` events → tree fold | `darkmatter`      | Markdown- and `pulldown-cmark`-specific; that crate already owns the parser + processors. |

`RenderNode` is the typed value `AstRenderable::render_ast` was always going to
grow into — it is a render-content concept, not a Markdown concept. `renderable`
gains **no** `pulldown-cmark` dependency.

## The render tree type

`RenderNode` lives in `renderable` (replacing the placeholder in `ast.rs` /
filling the empty `ast_utils.rs`). It mirrors **MDAST** vocabulary so it stays
structurally interchangeable with the wider syntax-tree ecosystem.

### Shape: envelope struct + payload enum

The node is an **envelope struct** carrying cross-cutting metadata, plus a
`NodeKind` payload enum carrying kind-specific data:

```rust
/// A node in the canonical render tree.
pub struct RenderNode {
    pub kind: NodeKind,
    /// Source byte range, when the node originated from parsed text.
    /// `None` for synthetic nodes emitted by components.
    pub position: Option<Position>,
}

pub enum NodeKind {
    Root      { children: Vec<RenderNode> },

    // Block content
    Heading   { depth: HeadingDepth, children: Vec<RenderNode> },
    Paragraph { children: Vec<RenderNode> },
    BlockQuote { children: Vec<RenderNode> },
    List      { ordered: bool, start: Option<u64>, items: Vec<RenderNode> },
    ListItem  { checked: Option<bool>, children: Vec<RenderNode> },
    Code      { lang: Option<String>, meta: Option<String>, value: String },
    ThematicBreak,
    Table     { align: Vec<ColumnAlign>, rows: Vec<RenderNode> },
    TableRow  { children: Vec<RenderNode> },
    TableCell { children: Vec<RenderNode> },

    // Inline content
    Text       { value: String },
    Emphasis   { children: Vec<RenderNode> },
    Strong     { children: Vec<RenderNode> },
    Delete     { children: Vec<RenderNode> },
    InlineCode { value: String },
    Link       { url: String, title: Option<String>, children: Vec<RenderNode> },
    Image      { url: String, title: Option<String>, alt: String },
    Break,

    // Escape hatch — raw HTML for MarkdownPlus / passthrough
    Html { value: String },
}

impl RenderNode {
    /// Children of any node; `&[]` for leaf nodes.
    pub fn children(&self) -> &[RenderNode] { /* match on kind */ }
    pub fn children_mut(&mut self) -> Option<&mut Vec<RenderNode>> { /* … */ }
}
```

### Rationale

- **Envelope struct, not a flat enum.** `position` is cross-cutting — every
  node can carry it. On the envelope it is declared once and read without a
  `match`; on a flat enum it would be repeated across ~20 variants. The
  envelope also makes adding `data` later a one-line change with zero churn to
  variants or match arms.
- **`position` included now; `data` deferred.** `position` is free — the
  darkmatter fold gets a byte range per event from `pulldown-cmark`'s
  `into_offset_iter()` — and darkmatter already does source-positioned error
  reporting. A generic `data`/annotation slot is deferred until the first
  tree-rewrite pass (`compose/` migration) needs it; the envelope makes that
  addition cheap.
- **Children live in the variants, not the envelope.** Leaf nodes (`Text`,
  `Code`, `Image`) have no children; a uniform `children` field would be a
  lie walkers must remember not to populate. Uniform traversal is recovered
  with the `children()` / `children_mut()` accessors instead.
- **Struct variants everywhere**, even single-field ones (`Text { value }`).
  Named fields read better at match sites, are forward-compatible, and serde
  serializes them to named JSON fields — required for MDAST-shape parity.
- **Constrained newtypes.** `HeadingDepth` (1–6), not raw `u8`. Same principle
  for any field with a real domain.
- **One node type — no inline/block split.** A type-level `BlockNode` /
  `InlineNode` split would enforce phrasing rules but double the type surface
  and complicate the fold, walkers, and components. MDAST does not split;
  structural validity, if needed, is a separate validation pass.
- **Owned strings** (`String`, not `CowStr`). The fold converts at the
  boundary; an owned tree carries no lifetime parameter.
- **serde with MDAST tagging.** `#[serde(tag = "type", rename_all =
  "camelCase")]` on `NodeKind` so JSON matches MDAST vocabulary (`tableCell`,
  etc.), keeping `as_ast` interchangeable.

Alignment is table-level (`Vec<ColumnAlign>`), matching MDAST and the `Table`
component's per-column model.

## The events → tree fold

Implemented in **`darkmatter`** (it owns `pulldown-cmark` and the wrapping
processors). A single stack-based pass converts a `pulldown-cmark` event stream
into a `renderable::RenderNode::Root`:

- Maintain a stack of in-progress nodes.
- `Event::Start(tag)` → push a new partially-built node.
- `Event::End(tag)` → pop the node, append it to the new top of stack.
- `Event::Text` / `Event::Code` / `Event::Html` → append a leaf to the top.
- `Event::SoftBreak` / `HardBreak` / `Rule` → append the corresponding leaf.

Notes:

- The fold consumes the **same wrapped event stream** darkmatter already builds
  (`pulldown-cmark` parser → `InlineStyleProcessor` → `RuleProcessor`), so the
  custom inline tags (`==mark==`, `⌄dim⌄`) and horizontal-rule-with-attributes
  become ordinary tree nodes instead of bolted-on iterator adapters.
- Raw HTML events fold into `RenderNode::Html` (opaque, not parsed — consistent
  with how MDAST treats inline HTML).
- The fold is total: any unhandled event becomes a no-op or a documented
  fallback, never a panic.

## Target walkers

Each target (Terminal, Browser, Markdown) is a generic walk of `RenderNode`,
implemented in `renderable` as a **`Visitor` trait with default methods**:

- One `visit_*` method per `NodeKind` variant, each defaulting to "recurse into
  `children()`".
- A target overrides only the variants it renders specially and inherits sane
  recursion for the rest — this is what makes adding a target *additive*
  rather than a full per-node-type re-implementation.

## Component integration

- Document-structural components implement `render_ast` (typed to return
  `renderable::RenderNode` rather than the current placeholder `String`).
- A component's `RenderNode` sub-tree can be spliced directly into a tree
  produced by the darkmatter fold — producers are symmetric (see Architecture).
- Generic `tree → {Terminal, Browser, Markdown}` walkers then render the
  component for free — the author writes one method, not three renderers.
- `TerminalRenderable` / `BrowserRenderable` / `MarkdownRenderable` remain
  **separate, opt-in traits**. Components for which a target is bespoke or
  meaningless implement only what applies.

## Scope & sequencing

This work delivers the **spine**, not the migration. It is two units of work in
dependency order:

**Unit 1 — `renderable` (this feature):**

1. `RenderNode` type + supporting types (`ColumnAlign`, etc.).
2. Generic `tree → Markdown` walker first — closest to the tree, and the
   `darkmatter` fold round-trips against it as a cheap correctness test
   (text → tree → Markdown).
3. Generic `tree → Terminal` and `tree → Browser` walkers.

**Unit 2 — `darkmatter` (follow-on feature):**

4. `pulldown-cmark` events → `renderable::RenderNode` fold.
5. Round-trip and parity tests against the Unit 1 walkers.

Explicitly deferred to later features:

- Migrating darkmatter's `as_html` / `for_terminal` onto the tree.
- Re-homing `compose/` transformations as tree-rewrite passes.
- Retiring the alpha `markdown` crate dependency / re-pointing `as_ast`.

Building any single component's `BrowserRenderable` as a one-off **before** the
tree exists is discouraged — it would be thrown away once the hub lands.

## Risks & open questions

- **`markdown` (alpha) crate.** Decision: do not build the hub on it. Open: does
  any external consumer depend on the exact MDAST JSON shape of `as_ast`? If so,
  `RenderNode` serialization must stay shape-compatible, or `as_ast` keeps the
  alpha crate as a thin separate path.
- **Memory.** A tree holds the whole document; event streaming is O(1)-ish.
  Negligible at darkmatter document sizes, but noted.
- **Correctness parity.** The event HTML path has accumulated behavior
  (code-fence DSL, `ImageRef`/`Link` structured parsing, mermaid, syntect).
  Tree walkers must reach parity before darkmatter cuts over — hence the
  staged, non-rewrite sequencing.
- **`render_ast` signature.** Currently returns `String` (placeholder). Moving
  to `RenderNode` is a breaking change to the `AstRenderable` trait — acceptable
  while it is still a placeholder with no implementors.

### Resolved

- **Node model shape.** Resolved: envelope struct (`RenderNode`) + payload enum
  (`NodeKind`), `position` carried now, `data` deferred, children in variants.
  See [The render tree type](#the-render-tree-type).
