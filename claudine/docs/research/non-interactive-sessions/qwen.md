---
$schema: ./_schema.yaml
created: 2026-07-02
last_updated: 2026-07-03
agent: codex
model: default
docs: https://qwenlm.github.io/qwen-code-docs/en/users/features/headless/
invocation:
  - command: 'qwen -p "prompt" --output-format stream-json --include-partial-messages'
    stdin_support: true
    prompt_arg: "--prompt/-p supplies the prompt; piped stdin can add prompt/context in text input mode"
    notes: "Starts a fresh one-shot headless session and emits JSONL on stdout."
  - command: 'cat input.txt | qwen -p "prompt" --output-format stream-json --include-partial-messages'
    stdin_support: true
    prompt_arg: "--prompt/-p plus text stdin"
    notes: "Fresh one-shot headless session with argv prompt and piped content."
  - command: 'qwen --continue -p "prompt" --output-format stream-json --include-partial-messages'
    stdin_support: true
    prompt_arg: "--prompt/-p"
    notes: "Continues the most recent project-scoped session; history is stored under QWEN_HOME projects state."
  - command: 'qwen --resume <session-id> -p "prompt" --output-format stream-json --include-partial-messages'
    stdin_support: true
    prompt_arg: "--prompt/-p"
    notes: "Resumes a specific session. Avoid bare --resume in automation because it can choose interactively."
  - command: 'qwen --input-format stream-json --output-format stream-json'
    stdin_support: true
    prompt_arg: "stdin is a JSON-line protocol, not plain prompt text"
    notes: "SDK/control-plane mode. It can emit and consume control_request/control_response records and is still described by upstream docs as under construction."
  - command: 'qwen -p "prompt" --output-format json'
    stdin_support: true
    prompt_arg: "--prompt/-p or piped stdin"
    notes: "One-shot headless session that buffers a JSON array until completion."
  - command: 'qwen -p "prompt" --output-format text'
    stdin_support: true
    prompt_arg: "--prompt/-p or piped stdin"
    notes: "Human-readable headless output; useful for humans, weak for Claudine supervision."
output_formats:
  - name: "text"
    cli_value: "text"
    stream: true
    format: text
    description: "Default human-readable output. With --json-schema, stdout becomes the validated JSON payload line on success."
    side_effects: "Not parser-safe for lifecycle supervision; diagnostics and warnings are on stderr."
  - name: "json"
    cli_value: "json"
    stream: false
    format: json
    description: "Single JSON array of system, assistant, user/tool-result, and result messages emitted after completion."
    side_effects: "Good for post-run auditing and stats; no live progress."
  - name: "stream-json"
    cli_value: "stream-json"
    stream: true
    format: jsonl
    description: "One complete JSON object per stdout line as messages occur. Add --include-partial-messages for stream_event deltas."
    side_effects: "Best Claudine mode. stdout is parseable JSONL; stderr still carries warnings, debug notices, MCP failures, retry heartbeats, and some failures without result events."
  - name: "stream-json input/control"
    cli_value: "stream-json"
    stream: true
    format: jsonl
    description: "Bidirectional JSON-line control protocol when --input-format stream-json is paired with --output-format stream-json."
    side_effects: "stdin is protocol input; hosts may need to answer control_request records such as can_use_tool."
schema_sources:
  - url: "https://github.com/QwenLM/qwen-code/blob/main/packages/cli/src/nonInteractive/types.ts"
    schema_type: typescript
    formal: false
    notes: "Authoritative CLI message, result, stream_event, permission, and control-plane union types; no published JSON Schema."
  - url: "https://github.com/QwenLM/qwen-code/blob/main/packages/cli/src/nonInteractive/io/BaseJsonOutputAdapter.ts"
    schema_type: typescript
    formal: false
    notes: "Builds assistant/user/tool-result/result messages, permission_denials, structured_result, and usage fields."
  - url: "https://github.com/QwenLM/qwen-code/blob/main/packages/cli/src/nonInteractive/io/StreamJsonOutputAdapter.ts"
    schema_type: typescript
    formal: false
    notes: "Defines JSONL emission and partial stream_event records for message_start, content_block_delta, tool_progress, active_goal, and related events."
  - url: "https://github.com/QwenLM/qwen-code/blob/main/packages/sdk-typescript/src/types/protocol.ts"
    schema_type: typescript
    formal: false
    notes: "SDK-facing protocol types mirror much of the CLI shape and are useful for --input-format stream-json, but should be treated as secondary to CLI-local types."
  - url: "https://qwenlm.github.io/qwen-code-docs/en/users/features/headless/"
    schema_type: examples
    formal: false
    notes: "Official examples document formats and high-level fields but are not a complete schema."
cli_params:
  - flag: "--prompt, -p"
    value: "string"
    description: "Runs Qwen Code in non-interactive/headless mode with the supplied prompt."
    example: 'qwen -p "review this diff"'
  - flag: "--output-format, -o"
    value: "text | json | stream-json"
    description: "Selects human text, buffered JSON, or streaming JSONL output for non-interactive mode."
    example: 'qwen -p "query" --output-format stream-json'
  - flag: "--input-format"
    value: "text | stream-json"
    description: "Selects stdin text input or the bidirectional JSON-line protocol; stream-json input requires stream-json output."
    example: "qwen --input-format stream-json --output-format stream-json"
  - flag: "--include-partial-messages"
    value: "boolean"
    description: "Adds stream_event records such as message_start and content_block_delta to stream-json output."
    example: 'qwen -p "query" --output-format stream-json --include-partial-messages'
  - flag: "--system-prompt"
    value: "string"
    description: "Overrides the main-session system prompt for this run."
    example: 'qwen -p "query" --system-prompt "You are terse."'
  - flag: "--append-system-prompt"
    value: "string"
    description: "Appends extra main-session instructions after built-in prompt and memory."
    example: 'qwen -p "query" --append-system-prompt "Avoid prompts."'
  - flag: "--json-schema"
    value: "inline JSON or @path"
    description: "Constrains the final answer through a synthetic structured_output tool; incompatible with --input-format stream-json, ACP, and prompt-interactive."
    example: 'qwen -p "audit" --json-schema @./schema.json --output-format stream-json'
  - flag: "--model, -m"
    value: "model id"
    description: "Selects the model for the session; emitted in system and assistant messages as the model string."
    example: "qwen -m qwen3-coder-plus -p query"
  - flag: "--approval-mode"
    value: "plan | default | auto-edit | auto | yolo"
    description: "Controls tool approval policy; default cannot prompt in SDK permission control without a host response."
    example: "qwen -p query --approval-mode yolo"
  - flag: "--yolo, -y"
    value: "boolean"
    description: "Auto-approves all tool calls; does not enable sandboxing."
    example: "qwen -p query --yolo"
  - flag: "--sandbox, -s"
    value: "boolean"
    description: "Enables sandbox mode for the session."
    example: "qwen -p query --sandbox"
  - flag: "--safe-mode"
    value: "boolean"
    description: "Disables customizations such as hooks, extensions, skills, MCP servers, custom subagents, permission rules, memory, and settings-sourced sandbox/approval overrides."
    example: "qwen -p query --safe-mode"
  - flag: "--all-files, -a"
    value: "boolean"
    description: "Adds all files under the current directory as prompt context."
    example: "qwen -p query --all-files"
  - flag: "--include-directories"
    value: "comma-separated paths"
    description: "Adds extra directories as context roots."
    example: "qwen -p query --include-directories src,docs"
  - flag: "--continue"
    value: "boolean"
    description: "Continues the most recent project-scoped saved session."
    example: 'qwen --continue -p "continue"'
  - flag: "--resume"
    value: "optional session id"
    description: "Resumes a specific session when an ID is supplied; omit the ID only for interactive use."
    example: 'qwen --resume 123e4567-e89b-12d3-a456-426614174000 -p "continue"'
  - flag: "--max-session-turns"
    value: "integer"
    description: "Caps session turns; overrun exits with code 53 and may not emit a result event."
    example: "qwen -p query --max-session-turns 30"
  - flag: "--max-wall-time"
    value: "seconds or duration"
    description: "Caps wall-clock run time; budget aborts use exit code 55."
    example: "qwen -p query --max-wall-time 10m"
  - flag: "--max-tool-calls"
    value: "integer"
    description: "Caps top-level tool dispatches; inner subagent tool calls are out of scope."
    example: "qwen -p query --max-tool-calls 50"
  - flag: "--debug, -d"
    value: "boolean"
    description: "Enables debug mode and prints debug log location to stderr."
    example: "qwen -p query --debug"
