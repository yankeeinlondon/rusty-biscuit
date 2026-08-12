---
$schema: ./_schema.yaml
created: 2026-04-06
last_updated: 2026-07-03
agent: codex
model: default
docs: https://code.claude.com/docs/en/headless
invocation:
  - command: "claude -p \"PROMPT\" --output-format stream-json --verbose"
    stdin_support: true
    prompt_arg: "Prompt may be the positional argv prompt; when omitted, stdin is prompt text."
    notes: "Starts a fresh non-interactive print-mode session and emits newline-delimited JSON events on stdout."
  - command: "claude -p --input-format stream-json --output-format stream-json --verbose"
    stdin_support: true
    prompt_arg: "stdin JSON message stream"
    notes: "SDK-style streaming input and output; use --replay-user-messages when the caller needs user-message acknowledgments on stdout."
  - command: "claude -p \"PROMPT\" --output-format json"
    stdin_support: true
    prompt_arg: "Prompt may be argv or stdin text."
    notes: "Single final JSON object after the run completes; useful for request/reply scripts but not for live Claudine wrapping."
  - command: "claude -p \"PROMPT\" --output-format text"
    stdin_support: true
    prompt_arg: "Prompt may be argv or stdin text."
    notes: "Human text output; not parser-grade."
  - command: "claude --resume SESSION_ID -p \"PROMPT\" --output-format stream-json --verbose"
    stdin_support: true
    prompt_arg: "Prompt may be argv or stdin text."
    notes: "Resumes a specific persisted session by ID or name; --fork-session creates a branched session."
  - command: "claude --continue -p \"PROMPT\" --output-format stream-json --verbose"
    stdin_support: true
    prompt_arg: "Prompt may be argv or stdin text."
    notes: "Continues the most recent session for the current project or worktree."
output_formats:
  - name: "text"
    cli_value: "text"
    stream: false
    format: text
    description: "Human-readable final assistant text on stdout."
    side_effects: "No reliable lifecycle, tool, usage, or session metadata."
  - name: "json"
    cli_value: "json"
    stream: false
    format: json
    description: "Single final JSON result after the agent finishes."
    side_effects: "Intermediate progress, tool calls, permission denials, hook events, and partial assistant deltas are not available live."
  - name: "stream-json"
    cli_value: "stream-json"
    stream: true
    format: ndjson
    description: "One SDKMessage-compatible JSON object per stdout line."
    side_effects: "Requires print mode; --verbose is the safest wrapper default. Hook events, prompt suggestions, replayed user messages, and raw partial API stream events are opt-in."
schema_sources:
  - url: "https://code.claude.com/docs/en/agent-sdk/typescript"
    schema_type: typescript
    formal: true
    notes: "Official TypeScript Agent SDK reference documents query() as AsyncGenerator<SDKMessage> and defines stream message unions."
  - url: "https://registry.npmjs.org/@anthropic-ai/claude-agent-sdk/-/claude-agent-sdk-0.3.200.tgz"
    schema_type: typescript
    formal: true
    notes: "Local inspection of package/sdk.d.ts from @anthropic-ai/claude-agent-sdk 0.3.200 is the strongest schema evidence for Claude Code 2.1.200."
  - url: "https://code.claude.com/docs/en/headless"
    schema_type: examples
    formal: false
    notes: "Official headless docs describe print mode, JSON modes, stream-json framing, system/api_retry, system/init, and plugin install records."
  - url: "https://code.claude.com/docs/en/agent-sdk/streaming-output"
    schema_type: examples
    formal: false
    notes: "Documents real-time SDK streaming behavior and the limitation that structured_output appears only in the final ResultMessage."
  - url: "https://code.claude.com/docs/en/cli-reference"
    schema_type: examples
    formal: false
    notes: "Defines CLI flags that select print mode, output format, input format, partial messages, hook events, permissions, budgets, model, and resume."
cli_params:
  - flag: "-p, --print"
    value: ""
    description: "Print response without starting the interactive TUI; required for output formats and most automation flags."
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
    value: ""
    description: "Enables fuller structured details and is required by several streaming options."
    example: "claude -p --output-format stream-json --verbose \"query\""
  - flag: "--include-partial-messages"
    value: ""
    description: "Adds stream_event records containing raw Claude API streaming events."
    example: "claude -p --output-format stream-json --verbose --include-partial-messages \"query\""
  - flag: "--include-hook-events"
    value: ""
    description: "Adds hook lifecycle events to stream-json output."
    example: "claude -p --output-format stream-json --verbose --include-hook-events \"query\""
  - flag: "--prompt-suggestions"
    value: ""
    description: "Emits prompt_suggestion messages after each turn."
    example: "claude -p --prompt-suggestions --output-format stream-json --verbose \"query\""
  - flag: "--replay-user-messages"
    value: ""
    description: "Re-emits stdin user messages to stdout in stream-json input/output mode."
    example: "claude -p --input-format stream-json --output-format stream-json --verbose --replay-user-messages"
  - flag: "--json-schema"
    value: "JSON Schema"
    description: "Requests final validated structured_output; this validates answer payloads, not the stream envelope."
    example: "claude -p --json-schema '{\"type\":\"object\"}' \"query\""
  - flag: "--max-turns"
    value: "positive integer"
    description: "Limits agentic turns; max-turn termination is represented in result.subtype and exits with an error."
    example: "claude -p --max-turns 3 \"query\""
  - flag: "--max-budget-usd"
    value: "decimal USD"
    description: "Stops when API cost budget is reached."
    example: "claude -p --max-budget-usd 5.00 \"query\""
  - flag: "--model"
    value: "alias or full model name"
    description: "Selects the model; overrides model setting and ANTHROPIC_MODEL."
    example: "claude -p --model sonnet \"query\""
  - flag: "--permission-mode"
    value: "default | acceptEdits | plan | auto | dontAsk | bypassPermissions"
    description: "Sets initial permission behavior and overrides settings permissions.defaultMode."
    example: "claude -p --permission-mode dontAsk \"query\""
  - flag: "--permission-prompt-tool"
    value: "MCP tool name"
    description: "Routes non-interactive permission prompts to a programmable MCP tool."
    example: "claude -p --permission-prompt-tool mcp__host__approve \"query\""
  - flag: "--allowedTools"
    value: "tool list"
    description: "Pre-approves listed tools or tool patterns."
    example: "claude -p --allowedTools \"Bash,Read,Edit\" \"fix tests\""
  - flag: "--disallowedTools"
    value: "tool list"
    description: "Denies listed tools or patterns."
    example: "claude -p --disallowedTools \"Bash(git push *)\" \"query\""
  - flag: "--dangerously-skip-permissions"
    value: ""
    description: "Legacy bypass-permissions path; only suitable inside an external sandbox."
    example: "claude -p --dangerously-skip-permissions \"query\""
  - flag: "--add-dir"
    value: "directories..."
    description: "Adds extra filesystem roots available to tools."
    example: "claude -p --add-dir ../shared \"query\""
  - flag: "--mcp-config"
    value: "JSON file or JSON string"
    description: "Loads MCP server configuration for the session."
    example: "claude -p --mcp-config ./mcp.json \"query\""
  - flag: "--resume, -r"
    value: "session ID or name"
    description: "Resumes a specific session; with --fork-session branches to a new session ID."
    example: "claude --resume abc123 -p \"continue\" --output-format stream-json --verbose"
  - flag: "--continue, -c"
    value: ""
    description: "Continues the most recent session in the current project."
    example: "claude --continue -p \"continue\" --output-format stream-json --verbose"
  - flag: "--no-session-persistence"
    value: ""
    description: "Disables saving session state; incompatible with later resume."
    example: "claude -p --no-session-persistence \"query\""
  - flag: "--setting-sources"
    value: "comma-separated user,project,local"
    description: "Limits which non-managed filesystem settings scopes are loaded."
    example: "claude -p --setting-sources user,project \"query\""
  - flag: "--settings"
    value: "JSON file or JSON string"
    description: "Supplies per-run settings that override user/project/local settings but not managed policy."
    example: "claude -p --settings '{\"permissions\":{\"defaultMode\":\"dontAsk\"}}' \"query\""
  - flag: "--safe-mode"
    value: ""
    description: "Disables most customizations while keeping auth, model selection, built-in tools, permissions, and managed policy."
    example: "claude -p --safe-mode \"query\""
  - flag: "--debug, --debug-file"
    value: "optional file path"
    description: "Enables diagnostics; keep separate from stdout parsing."
    example: "claude -p --debug-file ./claude-debug.txt \"query\""
