# bt Command

The `bt` CLI tool for terminal inspection, image rendering, and content analysis.

## Installation

```bash
# From workspace root
just -f biscuit-terminal/justfile install

# Or directly
cargo install --path biscuit-terminal/cli
```

## Commands

### Terminal Inspection (Default)

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

### Image Rendering

```bash
bt --image photo.jpg           # Default 50% width
bt --image "photo.jpg|75%"     # 75% of terminal
bt --image "photo.jpg|80"      # Fixed 80 columns
bt --image "photo.jpg|fill"    # Fill available width
```

Protocol selection:
- **Kitty**: Kitty, WezTerm, Ghostty, Konsole, Warp
- **iTerm2**: iTerm2 (forced even if Kitty advertised)
- **Fallback**: Alt text for unsupported terminals

### Content Analysis

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

## Examples

### Quick Terminal Check

```bash
$ bt
Terminal Metadata
═══════════════════════════════════════

Basic Info
  App:        Wezterm
  OS:         MacOS
  Size:       120 x 40
  Is TTY:     yes
  In CI:      no

Fonts
  Name:       JetBrains Mono
  Size:       14pt
  Nerd Font:  yes
  Ligatures:  likely

Colors
  Depth:      TrueColor
  Mode:       Dark
  Background: #1e1e2e (30, 30, 46)
  ...
```

### JSON Output for Scripting

```bash
# Check image support
bt --json | jq '.image_support'
# "Kitty"

# Get color depth
bt --json | jq '.color_depth'
# "TrueColor"

# Check if in CI
bt --json | jq '.is_ci'
# false
```

### Display Images

```bash
# Default width (50%)
bt --image ./screenshot.png

# Full width
bt --image "./diagram.svg|fill"

# Thumbnail
bt --image "./photo.jpg|25%"
```

### Analyze Styled Content

```bash
# Check if content has escape codes
$ bt "$(echo -e '\x1b[1mBold\x1b[0m')"

Content Analysis
══════════════════
  Lines:        1
  Line lengths: 4
  Total length: 4
  Color codes:  yes
  OSC8 links:   no
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
- [Image Rendering](./image-rendering.md) - `--image` implementation details
