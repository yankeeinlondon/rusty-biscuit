# queue-cli

Interactive TUI application for the Queue command scheduler. Built with ratatui for terminal rendering and crossterm for event handling.

## Command-Line Interface

### Usage

```bash
queue [OPTIONS] [COMMAND]
```

### Arguments

| Argument | Description |
|----------|-------------|
| `COMMAND` | Shell command to schedule (required if `--at` or `--in` provided) |

### Options

| Flag | Description |
|------|-------------|
| `--at TIME` | Schedule for specific time (conflicts with `--in`) |
| `--in DELAY` | Schedule after delay (conflicts with `--at`) |
| `--debug` | Enable debug logging to `~/.queue-debug.log` |
| `--version` | Display version and exit |
| `--help` | Display help and exit |

**Internal Options (Hidden):**

| Flag | Description |
|------|-------------|
| `--tui-pane` | Run TUI in current pane (used by Wezterm split workflow) |

### Examples

```bash
# Open TUI with empty task list
queue

# Schedule task for 7:00am
queue --at 7:00am "make build"

# Schedule task in 15 minutes
queue --in 15m "echo 'reminder'"

# Schedule with debug logging
queue --debug --in 30s "pytest"
```

---

## TUI Architecture

### Module Structure

```
cli/src/tui/
├── mod.rs           # Module exports
├── app.rs           # Application state (App, AppMode)
├── event.rs         # Event loop and input routing
├── input_modal.rs   # Task creation/editing form
├── history_modal.rs # History browser
├── modal.rs         # Modal infrastructure (Modal trait)
└── render.rs        # UI rendering
```

---

## Application State

### AppMode

Controls keyboard behavior and UI rendering.

| Mode | Description | Exit Keys |
|------|-------------|-----------|
| `Normal` | Standard navigation | - |
| `EditMode` | Modifying selected task | Esc, Enter |
| `RemoveMode` | Confirming task removal | Esc, Y/N |
| `InputModal` | Entering new/editing task | Esc, Enter |
| `HistoryModal` | Viewing completed tasks | Esc, Enter |
| `ConfirmQuit` | Awaiting Y/N to exit | Y, N, Esc |

### App

Main application state.

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
}
```

**Key Methods:**

| Method | Description |
|--------|-------------|
| `new()` | Create default app, detect terminal capabilities |
| `with_executor()` | Add executor and event channel |
| `schedule_task(task)` | Add task and schedule with executor |
| `cancel_task(id)` | Remove pending task by ID |
| `select_next()` / `select_previous()` | Navigate task list |
| `selected_task()` | Get currently selected task |
| `handle_task_event(event)` | Process executor status updates |

---

## Keyboard Shortcuts

### Normal Mode

| Key | Action |
|-----|--------|
| `q` / `Q` | Quit (with confirmation) |
| `Esc` | Quit immediately |
| `n` / `N` | New task (opens input modal) |
| `e` / `E` | Edit selected task |
| `r` / `R` | Remove mode |
| `x` / `X` | Cancel selected pending task |
| `h` / `H` | History (opens history modal) |
| `↑` / `k` | Select previous task |
| `↓` / `j` | Select next task |

### Input Modal

| Key | Action |
|-----|--------|
| `Tab` | Next field |
| `Shift+Tab` | Previous field |
| `Enter` | Submit form (if valid) |
| `Esc` | Cancel and close |
| `Space` | Toggle selector fields |
| `←` / `→` | Move cursor or toggle selectors |
| `Backspace` | Delete character |
| Characters | Input text |

### History Modal

| Key | Action |
|-----|--------|
| `↑` / `k` | Select previous |
| `↓` / `j` | Select next |
| `Enter` | Use selected command |
| `n` / `N` | Create new task from selected |
| `f` / `F` / `/` | Filter mode |
| `Esc` | Close modal |

### Confirm Quit

| Key | Action |
|-----|--------|
| `y` / `Y` / `Enter` | Confirm exit |
| `n` / `N` / `Esc` | Cancel, return to normal mode |

---

## Input Modal

### Fields

The input modal contains four fields navigated with Tab/Shift+Tab.

| Field | Type | Description |
|-------|------|-------------|
| Command | Text input | Shell command to execute |
| Schedule Type | Selector | `At Time` or `After Delay` |
| Schedule Value | Text input | Time (e.g., `7:00am`) or delay (e.g., `15m`) |
| Target | Selector | Where to execute (cycles with Space) |

### InputField Enum

```rust
pub enum InputField {
    Command,      // Text: shell command
    ScheduleType, // Selector: At Time / After Delay
    ScheduleValue,// Text: time or delay string
    Target,       // Selector: NewPane / NewWindow / Background
}
```

### ScheduleType Enum

```rust
pub enum ScheduleType {
    AtTime,     // Run at specific time
    AfterDelay, // Run after delay
}
```

### Target Cycling

The Target field cycles only through supported options based on terminal capabilities.

| Terminal | Available Targets |
|----------|-------------------|
| Wezterm | NewPane → NewWindow → Background |
| iTerm2 | NewPane → NewWindow → Background |
| Terminal.app | NewWindow → Background |
| GNOME Terminal | NewWindow → Background |
| Other GUI | NewWindow → Background |
| Unknown | Background only |

### Default Target Selection

| Terminal | Default Target |
|----------|----------------|
| Wezterm | `NewPane` |
| iTerm2 | `NewPane` |
| GUI terminals | `NewWindow` |
| Unknown | `Background` |

### Validation

Form validation runs on submit (Enter key).

| Field | Validation Rules |
|-------|------------------|
| Command | Must not be empty |
| Schedule Value | Must not be empty |
| Schedule Value (At Time) | Must match time format: `7:00am`, `19:30`, etc. |
| Schedule Value (After Delay) | Must match delay format: `15`, `30s`, `2h`, etc. |

Validation errors display in red at the bottom of the modal.

### Modal Dimensions

- **Width:** 70% of terminal width
- **Height:** 60% of terminal height
- **Title:** "New Task" or "Edit Task" (based on context)

---

## History Modal

### Functionality

- Loads all tasks from `~/.queue-history.jsonl` on open
- Displays command, scheduled time, and status
- Case-insensitive substring filtering
- Selection persists across filter changes (resets to first match)

### HistoryModal State

```rust
pub struct HistoryModal {
    pub items: Vec<ScheduledTask>,
    pub list_state: ListState,
    pub filter: String,
    pub filter_mode: bool,
}
```

### Filtering

1. Press `f`, `F`, or `/` to enter filter mode
2. Type to search (case-insensitive, matches command substring)
3. Press `Esc` or `Enter` to exit filter mode
4. Selection resets to first matching item

### Actions

| Action | Behavior |
|--------|----------|
| Select (Enter) | Populate command field in new input modal |
| New Task (N) | Create new task with selected command |

---

## Modal Infrastructure

### Modal Trait

```rust
pub trait Modal {
    fn render(&self, frame: &mut Frame, area: Rect);
    fn title(&self) -> &str;
    fn width_percent(&self) -> u16 { 60 }
    fn height_percent(&self) -> u16 { 40 }
}
```

### Rendering

Modals render as centered overlays:

1. Calculate centered area from percentages
2. Clear underlying content (essential for overlay effect)
3. Draw border with title
4. Delegate content rendering to `Modal::render()`

### Built-in Modals

| Modal | Width | Height | Purpose |
|-------|-------|--------|---------|
| InputModal | 70% | 60% | Create/edit tasks |
| HistoryModal | 60% | 60% | Browse command history |
| ConfirmQuitDialog | 40% | 20% | Confirm exit |

---

## Event Loop

### run_app(terminal, app) -> io::Result<()>

Main application loop in `event.rs`.

**Loop Steps:**

1. Render current frame
2. Check for task events via `try_recv()` (non-blocking)
3. Poll for keyboard input (50ms timeout)
4. Route input to mode-specific handler
5. Repeat until `should_quit` is true

### Event Processing

```rust
// Non-blocking task event collection
while let Ok(event) = event_rx.try_recv() {
    app.handle_task_event(event);
}

