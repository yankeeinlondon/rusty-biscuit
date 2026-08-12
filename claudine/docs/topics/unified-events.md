# Claudine Unified Events Design

This document defines the unified event model for Claudine: a system that normalizes lifecycle events from every compiled agentic CLI provider into 16 canonical hooks, then dispatches configurable actions (TTS, sound effects, logging, shell commands) when those hooks fire.

## Design Target and Migration Scope

This document is the **target architecture** for the unified-events refactor. It intentionally prioritizes design correctness over the current implementation state.

When there is a mismatch between this design and the existing code:

1. The design is authoritative.
2. The mismatch is treated as migration work.
3. Compatibility behavior (when needed) is called out explicitly.

The detailed research below documents the original seven-provider design set.
The generated provider catalog and the event-support matrix in the Claudine
architecture skill are authoritative for the current ten-provider roster,
which also includes Kilo, Pi, and Antigravity.

## Original Design CLIs

| # | CLI | Vendor | Events | Design Reference |
|---|-----|--------|:------:|------------------|
| 1 | Claude Code | Anthropic | 14 | [designs/claude-code.md](designs/claude-code.md) |
| 2 | Codex CLI | OpenAI | 10 | [designs/codex.md](designs/codex.md) |
| 3 | Gemini CLI | Google | 11 | [designs/gemini-cli.md](designs/gemini-cli.md) |
| 4 | Goose | Block | 7 | [designs/goose.md](designs/goose.md) |
| 5 | Kimi Code | Moonshot AI | 15 | [designs/kimi-code.md](designs/kimi-code.md) |
| 6 | OpenCode | OpenCode AI | 16 | [designs/opencode.md](designs/opencode.md) |
| 7 | Qwen CLI | Alibaba | 7 | [designs/qwen-cli.md](designs/qwen-cli.md) |

---

## 1. Individual CLI Summaries

### 1.1 Claude Code

**14 events** | Shell commands (JSON stdin/stdout, exit codes) | **7 blocking**

Claude Code is the baseline design. It provides the richest hook system with three handler types (command, prompt, agent), three-level permission decisions, and structured JSON payloads on stdin.

| Native Event | Blocking | AgenticEvent |
|---|:---:|---|
| SessionStart | - | `session_start` |
| SessionEnd | - | `session_end` |
| UserPromptSubmit | Y | `before_prompt` |
| PreToolUse | Y | `before_tool` |
| PostToolUse | Y | `after_tool` |
| PostToolUseFailure | - | `tool_error` |
| PermissionRequest | Y | `permission_request` |
| Notification | - | `notification` |
| SubagentStart | - | `subagent_start` |
| SubagentStop | Y | `subagent_stop` |
| Stop | Y | `turn_complete` |
| TeammateIdle | Y | `turn_complete` (lossy) |
| TaskCompleted | Y | `turn_complete` (lossy) |
| PreCompact | - | `before_compact` |

Unique: `CLAUDE_ENV_FILE` for env persistence, `stop_hook_active` infinite-loop guard, three handler types per hook (command, prompt, agent), `PostToolUse` decision is advisory (cannot undo tool execution).

### 1.2 Codex CLI

**10 events** | CLI argument + internal callback + JSONL stream | **0 blocking**

Only 1 user-configurable hook (`notify` in `config.toml`). The JSONL stream provides observational item lifecycle events. No return channel on any surface.

| Native Event | Blocking | AgenticEvent |
|---|:---:|---|
| AfterAgent | - | `turn_complete` |
| AfterToolUse | - | `after_tool` |
| ThreadStarted | - | `session_start` |
| TurnStarted | - | `before_prompt` |
| TurnCompleted | - | `turn_complete` |
| TurnFailed | - | `turn_error` |
| ItemStarted | - | `before_tool` |
| ItemUpdated | - | `after_model` |
| ItemCompleted | - | `after_tool` |
| Error | - | `turn_error` |

Unique: Token usage stats in TurnCompleted, 7 item types (agent_message, reasoning, command_execution, file_change, mcp_tool_call, web_search, plan_update), payload delivered as CLI argument (not stdin).

### 1.3 Gemini CLI

**11 events** | Shell commands (JSON stdin/stdout, exit codes) | **6 blocking**

Most similar to Claude Code in hook architecture but with model-level hooks (BeforeModel/AfterModel) and automatic retry semantics on AfterAgent. Exit code 2 has different per-event effects.

| Native Event | Blocking | AgenticEvent |
|---|:---:|---|
| SessionStart | - | `session_start` |
| SessionEnd | - | `session_end` |
| BeforeAgent | Y | `before_prompt` |
| AfterAgent | Y | `turn_complete` |
| BeforeModel | Y | `before_model` |
| AfterModel | Y | `after_model` |
| BeforeToolSelection | - | `before_model` (lossy) |
| BeforeTool | Y | `before_tool` |
| AfterTool | Y | `after_tool` |
| PreCompress | - | `before_compact` |
| Notification | - | `notification` |

Unique: Model override / synthetic response injection via BeforeModel, regex tool matchers, multi-hook aggregation strategies (OrDecision, FieldReplacement, Union, SimpleMerge), retry semantics on AfterAgent deny.

### 1.4 Goose

**7 events** | Status hook (fire-and-forget) + stream-json | **0 blocking**

Purely observe-only architecture. No event can influence agent behavior. Composite Message event bundles text, tool requests, and tool responses.

| Native Event | Blocking | AgenticEvent |
|---|:---:|---|
| StatusWaiting | - | `turn_complete` |
| StatusThinking | - | `before_model` |
| Message | - | `after_model` |
| Notification | - | `notification` |
| ModelChange | - | `notification` (lossy) |
| Error | - | `turn_error` |
| Complete | - | `session_end` |

Unique: GooseMode config (auto/approve/chat/smart_approve) replaces hook-based permissions, composite message payloads with 5 content types, MCP extension notifications.

### 1.5 Kimi Code

**15 events** | JSON-RPC 2.0 bidirectional protocol (Wire mode) | **2 blocking**

Only CLI using a proper RPC protocol. Notifications are fire-and-forget; requests block until JSON-RPC response. Supports recursive SubagentEvent nesting and streaming content parts.

| Native Event | Blocking | AgenticEvent |
|---|:---:|---|
| TurnBegin | - | `before_prompt` |
| TurnEnd | - | `turn_complete` |
| StepBegin | - | `notification` (lossy step-orchestration marker) |
| StepInterrupted | - | `turn_error` |
| CompactionBegin | - | `before_compact` |
| CompactionEnd | - | `notification` (lossy compaction-end marker) |
| StatusUpdate | - | `notification` |
| ContentPart | - | `after_model` |
| ToolCall | - | `before_tool` |
| ToolCallPart | - | `before_tool` |
| ToolResult | - | `after_tool` |
| ApprovalResponse | - | `permission_request` |
| SubagentEvent | - | `subagent_start` |
| ApprovalRequest | Y | `permission_request` |
| ToolCallRequest | Y | `before_tool` |

Unique: Wire protocol handshake with external tool registration, recursive subagent events, DisplayBlock rich rendering (brief, diff, todo, shell), streaming argument fragments, 4 custom JSON-RPC error codes.

### 1.6 OpenCode

**16 events** | In-process JS/TS plugin functions | **2 blocking**

Plugin-based mutation pattern (read-only `input` + mutable `output`). Catch-all `event` bus receives 40+ system bus types. 4 experimental hooks for prompt/message/compaction transformation.

