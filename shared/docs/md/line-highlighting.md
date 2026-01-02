# Line Highlighting

> **Syntax:**
>
>       highlight=4
>       highlight=4-6
>       highlight=4-6,8

The **highlight** feature allows you to highlight a particular row (or rows) in the code block. The value for `highlight` can be:

- a single number (indicating a single line should be highlighted)
- a range (e.g., `4-6`) which indicates a _range_ of line numbers which need to be highlighted
- a _list_ of single numbers or ranges:
    - `4-6,8`, `1,4,8`, and `1-3,8-10` are all valid representations for highlighting
    - the `,` character acts as a delimiter between groups
    - each member in a group is either a single number or a range of numbers

All code blocks start with line number 1.

> **Version Compatibility Note**
>
> This document targets:
> - `pulldown-cmark` 0.10+ (uses `TagEnd` for `Event::End` variants)
> - `syntect` 5.x
> - `two-face` 0.4+

---

## Example Code

### 1. The HighlightSpec Type

First, define a type to represent which lines should be highlighted. Using a `HashSet` provides O(1) lookup during rendering.

```rust
use std::collections::HashSet;

/// Represents which lines in a code block should be highlighted.
/// Line numbers are 1-indexed to match user expectations.
#[derive(Debug, Clone, Default)]
pub struct HighlightSpec {
    lines: HashSet<usize>,
}

impl HighlightSpec {
    /// Creates an empty spec (no lines highlighted).
    pub fn none() -> Self {
        Self::default()
    }

    /// Checks if a given line number should be highlighted.
    /// Line numbers are 1-indexed.
    pub fn contains(&self, line: usize) -> bool {
        self.lines.contains(&line)
    }

    /// Returns true if no lines are highlighted.
    pub fn is_empty(&self) -> bool {
        self.lines.is_empty()
    }
}
```

---

### 2. Parsing the Highlight Syntax

The parser handles three formats:
- Single line: `highlight=4`
- Range: `highlight=4-6`
- Mixed list: `highlight=4-6,8,10-12`

```rust
use std::collections::HashSet;
use std::str::FromStr;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ParseError {
    InvalidNumber(String),
    InvalidRange(String),
    EmptyValue,
}

impl std::fmt::Display for ParseError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::InvalidNumber(s) => write!(f, "invalid line number: '{}'", s),
            Self::InvalidRange(s) => write!(f, "invalid range: '{}'", s),
            Self::EmptyValue => write!(f, "empty highlight value"),
        }
    }
}

impl std::error::Error for ParseError {}

impl FromStr for HighlightSpec {
    type Err = ParseError;

    /// Parses a highlight value like "4", "4-6", or "4-6,8,10-12".
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let s = s.trim();
        if s.is_empty() {
            return Err(ParseError::EmptyValue);
        }

        let mut lines = HashSet::new();

        for part in s.split(',') {
            let part = part.trim();
            if part.is_empty() {
                continue;
            }

            if let Some((start, end)) = part.split_once('-') {
                // Range: "4-6"
                let start: usize = start
                    .trim()
                    .parse()
                    .map_err(|_| ParseError::InvalidNumber(start.to_string()))?;
                let end: usize = end
                    .trim()
                    .parse()
                    .map_err(|_| ParseError::InvalidNumber(end.to_string()))?;

                if start > end {
                    return Err(ParseError::InvalidRange(part.to_string()));
                }

                for line in start..=end {
                    lines.insert(line);
                }
            } else {
                // Single line: "4"
                let line: usize = part
                    .parse()
                    .map_err(|_| ParseError::InvalidNumber(part.to_string()))?;
                lines.insert(line);
            }
        }

        Ok(Self { lines })
    }
}
```

---

### 3. Extracting Highlight from the Info String

The code fence info string (e.g., `rust highlight=4-6`) needs parsing to extract both the language and the highlight spec.

```rust
/// Parsed metadata from a fenced code block's info string.
#[derive(Debug, Clone, Default)]
pub struct CodeBlockMeta {
    pub language: Option<String>,
    pub highlight: HighlightSpec,
}

impl CodeBlockMeta {
    /// Parses an info string like "rust highlight=4-6,8" or "python".
    pub fn parse(info: &str) -> Self {
        let info = info.trim();
        if info.is_empty() {
            return Self::default();
        }

        let mut parts = info.split_whitespace();
        let language = parts.next().map(|s| s.to_string());

        let mut highlight = HighlightSpec::none();

        for part in parts {
            if let Some(value) = part.strip_prefix("highlight=") {
                if let Ok(spec) = value.parse() {
                    highlight = spec;
                }
            }
            // Additional DSL options (title=, line-numbering=) can be parsed here
        }

        Self { language, highlight }
    }
}
```

