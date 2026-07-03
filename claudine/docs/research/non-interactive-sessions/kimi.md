---
$schema: ./_schema.yaml
created: 2026-04-06
last_updated: 2026-07-02
agent: codex
model: default
docs: https://moonshotai.github.io/kimi-cli/en/customization/wire-mode.html
invocation:
  - command: "kimi --wire"
    stdin_support: true
    prompt_arg: "JSON-RPC `prompt` request on stdin; `--prompt` is ignored in Wire mode"
    notes: "Starts a Wire JSON-RPC server over stdin/stdout for a fresh or resumed session."
  - command: "kimi --wire --session <session-id>"
    stdin_support: true
    prompt_arg: "JSON-RPC `prompt` request on stdin"
    notes: "Resumes the named session if present; creates a session with that ID if not found."
  - command: "kimi --wire --continue"
    stdin_support: true
    prompt_arg: "JSON-RPC `prompt` request on stdin"
    notes: "Continues the previous session for the selected work directory."
  - command: "kimi --print -p <prompt> --output-format stream-json"
    stdin_support: false
    prompt_arg: "`--prompt` / `-p` / `--command` / `-c`"
    notes: "One-shot print mode fallback; exits after the run and auto-enables AFK behavior."
  - command: "printf '%s\n' '{\"role\":\"user\",\"content\":\"...\"}' | kimi --print --input-format stream-json --output-format stream-json"
    stdin_support: true
    prompt_arg: "JSONL user messages on stdin"
    notes: "Print mode reads user-role JSONL messages until stdin closes; this is not Wire."
output_formats:
  - name: "wire"
    cli_value: "--wire"
    stream: true
    format: jsonrpc_lines
    description: "Bidirectional JSON-RPC 2.0 over newline-delimited JSON on stdin/stdout."
    side_effects: "Changes the process into a protocol server; `--prompt`, `--input-format`, and `--output-format` do not apply."
  - name: "stream-json"
    cli_value: "stream-json"
    stream: true
    format: jsonl
    description: "Print-mode JSONL made of assistant messages, tool messages, notifications, and plan display records."
    side_effects: "Whole assistant messages are flushed at step boundaries; many Wire events are intentionally suppressed."
  - name: "text"
    cli_value: "text"
    stream: true
    format: text
    description: "Human-oriented print-mode text."
    side_effects: "Not suitable as a parser contract; source uses Rich printing for non-final text mode."
  - name: "final-message-only text"
    cli_value: "--final-message-only with text, or --quiet"
    stream: false
    format: text
    description: "Only the final assistant text."
    side_effects: "Drops tool calls, progress, notifications, and intermediate state."
schema_sources:
  - url: "https://moonshotai.github.io/kimi-cli/en/customization/wire-mode.html"
    schema_type: other
    formal: true
    notes: "Official TypeScript-style Wire protocol documentation; current docs state protocol version 1.10."
  - url: "https://github.com/MoonshotAI/kimi-cli/blob/main/src/kimi_cli/wire/types.py"
    schema_type: other
    formal: true
    notes: "Provider source Pydantic models for Wire event/request unions and payload fields."
  - url: "https://github.com/MoonshotAI/kimi-cli/blob/main/src/kimi_cli/wire/jsonrpc.py"
    schema_type: other
    formal: true
    notes: "Provider source Pydantic models for JSON-RPC envelopes and inbound/outbound method sets."
  - url: "https://github.com/MoonshotAI/kimi-cli/blob/main/src/kimi_cli/wire/protocol.py"
    schema_type: other
    formal: true
    notes: "Source constant `WIRE_PROTOCOL_VERSION = \"1.10\"`."
  - url: "https://github.com/MoonshotAI/kimi-cli/blob/main/src/kimi_cli/ui/print/visualize.py"
    schema_type: other
    formal: false
    notes: "Source behavior for print `stream-json`; it is a projection of Wire messages, not the Wire schema."
