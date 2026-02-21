# Claudine Library

Core library for the Claudine cross-agent event handling and skill linking system. Provides the event model, provider adapters, dispatch pipeline, configuration management, agent capability catalog, and skill synchronization logic used by the `claudine` CLI.

## Architecture

The library is organized into eight top-level modules:

```
claudine/lib/src/
├── actions/     → Hook action types and response model
├── adapters/    → Provider-specific event parsers
├── agents/      → Agent capability catalog and registry
├── config/      → Agent detection and hook registration
├── dispatch/    → Event processing pipeline
├── events/      → Normalized event model and types
├── linking/     → Cross-provider skill and command synchronization
├── services/    → Cross-provider policy services (Protect)
└── error.rs     → ClaudineError enum
```

### Actions (`actions`)

Types for hook actions that execute when events fire, and response types for blocking hooks:

- `HookAction` — 6-variant tagged enum: `Speak`, `Log`, `FireAndForget`, `Call`, `Report`, `SoundEffect`
- `HookResponse` — Unified response a hook can return to influence agent behavior (decision, reason, updated input, additional context)
- `HookDecision` — 4-variant enum: `Allow`, `Deny`, `Ask`, `Continue`
- `LogTarget` — File (with daily rotation) or Server (HTTP POST with timeout)
- `ReportHandler` / `ReportFormat` — Report output formatting (Text, Json, Compact)
- `Mapper` / `CompiledMapper` — Transform command output into `HookResponse` (JsonField, JsonObject, ExitCode, Regex)

### Event Model (`events`)

16 normalized lifecycle events that abstract across all 8 provider APIs:

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
- `AgenticEvent` — 16-variant enum with snake_case serde, descriptions, payload schemas, return schemas, and abbreviations
- `Provider` — 8-variant enum (Claude, Codex, Gemini, Goose, KimiCode, OpenCode, QwenCode, RooCode) with slug, docs URL, event support queries, and native event name mappings
- `EventSupportLevel` — `Hook` | `NonHook` | `NotSupported` per provider-event pair
- `EventMeta` — Normalized event metadata (provider, event, tool name, error, prompt, session ID, timestamps, environment context)
- `ResolvedHook` — A fully resolved hook binding ready for execution (event, meta, provider, actions, can_block)
- `HookerConfig` / `ProviderConfig` / `EventBinding` — Configuration types
- `GlobalSettings` / `TtsSettings` / `LinkingSettings` / `CanonicalProviderSettings` — Settings types
- `EnvironmentContext` — Auto-detected OS, hardware, git, and repo context (via `sniff`)

### Agents (`agents`)

Comprehensive capability catalog for all 8 supported agentic CLIs:

- `Agent` trait — shared interface for capability descriptors (`id()`, `capabilities()`, `supports_skills()`, `supports_custom_slash_commands()`, `supports_subagents()`, `validate()`)
- `AgentCapabilities` — full capability model covering meta, docs, config, runtime, skills, commands, subagents, scripts, and confidence
- `AgentId` — 8-variant enum with string slugs and aliases for CLI parsing
- `agent_for(id)` / `all_agents()` / `parse_agent_id(input)` — registry functions
- Per-agent implementations: `ClaudeCodeAgent`, `CodexAgent`, `GeminiCliAgent`, `GooseAgent`, `KimiCodeAgent`, `OpenCodeAgent`, `QwenCliAgent`, `RooCodeAgent`

Each agent descriptor captures: model selection, non-interactive mode, system prompt, permissions, reasoning style, logging, billing, skill paths, command paths, subagent paths, and script support.

### Adapters (`adapters`)

Each provider has its own adapter implementing the `ProviderAdapter` trait:

| Adapter | Parses | Status |
|---------|--------|--------|
| `claude` | `hook_event_name` field from settings.json hooks | Implemented |
| `codex` | JSONL stream fields + notify hook | Implemented |
| `gemini` | Settings.json hook events | Implemented |
| `opencode` | Plugin-based event names | Implemented |
| `goose` | Stream-json + env var (type/event field) | Implemented (non-blocking) |
| `kimicode` | Wire mode JSON-RPC (event_name/method field) | Implemented (blocking: tool, permission) |
| `qwen` | Stream-json output (event_name/type field) | Implemented (blocking: permission) |
| `roo` | Stream-json event emitter (event_name/type field) | Implemented (non-blocking) |

The `adapter_for(provider)` factory returns the appropriate adapter singleton. Each adapter normalizes the provider's native JSON payload into `(AgenticEvent, EventMeta)` and can format `HookResponse` back into provider-native response payloads.

### Dispatch Pipeline (`dispatch`)

The core event processing pipeline runs in 6 steps:

1. **Select adapter** — `adapter_for(provider)`
2. **Parse event** — adapter normalizes raw JSON into `(AgenticEvent, EventMeta)`
3. **Load config** — merges user (`~/.claudine/config.json`) and repo (`.claudine/config.json`) configs, precompiling matcher and mapper regexes
4. **Look up binding** — finds `RuntimeEventBinding` for this provider + event, checks enabled and non-empty actions
5. **Check matcher** — precompiled regex match against event metadata (filters actions)
6. **Execute actions** — runs each action via `runner::execute_actions()`, collecting blocking responses from `Call` actions

Sub-modules:
- `loader` — Config file discovery, loading, merge logic, runtime compilation (matchers + mappers), and config save/validation
- `template` — `{{placeholder}}` Handlebars-style interpolation engine with 28 variables across 5 categories (legacy `{placeholder}` single-brace syntax is deprecated with warnings)
- `matcher` — Regex-based event filtering against tool name, notification type, or error
- `runner` — Executes actions (TTS via biscuit-speaks, logging, shell commands, sound effects via playa, report formatting)

**Config merge strategy**: repo-level provider configs completely replace user-level; global settings merge field-by-field with repo taking precedence.

### Template Variables (`dispatch::template`)

28 variables in 5 categories, available for `{{placeholder}}` interpolation in speak, report, and command templates:

| Category | Variables |
|----------|-----------|
| Event | `{{provider}}`, `{{event}}`, `{{timestamp}}`, `{{session_id}}`, `{{cwd}}`, `{{tool_name}}`, `{{error}}`, `{{prompt}}`, `{{agent_type}}`, `{{notification_type}}` |
| OS | `{{os.name}}`, `{{os.type}}`, `{{os.version}}`, `{{os.hostname}}` |
| Hardware | `{{hardware.arch}}`, `{{hardware.cpu}}`, `{{hardware.cores}}` |
| Git | `{{git.branch}}`, `{{git.is_dirty}}`, `{{git.head_sha}}`, `{{git.head_message}}`, `{{git.remote}}`, `{{git.hosting}}`, `{{git.repo_name}}`, `{{git.repo_org}}` |
| Project | `{{project.language}}`, `{{project.is_monorepo}}`, `{{project.monorepo_tool}}` |

Shell environment variables are also supported via `{{env.VAR_NAME}}` with optional defaults: `{{env.MY_VAR | "fallback"}}`.

Unknown placeholders are left as-is. `None` values render as empty strings.

### Configuration (`config`)

Agent detection and hook registration for all 8 providers:

- `detect_agents()` — returns detected providers with their configurators
- `discover_agents_full()` — all 8 providers with install/registration status (`AgentInfo`)
- `get_configurator(provider)` — returns the configurator for a specific provider
- `AgentConfigurator` trait — `register()`, `deregister()`, `is_registered()`, `registered_events()`, `create_minimal_config()`, `supports_config_registration()`, `registerable_events()`, `is_cli_installed()`

Configurators handle each provider's config format:
- **Claude/Gemini**: JSON `settings.json` with hooks array
- **Codex**: TOML `config.toml` with notify section
- **OpenCode**: JSON `opencode.json` with plugins
- **Goose/KimiCode/Qwen/Roo**: Wrapper-only (no config-based registration)

Atomic file writes (`config::atomic`) prevent corruption during concurrent access. Config backup utilities (`config::backup`) preserve originals before modification.

### Skill Linking (`linking`)

Cross-provider resource synchronization via symlinks and format-converted derived artifacts. See [linking-strategy.md](../../.claude/skills/claudine/linking-strategy.md) for the full algorithm deep dive.

**Linkable resources** (4 types): Skill, Command, Agent, Script

**Support levels** per provider: Full, CustomFormat, Limited, None

**Sync strategies**:
- **Direct symlinks** for same-format providers (Markdown to Markdown)
- **Derived artifacts** for different-format providers (Markdown to TOML for Gemini commands, Markdown to YAML for Goose/KimiCode/Roo agents), with embedded hash markers for staleness detection

**Algorithm** (6 phases):
1. **Canonical selection** — elect one provider as the source of truth per `(scope, resource_type)` pair, preferring providers with existing valid assets
2. **Discovery** — scan provider directories for skills, commands, agents, and scripts
3. **Hashing** — xxHash each resource (recursive walk for skill directories, file content for single files)
4. **Conflict analysis** — classify resources as LinkCandidate, InSync, Conflict, or AlreadyLinked; also-reads-from providers are excluded from link targets to avoid redundant symlinks
5. **Compatibility classification** — parse canonical frontmatter, apply deterministic upgrades (alias duplication, name derivation), check required properties per target provider
6. **Apply** — create symlinks (absolute for user scope, relative for repo scope) or generate format-converted derived artifacts; never overwrites real directories

**Provider skill paths** (all 8 providers):

