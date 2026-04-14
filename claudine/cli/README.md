# Claudine CLI

Binary: `claudine` — interactive setup, hook inspection, event handling, shared-resource management (skills, slash commands, agents), MCP management, provider wrapping, and composition pipelines for agentic CLIs.

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

Process an incoming event from a provider hook (hidden from help). Reads JSON payload from stdin, auto-detects the provider from payload structure (or accepts `--provider` override), resolves environment context, and dispatches through the event pipeline.

### `claudine actions`

Show which actions are configured and for which events across the user and repo configs.

### `claudine skills`

List shared skills across providers with their scopes. Displays link/sync state per resource using the four-type linking model (Skill, Command, Agent, Script). Replaces the retired `claudine link skills` subcommand.

### `claudine commands`

List slash commands across providers with their scopes and link/sync state.

### `claudine agents`

List agent/subagent definitions across providers with their scopes and link/sync state.

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

### `claudine` (no subcommand) or `claudine --help`

Renders rich help documentation grouped by category (Shared Resources, Hook Events and Actions, Wrapped Execution, Composition, Administration) using biscuit-terminal Prose rendering. This replaces the retired `claudine about` command.

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
- **Structured streaming**: non-interactive runs use provider-native structured output (stream-json, JSONL, NDJSON) as the internal control plane. Claudine deserializes each line into a strongly typed `*Event` enum from `claudine::stream::protocol` (one module per provider), reconstructs clean assistant text for stdout, and emits metadata summaries to stderr. Unknown event types fall through to a silent skip so provider format drift never turns into a hard failure.
- **Stderr summaries**: session-start info (session ID, model), completion summary (duration, tokens, cost, tool calls), and verbose details (tools used, turns, stop reason).
- **Stderr status lines**: `LiveSemanticSink` renders tool/subagent/info/warning/error status lines and `ProviderExtension` fall-throughs. Summaries always prefer a human-readable preview (nested-text walker: `message`, `status`, `content.parts[*].text`, ...) and never fall back to truncated raw JSON — when nothing readable is available the line collapses to `provider/kind` with no payload tail. A small silent-kind allowlist (Claude `stream_event`, `hook_started`, `hook_response`, `hook_progress`) suppresses the status line entirely for redundant extension events; dispatch and JSONL logging still happen. The OpenCode wrapper additionally suppresses OpenCode's default-mode TUI formatter output on stderr (lines starting with `✱ `, `$ `, `> build `, `████ `). See [Non-Interactive Sessions](../docs/topics/non-interactive-sessions.md) for the full rules.
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

### Composition commands

Composition turns a Markdown document with frontmatter into a provider session, optionally merging the result back into the source file or running a sequence of steps. All three commands reuse the wrapper pipeline (env setup, harness detection, structured streaming, handler-driven recovery).

- **`claudine compose [flags] <arg>...`** — compose a Markdown file (Darkmatter transclusion/interpolation/conditionals/`::shell`) and send the result as a prompt. No file mutation.
- **`claudine inline-compose [flags] <arg>...`** — compose the frontmatter `prompt` property and replace the document body with the provider's response. Original frontmatter is preserved byte-for-byte; `last_updated` is set to today's date; new frontmatter keys added by the provider are merged in.
- **`claudine sequence [flags] <arg>...`** — run a serial sequence of composition steps declared in a single document, with a shared approval cache across steps and `FAIL_FAST` propagation on failure.

Positional `<arg>...` is exactly one file reference plus zero or more `key=value` setters in any order:

```sh
claudine compose @prompts/review.md review=review.md
claudine inline-compose draft=false @notes/update.md
claudine sequence @research.md topic="async traits" retries=3
```

Setter values are parsed as JSON5 first and fall back to strings. Inline setters override `--set` on overlapping keys; `sequence` reserved overlay keys still win over both.

Shared composition flags include provider selectors (`--claude`, `--codex`, `--gemini`, `--opencode`, `--qwen`, `--goose`, `--kimi`), `--exclude <provider>`, `-i` / `--interactive`, `-m` / `--model`, `-s` / `--system-prompt`, `-t` / `--timeout`, `--dry-run`, `-q` / `--quiet`, and `--silent`. The file reference supports `@` magic paths, repo-relative, monorepo-package-relative, and absolute paths via `biscuit-file::FileReference`.

