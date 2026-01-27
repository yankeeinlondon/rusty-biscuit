# Terminal Multiplexing

Multiplexers allow multiple terminal sessions within a single window, with session persistence and remote access.

## Comparison Overview

| Feature | tmux | Zellij | Kitty | WezTerm | iTerm2 |
|---------|------|--------|-------|---------|--------|
| Session Persist | Yes | Yes | No | No | Via tmux |
| Remote Attach | Yes | Yes | No | No | Via tmux |
| GPU Render | No | No | Yes | Yes | Yes |
| Config | Text | YAML/KDL | Text | Lua | GUI/JSON |
| Plugin System | No | WASM | Python | Lua | Python |
| Learning Curve | High | Low | Medium | Medium | Low |

## tmux

Industry-standard terminal multiplexer. Essential for remote work.

### Installation

```bash
# macOS
brew install tmux

# Ubuntu/Debian
sudo apt install tmux

# Arch
sudo pacman -S tmux
```

### Key Concepts

- **Session**: Top-level container, persists after detach
- **Window**: Tab within a session
- **Pane**: Split within a window

### Essential Commands

| Command | Description |
|---------|-------------|
| `tmux` | Start new session |
| `tmux new -s name` | Start named session |
| `tmux attach -t name` | Attach to session |
| `tmux ls` | List sessions |
| `tmux kill-session -t name` | Kill session |

### Default Key Bindings

Prefix: `Ctrl+b` (then release, then press next key)

| Keys | Action |
|------|--------|
| `c` | New window |
| `n` / `p` | Next / previous window |
| `0-9` | Go to window N |
| `%` | Split vertically |
| `"` | Split horizontally |
| `o` | Next pane |
| `arrow` | Navigate panes |
| `d` | Detach |
| `x` | Kill pane |
| `z` | Toggle pane zoom |
| `[` | Enter copy mode |
| `:` | Command prompt |

### Configuration (~/.tmux.conf)

```bash
# Better prefix
unbind C-b
set -g prefix C-a
bind C-a send-prefix

# Mouse support
set -g mouse on

# Start windows/panes at 1
set -g base-index 1
setw -g pane-base-index 1

# True color support
set -g default-terminal "tmux-256color"
set -ga terminal-overrides ",*256col*:Tc"

# Faster escape
set -sg escape-time 10

# More history
set -g history-limit 50000

# Vim-style pane navigation
bind h select-pane -L
bind j select-pane -D
bind k select-pane -U
bind l select-pane -R

# Easy splits (current directory)
bind | split-window -h -c "#{pane_current_path}"
bind - split-window -v -c "#{pane_current_path}"

# Reload config
bind r source-file ~/.tmux.conf \; display "Config reloaded"

# Graphics passthrough (for kitty/sixel)
set -g allow-passthrough on
```

### Session Scripting

```bash
#!/bin/bash
# dev-session.sh - Create development environment

tmux new-session -d -s dev -n editor
tmux send-keys -t dev:editor 'vim' C-m

tmux new-window -t dev -n server
tmux send-keys -t dev:server 'npm run dev' C-m

tmux new-window -t dev -n git
tmux send-keys -t dev:git 'git status' C-m

tmux select-window -t dev:editor
tmux attach -t dev
```

### Remote Workflow

```bash
# On remote server
tmux new -s work

# Do work, then detach
# Ctrl+b d

# From any other machine
ssh server
tmux attach -t work
# Session restored exactly as left
```

## Zellij

Modern multiplexer with better UX and WebAssembly plugins.

### Installation

```bash
# macOS
brew install zellij

# Cargo
cargo install zellij

# Binary
# Download from https://github.com/zellij-org/zellij/releases
```

### Key Features

- Discoverable UI with on-screen hints
- WebAssembly plugin system
- Layout files for session templating
- Built-in session manager
- Floating panes

### Default Key Bindings

Zellij uses modes (like vim). Default mode shows hints at bottom.

| Keys | Action |
|------|--------|
| `Ctrl+p` then `n` | New pane |
| `Ctrl+p` then `d` | Pane down |
| `Ctrl+p` then `r` | Pane right |
| `Ctrl+p` then `x` | Close pane |
| `Ctrl+t` then `n` | New tab |
| `Ctrl+t` then `1-9` | Go to tab |
| `Alt+arrow` | Navigate panes |
| `Ctrl+s` then `d` | Detach |
| `Ctrl+q` | Quit |

