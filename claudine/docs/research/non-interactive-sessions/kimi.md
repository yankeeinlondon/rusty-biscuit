---
$schema: ./_schema.yaml
created: 2026-04-06
last_updated: 2026-07-03
agent: codex
model: default
docs: https://moonshotai.github.io/kimi-cli/en/customization/wire-mode.html
invocation:
  - command: "kimi --wire --work-dir <repo>"
    stdin_support: true
    prompt_arg: "Send a JSON-RPC `prompt` request on stdin after `initialize`; `--prompt` is not the Wire prompt channel."
    notes: "Starts a Wire JSON-RPC line-protocol server for a fresh session."
  - command: "kimi --wire --work-dir <repo> --session <session-id>"
    stdin_support: true
    prompt_arg: "Send a JSON-RPC `prompt` request on stdin."
    notes: "Resumes the named session; Kimi docs say an unknown ID creates a new session."
  - command: "kimi --wire --work-dir <repo> --continue"
    stdin_support: true
    prompt_arg: "Send a JSON-RPC `prompt` request on stdin."
    notes: "Continues the previous session for the selected work directory."
  - command: "kimi --print -p <prompt> --output-format stream-json"
    stdin_support: false
    prompt_arg: "`--prompt`, `-p`, `--command`, or `-c`."
    notes: "Simple non-interactive fallback. Print mode exits after the task and implicitly enables AFK behavior."
  - command: "printf '%s\n' '{\"role\":\"user\",\"content\":\"...\"}' | kimi --print --input-format stream-json --output-format stream-json"
    stdin_support: true
    prompt_arg: "User-role JSONL messages on stdin until EOF."
    notes: "Request/reply print mode, not Wire. Output is JSONL message projection."
output_formats:
  - name: "Wire"
    cli_value: "--wire"
    stream: true
    format: jsonrpc_lines
    description: "Bidirectional JSON-RPC 2.0 over newline-delimited JSON on stdin/stdout."
    side_effects: "Turns the CLI into a protocol peer. `--input-format`, `--output-format`, and final-message-only print options do not select Wire payloads."
  - name: "print stream-json"
    cli_value: "stream-json"
    stream: true
    format: jsonl
    description: "Print-mode JSONL made from assistant messages, tool messages, notifications, and plan display records."
    side_effects: "Lossy projection of Wire. Assistant content and tool-call deltas are buffered into whole messages at step boundaries."
  - name: "print text"
    cli_value: "text"
    stream: true
    format: text
    description: "Human-oriented print-mode output."
    side_effects: "Not a stable parser contract; source uses Rich printing for non-final text output."
  - name: "quiet/final message only"
    cli_value: "--quiet or --final-message-only"
    stream: false
    format: text
    description: "Only final assistant text."
    side_effects: "Suppresses intermediate progress, tool calls, tool results, notifications, and most lifecycle evidence."
schema_sources:
  - url: "https://moonshotai.github.io/kimi-cli/en/customization/wire-mode.html"
    schema_type: other
    formal: true
    notes: "Official Wire protocol documentation with TypeScript-style request, response, event, request, and display-block definitions; current protocol version is 1.10."
  - url: "https://github.com/MoonshotAI/kimi-cli/blob/main/src/kimi_cli/wire/types.py"
    schema_type: other
    formal: true
    notes: "Provider source Pydantic models for Wire event/request unions and payload fields."
  - url: "https://github.com/MoonshotAI/kimi-cli/blob/main/src/kimi_cli/wire/jsonrpc.py"
    schema_type: other
    formal: true
    notes: "Provider source Pydantic models for JSON-RPC envelopes, inbound/outbound method sets, statuses, and error codes."
  - url: "https://github.com/MoonshotAI/kimi-cli/blob/main/src/kimi_cli/ui/print/visualize.py"
    schema_type: other
    formal: false
    notes: "Source projection for print `stream-json`; useful for knowing what is omitted or buffered."
cli_params:
  - flag: "--wire"
    value: ""
    description: "Runs Wire server mode for structured bidirectional integration."
    example: "kimi --wire --work-dir /repo"
  - flag: "--print"
    value: ""
    description: "Runs non-interactively in print mode and implicitly enables AFK."
    example: "kimi --print -p \"summarize\" --output-format stream-json"
  - flag: "--input-format"
    value: "text | stream-json"
    description: "Print-only input format; `stream-json` reads user-role JSONL from stdin until EOF."
    example: "kimi --print --input-format stream-json --output-format stream-json"
  - flag: "--output-format"
    value: "text | stream-json"
    description: "Print-only output selector."
    example: "kimi --print -p \"task\" --output-format stream-json"
  - flag: "--final-message-only"
    value: ""
    description: "Print-only lossy final-answer output."
    example: "kimi --print -p \"commit message\" --final-message-only"
  - flag: "--quiet"
    value: ""
    description: "Shortcut for `--print --output-format text --final-message-only`."
    example: "kimi --quiet -p \"commit message\""
  - flag: "--prompt, -p, --command, -c"
    value: "TEXT"
    description: "One-shot prompt for shell/print mode; Wire clients should send JSON-RPC `prompt` instead."
    example: "kimi --print -p \"explain this repo\""
  - flag: "--work-dir, -w"
    value: "PATH"
    description: "Sets working directory/root for file operations."
    example: "kimi --wire --work-dir /repo"
  - flag: "--add-dir"
    value: "PATH"
    description: "Adds accessible workspace roots and persists them in session state."
    example: "kimi --wire --add-dir /shared"
  - flag: "--session, --resume, -S, -r"
    value: "[ID]"
    description: "Resumes a session by ID; ID-less picker is shell-only."
    example: "kimi --wire --session abc123"
  - flag: "--continue, -C"
    value: ""
    description: "Continues the previous session for the working directory."
    example: "kimi --wire --continue"
  - flag: "--model, -m"
    value: "NAME"
    description: "Overrides the configured default model for this process."
    example: "kimi --wire --model kimi-for-coding"
  - flag: "--thinking / --no-thinking"
    value: ""
    description: "Overrides thinking mode when model capabilities allow it."
    example: "kimi --wire --thinking"
  - flag: "--yolo, --yes, --auto-approve"
    value: ""
    description: "Auto-approves tool calls, while user questions may still be reachable."
    example: "kimi --wire --yolo"
  - flag: "--afk"
    value: ""
    description: "Away-from-keyboard mode: auto-approves tools and auto-dismisses AskUserQuestion."
    example: "kimi --wire --afk"
  - flag: "--plan"
    value: ""
    description: "Starts or resumes in read-only plan mode."
    example: "kimi --wire --plan"
  - flag: "--max-steps-per-turn"
    value: "N"
    description: "Overrides loop-control max steps for a turn."
    example: "kimi --print -p \"task\" --max-steps-per-turn 20"
  - flag: "--max-retries-per-step"
    value: "N"
    description: "Overrides retry count per step."
    example: "kimi --wire --max-retries-per-step 2"
  - flag: "--max-ralph-iterations"
    value: "N"
    description: "Runs Ralph Loop iterations; `0` disables and `-1` is unlimited."
    example: "kimi --print -p \"iterate\" --max-ralph-iterations 3"
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
    description: "Loads inline MCP config JSON; repeatable."
    example: "kimi --wire --mcp-config '{\"mcpServers\":{}}'"
  - flag: "--agent"
    value: "NAME"
    description: "Selects a built-in agent."
    example: "kimi --wire --agent default"
  - flag: "--agent-file"
    value: "PATH"
    description: "Selects a custom agent file; mutually exclusive with `--agent`."
    example: "kimi --wire --agent-file ./agent.md"
  - flag: "--skills-dir"
    value: "PATH"
    description: "Adds skills directories; repeatable."
    example: "kimi --wire --skills-dir ./.kimi/skills"
  - flag: "--verbose"
    value: ""
    description: "Outputs detailed runtime information; not a structured stream selector."
    example: "kimi --verbose --print -p \"task\""
  - flag: "--debug"
    value: ""
    description: "Writes TRACE-level logs to Kimi's log file."
    example: "kimi --debug --wire"
