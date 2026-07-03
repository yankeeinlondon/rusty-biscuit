---
$schema: ./_schema.yaml
created: 2026-04-06
last_updated: 2026-07-02
agent: codex
model: default
docs: https://code.claude.com/docs/en/cli-reference
invocation:
  - command: "claude -p \"PROMPT\" --output-format stream-json --verbose"
    stdin_support: true
    prompt_arg: "Prompt may be the positional argv prompt; stdin is prompt text when no prompt argument is supplied."
    notes: "Starts a print-mode non-interactive session and emits one JSON event per stdout line."
  - command: "claude -p --input-format stream-json --output-format stream-json --verbose"
    stdin_support: true
    prompt_arg: "stdin JSON message stream"
    notes: "Bidirectional streaming input/output mode for SDK-style clients; use --replay-user-messages if caller needs user-message acknowledgments on stdout."
  - command: "claude -p \"PROMPT\" --output-format json"
    stdin_support: true
    prompt_arg: "Prompt may be argv or stdin text."
    notes: "Single final JSON object after the agent completes; useful for request/reply scripts but weak for live wrapping."
  - command: "claude -p \"PROMPT\" --output-format text"
    stdin_support: true
    prompt_arg: "Prompt may be argv or stdin text."
    notes: "Default print-mode output; human text only."
  - command: "claude --resume SESSION_ID -p \"PROMPT\" --output-format stream-json --verbose"
    stdin_support: true
    prompt_arg: "Prompt may be argv or stdin text."
    notes: "Resumes a persisted session by ID or name; --fork-session creates a new branched session."
  - command: "claude --continue -p \"PROMPT\" --output-format stream-json --verbose"
    stdin_support: true
    prompt_arg: "Prompt may be argv or stdin text."
    notes: "Continues the most recent session for the current project/worktree."
output_formats:
  - name: text
    cli_value: text
    stream: false
    format: text
    description: "Human-readable final assistant text on stdout."
    side_effects: "No machine-readable lifecycle, tool, usage, or session metadata."
  - name: json
    cli_value: json
    stream: false
    format: json
    description: "Single final JSON result after the workflow finishes."
    side_effects: "Intermediate tool calls, progress, permission denials, hook events, and partial assistant deltas are not available live."
  - name: stream-json
    cli_value: stream-json
    stream: true
    format: ndjson
    description: "Newline-delimited JSON SDKMessage stream on stdout with init, assistant, tool, system, usage, and terminal result records."
    side_effects: "Requires -p/--print; --verbose is required by several stream-related flags and is the safest wrapper default. Hook events and partial raw API stream events are opt-in."
schema_sources:
  - url: https://code.claude.com/docs/en/agent-sdk/typescript
    schema_type: typescript
    formal: true
    notes: "Official TypeScript SDK reference documents SDKMessage, SDKResultMessage, SDKSystemMessage, SDKPartialAssistantMessage, and related unions used by stream-json."
  - url: https://registry.npmjs.org/@anthropic-ai/claude-agent-sdk/-/claude-agent-sdk-0.3.199.tgz
    schema_type: typescript
    formal: true
    notes: "Bundled package declaration package/sdk.d.ts is the strongest local schema evidence inspected for Claude Code 2.1.199 / SDK 0.3.199."
  - url: https://code.claude.com/docs/en/agent-sdk/python
    schema_type: other
    formal: true
    notes: "Official Python SDK types confirm the same message families but are less discriminated than the TypeScript declaration surface."
  - url: https://code.claude.com/docs/en/agent-sdk/streaming-output
    schema_type: examples
    formal: false
    notes: "Documents partial-message behavior and raw Claude API stream event nesting."
  - url: https://code.claude.com/docs/en/cli-reference
    schema_type: examples
    formal: false
    notes: "Official CLI reference defines the output-format, input-format, verbose, partial, hook, and permission flags."
cli_params:
  - flag: "-p, --print"
    value: null
    description: "Print response without interactive mode; required for output-format and most automation flags."
    example: "claude -p \"summarize\""
  - flag: "--output-format"
    value: "text | json | stream-json"
    description: "Selects print-mode output format."
    example: "claude -p --output-format stream-json \"query\""
  - flag: "--input-format"
    value: "text | stream-json"
    description: "Selects print-mode input framing; stream-json is SDK-style streaming input."
    example: "claude -p --input-format stream-json --output-format stream-json --verbose"
  - flag: "--verbose"
    value: null
    description: "Enables full structured streaming details and is required by include-partial-messages, include-hook-events, and prompt-suggestions examples."
    example: "claude -p --output-format stream-json --verbose \"query\""
  - flag: "--include-partial-messages"
    value: null
    description: "Adds type=stream_event records containing raw Claude API streaming events."
    example: "claude -p --output-format stream-json --verbose --include-partial-messages \"query\""
  - flag: "--include-hook-events"
    value: null
    description: "Adds hook_started, hook_progress, and hook_response lifecycle messages to stream-json."
    example: "claude -p --output-format stream-json --verbose --include-hook-events \"query\""
  - flag: "--prompt-suggestions"
    value: null
    description: "Emits prompt_suggestion messages after each turn; requires print mode, stream-json, and verbose."
    example: "claude -p --prompt-suggestions --output-format stream-json --verbose \"query\""
  - flag: "--replay-user-messages"
    value: null
    description: "Re-emits stdin user messages on stdout when both input and output are stream-json."
    example: "claude -p --input-format stream-json --output-format stream-json --verbose --replay-user-messages"
  - flag: "--json-schema"
    value: "JSON Schema"
    description: "Requests validated final structured_output after the workflow completes; this is output payload validation, not the stream schema."
    example: "claude -p --json-schema '{\"type\":\"object\"}' \"query\""
  - flag: "--max-turns"
    value: "positive integer"
    description: "Limits agentic turns; terminal result subtype becomes error_max_turns when reached."
    example: "claude -p --max-turns 3 \"query\""
  - flag: "--max-budget-usd"
    value: "decimal USD"
    description: "Stops when API cost budget is reached; terminal result subtype becomes error_max_budget_usd."
    example: "claude -p --max-budget-usd 5.00 \"query\""
  - flag: "--model"
    value: "alias or full model name"
    description: "Selects model and overrides model setting and ANTHROPIC_MODEL."
    example: "claude -p --model sonnet \"query\""
  - flag: "--permission-mode"
    value: "default | acceptEdits | plan | auto | dontAsk | bypassPermissions"
    description: "Sets initial permission behavior and overrides settings permissions.defaultMode."
    example: "claude -p --permission-mode dontAsk \"query\""
  - flag: "--permission-prompt-tool"
    value: "MCP tool name"
    description: "Routes non-interactive permission prompts to a programmable MCP tool."
    example: "claude -p --permission-prompt-tool mcp_auth_tool \"query\""
  - flag: "--dangerously-skip-permissions"
    value: null
    description: "Legacy/alias path into bypassPermissions behavior; only suitable for isolated sandboxes."
    example: "claude -p --dangerously-skip-permissions \"query\""
  - flag: "--add-dir"
    value: "directories..."
    description: "Adds extra filesystem roots available to tools."
    example: "claude -p --add-dir ../other \"query\""
  - flag: "--mcp-config"
    value: "JSON file or JSON string"
    description: "Loads MCP server configuration for the session."
    example: "claude -p --mcp-config ./mcp.json \"query\""
  - flag: "--resume, -r"
    value: "session ID or name"
    description: "Resumes a specific session; with --fork-session branches to a new session ID."
    example: "claude --resume abc123 -p \"continue\" --output-format stream-json --verbose"
  - flag: "--continue, -c"
    value: null
    description: "Continues the most recent session in the current project."
    example: "claude --continue -p \"continue\" --output-format stream-json --verbose"
  - flag: "--no-session-persistence"
    value: null
    description: "Disables saving session state; incompatible with later resume."
    example: "claude -p --no-session-persistence \"query\""
  - flag: "--setting-sources"
    value: "comma-separated user,project,local"
    description: "Limits filesystem setting scopes loaded by the CLI; managed settings still apply."
    example: "claude -p --setting-sources user,project \"query\""
  - flag: "--safe-mode"
    value: null
    description: "Disables most user/project customizations while keeping auth, model selection, built-in tools, permissions, and managed policy."
    example: "claude -p --safe-mode \"query\""
  - flag: "--debug, --debug-file"
    value: "optional file path"
    description: "Enables debug diagnostics; keep separate from stdout parsing."
    example: "claude -p --debug-file ./claude-debug.txt \"query\""
