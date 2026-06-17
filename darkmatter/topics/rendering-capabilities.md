# Rendering Capability Flags for `md render`

Status: design exploration
Scope: the granular per-capability CLI surface for `md render`, **independent of**
any `--pty` meta-flag. The meta-flag spec lives at
`darkmatter/features/_unscheduled/pty/spec.md` and layers on top of the surface
described here.

This document inventories every rendering capability Darkmatter could expose
through `md render`, surveys how peer CLIs name analogous flags, and proposes a
concrete flag set with environment-variable precedence rules. It deliberately
stops short of prescribing a single answer where Ken still has product-level
choices to make; those are collected in the open-questions section.

## 1. Inventory of Controllable Capabilities

Sourced from `biscuit-terminal/lib/src/terminal.rs` (`pub struct Terminal`) and
the `discovery` module. The "user-controllable in `md render`?" column reflects
whether overriding the auto-detected value has a plausible reason in the
rendering pipeline — not whether the underlying field is `pub`.

| Capability | Type / values | Field on `Terminal` | User-controllable? | Rationale |
|---|---|---|---|---|
| Color depth | `None` / `Ansi16` / `Ansi256` / `TrueColor` | `color_depth` | Yes | Universal need: pipes, CI, screenshots, accessibility. |
| TTY-ness | `bool` | `is_tty` | Indirectly | Drives `auto` semantics; not exposed as a flag, but `--color=always` overrides what `is_tty=false` would gate. |
| Italics | `bool` | `supports_italic` | Maybe | Some fonts/terminals render italics poorly; granular override is niche. |
| Underline (granular) | `straight`/`double`/`curly`/`dotted`/`dashed`/`colored` | `underline_support` | Maybe | Rarely surfaced; usually bundled into a "styles" toggle. |
| Strikethrough | `bool` (rendered via SGR 9; not a `Terminal` field) | n/a | Maybe | Same bucket as italics. |
| Bold / dim / blink / reverse | SGR-level, not detected | n/a | No | Always emitted when color is on; no detection layer to override. |
| OSC 8 hyperlinks | `bool` | `osc_link_support` | Yes | OSC 8 is pipe-safe and increasingly supported; override matters. |
| Image protocol | `None`/`Kitty`/`ITerm2`/`Sixel` | `image_support` | Yes | Slow/unsupported sinks need a hard off; some users want to force `kitty` etc. |
| Width (columns) | `u32` | `fixed_width` (override) / `terminal_width()` | Yes | Pipes (no TTY), screenshots, fixed-format publishing. |
| Height (rows) | `u32` | `fixed_height` | No (for `render`) | Render flows top-to-bottom; height only matters for pagers/TUI. |
| Unicode glyphs | `bool` | `supports_unicode` | Yes | ASCII fallback for legacy terminals / log files. |
| Color mode (light/dark) | enum | `color_mode` | Yes | Affects palette selection; user may want to force `--theme`. |
| Nerd Font | `Option<bool>` | `is_nerd_font` | Yes | Icon-bearing output should degrade when off. |
| Locale / encoding | enum | `locale`, `char_encoding` | No | Sourced from `LANG`/`LC_*`; CLI override is overkill. |
| Cell size (px) | `CellSize` | `cell_size` | No (for `render`) | Only relevant to image sizing; out of scope here. |
| Terminal app/vendor | enum | `app` | No | Diagnostic field, not a render input. |
| OS / distro / repo info | various | `os`, `distro`, `in_repo`, … | No | Not rendering inputs. |

Capabilities **outside the `Terminal` struct** that some CLIs expose but
Darkmatter likely should not:

- Alternate screen buffer (`smcup`/`rmcup`) — relevant to pagers, not `md render`.
- Mouse reporting — irrelevant for one-shot rendering.
- OSC 7 (working directory), OSC 52 (clipboard), OSC 0/2 (window title) —
  no current render path emits these.
- Cursor visibility / cursor shape — irrelevant for non-interactive output.

These can be listed in the doc as "explicitly out of scope" without further
discussion.

## 2. Ecosystem Conventions

### 2.1 Color

