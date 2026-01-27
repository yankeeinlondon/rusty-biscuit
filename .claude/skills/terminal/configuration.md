# Terminal Configuration Guide

Configuration guides for popular modern terminal emulators.

## Alacritty

Minimal, GPU-accelerated terminal. No tabs or splits by design.

### Config Location

- Linux/BSD: `~/.config/alacritty/alacritty.toml`
- macOS: `~/.config/alacritty/alacritty.toml`
- Windows: `%APPDATA%\alacritty\alacritty.toml`

### Key Settings

```toml
# Font configuration
[font]
size = 12.0

[font.normal]
family = "JetBrains Mono"
style = "Regular"

# Colors (Dracula theme example)
[colors.primary]
background = "#282a36"
foreground = "#f8f8f2"

[colors.normal]
black = "#000000"
red = "#ff5555"
green = "#50fa7b"
yellow = "#f1fa8c"
blue = "#bd93f9"
magenta = "#ff79c6"
cyan = "#8be9fd"
white = "#bfbfbf"

# Window settings
[window]
padding = { x = 10, y = 10 }
decorations = "full"
opacity = 1.0

# Scrollback
[scrolling]
history = 10000

# Shell (auto-start tmux/zellij)
[terminal]
shell = { program = "zellij", args = ["-l", "welcome"] }
```

### IPC (Inter-Process Communication)

Enable socket communication:
```toml
[ipc]
socket = true
```

Control from command line:
```bash
alacritty msg create-window --working-directory /tmp -e htop
```

## Kitty

Feature-rich, GPU-accelerated with native multiplexing.

### Config Location

- `~/.config/kitty/kitty.conf`

### Key Settings

```conf
# Font
font_family JetBrains Mono
font_size 12.0

# Colors
background #1e1e2e
foreground #cdd6f4

# Scrollback
scrollback_lines 10000

# Cursor
cursor_shape beam
cursor_blink_interval 0

# URL handling
url_style curly
open_url_with default

# Tab bar
tab_bar_edge bottom
tab_bar_style powerline
active_tab_foreground #1e1e2e
active_tab_background #cba6f7

# Window layout
enabled_layouts splits,stack,grid
remember_window_size yes

# Key mappings
map ctrl+shift+enter new_window
map ctrl+shift+t new_tab
map ctrl+shift+l next_layout
map ctrl+alt+enter launch --location=vsplit
map ctrl+alt+h neighboring_window left
map ctrl+alt+l neighboring_window right
```

### Remote Control

Start Kitty with remote control:
```bash
kitty -o allow_remote_control=yes --listen-on unix:/tmp/mykitty
```

Send commands:
```bash
kitten @ launch --type=window htop
kitten @ set-tab-title "Development"
kitten @ send-text "ls -la\n"
```

## WezTerm

Lua-configurable, cross-platform terminal.

### Config Location

- `~/.wezterm.lua` or `~/.config/wezterm/wezterm.lua`

### Key Settings

```lua
local wezterm = require 'wezterm'
local config = {}

-- Font
config.font = wezterm.font 'JetBrains Mono'
config.font_size = 12.0

-- Colors
config.color_scheme = 'Catppuccin Mocha'

-- Window
config.window_padding = { left = 10, right = 10, top = 10, bottom = 10 }
config.window_background_opacity = 0.95

-- Tab bar
config.enable_tab_bar = true
config.use_fancy_tab_bar = true
config.hide_tab_bar_if_only_one_tab = true

-- Scrollback
config.scrollback_lines = 10000

-- Key bindings
config.keys = {
  { key = 's', mods = 'CTRL|SHIFT', action = wezterm.action.SplitHorizontal { domain = 'CurrentPaneDomain' } },
  { key = 'v', mods = 'CTRL|SHIFT', action = wezterm.action.SplitVertical { domain = 'CurrentPaneDomain' } },
  { key = 'w', mods = 'CTRL|SHIFT', action = wezterm.action.CloseCurrentPane { confirm = true } },
  { key = 'h', mods = 'CTRL|SHIFT', action = wezterm.action.ActivatePaneDirection 'Left' },
  { key = 'l', mods = 'CTRL|SHIFT', action = wezterm.action.ActivatePaneDirection 'Right' },
}

-- SSH domains
config.ssh_domains = {
  { name = 'dev-server', remote_address = 'user@dev.example.com' },
}

return config
```

