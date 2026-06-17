---
last_updated: "2026-05-17"
component: GraphExpression
module: biscuit-terminal/lib/src/components/graph_expression.rs
status: research
scope: tree-rendering-migration
---

# Challenges of Migrating the `GraphExpression` Component to the Tree Rendering Architecture

## Functional and Design Goals

### Why GraphExpression was created

The `GraphExpression` component was created to give CLI tools in the rusty-biscuit monorepo a way to render graph diagrams (directed and undirected) inline in the terminal. Unlike textual components (lists, tables, headings) whose output is character-by-character terminal text, graph diagrams require a multi-stage visual pipeline: parse the graph definition, lay out nodes and edges, render to SVG, rasterize to PNG, and finally display the PNG inline using the terminal's image protocol (Kitty or iTerm2).

The design goals that shaped `GraphExpression`:

1. **Composable graph rendering** -- act as a first-class `TerminalRenderable` component so it composes with `Section`, `Compose`, lists, and every other container in biscuit-terminal.
2. **Terminal-aware output** -- automatically detect dark/light mode, adjust the color theme, compute pixel-perfect resolution from the terminal's cell size, and choose the correct image protocol.
3. **Graceful fallback** -- when a terminal lacks image support, degrade to a fenced code block containing the original graph source (`graph-expression` or `dot` info string).
4. **Dual-target rendering** -- support both terminal (`TerminalRenderable`) and browser/HTML (`BrowserRenderable`) output from the same struct, routing SVG to the browser path and PNG+image-protocol to the terminal path.
5. **Cached artifact generation** -- delegate to `biscuit-visualized`'s content-addressed cache so repeated renders of the same graph definition hit the cache rather than re-rasterizing.

### Where it is used today

`GraphExpression` surfaces in three places:

| Consumer | Crate | Use case |
|----------|-------|----------|
| `bt graph-expression` CLI command | `biscuit-terminal/cli` | User-facing CLI: parses expression or DOT input, renders the graph inline, emits metadata with `--meta`, supports `--inverse`, `--orientation`, `--width`, `--font`. |
| `sniff repo package-dependencies --ui` | `sniff/cli` | Builds a DOT digraph from workspace dependency data and renders the dependency diagram inline in the terminal. Also has an SVG export path (`render_repo_deps_svg`) for non-terminal consumers. |
| Library composition | `biscuit-terminal/lib` | Any code that builds a `GraphExpression` and composes it into `Section`, `Compose`, or other `TerminalRenderable` containers. |

### Example usage

**CLI (expression syntax):**

```bash
bt graph-expression "a -> b -> c"
bt graph-expression --syntax dot "digraph { A -> B; B -> C; }"
bt graph-expression --orientation left-to-right --title "My Graph" "Start -> End"
bt graph-expression --inverse "x -> y"
```

**Library (programmatic):**

```rust
use biscuit_terminal::components::graph_expression::{
    GraphExpression, GraphInputSyntax, GraphOrientation,
};
use biscuit_terminal::components::renderable::TerminalRenderable;
use biscuit_terminal::utils::layout::{Length, TargetValue};

let graph = GraphExpression::for_terminal("a -> b -> c", GraphInputSyntax::Auto)?
    .with_orientation(GraphOrientation::LeftToRight)
    .with_title("Example graph")
    .left_margin(TargetValue::universal(Length::ch(4)));

let term = biscuit_terminal::terminal::Terminal::new();
println!("{}", graph.display(&term));
```

**Library (sniff repo package-dependencies):**

```rust
let graph = GraphExpression::for_terminal(&dot, GraphInputSyntax::Dot)?
    .with_width(width)
    .with_orientation(orientation);
let term = Terminal::default();
graph.render(&term)
```

## Technical Implementation (current)

### Code structure

`GraphExpression` lives at `biscuit-terminal/lib/src/components/graph_expression.rs` (~483 lines of implementation + ~125 lines of tests). It is a thin adapter that wraps `biscuit_visualized::graph::GraphDiagram` and adds terminal-specific behavior.

```text
GraphExpression
├── diagram: GraphDiagram          ← from biscuit-visualized (owns the parsed graph + render logic)
├── scale: u32                     ← rasterization multiplier (default: 2)
├── transparent_background: bool   ← alpha channel control
├── width: ImageWidth              ← display width spec (Percent, Fill, Cells)
└── layout: Layout                 ← margins, alignment, word-wrap
```

