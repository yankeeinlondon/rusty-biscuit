# biscuit-terminal-cli

A CLI tool (`bt`) for inspecting terminal capabilities, rendering images, and generating Mermaid diagrams.

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
| `bt image` | Render inline images |
| `bt flowchart` | Flowchart diagrams |
| `bt quadrant` | Quadrant charts |
| `bt pie-chart` | Pie charts |
| `bt git-graph` | Git history diagrams |
| `bt bar-chart` | Bar charts |
| `bt line-chart` | Line charts |
| `bt timeline` | Timeline diagrams |
| `bt state-diagram` | State machine diagrams |
| `bt erd` | Entity relationship diagrams |

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
- **Fonts**: Name, size, Nerd Font status, ligature support
- **Colors**: Depth, mode (light/dark), background/foreground/cursor RGB
- **Features**: Italics, images, OSC8 links, OSC10/11/12 queries, OSC52 clipboard, Mode 2027
- **Underlines**: Straight, double, curly, dotted, dashed, colored
- **Multiplexing**: tmux, Zellij, or native terminal support
- **Connection**: Local, SSH, or Mosh
- **Locale**: Raw locale, BCP47 tag, character encoding
- **Config**: Path to terminal configuration file

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

**Requirements:**
- `mmdc` (Mermaid CLI): Install with `npm install -g @mermaid-js/mermaid-cli`
- Falls back to `npx` if mmdc is not installed
- Falls back to a code block if image rendering is not supported

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

**Requirements:**
- `mmdc` (Mermaid CLI): Install with `npm install -g @mermaid-js/mermaid-cli`
- Falls back to `npx` if mmdc is not installed

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

**Requirements:**
- `mmdc` (Mermaid CLI): Install with `npm install -g @mermaid-js/mermaid-cli`
- Falls back to `npx` if mmdc is not installed

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
```

Input formats: JSON array `"[1,8,7]"`, comma-separated `"1,8,7"`, or space-separated `1 8 7`

### Line Chart Rendering

Render Mermaid line charts:

```bash
bt line-chart 1 8 7 5 9 3
bt line-chart --x-axis "Mon,Tue,Wed" --y-axis Temperature 20 22 19
bt line-chart --bar 1 8 7 5  # Add bars under line
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

### Common Diagram Options

All diagram commands support:
- `--example` / `-e`: Render example with command shown
- `--width` / `-w`: Width spec (`50%`, `80ch`, `80`, `fill`)
- `--inverse`: Solid background with inverted colors
- `--title` / `-t`: Add title above diagram
- `--json`: Output as JSON for scripting

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
