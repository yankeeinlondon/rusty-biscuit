---
$schema: ./_schema.yaml
created: 2026-04-06
last_updated: 2026-07-03
agent: codex
model: default
docs: https://geminicli.com/docs/cli/headless/
invocation:
  - command: 'gemini --output-format stream-json -p "prompt"'
    stdin_support: true
    prompt_arg: "--prompt/-p supplies prompt text; piped stdin is accepted as additional input/context"
    notes: "Starts a fresh headless session and emits JSONL events on stdout."
  - command: 'gemini --output-format stream-json "prompt"'
    stdin_support: true
    prompt_arg: "Variadic positional query; use -p for deterministic wrapper launch"
    notes: "Documented as a query argument, but Gemini defaults to interactive mode in a TTY, so Claudine should prefer -p."
  - command: 'gemini --resume latest --output-format stream-json -p "prompt"'
    stdin_support: true
    prompt_arg: "--prompt/-p supplies the next user turn"
    notes: "Resumes the latest saved project session and emits a new stream for the current run."
  - command: 'gemini --resume <SESSION_ID_OR_INDEX> --output-format stream-json -p "prompt"'
    stdin_support: true
    prompt_arg: "--prompt/-p supplies the next user turn"
    notes: "Resumes a specific saved project session by ID or list index."
  - command: "gemini --experimental-acp"
    stdin_support: true
    prompt_arg: "ACP protocol messages, not a prompt string"
    notes: "Starts experimental ACP mode. This is a bidirectional IDE/protocol surface, not the preferred Claudine stream."
output_formats:
  - name: "text"
    cli_value: "text"
    stream: true
    format: text
    description: "Human-readable default output for headless runs."
    side_effects: "Not parser-safe; stdout can contain prose and stderr can contain warnings/status."
  - name: "json"
    cli_value: "json"
    stream: false
    format: json
    description: "Single final JSON object with session_id, response, stats, error, and warnings fields."
    side_effects: "No live tool or assistant-delta visibility until the process exits."
  - name: "stream-json"
    cli_value: "stream-json"
    stream: true
    format: jsonl
    description: "Newline-delimited JSON events on stdout. This is Claudine's preferred format."
    side_effects: "Stdout becomes parse-only JSONL; ANSI is stripped for programmatic output; internal usage, session_update, tool_update, elicitation, agent_start, and custom events are not projected."
schema_sources:
  - url: "https://geminicli.com/docs/cli/headless/"
    schema_type: examples
    formal: false
    notes: "Official headless docs list output modes, event names, and exit codes but do not publish a JSON Schema."
  - url: "https://github.com/google-gemini/gemini-cli/blob/main/packages/core/src/output/types.ts"
    schema_type: typescript
    formal: false
    notes: "Best exact source for OutputFormat, JsonOutput, JsonStreamEventType, JsonStreamEvent, and StreamStats."
  - url: "https://github.com/google-gemini/gemini-cli/blob/main/packages/core/src/output/stream-json-formatter.ts"
    schema_type: typescript
    formal: false
    notes: "Defines JSONL framing and converts SessionMetrics into stream result stats."
  - url: "https://github.com/google-gemini/gemini-cli/blob/main/packages/cli/src/nonInteractiveCliAgentSession.ts"
    schema_type: typescript
    formal: false
    notes: "Defines how internal AgentEvent values are projected to stdout stream-json events and which internal events are ignored."
  - url: "https://raw.githubusercontent.com/google-gemini/gemini-cli/main/schemas/settings.schema.json"
    schema_type: json_schema
    formal: true
    notes: "Formal schema for settings.json, not for the stream-json event protocol."
cli_params:
  - flag: "-p, --prompt"
    value: "string"
    description: "Pass prompt text and invoke headless mode."
    example: 'gemini -p "summarize this repo"'
  - flag: "--output-format, -o"
    value: "text | json | stream-json"
    description: "Select output format. Claudine should pass stream-json explicitly every run."
    example: "gemini -o stream-json -p prompt"
  - flag: "--model, -m"
    value: "alias or concrete model ID"
    description: "Select requested model; aliases can resolve through Gemini CLI model configuration."
    example: "gemini -m flash -o stream-json -p prompt"
  - flag: "--resume, -r"
    value: "latest | index | session ID"
    description: "Resume a previous project-scoped session."
    example: "gemini --resume latest -o stream-json -p prompt"
  - flag: "--list-sessions"
    value: "boolean"
    description: "List available sessions for the current project and exit; not the agent event stream."
    example: "gemini --list-sessions"
  - flag: "--approval-mode"
    value: "default | auto_edit | yolo | plan"
    description: "Controls tool approval behavior. Use plan for read-only automation or yolo only inside an external sandbox."
    example: "gemini --approval-mode=plan -o stream-json -p prompt"
  - flag: "--yolo, -y"
    value: "boolean"
    description: "Deprecated auto-approve alias; docs recommend --approval-mode=yolo."
    example: "gemini --approval-mode=yolo -o stream-json -p prompt"
  - flag: "--sandbox, -s"
    value: "boolean or configured sandbox"
    description: "Run in a sandboxed environment; settings and GEMINI_SANDBOX can also configure sandboxing."
    example: "gemini --sandbox -o stream-json -p prompt"
  - flag: "--skip-trust"
    value: "boolean"
    description: "Trust the current workspace for this session and skip the folder trust check."
    example: "gemini --skip-trust -o stream-json -p prompt"
  - flag: "--include-directories"
    value: "dir[,dir...]"
    description: "Add extra workspace directories to the session context."
    example: "gemini --include-directories ../shared -o stream-json -p prompt"
  - flag: "--allowed-mcp-server-names"
    value: "name[,name...]"
    description: "Restrict which configured MCP servers are available."
    example: "gemini --allowed-mcp-server-names github -o stream-json -p prompt"
  - flag: "--allowed-tools"
    value: "tool[,tool...]"
    description: "Deprecated allowlist for tools that may run without confirmation; docs point users to the policy engine instead."
    example: "gemini --allowed-tools read_file -o stream-json -p prompt"
  - flag: "--extensions, -e"
    value: "extension list"
    description: "Select extensions; disabling extensions can reduce wrapper drift."
    example: "gemini -e none -o stream-json -p prompt"
  - flag: "--debug, -d"
    value: "boolean"
    description: "Enable debug diagnostics; not part of the JSONL event schema."
    example: "gemini --debug -o stream-json -p prompt"
  - flag: "--experimental-acp"
    value: "boolean"
    description: "Starts experimental ACP mode instead of the normal CLI run."
    example: "gemini --experimental-acp"
  - flag: "--prompt-interactive, -i"
    value: "string"
    description: "Starts the interactive UI with an initial prompt; conflicts with Claudine's non-interactive wrapper."
    example: 'gemini -i "explain this code"'