config_files:
  - os: macos
    scope: user
    path: "~/.claude/settings.json"
    format: json
    effect: "User settings, hooks, env, permissions, model defaults, plugins, output style, agents, and related behavior."
    notes: "Lowest normal settings precedence; on Windows the same home-relative path resolves under %USERPROFILE%."
  - os: linux
    scope: user
    path: "~/.claude/settings.json"
    format: json
    effect: "User settings, hooks, env, permissions, model defaults, plugins, output style, agents, and related behavior."
    notes: "Lowest normal settings precedence."
  - os: windows
    scope: user
    path: "%USERPROFILE%\\.claude\\settings.json"
    format: json
    effect: "User settings, hooks, env, permissions, model defaults, plugins, output style, agents, and related behavior."
    notes: "Windows expansion of the documented ~/.claude path."
  - os: macos
    scope: repo
    path: ".claude/settings.json"
    format: json
    effect: "Shared project settings, hooks, permissions, agents, plugins, and repo customizations."
    notes: "Overrides user scalar settings; arrays such as permissions merge."
  - os: linux
    scope: repo
    path: ".claude/settings.json"
    format: json
    effect: "Shared project settings, hooks, permissions, agents, plugins, and repo customizations."
    notes: "Overrides user scalar settings; arrays such as permissions merge."
  - os: windows
    scope: repo
    path: ".claude\\settings.json"
    format: json
    effect: "Shared project settings, hooks, permissions, agents, plugins, and repo customizations."
    notes: "Overrides user scalar settings; arrays such as permissions merge."
  - os: macos
    scope: repo
    path: ".claude/settings.local.json"
    format: json
    effect: "Per-user local project overrides, usually for permissions or testing hooks."
    notes: "Overrides project and user settings; normally gitignored."
  - os: linux
    scope: repo
    path: ".claude/settings.local.json"
    format: json
    effect: "Per-user local project overrides, usually for permissions or testing hooks."
    notes: "Overrides project and user settings; normally gitignored."
  - os: windows
    scope: repo
    path: ".claude\\settings.local.json"
    format: json
    effect: "Per-user local project overrides, usually for permissions or testing hooks."
    notes: "Overrides project and user settings; normally gitignored."
  - os: macos
    scope: managed
    path: "/Library/Application Support/ClaudeCode/managed-settings.json"
    format: json
    effect: "Organization policy for settings, env vars, permissions, hooks, plugin restrictions, model allowlists, and telemetry."
    notes: "Managed settings have highest precedence and cannot be overridden by command-line, user, project, or local settings."
  - os: linux
    scope: managed
    path: "/etc/claude-code/managed-settings.json"
    format: json
    effect: "Organization policy equivalent to macOS managed settings."
    notes: "Managed settings have highest precedence."
  - os: windows
    scope: managed
    path: "C:\\Program Files\\ClaudeCode\\managed-settings.json"
    format: json
    effect: "Organization policy equivalent to macOS managed settings."
    notes: "Managed settings have highest precedence; Windows also supports registry-delivered policy."
  - os: macos
    scope: user
    path: "~/.claude.json"
    format: json
    effect: "User-level MCP server configuration and per-project Claude Code state."
    notes: "MCP trust and approval rules can affect whether servers load non-interactively."
  - os: linux
    scope: user
    path: "~/.claude.json"
    format: json
    effect: "User-level MCP server configuration and per-project Claude Code state."
    notes: "MCP trust and approval rules can affect whether servers load non-interactively."
  - os: windows
    scope: user
    path: "%USERPROFILE%\\.claude.json"
    format: json
    effect: "User-level MCP server configuration and per-project Claude Code state."
    notes: "MCP trust and approval rules can affect whether servers load non-interactively."
  - os: macos
    scope: repo
    path: ".mcp.json"
    format: json
    effect: "Project MCP server definitions."
    notes: "Project MCP servers are subject to workspace trust and approval constraints."
  - os: linux
    scope: repo
    path: ".mcp.json"
    format: json
    effect: "Project MCP server definitions."
    notes: "Project MCP servers are subject to workspace trust and approval constraints."
  - os: windows
    scope: repo
    path: ".mcp.json"
    format: json
    effect: "Project MCP server definitions."
    notes: "Project MCP servers are subject to workspace trust and approval constraints."
  - os: macos
    scope: repo
    path: "CLAUDE.md or .claude/CLAUDE.md"
    format: text
    effect: "Project memory/system instructions that can influence model behavior and tool use."
    notes: "--safe-mode disables non-managed memory."
  - os: linux
    scope: repo
    path: "CLAUDE.md or .claude/CLAUDE.md"
    format: text
    effect: "Project memory/system instructions that can influence model behavior and tool use."
    notes: "--safe-mode disables non-managed memory."
  - os: windows
    scope: repo
    path: "CLAUDE.md or .claude\\CLAUDE.md"
    format: text
    effect: "Project memory/system instructions that can influence model behavior and tool use."
    notes: "--safe-mode disables non-managed memory."
