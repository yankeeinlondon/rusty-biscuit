# Styling Utilities

Terminal-aware styling functions and the Prose component for rich text rendering.

## Styling Functions

All functions check terminal capabilities and fall back gracefully:

```rust
use biscuit_terminal::utils::styling::*;
use biscuit_terminal::terminal::Terminal;

let term = Terminal::new();

// Italic (checks supports_italic and is_tty)
let text = italic("emphasized", &term);

// Underline variants (check underline_support)
let text = underline("important", &term);
let text = double_underline("very important", &term);
let text = curly_underline("error", &term);      // LSP-style
let text = dotted_underline("warning", &term);
let text = dashed_underline("hint", &term);
```

### Fallback Behavior

```rust
// If curly not supported, falls back to straight
let text = curly_underline("error", &term);
// Returns "\x1b[4:3m...\x1b[0m" or "\x1b[4m...\x1b[0m"

// If no underline support, returns plain text
```

### Underline Escape Codes

| Style | Code |
|-------|------|
| Straight | `\x1b[4m` or `\x1b[4:1m` |
| Double | `\x1b[4:2m` |
| Curly | `\x1b[4:3m` |
| Dotted | `\x1b[4:4m` |
| Dashed | `\x1b[4:5m` |
| Colored | `\x1b[58:2::R:G:Bm` |

## Prose Component

Two components share one grammar (bracketed tags plus a Markdown subset) and
parse directly into the shared `renderable::tree::RenderNode` tree; Terminal,
Browser, Markdown, and MarkdownPlus all render through the shared tree
renderers, with no component-local IR.

- **`Prose`** is **block** content: paragraphs (split on blank lines) and
  fenced code blocks under a `Root`, with a `Layout` applied on every target
  (CSS in HTML) and a `ProseTag` choosing each paragraph's HTML element
  (default `<p>`). Use it for anything printed as its own message. A code
  block (fenced or `<code-block>`) inside a style or link splits the
  paragraph: the code is a sibling block and every enclosing wrapper resumes
  on both sides (`tree::split_around_blocks`, the one split used for styles,
  links, and paragraphs). A paragraph edge meeting a code block drops its
  breaks and spaces (`blocks::trim_edges_at_code`). Soft-break whitespace is
  trimmed once over the whole inline sequence
  (`tree::trim_soft_break_whitespace`), so it crosses wrapper boundaries.
- **`InlineProse`** is **inline**: one neutral `Span` of phrasing nodes, no
  layout builders, no outer HTML element. Use it for table cells, labels,
  badges, and values inside a line. A fenced block in it becomes one
  `InlineCode` (language dropped, line endings → spaces).

```rust
use biscuit_terminal::components::prose::Prose;
use biscuit_terminal::components::renderable::TerminalRenderable;

// Create prose with inline styling
let prose = Prose::new("Hello <b>world</b>!");
let output = prose.render_optimistic(None);
// → "Hello \x1b[1mworld\x1b[0m!\x1b[0m"

// With builder pattern
let prose = Prose::new("<b>Important</b> message")
    .with_word_wrap(WordWrap::WrapProse(None, None))
    .with_left_margin(TargetValue::universal(Length::ch(4)))
    .with_tag(ProseTag::Div);

// Inline content
let cell = InlineProse::new("run `md hash`");
```

### Newlines

Markdown rules, the same in both components:

| Input | Meaning |
|---|---|
| single `\n` | soft break (a space on terminal/HTML), spaces and tabs around it dropped, also across a style or link boundary |
| two or more `\n` (only spaces/tabs between) | paragraph boundary in `Prose`; one soft break in `InlineProse` |
| `\` before `\n` (Rust: `"a\\\nb"`) | hard break (`<br>`; Markdown `\` + newline) |
| trailing spaces before `\n` | **not** a hard break |

CRLF and lone CR are read as LF. Text that uses single `\n` as line breaks
needs `.with_line_breaks(LineBreaks::Hard)` (on either component); without it
the lines join. The implicit conversions are soft too: a `String` passed to
`StatusBlock::body`, a hint, a `Status::from_prose` description, and an
`InlineProse` table cell. Callers that build rows (`"Heading:\n- a\n- b"`,
`.join("\n")`, an error `Display` with a `hint:` line, pre-rendered component
output) pass `Prose::new(s).with_line_breaks(LineBreaks::Hard)` instead. A
leading or trailing `\n` meant as a blank spacing row is trimmed in both
modes, so put that newline outside the component. `Status::from_prose` takes
`.with_line_breaks(LineBreaks::Hard)` too: each newline starts a new row as
written.

### Block Tags

HTML-like tags that auto-reset. Former atomic tokens (`{{bold}}`,
`{{reset}}`, etc.) were removed in the 2026-05-17 Prose cross-target work and
now render as literal text.

| Token | Alias | Effect |
|-------|-------|--------|
| `<bold>` | `<b>` | Bold |
| `<dim>` | | Dim |
| `<italic>` | `<i>` | Italic |
| `<underline>` | `<u>` | Underline |
| `<double-underline>` | `<uu>` | Double underline |
| `<curly-underline>` | | Curly underline |
| `<dotted-underline>` | | Dotted underline |
| `<dashed-underline>` | | Dashed underline |
| `<blink>` | | Blinking |
| `<inverse>` | `<reverse>` | Inverse video |
| `<strikethrough>` | `<~>` | Strikethrough |
| `<a href="...">` | | OSC8 link |
| `<rgb 255,128,0>` | | RGB foreground |
| `<red>`, `<bright-red>` | | Basic/bright colors |
| `<coral>`, `<alice-blue>` | | Web/CSS colors |
| `<purple-500>`, `<slate-700>` | | Tailwind colors |
| `<clipboard>` | | Clipboard content |

#### Color Blocks

Foreground colors:
```
<red>basic color</red>
<bright-red>bright variant</bright-red>
<alice-blue>web/CSS color</alice-blue>
<purple-500>Tailwind color</purple-500>
<rgb 255,0,0>RGB color</rgb>
```

Background colors (prefix a web/CSS or Tailwind name with `bg-`, or use
`bg-rgb`; basic and bright names such as `bg-blue` stay literal text):
```
<bg-coral>web/CSS background</bg-coral>
<bg-navy>web/CSS background</bg-navy>
<bg-red-800>Tailwind background</bg-red-800>
<bg-rgb 255,128,0>RGB background</bg-rgb>
```

#### Hyperlinks

OSC8 links with relative path support:
```
<a href="https://example.com">URL</a>
<a href="/absolute/path">absolute file</a>
<a href="./relative/path">relative to CWD</a>
<a href="src/main.rs">relative to CWD, then git root</a>
```

The tree, Markdown, and HTML keep the authored `href`. Only the terminal
renderer resolves it (`render_tree::link`), with `biscuit_file::FileReference`
grammar, against the CWD at render time; a missing file still links to its
CWD-joined path, and a URL with a scheme or a `#fragment` is unchanged. Prose
marks its links with the `biscuit_terminal.link` hint; unmarked links (e.g.
darkmatter's) are emitted as authored.

Example:
```
"Click <a href=\"https://example.com\">here</a> for more"
→ "Click \x1b]8;;https://example.com\x07here\x1b]8;;\x07 for more"
```

### Markdown Subset

A strict CommonMark subset is recognised in addition to block tags. Markdown forms are pre-processed into the equivalent internal tag form before rendering:

| Markdown        | Equivalent block tag           |
|-----------------|--------------------------------|
| `[desc](ref)`   | `<a href="ref">desc</a>`       |
| `**text**`      | `<b>text</b>`                  |
| `_text_`        | `<i>text</i>`                  |

Strict subset — `__bold__` and `*italics*` are **not** recognised; both pass through as literal text. The pre-processor runs in a fixed order: one pass over the whole input sets opaque regions aside (fenced code blocks, code spans, HTML comments, `<code-block>` bodies, and recognized tag declarations with their quoted attributes, whichever starts first; a comment `<!-- … -->` contributes no node, so Prose reads its own `` `a`<!-- -->`b` `` Markdown back as two code values) → paragraph split → links → bold → italics → block-tag parser. No later phase ever sees code contents or attribute values, so a new phase never needs its own code/attribute protection; a `<` that opens no recognized tag is a literal `<` and parsing continues after it. Link URLs are placeholdered before the bold/italics phases so a URL like `https://example.com/path_with_underscores` is never re-interpreted. A code span (`` `x_y` ``) is opaque and becomes `NodeKind::InlineCode`: terminal dim (or theme colors) without backticks, HTML `<code>`, Markdown a safe backtick fence. Its value follows CommonMark: backslashes literal (`` `a\_b` `` shows `a\_b`), line endings → spaces, one edge space stripped when both edges have one. A span never crosses a paragraph boundary. On a terminal that emits no styling, inline code keeps a backtick fence. `` [`desc`](ref) `` is a link with code text; `` `[desc](ref)` `` is literal code (darkmatter's `code_link()` emits the former).

#### Flanking rules

`_` and `**` open or close emphasis only where CommonMark's left-/right-flanking rules allow (an `_` that is both opens only after punctuation and closes only before it). Neither ever acts between two word characters, and a closer counts only at its opener's tag depth, so no stray `</i>` can leak:

| Input | Output | Why |
|-------|--------|-----|
| `OPENCODE_CONFIG_CONTENT` | `OPENCODE_CONFIG_CONTENT` | Every `_` is intra-word |
| `_foo_bar_` | `<i>foo_bar</i>` | Outer `_` flanked by start/end; inner `_` skipped |
| `foo**bar**baz` | `foo**bar**baz` | Both `**` runs intra-word |
| `**foo**bar**baz**` | `<b>foo**bar**baz</b>` | Outer `**` flanked; inner pairs intra-word |
| `(_text_)` | `(<i>text</i>)` | Punctuation neighbours form boundaries |
| `**_pr/a.md** **_pr/b.md**` | `<b>_pr/a.md</b> <b>_pr/b.md</b>` | No valid closer for either `_` |
| `<dim>=OPENCODE_CONFIG_CONTENT</dim>` | unchanged | Tag wrapper preserved; intra-word `_` not triggered inside body |

**Practical consequence for callers:** author text, identifiers, paths, and error messages spliced into a Prose format string go through `Prose::escape_text` (attribute values through `Prose::quoted_attr`). Do not hand-roll a partial escaper; one that skips `_`, `*`, or `[` lets `_draft_` become italics. **Never** escape text placed inside a code span: the span is opaque, so the escape's backslashes would show. Fence a dynamic value with `renderable::markdown::code_span(value)` instead of hand-written backticks; it widens the fence when the value holds a backtick.

#### Escape mechanism

Outside code spans, a backslash escapes the immediately following character. Escapable: `* _ [ ] ( ) < > { \`. `Prose::escape_text` applies it to every escapable character. Inside a code span backslashes are literal.

```
\_text\_  →  _text_   (literal underscores, no italics)
\*\*x\*\* →  **x**    (literal asterisks)
\\        →  \        (literal backslash)
```

### Prose Options

`Prose`: `new`, `content`, `with_line_breaks(LineBreaks)`, `with_tag(ProseTag)`,
`with_word_wrap`, `with_left_margin` / `with_right_margin`, and the
`TerminalRenderable` layout helpers (`with_layout`, `alignment`).
`InlineProse`: `new`, `content`, `with_line_breaks`, `to_render_nodes`; it
stores a `Layout` only because the trait requires one, and logs a warning if
it is non-default. Both have `escape_text` and `quoted_attr`.

## Manual Styling

For direct escape code output:

### Colors

```rust
// Basic colors (30-37 fg, 40-47 bg)
println!("\x1b[31mRed\x1b[0m");
println!("\x1b[44mBlue background\x1b[0m");

// 256-color palette
println!("\x1b[38;5;208mOrange\x1b[0m");

// RGB (true color)
println!("\x1b[38;2;255;100;50mCustom\x1b[0m");
```

### Text Attributes

```rust
println!("\x1b[1mBold\x1b[0m");
println!("\x1b[3mItalic\x1b[0m");
println!("\x1b[4mUnderline\x1b[0m");
println!("\x1b[1;3;4mBold italic underline\x1b[0m");
```

### Combined Example

```rust
fn styled_error(msg: &str, term: &Terminal) -> String {
    if !term.is_tty {
        return format!("ERROR: {}", msg);
    }

    let underline = if term.underline_support.curly {
        "\x1b[4:3m"  // Curly
    } else {
        "\x1b[4m"    // Straight
    };

    format!("\x1b[1;31m{}ERROR:\x1b[0m {}", underline, msg)
}
```

## Hyperlinks

### OSC8 Links

```rust
fn hyperlink(text: &str, url: &str, term: &Terminal) -> String {
    if term.osc_link_support {
        format!("\x1b]8;;{}\x07{}\x1b]8;;\x07", url, text)
    } else {
        format!("{} ({})", text, url)
    }
}
```

### With ID Parameter

```rust
// For grouping related links
format!("\x1b]8;id=mylink;{}\x07{}\x1b]8;;\x07", url, text)
```

## Best Practices

### Always Check Capabilities

```rust
fn styled_output(term: &Terminal) {
    // Check TTY first
    if !term.is_tty {
        println!("Plain output");
        return;
    }

    // Check specific features
    if term.supports_italic {
        println!("\x1b[3mItalic\x1b[0m");
    }

    // Check color depth
    match term.color_depth {
        ColorDepth::TrueColor => {
            println!("\x1b[38;2;255;100;0mRGB\x1b[0m");
        }
        ColorDepth::Enhanced => {
            println!("\x1b[38;5;208m256-color\x1b[0m");
        }
        _ => {
            println!("\x1b[33mBasic yellow\x1b[0m");
        }
    }
}
```

### Reset After Styling

```rust
// Always reset to prevent style bleeding
println!("\x1b[1mBold\x1b[0m normal");

// Or use specific resets
println!("\x1b[1mBold\x1b[22m normal weight");
```

### Respect NO_COLOR

```rust
fn colored_output(msg: &str) {
    if std::env::var("NO_COLOR").is_ok() {
        println!("{}", msg);
    } else {
        println!("\x1b[32m{}\x1b[0m", msg);
    }
}
```

For components, do not strip SGR from a styled render: render through a
colorless terminal (`Terminal { color_depth: ColorDepth::None, ..term }`) so
the renderer picks the unstyled forms itself. A stripped styled render leaves
inline code unmarked, where a colorless render keeps its backtick fence. The
`bt` CLI does this with `colorless_when_no_color`.

## Related

- [Color System](./color-system.md) - Full color type hierarchy (BasicColor, RgbColor, WebColor, Tailwind)
- [Escape Codes](./escape-codes.md) - Stripping and analyzing escape codes
- [Terminal Struct](./terminal-struct.md) - Capability detection