### Configuration

Location: `~/.config/zellij/config.kdl`

```kdl
// Theme
theme "catppuccin-mocha"

// Default shell
default_shell "zsh"

// Copy on select
copy_on_select true

// Simplified UI
simplified_ui true
pane_frames false

// Keybindings
keybinds {
    normal {
        bind "Alt h" { MoveFocus "Left"; }
        bind "Alt l" { MoveFocus "Right"; }
        bind "Alt j" { MoveFocus "Down"; }
        bind "Alt k" { MoveFocus "Up"; }
    }
}
```

### Layout Files

```kdl
// ~/.config/zellij/layouts/dev.kdl
layout {
    pane split_direction="vertical" {
        pane command="vim"
        pane split_direction="horizontal" {
            pane command="npm" {
                args "run" "dev"
            }
            pane
        }
    }
}
```

Start with layout:
```bash
zellij --layout dev
```

### Session Management

```bash
zellij ls                    # List sessions
zellij attach session-name   # Attach to session
zellij kill-session name     # Kill session
zellij delete-session name   # Delete session
```

## Integrated Terminal Multiplexing

Modern terminals include native multiplexing without external tools.

### Kitty Native Multiplexing

Hierarchy: OS Windows > Tabs > Windows (panes)

```conf
# ~/.config/kitty/kitty.conf
enabled_layouts splits,stack,grid

map ctrl+shift+enter new_window
map ctrl+shift+t new_tab
map ctrl+shift+l next_layout
map ctrl+alt+enter launch --location=vsplit
```

Remote control:
```bash
kitty -o allow_remote_control=yes
kitten @ launch --type=window htop
```

Limitation: No session persistence (closing Kitty kills sessions).

### WezTerm Multiplexing

```lua
-- ~/.wezterm.lua
config.keys = {
  { key = 's', mods = 'CTRL|SHIFT', action = wezterm.action.SplitHorizontal { domain = 'CurrentPaneDomain' } },
  { key = 'v', mods = 'CTRL|SHIFT', action = wezterm.action.SplitVertical { domain = 'CurrentPaneDomain' } },
}

-- Unix domain for session persistence
config.unix_domains = { { name = 'unix' } }
```

### Ghostty Multiplexing

Built-in splits and tabs without external multiplexer:

```
keybind = ctrl+shift+h=split_horizontal
keybind = ctrl+shift+v=split_vertical
keybind = ctrl+shift+t=new_tab
```

### iTerm2 + tmux Integration

Best of both worlds: tmux persistence with native iTerm2 UI.

```bash
tmux -CC            # Start with iTerm2 integration
tmux -CC attach     # Reattach with iTerm2 integration
```

tmux windows appear as native iTerm2 tabs. Full iTerm2 features available.

## Choosing a Solution

### Use tmux when:

- Working on remote servers via SSH
- Need session persistence (survive disconnects)
- Want consistent experience across different machines
- Need to share sessions with colleagues
- Resource-constrained environments

### Use Zellij when:

- Want modern UX with discoverable keybindings
- Learning multiplexing for the first time
- Want plugin extensibility (WebAssembly)
- Prefer visual layout configuration
- Don't need maximum remote compatibility

### Use native terminal multiplexing when:

- Primarily local development
- Want GPU-accelerated rendering everywhere
- Prefer native look and feel
- Don't need session persistence
- Value simplicity over features

### Hybrid approach:

- **Local dev**: Native terminal splits (Kitty/WezTerm/Ghostty)
- **Remote work**: tmux (universally available)
- **Complex local**: Zellij (better than tmux for local use)

## Tips

### tmux + Modern Terminals

Enable graphics passthrough for Kitty/Sixel:
```bash
set -g allow-passthrough on
```

### Nesting Multiplexers

Avoid running tmux inside tmux. If needed, change inner prefix:
```bash
bind -n C-s send-prefix  # Send to inner tmux
```

### Starting Multiplexer Automatically

Alacritty:
```toml
[terminal]
shell = { program = "zellij", args = ["-l", "welcome"] }
```

Bash/Zsh (only in interactive non-nested sessions):
```bash
if [[ -z "$TMUX" && -z "$ZELLIJ" ]]; then
  tmux attach || tmux new
fi
```
