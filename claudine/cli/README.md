# Claudine CLI

Binary: `claudine` — interactive setup, hook inspection, event handling, and skill linking for agentic CLIs.

## Commands

### `claudine init [--quick] [--repo]`

Interactive setup wizard that walks through 5 phases:

1. **Agent Discovery** — detects installed agentic CLIs on the system
2. **Event Selection** — choose which events to subscribe to (filters to hook-supported events)
3. **Action Configuration** — configure actions per event (sound effects, TTS, logging, etc.)
4. **Global Settings** — TTS provider selection, default log targets
5. **Write & Register** — saves `~/.hooker` config and registers hooks with each provider

`--quick` skips prompts and uses sensible defaults (SessionStart, TurnComplete, ToolError, PermissionRequest with sound effects). `--repo` creates `.hooker` in the current directory (project-scoped) and adds it to `.gitignore`.

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
| `--fix` | Validate sound effect names and auto-fix with suggestions |

Sound effect validation uses a 5-tier fuzzy matching algorithm to suggest replacements for invalid effect names.

### `claudine link [provider] [flags]`

Synchronize skills, commands, and agents across providers via symlinks.

| Flag | Description |
|------|-------------|
| `--support` | Provider resource support matrix (Skill/Command/Agent/Script) |
| `<provider>` | Detailed capability view for one provider (fuzzy matching) |
| `--dry-run` | Preview what would be linked without creating symlinks |
| `--filter <name>` | Link only a specific skill by name |
| `--detailed` | Show detailed output |
| `--replace-duplicates` | Replace duplicate real directories with symlinks |

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

### `claudine about`

Renders rich help documentation using darkmatter markdown rendering with biscuit-terminal fallback.

### `claudine completions <shell>`

Generate shell completions for bash, zsh, fish, powershell, or elvish.

### `claudine uninstall [--keep-config]`

Remove hook registrations from all detected agents. `--keep-config` preserves the `~/.hooker` config file while removing only the hook registrations.

## Module Structure

```
cli/src/
├── main.rs              → Entry point, clap parser, command dispatch
├── log.rs               → Output formatting (message/data/info/warn/error)
└── commands/
    ├── about.rs         → Rich help rendering
    ├── completions.rs   → Shell completion generation
    ├── dry_run.rs       → Event simulation with mock payloads
    ├── handle.rs        → Event processing from stdin
    ├── hooks.rs         → Hook inspection and validation
    ├── link.rs          → Skill synchronization management
    ├── sync.rs          → Hook re-registration
    ├── uninstall.rs     → Hook removal
    └── init/
        ├── mod.rs       → Wizard orchestration (interactive + quick modes)
        ├── prompts.rs   → inquire-based interactive prompts
        └── defaults.rs  → Default configs and event-to-action mappings
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
| `error()` | stderr | Red "error:" prefix |

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
- **Sound effect suggestion engine**: `hooks --fix` uses 5 matching heuristics (exact, normalized, prefix, contains, Levenshtein-like) to suggest replacements for invalid sound effect names.
- **Stdin auto-detection in handle**: the provider is detected from JSON payload structure (`hook_event_name` → Claude, `type` + `thread_id` → Codex, etc.) so hooks don't need to pass `--provider` explicitly.
