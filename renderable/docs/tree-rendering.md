# Tree Rendering

This document describes the render-tree architecture introduced into the
`renderable`, `darkmatter`, and `biscuit-terminal` crates, how it relates to
the rendering paths that already exist, what is and is not covered by tests,
and a roadmap for adopting it.

It is a status-and-direction document. It is deliberately honest about what has
been *proven* versus what has only been *wired up*.

## 1. The tree-rendering architecture

The render tree is a **canonical, owned, target-agnostic representation** of a
document. Content sources produce it; render targets consume it. The slogan is
*parse once, build one tree, walk it per target*.

### Core model — `renderable::tree`

`renderable` owns the model so that every other crate can depend on it without
a dependency cycle.

- `RenderNode { kind: NodeKind, span: SourceSpan, attrs: NodeAttrs }` — the node
  envelope. `span` carries provenance (and an optional byte range); `attrs`
  carries identity (`id`), semantic `classes`, and namespaced extension `data`.
- `NodeKind` — a 25-variant payload enum covering document structure: `Root`,
  `Heading`, `Paragraph`, `BlockQuote`, `List`, `ListItem`, `Code`,
  `ThematicBreak`, `Table`, `TableRow`, `TableCell`, `FootnoteDefinition`,
  `Text`, `Emphasis`, `Strong`, `Delete`, `Span`, `InlineCode`, `Link`, `Image`,
  `FootnoteReference`, `SoftBreak`, `HardBreak`, `Html`, and `Unsupported`.
- `Document { sources: SourceRegistry, metadata: DocumentMetadata, root }` — the
  full document wrapper, with a source registry and frontmatter slot.
- `HeadingDepth`, `ColumnAlign` — constrained newtypes.

The whole public surface is `serde`-serializable to its own documented JSON
format (not MDAST-compatible).

### Producers

A tree has two kinds of producer, and the design treats them symmetrically:

1. **The fold** (in `darkmatter`) — turns a parsed document into a `Document`.
2. **`TreeRenderable` components** — a component implements
   `TreeRenderable { fn render_tree(&self) -> RenderNode }` to project itself
   into a document-structural subtree.

### Consumers — the renderers

Three renderers walk a `RenderNode` / `Document` with an **exhaustive `match`**
over `NodeKind` (no default-recursing visitor — adding a variant must break
every renderer until it makes a deliberate decision):

| Target   | Entry points                                          | Crate             |
|----------|-------------------------------------------------------|-------------------|
| Markdown | `render_markdown_node` / `render_markdown_document`   | `renderable`      |
| Browser  | `render_browser_node` / `render_browser_document`     | `renderable`      |
| Terminal | `render_terminal_node` / `render_terminal_document`   | `biscuit-terminal`|

The Terminal renderer lives in `biscuit-terminal` because a meaningful terminal
renderer needs `Terminal`, `Layout`, color depth, and OSC8 — types `renderable`
cannot depend on. `renderable` gains **no** `pulldown-cmark` or
`biscuit-terminal` dependency; the dependency direction
(`darkmatter` → `biscuit-terminal` → `renderable`) is preserved.

### Shared rendering contract

Every renderer follows the same shape:

- It **validates first** (`validate` / `ensure_valid`). A structural `Error`
  fails the render regardless of strictness; warnings become diagnostics.
- It honors a `RenderStrictness` mode — `Strict` (any loss is an error),
  `Warn` (best-effort output plus diagnostics), `Lossy` (documented degrade).
- It returns `Result<Rendered<T>, RenderError>`, where `Rendered<T>` bundles the
  output with any non-fatal `Diagnostic`s.

`Unsupported` is a real, visible node — never a silent drop.

### The component adapter

`biscuit-terminal` provides `TreeComponent<T: TreeRenderable>`, which wraps a
`TreeRenderable` and supplies an (infallible) `TerminalRenderable` impl by
calling `render_tree()` then `render_terminal_node`. It is the bridge that lets
a tree-producing component render to the terminal. It currently bridges **only**
the terminal target.

## 2. The existing darkmatter rendering path

This path is **unchanged** by the render-tree work and is still the public,
shipping behavior.

