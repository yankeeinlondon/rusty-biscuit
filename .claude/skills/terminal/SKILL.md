---
name: terminal
description: Expert knowledge for modern terminal emulators covering escape codes (CSI/OSC/SGR), graphics protocols (Sixel, Kitty, iTerm2), feature detection, configuration (Alacritty/Kitty/WezTerm/iTerm2/Warp/Ghostty), and multiplexing (tmux/Zellij). Use when building CLI apps with colors/styles, inline images, progress bars, terminal detection, or configuring terminals.
---

# Terminal Emulator Expert Knowledge

Modern terminals support rich features beyond plain text: 256/true colors, hyperlinks, inline images, and styled output. This skill covers escape sequences, graphics protocols, terminal configuration, and multiplexing.

## Quick Reference

### ANSI Escape Codes

```
CSI = "\x1b["   Control Sequence Introducer
OSC = "\x1b]"   Operating System Command
SGR = CSI...m   Select Graphic Rendition (styling)
```

Common SGR codes: `0` reset, `1` bold, `3` italic, `4` underline, `7` inverse, `9` strikethrough, `38;5;N` fg256, `48;5;N` bg256, `38;2;R;G;B` fgRGB, `48;2;R;G;B` bgRGB

### Hyperlinks (OSC 8)

```
\x1b]8;;URL\x1b\\TEXT\x1b]8;;\x1b\\
```

### Terminal Detection

```typescript
const term = process.env.TERM_PROGRAM || process.env.TERM || '';
const colorterm = process.env.COLORTERM || '';
const trueColor = colorterm === 'truecolor' || colorterm === '24bit';
const isKitty = term === 'xterm-kitty';
const isITerm = term === 'iTerm.app';
const isWezTerm = process.env.TERM_PROGRAM === 'WezTerm';
```

### Graphics Protocol Selection

| Protocol | Detection | Best For |
| -------- | --------- | -------- |
| Kitty | `TERM=xterm-kitty` | High-res, animations |
| iTerm2 | `TERM_PROGRAM=iTerm.app` | macOS inline images |
| Sixel | Query `\x1b[c` | Wide compatibility |

## Detailed Documentation

- [Escape Codes Reference](escape-codes.md) - Complete CSI/OSC/SGR sequences
- [Graphics Protocols](graphics-protocols.md) - Sixel, Kitty, iTerm2 image display
- [Terminal Detection](detection.md) - Feature detection and capability queries
- [TypeScript Examples](typescript-examples.md) - Node.js terminal utilities
- [Rust Examples](rust-examples.md) - crossterm, termcolor implementations
- [Terminal Configuration](configuration.md) - Alacritty, Kitty, WezTerm, iTerm2, Warp, Ghostty
- [Multiplexing](multiplexing.md) - tmux, Zellij, and integrated solutions

## Common Patterns

### Styled Output (TypeScript)

```typescript
const style = (text: string, codes: number[]) =>
  `\x1b[${codes.join(';')}m${text}\x1b[0m`;

console.log(style('Error', [1, 31]));  // Bold red
console.log(style('Link', [4, 34]));   // Underlined blue
```

### Styled Output (Rust)

```rust
use crossterm::style::{Color, Stylize};
println!("{}", "Error".red().bold());
println!("{}", "Success".green());
```

### Progress Bar Pattern

```typescript
const progressBar = (percent: number, width = 40) => {
  const filled = Math.round(width * percent / 100);
  const bar = '\u2588'.repeat(filled) + '\u2591'.repeat(width - filled);
  process.stdout.write(`\r${bar} ${percent}%`);
};
```

### Hyperlink (Cross-Platform)

```typescript
const hyperlink = (text: string, url: string) =>
  `\x1b]8;;${url}\x1b\\${text}\x1b]8;;\x1b\\`;
```

## Feature Support Matrix

| Feature | Kitty | iTerm2 | WezTerm | Alacritty | Ghostty | Warp |
| ------- | ----- | ------ | ------- | --------- | ------- | ---- |
| True Color | Yes | Yes | Yes | Yes | Yes | Yes |
| OSC 8 Links | Yes | Yes | Yes | Yes | Yes | Yes |
| Sixel | No | Yes | Yes | No | Yes | No |
| Kitty Graphics | Yes | No | Yes | No | No | No |
| iTerm2 Images | No | Yes | Yes | No | No | No |
| Native Tabs | Yes | Yes | Yes | No | Yes | Yes |
| Native Splits | Yes | Yes | Yes | No | Yes | Yes |

## Multiplexer Comparison

| Feature | tmux | Zellij | Kitty | WezTerm |
| ------- | ---- | ------ | ----- | ------- |
| Session Persist | Yes | Yes | No | No |
| Layout Config | Yes | Yes | Yes | Yes |
| Plugin System | No | Yes | Python | Lua |
| GPU Render | No | No | Yes | Yes |
| Remote Attach | Yes | Yes | No | No |

## When to Use What

- **CLI colors/styles**: Use SGR codes with feature detection
- **Inline images**: Detect terminal, use appropriate protocol
- **Progress bars**: CSI cursor control + Unicode blocks
- **Session persistence**: tmux or Zellij
- **Local dev splits**: Native terminal tabs/splits or Zellij
- **Remote work**: tmux (universally available)
