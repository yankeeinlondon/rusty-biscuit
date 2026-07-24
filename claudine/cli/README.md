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

`claudine providers agent-errors check <slug> [--findings <path>]` runs the
deterministic research gate and writes an explicit outcome report. Fleet
lifecycle documents use this trusted CLI surface instead of invoking the
globally blacklisted `cargo` executable directly.

### `claudine signals [check]`

Inspect the compiled signal-detection tables. Bare `claudine signals` prints a per-provider summary (record/extraction counts, bespoke count, per-source breakdown) for all nine corpora, including the dormant kilo/pi tables.

`claudine signals check` is dev/CI-facing and requires a rusty-biscuit checkout: it replays every detection record's evidence fixture from `docs/research/signals/` through the production engine, verifying evidence existence, positive firing, extraction resolution, and per-source-group overlap (benign overlaps are declared in `docs/research/signals/_overlap-exclusions.yaml` and reported as INFO lines). Bespoke records report as skipped until their E5 emitters land.

| Flag | Description |
|------|-------------|
| `--provider <slug>` | Check a single provider corpus |
| `--json` | Emit the machine-readable check report |

**Unmatched-event harvest (opt-in).** With `harvest_unmatched: true` in `~/.claudine/config.json` (or `CLAUDINE_HARVEST=1`; `0`/`false` forces off, env wins over config), wrapped runs append error/warning-class payloads that fired no detection record to `~/.claudine/harvest/<provider>/<YYYY-MM-DD>.jsonl` as candidate detection evidence. Payloads are scrubbed (API-key/token shapes, key-named secrets, home paths, emails) before they are buffered, and the tree is retention-capped (30 days / 50 MiB, oldest first). Promotion into `docs/research/signals/fixtures/` remains human-reviewed (provenance class `capture`).

### `claudine sync [flags]`

Re-apply hook registrations to match the current config.

| Flag | Description |
|------|-------------|
| `--dry-run` | Preview what would be added/removed per provider |
| `--provider <name>` | Sync only a specific provider |
| `--fix` | Remove unsupported events from config (cleans up warnings) |

### `claudine handle <event> [--provider <name>]`

Process an incoming event from a provider hook (hidden from help). Reads JSON payload from stdin, auto-detects the provider from payload structure (or accepts `--provider` override), resolves environment context, and dispatches through the event pipeline.

**Execution Deadline.** To prevent hook handlers from blocking the parent agent session, `claudine handle` enforces a hard **5-second deadline** by default (overridable via `CLAUDINE_HANDLE_DEADLINE_SECONDS`). When exceeded, the handler aborts with a diagnostic message to stderr and exits 124. Individual bash and messenger actions also have tighter 3s timeouts when running inside a hook handler.

### `claudine actions`

Show which actions are configured and for which events across the user and repo configs.

### Dispatch Expressions

Hook action templates, conditional `when` clauses, event binding matchers, and lifecycle stack expressions all share a single expression evaluator backed by Darkmatter's parser (see `claudine/lib/src/dispatch/expression.rs`). The same paths exposed by `claudine hooks --variables` are available everywhere expressions are evaluated.

**Template Interpolation.** Speak messages, bash params, report templates, and message bodies pass through `{{...}}` interpolation. In addition to simple variable references, authors can use:

- Fallbacks: `{{env.CI || "local"}}`
- Ternaries: `{{git.is_dirty ? "dirty" : "clean"}}`
- Comparisons: `{{hardware.cores > 8 ? "fast" : "slow"}}`
- Helper functions: `{{length(git.branch) > 30 ? "long-branch" : git.branch}}`

Booleans render as `true`/`false`. Unknown bare-variable references like `{{unknown_field}}` are preserved verbatim so misspellings stay visible. Composite expressions that fail to parse are also preserved unchanged. Legacy single-brace placeholders such as `{tool_name}` are still rewritten to `{{tool_name}}` for one compatibility cycle and emit a deprecation warning.

**Action `when` Clauses.** Every hook action variant accepts an optional `when` boolean expression. When it evaluates to false (or fails to parse), the action is skipped without short-circuiting the rest of the binding. A skipped `Call` action cannot replace a previously selected blocking response.

```json
{ "type": "speak", "message": "Bash on {{git.branch}}", "when": "tool_name == 'Bash'" }
{ "type": "bash",  "command": "say done", "when": "git.is_dirty" }
{ "type": "call",  "command": "/usr/local/bin/check", "when": "provider == 'claude' && tool_name == 'Bash'" }
{ "type": "message", "message": "build red on {{git.branch}}", "when": "event == 'tool_error'" }
```

