---
$schema: ./_schema.yaml
created: 2026-04-06
last_updated: 2026-07-02
agent: codex
model: default
docs: https://developers.openai.com/codex/noninteractive
invocation:
  - command: 'codex exec --json "prompt"'
    stdin_support: true
    prompt_arg: "PROMPT argument, omitted prompt, or '-' for stdin; if stdin is piped and PROMPT is also present, stdin is appended as a context block"
    notes: "Starts a fresh non-interactive session and emits JSONL events on stdout."
  - command: 'codex exec --json resume --last "prompt"'
    stdin_support: true
    prompt_arg: "Optional PROMPT argument or '-' for stdin"
    notes: "Resumes the newest recorded non-interactive session; requires a persisted session, so do not combine with the original run's --ephemeral."
  - command: 'codex exec --json resume <SESSION_ID> "prompt"'
    stdin_support: true
    prompt_arg: "Optional PROMPT argument or '-' for stdin"
    notes: "Resumes a specific session or thread name and emits the same exec JSONL stream."
  - command: 'codex app-server --listen stdio://'
    stdin_support: true
    prompt_arg: "JSON-RPC request lines on stdin after initialize/initialized handshake"
    notes: "Starts a long-running bidirectional JSON-RPC server, not the preferred Claudine exec wrapper surface."
output_formats:
  - name: "default text"
    cli_value: "no --json"
    stream: true
    format: text
    description: "Human progress is streamed to stderr and only the final agent message is printed to stdout."
    side_effects: "Stdout is not event telemetry; parsing requires prose scraping and loses tool/session structure."
  - name: "exec JSONL"
    cli_value: "--json"
    stream: true
    format: jsonl
    description: "One JSON object per stdout line with top-level type values such as thread.started, turn.started, item.started, item.updated, item.completed, turn.completed, turn.failed, and error. Claudine should prefer this mode."
    side_effects: "Stdout becomes parse-only JSONL; final prose is represented as an agent_message item and can also be copied with -o."
  - name: "final-message file"
    cli_value: "-o <FILE> / --output-last-message <FILE>"
    stream: false
    format: text
    description: "Writes the last assistant message to a file while preserving the selected stdout mode."
    side_effects: "Additive output sink; useful for final artifacts but not live supervision."
  - name: "schema-constrained final output"
    cli_value: "--output-schema <FILE>"
    stream: false
    format: other
    description: "Requests that the final assistant response conform to a JSON Schema; can be combined with --json and -o."
    side_effects: "Constrains the final agent message only; it is not a schema for the JSONL event stream."
  - name: "app-server stdio"
    cli_value: "codex app-server --listen stdio://"
    stream: true
    format: jsonrpc_lines
    description: "Bidirectional JSON-RPC-like line protocol with request id correlation and method notifications."
    side_effects: "Requires a client that sends initialize/initialized, starts threads/turns, handles approvals and cancellation, and keeps the server lifecycle."
schema_sources:
  - url: "https://github.com/openai/codex/blob/main/codex-rs/exec/src/exec_events.rs"
    schema_type: rust
    formal: false
    notes: "Best exact schema for codex exec --json; Rust Serde enum uses #[serde(tag = \"type\")] and item details use nested item.type."
  - url: "https://github.com/openai/codex/blob/main/codex-rs/exec/src/event_processor_with_jsonl_output.rs"
    schema_type: rust
    formal: false
    notes: "Authoritative projection layer from app-server notifications into the flattened exec JSONL stream."
  - url: "https://developers.openai.com/codex/noninteractive"
    schema_type: examples
    formal: false
    notes: "Official docs describe JSON Lines mode and list event and item families, but do not publish a complete exec JSON Schema."
  - url: "https://developers.openai.com/codex/app-server"
    schema_type: json_schema
    formal: true
    notes: "App-server can generate JSON Schema and TypeScript bindings for its broader JSON-RPC protocol; useful context but not the exec JSONL schema."
  - url: "https://raw.githubusercontent.com/openai/codex/main/codex-rs/app-server-protocol/schema/json/codex_app_server_protocol.v2.schemas.json"
    schema_type: json_schema
    formal: true
    notes: "Provider-authored JSON Schema Draft 7 bundle for app-server v2."
cli_params:
  - flag: "--json"
    value: "boolean"
    description: "Emit exec JSONL events to stdout."
    example: 'codex exec --json "summarize this repo"'
  - flag: "-o, --output-last-message"
    value: "FILE"
    description: "Write the final assistant message to a file in addition to the selected stdout mode."
    example: 'codex exec --json -o result.md "write summary"'
  - flag: "--output-schema"
    value: "FILE"
    description: "Path to a JSON Schema for the final response, not the event stream."
    example: 'codex exec --json --output-schema schema.json -o result.json "extract metadata"'
  - flag: "-m, --model"
    value: "MODEL"
    description: "Requested model for the run; exec JSONL does not currently echo it as stable metadata."
    example: 'codex exec --json -m gpt-5.5 "review"'
  - flag: "-C, --cd"
    value: "DIR"
    description: "Set the agent working root."
    example: 'codex exec --json -C /repo "inspect"'
  - flag: "--add-dir"
    value: "DIR"
    description: "Add an extra writable directory alongside the primary workspace."
    example: 'codex exec --json --add-dir ../shared "update files"'
  - flag: "-s, --sandbox"
    value: "read-only | workspace-write | danger-full-access"
    description: "Select sandbox policy for model-generated shell commands."
    example: 'codex exec --json --sandbox workspace-write "fix tests"'
  - flag: "--dangerously-bypass-approvals-and-sandbox"
    value: "boolean"
    description: "Disable approval prompts and sandboxing; intended only for externally sandboxed automation."
    example: 'codex exec --json --dangerously-bypass-approvals-and-sandbox "run migration"'
  - flag: "--ask-for-approval"
    value: "never | untrusted | on-request | granular"
    description: "Global/root option inherited by exec; controls approval behavior. Use never for deterministic read-only CI or bypass flag only in isolated runners."
    example: 'codex --ask-for-approval never exec --json "audit"'
  - flag: "--ephemeral"
    value: "boolean"
    description: "Run without persisted session files; disables later resume for that run."
    example: 'codex exec --json --ephemeral "one-shot analysis"'
  - flag: "--ignore-user-config"
    value: "boolean"
    description: "Skip $CODEX_HOME/config.toml while still using CODEX_HOME for auth/state."
    example: 'codex exec --json --ignore-user-config "controlled run"'
  - flag: "--ignore-rules"
    value: "boolean"
    description: "Skip user and project execpolicy .rules files."
    example: 'codex exec --json --ignore-rules "controlled run"'
  - flag: "-c, --config"
    value: "key=value"
    description: "Override TOML config values for this invocation."
    example: 'codex exec --json -c model="gpt-5.5" "review"'
  - flag: "-p, --profile"
    value: "CONFIG_PROFILE"
    description: "Load $CODEX_HOME/<name>.config.toml as a profile layer."
    example: 'codex exec --json --profile ci "review"'
  - flag: "-i, --image"
    value: "FILE..."
    description: "Attach image files to the initial prompt or resumed prompt."
    example: 'codex exec --json -i screenshot.png "analyze UI"'
  - flag: "--strict-config"
    value: "boolean"
    description: "Fail on unknown config fields for this Codex version."
    example: 'codex exec --json --strict-config "run"'
  - flag: "--color"
    value: "always | never | auto"
    description: "Color setting for human output; JSONL stdout remains JSON."
    example: 'codex exec --json --color never "run"'