| Tool | Flag spelling | Env vars honored |
|---|---|---|
| GNU `ls`, `grep`, `dir` | `--color=always\|auto\|never` (alias `--color` ≡ `always`) | — |
| GNU `coreutils` | also `--color=tty` (older synonym for `auto`) | — |
| `ripgrep` | `--color=always\|auto\|never\|ansi` | `NO_COLOR` |
| `git` | `--color=always\|auto\|never`; also `--no-color` | `GIT_TERMINAL_PROMPT`, palette in config |
| `cargo` | `--color=always\|auto\|never` | `CARGO_TERM_COLOR` |
| `bat` | `--color=always\|auto\|never` | `BAT_PAGER`, `NO_COLOR` |
| `glow` | `--style=...` (no `--color`); reads `GLAMOUR_STYLE` | `NO_COLOR` |
| `mdcat` | `--no-colour` only | `NO_COLOR` |
| `fd`, `eza`, `delta`, `hyperfine` | `--color=always\|auto\|never` | `NO_COLOR` |

**De-facto standards**:

- Tri-valued `--color=always|auto|never` is universal.
- `--no-color` is a near-universal alias for `--color=never`.
- `NO_COLOR=<anything-non-empty>` (https://no-color.org) is honored by virtually
  every modern CLI and **overrides** `--color=always`.
- `FORCE_COLOR` (Node/JS ecosystem, also `bat`, `clicolors-control`): values
  `0`/`1`/`2`/`3` map to off / 16 / 256 / truecolor. Some tools accept just
  `FORCE_COLOR=1`.
- `CLICOLOR=0` / `CLICOLOR_FORCE=1` (BSD lineage, `ls` on macOS): older but
  still widely honored. `biscuit-terminal::new_forced()` already mentions
  `FORCE_COLOR` and `CLICOLOR_FORCE` in its rustdoc.
- `COLORTERM=truecolor|24bit` signals 24-bit support to auto-detection.
- `TERM=dumb` is the universal "emit nothing" signal.

### 2.2 Hyperlinks (OSC 8)

| Tool | Flag spelling | Env vars |
|---|---|---|
| GNU `ls` 8.32+ | `--hyperlink=always\|auto\|never` | — |
| `ripgrep` 14+ | `--hyperlink-format=default\|none\|file\|<template>` | — |
| `eza` | `--hyperlink` (boolean) | — |
| `delta` | `--hyperlinks` (boolean) + `--hyperlinks-file-link-format` | — |
| `bat` | none (always emits if terminal supports) | — |
| `fd` | `--hyperlink` (boolean) | — |

**De-facto standard**: GNU coreutils' `--hyperlink=always|auto|never` mirrors the
`--color` tri-valued convention and is the closest thing to a standard. There is
**no widely-honored env var** for OSC 8. Some users set `NO_OSC8=1` informally;
nothing standardizes it. Auto-detection is heuristic-driven via
`TERM_PROGRAM`, `TERM`, and explicit allowlists.

### 2.3 Width / Wrapping

| Tool | Flag spelling | Env |
|---|---|---|
| `bat` | `--wrap=auto\|never\|character`, `--terminal-width N`, `--tabs N` | `COLUMNS` |
| `glow` | `-w N` / `--width N` (0 = auto) | `COLUMNS` |
| `mdcat` | `--columns N` | `COLUMNS` |
| `fold`, `fmt` | `-w N` / `--width=N` | `COLUMNS` |
| `pandoc` | `--columns=N` | — |
| `tput cols` | n/a | `COLUMNS`, `LINES` |

`COLUMNS` is POSIX. Honoring it as a fallback when stdout is not a TTY is
expected. `--width N` is the most common flag spelling; `--columns N` is the
second.

### 2.4 Images

| Tool | Flag spelling |
|---|---|
| `mdcat` | `--no-images` (and auto-detects Kitty/iTerm2/Sixel) |
| `glow` | no image rendering |
| `viu`, `chafa` | `--format=kitty\|iterm\|sixel\|symbols` (positional protocol) |
| `kitten icat` | n/a (always Kitty) |
| `timg` | `--upscale`, `-p kitty\|iterm2\|sixel` |

There is **no env var standard** for image protocol selection. The common
shapes are `--image-protocol=...` (selector) and `--no-images` (kill switch).

### 2.5 Granular Styles (italics, underline, strikethrough)

Mainstream CLIs do not expose per-SGR toggles. The closest precedent:

- `bat` and `delta` rely on theme/style profiles (`--style=numbers,grid,...`)
  rather than per-attribute flags.
- `glow` uses `--style` to pick a Glamour style file (rich vs notty vs custom).
- Some terminal-rendering libraries (e.g., Rich for Python) expose per-attribute
  toggles in their config but not in the CLIs that consume them.

Conclusion: there is no precedent that says we **must** ship granular per-style
flags. If we ship them, we are inventing the convention.

### 2.6 Color Mode (Light/Dark Theme)

| Tool | Flag |
|---|---|
| `bat` | `--theme=<name>`, `--theme-light`, `--theme-dark` (newer) |
| `delta` | `--light` / `--dark` |
| `glow` | `-s dark\|light\|notty\|auto` |
| `eza` | theme via config; no CLI flag |

Common shape: `--theme=light|dark|auto` or twin booleans `--light`/`--dark`.
Auto-detection uses OSC 11 (background color query), `COLORFGBG`, or
`termbg`-style probes.

## 3. Recommended Flag Set for `md render`

Three concrete tiers, with a final recommendation at the end.

### Tier A — must have

These mirror the universal ecosystem and cover the 95% case.

| Flag | Values | Maps to `Terminal` field |
|---|---|---|
| `--color <when>` | `always` \| `auto` \| `never` | `color_depth` (forces `TrueColor`/keeps detected/forces `None`) |
| `--hyperlinks <when>` | `always` \| `auto` \| `never` | `osc_link_support` |
| `--width <N>` | u32, ≥ some min (e.g. 20) | `fixed_width = Some(N)` |

Notes:

- Use `--color` (not `--colour`) per the en-US rule in repo CLAUDE.md, with
  no spelling alias.
- `auto` defers to `biscuit-terminal` detection (which already considers
  `is_tty`, `NO_COLOR`, `TERM`, etc.).
- `--hyperlinks` is plural for parity with `eza`/`delta`; `ls` uses the
  singular. Pick one and document it.

### Tier B — useful, conventional naming

| Flag | Values | Notes |
|---|---|---|
| `--no-color` | (boolean) | Alias for `--color=never`. Conflicts with `--color=always`. |
| `--no-hyperlinks` | (boolean) | Alias for `--hyperlinks=never`. |
| `--images <when>` | `auto` \| `never` (or `auto`/`kitty`/`iterm`/`sixel`/`never`) | Maps to `image_support`. See open-question 3. |
| `--no-images` | (boolean) | Alias for `--images=never`. Matches `mdcat`. |
| `--theme <when>` | `light` \| `dark` \| `auto` | Maps to `color_mode`. |
| `--wrap <when>` | `auto` \| `never` \| `character` | Bat-style. `never` would emit long lines verbatim. |
| `--no-wrap` | (boolean) | Alias for `--wrap=never`. |
| `--ascii` | (boolean) | Sets `supports_unicode=false`. (Alternative spelling: `--no-unicode`.) |

### Tier C — granular per-SGR overrides

| Flag | Values | Maps to |
|---|---|---|
| `--italics <when>` | `always` \| `auto` \| `never` | `supports_italic` |
| `--underline <when>` | `always` \| `auto` \| `never` | `underline_support.straight` (and others) |
| `--strikethrough <when>` | `always` \| `auto` \| `never` | (no `Terminal` field — would need plumbing) |
| `--bold <when>` | `always` \| `auto` \| `never` | (no detection layer; semantics fuzzy) |

Trade-offs:

- **Pro**: a single uniform model — every detected attribute has a corresponding
  override flag. This is internally consistent and easy to teach.
- **Con**: surface bloat. The 99th-percentile use case for "force italics off"
  is nonexistent. Help text gets long. Conflict matrices balloon.
- **Con**: strikethrough and bold do not currently have detection plumbing in
  `biscuit-terminal`; adding overrides would require new fields.

### Four candidate shapes

1. **(a) Tier A only.** Minimal, ships immediately, matches the most-loved
   peer CLIs (`ripgrep`, `cargo`, `git`). New flags can be added later
   without breaking compatibility.
2. **(b) Tier A + B.** Recommended baseline. Covers all realistic
   override needs (color/hyperlinks/images/width/theme/wrap/ascii) and
   adds the conventional `--no-X` aliases.
3. **(c) Tier A + B + a single `--styles=always|auto|never`** that
   collectively controls italics/underline/strikethrough/bold. Compromise
   between full granularity and surface bloat. `--styles=never` would emit
   color but no decoration SGR codes — useful for screenreaders, plaintext
   pipelines, and minimal-style screenshots.
4. **(d) Tier A + B + Tier C.** Full granularity. Only worth doing if
   Darkmatter explicitly wants to be the "every knob is a flag" library.

**Recommendation**: **(b) for v1, with (c)'s `--styles` deferred** behind a
real use case. Ship Tier C only if a user shows up asking for it.

## 4. Environment Variable Handling

### 4.1 Recommended honored variables and precedence

Highest precedence first. CLI flags sit between explicit "kill" env vars and
implicit "force" env vars, following the de-facto rule that `NO_COLOR` is
sacrosanct.

| Rank | Source | Effect | Notes |
|---|---|---|---|
| 1 | `NO_COLOR` set (any non-empty value) | `--color=never`; also disables hyperlinks and images | Per no-color.org. Wins over `--color=always`. |
| 2 | `TERM=dumb` | Equivalent to `--color=never --hyperlinks=never --images=never --ascii` | Universal. |
| 3 | Explicit CLI flag (`--color`, `--hyperlinks`, etc.) | As given | User intent beats heuristics. |
| 4 | `FORCE_COLOR` / `CLICOLOR_FORCE=1` | Equivalent to `--color=always` (and friends if we decide to scope them too) | Already supported by `Terminal::new_forced()`. |
| 5 | `COLORTERM=truecolor\|24bit` | Hint for auto color depth | Detection only; doesn't override flags. |
| 6 | `CLICOLOR=0` | Equivalent to `--color=never` (weak — yields to `CLICOLOR_FORCE`) | BSD convention. |
| 7 | `COLUMNS` | Width fallback when not a TTY | POSIX. |
| 8 | `TERM`, `TERM_PROGRAM`, `KITTY_WINDOW_ID`, `WEZTERM_*`, … | Detection inputs | Used only by `auto`. |

### 4.2 Recommended policy

- **Honor**: `NO_COLOR`, `TERM=dumb`, `CLICOLOR_FORCE`, `FORCE_COLOR`, `COLUMNS`.
- **Detect via**: `COLORTERM`, `TERM`, `TERM_PROGRAM`, vendor-specific vars
  (delegated to `biscuit-terminal`).
- **Defer**: `CLICOLOR=0` is BSD-only and `NO_COLOR` covers the same intent
  more cleanly; honoring it is optional.
- **Document precedence in `--help`**. Few CLIs do; the ones that do (`bat`,
  `delta`) get praised for it.

### 4.3 Scope question: does `FORCE_COLOR` force hyperlinks too?

`FORCE_COLOR` is, by name, about color. But pragmatically users set it to
mean "I know what I'm doing, give me rich output." Darkmatter could:

- (a) Treat it as color-only.
- (b) Treat it as a synonym for `--pty` (force every capability on). This
  blurs the line with the meta-flag and is probably wrong.
- (c) Apply only to `color_depth`, leave hyperlinks/images to their own
  flags. **Recommended.**

## 5. CLI Surface Examples

```sh
# Full auto-detect (the default)
md render foo.md

# Force color on; everything else auto
md render foo.md --color=always

# Hand-rolled rich profile (no --pty)
md render foo.md --color=always --hyperlinks=always --width 80

# NO_COLOR wins despite --color=always
NO_COLOR=1 md render foo.md --color=always
# → renders with no color, but hyperlinks still follow --hyperlinks=auto

# Piping to a pager that handles ANSI
md render foo.md --color=always | less -R

# Piping to a non-TTY sink that doesn't (auto detects no TTY → dumb)
md render foo.md | cat > out.txt

# Explicit dumb + width for archival
md render foo.md --no-color --no-hyperlinks --width 60 --ascii

# Force a specific image protocol (if we adopt the enum form of --images)
md render foo.md --images=kitty

# Hard-disable images on a Kitty terminal (network-limited render)
md render foo.md --no-images

# Force light theme palette regardless of background detection
md render foo.md --theme=light

# CI: emit color and hyperlinks (most CI logs render both)
FORCE_COLOR=1 md render foo.md --hyperlinks=always

# COLUMNS fallback when stdout isn't a TTY
COLUMNS=100 md render foo.md > foo.ansi
```

## 6. Open Design Questions

1. **`--no-color` alias or canonical `--color=never` only?**
   Ecosystem expectation is that both work. Recommendation: ship both.
   Clap can express this as a flag with `conflicts_with = "color"` and a
   `--color=never` override in the parser.

2. **Auto semantics differ per capability?**
   Color `auto` = "TTY only." Hyperlinks `auto` could reasonably mean "always
   emit, even off-TTY" because OSC 8 is plaintext and pipe-safe. But emitting
   `\e]8;;…\e\` into log files is surprising. Recommendation: `auto` for
   hyperlinks mirrors color (TTY only). Power users get `--hyperlinks=always`
   for the pipe-safe case.

3. **Default for `--images`: `auto` or `never`?**
   Auto means a Kitty user gets inline images for free; surprising for a
   pipeline that previously emitted text. `never` is conservative but opt-in
   feels reactionary. Recommendation: `auto`, but **only when stdout is a
   TTY** — i.e., image protocols never auto-emit through a pipe regardless
   of `TERM_PROGRAM`. (Setting `--images=always` overrides this.)

4. **`--images` shape: enum vs boolean.**
   - Boolean (`--images=auto|never`): simple; protocol is always detected.
   - Enum (`--images=auto|kitty|iterm|sixel|never`): power-user override
     for forcing a specific protocol, e.g., over SSH.
   - Recommendation: ship the enum form (`mdcat`-style). The detection
     answer is sometimes wrong inside multiplexers.

5. **Profile system (`--profile=rich|plain|ci`)?**
   Some tools (`bat --style=...`) bundle multiple toggles into named
   profiles. `--pty` is already one such profile. Adding `--profile`
   alongside `--pty` risks two ways to spell the same intent.
   Recommendation: do **not** introduce `--profile` unless three or more
   bundled profiles exist; `--pty` alone is acceptable.

6. **Config file (`~/.config/darkmatter/config.toml`)?**
   `bat`, `delta`, `glow` all support config files for persisting style
   choices. For `md render`, the same use cases apply (per-user theme,
   default width, always-on hyperlinks). Recommendation: defer to v2; flags
   + env vars cover v1.

7. **Granular styles: Tier C, `--styles`, or nothing?**
   See section 3. Recommendation: nothing in v1, add `--styles` if a real
   use case emerges.

8. **`--width 0` semantics.**
   Glow uses `0` to mean "auto." Bat uses positive integers and a separate
   `--wrap=never` for "don't wrap." Recommendation: use a positive minimum
   (e.g., `>= 20`); for "auto," let the user omit the flag. Reject `0`
   with a clear error rather than silently overloading it.

9. **How does `--pty` compose with these flags?**
   Two options:
   - (a) `--pty` is pure sugar: it sets `--color=always
     --hyperlinks=always --images=always --width=<detected-or-COLUMNS>`
     and nothing more. Per-flag overrides on the same invocation win.
   - (b) `--pty` carries extra semantics (e.g., force `TERM_PROGRAM`
     re-detection, force specific image protocol, ignore `NO_COLOR`).
   Recommendation: **(a)**, with explicit override precedence:
   `--pty --color=never` should produce uncolored output and not be a
   contradiction. The `pty/spec.md` document should make this explicit.

10. **`--theme` interaction with `--color=never`.**
    If color is off, theme is meaningless. Should `--theme=light
    --color=never` warn, error, or silently ignore? Recommendation:
    silently ignore (consistent with how `--hyperlinks=always` is silently
    ignored when `NO_COLOR=1` wins).

11. **`--ascii` vs `--no-unicode`.**
    `bat`/`glow` use neither. The closest precedent is `LC_ALL=C` /
    `LANG=C`. Recommendation: spell it `--ascii`; it's shorter and the
    intent is unambiguous. `--no-unicode` reads as "I don't like Unicode
    in my source," which is the wrong frame.

## 7. Out of Scope (For This Document)

- The `--pty` meta-flag itself. Spec lives at
  `darkmatter/features/_unscheduled/pty/spec.md`. This document only
  asserts the contract that meta-flags compose **on top of** per-capability
  flags, not the other way around.
- Image rendering implementation (Kitty graphics protocol details, cell-size
  math, etc.) — covered in `biscuit-terminal/docs/terminal-images.md`.
- The internal rendering algorithm (`renderable::layout::Layout` and the
  render tree fold).
- Pagination, alt-screen handling, mouse, OSC 7/52, cursor shape.
- Locale and encoding overrides beyond `--ascii`.
- Performance and caching of detection results.

## Appendix A — Mapping CLI Flags to `Terminal` Builder Calls

For implementation reference, here is how the recommended Tier A + B flags
would translate into `TerminalBuilder` invocations once parsed:

| Flag value | `TerminalBuilder` call |
|---|---|
| `--color=always` | `.color_depth(ColorDepth::TrueColor).supports_italic(true)` (or leave italics to its own flag) |
| `--color=never` | `.color_depth(ColorDepth::None)` |
| `--color=auto` | (no override; detection wins) |
| `--hyperlinks=always` | `.osc_link_support(true)` |
| `--hyperlinks=never` | `.osc_link_support(false)` |
| `--width N` | `.fixed_width(N)` |
| `--no-images` / `--images=never` | `.image_support(ImageSupport::None)` |
| `--images=kitty` | `.image_support(ImageSupport::Kitty)` |
| `--ascii` | `.supports_unicode(false)` |
| `--theme=light` | `.color_mode(ColorMode::Light)` |

`auto` for every flag means "do not call the corresponding builder method;
let detection populate the field."
