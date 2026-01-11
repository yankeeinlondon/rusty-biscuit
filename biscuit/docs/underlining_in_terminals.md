# Terminal Support for Underlining and How to Render It

Modern support for advanced underlining in terminals is currently **excellent**—provided you are using a "modern" emulator. The industry has largely converged on a set of extensions originally popularized by the **Kitty** terminal and coordinated with VTE.

Most high-end terminal emulators now support not just the standard single underline, but also double, curly (squiggly), dotted, and dashed styles, along with independent colors for those underlines.

## 1. Supported Underline Styles

The standard ANSI escape sequence for underlining is `\e[4m`. Modern terminals extend this using a "sub-parameter" syntax (separated by **colons**, not semicolons) to specify the style:

| Style | Escape Sequence | Common Use Case |
| --- | --- | --- |
| **No Underline** | `\e[4:0m` | Resetting specific styles |
| **Straight** | `\e[4:1m` | Standard emphasis |
| **Double** | `\e[4:2m` | Strong emphasis or headers |
| **Curly / Squiggly** | `\e[4:3m` | LSP errors/warnings (the "spellcheck" look) |
| **Dotted** | `\e[4:4m` | Information or hints |
| **Dashed** | `\e[4:5m` | Grammar or stylistic suggestions |

**Important:** Colons (`:`) must be used as separators, not semicolons (`;`). Using semicolons would be interpreted as separate SGR codes (e.g., `\e[4;2m` means "underline AND dim", not "double underline").

Backwards-compatible codes:

- `\e[4m` — straight underline (equivalent to `\e[4:1m`)
- `\e[24m` — disable underline (equivalent to `\e[4:0m`)

---

## 2. Underline Coloring

One of the best modern features is the ability to color the underline independently of the text. This uses SGR code 58, following the same format as foreground (38) and background (48) colors:

- **RGB Color:** `\e[58:2::R:G:Bm` (e.g., `\e[58:2::255:0:0m` for a red underline)
- **256-Color Palette:** `\e[58:5:Nm` where N is 0–255
- **Reset Color:** `\e[59m` (returns the underline to the foreground text color)

The double colon (`::`) in the RGB format is intentional—it follows ISO 8613-6, where the format is `58:2:<colorspace>:R:G:B`. When the colorspace identifier is omitted, the double colon remains.

---

## 3. Terminfo Detection

The terminfo database provides capabilities for detecting underline support:

- **Basic underline:** `smul` (EnterUnderlineMode) and `rmul` (ExitUnderlineMode)
- **Extended styles:** `Su` boolean capability (non-standard, Kitty extension)
- **Underline color:** `Setulc` string capability (non-standard)

Note: The `Su` and `Setulc` capabilities are not part of the ncurses standard terminfo and may not be present in older terminfo databases, even for terminals that support these features.

---

## 4. Terminal Support (2026 Status)

Support is now the "standard" for developer-focused terminals. If you use any of the following, these features should work out of the box:

- **Cross-Platform:** Kitty, WezTerm, Alacritty, Ghostty, Contour
- **macOS:** iTerm2 (since 3.4)
- **Windows:** Windows Terminal (since 2023), WezTerm
- **Linux:** GNOME Terminal (VTE 0.76+), Konsole, Foot, Tilix
- **Multiplexers:** **tmux** supports this, but you often need to "tell" it your terminal is capable by adding `set -as terminal-overrides` to your `.tmux.conf`

### Terminals with Limited Support

- **Konsole:** Supports colored underlines but not all style variants (no curly/dotted/dashed as of 2024)
- **Apple Terminal:** Basic underline only; no styled variants or colored underlines

---

## 5. How to Test Your Terminal

You can run this one-liner in your terminal to see if it supports the various styles:

```bash
printf '\e[4:1mStraight\e[0m \e[4:2mDouble\e[0m \e[4:3mCurly\e[0m \e[4:4mDotted\e[0m \e[4:5mDashed\e[0m\n'

```

To test a **red squiggly** underline:

```bash
printf '\e[4:3m\e[58:2::255:0:0mRed Squiggly\e[0m\n'

```

## 6. Limitations to Watch For

- **SSH/Old Environments:** If you are SSHing into an older server, the `TERM` definition might not know about these sequences, though modern tools like Neovim usually handle this gracefully.
- **Font Rendering:** Extremely small font sizes can sometimes make "dotted" and "dashed" look identical because there aren't enough pixels to draw the gaps.
- **Fallback Behavior:** Terminals that don't support extended styles will typically ignore the sub-parameter and render a straight underline, providing graceful degradation.
- **tmux Pass-through:** tmux versions before 3.0 may not correctly pass through underline escape sequences; ensure `terminal-overrides` is configured.
