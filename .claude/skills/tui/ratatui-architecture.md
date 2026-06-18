---
name: tui-ratatui-architecture
description: Building terminal UIs in Rust
---


# Ratatui Architecture (Rust TUI)

**Use this skill when:**
- Building terminal UIs in Rust
- Using immediate-mode rendering
- Creating high-performance TUIs
- Working with Ratatui/Crossterm
- Building production Rust CLIs

> The manual `enable_raw_mode` / `EnterAlternateScreen` setup below is the minimal
> pattern (and what you need on Ratatui ≤ 0.29). For new fullscreen apps on
> Ratatui 0.30+, prefer `ratatui::run`, which restores the terminal on exit and
> panic. For structuring a real app — state, update, lifecycle, resize, and a
> readiness checklist — see [Ratatui Best Practices](./ratatui-best-practices.md).

## Basic Application

```rust
use crossterm::{
    event::{self, Event, KeyCode},
    execute,
    terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen},
};
use ratatui::{
    backend::CrosstermBackend,
    widgets::{Block, Borders, Paragraph},
    Terminal,
};
use std::io;

struct App {
    counter: i32,
}

impl App {
    fn new() -> App {
        App { counter: 0 }
    }

    fn on_key(&mut self, key: KeyCode) {
        match key {
            KeyCode::Up => self.counter += 1,
            KeyCode::Down => self.counter -= 1,
            _ => {}
        }
    }
}

fn main() -> Result<(), io::Error> {
    enable_raw_mode()?;
    let mut stdout = io::stdout();
    execute!(stdout, EnterAlternateScreen)?;

    let backend = CrosstermBackend::new(stdout);
    let mut terminal = Terminal::new(backend)?;

    let mut app = App::new();

    loop {
        terminal.draw(|f| {
            let block = Block::default()
                .title("Counter")
                .borders(Borders::ALL);

            let paragraph = Paragraph::new(format!("Count: {}", app.counter))
                .block(block);

            f.render_widget(paragraph, f.area());
        })?;

        if let Event::Key(key) = event::read()? {
            match key.code {
                KeyCode::Char('q') => break,
                code => app.on_key(code),
            }
        }
    }

    disable_raw_mode()?;
    execute!(terminal.backend_mut(), LeaveAlternateScreen)?;

    Ok(())
}
```

## Event Loop

```rust
use crossterm::event::{self, Event, KeyCode};
use std::time::Duration;

fn run_app(terminal: &mut Terminal<impl Backend>, app: &mut App) -> io::Result<()> {
    loop {
        terminal.draw(|f| ui(f, app))?;

        if event::poll(Duration::from_millis(100))? {
            if let Event::Key(key) = event::read()? {
                match key.code {
                    KeyCode::Char('q') => return Ok(()),
                    KeyCode::Up => app.increment(),
                    KeyCode::Down => app.decrement(),
                    _ => {}
                }
            }
        }
    }
}
```