config_files:
  - os: macos
    scope: system
    path: "/Library/Application Support/GeminiCli/system-defaults.json"
    format: json
    effect: "Lowest-precedence system defaults for model, output, tools, MCP, sandbox, approvals, hooks, telemetry, and other settings."
    notes: "Override path with GEMINI_CLI_SYSTEM_DEFAULTS_PATH."
  - os: linux
    scope: system
    path: "/etc/gemini-cli/system-defaults.json"
    format: json
    effect: "Lowest-precedence system defaults for model, output, tools, MCP, sandbox, approvals, hooks, telemetry, and other settings."
    notes: "Override path with GEMINI_CLI_SYSTEM_DEFAULTS_PATH."
  - os: windows
    scope: system
    path: "C:\\ProgramData\\gemini-cli\\system-defaults.json"
    format: json
    effect: "Lowest-precedence system defaults for model, output, tools, MCP, sandbox, approvals, hooks, telemetry, and other settings."
    notes: "Override path with GEMINI_CLI_SYSTEM_DEFAULTS_PATH."
  - os: macos
    scope: user
    path: "~/.gemini/settings.json"
    format: json
    effect: "User defaults; can set output.format, model, approval mode, tool policy, MCP servers, hooks, telemetry, sandbox, extensions, and context."
    notes: "If GEMINI_CLI_HOME is set, the user .gemini directory is rooted there instead."
  - os: linux
    scope: user
    path: "~/.gemini/settings.json"
    format: json
    effect: "User defaults; can set output.format, model, approval mode, tool policy, MCP servers, hooks, telemetry, sandbox, extensions, and context."
    notes: "If GEMINI_CLI_HOME is set, the user .gemini directory is rooted there instead."
  - os: windows
    scope: user
    path: "%USERPROFILE%\\.gemini\\settings.json"
    format: json
    effect: "User defaults; can set output.format, model, approval mode, tool policy, MCP servers, hooks, telemetry, sandbox, extensions, and context."
    notes: "If GEMINI_CLI_HOME is set, the user .gemini directory is rooted there instead."
  - os: macos
    scope: repo
    path: ".gemini/settings.json"
    format: json
    effect: "Project settings override user settings when the workspace is trusted; can affect tools, MCP, output, model, hooks, sandbox, context, and permissions."
    notes: "Workspace settings are ignored for untrusted folders. Use --skip-trust or GEMINI_CLI_TRUST_WORKSPACE=true only in controlled automation."
  - os: linux
    scope: repo
    path: ".gemini/settings.json"
    format: json
    effect: "Project settings override user settings when the workspace is trusted; can affect tools, MCP, output, model, hooks, sandbox, context, and permissions."
    notes: "Workspace settings are ignored for untrusted folders. Use --skip-trust or GEMINI_CLI_TRUST_WORKSPACE=true only in controlled automation."
  - os: windows
    scope: repo
    path: ".gemini\\settings.json"
    format: json
    effect: "Project settings override user settings when the workspace is trusted; can affect tools, MCP, output, model, hooks, sandbox, context, and permissions."
    notes: "Workspace settings are ignored for untrusted folders. Use --skip-trust or GEMINI_CLI_TRUST_WORKSPACE=true only in controlled automation."
  - os: macos
    scope: system
    path: "/Library/Application Support/GeminiCli/settings.json"
    format: json
    effect: "Highest-precedence file-based system overrides."
    notes: "Override path with GEMINI_CLI_SYSTEM_SETTINGS_PATH; command-line arguments still win."
  - os: linux
    scope: system
    path: "/etc/gemini-cli/settings.json"
    format: json
    effect: "Highest-precedence file-based system overrides."
    notes: "Override path with GEMINI_CLI_SYSTEM_SETTINGS_PATH; command-line arguments still win."
  - os: windows
    scope: system
    path: "C:\\ProgramData\\gemini-cli\\settings.json"
    format: json
    effect: "Highest-precedence file-based system overrides."
    notes: "Override path with GEMINI_CLI_SYSTEM_SETTINGS_PATH; command-line arguments still win."
  - os: macos
    scope: repo
    path: ".env"
    format: text
    effect: "Can supply auth, model, trust, sandbox, telemetry, endpoint, and debug variables."
    notes: "Gemini searches upward from cwd to project root/home and then user home; first .env found is loaded, not merged."
  - os: linux
    scope: repo
    path: ".env"
    format: text
    effect: "Can supply auth, model, trust, sandbox, telemetry, endpoint, and debug variables."
    notes: "Gemini searches upward from cwd to project root/home and then user home; first .env found is loaded, not merged."
  - os: windows
    scope: repo
    path: ".env"
    format: text
    effect: "Can supply auth, model, trust, sandbox, telemetry, endpoint, and debug variables."
    notes: "Gemini searches upward from cwd to project root/home and then user home; first .env found is loaded, not merged."