### Key responsibilities and transforms

The component is responsible for the following transforms/mutations during a render:

1. **Input parsing and validation** -- Delegates to `GraphDiagram::parse(source, syntax)`. Supports three input modes (`Auto`, `Expression`, `Dot`). Expression syntax rejects mixed `->` / `--` edge kinds. Returns `GraphRenderError` on failure.

2. **Color theme detection and application** -- `for_terminal()` reads the terminal's `ColorMode` (Dark / Light / Unknown) and selects `GraphColorTheme::dark()` or `GraphColorTheme::light()`. `inverted_for_terminal()` selects the opposite theme and forces an opaque surface.

3. **Resolution calculation** -- During `render_to_image()`, resolves the display width in terminal cells via `TerminalImage::resolve_dimensions_for`, then translates cells to pixels using the terminal's detected cell pixel width (`term.cell_size()`). The resulting `target_width_px` is passed to the rasterizer so the PNG is rendered at exactly the display resolution.

4. **SVG rendering and rasterization** -- Delegates to `biscuit-visualized::GraphDiagram::render(RenderRequest)`. Supports both SVG-only output (browser path) and PNG output at a target pixel width (terminal path). SVG post-processing trims padding, applies fonts, and sets colors.

5. **Content-addressed caching** -- The render request includes the full configuration (source, theme, scale, target_width, transparent_background). `biscuit-visualized` hashes these into a cache key and returns `cache_hit: bool`.

6. **Image protocol dispatch** -- Wraps the cached PNG path in `TerminalImage`, which handles Kitty and iTerm2 protocol selection, cursor save/restore, and image placement.

7. **Fallback rendering** -- When `TerminalImage::render()` returns empty output (no image support), falls back to `diagram.fallback_code_block()` which produces a fenced code block (` ```graph-expression ` or ` ```dot `).

8. **Layout application** -- Applies block layout (margins, alignment) via `self.layout.apply_block_layout()` to the final string output (whether image escape sequence or code block fallback).

9. **Browser rendering** -- `BrowserRenderable::render_html_fragment()` renders the graph to SVG via `biscuit-visualized`, wraps it in a `BrowserFragment::RawHtml` node for verbatim HTML passthrough. On failure, emits an HTML comment + `<pre><code class="language-dot">` fallback.

### Render pipeline (terminal)

```text
Source string
    │
    ▼
GraphDiagram::parse(source, syntax)      ← biscuit-visualized
    │
    ▼
resolve display width (cells → px)       ← TerminalImage + cell_size
    │
    ▼
GraphDiagram::render(RenderRequest)      ← SVG → PNG via resvg (cached)
    │
    ▼
TerminalImage::new(png_path)             ← validate + load
    │
    ▼
TerminalImage::render(term)              ← Kitty/iTerm2 protocol
    │
    ▼ [empty output]
fallback_code_block()                    ← fenced code block
    │
    ▼
layout.apply_block_layout(output, w)     ← margins + alignment
    │
    ▼
Final terminal string
```

### Render pipeline (browser)

```text
Source string
    │
    ▼
GraphDiagram::parse(source, syntax)
    │
    ▼
GraphDiagram::render(RenderRequest { Svg })
    │
    ▼
read SVG file → raw SVG string
    │
    ▼
BrowserFragment::RawHtml(svg_string)     ← verbatim passthrough
```

## Implementation Challenges

The tree rendering architecture (`renderable::tree`) is designed around a canonical `RenderNode` / `NodeKind` model where content producers emit a document-structural tree and renderers walk it to produce target output. `GraphExpression` is explicitly called out in `tree-rendering.md` as an "inherently visual" component that should remain on bespoke renderers. This section explores *why* that classification exists and what would have to change if we wanted to route it through the tree.

### Challenge: No Natural NodeKind Representation

#### No Natural NodeKind Representation

`NodeKind` has 25 variants covering document-structural concepts: `Root`, `Heading`, `Paragraph`, `BlockQuote`, `List`, `Code`, `Table`, `Image`, etc. A graph diagram does not map cleanly to any of these. The closest candidates:

