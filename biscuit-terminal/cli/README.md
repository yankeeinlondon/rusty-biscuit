# biscuit-terminal-cli

A CLI tool (`bt`) for inspecting terminal capabilities and rendering images, styled text, directory trees, Mermaid diagrams, and graph diagrams.

## Installation

```bash
cargo install --path .
```

Or from the workspace root:

```bash
just -f biscuit-terminal/justfile install
```

## Commands Overview

| Command | Description |
|---------|-------------|
| `bt` | Terminal inspection (default) |
| `bt about [APP]` | Report on a specific terminal app |
| `bt image` | Render inline images |
| `bt prose` | Render styled text with tokens |
| `bt flowchart` | Flowchart diagrams |
| `bt quadrant` | Quadrant charts |
| `bt pie-chart` | Pie charts |
| `bt git-graph` | Git history diagrams |
| `bt bar-chart` | Bar charts |
| `bt line-chart` | Line charts |
| `bt timeline` | Timeline diagrams |
| `bt state-diagram` | State machine diagrams |
| `bt erd` | Entity relationship diagrams |
| `bt graph-expression` | Graph visualization (arrow, dash, DOT syntax) |
| `bt quote` | Block quote with left border |
| `bt list` | Bulleted list with hanging indents |
| `bt columns` | Two-column text layout |
| `bt block` | Styled text block via the render-tree `Style` primitive |
| `bt progress` | Progress bar via the render tree |
| `bt table` | Table via the render tree |
| `bt dir` | Directory tree with icons and gitignore awareness |

Every `bt <subcommand>` supports `--example` / `-e` to render a representative
example and then print the command that produced it.

## Usage

### Terminal Inspection (Default)

Display terminal metadata and capabilities:

```bash
bt              # Pretty-printed output
bt --json       # JSON output
bt -v           # Verbose output
```

Output includes:
- **Basic Info**: App, OS, distro, dimensions, TTY status, CI detection
- **Repository**: In-repo, monorepo, repo root path
- **Fonts**: Name, size, Nerd Font status, ligature support
- **Colors**: Depth, mode (light/dark), background/foreground/cursor RGB
- **Features**: Italics, images, OSC8 links, OSC10/11/12 queries, OSC52 clipboard, Mode 2027
- **Underlines**: Straight, double, curly, dotted, dashed, colored
- **Multiplexing**: tmux, Zellij, or native terminal support
- **Connection**: Local, SSH, or Mosh
- **Locale**: Raw locale, BCP47 tag, character encoding
- **Config**: Path to terminal configuration file

### About a Specific Terminal App

Report detailed config and environment metadata for a supported terminal app:

```bash
bt about                # Currently detected terminal
bt about kitty          # Exact match
bt about iterm          # Prefix/contains match
bt about VSCode         # Alias match
bt about kitty --json   # JSON output
bt about kitty --plain  # No ANSI escapes
```

Output includes install status, OS target, resolved config file, config
candidates, environment overrides, extracted settings, and live environment
facts (only when the queried app is the current terminal).

### Image Rendering

Render images inline using the terminal's graphics protocol:

```bash
bt image photo.jpg           # Default 50% width
bt image "photo.jpg|75%"     # 75% of terminal width
bt image "photo.jpg|80"      # Fixed 80 columns
bt image "photo.jpg|fill"    # Fill available width
```

Protocol selection:
- **Kitty protocol**: Kitty, WezTerm, Ghostty, Konsole, Warp
- **iTerm2 protocol**: iTerm2 (even if Kitty advertised)
- **Fallback**: Alt text for unsupported terminals

### Prose Rendering

Render styled text with bracketed tags or a Markdown subset:

```bash
bt prose "Hello <b>world</b>!"
bt prose "<red>Error:</red> Something went wrong"
bt prose "<b>Bold</b> and <i>italic</i> text"
bt prose "**Bold** and _italic_ via Markdown"
bt prose "<a href='https://example.com'>Click here</a>"
bt prose --margin-left 4 "Indented content"
bt prose --no-wrap "Long line without wrapping"
```