Provider selection precedence: explicit flag → single installed remaining after `--exclude` → `agent` frontmatter hint (fuzzy-matched) → `settings.linking.preference[0]` config favorite → interactive chooser (TTY only).

## Module Structure

```
cli/src/
├── main.rs                → Entry point, clap parser, command dispatch
├── args.rs                → Cli struct and Commands subcommand enum
├── log.rs                 → Output formatting (message/data/info/warn/error)
├── output.rs              → Execution line, badges, env details, prompt display
├── telemetry.rs           → Tracing subscriber configuration and root span
├── cli_utils.rs           → Shared CLI helpers
├── provider_values.rs     → Provider enum value parsing and fuzzy matching
├── table_utils.rs         → Shared table rendering helpers
└── commands/
    ├── help.rs            → Rich grouped help rendering (replaces `about`)
    ├── completions.rs     → Shell completion generation
    ├── handle.rs          → Event processing from stdin (hidden)
    ├── hooks.rs           → Hook inspection and validation
    ├── actions.rs         → Configured actions per event
    ├── skills.rs          → Shared skills listing with link state
    ├── agents.rs          → Shared agent definitions listing with link state
    ├── slash_commands.rs  → Shared slash commands listing with link state
    ├── link_display.rs    → Shared rendering helpers for link state tables
    ├── logs.rs            → JSONL log reporting queries
    ├── mcp.rs             → MCP catalog, defaults, aliasing, import, and sync commands
    ├── providers.rs       → Provider capability matrix (skill/slash/agent/hooks)
    ├── sync.rs            → Hook re-registration
    ├── uninstall.rs       → Hook removal
    ├── compose.rs         → `compose` and `inline-compose` command entry points
    ├── sequence.rs        → `sequence` command entry point
    ├── wrap/
    │   ├── mod.rs         → Shared wrapper pipeline, args, interactivity, stream summary
    │   ├── profile.rs     → Provider mapping profiles (yolo/non-interactive/model/output)
    │   ├── env.rs         → Env sanitization + injected context vars
    │   ├── exec.rs        → Child process execution, structured stream capture, timeout
    │   ├── composition.rs → Composition preparation, shell approval, closure write-back
    │   ├── sequence.rs    → Per-step sequence execution loop with shared approval cache
    │   ├── system_prompt.rs → System prompt resolution and wrapper injection
    │   └── repo_home.rs   → Shadow HOME for repo-scoped resource isolation
    └── init/
        ├── mod.rs         → Wizard orchestration (interactive + quick modes, default configs)
        └── prompts.rs     → inquire-based interactive prompts
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
| `claudine` (lib) | Core event model, dispatch, config, linking, composition, harness |
| `clap` + `clap_complete` | CLI parsing and shell completions |
| `biscuit-terminal` | Rich terminal output (tables, prose, lists) |
| `biscuit-file` | File reference resolution (`@` magic paths) for composition commands |
| `darkmatter` | Markdown rendering and composition pipeline (transclusion, interpolation, `::shell`) |
| `playa` | Sound effect names for validation |
| `sniff` | AI client detection for agent discovery |
| `inquire` | Interactive multi-select prompts for `init` wizard |
| `color-eyre` | Error reporting with backtraces |
| `tokio` | Async runtime for event handling |

## Lessons Learned

- **Provider fuzzy matching**: commands that accept a provider name use a 3-tier resolution: exact match → prefix match → contains match. This lets users type `cl` instead of `claude`.
- **Event name normalization in handle**: the event parser handles canonical snake_case, native provider names (e.g., `Stop` for Claude's `turn_complete`), PascalCase, kebab-case, and is case-insensitive. This makes hook wiring resilient.
- **Sound effect suggestion engine**: Sound effect validation runs automatically when viewing hooks and uses 5 matching heuristics (exact, normalized, prefix, contains, Levenshtein-like) to suggest replacements for invalid effect names.
- **Stdin auto-detection in handle**: the provider is detected from JSON payload structure (`hook_event_name` → Claude, `type` + `thread_id` → Codex, etc.) so hooks don't need to pass `--provider` explicitly.
- **Composition reuses the wrapper**: `compose`, `inline-compose`, and `sequence` all flow through the same execution pipeline as `claudine claude`/`codex`/... — including env sanitization, system prompt resolution, structured streaming, harness pre/post checks, and synthetic JSONL summary events. A shared approval cache lets `sequence` approve shell commands once per run.