- `NodeKind::Image { url, title, alt }` -- represents a reference image with a URL, but `GraphExpression` produces a *locally rasterized PNG* with no stable URL. The image does not exist until the render pipeline runs.
- `NodeKind::Code { lang, meta, value }` -- could carry the graph source as a code block, but this only represents the *fallback*, not the visual output.
- `NodeKind::Html { value, block }` -- could carry the SVG, but this is browser-specific and would require terminal renderers to understand embedded SVG.

None of these is a faithful representation. A graph diagram is simultaneously:
1. A source definition (expression or DOT)
2. A rasterized image (terminal path)
3. A vector image (browser path)
4. A text fallback (when images are unavailable)

Picking one `NodeKind` collapses this multi-modal nature into a single dimension.

**Example of how this presents itself:** If `GraphExpression` projects to `NodeKind::Image`, the Markdown renderer would try to emit `![alt](url)` -- but there is no URL, and the PNG only exists as a temporary file. If it projects to `NodeKind::Code`, the terminal tree renderer would output a code block on *every* terminal, including ones with full image support -- defeating the purpose of the component.

```rust
#[test]
fn graph_expression_tree_projection_chooses_correct_node_kind() {
    let graph = GraphExpression::parse("a -> b -> c", GraphInputSyntax::Auto).unwrap();
    let node = graph.render_tree_node();

    // What kind should this be?
    // - Image? But no URL exists at tree-construction time.
    // - Code? But that loses the visual output on capable terminals.
    // - A new variant? But adding NodeKind variants breaks every renderer.
    assert!(node.is_some(), "GraphExpression should project to some tree node");
}
```

### Challenge: Lazy vs Eager Artifact Generation

#### Lazy vs Eager Artifact Generation

The tree rendering model assumes that `render_tree()` produces a `RenderNode` *synchronously and cheaply* -- it builds a structural description, not the final rendered output. The expensive work (rendering, rasterization, cache I/O) is deferred to the target-specific renderer.

`GraphExpression` violates this assumption because the *identity* of its output depends on the render target:

- **Terminal:** requires a PNG at `target_width_px` (which depends on `Terminal::width()` and `cell_size()`).
- **Browser:** requires the SVG source string.
- **Markdown:** would require either the fenced code block or an image reference.

The `TreeRenderable::render_tree(&self)` signature takes `&self` only -- it has no access to `Terminal`, `TerminalRenderOptions`, or any target context. The tree node must be produced *before* the renderer walks it. But `GraphExpression` cannot decide what artifact to produce until it knows the target.

**Example of how this presents itself:** `sniff repo package-dependencies --ui` builds a `GraphExpression` and calls `graph.render(&term)`. The terminal width determines the PNG pixel dimensions, which changes the cache key, which determines whether a cached PNG is reused or a new one is rendered. This entire decision chain is triggered during `render()`, not during tree construction.

```rust
#[test]
fn graph_expression_cannot_resolve_width_at_tree_construction_time() {
    let graph = GraphExpression::parse("a -> b -> c", GraphInputSyntax::Auto).unwrap();

    // render_tree() has no Terminal parameter -- cannot resolve cell pixel width
    let node = graph.render_tree_node();

    // The node cannot carry the rasterized PNG path because the PNG
    // has not been rendered yet. The PNG dimensions depend on terminal
    // context that is unavailable here.
    assert!(node.is_some());
}
```

### Challenge: Multi-Target Output Divergence

#### Multi-Target Output Divergence

The three render targets produce *fundamentally different output types*:

| Target | Output type | Size | Format |
|--------|-------------|------|--------|
| Terminal | Kitty/iTerm2 escape sequences + cached PNG | ~10-50 KB | Binary protocol |
| Browser | Inline SVG element | ~2-10 KB | XML markup |
| Markdown | Fenced code block (`graph-expression` / `dot`) | ~100 bytes | Plain text |

The tree model expects a single `RenderNode` that all three renderers walk. But the terminal renderer would need to trigger PNG rasterization and image protocol dispatch, the browser renderer would need to read the SVG file, and the Markdown renderer would need the original source string. Each requires different side effects and different data from `biscuit-visualized`.

Today `GraphExpression` handles this divergence through two separate trait implementations (`TerminalRenderable` + `BrowserRenderable`), each with its own render pipeline. The tree model would have to unify these into a single node that somehow encodes all three possibilities.