config_files:
  - os: all
    scope: user
    path: "~/.codex/config.toml or $CODEX_HOME/config.toml"
    format: toml
    effect: "Durable defaults for model, provider, sandbox, approvals, MCP, hooks, tools, logging, reasoning visibility, shell environment, and history."
    notes: "Loaded below CLI flags, project config, and selected profile; can be skipped for exec with --ignore-user-config."
  - os: all
    scope: repo
    path: ".codex/config.toml"
    format: toml
    effect: "Trusted project-scoped overrides for many runtime settings."
    notes: "Loaded only for trusted projects; nearest project/subdirectory layer wins. Project config cannot override machine-local provider/auth/telemetry/profile/notify keys."
  - os: all
    scope: user
    path: "~/.codex/<profile>.config.toml or $CODEX_HOME/<profile>.config.toml"
    format: toml
    effect: "Named profile layer selected with --profile."
    notes: "Precedence is below trusted project config and above base user config."
  - os: linux
    scope: system
    path: "/etc/codex/config.toml"
    format: toml
    effect: "System default layer."
    notes: "Lower precedence than user config and higher than built-in defaults; official docs name Unix path."
  - os: all
    scope: managed
    path: "requirements.toml"
    format: toml
    effect: "Managed constraints for approval policies, sandbox modes, permission profiles, hooks, apps, web search, and related enterprise controls."
    notes: "Can restrict user/project choices; managed requirements take precedence for the constrained behavior."
  - os: all
    scope: user
    path: "$CODEX_HOME/auth.json or OS credential store"
    format: json
    effect: "Cached ChatGPT or API-key authentication."
    notes: "Sensitive secret material; codex exec can also use CODEX_API_KEY for a single invocation."
  - os: all
    scope: user
    path: "$CODEX_HOME/history.jsonl"
    format: other
    effect: "Persisted session transcript/history when history persistence is enabled."
    notes: "Not written for --ephemeral runs."
env_vars:
  - name: "CODEX_API_KEY"
    effect: "Provides an API key for one codex exec run."
    notes: "Officially supported only by codex exec; set inline for the single invocation rather than job-wide when repository-controlled code runs."
  - name: "CODEX_ACCESS_TOKEN"
    effect: "Provides a ChatGPT/Codex access token for trusted automation or login seeding."
    notes: "Can be piped to codex login --with-access-token for persisted auth; treat as secret."
  - name: "CODEX_HOME"
    effect: "Changes the root for config, auth, logs, sessions, skills, and state."
    notes: "Defaults to ~/.codex; directory must exist when set."
  - name: "CODEX_SQLITE_HOME"
    effect: "Changes SQLite-backed state location."
    notes: "sqlite_home config option takes precedence."
  - name: "CODEX_CA_CERTIFICATE"
    effect: "Sets PEM CA bundle for HTTPS/login/WebSocket clients."
    notes: "Takes precedence over SSL_CERT_FILE."
  - name: "SSL_CERT_FILE"
    effect: "Fallback PEM CA bundle for HTTPS/login/WebSocket clients."
    notes: "Used when CODEX_CA_CERTIFICATE is unset."
  - name: "RUST_LOG"
    effect: "Controls Codex Rust log verbosity; codex exec defaults to error-level output unless overridden."
    notes: "More verbose values can add diagnostics to stderr and should not be mixed into stdout parsing."
io_contract:
  stdout: structured_only
  stderr: diagnostics_only
  stdin: prompt
  framing: jsonl
  noise_handling: "When --json is set, parse stdout line-by-line as JSONL and treat stderr as diagnostics/lifecycle hints only. In text mode, stdout is final prose and stderr carries progress."
  notes: "app-server is a different stdin/stdout contract: bidirectional JSON-RPC-like request/response/notification lines."
stream_contract:
  discriminator: "type; nested item.type for item.started, item.updated, and item.completed"
  event_ordering: "thread.started is first; turn.started begins the turn; item lifecycle events follow; turn.completed or turn.failed is terminal for the exec run when emitted. A top-level error can appear before shutdown."
  correlation_fields: ["thread_id", "item.id", "item.sender_thread_id", "item.receiver_thread_ids"]
  terminal_event: "turn.completed or turn.failed"
  partial_message_events: false
  unknown_event_policy: "Skip unknown top-level or item types after logging parser telemetry; do not fail the wrapper unless completion cannot be classified."
  notes: "Events are complete item snapshots, not deltas. item.updated currently matters most for todo_list updates. No schema-version marker is present."
