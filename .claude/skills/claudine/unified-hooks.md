# Unified Hooks: Events, Provider Surfaces, and Canonical Actions

## Contents

- Current Model: "Hooks" vs "Actions"
- Event Mapping to Providers
- Event Support by Provider
- Key Types and Structures
- Current Configuration and Runtime Types
- Matrix Types
- ProviderAdapter trait
- HookAction enum
- CLI Surfaces
- How It Works
- Hook Handler Deadlines
- Support Level Implications

Use heading search to jump to the listed subsystem.


Reference for Claudine's unified event model, covering canonical event names,
provider-native mappings, support levels, and the current dispatch pipeline.

## Current Model: "Hooks" vs "Actions"

After the configuration refactor, these terms mean different things:

- **Hooks** are the provider-side ingress surfaces Claudine uses to receive lifecycle events.
  Depending on the provider, that may mean native config-file hooks, a notify command,
  stream parsing, wire/RPC traffic, or wrapper-managed capture.
- **Actions** are the user-configured responses Claudine executes when a canonical
  `AgenticEvent` fires. These live in `ClaudineConfig.actions`, keyed only by canonical
  event, not by provider.

The old per-provider config model built around `HookerConfig`, `ProviderConfig`, and a
provider-scoped `events` map has been removed. The current persisted config is centered on:

- `ClaudineConfig` for user scope (`~/.claudine/config.json`)
- `RepoOverrideConfig` for repo scope (`<repo>/.claudine/config.json`)
- `actions: HashMap<AgenticEvent, Vec<HookAction>>` as the canonical event-to-action map

Important implications:

- `claudine hooks` is about provider support and registration state.
- `claudine actions` is about which canonical events have configured actions.
- Logging and Protect are no longer modeled as hook actions. They are global services
  applied by the dispatch pipeline.

## Event Mapping to Providers

Each provider uses its own native event names. Claudine normalizes them into a single
`AgenticEvent` enum. The tables below show how each canonical event maps to
provider-native identifiers.

```txt
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


Table 2: Kimi Code, OpenCode, Qwen Code

Event              Kimi Code       OpenCode             Qwen Code
-----------------------------------------------------------------
session_start                      session.created
session_end                        session.deleted
before_prompt      TurnBegin       chat.message
before_tool        ToolCall        tool.execute.before
after_tool         ToolResult      tool.execute.after
tool_error         ToolResult
permission_request ApprovalRequest permission.ask       CanUseTool
human_in_the_loop  ApprovalRequest permission.asked
turn_complete      TurnEnd         session.idle         result
turn_error         prompt.status   session.error        result
subagent_start     SubagentEvent
subagent_stop      SubagentEvent
before_model                       chat.params
after_model        ContentPart     message.part.updated assistant
before_compact     CompactionBegin session.compacted
notification       StatusUpdate    tui.toast.show       system

Blank = not supported or no specific native name.
```

## Event Support by Provider

Support level indicates how Claudine can capture an event from a provider.

| Event | Claude | Codex | Gemini | Goose | Kimi Code | OpenCode | Qwen Code |
|---|---|---|---|---|---|---|---|
| session_start | Hook | NonHook | Hook | -- | -- | Hook | -- |
| session_end | Hook | -- | Hook | -- | -- | Hook | -- |
| before_prompt | Hook | NonHook | Hook | -- | NonHook | Hook | -- |
| before_tool | Hook | NonHook | Hook | -- | NonHook | Hook | -- |
| after_tool | Hook | NonHook | Hook | -- | NonHook | Hook | -- |
| tool_error | Hook | NonHook | -- | -- | NonHook | -- | -- |
| permission_request | Hook | -- | -- | -- | NonHook | Hook | NonHook |
| human_in_the_loop | Hook | NonHook | -- | NonHook | NonHook | Hook | -- |
| turn_complete | Hook | Hook | Hook | NonHook | NonHook | Hook | NonHook |
| turn_error | -- | NonHook | -- | NonHook | NonHook | Hook | NonHook |
| subagent_start | Hook | -- | -- | NonHook | NonHook | -- | -- |
| subagent_stop | Hook | -- | -- | NonHook | NonHook | -- | -- |
| before_model | -- | -- | Hook | -- | -- | Hook | -- |
| after_model | -- | NonHook | Hook | NonHook | NonHook | Hook | NonHook |
| before_compact | Hook | -- | Hook | -- | NonHook | Hook | -- |
| notification | Hook | NonHook | Hook | NonHook | NonHook | Hook | NonHook |

