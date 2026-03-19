# Biscuit Visualized

A pure Rust library for creating high-quality data visualizations. This library provides:

1. **Mermaid Diagrams**

    Convert [MermaidJS](https://mermaid.js.org/) code blocks into scalar images (`.png`) using the `mermaid-rs-renderer` crate. This pure Rust implementation is anywhere between 100% to 1600% faster than using the Mermaid CLI (with JavaScript). No external dependencies required.

2. **Graph Visualizations**

    Visualize graph structures using `layout-rs` with support for both programmatic creation and string-based descriptions. Supports multiple syntaxes:
    - **Arrow syntax**: `a -> b -> c` (directed edges)
    - **Dash syntax**: `a -- b -- c` (undirected edges)
    - **DOT syntax**: Full [Graphviz DOT](https://graphviz.org/doc/info/lang.html) language support

3. **SVG Rasterization**

    Convert SVG images to PNG using `resvg` for terminal rendering.

4. **File Caching**

    Content-hashed caching of rendered diagrams using `biscuit-hash` xxHash keys. Cached files are stored in the OS temp directory for fast re-rendering.

This crate is a library only. For CLI access to these capabilities, use the `biscuit-terminal` CLI which provides convenient terminal rendering.

## Module Overview

- **`artifact`** - Artifact types for generated visualizations (PNG, SVG)
- **`cache`** - Content-addressed file caching with xxHash keys
- **`mermaid`** - Mermaid diagram rendering via `mermaid-rs-renderer`
- **`graph`** - Graph visualization rendering via `layout-rs` with multiple syntaxes
- **`raster`** - SVG-to-PNG conversion via `resvg`

## Usage Examples

### Mermaid Diagrams

```rust
use biscuit_visualized::mermaid::MermaidDiagram;

// Create and render a flowchart
let diagram = MermaidDiagram::new(
    "flowchart LR\n    A --> B --> C",
    None,  // title
)?;

// Render to PNG
let png_path = diagram.render()?;
println!("Rendered to: {}", png_path.display());
```

### Graph Visualizations

```rust
use biscuit_visualized::graph::{GraphDiagram, GraphInputSyntax, GraphOrientation};

// Arrow syntax (directed graph)
let graph = GraphDiagram::new(
    "a -> b -> c; b -> d",
    GraphInputSyntax::Arrow,
    Some("My Graph"),
    GraphOrientation::LeftToRight,
)?;

// Dash syntax (undirected graph)
let graph = GraphDiagram::new(
    "a -- b -- c",
    GraphInputSyntax::Dash,
    None,
    GraphOrientation::TopToBottom,
)?;

// DOT syntax (full Graphviz)
let graph = GraphDiagram::new(
    "digraph { A -> B; B -> C; }",
    GraphInputSyntax::Dot,
    None,
    GraphOrientation::LeftToRight,
)?;

// Render to PNG
let png_path = graph.render()?;
```

## Expression Syntax for Graphs

### Arrow Syntax (Directed)

Simple directed edges using `->`:

```
a -> b -> c
a -> b; b -> c; c -> a
start -> validate -> render
```

### Dash Syntax (Undirected)

Undirected edges using `--`:

```
a -- b -- c
node1 -- node2; node2 -- node3
```

### DOT Syntax

Full Graphviz DOT language support:

```
digraph {
    A -> B;
    B -> C;
    C -> A;
}
```

## CLI Examples using `bt`

Try these commands with the `biscuit-terminal` CLI:

```sh
# Mermaid diagrams
bt bar-chart --example
bt pie-chart --example
bt git-graph --example
bt flowchart --example
bt quadrant --example
bt state-diagram --example
bt erd --example

# Graph visualization (using layout-rs)
bt graph-expression --example
bt graph-expression "a -> b -> c"
bt graph-expression "a -- b -- c"
bt graph-expression --syntax dot "digraph { A -> B; B -> C; }"
```
