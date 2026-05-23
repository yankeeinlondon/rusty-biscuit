# The Fold and Darkmatter Rendering

**Status:** design note, assuming renderable Stage 3 is complete.

This document describes how Darkmatter's Markdown compose and rendering
pipelines can move onto the render-tree architecture without losing the things
that already work well: `pulldown-cmark` as the parser, streaming performance
where it matters, and Darkmatter's ability to keep growing target-specific
rendering features.

The short version:

- Keep `pulldown-cmark` as the parser.
- Treat the fold as the Markdown-source-to-`Document` bridge.
- Let the tree renderers own Markdown, MarkdownPlus, Browser, and Terminal
  lowering once the fold has feature parity.
- Keep the existing event-stream renderers until parity and performance
  evidence justify switching public entry points.
- Do not expect `TerminalRenderable`, `MarkdownRenderable`, and
  `BrowserRenderable` components to be the main mechanism for rendering parsed
  Markdown. They will mostly coexist with the fold, while sharing the same
  `RenderNode` vocabulary and renderer backend.

## Current Pipeline Shape

Darkmatter has two related but different pipelines.

The compose pipeline prepares Markdown text. It evaluates frontmatter-driven
state, interpolates values, expands approved shell directives, resolves and
normalizes links, applies conditional page blocks, transcludes documents, and
then emits Markdown text. This pipeline is largely a source transformation
pipeline. It currently works before rendering and should keep doing so for the
public CLI path until a tree-native transformation story is proven.

The rendering pipeline takes Markdown text and emits a target format. Today,
the public `Markdown::as_html` and `for_terminal` paths are hand-written
`pulldown-cmark` event-to-string serializers. They parse, wrap the parser with
Darkmatter processors such as `InlineStyleProcessor` and `RuleProcessor`, and
stream target output directly.

The fold is the new middle layer:

```text
Markdown source
    |
    v
pulldown-cmark Event stream
    |
    v
darkmatter fold
    |
    v
renderable::tree::Document
    |
    +--> Markdown renderer
    +--> MarkdownPlus renderer
    +--> Browser renderer
    +--> Terminal renderer
```

The fold does not replace `pulldown-cmark`; it depends on it. Its job is to
turn the parser's event stream into the canonical owned tree that the
`renderable` and `biscuit-terminal` renderers can walk.

## What Works Well

### `pulldown-cmark` stays the right parser

The existing parser choice remains sound. `pulldown-cmark` is fast, well
tested, and event-oriented. That event model is a good fit for both the legacy
streaming renderers and the tree fold.

The important architectural point is that the render tree does not require
replacing `pulldown-cmark` with an AST parser. The tree is our owned
intermediate representation, not the parser's. That keeps Darkmatter aligned
with the crate it already relies on while still giving us a stable internal
shape for rendering and later transforms.

### The tree vocabulary has been exercised from both directions

The render tree now has two proven producers:

- parsed Markdown, via `fold_markdown_to_document`
- component projections, via `TreeRenderable`

Assuming Stage 3 is complete, the twelve migrated `biscuit-terminal`
components have structural projections all the way through nested component
containers. That matters for Darkmatter because it proves the tree is not only
a Markdown AST clone. It can also represent generated and component-like
document structure.

The renderers have also been forced to deal with real structure across
Markdown, Browser, and Terminal output: tables, lists, block quotes, sections,
task state, progress hints, columns hints, style, layout, and terminal-only
escape hatches. That is the right evidence base for using the same renderer
backend under Darkmatter.

### The fold creates a single renderable document

The current public renderers parse once per target. The fold creates a
`Document` that can be rendered more than once:

```text
parse + fold once -> render terminal
                  -> render HTML
                  -> render MarkdownPlus
                  -> inspect / test / transform
```

That is valuable for CLI modes that emit multiple artifacts, for diagnostics,
for test fixtures, and for future LSP or static-analysis work. The owned tree
also makes serialization and snapshot testing straightforward.

### Strictness and diagnostics are a better fit than silent degradation

The tree renderers have a shared strictness model: strict, warn, and lossy.
That is a cleaner contract than ad hoc target-specific behavior inside each
streaming serializer.

This matters because Darkmatter has features that cannot lower equally to every
target. Terminal can express ANSI styling and image protocols; Browser can
express CSS and richer interactive structures; portable Markdown cannot.
Strictness gives those differences a first-class place to live.

### Component migration made the renderer backend stronger

Stage 3's component work should be treated as directly relevant, even though
parsed Markdown will not normally call component `render()` methods. The
component migration forced missing renderer capabilities into the shared tree
layer: layout, style, table metadata, Markdown-safe table cells, progress
hints, task hints, list marker policy, columns hints, and structural projection
rules.