config_files:
  - os: all
    scope: user
    path: "~/.claude/settings.json"
    format: json
    effect: "User settings, hooks, env, permissions, model defaults, plugins, output style, and related behavior."
    notes: "Lowest setting precedence; project and local override most scalar settings. Permission rules merge rather than simply replace."
  - os: all
    scope: repo
    path: ".claude/settings.json"
    format: json
    effect: "Shared project settings, hooks, permissions, plugins, and repo customizations."
    notes: "Overrides user scalar settings; ignored for some approval-sensitive project MCP trust paths until the workspace is trusted."
  - os: all
    scope: repo
    path: ".claude/settings.local.json"
    format: json
    effect: "Per-user local project overrides, often for permissions or testing hooks."
    notes: "Overrides project and user settings; normally gitignored."
  - os: macos
    scope: managed
    path: "/Library/Application Support/ClaudeCode/managed-settings.json"
    format: json
    effect: "Organization policy: settings, env vars, permissions, hooks, plugin restrictions, model allowlists, telemetry."
    notes: "Highest precedence and cannot be overridden by CLI/user/project for managed keys."
  - os: linux
    scope: managed
    path: "/etc/claude-code/managed-settings.json"
    format: json
    effect: "Organization policy equivalent to macOS managed settings."
    notes: "Highest precedence."
  - os: windows
    scope: managed
    path: "C:\\Program Files\\ClaudeCode\\managed-settings.json"
    format: json
    effect: "Organization policy equivalent to macOS managed settings."
    notes: "Highest precedence."
  - os: all
    scope: user
    path: "~/.claude.json"
    format: json
    effect: "User-level MCP server configuration and per-project Claude Code state."
    notes: "MCP servers may be affected by workspace trust and settings approvals."
  - os: all
    scope: repo
    path: ".mcp.json"
    format: json
    effect: "Project MCP server definitions."
    notes: "Committed approvals cannot approve their own project MCP servers in an untrusted folder; user, managed, --settings, and local approvals can still apply."
  - os: all
    scope: repo
    path: "CLAUDE.md or .claude/CLAUDE.md"
    format: text
    effect: "Project memory/system instructions that can influence model behavior and tool use."
    notes: "Managed claudeMd and managed CLAUDE.md load before user and project memory; --safe-mode disables non-managed memory."
env_vars:
  - name: ANTHROPIC_API_KEY
    effect: "Selects direct API-key auth; in non-interactive mode this key is always used when present."
    notes: "Overrides logged-in subscription auth; unset it to use Claude subscription auth."
  - name: ANTHROPIC_AUTH_TOKEN
    effect: "Custom bearer Authorization value."
    notes: "Auth source may appear as apiKeySource/auth metadata depending on mode."
  - name: ANTHROPIC_MODEL
    effect: "Model selection when --model is not supplied."
    notes: "--model overrides this variable."
  - name: ANTHROPIC_BASE_URL
    effect: "Routes requests through a proxy or gateway and can disable MCP tool search by default on non-first-party hosts."
    notes: "Set ENABLE_TOOL_SEARCH=true only if the gateway forwards tool_reference blocks."
  - name: CLAUDE_CODE_USE_BEDROCK
    effect: "Uses Amazon Bedrock provider path."
    notes: "Changes auth/config requirements and may affect auto-mode availability."
  - name: CLAUDE_CODE_USE_VERTEX
    effect: "Uses Google Vertex AI provider path."
    notes: "Changes auth/config requirements and may affect auto-mode availability."
  - name: CLAUDE_CODE_USE_FOUNDRY
    effect: "Uses Microsoft Foundry provider path."
    notes: "Changes auth/config requirements and may affect auto-mode availability."
  - name: CLAUDE_CODE_USE_ANTHROPIC_AWS
    effect: "Uses Claude Platform on AWS provider path."
    notes: "Requires ANTHROPIC_AWS_WORKSPACE_ID and AWS credentials."
  - name: CLAUDE_CODE_ENABLE_AUTO_MODE
    effect: "Enables auto permission mode on Bedrock, Vertex AI, Foundry, and Claude apps gateway where eligible."
    notes: "Managed settings can disable auto mode."
  - name: CLAUDE_CODE_MAX_TURNS
    effect: "Default max agentic turns when --max-turns is absent."
    notes: "--max-turns takes precedence."
  - name: CLAUDE_CODE_MAX_OUTPUT_TOKENS
    effect: "Sets max output tokens for most requests."
    notes: "Can affect context budget and max-output failure behavior."
  - name: CLAUDE_CODE_MAX_RETRIES
    effect: "Overrides API retry count."
    notes: "Relevant to stream stalls and retry event volume."
  - name: CLAUDE_CODE_RETRY_WATCHDOG
    effect: "Raises retry behavior for unattended sessions."
    notes: "Use when wrappers should wait through longer outages."
  - name: API_TIMEOUT_MS
    effect: "Per-request timeout in milliseconds."
    notes: "Useful for gateways or slow networks."
  - name: API_FORCE_IDLE_TIMEOUT
    effect: "Controls byte-idle timeout on streaming model responses."
    notes: "Set 0 to disable for slow gateways; non-zero can prevent indefinite stalls."
  - name: BASH_DEFAULT_TIMEOUT_MS
    effect: "Default Bash tool timeout."
    notes: "Affects command execution in non-interactive sessions."
  - name: BASH_MAX_TIMEOUT_MS
    effect: "Maximum Bash timeout the model can request."
    notes: "Bound long-running shell tools."
  - name: BASH_MAX_OUTPUT_LENGTH
    effect: "Controls when Bash output is truncated/saved to file for model consumption."
    notes: "Tool results in the stream may contain previews rather than full output."
  - name: CLAUDE_CONFIG_DIR
    effect: "Moves session/config storage root from ~/.claude."
    notes: "Resume lookup uses projects under this directory."
  - name: CLAUDE_CODE_SKIP_PROMPT_HISTORY
    effect: "Disables session persistence/history behavior similarly to --no-session-persistence."
    notes: "Breaks resumability if enabled."
  - name: CLAUDE_AGENT_SDK_DISABLE_BUILTIN_AGENTS
    effect: "Disables built-in subagent types in non-interactive mode."
    notes: "Only applies with -p / Agent SDK."
  - name: CLAUDE_ASYNC_AGENT_STALL_TIMEOUT_MS
    effect: "Stall timeout for background subagents."
    notes: "Default is documented as 600000 ms."
  - name: CLAUDE_CODE_ENABLE_TELEMETRY
    effect: "Enables OpenTelemetry metrics/logs when exporters are configured."
    notes: "Secondary observability stream; useful but not a substitute for stdout stream-json."
  - name: OTEL_LOG_TOOL_DETAILS
    effect: "Includes tool parameters/input in telemetry events."
    notes: "High sensitivity; separate from stream-json."
  - name: OTEL_LOG_TOOL_CONTENT
    effect: "Includes tool input/output content in span events when tracing."
    notes: "High sensitivity; content is truncated in telemetry."
  - name: OTEL_LOG_USER_PROMPTS
    effect: "Includes user prompt content in telemetry logs."
    notes: "Disabled by default."
  - name: OTEL_LOG_ASSISTANT_RESPONSES
    effect: "Includes assistant response text on telemetry assistant_response events."
    notes: "Requires recent Claude Code; defaults to OTEL_LOG_USER_PROMPTS behavior when unset."
  - name: OTEL_LOG_RAW_API_BODIES
    effect: "Emits full raw API request/response bodies to telemetry."
    notes: "Highly sensitive; avoid in Claudine default wrappers."
  - name: DEBUG
    effect: "Can enable debug logging."
    notes: "Keep diagnostics off stdout; prefer explicit --debug-file if needed."
io_contract:
  stdout: structured_only
  stderr: diagnostics_only
  stdin: prompt
  framing: ndjson
  noise_handling: "When --output-format stream-json is supplied, parse stdout as one JSON object per line. Treat stderr as diagnostics/debug/auth text, not lifecycle truth, unless stdout never emits an init or result."
  notes: "With --input-format stream-json, stdin becomes a streaming JSON message input channel rather than plain prompt text. stdout remains the parse stream."
