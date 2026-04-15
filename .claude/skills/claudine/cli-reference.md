# Claudine CLI Reference

Complete command documentation with examples and options.

## Global Options

| Option | Description |
|--------|-------------|
| `-v, --verbose` | Enable verbose output |
| `--version` | Print version information |
| `-h, --help` | Print help information |

Environment variables:
- `DEBUG=INFO` - Enable debug logging
- `HOME` - Used to resolve `~` paths

---

## `claudine about`

Display rich help documentation using darkmatter markdown rendering with biscuit-terminal fallback.

```bash
claudine about
```

---

## `claudine init`

Interactive setup wizard for initial configuration. Walks through 4 phases:

1. **Agent Discovery** — detects installed agentic CLIs on the system
2. **Provider Preferences** — rank your favorite installed CLIs for canonical ordering
3. **Action Defaults** — global interview (logging `all/some/none`, then input-needed actions)
4. **Write & Register** — saves `~/.claudine/config.json` and registers hooks with each provider

Setup automatically configures all detected available agents (no per-agent selection prompt). Claudine auto-configures every event each provider supports via native hooks. Events with no actions are still registered as explicit no-op bindings.

```bash
claudine init [OPTIONS]
```

| Option | Description |
|--------|-------------|
| `--quick` | Use defaults without prompting |
| `--repo` | Create project-scoped configuration |

**Quick Mode:**

```bash
claudine init --quick
```

Creates configuration with sensible defaults:
- All detected agents enabled
- `session_start` → SoundEffect (power-up)
- `turn_complete` → SoundEffect (success)
- `tool_error` → SoundEffect (error)
- `permission_request` → SoundEffect (notification)
- `human_in_the_loop` → SoundEffect (notification)

`--repo` creates `.claudine/config.json` in the repository root and can add `.claudine/` to `.gitignore`.

---

## `claudine hooks`

Inspect hook registrations and provider capabilities.

```bash
claudine hooks [OPTIONS] [PROVIDER]
```

| Option | Description |
|--------|-------------|
| *(none)* | Table of providers with install status and subscribed hooks |
| `-v` | Adds action count indicators per event |
| `<provider>` | Detailed event/action view for one provider (fuzzy matching) |
| `--support` | Event support matrix across all providers (✅ hook / ⛔️ non-hook / ❌ none) |
| `--mapping` | Native event name mappings per provider |
| `--describe` | Event descriptions, payload schemas, and return schemas |
| `--variables` | All 28 template variables with current detected values |

**Provider fuzzy matching**: commands that accept a provider name use a 3-tier resolution: exact match → prefix match → contains match. This lets users type `cl` instead of `claude`.

**Sound effect validation**: runs automatically when viewing hooks and uses a 5-tier fuzzy matching algorithm (exact, normalized, prefix, contains, Levenshtein-like) to suggest replacements for invalid effect names.

**Basic output:**

```
Provider    Installed  Subscribed Hooks
Claude      ✓          session_start, turn_complete, tool_error
Codex       ✓          turn_complete
Gemini      ✓          -
OpenCode    ✗          -
```

**Provider detail view:**

```bash
claudine hooks claude
```

Shows detailed event/action configuration for a specific provider.

**Support matrix:**

```bash
claudine hooks --support
```

Shows which events each provider supports:
- ✓ = Hook support (config file registration)
- ○ = NonHook support (wrapper/proxy required)
- - = Not supported

---

## `claudine link`

Analyze and optionally repair skill/command/agent/script link state across providers.

```bash
claudine link [OPTIONS] [PROVIDER]
```

| Option | Description |
|--------|-------------|
| `--support` | Provider resource support matrix (Skill/Command/Agent/Script) |
| `<provider>` | Detailed capability view for one provider (fuzzy matching) |
| `--scope <user\|repo>` | Choose user-scope or repo-scope analysis (default: `user`) |
| `--apply` | Apply auto-fixable states (`LinkMissing`, `DerivedMissing`, `DerivedStale`) |
| `--filter <name>` | Analyze only resources with this name |
| `--detailed` | Show detailed output |

