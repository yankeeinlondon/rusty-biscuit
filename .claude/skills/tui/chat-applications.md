# Chat Applications

Building chat interfaces in Ratatui requires managing conversation history, input fields, scrolling, and styled message bubbles.

## Basic Structure

```rust
use ratatui::{
    layout::{Constraint, Direction, Layout},
    widgets::{Block, Borders, List, ListItem, Paragraph},
};

struct ChatApp {
    messages: Vec<ChatMessage>,
    input: String,
    list_state: ListState,
}

struct ChatMessage {
    author: String,
    content: String,
    is_user: bool,
}

fn ui(f: &mut Frame, app: &mut ChatApp) {
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Min(3),     // Chat history
            Constraint::Length(3),  // Input box
        ])
        .split(f.area());

    // Render chat history
    render_messages(f, chunks[0], app);

    // Render input
    let input = Paragraph::new(app.input.as_str())
        .style(Style::default().fg(Color::Yellow))
        .block(Block::default().borders(Borders::ALL).title("Input"));
    f.render_widget(input, chunks[1]);

    // Set cursor
    f.set_cursor(
        chunks[1].x + app.input.len() as u16 + 1,
        chunks[1].y + 1,
    );
}
```

## Chat Bubbles

Create left/right aligned bubbles:

```rust
fn render_chat_bubbles(f: &mut Frame, area: Rect, messages: &[ChatMessage]) {
    let main_chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints(
            messages.iter()
                .map(|_| Constraint::Min(3))
                .collect::<Vec<_>>()
        )
        .split(area);

    for (i, msg) in messages.iter().enumerate() {
        let bubble_area = main_chunks[i];

        // Create 3-column layout for alignment
        let bubble_chunks = Layout::default()
            .direction(Direction::Horizontal)
            .constraints(if msg.is_user {
                [Constraint::Percentage(20), Constraint::Percentage(80), Constraint::Length(0)]
            } else {
                [Constraint::Length(0), Constraint::Percentage(80), Constraint::Percentage(20)]
            })
            .split(bubble_area);

        let (target_area, color, title) = if msg.is_user {
            (bubble_chunks[1], Color::Blue, "You")
        } else {
            (bubble_chunks[1], Color::Green, "Assistant")
        };

        let paragraph = Paragraph::new(msg.content.as_str())
            .block(
                Block::default()
                    .borders(Borders::ALL)
                    .border_style(Style::default().fg(color))
                    .title(title)
            )
            .wrap(Wrap { trim: true });

        f.render_widget(paragraph, target_area);
    }
}
```

## Different Border Styles

```rust
use ratatui::widgets::BorderType;

let user_block = Block::default()
    .borders(Borders::ALL)
    .border_type(BorderType::Thick);  // Sharp for user

let bot_block = Block::default()
    .borders(Borders::ALL)
    .border_type(BorderType::Rounded);  // Rounded for bot
```

## Streaming Responses

Handle token-by-token streaming:

```rust
enum Message {
    UserMessage(String),
    StreamStart,
    StreamChunk(String),
    StreamEnd,
}

impl ChatApp {
    fn handle_stream_message(&mut self, msg: Message) {
        match msg {
            Message::StreamStart => {
                self.messages.push(ChatMessage {
                    author: "Assistant".to_string(),
                    content: String::new(),
                    is_user: false,
                });
                self.scroll_to_bottom();
            }
            Message::StreamChunk(chunk) => {
                if let Some(last) = self.messages.last_mut() {
                    last.content.push_str(&chunk);
                    last.content.push(' ');
                }
            }
            Message::StreamEnd => {
                // Finalize message
            }
            _ => {}
        }
    }
}
```

## Auto-Scroll Behavior

Only scroll to bottom if user is already near bottom:

```rust
impl ChatApp {
    fn should_auto_scroll(&self) -> bool {
        if let Some(selected) = self.list_state.selected() {
            selected >= self.messages.len().saturating_sub(5)
        } else {
            false
        }
    }

    fn add_message(&mut self, msg: ChatMessage) {
        let should_scroll = self.should_auto_scroll();
        self.messages.push(msg);

        if should_scroll {
            self.scroll_to_bottom();
        }
    }

    fn scroll_to_bottom(&mut self) {
        if !self.messages.is_empty() {
            self.list_state.select(Some(self.messages.len() - 1));
        }
    }
}
```

## Multi-line Input

Handle Ctrl+Enter for newlines:

```rust
fn handle_input(key: KeyEvent, app: &mut ChatApp, tx: &Sender<String>) {
    match key.code {
        KeyCode::Enter if key.modifiers.contains(KeyModifiers::CONTROL) => {
            app.input.push('\n');
        }
        KeyCode::Enter => {
            if !app.input.is_empty() {
                let msg = app.input.drain(..).collect::<String>();
                app.add_message(ChatMessage {
                    author: "You".to_string(),
                    content: msg.clone(),
                    is_user: true,
                });
                let _ = tx.send(msg);
            }
        }
        KeyCode::Char(c) => app.input.push(c),
        KeyCode::Backspace => { app.input.pop(); }
        _ => {}
    }
}
```

## Complete Async Chat Example

```rust
use tokio::sync::mpsc;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let mut app = ChatApp::new();
    let (tx_to_background, mut rx_from_ui) = mpsc::unbounded_channel();
    let (tx_to_ui, mut rx_from_background) = mpsc::unbounded_channel();

    // Background worker
    tokio::spawn(async move {
        while let Some(prompt) = rx_from_ui.recv().await {
            let _ = tx_to_ui.send(Message::StreamStart);

            // Simulate streaming
            for word in "This is a streaming response".split_whitespace() {
                tokio::time::sleep(Duration::from_millis(100)).await;
                let _ = tx_to_ui.send(Message::StreamChunk(word.to_string()));
            }

            let _ = tx_to_ui.send(Message::StreamEnd);
        }
    });

    // Main loop
    loop {
        terminal.draw(|f| ui(f, &mut app))?;

        // Handle background messages
        while let Ok(msg) = rx_from_background.try_recv() {
            app.handle_stream_message(msg);
        }

        // Handle input
        if event::poll(Duration::from_millis(16))? {
            if let Event::Key(key) = event::read()? {
                handle_input(key, &mut app, &tx_to_background);
            }
        }
    }
}
```

## Best Practices

1. **Cache Markdown parsing** - Parse once, render many times
2. **Implement smart auto-scroll** - Only scroll if user is near bottom
3. **Show typing indicators** - Visual feedback during streaming
4. **Handle long messages** - Calculate dynamic heights with wrapping
5. **Provide clear visual distinction** - Use colors, borders, alignment for user vs bot
