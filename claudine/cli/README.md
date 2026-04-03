# Claudine CLI

Binary: `claudine` — interactive setup, hook inspection, event handling, skill linking, and MCP management for agentic CLIs.

## Commands

### `claudine init [--quick] [--repo]`

Interactive setup wizard that walks through 4 phases:

1. **Agent Discovery** — detects installed agentic CLIs on the system
2. **Provider Preferences** — rank your favorite installed CLIs for canonical ordering
3. **Action Defaults** — global interview (logging `all/some/none`, then input-needed actions)
4. **Write & Register** — saves `~/.claudine/config.json` and registers hooks with each provider

Setup automatically configures all detected available agents (no per-agent selection prompt). Claudine auto-configures every event each provider supports via native hooks. Events with no actions are still registered as explicit no-op bindings.

`--quick` skips prompts and registers all hook-supported events for installed providers, with default sounds for `session_start`, `turn_complete`, `tool_error`, `permission_request`, and `human_in_the_loop`. `--repo` creates `.claudine/config.json` in the repository root and can add `.claudine/` to `.gitignore`.

### `claudine hooks [provider] [flags]`

Inspect hook registrations and provider capabilities.

| Flag | Description |
|------|-------------|
| *(none)* | Table of providers with install status and subscribed hooks |
| `-v` | Adds action count indicators per event |
| `<provider>` | Detailed event/action view for one provider (fuzzy matching) |
| `--support` | Event support matrix across all providers (✅ hook / ⛔️ non-hook / ❌ none) |
| `--mapping` | Native event name mappings per provider |
| `--describe` | Event descriptions, payload schemas, and return schemas |
| `--variables` | All 28 template variables with current detected values |

Sound effect validation runs automatically when viewing hooks and uses a 5-tier fuzzy matching algorithm to suggest replacements for invalid effect names.

### `claudine providers`

Show a compact provider capability matrix with:

- Provider name as an OSC8 link to provider documentation
- `Skill` support (custom skill definitions)
- `Slash` support (custom slash command definitions)
- `Agent` support (custom agent/subagent definitions)
- `Hooks` count (how many native hook events Claudine can attach to)

### `claudine sync [flags]`

Re-apply hook registrations to match the current config.

| Flag | Description |
|------|-------------|
| `--dry-run` | Preview what would be added/removed per provider |
| `--provider <name>` | Sync only a specific provider |
| `--fix` | Remove unsupported events from config (cleans up warnings) |

### `claudine handle <event> [--provider <name>]`

Process an incoming event from a provider hook. Reads JSON payload from stdin, auto-detects the provider from payload structure (or accepts `--provider` override), resolves environment context, and dispatches through the event pipeline.

### `claudine dry-run <event> [--provider <name>]`

Test what would happen for an event without side effects. Accepts event names in multiple formats: canonical (`turn_complete`), native (`Stop`), PascalCase (`TurnComplete`), kebab-case (`turn-complete`) — all case-insensitive. When no stdin is provided, generates realistic mock payloads for the selected provider.

### `claudine logs [subcommand] [flags]`

Query the local reporting index built from JSONL hook logs. Shared filters include `--provider`, `--repo`, `--package-area`, and `--package`, and read commands perform a best-effort sync before querying. Time-window commands also accept nested error drill-downs such as `claudine logs week errors` and `claudine logs today errors`.

### `claudine mcp [subcommand] [--json]`

Manage Claudine's normalized MCP catalog and provider sync state.

| Subcommand | Description |
|------------|-------------|
| *(none)* | List catalog entries, defaults, and provider presence |
| `init` | Import supported native provider MCP configs into `~/.claudine/mcp/` |
| `show <id>` | Show normalized definition and provenance for one server |
| `default [ids...]` | Replace user-scope default server IDs |
| `default --repo [ids...]` | Replace repo-scope default server IDs |
| `alias add <id> <alias>` | Add a catalog alias |
| `alias remove <alias>` | Remove an alias |
| `remove <id>` | Remove a catalog entry after confirmation |
| `sync <provider> [--scope user\|repo] [--apply]` | Dry-run or apply export of effective defaults to a provider's native config |