session_metadata:
  session_id: "thread.started.thread_id; always present as first JSONL event in normal exec runs and usable for resume unless the run is --ephemeral"
  cwd: "not emitted in exec JSONL; infer from wrapper invocation, -C/--cd, and process cwd"
  model: "not emitted in exec JSONL; infer requested model from CLI/config, or use hook/app-server surfaces for stronger evidence"
  provider: "not emitted in exec JSONL; infer from config model_provider/--oss/--local-provider"
  auth: "not emitted in exec JSONL; infer out-of-band from CODEX_API_KEY, auth status, or app-server account APIs"
  version: "not emitted in exec JSONL; wrapper must run codex --version out-of-band"
  mcp_servers: "not enumerated in exec JSONL; mcp_tool_call items reveal server names only when tools are called"
  permission_mode: "not emitted in exec JSONL; infer from flags/config/managed requirements"
  notes: "exec JSONL intentionally exposes only a compact thread/turn/item projection."
stream_events:
  - event: "thread.started"
    category: session
    fields: ["type", "thread_id"]
    notes: "First event; thread_id can be used for resume when persistence is enabled."
  - event: "turn.started"
    category: session
    fields: ["type"]
    notes: "Marks the start of model work."
  - event: "turn.completed"
    category: usage
    fields: ["type", "usage.input_tokens", "usage.cached_input_tokens", "usage.output_tokens", "usage.reasoning_output_tokens"]
    notes: "Terminal success for the turn; usage is total tokens from the latest app-server token usage snapshot."
  - event: "turn.failed"
    category: error
    fields: ["type", "error.message"]
    notes: "Terminal failure for the turn; structured CodexErrorInfo is not forwarded in the exec projection."
  - event: "error"
    category: error
    fields: ["type", "message"]
    notes: "Unrecoverable stream-level error; process may continue until turn.failed or shutdown."
  - event: "item.started:item.type=command_execution"
    category: tool_call
    fields: ["item.id", "item.command", "item.status"]
    notes: "Command start; status is usually in_progress."
  - event: "item.completed:item.type=command_execution"
    category: tool_result
    fields: ["item.id", "item.command", "item.aggregated_output", "item.exit_code", "item.status"]
    notes: "Command completion; status can be completed, failed, or declined."
  - event: "item.completed:item.type=file_change"
    category: file_change
    fields: ["item.id", "item.changes[].path", "item.changes[].kind", "item.status"]
    notes: "Patch/file change summary; declined file changes are collapsed to failed in the exec projection."
  - event: "item.started:item.type=mcp_tool_call"
    category: tool_call
    fields: ["item.id", "item.server", "item.tool", "item.arguments", "item.status"]
    notes: "MCP tool dispatch."
  - event: "item.completed:item.type=mcp_tool_call"
    category: tool_result
    fields: ["item.id", "item.server", "item.tool", "item.arguments", "item.result.content", "item.result._meta", "item.result.structured_content", "item.error.message", "item.status"]
    notes: "MCP tool success or failure."
  - event: "item.started:item.type=collab_tool_call"
    category: subagent
    fields: ["item.id", "item.tool", "item.sender_thread_id", "item.receiver_thread_ids", "item.prompt", "item.agents_states", "item.status"]
    notes: "Subagent/collaboration tool start."
  - event: "item.completed:item.type=collab_tool_call"
    category: subagent
    fields: ["item.id", "item.tool", "item.sender_thread_id", "item.receiver_thread_ids", "item.prompt", "item.agents_states", "item.status"]
    notes: "Subagent/collaboration tool terminal snapshot."
  - event: "item.completed:item.type=agent_message"
    category: assistant
    fields: ["item.id", "item.text"]
    notes: "Assistant message text; final answer is the last agent_message from the turn."
  - event: "item.completed:item.type=reasoning"
    category: reasoning
    fields: ["item.id", "item.text"]
    notes: "Reasoning summary text when available and not hidden; empty summaries are suppressed."
  - event: "item.started:item.type=web_search"
    category: tool_call
    fields: ["item.id", "item.query", "item.action"]
    notes: "Web search request."
  - event: "item.completed:item.type=web_search"
    category: tool_result
    fields: ["item.id", "item.query", "item.action"]
    notes: "Search item completion does not include raw search results in the exec item."
  - event: "item.started:item.type=todo_list"
    category: plan
    fields: ["item.id", "item.items[].text", "item.items[].completed"]
    notes: "Plan/todo list first snapshot."
  - event: "item.updated:item.type=todo_list"
    category: plan
    fields: ["item.id", "item.items[].text", "item.items[].completed"]
    notes: "Plan/todo status update."
  - event: "item.completed:item.type=todo_list"
    category: plan
    fields: ["item.id", "item.items[].text", "item.items[].completed"]
    notes: "Plan/todo final snapshot at turn end."
  - event: "item.completed:item.type=error"
    category: error
    fields: ["item.id", "item.message"]
    notes: "Non-fatal warning/config/deprecation/error item."
tools:
  - name: "shell / command execution"
    call_visible: true
    result_visible: true
    metadata: ["command", "aggregated_output", "exit_code", "status"]
    notes: "Visible as command_execution items; stdout/stderr are aggregated into aggregated_output rather than split streams."
  - name: "apply_patch / file changes"
    call_visible: false
    result_visible: true
    metadata: ["changes[].path", "changes[].kind", "status"]
    notes: "Visible as file_change completion snapshots only; no dedicated diff payload or approval reason in exec JSONL."
  - name: "MCP tools"
    call_visible: true
    result_visible: true
    metadata: ["server", "tool", "arguments", "result.content", "result._meta", "result.structured_content", "error.message", "status"]
    notes: "MCP startup state is not currently enumerated in exec JSONL; required server init failure exits with an error."
  - name: "web search"
    call_visible: true
    result_visible: true
    metadata: ["query", "action"]
    notes: "Search request/action visible; raw result set is not a structured exec field."
  - name: "plan / todo list"
    call_visible: true
    result_visible: true
    metadata: ["items[].text", "items[].completed"]
    notes: "Plan changes stream as todo_list started/updated/completed snapshots."
  - name: "collab / subagent tools"
    call_visible: true
    result_visible: true
    metadata: ["tool", "sender_thread_id", "receiver_thread_ids", "prompt", "agents_states", "status"]
    notes: "Parent stream sees collaboration tool snapshots, not every nested subagent event as separate parent events."
  - name: "image attachment / view_image"
    call_visible: false
    result_visible: false
    metadata: ["unknown"]
    notes: "Images can be attached with -i/--image; exact exec JSONL item visibility for image processing was not verified."