**Matcher Modes.** Event binding matchers accept either a Darkmatter condition or a regex (legacy mode). Matcher strings are first attempted as conditions; bare-variable strings like `Bash|Edit` deliberately fall through to regex compilation so historical configurations keep their original semantics. Examples:

| Matcher | Mode | Behavior |
|---------|------|----------|
| `Bash\|Edit` | Regex | Matches `tool_name` for tool events, `notification_type` for notifications |
| `tool_name == 'Bash' && git.branch == 'main'` | Expression | Evaluated against full `EventMeta` |
| `provider == 'claude' && !git.is_dirty` | Expression | Multi-field condition |
| `[invalid(regex` | Neither | Warned and skipped (binding fires unconditionally) |

Expression matchers that fail to evaluate at runtime log a warning and are treated as non-matches.

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
| `-t, --timeout <DURATION>` | Wall-clock timeout like 30s, 5m, 2h (non-interactive only) |
| `-o, --output <FORMAT>` | Set output format (json, text, stream) |
| `--include <ENV_NAME>` | Keep a sensitive env var name that would otherwise be filtered |
| `--mcp` | Compose a Claudine-managed MCP session from the effective defaults |
| `--use <ID[,ID...]>` | Add specific MCP catalog IDs or aliases and enable MCP composition |
| `--sandbox` | Enable provider-specific sandboxing |
| `--repo` | Use only repo-scoped skills, commands, and agents via a shadow HOME |
| `--dry-run` | Show what would be executed without requiring or launching the child executable |
| `--perf` | Emit a detailed performance report to stderr after execution |
| `-q, --quiet` | Show only the header line; suppress env details |
| `--silent` | Suppress all Claudine preflight output |
| `-- ...` | Force all remaining args to passthrough unchanged |

Wrapper behavior:

- **Dry-run executable independence**: provider wrappers render the planned
  command using the profile's executable name without resolving it on `PATH`;
  the provider need not be installed. Live runs still validate the executable
  before launch.
