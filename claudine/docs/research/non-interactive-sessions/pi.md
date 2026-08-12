---
$schema: ./_schema.yaml
created: 2026-07-02
last_updated: 2026-07-03
agent: codex
model: default
docs: https://pi.dev/docs/latest/json
invocation:
  - command: 'pi --mode json "PROMPT"'
    stdin_support: true
    prompt_arg: "Prompt text from argv; file and image references use @path arguments; piped stdin is merged into the initial prompt in print/json mode."
    notes: "Fresh non-interactive session unless --continue, --session, --session-id, or --fork is supplied. Emits live JSONL AgentSessionEvent records to stdout."
  - command: 'pi -p "PROMPT"'
    stdin_support: true
    prompt_arg: "Prompt text from argv; --print also consumes the following argv token as a prompt."
    notes: "Text print mode. It waits for completion and prints only final assistant text, so Claudine should not prefer it for live status."
  - command: 'pi --mode text -p "PROMPT"'
    stdin_support: true
    prompt_arg: "Prompt text from argv and piped stdin."
    notes: "Equivalent scriptable text-output shape; useful for final text only, not structured lifecycle parsing."
  - command: "pi --mode rpc --no-session"
    stdin_support: true
    prompt_arg: "JSONL commands on stdin, for example {\"type\":\"prompt\",\"message\":\"...\"}."
    notes: "Starts a long-running headless RPC server over stdin/stdout. Responses and events are JSONL; extension dialog UI requests may require JSONL responses."
output_formats:
  - name: "text print"
    cli_value: "text or -p"
    stream: false
    format: text
    description: "Final assistant text only on stdout after completion; errors are printed to stderr and return non-zero."
    side_effects: "No live tool, message, token, or session metadata stream."
  - name: "json event stream"
    cli_value: "json"
    stream: true
    format: jsonl
    description: "One JSON object per line on stdout. First line is the session header, followed by live AgentSessionEvent records."
    side_effects: "Runs print-mode internals with extension mode json and no UI. Claudine should prefer this for one-shot automation."
  - name: "rpc protocol"
    cli_value: "rpc"
    stream: true
    format: jsonl
    description: "Bidirectional JSONL protocol. Commands and extension UI responses go to stdin; command responses, events, and extension UI requests come from stdout."
    side_effects: "More controllable than json mode, but wrappers must drive a protocol and answer extension_ui_request records to avoid blocking."
schema_sources:
  - url: "https://pi.dev/docs/latest/json"
    schema_type: typescript
    formal: false
    notes: "Official docs describe JSON mode and link the AgentSessionEvent and AgentEvent TypeScript unions; no JSON Schema is published."
  - url: "https://github.com/earendil-works/pi/blob/main/packages/coding-agent/src/core/agent-session.ts"
    schema_type: typescript
    formal: false
    notes: "Authoritative source for AgentSessionEvent, including agent_end.willRetry, compaction, queue, entry, and session-info events."
  - url: "https://github.com/earendil-works/pi/blob/main/packages/agent/src/types.ts"
    schema_type: typescript
    formal: false
    notes: "Authoritative source for base AgentEvent and tool lifecycle fields."
  - url: "https://pi.dev/docs/latest/rpc"
    schema_type: typescript
    formal: false
    notes: "Official RPC protocol documentation; useful context but broader than the one-shot JSON stream."
  - url: "https://github.com/earendil-works/pi/blob/main/packages/coding-agent/src/modes/rpc/rpc-types.ts"
    schema_type: typescript
    formal: false
    notes: "Authoritative TypeScript union for RPC commands, responses, and extension UI request/response messages."
cli_params:
  - flag: "--mode"
    value: "text|json|rpc"
    description: "Selects text print output, JSON event stream, or bidirectional RPC protocol."
    example: "pi --mode json \"fix lint\""
  - flag: "--print, -p"
    value: ""
    description: "Runs non-interactively and exits after processing the prompt."
    example: "pi -p \"summarize\""
  - flag: "--model"
    value: "pattern or provider/model[:thinking]"
    description: "Selects model by pattern or explicit provider/model; supports thinking-level shorthand."
    example: "pi --mode json --model anthropic/claude-sonnet-4-5:high \"work\""
  - flag: "--provider"
    value: "name"
    description: "Narrows provider selection when --model is not provider-qualified."
    example: "pi --mode json --provider openai --model gpt-4o \"work\""
  - flag: "--api-key"
    value: "key"
    description: "Runtime API key for the selected provider; takes precedence over stored auth and environment variables."
    example: "pi --mode json --model openai/gpt-4o --api-key \"$OPENAI_API_KEY\" \"work\""
  - flag: "--thinking"
    value: "off|minimal|low|medium|high|xhigh"
    description: "Sets thinking level for supported models."
    example: "pi --mode json --thinking high \"solve\""
  - flag: "--tools, -t"
    value: "comma-separated tool names"
    description: "Allowlist built-in, extension, and custom tools."
    example: "pi --mode json --tools read,grep,find,ls \"review\""
  - flag: "--exclude-tools, -xt"
    value: "comma-separated tool names"
    description: "Denylist selected tools while keeping the rest available."
    example: "pi --mode json --exclude-tools bash,write \"review\""
  - flag: "--no-tools, -nt"
    value: ""
    description: "Disables all tools by default."
    example: "pi --mode json --no-tools \"answer only\""
  - flag: "--no-builtin-tools, -nbt"
    value: ""
    description: "Disables built-in tools while preserving extension/custom tools."
    example: "pi --mode json --no-builtin-tools -e ./tool.ts \"work\""
  - flag: "--extension, -e"
    value: "path"
    description: "Loads an explicit extension file; may alter tools, prompts, events, and UI request behavior."
    example: "pi --mode json -e ./my-extension.ts \"work\""
  - flag: "--no-extensions, -ne"
    value: ""
    description: "Disables extension discovery; explicit -e paths still load."
    example: "pi --mode json --no-extensions \"work\""
  - flag: "--skill"
    value: "path"
    description: "Loads a skill file or directory."
    example: "pi --mode json --skill ./skills/reviewer \"review\""
  - flag: "--no-skills, -ns"
    value: ""
    description: "Disables skill discovery and loading."
    example: "pi --mode json --no-skills \"work\""
  - flag: "--prompt-template"
    value: "path"
    description: "Loads prompt templates that can be invoked in prompts."
    example: "pi --mode json --prompt-template ./prompts \"run /fix\""
  - flag: "--no-prompt-templates, -np"
    value: ""
    description: "Disables prompt template discovery and loading."
    example: "pi --mode json --no-prompt-templates \"work\""
  - flag: "--no-context-files, -nc"
    value: ""
    description: "Disables AGENTS.md and CLAUDE.md discovery."
    example: "pi --mode json --no-context-files \"work\""
  - flag: "--approve, -a"
    value: ""
    description: "Trusts project-local files for this run, enabling project settings/resources in non-interactive mode."
    example: "pi --mode json --approve \"work\""
  - flag: "--no-approve, -na"
    value: ""
    description: "Ignores project-local files for this run."
    example: "pi --mode json --no-approve \"work\""
  - flag: "--continue, -c"
    value: ""
    description: "Continues the previous session."
    example: "pi --mode json --continue \"next step\""
  - flag: "--session"
    value: "path or partial id"
    description: "Uses a specific session file or partial UUID."
    example: "pi --mode json --session abc123 \"resume\""
  - flag: "--session-id"
    value: "id"
    description: "Uses or creates an exact project session id."
    example: "pi --mode json --session-id ci-run-42 \"work\""
  - flag: "--fork"
    value: "path or partial id"
    description: "Forks a specific session into a new session."
    example: "pi --mode json --fork abc123 \"try alternate\""
  - flag: "--session-dir"
    value: "dir"
    description: "Overrides session storage and lookup directory."
    example: "pi --mode json --session-dir .pi/sessions \"work\""
  - flag: "--no-session"
    value: ""
    description: "Disables session persistence."
    example: "pi --mode rpc --no-session"
  - flag: "--name, -n"
    value: "name"
    description: "Sets the session display name at startup."
    example: "pi --mode json --name ci-review \"review\""
  - flag: "--offline"
    value: ""
    description: "Disables startup network operations such as version checks, package checks, and install telemetry."
    example: "pi --mode json --offline \"work\""
  - flag: "@path"
    value: "file path"
    description: "Adds file/image references to the initial prompt."
    example: "pi --mode json @screenshot.png \"describe\""
