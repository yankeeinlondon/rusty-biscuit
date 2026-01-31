# biscuit-terminal-cli

A CLI tool (`bt`) for inspecting terminal capabilities and rendering images.

## Installation

```bash
cargo install --path .
```

Or from the workspace root:

```bash
just -f biscuit-terminal/justfile install
```

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

# Analyze escape code output
echo -e "\x1b[32mGreen\x1b[0m" | xargs bt
```

## Environment Variables

- `NO_COLOR`: When set, disables colored output in pretty-print mode
- `RUST_LOG`: Enables tracing output (e.g., `RUST_LOG=debug bt`)

## CLI Documentation Guidelines

When adding or updating CLI commands, follow these conventions:

1. **Command-specific examples**: Each subcommand should have its own examples section in `--help` output (using clap's `after_long_help` attribute). Examples should be specific to that command.

2. **Use long flag names in examples**: Always use long flag names (e.g., `--title`, `--width`) instead of short aliases (e.g., `-t`, `-w`) in documentation and examples. This improves readability and makes it clear what each flag does.

## License

AGPL-3.0
