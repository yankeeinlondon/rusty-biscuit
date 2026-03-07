# Claudine

> Claude Code's ex-girlfriend who knows Claude's inner secrets but is now dating other Agents

Universal event handler and skill linker for agentic CLIs. Normalizes 16 lifecycle events across 8 providers into a single configuration, then executes 6 action types (TTS, sound effects, logging, shell commands, reports, blocking calls) when those events fire. Also synchronizes skills, commands, and agents between providers via symlinks.

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
| Roo Code | - | ✓ | ✓ | Stream-json event emitter |

**Hook** = native hook/plugin system (config-driven).
**NonHook** = requires wrapper or stream parsing (not yet implemented for Goose/Kimi/Qwen/Roo).

## Quick Start

```bash
# Interactive setup
claudine init

# Quick setup with defaults
claudine init --quick

# Check hook registrations
claudine hooks

# Link skills across providers
claudine link

# Review today's agent activity
claudine logs
```

## CLI Commands

| Command | Description |
|---------|-------------|
| `claudine init [--quick] [--repo]` | Interactive setup wizard (or quick defaults) |
| `claudine hooks [provider]` | Show registered hooks for all or one provider |
| `claudine hooks --support` | Provider event support matrix |
| `claudine hooks --mapping` | Native event name mappings |
| `claudine hooks --describe` | Event descriptions and payload schemas |
| `claudine hooks --variables` | Template variables with current values |
| `claudine link [provider] [--scope <user\|repo>] [--apply] [--filter] [--detailed]` | Analyze resource link states and optionally fix auto-repairable issues |
| `claudine link --support` | Provider resource support matrix |
| `claudine providers` | Provider capability matrix (skill/slash/agent/hooks) |
| `claudine logs [today\|week\|month\|sessions\|tools\|errors\|repos\|trends\|sync]` | Reporting and sync for Claudine JSONL logs |
| `claudine sync [--dry-run] [--provider] [--fix]` | Re-apply hook registrations |
| `claudine handle <event> [--provider]` | Process event from stdin (called by hooks) |
| `claudine dry-run <event> [--provider]` | Test event handling without side effects |
| `claudine about` | Rich help documentation |
| `claudine completions <shell>` | Generate shell completions |
| `claudine uninstall [--keep-config]` | Remove hooks from all agents |

## Configuration

Configuration is stored in `~/.claudine/config.json` (user-scoped) or `<repo>/.claudine/config.json` (project-scoped).

## Packages

| Package | Description |
|---------|-------------|
| [claudine (lib)](./lib/) | Event model, provider adapters, dispatch pipeline, skill linking |
| [claudine-cli](./cli/) | Binary `claudine` — setup wizard, hook inspection, link management |

## Documentation

- [Shared Event Model](./docs/shared-event-model.md) - Universal event abstraction (16 events)
- [Agent Configuration](./docs/agent-configuration.md) - Per-provider setup details
- [Skill Linking](./docs/skill-linking.md) - Cross-provider skill synchronization
- [Log Reporting](./docs/log-reporting.md) - JSONL-to-SQLite reporting model and `claudine logs`
- [Provider Hooks](./docs/hooks/) - Per-provider hook specifications

## Key Dependencies

Uses the following libraries from this monorepo:

- `biscuit-hash` - xxHash content hashing for skill deduplication
- `biscuit-speaks` - Text-to-speech for speak actions
- `biscuit-terminal` - Terminal detection and rich output (tables, prose)
- `darkmatter` - Markdown rendering for `about` command
- `playa` - Sound effect playback (88 embedded effects)
- `sniff` - System and environment detection (OS, hardware, git, repo context)