config_files:
  - os: macos
    scope: user
    path: "~/.pi/agent/settings.json"
    format: json
    effect: "Global settings for model defaults, thinking, retry, compaction, sessionDir, proxy, trust fallback, resources, and startup behavior."
    notes: "Project settings override global settings with recursive object merge; arrays and scalars replace."
  - os: linux
    scope: user
    path: "~/.pi/agent/settings.json"
    format: json
    effect: "Same as macOS global settings."
    notes: "Pi uses os.homedir() plus .pi/agent by default, not XDG_CONFIG_HOME."
  - os: windows
    scope: user
    path: "%USERPROFILE%\\.pi\\agent\\settings.json"
    format: json
    effect: "Same as macOS global settings."
    notes: "Path inferred from os.homedir() and path.join; verify exact home expansion in packaged Windows binary if needed."
  - os: macos
    scope: repo
    path: ".pi/settings.json"
    format: json
    effect: "Project-local overrides for settings and resources."
    notes: "Loaded only when project trust allows it. Project overrides global; nested objects merge, arrays/scalars replace."
  - os: linux
    scope: repo
    path: ".pi/settings.json"
    format: json
    effect: "Same as macOS project settings."
    notes: "Loaded only when project trust allows it."
  - os: windows
    scope: repo
    path: ".pi\\settings.json"
    format: json
    effect: "Same as macOS project settings."
    notes: "Loaded only when project trust allows it."
  - os: macos
    scope: user
    path: "~/.pi/agent/models.json"
    format: json
    effect: "Custom provider/model catalog; affects model IDs, providers, costs, context windows, base URLs, and auth surfaces."
    notes: "Model/provider metadata can appear in assistant messages and RPC state."
  - os: linux
    scope: user
    path: "~/.pi/agent/models.json"
    format: json
    effect: "Same as macOS user model catalog."
    notes: "Model/provider metadata can appear in assistant messages and RPC state."
  - os: windows
    scope: user
    path: "%USERPROFILE%\\.pi\\agent\\models.json"
    format: json
    effect: "Same as macOS user model catalog."
    notes: "Model/provider metadata can appear in assistant messages and RPC state."
  - os: macos
    scope: user
    path: "~/.pi/agent/auth.json"
    format: json
    effect: "Stored provider API keys and OAuth tokens."
    notes: "Auth source is not emitted in JSON mode; wrappers should not inspect this file unless explicitly authorized."
  - os: linux
    scope: user
    path: "~/.pi/agent/auth.json"
    format: json
    effect: "Same as macOS stored auth."
    notes: "Auth source is not emitted in JSON mode."
  - os: windows
    scope: user
    path: "%USERPROFILE%\\.pi\\agent\\auth.json"
    format: json
    effect: "Same as macOS stored auth."
    notes: "Auth source is not emitted in JSON mode."
  - os: macos
    scope: user
    path: "~/.pi/agent/trust.json"
    format: json
    effect: "Saved project trust decisions by directory."
    notes: "Non-interactive modes use this before falling back to defaultProjectTrust."
  - os: linux
    scope: user
    path: "~/.pi/agent/trust.json"
    format: json
    effect: "Same as macOS trust decisions."
    notes: "Non-interactive modes use this before falling back to defaultProjectTrust."
  - os: windows
    scope: user
    path: "%USERPROFILE%\\.pi\\agent\\trust.json"
    format: json
    effect: "Same as macOS trust decisions."
    notes: "Non-interactive modes use this before falling back to defaultProjectTrust."
  - os: macos
    scope: user
    path: "~/.pi/agent/sessions/--{sanitized_cwd}--/{timestamp}_{uuid}.jsonl"
    format: other
    effect: "Persisted session transcript with header and tree entries."
    notes: "Useful for resume and audit. This is not the live stdout stream."
  - os: linux
    scope: user
    path: "~/.pi/agent/sessions/--{sanitized_cwd}--/{timestamp}_{uuid}.jsonl"
    format: other
    effect: "Same as macOS session files."
    notes: "May move when --session-dir, PI_CODING_AGENT_SESSION_DIR, or settings.sessionDir is set."
  - os: windows
    scope: user
    path: "%USERPROFILE%\\.pi\\agent\\sessions\\--{sanitized_cwd}--\\{timestamp}_{uuid}.jsonl"
    format: other
    effect: "Same as macOS session files."
    notes: "May move when --session-dir, PI_CODING_AGENT_SESSION_DIR, or settings.sessionDir is set."
