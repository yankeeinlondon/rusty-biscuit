---
name: syntect
description: Comprehensive guide to Syntect - high-performance syntax highlighting library for Rust using Sublime Text syntax definitions
created: 2025-12-23
last_updated: 2025-12-24T13:35:00Z
hash: 88d97fd6a3db3deb
tags:
  - rust
  - syntax-highlighting
  - terminal
  - html
  - sublime-text
  - cli
  - themes
  - typescript
---

# Syntect: High-Performance Syntax Highlighting in Rust

**Syntect** is a high-performance syntax highlighting library for Rust that utilizes **Sublime Text syntax definitions** (`.sublime-syntax`) and **TextMate themes** (`.tmTheme`). It is widely considered the industry standard for Rust-based syntax highlighting, used in projects like `bat`, `delta`, and various static site generators.

## Table of Contents

- [Core Architecture](#core-architecture)
- [Getting Started](#getting-started)
- [Terminal Output](#terminal-output)
- [HTML Generation](#html-generation)
  - [Inline Styles](#inline-styles)
  - [CSS Classes](#css-classes)
- [Advanced Usage](#advanced-usage)
  - [Streaming Large Files](#streaming-large-files)
  - [Binary Dumps for Performance](#binary-dumps-for-performance)
- [Modern Themes](#modern-themes)
  - [Loading External Themes](#loading-external-themes)
  - [Bundling Themes in Binary](#bundling-themes-in-binary)
  - [Theme Binary Dumps](#theme-binary-dumps)
- [Language Support](#language-support)
  - [TypeScript Grammar](#typescript-grammar)
  - [Custom Syntax Definitions](#custom-syntax-definitions)
- [Comparison with Alternatives](#comparison-with-alternatives)
- [Quick Reference](#quick-reference)

## Core Architecture

Syntect is divided into two primary logical components:

1. **Parsing:** Converts raw text into a stream of **scopes** (e.g., `keyword.control.rust`). It uses a state machine to track nested language structures.
2. **Highlighting:** Maps those scopes to specific **styles** (colors, bold, italics) based on a chosen theme.

### Key Features

- **Speed:** Highlighting is extremely fast (thousands of lines per second)
- **Accuracy:** Because it uses the same engine as Sublime Text, it supports complex features like multi-line regex and nested syntaxes (e.g., JavaScript inside HTML)
- **Memory Efficiency:** Can be used with a "pre-compiled" binary dump of syntaxes and themes, meaning you don't need to ship thousands of `.sublime-syntax` files with your application

## Getting Started

Add Syntect to your `Cargo.toml`:

```toml
[dependencies]
syntect = "5.2"
```

The library provides two main data structures:

- **SyntaxSet**: Collection of language syntax definitions
- **ThemeSet**: Collection of color themes

Both can be loaded from defaults bundled with the library or from custom files.

## Terminal Output

This is the most common use case for CLI tools. Syntect provides a utility to convert styled ranges into **24-bit (TrueColor) ANSI escape sequences**.

```rust
use syntect::easy::HighlightLines;
use syntect::parsing::SyntaxSet;
use syntect::highlighting::{ThemeSet, Style};
use syntect::util::{as_24_bit_terminal_escaped, LinesWithEndings};

fn main() {
    // 1. Load the default syntaxes and themes (bundled in the library)
    let ps = SyntaxSet::load_defaults_newlines();
    let ts = ThemeSet::load_defaults();

    // 2. Select a syntax (e.g., Rust) and a theme
    let syntax = ps.find_syntax_by_extension("rs").unwrap();
    let mut h = HighlightLines::new(syntax, &ts.themes["base16-ocean.dark"]);

    let code = r#"
fn main() {
    println!("Hello, Syntect!");
}
"#;

    // 3. Highlight line by line
    for line in LinesWithEndings::from(code) {
        let ranges: Vec<(Style, &str)> = h.highlight_line(line, &ps).unwrap();

        // 4. Convert styled ranges to ANSI escape codes for the terminal
        let escaped = as_24_bit_terminal_escaped(&ranges[..], false);
        print!("{}", escaped);
    }
    // Reset terminal color at the end
    println!("\x1b[0m");
}
```

**Key Points:**

- Use `SyntaxSet::load_defaults_newlines()` for proper line ending handling
- The `HighlightLines` struct maintains parsing state across multiple lines
- Always reset terminal colors after highlighting (`\x1b[0m`)
- The second parameter to `as_24_bit_terminal_escaped()` controls background color output

## HTML Generation

If you are building a blog, documentation tool, or web application, you need HTML output. Syntect can generate either **inline-styled spans** or **class-based spans** for use with CSS.

### Inline Styles

This approach generates self-contained HTML with colors embedded directly in `style` attributes. Best for simple use cases or when you need standalone HTML snippets.

```rust
use syntect::html::highlighted_html_for_string;
use syntect::parsing::SyntaxSet;
use syntect::highlighting::ThemeSet;

fn main() {
    let ps = SyntaxSet::load_defaults_newlines();
    let ts = ThemeSet::load_defaults();
    let syntax = ps.find_syntax_by_extension("rs").unwrap();
    let theme = &ts.themes["base16-ocean.dark"];

    let code = "let x = 5;";

    // Generates a <pre style="..."> with nested <span> tags containing inline colors
    let html = highlighted_html_for_string(code, &ps, syntax, theme).unwrap();

    println!("{}", html);
}
```

**Advantages:**

- Self-contained output
- No external CSS dependencies
- Works immediately in any HTML context

**Disadvantages:**

- Larger file sizes
- Difficult to implement theme switching
- Less efficient for multiple code blocks

### CSS Classes

Instead of hardcoding colors, this outputs `<span class="keyword">`. You then provide a CSS file (which Syntect can also generate for you).

```rust
use syntect::html::{ClassedHTMLGenerator, ClassStyle};
use syntect::parsing::SyntaxSet;
use syntect::util::LinesWithEndings;

fn main() {
    let ps = SyntaxSet::load_defaults_newlines();
    let syntax = ps.find_syntax_by_extension("rs").unwrap();
    let mut html_generator = ClassedHTMLGenerator::new_with_class_style(
        syntax, &ps, ClassStyle::Spaced
    );

    let code = "fn main() {}";
    for line in LinesWithEndings::from(code) {
        html_generator.parse_html_for_line_which_includes_newline(line).unwrap();
    }

    let html = html_generator.finalize();
    println!("{}", html);
    // Output: <span class="source rust"><span class="meta function rust">...</span></span>
}
```

**Advantages:**

- Smaller HTML output
- Easy theme switching via CSS
- Better performance for multiple code blocks
- Can leverage browser caching for CSS

**Disadvantages:**

- Requires separate CSS file
- More complex setup

**ClassStyle Options:**

- `ClassStyle::Spaced`: Generates class names like `source rust`
- `ClassStyle::SpacedPrefixed { prefix: "s-" }`: Generates `s-source s-rust`

## Advanced Usage

### Streaming Large Files

For large files, loading the entire content into a string is inefficient. Syntect provides a `HighlightFile` helper for streaming.

```rust
use syntect::easy::HighlightFile;
use syntect::highlighting::{ThemeSet, Style};
use syntect::parsing::SyntaxSet;
use syntect::util::as_24_bit_terminal_escaped;
use std::io::BufRead;

fn highlight_large_file(path: &str) {
    let ps = SyntaxSet::load_defaults_newlines();
    let ts = ThemeSet::load_defaults();

    let mut highlighter = HighlightFile::new(
        path,
        &ps,
        &ts.themes["base16-ocean.dark"]
    ).expect("Unable to open file");

    let mut line = String::new();
    // Read the file line by line without loading the whole thing into memory
    while highlighter.reader.read_line(&mut line).unwrap() > 0 {
        let regions: Vec<(Style, &str)> = highlighter
            .highlight_lines
            .highlight_line(&line, &ps)
            .unwrap();
        print!("{}", as_24_bit_terminal_escaped(&regions[..], false));
        line.clear();
    }
}
```

**Benefits:**

- Constant memory usage regardless of file size
- Can process files larger than available RAM
- Better performance for very large files

### Binary Dumps for Performance

If you find your application's startup time is slow because it's loading many syntaxes, use `SyntaxSetBuilder::build().to_binary()` during build time and `SyntaxSet::from_binary()` at runtime.

**Build Script Example:**

```rust
use syntect::parsing::SyntaxSetBuilder;
use std::fs::File;
use std::io::Write;

fn main() {
    let mut builder = SyntaxSetBuilder::new();
    builder.add_from_folder("path/to/syntaxes", true).unwrap();
    let ss = builder.build();

    let mut file = File::create("syntaxes.bin").unwrap();
    file.write_all(&ss.to_binary()).unwrap();
}
```

**Runtime Loading:**

```rust
use syntect::parsing::SyntaxSet;
use std::fs;

fn main() {
    let data = fs::read("syntaxes.bin").unwrap();
    let ps = SyntaxSet::from_binary(&data);
    // Use ps normally...
}
```

**Performance Impact:**

- Reduces startup time from ~100ms to <5ms
- Significantly smaller binary size when not bundling source files
- One-time build step complexity

## Modern Themes

By default, `syntect` includes a limited set of built-in themes (like `base16-ocean.dark` and `Solarized`) that are quite dated. To use modern themes like **Dracula**, **Nord**, **Ayu**, or **One Dark**, you can load external `.tmTheme` files or use helper crates.

### Loading External Themes

The most stable way to bring in new themes is to download their `.tmTheme` (XML) versions. Even though Sublime Text has moved to a JSON format, almost every popular theme maintains a "TextMate" or "Legacy" port.

**Where to find themes:**

- **Dracula:** [dracula/textmate](https://github.com/dracula/textmate)
- **Nord:** [nordtheme/sublime-text](https://github.com/nordtheme/sublime-text)
- **One Dark:** [Ice-Man-007/sublime-text-one-dark-theme](https://github.com/Ice-Man-007/sublime-text-one-dark-theme)
- **Rainglow:** [rainglow/sublime](https://github.com/rainglow/sublime) - Over 320 themes in one collection

**Loading a single theme:**

```rust
use syntect::highlighting::ThemeSet;

fn main() {
    let theme = ThemeSet::get_theme("path/to/Dracula.tmTheme")
        .expect("Failed to load theme");
    // Use theme for highlighting...
}
```

**Loading a folder of themes:**

```rust
use syntect::highlighting::ThemeSet;

fn main() {
    let mut theme_set = ThemeSet::load_defaults();
    let extra_themes = ThemeSet::load_from_folder("assets/themes")
        .expect("Failed to load folder");

    // Merge with defaults
    theme_set.themes.extend(extra_themes.themes);
}
```

**Modern JSON-based themes:**

Modern Sublime themes use a JSON-based `.sublime-color-scheme` format which `syntect` does not support natively. However, the community-made [sublime-color-scheme](https://crates.io/crates/sublime-color-scheme) crate can convert them:

```toml
[dependencies]
syntect = "5.0"
sublime-color-scheme = "0.2"
```

```rust
use syntect::highlighting::Theme;
use sublime_color_scheme::convert_sublime_scheme;

fn load_modern_theme(json_content: &str) -> Theme {
    convert_sublime_scheme(json_content)
        .expect("Failed to parse modern theme")
}
```

### Bundling Themes in Binary

For portable applications, you can embed themes directly in the executable using `include_bytes!`:

```rust
use std::io::Cursor;
use syntect::highlighting::{Theme, ThemeSet};

fn main() {
    // Embed the file at compile time
    let theme_bytes = include_bytes!("../assets/themes/Dracula.tmTheme");

    // Wrap bytes in a Cursor to provide a 'Read' trait
    let mut cursor = Cursor::new(theme_bytes);

    // Load the theme from the reader
    let dracula_theme = ThemeSet::load_from_reader(&mut cursor)
        .expect("Failed to parse embedded theme");
}
```

**Managing multiple embedded themes:**

```rust
use std::io::Cursor;
use syntect::highlighting::{Theme, ThemeSet};

fn get_embedded_theme(name: &str) -> Option<Theme> {
    let bytes = match name {
        "dracula" => Some(include_bytes!("../assets/themes/Dracula.tmTheme") as &[u8]),
        "nord"    => Some(include_bytes!("../assets/themes/Nord.tmTheme") as &[u8]),
        "ayu"     => Some(include_bytes!("../assets/themes/Ayu.tmTheme") as &[u8]),
        _ => None,
    }?;

    ThemeSet::load_from_reader(&mut Cursor::new(bytes)).ok()
}
```

You can also use the [`rust-embed`](https://crates.io/crates/rust-embed) crate to automatically bundle an entire directory:

```rust
use rust_embed::RustEmbed;
use std::io::Cursor;
use syntect::highlighting::{Theme, ThemeSet};

#[derive(RustEmbed)]
#[folder = "assets/themes/"]
struct ThemeAssets;

fn load_theme_from_bundle(name: &str) -> Option<Theme> {
    let asset = ThemeAssets::get(name)?;
    let mut cursor = Cursor::new(asset.data.as_ref());
    ThemeSet::load_from_reader(&mut cursor).ok()
}
```

### Theme Binary Dumps

For maximum performance, use binary dumps to pre-compile themes at build time. This is the "gold standard" for professional applications using `syntect`.

**Why use binary dumps:**

| Feature | XML Loading | Binary Dumping |
|---------|-------------|----------------|
| **Startup Speed** | Slow (parsing 10+ XML files) | **Instant** (direct memory load) |
| **Binary Size** | No change (assets stay external) | Slightly larger (themes inside) |
| **Portability** | Requires `assets/` folder alongside executable | **Single binary** with everything inside |
| **Complexity** | Low | Medium (requires `build.rs`) |

**Step 1: Configure `Cargo.toml`:**

```toml
[dependencies]
syntect = "5.3"

[build-dependencies]
syntect = { version = "5.3", features = ["plist-load"] }
```

**Step 2: Create `build.rs`:**

```rust
use syntect::highlighting::ThemeSet;
use syntect::dumps::dump_to_file;
use std::env;
use std::path::Path;

fn main() {
    let out_dir = env::var("OUT_DIR").unwrap();
    let theme_dir = "assets/themes";
    let dump_path = Path::new(&out_dir).join("themes.bin");

    // Load all .tmTheme files into a ThemeSet
    let theme_set = ThemeSet::load_from_folder(theme_dir)
        .expect("Failed to load themes from folder");

    // Serialize to compressed binary
    dump_to_file(&theme_set, &dump_path)
        .expect("Failed to dump themes to binary");

    // Rebuild only if themes change
    println!("cargo:rerun-if-changed={}", theme_dir);
}
```

**Step 3: Load in `main.rs`:**

```rust
use syntect::highlighting::ThemeSet;
use syntect::dumps::from_binary;

fn main() {
    // Embed the binary dump
    let theme_data = include_bytes!(concat!(env!("OUT_DIR"), "/themes.bin"));

    // Instant load (no XML parsing!)
    let theme_set: ThemeSet = from_binary(theme_data);

    let my_theme = &theme_set.themes["Dracula"];
    println!("Loaded {} themes instantly.", theme_set.themes.len());
}
```

**Real-world implementation example:**

The TA (TypeScript Analyzer) project uses this approach with lazy static initialization for optimal performance:

```rust
use syntect::highlighting::ThemeSet;
use syntect::dumps::from_binary;
use once_cell::sync::Lazy;

// Lazy-loaded static theme set from binary dump
static THEME_SET: Lazy<ThemeSet> = Lazy::new(|| {
    let theme_data = include_bytes!(concat!(env!("OUT_DIR"), "/themes.bin"));
    from_binary(theme_data)
});

pub fn get_default_theme_set() -> &'static ThemeSet {
    &THEME_SET
}
```

**Performance characteristics:**

- Binary size increase: ~200KB for 10 modern themes
- Startup time: <1ms with lazy initialization
- Theme load time: ~0.001ms (binary deserialization)
- Memory overhead: ~150KB shared across all instances
- Runtime parsing: 0 (all parsing done at compile time)

## Language Support

### TypeScript Grammar

TypeScript is often not included in the default `SyntaxSet` or uses an outdated version. To properly highlight TypeScript code, you need to load a custom `.sublime-syntax` file.

**Option 1: Manual Loading**

Download the TypeScript grammar (recommended source: [bat project](https://github.com/sharkdp/bat/blob/master/assets/syntaxes/02_Extra/TypeScript.sublime-syntax)) and load it:

```rust
use syntect::parsing::{SyntaxSet, SyntaxSetBuilder};
use syntect::highlighting::{ThemeSet, Style};
use syntect::easy::HighlightLines;
use syntect::util::{as_24_bit_terminal_escaped, LinesWithEndings};

fn main() {
    // Build custom SyntaxSet with TypeScript
    let mut builder = SyntaxSetBuilder::new();
    builder.add_from_folder("assets/syntaxes/")
        .expect("Failed to load TS grammar");

    let ps = builder.build();
    let ts = ThemeSet::load_defaults();

    // Find TypeScript syntax
    let syntax = ps.find_syntax_by_extension("ts")
        .unwrap_or_else(|| ps.find_syntax_plain_text());

    let mut h = HighlightLines::new(syntax, &ts.themes["base16-ocean.dark"]);
    let code = "interface User { id: number; name: string; }";

    for line in LinesWithEndings::from(code) {
        let ranges: Vec<(Style, &str)> = h.highlight_line(line, &ps).unwrap();
        let escaped = as_24_bit_terminal_escaped(&ranges[..], true);
        println!("{}", escaped);
    }
}
```

**Option 2: Using `two-face` Crate**

The [two-face](https://crates.io/crates/two-face) crate provides extra syntax definitions (including TypeScript, TOML, and Dockerfile) that are missing from syntect's default set:

```toml
[dependencies]
syntect = "5.2"
two-face = { version = "0.4", features = ["syntect-onig"] }
```

```rust
use syntect::parsing::SyntaxSet;
use two_face;

fn main() {
    // Get a SyntaxSet that includes TypeScript automatically
    let ps = two_face::syntax::extra_newlines();

    let syntax = ps.find_syntax_by_extension("ts")
        .expect("TypeScript should be here!");

    // Proceed with highlighting...
}
```

**Important: Regex Engine Choice**

Modern TypeScript grammars use advanced regular expressions (like lookaheads):

- **Oniguruma (`onig`):** Default C library, most compatible with Sublime grammars
- **fancy-regex:** Pure-Rust alternative, but some complex TypeScript grammars might fail

If you encounter issues, ensure the `onig` feature is enabled:

```toml
[dependencies]
syntect = { version = "5.2", features = ["default-onig"] }
```

### Custom Syntax Definitions

You can load custom `.sublime-syntax` files for any language not included in the defaults:

```rust
use syntect::parsing::SyntaxSetBuilder;

fn main() {
    let mut builder = SyntaxSetBuilder::new();

    // Add custom syntaxes from folder
    builder.add_from_folder("assets/syntaxes/")
        .expect("Failed to load syntaxes");

    // Optionally add plain text fallback
    builder.add_plain_text_syntax();

    let ps = builder.build();
}
```

Like themes, syntaxes can also be dumped to binary format for faster loading. However, use `dump_to_uncompressed_file` for syntaxes as they are already internally compressed:

```rust
use syntect::parsing::SyntaxSetBuilder;
use syntect::dumps::dump_to_uncompressed_file;

fn main() {
    let mut builder = SyntaxSetBuilder::new();
    builder.add_from_folder("assets/syntaxes/").unwrap();
    let ss = builder.build();

    dump_to_uncompressed_file(&ss, "syntaxes.bin").unwrap();
}
```

## Comparison with Alternatives

| Crate | Best For | Pros | Cons |
|-------|----------|------|------|
| **Syntect** | General purpose, CLI, HTML | Standard Sublime support, very fast | Uses `oniguruma` (C dependency) or `fancy-regex` |
| **Tree-sitter** | IDEs, Code analysis | Semantic understanding (AST), resilient to syntax errors | Harder to set up for simple "coloring," slower for pure highlighting |
| **Pygments (Python)** | Quick scripts | Huge language support | Extremely slow; requires Python bridge |

**When to Choose Syntect:**

- Building CLI tools that need syntax highlighting (like `bat`)
- Static site generators requiring code highlighting
- Documentation tools
- Any application where speed and accuracy are important
- Projects that benefit from Sublime Text's extensive language support

**When to Choose Alternatives:**

- **Tree-sitter**: Building an IDE or code editor with semantic features
- **Pygments**: Quick Python scripts where performance isn't critical

## Quick Reference

### Common Syntax Detection Methods

```rust
// By file extension
let syntax = ps.find_syntax_by_extension("rs").unwrap();

// By first line (e.g., shebang)
let syntax = ps.find_syntax_by_first_line("#!/bin/bash").unwrap();

// By file name
let syntax = ps.find_syntax_by_name("Rust").unwrap();
```

### Default Themes

Common themes included in `ThemeSet::load_defaults()`:

- `base16-ocean.dark`
- `base16-ocean.light`
- `base16-mocha.dark`
- `InspiredGitHub`
- `Solarized (dark)`
- `Solarized (light)`

### Essential Imports

```rust
// For terminal output
use syntect::easy::HighlightLines;
use syntect::parsing::SyntaxSet;
use syntect::highlighting::{ThemeSet, Style};
use syntect::util::{as_24_bit_terminal_escaped, LinesWithEndings};

// For HTML output (inline)
use syntect::html::highlighted_html_for_string;

// For HTML output (classes)
use syntect::html::{ClassedHTMLGenerator, ClassStyle};
```

### Dependency Options

Syntect supports two regex engines:

```toml
# Default: Uses oniguruma (C dependency, very fast)
[dependencies]
syntect = "5.2"

# Pure Rust: Uses fancy-regex (slightly slower, no C deps)
[dependencies]
syntect = { version = "5.2", default-features = false, features = ["default-fancy"] }
```

## Resources

- **Official Documentation**: https://docs.rs/syntect
- **Repository**: https://github.com/trishume/syntect
- **Crates.io**: https://crates.io/crates/syntect
- **Sublime Text Syntax Definitions**: https://www.sublimetext.com/docs/syntax.html
- **TextMate Themes**: https://github.com/textmate/textmate/wiki/Themes

**Projects Using Syntect:**

- [bat](https://github.com/sharkdp/bat) - A `cat` clone with syntax highlighting
- [delta](https://github.com/dandavison/delta) - A syntax-highlighting pager for git
- [mdBook](https://github.com/rust-lang/mdBook) - Rust documentation tool
