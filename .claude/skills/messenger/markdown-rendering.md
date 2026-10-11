# Markdown Rendering

## Architecture

Markdown rendering uses a three-stage pipeline:

1. **Parse**: `pulldown-cmark` (with strikethrough) into `RichNode` AST
2. **Transform**: Provider-agnostic intermediate representation
3. **Render**: Per-provider output (Discord Markdown, Slack mrkdwn, Telegram HTML, plain text)

The Markdown is parsed once per message. Each provider renderer walks the same AST.
Discord bot sends and Discord webhook sends share the same Discord renderer; the webhook adapter changes transport and capabilities, not markup syntax.

## Module Structure

```
markdown/
  mod.rs            # Rendering dispatcher (picks renderer by ProviderKind)
  ast.rs            # RichNode enum definition
  parse.rs          # pulldown-cmark parser -> Vec<RichNode>
  discord.rs        # Discord-flavored Markdown output
  slack_mrkdwn.rs   # Slack mrkdwn output
  telegram_html.rs  # Telegram Bot API HTML output
  plain_text.rs     # Stripped plain text (Signal, WhatsApp)
```

## RichNode AST

```rust
pub enum RichNode {
    Text(String),
    Bold(Vec<RichNode>),
    Italic(Vec<RichNode>),
    Strikethrough(Vec<RichNode>),
    Code(String),
    CodeBlock { language: Option<String>, code: String },
    Link { url: String, children: Vec<RichNode> },
    List { ordered: bool, items: Vec<Vec<RichNode>> },
    Paragraph(Vec<RichNode>),
    Heading { level: u8, children: Vec<RichNode> },
    SoftBreak,
    HardBreak,
}
```

Unsupported Markdown constructs (images, tables, block quotes) are flattened to their text children.

## Renderer Behavior

| Construct | Discord | Slack | Telegram | Plain text |
|-----------|---------|-------|----------|------------|
| Bold | `**text**` | `*text*` | `<b>text</b>` | `text` |
| Italic | `*text*` | `_text_` | `<i>text</i>` | `text` |
| Strikethrough | `~~text~~` | `~text~` | `<s>text</s>` | `text` |
| Inline code | `` `code` `` | `` `code` `` | `<code>code</code>` | `code` |
| Code block | ` ```lang ` | ` ```lang ` | `<pre>code</pre>` | `code` |
| Link | `[text](url)` | `<url\|text>` | `<a href="url">text</a>` | `text (url)` |
| Heading | `## text` | `*text*` | `<b>text</b>` | `TEXT` |

## Provider-Specific Notes

**Discord / Discord-Webhook**: Mostly pass-through Markdown. Both adapters use the same renderer because Discord webhooks accept the same formatting syntax as bot-authenticated sends. Parsed text is literal (the parser removed the source's escapes), so the renderer backslash-escapes `\`, `*`, `~`, `` ` ``, `|`, `[`, `]`, `<`, a `_` outside a word, and a heading/quote/list marker that starts a line: source `\*\*literal\*\*` stays literal instead of turning bold. Inline code containing a backtick uses a double-backtick span.

**Slack**: Uses mrkdwn dialect. Bold is `*text*` (not `**`), links use `<url|label>` pipe syntax. Headings render as bold text since Slack has no heading syntax. Slack reads `&`, `<`, and `>` as markup everywhere (`<!channel>`, `<url|label>`), so text and code escape them as `&amp;`, `&lt;`, `&gt;`; mrkdwn has no escape for `*`, `_`, `~`, or `` ` ``, so a literal formatting spelling in text still formats in Slack. Slack formats a delimiter only at a word boundary, so a span whose outer neighbor is a letter, digit, or the same delimiter (`a**b**c`, bold inside a heading) is written as plain text (`abc`); mrkdwn has no other spelling.

**Delimiter edges (Discord, Slack)**: edge whitespace and breaks of a span go outside its delimiters (`split_edges` in `ast.rs`). Discord's `*`, `**`, and `~~` have no word-boundary or punctuation rule, so intraword and punctuation-adjacent spans keep their delimiters there.

**Telegram**: Renders to HTML for the Bot API `parse_mode: "HTML"`. All formatting uses HTML tags. Code blocks use `<pre>` with optional `<code class="language-X">`.

**Signal / WhatsApp**: All formatting stripped to plain text. A `CompatibilityWarning` is emitted when Markdown messages target these providers.

## Location Rendering

Providers handle location differently:

- **Native location** (Telegram, WhatsApp): Sent as a separate API call. If message has both text and location, only the location is sent.
- **Text fallback** (Discord, Slack, Signal): Location is appended as a formatted text line to the rendered body.
