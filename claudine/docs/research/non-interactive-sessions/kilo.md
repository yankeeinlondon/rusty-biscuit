---
$schema: ./_schema.yaml
created: 2026-07-02
last_updated: 2026-07-03
agent: codex
model: default
docs: https://kilo.ai/docs/code-with-ai/platforms/cli#autonomous-mode-non-interactive
invocation:
  - command: "kilo run --auto --format json \"<prompt>\""
    stdin_support: true
    prompt_arg: "Prompt words are joined; non-TTY stdin is appended after a newline."
    notes: "Fresh non-interactive session. Preferred subprocess form for Claudine."
  - command: "kilo run --auto --format json -- \"<prompt beginning with dash>\""
    stdin_support: true
    prompt_arg: "Prompt words after -- are treated as positional message atoms."
    notes: "Use when the prompt begins with '-' or contains shell-looking leading options."
  - command: "kilo run --auto --format json --file <path> \"<prompt>\""
    stdin_support: true
    prompt_arg: "Prompt plus one or more file or directory attachments."
    notes: "Local paths resolve from --dir/current directory; attach mode resolves against the selected remote/server directory."
  - command: "kilo run --auto --format json --continue \"<prompt>\""
    stdin_support: true
    prompt_arg: "Same argv/stdin prompt behavior as a fresh run."
    notes: "Continues the latest top-level session."
  - command: "kilo run --auto --format json --session <session-id> \"<prompt>\""
    stdin_support: true
    prompt_arg: "Same argv/stdin prompt behavior as a fresh run."
    notes: "Continues a specific session ID."
  - command: "kilo run --auto --format json --session <session-id> --fork \"<prompt>\""
    stdin_support: true
    prompt_arg: "Same argv/stdin prompt behavior as a fresh run."
    notes: "Forks an existing local session, then continues the fork."
  - command: "kilo run --auto --format json --session <cloud-session-id> --cloud-fork \"<prompt>\""
    stdin_support: true
    prompt_arg: "Same argv/stdin prompt behavior as a fresh run."
    notes: "Imports a cloud session and continues it locally."
  - command: "kilo serve --port <port>; kilo run --auto --format json --attach http://127.0.0.1:<port> \"<prompt>\""
    stdin_support: true
    prompt_arg: "Prompt is sent to the long-running server through the SDK HTTP API."
    notes: "Requires server lifecycle and optional Basic Auth via --username/--password or KILO_SERVER_*."
  - command: "kilo acp --cwd <dir>"
    stdin_support: false
    prompt_arg: "ACP client protocol, not plain prompt stdin."
    notes: "Structured server mode for ACP clients; not the one-shot stdout stream Claudine should use first."
output_formats:
  - name: "formatted run output"
    cli_value: "default"
    stream: true
    format: text
    description: "Human-oriented formatted output; in non-TTY stdout the final assistant text is printed as text."
    side_effects: "Not parser-safe: status, warnings, ANSI/UI formatting, share URLs, and tool summaries can appear."
  - name: "raw JSON events"
    cli_value: "--format json"
    stream: true
    format: ndjson
    description: "One JSON object per stdout line from kilo run. Emitted types are tool_use, step_start, step_finish, text, reasoning, and error."
    side_effects: "Suppresses the human formatter for forwarded records, but there is no explicit terminal completion event."
  - name: "server SSE"
    cli_value: "kilo serve / SDK event.subscribe"
    stream: true
    format: sse
    description: "The local server exposes text/event-stream frames with event: message and JSON data payloads."
    side_effects: "Richer than run NDJSON but requires server lifecycle, directory routing, and auth handling."
  - name: "ACP server"
    cli_value: "kilo acp"
    stream: true
    format: other
    description: "Agent Client Protocol server mode."
    side_effects: "Bidirectional protocol integration; stdin/stdout are not a prompt/result stream."
schema_sources:
  - url: "https://kilo.ai/docs/code-with-ai/platforms/cli"
    schema_type: examples
    formal: false
    notes: "Official docs describe autonomous mode, exit codes, config files, and env overrides; they do not define the JSON event schema."
  - url: "https://github.com/Kilo-Org/kilocode/blob/419ff008ef180dd7076f679a89442883ba8f8d86/packages/opencode/src/cli/cmd/run.ts"
    schema_type: typescript
    formal: false
    notes: "Authoritative source for run flags, stdin merging, JSON emission, permission replies, and forwarded event names."
  - url: "https://github.com/Kilo-Org/kilocode/blob/419ff008ef180dd7076f679a89442883ba8f8d86/packages/opencode/test/cli/run/run-process.test.ts"
    schema_type: examples
    formal: false
    notes: "Subprocess tests assert parseable line-delimited JSON with type and sessionID and lock in mid-stream error exit behavior."
  - url: "https://github.com/Kilo-Org/kilocode/blob/419ff008ef180dd7076f679a89442883ba8f8d86/packages/opencode/src/session/message-v2.ts"
    schema_type: typescript
    formal: false
    notes: "Effect Schema source for part payloads, tool states, step usage/cost, assistant metadata, and error names."
  - url: "https://github.com/Kilo-Org/kilocode/blob/419ff008ef180dd7076f679a89442883ba8f8d86/packages/core/src/session-event.ts"
    schema_type: typescript
    formal: false
    notes: "Richer EventV2 union used by the server/SSE and SDK event stream; broader than kilo run NDJSON."
  - url: "https://github.com/Kilo-Org/kilocode/blob/419ff008ef180dd7076f679a89442883ba8f8d86/packages/opencode/src/server/routes/instance/httpapi/handlers/event.ts"
    schema_type: typescript
    formal: false
    notes: "SSE endpoint source; emits event: message frames whose data is JSON.stringify(event)."