env_vars:
  - name: "ANTHROPIC_MODEL"
    effect: "Sets the default model when --model is not supplied."
    notes: "Overridden by --model and /model."
  - name: "ANTHROPIC_API_KEY"
    effect: "Supplies Anthropic API-key auth."
    notes: "The stream can reveal auth source through system/init.apiKeySource when available; never log the value."
  - name: "API_TIMEOUT_MS"
    effect: "Changes API request timeout in milliseconds."
    notes: "Parser-relevant because timeout failures can surface as API error/retry events or process failure."
  - name: "BASH_DEFAULT_TIMEOUT_MS"
    effect: "Default timeout for Bash tool commands."
    notes: "Affects command execution duration and failure behavior."
  - name: "BASH_MAX_TIMEOUT_MS"
    effect: "Maximum timeout the model can request for Bash tool commands."
    notes: "Constrains long-running shell tools."
  - name: "BASH_MAX_OUTPUT_LENGTH"
    effect: "Controls when Bash output is truncated and saved to a file."
    notes: "Affects whether Claudine sees raw command output in tool results or a preview/path."
  - name: "CLAUDE_CODE_SUBPROCESS_ENV_SCRUB"
    effect: "Strips Anthropic/cloud credentials from child process environments."
    notes: "Important for Bash tools, hooks, and MCP stdio servers."
  - name: "CLAUDE_CODE_SYNC_PLUGIN_INSTALL"
    effect: "In print mode, waits for enabled plugin installation before the first query."
    notes: "When set, system/plugin_install events may precede system/init."
  - name: "CLAUDE_CODE_SYNC_PLUGIN_INSTALL_TIMEOUT_MS"
    effect: "Bounds synchronous plugin installation wait time."
    notes: "Without a timeout, synchronous plugin installation can wait until complete."
  - name: "CLAUDE_CODE_SYNC_SKILLS"
    effect: "Downloads enabled claude.ai skills before the first print-mode query and resyncs during the session."
    notes: "Requires claude.ai auth and can change tool/skill availability."
  - name: "CLAUDE_CODE_SYNC_SKILLS_WAIT_TIMEOUT_MS"
    effect: "Bounds initial skill sync wait."
    notes: "After timeout, query proceeds while downloads continue in the background."
  - name: "CLAUDE_CODE_SAFE_MODE"
    effect: "Indicates safe-mode customization suppression."
    notes: "Usually set by --safe-mode."
  - name: "CLAUDE_CODE_SKIP_PROMPT_HISTORY"
    effect: "Disables prompt history/session persistence."
    notes: "Equivalent to --no-session-persistence for this concern."
  - name: "CLAUDE_CODE_USE_BEDROCK"
    effect: "Routes model calls through Bedrock."
    notes: "Changes auth/provider behavior and can remove claude.ai plan rate-limit events."
  - name: "CLAUDE_CODE_USE_VERTEX"
    effect: "Routes model calls through Vertex AI when configured."
    notes: "Provider-specific auth and rate-limit behavior may differ."
io_contract:
  stdout: structured_only
  stderr: diagnostics_only
  stdin: prompt
  framing: ndjson
  noise_handling: "When --output-format stream-json is set, parse stdout line-by-line as JSON and keep stderr out of the parser. Use stderr only for diagnostics or when the process exits before a terminal result."
  notes: "Text and json modes have different contracts. stream-json stdout is the only recommended live parser surface."
stream_contract:
  discriminator: "type"
  event_ordering: "system/init is normally first, but system/plugin_install can precede it when CLAUDE_CODE_SYNC_PLUGIN_INSTALL is set; assistant/user/tool events follow; type=result is terminal for semantic completion."
  correlation_fields: ["session_id", "uuid", "request_id", "tool_use_id", "parent_tool_use_id", "agent_id", "subagent_type"]
  terminal_event: "type=result"
  partial_message_events: true
  unknown_event_policy: "Skip unknown type/subtype after recording the raw event at trace/debug; the SDKMessage union expands over time."
  notes: "Nested system events use subtype. Tool calls are in assistant.message.content tool_use blocks; tool results generally appear as user/tool_result content, system permission_denied, tool_progress, and final result.permission_denials."
session_metadata:
  session_id: "system/init.session_id and result.session_id; most SDKMessage records also carry session_id."
  cwd: "system/init.cwd or working directory metadata when emitted; otherwise wrapper launch cwd."
  model: "system/init.model for requested/current model; result.modelUsage keys show models actually billed; model-refusal fallback events expose original_model and fallback_model."
  provider: "Claude Code / Anthropic unless environment routes through Bedrock, Vertex, Foundry, Mantle, or gateway; stream provider field is not consistently documented."
  auth: "system/init.apiKeySource when available; auth_status and assistant/system errors expose auth failures without secrets."
  version: "Not consistently emitted in stream-json; use `claude --version` outside the run."
  mcp_servers: "system/init.mcp_servers or equivalent init metadata; MCP tool names use mcp__server__tool naming."
  permission_mode: "Selected by --permission-mode or settings permissions.defaultMode; denials appear as system/permission_denied and result.permission_denials."
  notes: "system/init is the early record Claudine should capture for session identity, model, tools, MCP servers, plugins, and permission-related context."
