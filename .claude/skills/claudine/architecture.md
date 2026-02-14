# Claudine Architecture

Deep technical documentation for Claudine's event model, provider adapters, dispatch pipeline, and configuration system.

## Library Module Structure

```
claudine/lib/src/
├── adapters/    → Provider-specific event parsers (ProviderAdapter trait)
├── config/      → Agent detection, hook registration, atomic writes, backups
├── dispatch/    → Event processing pipeline (loader, template, matcher, resolver, runner)
├── events/      → Normalized event model and types (16 events, 7 providers)
├── linking/     → Cross-provider skill synchronization (4 resource types)
└── error.rs     → ClaudineError enum
```

## Event Support Matrix

| Event | Claude | Codex | Gemini | Goose | Kimi | OpenCode | Qwen |
|-------|:------:|:-----:|:------:|:-----:|:----:|:--------:|:----:|
| session_start | ✓ | ○ | ✓ | - | - | ✓ | - |
| session_end | ✓ | - | ✓ | - | - | ✓ | - |
| before_prompt | ✓ | ○ | ✓ | - | ○ | ✓ | - |
| before_tool | ✓ | ○ | ✓ | - | ○ | ✓ | - |
| after_tool | ✓ | ○ | ✓ | - | ○ | ✓ | - |
| tool_error | ✓ | ○ | - | - | ○ | - | - |
| permission_request | ✓ | - | - | - | ○ | ✓ | - |
| human_in_the_loop | ✓ | - | - | - | - | - | - |
| turn_complete | ✓ | ✓ | ✓ | ○ | ○ | ✓ | ○ |
| turn_error | - | ○ | - | ○ | ○ | ✓ | ○ |
| subagent_start | ✓ | - | - | ○ | ○ | - | - |
| subagent_stop | ✓ | - | - | ○ | ○ | - | - |
| before_model | - | - | ✓ | - | - | ✓ | - |
| after_model | - | ○ | ✓ | ○ | ○ | ✓ | ○ |
| before_compact | ✓ | - | ✓ | - | ○ | ✓ | - |
| notification | ✓ | ○ | ✓ | ○ | ○ | ✓ | ○ |

**Legend:** ✓ = Hook support (config file), ○ = NonHook (wrapper/proxy required), - = Not supported

## Key Types

### AgenticEvent Enum

16-variant enum with snake_case serde, descriptions, payload schemas, and return schemas:

```rust
pub enum AgenticEvent {
    SessionStart, SessionEnd,
    BeforePrompt,
    BeforeTool, AfterTool, ToolError,
    PermissionRequest, HumanInTheLoop,
    TurnComplete, TurnError,
    SubagentStart, SubagentStop,
    BeforeModel, AfterModel,
    BeforeCompact,
    Notification,
}
```

### Provider Enum

7-variant enum with slug, docs URL, event support queries, and native event name mappings:

- `EventSupportLevel` — `Hook` | `NonHook` | `NotSupported` per provider-event pair

### EventAction Enum

5-variant tagged enum:

```rust
pub enum EventAction {
    Speak { message: String },
    Log { target: LogTarget },
    Report { handler: Option<ReportHandler> },
    SoundEffect { name: String, volume: f32, speed: f32 },
    Run { command: String, args: Option<Vec<String>>, blocking: bool },
}

pub enum LogTarget {
    Server { url: Url },
    LocalFile { path: PathBuf },
}
```

### EventMeta

Normalized event metadata: provider, event, tool name, error, prompt, session ID, timestamps, environment context.

### EnvironmentContext

Auto-detected OS, hardware, git, and repo context (via `sniff`).

## Provider Adapters

Each provider has its own adapter implementing the `ProviderAdapter` trait. The `adapter_for(provider)` factory returns the appropriate adapter. Each adapter normalizes the provider's native JSON payload into `(AgenticEvent, EventMeta)`.

