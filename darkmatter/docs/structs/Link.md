# Link Struct

A multi-format hyperlink container that supports rendering to terminal (OSC 8 escape sequences), HTML anchor elements, and Markdown links.

## Overview

The `Link` struct pairs display text with a destination URL or file path, storing optional HTML attributes (class, style, target, title, prompt, data-*) and providing methods to render to different output formats. It can parse both HTML and Markdown strings, and supports a structured Markdown format that preserves rich attributes during parsing.

```rust
use darkmatter::render::Link;

let link = Link::new("Click here", "https://example.com");
```

## Core Fields

| Field | Type | Description |
|-------|------|-------------|
| `display` | `String` | The visible text shown to users |
| `href` | `String` | The URL or file path destination |
| `kind` | `LinkType` | Auto-detected as `Url` (http/https) or `File` |
| `class` | `Option<String>` | CSS class for HTML output |
| `style` | `Option<String>` | Inline CSS for HTML output |
| `target` | `Option<String>` | Target attribute (e.g., "_blank") |
| `title` | `Option<String>` | Tooltip text (HTML and Markdown) |
| `prompt` | `Option<String>` | Popover hint text (HTML with Popover API) |
| `data` | `Option<HashMap<String, String>>` | Custom data-* attributes |

## Creating Links

### Direct Construction

Use `Link::new()` for basic links. The `kind` field is automatically determined from the URL scheme:

```rust
use darkmatter::render::{Link, LinkType};

let url_link = Link::new("Website", "https://example.com");
assert!(url_link.is_url());
assert_eq!(url_link.kind(), LinkType::Url);

let file_link = Link::new("README", "/path/to/file.md");
assert!(file_link.is_file());
assert_eq!(file_link.kind(), LinkType::File);
```

For links that need structured attributes parsed from a title string, use `Link::with_title_parsed()`:

```rust
let link = Link::with_title_parsed(
    "Submit",
    "https://example.com/form",
    r#"class="btn btn-primary" prompt="Submit the form""#
);

assert_eq!(link.class(), Some("btn btn-primary"));
assert_eq!(link.prompt(), Some("Submit the form"));
```

### Builder Pattern

Chain builder methods to add optional attributes:

```rust
let link = Link::new("Open", "https://example.com")
    .with_class("external-link")
    .with_style("color: blue; text-decoration: underline;")
    .with_target("_blank")
    .with_title("Opens in a new tab")
    .with_prompt("Click to visit example.com")
    .with_data("analytics-id", "link-123")
    .with_data("category", "navigation");

assert_eq!(link.class(), Some("external-link"));
assert_eq!(link.target(), Some("_blank"));
assert_eq!(link.data().unwrap().get("analytics-id"), Some(&"link-123".to_string()));
```

### From Tuple

Convert a tuple of `(display, url)` into a `Link`:

```rust
let link: Link = ("Documentation", "https://docs.rs").into();

assert_eq!(link.display(), "Documentation");
assert_eq!(link.href(), "https://docs.rs");
```

### Parsing Strings

The `Link` struct implements `TryFrom<String>` and `TryFrom<&str>` to parse existing link strings.

#### Parsing HTML

Parse complete anchor elements including all attributes:

```rust
use darkmatter::render::Link;

let html = r#"<a href="https://example.com" class="btn" target="_blank" title="Go to example" data-id="123">Click me</a>"#;
let link = Link::try_from(html).unwrap();

assert_eq!(link.display(), "Click me");
assert_eq!(link.href(), "https://example.com");
assert_eq!(link.class(), Some("btn"));
assert_eq!(link.target(), Some("_blank"));
assert_eq!(link.title(), Some("Go to example"));
assert_eq!(link.data().unwrap().get("id"), Some(&"123".to_string()));
```

HTML entities are automatically unescaped during parsing:

```rust
let html = r#"<a href="https://example.com?a=1&amp;b=2">&lt;Script&gt;</a>"#;
let link = Link::try_from(html).unwrap();

assert_eq!(link.display(), "<Script>");
assert_eq!(link.href(), "https://example.com?a=1&b=2");
```

#### Parsing Markdown

