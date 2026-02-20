# Unified Hooks: Events, Providers, and Dispatch

Reference for the claudine events module (`claudine/lib/src/events/`), covering the
unified event model, provider mappings, support levels, and hook resolution pipeline.

## Event Mapping to Providers

Each provider uses its own native event names. Claudine normalizes them into a single
`AgenticEvent` enum. The tables below show how each canonical event maps to
provider-native identifiers.

```
Table 1: Claude, Codex, Gemini, Goose

Event              Claude            Codex              Gemini       Goose
---------------------------------------------------------------------------
session_start      SessionStart      thread.started     SessionStart
session_end        SessionEnd                           SessionEnd
before_prompt      UserPromptSubmit  turn.started       BeforeAgent
before_tool        PreToolUse        item.started       BeforeTool
after_tool         PostToolUse       item.completed     AfterTool
tool_error         PostToolUseFailure error
permission_request PermissionRequest
human_in_the_loop  HumanInTheLoop    tool/requestUserInput              request_permission
turn_complete      Stop              turn.completed     AfterAgent   complete
turn_error                           turn.failed                     error
subagent_start     SubagentStart                                     subagent_tool_request
subagent_stop      SubagentStop                                      tasks_complete
before_model                                            BeforeModel
after_model                          agent_message      AfterModel   message
before_compact     PreCompact                           PreCompress
notification       Notification      reasoning          Notification notification


Table 2: Kimi Code, OpenCode, Qwen Code, Roo Code

Event              Kimi Code       OpenCode             Qwen Code  Roo Code
---------------------------------------------------------------------------
session_start                      session.created                 TaskCreated
session_end                        session.deleted                 TaskAborted
before_prompt      TurnBegin       chat.message
before_tool        ToolCall        tool.execute.before             ToolUseOutput
after_tool         ToolResult      tool.execute.after              ToolResultOutput
tool_error         ToolResult                                      TaskToolFailed
permission_request ApprovalRequest permission.ask       CanUseTool
human_in_the_loop  ApprovalRequest permission.asked                WaitingForInput
turn_complete      TurnEnd         session.idle         result     TaskCompleted
turn_error         prompt.status   session.error        result     Error
subagent_start     SubagentEvent                                   TaskSpawned
subagent_stop      SubagentEvent                                   TaskDelegationCompleted
before_model                       chat.params                     StreamingStarted
after_model        ContentPart     message.part.updated assistant  StreamingEnded
before_compact     CompactionBegin session.compacted
notification       StatusUpdate    tui.toast.show       system     ModeChanged

Blank = not supported or no specific native name.
```

## Event Support by Provider

Support level indicates _how_ an event can be captured from a given provider.

| Event | Claude | Codex | Gemini | Goose | Kimi Code | OpenCode | Qwen Code | Roo Code |
|---|---|---|---|---|---|---|---|---|
| session_start | Hook | NonHook | Hook | -- | -- | Hook | -- | NonHook |
| session_end | Hook | -- | Hook | -- | -- | Hook | -- | NonHook |
| before_prompt | Hook | NonHook | Hook | -- | NonHook | Hook | -- | -- |
| before_tool | Hook | NonHook | Hook | -- | NonHook | Hook | -- | NonHook |
| after_tool | Hook | NonHook | Hook | -- | NonHook | Hook | -- | NonHook |
| tool_error | Hook | NonHook | -- | -- | NonHook | -- | -- | NonHook |
| permission_request | Hook | -- | -- | -- | NonHook | Hook | NonHook | -- |
| human_in_the_loop | Hook | NonHook | -- | NonHook | NonHook | Hook | -- | NonHook |
| turn_complete | Hook | Hook | Hook | NonHook | NonHook | Hook | NonHook | NonHook |
| turn_error | -- | NonHook | -- | NonHook | NonHook | Hook | NonHook | NonHook |
| subagent_start | Hook | -- | -- | NonHook | NonHook | -- | -- | NonHook |
| subagent_stop | Hook | -- | -- | NonHook | NonHook | -- | -- | NonHook |
| before_model | -- | -- | Hook | -- | -- | Hook | -- | NonHook |
| after_model | -- | NonHook | Hook | NonHook | NonHook | Hook | NonHook | NonHook |
| before_compact | Hook | -- | Hook | -- | NonHook | Hook | -- | -- |
| notification | Hook | NonHook | Hook | NonHook | NonHook | Hook | NonHook | NonHook |

**Legend:**
- **Hook** -- Event can be registered via config-file modification (settings.json hooks, opencode.json plugins, config.toml notify).
- **NonHook** -- Event requires alternative capture (wrapper script, wire-mode JSON-RPC proxy, JSONL stream parsing).
- **--** -- Not supported by this provider.

