# Code Editor

The `ratatui-code-editor` crate provides a specialized widget for code editing with Tree-sitter syntax highlighting.

## Setup

```toml
[dependencies]
ratatui-code-editor = "0.3"
```

## Basic Usage

```rust
use ratatui_code_editor::{Editor, EditorState, SyntaxHighlighter};

struct App {
    editor_state: EditorState,
    highlighter: SyntaxHighlighter,
}

impl App {
    fn new() -> Self {
        let state = EditorState::new(
            "fn main() {\n    println!(\"Hello\");\n}".to_string()
        );
        let highlighter = SyntaxHighlighter::new("rust");

        Self { editor_state: state, highlighter }
    }

    fn draw(&mut self, frame: &mut Frame, area: Rect) {
        let editor = Editor::new(&mut self.editor_state)
            .highlight(Some(&self.highlighter))
            .theme(ratatui_code_editor::theme::vesper());

        frame.render_widget(editor, area);
    }

    fn handle_event(&mut self, event: &Event) {
        self.editor_state.on_event(event);
    }
}
```

## File I/O

```rust
use std::fs;
use std::path::PathBuf;

impl App {
    fn open_file(&mut self, path: PathBuf) -> std::io::Result<()> {
        let content = fs::read_to_string(&path)?;
        self.editor_state = EditorState::new(content);
        self.current_path = Some(path);
        self.is_modified = false;
        Ok(())
    }

    fn save_file(&mut self) -> std::io::Result<()> {
        if let Some(path) = &self.current_path {
            let content = self.editor_state.text().to_string();
            fs::write(path, content)?;
            self.is_modified = false;
        }
        Ok(())
    }
}
```

## Language Detection

```rust
fn get_language(path: &std::path::Path) -> &'static str {
    match path.extension().and_then(|s| s.to_str()) {
        Some("rs") => "rust",
        Some("py") => "python",
        Some("js") | Some("ts") => "javascript",
        Some("json") => "json",
        Some("md") => "markdown",
        _ => "plain",
    }
}
```

## Custom Theme

```rust
use ratatui::style::{Color, Style, Modifier};
use ratatui_code_editor::Theme;
use std::collections::HashMap;

fn cyberpunk_theme() -> Theme {
    let mut styles = HashMap::new();

    styles.insert("keyword", Style::default().fg(Color::Magenta).add_modifier(Modifier::BOLD));
    styles.insert("function", Style::default().fg(Color::Cyan));
    styles.insert("string", Style::default().fg(Color::Yellow));
    styles.insert("comment", Style::default().fg(Color::DarkGray).add_modifier(Modifier::ITALIC));
    styles.insert("variable", Style::default().fg(Color::White));

    Theme {
        name: "Cyberpunk".to_string(),
        background: Style::default().bg(Color::Black),
        cursor: Style::default().bg(Color::Cyan).fg(Color::Black),
        selection: Style::default().bg(Color::Indexed(236)),
        gutter: Style::default().fg(Color::DarkGray),
        styles,
    }
}
```

## Status Line

Extract cursor position for status display:

```rust
fn get_cursor_pos(state: &EditorState) -> (usize, usize) {
    let pos = state.cursor();
    let text = state.text();

    let line_idx = text.byte_to_line(pos);
    let line_start = text.line_to_byte(line_idx);
    let col_idx = pos - line_start;

    (line_idx + 1, col_idx + 1)  // 1-based for UI
}

fn draw_status_line(f: &mut Frame, app: &App, area: Rect) {
    let (line, col) = get_cursor_pos(&app.editor_state);
    let status = format!(" LN {}, COL {} | {}", line, col,
        if app.is_modified { "[*]" } else { "" });

    let status_bar = Paragraph::new(status)
        .style(Style::default().fg(Color::Black).bg(Color::Cyan));

    f.render_widget(status_bar, area);
}
```

## Tracking Modifications

```rust
impl App {
    fn handle_event(&mut self, event: &Event) {
        self.editor_state.on_event(event);

        // Mark as modified on text changes
        if let Event::Key(key) = event {
            if matches!(key.code, KeyCode::Char(_) | KeyCode::Backspace | KeyCode::Delete) {
                self.is_modified = true;
            }
        }
    }
}
```

## Search Functionality

```rust
impl App {
    fn find_next(&mut self, query: &str) {
        let text = self.editor_state.text().to_string();
        let current_pos = self.editor_state.cursor();

        if let Some(match_index) = text[current_pos..].find(query) {
            let absolute_index = current_pos + match_index;
            self.editor_state.set_cursor(absolute_index);
            self.editor_state.set_selection(absolute_index, absolute_index + query.len());
        }
    }
}
```

## Common Gotchas

### Heavyweight Highlighter

**Problem**: Creating `SyntaxHighlighter` on every frame is expensive

**Solution**: Store it in App struct, don't recreate:
```rust
struct App {
    highlighter: SyntaxHighlighter,  // ✓ Store once
}

// ✗ Don't create in draw
fn draw() {
    let highlighter = SyntaxHighlighter::new("rust");  // EXPENSIVE
}
```

### Tab Handling

**Problem**: Tab key might not insert spaces as expected

**Solution**: Intercept Tab and insert spaces:
```rust
if key.code == KeyCode::Tab {
    self.editor_state.insert_text("    ");
    return;
}
```

### Line Endings

**Problem**: CRLF from Windows causes cursor issues

**Solution**: Normalize to LF before loading:
```rust
let content = fs::read_to_string(path)?;
let normalized = content.replace("\r\n", "\n");
self.editor_state = EditorState::new(normalized);
```

## Best Practices

1. **Cache highlighter** - Create once, reuse across frames
2. **Track dirty state** - Mark file as modified on edits
3. **Normalize line endings** - Convert CRLF to LF
4. **Provide status feedback** - Show line/col, file path, modified flag
5. **Handle large files** - Ropey handles them well, but test performance
