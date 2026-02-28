# Darkmatter Table Rendering Recommendations

## Summary

Darkmatter should migrate terminal table rendering from `comfy-table` to `biscuit-terminal::Table` and treat `biscuit-terminal` renderables as the canonical terminal rendering layer.

Key recommendations:

- Use `biscuit_terminal::components::table::table::Table` as the terminal table engine.
- Keep table parsing in Darkmatter, but hand off final layout/rendering to `Table`.
- Use `Prose` selectively for cell-level style serialization, not as a Markdown parser.
- Keep `TerminalImage` as a block-level renderable; do not inject image protocol escape sequences directly into table cells.
- Standardize renderable usage around `render(&terminal)` (or `display(&terminal)` for direct print flows).

## Current State (Darkmatter)

Current table flow in `darkmatter/lib/src/markdown/output/terminal.rs`:

- Parses Markdown table events and buffers `table_rows` / `table_alignments`.
- Renders via `render_table()` using `comfy-table`.
- Uses marker workarounds for OSC8 links and inline code markers.
- Strips inline code styling in cells because ANSI breaks comfy-table wrapping/alignment.

Notable indicators in code/tests:

- Inline-code styling is intentionally removed in table cells due to alignment issues.
- Link handling requires marker replacement/reinsertion.
- Table rendering has accumulated width/word-splitting regressions and custom fixes.

This is a strong signal that Darkmatter is duplicating behavior `biscuit-terminal::Table` already solves (ANSI-aware width, wrapping, alignment, links, multiline handling).

## Why `biscuit-terminal::Table` Is a Better Fit

`biscuit-terminal::Table` already supports the capabilities Darkmatter is reimplementing:

- ANSI- and OSC-aware width calculations (`visible_width`, wrapped-line sanitization).
- Typed/aligned columns (`TableColumn`, `ColumnType`, per-column alignment and wrap behavior).
- Multiline cell content and vertical alignment.
- Conditional column visibility for narrower terminals.
- Optional alternating row styling and cursor-alignment mode.
- `Renderable` integration with `render(&Terminal)`.

Migrating to this component reduces bespoke table logic in Darkmatter and aligns with the architecture split where terminal behavior lives in `biscuit-terminal`.

## `Prose` Evaluation and Recommendation

`Prose` is useful, but with constraints:

- `Prose` is token/tag based (e.g. `{{bold}}`, `<a href="...">`), not Markdown-event based.
- It can style text, colors, and OSC8 links, but does not natively understand Darkmatter's Markdown inline event stream.

Recommendation:

- Use `Prose` as an optional serializer for cell content after Darkmatter parses inline Markdown events.
- Add a small adapter: Markdown-inline events -> `Prose` token/tag string -> `Prose::new(...).render(&terminal)`.
- Do not rely on `Prose` for code-span theming from syntect. Keep code-span styling from Darkmatter's existing style/highlighter path, then pass ANSI text into `TableCellContent`.

Practical split:

- Strong/emphasis/strikethrough/link in cells: good `Prose` candidate.
- Syntax-aware inline code colors from syntect: keep Darkmatter's existing style logic.

## `TerminalImage` Evaluation and Recommendation

Darkmatter already uses `TerminalImage` in a good direction (`render(&self.terminal)`).

For table rendering specifically:

- Do not place raw Kitty/iTerm image escape sequences directly inside `TableCellContent`.
- Table cells are width-managed text regions; terminal image protocols are cursor/protocol sequences and can break cell geometry.

Recommended behavior for images inside table cells:

- Phase 1: render a text fallback in-cell (e.g. `IMAGE[alt]` or link-style fallback).
- Phase 2 (optional): if rich in-cell image rendering is required, design a dedicated row-expansion pipeline (image rendered as a separate block under the table row), not inline cell text.

Use `TerminalImage` directly only for block-level image events outside table cells.

## Renderable Policy (Important)

Within Darkmatter, default policy should be:

- Use `render(&terminal)` for renderable composition.
- Use `display(&terminal)` when directly printing a renderable to terminal output and you want newline-safe output.
- Avoid `.render_optimistic(Some(width))` unless you intentionally need optimistic/capability-agnostic output (tests or controlled snapshots).

This matches `biscuit-terminal` guidance and avoids capability mismatches.

## Suggested Migration Plan

1. Add a new table renderer adapter in Darkmatter that builds `biscuit-terminal::Table` from buffered Markdown table data.
2. Map Markdown alignments to `TableColumn` alignment and default to `ColumnType::String`.
3. Feed cell text as ANSI-safe `TableCellContent::Text` (including link/style output).
4. Route rendering through `table.render(&terminal)`.
5. Keep current `comfy-table` path behind a short-lived fallback flag; remove after parity tests pass.
6. Remove `comfy-table` dependency from `darkmatter/lib/Cargo.toml` once migration is complete.

## Test Additions After Migration

Add/keep regression tests for:

- Inline code in cells retains styling without column drift.
- OSC8 links in cells keep correct width/alignment and fallback behavior.
- Mixed-width Unicode content alignment.
- Very narrow terminal width wrapping behavior.
- Images inside table cells use deterministic textual fallback.
- `HyperlinkMode::{Auto,Never,Always}` behavior in table cells remains correct.
