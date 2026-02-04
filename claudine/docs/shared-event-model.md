# Shared Event Model

## Overview

Claudine abstracts the hook/event systems of five agentic CLIs (Claude Code, Codex CLI, Gemini CLI, OpenCode, Roo Code) into a single, provider-neutral event model. The abstraction allows users to define reactions to agent lifecycle events once and have them execute consistently regardless of which agent is driving the session.

The model is defined by three core types:

1. **`AgenticEvent`** — an enum of normalized event names
2. **`EventAction`** — an enum of reactions that can fire when an event occurs
3. **`HookerConfig`** — the serializable schema for `~/.hooker`, the user configuration file

All types are fully serializable/deserializable to JSON via `serde`.

---

## Cross-Provider Event Mapping

The table below shows how each shared event maps to the native event in each provider. A dash (`—`) means the provider has no direct equivalent.

| AgenticEvent | Claude Code | Codex CLI | Gemini CLI | OpenCode | Roo Code |
|---|---|---|---|---|---|
| `SessionStart` | `SessionStart` | `thread.started` | `SessionStart` | `session.created` event | — (task-based) |
| `SessionEnd` | `SessionEnd` | — | `SessionEnd` | `session.deleted` event | — |
| `BeforePrompt` | `UserPromptSubmit` | — | `BeforeAgent` | `chat.message` hook | — |
| `BeforeTool` | `PreToolUse` | — | `BeforeTool` | `tool.execute.before` | — |
| `AfterTool` | `PostToolUse` | `item.completed` (command_execution) | `AfterTool` | `tool.execute.after` | `tool_result` (JSON stream) |
| `ToolError` | `PostToolUseFailure` | — | — | — | `taskToolFailed` event |
| `PermissionRequest` | `PermissionRequest` | — | `Notification` (ToolPermission) | `permission.ask` hook | `waitingForInput` (ask) |
| `TurnComplete` | `Stop` | `turn.completed` / `notify(agent-turn-complete)` | `AfterAgent` | `session.idle` event | `taskCompleted` |
| `TurnError` | — | `turn.failed` | — | `session.error` event | `error` event |
| `SubagentStart` | `SubagentStart` | — | — | — | `taskSpawned` |
| `SubagentStop` | `SubagentStop` | — | — | — | `taskDelegationCompleted` |
| `BeforeModel` | — | — | `BeforeModel` | `chat.params` hook | — |
| `AfterModel` | — | — | `AfterModel` | — | — |
| `BeforeCompact` | `PreCompact` | — | `PreCompress` | `experimental.session.compacting` | `session.compacted` event |
| `Notification` | `Notification` | — | `Notification` | various events | — |

### Naming Rationale

