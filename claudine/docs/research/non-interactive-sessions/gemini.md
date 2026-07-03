---
$schema: ./_schema.yaml
created: 2026-04-06
last_updated: 2026-07-02
agent: codex
model: default
docs: https://geminicli.com/docs/cli/headless/
invocation:
  - command: 'gemini --output-format stream-json -p "prompt"'
    stdin_support: true
    prompt_arg: "--prompt/-p string; stdin is additional context when piped"
    notes: "Starts a fresh non-interactive session and emits JSONL events on stdout."
  - command: 'gemini --output-format stream-json "prompt"'
    stdin_support: true
    prompt_arg: "Variadic positional query; docs prefer -p because positional arguments can default to interactive mode in a TTY"
    notes: "Can run headlessly when input or output is piped or redirected, but Claudine should prefer -p for deterministic non-interactive launch."
  - command: 'gemini --resume latest --output-format stream-json -p "prompt"'
    stdin_support: true
    prompt_arg: "--prompt/-p string; resumes the most recent project session"
    notes: "Resumes a saved project-scoped session; stream still starts with an init event for the current run."
  - command: 'gemini --resume <SESSION_ID> --output-format stream-json -p "prompt"'
    stdin_support: true
    prompt_arg: "--prompt/-p string; session ID, session index, or latest can be supplied to --resume"
    notes: "Resumes a specific saved session and emits the same JSONL stream."
  - command: 'gemini --experimental-acp'
    stdin_support: true
    prompt_arg: "ACP client protocol, not a prompt string"
    notes: "Starts experimental ACP mode. It is a bidirectional integration surface, not the preferred Claudine wrapper stream."
output_formats:
  - name: "text"
    cli_value: "text"
    stream: true
    format: text
    description: "Human-readable stdout in non-interactive mode; intermediate status and warnings may be human text on stderr."
    side_effects: "Not parser-safe for lifecycle supervision; use only for humans."
  - name: "json"
    cli_value: "json"
    stream: false
    format: json
    description: "One final JSON object with optional session_id, response, stats, error, and warnings."
    side_effects: "No live tool, warning, or assistant-delta visibility until process completion."
  - name: "stream-json"
    cli_value: "stream-json"
    stream: true
    format: jsonl
    description: "Newline-delimited JSON events on stdout with type values init, message, tool_use, tool_result, error, and result. Claudine should prefer this mode."
    side_effects: "Stdout becomes parse-only JSONL; source strips ANSI for programmatic output. Internal usage, session_update, tool_update, elicitation, agent_start, and custom events are intentionally not projected."
schema_sources:
  - url: "https://github.com/google-gemini/gemini-cli/blob/main/packages/core/src/output/types.ts"
    schema_type: typescript
    formal: false
    notes: "Best exact source for OutputFormat, JsonOutput, JsonStreamEventType, JsonStreamEvent, StreamStats, and per-event fields."
  - url: "https://github.com/google-gemini/gemini-cli/blob/main/packages/core/src/output/stream-json-formatter.ts"
    schema_type: typescript
    formal: false
    notes: "Defines JSONL framing and maps SessionMetrics into the simplified result.stats shape."
  - url: "https://github.com/google-gemini/gemini-cli/blob/main/packages/cli/src/nonInteractiveCliAgentSession.ts"
    schema_type: typescript
    formal: false
    notes: "Authoritative projection from internal AgentEvent values to stream-json stdout; also shows ignored internal events and fatal-error handling."
  - url: "https://geminicli.com/docs/cli/headless/"
    schema_type: examples
    formal: false
    notes: "Official headless docs list output modes, stream event names, and exit codes, but do not publish a complete JSON Schema."
  - url: "https://raw.githubusercontent.com/google-gemini/gemini-cli/main/schemas/settings.schema.json"
    schema_type: json_schema
    formal: true
    notes: "Formal schema for settings.json only; useful for configuration, not for stream-json output."
cli_params:
  - flag: "-p, --prompt"
    value: "string"
    description: "Pass prompt text and force non-interactive mode."
    example: 'gemini --output-format stream-json -p "summarize this repo"'
  - flag: "--output-format, -o"
    value: "text | json | stream-json"
    description: "Select non-interactive output format. Claudine should always pass stream-json explicitly."
    example: 'gemini -o stream-json -p "review"'
  - flag: "--resume, -r"
    value: "latest | index | session UUID"
    description: "Resume a saved project-scoped session."
    example: 'gemini --resume latest --output-format stream-json -p "continue"'
  - flag: "--list-sessions"
    value: "boolean"
    description: "List project sessions and exit; output is human text, not the agent stream."
    example: "gemini --list-sessions"
  - flag: "--model, -m"
    value: "model alias or concrete model"
    description: "Requested model for the session. The stream init.model reports config.getModel(), which may be a configured/resolved name rather than all backend routing details."
    example: 'gemini --model gemini-2.5-flash --output-format stream-json -p "run"'
  - flag: "--approval-mode"
    value: "default | auto_edit | yolo | plan"
    description: "Controls tool approval policy. Use yolo only inside an external sandbox; use plan to prevent mutating tools."
    example: 'gemini --approval-mode=plan --output-format stream-json -p "inspect only"'
  - flag: "--yolo, -y"
    value: "boolean"
    description: "Deprecated alias for automatic tool approval; docs recommend --approval-mode=yolo."
    example: 'gemini --approval-mode=yolo --output-format stream-json -p "fix tests"'
  - flag: "--sandbox, -s"
    value: "boolean or configured sandbox"
    description: "Enable sandbox mode for tool execution."
    example: 'gemini --sandbox --output-format stream-json -p "run tests"'
  - flag: "--skip-trust"
    value: "boolean"
    description: "Trust the current workspace for this session and skip the folder trust check."
    example: 'gemini --skip-trust --output-format stream-json -p "review"'
  - flag: "--include-directories"
    value: "dir[,dir...]"
    description: "Add up to five extra workspace directories; may be specified multiple times."
    example: 'gemini --include-directories ../shared --output-format stream-json -p "inspect"'
  - flag: "--allowed-mcp-server-names"
    value: "name[,name...]"
    description: "Restrict which configured MCP servers are allowed for the session."
    example: 'gemini --allowed-mcp-server-names github --output-format stream-json -p "use MCP"'
  - flag: "--extensions, -e"
    value: "extension names or none"
    description: "Select extensions; use -e none to disable extensions in controlled automation."
    example: 'gemini -e none --output-format stream-json -p "run"'
  - flag: "--debug, -d"
    value: "boolean"
    description: "Enable verbose diagnostic logging; debug details go through stderr/user-feedback paths, not the JSONL schema."
    example: 'gemini --debug --output-format stream-json -p "run"'
  - flag: "--experimental-acp"
    value: "boolean"
    description: "Start experimental ACP mode instead of the normal CLI run."
    example: "gemini --experimental-acp"
  - flag: "--prompt-interactive, -i"
    value: "string"
    description: "Starts the interactive UI with an initial prompt; conflicts with Claudine's non-interactive wrapper."
    example: 'gemini -i "explain this code"'