env_vars:
  - name: "PI_CODING_AGENT_DIR"
    effect: "Overrides the user config directory, including settings, auth, models, trust, and resources."
    notes: "Name is derived from APP_NAME; official package uses PI_CODING_AGENT_DIR."
  - name: "PI_CODING_AGENT_SESSION_DIR"
    effect: "Overrides session storage unless --session-dir is supplied."
    notes: "Precedence is --session-dir, PI_CODING_AGENT_SESSION_DIR, then settings.sessionDir."
  - name: "PI_OFFLINE"
    effect: "Disables startup network operations when set to 1/true/yes."
    notes: "Equivalent to --offline."
  - name: "PI_SKIP_VERSION_CHECK"
    effect: "Skips the Pi version update check."
    notes: "Narrower than PI_OFFLINE."
  - name: "PI_TELEMETRY"
    effect: "Overrides install/update telemetry and provider attribution headers."
    notes: "Does not disable update checks."
  - name: "PI_PACKAGE_DIR"
    effect: "Overrides package directory for package/resource lookup."
    notes: "Relevant when package installs or resource resolution would otherwise touch network/package managers."
  - name: "PI_CACHE_RETENTION"
    effect: "Sets prompt-cache retention preference for providers that support it."
    notes: "Can change provider request behavior and cost/usage characteristics."
  - name: "PI_OAUTH_CALLBACK_HOST"
    effect: "Overrides OAuth callback host for supported OAuth flows."
    notes: "Can matter for headless auth setup, but non-interactive runs should start with auth already configured."
  - name: "ANTHROPIC_API_KEY"
    effect: "Provider auth for Anthropic models."
    notes: "Provider env vars are lower precedence than CLI --api-key and auth.json."
  - name: "ANTHROPIC_OAUTH_TOKEN"
    effect: "OAuth token auth for Anthropic; preferred over ANTHROPIC_API_KEY by Pi's env-key map."
    notes: "Avoid leaking in wrapper logs."
  - name: "OPENAI_API_KEY"
    effect: "Provider auth for OpenAI models."
    notes: "Representative provider auth variable; many provider-specific API key variables exist."
  - name: "GEMINI_API_KEY"
    effect: "Provider auth for Google Gemini models."
    notes: "Representative provider auth variable."
  - name: "HTTP_PROXY"
    effect: "Proxy for provider/network requests."
    notes: "Can also be set through settings.httpProxy."
  - name: "HTTPS_PROXY"
    effect: "Proxy for provider/network requests."
    notes: "Can also be set through settings.httpProxy."
io_contract:
  stdout: structured_only
  stderr: diagnostics_only
  stdin: prompt
  framing: jsonl
  noise_handling: "For --mode json, parse stdout as JSONL only. Treat stderr as human diagnostics and startup/auth errors. In RPC mode, stdout is mixed protocol records and stdin is bidirectional JSONL."
  notes: "JSON mode writes the session header before subscription, then writes JSON.stringify(event) plus LF for each event. Text mode stdout is final text, not parseable events."
stream_contract:
  discriminator: "type"
  event_ordering: "session header first when present; agent_start before turn/message/tool events; message_update deltas precede message_end; turn_end precedes agent_end; agent_end is the final event for a run, though subscriber settlement may continue briefly."
  correlation_fields: ["toolCallId", "message.content[].id", "entry.id", "entry.parentId", "rpc.id"]
  terminal_event: "agent_end"
  partial_message_events: true
  unknown_event_policy: "Skip unknown type values, preserve raw record for logs, and continue parsing; TypeScript unions are informal and can drift."
  notes: "Nested assistant deltas use assistantMessageEvent.type. tool_execution_update.partialResult is accumulated progress, not necessarily a delta."
session_metadata:
  session_id: "session.id in the first JSON-mode header; RPC get_state.data.sessionId; session files also include header.id."
  cwd: "session.cwd in the first JSON-mode header; not repeated on every event."
  model: "message.model/provider/api/usage on assistant messages; RPC get_state.data.model returns a full Model object."
  provider: "assistant message provider/api/model fields; RPC get_state.data.model.provider."
  auth: "unknown; auth source is not emitted in JSON mode. Errors may mention missing auth."
  version: "CLI --version exists; JSON/RPC event stream does not include package version."
  mcp_servers: "not applicable; Pi docs inspected here do not expose MCP server metadata in JSON mode."
  permission_mode: "Project trust is represented by behavior/config, not a stream field. Tool allowlists/denylists are not emitted as a startup record."
  notes: "Header is available early enough for session correlation. Model identity usually arrives on assistant message completion or via RPC get_state, not as an init event."
stream_events:
  - event: "session"
    category: session
    fields: ["type", "version", "id", "timestamp", "cwd", "parentSession"]
    notes: "First JSON-mode line when a session header exists."
  - event: "agent_start"
    category: session
    fields: ["type"]
    notes: "Start of agent processing for a prompt."
  - event: "turn_start"
    category: session
    fields: ["type"]
    notes: "Start of one assistant-response turn."
  - event: "message_start"
    category: assistant
    fields: ["type", "message"]
    notes: "Emitted for user, assistant, custom, and toolResult messages."
  - event: "message_update"
    category: assistant
    fields: ["type", "message", "assistantMessageEvent", "assistantMessageEvent.type", "assistantMessageEvent.delta", "assistantMessageEvent.contentIndex", "assistantMessageEvent.partial"]
    notes: "Streaming assistant deltas; nested types include start, text_start, text_delta, text_end, thinking_start, thinking_delta, thinking_end, toolcall_start, toolcall_delta, toolcall_end, done, and error."
  - event: "message_end"
    category: assistant
    fields: ["type", "message", "message.role", "message.content", "message.stopReason", "message.errorMessage", "message.usage", "message.provider", "message.model", "message.timestamp"]
    notes: "Completed message snapshot. Assistant messages carry usage/cost when available."
  - event: "tool_execution_start"
    category: tool_call
    fields: ["type", "toolCallId", "toolName", "args"]
    notes: "Visible before execution after arguments are prepared/validated."
  - event: "tool_execution_update"
    category: tool_result
    fields: ["type", "toolCallId", "toolName", "args", "partialResult"]
    notes: "Progress event; partialResult is accumulated output for tools such as bash."
  - event: "tool_execution_end"
    category: tool_result
    fields: ["type", "toolCallId", "toolName", "result", "isError"]
    notes: "Tool terminal record; join with start/update by toolCallId."
  - event: "turn_end"
    category: session
    fields: ["type", "message", "toolResults"]
    notes: "Completed turn snapshot with final assistant message and tool result messages."
  - event: "agent_end"
    category: session
    fields: ["type", "messages", "willRetry"]
    notes: "Terminal event for a run. AgentSession adds willRetry to the base AgentEvent."
  - event: "queue_update"
    category: other
    fields: ["type", "steering", "followUp"]
    notes: "Full pending steering/follow-up queues whenever they change."
  - event: "compaction_start"
    category: usage
    fields: ["type", "reason"]
    notes: "reason is manual, threshold, or overflow."
  - event: "compaction_end"
    category: usage
    fields: ["type", "reason", "result", "result.tokensBefore", "result.estimatedTokensAfter", "aborted", "willRetry", "errorMessage"]
    notes: "Useful for context pressure and overflow recovery, but not a precise provider-token usage event."
  - event: "auto_retry_start"
    category: error
    fields: ["type", "attempt", "maxAttempts", "delayMs", "errorMessage"]
    notes: "Transient provider retry, including rate-limit/overload messages when surfaced."
  - event: "auto_retry_end"
    category: error
    fields: ["type", "success", "attempt", "finalError"]
    notes: "Final failure has success false and finalError."
  - event: "entry_appended"
    category: session
    fields: ["type", "entry", "entry.id", "entry.parentId", "entry.timestamp"]
    notes: "Source type includes this event, but official JSON docs have not yet documented it."
  - event: "session_info_changed"
    category: session
    fields: ["type", "name"]
    notes: "Session display-name change."
  - event: "thinking_level_changed"
    category: reasoning
    fields: ["type", "level"]
    notes: "Visible when thinking level changes during the session."
  - event: "response"
    category: other
    fields: ["type", "id", "command", "success", "data", "error"]
    notes: "RPC-only command response."
  - event: "extension_ui_request"
    category: other
    fields: ["type", "id", "method", "title", "message", "options", "timeout"]
    notes: "RPC-only UI subprotocol; dialog methods require extension_ui_response on stdin."
  - event: "extension_error"
    category: error
    fields: ["type", "extensionPath", "event", "error"]
    notes: "RPC docs list it as an event. JSON-mode source has extension errors printed to stderr for print/json bind errors."