**Basic format:**

```rust
let link = Link::try_from("[Example](https://example.com)").unwrap();
assert_eq!(link.display(), "Example");
assert_eq!(link.href(), "https://example.com");
```

**With title (Title Mode):**

```rust
let link = Link::try_from(r#"[Example](https://example.com "Visit example.com")"#).unwrap();
assert_eq!(link.title(), Some("Visit example.com"));
```

**With structured attributes (Structured Mode):**

When the parentheses contain `key=value` patterns, the parser enters Structured Mode:

```rust
let link = Link::try_from(
    r#"[Submit](https://example.com/form class="btn primary" prompt="Click to submit" data-action="submit")"#
).unwrap();

assert_eq!(link.class(), Some("btn primary"));
assert_eq!(link.prompt(), Some("Click to submit"));
assert_eq!(link.data().unwrap().get("action"), Some(&"submit".to_string()));
```

Structured mode supports comma or space delimiters and quoted or unquoted values:

```rust
// All equivalent:
let a = Link::try_from(r#"[X](url class="btn")"#).unwrap();
let b = Link::try_from("[X](url class=btn)").unwrap();
let c = Link::try_from("[X](url class=btn,target=_blank)").unwrap();
```

#### Error Handling

Parse failures return `LinkParseError`:

```rust
use darkmatter::render::{Link, LinkParseError};

// Unrecognized format
let err = Link::try_from("plain text").unwrap_err();
assert!(matches!(err, LinkParseError::UnrecognizedFormat));

// Missing URL
let err = Link::try_from("[Click]()").unwrap_err();
assert!(matches!(err, LinkParseError::MissingUrl));

// Malformed HTML
let err = Link::try_from("<div>not a link</div>").unwrap_err();
assert!(matches!(err, LinkParseError::MalformedHtml(_)));
```

## Output Formats

### Terminal (OSC 8 Hyperlinks)