config_files:
  - os: macos
    scope: user
    path: "~/.kimi/config.toml"
    format: toml
    effect: "Default providers, models, loop control, background behavior, hooks, MCP client settings, default YOLO/plan/thinking, theme, telemetry."
    notes: "Default user config. `--config-file` replaces this file for the process; `--config` bypasses file loading with inline JSON/TOML."
  - os: linux
    scope: user
    path: "~/.kimi/config.toml"
    format: toml
    effect: "Default providers, models, loop control, background behavior, hooks, MCP client settings, default YOLO/plan/thinking, theme, telemetry."
    notes: "Default user config. `--config-file` replaces this file for the process; `--config` bypasses file loading with inline JSON/TOML."
  - os: windows
    scope: user
    path: "%USERPROFILE%\\.kimi\\config.toml"
    format: toml
    effect: "Default providers, models, loop control, background behavior, hooks, MCP client settings, default YOLO/plan/thinking, theme, telemetry."
    notes: "Docs express the location as `~/.kimi/config.toml`; on Windows this resolves under the user's home directory."
  - os: macos
    scope: user
    path: "~/.kimi/config.json"
    format: json
    effect: "Legacy config source."
    notes: "Migrated to TOML with backup `config.json.bak` when TOML config does not exist."
  - os: linux
    scope: user
    path: "~/.kimi/config.json"
    format: json
    effect: "Legacy config source."
    notes: "Migrated to TOML with backup `config.json.bak` when TOML config does not exist."
  - os: windows
    scope: user
    path: "%USERPROFILE%\\.kimi\\config.json"
    format: json
    effect: "Legacy config source."
    notes: "Migrated to TOML with backup `config.json.bak` when TOML config does not exist."
  - os: macos
    scope: user
    path: "~/.kimi/mcp.json"
    format: json
    effect: "Default MCP server configuration loaded when no explicit MCP config is supplied."
    notes: "Additional MCP config files and inline MCP JSON are appended for the invocation."
  - os: linux
    scope: user
    path: "~/.kimi/mcp.json"
    format: json
    effect: "Default MCP server configuration loaded when no explicit MCP config is supplied."
    notes: "Additional MCP config files and inline MCP JSON are appended for the invocation."
  - os: windows
    scope: user
    path: "%USERPROFILE%\\.kimi\\mcp.json"
    format: json
    effect: "Default MCP server configuration loaded when no explicit MCP config is supplied."
    notes: "Additional MCP config files and inline MCP JSON are appended for the invocation."
  - os: macos
    scope: user
    path: "~/.kimi/sessions/<work-dir-hash>/<session-id>/wire.jsonl"
    format: other
    effect: "Persistent Wire event/request log used for replay, export, and visualizer workflows."
    notes: "Not startup configuration, but important for session recovery and fixture capture."
  - os: linux
    scope: user
    path: "~/.kimi/sessions/<work-dir-hash>/<session-id>/wire.jsonl"
    format: other
    effect: "Persistent Wire event/request log used for replay, export, and visualizer workflows."
    notes: "Not startup configuration, but important for session recovery and fixture capture."
  - os: windows
    scope: user
    path: "%USERPROFILE%\\.kimi\\sessions\\<work-dir-hash>\\<session-id>\\wire.jsonl"
    format: other
    effect: "Persistent Wire event/request log used for replay, export, and visualizer workflows."
    notes: "Not startup configuration, but important for session recovery and fixture capture."
  - os: macos
    scope: user
    path: "~/.kimi/sessions/<work-dir-hash>/<session-id>/state.json"
    format: json
    effect: "Session state: approval state, plan mode, extra roots, subagent metadata, and plan identifiers."
    notes: "Resumed sessions restore these values."
  - os: linux
    scope: user
    path: "~/.kimi/sessions/<work-dir-hash>/<session-id>/state.json"
    format: json
    effect: "Session state: approval state, plan mode, extra roots, subagent metadata, and plan identifiers."
    notes: "Resumed sessions restore these values."
  - os: windows
    scope: user
    path: "%USERPROFILE%\\.kimi\\sessions\\<work-dir-hash>\\<session-id>\\state.json"
    format: json
    effect: "Session state: approval state, plan mode, extra roots, subagent metadata, and plan identifiers."
    notes: "Resumed sessions restore these values."
  - os: macos
    scope: repo
    path: ".kimi/AGENTS.md"
    format: text
    effect: "Project instruction file that can affect agent behavior."
    notes: "Affects prompts and behavior, not stream framing."
  - os: linux
    scope: repo
    path: ".kimi/AGENTS.md"
    format: text
    effect: "Project instruction file that can affect agent behavior."
    notes: "Affects prompts and behavior, not stream framing."
  - os: windows
    scope: repo
    path: ".kimi\\AGENTS.md"
    format: text
    effect: "Project instruction file that can affect agent behavior."
    notes: "Affects prompts and behavior, not stream framing."
