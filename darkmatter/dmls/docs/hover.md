---
features:
- 2026-07-04-dmls
---
# DMLS Hover Effects

Hover (`textDocument/hover`) is how DMLS explains the symbol under the cursor without navigating away from it — a schema property's type and description, a `ctx.*` variable, a directive's semantics, a wiki link's target, and so on. This page describes how hover content is produced, the hard limits on how it can look, and the formatting rules DMLS applies within those limits.

## The rendering contract

Every DMLS hover is an LSP `MarkupContent` with `kind = Markdown`. We hand the editor a **Markdown string**; the **editor** renders it using **its own theme**.

That split is the whole story of "how much control we have":

- We choose the **Markdown constructs**.
- The editor decides how each construct *looks* — font, weight, and especially   **color**.

We cannot set colors, backgrounds, opacity, or CSS, and editors strip inline HTML (`<span style="color:…">` and friends) from hover content. So there is **no way to make hover text a specific color, or to "dim" it** — those are theme decisions, identical across VS Code, Zed, Neovim, and Helix. This is an LSP/editor limitation, not a DMLS one.

What a construct becomes when the editor renders it:

| We emit                                         | Rendered as                                                |
|-------------------------------------------------|------------------------------------------------------------|
| `` `name` `` (inline code)                          | a boxed / inverted background (theme-colored)          |
| `**name**` (bold)                               | bold weight, no box                                        |
| `_name_` (italic)                               | italic                                                     |
| `# name` / `## name` (heading)                  | larger / heavier heading                                   |
| lists, block quotes, links, tables, fenced code | supported (fenced code is syntax-highlighted by the theme) |
| raw HTML, hex colors, ANSI codes                | **ignored / sanitized**                                    |

The only places color ever appears are constructs the *theme* colors — inline code, links (the accent color, but underlined and clickable), and syntax-highlighted fenced blocks. None of these is a reliable lever for "make this token blue," so DMLS does not try; it expresses hierarchy with weight and style instead.

## Formatting rule: frontmatter schema hover

Hovering a schema-declared frontmatter property renders its type, whether it is required, its enum values, its default, and its `->` description. The style rule is chosen to convey hierarchy using only the constructs above:

| Element                      | Markup                                  | Rationale                                       |
|------------------------------|-----------------------------------------|-------------------------------------------------|
| the property being described | `` `area` `` (inline-code box)              | the box marks *the subject* of the hover        |
| its type                     | `Type: **string**` (bold)               | emphasized, but distinct from the boxed subject |
| enum values                  | `Values: _draft_, _published_` (italic) | softer metadata                                 |
| default value                | `Default: _draft_` (italic)             | softer metadata                                 |
| required flag                | `Required` (plain)                      | a plain word needs no emphasis                  |
| description                  | plain paragraph                         | the prose the author wrote                      |

The mental model: **box = the thing being described; bold = its type; italic = its example/enum/default values.** (An earlier idea — blue type, dim default — was dropped because color and dim are not expressible in hover Markdown.)

The body is assembled by the pure `schema_hover_body` function in `src/providers/frontmatter.rs`, kept free of any LSP session state so the rule is unit-testable in isolation.

## What answers a hover

Hover is a provider-chain capability: each provider may contribute, and the registry keeps the **first non-empty** result. Providers run in registration order, so a more specific overlay wins over the substrate for the same offset.

| Provider (in order)  | Hovers on                                            | Content                                                                                                   |
|----------------------|------------------------------------------------------|-----------------------------------------------------------------------------------------------------------|
| substrate (Markdown) | a link                                               | graph-sourced target preview (no disk read)                                                               |
| wiki                 | `[[target]]` / `[[target#heading]]`                  | resolved target + heading preview                                                                         |
| frontmatter          | a frontmatter key or its value                       | schema type/required/enum/default/description; or a `ctx.*` generated-key annotation                      |
| DSL                  | a directive, `{{ }}` interpolation, or a `$()` value | directive semantics + resolved target; interpolation's static value or `ctx.*` note; shell policy verdict |

Because the chain stops at the first non-empty hover, a schema-defined frontmatter key whose value contains `{{ }}` is explained by the **frontmatter** provider (the schema description), while an *undefined* key's `{{ … }}` value falls through to the **DSL** provider's interpolation hover.

## Passivity

Hover is read-only like every DMLS request: it resolves local files and reads the in-memory graph, but never executes a shell command (`$(...)`, `::shell`), fetches a remote URL, or mutates anything. A `$()` or `::shell` hover *explains* what compose would do and whether policy allows it — it never does it.

## Testing

The formatting rule is covered at two levels:

- **Unit** (`src/providers/frontmatter.rs`): `schema_hover_body` is called with   hand-built `PropertyDef`s and the emitted Markdown is asserted directly (type   is bold and not inline code; enum members and default are italic). No LSP   session is needed because the function is pure.
- **Integration** (`tests/lsp_session.rs`): a full   `initialize → didOpen → textDocument/hover` session over the in-memory   connection asserts the rendered `contents.value` the editor would receive. The   `hover_markup(fixture, uri, line, character)` helper collapses the   request-and-extract boilerplate so hover assertions stay one line.