- `Markdown::as_html(HtmlOptions)` and `for_terminal(&Markdown, TerminalOptions)`
  are hand-written **`pulldown-cmark` event → string** serializers. Each walks
  the event stream itself and emits the target string directly.
- The source text is parsed independently for each pipeline. A third path,
  `as_ast` (built on the alpha `markdown` crate, producing MDAST), exists but is
  a dead end — a tree is produced and handed back, but nothing renders from it.
- The parser is built with `markdown_parse_options()`
  (`ENABLE_TABLES | ENABLE_STRIKETHROUGH`) and wrapped by darkmatter's
  `InlineStyleProcessor` (custom `==mark==` / dim inline styles) and
  `RuleProcessor` (horizontal rules with attributes).
- `compose/` transformations (transclusion, interpolation, TOC linking) are
  implemented as string preprocessing and stream-mutating iterator adapters.

The new fold (`fold_markdown_to_document`) is an **additional, parallel,
experimental** path. It does not touch `as_html` / `for_terminal` / `compose`,
and nothing public routes through it yet.

## 3. The existing structural-component rendering path

Structural components such as `BlockQuote` (and `List`, `Table`, `Section`,
`HorizontalRule`, …) live in `biscuit-terminal` and render with **bespoke,
per-target trait implementations**:

- They implement `TerminalRenderable` directly (`render`, `render_optimistic`,
  `layout`, …). Some also implement `BrowserRenderable`.
- Each impl contains hand-written, target-specific layout and formatting code.

The render-tree work added exactly **one** adoption, as a proof of concept:

- `BlockQuote` now also implements `TreeRenderable`, projecting its content and
  attribution into a `BlockQuote` subtree.
- This impl is **additive**. `BlockQuote`'s original `TerminalRenderable` impl is
  unchanged and is still what runs on `quote.render(&term)`. The two renderers
  are parallel and do not share code.

No other component implements `TreeRenderable`. No component's `TerminalRenderable`
or `BrowserRenderable` impl has been re-pointed at the tree. Inherently visual
components (`TerminalImage`, `GraphExpression`) are intentionally **not**
intended to route through the tree — they keep bespoke renderers permanently.

## 4. Testing coverage — and the gaps

The render tree supports two distinct flows, and they are **not** equally
tested.

### Flow A — parsed document → tree → render (well covered)

This is the darkmatter use case, and it is exercised end-to-end against real
input:

- **Event inventory** — compile-time exhaustive-match tests pin every
  `pulldown-cmark` 0.13 `Event` / `Tag` / `TagEnd` variant to a disposition; a
  parser enum change breaks the build.
- **Fold unit tests** — folding of every Milestone 1 construct plus footnotes,
  HTML-block grouping, and superscript/subscript.
- **Golden round trips** (`render_tree_roundtrip.rs`) — 11 real Markdown
  fixtures folded, structurally asserted, rendered back through the Markdown
  renderer, and snapshotted; plus a serialized `Document` JSON-surface snapshot.
- **Parity gates** (`render_tree_parity.rs`) — the new pipeline
  (`fold → render_browser/terminal_document`) is run against real input **and
  compared, on semantic invariants, to the legacy `as_html` / `for_terminal`
  output**. This is the strongest evidence the new renderers are faithful.
- **Benchmarks** — fold + render stress benchmarks in both `darkmatter` and
  `biscuit-terminal`.

Flow A is genuinely proven at the test level, not theoretical.

### Flow B — component → tree → render (proof of concept only)

This is the `TreeRenderable` use case, and coverage is thin:

- Only `BlockQuote` is tested, with 8 happy-path tests: tree-structure
  assertions (root kind, child counts), text extraction from a `Prose`, and
  rendering the projected tree out through the Markdown and Browser renderers
  (`render_markdown_node` → `> Quoted text`; `render_browser_node` → contains
  `<blockquote>`).
- `TreeComponent` is unit-tested, but with synthetic stub types — not with a
  real component.

These tests prove the **cross-crate plumbing works** — a real component can
produce a valid tree and that tree renders. They do **not** prove the tree
approach can faithfully *replace* a component's existing renderers.

### Known gaps

