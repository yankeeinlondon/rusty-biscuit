---
blast_radius:
  - darkmatter/lib/src/render/link.rs
  - darkmatter/lib/src/render/mod.rs
  - darkmatter/lib/src/markdown/mod.rs
  - darkmatter/lib/src/markdown/output/html.rs
---

# `Link` Struct

The `Link` struct in `darkmatter` is a type-safe, multi-format hyperlink model that can:

1. Parse HTML or Markdown links into structured state
2. Render links for terminal (`OSC 8`), HTML, and Markdown
3. Preserve rich metadata in Markdown using a lossless policy by default

When links are embedded as raw HTML inside a markdown document, `Markdown::inline_html_links()` can extract them as `Link` values without changing the behavior of `Markdown::links()`.

## Overview

`Link` stores visible text, destination, and optional metadata commonly used by HTML anchors.

```rust
use darkmatter::render::Link;

let link = Link::new("Click here", "https://example.com").unwrap();
assert_eq!(link.href(), "https://example.com");
```

## Core State

| Field | Type | Notes |
|---|---|---|
| `display` | `String` | Visible text shown to users |
| `href` | `String` | Destination URL/path |
| `kind` | `LinkType` | Auto-detected: `Url` for `http/https`, otherwise `File` |
| `class` | `Option<String>` | HTML `class` attribute |
| `style` | `Option<Stylesheet>` | Typed inline CSS |
| `target` | `Option<LinkTarget>` | Typed anchor target |
| `title` | `Option<String>` | Optional title/tooltip text |
| `prompt` | `Option<String>` | Optional prompt text (`data-prompt`) |
| `data` | `BTreeMap<String, String>` | `data-*` attributes (stored without `data-` prefix) |

## Types

### `LinkType`

```rust
use darkmatter::render::LinkType;

let _ = LinkType::Url;
let _ = LinkType::File;
```

### `LinkTarget`

`LinkTarget` provides typed target handling:

- `LinkTarget::Self_`
- `LinkTarget::Blank`
- `LinkTarget::Parent`
- `LinkTarget::Top`
- `LinkTarget::Named(String)`

```rust
use darkmatter::render::{Link, LinkTarget};

let link = Link::new("Docs", "https://example.com")
    .unwrap()
    .with_target(LinkTarget::Blank);

assert_eq!(link.target_attr().as_deref(), Some("_blank"));
```

## Creating Links

### `new` (fallible)

`Link::new` validates that href is non-empty.

```rust
use darkmatter::render::Link;

let link = Link::new("README", "./README.md").unwrap();
assert!(link.is_file());

assert!(Link::new("x", "   ").is_err());
```

### `with_title_parsed`

`with_title_parsed` parses a Markdown title segment as either:

- plain title text, or
- structured metadata (`class=... style=... target=... prompt=... data-*`), or
- a lossless metadata payload (base64 JSON package used by `to_markdown(false)`)

```rust
use darkmatter::render::{Link, LinkTarget};

let link = Link::with_title_parsed(
    "Submit",
    "https://example.com/form",
    "class='btn' target='_blank' prompt='Continue'",
)
.unwrap();

assert_eq!(link.class(), Some("btn"));
assert_eq!(link.target(), Some(&LinkTarget::Blank));
assert_eq!(link.prompt(), Some("Continue"));
```

### Tuple conversions

`Link` implements `From` for:

- `(&str, &str)`
- `(&String, &String)`
- `(&str, &Prose)`
- `(&String, &Prose)`

These conversions are ergonomic, but they `expect(...)` internally and therefore require a valid, non-empty href.

## Builder API

```rust
use darkmatter::render::{Link, LinkTarget};

let link = Link::new("Open", "https://example.com")
    .unwrap()
    .with_class("external-link")
    .with_style_css("color: blue; text-decoration: underline;").unwrap()
    .with_target(LinkTarget::Blank)
    .with_title("Opens in a new tab")
    .with_prompt("Click to visit example.com")
    .with_data("analytics-id", "link-123")
    .with_data("category", "navigation");

assert_eq!(link.class(), Some("external-link"));
assert_eq!(link.target_attr().as_deref(), Some("_blank"));
assert_eq!(link.data().get("analytics-id"), Some(&"link-123".to_string()));
```

Notable setters:

- `with_display(impl Into<String>) -> Self`
- `with_href(...) -> Result<Self, LinkError>`
- `with_style(Stylesheet) -> Self`
- `with_style_css(...) -> Result<Self, LinkError>`
- `with_target(LinkTarget) -> Self`
- `with_target_str(...) -> Result<Self, LinkError>`
- `with_data_map(BTreeMap<String, String>) -> Self`

## Parsing (`TryFrom`)

`Link` supports parsing from `String`, `&String`, and `&str`.

### HTML input

```rust
use darkmatter::render::Link;

let html = r#"<a href="https://example.com" class="btn" target="_blank" title="Go" data-id="123">Click</a>"#;
let link = Link::try_from(html).unwrap();

assert_eq!(link.display(), "Click");
assert_eq!(link.href(), "https://example.com");
assert_eq!(link.class(), Some("btn"));
assert_eq!(link.target_attr().as_deref(), Some("_blank"));
assert_eq!(link.title(), Some("Go"));
assert_eq!(link.data().get("id"), Some(&"123".to_string()));
```

### Markdown input

