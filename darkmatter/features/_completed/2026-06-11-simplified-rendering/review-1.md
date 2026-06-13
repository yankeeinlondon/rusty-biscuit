---
ready: false
agent: codex
model: ""
---

# Review

## Findings

### High: `CodeBlock::with_theme` and `md code-block --theme` are no-ops

The spec makes `CodeBlock` the atomic rendering component and includes `with_theme(mut self, theme: ThemePair)` in the expected public surface (`spec.md:218-224`). The CLI also promises `md code-block --theme <theme>` as a direct `CodeBlock` override (`spec.md:728-731`).

The implementation stores the override on `CodeBlock` (`darkmatter/lib/src/markdown/code_block.rs:118-123`), and the CLI sets it (`darkmatter/cli/src/commands.rs:617-620`), but neither terminal nor browser rendering ever reads it. `CodeBlock::render` projects only a render-tree code node and installs `TerminalCodeRenderer::for_terminal(term, CodeBlockMode::default())` (`darkmatter/lib/src/markdown/code_block.rs:299-305`); `code_node()` carries language/meta/layout only (`darkmatter/lib/src/markdown/code_block.rs:243-252`). Browser rendering similarly installs `TerminalCodeRenderer::new()` with no theme override (`darkmatter/lib/src/markdown/code_block.rs:356-360`). The renderer then falls back to context/default theme state.

User impact: direct library callers and `md code-block --theme nord` get output rendered with the default theme, not the requested theme. The new CLI tests do not catch this: `code_block_theme_override_is_accepted` only checks that both outputs contain ANSI and the body text, not that `github` and `nord` differ (`darkmatter/cli/tests/code_block.rs:473-507`), and the HTML variant only runs one theme (`darkmatter/cli/tests/code_block.rs:509-532`).

Verification level present: Level 1 smoke only. Required: Level 1 assertions that theme override changes the resolved terminal and HTML output, plus existing Level 2 terminal capture is useful for rendered SGR/color behavior.

### High: browser code rendering ignores `DarkmatterPage` mode/theme policy

The spec requires browser rendering to use the same `DarkmatterPage` mode policy as terminal rendering: captured terminal mode wins, `Unknown` falls back to configured mode/default dark, and fenced code blocks use `CodeBlockMode` against that mode (`spec.md:98-101`, `spec.md:394-397`, `spec.md:722-724`).

`DarkmatterPage::render_to_browser` correctly builds `HtmlOptions` with the page's resolved `code_theme` and `ctx.render_color_mode` (`darkmatter/lib/src/layout/page.rs:993-1000`). However, the browser code renderer discards those options and constructs `HtmlOptions::default()` inside `render_browser_code` (`darkmatter/lib/src/markdown/render_tree/code_renderer.rs:220-244`). It also has a TODO that `CodeBlockMode` is not honored for browser (`darkmatter/lib/src/markdown/render_tree/code_renderer.rs:232-233`).

This leaves the page wrapper and stylesheet using one policy while the syntax-highlighted code markup uses another. The nearby stylesheet comment says the `.code-block` rule and highlighted markup agree (`darkmatter/lib/src/markdown/render_tree/entrypoints.rs:552-558`), but that is drift: the stylesheet is built from the caller's `HtmlOptions`, while the markup uses defaults.

The new browser tests only parse the `.code-block` stylesheet background (`darkmatter/lib/src/layout/page.rs:4138-4207`, `darkmatter/lib/src/layout/page.rs:4420-4459`). They do not assert the actual code renderer's resolved highlighter/theme, so they can pass while the highlighted code remains on the wrong theme.

Verification level present: Level 1 string/CSS extraction. Required: at least Level 1 that compares actual rendered code markup/theme behavior for configured dark/light and `CodeBlockMode`, and browser-tier computed-style coverage for user-visible browser styling.

### Medium: public `CodeBlock::new` shape does not match the spec

The spec's public API sketch has `pub fn new(code: impl Into<String>) -> Self` and a separate `with_fence_language(...)` builder (`spec.md:218-221`). The implementation exposes `pub fn new(code: impl Into<String>, language: Option<impl AsRef<str>>) -> Self` instead (`darkmatter/lib/src/markdown/code_block.rs:78-84`).

That is workable internally, but it misses the intended ergonomic split: `CodeBlock::new(code)` for plain code and `CodeBlock::new(code).with_fence_language("rust")` for language selection. It also forces callers to spell `None::<String>` / `None::<&str>` in otherwise simple examples and tests.

## Production Readiness

Not ready for production.

The core simplification direction is sound, and there is useful Level 1 plus existing Level 2 terminal coverage around code panels. The blockers above mean two public user-facing promises are currently broken or insufficiently verified: direct theme overrides do not work, and browser code rendering does not use the page's resolved mode/theme policy.
