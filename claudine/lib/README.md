# Claudine Library

Core library for the Claudine cross-agent event handling and skill linking system. Provides the event model, provider adapters, dispatch pipeline, configuration management, and skill synchronization logic used by the `claudine` CLI.

## Architecture

The library is organized into five top-level modules:

```
claudine/lib/src/
├── adapters/    → Provider-specific event parsers
├── config/      → Agent detection and hook registration
├── dispatch/    → Event processing pipeline
├── events/      → Normalized event model and types
├── linking/     → Cross-provider skill synchronization
└── error.rs     → ClaudineError enum
```

### Event Model (`events`)

16 normalized lifecycle events that abstract across all 7 provider APIs:

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

Key types:
- `AgenticEvent` — 16-variant enum with snake_case serde, descriptions, payload schemas, and return schemas
- `Provider` — 7-variant enum (Claude, Codex, Gemini, Goose, KimiCode, OpenCode, QwenCode) with slug, docs URL, event support queries, and native event name mappings
- `EventSupportLevel` — `Hook` | `NonHook` | `NotSupported` per provider-event pair
- `EventAction` — 5-variant tagged enum: `Speak`, `Log`, `Report`, `Run`, `SoundEffect`
- `EventMeta` — Normalized event metadata (provider, event, tool name, error, prompt, session ID, timestamps, environment context)
- `HookerConfig` / `ProviderConfig` / `EventBinding` — Configuration types
- `EnvironmentContext` — Auto-detected OS, hardware, git, and repo context (via `sniff`)

### Adapters (`adapters`)

Each provider has its own adapter implementing the `ProviderAdapter` trait:

| Adapter | Parses | Status |
|---------|--------|--------|
| `claude` | `hook_event_name` field from settings.json hooks | Implemented |
| `codex` | JSONL stream fields + notify hook | Implemented |
| `gemini` | Settings.json hook events | Implemented |
| `opencode` | Plugin-based event names | Implemented |
| `goose` | Stream-json + env var | Stub (needs wrapper) |
| `kimicode` | Wire mode JSON-RPC | Stub (needs wrapper) |
| `qwen` | Stream-json output | Stub (needs wrapper) |

The `adapter_for(provider)` factory returns the appropriate adapter. Each adapter normalizes the provider's native JSON payload into `(AgenticEvent, EventMeta)`.

### Dispatch Pipeline (`dispatch`)

The core event processing pipeline runs in 7 steps:

1. **Select adapter** — `adapter_for(provider)`
2. **Parse event** — adapter normalizes raw JSON into `(AgenticEvent, EventMeta)`
3. **Load config** — merges user (`~/.hooker`) and repo (`.hooker`) configs
4. **Look up binding** — finds `EventBinding` for this provider + event
5. **Resolve actions** — extracts enabled flag, actions list, optional matcher
6. **Check matcher** — regex match against event metadata (filters actions)
7. **Execute actions** — runs each action via `runner::execute_actions()`

Sub-modules:
- `loader` — Config file discovery, loading, and user+repo merge logic
- `template` — `{placeholder}` interpolation engine with 29 variables across 5 categories
- `matcher` — Regex-based event filtering
- `resolver` — Extracts enabled/actions/matcher from bindings
- `runner` — Executes actions (TTS via biscuit-speaks, logging, shell commands, sound effects via playa)

**Config merge strategy**: repo-level provider configs completely replace user-level; global settings merge field-by-field with repo taking precedence.

### Template Variables (`dispatch::template`)

28 variables in 5 categories, available for `{placeholder}` interpolation in speak and report templates:

| Category | Variables |
|----------|-----------|
| Event | `{provider}`, `{event}`, `{timestamp}`, `{session_id}`, `{cwd}`, `{tool_name}`, `{error}`, `{prompt}`, `{agent_type}`, `{notification_type}` |
| OS | `{os.name}`, `{os.type}`, `{os.version}`, `{os.hostname}` |
| Hardware | `{hardware.arch}`, `{hardware.cpu}`, `{hardware.cores}` |
| Git | `{git.branch}`, `{git.is_dirty}`, `{git.head_sha}`, `{git.head_message}`, `{git.remote}`, `{git.hosting}`, `{git.repo_name}`, `{git.repo_org}` |
| Project | `{project.language}`, `{project.is_monorepo}`, `{project.monorepo_tool}` |