| Native Event | Blocking | AgenticEvent |
|---|:---:|---|
| Event | - | depends on bus type |
| ToolExecuteBefore | Y | `before_tool` |
| ToolExecuteAfter | - | `after_tool` |
| ToolDefinition | - | -- |
| ShellEnv | - | -- |
| ChatMessage | - | `before_prompt` |
| ChatParams | - | `before_model` |
| ChatHeaders | - | `before_model` |
| PermissionAsk | Y | `permission_request` |
| CommandExecuteBefore | - | -- |
| Config | - | -- |
| Auth | - | -- |
| ExperimentalChatSystemTransform | - | `before_model` |
| ExperimentalChatMessagesTransform | - | `before_model` |
| ExperimentalSessionCompacting | - | `before_compact` |
| ExperimentalTextComplete | - | `after_model` |

Unique: 40+ bus event types via catch-all subscriber, input/output mutation pattern (not stdin/stdout), 6 hooks with no unified mapping (ToolDefinition, ShellEnv, CommandExecuteBefore, Config, Auth, Event), auth registration hooks, shell env injection.

### 1.7 Qwen CLI

**7 events** | SDK callback + internal hooks + stream-json | **1 blocking**

Most fragmented surface model with three independent integration surfaces. Only CanUseTool supports bidirectional communication via SDK callback with a 60-second auto-deny timeout.

| Native Event | Blocking | AgenticEvent |
|---|:---:|---|
| CanUseTool | Y | `permission_request` |
| SubagentPreToolUse | - | `before_tool` |
| SubagentPostToolUse | - | `after_tool` |
| SubagentStop | - | `subagent_stop` |
| StreamSessionStart | - | `session_start` |
| StreamAssistantMessage | - | `after_model` |
| StreamResult | - | `session_end` |

Unique: Permission priority chain (excludeTools > plan mode > yolo mode > allowedTools > callback > default deny), fire-and-forget SubagentPreToolUse (not even awaited), 60-second callback timeout with auto-deny.

---

## 2. Cross-CLI Comparison

### 2.1 AgenticEvent Coverage Matrix

| AgenticEvent | Claude | Codex | Gemini | Goose | Kimi | OpenCode | Qwen |
|---|:---:|:---:|:---:|:---:|:---:|:---:|:---:|
| `session_start` | H | S | H | - | - | H | S |
| `session_end` | H | - | H | S | - | H | S |
| `before_prompt` | H | S | H | - | W | H | - |
| `before_tool` | H | S | H | - | W | H | S |
| `after_tool` | H | S | H | - | W | H | S |
| `tool_error` | H | - | - | - | W | - | - |
| `permission_request` | H | - | - | - | W | H | C |
| `notification` | H | S | H | S | W | H | S |
| `subagent_start` | H | - | - | S | W | - | - |
| `subagent_stop` | H | - | - | S | W | - | S |
| `turn_complete` | H | H | H | S | W | H | S |
| `turn_error` | - | S | - | S | W | H | S |
| `before_model` | - | - | H | S | - | H | - |
| `after_model` | - | S | H | S | W | H | S |
| `before_compact` | H | - | H | - | W | H | - |
| `human_in_the_loop` | - | - | - | - | W | H | - |

**Legend:** H = Hook (config-file), S = Stream/NonHook, W = Wire/RPC, C = SDK Callback, - = Not supported

### 2.2 Blocking Capability

| CLI | Blocking Events | Total Events | Mechanism |
|---|---|---|---|
| Claude Code | 7 | 14 | Exit code 2 + JSON decision fields |
| Gemini CLI | 6 | 11 | Exit code 2 (per-event effects) + JSON |
| OpenCode | 2 | 16 | Throw exception / mutate status |
| Kimi Code | 2 | 15 | JSON-RPC response |
| Qwen CLI | 1 | 7 | SDK callback response |
| Codex CLI | 0 | 10 | Fire-and-forget |
| Goose | 0 | 7 | Purely observe-only |

### 2.3 Delivery Mechanisms

| Mechanism | CLIs | Protocol |
|---|---|---|
| Shell command (JSON I/O) | Claude, Gemini | JSON on stdin, JSON on stdout, exit codes |
| JSONL stream | Codex, Goose, Qwen | Newline-delimited JSON output |
| JSON-RPC 2.0 | Kimi | Bidirectional over stdin/stdout |
| In-process plugin | OpenCode | JS/TS function, input/output mutation |
| SDK callback | Qwen | Rust async function |
| Config notify | Codex | Shell command with CLI argument |
| Env var hook | Goose | `GOOSE_STATUS_HOOK` shell command |

---

## 3. Unified Design

This is the core section. It defines the types, traits, and dispatch logic that allow Claudine to operate across every compiled provider identity through a single abstraction layer.

### 3.1 `AgenticEvent` Enum

The 16-variant normalized event enum is the canonical hook set for the refactor target and the hub for all cross-provider mapping.

```rust
use serde::{Deserialize, Serialize};

/// Normalized lifecycle events across supported agentic CLI providers.
///
/// Each variant represents a lifecycle moment that Claudine can observe and
/// react to. Provider adapters convert their native events into this enum
/// before the dispatch pipeline processes them.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
#[non_exhaustive]
pub enum AgenticEvent {
    SessionStart,
    SessionEnd,
    BeforePrompt,
    BeforeTool,
    AfterTool,
    ToolError,
    PermissionRequest,
    HumanInTheLoop,
    TurnComplete,
    TurnError,
    SubagentStart,
    SubagentStop,
    BeforeModel,
    AfterModel,
    BeforeCompact,
    Notification,
}
```

### 3.1.1 `Provider` Enum (Canonical Set)

The provider enum is also part of the canonical design surface.

```rust
use serde::{Deserialize, Serialize};

/// Supported agentic CLI providers.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
#[non_exhaustive]
pub enum Provider {
    Claude,
    Codex,
    Gemini,
    Goose,
    KimiCode,
    OpenCode,
    QwenCode,
}
```

### 3.2 `UnifiedHook` -- Why It Is Not a Separate Enum

The unified hook is **not** a new enum. The existing `AgenticEvent` enum with its 16 variants already represents the complete set of hooks Claudine supports. Adding a separate `UnifiedHook` enum would create a redundant abstraction layer with no additional discriminating power.

Instead, the "unified hook" is the **`ResolvedHook` struct** -- the output of the dispatch pipeline after a native event has been parsed, matched, and resolved into actions:

```rust
/// A resolved hook binding ready for execution.
///
/// This is the output of the dispatch pipeline after:
/// 1. A provider adapter converts a native event to `AgenticEvent`
/// 2. The config loader finds the `EventBinding` for that event
/// 3. The matcher filters based on event metadata
/// 4. The resolver produces the action list
pub struct ResolvedHook {
    /// The normalized event that fired.
    pub event: AgenticEvent,

    /// Normalized metadata extracted from the native payload.
    pub meta: EventMeta,

    /// The provider that originated this event.
    pub provider: Provider,

    /// Actions to execute, in declaration order.
    pub actions: Vec<HookAction>,

    /// Whether this hook's event supports blocking on the originating CLI.
    ///
    /// When `true`, blocking actions (like `Call`) can return responses
    /// that influence the agent's behavior. When `false`, all actions
    /// are effectively fire-and-forget regardless of their type.
    pub can_block: bool,
}
```

The `ResolvedHook` struct captures everything needed for action dispatch:
- **What happened**: `event` (which `AgenticEvent` variant)
- **The details**: `meta` (normalized payload with event fields, context, extras)
- **Who sent it**: `provider` (which CLI triggered it)
- **What to do**: `actions` (ordered list of `HookAction` variants)
- **Can we respond**: `can_block` (whether the originating CLI accepts a return value)

### 3.3 Provider Adapter Trait

Each CLI has an adapter that converts its native event format into the unified representation and converts unified responses back to the native format.

