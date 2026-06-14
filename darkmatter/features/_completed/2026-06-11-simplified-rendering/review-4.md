---
ready: false
agent: codex
model: ""
---

# Review 4 - Simplified Rendering

## Findings

### High - `TerminalCodeRenderer` is still a public, undeprecated rendering surface

The spec narrows the public rendering model to two Darkmatter-owned components: `CodeBlock` for an atomic code panel and `DarkmatterPage` for a Markdown document. It also explicitly says `TerminalCodeRenderer` is adapter plumbing and "should not be part of the public rendering API"; Phase 4 says to deprecate direct public use if it is currently public.

The implementation still exposes `TerminalCodeRenderer` publicly and without a deprecation warning:

- `darkmatter/lib/src/markdown/mod.rs:73` re-exports it as `darkmatter::markdown::TerminalCodeRenderer`.
- `darkmatter/lib/src/markdown/render_tree/mod.rs:78` re-exports it as `darkmatter::markdown::render_tree::TerminalCodeRenderer`.
- `darkmatter/lib/src/markdown/render_tree/code_renderer.rs:38` defines `pub struct TerminalCodeRenderer` with public constructors/builders and no `#[deprecated]` attribute.

This keeps the low-level render-tree adapter as a first-class caller-facing API, which directly conflicts with the feature's success criterion that callers can render code without knowing about `TerminalCodeRenderer` or render-tree hooks.

Suggested fix: make the adapter crate-private if no compatibility window is required. If compatibility is required, add `#[deprecated]` to the type and public re-exports with a note directing callers to `CodeBlock` and `DarkmatterPage`.

### High - Direct/fenced `CodeBlock` parity is still under-verified for the required metadata/theme contract

The spec requires: "`CodeBlock`-direct output equals fenced-code-in-`DarkmatterPage` output for the same code, language, metadata, theme, and surface." The implementation now has a same-surface terminal equality test for the simplest Rust fence, but the broader required contract is still not verified.

Coverage gaps:

- `darkmatter/lib/src/markdown/code_block.rs:982` says browser output is byte-for-byte equal, but the test only checks that each output contains a `<pre><code class="language-rust">` wrapper. It never compares `direct` and `fenced`.
- `darkmatter/cli/tests/code_block.rs:645` compares direct terminal output against fenced HTML after stripping ANSI/tags, so it is cross-surface body-presence coverage, not parity for one surface.
- `darkmatter/cli/tests/code_block.rs:693` similarly checks wrapper/body presence for HTML, not direct-vs-fenced equality or a stable normalized code-block fragment.
- None of the direct/fenced parity tests combine `title`, `line-numbering`, `highlight`, and a theme override on the same surface, which is the part most likely to regress when metadata serialization or theme resolution differs between direct `CodeBlock` and fenced Markdown.

This is a Level 1 verification gap: the requirement is string-level renderer parity, so in-process/CLI tests are the appropriate level, but the strongest current tests do not assert the specified behavior. The existing simple terminal equality test is useful, but it does not cover the metadata/theme surface the spec calls out.

Suggested fix: add focused L1 same-surface parity tests:

- Terminal: compare direct `CodeBlock::rust(code).with_meta(meta).with_theme(theme).render(&term)` with an equivalent fenced block rendered through `DarkmatterPage` using the same terminal, page code theme/mode, title, line numbering, and highlight metadata.
- Browser: compare direct `CodeBlock` HTML against the equivalent fenced block rendered through `DarkmatterPage::render_to_browser`, either byte-for-byte when the surrounding page wrapper is excluded or by extracting and comparing the stable `.code-block` fragment.
- CLI: if CLI parity is kept, compare terminal-to-terminal and HTML-to-HTML outputs or normalized same-surface fragments; do not use terminal-vs-HTML body-presence as the parity oracle.

## Test Rigor Assessment

- `CodeBlock` construction, language aliases, direct env theme fallback, explicit theme overrides, CLI `md code-block` inputs/options, and `YamlBlock` delegation are covered at Level 1. That is the right level for pure API and string-rendering contracts.
- Browser-visible code-block mode/theme behavior now has headless Chromium computed-style coverage in `darkmatter/lib/tests/browser_render.rs`, which is the right level for CSS cascade and computed style.
- Existing Level 2 terminal harness tests cover real-terminal rendering semantics for glyphs, widths, SGR behavior, and layout. No Level 3 coverage is required because this feature does not specify OS keyboard/mouse input behavior.
- The direct/fenced parity requirement remains Level 1 under-covered as described above; it should not be treated as production-ready until the same-surface metadata/theme parity cases exist.

## Production Readiness

Not ready.

The focused tests I ran passed:

```text
cargo test -p darkmatter fenced_rust_block_routes_through_code_block --lib --color=never
cargo test -p darkmatter fenced_rust_block_browser_routes_through_code_block --lib --color=never
```

However, the feature still exposes an explicitly non-public adapter as public API, and one of the spec's core parity guarantees is not verified for metadata/theme-bearing code blocks on the same render surface.
