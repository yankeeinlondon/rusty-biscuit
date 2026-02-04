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

**Usage:**
```bash
claudine about
claudine about --verbose
```

**Output:**
- Feature overview
- Quick start guide
- Provider compatibility table
- Configuration examples

---

## `claudine init`

Interactive setup wizard for initial configuration.

**Usage:**
```bash
claudine init [OPTIONS]
```

**Options:**
| Option | Description |
|--------|-------------|
| `--quick` | Use defaults without prompting |
| `--repo` | Create project-scoped configuration |

**Interactive Flow:**

### Phase 1: Agent Discovery
```
Scanning for installed agents...

  Found:
    [x] Claude Code    (~/.claude/ exists, claude on PATH)
    [x] Gemini CLI     (~/.gemini/ exists, gemini on PATH)
    [x] Codex CLI      (~/.codex/ exists, codex on PATH)
    [ ] OpenCode       (not found)
    [ ] Roo Code       (not found)

Which agents do you want to configure?
```

### Phase 2: Event Selection
```
Which events do you want to hook into?
> [x] Session Start       (agent session begins)
  [x] Session End         (agent session ends)
  [x] Before Prompt       (user submits a prompt)
  [x] Before Tool         (before tool execution)
  [x] After Tool          (after tool completes)
  ...
```

### Phase 3: Action Configuration
For each selected event:
```
How should "Session Start" be handled?
> [x] Sound Effect
  [ ] Speak (TTS)
  [ ] Log to file
  [ ] Report to terminal

Which sound for "Session Start"?
  power-up (Recommended)
  notification
  beep
  > power-up

Message template for "Session Start":
  (Supports {placeholders}: {provider}, {env.branch}, {env.os}, etc.)
  > Session started on {env.branch}
```

### Phase 4: Global Settings
```
TTS voice preference:
  System default (Recommended)
  Samantha (macOS)
  Custom
  > System default
```

### Quick Mode

```bash
claudine init --quick
```

Creates configuration with:
- All detected agents enabled
- All events enabled
- `TurnComplete` → SoundEffect (success)
- `ToolError` → SoundEffect (error)
- `PermissionRequest` → SoundEffect (notification)
- `SessionStart` → SoundEffect (power-up)

### Repo Mode

```bash
claudine init --repo
```

Creates project-scoped configuration:
- Writes to `<repo-root>/.hooker`
- Registers in agent project configs (e.g., `.claude/settings.json`)
- Offers to add `.hooker` to `.gitignore`

---

## `claudine link`

Synchronize skills across all detected providers.

**Usage:**
```bash
claudine link [OPTIONS]
```

**Options:**
| Option | Description |
|--------|-------------|
| `--dry-run` | Preview changes without creating symlinks |
| `--provider <name>` | Only link to/from specific provider |
| `--replace-duplicates` | Replace duplicate skills with symlinks |
| `--verbose` | Show detailed hash values and paths |

**Behavior:**

When run inside a git repository:
- Links repo-scoped skills using **relative** symlinks
- Scans `<root>/.claude/skills/`, etc.

When run outside a git repository:
- Links user-scoped skills using **absolute** symlinks
- Scans `~/.claude/skills/`, etc.

**Example Output:**
```
Scanning skill directories...

  Claude Code:  ~/.claude/skills/  (12 skills)
  Roo Code:     ~/.roo/skills/     (8 skills)
  OpenCode:     ~/.config/opencode/skills/  (5 skills)
  Gemini CLI:   ~/.gemini/skills/  (3 skills)
  Codex CLI:    ~/.codex/skills/   (10 skills)

Analyzing 18 unique skill names...

  Linked:
    ✓ clap         Claude → Roo, Gemini, Codex
    ✓ tokio        Claude → Roo, Gemini, Codex
    ✓ serde        Claude → Roo, Gemini, Codex

  Already in sync:
    = axum         Claude ↔ Roo ↔ OpenCode (identical content)

  Skipped (OpenCode reads .claude/skills/ directly):
    ~ chrono       Claude → OpenCode (not needed)

  Conflicts (different content):
    ✗ react        Claude (hash: a1b2c3d4) ≠ Roo (hash: e5f6g7h8)
                     Resolve: compare ~/.claude/skills/react/ vs ~/.roo/skills/react/

Summary: 3 linked, 1 in sync, 1 skipped, 1 conflict
```

---

## `claudine sync`

Re-apply hook registrations based on current `~/.hooker` configuration.

**Usage:**
```bash
claudine sync [OPTIONS]
```

**Options:**
| Option | Description |
|--------|-------------|
| `--dry-run` | Show what would change without writing |
| `--provider <name>` | Only sync specific provider |

**Use cases:**
- After manually editing `~/.hooker`
- After updating Claudine binary location
- To restore hooks after agent config reset

---

## `claudine status`

Display current registration status for all providers.

**Usage:**
```bash
claudine status
```

**Example Output:**
```
Agent Registrations:

  Claude Code (~/.claude/settings.json)
    Registered events: session_start, before_tool, after_tool, turn_complete (4/15)
    Status: in sync

  Gemini CLI (~/.gemini/settings.json)
    Registered events: session_start, before_tool, after_tool, turn_complete (4/10)
    Status: in sync

  Codex CLI (~/.codex/config.toml)
    Registered events: turn_complete (1/1 available via hooks)
    Status: in sync
    Note: Use `claudine start codex` for full event coverage

  OpenCode (not installed)
  Roo Code (not installed)
```

---

## `claudine handle <event>`

Process an event (used by provider hooks, not typically called directly).

**Usage:**
```bash
claudine handle <EVENT> [OPTIONS]
```