```rust
use serde_json::Value;

/// Converts a provider's native event payload into the unified event model.
///
/// Each supported CLI implements this trait. The adapter is responsible for:
/// 1. Identifying which native event fired (parsing the raw JSON)
/// 2. Mapping it to the appropriate `AgenticEvent` variant
/// 3. Extracting normalized metadata into `EventMeta`
/// 4. Preserving provider-specific fields in `EventMeta::extra`
pub trait ProviderAdapter: Send + Sync {
    /// Parse a raw JSON payload into the unified event model.
    ///
    /// ## Returns
    ///
    /// - `Ok((event, meta))` on successful parsing
    /// - `Err` if the payload cannot be recognized or parsed
    fn parse_event(&self, raw: &Value) -> Result<(AgenticEvent, EventMeta), AdapterError>;

    /// The provider this adapter handles.
    fn provider(&self) -> Provider;

    /// Whether the given `AgenticEvent` can block on this provider.
    ///
    /// Returns `true` only if the originating native event supports a
    /// return channel that can influence agent behavior.
    fn can_block(&self, event: &AgenticEvent) -> bool;

    /// Convert a unified `HookResponse` back to the provider's native
    /// JSON response format.
    ///
    /// Called only for blocking events where `can_block()` returns `true`.
    /// For fire-and-forget events, this method is not called.
    fn format_response(
        &self,
        event: &AgenticEvent,
        response: &HookResponse,
    ) -> Result<Value, AdapterError>;

    /// Return the exit code that should be used for the given response.
    ///
    /// Only relevant for shell-based providers (Claude, Gemini).
    /// Returns `None` for providers that don't use exit codes (Kimi, OpenCode, Qwen).
    fn exit_code(&self, event: &AgenticEvent, response: &HookResponse) -> Option<i32>;
}

/// Stateless adapters are singletons; no heap allocation per lookup.
static CLAUDE_ADAPTER: ClaudeCodeAdapter = ClaudeCodeAdapter;
static CODEX_ADAPTER: CodexAdapter = CodexAdapter;
static GEMINI_ADAPTER: GeminiCliAdapter = GeminiCliAdapter;
static GOOSE_ADAPTER: GooseAdapter = GooseAdapter;
static KIMI_ADAPTER: KimiCodeAdapter = KimiCodeAdapter;
static OPENCODE_ADAPTER: OpenCodeAdapter = OpenCodeAdapter;
static QWEN_ADAPTER: QwenCliAdapter = QwenCliAdapter;

/// Factory function that returns the appropriate adapter for a provider.
pub fn adapter_for(provider: Provider) -> &'static dyn ProviderAdapter {
    match provider {
        Provider::Claude => &CLAUDE_ADAPTER,
        Provider::Codex => &CODEX_ADAPTER,
        Provider::Gemini => &GEMINI_ADAPTER,
        Provider::Goose => &GOOSE_ADAPTER,
        Provider::KimiCode => &KIMI_ADAPTER,
        Provider::OpenCode => &OPENCODE_ADAPTER,
        Provider::QwenCode => &QWEN_ADAPTER,
    }
}
```

### 3.4 Incoming Payload Conversion

Native events flow through a three-stage pipeline to reach the unified model:

```
[Native JSON payload]
       |
       v
  ProviderAdapter::parse_event()
       |
       v
  (AgenticEvent, EventMeta)
       |
       v
  Config lookup: provider + event -> EventBinding
       |
       v
  Matcher filter: regex against meta.tool_name / meta.notification_type / etc.
       |
       v
  ResolvedHook { event, meta, provider, actions, can_block }
       |
       v
  execute_actions() -> Option<HookResponse>
       |
       v
  ProviderAdapter::format_response() -> native JSON (if can_block)
       |
       v
  ProviderAdapter::exit_code() -> process exit code (if shell-based)
```

The `EventMeta` struct carries all normalized fields:

```rust
/// Normalized metadata attached to every fired event.
///
/// Provider adapters populate this from their native event payloads.
/// The `extra` map carries provider-specific fields that don't fit
/// the common schema.
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
    pub agent_type: Option<String>,
    pub notification_type: Option<String>,
    pub notification_message: Option<String>,
    pub extra: HashMap<String, Value>,
    pub env: EnvironmentContext,
}
```

Provider-specific fields that do not fit the common schema go into `extra`:

| Provider | Key Extra Fields |
|---|---|
| Claude Code | `permission_mode`, `stop_hook_active`, `transcript_path`, `tool_use_id`, `model`, `is_interrupt`, `permission_suggestions`, `teammate_name`, `team_name`, `task_id`, `task_subject`, `task_description` |
| Codex CLI | `thread_id`, `thread-id` (legacy notify payload), `token_usage`, `item_type`, `item_id` |
| Gemini CLI | `llm_request`, `llm_response`, `tool_config`, `aggregation_strategy`, `mcp_context` |
| Goose | `goose_mode`, `message_content_type`, `total_tokens`, `subagent_notification_type` |
| Kimi Code | `step_number`, `approval_id`, `display_blocks`, `content_variant`, `subagent_nested_event_type` |
| OpenCode | `bus_event_type`, `plugin_context`, `auth_method` |
| Qwen CLI | `approval_mode`, `permission_priority`, `subagent_id`, `can_use_tool_timeout_secs` |

`extra` is intentionally open-ended and should preserve provider-native key naming when useful for debugging (`thread-id` vs `thread_id`, etc.).

### 3.5 `HookAction` Enum

`HookAction` is the **only** action type in library code for the refactor target.

`EventAction` is not a library runtime type in this design. If legacy configuration migration is needed, it is handled by offline migration tooling (for example, a one-time `claudine migrate-config` command) that rewrites configs to canonical `HookAction` before library load.

