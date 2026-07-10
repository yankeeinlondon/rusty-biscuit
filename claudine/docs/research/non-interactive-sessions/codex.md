---
$schema: ./_schema.yaml
created: 2026-04-06
last_updated: 2026-07-03
agent: codex
model: default
docs: https://developers.openai.com/codex/noninteractive
invocation:
  - command: 'codex exec --json "prompt"'
    stdin_support: true
    prompt_arg: "PROMPT argument, omitted prompt, or '-' for stdin; if stdin is piped and PROMPT is also present, stdin is appended as a <stdin> context block"
    notes: "Starts a fresh non-interactive local session and emits JSONL events on stdout."
  - command: 'codex exec --json -'
    stdin_support: true
    prompt_arg: "'-' forces stdin to be the prompt"
    notes: "Starts a fresh non-interactive session with prompt text read from stdin."
  - command: 'codex exec --json resume --last "prompt"'
    stdin_support: true
    prompt_arg: "Optional PROMPT argument or '-' for stdin"
    notes: "Resumes the newest recorded session. Do not rely on this after an original --ephemeral run."
  - command: 'codex exec --json resume <SESSION_ID> "prompt"'
    stdin_support: true
    prompt_arg: "Optional PROMPT argument or '-' for stdin"
    notes: "Resumes a specific thread/session id or name and emits the same exec JSONL stream."
  - command: 'codex exec review --json'
    stdin_support: false
    prompt_arg: "review subcommand arguments"
    notes: "Runs non-interactive local code review through exec; JSONL stream shape is the same exec event family."
  - command: 'codex app-server --listen stdio://'
    stdin_support: true
    prompt_arg: "JSON-RPC request lines on stdin after initialize/initialized handshake"
    notes: "Starts a long-running bidirectional protocol server. It is richer than exec JSONL but is not the recommended Claudine local job wrapper."
  - command: 'codex app-server --listen ws://127.0.0.1:<PORT>'
    stdin_support: false
    prompt_arg: "JSON-RPC messages over WebSocket text frames"
    notes: "Experimental app-server WebSocket transport. Use only for product integrations that need the full server protocol."
  - command: 'codex cloud list --json'
    stdin_support: false
    prompt_arg: "cloud list filters"
    notes: "Scriptable cloud task listing, not a local non-interactive agent run."
output_formats:
  - name: "default text"
    cli_value: "no --json"
    stream: true
    format: text
    description: "Progress is streamed to stderr and only the final agent message is printed to stdout."
    side_effects: "Stdout is final text, not event telemetry; parser would lose session, tool, file-change, plan, and usage structure."
  - name: "exec JSONL"
    cli_value: "--json"
    stream: true
    format: jsonl
    description: "One JSON object per stdout line with top-level type values including thread.started, turn.started, item.started, item.updated, item.completed, turn.completed, turn.failed, and error. Claudine should prefer this mode."
    side_effects: "Stdout becomes parse-only JSONL. Final prose is represented as an agent_message item; -o can additionally copy it to a file."
  - name: "exec JSONL legacy alias"
    cli_value: "--experimental-json"
    stream: true
    format: jsonl
    description: "Alias used by the TypeScript SDK for the same exec JSONL mode."
    side_effects: "Equivalent to --json in the current CLI; prefer the stable --json spelling in wrappers."
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
    description: "Requests that the final assistant response conform to a JSON Schema."
    side_effects: "Constrains only the final agent message. It is not a schema for the JSONL event stream and can be combined with --json."
  - name: "app-server stdio"
    cli_value: "codex app-server --listen stdio://"
    stream: true
    format: jsonrpc_lines
    description: "Bidirectional JSON-RPC-like line protocol with method names, request ids, responses, and notifications."
    side_effects: "Requires a client that initializes the server, starts threads/turns, handles server requests, and owns cancellation/lifecycle."
  - name: "app-server websocket"
    cli_value: "codex app-server --listen ws://IP:PORT"
    stream: true
    format: other
    description: "Bidirectional JSON-RPC messages framed as WebSocket text frames."
    side_effects: "Experimental and unsupported; must configure authentication before exposing beyond localhost."
  - name: "cloud list JSON"
    cli_value: "codex cloud list --json"
    stream: false
    format: json
    description: "Single JSON object containing a tasks array and optional cursor."
    side_effects: "Applies only to cloud task listing, not local agent progress."
schema_sources:
  - url: "https://github.com/openai/codex/blob/main/codex-rs/exec/src/exec_events.rs"
    schema_type: rust
    formal: false
    notes: "Best exact schema for codex exec --json; Rust Serde enum uses top-level #[serde(tag = \"type\")] and nested item.type values."
  - url: "https://github.com/openai/codex/blob/main/codex-rs/exec/src/event_processor_with_jsonl_output.rs"
    schema_type: rust
    formal: false
    notes: "Authoritative projection layer from app-server notifications into the flattened exec JSONL stream."
  - url: "https://github.com/openai/codex/blob/main/sdk/typescript/src/events.ts"
    schema_type: typescript
    formal: false
    notes: "Typed SDK union for exec stream events, generated or maintained from the Rust exec event types."
  - url: "https://github.com/openai/codex/blob/main/sdk/typescript/src/items.ts"
    schema_type: typescript
    formal: false
    notes: "Typed SDK union for nested exec item payloads."
  - url: "https://developers.openai.com/codex/noninteractive"
    schema_type: examples
    formal: false
    notes: "Official docs describe JSON Lines mode and list event/item families, but do not publish a complete exec JSON Schema."
  - url: "https://developers.openai.com/codex/app-server"
    schema_type: json_schema
    formal: true
    notes: "App-server can generate JSON Schema and TypeScript bindings for its broader JSON-RPC protocol; useful context but not the exec JSONL schema."