---

### 4. HTML Rendering with Highlighted Lines

Highlighted lines receive an additional CSS class for styling.

```rust
use syntect::easy::HighlightLines;
use syntect::highlighting::Theme;
use syntect::html::{styled_line_to_highlighted_html, IncludeBackground};
use syntect::parsing::SyntaxSet;

fn render_html_with_highlights(
    code: &str,
    lang: &str,
    highlight: &HighlightSpec,
    ss: &SyntaxSet,
    theme: &Theme,
) -> String {
    let syntax = ss
        .find_syntax_by_token(lang)
        .unwrap_or_else(|| ss.find_syntax_plain_text());
    let mut h = HighlightLines::new(syntax, theme);

    let mut html = String::from("<pre class=\"code-block\"><table class=\"code-table\">");

    for (i, line) in code.trim_end_matches('\n').lines().enumerate() {
        let line_num = i + 1; // 1-indexed
        let line_with_nl = format!("{}\n", line);
        let regions = h.highlight_line(&line_with_nl, ss).unwrap();
        let highlighted_code = styled_line_to_highlighted_html(&regions[..], IncludeBackground::No);

        // Apply highlight class if this line should be highlighted
        let row_class = if highlight.contains(line_num) {
            "class=\"highlighted-line\""
        } else {
            ""
        };

        html.push_str(&format!(
            "<tr {}><td class=\"ln-gutter\">{}</td><td class=\"code-content\">{}</td></tr>",
            row_class, line_num, highlighted_code
        ));
    }

    html.push_str("</table></pre>");
    html
}
```

---

### 5. Terminal Rendering with Highlighted Lines

For terminal output, use a distinct background color for highlighted lines.

```rust
use syntect::easy::HighlightLines;
use syntect::parsing::SyntaxSet;
use syntect::util::as_24_bit_terminal_escaped;

fn render_terminal_with_highlights(
    code: &str,
    lang: &str,
    highlight: &HighlightSpec,
    ss: &SyntaxSet,
    theme: &syntect::highlighting::Theme,
) {
    let syntax = ss
        .find_syntax_by_token(lang)
        .unwrap_or_else(|| ss.find_syntax_plain_text());
    let mut h = HighlightLines::new(syntax, theme);

    // ANSI escape codes
    let grey = "\x1b[38;5;244m";           // Line number color
    let reset = "\x1b[0m";
    let highlight_bg = "\x1b[48;2;50;50;30m"; // Subtle yellow background for highlights

    for (i, line) in code.trim_end_matches('\n').lines().enumerate() {
        let line_num = i + 1;
        let line_with_nl = format!("{}\n", line);
        let regions = h.highlight_line(&line_with_nl, ss).unwrap();
        let escaped = as_24_bit_terminal_escaped(&regions[..], false);

        if highlight.contains(line_num) {
            // Highlighted line: add background color
            println!(
                "{}{}{:>3} │ {}{}",
                highlight_bg, grey, line_num, escaped.trim_end(), reset
            );
        } else {
            // Normal line
            println!(
                "{}{:>3} │{} {}",
                grey, line_num, reset, escaped.trim_end()
            );
        }
    }
}
```

---

### 6. Integration with pulldown-cmark

Extract the highlight spec when entering a code block, then pass it to the renderer.

```rust
use pulldown_cmark::{Event, Parser, Tag, TagEnd, CodeBlockKind};

pub fn process_markdown_with_highlights(markdown: &str) -> String {
    let parser = Parser::new(markdown);
    let mut output = String::new();

    let mut in_code_block = false;
    let mut code_buffer = String::new();
    let mut meta = CodeBlockMeta::default();

    // Setup highlighting (in production, use lazy_static or OnceCell)
    let ss = syntect::parsing::SyntaxSet::load_defaults_newlines();
    let ts = two_face::theme::extra();
    let theme = &ts.themes["Monokai Pro"];

    for event in parser {
        match event {
            Event::Start(Tag::CodeBlock(kind)) => {
                in_code_block = true;
                let info = match kind {
                    CodeBlockKind::Fenced(info) => info.to_string(),
                    CodeBlockKind::Indented => String::new(),
                };
                meta = CodeBlockMeta::parse(&info);
            }
            Event::End(TagEnd::CodeBlock) => {
                in_code_block = false;

                let lang = meta.language.as_deref().unwrap_or("text");
                let html = render_html_with_highlights(
                    &code_buffer,
                    lang,
                    &meta.highlight,
                    &ss,
                    theme,
                );
                output.push_str(&html);

                code_buffer.clear();
                meta = CodeBlockMeta::default();
            }
            Event::Text(text) if in_code_block => {
                code_buffer.push_str(&text);
            }
            _ => {
                // Handle other events (headers, paragraphs, etc.)
            }
        }
    }

    output
}
```

