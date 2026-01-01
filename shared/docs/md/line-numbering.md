Integrating line numbers into a rendering pipeline using `pulldown-cmark`, `syntect`, and `two-face` requires a custom approach. Since `pulldown-cmark` doesn't provide line numbering out of the box, you must intercept the code block events and process them manually.

Here is how you can handle this for both HTML and Terminal output.

---

## 1. The Strategy: Custom Event Mapping

Instead of passing the `pulldown-cmark` events directly to a string, you iterate through the events. When you encounter `Event::Start(Tag::CodeBlock(kind))`, you buffer the code, highlight it using `syntect`/`two-face`, and inject line numbers during the string construction.

### HTML Output

For HTML, the cleanest way is to use an HTML table or a flexbox layout where line numbers are in a separate non-selectable column.

```rust
use pulldown_cmark::{Event, Parser, Tag, CodeBlockKind};
use syntect::parsing::SyntaxSet;
use syntect::highlighting::ThemeSet;
use syntect::html::highlighted_html_for_string;

// In your rendering loop:
let mut html_output = String::new();
let ss = SyntaxSet::load_defaults_newlines();
let ts = ThemeSet::load_defaults();
let theme = &ts.themes["base16-ocean.dark"];

for event in parser {
    match event {
        Event::Start(Tag::CodeBlock(CodeBlockKind::Fenced(lang))) => {
            // 1. Capture the code content until Event::End
            // 2. Split the highlighted HTML by lines
            // 3. Wrap each line in a <tr> or <span> with a counter
        }
        _ => { /* handle other events */ }
    }
}

```

**Implementation Tip:** `syntect` provides `start_highlighted_html_for_string`, but for line numbers, it’s often easier to use the `HighlightLines` API to get an iterator of styled ranges, then wrap each line in:
`<span class="line-number">1</span><span class="line-content">...</span>`

---

## 2. Terminal Output

For the terminal, you can use the `two-face` library (which provides extra themes and integration) alongside `syntect`’s `Escaped` output or `console` crate for styling.

### Example: Logic for Terminal Line Numbers

```rust
use syntect::easy::HighlightLines;
use syntect::util::as_24_bit_terminal_escaped;

let mut highlighter = HighlightLines::new(syntax, theme);
let lines = code_content.lines();

for (idx, line) in lines.enumerate() {
    let ranges: Vec<(Style, &str)> = highlighter.highlight_line(line, &ss)?;
    let escaped = as_24_bit_terminal_escaped(&ranges[..], false);
    
    // Print the line number in a muted color, then the highlighted code
    println!("\x1b[38;5;244m{:>3} │\x1b[0m {}", idx + 1, escaped);
}

```

---

## 3. Comparison of Approaches

| Feature | HTML Approach | Terminal Approach |
| --- | --- | --- |
| **Styling** | CSS (`user-select: none` for numbers) | ANSI Escape Codes |
| **Layout** | Tables or Flexbox | String padding/formatting |
| **Library** | `syntect::html` | `syntect::util::as_24_bit_terminal_escaped` |

---

## Key Considerations

* **The "New Line" Problem:** `syntect` expects lines to end with `\n` to maintain state correctly across multiline comments. Ensure you don't strip these before passing them to the highlighter.
* **Two-Face Extra:** If you are using `two-face`, ensure you initialize your `SyntaxSet` and `ThemeSet` using the `two_face::theme_set()` to access the specialized themes like "Monokai Pro" or "GitHub Dark".
* **Performance:** For large Markdown files, avoid re-initializing the `SyntaxSet` inside the loop. Move it to a `Lazy` static or pass a reference.

## Example

To implement this, you need to step away from the high-level "all-in-one" functions and use the `HighlightLines` API. This allows you to process the code line-by-line, giving you the hook needed to inject a line counter.

Here is a complete logic flow for both targets.

### 1. HTML Implementation

