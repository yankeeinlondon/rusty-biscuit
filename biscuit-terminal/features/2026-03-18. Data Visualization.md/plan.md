# Data Visualization — Implementation Plan

This plan implements the spec and tech design for the `biscuit-visualized` crate and the corresponding changes to `biscuit-terminal`.

## Current State Summary

- **`biscuit-visualized/src/`**: Placeholder crate with an `add()` function, empty `[dependencies]`, edition `"2024"`, registered in workspace as `biscuit-visualized/src`
- **`biscuit-terminal/lib/src/components/mermaid.rs`**: 2,136-line MermaidRenderer with mmdc/npx/Chromium detection, 10KB limit, MmdcVersion, theme system, cache-aware rendering
- **`biscuit-terminal/lib/src/components/mermaid_cache.rs`**: 760-line MermaidCache with xxHash-based file caching in `$TMPDIR/mermaid-cache/`
- **`biscuit-terminal/cli/src/args.rs`**: 9 diagram subcommands (Image, Flowchart, Quadrant, PieChart, GitGraph, BarChart, LineChart, Timeline, StateDiagram, Erd)
- **`biscuit-terminal/cli/src/commands.rs`**: `display_mermaid_diagram()` helper used by all diagram commands, Mermaid-specific error handling

---

## Phase 1: Build `biscuit-visualized` Core

**Goal**: Replace the placeholder crate with the shared artifact, cache, and raster layers that both Mermaid and graph rendering will use.

### 1.1 Fix crate scaffold and dependencies

**File**: `biscuit-visualized/src/Cargo.toml`

- Fix `edition = "2024"` → `"2021"` (2024 is not yet stabilized)
- Add `[lib]` section: `name = "biscuit_visualized"`, `path = "src/lib.rs"`
- Add dependencies:
  - `biscuit-hash = { path = "../../biscuit-hash/lib" }`
  - `thiserror = "2.0"`
  - `tempfile = "3"`
  - `resvg = "0.45"`
  - `serde = { version = "1.0", features = ["derive"] }`
  - `serde_json = "1.0"`
  - `tracing = "0.1"`
- **Do not** add `mermaid-rs-renderer` or `layout-rs` yet — those are added in their respective phases

### 1.2 Create module structure

Replace `biscuit-visualized/src/src/lib.rs` placeholder with:

```
biscuit-visualized/src/src/
├── lib.rs
├── artifact.rs
├── cache/
│   ├── mod.rs
│   └── file_cache.rs
└── raster/
    ├── mod.rs
    └── png.rs
```

**`lib.rs`** — re-export public modules:

```rust
pub mod artifact;
pub mod cache;
pub mod raster;
```

### 1.3 Implement `artifact.rs` — shared types

Per the tech design:

```rust
pub enum OutputFormat { Svg, Png }

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

Add `Default` for `RenderRequest` (Png, scale=2, opaque).

### 1.4 Implement `cache/` — generalized file cache

Port and generalize logic from `biscuit-terminal/lib/src/components/mermaid_cache.rs`.

**`cache/mod.rs`**:

```rust
pub mod file_cache;
pub use file_cache::FileCache;

