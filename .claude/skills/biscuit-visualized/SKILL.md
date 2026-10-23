---
name: biscuit-visualized
description: Expert knowledge for the biscuit-visualized Rust library - the authority for diagram and graph visualization rendering in the rusty-biscuit monorepo. Provides Mermaid diagram rendering (flowcharts, sequence, pie, quadrant, gantt, ER, and more), graph/network diagram rendering (expression syntax and DOT format), SVG-to-PNG rasterization via resvg, content-addressed file caching, and dark/light theming. Use when rendering diagrams or graphs to SVG/PNG, working with Mermaid syntax, parsing graph expressions or DOT, building graphs programmatically, configuring visualization themes, or debugging cache behavior. Also owns the terminal diagram components (`MermaidDiagram`, `GraphExpression`, `GitGraph`) and the render-tree Mermaid hook behind its `terminal` feature; it depends on biscuit-terminal, never the reverse.
---

# biscuit-visualized

Pure-Rust visualization rendering backend for diagrams and graphs. Generates SVG and PNG artifacts with caching and theming. Library-only (no CLI binary); behind its `terminal` feature it also owns the diagram components that display through `biscuit-terminal`.

## Core Principles

1. **Render, don't display**: produces file artifacts (SVG/PNG); display is the consumer's (or the `terminal` components') job
2. **Cache by content**: xxHash content-addressed keys avoid redundant renders
3. **Measure what you draw**: one font-resolution chain serves Merman's text measurer and the rasterizer
4. **Graceful fallback**: every diagram type provides `fallback_code_block()`; any Mermaid failure (parse error, unsupported family, time budget) falls back to it

## Quick Start

```rust
use biscuit_visualized::mermaid::{MermaidDiagram, MermaidTheme};
use biscuit_visualized::graph::GraphDiagram;
use biscuit_visualized::artifact::{OutputFormat, RenderRequest};

// Mermaid diagram (rendered by Merman), vector SVG export
let diagram = MermaidDiagram::new("flowchart LR\n    A --> B --> C")
    .with_theme(MermaidTheme::Dark);
let svg_req = RenderRequest { format: OutputFormat::Svg, ..RenderRequest::default() };
let artifact = diagram.render(&svg_req)?;

// Graph from expression syntax, rasterized at a known display width
let graph = GraphDiagram::from_expression("a -> b -> c")?;
let artifact = graph.render(&RenderRequest::default().with_target_width(1600))?;
```

## Topics

| Topic | Description |
|-------|-------------|
| [Mermaid Rendering](./mermaid-rendering.md) | Merman pipeline, `MermaidDiagram` API, time budget, fallback, themes (interim), config, post-process, gitGraph `commitSpacing` |
| [Diagram Components](./diagram-components.md) | Terminal components, `TerminalDiagramRenderer` hook, GitGraph contract, `l1` tests |
| [Graph Rendering](./graph-rendering.md) | Expression syntax, DOT format, GraphBuilder, orientation, color themes, terminal resolution tuning |
| [Artifacts & Caching](./artifacts-caching.md) | RenderRequest, RenderedArtifact, FileCache, cache layout, Mermaid cache key |
| [Rasterization](./rasterization.md) | SVG-to-PNG via resvg, scale factors, font chain, terminal protocol ceiling |

### In-repo docs

- `biscuit-visualized/docs/merman.md`: the Mermaid pipeline with owners, time budget, fallback, themes, fonts (`MERMAID_FONT`, resolution chain, bundled Inter / Noto Emoji), the `\n` rewrite, post-process, the HTML static target, cache identity, and what is **planned** (terminal-derived palettes and accents).
- `biscuit-visualized/docs/mermaid-gitgraph.md`: gitGraph tag spacing (`commitSpacing` written only when tags on different commits collide, to a one-em gap; an author's own value wins) and the `gitgraph_geometry()` test API.
- `biscuit-visualized/docs/dot-graph.md`: the DOT subset, cluster syntax, and the resolution-tuning case study.

## Mermaid in one paragraph

`mermaid::MermaidDiagram::render` builds the font chain, rewrites the source (`\n` → `<br/>` labels for class, ER, mindmap, requirement; `commitSpacing` for a tagged LR gitGraph whose tags collide, found with one Merman `layout_json` pass), checks the cache, and on a miss calls Merman (fork `yankeeinlondon/merman` rev `6e6b1e6554e8`) for a resvg-safe SVG measured with our `LabelMeasurer`, then post-processes (solid gradient strokes, `sans-serif` default; for SVG with `with_target(MermaidTarget::HtmlStatic)`, measured face first in every `font-family` and visible label overflow, never in PNG), paints the background, and rasterizes for PNG. Each Merman request has its own deadline (`with_time_budget`, default `DEFAULT_TIME_BUDGET` = 2 s) covering Merman's work only. Failures are `MermaidError::RenderFailed`: parse errors, unknown types, ZenUML (by header), math labels (`$$…$$`, math not compiled in), and an exceeded budget. Themes `Dark`/`Default`/`Forest`/`Neutral` currently map to Merman's built-in themes of the same name; compiled palettes are **planned**. Details: [Mermaid Rendering](./mermaid-rendering.md).

### Themes and Quadrant Config

```rust
use biscuit_visualized::mermaid::{MermaidConfig, MermaidDiagram, MermaidTheme, QuadrantTheme};

let theme = MermaidTheme::for_color_mode(true);  // true = Dark, false = Default
let config = QuadrantTheme::MagicQuadrangle.apply(MermaidConfig::default(), true);
let diagram = MermaidDiagram::new("quadrantChart\n  ...").with_config(config).with_theme(theme);
```

## Graph Diagrams

```rust
use biscuit_visualized::graph::*;

let graph = GraphDiagram::from_expression("a -> b -> c; d -> e")?;     // expression syntax
let graph = GraphDiagram::from_dot("digraph { A -> B; B -> C; }")?;     // DOT subset
let graph = GraphDiagram::parse(input, GraphInputSyntax::Auto)?;        // auto-detect
let graph = GraphBuilder::directed().add_node("a", "Start").add_edge("a", "b").build()?;
let graph = graph.with_orientation(GraphOrientation::LeftToRight).with_color_theme(GraphColorTheme::dark());
```

Expression rules: `->` directed, `--` undirected (never mixed in one graph), quoted identifiers, `;` or newline separators. See [Graph Rendering](./graph-rendering.md).

## Rendering and Caching

`RenderRequest` is a plain struct; use `..RenderRequest::default()` or the builders `with_target_width`, `with_row_height`, `with_background`:

```rust
use biscuit_visualized::artifact::{Background, OutputFormat, RenderRequest};

let req = RenderRequest::default().with_target_width(1600);            // PNG at display width
let req = RenderRequest { format: OutputFormat::Png, scale: 2,
    background: Background::Transparent, ..RenderRequest::default() };  // HiDPI multiplier
let req = RenderRequest::default().with_target_width(540).with_row_height(18); // whole cell rows
```

`target_width` takes precedence over `scale`; `row_height` pads PNG height (ignored for SVG). `Background` is `Theme` (default) | `Transparent` | `Solid { r, g, b }`.

- Cache location: `$TMPDIR/biscuit-visualized/v1/{mermaid|graph}/{svg|png}/` (or `$BISCUIT_VISUALIZED_CACHE_DIR/v1/`)
- Key: xxHash of `v2 | kind | source | options | backend_id | format`. Mermaid hashes the **rewritten** source, backend `merman@6e6b1e6554e8+bv1`, and options: site-config hash (theme + `MermaidConfig`), font identity, title, and `RenderRequest::cache_identity()`. See [Artifacts & Caching](./artifacts-caching.md)
- `RenderedArtifact.cache_hit` reports a hit; no automatic cleanup

## Relationship to biscuit-terminal

**This crate decides what a diagram shows; `biscuit-terminal` decides how it reaches the screen.** The `terminal` feature adds the components, which wrap this library's artifact types and display through `biscuit-terminal`'s `TerminalImage`:

| Artifact type | Component (`terminal` feature) |
|---|---|
| `mermaid::MermaidDiagram` | `components::mermaid::MermaidDiagram` |
| `graph::GraphDiagram` | `components::graph_expression::GraphExpression` |
| — | `components::git_graph::GitGraph` |

The dependency runs one way (this crate → `biscuit-terminal`). See [Diagram Components](./diagram-components.md).

## Error Types

```rust
MermaidError::{RenderFailed(String), RasterizationFailed(RasterError), Io(io::Error)}
GraphError::{ExpressionParseFailed(String), MixedEdgeKinds, DotParseFailed(String),
    UnsupportedDotFeature(String), RenderFailed(String), RasterizationFailed(RasterError), Io(io::Error)}
RasterError::{SvgParseFailed(String), RenderFailed(String), IoError(io::Error)}
```

## Key Files

| Path (under `biscuit-visualized/src/src/`) | Description |
|------|-------------|
| `lib.rs` | Root module exports |
| `artifact.rs` | OutputFormat, RenderRequest, RenderedArtifact, Background |
| `mermaid/render.rs` | `MermaidDiagram`: pipeline, Merman requests, time budget, cache key, background, `natural_size` |
| `mermaid/newline.rs` | The `\n` → `<br/>` label rewrite (four families) |
| `mermaid/gitgraph_layout.rs` | Merman layout data → `GitGraphGeometry`; `commitSpacing` computation and directive |
| `mermaid/gitgraph.rs` | `GitGraphGeometry` types (doc-hidden), `TAG_GAP_EM`, `default_gitgraph_commit_step()` |
| `mermaid/postprocess.rs` | Gradient-stroke fix, `sans-serif` default |
| `mermaid/config.rs` | MermaidTheme, MermaidConfig, QuadrantTheme |
| `graph/{expression,dot,builder,render}.rs` | Expression parser, DOT, GraphBuilder, GraphDiagram |
| `cache/file_cache.rs` | `FileCache`, `MERMAID_BACKEND` |
| `raster/png.rs` | SVG-to-PNG rasterization via resvg |
| `fonts/` | Font chain (`chain.rs`, cache identity), shaper (`shaper.rs`), Merman measurer (`measure.rs`); crate-private |

## Dependencies

| Crate | Purpose |
|-------|---------|
| `merman` (git: `yankeeinlondon/merman`, rev `6e6b1e6554e8e75d468c1f991652c1c53a85bace`, branch `gitgraph-commit-spacing`) | Mermaid parse/layout/SVG; features `svg`, `all-diagrams`, `layout-cytoscape` (no `png`, `native-fonts`, ELK, `math`). The fork adds gitGraph `commitSpacing`. **Side effect:** `merman-core` enables serde_json's `preserve_order` (and `float_roundtrip`), and Cargo unifies it into every build that reaches this crate (darkmatter, sniff-cli, worktree-cli, claudine, …): `serde_json::Map` keeps insertion order instead of sorting, so code or tests that relied on sorted keys change. Merman needs it (its models iterate JSON maps in insertion order) |
| `layout-rs` v0.1 | Graph layout engine and DOT parsing |
| `resvg` v0.47 | SVG-to-PNG rasterization |
| `rustybuzz` 0.20.1, `ttf-parser` 0.25.1, `unicode-bidi`, `unicode-script`, `fontique` 0.12.0 | Font resolution and text measurement for Merman layout and rasterization (feature `image`); rustybuzz and ttf-parser stay on usvg's versions |
| Bundled fonts (`src/assets/fonts/`) | Inter 4.1 Regular/Bold (every OS) and Noto Emoji (Linux only), registered in the font chain |
| `biscuit-hash` | xxHash for cache keys and font identity |
| `serde` + `serde_json` | Site config and Merman layout data |
| `thiserror`, `tempfile`, `tracing` | Errors, temp files, logging |

## Development Commands

```bash
just -f biscuit-visualized/justfile build   # Build the library
just -f biscuit-visualized/justfile test    # Run all tests
just -f biscuit-visualized/justfile lint    # Run clippy
```

## Implementation Notes

- Graph SVG post-processing trims layout-rs canvas padding and applies custom fonts/colors; layout-rs supports LR and TB
- Rasterization draws text through `fonts::default_chain()` (lazily built behind `OnceLock`); see [Rasterization](./rasterization.md#font-database-and-resolution)
- Bump `+bv<n>` in `MERMAID_BACKEND` whenever the rewrites, post-process, or background change output; bump the rev part with the Merman pin
