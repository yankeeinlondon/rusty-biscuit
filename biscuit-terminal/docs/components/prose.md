---
blast_radius: biscuit-terminal/lib/src/components/prose.rs
---
# Prose

Styled text component with token and block tag support for rich terminal output. Prose is the primary text styling component in biscuit-terminal, supporting three input grammars that all resolve to ANSI escape codes:

1. **Atomic tokens** — `{{bold}}…{{reset}}` (require manual `{{reset}}`).
2. **Block tags** — `<bold>text</bold>` (auto-reset on close, nestable).
3. **Markdown subset** — `[desc](url)`, `**bold**`, `_italics_` (per the [Prose+ spec](../../features/2026-05-05-prose-plus/spec.md)).

The three grammars compose freely; they are processed in a fixed order (links → bold → italics → block tags / atomic tokens) so URL contents, bold runs, and tag attribute values are protected from later phases.

## Programmatic Use

```rust
use biscuit_terminal::prelude::*;

// Atomic tokens (require manual {{reset}})
let prose = Prose::new("{{bold}}Important:{{reset}} normal text");

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

Three Markdown forms are recognized in addition to the tag and token grammars. They are pre-processed into the equivalent block-tag form before rendering.

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

### Supported Tags/Tokens

**Text Styling**: `bold`, `dim`, `italic`, `underline`, `strikethrough`, `blink`

**Colors** (foreground): `red`, `green`, `blue`, `yellow`, `cyan`, `magenta`, `white`, `black`, plus bright variants (`bright-red`, etc.), Tailwind colors (`gray-800`, `blue-400`), and web colors (`coral`, `salmon`)

**Background Colors**: Prefix with `bg-` (e.g., `<bg-blue>`, `<bg-coral>`)

**Special**: `<a href="url">text</a>` for hyperlinks, `<rgb #hex>text</rgb>` for arbitrary colors, `{{reset}}` to clear all styles

### Prose in Other Components

`Todo` and `Status` both offer a `from_prose` constructor that renders the description through Prose at render time, so markup is resolved with full terminal context:

```rust
use biscuit_terminal::prelude::*;

let todo = Todo::from_prose("review <red>critical</red> PR");
let status = Status::from_prose("this is a <b>test</b>")
    .state(StatusState::Success);
```

### Key API

| Method | Description |
|--------|-------------|
| `Prose::new(text)` | Create with styled content |
| `.content()` | Get the raw content string |
| `.with_word_wrap(WordWrap)` | Set word wrap strategy |
| `.with_left_margin(Margin)` | Set left margin |
| `.with_right_margin(Margin)` | Set right margin |
| `.with_layout(Layout)` | Set full layout configuration |

## Graceful Degradation

`Prose` consults the active `Terminal`'s capability profile when it renders and
silently downgrades any markup that the terminal cannot display. The goal is
that low-capability emulators (Apple Terminal being the canonical example) see
clean, readable text instead of literal escape-code garbage.

Two markup categories degrade today: **OSC8 hyperlinks** and the
**double-underline** style (both as a `<double-underline>` block tag and as the
`{{double-underline}}` atomic token).

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
block tag and `{{double-underline}}` atomic token degrade according to the
straight-underline capability:

| `double` | `straight` | Behavior |
|----------|------------|----------|
| `true`   | `true`     | Emit `\x1b[4:2m … \x1b[0m` (double underline) |
| `false`  | `true`     | Emit `\x1b[4m … \x1b[0m` (straight underline) |
| `false`  | `false`    | Emit plain text — no underline SGR codes |

Both the block tag form (`<double-underline>important</double-underline>`) and
the atomic token form (`{{double-underline}}important{{reset}}`) share the same
policy; they route through the same capability check inside `Prose`.

The `\x1b[4:2m` sequence is **never** emitted when the active terminal does not
report double-underline support — verified by Level-1 PTY tests and Level-2
Apple Terminal harness tests.

## CLI

Exposed via `bt prose`:

```bash
bt prose "<bold>Hello</bold> <red>world</red>"
```
