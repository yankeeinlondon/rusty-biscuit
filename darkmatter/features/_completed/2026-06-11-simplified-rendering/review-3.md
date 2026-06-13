---
ready: false
agent: codex
model: ""
---

# Review 3 - Simplified Rendering

## Findings

### High - `TerminalCodeRenderer` is still part of the public rendering API

The spec's public API model is intentionally narrowed to `CodeBlock` for atomic code panels and `DarkmatterPage` for Markdown documents. It also says `TerminalCodeRenderer` should remain adapter plumbing and be deprecated if currently public. The implementation still publicly re-exports it from both `darkmatter::markdown` and `darkmatter::markdown::render_tree`:

- `darkmatter/lib/src/markdown/mod.rs:73`
- `darkmatter/lib/src/markdown/render_tree/mod.rs:78`

There is no `#[deprecated]` attribute or public warning on the type itself. This leaves library callers with exactly the lower-level adapter surface the feature is meant to retire, and it weakens the success criterion that callers can render code without knowing about `TerminalCodeRenderer`.

Suggested fix: either make the adapter crate-private if no external compatibility window is required, or add a deprecation attribute/note on `TerminalCodeRenderer` and its public re-exports pointing callers to `CodeBlock` and `DarkmatterPage`.

### Medium - Direct/fenced `CodeBlock` parity coverage is weaker than the spec requires

The spec requires targeted golden coverage that direct `CodeBlock` output equals fenced-code-in-`DarkmatterPage` output for the same code, language, metadata, theme, and surface. Current tests cover useful pieces, but not that full contract:

- `darkmatter/lib/src/markdown/code_block.rs:979` says browser output is byte-for-byte equal, but the test only checks that both direct and fenced HTML contain the wrapper. It never compares `direct` and `fenced`.
- `darkmatter/cli/tests/code_block.rs:645` names terminal parity, but compares terminal output to HTML output after stripping ANSI/tags, so it only proves the body text survived across two different surfaces.
- The parity cases do not cover metadata (`title`, `line-numbering`, `highlight`) and theme override together against `DarkmatterPage` on the same surface.

This matters because the feature's consolidation guarantee is not just that both paths contain the code body; it is that the same renderer path preserves header/title, gutters, highlighted-line markup/SGR, language metadata, and theme resolution.

Suggested fix: add L1 same-surface parity tests that compare:

- `CodeBlock::rust(code).with_meta(...).with_theme(...)` terminal output against a `DarkmatterPage` rendering of an equivalent fenced block with the same page code theme/mode.
- The same case for browser HTML, comparing the full fragment/panel markup or a stable normalized code-block fragment, not only wrapper/body substrings.

## Test Rigor Assessment

- Code-block construction, aliases, env theme fallback, explicit overrides, `YamlBlock` delegation, malformed CLI highlight ranges, and string-level terminal/HTML parity are covered at Level 1. That is appropriate for API and pure rendering-string contracts, but the parity assertion above needs to be strengthened.
- Browser-visible code-block background, syntax color, captured-mode behavior, and `CodeBlockMode` behavior are covered through headless Chromium computed-style tests. That is the right level for browser CSS/HTML behavior.
- Real terminal rendering of code-block contrast, layout, title/gutter/highlight survival, and SGR behavior has Level 2 coverage in the existing terminal harness tests. No Level 3 coverage is required because this feature does not specify OS keyboard/mouse input behavior.

## Production Readiness

Not ready. The main rendering behavior appears substantially implemented, and the focused test I ran passed:

```text
cargo test -p darkmatter fenced_rust_block_browser_routes_through_code_block --lib --color=never
```

But the feature should not be marked production-ready until the public adapter surface is retired/deprecated and the direct-vs-fenced parity tests match the spec's same-surface, metadata/theme-aware contract.
