# Tree Rendering

This is an introduction to the **IR-based rendering strategy** shared by the
`renderable`, `biscuit-terminal`, and `darkmatter` crates. It explains the
render tree (our intermediate representation), where trees come from, how they
become Terminal / Browser / Markdown output, and the design principles that hold
the whole thing together.

It is a conceptual overview. For the API surface, see the linked skill docs and
[`layout-and-style.md`](./layout-and-style.md); for the catalog of components and
their target support, see [`components.md`](./components.md).

## The core idea

A document can come from more than one place (parsed Markdown, a hand-built
component) and needs to render to more than one place (a terminal, a browser, a
Markdown file). Wiring every source directly to every target is an N×M tangle,
and each pairing tends to grow its own subtly different rendering rules.

The render tree breaks that tangle with a single intermediate representation:

> **Build one canonical tree, then walk it once per target.**

Everything that produces content lowers into the same owned, target-agnostic
tree, and every output format is produced by one renderer that walks that tree.
Producers never think about ANSI or HTML; renderers never think about where the
content came from.

```text
   producers                  the IR                    renderers
 ┌───────────────┐                                   ┌──────────────┐
 │ parsed Markdown│──┐                            ┌──▶│   Markdown   │
 │   (the fold)   │  │     ┌─────────────────┐    │   └──────────────┘
 └───────────────┘  ├────▶│ renderable::tree │────┤   ┌──────────────┐
 ┌───────────────┐  │     │    Document      │    ├──▶│   Browser    │
 │  components    │──┘     └─────────────────┘    │   └──────────────┘
 │(TreeRenderable)│                               │   ┌──────────────┐
 └───────────────┘                                └──▶│   Terminal   │
                                                      └──────────────┘
```

## The render tree — `renderable::tree`

The `renderable` crate owns the model so every other crate can depend on it
without a dependency cycle. The whole surface is `serde`-serializable to its own
documented JSON format (it is *our* IR, not the parser's AST — it is not
MDAST-compatible), which makes snapshotting, inspection, and tooling easy.

> **Serde contract: same-version only.** The render-tree JSON is debug,
> inspection, and same-process persistence output — not a promised cross-version
> durable format. Typed sparse attrs (`layout`, `style`, `text_layout`,
> `browser`, …) may add, rename, or re-shape fields between versions; default
> values are elided, so an alpha-less tree still serializes as it did before
> alpha paint existed. Round-trip a tree only with the version that wrote it.

Three types form the model:

- **`RenderNode { kind, span, attrs }`** — the node envelope.
  - `kind` is the payload (see `NodeKind` below).
  - `span` carries provenance — which source the node came from, and an optional
    byte range, so diagnostics and future transforms can point back at the
    original text.
  - `attrs` (`NodeAttrs`) carries identity (`id`) and semantic `classes`
    alongside **typed sparse fields** — `layout`, `style` (whose color slots are
    alpha-bearing `PaintColor`), `sequence_join`, `list_marker_policy`, the
    per-kind `component` hint group, `text_layout` (unresolved width-dependent
    text intent on link/image/list-item nodes), and `browser` (typed, validated
    browser-target attributes) — see
    [Layout and style](#layout-and-style-on-the-tree). Reads cost no serde
    round-trip. The `data` map is reserved for package-local extension
    namespaces (`darkmatter.*`); a stale `renderable.*` key in `data` is a
    validation error.
- **`NodeKind`** — the payload enum (~27 variants) covering document structure.
  Grouped by role:
  - *Block structure:* `Root`, `Section` (a heading grouped with its body),
    `Heading`, `Paragraph`, `BlockQuote`, `List`, `ListItem`, `Code`,
    `ThematicBreak`, `Table`, `TableRow`, `TableCell`, `FootnoteDefinition`.
  - *Inline content:* `Text`, `Emphasis`, `Strong`, `Delete`, `Span`,
    `InlineCode`, `Link`, `Image`, `FootnoteReference`, `SoftBreak`, `HardBreak`.
  - *Special:* `Html` (raw), `Extended` (a target-agnostic extension node — see
    [Extending the tree](#extending-the-tree)), and `Unsupported` (a real,
    visible placeholder — never a silent drop).
- **`Document { sources, metadata, root }`** — the full document: a
  `SourceRegistry` (provenance), a `DocumentMetadata` slot (frontmatter), and the
  `root` node.

### Crate ownership and dependency direction

`renderable` defines the tree and the Markdown and Browser renderers. The
**Terminal** renderer lives in `biscuit-terminal`, because a real terminal
renderer needs types `renderable` deliberately does not depend on (`Terminal`
capability detection, color depth, OSC8 hyperlinks). `renderable` gains **no**
`pulldown-cmark` or `biscuit-terminal` dependency. The direction is always:

```text
darkmatter ──▶ biscuit-terminal ──▶ renderable
```

## Producers: where trees come from

The design treats two producers symmetrically — both lower into the *same*
`RenderNode` vocabulary:

1. **The fold** (in `darkmatter`) turns parsed Markdown into a `Document`.
2. **`TreeRenderable` components** project themselves into a subtree:
   `fn render_tree(&self) -> RenderNode`.

The important architectural commitment is that these paths **converge at
`RenderNode`**, not at a per-target trait. Parsed Markdown does not render by
constructing component objects, and components do not render by emitting target
strings — both produce tree nodes, and the renderers do the rest.

### The fold (Markdown → tree)

`pulldown-cmark` remains the parser. It is fast, well tested, and event-oriented,
and the tree does **not** require swapping it for an AST parser — the tree is our
owned IR, layered *on top of* the parser's event stream.

```text
Markdown source ─▶ pulldown-cmark events ─▶ darkmatter fold ─▶ Document
                                                                  │
                              ┌───────────────┬───────────┬───────┘
                              ▼               ▼           ▼
                         Markdown /     Browser HTML   Terminal
                         MarkdownPlus
```

The fold is **span-aware**: it attaches source byte ranges to nodes, and it
preserves darkmatter's custom syntax — `==mark==` / dim inline styles (lowered to
`Extended` nodes) and `--- { … }` horizontal-rule attribute directives — with
their offsets intact, so provenance survives into the tree.

Because the fold produces an owned `Document`, the same parse can be rendered
many times:

```text
parse + fold once ─▶ render Terminal
                  ─▶ render Browser HTML
                  ─▶ render MarkdownPlus
                  ─▶ inspect / test / serialize
```

### Components (`TreeRenderable`)

A component implements `TreeRenderable::render_tree()` to project its structure
into the tree. Components share one private projection helper across their
render paths, so a component renders identically whether it is asked for its tree
directly or rendered to a specific target. When a component is nested *inside*
another component's tree, it projects its structural subtree rather than
degrading to "render to ANSI, strip, wrap in text."

Two small adapters bridge a `TreeRenderable` to a per-target trait without the
component author writing target code:

- **`TreeComponent<T>`** supplies a `TerminalRenderable` impl (project, then
  `render_terminal_node`).
- **`BrowserTreeComponent<T>`** supplies the same bridge for `BrowserRenderable`.

## Renderers: tree → output

Each target has a renderer that walks a `RenderNode` / `Document`:

| Target   | Entry points                                        | Crate              |
|----------|-----------------------------------------------------|--------------------|
| Markdown | `render_markdown_node` / `render_markdown_document` | `renderable`       |
| Browser  | `render_browser_node` / `render_browser_document`   | `renderable`       |
| Terminal | `render_terminal_node` / `render_terminal_document` | `biscuit-terminal` |

Every renderer dispatches with an **exhaustive `match`** over `NodeKind` — there
is no default-recursing visitor. Adding a new variant deliberately breaks every
renderer until each one makes an explicit decision about it. This is a guardrail:
the type system refuses to let a target silently ignore new structure.

The Browser target also offers `render_browser_document_html(doc, opts)`: a
direct `Document` → final HTML `String` path that streams the whole tree into one
buffer instead of building an intermediate fragment per node. Its bytes are
identical to composing through `render_browser_document`; reach for it when a
caller already owns a `Document` and only needs the final string.

### The rendering contract

Every renderer follows the same shape, which is the same shape across all three
targets:

- **Validate first.** A structural error fails the render regardless of
  strictness; non-fatal problems become diagnostics.
- **Honor a strictness mode** (`RenderStrictness`):
  - `Strict` — any loss of meaning is an error.
  - `Warn` — best-effort output plus diagnostics.
  - `Lossy` — a documented, intentional degrade.
- **Return `Result<Rendered<T>, RenderError>`**, where `Rendered<T>` bundles the
  output with any non-fatal `Diagnostic`s.

This gives target asymmetry a first-class home. Terminal can express ANSI and
image protocols; Browser can express CSS and richer structure; portable Markdown
can express the least. Strictness and diagnostics make those differences explicit
instead of hiding them inside each renderer.

## Page features — resolving CSS/JS dependencies

A browser render can carry more than markup: a component may declare that it
needs a shared CSS or JavaScript dependency to work. The `renderable` crate
models that as a **page feature** (`renderable::browser::feature`): a
`PageFeature` is a type-safe identity (`Popover`, `MermaidDiagram`, …) that a
component *requests*, and a `FeatureResolver` maps a requested feature to the
concrete `FeatureAssets` (inline CSS, a typed `FeatureScript`, and/or `<link>`
tags) that satisfy it.

### The flow

1. **Request.** A renderer emitting a feature-bearing node calls
   `add_feature(PageFeature::…)` (fragment path) or pushes onto the streaming
   writer's accumulator. Requests are collection-only — a Mermaid fence
   rendered as *code* or a link with no prompt requests nothing.
2. **Collect.** Requests ride the `Rendered<T>` side channel
   (`Rendered.features`, first-seen order) exactly like `diagnostics`.
   `Rendered::map` preserves them, so no renderer transform silently drops the
   channel. Both browser paths — recursive `BrowserFragment` collection and the
   streaming `StreamWriter` (which also merges features from code-renderer hook
   fragments at their document position) — surface the same feature set.
3. **Resolve + inject.** The outermost document assembler deduplicates the
   feature list by variant and resolves each through the installed
   `FeatureResolver`, then serializes the assets exactly once.

### Resolver installation

`HtmlPage` and `BrowserRenderOptions` each own an `Rc<dyn FeatureResolver>` plus
a `FeatureContext`, defaulting to `DefaultFeatureResolver`. A host installs its
own resolver on those entry points to own theme-aware or crate-specific
features (Darkmatter installs `DarkmatterFeatureResolver` on its full-page
browser path to own `MermaidDiagram`). Because the default is generic, a caller
constructing an `HtmlPage` directly gets only the shared assets and acquires no
dependency on the installing crate. `FeatureContext` carries only
renderable-owned values (resolved color mode, resolved semantic colors) so a
resolver in another crate can derive theme-aware assets while the dependency
direction stays `darkmatter → renderable`.

### Ordering

Feature assets are emitted in **first-seen feature order**; within one feature
the order is `<link>`, then `<style>`, then `<script>`. Page-authored
links/styles/scripts keep their existing relative order and feature assets
follow them, so a page requesting no feature is byte-for-byte unchanged and
feature code can rely on its own declarations landing after the page's.

### Targets and failures

- `RenderTarget::Markdown` and `RenderTarget::MarkdownPlus` (and Terminal)
  **bypass** feature collection and resolution entirely — their output is
  byte-for-byte neutral, and a resolver returns `Ok(None)` for them.
- On the Browser target, `Ok(None)` means a feature *intentionally* has no
  assets; a requested-but-unresolved Browser feature is a hard
  `FeatureResolveError::UnresolvedFeature` (naming the feature and target).
  Silently dropping a browser dependency is forbidden — an unowned feature
  fails the render rather than emitting an inert element.
- A **body-only** render (assets injected before the body, no document `<head>`)
  cannot host `<link>` dependencies; a feature that resolves to links there
  fails with `FeatureResolveError::HeadRequired`. V1's inline-only Mermaid and
  Popover assets never hit this.
- Both variants surface through `RenderError::FeatureResolution` at the fallible
  document entry points, so `HtmlPage::render()` itself stays infallible.

### Deduplication and divergent configuration (fieldless v1)

Deduplication identity is the `PageFeature` variant, preserved in first-seen
order. V1 features are fieldless (`PageFeature` is a `Copy` enum) and all
per-page configuration lives in the resolver/context pair, so two requests for
the same feature cannot diverge — a feature's assets are injected at most once.

The spec's rule that *divergent configuration for one feature on one page is a
hard error* is therefore **forward-looking**: it binds the first future feature
that gains per-request configuration. Activating it requires evolving the
identity to a comparable request type (for example `FeatureRequest { feature,
config }`) and failing the render on unequal configs. Because a fieldless enum
cannot represent divergent config, v1 deliberately ships no dead comparison
machinery.

## Layout and style on the tree

Block-level positioning (`Layout`: margins, alignment, max-width, wrapping) and
appearance (`Style`: color, emphasis, border, fill) are **target-agnostic
attributes carried on `NodeAttrs`**, not properties a component hand-codes per
target. A component declares them once; each renderer lowers them on its own
terms (CSS for Browser, cells and SGR for Terminal; Markdown ignores them). See
[`layout-and-style.md`](./layout-and-style.md) for the full model.

## Components and parsed Markdown coexist

Components and the fold are **separate producers that share one backend**. The
shared backend is the tree renderer, not a component-dispatch layer:

```text
Markdown source ─▶ pulldown-cmark ─▶ fold ─▶ RenderNode ─▶ tree renderer
Component        ─▶ TreeRenderable::render_tree   ─▶ RenderNode ─▶ tree renderer
```

So the fold never turns a parsed block quote into a `BlockQuote` *component*, and
the document renderer never instantiates a component per table or list. The
per-target component traits (`TerminalRenderable`, `MarkdownRenderable`,
`BrowserRenderable`) remain public convenience surfaces for component authors and
direct component consumers; the IR is the meeting point.

Most `biscuit-terminal` structural components project to the tree —
`BlockQuote`, `Compose`, `OrderedList`, `UnorderedList`, `Progress`, `Section`,
`StatusBlock`, `Table`, `TextBlock`, `Todo`, `TwoColumn`, plus `FileSystem` — as
do `Prose` and darkmatter's `YamlBlock`. A few components stay bespoke by design:
inherently visual ones (`TerminalImage`, `GraphExpression`) and simple
terminal-only helpers (`PadLeft`, `PadRight`, `InlineContent`, `HorizontalRule`,
`Status`) are out of scope for a structural tree. One component, `FileSystem`,
projects to the tree for Browser and Markdown but keeps a bespoke **terminal**
renderer, because its Nerd Font directory glyphs have no target-agnostic
equivalent yet. (`components.md` tracks each component's exact state.)

## Darkmatter's document pipeline

Darkmatter's public Markdown rendering runs on the tree. `Markdown::as_html`,
`Markdown::as_terminal`, and `DarkmatterPage::render` / `render_to_browser` all
build a **complete** `Document` — component policy, alpha-bearing `PaintColor`,
text layout, browser attributes, and HR defaults are baked onto the nodes during
construction by darkmatter's context-aware fold (`TreeBuildContext`) — and then
run **one target fold** over it. There is no post-fold decoration pass and no
output rewriting; the hand-written event-stream serializers darkmatter once used
have been removed.

A few responsibilities sit deliberately **outside** the fold:

- **The `DarkmatterPage` page frame** is the one documented exception to
  "everything is the tree." It is a slim **viewport-level assembler** that wraps
  the folded target output: terminal/page width, outer page margin/padding,
  full-page background, max-width centering, `PageBackground::Pronounced`
  code-theme contrast, and (for the browser) page-wrapper metadata and
  stylesheet assembly. The closeout audit signed this off as **Option A** — the
  frame carries **no** component policy, inspects **no** component node kinds,
  and mutates **no** component content; it operates on the already-folded output
  string / wrapper, never on the `RenderNode` tree. (See
  `renderable/features/_completed/2026-06-06-tree-closeout/traversal-inventory.md`.)
- **Frontmatter** is extracted by darkmatter and attached to the `Document`'s
  metadata above the fold — the fold does not re-parse YAML.
- **`style:` frontmatter** is a darkmatter policy layer that applies page and
  component settings (layout, color, HR defaults, stylesheet/meta/code-theme,
  hyperlink and image styling) to `DarkmatterPage` before rendering. It feeds the
  tree resolved policy rather than reinterpreting style keys inside the fold. See
  [`layout-and-style.md`](./layout-and-style.md).
- **The compose pipeline** (transclusion, interpolation, shell expansion, link
  normalization, conditional blocks) still transforms Markdown *source text*
  before the fold. Moving composition onto the tree is possible future work but
  needs its own design; source rewrites and minimal diffs have different
  requirements than rendering.
- **`as_ast`** (a `markdown`-crate MDAST export) remains an independent
  structural-export feature; it is not part of the render path.

## Performance characteristics

The tree is an *owned* representation, which is a real cost: strings are owned,
every node is allocated, the whole document is resident before rendering starts,
and rendering is at least two passes (parse/fold, then render). A streaming
serializer that writes one target string in a single pass is hard to beat for a
single render of a large document.

The tree earns that cost when one or more of these hold — which, in practice, is
most of the time:

- the same document renders to multiple targets,
- diagnostics, provenance, or structural inspection are needed,
- transformations want a stable document model,
- component-generated and Markdown-parsed content must share one renderer,
- testability and parity matter more than minimum allocations.

Design choices that keep the cost in check: `pulldown-cmark` stays the parse
frontend, and `RenderNode` stays owned and lifetime-free (no borrowed lifetimes
threaded through the tree) so the IR is easy to hold, pass around, and serialize.

## Extending the tree

New or experimental document features do not require a new `NodeKind` variant on
day one. The extension model has three tiers, in increasing order of commitment:

1. **`NodeAttrs` classes and namespaced `data`** — attach experimental,
   target-specific information to an existing node.
2. **The `Extended` node** — a target-agnostic extension identified by a `token`
   (for example `"mark"` or `"dim"`), carrying nested inline `children` and an
   optional scalar `payload`. Renderers dispatch on the token; a token a renderer
   does not recognize falls back to a neutral default that preserves the
   children, so an extension never silently erases content.
3. **A first-class `NodeKind` field or variant** — promote a feature here once it
   is load-bearing and stable, accepting the exhaustive-match cost across every
   renderer.

The guiding principle: keep target-specific lowering in the renderers, not in the
fold, and let MarkdownPlus and Browser preserve richer behavior when portable
Markdown cannot.

## Embedding a styled subtree in Markdown — `renderable::tree::embed`

A text-to-text Markdown pipeline (such as darkmatter's compose) cannot carry the
styling a `Style` expresses — color, dim, icon spans — because portable
CommonMark has no form for it. When a component's output must round-trip through
such a pipeline **losslessly**, embed its projected subtree instead of
serializing it to lossy Markdown:

- `encode_embedded_subtree(&node)` serializes the subtree into a Markdown-safe
  block: an HTML-comment marker carrying the hex-encoded subtree, a portable
  Markdown fallback, and a closing marker.
- A fold that recognizes the markers (`decode_embedded_open` / `is_embedded_close`)
  splices the **exact** decoded subtree back in and drops the fallback; a fold or
  consumer that does not recognize them simply renders the portable fallback.

Because the styling is carried structurally (not re-derived) and the component is
not re-run, the round-trip is both lossless and free of recomputation — no second
filesystem walk, no color-identity loss. This is the mechanism behind darkmatter's
`::file-links` directive, and it is reusable by any `TreeRenderable` component.

## See also

- [`components.md`](./components.md) — the component catalog and per-target
  support matrix.
- [`layout-and-style.md`](./layout-and-style.md) — the `Layout` and `Style`
  primitives that ride on the tree.
- `.claude/skills/renderable/tree.md` — the `renderable::tree` API.
- `.claude/skills/biscuit-terminal/render-tree.md` — terminal folding and layout
  application.