env_vars:
  - name: "GEMINI_API_KEY"
    effect: "Selects Gemini API key authentication for headless use."
    notes: "Recommended for headless mode when no cached login exists."
  - name: "GOOGLE_API_KEY"
    effect: "Selects Google Cloud API key authentication for Vertex AI express mode."
    notes: "Unset GEMINI_API_KEY/GOOGLE_API_KEY when using ADC or service-account Vertex auth."
  - name: "GOOGLE_APPLICATION_CREDENTIALS"
    effect: "Points Vertex AI to a service account JSON key for non-interactive authentication."
    notes: "Requires GOOGLE_CLOUD_PROJECT and GOOGLE_CLOUD_LOCATION."
  - name: "GOOGLE_CLOUD_PROJECT"
    effect: "Sets Google Cloud project for Code Assist or Vertex AI auth."
    notes: "Checked before GOOGLE_CLOUD_PROJECT_ID."
  - name: "GOOGLE_CLOUD_PROJECT_ID"
    effect: "Fallback Google Cloud project variable."
    notes: "Used when GOOGLE_CLOUD_PROJECT is absent."
  - name: "GOOGLE_CLOUD_LOCATION"
    effect: "Sets Vertex AI location."
    notes: "Required for Vertex AI non-express mode."
  - name: "GEMINI_MODEL"
    effect: "Sets default model and overrides the hardcoded default."
    notes: "Command-line --model should still be used when Claudine needs deterministic model selection."
  - name: "GEMINI_CLI_TRUST_WORKSPACE"
    effect: "When true, trusts the current workspace for the session and bypasses folder trust."
    notes: "Useful for CI; unsafe if the workspace is attacker-controlled."
  - name: "GEMINI_CLI_HOME"
    effect: "Changes the root directory used for user-level .gemini configuration and storage."
    notes: "Useful for wrapper-isolated state."
  - name: "GEMINI_CLI_SYSTEM_DEFAULTS_PATH"
    effect: "Overrides the system-defaults settings path."
    notes: "Can alter effective model, output, tools, MCP, sandbox, hooks, and telemetry."
  - name: "GEMINI_CLI_SYSTEM_SETTINGS_PATH"
    effect: "Overrides the system settings override path."
    notes: "System settings have higher file precedence than user and project settings."
  - name: "GEMINI_SANDBOX"
    effect: "Configures sandboxing outside settings.json."
    notes: "Accepts true, false, docker, podman, or a custom command string."
  - name: "SEATBELT_PROFILE"
    effect: "Selects macOS sandbox-exec profile."
    notes: "macOS only; examples include permissive-open, restrictive-open, strict-open, and strict-proxied."
  - name: "GEMINI_TELEMETRY_ENABLED"
    effect: "Enables or disables telemetry and overrides telemetry.enabled."
    notes: "Telemetry is separate from stdout stream-json."
  - name: "GEMINI_TELEMETRY_TRACES_ENABLED"
    effect: "Enables detailed traces and overrides telemetry.traces."
    notes: "Can include large attributes such as tool outputs and file reads."
  - name: "GEMINI_TELEMETRY_TARGET"
    effect: "Sets telemetry target."
    notes: "Supported values documented as local and gcp."
  - name: "GEMINI_TELEMETRY_OUTFILE"
    effect: "Sets local telemetry output file."
    notes: "Useful as a secondary diagnostic stream, but not equivalent to stream-json."
  - name: "DEBUG"
    effect: "Enables verbose debug logging when true or 1."
    notes: "Excluded from project .env by default."
  - name: "DEBUG_MODE"
    effect: "Enables verbose debug logging when true or 1."
    notes: "Excluded from project .env by default."
  - name: "NO_COLOR"
    effect: "Disables color output."
    notes: "Programmatic stream-json already strips ANSI in projected messages."
io_contract:
  stdout: structured_only
  stderr: diagnostics_only
  stdin: prompt
  framing: jsonl
  noise_handling: "In stream-json mode parse stdout line by line as JSONL. Treat stderr as diagnostics and fallback error evidence, not as lifecycle events."
  notes: "stream-json uses process.stdout.write(JSON.stringify(event) + '\\n'). Ctrl+C handling reads stdin only when stdin is a TTY; non-TTY stdin is prompt/input text, not a bidirectional protocol."
stream_contract:
  discriminator: "type"
  event_ordering: "init is emitted before user message and tool/assistant events; result is terminal on clean completion; fatal errors may exit through stderr/exit code without a result event."
  correlation_fields: ["session_id", "tool_id"]
  terminal_event: "result"
  partial_message_events: true
  unknown_event_policy: "Skip unknown event types after logging; do not fail the wrapper solely because a new event appears."
  notes: "Assistant messages are text deltas with delta=true. Tool results join to tool_use by tool_id. Timestamps are ISO-8601 strings created with new Date().toISOString()."
session_metadata:
  session_id: "init.session_id; json.session_id for single JSON"
  cwd: "Not emitted in stream-json; available to hooks as cwd and inferable from wrapper launch context."
  model: "init.model reports config.getModel(); result.stats.models keys expose models that consumed tokens."
  provider: "Implicit Gemini CLI; not emitted in stream-json."
  auth: "Not emitted; infer only from environment/settings or auth failures."
  version: "Not emitted in stream-json; collect with gemini --version before launch if needed."
  mcp_servers: "Not emitted in stream-json; configured in settings/mcp and visible indirectly through mcp_* tool names."
  permission_mode: "Not emitted in stream-json; infer from wrapper flags/settings such as --approval-mode, --yolo, tools.allowed, and policies."
  notes: "stream-json exposes minimal session metadata early. Hooks receive richer session_id/transcript_path/cwd metadata, but hooks are a separate stdin/stdout contract."
stream_events:
  - event: "init"
    category: session
    fields: ["type", "timestamp", "session_id", "model"]
    notes: "First projected event in stream-json mode."
  - event: "message"
    category: assistant
    fields: ["type", "timestamp", "role", "content", "delta"]
    notes: "User prompt is emitted once with role=user; assistant text emits as delta chunks with role=assistant and delta=true."
  - event: "tool_use"
    category: tool_call
    fields: ["type", "timestamp", "tool_name", "tool_id", "parameters"]
    notes: "Emitted before tool execution from internal tool_request."
  - event: "tool_result"
    category: tool_result
    fields: ["type", "timestamp", "tool_id", "status", "output", "error.type", "error.message"]
    notes: "Emitted after tool execution; output is display text, not necessarily raw stdout/stderr."
  - event: "error"
    category: error
    fields: ["type", "timestamp", "severity", "message"]
    notes: "Non-fatal warnings and some system errors; RESOURCE_EXHAUSTED maps severity to error."
  - event: "result"
    category: usage
    fields: ["type", "timestamp", "status", "error.type", "error.message", "stats.total_tokens", "stats.input_tokens", "stats.output_tokens", "stats.cached", "stats.input", "stats.duration_ms", "stats.tool_calls", "stats.models"]
    notes: "Terminal success/error event when emitted; clean success includes aggregated and per-model token stats."
  - event: "initialize"
    category: session
    fields: []
    notes: "Internal AgentEvent explicitly ignored by stream-json projection."
  - event: "session_update"
    category: session
    fields: []
    notes: "Internal AgentEvent explicitly ignored by stream-json projection."
  - event: "agent_start"
    category: subagent
    fields: []
    notes: "Internal AgentEvent explicitly ignored by stream-json projection."
  - event: "tool_update"
    category: tool_call
    fields: []
    notes: "Internal AgentEvent explicitly ignored by stream-json projection; no live tool progress."
  - event: "elicitation_request"
    category: permission
    fields: []
    notes: "Internal AgentEvent explicitly ignored by stream-json projection."
  - event: "elicitation_response"
    category: permission
    fields: []
    notes: "Internal AgentEvent explicitly ignored by stream-json projection."
  - event: "usage"
    category: usage
    fields: []
    notes: "Internal AgentEvent explicitly ignored by stream-json projection; use result.stats for final usage."
  - event: "custom"
    category: other
    fields: []
    notes: "Internal AgentEvent explicitly ignored by stream-json projection."
