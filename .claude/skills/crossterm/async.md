# Async Patterns

crossterm supports asynchronous event handling through the `event-stream` feature, which provides a futures::Stream for terminal events.

## When to Use Async

**Use async (event-stream) when:**
- You're already using an async runtime (tokio, async-std)
- You need to combine terminal events with other async streams
- You want structured concurrency for event handling
- Your application has other async I/O operations

**Use sync (poll/read) when:**
- Simple interactive applications
- Don't want async runtime overhead
- Prefer simpler event handling patterns
- Not using async elsewhere in your app

## Setup

Enable the `event-stream` feature and add an async runtime:

```toml
[dependencies.crossterm]
version = "0.29.0"
features = ["event-stream"]

[dependencies]
tokio = { version = "1", features = ["full"] }
futures-util = "0.3"
```

## Basic Async Event Loop

```rust
use crossterm::event::{Event, EventStream, KeyCode};
use futures_util::StreamExt;

#[tokio::main]
async fn main() -> std::io::Result<()> {
    let mut reader = EventStream::new();

    loop {
        match reader.next().await {
            Some(Ok(event)) => {
                match event {
                    Event::Key(key) => {
                        if key.code == KeyCode::Char('q') {
                            break;
                        }
                        println!("Key: {:?}", key.code);
                    }
                    Event::Resize(w, h) => {
                        println!("Resized to {}x{}", w, h);
                    }
                    _ => {}
                }
            }
            Some(Err(e)) => {
                eprintln!("Error: {:?}", e);
                break;
            }
            None => {
                println!("Stream ended");
                break;
            }
        }
    }

    Ok(())
}
```

## Async TUI Application Template

```rust
use crossterm::{
    event::{Event, EventStream, KeyCode, KeyModifiers},
    execute,
    terminal::{self, EnterAlternateScreen, LeaveAlternateScreen},
};
use futures_util::StreamExt;
use std::io::{self, stdout};
use tokio::time::{interval, Duration};

struct App {
    should_quit: bool,
}

impl App {
    fn new() -> Self {
        Self { should_quit: false }
    }

    async fn run(&mut self) -> io::Result<()> {
        // Setup
        terminal::enable_raw_mode()?;
        execute!(stdout(), EnterAlternateScreen)?;

        // Create event stream
        let mut reader = EventStream::new();

        // Create tick interval for updates
        let mut tick_interval = interval(Duration::from_millis(100));

        loop {
            tokio::select! {
                // Handle events
                maybe_event = reader.next() => {
                    match maybe_event {
                        Some(Ok(event)) => {
                            if self.handle_event(event)? {
                                break;
                            }
                        }
                        Some(Err(e)) => {
                            eprintln!("Error: {:?}", e);
                            break;
                        }
                        None => break,
                    }
                }

                // Periodic updates
                _ = tick_interval.tick() => {
                    self.update()?;
                    self.render()?;
                }
            }
        }

        // Cleanup
        execute!(stdout(), LeaveAlternateScreen)?;
        terminal::disable_raw_mode()?;

        Ok(())
    }

    fn handle_event(&mut self, event: Event) -> io::Result<bool> {
        match event {
            Event::Key(key) => {
                match (key.code, key.modifiers) {
                    (KeyCode::Char('c'), KeyModifiers::CONTROL) => {
                        return Ok(true); // Quit
                    }
                    (KeyCode::Char('q'), KeyModifiers::NONE) => {
                        return Ok(true); // Quit
                    }
                    _ => {
                        self.handle_key(key.code)?;
                    }
                }
            }
            Event::Resize(_, _) => {
                self.render()?;
            }
            _ => {}
        }
        Ok(false)
    }

    fn handle_key(&mut self, code: KeyCode) -> io::Result<()> {
        // Handle specific keys
        Ok(())
    }

    fn update(&mut self) -> io::Result<()> {
        // Update application state
        Ok(())
    }

    fn render(&self) -> io::Result<()> {
        // Render UI
        Ok(())
    }
}

#[tokio::main]
async fn main() -> io::Result<()> {
    let mut app = App::new();
    app.run().await
}
```

## Combining Multiple Async Streams