tools:
  - name: "read"
    call_visible: true
    result_visible: true
    metadata: ["toolCallId", "toolName", "args", "result.content", "result.details", "isError"]
    notes: "Built-in read tool. Permission denial is not a dedicated event; parse isError and result text/details."
  - name: "bash"
    call_visible: true
    result_visible: true
    metadata: ["args.command", "args.timeout", "partialResult", "result.content", "result.details.truncation", "result.details.fullOutputPath", "isError"]
    notes: "stdout and stderr from the child process are combined into the tool output stream. Command exit details are visible in result text/details rather than a normalized top-level exitCode for LLM tool calls."
  - name: "edit"
    call_visible: true
    result_visible: true
    metadata: ["args.path", "args.edits", "result.details.diff", "result.details.patch", "result.details.firstChangedLine", "isError"]
    notes: "File change is inferable from successful edit tool results; there is no separate file_change event."
  - name: "write"
    call_visible: true
    result_visible: true
    metadata: ["args.path", "args.content", "result.content", "isError"]
    notes: "File creation/overwrite is inferable from successful write tool results; there is no separate file_change event."
  - name: "grep"
    call_visible: true
    result_visible: true
    metadata: ["toolCallId", "toolName", "args", "result.content", "result.details", "isError"]
    notes: "Read-only search tool."
  - name: "find"
    call_visible: true
    result_visible: true
    metadata: ["toolCallId", "toolName", "args", "result.content", "result.details", "isError"]
    notes: "Read-only file discovery tool."
  - name: "ls"
    call_visible: true
    result_visible: true
    metadata: ["toolCallId", "toolName", "args", "result.content", "result.details", "isError"]
    notes: "Read-only directory listing tool."
  - name: "extension tools"
    call_visible: true
    result_visible: true
    metadata: ["toolCallId", "toolName", "args", "result", "isError"]
    notes: "Extensions can register tools; stream shape is the same, but result.details is extension-defined."
completion:
  success_event: "agent_end with final assistant message.stopReason not error or aborted"
  failure_event: "message_end/turn_end/agent_end containing final assistant message.stopReason error or aborted; auto_retry_end success false; process exits non-zero for print-mode catch/final text error path"
  exit_code_reliable: true
  result_fields: ["message.content[].text", "message.stopReason", "message.errorMessage", "agent_end.messages"]
  cost_fields: ["message.usage.cost.input", "message.usage.cost.output", "message.usage.cost.cacheRead", "message.usage.cost.cacheWrite", "message.usage.cost.total", "RPC get_session_stats.data.cost"]
  usage_fields: ["message.usage.input", "message.usage.output", "message.usage.cacheRead", "message.usage.cacheWrite", "message.usage.totalTokens", "RPC get_session_stats.data.tokens"]
  notes: "The terminal stream event is agent_end. Process exit catches startup/runtime exceptions, but Claudine should still parse final assistant stopReason and auto-retry events for classification."
blocking_behavior:
  permissions: configurable
  questions: configurable
  tool_approvals: configurable
  notes: "Pi does not implement a built-in per-tool permission prompt like some CLIs. Project trust prompts are suppressed in non-interactive modes: without saved trust, defaultProjectTrust ask/never ignores project resources and always trusts them; --approve/--no-approve override per run. Extensions can still request UI in RPC mode; JSON/print modes expose no UI and extension code must check ctx.hasUI."
subagents:
  supported: false
  start_visible: false
  stop_visible: false
  nested_events_visible: false
  prompt_injection_supported: false
  metadata_fields: []
  notes: "Pi's public site says it skips sub-agents and plan mode by default. No native subagent event family was found in JSON/RPC docs or the AgentSessionEvent union."
