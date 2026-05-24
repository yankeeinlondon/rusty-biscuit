Stylesheet is genuinely cross-target (the spec already references it from render_markdown_with_style(Option<StyleSheet>), and BrowserRenderable will need it for inline CSS variables). But there are several real entanglements to plan for. Let me lay them out.

That [source file](@darkmatter/lib/src/render/stylesheet.rs) is 2019 lines, not just a struct. To move Stylesheet you also move:

- Property type system: CssProp, CssSizingProp, CssSizingMultiProp, CssColorProp, CssIntegerProp, CssCustomProp, CssTypedProperty
- Value type system: CssUnit, CssSizing, CssSizingMulti, CssColor, CssRaw, CssValue, CssValueKind, IntoCssValue
- Error type: StylesheetError
- Output methods: to_css, to_terminal_string, to_terminal, to_json, to_json5

All of it is one tightly-coupled subsystem; you can't cleanly cherry-pick the container.

### Issue 1 — Direction-of-dependency reversal (the big one)

The spec moves BrowserRenderable up into renderable, with biscuit-terminal depending on renderable. But stylesheet.rs currently
depends on biscuit-terminal:

┌──────────────────┬──────────────────────────────────────────────────────┬───────────────────────────────────────────────────┐
│       Line       │                     Symbol used                      │                      Impact                       │
├──────────────────┼──────────────────────────────────────────────────────┼───────────────────────────────────────────────────┤
│ stylesheet.rs:10 │ biscuit_terminal::components::prose::Prose           │ Used in to_terminal_string for ANSI coloring      │
├──────────────────┼──────────────────────────────────────────────────────┼───────────────────────────────────────────────────┤
│ stylesheet.rs:11 │ biscuit_terminal::components::renderable::Renderable │ Called via Prose::render(terminal)                │
├──────────────────┼──────────────────────────────────────────────────────┼───────────────────────────────────────────────────┤
│ stylesheet.rs:12 │ biscuit_terminal::terminal::Terminal                 │ Parameter type for to_terminal_string/to_terminal │
├──────────────────┼──────────────────────────────────────────────────────┼───────────────────────────────────────────────────┤
│ stylesheet.rs:65 │ biscuit_terminal::errors::BlockError                 │ impl BlockError for StylesheetError               │
└──────────────────┴──────────────────────────────────────────────────────┴───────────────────────────────────────────────────┘

If renderable becomes a dependency of biscuit-terminal, then renderable cannot also depend on biscuit-terminal — that's a cycle.

Solution Approach:

Strip terminal rendering from `Stylesheet`'s new home. Move only the data + CSS-text rendering (to_css, to_json, to_json5). Add
an extension trait or free function in biscuit-terminal that re-introduces to_terminal_string(&Stylesheet, &Terminal). (Cleanest
layering; small ergonomic loss — stylesheet.to_terminal_string(t) becomes a free call.)

Secondary Concern and Solution:

Currently `stylesheet.rs` is a "god file", let's make sure the **style** module has the stylesheet symbols spread across multiple files in a sensible manner.

### Issue 2 — BlockError trait

`impl biscuit_terminal::errors::BlockError for StylesheetError (line 65)`: is in the same boat as the terminal rendering — it's a
back-dep on biscuit-terminal. Same fix as #1.

### Issue 3 — Cargo deps that travel with it

`renderable/src/Cargo.toml` is currently empty. Moving Stylesheet adds:
- serde_json (used in to_json)
- thiserror (StylesheetError)

We need to make sure these dependencies are added to the `renderable` library.

### Issue 4 — darkmatter consumers

Inside darkmatter, Stylesheet is used by:
- render/image_ref.rs (8 sites)
- render/link.rs (8 sites)
- markdown/errors/mod.rs (downcasts StylesheetError)
- markdown/yaml_block.rs (likely — uses Renderable/BrowserRenderable)
- Three test files

These are pure path renames (crate::render::stylesheet::* → renderable::stylesheet::*). No behavior changes. Easy.

### Issue 5 — Naming

**Resolved** by [`decisions.md`](./decisions.md) item 10 (Scheme A): the
existing single-block `Stylesheet` struct is renamed `CssStyle`; the name
`Stylesheet` is reassigned to the *collection* type (formerly
`HtmlStyleSheet`), and the `(selector, block)` pair becomes `CssRule`. "Stylesheet"
is one word throughout. The rename cost is trivial — no external consumers
outside darkmatter.

### Steps

- renderable::stylesheet — the data model, validation, and target-agnostic emit (to_css, to_json, to_json5, Display).
- biscuit-terminal::stylesheet_ext (or similar) — to_terminal_string / to_terminal as an extension trait or free fns over
&Stylesheet + &Terminal, plus impl BlockError for StylesheetError if BlockError stays in biscuit-terminal.

That gives every render target (MarkdownRenderable, BrowserRenderable, TerminalRenderable) equal access to Stylesheet without
forcing renderable to know what a terminal is.
