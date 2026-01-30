# Queue TUI Architecture

Detailed reference for `queue-cli` - the ratatui-based TUI application.

## Module Structure

```
cli/src/tui/
├── mod.rs           # Module exports
├── app.rs           # Application state (App, AppMode)
├── event.rs         # Event loop and input routing
├── input_modal.rs   # Task creation/editing form
├── history_modal.rs # History browser
├── modal.rs         # Modal infrastructure (Modal trait)
├── render.rs        # UI rendering
└── color_context.rs # Theme/color management
```

## Application State

### AppMode

Controls keyboard behavior and UI rendering.

```rust
pub enum AppMode {
    Normal,       // Standard navigation
    EditMode,     // Modifying selected task
    RemoveMode,   // Confirming task removal
    InputModal,   // Entering new/editing task
    HistoryModal, // Viewing completed tasks
    ConfirmQuit,  // Awaiting Y/N to exit
}
```

### App Struct

```rust
pub struct App {
    pub tasks: Vec<ScheduledTask>,
    pub mode: AppMode,
    pub selected_index: usize,
    pub should_quit: bool,
    pub event_rx: Option<mpsc::Receiver<TaskEvent>>,
    pub executor: Option<TaskExecutor>,
    pub table_state: TableState,
    pub input_modal: Option<InputModal>,
    pub history_modal: Option<HistoryModal>,
    pub capabilities: TerminalCapabilities,
    pub history_store: Option<JsonFileStore>,
    pub next_task_id: u64,
}
```

**Builder Pattern:**

```rust
let app = App::new()
    .with_executor()
    .with_history_store(JsonFileStore::default_path());
```

**Key Methods:**

| Method | Description |
|--------|-------------|
| `schedule_task(task)` | Add and schedule task |
| `update_task(...)` | Modify existing task |
| `cancel_task(id)` | Cancel pending task |
| `alloc_task_id()` | Get next available ID |
| `select_next/previous()` | Navigate task list |
| `selected_task()` | Get current selection |
| `handle_task_event(event)` | Process executor events |

## Keyboard Shortcuts

### Normal Mode

| Key | Action |
|-----|--------|
| `q` / `Q` | Quit (with confirmation if active tasks) |
| `Esc` | Quit immediately |
| `n` / `N` | New task (opens input modal) |
| `e` / `E` | Edit selected task |
| `r` / `R` | Remove mode |
| `x` / `X` | Cancel selected pending task |
| `h` / `H` | History modal |
| `↑` / `k` | Select previous |
| `↓` / `j` | Select next |

### Input Modal

| Key | Action |
|-----|--------|
| `Tab` | Next field |
| `Shift+Tab` | Previous field |
| `Enter` | Submit form |
| `Esc` | Cancel |
| `Space` | Toggle selector fields |
| `←` / `→` | Move cursor or toggle selectors |
| `Backspace` | Delete character |
| `Ctrl+A` / `Home` | Move to start |
| `Ctrl+E` / `End` | Move to end |

### History Modal

| Key | Action |
|-----|--------|
| `↑` / `k` | Select previous |
| `↓` / `j` | Select next |
| `Enter` | Use selected command |
| `n` / `N` | New task from selected |
| `f` / `F` / `/` | Filter mode |
| `Esc` | Close modal |

### Confirm Quit

| Key | Action |
|-----|--------|
| `y` / `Y` / `Enter` | Confirm exit |
| `n` / `N` / `Esc` | Cancel, return to normal |

## Input Modal

### Fields

| Field | Type | Description |
|-------|------|-------------|
| Command | Text | Shell command to execute |
| Schedule Type | Selector | `At Time` or `After Delay` |
| Schedule Value | Text | Time (`7:00am`) or delay (`15m`) |
| Target | Selector | Cycles based on terminal capabilities |

### InputField Enum

```rust
pub enum InputField {
    Command,
    ScheduleType,
    ScheduleValue,
    Target,
}
```

### Target Cycling by Terminal