Grammar:
- **Block tags**: `<b>`, `<i>`, `<u>`, `<uu>`, `<~>`, `<a href="...">`, `<red>`, `<rgb R,G,B>`, `<bg-rgb R,G,B>`, `<bg-coral>`, `<bg-red-800>`
- **Markdown subset**: `**bold**`, `_italic_`, `[desc](url)`, fenced code blocks
- **Color support**: Basic colors, bright colors, web colors, Tailwind colors (foreground and background)

Options:
- `--margin-left` (alias `--ml`): Left margin in characters
- `--margin-right` (alias `--mr`): Right margin in characters
- `--margin-top` (alias `--mt`): Top margin in blank lines
- `--margin-bottom` (alias `--mb`): Bottom margin in blank lines
- `--alignment` (alias `--align`): Text alignment (`left`, `center`, `right`)
- `--no-wrap`: Disable word wrapping

### Block Quote Rendering

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

### List Rendering

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
- `--alignment` (alias `--align`): Text alignment (`left`, `center`, `right`)

### Columns Rendering

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
- `--margin-left` (alias `--ml`): Left margin in characters
- `--margin-right` (alias `--mr`): Right margin in characters
- `--margin-top` (alias `--mt`): Top margin in blank lines
- `--margin-bottom` (alias `--mb`): Bottom margin in blank lines
- `--alignment` (alias `--align`): Text alignment (`left`, `center`, `right`)

### Styled Block Rendering

Render a text block through the render tree carrying a declared `Style` —
foreground/background color, text emphasis, a painted fill band, and a border:

```bash
bt block "Plain styled text" --fg red
bt block "Inverted notice" --fg white --bg blue --bold
bt block "Bordered notice" --border all
bt block "Rounded notice" --border all --border-radius 1
bt block "Full-width band" --fill subtle --fill-band full
bt block "Indented band" --fill pronounced --fill-band indented --inset 4
```

Options:
- `--fg`: Foreground color (named, e.g. `red`, or `#rrggbb`)
- `--bg`: Background color (named or `#rrggbb`)
- `--bold` / `--italic` / `--underline` / `--strike`: Text emphasis
- `--fill`: Paint a background fill band — `subtle` or `pronounced`
- `--fill-band`: Band painted by `--fill` — `full` (default), `padded`, `indented`
- `--inset`: Inset, in columns, applied to the fill band
- `--border`: Draw a border — `all`, `left`, `right`, `top`, `bottom`
- `--border-color`: Border color (named or `#rrggbb`)
- `--border-radius`: Corner radius in columns; any non-zero value rounds the corners

`bt block` renders through `render_terminal_node`, so it exercises the
render-tree `Style` primitive directly.

### Progress Bar Rendering

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
- `--fill-color`: Color of the filled track (named or `#rrggbb`)
- `--empty-color`: Color of the empty track (named or `#rrggbb`)
- `--bracket-color`: Color of the bracket glyphs (named or `#rrggbb`)

### Table Rendering

Render a data table through the render tree:

```bash
bt table --columns "Name,Score" --row "Ann,90" --row "Bob,75"
bt table --columns "Name,Score" --row "Ann,90" --row "Bob,75" --striped
bt table --columns "Name,Score" --row "Ann,90" --row "Bob,75" --striped --stripe-bg blue
bt table --columns "Name,Score" --row "Ann,90" --bold-header --body-color cyan
bt table --columns "Status,Count,Price" --column-types ",int,usd" --mixed-row "<b>active</b>,1234,9.99"
```

Options:
- `--columns`: Comma-separated column headers (required). Header text is literal
- `--column-types`: Comma-separated column types, positionally aligned with
  `--columns` (`int`/`integer`, `float`, a currency code `usd`/`gbp`/`eur`, or
  empty/`string` for a text column). A numeric or currency type declares a
  right-aligned column that formats its `--mixed-row` cells
- `--row`: Comma-separated cell values (repeatable — one per data row)
- `--mixed-row`: Comma-separated row whose cells take their kind from each
  column's type — numeric columns parse a typed, right-aligned value, other
  columns parse `Prose` markup (left-aligned). Repeatable
