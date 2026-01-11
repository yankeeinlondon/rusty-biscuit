# Highlighting the Prose

When we talk about _highlighting_ in Markdown pipelining we tend to spend most of our time discussing the fenced code blocks which are interspersed throughout the body of the Markdown content. However, the markdown document itself is a valid language grammar and ideally it too should be styled.

Unfortunately it presents us with a classic architectural challenge in building Markdown renderers. You have two distinct parsing paradigms colliding:

1. **Structural Parsing (`pulldown-cmark`):** Converts text into an Abstract Syntax Tree (AST) of events (Header, Paragraph, Bold). It strips away the syntax markers (like `**` or `#`).
2. **Lexical Parsing (`syntect`):** Uses regex to match patterns in raw text strings. It preserves everything and relies on exact character matching.

Since you are already using `pulldown-cmark` for the pipeline, **you should not run `syntect`'s Markdown grammar parser on the prose.** Doing so would be redundant (double-parsing), slow, and difficult to synchronize with your `pulldown` event loop.

Instead, the most performant and robust solution is a **Scope-Mapping Approach**. You will map `pulldown-cmark` events to `syntect` scopes, then ask the Theme for the color of that scope.

Here is the design for a high-performance solution.

## The Solution: Event-to-Scope Mapping

We will create a "Virtual Highlighter". As we traverse the `pulldown-cmark` events, we maintain a stack of `syntect` Scopes (e.g., `markup.bold.markdown`). For every piece of text, we ask `syntect` to calculate the style based on the current stack.

**The Tradeoff:**

* **Pro:** Extremely fast. You piggyback on the parsing `pulldown` is already doing.
* **Con:** You highlight the _content_ (the text inside the bold), not the _syntax markers_ (the `**` asterisks), because `pulldown` consumes the markers. This is usually preferred for reading prose anyway.

### Implementation Design

#### 1. The Dependencies

You need `syntect` for the styling engine and `two-face` for the bundled themes.

```toml
[dependencies]
pulldown-cmark = "0.9" # or 0.10
syntect = "5.0"
two-face = "0.3" # or whatever version you are using
# For terminal colors (optional, but helpful for ANSI generation)
console = "0.15" 

```

#### 2. The Scope Mapper

We need to map `pulldown` Tags to `syntect` Scope strings. We pre-parse these into `Scope` objects for performance so we aren't parsing strings inside the render loop.

#### 3. The Render Loop

Here is a simplified, high-performance implementation structure.

