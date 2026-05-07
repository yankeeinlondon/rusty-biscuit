# Bad Assumptions: pulldown-cmark Option Coverage

Audit of how darkmatter addresses the runtime parser options exposed by `pulldown-cmark`.

## Option Sets by Pipeline Stage

Darkmatter uses **three distinct option sets** depending on the pipeline stage, deliberately enabling only a subset of pulldown-cmark's available extensions.

### 1. Core Parsing & Rendering: `ENABLE_TABLES | ENABLE_STRIKETHROUGH`

Used in 4 locations — this is the primary option set:

| Location | File | Line |
|----------|------|------|
| `markdown_parse_options()` (core struct parsing) | `mod.rs` | 820 |
| Terminal output | `terminal.rs` | 886 |
| HTML output | `html.rs` | 141 (no `ENABLE_TABLES`) |
| Reference extraction | `local.rs` | 22 |

**Why these two:**

- **`ENABLE_TABLES`** — Pipe tables are central to GFM; both terminal and HTML renderers have dedicated table-buffering logic.
- **`ENABLE_STRIKETHROUGH`** — `~~text~~` is needed so pulldown-cmark consumes `~~` pairs before darkmatter's `InlineStyleProcessor` runs. Without it, the `==` highlight syntax could conflict with the tilde-based strikethrough tokenization. The inline processor's tests explicitly pass `ENABLE_STRIKETHROUGH` to `Parser::new_ext`.

### 2. Cleanup Pipeline: `Options::all() - ENABLE_SMART_PUNCTUATION - ENABLE_DEFINITION_LIST`

Used in `cleanup.rs` (line 36) for the normalize/reformat pass:

- Enables everything except two intentionally excluded options:
    - **`ENABLE_SMART_PUNCTUATION` excluded** — Would convert `"` → `\u{201c}`/`\u{201d}` and `'` → `\u{2018}`/`\u{2019}` (curly quotes). Cleanup must preserve the user's original quote characters.
    - **`ENABLE_DEFINITION_LIST` excluded** — Darkmatter's `::file`, `::code`, `::url` transclusion directives start with `::`. Definition list parsing would mangle the `::` prefix into `: :`, breaking transclusion during the round-trip parse→serialize cycle.

### 3. Other Consumers

| Module | Options | Rationale |
|--------|---------|-----------|
| `toc/` | `Parser::new()` (default/none) | TOC extraction only needs headings, no extensions |
| `normalize/` | `Parser::new()` (default/none) | Heading-level normalization only needs headings |
| `compose/expression/lexer.rs` | Used for `{{ }}` scanning | Parser used for code-block boundary detection only, not for extension events |
| `messenger/` | `ENABLE_STRIKETHROUGH` only | Lightweight markdown→plain-text conversion |
| `research/` (tests) | `TABLES \| FOOTNOTES \| STRIKETHROUGH \| TASKLISTS` | Test-only, broader coverage |

## Options Deliberately Not Used (and Why)

| Option | Status | Rationale |
|--------|--------|-----------|
| `ENABLE_FOOTNOTES` / `ENABLE_OLD_FOOTNOTES` | Not enabled | Darkmatter doesn't render footnotes. Would add parse events nobody handles. |
| `ENABLE_TASKLISTS` | Not enabled (except research tests) | Task list checkboxes aren't rendered in terminal or HTML output. |
| `ENABLE_SMART_PUNCTUATION` | Explicitly excluded | Would mutate user's quote characters — unacceptable for a document processing tool that round-trips markdown. |
| `ENABLE_HEADING_ATTRIBUTES` | Not enabled | Darkmatter generates its own heading IDs via `generate_slug()`, not via pulldown-cmark's attribute parsing. |
| `ENABLE_YAML_STYLE_METADATA_BLOCKS` | Not enabled | Darkmatter extracts frontmatter using `serde_yaml_ng` directly (before parsing), not through pulldown-cmark's metadata block events. The `---` delimiter is consumed as a raw string split. |
| `ENABLE_PLUSES_DELIMITED_METADATA_BLOCKS` | Not enabled | No `+++` metadata support needed. |
| `ENABLE_MATH` | Not enabled | No math rendering in darkmatter. |
| `ENABLE_GFM` | Not enabled | The GFM extension currently adds alert-style blockquote support, which darkmatter doesn't render. Tables and strikethrough are handled individually. |
| `ENABLE_DEFINITION_LIST` | Explicitly excluded | Would break `::`-prefixed transclusion directives (`::file`, `::code`, etc.). |
| `ENABLE_SUPERSCRIPT` / `ENABLE_SUBSCRIPT` | Not enabled | No rendering support for these. |
| `ENABLE_WIKILINKS` | Not enabled | Darkmatter uses its own `{{ }}` interpolation and `::file` transclusion, not Obsidian-style wikilinks. |

## Custom Extensions Darkmatter Adds On Top

Instead of relying on more pulldown-cmark extensions, darkmatter implements its own via `InlineStyleProcessor` (an iterator adapter over pulldown-cmark's event stream):

- **`==highlight==`** → `InlineTag::Mark` → `<mark>` HTML / ANSI highlight in terminal
- **`⌄dim⌄`** → `InlineTag::Dim` → faint/dim ANSI style in terminal
- **Horizontal rule attributes** (`--- { style: waves, width: "50%" }`) via `RuleProcessor`

These are processed after pulldown-cmark parsing by intercepting `Event::Text` events, which is why `ENABLE_STRIKETHROUGH` is important — it ensures pulldown-cmark properly consumes `~~` pairs before the inline processor sees them.

## Potential Gaps Worth Evaluating

These are options darkmatter currently ignores that may deserve future consideration:

1. **`ENABLE_GFM`** — Adds alert-style blockquote tags (`> [!NOTE]`, `> [!WARNING]`, etc.) which are increasingly common in GitHub docs. If darkmatter ever renders GitHub-flavored callouts, this would need enabling in both the terminal and HTML renderers.

2. **`ENABLE_TASKLISTS`** — `- [x]` / `- [ ]` syntax is widely used. Terminal rendering could show checkbox state (☑/☐). Currently these render as plain text.

3. **`ENABLE_HEADING_ATTRIBUTES`** — Would allow users to write `{#custom-id .class}` on headings instead of relying solely on darkmatter's auto-generated slugs. Could coexist with `generate_slug()` as a fallback.

4. **`ENABLE_FOOTNOTES`** — If darkmatter ever targets academic or long-form document workflows, footnote rendering (both terminal and HTML) would be valuable.