- `--striped`: Apply an alternating background stripe to even data rows
- `--stripe-bg`: Explicit stripe background color (named or `#rrggbb`)
- `--stripe-text`: Explicit stripe text color (named or `#rrggbb`)
- `--bold-header`: Render every column header in bold
- `--header-color`: Header text color (named or `#rrggbb`)
- `--body-color`: Body (data cell) text color (named or `#rrggbb`)

### Directory Tree

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
- `--margin-right` (alias `--mr`): Right margin in characters
- `--margin-top` (alias `--mt`): Top margin in blank lines
- `--margin-bottom` (alias `--mb`): Bottom margin in blank lines
- `--alignment` (alias `--align`): Text alignment (`left`, `center`, `right`)

### Flowchart Rendering

Render Mermaid flowcharts directly in the terminal:

```bash
bt flowchart "A --> B --> C"                       # Left-to-right (default)
bt flowchart --vertical "A --> B --> C"            # Top-down
bt flowchart --inverse "A --> B --> C"             # Solid background, inverted colors
bt flowchart --title "My Process" "A --> B --> C"  # With title
bt flowchart --width 50% "A --> B --> C"           # Render at 50% terminal width
bt flowchart --width 80ch "A --> B"                # Render at 80 characters wide
bt flowchart "A[Input] --> B{Decision}" "B -->|Yes| C[Output]"
bt flowchart --json "A --> B"                      # Output as JSON
```

**Features:**
- **Color mode detection**: Automatically uses light or dark theme based on terminal background
- **Transparent background**: Blends seamlessly with terminal (default)
- **Inverse mode**: Solid background with contrasting colors (`--inverse`)
- **High resolution**: 2x scale for sharp rendering on modern displays
- **Width control**: `-w`/`--width` accepts percentages (`50%`), characters (`80ch` or `80`), or `fill` (default: 50%)
- **Aspect ratio preservation**: Images always maintain correct proportions via viuer

**Rendering backend:**
- Pure Rust rendering via `biscuit-visualized` and `mermaid-rs-renderer`
- No Node.js, npm, Chromium, or `mmdc` dependency is required
- Use `--json` to inspect generated Mermaid instructions when image rendering is not available

**Error handling:**
- Syntax errors show the location and expected tokens
- Returns non-zero exit code on errors

### Quadrant Chart Rendering

Render Mermaid quadrant charts directly in the terminal:

```bash
bt quadrant "Item A: [0.3, 0.6]" "Item B: [0.7, 0.4]"
bt quadrant --x-axis "Low --> High" --y-axis "Small --> Large" "Item: [0.5, 0.5]"
bt quadrant --title "Priority Matrix" "Task A: [0.2, 0.8]" "Task B: [0.6, 0.3]"
bt quadrant --theme magic-quadrangle "Leaders: [0.8, 0.8]" "Niche: [0.2, 0.2]"
bt quadrant --inverse "Item: [0.5, 0.5]"               # Solid background, inverted colors
bt quadrant --width 60% "Item: [0.5, 0.5]"             # Render at 60% terminal width
bt quadrant --json "Item: [0.5, 0.5]"                  # Output as JSON
```

**Data points** are specified as `"Label: [x, y]"` where x and y are values between 0.0 and 1.0.

**Options:**

