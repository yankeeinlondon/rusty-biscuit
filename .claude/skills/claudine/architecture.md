# Claudine Architecture

Deep technical documentation for Claudine's event model, provider adapters, and configuration system.

## Shared Event Model

### AgenticEvent Enum

```rust
pub enum AgenticEvent {
    // Session lifecycle
    SessionStart,
    SessionEnd,
    
    // User input
    BeforePrompt,
    
    // Tool lifecycle
    BeforeTool,
    AfterTool,
    ToolError,
    
    // Permission
    PermissionRequest,
    
    // Turn lifecycle
    TurnComplete,
    TurnError,
    
    // Subagent lifecycle
    SubagentStart,
    SubagentStop,
    
    // LLM interaction
    BeforeModel,
    AfterModel,
    
    // Context management
    BeforeCompact,
    
    // Notifications
    Notification,
}
```

### EventAction Enum

```rust
pub enum EventAction {
    Speak { message: String },
    Log { target: LogTarget },
    Report { handler: Option<ReportHandler> },
    SoundEffect { name: String, volume: f32, speed: f32 },
}

pub enum LogTarget {
    Server { url: Url },
    LocalFile { path: PathBuf },
}
```

### EventMeta

Every event carries normalized metadata:

```rust
pub struct EventMeta {
    pub provider: Provider,
    pub event: AgenticEvent,
    pub timestamp: DateTime<Utc>,
    pub session_id: Option<String>,
    pub cwd: Option<String>,
    pub tool_name: Option<String>,
    pub tool_input: Option<Value>,
    pub tool_response: Option<Value>,
    pub error: Option<String>,
    pub prompt: Option<String>,
    pub env: EnvironmentContext,
    pub extra: HashMap<String, Value>,
}
```

## Environment Context

Detected once at session start using `sniff_lib`:

```rust
pub struct EnvironmentContext {
    pub os: OsContext,
    pub hardware: HardwareContext,
    pub git: Option<GitContext>,
    pub repo: Option<RepoContext>,
    pub primary_language: Option<String>,
}
```

### Sniff Configuration

```rust
let config = SniffConfig::new()
    .base_dir(cwd)
    .deep(false)          // No network calls
    .commit_count(1)      // Only HEAD
    .skip_network();      // Skip network interfaces
```

## Cross-Provider Event Mapping

| Claudine | Claude | Codex | Gemini | OpenCode | Roo |
|----------|--------|-------|--------|----------|-----|
| `SessionStart` | `SessionStart` | `thread.started` | `SessionStart` | `session.created` | — |
| `SessionEnd` | `SessionEnd` | — | `SessionEnd` | `session.deleted` | — |
| `BeforePrompt` | `UserPromptSubmit` | — | `BeforeAgent` | `chat.message` | — |
| `BeforeTool` | `PreToolUse` | — | `BeforeTool` | `tool.execute.before` | — |
| `AfterTool` | `PostToolUse` | `item.completed` | `AfterTool` | `tool.execute.after` | `tool_result` |
| `ToolError` | `PostToolUseFailure` | — | — | — | `taskToolFailed` |
| `PermissionRequest` | `PermissionRequest` | — | `Notification` | `permission.ask` | `waitingForInput` |
| `TurnComplete` | `Stop` | `turn.completed` | `AfterAgent` | `session.idle` | `taskCompleted` |
| `TurnError` | — | `turn.failed` | — | `session.error` | `error` |
| `SubagentStart` | `SubagentStart` | — | — | — | `taskSpawned` |
| `SubagentStop` | `SubagentStop` | — | — | — | `taskDelegationCompleted` |
| `BeforeModel` | — | — | `BeforeModel` | `chat.params` | — |
| `AfterModel` | — | — | `AfterModel` | — | — |
| `BeforeCompact` | `PreCompact` | — | `PreCompress` | `session.compacting` | `session.compacted` |
| `Notification` | `Notification` | — | `Notification` | various | — |

## Provider Adapters

### Claude Code Adapter

Claude hooks receive JSON on stdin:

```json
{
  "hook_event_name": "PreToolUse",
  "tool_name": "Bash",
  "tool_use_id": "toolu_01ABC...",
  "tool_input": {
    "command": "npm test",
    "description": "Run tests"
  }
}
```

Return JSON with control fields:

```json
{
  "hookSpecificOutput": {
    "hookEventName": "PreToolUse",
    "permissionDecision": "allow|deny|ask",
    "updatedInput": { "command": "modified" },
    "additionalContext": "Extra context"
  }
}
```

Exit codes: `0`=success, `2`=block action

### Gemini CLI Adapter

Similar to Claude but with differences:
- Hooks have `name` field (Claudine uses `claudine-<event>`)
- Timeouts in milliseconds (not seconds)
- Supports `description` field

### Codex CLI Adapter

Codex uses JSONL stream (`codex exec --json`):

```json
{"type": "thread.started", "thread_id": "abc123"}
{"type": "turn.started"}
{"type": "item.completed", "item": {...}}
{"type": "turn.completed", "usage": {...}}
```

The `notify` hook receives JSON as argv:

```json
{
  "type": "agent-turn-complete",
  "thread-id": "abc123",
  "turn-id": "def456",
  "cwd": "/project",
  "input-messages": [...],
  "last-assistant-message": "..."
}
```

### OpenCode Plugin Bridge

OpenCode requires a TypeScript plugin:

```typescript
import type { Plugin } from "@opencode-ai/plugin"
import { execFile } from "child_process"

export default (async ({ client, project }) => {
  return {
    event: async ({ event }) => {
      if (event.type === "session.created") {
        execFile("claudine", ["handle", "session_start"], {
          input: JSON.stringify({
            provider: "opencode",
            event_type: event.type,
            properties: event.properties,
            cwd: project.worktree,
          })
        })
      }
    },
    "tool.execute.before": async (input, output) => {
      // Handle before_tool
    },
    "tool.execute.after": async (input, output) => {
      // Handle after_tool
    },
  }
}) satisfies Plugin
```

### Roo Code Adapter

Roo Code has no native hooks. The wrapper parses NDJSON:

```bash
claudine start roo  # Wraps: roo --output-format stream-json
```

Events:

```json
{"type": "system", ...}
{"type": "assistant", "content": "...", "id": 1}
{"type": "tool_use", "tool_use": {"name": "Bash", "input": {...}}}
{"type": "tool_result", "tool_result": {"name": "Bash", "output": "..."}}
{"type": "result", "success": true, ...}
```

## Configuration Schema

### HookerConfig

```rust
pub struct HookerConfig {
    pub version: String,
    #[serde(default)]
    pub settings: GlobalSettings,
    pub events: HashMap<AgenticEvent, EventBinding>,
}

pub struct GlobalSettings {
    pub default_log_target: Option<LogTarget>,
    pub tts: Option<TtsSettings>,
}

pub struct EventBinding {
    #[serde(default = "default_true")]
    pub enabled: bool,
    #[serde(default)]
    pub actions: Vec<EventAction>,
    pub matcher: Option<String>,
    #[serde(default)]
    pub overrides: HashMap<Provider, ProviderOverride>,
}

pub struct ProviderOverride {
    #[serde(default = "default_true")]
    pub enabled: bool,
    pub actions: Vec<EventAction>,
    pub matcher: Option<String>,
}
```

### Scope Resolution

| Claudine Config | Agent Config Scope |
|-----------------|-------------------|
| `~/.hooker` (user) | Agent's user config |
| `<repo>/.hooker` (repo) | Agent's project config |

## Agent Configurator Trait