stream_contract:
  discriminator: "type; for type=system use subtype; for type=result use subtype; for type=stream_event use event.type"
  event_ordering: "system/init is expected early, assistant and system/tool progress records follow in generation order, and result is the terminal record for the query. Partial stream_event records precede the completed assistant message they compose."
  correlation_fields:
    - session_id
    - uuid
    - parent_tool_use_id
    - message.content[].id
    - tool_use_id
    - task_id
    - hook_id
    - request_id
  terminal_event: "result"
  partial_message_events: true
  unknown_event_policy: "Skip unknown type/subtype after recording raw event at trace/debug; the SDKMessage union has expanded over time."
  notes: "Tool calls appear inside assistant.message.content blocks with Claude API tool_use IDs; tool results usually arrive as user messages with tool_use_result and/or content tool_result blocks. SDK tool_progress and permission_denied events use tool_use_id for correlation."
session_metadata:
  session_id: "system/init.session_id and result.session_id; TypeScript docs also expose it in most SDKMessage records."
  cwd: "system/init.cwd."
  model: "system/init.model is requested/current model; result.modelUsage keys expose per-model accounting and can reveal actual fallback/final model usage."
  provider: "Not a first-class field in stream-json; infer from auth/env/provider config such as Bedrock/Vertex/Foundry settings."
  auth: "system/init.apiKeySource reports user/project/org/temporary/oauth; ANTHROPIC_API_KEY forces API-key auth in non-interactive mode."
  version: "system/init.claude_code_version."
  mcp_servers: "system/init.mcp_servers[].name/status."
  permission_mode: "system/init.permissionMode and status.permissionMode."
  notes: "system/init also includes tools, slash_commands, output_style, skills, plugins, agents, betas, and fast_mode_state when available."
stream_events:
  - event: "system/init"
    category: session
    fields: ["session_id", "uuid", "cwd", "model", "apiKeySource", "claude_code_version", "permissionMode", "tools", "mcp_servers", "slash_commands", "skills", "plugins", "agents"]
    notes: "Startup metadata; parse this before rendering provider status."
  - event: "assistant"
    category: assistant
    fields: ["session_id", "uuid", "message", "parent_tool_use_id", "request_id", "error", "subagent_type", "task_description", "supersedes"]
    notes: "Complete assistant message; tool_use blocks live in message.content."
  - event: "user"
    category: tool_result
    fields: ["session_id", "uuid", "message", "parent_tool_use_id", "tool_use_result", "origin", "timestamp", "subagent_type", "task_description"]
    notes: "Can represent tool results and replayed user messages; replay requires --replay-user-messages."
  - event: "stream_event"
    category: assistant
    fields: ["session_id", "uuid", "parent_tool_use_id", "event", "ttft_ms"]
    notes: "Raw Claude API event; only emitted with --include-partial-messages."
  - event: "result/success"
    category: usage
    fields: ["session_id", "uuid", "duration_ms", "duration_api_ms", "ttft_ms", "ttft_stream_ms", "num_turns", "result", "stop_reason", "total_cost_usd", "usage", "modelUsage", "permission_denials", "structured_output", "deferred_tool_use", "terminal_reason", "origin"]
    notes: "Normal terminal event; result text is here."
  - event: "result/error_during_execution"
    category: error
    fields: ["session_id", "uuid", "duration_ms", "duration_api_ms", "num_turns", "stop_reason", "total_cost_usd", "usage", "modelUsage", "permission_denials", "errors", "terminal_reason", "origin"]
    notes: "Terminal execution error."
  - event: "result/error_max_turns"
    category: error
    fields: ["session_id", "uuid", "num_turns", "usage", "modelUsage", "errors", "terminal_reason"]
    notes: "Terminal max-turn cap."
  - event: "result/error_max_budget_usd"
    category: error
    fields: ["session_id", "uuid", "total_cost_usd", "usage", "modelUsage", "errors", "terminal_reason"]
    notes: "Terminal cost budget cap."
  - event: "result/error_max_structured_output_retries"
    category: error
    fields: ["session_id", "uuid", "structured_output", "errors", "usage", "modelUsage"]
    notes: "Final JSON-schema/Pydantic/Zod structured output validation did not converge."
  - event: "system/api_retry"
    category: error
    fields: ["session_id", "uuid", "attempt", "max_retries", "retry_delay_ms", "error_status", "error"]
    notes: "Retryable API request failure."
  - event: "rate_limit_event"
    category: usage
    fields: ["session_id", "uuid", "rate_limit_info.status", "rate_limit_info.resetsAt", "rate_limit_info.rateLimitType", "rate_limit_info.utilization", "rate_limit_info.overageStatus", "rate_limit_info.overageResetsAt", "rate_limit_info.errorCode"]
    notes: "Subscription quota/overage state."
  - event: "auth_status"
    category: error
    fields: ["session_id", "uuid", "isAuthenticating", "output", "error"]
    notes: "Authentication flow status."
  - event: "system/status"
    category: session
    fields: ["session_id", "uuid", "status", "permissionMode", "compact_result", "compact_error"]
    notes: "Loop status such as requesting or compacting."
  - event: "system/compact_boundary"
    category: session
    fields: ["session_id", "uuid", "compact_metadata.trigger", "compact_metadata.pre_tokens", "compact_metadata.post_tokens", "compact_metadata.duration_ms"]
    notes: "Context compaction boundary."
  - event: "tool_progress"
    category: tool_call
    fields: ["session_id", "uuid", "tool_use_id", "tool_name", "parent_tool_use_id", "elapsed_time_seconds", "task_id"]
    notes: "Progress event for long-running tools."
  - event: "tool_use_summary"
    category: tool_call
    fields: ["session_id", "uuid", "summary", "preceding_tool_use_ids"]
    notes: "Summarizes preceding tool uses."
  - event: "system/permission_denied"
    category: permission
    fields: ["session_id", "uuid", "tool_name", "tool_use_id", "agent_id", "decision_reason_type", "decision_reason", "message"]
    notes: "Auto-denial path for canUseTool decisions."
  - event: "system/hook_started"
    category: other
    fields: ["session_id", "uuid", "hook_id", "hook_name", "hook_event"]
    notes: "Only emitted with --include-hook-events."
  - event: "system/hook_progress"
    category: other
    fields: ["session_id", "uuid", "hook_id", "hook_name", "hook_event", "stdout", "stderr", "output"]
    notes: "Only emitted with --include-hook-events."
  - event: "system/hook_response"
    category: other
    fields: ["session_id", "uuid", "hook_id", "hook_name", "hook_event", "stdout", "stderr", "output", "exit_code", "outcome"]
    notes: "Only emitted with --include-hook-events."
  - event: "system/task_started"
    category: subagent
    fields: ["session_id", "uuid", "task_id", "tool_use_id", "description", "subagent_type", "task_type", "workflow_name", "prompt", "skip_transcript"]
    notes: "Subagent/background task start."
  - event: "system/task_progress"
    category: subagent
    fields: ["session_id", "uuid", "task_id", "tool_use_id", "description", "subagent_type", "usage", "last_tool_name", "summary"]
    notes: "Subagent/background task progress."
  - event: "system/task_updated"
    category: subagent
    fields: ["session_id", "uuid", "task_id", "patch.status", "patch.description", "patch.end_time", "patch.total_paused_ms", "patch.error", "patch.is_backgrounded"]
    notes: "Patch-style background task state update."
  - event: "system/task_notification"
    category: subagent
    fields: ["session_id", "uuid", "task_id", "tool_use_id", "status", "output_file", "summary", "usage", "skip_transcript"]
    notes: "Subagent/background task completion notification."
  - event: "system/files_persisted"
    category: file_change
    fields: ["session_id", "uuid", "files", "failed", "processed_at"]
    notes: "Files saved to disk; processed_at is an ISO-like timestamp string."
  - event: "system/thinking_tokens"
    category: reasoning
    fields: ["session_id", "uuid", "estimated_tokens", "estimated_tokens_delta"]
    notes: "Approximate live thinking-token estimate, not authoritative billed output tokens."
  - event: "system/model_refusal_fallback"
    category: error
    fields: ["session_id", "uuid", "original_model", "fallback_model", "request_id", "api_refusal_category", "api_refusal_explanation", "retracted_message_uuids", "refused_user_message_uuid"]
    notes: "Model refusal fallback signal."
  - event: "system/model_refusal_no_fallback"
    category: error
    fields: ["session_id", "uuid", "original_model", "request_id", "api_refusal_category", "api_refusal_explanation", "refused_user_message_uuid"]
    notes: "Model refusal without fallback."
  - event: "prompt_suggestion"
    category: assistant
    fields: ["session_id", "uuid", "suggestion"]
    notes: "Only emitted with --prompt-suggestions."
  - event: "system/informational"
    category: other
    fields: ["session_id", "uuid", "content", "level", "tool_use_id", "prevent_continuation"]
    notes: "Generic status/hook/slash-command text."
  - event: "system/local_command_output"
    category: other
    fields: ["session_id", "uuid", "content"]
    notes: "Local slash command output."