config_files:
  - os: macos
    scope: user
    path: "~/.qwen/settings.json"
    format: jsonc
    effect: "User defaults for model, providers, permissions, MCP servers, telemetry, hooks, skills, and other settings."
    notes: "QWEN_HOME redirects this directory. CLI flags override for the current run where supported."
  - os: linux
    scope: user
    path: "~/.qwen/settings.json"
    format: jsonc
    effect: "User defaults for model, providers, permissions, MCP servers, telemetry, hooks, skills, and other settings."
    notes: "QWEN_HOME redirects this directory. CLI flags override for the current run where supported."
  - os: windows
    scope: user
    path: "%USERPROFILE%\\.qwen\\settings.json"
    format: jsonc
    effect: "User defaults for model, providers, permissions, MCP servers, telemetry, hooks, skills, and other settings."
    notes: "QWEN_HOME redirects this directory. CLI flags override for the current run where supported."
  - os: macos
    scope: repo
    path: ".qwen/settings.json"
    format: jsonc
    effect: "Workspace settings for model/providers, permissions, MCP, hooks, skills, and other repo-local behavior."
    notes: "Merged only when the workspace is trusted; untrusted workspace settings are ignored."
  - os: linux
    scope: repo
    path: ".qwen/settings.json"
    format: jsonc
    effect: "Workspace settings for model/providers, permissions, MCP, hooks, skills, and other repo-local behavior."
    notes: "Merged only when the workspace is trusted; untrusted workspace settings are ignored."
  - os: windows
    scope: repo
    path: ".qwen\\settings.json"
    format: jsonc
    effect: "Workspace settings for model/providers, permissions, MCP, hooks, skills, and other repo-local behavior."
    notes: "Merged only when the workspace is trusted; untrusted workspace settings are ignored."
  - os: macos
    scope: system
    path: "/Library/Application Support/QwenCode/settings.json"
    format: jsonc
    effect: "System override settings. Source merge order makes system settings highest precedence."
    notes: "Can be redirected by QWEN_CODE_SYSTEM_SETTINGS_PATH."
  - os: linux
    scope: system
    path: "/etc/qwen-code/settings.json"
    format: jsonc
    effect: "System override settings. Source merge order makes system settings highest precedence."
    notes: "Can be redirected by QWEN_CODE_SYSTEM_SETTINGS_PATH."
  - os: windows
    scope: system
    path: "C:\\ProgramData\\qwen-code\\settings.json"
    format: jsonc
    effect: "System override settings. Source merge order makes system settings highest precedence."
    notes: "Can be redirected by QWEN_CODE_SYSTEM_SETTINGS_PATH."
  - os: macos
    scope: system
    path: "/Library/Application Support/QwenCode/system-defaults.json"
    format: jsonc
    effect: "System default settings. Lowest precedence loaded settings scope."
    notes: "Can be redirected by QWEN_CODE_SYSTEM_DEFAULTS_PATH."
  - os: linux
    scope: system
    path: "/etc/qwen-code/system-defaults.json"
    format: jsonc
    effect: "System default settings. Lowest precedence loaded settings scope."
    notes: "Can be redirected by QWEN_CODE_SYSTEM_DEFAULTS_PATH."
  - os: windows
    scope: system
    path: "C:\\ProgramData\\qwen-code\\system-defaults.json"
    format: jsonc
    effect: "System default settings. Lowest precedence loaded settings scope."
    notes: "Can be redirected by QWEN_CODE_SYSTEM_DEFAULTS_PATH."
  - os: macos
    scope: user
    path: "~/.qwen/.env"
    format: text
    effect: "User-level environment defaults for auth/provider/config variables."
    notes: "QWEN-specific user .env wins over ~/.env for duplicate keys; existing process env is not overwritten."
  - os: linux
    scope: user
    path: "~/.qwen/.env"
    format: text
    effect: "User-level environment defaults for auth/provider/config variables."
    notes: "QWEN-specific user .env wins over ~/.env for duplicate keys; existing process env is not overwritten."
  - os: windows
    scope: user
    path: "%USERPROFILE%\\.qwen\\.env"
    format: text
    effect: "User-level environment defaults for auth/provider/config variables."
    notes: "QWEN_HOME can redirect this home directory."
  - os: macos
    scope: repo
    path: ".qwen/.env"
    format: text
    effect: "Project environment defaults; can influence auth/provider behavior and wrapper output indirectly."
    notes: "Project env loading excludes configured variables except that variables from .qwen/.env are never excluded."
  - os: linux
    scope: repo
    path: ".qwen/.env"
    format: text
    effect: "Project environment defaults; can influence auth/provider behavior and wrapper output indirectly."
    notes: "Project env loading excludes configured variables except that variables from .qwen/.env are never excluded."
  - os: windows
    scope: repo
    path: ".qwen\\.env"
    format: text
    effect: "Project environment defaults; can influence auth/provider behavior and wrapper output indirectly."
    notes: "Project env loading excludes configured variables except that variables from .qwen/.env are never excluded."