// Keyboard input with timeout
if poll(Duration::from_millis(50))? {
    if let Event::Key(key) = read()? {
        // Route to mode handler
    }
}
```

### Mode Handlers

| Mode | Handler Function |
|------|------------------|
| Normal | `handle_normal_mode(app, key)` |
| ConfirmQuit | `handle_confirm_quit(app, key)` |
| InputModal | `handle_input_modal(app, key, modifiers)` |
| HistoryModal | `handle_history_modal(app, key)` |

---

## Wezterm Split Pane Workflow

### Startup Sequence (Wezterm)

1. User runs `queue` command
2. CLI detects Wezterm via `WEZTERM_PANE` env var
3. Creates split layout:
   - **Top 80%:** Original pane (task execution area)
   - **Bottom 20%:** New pane for TUI
4. Spawns `queue --tui-pane` in bottom pane
5. Passes task pane ID to executor

### Task Execution (Wezterm)

1. Executor receives scheduled task
2. Creates new split in task pane area (top)
3. Task runs in new pane
4. TUI remains visible in bottom pane
5. Multiple tasks create multiple splits in top area

### Non-Wezterm Behavior

- No split layout created
- TUI runs in current terminal
- Tasks execute in new windows or background

---

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
│ 3      │ echo "done"          │ 11:30 AM      │ Done   │ Background│
└────────┴──────────────────────┴───────────────┴────────┴─────────┘
│ [N]ew  [E]dit  [R]emove  [X]Cancel  [H]istory  [Q]uit            │
└──────────────────────────────────────────────────────────────────┘
```

### Task Table Columns

| Column | Content |
|--------|---------|
| ID | Task ID number |
| Command | Shell command (truncated if needed) |
| Scheduled | Time or relative delay |
| Status | Pending / Running / Completed / Failed |
| Target | Pane / Window / Background |

### Status Styling

| Status | Display | Style |
|--------|---------|-------|
| Pending | "Pending" | Default |
| Running | "Running" | Yellow |
| Completed | "Done" | Green |
| Cancelled | "Cancelled" | Dimmed |
| Failed | "Failed" | Red |

---

## Debugging

### Debug Logging

Enable with `--debug` flag:

```bash
queue --debug
```

**Log Location:** `~/.queue-debug.log`

**Log Contents:**
- Terminal detection results
- Task scheduling events
- Executor status changes
- Error conditions

### Troubleshooting

| Issue | Solution |
|-------|----------|
| Tasks not executing | Check `~/.queue-debug.log` for errors |
| Panes not splitting | Verify Wezterm is detected (`WEZTERM_PANE` env var) |
| Wrong default target | Terminal may not be detected; use explicit target |
| History not loading | Check `~/.queue-history.jsonl` file permissions |

---

## Public Exports

```rust
pub use tui::App;
pub use tui::run_app;
```

---

## Testing

```bash
# Run all CLI tests
cargo test -p queue-cli

# Run with output
cargo test -p queue-cli -- --nocapture
```

**Test Coverage:**
- App state management
- Mode transitions
- Input validation
- Event handling
- Keyboard shortcuts