- **Interactivity default**: providing a prompt string implies non-interactive mode. Use `-i`/`--interactive` to override back to interactive when providing a startup prompt.
- **Execution line**: displays `Claudine ▸ {provider} {badges} {prompt}` — only the user's prompt text is shown (provider-specific switches are not leaked). Truncated to one terminal line.
- **Structured streaming**: non-interactive runs use provider-native structured output (stream-json, JSONL, NDJSON, or JSON-RPC 2.0 for Kimi) as the internal control plane. Claudine deserializes each line into a strongly typed `*Event` / `*Envelope` enum from `claudine::stream::protocol` (one module per provider), reconstructs clean assistant text for stdout, and emits metadata summaries to stderr. Every run follows a **9-section model** (execution line, env, system prompt, agent prompt, session ID, thinking prose, tool/info events, final STDOUT, and metadata) with strictly enforced spacing (at most one blank line between sections).
- **Kimi wire mode**: non-interactive `claudine kimi` and Kimi-resolved composition runs launch the Kimi child with `--wire` and drive a JSON-RPC 2.0 line transport (`claudine/cli/src/commands/wrap/wire_io.rs`). Claudine sends `initialize` (declaring `supports_question: false` and `supports_plan_mode: false`) and a `prompt` request whose `params.user_input` carries the resolved prompt body. `ApprovalRequest` envelopes are auto-approved and surface as visible `auto_approved` info lines; `QuestionRequest` envelopes (which should not arrive given the declared capabilities) are answered with empty synthetic answers and a warning; `ToolCallRequest` is rejected with `-32601 method not found`; `HookRequest` is forwarded through Claudine's existing dispatch pipeline. Cancellation (Ctrl+C / deadline) sends a `cancel` JSON-RPC request before tearing down the child. The legacy `--print --output-format stream-json` Kimi path was removed — that mode emitted OpenAI-shaped envelopes with no top-level `type` field, so every event silently dropped to `ProviderExtension`.
- **Thinking prose**: reasoning and thinking content from providers (Claude, Codex, OpenCode, etc.) is rendered on stderr in `Section::Thinking` as a `BlockQuote` with the wider `▌ ` border (matching System Prompt and Agent Prompt) and dim-italic gray text, ensuring continuous feedback during long turns. OpenCode reasoning (`{"type":"reasoning","text":"…"}`, including nested `part.text`) routes through `SemanticEvent::Reasoning` like every other provider rather than falling through `ProviderExtension`. Claude assistant prose that appears in the same `assistant` envelope as a `tool_use` is also promoted to `Reasoning`, so "Let me investigate..." tool-preface narration no longer leaks onto stdout or creates extra section breaks between tool calls.
- **Stderr status lines**: `LiveSemanticSink` renders tool/subagent/info/warning/error status lines. Tool calls use a canonical humanized contract — `→ {Name}({summary})` for outgoing and `← {Name}({slot})` for incoming — that reads like a function call. Shell tools (`Bash`, `bash`, `run_command`, Codex `shell`) prepend the canonical shell name to the command (`bash ls -la`) so the user can see how the line would actually execute. `Task` summaries prefer `description → subject → prompt → task` so the agent's task body wins over arbitrary fields like `subagent_type`. Unknown event types fall through to a silent skip so provider format drift never turns into a hard failure. Raw JSON is never dumped to the terminal for known tools.
- **Typed error blocks**: `SemanticEvent::Error` now carries a `SemanticErrorKind` (`Configuration`, `AgentNative`, `ApiRemote`, `Interrupted`, `Unknown`) and renders as a colored `BlockQuote` with `▌ ` border instead of a single failure status line. Border colors and labels are: orange `Configuration Error`, red `Agent Error`, red `API Error`, yellow `Interrupted`, red `Error`. Replays of older JSONL streams without a `kind` field default to `Unknown` via `#[serde(default)]`. The kind maps directly onto `AgentErrorCategory` for end-of-run reports via `From<SemanticErrorKind> for AgentErrorCategory`. Dispatch behavior remains keyed off `terminal: bool`; `kind` is classificatory metadata, not a new dispatch switch.
- **Idle output flush**: the lib `AssistantStream` streaming component records when the block buffer last grew. When the heartbeat thread runs, it calls `flush_idle(silence_window)` (default **30 s**) before emitting its own status line, so a dangling final paragraph from a slow-to-close provider becomes visible within the silence window even if the provider never closes stdout. Buffered content always appears above the next heartbeat.
- **Unified timeout watchdog** (OpenCode stability): the wrapper enforces exactly two timeouts via a single ticker — `timeout` (wall-clock budget from child spawn, opt-in, no default) and `step_timeout` (stream-silence since the last parent-stream event, default **30m**). Both can be set via CLI flags (`--timeout`, `--step-timeout`), markdown frontmatter (`timeout:`, `step_timeout:`), or env-var defaults (`CLAUDINE_TIMEOUT`, `CLAUDINE_STEP_TIMEOUT`); precedence is CLI > frontmatter > env > built-in. Setting an env var to `0s` disables that rule. Watchdog cadence and SIGTERM→SIGKILL grace are tunable via `CLAUDINE_WATCHDOG_INTERVAL` (default **5s**) and `CLAUDINE_KILL_GRACE` (default **10s**). On breach, the watchdog renders an `Agent Error` `BlockQuote` on stderr (enumerating any outstanding subagents for `step_timeout`) and the synthesised summary records `error_kind: "timeout"` or `"step_timeout"`. See [`docs/topics/timeouts.md`](../docs/topics/timeouts.md) for the canonical reference.
- **Verbosity**: `--quiet` shows only a compact completion line; `--silent` suppresses all Claudine output; `-v` adds detailed human-facing metadata on the second summary line.
- **Diagnostics**: `--debug <level>` controls Claudine tracing (`trace`, `debug`, `info`, `warn`, `error`). `RUST_LOG` takes precedence and supports per-module targeting such as `RUST_LOG=claudine::dispatch=trace,claudine::stream=debug`.
- Validates provider binary availability before spawn (with provider docs URL in errors).
- Filters sensitive env vars whose names contain `API_KEY` or `TOKEN` unless explicitly included.
- Reports removed env variable names to stderr (names only, sorted/unique).
- Injects `AGENT`, `YOLO`, `INTERACTIVE`, `AGENT_PARAMS`, `CLAUDINE_SESSION_ID`, `CLAUDINE_PID`, and, when resolvable in monorepos, `PACKAGE_AREA` and `PACKAGE`.
- `claudine handle` records wrapper-provided `PACKAGE_AREA` / `PACKAGE` values into event logs so they can be used in reporting filters.
- `--mcp` resolves repo defaults if `<repo>/.claudine/mcp.json` exists, otherwise user defaults; `--use` appends explicit IDs or aliases and also enables MCP mode.
- Non-interactive Codex, Gemini, and OpenCode runs also strip catalog-resolvable `#tags` from the prompt and activate the matching servers.
- Runtime MCP injection currently exists for Codex, Gemini, and OpenCode only. Codex and Gemini use a shadow HOME under `~/.claudine`; OpenCode injects `OPENCODE_CONFIG_CONTENT`.
- Gemini runtime sessions append `--allowed-mcp-server-names` for the resolved server list.
- Claude, Goose, Kimi, and Qwen wrappers fail fast with guidance to use `claudine mcp sync <provider>` instead. Roo is import/sync only and has no wrapper command.
- Runs child process with inherited stdio/cwd and propagates child exit code.
- Writes a synthetic JSONL summary event per session for reporting completeness.

