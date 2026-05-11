---
agent: codex
model: ""
ready: true
---

# Review: Darkmatter Layout

## Findings

### High: list and blockquote fill widths are computed but not actually used

The list and blockquote paths push a component-specific width with
`wrapper.push_component_width(component_width as usize)` in
`darkmatter/lib/src/markdown/output/terminal.rs:1390` and
`darkmatter/lib/src/markdown/output/terminal.rs:1685`, but
`LineWrapper::effective_max_width()` reads `width_stack.last()` instead of the
active `self.max_width` at `darkmatter/lib/src/markdown/output/terminal.rs:2333`.

Because `push_component_width()` stores the previous width in `width_stack` and
then assigns the override to `self.max_width`, `effective_max_width()` returns
the old width while inside the component. That means list and blockquote wrapping
still uses the pre-layout width, so `PageFill::Max`, `Explicit`, `Pad`, and
`Indent` are incomplete for those components.

The existing unit tests do not catch this:

- `render_blockquote_with_indent_fill` says the quote should wrap at 70 columns,
  but only asserts `max_len <= 80` at `darkmatter/lib/src/layout/page.rs:1387`.
- `render_list_with_max_fill` uses short list items that pass even without width
  constraint at `darkmatter/lib/src/layout/page.rs:1398`.

Required fix: make `effective_max_width()` return the active override
(`self.max_width`) and keep the stack only for restoration, then add tests with
long list and blockquote content that fails without the override.

Verification level: currently Level 1 only, and weak. The user-observable
terminal wrapping requirement also needs Level 2 capture for lists and
blockquotes.

### High: Level 2 coverage is incomplete for user-observable component layout

The spec requires per-component alignment and fill for images, block quotes,
tables, code blocks, and lists. The Level 2 file only exercises margin,
background SGR, max-width, line-number gutter, and code-block fill/alignment
(`darkmatter/cli/tests/level2_layout.rs:53` through `darkmatter/cli/tests/level2_layout.rs:309`).

Missing Level 2 coverage:

- Tables: alignment/fill through real terminal capture.
- Images: fallback/protocol text alignment and fill through real terminal capture.
- Block quotes: alignment/fill/wrapping through real terminal capture.
- Lists: alignment/fill/wrapping through real terminal capture.
- End-to-end layout dimensions from the spec example through real terminal
  capture, including top/bottom rows and visible row widths.

Per the requested rigor rubric, these are user-observable terminal rendering
requirements. Unit tests and PTY-style CLI tests are not enough because they do
not prove real terminal rendering of glyph widths, SGR background fills, and
captured pane layout.

Verification level: code blocks have Level 2. Tables/images/blockquotes/lists
are effectively Level 1. This is a readiness blocker.

### High: Level 2 tests skip silently, so production readiness depends on local environment

The Level 2 harness returns early when WezTerm is unavailable at
`darkmatter/cli/tests/level2_layout.rs:25`. That is useful for developer
ergonomics, but it also means the strongest verification level may not run in CI
unless CI explicitly provides WezTerm and `WEZTERM_UNIX_SOCKET`.

For this feature, Level 2 is the required verification level for many
user-observable terminal rendering requirements. A production gate should run
these tests in a real terminal job, or the review should treat them as present
but unenforced.

### Medium: zero-config equivalence can be affected by the captured Terminal width

`DarkmatterPage::render()` always passes `Some(&ctx)` into the terminal renderer
at `darkmatter/lib/src/layout/page.rs:435`, even when no layout builder was
called. Several component paths then resolve widths from that context. If a
caller constructs `DarkmatterPage::new()` from a `Terminal` whose captured width
differs from `TerminalOptions::default()` auto-detection during rendering,
zero-config output for components can diverge from `for_terminal()`.

The current equivalence tests use `Terminal::new()`, so they do not cover this
edge. Either avoid passing a layout context when `is_default_layout()` is true,
or add explicit tests documenting that captured terminal width is intentionally
part of zero-config behavior.

Verification level: Level 1 only. This is mostly API semantics, so Level 1 is
appropriate once the intended behavior is clarified.

## Verification Matrix