env_vars:
  - name: "QWEN_HOME"
    effect: "Redirects the global configuration directory from ~/.qwen."
    notes: "Relative paths are resolved from cwd; empty string is unset."
  - name: "QWEN_RUNTIME_DIR"
    effect: "Redirects runtime output such as conversations, logs, and todos."
    notes: "Useful for separating ephemeral runtime data from persistent config."
  - name: "QWEN_SANDBOX"
    effect: "Enables sandboxing from the environment."
    notes: "Use with headless auto-approval to reduce host-risk."
  - name: "QWEN_SANDBOX_IMAGE"
    effect: "Selects sandbox image unless overridden by --sandbox-image."
    notes: "Precedence is --sandbox-image > QWEN_SANDBOX_IMAGE > tools.sandboxImage > built-in default."
  - name: "QWEN_CODE_SAFE_MODE"
    effect: "Enables safe mode, disabling settings-derived customizations and tools as documented."
    notes: "CLI --safe-mode is the explicit per-run equivalent."
  - name: "QWEN_CODE_SUPPRESS_YOLO_WARNING"
    effect: "Suppresses the headless YOLO-without-sandbox stderr warning."
    notes: "Only use after the wrapper deliberately accepts that risk."
  - name: "QWEN_CODE_UNATTENDED_RETRY"
    effect: "When true or 1, retries transient 429 and 529 provider errors indefinitely with stderr heartbeats."
    notes: "Must be paired with a wall-clock budget for deterministic automation."
  - name: "QWEN_CODE_DEBUG"
    effect: "Enables additional debug logging paths in some failure reports."
    notes: "Debug logs are not part of stdout JSONL."
  - name: "QWEN_CODE_SYSTEM_SETTINGS_PATH"
    effect: "Overrides system settings file path."
    notes: "Useful for managed deployments or tests."
  - name: "QWEN_CODE_SYSTEM_DEFAULTS_PATH"
    effect: "Overrides system defaults file path."
    notes: "Useful for managed deployments or tests."
  - name: "QWEN_CODE_TRUSTED_FOLDERS_PATH"
    effect: "Overrides trusted-folder state location."
    notes: "Workspace settings are ignored unless the workspace is trusted."
  - name: "QWEN_TELEMETRY_ENABLED"
    effect: "Overrides telemetry.enabled."
    notes: "Telemetry/logging is a secondary signal, not the primary stream."
  - name: "QWEN_TELEMETRY_OUTFILE"
    effect: "Can route telemetry to a file via telemetry settings."
    notes: "Use as secondary diagnostics if enabled; not equivalent to stream-json."
  - name: "QWEN_CODE_ENABLE_AGENT_TEAM"
    effect: "Enables experimental agent-team tools."
    notes: "Can introduce teammate/subagent approval behavior relevant to non-interactive runs."
  - name: "QWEN_CODE_DISABLE_CRON"
    effect: "Disables cron/loop tools."
    notes: "Affects available tools and possible task_notification events."
  - name: "QWEN_CODE_EMIT_TOOL_USE_SUMMARIES"
    effect: "Overrides experimental tool-use summary generation."
    notes: "Docs say SDK/non-interactive emission of the summary message is not yet wired."
io_contract:
  stdout: structured_only
  stderr: mixed
  stdin: prompt
  framing: jsonl
  noise_handling: "In preferred mode, parse stdout line-by-line as JSON and treat stderr as diagnostics/lifecycle adjunct. Always inspect exit code because some failures have no terminal result line."
  notes: "With --input-format stream-json, stdin changes from prompt text to JSON-line protocol input."
stream_contract:
  discriminator: "type"
  event_ordering: "system init/session_start precedes assistant/user/result messages for normal runs; stream_event records may precede completed assistant messages when partials are enabled; result is terminal when present."
  correlation_fields: ["session_id", "uuid", "message.id", "content[].id", "content[].tool_use_id", "event.tool_use_id", "parent_tool_use_id", "request_id"]
  terminal_event: "type=result"
  partial_message_events: true
  unknown_event_policy: "Skip unknown type or event.type, log at trace, and preserve raw JSON for drift analysis."
  notes: "Top-level type discriminates message/control families. Nested subtype distinguishes result/system/control payloads; stream_event.event.type distinguishes deltas."
session_metadata:
  session_id: "system.session_id and every message.session_id; emitted early in normal structured runs"
  cwd: "system.cwd in CLI-local init messages; docs examples omit exact guarantee"
  model: "system.model and assistant.message.model; requested/resolved distinction is not explicit"
  provider: "not emitted as a separate provider field; infer Qwen Code from wrapper/invocation"
  auth: "not emitted in stream-json; startup/auth failures may appear on stderr or as result.error.message"
  version: "system.qwen_code_version in CLI-local init message"
  mcp_servers: "system.mcp_servers array with name/status in CLI-local init message"
  permission_mode: "system.permission_mode in CLI-local init message"
  notes: "System init shape in source uses subtype init, while public docs examples show subtype session_start; parser should accept both."
