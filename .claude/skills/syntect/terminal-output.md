# Terminal/Console Output

Syntect provides utilities to convert styled ranges into **24-bit (TrueColor) ANSI escape sequences** for terminal display. This is the most common use case for CLI tools like `bat` and `delta`.

## Basic Terminal Highlighting

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

## Key Concepts

### `LinesWithEndings`

The `LinesWithEndings` iterator preserves line endings (`\n`), which is important for correct highlighting of multi-line constructs. Always use this instead of `.lines()` when working with Syntect.

### `as_24_bit_terminal_escaped()`

This function converts styled ranges into ANSI escape sequences:

```rust
pub fn as_24_bit_terminal_escaped(
    ranges: &[(Style, &str)],
    true_color: bool
) -> String
```

**Parameters:**
- `ranges`: The styled text ranges from `highlight_line()`
- `true_color`: Set to `true` for 24-bit color, `false` for backwards compatibility (both produce 24-bit in practice)

### Always Reset Terminal

After outputting ANSI-colored text, always reset the terminal to prevent color bleed:

```rust
println!("\x1b[0m");
```

## Patterns

### Highlighting from stdin

```rust
use std::io::{self, BufRead};
use syntect::easy::HighlightLines;
use syntect::parsing::SyntaxSet;
use syntect::highlighting::ThemeSet;
use syntect::util::as_24_bit_terminal_escaped;

fn highlight_stdin(extension: &str) {
    let ps = SyntaxSet::load_defaults_newlines();
    let ts = ThemeSet::load_defaults();
    let syntax = ps.find_syntax_by_extension(extension).unwrap();
    let mut h = HighlightLines::new(syntax, &ts.themes["base16-ocean.dark"]);

    let stdin = io::stdin();
    for line in stdin.lock().lines() {
        let line = line.unwrap();
        let ranges = h.highlight_line(&line, &ps).unwrap();
        println!("{}", as_24_bit_terminal_escaped(&ranges[..], false));
    }
    println!("\x1b[0m");
}
```

### Detecting Syntax from Shebang

For files without extensions, use the first line:

```rust
let first_line = "#!/usr/bin/env python3";
let syntax = ps.find_syntax_by_first_line(first_line)
    .or_else(|| ps.find_syntax_plain_text());
```

## Gotchas

- **Don't forget to reset**: Always print `\x1b[0m` at the end
- **Use `LinesWithEndings`**: Don't use `.lines()` as it strips endings
- **State is preserved**: `HighlightLines` maintains state across lines for multi-line constructs
- **24-bit terminal support**: Ensure your terminal supports TrueColor (most modern terminals do)

## Related

- [Large File Handling](./large-files.md) - Streaming large files efficiently
- [Binary Dumps](./binary-dumps.md) - Optimizing startup time