```rust
use std::collections::HashMap;
use std::path::PathBuf;

use serde::{Deserialize, Serialize};

/// Logging destination for `HookAction::Log`.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case", deny_unknown_fields)]
#[non_exhaustive]
pub enum LogTarget {
    /// Append JSONL records to a local file.
    File {
        /// Optional explicit path. When omitted, Claudine uses
        /// `~/.claudine/logs/YYYY-MM-DD.jsonl`.
        #[serde(default, skip_serializing_if = "Option::is_none")]
        path: Option<PathBuf>,

        /// Whether default file paths rotate by local day boundary.
        #[serde(default = "default_true")]
        rotate_daily: bool,
    },

    /// POST structured hook records to an HTTP endpoint.
    Server {
        /// Endpoint URL.
        url: String,

        /// Request timeout in milliseconds (default: 10_000).
        #[serde(default = "default_log_timeout_ms")]
        timeout_ms: u64,

        /// Optional additional HTTP headers.
        #[serde(default, skip_serializing_if = "Option::is_none")]
        headers: Option<HashMap<String, String>>,
    },
}

/// An action to execute when a unified hook fires.
///
/// Actions execute in declaration order. Multiple actions can be bound
/// to a single event. Fire-and-forget actions (`SoundEffect`, `Speak`,
/// `FireAndForget`) spawn tokio tasks and never block the pipeline.
/// Synchronous actions (`Log`, `Report`) run inline. The `Call` action
/// blocks and can produce a `HookResponse` that flows back to the agent.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case", deny_unknown_fields)]
#[non_exhaustive]
pub enum HookAction {
    /// Play an embedded sound effect from the playa library.
    ///
    /// Uses playa's 88 built-in effects across 6 feature-gated categories:
    /// `sfx-ui` (43 effects), `sfx-cartoon` (13), `sfx-reactions` (9),
    /// `sfx-scifi` (11), `sfx-atmosphere` (5), `sfx-motion` (7).
    ///
    /// Playback is non-blocking (spawns a tokio task). Invalid effect
    /// names are caught at config validation time; use `claudine hooks --fix`
    /// for fuzzy-match suggestions.
    ///
    /// ## Examples
    ///
    /// ```json
    /// { "type": "sound_effect", "name": "success" }
    /// { "type": "sound_effect", "name": "sad-trombone", "volume": 0.5, "speed": 1.25 }
    /// ```
    SoundEffect {
        /// Effect name matching one of playa's built-in effects.
        ///
        /// Common effects: success, error, notification (UI); boing, pop,
        /// whoosh (cartoon); applause, sad-trombone, drumroll (reactions);
        /// laser, power-up, alarm (sci-fi); swoosh, impact (motion).
        name: String,

        /// Playback volume (0.0 = silent, 1.0 = full). Default: 1.0.
        #[serde(default = "default_volume")]
        volume: f32,

        /// Playback speed multiplier (1.0 = normal). Default: 1.0.
        #[serde(default = "default_speed")]
        speed: f32,
    },

    /// Speak a message aloud using biscuit-speaks TTS.
    ///
    /// The message is a Handlebars template with access to event payload
    /// fields, environment variables, and context variables (see section 3.7).
    /// During migration, single-brace placeholders are accepted and rewritten
    /// to Handlebars form before template evaluation.
    ///
    /// TTS playback is non-blocking (spawns a tokio task). The provider is
    /// selected automatically by biscuit-speaks based on the host system:
    /// - **macOS**: Say -> Kokoro -> EchoGarden -> Piper -> Sherpa -> ESpeak
    /// - **Linux**: Kokoro -> EchoGarden -> Sherpa -> Piper -> Mimic3 -> ESpeak
    /// - **Windows**: Sapi -> Piper -> EchoGarden -> Kokoro -> Sherpa -> ESpeak
    ///
    /// Cloud providers (ElevenLabs) are never used automatically. The TTS
    /// provider can be overridden in `~/.hooker` settings.
    ///
    /// ## Examples
    ///
    /// ```json
    /// { "type": "speak", "message": "Tool {{tool_name}} completed" }
    /// { "type": "speak", "message": "{{env.GREETING || \"Hello\"}}, session started" }
    /// ```
    Speak {
        /// Handlebars template message. See section 3.7 for available variables.
        message: String,
    },

    /// Write the event to a configured log target.
    ///
    /// `LogTarget::File` appends JSONL records and supports daily partitioning.
    /// `LogTarget::Server` posts structured records to an HTTP endpoint.
    /// Logging executes inline and is treated as best-effort (errors are logged,
    /// not surfaced to the originating agent).
    ///
    /// ## Examples
    ///
    /// ```json
    /// { "type": "log", "target": { "type": "file" } }
    /// { "type": "log", "target": { "type": "file", "path": "~/.claudine/logs/custom.jsonl" } }
    /// { "type": "log", "target": { "type": "server", "url": "https://hooks.example.com/events" } }
    /// ```
    Log {
        /// Destination target.
        #[serde(default = "default_log_target")]
        target: LogTarget,
    },

    /// Execute a command asynchronously without waiting for a result.
    ///
    /// The command spawns in the background. If it fails, the error is
    /// logged to the Claudine log stream but does not affect the hook
    /// response or the agent's behavior. Stdout and stderr are captured
    /// for logging but never returned to the agent.
    ///
    /// Command and args support Handlebars template interpolation.
    ///
    /// ## Examples
    ///
    /// ```json
    /// { "type": "fire_and_forget", "command": "notify-send", "args": ["Task done"] }
    /// { "type": "fire_and_forget", "command": "curl", "args": ["-X", "POST", "https://hooks.example.com/{{event}}"] }
    /// ```
    FireAndForget {
        /// Command name or path to executable. Supports template interpolation.
        command: String,

        /// Optional arguments. Each arg supports template interpolation.
        #[serde(default, skip_serializing_if = "Option::is_none")]
        args: Option<Vec<String>>,
    },

    /// Execute a command synchronously and use its output to form a response.
    ///
    /// The command blocks the hook pipeline until it completes. Its stdout
    /// is captured and optionally transformed via a `Mapper` before being
    /// converted to a `HookResponse`. Only effective when the event
    /// supports blocking (`ResolvedHook::can_block == true`); on non-blocking
    /// events, behaves like `FireAndForget` (output is logged but discarded).
    ///
    /// Command and args support Handlebars template interpolation.
    ///
    /// ## Examples
    ///
    /// ```json
    /// {
    ///   "type": "call",
    ///   "command": "security-scanner",
    ///   "args": ["--check", "{{tool_name}}"],
    ///   "mapper": { "type": "json_field", "field": "decision" }
    /// }
    /// ```
    Call {
        /// Command name or path to executable. Supports template interpolation.
        command: String,

        /// Optional arguments. Each arg supports template interpolation.
        #[serde(default, skip_serializing_if = "Option::is_none")]
        args: Option<Vec<String>>,

        /// Optional timeout in milliseconds. When omitted, provider-specific
        /// default timeouts are used.
        #[serde(default, skip_serializing_if = "Option::is_none")]
        timeout_ms: Option<u64>,

        /// Optional response mapper. Transforms command output into a
        /// `HookResponse`. When `None`, defaults to `ExitCode` mapping
        /// (exit 0 = allow, exit 2 = deny, stdout = reason).
        #[serde(default, skip_serializing_if = "Option::is_none")]
        mapper: Option<Mapper>,
    },

    /// Report the event into the agent's output stream.
    ///
    /// Writes formatted event information to stdout. Used for debugging
    /// and audit trails. Synchronous (inline).
    Report {
        /// Report output handler. `None` uses default plain-text summary.
        #[serde(default, skip_serializing_if = "Option::is_none")]
        handler: Option<ReportHandler>,
    },
}

fn default_volume() -> f32 { 1.0 }
fn default_speed() -> f32 { 1.0 }
fn default_true() -> bool { true }
fn default_log_timeout_ms() -> u64 { 10_000 }
fn default_log_target() -> LogTarget {
    LogTarget::File {
        path: None,
        rotate_daily: true,
    }
}
```

### 3.6 `Mapper` and `HookResponse`

The `Mapper` transforms raw command output from a `Call` action into a structured `HookResponse`. The `HookResponse` is then converted back to the provider's native format by the adapter.

```rust
use serde::{Deserialize, Serialize};
use serde_json::Value;

/// Transforms raw command output into a structured `HookResponse`.
///
/// Applied to the stdout of a `Call` action before the response flows
/// back to the originating agent via the provider adapter.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case", deny_unknown_fields)]
#[non_exhaustive]
pub enum Mapper {
    /// Parse stdout as JSON and extract a specific field as the decision.
    ///
    /// The field value is interpreted as the decision string. For example,
    /// if stdout is `{"decision": "allow", "reason": "safe"}` and
    /// `field` is `"decision"`, the response decision becomes `Allow`.
    /// A sibling `"reason"` field is automatically extracted if present.
    JsonField {
        /// Dot-separated path into the JSON object (e.g., `"result.decision"`).
        field: String,
    },

    /// Parse stdout as JSON and use the entire object as the response.
    ///
    /// The JSON must conform to `HookResponse` schema or be a raw
    /// provider-native response object (stored in `HookResponse::raw`
    /// and passed through to the adapter).
    JsonObject,

    /// Interpret the exit code as the decision.
    ///
    /// Exit 0 = Allow, Exit 2 = Deny, other = Allow with warning.
    /// Stdout is used as the reason/message. This is the **default**
    /// when no mapper is specified.
    ExitCode,

    /// Map stdout lines to specific response fields using named regex
    /// capture groups.
    ///
    /// Groups named `decision`, `reason`, `context` are recognized.
    Regex {
        /// Regex pattern with named capture groups.
        /// Example: `"(?P<decision>allow|deny)\\s+(?P<reason>.*)"`.
        pattern: String,
    },
}

/// Runtime mapper form used by the dispatcher.
///
/// Regex patterns are compiled at config load time, not per event.
pub enum CompiledMapper {
    JsonField { field: String },
    JsonObject,
    ExitCode,
    Regex { pattern: regex::Regex },
}

