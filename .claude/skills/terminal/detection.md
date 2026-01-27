# Terminal Feature Detection

Reliable terminal detection enables graceful degradation and optimal feature usage.

## Environment Variables

### Primary Detection Variables

| Variable | Purpose | Example Values |
|----------|---------|----------------|
| `TERM` | Terminal type | `xterm-256color`, `xterm-kitty`, `screen` |
| `TERM_PROGRAM` | Terminal application | `iTerm.app`, `WezTerm`, `Apple_Terminal` |
| `TERM_PROGRAM_VERSION` | Application version | `3.4.19` |
| `COLORTERM` | Color capability | `truecolor`, `24bit` |

### Terminal-Specific Variables

| Variable | Terminal |
|----------|----------|
| `KITTY_WINDOW_ID` | Kitty |
| `ITERM_SESSION_ID` | iTerm2 |
| `WEZTERM_PANE` | WezTerm |
| `ALACRITTY_SOCKET` | Alacritty |
| `WT_SESSION` | Windows Terminal |
| `GHOSTTY_RESOURCES_DIR` | Ghostty |

## TypeScript Detection Module

```typescript
export interface TerminalInfo {
  name: string;
  version?: string;
  capabilities: {
    colors: 0 | 16 | 256 | 'truecolor';
    hyperlinks: boolean;
    kittyGraphics: boolean;
    iterm2Graphics: boolean;
    sixel: boolean;
    unicode: boolean;
    mouse: boolean;
    bracketed_paste: boolean;
  };
  isMultiplexed: boolean;
  multiplexer?: 'tmux' | 'screen' | 'zellij';
}

export function detectTerminal(): TerminalInfo {
  const env = process.env;
  const term = env.TERM || '';
  const termProgram = env.TERM_PROGRAM || '';
  const colorterm = env.COLORTERM || '';

  // Detect multiplexer
  const isMultiplexed = !!(env.TMUX || env.STY || env.ZELLIJ);
  const multiplexer = env.TMUX ? 'tmux' : env.STY ? 'screen' : env.ZELLIJ ? 'zellij' : undefined;

  // Detect terminal application
  let name = 'unknown';
  let version: string | undefined;

  if (term === 'xterm-kitty' || env.KITTY_WINDOW_ID) {
    name = 'kitty';
  } else if (termProgram === 'iTerm.app') {
    name = 'iterm2';
    version = env.TERM_PROGRAM_VERSION;
  } else if (termProgram === 'WezTerm' || env.WEZTERM_PANE) {
    name = 'wezterm';
    version = env.TERM_PROGRAM_VERSION;
  } else if (env.ALACRITTY_SOCKET) {
    name = 'alacritty';
  } else if (env.WT_SESSION) {
    name = 'windows-terminal';
  } else if (env.GHOSTTY_RESOURCES_DIR) {
    name = 'ghostty';
  } else if (termProgram === 'Apple_Terminal') {
    name = 'macos-terminal';
  } else if (term.includes('xterm')) {
    name = 'xterm-compatible';
  }

  // Detect color support
  let colors: 0 | 16 | 256 | 'truecolor' = 0;
  if (colorterm === 'truecolor' || colorterm === '24bit') {
    colors = 'truecolor';
  } else if (term.includes('256color')) {
    colors = 256;
  } else if (term.includes('color') || term.includes('ansi')) {
    colors = 16;
  }

  // Build capabilities
  const capabilities = {
    colors,
    hyperlinks: colors !== 0, // Most modern terminals support OSC 8
    kittyGraphics: name === 'kitty' || name === 'wezterm',
    iterm2Graphics: name === 'iterm2' || name === 'wezterm',
    sixel: name === 'wezterm' || name === 'iterm2', // Conservative default
    unicode: true, // Assume modern systems support Unicode
    mouse: colors !== 0,
    bracketed_paste: colors !== 0,
  };

  return {
    name,
    version,
    capabilities,
    isMultiplexed,
    multiplexer,
  };
}
```

## Rust Detection Module