cli_params:
  - flag: "--json"
    value: "boolean"
    description: "Emit exec events to stdout as JSONL."
    example: 'codex exec --json "summarize this repo"'
  - flag: "--experimental-json"
    value: "boolean"
    description: "Legacy alias for --json used by the TypeScript SDK."
    example: 'codex exec --experimental-json "summarize this repo"'
  - flag: "--output-schema"
    value: "FILE"
    description: "Request a final assistant response matching the supplied JSON Schema."
    example: "codex exec --json --output-schema ./schema.json 'extract metadata'"
  - flag: "--output-last-message / -o"
    value: "FILE"
    description: "Write the final assistant message to a file in addition to normal stdout behavior."
    example: "codex exec --json -o ./final.md 'write release notes'"
  - flag: "--model / -m"
    value: "MODEL"
    description: "Override the configured model."
    example: 'codex exec --json -m gpt-5.5 "review this change"'
  - flag: "--sandbox / -s"
    value: "read-only | workspace-write | danger-full-access"
    description: "Set sandbox policy for model-generated shell commands."
    example: 'codex exec --json --sandbox workspace-write "fix the test"'
  - flag: "--ask-for-approval / -a"
    value: "untrusted | on-request | never"
    description: "Global approval policy flag; exec defaults to never asking unless configuration and auto-review rebuild alter the effective policy."
    example: 'codex exec --json -a never "run checks"'
  - flag: "--dangerously-bypass-approvals-and-sandbox / --yolo"
    value: "boolean"
    description: "Disable approvals and sandboxing for externally isolated automation."
    example: 'codex exec --json --yolo "apply the patch"'
  - flag: "--cd / -C"
    value: "DIR"
    description: "Set the working directory before the run."
    example: 'codex exec --json -C /repo "summarize"'
  - flag: "--add-dir"
    value: "DIR"
    description: "Grant additional writable roots alongside the primary workspace."
    example: "codex exec --json --add-dir ../shared 'update both crates'"
  - flag: "--image / -i"
    value: "FILE[,FILE...]"
    description: "Attach images to the initial prompt."
    example: "codex exec --json -i screenshot.png 'prototype this UI'"
  - flag: "--profile / -p"
    value: "NAME"
    description: "Layer CODEX_HOME/<name>.config.toml on top of base user config."
    example: 'codex exec --json --profile ci "review"'
  - flag: "--config / -c"
    value: "key=value"
    description: "One-off TOML config override; CLI overrides have highest precedence."
    example: "codex exec --json -c 'web_search=\"disabled\"' 'answer from repo only'"
  - flag: "--ignore-user-config"
    value: "boolean"
    description: "Do not load CODEX_HOME/config.toml; auth still uses CODEX_HOME."
    example: 'codex exec --json --ignore-user-config "inspect"'
  - flag: "--ignore-rules"
    value: "boolean"
    description: "Skip user and project execpolicy .rules files."
    example: 'codex exec --json --ignore-rules "run scripted task"'
  - flag: "--skip-git-repo-check"
    value: "boolean"
    description: "Allow running outside a Git repository."
    example: 'codex exec --json --skip-git-repo-check "summarize this folder"'
  - flag: "--ephemeral"
    value: "boolean"
    description: "Do not persist session files; makes later resume unavailable."
    example: 'codex exec --json --ephemeral "one-shot triage"'
  - flag: "--strict-config"
    value: "boolean"
    description: "Fail when config.toml contains fields this Codex version does not recognize."
    example: 'codex exec --json --strict-config "check"'
  - flag: "--color"
    value: "always | never | auto"
    description: "Controls human color output; JSONL stdout remains JSON."
    example: 'codex exec --json --color never "inspect"'
  - flag: "--oss"
    value: "boolean"
    description: "Use local open-source provider defaults."
    example: 'codex exec --json --oss "summarize"'
config_files:
  - os: macos
    scope: user
    path: "~/.codex/config.toml"
    format: toml
    effect: "Sets default model, provider, approval policy, sandbox, MCP servers, web search, hooks, permissions, logging, OTel, and features."
    notes: "Can be moved with CODEX_HOME. CLI flags and -c overrides win. --ignore-user-config skips this file but not auth state."
  - os: linux
    scope: user
    path: "~/.codex/config.toml"
    format: toml
    effect: "Same user config effects as macOS."
    notes: "Can be moved with CODEX_HOME. CLI flags and -c overrides win. --ignore-user-config skips this file but not auth state."
  - os: windows
    scope: user
    path: "%USERPROFILE%\\.codex\\config.toml"
    format: toml
    effect: "Same user config effects as macOS/Linux."
    notes: "Default CODEX_HOME is the user home .codex directory; Windows system config uses ProgramData separately."
  - os: macos
    scope: user
    path: "~/.codex/<profile>.config.toml"
    format: toml
    effect: "Profile layer selected by --profile; commonly changes model, reasoning, provider, approval, sandbox, and model catalog."
    notes: "Loaded above user config and below trusted project config and CLI overrides."
  - os: linux
    scope: user
    path: "~/.codex/<profile>.config.toml"
    format: toml
    effect: "Same profile behavior as macOS."
    notes: "Loaded above user config and below trusted project config and CLI overrides."
  - os: windows
    scope: user
    path: "%USERPROFILE%\\.codex\\<profile>.config.toml"
    format: toml
    effect: "Same profile behavior as macOS/Linux."
    notes: "Loaded above user config and below trusted project config and CLI overrides."
  - os: macos
    scope: repo
    path: ".codex/config.toml"
    format: toml
    effect: "Trusted project overrides for model, sandbox, MCP, hooks, permissions, instructions, and feature-relevant settings."
    notes: "Codex walks from project root toward cwd; closest trusted project config wins for duplicate keys. Provider/auth redirection keys are ignored in project config."
  - os: linux
    scope: repo
    path: ".codex/config.toml"
    format: toml
    effect: "Same trusted project override behavior as macOS."
    notes: "Ignored when the project is untrusted."
  - os: windows
    scope: repo
    path: ".codex\\config.toml"
    format: toml
    effect: "Same trusted project override behavior as macOS/Linux."
    notes: "Ignored when the project is untrusted."
  - os: macos
    scope: system
    path: "/etc/codex/config.toml"
    format: toml
    effect: "System default config layer below user config."
    notes: "Lowest explicit config layer before built-in defaults."
  - os: linux
    scope: system
    path: "/etc/codex/config.toml"
    format: toml
    effect: "System default config layer below user config."
    notes: "Lowest explicit config layer before built-in defaults."
  - os: windows
    scope: system
    path: "%ProgramData%\\OpenAI\\Codex\\config.toml"
    format: toml
    effect: "System default config layer below user config."
    notes: "Source code falls back to C:\\ProgramData if the ProgramData known folder cannot be resolved."
  - os: macos
    scope: managed
    path: "com.openai.codex:config_toml_base64"
    format: toml
    effect: "MDM managed default config encoded as base64 TOML."
    notes: "Managed defaults apply before ordinary user changes; admin requirements can still constrain effective values."
  - os: linux
    scope: managed
    path: "cloud-managed config bundle"
    format: toml
    effect: "ChatGPT Business/Enterprise managed defaults when available for signed-in users."
    notes: "Best-effort remote managed layer; exact local cache path not verified."
  - os: windows
    scope: managed
    path: "cloud-managed config bundle"
    format: toml
    effect: "ChatGPT Business/Enterprise managed defaults when available for signed-in users."
    notes: "Best-effort remote managed layer; exact local cache path not verified."
  - os: macos
    scope: managed
    path: "com.openai.codex:requirements_toml_base64"
    format: toml
    effect: "MDM admin-enforced requirements for allowed approval policies, sandbox modes, permission profiles, MCP, hooks, web search, and features."
    notes: "Requirements have precedence over user attempts to broaden restricted settings."
  - os: linux
    scope: managed
    path: "/etc/codex/requirements.toml"
    format: toml
    effect: "System admin-enforced requirements."
    notes: "Cloud-managed requirements outrank system requirements when present."
  - os: macos
    scope: managed
    path: "/etc/codex/requirements.toml"
    format: toml
    effect: "System admin-enforced requirements."
    notes: "Cloud-managed and MDM requirements outrank this file when present."
  - os: windows
    scope: managed
    path: "%ProgramData%\\OpenAI\\Codex\\requirements.toml"
    format: toml
    effect: "System admin-enforced requirements."
    notes: "Cloud-managed requirements outrank this file when present."
  - os: macos
    scope: user
    path: "~/.codex/hooks.json"
    format: json
    effect: "User lifecycle hooks can affect tool approval, logging, and run behavior."
    notes: "Hooks can also be inline in config.toml. Project hooks require trust."
  - os: linux
    scope: user
    path: "~/.codex/hooks.json"
    format: json
    effect: "Same hook behavior as macOS."
    notes: "Hooks can also be inline in config.toml."
  - os: windows
    scope: user
    path: "%USERPROFILE%\\.codex\\hooks.json"
    format: json
    effect: "Same hook behavior as macOS/Linux."
    notes: "Hooks can also be inline in config.toml."