/// Unified response that a hook can return to influence agent behavior.
///
/// Only meaningful for blocking events. The provider adapter converts
/// this into the native response format expected by the originating CLI.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(default, deny_unknown_fields)]
pub struct HookResponse {
    /// The decision to communicate back to the agent.
    ///
    /// `None` means "no opinion" -- the agent proceeds with default behavior.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub decision: Option<HookDecision>,

    /// Human-readable reason for the decision.
    ///
    /// How this is used depends on the provider and event:
    /// - Claude Code: shown to user (allow/ask) or to Claude (deny)
    /// - Gemini CLI: used as correction text on AfterAgent retry
    /// - Kimi Code: included in approval/rejection response
    /// - OpenCode: used as throw message on deny
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub reason: Option<String>,

    /// Modified tool input to substitute before execution.
    ///
    /// Applicable to `before_tool` and `permission_request` events on
    /// providers that support input modification (Claude, Gemini, OpenCode).
    /// Ignored by providers without input modification support.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub updated_input: Option<Value>,

    /// Additional context string to inject into the agent's context.
    ///
    /// Supported by Claude Code (SessionStart and tool events via
    /// additionalContext), Gemini (SessionStart), and OpenCode (via mutation).
    /// Ignored by other providers.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub additional_context: Option<String>,

    /// Raw provider-specific response fields.
    ///
    /// When a `Call` action's `JsonObject` mapper produces provider-native
    /// JSON, it is stored here and passed through directly to the adapter's
    /// `format_response()` method without further interpretation by the
    /// unified pipeline.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub raw: Option<Value>,
}

/// Decisions a hook can communicate back to the agent.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
#[non_exhaustive]
pub enum HookDecision {
    /// Allow the action to proceed.
    Allow,

    /// Block/deny the action.
    Deny,

    /// Show the user the permission dialog (Claude Code PreToolUse only).
    ///
    /// Falls back to `Deny` on providers that don't support three-level
    /// permission decisions.
    Ask,

    /// Continue the conversation instead of stopping.
    ///
    /// Used by Claude Code Stop/SubagentStop to keep the agent going.
    /// Gemini CLI AfterAgent uses this to trigger retry with correction.
    /// Falls back to `Allow` on providers that don't support continuation.
    Continue,
}
```

Failure semantics:

- `HookResponse::default()` means "no opinion" and is converted to provider-default allow behavior.
- A failed `Call` action execution (command error or timeout) yields **no** `HookResponse` from that action.
- Those two outcomes are intentionally different and must remain distinguishable in logs and metrics.

### 3.7 Context Variables

Template variables available in `Speak` messages, `FireAndForget` args, and `Call` args. Variables use Handlebars double-brace syntax (`{{...}}`).

#### Event Fields

| Variable | Source | Available On |
|---|---|---|
| `{{provider}}` | `meta.provider` | All events |
| `{{event}}` | `meta.event` | All events |
| `{{session_id}}` | `meta.session_id` | All events (may be empty) |
| `{{cwd}}` | `meta.cwd` | All events (may be empty) |
| `{{tool_name}}` | `meta.tool_name` | `before_tool`, `after_tool`, `tool_error`, `permission_request` |
| `{{error}}` | `meta.error` | `tool_error`, `turn_error` |
| `{{prompt}}` | `meta.prompt` | `before_prompt` |
| `{{timestamp}}` | `meta.timestamp` | All events (ISO 8601 UTC) |
| `{{agent_type}}` | `meta.agent_type` | `subagent_start`, `subagent_stop` |
| `{{notification_type}}` | `meta.notification_type` | `notification` |

#### Environment Variables

| Syntax | Description |
|---|---|
| `{{env.VAR_NAME}}` | Value of environment variable `VAR_NAME`. Empty string if unset. |
| `{{env.VAR_NAME \|\| "default"}}` | Value of `VAR_NAME` with fallback to `"default"` if unset or empty. |

#### OS Context (auto-detected via sniff)

| Variable | Source | Example |
|---|---|---|
| `{{os.name}}` | `meta.env.os.name` | `"macOS"` |
| `{{os.type}}` | `meta.env.os.os_type` | `"macos"` |
| `{{os.version}}` | `meta.env.os.version` | `"15.3"` |
| `{{os.hostname}}` | `meta.env.os.hostname` | `"kens-laptop"` |

#### Hardware Context

| Variable | Source | Example |
|---|---|---|
| `{{hardware.arch}}` | `meta.env.hardware.arch` | `"aarch64"` |
| `{{hardware.cpu}}` | `meta.env.hardware.cpu` | `"Apple M4 Max"` |
| `{{hardware.cores}}` | `meta.env.hardware.cores` | `"16"` |

#### Git Context

| Variable | Source | Example |
|---|---|---|
| `{{git.branch}}` | `meta.env.git.branch` | `"main"` |
| `{{git.is_dirty}}` | `meta.env.git.is_dirty` | `"true"` |
| `{{git.head_sha}}` | `meta.env.git.head_sha` | `"abc123f"` |
| `{{git.head_message}}` | `meta.env.git.head_message` | `"fix: resolve race"` |
| `{{git.remote}}` | `meta.env.git.remote_url` | `"git@github.com:..."` |
| `{{git.hosting}}` | `meta.env.git.hosting_provider` | `"github"` |
| `{{git.repo_name}}` | `meta.env.git.repo_name` | `"rusty-biscuit"` |
| `{{git.repo_org}}` | `meta.env.git.repo_org` | `"yankeeinlondon"` |

#### Project Context

| Variable | Source | Example |
|---|---|---|
| `{{project.language}}` | `meta.env.primary_language` | `"Rust"` |
| `{{project.is_monorepo}}` | `meta.env.repo.is_monorepo` | `"true"` |
| `{{project.monorepo_standard}}` | `meta.env.repo.monorepo_standard` | `"cargo-workspace"` |
| `{{project.monorepo_orchestrators}}` | `meta.env.repo.monorepo_orchestrators` | `"nx"` |
| `{{project.monorepo_tool}}` | `meta.env.repo.monorepo_standard` (deprecated alias) | `"cargo-workspace"` |

**Resolution rules:**

1. Unknown placeholders are left as-is in the output string (no error).
2. `None` values render as empty strings.
3. Environment variable lookups use `std::env::var()` at template evaluation time (not cached).
4. OS/hardware/git/project context fields are snapshotted once at session start via `sniff` and reused for all events in the session.
5. The `env.VAR || "default"` fallback syntax uses the default when the variable is not set or empty (Darkmatter `||` is short-circuit on falsy/empty values). The legacy single-pipe `|` form is no longer supported -- see the migration note below.
6. Migration compatibility: legacy single-brace placeholders (`{tool_name}`) are accepted during transition, rewritten to `{{tool_name}}`, and logged with a deprecation warning.

#### Migration: single-pipe fallback removed

Earlier Claudine builds accepted `{{env.VAR | "default"}}` (single pipe) as a fallback. The new template engine uses Darkmatter's expression parser, which only accepts `||`. Any remaining single-pipe templates are preserved verbatim in output (no error, no replacement). Search your config for ` | ` inside `{{...}}` blocks and replace with ` || `.

### 3.8 Comprehensive Mapping Tables

#### AgenticEvent to Native Events

| AgenticEvent | Claude Code | Codex CLI | Gemini CLI | Goose | Kimi Code | OpenCode | Qwen CLI |
|---|---|---|---|---|---|---|---|
| `session_start` | SessionStart | ThreadStarted | SessionStart | -- | -- | session.created | StreamSessionStart |
| `session_end` | SessionEnd | -- | SessionEnd | Complete | -- | session.deleted | StreamResult |
| `before_prompt` | UserPromptSubmit | TurnStarted | BeforeAgent | -- | TurnBegin | chat.message | -- |
| `before_tool` | PreToolUse | ItemStarted (`command_execution`, `file_change`, `mcp_tool_call`) | BeforeTool | -- | ToolCall, ToolCallPart, ToolCallRequest | tool.execute.before | SubagentPreToolUse |
| `after_tool` | PostToolUse | ItemCompleted | AfterTool | -- | ToolResult | tool.execute.after | SubagentPostToolUse |
| `tool_error` | PostToolUseFailure | -- | -- | -- | ToolResult (is_error=true) | -- | -- |
| `permission_request` | PermissionRequest | -- | -- | -- | ApprovalRequest, ApprovalResponse | permission.ask | CanUseTool |
| `notification` | Notification | ItemUpdated (`agent_message`, `reasoning`, `web_search`, `plan_update`) | Notification | Notification, ModelChange | StatusUpdate, StepBegin, CompactionEnd | event (bus) | -- |
| `subagent_start` | SubagentStart | -- | -- | Notification (`subagent_tool_request`) | SubagentEvent | -- | -- |
| `subagent_stop` | SubagentStop | -- | -- | Notification (`tasks_complete`) | SubagentEvent (done) | -- | SubagentStop |
| `turn_complete` | Stop, TeammateIdle, TaskCompleted | TurnCompleted, AfterAgent | AfterAgent | StatusWaiting | TurnEnd | session.idle | -- |
| `turn_error` | -- | TurnFailed, Error | -- | Error | StepInterrupted | session.error | -- |
| `before_model` | -- | -- | BeforeModel, BeforeToolSelection | StatusThinking | -- | chat.params, chat.headers, Experimental* | -- |
| `after_model` | -- | ItemUpdated | AfterModel | Message | ContentPart | ExperimentalTextComplete | StreamAssistantMessage |
| `before_compact` | PreCompact | -- | PreCompress | -- | CompactionBegin | ExperimentalSessionCompacting | -- |
| `human_in_the_loop` | -- | -- | -- | -- | ApprovalRequest | permission.asked (observational event-bus signal) | -- |

Mapping caveats:

1. Codex `item.started` is only tool-like for `command_execution`, `file_change`, and `mcp_tool_call`; other item types are mapped through `notification`/`after_model`.
2. OpenCode `event` bus payloads must be de-duplicated against specific hook mappings (`session.*`, `permission.ask`, etc.) to avoid double emission.
3. Kimi `SubagentEvent` requires inspecting nested payload type to distinguish start vs stop.
4. Qwen `CanUseTool` has a hard 60-second timeout; adapters should annotate this in `meta.extra.can_use_tool_timeout_secs`.
5. Design scope intentionally excludes speculative `human_in_the_loop` mappings for Codex/Goose until those surfaces are documented as stable hook contracts.

#### Blocking Capability per AgenticEvent per Provider

This table shows which provider-event combinations support blocking (the hook can return a response that influences agent behavior):

| AgenticEvent | Claude | Codex | Gemini | Goose | Kimi | OpenCode | Qwen |
|---|:---:|:---:|:---:|:---:|:---:|:---:|:---:|
| `session_start` | -- | -- | -- | -- | -- | -- | -- |
| `session_end` | -- | -- | -- | -- | -- | -- | -- |
| `before_prompt` | **B** | -- | **B** | -- | -- | -- | -- |
| `before_tool` | **B** | -- | **B** | -- | **B** | **B** | -- |
| `after_tool` | **B*** | -- | **B** | -- | -- | -- | -- |
| `tool_error` | -- | -- | -- | -- | -- | -- | -- |
| `permission_request` | **B** | -- | -- | -- | **B** | **B** | **B** |
| `notification` | -- | -- | -- | -- | -- | -- | -- |
| `subagent_start` | -- | -- | -- | -- | -- | -- | -- |
| `subagent_stop` | **B** | -- | -- | -- | -- | -- | -- |
| `turn_complete` | **B** | -- | **B** | -- | -- | -- | -- |
| `turn_error` | -- | -- | -- | -- | -- | -- | -- |
| `before_model` | -- | -- | **B** | -- | -- | -- | -- |
| `after_model` | -- | -- | **B** | -- | -- | -- | -- |
| `before_compact` | -- | -- | -- | -- | -- | -- | -- |
| `human_in_the_loop` | -- | -- | -- | -- | **B** | -- | -- |

**B** = Blocking supported (return value influences agent behavior)

**B*** = Advisory blocking (response can influence follow-up behavior but cannot undo already-executed tool side effects)

### 3.9 Response Flow

The response flow handles the fundamental asymmetry between providers that support blocking and those that do not.

```
 [HookAction executes]
        |
        v
   Is action type `Call`?
        |
     No |              Yes
        v                v
   (no response)    [Command runs synchronously, stdout captured]
                         |
                         v
                    [Mapper applied (ExitCode default)]
                         |
                         v
                    HookResponse { decision, reason, updated_input, ... }
                         |
                         v
                    Does provider support blocking for this event?
                    (ResolvedHook::can_block)
                         |
                      No |              Yes
                         v                v
                    [Log response,   [ProviderAdapter::format_response()]
                     discard]              |
                                           v
                                      Native JSON response
                                           |
                                           v
                                      [Write to stdout / return via protocol]
                                           |
                                           v
                                      [ProviderAdapter::exit_code()]
                                           |
                                           v
                                      [Process exits with appropriate code]