### Multiplexer Domains

WezTerm supports multiple "domains" for session management:

```lua
-- Unix domain (local persistence)
config.unix_domains = {
  { name = 'unix' },
}

-- WSL domain (Windows)
config.wsl_domains = {
  { name = 'WSL:Ubuntu', distribution = 'Ubuntu' },
}
```

## iTerm2

macOS-only, feature-rich terminal with tmux integration.

### Config Location

- Main prefs: `~/Library/Preferences/com.googlecode.iterm2.plist`
- Dynamic profiles: `~/Library/Application Support/iTerm2/DynamicProfiles/`
- Colors: `~/.iterm2/colors/`

### Shell Integration

Install via: iTerm2 > Install Shell Integration

Adds features:
- Command marks navigation (Cmd+Shift+Up/Down)
- File transfer via drag-drop
- Current directory tracking
- Badges and status info

### tmux Integration

Use iTerm2 as native UI for tmux:
```bash
tmux -CC              # New session with iTerm2 integration
tmux -CC attach       # Attach to existing session
```

Benefits:
- tmux windows become native iTerm2 tabs
- Native split panes instead of tmux panes
- Full iTerm2 features (search, instant replay)
- Session persistence across disconnects

### Dynamic Profiles

JSON files in DynamicProfiles directory:
```json
{
  "Profiles": [{
    "Name": "Development",
    "Guid": "dev-profile-uuid",
    "Custom Command": "Yes",
    "Command": "cd ~/projects && zsh",
    "Working Directory": "~/projects",
    "Badge Text": "DEV"
  }]
}
```

## Ghostty

Modern GPU-accelerated terminal (Zig-based).

### Config Location

- Linux: `~/.config/ghostty/config`
- macOS: `~/Library/Application Support/com.mitchellh.ghostty/config`

### Key Settings

```
# Font
font-family = JetBrains Mono
font-size = 12

# Colors
background = 1e1e2e
foreground = cdd6f4

# Window
window-padding-x = 10
window-padding-y = 10

# Theme (100+ built-in)
theme = TokyoNight
# Light/dark auto-switch
theme = light:Rose Pine Dawn,dark:Rose Pine

# Keybindings
keybind = ctrl+shift+h=split_horizontal
keybind = ctrl+shift+v=split_vertical
keybind = ctrl+shift+t=new_tab
```

### Theme Preview

```bash
ghostty +list-themes         # List all themes
ghostty +list-themes --tui   # Interactive preview
```

## Warp

AI-powered terminal with modern UI.

### Config Location

Configuration primarily through GUI settings, with some options in:
- macOS: `~/.warp/`

### Key Features

- Block-based output (commands as discrete blocks)
- AI command suggestions
- Workflows (shareable command sequences)
- Built-in tmux integration

### Workflows

Create reusable command sequences:
```yaml
# ~/.warp/workflows/dev-setup.yaml
name: dev-setup
description: Start development environment
commands:
  - cd ~/projects/myapp
  - npm install
  - npm run dev
```

## Common Gotchas

### Font Rendering

- Install Nerd Fonts for icons/powerline symbols
- Set both ASCII and non-ASCII fonts if needed (iTerm2)
- Enable ligatures if font supports them

### True Color

Ensure `TERM` supports true color:
```bash
export TERM=xterm-256color  # Or terminal-specific value
```

Some apps check `COLORTERM`:
```bash
export COLORTERM=truecolor
```

### SSH and Remote

Terminal features may not work over SSH:
- Graphics protocols often fail
- Ensure remote `TERM` is set correctly
- Consider tmux for session persistence

### Multiplexer Conflicts

When using tmux/screen:
- Graphics protocols need passthrough: `set -g allow-passthrough on`
- `TERM` inside tmux differs from outside
- Some keybindings may conflict
