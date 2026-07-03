---
$schema: ./_schema.yaml
created: 2026-07-02
last_updated: 2026-07-02
agent: codex
model: default
docs: https://pi.dev/docs/latest/json
invocation:
  - command: "pi --mode json \"<prompt>\""
    stdin_support: true
    prompt_arg: "Positional messages plus non-TTY stdin merged into the initial prompt; @file arguments attach text or images."
    notes: "Fresh or selected session depending on session flags. Preferred simple subprocess mode for Claudine live parsing."
  - command: "pi -p \"<prompt>\""
    stdin_support: true
    prompt_arg: "Optional prompt immediately after -p/--print, additional positional messages, and piped stdin."
    notes: "Print mode emits only final assistant text on stdout; stderr receives diagnostics. Useful for humans, weak for wrappers."
  - command: "pi --mode rpc"
    stdin_support: true
    prompt_arg: "JSONL commands on stdin, e.g. {\"id\":\"req-1\",\"type\":\"prompt\",\"message\":\"...\"}; images are base64 ImageContent objects."
    notes: "Starts a long-running bidirectional protocol process. Richer control than JSON mode, but Claudine must send commands, correlate responses, and answer extension UI requests."
  - command: "pi --mode json --continue \"<prompt>\""
    stdin_support: true
    prompt_arg: "Same prompt surfaces as --mode json."
    notes: "Continues the most recent session in the current project."
  - command: "pi --mode json --session <path|id> \"<prompt>\""
    stdin_support: true
    prompt_arg: "Same prompt surfaces as --mode json."
    notes: "Loads a specific session file or matching session ID before running non-interactively."
  - command: "pi --mode json --fork <path|id> \"<prompt>\""
    stdin_support: true
    prompt_arg: "Same prompt surfaces as --mode json."
    notes: "Forks a specific session into a new session before running."
  - command: "TypeScript SDK: createAgentSession(...); session.subscribe(listener); await session.prompt(message)"
    stdin_support: false
    prompt_arg: "Programmatic string plus optional ImageContent array."
    notes: "In-process API; same AgentSessionEvent stream as CLI JSON mode, without subprocess framing."
output_formats:
  - name: "print text"
    cli_value: "-p | --print"
    stream: false
    format: text
    description: "Final assistant text only on stdout after the run completes."
    side_effects: "No live tool, token, retry, or message delta visibility; assistant error/aborted stop reasons print to stderr and exit 1."
  - name: "json event stream"
    cli_value: "--mode json"
    stream: true
    format: ndjson
    description: "One JSON object per stdout line. First line is the session header, followed by AgentSessionEvent records as they occur."
    side_effects: "TUI is not opened; Pi writes JSON via raw stdout and routes accidental stdout writes to stderr."
  - name: "rpc protocol"
    cli_value: "--mode rpc"
    stream: true
    format: jsonrpc_lines
    description: "Strict LF-delimited JSONL command/response/event protocol over stdin/stdout."
    side_effects: "Bidirectional: stdout contains responses, AgentSessionEvent records, extension_ui_request records, and extension_error records; stdin is not prompt text."
  - name: "interactive text"
    cli_value: "default"
    stream: true
    format: text
    description: "Full TUI when stdin and stdout are TTYs."
    side_effects: "Not scriptable and may prompt for project trust, session selection, settings, or extension UI."
schema_sources:
  - url: "https://pi.dev/docs/latest/json"
    schema_type: typescript
    formal: false
    notes: "Official JSON-mode docs list the session header, AgentSessionEvent union, AgentEvent union, and example records."
  - url: "https://github.com/earendil-works/pi/blob/main/packages/coding-agent/src/core/agent-session.ts"
    schema_type: typescript
    formal: false
    notes: "Current source of truth for AgentSessionEvent; includes events not shown on the JSON docs page such as entry_appended, session_info_changed, thinking_level_changed, and agent_end.willRetry."
  - url: "https://github.com/earendil-works/pi/blob/main/packages/agent/src/types.ts"
    schema_type: typescript
    formal: false
    notes: "Current source of truth for AgentEvent lifecycle, message, and tool execution event shapes."
  - url: "https://pi.dev/docs/latest/rpc"
    schema_type: typescript
    formal: false
    notes: "Official RPC docs define command, response, event, extension UI, framing, and error behavior."
  - url: "https://github.com/earendil-works/pi/blob/main/packages/coding-agent/src/modes/rpc/rpc-types.ts"
    schema_type: typescript
    formal: false
    notes: "Authoritative TypeScript union for RpcCommand, RpcResponse, RpcExtensionUIRequest, and RpcExtensionUIResponse."
  - url: "https://pi.dev/docs/latest/session-format"
    schema_type: typescript
    formal: false
    notes: "Documents persisted JSONL session entries and message payloads. Useful context but not identical to the live event stream."