- **No component-side parity test.** Nothing compares a component's bespoke
  output (`BlockQuote::render(&term)`) against its tree-routed output
  (`TreeComponent::new(quote).render(&term)`). Flow A has exactly this kind of
  parity gate (Phase 11); Flow B does not. Until it exists, "components can
  migrate to the tree" is asserted-by-design, not demonstrated. **Adding this
  test — render the same component both ways and assert semantic equivalence —
  is the single highest-value next step for Flow B**, and should be the gate
  before any further component adoption.
- **No production wiring.** `as_html`, `for_terminal`, and every component
  `.render()` still run legacy code. The tree path is reachable only from tests.
- **The component terminal-via-tree path is untested for `BlockQuote`** — its
  tree tests cover Markdown and Browser only.
- **Lossy projection fidelity is untested.** `BlockQuote`'s text extraction from
  a component is documented as lossy (ANSI stripped, structure collapsed); no
  test characterizes what is lost.
- **Hard components are unprojected.** `BlockQuote` is nearly the simplest
  structural component. Tables, nested lists, and mixed inline/block content —
  the cases that would actually stress the `NodeKind` vocabulary from the
  component side — have no `TreeRenderable` impl.
- **Deferred fold coverage.** Custom darkmatter inline styles (`==mark==`, dim)
  and horizontal-rule attributes are not folded yet, because darkmatter's
  `InlineStyleProcessor` / `RuleProcessor` discard source offsets; reconciling
  that with span-carrying nodes is an open design decision.

## 5. Roadmap for integration

This is a direction sketch, not a detailed plan. Each step should land as its
own feature with its own parity gate.

### Near term — close the Flow B confidence gap

1. **Add the component-side parity test** for `BlockQuote`: bespoke
   `TerminalRenderable` vs `TreeComponent` through the tree, asserting semantic
   equivalence. This converts "the plumbing runs" into "the tree can replace a
   renderer."
2. **Build the `BrowserRenderable` tree adapter.** `TreeComponent` only bridges
   the terminal today. The browser adapter must define an error policy because
   `BrowserRenderable::render_html_fragment` is infallible.

### Component adoption (`biscuit-terminal`)

3. Adopt structural components incrementally — `Section`, `List`, then `Table` —
   each with a `TreeRenderable` impl and its own bespoke-vs-tree parity test.
   Tables and nested lists will genuinely exercise the `NodeKind` vocabulary
   from the producer side.
4. Once a component's parity test is green, **flip it**: replace its bespoke
   `TerminalRenderable` / `BrowserRenderable` bodies with delegation through the
   tree, deleting the duplicated per-target formatting code.
5. Leave inherently visual components (`TerminalImage`, `GraphExpression`) on
   bespoke renderers — they are out of scope for the tree by design.

### darkmatter migration

6. **Resolve the deferred fold work** — decide how the offset-destroying
   `InlineStyleProcessor` / `RuleProcessor` reconcile with span-carrying nodes,
   then fold `==mark==`, dim, and HR-with-attributes.
7. **Expand the parity fixtures** until the tree pipeline reaches accepted
   parity with `as_html` / `for_terminal` across the real darkmatter corpus
   (the Phase 11 harness already surfaces mismatches — e.g. legacy
   `for_terminal` silently drops raw block HTML).
8. **Migrate the public render paths** behind the parity gate: re-point
   `as_html` at `fold → render_browser_document` and `for_terminal` at
   `fold → render_terminal_document`. Watch the existing render/compose
   benchmarks — the owned tree is a heavier cost profile than the streaming
   serializers.
9. **Re-home `compose/`** transformations (transclusion, interpolation, TOC
   linking) as composable tree-rewrite passes; the node model already reserves
   the hooks (`SourceSpan` provenance, `NodeAttrs`, `DocumentMetadata`).
10. **Retire or adapt `as_ast`** — either drop the dead MDAST path or implement
    a dedicated `Document → MDAST JSON` adapter if external consumers need it.

### Guiding principle

Every migration step — component or darkmatter — should be gated by a parity
test against the renderer it replaces, the same discipline Phase 11 applied to
the darkmatter pipeline. The tree is adopted *because a parity gate proved it
faithful*, never on the assumption that it is.