## Key Types and Structures

### `AgenticEvent` enum

**File:** `agentic_event.rs`

Normalized event names across all 8 supported agentic CLI providers. Each variant
represents a lifecycle moment that at least 2 providers expose. Provider adapters
map their native events to the appropriate variant.

**Derives:** `Debug`, `Clone`, `Copy`, `PartialEq`, `Eq`, `Hash`, `Serialize`, `Deserialize`

**Attributes:** `#[serde(rename_all = "snake_case")]`, `#[non_exhaustive]`

**Variants (16):**

| Variant | Slug | Description |
|---|---|---|
| `SessionStart` | `session_start` | Agent session started, resumed, or cleared |
| `SessionEnd` | `session_end` | Agent session ended or terminated |
| `BeforePrompt` | `before_prompt` | User prompt submitted, before agent processes it |
| `BeforeTool` | `before_tool` | Tool call created, before execution begins |
| `AfterTool` | `after_tool` | Tool call completed successfully |
| `ToolError` | `tool_error` | Tool call failed |
| `PermissionRequest` | `permission_request` | Agent is requesting user permission |
| `HumanInTheLoop` | `human_in_the_loop` | Agent is asking user a clarifying question |
| `TurnComplete` | `turn_complete` | Agent turn (request/response cycle) completed |
| `TurnError` | `turn_error` | Agent turn failed with an error |
| `SubagentStart` | `subagent_start` | Sub-agent spawned |
| `SubagentStop` | `subagent_stop` | Sub-agent finished |
| `BeforeModel` | `before_model` | Before sending prompt to the model |
| `AfterModel` | `after_model` | After receiving response from the model |
| `BeforeCompact` | `before_compact` | Before context compaction/summarization |
| `Notification` | `notification` | Provider-specific notification |

**Key methods:**

- `ALL: [AgenticEvent; 16]` -- Constant array of all variants in display order.
- `from_slug(name: &str) -> Option<Self>` -- Parse a canonical `snake_case` event name.
- `parse_name_or_alias(input: &str) -> Option<Self>` -- Parse from canonical name _or_ any provider-native alias. Case-insensitive, tolerant of `-`/`_`/camelCase separators. Tries canonical first, then shared native mappings, then all providers' native names.
- `abbrev(&self) -> &'static str` -- Short emoji abbreviation for table column headers.
- `description(&self) -> &'static str` -- Human-readable description.
- `response_schema(&self) -> &'static str` -- Describes fields available in the event payload.
- `return_schema(&self) -> &'static str` -- Describes what a hook can return to influence agent behavior.

### `EventMeta` struct

**File:** `event_meta.rs`

Normalized metadata attached to every fired event. Provider adapters populate this
from their native event payloads. The `extra` map carries provider-specific fields
that do not fit the common schema.

**Fields:**

| Field | Type | Description |
|---|---|---|
| `provider` | `Provider` | Which agent provider fired the event |
| `event` | `AgenticEvent` | The shared event that was matched |
| `timestamp` | `DateTime<Utc>` | UTC timestamp of when the event was received |
| `session_id` | `Option<String>` | Session or thread identifier |
| `cwd` | `Option<String>` | Current working directory at event time |
| `tool_name` | `Option<String>` | Tool name (for tool-related events) |
| `tool_input` | `Option<Value>` | Tool input/arguments |
| `tool_response` | `Option<Value>` | Tool output/response (post-tool events) |
| `error` | `Option<String>` | Error message (failure events) |
| `prompt` | `Option<String>` | User's prompt text (prompt-related events) |
| `agent_type` | `Option<String>` | Agent/subagent type or identifier |
| `notification_type` | `Option<String>` | Notification type string |
| `notification_message` | `Option<String>` | Notification message text |
| `extra` | `HashMap<String, Value>` | Provider-specific fields |
| `env` | `EnvironmentContext` | Snapshot of host and repository environment |

**Key methods:**

- `dummy_with_env(env: EnvironmentContext) -> Self` -- Create a minimal `EventMeta` with only environment context populated, useful for resolving context variables without a real event.

### `Provider` enum

**File:** `provider.rs`

Supported agentic CLI providers. Serialized as `snake_case`.

**Derives:** `Debug`, `Clone`, `Copy`, `PartialEq`, `Eq`, `Hash`, `Serialize`, `Deserialize`

**Attributes:** `#[serde(rename_all = "snake_case")]`, `#[non_exhaustive]`

