# Queue Library API Reference

Detailed reference for `queue-lib` - the core library for the Queue command scheduler.

## Data Types

### ScheduledTask

The primary data structure representing a scheduled command.

```rust
pub struct ScheduledTask {
    pub id: u64,
    pub command: String,
    pub scheduled_at: DateTime<Utc>,
    pub target: ExecutionTarget,
    pub status: TaskStatus,
    pub created_at: DateTime<Utc>,
    pub schedule_kind: Option<ScheduleKind>,
}
```

**Constructors:**

| Method | Description |
|--------|-------------|
| `new(id, command, scheduled_at, target)` | Create task (schedule_kind = None) |
| `with_schedule_kind(id, command, scheduled_at, target, kind)` | Create with explicit schedule kind |

**Status Transitions:**

| Method | Description |
|--------|-------------|
| `mark_running()` | Transition to Running |
| `mark_completed()` | Transition to Completed |
| `mark_cancelled()` | Transition to Cancelled |
| `mark_failed(error)` | Transition to Failed with message |

**Status Checks:** `is_pending()`, `is_running()`, `is_completed()`, `is_cancelled()`, `is_failed()`

### ExecutionTarget

Where the command executes.

```rust
pub enum ExecutionTarget {
    NewPane,      // Wezterm split pane (default)
    NewWindow,    // Native terminal window
    Background,   // Detached process
}
```

**Serialization:** `"new_pane"`, `"new_window"`, `"background"`

### TaskStatus

Task lifecycle states.

```rust
pub enum TaskStatus {
    Pending,                    // {"status": "pending"}
    Running,                    // {"status": "running"}
    Completed,                  // {"status": "completed"}
    Cancelled,                  // {"status": "cancelled"}
    Failed { error: String },   // {"status": "failed", "error": "..."}
}
```

### ScheduleKind

How the task was scheduled - affects WHEN column display.

```rust
pub enum ScheduleKind {
    AtTime,      // Shows clock time until <1min, then countdown
    AfterDelay,  // Always shows countdown
}
```

**Note:** `None` for backwards compatibility with old tasks (treated as AfterDelay).

## Persistence

### HistoryStore Trait

```rust
pub trait HistoryStore {
    fn load_all(&self) -> Result<Vec<ScheduledTask>, HistoryError>;
    fn save(&self, task: &ScheduledTask) -> Result<(), HistoryError>;
    fn update(&self, task: &ScheduledTask) -> Result<(), HistoryError>;
}
```

### JsonFileStore

JSONL file-based implementation with cross-platform file locking (fs2).

**Default Path:** `~/.queue-history.jsonl`

**Format:** One JSON object per line

**Deduplication:** Same command entries de-duplicated, keeping most recent.

**File Locking:**

| Operation | Lock Type | Behavior |
|-----------|-----------|----------|
| `load_all()` | Shared | Multiple readers |
| `save()` | Exclusive | Append with dedup |
| `update()` | Exclusive | Rewrite entire file |

### HistoryError

```rust
pub enum HistoryError {
    Read(io::Error),           // File I/O failures
    Parse(serde_json::Error),  // JSON parsing failures
    Lock,                      // Lock acquisition failures
}
```

## Task Execution

### TaskExecutor

Async task scheduling engine using tokio.

```rust
impl TaskExecutor {
    pub fn new(event_tx: mpsc::Sender<TaskEvent>) -> Self;
    pub async fn set_task_pane_id(&self, pane_id: Option<String>);
    pub fn set_task_pane_id_sync(&self, pane_id: Option<String>);
    pub fn schedule(&self, task: ScheduledTask);
    pub fn cancel_task(&self, task_id: u64) -> bool;
}
```

**Execution Flow:**

1. `schedule(task)` spawns tokio background task
2. Task sleeps until `scheduled_at` time
3. Emits `StatusChanged(Running)`
4. Executes based on `ExecutionTarget`
5. Emits `StatusChanged(Completed)` or `StatusChanged(Failed)`

### TaskEvent

```rust
pub enum TaskEvent {
    StatusChanged { id: u64, status: TaskStatus },
}
```

### Execution Methods

| Target | Method | Behavior |
|--------|--------|----------|
| `NewPane` | `execute_in_pane()` | Wezterm split; falls back to window |
| `NewWindow` | `execute_in_window()` | Terminal-specific window command |
| `Background` | `execute_background()` | Spawns `/bin/sh -c` detached |

**Interactive Shell Persistence:** Commands wrapped with `; exec $SHELL` to keep terminal alive after completion.

## Terminal Detection

### TerminalKind

```rust
pub enum TerminalKind {
    Wezterm,        // WEZTERM_PANE
    ITerm2,         // ITERM_SESSION_ID
    Ghostty,        // TERM_PROGRAM=ghostty
    GnomeTerminal,  // GNOME_TERMINAL_SCREEN or VTE_VERSION
    Konsole,        // KONSOLE_VERSION
    Xfce4Terminal,  // COLORTERM=xfce4-terminal
    TerminalApp,    // TERM_PROGRAM=Apple_Terminal
    Xterm,          // TERM=xterm*
    Unknown,
}
```

### TerminalCapabilities

```rust
pub struct TerminalCapabilities {
    pub kind: TerminalKind,
    pub supports_panes: bool,
    pub supports_new_window: bool,
}
```

**Capability Matrix:**

| Terminal | Panes | Windows |
|----------|:-----:|:-------:|
| Wezterm | Yes | Yes |
| iTerm2 | Yes | Yes |
| Others | No | Yes |
| Unknown | No | No |

### TerminalDetector

Static detection methods:

| Method | Description |
|--------|-------------|
| `detect()` | Full capability detection |
| `detect_kind()` | Just terminal type |
| `is_wezterm()` | Check WEZTERM_PANE |
| `is_iterm2()` | Check ITERM_SESSION_ID |
| `get_wezterm_pane_id()` | Get current pane ID |
| `setup_tui_layout()` | Create Wezterm split layout |
| `create_task_pane()` | Create new task pane |

### TuiLayoutResult

```rust
pub struct TuiLayoutResult {
    pub tui_pane_id: Option<String>,   // Bottom pane for TUI
    pub task_pane_id: Option<String>,  // Top pane for tasks
    pub layout_created: bool,
}
```

## Parsing Utilities

### parse_at_time

Parses time strings for `--at` flag.

| Format | Example | Result |
|--------|---------|--------|
| 12-hour with minutes | `7:00am` | 07:00 |
| 12-hour without minutes | `7am` | 07:00 |
| 24-hour | `19:30` | 19:30 |

Case-insensitive, whitespace trimmed.

### parse_delay

Parses delay strings for `--in` flag.

| Unit | Example | Duration |
|------|---------|----------|
| (none) | `15` | 15 minutes |
| `s` | `30s` | 30 seconds |
| `m` | `5m` | 5 minutes |
| `h` | `2h` | 2 hours |
| `d` | `1d` | 1 day |

**Validation:** Must start with number > 0, units must be s/m/h/d.

## Public Exports

```rust
// Data types
pub use types::{ScheduledTask, ExecutionTarget, TaskStatus, ScheduleKind};

// Execution
pub use executor::{TaskExecutor, TaskEvent};

// Persistence
pub use history::{HistoryStore, JsonFileStore, HistoryError};

// Terminal detection
pub use terminal::{TerminalDetector, TerminalCapabilities, TerminalKind, TuiLayoutResult};

// Parsing
pub use parse::{parse_at_time, parse_delay};
```

## Testing

```bash
cargo test -p queue-lib
cargo test -p queue-lib --lib types::tests
cargo test -p queue-lib --lib terminal::tests
```