**Behavior:**

- Inside git repo: Links repo-scoped skills using **relative** symlinks
- Outside git repo: Links user-scoped skills using **absolute** symlinks

**Example output:**

```
Linked:
  ✓ clap         Claude → Codex, Gemini
  ✓ tokio        Claude → Codex, Gemini

Already in sync:
  = axum         Claude ↔ OpenCode (identical content)

Skipped:
  ~ chrono       Claude → OpenCode (OpenCode reads .claude/skills/)

Conflicts:
  ✗ react        Claude (a1b2c3) ≠ Codex (e5f6g7)
```

---

## `claudine providers`

Show a compact provider capability matrix with provider name as an OSC8 link to provider documentation, plus Skill, Slash, Agent, and Hooks columns.

```bash
claudine providers
```

---

## `claudine sync`

Re-apply hook registrations to match the current config.

```bash
claudine sync [OPTIONS]
```

| Option | Description |
|--------|-------------|
| `--dry-run` | Show what would change without writing |
| `--provider <name>` | Only sync specific provider |
| `--fix` | Remove unsupported events from config |

**Use cases:**
- After manually editing `~/.claudine/config.json`
- After updating `claudine` binary location
- To restore hooks after agent config reset
- With `--fix`: clean up events that don't work with certain providers

**Fix mode:**

When `claudine sync` warns about unsupported events:

```
⚠ Warning: Some configured events are not supported by their providers:
  Codex: tool_error, subagent_stop
  OpenCode: subagent_stop
```

Use `--fix` to automatically remove them:

```bash
claudine sync --fix
```

Preview what would be removed:

```bash
claudine sync --fix --dry-run
```

---

## `claudine handle <event>`

Process an incoming event from a provider hook. Reads JSON payload from stdin, auto-detects the provider from payload structure (or accepts `--provider` override), resolves environment context, and dispatches through the event pipeline.

```bash
claudine handle <EVENT> [OPTIONS]
```

**Execution Deadline.** To prevent hook handlers from blocking the parent agent session, `claudine handle` enforces a hard **5-second deadline** by default (overridable via `CLAUDINE_HANDLE_DEADLINE_SECONDS`). When exceeded, the handler aborts with a diagnostic message to stderr and exits 124. Individual bash and messenger actions also have tighter 3s timeouts when running inside a hook handler.

| Option | Description |
|--------|-------------|
| `--provider <name>` | Provider hint (auto-detected from payload) |

**Input:** JSON event payload via stdin

**Output:** JSON response to stdout (if provider expects it)

**Exit codes:** `0` = success, `2` = block (if supported), `124` = deadline exceeded

**Stdin auto-detection**: provider is detected from JSON payload structure (`hook_event_name` → Claude, `type` + `thread_id` → Codex, etc.) so hooks don't need to pass `--provider` explicitly.

```bash
echo '{"hook_event_name": "PreToolUse", "tool_name": "Bash"}' | claudine handle before_tool
```

---

## Composition Commands

Markdown frontmatter-based composition pipelines for delivering prompts to provider sessions. All three commands reuse the wrapper pipeline (env setup, harness detection, structured streaming, handler-driven recovery).

### `claudine compose <file-ref> [key=value ...]`

Compose a Markdown file and send the result as a prompt. No file mutation.

```bash
claudine compose @prompts/review.md review=review.md
```

### `claudine inline-compose <file-ref> [key=value ...]`

Use frontmatter `prompt` to generate content and replace the document body. Preserves frontmatter, updates `last_updated`.

```bash
claudine inline-compose @notes/update.md draft=false
```

### `claudine sequence <file-ref> [key=value ...]`

Run a serial sequence of composition steps declared in a single document. Shared shell approval cache across steps.

```bash
claudine sequence @research.md topic="async traits"
```

**Positional Arguments:**
- Exactly one file reference (supports `@` magic paths)
- Zero or more `key=value` setters (overrides frontmatter)

