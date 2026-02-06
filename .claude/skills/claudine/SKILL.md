---
name: claudine
description: Universal event handler and skill linker for agentic CLIs (Claude, Codex, Gemini, Goose, Kimi Code, OpenCode, Qwen Code). Use when working with agent hooks, cross-provider skill synchronization, or configuring event reactions like TTS and sound effects.
---

## Purpose

Claudine provides:

1. **Universal event handling** - React to agent lifecycle events across 7 CLI providers
2. **Skill linking** - Synchronize skills across provider directories using symlinks
3. **Non-destructive integration** - Registers hooks without clobbering existing configs

## Quick Start

```bash
# Initialize configuration interactively
claudine init

# Or use quick defaults
claudine init --quick

# Check registered hooks
claudine hooks

# Link skills across all detected providers
claudine link
```

## Supported Providers

| Provider | Hook Support | Skills | Config Method |
|----------|:------------:|:------:|---------------|
| Claude | ✓ Hook | ✓ | `settings.json` hooks |
| Codex | Partial* | ✓ | `config.toml` notify + JSONL |
| Gemini | ✓ Hook | ✓ | `settings.json` hooks |
| Goose | NonHook | - | Stream-json + env var |
| Kimi Code | NonHook | - | Wire mode JSON-RPC |
| OpenCode | ✓ Hook | ✓ | `opencode.json` plugins |
| Qwen Code | NonHook | ✓ | Stream-json output |

*Codex: `turn_complete` via hook; other events via JSONL stream
**NonHook**: Requires wrapper/proxy (not yet implemented)

## Event Model

15 normalized events across providers:

| Category | Events |
|----------|--------|
| Session | `session_start`, `session_end` |
| Tool | `before_tool`, `after_tool`, `tool_error` |
| Turn | `before_prompt`, `turn_complete`, `turn_error` |
| Other | `permission_request`, `subagent_start/stop`, `before/after_model`, `before_compact`, `notification` |

## CLI Commands

| Command | Purpose |
|---------|---------|
| `claudine init [--quick] [--repo]` | Setup wizard |
| `claudine hooks [provider]` | Show hook status |
| `claudine hooks --support` | Event support matrix |
| `claudine hooks --mapping` | Native event mappings |
| `claudine link [--dry-run]` | Sync skills |
| `claudine sync [--dry-run]` | Re-apply registrations |
| `claudine handle <event>` | Process event (hook target) |
| `claudine uninstall` | Remove all hooks |

## Configuration (`~/.hooker`)

```json
{
  "version": "1.0",
  "settings": { "tts": { "provider": "say" } },
  "providers": {
    "claude": {
      "events": {
        "turn_complete": {
          "enabled": true,
          "actions": [{ "type": "sound_effect", "name": "success" }]
        }
      }
    }
  }
}
```

## Actions

| Type | Description |
|------|-------------|
| `speak` | TTS with template interpolation |
| `sound_effect` | 53 embedded effects |
| `log` | JSONL file or HTTP POST |
| `report` | Output to agent stdout |
| `run` | Execute shell command |

## Template Placeholders

**Event fields:** `{provider}`, `{event}`, `{tool_name}`, `{error}`, `{prompt}`, `{session_id}`, `{timestamp}`

**Context fields** (auto-detected):
- `{os.*}` - `{os.name}`, `{os.type}`, `{os.hostname}`
- `{hardware.*}` - `{hardware.arch}`, `{hardware.cpu}`, `{hardware.cores}`
- `{git.*}` - `{git.branch}`, `{git.repo_name}`, `{git.repo_org}`, `{git.hosting}`
- `{project.*}` - `{project.language}`, `{project.is_monorepo}`

Run `claudine hooks --variables` for the complete list.

## Troubleshooting

- **Hooks not firing?** Check `claudine hooks`, verify PATH, restart agent
- **Skills not linking?** Use `claudine link --dry-run` to preview
- **OpenCode shows no links?** OpenCode reads `.claude/skills/` directly

## Additional Resources

- [architecture.md](architecture.md) - Event model, provider adapters, support matrix
- [cli-reference.md](cli-reference.md) - Full command documentation
- `claudine/docs/hooks/` - Per-provider hook specifications
