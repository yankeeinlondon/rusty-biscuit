# MermaidDiagram

Renders Mermaid diagrams inline in the terminal as raster images. Delegates diagram rendering to `biscuit-visualized` and displays via the terminal's image protocol (Kitty or iTerm2). Falls back to a code block on terminals without image support.

Supports all Mermaid diagram types: flowcharts, sequence diagrams, class diagrams, state diagrams, ERD, Gantt charts, pie charts, quadrant charts, git graphs, timelines, bar charts, and line charts.

## Programmatic Use

```rust
use biscuit_terminal::prelude::*;

// Basic usage
let diagram = MermaidDiagram::new("flowchart LR\n    A --> B --> C");
let term = Terminal::new();

// Infallible render (via Renderable trait)
println!("{}", diagram.display(&term));

// Fallible render with metadata
match diagram.try_render(&term) {
    Ok(result) => {
        print!("{}", result.output);
        eprintln!("Rendered in {}ms", result.render_time_ms);
    }
    Err(e) => eprintln!("Render failed: {}", e),
}

// With theme and config
let diagram = MermaidDiagram::new("pie\n    \"A\" : 40\n    \"B\" : 60")
    .with_theme(MermaidTheme::Forest)
    .with_title("Distribution")
    .with_config(MermaidConfig::new().with_point_label_font_size(16));
```

### Key API

| Method | Description |
|--------|-------------|
| `MermaidDiagram::new(source)` | Create from Mermaid source text |
| `.with_theme(MermaidTheme)` | Set rendering theme (Dark, Forest, etc.) |
| `.with_title(str)` | Add a diagram title |
| `.with_config(MermaidConfig)` | Customize rendering config |
| `.try_render(&Terminal)` | Fallible render returning `MermaidRenderResult` |
| `.render(&Terminal)` | Infallible render (falls back to code block) |

### Error Handling

Returns `MermaidRenderError` for visualization errors, unsupported terminals, and image display failures.

## CLI

Exposed via multiple `bt` subcommands, one per diagram type:

```bash
bt flowchart "A --> B --> C"
bt pie-chart "A:40" "B:60"
bt quadrant --title "Priority" --x-axis "Effort" --y-axis "Impact"
bt git-graph
bt bar-chart "Q1:100" "Q2:150"
bt line-chart "Jan:10" "Feb:20"
bt timeline "2024:Launch" "2025:Scale"
bt state-diagram "s1 --> s2"
bt erd "CUSTOMER ||--o{ ORDER : places"
```

All diagram commands support `--json` for JSON output, `--meta` for render metadata, `--width` for sizing, and layout args (`--margin-left`, `--alignment`, etc.).