**Common Flags:**
- `--claude`, `--codex`, `--gemini`, `--opencode`, etc.
- `-i, --interactive`
- `-m, --model <MODEL>`
- `-s, --system-prompt <PROMPT|FILE>`
- `-t, --timeout <SECONDS>`
- `--dry-run`, `-q, --quiet`, `--silent`

---

## `claudine dry-run <event>`

Test what would happen for an event without side effects.

```bash
claudine dry-run <EVENT> [--provider <name>]
```

Accepts event names in multiple formats: canonical (`turn_complete`), native (`Stop`), PascalCase (`TurnComplete`), kebab-case (`turn-complete`) — all case-insensitive. When no stdin is provided, generates realistic mock payloads for the selected provider.

Shows:
- Which actions would fire
- Template interpolation results
- Matcher checks
- Provider overrides applied

---

## `claudine logs`

Query the local reporting index built from JSONL hook logs.

```bash
claudine logs [SUBCOMMAND] [FLAGS]
```

Shared filters: `--provider`, `--repo`, `--package-area`, `--package`. Read commands perform a best-effort sync before querying. Time-window commands also accept nested error drill-downs such as `claudine logs week errors` and `claudine logs today errors`.

| Subcommand | Description |
|------------|-------------|
| `today` | Today's session summary |
| `week` | This week's summary |
| `month` | This month's summary |
| `sessions` | List recent sessions |
| `tools` | Tool usage breakdown |
| `errors` | Error log |
| `repos` | Per-repo summary |
| `trends` | Usage trends over time |
| `sync` | Force re-sync of JSONL logs into SQLite |

---

## `claudine mcp`

Manage Claudine's normalized MCP catalog and provider sync state.

```bash
claudine mcp [SUBCOMMAND] [--json]
```

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

---

## Wrapped Provider Commands

Claudine can wrap provider CLIs with preflight checks, argument translation, environment sanitization, and structured streaming:

- `claudine claude`
- `claudine codex`
- `claudine gemini`
- `claudine kimi`
- `claudine qwen`
- `claudine opencode`
- `claudine goose`

### Shared Wrapper Flags

| Flag | Description |
|------|-------------|
| `-y, --yolo` | Translate to provider-specific auto-approval mode (warn-only for OpenCode) |
| `-i, --interactive` | Force interactive mode even when a prompt string is provided |
| `-m, --model <MODEL>` | Override the model used by the provider |
| `--asp <FILE>` | Append a system prompt from a file (alias: `--append-system-prompt`) |
| `--rsp <FILE>` | Replace the provider's system prompt with contents from a file (alias: `--replace-system-prompt`) |
| `-t, --timeout <SECONDS>` | Timeout in seconds (non-interactive only) |
| `-o, --output <FORMAT>` | Set output format (json, text, stream) |
| `--include <ENV_NAME>` | Keep a sensitive env var name that would otherwise be filtered |
| `--mcp` | Compose a Claudine-managed MCP session from the effective defaults |
| `--use <ID[,ID...]>` | Add specific MCP catalog IDs or aliases and enable MCP composition |
| `--sandbox` | Enable provider-specific sandboxing |
| `--repo` | Use only repo-scoped skills, commands, and agents via a shadow HOME |
| `-p, --prompt-file <FILE>` | Source initial prompt from a Markdown file (composed with Darkmatter) |
| `--frontmatter-prompt <FILE>` | Inline composition: use frontmatter prompt as input |
| `--compose <FILE>` | Chained composition: compose full document and use as prompt |
| `--dry-run` | Show what would be executed without launching the child |
| `-q, --quiet` | Show only the header line; suppress env details |
| `--silent` | Suppress all Claudine preflight output |
| `-- ...` | Force all remaining args to passthrough unchanged |

### Wrapper Behavior

