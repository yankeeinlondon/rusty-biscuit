---
blast_radius: biscuit-terminal/lib/src/components/prose/
---
# Prose

Styled inline text component for rich cross-target output. Prose is the primary inline text styling component in biscuit-terminal and renders to the **Terminal**, **Browser**, and **Markdown** targets through the shared [`renderable::tree`](../../../renderable/src/tree/mod.rs) render tree. It supports two input grammars:

1. **Block tags** — `<bold>text</bold>` (auto-reset on close, nestable).
2. **Markdown subset** — `[desc](url)`, `**bold**`, `_italics_` (per the [Prose+ spec](../../features/2026-05-05-prose-plus/spec.md)).

The two grammars compose freely. Raw input is first lowered by the Markdown pre-processor (fenced code blocks → links → bold → italics), then parsed by the bracketed-tag parser **directly into** the shared `renderable::tree::RenderNode` shape. Terminal, browser, and Markdown output all fold that one tree through the shared tree renderers; Prose carries no component-local rendering IR.

> The atomic-token grammar (`{{bold}}…{{reset}}`) was removed in the
> 2026-05-17 Prose cross-target work. A stray `{{…}}` now renders as
> ordinary literal text on every target.

## Rendering Model

`Prose` parses its grammar **directly into** the shared
`renderable::tree::RenderNode` tree and renders every target through the shared
tree renderers — it carries no component-local rendering IR. It implements
`TreeRenderable`, and containers embed its parsed inline nodes via
`Prose::to_render_nodes()`.

```text
raw Prose input
      │
      ▼
Markdown subset pre-processor
      │
      ▼
bracketed-tag parser
      │
      ▼
renderable::tree::RenderNode
      │
      ├── shared terminal renderer  → ANSI / OSC8
      ├── shared browser renderer   → HTML fragment
      └── shared markdown renderer  → Markdown / MarkdownPlus
```

## Programmatic Use

```rust
use biscuit_terminal::prelude::*;

// Block tags (auto-reset after closing tag)
let prose = Prose::new("<bold>This is bold</bold> and <red>this is red</red>");

// Markdown subset (links, **bold**, _italics_)
let prose = Prose::new("hit _Esc_ to cancel — see [docs](https://example.com) for **details**");

// Hyperlinks
let prose = Prose::new(r#"<a href="https://example.com">Click here</a>"#);

// RGB colors
let prose = Prose::new("<rgb #ff0000>Red text</rgb>");

// Nested tags
let prose = Prose::new("<bold><blue>Bold blue text</blue></bold>");

// Escaping literal characters
let prose = Prose::new(r"\<not a tag\>");

// With layout configuration
let prose = Prose::new("Centered content")
    .with_layout(Layout {
        alignment: Alignment::Center,
        ..Layout::default()
    });

// Render
let term = Terminal::default();
println!("{}", prose.display(&term));
```

### Markdown Subset

Three Markdown forms are recognized in addition to bracketed tags. They are pre-processed into an equivalent internal tag form before the render tree is built.

| Markdown        | Equivalent tag                | Notes                                   |
|-----------------|-------------------------------|-----------------------------------------|
| `[desc](ref)`   | `<a href="ref">desc</a>`      | URL contents protected from later phases |
| `**text**`      | `<b>text</b>`                 | Only the doubled-asterisk form is bold  |
| `_text_`        | `<i>text</i>`                 | Only the single-underscore form is italic |

**Strict subset.** `__bold__` is not bold and `*italics*` is not italic — both pass through as literal text. This is intentional: the Prose+ spec keeps each emphasis style mapped to a single sigil so authored intent is unambiguous.

### Flanking Rules (Intra-word Inhibition)

To prevent identifiers, env-var names, and file paths from being chewed up by emphasis pre-processing, an emphasis delimiter (`_` or `**`) is treated as **literal text** when it sits between two word characters (Unicode alphanumeric). This is the "intra-word inhibition" rule.

| Input | Output | Why |
|-------|--------|-----|
| `OPENCODE_CONFIG_CONTENT` | `OPENCODE_CONFIG_CONTENT` | Every `_` is intra-word; no opener triggers |
| `foo_bar` | `foo_bar` | Single intra-word `_` |
| `_foo_bar_` | `<i>foo_bar</i>` | Outer `_` flanked by start/end; inner `_` intra-word and skipped |
| `foo**bar**baz` | `foo**bar**baz` | Both `**` runs intra-word |
| `**foo**bar**baz**` | `<b>foo**bar**baz</b>` | Outer `**` flanked; inner pairs intra-word |
| `(_text_)`, `hit _Esc_.` | `<i>text</i>`, `hit <i>Esc</i>.` | Punctuation neighbours form boundaries |
| `<dim>=OPENCODE_CONFIG_CONTENT</dim>` | unchanged | Tag wrapper preserved; intra-word `_` not triggered inside body |

