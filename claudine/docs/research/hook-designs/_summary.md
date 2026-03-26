# Condensed Design Summary -- All 8 CLI Event Systems

This document provides a compact summary of all 8 agentic CLI event designs for use by the finalization agent.

---

## 1. Claude Code

**Enum name**: `ClaudeCodeEvent`
**Variant count**: 14

| Variant | Can Block | Matcher Field | Payload Type | Response Pattern | Closest AgenticEvent |
|---------|-----------|---------------|--------------|-----------------|---------------------|
| SessionStart | No | source | SessionStartPayload | ContextInjection | SessionStart |
| SessionEnd | No | reason | SessionEndPayload | CleanupOnly | SessionEnd |
| UserPromptSubmit | Yes | -- | UserPromptSubmitPayload | PromptDecision (top-level decision:"block") | BeforePrompt |
| PreToolUse | Yes | tool_name | PreToolUsePayload | PermissionDecision (allow/deny/ask 3-level) | BeforeTool |
| PostToolUse | Yes | tool_name | PostToolUsePayload | PostToolFeedback (advisory block) | AfterTool |
| PostToolUseFailure | No | tool_name | PostToolUseFailurePayload | ContextOnly | ToolError |
| PermissionRequest | Yes | tool_name | PermissionRequestPayload | PermissionBehavior (allow/deny + updatedInput/Permissions) | PermissionRequest |
| Notification | No | notification_type | NotificationPayload | ContextOnly | Notification |
| SubagentStart | No | agent_type | SubagentStartPayload | ContextOnly | SubagentStart |
| SubagentStop | Yes | agent_type | SubagentStopPayload | StopDecision (block continues) | SubagentStop |
| Stop | Yes | -- | StopPayload | StopDecision (block continues) | TurnComplete |
| TeammateIdle | Yes | -- | TeammateIdlePayload | ExitCodeOnly | TurnComplete (lossy) |
| TaskCompleted | Yes | -- | TaskCompletedPayload | ExitCodeOnly | TurnComplete (lossy) |
| PreCompact | No | trigger | PreCompactPayload | Informational | BeforeCompact |

**Delivery mechanism**: Shell commands via settings.json. JSON on stdin, JSON on stdout. Exit codes: 0=success, 2=block. Three handler types: command, prompt, agent.

**Unique features**:
- Three handler types (command, prompt, agent); TeammateIdle only supports command
- CLAUDE_ENV_FILE for persisting env vars to subsequent Bash calls (SessionStart only)
- `stop_hook_active` infinite-loop guard on Stop/SubagentStop

**Key types needed**:
- `CommonInputFields` -- session_id, transcript_path, cwd, permission_mode, hook_event_name (on ALL events)
- `MatcherField` -- ToolName, Source, Reason, NotificationType, AgentType, Trigger
- `HookHandlerType` -- Command, Prompt, Agent
- `DecisionPattern` -- HookSpecificPermission, HookSpecificBehavior, TopLevelDecision, ExitCodeOnly, Informational
- `PermissionMode` -- Default, Plan, AcceptEdits, DontAsk, BypassPermissions
- `PreToolUseDecision` -- Allow, Deny, Ask (3-level)
- `ExitCode` -- Success(0), Block(2)
- `TypedToolInput` -- 9 built-in tools (Bash, Write, Edit, Read, Glob, Grep, WebFetch, WebSearch, Task)

**Mapping to AgenticEvent**: All 14 map (total From). TeammateIdle+TaskCompleted are lossy->TurnComplete. Reverse TryFrom fails for BeforeModel, AfterModel, TurnError, HumanInTheLoop.

---

## 2. Codex

**Enum name**: `CodexEvent`
**Variant count**: 10