**Example of how this presents itself:** The browser path calls `diagram.render(RenderRequest { format: Svg })` and reads the SVG file. The terminal path calls `diagram.render(RenderRequest { format: Png, target_width: Some(px) })` and wraps the PNG in `TerminalImage`. These are completely different side effects. A `render_tree()` call cannot produce both artifacts at once without always paying the cost of both.

```rust
#[test]
fn graph_expression_tree_node_carries_both_svg_and_png_context() {
    let graph = GraphExpression::parse("a -> b -> c", GraphInputSyntax::Auto).unwrap();
    let node = graph.render_tree_node().unwrap();

    // The terminal renderer needs PNG info.
    // The browser renderer needs SVG info.
    // The Markdown renderer needs the source text.
    // A single RenderNode cannot carry all three without eager rendering.
    // And if it carries none, each renderer must independently trigger
    // biscuit-visualized, duplicating work and breaking the "build one tree"
    // contract.
    assert!(false, "This test documents an unsolved problem");
}
```

### Challenge: Terminal-Side Effects in a Pure Tree

#### Terminal-Side Effects in a Pure Tree

The tree rendering contract is: produce a `RenderNode`, then walk it to produce output. This is a pure transformation -- the tree itself should have no side effects.

But `GraphExpression`'s terminal render path has significant side effects:

1. **File system writes** -- `biscuit-visualized` writes a PNG to `$TMPDIR/biscuit-visualized/v1/graph/png/`.
2. **External process coupling** -- The rasterization depends on system font availability (`resvg` loads the system font database).
3. **Terminal state mutation** -- `TerminalImage` emits cursor save/restore escape sequences (`\x1b[s`, `\x1b[u`), which interact with the terminal's cursor position.
4. **Cache interactions** -- The render may or may not hit the cache, affecting latency and output behavior.

If the tree renderer encounters a `GraphExpression`-derived node during its walk, it would need to trigger all of these side effects mid-render. This breaks the "tree is a pure data structure" contract and makes the renderer non-deterministic (output depends on cache state).

**Example of how this presents itself:** Two consecutive `render_terminal_node()` calls on the same tree produce different timing characteristics (first call is a cache miss, second is a hit). The output strings are identical, but the side-effect profile is different. This matters for testing and for the strictness model (a cache miss is not an error, but a file-system write failure would be).

```rust
#[test]
fn graph_expression_render_is_idempotent_across_cache_states() {
    let graph = GraphExpression::parse("a -> b -> c", GraphInputSyntax::Auto).unwrap();
    let term = Terminal::new_optimistic(80);

    // First render -- may or may not be a cache hit
    let result1 = graph.render(&term);

    // Second render -- should produce identical output
    let result2 = graph.render(&term);

    assert_eq!(result1, result2,
        "Rendering the same graph twice must produce identical output");
}
```

### Challenge: Fallible Rendering in an Infallible Tree Walk

#### Fallible Rendering in an Infallible Tree Walk

The `TreeComponent<T>` adapter bridges `TreeRenderable` to `TerminalRenderable` by calling `render_tree()` then `render_terminal_node()`. The adapter is infallible on the `TerminalRenderable` side -- `render(&self, term: &Terminal) -> String`.

But `GraphExpression::try_render()` can fail in multiple ways:

| Error | Cause |
|-------|-------|
| `GraphRenderError::Visualization` | `biscuit-visualized` render or rasterization failure |
| `GraphRenderError::NoImageSupport` | Terminal lacks Kitty/iTerm2 |
| `GraphRenderError::DisplayError` | PNG file cannot be loaded as a `TerminalImage` |

The current `TerminalRenderable` impl handles these by falling back silently: `render_raw()` catches any error and returns `fallback_code_block()`. This is lossy but infallible.

The tree rendering strictness model (`Strict` / `Warn` / `Lossy`) has a clear policy for `Unsupported` nodes, but does not have a policy for "this node's primary render path failed, here's a degraded fallback." A `GraphExpression` node would need a way to communicate partial success (image rendered) vs. degradation (code block fallback) vs. total failure (even the code block is wrong).

**Example of how this presents itself:** Under `RenderStrictness::Strict`, should a graph that falls back to a code block be treated as an error? The user explicitly asked for a visual diagram; getting a code block is a significant degradation. But under `Lossy`, silently falling back is exactly the desired behavior.

