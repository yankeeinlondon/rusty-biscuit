# Data Visualization Tech Design

This document defines the implementation-ready technical design for the data visualization work described in:

- `biscuit-terminal/features/2026-03-18. Data Visualization.md/spec.md`
- `biscuit-terminal/docs/data-visualization/visualizing-graph-expressions.md`
- the current Mermaid implementation in `biscuit-terminal/lib/src/components/mermaid.rs`
- the current Mermaid cache in `biscuit-terminal/lib/src/components/mermaid_cache.rs`
- the current CLI command surface in `biscuit-terminal/cli/src/args.rs` and `biscuit-terminal/cli/src/commands.rs`
- the new package stub in `biscuit-visualized/src`

The design goal is to move visualization generation into `biscuit-visualized`, replace the current `mmdc`/Node.js execution model with native Rust rendering, add graph visualization support, and keep `biscuit-terminal` focused on terminal presentation rather than visualization ownership.

## Overview

Today `biscuit-terminal` does two different jobs:

1. it detects and renders to terminal capabilities
2. it owns Mermaid diagram generation and caching

That boundary made sense when Mermaid rendering was fundamentally terminal-oriented and depended on the external `mmdc` CLI. It no longer fits the intended future state.

After this change:

- `biscuit-visualized` will own visualization generation, artifact caching, and format conversion
- `biscuit-terminal` will own only terminal-specific concerns:
  - width/layout handling
  - terminal image display
  - terminal-aware defaults
  - terminal-facing wrapper APIs and CLI commands
- Mermaid rendering will move from `mmdc` to the Rust crate the spec refers to as `mermaid-rs`; the concrete dependency should be `mermaid-rs-renderer`
- graph visualization will be added using `layout-rs` for DOT parsing/layout and `resvg` for PNG rasterization

## Goals

1. Move Mermaid rendering and caching out of `biscuit-terminal` and into `biscuit-visualized`.
2. Remove the Node/npm/Chromium dependency chain from Mermaid rendering.
3. Keep temp-file-based caching, but generalize it so it works for both Mermaid and graph visualizations.
4. Add a programmatic graph API in `biscuit-visualized`.
5. Add text-based graph rendering from:
   - a lightweight graph-expression syntax for CLI convenience
   - DOT input for Markdown code blocks and advanced users
6. Add `bt graph-expression <exp>` with the same terminal display ergonomics as the existing diagram commands.
7. Preserve a usable `biscuit-terminal` Mermaid API for terminal consumers while moving the real implementation boundary down into `biscuit-visualized`.
8. Make the design suitable for `darkmatter` to consume directly for future ` ```mermaid ` and ` ```dot ` rendering.

## Non-Goals

1. Building a full Graphviz-compatible rendering engine beyond what `layout-rs` supports.
2. Preserving the `mmdc`-specific API surface indefinitely.
3. Adding a general charting abstraction beyond Mermaid plus graph visualization.
4. Implementing `darkmatter` DOT code block rendering in this feature.
5. Designing a large styling DSL for graphs in v1.

## Primary Decisions

### 1. `biscuit-visualized` becomes the canonical visualization crate

This is the main architectural move. The crate should own:

- Mermaid parsing and rendering
- graph parsing/building and rendering
- SVG/PNG artifact generation
- file-based temp cache
- backend-neutral render errors

It should not own:

- terminal capability detection
- terminal image protocol selection
- terminal layout/margin/alignment policy

### 2. Standardize on SVG as the canonical internal artifact

Both backends fit this well:

- `mermaid-rs-renderer` can render SVG directly
- `layout-rs` naturally renders SVG

Terminal display still requires PNG, so PNG becomes a derived artifact produced via `resvg`.

This gives a cleaner cross-surface model:

- HTML consumers can use SVG directly
- terminal consumers request PNG derived from the same cached source artifact
- both backends share one rasterization path

If later benchmarking shows that direct Mermaid-to-PNG is materially better, the implementation can optimize that path behind the same public API without changing the crate boundary.

### 3. No broad re-export of `biscuit-visualized` from `biscuit-terminal`

This is the explicit position for the open question in the spec.

`biscuit-terminal` should **not** become a pass-through API crate for `biscuit-visualized`.

Reasoning:

1. `biscuit-terminal` is a terminal library, not the home of reusable visualization semantics.
2. Broad re-exports make the dependency graph lie. A downstream crate that wants HTML/SVG generation should depend on `biscuit-visualized` directly.
3. `darkmatter` is the strongest argument against re-exporting. It is already a cross-surface renderer. It should depend on the visualization crate directly for HTML and Markdown code-block rendering instead of continuing to route that responsibility through `biscuit-terminal`.
4. A broad re-export would freeze `biscuit-terminal` as the accidental canonical public surface for visualization, which is exactly what this split is trying to undo.

What `biscuit-terminal` should expose instead:

- terminal-specific wrapper types such as `MermaidRenderer` and `GraphExpressionRenderer`
- helper functions that render a visualization artifact into a terminal
- compatibility shims for current Mermaid terminal users during migration

What it should not expose:

- a blanket `pub use biscuit_visualized::*`
- a mirrored copy of the entire `biscuit-visualized` module tree

### 4. Keep current Mermaid ergonomics where they are terminal-oriented

The current builder-style Mermaid API is good for terminal usage:

- `MermaidRenderer::new(...)`
- `MermaidRenderer::for_terminal(...)`
- `with_theme(...)`
- `with_title(...)`
- `render_to_cached_png()`

That API can remain in `biscuit-terminal` as a thin terminal adapter. The logic behind it should move into `biscuit-visualized`.

### 5. Graph input will support both DOT and a small native expression syntax

The spec asks for:

- programmatic graph visualization
- DOT code block rendering
- a CLI form `bt graph-expression <exp>`

The research shows that `layout-rs` covers DOT very well, but it does not provide a separate ergonomic CLI syntax for quick one-liners. To satisfy the CLI requirement cleanly, v1 should support:

- `GraphInputSyntax::Expression`
- `GraphInputSyntax::Dot`
- `GraphInputSyntax::Auto`

`Auto` will be the CLI default.

## Proposed Package Layout

The new crate should replace the current placeholder in `biscuit-visualized/src/src/lib.rs` with this structure:

```txt
biscuit-visualized/src/src/
├── lib.rs
├── artifact.rs
├── cache/
│   ├── mod.rs
│   └── file_cache.rs
├── raster/
│   ├── mod.rs
│   └── png.rs
├── mermaid/
│   ├── mod.rs
│   ├── config.rs
│   ├── error.rs
│   └── render.rs
└── graph/
    ├── mod.rs
    ├── builder.rs
    ├── error.rs
    ├── expression.rs
    ├── dot.rs
    └── render.rs
```

Recommended `lib.rs` exports:

```rust
pub mod artifact;
pub mod cache;
pub mod raster;
pub mod mermaid;
pub mod graph;
```

## Shared Artifact Model

The shared layer is what makes the package split worth doing.

Recommended types:

```rust
pub enum OutputFormat {
    Svg,
    Png,
}

pub struct RenderRequest {
    pub format: OutputFormat,
    pub scale: u32,
    pub transparent_background: bool,
}

pub struct RenderedArtifact {
    pub path: PathBuf,
    pub format: OutputFormat,
    pub cache_hit: bool,
    pub alt_text: String,
}
```

Notes:

- `scale` only affects raster outputs
- `transparent_background` remains relevant for both Mermaid and graph PNG output
- `RenderedArtifact` intentionally returns a file path because both terminal rendering and current cache behavior are file-based

## Cache Design

### Ownership

The cache moves into `biscuit-visualized`.

`biscuit-terminal` should not continue to own `MermaidCache` once the rendering engines live elsewhere.

### Directory layout

Recommended temp layout:

```txt
$TMPDIR/biscuit-visualized/
└── v1/
    ├── mermaid/
    │   ├── svg/
    │   └── png/
    └── graph/
        ├── svg/
        └── png/
```

### Cache key

The key should include:

- cache schema version
- visualization kind: `mermaid` or `graph`
- input syntax kind where relevant: `expression` or `dot`
- normalized source input
- render options affecting output
- backend identifier and version string
- output format

Recommended shape:

```text
v1|kind|syntax|source|options-json|backend-id|format
```

Use `biscuit-hash::xx_hash` for the final filename hash, just like the current cache.

### Backend identifiers

Use explicit constants rather than runtime introspection:

- `mermaid-rs-renderer@0.2.x`
- `layout-rs@0.1.x`
- `resvg@0.45`

This keeps cache invalidation deterministic when backend upgrades change output.

## Mermaid Design

### New core API in `biscuit-visualized`

Recommended public API:

```rust
pub struct MermaidDiagram {
    instructions: String,
    title: Option<String>,
    theme: MermaidTheme,
    config: MermaidConfig,
}

impl MermaidDiagram {
    pub fn new(instructions: impl Into<String>) -> Self;
    pub fn with_theme(self, theme: MermaidTheme) -> Self;
    pub fn with_title(self, title: impl Into<String>) -> Self;
    pub fn with_config(self, config: MermaidConfig) -> Self;
    pub fn render(&self, request: &RenderRequest) -> Result<RenderedArtifact, MermaidError>;
    pub fn fallback_code_block(&self) -> String;
}
```

The existing `MermaidTheme`, `MermaidConfig`, and `QuadrantTheme` concepts should move here.

### Behavior changes from the current implementation

1. Remove all `mmdc`/npm/npx/chromium detection logic.
2. Remove the current 10KB content limit. That limit was tied to shelling out to `mmdc` and is no longer justified.
3. Replace CLI-specific errors with backend-neutral errors:
   - parse/render failure
   - rasterization failure
   - I/O failure
4. Keep `fallback_code_block()` because it is still useful in terminal and markdown contexts.

### `biscuit-terminal` adapter

`biscuit-terminal::components::mermaid::MermaidRenderer` should remain, but it becomes a thin adapter over `biscuit_visualized::mermaid::MermaidDiagram`.

That wrapper continues to provide:

- `new`
- `for_terminal`
- `with_theme`
- `with_title`
- `with_config`
- `render_to_cached_png`
- `render_for_terminal`

It should stop exposing `mmdc`-specific concepts such as:

- `MMDC_MIN_VERSION`
- `MmdcVersion`
- `detect_mmdc_version`
- `MmdcNotFound`
- `NpmNotFound`

This is an intentional API cleanup because those concepts are implementation artifacts of the removed backend.

## Graph Visualization Design

### Public API

Recommended top-level graph types:

```rust
pub enum GraphInputSyntax {
    Auto,
    Expression,
    Dot,
}

pub struct GraphExpression {
    // programmatic IR for the lightweight expression syntax
}

pub struct GraphDiagram {
    source: GraphSource,
    title: Option<String>,
    orientation: GraphOrientation,
}

pub enum GraphSource {
    Expression(GraphExpression),
    Dot(String),
}
```

Recommended constructors:

```rust
impl GraphDiagram {
    pub fn from_expression(source: impl Into<String>) -> Result<Self, GraphError>;
    pub fn from_dot(source: impl Into<String>) -> Result<Self, GraphError>;
    pub fn parse(source: impl Into<String>, syntax: GraphInputSyntax) -> Result<Self, GraphError>;
    pub fn with_title(self, title: impl Into<String>) -> Self;
    pub fn with_orientation(self, orientation: GraphOrientation) -> Self;
    pub fn render(&self, request: &RenderRequest) -> Result<RenderedArtifact, GraphError>;
}
```

`layout-rs` currently exposes the two graph directions that are practical to support here:

- `LeftToRight`
- `TopToBottom`

### Programmatic builder

For library consumers, also expose a builder so callers do not have to serialize to text:

```rust
pub struct GraphBuilder { /* ... */ }

impl GraphBuilder {
    pub fn directed() -> Self;
    pub fn undirected() -> Self;
    pub fn with_orientation(&mut self, orientation: GraphOrientation) -> &mut Self;
    pub fn add_node(&mut self, id: impl Into<String>, label: Option<String>) -> &mut Self;
    pub fn add_edge(&mut self, from: impl Into<String>, to: impl Into<String>) -> &mut Self;
    pub fn build(&self) -> Result<GraphDiagram, GraphError>;
}
```

The builder should stay intentionally modest in v1:

- node id
- optional label
- directed vs undirected edges
- orientation

More advanced DOT styling can stay in the DOT path for now.

### Native expression syntax

The lightweight expression syntax is for CLI convenience, not for full DOT replacement.

Recommended v1 grammar:

- node identifiers: bare words or quoted strings
- directed edge: `->`
- undirected edge: `--`
- chain support: `a -> b -> c`
- statement separators: `;` or newline

Examples:

```txt
a -> b -> c
service-a -> queue; queue -> worker
"My App" -- "Postgres"
```

This is enough to satisfy `bt graph-expression <exp>` without forcing users to type full DOT for simple cases.

### DOT support

DOT input should use `layout-rs` directly:

- parse with `layout::gv::DotParser`
- build the renderable graph with `GraphBuilder::build_visual_graph`
- render to SVG with `SVGWriter`

The implementation should treat DOT as a first-class path, not as something converted into the native expression syntax.

### DOT support policy

The research notes important `layout-rs` limitations. v1 should document and enforce them clearly:

- nested subgraphs/clusters are not guaranteed
- HTML labels are not supported
- unsupported constructs should raise a structured `GraphError`, not silently degrade

That is better than pretending to support full Graphviz semantics.

## Terminal Integration in `biscuit-terminal`

### New wrapper type

Add a new terminal-facing wrapper:

```rust
pub struct GraphExpressionRenderer {
    graph: biscuit_visualized::graph::GraphDiagram,
    transparent_background: bool,
    scale: u32,
}
```

Recommended API:

```rust
impl GraphExpressionRenderer {
    pub fn parse(source: impl Into<String>, syntax: GraphInputSyntax) -> Result<Self, GraphRenderError>;
    pub fn for_terminal(source: impl Into<String>, syntax: GraphInputSyntax) -> Result<Self, GraphRenderError>;
    pub fn with_title(self, title: impl Into<String>) -> Self;
    pub fn with_orientation(self, orientation: GraphOrientation) -> Self;
    pub fn with_transparent_background(self, transparent: bool) -> Self;
    pub fn with_scale(self, scale: u32) -> Self;
    pub fn render_to_cached_png(&self) -> Result<(PathBuf, bool), GraphRenderError>;
    pub fn render_for_terminal(&self) -> Result<(), GraphRenderError>;
    pub fn fallback_code_block(&self) -> String;
}
```

### Shared CLI display helper

`biscuit-terminal/cli/src/commands.rs` should stop having Mermaid-specific image-display logic only.

Replace `display_mermaid_diagram(...)` with a generic helper that accepts:

- a `RenderedArtifact`
- width/layout arguments
- metadata flag

This reduces duplication immediately and lets Mermaid and graph-expression share:

- width parsing
- margin handling
- image output
- `--meta` JSON reporting

## CLI Design

### Command name

Follow the spec and standardize on:

```sh
bt graph-expression <exp>
```

The `biscuit-visualized/README.md` reference to `bt graph-structure --example` should be treated as stale and corrected during implementation.

### Proposed subcommand shape

```rust
GraphExpression {
    #[arg(long, short = 'e')]
    example: bool,

    #[arg(long, default_value = "auto", value_enum)]
    syntax: GraphInputSyntaxArg,

    #[arg(long, short = 't')]
    title: Option<String>,

    #[arg(long, short = 'w')]
    width: Option<String>,

    #[arg(long)]
    inverse: bool,

    #[arg(long, value_enum, default_value = "top-to-bottom")]
    orientation: GraphOrientationArg,

    #[command(flatten)]
    layout: LayoutArgs,

    #[arg(long)]
    meta: bool,

    #[arg(value_name = "EXP", required_unless_present = "example")]
    content: Vec<String>,
}
```

### Syntax behavior

`--syntax auto` rules:

1. If the first non-whitespace token is `graph` or `digraph`, parse as DOT.
2. If the joined content contains `{` and `}`, parse as DOT.
3. Otherwise parse as native expression syntax.

This keeps the simple form ergonomic while still allowing raw DOT input without a separate command.

### Example behavior

`--example` should use the native expression syntax:

```txt
Start -> Validate -> Render
Validate -> Retry
```

The help text should also show a DOT example:

```sh
bt graph-expression --syntax dot 'digraph { A -> B; B -> C; }'
```

### Output behavior

- default output: render PNG in terminal
- `--json`: print normalized command metadata and source instead of rendering
- `--meta`: emit render metadata to stderr, same pattern as existing Mermaid commands
- fallback: on terminals without image support, print a fenced code block using either `dot` or `graph-expression` as the info string

