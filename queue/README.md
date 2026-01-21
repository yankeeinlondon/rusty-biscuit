# Queue

A TUI-based command scheduler that queues jobs for later execution.

## Installation

```sh
cargo install --path cli
```

## Usage

All invocations open an interactive TUI. You can optionally pre-schedule a task from the command line:

```sh
# Open the TUI with an empty task list
queue

# Open the TUI with a task scheduled for 7:00am
queue --at 7:00am "so-you-say 'good morning'"

# Open the TUI with a task scheduled in 15 minutes
queue --in 15m "echo 'reminder'"
```

### Flags

| Flag | Description |
|------|-------------|
| `--at TIME` | Pre-schedule a command for a specific time |
| `--in DELAY` | Pre-schedule a command after a delay |
| `--debug` | Enable debug logging to `~/.queue-debug.log` |
| `--version` | Display version and exit |
| `--help` | Display help and exit |

## TUI Keyboard Shortcuts

### Main Window

| Key | Action |
|-----|--------|
| `Q` | Quit (with confirmation) |
| `Esc` | Quit immediately |
| `N` | New task (opens input modal) |
| `E` | Edit selected task |
| `R` | Remove mode |
| `X` | Cancel selected pending task |
| `H` | History (opens history modal) |
| `↑` / `k` | Select previous task |
| `↓` / `j` | Select next task |

### Input Modal

| Key | Action |
|-----|--------|
| `Tab` | Next field |
| `Shift+Tab` | Previous field |
| `Enter` | Submit form |
| `Esc` | Cancel |
| `Space` | Toggle selector fields |
| `←` / `→` | Move cursor / toggle selectors |

### History Modal

| Key | Action |
|-----|--------|
| `↑` / `k` | Select previous |
| `↓` / `j` | Select next |
| `Enter` | Use selected command |
| `N` | Create new task from selected |
| `F` / `/` | Filter mode |
| `Esc` | Close modal |

## Time Formats

The `--at` flag accepts:

- 12-hour format: `7:00am`, `11:30pm`
- 24-hour format: `19:30`, `07:00`

## Delay Formats

The `--in` flag accepts delays with optional units:

| Unit | Example | Description |
|------|---------|-------------|
| (none) | `15` | 15 minutes (default) |
| `s` | `30s` | 30 seconds |
| `m` | `5m` | 5 minutes |
| `h` | `2h` | 2 hours |
| `d` | `1d` | 1 day |

## Execution Targets

Tasks can execute in different environments:

- **Pane**: Opens in a new Wezterm pane (default in Wezterm)
- **Window**: Opens in a new terminal window (Terminal.app, iTerm2, etc.)
- **Background**: Runs detached (no terminal output)

## Architecture

The queue package is split into two crates:

- **queue-lib**: Core library with data types, persistence, and execution
- **queue-cli**: TUI application and CLI interface

## Development

```sh
# Build
cargo build -p queue-cli

# Test
cargo test -p queue-lib
cargo test -p queue-cli

# Lint
cargo clippy -p queue-lib -p queue-cli

# Generate documentation
cargo doc -p queue-lib -p queue-cli --no-deps
```