That improves Darkmatter's eventual fold path because the same terminal,
browser, and Markdown tree renderers will be used for parsed documents.

## Challenges That Remain

### Custom Darkmatter processors still need span-aware integration

The largest feature gap is still Darkmatter's custom event processors:

- `InlineStyleProcessor` for mark and dim inline styles
- `RuleProcessor` for horizontal rules with attribute blocks

The current fold uses `Parser::new_ext(input, options).into_offset_iter()` so
it can attach source byte ranges to nodes. The existing processors wrap
ordinary event iterators and synthesize or replace events without preserving
offsets. Feeding them into the fold would lose the provenance that makes the
tree useful for diagnostics and future transforms.

The preferred path is to make these processors span-aware. They should be able
to consume and emit `(EventLike, Range<usize>)`, with a documented policy for
synthetic ranges when a text event splits into several inline nodes. This is
more invasive than duplicating the grammar inside the fold, but it avoids two
implementations of Darkmatter's custom inline syntax.

Until that lands, the tree path cannot claim full parity with `as_html` or
`for_terminal`.

### Parser options must become a single deliberate contract

The fold currently wants a richer option set than the legacy renderers. The
legacy public rendering path uses a narrower `pulldown-cmark` configuration,
while the fold can model constructs such as task lists, footnotes, superscript,
and subscript.

That divergence is manageable while the fold is experimental, but it is risky
at cutover. The same input should not parse differently depending on which
renderer path a caller selected unless that difference is deliberate and
documented.

The right migration target is a shared parse-options policy:

- one central function for the options used by public rendering
- fixtures that classify every widened option as accepted behavior or deferred
- no silent widening of the public parser surface as a side effect of adopting
  the fold

This keeps `pulldown-cmark` central while making option changes reviewable.

### Frontmatter needs to be attached above the fold

Darkmatter already owns frontmatter extraction. The fold should not duplicate
that parser. Instead, the render-tree entry point for Darkmatter should accept
the already extracted metadata and attach it to `DocumentMetadata`.

This keeps responsibilities clean:

- Darkmatter extracts and composes document state.
- The fold converts Markdown body events to tree nodes.
- The final `Document` carries both body structure and metadata.

### The compose pipeline is not automatically tree-native

The compose pipeline currently transforms source text before rendering. Moving
rendering onto the tree does not automatically move composition onto the tree.

Some compose operations naturally remain source-text operations for now:

- interpolation
- shell expansion
- frontmatter state evaluation
- transclusion that emits Markdown
- cleanup and normalization rules that intentionally preserve editable
  Markdown

A future tree-native compose pipeline is possible, especially for operations
that benefit from structural spans, but it needs its own design. The fold's
container spans should be treated as diagnostics-grade until tests prove they
are suitable for source rewrites and minimal diffs.

### Some rendering features are still wider than the tree

Darkmatter has planned or existing target features that are not all represented
as first-class tree constructs yet: disclosure blocks, popovers, smart images,
YouTube embeds, audio content, product/place/person cards, richer table
behavior, and other document components.

That is not a reason to avoid the tree. It is a reason to keep the tree
extension model ergonomic:

- use `NodeAttrs` classes and namespaced data for experimental features
- promote repeated, load-bearing data into typed hints or `NodeKind` fields
- keep target-specific lowering in renderers, not in the fold
- allow MarkdownPlus and Browser to preserve richer behavior when portable
  Markdown cannot

This is the balance point: performance matters, but the architecture still
needs room to absorb features that have not landed yet.

## Performance Considerations

The legacy renderers are streaming. They parse with `pulldown-cmark`, walk the
event stream, and write one target string. That is hard to beat for a single
render of a large document.

The fold builds an owned tree first. That adds costs:

- every relevant string becomes owned
- every node is allocated
- the whole document is resident before rendering starts
- rendering becomes at least two passes: parse/fold, then render

Those costs are real and should not be hand-waved away. The tree path earns
its keep when one or more of these are true:

- the same document is rendered to multiple targets
- diagnostics, provenance, or structural inspection are needed
- transformations need a stable document model
- component-generated and Markdown-parsed content need to share one renderer
- testability and parity checks are more valuable than minimum allocation count

For single-target hot paths, especially terminal-only rendering of large
documents, the old streaming path may remain faster for some time. A cutover
should be benchmark-gated, not assumed.

Pragmatic performance policy:

1. Keep `pulldown-cmark` as the parse frontend.
2. Keep the fold owned and lifetime-free; do not infect `RenderNode` with
   borrowed lifetimes unless benchmarks force a major redesign.