cli_params:
  - flag: "--mode"
    value: "text | json | rpc"
    description: "Selects human text, NDJSON event stream, or bidirectional RPC protocol."
    example: "pi --mode json \"List files\""
  - flag: "--print, -p"
    value: ""
    description: "Non-interactive final-text mode; can consume one following prompt argument."
    example: "pi -p \"Summarize\""
  - flag: "--provider"
    value: "NAME"
    description: "Selects provider context for model resolution."
    example: "pi --provider anthropic --model claude-opus-4-8 --mode json \"task\""
  - flag: "--model"
    value: "PATTERN"
    description: "Model pattern or provider/model ID; supports :thinking suffix."
    example: "pi --model openai/gpt-5.5:high --mode json \"task\""
  - flag: "--api-key"
    value: "KEY"
    description: "Runtime API key override; requires an explicit model selection."
    example: "pi --model anthropic/claude-opus-4-8 --api-key \"$ANTHROPIC_API_KEY\" --mode json \"task\""
  - flag: "--thinking"
    value: "off | minimal | low | medium | high | xhigh"
    description: "Reasoning effort for compatible models."
    example: "pi --thinking high --mode json \"task\""
  - flag: "--models"
    value: "PATTERN[,PATTERN...]"
    description: "Scoped model list for startup selection and cycling."
    example: "pi --models \"claude-*,gpt-*\" --mode json \"task\""
  - flag: "--continue, -c"
    value: ""
    description: "Continue the most recent current-project session."
    example: "pi --mode json --continue \"next step\""
  - flag: "--resume, -r"
    value: ""
    description: "Browse/select a session; unsafe for non-interactive wrappers because it invokes selection UI."
    example: "pi --resume"
  - flag: "--session"
    value: "path|id"
    description: "Use a specific session file or partial UUID."
    example: "pi --mode json --session 019de86e \"continue\""
  - flag: "--session-id"
    value: "id"
    description: "Use an exact project session ID, creating it if missing."
    example: "pi --mode json --session-id ci-run-1 \"task\""
  - flag: "--fork"
    value: "path|id"
    description: "Fork a specific session file or partial UUID; cannot combine with --session, --continue, --resume, or --no-session."
    example: "pi --mode json --fork 019de86e \"try another path\""
  - flag: "--session-dir"
    value: "DIR"
    description: "Custom session storage and lookup directory."
    example: "pi --mode json --session-dir .pi/sessions \"task\""
  - flag: "--no-session"
    value: ""
    description: "Ephemeral mode; do not save session transcript."
    example: "pi --mode json --no-session \"task\""
  - flag: "--name, -n"
    value: "NAME"
    description: "Set session display name at startup across print, JSON, and RPC modes."
    example: "pi --mode json --name \"CI repair\" \"task\""
  - flag: "--tools, -t"
    value: "NAME[,NAME...]"
    description: "Allowlist built-in, extension, and custom tools."
    example: "pi --mode json --tools read,grep,find,ls \"review\""
  - flag: "--exclude-tools, -xt"
    value: "NAME[,NAME...]"
    description: "Disable specific tool names."
    example: "pi --mode json --exclude-tools bash \"task\""
  - flag: "--no-builtin-tools, -nbt"
    value: ""
    description: "Disable built-in tools while leaving extension/custom tools."
    example: "pi --mode json --no-builtin-tools \"task\""
  - flag: "--no-tools, -nt"
    value: ""
    description: "Disable all built-in, extension, and custom tools by default."
    example: "pi --mode json --no-tools \"answer only\""
  - flag: "--extension, -e"
    value: "SOURCE"
    description: "Load an extension file, directory, npm package, or git source; repeatable."
    example: "pi --mode json -e ./audit-extension.ts \"task\""
  - flag: "--no-extensions, -ne"
    value: ""
    description: "Disable extension discovery; explicit -e paths still load."
    example: "pi --mode json --no-extensions \"task\""
  - flag: "--skill"
    value: "PATH"
    description: "Load a skill file or directory; repeatable."
    example: "pi --mode json --skill ./skills/reviewer/SKILL.md \"task\""
  - flag: "--no-skills, -ns"
    value: ""
    description: "Disable skill discovery and loading."
    example: "pi --mode json --no-skills \"task\""
  - flag: "--prompt-template"
    value: "PATH"
    description: "Load a prompt template; repeatable."
    example: "pi --mode json --prompt-template ./prompt.md \"/prompt-name\""
  - flag: "--no-prompt-templates, -np"
    value: ""
    description: "Disable prompt-template discovery and loading."
    example: "pi --mode json --no-prompt-templates \"task\""
  - flag: "--theme"
    value: "PATH"
    description: "Load a theme. Parser-irrelevant in JSON/RPC unless extensions inspect it."
    example: "pi --theme ./theme.json"
  - flag: "--no-themes"
    value: ""
    description: "Disable theme discovery and loading."
    example: "pi --mode json --no-themes \"task\""
  - flag: "--no-context-files, -nc"
    value: ""
    description: "Disable AGENTS.md and CLAUDE.md context discovery."
    example: "pi --mode json --no-context-files \"task\""
  - flag: "--approve, -a"
    value: ""
    description: "Trust project-local settings/resources for this run without a prompt."
    example: "pi --mode json --approve \"task\""
  - flag: "--no-approve, -na"
    value: ""
    description: "Ignore project-local settings/resources for this run."
    example: "pi --mode json --no-approve \"task\""
  - flag: "--offline"
    value: ""
    description: "Disable startup network operations; same behavior as PI_OFFLINE=1."
    example: "pi --mode json --offline \"task\""
  - flag: "--verbose"
    value: ""
    description: "Force verbose startup in interactive mode; no proven JSON stream schema effect."
    example: "pi --verbose"
config_files:
  - os: all
    scope: user
    path: "~/.pi/agent/settings.json"
    format: json
    effect: "Global settings for model defaults, thinking, retry, compaction, resources, terminal display, sessionDir, proxy, defaultProjectTrust, and startup/network behavior."
    notes: "Baseline settings. Project settings override it with nested object merge; arrays/scalars replace. defaultProjectTrust is global-only."
  - os: all
    scope: repo
    path: ".pi/settings.json"
    format: json
    effect: "Project-local overrides for settings, resources, packages, extensions, skills, prompt templates, themes, sessionDir, model defaults, retry, and compaction."
    notes: "Loaded only when project is trusted. Project overrides global; nested objects merge, arrays/scalars replace."
  - os: all
    scope: user
    path: "~/.pi/agent/models.json"
    format: json
    effect: "Custom model/provider catalog; can affect model IDs, providers, costs, context windows, base URLs, and auth surfaces."
    notes: "Parser-relevant because model/provider/cost fields in assistant messages and RPC state come from model definitions."
  - os: all
    scope: repo
    path: ".pi/models.json"
    format: json
    effect: "Project custom model catalog when project resources are trusted."
    notes: "Same model-shape effect as user models; verify exact merge behavior before relying on project model overrides."
  - os: all
    scope: user
    path: "~/.pi/agent/trust.json"
    format: json
    effect: "Saved project trust decisions by canonical directory."
    notes: "Closest saved current-or-parent path decision applies before defaultProjectTrust."
  - os: all
    scope: user
    path: "~/.pi/agent/auth.json"
    format: json
    effect: "Stored provider credentials and OAuth tokens."
    notes: "Auth source is not emitted in JSON mode; do not read this file unless the user explicitly authorizes credential inspection."
  - os: all
    scope: user
    path: "~/.pi/agent/sessions/--{sanitized_cwd}--/{timestamp}_{uuid}.jsonl"
    format: other
    effect: "Persisted session transcript with header and tree entries."
    notes: "Useful for resume/recovery and post-run audit. Not the same as live AgentSessionEvent stream."
  - os: all
    scope: repo
    path: ".pi/extensions, .pi/skills, .pi/prompts, .pi/themes, .pi/SYSTEM.md, .pi/APPEND_SYSTEM.md"
    format: other
    effect: "Project resources that can register tools, commands, UI requests, prompt changes, and behavior."
    notes: "Loaded only after project trust; these resources can materially change non-interactive stream behavior."
