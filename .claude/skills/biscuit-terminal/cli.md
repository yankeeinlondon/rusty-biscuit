# bt Command

The `bt` CLI tool for terminal inspection, image rendering, Mermaid diagrams, and content analysis.

## Installation

```bash
# From workspace root
just -f biscuit-terminal/justfile install

# Or directly
cargo install --path biscuit-terminal/cli
```

## Commands Overview

| Command | Description |
|---------|-------------|
| `bt` | Terminal inspection (default) |
| `bt image` | Render inline images |
| `bt prose` | Render styled prose text |
| `bt flowchart` | Flowchart diagrams |
| `bt quadrant` | Quadrant charts |
| `bt pie-chart` | Pie charts |
| `bt git-graph` | Git history diagrams |
| `bt bar-chart` | Bar charts |
| `bt line-chart` | Line charts |
| `bt timeline` | Timeline diagrams |
| `bt state-diagram` | State machine diagrams |
| `bt erd` | Entity relationship diagrams |
| `bt columns` | Two-column text layout |

All diagram commands support:
- `--example` / `-e`: Render example with command shown
- `--width` / `-w`: Width spec (`50%`, `80ch`, `80`, `fill`)
- `--inverse`: Solid background with inverted colors
- `--title` / `-t`: Add title above diagram
- `--json`: Output as JSON (for scripting)

Bar/line chart extras: `--horizontal`, `--show-data-label`, `--aspect-ratio`

## Terminal Inspection (Default)

```bash
bt              # Pretty-printed output
bt --json       # JSON output for scripting
bt -v           # Verbose output
```

Output sections:
- **Basic Info**: App, OS, distro, dimensions, TTY, CI
- **Fonts**: Name, size, Nerd Font, ligatures
- **Colors**: Depth, mode, background/foreground/cursor RGB
- **Features**: Italics, images, OSC8, OSC10/11/12, OSC52, Mode 2027
- **Underlines**: Straight, double, curly, dotted, dashed, colored
- **Multiplexing**: Type (tmux, Zellij, native)
- **Connection**: Local, SSH, Mosh
- **Locale**: Raw, BCP47 tag, encoding
- **Config**: Path to terminal config file

## Image Rendering

```bash
bt image photo.jpg           # Default 50% width
bt image "photo.jpg|75%"     # 75% of terminal
bt image "photo.jpg|80"      # Fixed 80 columns
bt image "photo.jpg|fill"    # Fill available width
```

Protocol selection:
- **Kitty**: Kitty, WezTerm, Ghostty, Konsole, Warp
- **iTerm2**: iTerm2 (forced even if Kitty advertised)
- **Fallback**: Alt text for unsupported terminals

## Diagram Commands

See [Mermaid Diagrams](./mermaid-diagrams.md) for comprehensive diagram documentation.

Render failures are reported as errors and return a non-zero exit code. `bt` does not auto-print Mermaid code-block fallback output.

### Quick Examples

```bash
# Flowchart
bt flowchart "A --> B --> C"
bt flowchart --vertical "Start --> Process --> End"

# Quadrant chart
bt quadrant "Task A: [0.2, 0.8]" "Task B: [0.7, 0.3]"
bt quadrant --theme magic-quadrangle "Leaders: [0.8, 0.8]"

# Pie chart
bt pie-chart "Dogs: 386" "Cats: 85" "Birds: 15"
bt pie-chart --show-data "TypeScript: 45 #3178c6" "Rust: 35"

# Git graph
bt git-graph "commit" "branch feature" "commit" "merge feature"

# Bar/Line charts
bt bar-chart --horizontal --show-data-label --aspect-ratio 2.0 --inverse 10 20 15 25
bt line-chart --show-data-label --horizontal 1 8 7 5
bt line-chart --aspect-ratio 1.8 --inverse --width 60% 1 8 7 5

# Timeline
bt timeline "2020: Founded" "2022: Series A" "2024: IPO"

# State diagram
bt state-diagram "[*] --> Idle" "Idle --> Running" "Running --> [*]"

# ERD
bt erd "Customer ||--o{ Order : places"
```

## Columns Command

Render two columns of text with optional gap and width control:

