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
| `bt about [APP]` | Report on a specific terminal app |
| `bt image` | Render inline images |
| `bt prose` | Render styled prose text |
| `bt quote` | Block quote with left border |
| `bt list` | Bulleted list with hanging indents |
| `bt columns` | Two-column text layout |
| `bt block` | Styled text block via the render tree (fg/bg/emphasis/fill/border `Style`) |
| `bt progress` | Progress bar via the render tree (slot colors) |
| `bt table` | Table via the render tree (column headers, rows, striping) |
| `bt dir` | Directory tree with icons and gitignore awareness |
| `bt flowchart` | Flowchart diagrams |
| `bt quadrant` | Quadrant charts |
| `bt pie-chart` | Pie charts |
| `bt git-graph` | Git history diagrams |
| `bt bar-chart` | Bar charts |
| `bt line-chart` | Line charts |
| `bt timeline` | Timeline diagrams |
| `bt state-diagram` | State machine diagrams |
| `bt erd` | Entity relationship diagrams |

All diagram commands support:
- `--example` / `-e`: Render example with command shown
- `--width` / `-w`: Width spec (`50%`, `80ch`, `80`, `fill`)
- `--inverse`: Solid background with inverted colors
- `--title` / `-t`: Add title above diagram
- `--json`: Output as JSON (for scripting)
- `--meta`: Output rendering metadata to stderr (filename, cache hit, file size, render time)

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

## About a Specific Terminal App

```bash
bt about                # Report on the currently detected terminal
bt about kitty          # Report on Kitty
bt about iterm          # Prefix/contains match to iTerm2
bt about VSCode         # Alias match to VS Code
bt about kitty --json   # JSON output
bt about kitty --plain  # Plain text, no ANSI escapes
```

Output sections:
- **Identity**: App name, internal variant, whether it is the current terminal
- **Install Status**: Discovered executable/bundle path via `sniff`, or not installed/unknown
- **OS Target**: Config-resolution target (Linux, MacOS, Windows, Wsl1, Wsl2)
- **Resolved Config**: In-use config-file path and provenance
- **Config Candidates**: Default candidate paths for the current OS target
- **Config Overrides**: Config-relocating environment variables and their current values
- **Settings**: Extracted raw config values where statically parseable
- **Environment Facts**: Live env values (only when the queried app is the current terminal)

Invalid app names exit with code 2 and list the supported apps.

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

## Quote Command

Render styled text in a block quote with a left border:

```bash
bt quote "To be or not to be"
bt quote --attribution "Shakespeare" "To be or not to be"
bt quote "<bold>Important:</bold> This is <red>critical</red> information"
bt quote --attribution "Albert Einstein" "<i>Imagination is more important than knowledge.</i>"
```

Options:
- `--attribution`: Attribution (author/source) displayed below the quote
- `--margin-left` (alias `--ml`): Left margin in characters
- `--margin-right` (alias `--mr`): Right margin in characters
- `--margin-top` (alias `--mt`): Top margin in blank lines
- `--margin-bottom` (alias `--mb`): Bottom margin in blank lines
- `--alignment` (alias `--align`): Text alignment (`left`, `center`, `right`)

## List Command

Render a bulleted list with hanging indents:

```bash
bt list "First item" "Second item" "Third item"
bt list --bullet "- " "Item one" "Item two"
bt list --bullet "→ " "Step one" "Step two" "Step three"
bt list --no-hanging-indent "Item without hanging indent on wrap"
```

Options:
- `-b`/`--bullet`: Custom bullet string (default: `"• "`)
- `--no-hanging-indent`: Disable hanging indent on wrapped lines
- `--margin-left` (alias `--ml`): Left margin in characters
- `--margin-right` (alias `--mr`): Right margin in characters
- `--margin-top` (alias `--mt`): Top margin in blank lines
- `--margin-bottom` (alias `--mb`): Bottom margin in blank lines

## Block Command

Render a text block through the render tree carrying a declared `Style`
(foreground/background, emphasis, background, border) and an optional `Layout`
box (padding/margin/width/alignment). `bt block` renders via
`render_terminal_node`, so it exercises the render-tree `Style` and `Layout`
primitives directly:

```bash
bt block "Plain styled text" --fg red
bt block "Inverted notice" --fg white --bg blue --bold
bt block "Bordered notice" --border all
bt block "Rounded notice" --border all --border-radius 1
bt block "Painted padding" --bg blue --padding 1
bt block "Centered box" --width 20 --align center
```

Options:
- `--fg` / `--bg`: Foreground / background color (named or `#rrggbb`)
- `--bold` / `--italic` / `--underline` / `--strike`: Text emphasis
- `--fill`: Paint a background tint behind the text — `subtle` or `pronounced`
- `--border`: Draw a border — `all`, `left`, `right`, `top`, `bottom`
- `--border-color`: Border color (named or `#rrggbb`)
- `--border-radius`: Corner radius in columns; any non-zero value rounds corners
- `--padding`: Padding reserved inside the box, in columns, on all four sides
  (painted by `--bg`/`--fill`)