env_vars:
  - name: "KIMI_SHARE_DIR"
    effect: "Moves Kimi's runtime data directory away from `~/.kimi`."
    notes: "Affects config, metadata, credentials, MCP OAuth tokens, sessions, plans, history, and logs."
io_contract:
  stdout: structured_only
  stderr: diagnostics_only
  stdin: stream_protocol
  framing: jsonrpc_lines
  noise_handling: "For `--wire`, parse stdout as one JSON-RPC message per line and treat stderr/log files as diagnostics. For print `stream-json`, parse stdout as JSONL but expect a lossy projection."
  notes: "Wire uses stdin for client requests/responses and stdout for server responses/events/requests. Print mode uses stdin as prompt text or user-message JSONL."
stream_contract:
  discriminator: "JSON-RPC `method`; Wire payload discriminator is `params.type` for `event` and `request` messages; nested subagent payloads use `params.payload.event.type`."
  event_ordering: "`TurnBegin` starts a turn, `StepBegin` starts each step, `StepRetry` indicates retryable failure, and `TurnEnd` ends a normal turn. `TurnEnd` can be omitted on interruption."
  correlation_fields:
    - "id"
    - "params.payload.id"
    - "params.payload.tool_call_id"
    - "params.payload.request_id"
    - "params.payload.parent_tool_call_id"
    - "params.payload.message_id"
  terminal_event: "JSON-RPC response to `prompt` with `result.status`; do not rely only on `TurnEnd`."
  partial_message_events: true
  unknown_event_policy: "Preserve raw payload, classify known fields, and skip unknown `params.type`; answer unknown JSON-RPC requests only when Claudine is the responsible peer."
  notes: "Wire emits content/tool deltas and complete records. Print `stream-json` merges assistant content into whole messages and drops many control events."
session_metadata:
  session_id: "Not emitted by `initialize`; capture from launch/resume args and the session directory."
  cwd: "`--work-dir` controls it; Wire does not emit a stable cwd metadata event."
  model: "Requested via `--model` or config; not reliably emitted in Wire."
  provider: "Resolved through config/model mapping; not reliably emitted in Wire."
  auth: "Auth failures have error codes/messages, but auth kind/source is not exposed as stable Wire metadata."
  version: "`initialize.result.server.version`; `kimi info` is also documented for version/protocol information."
  mcp_servers: "`StatusUpdate.payload.mcp_status.servers[]` can include name, status, and tools during startup."
  permission_mode: "Controlled by launch flags, config defaults, and resumed session state; not emitted as a single stable field."
  notes: "Claudine should record launch metadata beside parsed stream records."
stream_events:
  - event: "initialize"
    category: session
    fields: ["protocol_version", "server.name", "server.version", "slash_commands", "capabilities", "hooks.supported_events", "external_tools"]
    notes: "Client request/server response, not an agent event notification."
  - event: "prompt"
    category: session
    fields: ["params.user_input", "result.status", "result.steps", "error.code", "error.message"]
    notes: "Starts a turn; its JSON-RPC response is the terminal record for that turn."
  - event: "replay"
    category: session
    fields: ["result.status", "result.events", "result.requests"]
    notes: "Replays saved Wire logs; clients should not answer replayed requests."
  - event: "steer"
    category: session
    fields: ["params.user_input", "result.status"]
    notes: "Queues input into an active turn."
  - event: "set_plan_mode"
    category: plan
    fields: ["params.enabled", "result.plan_mode", "error.message"]
    notes: "Requires plan-mode capability support."
  - event: "cancel"
    category: session
    fields: ["result.status"]
    notes: "Cancels active prompt or replay."
  - event: "TurnBegin"
    category: session
    fields: ["user_input"]
    notes: "Beginning of an agent turn."
  - event: "SteerInput"
    category: session
    fields: ["user_input"]
    notes: "Injected steer message was consumed."
  - event: "TurnEnd"
    category: session
    fields: []
    notes: "Normal turn end; may be absent on interruption."
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
    notes: "Retryable step failure; wait is in seconds."
  - event: "StatusUpdate"
    category: usage
    fields: ["context_usage", "context_tokens", "max_context_tokens", "token_usage", "message_id", "plan_mode", "mcp_status"]
    notes: "Primary live token/context/MCP/status event."
  - event: "ContentPart/TextPart"
    category: assistant
    fields: ["payload.type", "payload.text"]
    notes: "Assistant content part; payload subtype distinguishes text/media/reasoning."
  - event: "ContentPart/ThinkPart"
    category: reasoning
    fields: ["payload.type", "payload.think", "payload.encrypted"]
    notes: "Reasoning content when available and not suppressed."
  - event: "ToolCall"
    category: tool_call
    fields: ["id", "type", "function.name", "function.arguments", "extras"]
    notes: "Native tool call start/input."
  - event: "ToolCallPart"
    category: tool_call
    fields: ["arguments_part"]
    notes: "Streaming tool argument fragment."
  - event: "ToolResult"
    category: tool_result
    fields: ["tool_call_id", "return_value.is_error", "return_value.output", "return_value.message", "return_value.display", "return_value.extras"]
    notes: "Native tool result; join to call by `tool_call_id`."
  - event: "ApprovalRequest"
    category: permission
    fields: ["id", "tool_call_id", "sender", "action", "description", "source_kind", "source_id", "agent_id", "subagent_type", "display"]
    notes: "JSON-RPC `request`; client must respond."
  - event: "ApprovalResponse"
    category: permission
    fields: ["request_id", "response", "feedback"]
    notes: "Response values are `approve`, `approve_for_session`, and `reject`."
  - event: "QuestionRequest"
    category: permission
    fields: ["id", "tool_call_id", "questions[].question", "questions[].options[]", "questions[].multi_select"]
    notes: "Only sent when client declares question support."
  - event: "ToolCallRequest"
    category: tool_call
    fields: ["id", "name", "arguments"]
    notes: "External tool call registered during `initialize`; client must execute and respond."
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
    notes: "Plan markdown and plan file path."
  - event: "SubagentEvent"
    category: subagent
    fields: ["parent_tool_call_id", "agent_id", "subagent_type", "event.type", "event.payload"]
    notes: "Nested Wire event from a subagent; parsers need recursion."
  - event: "Notification"
    category: other
    fields: ["id", "category", "type", "source_kind", "source_id", "title", "body", "severity", "created_at", "payload"]
    notes: "`created_at` is a float timestamp; unit/timezone are not formally specified in docs."
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
    notes: "In Wire it requires capability negotiation unless AFK suppresses it; in print AFK behavior handles it automatically."
  - name: "SetTodoList"
    call_visible: true
    result_visible: true
    metadata: ["ToolCall.function.arguments", "ToolResult.return_value.display[type=todo]"]
    notes: "Todo state can appear in tool display blocks."
  - name: "Shell/CMD"
    call_visible: true
    result_visible: true
    metadata: ["ToolCall.function.arguments.command", "ToolResult.return_value", "ToolResult.return_value.display[type=shell]", "Notification"]
    notes: "Requires approval unless approval mode auto-approves. Windows uses a CMD-shaped backend according to changelog notes."
  - name: "ReadFile / ReadMediaFile / Glob / Grep"
    call_visible: true
    result_visible: true
    metadata: ["ToolCall.function.arguments", "ToolResult.return_value"]
    notes: "Read denials are inferred from tool errors/results, not a dedicated denial event."
  - name: "WriteFile / StrReplaceFile / PatchFile"
    call_visible: true
    result_visible: true
    metadata: ["ApprovalRequest.display[type=diff]", "ToolCall.function.arguments.path", "ToolResult.return_value.display"]
    notes: "Write/edit operations require approval unless auto-approved."
  - name: "SearchWeb / FetchURL"
    call_visible: true
    result_visible: true
    metadata: ["ToolCall.function.arguments", "ToolResult.return_value"]
    notes: "Requires provider/service configuration for search/fetch behavior."
  - name: "EnterPlanMode / ExitPlanMode"
    call_visible: true
    result_visible: true
    metadata: ["set_plan_mode", "StatusUpdate.plan_mode", "PlanDisplay.file_path"]
    notes: "Plan tools are controlled by capability negotiation and plan mode."
  - name: "External tools"
    call_visible: true
    result_visible: true
    metadata: ["initialize.external_tools", "ToolCallRequest.id", "ToolCallRequest.name", "ToolCallRequest.arguments"]
    notes: "Wire client registers tools during initialize and must answer tool-call requests."