| Adapter | Parses | Status |
|---------|--------|--------|
| `claude` | `hook_event_name` field from settings.json hooks | Implemented |
| `codex` | JSONL stream fields + notify hook | Implemented |
| `gemini` | Settings.json hook events | Implemented |
| `opencode` | Plugin-based event names | Implemented |
| `goose` | Stream-json + env var | Stub (needs wrapper) |
| `kimicode` | Wire mode JSON-RPC | Stub (needs wrapper) |
| `qwen` | Stream-json output | Stub (needs wrapper) |

### Claude Code

Hooks receive JSON on stdin, return JSON with control fields:

```json
// Input
{
  "hook_event_name": "PreToolUse",
  "tool_name": "Bash",
  "tool_input": { "command": "npm test" }
}

// Output (optional)
{
  "hookSpecificOutput": {
    "permissionDecision": "allow|deny|ask",
    "updatedInput": { "command": "modified" }
  }
}
```

Exit codes: `0` = success, `2` = block action

**Stdin auto-detection**: provider is detected from JSON payload structure (`hook_event_name` → Claude, `type` + `thread_id` → Codex, etc.) so hooks don't need `--provider`.

### Codex CLI

Uses JSONL stream (`codex exec --json`) plus `notify` hook for turn_complete:

```json
{"type": "thread.started", "thread_id": "abc123"}
{"type": "item.completed", "item": {...}}
{"type": "turn.completed", "usage": {...}}
```

### Gemini CLI

Similar to Claude but with differences:
- Hooks have `name` field (Claudine uses `claudine-<event>`)
- Timeouts in milliseconds

### Goose

Events via stream-json output and `GOOSE_STATUS_HOOK` env var (NonHook):

```json
{"type": "complete", ...}
{"type": "message", ...}
{"type": "notification", ...}
```

### Kimi Code

Wire mode JSON-RPC proxy (NonHook):

```json
{"method": "TurnBegin", "params": {...}}
{"method": "ToolCall", "params": {...}}
{"method": "TurnEnd", "params": {...}}
```

### OpenCode

Plugin-based hooks via `opencode.json`:

```typescript
export default (async ({ client, project }) => {
  return {
    event: async ({ event }) => {
      if (event.type === "session.created") {
        execFile("claudine", ["handle", "session_start"], {...})
      }
    }
  }
}) satisfies Plugin
```

### Qwen Code

Limited events via stream-json output (NonHook):

```json
{"type": "result", ...}
{"type": "assistant", ...}
{"type": "system", ...}
```

## Dispatch Pipeline

The core event processing pipeline runs in 7 steps:

1. **Select adapter** — `adapter_for(provider)`
2. **Parse event** — adapter normalizes raw JSON into `(AgenticEvent, EventMeta)`
3. **Load config** — merges user (`~/.hooker`) and repo (`.hooker`) configs
4. **Look up binding** — finds `EventBinding` for this provider + event
5. **Resolve actions** — extracts enabled flag, actions list, optional matcher
6. **Check matcher** — regex match against event metadata (filters actions)
7. **Execute actions** — runs each action via `runner::execute_actions()`

### Dispatch Sub-modules

- `loader` — Config file discovery, loading, and user+repo merge logic
- `template` — `{placeholder}` interpolation engine with 28 variables across 5 categories
- `matcher` — Regex-based event filtering
- `resolver` — Extracts enabled/actions/matcher from bindings
- `runner` — Executes actions (TTS via biscuit-speaks, logging, shell commands, sound effects via playa)

### Config Merge Strategy

Repo-level provider configs completely replace user-level (not merged per-event) to give projects complete control. Settings merge field-by-field because they're global preferences.

## Configuration Schema

```rust
pub struct HookerConfig {
    pub version: String,
    pub settings: GlobalSettings,
    pub providers: HashMap<Provider, ProviderConfig>,
}

pub struct ProviderConfig {
    pub events: HashMap<AgenticEvent, EventBinding>,
}

pub struct EventBinding {
    pub enabled: bool,
    pub actions: Vec<EventAction>,
    pub matcher: Option<String>,  // Regex filter
}
```

### Config Management

- `detect_agents()` — returns detected providers with their configurators
- `discover_agents_full()` — all 7 providers with install/registration status
- `AgentConfigurator` trait — `register()`, `deregister()`, `is_registered()`, `registered_events()`, `create_minimal_config()`