completion:
  success_event: "turn.completed"
  failure_event: "turn.failed or top-level error without a later turn.completed"
  exit_code_reliable: false
  result_fields: ["item.completed where item.type=agent_message -> item.text", "output-last-message file when -o is supplied"]
  cost_fields: []
  usage_fields: ["turn.completed.usage.input_tokens", "turn.completed.usage.cached_input_tokens", "turn.completed.usage.output_tokens", "turn.completed.usage.reasoning_output_tokens"]
  notes: "Outer process exit is useful for launch/auth/crash failures, but Claudine should classify agent/tool success from terminal events and item statuses. Cost and rate-limit fields are absent from exec JSONL."
blocking_behavior:
  permissions: configurable
  questions: fail
  tool_approvals: configurable
  notes: "Use --ask-for-approval never with read-only CI, --sandbox workspace-write for edit automation, or the bypass flag only inside external isolation. App-server has structured approval/user-input flows; exec mostly collapses unsupported elicitation paths into failures or declined/failed tool items."
subagents:
  supported: true
  start_visible: true
  stop_visible: true
  nested_events_visible: false
  prompt_injection_supported: true
  metadata_fields: ["collab_tool_call.tool", "sender_thread_id", "receiver_thread_ids", "prompt", "agents_states.<thread_id>.status", "agents_states.<thread_id>.message"]
  notes: "Subagents run through collaboration tools. The parent exec JSONL stream exposes collab tool snapshots and agent states; it does not flatten all nested subagent tool calls into parent stream events. Non-interactive instructions can be supplied through the root prompt, AGENTS.md, and custom agent config."
use_cases:
  - name: plan_cap_approaching
    detectable: false
    event_types: []
    fields: []
    hook_parity: "No hook parity found for exec JSONL."
    notes: "exec JSONL has no stable remaining-plan/quota percentage, reset time, or cap window."
  - name: plan_capped
    detectable: false
    event_types: ["turn.failed", "error"]
    fields: ["error.message", "message"]
    hook_parity: "No direct hook parity."
    notes: "Only text matching is available in exec JSONL; app-server has richer CodexErrorInfo but exec does not forward it."
  - name: no_funds
    detectable: false
    event_types: ["turn.failed", "error"]
    fields: ["error.message", "message"]
    hook_parity: "No direct hook parity."
    notes: "Billing/credit failure is not a distinct exec event."
  - name: auth
    detectable: false
    event_types: ["turn.failed", "error"]
    fields: ["error.message", "message"]
    hook_parity: "Out-of-band auth status and app-server account APIs are stronger."
    notes: "Auth kind is not emitted in exec JSONL; classify failures by launch stderr/exit and error message."
  - name: permission_read_denied
    detectable: true
    event_types: ["item.completed:item.type=command_execution"]
    fields: ["item.command", "item.aggregated_output", "item.exit_code", "item.status"]
    hook_parity: "PreToolUse/PermissionRequest hooks can be more precise when configured."
    notes: "Detect best-effort from declined/failed command_execution and OS error text; path extraction is not structured."
  - name: permission_write_denied
    detectable: true
    event_types: ["item.completed:item.type=file_change", "item.completed:item.type=command_execution"]
    fields: ["item.changes[].path", "item.status", "item.command", "item.aggregated_output"]
    hook_parity: "PreToolUse/PermissionRequest hooks can be more precise when configured."
    notes: "file_change failed can mean policy decline or patch failure; exec collapses declined to failed."
  - name: tokens_consumed
    detectable: true
    event_types: ["turn.completed"]
    fields: ["usage.input_tokens", "usage.cached_input_tokens", "usage.output_tokens", "usage.reasoning_output_tokens"]
    hook_parity: "No direct hook parity."
    notes: "Turn-total token counts; no cost units or per-tool token usage."
  - name: model_used
    detectable: false
    event_types: []
    fields: []
    hook_parity: "Hooks include model in their payloads; exec JSONL does not."
    notes: "Use wrapper-known requested model/config as a fallback; do not call it resolved backend model."
  - name: model_fallback
    detectable: true
    event_types: ["item.completed:item.type=error"]
    fields: ["item.message"]
    hook_parity: "unknown"
    notes: "The JSONL projection maps ModelRerouted to an error item message like 'model rerouted: from -> to (reason)', not a structured model_fallback event."
  - name: human_in_loop
    detectable: true
    event_types: ["item.completed:item.type=mcp_tool_call", "item.completed:item.type=command_execution", "turn.failed", "error"]
    fields: ["item.error.message", "item.status", "error.message", "message"]
    hook_parity: "App-server approval/user-input methods are richer; hooks are partial."
    notes: "Detect by declined statuses and messages such as unsupported request_user_input/user cancelled; exact question/options are not exposed in exec JSONL."
  - name: session_resumable
    detectable: true
    event_types: ["thread.started"]
    fields: ["thread_id"]
    hook_parity: "unknown"
    notes: "thread_id is emitted early; resume requires non-ephemeral persistence."
  - name: subagent_prompt_injection
    detectable: true
    event_types: ["item.started:item.type=collab_tool_call", "item.completed:item.type=collab_tool_call"]
    fields: ["item.prompt", "item.receiver_thread_ids", "item.agents_states"]
    hook_parity: "SubagentStart/SubagentStop hooks exist in config, but parent exec JSONL is the parser surface."
    notes: "Root prompts, AGENTS.md, and agent config can steer subagents; no dedicated CLI flag appends text to every subagent prompt."