```bash
bt columns "Left column" "Right column"
bt columns --gap 6 "Left" "Right"
bt columns --left 24 "Title" "Longer description on the right"
bt columns --left 40% "Short" "Longer content that wraps"
bt columns --margin-left 2 --margin-right 2 --alignment center "Left" "Right"
```

Options:
- `--gap`: Gap between columns in characters (default: 3)
- `--left`: Left column width (e.g., `20`, `20ch`, `40%`)

## Prose Command

Render styled prose text with inline tokens:

```bash
# Atomic tokens
bt prose "Hello {{bold}}world{{reset}}!"
bt prose "{{red}}Error:{{reset}} Something went wrong"

# Block tags
bt prose "<b>Bold</b> and <i>italic</i> text"
bt prose "<a href='https://example.com'>Click here</a>"

# With margins
bt prose --left-margin 4 "Indented content"
bt prose -l 2 -r 2 "With margins on both sides"

# Disable word wrapping
bt prose --no-wrap "Long line that should not wrap"
```

Supported tokens:
- **Atomic**: `{{bold}}`, `{{italic}}`, `{{red}}`, `{{bg-blue}}`, `{{reset}}`
- **Block**: `<b>`, `<i>`, `<u>`, `<uu>`, `<~>`, `<a href="...">`, `<red>`, `<rgb R,G,B>`

## Content Analysis

```bash
bt "Hello World"
bt "$(echo -e '\x1b[32mGreen\x1b[0m')"
```

Output:
- Line count
- Line lengths (escape codes stripped)
- Color escape code presence
- OSC8 link presence
- Total character length

## Shell Completions

```bash
# Dynamic (recommended)
echo 'source <(COMPLETE=bash bt)' >> ~/.bashrc    # Bash
echo 'source <(COMPLETE=zsh bt)' >> ~/.zshrc      # Zsh

# Static
bt --completions bash >> ~/.bashrc
bt --completions zsh > ~/.zfunc/_bt
```

## Environment Variables

| Variable | Effect |
|----------|--------|
| `NO_COLOR` | Disables colored output in pretty-print mode |
| `RUST_LOG` | Enables tracing (e.g., `RUST_LOG=debug bt`) |

## JSON Output Schema

```json
{
  "app": "Wezterm",
  "os": "MacOS",
  "distro": null,
  "width": 120,
  "height": 40,
  "is_tty": true,
  "is_ci": false,
  "font": "JetBrains Mono",
  "font_size": 14,
  "is_nerd_font": true,
  "ligatures_likely": true,
  "color_depth": "TrueColor",
  "color_mode": "Dark",
  "bg_color": { "r": 30, "g": 30, "b": 46, "hex": "#1e1e2e" },
  "text_color": { "r": 205, "g": 214, "b": 244, "hex": "#cdd6f4" },
  "supports_italic": true,
  "image_support": "Kitty",
  "underline_support": {
    "straight": true,
    "double": true,
    "curly": true,
    "dotted": true,
    "dashed": true,
    "colored": true
  },
  "osc_link_support": true,
  "osc10_fg_color": true,
  "osc11_bg_color": true,
  "osc12_cursor_color": true,
  "osc52_clipboard": true,
  "mode_2027_graphemes": true,
  "multiplex": "Native",
  "connection": { "type": "Local" },
  "locale_raw": "en_US.UTF-8",
  "locale_tag": "en-US",
  "char_encoding": "UTF8",
  "config_file": "/Users/user/.wezterm.lua"
}
```

## Use Cases

### Terminal Debugging

```bash
# Check why images aren't working
bt --json | jq '{app, image_support, is_tty}'

# Verify color support
bt --json | jq '{color_depth, color_mode}'
```

### CI Detection

```bash
# In scripts
if bt --json | jq -e '.is_ci' > /dev/null; then
    echo "Running in CI"
fi
```

### Configuration Verification

```bash
# Find config file location
bt --json | jq -r '.config_file // "Not found"'

# Check font setup
bt --json | jq '{font, font_size, is_nerd_font}'
```

## Related

- [Terminal Struct](./terminal-struct.md) - Same data as library API
- [Image Rendering](./image-rendering.md) - Image implementation details
- [Mermaid Diagrams](./mermaid-diagrams.md) - Comprehensive diagram documentation
