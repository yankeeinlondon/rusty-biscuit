# Mermaid Diagrams

The `bt` CLI and `MermaidRenderer` library render Mermaid diagrams inline in terminals using the mmdc CLI.

`bt` does not auto-fallback to code blocks on render errors; it reports the error and exits non-zero. If you need textual fallback, use `--json` or call `fallback_code_block()` in your own app flow.

## Requirements

```bash
# Install mermaid-cli globally (recommended)
npm install -g @mermaid-js/mermaid-cli

# Falls back to npx if not installed
```

Minimum recommended version: **10.6.0** (for improved SVG rendering and icon support)

## CLI Diagram Commands

All diagram commands share common options:

| Option | Description |
|--------|-------------|
| `--example` / `-e` | Render example with command shown |
| `--width` / `-w` | Width: `50%`, `80ch`, `80`, or `fill` |
| `--inverse` | Solid background with inverted colors |
| `--title` / `-t` | Add title above diagram |
| `--json` | Output as JSON for scripting |

### Flowchart

```bash
bt flowchart "A --> B --> C"
bt flowchart --vertical "Start --> Process --> End"
bt flowchart "A[Input] --> B{Decision}" "B -->|Yes| C[Output]"
```

Options:
- `--vertical`: Render top-down instead of left-right

### Quadrant Chart

```bash
bt quadrant "Item A: [0.3, 0.6]" "Item B: [0.7, 0.4]"
bt quadrant --x-axis "Low --> High" --y-axis "Small --> Large" "Item: [0.5, 0.5]"
bt quadrant --theme magic-quadrangle "Leaders: [0.8, 0.8]" "Niche: [0.2, 0.2]"
```

Data points: `"Label: [x, y]"` where x,y are 0.0-1.0

Options:
- `-x`/`--x-axis`, `-y`/`--y-axis`: Axis labels
- `--top-left`, `--top-right`, `--bottom-left`, `--bottom-right`: Quadrant labels
- `--theme`: `default` or `magic-quadrangle`
- `--point-radius`, `--label-size`: Point styling
- `--q1-fill` through `--q4-fill`: Custom quadrant colors (hex)

Inline point styling:
```bash
bt quadrant "Item: [0.5, 0.5] color: #ff3300, radius: 10"
```

### Pie Chart

```bash
bt pie-chart "Dogs: 386" "Cats: 85" "Birds: 15"
bt pie-chart --show-data "Dogs: 386" "Cats: 85"
bt pie-chart "TypeScript: 45 #3178c6" "Rust: 35 #dea584"
```

Input formats:
- Simple: `"Label: value"`
- Semicolon-delimited: `"Label1: 10; Label2: 20"`
- With color: `"Label: value #rrggbb"` or `"Label: value color: #rrggbb"`

Options:
- `--show-data`: Show percentages on slices

### Git Graph

```bash
bt git-graph "commit" "branch feature" "checkout feature" "commit" "merge feature"
bt git-graph "commit id: \"abc123\"" "commit tag: \"v1.0\""
```

Commands: `commit`, `branch <name>`, `checkout <name>`, `merge <name>`, `cherry-pick id: "..."`

### Bar Chart

```bash
bt bar-chart 10 20 15 25
bt bar-chart --x-axis "Q1,Q2,Q3,Q4" --y-axis Sales 10 20 15 25
bt bar-chart --horizontal --show-data-label 1 8 7 5
bt bar-chart --line 10 20 15 25  # Add trend line
bt bar-chart --aspect-ratio 2.0 --width 60% --inverse 10 20 15 25
```

Input formats: JSON array `"[1,8,7]"`, comma-separated `"1,8,7"`, or space-separated `1 8 7`

Options:
- `--horizontal`: Horizontal bars
- `--show-data-label`: Show values on bars
- `--line`: Also render as line
- `--aspect-ratio`: Width/height ratio (default: 1.5)

### Line Chart

```bash
bt line-chart 1 8 7 5 9 3
bt line-chart --x-axis "Mon,Tue,Wed" --y-axis Temperature 20 22 19
bt line-chart --bar 1 8 7 5  # Add bars under line
bt line-chart --show-data-label --horizontal 1 8 7 5
bt line-chart --aspect-ratio 1.8 --inverse --width 60% 1 8 7 5
```

Same options as bar-chart, plus `--bar` to add bars.

### Timeline

```bash
bt timeline "2020: Project started" "2021: First release" "2022: Major update"
bt timeline --section "Early Years" "2020: Founded" --section "Growth" "2022: Series A"
```

Format: `"YYYY: Description"`

