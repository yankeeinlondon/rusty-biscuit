# queue-lib

Core library for the Queue command scheduler. Provides data types, persistence, task execution, and terminal detection.

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
}
```

**Methods:**

| Method | Description |
|--------|-------------|
| `new(id, command, scheduled_at, target)` | Create a new pending task |
| `mark_running()` | Transition status to Running |
| `mark_completed()` | Transition status to Completed |
| `mark_cancelled()` | Transition status to Cancelled |
| `mark_failed(error)` | Transition status to Failed with error message |
| `is_pending()` | Check if status is Pending |
| `is_running()` | Check if status is Running |
| `is_completed()` | Check if status is Completed |
| `is_cancelled()` | Check if status is Cancelled |
| `is_failed()` | Check if status is Failed |

### ExecutionTarget

Where the command executes.

| Variant | Description | Serialization |
|---------|-------------|---------------|
| `NewPane` | Creates a new Wezterm pane (default in Wezterm) | `"new_pane"` |
| `NewWindow` | Opens in a native terminal window | `"new_window"` |
| `Background` | Runs detached, no terminal output | `"background"` |

### TaskStatus

Task lifecycle states.

| Variant | Description | Serialization |
|---------|-------------|---------------|
| `Pending` | Waiting to execute (default) | `{"status": "pending"}` |
| `Running` | Currently executing | `{"status": "running"}` |
| `Completed` | Finished successfully | `{"status": "completed"}` |
| `Cancelled` | Cancelled before execution | `{"status": "cancelled"}` |
| `Failed { error }` | Failed with error message | `{"status": "failed", "error": "..."}` |

---

## Persistence

### HistoryStore Trait

Abstract interface for task persistence.

```rust
pub trait HistoryStore {
    fn load_all(&self) -> Result<Vec<ScheduledTask>, HistoryError>;
    fn save(&self, task: &ScheduledTask) -> Result<(), HistoryError>;
    fn update(&self, task: &ScheduledTask) -> Result<(), HistoryError>;
}
```

### JsonFileStore

JSONL file-based implementation with cross-platform file locking.

**Default Path:** `~/.queue-history.jsonl`

**Format:** One JSON object per line (JSONL)

**Deduplication:** When saving, entries with the same command are de-duplicated and the most recent entry is preserved.

```json
{"id":1,"command":"echo hello","scheduled_at":"2024-01-15T10:00:00Z","target":"new_pane","status":{"status":"completed"},"created_at":"2024-01-15T09:55:00Z"}
{"id":2,"command":"make build","scheduled_at":"2024-01-15T11:00:00Z","target":"new_window","status":{"status":"pending"},"created_at":"2024-01-15T10:30:00Z"}
```

**Methods:**

| Method | Description |
|--------|-------------|
| `new(path: PathBuf)` | Create store at custom path |
| `default_path()` | Create store at `~/.queue-history.jsonl` |
| `path(&self)` | Get current file path |
| `ensure_file_exists()` | Create file if missing |

**File Locking:**

| Operation | Lock Type | Behavior |
|-----------|-----------|----------|
| `load_all()` | Shared | Multiple readers allowed |
| `save()` | Exclusive | Appends new task |
| `update()` | Exclusive | Rewrites entire file with updated task |

### HistoryError

Error types for persistence operations.

| Variant | Description |
|---------|-------------|
| `Read(io::Error)` | File I/O failures |
| `Parse(serde_json::Error)` | JSON parsing failures |
| `Lock` | File lock acquisition failures |

---

## Task Execution

### TaskExecutor

Async task scheduling engine using tokio.

**Responsibilities:**
- Spawn background tasks for each scheduled command
- Wait until `scheduled_at` time before executing
- Execute commands in the appropriate environment
- Send status updates through mpsc channel

**Methods:**

| Method | Description |
|--------|-------------|
| `new(event_tx)` | Create executor with event channel |
| `set_task_pane_id(pane_id)` | Set Wezterm task pane (async) |
| `set_task_pane_id_sync(pane_id)` | Set pane during initialization (sync) |
| `schedule(task)` | Schedule task for execution |
| `cancel_task(task_id)` | Cancel a scheduled task before it starts |

### TaskEvent

Status notifications from the executor.

```rust
pub enum TaskEvent {
    StatusChanged { id: u64, status: TaskStatus },
}
```

### Execution Flow

1. `schedule(task)` spawns a tokio background task
2. Task sleeps until `scheduled_at` time
3. Emits `StatusChanged(Running)` event
4. Executes command based on `ExecutionTarget`
5. Emits `StatusChanged(Completed)` or `StatusChanged(Failed)` event

### Execution Methods

| Target | Method | Behavior |
|--------|--------|----------|
| `NewPane` | `execute_in_pane()` | Creates Wezterm split pane; falls back to window |
| `NewWindow` | `execute_in_window()` | Opens terminal-specific window (see table below) |
| `Background` | `execute_background()` | Spawns detached process via `/bin/sh -c` |

**Interactive Shell Persistence:**

For `NewPane` and `NewWindow` targets, commands are wrapped to keep the terminal alive after execution. The pattern `command; exec $SHELL` runs the user's command then starts an interactive shell, allowing:
- User interaction with command output
- Follow-up commands in the same terminal
- Natural exit when the user closes the terminal

**Terminal-Specific Window Commands:**

| Terminal | Method |
|----------|--------|
| Wezterm | `wezterm cli spawn` |
| Ghostty | `ghostty -e` (spawned, non-blocking) |
| iTerm2 | AppleScript via `osascript` |
| Terminal.app | AppleScript via `osascript` |
| GNOME Terminal | `gnome-terminal -- /bin/sh -c` |
| Konsole | `konsole -e /bin/sh -c` |
| Xfce4 Terminal | `xfce4-terminal -e` |
| XTerm | `xterm -e /bin/sh -c` |
| Unknown (macOS) | Terminal.app via AppleScript |
| Unknown (Linux) | xterm, fallback to `x-terminal-emulator` |

---

## Terminal Detection

### TerminalKind

Detected terminal types.

| Variant | Environment Variable | Priority |
|---------|---------------------|----------|
| `Wezterm` | `WEZTERM_PANE` | 1 (highest) |
| `ITerm2` | `ITERM_SESSION_ID` | 2 |
| `Ghostty` | `TERM_PROGRAM=ghostty` | 3 |
| `GnomeTerminal` | `GNOME_TERMINAL_SCREEN` or `VTE_VERSION` | 4 |
| `Konsole` | `KONSOLE_VERSION` | 5 |
| `Xfce4Terminal` | `COLORTERM=xfce4-terminal` | 6 |
| `TerminalApp` | `TERM_PROGRAM=Apple_Terminal` | 7 |
| `Xterm` | `TERM=xterm*` | 8 |
| `Unknown` | (fallback) | 9 (lowest) |

### TerminalCapabilities

Capability flags based on detected terminal.

```rust
pub struct TerminalCapabilities {
    pub kind: TerminalKind,
    pub supports_panes: bool,
    pub supports_new_window: bool,
}
```

**Capability Matrix:**

| Terminal | Panes | Window |
|----------|:-----:|:------:|
| Wezterm | Yes | Yes |
| iTerm2 | Yes | Yes |
| Ghostty | No | Yes |
| Terminal.app | No | Yes |
| GNOME Terminal | No | Yes |
| Konsole | No | Yes |
| Xfce4 Terminal | No | Yes |
| XTerm | No | Yes |
| Unknown | No | No |

### TerminalDetector

Static methods for terminal detection.

| Method | Description |
|--------|-------------|
| `detect()` | Full capability detection |
| `detect_kind()` | Just the terminal type |
| `is_wezterm()` | Check if running in Wezterm |
| `is_iterm2()` | Check if running in iTerm2 |
| `get_wezterm_pane_id()` | Get current Wezterm pane ID |
| `setup_tui_layout()` | Create split layout for TUI (Wezterm only) |
| `create_task_pane()` | Create new pane for task execution |

### TuiLayoutResult

Result of Wezterm layout setup.

```rust
pub struct TuiLayoutResult {
    pub tui_pane_id: Option<String>,   // Bottom max(12 rows, 20%)
    pub task_pane_id: Option<String>,  // Top ~80%
    pub layout_created: bool,
}
```

---

## Parsing Utilities

### parse_at_time(value: &str) -> Result<NaiveTime, String>

Parses time strings for the `--at` flag.

**Supported Formats:**

| Format | Example | Result |
|--------|---------|--------|
| 12-hour with minutes | `7:00am`, `11:30pm` | 07:00, 23:30 |
| 12-hour without minutes | `7am`, `11pm` | 07:00, 23:00 |
| 24-hour | `19:30`, `07:00` | 19:30, 07:00 |

**Features:**
- Case-insensitive (`7AM` = `7am`)
- Whitespace trimmed
- Returns `NaiveTime` (0:00 - 23:59)

### parse_delay(value: &str) -> Result<chrono::Duration, String>

Parses delay strings for the `--in` flag.

**Supported Units:**

| Unit | Example | Duration |
|------|---------|----------|
| (none) | `15` | 15 minutes |
| `s` | `30s` | 30 seconds |
| `m` | `5m` | 5 minutes |
| `h` | `2h` | 2 hours |
| `d` | `1d` | 1 day |

**Validation:**
- Must start with a number
- Must be > 0
- Units must be `s`, `m`, `h`, or `d`

---

## Public Exports

```rust
// Data types
pub use types::{ScheduledTask, ExecutionTarget, TaskStatus};

// Execution
pub use executor::{TaskExecutor, TaskEvent};

// Persistence
pub use history::{HistoryStore, JsonFileStore, HistoryError};

// Terminal detection
pub use terminal::{
    TerminalDetector, TerminalCapabilities, TerminalKind, TuiLayoutResult
};

// Parsing
pub use parse::{parse_at_time, parse_delay};
```

---

## Testing

```bash
# Run all library tests
cargo test -p queue-lib

# Run specific test module
cargo test -p queue-lib --lib types::tests
cargo test -p queue-lib --lib history::tests
cargo test -p queue-lib --lib terminal::tests
cargo test -p queue-lib --lib parse::tests
```

**Test Coverage:**
- Data type serialization/deserialization (JSONL format)
- Status transitions (Pending → Running → Completed/Failed)
- File persistence with locking
- Terminal detection priority order
- Time/delay parsing edge cases
