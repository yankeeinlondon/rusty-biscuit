---
ready: false
agent: codex
model: ""
---

# Review: Darkmatter Layout

## Findings

### High: Terminal layout rendering is only verified at Level 1, but the spec requires real-terminal visual behavior

The layout feature has user-observable terminal requirements for margins, padding, max width, page background SGR fill, line numbers, and component alignment/fill. The current coverage is in-process unit/snapshot testing (`darkmatter/lib/src/layout/page.rs`, `darkmatter/lib/tests/layout_snapshots.rs`) plus CLI process tests (`darkmatter/cli/tests/cli.rs`). The existing Level-2 tests in `darkmatter/cli/tests/level2_errors.rs` cover error rendering only, not layout.

Strongest verification by requirement:

| Requirement | Current strongest level | Required level | Status |
| --- | --- | --- | --- |
| Zero-config equivalence with `for_terminal` | Level 1 | Level 1 | OK |
| CLI parse/precedence for margin, padding, alignment, fill | Level 1 | Level 1 | OK |
| Terminal row dimensions for margin/padding/max-width | Level 1 | Level 2 | Gap |
| Page background SGR colors and reset behavior | Level 1 | Level 2 | Gap |
| Code/table/list/image/block quote alignment and fill rendering | Level 1 | Level 2 | Gap |
| `--line-numbers` visible code block output | Level 1 | Level 2 | Gap |
| Browser HTML/CSS wrapper output | Level 1 | Level 1 | OK for string output |

Per the requested rigor rules, these Level-2 mismatches are production blockers until at least one real-terminal capture path verifies the actual rendered pane text/raw SGR behavior for the layout cases.

### High: `PageFill::Pad` and `PageFill::Indent` do not implement the documented side-padding behavior

The spec says `Pad` adds symmetric component padding and `Indent` adds one-sided padding based on alignment. The implementation computes reduced component widths, but the actual padding is not applied for left-aligned components because `apply_component_layout` returns immediately when alignment is `Left` (`darkmatter/lib/src/markdown/output/terminal.rs:2789`). The helper that computes `(left, right)` padding exists, but it is not used in the terminal renderer.

This means `--fill-code-blocks pad=4`, `--fill-lists indent=2`, and similar defaults do not produce the documented visible padding/fill when alignment is left, even though left is the default alignment. It also means the reclaimed cells are not filled with page background as specified.

### High: Code-block fill width is applied to the header, but not to the highlighted body

For code blocks, `resolve_component_render_width` is used for `format_header_row` (`darkmatter/lib/src/markdown/output/terminal.rs:1175`), but the highlighted code body is rendered by `highlight_code` with the original `TerminalOptions` (`darkmatter/lib/src/markdown/output/terminal.rs:1198`). The component-specific width is not passed into the highlighter.

As a result, `PageFill::Max` / `Explicit` can make the code header narrower while the body still wraps/renders at the page-level width. That breaks the spec requirement that component fill controls the component render width.

### High: Page background color mode ignores the captured terminal color mode

`DarkmatterPage::new(&Terminal)` stores `terminal_color_mode`, but `LayoutContext::from_page` names that argument `_terminal_color_mode` and never uses it (`darkmatter/lib/src/layout/context.rs:120`). Background selection and pronounced inversion are driven by `options_color_mode` instead (`darkmatter/lib/src/layout/context.rs:148`).

For library callers using `DarkmatterPage::new(&terminal)` without also calling `with_color_mode`, a light terminal can still resolve `Subtle` / `Pronounced` as if it were dark. That violates the spec’s statement that page backgrounds resolve from the terminal’s detected color mode and that `Pronounced` inverts from the terminal surface.

### Medium: CLI `--line-numbers` shape does not match the spec

The spec defines `--line-numbers <true|false>`. The implementation exposes `--line-numbers` as a boolean switch plus a separate `--no-line-numbers` (`darkmatter/cli/src/args.rs:489`). That may be a reasonable CLI design, but it is not the specified public surface and will reject documented invocations such as `--line-numbers false`.

### Medium: Browser rendering is only selector-based and not covered by golden tests

The browser path emits a wrapper and broad selectors such as `.darkmatter-page ul, ol`, `.darkmatter-page .code-block, pre`, and `img`. There are unit assertions for fragments of CSS, but no golden test for a representative HTML document. The spec asked for browser HTML golden tests and per-component layout mapping; the current tests do not verify that component selectors target the intended elements without leaking to descendants or unrelated elements.

## Verification Run

- `cargo test -p darkmatter layout --lib --tests` passed.
- `cargo test -p darkmatter-cli layout_` passed.

Both runs are Level 1 for this feature. I found no Level-2 or Level-3 tests exercising the new layout behavior.

## Recommendation

Do not mark this production-ready yet. First fix terminal component fill so `Pad`, `Indent`, `Max`, and `Explicit` are applied consistently to component bodies and visible side padding. Then add Level-2 layout captures for margins/padding/background, max-width, line numbers, and component alignment/fill. Also decide whether to change the CLI to match `--line-numbers <true|false>` or update the spec/docs to bless the current flag pair.