tools:
  - name: "Read"
    call_visible: true
    result_visible: true
    metadata: ["tool_use id/name/input in assistant.message.content", "tool_result/user message correlation by tool_use_id"]
    notes: "Read attempts and results are visible as normal tool use/result blocks; permission denials may also emit system/permission_denied."
  - name: "Write"
    call_visible: true
    result_visible: true
    metadata: ["file_path in tool input", "tool_use_id", "permission_denials"]
    notes: "File changes are primarily tool calls/results; files_persisted is a separate persistence event when emitted."
  - name: "Edit/MultiEdit"
    call_visible: true
    result_visible: true
    metadata: ["file_path", "old/new text or edits", "tool_use_id"]
    notes: "AcceptEdits mode can approve edits without asking; protected paths still block except where rules allow."
  - name: "Bash/PowerShell"
    call_visible: true
    result_visible: true
    metadata: ["command", "timeout", "tool_use_id", "BASH_* env-controlled truncation/timeouts"]
    notes: "Command stdout/stderr/exit details are inside the tool result content rather than a dedicated universal command event."
  - name: "Glob/Grep/LS"
    call_visible: true
    result_visible: true
    metadata: ["query/path/pattern input", "tool_use_id"]
    notes: "Read/search tools usually do not require approval in default mode."
  - name: "WebSearch/WebFetch"
    call_visible: true
    result_visible: true
    metadata: ["query/url", "tool_use_id"]
    notes: "Network permission behavior depends on mode, rules, and auto classifier."
  - name: "TodoWrite"
    call_visible: true
    result_visible: true
    metadata: ["todo content in tool input/result", "tool_use_id"]
    notes: "No dedicated plan event; todo updates are tool-visible."
  - name: "Agent/Task"
    call_visible: true
    result_visible: true
    metadata: ["task_id", "tool_use_id", "subagent_type", "task_started/progress/updated/notification"]
    notes: "Subagent messages may include parent_tool_use_id and subagent_type; parent streams do not necessarily include every internal subagent tool call unless emitted by the SDK surface."
  - name: "MCP tools"
    call_visible: true
    result_visible: true
    metadata: ["mcp__server__tool naming", "tool_use_id", "mcp_servers status in init"]
    notes: "MCP tools appear as tools. MCP elicitation and OAuth/user-interaction paths can still block or require programmable handling."
completion:
  success_event: "type=result, subtype=success, is_error=false"
  failure_event: "type=result with subtype error_during_execution, error_max_turns, error_max_budget_usd, or error_max_structured_output_retries; assistant.error and system/api_retry give earlier classification."
  exit_code_reliable: true
  result_fields: ["result.result", "result.structured_output", "result.stop_reason", "result.terminal_reason", "result.deferred_tool_use", "result.errors", "result.permission_denials"]
  cost_fields: ["result.total_cost_usd", "result.modelUsage.*.costUSD"]
  usage_fields: ["result.usage", "result.modelUsage.*.inputTokens", "result.modelUsage.*.outputTokens", "result.modelUsage.*.cacheReadInputTokens", "result.modelUsage.*.cacheCreationInputTokens", "result.modelUsage.*.webSearchRequests", "result.modelUsage.*.contextWindow", "result.modelUsage.*.maxOutputTokens"]
  notes: "Use result subtype for agent outcome and process exit as a transport-level guard. If the process exits without result, classify as wrapper/process failure using stderr and exit code."
blocking_behavior:
  permissions: configurable
  questions: configurable
  tool_approvals: configurable
  notes: "Default permission prompts are unsuitable for unattended wrappers. For deterministic automation use dontAsk with allow/deny rules, acceptEdits for scoped edits, auto when supported, bypassPermissions only in isolated sandboxes, or --permission-prompt-tool for programmable approvals. SDK canUseTool callbacks may wait indefinitely; CLI non-interactive should be configured to deny, approve, or route prompts rather than hang."
subagents:
  supported: true
  start_visible: true
  stop_visible: true
  nested_events_visible: true
  prompt_injection_supported: true
  metadata_fields: ["parent_tool_use_id", "task_id", "tool_use_id", "subagent_type", "task_description", "task_type", "workflow_name", "agent_id", "origin.senderTaskId"]
  notes: "Subagents are invoked through Agent/Task tools and can be defined in .claude/agents or SDK options. Messages from subagent context can carry parent_tool_use_id; task_started/progress/updated/notification expose background-subagent state. Use explicit prompt instructions and filesystem agent definitions to enforce non-interactive behavior."
use_cases:
  - name: plan_cap_approaching
    detectable: true
    event_types: ["rate_limit_event"]
    fields: ["rate_limit_info.status=allowed_warning", "rate_limit_info.utilization", "rate_limit_info.resetsAt", "rate_limit_info.rateLimitType", "rate_limit_info.surpassedThreshold"]
    hook_parity: "unknown"
    notes: "Subscription quota warnings are detectable when rate_limit_event is emitted; API-key credit warnings are less explicit."
  - name: plan_capped
    detectable: true
    event_types: ["rate_limit_event", "system/api_retry", "assistant", "result"]
    fields: ["rate_limit_info.status=rejected", "rate_limit_info.resetsAt", "rate_limit_info.errorCode", "system/api_retry.error=rate_limit", "assistant.error=rate_limit", "result.errors"]
    hook_parity: "unknown"
    notes: "resetsAt/overageResetsAt are numeric epoch-like fields in SDK types but docs do not specify unit; treat as unknown until observed."
  - name: no_funds
    detectable: true
    event_types: ["rate_limit_event", "system/api_retry", "assistant", "result"]
    fields: ["rate_limit_info.errorCode=credits_required", "rate_limit_info.overageDisabledReason=out_of_credits", "system/api_retry.error=billing_error", "assistant.error=billing_error", "result.errors"]
    hook_parity: "unknown"
    notes: "API-key billing failures may appear as billing_error rather than subscription rate_limit_event."
  - name: auth
    detectable: true
    event_types: ["system/init", "auth_status", "system/api_retry", "assistant"]
    fields: ["init.apiKeySource", "auth_status.isAuthenticating", "auth_status.error", "system/api_retry.error=authentication_failed", "assistant.error=authentication_failed|oauth_org_not_allowed"]
    hook_parity: "unknown"
    notes: "ANTHROPIC_API_KEY presence changes auth selection in non-interactive mode."
  - name: permission_read_denied
    detectable: true
    event_types: ["system/permission_denied", "result", "user"]
    fields: ["tool_name", "tool_use_id", "decision_reason_type", "decision_reason", "message", "result.permission_denials[].tool_input"]
    hook_parity: "PermissionDenied hook"
    notes: "Distinguish read/write by tool_name and tool_input path fields."
  - name: permission_write_denied
    detectable: true
    event_types: ["system/permission_denied", "result", "user"]
    fields: ["tool_name", "tool_use_id", "decision_reason_type", "decision_reason", "message", "result.permission_denials[].tool_input"]
    hook_parity: "PermissionDenied hook"
    notes: "Deny rules apply even in bypassPermissions for explicit deny patterns."
  - name: tokens_consumed
    detectable: true
    event_types: ["result", "system/thinking_tokens", "system/task_progress", "system/task_notification"]
    fields: ["result.usage", "result.modelUsage", "thinking_tokens.estimated_tokens", "task_progress.usage.total_tokens"]
    hook_parity: "OpenTelemetry metrics can also report usage when enabled."
    notes: "result usage is authoritative final session/turn accounting; thinking_tokens is approximate live progress."
  - name: model_used
    detectable: true
    event_types: ["system/init", "result", "system/model_refusal_fallback"]
    fields: ["init.model", "result.modelUsage keys", "model_refusal_fallback.original_model", "model_refusal_fallback.fallback_model"]
    hook_parity: "OpenTelemetry may include model fields."
    notes: "init.model may be an alias/current model; result.modelUsage keys are better for actual billed model usage."
  - name: model_fallback
    detectable: true
    event_types: ["system/model_refusal_fallback", "system/model_refusal_no_fallback"]
    fields: ["original_model", "fallback_model", "request_id", "api_refusal_category", "retracted_message_uuids"]
    hook_parity: "unknown"
    notes: "Fallback records are documented in SDK declarations inspected from package 0.3.199."
  - name: human_in_loop
    detectable: true
    event_types: ["system/permission_denied", "result", "system/informational"]
    fields: ["permission_denied.*", "result.deferred_tool_use", "result.terminal_reason=tool_deferred|stop_hook_prevented", "informational.prevent_continuation"]
    hook_parity: "PermissionRequest, Elicitation, ElicitationResult, AskUserQuestion/canUseTool"
    notes: "SDK canUseTool callbacks can stay pending indefinitely; CLI wrappers should avoid unresolved prompts via mode/rules or permission-prompt-tool."
  - name: session_resumable
    detectable: true
    event_types: ["system/init", "result"]
    fields: ["session_id", "cwd", "CLAUDE_CONFIG_DIR", "no-session-persistence absent"]
    hook_parity: "SessionStart/SessionEnd hooks"
    notes: "Resume depends on matching cwd and local session file under ~/.claude/projects or CLAUDE_CONFIG_DIR/projects."
  - name: subagent_prompt_injection
    detectable: true
    event_types: ["system/init", "system/task_started", "assistant"]
    fields: ["init.agents", "task_started.prompt", "task_started.subagent_type", "assistant.subagent_type", "assistant.task_description"]
    hook_parity: "SubagentStart/SubagentStop hooks"
    notes: "Define filesystem or SDK agents with explicit non-interactive instructions; also include non-interactive guidance in the top-level prompt for built-in agents."