```rust
#[test]
fn graph_expression_strict_mode_rejects_fallback_in_strict() {
    let graph = GraphExpression::parse("a -> b -> c", GraphInputSyntax::Auto).unwrap();
    let term = Terminal::new_optimistic(80);

    // Simulate a terminal with no image support
    let opts = TerminalRenderOptions::new(&term, RenderStrictness::Strict);

    // If GraphExpression projects to the tree and the terminal has no image support,
    // the tree renderer under Strict should report a diagnostic or error,
    // not silently emit a code block.
    let result = render_terminal_node(&graph.render_tree_node().unwrap(), &opts);

    assert!(result.is_err() || result.unwrap().diagnostics.iter().any(|d| d.severity == Severity::Warning),
        "Strict mode should surface the fallback as a problem, not silently degrade");
}
```

### Challenge: Layout Interaction with Image Dimensions

#### Layout Interaction with Image Dimensions

`GraphExpression` applies layout *after* rendering the image. The `layout.apply_block_layout()` call pads and aligns the final string (which contains image escape sequences). This works because the image escape sequences are treated as opaque blocks -- layout only adds whitespace around them.

In the tree model, layout is applied via `NodeAttrs::layout()` on the node, and the terminal tree renderer resolves margins and alignment during its walk. But the tree renderer's layout system works in *cell columns*. A `GraphExpression`'s image width is expressed as `ImageWidth::Percent(0.5)` or `ImageWidth::Cells(40)`, which resolves to a pixel width for rasterization. The margin and alignment calculations happen at a different level than the image dimension calculations.

If the tree renderer handles layout, it needs to:
1. Know the image's cell-column dimensions *before* rasterizing (to compute available width for margins).
2. Pass the resolved cell width to the rasterizer.
3. Then apply the layout to the rendered image output.

This creates a chicken-and-egg problem: layout needs the image dimensions, but image dimensions depend on the available width after layout is applied.

**Example of how this presents itself:** A `GraphExpression` with `ImageWidth::Percent(0.5)`, `left_margin: ch(4)`, and `right_margin: ch(4)` on an 80-column terminal. The available width is `80 - 4 - 4 = 72` columns. The image should be `72 * 0.5 = 36` columns wide. But the tree renderer resolves margins *for the node*, not for the image inside it. The percent-width calculation needs to know the post-margin available width, which the tree renderer computes but does not expose to the node's render logic.

```rust
#[test]
fn graph_expression_image_width_respects_margin_narrowing() {
    let graph = GraphExpression::parse("a -> b -> c", GraphInputSyntax::Auto)
        .unwrap()
        .with_width(ImageWidth::Percent(0.5))
        .left_margin(TargetValue::universal(Length::ch(4)))
        .right_margin(TargetValue::universal(Length::ch(4)));

    let term = Terminal::new_optimistic(80);

    // On an 80-col terminal with 4+4 margin, available = 72 cols.
    // 50% of 72 = 36 cols image width.
    // The PNG should be rasterized at 36 * cell_pixel_width pixels.
    let result = graph.try_render(&term);
    if let Ok(r) = result {
        let output = r.output;
        // The image escape sequences should reflect a 36-column-wide image,
        // not a 40-column one (50% of 80).
        assert!(!output.is_empty());
    }
}
```

### Challenge: Inverted Theme State Across Targets

#### Inverted Theme State Across Targets

`GraphExpression` supports an `inverted()` builder method that flips the color theme relative to the terminal's detected color mode and forces an opaque background. This state affects both:
1. The `GraphColorTheme` passed to `biscuit-visualized` (changes node/edge/surface colors in the SVG/PNG).
2. The `transparent_background` flag (inverted = opaque, normal = transparent).

In the tree model, this theme state would need to be captured in the `RenderNode` and respected by all three renderers. But:
- The terminal renderer would need to know the theme to choose the right PNG.
- The browser renderer would need the theme for SVG post-processing.
- The Markdown renderer would ignore theme entirely (it just emits the source).

Currently, theme is baked into the `GraphDiagram` at construction time (`for_terminal_mode`). The tree node would need to either carry the `GraphDiagram` itself (defeating the purpose of the tree abstraction) or carry enough metadata for each renderer to reconstruct the theme.

