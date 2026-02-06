# Claudine Architecture

Deep technical documentation for Claudine's event model, provider adapters, and configuration system.

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
| turn_complete | ✓ | ✓ | ✓ | ○ | ○ | ✓ | ○ |
| turn_error | - | ○ | - | ○ | ○ | ✓ | ○ |
| subagent_start | ✓ | - | - | ○ | ○ | - | - |
| subagent_stop | ✓ | - | - | ○ | ○ | - | - |
| before_model | - | - | ✓ | - | - | ✓ | - |
| after_model | - | ○ | ✓ | ○ | ○ | ✓ | ○ |
| before_compact | ✓ | - | ✓ | - | ○ | ✓ | - |
| notification | ✓ | ○ | ✓ | ○ | ○ | ✓ | ○ |

**Legend:** ✓ = Hook support (config file), ○ = NonHook (wrapper/proxy required), - = Not supported

## AgenticEvent Enum

```rust
pub enum AgenticEvent {
    SessionStart, SessionEnd,
    BeforePrompt,
    BeforeTool, AfterTool, ToolError,
    PermissionRequest,
    TurnComplete, TurnError,
    SubagentStart, SubagentStop,
    BeforeModel, AfterModel,
    BeforeCompact,
    Notification,
}
```

## EventAction Enum

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

## Provider Adapters

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

## Template Interpolation

### Event Fields

| Placeholder | Field |
|-------------|-------|
| `{provider}` | `meta.provider` |
| `{event}` | `meta.event` |
| `{session_id}` | `meta.session_id` |
| `{tool_name}` | `meta.tool_name` |
| `{error}` | `meta.error` |
| `{prompt}` | `meta.prompt` |
| `{timestamp}` | `meta.timestamp` |

### Context Fields (auto-detected at runtime)

| Namespace | Placeholders |
|-----------|--------------|
| `os.*` | `{os.name}`, `{os.type}`, `{os.version}`, `{os.hostname}` |
| `hardware.*` | `{hardware.arch}`, `{hardware.cpu}`, `{hardware.cores}` |
| `git.*` | `{git.branch}`, `{git.repo_name}`, `{git.repo_org}`, `{git.hosting}`, `{git.is_dirty}`, `{git.head_sha}` |
| `project.*` | `{project.language}`, `{project.is_monorepo}`, `{project.monorepo_tool}` |

Run `claudine hooks --variables` for the complete list.

## Skill Linking Algorithm

1. **Discovery**: Scan provider skill directories for `SKILL.md` files
2. **Hashing**: Compute xxHash of skill directory content
3. **Conflict Detection**: Group by skill name, compare hashes
4. **Linking**: Create symlinks (absolute for user, relative for repo)

## Provider Skill Paths

| Provider | User Skills | Repo Skills |
|----------|-------------|-------------|
| Claude | `~/.claude/skills/` | `.claude/skills/` |
| Codex | `~/.codex/skills/` | `.codex/skills/` |
| Gemini | `~/.gemini/skills/` | `.gemini/skills/` |
| OpenCode | `~/.config/opencode/skills/` | `.opencode/skills/` |
| Qwen | `~/.qwen/skills/` | `.qwen/skills/` |

Note: OpenCode also reads `.claude/skills/` directly

## Event Dispatch Flow

```
1. Provider adapter parses raw input → (AgenticEvent, EventMeta)
2. Look up AgenticEvent in HookerConfig.providers[provider].events
3. If no binding or disabled → exit 0
4. Check binding.matcher against metadata
5. Execute each EventAction in order
6. Return exit code to provider
```

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
