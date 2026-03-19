# Biscuit Visualized

`biscuit-visualized` is the visualization backend for the terminal and markdown tooling in this monorepo. It renders diagrams in pure Rust and caches generated artifacts in the OS temp directory.

## Capabilities

1. Mermaid diagram rendering via `mermaid-rs-renderer`
2. Graph rendering via `layout-rs`
3. SVG and PNG artifact generation
4. Content-addressed file caching via `biscuit-hash`

This crate is library-only. For terminal display, use the `bt` CLI from [`biscuit-terminal`](../biscuit-terminal/).

## Modules

- `artifact`: output format and render request types
- `cache`: file-backed artifact cache
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

## Graph Example

```rust,no_run
use biscuit_visualized::artifact::{OutputFormat, RenderRequest};
use biscuit_visualized::graph::{GraphDiagram, GraphInputSyntax, GraphOrientation};

let graph = GraphDiagram::parse("a -> b -> c", GraphInputSyntax::Auto)?
    .with_orientation(GraphOrientation::LeftToRight)
    .with_title("Example graph");

let artifact = graph.render(&RenderRequest {
    format: OutputFormat::Svg,
    scale: 1,
    transparent_background: true,
})?;

println!("{}", artifact.path.display());
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

Graph orientation support currently matches `layout-rs`:

- `top-to-bottom`
- `left-to-right`

## CLI Examples

```sh
bt graph-expression --example
bt graph-expression "a -> b -> c"
bt graph-expression --syntax dot "digraph { A -> B; B -> C; }"
bt flowchart --example
bt quadrant --example
```
