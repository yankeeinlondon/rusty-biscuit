# Mapping CSS Styling to the Terminal

> **Document Status**: Research notes compiled January 2026. Crate availability and APIs should be verified before use.

There are several Rust crates that bridge the gap between CSS-like styling and terminal escape codes. While most terminal libraries use a "fluent" builder API (e.g., `Style::new().bold().red()`), some crates allow you to use CSS syntax or logic directly.

---

## Established Crates (Production-Ready)

### 1. **colored** (Recommended)

The most popular terminal coloring crate with 26M+ downloads. Supports truecolor and hex values.

* **Crates.io**: https://crates.io/crates/colored
* **Key Features:**
  * Fluent API: `"text".red().bold()`
  * Truecolor: `.truecolor(r, g, b)` and `.on_truecolor(r, g, b)`
  * Hex colors: `.color("#ff8000")` and `.on_color("#0057B7")`
  * Respects `NO_COLOR` environment variable

* **Example Usage:**
```rust
use colored::Colorize;

println!("{}", "Warning!".yellow().bold());
println!("{}", "Custom".truecolor(255, 128, 0));
println!("{}", "Hex color".color("#ff8000"));
```

---

### 2. **owo-colors**

Zero-allocation, const-compatible terminal colors. Excellent for embedded or performance-critical applications.

* **Crates.io**: https://crates.io/crates/owo-colors
* **Key Features:**
  * Zero heap allocations
  * `const fn` compatible
  * Supports RGB and custom colors
  * Optional `supports-colors` feature for terminal detection

* **Example Usage:**
```rust
use owo_colors::OwoColorize;

println!("{}", "Error".red().bold());
println!("{}", "RGB".truecolor(255, 128, 0));
```

---

### 3. **nu-ansi-term**

Fork of `ansi_term` actively maintained by the Nushell team. Good for cross-platform terminal styling.

* **Crates.io**: https://crates.io/crates/nu-ansi-term
* **Key Features:**
  * RGB/Truecolor support via `Color::Rgb(r, g, b)`
  * 256-color palette via `Color::Fixed(n)`
  * Style composition with `.bold()`, `.italic()`, etc.

---

### 4. **colored_text**

A simple library supporting CSS color formats including hex and HSL.

* **Crates.io**: https://crates.io/crates/colored_text
* **Key Features:**
  * Hex colors: `.hex("#ff8000")` or `.hex("ff8000")`
  * HSL colors: `.hsl(h, s, l)` and `.on_hsl(h, s, l)`
  * RGB colors: `.rgb(r, g, b)`
  * Respects `NO_COLOR` environment variable

* **Example:**
```rust
use colored_text::Colorize;

println!("{}", "Orange Text".hex("#ff8000"));
println!("{}", "Seafoam".on_hsl(150.0, 100.0, 50.0));
println!("{}", "Pure Red".hsl(0.0, 100.0, 50.0));
```

---

### 5. **ansi-hex-color**

Focused utility for hex-to-ANSI-256 color conversion.

* **Crates.io**: https://crates.io/crates/ansi-hex-color
* **Best for:** Low-level color conversion when building custom styling systems

* **Example:**
```rust
use ansi_hex_color::colored;

let text = colored("#FF0000", "#004082", "Hello world");
println!("{}", text);
```

---

## Experimental Crates

### 6. **termio** (CSS-like Syntax)

> ⚠️ **Maturity Warning**: Created March 2025, currently at v0.1.0 with minimal commit history. Not recommended for production use until the project matures.

`termio` brings CSS-like syntax to terminal styling with a `tcss!` macro for defining stylesheets.

* **GitHub**: https://github.com/KirillFurtikov/termio
* **Key Features:**
  * CSS-like `@element` syntax
  * Supports `color`, `background`, `decoration`, `padding`, `border`
  * Runtime CSS parsing

* **Example Usage:**
```rust
use termio::Termio;

let mut tcss = Termio::new();
tcss.parse(r#"
    @element "warning" {
        color: yellow;
        background: black;
        decoration: bold;
        border: solid red;
    }
"#).unwrap();

let styled_text = "Warning!".style("warning", &tcss);
println!("{}", styled_text);
```

---

### 7. **Syntect** (Theme-Based Highlighting)

For mapping CSS-like themes (`.tmTheme` or Sublime Text styles) to terminal output. Industry standard for syntax highlighting.

* **Crates.io**: https://crates.io/crates/syntect
* **Best for:** Code syntax highlighting using existing editor themes
* **See also:** Local skill at `.claude/skills/syntect/SKILL.md`