**Variants (8):**

| Variant | Slug | Display Name |
|---|---|---|
| `Claude` | `claude` | Claude |
| `Codex` | `codex` | Codex |
| `Gemini` | `gemini` | Gemini |
| `Goose` | `goose` | Goose |
| `KimiCode` | `kimi_code` | Kimi Code |
| `OpenCode` | `open_code` | OpenCode |
| `QwenCode` | `qwen_code` | Qwen Code |
| `RooCode` | `roo_code` | Roo Code |

**Key methods:**

- `as_slug(&self) -> &'static str` -- Stable snake_case identifier for file paths and config keys.
- `cli_aliases(&self) -> &'static [&'static str]` -- Common CLI aliases accepted for this provider.
- `parse_cli_name(input: &str) -> Option<Self>` -- Parse from a CLI-facing name or alias (case-insensitive, separator-tolerant).
- `fuzzy_match_cli_name(input: &str) -> Option<Self>` -- Fuzzy match via exact, prefix, then contains strategies.
- `detect_from_payload(raw: &Value) -> Option<Self>` -- Detect provider from raw JSON payload shape (checks for `hook_event_name`, `type`+`thread_id`, `event_type`, `event_name`, `method`).
- `event_support_level(&self, event: &AgenticEvent) -> EventSupportLevel` -- Returns Hook, NonHook, or NotSupported for each event.
- `supports_event(&self, event: &AgenticEvent) -> bool` -- Whether the provider supports the event via any method.
- `supports_event_via_hook(&self, event: &AgenticEvent) -> bool` -- Whether the provider supports the event via native hooks (config-file based).
- `native_event_name(&self, event: &AgenticEvent) -> Option<&'static str>` -- Returns the provider-native event name. `None` if unsupported, `Some("")` if supported but no specific name.
- `supports_skills(&self) -> bool` -- Whether the provider supports skill discovery (Claude, Codex, Gemini, OpenCode, QwenCode, RooCode).
- `docs_url(&self) -> &'static str` -- Documentation URL for the provider.
- `sniff_ai_cli(&self) -> AiCli` -- Corresponding `sniff::programs::AiCli` variant for install detection.

**Constants:**

- `PROVIDERS_DISPLAY_ORDER: [Provider; 8]` -- Canonical display order for matrix-style reporting.

### `EventSupportLevel` enum

**File:** `provider.rs`

Describes the level of support a provider has for a given event.

**Variants:**

| Variant | Meaning |
|---|---|
| `Hook` | Event is supported via native hooks (config-file based). Claudine can register handlers by modifying the provider's config file. |
| `NonHook` | Event is supported via non-hook methods (wrapper scripts, wire-mode proxy, JSONL stream parsing). Not yet fully implemented. |
| `NotSupported` | Event is not available from this provider. |

**Key methods:**

- `is_supported(&self) -> bool` -- Returns `true` for `Hook` or `NonHook`.
- `is_hook(&self) -> bool` -- Returns `true` only for `Hook`.

### `ResolvedHook` struct

**File:** `resolved_hook.rs`

A resolved hook binding ready for execution. This is the output of matching an
incoming event against the user's configuration -- it bundles together the normalized
event, the full metadata, and the list of actions to execute.

**Fields:**

| Field | Type | Description |
|---|---|---|
| `event` | `AgenticEvent` | The normalized event that fired |
| `meta` | `EventMeta` | Normalized metadata extracted from the native payload |
| `provider` | `Provider` | The provider that originated this event |
| `actions` | `Vec<HookAction>` | Actions to execute in declaration order |
| `can_block` | `bool` | Whether this hook's event supports blocking the originating CLI |

### `EnvironmentContext` struct

**File:** `environment.rs`

Host and repository environment snapshot. Detected once at session start via
`sniff::detect_with_config` and cached for the session lifetime. Attached to
every `EventMeta`.

**Fields:**

| Field | Type | Description |
|---|---|---|
| `os` | `OsContext` | OS family, name, version, kernel, hostname, linux family, package managers |
| `hardware` | `HardwareContext` | CPU arch, brand, cores, total/available memory |
| `git` | `Option<GitContext>` | Git repo state: root, branch, dirty status, staged/unstaged/untracked counts, HEAD commit, user info, remote info, hosting provider, org/repo names |
| `repo` | `Option<RepoContext>` | Monorepo detection: tool, root path, package names |
| `primary_language` | `Option<String>` | Primary programming language detected in the project |

**Key function:**

- `detect_environment(cwd: &Path) -> EnvironmentContext` -- Runs `sniff` with a fast configuration (no network, single commit, no deep inspection) to populate the context.

