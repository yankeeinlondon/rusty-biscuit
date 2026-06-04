---
ready: true
agent: codex
model: ""
---

# Review 7

## Findings

### Low -- darkmatter tree adapter docs still describe completed graphics work as deferred

The implementation now maps the browser Mermaid opt-in in `browser_options_from_html_options`
(`darkmatter/lib/src/markdown/render_tree/entrypoints.rs:172-184`) and maps terminal
`TerminalImageMode` / `MermaidMode` into `GraphicsMode`, `force_graphics`, and
`TerminalMermaidMode` (`darkmatter/lib/src/markdown/render_tree/entrypoints.rs:222-249`).
The comments above those entry points still say Mermaid / HR CSS variables and
image handling are deferred parity gaps (`darkmatter/lib/src/markdown/render_tree/entrypoints.rs:66-67`,
`darkmatter/lib/src/markdown/render_tree/entrypoints.rs:92-94`,
`darkmatter/lib/src/markdown/render_tree/entrypoints.rs:157-158`,
`darkmatter/lib/src/markdown/render_tree/entrypoints.rs:194-195`).

This is documentation drift, not a functional gap. Per repo convention, assume
the code is correct and update or remove the stale comments so future reviewers
do not treat implemented graphics-policy behavior as intentionally deferred.

Requirement verification level: documentation only. L1 is sufficient.

## Test Rigor Notes

- `GraphicsMode` defaults and entry-point mappings have L1 coverage. The
  terminal adapter maps `TerminalImageMode::Never` to `Off`, `Auto` / `Force`
  to `Rich`, and sets `force_graphics` for `Force`; Mermaid promotion remains
  a separate opt-in.
- Terminal HR policy has L1 unit coverage for `Off`, `Vector`, `Rich`, and
  `force_graphics`, plus Level 2 WezTerm coverage proving `Vector` renders the
  text/glyph tier in a real terminal. Rich image-node rendering has L1 protocol
  coverage and Level 2 WezTerm/pixel-readback coverage for real terminal paint.
- Browser HR fidelity has L1 source/DOM-shape coverage and browser-tier
  computed-style coverage for the styled SVG at `Vector` / `Rich`, including
  hostile HR hint regression coverage.
- Browser Mermaid promotion has L1 policy/strictness/fallback coverage, browser
  tier coverage for promoted static SVG DOM parsing, and browser tier sanitizer
  coverage for active markup plus CSS/external-reference payloads.
- Markdown output remains structurally unaffected: Mermaid stays fenced code and
  HR stays Markdown.
- No Level 3 requirement was identified. The spec defines no keyboard, mouse,
  paste, IME, or OS input behavior.

## Verification Performed

- Read `renderable/features/2026-05-26-graphics-policy/spec.md` and prior
  reviews, especially `review-6.md`.
- Inspected the implementation in `renderable`, `biscuit-terminal`, and
  `darkmatter`, including the browser renderer, terminal renderer, darkmatter
  entry-point mappings, and Mermaid SVG sanitizer.
- Confirmed via `cargo metadata --no-deps --format-version 1` that
  `renderable` did not gain `biscuit-terminal` or `biscuit-visualized`
  dependencies.
- Ran `cargo test -p renderable --lib mermaid --color=never`: passed.
- Ran `cargo test -p darkmatter --test browser_render sanitized --color=never`:
  passed.
- Ran `cargo test -p darkmatter render_tree_terminal_maps --color=never`:
  passed, though the filter matched no test names.
- Ran `cargo test -p darkmatter render_tree_html_mermaid --color=never`: passed.
- Ran `cargo test -p biscuit-terminal render_tree_image --color=never`: passed.
- Ran `cargo test -p biscuit-terminal render_tree_thematic_break --color=never`:
  passed.
- Ran `cargo test -p biscuit-terminal mermaid_promotion_failure --color=never`:
  passed.

## Production Readiness

Ready for production. The prior sanitizer gap is addressed with both L1 and
browser-tier regressions, and every user-observable graphics-policy requirement
has the appropriate verification level for this spec.