cli_params:
  - flag: "--format"
    value: "default | json"
    description: "Selects formatted text or raw JSON event output for kilo run."
    example: "kilo run --auto --format json \"fix tests\""
  - flag: "--auto"
    value: ""
    description: "Autonomous/pipeline mode. The run loop auto-approves root and tracked task-child permission asks."
    example: "kilo run --auto --format json \"fix tests\""
  - flag: "--dangerously-skip-permissions"
    value: ""
    description: "Approves permissions that are not explicitly denied. Riskier than --auto with explicit permission config."
    example: "kilo run --dangerously-skip-permissions --format json \"fix tests\""
  - flag: "--interactive, -i"
    value: ""
    description: "Starts direct interactive split-footer mode; conflicts with --format json and requires TTY stdout."
    example: "kilo run --interactive"
  - flag: "--model, -m"
    value: "provider/model"
    description: "Requests a provider/model pair for the run."
    example: "kilo run -m anthropic/claude-sonnet-4-20250514 --auto --format json \"task\""
  - flag: "--variant"
    value: "provider-specific"
    description: "Provider-specific reasoning/model variant such as high, max, or minimal."
    example: "kilo run --variant high --auto --format json \"task\""
  - flag: "--thinking"
    value: ""
    description: "Includes completed reasoning parts in CLI output when present."
    example: "kilo run --thinking --auto --format json \"task\""
  - flag: "--agent"
    value: "name"
    description: "Selects a primary agent. Subagent names are rejected with a warning and fallback."
    example: "kilo run --agent code --auto --format json \"task\""
  - flag: "--command"
    value: "command"
    description: "Runs a built-in or slash command with the message as arguments."
    example: "kilo run --command review --auto --format json"
  - flag: "--file, -f"
    value: "path"
    description: "Attaches one or more local files or directories to the prompt."
    example: "kilo run -f README.md --auto --format json \"summarize\""
  - flag: "--continue, -c"
    value: ""
    description: "Continues the latest top-level session."
    example: "kilo run --continue --auto --format json \"continue\""
  - flag: "--session, -s"
    value: "session-id"
    description: "Continues a specific session."
    example: "kilo run --session ses_123 --auto --format json \"continue\""
  - flag: "--fork"
    value: ""
    description: "Forks the selected session before continuing; requires --continue or --session."
    example: "kilo run --session ses_123 --fork --auto --format json \"try another fix\""
  - flag: "--cloud-fork"
    value: ""
    description: "Imports a cloud session before continuing locally; used with --session."
    example: "kilo run --session cloud-id --cloud-fork --auto --format json \"continue\""
  - flag: "--dir"
    value: "path"
    description: "Runs in a local directory, or a remote server directory when --attach is used."
    example: "kilo run --dir packages/app --auto --format json \"task\""
  - flag: "--attach"
    value: "url"
    description: "Uses an existing Kilo server instead of an in-process server."
    example: "kilo run --attach http://127.0.0.1:4096 --auto --format json \"task\""
  - flag: "--username, -u"
    value: "name"
    description: "Basic auth username for --attach. Defaults to KILO_SERVER_USERNAME or kilo."
    example: "kilo run --attach http://127.0.0.1:4096 -u kilo --auto --format json \"task\""
  - flag: "--password, -p"
    value: "password"
    description: "Basic auth password for --attach. Defaults to KILO_SERVER_PASSWORD."
    example: "kilo run --attach http://127.0.0.1:4096 -p \"$KILO_SERVER_PASSWORD\" --auto --format json \"task\""
  - flag: "--print-logs"
    value: ""
    description: "Global flag that prints logs to stderr; useful for startup/auth diagnosis but not part of the JSON stream."
    example: "kilo --print-logs --log-level INFO run --auto --format json \"task\""
  - flag: "--log-level"
    value: "DEBUG | INFO | WARN | ERROR"
    description: "Global log verbosity flag."
    example: "kilo --print-logs --log-level DEBUG run --auto --format json \"task\""
  - flag: "--pure"
    value: ""
    description: "Global flag that disables external plugins for a more reproducible run."
    example: "kilo --pure run --auto --format json \"task\""
config_files:
  - os: macos
    scope: user
    path: "~/.config/kilo/kilo.jsonc"
    format: jsonc
    effect: "Global model, provider, permission, MCP, plugin, agent, and tool settings."
    notes: "Loaded after config.json and kilo.json; later project/env/managed scopes can override."
  - os: linux
    scope: user
    path: "~/.config/kilo/kilo.jsonc"
    format: jsonc
    effect: "Global model, provider, permission, MCP, plugin, agent, and tool settings."
    notes: "XDG_CONFIG_HOME can move this path; later project/env/managed scopes can override."
  - os: windows
    scope: user
    path: "%LOCALAPPDATA%\\kilo\\kilo.jsonc"
    format: jsonc
    effect: "Global model, provider, permission, MCP, plugin, agent, and tool settings."
    notes: "Kilo uses xdg-basedir; docs warn Windows config dir may vary."
  - os: macos
    scope: repo
    path: "./kilo.jsonc"
    format: jsonc
    effect: "Project config; can override global model, provider, permission, MCP, plugin, and agent settings."
    notes: "Discovered upward from cwd/worktree unless KILO_DISABLE_PROJECT_CONFIG is set."
  - os: linux
    scope: repo
    path: "./kilo.jsonc"
    format: jsonc
    effect: "Project config; can override global model, provider, permission, MCP, plugin, and agent settings."
    notes: "Discovered upward from cwd/worktree unless KILO_DISABLE_PROJECT_CONFIG is set."
  - os: windows
    scope: repo
    path: ".\\kilo.jsonc"
    format: jsonc
    effect: "Project config; can override global model, provider, permission, MCP, plugin, and agent settings."
    notes: "Discovered upward from cwd/worktree unless KILO_DISABLE_PROJECT_CONFIG is set."
  - os: macos
    scope: repo
    path: "./.kilo/kilo.jsonc"
    format: jsonc
    effect: "Project config directory; also supports agents, commands, plugins, and skills."
    notes: "Legacy .kilocode is also read; config directory files are merged after root project files."
  - os: linux
    scope: repo
    path: "./.kilo/kilo.jsonc"
    format: jsonc
    effect: "Project config directory; also supports agents, commands, plugins, and skills."
    notes: "Legacy .kilocode is also read; config directory files are merged after root project files."
  - os: windows
    scope: repo
    path: ".\\.kilo\\kilo.jsonc"
    format: jsonc
    effect: "Project config directory; also supports agents, commands, plugins, and skills."
    notes: "Legacy .kilocode is also read; config directory files are merged after root project files."
  - os: macos
    scope: managed
    path: "/Library/Application Support/kilo/kilo.jsonc"
    format: jsonc
    effect: "Enterprise/managed config loaded after user, project, env content, and cloud org config."
    notes: "macOS MDM preferences under /Library/Managed Preferences can override managed files."
  - os: linux
    scope: managed
    path: "/etc/kilo/kilo.jsonc"
    format: jsonc
    effect: "Enterprise/managed config loaded after user, project, env content, and cloud org config."
    notes: "No managed plist layer on Linux."
  - os: windows
    scope: managed
    path: "%ProgramData%\\kilo\\kilo.jsonc"
    format: jsonc
    effect: "Enterprise/managed config loaded after user, project, env content, and cloud org config."
    notes: "ProgramData defaults to C:\\ProgramData when unset."