The rule is symmetric — the same predicate gates openers and closers — and applies to both `_` (italics) and `**` (bold). The Prose+ implementation deliberately uses this simpler rule rather than full CommonMark left/right-flanking classification: terminal markup is overwhelmingly ASCII identifiers, predictability beats spec parity, and the simple rule covers every documented acceptance case.

### Escape Mechanism

A backslash escapes the immediately following character, treating it as literal text. Escapable characters are `*`, `_`, `[`, `]`, `(`, `)`, `<`, `>`, `{`, and `\` itself. Use this when you need a Markdown sigil to render literally **and** the flanking rule wouldn't already inhibit it.

| Input | Output |
|-------|--------|
| `\_text\_` | `_text_` (literal underscores, no italics) |
| `\*\*not bold\*\*` | `**not bold**` (literal asterisks) |
| `\\` | `\` (literal backslash) |
| `\<not a tag\>` | `<not a tag>` (literal angle brackets) |

In practice, dynamic content interpolated into a Prose format string usually does **not** require escaping — the flanking rule already protects identifier-shaped values. Reach for the escape mechanism when you need a literal sigil at a position where it *would* otherwise trigger emphasis (e.g. `\_emphasis_` where you want a leading literal `_`).

### Supported Tags

**Text Styling**: `bold`, `dim`, `italic`, `underline`, `strikethrough`, `blink`, `inverse` (alias `reverse`)

**Colors** (foreground): `red`, `green`, `blue`, `yellow`, `cyan`, `magenta`, `white`, `black`, plus bright variants (`bright-red`, etc.), Tailwind colors (`gray-800`, `blue-400`), and web colors (`coral`, `salmon`)

**Background Colors**: Prefix with `bg-` (e.g., `<bg-blue>`, `<bg-coral>`)

**Special**: `<a href="url">text</a>` for hyperlinks, fenced code blocks from the Markdown subset, and `<rgb #hex>text</rgb>` for arbitrary colors. Styles auto-reset when their tag closes — there is no standalone reset token.

> **Removed:** the `<hidden>` tag (SGR 8) is no longer recognized. It had zero callers across the workspace and was dropped when Prose moved to the shared render tree; `<hidden>text</hidden>` now renders as inert literal text like any unknown tag. `<inverse>` / `<reverse>` (SGR 7 reverse video) is supported and lowers to `filter: invert(1)` in the browser.

### Cross-Target Rendering

`Prose` parses its input into a target-neutral `renderable::tree::RenderNode`
tree and renders that tree to three targets:

| Target | Trait | Notes |
|--------|-------|-------|
| Terminal | `TerminalRenderable` | ANSI/OSC8; the behavioral oracle. Capability-aware degradation. |
| Browser | `BrowserRenderable` | Semantic HTML (`<strong>`, `<em>`, `<s>`, `<a>`, `<pre><code>`); presentational styles use `<span style="…">`. User text and attribute values are escaped. |
| Markdown | `MarkdownRenderable` | Portable Markdown and MarkdownPlus. Portable Markdown keeps semantic styles and degrades color/underline variants to readable inner text; MarkdownPlus preserves more presentation with inline HTML. |

`Prose` implements `TreeRenderable`: its `render_tree()` wraps the parsed
inline nodes into a document-shaped root (contiguous inline runs become
`Paragraph` blocks; a fenced code block stays a block-level `Code` child),
and `Prose::to_render_nodes()` exposes the parsed inline node sequence for
containers that embed Prose. In component inventories this means both
`Tree = ✅` and `IR State = tree render only` — Prose renders only through the
shared tree renderers, with no remaining component-local IR.

Unknown tags and former atomic-token syntax (`{{…}}`) render as escaped
literal text on every target.

### Prose in Other Components

`Todo` and `Status` both offer a `from_prose` constructor that renders the description through Prose at render time, so markup is resolved with full terminal context:

```rust
use biscuit_terminal::prelude::*;

let todo = Todo::from_prose("review <red>critical</red> PR");
let status = Status::from_prose("this is a <b>test</b>")
    .state(StatusState::Success);
```

### Prose in Table Cells

`TableCellContent::StyledProse(Box<Prose>)` lets a [`Table`](../../lib/src/components/table/README.md)
cell carry inline Prose without pre-rendering it to terminal bytes during
construction. Build a cell with `Prose::new(...).into()`:

```rust
use biscuit_terminal::prelude::*;
use biscuit_terminal::components::table::{Table, TableColumn};

let table = Table::new()
    .with_columns(vec![TableColumn::new("Feature"), TableColumn::new("Status")])
    .with_data(vec![vec![
        Prose::new("**Bold** feature").into(),
        Prose::new("[docs](https://example.com) — _ready_").into(),
    ]]);
```

Resolution is target-aware and follows the same render tree the standalone Prose
component uses:

- **Render tree** (Browser/Markdown, and the terminal tree path) — the cell
  projects Prose's parsed inline `RenderNode` children directly via
  `Prose::to_render_nodes()`, so semantic emphasis, links, and supported style
  attributes survive into `<td>` and into the GFM/MarkdownPlus table cell. A
  top-level fenced-code child degrades to escaped literal text so the projected
  table still passes render-tree validation.
- **Terminal bespoke path** — each `StyledProse` cell resolves to
  `Text(prose.render(term))` exactly **once**, before width planning, so the
  ANSI-aware table machinery measures by visible width and styles never bleed
  into borders, padding, or adjacent rows.

**Table-owned layout rule.** Prose's own outer `Layout` (margins, alignment,
word-wrap) is intentionally *not* promoted to nested cell layout — the table
owns all cell geometry. Only Prose's inline content participates in the cell;
column width, alignment, and wrapping come from the `TableColumn`. The cell hint
records `kind == "styled_prose"` with a null `raw_value`.

### Key API

| Method | Description |
|--------|-------------|
| `Prose::new(text)` | Create with styled content |
| `.content()` | Get the raw content string |
| `.with_word_wrap(WordWrap)` | Set word wrap strategy |
| `.with_left_margin(Margin)` | Set left margin |
| `.with_right_margin(Margin)` | Set right margin |
| `.with_layout(Layout)` | Set full layout configuration |
| `.to_render_nodes()` | Get the parsed inline `RenderNode` sequence (for container embedding) |
| `.render_html_fragment()` | Render the parsed render tree as a browser fragment |
| `.render_markdown()` | Render the parsed render tree as portable Markdown |
| `.render_markdown_plus()` | Render the parsed render tree as MarkdownPlus with inline HTML for richer styling |

## Graceful Degradation

`Prose` consults the active `Terminal`'s capability profile when it renders and
silently downgrades any markup that the terminal cannot display. The goal is
that low-capability emulators (Apple Terminal being the canonical example) see
clean, readable text instead of literal escape-code garbage.

Two markup categories degrade today: **OSC8 hyperlinks** and the
**double-underline** style (the `<double-underline>` block tag).

### OSC8 Hyperlinks

When `Terminal.osc_link_support == false`, the `<a href="…">…</a>` tag emits a
markdown-style fallback instead of the OSC8 escape sequence pair. This keeps
both the visible description and the URL on screen.

| Capability | Input | Output |
|------------|-------|--------|
| `osc_link_support == true` | `<a href="https://example.com">click here</a>` | `\x1b]8;;https://example.com\x1b\\click here\x1b]8;;\x1b\\` |
| `osc_link_support == false` | `<a href="https://example.com">click here</a>` | `[click here](https://example.com)` |

The fallback never emits the OSC8 introducer (`\x1b]8;;`) — verified by Level-1
PTY tests against a spoofed `TERM_PROGRAM=Apple_Terminal` profile.

### Double Underline

When `Terminal.underline_support.double == false`, the `<double-underline>`
block tag degrades according to the straight-underline capability:

| `double` | `straight` | Behavior |
|----------|------------|----------|
| `true`   | `true`     | Emit `\x1b[4:2m … \x1b[0m` (double underline) |
| `false`  | `true`     | Emit `\x1b[4m … \x1b[0m` (straight underline) |
| `false`  | `false`    | Emit plain text — no underline SGR codes |

The `\x1b[4:2m` sequence is **never** emitted when the active terminal does not
report double-underline support — verified by Level-1 PTY tests and Level-2
Apple Terminal harness tests.

## CLI

Exposed via `bt prose`:

```bash
bt prose "<bold>Hello</bold> <red>world</red>"
```

By default `bt prose` renders to the terminal. Pass `--html` to render an
HTML fragment, `--md` to render portable Markdown, or `--md-plus` to render
MarkdownPlus instead:

```bash
bt prose "<bold>Hello</bold> [docs](https://example.com)" --html
bt prose "<bold>Hello</bold> [docs](https://example.com)" --md
bt prose "<purple-800>Dark purple</purple-800>" --md-plus
```

Portable Markdown keeps semantic styling and drops terminal/browser-only
presentation such as colors. MarkdownPlus preserves richer styling with inline
HTML, for example:

```markdown
<span style="color: rgb(107, 33, 168)">Dark purple</span>
```

When `--md` is combined with horizontal layout flags, the CLI preserves those
page-level layout hints in YAML frontmatter:

```bash
bt prose "<b>bold</b>" --margin-left 4 --md
```

```markdown
---
style:
  page:
    margin-left: 4ch
---

**bold**
```

When `--html` is combined with layout flags, the CLI preserves them as CSS on
an outer wrapper around the Prose fragment:

```bash
bt prose "<b>bold</b>" --margin-left 4 --html
```

```html
<div style="margin-left: 4ch"><span class="prose"><strong>bold</strong></span></div>
```