stream_events:
  - event: "system/init"
    category: session
    fields: ["type", "subtype", "uuid", "session_id", "cwd", "tools", "mcp_servers", "model", "permission_mode", "slash_commands", "qwen_code_version", "agents"]
    notes: "Source-level system initialization message; docs examples call the subtype session_start."
  - event: "assistant"
    category: assistant
    fields: ["type", "uuid", "session_id", "parent_tool_use_id", "message.id", "message.model", "message.content", "message.usage", "message.stop_reason"]
    notes: "Completed assistant message. Content blocks include text, thinking, tool_use, and tool_result shapes."
  - event: "user"
    category: tool_result
    fields: ["type", "uuid", "session_id", "parent_tool_use_id", "message.role", "message.content[].tool_use_id", "message.content[].content", "message.content[].is_error"]
    notes: "Tool results are emitted as user messages containing tool_result blocks."
  - event: "result/success"
    category: session
    fields: ["type", "subtype", "uuid", "session_id", "is_error", "duration_ms", "duration_api_ms", "num_turns", "result", "usage", "modelUsage", "permission_denials", "structured_result"]
    notes: "Terminal success when emitted."
  - event: "result/error_during_execution"
    category: error
    fields: ["type", "subtype", "uuid", "session_id", "is_error", "duration_ms", "duration_api_ms", "num_turns", "usage", "permission_denials", "error.message"]
    notes: "Terminal execution error when emitted."
  - event: "result/error_max_turns"
    category: error
    fields: ["type", "subtype", "uuid", "session_id", "is_error", "duration_ms", "duration_api_ms", "num_turns", "usage", "permission_denials", "error.message"]
    notes: "Type union defines this subtype, but docs warn max-session-turns can exit with stderr only."
  - event: "stream_event/message_start"
    category: assistant
    fields: ["type", "uuid", "session_id", "parent_tool_use_id", "event.type", "event.message.id", "event.message.role", "event.message.model"]
    notes: "Only with --include-partial-messages."
  - event: "stream_event/content_block_start"
    category: assistant
    fields: ["type", "uuid", "session_id", "parent_tool_use_id", "event.type", "event.index", "event.content_block"]
    notes: "Starts text, thinking, or tool_use block."
  - event: "stream_event/content_block_delta"
    category: assistant
    fields: ["type", "uuid", "session_id", "parent_tool_use_id", "event.type", "event.index", "event.delta.type", "event.delta.text", "event.delta.thinking", "event.delta.partial_json"]
    notes: "Deltas are text_delta, thinking_delta, or input_json_delta. Tool inputs arrive as JSON-string deltas."
  - event: "stream_event/content_block_stop"
    category: assistant
    fields: ["type", "uuid", "session_id", "parent_tool_use_id", "event.type", "event.index"]
    notes: "Closes a content block."
  - event: "stream_event/message_stop"
    category: assistant
    fields: ["type", "uuid", "session_id", "parent_tool_use_id", "event.type"]
    notes: "Closes a streamed assistant message."
  - event: "stream_event/tool_progress"
    category: tool_call
    fields: ["type", "uuid", "session_id", "event.type", "event.tool_use_id", "event.content"]
    notes: "MCP progress data only when partial messages are enabled and the tool emits progress."
  - event: "stream_event/active_goal"
    category: plan
    fields: ["type", "uuid", "session_id", "event.type", "event.active_goal"]
    notes: "Active goal updates bypass message finalization guard."
  - event: "system/task_started"
    category: subagent
    fields: ["type", "subtype", "uuid", "session_id", "data.task_id", "data.tool_use_id", "data.description", "data.subagent_type"]
    notes: "Emitted for background agents/monitors."
  - event: "system/task_notification"
    category: subagent
    fields: ["type", "subtype", "uuid", "session_id", "data.task_id", "data.tool_use_id", "data.status", "data.usage.total_tokens", "data.usage.tool_uses", "data.usage.duration_ms"]
    notes: "Terminal/progress notification for background agents/monitors."
  - event: "system/worktree_started"
    category: other
    fields: ["type", "subtype", "uuid", "session_id", "data"]
    notes: "Worktree lifecycle record in source."
  - event: "system/worktree_restored"
    category: other
    fields: ["type", "subtype", "uuid", "session_id", "data.slug", "data.path", "data.branch"]
    notes: "Worktree lifecycle record in source."
  - event: "system/continue_turn_failed"
    category: error
    fields: ["type", "subtype", "uuid", "session_id", "data.error"]
    notes: "Structured diagnostic in stream-json session continuation when a result was already emitted."
  - event: "control_request/can_use_tool"
    category: permission
    fields: ["type", "request_id", "request.subtype", "request.tool_name", "request.tool_use_id", "request.input", "request.permission_suggestions", "request.blocked_path"]
    notes: "Only in --input-format stream-json/control mode."
  - event: "control_response"
    category: permission
    fields: ["type", "response.subtype", "response.request_id", "response.response", "response.error"]
    notes: "Acknowledges control requests or errors in control mode."
  - event: "control_cancel_request"
    category: other
    fields: ["type", "request_id"]
    notes: "Cancels a pending control request in control mode."
tools:
  - name: "read/write/edit/shell built-ins"
    call_visible: true
    result_visible: true
    metadata: ["tool_use.id", "tool_use.name", "tool_use.input", "tool_result.tool_use_id", "tool_result.content", "tool_result.is_error"]
    notes: "Tool starts appear as assistant tool_use content blocks; results appear as user tool_result blocks. Command stdout/stderr are not normalized as separate fields."
  - name: "MCP tools"
    call_visible: true
    result_visible: true
    metadata: ["system.mcp_servers", "tool_use.name", "tool_progress.content", "tool_result.content"]
    notes: "MCP server list is in init metadata; progress is visible only with partial messages."
  - name: "structured_output"
    call_visible: true
    result_visible: true
    metadata: ["tool_use.name", "tool_use.input", "result.structured_result", "result.result"]
    notes: "Only exists with --json-schema; final object should be read from result.structured_result."
  - name: "agent/background task tools"
    call_visible: true
    result_visible: true
    metadata: ["parent_tool_use_id", "system.task_started", "system.task_notification", "data.usage"]
    notes: "Parent stream sees task lifecycle and parent_tool_use_id. Inner subagent tool-call completeness is not guaranteed as a full nested transcript."
  - name: "cron/loop/team experimental tools"
    call_visible: true
    result_visible: true
    metadata: ["system.task_started", "system.task_notification", "permission/control events"]
    notes: "Availability depends on settings/env. Team approvals can auto-cancel outside stream-json control mode unless YOLO is active."
completion:
  success_event: "type=result, subtype=success, is_error=false"
  failure_event: "type=result with is_error=true when emitted; otherwise classify by non-zero exit code and stderr"
  exit_code_reliable: true
  result_fields: ["result", "structured_result", "error.message", "duration_ms", "duration_api_ms", "num_turns", "permission_denials"]
  cost_fields: []
  usage_fields: ["usage.input_tokens", "usage.output_tokens", "usage.total_tokens", "usage.cache_creation_input_tokens", "usage.cache_read_input_tokens", "modelUsage.*", "data.usage.total_tokens"]
  notes: "Exit code remains necessary. Structured-output docs explicitly state max-session-turns exits 53 and signal interrupts exit 130 with stderr only; budget aborts use 55."
blocking_behavior:
  permissions: configurable
  questions: unknown
  tool_approvals: configurable
  notes: "Use --approval-mode yolo/auto/auto-edit/plan, --yolo, or stream-json control responses for deterministic runs. Default manual approval cannot be answered by a non-control wrapper."
subagents:
  supported: true
  start_visible: true
  stop_visible: true
  nested_events_visible: true
  prompt_injection_supported: true
  metadata_fields: ["parent_tool_use_id", "system.task_started.data", "system.task_notification.data", "agents", "data.usage"]
  notes: "Subagent/background-task lifecycle is visible through task_started/task_notification and parent_tool_use_id. Full nested event parity and per-subagent model identity remain partially unverified."