3. Add benchmarks that compare:
   - legacy `for_terminal`
   - fold plus terminal renderer
   - legacy `as_html`
   - fold plus browser renderer
   - fold once plus multiple target renders
4. Use the fold by default only where parity and performance are acceptable.
5. Keep a streaming escape hatch if a documented workload needs it.

The likely steady state is not "tree everywhere at any cost." It is "tree as
the default architecture once proven, with targeted streaming paths retained
where they are measurably better and semantically complete."

## How Components Fit Into Darkmatter Rendering

The components that implement `TerminalRenderable`, `MarkdownRenderable`, and
`BrowserRenderable` should not be thought of as the engine that renders parsed
Markdown documents.

Parsed Markdown should flow like this:

```text
Markdown source -> pulldown-cmark -> fold -> RenderNode tree -> tree renderer
```

Component rendering flows like this:

```text
Component -> TreeRenderable::render_tree -> RenderNode tree -> tree renderer
```

Those paths converge at `RenderNode`, not at `TerminalRenderable::render`,
`MarkdownRenderable::render_markdown`, or
`BrowserRenderable::render_html_fragment`.

That distinction is important:

- The fold should not convert a Markdown block quote into a `BlockQuote`
  component and call its render methods.
- The terminal document renderer should not render parsed Markdown by
  constructing component objects for every table, list, or section.
- The per-target component traits remain public convenience surfaces for
  component authors and direct component consumers.
- The shared renderer backend is the tree renderer, not a component dispatch
  layer.

There are still places where components are useful to Darkmatter:

- Generated Darkmatter features may be implemented as tree-producing
  components and spliced into a document as `RenderNode`s.
- Error rendering can continue using rich terminal components where the output
  is component-first rather than parsed Markdown-first.
- A future extension syntax might choose to instantiate a component during
  composition, then insert that component's `TreeRenderable` projection into
  the folded document.
- The component migrations keep pressure on the tree renderers to support real
  target behavior, which benefits parsed Markdown too.

So the answer is: components mostly coexist as a separate producer path, but
they share the same IR and renderer backend. Darkmatter's rendering pipeline
should leverage the capabilities made available by component work, not route
ordinary Markdown rendering through component trait objects.

## Recommended Migration Path

### 1. Keep public rendering on the current paths while closing fold parity

Do not flip `Markdown::as_html` or `for_terminal` until the fold handles
Darkmatter-specific processors, frontmatter metadata, parser option policy, and
the known feature gaps. The current public paths are mature and streaming.

### 2. Add explicit tree-rendering entry points

Expose experimental or internal functions that make the intended path easy to
exercise:

```text
Markdown -> Document
Markdown -> Document -> Markdown
Markdown -> Document -> MarkdownPlus
Markdown -> Document -> Browser HTML
Markdown -> Document -> Terminal
```

These should be easy to benchmark and parity-test without changing stable
behavior.

### 3. Make parser configuration shared and reviewed

Move toward one render parse-options policy. If the fold needs richer options,
add fixtures and document each behavior change before the public renderer uses
it.

### 4. Make Darkmatter processors span-aware

This is the key unlock for custom inline styles and horizontal-rule
attributes. Avoid a second grammar inside the fold unless the span-aware
processor approach proves too complex.

### 5. Benchmark before cutover

Use representative corpora:

- small docs rendered once
- large docs rendered once
- transcluded/generated docs
- docs rendered to multiple targets
- docs with tables, code blocks, images, links, and custom inline styles

The cutover decision should be made from these numbers plus parity fixtures.

### 6. Flip one target at a time

The browser path is likely the easiest first public target because HTML already
benefits from a structural tree and can preserve rich hints. Terminal has the
most performance and capability sensitivity. Portable Markdown has the strictest
lossiness constraints. Treat them separately.

## Summary

The fold is still the right bridge from Darkmatter's Markdown source world to
the renderable tree world. It works well where `pulldown-cmark` events map
directly to `NodeKind`s, and Stage 3 component work strengthens the shared
renderers it will eventually use.

The remaining work is not about replacing `pulldown-cmark`; it is about making
Darkmatter's parser options, custom processors, metadata, provenance, and
feature extensions line up with the tree without giving up the performance
benefits of the existing streaming pipeline too early.

The safest architecture is a converged IR with multiple producers:

- Darkmatter folded Markdown documents
- renderable components
- generated/transcluded document fragments

and shared target renderers underneath them. That gives the project a path to
more consistent rendering without forcing every feature through one premature
abstraction.
