---
ready: true
agent: codex
model: ""
---

# Review 5 - Simplified Rendering

## Findings

No blocking findings.

The two high-priority gaps from review 4 are addressed:

- `TerminalCodeRenderer` is now explicitly deprecated as adapter plumbing, with public guidance to use `CodeBlock` or `DarkmatterPage` instead.
- Direct/fenced `CodeBlock` parity now has same-surface Level 1 tests for terminal and browser output, including `title`, `line-numbering`, `highlight`, and an explicit theme override.

## Test Rigor Assessment

- `CodeBlock` construction, language aliases, direct environment theme fallback, explicit theme overrides, `YamlBlock` delegation, `md code-block` file/content input, markdown/html/terminal output, line numbering, and highlight ranges are covered at Level 1. This is the right level for pure API and string-rendering contracts.
- Direct `CodeBlock` output versus fenced code rendered through `DarkmatterPage` is covered at Level 1 for both terminal and browser surfaces, including metadata and theme override. This matches the spec's byte/string parity requirement.
- Browser-visible code-block background, syntax span color, captured terminal mode, and `CodeBlockMode::Same` versus `Inverse` are covered by browser-tier computed-style tests in headless Chromium. This is the right level for CSS cascade and computed browser styling.
- Real terminal rendering for code-panel rectangle layout, SGR/background continuity, widths, and blank-row behavior has Level 2 coverage in `level2_render_tree_terminal.rs`. No Level 3 coverage is required because this feature does not specify OS keyboard, paste, mouse, or modifier-key behavior.

## Verification Run

Focused checks passed:

```text
cargo test -p darkmatter fenced_block_with_metadata_and_theme_equals_direct --lib --color=never
cargo test -p darkmatter theme_env --lib --color=never
cargo test -p darkmatter fenced_rust_block_browser_routes_through_code_block --lib --color=never
cargo test -p darkmatter-cli --test code_block --color=never
cargo test -p darkmatter --test browser_render browser_page_code_block --color=never
```

One attempted Cargo invocation used multiple test filters and was rejected by Cargo before running tests; it was replaced by the valid filtered runs above.

## Production Readiness

Ready for production.