headless_constraints:
  - constraint: "exec JSONL has no formal standalone JSON Schema or schema-version marker."
    mitigation: "Generate parser types from codex-rs/exec/src/exec_events.rs or maintain fixtures per Codex version."
    notes: "Do not validate exec JSONL against the app-server schema bundle."
  - constraint: "Model, provider, auth kind, cwd, permission mode, MCP server inventory, cost, and rate-limit caps are not emitted as stable exec JSONL metadata."
    mitigation: "Capture wrapper-side invocation/config and optionally run out-of-band status/version/config probes."
    notes: "The app-server protocol exposes some richer account/thread fields, but that is a different integration mode."
  - constraint: "Human approval and elicitation paths are not first-class exec events."
    mitigation: "Use deterministic approval/sandbox flags and classify declined/failed tool items plus error messages."
    notes: "MCP elicitation may fail instead of yielding a programmable prompt in exec mode."
  - constraint: "Process exit code alone is not enough to classify command/tool failure."
    mitigation: "Parse item statuses and terminal turn events; treat nonzero process exit as launch/crash/auth failure."
    notes: "A successful agent turn can contain failed commands."
  - constraint: "Reasoning visibility is configurable and may be absent."
    mitigation: "Do not use reasoning items as required heartbeats."
    notes: "hide_agent_reasoning suppresses reasoning in TUI and codex exec output."
quirks:
  - "The exact exec stream schema is the Rust Serde surface, not the app-server JSON Schema."
  - "item.type is a nested discriminator only inside item.* events; top-level type remains item.started/item.updated/item.completed."
  - "file_change declined is mapped to failed by the exec projection, losing the reason distinction."
  - "Model reroute is projected as an error item message rather than structured model fields."
  - "agent_message has text only; current public issue reports that phase/commentary vs final-answer distinction is dropped."
  - "No timestamps are present in the exec JSONL events."
  - "RUST_LOG can increase stderr diagnostics; stdout remains the only parse stream in --json mode."
gaps:
  - "No current local authenticated codex exec fixture was captured in this update; event details are from official docs, local help, and upstream source."
  - "Exact exit-code behavior for every launch/auth/rate-limit/cancellation path was not exhaustively re-tested on 0.142.5."
  - "Image attachment event visibility in exec JSONL was not verified."
  - "Whether app-server should become a future Claudine high-control mode remains a product decision; this document recommends exec JSONL for the current wrapper."
claudine_strategy:
  preferred_invocation: 'codex exec --json --sandbox workspace-write --ask-for-approval never "..."'
  required_flags: ["exec", "--json", "--sandbox <mode>", "--ask-for-approval never for deterministic CI or explicit bypass only in external isolation"]
  conflicting_flags: ["--ephemeral when Claudine needs resume", "text mode/no --json for lifecycle parsing", "app-server when using the simple exec parser"]
  parser_notes: "Parse stdout as JSONL with top-level type and nested item.type. Treat turn.completed/turn.failed as terminal, item.id as lifecycle correlation, thread.started.thread_id as resume identity, and unknown events as forward-compatible telemetry."
  wrapper_notes: "Capture codex --version, cwd, argv, config overrides, selected profile, env auth source, and process exit separately because exec JSONL omits them. Keep stderr for diagnostics but do not mix it into the JSON parser."
data_format: jsonl
changes:
  - "2026-07-02: Rewrote Codex non-interactive research against official docs, local codex-cli 0.142.5 help, and upstream Rust exec JSONL source; added schema-backed frontmatter."
requires_claudine_update: false
reason: "Research refresh only; no mandatory Claudine code change was proven beyond keeping provider metadata aligned with this document."
---

## Summary

Claudine can run Codex CLI non-interactively with structured live output by using `codex exec --json`. The official non-interactive documentation describes `codex exec` as the script/CI entry point and says that `--json` turns stdout into a JSON Lines stream where each line is a JSON object. The stream is useful while the run is still active: it exposes a thread id, turn boundaries, command/tool lifecycle, file-change summaries, plan updates, subagent collaboration tool snapshots, final assistant messages, and token usage on successful completion.

Claudine should prefer `codex exec --json` over text mode and over the app-server protocol for the current wrapper. Text mode loses most operational structure. App-server is richer and formally schema-generatable, but it is a bidirectional JSON-RPC-like protocol that requires Claudine to act as a full client, handle initialization, start threads and turns, answer approvals/user-input requests, and own server lifecycle. The main parser risks for `exec --json` are that the exact schema is defined by Rust Serde types rather than a published standalone JSON Schema, the stream omits important metadata such as model/auth/cost/rate limits, and some app-server concepts are flattened or lost in the exec projection.

## Non-Interactive Entry Points

The primary non-interactive entry point is `codex exec`. Official docs position it for scripts, CI, pipelines, and explicit sandbox/approval settings. A prompt can be passed as a positional argument, omitted and read from stdin, or represented by `-` to force stdin. Local `codex-cli 0.142.5` help confirms that if stdin is piped and a prompt argument is also provided, Codex treats the argument as the instruction and appends stdin as a `<stdin>` block.

Recommended fresh-session shape:

```bash
codex exec --json --sandbox workspace-write --ask-for-approval never "fix the failing tests"
```

Use `--sandbox read-only --ask-for-approval never` for read-only CI and `--sandbox workspace-write` when edits are required. `--dangerously-bypass-approvals-and-sandbox` is available, but it should only be used when Claudine or the caller has already put the process inside a separate isolation boundary. The deprecated `codex exec --full-auto` compatibility path should not be used for new wrapper work.

Resume is also scriptable:

```bash
codex exec --json resume --last "continue"
codex exec --json resume <SESSION_ID> "continue"
```

`thread.started.thread_id` is the identity Claudine can record for resume. That does not help if the original run used `--ephemeral`, because that disables persisted session files.

Codex also ships `codex app-server`, which can run over stdio, WebSocket, or Unix socket. App-server is not an output format for `exec`; it is a separate long-running protocol. It is attractive for a future deep integration because it exposes approvals, account APIs, richer thread items, cancellation, steering, and generated schemas. It is also more work and changes Claudine from a process wrapper into a protocol client.

## Output Formats

Codex has one best live structured format for non-interactive CLI wrapping: `codex exec --json`.

