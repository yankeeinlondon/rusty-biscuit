# Todo

A TODO item with state tracking for terminal rendering. Supports five states (Open, InProgress, Completed, Cancelled, Blocked) with automatic icon and color adaptation based on terminal capabilities: Nerd Font icons for patched fonts, colored ASCII fallbacks for regular terminals, and plain ASCII for colorless environments.

## Programmatic Use

```rust
use biscuit_terminal::prelude::*;
use biscuit_terminal::components::todo::{Todo, TodoState};

// Create a new open TODO
let todo = Todo::new("Review pull request #42");

// Render
let term = Terminal::default();
let output = todo.display(&term);
```

### State Rendering

| State | Nerd Font | Color Fallback | No-Color Fallback |
|-------|-----------|---------------|-------------------|
| Open | checkbox outline | `[ ]` | `[ ]` |
| InProgress | progress icon | `[⏺]` (green) | `[>]` |
| Completed | checkmark icon | `[✔]` (green) | `[x]` |
| Cancelled | cancelled icon | `[-]` (red) | `[-]` |
| Blocked | blocked icon | `[⏺]` (red) | `[!]` |

### TodoState Enum

```rust
use biscuit_terminal::components::todo::TodoState;

let states = vec![
    TodoState::Open,
    TodoState::InProgress,
    TodoState::Completed,
    TodoState::Cancelled,
    TodoState::Blocked,
];
```

### Icon Lookup

Use `TODO_CHAR_LOOKUP` to get the string representation for each state:

```rust
use biscuit_terminal::components::todo::{TODO_CHAR_LOOKUP, TodoState};

let rep = &TODO_CHAR_LOOKUP[&TodoState::Completed];
println!("Nerd: {}", rep.nerd);
println!("Fallback: {}", rep.fallback);
```

## CLI

Not directly exposed as a standalone CLI command. Todo rendering is used by `darkmatter` when processing Markdown task lists (`- [x]`, `- [ ]`, etc.).