### `HookerConfig` and related config types

**File:** `config.rs`

Root configuration loaded from `~/.claudine/config.json`. Organized per-provider,
each with its own set of event bindings.

- **`HookerConfig`** -- Root: `version`, `settings` (GlobalSettings), `providers` (HashMap<Provider, ProviderConfig>).
- **`ProviderConfig`** -- Contains `events: HashMap<AgenticEvent, EventBinding>`.
- **`EventBinding`** -- An event's configuration: `enabled` (bool, default true), `actions` (Vec<HookAction>), `matcher` (optional regex filter on tool name, notification type, etc.).
- **`GlobalSettings`** -- `default_log_target`, `tts` (TtsSettings), `linking` (LinkingSettings).
- **`TtsSettings`** -- TTS provider, voice, and rate forwarded to biscuit-speaks.
- **`LinkingSettings`** -- Provider preference ordering and canonical provider slots for skill/command/agent/script linking.

### Matrix types

**File:** `matrix.rs`

Structured types for generating support and mapping matrices used by CLI reporting:

- **`EventSupportCell`** -- A (provider, support level) pair.
- **`EventSupportRow`** -- An event plus a vector of support cells.
- **`EventNativeMappingCell`** -- A (provider, native event name) pair.
- **`EventNativeMappingRow`** -- An event plus a vector of mapping cells.
- **`NativeEventName`** -- Enum: `Unsupported`, `NoSpecificName`, `Named(&'static str)`.

**Key functions:**

- `event_support_matrix(providers: &[Provider]) -> Vec<EventSupportRow>` -- Build the full support matrix.
- `event_native_mapping_matrix(providers: &[Provider]) -> Vec<EventNativeMappingRow>` -- Build the native-name mapping matrix.

### `SharedNativeEventMapping` struct

**File:** `provider.rs` (crate-internal)

Deduplicates the mapping between canonical events and provider-native event names.
Used by both configurators (to register hooks) and adapters (to parse incoming events).

**Fields:** `event: AgenticEvent`, `native_name: &'static str`, `parse_aliases: &'static [&'static str]`

Currently defined for Claude, Gemini, and OpenCode (the three providers with hook-based support that use shared mappings).

### `ProviderAdapter` trait

**File:** `adapters/mod.rs`

Trait for provider-specific event adapters. One static singleton per provider.

**Methods:**

| Method | Signature | Description |
|---|---|---|
| `provider()` | `-> Provider` | Which provider this adapter handles |
| `parse_event()` | `(&self, raw: &Value) -> Result<(AgenticEvent, EventMeta), AdapterError>` | Parse raw provider JSON into normalized event + metadata |
| `can_block()` | `(&self, event: &AgenticEvent) -> bool` | Whether this provider/event pair supports blocking response semantics |
| `format_response()` | `(&self, event: &AgenticEvent, response: &HookResponse) -> Result<Value, AdapterError>` | Convert unified hook response into provider-native response payload |
| `exit_code()` | `(&self, event: &AgenticEvent, response: &HookResponse) -> Option<i32>` | Exit code for shell-driven providers |

**Factory:** `adapter_for(provider: Provider) -> &'static dyn ProviderAdapter`

### `HookAction` enum

**File:** `actions/hook_action.rs`

Actions that can be attached to an event binding. Tagged union serialized as `{ "type": "..." }`.

**Variants:**

| Variant | Fields | Description |
|---|---|---|
| `SoundEffect` | `name`, `volume`, `speed` | Play an embedded sound effect from playa |
| `Speak` | `message` | Speak a message aloud using biscuit-speaks TTS |
| `Log` | `target` | Write the event to a configured log target |
| `FireAndForget` | `command`, `args` | Execute a command asynchronously without waiting |
| `Call` | `command`, `args`, `timeout_ms`, `mapper` | Execute a command synchronously, map output to hook response |
| `Report` | `handler` | Report the event into the agent's output stream |

### Init defaults

**File:** `init_defaults.rs`

Constants and functions supporting the `claudine init` wizard:

- `INIT_EVENT_DISPLAY_ORDER` -- All 16 events in UI display order.
- `INIT_RECOMMENDED_EVENTS` -- 4 pre-selected events: SessionStart, TurnComplete, ToolError, PermissionRequest.
- `INIT_TTS_PROVIDERS` -- TTS options: macOS Say, eSpeak, ElevenLabs, Kokoro.
- `recommended_sound(event) -> &str` -- Default sound effect for each event.
- `default_speak_template(event) -> &str` -- Default TTS message template (supports `{{tool_name}}` etc.).
- `quick_start_supported_providers() -> Vec<Provider>` -- Providers that support at least one recommended event via hooks (Claude, Codex, Gemini, OpenCode).