env_vars:
  - name: "PI_CODING_AGENT_DIR"
    effect: "Overrides the config directory, including settings.json, models.json, auth.json, trust.json, resources, and default sessions root."
    notes: "Use for isolated wrapper state."
  - name: "PI_CODING_AGENT_SESSION_DIR"
    effect: "Overrides session storage directory unless --session-dir is supplied."
    notes: "Affects session resumability and transcript location."
  - name: "PI_PACKAGE_DIR"
    effect: "Overrides package installation directory."
    notes: "Can change extension/package code loaded into the run."
  - name: "PI_OFFLINE"
    effect: "Disables startup network operations when set to 1/true/yes."
    notes: "Equivalent to --offline."
  - name: "PI_SKIP_VERSION_CHECK"
    effect: "Skips Pi version update check at startup."
    notes: "Prevents the pi.dev latest-version request but does not disable all startup network activity."
  - name: "PI_TELEMETRY"
    effect: "Overrides install/update telemetry and provider attribution headers."
    notes: "Does not disable update checks."
  - name: "PI_CACHE_RETENTION"
    effect: "Sets long prompt-cache retention where supported."
    notes: "Can affect provider request behavior and costs but not stream framing."
  - name: "PI_SHARE_VIEWER_URL"
    effect: "Base URL for /share command."
    notes: "Only relevant if commands/extensions share sessions."
  - name: "PI_STARTUP_BENCHMARK"
    effect: "Interactive-only startup benchmark; non-interactive runs exit with an error when set."
    notes: "Claudine should clear this for headless runs."
  - name: "ANTHROPIC_API_KEY, OPENAI_API_KEY, GEMINI_API_KEY, and other provider API key env vars"
    effect: "Provide provider authentication and determine which auth-gated default models are available."
    notes: "Pi documents many provider-specific auth vars in --help and usage docs; JSON mode does not emit which auth source was used."
  - name: "AWS_PROFILE, AWS_ACCESS_KEY_ID, AWS_SECRET_ACCESS_KEY, AWS_BEARER_TOKEN_BEDROCK, AWS_REGION"
    effect: "Amazon Bedrock authentication and region."
    notes: "Can affect model availability and provider failures."
  - name: "VISUAL, EDITOR"
    effect: "Fallback external editor for interactive Ctrl+G when externalEditor is unset."
    notes: "Not normally used in JSON mode, but extensions may still invoke editor-like UI through RPC."
io_contract:
  stdout: structured_only
  stderr: diagnostics_only
  stdin: prompt
  framing: ndjson
  noise_handling: "For --mode json, parse stdout as one JSON object per LF-delimited line and treat stderr as diagnostics/warnings. Pi's stdout takeover routes accidental stdout writes to stderr in RPC and is used before JSON print-mode execution."
  notes: "--mode rpc changes stdin to stream_protocol and stdout to mixed protocol records. Print mode uses text_only stdout."
stream_contract:
  discriminator: "type"
  event_ordering: "JSON mode emits a session header first when available, then events in subscription order. agent_start starts a run; agent_end is the normal terminal event for accepted prompt processing, but process setup errors can occur before any agent_end."
  correlation_fields: ["toolCallId", "assistantMessageEvent.contentIndex", "entry.id", "entry.parentId", "id in RPC responses and extension_ui_request"]
  terminal_event: "agent_end"
  partial_message_events: true
  unknown_event_policy: "Skip and log at trace; source adds event variants ahead of docs."
  notes: "message_update contains assistantMessageEvent subtypes for text/thinking/tool-call deltas. tool_execution_update.partialResult is accumulated state, not a delta."
session_metadata:
  session_id: "session header $.id at first JSON-mode line; RPC get_state data.sessionId; persisted transcript header $.id"
  cwd: "session header $.cwd at first JSON-mode line; transcript header $.cwd"
  model: "assistant message fields $.provider, $.model, $.api and RPC get_state data.model; not emitted in the initial JSON header"
  provider: "assistant message $.provider and full Model.provider in RPC state/model responses"
  auth: "unknown"
  version: "CLI --version exists but JSON stream does not emit version; session header emits transcript schema version"
  mcp_servers: "unsupported"
  permission_mode: "No native permission mode; only tool allowlists/denylists and project trust"
  notes: "Session header is early enough for log correlation and resume. Model identity appears only after model state is queried over RPC or assistant messages are emitted."
stream_events:
  - event: "session"
    category: session
    fields: ["type", "version", "id", "timestamp", "cwd", "parentSession?"]
    notes: "Header line emitted before AgentSessionEvent records in JSON mode when a session header exists."
  - event: "agent_start"
    category: session
    fields: ["type"]
    notes: "Agent begins processing a prompt."
  - event: "agent_end"
    category: session
    fields: ["type", "messages", "willRetry?"]
    notes: "Current source adds willRetry; docs show messages only."
  - event: "turn_start"
    category: session
    fields: ["type"]
    notes: "One assistant response plus tool calls/results starts."
  - event: "turn_end"
    category: session
    fields: ["type", "message", "toolResults"]
    notes: "Final assistant message for a turn and tool results."
  - event: "message_start"
    category: assistant
    fields: ["type", "message"]
    notes: "Emitted for user, assistant, and toolResult messages."
  - event: "message_update"
    category: assistant
    fields: ["type", "message", "assistantMessageEvent"]
    notes: "Streaming assistant update; nested assistantMessageEvent.type carries start/text/thinking/toolcall/done/error subtypes."
  - event: "message_end"
    category: assistant
    fields: ["type", "message"]
    notes: "Complete message snapshot."
  - event: "tool_execution_start"
    category: tool_call
    fields: ["type", "toolCallId", "toolName", "args"]
    notes: "Tool begins execution; input args visible."
  - event: "tool_execution_update"
    category: tool_result
    fields: ["type", "toolCallId", "toolName", "args", "partialResult"]
    notes: "Tool progress; partialResult is accumulated output."
  - event: "tool_execution_end"
    category: tool_result
    fields: ["type", "toolCallId", "toolName", "result", "isError"]
    notes: "Tool completion and error flag."
  - event: "queue_update"
    category: other
    fields: ["type", "steering", "followUp"]
    notes: "Full pending steering/follow-up queues."
  - event: "compaction_start"
    category: other
    fields: ["type", "reason"]
    notes: "reason is manual, threshold, or overflow."
  - event: "compaction_end"
    category: other
    fields: ["type", "reason", "result", "aborted", "willRetry", "errorMessage?"]
    notes: "result includes summary, firstKeptEntryId, tokensBefore, estimatedTokensAfter, and details when successful."
  - event: "auto_retry_start"
    category: error
    fields: ["type", "attempt", "maxAttempts", "delayMs", "errorMessage"]
    notes: "Transient error retry began."
  - event: "auto_retry_end"
    category: error
    fields: ["type", "success", "attempt", "finalError?"]
    notes: "Retry attempt outcome."
  - event: "entry_appended"
    category: session
    fields: ["type", "entry"]
    notes: "Present in current source; not listed on JSON docs page."
  - event: "session_info_changed"
    category: session
    fields: ["type", "name"]
    notes: "Present in current source; emitted when session display name changes."
  - event: "thinking_level_changed"
    category: reasoning
    fields: ["type", "level"]
    notes: "Present in current source; emitted when effective thinking level changes."
  - event: "extension_error"
    category: error
    fields: ["type", "extensionPath", "event", "error"]
    notes: "Documented for RPC; JSON print mode reports extension errors to stderr through bindExtensions onError."
  - event: "extension_ui_request"
    category: other
    fields: ["type", "id", "method"]
    notes: "RPC-only request from extension UI methods; may require stdin response for dialog methods."