tools:
  - name: "run_shell_command"
    call_visible: true
    result_visible: true
    metadata: ["tool_use.tool_name", "tool_use.tool_id", "tool_use.parameters", "tool_result.status", "tool_result.output", "tool_result.error"]
    notes: "Shell commands require confirmation unless approval/policy allows them. Raw stdout/stderr and exit code are not separately structured in stream-json."
  - name: "read_file"
    call_visible: true
    result_visible: true
    metadata: ["tool_use.parameters", "tool_result.output"]
    notes: "Can read text, images, audio, and PDF according to tools docs; file content may be summarized/truncated in display output."
  - name: "read_many_files"
    call_visible: true
    result_visible: true
    metadata: ["tool_use.parameters", "tool_result.output"]
    notes: "Often triggered by @ path syntax; stream does not emit attachment/file-reference events separately."
  - name: "glob"
    call_visible: true
    result_visible: true
    metadata: ["tool_use.parameters", "tool_result.output"]
    notes: "Search tool visibility follows generic tool_use/tool_result."
  - name: "grep_search"
    call_visible: true
    result_visible: true
    metadata: ["tool_use.parameters", "tool_result.output"]
    notes: "Legacy alias search_file_content may appear depending on tool naming/version."
  - name: "list_directory"
    call_visible: true
    result_visible: true
    metadata: ["tool_use.parameters", "tool_result.output"]
    notes: "Directory listing output is not a dedicated event family."
  - name: "replace"
    call_visible: true
    result_visible: true
    metadata: ["tool_use.parameters", "tool_result.status", "tool_result.output", "tool_result.error"]
    notes: "Edit tool requires confirmation unless approval/policy allows it; file changes are visible only through tool events."
  - name: "write_file"
    call_visible: true
    result_visible: true
    metadata: ["tool_use.parameters", "tool_result.status", "tool_result.output", "tool_result.error"]
    notes: "Write tool requires confirmation unless approval/policy allows it; no dedicated file_change event."
  - name: "ask_user"
    call_visible: true
    result_visible: true
    metadata: ["tool_use.parameters", "tool_result.status", "tool_result.output", "tool_result.error"]
    notes: "Interactive clarification tool is automation-sensitive; exact non-TTY answer/failure behavior was not verified."
  - name: "write_todos"
    call_visible: true
    result_visible: true
    metadata: ["tool_use.parameters", "tool_result.output"]
    notes: "Todo state does not have dedicated plan/todo stream events."
  - name: "mcp_*"
    call_visible: true
    result_visible: true
    metadata: ["tool_use.tool_name", "tool_use.parameters", "tool_result.status", "tool_result.error"]
    notes: "MCP tools are named with mcp_<server>_<tool>; server inventory is not emitted at session start."
  - name: "subagent tools"
    call_visible: true
    result_visible: true
    metadata: ["tool_use.tool_name", "tool_id", "tool_result.output"]
    notes: "Subagents are exposed as tools, so parent stream shows only the parent tool call/result, not nested subagent lifecycle."
completion:
  success_event: "result with status=success"
  failure_event: "result with status=error when emitted; otherwise process exit plus stderr/error handling"
  exit_code_reliable: true
  result_fields: ["result.status", "result.error.type", "result.error.message", "json.response", "json.error"]
  cost_fields: []
  usage_fields: ["result.stats.total_tokens", "result.stats.input_tokens", "result.stats.output_tokens", "result.stats.cached", "result.stats.input", "result.stats.duration_ms", "result.stats.tool_calls", "result.stats.models.*"]
  notes: "Official headless exit codes are 0 success, 1 general/API failure, 42 input error, and 53 turn limit exceeded. Claudine should still prefer a terminal result event when present and use exit/stderr when fatal errors bypass result."
blocking_behavior:
  permissions: configurable
  questions: unknown
  tool_approvals: configurable
  notes: "Default approval mode prompts for tools; plan is read-only, auto_edit auto-approves edit tools, and yolo auto-approves all actions but can be disabled by security/admin settings. Headless auth must be preconfigured with cached credentials or environment variables. Exact no-TTY behavior for ask_user and MCP OAuth was not verified."
subagents:
  supported: true
  start_visible: false
  stop_visible: false
  nested_events_visible: false
  prompt_injection_supported: true
  metadata_fields: ["tool_use.tool_name", "tool_result.output"]
  notes: "Subagents operate as tools with separate context loops. Prompt can force a subagent with leading @name syntax, but stream-json ignores internal agent_start and does not expose nested events."