env_vars:
  - name: "KILO_CONFIG"
    effect: "Loads an explicit config file into the effective config."
    notes: "Merged after global files and before project files."
  - name: "KILO_CONFIG_DIR"
    effect: "Overrides the global config directory and adds a config directory to the load chain."
    notes: "Also affects instruction discovery."
  - name: "KILO_CONFIG_CONTENT"
    effect: "Injects inline JSON/JSONC config content."
    notes: "Used by SDK/test harnesses; value is not exposed by config-source reporting."
  - name: "KILO_DISABLE_PROJECT_CONFIG"
    effect: "Disables project-level config files and directories."
    notes: "Useful for deterministic CI wrappers."
  - name: "KILO_PERMISSION"
    effect: "Runtime JSON overlay for permission rules."
    notes: "Invalid JSON is skipped with a warning."
  - name: "KILO_SERVER_USERNAME"
    effect: "Default Basic Auth username for attach/server flows."
    notes: "Defaults to kilo when unset."
  - name: "KILO_SERVER_PASSWORD"
    effect: "Default Basic Auth password for attach/server flows."
    notes: "Absence makes serve/web warn that the server is unsecured."
  - name: "KILO_API_KEY"
    effect: "Kilo Gateway API key/auth source and provider env override."
    notes: "Used by Kilo Gateway provider and account/session features."
  - name: "KILO_ORG_ID"
    effect: "Selects Kilo organization for non-interactive kilo run."
    notes: "Official docs list it as the highest-priority organization selector for kilo run."
  - name: "KILO_PURE"
    effect: "Disables external plugins."
    notes: "Equivalent to --pure after top-level parsing."
  - name: "KILO_DISABLE_DEFAULT_PLUGINS"
    effect: "Disables Kilo default plugin injection."
    notes: "Can reduce tool/provider surface drift in automation."
  - name: "KILO_ENABLE_QUESTION_TOOL"
    effect: "Enables the question tool for additional clients."
    notes: "Non-interactive run still installs question denial rules."
  - name: "KILO_EXPERIMENTAL_BACKGROUND_SUBAGENTS"
    effect: "Enables background subagents."
    notes: "Subagent tool calls can create child session permission behavior."
  - name: "KILO_EXPERIMENTAL_OUTPUT_TOKEN_MAX"
    effect: "Sets an experimental output token cap."
    notes: "Parser can only observe resulting model/session errors, not the configured value."
  - name: "KILO_DIRECT_TRACE"
    effect: "Writes direct interactive JSONL traces under the Kilo log directory."
    notes: "Direct interactive only; not the kilo run NDJSON stream."
  - name: "OTEL_EXPORTER_OTLP_ENDPOINT"
    effect: "Enables OpenTelemetry export of traces/logs."
    notes: "Secondary telemetry stream, not stdout."
io_contract:
  stdout: structured_only
  stderr: diagnostics_only
  stdin: prompt
  framing: ndjson
  noise_handling: "With --format json, parse stdout as one JSON object per line; keep stderr for diagnostics and startup failures."
  notes: "Formatted mode is mixed text/UI. The preferred --format json stream has no terminal completion record."
stream_contract:
  discriminator: "type"
  event_ordering: "Events are emitted in subscription order for the active session; session.status idle is consumed internally and not emitted."
  correlation_fields: ["sessionID", "part.id", "part.messageID", "part.callID"]
  terminal_event: "none"
  partial_message_events: false
  unknown_event_policy: "Skip unknown top-level types and log at trace; preserve unknown part fields."
  notes: "Top-level timestamp is Date.now() Unix milliseconds; part time fields are non-negative integer milliseconds from provider/session internals."
session_metadata:
  session_id: "sessionID on every emitted JSON record; available on the first emitted event, not as a separate start event."
  cwd: "Not emitted by run NDJSON; assistant message/SDK schema has path.cwd/path.root, and server config/path APIs can reveal cwd."
  model: "step_finish.part.model may contain providerID/modelID; assistant message and EventV2 step.started have richer model refs."
  provider: "step_finish.part.model.providerID when present; assistant message has providerID."
  auth: "Not emitted in run NDJSON; auth failures surface as ProviderAuthError or APIError records."
  version: "Not emitted in stream; use kilo --version out of band."
  mcp_servers: "Not emitted in run NDJSON; effective config controls MCP."
  permission_mode: "--auto/--dangerously-skip-permissions are not emitted; infer from invocation."
  notes: "Run NDJSON is operational but sparse. Server/SSE and exported sessions expose broader state."
