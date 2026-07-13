---
name: viuer
description: Expert knowledge for rendering images in Rust terminals with viuer, including protocol auto-detection (Kitty/iTerm2/Sixel) and half-block fallback, plus sizing/positioning via Config. Use when building CLI/TUI image previews, adding terminal thumbnails, or troubleshooting terminal compatibility and feature flags.
tools: [Read, Write, Edit, Grep, Glob, Bash]
---

# viuer

## What this skill is for
Use this skill when you need to **display images in a terminal from Rust** using `viuer`—either by printing a `DynamicImage` or printing directly from a file (feature-gated). Includes guidance on **protocol selection/fallback**, **sizing in terminal cells**, **cursor/offset control**, and **debugging “only blocks show up”** situations.

## Quick decision guide
- Need to **print one image and exit** → `viuer` is ideal.
- Need images **inside a Ratatui render loop** → consider `ratatui-image` (wraps similar backends).
- Need best possible character-art on “dumb” terminals → consider `chafa` bindings.
- Need video/streaming frames → consider `rasteroid`.

## Core API (most-used)
- `viuer::print(&DynamicImage, &Config) -> ViuResult<()>`
- `viuer::print_from_file(path, &Config) -> ViuResult<()>` *(requires `print-file` feature)*
- `Config { width/height/x/y/transparent/truecolor/use_kitty/use_iterm/use_sixel/... }`
- Helpers: `terminal_size()`, `resize()`, `get_kitty_support()`, `is_iterm_supported()`.

## Key concepts to remember
- `width`/`height` are in **terminal cells**, not pixels.
- Default fallback uses **lower half blocks** (`▄`) where **one cell represents ~2 vertical pixels**; image may look “half height” if you expect pixel semantics.
- viuer does **auto-detect + fallback** based on terminal; you can **force protocols** via `use_kitty/use_iterm/use_sixel`.
- `print_from_file` and Sixel support are **feature gated**.

## Common workflows
- Load via `image` crate → `DynamicImage` → `viuer::print`
- Print from path → `viuer::print_from_file` (enable feature)
- Fit to terminal → `terminal_size()` + `resize()` or set `Config.width/height`
- Place inside a “pane” → set `x`, `y`, and consider `absolute_offset`

## Gotchas you will likely hit
- “`print_from_file` not found” → enable `features=["print-file"]`.
- Image shows as colored blocks only → terminal lacks Kitty/iTerm/Sixel; that’s the fallback.
- Kitty not detected in Kitty terminal → likely non-TTY/piped output; force `use_kitty: true`.
- Transparency looks wrong (black) → block fallback can’t do true transparency; try protocol terminals + `transparent: true`, `truecolor: true`.
- Slow rendering on large images → **pre-resize** before printing.

## Linked guides (start here)
- [Setup & feature flags](setup-and-features.md)
- [Core usage patterns (print, print_from_file, stdin)](core-usage.md)
- [Sizing, terminal cells, resizing to fit](sizing-and-layout.md)
- [Protocol detection, forcing protocols, compatibility](protocols-and-compat.md)
- [Troubleshooting & gotchas checklist](troubleshooting.md)
- [Integration patterns (image, crossterm, ratatui-image)](integration-patterns.md)