```

#### Response Conversion Examples

**Claude Code** -- PreToolUse (three-level permission):

```rust
// HookResponse { decision: Some(Deny), reason: Some("unsafe command") }
// Converts to:
{
    "hookSpecificOutput": {
        "hookEventName": "PreToolUse",
        "permissionDecision": "deny",
        "permissionDecisionReason": "unsafe command"
    }
}
// Exit code: 0 (JSON carries the decision)
```

**Claude Code** -- Stop (continue conversation):

```rust
// HookResponse { decision: Some(Continue), reason: Some("not done yet") }
// Converts to:
{
    "decision": "block",
    "reason": "not done yet"
}
// Exit code: 0 (JSON carries the decision; exit 2 would also work)
```

**Gemini CLI** -- BeforeTool (block tool, continue turn):

```rust
// HookResponse { decision: Some(Deny), reason: Some("disallowed tool") }
// Converts to:
{
    "error": "disallowed tool"
}
// Exit code: 2 (means "block tool, continue turn" for BeforeTool)
```

**Gemini CLI** -- AfterAgent (reject and retry):

```rust
// HookResponse { decision: Some(Deny), reason: Some("response too short") }
// Converts to:
{
    "reason": "response too short",
    "clearContext": false
}
// Exit code: 2 (means "reject, retry with reason as correction" for AfterAgent)
```

**Kimi Code** -- ApprovalRequest (JSON-RPC response):

```rust
// HookResponse { decision: Some(Allow) }
// Converts to JSON-RPC result:
{
    "jsonrpc": "2.0",
    "id": <request_id>,
    "result": { "decision": "approve" }
}
// No exit code (JSON-RPC protocol)
```

**OpenCode** -- ToolExecuteBefore (throw to block):

```rust
// HookResponse { decision: Some(Deny), reason: Some("blocked by policy") }
// Adapter signals the plugin bridge to throw:
{
    "__action": "throw",
    "message": "blocked by policy"
}
// No exit code (in-process)
```

**OpenCode** -- PermissionAsk (mutate status):

```rust
// HookResponse { decision: Some(Allow) }
// Adapter signals the plugin bridge to set output.status:
{
    "__action": "mutate",
    "status": "allow"
}
// No exit code (in-process)
```

**Non-blocking providers** (Codex, Goose, Qwen stream):

The `Call` action still executes (its output is logged), but `format_response()` returns `Value::Null` and the response is discarded. This allows a single config to work across all providers -- blocking where supported, observing where not.

If a `Call` command fails (process error, timeout, mapper parse error), the pipeline emits no `HookResponse` for that action and falls back to provider-default behavior for the event.

### 3.10 Action Execution Pipeline

```rust
use std::sync::Arc;
use std::time::Duration;