### Composition commands

Composition turns a Markdown document with frontmatter into a provider session, optionally merging the result back into the source file or running a sequence of steps. All three commands reuse the wrapper pipeline (env setup, harness detection, structured streaming, lifecycle-stack recovery).

- **`claudine compose [flags] <file-ref> [key=value ...]`** — compose a Markdown file (Darkmatter transclusion/interpolation/conditionals/`::shell`) and send the result as a prompt. No file mutation.
- **`claudine inline-compose [flags] <file-ref> [key=value ...]`** — compose the frontmatter `prompt` property and replace the document body with the provider's response. Original frontmatter is preserved byte-for-byte; `last_updated` is set to today's date; new frontmatter keys added by the provider are merged in.
- **`claudine sequence [flags] <file-ref> [key=value ...]`** — run a serial sequence of composition steps declared in a single document, with a shared approval cache across steps and `FAIL_FAST` propagation on failure.

Positional arguments include exactly one file reference plus zero or more `key=value` setters in any order:

```sh
claudine compose @prompts/review.md review=review.md
claudine inline-compose draft=false @notes/update.md
claudine sequence @research.md topic="async traits" retries=3
```

Setter values are parsed as JSON5 first and fall back to strings. Inline setters override `--set` on overlapping keys; `sequence` reserved overlay keys still win over both.

Shared composition flags include provider selectors (`--claude`, `--codex`, `--gemini`, `--opencode`, `--qwen`, `--goose`, `--kimi`), `--exclude <provider>`, `-i` / `--interactive`, `--no-interactive`, `-m` / `--model`, `-s` / `--system-prompt`, `-t` / `--timeout`, `--dry-run`, `-q` / `--quiet`, and `--silent`. The file reference supports `@` magic paths, repo-relative, monorepo-package-relative, and absolute paths via `biscuit-file::FileReference`. Session mode resolves as `--no-interactive` > `--interactive` > `interactive: true` frontmatter > default (non-interactive); `--interactive` and `--no-interactive` are mutually exclusive, and `sequence` rejects `interactive: true` frontmatter.

Provider selection follows a TTY/non-TTY split. In **TTY mode** with no explicit flag, an interactive `biscuit-tui` picker is always shown. In **non-TTY mode**, resolution follows a strict chain: explicit flag → singular frontmatter `agent` → list-valued frontmatter `agent` (first installed match) → configured `favorite_agent` → structured hard error. The old "single installed" auto-selection shortcut has been removed.

Model resolution is independent of TTY mode: CLI `--model` → provider-specific env var (`CODEX_MODEL`, `CLAUDE_MODEL`, etc.) → generic `MODEL` env → frontmatter `model` → provider default. OpenCode requires a model in non-interactive mode and fails hard if none is resolved.

Set or clear the favorite agent with:

```sh
claudine config set favorite-agent codex
claudine config set favorite-agent none
```

**Unified Harness Execution (2026-06-16).** Every non-dry-run `compose` and `inline-compose` run now routes through `run_harness_loop` with `HarnessPromptMode::Compose` or `HarnessPromptMode::Inline`. Documents without harness frontmatter yield the empty/bare plan; the plan now carries only timeout configuration (the pre/post validation and handler-recovery DSL has been retired in favor of lifecycle stacks). The loop handles structured streaming, captured/non-structured fallback, inline closure, summary emission, and lifecycle-stack recovery (`Retry`/`Resume`/`Proxy`) through one code path.

**Performance Reporting.** All three composition commands support `--perf`, which emits a post-execution performance report to stderr. `sequence` produces a single aggregated report covering all steps; `compose` and `inline-compose` produce one report per invocation. See the main README for report layout details.

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

Rich formatting uses biscuit-terminal components (Table, Prose with `<b>` / `<cyan>` / `<dim>` bracketed-tag markup, UnorderedList, OSC8 hyperlinks).

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