```rust
use crossterm::event::{Event, EventStream, KeyCode};
use futures_util::{stream::StreamExt, FutureExt};
use tokio::sync::mpsc;
use tokio::time::{interval, Duration};

enum AppEvent {
    Terminal(Event),
    Tick,
    NetworkMessage(String),
}

#[tokio::main]
async fn main() -> io::Result<()> {
    let mut events = EventStream::new();
    let mut tick = interval(Duration::from_millis(100));
    let (tx, mut rx) = mpsc::channel(100);

    // Spawn network task
    tokio::spawn(async move {
        // Simulate network events
        loop {
            tokio::time::sleep(Duration::from_secs(1)).await;
            let _ = tx.send(AppEvent::NetworkMessage("Data".to_string())).await;
        }
    });

    loop {
        tokio::select! {
            maybe_event = events.next() => {
                if let Some(Ok(event)) = maybe_event {
                    handle_terminal_event(event)?;
                }
            }
            _ = tick.tick() => {
                // Periodic update
            }
            Some(msg) = rx.recv() => {
                match msg {
                    AppEvent::NetworkMessage(data) => {
                        println!("Received: {}", data);
                    }
                    _ => {}
                }
            }
        }
    }
}

fn handle_terminal_event(event: Event) -> io::Result<()> {
    // Handle event
    Ok(())
}
```

## Graceful Shutdown

```rust
use tokio::signal;
use tokio::sync::mpsc;

async fn run_with_shutdown() -> io::Result<()> {
    terminal::enable_raw_mode()?;
    execute!(stdout(), EnterAlternateScreen)?;

    let (shutdown_tx, mut shutdown_rx) = mpsc::channel(1);

    // Spawn signal handler
    tokio::spawn(async move {
        signal::ctrl_c().await.expect("Failed to listen for Ctrl+C");
        let _ = shutdown_tx.send(()).await;
    });

    let mut reader = EventStream::new();

    loop {
        tokio::select! {
            maybe_event = reader.next() => {
                // Handle events
            }
            _ = shutdown_rx.recv() => {
                println!("Shutting down...");
                break;
            }
        }
    }

    // Cleanup
    execute!(stdout(), LeaveAlternateScreen)?;
    terminal::disable_raw_mode()?;

    Ok(())
}
```

## Background Tasks

```rust
use tokio::task;

struct App {
    update_handle: Option<task::JoinHandle<()>>,
}

impl App {
    fn start_background_update(&mut self) {
        let handle = task::spawn(async {
            let mut interval = interval(Duration::from_secs(5));
            loop {
                interval.tick().await;
                // Perform background update
            }
        });
        self.update_handle = Some(handle);
    }

    async fn stop_background_update(&mut self) {
        if let Some(handle) = self.update_handle.take() {
            handle.abort();
        }
    }
}
```

## Performance Considerations

### Async vs Sync Performance

**Sync (poll/read):**
- Lower overhead for simple event handling
- No async runtime required
- Simpler code and smaller binary

**Async (event-stream):**
- Better for concurrent operations
- Easier to manage multiple async sources
- Higher overhead from async runtime

**Benchmark:** For simple event loops, sync can be 2-3x faster due to lower overhead.

### Optimizing Async Event Handling

```rust
// Good - batch updates
let mut pending_events = Vec::new();

loop {
    // Collect events
    while let Some(Ok(event)) = reader.next().now_or_never().flatten() {
        pending_events.push(event);
    }

    // Process batch
    for event in pending_events.drain(..) {
        handle_event(event)?;
    }

    // Render once
    render()?;

    // Wait for next event
    if let Some(Ok(event)) = reader.next().await {
        pending_events.push(event);
    }
}
```

## Gotchas

### Runtime Requirements

**Issue:** `event-stream` requires an async runtime

**Solution:** Choose appropriate runtime:

```toml
# Tokio (most common)
[dependencies]
tokio = { version = "1", features = ["full"] }

# async-std (alternative)
[dependencies]
async-std = { version = "1", features = ["attributes"] }
```

### Stream Ending

**Issue:** EventStream can end unexpectedly on errors

**Solution:** Handle None case explicitly:

```rust
match reader.next().await {
    Some(Ok(event)) => { /* handle */ }
    Some(Err(e)) => {
        eprintln!("Error: {:?}", e);
        // Decide whether to break or continue
    }
    None => {
        eprintln!("Event stream ended");
        break;
    }
}
```

### Blocking Operations

**Issue:** Don't perform blocking operations in async context

```rust
// Bad - blocks async runtime
let event = event::read()?; // Sync blocking call

// Good - use async EventStream
let event = reader.next().await;
```

## When NOT to Use Async

For most simple TUI applications, sync event handling is simpler and more efficient:

```rust
// Simple and efficient for basic TUIs
loop {
    if event::poll(Duration::from_millis(100))? {
        let event = event::read()?;
        // Handle event
    }
}
```

Only reach for async when you genuinely need concurrent operations beyond event handling.

## Related

- [Feature Flags](./features.md) - Enabling event-stream feature
- [Event Handling](./events.md) - Sync event handling patterns
- [Raw Mode](./raw-mode.md) - Setting up interactive mode
