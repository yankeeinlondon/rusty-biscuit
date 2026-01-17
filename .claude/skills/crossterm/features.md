# Feature Flags

crossterm provides optional features to reduce dependency footprint and enable additional functionality. Choose features based on your application's needs.

## Available Features

| Feature | Description | Default | Dependency Impact |
|---------|-------------|---------|-------------------|
| `events` | Input/event reading functionality | Yes | Moderate |
| `derive-more` | Adds `is_*` helper functions for event types | Yes | Small |
| `bracketed-paste` | Bracketed paste mode for better paste handling | No | None |
| `event-stream` | Async event stream with futures::Stream | No | Adds tokio/futures |
| `filedescriptor` | Raw file descriptor instead of mio | No | Removes mio |
| `serde` | Serialize/deserialize events | No | Adds serde |
| `osc52` | Clipboard support via OSC52 escape sequence | No | None |

## Default Configuration

```toml
[dependencies]
crossterm = "0.29.0"
```

This includes:
- `events` - Event handling
- `derive-more` - Helper functions

Binary size impact: ~4,600 lines + up to 20,000 lines in dependencies (mostly mio)

## Minimal Configuration

For size-constrained applications, disable default features:

```toml
[dependencies.crossterm]
version = "0.29.0"
default-features = false
```

**What you lose:**
- No event handling (`event::read()`, `event::poll()`)
- No `is_*` helper methods on events
- Can only use terminal/cursor/style modules

**When to use:**
- Simple CLI tools that don't need input handling
- Applications using alternative input libraries
- Embedded systems with strict size constraints

## Common Feature Combinations

### Basic CLI Tool (No Events)

```toml
[dependencies.crossterm]
version = "0.29.0"
default-features = false
```

**Use case:** Simple output-only CLI tools that just need colors and cursor control.

### Interactive CLI (Sync Events)

```toml
[dependencies]
crossterm = "0.29.0"
# Default features are fine
```

**Use case:** Most interactive applications using `poll()`/`read()` pattern.

### Async TUI Application

```toml
[dependencies.crossterm]
version = "0.29.0"
features = ["event-stream"]

[dependencies]
tokio = { version = "1", features = ["full"] }
futures-util = "0.3"
```

**Use case:** Applications using async/await with EventStream.

### Text Editor with Paste Support

```toml
[dependencies.crossterm]
version = "0.29.0"
features = ["bracketed-paste"]
```

**Use case:** Text editors that need to distinguish pasted text from typed text.

### Clipboard-Enabled Application

```toml
[dependencies.crossterm]
version = "0.29.0"
features = ["osc52"]
```

**Use case:** Applications that need clipboard integration (especially over SSH).

**Gotcha:** Not all terminals support OSC52. Test on target terminals.

### Event Serialization

```toml
[dependencies.crossterm]
version = "0.29.0"
features = ["serde"]
```

**Use case:** Applications that need to save/load events, send events over network, etc.

### Reduced Dependencies

```toml
[dependencies.crossterm]
version = "0.29.0"
features = ["filedescriptor"]
```

**Use case:** Avoid mio dependency by using raw file descriptors directly.

**Tradeoff:** Smaller binary size, but less abstraction over platform differences.

## Feature Details

### bracketed-paste

Enables bracketed paste mode, which helps distinguish pasted text from typed text.

```rust
use crossterm::event::Event;

match event::read()? {
    Event::Paste(data) => {
        // Handle pasted text differently
        println!("Pasted {} characters", data.len());
    }
    Event::Key(key) => {
        // Handle individual keystrokes
    }
    _ => {}
}
```

**Why it matters:**
- Prevents pasted text from triggering shortcuts
- Allows special handling of large paste operations
- Essential for text editors and REPLs

### event-stream

Provides `EventStream` for async event handling:

```rust
use crossterm::event::EventStream;
use futures_util::StreamExt;

let mut reader = EventStream::new();

while let Some(Ok(event)) = reader.next().await {
    // Process event asynchronously
}
```

**When to use:**
- Async applications using tokio/async-std
- Need to combine events with other async streams
- Want structured concurrency for event handling

**When NOT to use:**
- Simple synchronous applications
- Don't want async runtime overhead
- Prefer simpler `poll()`/`read()` API

### filedescriptor

Uses raw file descriptor APIs instead of mio for event reading.

**Benefits:**
- Smaller binary size (removes mio dependency)
- More direct platform access

**Drawbacks:**
- Less abstraction over platform differences
- May be less portable

**When to use:**
- Size-constrained embedded systems
- Want minimal dependencies
- Only targeting specific platforms

### serde

Enables serialization/deserialization of event types:

```rust
use crossterm::event::KeyEvent;
use serde::{Serialize, Deserialize};

#[derive(Serialize, Deserialize)]
struct AppEvent {
    key: KeyEvent,
    timestamp: u64,
}

// Serialize events
let json = serde_json::to_string(&app_event)?;

// Deserialize events
let event: AppEvent = serde_json::from_str(&json)?;
```

**Use cases:**
- Recording/replaying user input
- Sending events over network
- Saving event history to disk

### osc52

Enables clipboard operations via OSC52 escape sequences:

```rust
use crossterm::{execute, clipboard::CopyToClipboard};

execute!(stdout(), CopyToClipboard("Hello, Clipboard!"))?;
```

**Limitations:**
- Not supported in all terminals
- Paste from clipboard often unsupported
- May not work over SSH depending on terminal

**When to use:**
- Terminal-based text editors
- Applications running over SSH
- Cross-platform clipboard needs

**Test on target terminals** before relying on this feature.

## Binary Size Comparison

Approximate sizes (release build):

| Configuration | Binary Size |
|---------------|-------------|
| Default features | ~800 KB |
| No default features | ~300 KB |
| + event-stream | ~1.2 MB (adds tokio) |
| + filedescriptor | ~700 KB (removes mio) |
| + serde | ~850 KB |

**Note:** Actual sizes vary based on application code and optimization settings.

## Optimization Tips

1. **Start minimal, add features as needed**

   ```toml
   # Start here
   [dependencies.crossterm]
   version = "0.29.0"
   default-features = false

   # Add features incrementally
   features = ["events"]
   ```

2. **Avoid event-stream unless you need async**

   The sync `poll()`/`read()` API is simpler and smaller for most use cases.

3. **Use filedescriptor for size-critical apps**

   Removes mio dependency at cost of some abstraction.

4. **Test without derive-more**

   The helper functions are convenient but not essential:

   ```rust
   // With derive-more
   if key.is_char('q') { }

   // Without derive-more
   if key.code == KeyCode::Char('q') { }
   ```

## Related

- [Async Patterns](./async.md) - Using event-stream feature
- [Platform Issues](./platform-issues.md) - Feature compatibility across platforms