To ensure line numbers aren't copied when a user selects the code, we use a `data-line-number` attribute or a specific CSS class with `user-select: none`.

```rust
use syntect::easy::HighlightLines;
use syntect::html::{styled_line_to_highlighted_html, IncludeBackground};
use syntect::parsing::SyntaxSet;
use syntect::highlighting::{ThemeSet, Style};

fn render_html_with_line_numbers(code: &str, syntax_name: &str) -> String {
    let ss = SyntaxSet::load_defaults_newlines();
    let ts = ThemeSet::load_defaults();
    let syntax = ss.find_syntax_by_token(syntax_name).unwrap_or_else(|| ss.find_syntax_plain_text());
    let theme = &ts.themes["base16-ocean.dark"];
    
    let mut h = HighlightLines::new(syntax, theme);
    let mut html = String::from("<pre class=\"code\"><table>");

    for (i, line) in code.lines().enumerate() {
        let regions = h.highlight_line(line, &ss).unwrap();
        let highlighted_line = styled_line_to_highlighted_html(&regions[..], IncludeBackground::No).unwrap();
        
        // Wrap in table rows for perfect alignment
        html.push_str(&format!(
            "<tr><td class=\"ln\">{}</td><td>{}</td></tr>",
            i + 1,
            highlighted_line
        ));
    }

    html.push_str("</table></pre>");
    html
}

```

### 2. Terminal Implementation

For the terminal, we use ANSI escape codes. `two-face` provides great themes, but the actual printing is handled by `syntect`'s terminal utilities.

```rust
use syntect::util::as_24_bit_terminal_escaped;

fn render_terminal_with_line_numbers(code: &str, syntax_name: &str) {
    let ss = SyntaxSet::load_defaults_newlines();
    let theme = two_face::theme_set().get_theme("Monokai Pro"); // Example using two-face
    let syntax = ss.find_syntax_by_token(syntax_name).unwrap_or_else(|| ss.find_syntax_plain_text());

    let mut h = HighlightLines::new(syntax, theme);

    for (i, line) in code.lines().enumerate() {
        // Ensure the line has a newline for state tracking, but we'll trim for printing
        let line_with_nl = format!("{}\n", line);
        let regions = h.highlight_line(&line_with_nl, &ss).unwrap();
        let escaped = as_24_bit_terminal_escaped(&regions[..], false);
        
        // Print: Gray line number + Vertical pipe + Highlighted code
        println!("\x1b[38;5;244m{:>3} │\x1b[0m {}", i + 1, escaped.trim_end());
    }
}

```

---

### Integrating with `pulldown-cmark`

Since you are using `pulldown-cmark`, you need to intercept the events. Here is the architecture of the loop:

1. **Iterate** through the `Parser`.
2. When you hit `Event::Start(Tag::CodeBlock(kind))`, set a flag `is_in_code = true` and store the language.
3. **Buffer** all `Event::Text` content into a `String` until you hit `Event::End(Tag::CodeBlock)`.
4. **Process** that buffered string through the functions above.
5. **Emit** the resulting string into your final output.

### Essential CSS for HTML

To make the HTML output look professional, add this to your stylesheet:

```css
.ln {
    color: #666;
    padding-right: 15px;
    text-align: right;
    user-select: none; /* Prevents line numbers from being copied */
    border-right: 1px solid #444;
    margin-right: 10px;
}
table { border-collapse: collapse; }
td { padding: 0 5px; }

```

## Example

Integrating this into `pulldown-cmark` requires a "stateful" loop. Since a single code block can be emitted as multiple `Event::Text` chunks, you shouldn't process them until you hit the closing tag.

### The Integration Pattern

You can create a custom function that consumes the parser and returns the final string. This ensures that while standard Markdown (headers, lists) is handled normally, code blocks are intercepted for your custom rendering.

