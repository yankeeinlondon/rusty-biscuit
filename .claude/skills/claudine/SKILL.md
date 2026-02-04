---
name: claudine
description: Claudine - Universal event handler and skill linker for agentic CLIs (Claude Code, Codex, Gemini, OpenCode, Roo). Use when working with agent hooks, cross-provider skill synchronization, or configuring event reactions like TTS and sound effects.
---

## Purpose

Claudine provides:

1. **Universal event handling** - React to agent lifecycle events across 5 CLI providers with consistent configuration
2. **Skill linking** - Synchronize skills across provider directories using symlinks
3. **Non-destructive integration** - Registers hooks without clobbering existing agent configs

## Quick Start

```bash
# Initialize configuration interactively
claudine init

# Or use quick defaults
claudine init --quick

# Link skills across all detected providers
claudine link

# Check status
claudine status
```

## Supported Providers

| Provider | Events | Skill Linking | Config Path |
|----------|--------|---------------|-------------|
| Claude Code | 12 hooks | Yes | `~/.claude/settings.json` |
| Codex CLI | JSONL stream | Yes | `~/.codex/config.toml` |
| Gemini CLI | 11 hooks | Yes | `~/.gemini/settings.json` |
| OpenCode | Plugin-based | Yes* | `~/.config/opencode/opencode.json` |
| Roo Code | Stream only | Yes | `~/.config/roo/` |

*OpenCode also reads from `.claude/skills/` directly

## Event Model

Claudine normalizes provider-native events into a shared `AgenticEvent` enum:

### Session Lifecycle
- `session_start` - Agent session begins
- `session_end` - Agent session terminates

### Tool Lifecycle
- `before_tool` - Before tool execution
- `after_tool` - After tool completes
- `tool_error` - Tool execution failed

### Turn Lifecycle
- `before_prompt` - User prompt submitted
- `turn_complete` - Agent finished responding
- `turn_error` - Agent turn failed

### Other Events
- `permission_request` - User permission needed
- `subagent_start` / `subagent_stop` - Subagent lifecycle
- `before_compact` - Context compaction
- `notification` - System notification
- `before_model` / `after_model` - LLM interaction (Gemini only)

## Configuration (`~/.hooker`)

The main configuration file is JSON:

```json
{
  "version": "1.0",
  "settings": {
    "tts": { "provider": "say", "voice": "Samantha" }
  },
  "events": {
    "session_start": {
      "actions": [
        { "type": "sound_effect", "name": "power-up" },
        { "type": "speak", "message": "Session started on {env.branch}" }
      ]
    },
    "turn_complete": {
      "actions": [{ "type": "sound_effect", "name": "success" }]
    },
    "tool_error": {
      "actions": [
        { "type": "sound_effect", "name": "error" },
        { "type": "speak", "message": "Tool {tool_name} failed" }
      ]
    }
  }
}
```

## Available Actions

### `speak`
Text-to-speech using biscuit-speaks:

```json
{
  "type": "speak",
  "message": "Starting on {env.branch} in {env.language} project"
}
```

### `sound_effect`
Play embedded sound effects (53 effects across 6 categories):

```json
{
  "type": "sound_effect",
  "name": "success",
  "volume": 0.8,
  "speed": 1.0
}
```

**Common effects:** `power-up`, `success`, `error`, `notification`, `sad-trombone`, `beep`

### `log`
Append to JSONL file or POST to server:

```json
{
  "type": "log",
  "target": { "type": "local_file", "path": "~/.claudine/events.jsonl" }
}
```

### `report`
Output to agent's stdout:

```json
{
  "type": "report",
  "handler": {
    "format": "compact",
    "template": "[TOOL] {tool_name} executing"
  }
}
```

## Template Interpolation

Actions support `{placeholder}` syntax:

