# Markdown Rendering

## Architecture

Markdown rendering uses a three-stage pipeline:

1. **Parse**: `pulldown-cmark` (with strikethrough) into `RichNode` AST
2. **Transform**: Provider-agnostic intermediate representation
3. **Render**: Per-provider output (Discord Markdown, Slack mrkdwn, Telegram HTML, plain text)

The Markdown is parsed once per message. Each provider renderer walks the same AST.

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

**Discord**: Mostly pass-through Markdown. Discord natively supports the same syntax.

**Slack**: Uses mrkdwn dialect. Bold is `*text*` (not `**`), links use `<url|label>` pipe syntax. Headings render as bold text since Slack has no heading syntax.

**Telegram**: Renders to HTML for the Bot API `parse_mode: "HTML"`. All formatting uses HTML tags. Code blocks use `<pre>` with optional `<code class="language-X">`.

**Signal / WhatsApp**: All formatting stripped to plain text. A `CompatibilityWarning` is emitted when Markdown messages target these providers.

## Location Rendering

Providers handle location differently:

- **Native location** (Telegram, WhatsApp): Sent as a separate API call. If message has both text and location, only the location is sent.
- **Text fallback** (Discord, Slack, Signal): Location is appended as a formatted text line to the rendered body.
