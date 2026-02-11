---
name: claudine
description: Universal event handler and skill linker for agentic CLIs (Claude, Codex, Gemini, Goose, Kimi Code, OpenCode, Qwen Code). Use when working with agent hooks, cross-provider skill synchronization, or configuring event reactions like TTS and sound effects.
---

## Purpose

Claudine normalizes 16 lifecycle events across 7 agentic CLI providers into a single configuration, then executes actions (TTS, sound effects, logging, shell commands) when those events fire. Also synchronizes skills, commands, and agents between providers via symlinks.

1. **Universal event handling** - React to agent lifecycle events across 7 CLI providers
2. **Skill linking** - Synchronize skills, commands, agents, and scripts across provider directories
3. **Non-destructive integration** - Atomic config writes, backup utilities, registers hooks without clobbering

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

| Provider | Hook | NonHook | Skills | Config Method |
|----------|:----:|:-------:|:------:|---------------|
| Claude Code | ✓ | - | ✓ | `settings.json` hooks |
| Codex CLI | partial | ✓ | ✓ | `config.toml` notify + JSONL stream |
| Gemini CLI | ✓ | - | ✓ | `settings.json` hooks |
| Goose | - | ✓ | - | Stream-json + env var |
| Kimi Code | - | ✓ | - | Wire mode JSON-RPC |
| OpenCode | ✓ | - | ✓ | `opencode.json` plugins |
| Qwen Code | - | ✓ | ✓ | Stream-json output |

**Hook** = native hook/plugin system (config-driven).
**NonHook** = requires wrapper or stream parsing (not yet implemented for Goose/Kimi/Qwen).

## Event Model

16 normalized lifecycle events across 7 providers:

| Category | Events |
|----------|--------|
| Session | `session_start`, `session_end` |
| Prompt | `before_prompt` |
| Tool | `before_tool`, `after_tool`, `tool_error` |
| Turn | `turn_complete`, `turn_error` |
| Permission | `permission_request`, `human_in_the_loop` |
| Subagent | `subagent_start`, `subagent_stop` |
| Model | `before_model`, `after_model` |
| Other | `before_compact`, `notification` |

## CLI Commands

| Command | Purpose |
|---------|---------|
| `claudine init [--quick] [--repo]` | Setup wizard (interactive or quick defaults) |
| `claudine hooks [provider]` | Show hook status for all or one provider |
| `claudine hooks --support` | Event support matrix |
| `claudine hooks --mapping` | Native event mappings |
| `claudine hooks --describe` | Event descriptions and payload schemas |
| `claudine hooks --variables` | Template variables with current values |
| `claudine hooks --fix` | Auto-fix invalid sound effect names |
| `claudine link [--dry-run] [--filter]` | Sync skills across providers |
| `claudine link --support` | Provider resource support matrix |
| `claudine sync [--dry-run] [--provider] [--fix]` | Re-apply registrations |
| `claudine handle <event> [--provider]` | Process event from stdin (hook target) |
| `claudine dry-run <event> [--provider]` | Test event handling without side effects |
| `claudine about` | Rich help documentation |
| `claudine completions <shell>` | Generate shell completions |
| `claudine uninstall [--keep-config]` | Remove hooks from all agents |

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

**Merge strategy**: repo-level (`.hooker`) provider configs completely replace user-level (`~/.hooker`); global settings merge field-by-field with repo taking precedence.

## Actions

| Type | Behavior | Blocking |
|------|----------|----------|
| `speak` | TTS via biscuit-speaks with template interpolation | Fire-and-forget |
| `sound_effect` | 53 embedded effects via playa with volume/speed | Fire-and-forget |
| `log` | JSONL file append or HTTP POST (10s timeout) | Synchronous |
| `report` | Output to stdout with optional template/format | Synchronous |
| `run` | Execute shell command | Configurable |

## Template Variables (29)

**Event:** `{provider}`, `{event}`, `{timestamp}`, `{session_id}`, `{cwd}`, `{tool_name}`, `{error}`, `{prompt}`, `{agent_type}`, `{notification_type}`

**Context** (auto-detected via sniff):
- `{os.*}` - `{os.name}`, `{os.type}`, `{os.version}`, `{os.hostname}`
- `{hardware.*}` - `{hardware.arch}`, `{hardware.cpu}`, `{hardware.cores}`
- `{git.*}` - `{git.branch}`, `{git.is_dirty}`, `{git.head_sha}`, `{git.head_message}`, `{git.remote}`, `{git.hosting}`, `{git.repo_name}`, `{git.repo_org}`
- `{project.*}` - `{project.language}`, `{project.is_monorepo}`, `{project.monorepo_tool}`

Unknown placeholders are left as-is. `None` values render as empty strings.

## Troubleshooting

- **Hooks not firing?** Check `claudine hooks`, verify PATH, restart agent
- **Skills not linking?** Use `claudine link --dry-run` to preview
- **OpenCode shows no links?** OpenCode reads `.claude/skills/` directly
- **Invalid sound effects?** Use `claudine hooks --fix` for 5-tier fuzzy matching suggestions

## Additional Resources

- [architecture.md](architecture.md) - Event model, dispatch pipeline, provider adapters, linking algorithm
- [cli-reference.md](cli-reference.md) - Full command documentation with examples
- `claudine/docs/hooks/` - Per-provider hook specifications