Render clickable terminal links using [OSC 8 escape sequences](https://gist.github.com/egmontkob/eb114294efbcd5adb1944c9f3cb5feda):

```rust
let link = Link::new("GitHub", "https://github.com");

// Auto-detects terminal support
let terminal = link.to_terminal();
// With support: \x1b]8;;https://github.com\x07GitHub\x1b]8;;\x07
// Without support: GitHub [https://github.com]

// Force output regardless of detection
let forced = link.to_terminal_unchecked();
// Always: \x1b]8;;https://github.com\x07GitHub\x1b]8;;\x07
```

#### Display Trait

The `Display` trait implementation uses terminal format:

```rust
let link = Link::new("Home", "https://example.com");
println!("{}", link); // Outputs terminal-formatted link
```

### HTML

Render as an anchor element with all attributes:

```rust
let link = Link::new("Submit", "https://example.com/form")
    .with_class("btn btn-primary")
    .with_style("font-weight: bold;")
    .with_target("_blank")
    .with_title("Submit the form")
    .with_prompt("Click to submit");

let html = link.to_browser();
// <a href="https://example.com/form" class="btn btn-primary" style="font-weight: bold;" target="_blank" title="Submit the form" data-prompt="Click to submit">Submit</a>
```

All values are HTML-escaped to prevent XSS:

```rust
let link = Link::new("<script>alert('xss')</script>", "https://example.com?a=1&b=2");
let html = link.to_browser();
// <a href="https://example.com?a=1&amp;b=2">&lt;script&gt;alert('xss')&lt;/script&gt;</a>
```

#### Popover API

For modern browsers supporting the Popover API, render with a companion popover element:

```rust
let link = Link::new("Help", "https://docs.example.com")
    .with_prompt("Open documentation in a new tab")
    .with_target("_blank");

if let Some((anchor, popover)) = link.to_browser_with_popover() {
    // anchor: <a href="..." interestfor="popover-abc123" target="_blank">Help</a>
    // popover: <div id="popover-abc123" popover="hint">Open documentation in a new tab</div>
    println!("{}", anchor);
    println!("{}", popover);
}
```

### Markdown

Render as standard Markdown link format:

```rust
let link = Link::new("Example", "https://example.com");
assert_eq!(link.to_markdown(), "[Example](https://example.com)");

let link_with_title = Link::new("Example", "https://example.com")
    .with_title("Visit example.com");
assert_eq!(link_with_title.to_markdown(), r#"[Example](https://example.com "Visit example.com")"#);
```

Special characters are escaped:

```rust
let link = Link::new("[text]", "https://example.com/path(1)");
assert_eq!(link.to_markdown(), r"[\[text\]](https://example.com/path%281%29)");
```

## Helper Methods

### parsed_style()

Parse the inline style string into a HashMap of CSS property-value pairs:

```rust
let link = Link::new("Button", "#")
    .with_style("color: red; font-size: 14px; margin: 10px 20px;");

let styles = link.parsed_style().unwrap();
assert_eq!(styles.get("color"), Some(&"red".to_string()));
assert_eq!(styles.get("font-size"), Some(&"14px".to_string()));
assert_eq!(styles.get("margin"), Some(&"10px 20px".to_string()));
```

Property names are normalized to lowercase, and edge cases like extra whitespace and trailing semicolons are handled gracefully.

### Getters

Access all fields through getter methods:

```rust
let link = Link::new("Text", "https://example.com")
    .with_class("my-class");

assert_eq!(link.display(), "Text");
assert_eq!(link.href(), "https://example.com");
assert_eq!(link.kind(), LinkType::Url);
assert_eq!(link.class(), Some("my-class"));
assert_eq!(link.style(), None);
assert_eq!(link.target(), None);
assert_eq!(link.title(), None);
assert_eq!(link.prompt(), None);
assert_eq!(link.data(), None);
```

## Roundtrip Considerations

When converting between formats, some attributes may be lost:

| Attribute | HTML → Link | Markdown → Link | Link → HTML | Link → Markdown |
|-----------|-------------|-----------------|-------------|-----------------|
| `href` | ✅ | ✅ | ✅ | ✅ |
| `display` | ✅ | ✅ | ✅ | ✅ |
| `title` | ✅ | ✅ | ✅ | ✅ |
| `class` | ✅ | ✅ (structured) | ✅ | ❌ |
| `style` | ✅ | ✅ (structured) | ✅ | ❌ |
| `target` | ✅ | ✅ (structured) | ✅ | ❌ |
| `prompt` | ✅ | ✅ (structured) | ✅ | ❌ |
| `data-*` | ✅ | ✅ (structured) | ✅ | ❌ |

**Note:** The Markdown parser supports Structured Mode (e.g., `[text](url class="btn")`), but `to_markdown()` only outputs the standard format with title. This means:

- **HTML → Markdown → Link**: You lose class, style, target, prompt, and data-* attributes
- **Structured Markdown → Link → Markdown**: You lose those same attributes on output

## Complete Example

A realistic workflow showing the full lifecycle:

```rust
use darkmatter::render::{Link, LinkType};

// Create a rich link with all attributes
let original = Link::new("API Documentation", "https://docs.example.com/v2")
    .with_class("doc-link external")
    .with_style("color: #0066cc; font-weight: 500;")
    .with_target("_blank")
    .with_title("Open API reference (new tab)")
    .with_prompt("External link - opens in new tab")
    .with_data("section", "api")
    .with_data("version", "v2");

// Output for different contexts
let terminal_output = original.to_terminal();  // For CLI tools
let html_output = original.to_browser();       // For web apps
let markdown_output = original.to_markdown();  // For documentation

// Parse from different sources
let from_html = Link::try_from(r#"<a href="https://docs.example.com/v2" class="doc-link">API Docs</a>"#).unwrap();
let from_markdown = Link::try_from("[API Docs](https://docs.example.com/v2)").unwrap();

// Use parsed style for programmatic access
if let Some(styles) = original.parsed_style() {
    if let Some(color) = styles.get("color") {
        println!("Link color: {}", color);
    }
}

// Check link type for routing logic
match original.kind() {
    LinkType::Url => println!("Web URL - use HTTP client"),
    LinkType::File => println!("Local file - use filesystem"),
}

// Display trait for quick terminal output
println!("Link: {}", original);
```