| Variant | Can Block | Matcher | Payload Type | Response Pattern | Closest AgenticEvent |
|---------|-----------|---------|--------------|-----------------|---------------------|
| AfterAgent | No | -- | AfterAgentPayload (kebab-case JSON) | Ignored (fire-and-forget) | TurnComplete |
| AfterToolUse | No (internal abort) | -- | AfterToolUsePayload (nested hook_event) | HookResult (internal only) | AfterTool |
| ThreadStarted | No | -- | ThreadStartedPayload | None (stream) | SessionStart |
| TurnStarted | No | -- | TurnStartedPayload (minimal) | None (stream) | BeforePrompt |
| TurnCompleted | No | -- | TurnCompletedPayload + TokenUsage | None (stream) | TurnComplete |
| TurnFailed | No | -- | TurnFailedPayload + StreamError | None (stream) | TurnError |
| ItemStarted | No | -- | ItemPayload (generic item lifecycle) | None (stream) | BeforeTool |
| ItemUpdated | No | -- | ItemPayload (incremental delta) | None (stream) | AfterModel |
| ItemCompleted | No | -- | ItemPayload (final state) | None (stream) | AfterTool |
| Error | No | -- | ErrorPayload | None (stream) | TurnError |

**Delivery mechanism**: Three surfaces: (1) notify hook via CLI argument (AfterAgent), (2) internal Rust callback (AfterToolUse), (3) JSONL stream via `codex exec --json`. All fire-and-forget -- no return channel.

**Unique features**:
- Only 1 user-configurable hook (AfterAgent via config.toml `notify`); payload delivered as CLI argument, not stdin
- JSONL stream item lifecycle (started/updated/completed) with 7 item types (agent_message, reasoning, command_execution, file_change, mcp_tool_call, web_search, plan_update)
- TokenUsage stats in TurnCompleted (input_tokens, cached_input_tokens, output_tokens)

**Key types needed**:
- `DeliveryMechanism` -- CliArgument, InternalCallback, JsonlStream
- `ItemType` -- AgentMessage, Reasoning, CommandExecution, FileChange, McpToolCall, WebSearch, PlanUpdate
- `ToolKind` -- Function, Custom, LocalShell, Mcp
- `HookResult` -- Success, FailedContinue, FailedAbort (internal only)
- `TokenUsage` -- input_tokens, cached_input_tokens, output_tokens

**Mapping to AgenticEvent**: All 10 map (total From, lossy). No Codex equivalent for: SessionEnd, ToolError (separate), PermissionRequest, Notification, SubagentStart/Stop, BeforeModel, BeforeCompact, HumanInTheLoop (9 gaps).

---

## 3. Gemini CLI

**Enum name**: `GeminiCliEvent`
**Variant count**: 11

| Variant | Can Block | Matcher Field | Payload Type | Response Pattern | Closest AgenticEvent |
|---------|-----------|---------------|--------------|-----------------|---------------------|
| SessionStart | No | source (exact) | GeminiSessionStartPayload | ContextInjection | SessionStart |
| SessionEnd | No | reason (exact) | GeminiSessionEndPayload | BestEffort (not awaited) | SessionEnd |
| BeforeAgent | Yes | -- | GeminiBeforeAgentPayload | PromptDecision (deny erases prompt) | BeforePrompt |
| AfterAgent | Yes | -- | GeminiAfterAgentPayload | RetryDecision (deny=retry, clearContext) | TurnComplete |
| BeforeModel | Yes | -- | GeminiBeforeModelPayload (LlmRequest) | ModelOverride (override request or inject synthetic response) | BeforeModel |
| AfterModel | Yes | -- | GeminiAfterModelPayload (LlmRequest+LlmResponse) | ChunkReplacement (per streaming chunk) | AfterModel |
| BeforeToolSelection | No | -- | GeminiBeforeToolSelectionPayload | ToolConfig (mode+allowedFunctionNames) | BeforeModel (lossy) |
| BeforeTool | Yes | tool_name (regex) | GeminiBeforeToolPayload | ToolDecision (deny, reason=tool error msg) | BeforeTool |
| AfterTool | Yes | tool_name (regex) | GeminiAfterToolPayload | ResultRedaction (deny hides output) | AfterTool |
| PreCompress | No | trigger (exact) | GeminiPreCompressPayload | Informational | BeforeCompact |
| Notification | No | -- | GeminiNotificationPayload | Informational | Notification |