Storage lives in `~/.claudine/mcp/catalog.json`, `~/.claudine/mcp/defaults.json`, `~/.claudine/mcp/provider-state.json`, and optional repo defaults at `<repo>/.claudine/mcp.json`. Repo defaults replace user defaults.

### `claudine about`

Renders rich help documentation using darkmatter markdown rendering with biscuit-terminal fallback.

### `claudine completions <shell>`

Generate shell completions for bash, zsh, fish, powershell, or elvish.

### `claudine uninstall [--keep-config]`

Remove hook registrations from all detected agents. `--keep-config` preserves `~/.claudine/config.json` while removing only the hook registrations.

### Wrapped provider commands

Claudine can wrap provider CLIs with preflight checks, argument translation, and environment sanitization:

- `claudine claude`
- `claudine codex`
- `claudine gemini`
- `claudine kimi`
- `claudine qwen`
- `claudine opencode`
- `claudine goose`

Shared wrapper flags:

| Flag | Description |
|------|-------------|
| `-y, --yolo` | Translate to provider-specific auto-approval mode (warn-only for OpenCode) |
| `-i, --interactive` | Force interactive mode even when a prompt string is provided |
| `-m, --model <MODEL>` | Override the model used by the provider |
| `-s, --system-prompt <PROMPT\|FILE>` | Set or append a system prompt (string or file path) |
| `-t, --timeout <SECONDS>` | Timeout in seconds (non-interactive only) |
| `-o, --output <FORMAT>` | Set output format (json, text, stream) |
| `--include <ENV_NAME>` | Keep a sensitive env var name that would otherwise be filtered |
| `--mcp` | Compose a Claudine-managed MCP session from the effective defaults |
| `--use <ID[,ID...]>` | Add specific MCP catalog IDs or aliases and enable MCP composition |
| `--sandbox` | Enable provider-specific sandboxing |
| `--repo` | Use only repo-scoped skills, commands, and agents via a shadow HOME |
| `--dry-run` | Show what would be executed without launching the child |
| `-q, --quiet` | Show only the header line; suppress env details |
| `--silent` | Suppress all Claudine preflight output |
| `-- ...` | Force all remaining args to passthrough unchanged |

Wrapper behavior:

- **Interactivity default**: providing a prompt string implies non-interactive mode. Use `-i`/`--interactive` to override back to interactive when providing a startup prompt.
- **Execution line**: displays `Claudine ▸ {provider} {badges} {prompt}` — only the user's prompt text is shown (provider-specific switches are not leaked). Truncated to one terminal line.
- **Structured streaming**: non-interactive runs use provider-native structured output (stream-json, JSONL, NDJSON) as the internal control plane. Claudine parses the stream live, reconstructs clean assistant text for stdout, and emits metadata summaries to stderr.
- **Stderr summaries**: session-start info (session ID, model), completion summary (duration, tokens, cost, tool calls), and verbose details (tools used, turns, stop reason).
- **Verbosity**: `--quiet` shows only a compact completion line; `--silent` suppresses all Claudine output; `-v` adds detailed human-facing metadata on the second summary line.
- **Diagnostics**: `--debug <level>` controls Claudine tracing (`trace`, `debug`, `info`, `warn`, `error`). `RUST_LOG` takes precedence and supports per-module targeting such as `RUST_LOG=claudine::dispatch=trace,claudine::stream=debug`.
- Validates provider binary availability before spawn (with provider docs URL in errors).
- Filters sensitive env vars whose names contain `API_KEY` or `TOKEN` unless explicitly included.
- Reports removed env variable names to stderr (names only, sorted/unique).
- Injects `AGENT`, `YOLO`, `INTERACTIVE`, `AGENT_PARAMS`, `CLAUDINE_SESSION_ID`, and, when resolvable in monorepos, `PACKAGE_AREA` and `PACKAGE`.
- `claudine handle` records wrapper-provided `PACKAGE_AREA` / `PACKAGE` values into event logs so they can be used in reporting filters.
- `--mcp` resolves repo defaults if `<repo>/.claudine/mcp.json` exists, otherwise user defaults; `--use` appends explicit IDs or aliases and also enables MCP mode.
- Non-interactive Codex, Gemini, and OpenCode runs also strip catalog-resolvable `#tags` from the prompt and activate the matching servers.
- Runtime MCP injection currently exists for Codex, Gemini, and OpenCode only. Codex and Gemini use a shadow HOME under `~/.claudine`; OpenCode injects `OPENCODE_CONFIG_CONTENT`.
- Gemini runtime sessions append `--allowed-mcp-server-names` for the resolved server list.
- Claude, Goose, Kimi, and Qwen wrappers fail fast with guidance to use `claudine mcp sync <provider>` instead. Roo is import/sync only and has no wrapper command.
- Runs child process with inherited stdio/cwd and propagates child exit code.
- Writes a synthetic JSONL summary event per session for reporting completeness.

