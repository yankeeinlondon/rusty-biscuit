# Status

A status indicator with themed icons for terminal rendering. Reports the state of a validation or action item using one of three visual themes (Circular, Rounded, Timeline) with Tailwind-based colorization and light/dark mode support. Unlike `Todo`, Status uses plain Unicode icons rather than GFM-compatible square brackets.

## Programmatic Use

```rust
use biscuit_terminal::prelude::*;
use biscuit_terminal::components::status::{Status, StatusState, StatusTheme};

// Create a new status (defaults to NotStarted, Circular theme)
let status = Status::new("Review pull request #42");

// Set state and theme
let status = Status::new("Deploy to production")
    .state(StatusState::Success)
    .theme(StatusTheme::Timeline);

// Create with Prose-formatted description
let status = Status::from_prose("this is a <b>test</b>")
    .state(StatusState::Success);

// Disable icon colorization
let status = Status::new("Plain check")
    .state(StatusState::Success)
    .no_color_icons();

// Render
let term = Terminal::default();
let output = status.display(&term);
```

### State Rendering

| State      | Fallback | Color        |
|------------|----------|--------------|
| NotStarted | ◻        | gray-500     |
| Active     | ◽        | gray-600/400 |
| Success    | ✓        | green-500    |
| Failure    | ⤫        | red-500      |
| Warning    | ⚠        | orange-500   |
| Info       | ℹ        | blue-500     |

Colors with `/` notation (e.g., `gray-600/400`) indicate a light-mode variant: `gray-600` in dark mode, `gray-400` in light mode.

### Themes

| Theme    | Description |
|----------|-------------|
| Circular | Circle-shaped Nerd Font icons (default) |
| Rounded  | Rounded-square shaped Nerd Font icons |
| Timeline | Vertical-bar icons for stacked timeline display |

Each theme provides a distinct set of Nerd Font icons. All themes share the same Unicode fallback characters for non-Nerd Font terminals.

### StatusState Enum

```rust
use biscuit_terminal::components::status::StatusState;

let states = vec![
    StatusState::NotStarted,
    StatusState::Active,
    StatusState::Success,
    StatusState::Failure,
    StatusState::Warning,
    StatusState::Info,
];
```

### Key API

| Method | Description |
|--------|-------------|
| `Status::new(text)` | Create with plain description |
| `Status::from_prose(text)` | Create with Prose-formatted description |
| `.state(StatusState)` | Set the status state |
| `.theme(StatusTheme)` | Set the visual theme |
| `.no_color_icons()` | Disable icon colorization |

## CLI

Not directly exposed as a standalone CLI command.