cli_params:
  - flag: "--wire"
    value: ""
    description: "Starts Wire server mode; mutually exclusive with shell, print, and ACP modes."
    example: "kimi --wire --work-dir /repo"
  - flag: "--print"
    value: ""
    description: "Runs non-interactively in print mode and auto-dismisses questions/approvals for that invocation."
    example: "kimi --print -p \"fix tests\" --output-format stream-json"
  - flag: "--input-format"
    value: "text | stream-json"
    description: "Print-only input format; `stream-json` consumes user-role JSONL from stdin."
    example: "kimi --print --input-format stream-json --output-format stream-json"
  - flag: "--output-format"
    value: "text | stream-json"
    description: "Print-only output format; use `stream-json` for JSONL fallback output."
    example: "kimi --print -p \"summarize\" --output-format stream-json"
  - flag: "--prompt, -p, --command, -c"
    value: "TEXT"
    description: "One-shot prompt for shell/print mode; ignored by Wire and ACP server modes."
    example: "kimi --print -p \"explain this repo\""
  - flag: "--quiet"
    value: ""
    description: "Alias for `--print --output-format text --final-message-only`; parser-hostile."
    example: "kimi --quiet -p \"commit message\""
  - flag: "--final-message-only"
    value: ""
    description: "Print-only lossy final-answer output."
    example: "kimi --print -p \"commit message\" --final-message-only"
  - flag: "--work-dir, -w"
    value: "PATH"
    description: "Sets the working directory/root for file operations."
    example: "kimi --wire --work-dir /repo"
  - flag: "--add-dir"
    value: "PATH"
    description: "Adds extra accessible workspace roots and persists them in session state."
    example: "kimi --wire --add-dir /shared"
  - flag: "--session, --resume, -S, -r"
    value: "[ID]"
    description: "Resumes a session by ID; ID-less picker is shell-only and invalid in Wire/print."
    example: "kimi --wire --session abc123"
  - flag: "--continue, -C"
    value: ""
    description: "Continues the previous session for the working directory."
    example: "kimi --wire --continue"
  - flag: "--model, -m"
    value: "NAME"
    description: "Overrides `default_model` from config for this process."
    example: "kimi --wire --model kimi-for-coding"
  - flag: "--thinking / --no-thinking"
    value: ""
    description: "Overrides default or previous-session thinking mode when model capabilities allow it."
    example: "kimi --wire --thinking"
  - flag: "--yolo, --yes, --auto-approve"
    value: ""
    description: "Auto-approves actions; docs distinguish it from AFK because user questions may still be reachable."
    example: "kimi --wire --yolo"
  - flag: "--afk"
    value: ""
    description: "No-user-present mode: auto-approves tools and auto-dismisses AskUserQuestion."
    example: "kimi --wire --afk"
  - flag: "--plan"
    value: ""
    description: "Starts or resumes in plan mode; Wire clients should also negotiate plan-mode support."
    example: "kimi --wire --plan"
  - flag: "--max-steps-per-turn"
    value: "N"
    description: "Overrides loop-control max steps for a turn."
    example: "kimi --print -p \"task\" --max-steps-per-turn 20"
  - flag: "--max-retries-per-step"
    value: "N"
    description: "Overrides retry count per step."
    example: "kimi --wire --max-retries-per-step 2"
  - flag: "--config"
    value: "TOML_OR_JSON"
    description: "Loads complete config from a string; mutually exclusive with `--config-file`."
    example: "kimi --wire --config '{\"default_model\":\"kimi-for-coding\"}'"
  - flag: "--config-file"
    value: "PATH"
    description: "Loads TOML or JSON config from a file instead of the default user config."
    example: "kimi --wire --config-file ./kimi.toml"
  - flag: "--mcp-config-file"
    value: "PATH"
    description: "Loads MCP config JSON; repeatable. Defaults to `~/.kimi/mcp.json` when present."
    example: "kimi --wire --mcp-config-file ./mcp.json"
  - flag: "--mcp-config"
    value: "JSON"
    description: "Inline MCP config JSON; repeatable."
    example: "kimi --wire --mcp-config '{\"mcpServers\":{}}'"
  - flag: "--verbose"
    value: ""
    description: "Prints verbose runtime information; not a structured stream selector."
    example: "kimi --verbose --print -p \"task\""
  - flag: "--debug"
    value: ""
    description: "Enables trace logging to `~/.kimi/logs/kimi.log`."
    example: "kimi --debug --wire"
config_files:
  - os: all
    scope: user
    path: "~/.kimi/config.toml"
    format: toml
    effect: "Default providers, models, loop control, background behavior, hooks, MCP client settings, default YOLO/plan/thinking, theme, telemetry."
    notes: "Created automatically; `--config-file` can replace it and `--config` can replace file loading for a process."
  - os: all
    scope: user
    path: "~/.kimi/config.json"
    format: json
    effect: "Legacy config source."
    notes: "Migrated to `config.toml` with backup `config.json.bak` when TOML config does not exist."
  - os: all
    scope: user
    path: "~/.kimi/mcp.json"
    format: json
    effect: "Default MCP server configuration loaded when no explicit MCP config is supplied."
    notes: "Additional MCP config files and inline JSON are appended for the invocation."
  - os: all
    scope: user
    path: "~/.kimi/sessions/<work-dir-hash>/<session-id>/wire.jsonl"
    format: other
    effect: "Persistent Wire event/request log used for replay and visualizer/export."
    notes: "Not startup configuration, but important for resume/replay and parser fixture capture."
  - os: all
    scope: user
    path: "~/.kimi/sessions/<work-dir-hash>/<session-id>/state.json"
    format: json
    effect: "Session state: approval state, plan mode, extra roots, subagent metadata, and plan identifiers."
    notes: "Resumed sessions restore these values."
  - os: all
    scope: repo
    path: ".kimi/AGENTS.md"
    format: text
    effect: "Merged into the generated system prompt via `KIMI_AGENTS_MD`."
    notes: "Affects agent behavior, not stream framing."
env_vars:
  - name: "KIMI_SHARE_DIR"
    effect: "Moves the Kimi runtime data directory away from `~/.kimi`."
    notes: "Affects config, sessions, wire logs, credentials, MCP OAuth, and logs."
  - name: "NO_COLOR"
    effect: "Standard color suppression for terminal-oriented output."
    notes: "Not a substitute for structured output; Wire remains JSON-RPC lines."
  - name: "FORCE_COLOR"
    effect: "May force color in human output through terminal libraries."
    notes: "Avoid text mode parsing regardless."
io_contract:
  stdout: structured_only
  stderr: diagnostics_only
  stdin: stream_protocol
  framing: jsonrpc_lines
  noise_handling: "For `--wire`, parse stdout as one JSON-RPC message per line and treat stderr/log files as diagnostics. For print `stream-json`, parse stdout as JSONL but expect a lossy projection."
  notes: "Wire uses stdin for client requests/responses and stdout for server responses/events/requests; print mode uses stdin either as prompt text or user-message JSONL."
stream_contract:
  discriminator: "JSON-RPC `method`; Wire payload discriminator is `params.type` for `event` and `request` messages"
  event_ordering: "`TurnBegin` before turn events, `StepBegin` before step events, `TurnEnd` after normal turn events; JSON-RPC `prompt` response is terminal for that turn."
  correlation_fields:
    - "id"
    - "params.payload.id"
    - "params.payload.tool_call_id"
    - "params.payload.request_id"
    - "params.payload.parent_tool_call_id"
    - "params.payload.message_id"
  terminal_event: "JSON-RPC response to `prompt` with `result.status`; `TurnEnd` is useful but may be omitted on interruption."
  partial_message_events: true
  unknown_event_policy: "Skip unknown `params.type` after preserving raw payload; unknown JSON-RPC methods should be logged and answered with method-not-found only when Claudine is the peer."
  notes: "Wire content/tool parts may be deltas or complete records; print `stream-json` merges assistant content into whole messages."