## Darkmatter Integration Direction

This feature should deliberately set up the next step for `darkmatter`.

Recommended downstream direction:

1. `darkmatter` depends directly on `biscuit-visualized` for visualization generation.
2. `darkmatter` uses SVG directly for HTML output.
3. `darkmatter` maps fenced code blocks:
   - ` ```mermaid ` -> `biscuit_visualized::mermaid`
   - ` ```dot ` -> `biscuit_visualized::graph`
4. `darkmatter` only uses `biscuit-terminal` when it specifically wants terminal display behavior.

This is the concrete consequence of the no-broad-re-export decision.

## Dependency Changes

### `biscuit-visualized/src/Cargo.toml`

Add:

- `biscuit-hash`
- `thiserror`
- `tempfile`
- `resvg`
- `mermaid-rs-renderer`
- `layout-rs`

Optional if required by the chosen rasterization path:

- `usvg`
- `tiny-skia`

### `biscuit-terminal/lib/Cargo.toml`

Add:

- path dependency on `biscuit-visualized`

Potential removals from the Mermaid move:

- `biscuit-hash` if nothing else uses it
- `which` if Mermaid was the only consumer

The exact removals should be done after code movement, not assumed up front.

## Migration Plan

### Phase 1. Build `biscuit-visualized`

1. Replace placeholder crate contents with real module structure.
2. Implement shared artifact and cache layers.
3. Implement Mermaid rendering and tests.
4. Implement graph expression parsing, DOT rendering, and tests.

### Phase 2. Rewire `biscuit-terminal`

1. Replace internal Mermaid implementation with thin wrappers over `biscuit-visualized`.
2. Move cache ownership out of `biscuit-terminal`.
3. Add `GraphExpressionRenderer`.
4. Generalize CLI image display helper.
5. Add `bt graph-expression`.

### Phase 3. Clean up docs and downstreams

1. Update `biscuit-terminal` READMEs and docs to remove `mmdc`/npm references.
2. Update `biscuit-terminal/docs/dependencies.md`.
3. Update `biscuit-visualized/README.md` examples and command naming.
4. Update `darkmatter` wrappers to the new backend-neutral model.

## Testing Strategy

### `biscuit-visualized`

Unit tests:

- Mermaid cache key stability
- graph cache key stability
- expression parser success/failure cases
- DOT parser error mapping
- SVG generation returns non-empty artifacts
- PNG rasterization returns readable PNG files

Golden-style tests:

- snapshot normalized SVG headers or structural markers
- snapshot CLI example normalized source text

### `biscuit-terminal`

Unit tests:

- terminal wrapper defaults
- fallback code-block formatting
- `for_terminal()` color/background defaults

CLI integration tests:

- `bt graph-expression --example --json`
- `bt graph-expression --syntax dot --json ...`
- existing Mermaid JSON flows still work

Important improvement over the current state:

- render tests no longer depend on `mmdc`, npm, or Chromium being installed

## Risks and Mitigations

### 1. `layout-rs` DOT support is not full Graphviz

Mitigation:

- document the supported subset explicitly
- validate known unsupported constructs early
- keep the native expression syntax intentionally simple

### 2. Mermaid output fidelity may differ from `mmdc`

Mitigation:

- snapshot a representative set of diagrams already supported by `bt`
- prioritize correctness for the currently exposed command set:
  - flowchart
  - quadrant
  - pie
  - git-graph
  - bar/line chart
  - timeline
  - state diagram
  - ERD

### 3. Removing `mmdc`-specific types is a public API break

Mitigation:

- keep terminal-facing wrapper names stable where possible
- update in-repo consumers in the same change
- document the API cleanup in the changelog/README updates

## Final Recommendation

Implement this feature around a strict ownership split:

- `biscuit-visualized` owns visualization generation, artifact caching, and format conversion
- `biscuit-terminal` owns terminal display and terminal-oriented wrappers only

And take the re-export question to a concrete answer:

- do **not** broadly re-export `biscuit-visualized` from `biscuit-terminal`
- keep only thin terminal adapters in `biscuit-terminal`
- move cross-surface consumers such as `darkmatter` to `biscuit-visualized` directly

That gives the repo the cleanest long-term shape and avoids recreating the same layering problem under a new crate name.