config_files:
  - os: linux
    scope: system
    path: "/etc/gemini-cli/system-defaults.json"
    format: json
    effect: "Lowest-precedence system defaults for settings that can include output.format, model, sandbox, policy, MCP, UI, and telemetry."
    notes: "Path can be overridden with GEMINI_CLI_SYSTEM_DEFAULTS_PATH."
  - os: macos
    scope: system
    path: "/Library/Application Support/GeminiCli/system-defaults.json"
    format: json
    effect: "Lowest-precedence system defaults."
    notes: "Path can be overridden with GEMINI_CLI_SYSTEM_DEFAULTS_PATH."
  - os: windows
    scope: system
    path: "C:\\ProgramData\\gemini-cli\\system-defaults.json"
    format: json
    effect: "Lowest-precedence system defaults."
    notes: "Path can be overridden with GEMINI_CLI_SYSTEM_DEFAULTS_PATH."
  - os: all
    scope: user
    path: "~/.gemini/settings.json or $GEMINI_CLI_HOME/.gemini/settings.json"
    format: json
    effect: "User defaults for output.format, model, sandbox, approvals/policy, MCP servers, extensions, context, telemetry, hooks, and UI."
    notes: "Overrides system defaults. output.format only documents text and json, so stream-json should be supplied as a CLI flag."
  - os: all
    scope: repo
    path: ".gemini/settings.json"
    format: json
    effect: "Project-local settings, including MCP servers, policies, context file names, sandbox profiles, hooks, and output.format."
    notes: "Overrides user settings and system defaults. Trust behavior can gate workspace access; use --skip-trust or GEMINI_CLI_TRUST_WORKSPACE=true in CI when appropriate."
  - os: linux
    scope: system
    path: "/etc/gemini-cli/settings.json"
    format: json
    effect: "Highest-precedence system override settings for all users."
    notes: "Path can be overridden with GEMINI_CLI_SYSTEM_SETTINGS_PATH."
  - os: macos
    scope: system
    path: "/Library/Application Support/GeminiCli/settings.json"
    format: json
    effect: "Highest-precedence system override settings for all users."
    notes: "Path can be overridden with GEMINI_CLI_SYSTEM_SETTINGS_PATH."
  - os: windows
    scope: system
    path: "C:\\ProgramData\\gemini-cli\\settings.json"
    format: json
    effect: "Highest-precedence system override settings for all users."
    notes: "Path can be overridden with GEMINI_CLI_SYSTEM_SETTINGS_PATH."
  - os: all
    scope: repo
    path: ".env"
    format: text
    effect: "Environment source for auth, model, sandbox, telemetry, base URL, and trust behavior."
    notes: "Loaded from cwd or nearest parent until project root/home; project .env excludes DEBUG/DEBUG_MODE by default, while .gemini/.env files are never excluded."
  - os: all
    scope: user
    path: "~/.gemini/tmp/<project_hash>/chats/"
    format: other
    effect: "Project-scoped persisted sessions used by --resume and --list-sessions."
    notes: "Stores conversation history, tool executions, token usage, and reasoning summaries when available."
env_vars:
  - name: "GEMINI_API_KEY"
    effect: "Authentication for Gemini API key mode."
    notes: "One of several supported auth sources; never log value."
  - name: "GOOGLE_API_KEY"
    effect: "API key for Google Cloud / Vertex AI express use cases."
    notes: "Auth behavior depends on selected auth mode."
  - name: "GOOGLE_APPLICATION_CREDENTIALS"
    effect: "Path to Google Application Credentials JSON for Vertex AI."
    notes: "Treat path and file as sensitive."
  - name: "GOOGLE_CLOUD_PROJECT"
    effect: "Project ID required for Code Assist or Vertex AI."
    notes: "Can affect provider quota and billing classification."
  - name: "GOOGLE_CLOUD_LOCATION"
    effect: "Vertex AI region for non-express mode."
    notes: "Required for some Vertex AI modes."
  - name: "GEMINI_MODEL"
    effect: "Default model override."
    notes: "CLI --model takes precedence."
  - name: "GEMINI_CLI_HOME"
    effect: "Changes the root for user-level Gemini CLI configuration and storage."
    notes: "Useful for isolated Claudine runs."
  - name: "GEMINI_CLI_TRUST_WORKSPACE"
    effect: "When true, trusts the current workspace for this session and bypasses folder trust."
    notes: "Useful for headless CI, but should be paired with external workspace policy."
  - name: "GEMINI_CLI_TRUSTED_FOLDERS_PATH"
    effect: "Overrides trustedFolders.json location."
    notes: "Affects trust gating."
  - name: "GEMINI_SANDBOX"
    effect: "Alternative to sandbox setting; accepts true, false, docker, podman, or a custom command string."
    notes: "Can change whether mutating tools run in isolation."
  - name: "GEMINI_SYSTEM_MD"
    effect: "Replaces the built-in system prompt from a Markdown file."
    notes: "Changes agent instructions and therefore wrapper-visible behavior."
  - name: "GEMINI_TELEMETRY_ENABLED"
    effect: "Overrides telemetry.enabled."
    notes: "Telemetry is a secondary surface, not the stream-json contract."
  - name: "GEMINI_TELEMETRY_TRACES_ENABLED"
    effect: "Overrides telemetry.traces."
    notes: "Can add detailed tracing outside stdout."
  - name: "GEMINI_TELEMETRY_TARGET"
    effect: "Selects telemetry target such as local or gcp."
    notes: "Use only as secondary evidence."
  - name: "GEMINI_TELEMETRY_OUTFILE"
    effect: "Path for local telemetry output."
    notes: "Potential secondary lifecycle data; not a replacement for stream-json."
  - name: "GEMINI_CLI_ACTIVITY_LOG_TARGET"
    effect: "Enables initial activity logging in nonInteractiveCliAgentSession.ts."
    notes: "Development/devtools surface; not documented as the stable wrapper stream."