```rust
use pulldown_cmark::{Event, Parser, Tag};
use syntect::highlighting::{Highlighter, Style, Theme, ThemeSet};
use syntect::parsing::{Scope, ScopeStack};
use syntect::easy::HighlightLines;
use syntect::util::LinesWithEndings;
use std::borrow::Cow;

pub struct MarkdownRenderer<'a> {
    prose_theme: &'a Theme,
    code_theme: &'a Theme,
    syntax_set: &'a syntect::parsing::SyntaxSet,
}

impl<'a> MarkdownRenderer<'a> {
    pub fn new(
        prose_theme: &'a Theme,
        code_theme: &'a Theme,
        syntax_set: &'a syntect::parsing::SyntaxSet,
    ) -> Self {
        Self {
            prose_theme,
            code_theme,
            syntax_set,
        }
    }

    pub fn render(&self, markdown_input: &str) {
        let parser = Parser::new(markdown_input);
        
        // 1. State Management
        // We manually maintain a stack of scopes to simulate what syntect 
        // would do if it parsed the grammar itself.
        let mut scope_stack = ScopeStack::new();
        
        // Push the base scope for the document
        scope_stack.push(Scope::new("text.html.markdown").unwrap());

        // Create a highlighter for the prose theme
        let prose_highlighter = Highlighter::new(self.prose_theme);

        // State for handling code blocks specifically
        let mut in_code_block = false;
        let mut code_block_lang = String::new();
        let mut code_buffer = String::new();

        for event in parser {
            match event {
                // --- PROSE STYLING ---
                Event::Start(tag) => {
                    if let Tag::CodeBlock(kind) = tag {
                        in_code_block = true;
                        code_block_lang = match kind {
                            pulldown_cmark::CodeBlockKind::Fenced(lang) => lang.to_string(),
                            _ => String::new(),
                        };
                        continue;
                    }

                    // Map pulldown tags to syntect scopes
                    if let Some(scope) = self.map_tag_to_scope(&tag) {
                        scope_stack.push(scope);
                    }
                }
                
                Event::End(tag) => {
                    if let Tag::CodeBlock(_) = tag {
                        in_code_block = false;
                        self.render_code_block(&code_buffer, &code_block_lang);
                        code_buffer.clear();
                        continue;
                    }

                    // Pop the scope if we pushed one for this tag
                    if self.map_tag_to_scope(&tag).is_some() {
                         // ScopeStack::pop is slightly internal, we might just reduce stack size
                         // or re-build stack. Syntect ScopeStack is a wrapper around Vec.
                         // For simplicity here, assume we can pop. 
                         // In real syntect, you might need to use `stack.scopes.pop()`.
                         let _ = scope_stack.as_slice(); // access underlying if needed
                    }
                }

                Event::Text(text) => {
                    if in_code_block {
                        code_buffer.push_str(&text);
                    } else {
                        // THIS IS THE MAGIC:
                        // Ask the theme: "What color is this stack of scopes?"
                        let style = prose_highlighter.style_for_stack(scope_stack.as_slice());
                        
                        // Output the text with the calculated style
                        self.emit_styled_text(&text, style);
                    }
                }

                // Handle Inline Code (backticks)
                Event::Code(text) => {
                    // Push inline raw scope
                    let raw_scope = Scope::new("markup.raw.inline.markdown").unwrap();
                    let mut temp_stack = scope_stack.clone(); 
                    temp_stack.push(raw_scope);
                    
                    let style = prose_highlighter.style_for_stack(temp_stack.as_slice());
                    self.emit_styled_text(&text, style);
                }

                _ => {} // Handle other events (HTML, Footnotes, etc)
            }
        }
    }

    // Performance Note: 
    // In a production app, memoize/cache this mapping or use a match block 
    // that returns const Scope objects to avoid parsing strings every time.
    fn map_tag_to_scope(&self, tag: &Tag) -> Option<Scope> {
        match tag {
            Tag::Heading(..) => Some(Scope::new("markup.heading.markdown").unwrap()),
            Tag::Strong => Some(Scope::new("markup.bold.markdown").unwrap()),
            Tag::Emphasis => Some(Scope::new("markup.italic.markdown").unwrap()),
            Tag::BlockQuote => Some(Scope::new("markup.quote.markdown").unwrap()),
            Tag::Link(..) => Some(Scope::new("markup.underline.link.markdown").unwrap()),
            // Add List, Strikethrough, etc.
            _ => None,
        }
    }

    fn render_code_block(&self, content: &str, lang: &str) {
        // Here we switch to the Code Theme and the specific Language Grammar
        let syntax = self.syntax_set.find_syntax_by_token(lang)
            .unwrap_or_else(|| self.syntax_set.find_syntax_plain_text());
            
        let mut highlighter = HighlightLines::new(syntax, self.code_theme);
        
        // Standard Syntect Loop for the code block
        for line in LinesWithEndings::from(content) {
            let ranges: Vec<(Style, &str)> = highlighter.highlight_line(line, self.syntax_set).unwrap();
            for (style, text) in ranges {
                self.emit_styled_text(text, style);
            }
        }
    }

    fn emit_styled_text(&self, text: &str, style: Style) {
        // IMPLEMENTATION DEPENDS on Browser vs Terminal
        
        // For Terminal (pseudocode using console/crossterm logic):
        // let ansi_color = convert_syntect_color_to_ansi(style.foreground);
        // print!("{}", text.with_color(ansi_color));
        
        // For Browser (HTML):
        // print!("<span style='color: {:?}'>{}</span>", style.foreground, escape_html(text));
    }
}

```

### Key Considerations for "Performant" design

1. **Scope Parsing:** The `Scope::new("string")` call parses the string. Do **not** do this inside the loop (as I did in the example for brevity).

* _Optimization:_ Create a `struct ScopeCache` that holds the pre-parsed `Scope` atoms for Heading, Bold, Italic, etc., and pass that into your renderer.


1. **Stack Clone:** Cloning the stack for inline code (`Event::Code`) is cheap because `ScopeStack` is small (just a Vec of atoms), but minimize it where possible.
2. **Theme Selection:** You successfully satisfy the requirement of "different themes" by instantiating two different `Highlighter` objects—one used in the main loop (Prose), one used in the `render_code_block` function (Code).

