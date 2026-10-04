---
description: Markdown rendering trait in the renderable library — MarkdownRenderable and style-aware Markdown output.
---

# Markdown Module

The `MarkdownRenderable` trait for components that render to Markdown output.

## MarkdownRenderable

```rust
pub trait MarkdownRenderable {
    /// Renders the component as a Markdown string.
    fn render_markdown(&self) -> String;

    /// Renders with an optional Stylesheet for style-aware output.
    /// Default ignores the stylesheet and delegates to render_markdown.
    fn render_markdown_with_style(&self, _style: Option<Stylesheet>) -> String {
        self.render_markdown()
    }
}
```

## Plain Markdown

Components that lower cleanly to ergonomic Markdown implement `render_markdown`:

```rust
impl MarkdownRenderable for MyComponent {
    fn render_markdown(&self) -> String {
        format!("## {}\n\n{}", self.title, self.body)
    }
}
```

## Style-Aware Markdown

Components that need richer styling can consume a `Stylesheet` and project Markdown-addressable rules into the output:

```rust
impl MarkdownRenderable for StyledComponent {
    fn render_markdown(&self) -> String {
        "# Hello\n\nBasic content".to_string()
    }

    fn render_markdown_with_style(&self, style: Option<Stylesheet>) -> String {
        let mut output = self.render_markdown();
        
        if let Some(sheet) = style {
            // Extract Markdown-addressable rules and inject
            // as inline HTML style attributes or classes
            output.push_str(&format!("\n\n<style>{}</style>", sheet.to_css()));
        }
        
        output
    }
}
```

## Markdown vs MarkdownPlus

- **Markdown** — standard CommonMark/GFM, most ergonomic for authors
- **MarkdownPlus** — same trait, but output includes more inline HTML for richer features

A component can detect which target is requested and adjust output accordingly:

```rust
impl MarkdownRenderable for MyComponent {
    fn render_markdown(&self) -> String {
        self.render_to_markdown(false)
    }

    fn render_markdown_with_style(&self, style: Option<Stylesheet>) -> String {
        let mut md = self.render_to_markdown(true); // MarkdownPlus mode
        
        if let Some(sheet) = style {
            // Inject stylesheet as inline HTML
        }
        
        md
    }
}
```

## Tree Renderer Spellings

`render_markdown_node` (both dialects) makes these spelling choices, which other
producers must match:

- **Breaks by position** (`write_break` in the writer) → a soft break is a
  newline and a hard break `\` + newline (never two trailing spaces) only
  where content precedes it on its line and follows it in its block. With
  nothing after it in the block: soft → `&#32;`, hard → `<br>` (a reader
  drops a final newline and reads a final `\` literally). On a blank line
  (block or link/span body start, after another break): soft → a space, so
  no blank line ends the paragraph and no MarkdownPlus span opener sits alone
  on a line (a raw HTML block). Table cells, heading text, and the
  MarkdownPlus summary and columns (raw HTML): soft → space, hard → `<br>`;
  progress label: both → space. A block holding only one hard break is `<br>` (read
  as a raw HTML line break). A break child of a block container joins its
  phrasing neighbors into one inline sequence. Moved delimiter-edge breaks
  are spelled for their new position.
- **Literal text** → a `Text` value reads back as exactly its characters;
  formatting comes only from structural nodes. Escaped only where a reader
  could act: `` ` ``/`[`/`]` always; `*`/`_`/`~` where flanking lets them
  open or close (`snake_case`, `2 * 3` unchanged); `<` before a tag, URI
  autolink, or anything that could still complete an email autolink
  (`<3@example.com>` → `\<3@…`; `I <3 you` unchanged); `&` that could form a
  reference (`&copy;` → `\&copy;`); `==` and an edge `=` (darkmatter mark); a
  backslash before punctuation, a line ending, or the value's end (`C:\dir`
  unchanged); a lone CR as `&#13;`. A raw `Html` payload's CR is a line
  ending to `LineWriter` (text after it is protected as a line start; an LF
  in the next text value completes that CRLF). Block syntax at a line start (`\# x`,
  `1\. x`, `\> x`, a `--|--` table delimiter row) is escaped on the **whole
  logical line** the inline pieces assemble (`LineWriter` in the writer), so
  `Text("1")` + `Text(". x")` or a flattened span cannot form a list. Edge
  whitespace of a line, a heading, or a table cell is a reference (`&#32;`,
  `&#9;`): readers strip it, read 4 columns as code, and read two trailing
  spaces as a hard break. A heading's line feed is `&#10;` (CRLF →
  `&#13;&#10;`); its trailing
  ` #` run is escaped. Table cells add `\|`/`<br>`; MarkdownPlus span bodies HTML-escape
  `<>&` and keep the Markdown escapes. Image alt and table title use the
  text rules; link/image titles and destinations escape a reference-forming
  `&` (`https://x.io/&copy;` stays exact), and titles write line endings as
  references (below). Code and raw `Html` are never
  escaped (except a raw line ending in a heading or cell, and cell pipes,
  below).
  Hand-written Markdown uses `renderable::markdown::escape_text`.