io_contract:
  stdout: structured_only
  stderr: diagnostics_only
  stdin: prompt
  framing: jsonl
  noise_handling: "With --output-format stream-json, parse stdout as one JSON object per line and treat stderr as diagnostics/user feedback. Do not merge stderr into the event parser."
  notes: "Programmatic output strips ANSI from assistant text and errors. Stdin is prompt/context, except experimental ACP which is a separate bidirectional protocol."
stream_contract:
  discriminator: "type"
  event_ordering: "init is emitted before user message and agent loop; tool_use precedes its matching tool_result; result is emitted on normal agent_end success. Fatal errors are handled after the catch path and may rely on process exit/stderr rather than a result event."
  correlation_fields: ["session_id", "tool_id"]
  terminal_event: "result"
  partial_message_events: true
  unknown_event_policy: "Skip unknown top-level type values after logging parser telemetry; preserve raw event for fixtures because the TypeScript union is not versioned."
  notes: "message events use role user or assistant; assistant message chunks use delta: true. Timestamps are ISO-8601 strings from new Date().toISOString()."
session_metadata:
  session_id: "init.session_id always in stream-json; JsonOutput.session_id optional in final json"
  cwd: "not emitted in stream-json; available to hooks as cwd"
  model: "init.model reports config.getModel()"
  provider: "not emitted; inferred from executable/provider"
  auth: "not emitted"
  version: "not emitted; use gemini --version outside the run if needed"
  mcp_servers: "not emitted; infer from settings/CLI flags or tool_name prefix mcp_<server>_<tool>"
  permission_mode: "not emitted; infer from invocation/config"
  notes: "The stream exposes early session_id and model only. Project root, trust, sandbox, auth source, version, roots, and MCP inventory are wrapper-side/config facts unless reflected indirectly in tool names or errors."
stream_events:
  - event: "init"
    category: session
    fields: ["type", "timestamp", "session_id", "model"]
    notes: "First stream-json event; enough for session log correlation and resume hints."
  - event: "message"
    category: assistant
    fields: ["type", "timestamp", "role", "content", "delta"]
    notes: "Represents user prompt snapshot and assistant text chunks. Assistant chunks set delta: true."
  - event: "tool_use"
    category: tool_call
    fields: ["type", "timestamp", "tool_name", "tool_id", "parameters"]
    notes: "Emitted before tool execution from internal tool_request."
  - event: "tool_result"
    category: tool_result
    fields: ["type", "timestamp", "tool_id", "status", "output", "error.type", "error.message"]
    notes: "Emitted after tool execution from internal tool_response; joins to tool_use by tool_id."
  - event: "error"
    category: error
    fields: ["type", "timestamp", "severity", "message"]
    notes: "Non-fatal warnings and system errors projected from internal non-fatal error events."
  - event: "result"
    category: session
    fields: ["type", "timestamp", "status", "error.type", "error.message", "stats"]
    notes: "Final normal stream event. stats includes aggregate and per-model token usage, duration_ms, and tool_calls."
  - event: "internal initialize"
    category: other
    fields: []
    notes: "Internal AgentEvent explicitly ignored by non-interactive stream-json."
  - event: "internal session_update"
    category: other
    fields: []
    notes: "Internal AgentEvent explicitly ignored; stream-json does not expose cwd/project/session updates beyond init."
  - event: "internal agent_start"
    category: subagent
    fields: []
    notes: "Internal AgentEvent explicitly ignored; no parent-visible subagent start event."
  - event: "internal tool_update"
    category: tool_call
    fields: []
    notes: "Internal AgentEvent explicitly ignored; no structured tool progress."
  - event: "internal elicitation_request"
    category: permission
    fields: []
    notes: "Internal AgentEvent explicitly ignored; human-in-loop attempts are not directly projected."
  - event: "internal elicitation_response"
    category: permission
    fields: []
    notes: "Internal AgentEvent explicitly ignored."
  - event: "internal usage"
    category: usage
    fields: []
    notes: "Internal AgentEvent explicitly ignored; only final result.stats exposes usage."
  - event: "internal custom"
    category: other
    fields: []
    notes: "Internal AgentEvent explicitly ignored."
tools:
  - name: "read/search/filesystem tools"
    call_visible: true
    result_visible: true
    metadata: ["tool_use.tool_name", "tool_use.tool_id", "tool_use.parameters", "tool_result.status", "tool_result.output", "tool_result.error"]
    notes: "No dedicated file-read event; file activity appears as generic tool_use/tool_result and may be summarized in display output."
  - name: "write/edit/delete tools"
    call_visible: true
    result_visible: true
    metadata: ["tool_use.parameters", "tool_result.status", "tool_result.output", "tool_result.error.type"]
    notes: "No dedicated file_change event. Permission denials are likely tool_result error or error message, not a normalized permission event."
  - name: "run_shell_command"
    call_visible: true
    result_visible: true
    metadata: ["tool_use.parameters", "tool_result.output", "tool_result.status", "tool_result.error"]
    notes: "The tool documentation says command, directory, stdout, stderr, exit code, and background PIDs are returned, but stream-json flattens visible result text into output."
  - name: "web tools"
    call_visible: true
    result_visible: true
    metadata: ["tool_name", "tool_id", "parameters", "output", "status"]
    notes: "Visible only through generic tool events."
  - name: "MCP tools"
    call_visible: true
    result_visible: true
    metadata: ["tool_name", "tool_id", "parameters", "output", "status"]
    notes: "MCP tools are discovered from configured servers and named with mcp_<serverAlias>_<tool>; server aliases containing underscores are a documented policy-engine footgun."
  - name: "subagent tools"
    call_visible: true
    result_visible: true
    metadata: ["tool_name", "tool_id", "parameters", "output", "status"]
    notes: "Subagents are exposed to the main agent as tools; nested subagent events are not projected separately in stream-json."