env_vars:
  - name: "CODEX_HOME"
    effect: "Sets Codex state root including config, auth, logs, sessions, skills, and package metadata."
    notes: "Default is ~/.codex; directory must already exist when set."
  - name: "CODEX_SQLITE_HOME"
    effect: "Sets SQLite-backed state location unless sqlite_home config takes precedence."
    notes: "Relative paths resolve from current working directory."
  - name: "CODEX_API_KEY"
    effect: "Supplies an API key for a single codex exec run."
    notes: "Only supported by codex exec; set inline rather than job-wide around untrusted repository code."
  - name: "CODEX_ACCESS_TOKEN"
    effect: "Supplies a ChatGPT/Codex access token for trusted automation or login --with-access-token."
    notes: "Useful when automation needs ChatGPT-managed access."
  - name: "CODEX_CA_CERTIFICATE"
    effect: "PEM CA bundle for HTTPS, login, and WebSocket clients."
    notes: "Takes precedence over SSL_CERT_FILE."
  - name: "SSL_CERT_FILE"
    effect: "Fallback PEM CA bundle when CODEX_CA_CERTIFICATE is unset."
    notes: "Affects network/auth behavior, not stream shape."
  - name: "RUST_LOG"
    effect: "Controls Rust log verbosity; codex exec defaults to error output unless overridden."
    notes: "More verbose values can add stderr diagnostics; stdout JSONL remains parse-only in --json mode."
  - name: "CODEX_NON_INTERACTIVE"
    effect: "Makes standalone installer scripts skip installer prompts."
    notes: "Installer-only; not a runtime exec-mode flag."
  - name: "CODEX_INSTALL_DIR"
    effect: "Changes standalone installer destination."
    notes: "Installer-only."
  - name: "provider env_key variables"
    effect: "Custom provider API keys are read from whatever env var is named by model_providers.<id>.env_key."
    notes: "Variable names are user-defined; record effective provider config rather than assuming OPENAI_API_KEY."
io_contract:
  stdout: structured_only
  stderr: diagnostics_only
  stdin: prompt
  framing: jsonl
  noise_handling: "With --json, parse stdout line-by-line as JSON and keep stderr as diagnostics for startup/config/auth failures and non-zero exits. Without --json, stdout is final text and stderr is human progress."
  notes: "The exec crate denies accidental stdout writes: default mode reserves stdout for final message, JSON mode reserves stdout for valid JSONL, and all other output goes to stderr."
stream_contract:
  discriminator: "type"
  event_ordering: "thread.started is first for a new thread; turn.started follows turn submission; item events stream during the turn; turn.completed or turn.failed is terminal for normal/failing turns. Interrupted turns may initiate shutdown without an exec JSON terminal event."
  correlation_fields: ["thread_id", "item.id", "item.sender_thread_id", "item.receiver_thread_ids"]
  terminal_event: "turn.completed or turn.failed"
  partial_message_events: false
  unknown_event_policy: "Skip unknown top-level or item types, preserve raw JSON for logs, and continue parsing by discriminator."
  notes: "item.started and item.completed for the same raw app-server item are mapped to the same flattened item.id while in progress. Assistant messages and reasoning are emitted as completed items, not token deltas."
session_metadata:
  session_id: "thread.started.thread_id, first event for new exec threads and usable with codex exec resume <SESSION_ID>"
  cwd: "Not emitted in exec JSONL; effective cwd comes from wrapper invocation (--cd), config, or app-server/OTel surfaces."
  model: "Not emitted in exec JSONL; requested model is known from --model/config. Model reroute appears only as item.completed item.type=error text."
  provider: "Not emitted in exec JSONL; infer from config/model_provider or --oss."
  auth: "Not emitted in exec JSONL; auth failures appear as stderr/startup errors or error/turn.failed messages."
  version: "Not emitted in exec JSONL; wrapper must capture `codex --version` separately if needed."
  mcp_servers: "Not enumerated in exec JSONL; individual MCP calls expose item.server and item.tool."
  permission_mode: "Not emitted in exec JSONL; wrapper must record supplied --sandbox, --ask-for-approval, --yolo, and effective config if it needs this."
  notes: "The app-server protocol and OTel logs expose richer metadata, but the flattened exec JSONL stream is intentionally smaller."
stream_events:
  - event: "thread.started"
    category: session
    fields: ["type", "thread_id"]
    notes: "First event for a new thread; thread_id is the resume handle."
  - event: "turn.started"
    category: session
    fields: ["type"]
    notes: "No turn id in exec JSONL."
  - event: "turn.completed"
    category: usage
    fields: ["type", "usage.input_tokens", "usage.cached_input_tokens", "usage.output_tokens", "usage.reasoning_output_tokens"]
    notes: "Terminal success event for completed turns; usage is total token usage from the last token usage notification."
  - event: "turn.failed"
    category: error
    fields: ["type", "error.message"]
    notes: "Terminal failure event when app-server marks the turn failed."
  - event: "error"
    category: error
    fields: ["type", "message"]
    notes: "Unrecoverable stream error notification; process can continue until turn completion/failure."
  - event: "item.started"
    category: other
    fields: ["type", "item.id", "item.type"]
    notes: "Starts in-progress command_execution, mcp_tool_call, collab_tool_call, web_search, and todo_list items."
  - event: "item.updated"
    category: plan
    fields: ["type", "item.id", "item.type", "item.items"]
    notes: "Currently important for todo_list plan updates."
  - event: "item.completed"
    category: other
    fields: ["type", "item.id", "item.type"]
    notes: "Terminal item event for messages, reasoning, tools, file changes, errors, and final todo list."
  - event: "item.type=agent_message"
    category: assistant
    fields: ["item.id", "item.text"]
    notes: "Completed assistant message. The final answer is the last completed agent_message."
  - event: "item.type=reasoning"
    category: reasoning
    fields: ["item.id", "item.text"]
    notes: "Reasoning summary text, not raw hidden reasoning."
  - event: "item.type=command_execution"
    category: tool_call
    fields: ["item.id", "item.command", "item.aggregated_output", "item.exit_code", "item.status"]
    notes: "Represents shell/command execution. Output is aggregated, not stdout/stderr-separated."
  - event: "item.type=file_change"
    category: file_change
    fields: ["item.id", "item.changes[].path", "item.changes[].kind", "item.status"]
    notes: "Emitted after patch success/failure; no per-hunk content."
  - event: "item.type=mcp_tool_call"
    category: tool_call
    fields: ["item.id", "item.server", "item.tool", "item.arguments", "item.result.content", "item.result.structured_content", "item.result._meta", "item.error.message", "item.status"]
    notes: "MCP call start and completion are visible, including arguments and result/error payload."
  - event: "item.type=collab_tool_call"
    category: subagent
    fields: ["item.id", "item.tool", "item.sender_thread_id", "item.receiver_thread_ids", "item.prompt", "item.agents_states", "item.status"]
    notes: "Subagent orchestration tool. Nested child tool calls are not flattened into the parent stream unless surfaced as collab state/messages."
  - event: "item.type=web_search"
    category: tool_call
    fields: ["item.id", "item.query", "item.action"]
    notes: "Search action is preserved from WebSearchAction when available."
  - event: "item.type=todo_list"
    category: plan
    fields: ["item.id", "item.items[].text", "item.items[].completed"]
    notes: "Plan/todo state starts, updates, then completes at turn end."
  - event: "item.type=error"
    category: error
    fields: ["item.id", "item.message"]
    notes: "Non-fatal warnings, config warnings, deprecation notices, and model reroute messages are flattened as error items."