**Example of how this presents itself:** `bt graph-expression --inverse "a -> b"` on a dark terminal produces a graph with a light color theme and opaque background. The tree node would need to encode "this graph should be rendered with the opposite of the detected color mode" -- but the detected color mode is a runtime property, not a tree property.

```rust
#[test]
fn graph_expression_inverted_mode_survives_tree_roundtrip() {
    let graph = GraphExpression::parse("a -> b", GraphInputSyntax::Auto)
        .unwrap()
        .inverted(true);

    // If this were projected to a tree node and then rendered by the
    // terminal tree renderer, the renderer would need to know:
    // 1. This graph is inverted (opaque background, opposite theme).
    // 2. The terminal's detected color mode to pick the right theme.
    // The tree node would need to carry inversion intent, and the renderer
    // would need to resolve it at walk time.
    assert!(!graph.transparent_background);
}
```

### Challenge: Cache Metadata Propagation

#### Cache Metadata Propagation

`GraphExpression::try_render()` returns a `GraphRenderResult` struct that carries not just the output string, but also:

- `png_path: PathBuf` -- the path to the cached PNG file.
- `cache_hit: bool` -- whether the PNG was served from cache.

This metadata is consumed by the CLI's `--meta` flag, which prints render timing, file size, and cache status. In the tree model, `render_terminal_node()` returns `Result<Rendered<String>, RenderError>`, where `Rendered<T>` bundles the output string with `Diagnostic`s. There is no mechanism to carry arbitrary component-specific metadata (like `png_path` or `cache_hit`) through the tree render pipeline.

**Example of how this presents itself:** `bt graph-expression --meta "a -> b"` needs to print the PNG path and cache status after rendering. If the graph were rendered through the tree, this metadata would be lost because the tree renderer only returns a string + diagnostics.

```rust
#[test]
fn graph_expression_meta_data_is_accessible_after_tree_render() {
    let graph = GraphExpression::parse("a -> b -> c", GraphInputSyntax::Auto).unwrap();
    let term = Terminal::new_optimistic(80);

    // The current API:
    let result = graph.try_render(&term);
    if let Ok(r) = result {
        // These fields are lost if rendering goes through the tree:
        assert!(!r.png_path.as_os_str().is_empty());
        // cache_hit may be true or false, but the field exists.
        let _ = r.cache_hit;
    }

    // Through the tree, render_terminal_node() returns Rendered<String>,
    // which has no png_path or cache_hit fields.
}
```

## Solution Suggestions

### Solution: Extension Data on RenderNode via NodeAttrs

#### Extension Data on RenderNode via NodeAttrs

**Description:** `NodeAttrs` already has a `data: BTreeMap<HintNamespace, BTreeMap<String, String>>` field for namespaced extension data. A `GraphExpression` could project to a `NodeKind::Image` (or a custom extension node) and carry its configuration as namespaced hints:

```text
renderable.graph.source: "a -> b -> c"
renderable.graph.syntax: "expression"
renderable.graph.orientation: "left-to-right"
renderable.graph.inverted: "true"
renderable.graph.scale: "2"
```

Each renderer would read these hints at walk time and perform the appropriate render. The terminal renderer would parse the graph, rasterize the PNG, and emit image protocol escapes. The browser renderer would render the SVG. The Markdown renderer would emit the fenced code block.

**Which challenges this helps with:**
- *No Natural NodeKind Representation* -- treats the graph as an `Image` with extension data, avoiding a new `NodeKind` variant.
- *Lazy vs Eager Artifact Generation* -- defers the expensive work to the renderer's walk, keeping `render_tree()` cheap.
- *Inverted Theme State Across Targets* -- the inversion flag lives in the hints, and each renderer resolves the actual theme at walk time.

**Variant solutions:**
- Add a dedicated `NodeKind::Diagram { kind, source, hints }` variant instead of reusing `Image`. This is more explicit but requires updating every renderer.
- Use `NodeKind::Html { value, block: true }` to carry the pre-rendered SVG, with a hint indicating the source is also available for terminal fallback. This avoids a new variant but mixes concerns.

### Solution: Side-Effect Hooks in the Tree Renderer

#### Side-Effect Hooks in the Tree Renderer

**Description:** Introduce a typed side-effect hook mechanism (similar to the existing `CodeRenderer` hook on `TerminalRenderOptions`) that allows the terminal tree renderer to delegate certain node kinds to external handlers. A `GraphRenderer` trait could be defined:

```rust
pub trait GraphRenderer {
    fn render_graph_terminal(
        &self,
        source: &str,
        syntax: GraphInputSyntax,
        orientation: GraphOrientation,
        theme: GraphColorTheme,
        width: ImageWidth,
        term: &Terminal,
    ) -> Result<String, GraphRenderError>;
}
```

The terminal tree renderer would check for this hook when encountering a graph node. If present, it delegates; if absent, it falls back to a code block.

**Which challenges this helps with:**
- *Terminal-Side Effects in a Pure Tree* -- side effects are isolated in the hook, not in the tree walk itself.
- *Fallible Rendering in an Infallible Tree Walk* -- the hook returns `Result`, and the tree renderer can map errors to diagnostics per the strictness model.
- *Cache Metadata Propagation* -- the hook could attach metadata to a render context that survives the tree walk.

**Variant solutions:**
- Use a general-purpose `ComponentRenderer` trait that covers all inherently visual components (images, graphs, Mermaid diagrams), not just graphs.
- Attach the hook to `NodeAttrs::data` as a function pointer or closure stored in the render options, rather than a separate trait.

### Solution: Post-Render Metadata Collector

#### Post-Render Metadata Collector

**Description:** Add an optional `MetadataCollector` to `TerminalRenderOptions` (or `Rendered<T>`) that components can populate during rendering. This would be a simple `RefCell<BTreeMap<String, serde_json::Value>>` or similar, keyed by node ID or component type. The `GraphExpression` hook would write `png_path` and `cache_hit` into this collector. After the tree walk completes, the caller can extract the metadata.

**Which challenges this helps with:**
- *Cache Metadata Propagation* -- the CLI's `--meta` flag can read from the collector after rendering.

**Variant solutions:**
- Return metadata as part of the `Diagnostic` system (abuse warnings to carry metadata -- fragile).
- Add a generic `Rendered<T>` type parameter that allows T to be a tuple of (String, Metadata) instead of just String.

### Solution: Two-Phase Layout Resolution for Image Nodes

#### Two-Phase Layout Resolution for Image Nodes

**Description:** Extend the tree renderer's layout system with a "pre-flight" pass for image-containing nodes. Before rendering the image, the renderer resolves the layout (margins, alignment) to determine the available width, then passes that width to the image render hook. The hook rasterizes at the correct resolution, and the renderer applies the layout to the resulting output.

This is essentially what `GraphExpression` already does internally (`resolve_dimensions_for` + `render_to_cached_png_at_width` + `apply_block_layout`), but it would need to be split across the tree renderer's layout resolution and the graph hook's rasterization.

**Which challenges this helps with:**
- *Layout Interaction with Image Dimensions* -- the renderer provides the post-margin width, and the hook uses it for rasterization.
- *Lazy vs Eager Artifact Generation* -- rasterization happens at the right time (after layout, during walk), not at tree construction time.

**Variant solutions:**
- Carry unresolved `ImageWidth` on the node and let the renderer resolve it. The renderer already does width resolution for layout; extending it to also resolve image dimensions would be natural.
- Provide the full `TerminalRenderContext` to the graph hook, letting it call `resolve_dimensions_for` itself.

### Solution: Explicitly Document GraphExpression as Out-of-Scope for the Tree

#### Explicitly Document GraphExpression as Out-of-Scope for the Tree

**Description:** Rather than forcing inherently visual components into the tree model, formally document a "bespoke renderer escape hatch" pattern. Components like `GraphExpression`, `MermaidDiagram`, and `TerminalImage` implement `TerminalRenderable` and `BrowserRenderable` directly, and the tree's `render_tree_node()` method returns `None` for them. The `Unsupported` / `TreeProjectionContext` handling already provides a clean fallback path.

This is the approach recommended in `tree-rendering.md` Section 5 ("Leave inherently visual components on bespoke renderers -- they are out of scope for the tree by design").

**Which challenges this helps with:**
- All of them. By not routing `GraphExpression` through the tree, every challenge documented above becomes moot. The component keeps its two-target bespoke implementations, and the tree focuses on document-structural content where it provides genuine value.

**Variant solutions:**
- Create a "visual component" registry that the tree can reference by ID, allowing tree-structured documents to embed visual components by reference rather than by value. The tree node carries a component ID; the renderer looks it up at walk time.