**Delivery mechanism**: Shell commands. JSON on stdin, JSON on stdout. Exit code 2 has DIFFERENT effects per event (unlike Claude Code's uniform "block"). Only `command` handler type (no prompt/agent).

**Unique features**:
- Model-level hooks (BeforeModel/AfterModel) with stable LlmRequest/LlmResponse API; can inject synthetic response to skip LLM
- AfterAgent retry semantics: deny triggers automatic retry with reason as correction + clearContext
- Dual matcher strategy: regex for tool events, exact string for lifecycle events
- Multi-hook aggregation strategies: OrDecision, FieldReplacement, Union, SimpleMerge

**Key types needed**:
- `MatcherStrategy` -- Regex, ExactString
- `GeminiDecisionPattern` -- PromptDecision, RetryDecision, ModelOverride, ChunkReplacement, ToolConfig, ToolDecision, ResultRedaction, ContextInjection, Informational
- `ExitCode2Effect` -- 8 different effects (BlockToolContinueTurn, HideResultContinueTurn, AbortTurnErasePrompt, RejectAndRetry, AbortTurnSkipLlm, AbortTurnDiscardOutput, Advisory, NotSupported)
- `AggregationStrategy` -- OrDecision, FieldReplacement, Union, SimpleMerge
- `LlmRequest/LlmResponse` -- Stable model API types (messages, config, candidates, usage)
- `ToolConfig` -- mode (AUTO/ANY/NONE) + allowedFunctionNames
- `McpContext` -- MCP server connection details on tool events
- 15 built-in tools (snake_case: write_file, run_shell_command, etc.)

**Mapping to AgenticEvent**: All 11 map (total From). BeforeToolSelection is lossy->BeforeModel. No Gemini equivalent for: ToolError (merged into AfterTool), PermissionRequest, SubagentStart/Stop, TurnError, HumanInTheLoop (6 gaps).

---

## 4. Goose

**Enum name**: `GooseEvent`
**Variant count**: 7

| Variant | Can Block | Matcher | Payload Type | Response Pattern | Closest AgenticEvent |
|---------|-----------|---------|--------------|-----------------|---------------------|
| StatusWaiting | No | -- | GooseStatus string | None | TurnComplete |
| StatusThinking | No | -- | GooseStatus string | None | BeforeModel |
| Message | No | -- | MessagePayload (composite: text+tools+responses) | None | AfterModel |
| Notification | No | -- | NotificationPayload (log or progress) | None | Notification |
| ModelChange | No | -- | ModelChangePayload (model+mode) | None | Notification (lossy) |
| Error | No | -- | ErrorPayload (error string) | None | TurnError |
| Complete | No | -- | CompletePayload (total_tokens) | None | SessionEnd |

**Delivery mechanism**: Two surfaces: (1) GOOSE_STATUS_HOOK shell command (fire-and-forget, stdout/stderr suppressed, exit code ignored), (2) stream-json output (`--output-format stream-json`). ALL events outbound-only, zero return channel.

**Unique features**:
- Purely observe-only architecture -- no event can influence agent behavior at all
- Composite Message event contains text, tool requests, tool responses, action-required, and thinking in a single payload
- GooseMode config (auto/approve/chat/smart_approve) replaces hook-based permission

**Key types needed**:
- `DeliveryMechanism` -- StatusHook, StreamJson
- `GooseStatus` -- Waiting, Thinking
- `MessageContent` -- Text, ToolRequest, ToolResponse, ActionRequired, Thinking (tagged union)
- `GooseMode` -- Auto, Approve, Chat, SmartApprove

**Mapping to AgenticEvent**: All 7 map (total From, very lossy). No Goose equivalent for: SessionStart, BeforePrompt, BeforeTool, AfterTool, ToolError, PermissionRequest, SubagentStart/Stop, BeforeCompact, HumanInTheLoop (10 gaps -- highest).

---

## 5. Kimi Code

**Enum name**: `KimiCodeEvent`
**Variant count**: 15

| Variant | Can Block | Matcher | Payload Type | Response Pattern | Closest AgenticEvent |
|---------|-----------|---------|--------------|-----------------|---------------------|
| TurnBegin | No | -- | TurnBeginPayload (user_input) | None (notification) | BeforePrompt |
| TurnEnd | No | -- | Empty | None (notification) | TurnComplete |
| StepBegin | No | -- | StepBeginPayload (step number) | None (notification) | BeforeModel |
| StepInterrupted | No | -- | Empty | None (notification) | TurnError |
| CompactionBegin | No | -- | Empty | None (notification) | BeforeCompact |
| CompactionEnd | No | -- | Empty | None (notification) | AfterModel (lossy) |
| StatusUpdate | No | -- | StatusUpdatePayload (context_usage, token_usage) | None (notification) | Notification |
| ContentPart | No | -- | ContentPartPayload (text/think/image/audio/video) | None (notification) | AfterModel |
| ToolCall | No | -- | ToolCallPayload (function name+args) | None (notification) | BeforeTool |
| ToolCallPart | No | -- | ToolCallPartPayload (streaming args fragment) | None (notification) | BeforeTool |
| ToolResult | No | -- | ToolResultPayload (is_error, output, display blocks) | None (notification) | AfterTool |
| ApprovalResponse | No | -- | ApprovalResponsePayload (resolved decision) | None (notification) | PermissionRequest |
| SubagentEvent | No | -- | SubagentEventPayload (recursive nested event) | None (notification) | SubagentStart |
| ApprovalRequest | **Yes** | -- | ApprovalRequestPayload (id, tool, action, display) | ApprovalResponse (approve/approve_for_session/reject) | PermissionRequest |
| ToolCallRequest | **Yes** | -- | ToolCallRequestPayload (external tool invocation) | ToolResult (client executes tool) | BeforeTool |

**Delivery mechanism**: JSON-RPC 2.0 bidirectional protocol over stdin/stdout ("Wire mode"). Notifications (method="event") are fire-and-forget. Requests (method="request") are blocking -- agent pauses until JSON-RPC response returned.

**Unique features**:
- Wire protocol with client-to-agent methods (initialize, prompt, replay, cancel) and external tool registration via initialize handshake
- Recursive SubagentEvent (nested Wire messages, can contain another SubagentEvent)
- Streaming events: ContentPart (text/think/image/audio/video) and ToolCallPart (argument fragments)
- DisplayBlock rich UI rendering (brief, diff, todo, shell) shared across ToolResult and ApprovalRequest

**Key types needed**:
- `ApprovalDecision` -- Approve, ApproveForSession, Reject
- `ContentPartVariant` -- Text, Think, ImageUrl, AudioUrl, VideoUrl
- `DisplayBlock` -- Brief, Diff, Todo, Shell, Unknown
- `KimiClientMethod` -- Initialize, Prompt, Replay, Cancel
- `InitializeParams/Result` -- Protocol negotiation + external tool registration
- `KimiErrorCode` -- Standard JSON-RPC + 4 custom codes (-32000 to -32003)
- `KimiCodeWireMessage` -- Wrapper for recursive event dispatch
- 7 built-in tools (PascalCase: Shell, FileWrite, FileRead, Grep, Glob, WebFetch, Task)

**Mapping to AgenticEvent**: All 15 map (total From, lossy). No Kimi equivalent for: SessionStart, SessionEnd, SubagentStop, HumanInTheLoop (4 gaps).

---

## 6. OpenCode

**Enum name**: `OpenCodeEvent`
**Variant count**: 16

| Variant | Can Block | Matcher | Payload Type | Response Pattern | Closest AgenticEvent |
|---------|-----------|---------|--------------|-----------------|---------------------|
| Event | No | -- | BusEventPayload (40+ types) | FireAndForget | depends on inner type |
| ToolExecuteBefore | Yes (throw) | -- | ToolExecuteBeforeInput | MutableArgs | BeforeTool |
| ToolExecuteAfter | No | -- | ToolExecuteAfterInput | MutableToolOutput | AfterTool |
| ToolDefinition | No | -- | ToolDefinitionInput | MutableToolDefinition | None |
| ShellEnv | No | -- | ShellEnvInput | MutableEnv | None |
| ChatMessage | No | -- | ChatMessageInput | MutableMessage | BeforePrompt |
| ChatParams | No | -- | ChatParamsInput | MutableParams | BeforeModel |
| ChatHeaders | No | -- | ChatParamsInput | MutableHeaders | BeforeModel |
| PermissionAsk | Yes (status) | -- | PermissionAskInput | PermissionDecision (ask/allow/deny) | PermissionRequest |
| CommandExecuteBefore | No | -- | CommandExecuteBeforeInput | MutableParts | None |
| Config | No | -- | ConfigInput (full config) | FireAndForget | None |
| Auth | No | -- | AuthHookDef (structured registration) | AuthRegistration | None |
| ExperimentalChatSystemTransform | No | -- | SystemTransformInput | MutableSystemPrompt | BeforeModel |
| ExperimentalChatMessagesTransform | No | -- | Empty | MutableMessages | BeforeModel |
| ExperimentalSessionCompacting | No | -- | SessionCompactingInput | MutableCompactionPrompt | BeforeCompact |
| ExperimentalTextComplete | No | -- | TextCompleteInput | MutableText | AfterModel |

**Delivery mechanism**: In-process JS/TS plugin functions. Plugin receives read-only `input` + mutable `output`, mutates output, returns Promise<void>. Blocking via throwing errors (ToolExecuteBefore) or mutating status field (PermissionAsk). No exit codes, no shell processes.

**Unique features**:
- Catch-all `event` bus hook receives 40+ system bus event types (session.created, file.edited, permission.asked, etc.)
- Input/output pair mutation pattern (not stdin/stdout JSON)
- 6 hooks with NO unified mapping (Event, ToolDefinition, ShellEnv, CommandExecuteBefore, Config, Auth)
- 4 experimental hooks for system prompt, message history, compaction prompt, and text post-processing

**Key types needed**:
- `FlowPattern` -- Informational, MutateOnly, MutateOrThrow, StatusDecision, Registration
- `BusEventType` -- 40+ known event types (#[non_exhaustive])
- `PermissionStatus` -- Ask, Allow, Deny (3-level)
- `AuthHookDef/AuthMethod` -- OAuth or ApiKey structured registration
- `PluginContext` -- directory, worktree, serverUrl (passed at init)

**Mapping to AgenticEvent**: 10 of 16 map (partial, uses Option not From). 6 have no mapping. Event bus maps via separate `bus_event_type_to_agentic()` function. No OpenCode equivalent for: ToolError (after only fires on success), SubagentStart/Stop.

---

## 7. Qwen CLI

**Enum name**: `QwenCliEvent`
**Variant count**: 7

| Variant | Can Block | Matcher | Payload Type | Response Pattern | Closest AgenticEvent |
|---------|-----------|---------|--------------|-----------------|---------------------|
| CanUseTool | **Yes** | -- | CanUseToolPayload (tool_name, input, aborted) | PermissionResult (Allow+updatedInput / Deny+message+interrupt) | PermissionRequest |
| SubagentPreToolUse | No | -- | SubagentPreToolUsePayload (subagent_id, tool_name, args) | Void (fire-and-forget, NOT awaited) | BeforeTool |
| SubagentPostToolUse | No | -- | SubagentPostToolUsePayload (success, duration_ms, error_message) | Void (awaited but no return) | AfterTool |
| SubagentStop | No | -- | SubagentStopPayload (terminate_reason, summary) | Void | SubagentStop |
| StreamSessionStart | No | -- | StreamSessionStartPayload (session_id, model) | OutputOnly (no return channel) | SessionStart |
| StreamAssistantMessage | No | -- | StreamAssistantMessagePayload (content blocks, usage) | OutputOnly | AfterModel |
| StreamResult | No | -- | StreamResultPayload (status, duration_ms, usage, summary) | OutputOnly | SessionEnd |

**Delivery mechanism**: Three fragmented surfaces: (1) SDK callback (CanUseTool -- only bidirectional one), (2) internal SubagentHooks (not user-configurable, notification-only), (3) headless stream-json output (output-only). 60-second timeout on CanUseTool auto-denies.

**Unique features**:
- Most fragmented surface model -- three independent surfaces with different access patterns
- Only 1 blocking event (CanUseTool) and only via SDK, not config files
- SubagentPreToolUse is fire-and-forget (NOT awaited); errors silently swallowed
- Permission priority chain: excludeTools > plan mode > yolo mode > allowedTools > callback > default deny

**Key types needed**:
- `QwenSurface` -- SdkCallback, InternalSubagentHook, HeadlessStream
- `QwenPermissionResult` -- Allow{updatedInput} | Deny{message, interrupt}
- `QwenApprovalMode` -- Plan, Default, AutoEdit, Yolo
- `PermissionPriority` -- 6-step chain determining whether CanUseTool fires

**Mapping to AgenticEvent**: All 7 map (total From). No Qwen equivalent for: BeforePrompt, TurnComplete, TurnError, SubagentStart, BeforeModel, BeforeCompact, Notification, HumanInTheLoop (8 gaps).

---

## 8. Roo Code

**Enum name**: `RooCodeEvent`
**Variant count**: 47

| Variant (key subset) | Can Block | Matcher | Payload Type | Response Pattern | Closest AgenticEvent |
|---------|-----------|---------|--------------|-----------------|---------------------|
| WaitingForInput | No (actionable) | -- | WaitingForInputPayload | Void (call approve/reject/respond) | HumanInTheLoop |
| TaskCompleted | No (actionable) | -- | TaskCompletedPayload | Void (call respond/newTask) | TurnComplete |
| Error | No | -- | Error string | Void | TurnError |
| StreamingStarted | No | -- | Void | Void | BeforeModel |
| StreamingEnded | No | -- | Void | Void | AfterModel |
| ToolUseOutput | No | -- | JsonOutputEventPayload | Void | BeforeTool |
| ToolResultOutput | No | -- | JsonOutputEventPayload | Void | AfterTool |
| TaskCreated | No | -- | task_id | Void | SessionStart |
| TaskAborted | No | -- | task_id | Void | SessionEnd |
| TaskSpawned | No | -- | parent+child task_ids | Void | SubagentStart |
| TaskDelegationCompleted | No | -- | DelegationCompletedPayload | Void | SubagentStop |
| TaskToolFailed | No | -- | TaskToolFailedPayload | Void | ToolError |
| ModeChanged | No | -- | ModeChangedPayload | Void | Notification |

*30 additional events omitted (UI focus, state granularity, query-response, eval, streaming partials) -- none map to AgenticEvent.*

**Delivery mechanism**: Three surfaces: (1) CLI programmatic EventEmitter (11 events), (2) CLI stream-json NDJSON (8 events), (3) VS Code extension API (28 events). ALL events observational only -- listener return values ignored. Flow control via explicit client method calls.

**Unique features**:
- Largest event surface (47 variants) but ALL are observational -- zero hooks can influence flow via return values
- Flow control via `ClientAction` enum: Approve, Reject, Respond, NewTask, CancelTask, ClearTask, ResumeTask, RetryApiRequest, ContinueTerminal, AbortTerminal
- Rich agent state machine: AgentLoopState (NoTask, Running, Streaming, WaitingForInput, Idle, Resumable) + RequiredAction + ClineAsk (11 ask types)
- Subtask delegation lifecycle: TaskSpawned -> TaskDelegated -> TaskDelegationCompleted -> TaskDelegationResumed

**Key types needed**:
- `EventSurface` -- CliProgrammatic, CliStructuredOutput, VsCodeExtensionApi
- `ClientAction` -- 10 explicit methods for flow control
- `AgentLoopState` -- NoTask, Running, Streaming, WaitingForInput, Idle, Resumable
- `RequiredAction` -- None, Approve, Answer, RetryOrNewTask, ProceedOrNewTask, StartTask, ResumeOrAbandon, StartNewTask, ContinueOrAbort
- `ClineAsk` -- 11 ask types (followup, command, tool, api_req_failed, use_mcp_server, etc.)
- `BuiltinMode` -- Code, Architect, Ask, Debug
- `CostInfo` -- total_cost, input_tokens, output_tokens, cache_writes, cache_reads

**Mapping to AgenticEvent**: Only 17 of 47 map (TryFrom, partial). 30 events have no unified equivalent. No Roo equivalent for: BeforePrompt (blocking), PermissionRequest (as hook), BeforeCompact.

---

## Cross-CLI Comparison Matrix

### AgenticEvent coverage (which CLIs support each unified event)

| AgenticEvent | Claude | Codex | Gemini | Goose | Kimi | OpenCode | Qwen | Roo |
|-------------|--------|-------|--------|-------|------|----------|------|-----|
| SessionStart | Y | Y | Y | -- | -- | bus | Y | Y |
| SessionEnd | Y | -- | Y | Y | -- | bus | Y | Y |
| BeforePrompt | Y | Y | Y | -- | Y | Y | -- | -- |
| BeforeTool | Y | Y | Y | -- | Y | Y | Y | Y* |
| AfterTool | Y | Y | Y | -- | Y | Y | Y | Y* |
| ToolError | Y | -- | -- | -- | Y | -- | -- | Y |
| PermissionRequest | Y | -- | -- | -- | Y | Y | Y | -- |
| Notification | Y | -- | Y | Y | Y | bus | -- | Y |
| SubagentStart | Y | -- | -- | -- | Y | -- | -- | Y |
| SubagentStop | Y | -- | -- | -- | -- | -- | Y | Y |
| TurnComplete | Y | Y | Y | Y | Y | bus | -- | Y |
| TurnError | -- | Y | -- | Y | Y | bus | -- | Y |
| BeforeModel | -- | -- | Y | Y | Y | Y | -- | Y |
| AfterModel | -- | Y | Y | Y | Y | Y | Y | Y |
| BeforeCompact | Y | -- | Y | -- | Y | Y | -- | -- |
| HumanInTheLoop | -- | -- | -- | -- | -- | bus | -- | Y |

Y = dedicated event, bus = only via catch-all event bus, Y* = observational only, -- = not supported

### Blocking capability

| CLI | Blocking events | Mechanism |
|-----|----------------|-----------|
| Claude Code | 7 of 14 | Exit code 2 + JSON decision fields |
| Codex | 0 of 10 | Fire-and-forget (internal AfterToolUse can abort) |
| Gemini CLI | 6 of 11 | Exit code 2 (per-event effects) + JSON decision |
| Goose | 0 of 7 | Purely observe-only |
| Kimi Code | 2 of 15 | JSON-RPC response (ApprovalRequest, ToolCallRequest) |
| OpenCode | 2 of 16 | Throw exception (ToolExecuteBefore) or mutate status (PermissionAsk) |
| Qwen CLI | 1 of 7 | SDK callback response (CanUseTool) |
| Roo Code | 0 of 47 | Observational only; flow via explicit client method calls |

### Delivery mechanisms

| CLI | Mechanism |
|-----|-----------|
| Claude Code | Shell commands (JSON stdin/stdout/exit code) |
| Codex | CLI argument (notify), internal callback, JSONL stream |
| Gemini CLI | Shell commands (JSON stdin/stdout/exit code) |
| Goose | Status hook (shell, fire-and-forget), stream-json |
| Kimi Code | JSON-RPC 2.0 bidirectional protocol (stdin/stdout) |
| OpenCode | In-process JS/TS plugins (input/output mutation) |
| Qwen CLI | SDK callback, internal hooks, stream-json |
| Roo Code | CLI EventEmitter, CLI stream-json, VS Code extension API |