use_cases:
  - name: plan_cap_approaching
    detectable: true
    event_types: ["compaction_start", "compaction_end", "RPC get_session_stats response"]
    fields: ["compaction_end.result.tokensBefore", "compaction_end.result.estimatedTokensAfter", "RPC response.data.contextUsage.percent", "RPC response.data.contextUsage.contextWindow"]
    hook_parity: "unknown"
    notes: "Detectable as context pressure/compaction, not provider subscription-plan pressure."
  - name: plan_capped
    detectable: true
    event_types: ["message_end", "auto_retry_start", "auto_retry_end", "compaction_end"]
    fields: ["message.errorMessage", "message.stopReason", "auto_retry_start.errorMessage", "auto_retry_end.finalError", "compaction_end.errorMessage"]
    hook_parity: "unknown"
    notes: "Provider quota/rate-limit exhaustion is surfaced as error text, not normalized quota fields."
  - name: no_funds
    detectable: true
    event_types: ["message_end", "auto_retry_end"]
    fields: ["message.errorMessage", "auto_retry_end.finalError"]
    hook_parity: "unknown"
    notes: "Detect by provider-specific billing text only; no normalized billing event."
  - name: auth
    detectable: true
    event_types: ["message_end", "stderr"]
    fields: ["message.errorMessage", "message.stopReason"]
    hook_parity: "unknown"
    notes: "Missing/invalid auth can fail before a normal agent stream; retain stderr when no terminal event appears."
  - name: permission_read_denied
    detectable: true
    event_types: ["tool_execution_end"]
    fields: ["toolName", "args.path", "result.content[].text", "isError"]
    hook_parity: "unknown"
    notes: "No dedicated permission event. Filesystem access errors appear as tool errors."
  - name: permission_write_denied
    detectable: true
    event_types: ["tool_execution_end"]
    fields: ["toolName", "args.path", "result.content[].text", "isError"]
    hook_parity: "unknown"
    notes: "No dedicated permission event. Write/edit access errors appear as tool errors."
  - name: tokens_consumed
    detectable: true
    event_types: ["message_end", "turn_end", "RPC get_session_stats response"]
    fields: ["message.usage", "turn_end.message.usage", "RPC response.data.tokens", "RPC response.data.contextUsage"]
    hook_parity: "unknown"
    notes: "Assistant usage is per assistant message; RPC session stats aggregate current session totals."
  - name: model_used
    detectable: true
    event_types: ["message_end", "RPC get_state response"]
    fields: ["message.provider", "message.api", "message.model", "RPC response.data.model"]
    hook_parity: "unknown"
    notes: "JSON mode usually reveals model identity when assistant messages arrive, not at startup."
  - name: model_fallback
    detectable: false
    event_types: []
    fields: []
    hook_parity: "unknown"
    notes: "No explicit fallback event verified."
  - name: human_in_loop
    detectable: true
    event_types: ["extension_ui_request"]
    fields: ["method", "id", "timeout"]
    hook_parity: "RPC only"
    notes: "RPC exposes extension dialog requests. JSON/print modes set hasUI false, so well-behaved extensions should not prompt."
  - name: session_resumable
    detectable: true
    event_types: ["session", "RPC get_state response"]
    fields: ["session.id", "session.cwd", "RPC response.data.sessionFile", "RPC response.data.sessionId"]
    hook_parity: "session file"
    notes: "Session header is emitted before run events in JSON mode when sessions are enabled."
  - name: subagent_prompt_injection
    detectable: false
    event_types: []
    fields: []
    hook_parity: "none"
    notes: "No native subagent support verified."
headless_constraints:
  - constraint: "Project-local settings/resources are not prompted for in non-interactive modes."
    mitigation: "Use --approve to load trusted project resources, or --no-approve/--no-context-files/--no-extensions/--no-skills for deterministic locked-down runs."
    notes: "defaultProjectTrust ask and never ignore project resources without saved trust."
  - constraint: "JSON mode does not emit version, auth source, initial tool set, trust decision, or config provenance as startup metadata."
    mitigation: "Capture command line and optional preflight probes separately; use RPC get_state/get_session_stats if a controller can drive RPC."
    notes: "Do not infer absent metadata from config files."
  - constraint: "RPC mode is bidirectional and can block on extension dialog methods."
    mitigation: "Use JSON mode for one-shot Claudine runs; if using RPC, implement extension_ui_request handling and timeout/cancel policy."
    notes: "Dialog methods include select, confirm, input, and editor."
  - constraint: "No dedicated file_change event."
    mitigation: "Infer file changes from successful write/edit tool events and session entries."
    notes: "This is less reliable for extension tools."
  - constraint: "No built-in sandbox or permission system for filesystem/process/network."
    mitigation: "Constrain tools with --tools/--exclude-tools/--no-tools and run Pi in an external sandbox/container when needed."
    notes: "The repository README recommends containerization for stronger boundaries."
quirks:
  - "JSON mode is implemented by print mode with mode=json, not a separate command; it writes the session header before subscribing to later events."
  - "RPC responses include request id correlation, but ordinary agent events do not include an id."
  - "tool_execution_update.partialResult is documented as accumulated output, so displaying it as a delta will duplicate text."
  - "agent_end is the terminal event for the run, but Agent.subscribe listeners for agent_end are still part of settlement."
  - "Project trust affects whether repo-local extensions, skills, prompts, themes, and settings are loaded; this can materially change event shape."
  - "Assistant usage and cost live on assistant messages, not as a standalone usage event in JSON mode."
  - "The official JSON docs lag the source union: source includes entry_appended, session_info_changed, and thinking_level_changed beyond the shorter docs excerpt."
gaps:
  - "No formal JSON Schema or versioned event schema was found for JSON mode or RPC mode."
  - "Exact stderr behavior for every startup/auth/config failure was not exhaustively captured with live executions."
  - "Whether package-install/update paths can ever write non-JSON to stdout during --mode json was not re-tested locally; issue history indicates stdout purity has been a concern."
  - "No normalized rate-limit, quota reset, plan cap, or no-funds fields were found beyond provider error strings."
  - "No direct MCP metadata was found in Pi's JSON/RPC stream."
  - "Exact Windows packaged-binary config path expansion was inferred from source using os.homedir() and path.join."
claudine_strategy:
  preferred_invocation: 'pi --mode json --no-approve --no-extensions --no-skills --no-prompt-templates --no-context-files "PROMPT"'
  required_flags: ["--mode json"]
  conflicting_flags: ["--mode rpc unless Claudine implements the RPC command/UI-response protocol", "-p/text when live events are required"]
  parser_notes: "Parse stdout as JSONL split only on LF. Use top-level type as the discriminator and assistantMessageEvent.type as the nested delta discriminator. Join tool lifecycle by toolCallId. Treat agent_end as terminal, but inspect final assistant stopReason/errorMessage and auto_retry events for success/failure classification."
  wrapper_notes: "For deterministic automation, decide project trust explicitly with --approve or --no-approve and disable project resources unless they are intentionally part of the run. Preserve stderr for diagnostics when no terminal event appears."
data_format: jsonl
changes: []
requires_claudine_update: true
reason: "Pi is researched but not yet one of Claudine's compiled providers. Adding support would require a Pi provider adapter for JSONL AgentSessionEvent parsing and metadata/config defaults."
---

# Pi Non-Interactive Sessions

## Summary