headless_constraints:
  - constraint: "Permission prompts can block automation if neither policy nor prompt-tool handling is configured."
    mitigation: "Use --permission-mode dontAsk/acceptEdits/auto/bypassPermissions as appropriate, allow/deny rules, or --permission-prompt-tool."
    notes: "bypassPermissions is appropriate only in isolated containers/VMs."
  - constraint: "MCP servers from .mcp.json can remain pending in untrusted workspaces."
    mitigation: "Pre-trust workspace interactively, approve from user/managed/local settings, or inject trusted MCP config explicitly."
    notes: "Committed project settings cannot approve their own project MCP servers in an untrusted folder."
  - constraint: "Streaming input mode changes stdin from plain prompt text into protocol messages."
    mitigation: "Use --input-format text unless Claudine implements the bidirectional stream-json input protocol."
    notes: "--replay-user-messages helps correlate stdin messages only in stream-json input/output mode."
  - constraint: "Hook events and raw partial deltas are omitted unless explicitly enabled."
    mitigation: "Add --include-hook-events when hook observability matters; add --include-partial-messages only when token-level UI is needed."
    notes: "Partial events require nested parsing of raw Claude API event.type."
  - constraint: "Telemetry can expose useful secondary events but is separate from stdout and can leak sensitive content when configured."
    mitigation: "Do not rely on OTEL for default parser behavior; only enable detail flags with explicit user/admin consent."
    notes: "OTEL logs/exporters may be controlled by managed settings."
  - constraint: "Session resume is cwd and local-storage sensitive."
    mitigation: "Capture session_id early, preserve CLAUDE_CONFIG_DIR/projects/<encoded-cwd>, and resume from the same cwd."
    notes: "--no-session-persistence and CLAUDE_CODE_SKIP_PROMPT_HISTORY break this."
quirks:
  - "The public docs describe CLI output formats, but the stream schema is effectively the Agent SDK TypeScript SDKMessage union, not a JSON Schema."
  - "SDKMessage has grown well beyond the older 21-message list; package 0.3.199 includes model fallback, permission_denied, thinking_tokens, task_updated, notification, mirror_error, informational, and conversation reset families."
  - "type=system is a large nested union; parsers must inspect subtype and tolerate new subtype values."
  - "type=stream_event wraps raw Claude API streaming events; parse event.type separately and accumulate deltas yourself."
  - "Tool calls are not top-level tool_call events in the base stream; they are content blocks inside assistant.message. Separate tool_progress and tool_use_summary events are supplemental."
  - "Tool results may arrive as user-role SDK messages because that matches Claude API conversation structure."
  - "rate_limit_event reset fields are typed as numbers, but official docs do not state seconds vs milliseconds; treat unit as unverified until fixture capture."
  - "Command stdout/stderr are not a universal top-level command event; they are embedded in tool result content or hook_progress/hook_response for hooks."
  - "Managed settings can force env vars and policy behavior that the wrapper cannot override."
  - "ANTHROPIC_API_KEY silently wins in non-interactive mode when present, changing billing/quota behavior."
gaps:
  - "No provider-published JSON Schema, OpenAPI, AsyncAPI, or explicit versioned stream schema was found for CLI stream-json."
  - "No official guarantee found that every CLI stream-json record exactly equals the TypeScript SDKMessage union in every release, though package declarations and docs strongly align."
  - "Exact process exit-code mapping for each result subtype was not documented in the cited CLI reference; treat result as semantic truth and exit code as transport truth."
  - "The unit/timezone of rate_limit_info.resetsAt and overageResetsAt needs observed fixture confirmation."
  - "The exact stderr contract under auth failures, config parse failures before init, and debug mode needs captured examples."
  - "Subagent nested tool-call visibility should be fixture-tested for current Claude Code; docs say subagent messages can carry parent_tool_use_id, but not every internal event is guaranteed."
  - "CLI help output on this host truncated after the first options page, so CLI flags were taken from official docs plus local version verification."
claudine_strategy:
  preferred_invocation: "claude -p --output-format stream-json --verbose --permission-mode dontAsk \"PROMPT\""
  required_flags: ["-p", "--output-format stream-json", "--verbose"]
  conflicting_flags: ["--output-format text", "--output-format json for live wrapping", "--no-session-persistence when resume/recovery is needed", "--input-format stream-json unless Claudine is prepared to speak the stdin protocol"]
  parser_notes: "Parse stdout as NDJSON. Dispatch first on type, then subtype for system/result, then event.type for stream_event. Treat result as the terminal semantic event. Join tools by Claude API tool_use id from assistant.message.content, user tool_result content, tool_progress.tool_use_id, permission_denials[].tool_use_id, and task tool_use_id."
  wrapper_notes: "Set deterministic permission behavior explicitly. Keep stderr separate as diagnostics. Capture system/init.session_id immediately. Consider --include-hook-events for Claudine hook parity; avoid --include-partial-messages unless token-level rendering is needed. Treat unknown events as forward-compatible."
data_format: ndjson
changes:
  - "2026-07-02: Rewrote stale research into schema-backed non-interactive session profile for Claude Code 2.1.199 / Agent SDK 0.3.199."
requires_claudine_update: true
reason: "The current SDKMessage stream has more event families and parser-relevant fields than the old document captured; Claudine provider metadata and stream parsing should be checked against system subtypes such as permission_denied, task_updated, thinking_tokens, model_refusal_fallback, and prompt_suggestion."
---

# Claude Code: Non-Interactive Sessions

## Summary

Claude Code can run non-interactively with structured live output. Claudine should prefer `claude -p --output-format stream-json --verbose` because it turns stdout into newline-delimited JSON records that expose session startup, assistant messages, tool use through Claude API message blocks, permission denials, rate-limit events, hooks when enabled, task/subagent progress, token usage, cost, and a terminal `result` record while the process is still active.

