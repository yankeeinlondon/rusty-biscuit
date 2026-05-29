# PTY mode

## Overview

Add a `--pty` switch and a small family of decomposed capability flags to the Darkmatter CLI's `render` subcommand. When `--pty` is set, the renderer behaves as if it were writing to a rich, capable terminal even when stdout is a pipe or file. The flag is implemented as a capability-detection override inside Darkmatter — no real pseudoterminal is allocated.

## Background

`md render <file>` currently produces terminal output using `biscuit-terminal`'s capability detection. When stdout is a TTY, capability detection inspects the surrounding terminal and emits styled output appropriate to it. When stdout is not a TTY (piped, redirected, captured in CI), capability detection falls back to a minimal/dumb profile and the styled output is lost.

Users want to capture or pipe rich-styled output without losing color, italics, underline, strikethrough, or OSC 8 hyperlinks. They do not want to capture terminal-specific binary blobs such as Kitty graphics, iTerm2 inline images, or Sixel — those produce garbage in non-rendering consumers.

## Goals

- Provide a single user-friendly switch (`--pty`) that forces a pipe-safe rich rendering profile.
- Provide decomposed per-capability flags (`--color`, `--hyperlinks`, `--width`) that follow ecosystem conventions (`ls`, `rg`, GNU coreutils).
- Honor the `NO_COLOR` convention from <https://no-color.org>.
- Keep auto-detection as the default behavior — `md render` with no extra flags continues to work as today.

## Non-Goals

- Allocating a real PTY (no `portable-pty`, no `nix::pty::openpty`, no child re-exec).
- Mirroring a specific invoking terminal's capability profile over a pipe (e.g., emitting Kitty graphics into a file).
- Applying these flags to subcommands other than `render` in this iteration.
- Opt-in for image protocols (Kitty graphics, iTerm2 inline images, Sixel) when piped.
- Windows-specific PTY handling.

## CLI Surface

The `render` subcommand gains the following flags:

| Flag | Values | Default | Description |
|------|--------|---------|-------------|
| `--pty` | boolean | off | Meta-flag: forces a pipe-safe rich profile. |
| `--color` | `always`, `auto`, `never` | `auto` | SGR color emission. |
| `--hyperlinks` | `always`, `auto`, `never` | `auto` | OSC 8 hyperlink emission. |
| `--width` | integer (columns) | see width rules | Explicit output width. |

### `--pty` Semantics

`--pty` is a meta-flag equivalent to setting:

- `--color=always`
- `--hyperlinks=always`
- italics, underline, and strikethrough forced on
- width resolved per the rules in [Width Source of Truth](#width-source-of-truth)

It does not enable image protocols, alt-screen, mouse tracking, or cursor positioning.

### Precedence Rules

When multiple sources affect a capability, the following order applies (highest priority first):

1. `NO_COLOR` env var set to any value — disables color regardless of other flags.
2. Explicit `--color=never` / `--hyperlinks=never` — disables the specific capability.
3. `--pty` — enables all pipe-safe capabilities.
4. Explicit `--color=always`, `--hyperlinks=always`, `--width N` — override the `--pty` defaults for that capability.
5. Auto-detection — inspect the real terminal (default behavior).

### Conflict Warnings

When contradictory flags are combined (for example `--pty --color=never`), Darkmatter emits a single human-readable warning to stderr describing the conflict and which value won per the precedence rules. The command still exits 0.

### No-Op Behavior

When stdout is already a rich TTY, passing `--pty` is a no-op in observable output: auto-detection already produces rich output. The command does not error and exits 0.

## Capability Profile

When `--pty` (or its decomposed equivalents) is active, the following capabilities are enabled:

- truecolor (24-bit color)
- italics
- underline
- strikethrough
- OSC 8 hyperlinks
- width-aware wrapping

The following capabilities are explicitly disabled in `--pty` mode:

- image protocols: Kitty graphics, iTerm2 inline images, Sixel
- alt-screen toggling (`\x1b[?1049h` / `\x1b[?1049l`)
- mouse tracking
- cursor positioning escapes

The rationale for disabling image protocols: they emit binary or large escape-sequence payloads that produce garbage when piped to consumers that do not implement those protocols. They are not pipe-safe.

## Width Source of Truth

When width is needed and `--width N` is not provided, Darkmatter resolves width in this priority order:

1. Explicit `--width N` flag.
2. `$COLUMNS` environment variable (if set and parses as a positive integer).
3. Default constant: **100 columns**.

This rule applies whenever the capability override is active (i.e., `--pty` or any decomposed flag forces non-auto behavior). Under pure auto-detection on a TTY, the existing `biscuit-terminal` width logic continues to apply.

## Implementation Approach

Darkmatter owns both the CLI and the renderer, so the rendering pipeline does not need a real PTY to behave as if it were rendering to one. The implementation is a capability-detection override:

- The `render` command builds a `biscuit-terminal` capability profile from CLI flags and environment, rather than always asking `biscuit-terminal` to auto-detect.
- When `--pty` (or any forcing flag) is present, the constructed profile is the fixed "pipe-safe rich" profile described above, modulated by precedence rules.
- When no forcing flag is present, the existing auto-detection path is used unchanged.

Rationale for rejecting a real PTY:

- A real PTY would require platform-specific code (Unix-only `openpty`, plus extra work on Windows) and a child re-exec or fd-plumbing dance.
- It does not give a functional benefit over a direct capability override, because Darkmatter controls what bytes it emits.
- It would add a dependency (e.g., `portable-pty`) for no observable advantage.

The name `--pty` is retained because users familiar with `script(1)` / `unbuffer` / `script -q` recognize the intent ("render as if to a terminal"), even though the implementation is not a PTY.

## Acceptance Criteria

All criteria below must hold for the feature to be considered complete. Each is independently testable.

1. **SGR present under `--pty`:** `md render foo.md --pty | xxd` contains ANSI SGR escape sequences for color, bold, and italics in the byte stream.
2. **OSC 8 present under `--pty`:** `md render foo.md --pty | xxd` contains OSC 8 hyperlink sequences for any links in `foo.md`.
3. **No image / alt-screen / mouse under `--pty`:** `md render foo.md --pty | xxd` does **not** contain Kitty graphics protocol bytes (`\x1b_G`), iTerm2 inline image sequences (`\x1b]1337;File=`), alt-screen toggles (`\x1b[?1049h` / `\x1b[?1049l`), or mouse-tracking sequences (`\x1b[?1000h`, `\x1b[?1006h`, etc.).
4. **Decomposed flags:** `md render foo.md --color=always --hyperlinks=never | xxd` contains SGR codes but no OSC 8 sequences.
5. **`NO_COLOR` wins:** `NO_COLOR=1 md render foo.md --pty | xxd` contains no color SGR sequences.
6. **Width default:** with no `--width` flag and `COLUMNS` unset, output wraps at 100 columns.
7. **Width via `$COLUMNS`:** with `COLUMNS=50` exported and no `--width` flag, output wraps at 50 columns.
8. **Width via flag:** with `--width 70`, output wraps at 70 columns regardless of `$COLUMNS`.
9. **Conflict warning:** `md render foo.md --pty --color=never` exits 0 and emits a single stderr warning describing the contradiction and the resolved value.
10. **TTY no-op:** when stdout is a TTY and `--pty` is passed, the command exits 0 and produces output that is functionally equivalent to or richer than the same command without `--pty`.

## Out of Scope / Deferred

- Extending these flags to subcommands beyond `render` (deferred to a follow-up).
- Image protocol opt-in over pipes (e.g., a future `--images=always`).
- Mirroring the invoking terminal's exact capability set into the pipe.
- Windows-specific PTY or console-mode handling.
- A `script(1)`-style real PTY mode for capturing third-party command output.