## Module Structure

```
cli/src/
├── main.rs              → Entry point, clap parser, command dispatch
├── log.rs               → Output formatting (message/data/info/warn/error)
├── output.rs            → Execution line, badges, env details, prompt display
└── commands/
    ├── about.rs         → Rich help rendering
    ├── completions.rs   → Shell completion generation
    ├── dry_run.rs       → Event simulation with mock payloads
    ├── handle.rs        → Event processing from stdin
    ├── hooks.rs         → Hook inspection and validation
    ├── logs.rs          → JSONL log reporting queries
    ├── mcp.rs           → MCP catalog, defaults, aliasing, import, and sync commands
    ├── providers.rs     → Provider capability matrix (skill/slash/agent/hooks)
    ├── sync.rs          → Hook re-registration
    ├── uninstall.rs     → Hook removal
    ├── wrap/
    │   ├── mod.rs       → Shared wrapper pipeline, args, interactivity logic, stream summary
    │   ├── profile.rs   → Provider mapping profiles (yolo/non-interactive/model/output)
    │   ├── env.rs       → Env sanitization + injected context vars
    │   ├── exec.rs      → Child process execution, structured stream capture, timeout
    │   ├── prompt_file.rs → Prompt file resolution and Darkmatter composition
    │   └── repo_home.rs → Shadow HOME for repo-scoped resource isolation
    └── init/
        ├── mod.rs       → Wizard orchestration (interactive + quick modes, default configs)
        └── prompts.rs   → inquire-based interactive prompts
```

## Output System

All user-facing output goes through `log.rs`:

| Function | Target | Purpose |
|----------|--------|---------|
| `message()` | stderr | Always visible (status messages) |
| `data()` | stdout | Pipeable data output |
| `output()` | stdout | Inline output (no trailing newline) |
| `info()` | stderr | Only when verbosity enabled |
| `warn()` | stderr | Yellow "warning:" prefix |
| `error()` | stderr | Red "Error:" prefix (with leading blank line) |

Rich formatting uses biscuit-terminal components (Table, Prose with `{{bold}}` / `{{cyan}}` / `{{dim}}` markup, UnorderedList, OSC8 hyperlinks).

## Key Dependencies

| Crate | Purpose |
|-------|---------|
| `claudine` (lib) | Core event model, dispatch, config, linking |
| `clap` + `clap_complete` | CLI parsing and shell completions |
| `biscuit-terminal` | Rich terminal output (tables, prose, lists) |
| `darkmatter` | Markdown rendering for `about` command |
| `playa` | Sound effect names for validation |
| `sniff` | AI client detection for agent discovery |
| `inquire` | Interactive multi-select prompts for `init` wizard |
| `color-eyre` | Error reporting with backtraces |
| `tokio` | Async runtime for event handling |

## Lessons Learned

- **Provider fuzzy matching**: commands that accept a provider name use a 3-tier resolution: exact match → prefix match → contains match. This lets users type `cl` instead of `claude`.
- **Event name normalization in dry-run**: the event parser handles canonical snake_case, native provider names (e.g., `Stop` for Claude's `turn_complete`), PascalCase, kebab-case, and is case-insensitive. This makes testing easier.
- **Sound effect suggestion engine**: Sound effect validation runs automatically when viewing hooks and uses 5 matching heuristics (exact, normalized, prefix, contains, Levenshtein-like) to suggest replacements for invalid effect names.
- **Stdin auto-detection in handle**: the provider is detected from JSON payload structure (`hook_event_name` → Claude, `type` + `thread_id` → Codex, etc.) so hooks don't need to pass `--provider` explicitly.