---

## Summary Comparison

| Crate | Maturity | CSS Feature | Best Use Case |
|-------|----------|-------------|---------------|
| **`colored`** | Stable | Hex colors, truecolor | General terminal coloring |
| **`owo-colors`** | Stable | RGB/truecolor | Zero-allocation, embedded |
| **`nu-ansi-term`** | Stable | RGB, 256-color | Cross-platform styling |
| **`colored_text`** | Stable | Hex, HSL, RGB | Web-style color values |
| **`ansi-hex-color`** | Stable | Hex → ANSI-256 | Color conversion utilities |
| **`termio`** | ⚠️ Experimental | Full CSS-like syntax | Prototyping only |
| **`syntect`** | Stable | Theme-based mapping | Code syntax highlighting |

---

## Using `pulldown-cmark`, `syntect`, and `two-face`

Since this project uses `pulldown-cmark`, `syntect`, and `two-face`, you have a robust foundation for CSS-to-terminal mapping. The themes in `two-face` act as "stylesheets" mapping scopes to colors.

### Architectural Approach

1. **Theme Loading:** Use `two-face` to load themes (which are CSS-like XML mappings)
2. **State Tracking:** Track nested Markdown tags with a stack
3. **Color Mapping:** Convert `syntect::highlighting::Style` RGB values to ANSI escape codes

---

### Implementation Example

This example demonstrates mapping Markdown events to terminal colors using the existing stack. Based on the actual API usage in `shared/src/markdown/highlighting/themes.rs`.

```rust
use pulldown_cmark::{Event, Parser, Tag, HeadingLevel};
use syntect::highlighting::{Style, Theme};
use two_face::theme::{extra as extra_themes, EmbeddedThemeName};

/// Maps Markdown roles to terminal styles using syntect themes
struct TerminalHighlighter {
    theme: Theme,
}

impl TerminalHighlighter {
    fn from_theme(theme_name: EmbeddedThemeName) -> Self {
        let themes = extra_themes();
        let theme = themes.get(theme_name).clone();
        Self { theme }
    }

    /// Get the default foreground style from theme settings
    fn body_style(&self) -> Style {
        Style {
            foreground: self.theme.settings.foreground.unwrap_or_default(),
            background: self.theme.settings.background.unwrap_or_default(),
            font_style: syntect::highlighting::FontStyle::empty(),
        }
    }

    /// Convert syntect Style to 24-bit ANSI escape code
    fn to_ansi(&self, style: Style, text: &str) -> String {
        let fg = style.foreground;
        // TrueColor (24-bit) escape sequence: \x1b[38;2;R;G;Bm
        format!("\x1b[38;2;{};{};{}m{}\x1b[0m", fg.r, fg.g, fg.b, text)
    }

    /// Apply bold formatting
    fn to_ansi_bold(&self, style: Style, text: &str) -> String {
        let fg = style.foreground;
        format!("\x1b[1;38;2;{};{};{}m{}\x1b[0m", fg.r, fg.g, fg.b, text)
    }
}

fn main() {
    let markdown = "# Hello\nThis is **bold** text in the terminal.";
    let highlighter = TerminalHighlighter::from_theme(EmbeddedThemeName::Nord);
    let body_style = highlighter.body_style();

    let parser = Parser::new(markdown);
    let mut in_header = false;
    let mut in_bold = false;

    for event in parser {
        match event {
            Event::Start(Tag::Heading { level: HeadingLevel::H1, .. }) => in_header = true,
            Event::End(Tag::Heading { .. }) => {
                in_header = false;
                println!();
            }
            Event::Start(Tag::Strong) => in_bold = true,
            Event::End(Tag::Strong) => in_bold = false,
            Event::Text(text) => {
                let output = if in_header || in_bold {
                    highlighter.to_ansi_bold(body_style, &text)
                } else {
                    highlighter.to_ansi(body_style, &text)
                };
                print!("{}", output);
            }
            _ => {}
        }
    }
    println!();
}
```

---

### Why This Works

* **two-face & syntect:** Themes define colors for scopes, acting as "stylesheets"
* **pulldown-cmark:** Acts as the DOM parser, mapping tags to style roles
* **TrueColor escape codes:** `syntect` provides `Color { r, g, b, a }` for exact color reproduction

### Parsing Actual CSS

To parse a `.css` file and apply it to this pipeline, use the **`cssparser`** crate (used by Servo). Parse declarations like `color: #ff0000` into RGB triplets, then pass them to the `to_ansi` logic above.

* **Crates.io**: https://crates.io/crates/cssparser