session_metadata:
  session_id: "Not emitted as a first-class Wire event; available from selected session directory/state and hook payloads. Claudine should track it from launch/resume and `wire.jsonl` path."
  cwd: "`--work-dir` controls it; `SessionStart` hook input has cwd, but Wire initialize result does not."
  model: "Requested via `--model` or config; not reliably emitted in the Wire stream."
  provider: "Config/model mapping; not reliably emitted in the Wire stream."
  auth: "OAuth vs API key can be inferred internally by Kimi but is not exposed in Wire initialize result."
  version: "Initialize result `server.version`; `kimi info --json` also reports `kimi_cli_version` and `wire_protocol_version` on current versions."
  mcp_servers: "StatusUpdate payload can include `mcp_status.servers[]` with name/status/tools during startup."
  permission_mode: "Approval behavior is controlled by `--yolo`, `--afk`, print runtime AFK, config `default_yolo`, and session approval state; not emitted as a single mode field."
  notes: "Wire clients must keep launch-time metadata alongside parsed stream data."
stream_events:
  - event: "initialize"
    category: session
    fields: ["protocol_version", "server.name", "server.version", "slash_commands", "capabilities", "hooks.supported_events", "external_tools"]
    notes: "JSON-RPC request/response, not an event notification."
  - event: "prompt"
    category: session
    fields: ["result.status", "result.steps", "error.code", "error.message"]
    notes: "Starts a turn; response completes the turn."
  - event: "replay"
    category: session
    fields: ["result.status", "result.events", "result.requests"]
    notes: "Replays saved `wire.jsonl`; clients should not answer replayed requests."
  - event: "steer"
    category: session
    fields: ["params.user_input", "result.status"]
    notes: "Injects input into an active turn."
  - event: "set_plan_mode"
    category: plan
    fields: ["params.enabled", "result.plan_mode", "error.message"]
    notes: "Requires capability negotiation."
  - event: "cancel"
    category: session
    fields: ["result.status"]
    notes: "Cancels active prompt or replay."
  - event: "TurnBegin"
    category: session
    fields: ["user_input"]
    notes: "First event in a turn."
  - event: "TurnEnd"
    category: session
    fields: []
    notes: "Normal turn end; may be omitted on interruption."
  - event: "StepBegin"
    category: session
    fields: ["n"]
    notes: "Step boundary."
  - event: "StepInterrupted"
    category: error
    fields: []
    notes: "Current step interrupted by user or error."
  - event: "StepRetry"
    category: error
    fields: ["n", "next_attempt", "max_attempts", "wait_s", "error_type", "status_code"]
    notes: "Retryable step failure with wait timing."
  - event: "StatusUpdate"
    category: usage
    fields: ["context_usage", "context_tokens", "max_context_tokens", "token_usage", "message_id", "plan_mode", "mcp_status"]
    notes: "Primary live token/context/MCP/status event."
  - event: "TextPart"
    category: assistant
    fields: ["type", "text"]
    notes: "Content-part class from the `kosong` message model."
  - event: "ThinkPart"
    category: reasoning
    fields: ["type", "text"]
    notes: "Reasoning content when available and not suppressed by model/config."
  - event: "ToolCall"
    category: tool_call
    fields: ["id", "type", "function.name", "function.arguments"]
    notes: "Native tool call start/input."
  - event: "ToolCallPart"
    category: tool_call
    fields: ["id", "function.arguments"]
    notes: "Incremental tool-call delta merged by print JSON printer."
  - event: "ToolResult"
    category: tool_result
    fields: ["tool_call_id", "return_value", "display"]
    notes: "Native tool result; join to call by `tool_call_id`."
  - event: "ApprovalRequest"
    category: permission
    fields: ["id", "tool_call_id", "sender", "action", "description", "source_kind", "source_id", "agent_id", "subagent_type", "display"]
    notes: "JSON-RPC `request`; client must respond with `ApprovalResponse`."
  - event: "ApprovalResponse"
    category: permission
    fields: ["request_id", "response", "feedback"]
    notes: "Response values: `approve`, `approve_for_session`, `reject`."
  - event: "QuestionRequest"
    category: permission
    fields: ["id", "tool_call_id", "questions[].question", "questions[].options[]", "questions[].multi_select"]
    notes: "Only sent when client declares `supports_question`; otherwise AskUserQuestion is hidden."
  - event: "ToolCallRequest"
    category: tool_call
    fields: ["id", "name", "arguments"]
    notes: "External tool call registered during initialize; client must execute and answer."
  - event: "HookRequest"
    category: permission
    fields: ["id", "subscription_id", "event", "target", "input_data"]
    notes: "Client-side hook request; answer with allow/block."
  - event: "HookTriggered"
    category: other
    fields: ["event", "target", "hook_count"]
    notes: "Lifecycle hook batch started."
  - event: "HookResolved"
    category: other
    fields: ["event", "target", "action", "reason", "duration_ms"]
    notes: "Lifecycle hook batch finished."
  - event: "PlanDisplay"
    category: plan
    fields: ["content", "file_path"]
    notes: "Plan markdown and file path."
  - event: "SubagentEvent"
    category: subagent
    fields: ["parent_tool_call_id", "agent_id", "subagent_type", "event.type", "event.payload"]
    notes: "Nested Wire event from a subagent."
  - event: "Notification"
    category: other
    fields: ["id", "category", "type", "source_kind", "source_id", "title", "body", "severity", "created_at", "payload"]
    notes: "`created_at` is a float timestamp; timezone/unit is not formally specified in docs."
  - event: "MCPLoadingBegin"
    category: session
    fields: []
    notes: "MCP startup progress marker."
  - event: "MCPLoadingEnd"
    category: session
    fields: []
    notes: "MCP startup progress marker."
  - event: "CompactionBegin"
    category: other
    fields: []
    notes: "Context compaction marker."
  - event: "CompactionEnd"
    category: other
    fields: []
    notes: "Context compaction marker."
  - event: "BtwBegin"
    category: other
    fields: ["id", "question"]
    notes: "Side-question start."
  - event: "BtwEnd"
    category: other
    fields: ["id", "response", "error"]
    notes: "Side-question completion."
