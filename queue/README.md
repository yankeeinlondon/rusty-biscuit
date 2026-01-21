# Queue

A TUI-based command scheduler that queues jobs for later execution. Schedule commands to run at specific times or after delays, with intelligent terminal detection for optimal execution.

## Quick Start

```bash
# Install
cargo install --path cli

# Open TUI with empty task list
queue

# Schedule a task for 7:00am
queue --at 7:00am "make build"

# Schedule a task in 15 minutes
queue --in 15m "echo 'reminder'"
```

## Architecture

Queue consists of two crates with distinct responsibilities:

```text
queue/
├── lib/    # Core library: data types, persistence, execution, terminal detection
└── cli/    # TUI application: interface, event handling, modals
```

### queue-lib

The core library provides:

- **Data Types** - `ScheduledTask`, `ExecutionTarget`, `TaskStatus`
- **Persistence** - JSONL file storage with cross-platform locking
- **Execution** - Async task scheduling with tokio
- **Terminal Detection** - Capability detection for 8 terminal types

See [lib/README.md](lib/README.md) for detailed API documentation.

### queue-cli

The TUI application provides:

- **Interactive Interface** - ratatui-based terminal UI
- **Modal Forms** - Task creation, editing, and history browsing
- **Event Loop** - Non-blocking input handling with 50ms responsiveness
- **Wezterm Integration** - Split pane workflow for seamless task viewing

See [cli/README.md](cli/README.md) for detailed TUI documentation.

## Key Features

### Terminal-Aware Execution

Commands execute in the most appropriate environment based on detected terminal:

| Terminal       | Default Target | Pane Support |
|----------------|----------------|:------------:|
| Wezterm        | New Pane       | Yes          |
| iTerm2         | New Pane       | Yes          |
| Terminal.app   | New Window     | No           |
| GNOME Terminal | New Window     | No           |
| Konsole        | New Window     | No           |
| XTerm          | New Window     | No           |
| Unknown        | Background     | No           |

### Wezterm Split Workflow

When running in Wezterm, Queue creates an optimized layout:

```text
┌──────────────────────────────────────────────────────────────────┐
│                                                                  │
│                   Task Execution Area (~80%)                    │
│                    Commands run here in splits                   │
│                                                                  │
├──────────────────────────────────────────────────────────────────┤
│            TUI Control Pane (max(12 rows, 20%))                  │
│                    Schedule and monitor tasks                    │
└──────────────────────────────────────────────────────────────────┘
```

### Flexible Scheduling

Schedule commands with times or delays:

```bash
# Time formats
queue --at 7:00am "command"    # 12-hour
queue --at 19:30 "command"     # 24-hour

# Delay formats
queue --in 15 "command"        # 15 minutes (default)
queue --in 30s "command"       # 30 seconds
queue --in 2h "command"        # 2 hours
queue --in 1d "command"        # 1 day
```

## TUI Overview

### Main Screen

| Key | Action         |
|-----|----------------|
| `N` | New task       |
| `E` | Edit selected  |
| `X` | Cancel pending |
| `H` | View history   |
| `Q` | Quit           |

### Input Modal

| Key     | Action          |
|---------|-----------------|
| `Tab`   | Next field      |
| `Enter` | Submit          |
| `Space` | Toggle selector |
| `Esc`   | Cancel          |

See [cli/README.md](cli/README.md) for complete keyboard reference.

## Data Storage

Tasks persist to `~/.queue-history.jsonl` in JSONL format:

Duplicate commands are de-duplicated, keeping the most recent entry.

```json
{"id":1,"command":"make build","scheduled_at":"2024-01-15T10:00:00Z","target":"new_pane","status":{"status":"completed"},"created_at":"2024-01-15T09:55:00Z"}
```

## Development

```bash
# Build
cargo build -p queue-cli
cargo build -p queue-lib

# Test
cargo test -p queue-cli
cargo test -p queue-lib

# Lint
cargo clippy -p queue-lib -p queue-cli

# Debug mode (logs to ~/.queue-debug.log)
queue --debug
```

## Detailed Documentation

| Document                       | Contents                                                  |
|--------------------------------|-----------------------------------------------------------|
| [lib/README.md](lib/README.md) | Data types, persistence API, executor, terminal detection |
| [cli/README.md](cli/README.md) | TUI architecture, keyboard shortcuts, modal system        |
