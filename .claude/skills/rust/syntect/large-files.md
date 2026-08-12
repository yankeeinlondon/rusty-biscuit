# Large File Handling

For large files, loading the entire content into a string is inefficient. Syntect provides `HighlightFile` for **streaming** file processing with minimal memory overhead.

## Using HighlightFile

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

    println!("\x1b[0m"); // Reset terminal
}
```

## Key Concepts

### Memory Efficiency

`HighlightFile` uses a `BufReader` internally, which:
- Reads the file in chunks (typically 8KB buffer)
- Processes one line at a time
- Reuses the same `String` buffer (`line.clear()` instead of allocating new strings)

**Result:** You can highlight gigabyte-sized files with constant memory usage.

### How It Works

`HighlightFile` is a wrapper around:
- `BufReader<File>` - For efficient file I/O
- `HighlightLines` - For syntax highlighting state

```rust
pub struct HighlightFile<'a> {
    pub reader: BufReader<File>,
    pub highlight_lines: HighlightLines<'a>,
}
```

You access both fields directly:
- `highlighter.reader.read_line(&mut line)` - Read next line
- `highlighter.highlight_lines.highlight_line(&line, &ps)` - Highlight it

## Common Patterns

### Syntax Detection from Path

```rust
use std::path::Path;

fn highlight_file_auto_detect(path: &str) {
    let ps = SyntaxSet::load_defaults_newlines();
    let ts = ThemeSet::load_defaults();

    // Detect syntax from file extension
    let syntax = Path::new(path)
        .extension()
        .and_then(|ext| ext.to_str())
        .and_then(|ext| ps.find_syntax_by_extension(ext))
        .unwrap_or_else(|| ps.find_syntax_plain_text());

    let mut highlighter = HighlightFile::new(path, &ps, &ts.themes["base16-ocean.dark"])
        .expect("Unable to open file");

    let mut line = String::new();
    while highlighter.reader.read_line(&mut line).unwrap() > 0 {
        let ranges = highlighter.highlight_lines.highlight_line(&line, &ps).unwrap();
        print!("{}", as_24_bit_terminal_escaped(&ranges[..], false));
        line.clear();
    }
    println!("\x1b[0m");
}
```

### Error Handling

```rust
use std::io::{self, BufRead};

fn highlight_with_error_handling(path: &str) -> io::Result<()> {
    let ps = SyntaxSet::load_defaults_newlines();
    let ts = ThemeSet::load_defaults();

    let mut highlighter = HighlightFile::new(path, &ps, &ts.themes["base16-ocean.dark"])?;

    let mut line = String::new();
    while highlighter.reader.read_line(&mut line)? > 0 {
        let ranges = highlighter.highlight_lines.highlight_line(&line, &ps)
            .map_err(|e| io::Error::new(io::ErrorKind::Other, e))?;
        print!("{}", as_24_bit_terminal_escaped(&ranges[..], false));
        line.clear();
    }
    println!("\x1b[0m");
    Ok(())
}
```

### Line Numbers

Add line numbers alongside highlighted output:

```rust
let mut line = String::new();
let mut line_number = 1;

while highlighter.reader.read_line(&mut line).unwrap() > 0 {
    let ranges = highlighter.highlight_lines.highlight_line(&line, &ps).unwrap();
    print!("{:4} | {}", line_number, as_24_bit_terminal_escaped(&ranges[..], false));
    line.clear();
    line_number += 1;
}
```

## When to Use

| Scenario | Use HighlightFile |
|----------|-------------------|
| File size > 1MB | Yes - streaming saves memory |
| CLI tools (cat/bat-like) | Yes - user may pipe large files |
| Web services | Maybe - depends on max expected file size |
| Small snippets | No - `highlighted_html_for_string()` is simpler |

## Gotchas

- **Must clear the line buffer**: Always call `line.clear()` in the loop to reuse the buffer
- **State persists**: `HighlightLines` maintains state across lines (needed for multi-line constructs)
- **File must exist**: `HighlightFile::new()` returns `Result`, handle the error
- **Still line-by-line**: If you need to highlight only specific line ranges, you'll need to skip lines manually

## Related

- [Terminal Output](./terminal-output.md) - ANSI escape codes for display
- [Binary Dumps](./binary-dumps.md) - Optimize startup time for large syntax sets
