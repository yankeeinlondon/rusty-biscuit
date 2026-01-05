# Advanced Links

## Leveraging the Markdown Standard

In Markdown the overwhelming popular usage for _hyperlinks_ is: `[display text](resource)` where:

- the resource is either a relative link to something (but typically another Markdown page)

Most people do not know that the specification actually allows for a second parameter within the parenthesis:

> [display text](resource title)

The `title` was added to the original specification so that people could add some basic "popover" content since browsers have for a long time provided a rather crude popover effect for links if you added a `title` property.

### How to use the `title` Property

When we render Markdown which use the title property we will do it in one of two modes:

1. **Title Mode**
2. **Structured Mode**

The mode which will be used will be based on whether structured content is found in the `title` property or not. This ensures that if the author of the Markdown was intended to just use the standard then they will get that behavior but the extra capabilities of **Structured Mode** can be leveraged if the author choses to use them.

#### Title Mode

This mode resembles what the Markdown standard expects and Uses all content after the URI/resource link as the "title".

#### Structured Mode

This mode leverages the standard but extends it's capabilities by viewing the `title` text as a bunch of key/value pairs. The key's which have special meaning are:

| key       | terminal     | browser |
| ---       | --------     | ------- |
| `title`   | no effect    | adds the text to `title` property of link |
| `prompt`  | no effect    | adds the text to the `prompt` property of a link (which will trigger modern Popover) |
| `class`   | limited*    | adds the classes specified in the key's value to the HTML link |
| `style`   | limited*    | allows the user to add CSS properties to the HTML link |
| `data-*`       | no effect   | passed through as properties to the HTML link |


The syntax which is used here follows a `key=value` syntax and can be delimited by a comma or whitespace:

- `[my link](https://somewhere.com prompt="click me",class=buttercup style="background:red" )`

The above example is a valid syntax and:

- we can see that property values can be quoted but don't need to be (though "quoting generally considered the safer option")
- key/values can be delimited by a `,` or whitespace

Now that we have the basic concept down let's discuss the `Link` struct and then move into the details of each target platform.

## The Link struct

The `Link` struct, defined in the shared library (`shared/src/render/link.rs`), provides comprehensive support for creating and rendering links across different output targets.

### Core Features

- **Builder pattern** for constructing links with optional attributes
- **Parsing support** via `TryFrom<&str>` and `TryFrom<String>` for HTML and Markdown formats
- **Multi-target output**: terminal (OSC 8), browser (HTML), and markdown

### CSS Style Parsing

The `parsed_style()` method allows you to access CSS properties as a structured `HashMap`:

```rust
use shared::render::link::Link;

let link = Link::new("Click", "https://example.com")
    .with_style("color: red; font-size: 14px; background: blue");

if let Some(styles) = link.parsed_style() {
    println!("Color: {:?}", styles.get("color")); // Some("red")
    println!("Font size: {:?}", styles.get("font-size")); // Some("14px")
}
```

The parser handles:
- Whitespace normalization
- Trailing semicolons
- Empty declarations (e.g., `;;`)
- Case-insensitive property names (normalized to lowercase)
- Complex values (e.g., `url()`, multiple values)

## Output Targets

### Targeting the Terminal

Terminal hyperlinks use the OSC 8 escape sequence format. The `Link` struct supports this via two methods:

#### `to_terminal()`

Renders a clickable hyperlink if the terminal supports OSC 8, otherwise falls back to displaying the URL in brackets:

```rust
let link = Link::new("Example", "https://example.com");

// In a supporting terminal: clickable "Example" text
// In a non-supporting terminal: "Example [https://example.com]"
println!("{}", link.to_terminal());
```

#### `to_terminal_unchecked()`

Always outputs OSC 8 format, bypassing terminal capability detection:

```rust
let link = Link::new("Example", "https://example.com");
let osc8 = link.to_terminal_unchecked();
// Always produces: ESC ] 8 ; ; <URL> BEL <text> ESC ] 8 ; ; BEL
```

The implementation uses BEL (`\x07`) as the sequence terminator, which has broader terminal support than the ST terminator (`ESC \`).

### Targeting the Browser

#### Basic HTML Output

The `to_browser()` method renders a standard HTML anchor element:

```rust
let link = Link::new("Click", "https://example.com")
    .with_class("btn")
    .with_style("color: blue")
    .with_target("_blank")
    .with_title("Tooltip text");

let html = link.to_browser();
// <a href="https://example.com" class="btn" style="color: blue" target="_blank" title="Tooltip text">Click</a>
```

#### Modern Popover API Integration

For links with a `prompt` attribute, use `to_browser_with_popover()` to generate HTML that leverages the modern Popover API:

```rust
let link = Link::new("Hover me", "https://example.com")
    .with_prompt("This is tooltip content!")
    .with_class("link-with-tooltip");

if let Some((anchor, popover)) = link.to_browser_with_popover() {
    println!("Anchor: {}", anchor);
    // <a href="https://example.com" interestfor="popover-abc123" class="link-with-tooltip">Hover me</a>

    println!("Popover: {}", popover);
    // <div id="popover-abc123" popover="hint">This is tooltip content!</div>
}
```

This generates:
- An anchor with `interestfor="{id}"` for hover/focus activation
- A companion `<div>` with `popover="hint"` containing the prompt content

For more details on the Popover API, see [Modern Popovers in the Browser](./modern-popovers-in-the-browser.md).

#### Browser Support Notes

The `interestfor` attribute is experimental (as of 2025). Use feature detection in your JavaScript:

```javascript
// Check for interest invokers support
const supportsInterest = 'interestForElement' in HTMLButtonElement.prototype;

// Check for basic popover support
const supportsPopover = 'popover' in HTMLElement.prototype;
```

For browsers that don't support `interestfor`, you may need JavaScript fallbacks:

```javascript
if (!supportsInterest) {
    document.querySelectorAll('a[interestfor]').forEach(anchor => {
        const targetId = anchor.getAttribute('interestfor');
        const popover = document.getElementById(targetId);

        anchor.addEventListener('mouseenter', () => popover?.showPopover());
        anchor.addEventListener('mouseleave', () => popover?.hidePopover());
    });
}
```

## Parsing Links

The `Link` struct can parse both HTML and Markdown link formats:

```rust
use shared::render::link::Link;
use std::convert::TryFrom;

// Parse HTML
let html_link = Link::try_from(r#"<a href="https://example.com" class="btn">Click</a>"#)?;

// Parse Markdown (basic)
let md_link = Link::try_from("[Click](https://example.com)")?;

// Parse Markdown (with title)
let md_titled = Link::try_from(r#"[Click](https://example.com "My tooltip")"#)?;

// Parse Markdown (structured mode)
let md_structured = Link::try_from(
    r#"[Click](https://example.com class="btn" prompt="hover text")"#
)?;
```
