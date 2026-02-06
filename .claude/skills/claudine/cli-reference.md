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

Display rich help documentation using darkmatter markdown rendering.

```bash
claudine about
```

---

## `claudine init`

Interactive setup wizard for initial configuration.

```bash
claudine init [OPTIONS]
```

| Option | Description |
|--------|-------------|
| `--quick` | Use defaults without prompting |
| `--repo` | Create project-scoped configuration |

**Interactive Flow:**

1. **Agent Discovery** - Scan for installed agents
2. **Event Selection** - Choose which events to hook
3. **Action Configuration** - Configure actions per event
4. **Global Settings** - TTS voice, defaults
5. **Write & Register** - Save config and register with agents

**Quick Mode:**

```bash
claudine init --quick
```

Creates configuration with sensible defaults:
- All detected agents enabled
- `turn_complete` → SoundEffect (success)
- `tool_error` → SoundEffect (error)
- `permission_request` → SoundEffect (notification)
- `session_start` → SoundEffect (power-up)

---

## `claudine hooks`

Show registered hooks for all detected agents.

```bash
claudine hooks [OPTIONS] [PROVIDER]
```

| Option | Description |
|--------|-------------|
| `--support` | Show provider event support matrix (✓/○/-) |
| `--mapping` | Show native event name mappings |
| `--describe` | Show event descriptions and schemas |
| `--fix` | Auto-fix invalid sound effect names |
| `-v` | Verbose mode with per-event action counts |

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

Synchronize skills across all detected providers.

```bash
claudine link [OPTIONS]
```

| Option | Description |
|--------|-------------|
| `--dry-run` | Preview changes without creating symlinks |
| `--provider <name>` | Only link to/from specific provider |
| `--replace-duplicates` | Replace duplicate skills with symlinks |
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

Re-apply hook registrations based on current `~/.hooker` configuration.

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

This removes unsupported events from `~/.hooker` and saves the cleaned config.

Preview what would be removed:

```bash
claudine sync --fix --dry-run
```

---

## `claudine handle <event>`

Process an event (used by provider hooks, not typically called directly).

```bash
claudine handle <EVENT> [OPTIONS]
```

| Option | Description |
|--------|-------------|
| `--provider <name>` | Provider hint (auto-detected from payload) |

**Input:** JSON event payload via stdin

**Output:** JSON response to stdout (if provider expects it)

**Exit codes:** `0` = success, `2` = block (if supported)

```bash
echo '{"hook_event_name": "PreToolUse", "tool_name": "Bash"}' | claudine handle before_tool
```

---

## `claudine dry-run <event>`

Test event handling without executing actions.

```bash
claudine dry-run <EVENT>
```

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

Remove all Claudine hooks from agent configs.

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