stream_events:
  - event: "system/init"
    category: session
    fields: ["type", "subtype", "session_id", "uuid", "model", "tools", "mcp_servers", "plugins", "plugin_errors", "apiKeySource"]
    notes: "Normally first event; reports startup/session metadata."
  - event: "system/plugin_install"
    category: session
    fields: ["type", "subtype", "status", "name", "error", "uuid", "session_id"]
    notes: "Only when CLAUDE_CODE_SYNC_PLUGIN_INSTALL is set; can precede init."
  - event: "assistant"
    category: assistant
    fields: ["type", "message", "parent_tool_use_id", "error", "uuid", "session_id", "request_id", "supersedes", "subagent_type", "task_description"]
    notes: "Completed assistant message; tool_use blocks live in message.content."
  - event: "user"
    category: tool_result
    fields: ["type", "message", "parent_tool_use_id", "uuid", "session_id"]
    notes: "Can carry tool_result content blocks as the model-visible result of tool execution."
  - event: "user_message_replay"
    category: session
    fields: ["type", "message", "parent_tool_use_id", "uuid", "session_id"]
    notes: "Only when replaying stdin stream messages."
  - event: "stream_event"
    category: assistant
    fields: ["type", "event", "parent_tool_use_id", "uuid", "session_id", "ttft_ms"]
    notes: "Raw Claude API stream event; only with --include-partial-messages."
  - event: "system/api_retry"
    category: error
    fields: ["type", "subtype", "attempt", "max_retries", "retry_delay_ms", "error_status", "error", "uuid", "session_id"]
    notes: "Retryable API failure before retry; error enum includes auth, billing, rate-limit, overload, invalid request, model not found, server, max output tokens, and unknown."
  - event: "system/permission_denied"
    category: permission
    fields: ["type", "subtype", "tool_name", "tool_use_id", "agent_id", "decision_reason_type", "decision_reason", "message", "uuid", "session_id"]
    notes: "Auto-denied tool call; ask-path prompts may instead require permission-prompt tooling or SDK callbacks."
  - event: "tool_progress"
    category: tool_call
    fields: ["type", "tool_use_id", "tool_name", "parent_tool_use_id", "elapsed_time_seconds", "task_id", "uuid", "session_id"]
    notes: "Progress for long-running tools; correlate by tool_use_id."
  - event: "tool_use_summary"
    category: tool_result
    fields: ["type", "summary", "preceding_tool_use_ids", "uuid", "session_id"]
    notes: "Summary attached to prior tool calls."
  - event: "rate_limit_event"
    category: usage
    fields: ["type", "rate_limit_info.status", "rate_limit_info.resetsAt", "rate_limit_info.rateLimitType", "rate_limit_info.utilization", "rate_limit_info.errorCode", "uuid", "session_id"]
    notes: "Subscription-plan rate-limit information; API-key and third-party provider sessions may not emit it."
  - event: "auth_status"
    category: error
    fields: ["type", "isAuthenticating", "output", "error", "uuid", "session_id"]
    notes: "Authentication progress or failure status."
  - event: "system/model_refusal_fallback"
    category: error
    fields: ["type", "subtype", "trigger", "direction", "original_model", "fallback_model", "request_id", "retracted_message_uuids", "uuid", "session_id"]
    notes: "Model fallback after a refusal; parser must evict superseded message UUIDs."
  - event: "system/model_refusal_no_fallback"
    category: error
    fields: ["type", "subtype", "original_model", "request_id", "content", "uuid", "session_id"]
    notes: "Refusal ended the turn without fallback."
  - event: "system/thinking_tokens"
    category: reasoning
    fields: ["type", "subtype", "estimated_tokens", "estimated_tokens_delta", "uuid", "session_id"]
    notes: "Approximate live thinking-token progress; not authoritative billing usage."
  - event: "prompt_suggestion"
    category: assistant
    fields: ["type", "suggestion", "uuid", "session_id"]
    notes: "Only with --prompt-suggestions; arrives after the result message for a turn."
  - event: "hook_started"
    category: other
    fields: ["type", "hook_event_name", "tool_name", "tool_use_id", "uuid", "session_id"]
    notes: "Only with --include-hook-events; exact fields vary by hook."
  - event: "hook_progress"
    category: other
    fields: ["type", "hook_event_name", "stdout", "stderr", "uuid", "session_id"]
    notes: "Only with --include-hook-events."
  - event: "hook_response"
    category: other
    fields: ["type", "hook_event_name", "decision", "reason", "uuid", "session_id"]
    notes: "Only with --include-hook-events."
  - event: "result"
    category: session
    fields: ["type", "subtype", "duration_ms", "duration_api_ms", "is_error", "num_turns", "result", "stop_reason", "total_cost_usd", "usage", "modelUsage", "permission_denials", "errors", "terminal_reason", "structured_output", "uuid", "session_id"]
    notes: "Terminal semantic completion event; subtype identifies success or major error class."
tools:
  - name: "Bash"
    call_visible: true
    result_visible: true
    metadata: ["tool_use_id", "command", "description", "timeout", "stdout", "stderr", "exit code when represented in tool output", "truncation behavior controlled by BASH_MAX_OUTPUT_LENGTH"]
    notes: "Tool call starts as assistant tool_use; results return through tool_result/user content and may be summarized or truncated."
  - name: "Read"
    call_visible: true
    result_visible: true
    metadata: ["tool_use_id", "file_path", "offset", "limit", "file type", "line counts"]
    notes: "Read denials can surface as system/permission_denied or tool_result errors."
  - name: "Edit"
    call_visible: true
    result_visible: true
    metadata: ["tool_use_id", "filePath", "oldString", "newString", "structuredPatch", "gitDiff", "userModified"]
    notes: "SDK tool output includes structured patch and optional git diff, but file changes are not a separate universal file_change event."
  - name: "Write"
    call_visible: true
    result_visible: true
    metadata: ["tool_use_id", "file_path", "content", "permission_denials"]
    notes: "Permission and final status must be derived from tool_result, system/permission_denied, hooks, and final result.permission_denials."
  - name: "Grep/Glob/LS"
    call_visible: true
    result_visible: true
    metadata: ["tool_use_id", "path", "pattern", "matches"]
    notes: "Read/search family is normally read-only and may not require approval unless denied by rules."
  - name: "WebFetch/WebSearch"
    call_visible: true
    result_visible: true
    metadata: ["tool_use_id", "url", "domain", "query", "result preview"]
    notes: "Network access depends on provider settings and permission policy."
  - name: "MCP tools"
    call_visible: true
    result_visible: true
    metadata: ["tool_use_id", "tool name mcp__server__tool", "server", "input", "result", "permission mode"]
    notes: "MCP approval/elicitation can block automation unless preconfigured or routed through a prompt tool/SDK host callback."
  - name: "Agent/subagent tool"
    call_visible: true
    result_visible: true
    metadata: ["tool_use_id", "agent_id", "agent_type", "subagent_type", "task_description", "SubagentStart/SubagentStop hook fields when included"]
    notes: "Parent stream can show subagent-produced assistant messages and hook events, but subagent-private transcript detail is not guaranteed without extra surfaces."
completion:
  success_event: "type=result with subtype=success and is_error=false"
  failure_event: "type=result with subtype=error_during_execution, error_max_turns, error_max_budget_usd, or error_max_structured_output_retries; earlier assistant.error/system events refine classification."
  exit_code_reliable: true
  result_fields: ["result", "structured_output", "errors", "terminal_reason", "stop_reason", "permission_denials", "deferred_tool_use", "session_id", "uuid"]
  cost_fields: ["total_cost_usd", "duration_api_ms", "duration_ms"]
  usage_fields: ["usage.input_tokens", "usage.output_tokens", "usage.cache_creation_input_tokens", "usage.cache_read_input_tokens", "modelUsage"]
  notes: "Use result as semantic completion. Use process exit for transport/CLI failure and as a consistency check; missing result before exit is ambiguous or a wrapper failure."