| Requirement | Strongest verification present | Status |
| --- | --- | --- |
| Zero-config equivalence | Level 1 unit tests | Partial; missing mismatched captured-width case |
| Margin / padding rows | Level 2 margin; Level 1 padding dimensions | Partial |
| Page background SGR / reset | Level 2 presence check | Partial; exact dimensions/colors still mostly Level 1 |
| Max width prose wrapping | Level 2 | OK |
| Line-number gutter | Level 2 | OK |
| Code-block fill/alignment | Level 2 plus Level 1 | OK |
| Table fill/alignment | Level 1 only | Gap |
| Image fill/alignment | Level 1 only | Gap |
| Blockquote fill/alignment/wrapping | Weak Level 1 only | Gap |
| List fill/alignment/wrapping | Weak Level 1 only | Gap |
| Browser wrapper CSS | Level 1 string assertions | Acceptable for current HTML contract, but no golden fixture |
| Error variants reachable | Level 1 | OK |
| CLI parse/precedence | Mostly Level 1 CLI process tests | Partial; precedence asserts parse success more than resolved output |

## Ergonomics / Performance Notes

- `LineWrapper` would be simpler and less error-prone if the active width were
  always `self.max_width`, with the stack named `previous_widths` to make the
  push/pop contract obvious.
- CLI precedence tests should assert observable resolved behavior, not only
  successful parsing. For example, `-m 2 --mt 0` should assert the first content
  row position and absence of top margin.
- Browser tests would be more maintainable as small golden fixtures for wrapper
  style output rather than many substring checks.

## Production Readiness

Not ready. The feature has a real implementation bug for list and blockquote
fill/wrapping, and several user-observable component layout requirements are
below the required verification level.

I attempted `cargo test -p darkmatter layout --color=never`, but compilation was
still in progress after roughly 60 seconds, so I stopped it per the
non-interactive session constraints.

## Remediation (2026-05-11)

All findings addressed in the same branch:

- **High — list/blockquote width bug.** `LineWrapper::effective_max_width()`
  now returns `self.max_width` (the active value) and `width_stack` was
  renamed to `previous_widths` to make the push/pop contract obvious
  (`darkmatter/lib/src/markdown/output/terminal.rs:2298-2360`). The
  `render_blockquote_with_indent_fill` and `render_list_with_max_fill` unit
  tests were strengthened to use long content and assert the actual wrap cap
  (70 cols / 50 cols respectively) — they fail without the fix
  (`darkmatter/lib/src/layout/page.rs:1374-1442`).

- **High — Level 2 coverage for tables/images/blockquotes/lists.** Added
  real-terminal captures via the existing WezTerm harness in
  `darkmatter/cli/tests/level2_layout.rs`:
  - `level2_table_max_fill_constrains_visible_width`
  - `level2_table_center_alignment_indents_more_than_left`
  - `level2_blockquote_indent_fill_caps_wrap_width`
  - `level2_blockquote_center_alignment_indents_more_than_left`
  - `level2_list_max_fill_caps_wrap_width`
  - `level2_list_center_alignment_indents_more_than_left`
  - `level2_image_fallback_text_respects_alignment`
  - `level2_end_to_end_layout_dimensions` (margin rows + max row width +
    component presence)

- **High — Level 2 silent-skip CI gap.** The harness now honours
  `DARKMATTER_LEVEL2_REQUIRED=1`; when set, missing WezTerm panics instead of
  silently skipping. CI jobs that provision WezTerm should set this so
  Level 2 is actually enforced
  (`darkmatter/cli/tests/level2_layout.rs:21-44`).

- **Medium — zero-config equivalence under captured Terminal width.**
  `DarkmatterPage::render()` now passes `None` to the underlying renderer
  when `is_default_layout()` is true, so component width resolution doesn’t
  leak the captured terminal width
  (`darkmatter/lib/src/layout/page.rs:433-447`). Added
  `zero_config_render_ignores_captured_terminal_width`, which constructs a
  page from `Terminal::new_optimistic(40 | 100 | 200)` and verifies
  byte-for-byte equivalence with `for_terminal(.., default())`.

- **Ergonomics — CLI precedence asserts resolved behavior.** Added
  `layout_resolved_*` tests in `darkmatter/cli/tests/cli.rs` that drive
  `apply_cli_layout_flags` and assert the resolved `DarkmatterPage` state
  (margin sides, padding sides, global-vs-component fill, global-vs-component
  alignment, max width). The previous tests only checked parse success.

### Verification

- `cargo test -p darkmatter --lib layout::` — 85 passed.
- `cargo test -p darkmatter-cli --test cli layout_` — 30 passed.
- `cargo test -p darkmatter-cli --test level2_layout` — 17 passed (all 17,
  real WezTerm captures, including the 8 new component/end-to-end tests).
