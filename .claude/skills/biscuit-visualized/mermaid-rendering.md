# Mermaid Rendering

## MermaidDiagram API

The main entry point for rendering Mermaid diagrams to SVG or PNG.

### Construction

```rust
use biscuit_visualized::mermaid::{MermaidDiagram, MermaidTheme, MermaidConfig};

let diagram = MermaidDiagram::new("flowchart LR\n    A --> B")
    .with_theme(MermaidTheme::Dark)
    .with_title("My Diagram")
    .with_config(config);
```

### Rendering

```rust
use biscuit_visualized::artifact::{RenderRequest, OutputFormat};

let req = RenderRequest::new(OutputFormat::Svg);
let artifact = diagram.render(req)?;

// artifact.path: PathBuf to the rendered file (may be in cache)
// artifact.format: OutputFormat::Svg
// artifact.cache_hit: bool
// artifact.alt_text: Option<String>
```

### Fallback

When rendering fails or the consumer doesn't support images:

```rust
let code_block = diagram.fallback_code_block();
// Returns: ```mermaid\n<source>\n```
```

## MermaidTheme

| Variant | Description | Best For |
|---------|-------------|----------|
| `Dark` | Dark background, light elements | Dark terminal themes |
| `Default` | Standard Mermaid light theme | Light terminals |
| `Forest` | Green tones | Nature-themed output |
| `Neutral` | Muted, low-contrast | Formal/documentation |

### Adaptive Theme Selection

```rust
// Pick theme based on detected color mode
let theme = MermaidTheme::for_color_mode(is_dark);
// is_dark=true  → Dark
// is_dark=false → Default

// Get the opposite theme
let inverted = theme.inverse();
```

### Theme Variables via Init Directives

Mermaid supports inline theme customization through `%%{init: {...}}%%` directives. These are passed through as-is:

```
%%{init: {'theme': 'dark', 'themeVariables': {'primaryColor': '#ff0000'}}}%%
flowchart LR
    A --> B
```

The renderer detects existing init directives and avoids adding conflicting theme configuration.

## MermaidConfig (Quadrant Charts)

For fine-grained quadrant chart customization:

```rust
use biscuit_visualized::mermaid::MermaidConfig;

let config = MermaidConfig::default()
    .with_point_label_font_size(14)
    .with_point_radius(8)
    .with_quadrant_fill(1, "#ff0000")   // Top-right
    .with_quadrant_fill(2, "#00ff00")   // Top-left
    .with_quadrant_fill(3, "#0000ff")   // Bottom-left
    .with_quadrant_fill(4, "#ffff00");  // Bottom-right
```

### QuadrantTheme Presets

| Preset | Description |
|--------|-------------|
| `Default` | No customization, uses Mermaid defaults |
| `MagicQuadrangle` | Gartner Magic Quadrant-inspired colors and sizing |

```rust
use biscuit_visualized::mermaid::{QuadrantTheme, MermaidConfig};

// Apply a preset theme to a config, adapting to dark/light mode
let config = QuadrantTheme::MagicQuadrangle.apply(MermaidConfig::default(), true);

// Parse from string (for CLI args)
let theme = QuadrantTheme::parse("magic-quadrangle");
```

## Supported Diagram Types

All standard Mermaid diagram types are supported:

- **Flowchart** (`flowchart LR/TB/RL/BT`)
- **Sequence** (`sequenceDiagram`)
- **Class** (`classDiagram`)
- **State** (`stateDiagram-v2`)
- **ER** (`erDiagram`)
- **Pie** (`pie`)
- **XY Chart** (`xychart-beta`)
- **Quadrant** (`quadrant-beta`)
- **Gantt** (`gantt`)
- **Timeline** (`timeline`)
- **Journey** (`journey`)
- **Mindmap** (`mindmap`)
- **Git Graph** (`gitGraph`)

## SVG Post-Processing

The renderer applies fixes to the raw Mermaid SVG output:

1. **Legend text alignment**: Adds `dominant-baseline="central"` to fix vertical centering
2. **Pie chart contrast**: Adjusts text color per slice based on WCAG luminance calculation — dark text on light slices, light text on dark slices
3. **Init directive handling**: Detects existing `%%{init:...}%%` blocks and avoids adding conflicting theme/config directives

## MermaidError

```rust
pub enum MermaidError {
    /// Mermaid rendering failed (invalid syntax, unsupported features)
    RenderFailed(String),
    /// SVG-to-PNG rasterization failed
    RasterizationFailed(RasterError),
    /// File I/O error during caching or temp file operations
    Io(#[from] std::io::Error),
}
```

## Source Files

| File | Contents |
|------|----------|
| `biscuit-visualized/src/mermaid/mod.rs` | Module re-exports |
| `biscuit-visualized/src/mermaid/render.rs` | `MermaidDiagram`, SVG post-processing |
| `biscuit-visualized/src/mermaid/config.rs` | `MermaidTheme`, `MermaidConfig`, `QuadrantTheme` |
| `biscuit-visualized/src/mermaid/error.rs` | `MermaidError` |
