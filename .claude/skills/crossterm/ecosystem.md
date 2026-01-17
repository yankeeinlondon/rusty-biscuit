# Ecosystem

crossterm is rarely used in isolation. This guide covers the most common companion crates and how they integrate.

## UI Frameworks

### Ratatui

**The gold standard** for building terminal user interfaces with crossterm.

```toml
[dependencies]
crossterm = "0.29.0"
ratatui = "0.29"
```

**What it provides:**
- Layout engine (flexbox-like constraints)
- Widget library (paragraphs, lists, tables, charts, gauges)
- Buffer management (double buffering, diff-based rendering)
- Event loop patterns

**Relationship to crossterm:**
- Uses crossterm as default backend for terminal operations
- Abstracts away raw crossterm calls
- Provides higher-level API for complex UIs

**When to use:**
- Building dashboards, file managers, monitoring tools
- Need layout system for complex interfaces
- Want pre-built widgets (charts, sparklines, etc.)

**Example:**

```rust
use crossterm::{
    terminal::{enable_raw_mode, disable_raw_mode},
    event::{self, Event, KeyCode},
};
use ratatui::{
    backend::CrosstermBackend,
    Terminal, widgets::Paragraph,
};

fn main() -> Result<(), Box<dyn std::error::Error>> {
    enable_raw_mode()?;
    let mut stdout = std::io::stdout();
    let backend = CrosstermBackend::new(stdout);
    let mut terminal = Terminal::new(backend)?;

    terminal.draw(|f| {
        let widget = Paragraph::new("Hello, Ratatui!");
        f.render_widget(widget, f.area());
    })?;

    // Event loop with crossterm
    loop {
        if event::poll(std::time::Duration::from_millis(100))? {
            if let Event::Key(key) = event::read()? {
                if key.code == KeyCode::Char('q') {
                    break;
                }
            }
        }
    }

    disable_raw_mode()?;
    Ok(())
}
```

**Alternatives:**
- **Cursive** - Higher-level, more opinionated TUI framework
- **tui-realm** - Component-based framework inspired by React

## Interactive Prompts

### Inquire (Recommended)

**The most popular** library for interactive CLI prompts and surveys.

```toml
[dependencies]
inquire = "0.7"
```

**What it provides:**
- Text input with validation
- Password input (hidden)
- Confirmation prompts (y/n)
- Single-select from list
- Multi-select checkboxes
- Date pickers

**Relationship to crossterm:**
- Uses crossterm under the hood for terminal control
- Handles raw mode, cursor positioning automatically
- Cleans up terminal state on Ctrl+C

**When to use:**
- Building interactive CLI wizards
- Need user input with validation
- Want professional-looking prompts without manual cursor control

**Example:**

```rust
use inquire::{Text, Select, MultiSelect, Confirm};

fn main() {
    let name = Text::new("What is your name?")
        .with_placeholder("Ferris")
        .prompt();

    let language = Select::new(
        "Favorite language?",
        vec!["Rust", "Python", "Go", "JavaScript"]
    ).prompt();

    let features = MultiSelect::new(
        "Which features matter most?",
        vec!["Speed", "Memory Safety", "Ease of Use"]
    ).prompt();

    let confirm = Confirm::new("Save these preferences?")
        .with_default(true)
        .prompt();
}
```

**Why it works well with crossterm:**
- Raw mode for instant keystroke capture
- Cursor save/restore for live updates
- ANSI styling for question marks and help text

### Alternatives

**Dialoguer:**
- Simpler API than inquire
- Uses `console` crate by default (not crossterm)
- Good for basic prompts

```toml
[dependencies]
dialoguer = "0.11"
```

**Cliclack:**
- Beautiful aesthetic (vertical lines, modern symbols)
- Inspired by JavaScript Clack library
- More opinionated design

```toml
[dependencies]
cliclack = "0.3"
```

**Requestty:**
- Widget-based system
- Inspired by Node.js inquirer
- More complex, more customizable

```toml
[dependencies]
requestty = "0.5"
```

## CLI Argument Parsing

### Clap