stream_events:
  - event: "tool_use"
    category: tool_result
    fields: ["type", "timestamp", "sessionID", "part.type", "part.callID", "part.tool", "part.state.status", "part.state.input", "part.state.output", "part.state.error", "part.state.metadata", "part.state.attachments"]
    notes: "Only emitted when a tool part reaches completed or error status."
  - event: "step_start"
    category: assistant
    fields: ["type", "timestamp", "sessionID", "part.id", "part.messageID", "part.snapshot"]
    notes: "Start of an assistant step; no model field in run part."
  - event: "step_finish"
    category: usage
    fields: ["type", "timestamp", "sessionID", "part.reason", "part.model.providerID", "part.model.modelID", "part.cost", "part.tokens.input", "part.tokens.output", "part.tokens.reasoning", "part.tokens.cache.read", "part.tokens.cache.write", "part.tokens.total"]
    notes: "Best source for model, token, and cost totals in kilo run NDJSON."
  - event: "text"
    category: assistant
    fields: ["type", "timestamp", "sessionID", "part.id", "part.messageID", "part.text", "part.time.start", "part.time.end", "part.metadata"]
    notes: "Only completed text parts are emitted; deltas are not emitted."
  - event: "reasoning"
    category: reasoning
    fields: ["type", "timestamp", "sessionID", "part.id", "part.messageID", "part.text", "part.time.start", "part.time.end", "part.metadata"]
    notes: "Only emitted when --thinking is enabled and the reasoning part has ended."
  - event: "error"
    category: error
    fields: ["type", "timestamp", "sessionID", "error.name", "error.data", "error.message"]
    notes: "Emitted for session.error and immediate SDK/builtin command errors."
  - event: "session.next.text.delta"
    category: assistant
    fields: ["type", "timestamp", "sessionID", "delta"]
    notes: "Server/SSE EventV2 only; not forwarded by kilo run NDJSON."
  - event: "session.next.tool.called"
    category: tool_call
    fields: ["type", "timestamp", "sessionID", "callID", "tool", "input", "provider.executed", "provider.metadata"]
    notes: "Server/SSE EventV2 only; richer than run tool_use."
  - event: "session.next.tool.success"
    category: tool_result
    fields: ["type", "timestamp", "sessionID", "callID", "structured", "content", "provider.executed", "provider.metadata"]
    notes: "Server/SSE EventV2 only."
  - event: "server.connected"
    category: session
    fields: ["id", "type", "properties"]
    notes: "SSE-only first event from the server event endpoint."
  - event: "server.heartbeat"
    category: session
    fields: ["id", "type", "properties"]
    notes: "SSE-only heartbeat every 10 seconds after the initial delay."
tools:
  - name: "shell/bash/process"
    call_visible: false
    result_visible: true
    metadata: ["part.tool", "part.callID", "part.state.input", "part.state.output", "part.state.error", "part.state.metadata", "part.state.time"]
    notes: "Run NDJSON emits completed/error tool_use only; server EventV2 exposes called/progress/success/failed."
  - name: "read/write/edit/apply_patch/glob/grep"
    call_visible: false
    result_visible: true
    metadata: ["part.state.input", "part.state.output", "part.state.error", "part.state.attachments"]
    notes: "File changes are visible through tool payloads or patch/snapshot parts, not a dedicated run-level file_change event."
  - name: "task/subagent"
    call_visible: false
    result_visible: true
    metadata: ["part.tool", "part.state.metadata.sessionId", "part.state.output", "part.state.error"]
    notes: "Kilo tracks task child session IDs for permission replies; nested child events are not forwarded in the parent run NDJSON."
  - name: "question"
    call_visible: false
    result_visible: true
    metadata: ["part.state.error", "part.state.output"]
    notes: "Non-interactive run installs question denial rules; docs describe autonomous responses, but current source also denies question permission."
  - name: "webfetch/websearch/mcp"
    call_visible: false
    result_visible: true
    metadata: ["part.tool", "part.state.input", "part.state.output", "part.state.error", "part.state.metadata"]
    notes: "MCP tools use the same tool part envelope in run NDJSON."
completion:
  success_event: "none"
  failure_event: "error"
  exit_code_reliable: false
  result_fields: ["text.part.text", "error.error", "step_finish.part.reason"]
  cost_fields: ["step_finish.part.cost"]
  usage_fields: ["step_finish.part.tokens.input", "step_finish.part.tokens.output", "step_finish.part.tokens.reasoning", "step_finish.part.tokens.cache.read", "step_finish.part.tokens.cache.write", "step_finish.part.tokens.total"]
  notes: "Process exit 0 is reliable for happy-path startup success but mid-stream LLM errors currently emit session.error and still exit 0. Treat error events as failure even if exit code is 0."
blocking_behavior:
  permissions: configurable
  questions: configurable
  tool_approvals: configurable
  notes: "--auto replies once to permissions for root and tracked task child sessions. Without --auto/--dangerously-skip-permissions, root permissions are auto-rejected and headless child asks fail instead of hanging. Questions are denied by non-interactive permission rules despite public docs describing an autonomous response."
subagents:
  supported: true
  start_visible: false
  stop_visible: false
  nested_events_visible: false
  prompt_injection_supported: true
  metadata_fields: ["tool_use.part.state.metadata.sessionId", "tool_use.part.state.input", "tool_use.part.state.output", "tool_use.part.state.error"]
  notes: "Task/subagent tool results are visible after completion. Parent run NDJSON does not stream child session tool calls."