| Terminal | Available Targets |
|----------|-------------------|
| Wezterm/iTerm2 | NewPane → NewWindow → Background |
| GUI terminals | NewWindow → Background |
| Unknown | Background only |

### Default Target Selection

| Terminal | Default |
|----------|---------|
| Wezterm/iTerm2 | NewPane |
| GUI terminals | NewWindow |
| Unknown | Background |

### Validation

| Field | Rules |
|-------|-------|
| Command | Must not be empty |
| Schedule Value | Must not be empty |
| At Time | Must match `7:00am`, `19:30` formats |
| After Delay | Must match `15`, `30s`, `2h` formats |

Errors display in red at modal bottom.

### Layout Modes

| Mode | Threshold | Min Height |
|------|-----------|------------|
| Full | >= 18 rows | 16 rows |
| Compact | < 18 rows | 12 rows |

Compact mode uses inline labels instead of bordered fields.

## History Modal

### State

```rust
pub struct HistoryModal {
    pub items: Vec<ScheduledTask>,
    pub list_state: ListState,
    pub filter: String,
    pub filter_mode: bool,
    pub layout: HistoryLayout,
}
```

### Filtering

1. Press `f`, `F`, or `/` to enter filter mode
2. Type to search (case-insensitive command substring)
3. Press `Esc` or `Enter` to exit filter mode
4. Selection resets to first matching item

## Modal Infrastructure

### Modal Trait

```rust
pub trait Modal {
    fn render(&self, frame: &mut Frame, area: Rect, color_context: &ColorContext);
    fn title(&self) -> &str;
    fn width_percent(&self) -> u16 { 60 }
    fn height_percent(&self) -> u16 { 40 }
    fn min_height(&self) -> u16 { 0 }
}
```

### Built-in Modals

| Modal | Width | Height | Purpose |
|-------|-------|--------|---------|
| InputModal | 75% | 60% | Create/edit tasks |
| HistoryModal | 60% | 60% | Browse history |
| ConfirmQuitDialog | 40% | 20% | Confirm exit |

## Event Loop

### run_app() Flow

```rust
loop {
    terminal.draw(|frame| render::render(app, frame))?;

    // Non-blocking task event collection
    while let Ok(event) = event_rx.try_recv() {
        app.handle_task_event(event);
    }

    // Keyboard input with 50ms timeout
    if poll(Duration::from_millis(50))? {
        if let Event::Key(key) = read()? {
            // Route to mode handler
        }
    }

    if app.should_quit { return Ok(()); }
}
```

### Mode Handlers

| Mode | Handler |
|------|---------|
| Normal | `handle_normal_mode()` |
| ConfirmQuit | `handle_confirm_quit()` |
| InputModal | `handle_input_modal()` |
| HistoryModal | `handle_history_modal()` |

## UI Rendering

### Main Screen Layout

```
┌──────────────────────────────────────────────────────────────────┐
│ Queue - Task Scheduler                                           │
├────────┬──────────────────────┬───────────────┬────────┬─────────┤
│ ID     │ Command              │ Scheduled     │ Status │ Target  │
├────────┼──────────────────────┼───────────────┼────────┼─────────┤
│ 1      │ make build           │ 10:00 AM      │ Pending│ Pane    │
│ 2      │ pytest               │ In 15m        │ Running│ Window  │
└────────┴──────────────────────┴───────────────┴────────┴─────────┘
│ [N]ew  [E]dit  [R]emove  [X]Cancel  [H]istory  [Q]uit            │
└──────────────────────────────────────────────────────────────────┘
```

### Status Styling

| Status | Display | Style |
|--------|---------|-------|
| Pending | "Pending" | Default |
| Running | "Running" | Yellow |
| Completed | "Done" | Green |
| Cancelled | "Cancelled" | Dimmed |
| Failed | "Failed" | Red |

## Testing

```bash
cargo test -p queue-cli
cargo test -p queue-cli -- --nocapture
```

**Test Coverage:**

- App state management
- Mode transitions
- Input validation
- Event handling
- Keyboard shortcuts
- Cursor positioning
- Modal rendering
