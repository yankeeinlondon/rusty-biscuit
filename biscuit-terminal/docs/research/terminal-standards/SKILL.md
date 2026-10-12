---
name: terminal-standards
description: details on every terminal standard including OSC, ECMA, and modes.
---

# Terminal Standards

Research on the escape-sequence standards that terminal emulators and terminal applications rely on. Most of these are _informal_ (xterm-originated or emulator-specific), so each document covers how the sequence works, how to detect support, which terminals implement it, and where behavior diverges.

The research is grouped by kind of standard:

- [`other-standards/`](#ecma-48-and-other-standards): formal standards
- [`modes/`](#dec-private-modes): DEC private modes (`CSI ? Pm h` / `CSI ? Pm l`)
- [`osc/`](#osc-operating-system-commands): Operating System Commands (`OSC Ps ; Pt ST`)

Each document's frontmatter lists `apps_supporting`, which is the quickest way to see which terminals implement a given standard.

## ECMA-48 and Other Standards

| Document                                     | Covers                                                                                            |
| -------------------------------------------- | ------------------------------------------------------------------------------------------------- |
| [ECMA-48](other-standards/ECMA-48.md)        | ISO/IEC 6429: C0/C1 controls and the escape-sequence families behind cursor, screen, and SGR text formatting |

## DEC Private Modes

| Mode(s)                                                        | Purpose                                                            |
| -------------------------------------------------------------- | ------------------------------------------------------------------ |
| [9, 1000, 1002, 1003](modes/mode-9-1000-1002-1003.md)          | Mouse press, release, drag, and motion reporting                   |
| [1004](modes/mode-1004.md)                                     | Focus in/out reporting (`CSI I` / `CSI O`)                         |
| [1005, 1006, 1015, 1016](modes/mode-1005-1006-1015-1016.md)    | Mouse coordinate encodings (UTF-8, SGR, urxvt, SGR-pixels)         |
| [2004](modes/mode-2004.md)                                     | Bracketed paste                                                    |
| [2026](modes/mode-2026.md)                                     | Synchronized output (atomic screen updates)                        |
| [2027](modes/mode-2027.md)                                     | Grapheme cluster width handling                                    |
| [2031](modes/mode-2031.md)                                     | Color palette change notifications (with `CSI ? 996 n` dark/light query) |
| [2048](modes/mode-2048.md)                                     | In-band text-area resize notifications                             |

## OSC (Operating System Commands)

### Window and Identity

| OSC                                | Purpose                                                        |
| ---------------------------------- | -------------------------------------------------------------- |
| [0, 1, 2](osc/osc-0-1-2.md)        | Set icon name and/or window title                              |
| [7](osc/osc-7.md)                  | Report current working directory as a `file://` URI            |
| [22](osc/osc-22.md)                | Set the mouse pointer shape                                    |
| [50](osc/osc-50.md)                | Select or query the terminal font                              |
| [3008](osc/osc-3008.md)            | Announce nested contexts (shells, containers, VMs, remotes)    |

### Colors

| OSC                                        | Purpose                                                          |
| ------------------------------------------ | ---------------------------------------------------------------- |
| [4, 104](osc/osc-4-104.md)                 | Set, query, and reset indexed palette colors                     |
| [5, 105](osc/osc-5-105.md)                 | Special colors for bold, underline, blink, reverse, and italic   |
| [10, 11, 12](osc/osc-10-11-12.md)          | Default foreground, background, and cursor colors                |
| [13 to 19](osc/osc-13-to-19.md)            | Pointer, Tektronix, and selection colors (with resets)           |
| [21](osc/osc-21.md)                        | Kitty's string-keyed color protocol                              |

### Links, Clipboard, and Files

| OSC                          | Purpose                                                              |
| ---------------------------- | -------------------------------------------------------------------- |
| [8](osc/osc-8.md)            | Hyperlinks                                                           |
| [52](osc/osc-52.md)          | Clipboard access                                                     |
| [5522](osc/osc-5522.md)      | Kitty's extended clipboard (MIME types, large payloads)              |
| [5113](osc/osc-5113.md)      | Kitty's file transfer over the TTY                                   |
| [1337](osc/osc-1337.md)      | iTerm2 namespace: inline images, file transfer, user variables, badges, and more |

### Notifications and Progress

| OSC                                     | Purpose                                                                 |
| --------------------------------------- | ----------------------------------------------------------------------- |
| [9](osc/osc-9.md)                       | Desktop text notification (iTerm2 style)                                |
| [9;4](osc/osc-9-4-protocol.md)          | ConEmu / Windows Terminal progress indicator                            |
| [9;n other sub-commands](osc/osc-9-conemu.md) | The remaining ConEmu `OSC 9 ; n` sub-commands (e.g., `9;9` working directory) |
| [99](osc/osc-99.md)                     | Kitty's structured notification protocol                                |
| [777](osc/osc-777.md)                   | `notify` (and related) extension used by urxvt-derived terminals        |

### Shell Integration

| OSC                        | Purpose                                                                  |
| -------------------------- | ------------------------------------------------------------------------ |
| [133](osc/osc-133.md)      | Semantic prompt, input, output, and command-completion marks             |
| [633](osc/osc-633.md)      | VS Code's superset of OSC 133 (command line, cwd, properties)            |

### Text Rendering

| OSC                    | Purpose                                                        |
| ---------------------- | -------------------------------------------------------------- |
| [66](osc/osc-66.md)    | Kitty's text sizing protocol (scaled text, explicit cell widths) |

### Application and Xterm Administration

| OSC                             | Purpose                                                                 |
| ------------------------------- | ----------------------------------------------------------------------- |
| [46, 60, 61](osc/osc-46-60-61.md) | xterm log file and allowed/disallowed window-operation queries        |
| [51](osc/osc-51.md)             | Reserved for Emacs; reused by Vim's `:terminal` API                     |

## Related

- The `biscuit-terminal` skill covers the library that consumes these standards.