completion:
  success_event: "JSON-RPC success response to `prompt` with `result.status = finished`; print success exits 0."
  failure_event: "JSON-RPC error response, `result.status = cancelled|max_steps_reached`, `StepInterrupted`, or print exit 1/75."
  exit_code_reliable: true
  result_fields: ["result.status", "result.steps", "error.code", "error.message"]
  cost_fields: []
  usage_fields: ["StatusUpdate.token_usage.input_other", "StatusUpdate.token_usage.output", "StatusUpdate.token_usage.input_cache_read", "StatusUpdate.token_usage.input_cache_creation", "StatusUpdate.context_tokens", "StatusUpdate.max_context_tokens", "StatusUpdate.context_usage"]
  notes: "For Wire, the prompt response is more precise than process exit because the server can remain alive across turns. Print mode documents exit 0/1/75."
blocking_behavior:
  permissions: configurable
  questions: configurable
  tool_approvals: configurable
  notes: "Print mode implicitly runs AFK. Wire clients must either negotiate and answer ApprovalRequest/QuestionRequest/HookRequest/ToolCallRequest or use AFK/YOLO depending on desired policy."
subagents:
  supported: true
  start_visible: true
  stop_visible: true
  nested_events_visible: true
  prompt_injection_supported: true
  metadata_fields: ["SubagentEvent.parent_tool_call_id", "SubagentEvent.agent_id", "SubagentEvent.subagent_type", "ToolCall.function.arguments.prompt", "ToolCall.function.arguments.model"]
  notes: "Subagents run through the `Agent` tool and have their own session subdirectories; prompt steering is through the Agent tool prompt or agent definitions."
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
    notes: "Print docs classify quota exhaustion as non-retryable exit 1, but Wire exposes provider/config failures as generic errors."
  - name: no_funds
    detectable: false
    event_types: ["prompt error", "print exit 1"]
    fields: ["error.message"]
    hook_parity: "none verified"
    notes: "No dedicated insufficient-balance event found."
  - name: auth
    detectable: true
    event_types: ["prompt error", "print exit 1"]
    fields: ["error.code", "error.message", "initialize.result.server.version"]
    hook_parity: "none verified"
    notes: "Source has `AUTH_EXPIRED = -32004`; print docs classify authentication failures as non-retryable exit 1."
  - name: permission_read_denied
    detectable: true
    event_types: ["ToolResult"]
    fields: ["tool_call_id", "return_value.is_error", "return_value.message", "ToolCall.function.arguments.path"]
    hook_parity: "possible through hook input_data if subscribed"
    notes: "Infer from read tool result/error; no dedicated read-denial event."
  - name: permission_write_denied
    detectable: true
    event_types: ["ApprovalResponse", "HookResolved", "ToolResult"]
    fields: ["response", "feedback", "action", "reason", "display.path", "tool_call_id"]
    hook_parity: "Wire HookRequest/HookResolved can carry hook block details"
    notes: "Approval rejection and hook block are explicit; path may be in display blocks or tool args."
  - name: tokens_consumed
    detectable: true
    event_types: ["StatusUpdate"]
    fields: ["token_usage.input_other", "token_usage.output", "token_usage.input_cache_read", "token_usage.input_cache_creation", "context_tokens", "max_context_tokens", "context_usage"]
    hook_parity: "none verified"
    notes: "Units are tokens; context usage is a ratio/percentage-like float per docs/source wording."
  - name: model_used
    detectable: false
    event_types: []
    fields: []
    hook_parity: "none verified"
    notes: "Requested model is launch/config metadata; resolved model/provider is not reliably emitted in Wire."
  - name: model_fallback
    detectable: false
    event_types: []
    fields: []
    hook_parity: "none verified"
    notes: "No fallback event found."
  - name: human_in_loop
    detectable: true
    event_types: ["ApprovalRequest", "QuestionRequest", "HookRequest"]
    fields: ["method", "params.type", "params.payload.id", "params.payload.questions", "params.payload.action"]
    hook_parity: "HookRequest is itself a programmable policy callback surface"
    notes: "If questions are not negotiated or AFK is active, AskUserQuestion may be suppressed or auto-dismissed."
  - name: session_resumable
    detectable: true
    event_types: ["launch metadata", "wire.jsonl replay"]
    fields: ["session_id from argv/session directory", "replay.result.events", "replay.result.requests"]
    hook_parity: "SessionStart hook has source context"
    notes: "Capture session ID from launch/resume and filesystem path, not from `initialize`."
  - name: subagent_prompt_injection
    detectable: true
    event_types: ["ToolCall"]
    fields: ["function.name", "function.arguments.prompt", "function.arguments.subagent_type", "function.arguments.model"]
    hook_parity: "Agent tool hooks can inspect/block"
    notes: "Caller can steer subagents by changing the Agent tool prompt or custom agent definitions."