```rust
use pulldown_cmark::{Event, Parser, Tag, CodeBlockKind};

pub fn markdown_to_html_with_line_numbers(markdown_input: &str) -> String {
    let parser = Parser::new(markdown_input);
    let mut result = String::new();
    
    // State tracking for code blocks
    let mut in_code_block = false;
    let mut code_buffer = String::new();
    let mut current_lang = String::new();

    for event in parser {
        match event {
            Event::Start(Tag::CodeBlock(kind)) => {
                in_code_block = true;
                if let CodeBlockKind::Fenced(lang) = kind {
                    current_lang = lang.to_string();
                }
            }
            Event::End(Tag::CodeBlock(_)) => {
                in_code_block = false;
                // Now process the fully captured block
                let highlighted = render_html_with_line_numbers(&code_buffer, &current_lang);
                result.push_str(&highlighted);
                
                code_buffer.clear();
                current_lang.clear();
            }
            Event::Text(text) => {
                if in_code_block {
                    code_buffer.push_str(&text);
                } else {
                    result.push_str(&text);
                }
            }
            // Pass all other events (headers, bold, etc.) to a standard renderer or push manually
            _ => { 
                // Note: For a real app, use pulldown_cmark::html::push_html
                // or handle other Tags here.
            }
        }
    }
    result
}

```

### Why this approach is necessary

