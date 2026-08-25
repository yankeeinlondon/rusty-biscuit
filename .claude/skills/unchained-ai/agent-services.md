# Agent Services

Detailed reference for the agent status detection system in `unchained-ai/lib/src/primitives/services/` and the `unchained` CLI.

## Module Structure

```
primitives/services/
├── mod.rs              # Module declarations
├── agent_status.rs     # Core types and detection logic
├── error.rs            # AgentStatusError enum
├── parsers.rs          # Platform-specific output parsers
└── pty_runner.rs       # PTY command execution with ANSI stripping
```

## Core Types (`agent_status.rs`)

### AgenticStatusPlatform

```rust
#[non_exhaustive]
pub enum AgenticStatusPlatform {
    Codex,
    ClaudeCode,
}
```

Implements `Display` for human-readable names ("Claude Code", "Codex").

### CapWindowSize

```rust
pub enum CapWindowSize {
    Hours(u8),    // e.g., Hours(5) -> "5 hours"
    Daily,
    Weekly,
    Monthly,
}
```

### AgenticCapLimit

```rust
pub struct AgenticCapLimit {
    pub short_cap_usage: f32,              // 0.0-1.0 percentage
    pub short_cap_until: Datetime,         // Reset time
    pub short_cap_window_size: CapWindowSize,
    pub long_cap_usage: f32,               // 0.0-1.0 percentage
    pub long_cap_window_size: CapWindowSize,
    pub long_cap_until: Datetime,          // Reset time
}
```

Derives `Serialize` for JSON output in the CLI.

### AgentStatus

```rust
pub struct AgentStatus {
    pub installed: HashMap<AgenticStatusPlatform, bool>,
    pub available: Vec<AgenticStatusPlatform>,
    pub limits: HashMap<String, AgenticCapLimit>,
}
```

**Key methods**:
- `new(keys: Option<AgenticStatusApiKeys>)` - Detects platforms via `sniff::programs::InstalledAiClients`
- `limits(platform: Option<AgenticStatusPlatform>)` - Queries cap limits via PTY commands
- `status(platform: Option<AgenticStatusPlatform>)` - Combined detection + limits query

### AgenticStatusApiKeys

```rust
pub struct AgenticStatusApiKeys {
    pub claude_code_api_key: Option<String>,
    pub codex_api_key: Option<String>,
}
```

## Error Types (`error.rs`)

```rust
pub enum AgentStatusError {
    PtySpawnError(String),
    PtyReadError(String),
    ParseError(String),
    TimeoutError(String),
    PlatformNotInstalled(String),
    UnsupportedPlatform(String),
}
```

## PTY Runner (`pty_runner.rs`)

Executes CLI commands in a pseudo-terminal to capture output that may differ when not running in a TTY.

```rust
pub async fn run_pty_command(
    program: &str,
    args: &[&str],
    timeout_duration: Option<Duration>,
) -> Result<String, AgentStatusError>
```

**Implementation**:
- Wraps blocking PTY ops in `tokio::task::spawn_blocking`
- Uses `xpty` through its `portable_pty`-compatible API with 24x80 terminal size
- Drops the slave PTY immediately after spawning
- Gives short-lived children time to attach before closing PTY input on macOS
- Lets fast Windows commands exit during a bounded ConPTY attachment grace
- Sends cooked-mode console EOF to Windows commands still running after that grace
- Waits for the child before closing the master PTY so ConPTY readers receive EOF
- Lets ConPTY's asynchronous output pump become quiet before closing the master
- Reads output via `mpsc::channel` in separate thread
- Default timeout is 5 seconds on Unix and 10 seconds on Windows (configurable)
- Strips ANSI escape codes via `strip-ansi-escapes`
- Validates UTF-8 encoding

## Parsers (`parsers.rs`)

Platform-specific output parsers extract usage percentages from status commands.

### Dispatcher

```rust
pub fn parse_status_output(
    platform: AgenticStatusPlatform,
    output: &str,
) -> Result<AgenticCapLimit, AgentStatusError>
```

### Claude Code Parser

Spawns `claude /status`, extracts from keywords:
- Short-term: `["usage:", "cap usage:"]` -> `CapWindowSize::Hours(5)`
- Long-term: `["monthly:", "long term:"]` -> `CapWindowSize::Monthly`

### Codex Parser

Spawns `codex --status`, extracts from keywords:
- Short-term: `["usage:", "rate limit:"]` -> `CapWindowSize::Hours(4)`
- Long-term: `["monthly:", "long term:"]` -> `CapWindowSize::Weekly`

### Percentage Extraction

`extract_percentage(text, keywords)` - Case-insensitive keyword search, finds first `N%` pattern after keyword match. Returns raw percentage (0-100), not normalized 0.0-1.0.

## CLI Binary (`cli/`)

### `unchained limits`

```bash
# Show limits for all detected platforms
unchained limits

# Filter to a specific platform
unchained limits --platform claude
unchained limits --platform codex

# Output as JSON
unchained limits --json
```

**Terminal rendering**: Uses `biscuit-terminal::components::progress::Progress` for visual bars showing short-term and long-term cap usage.

**JSON rendering**: Structured output with `short_cap`/`long_cap` nested objects per platform.

### Global flags

- `--json` - JSON output mode
- `-v`/`-vv`/`-vvv` - Verbosity (tracing levels)
- `--completions <SHELL>` - Generate shell completions (bash, zsh)

## Dependencies

| Crate | Purpose |
|-------|---------|
| `xpty` v0.3 | PTY spawning without ConPTY cursor inheritance hangs |
| `strip-ansi-escapes` v0.2 | ANSI code stripping from PTY output |
| `sniff` (workspace) | Platform detection (`InstalledAiClients`) |
| `biscuit-terminal` (workspace) | Progress bar rendering in CLI |
| `clap` v4 + `clap_complete` v4 | CLI argument parsing + shell completions |

## Testing

- **Unit tests** in each module: parser extraction, PTY echo commands, error handling
- **Integration tests** (`cli/tests/cli.rs`): help, version, limits help, invalid platform, completions
- PTY tests use real commands (`echo`), so they exercise actual terminal interaction
