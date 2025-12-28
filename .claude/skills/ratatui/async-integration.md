# Async Integration

Integrating async operations into a Ratatui TUI requires careful coordination to keep the UI responsive while performing background work.

## The Problem

Blocking the main loop freezes the entire UI:

```rust
// ✗ BAD: Freezes UI during network call
loop {
    terminal.draw(|f| ui(f))?;

    // This blocks everything!
    let data = make_blocking_api_call();
    app.update(data);
}
```

## The Solution: Channels

Use Tokio channels for communication between the UI thread and background tasks:

```rust
use tokio::sync::mpsc;
use std::time::Duration;

enum Message {
    UserPrompt(String),
    BotResponseChunk(String),
    Error(String),
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Create channels
    let (tx_to_main, mut rx_from_background) = mpsc::unbounded_channel::<Message>();
    let (tx_to_background, mut rx_from_main) = mpsc::unbounded_channel::<String>();

    // Spawn background worker
    tokio::spawn(async move {
        while let Some(prompt) = rx_from_main.recv().await {
            // Simulate API call
            tokio::time::sleep(Duration::from_secs(1)).await;

            let _ = tx_to_main.send(Message::BotResponseChunk(
                format!("Response to: {}", prompt)
            ));
        }
    });

    // Main UI loop
    loop {
        terminal.draw(|f| app.render(f))?;

        // Non-blocking check for messages
        while let Ok(msg) = rx_from_background.try_recv() {
            match msg {
                Message::BotResponseChunk(text) => {
                    app.messages.push(format!("Bot: {}", text));
                    app.auto_scroll_to_bottom();
                }
                Message::Error(e) => app.log_error(e),
                _ => {}
            }
        }

        // Handle user input
        if event::poll(Duration::from_millis(10))? {
            if let Event::Key(key) = event::read()? {
                if key.code == KeyCode::Enter && !app.input.is_empty() {
                    let user_text = app.input.drain(..).collect::<String>();
                    app.messages.push(format!("You: {}", user_text));

                    // Send to background task
                    let _ = tx_to_background.send(user_text);
                }
            }
        }
    }
}
```

## Streaming Responses

For LLM-style streaming where tokens arrive incrementally:

```rust
enum Message {
    StreamStart,
    StreamChunk(String),
    StreamEnd,
}

// Background worker
tokio::spawn(async move {
    while let Some(prompt) = rx_from_main.recv().await {
        let _ = tx_to_main.send(Message::StreamStart);

        // Simulate streaming tokens
        let response = "Hello this is a streaming response";
        for word in response.split_whitespace() {
            tokio::time::sleep(Duration::from_millis(100)).await;
            let _ = tx_to_main.send(Message::StreamChunk(word.to_string()));
        }

        let _ = tx_to_main.send(Message::StreamEnd);
    }
});

// In UI loop
while let Ok(msg) = rx_from_background.try_recv() {
    match msg {
        Message::StreamStart => {
            app.messages.push(String::new());
            app.is_streaming = true;
        }
        Message::StreamChunk(chunk) => {
            if let Some(last) = app.messages.last_mut() {
                last.push_str(&chunk);
                last.push(' ');
            }
        }
        Message::StreamEnd => {
            app.is_streaming = false;
        }
    }
}
```

## Loading States

Show visual feedback during async operations:

```rust
struct App {
    is_loading: bool,
    spinner_state: usize,
}

impl App {
    fn render(&self, f: &mut Frame) {
        if self.is_loading {
            let spinner = ["⠋", "⠙", "⠹", "⠸", "⠼", "⠴", "⠦", "⠧", "⠇", "⠏"];
            let symbol = spinner[self.spinner_state % spinner.len()];

            let loading = Paragraph::new(format!("{} Loading...", symbol))
                .style(Style::default().fg(Color::Yellow));
            f.render_widget(loading, area);
        }
    }

    fn tick(&mut self) {
        if self.is_loading {
            self.spinner_state += 1;
        }
    }
}

// In main loop
if app.is_loading {
    app.tick();
}
```

## Common Gotchas

### The Borrow Checker in terminal.draw

**Problem**: Cannot mutate app while it's borrowed by the draw closure

**Solution**: Process channel messages before or after draw, never inside:

```rust
// ✓ CORRECT
while let Ok(msg) = rx.try_recv() {
    app.handle_message(msg);
}
terminal.draw(|f| app.render(f))?;

// ✗ WRONG
terminal.draw(|f| {
    while let Ok(msg) = rx.try_recv() {  // Cannot borrow app as mutable here
        app.handle_message(msg);
    }
    app.render(f);
})?;
```

### Shutdown Panics

**Problem**: Background thread hangs when app quits

**Solution**: Use cancellation tokens or let channels close naturally:

```rust
use tokio_util::sync::CancellationToken;

let cancel_token = CancellationToken::new();
let worker_token = cancel_token.clone();

tokio::spawn(async move {
    loop {
        tokio::select! {
            _ = worker_token.cancelled() => break,
            Some(msg) = rx.recv() => {
                // Process message
            }
        }
    }
});

// When quitting
cancel_token.cancel();
```

### Message Ordering

**Problem**: Messages arrive out of order from multiple async tasks

**Solution**: Tag messages with sequence numbers or use a single coordinating task:

```rust
enum Message {
    Response { id: u64, text: String },
}

struct App {
    next_id: u64,
    pending_requests: HashMap<u64, String>,
}

impl App {
    fn send_request(&mut self, prompt: String, tx: &Sender<(u64, String)>) {
        let id = self.next_id;
        self.next_id += 1;
        self.pending_requests.insert(id, prompt.clone());
        let _ = tx.send((id, prompt));
    }

    fn handle_response(&mut self, id: u64, text: String) {
        if self.pending_requests.remove(&id).is_some() {
            // Valid response
            self.messages.push(text);
        }
    }
}
```

## Best Practices

1. **Use `try_recv()` not `recv()`** - Never block the UI thread
2. **Keep background tasks simple** - Offload heavy computation, return results
3. **Handle channel closure** - Both ends should gracefully handle disconnection
4. **Limit message queue size** - Use bounded channels to prevent memory bloat
5. **Show loading indicators** - Always provide visual feedback for async work