tools:
  - name: "Agent"
    call_visible: true
    result_visible: true
    metadata: ["ToolCall.id", "ToolResult.tool_call_id", "SubagentEvent.parent_tool_call_id", "SubagentEvent.agent_id", "SubagentEvent.subagent_type"]
    notes: "Starts/resumes subagents; nested events may be wrapped as `SubagentEvent`."
  - name: "AskUserQuestion"
    call_visible: true
    result_visible: true
    metadata: ["QuestionRequest.id", "QuestionRequest.tool_call_id", "QuestionResponse.answers"]
    notes: "In Wire it requires capability negotiation; in AFK/print it is auto-dismissed."
  - name: "SetTodoList"
    call_visible: true
    result_visible: true
    metadata: ["ToolCall.function.arguments", "ToolResult.display[type=todo]"]
    notes: "Todo state can also appear in display blocks."
  - name: "Shell"
    call_visible: true
    result_visible: true
    metadata: ["ToolCall.function.arguments.command", "ToolResult.return_value", "ToolResult.display[type=shell]", "Notification for background tasks"]
    notes: "Requires approval unless approval mode auto-approves; background tasks can produce notifications."
  - name: "ReadFile / ReadMediaFile / Glob / Grep"
    call_visible: true
    result_visible: true
    metadata: ["ToolCall.function.arguments", "ToolResult.return_value"]
    notes: "Read denials are inferred from tool result/error text, not a dedicated denial event."
  - name: "WriteFile / StrReplaceFile"
    call_visible: true
    result_visible: true
    metadata: ["ApprovalRequest.display[type=diff]", "ToolCall.function.arguments.path", "ToolResult.display"]
    notes: "Write/edit operations require approval unless auto-approved."
  - name: "SearchWeb / FetchURL"
    call_visible: true
    result_visible: true
    metadata: ["ToolCall.function.arguments", "ToolResult.return_value"]
    notes: "Search requires configured service; fetch falls back locally when service is absent."
  - name: "EnterPlanMode / ExitPlanMode"
    call_visible: true
    result_visible: true
    metadata: ["set_plan_mode", "StatusUpdate.plan_mode", "PlanDisplay.file_path"]
    notes: "Wire plan tools are hidden unless client declares `supports_plan_mode`."
  - name: "External tools"
    call_visible: true
    result_visible: true
    metadata: ["initialize.external_tools", "ToolCallRequest.id", "ToolCallRequest.name", "ToolCallRequest.arguments"]
    notes: "Wire client registers tools during initialize and must answer tool-call requests."
completion:
  success_event: "JSON-RPC success response to `prompt` with `result.status = finished`; process exit 0 for print success."
  failure_event: "JSON-RPC error response to `prompt`, `result.status = cancelled|max_steps_reached`, `StepInterrupted`, or process exit 1/75 in print mode."
  exit_code_reliable: true
  result_fields: ["result.status", "result.steps", "error.code", "error.message"]
  cost_fields: []
  usage_fields: ["StatusUpdate.token_usage", "StatusUpdate.context_tokens", "StatusUpdate.max_context_tokens", "StatusUpdate.context_usage"]
  notes: "For Wire, the per-request response is more precise than process exit because the server may stay alive after a turn. Print mode documents exit 0/1/75."
blocking_behavior:
  permissions: configurable
  questions: configurable
  tool_approvals: configurable
  notes: "Print mode implicitly runs AFK: tool calls are auto-approved and AskUserQuestion/plan switches are handled automatically. Wire can either negotiate questions and approvals or use `--afk`/`--yolo`; on Wire shutdown unresolved foreground approvals are rejected, questions resolve to empty answers, external tools return a ToolError, and hooks allow."
subagents:
  supported: true
  start_visible: true
  stop_visible: true
  nested_events_visible: true
  prompt_injection_supported: true
  metadata_fields: ["SubagentEvent.parent_tool_call_id", "SubagentEvent.agent_id", "SubagentEvent.subagent_type", "ToolCall.function.arguments.prompt", "ToolCall.function.arguments.model"]
  notes: "Subagents run through the `Agent` tool and store their own context/wire logs; prompt injection is through the Agent tool prompt or custom agent files, not a separate protocol field."
