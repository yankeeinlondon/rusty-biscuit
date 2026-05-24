---
ready: false
agent: codex
model: ""
---

# Review: Sub-Spec #5 Color & Background-Color Mutations

## Findings

### High: `ColorDepth::None` bypasses terminal rendering instead of preserving visible layout

The spec requires color settings to emit no SGR at `ColorDepth::None` while the visible text/layout remains unchanged. The current terminal renderer exits early for `ColorDepth::None` and writes the raw Markdown source ([darkmatter/lib/src/markdown/output/terminal.rs](/Users/ken/.claudine/worktrees/rusty-biscuit/renderable/darkmatter/lib/src/markdown/output/terminal.rs:897)). That means a styled table/list/code block rendered with color disabled no longer goes through table layout, list layout, code-block rendering, or component decoration; it falls back to source text.

This is especially visible now that color-only style makes `DarkmatterPage` non-default and can thread explicit color depth into rendering. `style.page.color` plus `ColorDepth::None` should produce the same visible terminal layout as without the color, just without color SGR. It currently produces raw Markdown.

Verification level present: Level 1, but the new test only checks absence of `38;2` bytes against a heading ([darkmatter/lib/src/layout/page.rs](/Users/ken/.claudine/worktrees/rusty-biscuit/renderable/darkmatter/lib/src/layout/page.rs:2770)); it does not assert table/list/code visible layout parity. Required: Level 1 parity test for `ColorDepth::None` and Level 2 capture for visible terminal layout.

### High: `style.ul.color` / `style.ol.color` is erased from list item bodies when `style.li.color` is unset

The terminal renderer pushes a `Ul`/`Ol` color scope at list start ([darkmatter/lib/src/markdown/output/terminal.rs](/Users/ken/.claudine/worktrees/rusty-biscuit/renderable/darkmatter/lib/src/markdown/output/terminal.rs:1379)), then pushes a `Li` color scope after the marker for every item ([darkmatter/lib/src/markdown/output/terminal.rs](/Users/ken/.claudine/worktrees/rusty-biscuit/renderable/darkmatter/lib/src/markdown/output/terminal.rs:1486)). `LayoutContext::component_color(Li)` falls back only to page color, not the active list-container color ([darkmatter/lib/src/layout/context.rs](/Users/ken/.claudine/worktrees/rusty-biscuit/renderable/darkmatter/lib/src/layout/context.rs:253)). The color stack uses only the top entry ([darkmatter/lib/src/markdown/output/terminal.rs](/Users/ken/.claudine/worktrees/rusty-biscuit/renderable/darkmatter/lib/src/markdown/output/terminal.rs:2504)), so an unset `Li` pushes `(None, None)` and clears the outer `Ul`/`Ol` color for the body text.

The result is that `style.ul.color` colors the marker but not the list item body, and `style.page.color + style.ul.color` makes the body inherit the page color instead of the explicit `ul` override. Browser CSS inherits correctly through `ul`/`ol`, so terminal and browser diverge.

Verification level present: no targeted test. The browser list selector test only checks selector presence ([darkmatter/lib/src/layout/page.rs](/Users/ken/.claudine/worktrees/rusty-biscuit/renderable/darkmatter/lib/src/layout/page.rs:2930)), and there is no terminal assertion that `ul`/`ol` colors apply to item body text. Required: Level 1 byte test for `ul`/`ol` body color inheritance plus Level 2 terminal capture because this is user-visible styling.

### High: Hyperlink color is not applied to links inside tables

`style.hyperlinks.color` pushes a wrapper color scope on every `Start(Link)` ([darkmatter/lib/src/markdown/output/terminal.rs](/Users/ken/.claudine/worktrees/rusty-biscuit/renderable/darkmatter/lib/src/markdown/output/terminal.rs:1296)), but table links are rendered through `push_table_link`, which bypasses the wrapper and passes `None` for component foreground/background ([darkmatter/lib/src/markdown/output/terminal.rs](/Users/ken/.claudine/worktrees/rusty-biscuit/renderable/darkmatter/lib/src/markdown/output/terminal.rs:2120)). So `style.hyperlinks.color` works for normal prose links, but not for links inside table cells.

The spec says `PageComponent::Hyperlinks` maps to terminal link label text while preserving OSC8 sequences; it does not exclude table-cell links. This also prevents hyperlink color from overriding table color inside tables.

Verification level present: Level 1 only for a prose link ([darkmatter/lib/src/layout/page.rs](/Users/ken/.claudine/worktrees/rusty-biscuit/renderable/darkmatter/lib/src/layout/page.rs:2957)). Required: Level 1 table-link byte test, and Level 2 capture for OSC8/styling interaction in a real terminal.

### High: User-visible color rendering is not verified at the required level

Most sub-spec #5 requirements are user-observable terminal or browser rendering behavior: foreground/background SGR, resets at component boundaries, code-block panel background without clobbering highlighting, hyperlink label styling with OSC8, list selector behavior, opacity-preserving browser CSS, and special CSS colors. The implementation adds many in-process assertions in `layout/page.rs`, but they are mostly broad substring checks such as `contains("\x1b[38;2;")`, `contains("color: rgb(")`, or selector presence.

Per the requested rigor model, terminal glyph/styling/width/reset behavior needs Level 2 real-terminal capture, and browser CSS should be checked through computed style in a real browser. No new Level 2 color cases were added; the only touched Level 2 file changes an existing list alignment test and does not cover sub-spec #5 color behavior ([darkmatter/cli/tests/level2_layout.rs](/Users/ken/.claudine/worktrees/rusty-biscuit/renderable/darkmatter/cli/tests/level2_layout.rs:697)).

This is a readiness blocker by the review instructions: a feature may be marked production-ready only when each user-observable requirement has the appropriate verification level.

## Requirement Coverage

- Page/component color storage and effective accessors: Level 1 present.
- Active wiring warnings for sub-spec #5 keys: Level 1 present.
- Terminal SGR emission/reset boundaries: Level 1 partial, Level 2 missing.
- `ColorDepth::None` visible-layout preservation: implemented incorrectly; Level 1 inadequate and Level 2 missing.
- List color routing: terminal implementation incomplete for item bodies; Level 1 and Level 2 missing.
- Hyperlink color routing: prose links partially covered; table-cell links missing; Level 2 missing.
- Browser CSS opacity and special colors: Level 1 string checks only; computed-style browser coverage missing.
- Code-block background without clobbering highlighting: Level 1 partial; Level 2 missing for real terminal rendering.

## Verification

Attempted targeted runs:

- `cargo test -p darkmatter color_ -- --nocapture`
- `cargo test -p darkmatter style_frontmatter -- --nocapture`

Both commands were still compiling dependencies and contending on Cargo locks beyond the non-interactive time budget, so I stopped the cargo processes. No completed test result is available from this review pass.

## Production Readiness

Not ready. The API/storage plumbing is mostly present, but terminal rendering still has functional gaps for color-disabled output, list body inheritance, and table-cell hyperlinks. The user-visible rendering surface also does not meet the required Level 2/browser verification bar.
