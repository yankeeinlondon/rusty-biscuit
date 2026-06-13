---
ready: false
agent: codex
model: ""
---

# Review

## Findings

### High: direct `CodeBlock` / `md code-block` still bypass environment theme fallback

The spec's terminal resolution chain for a standalone code block is explicit:
`CodeBlock.theme.or(DarkmatterPage.code_theme).or(env/default)` (`spec.md`, Theme
Resolution Policy and Public API Sketch). The boundary resolver also documents
that passing `None` for the override is what honors `THEME` (`darkmatter/lib/src/markdown/highlighting/resolve.rs:75-77`).

The fixed direct-render path now threads explicit `CodeBlock::with_theme(...)`
through correctly, but when no explicit theme is set it still constructs a
concrete fallback theme before calling the resolver:

- terminal: `theme_override.or_else(context.code_theme_name()).unwrap_or(ThemePair::OneHalf)` at `darkmatter/lib/src/markdown/render_tree/code_renderer.rs:173-176`
- browser: `CodeBlock::render_html_fragment` only passes `with_theme_override(self.theme)` and no `HtmlOptions` / env-derived theme at `darkmatter/lib/src/markdown/code_block.rs:363-369`
- CLI: `md code-block` constructs a direct `CodeBlock` and renders it directly at `darkmatter/cli/src/commands.rs:612-657`

That means `THEME=github md code-block 'fn main() {}' --language rust` and the
same direct library call without `with_theme(...)` resolve as `OneHalf`, not the
environment-selected theme. The new env fallback test covers
`DarkmatterPage::new(...).render(...)` only (`darkmatter/lib/src/layout/page.rs:4497-4539`),
where `TerminalOptions::default()` has already read the environment; it does not
cover the new atomic renderer or CLI surface.

Verification level present: Level 1 for explicit overrides only. Required:
Level 1 direct `CodeBlock` terminal/browser tests and CLI tests proving `THEME`
/ `CODE_THEME` affect output when `--theme` / `with_theme` is absent, plus
explicit override tests proving caller overrides still win.

### High: browser-visible code-block mode/theme behavior is not verified at browser tier

The spec requires browser rendering to use `DarkmatterPage`'s resolved page mode,
captured terminal mode, and `CodeBlockMode` for fenced code blocks
(`spec.md`, Decisions #5/#9 and Testing Requirements). This is user-observable
browser styling: the `.code-block` panel background and syntax-color markup must
compute to the intended colors in an actual browser.

The new tests added for review-1 are Level 1 string inspections inside
`page.rs`: they extract colors from generated HTML/CSS and compare luminance
(`darkmatter/lib/src/layout/page.rs:4274-4366`). They are useful, but they do not
exercise the browser parser/cascade/computed-style path. Existing browser-tier
coverage only checks a default `Markdown::as_html(HtmlOptions)` `.code-block`
background in Chrome (`darkmatter/lib/tests/browser_render.rs:31-64`); it does
not cover `DarkmatterPage::render_to_browser`, captured terminal dark/light
winning over fallback, `with_code_block_mode(Same)`, or computed syntax-span
colors under the page-resolved options.

Verification level present: Level 1 for the new page/mode/markup behavior, plus
older browser-tier coverage for a narrower default `.code-block` background
case. Required: Browser-tier computed-style tests for at least the
`DarkmatterPage::render_to_browser` path with captured dark and light terminal
modes, and a `CodeBlockMode::Same` vs `Inverse` case. If syntax colors remain
inline spans, query one representative span's computed `color` as well as the
panel's computed `background-color`.

## Production Readiness

Not ready for production.

The previous implementation blockers are mostly addressed, and the targeted L1
tests I ran pass:

```text
cargo nextest run -p darkmatter -E 'test(with_theme_changes) | test(browser_code_markup) | test(browser_code_block_honors_code_block_mode)' --no-tests=fail
```

Result: 5 tests run, 5 passed.

The remaining issues are both user-facing: the new direct code-block surface does
not honor the environment fallback promised by the spec, and the browser-visible
mode/theme behavior has not been verified at the required browser tier.