/// Execute all actions for a resolved hook.
///
/// Fire-and-forget actions (`SoundEffect`, `Speak`, `FireAndForget`) spawn
/// background tokio tasks. Synchronous actions (`Log`, `Report`) run inline.
/// `Call` actions block and may produce a `HookResponse`.
///
/// ## Returns
///
/// The selected `HookResponse` produced by `Call` actions, or `None`
/// if no `Call` actions are present or none produced a response.
///
/// Selection strategy for multiple `Call` responses:
/// 1. Keep the first response by default (stable ordering).
/// 2. If a later response requests `STOP` and the current selected
///    response requests `CONTINUE`, replace the selected response.
/// 3. Otherwise keep the currently selected response.
pub async fn execute_actions(
    hook: Arc<ResolvedHook>,
    tts_config: Arc<TtsConfig>,
) -> Option<HookResponse> {
    const DEFAULT_CALL_TIMEOUT: Duration = Duration::from_secs(60);

    let mut response: Option<HookResponse> = None;

    let prefers_stop_over_continue = |current: &HookResponse, candidate: &HookResponse| -> bool {
        let current_is_continue = matches!(current.decision, Some(HookDecision::Continue));
        let candidate_is_continue = matches!(candidate.decision, Some(HookDecision::Continue));

        // Any non-Continue decision is treated as a `STOP` response for
        // precedence purposes in multi-Call resolution.
        !candidate_is_continue && current_is_continue
    };

    for action in &hook.actions {
        match action {
            HookAction::SoundEffect { name, volume, speed } => {
                let name = name.clone();
                let vol = *volume;
                let spd = *speed;
                tokio::spawn(async move {
                    if let Err(e) = play_sound_effect(&name, vol, spd).await {
                        tracing::warn!(effect = %name, error = %e, "Sound effect playback failed");
                    }
                });
            }

            HookAction::Speak { message } => {
                let rendered = render_template(message, &hook.meta);
                let tts = Arc::clone(&tts_config);
                tokio::spawn(async move {
                    if let Err(e) = speak_text(&rendered, &tts).await {
                        tracing::warn!(error = %e, "TTS playback failed");
                    }
                });
            }

            HookAction::Log { target } => match target {
                LogTarget::File { path, rotate_daily } => {
                    let log_path = resolve_log_path(path.as_deref(), *rotate_daily);
                    if let Err(e) = append_log_entry(&hook.meta, &log_path).await {
                        tracing::warn!(path = %log_path.display(), error = %e, "Log append failed");
                    }
                }
                LogTarget::Server {
                    url,
                    timeout_ms,
                    headers,
                } => {
                    if let Err(e) = post_log_entry(&hook.meta, url, *timeout_ms, headers.as_ref()).await {
                        tracing::warn!(url = %url, error = %e, "Remote log post failed");
                    }
                }
            },

            HookAction::FireAndForget { command, args } => {
                let cmd = render_template(command, &hook.meta);
                let rendered_args: Option<Vec<String>> = args.as_ref().map(|a| {
                    a.iter().map(|arg| render_template(arg, &hook.meta)).collect()
                });
                tokio::spawn(async move {
                    if let Err(e) = run_command(&cmd, rendered_args.as_deref()).await {
                        tracing::warn!(command = %cmd, error = %e, "Fire-and-forget command failed");
                    }
                });
            }

            HookAction::Call {
                command,
                args,
                timeout_ms,
                mapper,
            } => {
                let cmd = render_template(command, &hook.meta);
                let rendered_args: Option<Vec<String>> = args.as_ref().map(|a| {
                    a.iter().map(|arg| render_template(arg, &hook.meta)).collect()
                });
                let mapper_ref = mapper.as_ref();
                let timeout = timeout_ms
                    .map(Duration::from_millis)
                    .unwrap_or(DEFAULT_CALL_TIMEOUT);

                match tokio::time::timeout(timeout, run_command_blocking(&cmd, rendered_args.as_deref())).await {
                    Ok(Ok(output)) => {
                        match apply_mapper(mapper_ref, &output) {
                            Ok(mapped) => {
                                match response.as_ref() {
                                    None => {
                                        response = Some(mapped);
                                    }
                                    Some(current) if prefers_stop_over_continue(current, &mapped) => {
                                        tracing::debug!(
                                            command = %cmd,
                                            "Call response replaced (stop overrides continue)"
                                        );
                                        response = Some(mapped);
                                    }
                                    Some(_) => {
                                        tracing::debug!(
                                            command = %cmd,
                                            "Additional Call response discarded (selected response retained)"
                                        );
                                    }
                                }
                            }
                            Err(e) => {
                                tracing::warn!(command = %cmd, error = %e, "Call mapper failed");
                            }
                        }
                    }
                    Ok(Err(e)) => {
                        tracing::warn!(command = %cmd, error = %e, "Call command failed");
                    }
                    Err(_) => {
                        tracing::warn!(command = %cmd, timeout_ms = timeout.as_millis(), "Call command timed out");
                    }
                }
            }

            HookAction::Report { handler } => {
                report_event(&hook.meta, handler.as_ref());
            }
        }
    }

    response
}
```

### 3.11 Sound Effect Integration (playa)

Sound effects use the `playa` library's embedded effects system.

```rust
use playa::{Playa, SoundEffect};
use thiserror::Error;