The strongest schema source is not a standalone JSON Schema. It is the official Agent SDK type surface, especially the TypeScript `SDKMessage` union documented in the SDK reference and shipped in `@anthropic-ai/claude-agent-sdk`. The parser risk is therefore version drift: `type` and `subtype` values have expanded over time, so Claudine should parse known events, preserve unknown events for diagnostics, and treat the final `result` event as semantic completion while still using the process exit code for transport failures.

## Non-Interactive Entry Points

Claude Code starts an interactive TUI by default. The non-interactive entry point is print mode:

```bash
claude -p "Summarize this repository"
claude --print "Summarize this repository"
```

The prompt can be supplied as the positional argv prompt, or as stdin text when no prompt argument is supplied. The CLI reference documents `--print`, `--output-format`, `--input-format`, `--max-turns`, `--max-budget-usd`, `--model`, `--permission-mode`, `--permission-prompt-tool`, `--resume`, `--continue`, `--fork-session`, `--add-dir`, and `--mcp-config` as relevant scriptable controls.

For Claudine, the practical launch shape is:

```bash
claude -p --output-format stream-json --verbose --permission-mode dontAsk "PROMPT"
```

`dontAsk` is not universally the right policy, but the wrapper must choose an explicit permission posture. `default` can require human approval. `acceptEdits` is useful for scoped file edits. `auto` can reduce prompts when the account, provider, and model support it. `bypassPermissions` or the older `--dangerously-skip-permissions` path should only be used in isolated sandboxes. `--permission-prompt-tool` is the programmable alternative when Claudine wants an MCP tool to answer approval requests.

Resumability is available through `--resume SESSION_ID`, `--continue`, and `--fork-session`. The SDK docs state that session IDs are present on every result and available earlier on TypeScript `system/init`; resume depends on matching cwd and local transcript storage under `~/.claude/projects/<encoded-cwd>` or `$CLAUDE_CONFIG_DIR/projects/<encoded-cwd>`.

## Output Formats

Claude Code print mode exposes three output formats:

| Format | CLI value | Framing | Streams | Claudine recommendation |
| --- | --- | --- | --- | --- |
| Text | `text` | Plain text | No | Avoid for wrapping; useful only for humans. |
| Final JSON | `json` | One JSON object | No | Acceptable for request/reply scripts, but loses live progress. |
| Streaming JSON | `stream-json` | NDJSON / JSONL | Yes | Prefer. This is the only CLI output mode that gives Claudine live lifecycle and tool visibility. |

`json` is tempting because it is simpler, and `--json-schema` can validate the final payload for application data. That schema validates the agent's final `structured_output`, not the wrapper stream. It does not tell Claudine which tools ran, whether a permission prompt was denied, whether the model retried, or whether the run is stalled before exit.

`stream-json` is better because every stdout line is a separate JSON message. With `--verbose`, it exposes the `system/init` metadata Claudine needs before the run finishes. Optional flags add more streams into the same stdout NDJSON channel:

| Flag | Added records | Use in Claudine |
| --- | --- | --- |
| `--include-hook-events` | `system/hook_started`, `system/hook_progress`, `system/hook_response` | Useful when Claudine needs hook parity or hook stdout/stderr. |
| `--include-partial-messages` | `stream_event` raw Claude API events | Useful for token-level UI; otherwise avoid parser and volume cost. |
| `--prompt-suggestions` | `prompt_suggestion` | Usually not needed for wrapper execution. |
| `--replay-user-messages` | user-message acknowledgments | Use only with `--input-format stream-json`. |

There is a secondary observability surface through OpenTelemetry. It can report logs, metrics, tool details, tool content, and raw API bodies depending on `OTEL_*` variables. Claudine should not rely on telemetry for core lifecycle parsing because it is separately configured, may be disabled, and can be sensitive. It is a useful optional reporting stream, not the primary agent stream.

## Schema Sources

The schema evidence is strong but not JSON-Schema formal. The public CLI reference defines the mode and flags, while the official Agent SDK docs define the event/message types.

The best schema source is the TypeScript SDK:

```typescript
type SDKMessage =
  | SDKAssistantMessage
  | SDKUserMessage
  | SDKUserMessageReplay
  | SDKResultMessage
  | SDKSystemMessage
  | SDKPartialAssistantMessage
  | SDKCompactBoundaryMessage
  | SDKStatusMessage
  | SDKAPIRetryMessage
  | SDKModelRefusalFallbackMessage
  | SDKModelRefusalNoFallbackMessage
  | SDKLocalCommandOutputMessage
  | SDKHookStartedMessage
  | SDKHookProgressMessage
  | SDKHookResponseMessage
  | SDKToolProgressMessage
  | SDKAuthStatusMessage
  | SDKTaskNotificationMessage
  | SDKTaskStartedMessage
  | SDKTaskUpdatedMessage
  | SDKTaskProgressMessage
  | SDKThinkingTokensMessage
  | SDKFilesPersistedEvent
  | SDKToolUseSummaryMessage
  | SDKRateLimitEvent
  | SDKPermissionDeniedMessage
  | SDKPromptSuggestionMessage
  | ...;
```

That abbreviated list comes from local inspection of `@anthropic-ai/claude-agent-sdk@0.3.199` in `package/sdk.d.ts`, which matches the installed Claude Code version on this host, `2.1.199`. The package declaration is a better schema source than examples because it includes discriminators, fields, and comments for newer events such as `system/permission_denied`, `system/thinking_tokens`, `system/task_updated`, and model-refusal fallback.

The Python SDK is also official and confirms the main message families, but it is less exact for parser generation because several messages are represented as broader dataclasses or dictionaries. The streaming-output docs are especially useful for `stream_event`: the outer record has `type: "stream_event"`, while the nested raw Claude API event has its own `event.type` values such as `message_start`, `content_block_start`, `content_block_delta`, `content_block_stop`, `message_delta`, and `message_stop`.

No standalone JSON Schema, OpenAPI, AsyncAPI, or declared stream schema version was found. Claudine should treat SDK types as authoritative enough for implementation but expect additive drift.

## IO Contract

With `--output-format stream-json`, stdout is the machine stream. Claudine should parse stdout line-by-line as NDJSON and should not mix wrapper status text into stdout. Stderr is diagnostics, debug logging, auth/config errors before the stream starts, and human-readable CLI messages. Stderr may be necessary to classify a startup failure if no `system/init` or `result` arrives, but it is not the primary lifecycle stream.

Stdin depends on `--input-format`:

| Input mode | Stdin meaning |
| --- | --- |
| `--input-format text` or omitted | Prompt text. |
| `--input-format stream-json` | Streaming JSON message input protocol. |

Claudine should use text input until it intentionally implements the bidirectional stream protocol. `--input-format stream-json` is powerful, but it changes the wrapper problem from "send a prompt and parse events" to "speak a protocol."

## Stream Contract

The primary discriminator is `type`. For `type: "system"` and `type: "result"`, parse `subtype`. For `type: "stream_event"`, parse the nested `event.type`.

Important ordering:

1. `system/init` is expected early and carries `session_id`, `cwd`, `model`, version, auth source, tools, MCP server status, permission mode, skills, agents, plugins, and slash commands.
2. `assistant` records carry completed assistant messages. Tool calls appear inside `assistant.message.content` as Claude API `tool_use` blocks.
3. `stream_event` records, when enabled, arrive before the completed `assistant` message and are deltas rather than snapshots.
4. Tool results commonly appear as user-role SDK messages because Claude API conversations represent tool results as user content.
5. Supplemental events such as `tool_progress`, `system/permission_denied`, `system/task_progress`, `rate_limit_event`, or `system/api_retry` can arrive between assistant and result records.
6. `result` is the terminal semantic event for the query.

Correlation fields include `session_id`, `uuid`, `request_id`, Claude API tool-use IDs inside message blocks, `tool_use_id`, `parent_tool_use_id`, `task_id`, and `hook_id`. For subagents, `parent_tool_use_id`, `subagent_type`, `task_description`, and task events are the key routing fields.

Unknown event behavior should be forward-compatible: skip for core state, retain raw JSON at trace/debug, and do not crash the run. The current SDK union is broader than older research captured, and future versions are likely to add more system subtypes.

## Session Metadata

`system/init` is the wrapper-grade metadata record. It includes:

| Field | Meaning |
| --- | --- |
| `session_id` | Stable session UUID for logs and resume. |
| `cwd` | Working directory used by the session and resume lookup. |
| `model` | Requested/current model string. |
| `apiKeySource` | Auth source such as `user`, `project`, `org`, `temporary`, or `oauth`. |
| `claude_code_version` | CLI/runtime version. |
| `permissionMode` | Effective starting permission mode. |
| `tools` | Tool names available to the model. |
| `mcp_servers[].name/status` | MCP server startup state. |
| `slash_commands`, `skills`, `agents`, `plugins`, `output_style`, `betas` | Loaded customization surface. |

Model identity needs care. `system/init.model` is the session's selected model string. `result.modelUsage` is better evidence of billed model usage because it is keyed by model name and includes token/cost details. Model fallback can be visible through `system/model_refusal_fallback`, which includes `original_model`, `fallback_model`, `request_id`, and retracted message UUIDs.

Provider identity is not a dedicated stream field. Claudine must infer Bedrock, Vertex, Foundry, Claude Platform on AWS, direct Anthropic, or gateway routing from env/config and auth behavior. `ANTHROPIC_API_KEY` is especially important: the environment-variable docs state that in non-interactive mode it is always used when present, even if the user is logged into a subscription.

## Event Families

The main event families Claudine should handle are:

| Event | Category | Notes |
| --- | --- | --- |
| `system/init` | Session | Startup metadata. |
| `assistant` | Assistant/tool call | Completed assistant message; tool calls are content blocks. |
| `user` | User/tool result | User prompts, replayed messages, and tool results. |
| `stream_event` | Partial assistant | Raw Claude API deltas, opt-in. |
| `result/*` | Completion/usage | Terminal success or failure. |
| `system/api_retry` | Error/retry | Retry attempt, delay, HTTP status, error enum. |
| `rate_limit_event` | Quota | Subscription rate-limit/overage state. |
| `auth_status` | Auth | Authentication progress/error. |
| `system/status` | Runtime | Requesting/compacting status. |
| `system/compact_boundary` | Context | Manual/auto compaction metrics. |
| `tool_progress` | Tool | Long-running tool progress by tool-use ID. |
| `tool_use_summary` | Tool | Summary over preceding tool-use IDs. |
| `system/permission_denied` | Permission | Auto-denied tool call with reason fields. |
| `system/hook_*` | Hooks | Only with `--include-hook-events`. |
| `system/task_*` | Subagent/task | Start, progress, patch update, and notification. |
| `system/files_persisted` | File | File persistence successes/failures. |
| `system/thinking_tokens` | Reasoning | Approximate live thinking-token estimate. |
| `system/model_refusal_*` | Model fallback/error | Refusal fallback or no-fallback state. |
| `prompt_suggestion` | Assistant | Optional next-prompt suggestion. |
| `system/informational` | Status | Generic human-facing notice/warning/suggestion. |

The result subtypes currently relevant to wrappers are `success`, `error_during_execution`, `error_max_turns`, `error_max_budget_usd`, and `error_max_structured_output_retries`.

## Tools

Claude Code's tools are visible through the message stream, but not as a simple `tool_call` / `tool_result` top-level pair. The call starts inside an `assistant.message.content` block with a Claude API `tool_use` ID, name, and input. Results commonly arrive as a `user` message containing a `tool_result` block or `tool_use_result`. Long-running tools can also emit `tool_progress` with `tool_use_id`, `tool_name`, `elapsed_time_seconds`, and optional `task_id`.

Built-in tool families include filesystem reads and writes (`Read`, `Write`, `Edit`, `MultiEdit`), searches (`Glob`, `Grep`, `LS`), shells (`Bash` and PowerShell where enabled), web tools, todo tools, and agent/task tools. MCP tools appear as normal tools, typically with names like `mcp__server__tool`, and MCP server status is visible in `system/init.mcp_servers`.

File changes are not guaranteed to have a single dedicated "file changed" event in the stdout stream. Claudine should infer writes from Write/Edit/MultiEdit/Bash tool calls and results, plus parse `system/files_persisted` when it appears. Command execution stdout, stderr, and exit status are embedded in tool result content rather than a universal command-execution event. Hook command stdout/stderr and exit code are available only in `system/hook_progress` and `system/hook_response` when hook events are included.

Permission denials are better now than old research suggested. The current SDK declaration includes `system/permission_denied` with `tool_name`, `tool_use_id`, optional `agent_id`, `decision_reason_type`, `decision_reason`, and `message`. The terminal result also carries `permission_denials[]` with `tool_name`, `tool_use_id`, and `tool_input`.

## Completion and Exit Status

Claudine should treat `type: "result"` as the semantic terminal event. For success, parse:

- `subtype: "success"`
- `result` for final assistant text
- `structured_output` when `--json-schema` or SDK structured-output options are used
- `duration_ms`, `duration_api_ms`, `num_turns`, `stop_reason`, `terminal_reason`
- `total_cost_usd`, `usage`, and `modelUsage`
- `permission_denials`
- `deferred_tool_use` for deferred approval/tool execution

For failures, parse `subtype`, `errors`, `terminal_reason`, `stop_reason`, `usage`, `modelUsage`, and `permission_denials`. Earlier events refine classification: `system/api_retry.error` can be `authentication_failed`, `billing_error`, `rate_limit`, `server_error`, `invalid_request`, `max_output_tokens`, and related values; `assistant.error` has a broader enum including `oauth_org_not_allowed`, `model_not_found`, `overloaded`, and `unknown`.

Process exit code is still reliable as a transport signal: if the process exits non-zero without a `result`, the wrapper should classify startup/process failure using stderr. When a `result` is present, the stream result is richer than exit code and should drive reports.

## Blocking Behavior

Unattended runs can fail or stall if permissions, questions, MCP OAuth, or elicitation require a human. Claude Code provides several controls, but Claudine must choose deliberately.

Permission modes:

| Mode | Automation behavior |
| --- | --- |
| `default` | Reads are allowed; edits/commands may prompt. Risky for non-interactive wrappers. |
| `acceptEdits` | Reads, edits, and common filesystem operations in allowed roots can proceed. |
| `plan` | Reads/exploration only; no edits. A plan-approval prompt is not useful unattended. |
| `auto` | Classifier approves/denies many actions when supported; still configurable and account/model/provider gated. |
| `dontAsk` | Only pre-approved tools run; otherwise deny. Good for locked-down CI. |
| `bypassPermissions` | Runs almost everything except explicit deny/protected policy; use only in isolated environments. |

The Agent SDK user-input docs state that approvals and `AskUserQuestion` go through `canUseTool` and the callback can remain pending indefinitely. The CLI equivalent for wrappers is to avoid unresolved prompts through permission mode, allow/deny rules, or `--permission-prompt-tool`.

MCP is another blocker. The MCP docs state that project `.mcp.json` servers can remain pending in untrusted folders and committed project settings cannot approve their own servers. User, managed, command-line settings, and local untracked settings can approve them. A wrapper should either pre-provision trusted MCP state or treat pending MCP as a startup condition.

## Subagents

Subagents are supported in non-interactive sessions. They can be built-in, filesystem-defined under `.claude/agents/`, or SDK-defined. Claude invokes them through Agent/Task tools, so tool permissions still matter.

The stream exposes several subagent/task signals:

- `system/task_started` with `task_id`, optional `tool_use_id`, `description`, `subagent_type`, `task_type`, `workflow_name`, and optional `prompt`
- `system/task_progress` with `usage.total_tokens`, `tool_uses`, `duration_ms`, `last_tool_name`, and `summary`
- `system/task_updated` with a patch such as `status`, `end_time`, `error`, or `is_backgrounded`
- `system/task_notification` with `status`, `output_file`, `summary`, and optional usage
- `assistant` and `user` messages may include `parent_tool_use_id`, `subagent_type`, and `task_description`
- `system/permission_denied.agent_id` can identify denials inside subagents

The SDK overview says messages from within a subagent context include `parent_tool_use_id`, letting callers track which subagent execution they belong to. That is enough for Claudine to show nested progress, but not enough to assume every internal subagent tool event is always visible with full fidelity. This needs fixture testing per Claude Code version.