use_cases:
  - name: "plan_cap_approaching"
    detectable: false
    event_types: []
    fields: []
    hook_parity: "unknown"
    notes: "No structured near-cap event was found in stream-json."
  - name: "plan_capped"
    detectable: true
    event_types: ["error", "result", "process_exit"]
    fields: ["error.message", "result.error.message", "exit_code"]
    hook_parity: "unknown"
    notes: "Quota/resource exhaustion can surface as error severity=error or fatal process failure; reset time/window/upgrade URL are not structured."
  - name: "no_funds"
    detectable: true
    event_types: ["error", "result", "process_exit"]
    fields: ["error.message", "result.error.message", "exit_code"]
    hook_parity: "unknown"
    notes: "Detect from provider error text; no dedicated billing event or cost field."
  - name: "auth"
    detectable: true
    event_types: ["result", "process_exit"]
    fields: ["result.error.type", "result.error.message", "stderr", "exit_code"]
    hook_parity: "unknown"
    notes: "FatalAuthenticationError exists in source; headless docs require cached auth or env-based auth."
  - name: "permission_read_denied"
    detectable: true
    event_types: ["tool_result", "error"]
    fields: ["tool_result.tool_id", "tool_result.status", "tool_result.error.type", "tool_result.error.message", "error.message"]
    hook_parity: "BeforeTool and AfterTool hooks expose tool_name/tool_input/tool_response."
    notes: "No normalized read-denied code/path field is guaranteed in stream-json; parse tool name and error text."
  - name: "permission_write_denied"
    detectable: true
    event_types: ["tool_result", "error"]
    fields: ["tool_result.tool_id", "tool_result.status", "tool_result.error.type", "tool_result.error.message", "error.message"]
    hook_parity: "BeforeTool can deny with decision/reason; exit code 2 blocks tool."
    notes: "No dedicated write-denied event or file-change event; infer from write/edit tool names and errors."
  - name: "tokens_consumed"
    detectable: true
    event_types: ["result"]
    fields: ["result.stats.total_tokens", "result.stats.input_tokens", "result.stats.output_tokens", "result.stats.cached", "result.stats.input", "result.stats.models"]
    hook_parity: "unknown"
    notes: "Final session totals in tokens; per-model breakdown is result.stats.models keyed by model name."
  - name: "model_used"
    detectable: true
    event_types: ["init", "result"]
    fields: ["init.model", "result.stats.models"]
    hook_parity: "unknown"
    notes: "init.model is config.getModel; result.stats.models reveals models with token usage."
  - name: "model_fallback"
    detectable: true
    event_types: ["result"]
    fields: ["result.stats.models"]
    hook_parity: "unknown"
    notes: "Infer fallback if result.stats.models contains a different model than requested; no explicit fallback event."
  - name: "human_in_loop"
    detectable: true
    event_types: ["tool_use", "error", "process_stall"]
    fields: ["tool_use.tool_name", "tool_use.parameters", "error.message"]
    hook_parity: "BeforeTool can observe ask_user or confirmation-sensitive tools."
    notes: "ask_user tool calls are visible, but internal elicitation_request is ignored; wrappers need an inactivity timeout."
  - name: "session_resumable"
    detectable: true
    event_types: ["init"]
    fields: ["init.session_id"]
    hook_parity: "hooks include session_id and transcript_path"
    notes: "Use init.session_id with --resume when available; session listing is human text."
  - name: "subagent_prompt_injection"
    detectable: true
    event_types: ["tool_use"]
    fields: ["tool_use.tool_name", "tool_use.parameters"]
    hook_parity: "unknown"
    notes: "Caller can steer subagent use with @subagent_name prompt syntax; nested prompt contents are only visible if projected as tool parameters."
headless_constraints:
  - constraint: "stream-json is a projection, not the full internal event bus."
    mitigation: "Parse stream-json for wrapper status and optionally use hooks/telemetry for deeper diagnostics."
    notes: "Source explicitly ignores usage, session_update, tool_update, elicitation, agent_start, and custom AgentEvents."
  - constraint: "Fatal errors can bypass a terminal result event."
    mitigation: "Use exit code and stderr as fallback when stdout ends without result."
    notes: "The source throws reconstructed fatal errors for fatal internal error events."
  - constraint: "Default tool approval can require a human."
    mitigation: "Use --approval-mode=plan for read-only runs, --approval-mode=yolo only in a sandbox, or configure policy/tools.allowed deliberately."
    notes: "YOLO can be disabled by security/admin settings."
  - constraint: "Headless auth must be configured before launch."
    mitigation: "Use GEMINI_API_KEY or Vertex AI environment variables/service account, or ensure cached credentials already exist."
    notes: "Interactive Google login opens a browser and is not automation-safe."
  - constraint: "Project settings are gated by folder trust."
    mitigation: "Use a wrapper-controlled GEMINI_CLI_HOME and --skip-trust/GEMINI_CLI_TRUST_WORKSPACE only for trusted workspaces."
    notes: "Untrusted project settings are ignored."
  - constraint: "No dedicated file_change, plan, subagent, or tool progress events in stream-json."
    mitigation: "Infer from tool_use/tool_result and final stats; do not promise live file-change or nested subagent rendering."
    notes: "tool_update and agent_start are ignored."
quirks:
  - "settings.json documents output.format as text/json, while the CLI flag supports text/json/stream-json. Claudine should pass --output-format stream-json every run."
  - "stream-json messages are deltas for assistant text; concatenate role=assistant content in order for final answer text."
  - "Tool_result.output is display text, not a typed command result with separate stdout/stderr/exit_code."
  - "MCP server aliases with underscores are discouraged because mcp_<server>_<tool> parsing can misidentify the server."
  - "Hooks are a separate JSON stdin/stdout contract and can expose cwd/transcript_path/tool inputs, but hook events are not emitted as stream-json events."
  - "The installed local Gemini CLI observed during this research was 0.46.0; official docs advertised latest stable 0.45.0 in the changelog area, so source/docs may drift quickly."
gaps:
  - "No local authenticated agent run was executed, so real provider auth/quota errors were not captured."
  - "Exact non-TTY behavior for ask_user, MCP OAuth, and approval prompts was not verified beyond docs/source."
  - "No formal JSON Schema or protocol version marker was found for stream-json events."
  - "Whether stderr can contain non-diagnostic structured telemetry in all debug configurations was not exhaustively verified."
  - "Precise truncation/summarization behavior for each tool_result.output is tool- and setting-dependent."
claudine_strategy:
  preferred_invocation: 'gemini --output-format stream-json --prompt "<prompt>"'
  required_flags: ["--output-format stream-json", "--prompt", "--approval-mode=plan or an explicit sandboxed approval policy"]
  conflicting_flags: ["--prompt-interactive", "--experimental-acp", "--output-format text", "--output-format json for live parsing"]
  parser_notes: "Parse stdout as JSONL using top-level type. Join tool_use/tool_result by tool_id. Concatenate assistant message content where role=assistant and delta=true. Treat result as terminal when present; if missing, classify using exit code and stderr. Skip unknown events."
  wrapper_notes: "Set a controlled cwd and optionally GEMINI_CLI_HOME. Preconfigure auth. Prefer --skip-trust only for trusted workspaces. Use stderr for diagnostics, not primary lifecycle. Collect gemini --version separately because stream-json does not emit it."
data_format: jsonl
changes:
  - "2026-07-03: Refreshed Gemini CLI non-interactive research against official docs, TypeScript source, settings schema, and local 0.46.0 CLI help/version."
requires_claudine_update: false
reason: "No immediate code change is required by the research itself; Claudine should continue preferring stream-json and account for the documented projection gaps."
---