tools:
  - name: "read"
    call_visible: true
    result_visible: true
    metadata: ["toolCallId", "toolName", "args", "result", "isError"]
    notes: "Built-in file read tool. No dedicated permission-denied event; errors appear in tool result or assistant error."
  - name: "bash"
    call_visible: true
    result_visible: true
    metadata: ["toolCallId", "args.command", "partialResult", "result.details", "isError"]
    notes: "Streaming output can appear in tool_execution_update. RPC also has a separate bash command whose result is a response and enters LLM context on the next prompt."
  - name: "edit"
    call_visible: true
    result_visible: true
    metadata: ["toolCallId", "args", "result", "isError"]
    notes: "File modifications are visible as tool calls/results, not as dedicated file_change events."
  - name: "write"
    call_visible: true
    result_visible: true
    metadata: ["toolCallId", "args", "result", "isError"]
    notes: "No native approval flow; command executes with process permissions when tool is available."
  - name: "grep"
    call_visible: true
    result_visible: true
    metadata: ["toolCallId", "args", "result", "isError"]
    notes: "Built-in search tool."
  - name: "find"
    call_visible: true
    result_visible: true
    metadata: ["toolCallId", "args", "result", "isError"]
    notes: "Built-in file discovery tool."
  - name: "ls"
    call_visible: true
    result_visible: true
    metadata: ["toolCallId", "args", "result", "isError"]
    notes: "Built-in directory listing tool."
  - name: "extension/custom tools"
    call_visible: true
    result_visible: true
    metadata: ["toolCallId", "toolName", "args", "partialResult", "result", "isError"]
    notes: "Same tool_execution_* envelope; details schema is tool-defined."
completion:
  success_event: "agent_end with last assistant message stopReason not error/aborted; process exit code 0"
  failure_event: "assistant message stopReason error|aborted, message_update assistantMessageEvent.type=error, auto_retry_end success=false, compaction_end errorMessage, RPC response success=false, or process setup error on stderr"
  exit_code_reliable: true
  result_fields: ["agent_end.messages", "turn_end.message", "message_end.message", "message.content[].text", "message.stopReason", "message.errorMessage"]
  cost_fields: ["assistant message usage.cost.input", "assistant message usage.cost.output", "assistant message usage.cost.cacheRead", "assistant message usage.cost.cacheWrite", "assistant message usage.cost.total", "RPC get_session_stats data.cost"]
  usage_fields: ["assistant message usage.input", "assistant message usage.output", "assistant message usage.cacheRead", "assistant message usage.cacheWrite", "assistant message usage.totalTokens", "RPC get_session_stats data.tokens"]
  notes: "JSON mode returns runPrintMode's exit code; setup/auth/model errors exit 1 before a terminal event. RPC process exits when stdin closes or shutdown is requested, not when a prompt finishes."
blocking_behavior:
  permissions: configurable
  questions: configurable
  tool_approvals: configurable
  notes: "Pi has no native tool approval prompts. Project trust does not prompt in non-interactive modes; ask/never ignore project resources and always/--approve trusts them. RPC extension dialog UI can block until Claudine sends extension_ui_response unless the request has a timeout or the extension handles cancellation."
subagents:
  supported: false
  start_visible: false
  stop_visible: false
  nested_events_visible: false
  prompt_injection_supported: false
  metadata_fields: []
  notes: "Core docs state Pi intentionally does not include built-in sub-agents. SDK/custom tools can spawn their own AgentSession or external process, but the parent stream has no standard subagent events."
use_cases:
  - name: plan_cap_approaching
    detectable: false
    event_types: []
    fields: []
    hook_parity: "none"
    notes: "No native plan-cap event. Context compaction threshold is visible separately through compaction_start/end."
  - name: plan_capped
    detectable: false
    event_types: ["message_update", "message_end", "agent_end", "auto_retry_end"]
    fields: ["assistantMessageEvent.error.errorMessage?", "message.errorMessage", "finalError"]
    hook_parity: "none"
    notes: "Provider quota/cap failures may appear as provider error text, but there is no normalized plan-capped event."
  - name: no_funds
    detectable: false
    event_types: ["message_update", "message_end", "agent_end"]
    fields: ["message.errorMessage", "assistantMessageEvent.error"]
    hook_parity: "none"
    notes: "Only infer from provider error text."
  - name: auth
    detectable: true
    event_types: ["process_setup_error", "message_update", "message_end", "RPC response"]
    fields: ["stderr text", "message.errorMessage", "response.error"]
    hook_parity: "none"
    notes: "Missing model/auth can exit before stream events; provider auth errors may become assistant error messages. Auth source is not exposed."
  - name: permission_read_denied
    detectable: true
    event_types: ["tool_execution_end"]
    fields: ["toolName", "args.path", "result", "isError"]
    hook_parity: "extension tool_call handlers can block tools, but no native permission event"
    notes: "OS/tool errors are visible as failed tool results; no policy decision record."
  - name: permission_write_denied
    detectable: true
    event_types: ["tool_execution_end"]
    fields: ["toolName", "args.path", "result", "isError"]
    hook_parity: "extension tool_call handlers can block tools, but no native permission event"
    notes: "OS/tool errors are visible as failed edit/write/bash results; no native permission mode."
  - name: tokens_consumed
    detectable: true
    event_types: ["message_end", "turn_end", "agent_end", "RPC get_session_stats response"]
    fields: ["message.usage.input", "message.usage.output", "message.usage.cacheRead", "message.usage.cacheWrite", "message.usage.totalTokens", "data.tokens"]
    hook_parity: "none"
    notes: "Assistant-message usage is per assistant response; RPC get_session_stats aggregates current session state."
  - name: model_used
    detectable: true
    event_types: ["message_end", "turn_end", "agent_end", "RPC get_state response"]
    fields: ["message.provider", "message.model", "message.api", "data.model.provider", "data.model.id"]
    hook_parity: "none"
    notes: "Initial JSON header does not carry model."
  - name: model_fallback
    detectable: false
    event_types: []
    fields: []
    hook_parity: "none"
    notes: "Main startup may have a modelFallbackMessage for the interactive UI, but JSON stream does not expose a normalized fallback event."
  - name: human_in_loop
    detectable: true
    event_types: ["extension_ui_request"]
    fields: ["id", "method", "title", "message", "timeout"]
    hook_parity: "RPC extension UI only"
    notes: "Only visible in RPC mode. JSON mode has no channel to answer extension UI dialogs."
  - name: session_resumable
    detectable: true
    event_types: ["session", "RPC get_state response"]
    fields: ["id", "cwd", "data.sessionId", "data.sessionFile"]
    hook_parity: "session transcript"
    notes: "Use --no-session to intentionally disable persistence."
  - name: subagent_prompt_injection
    detectable: false
    event_types: []
    fields: []
    hook_parity: "none"
    notes: "No built-in subagent model."