```rust
use darkmatter::render::Link;

let basic = Link::try_from("[Example](https://example.com)").unwrap();
assert_eq!(basic.title(), None);

let title_mode = Link::try_from(r#"[Example](https://example.com "Visit example")"#).unwrap();
assert_eq!(title_mode.title(), Some("Visit example"));

let structured = Link::try_from(
    r#"[X](https://example.com class='btn' prompt='go' data-id=42)"#
).unwrap();
assert_eq!(structured.class(), Some("btn"));
assert_eq!(structured.prompt(), Some("go"));
assert_eq!(structured.data().get("id"), Some(&"42".to_string()));
```

## Errors

Parsing and construction use `LinkError` (alias: `LinkParseError`):

- `EmptyHref`
- `UnrecognizedFormat`
- `MalformedHtml(String)`
- `MalformedMarkdown(String)`
- `MissingHref`
- `InvalidStyle(StylesheetError)`
- `InvalidTarget { value }`

```rust
use darkmatter::render::{Link, LinkParseError};

let err = Link::try_from("plain text").unwrap_err();
assert!(matches!(err, LinkParseError::UnrecognizedFormat));

let err = Link::try_from("[Click]()").unwrap_err();
assert!(matches!(err, LinkParseError::MissingHref));
```

## Output Formats

### Terminal

```rust
use darkmatter::render::Link;

let link = Link::new("GitHub", "https://github.com").unwrap();

let auto = link.to_terminal();
let forced = link.to_terminal_unchecked();

assert!(forced.starts_with("\x1b]8;;https://github.com\x07"));
assert!(forced.ends_with("\x1b]8;;\x07"));
```

### HTML

Canonical renderer: `to_html()`

Compatibility alias: `to_browser()`

```rust
use darkmatter::render::{Link, LinkTarget};

let link = Link::new("Submit", "https://example.com/form")
    .unwrap()
    .with_class("btn")
    .with_style_css("font-weight: bold;").unwrap()
    .with_target(LinkTarget::Blank)
    .with_title("Submit the form");

let html = link.to_html();
assert!(html.contains("href=\"https://example.com/form\""));
assert!(html.contains("target=\"_blank\""));
```

`display` and `title` are sanitized for HTML output via `display_plain()` and `title_plain()`.

### HTML + Popover

Canonical renderer: `to_html_with_popover()`

Compatibility alias: `to_browser_with_popover()`

Returns `Option<String>` — the canonical accessible wrapper/anchor/prompt
markup the render-tree browser path emits, or `None` when the link carries no
prompt. See [Prompted Links (Popover)](../rendering/popover.md) for the full
markup, ARIA, and keyboard contract.

```rust
use darkmatter::render::Link;

let link = Link::new("Help", "https://docs.example.com")
    .unwrap()
    .with_prompt("Open documentation");

let markup = link.to_html_with_popover().unwrap();
assert!(markup.contains(r#"class="dm-popover-wrapper""#));
assert!(markup.contains("interestfor="));
assert!(markup.contains("aria-describedby="));
assert!(markup.contains(r#"popover="hint""#));

// A link without a prompt has no popover markup.
let plain = Link::new("Help", "https://docs.example.com").unwrap();
assert!(plain.to_html_with_popover().is_none());
```

### Markdown

`to_markdown(with_inline: bool)` uses policy-based handling when metadata beyond `(display, href, title)` exists.

- If `with_inline == true`: emits inline HTML.
- Else if `LINK_METADATA=inline`: emits inline HTML.
- Else if `LINK_METADATA=strip`: emits idiomatic Markdown and drops extended metadata.
- Else (default): emits idiomatic Markdown with a lossless metadata payload in the title field.

```rust
use darkmatter::render::{Link, LinkTarget};

let basic = Link::new("Example", "https://example.com").unwrap();
assert_eq!(basic.to_markdown(false), "[Example](https://example.com)");

let rich = Link::new("Example", "https://example.com")
    .unwrap()
    .with_class("chip")
    .with_target(LinkTarget::Blank);

let inline = rich.to_markdown(true);
assert!(inline.starts_with("<a "));
```

Compatibility alias: `to_markdown_legacy()` (equivalent to `to_markdown(false)`).

## Helper Accessors

- `display_plain()` strips ANSI from display text.
- `title_plain()` strips ANSI from title text.
- `style_css()` renders typed `Stylesheet` as inline CSS text.
- `target_attr()` returns string form of `LinkTarget`.
- `parsed_style()` parses style into `BTreeMap<String, String>` for inspection.

```rust
use darkmatter::render::Link;

let link = Link::new("Button", "#")
    .unwrap()
    .with_style_css("color: red; font-size: 14px;").unwrap();

let styles = link.parsed_style().unwrap();
assert_eq!(styles.get("color"), Some(&"red".to_string()));
```

## Roundtrip Behavior

By default, markdown output is designed to preserve metadata (lossless mode), unlike legacy lossy behavior.

```rust
use darkmatter::render::{Link, LinkTarget};

let original = Link::new("Docs", "https://example.com")
    .unwrap()
    .with_class("chip")
    .with_target(LinkTarget::Blank)
    .with_prompt("Open docs")
    .with_data("id", "abc");

let markdown = original.to_markdown(false);
let reparsed = Link::try_from(markdown.as_str()).unwrap();

assert_eq!(reparsed.class(), Some("chip"));
assert_eq!(reparsed.target(), Some(&LinkTarget::Blank));
assert_eq!(reparsed.prompt(), Some("Open docs"));
assert_eq!(reparsed.data().get("id"), Some(&"abc".to_string()));
```