completion:
  success_event: "result with status=success"
  failure_event: "result with status=error when emitted; fatal failures may exit non-zero without result"
  exit_code_reliable: true
  result_fields: ["message.content for assistant deltas", "result.status", "result.error", "JsonOutput.response", "JsonOutput.error", "JsonOutput.warnings"]
  cost_fields: []
  usage_fields: ["result.stats.total_tokens", "result.stats.input_tokens", "result.stats.output_tokens", "result.stats.cached", "result.stats.input", "result.stats.duration_ms", "result.stats.tool_calls", "result.stats.models.<model>.total_tokens", "result.stats.models.<model>.input_tokens", "result.stats.models.<model>.output_tokens", "result.stats.models.<model>.cached", "result.stats.models.<model>.input"]
  notes: "Official headless exit codes are 0 success, 1 general/API failure, 42 input error, and 53 turn limit exceeded. Claudine should trust result when present and still classify non-zero exits/stderr when fatal errors prevent a terminal event."
blocking_behavior:
  permissions: configurable
  questions: unknown
  tool_approvals: configurable
  notes: "Default security policy can require human confirmation for mutating file and shell tools. Non-interactive behavior for ignored elicitation_request events is not fully documented; use --approval-mode=plan, --approval-mode=yolo in an external sandbox, policy settings, --skip-trust, and/or GEMINI_CLI_TRUST_WORKSPACE=true to avoid mid-run prompts."
subagents:
  supported: true
  start_visible: false
  stop_visible: false
  nested_events_visible: false
  prompt_injection_supported: true
  metadata_fields: ["tool_use.tool_name", "tool_use.tool_id", "tool_use.parameters", "tool_result.output"]
  notes: "Gemini subagents are exposed as tools and can be forced with @subagent syntax; stream-json shows the parent tool call/result, not nested subagent lifecycle or model/session metadata."
use_cases:
  - name: "plan_cap_approaching"
    detectable: false
    event_types: []
    fields: []
    hook_parity: "unknown"
    notes: "No stream-json event exposes remaining plan quota, threshold, or reset time."
  - name: "plan_capped"
    detectable: true
    event_types: ["error", "result", "process_exit"]
    fields: ["error.message", "result.error.message", "exit_code"]
    hook_parity: "unknown"
    notes: "Detect by RESOURCE_EXHAUSTED-derived error severity or fatal provider/API messages; exact cap window/reset fields are not structured."
  - name: "no_funds"
    detectable: true
    event_types: ["error", "result", "process_exit"]
    fields: ["error.message", "result.error.message", "stderr"]
    hook_parity: "unknown"
    notes: "Only message classification; no billing balance fields."
  - name: "auth"
    detectable: true
    event_types: ["result", "process_exit"]
    fields: ["result.error.type", "result.error.message", "stderr", "exit_code"]
    hook_parity: "unknown"
    notes: "FatalAuthenticationError is reconstructed in source, but fatal errors may be handled outside stream-json; parse stderr/non-zero exit too."
  - name: "permission_read_denied"
    detectable: true
    event_types: ["tool_result", "error"]
    fields: ["tool_result.tool_id", "tool_result.error.type", "tool_result.error.message", "tool_result.output", "error.message"]
    hook_parity: "hooks expose BeforeTool/AfterTool inputs separately"
    notes: "No normalized permission event or path field; infer path from tool parameters joined by tool_id."
  - name: "permission_write_denied"
    detectable: true
    event_types: ["tool_result", "error"]
    fields: ["tool_use.parameters", "tool_result.error.type", "tool_result.error.message", "error.message"]
    hook_parity: "hooks expose BeforeTool/AfterTool inputs separately"
    notes: "Distinguish from model/tool failures by tool family and error text/type."
  - name: "tokens_consumed"
    detectable: true
    event_types: ["result"]
    fields: ["result.stats.total_tokens", "result.stats.input_tokens", "result.stats.output_tokens", "result.stats.cached", "result.stats.input", "result.stats.models"]
    hook_parity: "session transcript/telemetry may contain richer usage"
    notes: "Final session aggregate only; no per-step stream usage because internal usage events are ignored."
  - name: "model_used"
    detectable: true
    event_types: ["init", "result"]
    fields: ["init.model", "result.stats.models"]
    hook_parity: "unknown"
    notes: "init.model is early; result.stats.models is the per-model token map after completion."
  - name: "model_fallback"
    detectable: true
    event_types: ["result"]
    fields: ["init.model", "result.stats.models"]
    hook_parity: "unknown"
    notes: "Inferred only when final stats include model keys different from init.model; no explicit fallback event."
  - name: "human_in_loop"
    detectable: false
    event_types: []
    fields: []
    hook_parity: "hooks can block/allow tools, but stream-json omits elicitation_request/response"
    notes: "Internal elicitation events are explicitly ignored, so a parser cannot reliably detect a pending prompt from stdout alone."
  - name: "session_resumable"
    detectable: true
    event_types: ["init"]
    fields: ["init.session_id"]
    hook_parity: "hooks include session_id and transcript_path"
    notes: "Session ID appears early. Resume availability also depends on persisted project chat storage."
  - name: "subagent_prompt_injection"
    detectable: true
    event_types: ["tool_use"]
    fields: ["tool_use.tool_name", "tool_use.parameters"]
    hook_parity: "unknown"
    notes: "Caller can put non-interactive instructions in the top-level prompt and force subagents with @name syntax, but nested prompts are not separately exposed."
