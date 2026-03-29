---
name: biscuit-visualized
description: Expert knowledge for the biscuit-visualized Rust library - the authority for diagram and graph visualization rendering in the rusty-biscuit monorepo. Provides Mermaid diagram rendering (flowcharts, sequence, pie, quadrant, gantt, ER, and more), graph/network diagram rendering (expression syntax and DOT format), SVG-to-PNG rasterization via resvg, content-addressed file caching, and dark/light theming. Use when rendering diagrams or graphs to SVG/PNG, working with Mermaid syntax, parsing graph expressions or DOT, building graphs programmatically, configuring visualization themes, or debugging cache behavior. biscuit-terminal depends on this for all diagram artifact generation.
---

# biscuit-visualized

Pure-Rust visualization rendering backend for diagrams and graphs. Generates SVG and PNG artifacts with caching and theming. Library-only (no CLI binary) — terminal display is handled by `biscuit-terminal` adapters.

## Core Principles

1. **Render, don't display**: Produces file artifacts (SVG/PNG); display is the consumer's responsibility
2. **Cache by content**: xxHash-based content-addressed caching avoids redundant renders
3. **Theme-aware**: All renderers accept dark/light themes with sensible defaults
4. **Graceful fallback**: Every diagram type provides `fallback_code_block()` for text-only output

## Quick Start

```rust
use biscuit_visualized::mermaid::{MermaidDiagram, MermaidTheme};
use biscuit_visualized::graph::{GraphDiagram, GraphInputSyntax};
use biscuit_visualized::artifact::{RenderRequest, OutputFormat};

// Mermaid diagram
let diagram = MermaidDiagram::new("flowchart LR\n    A --> B --> C")
    .with_theme(MermaidTheme::Dark);
let artifact = diagram.render(RenderRequest::new(OutputFormat::Svg))?;
println!("SVG at: {}", artifact.path.display());

// Graph from expression syntax
let graph = GraphDiagram::from_expression("a -> b -> c")?;
let artifact = graph.render(RenderRequest::new(OutputFormat::Png).with_scale(2.0))?;
```

## Topics

| Topic | Description |
|-------|-------------|
| [Mermaid Rendering](./mermaid-rendering.md) | MermaidDiagram API, themes, config, quadrant charts |
| [Graph Rendering](./graph-rendering.md) | Expression syntax, DOT format, GraphBuilder, orientation, color themes |
| [Artifacts & Caching](./artifacts-caching.md) | RenderRequest, RenderedArtifact, FileCache, cache layout |
| [Rasterization](./rasterization.md) | SVG-to-PNG via resvg, scale factors, font handling |

## Mermaid Diagrams

### Supported Diagram Types

Flowcharts, Sequence, Class, State, ER, Pie, XY Charts, Quadrant Charts, Gantt, Timeline, Journey, Mindmap, Git Graph

### Themes

```rust
use biscuit_visualized::mermaid::MermaidTheme;

let theme = MermaidTheme::Dark;       // Dark background
let theme = MermaidTheme::Default;    // Light (default Mermaid)
let theme = MermaidTheme::Forest;     // Green tones
let theme = MermaidTheme::Neutral;    // Muted/neutral
let theme = MermaidTheme::for_color_mode(true);  // true = dark
```

### Quadrant Chart Theming

```rust
use biscuit_visualized::mermaid::{MermaidDiagram, QuadrantTheme, MermaidConfig};

let config = QuadrantTheme::MagicQuadrangle.apply(MermaidConfig::default(), true);
let diagram = MermaidDiagram::new("quadrant-beta\n  ...")
    .with_config(config)
    .with_theme(MermaidTheme::Dark);
```

## Graph Diagrams

### Three Input Methods

```rust
use biscuit_visualized::graph::*;

// 1. Expression syntax (lightweight)
let graph = GraphDiagram::from_expression("a -> b -> c; d -> e")?;

// 2. DOT format (full Graphviz)
let graph = GraphDiagram::from_dot("digraph { A -> B; B -> C; }")?;

// 3. Auto-detect syntax
let graph = GraphDiagram::parse(input, GraphInputSyntax::Auto)?;
```

### Expression Syntax Rules

- Directed edges: `a -> b -> c`
- Undirected edges: `a -- b -- c`
- Quoted identifiers: `"My Node" -> "Other Node"`
- Statement separator: `;` or newline
- Mixed `->` and `--` in same graph is rejected

### GraphBuilder (Programmatic)

```rust
use biscuit_visualized::graph::GraphBuilder;

let graph = GraphBuilder::directed()
    .add_node("a", "Start")
    .add_edge("a", "b")
    .add_node("b", "End")
    .with_orientation(GraphOrientation::TopToBottom)
    .build()?;
```