Options:
- `-s`/`--section`: Group events into sections

### State Diagram

```bash
bt state-diagram "[*] --> Idle" "Idle --> Running" "Running --> [*]"
bt state-diagram "[*] --> Idle" "Idle --> Running: start" "Running --> Stopped: stop"
```

Syntax:
- `[*]`: Start/end state
- `State1 --> State2`: Transition
- `State1 --> State2: label`: Labeled transition

### Entity Relationship Diagram

```bash
bt erd "Customer ||--o{ Order : places" "Order ||--|{ LineItem : contains"
bt erd --entity "Customer { id int PK, name string }" "Customer ||--o{ Order : places"
```

Relationships:
- `||--||`: One to one
- `||--o{`: One to many
- `}o--o{`: Many to many
- `||--o|`: One to zero or one

Options:
- `-E`/`--entity`: Entity definition with attributes

## Library Usage

### Basic Rendering

```rust
use biscuit_terminal::components::mermaid::MermaidRenderer;

let renderer = MermaidRenderer::new("flowchart LR\n    A --> B");
if let Err(err) = renderer.render_for_terminal() {
    eprintln!("Render failed: {err}");
    // Optional app-level fallback:
    println!("{}", renderer.fallback_code_block());
}
```

### Terminal-Aware Rendering

```rust
use biscuit_terminal::components::mermaid::MermaidRenderer;

// Auto-detects color mode and uses appropriate theme
let renderer = MermaidRenderer::for_terminal("flowchart LR\n    A --> B");
renderer.render_for_terminal()?;
```

### Configuration

```rust
use biscuit_terminal::components::mermaid::{MermaidRenderer, MermaidTheme, MermaidConfig};

let renderer = MermaidRenderer::new("flowchart LR\n    A --> B")
    .with_theme(MermaidTheme::Dark)
    .with_scale(3)  // Higher resolution
    .with_transparent_background(true);

// Quadrant chart config
let config = MermaidConfig::new()
    .with_point_label_font_size(18)
    .with_point_radius(10)
    .with_quadrant_fill(1, "#1e2a1e");

let renderer = MermaidRenderer::new(diagram).with_config(config);
```

### Themes

| Theme | Description |
|-------|-------------|
| `MermaidTheme::Dark` | Light text on dark background |
| `MermaidTheme::Default` | Dark text on light background |
| `MermaidTheme::Forest` | Green tones |
| `MermaidTheme::Neutral` | Grayscale |

```rust
use biscuit_terminal::components::mermaid::MermaidTheme;
use biscuit_terminal::terminal::Terminal;

let theme = MermaidTheme::for_color_mode(Terminal::color_mode());
let inverse_theme = theme.inverse();  // For solid backgrounds
```

### Quadrant Themes

```rust
use biscuit_terminal::components::mermaid::{QuadrantTheme, MermaidConfig};
use biscuit_terminal::terminal::Terminal;

let config = QuadrantTheme::MagicQuadrangle.apply(MermaidConfig::new(), Terminal::color_mode());
```

| Theme | Description |
|-------|-------------|
| `Default` | Standard Mermaid colors |
| `MagicQuadrangle` | Gartner-style: green top-right, red bottom-left |

### Version Detection

```rust
use biscuit_terminal::components::mermaid::{detect_mmdc_version, MmdcVersion};

if let Some(version) = detect_mmdc_version() {
    if !version.meets_minimum() {
        eprintln!("Warning: mmdc {} is older than recommended {}", version, MmdcVersion::minimum());
    }
}
```

## Caching

Mermaid renders are cached to avoid redundant CLI calls. The cache key is an xxHash derived from content + configuration (`biscuit-hash`).

```rust
use biscuit_terminal::components::mermaid_cache::MermaidCache;

let cache = MermaidCache::new();
// Renders are automatically cached when using MermaidRenderer
```

## Security

- **Size limit**: Diagrams over 10KB are rejected
- **Terminal check**: Only renders when image protocols are supported
- **No automatic CLI fallback**: failures are returned as errors; use `fallback_code_block()` manually if desired

## Icon Support

Diagrams support icons via Mermaid's icon packs:

```
A[icon:fa7-brands:github] --> B[icon:lucide:star]
```

Available packs:
- `@iconify-json/fa7-brands`: Font Awesome 7 brands
- `@iconify-json/lucide`: Lucide icons
- `@iconify-json/carbon`: Carbon Design icons
- `@iconify-json/system-uicons`: System UI icons

## Related

- [bt Command](./cli.md) - CLI reference
- [Image Rendering](./image-rendering.md) - Image protocol details