```rust
pub trait AgentConfigurator: Send + Sync {
    fn provider(&self) -> Provider;
    fn is_available(&self) -> bool;
    fn user_config_path(&self) -> PathBuf;
    fn project_config_path(&self, project_root: &Path) -> PathBuf;
    fn read_registrations(&self, config_path: &Path) -> Result<Vec<AgenticEvent>>;
    fn register(&self, config_path: &Path, events: &[AgenticEvent]) -> Result<RegistrationResult>;
    fn deregister(&self, config_path: &Path) -> Result<()>;
    fn has_external_changes(&self, config_path: &Path) -> Result<bool>;
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
| `{agent_type}` | `meta.agent_type` |
| `{timestamp}` | `meta.timestamp` |

### Environment Fields

| Placeholder | Field |
|-------------|-------|
| `{env.os}` | `env.os.name` |
| `{env.os_type}` | `env.os.os_type` |
| `{env.os_version}` | `env.os.version` |
| `{env.hostname}` | `env.os.hostname` |
| `{env.arch}` | `env.hardware.arch` |
| `{env.cpu}` | `env.hardware.cpu` |
| `{env.cores}` | `env.hardware.cores` |
| `{env.branch}` | `env.git.branch` |
| `{env.is_dirty}` | `env.git.is_dirty` |
| `{env.head_sha}` | `env.git.head_sha` |
| `{env.head_message}` | `env.git.head_message` |
| `{env.remote}` | `env.git.remote_name` |
| `{env.hosting}` | `env.git.hosting_provider` |
| `{env.is_monorepo}` | `env.repo.is_monorepo` |
| `{env.monorepo_tool}` | `env.repo.monorepo_tool` |
| `{env.language}` | `env.primary_language` |

## Skill Linking Algorithm

### Phase 1: Discovery

Scan each provider's skill directory for `SKILL.md` files.

### Phase 2: Hashing

Compute xxHash of skill directory content:

```rust
fn hash_skill_dir(skill_dir: &Path) -> Result<u64> {
    let mut paths: Vec<_> = collect_files(skill_dir)?;
    paths.sort();
    
    let mut combined = Vec::new();
    for path in &paths {
        let relative = path.strip_prefix(skill_dir)?;
        combined.extend_from_slice(relative.to_string_lossy().as_bytes());
        combined.push(0);
        combined.extend_from_slice(&fs::read(path)?);
        combined.push(0);
    }
    
    Ok(xx_hash_bytes(&combined))
}
```

### Phase 3: Conflict Detection

Group by skill name across providers:

- **One provider** → Link candidate
- **Multiple providers, same hash** → Already in sync
- **Multiple providers, different hashes** → Conflict (manual resolution)
- **Symlink to other provider** → Already linked

### Phase 4: Linking

Create symlinks with appropriate scope:

| Scope | Symlink Type |
|-------|--------------|
| User | Absolute path |
| Repo | Relative path |

## Provider Skill Paths

```rust
pub struct ProviderSkillPaths {
    pub provider: Provider,
    pub user_skills: PathBuf,
    pub repo_skills: Option<PathBuf>,
    pub user_commands: Option<PathBuf>,
    pub repo_commands: Option<PathBuf>,
    pub also_reads_from: Vec<Provider>,
}
```

| Provider | User Skills | Repo Skills | Commands | Also Reads |
|----------|-------------|-------------|----------|------------|
| Claude | `~/.claude/skills/` | `.claude/skills/` | Yes | — |
| Roo | `~/.roo/skills/` | `.roo/skills/` | No | — |
| OpenCode | `~/.config/opencode/skills/` | `.opencode/skills/` | Yes | Claude |
| Gemini | `~/.gemini/skills/` | `.gemini/skills/` | No | — |
| Codex | `~/.codex/skills/` | — | No | — |

## Event Dispatch Flow

```
1. Provider adapter parses raw input → (AgenticEvent, EventMeta)
2. Look up AgenticEvent in HookerConfig.events
3. If no binding or disabled → exit 0
4. Check binding.matcher against metadata
5. Check for provider-specific override
6. Execute each EventAction in order
7. Return exit code to provider
```

## Non-Destructive Config Strategy

1. **Read-Modify-Write** - Parse as `serde_json::Value`, modify only Claudine-owned keys
2. **Claudine-owned markers** - All hook entries use `claudine handle <event>` pattern
3. **Backup before write** - Copy to `~/.claudine/backups/<provider>/<timestamp>.bak`
4. **Atomic writes** - Write temp, then rename
5. **Conflict detection** - Compare against last-known state before writing

## Exit Code Behavior

| Provider | Exit 0 | Exit 1 | Exit 2 |
|----------|--------|--------|--------|
| Claude | Success | Non-blocking error | Block action |
| Gemini | Success | Warning | Block action |
| Codex | Success | — | — |
| OpenCode | Success | — | Error |
| Roo | — | — | — |

## Capability Matrix

| Capability | Claude | Codex | Gemini | OpenCode | Roo |
|------------|--------|-------|--------|----------|-----|
| Observe events | Yes | Yes | Yes | Yes | Yes |
| Block actions | Yes | No | Yes | Yes (throw) | No |
| Modify tool input | Yes | No | Yes | Yes | No |
| Inject context | Yes | No | Yes | Yes | No |
| Async hooks | Yes | N/A | No | N/A | N/A |
