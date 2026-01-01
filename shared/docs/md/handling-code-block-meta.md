# Code Block Meta

A fenced code block in a Markdown file is started with the triple backtick and then you're supposed to add the _language_ for the code block afterward. However, since we're using `pulldown-cmark` to parse our Markdown docs we will have access not only to the language but whatever else is on that line.

So for example:

~~~md
This is an example.

```ts
function greet() {
    console.log("hi");
    console.log("there");
}
```
~~~

- this example is fairly typical for a fenced code block and by responding to the `Tag::CodeBlock` event (see [all events](./pulldown-cmark-events.md)) we can gain access to the entire line.

Let's instead evaluate a similar code block but one which does _more_ with the HEAD line of the code block:

~~~
This is an example.

```ts title=Greet Function line-numbers=true highlight=1
function greet() {
    console.log("hi");
    console.log("there")
}
```
~~~

Now we have the language `ts` followed by `title=Greet Function line-numbers=true highlight=1`. By default this will be "available" but will be ignored in a render to the terminal or to HTML but it doesn't have to ignored.

To give a name to the text _after_ the language specifier, we will refer to this area as the **fenced metadata**.

## How to Leverage the Fenced Metadata

### How `pulldown-cmark` Handles the Info String

When the parser encounters a fenced code block, it emits a `Start(Tag::CodeBlock(kind))` event. The `kind` is an enum of type `CodeBlockKind`.

- If it's a fenced block, it contains a `CowStr`.
- This string contains everything from the first non-whitespace character after the backticks to the end of that line.

### Preserving Cows During Accumulation Phase

> Markdown text is a `CowStr` in `pulldown-cmark` and `push_str` is the standard way to "collect" it into the final buffer required by syntect.

While `pulldown-cmark` is very efficient with `CowStr`, `syntect` functions generally require a `&str`. Because a code block in Markdown is often split into multiple `Event::Text` chunks by the parser, you must join them into a single contiguous String buffer before highlighting.

However, you can make your document more "Rust-idiomatic" by handling the `CowStr` efficiently during the accumulation phase.

> **Note on String Memory**
> 
> `pulldown-cmark` provides code content via `Event::Text(CowStr)`. While it's tempting to try and avoid allocations, `syntect`'s highlighting engines require a single, contiguous string slice (&str).
> 
> Therefore, we accumulate the parts into a single `String`. To optimize, we can initialize the code_accumulator with a capacity estimate if the document is large, reducing re-allocations: `let mut code_accumulator = String::with_capacity(1024);`

In Code Example 1 we demonstrate an effective means of preserving the `CoeStr` type.

### Understanding the "Newline" Risk

If you simply push every text event, you are usually safe, but some logic can accidentally trim the final newline of a code block.

`syntect`'s HTML generator handles the trailing newline to determine where the last </span> or </div> goes. If your accumulator trims the code, the highlighted background might look "cut off" on the last line.

### Effective Variable Naming in the Fenced Metadata

The standard convention is to use kebab-case variable naming.

> Note: because Rust loves snake_case, just remember that you can use the `serde(rename_all = "snake_case" )` directive when you need to.


## Code Example 1

### Implementation Example

This example demonstrates how to extract the metadata, use `two-face` (via `syntect`) to highlight the code, and bundle it all together into a formatted string.

### Integrated Highlighting Example

This code uses `two-face` for the syntax/theme definitions and `pulldown-cmark` to drive the parsing.

```rust
use pulldown_cmark::{Parser, Event, Tag, CodeBlockKind};
use syntect::html::highlighted_html_for_string;
use two_face::syntax_set;
use two_face::theme_set;

fn render_markdown_with_metadata(markdown: &str) -> String {
    let mut html_output = String::new();
    let p = Parser::new(markdown);
    
    // Load syntaxes and themes from two-face
    let syn_set = syntax_set();
    let thm_set = theme_set();
    let theme = &thm_set.themes["base16-ocean.dark"];

    let mut in_code_block = false;
    let mut current_lang = String::new();
    let mut current_metadata = String::new();
    let mut code_accumulator = String::new();

    for event in p {
        match event {
            Event::Start(Tag::CodeBlock(kind)) => {
                in_code_block = true;
                match kind {
                    CodeBlockKind::Fenced(info_string) => {
                        let (lang, meta) = parse_code_info(&info_string);
                        current_lang = lang;
                        current_metadata = meta;
                    }
                    CodeBlockKind::Indented => {
                        current_lang = "text".to_string(); // Fallback for indented blocks
                    }
                }
            }
            Event::Text(text) if in_code_block => {
                // text is a CowStr; push_str is efficient here as it 
                // dereferences the Cow to a &str automatically.
                code_accumulator.push_str(&text);
            }
            Event::End(Tag::CodeBlock(_)) => {
                in_code_block = false;

                // Ensure we don't have leading/trailing whitespace 
                // that wasn't in the original code block
                let code_to_highlight = &code_accumulator;
                
                // Get the syntax for the language, fallback to plain text
                let syntax = syn_set
                    .find_syntax_by_token(&current_lang)
                    .unwrap_or_else(|| syn_set.find_syntax_plain_text());

                // Perform highlighting
                let highlighted_code = highlighted_html_for_string(
                    &code_accumulator,
                    &syn_set,
                    syntax,
                    theme,
                ).unwrap_or_else(|_| code_accumulator.clone());

                // Build custom HTML structure
                html_output.push_str("<div class='code-container'>");
                if !current_metadata.is_empty() {
                    html_output.push_str(&format!("<div class='code-title'>{}</div>", current_metadata));
                }
                html_output.push_str(&highlighted_code);
                html_output.push_str("</div>");

                // Reset buffers
                code_accumulator.clear();
            }
            // Handle standard Markdown (non-code block events)
            _ => {
                if !in_code_block {
                    // You'd typically use pulldown_cmark::html::push_html here
                }
            }
        }
    }
    html_output
}

```