### Dealing with "Markdown Grammar" Nuances

The user mentioned specifically using the **Markdown grammar**.
In `syntect`, the Markdown grammar defines scopes like:

* `meta.paragraph.markdown`
* `punctuation.definition.bold.markdown` (The `**` asterisks)

Since `pulldown-cmark` swallows the asterisks, you cannot style the punctuation using this method. If you absolutely **must** see colored asterisks in the terminal output, you have to abandon `pulldown-cmark` for rendering and use `syntect` directly on the raw file string.

However, based on your request, you likely want the **text content** to be styled according to the theme. The mapping above (`markup.bold.markdown`) will trigger the correct color in almost all standard TextMate/Sublime themes used by `syntect`.

### Handling Terminal vs Browser output

Since you need both, your `emit_styled_text` function should probably call into a trait:

```rust
trait OutputFormatter {
    fn write_text(&mut self, text: &str, style: Style);
}

struct TerminalFormatter; // Uses ANSI codes
struct HtmlFormatter;     // Uses <span style="...">

```

For the **Terminal**, you must convert `syntect::highlighting::Color` (RGBA) to ANSI.

* **TrueColor:** If the terminal supports it, emit `\x1b[38;2;R;G;Bm`.
* **256 Color:** You'll need a down-sampling algorithm if you want to be safe, or just rely on modern terminals supporting TrueColor (most do nowadays).


This "Yes" implies we should tackle both the performance optimization (`ScopeCache`) and the output handling (ANSI/HTML conversion) to make this a complete, production-ready solution.

Here is the complete implementation strategy broken down into those two critical components.

### Part 1: High-Performance Scope Caching

In `syntect`, `Scope::new("markup.bold.markdown")` parses the string every time it is called. Doing this inside a `for` loop over thousands of Markdown events is a performance killer. We will move this work to initialization.

```rust
use syntect::parsing::Scope;

/// Pre-parsed scopes to avoid overhead during the render loop.
pub struct ScopeCache {
    pub heading: Scope,
    pub bold: Scope,
    pub italic: Scope,
    pub quote: Scope,
    pub link: Scope,
    pub image: Scope,
    pub code_inline: Scope,
    pub list: Scope,
    pub base: Scope,
}

impl ScopeCache {
    pub fn new() -> Self {
        // We use .unwrap() here because these strings are hardcoded and known to be valid.
        // In a real app, you might use expect("Invalid hardcoded scope")
        Self {
            base: Scope::new("text.html.markdown").unwrap(),
            heading: Scope::new("markup.heading.markdown").unwrap(),
            bold: Scope::new("markup.bold.markdown").unwrap(),
            italic: Scope::new("markup.italic.markdown").unwrap(),
            quote: Scope::new("markup.quote.markdown").unwrap(),
            link: Scope::new("markup.underline.link.markdown").unwrap(),
            image: Scope::new("meta.image.inline.markdown").unwrap(),
            code_inline: Scope::new("markup.raw.inline.markdown").unwrap(),
            list: Scope::new("markup.list.markdown").unwrap(),
        }
    }
}

```

Now, update your renderer struct to hold this cache:

```rust
pub struct MarkdownRenderer<'a> {
    // ... themes ...
    scope_cache: ScopeCache, // Add this
}

```

And your mapping function becomes a fast lookup rather than a parsing operation:

```rust
// Inside MarkdownRenderer implementation
fn map_tag_to_scope(&self, tag: &Tag) -> Option<Scope> {
    match tag {
        Tag::Heading(..) => Some(self.scope_cache.heading),
        Tag::Strong => Some(self.scope_cache.bold),
        Tag::Emphasis => Some(self.scope_cache.italic),
        Tag::BlockQuote => Some(self.scope_cache.quote),
        Tag::Link(..) => Some(self.scope_cache.link),
        Tag::Image(..) => Some(self.scope_cache.image),
        Tag::List(_) => Some(self.scope_cache.list),
        _ => None,
    }
}

```

---

### Part 2: The Output Backend (Terminal & Browser)

To support both outputs efficiently, we define a trait. This allows you to swap backends without changing the complex parsing logic.

#### The Trait