### Orientation and Color Themes

```rust
let graph = GraphDiagram::from_expression("a -> b")?
    .with_orientation(GraphOrientation::LeftToRight)  // or TopToBottom
    .with_color_theme(GraphColorTheme::dark())        // or light()
    .with_title("My Graph");
```

## Rendering and Caching

### RenderRequest

```rust
use biscuit_visualized::artifact::{RenderRequest, OutputFormat};

// SVG (default scale 1.0)
let req = RenderRequest::new(OutputFormat::Svg);

// PNG at 2x scale, transparent background
let req = RenderRequest::new(OutputFormat::Png)
    .with_scale(2.0)
    .with_transparent_background(true);
```

### Cache Behavior

- Location: `$TMPDIR/biscuit-visualized/v1/{mermaid|graph}/{svg|png}/`
- Key: xxHash of `version + kind + source + options + backend_id + format`
- `RenderedArtifact.cache_hit` indicates whether the result was cached
- No automatic cleanup — relies on OS temp directory lifecycle

### Fallback

Every diagram type provides a text fallback for environments without image support:

```rust
let fallback = diagram.fallback_code_block();
// Returns a markdown fenced code block with the source
```

## Relationship to biscuit-terminal

`biscuit-terminal` wraps this library's types with terminal-aware adapters:

| biscuit-visualized | biscuit-terminal adapter |
|---|---|
| `mermaid::MermaidDiagram` | `components::mermaid::MermaidDiagram` |
| `graph::GraphDiagram` | `components::graph_expression::GraphExpression` |

The adapters handle terminal width selection, image protocol rendering, and CLI argument integration. When working on diagram features, changes to rendering logic go in `biscuit-visualized`; changes to terminal display go in `biscuit-terminal`.

## Error Types

```rust
// Mermaid errors
MermaidError::RenderFailed(String)
MermaidError::RasterizationFailed(RasterError)
MermaidError::Io(std::io::Error)

// Graph errors
GraphError::ExpressionParseFailed(String)
GraphError::MixedEdgeKinds
GraphError::DotParseFailed(String)
GraphError::UnsupportedDotFeature(String)
GraphError::RenderFailed(String)
GraphError::RasterizationFailed(RasterError)
GraphError::Io(std::io::Error)

// Raster errors
RasterError::SvgParseFailed(String)
RasterError::RenderFailed(String)
RasterError::IoError(std::io::Error)
```

## Key Files

| Path | Description |
|------|-------------|
| `biscuit-visualized/src/lib.rs` | Root module exports |
| `biscuit-visualized/src/artifact.rs` | OutputFormat, RenderRequest, RenderedArtifact |
| `biscuit-visualized/src/mermaid/render.rs` | MermaidDiagram rendering, SVG post-processing |
| `biscuit-visualized/src/mermaid/config.rs` | MermaidTheme, MermaidConfig, QuadrantTheme |
| `biscuit-visualized/src/graph/expression.rs` | Expression syntax tokenizer/parser |
| `biscuit-visualized/src/graph/dot.rs` | DOT parsing, validation, expression-to-DOT conversion |
| `biscuit-visualized/src/graph/builder.rs` | GraphBuilder fluent API |
| `biscuit-visualized/src/graph/render.rs` | GraphDiagram, GraphColorTheme, orientation |
| `biscuit-visualized/src/cache/file_cache.rs` | Content-addressed FileCache |
| `biscuit-visualized/src/raster/png.rs` | SVG-to-PNG rasterization via resvg |

## Dependencies

| Crate | Purpose |
|-------|---------|
| `mermaid-rs-renderer` v0.2 | Mermaid diagram rendering backend |
| `layout-rs` v0.1 | Graph layout engine and DOT parsing |
| `resvg` v0.45 | SVG-to-PNG rasterization |
| `biscuit-hash` | xxHash for cache key generation |
| `serde` + `serde_json` | Theme config serialization |
| `thiserror` v2.0 | Error type derivation |
| `tempfile` v3 | Temporary file operations |
| `tracing` v0.1 | Structured logging |

## Development Commands

```bash
just -f biscuit-visualized/justfile build   # Build the library
just -f biscuit-visualized/justfile test    # Run all tests
just -f biscuit-visualized/justfile lint    # Run clippy
```

## Implementation Notes

- Mermaid SVG post-processing fixes legend text alignment (`dominant-baseline="central"`)
- Pie chart text contrast is adjusted per-slice using WCAG luminance formula
- Graph SVG post-processing trims layout-rs canvas padding and applies custom fonts/colors
- layout-rs supports LR and TB orientations; `GraphOrientation` maps to these
- System font database is lazily loaded with `OnceLock` for thread-safe caching in rasterization