Pi can run non-interactively with structured live output. For a Claudine one-shot wrapper, the best default is `pi --mode json "PROMPT"`, which emits one JSON object per line on stdout: a session header first, then live session events such as `message_update`, `tool_execution_start`, `tool_execution_update`, `tool_execution_end`, `turn_end`, and terminal `agent_end`. This is better than text print mode because it exposes progress before process exit, and it is simpler than RPC because Claudine does not need to drive a bidirectional protocol.

Pi also has `--mode rpc`, a JSONL protocol over stdin/stdout. RPC is valuable for a future long-running Claudine controller because it can send commands like `prompt`, `get_state`, `get_session_stats`, `abort`, `get_entries`, and `switch_session`. It is not the safest first integration target for simple automation because extensions can emit `extension_ui_request` records that require matching `extension_ui_response` records on stdin. If Claudine does not answer those dialog requests, automation can block until an extension-side timeout or forever when no timeout is set.

## Non-Interactive Entry Points

Pi exposes three scriptable surfaces:

| Entry point | Shape | Prompt input | Best use |
| --- | --- | --- | --- |
| `pi -p "PROMPT"` | final text | argv, `@file`, images, piped stdin | Human scripts that only need final answer text |
| `pi --mode json "PROMPT"` | stdout JSONL event stream | argv, `@file`, images, piped stdin | Claudine one-shot automation and live progress parsing |
| `pi --mode rpc --no-session` | bidirectional JSONL protocol | JSONL commands on stdin | IDE/app integration or a controller that can keep a Pi process alive |

The CLI parser accepts `--mode text|json|rpc`, `--print/-p`, model/provider/auth flags, session flags, tool allow/deny flags, extension/resource flags, and project-trust flags. File references are passed as `@path`; images use the same content path mechanism and appear as image content in prompts.

Session behavior is configurable. A plain JSON-mode run starts a normal session and writes a `session` header if session persistence is enabled. `--no-session` makes the session ephemeral. `--continue`, `--session`, `--session-id`, and `--fork` change whether the run resumes or branches from previous state.

The important automation flag is project trust. Pi's settings docs say non-interactive modes do not show the project trust prompt. Without a saved trust decision, global `defaultProjectTrust: "ask"` and `"never"` ignore project-local resources, while `"always"` trusts them. `--approve` trusts project files for one run; `--no-approve` ignores them for one run. Claudine should set this deliberately instead of inheriting a user's trust state by accident.

## Output Formats

`--mode json` should be Claudine's preferred format. It is streaming, line-delimited, and one-way: Claudine can parse stdout without sending additional protocol records. It exposes tool starts, accumulated tool progress, tool results, assistant deltas, compaction, auto-retry, and terminal state.

Text print mode is useful only for final assistant text. Source implementation confirms that when `mode === "text"`, Pi waits for the run to finish, finds the last assistant message, and writes only text blocks to stdout. If the assistant stop reason is `error` or `aborted`, it writes an error to stderr and returns exit code 1. That is too opaque for Claudine's live status needs.

RPC is richer than JSON mode, but it is a protocol rather than an output format. The docs define strict JSONL framing, command responses with optional `id` correlation, live agent events without request ids, and an extension UI subprotocol. RPC gives access to state/stat commands that JSON mode does not automatically emit, especially `get_state` and `get_session_stats`, but Claudine would need a controller loop and policy for `extension_ui_request`.

## Schema Sources

Pi does not publish a formal JSON Schema for the JSON stream. The strongest schema source is the TypeScript source:

| Surface | Best schema evidence | Confidence |
| --- | --- | --- |
| JSON mode stdout | `AgentSessionEvent` in `packages/coding-agent/src/core/agent-session.ts` and `AgentEvent` in `packages/agent/src/types.ts` | High for current source, informal for compatibility |
| RPC stdin/stdout | `RpcCommand`, `RpcResponse`, and `RpcExtensionUIRequest` in `packages/coding-agent/src/modes/rpc/rpc-types.ts` | High for current source, informal for compatibility |
| Session files | `SessionHeader` and `SessionEntry` in `packages/coding-agent/src/core/session-manager.ts` plus the Session File Format docs | High for persisted transcript, not identical to live JSON mode |
| Message payloads | `AssistantMessage`, `ToolResultMessage`, and usage types in `packages/ai/src/types.ts` | High for payload fields, provider-specific data may vary |

The official JSON docs are aligned with these types for the core event families and explicitly state that stdout is JSON lines. The source currently has additional `AgentSessionEvent` variants beyond the shorter JSON docs excerpt, including `entry_appended`, `session_info_changed`, and `thinking_level_changed`. Claudine should therefore treat unknown `type` values as forward-compatible records, not parse failures.

## IO Contract

In JSON mode, stdout is parse-only JSONL. The implementation writes `JSON.stringify(header) + "\n"` for the session header, then subscribes to session events and writes `JSON.stringify(event) + "\n"` for each event. The stream should be parsed line by line with LF as the delimiter. A trailing CR should be stripped if present.

Stderr is diagnostics. Pi prints extension errors, caught startup/runtime errors, and text-mode assistant errors there. Stderr is not part of the JSON event stream, but Claudine should retain it because failures before the first JSON line may only be visible there.

In RPC mode, stdin is not prompt text. It is a JSONL command stream, and stdout contains responses, events, and extension UI requests. The RPC docs explicitly warn clients to split records on LF only and not use line readers that split on Unicode line separators.

## Stream Contract

The top-level discriminator is `type`. `message_update` has a nested discriminator at `assistantMessageEvent.type`. The important nested assistant event types are:

| Nested type | Meaning |
| --- | --- |
| `start` | message generation started |
| `text_start`, `text_delta`, `text_end` | assistant text block lifecycle |
| `thinking_start`, `thinking_delta`, `thinking_end` | thinking block lifecycle |
| `toolcall_start`, `toolcall_delta`, `toolcall_end` | model-emitted tool-call block lifecycle |
| `done` | assistant message complete |
| `error` | assistant stream error or abort |

Tool execution has a separate lifecycle: `tool_execution_start`, zero or more `tool_execution_update`, and `tool_execution_end`. These records join by `toolCallId`. The docs state that `tool_execution_update.partialResult` is accumulated output so a client can replace its display on each update; Claudine must not append it as a raw delta unless it first computes the delta itself.

The terminal event is `agent_end`. The lower-level `AgentEvent` source states that `agent_end` is the last event emitted for a run, while awaited subscribe listeners still count as run settlement. In practice, Claudine can treat `agent_end` as the parser's terminal record and then wait for process exit to collect any trailing diagnostics.