headless_constraints:
  - constraint: "JSON mode has no formal JSON Schema and source can outrun docs."
    mitigation: "Parse top-level type as an open union and tolerate unknown variants."
    notes: "Current AgentSessionEvent source includes variants absent from the JSON docs page."
  - constraint: "Project trust is skipped, not prompted, in non-interactive modes."
    mitigation: "Pass --approve for trusted repos or --no-approve for deterministic isolation."
    notes: "defaultProjectTrust ask behaves like never without UI."
  - constraint: "No native permission prompts or sandbox."
    mitigation: "Use --no-tools, --tools read,grep,find,ls, --no-extensions, or external containers/sandboxes."
    notes: "Pi runs tools with the OS permissions of the pi process."
  - constraint: "RPC mode can block on extension_ui_request dialog methods."
    mitigation: "Prefer --mode json for one-shot wrapper execution, or implement extension_ui_response defaults/timeouts in an RPC client."
    notes: "Dialog methods are select, confirm, input, and editor."
  - constraint: "--resume invokes session selection UI."
    mitigation: "Use --continue or --session <path|id> for automation."
    notes: "A missing session cwd also fails non-interactively instead of asking."
  - constraint: "Model/auth errors can happen before any session event."
    mitigation: "Classify stderr and exit code when no JSON terminal event was seen."
    notes: "Non-interactive with no model exits 1 after printing the no-model guidance."
  - constraint: "stdin means different things by mode."
    mitigation: "Send prompt text on stdin only for print/JSON/text modes; in RPC send JSONL commands."
    notes: "main.ts intentionally skips piped-stdin prompt reading for RPC."
quirks:
  - "Pi's --mode json is implemented by print mode with mode=json, so final success is inferred from agent_end plus process exit rather than a separate result object."
  - "Current source emits entry_appended, session_info_changed, thinking_level_changed, and agent_end.willRetry even though the JSON docs page omits them."
  - "tool_execution_update.partialResult is accumulated output, not a patch/delta."
  - "RPC events do not have per-event ids; only command responses and extension UI requests use id correlation."
  - "The session header uses type=session like transcript files, while live event records use AgentSessionEvent types."
  - "Pi intentionally lacks built-in MCP, subagents, plan mode, todos, permission popups, and sandboxing; packages/extensions can add workflows with custom schemas."
gaps:
  - "No captured live fixture was generated in this environment because running a real Pi prompt requires configured provider credentials."
  - "Exact stderr vocabulary for every provider auth/quota/billing failure was not exhaustively verified."
  - "No official machine-readable JSON Schema or protocol version for the live JSON stream was found."
  - "Project models.json merge behavior needs separate verification before parser code relies on project-level model overrides."
  - "Whether JSON mode can surface extension UI dialog attempts as structured events was not proven; RPC explicitly can."
claudine_strategy:
  preferred_invocation: "pi --mode json --no-approve --no-extensions \"<prompt>\""
  required_flags: ["--mode json", "--no-approve or --approve depending on wrapper policy"]
  conflicting_flags: ["--mode rpc unless Claudine implements a bidirectional client", "--resume", "PI_STARTUP_BENCHMARK=1"]
  parser_notes: "Parse stdout as LF-delimited JSON objects with top-level type. Treat session as header metadata, agent_end as normal terminal event, message_update.assistantMessageEvent.type as nested delta discriminator, toolCallId as tool correlation, and unknown events as non-fatal."
  wrapper_notes: "Capture stderr for setup/auth/model/resource errors. Prefer --no-extensions and explicit tool allowlists for deterministic automation; use RPC only when Claudine needs get_state/get_session_stats/extension UI control."
data_format: ndjson
changes: []
requires_claudine_update: true
reason: "Pi is not currently in Claudine's compiled provider set. Supporting it would require a new provider adapter for Pi's NDJSON AgentSessionEvent stream and provider metadata for Pi's all-permissive tool model, project trust, and RPC caveats."
---

## Summary

Pi can run non-interactively with structured output. For Claudine's normal wrapper use, the best entry point is `pi --mode json "<prompt>"`: stdout is an NDJSON stream whose first record is a session header and whose remaining records are live `AgentSessionEvent` objects. This gives Claudine live assistant deltas, tool starts, tool updates, tool completions, retries, compaction, session metadata, and final `agent_end` without needing to drive a protocol.

`pi --mode rpc` is richer, but it is not just an output format. It is a bidirectional JSONL protocol over stdin/stdout. A client sends commands such as `prompt`, `get_state`, `get_session_stats`, `abort`, and `set_model`; stdout then contains command responses, agent events, extension UI requests, and extension errors. That is valuable for an embedded UI or long-running controller, but it is a bigger integration surface for Claudine because it must correlate response `id`s and answer or cancel extension UI dialogs. Print mode (`-p`) is scriptable but too opaque for wrapper progress because it only emits final assistant text.

## Non-Interactive Entry Points

Pi's CLI syntax is `pi [options] [@files...] [messages...]`. The official usage docs list four relevant modes: default interactive mode, `-p` / `--print`, `--mode json`, and `--mode rpc` ([usage docs](https://pi.dev/docs/latest/usage)). The current CLI parser accepts the same mode values in source (`packages/coding-agent/src/cli/args.ts`).