| Placeholder | Source |
|-------------|--------|
| `{provider}` | Agent provider name |
| `{event}` | Event name |
| `{tool_name}` | Tool being executed |
| `{error}` | Error message |
| `{env.branch}` | Git branch |
| `{env.os}` | Operating system |
| `{env.language}` | Primary project language |

See full template reference in [architecture.md](architecture.md#template-interpolation).

## CLI Commands

| Command | Purpose |
|---------|---------|
| `claudine init` | Interactive setup wizard |
| `claudine init --quick` | Quick setup with defaults |
| `claudine init --repo` | Project-scoped configuration |
| `claudine link` | Sync skills across providers |
| `claudine link --dry-run` | Preview skill linking |
| `claudine sync` | Re-apply hook registrations |
| `claudine status` | Show current registrations |
| `claudine about` | Rich help documentation |
| `claudine handle <event>` | Process event (called by hooks) |

## Per-Provider Event Mapping

### Claude Code
| Claudine Event | Claude Hook |
|----------------|-------------|
| `session_start` | `SessionStart` |
| `before_tool` | `PreToolUse` |
| `after_tool` | `PostToolUse` |
| `turn_complete` | `Stop` |

### Codex CLI
Limited to `turn_complete` via `notify` hook. Full coverage requires `claudine start codex` wrapper.

### Gemini CLI
| Claudine Event | Gemini Hook |
|----------------|-------------|
| `session_start` | `SessionStart` |
| `before_prompt` | `BeforeAgent` |
| `before_tool` | `BeforeTool` |
| `after_tool` | `AfterTool` |
| `turn_complete` | `AfterAgent` |

### OpenCode
Uses TypeScript plugin bridge. See [architecture.md](architecture.md#opencode-plugin-bridge) for details.

### Roo Code
No native hooks. Requires `claudine start roo` wrapper with `--output-format stream-json` parsing.

## Skill Linking

The `claudine link` command synchronizes skills across providers:

1. **Discovers** all skills in provider directories
2. **Hashes** skill content using xxHash
3. **Detects conflicts** (same skill name, different content)
4. **Creates symlinks** from source to target providers

**Linking rules:**
- User-scoped: Absolute symlinks (`~/.claude/skills/` → `~/.roo/skills/`)
- Repo-scoped: Relative symlinks (`.claude/skills/` → `.roo/skills/`)
- OpenCode is skipped for Claude skills (OpenCode reads `.claude/skills/` directly)
- Codex has no repo-scoped skills directory

## Safety Features

1. **Non-destructive config editing** - Preserves unknown keys and formatting
2. **Automatic backups** - Backs up agent configs before modification (`~/.claudine/backups/`)
3. **Atomic writes** - Writes to temp file, then renames
4. **Conflict detection** - Reports when same skill exists with different content
5. **No overwrite policy** - Never overwrites real directories with symlinks

## Troubleshooting

**Hooks not firing?**
- Check `claudine status` for registration status
- Verify `claudine` is on PATH (or use absolute path)
- Restart agent session (hooks loaded at startup)

**Skills not linking?**
- Check for conflicts with `claudine link --dry-run`
- Verify skill `name` in frontmatter matches directory name
- OpenCode reads `.claude/skills/` directly (not a bug)

**Events not triggering actions?**
- Check event is enabled in `~/.hooker`
- Verify matcher regex (if configured)
- Check provider-specific override hasn't disabled the event

## Additional Resources

- **Deep architecture**: [architecture.md](architecture.md) - Event model, provider adapters, configuration schema
- **CLI reference**: [cli-reference.md](cli-reference.md) - Full command documentation with examples
- **Per-provider hooks**: See `/claudine/docs/hooks/` for detailed event specifications

## Dependencies

Uses libraries from this monorepo:
- `darkmatter` - Rich terminal markdown rendering
- `biscuit-speaks` - Text-to-speech
- `unchained-ai` - AI provider interactions
- `sniff_lib` - Environment detection
- `playa` - Sound effects
- `biscuit_hash` - Content hashing