- **Pipes in a table cell** → GFM splits the row before inline parsing
  and removes one `\` before each `|`, so every field in a cell protects
  its pipes exactly once, last: authored payloads (text, code span,
  link/image fields, raw `Html`, footnote identifier, the `Warn` unsupported
  placeholder) get `escape_cell_pipes` (`|` → `\|`; a payload's own `\|`
  becomes `\\|` and reads back as `\|`); generated attribute values and
  generated HTML (MarkdownPlus span `class`, progress widget) get
  `encode_generated_attribute(…, in_cell)` (`&#124;`). pulldown-cmark and
  markdown-rs keep the `\` inside an inline HTML token (cmark-gfm removes
  it), which is why generated attributes use the reference. Outside a
  table nothing changes. A span `class` is attribute-escaped everywhere.
- **Line endings in quoted fields** → link/image titles
  (`escape_link_title`) and generated attribute values
  (`encode_generated_attribute`: span `class`, progress `aria-label` and the
  four glyph `data-*` attributes) write CR/LF as `&#13;`/`&#10;` in every
  context (`encode_line_endings`), since a literal ending ends a cell row or
  heading and a blank line ends the paragraph; both readers decode the
  reference, so the value is exact and no diagnostic is due. A title never
  takes the cell `<br>` spelling. Image alt in a heading takes the heading
  text's `&#10;` (`encode_heading_line_feeds`). Raw HTML, destinations,
  labels, text, and code are untouched. Tests:
  `renderable/tests/markdown_line_endings_in_fields.rs`.
- **Blocks in a table cell or heading** (`render_one_line_block`,
  `record_one_line_block`) → a GFM cell is one inline line (so is a
  heading, which can get a block only through an `Extended`). A cell of inline content or exactly one
  `Paragraph` is clean. Any other block, at any depth (incl. inside an
  `Extended`), is lossy once per cell: `Strict` → `LossyRejected`; `Warn`
  → one `Diagnostic::lossy`; `Lossy` silent. Spelling (both dialects): each
  block's inline text set off by `<br>`; `Code` → `code_span` (line endings
  → spaces, no lang); list items get text markers (`- `, `3. `, `[x] `);
  `ThematicBreak` → empty line; nested table → one line per cell; columns
  and disclosure are flattened, never HTML (`lower_to_html` never runs in a
  cell). Validation still allows blocks in a cell (browser/terminal show
  them). Tests: `tests/markdown_table_cell_blocks.rs`.
- **Footnote labels** (`footnote_label`, `footnote_label_spelling`, reference
  *and* definition) → a reader matches labels without processing escapes
  and normalizes whitespace, so an identifier reads back exactly only with
  no unescaped `[`/`]`, no odd trailing `\` run, no line ending/tab/space
  run/edge space, and some non-whitespace. Otherwise `Strict` →
  `LossyRejected`; `Warn`/`Lossy` write the same degraded label in both
  places (`a]b` → `a\]b`, `a\` → `a\\`, `a\nb` → `a b`, empty → `_`), `Warn`
  adds a lossy diagnostic per site. Tests: `tests/markdown_footnote_labels.rs`.
- **`Warn` unsupported placeholder** (`comment_safe_label`) → the label in
  `<!-- unsupported: … -->` writes `>` as `&gt;` (no `-->`/`--!>` can close
  it) and line endings as spaces. Tests:
  `tests/markdown_unsupported_placeholder.rs`.
- **Raw `Html` in a heading or table cell** (`render_html`, any nesting:
  direct, or inside `Strong`/`Link`/span) → both are one line, so a raw
  LF/CR/CRLF would end the heading or row. Lossy in both dialects: `Strict`
  rejects (`LossyRejected`); `Warn` writes each CR as `&#13;` and LF as
  `&#10;` and records `Diagnostic::lossy` with the node's span; `Lossy`
  same output, silent. A cell uses the reference, not its text `<br>`
  (HTML decodes the reference to the payload's character in text and
  quoted attributes). Each CR/LF is encoded alone, so a CRLF split across
  two payloads matches one payload (one diagnostic per payload). A payload
  with no line ending is byte-identical. Tests:
  `tests/markdown_single_line_raw_html.rs`.
