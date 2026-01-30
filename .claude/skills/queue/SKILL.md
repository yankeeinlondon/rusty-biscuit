---
name: queue
description: TUI-based command scheduler for queuing jobs. Use when working with queue package, implementing task scheduling, terminal detection, async execution, or ratatui-based TUI modal forms.
---

# Queue Command Scheduler

A TUI-based command scheduler that queues jobs for later execution with intelligent terminal detection for optimal execution.

## Architecture

```
queue/
├── lib/    # Core library: types, persistence, execution, terminal detection
└── cli/    # TUI application: interface, event handling, modals
```

**Key Pattern:** queue-lib handles all core logic; queue-cli is purely presentation via ratatui.

## Quick Reference

### Core Types (queue-lib)

| Type | Purpose |
|------|---------|
| `ScheduledTask` | Task with id, command, scheduled_at, target, status, schedule_kind |
| `ExecutionTarget` | `NewPane`, `NewWindow`, `Background` |
| `TaskStatus` | `Pending`, `Running`, `Completed`, `Cancelled`, `Failed { error }` |
| `ScheduleKind` | `AtTime`, `AfterDelay` - affects WHEN column display |
| `TaskExecutor` | Async scheduling engine with tokio |
| `JsonFileStore` | JSONL persistence at `~/.queue-history.jsonl` |
| `TerminalCapabilities` | Detected terminal features (panes, windows) |

### Terminal Detection Priority

1. Wezterm (`WEZTERM_PANE`) - panes + windows
2. iTerm2 (`ITERM_SESSION_ID`) - panes + windows
3. Ghostty (`TERM_PROGRAM=ghostty`) - windows only
4. GNOME Terminal - windows only
5. Konsole - windows only
6. Xfce4 Terminal - windows only
7. Terminal.app - windows only
8. XTerm - windows only
9. Unknown - background only

### CLI Usage

```bash
queue                           # Open TUI
queue --at 7:00am "command"     # Pre-schedule at time
queue --in 15m "command"        # Pre-schedule after delay
queue --debug                   # Enable logging to ~/.queue-debug.log
```

### TUI Keyboard Shortcuts

| Mode | Key | Action |
|------|-----|--------|
| Normal | `N` | New task |
| Normal | `E` | Edit selected |
| Normal | `X` | Cancel pending |
| Normal | `H` | History modal |
| Normal | `Q` | Quit (confirm if active tasks) |
| Modal | `Tab` | Next field |
| Modal | `Enter` | Submit |
| Modal | `Esc` | Cancel |

## Detailed Documentation

- [Library API Reference](./lib-api.md) - Data types, persistence, execution, terminal detection
- [TUI Architecture](./tui-architecture.md) - App state, modals, event handling, rendering
- [Wezterm Integration](./wezterm-integration.md) - Split pane workflow details

## Common Tasks

### Adding a New Terminal Type

1. Add variant to `TerminalKind` in `lib/src/terminal.rs`
2. Add detection method `is_<terminal>()` checking env vars
3. Update `detect_kind()` with proper priority order
4. Add capability flags in `TerminalCapabilities::for_kind()`
5. Add execution method in `executor.rs` `execute_in_window()`
6. Add tests for detection and capabilities

### Adding a New Modal Field

1. Add field enum variant in `input_modal.rs`
2. Update `next_field()` / `prev_field()` cycle order
3. Add rendering in `render_full()` / `render_compact()`
4. Add input handling in `event.rs` `handle_input_modal()`
5. Update `validate()` for new field validation

### Task Lifecycle

```
schedule_task() → Pending → [wait for scheduled_at]
                         → Running → Completed | Failed
                         → Cancelled (via cancel_task)
```

Events flow through `TaskEvent::StatusChanged` via mpsc channel.