Almost every crossterm application needs to parse command-line arguments.

```toml
[dependencies]
clap = { version = "4", features = ["derive"] }
crossterm = "0.29.0"
```

**Example:**

```rust
use clap::Parser;

#[derive(Parser)]
struct Args {
    /// Enable debug output
    #[arg(short, long)]
    debug: bool,

    /// File to process
    file: String,
}

fn main() {
    let args = Args::parse();

    // Use crossterm to display colored output based on args
    if args.debug {
        println!("Debug mode enabled");
    }
}
```

## Error Handling

### Anyhow

Simplifies error handling in crossterm applications.

```toml
[dependencies]
anyhow = "1"
crossterm = "0.29.0"
```

**Why it matters:**
- crossterm operations return `io::Result<T>`
- Anyhow provides `?` operator for easy propagation
- Context methods for better error messages

**Example:**

```rust
use anyhow::{Context, Result};
use crossterm::{execute, terminal::{Clear, ClearType}};

fn main() -> Result<()> {
    execute!(
        std::io::stdout(),
        Clear(ClearType::All)
    ).context("Failed to clear terminal")?;

    Ok(())
}
```

### Thiserror

For library code that defines custom error types.

```toml
[dependencies]
thiserror = "1"
crossterm = "0.29.0"
```

## Styling & Formatting

### Comfy-table

For complex table rendering.

```toml
[dependencies]
comfy-table = "7"
crossterm = "0.29.0"
```

**What it provides:**
- Auto-wrapping text
- Cell alignment
- Border styles
- Color integration with crossterm

**Example:**

```rust
use comfy_table::{Table, Cell, Color};

let mut table = Table::new();
table.add_row(vec![
    Cell::new("Name").fg(Color::Blue),
    Cell::new("Age").fg(Color::Blue),
]);
table.add_row(vec!["Alice", "30"]);

println!("{}", table);
```

### Owo-colors

Simpler ANSI coloring alternative to crossterm's styling.

```toml
[dependencies]
owo-colors = "4"
```

**When to use:**
- Simple colored text output
- Don't need full crossterm terminal control
- Want more ergonomic color API

**Example:**

```rust
use owo_colors::OwoColorize;

println!("{}", "Error:".red().bold());
println!("{}", "Success!".green());
```

## Async Runtimes

### Tokio

Required when using crossterm's `event-stream` feature.

```toml
[dependencies.crossterm]
version = "0.29.0"
features = ["event-stream"]

[dependencies]
tokio = { version = "1", features = ["full"] }
futures-util = "0.3"
```

**Example:**

```rust
use crossterm::event::{EventStream, Event};
use futures_util::StreamExt;

#[tokio::main]
async fn main() {
    let mut reader = EventStream::new();

    while let Some(Ok(event)) = reader.next().await {
        match event {
            Event::Key(key) => println!("Key: {:?}", key),
            _ => {}
        }
    }
}
```

## Comparison Summary

| Crate | Purpose | Complexity | Integration |
|-------|---------|------------|-------------|
| **Ratatui** | Full TUI framework | High | Uses crossterm as backend |
| **Inquire** | Interactive prompts | Medium | Uses crossterm internally |
| **Clap** | Argument parsing | Low | Complements crossterm apps |
| **Anyhow** | Error handling | Low | Simplifies crossterm error handling |
| **Comfy-table** | Table formatting | Medium | Uses crossterm for styling |

## Typical Stack

A complete crossterm application often uses:

```toml
[dependencies]
# Core terminal control
crossterm = "0.29.0"

# UI (choose one)
ratatui = "0.29"           # For complex TUIs
inquire = "0.7"            # For interactive prompts

# Utilities
clap = { version = "4", features = ["derive"] }
anyhow = "1"

# Optional
tokio = { version = "1", features = ["full"] }  # If using event-stream
serde = { version = "1", features = ["derive"] } # For config files
```

## Related

- [Use Cases](./use-cases.md) - When to use crossterm
- [Features](./features.md) - crossterm feature flags
- [Async Patterns](./async.md) - Using event-stream with tokio