```rust
use syntect::highlighting::Style;

pub trait OutputFormatter {
    fn write_text(&mut self, text: &str, style: Style);
    fn start_block(&mut self); // Useful for flushing buffers or setting block-level styles
    fn finish(&mut self);
}

```

#### A. The HTML Formatter (Browser)

This converts the `syntect` color (RGBA) into a CSS hex string or `rgb()` value.

```rust
use std::fmt::Write; // Allows writing to String

pub struct HtmlFormatter {
    output: String,
}

impl HtmlFormatter {
    pub fn new() -> Self { Self { output: String::new() } }
    
    fn color_to_css(c: syntect::highlighting::Color) -> String {
        // Syntect uses RGBA, usually with A=255. 
        // We can ignore Alpha for text unless you specifically need opacity.
        format!("#{:02x}{:02x}{:02x}", c.r, c.g, c.b)
    }
}

impl OutputFormatter for HtmlFormatter {
    fn start_block(&mut self) {}
    
    fn write_text(&mut self, text: &str, style: Style) {
        let css_color = Self::color_to_css(style.foreground);
        
        // Note: You must escape HTML entities in 'text' to prevent XSS/rendering errors!
        let escaped_text = html_escape::encode_text(text); 
        
        let _ = write!(
            self.output, 
            "<span style=\"color: {}\">{}</span>", 
            css_color, escaped_text
        );
    }

    fn finish(&mut self) {}
}

```

#### B. The Terminal Formatter (ANSI)

This is trickier. Modern terminals support "TrueColor" (24-bit), which matches the themes perfectly. If you need to support ancient terminals (standard 16-color or 256-color), you need complex down-sampling logic.

Here is the **TrueColor** implementation, which covers 99% of modern dev use cases (VS Code terminal, iTerm2, Alacritty, Windows Terminal).

```rust
pub struct TerminalFormatter {
    // We write directly to stdout or a buffer
}

impl OutputFormatter for TerminalFormatter {
    fn start_block(&mut self) {}

    fn write_text(&mut self, text: &str, style: Style) {
        let c = style.foreground;
        
        // ANSI Escape Sequence for Foreground Color:
        // \x1b[38;2;<r>;<g>;<b>m
        print!("\x1b[38;2;{};{};{}m{}", c.r, c.g, c.b, text);
    }

    fn finish(&mut self) {
        // Reset color at the end so your terminal doesn't stay weirdly colored
        print!("\x1b[0m"); 
    }
}

```

### Integration

Here is how you wire it into the `render` function from the previous step.

```rust
pub fn render<F: OutputFormatter>(&self, markdown_input: &str, formatter: &mut F) {
    let parser = Parser::new(markdown_input);
    let mut scope_stack = ScopeStack::new();
    scope_stack.push(self.scope_cache.base);

    let prose_highlighter = Highlighter::new(self.prose_theme);
    
    // ... (Code block state logic from previous answer) ...

    for event in parser {
        match event {
            // ... (Tag logic) ...

            Event::Text(text) => {
                if in_code_block {
                    // Accumulate code buffer...
                } else {
                    let style = prose_highlighter.style_for_stack(scope_stack.as_slice());
                    
                    // DELEGATE TO FORMATTER
                    formatter.write_text(&text, style);
                }
            }
            
            // ... (Rest of logic) ...
        }
    }
    formatter.finish();
}

```

### Usage Example

```rust
fn main() {
    // Load themes and syntax set (assumed loaded via two-face)
    let (prose_theme, code_theme, syntax_set) = load_assets(); 
    
    let renderer = MarkdownRenderer::new(&prose_theme, &code_theme, &syntax_set);
    let input = "# Hello World\nThis is **bold** text.";

    // Render for Terminal
    let mut term_fmt = TerminalFormatter {};
    renderer.render(input, &mut term_fmt);

    // Render for Browser
    let mut html_fmt = HtmlFormatter::new();
    renderer.render(input, &mut html_fmt);
    println!("\nHTML Output: {}", html_fmt.output);
}

```

### A Final Performance Tip: `Highlighter` Caching

You might be tempted to create `Highlighter::new(theme)` inside the render loop (or once per request).

* **The Problem:** Creating a `Highlighter` is relatively cheap, but it does some caching internally.
* **The Fix:** If you are processing thousands of documents in a batch job, keep the `Highlighter` instance alive alongside the renderer, rather than recreating it for every `render()` call.