- **HTML-backed MarkdownPlus content** (`lower_to_html` in the writer,
  `browser::lower_to_html`) → the disclosure `<summary>` and the whole
  columns container are raw HTML blocks a reader does not parse as
  Markdown, so they are written by the browser writer: code is `<code>` with
  its value escaped (`<em>x</em>`, `&copy;` stay literal), emphasis/links/
  images/footnote refs are elements, `mark` → `<mark>`, `dim` → opacity span,
  text is HTML-escaped. Only generated HTML is re-encoded to keep the block
  open (a line ending beside a blank line → `&#10;`, CR → `&#13;`); raw
  `Html` payloads are byte-identical (the browser fragment marks their
  ranges, `RawRange`). Line endings are read on the **concatenated** HTML
  (a CR and the LF after it are one ending even across adjacent raw nodes or
  a raw/generated boundary); ownership only decides which bytes may change.
  A generated CR is always `&#13;` (never part of an ending); an ending is
  encoded only when every byte of it is generated. A blank line with no
  wholly generated ending beside it is lossy:
  `Strict` rejects, `Warn` encodes its line ending and records a diagnostic.
  Never encode the concatenated HTML as a whole: script/style/comments do
  not decode references and whitespace separates unquoted attributes.
  Mermaid stays a code block. The progress label is `browser::plain_text` of the children, as in
  the browser. Disclosure bodies, paragraphs, and styled spans stay
  Markdown. Never reparse generated Markdown or escape per component.
- **Edge whitespace and breaks of a delimiter wrapper** (`Emphasis`,
  `Strong`, `Delete`, `mark`, `dim`, nested ones included) → written outside
  the delimiters so the run flanks its text (`a<b> b</b>` → `a **b**`); a
  wrapper of only whitespace/breaks gets no delimiters.
- **Delimiter spelling** → chosen from the characters around each run so it
  can open and close (CommonMark flanking; `_` never intraword; no run beside
  the same unescaped character): `Emphasis` `_` → `*` → `<em>`, `Strong`
  `**` → `__` → `<strong>`, `Delete` `~~` → `<del>`, `mark` `==` / `dim` `⌄`
  → plain text with a lossy diagnostic (darkmatter's reader rules, no HTML
  form). `a<i>b</i>c` → `a*b*c`; `a<b>(b)</b>c` → `a<strong>(b)</strong>c`.
  The HTML fallback applies in both dialects (like a cell's `<br>`); output
  that already flanked is unchanged.
- **Code block** → a backtick fence of three, or one longer than the longest
  backtick run that starts a body line (a line starts after LF, CR, or
  CRLF), so a body holding a ```` ``` ```` line reads back whole.
- **Block quote / list item / footnote prefixes** (`prefix_lines`,
  `indent_continuation` via `split_line_endings`) → lines end at LF, CR, or
  CRLF, as a reader reads them; every line gets `> ` (bare `>` when empty),
  the item's indent, or a footnote definition's four spaces
  (`FOOTNOTE_CONTINUATION`; GFM's continuation rule, independent of label
  width), and keeps its own ending bytes, so a raw `Html` CR or CRLF is
  byte-identical and the next line stays in the container. Never split with
  `str::lines()` here: it drops a CRLF's CR and misses a lone CR. A list
  item or footnote body drops only a final LF (the join supplies one).
  Tests: `tests/markdown_footnote_continuation.rs`. pulldown-cmark
  0.13 does not end a line at a lone CR inside HTML/code blocks, so tests
  normalize endings to LF before reading container/fence boundaries.
- **Inline code** → `renderable::markdown::code_span(value)`: a backtick fence
  one longer than the longest run in the value, one space of padding when the
  value starts/ends with a backtick or starts *and* ends with a space (and is
  not all spaces). Line endings become spaces; an empty value is `""` with no
  fence. Inside a table cell `|` is escaped to `\|` *before* fencing. Use
  `code_span` anywhere a code span is spelled (the terminal backtick fallback,
  darkmatter `code_link()`) instead of hand-wrapping the value in single backticks.
  `code_span` is per value: the writer's inline joiner (`join_pieces`) writes
  `<!-- -->` (`CODE_SPAN_SEPARATOR`) between two code spans that would
  touch, because touching fences merge into one backtick run (`` `a``b` ``
  is one value). `InlinePiece::Code` and the `FenceEdges` returned with a
  nested sequence (`Joined`) carry the check across empty nodes, flattened
  wrappers, unstyled `mark`/`dim` bodies, and one-line blocks; an empty
  `InlineCode` counts as blank. A hand-built serializer that concatenates
  two `code_span` results needs the same boundary.
  Every repo reader treats the comment as structure: `is_html_comment_only`
  (in `renderable::tree`) marks a raw HTML value that is only comments; the
  terminal renderer writes nothing for it in every strictness, the browser
  renderer writes nothing under `Escape`/`Reject` (verbatim under `Allow`),
  plain Markdown does not report it as unportable, and Prose's grammar lifts
  a comment as `Lifted::Comment` (no node). Do not delete comment-shaped text
  anywhere else; code, text, and attribute values keep it.
  Tests: `tests/markdown_adjacent_code_spans.rs` (fixtures shared through
  `tests/support/adjacent_code_matrix.rs`, also read by darkmatter's
  `tests/l1/serialized_code_neighbors.rs`), `tests/html_policy_comment_nodes.rs`.
