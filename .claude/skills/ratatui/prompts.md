# Prompts and Forms

The `tui-prompts` crate provides reusable input widgets for text entry, passwords, confirmations, and more.

## Basic Text Prompt

```toml
[dependencies]
tui-prompts = "0.3"
```

```rust
use tui_prompts::{prelude::*, State as _};

struct App<'a> {
    name_state: TextState<'a>,
}

impl<'a> App<'a> {
    fn new() -> Self {
        Self {
            name_state: TextState::new(),
        }
    }

    fn draw(&mut self, frame: &mut Frame) {
        let prompt = TextPrompt::from("Enter your name:")
            .with_block(Block::default().borders(Borders::ALL));

        frame.render_stateful_widget(prompt, frame.area(), &mut self.name_state);

        // Set cursor position
        if self.name_state.is_focused() {
            let pos = self.name_state.cursor();
            frame.set_cursor_position((pos.x, pos.y));
        }
    }

    fn handle_event(&mut self, event: &Event) {
        let status = self.name_state.handle_event(event);

        match status {
            Status::Completed => {
                let value = self.name_state.value();
                println!("User entered: {}", value);
            }
            Status::Aborted => {
                // User pressed Esc
            }
            _ => {}
        }
    }
}
```

## Multi-Field Form

```rust
struct FormApp<'a> {
    states: [TextState<'a>; 2],
    focus_index: usize,
}

impl<'a> FormApp<'a> {
    fn draw(&mut self, frame: &mut Frame) {
        let chunks = Layout::default()
            .direction(Direction::Vertical)
            .constraints([
                Constraint::Length(3),  // Name field
                Constraint::Length(3),  // Email field
                Constraint::Min(1),     // Help text
            ])
            .split(frame.area());

        // Name prompt
        let name_prompt = TextPrompt::from("Name: ")
            .with_block(Block::default().borders(Borders::ALL));
        frame.render_stateful_widget(name_prompt, chunks[0], &mut self.states[0]);

        // Email prompt
        let email_prompt = TextPrompt::from("Email: ")
            .with_block(Block::default().borders(Borders::ALL));
        frame.render_stateful_widget(email_prompt, chunks[1], &mut self.states[1]);

        // Set cursor
        let active_state = &self.states[self.focus_index];
        frame.set_cursor_position((
            chunks[self.focus_index].x + active_state.cursor().x + 7,
            chunks[self.focus_index].y + 1,
        ));
    }

    fn handle_input(&mut self, event: Event) {
        if let Event::Key(key) = event {
            match key.code {
                KeyCode::Tab => {
                    self.focus_index = (self.focus_index + 1) % self.states.len();
                }
                KeyCode::Esc => return,
                _ => {
                    self.states[self.focus_index].handle_event(&event);
                }
            }
        }
    }
}
```

## Password Masking

```rust
let password_prompt = TextPrompt::from("Password: ")
    .with_mask('*')
    .with_block(Block::default().borders(Borders::ALL));
```

## Custom Styling

```rust
let theme_color = Color::Rgb(255, 121, 198);

let prompt = TextPrompt::from(
    Span::styled(
        " ➤ Query: ",
        Style::default().fg(theme_color).add_modifier(Modifier::BOLD),
    )
)
.with_placeholder(
    Span::styled(
        "type something...",
        Style::default().fg(Color::DarkGray).add_modifier(Modifier::ITALIC),
    )
)
.with_style(Style::default().fg(Color::White))
.with_block(
    Block::default()
        .borders(Borders::ALL)
        .border_type(BorderType::Rounded)
        .border_style(Style::default().fg(theme_color))
);
```

## Live Validation

```rust
fn draw_with_validation(&mut self, frame: &mut Frame) {
    let is_valid = self.email_state.value().contains('@');

    let border_color = if is_valid || self.email_state.value().is_empty() {
        Color::Green
    } else {
        Color::Red
    };

    let prompt = TextPrompt::from("Email: ")
        .with_block(
            Block::default()
                .borders(Borders::ALL)
                .border_style(Style::default().fg(border_color))
        );

    frame.render_stateful_widget(prompt, area, &mut self.email_state);
}
```

## Autocomplete

```rust
const COMMANDS: [&str; 4] = ["fetch", "commit", "push", "pull"];

fn handle_input(app: &mut App, event: &Event) {
    if let Event::Key(key) = event {
        if key.code == KeyCode::Tab {
            let current_input = app.query_state.value();

            if let Some(matched) = COMMANDS.iter()
                .find(|&&c| c.starts_with(current_input))
            {
                app.query_state.set_value(matched.to_string());
                return;
            }
        }
    }

    app.query_state.handle_event(event);
}
```

## Common Gotchas

### Event Swallowing

**Problem**: Prompt consumes keys you need for global hotkeys

**Solution**: Check for global keys before passing to prompt:
```rust
if let Event::Key(key) = event {
    if key.code == KeyCode::Char('q') && !self.state.is_focused() {
        return Ok(None);  // Quit only if not typing
    }
}
self.state.handle_event(event);
```

### Cursor Visibility

**Problem**: Cursor doesn't appear where expected

**Solution**: Always call `frame.set_cursor_position()` based on `state.cursor()`:
```rust
if self.input_state.is_focused() {
    let pos = self.input_state.cursor();
    frame.set_cursor_position((area.x + pos.x + offset, area.y + pos.y));
}
```

### State Persistence

**Problem**: User loses typed text when recreating state

**Solution**: Store state in App struct, not in draw function:
```rust
struct App {
    input_state: tui_prompts::State,  // ✓ Persist here
}

// ✗ Don't create new state in draw
fn draw() {
    let state = TextState::new();  // WRONG
}
```

## Best Practices

1. **Store states at App level** - Never recreate state in draw loop
2. **Handle Tab for focus management** - Move between fields with Tab
3. **Validate on every frame** - Show visual feedback immediately
4. **Clear completed prompts** - Call `state.clear()` after submission
5. **Use placeholders** - Guide users with example text