1. **State Management**: `syntect` is a stateful highlighter. If you try to highlight each `Event::Text` fragment individually, the highlighter will lose track of context (like whether it's currently inside a multi-line comment).
2. **Line Counting**: You can only accurately count lines once you have the full block.
3. **Terminal vs HTML**: By capturing the string in `code_buffer`, you can easily swap the `render` function depending on your output target (e.g., using `render_terminal_with_line_numbers` instead).

### Pro-Tip for Terminal: Handling Width

When rendering for the terminal, you might encounter issues if the code block is wider than the terminal window. You can use the `terminal_size` crate to get the width and use `textwrap` to wrap the code *before* passing it to `syntect`, or simply let the terminal handle the overflow (which usually breaks the line number alignment).

Most developers prefer to let the terminal scroll horizontally or truncate, as wrapping code manually often breaks the syntax highlighting logic of the next line.

## Getting `two-face` and `syntect` aligned and streamlined

To get `two-face` working with `syntect`, you need to understand that `two-face` provides the **themes** (the colors), while `syntect` provides the **syntax definitions** (the logic that understands what a "keyword" is).

### 1. Initializing the Assets

You should initialize these once (e.g., in a `lazy_static` or a `OnceCell`) because loading them is computationally expensive.

```rust
use syntect::parsing::SyntaxSet;
use syntect::highlighting::ThemeSet;

// Load Syntect's default syntaxes
let syntax_set = SyntaxSet::load_defaults_newlines();

// Load Two-Face's extra themes
// This includes "Monokai Pro", "GitHub Dark", "One Dark", etc.
let theme_set = two_face::theme_set(); 

// Choose a theme by name
let theme = &theme_set.themes["Monokai Pro"];

```

---

### 2. The Integrated Event Loop (Complete Pattern)

Here is how you combine `pulldown-cmark`, `syntect`, and `two-face` into a single logic flow. Note how we handle the `Event` stream to ensure standard Markdown text is still rendered as HTML while code blocks are intercepted.

```rust
use pulldown_cmark::{html, Event, Options, Parser, Tag, CodeBlockKind};

pub fn render_markdown(markdown_input: &str) -> String {
    let mut options = Options::empty();
    options.insert(Options::ENABLE_TABLES);
    
    let parser = Parser::new_with_options(markdown_input, options);
    let mut output = String::new();
    
    // Setup Syntect/Two-Face
    let ss = SyntaxSet::load_defaults_newlines();
    let ts = two_face::theme_set();
    let theme = &ts.themes["Monokai Pro"];

    let mut in_code_block = false;
    let mut code_buffer = String::new();
    let mut lang_id = String::new();

    // Custom loop to intercept events
    let events = parser.map(|event| {
        match event {
            Event::Start(Tag::CodeBlock(kind)) => {
                in_code_block = true;
                if let CodeBlockKind::Fenced(lang) = kind {
                    lang_id = lang.to_string();
                }
                Event::Text("".into()) // Suppress the start tag
            }
            Event::End(Tag::CodeBlock(_)) => {
                in_code_block = false;
                // Highlight the whole block at once
                let html = render_html_with_line_numbers(&code_buffer, &lang_id, &ss, theme);
                code_buffer.clear();
                Event::Html(html.into()) // Inject our custom HTML
            }
            Event::Text(t) => {
                if in_code_block {
                    code_buffer.push_str(&t);
                    Event::Text("".into()) // Buffer it, don't emit yet
                } else {
                    Event::Text(t) // Regular text
                }
            }
            _ => event, // Pass through headers, links, etc.
        }
    });

    // Use pulldown_cmark's built-in HTML generator for everything else
    html::push_html(&mut output, events);
    output
}

```

---

### 3. Visualizing the Data Flow

The complexity here is that `pulldown-cmark` is a "stream of events," while `syntect` needs a "buffer of lines."

### 4. Important: Handling "Ghost" Newlines

One common bug is that `code_buffer` often ends with a trailing newline that `pulldown-cmark` includes. When you use `.lines()` in your highlighter, this can result in an extra empty line at the bottom with a line number.

To fix this, you can use `.trim_end_matches('\n')` on your `code_buffer` before processing:

```rust
let clean_code = code_buffer.trim_end_matches('\n');
for (i, line) in clean_code.lines().enumerate() { ... }

```

### Summary of Benefits

* **Accessibility:** Using `user-select: none` in CSS ensures your line numbers don't annoy users trying to copy-paste code.
* **Aesthetics:** `two-face` provides modern themes that look much better than the `syntect` defaults.
* **Performance:** By buffering only code blocks and using the iterator mapping, you keep the memory footprint low even for large documents.

## Production Ready CSS

To make your code blocks look like a professional IDE (like VS Code or IntelliJ), you need a CSS structure that handles the "gutter" (the area containing line numbers) separately from the code content.

### 1. The CSS (VS Code Style)

This styling ensures that line numbers are perfectly aligned, vertically separated, and—most importantly—**not selectable** when a user highlights the code.

```css
/* Container for the code block */
pre.code-block {
    background-color: #1e1e1e; /* Match your Two-Face theme bg */
    border-radius: 6px;
    padding: 10px 0;
    overflow-x: auto;
    font-family: 'Fira Code', 'Cascadia Code', monospace;
    font-size: 14px;
    line-height: 1.5;
}

/* Table layout for alignment */
.code-table {
    border-collapse: collapse;
    width: 100%;
}

/* The Line Number Gutter */
.ln-gutter {
    color: #858585;
    text-align: right;
    padding: 0 15px 0 10px;
    width: 40px;
    user-select: none; /* Critical: Numbers won't be copied */
    border-right: 1px solid #404040;
    vertical-align: top;
}

/* The Code Content */
.code-content {
    padding-left: 15px;
    white-space: pre;
    vertical-align: top;
}

/* Optional: Highlight row on hover */
.code-table tr:hover {
    background-color: #2a2d2e;
}

```

---

### 2. Final Logic Polish

When you assemble the HTML in your Rust loop, ensure you use the classes defined above.

```rust
fn render_html_with_line_numbers(code: &str, lang: &str, ss: &SyntaxSet, theme: &Theme) -> String {
    let syntax = ss.find_syntax_by_token(lang).unwrap_or_else(|| ss.find_syntax_plain_text());
    let mut h = HighlightLines::new(syntax, theme);
    
    let mut html = String::from("<pre class=\"code-block\"><table class=\"code-table\">");

    // Use lines() but be mindful of the trailing newline from pulldown-cmark
    for (i, line) in code.trim_end_matches('\n').lines().enumerate() {
        let regions = h.highlight_line(line, ss).unwrap();
        let highlighted = styled_line_to_highlighted_html(&regions[..], IncludeBackground::No).unwrap();
        
        html.push_str("<tr>");
        html.push_str(&format!("<td class=\"ln-gutter\">{}</td>", i + 1));
        html.push_str(&format!("<td class=\"code-content\">{}</td>", highlighted));
        html.push_str("</tr>");
    }

    html.push_str("</table></pre>");
    html
}

```

---

### 3. Terminal Refinement

For the terminal, you don't have CSS, so you rely on **ANSI RGB sequences** to mimic the look. Use a subtle gray for the line numbers so they don't distract from the code.

```rust
// ANSI Escape for hex #858585 (Grey)
let grey = "\x1b[38;2;133;133;133m";
let reset = "\x1b[0m";
let gutter_border = "\x1b[38;2;64;64;64m│";

println!(" {}{:3} {} {}{}", grey, i + 1, gutter_border, escaped, reset);

```

## The Highlighter Struct

Managing `SyntaxSet` and `ThemeSet` in a struct is the best way to handle performance. These sets are large (the default `SyntaxSet` can be over 200KB of serialized data), and parsing them on every request or file render will significantly slow down your application.

By using a `Highlighter` struct, you load the assets once and then pass references during the rendering loop.

### 1. The Highlighter Struct

This struct encapsulates `syntect` and `two-face`, providing a clean interface for your Markdown parser.

```rust
use syntect::parsing::SyntaxSet;
use syntect::highlighting::{Theme, ThemeSet};
use two_face;

pub struct CodeHighlighter {
    pub syntax_set: SyntaxSet,
    pub theme: Theme,
}

impl CodeHighlighter {
    pub fn new(theme_name: &str) -> Self {
        let syntax_set = SyntaxSet::load_defaults_newlines();
        let theme_set = two_face::theme_set();
        
        // Fallback to a default theme if the name is not found
        let theme = theme_set
            .themes
            .get(theme_name)
            .cloned()
            .unwrap_or_else(|| theme_set.themes["Monokai Pro"].clone());

        Self { syntax_set, theme }
    }
}

```

### 2. Implementation with the Event Loop

Now you can pass this struct into a function that handles your `pulldown-cmark` events. This keeps your logic decoupled from your assets.

```rust
use pulldown_cmark::{html, Event, Parser, Tag, CodeBlockKind};

pub fn render_markdown_to_html(markdown: &str, highlighter: &CodeHighlighter) -> String {
    let parser = Parser::new(markdown);
    let mut output = String::new();
    
    let mut in_code_block = false;
    let mut code_buffer = String::new();
    let mut lang_id = String::new();

    let events = parser.map(|event| {
        match event {
            Event::Start(Tag::CodeBlock(kind)) => {
                in_code_block = true;
                if let CodeBlockKind::Fenced(lang) = kind {
                    lang_id = lang.to_string();
                }
                Event::Text("".into())
            }
            Event::End(Tag::CodeBlock(_)) => {
                in_code_block = false;
                // Use our struct's assets here
                let html = render_html_with_line_numbers(
                    &code_buffer, 
                    &lang_id, 
                    &highlighter.syntax_set, 
                    &highlighter.theme
                );
                code_buffer.clear();
                Event::Html(html.into())
            }
            Event::Text(t) => {
                if in_code_block {
                    code_buffer.push_str(&t);
                    Event::Text("".into())
                } else {
                    Event::Text(t)
                }
            }
            _ => event,
        }
    });

    html::push_html(&mut output, events);
    output
}

```

### 3. Usage Pattern

In a real-world application (like a web server or a CLI tool), you would initialize the highlighter once at startup:

```rust
fn main() {
    // Initialize once
    let highlighter = CodeHighlighter::new("GitHub Dark");

    let markdown = "```rust\nfn main() {\n    println!(\"Hello World\");\n}\n```";
    
    // Use many times
    let html_output = render_markdown_to_html(markdown, &highlighter);
    
    println!("{}", html_output);
}

```

### Performance Benchmarks to Keep in Mind

* **Asset Loading:** ~50–100ms (One-time cost).
* **Highlighting (Small block):** ~1–2ms.
* **Highlighting (Large file):** Scalable, but stateful line-by-line processing is much faster than re-parsing the entire block for every line.

### Final "Gotcha" on Memory

If you are building a CLI tool and concerned about binary size, `syntect`'s default `SyntaxSet` includes many languages you might not need. You can use `SyntaxSetBuilder` to include only specific `.sublime-syntax` files, but for most use cases, the defaults provided by `two-face` and `syntect` are the standard.

## Unit Tests

Testing this logic is crucial, especially to ensure that multi-line strings or comments don't "leak" or break the line numbering logic. You want to verify that `syntect` maintains its internal state across the `lines()` iterator and that your `gutter` count matches the actual line count.

### 1. The Unit Test

This test checks a few edge cases: a multi-line string (to check state) and an empty trailing line.

```rust
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_line_number_integrity() {
        let highlighter = CodeHighlighter::new("Monokai Pro");
        let code = "fn main() {\n    let s = \"multi-line\n    string\";\n}";
        
        // We expect 4 lines of output
        let html = render_html_with_line_numbers(
            code, 
            "rust", 
            &highlighter.syntax_set, 
            &highlighter.theme
        );

        // Assert that we see the line number indicators in the HTML
        assert!(html.contains("<td class=\"ln-gutter\">1</td>"));
        assert!(html.contains("<td class=\"ln-gutter\">2</td>"));
        assert!(html.contains("<td class=\"ln-gutter\">3</td>"));
        assert!(html.contains("<td class=\"ln-gutter\">4</td>"));
        
        // Ensure we haven't created a 5th empty line
        assert!(!html.contains("<td class=\"ln-gutter\">5</td>"));
    }
}

```

### 2. Handling the "Missing Newline" State

In your `render_html_with_line_numbers` function, there is a subtle detail: `syntect` works best when lines include their trailing `\n`. If you use `.lines()`, the newline is stripped.

To ensure syntax like multi-line comments (`/* ... */`) highlights correctly on subsequent lines, you should append a newline to the string you pass to `highlight_line`.

**Updated line-processing logic:**

```rust
for (i, line) in code.trim_end_matches('\n').lines().enumerate() {
    // Re-add the newline so syntect knows the line ended (important for state)
    let line_for_syntect = format!("{}\n", line); 
    let regions = h.highlight_line(&line_for_syntect, ss).unwrap();
    
    // Convert to HTML (IncludeBackground::No prevents nested background colors)
    let highlighted = styled_line_to_highlighted_html(&regions[..], IncludeBackground::No).unwrap();
    
    // ... push to table row ...
}

```

### 3. Summary of the Architecture

| Component | Responsibility |
| --- | --- |
| **`pulldown-cmark`** | Breaks Markdown into events; identifies code blocks. |
| **`CodeHighlighter`** | Holds the heavy `SyntaxSet` and `two-face` theme data in memory. |
| **`HighlightLines`** | A stateful object that tracks syntax context line-by-line. |
| **The HTML Table** | Separates the "Gutter" (numbers) from "Content" (code) for clean UI. |

---

### Final Implementation Checklist

1. **Don't reload assets:** Ensure `CodeHighlighter` is initialized once.
2. **CSS `user-select`:** Double-check that your CSS prevents line numbers from being copied.
3. **Language Fallback:** Use `ss.find_syntax_plain_text()` if a user provides an unknown language tag (like ````random`).
4. **Terminal Colors:** If using the terminal, ensure you use `as_24_bit_terminal_escaped` for the best color fidelity from `two-face`.