pub enum VisualizationKind { Mermaid, Graph }
```

**`cache/file_cache.rs`**:

- `FileCache` struct with methods: `new()`, `get()`, `store()`, `clear()`, `size_bytes()`, `entry_count()`
- Cache directory: `$TMPDIR/biscuit-visualized/v1/{mermaid,graph}/{svg,png}/`
- Cache key format: `v1|kind|syntax|source|options-json|backend-id|format`
- Hash with `biscuit_hash::xx_hash` (same as current cache)
- Backend identifiers as constants: `MERMAID_BACKEND = "mermaid-rs-renderer@0.2.x"`, `GRAPH_BACKEND = "layout-rs@0.1.x"`, `RASTERIZER = "resvg@0.45"`

### 1.5 Implement `raster/` — SVG-to-PNG conversion

**`raster/png.rs`**:

- `rasterize_svg(svg_path: &Path, png_path: &Path, scale: u32) -> Result<(), RasterError>`
- Uses `resvg` (already a dependency of `biscuit-terminal`)
- `RasterError` enum: `SvgParseFailed`, `RenderFailed`, `IoError`

### 1.6 Tests for Phase 1

- Cache key stability (same inputs → same hash, different inputs → different hash)
- Cache round-trip (store + get returns the stored file)
- Cache directory layout matches spec
- `RenderRequest::default()` returns expected values
- Raster module compiles (rendering tests come with Mermaid/graph phases)

---

## Phase 2: Mermaid Rendering in `biscuit-visualized`

**Goal**: Implement Mermaid diagram generation using `mermaid-rs-renderer`, replacing the mmdc pipeline.

### 2.1 Add `mermaid-rs-renderer` dependency

**File**: `biscuit-visualized/src/Cargo.toml`

- Add `mermaid-rs-renderer = "0.2"` (verify exact crate name on crates.io first — the tech design uses this name but the research doc refers to it as a Rust crate with 2.7K downloads)

### 2.2 Create mermaid module

```
biscuit-visualized/src/src/mermaid/
├── mod.rs
├── config.rs
├── error.rs
└── render.rs
```

### 2.3 Implement `mermaid/error.rs`

```rust
#[derive(Debug, thiserror::Error)]
pub enum MermaidError {
    #[error("Mermaid parse/render failure: {0}")]
    RenderFailed(String),
    #[error("SVG rasterization failed: {0}")]
    RasterizationFailed(#[from] crate::raster::RasterError),
    #[error("I/O error: {0}")]
    Io(#[from] std::io::Error),
}
```

No mmdc-specific errors. No content size limit.

### 2.4 Implement `mermaid/config.rs`

Move these types from `biscuit-terminal/lib/src/components/mermaid.rs`:

- `MermaidTheme` enum (Dark, Default, Forest, Neutral) with `as_str()`, `for_color_mode()`, `inverse()`
- `MermaidConfig` struct (quadrant fills, point sizes, JSON serialization)
- `QuadrantTheme` enum and its `apply()` method

These are visualization concerns, not terminal concerns, so they belong here.

### 2.5 Implement `mermaid/render.rs` — `MermaidDiagram`

Per the tech design:

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

The `render()` method:

1. Build cache key from instructions + theme + config + request params + `MERMAID_BACKEND`
2. Check cache → return `RenderedArtifact { cache_hit: true, ... }` on hit
3. On miss: call `mermaid-rs-renderer` to produce SVG
4. If PNG requested: rasterize SVG via `raster::png::rasterize_svg()`
5. Store result in cache
6. Return `RenderedArtifact`

### 2.6 Update `lib.rs`

```rust
pub mod artifact;
pub mod cache;
pub mod raster;
pub mod mermaid;
```

### 2.7 Tests for Phase 2

- `MermaidDiagram::new("graph LR; A-->B").render(&RenderRequest::default())` produces a non-empty PNG
- `MermaidDiagram` with same inputs returns `cache_hit: true` on second call
- `fallback_code_block()` wraps instructions in a fenced code block
- Theme/config builder methods are chainable
- Invalid Mermaid syntax produces `MermaidError::RenderFailed`
- SVG output format works (if format is `OutputFormat::Svg`)

---

## Phase 3: Graph Visualization in `biscuit-visualized`

**Goal**: Add graph expression parsing, DOT rendering, and the programmatic builder.

### 3.1 Add `layout-rs` dependency

**File**: `biscuit-visualized/src/Cargo.toml`

- Add `layout-rs = "0.1"` (verify exact crate name — the research doc confirms `layout-rs` v0.1.3)

### 3.2 Create graph module

```
biscuit-visualized/src/src/graph/
├── mod.rs
├── builder.rs
├── error.rs
├── expression.rs
├── dot.rs
└── render.rs
```

### 3.3 Implement `graph/error.rs`

```rust
#[derive(Debug, thiserror::Error)]
pub enum GraphError {
    #[error("Expression parse error: {0}")]
    ExpressionParseFailed(String),
    #[error("DOT parse error: {0}")]
    DotParseFailed(String),
    #[error("Unsupported DOT feature: {0}")]
    UnsupportedDotFeature(String),
    #[error("Graph rendering failed: {0}")]
    RenderFailed(String),
    #[error("SVG rasterization failed: {0}")]
    RasterizationFailed(#[from] crate::raster::RasterError),
    #[error("I/O error: {0}")]
    Io(#[from] std::io::Error),
}
```

### 3.4 Implement `graph/expression.rs` — native expression parser

Parse the lightweight expression syntax:

- Node identifiers: bare words or `"quoted strings"`
- Directed edge: `->`
- Undirected edge: `--`
- Chain support: `a -> b -> c`
- Statement separators: `;` or newline

Output: a `GraphExpression` IR struct containing nodes and edges.

### 3.5 Implement `graph/dot.rs` — DOT rendering

- Parse DOT input using `layout::gv::DotParser`
- Build visual graph with `GraphBuilder::build_visual_graph`
- Render SVG with `SVGWriter`
- Validate for known unsupported constructs (nested subgraphs, HTML labels) and return `GraphError::UnsupportedDotFeature`

### 3.6 Implement `graph/builder.rs` — programmatic API

```rust
pub struct GraphBuilder { /* ... */ }

impl GraphBuilder {
    pub fn directed() -> Self;
    pub fn undirected() -> Self;
    pub fn add_node(&mut self, id: impl Into<String>, label: Option<String>) -> &mut Self;
    pub fn add_edge(&mut self, from: impl Into<String>, to: impl Into<String>) -> &mut Self;
    pub fn build(self) -> GraphDiagram;
}
```

The builder generates DOT internally and delegates to the DOT rendering path.

### 3.7 Implement `graph/render.rs` — `GraphDiagram`

```rust
pub enum GraphInputSyntax { Auto, Expression, Dot }
pub enum GraphOrientation { LeftRight, TopBottom, BottomTop, RightLeft }

pub enum GraphSource {
    Expression(GraphExpression),
    Dot(String),
}

pub struct GraphDiagram {
    source: GraphSource,
    title: Option<String>,
    orientation: GraphOrientation,
}

impl GraphDiagram {
    pub fn from_expression(source: impl Into<String>) -> Result<Self, GraphError>;
    pub fn from_dot(source: impl Into<String>) -> Result<Self, GraphError>;
    pub fn parse(source: impl Into<String>, syntax: GraphInputSyntax) -> Result<Self, GraphError>;
    pub fn with_title(self, title: impl Into<String>) -> Self;
    pub fn with_orientation(self, orientation: GraphOrientation) -> Self;
    pub fn render(&self, request: &RenderRequest) -> Result<RenderedArtifact, GraphError>;
}
```

**`Auto` detection** rules (per tech design):

1. If first non-whitespace token is `graph` or `digraph` → DOT
2. If content contains `{` and `}` → DOT
3. Otherwise → native expression syntax

**`render()`** method follows the same pattern as Mermaid: cache check → render SVG → rasterize if PNG → store → return artifact.

### 3.8 Update `lib.rs`

```rust
pub mod artifact;
pub mod cache;
pub mod raster;
pub mod mermaid;
pub mod graph;
```

### 3.9 Tests for Phase 3

**Expression parser**:

- `"a -> b -> c"` produces 3 nodes and 2 directed edges
- `"a -- b"` produces undirected edge
- `"\"My App\" -- Postgres"` handles quoted identifiers
- `"a -> b; b -> c"` handles semicolon separators
- Empty/invalid input produces `GraphError::ExpressionParseFailed`

**DOT rendering**:

- Valid DOT string produces non-empty SVG
- Invalid DOT produces `GraphError::DotParseFailed`
- HTML labels produce `GraphError::UnsupportedDotFeature`

**GraphDiagram**:

- `parse("a -> b", Auto)` → expression path
- `parse("digraph { A -> B }", Auto)` → DOT path
- `render()` produces valid PNG
- Cache hit on second render with same inputs

**GraphBuilder**:

- `GraphBuilder::directed().add_node("a", None).add_edge("a", "b").build()` produces renderable diagram

---

## Phase 4: Rewire `biscuit-terminal` — Mermaid Adapter

**Goal**: Replace the internal Mermaid implementation with thin wrappers over `biscuit-visualized`.

### 4.1 Add `biscuit-visualized` dependency

**File**: `biscuit-terminal/lib/Cargo.toml`

- Add `biscuit-visualized = { path = "../../biscuit-visualized/src" }`

### 4.2 Rewrite `mermaid.rs` as thin adapter

**File**: `biscuit-terminal/lib/src/components/mermaid.rs`

The current 2,136-line file becomes a ~200-line terminal adapter.

**Keep** (as wrappers):

- `MermaidRenderer` struct — delegates to `biscuit_visualized::mermaid::MermaidDiagram`
- `MermaidRenderer::new()`, `for_terminal()`, `with_theme()`, `with_title()`, `with_config()`, `with_scale()`, `with_transparent_background()`
- `render_to_cached_png()` — calls `diagram.render(&request)`, returns `(PathBuf, bool)`
- `render_for_terminal()` — renders + displays via viuer
- `fallback_code_block()`, `print_fallback()`
- `terminal_supports_images()`

**Re-export from `biscuit_visualized`**:

- `MermaidTheme`, `MermaidConfig`, `QuadrantTheme` (these types are used in CLI args)

**Remove**:

- `MMDC_MIN_VERSION`, `MmdcVersion`, `detect_mmdc_version()`
- `MAX_DIAGRAM_SIZE` (10KB limit)
- All `mmdc`/`npx`/Chromium detection logic (~300 lines)
- `MmdcNotFound`, `NpmNotFound` error variants
- `render_to_temp_png()` (mmdc invocation)
- `render_to_file()` (mmdc invocation)
- `ICON_PACKS` constant and icon-pack file writing

**Rewrite `MermaidRenderError`**:

```rust
#[derive(Debug, thiserror::Error)]
pub enum MermaidRenderError {
    #[error(transparent)]
    Visualization(#[from] biscuit_visualized::mermaid::MermaidError),
    #[error("Terminal does not support inline images")]
    NoImageSupport,
    #[error("Image display failed: {0}")]
    DisplayError(String),
}
```

### 4.3 Remove `mermaid_cache.rs`

**File**: `biscuit-terminal/lib/src/components/mermaid_cache.rs` — **delete entirely**

Cache is now owned by `biscuit-visualized`.

**File**: `biscuit-terminal/lib/src/components/mod.rs` — remove `pub mod mermaid_cache;`

### 4.4 Evaluate dependency removals

After the rewrite, check if these are still needed in `biscuit-terminal/lib/Cargo.toml`:

- `which` — was used for mmdc/Chromium detection → **remove** if nothing else uses it
- `biscuit-hash` — was used for cache keys → **remove** if nothing else uses it (cache is now in `biscuit-visualized`)

### 4.5 Tests for Phase 4

- All existing `bt flowchart --json`, `bt pie-chart --json`, etc. CLI integration tests still pass
- `MermaidRenderer::new("graph LR; A-->B").render_to_cached_png()` works
- `MermaidRenderer::for_terminal(...)` sets correct theme for dark/light terminals
- `fallback_code_block()` output unchanged
- No compile errors referencing removed mmdc types

---

## Phase 5: Add `GraphExpressionRenderer` to `biscuit-terminal`

**Goal**: Add the terminal-facing graph wrapper and the `bt graph-expression` CLI command.

### 5.1 Create `graph_expression.rs` component

**File**: `biscuit-terminal/lib/src/components/graph_expression.rs`

```rust
pub struct GraphExpressionRenderer {
    graph: biscuit_visualized::graph::GraphDiagram,
    transparent_background: bool,
    scale: u32,
}

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

**`GraphRenderError`**:

```rust
#[derive(Debug, thiserror::Error)]
pub enum GraphRenderError {
    #[error(transparent)]
    Visualization(#[from] biscuit_visualized::graph::GraphError),
    #[error("Terminal does not support inline images")]
    NoImageSupport,
    #[error("Image display failed: {0}")]
    DisplayError(String),
}
```

Register in `biscuit-terminal/lib/src/components/mod.rs`: `pub mod graph_expression;`

### 5.2 Generalize CLI display helper

**File**: `biscuit-terminal/cli/src/commands.rs`

Extract `display_mermaid_diagram()` into a generic `display_diagram()` that accepts a `RenderedArtifact`:

```rust
pub fn display_diagram(
    artifact: &RenderedArtifact,
    diagram_type: &str,
    width: Option<&str>,
    layout: &LayoutArgs,
    meta: bool,
    render_time_ms: u64,
) -> color_eyre::Result<()> { ... }
```

Then rewrite `display_mermaid_diagram()` to call `display_diagram()` and add a parallel `display_graph_diagram()` that does the same.

All existing Mermaid commands (flowchart, quadrant, pie-chart, etc.) continue to call `display_mermaid_diagram()` — no changes to their call sites needed.

### 5.3 Add `GraphExpression` CLI subcommand

**File**: `biscuit-terminal/cli/src/args.rs`

Add to the `Command` enum:

```rust
/// Render a graph expression as a diagram
#[command(display_order = 11)]
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

    #[arg(long, value_enum, default_value = "left-right")]
    orientation: GraphOrientationArg,

    #[command(flatten)]
    layout: LayoutArgs,

    #[arg(long)]
    meta: bool,

    #[arg(value_name = "EXP", required_unless_present = "example")]
    content: Vec<String>,
}
```

Add `GraphInputSyntaxArg` and `GraphOrientationArg` clap value enums.

### 5.4 Implement `render_graph_expression()` command

**File**: `biscuit-terminal/cli/src/commands.rs`

- Join `content` args with space
- Create `GraphExpressionRenderer::for_terminal(...)` (or `parse(...)` for non-terminal)
- Apply title, orientation, inverse
- Call `display_graph_diagram()`
- `--example` renders: `"Start -> Validate -> Render; Validate -> Retry"`

### 5.5 Wire up in `main.rs`

**File**: `biscuit-terminal/cli/src/main.rs`

Add the `Command::GraphExpression { .. }` match arm calling `render_graph_expression()`.

### 5.6 Tests for Phase 5

- `bt graph-expression --example --json` outputs valid JSON
- `bt graph-expression "a -> b -> c" --json` outputs valid JSON
- `bt graph-expression --syntax dot "digraph { A -> B }" --json` outputs valid JSON
- `bt graph-expression --example --meta` emits metadata to stderr
- Fallback code block on non-image terminal

---

## Phase 6: Documentation and Cleanup

**Goal**: Update all documentation to reflect the new architecture.

### 6.1 Update `biscuit-visualized/README.md`

- Fix stale `bt graph-structure --example` → `bt graph-expression --example`
- Add library usage examples for `MermaidDiagram` and `GraphDiagram`
- Document the expression syntax

### 6.2 Update `biscuit-terminal` docs

- Remove references to mmdc, npm, npx, Chromium from READMEs
- Update `biscuit-terminal/docs/dependencies.md` (add `biscuit-visualized`, note removed deps)
- Update lib README to describe the new adapter-only role for Mermaid

### 6.3 Update root-level docs

- Add `biscuit-visualized` to `docs/dependencies.md`
- Update `CLAUDE.md` monorepo structure to include `biscuit-visualized`

### 6.4 Update `.claude/skills/biscuit-terminal/SKILL.md`

- Add graph-expression to the skill's capability list
- Note the `biscuit-visualized` dependency for rendering

---

## Dependency Summary

### New: `biscuit-visualized/src/Cargo.toml`

| Crate | Version | Purpose |
|-------|---------|---------|
| `biscuit-hash` | path | xxHash for cache keys |
| `thiserror` | 2.0 | Error derive |
| `tempfile` | 3 | Temp file management |
| `resvg` | 0.45 | SVG rasterization |
| `serde` | 1.0 | Serialization (cache keys) |
| `serde_json` | 1.0 | JSON config serialization |
| `tracing` | 0.1 | Structured logging |
| `mermaid-rs-renderer` | 0.2 | Mermaid rendering (Phase 2) |
| `layout-rs` | 0.1 | DOT/graph rendering (Phase 3) |

### Changed: `biscuit-terminal/lib/Cargo.toml`

| Change | Crate | Reason |
|--------|-------|--------|
| **Add** | `biscuit-visualized` (path) | Core visualization dependency |
| **Remove** (if unused) | `which` | Was for mmdc/Chromium detection |
| **Remove** (if unused) | `biscuit-hash` | Was for cache keys (now in biscuit-visualized) |

---

## Risk Checklist

| Risk | Mitigation | Phase |
|------|-----------|-------|
| `mermaid-rs-renderer` crate name/API differs from expected | Verify on crates.io before Phase 2; adapt API wrappers | 2.1 |
| `layout-rs` DOT coverage gaps | Document supported subset; validate unsupported constructs early with structured errors | 3.5 |
| Mermaid output fidelity differs from mmdc | Snapshot current mmdc outputs for key diagram types; compare after migration | 2.7 |
| Removing mmdc types is a public API break | All in-repo consumers updated in same change; no external consumers | 4.2 |
| `resvg` duplication (both crates depend on it) | Workspace-level dependency dedup via `[workspace.dependencies]` or just let Cargo deduplicate | 1.1 |

---

## Execution Order

```
Phase 1 (core layers) ──→ Phase 2 (mermaid) ──→ Phase 4 (rewire terminal mermaid)
                      └──→ Phase 3 (graph)   ──→ Phase 5 (terminal graph + CLI)
                                                       └──→ Phase 6 (docs)
```

Phases 2 and 3 are independent and can be parallelized. Phase 4 depends on Phase 2. Phase 5 depends on Phases 3 and 4 (needs the generalized display helper from 4). Phase 6 is last.
