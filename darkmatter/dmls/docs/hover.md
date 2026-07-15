---
features:
- 2026-07-04-dmls
- 2026-07-08-modal-and-autocomplete
- 2026-07-10-interpolation-literal
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

## Formatting rule: interpolation `ctx.*` hover

Inside a `{{ … }}` body interpolation, a cursor on an explicitly `ctx.`-qualified variable renders the shared catalog-backed block followed by an interpolation-only note. The variable counts as the root through member access and index access too (for example `ctx.packages[0].first`), because the lexer keeps a dotted `ctx.packages` as a single token.

The catalog-backed block is single-sourced through one adapter, `overlay::expressions::format_ctx_hover_block`, which the frontmatter `ctx.*` hover also renders — so the two surfaces produce the same bytes. The block carries the qualified name, the rendered type, the read-only / Darkmatter-owned ownership note, and the description:

| Element      | Markup                                         | Rationale                                        |
|--------------|------------------------------------------------|--------------------------------------------------|
| the variable | `` `ctx.<name>` ``(inline-code box) + `(type)` | the box marks the subject; the type rides inline |
| ownership    | `read-only, Darkmatter-owned` (plain)          | a passive reminder, not a captured value         |
| description  | plain paragraph                                | the prose the catalog author wrote               |

After the block, the interpolation surface appends one interpolation-only line:

> The `ctx` variable is evaluated at _compose_ time (rather than now).

The frontmatter surface does not emit that note — it is DMLS-owned because it describes passive editor behavior (DMLS never captures host context or evaluates the expression to build hover content).

Classification requires an explicit `ctx.` prefix (the D2 rule): a bare `{{ today }}` is a **frontmatter** variable even when `today` is also a known context-variable tail, and an unknown `ctx.<name>` keeps the generic expression hover without borrowing a similarly named bare key's value. The hover range remains the complete `{{ … }}` expression.

## Formatting rule: function-call hover

A cursor on a known function-name identifier — the name token of a `FunctionCall` such as `as_csv(...)` — renders the catalog typed signature and description through `overlay::expressions::format_function_block`.

| Element      | Markup                                     | Rationale                          |
|--------------|--------------------------------------------|------------------------------------|
| the function | `` `<typed signature>` ``(inline-code box) | the box marks the subject          |
| description  | plain paragraph                            | the prose the catalog author wrote |

Cursor precedence: the function name wins on its own name identifier; an offset on an argument, parenthesis, or comma falls through to that inner expression's `ctx.*` or frontmatter hover (`function_call_at` declines on a non-name offset, so `expression_at` serves the argument). Unknown functions keep the generic `**Expression**` hover. The hover range remains the complete `{{ … }}` expression.

## Formatting rule: bare-identifier (frontmatter variable) hover

A bare identifier inside `{{ … }}` is a **frontmatter variable** (the D2 rule above). When the document sets the key, hover shows its static value. When the key is *declared by the effective schema but unset in the document* — a caller-supplied parameter injected at compose time — hover falls back to the schema block: type, constraints, and `->` description, rendered by `frontmatter::schema_hover_details` (the schema body without the leading `` `key` `` heading, so the name is not duplicated under the `**Expression**` header). A set frontmatter value always wins over the schema block.

## Formatting rule: interpolation literal hover

A cursor anywhere inside an interpolation literal (`{{{ … }}}`) renders an `**Interpolation literal**` block showing the composed output — the inner content wrapped in `{{ … }}` as inline code (a fenced block when multiline, with backtick fences sized to survive content containing backticks) — followed by the note that the content is rendered as literal `{{ … }}` text and is not interpolated. The body is assembled by the pure `literal_hover_markdown` function in `src/providers/dsl.rs`. The hover range is the literal's complete outer `{{{ … }}}` span.

## What answers a hover

Hover is a provider-chain capability: each provider may contribute, and the registry keeps the **first non-empty** result. Providers run in registration order, so a more specific overlay wins over the substrate for the same offset.

| Provider (in order)  | Hovers on                                            | Content                                                                                                   |
|----------------------|------------------------------------------------------|-----------------------------------------------------------------------------------------------------------|
| substrate (Markdown) | a link                                               | graph-sourced target preview (no disk read)                                                               |
| wiki                 | `[[target]]` / `[[target#heading]]`                  | resolved target + heading preview                                                                         |
| frontmatter          | a frontmatter key or its value                       | schema type/required/enum/default/description; or a `ctx.*` generated-key annotation                      |
| DSL                  | a directive, `{{ }}` interpolation, `{{{ }}}` literal, or a `$()` value | directive semantics + resolved target; interpolation's static value, schema-property fallback, or `ctx.*` note; literal composed-output + inert note; shell policy verdict |

Because the chain stops at the first non-empty hover, a schema-defined frontmatter key whose value contains `{{ }}` is explained by the **frontmatter** provider (the schema description), while an *undefined* key's `{{ … }}` value falls through to the **DSL** provider's interpolation hover.

## Passivity

Hover is read-only like every DMLS request: it resolves local files and reads the in-memory graph, but never executes a shell command (`$(...)`, `::shell`), fetches a remote URL, or mutates anything. A `$()` or `::shell` hover *explains* what compose would do and whether policy allows it — it never does it.

## Testing

The formatting rule is covered at two levels:

- **Unit** (`src/providers/frontmatter.rs`): `schema_hover_body` is called with   hand-built `PropertyDef`s and the emitted Markdown is asserted directly (type   is bold and not inline code; enum members and default are italic). No LSP   session is needed because the function is pure.
- **Integration** (`tests/lsp_session.rs`): a full   `initialize → didOpen → textDocument/hover` session over the in-memory   connection asserts the rendered `contents.value` the editor would receive. The   `hover_markup(fixture, uri, line, character)` helper collapses the   request-and-extract boilerplate so hover assertions stay one line.
