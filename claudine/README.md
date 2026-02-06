# Claudine

> Claude Code's ex-girlfriend who knows Claude's inner secrets but is now dating other Agents

Universal event handler and skill linker for agentic CLIs. Provides consistent hook/event responses across 7 providers while synchronizing skills between those that support them.

## Supported Providers

| Provider | Events | Skills | Config Method |
|----------|:------:|:------:|---------------|
| Claude Code | ✓ | ✓ | `settings.json` hooks |
| Codex CLI | ✓ | ✓ | `config.toml` notify + JSONL stream |
| Gemini CLI | ✓ | ✓ | `settings.json` hooks |
| Goose | ✓ | - | Stream-json + env var |
| Kimi Code | ✓ | - | Wire mode JSON-RPC |
| OpenCode | ✓ | ✓ | `opencode.json` plugins |
| Qwen Code | ✓ | ✓ | Stream-json output |

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
```

## CLI Commands

| Command | Description |
|---------|-------------|
| `claudine init` | Interactive setup wizard |
| `claudine init --quick` | Quick setup with sensible defaults |
| `claudine init --repo` | Project-scoped configuration |
| `claudine hooks` | Show registered hooks for all providers |
| `claudine hooks <provider>` | Detailed view for specific provider |
| `claudine hooks --support` | Provider event support matrix |
| `claudine hooks --mapping` | Native event name mappings |
| `claudine link` | Sync skills across providers |
| `claudine link --dry-run` | Preview skill linking |
| `claudine sync` | Re-apply hook registrations |
| `claudine sync --dry-run` | Preview sync changes |
| `claudine handle <event>` | Process event (called by hooks) |
| `claudine dry-run <event>` | Test event handling |
| `claudine about` | Rich help documentation |
| `claudine completions <shell>` | Generate shell completions |
| `claudine uninstall` | Remove hooks from all agents |

## Configuration

Configuration is stored in `~/.hooker` (user-scoped) or `<repo>/.hooker` (project-scoped) as JSON.

## Documentation

- [Shared Event Model](./docs/shared-event-model.md) - Universal event abstraction
- [Agent Configuration](./docs/agent-configuration.md) - Per-provider setup details
- [Skill Linking](./docs/skill-linking.md) - Cross-provider skill synchronization
- [Provider Hooks](./docs/hooks/) - Per-provider hook specifications

## Key Dependencies

Uses the following libraries from this monorepo:

- `darkmatter` - Rich terminal markdown rendering
- `biscuit-speaks` - Text-to-speech functionality
- `biscuit-terminal` - Terminal detection and rendering
- `playa` - Sound effect playback
- `sniff` - System and environment detection
- `biscuit-hash` - Content hashing for skill deduplication