blocking_behavior:
  permissions: configurable
  questions: configurable
  tool_approvals: configurable
  notes: "Use --permission-mode dontAsk, explicit allow/deny rules, or --permission-prompt-tool for deterministic print-mode runs. The SDK can pause indefinitely in canUseTool for approvals and AskUserQuestion unless the host responds or cancels."
subagents:
  supported: true
  start_visible: true
  stop_visible: true
  nested_events_visible: false
  prompt_injection_supported: true
  metadata_fields: ["agent_id", "agent_type", "subagent_type", "task_description", "agent_transcript_path", "parent_tool_use_id", "tool_use_id"]
  notes: "Subagents can run in non-interactive sessions. Start/stop is visible through hook events when --include-hook-events is enabled; some subagent-produced assistant messages carry subagent_type/task_description, but full nested tool streams are not guaranteed in the parent stream."
use_cases:
  - name: plan_cap_approaching
    detectable: true
    event_types: ["rate_limit_event"]
    fields: ["rate_limit_info.status=allowed_warning", "rate_limit_info.utilization", "rate_limit_info.resetsAt", "rate_limit_info.rateLimitType", "rate_limit_info.surpassedThreshold"]
    hook_parity: "No exact hook parity verified."
    notes: "Plan rate-limit fields are mainly for claude.ai subscription sessions; resetsAt unit is inferred from SDK as number and should be treated as provider-defined until fixture verified."
  - name: plan_capped
    detectable: true
    event_types: ["rate_limit_event", "system/api_retry", "assistant", "result"]
    fields: ["rate_limit_info.status=rejected", "rate_limit_info.resetsAt", "rate_limit_info.errorCode", "system/api_retry.error=rate_limit", "assistant.error=rate_limit", "result.errors"]
    hook_parity: "No exact hook parity verified."
    notes: "Differentiate subscription caps from API rate-limit retries."
  - name: no_funds
    detectable: true
    event_types: ["rate_limit_event", "system/api_retry", "assistant", "result"]
    fields: ["rate_limit_info.errorCode=credits_required", "rate_limit_info.overageDisabledReason=out_of_credits", "system/api_retry.error=billing_error", "assistant.error=billing_error", "result.errors"]
    hook_parity: "No exact hook parity verified."
    notes: "Billing and credit failures may appear as retry/system errors or terminal result errors."
  - name: auth
    detectable: true
    event_types: ["system/init", "auth_status", "system/api_retry", "assistant", "result"]
    fields: ["apiKeySource", "auth_status.isAuthenticating", "auth_status.error", "system/api_retry.error=authentication_failed", "assistant.error=authentication_failed", "assistant.error=oauth_org_not_allowed", "result.errors"]
    hook_parity: "No exact hook parity verified."
    notes: "Auth source is non-secret metadata; error strings must still be redacted defensively."
  - name: permission_read_denied
    detectable: true
    event_types: ["system/permission_denied", "result", "user"]
    fields: ["tool_name=Read|Grep|Glob|LS", "tool_use_id", "decision_reason_type", "decision_reason", "message", "result.permission_denials[].tool_input"]
    hook_parity: "PreToolUse/PostToolUse hooks can observe or enforce related policy, but stream-json is the wrapper surface."
    notes: "Classify read denial by tool_name and tool_input path fields."
  - name: permission_write_denied
    detectable: true
    event_types: ["system/permission_denied", "result", "user"]
    fields: ["tool_name=Write|Edit|NotebookEdit|Bash", "tool_use_id", "decision_reason_type", "decision_reason", "message", "result.permission_denials[].tool_input"]
    hook_parity: "PreToolUse/PostToolUse hooks can observe or enforce related policy, but stream-json is the wrapper surface."
    notes: "Bash can be write-shaped only by command inspection; do not classify all Bash denials as writes."
  - name: tokens_consumed
    detectable: true
    event_types: ["result", "system/thinking_tokens"]
    fields: ["result.usage.*", "result.modelUsage", "thinking_tokens.estimated_tokens", "thinking_tokens.estimated_tokens_delta"]
    hook_parity: "No exact hook parity verified."
    notes: "Final result usage is authoritative; thinking_tokens is live approximate progress."
  - name: model_used
    detectable: true
    event_types: ["system/init", "result", "system/model_refusal_fallback"]
    fields: ["init.model", "result.modelUsage", "original_model", "fallback_model"]
    hook_parity: "No exact hook parity verified."
    notes: "init.model can be alias/current selection; modelUsage keys are the billed model identities."
  - name: model_fallback
    detectable: true
    event_types: ["system/model_refusal_fallback"]
    fields: ["original_model", "fallback_model", "direction", "request_id", "retracted_message_uuids"]
    hook_parity: "No exact hook parity verified."
    notes: "Parser should remove superseded messages listed by the fallback event."
  - name: human_in_loop
    detectable: true
    event_types: ["system/permission_denied", "result", "hook_response"]
    fields: ["permission_denied.*", "result.deferred_tool_use", "result.terminal_reason", "result.permission_denials", "hook_response.decision"]
    hook_parity: "PermissionRequest hooks can signal approval waits when hooks are configured."
    notes: "SDK mode can pause in canUseTool or AskUserQuestion; CLI print mode should be made deterministic with permission flags."
  - name: session_resumable
    detectable: true
    event_types: ["system/init", "result"]
    fields: ["session_id"]
    hook_parity: "Hook input also carries session_id."
    notes: "Do not use --no-session-persistence if Claudine needs resume/recovery."
  - name: subagent_prompt_injection
    detectable: true
    event_types: ["assistant", "hook_started", "hook_response"]
    fields: ["subagent_type", "task_description", "agent_id", "agent_type"]
    hook_parity: "SubagentStart/SubagentStop hook input carries agent_id and agent_type."
    notes: "Inject non-interactive constraints in the top-level prompt and agent definitions; direct per-subagent prompt override from CLI is not a stable stream feature."