- **Interactivity default**: providing a prompt string implies non-interactive mode. Use `-i`/`--interactive` to override back to interactive when providing a startup prompt.
- **Execution line**: displays `Claudine ▸ {provider} {badges} {prompt}` — only the user's prompt text is shown (provider-specific switches are not leaked). Truncated to one terminal line.
- **Structured streaming**: non-interactive runs use provider-native structured output (stream-json, JSONL, NDJSON) as the internal control plane. Claudine parses the stream live, reconstructs clean assistant text for stdout, and emits metadata summaries to stderr.
- **Stderr summaries**: session-start info (session ID, model), completion summary (duration, tokens, cost, tool calls), and verbose details (tools used, turns, stop reason).
- **Verbosity**: `--quiet` shows only a compact completion line; `--silent` suppresses all Claudine output; `-v` adds detailed metadata on the second summary line.
- Validates provider binary availability before spawn (with provider docs URL in errors).
- Filters sensitive env vars whose names contain `API_KEY` or `TOKEN` unless explicitly included.
- Reports removed env variable names to stderr (names only, sorted/unique).
- Injects `AGENT`, `YOLO`, `INTERACTIVE`, `AGENT_PARAMS`, `CLAUDINE_SESSION_ID`, and, when resolvable in monorepos, `PACKAGE_AREA` and `PACKAGE`.
- `--mcp` resolves repo defaults if `<repo>/.claudine/mcp.json` exists, otherwise user defaults; `--use` appends explicit IDs or aliases and also enables MCP mode.
- Non-interactive Codex, Gemini, and OpenCode runs also strip catalog-resolvable `#tags` from the prompt and activate the matching servers.
- Writes a synthetic JSONL summary event per session for reporting completeness.

---

## `claudine completions <shell>`

Generate shell completion scripts.

```bash
claudine completions <SHELL>
```

Supported shells: `bash`, `zsh`, `fish`, `powershell`, `elvish`

```bash
# Bash
claudine completions bash > ~/.local/share/bash-completion/completions/claudine

# Zsh
claudine completions zsh > ~/.zfunc/_claudine

# Fish
claudine completions fish > ~/.config/fish/completions/claudine.fish
```

---

## `claudine uninstall`

Remove hook registrations from all detected agents.

```bash
claudine uninstall [OPTIONS]
```

| Option | Description |
|--------|-------------|
| `--keep-config` | Keep `~/.claudine/config.json` (only remove hooks) |

**What it does:**
1. Deregisters hooks from all agent configs
2. Removes backup directory (`~/.claudine/backups/`)
3. Optionally removes `~/.claudine/config.json`

---

## CLI Module Structure

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
    ├── link.rs          → Skill synchronization management
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

---

## Exit Codes

| Code | Meaning |
|------|---------|
| `0` | Success |
| `1` | General error |
| `2` | Block action (when called as hook) |
| `3` | Configuration error |
| `4` | Provider not found |
| `5` | Permission denied |

---

## File Locations

| File | Path |
|------|------|
| User config | `~/.claudine/config.json` |
| Repo config | `<repo>/.claudine/config.json` |
| MCP catalog | `~/.claudine/mcp/catalog.json` |
| MCP defaults | `~/.claudine/mcp/defaults.json` |
| MCP state | `~/.claudine/mcp/provider-state.json` |
| Repo MCP defaults | `<repo>/.claudine/mcp.json` |
| Backups | `~/.claudine/backups/<provider>/<timestamp>.bak` |
| Event logs | `~/.claudine/logs/` (JSONL, daily rotation) |
| Reporting DB | `~/.claudine/logs/metrics.db` |

---

## Environment Variables

| Variable | Description |
|----------|-------------|
| `DEBUG` | Enable debug logging (`DEBUG=INFO`) |
| `HOME` | Used for path resolution |
| `PATH` | Must include `claudine` binary |
| `AGENT` | Injected by wrapper: provider name |
| `YOLO` | Injected by wrapper: auto-approval mode |
| `INTERACTIVE` | Injected by wrapper: interactivity flag |
| `AGENT_PARAMS` | Injected by wrapper: provider-specific args |
| `CLAUDINE_SESSION_ID` | Injected by wrapper: session identifier |
| `PACKAGE_AREA` | Injected by wrapper: monorepo package area (when resolvable) |
| `PACKAGE` | Injected by wrapper: monorepo package (when resolvable) |