- **Before/After** prefix chosen over Pre/Post because 3 of 5 agents (Gemini, OpenCode, Roo) use this pattern. Claude's `Pre`/`Post` prefix is the outlier.
- **`BeforePrompt`** over `UserPromptSubmit` because it aligns with Gemini's `BeforeAgent` and OpenCode's `chat.message` — all fire before the user's prompt is processed. "Prompt" is the most universally understood term.
- **`TurnComplete`** over `Stop` because Codex uses `turn.completed`, Roo uses `taskCompleted`, and Gemini uses `AfterAgent`. The word "turn" is the industry term for one request/response cycle.
- **`BeforeCompact`** over `PreCompact`/`PreCompress` — follows the Before/After convention and "compact" is more precise than "compress" (it's context summarization, not data compression).

---

## `AgenticEvent` Enum

```rust
use serde::{Deserialize, Serialize};

/// Normalized event names across all supported agentic CLI providers.
///
/// Each variant represents a lifecycle moment that at least 2 of the 5
/// supported providers expose. Provider adapters map their native events
/// to the appropriate variant.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum AgenticEvent {
    // ── Session lifecycle ──────────────────────────────────────────

    /// Agent session has started, resumed, or been cleared.
    ///
    /// ## Provider coverage
    /// Claude (SessionStart), Codex (thread.started),
    /// Gemini (SessionStart), OpenCode (session.created)
    SessionStart,

    /// Agent session has ended or been terminated.
    ///
    /// ## Provider coverage
    /// Claude (SessionEnd), Gemini (SessionEnd),
    /// OpenCode (session.deleted)
    SessionEnd,

    // ── User input ─────────────────────────────────────────────────

    /// User prompt submitted, before the agent processes it.
    ///
    /// ## Provider coverage
    /// Claude (UserPromptSubmit), Gemini (BeforeAgent),
    /// OpenCode (chat.message)
    BeforePrompt,

    // ── Tool lifecycle ─────────────────────────────────────────────

    /// Tool call created, before execution begins.
    ///
    /// ## Provider coverage
    /// Claude (PreToolUse), Gemini (BeforeTool),
    /// OpenCode (tool.execute.before)
    BeforeTool,

    /// Tool call completed successfully.
    ///
    /// ## Provider coverage
    /// Claude (PostToolUse), Gemini (AfterTool),
    /// OpenCode (tool.execute.after), Codex (item.completed),
    /// Roo (tool_result)
    AfterTool,

    /// Tool call failed with an error.
    ///
    /// ## Provider coverage
    /// Claude (PostToolUseFailure), Roo (taskToolFailed)
    ToolError,

    // ── Permission ─────────────────────────────────────────────────

    /// Agent is requesting user permission before proceeding.
    ///
    /// ## Provider coverage
    /// Claude (PermissionRequest), Gemini (Notification/ToolPermission),
    /// OpenCode (permission.ask), Roo (waitingForInput)
    PermissionRequest,

    // ── Turn lifecycle ─────────────────────────────────────────────

    /// Agent has finished its response for the current turn.
    ///
    /// ## Provider coverage
    /// Claude (Stop), Codex (turn.completed + notify),
    /// Gemini (AfterAgent), OpenCode (session.idle),
    /// Roo (taskCompleted)
    TurnComplete,

    /// Agent turn failed with an error.
    ///
    /// ## Provider coverage
    /// Codex (turn.failed), Roo (error)
    TurnError,

    // ── Subagent lifecycle ─────────────────────────────────────────

    /// A subagent/subtask has been spawned.
    ///
    /// ## Provider coverage
    /// Claude (SubagentStart), Roo (taskSpawned)
    SubagentStart,

    /// A subagent/subtask has finished.
    ///
    /// ## Provider coverage
    /// Claude (SubagentStop), Roo (taskDelegationCompleted)
    SubagentStop,

    // ── LLM interaction ────────────────────────────────────────────

    /// Before sending a request to the LLM provider.
    ///
    /// ## Provider coverage
    /// Gemini (BeforeModel), OpenCode (chat.params + chat.headers)
    BeforeModel,

    /// After receiving a response from the LLM provider.
    ///
    /// ## Provider coverage
    /// Gemini (AfterModel)
    AfterModel,

    // ── Context management ─────────────────────────────────────────

    /// Context window compaction/compression is about to occur.
    ///
    /// ## Provider coverage
    /// Claude (PreCompact), Gemini (PreCompress),
    /// OpenCode (experimental.session.compacting)
    BeforeCompact,

    // ── Notifications ──────────────────────────────────────────────

    /// System notification emitted by the agent.
    ///
    /// ## Provider coverage
    /// Claude (Notification), Codex (notify),
    /// Gemini (Notification), OpenCode (various events)
    Notification,
}
```

---

## `EventAction` Enum

Actions define what happens when an event fires. Multiple actions can be attached to a single event.

```rust
use std::path::PathBuf;
use url::Url;

/// An action to execute when an `AgenticEvent` fires.
///
/// Actions are executed in declaration order. Multiple actions can be
/// attached to a single event binding.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum EventAction {
    /// Speak the message aloud using biscuit-speaks TTS.
    ///
    /// The message string supports `{placeholder}` interpolation
    /// from the event's metadata fields (e.g., `{tool_name}`,
    /// `{session_id}`).
    Speak {
        message: String,
    },

    /// Log the event to a remote server or local file.
    Log {
        target: LogTarget,
    },

    /// Report the event into the agent's output stream.
    ///
    /// When `handler` is `None`, a default plain-text summary is
    /// emitted. When provided, the `ReportHandler` controls
    /// formatting and filtering.
    Report {
        handler: Option<ReportHandler>,
    },

    /// Play an embedded sound effect from the playa library.
    ///
    /// Uses playa's 53 built-in effects across 6 categories
    /// (UI, cartoon, reactions, sci-fi, atmosphere, motion).
    /// Playback is non-blocking — the effect plays in the background
    /// and does not delay subsequent actions or the agent's response.
    SoundEffect {
        /// Effect name (e.g., "success", "error", "sad-trombone").
        /// Must match one of playa's built-in effect names.
        name: String,

        /// Playback volume as a float from 0.0 (silent) to 1.0 (full).
        /// Defaults to 1.0 when omitted.
        #[serde(default = "default_volume")]
        volume: f32,

        /// Playback speed multiplier. 1.0 = normal, 1.5 = faster,
        /// 0.75 = slower. Defaults to 1.0 when omitted.
        #[serde(default = "default_speed")]
        speed: f32,
    },
}

fn default_volume() -> f32 {
    1.0
}

fn default_speed() -> f32 {
    1.0
}

/// Where to send log output.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum LogTarget {
    /// POST the event JSON to a remote endpoint.
    Server { url: Url },

    /// Append the event as a JSONL line to a local file.
    LocalFile { path: PathBuf },
}

/// Controls how a `Report` action formats its output.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub struct ReportHandler {
    /// Output format for the report line.
    pub format: ReportFormat,

    /// Optional template string for custom formatting.
    /// Supports `{placeholder}` interpolation from event metadata.
    /// If `None`, uses the default format for the chosen `ReportFormat`.
    pub template: Option<String>,

    /// When true, include the full event metadata in the report.
    /// When false (default), only include the summary.
    #[serde(default)]
    pub include_metadata: bool,
}

/// Format options for report output.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ReportFormat {
    /// Human-readable plain text (default).
    Text,
    /// Structured JSON object.
    Json,
    /// Compact single-line format: `[EVENT] message`.
    Compact,
}
```

### Sound Effect Reference

The `SoundEffect` action plays one of playa's 53 embedded effects. Effects are grouped into 6 feature-gated categories. Claudine enables all categories via the `sound-effects` feature flag.

| Category | Feature flag | Example effects |
|---|---|---|
| UI | `sfx-ui` | `click`, `beep`, `notification`, `error`, `success` |
| Cartoon | `sfx-cartoon` | `boing`, `pop`, `whoosh`, `splat`, `slide-whistle` |
| Reactions | `sfx-reactions` | `applause`, `sad-trombone`, `drumroll`, `rimshot` |
| Sci-Fi | `sfx-scifi` | `laser`, `teleport`, `power-up`, `alarm` |
| Atmosphere | `sfx-atmosphere` | `wind`, `rain`, `thunder`, `fire` |
| Motion | `sfx-motion` | `swoosh`, `impact`, `bounce`, `roll` |

Suggested mappings for common events:

| Event | Suggested effect | Why |
|---|---|---|
| `SessionStart` | `power-up` | Session coming online |
| `SessionEnd` | `notification` | Clean exit signal |
| `TurnComplete` | `success` or `beep` | Subtle completion indicator |
| `TurnError` | `error` | Audible error without TTS overhead |
| `ToolError` | `sad-trombone` | Distinctive failure signal |
| `PermissionRequest` | `notification` or `alarm` | Attention needed |
| `SubagentStart` | `whoosh` | Something launching |
| `SubagentStop` | `pop` | Something finishing |
| `BeforeCompact` | `swoosh` | Context being compressed |

Playback uses playa's player-matching system, which ranks available audio players by capability. The best available player on the host is selected automatically (mpv > FFplay > SoX > afplay > etc.).

---

## Event Metadata

When an event fires, it carries metadata from the underlying provider. This metadata is normalized into a common structure that actions can interpolate against.

```rust
use std::collections::HashMap;
use chrono::{DateTime, Utc};

/// Normalized metadata attached to every fired event.
///
/// Provider adapters populate this from their native event payloads.
/// The `extra` map carries provider-specific fields that don't fit
/// the common schema.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EventMeta {
    /// Which agent provider fired the event.
    pub provider: Provider,

    /// The shared event that was matched.
    pub event: AgenticEvent,

    /// UTC timestamp of when the event was received.
    pub timestamp: DateTime<Utc>,

    /// Session or thread identifier (provider-dependent format).
    pub session_id: Option<String>,

    /// Current working directory at the time of the event.
    pub cwd: Option<String>,

    /// Tool name, if the event is tool-related.
    pub tool_name: Option<String>,

    /// Tool input/arguments, if the event is tool-related.
    pub tool_input: Option<serde_json::Value>,

    /// Tool output/response, if the event is a post-tool event.
    pub tool_response: Option<serde_json::Value>,

    /// Error message, if the event represents a failure.
    pub error: Option<String>,

    /// The user's prompt text, for prompt-related events.
    pub prompt: Option<String>,

    /// Agent/subagent type or identifier.
    pub agent_type: Option<String>,

    /// Notification type string.
    pub notification_type: Option<String>,

    /// Notification message text.
    pub notification_message: Option<String>,

    /// Provider-specific fields that don't map to common fields.
    pub extra: HashMap<String, serde_json::Value>,

    /// Snapshot of the host and repository environment.
    ///
    /// Populated once at session start via `sniff_lib` and reused
    /// for all events in the session. Provides OS, hardware, git,
    /// and project context for action interpolation and conditional
    /// logic.
    pub env: EnvironmentContext,
}

/// Supported agentic CLI providers.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Provider {
    Claude,
    Codex,
    Gemini,
    OpenCode,
    RooCode,
}
```

---

## Environment Context (powered by Sniff)

Every event carries an `EnvironmentContext` — a lightweight snapshot of the host machine and repository state. This context is detected once at session start using `sniff_lib` and cached for the lifetime of the session. It enables actions to make decisions based on the environment (e.g., different TTS messages on macOS vs Linux, branch-aware logging, monorepo-aware tool filtering).

### Design Principles

1. **Detect once, reuse always.** Sniff detection involves filesystem traversal and (optionally) network calls. Running it per-event would be unacceptable. The context is captured at `SessionStart` and attached to every subsequent `EventMeta`.

2. **Curated subset, not the full `SniffResult`.** The full Sniff result includes GPU capabilities, SIMD instruction sets, network interfaces, and file diffs — none of which are useful for event handling. `EnvironmentContext` cherry-picks the fields that actually inform action decisions.

3. **Flat-enough for interpolation.** Fields are accessible via `{env.branch}`, `{env.os}`, etc. in templates. Nested structures are kept to one level where possible.

### Struct Definition

```rust
use std::path::PathBuf;

/// Host and repository environment snapshot.
///
/// Detected once at session start via `sniff_lib::detect_with_config`
/// and cached for the session lifetime. Attached to every `EventMeta`.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EnvironmentContext {
    /// Operating system information.
    pub os: OsContext,

    /// Hardware summary.
    pub hardware: HardwareContext,

    /// Git repository state (if cwd is inside a repo).
    pub git: Option<GitContext>,

    /// Project/repository structure.
    pub repo: Option<RepoContext>,

    /// Primary programming language detected in the project.
    pub primary_language: Option<String>,
}

/// Operating system identification.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OsContext {
    /// OS family: "macos", "linux", "windows", etc.
    pub os_type: String,

    /// Display name (e.g., "macOS", "Ubuntu", "Windows 11").
    pub name: String,

    /// Short version string (e.g., "15.3", "24.04").
    pub version: String,

    /// Kernel version string.
    pub kernel: String,

    /// Machine hostname.
    pub hostname: String,

    /// Linux distribution family, if applicable.
    /// Values: "debian", "redhat", "arch", "alpine", "nixos", "gentoo", or `None`.
    pub linux_family: Option<String>,

    /// Detected system package managers (e.g., ["brew", "port"] on macOS).
    #[serde(default)]
    pub package_managers: Vec<String>,
}

/// Hardware summary relevant to event handling.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HardwareContext {
    /// CPU architecture: "x86_64", "aarch64", etc.
    pub arch: String,

    /// CPU brand string (e.g., "Apple M4 Max").
    pub cpu: String,

    /// Logical core count.
    pub cores: usize,

    /// Total system memory in bytes.
    pub memory_bytes: u64,

    /// Available system memory in bytes at detection time.
    pub memory_available_bytes: u64,
}

/// Git repository state.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GitContext {
    /// Absolute path to the repository root.
    pub repo_root: PathBuf,

    /// Current branch name, or `None` for detached HEAD.
    pub branch: Option<String>,

    /// Whether the working tree has uncommitted changes.
    pub is_dirty: bool,

    /// Count of staged files.
    pub staged_count: usize,

    /// Count of modified but unstaged files.
    pub unstaged_count: usize,

    /// Count of untracked files.
    pub untracked_count: usize,

    /// SHA of the most recent commit.
    pub head_sha: Option<String>,

    /// Message of the most recent commit.
    pub head_message: Option<String>,

    /// Git user.name from config.
    pub user_name: Option<String>,

    /// Git user.email from config.
    pub user_email: Option<String>,

    /// Primary remote name (usually "origin").
    pub remote_name: Option<String>,

    /// Primary remote URL.
    pub remote_url: Option<String>,

    /// Hosting provider for the primary remote.
    /// Values: "github", "gitlab", "bitbucket", "azure_devops", etc.
    pub hosting_provider: Option<String>,
}

/// Project and monorepo structure.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RepoContext {
    /// Whether this project is a monorepo.
    pub is_monorepo: bool,

    /// Monorepo tool if detected.
    /// Values: "cargo_workspace", "npm_workspaces", "pnpm_workspaces",
    /// "yarn_workspaces", "nx", "turborepo", "lerna".
    pub monorepo_tool: Option<String>,

    /// Absolute path to the project root.
    pub root: PathBuf,

    /// Package names within the monorepo (empty for single-package repos).
    #[serde(default)]
    pub packages: Vec<String>,
}
```

### Mapping from `SniffResult`

The `EnvironmentContext` is built from `sniff_lib::SniffResult` with the following field mapping:

| `EnvironmentContext` field | `SniffResult` source | Notes |
|---|---|---|
| `os.os_type` | `result.os.os_type` | Serialized to lowercase string |
| `os.name` | `result.os.name` | Direct |
| `os.version` | `result.os.version` | Direct |
| `os.kernel` | `result.os.kernel` | Direct |
| `os.hostname` | `result.os.hostname` | Direct |
| `os.linux_family` | `result.os.linux_distro.family` | Only on Linux |
| `os.package_managers` | `result.os.system_package_managers` | Flattened to name strings |
| `hardware.arch` | `result.hardware.cpu.arch` | Direct |
| `hardware.cpu` | `result.hardware.cpu.brand` | Direct |
| `hardware.cores` | `result.hardware.cpu.logical_cores` | Direct |
| `hardware.memory_bytes` | `result.hardware.memory.total_bytes` | Direct |
| `hardware.memory_available_bytes` | `result.hardware.memory.available_bytes` | Direct |
| `git.repo_root` | `result.filesystem.git.repo_root` | Direct |
| `git.branch` | `result.filesystem.git.current_branch` | Direct |
| `git.is_dirty` | `result.filesystem.git.status.is_dirty` | Direct |
| `git.staged_count` | `result.filesystem.git.status.staged_count` | Direct |
| `git.unstaged_count` | `result.filesystem.git.status.unstaged_count` | Direct |
| `git.untracked_count` | `result.filesystem.git.status.untracked_count` | Direct |
| `git.head_sha` | `result.filesystem.git.recent[0].sha` | First entry |
| `git.head_message` | `result.filesystem.git.recent[0].message` | First entry |
| `git.user_name` | `result.filesystem.git.config.user_name` | Direct |
| `git.user_email` | `result.filesystem.git.config.user_email` | Direct |
| `git.remote_name` | `result.filesystem.git.remotes[0].name` | First remote |
| `git.remote_url` | `result.filesystem.git.remotes[0].url` | First remote |
| `git.hosting_provider` | `result.filesystem.git.remotes[0].provider` | Serialized to string |
| `repo.is_monorepo` | `result.filesystem.repo.is_monorepo` | Direct |
| `repo.monorepo_tool` | `result.filesystem.repo.monorepo_tool` | Serialized to string |
| `repo.root` | `result.filesystem.repo.root` | Direct |
| `repo.packages` | `result.filesystem.repo.packages[*].name` | Names only |
| `primary_language` | `result.filesystem.languages.primary` | Direct |

### Fields intentionally excluded

| Excluded field | Reason |
|---|---|
| GPU info | Not relevant to code agent event handling |
| SIMD capabilities | Not relevant to event action decisions |
| Network interfaces / IPs | Privacy-sensitive, not useful for actions |
| Storage devices | Not relevant to event handling |
| File diffs (`DirtyFile.diff`) | Too large for event metadata; use git tools instead |
| Full dependency lists | Too verbose; packages list provides enough monorepo context |
| EditorConfig / formatting | Not relevant to event action decisions |
| CPU usage sampling | Requires 200ms delay; not worth the latency |
| All recent commits | Only HEAD is useful for event context |
| All branches | Only current branch is useful for event context |
| Remote tracking (ahead/behind) | Requires `--deep` (network); not worth latency at startup |

### Sniff Configuration

The environment detection uses a tuned `SniffConfig`:

```rust
use sniff_lib::SniffConfig;

fn detect_environment(cwd: &Path) -> EnvironmentContext {
    let config = SniffConfig::new()
        .base_dir(cwd.to_path_buf())
        .deep(false)          // No network calls — fast startup
        .commit_count(1)      // Only need HEAD commit
        .skip_network();      // Skip network interfaces entirely

    let result = sniff_lib::detect_with_config(config)
        .unwrap_or_default();

    EnvironmentContext::from(result)
}
```

Key choices:
- **`deep(false)`**: Avoids network calls for remote branch status and latest package versions. Keeps startup fast.
- **`commit_count(1)`**: Only the HEAD commit is needed for context.
- **`skip_network()`**: Network interface data is not useful for event handling and adds detection time.
- OS, hardware, and filesystem sections are all enabled.

### Template Interpolation

Environment fields are available in `Speak` messages and `Report` templates via the `{env.*}` namespace:

| Placeholder | Source |
|---|---|
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

### Example: Environment-Aware Actions

```json
{
  "events": {
    "session_start": {
      "actions": [
        {
          "type": "sound_effect",
          "name": "power-up",
          "volume": 0.5
        },
        {
          "type": "speak",
          "message": "Starting on {env.branch} in {env.language} project on {env.os}"
        },
        {
          "type": "log",
          "target": { "type": "local_file", "path": "~/.claudine/events.jsonl" }
        }
      ]
    },
    "turn_complete": {
      "actions": [
        {
          "type": "sound_effect",
          "name": "beep"
        }
      ]
    },
    "tool_error": {
      "actions": [
        {
          "type": "sound_effect",
          "name": "error"
        },
        {
          "type": "report",
          "handler": {
            "format": "compact",
            "template": "[ERR] {tool_name} failed on {env.os} ({env.arch}): {error}",
            "include_metadata": true
          }
        }
      ]
    }
  }
}
```

---

## Configuration Schema (`~/.hooker`)

The configuration file is JSON. It defines event bindings with optional per-provider overrides.

### Schema

```rust
/// Root configuration loaded from `~/.hooker`.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HookerConfig {
    /// Schema version for forward compatibility.
    pub version: String,

    /// Global settings.
    #[serde(default)]
    pub settings: GlobalSettings,

    /// Event bindings: map from event name to its configuration.
    pub events: HashMap<AgenticEvent, EventBinding>,
}

/// Global settings that apply to all event bindings.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct GlobalSettings {
    /// Default log target used when an event's `Log` action
    /// doesn't specify its own target.
    pub default_log_target: Option<LogTarget>,

    /// Default TTS voice/engine settings for `Speak` actions.
    /// Passed through to biscuit-speaks.
    pub tts: Option<TtsSettings>,
}

/// TTS configuration forwarded to biscuit-speaks.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TtsSettings {
    /// Preferred TTS provider (e.g., "say", "espeak", "elevenlabs").
    pub provider: Option<String>,

    /// Voice name or identifier.
    pub voice: Option<String>,

    /// Speech rate multiplier (1.0 = normal).
    pub rate: Option<f32>,
}

/// Configuration for a single event binding.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EventBinding {
    /// Whether this binding is active. Defaults to `true`.
    #[serde(default = "default_true")]
    pub enabled: bool,

    /// Actions to execute when this event fires (all providers).
    #[serde(default)]
    pub actions: Vec<EventAction>,

    /// Optional filter: only fire for events matching this regex
    /// against the tool name, notification type, or session source.
    pub matcher: Option<String>,

    /// Per-provider overrides. When a provider key is present,
    /// its actions REPLACE the top-level actions for that provider.
    /// Use this to handle provider-specific variations.
    #[serde(default)]
    pub overrides: HashMap<Provider, ProviderOverride>,
}

/// Provider-specific override for an event binding.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProviderOverride {
    /// Whether this override is active. Defaults to `true`.
    #[serde(default = "default_true")]
    pub enabled: bool,

    /// Actions that REPLACE the parent binding's actions
    /// for this specific provider.
    pub actions: Vec<EventAction>,

    /// Provider-specific matcher override.
    pub matcher: Option<String>,
}

fn default_true() -> bool {
    true
}
```

### Example Configuration

```json
{
  "version": "1.0",
  "settings": {
    "default_log_target": {
      "type": "local_file",
      "path": "~/.claudine/events.jsonl"
    },
    "tts": {
      "provider": "say",
      "voice": "Samantha",
      "rate": 1.2
    }
  },
  "events": {
    "session_start": {
      "enabled": true,
      "actions": [
        {
          "type": "sound_effect",
          "name": "power-up"
        },
        {
          "type": "speak",
          "message": "Session started"
        },
        {
          "type": "log",
          "target": {
            "type": "local_file",
            "path": "~/.claudine/events.jsonl"
          }
        }
      ]
    },
    "turn_complete": {
      "enabled": true,
      "actions": [
        {
          "type": "sound_effect",
          "name": "success",
          "volume": 0.6
        }
      ],
      "overrides": {
        "claude": {
          "enabled": true,
          "actions": [
            {
              "type": "speak",
              "message": "Claude has finished"
            }
          ]
        },
        "codex": {
          "enabled": true,
          "actions": [
            {
              "type": "speak",
              "message": "Codex turn done"
            }
          ]
        }
      }
    },
    "before_tool": {
      "enabled": true,
      "matcher": "Bash|bash",
      "actions": [
        {
          "type": "report",
          "handler": {
            "format": "compact",
            "template": "[TOOL] {tool_name}: executing",
            "include_metadata": false
          }
        }
      ]
    },
    "tool_error": {
      "enabled": true,
      "actions": [
        {
          "type": "sound_effect",
          "name": "sad-trombone",
          "volume": 0.8
        },
        {
          "type": "speak",
          "message": "Tool {tool_name} failed: {error}"
        },
        {
          "type": "log",
          "target": {
            "type": "server",
            "url": "https://my-logging-server.example.com/events"
          }
        }
      ]
    },
    "notification": {
      "enabled": true,
      "matcher": "permission_prompt|ToolPermission",
      "actions": [
        {
          "type": "sound_effect",
          "name": "notification"
        },
        {
          "type": "speak",
          "message": "Attention needed"
        }
      ]
    }
  }
}
```

---

## Provider Adapter Architecture

Each supported provider has an adapter that:

1. **Receives** native events from the provider's hook mechanism
2. **Maps** them to an `AgenticEvent` variant
3. **Extracts** metadata into `EventMeta`
4. **Dispatches** to the event handler which executes configured actions

```
┌─────────────────────────────────────────────────┐
│                   ~/.hooker                      │
│              (HookerConfig JSON)                 │
└──────────────────────┬──────────────────────────┘
                       │ loaded at startup
                       ▼
┌─────────────────────────────────────────────────┐
│              EventDispatcher                     │
│  ┌─────────────┐  ┌──────────┐  ┌────────────┐ │
│  │ ActionRunner │  │ Matcher  │  │  Override   │ │
│  │ (speak/log/ │  │  Engine  │  │  Resolver   │ │
│  │ report/sfx) │  │          │  │             │ │
│  └─────────────┘  └──────────┘  └────────────┘ │
└──────────────────────┬──────────────────────────┘
                       │ AgenticEvent + EventMeta
                       │    (includes EnvironmentContext)
                       │
     ┌─────────────────┼──────────────────┐
     │                 │                  │
     ▼                 ▼                  ▼
┌──────────────┐ ┌──────────────┐ ┌──────────────┐
│ ClaudeAdapter│ │ GeminiAdapter│ │ CodexAdapter │  ...
│              │ │              │ │              │
│ stdin JSON → │ │ stdin JSON → │ │ JSONL →      │
│ AgenticEvent │ │ AgenticEvent │ │ AgenticEvent │
└──────────────┘ └──────────────┘ └──────────────┘

┌─────────────────────────────────────────────────┐
│           sniff_lib (detect once)                │
│  ┌──────┐  ┌──────────┐  ┌─────┐  ┌──────────┐│
│  │  OS  │  │ Hardware  │  │ Git │  │   Repo   ││
│  └──────┘  └──────────┘  └─────┘  └──────────┘│
│         → EnvironmentContext (cached)           │
└─────────────────────────────────────────────────┘
```

### Adapter trait

```rust
/// Trait implemented by each provider adapter.
///
/// Adapters are responsible for parsing the provider's native
/// event format and mapping it to the shared model.
pub trait ProviderAdapter {
    /// Which provider this adapter handles.
    fn provider(&self) -> Provider;

    /// Parse a native event payload into the shared model.
    ///
    /// Returns `None` if the native event has no shared equivalent
    /// (e.g., provider-specific internal events).
    fn parse_event(&self, raw: &serde_json::Value) -> Option<(AgenticEvent, EventMeta)>;
}
```

### Adapter mapping details

#### Claude Code

Claude hooks receive JSON on stdin and return JSON on stdout. The adapter runs as the hook script itself (Claudine is invoked as the command).

| Native event | `hook_event_name` field | Maps to |
|---|---|---|
| `SessionStart` | `"SessionStart"` | `AgenticEvent::SessionStart` |
| `SessionEnd` | `"SessionEnd"` | `AgenticEvent::SessionEnd` |
| `UserPromptSubmit` | `"UserPromptSubmit"` | `AgenticEvent::BeforePrompt` |
| `PreToolUse` | `"PreToolUse"` | `AgenticEvent::BeforeTool` |
| `PostToolUse` | `"PostToolUse"` | `AgenticEvent::AfterTool` |
| `PostToolUseFailure` | `"PostToolUseFailure"` | `AgenticEvent::ToolError` |
| `PermissionRequest` | `"PermissionRequest"` | `AgenticEvent::PermissionRequest` |
| `Stop` | `"Stop"` | `AgenticEvent::TurnComplete` |
| `SubagentStart` | `"SubagentStart"` | `AgenticEvent::SubagentStart` |
| `SubagentStop` | `"SubagentStop"` | `AgenticEvent::SubagentStop` |
| `PreCompact` | `"PreCompact"` | `AgenticEvent::BeforeCompact` |
| `Notification` | `"Notification"` | `AgenticEvent::Notification` |

**Configuration**: Claude hooks point to the Claudine binary:

```json
{
  "hooks": {
    "SessionStart": [{ "hooks": [{ "type": "command", "command": "claudine handle session_start" }] }],
    "PreToolUse": [{ "hooks": [{ "type": "command", "command": "claudine handle before_tool" }] }]
  }
}
```

#### Gemini CLI

Gemini hooks also use stdin/stdout JSON. Same invocation pattern as Claude.

| Native event | `hook_event_name` field | Maps to |
|---|---|---|
| `SessionStart` | `"SessionStart"` | `AgenticEvent::SessionStart` |
| `SessionEnd` | `"SessionEnd"` | `AgenticEvent::SessionEnd` |
| `BeforeAgent` | `"BeforeAgent"` | `AgenticEvent::BeforePrompt` |
| `BeforeTool` | `"BeforeTool"` | `AgenticEvent::BeforeTool` |
| `AfterTool` | `"AfterTool"` | `AgenticEvent::AfterTool` |
| `AfterAgent` | `"AfterAgent"` | `AgenticEvent::TurnComplete` |
| `BeforeModel` | `"BeforeModel"` | `AgenticEvent::BeforeModel` |
| `AfterModel` | `"AfterModel"` | `AgenticEvent::AfterModel` |
| `PreCompress` | `"PreCompress"` | `AgenticEvent::BeforeCompact` |
| `Notification` | `"Notification"` | `AgenticEvent::Notification` |

#### Codex CLI

Codex uses a JSONL event stream (`codex exec --json`). The adapter reads this stream and maps events.

| Native event | `type` field | Maps to |
|---|---|---|
| `thread.started` | `"thread.started"` | `AgenticEvent::SessionStart` |
| `turn.completed` | `"turn.completed"` | `AgenticEvent::TurnComplete` |
| `turn.failed` | `"turn.failed"` | `AgenticEvent::TurnError` |
| `item.completed` (command_execution) | `"item.completed"` | `AgenticEvent::AfterTool` |
| `error` | `"error"` | `AgenticEvent::TurnError` |

Additionally, the `notify` hook (`agent-turn-complete`) maps to `AgenticEvent::TurnComplete`.

#### OpenCode

OpenCode uses a TypeScript plugin system. The adapter is implemented as an OpenCode plugin that bridges to Claudine via subprocess or IPC.

| Native hook | Maps to |
|---|---|
| `event` → `session.created` | `AgenticEvent::SessionStart` |
| `event` → `session.deleted` | `AgenticEvent::SessionEnd` |
| `event` → `session.idle` | `AgenticEvent::TurnComplete` |
| `event` → `session.error` | `AgenticEvent::TurnError` |
| `event` → `session.compacted` | `AgenticEvent::BeforeCompact` |
| `event` → `permission.asked` | `AgenticEvent::PermissionRequest` |
| `chat.message` hook | `AgenticEvent::BeforePrompt` |
| `tool.execute.before` hook | `AgenticEvent::BeforeTool` |
| `tool.execute.after` hook | `AgenticEvent::AfterTool` |
| `chat.params` hook | `AgenticEvent::BeforeModel` |

#### Roo Code

Roo Code uses Node EventEmitter-based events. The adapter bridges via `--output-format stream-json` for CLI automation.

| Native event | Maps to |
|---|---|
| `taskCompleted` | `AgenticEvent::TurnComplete` |
| `taskToolFailed` | `AgenticEvent::ToolError` |
| `taskSpawned` | `AgenticEvent::SubagentStart` |
| `taskDelegationCompleted` | `AgenticEvent::SubagentStop` |
| `error` | `AgenticEvent::TurnError` |
| `tool_result` (JSON stream) | `AgenticEvent::AfterTool` |

---

## Event Dispatch Flow

When Claudine receives an event from any provider:

```
1. Provider adapter parses raw input → (AgenticEvent, EventMeta)
2. Look up AgenticEvent in HookerConfig.events
3. If no binding exists or binding.enabled == false → exit 0
4. Check binding.matcher against relevant metadata field
   - Tool events: match against tool_name
   - Notification: match against notification_type
   - SessionStart: match against source (from extra)
5. Check for provider-specific override:
   - If overrides[provider] exists and is enabled → use override.actions
   - Otherwise → use binding.actions
6. Execute each EventAction in order:
   a. Speak → invoke biscuit-speaks with interpolated message
   b. Log → append JSONL to file or POST to server
   c. Report → write formatted line to stdout
   d. SoundEffect → play embedded effect via playa (non-blocking)
7. Return appropriate exit code and JSON to the provider
```

### Template Interpolation

`Speak` messages and `Report` templates support `{placeholder}` interpolation from `EventMeta` fields:

| Placeholder | Source field |
|---|---|
| `{provider}` | `meta.provider` |
| `{event}` | `meta.event` (serialized name) |
| `{session_id}` | `meta.session_id` |
| `{tool_name}` | `meta.tool_name` |
| `{error}` | `meta.error` |
| `{prompt}` | `meta.prompt` |
| `{agent_type}` | `meta.agent_type` |
| `{notification_type}` | `meta.notification_type` |
| `{cwd}` | `meta.cwd` |
| `{timestamp}` | `meta.timestamp` (ISO 8601) |
| `{env.os}` | `meta.env.os.name` |
| `{env.os_type}` | `meta.env.os.os_type` |
| `{env.hostname}` | `meta.env.os.hostname` |
| `{env.arch}` | `meta.env.hardware.arch` |
| `{env.cpu}` | `meta.env.hardware.cpu` |
| `{env.cores}` | `meta.env.hardware.cores` |
| `{env.branch}` | `meta.env.git.branch` |
| `{env.is_dirty}` | `meta.env.git.is_dirty` |
| `{env.head_sha}` | `meta.env.git.head_sha` |
| `{env.remote}` | `meta.env.git.remote_name` |
| `{env.hosting}` | `meta.env.git.hosting_provider` |
| `{env.is_monorepo}` | `meta.env.repo.is_monorepo` |
| `{env.monorepo_tool}` | `meta.env.repo.monorepo_tool` |
| `{env.language}` | `meta.env.primary_language` |

Unknown placeholders are left as-is (not an error).

---

## JSONL Log Format

When `EventAction::Log` writes to a local file, each line is a complete JSON object containing both event metadata and environment context:

```json
{"timestamp":"2026-02-04T15:30:00Z","provider":"claude","event":"before_tool","session_id":"abc123","tool_name":"Bash","tool_input":{"command":"npm test"},"env":{"os":{"os_type":"macos","name":"macOS","version":"15.3"},"hardware":{"arch":"aarch64","cpu":"Apple M4 Max","cores":16},"git":{"branch":"feat/hooks","is_dirty":true,"staged_count":2},"repo":{"is_monorepo":true,"monorepo_tool":"cargo_workspace"},"primary_language":"Rust"},"extra":{}}
```

When posting to a server, the same JSON object is sent as the request body with `Content-Type: application/json`. The environment context makes log entries self-describing — you can filter and aggregate by OS, architecture, branch, project language, or monorepo package without needing external correlation.

---

## Capability Matrix

Not all providers support all interaction modes. This matrix shows what Claudine can do per provider:

| Capability | Claude | Codex | Gemini | OpenCode | Roo |
|---|---|---|---|---|---|
| Observe events | Yes | Yes | Yes | Yes | Yes |
| Block actions (exit 2) | Yes | No | Yes | Yes (throw) | No |
| Modify tool input | Yes | No | Yes | Yes (mutate) | No |
| Inject context | Yes | No | Yes | Yes | No |
| Async/non-blocking | Yes | N/A (stream) | No | N/A (plugin) | N/A (stream) |
| Matcher filtering | Yes (regex) | N/A | N/A (in-hook) | N/A (in-hook) | N/A |

The shared model focuses on the **observe-and-react** pattern (Speak, SoundEffect, Log, Report) which works universally. Blocking and modification are provider-specific capabilities that may be exposed in future versions.

---

## Design Decisions

### Why a flat enum instead of a hierarchy?

A nested enum (e.g., `Tool::Before`, `Tool::After`) would be more type-safe but makes serialization awkward and configuration verbose. The flat enum serializes to simple strings (`"before_tool"`) that are easy to read and write in JSON config files.

### Why `serde(rename_all = "snake_case")`?

The JSON configuration uses snake_case keys (`before_tool`, `session_start`) because:

1. It's the most common JSON convention
2. It's readable in config files
3. It avoids quoting issues with kebab-case in JSON

### Why overrides replace rather than merge?

When a provider override is present, its actions fully replace the parent's actions. This is simpler to reason about than merge semantics (which action wins? what order?) and matches how CSS cascading works — the most specific rule wins entirely.

### Why `Option<ReportHandler>` instead of always requiring a handler?

Most users will want the default reporting behavior. Making the handler optional means `{"type": "report"}` is sufficient for the common case, while power users can customize with `{"type": "report", "handler": {...}}`.

---

## Future Considerations

1. **Conditional actions**: Add `when` clauses to actions (e.g., only speak on error, only log when tool_name matches a pattern). Currently, the `matcher` field on `EventBinding` serves this role at the event level.

2. **Action chaining**: Allow actions to reference the output of previous actions (e.g., log what was spoken).

3. **Provider-specific blocking**: Expose the ability to block/deny tool calls through the shared model for providers that support it (Claude, Gemini, OpenCode).

4. **Custom events**: Allow users to define synthetic events that combine multiple provider events (e.g., "any error" = `ToolError | TurnError`).

5. **Hot reload**: Watch `~/.hooker` for changes and reload configuration without restarting the agent session (where the provider supports it).