| Provider | User scope | Repo scope | Also reads from |
|----------|-----------|------------|-----------------|
| Claude | `~/.claude/skills/` | `.claude/skills/` | -- |
| Codex | `~/.codex/skills/` | `.codex/skills/` | `.claude/skills`, `.agents/skills` |
| Gemini | `~/.gemini/skills/` | `.gemini/skills/` | -- |
| Goose | `~/.config/goose/skills/` | `.goose/skills/` | `.claude/skills`, `.agents/skills` |
| KimiCode | `~/.config/agents/skills/` | `.kimi/skills/` | `.claude/skills`, `.agents/skills`, `.codex/skills` |
| OpenCode | `~/.config/opencode/skills/` | `.opencode/skills/` | `.claude/skills`, `.agents/skills` |
| QwenCode | `~/.qwen/skills/` | `.qwen/skills/` | -- |
| RooCode | `~/.roo/skills/` | `.roo/skills/` | -- |

Additional sub-modules: `canonical` (canonical provider selection), `capabilities` (provider resource support metadata with required/optional property schemas), `compatibility` (candidate/reference classification with alias duplication and name derivation), `conflict` (sync status analysis with also-reads-from awareness), `detector` (per-resource-type detectors via `LinkDetector` trait), `discovery` (legacy skill and command discovery), `execution` (analyze and apply resource links with derived artifact generation), `hashing` (xxHash content dedup with symlink resolution), `model` (data types including `ResourceReference` 9-variant state machine), `paths` (provider path resolution with repo-root awareness), `report` (link report types), `symlink` (symlink creation with relative path support).

### Services (`services`)

Cross-provider policy engines that operate on normalized event context:

- `protect` — Capability-aware policy evaluation service used to normalize safety decisions (`allow`, `ask`, `stop`, `advisory`) across providers with different control surfaces.

## Action Execution

| Action | Behavior | Blocking |
|--------|----------|----------|
| `Speak` | TTS via biscuit-speaks with template interpolation | Fire-and-forget (tokio::spawn) |
| `SoundEffect` | Playa embedded effects with volume/speed control | Fire-and-forget (tokio::spawn_blocking) |
| `Log` (file) | Append JSONL, creates parent dirs, supports daily rotation | Synchronous |
| `Log` (server) | POST JSON with configurable timeout and headers | Non-fatal on failure |
| `Report` | Write to stdout with optional template/format (Text, Json, Compact) | Synchronous |
| `FireAndForget` | Execute command asynchronously without waiting | Fire-and-forget (tokio::spawn) |
| `Call` | Execute command synchronously with timeout and response mapper | Blocking (returns `HookResponse`) |

## Key Dependencies

| Crate | Purpose |
|-------|---------|
| `sniff` | Environment detection (OS, hardware, git, repo) |
| `biscuit-hash` | xxHash for skill content deduplication |
| `playa` | Sound effect playback (async) |
| `biscuit-speaks` | TTS for speak actions |
| `serde` / `serde_json` | JSON serialization for configs and events |
| `tokio` | Async runtime for concurrent action execution |
| `regex` | Event matcher and mapper pattern compilation |
| `reqwest` | HTTP client for log server POSTing |
| `toml_edit` | Format-preserving TOML edits (Codex config) |
| `thiserror` | Error type derivation |
| `walkdir` | Directory traversal for skill discovery |
| `chrono` | Timestamp handling and daily log rotation |
| `dirs` | Home directory resolution |
| `serde_yaml` | YAML parsing for skill frontmatter |
| `url` | URL parsing and validation |

## Lessons Learned

- **Config merge is intentionally asymmetric**: repo provider configs fully replace user-level (not merged per-event) to give projects complete control. Settings merge field-by-field because they're global preferences.
- **All 8 adapters are implemented**: each provider adapter has full event mapping, metadata extraction, and tests. Claude, Gemini, OpenCode, and Codex use config-based hooks; Goose, KimiCode, Qwen, and Roo parse stream-json or wire-mode payloads directly. KimiCode and Qwen support blocking responses; Goose and Roo are observation-only.
- **Template regex is lazy-compiled**: `LazyLock<Regex>` ensures the Handlebars `\{\{\s*([^{}]+?)\s*\}\}` pattern compiles once across all interpolation calls.
- **Sound effects are fire-and-forget**: TTS and sound playback spawn tokio tasks to avoid blocking the event pipeline. Log and report actions run inline because they're fast.
- **Atomic writes prevent config corruption**: all config file mutations go through `config::atomic` to handle concurrent hook firings safely.
- **Runtime config precompiles regexes**: matcher patterns and Call action mapper regexes are compiled once at config load time, failing fast on invalid patterns with contextual error messages.
- **Legacy single-brace templates are deprecated**: `{placeholder}` is automatically rewritten to `{{placeholder}}` with a tracing warning. New configs should use Handlebars-style double braces.