---

### Key Takeaways for the Workflow

- **The Info String:** `pulldown-cmark` considers everything after the backticks (until the end of the line) as the `CodeBlockKind::Fenced` value. It does not try to be "smart" and split the language from the description—that is left to you.
- **The `split_once` Pattern:** Using `.split_once(' ')` is the cleanest way to handle this. If there is no space, it returns `None`, meaning there is only a language tag (or no tag at all).
- **The Accumulator:** Because `pulldown-cmark` emits code contents as one or more `Event::Text` events, you must collect them into a buffer string before passing them to `syntect` for highlighting.

### Styling Suggestion

To make your "Greeting Function" text look professional, you can use CSS to style that metadata div:

```css
.code-container {
    border: 1px solid #333;
    border-radius: 4px;
    overflow: hidden;
}
.code-title {
    background: #222;
    color: #eee;
    padding: 4px 12px;
    font-size: 0.85em;
    font-family: sans-serif;
    border-bottom: 1px solid #333;
}
```

## Code Example 2

If you want to handle more complex metadata (like `ts {1, 4-6} title="Main.ts"`), a simple space split won't be enough because the metadata itself might contain spaces within quotes.

To handle this properly, you can use a small **Regex** or a **Scanner** pattern to separate the language from the "attribute string," and then parse those attributes.

### Handling Key-Value Pairs

Here is how you can modify the logic to handle structured metadata:

```rust
use lazy_static::lazy_static;
use regex::Regex;
use std::collections::HashMap;

lazy_static! {
    /// Pre-compiled regex to find patterns like key="value", key='value', or key=value.
    /// It captures the key in group 1 and the value in group 2 or 3.
    static ref METADATA_RE: Regex = Regex::new(r#"(\w+)=(?:"([^"]*)"|'([^']*)'|(\S+))"#)
        .expect("Failed to compile metadata regex");
}

fn parse_code_info(info_string: &str) -> (String, HashMap<String, String>) {
    let mut parts = info_string.splitn(2, ' ');
    let lang = parts.next().unwrap_or("").to_string();
    let mut attrs = HashMap::new();

    if let Some(remainder) = parts.next() {
        // Use the pre-compiled static regex
        for cap in METADATA_RE.captures_iter(remainder) {
            let key = cap[1].to_string();
            // Value could be in group 2 (quoted) or group 3 (unquoted)
            let value = cap.get(2).map(|m| m.as_str()).unwrap_or(&cap[0]).to_string();
            attrs.insert(key, value);
        }
        
        // Fallback: If no key-value pairs were found, treat the whole string as a title
        if attrs.is_empty() && !remainder.is_empty() {
            attrs.insert("title".to_string(), remainder.trim().to_string());
        }
    }

    (lang, attrs)
}
```

### Using the Parsed Metadata

In your rendering loop, you can now use this hashmap to apply specific logic, such as highlighting specific lines or adding a "Copy" button with a filename.

```rust
// Inside your Event::Start(Tag::CodeBlock) match:
let (lang, metadata) = parse_code_info(&info_string);

if let Some(title) = metadata.get("title") {
    html_output.push_str(&format!("<div class='code-header'>{}</div>", title));
}

if let Some(lines) = metadata.get("highlight") {
    // Logic to pass line-highlighting instructions to your renderer
}

```

### Advanced: Integration with `two-face` Themes

If you are using `two-face`, you might want the background color of your "Title Bar" to match the theme you've selected from the `ThemeSet`.

You can access the theme colors directly:

```rust
let theme = &thm_set.themes["base16-ocean.dark"];
let bg_color = theme.settings.background.unwrap_or(syntect::highlighting::Color::BLACK);

// Convert syntect Color to CSS rgba or hex
let css_bg = format!("background-color: #:{:02x}{:02x}{:02x};", bg_color.r, bg_color.g, bg_color.b);

```

### Important Considerations

1. **Performance:** If you are processing large Markdown files, pre-compile your `Regex` using `lazy_static` or `once_cell` so you don't re-compile it for every code block.
2. **Safety:** `pulldown-cmark` provides `escape_html`. Always wrap your metadata string in an escape function before pushing it to your HTML output to prevent XSS if you're rendering user-provided Markdown.
3. Whitespace and Newlines: `syntect`'s `highlighted_html_for_string` expects the full block of code. Ensure that your code_accumulator doesn't accidentally strip or add trailing newlines, as this can affect how line-numbering logic (if added later) aligns with the background.


