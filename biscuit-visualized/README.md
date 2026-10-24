# Biscuit Visualized

`biscuit-visualized` owns diagrams for every target in this monorepo. It renders Mermaid and graph artifacts in pure Rust, caches generated SVG/PNG outputs in the OS temp directory, and, behind its `terminal` feature, provides the diagram components that terminal, browser, and render-tree callers use.

## Capabilities

1. Mermaid diagram rendering via [Merman](docs/merman.md), a pure-Rust reimplementation of Mermaid.js, with labels measured and drawn in the same fonts
2. Graph rendering via `layout-rs`, from arrow expressions (`a -> b`), DOT, or a builder
3. SVG and PNG artifact generation
4. Content-addressed file caching via `biscuit-hash`
5. Diagram components (`MermaidDiagram`, `GraphExpression`, `GitGraph`) for terminal, browser, and render-tree output (`terminal` feature)

This crate is library-only. It decides what a diagram shows; [`biscuit-terminal`](../biscuit-terminal/) decides how pixels reach the screen. The dependency runs one way: with `terminal` enabled this crate depends on `biscuit-terminal`, never the reverse. The `bt` CLI exposes the components as commands.

## Installation

It is a workspace crate; depend on it by path from another package in the monorepo:

```toml
[dependencies]
biscuit-visualized = { path = "../../biscuit-visualized/src", features = ["terminal"] }
```

| Feature | Default | Description |
|---------|---------|-------------|
| `image` | No | SVG rasterization and Mermaid rendering (`resvg`, Merman) |
| `terminal` | No | The diagram components and the render-tree diagram hook; enables `image` and depends on `biscuit-terminal` |
| `clap` | No | Derive `clap::ValueEnum` on [`QuadrantTheme`](src/src/mermaid/config.rs), so a CLI can take a quadrant theme as an argument |

Without `image` only the `artifact` and `cache` modules are compiled.

## Modules

- [`artifact`](src/src/artifact.rs): output format and render request types
- [`cache`](src/src/cache/mod.rs): file-backed artifact cache
- [`components`](src/src/components/mod.rs): the diagram components and `TerminalDiagramRenderer`, the render-tree hook that draws Mermaid fences (`terminal` feature); see [`docs/components/`](docs/components/mermaid_diagram.md)
- [`graph`](src/src/graph/mod.rs): expression syntax, DOT rendering, and programmatic graph building
- [`mermaid`](src/src/mermaid/mod.rs): Mermaid diagrams, themes, and quadrant-specific configuration
- [`raster`](src/src/raster/mod.rs): SVG-to-PNG conversion

## Mermaid Example

```rust,no_run
use biscuit_visualized::artifact::RenderRequest;
use biscuit_visualized::mermaid::{MermaidDiagram, MermaidTheme};

let diagram = MermaidDiagram::new("flowchart LR\n    A --> B")
    .with_theme(MermaidTheme::Dark)
    .with_title("Example flow");

let artifact = diagram.render(&RenderRequest::default())?;
println!("{}", artifact.path.display());
# Ok::<(), Box<dyn std::error::Error>>(())
```

This writes a cached PNG and returns its path. A diagram that cannot be drawn is an error, and its source can be shown as a code block instead; the terminal component does that for you. See [Merman rendering](docs/merman.md) for the pipeline, themes, fonts, and [when a diagram falls back to its source](docs/merman.md#when-a-diagram-falls-back-to-its-source), and [Mermaid gitGraph tag spacing](docs/mermaid-gitgraph.md) for how gitGraph tags are kept apart.

## Graph Example

```rust,no_run
use biscuit_visualized::artifact::{Background, OutputFormat, RenderRequest};
use biscuit_visualized::graph::{GraphColorTheme, GraphDiagram, GraphInputSyntax, GraphOrientation};

let graph = GraphDiagram::parse("a -> b -> c", GraphInputSyntax::Auto)?
    .with_orientation(GraphOrientation::LeftToRight)
    .with_color_theme(GraphColorTheme::dark())
    .with_title("Example graph");

let artifact = graph.render(&RenderRequest {
    format: OutputFormat::Svg,
    background: Background::Transparent,
    ..RenderRequest::default()
})?;

println!("{}", artifact.path.display());
# Ok::<(), Box<dyn std::error::Error>>(())
```

The same call accepts undirected expressions (`a -- b -- c`) and DOT (`digraph { A -> B; B -> C; }`). Graphs can also be built in code:

```rust,no_run
use biscuit_visualized::graph::{GraphBuilder, GraphOrientation};

let graph = GraphBuilder::directed()
    .with_orientation(GraphOrientation::LeftToRight)
    .add_node("app", Some("App".to_string()))
    .add_node("db", Some("Database".to_string()))
    .add_edge("app", "db")
    .build()?;
# Ok::<(), Box<dyn std::error::Error>>(())
```

See [DOT graph rendering](docs/dot-graph.md) for the supported grammar, backgrounds, and pixel-exact sizing for terminals, and [GraphExpression](docs/components/graph_expression.md) for the input syntaxes and the terminal component.

## CLI Examples

```sh
bt graph-expression --example
bt graph-expression "a -> b -> c"
bt graph-expression --syntax dot "digraph { A -> B; B -> C; }"
bt graph-expression --meta "a -> b"
bt flowchart --example
bt quadrant --example
bt git-graph --example
```

## Documentation

- [Merman rendering](docs/merman.md): how Mermaid diagrams are rendered, themed, cached, and tested
- [Mermaid gitGraph tag spacing](docs/mermaid-gitgraph.md)
- [Embedded font for Mermaid static SVG](docs/mermaid-embedded-font.md): an HTML page that carries the face its diagrams were measured in
- [DOT graph rendering](docs/dot-graph.md)
- Components: [MermaidDiagram](docs/components/mermaid_diagram.md), [GraphExpression](docs/components/graph_expression.md), [GitGraph](docs/components/git_graph.md)
- [Dependencies](docs/dependencies.md)