**Legend:**

- **Hook**: provider has a native registration surface Claudine can install into.
- **NonHook**: event is observable through wrappers, streams, RPC, SDK callbacks, or other
  non-config registration surfaces.
- **--**: not supported by this provider.

## Key Types and Structures

### `AgenticEvent` enum

**File:** `claudine/lib/src/events/agentic_event.rs`

Normalized event names across all supported providers. Each variant represents a
lifecycle moment Claudine can observe or react to.

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
| `HumanInTheLoop` | `human_in_the_loop` | Agent is asking the user for input or clarification |
| `TurnComplete` | `turn_complete` | Agent turn completed |
| `TurnError` | `turn_error` | Agent turn failed |
| `SubagentStart` | `subagent_start` | Sub-agent spawned |
| `SubagentStop` | `subagent_stop` | Sub-agent finished |
| `BeforeModel` | `before_model` | Before sending prompt to the model |
| `AfterModel` | `after_model` | After receiving response from the model |
| `BeforeCompact` | `before_compact` | Before context compaction or summarization |
| `Notification` | `notification` | Provider-specific notification |

**Key methods:**

- `ALL: [AgenticEvent; 16]` returns all variants in display order.
- `from_slug(name: &str) -> Option<Self>` parses a canonical `snake_case` event name.
- `parse_name_or_alias(input: &str) -> Option<Self>` parses canonical names and provider-native aliases.
- `description(&self) -> &'static str` returns human-readable descriptions.
- `response_schema(&self) -> &'static str` describes payload fields available to actions.
- `return_schema(&self) -> &'static str` describes what a blocking response can return.

### `EventMeta` struct

**File:** `claudine/lib/src/events/event_meta.rs`

Normalized metadata attached to every incoming event. Provider adapters populate this
from native payloads; provider-specific fields that do not fit the shared schema are
stored in `extra`.

**Fields:**

| Field | Type | Description |
|---|---|---|
| `provider` | `Provider` | Which provider fired the event |
| `event` | `AgenticEvent` | Shared canonical event |
| `timestamp` | `DateTime<Utc>` | UTC timestamp of when the event was received |
| `session_id` | `Option<String>` | Session or thread identifier |
| `cwd` | `Option<String>` | Current working directory at event time |
| `tool_name` | `Option<String>` | Tool name for tool-related events |
| `tool_input` | `Option<Value>` | Tool input or arguments |
| `tool_response` | `Option<Value>` | Tool output or response |
| `error` | `Option<String>` | Error text |
| `prompt` | `Option<String>` | Prompt text |
| `agent_type` | `Option<String>` | Agent or subagent type |
| `notification_type` | `Option<String>` | Notification type string |
| `notification_message` | `Option<String>` | Notification body |
| `extra` | `HashMap<String, Value>` | Provider-specific fields |
| `env` | `EnvironmentContext` | Snapshot of host and repo environment |

**Key method:**

- `dummy_with_env(env: EnvironmentContext) -> Self` creates a minimal metadata value with environment attached.

### `Provider` enum

**File:** `claudine/lib/src/events/provider.rs`

Supported agentic CLI providers. Serialized as `snake_case`.

**Variants (7):**

| Variant | Slug | Display Name |
|---|---|---|
| `Claude` | `claude` | Claude |
| `Codex` | `codex` | Codex |
| `Gemini` | `gemini` | Gemini |
| `Goose` | `goose` | Goose |
| `KimiCode` | `kimi` | Kimi Code |
| `OpenCode` | `opencode` | OpenCode |
| `QwenCode` | `qwen` | Qwen Code |

**Key methods:**