headless_constraints:
  - constraint: "Wire is bidirectional; stdout is not merely an event feed."
    mitigation: "Implement a JSON-RPC peer that sends `initialize` and `prompt` and answers requests."
    notes: "ApprovalRequest, QuestionRequest, ToolCallRequest, and HookRequest can block a turn until answered."
  - constraint: "Print `stream-json` is a lossy projection."
    mitigation: "Use Wire for wrapper-grade supervision; use print JSONL only for simple one-shot automation."
    notes: "Print JSONL buffers assistant/tool-call deltas and ignores many Wire messages."
  - constraint: "Wire does not expose complete launch metadata."
    mitigation: "Record argv, cwd, resolved config snapshot, and session path beside parsed records."
    notes: "Session ID, cwd, requested model, provider, auth kind, and permission mode are not all emitted in `initialize`."
  - constraint: "Questions and external tool calls require client capability/response handling."
    mitigation: "Use `--afk` for deterministic unattended runs or explicitly implement request handlers."
    notes: "YOLO is weaker than AFK because user questions can still be reachable."
  - constraint: "Config and resumed session state can silently change permissions, plan mode, roots, and model."
    mitigation: "Prefer explicit flags and isolated `KIMI_SHARE_DIR` for reproducible wrapper runs."
    notes: "State is persisted under the share directory."
quirks:
  - "The best structured mode is a bidirectional JSON-RPC line protocol, not an `--output-format` value."
  - "Wire transport discrimination is JSON-RPC `method`; event/request subtype discrimination is `params.type`, with recursive `SubagentEvent.event.type`."
  - "Print `stream-json` emits OpenAI-like message records plus plan/notification records, not the Wire event union."
  - "Print mode implicitly uses AFK behavior, which is convenient for CI but can execute edits and shell commands without approval."
  - "Wire `initialize` exposes server and protocol version but not session ID, cwd, model, provider, auth kind, or a single permission-mode field."
  - "Some payload classes come from dependency packages (`kosong`), so the complete schema spans more than Kimi's `wire/types.py`."
gaps:
  - "No official JSON Schema, OpenAPI, AsyncAPI, or versioned schema artifact was found for Wire."
  - "No captured local fixture was produced because exercising Kimi would require auth and could run tools."
  - "Could not verify a stable field for resolved provider/model or auth kind in the Wire stream."
  - "Could not verify dedicated quota-near-cap, quota-exhausted, no-funds, or model-fallback events."
  - "Timestamp unit/timezone for `Notification.created_at` is not formally documented."
  - "Exact handling of unanswered Wire requests in every shutdown/error path should be fixture-tested against an authenticated installation."
claudine_strategy:
  preferred_invocation: "kimi --wire --work-dir <repo> --afk"
  required_flags: ["--wire", "--work-dir <repo>", "--afk unless Claudine implements approval/question/external-tool/hook request responses"]
  conflicting_flags: ["--print", "--quiet", "--final-message-only", "--output-format", "--input-format", "--prompt as Wire input"]
  parser_notes: "Parse stdout as JSON-RPC lines. Drive `initialize` first, then `prompt`. Classify by `method`; for `event` and `request`, classify by `params.type`. Recurse into `SubagentEvent.event`. Join tool calls/results by `id` and `tool_call_id`; join approvals/questions/hooks by request `id` and response `request_id`."
  wrapper_notes: "Persist launch metadata because Wire does not emit enough session/model/auth context. Treat stderr/log files as diagnostics. Use print `stream-json` only when Claudine accepts missing live control events."
data_format: jsonrpc_lines
changes:
  - "2026-07-03: Refreshed against current official docs and source; fixed per-OS config records for schema validation and clarified Wire-vs-print parser strategy."
requires_claudine_update: true
reason: "Claudine's Kimi wrapper should prefer Wire JSON-RPC lines with a real bidirectional peer; print `stream-json` is insufficient for live lifecycle supervision."
---

# Kimi Code CLI Non-Interactive Sessions

## Summary

Kimi Code CLI can run non-interactively in two useful ways. `kimi --print` is the simple scripting mode: it accepts a prompt from `-p`/`-c` or stdin, exits automatically, and can emit `--output-format stream-json` JSONL. That mode is useful for one-shot automation, but it is not the best Claudine integration point because it buffers assistant content into whole messages, projects tool results into message records, and drops many lifecycle/control events.

Claudine should prefer `kimi --wire --work-dir <repo> --afk`. Wire mode is a JSON-RPC 2.0 line protocol over stdin/stdout, with official TypeScript-style definitions and matching Pydantic source models. It exposes turn, step, status, tool, approval, hook, plan, notification, MCP, and subagent events while a run is active. The main wrapper risk is that Wire is bidirectional: Claudine must act as a JSON-RPC peer, not just tail stdout. It must send `initialize`/`prompt` and either answer approval, question, external-tool, and hook requests or launch with AFK semantics.

## Non-Interactive Entry Points

Kimi's documented non-interactive entry point is print mode. The print-mode page says `--print` is suitable for scripting and automation, can take `-p`/`-c` or stdin, and exits automatically after executing instructions. It also says print mode implicitly enables AFK: tool calls are auto-approved and interactive questions/plan switches are handled automatically.

Wire mode is lower level. The Wire docs describe it as a structured bidirectional protocol for external programs, custom UIs, embedding, and automated testing. The CLI invocation is `kimi --wire`, but a prompt is not an argv field in this mode; the client sends a JSON-RPC `prompt` request after optionally sending `initialize`. The command reference lists `--print`, `--quiet`, `--acp`, and `--wire` as mutually exclusive UI modes, with shell mode as the default.

The scriptable forms Claudine should care about are:

| Mode | Command shape | Prompt input | Session behavior | Automation fit |
| --- | --- | --- | --- | --- |
| Wire fresh | `kimi --wire --work-dir <repo>` | JSON-RPC `prompt` request on stdin | New session | Best for wrappers that need live events |
| Wire resume | `kimi --wire --work-dir <repo> --session <id>` | JSON-RPC `prompt` request on stdin | Resumes ID, or creates if missing | Best for resumable wrapper sessions |
| Wire continue | `kimi --wire --work-dir <repo> --continue` | JSON-RPC `prompt` request on stdin | Continues previous session for cwd | Useful when caller controls Kimi state |
| Print argv | `kimi --print -p "..." --output-format stream-json` | Argv prompt | One-shot session | Simple CI/scripting fallback |
| Print stdin JSONL | `kimi --print --input-format stream-json --output-format stream-json` | User-role JSONL until EOF | Processes messages until stdin closes | Batch request/reply fallback |

Kimi also has ACP, web, and visualizer modes, but they are not the preferred non-interactive stream for Claudine's wrapper. ACP is listed as deprecated in favor of `kimi acp`; web and visualizer are server/UI surfaces rather than the direct process stream Claudine needs.

## Output Formats

Wire is the richest stream. The official Wire page says each message is a single JSON line conforming to JSON-RPC 2.0 and that the protocol version is `1.10`. It includes ordinary JSON-RPC requests/responses plus server notifications/requests with method names such as `event` and `request`. For Claudine, this means stdout is parseable line-by-line only after Claudine understands that some lines require a response on stdin.

Print `stream-json` is easier but weaker. The print-mode docs call it JSONL and show assistant/tool messages emitted sequentially. Source inspection of `src/kimi_cli/ui/print/visualize.py` shows why it is lossy: `JsonPrinter` buffers `ContentPart` and `ToolCall` records, flushes an assistant `Message` at `StepBegin`/`StepInterrupted` boundaries or before a `ToolResult`, emits `ToolResult` as a tool message, emits `PlanDisplay`, buffers notifications until safe boundaries, and ignores other Wire messages. That makes print JSONL usable for transcripts but poor for live lifecycle detection.

| Output | Selector | Framing | Streams while active | What changes | Claudine preference |
| --- | --- | --- | --- | --- | --- |
| Wire | `--wire` | JSON-RPC lines | Yes | Process becomes a bidirectional protocol server | Prefer |
| Print JSONL | `--print --output-format stream-json` | JSONL | Partly | Emits projected messages, not full Wire events | Fallback |
| Print text | `--print --output-format text` | Text | Partly | Human-oriented output | Avoid for parsing |
| Quiet/final-only | `--quiet` or `--final-message-only` | Text | No | Drops intermediate events | Avoid for supervision |

The tradeoff is straightforward: print JSONL is simpler for request/reply scripts, but Wire is the only documented mode that exposes enough live operational state for Claudine to show progress, classify blocking behavior, correlate tool calls, and drive cancellation or request handling before process exit.

## Schema Sources

Kimi does not publish a JSON Schema, OpenAPI document, or AsyncAPI document for Wire. The best formal evidence is the official Wire documentation, which provides TypeScript-style interfaces for JSON-RPC envelopes, initialize, prompt, replay, steer, plan mode, request/response errors, events, requests, and display blocks.

The source is also strong evidence. `src/kimi_cli/wire/jsonrpc.py` defines Pydantic models for JSON-RPC message envelopes, inbound methods, outbound methods, statuses, and error codes. It defines inbound methods `initialize`, `prompt`, `steer`, `replay`, `set_plan_mode`, and `cancel`, and outbound methods `event` and `request`. It also defines statuses `finished`, `cancelled`, `max_steps_reached`, and `steered`, plus Kimi-specific error codes such as `LLM_NOT_SET`, `LLM_NOT_SUPPORTED`, `CHAT_PROVIDER_ERROR`, and `AUTH_EXPIRED`.

`src/kimi_cli/wire/types.py` defines the Wire event/request union. Important event classes include `TurnBegin`, `SteerInput`, `TurnEnd`, `StepBegin`, `StepInterrupted`, `StepRetry`, `CompactionBegin`, `CompactionEnd`, `HookTriggered`, `HookResolved`, `MCPLoadingBegin`, `MCPLoadingEnd`, `StatusUpdate`, `Notification`, `PlanDisplay`, `BtwBegin`, `BtwEnd`, `SubagentEvent`, `ApprovalResponse`, content parts, tool calls, tool-call parts, and tool results. Request classes include `ApprovalRequest`, `QuestionRequest`, `ToolCallRequest`, and `HookRequest`.

Some nested payloads come from Kimi dependencies rather than Kimi's own module, especially `kosong.message`, `kosong.tooling`, and `kosong.chat_provider.TokenUsage`. That is a schema risk: the public docs and Kimi source identify the fields Claudine needs, but a generated parser should keep unknown payload fields and tolerate additive variants.

## IO Contract

In Wire mode, stdout is the structured transport. Each stdout line is one JSON-RPC message. Stdin is also structured: Claudine must send JSON-RPC requests and responses. Stderr should be treated as diagnostics, not the authoritative event stream. The Kimi data-location docs say runtime logs are stored under `~/.kimi/logs/kimi.log` and `--debug` enables TRACE-level logging there.

In print mode, stdout is either text or JSONL depending on `--output-format`. With `--input-format stream-json`, stdin is user-message JSONL, not a bidirectional protocol. Print mode's documented exit codes are useful for CI: `0` for success, `1` for non-retryable failures such as configuration, authentication, quota exhaustion, and other permanent errors, and `75` for retryable failures such as rate limits, 5xx server errors, and timeouts.

Claudine should not mix Wire and print assumptions. `--output-format stream-json` is print-only; it does not make Wire more or less structured.

## Stream Contract

Wire has two discriminator layers. The transport discriminator is JSON-RPC `method`. Server event notifications use `method: "event"` with the Wire subtype in `params.type`; server requests use `method: "request"` with the Wire request subtype in `params.type`. Subagent records are recursive: `SubagentEvent` carries another Wire event under its nested `event` field.

The normal turn ordering is:

```mermaid
sequenceDiagram
    participant C as Claudine
    participant K as kimi --wire
    C->>K: initialize
    K-->>C: initialize result
    C->>K: prompt
    K-->>C: event TurnBegin
    K-->>C: event StepBegin
    K-->>C: event StatusUpdate
    K-->>C: event ContentPart / ToolCall / ToolResult
    K-->>C: request ApprovalRequest / QuestionRequest / HookRequest
    C->>K: request response
    K-->>C: event TurnEnd
    K-->>C: prompt result {status}
```