For one-shot automation, `pi --mode json "Your prompt"` is the right launch form. It does not open the TUI, and it streams structured records while the agent is still active. Positional messages become prompt text. Non-TTY stdin is read and merged into the initial prompt for non-RPC modes; Pi's main entry point explicitly skips this stdin prompt handling when the app mode is RPC because RPC stdin is reserved for JSONL commands.

`pi -p "Your prompt"` is also non-interactive, and the docs show piped stdin being merged into the prompt. It is useful when a caller only needs the final text. It is not enough for Claudine's lifecycle needs because it suppresses live events and emits errors only as stderr text plus a non-zero exit.

`pi --mode rpc` starts a long-running headless protocol process. A client sends JSON objects on stdin, one per line. A prompt is a command such as:

```json
{"id":"req-1","type":"prompt","message":"Hello, world!"}
```

Images can be sent in that command using `ImageContent` objects with base64 data and `mimeType`. RPC can also query state, messages, session entries, model lists, session stats, commands, and last assistant text, and can send `abort`, `steer`, `follow_up`, `compact`, `set_model`, and `set_thinking_level` commands ([RPC docs](https://pi.dev/docs/latest/rpc)).

Session-related flags work with non-interactive modes: `--continue`, `--session <path|id>`, `--fork <path|id>`, `--session-id <id>`, `--session-dir <dir>`, `--no-session`, and `--name`. Avoid `--resume` in wrappers because it is a session picker. If a session file points at a missing cwd, the non-interactive path prints an error and exits rather than asking where to continue.

## Output Formats

| Mode | Selector | Format | Streams? | Claudine recommendation |
|------|----------|--------|----------|-------------------------|
| Print text | `-p`, `--print` | Text | No live structured stream | Avoid for wrapping; only final answer text |
| JSON event stream | `--mode json` | NDJSON on stdout | Yes | Prefer for one-shot subprocess runs |
| RPC | `--mode rpc` | Bidirectional JSONL | Yes | Use only when Claudine implements protocol client behavior |
| Interactive | default when stdin/stdout are TTYs | TUI text/control sequences | Yes, but human UI | Do not use for automation |

Pi's JSON event stream is documented as "all session events as JSON lines to stdout" ([JSON docs](https://pi.dev/docs/latest/json)). Each stdout line is a complete JSON object. The first line is a session header:

```json
{"type":"session","version":3,"id":"uuid","timestamp":"...","cwd":"/path"}
```

After that, events stream as they occur: `agent_start`, `turn_start`, `message_start`, `message_update`, `tool_execution_start`, `tool_execution_update`, `tool_execution_end`, `turn_end`, and `agent_end`, among others.

The JSON stream is better than print mode because Claudine can render progress and classify many failures before process exit. It is simpler than RPC because stdout is a one-way event stream for the run, and stdin can still be treated as prompt text. The main tradeoff is that JSON mode does not expose request/response state queries such as `get_session_stats`; Claudine should compute what it can from assistant messages and use persisted transcripts if it needs post-run audit details.

RPC is more expressive but operationally different. stdout is a mix of command responses, agent events, `extension_ui_request` records, and `extension_error` records. stdin is not prompt text; it is the protocol input. If an extension asks for `select`, `confirm`, `input`, or `editor`, the agent waits for an `extension_ui_response` unless the request times out. That makes RPC the right choice for an embedded controller, not the default simple wrapper.

## Schema Sources

Pi does not publish a formal JSON Schema for the live stream. The strongest schema evidence is TypeScript source plus official docs.

The JSON docs point to `AgentSessionEvent` and `AgentEvent`. In the current repository, `AgentSessionEvent` is defined in [`packages/coding-agent/src/core/agent-session.ts`](https://github.com/earendil-works/pi/blob/main/packages/coding-agent/src/core/agent-session.ts), and `AgentEvent` is defined in [`packages/agent/src/types.ts`](https://github.com/earendil-works/pi/blob/main/packages/agent/src/types.ts). The docs are useful, but source inspection matters because source currently includes variants not shown on the JSON page: `entry_appended`, `session_info_changed`, `thinking_level_changed`, and `agent_end.willRetry`.

RPC has a similarly informal but strong schema. The RPC docs describe commands, responses, events, extension UI requests, framing, and error responses. The exact command/response union lives in [`packages/coding-agent/src/modes/rpc/rpc-types.ts`](https://github.com/earendil-works/pi/blob/main/packages/coding-agent/src/modes/rpc/rpc-types.ts). The strict LF-only framing implementation lives in [`packages/coding-agent/src/modes/rpc/jsonl.ts`](https://github.com/earendil-works/pi/blob/main/packages/coding-agent/src/modes/rpc/jsonl.ts).

The persisted session file format is related but not identical. Session transcripts are JSONL files with a `type` discriminator and tree links via `id` / `parentId` ([session format docs](https://pi.dev/docs/latest/session-format)). They are useful for resume and post-run audit, but the live event stream includes transient lifecycle events that are not persisted.

## IO Contract

In `--mode json`, stdout should be treated as parse-only NDJSON. Pi's print-mode implementation writes the session header and events using raw stdout writes. Diagnostics, extension errors in JSON print mode, model/setup errors, and warnings go to stderr.

Each record is LF-delimited JSON. RPC docs make the framing requirement explicit for RPC: split only on `\n`, accept `\r\n` by stripping a trailing `\r`, and do not use generic line readers that split on Unicode line separators. Claudine should use the same conservative LF-only parser for JSON mode.

stdin depends on mode. In `-p` and `--mode json`, non-TTY stdin is prompt text and is merged into the initial message. In `--mode rpc`, stdin is a JSONL protocol channel and must not be used for raw prompt text.

stderr is not structured, but it is not ignorable. Setup failures, missing model/auth guidance, resource-load errors, extension errors in print/JSON mode, and non-interactive trust/resource warnings may appear there. If stdout never produces a terminal event, Claudine should classify based on stderr plus process exit code.

## Stream Contract

The top-level discriminator is `type`. For assistant deltas, the nested discriminator is `assistantMessageEvent.type` inside `message_update`. For RPC extension UI, the nested discriminator is `method` inside `extension_ui_request`.

JSON-mode event order is suitable for a single-pass parser. The session header appears first when available. A prompt run emits `agent_start`, one or more turns, and then `agent_end` when the agent is done. Message deltas are partial events; `message_update.message` is a snapshot of the current partial assistant message, while `assistantMessageEvent` is the delta/control event. Tool progress uses `toolCallId` for correlation; `tool_execution_update.partialResult` is accumulated output so a renderer can replace its current display rather than append blindly.

There is no separate `result` object in JSON mode. `agent_end` is the normal terminal event. Failures can also be visible earlier as `assistantMessageEvent.type: "error"`, assistant `stopReason: "error"` or `"aborted"`, failed tool results, `compaction_end.errorMessage`, and `auto_retry_end.success: false`.

Claudine should treat the stream union as open. Pi's docs already lag current source in small ways, and packages/extensions can add behavior around the core events. Unknown top-level `type` values should be skipped with trace logging, not treated as fatal parser errors.

## Session Metadata

The JSON stream begins with a session header containing `type`, `version`, `id`, `timestamp`, and `cwd`. This gives Claudine an early session ID and cwd for logs and resume. If `--no-session` is used, verify fixture behavior before assuming a persisted session file exists.

Model identity is not in the initial header. It appears on assistant messages as fields such as `provider`, `model`, `api`, and `usage`, and in RPC through `get_state` or model responses as a full `Model` object. This means JSON-mode Claudine cannot display the resolved model until the first assistant message or until it reads a transcript/state through another channel.

Auth source is not emitted in the JSON event stream. Pi can use stored credentials in `~/.pi/agent/auth.json`, provider environment variables, runtime `--api-key`, OAuth tokens, AWS ambient credentials, and custom model/provider configuration. Claudine should not claim more than "provider/model observed" unless it controls the invocation.

The JSON stream does not emit CLI version. `pi --version` exists, and the session header `version` is the session file schema version, not the CLI version.

## Event Families

Pi's event families are compact and directly useful:

| Family | Events | Parser value |
|--------|--------|--------------|
| Session/run | `session`, `agent_start`, `agent_end`, `turn_start`, `turn_end` | Start/end, final message, retry intent, session identity |
| Messages | `message_start`, `message_update`, `message_end` | Assistant text, thinking, tool-call deltas, final assistant snapshots |
| Tools | `tool_execution_start`, `tool_execution_update`, `tool_execution_end` | Tool input, progress, output, status, errors |
| Queue/control | `queue_update`, `compaction_start`, `compaction_end`, `auto_retry_start`, `auto_retry_end` | Steering/follow-up queues, context compaction, transient provider retry |
| Source-only additions | `entry_appended`, `session_info_changed`, `thinking_level_changed` | Transcript append, session naming, reasoning level changes |
| RPC-only | `response`, `extension_ui_request`, `extension_error` | Command correlation, human-in-loop dialogs, extension failures |

The nested `assistantMessageEvent.type` values include `start`, `text_start`, `text_delta`, `text_end`, `thinking_start`, `thinking_delta`, `thinking_end`, `toolcall_start`, `toolcall_delta`, `toolcall_end`, `done`, and `error` according to the RPC docs. These nested values are parser-significant because top-level `message_update` alone does not tell Claudine whether the update is visible assistant text, hidden reasoning, tool-call JSON, or an error.

## Tools

Pi's built-in tools are `read`, `bash`, `edit`, `write`, `grep`, `find`, and `ls`. The tool visibility flags apply to built-in, extension, and custom tools: `--tools`, `--exclude-tools`, `--no-builtin-tools`, and `--no-tools`.

All built-in and custom tools use the same live event envelope. `tool_execution_start` exposes `toolCallId`, `toolName`, and `args`. `tool_execution_update` exposes the same identity plus `partialResult`. `tool_execution_end` exposes `result` and `isError`. File changes do not have a separate `file_change` event; they are inferred from `edit`, `write`, or `bash` tool calls and results.

The `bash` tool is visible like any other model-called tool. RPC also exposes a separate `bash` command for the controlling client. That command immediately runs shell, returns a `response` with `output`, `exitCode`, `cancelled`, `truncated`, and optional `fullOutputPath`, and adds a `BashExecutionMessage` to context for the next prompt. The RPC docs explicitly say that this direct RPC bash command does not emit an agent event by itself.

Pi has no native MCP support in the core docs reviewed here. MCP-like behavior can be added by extensions, but then the schema is extension-defined and should not be parsed as a first-class Pi event family.

## Completion and Exit Status

For JSON mode, normal completion is `agent_end`. The process exit code is still important. Setup failures can exit before a session header or terminal event. Print/JSON mode returns exit 1 when final assistant text mode sees an assistant `stopReason` of `error` or `aborted`, and setup diagnostics also exit 1.

For wrapper classification, Claudine should use both stream semantics and process status:

- Successful run: saw `agent_end`, final assistant message is not `stopReason: "error"` or `"aborted"`, and process exits 0.
- Agent/provider failure: assistant error delta, assistant message `stopReason: "error"` or `"aborted"`, `auto_retry_end.success: false`, `compaction_end.errorMessage`, failed RPC response, or non-zero process exit.
- Cancellation/interruption: process exits 129/143 for SIGHUP/SIGTERM paths in print/RPC mode, or assistant `stopReason: "aborted"` where the provider encoded it.

RPC is different: the process is long-running and exits when stdin closes or a shutdown is requested. A prompt command response only means the prompt was accepted or rejected before acceptance. Failures after acceptance are reported in the event/message stream, not as a second response for the same request id.

Usage and cost are available on assistant messages and through RPC `get_session_stats`. Assistant messages include usage fields such as `input`, `output`, `cacheRead`, `cacheWrite`, `totalTokens`, and nested `cost` fields. RPC session stats aggregate the current session state and include token totals, cost, and context usage when available.

## Blocking Behavior

Pi has no native tool approval prompt, permission popup, or sandbox. Its security docs state that built-in tools and extensions run with the permissions of the process that launched Pi, and that project trust is not a sandbox ([security docs](https://pi.dev/docs/latest/security)). For unattended runs, the deterministic controls are tool visibility flags, project trust flags, and external containment.

Project trust is the main built-in non-interactive gate. In interactive mode, Pi can ask whether to trust project-local `.pi` settings/resources/packages/extensions. Non-interactive modes (`-p`, `--mode json`, and `--mode rpc`) do not show that trust prompt. Without a saved trust decision, global `defaultProjectTrust: "ask"` behaves like "do not load those resources"; `"never"` also ignores them; `"always"` trusts them. `--approve` and `--no-approve` override this for one run.

RPC adds a programmable human-in-loop surface for extensions. Extension dialog methods emit `extension_ui_request` and block until the client sends `extension_ui_response`, unless the request has an agent-side timeout or is cancelled by a signal. Fire-and-forget UI methods such as `notify`, `setStatus`, `setWidget`, `setTitle`, and `set_editor_text` do not require responses.

## Subagents

Pi core does not include built-in subagents. The usage docs explicitly say Pi intentionally does not include sub-agents, plan mode, todos, permission popups, built-in MCP, or background bash; those workflows can be built or installed as extensions/packages.

The SDK can create multiple `AgentSession` instances, and custom tools can spawn other processes or sessions. That is not a standard parent-stream subagent model. There are no core events such as `subagent_start`, `subagent_stop`, nested session IDs, or nested tool call relays in the JSON stream. If a Pi extension implements a subagent, Claudine should treat it as extension-specific until that extension publishes a stable schema.

## Use Case Detection

| Use case | Detection | Notes |
|----------|-----------|-------|
| `plan_cap_approaching` | Not natively detectable | No plan/quota cap event. Context compaction threshold is a different signal. |
| `plan_capped` | Infer only from provider error text | Look at assistant errors, `auto_retry_end.finalError`, and stderr. No normalized reset time/window. |
| `no_funds` | Infer only from provider error text | No billing-specific event. |
| `auth` | Detectable as setup error, assistant error, or RPC failure | Missing model/auth can exit before stream events; auth source is not exposed. |
| `permission_read_denied` | Failed `read`/`grep`/`find`/`ls` tool result | OS/tool denial is visible; no policy decision record. |
| `permission_write_denied` | Failed `edit`/`write`/`bash` tool result | OS/tool denial is visible; no native approval mode. |
| `tokens_consumed` | Assistant `usage` or RPC `get_session_stats` | Assistant usage is per response; RPC stats aggregate session state. |
| `model_used` | Assistant `provider`/`model`/`api` or RPC state | Not in initial JSON header. |
| `model_fallback` | Not natively detectable in JSON stream | Startup may have UI fallback messaging, but no normalized event. |
| `human_in_loop` | RPC `extension_ui_request` | JSON mode cannot answer these requests. |
| `session_resumable` | JSON header `id`/`cwd`, RPC `sessionId`/`sessionFile` | `--no-session` changes persistence assumptions. |
| `subagent_prompt_injection` | Not applicable | No built-in subagent facility. |

## Headless Constraints

The main automation risk is not output parsing; it is execution authority. Pi's default posture is permissive. If Claudine runs Pi unattended against an untrusted repo, it should use external isolation or restrict tools with flags such as `--no-tools`, `--tools read,grep,find,ls`, `--no-extensions`, and `--no-approve`.

The second risk is protocol choice. `--mode rpc` may look like "more structured JSON," but it can require live client responses. Claudine should not use it as a drop-in replacement for `--mode json` unless it implements a proper RPC loop, including default behavior for `extension_ui_request`.

The third risk is schema drift. There is no formal stream schema, and current source has already moved beyond the public JSON docs. A parser must tolerate unknown top-level event types and optional fields.

## Timeline

- 2026-07-02: Verified Pi documentation and current repository source for non-interactive modes, JSON event stream, RPC protocol, settings, project trust, security posture, and tool event types.

## Quirks and Gaps

Pi's JSON mode is strong for a one-shot wrapper, but it lacks a formal schema, an explicit terminal result object, initial model metadata, auth-source metadata, permission decision records, and dedicated file-change events. Use `agent_end` plus process exit as completion, and infer file changes from tool calls/results.

The public JSON docs are slightly behind source. This is not a problem if Claudine treats the event union as open, but it is a problem for generated parsers that assume an exact closed list.

No live fixture was captured during this research because running a real prompt requires configured provider credentials. The document therefore relies on official docs and source inspection for stream shape and on existing Pi research docs in this repo for model, logging, and permission context.

## Claudine Integration Notes

Recommended subprocess shape:

```bash
pi --mode json --no-approve --no-extensions "<prompt>"
```

For trusted repos where project resources are intentionally part of the task, use `--approve` instead of `--no-approve`. For read-only review, add `--tools read,grep,find,ls`. For tool-free inference, add `--no-tools`.

Parser notes:

- Parse stdout as LF-delimited JSON objects.
- Expect the first object to be `{"type":"session", ...}` when a session header exists.
- Use top-level `type` as the event discriminator.
- Use `message_update.assistantMessageEvent.type` for assistant text/thinking/toolcall/error deltas.
- Join tool lifecycle events by `toolCallId`.
- Treat `tool_execution_update.partialResult` as accumulated state.
- Treat unknown event types as non-fatal.
- Capture stderr and classify it when no terminal `agent_end` appears.

Avoid `--mode rpc` until Claudine has a bidirectional Pi client. If Claudine does add RPC support, it should parse stdout as a mixed stream of `response`, `extension_ui_request`, `extension_error`, and AgentSessionEvent objects; send every command with an `id`; and provide deterministic default responses for extension dialog methods.

## Sources

- [Pi JSON Event Stream Mode](https://pi.dev/docs/latest/json)
- [Pi RPC Mode](https://pi.dev/docs/latest/rpc)
- [Pi Usage / CLI Reference](https://pi.dev/docs/latest/usage)
- [Pi Settings](https://pi.dev/docs/latest/settings)
- [Pi Security](https://pi.dev/docs/latest/security)
- [Pi Containerization](https://pi.dev/docs/latest/containerization)
- [Pi Session File Format](https://pi.dev/docs/latest/session-format)
- [Pi repository](https://github.com/earendil-works/pi)
- [`packages/coding-agent/src/core/agent-session.ts`](https://github.com/earendil-works/pi/blob/main/packages/coding-agent/src/core/agent-session.ts)
- [`packages/agent/src/types.ts`](https://github.com/earendil-works/pi/blob/main/packages/agent/src/types.ts)
- [`packages/coding-agent/src/modes/print-mode.ts`](https://github.com/earendil-works/pi/blob/main/packages/coding-agent/src/modes/print-mode.ts)
- [`packages/coding-agent/src/modes/rpc/rpc-types.ts`](https://github.com/earendil-works/pi/blob/main/packages/coding-agent/src/modes/rpc/rpc-types.ts)
- [`packages/coding-agent/src/modes/rpc/rpc-mode.ts`](https://github.com/earendil-works/pi/blob/main/packages/coding-agent/src/modes/rpc/rpc-mode.ts)
- [`packages/coding-agent/src/modes/rpc/jsonl.ts`](https://github.com/earendil-works/pi/blob/main/packages/coding-agent/src/modes/rpc/jsonl.ts)
- [`packages/coding-agent/src/cli/args.ts`](https://github.com/earendil-works/pi/blob/main/packages/coding-agent/src/cli/args.ts)
- [`packages/coding-agent/src/main.ts`](https://github.com/earendil-works/pi/blob/main/packages/coding-agent/src/main.ts)