headless_constraints:
  - constraint: "stream-json has no formal JSON Schema or version marker"
    mitigation: "Generate parser fixtures from provider TypeScript union and tolerate unknown event types."
    notes: "The settings file has JSON Schema; the output stream does not."
  - constraint: "output.format setting documents only text and json"
    mitigation: "Always pass --output-format stream-json on the command line."
    notes: "Do not rely on persistent config to select stream-json."
  - constraint: "mutating tools can require confirmation"
    mitigation: "Use --approval-mode=plan for read-only runs or --approval-mode=yolo only inside external sandboxing; configure policy/trust deterministically."
    notes: "The stream omits elicitation events, so waiting for approval is hard to classify from stdout alone."
  - constraint: "fatal errors may not emit a terminal result event"
    mitigation: "Use result when present; otherwise classify stderr and process exit code."
    notes: "Source catches fatal errors and delegates to handleError after stream cleanup."
  - constraint: "file changes are not dedicated stream events"
    mitigation: "Infer file changes from tool_use parameters and tool_result output, or supplement with filesystem diffing."
    notes: "No file_change family in the JSONL union."
  - constraint: "tool progress and internal usage events are dropped"
    mitigation: "Render tool start on tool_use, completion on tool_result, and final usage on result.stats."
    notes: "Internal tool_update and usage AgentEvents are explicitly ignored."
quirks:
  - "The stream discriminator is top-level type, but role and status are nested subtypes for message/result/tool_result."
  - "assistant message chunks are complete JSONL records with content fragments and delta: true, not raw token deltas outside JSON."
  - "The user prompt is echoed as a message event; wrappers must avoid leaking sensitive prompt text in logs."
  - "stream-json emits session_id early but omits cwd, root, trust state, auth source, version, sandbox, approval mode, and MCP inventory."
  - "MCP tool names use mcp_<serverAlias>_<tool>; underscores in server aliases can confuse policy parsing."
  - "json final output can contain warnings in source even though the headless docs emphasize response/stats/error."
gaps:
  - "No official JSON Schema or schema version exists for stream-json."
  - "Could not verify from docs whether all fatal errors in stream-json mode emit a result status=error before process exit; source suggests some fatal paths rely on handleError."
  - "Non-TTY behavior for approval/elicitation requests is not fully documented; source shows elicitation events are ignored by stream-json."
  - "No structured auth source, quota reset time, billing balance, cost, sandbox mode, cwd, root, CLI version, or permission mode fields were found in stream-json."
  - "No local authenticated run was captured in this refresh, so examples are source-derived rather than live-run fixtures."
claudine_strategy:
  preferred_invocation: 'gemini --skip-trust --approval-mode=plan --output-format stream-json -p "<prompt>"'
  required_flags: ["--output-format stream-json", "-p/--prompt", "--skip-trust or GEMINI_CLI_TRUST_WORKSPACE=true when CI workspace trust is expected", "--approval-mode=plan for read-only automation or --approval-mode=yolo only inside an external sandbox"]
  conflicting_flags: ["--prompt-interactive/-i", "--experimental-acp for the normal wrapper stream", "persistent output.format when stream-json is required", "--yolo outside external sandboxing"]
  parser_notes: "Parse stdout as JSONL by top-level type. Join tool_use/tool_result by tool_id. Treat result as terminal when present; if absent, classify by exit code and stderr. Accumulate assistant message content where role=assistant and delta=true. Preserve unknown events for drift fixtures."
  wrapper_notes: "Keep stderr separate as diagnostics. Capture gemini --version separately if version metadata is required. Consider filesystem diffing for file changes and wrapper-side config capture for cwd, roots, sandbox, approval, auth kind, and MCP settings."
data_format: jsonl
changes:
  - "2026-07-02: Rewrote older Gemini research into the current non-interactive-sessions schema; refreshed official docs, TypeScript stream union, config files, and Claudine strategy."
requires_claudine_update: true
reason: "The refreshed metadata confirms Gemini should be driven with --output-format stream-json explicitly and parsed as a six-event JSONL union; Claudine provider metadata/parser fixtures should reflect missing terminal events on fatal errors and omitted internal events."
---

# Gemini CLI: Non-Interactive Sessions

## Summary

Gemini CLI can run non-interactively and can emit structured output. Claudine should prefer `gemini --output-format stream-json -p "<prompt>"` because it is the only documented Gemini CLI mode that streams parseable progress while the run is active. It emits one JSON object per stdout line with a top-level `type` discriminator and event types `init`, `message`, `tool_use`, `tool_result`, `error`, and `result`.

The main parser risk is not framing; the JSONL framing is simple. The risk is projection loss. The non-interactive implementation intentionally drops several internal agent events, including session updates, tool progress, usage increments, elicitation request/response, and subagent start events. Claudine can supervise session start, assistant text, tool start/result, non-fatal warnings, final usage, and final success when a `result` event is emitted, but it must use process exit and stderr as fallback evidence for fatal errors and possible approval/auth failures.

## Non-Interactive Entry Points

Headless mode is triggered by `-p` / `--prompt` or by non-TTY pipe/redirect conditions. The official automation tutorial recommends `-p` for headless scripts, and the CLI reference notes that positional query arguments can default to interactive mode in a TTY. For Claudine, that makes `-p` the deterministic launch form.

Useful launch forms:

| Command | Prompt input | Session behavior | Claudine use |
| --- | --- | --- | --- |
| `gemini --output-format stream-json -p "prompt"` | argv prompt plus optional piped stdin context | fresh session | Preferred |
| `cat diff.txt \| gemini --output-format stream-json -p "review"` | stdin context plus argv prompt | fresh session | Preferred for piped context |
| `gemini --resume latest --output-format stream-json -p "continue"` | argv prompt | resumes latest project session | Useful when Claudine intentionally resumes |
| `gemini --resume <SESSION_ID> --output-format stream-json -p "continue"` | argv prompt | resumes specific project session | Useful when Claudine has `init.session_id` |
| `gemini --experimental-acp` | protocol messages | long-running integration mode | Not the normal wrapper stream |

The prompt can include file references using Gemini CLI's `@` syntax, and the CLI can include additional workspace directories with `--include-directories`. Model selection is `--model`; trust and permission behavior are influenced by `--skip-trust`, `GEMINI_CLI_TRUST_WORKSPACE`, `--approval-mode`, `--sandbox`, settings files, and policy files.

Subagents are available in non-interactive prompts because Gemini exposes them as tools. A user can force a subagent with leading `@subagent_name` syntax, which instructs the main model to call that subagent tool. The stream still shows the subagent as a normal tool call/result, not as nested subagent lifecycle events.