- `as_slug(&self) -> &'static str` returns the stable identifier used in CLI/config paths.
- `parse_cli_name(input: &str) -> Option<Self>` parses user-facing names or aliases.
- `fuzzy_match_cli_name(input: &str) -> Option<Self>` resolves exact, prefix, then contains matches.
- `detect_from_payload(raw: &Value) -> Option<Self>` detects provider from raw payload shape.
- `event_support_level(&self, event: &AgenticEvent) -> EventSupportLevel` returns Hook, NonHook, or NotSupported.
- `supports_event_via_hook(&self, event: &AgenticEvent) -> bool` indicates whether native hook registration exists.
- `native_event_name(&self, event: &AgenticEvent) -> Option<&'static str>` returns the provider-native name when one exists.

### `EventSupportLevel` enum

**File:** `claudine/lib/src/events/provider.rs`

Describes the capture surface Claudine has for a provider/event pair.

| Variant | Meaning |
|---|---|
| `Hook` | Supported through native provider registration |
| `NonHook` | Supported through a wrapper, stream, RPC, SDK callback, or similar surface |
| `NotSupported` | Event concept is unavailable on that provider |

### `ResolvedHook` struct

**File:** `claudine/lib/src/events/resolved_hook.rs`

A resolved canonical event ready for action execution.

**Fields:**

| Field | Type | Description |
|---|---|---|
| `event` | `AgenticEvent` | Canonical event that fired |
| `meta` | `EventMeta` | Normalized metadata |
| `provider` | `Provider` | Provider that originated the event |
| `actions` | `Vec<HookAction>` | Actions to execute in declaration order |
| `can_block` | `bool` | Whether the originating provider/event supports a blocking response |

### `EnvironmentContext` struct

**File:** `claudine/lib/src/events/environment.rs`

Host and repository environment snapshot, detected once and attached to each `EventMeta`.

**Fields:**

| Field | Type | Description |
|---|---|---|
| `os` | `OsContext` | OS family, version, kernel, hostname, package managers |
| `hardware` | `HardwareContext` | CPU architecture, model, cores, memory |
| `git` | `Option<GitContext>` | Git root, branch, dirty state, commit, remote, owner/repo |
| `repo` | `Option<RepoContext>` | Monorepo detection and root/package information |
| `primary_language` | `Option<String>` | Primary language detected in the project |

**Key function:**

- `detect_environment(cwd: &Path) -> EnvironmentContext` builds a full environment snapshot.
- `detect_environment_fast(cwd: &Path) -> EnvironmentContext` is the lighter-weight path used by `claudine handle`.

## Current Configuration and Runtime Types

### `ClaudineConfig`

**File:** `claudine/lib/src/config/claudine_config.rs`

User-scope Claudine configuration. This is now the primary persisted config model.

**Relevant fields:**

| Field | Type | Description |
|---|---|---|
| `tts` | `TtsValue` | Global TTS defaults or explicit provider/voice config |
| `messenger` | `Option<ClaudineMessengerConfig>` | Messaging route configuration for `Message` actions |
| `logging` | `bool` | Enables global JSONL logging for handled events |
| `protect` | `ProtectConfig` | Enables and configures Protect |
| `actions` | `HashMap<AgenticEvent, Vec<HookAction>>` | Canonical event-to-action map |
| `preferred_agent` | `Provider` | Default provider for lazy composition workflows |
| `canonical_provider` | `Option<Provider>` | Scope-level canonical provider override |
| `default_sounds` | `DefaultSounds` | Outcome sounds used by the dispatch pipeline |

Notes:

- Actions are keyed by canonical event only. They are not nested under providers.
- A missing event key means "no configured actions for that event."
- Logging and Protect are independent of the `actions` map.

### `RepoOverrideConfig`

**File:** `claudine/lib/src/config/claudine_config.rs`

Repo-scope override model loaded from `<repo>/.claudine/config.json`.

**Relevant fields:**

| Field | Type | Description |
|---|---|---|
| `canonical_provider` | `Option<Provider>` | Repo override for canonical provider |
| `actions` | `HashMap<AgenticEvent, Vec<HookAction>>` | Per-event replacement of user actions |
| `active_messenger` | `Option<Option<String>>` | Repo override for active messenger config |

Merge rule for actions:

- If the repo config defines actions for an event, that vector fully replaces the user-scope vector for the same event.

### `CanonicalRuntimeConfig`

**File:** `claudine/lib/src/dispatch/loader.rs`

Compiled runtime form of `ClaudineConfig`.

**Fields:**

| Field | Type | Description |
|---|---|---|
| `config` | `ClaudineConfig` | Effective merged config |
| `messaging` | `RuntimeMessagingSettings` | Bridged runtime messaging settings |
| `protect_service` | `Option<ProtectService>` | Cached Protect service instance |
| `events` | `HashMap<AgenticEvent, RuntimeEventBinding>` | Compiled event bindings keyed by canonical event |

### `RuntimeEventBinding`

**File:** `claudine/lib/src/dispatch/loader.rs`

Dispatch-ready action binding for a canonical event.

**Fields:**

| Field | Type | Description |
|---|---|---|
| `enabled` | `bool` | Whether the binding is active |
| `actions` | `Vec<HookAction>` | Actions to execute |
| `matcher` | `Option<Regex>` | Optional runtime regex filter |
| `compiled_mappers` | `Vec<Option<CompiledMapper>>` | Precompiled mapper metadata for `Call` actions |

Current behavior:

- `compile_canonical_runtime()` builds these bindings from `ClaudineConfig.actions`.
- The current canonical config schema does not expose per-event matchers, so compiled bindings are created with `matcher: None`.
- Mapper compilation is an internal runtime optimization for `Call` actions.

### Legacy bridge types that still exist

**File:** `claudine/lib/src/events/config.rs`

`GlobalSettings`, `TtsSettings`, `LinkingSettings`, `CanonicalProviderSettings`, and `EventBinding`
still exist in the codebase as supporting or bridge types. They are not the primary persisted
"hooks config" model anymore and should not be used to describe current user configuration.

## Matrix Types

**File:** `claudine/lib/src/events/matrix.rs`

Used by `claudine hooks --support` and `claudine hooks --mapping`.

- `EventSupportCell`
- `EventSupportRow`
- `EventNativeMappingCell`
- `EventNativeMappingRow`
- `NativeEventName`

Key functions:

- `event_support_matrix(providers: &[Provider]) -> Vec<EventSupportRow>`
- `event_native_mapping_matrix(providers: &[Provider]) -> Vec<EventNativeMappingRow>`

## `ProviderAdapter` trait

**File:** `claudine/lib/src/adapters/mod.rs`

Provider-specific adapter interface used to normalize native payloads and format blocking responses.

**Methods:**

| Method | Signature | Description |
|---|---|---|
| `provider()` | `-> Provider` | Which provider the adapter handles |
| `parse_event()` | `(&self, raw: &Value) -> Result<(AgenticEvent, EventMeta), AdapterError>` | Parse native JSON into canonical event + metadata |
| `can_block()` | `(&self, event: &AgenticEvent) -> bool` | Whether this event supports a blocking response on this provider |
| `format_response()` | `(&self, event: &AgenticEvent, response: &HookResponse) -> Result<Value, AdapterError>` | Convert canonical response back to provider-native JSON |
| `exit_code()` | `(&self, event: &AgenticEvent, response: &HookResponse) -> Option<i32>` | Exit code for shell-driven providers |

Factory:

- `adapter_for(provider: Provider) -> &'static dyn ProviderAdapter`

## `HookAction` enum

**File:** `claudine/lib/src/actions/hook_action.rs`

Actions attached to canonical events in `ClaudineConfig.actions`.

**Current variants:**

| Variant | Fields | Description |
|---|---|---|
| `SoundEffect` | `effect`, `volume`, `speed` | Play an embedded sound effect via `playa` |
| `Speak` | `message`, `voice`, `gender` | Speak a templated message via biscuit-speaks |
| `Bash` | `command`, `params` | Spawn a shell command asynchronously |
| `Call` | `command`, `args`, `timeout_ms`, `mapper` | Run a command synchronously and optionally produce a `HookResponse` |
| `Report` | `handler` | Emit structured human-readable output into the agent stream |
| `Message` | `message`, `image` | Send a message through the configured messenger route |

Important changes versus the pre-refactor model:

- There is no `HookAction::Log`. Event logging is controlled by `ClaudineConfig.logging`.
- There is no `HookAction::FireAndForget`. The current async shell action is `Bash`.
- `Message` is now a first-class action.

## CLI Surfaces

The current CLI splits hook registration from action configuration:

- `claudine hooks` shows provider hook support, native event mappings, and registration state.
- `claudine actions` shows which canonical events have configured actions.
- `claudine sync` re-applies native provider registrations where supported.
- `claudine uninstall` removes registered provider hooks.
- `claudine handle [event] [--provider]` is the hidden ingress command used by registered hooks and wrappers.

## How It Works

### 1. Event ingress

An event reaches Claudine through one of these surfaces:

- native provider hook registration
- notify-style hook commands
- wrapper-managed structured stream parsing
- wire/RPC or SDK callback surfaces

`claudine handle` reads the raw JSON payload from stdin. The provider is resolved in this order:

1. explicit `--provider`
2. wrapper environment hint
3. `Provider::detect_from_payload(raw)`

### 2. Event normalization

`adapter_for(provider).parse_event(raw)` converts the provider-native payload into:

- a canonical `AgenticEvent`
- an `EventMeta` populated with normalized fields and provider-specific `extra` data

### 3. Environment enrichment

`detect_environment_fast()` builds the environment snapshot attached to the event metadata.
This makes host, git, and repo context available to action templates.

### 4. Config load and merge

Dispatch loads the effective config by:

1. reading user config into `ClaudineConfig`
2. reading repo config into `RepoOverrideConfig` when present
3. applying repo overrides, including per-event action replacement

There is no provider-scoped hook lookup anymore. The canonical action map is global.

### 5. Runtime compilation

`compile_canonical_runtime()` converts the merged config into `CanonicalRuntimeConfig`:

- builds one `RuntimeEventBinding` per configured canonical event
- precompiles `Call` mappers
- constructs the runtime messenger bridge
- constructs `ProtectService` when enabled

### 6. Canonical binding lookup

`dispatch_canonical_with_runtime()` looks up the event by canonical event only:

1. `runtime.get_binding(&event)`
2. if a binding exists and is enabled, execute its actions in declaration order
3. if no binding exists, dispatch still continues with logging/protect/default-sound behavior

When a binding is executed, Claudine materializes a `ResolvedHook` with:

- the canonical event
- normalized metadata
- originating provider
- ordered actions
- whether the provider/event supports blocking

### 7. Action dispatch and cross-cutting services

`runner::execute_actions()` runs configured actions in order.

Separately from configured actions:

- global JSONL logging runs when `config.logging` is true
- Protect runs both before and after selected events
- default sounds may still play based on outcome

This means "no configured actions" does not imply "nothing happens."

### 8. Response formatting

If the provider/event supports blocking and a `Call` action returns a response:

- the adapter formats the canonical `HookResponse` into the provider-native payload
- shell-based providers may also receive an exit code

On non-blocking events, `Call` can still run, but any response it produces is informational only.

## Hook Handler Deadlines

To prevent hook handlers from blocking the parent agent session indefinitely (e.g., during a 30s hang), `claudine handle` enforces a hard execution deadline.

- **Global Deadline:** 5 seconds (default), overridable via `CLAUDINE_HANDLE_DEADLINE_SECONDS`.
- **Exit Code:** Exits `124` when the deadline is exceeded.
- **Action Timeouts:** `Bash` and `Message` actions have a tighter **3s timeout** when running inside `claudine handle`.
- **Tracing:** Phase-level spans (`handle_stdin_read`, `handle_dispatch_canonical`, `load_config`, `run_bindings`, etc.) ensure that any hang can be diagnosed via `RUST_LOG=claudine=debug`.

## Support Level Implications

- **Hook providers** have a native registration surface Claudine can install into. This is what `claudine sync` manages.
- **NonHook providers** can still participate in the canonical event model, but their events arrive through wrappers, streams, wire protocols, or SDK callbacks rather than config-file hook registration.
- **NotSupported** means the provider does not expose the lifecycle moment at all, so Claudine cannot synthesize a canonical event for it.
