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

Interactive setup wizard for initial configuration. Walks through 5 phases:

1. **Agent Discovery** — detects installed agentic CLIs on the system
2. **Event Selection** — choose which events to subscribe to (filters to hook-supported events)
3. **Action Configuration** — configure actions per event (sound effects, TTS, logging, etc.)
4. **Global Settings** — TTS provider selection, default log targets
5. **Write & Register** — saves `~/.hooker` config and registers hooks with each provider

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

`--repo` creates `.hooker` in the current directory (project-scoped) and adds it to `.gitignore`.

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
| `--fix` | Validate sound effect names and auto-fix with suggestions |

**Provider fuzzy matching**: commands that accept a provider name use a 3-tier resolution: exact match → prefix match → contains match. This lets users type `cl` instead of `claude`.

**Sound effect validation**: `hooks --fix` uses a 5-tier fuzzy matching algorithm (exact, normalized, prefix, contains, Levenshtein-like) to suggest replacements for invalid effect names.

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

Synchronize skills, commands, and agents across providers via symlinks.

```bash
claudine link [OPTIONS] [PROVIDER]
```

| Option | Description |
|--------|-------------|
| `--support` | Provider resource support matrix (Skill/Command/Agent/Script) |
| `<provider>` | Detailed capability view for one provider (fuzzy matching) |
| `--dry-run` | Preview what would be linked without creating symlinks |
| `--filter <name>` | Link only a specific skill by name |
| `--detailed` | Show detailed output |
| `--replace-duplicates` | Replace duplicate real directories with symlinks |
| `-v, --verbose` | Show detailed hash values and paths |

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
- After manually editing `~/.hooker`
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

| Option | Description |
|--------|-------------|
| `--provider <name>` | Provider hint (auto-detected from payload) |

**Input:** JSON event payload via stdin

**Output:** JSON response to stdout (if provider expects it)

**Exit codes:** `0` = success, `2` = block (if supported)

**Stdin auto-detection**: provider is detected from JSON payload structure (`hook_event_name` → Claude, `type` + `thread_id` → Codex, etc.) so hooks don't need to pass `--provider` explicitly.

```bash
echo '{"hook_event_name": "PreToolUse", "tool_name": "Bash"}' | claudine handle before_tool
```

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
| `--keep-config` | Keep `~/.hooker` file (only remove hooks) |

**What it does:**
1. Deregisters hooks from all agent configs
2. Removes backup directory (`~/.claudine/backups/`)
3. Optionally removes `~/.hooker`

---

## CLI Module Structure

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
| User config | `~/.hooker` |
| Repo config | `<repo>/.hooker` |
| Backups | `~/.claudine/backups/<provider>/<timestamp>.bak` |
| Event logs | `~/.claudine/events.jsonl` (default) |

---

## Environment Variables

| Variable | Description |
|----------|-------------|
| `DEBUG` | Enable debug logging (`DEBUG=INFO`) |
| `HOME` | Used for path resolution |
| `PATH` | Must include `claudine` binary |