Configurators handle each provider's config format:
- **Claude/Gemini**: JSON `settings.json` with hooks array
- **Codex**: TOML `config.toml` with notify section (format-preserving via `toml_edit`)
- **OpenCode**: JSON `opencode.json` with plugins
- **Goose/KimiCode/Qwen**: Wrapper-only (no config-based registration)

Atomic file writes (`config::atomic`) prevent corruption during concurrent access. Config backup utilities (`config::backup`) preserve originals before modification.

## Template Interpolation

28 variables in 5 categories. Template regex is lazy-compiled via `LazyLock<Regex>`.

### Event Fields

| Placeholder | Field |
|-------------|-------|
| `{provider}` | `meta.provider` |
| `{event}` | `meta.event` |
| `{session_id}` | `meta.session_id` |
| `{cwd}` | `meta.cwd` |
| `{tool_name}` | `meta.tool_name` |
| `{error}` | `meta.error` |
| `{prompt}` | `meta.prompt` |
| `{timestamp}` | `meta.timestamp` |
| `{agent_type}` | `meta.agent_type` |
| `{notification_type}` | `meta.notification_type` |

### Context Fields (auto-detected at runtime)

| Namespace | Placeholders |
|-----------|--------------|
| `os.*` | `{os.name}`, `{os.type}`, `{os.version}`, `{os.hostname}` |
| `hardware.*` | `{hardware.arch}`, `{hardware.cpu}`, `{hardware.cores}` |
| `git.*` | `{git.branch}`, `{git.is_dirty}`, `{git.head_sha}`, `{git.head_message}`, `{git.remote}`, `{git.hosting}`, `{git.repo_name}`, `{git.repo_org}` |
| `project.*` | `{project.language}`, `{project.is_monorepo}`, `{project.monorepo_tool}` |

Unknown placeholders are left as-is. `None` values render as empty strings.

## Skill Linking

Cross-provider skill synchronization via symlinks.

### Linkable Resources (4 types)

Skill, Command, Agent, Script

### Support Levels per Provider

Full, CustomFormat, Limited, None

### Algorithm (4 phases)

1. **Discovery** — find skills/commands/agents across provider directories
2. **Hashing** — xxHash each resource directory for content deduplication
3. **Analysis** — detect conflicts, candidates, and already-in-sync state
4. **Linking** — create symlinks for candidates

### Provider Skill Paths

| Provider | User Scope | Repo Scope |
|----------|-----------|------------|
| Claude | `~/.claude/skills/` | `.claude/skills/` |
| Codex | `~/.codex/skills/` | `.codex/skills/` |
| Gemini | `~/.gemini/skills/` | `.gemini/skills/` |
| OpenCode | `~/.config/opencode/skills/` | `.opencode/skills/` |

Note: OpenCode also reads `.claude/skills/` directly

## Exit Code Behavior

| Provider | Exit 0 | Exit 1 | Exit 2 |
|----------|--------|--------|--------|
| Claude | Success | Non-blocking error | Block action |
| Gemini | Success | Warning | Block action |
| Codex | Success | — | — |
| OpenCode | Success | — | Error |

## Capability Matrix

| Capability | Claude | Codex | Gemini | OpenCode |
|------------|:------:|:-----:|:------:|:--------:|
| Observe events | ✓ | ✓ | ✓ | ✓ |
| Block actions | ✓ | - | ✓ | ✓ |
| Modify tool input | ✓ | - | ✓ | ✓ |
| Inject context | ✓ | - | ✓ | ✓ |

## Key Lessons

- **Goose/Kimi/Qwen adapters are stubs**: these providers use stream-json or wire mode rather than config-based hooks, requiring a wrapper/proxy that isn't yet implemented. The adapter infrastructure is in place for when wrappers are built.
- **Sound effects are fire-and-forget**: TTS and sound playback spawn tokio tasks to avoid blocking the event pipeline. Log and report actions run inline because they're fast.
- **Atomic writes prevent config corruption**: all config file mutations go through `config::atomic` to handle concurrent hook firings safely.