Unknown placeholders are left as-is. `None` values render as empty strings.

### Configuration (`config`)

Agent detection and hook registration for all 7 providers:

- `detect_agents()` — returns detected providers with their configurators
- `discover_agents_full()` — all 7 providers with install/registration status
- `AgentConfigurator` trait — `register()`, `deregister()`, `is_registered()`, `registered_events()`, `create_minimal_config()`

Configurators handle each provider's config format:
- **Claude/Gemini**: JSON `settings.json` with hooks array
- **Codex**: TOML `config.toml` with notify section
- **OpenCode**: JSON `opencode.json` with plugins
- **Goose/KimiCode/Qwen**: Wrapper-only (no config-based registration)

Atomic file writes (`config::atomic`) prevent corruption during concurrent access. Config backup utilities (`config::backup`) preserve originals before modification.

### Skill Linking (`linking`)

Cross-provider skill synchronization via symlinks:

**Linkable resources** (4 types): Skill, Command, Agent, Script

**Support levels** per provider: Full, CustomFormat, Limited, None

**Algorithm** (4 phases):
1. **Discovery** — find skills/commands/agents across provider directories
2. **Hashing** — xxHash each resource directory for content deduplication
3. **Analysis** — detect conflicts, candidates, and already-in-sync state
4. **Linking** — create symlinks for candidates

**Provider skill paths**:

| Provider | User scope | Repo scope |
|----------|-----------|------------|
| Claude | `~/.claude/skills/` | `.claude/skills/` |
| Codex | `~/.codex/skills/` | `.codex/skills/` |
| Gemini | `~/.gemini/skills/` | `.gemini/skills/` |
| OpenCode | `~/.config/opencode/skills/` | `.opencode/skills/` |

## Action Execution

| Action | Behavior | Blocking |
|--------|----------|----------|
| `Speak` | TTS via biscuit-speaks with template interpolation | Fire-and-forget (tokio::spawn) |
| `SoundEffect` | Playa embedded effects with volume/speed control | Fire-and-forget (tokio::spawn) |
| `Log` (file) | Append JSONL, creates parent dirs | Synchronous |
| `Log` (server) | POST JSON with 10s timeout | Non-fatal on failure |
| `Report` | Write to stdout with optional template/format | Synchronous |
| `Run` | Execute shell command | Configurable (`blocking` field) |

## Key Dependencies

| Crate | Purpose |
|-------|---------|
| `sniff` | Environment detection (OS, hardware, git, repo) |
| `biscuit-hash` | xxHash for skill content deduplication |
| `playa` | Sound effect playback (53 effects, async) |
| `biscuit-speaks` | TTS for speak actions |
| `serde` / `serde_json` | JSON serialization for configs and events |
| `tokio` | Async runtime for concurrent action execution |
| `regex` | Event matcher pattern compilation |
| `reqwest` | HTTP client for log server POSTing |
| `toml_edit` | Format-preserving TOML edits (Codex config) |
| `thiserror` | Error type derivation |
| `walkdir` | Directory traversal for skill discovery |

## Lessons Learned

- **Config merge is intentionally asymmetric**: repo provider configs fully replace user-level (not merged per-event) to give projects complete control. Settings merge field-by-field because they're global preferences.
- **Goose/Kimi/Qwen adapters are stubs**: these providers use stream-json or wire mode rather than config-based hooks, requiring a wrapper/proxy that isn't yet implemented. The adapter infrastructure is in place for when wrappers are built.
- **Template regex is lazy-compiled**: `LazyLock<Regex>` ensures the `\{([a-z_.]+)\}` pattern compiles once across all interpolation calls.
- **Sound effects are fire-and-forget**: TTS and sound playback spawn tokio tasks to avoid blocking the event pipeline. Log and report actions run inline because they're fast.
- **Atomic writes prevent config corruption**: all config file mutations go through `config::atomic` to handle concurrent hook firings safely.