---

### 7. CSS for Highlighted Lines

Style highlighted lines with a subtle background that doesn't interfere with syntax highlighting.

```css
/* Highlighted line background */
.highlighted-line {
    background-color: rgba(255, 255, 0, 0.1); /* Subtle yellow */
}

/* Alternative: mark in the gutter */
.highlighted-line .ln-gutter {
    background-color: rgba(255, 200, 0, 0.3);
    border-left: 3px solid #ffc800;
}

/* Dark theme variant */
@media (prefers-color-scheme: dark) {
    .highlighted-line {
        background-color: rgba(255, 255, 0, 0.08);
    }
}
```

---

### 8. Unit Tests

```rust
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_single_line() {
        let spec: HighlightSpec = "4".parse().unwrap();
        assert!(spec.contains(4));
        assert!(!spec.contains(3));
        assert!(!spec.contains(5));
    }

    #[test]
    fn test_parse_range() {
        let spec: HighlightSpec = "4-6".parse().unwrap();
        assert!(!spec.contains(3));
        assert!(spec.contains(4));
        assert!(spec.contains(5));
        assert!(spec.contains(6));
        assert!(!spec.contains(7));
    }

    #[test]
    fn test_parse_mixed_list() {
        let spec: HighlightSpec = "1-3,8,10-12".parse().unwrap();
        // Range 1-3
        assert!(spec.contains(1));
        assert!(spec.contains(2));
        assert!(spec.contains(3));
        // Gap
        assert!(!spec.contains(4));
        assert!(!spec.contains(7));
        // Single 8
        assert!(spec.contains(8));
        assert!(!spec.contains(9));
        // Range 10-12
        assert!(spec.contains(10));
        assert!(spec.contains(11));
        assert!(spec.contains(12));
        assert!(!spec.contains(13));
    }

    #[test]
    fn test_parse_with_whitespace() {
        let spec: HighlightSpec = " 4 - 6 , 8 ".parse().unwrap();
        assert!(spec.contains(4));
        assert!(spec.contains(5));
        assert!(spec.contains(6));
        assert!(spec.contains(8));
    }

    #[test]
    fn test_invalid_range() {
        let result: Result<HighlightSpec, _> = "6-4".parse(); // end < start
        assert!(matches!(result, Err(ParseError::InvalidRange(_))));
    }

    #[test]
    fn test_invalid_number() {
        let result: Result<HighlightSpec, _> = "abc".parse();
        assert!(matches!(result, Err(ParseError::InvalidNumber(_))));
    }

    #[test]
    fn test_code_block_meta_parsing() {
        let meta = CodeBlockMeta::parse("rust highlight=4-6,8");
        assert_eq!(meta.language, Some("rust".to_string()));
        assert!(meta.highlight.contains(4));
        assert!(meta.highlight.contains(5));
        assert!(meta.highlight.contains(6));
        assert!(meta.highlight.contains(8));
    }

    #[test]
    fn test_code_block_meta_no_highlight() {
        let meta = CodeBlockMeta::parse("python");
        assert_eq!(meta.language, Some("python".to_string()));
        assert!(meta.highlight.is_empty());
    }
}
```

---

## Design Considerations

| Aspect | Decision | Rationale |
| --- | --- | --- |
| **Data Structure** | `HashSet<usize>` | O(1) lookup per line during rendering |
| **Line Indexing** | 1-indexed | Matches user mental model and Markdown conventions |
| **Error Handling** | `Result` with `ParseError` | Graceful degradation; invalid specs default to no highlighting |
| **CSS Strategy** | Background color | Preserves syntax highlighting colors |
| **Terminal Strategy** | ANSI background escape | Works in modern terminals with 24-bit color support |

## Edge Cases

1. **Empty highlight value**: `highlight=` → no lines highlighted (not an error)
2. **Out-of-range lines**: `highlight=100` on 10-line code → silently ignored
3. **Duplicate lines**: `highlight=4,4-6` → line 4 only highlighted once (HashSet deduplication)
4. **Zero or negative**: `highlight=0` → parse error (lines are 1-indexed)