use_cases:
  - name: "plan_cap_approaching"
    detectable: false
    event_types: []
    fields: []
    hook_parity: "unknown"
    notes: "No plan/quota near-cap event verified in stream-json."
  - name: "plan_capped"
    detectable: true
    event_types: ["result", "stderr"]
    fields: ["error.message", "exit_code"]
    hook_parity: "unknown"
    notes: "Provider quota or max-turn/budget caps can be inferred from result errors or exit codes, but reset windows/upgrades are not normalized."
  - name: "no_funds"
    detectable: true
    event_types: ["result", "stderr"]
    fields: ["error.message", "exit_code"]
    hook_parity: "unknown"
    notes: "Detect by provider error text; no dedicated no_funds discriminator verified."
  - name: "auth"
    detectable: true
    event_types: ["result", "stderr"]
    fields: ["error.type", "error.message", "exit_code"]
    hook_parity: "unknown"
    notes: "Missing/invalid auth may fail before or during the run; auth kind is not emitted as stable stream metadata."
  - name: "permission_read_denied"
    detectable: true
    event_types: ["control_request", "control_response", "user/tool_result", "result"]
    fields: ["request.blocked_path", "request.tool_name", "message.content[].is_error", "permission_denials[].tool_input"]
    hook_parity: "unknown"
    notes: "Permission denials are collected in result.permission_denials but do not explicitly classify read vs write."
  - name: "permission_write_denied"
    detectable: true
    event_types: ["control_request", "control_response", "user/tool_result", "result"]
    fields: ["request.blocked_path", "request.tool_name", "message.content[].is_error", "permission_denials[].tool_input"]
    hook_parity: "unknown"
    notes: "Classify by tool name/input path heuristics; no stable write_denied event name verified."
  - name: "tokens_consumed"
    detectable: true
    event_types: ["assistant", "result", "system/task_notification"]
    fields: ["message.usage.input_tokens", "message.usage.output_tokens", "result.usage", "result.modelUsage", "data.usage.total_tokens"]
    hook_parity: "unknown"
    notes: "Units are tokens. Result usage is session aggregate from metrics; assistant usage is message-level."
  - name: "model_used"
    detectable: true
    event_types: ["system/init", "assistant", "result"]
    fields: ["model", "message.model", "modelUsage"]
    hook_parity: "unknown"
    notes: "Model string is emitted, but alias vs resolved backend is not explicitly marked."
  - name: "model_fallback"
    detectable: false
    event_types: []
    fields: []
    hook_parity: "unknown"
    notes: "No dedicated fallback event verified."
  - name: "human_in_loop"
    detectable: true
    event_types: ["control_request/can_use_tool", "stderr"]
    fields: ["request.subtype", "request.tool_name", "request.blocked_path"]
    hook_parity: "unknown"
    notes: "In control mode, approval requests are explicit. In one-shot mode, teammate approval can auto-cancel with stderr notice unless YOLO is active."
  - name: "session_resumable"
    detectable: true
    event_types: ["system/init", "assistant", "result"]
    fields: ["session_id"]
    hook_parity: "unknown"
    notes: "Use session_id with --resume when a specific saved session should be resumed."
  - name: "subagent_prompt_injection"
    detectable: true
    event_types: ["system/init", "system/task_started"]
    fields: ["agents", "data.subagent_type", "data.description"]
    hook_parity: "unknown"
    notes: "Main-session system/append prompt can instruct subagents indirectly; SDK initialize can also supply agents."
headless_constraints:
  - constraint: "stream-json input mode is documented as under construction and intended for SDK integration."
    mitigation: "Prefer one-shot qwen -p with --output-format stream-json for Claudine unless Claudine implements the control protocol."
    notes: "Control mode requires bidirectional stdin/stdout handling."
  - constraint: "Some failures do not emit a terminal result line."
    mitigation: "Always combine stream parsing with process exit-code and stderr classification."
    notes: "Docs name max-session-turns exit 53 and signal interrupts exit 130 as stderr-only cases."
  - constraint: "Default approval mode cannot prompt in a plain non-interactive wrapper."
    mitigation: "Use --approval-mode yolo/auto/auto-edit/plan, restrict tools, or implement --input-format stream-json control responses."
    notes: "YOLO without sandbox is risky and emits a warning unless suppressed."
  - constraint: "--json-schema conflicts with --input-format stream-json, ACP, and prompt-interactive."
    mitigation: "Use --json-schema only with one-shot headless text input/output modes."
    notes: "Prefer result.structured_result over result.result when using JSON formats."
  - constraint: "stderr contains meaningful lifecycle diagnostics."
    mitigation: "Do not discard stderr; capture and classify warnings, auth, MCP failures, retry heartbeats, and budget/interrupt failures."
    notes: "stdout remains parse-only in stream-json mode."
  - constraint: "Tool command stdout/stderr are folded into tool_result content."
    mitigation: "Classify shell outcomes from tool_result content and is_error; do not expect structured per-command stdout/stderr fields."
    notes: "No dedicated command exit-code field verified in stream-json."
quirks:
  - "Public docs examples use system subtype session_start, while current CLI source builds subtype init for normal non-interactive system metadata."
  - "stream-json without --include-partial-messages streams completed messages but not token/content deltas."
  - "stream_event content_block_delta for tool input uses delta.partial_json as a JSON string, not an object."
  - "The strongest schema is TypeScript source, not a formal JSON Schema; parser drift should be expected across releases."
  - "JSON output carries richer stats than stream-json in current source; stream-json is better for live supervision but weaker for post-run auditing."
  - "YOLO auto-approves host-level shell/write/edit unless sandbox is separately enabled."
gaps:
  - "No formal JSON Schema or protocol version for CLI stream-json was found."
  - "Exact package version tested at runtime was not captured; findings are from official docs and main-branch source on 2026-07-03."
  - "Exact auth failure payloads across Qwen OAuth, Coding Plan, OpenAI-compatible providers, and custom modelProviders were not exhaustively sampled."
  - "No stable dedicated file-change event was verified; file changes must be inferred from tool_use/tool_result content."
  - "No stable cost field was verified; token usage exists but currency cost does not."
  - "No dedicated model fallback event was verified."
  - "Full nested subagent transcript completeness and per-subagent model identity need fixture capture."
claudine_strategy:
  preferred_invocation: 'qwen -p "$PROMPT" --output-format stream-json --include-partial-messages --approval-mode auto --max-session-turns <N> --max-wall-time <duration>'
  required_flags: ["--output-format stream-json", "--include-partial-messages", "--prompt/-p or stdin prompt", "--max-session-turns", "--max-wall-time"]
  conflicting_flags: ["--prompt-interactive", "--input-format stream-json unless implementing control protocol", "--json-schema with --input-format stream-json", "--acp with --json-schema"]
  parser_notes: "Parse stdout as JSONL using type, subtype, and event.type. Accept both system/init and documented system/session_start. Treat result as terminal only when present; classify missing terminal result with exit code and stderr."
  wrapper_notes: "Capture stderr, preserve exit code, set deterministic approval/sandbox/budget flags, avoid bare --resume, and keep unknown events for drift analysis."