tools:
  - name: "shell command / command_execution"
    call_visible: true
    result_visible: true
    metadata: ["command", "aggregated_output", "exit_code", "status"]
    notes: "Start is visible before execution completes. stdout/stderr are combined into aggregated_output."
  - name: "apply_patch / file_change"
    call_visible: false
    result_visible: true
    metadata: ["changes[].path", "changes[].kind", "status"]
    notes: "File changes are dedicated completed items after patch success/failure; no patch body or hunk details."
  - name: "MCP tools"
    call_visible: true
    result_visible: true
    metadata: ["server", "tool", "arguments", "result.content", "result.structured_content", "result._meta", "error.message", "status"]
    notes: "MCP OAuth/login setup is outside the exec stream; configured required MCP startup failure exits with an error."
  - name: "web_search"
    call_visible: true
    result_visible: true
    metadata: ["query", "action"]
    notes: "Docs say web_search items appear in transcripts or codex exec --json output."
  - name: "todo_list / plan"
    call_visible: true
    result_visible: true
    metadata: ["items[].text", "items[].completed"]
    notes: "Plan updates are item.updated events, then completed at turn end."
  - name: "collab_tool_call / subagents"
    call_visible: true
    result_visible: true
    metadata: ["tool", "sender_thread_id", "receiver_thread_ids", "prompt", "agents_states", "status"]
    notes: "Covers spawn_agent, send_input, wait, and close_agent orchestration."
completion:
  success_event: "turn.completed"
  failure_event: "turn.failed or top-level error followed by non-zero process exit"
  exit_code_reliable: true
  result_fields: ["item.type=agent_message item.text", "turn.completed.usage"]
  cost_fields: []
  usage_fields: ["turn.completed.usage.input_tokens", "turn.completed.usage.cached_input_tokens", "turn.completed.usage.output_tokens", "turn.completed.usage.reasoning_output_tokens"]
  notes: "Source tracks fatal server errors and failed/interrupted turns for non-zero process status. The TypeScript SDK also treats non-zero exit or signal as failure and includes stderr in the thrown error."
blocking_behavior:
  permissions: fail
  questions: fail
  tool_approvals: fail
  notes: "exec defaults to approval_policy=never. MCP elicitation is auto-canceled. Command, file-change, apply_patch, exec-command, permission, request_user_input, dynamic-tool, auth-refresh, attestation, and current-time server requests are rejected as unsupported in exec mode."
subagents:
  supported: true
  start_visible: true
  stop_visible: true
  nested_events_visible: false
  prompt_injection_supported: true
  metadata_fields: ["item.type=collab_tool_call", "item.tool", "item.sender_thread_id", "item.receiver_thread_ids", "item.prompt", "item.agents_states.<thread_id>.status", "item.agents_states.<thread_id>.message", "item.status"]
  notes: "Subagents are enabled by default and only spawn when explicitly requested. Non-interactive approvals that cannot surface fail and report back to the parent workflow. Parent prompts can include non-interactive instructions for spawned agents."
use_cases:
  - name: plan_cap_approaching
    detectable: false
    event_types: []
    fields: []
    hook_parity: "unknown"
    notes: "No plan/quota approaching-cap event found in exec JSONL."
  - name: plan_capped
    detectable: false
    event_types: ["error", "turn.failed"]
    fields: ["message", "error.message"]
    hook_parity: "unknown"
    notes: "Quota/rate/billing errors may appear as generic messages, but no structured cap/reset fields are exposed."
  - name: no_funds
    detectable: false
    event_types: ["error", "turn.failed"]
    fields: ["message", "error.message"]
    hook_parity: "unknown"
    notes: "Insufficient credits can only be classified from provider error text unless a future event adds structured billing fields."
  - name: auth
    detectable: true
    event_types: ["error", "turn.failed", "process_exit"]
    fields: ["message", "error.message", "stderr"]
    hook_parity: "no"
    notes: "Missing/invalid auth may fail before thread.started and appear on stderr with non-zero exit; exec JSONL does not expose auth kind."
  - name: permission_read_denied
    detectable: true
    event_types: ["item.completed", "turn.failed"]
    fields: ["item.type=command_execution item.status", "item.aggregated_output", "item.exit_code", "error.message"]
    hook_parity: "partial: permission hooks/app-server expose richer fields than exec JSONL"
    notes: "Filesystem deny details are generally command output or error text, not a dedicated read-denied event."
  - name: permission_write_denied
    detectable: true
    event_types: ["item.completed", "turn.failed"]
    fields: ["item.type=file_change item.status", "item.changes[].path", "item.type=command_execution item.aggregated_output", "error.message"]
    hook_parity: "partial: permission hooks/app-server expose richer fields than exec JSONL"
    notes: "File-change failure gives paths and failed status; shell write denials appear in command output."
  - name: tokens_consumed
    detectable: true
    event_types: ["turn.completed"]
    fields: ["usage.input_tokens", "usage.cached_input_tokens", "usage.output_tokens", "usage.reasoning_output_tokens"]
    hook_parity: "OTel sse_event can include token counts on response.completed"
    notes: "Units are tokens. The exec event reports total usage for the completed turn."
  - name: model_used
    detectable: false
    event_types: []
    fields: []
    hook_parity: "OTel conversation_starts includes model"
    notes: "Exec JSONL does not emit model; wrapper must record --model/effective config or use OTel/app-server metadata."
  - name: model_fallback
    detectable: true
    event_types: ["item.completed"]
    fields: ["item.type=error", "item.message"]
    hook_parity: "unknown"
    notes: "Model reroute is flattened to an error item message like 'model rerouted: from -> to (reason)'; parse as best-effort text, not a typed field."
  - name: human_in_loop
    detectable: true
    event_types: ["error", "turn.failed", "process_exit"]
    fields: ["message", "error.message", "stderr"]
    hook_parity: "app-server server requests and hooks are richer"
    notes: "exec rejects or cancels human-input surfaces; classify messages containing unsupported approval/request_user_input/elicitation text."
  - name: session_resumable
    detectable: true
    event_types: ["thread.started"]
    fields: ["thread_id"]
    hook_parity: "unknown"
    notes: "thread_id arrives first and can be used with codex exec resume <SESSION_ID> unless the run used --ephemeral."
  - name: subagent_prompt_injection
    detectable: true
    event_types: ["item.started", "item.completed"]
    fields: ["item.type=collab_tool_call", "item.prompt"]
    hook_parity: "subagentStart/subagentStop hooks exist outside exec JSONL"
    notes: "Parent prompt can explicitly instruct subagents; collab tool call exposes prompt when present."