`TurnEnd` is useful but not terminal enough. Source and docs say it may be omitted when a turn is interrupted. Claudine should treat the JSON-RPC response to its `prompt` request as the turn terminal record and use `result.status` or `error` for classification. Tool calls join by `ToolCall.id` and `ToolResult.tool_call_id`. Approval, question, external-tool, and hook requests join by request `id` and response `request_id`.

Unknown event handling should be conservative: preserve raw JSON, classify known envelope fields, skip unknown `params.type`, and continue. The Pydantic source already models additive behavior through optional fields and dependency-owned nested payloads, so a strict closed union would be brittle.

## Session Metadata

Wire exposes some initialization metadata but not everything Claudine wants. The `initialize` response includes `protocol_version`, `server.name`, `server.version`, slash commands, capabilities, hook support, and external-tool registration results. It does not provide a stable session ID, cwd, requested/resolved model, provider, auth kind, permission mode, sandbox mode, or roots.

Those missing fields must come from launch context and Kimi storage. The data-location docs say Kimi stores sessions under `~/.kimi/sessions/<work-dir-hash>/<session-id>/`, with `context.jsonl`, `wire.jsonl`, and `state.json`. `state.json` stores approval state, plan mode, plan identifiers, subagent metadata, and additional directories. `wire.jsonl` stores Wire messages for replay and visualizer/export workflows. `KIMI_SHARE_DIR` can move the whole runtime data directory, so Claudine should not hard-code `~/.kimi` when that variable is set.

MCP metadata can appear in stream status. Source defines `StatusUpdate.mcp_status` with loading state, connected/total/tool counts, and per-server snapshots with `name`, `status`, and `tools`.

## Event Families

Kimi's Wire events are broad enough for wrapper supervision:

| Family | Events or methods | Wrapper value |
| --- | --- | --- |
| Session/turn | `initialize`, `prompt`, `replay`, `steer`, `cancel`, `TurnBegin`, `SteerInput`, `TurnEnd` | Lifecycle and cancellation |
| Step/retry | `StepBegin`, `StepInterrupted`, `StepRetry` | Progress, retry classification, transient errors |
| Status/usage | `StatusUpdate` | Context ratio, context tokens, token usage, plan mode, MCP status |
| Assistant/reasoning | `ContentPart` with nested text/think/media types | Live assistant output and reasoning where available |
| Tools | `ToolCall`, `ToolCallPart`, `ToolResult`, `ToolCallRequest` | Native and external tool execution |
| Permissions/questions | `ApprovalRequest`, `ApprovalResponse`, `QuestionRequest`, `HookRequest`, `HookResolved` | Human-in-loop and policy decisions |
| Plan | `set_plan_mode`, `PlanDisplay`, `StatusUpdate.plan_mode` | Plan-mode state and plan artifacts |
| Subagents | `SubagentEvent` | Nested agent progress and results |
| MCP | `MCPLoadingBegin`, `MCPLoadingEnd`, `StatusUpdate.mcp_status` | MCP startup and server status |
| Other | `Notification`, `CompactionBegin`, `CompactionEnd`, `BtwBegin`, `BtwEnd` | UI notifications, compaction, side questions |

The source token usage fields are `input_other`, `output`, `input_cache_read`, and `input_cache_creation`. `StatusUpdate.context_usage`, `context_tokens`, and `max_context_tokens` describe current context pressure.

## Tools

Wire exposes both call and result records for native tools. `ToolCall` includes `id`, `function.name`, and JSON-string `function.arguments`. `ToolCallPart` streams argument fragments. `ToolResult` joins by `tool_call_id` and carries `return_value.is_error`, model-facing output, message, display blocks, and extras.

Display blocks matter for file and shell observability. The Wire docs define `DiffDisplayBlock` with `path`, `old_text`, `new_text`, and optional `is_summary`; `ShellDisplayBlock` with `language` and `command`; and todo/brief/unknown blocks. For write tools, an approval request may carry a diff display before execution. For shell tools, command text may appear in tool arguments and shell display blocks. Exact stdout/stderr split for command execution is not documented as separate top-level fields; Claudine should treat shell result output as tool-return content unless fixtures prove a more specific structure.

External tools are different. During `initialize`, Claudine can register `external_tools` with JSON Schema parameters. Kimi can then send `ToolCallRequest` records to the Wire client. Claudine must execute or reject those and send a JSON-RPC response. If Claudine does not implement external tools, it should not register them.

## Completion and Exit Status

For Wire, the terminal event for a turn is the JSON-RPC response to the `prompt` request. `result.status = "finished"` is normal success. `cancelled` and `max_steps_reached` are terminal non-success statuses. JSON-RPC errors carry `error.code`, `error.message`, and optional `error.data`; source includes standard JSON-RPC codes plus Kimi-specific codes for invalid state, missing/unsupported LLM, chat provider errors, and expired auth.

For print mode, documented process exit codes are meaningful: `0` success, `1` non-retryable failure, and `75` retryable failure. Claudine can trust those in print mode, but in Wire mode process exit is not the per-turn success signal because the server can stay alive across turns. The per-request response is more precise.

Usage is available during the run through `StatusUpdate`, not only at completion. No cost fields were found. Quota and billing failures are not exposed as dedicated typed events; print documentation maps quota exhaustion to exit code `1`, and Wire should be classified from error code/message until a richer event is verified.

## Blocking Behavior

Print mode is the safest noninteractive mode from a blocking perspective because it implicitly enables AFK. The cost is control: it auto-approves tool calls and auto-handles interactive questions and plan switches. That can be acceptable for isolated CI, but it is not a permission-supervision strategy.

Wire mode is configurable. `--yolo`, `--yes`, and `--auto-approve` auto-approve tool calls, but the command reference says the user can still be reachable for `AskUserQuestion`. `--afk` is the stronger unattended flag: it auto-approves tool calls and auto-dismisses questions. Without AFK or implemented request handlers, Wire can block on `ApprovalRequest`, `QuestionRequest`, `ToolCallRequest`, or `HookRequest`.

For Claudine's first robust integration, `--afk` is the deterministic choice. A richer integration can omit AFK only after it has request handlers and a policy for approval, questions, client tools, and hooks.

## Subagents