## Output Formats

Gemini CLI exposes three non-interactive output formats through `--output-format`:

| Format | CLI value | Shape | Streams | Claudine recommendation |
| --- | --- | --- | --- | --- |
| Text | `text` | human text | Yes, but not structured | Avoid for wrappers |
| Final JSON | `json` | single JSON object | No | Useful for simple scripts, not lifecycle supervision |
| Streaming JSON | `stream-json` | JSONL / NDJSON | Yes | Preferred |

The `json` mode returns a final object. The TypeScript `JsonOutput` interface includes optional `session_id`, `response`, `stats`, `error`, and `warnings`. It is good for one-shot scripts that only need final answer text and statistics. It is weak for Claudine because it hides tool calls, warnings, and partial assistant output until the process exits.

The `stream-json` mode emits newline-delimited JSON on stdout. It is the right stream for Claudine because it provides session identity early, assistant output while generation is active, tool call starts before execution, tool results after execution, non-fatal warnings, and final aggregate usage. Stderr remains a diagnostics/user-feedback stream and should not be merged into the JSONL parser.

There is no separate stable hook stream that should replace stdout. Gemini CLI hooks are useful for policy and external side effects; their reference defines hook stdin/stdout JSON contracts, including `session_id`, `transcript_path`, `cwd`, `hook_event_name`, and `timestamp`. Those hook payloads are not the same as the headless stream. Claudine should parse stdout `stream-json` as primary lifecycle data and optionally use hooks/telemetry only as secondary evidence.

One important persistent-config gotcha: the command-line flag accepts `text`, `json`, and `stream-json`, but the documented `output.format` setting currently lists only `text` and `json`. Claudine should pass `--output-format stream-json` every run instead of relying on user or repo settings.

## Schema Sources

The strongest stream schema evidence is provider-authored TypeScript source, not a formal JSON Schema. The relevant files in the official repository are:

| Source | What it defines | Confidence |
| --- | --- | --- |
| [`packages/core/src/output/types.ts`](https://github.com/google-gemini/gemini-cli/blob/main/packages/core/src/output/types.ts) | `OutputFormat`, `JsonOutput`, `JsonStreamEventType`, `JsonStreamEvent`, `StreamStats` | High |
| [`packages/core/src/output/stream-json-formatter.ts`](https://github.com/google-gemini/gemini-cli/blob/main/packages/core/src/output/stream-json-formatter.ts) | JSONL framing and stats conversion | High |
| [`packages/core/src/output/json-formatter.ts`](https://github.com/google-gemini/gemini-cli/blob/main/packages/core/src/output/json-formatter.ts) | final JSON object formatting | High |
| [`packages/cli/src/nonInteractiveCliAgentSession.ts`](https://github.com/google-gemini/gemini-cli/blob/main/packages/cli/src/nonInteractiveCliAgentSession.ts) | projection from internal agent events to stdout | High |
| [Headless mode reference](https://geminicli.com/docs/cli/headless/) | public output mode/event summary and exit codes | Medium |
| [`schemas/settings.schema.json`](https://raw.githubusercontent.com/google-gemini/gemini-cli/main/schemas/settings.schema.json) | settings schema only | High for config, none for output |

The TypeScript union is strong enough for a Claudine parser because it is the code-level output contract used by the formatter. The residual risk is version drift: there is no explicit stream schema version marker, no published JSON Schema for the JSONL stream, and no documented unknown-event policy.

The stream union currently is:

| `type` | Fields |
| --- | --- |
| `init` | `timestamp`, `session_id`, `model` |
| `message` | `timestamp`, `role`, `content`, optional `delta` |
| `tool_use` | `timestamp`, `tool_name`, `tool_id`, `parameters` |
| `tool_result` | `timestamp`, `tool_id`, `status`, optional `output`, optional `error.type`, optional `error.message` |
| `error` | `timestamp`, `severity`, `message` |
| `result` | `timestamp`, `status`, optional `error`, optional `stats` |

`result.stats` contains `total_tokens`, `input_tokens`, `output_tokens`, `cached`, `input`, `duration_ms`, `tool_calls`, and `models`. Each `models.<model>` entry contains `total_tokens`, `input_tokens`, `output_tokens`, `cached`, and `input`.

## IO Contract

In `stream-json` mode, stdout is parse-only JSONL. The formatter writes `JSON.stringify(event) + "\n"` directly to stdout. Each line is independently parseable as a complete event.

Stderr is diagnostics and user feedback. The non-interactive source writes warning, cancellation, raw-output warning, and debug stack details to stderr. The `ConsolePatcher` also routes console logs into internal events/user feedback. Claudine should keep stderr separate, preserve it for diagnostics, and use it as fallback classification when a fatal error exits without a terminal `result` event.

Stdin is ordinary prompt/context input, not a bidirectional protocol, for the normal CLI run. Experimental ACP mode is the exception and should be treated as a separate integration surface rather than as `stream-json`.

Programmatic output is sanitized. The non-interactive code strips ANSI from assistant text and error messages for `json` and `stream-json` unless raw-output risk flags are involved for text mode. That reduces parser risk, but Claudine should still treat `message.content` as model-controlled text and avoid feeding it into terminal control paths without escaping.

## Stream Contract

The discriminator is top-level `type`. The stream starts with `init`, then echoes the user prompt as a `message` event with `role: "user"`, then streams assistant chunks as `message` events with `role: "assistant"` and `delta: true`. Tool calls use `tool_use`; results use `tool_result`; the join key is `tool_id`.

Normal success ends with `result` and `status: "success"`. The source emits that result when the internal agent loop reaches `agent_end` without an abort or configured max-turn fatal condition. For fatal paths, the implementation catches errors and delegates to `handleError` after cleanup; a parser should not assume that every failure includes `result.status: "error"`.

Timestamps are ISO-8601 strings from `new Date().toISOString()`, so they are UTC timestamps with millisecond precision when JavaScript includes it.

Internal events explicitly ignored by the non-interactive projection are:

| Internal event | Parser consequence |
| --- | --- |
| `initialize` | No richer session-start metadata than `init` |
| `session_update` | No cwd/project/root updates in the stream |
| `agent_start` | No subagent start/lifecycle records |
| `tool_update` | No structured tool progress |
| `elicitation_request` | No direct human-in-loop prompt event |
| `elicitation_response` | No direct approval/answer event |
| `usage` | No incremental token usage |
| `custom` | No extension/custom event projection |

Unknown top-level event types should be skipped and logged at trace/debug level with raw-event preservation for drift fixtures. Failing closed on unknown events would make Claudine brittle against Gemini CLI releases that add fields or event types.

## Session Metadata

`stream-json` exposes only two early session metadata fields: `init.session_id` and `init.model`. `session_id` is emitted before the agentic loop begins, so Claudine can use it for log correlation and possible resume links.

The stream does not emit cwd, project root, git branch, workspace roots, trust status, sandbox status, approval mode, auth kind, CLI version, provider identity, or configured MCP server inventory. Claudine should capture those as wrapper-side facts from invocation, environment, settings inspection, and optional `gemini --version`. MCP tools can be inferred after the fact from `tool_use.tool_name` names beginning with `mcp_`, but that is not equivalent to a complete server inventory.

Gemini session history is project-scoped and saved under `~/.gemini/tmp/<project_hash>/chats/` according to the session-management docs. That history includes conversation, tool executions, token usage, and reasoning summaries when available. The stream's `session_id` is useful, but actual resumability also depends on this persisted state.

## Event Families

The visible stream families are small but useful:

| Family | Event(s) | What Claudine can render |
| --- | --- | --- |
| Session | `init`, `result` | session ID, model, final status, aggregate usage |
| Assistant text | `message` | live assistant deltas and prompt echo |
| Tools | `tool_use`, `tool_result` | tool start, arguments, result/error, correlation by `tool_id` |
| Errors | `error`, sometimes `result.error` | warnings, resource exhaustion, general runtime errors |
| Usage | `result.stats` | final aggregate and per-model token counts |

There are no dedicated visible events for plans, file changes, permission decisions, terminal resize/status, subagent lifecycle, or rate-limit reset metadata.

## Tools

Gemini CLI's tools cover execution, filesystem, interaction, task tracking, MCP, memory, planning, system operations, and web. In the headless stream, all visible tool families are normalized into the same two events:

1. `tool_use` before execution, with `tool_name`, `tool_id`, and `parameters`.
2. `tool_result` after execution, with `tool_id`, `status`, `output`, and optional `error`.

The tool stream is live enough for Claudine to show "tool started" before completion. It is not rich enough to show structured progress because `tool_update` is ignored. File writes and edits are not separate `file_change` events; Claudine must infer them from tool names/parameters/results or perform its own filesystem diffing.

The shell tool documentation says the tool returns command, directory, stdout, stderr, exit code, and background PIDs. The JSONL projection does not preserve those as separate stable fields; it converts display content to a string `tool_result.output`. A parser should not rely on a structured `exit_code` path in `stream-json` unless future source adds one.

MCP tools are represented like native tools. The configuration docs say MCP-discovered tools are named with an `mcp_` prefix plus the server alias and tool name. The docs warn that underscores in server aliases can confuse policy parsing because the policy engine uses the first underscore after `mcp_`.

## Completion and Exit Status

For normal success, trust `result.status == "success"` and then process exit code `0`. The final answer text is the concatenation of assistant `message.content` chunks with `role: "assistant"`; in `json` mode it is `response`.

The official headless docs list these exit codes:

| Code | Meaning |
| --- | --- |
| `0` | Success |
| `1` | General error or API failure |
| `42` | Input error |
| `53` | Turn limit exceeded |

Because fatal paths are handled outside the normal `result` emission path, Claudine should use a layered completion rule:

1. If a `result` event appears, use it as the terminal stream record.
2. If stdout ends without `result`, classify by process exit code and stderr.
3. If exit code is non-zero and stderr/auth/error text is available, map known classes such as auth, input, sandbox, config, turn limit, cancellation, and tool execution.
4. If stdout ends without `result` but exit code is `0`, mark completion ambiguous and keep raw logs for fixture work.

Token usage is final-only. `result.stats` includes aggregate token counts and per-model token maps, but internal incremental `usage` events are ignored. There is no cost field.

## Blocking Behavior

Gemini CLI's default security posture can require user confirmation for mutating file tools and shell commands. The tools docs explicitly say users must manually approve mutators, and the CLI exposes `--approval-mode` plus the deprecated `--yolo` shortcut to change approval behavior.

For automation, Claudine should choose one of two deterministic postures:

| Posture | Suggested flags | Use case |
| --- | --- | --- |
| Read-only/planning | `--approval-mode=plan --output-format stream-json` | CI analysis, audits, reports |
| Mutating in external sandbox | `--approval-mode=yolo --sandbox --output-format stream-json` or externally stronger sandbox | autonomous repair jobs |

Workspace trust is a separate blocker. Use `--skip-trust` or `GEMINI_CLI_TRUST_WORKSPACE=true` only when the wrapper has decided the workspace is safe for the run.

The biggest gap is human-in-loop detection. The internal `elicitation_request` and `elicitation_response` events are explicitly ignored by `stream-json`. If a non-interactive run needs approval, the parser may see only silence, stderr diagnostics, a tool error, or a later process failure. Claudine should bound runs with its own silence timeout and classify missing terminal events carefully.

## Subagents

Gemini subagents are supported and documented as specialist agents with their own prompts, tools, and context loops. They are exposed to the main agent as tools. Users can allow automatic delegation or force a subagent by beginning the prompt with `@subagent_name`.

In `stream-json`, subagents are not a distinct event family. The main stream can show the parent `tool_use` for a subagent tool and the final `tool_result`, but not nested subagent start/stop events, nested tool calls, nested model identity, nested session IDs, or nested errors. Claudine can inject non-interactive instructions into the top-level prompt and into forced-subagent prompt text, but cannot verify from the parent stream that a nested subagent avoided interactive behavior except by reading the final subagent result/error.

## Use Case Detection

| Use case | Detectable | Evidence | Notes |
| --- | --- | --- | --- |
| `plan_cap_approaching` | No | none | No remaining quota/threshold/reset fields. |
| `plan_capped` | Partial | `error.message`, `result.error.message`, non-zero exit | `RESOURCE_EXHAUSTED` becomes severity `error`, but reset window is not structured. |
| `no_funds` | Partial | error/stderr text | No balance or billing fields. |
| `auth` | Partial | stderr, exit code, possible `result.error` | Source reconstructs `FatalAuthenticationError`; fatal errors may bypass `result`. |
| `permission_read_denied` | Partial | `tool_use.parameters` + `tool_result.error` | Join by `tool_id`; no normalized path/policy field. |
| `permission_write_denied` | Partial | `tool_use.parameters` + `tool_result.error` | Distinguish by tool family and error text/type. |
| `tokens_consumed` | Yes | `result.stats.*` | Final session aggregate and per-model counts only. |
| `model_used` | Yes | `init.model`, `result.stats.models` | `init.model` is early; `stats.models` is final. |
| `model_fallback` | Inferred | compare `init.model` to `result.stats.models` keys | No explicit fallback event. |
| `human_in_loop` | No | none in stdout | Elicitation events are intentionally ignored. |
| `session_resumable` | Yes | `init.session_id` | Also depends on persisted project chat storage. |
| `subagent_prompt_injection` | Partial | prompt text and subagent tool call | Parent stream does not show nested prompts. |

## Headless Constraints

The main automation constraints are:

- `stream-json` must be selected on the command line because persistent `output.format` does not document `stream-json`.
- Fatal errors can end the process without a terminal `result` event.
- Mutating tools can require approvals unless approval mode and trust are configured.
- The stream omits elicitation events, so human-in-loop blocking is not directly visible.
- The stream omits cwd, auth source, CLI version, sandbox, approval mode, and MCP inventory.
- Tool progress, incremental token usage, and file changes are not first-class stream events.
- Subagent internals are hidden behind generic parent tool events.

## Timeline

The current docs indicate:

- Headless mode docs list `json` and `stream-json`, with `stream-json` described as newline-delimited JSON events.
- The changelog mentions stream JSON output as a release feature for monitoring real-time agent progress.
- The current TypeScript source includes `warnings` on final `JsonOutput` and per-model token breakdowns under `result.stats.models`.
- The configuration reference still documents persistent `output.format` values as `text` and `json`, while the CLI flag supports `stream-json`.

## Quirks and Gaps

The most important quirk is that `stream-json` looks like an event bus but is actually a curated projection. It is useful for Claudine, but it is not enough to reconstruct every internal state transition. In particular, `usage`, `tool_update`, `session_update`, and elicitation events are dropped.

The second quirk is final failure handling. The public docs say exit codes are meaningful, and source shows fatal errors are reconstructed and handled after cleanup. Claudine should not require a final `result` event to classify failure.

Gaps that need fixture evidence:

- Exact stdout/stderr shape for missing auth, expired auth, and quota exhaustion in `stream-json`.
- Non-TTY behavior when a mutating tool needs approval and approval mode is default.
- Whether all configured max-turn exits consistently use exit code `53`.
- Whether `init.model` is always the requested/configured alias or sometimes a resolved backend name.
- Whether local telemetry/activity logs expose stable secondary lifecycle records worth parsing.

## Claudine Integration Notes

Recommended default for read-only automation:

```bash
gemini --skip-trust --approval-mode=plan --output-format stream-json -p "<prompt>"
```

Recommended default for mutating automation only inside a stronger external sandbox:

```bash
gemini --skip-trust --approval-mode=yolo --sandbox --output-format stream-json -p "<prompt>"
```

Parser notes:

- Parse stdout line-by-line as JSONL.
- Use `type` as the discriminator.
- Treat `init.session_id` as the earliest correlation ID.
- Accumulate final assistant text from `message` events where `role == "assistant"`.
- Join `tool_use` and `tool_result` by `tool_id`.
- Render `tool_use` as live tool start; render `tool_result` as completion.
- Use `result.stats` for final token usage.
- Use `result` as terminal when present, but fall back to exit code and stderr when missing.
- Preserve unknown event records for drift analysis.

Wrapper notes:

- Keep stderr separate. It is not structured JSONL, but it is important for fatal error classification.
- Capture wrapper-side metadata that Gemini omits: cwd, roots, trust, sandbox, approval mode, MCP configuration, auth source, and CLI version.
- Use Claudine timeouts to detect silence/hangs because the stream omits elicitation events.
- Consider filesystem diffing when file-change reporting matters.
- Do not use `--prompt-interactive` or positional prompt forms for deterministic wrapper runs.

## Changelog

- 2026-07-02: Rewrote the older Gemini non-interactive note into the current `_schema.yaml` frontmatter shape, refreshed against current official docs and source, and documented the preferred Claudine `stream-json` strategy.

## Sources

- [Gemini CLI headless mode reference](https://geminicli.com/docs/cli/headless/)
- [Gemini CLI automation tutorial](https://geminicli.com/docs/cli/tutorials/automation/)
- [Gemini CLI command reference](https://geminicli.com/docs/cli/cli-reference/)
- [Gemini CLI configuration reference](https://geminicli.com/docs/reference/configuration/)
- [Gemini CLI tools reference](https://geminicli.com/docs/reference/tools/)
- [Gemini CLI hooks reference](https://geminicli.com/docs/hooks/reference/)
- [Gemini CLI session management](https://geminicli.com/docs/cli/session-management/)
- [Gemini CLI subagents](https://geminicli.com/docs/core/subagents/)
- [Gemini CLI quotas and pricing](https://geminicli.com/docs/resources/quota-and-pricing/)
- [Output TypeScript types](https://github.com/google-gemini/gemini-cli/blob/main/packages/core/src/output/types.ts)
- [Stream JSON formatter source](https://github.com/google-gemini/gemini-cli/blob/main/packages/core/src/output/stream-json-formatter.ts)
- [JSON formatter source](https://github.com/google-gemini/gemini-cli/blob/main/packages/core/src/output/json-formatter.ts)
- [Non-interactive CLI agent session source](https://github.com/google-gemini/gemini-cli/blob/main/packages/cli/src/nonInteractiveCliAgentSession.ts)