use_cases:
  - name: plan_cap_approaching
    detectable: false
    event_types: []
    fields: []
    hook_parity: "unknown"
    notes: "No explicit near-cap event in run NDJSON."
  - name: plan_capped
    detectable: true
    event_types: ["error", "step_finish"]
    fields: ["error.name", "error.data.message", "error.data.statusCode", "step_finish.part.reason"]
    hook_parity: "unknown"
    notes: "Quota/billing failures must be classified from provider error names/messages/status, not a normalized quota event."
  - name: no_funds
    detectable: true
    event_types: ["error"]
    fields: ["error.name", "error.data.message", "error.data.statusCode", "error.data.responseBody"]
    hook_parity: "unknown"
    notes: "Provider error mapping includes insufficient_quota messaging; exact no-funds classification is provider-dependent."
  - name: auth
    detectable: true
    event_types: ["error"]
    fields: ["error.name", "error.data.message"]
    hook_parity: "unknown"
    notes: "LoadAPIKeyError and expired auth map to ProviderAuthError."
  - name: permission_read_denied
    detectable: true
    event_types: ["tool_use", "error"]
    fields: ["part.tool", "part.state.status", "part.state.input", "part.state.error", "error.name", "error.data"]
    hook_parity: "unknown"
    notes: "Classify read-shaped tools from tool input/error; no dedicated permission_denied event is forwarded."
  - name: permission_write_denied
    detectable: true
    event_types: ["tool_use", "error"]
    fields: ["part.tool", "part.state.status", "part.state.input", "part.state.error", "error.name", "error.data"]
    hook_parity: "unknown"
    notes: "Classify edit/write/apply_patch/shell failures from tool payloads; no dedicated write-denied event is forwarded."
  - name: tokens_consumed
    detectable: true
    event_types: ["step_finish"]
    fields: ["part.tokens.input", "part.tokens.output", "part.tokens.reasoning", "part.tokens.cache.read", "part.tokens.cache.write", "part.tokens.total"]
    hook_parity: "unknown"
    notes: "Per-step values; session totals require summing step_finish events."
  - name: model_used
    detectable: true
    event_types: ["step_finish"]
    fields: ["part.model.providerID", "part.model.modelID"]
    hook_parity: "unknown"
    notes: "Model may be absent on some step_finish records; CLI invocation/config are fallback evidence."
  - name: model_fallback
    detectable: false
    event_types: []
    fields: []
    hook_parity: "unknown"
    notes: "No explicit fallback event in run NDJSON."
  - name: human_in_loop
    detectable: true
    event_types: ["tool_use", "error"]
    fields: ["part.tool", "part.state.error", "error.name", "error.data.message"]
    hook_parity: "unknown"
    notes: "Questions/permission asks are not forwarded as asks; infer from denied question/permission tool errors."
  - name: session_resumable
    detectable: true
    event_types: ["tool_use", "step_start", "step_finish", "text", "reasoning", "error"]
    fields: ["sessionID"]
    hook_parity: "unknown"
    notes: "Every emitted NDJSON line has sessionID, but there is no early session_start record."
  - name: subagent_prompt_injection
    detectable: true
    event_types: ["tool_use"]
    fields: ["part.tool", "part.state.input", "part.state.metadata.sessionId"]
    hook_parity: "unknown"
    notes: "The caller can steer subagents through the root prompt; Kilo exposes selected task input/result after completion."
headless_constraints:
  - constraint: "No terminal complete event in run NDJSON."
    mitigation: "Treat process exit plus absence/presence of error events as completion; accumulate final text events."
    notes: "Internal session.status idle breaks the loop but is not emitted."
  - constraint: "Mid-stream LLM errors currently exit 0."
    mitigation: "Classify any error event as run failure even when exit code is 0."
    notes: "Locked by subprocess test on current main."
  - constraint: "--interactive conflicts with --format json and requires TTY stdout."
    mitigation: "Never use --interactive for Claudine subprocess automation."
    notes: "The run handler exits 1 for this combination."
  - constraint: "Run NDJSON omits tool start/progress and assistant deltas."
    mitigation: "Use server/SSE only if Claudine needs richer live operational detail and can manage a server."
    notes: "Subprocess NDJSON is simpler and more stable for first integration."
  - constraint: "Question behavior differs between docs and current run source."
    mitigation: "Avoid tasks that require human clarification; include non-interactive instructions in the prompt."
    notes: "Docs describe autonomous follow-up responses; source installs question denial rules."
quirks:
  - "Kilo CLI is a Kilo-branded fork of OpenCode; source paths often live under packages/opencode and tests still say opencode."
  - "The preferred stream is line-delimited JSON but has no schema version marker."
  - "The top-level run event type names use snake_case, while server EventV2 names are dotted strings such as session.next.tool.success."
  - "tool_use is a result event, not a call-start event."
  - "Permissions are operationally handled by the run loop but permission.asked is not emitted to --format json stdout."
  - "Configured output format is a CLI flag; no evidence that config files persistently select run --format json."
gaps:
  - "No formal JSON Schema/OpenAPI definition was found for kilo run --format json."
  - "Could not verify a real installed kilo binary run with live credentials in this workspace."
  - "Exact stdout/stderr behavior for all startup/auth/config failures is source-inferred, not fixture-captured here."
  - "Exact provider-specific quota/no-funds payloads vary by backend and were not exhaustively captured."
  - "ACP schema and behavior were not researched deeply because kilo run NDJSON is the recommended first integration surface."
claudine_strategy:
  preferred_invocation: "kilo run --auto --format json --dir <cwd> \"<prompt>\""
  required_flags: ["run", "--auto", "--format json"]
  conflicting_flags: ["--interactive", "--replay", "--replay-limit", "--demo"]
  parser_notes: "Parse stdout as NDJSON with top-level type. Join tool results by part.callID, session records by sessionID, and assistant text by part.messageID/part.id. Treat error events as failure even if exit code is 0. Unknown types should be skipped and logged."
  wrapper_notes: "Keep stderr for diagnostics. Use --pure and KILO_DISABLE_PROJECT_CONFIG for deterministic runs when desired. Consider server/SSE later for richer progress, but it adds lifecycle/auth complexity."
data_format: ndjson
changes:
  - "2026-07-03: Refreshed Kilo non-interactive research against upstream main 419ff008ef180dd7076f679a89442883ba8f8d86; updated preferred NDJSON strategy, current headless permission behavior, config scopes, and exit-code caveats."
requires_claudine_update: true
reason: "Kilo is not yet a compiled Claudine provider; supporting it would require provider metadata and an NDJSON parser for the run stream."
---

# Kilo Code Non-Interactive Sessions

## Summary