## How It Works

The unified hook system translates provider-native event payloads into a canonical
event model, matches them against user configuration, and dispatches actions. The
flow proceeds through these stages:

### 1. Provider Detection

When an event payload arrives (as raw JSON), the system first determines which
provider sent it. `Provider::detect_from_payload()` inspects the JSON shape for
provider-specific marker fields:

- `hook_event_name` present --> Claude
- `type` + `thread_id` present --> Codex
- `event_type` present --> OpenCode
- `event_name` present --> Gemini
- `method` present --> Kimi Code

For providers that register via config-file hooks (Claude, Gemini, OpenCode, Codex),
the payload arrives because claudine was registered as the hook handler during
`claudine init` or `claudine register`.

### 2. Event Parsing and Normalization

Once the provider is identified, the corresponding `ProviderAdapter` singleton is
retrieved via `adapter_for(provider)`. Each adapter implements `parse_event()`, which:

1. Extracts the provider-native event name from the raw JSON.
2. Maps it to a canonical `AgenticEvent` variant using `SharedNativeEventMapping`
   tables (for Claude, Gemini, OpenCode) or provider-specific match logic.
3. Populates an `EventMeta` struct with normalized fields extracted from the payload
   (session_id, tool_name, tool_input, error, prompt, etc.).
4. Stashes any unrecognized fields into `EventMeta.extra`.

The `SharedNativeEventMapping` tables serve as a single source of truth used by both
the adapter parse logic (`native_name -> AgenticEvent`) and the configurator
registration logic (`AgenticEvent -> native_name`), preventing drift between the two.

### 3. Environment Enrichment

At session start, `detect_environment(cwd)` runs `sniff` to capture a snapshot of the
host environment (OS, hardware, git state, repo structure, primary language). This
`EnvironmentContext` is attached to every `EventMeta.env` for the session lifetime,
making it available to templates and action handlers without repeated detection.

### 4. Configuration Matching

The `HookerConfig` (loaded from `~/.claudine/config.json`) is consulted to find the
matching `EventBinding` for the (provider, event) pair:

1. Look up `config.providers[provider].events[event]`.
2. If found and `enabled == true`, check the optional `matcher` regex against the
   relevant event field (tool_name for tool events, notification_type for notifications).
3. If the matcher passes (or is absent), the binding's `actions` list is used.

### 5. Hook Resolution

The matched binding is assembled into a `ResolvedHook`:

- `event`: The canonical `AgenticEvent`.
- `meta`: The fully populated `EventMeta`.
- `provider`: The originating provider.
- `actions`: The `Vec<HookAction>` from the `EventBinding`.
- `can_block`: Determined by querying `adapter.can_block(event)` -- whether the
  provider supports synchronous response semantics for this event (e.g., Claude's
  `PreToolUse` can return allow/deny, while `Stop` cannot).

### 6. Action Dispatch

Each `HookAction` in the resolved hook is executed in declaration order:

- **SoundEffect** -- Plays an embedded audio clip via playa.
- **Speak** -- Renders a Handlebars-style template against `EventMeta` fields and
  speaks it via biscuit-speaks TTS.
- **Log** -- Serializes the event metadata to a configured log target (file, stdout).
- **FireAndForget** -- Spawns a command asynchronously without waiting.
- **Call** -- Executes a command synchronously and optionally maps its output to a
  hook response (for blocking events).
- **Report** -- Writes event information into the agent's output stream.

### 7. Response Formatting

For blocking events (where `can_block == true`), if a `Call` action produces a
response, it is converted back to the provider-native format via
`adapter.format_response(event, response)`. The adapter also determines the
appropriate exit code via `adapter.exit_code(event, response)` for shell-driven
providers like Claude (where exit code 0 = allow, exit code 2 = deny for
`PreToolUse`).

### Support Level Implications

The three-tier `EventSupportLevel` system determines what claudine can do:

- **Hook providers** (Claude, Gemini, OpenCode, and Codex for `turn_complete`): Full
  support. Claudine registers itself in the provider's config file, receives events
  as JSON payloads, and can return blocking responses.
- **NonHook providers** (Goose, Kimi Code, Qwen Code, Roo Code, and most Codex events):
  Events are available but require alternative capture methods (wrapper scripts, wire-mode
  proxy, stream parsing) that are tracked but not yet fully implemented.
- **NotSupported**: The event concept does not exist in the provider's architecture.