headless_constraints:
  - constraint: "Permission prompts can block or fail automation if policy is not deterministic."
    mitigation: "Use --permission-mode dontAsk, explicit --allowedTools/--disallowedTools, settings permissions, or --permission-prompt-tool."
    notes: "SDK can wait indefinitely for canUseTool callbacks; CLI print mode should avoid human prompts."
  - constraint: "stream-json schema is not published as JSON Schema."
    mitigation: "Generate parser fixtures from official SDK TypeScript declarations and tolerate unknown events."
    notes: "SDKMessage grows across releases."
  - constraint: "Hook events are not present unless requested."
    mitigation: "Add --include-hook-events when hook observability matters."
    notes: "This increases stream volume but is lower volume than raw partial token events."
  - constraint: "Raw partial deltas are opt-in and high volume."
    mitigation: "Avoid --include-partial-messages unless token-level UI or early tool-input streaming is required."
    notes: "Completed assistant/result messages remain available without this flag."
  - constraint: "Plugin and skill sync can delay startup."
    mitigation: "Set explicit sync timeout variables or disable sync for deterministic CI."
    notes: "Plugin install can produce events before init."
  - constraint: "Managed settings can override CLI/user/project configuration."
    mitigation: "Record effective settings where possible and treat managed policy as authoritative."
    notes: "Managed settings cannot be overridden by command-line arguments."
quirks:
  - "The recommended stream is NDJSON on stdout, but the schema authority is the Agent SDK TypeScript declaration, not a standalone stream JSON Schema."
  - "system/init is usually first, but synchronous plugin installation can emit system/plugin_install before init."
  - "prompt_suggestion can arrive after result, so a parser should not treat every post-result record as impossible when that flag is enabled."
  - "system/thinking_tokens is progress telemetry, not billable token usage."
  - "Permission denials can appear both as system/permission_denied and final result.permission_denials; join by tool_use_id."
  - "File changes are not a single provider-wide file_change event family; infer them from Edit/Write tool outputs, hooks, and diffs."
  - "Model identity can be split across requested/init model, result.modelUsage keys, and model-refusal fallback events."
gaps:
  - "No official JSON Schema, OpenAPI document, or explicit stream schema version was found for CLI stream-json."
  - "No documented guarantee was found that every CLI stream-json event is exactly the same union as the TypeScript SDKMessage in every release, though docs and package declarations strongly align."
  - "Exact timestamp units are absent for most stream events; rate_limit_info.resetsAt is numeric but the unit needs fixture verification."
  - "Exit-code mapping for every result subtype, cancellation path, and signal interruption needs captured fixtures."
  - "Full parent-stream visibility for nested subagent internal tool calls is not guaranteed by the public docs."
  - "MCP OAuth/elicitation behavior in plain CLI print mode needs fixture coverage; SDK exposes callbacks for elicitation and token refresh."
claudine_strategy:
  preferred_invocation: "claude -p --output-format stream-json --verbose --permission-mode dontAsk \"PROMPT\""
  required_flags: ["-p/--print", "--output-format stream-json", "--verbose", "explicit --permission-mode chosen by Claudine policy"]
  conflicting_flags: ["--output-format text", "--output-format json for live wrapping", "--no-session-persistence when resume/recovery is required", "--include-partial-messages unless token-level UI is needed"]
  parser_notes: "Parse stdout as NDJSON SDKMessage records using type as the top-level discriminator and subtype for system/result variants. Treat result as terminal semantic completion, tolerate unknown events, correlate tools by tool_use_id, and keep stderr separate."
  wrapper_notes: "Capture system/init.session_id early, select permission behavior explicitly, use --include-hook-events when hook parity matters, set plugin/skill sync timeouts if enabling sync, and preserve raw records for drift analysis."
data_format: ndjson
changes:
  - "2026-07-03: Refreshed against official Claude Code docs, local Claude Code 2.1.200, and @anthropic-ai/claude-agent-sdk 0.3.200; split config files into per-OS records and expanded stream/parser metadata."
requires_claudine_update: true
reason: "Current Claude Code stream-json exposes newer event families and fields such as system/permission_denied, rate_limit_event, auth_status, prompt_suggestion, system/thinking_tokens, and model-refusal fallback that Claudine metadata and parsers should verify against."
---

# Claude Code Non-Interactive Sessions

## Summary

Claude Code can run non-interactively with structured live output. Claudine should prefer `claude -p --output-format stream-json --verbose` because it makes stdout a newline-delimited JSON stream that exposes startup metadata, assistant messages, tool calls/results, permission denials, API retries, rate-limit signals, usage/cost, and a terminal `result` record while the process is still active.

The main risk is schema drift. The public docs describe the mode and examples, but the practical schema authority is the Agent SDK `SDKMessage` TypeScript union shipped in `@anthropic-ai/claude-agent-sdk`, not a standalone JSON Schema. Claudine should parse known `type`/`subtype` values, preserve unknown records for diagnostics, and use the final `result` event as semantic completion while still treating process exit as important transport evidence.

## Non-Interactive Entry Points

The documented non-interactive entry point is print mode:

```sh
claude -p "PROMPT"
```

For Claudine, the scriptable form should be:

```sh
claude -p --output-format stream-json --verbose --permission-mode dontAsk "PROMPT"
```

The prompt can be passed as an argv prompt or via stdin when the positional prompt is omitted. `--input-format stream-json` changes stdin into an SDK-style stream of JSON messages; with `--replay-user-messages`, Claude Code acknowledges those user messages back on stdout. Session continuity is available through `--resume SESSION_ID`, `--continue`, and normal session persistence; `--no-session-persistence` should be avoided when Claudine needs recovery or resume.

Attachments and richer message shapes are better represented through streaming input or the Agent SDK. The CLI also supports roots (`--add-dir`), model selection (`--model`), MCP configuration (`--mcp-config`), agents (`--agent`, `--agents`), output validation (`--json-schema`), and tool policy (`--permission-mode`, `--allowedTools`, `--disallowedTools`, `--permission-prompt-tool`).

## Output Formats

| Format | CLI value | Framing | Streams | Claudine use |
| --- | --- | --- | --- | --- |
| Text | `text` | Plain text | No | Avoid for wrappers; useful only for humans. |
| Final JSON | `json` | One JSON object | No | Useful for simple request/reply scripts; weak for lifecycle monitoring. |
| Streaming JSON | `stream-json` | NDJSON | Yes | Preferred parser surface for Claudine. |

`stream-json` is the right default because Claudine is supervising a live autonomous process, not just collecting a final answer. It can render progress before exit, classify retry/auth/permission/rate-limit states, capture session IDs early, and report usage/cost from the terminal record. `json` can be prettier for shell scripts, but it hides the operational timeline until the run is over.

Two optional streams matter:

| Flag | Added records | Recommendation |
| --- | --- | --- |
| `--include-hook-events` | Hook lifecycle records such as hook start/progress/response | Enable when Claudine needs hook parity or hook diagnostics. |
| `--include-partial-messages` | Raw Claude API `stream_event` deltas | Leave off by default; enable for token-level UI or early low-level tool input visibility. |

## Schema Sources

