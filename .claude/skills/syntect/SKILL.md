---
name: syntect
description: Expert knowledge for syntax highlighting in Rust using Syntect, a high-performance library that uses Sublime Text syntax definitions and TextMate themes for terminal ANSI output and HTML generation, including TypeScript support and modern themes
last_updated: 2025-12-24T00:00:00Z
hash: 285587f8ac19de6c
---

# Syntect

Syntect is a high-performance syntax highlighting library for Rust that uses **Sublime Text syntax definitions** (`.sublime-syntax`) and **TextMate themes** (`.tmTheme`). It's the industry standard for Rust-based syntax highlighting, used in `bat`, `delta`, and various static site generators.

## Core Principles

- **Two-stage pipeline**: Parsing (text → scopes) then Highlighting (scopes → styles)
- **Line-by-line processing**: Process code incrementally for memory efficiency
- **Pre-load defaults**: Use `SyntaxSet::load_defaults_newlines()` and `ThemeSet::load_defaults()` for convenience
- **Binary dumps for production**: Compile syntaxes/themes to binary at build time for <5ms startup
- **ANSI for terminals**: Use `as_24_bit_terminal_escaped()` for 24-bit color terminal output
- **HTML has two modes**: Inline styles (self-contained) or CSS classes (better performance/theme switching)
- **Streaming for large files**: Use `HighlightFile` for memory-efficient file processing
- **Always reset terminal**: Print `\x1b[0m` after ANSI output to reset colors

## Setup

```toml
[dependencies]
syntect = "5.2"
```

## Quick Reference

### Terminal Output (ANSI Escape Codes)

```rust
use syntect::easy::HighlightLines;
use syntect::parsing::SyntaxSet;
use syntect::highlighting::ThemeSet;
use syntect::util::{as_24_bit_terminal_escaped, LinesWithEndings};

let ps = SyntaxSet::load_defaults_newlines();
let ts = ThemeSet::load_defaults();
let syntax = ps.find_syntax_by_extension("rs").unwrap();
let mut h = HighlightLines::new(syntax, &ts.themes["base16-ocean.dark"]);

let code = "fn main() {}\n";
for line in LinesWithEndings::from(code) {
    let ranges = h.highlight_line(line, &ps).unwrap();
    print!("{}", as_24_bit_terminal_escaped(&ranges[..], false));
}
println!("\x1b[0m"); // Reset terminal color
```

### HTML Output (Inline Styles)

```rust
use syntect::html::highlighted_html_for_string;
use syntect::parsing::SyntaxSet;
use syntect::highlighting::ThemeSet;

let ps = SyntaxSet::load_defaults_newlines();
let ts = ThemeSet::load_defaults();
let syntax = ps.find_syntax_by_extension("rs").unwrap();
let theme = &ts.themes["base16-ocean.dark"];

let html = highlighted_html_for_string("let x = 5;", &ps, syntax, theme).unwrap();
// Generates <pre style="..."> with nested <span> tags with inline colors
```

### HTML Output (CSS Classes)

```rust
use syntect::html::{ClassedHTMLGenerator, ClassStyle};
use syntect::parsing::SyntaxSet;
use syntect::util::LinesWithEndings;

let ps = SyntaxSet::load_defaults_newlines();
let syntax = ps.find_syntax_by_extension("rs").unwrap();
let mut html_gen = ClassedHTMLGenerator::new_with_class_style(
    syntax, &ps, ClassStyle::Spaced
);

for line in LinesWithEndings::from("fn main() {}") {
    html_gen.parse_html_for_line_which_includes_newline(line).unwrap();
}

let html = html_gen.finalize();
// Output: <span class="source rust"><span class="meta function rust">...</span></span>
```

## Topics

### Output Targets

- [Terminal/Console Output](./terminal-output.md) - ANSI escape codes for CLI tools
- [HTML Generation](./html-generation.md) - Inline styles and CSS class approaches
- [Large File Handling](./large-files.md) - Streaming with `HighlightFile`

### Language Support

- [TypeScript Grammar](./typescript-grammar.md) - Loading TypeScript syntax definitions and using the two-face crate

### Optimization

- [Binary Dumps](./binary-dumps.md) - Pre-compile syntaxes/themes for fast startup, modern themes integration

## Common Patterns

### Finding a Syntax by Extension

```rust
let ps = SyntaxSet::load_defaults_newlines();
let syntax = ps.find_syntax_by_extension("rs").unwrap();
```

### Finding a Syntax by First Line (Shebang)

```rust
let syntax = ps.find_syntax_by_first_line("#!/usr/bin/env python3");
```

### Available Default Themes

Common themes in `ThemeSet::load_defaults()`:
- `base16-ocean.dark`
- `base16-ocean.light`
- `InspiredGitHub`
- `Solarized (dark)`
- `Solarized (light)`

## When to Use Syntect

| Use Case | Choose Syntect When |
|----------|-------------------|
| CLI tools | Need fast, accurate terminal highlighting |
| Static site generators | Generating HTML from code blocks |
| Documentation tools | Rendering code examples in docs |
| Code viewers | Displaying source code with syntax colors |

**Alternatives:**
- **Tree-sitter**: For semantic analysis, ASTs, or IDE features (slower for pure highlighting)
- **Pygments (Python)**: For Python-based tools (much slower, requires Python bridge)

## Resources

- [Crates.io](https://crates.io/crates/syntect)
- [Documentation](https://docs.rs/syntect/)
- [GitHub](https://github.com/trishume/syntect)