**Arguments:**
| Argument | Description |
|----------|-------------|
| `<EVENT>` | Event name (e.g., `session_start`, `before_tool`) |

**Input:**
- JSON event payload via stdin (most providers)
- JSON as last argv argument (Codex only)

**Output:**
- JSON response to stdout (if provider expects it)
- Exit code: `0`=success, `2`=block (if supported)

**Example:**
```bash
echo '{
  "hook_event_name": "PreToolUse",
  "tool_name": "Bash",
  "tool_input": {"command": "npm test"}
}' | claudine handle before_tool
```

---

## `claudine dry-run <event>`

Test event handling without executing actions.

**Usage:**
```bash
claudine dry-run <EVENT>
```

Shows:
- Which actions would fire
- Template interpolation results
- Any matcher checks
- Provider overrides applied

---

## `claudine start <agent>`

Wrap an agent CLI to intercept its event stream.

**Usage:**
```bash
claudine start <AGENT> [ARGS...]
```

**Arguments:**
| Argument | Description |
|----------|-------------|
| `<AGENT>` | Agent to wrap (`claude`, `codex`, `gemini`, `opencode`, `roo`) |
| `[ARGS...]` | Arguments to pass to the agent |

**Required for:**
- **Roo Code** - Only way to receive events (no native hooks)
- **Codex CLI** - Unlock full JSONL event stream

**Example:**
```bash
# Wrap Codex for full event coverage
claudine start codex exec --json "npm test"

# Wrap Roo Code
claudine start roo "implement fibonacci"

# Pass through to hook-based agent
claudine start claude  # Just execs `claude` after SessionStart
```

---

## `claudine completions <shell>`

Generate shell completion scripts.

**Usage:**
```bash
claudine completions <SHELL>
```

**Supported shells:**
- `bash`
- `zsh`
- `fish`
- `powershell`
- `elvish`

**Example:**
```bash
# Bash
claudine completions bash > /usr/local/etc/bash_completion.d/claudine

# Zsh
claudine completions zsh > /usr/local/share/zsh/site-functions/_claudine

# Fish
claudine completions fish > ~/.config/fish/completions/claudine.fish
```

---

## `claudine uninstall`

Remove all Claudine hooks from agent configs.

**Usage:**
```bash
claudine uninstall [OPTIONS]
```

**Options:**
| Option | Description |
|--------|-------------|
| `--keep-config` | Keep `~/.hooker` file (only remove hooks) |

**What it does:**
1. Deregisters hooks from all agent configs
2. Removes OpenCode bridge plugin
3. Removes backup directory (`~/.claudine/backups/`)
4. Optionally removes `~/.hooker`

---

## Configuration Examples

### Basic TTS Setup

```json
{
  "version": "1.0",
  "settings": {
    "tts": { "provider": "say", "voice": "Samantha" }
  },
  "events": {
    "session_start": {
      "actions": [
        { "type": "speak", "message": "Session started" }
      ]
    }
  }
}
```

### Development Workflow

```json
{
  "version": "1.0",
  "events": {
    "session_start": {
      "actions": [
        { "type": "sound_effect", "name": "power-up" },
        { "type": "speak", "message": "Starting on {env.branch} in {env.language}" }
      ]
    },
    "turn_complete": {
      "actions": [
        { "type": "sound_effect", "name": "success", "volume": 0.5 }
      ]
    },
    "tool_error": {
      "actions": [
        { "type": "sound_effect", "name": "error" },
        { "type": "speak", "message": "Tool failed: {tool_name}" }
      ]
    },
    "before_tool": {
      "matcher": "Bash",
      "actions": [
        {
          "type": "report",
          "handler": {
            "format": "compact",
            "template": "[BASH] {tool_input.command}"
          }
        }
      ]
    }
  }
}
```

### With Provider Overrides

```json
{
  "version": "1.0",
  "events": {
    "turn_complete": {
      "actions": [{ "type": "sound_effect", "name": "success" }],
      "overrides": {
        "claude": {
          "actions": [
            { "type": "speak", "message": "Claude finished" }
          ]
        },
        "gemini": {
          "actions": [
            { "type": "sound_effect", "name": "beep" }
          ]
        }
      }
    }
  }
}
```

### Server Logging

```json
{
  "version": "1.0",
  "settings": {
    "default_log_target": {
      "type": "server",
      "url": "https://my-logs.example.com/events"
    }
  },
  "events": {
    "session_start": {
      "actions": [{ "type": "log", "target": { "type": "server", "url": "https://my-logs.example.com/sessions" } }]
    },
    "after_tool": {
      "actions": [{ "type": "log" }]
    }
  }
}
```

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
| User config | `~/.hooker` or `~/.hook-config` |
| Repo config | `<repo>/.hooker` or `<repo>/.hook-config` |
| Backups | `~/.claudine/backups/<provider>/<timestamp>.bak` |
| Codex wrapper | `~/.claudine/codex-notify-wrapper.sh` |
| OpenCode bridge | `~/.config/opencode/plugin/claudine-bridge.ts` |
| Event logs | `~/.claudine/events.jsonl` (default) |

---

## Environment Variables

| Variable | Description |
|----------|-------------|
| `CLAUDE_PROJECT_DIR` | Set by Claude Code during hooks |
| `CLAUDE_PLUGIN_ROOT` | Set by Claude Code for plugin hooks |
| `CLAUDE_ENV_FILE` | SessionStart only, for persisting env vars |
| `DEBUG` | Enable debug logging (`DEBUG=INFO`) |
| `HOME` | Used for path resolution |
| `PATH` | Must include `claudine` binary |