# Gemini CLI Non-Interactive Sessions

## Summary

Gemini CLI can run as a non-interactive agent and can emit structured output. Claudine should prefer `gemini --output-format stream-json --prompt "<prompt>"`, because it emits one JSON object per line on stdout while the run is still active. The stream is suitable for live progress rendering: it includes session initialization, assistant deltas, tool call starts, tool results, warnings/errors, and final usage.

The main caveat is that `stream-json` is not the full internal event bus. The TypeScript projection in `nonInteractiveCliAgentSession.ts` explicitly ignores internal `usage`, `session_update`, `tool_update`, `elicitation_request`, `elicitation_response`, `agent_start`, and `custom` events. Claudine can reliably supervise the public stream, but it cannot claim live tool progress, nested subagent lifecycle, dedicated file-change events, or per-step usage from this mode alone.

## Non-Interactive Entry Points

Gemini's official headless reference says headless mode is triggered when the CLI runs in a non-TTY environment or when a query is supplied with `-p` / `--prompt` ([Headless mode reference](https://geminicli.com/docs/cli/headless/)). The CLI reference also documents a positional `[query..]`, but the CLI defaults to interactive mode in a TTY, so wrappers should use `--prompt` rather than relying on positional detection.

Useful launch forms:

| Purpose | Command shape | Notes |
| --- | --- | --- |
| Fresh structured run | `gemini --output-format stream-json -p "prompt"` | Preferred Claudine launch form. |
| Stdin plus prompt | `cat file | gemini --output-format stream-json -p "summarize"` | Stdin is input/context, not a protocol. |
| Resume latest | `gemini --resume latest --output-format stream-json -p "continue"` | Resumes a saved project-scoped session. |
| Resume selected session | `gemini --resume <id-or-index> --output-format stream-json -p "continue"` | Session listing is human-oriented output. |
| ACP mode | `gemini --experimental-acp` | Experimental bidirectional protocol surface; not the stream Claudine should parse for normal runs. |

Prompts can include `@` file references. The non-interactive source path runs the same at-command processor before sending the query to the agent, so file references can trigger file-reading tools. The `--include-directories` flag adds workspace roots. Model selection is available with `--model`; aliases such as `auto`, `pro`, and `flash` can resolve through model configuration rather than naming the final backend model directly.

## Output Formats

