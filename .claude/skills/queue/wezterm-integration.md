# Wezterm Integration

Detailed reference for Queue's Wezterm split pane workflow.

## Split Layout Architecture

When running in Wezterm, Queue creates an optimized split layout:

```
┌──────────────────────────────────────────────────────────────────┐
│                                                                  │
│                   Task Execution Area (~80%)                     │
│                    Commands run here in splits                   │
│                                                                  │
├──────────────────────────────────────────────────────────────────┤
│            TUI Control Pane (max(12 rows, 20%))                  │
│                    Schedule and monitor tasks                    │
└──────────────────────────────────────────────────────────────────┘
```

**Key Design:** The TUI stays in a small bottom pane while tasks execute in the larger top area. Each task gets its own split pane in the task area.

## Startup Sequence

### Detection Phase

1. User runs `queue` command
2. CLI checks `WEZTERM_PANE` environment variable
3. If present, Wezterm workflow activates

### Layout Creation (spawn_tui_in_split_pane)

1. Get current pane ID (becomes task execution pane)
2. Calculate TUI pane height: `max(12 rows, 20% of terminal)`
3. Run `wezterm cli split-pane --bottom --cells N`
4. Spawn `queue --tui-pane` with `QUEUE_TASK_PANE_ID` env var
5. Focus the new TUI pane
6. Original shell remains in top pane, interactive

### Internal Flag: --tui-pane

Hidden CLI flag that indicates "already in TUI pane, don't split again".

```rust
#[arg(long, hide = true)]
tui_pane: bool,
```

Prevents infinite recursion of pane splits.

## Task Execution in Wezterm

### NewPane Target Flow

1. Executor receives scheduled task with `ExecutionTarget::NewPane`
2. Gets `task_pane_id` from executor state (set during startup)
3. Runs: `wezterm cli split-pane --top --pane-id {task_pane_id} -- /bin/sh -c "{command}; exec $SHELL"`
4. New pane appears in task area (above TUI)
5. Multiple tasks create multiple splits

### Pane ID Management

```rust
pub struct TaskExecutor {
    task_pane_id: Arc<RwLock<Option<String>>>,
    // ...
}
```

**Setting the pane ID:**

```rust
// During TUI initialization
if TerminalDetector::is_wezterm() {
    let parent_pane = std::env::var("QUEUE_TASK_PANE_ID").ok();
    let current_pane = TerminalDetector::get_wezterm_pane_id();
    let task_pane = parent_pane.or(current_pane);
    executor.set_task_pane_id_sync(task_pane);
}
```

### Command Wrapping

Commands are wrapped to keep the shell alive:

```rust
fn wrap_command_for_interactive_shell(command: &str) -> String {
    let shell = std::env::var("SHELL").unwrap_or_else(|_| "/bin/sh".to_string());
    format!("{}; exec {}", command, shell)
}
```

**Result:** User can interact with command output and run follow-up commands.

## Wezterm CLI Commands Used

### split-pane

Create new pane by splitting an existing one:

```bash
# Create TUI pane (bottom)
wezterm cli split-pane --bottom --cells 12 --pane-id {parent} -- env QUEUE_TASK_PANE_ID={parent} queue --tui-pane

# Create task pane (top, in task area)
wezterm cli split-pane --top --pane-id {task_area} -- /bin/sh -c "command; exec $SHELL"
```

### activate-pane

Focus a specific pane:

```bash
wezterm cli activate-pane --pane-id {pane_id}
```

### spawn

Create new tab (used for NewWindow target):

```bash
wezterm cli spawn -- /bin/sh -c "command; exec $SHELL"
```

## Non-Wezterm Behavior

When not in Wezterm:

1. `setup_tui_layout()` returns `TuiLayoutResult::default()` (no split)
2. TUI runs fullscreen in current terminal
3. Tasks execute per their target:
   - `NewPane` falls back to `NewWindow`
   - `NewWindow` uses terminal-specific commands
   - `Background` spawns detached process

## Terminal-Specific Window Commands

| Terminal | Command |
|----------|---------|
| Wezterm | `wezterm cli spawn` |
| Ghostty | `ghostty -e` (spawned, non-blocking) |
| iTerm2 | AppleScript via `osascript` |
| Terminal.app | AppleScript via `osascript` |
| GNOME Terminal | `gnome-terminal --` |
| Konsole | `konsole -e` |
| Xfce4 Terminal | `xfce4-terminal -e` |
| XTerm | `xterm -e` |
| Unknown (macOS) | Terminal.app fallback |
| Unknown (Linux) | xterm, then `x-terminal-emulator` |

## Environment Variables

| Variable | Purpose |
|----------|---------|
| `WEZTERM_PANE` | Indicates running in Wezterm (pane ID) |
| `QUEUE_TASK_PANE_ID` | Passed to TUI subprocess for task pane targeting |
| `SHELL` | User's shell for command wrapping |
| `LINES` | Terminal height for pane size calculation |

## Debugging

Enable debug logging:

```bash
queue --debug
```

**Log Location:** `~/.queue-debug.log`

**Logged Events:**

- Terminal detection results
- Pane split operations
- Task scheduling events
- Executor status changes
- Error conditions

## Troubleshooting

| Issue | Solution |
|-------|----------|
| Tasks not splitting | Check `WEZTERM_PANE` env var present |
| TUI takes full screen | Verify not running with `--tui-pane` directly |
| Panes appear wrong place | Check `QUEUE_TASK_PANE_ID` is set correctly |
| Commands exit immediately | Check shell wrapping (`; exec $SHELL`) |

## Implementation Files

| File | Responsibility |
|------|----------------|
| `cli/src/main.rs` | `spawn_tui_in_split_pane()` |
| `lib/src/terminal.rs` | `setup_tui_layout()`, `create_task_pane()` |
| `lib/src/executor.rs` | `execute_in_pane()`, pane ID management |
