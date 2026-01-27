# Terminal Escape Codes Reference

## Escape Sequence Types

### CSI - Control Sequence Introducer

Format: `ESC [` followed by parameters and a final byte.

```
ESC = \x1b (hex) or \033 (octal)
CSI = ESC [
```

### OSC - Operating System Command

Format: `ESC ]` followed by command number, parameters, and terminator.

```
OSC = ESC ]
Terminator = BEL (\x07) or ST (ESC \)
```

### SGR - Select Graphic Rendition

Format: `CSI parameters m`

## SGR Attribute Codes

### Basic Attributes

| Code | Effect | Reset |
|------|--------|-------|
| 0 | Reset all attributes | - |
| 1 | Bold/Bright | 22 |
| 2 | Dim/Faint | 22 |
| 3 | Italic | 23 |
| 4 | Underline | 24 |
| 5 | Slow blink | 25 |
| 6 | Rapid blink | 25 |
| 7 | Inverse/Reverse | 27 |
| 8 | Hidden/Conceal | 28 |
| 9 | Strikethrough | 29 |

### Standard Colors (Foreground)

| Code | Color |
|------|-------|
| 30 | Black |
| 31 | Red |
| 32 | Green |
| 33 | Yellow |
| 34 | Blue |
| 35 | Magenta |
| 36 | Cyan |
| 37 | White |
| 39 | Default |

### Standard Colors (Background)

| Code | Color |
|------|-------|
| 40 | Black |
| 41 | Red |
| 42 | Green |
| 43 | Yellow |
| 44 | Blue |
| 45 | Magenta |
| 46 | Cyan |
| 47 | White |
| 49 | Default |

### Bright Colors

Foreground: 90-97 (bright black through bright white)
Background: 100-107 (bright black through bright white)

### 256-Color Mode

```
Foreground: CSI 38;5;N m   (N = 0-255)
Background: CSI 48;5;N m   (N = 0-255)
```

Color ranges:
- 0-7: Standard colors
- 8-15: Bright colors
- 16-231: 6x6x6 color cube (216 colors)
- 232-255: Grayscale (24 shades)

### True Color (24-bit RGB)

```
Foreground: CSI 38;2;R;G;B m
Background: CSI 48;2;R;G;B m
```

## Cursor Control

| Sequence | Effect |
|----------|--------|
| `CSI n A` | Cursor up n lines |
| `CSI n B` | Cursor down n lines |
| `CSI n C` | Cursor forward n columns |
| `CSI n D` | Cursor back n columns |
| `CSI n E` | Cursor to next line, n lines down |
| `CSI n F` | Cursor to previous line, n lines up |
| `CSI n G` | Cursor to column n |
| `CSI n ; m H` | Cursor to row n, column m |
| `CSI s` | Save cursor position |
| `CSI u` | Restore cursor position |

## Erase Operations

| Sequence | Effect |
|----------|--------|
| `CSI 0 J` | Clear from cursor to end of screen |
| `CSI 1 J` | Clear from start of screen to cursor |
| `CSI 2 J` | Clear entire screen |
| `CSI 3 J` | Clear entire screen + scrollback |
| `CSI 0 K` | Clear from cursor to end of line |
| `CSI 1 K` | Clear from start of line to cursor |
| `CSI 2 K` | Clear entire line |

## Screen Modes

| Sequence | Effect |
|----------|--------|
| `CSI ? 25 h` | Show cursor |
| `CSI ? 25 l` | Hide cursor |
| `CSI ? 1049 h` | Enable alternate screen buffer |
| `CSI ? 1049 l` | Disable alternate screen buffer |
| `CSI ? 2004 h` | Enable bracketed paste mode |
| `CSI ? 2004 l` | Disable bracketed paste mode |

## OSC Sequences

### OSC 8 - Hyperlinks

```
OSC 8 ; params ; URI ST text OSC 8 ; ; ST
```

Example:
```
\x1b]8;;https://example.com\x1b\\Click here\x1b]8;;\x1b\\
```

With ID parameter (for multi-line links):
```
\x1b]8;id=mylink;https://example.com\x1b\\Click\x1b]8;;\x1b\\
```

### OSC 52 - Clipboard Operations

```
OSC 52 ; c ; base64-data ST   (Set clipboard)
OSC 52 ; c ; ? ST              (Query clipboard)
```

### OSC 4 - Set/Query Color Palette

```
OSC 4 ; index ; spec ST        (Set color)
OSC 4 ; index ; ? ST           (Query color)
```

### OSC 10/11 - Foreground/Background Color

```
OSC 10 ; spec ST               (Set foreground)
OSC 11 ; spec ST               (Set background)
OSC 10 ; ? ST                  (Query foreground)
OSC 11 ; ? ST                  (Query background)
```

### OSC 1337 - iTerm2 Proprietary

```
OSC 1337 ; key=value ST
```

Common keys:
- `File=...` - Inline image
- `SetUserVar=...` - User variables
- `CurrentDir=...` - Current directory

## Terminal Type Detection

Check `$TERM` and `$TERM_PROGRAM` environment variables:

| Variable | Terminal |
|----------|----------|
| `TERM=xterm-kitty` | Kitty |
| `TERM_PROGRAM=iTerm.app` | iTerm2 |
| `TERM_PROGRAM=WezTerm` | WezTerm |
| `TERM_PROGRAM=Apple_Terminal` | macOS Terminal |
| `ALACRITTY_SOCKET` set | Alacritty |
| `WT_SESSION` set | Windows Terminal |

## Color Support Detection

```
COLORTERM=truecolor     # 24-bit color support
COLORTERM=24bit         # 24-bit color support
```

Check if terminal supports 256 colors:
```bash
[[ "$TERM" == *"256color"* ]]
```

## Example: Combining Sequences

Bold red text on yellow background:
```
\x1b[1;31;43mText\x1b[0m
```

True color gradient:
```
for i in range(256):
    print(f"\x1b[38;2;{i};0;{255-i}m#\x1b[0m", end="")
```

Save cursor, move, print, restore:
```
\x1b[s\x1b[10;20HHello\x1b[u
```