Gemini CLI exposes three headless output modes through `--output-format` / `-o`, documented in the headless reference and CLI cheatsheet ([Headless mode](https://geminicli.com/docs/cli/headless/), [CLI cheatsheet](https://geminicli.com/docs/cli/cli-reference/)).

| Format | CLI value | Framing | Streams? | Claudine recommendation |
| --- | --- | --- | --- | --- |
| Text | `text` | Human text | Yes | Avoid for wrappers; stdout is prose. |
| Single JSON | `json` | One final JSON object | No | Useful for request/reply scripts, weak for lifecycle supervision. |
| Streaming JSON | `stream-json` | JSONL / NDJSON-style line events | Yes | Preferred. Parse stdout live. |

Single JSON includes fields such as `session_id`, `response`, `stats`, `error`, and `warnings` according to the TypeScript `JsonOutput` interface. It is attractive for simple scripts, but Claudine needs live progress and failure classification before process exit. `stream-json` provides that.

`stream-json` writes each event with `process.stdout.write(JSON.stringify(event) + "\n")` in `StreamJsonFormatter` ([source](https://github.com/google-gemini/gemini-cli/blob/main/packages/core/src/output/stream-json-formatter.ts)). In this mode stdout is parse-only JSONL. Stderr remains diagnostic: warnings, debug stacks when enabled, cancellation notices, and fatal error presentation can appear there. Claudine should not merge stderr into the event stream.

## Schema Sources

There is no public JSON Schema for the `stream-json` protocol. The strongest schema evidence is the TypeScript union in `packages/core/src/output/types.ts` ([source](https://github.com/google-gemini/gemini-cli/blob/main/packages/core/src/output/types.ts)). It defines:

| Event | Important fields |
| --- | --- |
| `init` | `timestamp`, `session_id`, `model` |
| `message` | `timestamp`, `role`, `content`, optional `delta` |
| `tool_use` | `timestamp`, `tool_name`, `tool_id`, `parameters` |
| `tool_result` | `timestamp`, `tool_id`, `status`, optional `output`, optional `error.type`, `error.message` |
| `error` | `timestamp`, `severity`, `message` |
| `result` | `timestamp`, `status`, optional `error`, optional `stats` |

The formatter source defines line framing and the final stats projection. `StreamStats` includes aggregate `total_tokens`, `input_tokens`, `output_tokens`, `cached`, `input`, `duration_ms`, `tool_calls`, and a per-model `models` map.

The settings file has a formal JSON Schema at `schemas/settings.schema.json`, but that schema is for configuration, not events. It is still useful for wrapper configuration drift checks.

## IO Contract

With `--output-format stream-json`, stdout is the structured event stream. Each line is independently parseable JSON with a top-level `type`. Assistant text is emitted as `message` events, not as raw prose.

Stderr is diagnostics. The non-interactive source wires user feedback to stderr and only writes JSONL through the formatter. Debug mode can add stack traces to stderr. Fatal errors may be reported through the usual error handler after the stream loop exits, so Claudine should retain stderr for failure reports and classify runs that end without a `result`.

Stdin is prompt/context input. It is not bidirectional in normal headless mode. The source installs Ctrl+C keypress handling only when `process.stdin.isTTY`; non-TTY stdin is not used for interactive protocol replies.

## Stream Contract

The discriminator is `type`. `init` is emitted before the user message and before assistant/tool events. The source then projects internal agent events as follows:

```mermaid
flowchart TD
    A[launch] --> B[init]
    B --> C[user message]
    C --> D{agent stream}
    D --> E[assistant message delta]
    D --> F[tool_use]
    F --> G[tool_result]
    D --> H[error]
    D --> I[result]
```

Tool calls correlate by `tool_id`: `tool_use.tool_id` equals `tool_result.tool_id`. Assistant output is a sequence of `message` events where `role` is `assistant` and `delta` is usually `true`; concatenate those chunks in stream order to reconstruct the final text.

Timestamps are ISO-8601 strings generated with `new Date().toISOString()`, so they are UTC instants. There is no schema version marker in the event stream. Unknown event types should be skipped and logged because the format has no formal compatibility contract.

The terminal event is `result` when the run ends cleanly. Fatal internal errors can throw out of the projection path and be handled by the process-level error handler, so absence of `result` plus non-zero exit is a meaningful failure state.

## Session Metadata

`init` exposes `session_id` and `model` early. The model is `config.getModel()`, so it may be a configured alias or resolved value depending on Gemini CLI model configuration. Final `result.stats.models` is a better signal for which model IDs actually consumed tokens, especially when model routing/fallback is enabled.

The stream does not expose cwd, project root, git branch, provider version, auth kind, approval mode, sandbox mode, roots, or MCP server inventory. Claudine should supply or record these from wrapper context and preflight:

| Metadata | Stream availability | Wrapper strategy |
| --- | --- | --- |
| Session ID | `init.session_id` | Store for resume/log correlation. |
| Model | `init.model`; final `stats.models` | Record requested `--model` separately. |
| Version | Not emitted | Run `gemini --version` outside the stream. |
| Cwd/project | Not emitted | Use wrapper launch cwd. |
| Auth kind | Not emitted | Infer from environment/settings and auth errors. |
| MCP servers | Not emitted | Record settings/Claudine MCP injection; infer MCP calls from `mcp_*` tool names. |
| Approval/sandbox | Not emitted | Record flags/settings supplied by wrapper. |

Hooks are a separate metadata source. The hooks reference says hook stdin includes `session_id`, `transcript_path`, `cwd`, `hook_event_name`, and `timestamp` ([Hooks reference](https://geminicli.com/docs/hooks/reference/)). That is useful for custom instrumentation, but it is not part of stdout `stream-json`.

## Event Families

The public stream has six event names. It does not have dedicated `file_change`, `plan`, `reasoning`, `subagent_start`, `subagent_stop`, `tool_progress`, or `usage_delta` events.

| Family | Event(s) | Notes |
| --- | --- | --- |
| Session | `init`, `result` | `result` is terminal when emitted. |
| Assistant text | `message` | User prompt and assistant deltas share the event name and differ by `role`. |
| Tool calls | `tool_use` | Input parameters are visible. |
| Tool results | `tool_result` | Status and display output are visible; raw stdout/stderr/exit code are not separately structured. |
| Errors/warnings | `error`, `result.error` | Non-fatal errors are streamed; fatal errors may bypass `result`. |
| Usage | `result.stats` | Final aggregate and per-model token counts only. |

Source inspection is important here because the projection explicitly ignores internal events that would otherwise be valuable to Claudine: `usage`, `tool_update`, `session_update`, `elicitation_request`, `elicitation_response`, `agent_start`, and `custom`.

## Tools

Gemini CLI's tools reference lists execution, file-system, interaction, task-tracking, MCP, memory, planning, and system tools ([Tools reference](https://geminicli.com/docs/reference/tools/)). In `stream-json`, they all collapse to the same public envelope: `tool_use` before execution and `tool_result` after execution.

For wrapper purposes:

| Tool signal | Visible? | Details |
| --- | --- | --- |
| Call start | Yes | `tool_use.tool_name`, `tool_id`, `parameters`. |
| Progress | No | Internal `tool_update` is ignored. |
| Result | Yes | `tool_result.status`, `output`, optional `error`. |
| Command stdout/stderr | Partly | Usually folded into display `output`, not separate fields. |
| Command exit code | Not guaranteed | No dedicated `exit_code` field in stream schema. |
| File changes | Indirect | Infer from write/edit tool names and parameters/results. |
| MCP identity | Partly | MCP tools use `mcp_<server>_<tool>` names. |

Approval behavior is configurable. The CLI docs list `--approval-mode` values `default`, `auto_edit`, `yolo`, and `plan`, with `--yolo` deprecated in favor of `--approval-mode=yolo` ([CLI cheatsheet](https://geminicli.com/docs/cli/cli-reference/)). Tools such as `run_shell_command`, `replace`, and `write_file` require confirmation unless policy/settings/approval mode allow them. Claudine should use `plan` for read-only automation or use `yolo` only when an external sandbox and policy envelope make that acceptable.

## Completion and Exit Status

A successful stream ends with:

```json
{"type":"result","timestamp":"...","status":"success","stats":{"total_tokens":0,"input_tokens":0,"output_tokens":0,"cached":0,"input":0,"duration_ms":0,"tool_calls":0,"models":{}}}
```

The exact values vary, but the field shape comes from the TypeScript `ResultEvent` and `StreamStats` interfaces. The official headless docs list exit code `0` for success, `1` for general/API failure, `42` for input error, and `53` for turn limit exceeded ([Headless mode reference](https://geminicli.com/docs/cli/headless/)).

Claudine should prefer the terminal `result` event when it exists. If stdout ends without `result`, fall back to process exit code and stderr. This matters because fatal internal errors are reconstructed and thrown in the non-interactive source path before the normal final result emission.

Single JSON mode puts the final assistant answer in `response`. Streaming mode does not provide a separate final text field, so Claudine must concatenate assistant `message.content` deltas in order.

## Blocking Behavior

Headless authentication must be configured before launch. The authentication docs say headless mode uses existing cached credentials if present; otherwise users must configure environment-variable authentication, such as `GEMINI_API_KEY` or Vertex AI variables ([Authentication setup](https://geminicli.com/docs/get-started/authentication/)). Starting an interactive Google sign-in flow is not safe for automation.

Tool approval can block automation if left at default. Deterministic choices are:

| Goal | Flag/config |
| --- | --- |
| Read-only inspection | `--approval-mode=plan` |
| Controlled edit/run in sandbox | `--approval-mode=yolo` plus `--sandbox` or external sandbox |
| Allow specific tools | Policy/settings such as `tools.allowed` or policy engine configuration |
| Avoid workspace trust prompt | `--skip-trust` or `GEMINI_CLI_TRUST_WORKSPACE=true` in controlled workspaces |

The exact no-TTY behavior for `ask_user`, MCP OAuth, and every approval prompt was not verified with a live authenticated run. Since `elicitation_request` is ignored by the stream projection, Claudine should run with an inactivity timeout and treat visible `ask_user` tool calls as human-in-loop signals.

## Subagents

Gemini CLI supports subagents. The docs describe them as specialists with their own prompt, tools, and independent context loop; they are exposed to the main agent as tools, and a prompt can force one with leading `@subagent_name` syntax ([Subagents](https://geminicli.com/docs/core/subagents/)).

For Claudine, the critical stream fact is that subagents are visible only as parent-level tool calls/results. The source ignores `agent_start`, and there are no public `subagent_start`, `subagent_stop`, nested tool, nested model, or nested usage events in `stream-json`. Claudine can show "called subagent tool X" if the tool name reveals it, but cannot render nested subagent progress from stdout alone.

## Use Case Detection

| Use case | Detectable from stream-json? | How |
| --- | --- | --- |
| `tokens_consumed` | Yes | Final `result.stats.*` and `result.stats.models.*`. |
| `model_used` | Yes | `init.model`; final `result.stats.models` for token-consuming models. |
| `model_fallback` | Inferred | Compare requested model to `result.stats.models` keys. |
| `session_resumable` | Yes | `init.session_id`. |
| `auth` | Yes, mostly failure-side | `result.error`, stderr, exit code; no auth kind field. |
| `plan_capped` / quota exhausted | Partly | `error.severity=error`, `RESOURCE_EXHAUSTED`-derived messages, final failure/exit. Reset windows are not structured. |
| `no_funds` | Partly | Provider error text; no billing-specific event. |
| `permission_read_denied` | Partly | Denied read tool `tool_result.error` or streamed `error.message`; no normalized path/policy fields. |
| `permission_write_denied` | Partly | Denied write/edit tool `tool_result.error`; no dedicated write-denied event. |
| `human_in_loop` | Partly | `ask_user` tool call or stalled run; internal elicitation events are ignored. |
| `subagent_prompt_injection` | Yes | Prompt can use `@subagent`; parent stream may show subagent tool call. |
| `plan_cap_approaching` | No | No near-cap quota event found. |

Hooks can provide stronger permission and tool detail because `BeforeTool` and `AfterTool` receive tool names, inputs, responses, and can deny with structured hook output. But hooks are not a secondary stdout stream; they are separately configured scripts with their own stdin/stdout/stderr contract.

## Headless Constraints

The highest-risk constraints for automation are:

| Constraint | Impact | Mitigation |
| --- | --- | --- |
| Default approvals can prompt | The process may block or fail waiting for a human. | Use `--approval-mode=plan`, explicit policies, or sandboxed `yolo`. |
| Auth can prompt | Browser login is not automation-safe. | Preconfigure API key or Vertex AI credentials. |
| Project trust gates settings | Repo `.gemini/settings.json` may be ignored. | Trust only controlled workspaces with `--skip-trust` or env. |
| Stream omits internal events | No live usage/tool progress/subagent lifecycle. | Parse public stream and optionally add hooks/telemetry. |
| Fatal errors may skip `result` | Terminal state can be ambiguous if parser waits only for `result`. | Combine terminal event, exit code, and stderr. |
| No formal stream schema | Parser may drift across releases. | Generate parser from TypeScript union and tolerate unknown events. |

## Timeline

The currently published headless reference lists `stream-json` as an official output mode and was last updated on March 10, 2026. The local CLI observed during this research was `0.46.0`; the public changelog page still referenced a recent stable release separately, so Gemini CLI should be treated as moving quickly. The document was refreshed on July 3, 2026 against official docs, source on `main`, the settings schema, and local CLI help/version output.

## Quirks and Gaps

The most surprising quirk is that persistent `output.format` in settings is documented as only `text` or `json`, while the CLI flag supports `text`, `json`, and `stream-json`. Claudine should not rely on config to select the streaming format; pass `--output-format stream-json` every time.

The stream also does not expose exact cwd, version, auth kind, approval mode, sandbox, or MCP server inventory. Those are wrapper/preflight facts, not stream facts.

Gaps that remain:

- No authenticated live run was executed, so real auth/quota/provider error payloads were not captured.
- No formal JSON Schema or protocol version marker was found for `stream-json`.
- Exact non-TTY behavior for `ask_user`, MCP OAuth, and default approval prompts needs fixture evidence.
- Tool output truncation/summarization is setting- and tool-dependent.

## Claudine Integration Notes

Recommended command:

```bash
gemini --output-format stream-json --prompt "$PROMPT"
```

For read-only work, add `--approval-mode=plan`. For editing or command execution, use an external sandbox and an explicit approval policy; `--approval-mode=yolo` should not be used as a safety mechanism by itself. Use a wrapper-controlled `GEMINI_CLI_HOME` when Claudine needs isolated config and state.

Parser rules:

- Parse stdout as JSONL and require a top-level `type`.
- Treat stderr as diagnostics and fallback failure evidence.
- Store `init.session_id` immediately.
- Concatenate `message` events where `role=assistant`.
- Join tools by `tool_id`.
- Treat `result` as terminal when present.
- If no `result` arrives, classify using exit code and stderr.
- Skip unknown events and log them for drift analysis.

Streams to avoid:

- `text` output for automation.
- `json` output when live progress matters.
- `--experimental-acp` unless Claudine is intentionally implementing ACP instead of wrapping the CLI stream.

## Changelog

- 2026-07-03: Refreshed Gemini CLI non-interactive research against official headless docs, CLI/config docs, TypeScript stream source, settings schema, and local Gemini CLI `0.46.0` version/help output.

## Sources

- [Gemini CLI headless mode reference](https://geminicli.com/docs/cli/headless/)
- [Gemini CLI cheatsheet / CLI options](https://geminicli.com/docs/cli/cli-reference/)
- [Gemini CLI configuration reference](https://geminicli.com/docs/reference/configuration/)
- [Gemini CLI authentication setup](https://geminicli.com/docs/get-started/authentication/)
- [Gemini CLI tools reference](https://geminicli.com/docs/reference/tools/)
- [Gemini CLI subagents](https://geminicli.com/docs/core/subagents/)
- [Gemini CLI hooks reference](https://geminicli.com/docs/hooks/reference/)
- [Output event TypeScript types](https://github.com/google-gemini/gemini-cli/blob/main/packages/core/src/output/types.ts)
- [Stream JSON formatter source](https://github.com/google-gemini/gemini-cli/blob/main/packages/core/src/output/stream-json-formatter.ts)
- [Non-interactive session projection source](https://github.com/google-gemini/gemini-cli/blob/main/packages/cli/src/nonInteractiveCliAgentSession.ts)
- [Gemini CLI settings JSON Schema](https://raw.githubusercontent.com/google-gemini/gemini-cli/main/schemas/settings.schema.json)