## Session Metadata

JSON mode emits the session header first:

```json
{"type":"session","version":3,"id":"uuid","timestamp":"...","cwd":"/path"}
```

That gives Claudine an early session id and cwd for correlation. The same session id can be used to locate or resume persisted state through Pi's session mechanisms, but the exact resume command depends on whether Claudine uses a session path, partial UUID, or `--session-id`.

Model metadata is not emitted as a dedicated startup event in JSON mode. It appears on assistant messages as fields such as `provider`, `api`, `model`, `usage`, and `stopReason`. RPC can return a full model object earlier through `get_state`, including provider, model id, API, base URL, context window, max tokens, input modalities, and cost table.

Auth source is not emitted as structured metadata. Pi supports CLI `--api-key`, stored `auth.json`, provider environment variables, and custom provider keys from `models.json`, with documented credential resolution order. Claudine should classify auth failures from stderr or assistant `errorMessage`, not from a startup field.

## Event Families

The core JSON-mode event families are:

| Family | Events | Parser value |
| --- | --- | --- |
| Session/run | `session`, `agent_start`, `turn_start`, `turn_end`, `agent_end` | lifecycle and terminal state |
| Messages | `message_start`, `message_update`, `message_end` | assistant text, thinking, tool-call blocks, final answer, usage, errors |
| Tools | `tool_execution_start`, `tool_execution_update`, `tool_execution_end` | native and extension tool calls, progress, results, errors |
| Queues | `queue_update` | pending steering/follow-up messages |
| Compaction | `compaction_start`, `compaction_end` | context pressure, overflow recovery, compaction failures |
| Retry | `auto_retry_start`, `auto_retry_end` | transient provider retry and final retry failure |
| Session mutation | `entry_appended`, `session_info_changed`, `thinking_level_changed` | source-visible session changes, less fully documented in JSON docs |
| RPC-only | `response`, `extension_ui_request`, `extension_error` | protocol control and extension UI |

`agent_end.messages` contains all messages generated during this run. `turn_end.message` contains the completed assistant message for that turn, and `turn_end.toolResults` contains tool result messages. Assistant messages carry `usage` and `cost` fields when provider usage is available. The cost fields are already monetary totals as reported/calculated by Pi's model metadata, but the currency is not explicitly marked in the event.

## Tools

Pi's built-in coding tools are `read`, `bash`, `edit`, `write`, `grep`, `find`, and `ls`. The source also provides convenience sets for coding tools (`read`, `bash`, `edit`, `write`) and read-only tools (`read`, `grep`, `find`, `ls`). The CLI can disable all tools with `--no-tools`, disable built-ins with `--no-builtin-tools`, allowlist with `--tools`, or denylist with `--exclude-tools`.

All tools share the same live stream envelope. `tool_execution_start` exposes `toolCallId`, `toolName`, and `args`. `tool_execution_update` exposes accumulated `partialResult`. `tool_execution_end` exposes `result` and `isError`.

`bash` streams child stdout and stderr together through the tool output path. For LLM tool calls, command exit information is not normalized as a top-level `exitCode` field in the event envelope; wrappers should inspect the tool result content/details and `isError`. RPC also has a separate `bash` command that returns a `BashResult` with `output`, `exitCode`, `cancelled`, `truncated`, and optional `fullOutputPath`, but that is a controller command, not the same as model-requested bash tool execution.

Pi does not emit a dedicated `file_change` event. Successful `write` and `edit` tool executions imply file changes. `edit` results include display-oriented diff and unified patch details. Extension tools can mutate files too, but their result details are extension-defined.

## Completion and Exit Status

For JSON mode, Claudine should treat `agent_end` as the terminal stream event. Success is a terminal run whose final assistant message does not have `stopReason: "error"` or `stopReason: "aborted"`. Failures can appear as:

- assistant `message_end` or `turn_end.message` with `stopReason: "error"` and `errorMessage`
- assistant `stopReason: "aborted"`
- `auto_retry_end` with `success: false` and `finalError`
- `compaction_end` with `result: null`, `aborted: false`, and `errorMessage`
- process exit without a terminal event, with stderr carrying startup/auth/config errors

Process exit code is still useful and should be considered reliable for process failure. It is not enough by itself for rich classification because provider quota, auth, model, and context errors are better described inside assistant/error events.

Token usage and cost are per assistant message in JSON mode. RPC's `get_session_stats` aggregates current session totals into `tokens.input`, `tokens.output`, `tokens.cacheRead`, `tokens.cacheWrite`, `tokens.total`, and `cost`. `contextUsage` contains a current context-window estimate with `tokens`, `contextWindow`, and `percent`, but docs note it can be omitted or null immediately after compaction until a fresh assistant response provides valid usage.

## Blocking Behavior

Pi's most important non-interactive blocking rule is project trust. Non-interactive modes do not prompt. If no saved trust applies, project resources are ignored under the default `ask` behavior. This avoids a TTY prompt, but it also means a run may silently omit repo-local settings, extensions, skills, prompt templates, themes, or system prompt files unless Claudine passes `--approve` or the user has configured trust.

Pi does not include a built-in permission system for restricting filesystem, process, network, or credential access. Tool execution is governed by which tools are enabled, by extensions, and by external OS/container restrictions. Claudine should not assume Pi will ask before bash, write, or edit. For deterministic automation, prefer an explicit tool policy such as `--tools read,grep,find,ls` for read-only review, or run Pi inside an external sandbox.

Extensions are the other blocking surface. In print and JSON modes, extension context has `hasUI: false`, and well-behaved extensions should avoid prompting or return defaults. In RPC mode, `hasUI: true`; dialog methods emit `extension_ui_request` and block until the client sends `extension_ui_response` with a matching `id`, unless the request carries a timeout and the agent auto-resolves it.

## Subagents

No native subagent support was found in the JSON docs, RPC docs, `AgentSessionEvent` union, or `AgentEvent` union. Pi's public site describes it as a customizable harness that skips sub-agents and plan mode by default. For Claudine, subagent start/stop, nested tool calls, subagent model identity, and subagent prompt-injection controls should all be treated as unsupported unless a user installs an extension that implements its own convention.

## Use Case Detection