The official [headless documentation](https://code.claude.com/docs/en/headless) defines print mode, `--output-format json`, `--output-format stream-json`, and the fact that each streaming line is a JSON event. It also documents `system/api_retry`, `system/init`, and `system/plugin_install` fields. The [CLI reference](https://code.claude.com/docs/en/cli-reference) is the source for the flags that affect the stream: `--output-format`, `--input-format`, `--verbose`, `--include-partial-messages`, `--include-hook-events`, `--prompt-suggestions`, `--replay-user-messages`, `--permission-mode`, and budget/session flags.

The strongest schema source is the official [TypeScript Agent SDK reference](https://code.claude.com/docs/en/agent-sdk/typescript) plus the package declarations in `@anthropic-ai/claude-agent-sdk`. Local inspection on 2026-07-03 used Claude Code `2.1.200` and `@anthropic-ai/claude-agent-sdk@0.3.200`; `package/sdk.d.ts` defines event shapes such as `SDKAssistantMessage`, `SDKAPIRetryMessage`, `SDKPermissionDeniedMessage`, `SDKRateLimitEvent`, `SDKResultMessage`, `SDKToolProgressMessage`, and `SDKThinkingTokensMessage`.

This is formal TypeScript API surface, but it is not a formal JSON Schema for CLI stdout. Claudine should treat it as authoritative enough to generate parser expectations, while keeping forward compatibility for new event families.

## IO Contract

With `--output-format stream-json`, stdout is parse-only NDJSON: one JSON object per line. Stderr should be kept separate for diagnostics, startup failures, debug output, or cases where the process exits before a terminal stream record. Stdin is prompt text in normal print mode and a JSON message stream when `--input-format stream-json` is selected.

Do not parse text mode. Do not assume stderr is irrelevant: it can be the only evidence for CLI startup, auth, policy-helper, or configuration failures that happen before the stream begins.

## Stream Contract

The top-level discriminator is `type`. Many operational records also use a nested `subtype`, especially `type: "system"` and `type: "result"`.

Typical ordering:

1. `system/plugin_install` may appear first when `CLAUDE_CODE_SYNC_PLUGIN_INSTALL=1`.
2. `system/init` normally follows and carries session metadata.
3. `assistant`, `user`, `tool_progress`, `system/api_retry`, `system/permission_denied`, `rate_limit_event`, hook events, and partial `stream_event` records appear as work proceeds.
4. `result` marks semantic completion for the turn/run.
5. `prompt_suggestion` can arrive after `result` when enabled, so parsers should allow known post-result advisory records.

Tool calls are correlated by Claude API `tool_use` IDs. A call is visible inside `assistant.message.content`; results commonly return as `user` messages containing `tool_result` content. Additional progress and denial records use `tool_use_id`. `parent_tool_use_id`, `agent_id`, `subagent_type`, and `task_description` connect nested or subagent-originated work.

## Session Metadata

`system/init` is the record Claudine should capture immediately. The docs state it reports session metadata including model, tools, MCP servers, loaded plugins, and plugin errors. Most SDK messages also carry `session_id` and `uuid`; the terminal `result` repeats `session_id`.

Model identity has multiple layers. `system/init.model` is the selected/current model, which may be an alias. `result.modelUsage` is better for billed/resolved model identity. `system/model_refusal_fallback` exposes `original_model` and `fallback_model` when a refusal retry swaps models.

Claude Code version is not a stable stream field in the public contract; run `claude --version` outside the agent process if Claudine needs it. Local verification found `2.1.200 (Claude Code)`.

## Event Families

Important stream families for Claudine:

| Event | Category | Notes |
| --- | --- | --- |
| `system/init` | Session | Startup metadata; normally first. |
| `system/plugin_install` | Session | Plugin install progress before init when synchronous plugin install is enabled. |
| `assistant` | Assistant/tool call | Completed assistant message; may include tool_use blocks, `error`, `request_id`, `supersedes`, and subagent metadata. |
| `user` | User/tool result | User messages and tool_result blocks. |
| `stream_event` | Partial | Raw API delta events only with `--include-partial-messages`. |
| `system/api_retry` | Error/retry | Retry attempt, delay, HTTP status, and typed error category. |
| `system/permission_denied` | Permission | Auto-denied tool call with tool name, tool ID, reason, and optional subagent ID. |
| `tool_progress` | Tool progress | Tool name, tool ID, elapsed seconds, optional task ID. |
| `rate_limit_event` | Usage/quota | Subscription plan status, reset, utilization, overage, and credit signals. |
| `auth_status` | Auth | Auth progress/failure status without secrets. |
| `system/model_refusal_fallback` | Model fallback | Fallback model retry and retracted message UUIDs. |
| `system/thinking_tokens` | Reasoning progress | Approximate live thinking-token count, not billing usage. |
| `hook_*` | Hooks | Only with `--include-hook-events`. |
| `result` | Completion | Terminal result with success/error subtype, usage, cost, errors, permission denials, and answer text. |

## Tools

Claude Code tool calls are visible, but not as a single normalized `tool_call_start`/`tool_call_result` pair. The model emits tool calls inside assistant content blocks; tool results return through user/tool_result content and supplemental records. `tool_progress` gives live progress for long-running tools. `system/permission_denied` and `result.permission_denials[]` give explicit denial records keyed by `tool_use_id`.

File changes are inferred from tool outputs. The SDK declaration for `Edit` includes structured patch data and optional git diff metadata. There is no universal provider-level `file_change` event that Claudine can rely on across every edit/write path; hooks can add more visibility when enabled.

MCP tools use the `mcp__server__tool` naming pattern and follow the same broad stream mechanics. MCP OAuth or elicitation can still be a headless constraint unless preconfigured or hosted through SDK callbacks.

## Completion and Exit Status

Normal completion is `type: "result", subtype: "success", is_error: false`. The result includes final answer text in `result`, final validated `structured_output` when `--json-schema` is used, `duration_ms`, `duration_api_ms`, `num_turns`, `stop_reason`, `total_cost_usd`, `usage`, `modelUsage`, `permission_denials`, and `session_id`.

Known error result subtypes include `error_during_execution`, `error_max_turns`, `error_max_budget_usd`, and `error_max_structured_output_retries`. Earlier events such as `assistant.error`, `system/api_retry.error`, `auth_status.error`, `rate_limit_event.rate_limit_info`, and `system/permission_denied` provide better classification than the final subtype alone.

Claudine should treat `result` as semantic completion and process exit as transport completion. If the process exits without a `result`, classify it as ambiguous or a wrapper/CLI startup failure and inspect stderr.

## Blocking Behavior

Claude Code has several ways to avoid human prompts, but Claudine must choose one deliberately. For deterministic print-mode runs, use an explicit permission mode and tool policy. `dontAsk` auto-denies tools that would require approval; `bypassPermissions` or `--dangerously-skip-permissions` auto-approves broadly and should only be used inside an external sandbox. `--permission-prompt-tool` lets a programmable MCP tool answer approvals in non-interactive mode.

The Agent SDK has a richer but more dangerous control surface: approval requests and `AskUserQuestion` both flow through the host callback. The docs state that callback can remain pending indefinitely. That is useful for a real app with a user present, but it is a hang risk for Claudine unless the host implements a timeout and deterministic response.

Managed policy can override local expectations. The settings docs state managed settings have highest precedence and cannot be overridden by command-line arguments. That means Claudine should surface policy-driven denials clearly instead of assuming its CLI flags won.

## Subagents

Subagents can run during non-interactive sessions. Parent-stream visibility is partial:

| Surface | Visibility |
| --- | --- |
| Assistant messages | May include `subagent_type` and `task_description`. |
| Permission denials | May include `agent_id`. |
| Hook events | `SubagentStart` and `SubagentStop` are visible when hook events are included. |
| Full nested transcript/tool stream | Not guaranteed in the parent stream by public docs. |

Claudine can steer subagents through the top-level prompt, agent definitions, and any supported `appendSubagentSystemPrompt` SDK option, but the plain CLI stream is not a stable per-subagent prompt-injection protocol.

## Use Case Detection

| Use case | Detection |
| --- | --- |
| `plan_cap_approaching` | `rate_limit_event.rate_limit_info.status == "allowed_warning"` plus utilization/reset fields when present. |
| `plan_capped` | `rate_limit_event.status == "rejected"`, `system/api_retry.error == "rate_limit"`, assistant error, or terminal result errors. |
| `no_funds` | `billing_error`, `credits_required`, or `overageDisabledReason == "out_of_credits"`. |
| `auth` | `auth_status.error`, `system/api_retry.error == "authentication_failed"`, `assistant.error == "authentication_failed"` or `oauth_org_not_allowed`, and init auth-source metadata. |
| `permission_read_denied` | `system/permission_denied` or `result.permission_denials[]` for read/search tools. |
| `permission_write_denied` | Same denial records for write/edit/notebook tools; inspect Bash command shape before treating Bash as write. |
| `tokens_consumed` | Final `result.usage` and `result.modelUsage`; live `system/thinking_tokens` is approximate only. |
| `model_used` | `system/init.model` plus final `result.modelUsage`. |
| `model_fallback` | `system/model_refusal_fallback.original_model` and `.fallback_model`. |
| `human_in_loop` | Permission denials, deferred tool use, prompt-tool decisions, or SDK callback wait states. |
| `session_resumable` | `session_id` in init/result and no `--no-session-persistence`. |
| `subagent_prompt_injection` | Prompt/agent configuration rather than a dedicated stream event; stream can expose subagent metadata for audit. |

## Headless Constraints

Permissions are the biggest automation hazard. Without a deterministic mode, a run can need approval for Bash, edits, MCP tools, OAuth, or user questions. Claudine should set a policy every time rather than inheriting ambient user settings.

Configuration is another hazard. User, project, local, command-line, and managed settings combine. Scalar settings override by precedence, while array settings such as permissions merge and deduplicate. Managed settings win over everything and can disable bypass behavior or require managed-only permission rules.

Plugin and skill sync can delay startup. If Claudine opts into `CLAUDE_CODE_SYNC_PLUGIN_INSTALL` or `CLAUDE_CODE_SYNC_SKILLS`, it should set timeouts and parse install/sync records where available.

## Timeline

- 2026-07-03: Verified current docs and local CLI/package versions: Claude Code `2.1.200`, `@anthropic-ai/claude-agent-sdk@0.3.200`.
- 2026-07-03: Official docs show `system/api_retry`, `system/init`, and `system/plugin_install` as first-class headless stream records.
- 2026-07-03: SDK declaration includes newer parser-relevant records: `system/permission_denied`, `rate_limit_event`, `auth_status`, `prompt_suggestion`, `system/thinking_tokens`, `tool_progress`, and model-refusal fallback/no-fallback records.

## Quirks and Gaps

The largest gap is the lack of a standalone JSON Schema or explicit stream schema version. The TypeScript SDK declaration is strong evidence, but not a guarantee that every CLI stdout record will match it forever.

The second gap is fixture coverage. Exact exit codes for every terminal subtype, signal interruption, cancellation, MCP OAuth, and elicitation should be captured in local tests before Claudine treats those paths as fully normalized.

The third quirk is post-result advisory output. `prompt_suggestion` is documented as arriving after the result for a turn. A parser should mark `result` as terminal for the main run while tolerating known advisory records until process exit.

## Claudine Integration Notes

Use this as the default wrapper shape:

```sh
claude -p --output-format stream-json --verbose --permission-mode dontAsk "PROMPT"
```

Parse stdout only as NDJSON. Keep stderr as diagnostics. Capture `system/init.session_id` immediately, then correlate subsequent records by `session_id`, `uuid`, `tool_use_id`, `parent_tool_use_id`, and subagent fields where present. Treat unknown events as forward-compatible, not fatal.

Add `--include-hook-events` when Claudine needs hook lifecycle parity. Avoid `--include-partial-messages` by default because it increases stream volume and exposes lower-level API deltas; completed assistant and result records are enough for most wrapper workflows. Avoid `--output-format json` for live execution because it withholds the timeline until completion.

## Changelog

- 2026-07-03: Refreshed the document against current official docs, local Claude Code `2.1.200`, and Agent SDK `0.3.200`; updated frontmatter for per-OS config records and expanded stream contract/use-case detection.

## Sources

- [Run Claude Code programmatically](https://code.claude.com/docs/en/headless)
- [Claude Code CLI reference](https://code.claude.com/docs/en/cli-reference)
- [Agent SDK reference - TypeScript](https://code.claude.com/docs/en/agent-sdk/typescript)
- [Stream responses in real time](https://code.claude.com/docs/en/agent-sdk/streaming-output)
- [Streaming input](https://code.claude.com/docs/en/agent-sdk/streaming-vs-single-mode)
- [Handle approvals and user input](https://code.claude.com/docs/en/agent-sdk/user-input)
- [Claude Code settings](https://code.claude.com/docs/en/settings)
- [Configure permissions](https://code.claude.com/docs/en/permissions)
- [Configure server-managed settings](https://code.claude.com/docs/en/server-managed-settings)
- [Environment variables](https://code.claude.com/docs/en/env-vars)
- `@anthropic-ai/claude-agent-sdk@0.3.200`, local package inspection of `package/sdk.d.ts`