#[derive(Debug, Error)]
enum SoundActionError {
    #[error("unknown sound effect: {0}")]
    UnknownEffect(String),
    #[error(transparent)]
    Playa(#[from] playa::Error),
}

/// Play a named sound effect with volume and speed adjustments.
///
/// Delegates to playa's `Playa` builder which selects the best available
/// audio player on the host system (mpv, FFplay, SoX, afplay, etc.).
async fn play_sound_effect(name: &str, volume: f32, speed: f32) -> Result<(), SoundActionError> {
    let effect = SoundEffect::from_name(name)
        .ok_or_else(|| SoundActionError::UnknownEffect(name.to_string()))?;

    Playa::from_bytes(effect.bytes().to_vec())?
        .volume(volume)
        .speed(speed)
        .play_async()
        .await?;

    Ok(())
}
```

playa provides 88 effects across 6 feature-gated categories. Claudine enables the `sound-effects` meta-feature (all categories) by default, adding approximately 31MB to the binary. Invalid effect names are caught at config validation time (`claudine hooks --fix` provides 5-tier fuzzy-match suggestions).

### 3.12 TTS Integration (biscuit-speaks)

TTS uses the `biscuit-speaks` library's `Speak` builder for automatic provider selection and failover.

```rust
use biscuit_speaks::Speak;

#[derive(Debug, Clone, Default)]
pub struct TtsConfig {
    pub provider: Option<String>,
    pub voice: Option<String>,
    pub gender: Option<String>,
    pub volume: Option<f32>,
    pub speed: Option<f32>,
}

/// Speak rendered text using the best available TTS provider.
///
/// Provider selection is automatic based on the host OS. Voice capability
/// caching prevents expensive re-enumeration. Audio file caching uses
/// content-addressed storage (xxHash) to avoid re-synthesis of identical text.
async fn speak_text(text: &str, cfg: &TtsConfig) -> Result<(), biscuit_speaks::TtsError> {
    // Provider strategy selection is resolved from cfg.provider by the
    // configuration layer before this call.
    let mut speak = Speak::new(text);

    if let Some(voice) = cfg.voice.as_deref() {
        speak = speak.with_voice(voice);
    }
    if let Some(gender) = cfg.gender.as_deref() {
        speak = speak.with_gender(gender);
    }
    if let Some(volume) = cfg.volume {
        speak = speak.with_volume(volume);
    }
    if let Some(speed) = cfg.speed {
        speak = speak.with_speed(speed);
    }

    speak.play().await
}
```

The TTS provider and voice parameters flow from `~/.hooker` (`settings.tts.*`) into `TtsConfig` at config-load time. Cloud providers (ElevenLabs) are never selected automatically -- they must be explicitly configured via provider strategy settings.

---

## 4. Design Rationale

### 4.1 `AgenticEvent` IS the Unified Hook

The design does not introduce a separate `UnifiedHook` enum. The existing `AgenticEvent` enum with its 16 variants already represents the complete set of hooks Claudine supports. Adding another enum would create a redundant abstraction layer -- every variant would map 1:1 to an `AgenticEvent` variant, with no additional information. Instead, the "unified hook" is the `ResolvedHook` struct that pairs an `AgenticEvent` with metadata, actions, and blocking capability.

### 4.2 Adapter Pattern for Provider Diversity

The provider CLIs use fundamentally different architectures:

| Architecture | CLIs |
|---|---|
| Shell commands with JSON I/O and exit codes | Claude Code, Gemini CLI |
| JSONL/NDJSON output streams | Codex, Goose, Qwen |
| JSON-RPC 2.0 bidirectional protocol | Kimi Code |
| In-process JS/TS plugin functions | OpenCode |
| Rust SDK async callbacks | Qwen (CanUseTool only) |

The `ProviderAdapter` trait abstracts over all of these. Each adapter knows how to parse its provider's native format into `(AgenticEvent, EventMeta)` and how to format `HookResponse` back into the native response format. The core dispatch pipeline operates entirely on unified types.

### 4.3 Graceful Degradation for Non-Blocking Providers

Within the original seven-provider research set, five CLIs support some form of blocking (Claude, Gemini, OpenCode, Kimi, Qwen), and the number of blockable events varies by provider. The unified model handles this through the `can_block` flag:

1. Every `ResolvedHook` carries a `can_block` boolean set by the adapter for the specific event
2. `Call` actions always execute regardless of `can_block` (for side effects and logging)
3. Response routing checks `can_block` before attempting to send a response back
4. On non-blocking events, the `HookResponse` is logged but discarded silently
5. If a `Call` command fails or times out, no `HookResponse` is emitted from that action

This means users can write a single configuration with `Call` actions, and it works across all providers -- blocking where the CLI supports it, logging where it does not. No per-provider configuration branching is needed.

### 4.4 Lossy Mappings Are Acceptable

Several native events map to the same `AgenticEvent` variant with information loss:

| Lossy Mapping | Native Events | Unified Event |
|---|---|---|
| Claude Code turn variants | Stop, TeammateIdle, TaskCompleted | `turn_complete` |
| Codex item lifecycle | ItemStarted/ItemUpdated mixed item kinds | `before_tool`, `after_model`, `notification` |
| Gemini tool selection | BeforeToolSelection | `before_model` |
| Kimi tool lifecycle | ToolCall, ToolCallPart, ToolCallRequest | `before_tool` |
| Kimi orchestration markers | StepBegin, CompactionEnd | `notification` |

This is by design. The unified model captures **lifecycle semantics meaningful for action dispatch** -- play a sound when a tool runs, speak when a turn completes, log when an error occurs. Provider-specific implementation details are preserved in `EventMeta::extra` for users who need fine-grained control. The per-provider event enums (in `designs/*.md`) remain available for provider-specific logic.

### 4.5 Template System Uses Handlebars Syntax

The template system uses `{{variable}}` double-brace Handlebars syntax rather than `{variable}` single-brace. This decision avoids three categories of ambiguity:

1. **JSON in shell**: Single braces conflict with JSON payloads passed as arguments
2. **Shell expansion**: Single braces can be interpreted by some shells
3. **Established convention**: Handlebars is widely understood and the `handlebars-rust` crate provides a proven implementation

Environment variables use the `env.` prefix with optional fallbacks via the Darkmatter `||` operator: `{{env.MESSAGE || "fallback"}}`. The legacy single-pipe `|` form is no longer supported (see the migration note in section 3.7).

Migration behavior: legacy single-brace syntax remains accepted temporarily, is rewritten to Handlebars form at load/render time, and emits deprecation warnings.

### 4.6 Sound Effects and TTS Are Fire-and-Forget

Both `SoundEffect` and `Speak` actions spawn tokio tasks and return immediately. This is essential because:

1. **TTS latency**: Provider detection + synthesis can take 1-3 seconds, which would exceed shell hook timeouts (Claude Code: 60s default, but responsiveness matters)
2. **Audio playback**: Even fast effects (50-200ms) should not add perceived latency to the agent's response
3. **Failure isolation**: Audio/TTS errors must never prevent the agent from proceeding; they are logged via `tracing::warn` but do not propagate

### 4.7 Log Partitioning by Day

The `Log` action supports both local JSONL append and HTTP POST delivery:

1. `LogTarget::File` (default):
   - Default path: `~/.claudine/logs/YYYY-MM-DD.jsonl`
   - Each entry: single JSON line with full `EventMeta` plus action execution metadata
   - Rotation: natural daily rotation prevents unbounded file growth
   - Override: explicit `path` field uses the given path as-is
2. `LogTarget::Server`:
   - Sends JSON payloads to an HTTP endpoint
   - Default request timeout: 10 seconds
   - Network errors are logged and do not block agent progress

### 4.8 `Mapper` for Flexible Response Transformation

The `Mapper` enum supports four strategies, from simple to complex:

| Strategy | Input | Use Case |
|---|---|---|
| `ExitCode` (default) | Process exit code + stdout | Simple allow/deny scripts |
| `JsonField` | JSON stdout, specific field | Security scanners with structured output |
| `JsonObject` | Entire JSON stdout | Full HookResponse or native passthrough |
| `Regex` | Named capture groups on stdout | Legacy tools with text output |

The default `ExitCode` mapper aligns with the convention established by Claude Code and Gemini CLI (exit 0 = allow, exit 2 = deny), making existing hook scripts work without modification.

Regex mappers are validated and compiled once during config load, so invalid patterns fail fast and do not incur per-event compilation overhead.

### 4.9 Forward Compatibility

The design is structured for extensibility without breaking changes:

| Extension Point | Mechanism |
|---|---|
| New providers | Implement `ProviderAdapter` trait; no core type changes |
| New events | Add variant to `AgenticEvent` (marked `#[non_exhaustive]`) |
| New actions | Add variant to `HookAction` (serde tagged enum, `#[non_exhaustive]`) |
| New decisions/mappers/providers | `HookDecision`, `Mapper`, and `Provider` are also `#[non_exhaustive]` |
| Provider-specific data | `EventMeta::extra` HashMap carries arbitrary fields |
| Native response passthrough | `HookResponse::raw` allows direct adapter bypass |
| New template variables | Add resolution logic to template engine; existing templates unaffected |

For config robustness, config-facing action/mapper payloads use strict field validation (`deny_unknown_fields`) in canonical mode. Any legacy-shape handling is expected to occur in external migration tooling, not in the library runtime.

### 4.10 Stop Overrides Continue

When multiple `Call` actions are bound to a single event, response selection uses `STOP`-over-`CONTINUE` precedence:

1. Start with the first `HookResponse` in action order.
2. If a later `Call` yields a `STOP` response while the currently selected response is `CONTINUE`, replace the selected response.
3. Otherwise retain the currently selected response.

This preserves deterministic ordering while ensuring safety-oriented stop decisions are honored over continue decisions.

### 4.11 Target-First Refactor Posture

This design intentionally leads implementation, not the other way around.

1. Provider/event support mismatches in current code are tracked as refactor tasks, not design compromises.
2. Canonical types in this document (`Provider`, `AgenticEvent`, `HookAction`, `Mapper`, `HookDecision`) define the long-term public model.
3. Legacy config migration belongs in external tooling; library internals remain canonical-only.