headless_constraints:
  - constraint: "No formal JSON Schema for exec JSONL."
    mitigation: "Generate parser types from codex-rs/exec/src/exec_events.rs or SDK TypeScript unions and preserve unknown events."
    notes: "App-server schema is formal but broader and not identical to exec JSONL."
  - constraint: "Exec JSONL does not emit model, provider, auth kind, cwd, sandbox, approval policy, version, or MCP server inventory."
    mitigation: "Wrapper should record invocation flags, selected config/profile, cwd, environment, and `codex --version` beside the stream."
    notes: "OTel/app-server can expose richer metadata but is a separate integration."
  - constraint: "Interrupted turns may not produce turn.failed."
    mitigation: "Treat process signal/non-zero exit and missing terminal event as cancellation/ambiguous failure."
    notes: "Source marks interrupted turns as error_seen for exit status but the JSONL projector initiates shutdown without emitting turn.failed."
  - constraint: "Approval and user-input requests cannot be answered in exec mode."
    mitigation: "Use approval_policy=never, preconfigure permissions, avoid prompt_tool-style workflows, and parse rejection messages as human-in-loop attempts."
    notes: "MCP elicitation is auto-canceled; approval requests fail closed."
  - constraint: "Command output is aggregated."
    mitigation: "Do not promise stdout/stderr separation for shell tools from exec JSONL."
    notes: "Use app-server command execution deltas if separate streams are required."
  - constraint: "Project config and hooks load only when project is trusted."
    mitigation: "For deterministic automation, use --ignore-user-config/--ignore-rules or explicit -c overrides, and record trust assumptions."
    notes: "Project-local provider/auth redirection keys are ignored even when trusted."
quirks:
  - "The stable user-facing flag is --json, but the TypeScript SDK still invokes --experimental-json as an alias."
  - "thread.started.thread_id is the only session id in exec JSONL; there is no separate turn id."
  - "Model reroute is not a typed model_fallback event; it is flattened into an item.type=error message."
  - "File changes are summarized by path and add/delete/update kind only; no patch text is included."
  - "MCP tool result uses structured_content in exec JSONL, while MCP protocol names the field structuredContent in some broader schemas."
  - "Reasoning items contain summaries/text, not hidden reasoning."
  - "JSONL item ids are synthesized by the exec projector and are not necessarily raw app-server item ids."
  - "Without --json, stdout is intentionally only the final assistant message, which is good for pipes but poor for lifecycle supervision."
gaps:
  - "No official, versioned JSON Schema for codex exec --json was found."
  - "Exact stderr text and exit codes for every auth, quota, rate-limit, and billing failure were not exhaustively fixture-tested."
  - "Cost fields are not exposed in exec JSONL."
  - "The exact local cache path for cloud-managed config/requirements was not verified."
  - "Nested subagent child tool events were not verified as visible in the parent exec JSONL stream; source exposes collab state in parent events."
  - "No timestamp fields were found in exec JSONL events."
claudine_strategy:
  preferred_invocation: 'codex exec --json --sandbox workspace-write --skip-git-repo-check "PROMPT"'
  required_flags: ["exec", "--json", "--sandbox <mode chosen by Claudine>", "--color never when wrapping human stderr"]
  conflicting_flags: ["no --json for live parsing", "--ephemeral when Claudine needs resume", "--yolo unless the outer runner is isolated", "codex app-server for simple one-shot jobs"]
  parser_notes: "Parse stdout as JSONL with top-level type discriminator and nested item.type. Use thread.started.thread_id for resume, last item.completed agent_message for final text, turn.completed usage for tokens, and turn.failed/error/non-zero exit for failure. Preserve unknown events."
  wrapper_notes: "Capture stderr, process exit status, signal, `codex --version`, cwd, selected config/profile, sandbox/approval flags, and environment auth source separately because exec JSONL omits much of that metadata."
data_format: jsonl
changes:
  - "2026-07-03: Refreshed Codex CLI non-interactive research from current OpenAI Codex manual, local codex-cli 0.142.5 help, and openai/codex source event types."
requires_claudine_update: true
reason: "Claudine should prefer codex exec --json and parse the current ThreadEvent/item.type JSONL contract; wrappers also need side-channel capture for metadata omitted from exec JSONL."
---

# Codex CLI Non-Interactive Sessions

## Summary

Codex CLI can run non-interactively with `codex exec`. For Claudine, the preferred local wrapper mode is `codex exec --json`, because it turns stdout into a live JSON Lines stream with session, turn, item, tool, file-change, plan, error, and usage events. Plain `codex exec` is useful for shell pipelines because stdout contains only the final assistant message, but that mode hides the operational state Claudine needs while a run is active.

The main parser risk is that the `exec --json` stream is not published as a formal JSON Schema. Its best schema source is the Rust Serde union in `codex-rs/exec/src/exec_events.rs`, plus the JSONL projection code in `event_processor_with_jsonl_output.rs` and the TypeScript SDK event/item unions. The stream is intentionally smaller than the broader app-server protocol: it does not emit model, provider, auth kind, cwd, sandbox, approval policy, version, full MCP inventory, timestamps, or cost. Claudine should parse stdout JSONL and separately capture invocation/config metadata, stderr, process exit status, and `codex --version`.

## Non-Interactive Entry Points

The official non-interactive entry point is `codex exec`. It starts the agent without opening the TUI, accepts a prompt as an argv argument, reads stdin when no prompt is supplied or when the prompt is `-`, and treats piped stdin plus a prompt argument as additional context. The local CLI observed on this host was `codex-cli 0.142.5`, and `codex exec --help` confirmed the documented flags: `--json`, `--output-schema`, `--output-last-message`, `--sandbox`, `--cd`, `--add-dir`, `--image`, `--profile`, `--config`, `--ignore-user-config`, `--ignore-rules`, `--skip-git-repo-check`, `--ephemeral`, and `resume`.