use_cases:
  - name: plan_cap_approaching
    detectable: false
    event_types: []
    fields: []
    hook_parity: "none verified"
    notes: "No typed account-plan cap event found."
  - name: plan_capped
    detectable: false
    event_types: ["prompt error", "print exit 1"]
    fields: ["error.message"]
    hook_parity: "none verified"
    notes: "Quota exhaustion is documented as print exit 1 but Wire exposes it as provider/config error text, not a dedicated cap schema."
  - name: no_funds
    detectable: false
    event_types: ["prompt error", "print exit 1"]
    fields: ["error.message"]
    hook_parity: "none verified"
    notes: "No dedicated insufficient-balance event found."
  - name: auth
    detectable: true
    event_types: ["prompt error", "print exit 1"]
    fields: ["error.code", "error.message", "initialize.server.version"]
    hook_parity: "none verified"
    notes: "Print docs classify authentication failures as non-retryable exit 1; Wire lacks an auth-kind field."
  - name: permission_read_denied
    detectable: true
    event_types: ["ToolResult"]
    fields: ["tool_call_id", "return_value", "ToolCall.function.arguments.path"]
    hook_parity: "possible through hooks input_data if subscribed"
    notes: "Infer from read tool result/error; no dedicated denial event."
  - name: permission_write_denied
    detectable: true
    event_types: ["ApprovalResponse", "HookResolved", "ToolResult"]
    fields: ["response", "feedback", "action", "reason", "display.path", "tool_call_id"]
    hook_parity: "Wire HookRequest/HookResolved can carry hook block details"
    notes: "Approval rejection is explicit; path may be in display blocks or tool arguments."
  - name: tokens_consumed
    detectable: true
    event_types: ["StatusUpdate"]
    fields: ["token_usage", "context_tokens", "max_context_tokens", "context_usage"]
    hook_parity: "none verified"
    notes: "TokenUsage shape comes from the `kosong.chat_provider` type; units are tokens."
  - name: model_used
    detectable: false
    event_types: []
    fields: []
    hook_parity: "none verified"
    notes: "Requested model is launch/config metadata; resolved provider/model is not reliably emitted in Wire."
  - name: model_fallback
    detectable: false
    event_types: []
    fields: []
    hook_parity: "none verified"
    notes: "No fallback event found."
  - name: human_in_loop
    detectable: true
    event_types: ["ApprovalRequest", "QuestionRequest", "HookRequest"]
    fields: ["params.type", "params.payload.id", "params.payload.questions", "params.payload.action"]
    hook_parity: "HookRequest is itself a programmable human/policy callback surface"
    notes: "If questions are not negotiated, AskUserQuestion is hidden rather than emitted."
  - name: session_resumable
    detectable: true
    event_types: ["launch metadata", "wire.jsonl replay"]
    fields: ["session_id from argv/session directory", "replay.result.events", "replay.result.requests"]
    hook_parity: "SessionStart hook has session_id"
    notes: "Claudine should capture session ID from launch/resume and filesystem path, not wait for Wire initialize."
  - name: subagent_prompt_injection
    detectable: true
    event_types: ["ToolCall"]
    fields: ["function.name", "function.arguments.prompt", "function.arguments.subagent_type", "function.arguments.model"]
    hook_parity: "Agent tool hooks can inspect/block"
    notes: "The caller can steer subagents by changing the Agent tool prompt or custom agent definitions."
headless_constraints:
  - constraint: "Wire is bidirectional; stdout is not just events from the agent."
    mitigation: "Implement a JSON-RPC peer that sends initialize/prompt and answers request messages."
    notes: "ApprovalRequest, QuestionRequest, ToolCallRequest, and HookRequest can block the turn until answered."
  - constraint: "`--prompt` is ignored in Wire mode."
    mitigation: "Send a JSON-RPC `prompt` request after initialize."
    notes: "The CLI logs a warning but does not consume the prompt."
  - constraint: "Print `stream-json` is a lossy projection."
    mitigation: "Use Wire for lifecycle supervision; use print JSONL only for simple automation."
    notes: "Print mode suppresses many control/status events and emits whole assistant messages at boundaries."
  - constraint: "Questions and plan mode require capability negotiation in Wire."
    mitigation: "Declare `capabilities.supports_question` and `capabilities.supports_plan_mode` when Claudine can service them; otherwise use `--afk`."
    notes: "Unsupported tools are hidden from the model."
  - constraint: "Model/provider/auth metadata is not emitted as a stable event."
    mitigation: "Record launch flags, effective config, and `kimi info --json` output where available."
    notes: "Do not infer model from assistant text."
quirks:
  - "The best structured mode is a bidirectional JSON-RPC line protocol, not an `--output-format` value."
  - "Wire payload discriminators are in `params.type` / nested `event.type`, while transport discrimination is JSON-RPC `method`."
  - "Print `stream-json` says JSONL but emits whole assistant/tool messages plus non-message records like notifications and plans."
  - "Print mode implicitly uses AFK behavior; this is convenient for CI but can execute edits and shell commands without approval."
  - "Wire initialize exposes server version and protocol version but not session ID, cwd, model, provider, or auth kind."
  - "Subagent events can be nested under `SubagentEvent.event`; parsers need recursion."
  - "Some event payload classes come from dependency packages (`kosong`), so the complete schema spans more than `kimi_cli/wire/types.py`."
gaps:
  - "No official JSON Schema, OpenAPI, AsyncAPI, or versioned schema artifact was found for Wire."
  - "Could not verify a stable field for resolved provider/model or auth kind in the Wire stream."
  - "Could not verify dedicated quota-near-cap, quota-exhausted, no-funds, or model-fallback events."
  - "Timestamp units/timezone for `Notification.created_at` are not formally documented."
  - "Local installed `kimi` was version 0.14.0 and did not support `kimi info --json`; local runtime fixtures were not captured from the current CLI."
  - "Exact `TokenUsage` nested fields come from the external `kosong` package and were not independently schema-dumped."
claudine_strategy:
  preferred_invocation: "kimi --wire --work-dir <repo> --afk"
  required_flags: ["--wire", "--work-dir <repo>", "--afk unless Claudine implements approval/question responses"]
  conflicting_flags: ["--print", "--quiet", "--final-message-only", "--output-format", "--input-format", "--prompt for Wire"]
  parser_notes: "Parse stdout as JSON-RPC lines. Drive initialize first, then prompt. Classify by `method`; for `event` and `request`, classify by `params.type`. Recurse into `SubagentEvent.event`. Join tool calls/results by `id` and `tool_call_id`; join approvals/questions/hooks by request `id` and response `request_id`."
  wrapper_notes: "Keep launch metadata beside stream records because Wire does not emit enough session/model/auth metadata. Parse stderr/logs only for diagnostics. Use print `stream-json` only as a fallback when Claudine wants a one-shot transcript and accepts missing live control events."
data_format: jsonrpc_lines
changes:
  - "2026-07-02: Reworked Kimi research into schema-backed non-interactive format; updated Wire protocol from 1.8-era notes to 1.10 docs/source; added required body sections and normalized frontmatter."
requires_claudine_update: true
reason: "Claudine's Kimi strategy should prefer Wire JSON-RPC lines with a real bidirectional peer; print `stream-json` is insufficient for wrapper-grade supervision."
---

## Summary