Kilo Code can run non-interactively through `kilo run`. For Claudine, the best first integration surface is `kilo run --auto --format json`, which emits newline-delimited JSON records on stdout while the session is active. The stream is operationally useful but intentionally narrow: it includes completed text, optional completed reasoning, step start/finish records, completed or errored tool parts, and errors. It does not emit a session-start event, a terminal success event, tool-call-start records, tool progress, or assistant text deltas.

The main parser risk is that the stream shape is source-defined, not formally schema-defined. Kilo also has a richer server/SSE event stream, but that requires managing `kilo serve`, server auth, directory routing, and a different SDK-shaped event union. Claudine should start with the subprocess NDJSON stream, keep stderr as diagnostics, and classify any `error` event as a failed run even if the process exits `0`.

## Non-Interactive Entry Points

The official CLI docs describe autonomous mode as a CI/CD-oriented mode started with `kilo run --auto "<message>"`. The same docs state that autonomous mode avoids user interaction, handles approvals automatically according to configuration, and exits when the task completes or times out. The command reference lists `kilo run [message..]` and includes `--format`, `--auto`, `--dangerously-skip-permissions`, `--model`, `--agent`, `--file`, `--dir`, `--attach`, `--continue`, `--session`, `--fork`, `--cloud-fork`, and `--thinking`.

Prompt input can come from argv or non-TTY stdin. In current source, `buildRunMessage(args.message, args["--"])` builds the argv prompt, then `loadInput()` reads `Bun.stdin.text()` when stdin is not a TTY and appends it after a newline. If neither a prompt nor a command is provided, the process exits with an error.

Kilo also exposes server and protocol entry points:

| Entry point | Shape | Claudine fit |
| --- | --- | --- |
| `kilo run --auto --format json "<prompt>"` | One-shot subprocess, NDJSON stdout | Best first integration |
| `kilo run --attach <url> --auto --format json "<prompt>"` | Subprocess talking to an existing server | Useful when Claudine manages a reusable server |
| `kilo serve` plus SDK event subscription | Long-running HTTP server with SSE | Richer events, higher lifecycle cost |
| `kilo acp` | Agent Client Protocol server | Separate protocol integration, not a stdout result stream |

Use `--dir` to select a local working directory. Use `--file` for file/directory attachments. Use `--model provider/model`, `--variant`, and `--agent` to steer model/agent selection. If `--agent` names a subagent, the run command warns and falls back to the default primary agent.

## Output Formats

`kilo run` has two direct output formats:

| Format | Selector | Streaming | Parser value | Notes |
| --- | --- | --- | --- | --- |
| Formatted text | default | Yes | Low | Human/UI output, not safe for machine parsing. |
| Raw JSON events | `--format json` | Yes | High | One JSON object per stdout line, source-tested as parseable. |

The preferred format is `--format json`. The source `emit()` helper writes:

```json
{"type":"text","timestamp":1760000000000,"sessionID":"ses_...","part":{}}
```

The concrete top-level event names forwarded by the run command are `tool_use`, `step_start`, `step_finish`, `text`, `reasoning`, and `error`. `reasoning` is only emitted when `--thinking` is enabled and a reasoning part has ended. `tool_use` is only emitted when a tool part is completed or errored; it is not a call-start event.

The server/SSE stream is richer. The server event handler emits `text/event-stream` with `event: message` and `data: JSON.stringify(event)`. It starts with `server.connected`, emits `server.heartbeat`, and forwards broader bus/SDK events. The source EventV2 union includes deltas and tool lifecycle records such as `session.next.text.delta`, `session.next.tool.called`, `session.next.tool.progress`, `session.next.tool.success`, and `session.next.tool.failed`. This is better observability but worse subprocess ergonomics.

## Schema Sources

There is no formal JSON Schema for `kilo run --format json`. The best schema evidence is the current TypeScript source at commit `419ff008ef180dd7076f679a89442883ba8f8d86`:

| Source | What it proves | Confidence |
| --- | --- | --- |
| [`run.ts`](https://github.com/Kilo-Org/kilocode/blob/419ff008ef180dd7076f679a89442883ba8f8d86/packages/opencode/src/cli/cmd/run.ts) | Flags, stdin behavior, JSON emitter, event names, permission replies | High |
| [`run-process.test.ts`](https://github.com/Kilo-Org/kilocode/blob/419ff008ef180dd7076f679a89442883ba8f8d86/packages/opencode/test/cli/run/run-process.test.ts) | Parseable NDJSON, `type`/`sessionID`, current exit-code behavior | High |
| [`message-v2.ts`](https://github.com/Kilo-Org/kilocode/blob/419ff008ef180dd7076f679a89442883ba8f8d86/packages/opencode/src/session/message-v2.ts) | `part` payload schemas, tool states, tokens, cost, assistant errors | High |
| [`session-event.ts`](https://github.com/Kilo-Org/kilocode/blob/419ff008ef180dd7076f679a89442883ba8f8d86/packages/core/src/session-event.ts) | Richer server/EventV2 stream union | Medium for CLI, high for SSE |
| [`event.ts`](https://github.com/Kilo-Org/kilocode/blob/419ff008ef180dd7076f679a89442883ba8f8d86/packages/opencode/src/cli/cmd/run/event.ts) | SDK sync-event normalization to message events | High |

The official docs are authoritative for supported commands, autonomous mode, config file locations, and documented exit codes, but not for JSON stream fields.

## IO Contract

With `--format json`, stdout should be treated as parse-only NDJSON. Each line is independently parseable JSON. The subprocess test explicitly parses stdout lines and asserts that every event has a string `type` and string `sessionID`.

Stderr is not part of the structured stream. It can contain logs, warnings, and startup/auth/config diagnostics, especially when `--print-logs` is used. Claudine should retain stderr and show it on startup failure or when the JSON stream ends without enough context.

Stdin is prompt text, not a bidirectional protocol. If stdin is not a TTY, Kilo reads all of it and appends it to the argv prompt. The ACP mode is the separate bidirectional/protocol surface.

## Stream Contract

The top-level discriminator is `type`. All emitted run records also include `timestamp` and `sessionID`. `timestamp` is produced with `Date.now()`, so it is Unix time in milliseconds. The nested `part` object uses the Effect Schema part union from `message-v2.ts`; nested discriminators are `part.type` and, for tools, `part.state.status`.

Run NDJSON emits completed snapshots rather than deltas. The richer server/EventV2 stream has deltas, but the subprocess stream waits for `part.time.end` before emitting `text` or `reasoning`, and waits for `completed` or `error` before emitting `tool_use`.

There is no terminal success event. The run loop consumes `session.status` internally and breaks when status becomes `idle`, but it does not forward that event to stdout. Completion is therefore inferred from process exit plus accumulated stream state. Unknown event types should be skipped and logged; the format has no schema version marker.

## Session Metadata

Every emitted JSON record contains `sessionID`, so a resumable session ID is available as soon as the first event arrives. There is no separate `session_start` record, and if a run fails before emitting any event, Claudine may not receive a session ID.

Model metadata is best read from `step_finish.part.model.providerID` and `step_finish.part.model.modelID` when present. The assistant message schema has `providerID`, `modelID`, `agent`, `path.cwd`, `path.root`, `cost`, `tokens`, and `finish`, but the run stream forwards only selected part records, not the full assistant message. Cwd/root, effective config, MCP servers, auth source, and CLI version are not emitted by run NDJSON. Use `kilo --version`, config inspection, or server/SDK APIs out of band if Claudine needs those fields.

## Event Families

Run NDJSON has these event families:

| Event | Category | Live or completed | Key fields |
| --- | --- | --- | --- |
| `step_start` | Assistant step | Live start | `part.id`, `part.messageID`, `part.snapshot` |
| `text` | Assistant output | Completed block | `part.text`, `part.time.start`, `part.time.end` |
| `reasoning` | Reasoning | Completed block, gated by `--thinking` | `part.text`, `part.time.*` |
| `tool_use` | Tool result | Completed/error only | `part.callID`, `part.tool`, `part.state.*` |
| `step_finish` | Usage/completion | Completed step | `part.reason`, `part.model`, `part.cost`, `part.tokens` |
| `error` | Failure | Immediate when observed | `error.name`, `error.data` |

The server/SSE family is broader and includes `session.next.*` events for agent/model switches, prompts, shell start/end, step start/end/failure, text/reasoning deltas, tool input deltas, tool called/progress/success/failed, retries, and compaction.

## Tools

Built-in tools include shell/process, read, write, edit, apply_patch, glob, grep, diagnostics/LSP, webfetch, websearch, MCP tools, task/subagent, skill, todo, plan enter/exit, question, repository tools, and Kilo-specific tools such as semantic search, background process, notebook, and document extractors.

In `kilo run --format json`, tool calls are visible only after completion or error through `tool_use`. The payload carries the stored tool part:

| Field | Meaning |
| --- | --- |
| `part.callID` | Tool-call correlation ID |
| `part.tool` | Tool name |
| `part.state.status` | `completed` or `error` in emitted records |
| `part.state.input` | Tool input object |
| `part.state.output` | Completed output string |
| `part.state.error` | Error string |
| `part.state.metadata` | Tool-specific metadata |
| `part.state.attachments` | Optional file attachments |

For command execution, stdout/stderr/exit-code details are tool-specific and appear inside tool output or metadata rather than a normalized command event. File changes are not normalized as `file_change` events in run NDJSON. They can be inferred from edit/write/apply_patch tool payloads, patch parts in the underlying message schema, snapshots, or exported sessions.

## Completion and Exit Status

Official docs list exit codes `0` for success, `124` for timeout, and `1` for initialization or execution failure. Current tests add an important caveat: a mid-stream LLM/provider error emits a `session.error` event but currently exits `0`. The test comments call this a locked-in current behavior and warn that changing it should be deliberate.

Claudine should therefore treat process exit code as advisory. A robust classifier should:

1. Parse all stdout NDJSON events.
2. Mark the run failed if any top-level `error` event appears.
3. Treat non-zero exit as failed even if no JSON error appears.
4. Treat exit `0` with no error events as success, using the accumulated `text` events as final answer.
5. Sum `step_finish.part.tokens.*` and `step_finish.part.cost` for session totals.

## Blocking Behavior

Kilo has three relevant permission modes. With `--auto`, the run loop replies `once` to permission asks for the root session and tracked `task` child sessions. With `--dangerously-skip-permissions`, permission asks that are not explicitly denied are also approved. Without either flag, non-interactive runs auto-reject permission requests, and current source marks plain headless root sessions so subagent permission asks fail instead of waiting forever.

At session creation, non-interactive runs also install deny rules for `question`, `interactive_terminal`, `plan_enter`, and `plan_exit`. That makes automation safer but creates a documentation tension: the public docs state that autonomous follow-up questions receive an instruction telling the AI to decide autonomously. The current source also denies the `question` permission, so Claudine should not rely on a programmable question-answer path in `kilo run`.

Network retry prompts are handled internally: `session.network.asked` is retried with exponential delay up to three times, then rejected. These retry prompts are not emitted to `--format json` stdout.

## Subagents

Subagents can run through the task tool. Parent run NDJSON exposes the task tool result after completion or error, and Kilo tracks child session IDs from task tool metadata so `--auto` can reply to child-session permission asks. Current source also handles tracked child permission requests in non-auto headless modes so they are rejected rather than hanging.

Nested subagent events are not streamed into the parent NDJSON. There are no parent-stream `subagent_start` or `subagent_stop` records. Claudine can inject non-interactive instructions through the root prompt, but it cannot directly observe child tool calls unless it uses a richer server/session surface or inspects exported session data.

## Use Case Detection

| Use case | Detectable from run NDJSON | Evidence |
| --- | --- | --- |
| Plan cap approaching | No | No near-cap event found. |
| Plan capped/quota | Partially | Classify `error.name`, `error.data.message`, status, and provider response text. |
| No funds | Partially | Provider-specific `APIError`/quota messages; no normalized no-funds event. |
| Auth failure | Yes | `ProviderAuthError`, `APIError`, or startup stderr/exit. |
| Permission read/write denied | Partially | Tool error payloads and error records; no forwarded `permission.asked/replied`. |
| Tokens consumed | Yes | `step_finish.part.tokens.*`, per step. |
| Cost | Yes | `step_finish.part.cost`, per step. |
| Model used | Yes | `step_finish.part.model.*` when present. |
| Model fallback | No | No explicit fallback event found. |
| Human in loop | Partially | Infer from question/permission tool errors; asks are not directly emitted. |
| Session resumable | Yes | `sessionID` on every emitted event. |
| Subagent prompt injection | Yes | Prompt-level only; result visible as task tool payload. |

## Headless Constraints

The biggest integration constraint is missing terminal state in the preferred stream. Claudine cannot wait for a `session.complete` event because none is emitted. It must combine stream parsing with process lifecycle.

The second constraint is event sparsity. `tool_use` is not a start event, `text` is not a delta event, and permission asks are not forwarded. For clean terminal status, Claudine can show step starts, completed tool summaries, completed text blocks, and usage updates, but it cannot render precise live tool progress from the subprocess stream.

The third constraint is configuration drift. Kilo merges global config, explicit config file/content, project config, config directories, cloud organization config, managed files, managed preferences, and runtime env overlays. For deterministic CI, Claudine should consider setting `KILO_DISABLE_PROJECT_CONFIG=1`, using `--pure`, and passing a known `KILO_CONFIG_CONTENT` or `KILO_CONFIG` when appropriate.

## Timeline

| Date | Finding |
| --- | --- |
| 2026-07-03 | Verified upstream main at `419ff008ef180dd7076f679a89442883ba8f8d86`. |
| 2026-07-03 | Confirmed `kilo run --format json` emits NDJSON with `type`, `timestamp`, `sessionID`, and event-specific payload. |
| 2026-07-03 | Confirmed current tests lock in mid-stream LLM errors exiting `0`. |
| 2026-07-03 | Confirmed current headless handling rejects plain non-interactive subagent permission asks instead of hanging. |

## Quirks and Gaps

Kilo is Kilo-branded, but many source paths, package names, and tests still use `opencode`. Parser work should not assume package names imply OpenCode behavior, but source citations often live under `packages/opencode`.

The JSON stream has no formal schema or version marker. It is source-defined by `run.ts` and the message part Effect schemas. If Claudine implements a parser, it should be tolerant of additional top-level event types and additional nested fields.

The public docs say autonomous follow-up questions receive an autonomous-decision response, while current source denies `question` permission in non-interactive runs. This needs fixture verification against a real model/tool call before Claudine depends on either behavior.

This research did not run a live Kilo session with real credentials, did not capture provider-specific billing/quota examples, and did not deeply research ACP.

## Claudine Integration Notes

Use:

```bash
kilo run --auto --format json --dir "$PWD" "<prompt>"
```

Parse stdout as NDJSON. Keep stderr for diagnostics only. Use top-level `type` as the discriminator. Join tool data by `part.callID`; join assistant blocks by `part.messageID` and `part.id`; group everything by `sessionID`. Sum `step_finish.part.tokens` and `step_finish.part.cost` for session usage. Treat any `error` event as failure regardless of process exit code.

Avoid `--interactive`, `--replay`, `--replay-limit`, and `--demo` in automation. Add `--thinking` only if Claudine wants completed reasoning blocks and is prepared to store/render them. Add `--pure` and/or `KILO_DISABLE_PROJECT_CONFIG=1` when wrapper reproducibility matters more than repo-local customization.

Do not implement the server/SSE integration first unless Claudine needs live deltas, tool call starts, tool progress, or richer status. SSE is better observability but adds a long-running process, auth, and SDK event-shape complexity.

## Changelog

- 2026-07-03: Refreshed against upstream main `419ff008ef180dd7076f679a89442883ba8f8d86`; updated the recommended NDJSON invocation, source-defined schema evidence, headless permission behavior, config/env scopes, and mid-stream error exit-code caveat.

## Sources

- [Kilo Code CLI docs](https://kilo.ai/docs/code-with-ai/platforms/cli)
- [Kilo CLI command reference](https://kilo.ai/docs/code-with-ai/platforms/cli-reference)
- [`packages/opencode/src/cli/cmd/run.ts`](https://github.com/Kilo-Org/kilocode/blob/419ff008ef180dd7076f679a89442883ba8f8d86/packages/opencode/src/cli/cmd/run.ts)
- [`packages/opencode/test/cli/run/run-process.test.ts`](https://github.com/Kilo-Org/kilocode/blob/419ff008ef180dd7076f679a89442883ba8f8d86/packages/opencode/test/cli/run/run-process.test.ts)
- [`packages/opencode/src/session/message-v2.ts`](https://github.com/Kilo-Org/kilocode/blob/419ff008ef180dd7076f679a89442883ba8f8d86/packages/opencode/src/session/message-v2.ts)
- [`packages/core/src/session-event.ts`](https://github.com/Kilo-Org/kilocode/blob/419ff008ef180dd7076f679a89442883ba8f8d86/packages/core/src/session-event.ts)
- [`packages/opencode/src/server/routes/instance/httpapi/handlers/event.ts`](https://github.com/Kilo-Org/kilocode/blob/419ff008ef180dd7076f679a89442883ba8f8d86/packages/opencode/src/server/routes/instance/httpapi/handlers/event.ts)
- [`packages/opencode/src/kilocode/permission/headless.ts`](https://github.com/Kilo-Org/kilocode/blob/419ff008ef180dd7076f679a89442883ba8f8d86/packages/opencode/src/kilocode/permission/headless.ts)
- [`packages/opencode/src/kilocode/cli/run-auto.ts`](https://github.com/Kilo-Org/kilocode/blob/419ff008ef180dd7076f679a89442883ba8f8d86/packages/opencode/src/kilocode/cli/run-auto.ts)