Typical invocations:

| Purpose | Command shape | Notes |
| --- | --- | --- |
| Fresh local run | `codex exec --json "prompt"` | Best Claudine default for live parsing. |
| Prompt from stdin | `codex exec --json -` | stdin is the prompt. |
| Prompt plus context | `producer | codex exec --json "instruction"` | piped content is appended as context. |
| Resume latest | `codex exec --json resume --last "prompt"` | Requires persisted session state. |
| Resume by id | `codex exec --json resume <SESSION_ID> "prompt"` | `SESSION_ID` is `thread.started.thread_id`. |
| Review | `codex exec review --json` | Uses the exec stream for a code review workflow. |

Codex also has programmatic surfaces that are adjacent but not the right default for a one-shot Claudine local run. `codex app-server` exposes a bidirectional JSON-RPC-like protocol over stdio, WebSocket, or Unix socket. It is richer, can generate schemas, and is appropriate for a product integration that wants to manage threads, turns, approvals, and cancellation directly. For a wrapper around an autonomous process, `exec --json` is simpler and already handles a complete run lifecycle. `codex cloud list --json` is scriptable JSON, but it lists cloud tasks rather than running a local agent session.

Attachments and configuration are available in `exec`: `--image` attaches images; `--cd` changes the working directory; `--add-dir` adds writable roots; `--model`, `--profile`, and `-c key=value` influence model/provider/reasoning/tools; MCP servers are configured through `config.toml`; and `--sandbox`/approval settings control tool permissions.

## Output Formats

Codex has several output modes with different wrapper value:

| Format | Selector | Framing | Streams? | Claudine preference |
| --- | --- | --- | --- | --- |
| Default text | no `--json` | text | stderr progress, stdout final text | Avoid for supervision; useful only for final-message pipes. |
| Exec JSONL | `--json` | JSONL on stdout | yes | Prefer. |
| Exec JSONL alias | `--experimental-json` | JSONL on stdout | yes | Same behavior today; use `--json` in new wrappers. |
| Final-message file | `-o FILE` / `--output-last-message FILE` | text file | no | Optional side sink. |
| Schema-constrained final answer | `--output-schema FILE` | final answer is model-produced JSON text | no | Useful for final artifact shape, not stream telemetry. |
| App-server stdio | `codex app-server --listen stdio://` | JSON-RPC lines | yes, bidirectional | Use only for deep integrations. |
| App-server WebSocket | `codex app-server --listen ws://...` | WebSocket text frames | yes, bidirectional | Experimental; not default. |
| Cloud list JSON | `codex cloud list --json` | single JSON | no | Separate cloud task listing use case. |

The official docs state that default `codex exec` streams progress to stderr and prints only the final agent message to stdout. That is good Unix behavior, but weak wrapper telemetry. With `--json`, stdout becomes JSON Lines and captures every event the exec projector emits while the run is active. That lets Claudine render tool progress, detect file changes, classify turn failure, collect token usage, and obtain the resume id before process exit.

`--output-schema` is easy to misread. It asks the model to make the final assistant response conform to a caller-supplied JSON Schema. It does not define or validate the event stream. In `--json` mode, the final structured answer appears as text inside an `item.completed` event whose nested item is `type: "agent_message"`.

The app-server stream is a different API style. It is a request/reply plus notification protocol with `method`, `params`, and `id`, and the client must answer some server requests. Its richer protocol is useful context, but Claudine should not use it for ordinary one-shot runs unless it wants to become a full Codex client.

## Schema Sources

No formal JSON Schema for `codex exec --json` was found. The authoritative stream shape is the Rust source:

| Source | Evidence type | Usefulness |
| --- | --- | --- |
| [`codex-rs/exec/src/exec_events.rs`](https://github.com/openai/codex/blob/main/codex-rs/exec/src/exec_events.rs) | Rust Serde types | Best schema for top-level `ThreadEvent` and nested `ThreadItemDetails`. |
| [`codex-rs/exec/src/event_processor_with_jsonl_output.rs`](https://github.com/openai/codex/blob/main/codex-rs/exec/src/event_processor_with_jsonl_output.rs) | Rust projection logic | Explains which app-server notifications become exec JSONL events and which are dropped. |
| [`sdk/typescript/src/events.ts`](https://github.com/openai/codex/blob/main/sdk/typescript/src/events.ts) and [`items.ts`](https://github.com/openai/codex/blob/main/sdk/typescript/src/items.ts) | TypeScript unions | Good consumer-facing typed examples for SDK parsers. |
| [Non-interactive mode](https://developers.openai.com/codex/noninteractive) | Official examples | Documents `--json`, event families, and sample JSONL. |
| [Codex App Server](https://developers.openai.com/codex/app-server) | Formal app-server schema generation | Useful for broader protocol context, not equivalent to exec JSONL. |

The Rust event enum uses `#[serde(tag = "type")]`, so the top-level discriminator is `type`. Nested items also use `type`, with snake_case item variants such as `agent_message`, `command_execution`, `file_change`, `mcp_tool_call`, `collab_tool_call`, `web_search`, `todo_list`, and `error`.

The app-server protocol can generate TypeScript and JSON Schema for its own methods and notifications. That schema is formal and version-specific, but it is not the flattened `exec --json` schema. Claudine should treat it as a secondary reference when deciding whether a missing field might exist in a richer integration.

## IO Contract

`codex exec` has a clean stdio contract. Source comments in the exec crate say default mode reserves stdout for the final message, JSON mode reserves stdout for valid JSONL, and all other output belongs on stderr. That is exactly what a wrapper needs: with `--json`, stdout can be parsed line-by-line, and stderr can be captured as diagnostics.

stdin is prompt input, not a bidirectional protocol. If a prompt argument is absent or `-`, Codex reads stdin as the prompt. If stdin is piped while a prompt argument is present, the prompt remains the instruction and stdin becomes additional context. In contrast, `app-server` uses stdin as a bidirectional JSON-RPC line transport and requires an initialize handshake.

stderr is not structured, but it is operationally important. Startup/config/auth failures can happen before `thread.started`, and the TypeScript SDK includes stderr text when the child exits non-zero. Claudine should keep stderr attached to the run record and use it for failure classification when the JSONL stream has no terminal event.

## Stream Contract

The top-level `type` values are:

| Event | Meaning |
| --- | --- |
| `thread.started` | New thread/session started; contains `thread_id`. |
| `turn.started` | A prompt was submitted and a turn began. |
| `item.started` | An in-progress item became visible. |
| `item.updated` | An item changed, especially `todo_list`. |
| `item.completed` | An item reached a terminal state. |
| `turn.completed` | The turn completed successfully; includes token usage. |
| `turn.failed` | The turn failed; includes `error.message`. |
| `error` | Unrecoverable stream/server error message. |

For a new thread, `thread.started` is documented in source as the first event, and the TypeScript SDK updates `thread.id` immediately when it sees that event. `turn.completed` and `turn.failed` are the normal terminal stream events. One caveat from source: an interrupted turn initiates shutdown and contributes to non-zero exit handling, but the JSONL projector does not emit `turn.failed` for `TurnStatus::Interrupted`. Claudine should treat process signal, non-zero exit, or EOF without terminal event as cancellation or ambiguous failure.

Tool and item correlation is by `item.id` in the flattened stream. The projector maps raw app-server item ids to synthetic `item_N` ids, keeps the same id across started/completed while an item is in progress, then removes the mapping. There is no top-level turn id in exec JSONL.

Assistant messages are not token deltas. The final answer is the last completed `agent_message` item. Reasoning appears only as summary text in completed `reasoning` items. Command output is aggregated into one `aggregated_output` field rather than split into stdout/stderr channels.

## Session Metadata

The only reliable session metadata in exec JSONL is:

| Metadata | Field | Notes |
| --- | --- | --- |
| Resume/session id | `thread.started.thread_id` | Arrives first; usable with `codex exec resume <SESSION_ID>`. |
| Token usage | `turn.completed.usage.*` | Input, cached input, output, and reasoning output tokens. |
| MCP server/tool for a call | `item.server`, `item.tool` on `mcp_tool_call` | Per-call only, not inventory. |
| Subagent ids/state | `collab_tool_call.receiver_thread_ids`, `agents_states` | Parent orchestration state, not full child transcript. |

Important metadata is absent from exec JSONL: cwd, project root, git branch, model, provider, auth source, CLI version, sandbox mode, approval policy, full MCP server list, roots, terminal size, timestamps, and cost. Some of this is available elsewhere: OTel `conversation_starts` includes model and sandbox/approval settings, app-server exposes richer config and thread structures, and the wrapper already knows invocation flags. For Claudine, the pragmatic strategy is to record wrapper-side metadata beside the JSONL stream.

## Event Families

`item.completed` is a broad event; the nested `item.type` is the real operational category:

| `item.type` | Start/update/completion behavior | Key fields |
| --- | --- | --- |
| `agent_message` | completed only | `text` |
| `reasoning` | completed only | `text` |
| `command_execution` | started and completed | `command`, `aggregated_output`, `exit_code`, `status` |
| `file_change` | completed after patch succeeds/fails | `changes[].path`, `changes[].kind`, `status` |
| `mcp_tool_call` | started and completed | `server`, `tool`, `arguments`, `result`, `error`, `status` |
| `collab_tool_call` | started and completed | `tool`, `sender_thread_id`, `receiver_thread_ids`, `prompt`, `agents_states`, `status` |
| `web_search` | started and completed | `query`, `action` |
| `todo_list` | started, updated, completed | `items[].text`, `items[].completed` |
| `error` | completed | `message` |

Warnings, config warnings, deprecation notices, and model reroutes are flattened into `item.type=error`. A model reroute message currently looks like prose containing the source model, destination model, and reason; it is not a typed `model_fallback` event.

## Tools

Shell command execution is visible before and after completion. Claudine can show `item.started` with `command_execution.status=in_progress`, then update final status from `item.completed`. The command result includes an exit code when available and combined output in `aggregated_output`.

File edits are visible as `file_change` items after the patch succeeds or fails. The stream includes path and add/delete/update kind, but not hunk text. If Claudine needs an exact diff, it must run `git diff` or inspect files separately.

MCP tool calls include server name, tool name, arguments, and either result content/structured content/meta or an error message. Required MCP server startup failures are documented to make `codex exec` exit with an error instead of silently continuing without the server. MCP OAuth setup is not solved mid-run by exec JSONL; configure MCP authentication before automation.

Web search appears as `web_search` items when used. Plan state appears as `todo_list`, not as a separate top-level plan event. Subagents are represented as `collab_tool_call` items with tool names such as spawn/send/wait/close and maps of child agent states.

## Completion and Exit Status

For normal success, trust `turn.completed` and collect the last `agent_message` as final text. Token usage is on `turn.completed.usage`:

```json
{"type":"turn.completed","usage":{"input_tokens":24763,"cached_input_tokens":24448,"output_tokens":122,"reasoning_output_tokens":0}}
```

For failure, trust `turn.failed` when present, and also watch top-level `error` events. Source code tracks fatal server errors and failed/interrupted turns so that automation gets a non-zero process status. The TypeScript SDK also treats non-zero exit or signal as failure and includes captured stderr in the thrown error. That makes exit code reliable as process-level failure signaling, but not sufficient as the only classifier: stderr and stream events carry the human-readable reason.

Cancellation/interruption is the important caveat. Source sets error state for interrupted turns, but the JSONL projection does not emit a `turn.failed` event for `TurnStatus::Interrupted`; it just initiates shutdown. Claudine should classify EOF without `turn.completed`/`turn.failed`, a signal, or non-zero exit as canceled or ambiguous depending on the process status.

## Blocking Behavior

`codex exec` is designed not to wait for a human TTY. The exec config path defaults approval policy to `never` for headless mode. Source-level request handling then rejects approval and user-input requests that would otherwise need a human:

| Request | Exec behavior |
| --- | --- |
| MCP elicitation | Resolves with cancel. |
| Command execution approval | Rejects as unsupported in exec mode. |
| File-change approval | Rejects as unsupported in exec mode. |
| `request_user_input` | Rejects as unsupported in exec mode. |
| Dynamic tool call request | Rejects as unsupported in exec mode. |
| ChatGPT auth token refresh request | Rejects as unsupported in exec mode. |
| Apply patch / exec command approval | Rejects as unsupported in exec mode. |
| Permissions request approval | Rejects as unsupported in exec mode. |

This is favorable for automation: the run should fail closed rather than hang for a prompt. It also means Claudine must preconfigure sandbox and permissions. If a workflow requires human approval, use an interactive Codex surface or the app-server protocol with a client that can answer server requests.

Authentication is separate. If no usable login or API key exists, `exec` can fail before a thread starts. For CI, official docs recommend setting `CODEX_API_KEY` only for the single `codex exec` invocation, or using trusted ChatGPT-managed auth/access-token workflows when API-key billing is not the desired source.

## Subagents

Codex subagents are supported in current CLI releases and are enabled by default. They do not spawn automatically; the user must explicitly ask for subagents or parallel agent work. In non-interactive flows, an action that needs a fresh approval fails and is surfaced back to the parent workflow.

In the exec JSONL stream, parent-visible subagent activity appears through `collab_tool_call` items. These include the collaboration tool, sender thread id, receiver thread ids, optional prompt, per-agent state map, and status. This is enough to know that a subagent was spawned, waited on, or closed, and to see high-level child state. It is not proof that every nested child tool call is replayed into the parent stream. Claudine should treat nested child tool visibility as a gap unless fixture evidence proves otherwise.

The caller can steer subagents by prompt: because Codex only starts subagents when explicitly requested, Claudine can include non-interactive constraints in the parent prompt, such as requiring child agents to avoid prompts and return concise summaries. Custom agents can also be configured under `~/.codex/agents/` or trusted project `.codex/agents/`.

## Use Case Detection

| Use case | Detectable from exec JSONL? | Detection |
| --- | --- | --- |
| `session_resumable` | Yes | `thread.started.thread_id`, unless `--ephemeral` prevents persisted resume. |
| `tokens_consumed` | Yes | `turn.completed.usage.*`, units are tokens for the completed turn. |
| `model_used` | No | Capture `--model`, config/profile, or OTel/app-server metadata outside exec JSONL. |
| `model_fallback` | Partial | Parse `item.type=error` message beginning with model reroute text; no typed fields. |
| `auth` | Partial | Startup stderr, top-level `error`, `turn.failed.error.message`, non-zero exit. |
| `plan_cap_approaching` | No | No structured plan/quota cap event found. |
| `plan_capped` | Partial | Generic provider error text only; no reset/window fields. |
| `no_funds` | Partial | Generic billing/quota error text only. |
| `permission_read_denied` | Partial | Command failure/output or `turn.failed`; no dedicated read-denied event. |
| `permission_write_denied` | Partial | Failed `file_change`, command output, or `turn.failed`. |
| `human_in_loop` | Yes | Unsupported approval/input/elicitation messages, failed request handling, or non-zero exit. |
| `subagent_prompt_injection` | Yes | `collab_tool_call.prompt` and parent prompt content. |

No timestamps are present in exec JSONL, so timezone/window fields for quotas or reset times cannot be extracted unless an error message includes them as prose.

## Headless Constraints

The most important constraint is metadata absence. `exec --json` is good for live progress, but it is not a complete run manifest. Claudine should wrap it with its own manifest: executable path/version, cwd, prompt source, config/profile, model override, sandbox/approval flags, environment auth source, start/end timestamps, process id, exit code, signal, and stderr.

The second constraint is permission determinism. Exec will not ask a human. That is good, but a misconfigured task can fail instead of pausing. Use explicit `--sandbox`, approval policy, MCP tool approval modes, and project trust assumptions. Avoid `--yolo` unless the outer runner is isolated.

The third constraint is schema drift. Since there is no formal exec JSON Schema, the parser should be tolerant: unknown top-level `type` or nested `item.type` should be logged and preserved, not fatal. Required behavior should be based on the documented/core events: `thread.started`, item events, `turn.completed`, `turn.failed`, and process status.

## Timeline

| Date | Evidence |
| --- | --- |
| 2026-07-03 | Fetched current Codex manual through the OpenAI docs skill helper; non-interactive docs describe `codex exec`, `--json`, JSONL events, stdin behavior, permissions, auth, and resume. |
| 2026-07-03 | Locally observed `codex-cli 0.142.5`; `codex exec --help` confirmed current CLI flags and prompt/stdin behavior. |
| 2026-07-03 | Cloned `openai/codex` and inspected `codex-rs/exec` plus TypeScript SDK event parsing for source-level schema and blocking behavior. |

## Quirks and Gaps

`--json` and `--experimental-json` currently select the same stream, but new wrappers should use the stable documented spelling. The SDK still uses the alias internally, so parsers may see examples using either.

The stream is flattened from app-server notifications. Some broader app-server events are intentionally ignored by the JSONL projector, including hook started/completed and several metadata-rich notifications. That keeps `exec` simple, but it means Claudine should not assume absence from JSONL means absence from Codex internally.

Known gaps remain: no formal exec JSON Schema, no cost fields, no timestamps, no structured quota reset fields, no exhaustive fixture matrix for auth/rate-limit/billing stderr, and no proof that nested subagent child tool calls are visible in the parent exec stream.

## Claudine Integration Notes

Recommended default:

```bash
codex exec --json --sandbox workspace-write "PROMPT"
```

Add `--skip-git-repo-check` only when Claudine intentionally runs outside a Git repository. Add `--output-schema` only when the final answer needs a schema; it does not replace event parsing. Avoid `--ephemeral` when resume/recovery matters. Avoid `--yolo` unless Claudine runs Codex inside a separately hardened container, VM, or throwaway workspace.

Parser rules:

- Parse stdout as JSONL and treat each line as one event.
- Use top-level `type` as the discriminator and nested `item.type` for item families.
- Capture `thread.started.thread_id` immediately for resume/recovery.
- Show live tool progress from `item.started`, `item.updated`, and `item.completed`.
- Use the last completed `agent_message` as final text.
- Use `turn.completed.usage` for token usage.
- Treat `turn.failed`, top-level `error`, non-zero exit, signal, or EOF without a terminal event as failure/cancellation evidence.
- Preserve unknown events for drift analysis.

Wrapper rules:

- Capture stderr, even in JSON mode.
- Capture process exit code and signal.
- Capture `codex --version` separately.
- Record cwd, prompt source, selected model/config/profile, sandbox/approval flags, MCP config assumptions, and auth source separately.
- Keep app-server integration separate from the one-shot `exec --json` adapter. App-server is powerful, but it changes Claudine from a process wrapper into a protocol client.

## Changelog

- 2026-07-03: Refreshed the document from the current OpenAI Codex manual, local `codex-cli 0.142.5` help output, and current `openai/codex` source. Preserved the original `created` date and updated the recommended Claudine strategy to `codex exec --json`.

## Sources

- [Codex non-interactive mode](https://developers.openai.com/codex/noninteractive)
- [Codex CLI command reference](https://developers.openai.com/codex/cli/reference)
- [Codex config basics](https://developers.openai.com/codex/config-basic)
- [Codex advanced config](https://developers.openai.com/codex/config-advanced)
- [Codex environment variables](https://developers.openai.com/codex/environment-variables)
- [Codex managed configuration](https://developers.openai.com/codex/enterprise/managed-configuration)
- [Codex permissions](https://developers.openai.com/codex/permissions)
- [Codex MCP](https://developers.openai.com/codex/mcp)
- [Codex app-server](https://developers.openai.com/codex/app-server)
- [Codex subagents](https://developers.openai.com/codex/subagents)
- [openai/codex `exec_events.rs`](https://github.com/openai/codex/blob/main/codex-rs/exec/src/exec_events.rs)
- [openai/codex `event_processor_with_jsonl_output.rs`](https://github.com/openai/codex/blob/main/codex-rs/exec/src/event_processor_with_jsonl_output.rs)
- [openai/codex `exec` CLI flags](https://github.com/openai/codex/blob/main/codex-rs/exec/src/cli.rs)
- [openai/codex `exec` runtime and request handling](https://github.com/openai/codex/blob/main/codex-rs/exec/src/lib.rs)
- [OpenAI Codex TypeScript SDK events](https://github.com/openai/codex/blob/main/sdk/typescript/src/events.ts)
- [OpenAI Codex TypeScript SDK items](https://github.com/openai/codex/blob/main/sdk/typescript/src/items.ts)
- [OpenAI Codex TypeScript SDK exec wrapper](https://github.com/openai/codex/blob/main/sdk/typescript/src/exec.ts)