data_format: jsonl
changes: []
requires_claudine_update: true
reason: "Claudine should prefer Qwen stream-json with --include-partial-messages, but parser metadata must account for system/init vs session_start, stream_event deltas, control mode, stderr-only failures, and result-missing exit-code classification."
---

# Qwen CLI Non-Interactive Sessions

## Summary

Qwen Code can run non-interactively with structured output. The best Claudine mode is `qwen -p "..." --output-format stream-json --include-partial-messages`: stdout becomes line-delimited JSON, completed messages are available as they occur, and partial `stream_event` records expose live text, thinking, tool inputs, MCP progress, and active-goal updates. The official headless docs define `text`, `json`, and `stream-json`; they explicitly describe `stream-json` as one complete JSON object per line and name `--include-partial-messages` as the flag that adds `message_start`, `content_block_delta`, and related live events.

The main wrapper risk is that `stream-json` is not a complete lifecycle oracle. The structured-output docs say some failures, including max-session-turns and signal interrupts, can exit with stderr only and no final `result` event. Claudine must parse stdout JSONL and capture stderr and exit status. The strongest schema evidence is TypeScript source in Qwen's `packages/cli/src/nonInteractive` tree, not a published JSON Schema, and there is a visible naming drift risk: public examples show `system` subtype `session_start`, while current CLI source builds a normal init message with subtype `init`.

## Non-Interactive Entry Points

The normal one-shot entry point is `qwen --prompt/-p`, or piped stdin. The headless docs say headless mode accepts prompts through command-line arguments or stdin, supports file redirection and piping, and can resume prior project-scoped sessions. The same page documents:

| Entry point | Prompt source | Session behavior | Claudine fit |
| --- | --- | --- | --- |
| `qwen -p "prompt"` | argv | Fresh one-shot headless session | Preferred |
| `cat file | qwen -p "prompt"` | argv plus stdin | Fresh one-shot with extra context | Preferred |
| `qwen --continue -p "prompt"` | argv | Continues most recent project session | Use only when requested |
| `qwen --resume <session-id> -p "prompt"` | argv | Resumes specific session | Safe if ID supplied |
| `qwen --input-format stream-json --output-format stream-json` | JSONL stdin | Long-lived SDK/control protocol | Only if Claudine implements bidirectional protocol |

Qwen also supports per-run prompt controls: `--system-prompt` replaces the main-session prompt for that run, and `--append-system-prompt` adds instructions after the built-in prompt and loaded memory. For automation, Claudine should set bounded execution controls such as `--max-session-turns` and `--max-wall-time`. The docs distinguish max-session-turns exit code `53`, run-budget exit code `55`, and signal interrupt exit code `130`.

## Output Formats

Qwen's non-interactive output formats are selected with `--output-format`.

| Format | CLI value | Streamed | Shape | Notes |
| --- | --- | --- | --- | --- |
| Text | `text` | Yes, as human output | Plain text | Default. Not safe for live parser supervision. With `--json-schema`, successful stdout is the validated JSON payload line. |
| Buffered JSON | `json` | No | Single JSON array | Includes message log and final `result`; useful for post-run stats, but no live progress. |
| Streaming JSON | `stream-json` | Yes | JSONL / NDJSON-like | Best Claudine format. Each stdout line is a complete JSON object. |
| Stream JSON input/control | `--input-format stream-json --output-format stream-json` | Yes | Bidirectional JSONL | SDK/control protocol, not prompt text. Upstream docs call it under construction. |

Claudine should prefer `stream-json` plus `--include-partial-messages`. Plain `stream-json` is already structured, but it emits completed message objects; the partial flag adds live `stream_event` records for progress and tool inputs. That matters because Claudine is supervising a running process, not just collecting a final answer.

Buffered `json` remains useful for fixture capture and post-run audits. The current source only attaches the full `stats` object to result messages when the selected output format is `json`; stream-json carries `usage` and optional `modelUsage`, but not the same full stats object. For live wrapping, that tradeoff favors stream-json; for offline usage reports, Claudine may optionally run or replay buffered JSON fixtures.

## Schema Sources