Kimi Code CLI can be run non-interactively with structured output, but the best mode is not a conventional `--output-format` stream. Claudine should prefer `kimi --wire`, which exposes Kimi's bidirectional Wire protocol as JSON-RPC 2.0 messages framed one JSON object per line on stdin/stdout. Wire is the only mode that can show live lifecycle state, request approvals, ask structured questions, report hooks, stream token/context updates, expose MCP startup snapshots, and carry nested subagent events.

`kimi --print --output-format stream-json` remains useful as a simple automation fallback, but it is a lossy projection of Wire messages. It emits whole assistant/tool-style JSONL messages and selected display objects, not the full event/request protocol. The main parser risk is that Wire requires Claudine to be a peer: it must send JSON-RPC requests and respond to Kimi's `request` messages, not just tail stdout.

## Non-Interactive Entry Points

Kimi has two relevant non-interactive entry points. Print mode is command-shaped: pass a prompt with `--prompt`/`-p`, pipe text to stdin, or use JSONL user messages with `--input-format stream-json`. The official print docs define it as non-interactive and suitable for scripting; they also state that print mode implicitly runs in AFK mode, so tool calls are auto-approved and interactive questions/plan switches are handled automatically.

Wire mode is protocol-shaped:

```bash
kimi --wire --work-dir /path/to/repo --afk
```

After launch, the client sends an optional `initialize` request, then a `prompt` request:

```json
{"jsonrpc":"2.0","id":"init-1","method":"initialize","params":{"protocol_version":"1.10","client":{"name":"claudine"},"capabilities":{"supports_question":false,"supports_plan_mode":false}}}
```

```json
{"jsonrpc":"2.0","id":"turn-1","method":"prompt","params":{"user_input":"Fix the failing tests."}}
```

`--prompt` is not the prompt channel for Wire. The current CLI source logs that Wire ignores the prompt argument; Claudine must send a JSON-RPC `prompt` request instead. Resume and workspace controls are still normal CLI flags: `--work-dir`, `--add-dir`, `--continue`, and `--session <id>`.

## Output Formats

Kimi's output modes differ in kind, not only in detail:

| Mode | Selector | Framing | Streams live? | Claudine preference | Notes |
| --- | --- | --- | --- | --- | --- |
| Wire | `--wire` | JSON-RPC lines | Yes | Prefer | Bidirectional protocol over stdin/stdout. |
| Print JSONL | `--print --output-format stream-json` | JSONL | Partly | Fallback | Whole assistant/tool messages plus notifications/plans; many Wire events omitted. |
| Print text | `--print --output-format text` | Text | Partly | Avoid for parsing | Human/Rich output. |
| Final only | `--quiet` or `--final-message-only` | Text or JSONL final assistant message | No | Avoid for supervision | Drops intermediate state. |

Wire is the best Claudine format because it exposes the interaction surface that matters while a run is active. The response stream contains `event` notifications such as `StatusUpdate`, `StepBegin`, `ToolCall`, `ToolResult`, `PlanDisplay`, `HookTriggered`, and `SubagentEvent`. The same stdout stream can also contain server-to-client `request` messages such as `ApprovalRequest`, `QuestionRequest`, `ToolCallRequest`, and `HookRequest`, which require a JSON-RPC response before the run can continue.

Print `stream-json` is easier to consume but materially weaker. The source JSON printer merges content parts into assistant messages, flushes them at step boundaries, emits tool results as tool messages, and ignores many control events. A parser can see tool calls and tool results, but it loses most session lifecycle, approval, hook, MCP, retry, compaction, and nested subagent structure.

## Schema Sources

Kimi does not publish a standalone JSON Schema, OpenAPI document, or AsyncAPI document for Wire. The best schema evidence is provider-authored source plus the official Wire page.

The official Wire docs describe JSON-RPC envelopes, protocol version `1.10`, request methods, response shapes, event/request envelopes, display blocks, and capability negotiation. The source is still important because the actual runtime schema is Pydantic:

- [`src/kimi_cli/wire/types.py`](https://github.com/MoonshotAI/kimi-cli/blob/main/src/kimi_cli/wire/types.py) defines the event/request unions and payload models.
- [`src/kimi_cli/wire/jsonrpc.py`](https://github.com/MoonshotAI/kimi-cli/blob/main/src/kimi_cli/wire/jsonrpc.py) defines JSON-RPC inbound/outbound message types and method sets.
- [`src/kimi_cli/wire/protocol.py`](https://github.com/MoonshotAI/kimi-cli/blob/main/src/kimi_cli/wire/protocol.py) defines the current protocol constant.
- [`src/kimi_cli/ui/print/visualize.py`](https://github.com/MoonshotAI/kimi-cli/blob/main/src/kimi_cli/ui/print/visualize.py) defines the print `stream-json` projection.

The source types are authoritative enough for Claudine parser work, but not complete in one file. Some payloads come from Kimi's dependencies, especially `kosong.message` and `kosong.tooling`, so parser generation should preserve unknown nested fields.

## IO Contract

In Wire mode, stdout is parse-only JSON-RPC lines and stdin is also part of the protocol. Claudine must keep stdin open long enough to send `initialize`, `prompt`, possible responses to Kimi requests, `cancel`, and optional `steer` messages. Stderr should be treated as diagnostics; Kimi also writes runtime logs under the Kimi share directory.

In print mode, stdout is either human text or JSONL depending on `--output-format`. Stdin is prompt text for `--input-format text`, or user-message JSONL for `--input-format stream-json`. Print mode can read multiple JSONL user messages until EOF, but this is not the same as Wire because there is no JSON-RPC request/response channel for approvals, hooks, or external tools.

## Stream Contract

Wire has two discriminator layers. The transport discriminator is JSON-RPC's `method`: `event`, `request`, or a response with `id` and either `result` or `error`. For `event` and `request`, Kimi's Wire envelope uses `params.type` plus `params.payload`.

Normal turn flow is:

```mermaid
sequenceDiagram
    participant C as Claudine
    participant K as kimi --wire
    C->>K: initialize
    K-->>C: initialize result
    C->>K: prompt
    K-->>C: event TurnBegin
    K-->>C: event StepBegin / content / tool events / StatusUpdate
    K-->>C: request ApprovalRequest or QuestionRequest
    C->>K: response ApprovalResponse or QuestionResponse
    K-->>C: event TurnEnd
    K-->>C: prompt result {status}
```

The prompt response is the terminal record for a turn. `TurnEnd` is useful, but the source model warns it may be omitted if the turn is interrupted. Tool calls and results are joined by tool-call ID. Request/response families use the JSON-RPC `id` plus payload fields such as `request_id` and `tool_call_id`.

Unknown `params.type` values should not crash Claudine. The protocol has changed over time and display blocks include an explicit unknown-block fallback. Claudine should preserve raw payloads, classify them as unknown, and continue unless the unknown record is a request that requires a response.

## Session Metadata

Wire's initialize result gives protocol version, server name/version, slash commands, capabilities, hook info, and external-tool registration results. It does not provide a complete wrapper metadata envelope. Session ID, working directory, selected model, provider, auth source, permission mode, and sandbox/root information must be tracked from launch arguments, config, session files, and Claudine wrapper state.

MCP is more visible. `StatusUpdate` can include `mcp_status` with `loading`, connected/total counts, total tool count, and per-server snapshots with `name`, `status`, and `tools`. This is a useful live signal during startup and tool discovery.

## Event Families

Wire event families that matter for Claudine:

| Family | Concrete records | Parser value |
| --- | --- | --- |
| Turn/step lifecycle | `TurnBegin`, `TurnEnd`, `StepBegin`, `StepInterrupted`, `StepRetry` | Progress, retry classification, step counters. |
| Assistant/reasoning | `TextPart`, `ThinkPart`, other content parts | Live transcript and reasoning when available. |
| Tools | `ToolCall`, `ToolCallPart`, `ToolResult`, `ToolCallRequest` | Tool start/input/result and external tool execution. |
| Permissions/questions | `ApprovalRequest`, `ApprovalResponse`, `QuestionRequest`, `QuestionResponse` | Human-in-loop detection and deterministic approvals. |
| Status/usage | `StatusUpdate` | Context usage, token usage, message IDs, plan mode, MCP snapshots. |
| Plans | `PlanDisplay`, `set_plan_mode`, `StatusUpdate.plan_mode` | Plan content and read-only mode state. |
| Hooks | `HookRequest`, `HookTriggered`, `HookResolved` | Policy callbacks and hook timing/allow/block outcome. |
| Subagents | `SubagentEvent`, `Agent` tool calls/results | Nested agent observability. |
| Background/notifications | `Notification`, `BtwBegin`, `BtwEnd` | Background task and side-question status. |
| Maintenance | `CompactionBegin`, `CompactionEnd`, `MCPLoadingBegin`, `MCPLoadingEnd` | Context and MCP lifecycle signals. |

Print JSONL collapses this substantially. It primarily emits assistant messages, tool messages, notifications, and plan displays.

## Tools

The default Kimi agent exposes `Agent`, `AskUserQuestion`, `SetTodoList`, `Shell`, `ReadFile`, `ReadMediaFile`, `Glob`, `Grep`, `WriteFile`, `StrReplaceFile`, `SearchWeb`, `FetchURL`, `EnterPlanMode`, `ExitPlanMode`, `TaskList`, `TaskOutput`, and `TaskStop`. The docs also describe `Think` and the experimental `okabe` agent's `SendDMail`.

Wire exposes native tool calls as `ToolCall` and results as `ToolResult`; incremental tool call fragments can arrive as `ToolCallPart`. Built-in writes and shell commands can produce `ApprovalRequest` before execution. Display blocks are important: diffs carry paths and old/new text, shell blocks carry command text, and todo blocks carry item state. Claudine should not rely only on tool result text when a structured display block is present.

External tools are a Wire-specific feature. Claudine can declare tools in `initialize.external_tools`; Kimi then sends `ToolCallRequest` messages and waits for Claudine to return a `ToolResult`.

## Completion and Exit Status

For Wire, completion is per JSON-RPC request. A `prompt` success response contains `result.status`, documented as `finished`, `cancelled`, or `max_steps_reached`; `steps` may be present for max-steps completion. Errors return JSON-RPC error objects. The server process can remain alive after one turn, so process exit is not the right success signal for a single prompt.

For print mode, the docs define exit codes: `0` for success, `1` for non-retryable failures such as configuration errors, authentication failures, quota exhaustion, and permanent errors, and `75` for retryable failures such as 429, 5xx, and timeouts. The source classifies provider connection/timeout/empty-response errors and selected HTTP status codes as retryable.

## Blocking Behavior

Print mode is designed not to block on a human. It implicitly enables AFK behavior, auto-approves tool calls, and auto-dismisses `AskUserQuestion`. This is automation-friendly but high-trust: file modifications and shell commands can execute without approval.

Wire can block unless Claudine responds. `ApprovalRequest`, `QuestionRequest`, `ToolCallRequest`, and `HookRequest` are JSON-RPC requests from Kimi to the client. Kimi's shutdown path resolves unresolved foreground approvals as reject, unresolved questions as empty answers, external tool calls as a tool error, and hooks as allow, but Claudine should not depend on shutdown cleanup for normal control flow.

Question and plan tools are capability-gated. If the client does not declare `supports_question`, Kimi hides `AskUserQuestion`; if it does not declare `supports_plan_mode`, plan-mode tools are hidden. Claudine can use this to avoid unsupported human-in-loop behavior.

## Subagents

Subagents are supported non-interactively through the `Agent` tool. The docs state that subagents run in isolated contexts, can run in foreground or background, can be resumed, and have per-instance storage under the session directory. Wire wraps nested events in `SubagentEvent` with `parent_tool_call_id`, `agent_id`, and `subagent_type`, and the nested event itself has its own `type` and `payload`.

Claudine can steer subagents by controlling the root prompt, the `Agent` tool prompt when it appears, or custom agent files. There is no separate Wire field named "subagent prompt injection"; it is ordinary agent/tool input plus custom agent configuration.

## Use Case Detection

| Use case | Detectability | Events and fields | Caveat |
| --- | --- | --- | --- |
| `tokens_consumed` | Good | `StatusUpdate.token_usage`, `context_tokens`, `max_context_tokens`, `context_usage` | Nested `TokenUsage` fields come from `kosong`. |
| `human_in_loop` | Good | `ApprovalRequest`, `QuestionRequest`, `HookRequest` | Questions only appear if negotiated. |
| `permission_write_denied` | Partial | `ApprovalResponse.response = reject`, `HookResolved.action = block`, tool result errors | Path may be in display blocks or tool args. |
| `permission_read_denied` | Partial | Read tool `ToolResult` error/result plus prior `ToolCall` args | No dedicated denial event. |
| `auth` | Partial | JSON-RPC error text or print exit `1` | Auth kind is not emitted. |
| `plan_capped` / `no_funds` | Weak | Error text or print exit `1` | No dedicated quota/funds schema found. |
| `model_used` | Weak | Launch/config metadata | Not reliably emitted in stream. |
| `session_resumable` | Good with wrapper state | `--session`, session directory, `wire.jsonl`, `replay` counts | Not emitted early by initialize. |
| `subagent_prompt_injection` | Good with tool parsing | `Agent` tool arguments and custom agent files | No special event name. |

## Headless Constraints

The biggest constraint is that Wire is a live protocol. A wrapper that only reads stdout can deadlock when Kimi sends a request that expects a response. Claudine either needs to run with `--afk` and with unsupported question/plan capabilities disabled, or it needs to implement the request handlers deliberately.

Print `stream-json` is safer to pipe but weaker for supervision. It is acceptable for a one-shot final transcript or rough tool transcript, but it should not be the primary Claudine integration because it hides or merges too many operational signals.

The current installed local `kimi` on this host reported `0.14.0` and did not support `kimi info --json`, so this document relies on current official docs and GitHub source rather than local runtime captures.

## Timeline

Kimi's structured-output surface has grown from simple print JSONL into a protocol surface:

| Date or version | Change | Wrapper significance |
| --- | --- | --- |
| Wire 1.1 | `initialize` added | Clients can negotiate protocol details and external tools. |
| Wire 1.3 | `replay` added | Saved `wire.jsonl` can be replayed for UI/recovery. |
| Wire 1.4 | `steer` and structured questions documented | Human-in-loop can become protocol-driven. |
| Wire 1.6 | Approval response feedback added | Rejections can include model guidance. |
| Wire 1.8 | Display-block changes such as diff summaries | Display schemas can drift and need tolerant parsing. |
| Wire 1.10 | Current documented/source protocol version | Claudine should target this version while tolerating older/newer payloads. |

## Quirks and Gaps

Kimi has unusually useful Wire visibility, but it is not a complete wrapper metadata feed. Claudine must carry launch/config metadata for model, provider, auth, workdir, roots, and permission mode. The stream has strong tool and lifecycle events, but no dedicated account quota, no-funds, model-fallback, or auth-kind events.

The protocol also spans packages. `types.py` is the right starting point, but content parts, token usage, tool calls, tool results, and display blocks also come from imported models. Parser code should preserve unknown fields and not assume the public docs are exhaustive.

## Claudine Integration Notes

Recommended command:

```bash
kimi --wire --work-dir "$REPO" --afk
```

Use `--session <id>` or `--continue` only when Claudine intentionally wants resume semantics. Send `initialize` with Claudine client metadata. If Claudine cannot answer human questions, set `supports_question: false`; if it cannot service plan-mode interactions, set `supports_plan_mode: false`. Then send `prompt`.

Parse stdout as JSON-RPC lines. Classify by JSON-RPC `method`, then by `params.type` for `event` and `request`. Recurse into `SubagentEvent.event`. Join `ToolCall` and `ToolResult` by `id`/`tool_call_id`; join approval, question, and hook requests by JSON-RPC `id` and payload `request_id`. Keep stderr/logs for diagnostics only.

Avoid `--quiet`, `--final-message-only`, and text output for Claudine supervision. Use `--print --output-format stream-json` only when implementing a simpler fallback that does not need approvals, questions, hooks, MCP status, retries, or full subagent visibility.

## Changelog

- 2026-07-02: Reworked Kimi research into the schema-backed non-interactive format, updated Wire to protocol `1.10`, and made Wire JSON-RPC the explicit Claudine preference.

## Sources

- [Kimi Code CLI Wire mode documentation](https://moonshotai.github.io/kimi-cli/en/customization/wire-mode.html)
- [Kimi Code CLI Print mode documentation](https://moonshotai.github.io/kimi-cli/en/customization/print-mode.html)
- [Kimi command reference](https://moonshotai.github.io/kimi-cli/en/reference/kimi-command.html)
- [Kimi config files documentation](https://moonshotai.github.io/kimi-cli/en/configuration/config-files.html)
- [Kimi data locations documentation](https://moonshotai.github.io/kimi-cli/en/configuration/data-locations.html)
- [Kimi agents and subagents documentation](https://moonshotai.github.io/kimi-cli/en/customization/agents.html)
- [Wire Pydantic types source](https://github.com/MoonshotAI/kimi-cli/blob/main/src/kimi_cli/wire/types.py)
- [Wire JSON-RPC source](https://github.com/MoonshotAI/kimi-cli/blob/main/src/kimi_cli/wire/jsonrpc.py)
- [Wire protocol version source](https://github.com/MoonshotAI/kimi-cli/blob/main/src/kimi_cli/wire/protocol.py)
- [Print JSONL projection source](https://github.com/MoonshotAI/kimi-cli/blob/main/src/kimi_cli/ui/print/visualize.py)
- [Print runtime source](https://github.com/MoonshotAI/kimi-cli/blob/main/src/kimi_cli/ui/print/__init__.py)
