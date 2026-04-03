# Markdown Rendering

Integrate Markdown rendering into Ratatui applications using `pulldown-cmark` for parsing and `tui-markdown` for rendering.

## Using tui-markdown (Recommended)

The simplest approach:

```toml
[dependencies]
ratatui = "0.29"
tui-markdown = "0.3"
pulldown-cmark = "0.13"
```

```rust
use tui_markdown::from_str;
use ratatui::text::Text;

fn render_markdown(frame: &mut Frame, markdown: &str, area: Rect) {
    let text_widget: Text = from_str(markdown);
    frame.render_widget(text_widget, area);
}
```

## Features

- **Syntax highlighting**: Enabled by default via `syntect`
- **Headings, lists, tables**: Full CommonMark support
- **Code blocks**: With language-specific highlighting
- **Links**: Rendered with underline and blue color

## Disable Syntax Highlighting

```toml
[dependencies.tui-markdown]
version = "0.3"
default-features = false
```

## Direct Integration with pulldown-cmark

For custom control:

```rust
use pulldown_cmark::{Event, Options, Parser, Tag, TagEnd};
use ratatui::text::{Line, Span, Text};
use ratatui::style::{Color, Modifier, Style};

fn parse_markdown(markdown: &str) -> Text<'static> {
    let mut options = Options::empty();
    options.insert(Options::ENABLE_STRIKETHROUGH);
    options.insert(Options::ENABLE_TABLES);

    let parser = Parser::new_ext(markdown, options);
    let mut lines = Vec::new();
    let mut current_line_spans = Vec::new();
    let mut style_stack: Vec<Style> = vec![Style::default()];

    for event in parser {
        match event {
            Event::Start(tag) => {
                let mut current_style = *style_stack.last().unwrap();
                match tag {
                    Tag::Strong => {
                        current_style = current_style.add_modifier(Modifier::BOLD);
                    }
                    Tag::Emphasis => {
                        current_style = current_style.add_modifier(Modifier::ITALIC);
                    }
                    Tag::Heading { .. } => {
                        current_style = current_style
                            .add_modifier(Modifier::BOLD)
                            .fg(Color::Blue);
                    }
                    Tag::Link { .. } => {
                        current_style = current_style
                            .fg(Color::Cyan)
                            .add_modifier(Modifier::UNDERLINED);
                    }
                    _ => {}
                }
                style_stack.push(current_style);
            }
            Event::End(tag_end) => {
                style_stack.pop();
                match tag_end {
                    TagEnd::Paragraph | TagEnd::Heading(_) => {
                        if !current_line_spans.is_empty() {
                            lines.push(Line::from(current_line_spans.drain(..).collect::<Vec<_>>()));
                        }
                        lines.push(Line::from(""));  // Spacing
                    }
                    _ => {}
                }
            }
            Event::Text(text) => {
                let style = *style_stack.last().unwrap();
                current_line_spans.push(Span::styled(text.to_string(), style));
            }
            Event::Code(code) => {
                let code_style = Style::default()
                    .fg(Color::Yellow)
                    .bg(Color::Rgb(40, 44, 52));
                current_line_spans.push(Span::styled(format!(" {} ", code), code_style));
            }
            Event::SoftBreak | Event::HardBreak => {
                lines.push(Line::from(current_line_spans.drain(..).collect::<Vec<_>>()));
            }
            _ => {}
        }
    }

    if !current_line_spans.is_empty() {
        lines.push(Line::from(current_line_spans));
    }

    Text::from(lines)
}
```

## Chat Application Pattern

Cache parsed Markdown to avoid re-parsing every frame:

```rust
struct ChatMessage {
    raw_content: String,
    styled_content: Text<'static>,
    is_user: bool,
}

impl ChatMessage {
    fn new(author: &str, content: &str, is_user: bool) -> Self {
        Self {
            raw_content: content.to_string(),
            styled_content: parse_markdown(content),
            is_user,
        }
    }
}

// In render loop
let paragraph = Paragraph::new(message.styled_content.clone())
    .block(Block::default().borders(Borders::ALL))
    .wrap(Wrap { trim: true });
```

## Code Block Highlighting

For custom syntax highlighting with `syntect`:

```rust
use syntect::{easy::HighlightLines, parsing::SyntaxSet, highlighting::ThemeSet};

// Setup (outside render loop)
let ps = SyntaxSet::load_defaults_newlines();
let ts = ThemeSet::load_defaults();
let theme = &ts.themes["base16-ocean.dark"];

// When processing code block
let syntax = ps.find_syntax_by_extension("rs").unwrap();
let mut highlighter = HighlightLines::new(syntax, theme);

for line in code.lines() {
    let ranges = highlighter.highlight_line(line, &ps)?;
    // Convert ranges to Ratatui Spans
}
```

## Common Issues

### List Items Wrapping

**Problem**: Styled list items may start on a new line in `tui-markdown`

**Solution**: Avoid complex styling within list items or use direct integration

### Performance with Large Documents

**Problem**: Parsing Markdown on every frame is expensive

**Solution**: Cache the parsed `Text` and only re-parse when content changes

```rust
struct Document {
    raw: String,
    cached: Option<Text<'static>>,
}

impl Document {
    fn get_text(&mut self) -> &Text<'static> {
        if self.cached.is_none() {
            self.cached = Some(parse_markdown(&self.raw));
        }
        self.cached.as_ref().unwrap()
    }

    fn update(&mut self, new_content: String) {
        self.raw = new_content;
        self.cached = None;  // Invalidate cache
    }
}
```

## Best Practices

1. **Use tui-markdown for standard needs** - Saves implementation time
2. **Cache parsed output** - Parse once, render many times
3. **Handle code blocks specially** - Provide syntax highlighting
4. **Test across terminals** - Some terminals have limited Unicode support
5. **Provide plain text fallback** - For terminals without color support