| Mode | Selector | Shape | Streams? | Claudine recommendation |
| --- | --- | --- | --- | --- |
| Default text | no `--json` | final prose on stdout, progress on stderr | yes, but human-oriented | Avoid for lifecycle parsing |
| Exec JSONL | `--json` | one JSON object per stdout line | yes | Prefer |
| Final-message file | `-o FILE` / `--output-last-message FILE` | final assistant text file | no live stream | Use as an optional artifact sink |
| Schema-constrained final output | `--output-schema FILE` | final response requested to match JSON Schema | no live stream | Optional final artifact constraint, not telemetry |
| App-server | `codex app-server --listen stdio://` | JSON-RPC-like request/response/notification lines | yes and bidirectional | Future high-control integration, not the exec parser |

In default text mode, Codex streams progress to stderr and prints only the final agent message to stdout. That makes shell pipelines pleasant for humans, but it is weak input for Claudine because tool calls, file changes, plans, usage, and failure shape are not structured on stdout.

With `--json`, official docs say stdout becomes JSON Lines and list event families including `thread.started`, `turn.started`, `turn.completed`, `turn.failed`, `item.*`, and `error`. Local `codex exec --help` for `codex-cli 0.142.5` says the same in shorter form: `--json` prints events to stdout as JSONL. This is the stream Claudine should parse.

`--output-schema` should be understood narrowly. It asks the model to make the final assistant response conform to a JSON Schema, and it can be paired with `-o` for a final JSON artifact. It does not validate the event stream and does not replace the JSONL parser.

## Schema Sources

The exact `codex exec --json` stream does not currently have a provider-published standalone JSON Schema. The best exact schema is the upstream Rust source:

- [`codex-rs/exec/src/exec_events.rs`](https://github.com/openai/codex/blob/main/codex-rs/exec/src/exec_events.rs)
- [`codex-rs/exec/src/event_processor_with_jsonl_output.rs`](https://github.com/openai/codex/blob/main/codex-rs/exec/src/event_processor_with_jsonl_output.rs)

`exec_events.rs` defines `ThreadEvent` as a Serde tagged enum with top-level `type`. It also defines `ThreadItemDetails` as a nested Serde tagged enum with `item.type` values such as `agent_message`, `reasoning`, `command_execution`, `file_change`, `mcp_tool_call`, `collab_tool_call`, `web_search`, `todo_list`, and `error`.

`event_processor_with_jsonl_output.rs` is equally important because it shows what the app-server notification stream loses or rewrites when projected into exec JSONL. Examples: agent messages and reasoning are not emitted as started items; file-change `declined` is mapped to `failed`; model reroute becomes an `error` item message; token usage is stored from token-usage notifications and emitted on `turn.completed`.

The app-server schema is formal but broader. Official app-server docs say clients can generate both TypeScript bindings and a JSON Schema bundle with `codex app-server generate-ts --out ./schemas` and `codex app-server generate-json-schema --out ./schemas`. The published schema bundle is useful context for richer concepts like account/rate-limit APIs, approvals, user-input requests, and thread notifications, but Claudine must not validate `exec --json` as if it were raw app-server JSON-RPC.

## IO Contract

For `codex exec --json`, stdout is parse-only JSONL. Each line is one complete JSON object. Claudine should parse stdout incrementally and should not expect banners, Markdown, or progress bars there.

Stderr is diagnostics and human progress/error output. Official docs say text mode uses stderr for progress and stdout for final content. Environment-variable docs also note that `codex exec` defaults Rust logging to `error` unless `RUST_LOG` is set, and that non-interactive mode prints messages inline instead of using a separate TUI log file. Claudine should preserve stderr for operator diagnostics and launch-failure classification, but not feed it into the JSONL parser.

Stdin is prompt/context input for `exec`. It is not a bidirectional protocol. App-server is the opposite: stdin/stdout are protocol lines, and the client must send requests such as `initialize`, `thread/start`, and `turn/start` while reading responses and notifications.

## Stream Contract

The exec JSONL top-level discriminator is `type`. For item events, there is a second discriminator at `item.type`.

The stable top-level union from current Rust source is:

- `thread.started`
- `turn.started`
- `turn.completed`
- `turn.failed`
- `item.started`
- `item.updated`
- `item.completed`
- `error`

Normal ordering starts with `thread.started`, then `turn.started`, then zero or more item events, and finally `turn.completed` or `turn.failed`. A top-level `error` can appear as an unrecoverable stream-level error. Claudine should treat `turn.completed` and `turn.failed` as terminal when they appear, while still using process exit to catch launch crashes, auth setup failures, and abnormal termination before a terminal event.

Events are snapshots, not partial deltas. `item.updated` carries a full current snapshot for the item; today the important case is `todo_list`. Assistant text is not streamed as token deltas in exec JSONL. The useful assistant field is `item.completed` with `item.type = "agent_message"` and `item.text`.

Correlation is mostly by `item.id`. Subagent/collaboration items additionally expose `sender_thread_id`, `receiver_thread_ids`, and an `agents_states` map. `thread.started.thread_id` is the session identity. There are no timestamps, no schema version marker, and no event sequence number, so Claudine should process events in arrival order and tolerate unknown future events by logging and skipping them.

## Session Metadata

`thread.started.thread_id` is emitted first and is the strongest session identity in exec JSONL. It is suitable for logs and resume recovery as long as the session is persisted.

Most other metadata Claudine wants is absent from the stream:

| Metadata | Exec JSONL support | Practical source |
| --- | --- | --- |
| cwd/project root | not emitted | wrapper cwd, `-C/--cd`, config |
| CLI version | not emitted | `codex --version` before launch |
| requested model | not emitted | argv/config/profile captured by wrapper |
| resolved model/provider | not emitted | out-of-band config/app-server/hook evidence |
| auth kind/source | not emitted | env/auth status/app-server account APIs |
| sandbox/approval mode | not emitted | argv/config/managed requirements |
| MCP server inventory | not emitted | config; `mcp_tool_call.server` only when called |
| tools enabled | partially visible | item types that actually occur |

The absence of model identity is a known integration gap. The stream can reveal model reroute only indirectly because the projection maps `ModelRerouted` into an `item.completed` error message such as `model rerouted: from -> to (reason)`.

## Event Families

The event families Claudine can parse are:

| Family | Events | Notes |
| --- | --- | --- |
| Session/turn | `thread.started`, `turn.started`, `turn.completed`, `turn.failed` | Boundaries and final usage |
| Assistant | `item.completed` / `agent_message` | Final answer is the last agent message |
| Reasoning | `item.completed` / `reasoning` | Summary text only; can be hidden or absent |
| Command tools | `item.started`, `item.completed` / `command_execution` | Command, aggregate output, exit code, status |
| File changes | `item.completed` / `file_change` | Paths, add/delete/update, status |
| MCP tools | `item.started`, `item.completed` / `mcp_tool_call` | Server/tool/args/result/error/status |
| Web search | `item.started`, `item.completed` / `web_search` | Query and action, not raw result set |
| Plan/todo | `item.started`, `item.updated`, `item.completed` / `todo_list` | Step text plus completed booleans |
| Subagents | `item.started`, `item.completed` / `collab_tool_call` | Collaboration tool snapshots and agent states |
| Errors/warnings | top-level `error`, `turn.failed`, `item.completed` / `error` | Mixed severity; only message is stable |

The app-server protocol has many more event and item types. Claudine should not assume they appear in exec JSONL unless `event_processor_with_jsonl_output.rs` maps them.

## Tools

Command execution is the strongest tool surface in exec JSONL. The start event exposes `item.command` and `status: "in_progress"`. The completion event adds `aggregated_output`, `exit_code`, and final `status`. The output is aggregated; stdout and stderr are not separately typed.

File edits are visible as `file_change` completed items with `changes[].path`, `changes[].kind`, and `status`. There is no dedicated start event in the exec projection. A declined file change from the richer protocol is collapsed to `failed`, so Claudine cannot reliably distinguish policy denial from patch failure using only `file_change.status`.

MCP tool calls expose start and completion, including `server`, `tool`, `arguments`, optional `result.content`, optional `result._meta`, optional `result.structured_content`, optional `error.message`, and `status`. MCP server initialization state is not currently a first-class exec event, but the official non-interactive docs say a required MCP server that fails to initialize makes `codex exec` exit with an error.

Subagents are represented through `collab_tool_call` items. Supported collab tool enum values in the exec Rust type are `spawn_agent`, `send_input`, `wait`, and `close_agent`. The payload includes sender/receiver thread ids, optional prompt, agent state map, and status. This is enough for Claudine to show that subagent machinery was invoked, but not enough to reconstruct every nested subagent command/tool event as if it were part of the parent stream.

Web search and plan updates are visible enough for progress UI. Web search exposes the query/action. The plan stream uses `todo_list` snapshots and updates.

## Completion and Exit Status

For agent success, trust `turn.completed`. It includes token usage:

```json
{"type":"turn.completed","usage":{"input_tokens":24763,"cached_input_tokens":24448,"output_tokens":122,"reasoning_output_tokens":0}}
```

For agent failure, trust `turn.failed` when present:

```json
{"type":"turn.failed","error":{"message":"turn failed"}}
```

The final answer is the last `item.completed` where `item.type` is `agent_message`. If `-o/--output-last-message` is supplied, Codex also writes that final message to a file. Claudine should still parse the stream because the file is not live telemetry.

The process exit code should be treated as necessary but not sufficient. A nonzero exit before any terminal event is meaningful for launch/auth/crash failures. A zero exit does not prove every tool succeeded, because a completed agent turn can contain failed or declined command/file/MCP items. Claudine should classify tool failures from item statuses and classify run failure from `turn.failed`, missing terminal event plus abnormal exit, or timeout/cancellation.

Token units are tokens. They are turn-total values as exposed by `turn.completed.usage`. There is no cost field, no currency, no per-tool token count, and no reset window. There are also no timestamps.

## Blocking Behavior

Non-interactive runs should be launched with explicit sandbox and approval settings. Official docs say `codex exec` defaults to a read-only sandbox and recommend least-privilege sandbox choices for automation. The approvals/security docs list read-only CI as `--sandbox read-only --ask-for-approval never`, automatic edit mode as `--sandbox workspace-write`, and dangerous full access as `--dangerously-bypass-approvals-and-sandbox` / `--yolo`.

When a human decision would be needed, exec JSONL does not expose a rich programmable question/answer event. Depending on the surface, Claudine may see a declined command, a failed file change, a failed MCP tool with an error message, `turn.failed`, a top-level `error`, or process failure. App-server is the surface with structured approval requests and user-input methods; `exec` is the safer current choice only when Claudine preconfigures permissions so the run is deterministic.

Authentication is another automation boundary. Official docs recommend API-key auth for programmatic CLI workflows and document `CODEX_API_KEY` for a single `codex exec` invocation. Browser login is interactive; ChatGPT-managed auth on CI requires seeding persisted auth carefully and is only appropriate for trusted runners.

## Subagents

Codex supports subagent/collaboration tools non-interactively, and exec JSONL exposes them as `collab_tool_call` items. The payload identifies the collab tool, sender thread id, receiver thread ids, optional prompt, agent states, and final status. This is enough for high-level Claudine lifecycle reporting: a subagent was spawned, waited on, or closed; which child thread ids were involved; and what the last known child states were.

The parent stream does not flatten all nested subagent events into separate parent events. Claudine should not expect a child command execution to appear as a normal parent `command_execution` item unless the parent itself ran it. For prompt injection, there is no dedicated `--subagent-system-prompt` flag in `codex exec`; the practical controls are the root prompt, AGENTS.md/project instructions, skills, and custom agent config.

## Use Case Detection

`tokens_consumed` is strongly detectable from `turn.completed.usage`. The fields are `input_tokens`, `cached_input_tokens`, `output_tokens`, and `reasoning_output_tokens`.

`session_resumable` is strongly detectable from `thread.started.thread_id`, with the caveat that `--ephemeral` prevents persistence.

`permission_read_denied` and `permission_write_denied` are only partially detectable. Read denial usually appears as a `command_execution` item with `status: "declined"` or `status: "failed"` plus OS/policy text in `aggregated_output`. Write denial through patching appears as a `file_change` with `status: "failed"` and affected paths. That does not cleanly distinguish policy denial from patch failure because exec collapses declined file changes to failed.

`human_in_loop` is detectable only as a near miss: declined statuses, failed MCP tool calls, or messages that mention unsupported user input or cancellation. The actual question/options are not emitted in exec JSONL.

`model_used` is not detectable from exec JSONL. Claudine can record the requested model from argv/config, but that is not a resolved backend model. `model_fallback` is weakly detectable because reroute is projected as a text error item rather than structured fields.

Plan-cap, quota, billing/no-funds, and auth-kind detection are weak in exec JSONL. They may surface as `turn.failed` or `error` message text, but there is no stable enum, remaining quantity, reset time, or billing discriminator. App-server account/rate-limit APIs are richer, but they are outside the exec stream.

## Headless Constraints

The biggest constraint is schema stability. `exec --json` is documented, but the exact event contract is a Rust Serde type and projection layer. Claudine should maintain fixtures and tolerate unknown events.

The second constraint is metadata absence. Claudine must capture wrapper-side context: `codex --version`, process cwd, argv, selected profile, config overrides, sandbox and approval flags, intended model, and auth source. None of that is reliably present in JSONL.

The third constraint is approval and elicitation behavior. To avoid automation hangs or ambiguous failures, Claudine should launch with deterministic settings. Use `--ask-for-approval never` for read-only CI. Use `workspace-write` only when writes are intended. Use bypass only when the outer environment is disposable and sandboxed.

Finally, do not use app-server accidentally as if it were an output format. App-server is a bidirectional protocol. If Claudine adopts it later, the parser must become a client that sends requests, handles ids, answers or rejects approvals, and issues cancellation/interrupt methods.

## Timeline

| Date | Event | Why it matters |
| --- | --- | --- |
| 2025-07-24 | GitHub issue requesting JSON Schema for `--json` output was opened | Confirms the lack of a standalone exec schema was already an integration concern |
| 2026-03-10 | Codex CLI 0.113.0 changelog period wired exec closer to app-server internals | Explains why exec JSONL mirrors a subset of app-server concepts |
| 2026-03-11 | Codex CLI 0.114.0 changelog period added hooks/schema groundwork | Relevant to lifecycle and generated protocol types |
| 2026-04-06 | Earlier Claudine Codex research captured the exec Rust event union | Baseline for this refresh |
| 2026-07-02 | This document was refreshed against current official docs, local `codex-cli 0.142.5` help, and upstream Rust source | Current recommendation remains `codex exec --json` |

## Quirks and Gaps

`exec --json` and app-server are related but not interchangeable. The app-server schema has richer account, rate-limit, approval, hook, and thread concepts. The exec projection intentionally flattens the stream and drops many of those fields.

Some fields are lossy. File-change decline becomes failure. Model reroute becomes a text error item. Agent message items expose text but not a stable phase that distinguishes mid-turn commentary from final answer. Reasoning can be hidden with `hide_agent_reasoning` and should not be used as a heartbeat.

No authenticated local run fixture was captured during this update. The event taxonomy comes from official docs and source, and local execution evidence is limited to `codex-cli 0.142.5` help/version output. Image attachment event shape remains unverified.

## Claudine Integration Notes

Use this as the default wrapper shape:

```bash
codex exec --json --sandbox workspace-write --ask-for-approval never "..."
```

Adjust sandbox downward to `read-only` for audit-only tasks. Add `-o <path>` only when Claudine needs a final artifact file. Add `--output-schema <schema.json>` only for final response shape; keep parsing JSONL for lifecycle.

Parse stdout only. The parser should use `type` as the top-level discriminator and `item.type` for `item.*`. Join item start/update/completion by `item.id`. Treat `thread.started.thread_id` as session identity. Treat `turn.completed` and `turn.failed` as terminal stream events. Keep stderr for diagnostics and pre-terminal failures, especially auth/config/MCP startup errors.

Before launch, capture out-of-band metadata that the stream lacks: `codex --version`, cwd, requested model, profile/config overrides, sandbox mode, approval mode, auth source/env choice, and whether `--ephemeral` was used. After launch, classify success from the terminal event and item statuses rather than process exit alone.

Avoid app-server for the current simple wrapper. It is the right future surface if Claudine needs structured approvals, mid-turn steering, account/rate-limit APIs, or cancellation as protocol methods, but it requires a separate client implementation.

## Changelog

- 2026-07-02: Rewrote the document into the requested non-interactive research shape, added `$schema: ./_schema.yaml`, refreshed commands and config behavior, and reconciled the event contract against current upstream Rust source.

## Sources

- [OpenAI Codex non-interactive mode](https://developers.openai.com/codex/noninteractive)
- [OpenAI Codex CLI reference](https://developers.openai.com/codex/cli/reference)
- [OpenAI Codex app-server docs](https://developers.openai.com/codex/app-server)
- [OpenAI Codex config basics](https://developers.openai.com/codex/config-basic)
- [OpenAI Codex configuration reference](https://developers.openai.com/codex/config-reference)
- [OpenAI Codex environment variables](https://developers.openai.com/codex/environment-variables)
- [OpenAI Codex authentication](https://developers.openai.com/codex/auth)
- [OpenAI Codex approvals and security](https://developers.openai.com/codex/agent-approvals-security)
- [openai/codex `exec_events.rs`](https://github.com/openai/codex/blob/main/codex-rs/exec/src/exec_events.rs)
- [openai/codex `event_processor_with_jsonl_output.rs`](https://github.com/openai/codex/blob/main/codex-rs/exec/src/event_processor_with_jsonl_output.rs)
- [openai/codex app-server JSON Schema bundle](https://raw.githubusercontent.com/openai/codex/main/codex-rs/app-server-protocol/schema/json/codex_app_server_protocol.v2.schemas.json)
- Local inspection: `codex-cli 0.142.5`, `codex exec --help`, and `codex exec resume --help` on 2026-07-02.
