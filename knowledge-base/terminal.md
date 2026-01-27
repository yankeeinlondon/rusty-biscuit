# Terminal Emulators and Escape Codes

A comprehensive guide to modern terminal emulators, escape codes, multiplexing, graphics protocols, and programmatic interaction for building rich command-line interfaces and terminal applications.

## Table of Contents

- [Introduction](#introduction)
- [Terminal Modes](#terminal-modes)
- [Escape Code Fundamentals](#escape-code-fundamentals)
- [Text Styling with SGR](#text-styling-with-sgr)
- [Cursor and Screen Control](#cursor-and-screen-control)
- [OSC Sequences](#osc-sequences)
- [Character Display and Unicode](#character-display-and-unicode)
- [Graphics Protocols](#graphics-protocols)
- [Terminal Emulators](#terminal-emulators)
- [Multiplexers](#multiplexers)
- [Code Examples](#code-examples)
- [Library Ecosystems](#library-ecosystems)
- [Resources](#resources)

## Introduction

Terminal escape codes are special byte sequences that instruct terminal emulators to perform actions beyond displaying literal text. These codes control cursor movement, text styling, colors, and advanced features like clickable hyperlinks and clipboard access.

All escape codes begin with the ASCII "Escape" character (ESC, decimal 27, hex 0x1B, often written as `\e` or `\x1b`), followed by additional characters specifying the command. The two primary sequence types are:

- **CSI (Control Sequence Introducer)**: Starts with `ESC[` - used for cursor movement, text styling, and screen manipulation
- **OSC (Operating System Command)**: Starts with `ESC]` - used for terminal-specific features like titles, hyperlinks, and clipboard

## Terminal Modes

Understanding terminal modes is essential for building interactive applications.

### Cooked Mode (Canonical Mode)

The default mode for most shells. The operating system processes input before applications receive it.

- **Line Buffering**: Programs receive text only after Enter is pressed
- **Line Editing**: Kernel handles Backspace, Delete, and other editing keys
- **Signal Processing**: `Ctrl+C` generates `SIGINT`, `Ctrl+Z` generates `SIGTSTP`

### Raw Mode (Non-Canonical Mode)

All processing is disabled. Every keystroke is passed directly to the program.

- **No Buffering**: Each key press is received instantly
- **No Editing**: Backspace sends its character code to the program
- **No Signals**: `Ctrl+C` and `Ctrl+Z` are passed as raw data

### Mode Comparison

| Feature | Cooked | Cbreak | Raw |
|---------|--------|--------|-----|
| **Input received** | After Enter | Instantly | Instantly |
| **Backspace works** | Yes (OS) | No | No |
| **Ctrl+C kills app** | Yes | Yes | No |
| **Common Use** | Shells | Menus | Text Editors |

## Escape Code Fundamentals

### Basic Structure

Escape codes always begin with ESC (0x1B) followed by control characters:

- Bash: `\e` or `\033`
- TypeScript/JavaScript: `\x1b`
- Octal: `\033`

### CSI Sequence Format

```
ESC [ <parameters> <command-letter>
```

Parameters are typically semicolon-separated numbers:
- `ESC[1;31m` = Bold (1) + Red foreground (31)
- `ESC[5;10H` = Move cursor to row 5, column 10

### OSC Sequence Format

```
ESC ] <code> ; <data> <terminator>
```

Terminators:
- `BEL` (0x07, `\a`)
- `ST` (String Terminator: `ESC\`)

## Text Styling with SGR

SGR (Select Graphic Rendition) controls text formatting using `ESC[...m`.

### Basic Attributes

| Attribute | Code | Reset | Notes |
|-----------|------|-------|-------|
| Reset | 0 | - | Reset all attributes |
| Bold | 1 | 22 | May render as bright color |
| Faint/Dim | 2 | 22 | Limited support |
| Italic | 3 | 23 | Not widely supported |
| Underline | 4 | 24 | Well supported |
| Blink | 5 | 25 | Rarely supported |
| Inverse | 7 | 27 | Swap FG/BG |
| Strikethrough | 9 | 29 | Modern terminals |

### Color Codes

**Basic 8 Colors:**

| Color | Foreground | Background |
|-------|------------|------------|
| Black | 30 | 40 |
| Red | 31 | 41 |
| Green | 32 | 42 |
| Yellow | 33 | 43 |
| Blue | 34 | 44 |
| Magenta | 35 | 45 |
| Cyan | 36 | 46 |
| White | 37 | 47 |

**Bright Colors:** Foreground 90-97, Background 100-107

**256-color palette (8-bit):**
```
ESC[38;5;<n>m   # Foreground (n = 0-255)
ESC[48;5;<n>m   # Background
```

**True color (24-bit):**
```
ESC[38;2;<R>;<G>;<B>m   # Foreground RGB
ESC[48;2;<R>;<G>;<B>m   # Background RGB
```

## Cursor and Screen Control

| Command | Code | Description |
|---------|------|-------------|
| Cursor Up | `ESC[<n>A` | Move up n lines |
| Cursor Down | `ESC[<n>B` | Move down n lines |
| Cursor Forward | `ESC[<n>C` | Move right n columns |
| Cursor Back | `ESC[<n>D` | Move left n columns |
| Cursor Position | `ESC[<row>;<col>H` | Move to position (1-indexed) |
| Erase Display | `ESC[<n>J` | Clear screen (0=end, 1=start, 2=all) |
| Erase Line | `ESC[<n>K` | Clear line (0=end, 1=start, 2=all) |
| Hide Cursor | `ESC[?25l` | Hide cursor |
| Show Cursor | `ESC[?25h` | Show cursor |
| Save Cursor | `ESC[s` | Save position |
| Restore Cursor | `ESC[u` | Restore position |

## OSC Sequences

### Hyperlinks (OSC 8)

Enables clickable hyperlinks in the terminal:

```
ESC]8;;URL\alink-text ESC]8;;\a
```

**Support:** iTerm2, GNOME Terminal, Windows Terminal (v1.4+), Kitty, WezTerm, Alacritty

### Clipboard Access (OSC 52)

Copy text to system clipboard (base64-encoded):

```
ESC]52;<clipboard>;<base64-data> BEL
```

- `c` = GUI clipboard
- `p` = primary selection (X11)

**Security:** Often disabled by default; may require terminal configuration.

### Window Title (OSC 0, 1, 2)

```
ESC]0;title\a    # Set title and icon
ESC]2;title\a    # Set title only
```

### Color Queries (OSC 4, 10, 11, 12)

- **OSC 4**: Query/set palette colors
- **OSC 10**: Default foreground color
- **OSC 11**: Default background color
- **OSC 12**: Cursor color

Query format: `ESC]<code>;?\a`

### Desktop Notifications (OSC 9)

```
ESC]9;message\a
```

**Support:** iTerm2, some Linux terminals

## Character Display and Unicode

### Multi-byte and Combining Characters

UTF-8 encodes characters as 1-4 bytes. Combining characters overlay base characters:
- "e" + combining acute accent = "e"
- Both render in one cell

### Grapheme Clusters and ZWJ

Zero-Width Joiner (U+200D) joins characters into graphemes:
- "farmer" (adult + ZWJ + ear of rice)
- Family emojis (multiple people joined with ZWJ)

**Mode 2027:** Proposed standard for terminals to handle grapheme clusters consistently.

| Terminal | Grapheme Width | Mode 2027 |
|----------|---------------|-----------|
| Ghostty | Correct (2) | Yes |
| iTerm2 | Correct (2) | No (always clusters) |
| WezTerm | Correct (2) | Yes |
| Kitty | Incorrect (4) | No |
| Alacritty | Incorrect (4) | No |

### Double Width Characters

CJK ideograms and some emoji occupy 2 columns. Terminals use `wcwidth` or Unicode East Asian Width properties.

## Graphics Protocols

### Sixel

Legacy bitmap graphics protocol from DEC terminals (1980s). Uses a palette-based encoding with 6-pixel vertical strips.

```
ESC P <params> q <sixel-data> ESC \
```

**Support:** Xterm (with `-ti vt340`), mlterm, mintty, WezTerm, foot

**Limitations:**
- Maximum 256 colors (with palette)
- No transparency
- Requires special encoding

### iTerm2 Image Protocol

```
ESC]1337;File=[params]:<base64-data>\a
```

**Parameters:**
- `inline=1`: Display inline
- `width=<N>`: Width in columns/pixels/percent
- `height=<N>`: Height specification
- `preserveAspectRatio=1`: Maintain aspect ratio

**Support:** iTerm2 (native), WezTerm

### Kitty Graphics Protocol

More advanced protocol with arbitrary pixel placement:

```
ESC_G <control-data>;<payload> ESC\
```

**Features:**
- RGBA and PNG support
- Compression (`o=z`)
- Animations
- Pixel-precise positioning

**Support:** Kitty (native), partial in Konsole

### Graphics Protocol Detection

```typescript
// Environment-based detection
function detectGraphicsProtocol(): string | null {
  const termProgram = process.env.TERM_PROGRAM || '';
  const term = process.env.TERM || '';

  if (termProgram === 'iTerm.app' || termProgram === 'WezTerm') {
    return 'iterm2';
  }
  if (term.includes('kitty')) {
    return 'kitty';
  }
  return null;
}
```

## Terminal Emulators

### Feature Comparison

| Feature | iTerm2 | Kitty | WezTerm | Alacritty | Ghostty | Warp |
|---------|--------|-------|---------|-----------|---------|------|
| **Platform** | macOS | Linux/macOS | Cross-platform | Cross-platform | Cross-platform | macOS/Linux |
| **GPU Rendering** | Yes | Yes | Yes | Yes | Yes | Yes |
| **Ligatures** | Yes | Yes | Yes | No | Yes | Yes |
| **Built-in Multiplex** | No | Yes | Yes | No | No | Yes |
| **Sixel** | No | No | Yes | No | No | No |
| **iTerm2 Images** | Yes | No | Yes | No | No | No |
| **Kitty Graphics** | No | Yes | Partial | No | No | No |
| **OSC 8 Links** | Yes | Yes | Yes | Yes | Yes | Yes |
| **OSC 52 Clipboard** | Yes | Yes | Yes | Yes | Yes | Yes |
| **True Color** | Yes | Yes | Yes | Yes | Yes | Yes |

### iTerm2

macOS-native terminal with extensive features.

**Configuration:** `~/Library/Preferences/com.googlecode.iterm2.plist` or JSON profiles

**Key Features:**
- Shell integration marks
- Inline images and badges
- Triggers and automatic profile switching
- tmux integration mode

### Kitty

GPU-accelerated with native multiplexing.

**Configuration:** `~/.config/kitty/kitty.conf`

```conf
enabled_layouts splits,stack,grid
font_family JetBrains Mono
font_size 12.0
map ctrl+shift+enter launch --location=vsplit
```

**Key Features:**
- Layouts: Splits, Stack, Grid, Tall, Fat
- Remote control via `kitten @` commands
- Kittens (Python extensions)

### WezTerm

Cross-platform with Lua configuration.

**Configuration:** `~/.wezterm.lua`

```lua
local wezterm = require 'wezterm'
local config = {}

config.font = wezterm.font 'JetBrains Mono'
config.color_scheme = 'Catppuccin Mocha'

config.keys = {
  { key = 's', mods = 'CTRL|SHIFT',
    action = wezterm.action.SplitHorizontal { domain = 'CurrentPaneDomain' } },
}

return config
```

**Key Features:**
- Multiplexer domains (SSH, WSL, local)
- Lua-based event system
- Workspaces and session management

### Alacritty

Minimal, fast terminal focused on simplicity.

**Configuration:** `~/.config/alacritty/alacritty.toml`

```toml
[font]
normal = { family = "JetBrains Mono" }
size = 12.0

[colors.primary]
background = "#1e1e2e"
foreground = "#cdd6f4"
```

**Key Features:**
- Vi mode for selection
- Hints for URL detection
- No built-in tabs/splits (use tmux)

### Ghostty

Modern terminal with native platform integration.

**Configuration:** `~/.config/ghostty/config`

```
font-family = JetBrains Mono
font-size = 12
theme = catppuccin-mocha
```

**Key Features:**
- Native macOS/GTK integration
- Mode 2027 grapheme cluster support
- Fast startup time

### Warp

AI-enhanced terminal with modern UI.

**Configuration:** `~/.warp/` directory

**Key Features:**
- Blocks-based command history
- AI command suggestions
- Built-in workflows
- Collaborative features

**Limitations:**
- Requires account login
- macOS/Linux only
- Some features require subscription

## Multiplexers

### tmux

The standard terminal multiplexer for persistent sessions.

**Configuration:** `~/.tmux.conf`

```bash
# Set prefix to Ctrl+a
set -g prefix C-a
unbind C-b

# Enable mouse support
set -g mouse on

# True color support
set -g default-terminal "tmux-256color"
set -ga terminal-overrides ",*256col*:Tc"

# OSC 52 clipboard
set -g set-clipboard on
```

**Key Bindings:**

| Action | Keys |
|--------|------|
| Split horizontal | `prefix "` |
| Split vertical | `prefix %` |
| New window | `prefix c` |
| Next window | `prefix n` |
| Navigate panes | `prefix arrow` |
| Detach | `prefix d` |

**Session Management:**
```bash
tmux new -s mysession     # Create named session
tmux attach -t mysession  # Attach to session
tmux ls                   # List sessions
```

### Zellij

Modern multiplexer written in Rust.

**Configuration:** `~/.config/zellij/config.kdl`

```kdl
keybinds {
    normal {
        bind "Ctrl a" { SwitchToMode "tmux"; }
    }
}

themes {
    catppuccin {
        fg "#cdd6f4"
        bg "#1e1e2e"
    }
}
```

**Key Features:**
- Session resurrection
- Layout system with KDL
- WebAssembly plugins
- Floating panes

**Environment Detection:**
```typescript
function isZellij(): boolean {
  return !!process.env.ZELLIJ;
}
```

### Multiplexer Escape Code Passthrough

When running inside tmux, escape codes must be wrapped:

```typescript
function wrapForTmux(code: string): string {
  if (!process.env.TMUX) return code;

  // Double ESC characters and wrap in DCS
  const doubled = code.replace(/\x1b/g, '\x1b\x1b');
  return `\x1bPtmux;${doubled}\x1b\\`;
}
```

### Native Terminal Multiplexing

Modern terminals (Kitty, WezTerm, Ghostty) provide built-in multiplexing.

**Kitty vs tmux:**

| Feature | Kitty (Native) | tmux |
|---------|---------------|------|
| Persistence | Lost on close | Persistent |
| Performance | GPU-accelerated | CPU-based |
| Graphics | Full protocol support | Requires passthrough |
| Scripting | Python (Kittens) | Shell scripts |

**WezTerm Domains:**

```lua
config.ssh_domains = {
  {
    name = 'dev-server',
    remote_address = 'user@dev.example.com',
  },
}

config.wsl_domains = {
  {
    name = 'WSL:Ubuntu',
    distribution = 'Ubuntu',
  },
}
```

## Code Examples

### TypeScript: Progress Dashboard

```typescript
import { stdout } from 'process';

const ESC = '\x1b';
const CSI = `${ESC}[`;
const CURSOR_HIDE = `${CSI}?25l`;
const CURSOR_SHOW = `${CSI}?25h`;
const CURSOR_UP = `${CSI}1A`;
const ERASE_LINE = `${CSI}2K`;
const CURSOR_LEFT = `${CSI}1G`;
const COLOR_GREEN = `${CSI}32m`;
const COLOR_BLUE = `${CSI}34m`;
const RESET = `${CSI}0m`;

function getProgressBar(percent: number, width: number = 20): string {
  const completeLen = Math.round(width * (percent / 100));
  const bar = '='.repeat(completeLen) + ' '.repeat(width - completeLen);
  return `[${bar}] ${percent}%`;
}

async function runDashboard() {
  stdout.write(CURSOR_HIDE);

  let progressA = 0;
  let progressB = 0;

  console.log('\n');

  const interval = setInterval(() => {
    if (progressA < 100) progressA += 2;
    if (progressB < 100) progressB += 5;

    stdout.write(CURSOR_UP + CURSOR_UP);
    stdout.write(ERASE_LINE + CURSOR_LEFT);
    stdout.write(`${COLOR_GREEN}Download A:${RESET} ${getProgressBar(progressA)}\n`);
    stdout.write(ERASE_LINE + CURSOR_LEFT);
    stdout.write(`${COLOR_BLUE}Download B:${RESET} ${getProgressBar(progressB)}\n`);

    if (progressA >= 100 && progressB >= 100) {
      clearInterval(interval);
      stdout.write(CURSOR_SHOW);
      console.log('\nAll tasks finished!');
    }
  }, 100);
}

process.on('SIGINT', () => {
  stdout.write(CURSOR_SHOW);
  process.exit();
});

runDashboard();
```

### TypeScript: Hyperlinks and Clipboard

```typescript
function printHyperlink(url: string, text: string): void {
  console.log(`\x1b]8;;${url}\x1b\\${text}\x1b]8;;\x1b\\`);
}

function copyToClipboard(text: string): void {
  const base64 = Buffer.from(text, 'utf-8').toString('base64');
  process.stdout.write(`\x1b]52;c;${base64}\x1b\\`);
}

function isMultiplexer(): boolean {
  const term = process.env.TERM || '';
  const tmux = process.env.TMUX || '';
  return !!(tmux || term.startsWith('screen') || term.startsWith('tmux'));
}

function wrapEscapeCode(code: string): string {
  if (!isMultiplexer()) return code;

  const doubled = code.replace(/\x1b/g, '\x1b\x1b');
  return `\x1bPtmux;${doubled}\x1b\\`;
}
```

### Rust: Progress Dashboard

```rust
use std::io::{self, Write};
use std::thread;
use std::time::Duration;

const CURSOR_HIDE: &str = "\x1b[?25l";
const CURSOR_SHOW: &str = "\x1b[?25h";
const CURSOR_UP: &str = "\x1b[1A";
const ERASE_LINE: &str = "\x1b[2K";
const CURSOR_LEFT: &str = "\x1b[1G";
const COLOR_GREEN: &str = "\x1b[32m";
const COLOR_BLUE: &str = "\x1b[34m";
const RESET: &str = "\x1b[0m";

fn get_progress_bar(percent: u32, width: usize) -> String {
    let p = percent.min(100);
    let complete_len = (width as u32 * p / 100) as usize;
    let bar = "=".repeat(complete_len) + &" ".repeat(width - complete_len);
    format!("[{}] {}%", bar, p)
}

fn main() {
    let stdout = io::stdout();
    let mut handle = stdout.lock();

    write!(handle, "{}", CURSOR_HIDE).unwrap();

    let mut progress_a = 0;
    let mut progress_b = 0;

    writeln!(handle, "\n").unwrap();

    loop {
        if progress_a < 100 { progress_a += 2; }
        if progress_b < 100 { progress_b += 5; }

        write!(handle, "{}{}", CURSOR_UP, CURSOR_UP).unwrap();
        write!(handle, "{}{}", ERASE_LINE, CURSOR_LEFT).unwrap();
        write!(handle, "{}Download A:{} {}\n",
               COLOR_GREEN, RESET, get_progress_bar(progress_a, 20)).unwrap();
        write!(handle, "{}{}", ERASE_LINE, CURSOR_LEFT).unwrap();
        write!(handle, "{}Download B:{} {}\n",
               COLOR_BLUE, RESET, get_progress_bar(progress_b, 20)).unwrap();

        handle.flush().unwrap();

        if progress_a >= 100 && progress_b >= 100 {
            break;
        }

        thread::sleep(Duration::from_millis(100));
    }

    write!(handle, "{}", CURSOR_SHOW).unwrap();
    handle.flush().unwrap();
    println!("\nAll tasks finished!");
}
```

### Rust: Hyperlinks and Multiplexer Detection

```rust
use std::env;
use std::io::{self, Write};

fn print_hyperlink(url: &str, text: &str) {
    println!("\x1b]8;;{}\x1b\\{}\x1b]8;;\x1b\\", url, text);
}

fn is_multiplexer() -> bool {
    let term = env::var("TERM").unwrap_or_default();
    let tmux = env::var("TMUX").unwrap_or_default();
    !tmux.is_empty() || term.starts_with("screen") || term.starts_with("tmux")
}

fn is_zellij() -> bool {
    env::var("ZELLIJ").is_ok()
}

fn wrap_escape_code(code: &str) -> String {
    if !is_multiplexer() {
        return code.to_string();
    }
    let doubled = code.replace("\x1b", "\x1b\x1b");
    format!("\x1bPtmux;{}\x1b\\", doubled)
}
```

### Rust: Raw Mode with stty

```rust
use std::io::{self, Read, Write};
use std::process::Command;

fn set_raw_mode(enable: bool) {
    if enable {
        Command::new("stty")
            .arg("raw")
            .arg("-echo")
            .status()
            .expect("Failed to set raw mode");
    } else {
        Command::new("stty")
            .arg("sane")
            .status()
            .expect("Failed to restore cooked mode");
    }
}

fn main() {
    println!("Press 'q' to quit, 'h' for hello");

    set_raw_mode(true);

    let mut stdin = io::stdin();
    let mut buffer = [0; 1];

    loop {
        match stdin.read(&mut buffer) {
            Ok(_) => {
                let key = buffer[0] as char;
                match key {
                    'q' => {
                        print!("\r\nExiting...\r\n");
                        break;
                    }
                    'h' => print!("\r\nHello World!\r\n"),
                    _ => print!("Pressed: {:?}\r", key),
                }
                io::stdout().flush().unwrap();
            }
            Err(_) => break,
        }
    }

    set_raw_mode(false);
}
```

## Library Ecosystems

### Rust Libraries

| Crate | Focus | Windows | Raw Mode |
|-------|-------|---------|----------|
| **crossterm** | All-in-one | Yes | Manual |
| **termion** | Pure Rust Unix | No | RAII |
| **console** | Styling & I/O | Yes | High-level |
| **anstyle** | Colors (no-std) | Yes | N/A |
| **terminal-hyperlink** | OSC 8 links | Yes | N/A |
| **terminal-colorsaurus** | Color queries | Yes | Internal |
| **vte** | Sequence parsing | Yes | N/A |

**crossterm Example:**

```rust
use crossterm::{
    execute,
    terminal::{enable_raw_mode, disable_raw_mode, EnterAlternateScreen},
    cursor, style::Stylize,
};
use std::io::stdout;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    enable_raw_mode()?;
    execute!(stdout(), EnterAlternateScreen, cursor::Hide)?;

    println!("{}", "Raw Mode Active!".blue().bold());

    disable_raw_mode()?;
    Ok(())
}
```

### TypeScript Libraries

| Library | Mode | Focus | Best For |
|---------|------|-------|----------|
| **chalk** | Cooked | Color & Styling | CLI logs |
| **ansi-escapes** | Both | Specific sequences | OSC 8, cursor |
| **terminal-kit** | Raw | Full TUI | Dashboards |
| **ink** | Raw | React TUI | Complex UIs |
| **blessed** | Raw | Curses-like | Widgets |

**chalk Example:**

```typescript
import chalk from 'chalk';

console.log(chalk.blue('Hello') + ' ' + chalk.red.bold('World'));
console.log(chalk.hex('#DEADED').underline('Custom color'));
```

**terminal-kit Example:**

```typescript
import { terminal } from 'terminal-kit';

terminal.grabInput(true);

terminal.on('key', (name: string) => {
    if (name === 'CTRL_C') terminal.processExit(0);
    terminal.green(`Pressed: ${name}\n`);
});

terminal.moveTo(10, 5).red('At (10, 5)');
```

## Resources

### Standards and Specifications

- [ECMA-48](https://ecma-international.org/publications-and-standards/standards/ecma-48/) - Control Functions for Coded Character Sets
- [Unicode Standard Annex #29](https://www.unicode.org/reports/tr29/) - Text Segmentation
- [X/Open Curses](https://publications.opengroup.org/c243-1) - Terminfo standard

### Documentation

- [Xterm Control Sequences](https://invisible-island.net/xterm/ctlseqs/ctlseqs.html) - Comprehensive reference
- [iTerm2 Escape Codes](https://iterm2.com/documentation-escape-codes.html)
- [Kitty Graphics Protocol](https://sw.kovidgoyal.net/kitty/graphics-protocol/)
- [OSC 8 Hyperlinks](https://gist.github.com/egmontkob/eb114294efbcd5adb1944c9f3cb5feda)

### Terminal Emulators

- [iTerm2](https://iterm2.com/) - macOS
- [Kitty](https://sw.kovidgoyal.net/kitty/) - Linux, macOS
- [WezTerm](https://wezfurlong.org/wezterm/) - Cross-platform
- [Alacritty](https://alacritty.org/) - Cross-platform
- [Ghostty](https://ghostty.org/) - Cross-platform
- [Warp](https://www.warp.dev/) - macOS, Linux
- [Windows Terminal](https://github.com/microsoft/terminal)

### Multiplexers

- [tmux](https://github.com/tmux/tmux) - Terminal multiplexer
- [Zellij](https://zellij.dev/) - Modern multiplexer in Rust

### Articles

- [Julia Evans on OSC 52](https://jvns.ca/til/vim-osc52/)
- [Mitchell Hashimoto on Grapheme Clusters](https://mitchellh.com/writing/grapheme-clusters-in-terminals)
- [Julia Evans on Escape Code Standards](https://jvns.ca/blog/2025/03/07/escape-code-standards/)