| Option | Description |
|--------|-------------|
| `-x`/`--x-axis` | X-axis label (e.g., "Low Reach --> High Reach") |
| `-y`/`--y-axis` | Y-axis label (e.g., "Low Engagement --> High Engagement") |
| `-t`/`--title` | Chart title (appears at top of diagram) |
| `--top-right`/`--tr` | Top-right quadrant label (Mermaid's quadrant-1) |
| `--top-left`/`--tl` | Top-left quadrant label (Mermaid's quadrant-2) |
| `--bottom-left`/`--bl` | Bottom-left quadrant label (Mermaid's quadrant-3) |
| `--bottom-right`/`--br` | Bottom-right quadrant label (Mermaid's quadrant-4) |
| `--point-radius` | Default point radius (default: 5) |
| `--label-size` | Point label font size (default: 18 for ≤6 points, 15 for >6) |
| `--theme` | Color theme preset (`default`, `magic-quadrangle`) |
| `--q1-fill` to `--q4-fill` | Individual quadrant fill colors (hex) |
| `--inverse` | Solid background with contrasting colors |
| `-w`/`--width` | Width: percentages (`50%`), characters (`80ch`), or `fill` |

**Quadrant numbering** (matches Mermaid convention):
```
        +-------------+-------------+
        |  quadrant-2 |  quadrant-1 |
        |  (top-left) | (top-right) |
        +-------------+-------------+
        |  quadrant-3 |  quadrant-4 |
        |(bottom-left)|(bottom-right)|
        +-------------+-------------+
```

**Themes:**
- `default`: Standard Mermaid colors
- `magic-quadrangle`: Gartner-style with subtle green top-right (leaders), subtle red bottom-left (niche players), and neutral colors for top-left and bottom-right. Colors automatically adapt to terminal light/dark mode.

> **Tip:** With shell completions enabled, typing `--theme <TAB>` will show available theme options.

**Inline point styling** - individual points can override defaults using comma-separated properties:
```bash
bt quadrant "Item A: [0.3, 0.6] color: #ff3300, radius: 12" \
            "Item B: [0.7, 0.4] color: #00ff00"
```

Available inline properties: `color`, `radius`, `stroke-color`, `stroke-width`

> **Note:** Multiple properties must be comma-separated. Space-only separation causes parsing errors.

**Rendering backend:**
- Pure Rust rendering via `biscuit-visualized` and `mermaid-rs-renderer`
- No external Mermaid CLI is required

### Git Graph Rendering

Render Mermaid git graphs directly in the terminal:

```bash
bt git-graph "commit" "branch develop" "checkout develop" "commit"
bt git-graph --inverse "commit" "commit" "commit"
bt git-graph --title "Feature Branch" "commit" "branch feature" "commit" "checkout main" "merge feature"
bt git-graph --width 50% "commit" "commit"      # Render at 50% terminal width
bt git-graph --width 80ch "commit"              # Render at 80 characters wide
bt git-graph --json "commit" "branch feature"   # Output as JSON
```

**Git commands:**
- `commit` - Add a commit to the current branch
- `commit id: "abc123"` - Commit with custom ID
- `commit tag: "v1.0"` - Commit with a tag
- `branch <name>` - Create a new branch
- `checkout <name>` - Switch to a branch
- `merge <name>` - Merge a branch into the current branch
- `cherry-pick id: "abc123"` - Cherry-pick a commit

**Features:**
- **Color mode detection**: Automatically uses light or dark theme based on terminal background
- **Transparent background**: Blends seamlessly with terminal (default)
- **Inverse mode**: Solid background with contrasting colors (`--inverse`)
- **Title support**: Add a title above the diagram (`-t`/`--title`)
- **Width control**: `-w`/`--width` accepts percentages (`50%`), characters (`80ch` or `80`), or `fill` (default: 50%)
- **Aspect ratio preservation**: Images always maintain correct proportions via viuer

**Rendering backend:**
- Pure Rust rendering via `biscuit-visualized` and `mermaid-rs-renderer`
- No external Mermaid CLI is required

### Pie Chart Rendering

Render Mermaid pie charts:

```bash
bt pie-chart "Dogs: 386" "Cats: 85" "Birds: 15"
bt pie-chart --title "Pet Distribution" "Dogs: 386" "Cats: 85"
bt pie-chart --show-data "TypeScript: 45" "Rust: 35"  # Show percentages
bt pie-chart "TypeScript: 45 #3178c6" "Rust: 35 #dea584"  # Custom colors
```

### Bar Chart Rendering

Render Mermaid bar charts:

```bash
bt bar-chart 10 20 15 25
bt bar-chart --x-axis "Q1,Q2,Q3,Q4" --y-axis Sales 10 20 15 25
bt bar-chart --horizontal 1 8 7 5
bt bar-chart --show-data-label 1 8 7 5
bt bar-chart --line 10 20 15 25  # Add trend line
bt bar-chart --aspect-ratio 2.0 --width 60% 10 20 15 25
```

Input formats: JSON array `"[1,8,7]"`, comma-separated `"1,8,7"`, or space-separated `1 8 7`

### Line Chart Rendering

Render Mermaid line charts:

```bash
bt line-chart 1 8 7 5 9 3
bt line-chart --x-axis "Mon,Tue,Wed" --y-axis Temperature 20 22 19
bt line-chart --bar 1 8 7 5  # Add bars under line
bt line-chart --show-data-label --horizontal 1 8 7 5
bt line-chart --aspect-ratio 1.8 --inverse --width 60% 1 8 7 5
```

### Timeline Rendering

Render Mermaid timelines:

```bash
bt timeline "2020: Project started" "2021: First release" "2022: Major update"
bt timeline --title "Company History" "2020: Founded" "2022: IPO"
bt timeline --section "Early Years" "2020: Founded" --section "Growth" "2022: Series A"
```

### State Diagram Rendering

Render Mermaid state diagrams:

```bash
bt state-diagram "[*] --> Idle" "Idle --> Running" "Running --> [*]"
bt state-diagram "[*] --> Idle" "Idle --> Running: start" "Running --> Stopped: stop"
```

Syntax: `[*]` = start/end state, `State1 --> State2: label` = labeled transition

### ERD Rendering

Render Mermaid entity relationship diagrams:

```bash
bt erd "Customer ||--o{ Order : places" "Order ||--|{ LineItem : contains"
bt erd --entity "Customer { id int PK, name string }" "Customer ||--o{ Order : places"
```

Relationships: `||--||` (one-to-one), `||--o{` (one-to-many), `}o--o{` (many-to-many)

### Graph Expression Rendering

Render graph structures using `biscuit-visualized`'s `layout-rs` backend with multiple syntax options:

```bash
bt graph-expression "a -> b -> c"                           # Arrow syntax (directed)
bt graph-expression "a -- b -- c"                           # Dash syntax (undirected)
bt graph-expression --syntax dot "digraph { A -> B; B -> C; }"  # DOT syntax
bt graph-expression --title "Data Flow" "start -> validate -> render"
bt graph-expression --orientation top-to-bottom "a -> b -> c"
bt graph-expression --orientation left-to-right "a -> b -> c"
bt graph-expression --width 60% "a -> b -> c"
bt graph-expression --inverse "a -> b -> c"
bt graph-expression --meta "a -> b"
bt graph-expression --json "a -> b"
```

**Syntax options:**
- **Arrow (`->`)**: Directed edges, e.g., `a -> b -> c; b -> d`
- **Dash (`--`)**: Undirected edges, e.g., `a -- b -- c`
- **DOT**: Full Graphviz DOT language support, e.g., `digraph { A -> B; }`

Mixed directed and undirected expression syntax is rejected. For example, `a -> b; c -- d` is invalid and should be split into separate graphs.

**Orientation:**
- `top-to-bottom` (default)
- `left-to-right`

> `layout-rs` currently supports these two layout directions in `bt graph-expression`.

**Features:**
- **Pure Rust rendering**: Uses `layout-rs` via `biscuit-visualized` (no external dependencies)
- **Color mode detection**: Automatically uses light or dark theme
- **Transparent background**: Blends with terminal (default)
- **Inverse mode**: Solid background with contrasting colors (`--inverse`)
- **Metadata output**: `--meta` writes render metadata to stderr (filename, cache hit, file size, render time)
- **Width control**: `-w`/`--width` accepts percentages (`50%`), characters (`80ch` or `80`), or `fill` (default: 50%)

### Common Diagram Options

All diagram commands support:
- `--example` / `-e`: Render example with command shown
- `--width` / `-w`: Width spec (`50%`, `80ch`, `80`, `fill`)
- `--inverse`: Solid background with inverted colors
- `--title` / `-t`: Add title above diagram
- `--json`: Output as JSON for scripting
- `--meta`: Output rendering metadata to stderr (filename, cache hit, file size, render time)

### Shell Completions

Enable tab completion for your shell:

**Dynamic completions (recommended)** - includes image file filtering:

```bash
# Bash
echo 'source <(COMPLETE=bash bt)' >> ~/.bashrc

# Zsh
echo 'source <(COMPLETE=zsh bt)' >> ~/.zshrc

# Fish
echo 'COMPLETE=fish bt | source' >> ~/.config/fish/config.fish
```

**Static completions** - generates a script once:

```bash
bt --completions bash >> ~/.bashrc
bt --completions zsh > ~/.zfunc/_bt
bt --completions fish > ~/.config/fish/completions/bt.fish
bt --completions powershell >> $PROFILE
```

For detailed setup instructions:

```bash
bt --completions help
```

### Content Analysis

Analyze text content for escape codes and visual widths:

```bash
bt "Hello \x1b[31mWorld\x1b[0m"
```

Output:
- Line count and lengths (escape codes stripped)
- Color escape code detection
- OSC8 link detection
- Total character length

## Examples

```bash
# Quick terminal check
bt

# Machine-readable output for scripting
bt --json | jq '.image_support'

# Display an image
bt image ./screenshot.png

# Render a flowchart
bt flowchart "Start --> Process --> End"

# Render a git graph showing a feature branch workflow
bt git-graph "commit" "branch feature" "commit" "commit" "checkout main" "merge feature"

# Render a quadrant chart for priority analysis
bt quadrant --title "Priority Matrix" \
            --x-axis "Low Effort --> High Effort" \
            --y-axis "Low Impact --> High Impact" \
            --top-left "Quick Wins" --top-right "Major Projects" \
            --bottom-left "Fill-ins" --bottom-right "Thankless Tasks" \
            "Task A: [0.2, 0.8]" "Task B: [0.7, 0.3]"

# Render a pie chart
bt pie-chart "Dogs: 386" "Cats: 85" "Birds: 15"

# Render a bar chart
bt bar-chart --x-axis "Q1,Q2,Q3,Q4" 10 20 15 25

# Render a timeline
bt timeline "2020: Started" "2022: Launch" "2024: Expansion"

# Render a state diagram
bt state-diagram "[*] --> Idle" "Idle --> Running" "Running --> [*]"

# Render an ERD
bt erd "Customer ||--o{ Order : places"

# Render a graph visualization
bt graph-expression "a -> b -> c"
bt graph-expression "a -- b -- c"
bt graph-expression --syntax dot "digraph { A -> B; B -> C; }"

# Display a directory tree
bt dir src --depth 2 --filter ".rs"

# Analyze escape code output
echo -e "\x1b[32mGreen\x1b[0m" | xargs bt
```

## Environment Variables

- `NO_COLOR`: When set, disables colored output in pretty-print mode
- `RUST_LOG`: Enables tracing output (e.g., `RUST_LOG=debug bt`)

## Library Integration

This CLI uses `biscuit-terminal` with the `clap` feature enabled, which provides:
- Shell completions for enum-based arguments (e.g., `--theme` shows `default`, `magic-quadrangle`)
- Automatic help text listing valid enum values

If you're building your own CLI using `biscuit-terminal`, enable the feature:

```toml
[dependencies]
biscuit-terminal = { version = "0.1", features = ["clap"] }
```

## CLI Documentation Guidelines

When adding or updating CLI commands, follow these conventions:

1. **Command-specific examples**: Each subcommand should have its own examples section in `--help` output (using clap's `after_long_help` attribute). Examples should be specific to that command.

2. **Use long flag names in examples**: Always use long flag names (e.g., `--title`, `--width`) instead of short aliases (e.g., `-t`, `-w`) in documentation and examples. This improves readability and makes it clear what each flag does.

3. **Section header styling**: Custom section headers in `after_long_help` should match clap's built-in styling (bold + underline) using ANSI escape codes. Use sentence case (e.g., "Examples:" not "EXAMPLES:"):
   ```
   \x1b[1m\x1b[4mExamples:\x1b[0m
   ```
   - `\x1b[1m` = bold
   - `\x1b[4m` = underline
   - `\x1b[0m` = reset

## License

AGPL-3.0