Subagents are visible. Source defines `SubagentEvent` with `parent_tool_call_id`, `agent_id`, `subagent_type`, and a nested Wire `event`. The data-location docs say each subagent instance has its own storage directory under the session directory with `context.jsonl`, `wire.jsonl`, `meta.json`, `prompt.txt`, and output.

The parent stream can therefore show nested subagent events, but parsers must recurse. Prompt injection is not a separate protocol control; it is available through the `Agent` tool prompt/model arguments or custom agent files. If Claudine needs subagents to avoid interactive behavior, it should inject that instruction into the parent prompt and, where possible, into the Agent tool prompt or agent definition.

## Use Case Detection

| Use case | Detectable | Evidence | Notes |
| --- | --- | --- | --- |
| `plan_cap_approaching` | No | None found | No typed account-plan cap event verified. |
| `plan_capped` | Weak | print exit `1`, Wire error text | Quota exhaustion is documented only as a non-retryable print failure. |
| `no_funds` | Weak | Wire/print error text | No dedicated billing/no-funds event verified. |
| `auth` | Yes | JSON-RPC error, `AUTH_EXPIRED`, print exit `1` | Auth kind/source is not exposed. |
| `permission_read_denied` | Yes, inferred | `ToolResult.return_value.is_error` and read tool args | No dedicated read-denial event found. |
| `permission_write_denied` | Yes | `ApprovalResponse`, `HookResolved`, `ToolResult` | Path may be in diff display or tool args. |
| `tokens_consumed` | Yes | `StatusUpdate.token_usage` | Units are tokens; fields are per status update/current step. |
| `model_used` | Launch-only | argv/config | Not reliably emitted in Wire. |
| `model_fallback` | No | None found | No fallback event verified. |
| `human_in_loop` | Yes | `ApprovalRequest`, `QuestionRequest`, `HookRequest` | AFK may suppress or auto-handle some cases. |
| `session_resumable` | Yes | argv/session directory/`wire.jsonl` | Do not wait for `initialize` to emit session ID. |
| `subagent_prompt_injection` | Yes | `ToolCall.function.arguments` for Agent tool | Injection path is prompt/agent configuration, not a protocol field. |

## Headless Constraints

The major constraint is bidirectionality. A wrapper that only reads stdout can parse events for a while, but it will eventually fail or block when Kimi sends a request that requires a response. This is a design feature of Wire, not incidental noise.

The second constraint is metadata. Kimi's stream is good at operational events but weak at launch context. Claudine must preserve its own launch record: cwd, intended worktree, session ID, model flag, config file, `KIMI_SHARE_DIR`, approval mode, and MCP config sources.

The third constraint is persisted state. Resuming a session restores plan mode, approval state, additional directories, and subagent metadata from `state.json`. For reproducible automation, Claudine should prefer explicit flags and an isolated share directory.

## Timeline

The changelog records `--print` and `--output-format stream-json` support in the 0.21 release line, and later command docs show `--wire` as an experimental UI mode. Current Wire docs identify protocol version `1.10`. The source files inspected on `main` match the documented split: Wire as JSON-RPC lines and print JSON as a projection over Wire messages.

## Quirks and Gaps

Kimi's best mode is not named like a conventional output format. `--wire` changes the process contract into JSON-RPC over stdio. Treating it like NDJSON events would miss request/response obligations and cancellation semantics.

Print `stream-json` can look complete in small examples because it emits assistant and tool messages. Source shows it ignores many Wire messages and buffers deltas, so it is not enough for Claudine's live status, permission, subagent, and lifecycle needs.

Verified gaps remain:

- No official JSON Schema or OpenAPI/AsyncAPI artifact was found.
- No authenticated local fixture was captured.
- Resolved provider/model and auth kind were not found as stable Wire fields.
- Dedicated quota-near-cap, quota-exhausted, no-funds, and model-fallback events were not found.
- `Notification.created_at` is a float, but its unit/timezone are not formally specified.

## Claudine Integration Notes

Recommended command:

```sh
kimi --wire --work-dir <repo> --afk
```

Claudine should parse stdout as JSON-RPC lines and treat stderr plus `~/.kimi/logs/kimi.log` as diagnostics. It should send `initialize` with `protocol_version: "1.10"` and client info, then send `prompt`. If Claudine wants to handle human-in-loop events itself, it can declare `supports_question`, hook subscriptions, and external tools during `initialize`; otherwise it should use `--afk` and avoid registering external tools.

Parser notes:

- Discriminate first by JSON-RPC `method`.
- For `method: "event"` and `method: "request"`, discriminate by `params.type`.
- Preserve raw payloads for unknown event/request types.
- Recurse into `SubagentEvent.event`.
- Join tool calls/results by `ToolCall.id` and `ToolResult.tool_call_id`.
- Treat the `prompt` response as terminal for the turn.
- Keep launch metadata beside the stream because Kimi does not emit enough session/model/auth context.

Use print `stream-json` only when Claudine needs a simple transcript and accepts the loss of live control events. Avoid `--quiet`, `--final-message-only`, and text output for wrapper supervision.

## Changelog

- 2026-07-03: Refreshed against current Kimi docs and source. Preserved the original creation date, fixed schema-invalid all-OS config records, and clarified that Wire is the preferred bidirectional protocol while print JSONL is only a fallback.

## Sources

- [Wire mode documentation](https://moonshotai.github.io/kimi-cli/en/customization/wire-mode.html)
- [Print mode documentation](https://moonshotai.github.io/kimi-cli/en/customization/print-mode.html)
- [`kimi` command reference](https://moonshotai.github.io/kimi-cli/en/reference/kimi-command.html)
- [Config files documentation](https://moonshotai.github.io/kimi-cli/en/configuration/config-files.html)
- [Data locations documentation](https://moonshotai.github.io/kimi-cli/en/configuration/data-locations.html)
- [Hooks documentation](https://moonshotai.github.io/kimi-cli/en/customization/hooks.html)
- [`src/kimi_cli/wire/types.py`](https://github.com/MoonshotAI/kimi-cli/blob/main/src/kimi_cli/wire/types.py)
- [`src/kimi_cli/wire/jsonrpc.py`](https://github.com/MoonshotAI/kimi-cli/blob/main/src/kimi_cli/wire/jsonrpc.py)
- [`src/kimi_cli/ui/print/visualize.py`](https://github.com/MoonshotAI/kimi-cli/blob/main/src/kimi_cli/ui/print/visualize.py)