- `--margin`: Transparent horizontal margin, in columns
- `--width`: Content-box width — `auto`, `fit` (fit-content), or a column count
- `--max-width`: Cap the resolved content-box width, in columns
- `--align`: Place a sub-available box — `left`, `center`, `right`

## Progress Command

Render a progress bar through the render tree:

```bash
bt progress 60
bt progress 60 --label Loading
bt progress 75 --width 30 --fill-color green --bracket-color cyan
```

Options:
- `<PERCENT>`: Completion percentage, `0`–`100` (positional, required)
- `--label`: Text shown before the bar
- `--width`: Width of the bar portion in characters
- `--fill-color` / `--empty-color` / `--bracket-color`: Slot colors (named or `#rrggbb`)

## Table Command

Render a data table through the render tree:

```bash
bt table --columns "Name,Score" --row "Ann,90" --row "Bob,75"
bt table --columns "Name,Score" --row "Ann,90" --row "Bob,75" --striped
bt table --columns "Name,Score" --row "Ann,90" --row "Bob,75" --striped --stripe-bg blue
bt table --columns "Name,Score" --row "Ann,90" --bold-header --body-color cyan
```

Options:
- `--columns`: Comma-separated column headers (required)
- `--row`: Comma-separated cell values (repeatable — one per data row)
- `--striped`: Apply an alternating background stripe to even data rows
- `--stripe-bg` / `--stripe-text`: Explicit stripe colors (named or `#rrggbb`)
- `--bold-header`: Render every column header in bold
- `--header-color` / `--body-color`: Header / body text colors (named or `#rrggbb`)

## Directory Tree Command

Display a filesystem tree with Nerd Font icons and gitignore-aware dimming:

```bash
bt dir                              # Current directory
bt dir /path/to/project             # Specific path
bt dir --depth 2                    # Limit recursion depth
bt dir --filter ".rs"               # Filter by extension
bt dir -f ".rs" -f ".toml"          # Multiple filters
bt dir src --depth 3 --filter ".rs" # Combined
```

File metrics:

```bash
bt dir --size                       # Show file sizes (human-readable)
bt dir --tokens                     # Show estimated LLM token counts
bt dir --modified                   # Show absolute modification timestamps
bt dir --updated                    # Show relative times ("2 days ago")
bt dir --size --tokens --modified   # Combine metrics
```

Options:
- `-d`/`--depth`: Maximum recursion depth
- `-f`/`--filter`: Filter pattern (repeatable, e.g., `.rs`, `.toml`)
- `--skip-root`: Hide the root directory header line
- `--size`: Show human-readable file sizes
- `--tokens`: Show estimated LLM token counts
- `--modified`: Show absolute modification timestamps
- `--updated`: Show relative modification times
- `--margin-left` (alias `--ml`): Left margin in characters

## Prose Command

Render styled prose text with bracketed tags or a Markdown subset:

```bash
# Block tags
bt prose "Hello <b>world</b>!"
bt prose "<red>Error:</red> Something went wrong"
bt prose "<b>Bold</b> and <i>italic</i> text"
bt prose "<a href='https://example.com'>Click here</a>"

# Cross-target output
bt prose "<purple-800>Dark purple</purple-800>" --md-plus
bt prose "<b>Bold</b>" --margin-left 4 --md
bt prose "<b>Bold</b>" --margin-left 4 --html

# With margins and alignment
bt prose --margin-left 4 "Indented content"
bt prose --ml 2 --mr 2 "With margins on both sides"
bt prose --alignment center "Centered text"
bt prose --no-wrap "Long line that should not wrap"
```

Options:
- `--margin-left` (alias `--ml`): Left margin in characters
- `--margin-right` (alias `--mr`): Right margin in characters
- `--margin-top` (alias `--mt`): Top margin in blank lines
- `--margin-bottom` (alias `--mb`): Bottom margin in blank lines
- `--alignment` (alias `--align`): Text alignment (`left`, `center`, `right`)
- `--no-wrap`: Disable word wrapping
- `--html`: Render an HTML fragment instead of terminal output
- `--md`: Render portable Markdown instead of terminal output
- `--md-plus`: Render MarkdownPlus instead of terminal output

Supported syntax:
- **Block tags**: `<b>`, `<i>`, `<u>`, `<uu>`, `<~>`, `<a href="...">`, `<red>`, `<rgb R,G,B>`, `<bg-rgb R,G,B>`, `<bg-coral>`, `<bg-red-800>`
- **Markdown subset**: `[desc](url)`, `**bold**`, `_italic_`

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
