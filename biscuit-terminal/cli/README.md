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
bt --image photo.jpg           # Default 50% width
bt --image "photo.jpg|75%"     # 75% of terminal width
bt --image "photo.jpg|80"      # Fixed 80 columns
bt --image "photo.jpg|fill"    # Fill available width
```

Protocol selection:
- **Kitty protocol**: Kitty, WezTerm, Ghostty, Konsole, Warp
- **iTerm2 protocol**: iTerm2 (even if Kitty advertised)
- **Fallback**: Alt text for unsupported terminals

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
bt --image ./screenshot.png

# Analyze escape code output
echo -e "\x1b[32mGreen\x1b[0m" | xargs bt
```

## Environment Variables

- `NO_COLOR`: When set, disables colored output in pretty-print mode
- `RUST_LOG`: Enables tracing output (e.g., `RUST_LOG=debug bt`)

## License

AGPL-3.0