There is no formal JSON Schema for the CLI stream. The authoritative shape is the TypeScript union in [`packages/cli/src/nonInteractive/types.ts`](https://github.com/QwenLM/qwen-code/blob/main/packages/cli/src/nonInteractive/types.ts). It defines top-level `CLIMessage` variants (`user`, `assistant`, `system`, `result`, `stream_event`) and `ControlMessage` variants (`control_request`, `control_response`, `control_cancel_request`).

The output adapters are also schema evidence. [`BaseJsonOutputAdapter.ts`](https://github.com/QwenLM/qwen-code/blob/main/packages/cli/src/nonInteractive/io/BaseJsonOutputAdapter.ts) builds assistant messages, tool-result user messages, result messages, `permission_denials`, and `structured_result`. [`StreamJsonOutputAdapter.ts`](https://github.com/QwenLM/qwen-code/blob/main/packages/cli/src/nonInteractive/io/StreamJsonOutputAdapter.ts) writes each message as `JSON.stringify(message) + "\n"` and emits partial events only when `includePartialMessages` is enabled.

The SDK protocol types in [`packages/sdk-typescript/src/types/protocol.ts`](https://github.com/QwenLM/qwen-code/blob/main/packages/sdk-typescript/src/types/protocol.ts) are useful for control mode, but they are secondary for Claudine's one-shot wrapper because the CLI-local adapters define the actual stdout records. The official docs provide examples and behavior descriptions, but not a complete schema.

## IO Contract

In preferred mode, stdout is parse-only JSONL. Each line is independently parseable JSON. Stderr is not ignorable: startup warnings, debug log paths, MCP server failures, YOLO-without-sandbox warnings, persistent retry heartbeats, auth errors, and some terminal failures can appear there. The headless docs explicitly say persistent retry prints heartbeat messages to stderr and keeps JSON stdout clean.

Stdin is ordinary prompt/context text in default input mode. When `--input-format stream-json` is selected, stdin becomes protocol input. A wrapper that only writes a prompt and closes stdin should not use control mode unless it implements the request/response contract.

## Stream Contract

The top-level discriminator is `type`. Important nested discriminators are:

| Path | Meaning |
| --- | --- |
| `subtype` | System/result/control subtype, such as `init`, `success`, `error_during_execution`, or control response status. |
| `event.type` | Partial stream event subtype under top-level `type: "stream_event"`. |
| `message.content[].type` | Content block subtype: `text`, `thinking`, `tool_use`, `tool_result`. |
| `event.delta.type` | Delta subtype: `text_delta`, `thinking_delta`, `input_json_delta`. |

Normal completion is a top-level `result` record. Success uses `subtype: "success"` and `is_error: false`; execution failures that reach the adapter use `is_error: true` with `error.message`. Because not all failures emit a result, Claudine should treat `result` as a preferred terminal event, not the only terminal signal.

Correlation fields are `session_id`, message `uuid`, assistant `message.id`, tool-use `id`, tool-result `tool_use_id`, partial `event.tool_use_id`, `parent_tool_use_id`, and control `request_id`. Tool input deltas are stringified JSON in `event.delta.partial_json`; parsers should decode that string if they need the object.

Unknown events should be skipped but retained in raw form for drift reports. The source-level schema is not versioned as a JSON protocol, so fail-open parsing with trace logging is safer than rejecting a whole run.

## Session Metadata

The source-level init system message includes `session_id`, `cwd`, `tools`, `mcp_servers`, `model`, `permission_mode`, `slash_commands`, `qwen_code_version`, and `agents`. The helper that builds it uses `subtype: "init"`. Public docs examples show a `system` message with `subtype: "session_start"`, `session_id`, and `model`. Claudine should accept both names and map both to session start.

Model identity appears as `system.model` and `assistant.message.model`. The docs and source do not prove whether that string is the requested alias or a fully resolved backend model in every provider configuration. Auth source is not emitted as a stable field. Provider identity is implicit from the wrapper invocation.

Qwen session history is project-scoped. The headless docs state that session data is stored as JSONL under `~/.qwen/projects/<sanitized-cwd>/chats` and that `--continue`/`--resume` restore conversation history, tool outputs, and compression checkpoints.

## Event Families

The core event families visible to Claudine are:

| Family | Records | Use |
| --- | --- | --- |
| Session | `system/init` or documented `system/session_start`, `result/*` | Session identity, metadata, final status. |
| Assistant | `assistant`, `stream_event/message_start`, content block events, `message_stop` | Live and completed model output. |
| Reasoning | `thinking` content blocks and `thinking_delta` | Reasoning-like content when the provider emits it. |
| Tool calls | Assistant `tool_use` blocks, partial `input_json_delta`, `tool_progress` | Tool start, input, progress. |
| Tool results | User `tool_result` blocks | Tool output/error content. |
| Permission/control | `control_request`, `control_response`, `control_cancel_request` | SDK/control mode approvals and runtime control. |
| Subagents/tasks | `system/task_started`, `system/task_notification`, `parent_tool_use_id` | Background/subagent lifecycle and usage. |
| Errors | `result` with `is_error: true`, `system/continue_turn_failed`, stderr, exit code | Terminal and mid-session failure classification. |

There is no dedicated file-change event verified in the CLI stream. Claudine should infer file changes from tool names, inputs, and results unless future Qwen releases add explicit events.

## Tools

Built-in tool calls are visible before execution as assistant `tool_use` blocks. The block has `id`, `name`, and `input`. The result is emitted as a `user` message containing a `tool_result` block with `tool_use_id`, optional `content`, and optional `is_error`. This gives Claudine enough to join calls to results by tool ID.

MCP tools use the same content-block mechanism. The init metadata lists MCP servers with `name` and `status`, and the stream can emit `tool_progress` events with `event.tool_use_id` and structured MCP progress content when partial messages are enabled.

Shell command details are not normalized into separate `stdout`, `stderr`, and `exit_code` fields in the stream shape verified here. The adapter folds tool output into `tool_result.content`, and errors are indicated by `is_error` and error text. Wrappers should not assume per-command exit-code structure unless a specific tool result payload proves it.

The `structured_output` tool is special. With `--json-schema`, Qwen registers a synthetic tool whose parameter schema is the caller's JSON Schema. In JSON formats, the final result carries both `result` as a stringified payload and `structured_result` as the raw object. The docs recommend reading `structured_result` for machine use.

## Completion and Exit Status

A normal stream-json success ends with:

```json
{"type":"result","subtype":"success","is_error":false,"result":"...","usage":{}}
```

Execution errors that reach the adapter end with `type: "result"`, `is_error: true`, and `error.message`. In current source, the run attempts to emit this terminal envelope for JSON and stream-json failures before invoking the error handler.

Exit code still matters. The structured-output docs warn that not all failures emit a result on stdout: max-session-turns exits `53` and signal interrupts exit `130` with stderr only. Headless docs add budget aborts as exit `55`. Claudine should classify completion in this order:

1. If a terminal `result` line exists, use it for final text/error, usage, and permission denials.
2. Always preserve the process exit code. A non-zero exit after a success result is suspicious and should be reported.
3. If no terminal result exists, classify from exit code and stderr.

## Blocking Behavior

Qwen has configurable approval modes: `plan`, `default`, `auto-edit`, `auto`, and `yolo`. The headless docs warn that `--yolo` auto-approves shell/write/edit but does not enable sandboxing. It prints a one-line stderr warning when YOLO is used without sandbox unless `QWEN_CODE_SUPPRESS_YOLO_WARNING=1` is set.

In `--input-format stream-json` control mode, permission requests can be represented as `control_request` with `request.subtype: "can_use_tool"`, `tool_name`, `tool_use_id`, `input`, suggestions, and `blocked_path`. A host can respond with `control_response`. In plain one-shot non-stream-json mode there is no interactive prompt channel. Source comments show teammate approval requests auto-proceed in YOLO; otherwise they are auto-cancelled with a stderr notice and a model-visible team notice so the run does not hang for 600 seconds.

For deterministic Claudine runs, prefer bounded permissions. Use `--approval-mode auto` or a stricter mode with tool allow/deny settings for routine automation; use `--approval-mode yolo` only inside an isolated sandbox/container and with explicit budgets.

## Subagents

Subagents/background tasks can run in headless mode. Source emits `system/task_started` with `task_id`, optional `tool_use_id`, `description`, and sometimes `subagent_type`. It later emits `system/task_notification` with `task_id`, optional `tool_use_id`, `status`, and optional usage (`total_tokens`, `tool_uses`, `duration_ms`). Assistant and user messages also carry `parent_tool_use_id`, which lets Claudine associate nested work with the parent tool call.

This is enough for high-level progress and final task status. It does not prove full nested transcript parity for every subagent inner tool call or per-subagent model identity. Claudine should record `task_started`/`task_notification` as subagent lifecycle signals and treat deeper nested events as best-effort.

## Use Case Detection

| Use case | Detectable | Signal |
| --- | --- | --- |
| `tokens_consumed` | Yes | `assistant.message.usage`, `result.usage`, optional `result.modelUsage`, task notification `data.usage.total_tokens`. |
| `model_used` | Yes | `system.model`, `assistant.message.model`, `modelUsage` keys when present. |
| `session_resumable` | Yes | `session_id` in system/assistant/result messages; use with `--resume <session-id>`. |
| `human_in_loop` | Yes in control mode | `control_request` with `request.subtype: "can_use_tool"`; in plain mode, stderr/team notice can indicate auto-cancelled approvals. |
| `permission_read_denied` | Partially | `permission_denials[]`, tool_result `is_error`, control `blocked_path`; classify read/write by tool name and path. |
| `permission_write_denied` | Partially | Same as read denied; no dedicated write-denied event verified. |
| `auth` | Partially | stderr or `result.error.message`; no stable auth kind field. |
| `plan_capped` | Partially | exit code `53` for max turns, `55` for budget, provider error text for quota; no reset-window field. |
| `plan_cap_approaching` | No | No verified near-cap event. |
| `no_funds` | Partially | Provider error text only. |
| `model_fallback` | No | No verified fallback event. |
| `subagent_prompt_injection` | Partially | Main prompt/system prompt can instruct subagents indirectly; SDK initialize can define agents. |

## Headless Constraints

`--input-format stream-json` is not just another output format. It makes stdin a bidirectional JSON-line protocol, and upstream docs call it under construction and intended for SDK integration. Claudine should avoid it until it can answer `control_request` messages, cancel requests, and handle initialization.

`--json-schema` is useful for final structured answers, but it has its own terminal contract. It is rejected with prompt-interactive, stream-json input mode, and ACP. In JSON/stream-json output, read the final `structured_result`; in text mode, stdout is the JSON payload line instead of the usual prose.

Unbounded unattended runs are unsafe. Use `--max-session-turns`, `--max-wall-time`, and optionally `--max-tool-calls`. The docs note that `--max-tool-calls` only counts top-level dispatches, not inner subagent tool calls, so excluding or constraining the `agent` tool is necessary for tight caps.

## Timeline

This research was verified on 2026-07-03 against the Qwen Code official docs last updated on 2026-07-02 and current `main` branch source paths under `packages/cli/src/nonInteractive`. The upstream docs describe `stream-json` input as under construction, and the schema is source-defined rather than formally versioned, so Claudine should schedule drift checks against the TypeScript union and adapters.

## Quirks and Gaps

The biggest parser footgun is `system` subtype naming. The public docs show `session_start`; current source builds `init`. Both carry session metadata and should normalize to Claudine session start.

The second footgun is assuming stdout alone proves completion. It does not. Missing terminal `result` plus exit `53`, `55`, or `130` is meaningful and should not be rendered as an ambiguous parser failure.

Important gaps remain: exact auth error envelopes across all auth/provider modes, exact rate-limit/quota/no-funds payloads, full nested subagent event parity, file-change-specific events, command exit-code structure, and whether model identity is always resolved rather than requested.

## Claudine Integration Notes

Recommended default:

```sh
qwen -p "$PROMPT" \
  --output-format stream-json \
  --include-partial-messages \
  --approval-mode auto \
  --max-session-turns "$MAX_TURNS" \
  --max-wall-time "$MAX_WALL_TIME"
```

For high-trust disposable CI, `--approval-mode yolo --sandbox` or container isolation may be acceptable. For shared machines, prefer `--sandbox`, explicit allow/deny tool configuration, and avoid YOLO.

Parser requirements:

| Requirement | Handling |
| --- | --- |
| Framing | Parse stdout by newline; each line should be a complete JSON object. |
| Discriminator | Use top-level `type`, then nested `subtype` or `event.type`. |
| Session start | Normalize both `system/init` and documented `system/session_start`. |
| Partial messages | Enable and parse `stream_event`; reconstruct deltas by `session_id`, `message.id`, and block `index`. |
| Tool joins | Join `tool_use.id`, `tool_result.tool_use_id`, and `event.tool_use_id`. |
| Completion | Prefer terminal `result`, but always inspect exit code and stderr. |
| Unknown events | Preserve raw JSON and continue. |

Avoid `--input-format stream-json` until Claudine has a bidirectional protocol adapter. Avoid bare `--resume` because it can imply interactive selection. Do not discard stderr.

## Changelog

Initial Claudine research file for Qwen CLI non-interactive sessions.

## Sources

- [Qwen Code Headless Mode](https://qwenlm.github.io/qwen-code-docs/en/users/features/headless/) documents `--prompt`, stdin, resume, output formats, `stream-json`, partial messages, budgets, YOLO safety, and persistent retry.
- [Qwen Code Configuration](https://qwenlm.github.io/qwen-code-docs/en/users/configuration/settings/) documents output/input format flags, approval modes, sandbox, settings, environment variables, telemetry, MCP settings, and config precedence.
- [Qwen Structured Output](https://qwenlm.github.io/qwen-code-docs/en/users/features/structured-output/) documents `--json-schema`, `structured_result`, restrictions, and failure behavior where some failures emit no stdout result.
- [Qwen structured-output design](https://qwenlm.github.io/qwen-code-docs/en/design/structured-output/structured-output/) explains the synthetic `structured_output` tool and validation approach.
- [CLI nonInteractive types](https://github.com/QwenLM/qwen-code/blob/main/packages/cli/src/nonInteractive/types.ts) defines the TypeScript unions for CLI messages, stream events, control requests, result messages, usage, and permission denials.
- [BaseJsonOutputAdapter](https://github.com/QwenLM/qwen-code/blob/main/packages/cli/src/nonInteractive/io/BaseJsonOutputAdapter.ts) builds result messages, tool results, permission denials, and structured output fields.
- [StreamJsonOutputAdapter](https://github.com/QwenLM/qwen-code/blob/main/packages/cli/src/nonInteractive/io/StreamJsonOutputAdapter.ts) emits JSONL and partial stream events.
- [nonInteractive session manager](https://github.com/QwenLM/qwen-code/blob/main/packages/cli/src/nonInteractive/session.ts) implements bidirectional stream-json session/control behavior.
- [runNonInteractive](https://github.com/QwenLM/qwen-code/blob/main/packages/cli/src/nonInteractiveCli.ts) implements one-shot headless execution, budgets, task notifications, final result emission, and error handling.
- [Qwen SDK protocol types](https://github.com/QwenLM/qwen-code/blob/main/packages/sdk-typescript/src/types/protocol.ts) provide secondary TypeScript evidence for SDK/control mode.