Prompt injection into subagents is supported operationally by defining subagent prompts in `.claude/agents` or SDK options and by telling the parent prompt to pass non-interactive constraints. For built-in agents, Claudine should include non-interactive constraints in the top-level prompt and, where possible, avoid granting tools that force human approval.

## Use Case Detection

| Use case | Detection | Fields |
| --- | --- | --- |
| `plan_cap_approaching` | `rate_limit_event` with `status: "allowed_warning"` | `utilization`, `surpassedThreshold`, `resetsAt`, `rateLimitType` |
| `plan_capped` | `rate_limit_event.status: "rejected"` or retry/assistant `rate_limit` | `resetsAt`, `overageResetsAt`, `errorCode`, `errors` |
| `no_funds` | `billing_error`, `credits_required`, or `out_of_credits` | `system/api_retry.error`, `assistant.error`, `rate_limit_info.*` |
| `auth` | Init auth source or auth errors | `init.apiKeySource`, `auth_status.error`, `authentication_failed`, `oauth_org_not_allowed` |
| `permission_read_denied` | Permission denial event/result | `tool_name`, `tool_input`, `decision_reason_type`, `message` |
| `permission_write_denied` | Same as read, classified by write-shaped tool/path | `tool_name`, `tool_input.path/file_path`, `decision_reason` |
| `tokens_consumed` | Terminal result and task progress | `usage`, `modelUsage`, `task_progress.usage`, `thinking_tokens` |
| `model_used` | Init and result usage | `init.model`, `modelUsage` keys |
| `model_fallback` | Model refusal fallback events | `original_model`, `fallback_model`, `request_id` |
| `human_in_loop` | Permission denied, deferred tool, stop prevented, prompt tool path | `deferred_tool_use`, `terminal_reason`, `permission_denied.*` |
| `session_resumable` | Session ID and persistence enabled | `session_id`, `cwd`, absence of no-persistence settings |
| `subagent_prompt_injection` | Agent definitions and task starts | `init.agents`, `task_started.prompt`, `subagent_type` |

Token and cost units are clear in the SDK declarations: `duration_ms` and `duration_api_ms` are milliseconds, `total_cost_usd` and `modelUsage.*.costUSD` are USD, and token fields are token counts. `thinking_tokens.estimated_tokens` is explicitly approximate and not authoritative billing data. The gap is rate-limit reset time: `resetsAt` is typed as a number, but the docs inspected did not state seconds versus milliseconds or timezone.

## Headless Constraints

The main automation hazards are configuration-driven, not parser-driven.

First, permission handling must be deterministic. A wrapper that launches `default` mode in an unknown repo may block on edits or shell commands. Use explicit `--permission-mode`, permission rules, or `--permission-prompt-tool`.

Second, repo and user configuration can change behavior. Claude Code settings have managed, CLI, local, project, and user precedence for scalar settings; permission rules merge; managed settings can force env vars and policy. `--setting-sources` can limit user/project/local settings, but managed policy still applies.

Third, MCP servers can remain pending until workspace trust or external approval is present. This matters because the model may call a tool that is unavailable or waiting.

Fourth, `--input-format stream-json` is not just "JSON stdin"; it is a streaming input protocol. Claudine should not enable it casually.

Fifth, telemetry is sensitive and separately configured. Do not enable `OTEL_LOG_RAW_API_BODIES`, `OTEL_LOG_TOOL_CONTENT`, or prompt/assistant text logging as a default wrapper behavior.

## Timeline

- Claude Code still supports `-p/--print` for non-interactive execution.
- The docs now frame production programmatic use through the Claude Agent SDK, with the CLI offering `stream-json` as the SDK-compatible command-line stream.
- As of local inspection on 2026-07-02, the installed CLI is `2.1.199 (Claude Code)` and the current npm package is `@anthropic-ai/claude-agent-sdk@0.3.199`.
- Recent docs and types include newer automation-relevant records and settings: `--permission-prompt-tool`, `prompt_suggestion`, `system/permission_denied`, `system/task_updated`, model-refusal fallback events, background subagent stall timeout, and stricter MCP/project trust behavior.

## Quirks and Gaps

The biggest quirk is that the stream is typed by SDK declarations, not a formal JSON Schema. That is good enough to implement a parser, but not enough to freeze behavior. Claudine should expect additive fields and events.

The second quirk is tool visibility. Tool starts are visible before execution if Claudine parses assistant content blocks and, with partial messages, even before the completed assistant message. But there is no universal top-level `tool_call_started` record for all tools. Tool results follow Claude API message structure, often as `user` messages.

The third quirk is that enabling more observability changes stream volume. `--include-partial-messages` can create many raw API delta records. Claudine should leave it off unless it needs token-level progress. `--include-hook-events` is lower volume and useful when hook behavior matters.

Unverified gaps remain:

- exact exit-code mapping for every `result.subtype`
- rate-limit reset numeric units
- stderr shape for startup auth/config failures before `system/init`
- current-version fixture evidence for how much nested subagent tool detail appears in the parent CLI stream
- exact JSON shape of CLI `--input-format stream-json` messages beyond SDK examples

## Claudine Integration Notes

Recommended default:

```bash
claude -p --output-format stream-json --verbose --permission-mode dontAsk "PROMPT"
```

Recommended variants:

- Add `--include-hook-events` when Claudine wants hook lifecycle parity or hook stdout/stderr.
- Add `--include-partial-messages` only for token-level display or early tool-input streaming.
- Add `--max-turns` and/or `--max-budget-usd` when the caller supplies a hard automation budget.
- Use `--permission-prompt-tool` only after Claudine can provide the MCP approval tool deterministically.
- Avoid `--input-format stream-json` until Claudine implements and tests the stdin protocol.
- Avoid `--no-session-persistence` when resumability or recovery matters.

Parser notes:

- stdout is the NDJSON stream; stderr is diagnostics.
- Dispatch by `type`, then `subtype`, then nested `event.type`.
- Treat `result` as terminal semantic completion.
- Join tool calls/results by tool-use IDs in assistant/user message content, `tool_progress.tool_use_id`, `permission_denials[].tool_use_id`, and task `tool_use_id`.
- Capture `system/init.session_id` immediately for logs and recovery.
- Parse unknown events leniently.

Wrapper notes:

- Pass explicit permission mode and roots.
- Keep managed/user/repo config in mind; `--setting-sources` cannot bypass managed policy.
- Preflight MCP trust when MCP servers are required.
- Classify auth source from `system/init.apiKeySource` and relevant env vars.
- Store final usage and cost from `result`, not from partial progress.

## Changelog

- 2026-07-02: Rewrote the existing stale Claude Code non-interactive research into the schema-backed research model. Refreshed against official Claude Code docs, local Claude Code `2.1.199`, and `@anthropic-ai/claude-agent-sdk@0.3.199` TypeScript declarations.

## Sources

- Claude Code CLI reference: <https://code.claude.com/docs/en/cli-reference>
- Claude Code Agent SDK overview: <https://code.claude.com/docs/en/agent-sdk/overview>
- Claude Code TypeScript SDK reference: <https://code.claude.com/docs/en/agent-sdk/typescript>
- Claude Code Python SDK reference: <https://code.claude.com/docs/en/agent-sdk/python>
- Stream responses in real time: <https://code.claude.com/docs/en/agent-sdk/streaming-output>
- Structured outputs: <https://code.claude.com/docs/en/agent-sdk/structured-outputs>
- Work with sessions: <https://code.claude.com/docs/en/agent-sdk/sessions>
- Handle approvals and user input: <https://code.claude.com/docs/en/agent-sdk/user-input>
- Configure permissions: <https://code.claude.com/docs/en/agent-sdk/permissions>
- Permission modes: <https://code.claude.com/docs/en/permission-modes>
- Claude Code settings: <https://code.claude.com/docs/en/settings>
- Environment variables: <https://code.claude.com/docs/en/env-vars>
- MCP configuration and trust behavior: <https://code.claude.com/docs/en/mcp>
- Hooks reference: <https://code.claude.com/docs/en/hooks>
- Monitoring and OpenTelemetry: <https://code.claude.com/docs/en/monitoring-usage>
- Local inspection: `claude --version` reported `2.1.199 (Claude Code)` on 2026-07-02.
- Local package/schema inspection: `npm pack @anthropic-ai/claude-agent-sdk@0.3.199`, especially `package/sdk.d.ts`.