| Use case | Detectability | Events and fields | Caveat |
| --- | --- | --- | --- |
| `plan_cap_approaching` | partial | `compaction_start`, `compaction_end.result.tokensBefore`, RPC `contextUsage.percent` | Context pressure only, not subscription plan pressure |
| `plan_capped` | text-classified | `message.errorMessage`, `auto_retry_start.errorMessage`, `auto_retry_end.finalError` | Provider-specific wording |
| `no_funds` | text-classified | same error fields | No normalized billing event |
| `auth` | text-classified | stderr, `message.errorMessage` | Startup auth failures may occur before JSON events |
| `permission_read_denied` | yes | `tool_execution_end.isError`, `toolName`, args path, result text | No dedicated permission event |
| `permission_write_denied` | yes | `tool_execution_end.isError`, `toolName`, args path, result text | Same as read denial |
| `tokens_consumed` | yes | assistant `message.usage`, RPC `get_session_stats.data.tokens` | JSON is per assistant message; RPC stats are session totals |
| `model_used` | yes | assistant `provider/api/model`, RPC `get_state.data.model` | JSON mode may not expose it until assistant output |
| `model_fallback` | not verified | none found | No explicit fallback event found |
| `human_in_loop` | RPC only | `extension_ui_request.method`, `id`, `timeout` | JSON/print mode has no UI channel |
| `session_resumable` | yes | `session.id`, `session.cwd`, RPC `sessionFile` | Disabled by `--no-session` |
| `subagent_prompt_injection` | no | none found | No native subagents |

For quota/rate-limit detection, Pi's auto-retry events are useful: `auto_retry_start` includes `attempt`, `maxAttempts`, `delayMs`, and `errorMessage`; final retry failure includes `finalError`. These fields give retry timing in milliseconds, but not quota reset timestamps or plan windows.

## Headless Constraints

The main constraints for Claudine are:

- Use `--mode json` for live one-shot parsing.
- Do not use `-p` or `--mode text` when live progress matters.
- Do not use `--mode rpc` unless Claudine implements stdin command writing, response correlation, cancellation, and extension UI responses.
- Decide project trust explicitly with `--approve` or `--no-approve`.
- Disable project/user resources when deterministic stream shape matters: `--no-extensions`, `--no-skills`, `--no-prompt-templates`, and `--no-context-files`.
- Constrain tools explicitly when file or command side effects are not acceptable.
- Preserve stderr for failures before the session header.
- Treat absent startup metadata as absent, not as a default.

## Timeline

- 2026-05-07: Pi moved to the Earendil Works organization and the package scope changed to `@earendil-works/pi-coding-agent`.
- 2026-07-03: Source inspection was performed against `@earendil-works/pi-coding-agent` version `0.80.3` from the `earendil-works/pi` main branch.

## Quirks and Gaps

The JSON stream has an informal TypeScript schema rather than a formal JSON Schema. That is good enough for an initial parser, but Claudine should version its fixture captures and keep unknown-event handling permissive.

The official JSON docs show the important core events but are shorter than the source union. Source-visible events such as `entry_appended`, `session_info_changed`, and `thinking_level_changed` may appear and should be retained.

Pi's structured stream does not expose every wrapper-grade metadata field Claudine wants. There is no startup record for CLI version, auth source, project trust decision, enabled tool set, config files loaded, or sandbox mode. The session header gives `id`, `version`, `timestamp`, and `cwd`; model/provider/usage arrive later on assistant messages.

File changes are not first-class events. Claudine can infer changes from successful `write` and `edit` tool events, but extension tools may have their own mutation semantics. A robust wrapper should separately snapshot filesystem changes if it needs a provider-independent changed-file list.

## Claudine Integration Notes

Recommended initial invocation:

```bash
pi --mode json --no-approve --no-extensions --no-skills --no-prompt-templates --no-context-files "PROMPT"
```

That command gives Claudine a parseable live stream and a stable resource baseline. When the caller intentionally wants repo-local Pi behavior, replace `--no-approve` with `--approve` and selectively allow resources.

Parser notes:

- Parse stdout as JSONL and never mix stderr into the JSON parser.
- Use top-level `type` as the discriminator.
- For assistant deltas, use `assistantMessageEvent.type`.
- For tools, join `tool_execution_start`, `tool_execution_update`, and `tool_execution_end` by `toolCallId`.
- Treat `tool_execution_update.partialResult` as an accumulated snapshot.
- Treat `agent_end` as the terminal event, then inspect final assistant `stopReason`, `errorMessage`, and retry events before classifying success.
- Keep unknown events in logs and continue.

RPC should be a later, separate adapter. It can provide better state and cancellation control, but only after Claudine implements command ids, response matching, prompt acceptance versus later run failure, `abort`, `get_state`, `get_session_stats`, and `extension_ui_request` policy.

## Changelog

- 2026-07-03: Replaced the prior placeholder/invalid metadata with independent research from Pi official docs and source inspection.

## Sources

- [Pi JSON Event Stream Mode](https://pi.dev/docs/latest/json)
- [Pi RPC Mode](https://pi.dev/docs/latest/rpc)
- [Pi Using Pi](https://pi.dev/docs/latest/usage)
- [Pi Settings](https://pi.dev/docs/latest/settings)
- [Pi Session File Format](https://pi.dev/docs/latest/session-format)
- [Pi Extensions](https://pi.dev/docs/latest/extensions)
- [Pi repository](https://github.com/earendil-works/pi)
- [`packages/coding-agent/src/modes/print-mode.ts`](https://github.com/earendil-works/pi/blob/main/packages/coding-agent/src/modes/print-mode.ts)
- [`packages/coding-agent/src/core/agent-session.ts`](https://github.com/earendil-works/pi/blob/main/packages/coding-agent/src/core/agent-session.ts)
- [`packages/agent/src/types.ts`](https://github.com/earendil-works/pi/blob/main/packages/agent/src/types.ts)
- [`packages/agent/src/agent-loop.ts`](https://github.com/earendil-works/pi/blob/main/packages/agent/src/agent-loop.ts)
- [`packages/coding-agent/src/modes/rpc/rpc-types.ts`](https://github.com/earendil-works/pi/blob/main/packages/coding-agent/src/modes/rpc/rpc-types.ts)
- [`packages/coding-agent/src/core/session-manager.ts`](https://github.com/earendil-works/pi/blob/main/packages/coding-agent/src/core/session-manager.ts)
- [`packages/coding-agent/src/cli/args.ts`](https://github.com/earendil-works/pi/blob/main/packages/coding-agent/src/cli/args.ts)
- [`packages/coding-agent/src/config.ts`](https://github.com/earendil-works/pi/blob/main/packages/coding-agent/src/config.ts)
- [`packages/coding-agent/src/core/tools/index.ts`](https://github.com/earendil-works/pi/blob/main/packages/coding-agent/src/core/tools/index.ts)