```rust
use std::env;

#[derive(Debug, Clone)]
pub enum ColorSupport {
    None,
    Basic16,
    Extended256,
    TrueColor,
}

#[derive(Debug, Clone)]
pub enum Multiplexer {
    Tmux,
    Screen,
    Zellij,
}

#[derive(Debug, Clone)]
pub struct TerminalInfo {
    pub name: String,
    pub color_support: ColorSupport,
    pub hyperlinks: bool,
    pub kitty_graphics: bool,
    pub iterm2_graphics: bool,
    pub sixel: bool,
    pub multiplexer: Option<Multiplexer>,
}

impl TerminalInfo {
    pub fn detect() -> Self {
        let term = env::var("TERM").unwrap_or_default();
        let term_program = env::var("TERM_PROGRAM").unwrap_or_default();
        let colorterm = env::var("COLORTERM").unwrap_or_default();

        // Detect multiplexer
        let multiplexer = if env::var("TMUX").is_ok() {
            Some(Multiplexer::Tmux)
        } else if env::var("STY").is_ok() {
            Some(Multiplexer::Screen)
        } else if env::var("ZELLIJ").is_ok() {
            Some(Multiplexer::Zellij)
        } else {
            None
        };

        // Detect terminal
        let name = if term == "xterm-kitty" || env::var("KITTY_WINDOW_ID").is_ok() {
            "kitty".to_string()
        } else if term_program == "iTerm.app" {
            "iterm2".to_string()
        } else if term_program == "WezTerm" || env::var("WEZTERM_PANE").is_ok() {
            "wezterm".to_string()
        } else if env::var("ALACRITTY_SOCKET").is_ok() {
            "alacritty".to_string()
        } else if env::var("WT_SESSION").is_ok() {
            "windows-terminal".to_string()
        } else if env::var("GHOSTTY_RESOURCES_DIR").is_ok() {
            "ghostty".to_string()
        } else {
            "unknown".to_string()
        };

        // Detect color support
        let color_support = if colorterm == "truecolor" || colorterm == "24bit" {
            ColorSupport::TrueColor
        } else if term.contains("256color") {
            ColorSupport::Extended256
        } else if term.contains("color") || term.contains("ansi") {
            ColorSupport::Basic16
        } else {
            ColorSupport::None
        };

        let has_colors = !matches!(color_support, ColorSupport::None);

        Self {
            name: name.clone(),
            color_support,
            hyperlinks: has_colors,
            kitty_graphics: name == "kitty" || name == "wezterm",
            iterm2_graphics: name == "iterm2" || name == "wezterm",
            sixel: name == "wezterm" || name == "iterm2",
            multiplexer,
        }
    }
}
```

## Query-Based Detection

Some features require querying the terminal and parsing responses.

### DA1 (Primary Device Attributes)

Query: `\x1b[c`

Response: `\x1b[?` followed by parameters

Common response codes:
- `4` - Sixel graphics
- `22` - ANSI color

### DA2 (Secondary Device Attributes)

Query: `\x1b[>c`

Response contains terminal identification.

### XTVERSION

Query: `\x1b[>0q`

Returns terminal name and version.

### Background Color Query

Query: `\x1b]11;?\x1b\\`

Returns current background color (for light/dark detection).

## Terminal-Specific Quirks

### tmux Passthrough

Graphics protocols often require passthrough mode in tmux:

```
# Enable in tmux.conf
set -g allow-passthrough on
```

Wrap escape sequences:
```
\x1bPtmux;\x1b ACTUAL_ESCAPE \x1b\\
```

### SSH Sessions

When connected via SSH:
- `SSH_CONNECTION` is set
- Graphics protocols may have latency issues
- Consider reducing image quality/size

### CI/CD Environments

Detect non-interactive environments:
```typescript
const isCI = !!(
  process.env.CI ||
  process.env.CONTINUOUS_INTEGRATION ||
  process.env.GITHUB_ACTIONS ||
  process.env.GITLAB_CI ||
  process.env.JENKINS_URL
);

const isTTY = process.stdout.isTTY;
```

## Best Practices

1. **Check `isTTY`** before using any escape sequences
2. **Detect CI environments** and disable fancy output
3. **Provide `NO_COLOR` support** (https://no-color.org)
4. **Fall back gracefully** when features unavailable
5. **Cache detection results** - environment doesn't change during runtime
6. **Consider `FORCE_COLOR`** for testing colored output in CI

### NO_COLOR / FORCE_COLOR Support

```typescript
function shouldUseColor(): boolean {
  // NO_COLOR takes precedence
  if (process.env.NO_COLOR !== undefined) {
    return false;
  }

  // FORCE_COLOR overrides TTY check
  if (process.env.FORCE_COLOR !== undefined) {
    return process.env.FORCE_COLOR !== '0';
  }

  // Default: use color only if TTY
  return process.stdout.isTTY === true;
}
```
