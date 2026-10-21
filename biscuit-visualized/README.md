# Biscuit Visualized

`biscuit-visualized` owns diagrams for every target in this monorepo. It renders Mermaid and graph artifacts in pure Rust, caches generated SVG/PNG outputs in the OS temp directory, and, behind its `terminal` feature, provides the diagram components that terminal, browser, and render-tree callers use.

## Capabilities

1. Mermaid diagram rendering via `mermaid-rs-renderer`
2. Graph rendering via `layout-rs`
3. SVG and PNG artifact generation
4. Content-addressed file caching via `biscuit-hash`
5. Diagram components (`MermaidDiagram`, `GraphExpression`, `GitGraph`) for terminal, browser, and render-tree output (`terminal` feature)

This crate is library-only. It decides what a diagram shows; [`biscuit-terminal`](../biscuit-terminal/) decides how pixels reach the screen. The dependency runs one way: with `terminal` enabled this crate depends on `biscuit-terminal`, never the reverse. The `bt` CLI exposes the components as commands.

## Cargo Features

| Feature | Default | Description |
|---------|---------|-------------|
| `image` | No | SVG rasterization and Mermaid rendering (`resvg`, `mermaid-rs-renderer`) |
| `terminal` | No | The diagram components and the render-tree diagram hook; enables `image` and depends on `biscuit-terminal` |
| `clap` | No | Derive `clap::ValueEnum` on `QuadrantTheme` |

## Modules

- `artifact`: output format and render request types
- `cache`: file-backed artifact cache
- `components`: the diagram components and `TerminalDiagramRenderer`, the render-tree hook that draws Mermaid fences (`terminal` feature); see [`docs/components/`](docs/components/mermaid_diagram.md)
- `graph`: expression syntax, DOT rendering, and programmatic graph building
- `mermaid`: Mermaid diagrams, themes, and quadrant-specific configuration
- `raster`: SVG-to-PNG conversion

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

The artifact API only generates the file. Width selection, render metadata, and fallback-to-code-block behavior live in the `components` module's `MermaidDiagram`; terminal image protocols live in `biscuit-terminal`.

For a `gitGraph`, rendering also restores the second parent of labeled merges and spaces commits so tags never overlap. See [Mermaid gitGraph corrections](docs/mermaid-gitgraph.md).

## Graph Example

```rust,no_run
use biscuit_visualized::artifact::{Background, OutputFormat, RenderRequest};
use biscuit_visualized::graph::{GraphDiagram, GraphInputSyntax, GraphOrientation};

let graph = GraphDiagram::parse("a -> b -> c", GraphInputSyntax::Auto)?
    .with_orientation(GraphOrientation::LeftToRight)
    .with_title("Example graph");

let artifact = graph.render(&RenderRequest {
    format: OutputFormat::Svg,
    background: Background::Transparent,
    ..RenderRequest::default()
})?;

println!("{}", artifact.path.display());
# Ok::<(), Box<dyn std::error::Error>>(())
```

`Background` is `Theme` by default; `Transparent` leaves the area behind the diagram unpainted and `Solid { r, g, b }` fills it with one color. For PNG output, `RenderRequest::with_target_width(px)` renders at an exact pixel width and `with_row_height(px)` pads the height to whole rows of that many pixels, which is how the terminal components fill the cells they reserve. See [DOT graph rendering](docs/dot-graph.md) for the details.

Graph diagrams also support explicit theming before rendering:

```rust,no_run
use biscuit_visualized::graph::{GraphColorTheme, GraphDiagram, GraphInputSyntax};

let graph = GraphDiagram::parse("a -> b -> c", GraphInputSyntax::Auto)?
    .with_color_theme(GraphColorTheme::dark())
    .with_title("Dark graph");
# Ok::<(), Box<dyn std::error::Error>>(())
```

## Programmatic Graph Builder

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

## Graph Input Formats

- Expression syntax: `a -> b -> c`
- Undirected expression syntax: `a -- b -- c`
- DOT: `digraph { A -> B; B -> C; }`

Mixed directed and undirected expression syntax is not supported in a single parsed expression.

Graph orientation support currently matches `layout-rs`:

- `top-to-bottom`
- `left-to-right`

## CLI Examples

```sh
bt graph-expression --example
bt graph-expression "a -> b -> c"
bt graph-expression --syntax dot "digraph { A -> B; B -> C; }"
bt graph-expression --meta "a -> b"
bt flowchart --example
bt quadrant --example
```
