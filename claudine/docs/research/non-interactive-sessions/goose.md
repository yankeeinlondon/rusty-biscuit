---
$schema: ./_schema.yaml
created: 2026-04-10
last_updated: 2026-07-02
agent: codex
model: default
docs: https://goose-docs.ai/docs/guides/running-tasks/
invocation:
  - command: "goose run --output-format stream-json -t \"<prompt>\""
    stdin_support: false
    prompt_arg: "--text/-t"
    notes: "Starts a fresh headless run unless --resume and an identifier are supplied."
  - command: "goose run --output-format stream-json -i -"
    stdin_support: true
    prompt_arg: "--instructions/-i -"
    notes: "Reads the complete prompt from stdin, then executes one non-interactive run."
  - command: "goose run --output-format stream-json -i <file>"
    stdin_support: false
    prompt_arg: "--instructions/-i <file>"
    notes: "Reads instructions from a file; useful for generated prompt files."
  - command: "goose run --output-format stream-json --recipe <recipe.yaml> --params key=value"
    stdin_support: false
    prompt_arg: "--recipe plus optional --params and --sub-recipe"
    notes: "Runs a recipe headlessly; the recipe must provide a prompt."
  - command: "goose run --output-format stream-json --resume --name <name> -t \"<prompt>\""
    stdin_support: false
    prompt_arg: "--text/-t"
    notes: "Resumes an existing stored session by name, session id, or legacy path."
  - command: "goose run --output-format stream-json --no-session -t \"<prompt>\""
    stdin_support: false
    prompt_arg: "--text/-t"
    notes: "Uses a null session path and discards the transcript after completion."
output_formats:
  - name: text
    cli_value: text
    stream: true
    format: text
    description: "Human terminal rendering with markdown, progress, and optional stats."
    side_effects: "Not parse-safe; stdout can contain prose, progress, ANSI styling, and tool display."
  - name: json
    cli_value: json
    stream: false
    format: json
    description: "One pretty-printed final JSON object with messages and metadata after the run finishes."
    side_effects: "Useful for batch logs, but Claudine cannot observe live progress before process exit."
  - name: stream-json
    cli_value: stream-json
    stream: true
    format: ndjson
    description: "One compact JSON event per stdout line as events occur; this is the format Claudine should prefer."
    side_effects: "Suppresses normal terminal rendering and emits message, notification, error, and complete events."
schema_sources:
  - url: https://github.com/aaif-goose/goose/blob/main/crates/goose-cli/src/session/mod.rs
    schema_type: rust
    formal: false
    notes: "Authoritative source for JsonOutput, JsonMetadata, StreamEvent, and NotificationData; not a published schema artifact."
  - url: https://github.com/aaif-goose/goose/blob/main/crates/goose-provider-types/src/conversation/message.rs
    schema_type: rust
    formal: false
    notes: "Authoritative Rust serde model for nested Message, MessageContent, tool, actionRequired, thinking, and metadata objects."
  - url: https://raw.githubusercontent.com/aaif-goose/goose/main/crates/goose-server/ui/desktop/openapi.json
    schema_type: openapi
    formal: true
    notes: "Formal server/Desktop API schema for nested message content and related objects; broader than the exact CLI stream envelope."
  - url: https://github.com/aaif-goose/goose/blob/main/ui/sdk/src/generated/types.gen.ts
    schema_type: typescript
    formal: true
    notes: "Generated SDK types from OpenAPI; useful corroboration for nested server types, not the CLI stdout envelope."
  - url: https://goose-docs.ai/docs/guides/running-tasks/
    schema_type: examples
    formal: false
    notes: "Official prose examples document the flags and high-level JSON/stream-json behavior."
cli_params:
  - flag: "--output-format"
    value: "text|json|stream-json"
    description: "Selects human text, final JSON, or streaming JSON events."
    example: "goose run --output-format stream-json -t \"summarize changes\""
  - flag: "-t, --text"
    value: "TEXT"
    description: "Prompt text passed on argv."
    example: "goose run -t \"fix the failing test\""
  - flag: "-i, --instructions"
    value: "FILE|-"
    description: "Instruction file path, or '-' to read stdin."
    example: "cat prompt.md | goose run --output-format stream-json -i -"
  - flag: "--recipe"
    value: "RECIPE_NAME or PATH"
    description: "Runs a recipe; conflicts with --text and --instructions."
    example: "goose run --output-format stream-json --recipe audit.yaml"
  - flag: "--params"
    value: "KEY=VALUE"
    description: "Supplies recipe parameters; can be repeated."
    example: "goose run --recipe deploy.yaml --params env=staging"
  - flag: "--sub-recipe"
    value: "RECIPE"
    description: "Includes additional sub-recipes for recipe execution."
    example: "goose run --recipe main.yaml --sub-recipe security.yaml"
  - flag: "--system"
    value: "TEXT"
    description: "Adds system instructions for non-recipe runs."
    example: "goose run --system \"Be concise\" -t \"review this repo\""
  - flag: "--provider"
    value: "PROVIDER"
    description: "Overrides the configured provider for this run."
    example: "goose run --provider anthropic --model claude-sonnet-4-5-20250929 -t \"...\""
  - flag: "--model"
    value: "MODEL"
    description: "Overrides the configured model for this run."
    example: "goose run --provider openai --model gpt-4.1 -t \"...\""
  - flag: "--with-builtin"
    value: "NAME[,NAME]"
    description: "Adds builtin extensions such as developer or computercontroller."
    example: "goose run --with-builtin developer -t \"edit files\""
  - flag: "--with-extension"
    value: "COMMAND"
    description: "Adds stdio extensions; can include leading ENV=value pairs."
    example: "goose run --with-extension \"API_KEY=... tool-server\" -t \"...\""
  - flag: "--with-streamable-http-extension"
    value: "URL [timeout=SECONDS]"
    description: "Adds a streamable HTTP extension for the run."
    example: "goose run --with-streamable-http-extension \"https://example/mcp timeout=100\" -t \"...\""
  - flag: "--no-profile"
    value: ""
    description: "Does not load default configured extensions; only CLI-specified extensions are used."
    example: "goose run --no-profile --with-builtin developer -t \"...\""
  - flag: "--no-session"
    value: ""
    description: "Runs without storing a session file; conflicts with --resume, --name, and --path."
    example: "goose run --no-session --output-format stream-json -t \"...\""
  - flag: "--resume, -r"
    value: ""
    description: "Resumes a stored session; can be combined with --name, --session-id, or legacy --path."
    example: "goose run --resume --name project-x -t \"continue\""
  - flag: "--name, -n"
    value: "NAME"
    description: "Names a new session or selects a session when used with --resume."
    example: "goose run -n project-x -t \"initial prompt\""
  - flag: "--session-id, --id"
    value: "SESSION_ID"
    description: "Resumes a specific session id; requires --resume."
    example: "goose run --resume --session-id 20260702_120000 -t \"continue\""
  - flag: "--max-turns"
    value: "NUMBER"
    description: "Limits turns without user input; also configurable as GOOSE_MAX_TURNS."
    example: "goose run --max-turns 20 -t \"...\""
  - flag: "--debug"
    value: ""
    description: "Shows complete tool responses, detailed parameters, and full paths in human rendering; structured message payloads still come from the same message model."
    example: "goose run --debug --output-format stream-json -t \"...\""
  - flag: "--quiet, -q"
    value: ""
    description: "Suppresses non-response output for text mode; do not rely on it as the structured-output selector."
    example: "goose run --quiet -t \"...\""
  - flag: "--stats"
    value: ""
    description: "Prints generation stats after text-mode completion; not needed with stream-json completion token fields."
    example: "goose run --stats -t \"...\""
  - flag: "--interactive, -s"
    value: ""
    description: "Continues in an interactive session after the initial input; avoid for Claudine headless execution."
    example: "goose run -i prompt.md --interactive"
config_files:
  - os: macos
    scope: user
    path: "~/.config/goose/config.yaml"
    format: yaml
    effect: "Persistent provider, model, mode, extensions, tool output, telemetry, and related settings."
    notes: "Environment variables override config file values; no documented persistent setting for run --output-format."
  - os: linux
    scope: user
    path: "~/.config/goose/config.yaml"
    format: yaml
    effect: "Persistent provider, model, mode, extensions, tool output, telemetry, and related settings."
    notes: "Environment variables override config file values; no documented persistent setting for run --output-format."
  - os: windows
    scope: user
    path: "%APPDATA%\\Block\\goose\\config\\config.yaml"
    format: yaml
    effect: "Persistent provider, model, mode, extensions, tool output, telemetry, and related settings."
    notes: "Environment variables override config file values; no documented persistent setting for run --output-format."
  - os: all
    scope: user
    path: "permission.yaml"
    format: yaml
    effect: "Tool permission levels managed by goose configure."
    notes: "Lives under the Goose config directory; influences whether tools require confirmation."
  - os: all
    scope: user
    path: "permissions/tool_permissions.json"
    format: json
    effect: "Runtime permission decisions managed by Goose."
    notes: "Auto-managed; exact path is under the Goose config directory."
  - os: all
    scope: user
    path: "secrets.yaml"
    format: yaml
    effect: "File-based API key and secret storage when keyring is disabled or unavailable."
    notes: "Goose prefers the system keyring; file fallback is relevant in CI and containers."
  - os: all
    scope: user
    path: "prompts/"
    format: text
    effect: "Custom prompt templates can affect agent and subagent behavior."
    notes: "Located under the Goose config directory."
  - os: all
    scope: repo
    path: ".goosehints"
    format: text
    effect: "Project context file loaded from cwd-derived context file names."
    notes: "Default context filename is .goosehints; CONTEXT_FILE_NAMES can add alternatives."
env_vars:
  - name: GOOSE_PROVIDER
    effect: "Selects the default provider when --provider is not supplied."
    notes: "Overrides config file setting."
  - name: GOOSE_MODEL
    effect: "Selects the default model when --model is not supplied."
    notes: "Overrides config file setting."
  - name: GOOSE_PROVIDER__TYPE
    effect: "Selects provider implementation/type for advanced provider configuration."
    notes: "Used with custom endpoints or enterprise deployments."
  - name: GOOSE_PROVIDER__HOST
    effect: "Overrides provider API endpoint."
    notes: "Can route a provider to custom or proxy endpoints."
  - name: GOOSE_PROVIDER__API_KEY
    effect: "Provides API-key auth for providers."
    notes: "Avoid logging; alternate auth can come from keyring or secrets file."
  - name: GOOSE_MODE
    effect: "Controls tool execution behavior: auto, approve, chat, or smart_approve."
    notes: "For headless automation, official docs recommend auto."
  - name: GOOSE_CONTEXT_STRATEGY
    effect: "Controls context limit behavior; headless default is summarize."
    notes: "Values include summarize, truncate, clear, and prompt."
  - name: GOOSE_MAX_TURNS
    effect: "Maximum turns allowed without user input."
    notes: "Equivalent operational concern to --max-turns; default documented as 1000."
  - name: GOOSE_SUBAGENT_MAX_TURNS
    effect: "Maximum turns a subagent can take before timeout."
    notes: "Can be overridden by recipe settings or subagent calls."
  - name: GOOSE_MAX_BACKGROUND_TASKS
    effect: "Limits concurrent background subagent tasks."
    notes: "Default documented as 5."
  - name: GOOSE_DISABLE_SESSION_NAMING
    effect: "Disables the extra background model call used to name sessions."
    notes: "Useful for CI/headless runs to reduce noise/cost."
  - name: GOOSE_DISABLE_TOOL_CALL_SUMMARY
    effect: "Disables the per-tool-call AI-generated summary title."
    notes: "Saves one provider call per tool invocation."
  - name: GOOSE_CLI_SHOW_THINKING
    effect: "Shows reasoning/thinking output in CLI responses where providers expose it."
    notes: "Structured streams may include nested thinking content when such messages are produced."
  - name: GOOSE_CLI_SHOW_COST
    effect: "Toggles cost estimates in CLI output."
    notes: "Human output setting; cost is not exposed as a stable stream-json field."
  - name: GOOSE_CLI_MIN_PRIORITY
    effect: "Controls tool output verbosity for CLI rendering."
    notes: "Can affect human-visible notification/log rendering; do not treat as a schema control."
  - name: GOOSE_DISABLE_KEYRING
    effect: "Can force file-based secret storage instead of OS keyring."
    notes: "Relevant for headless servers, CI, and containers."
  - name: GOOSE_ALLOWLIST
    effect: "URL for allowed extensions."
    notes: "Can constrain extension availability in automated runs."
  - name: CONTEXT_FILE_NAMES
    effect: "JSON array of project context filenames Goose loads."
    notes: "Default is [\".goosehints\"]."
io_contract:
  stdout: structured_only
  stderr: diagnostics_only
  stdin: prompt
  framing: ndjson
  noise_handling: "When --output-format stream-json is supplied, parse stdout line-by-line as JSON and treat stderr as diagnostics/fallback error context."
  notes: "The CLI prints stream events with serde_json::to_string and println; stdin is only prompt text via -i -, not a bidirectional protocol."
stream_contract:
  discriminator: "type"
  event_ordering: "Message and notification events are emitted as the agent stream yields them; error cancels processing; complete is emitted after the loop reaches finalization."
  correlation_fields:
    - "message.content[].id"
    - "message.content[].toolCall.name"
    - "extension_id"
  terminal_event: "complete"
  partial_message_events: false
  unknown_event_policy: "Skip unknown top-level events with a warning; preserve unknown nested message content for drift analysis."
  notes: "Outer event names are snake_case; nested message content uses camelCase type names and fields."
session_metadata:
  session_id: "Not emitted in the stream-json envelope; available only from invocation choice, stored session files, logs, or wrapper-side session selection."
  cwd: "Not emitted in stream-json; stored when the session is created but not present on stdout."
  model: "Nested message.metadata.inference.requestedModel/resolvedModel may appear on model-originated messages; no guaranteed init event."
  provider: "Nested message.metadata.inference.provider may appear; no guaranteed top-level provider field."
  auth: "Not emitted except through provider error text or configuration."
  version: "Not emitted; use goose --version out of band."
  mcp_servers: "Not listed at session start; extension notifications include extension_id when they occur."
  permission_mode: "Not emitted; effective GOOSE_MODE comes from env/config and must be captured by the wrapper."
  notes: "Goose run has no session_start/init record in stream-json, so Claudine must supplement launch metadata itself."
stream_events:
  - event: message
    category: assistant
    fields:
      - "message.role"
      - "message.created"
      - "message.content[]"
      - "message.metadata"
    notes: "Carries completed Message objects, including text, toolRequest, toolResponse, actionRequired, thinking, redactedThinking, systemNotification, and model inference metadata where present."
  - event: notification
    category: other
    fields:
      - "extension_id"
      - "message"
      - "progress"
      - "total"
    notes: "Flattened notification payload from MCP/server notifications; log notifications have message, progress notifications have progress/total/message."
  - event: error
    category: error
    fields:
      - "error"
    notes: "Stringified agent error emitted before cancellation and error return."
  - event: complete
    category: session
    fields:
      - "total_tokens"
      - "input_tokens"
      - "output_tokens"
    notes: "Terminal success event emitted after the stream loop; token fields are optional i32 values."
  - event: "message.content[].type=text"
    category: assistant
    fields:
      - "text"
    notes: "Assistant/user text content inside a message event."
  - event: "message.content[].type=toolRequest"
    category: tool_call
    fields:
      - "id"
      - "toolCall.name"
      - "toolCall.arguments"
      - "metadata"
      - "_meta"
    notes: "Tool call request; id is the primary join key when present."
  - event: "message.content[].type=toolResponse"
    category: tool_result
    fields:
      - "id"
      - "toolResult"
      - "metadata"
    notes: "Tool result or tool error; join to toolRequest by id."
  - event: "message.content[].type=toolConfirmationRequest"
    category: permission
    fields:
      - "id"
      - "toolName"
      - "arguments"
      - "prompt"
    notes: "Nested permission request shape; in current headless processing Goose handles confirmations before normal message emission."
  - event: "message.content[].type=actionRequired"
    category: permission
    fields:
      - "data.actionType"
      - "data.id"
      - "data.toolName"
      - "data.arguments"
      - "data.message"
      - "data.requestedSchema"
    notes: "Nested user-input or tool-confirmation action model; actionType values include toolConfirmation, elicitation, and elicitationResponse."
  - event: "message.content[].type=thinking"
    category: reasoning
    fields:
      - "thinking"
      - "signature"
    notes: "Reasoning content when provider and configuration expose it."
  - event: "message.content[].type=redactedThinking"
    category: reasoning
    fields:
      - "data"
    notes: "Redacted reasoning placeholder."
  - event: "message.content[].type=systemNotification"
    category: other
    fields:
      - "notificationType"
      - "msg"
      - "data"
    notes: "Nested system notification; notificationType includes thinkingMessage, inlineMessage, and creditsExhausted."
tools:
  - name: builtin developer extension
    call_visible: true
    result_visible: true
    metadata:
      - "toolRequest.id"
      - "toolRequest.toolCall.name"
      - "toolRequest.toolCall.arguments"
      - "toolResponse.id"
      - "toolResponse.toolResult"
    notes: "File reads, edits, shell commands, and command output are visible only as generic tool request/response payloads; no dedicated file_change event."
  - name: builtin computercontroller extension
    call_visible: true
    result_visible: true
    metadata:
      - "toolRequest.id"
      - "toolResponse.id"
      - "toolResponse.toolResult"
    notes: "Visible through the same nested tool content model."
  - name: stdio MCP extensions
    call_visible: true
    result_visible: true
    metadata:
      - "extension_id"
      - "notification.message"
      - "notification.progress"
      - "toolRequest.id"
      - "toolResponse.id"
    notes: "Tool calls/results are nested messages; MCP logging/progress notifications are flattened top-level notification events."
  - name: streamable HTTP extensions
    call_visible: true
    result_visible: true
    metadata:
      - "extension_id"
      - "notification.progress"
      - "notification.total"
      - "notification.message"
    notes: "Configured by --with-streamable-http-extension; no separate server-event stream is exposed by goose run."
  - name: subagent tasks
    call_visible: true
    result_visible: true
    metadata:
      - "notification.extension_id"
      - "notification.message"
      - "toolRequest.toolCall.name"
      - "toolResponse.toolResult"
    notes: "Subagent activity can appear as formatted notification strings and tool payloads, not as a dedicated structured subagent start/stop event family."
completion:
  success_event: complete
  failure_event: error
  exit_code_reliable: true
  result_fields:
    - "message.content[].text"
    - "json.messages"
    - "json.metadata.status"
  cost_fields: []
  usage_fields:
    - "complete.total_tokens"
    - "complete.input_tokens"
    - "complete.output_tokens"
    - "json.metadata.total_tokens"
    - "json.metadata.input_tokens"
    - "json.metadata.output_tokens"
  notes: "Use complete plus process exit 0 for success. Error events are string payloads and are followed by a non-zero command result in the current source path."
blocking_behavior:
  permissions: configurable
  questions: fail
  tool_approvals: configurable
  notes: "Headless Goose cannot collect elicitation input and cancels the run. In Approve/SmartApprove modes, a tool confirmation is an invalid non-interactive configuration and fails; in Auto mode current source auto-allows tool confirmations with a warning."
subagents:
  supported: true
  start_visible: false
  stop_visible: false
  nested_events_visible: true
  prompt_injection_supported: true
  metadata_fields:
    - "notification.message"
    - "toolRequest.toolCall.name"
    - "toolResponse.toolResult"
    - "GOOSE_SUBAGENT_MAX_TURNS"
  notes: "Goose supports subagents and sub-recipes, but the run stream does not expose normalized parent/child lifecycle records. Behavior can be steered through recipe/subagent prompts and prompt templates."
use_cases:
  - name: plan_cap_approaching
    detectable: false
    event_types: []
    fields: []
    hook_parity: "unknown"
    notes: "No structured plan/quota approaching event was verified in stream-json."
  - name: plan_capped
    detectable: true
    event_types:
      - "message.content[].type=systemNotification"
    fields:
      - "notificationType=creditsExhausted"
      - "msg"
      - "data"
    hook_parity: "unknown"
    notes: "Credits exhaustion is detectable as a nested system notification when emitted; quota reset/window fields are not guaranteed."
  - name: no_funds
    detectable: true
    event_types:
      - "message.content[].type=systemNotification"
      - error
    fields:
      - "notificationType=creditsExhausted"
      - "msg"
      - "error"
    hook_parity: "unknown"
    notes: "Detect creditsExhausted separately from generic provider error text."
  - name: auth
    detectable: true
    event_types:
      - error
    fields:
      - "error"
    hook_parity: "unknown"
    notes: "Auth failures are not typed in the CLI envelope; classify from provider error strings."
  - name: permission_read_denied
    detectable: false
    event_types:
      - "message.content[].type=toolResponse"
      - error
    fields:
      - "toolResult"
      - "error"
    hook_parity: "unknown"
    notes: "Read-denial semantics are tool/provider specific and not normalized."
  - name: permission_write_denied
    detectable: true
    event_types:
      - error
      - "message.content[].type=toolResponse"
    fields:
      - "error"
      - "toolResult"
    hook_parity: "unknown"
    notes: "Approve/SmartApprove headless confirmation failures are detectable from error text; tool-level denials require heuristics."
  - name: tokens_consumed
    detectable: true
    event_types:
      - complete
    fields:
      - "total_tokens"
      - "input_tokens"
      - "output_tokens"
    hook_parity: "unknown"
    notes: "Token fields are session totals from accumulated usage when available; fields are optional i32 values."
  - name: model_used
    detectable: true
    event_types:
      - "message"
    fields:
      - "message.metadata.inference.provider"
      - "message.metadata.inference.requestedModel"
      - "message.metadata.inference.resolvedModel"
    hook_parity: "unknown"
    notes: "Not guaranteed early; wrapper should also record --provider/--model and env/config values."
  - name: model_fallback
    detectable: true
    event_types:
      - "message"
    fields:
      - "message.metadata.inference.requestedModel"
      - "message.metadata.inference.resolvedModel"
    hook_parity: "unknown"
    notes: "Only detectable when inference metadata is present and requested/resolved values differ."
  - name: human_in_loop
    detectable: true
    event_types:
      - error
      - "message.content[].type=actionRequired"
      - "message.content[].type=toolConfirmationRequest"
    fields:
      - "error"
      - "data.actionType"
      - "prompt"
      - "requestedSchema"
    hook_parity: "unknown"
    notes: "Elicitation cancels in headless mode; tool confirmations fail in approval modes and auto-allow in Auto mode."
  - name: session_resumable
    detectable: false
    event_types: []
    fields: []
    hook_parity: "unknown"
    notes: "The stream lacks a session id; Claudine must know the chosen --name/--session-id or inspect Goose session storage out of band."
  - name: subagent_prompt_injection
    detectable: true
    event_types: []
    fields:
      - "--recipe"
      - "--sub-recipe"
      - "prompts/"
      - "GOOSE_SUBAGENT_MAX_TURNS"
    hook_parity: "unknown"
    notes: "Supported through authored recipe/subagent/prompt-template instructions, not a stream event."
headless_constraints:
  - constraint: "stream-json has no init/session_start event."
    mitigation: "Claudine should synthesize launch metadata from cwd, args, env, and goose --version."
    notes: "Do not wait for a session id on stdout."
  - constraint: "Approve and SmartApprove modes require interactive approval in headless runs."
    mitigation: "Set GOOSE_MODE=auto or preconfigure deterministic permissions before launch."
    notes: "Current source returns an error if a tool confirmation is needed in these modes."
  - constraint: "Auto mode auto-allows tool confirmations in headless mode."
    mitigation: "Use Claudine policy/protect controls before launch if stronger non-interactive safety is required."
    notes: "This is automation-friendly but should not be confused with sandboxing."
  - constraint: "Elicitation/user questions cannot be answered in headless mode."
    mitigation: "Provide detailed prompts and avoid MCP tools/recipes that require elicitation."
    notes: "Current source cancels when elicitation is requested in non-interactive mode."
  - constraint: "No dedicated file_change events."
    mitigation: "Infer file changes from developer tool calls/results or filesystem diffing around the process."
    notes: "Tool payloads are generic MCP call results."
  - constraint: "Cost is not in stream-json completion."
    mitigation: "Use tokens from complete and compute cost externally from model catalog where possible."
    notes: "GOOSE_CLI_SHOW_COST is a human rendering setting."
  - constraint: "Output format is not documented as persistently configurable."
    mitigation: "Always pass --output-format stream-json explicitly."
    notes: "Config/env can affect model, tools, mode, and verbosity."
quirks:
  - "The flag is named stream-json, but the wire format is NDJSON: one JSON object per line."
  - "Outer stream event names and notification payload keys are snake_case; nested message content type names and fields are camelCase."
  - "Top-level notification events flatten either log or progress data rather than nesting it under data."
  - "The broader OpenAPI/server schema is formal but is not the exact goose run stdout schema."
  - "The stream lacks a session id, cwd, provider version, permission mode, and tool inventory init event."
  - "Subagent notifications may be formatted strings rather than structured subagent lifecycle events."
  - "Tool confirmations in headless Auto mode are auto-allowed, while approval modes fail; this is easy to misclassify as simple auto-deny."
gaps:
  - "No provider-published JSON Schema or OpenAPI component for the exact goose run --output-format stream-json envelope."
  - "No verified stable schema-version marker for stream-json."
  - "No local captured run fixture was produced in this update because provider credentials and model behavior are environment-dependent."
  - "Exact exit code taxonomy for auth, rate limit, context overflow, max-turn, and cancellation is not documented separately from generic command failure."
  - "No structured cost field was verified in stream-json."
  - "No dedicated structured file-change, sandbox, roots, or permission-denial event was verified."
  - "No exact merge semantics for repo-scoped context files beyond documented config precedence and CONTEXT_FILE_NAMES were verified."
claudine_strategy:
  preferred_invocation: "goose run --output-format stream-json --no-session -i -"
  required_flags:
    - "--output-format stream-json"
    - "-i -"
    - "--no-session"
  conflicting_flags:
    - "--interactive"
    - "--output-format text"
    - "--output-format json"
  parser_notes: "Parse stdout as NDJSON by top-level type; parse nested message.content[].type separately; join toolRequest/toolResponse by content id; treat complete as terminal success only if process exit is zero."
  wrapper_notes: "Set or record GOOSE_MODE explicitly, prefer auto for deterministic headless runs, capture cwd/provider/model/env/config out of band, and use stderr only for diagnostics or fallback classification."
data_format: ndjson
changes:
  - "2026-07-02: Rewrote Goose non-interactive research against current aaif-goose docs and source; normalized frontmatter to the topic schema and added headless permission, token, and parser caveats."
requires_claudine_update: true
reason: "Goose metadata/parser generation should prefer stream-json NDJSON, recognize complete.input_tokens/output_tokens, synthesize missing init metadata, and handle current headless permission behavior."
---

# Goose CLI Non-Interactive Sessions

## Summary

Claudine can run Goose non-interactively through `goose run`. Goose documents three output formats for that command: human `text`, final `json`, and streaming `stream-json`. Claudine should prefer `goose run --output-format stream-json`, ideally with `-i -` for prompt stdin and `--no-session` for disposable automation runs. Despite the flag name, the stream is newline-delimited JSON: every stdout line is a complete JSON object.

The main parser risk is that Goose does not publish a formal schema for the exact CLI stream envelope. The outer events are defined in Rust source as `message`, `notification`, `error`, and `complete`; nested message payloads are better specified by Goose's Rust message types and the server OpenAPI/SDK types. Claudine must also supplement missing launch metadata itself because the stream has no `session_start` event, no session id, no cwd, no permission mode, and no tool inventory snapshot.

## Non-Interactive Entry Points

`goose run` is the automation entry point. The official Running Tasks guide says it starts a new session, executes the supplied arguments, and exits automatically when the task is complete. The prompt can come from `--text/-t`, an instruction file through `--instructions/-i`, stdin through `-i -`, or a recipe through `--recipe`.

The most wrapper-friendly command shape is:

```bash
goose run --output-format stream-json --no-session -i -
```

That lets Claudine feed the prompt over stdin and parse stdout as a live event stream. `--no-session` avoids creating a stored session and, per the docs, routes session output to the platform null path (`/dev/null` on Unix and `NUL` on Windows). If resume/recovery matters more than a disposable run, Claudine can omit `--no-session` and select a stored session with `--name`, `--session-id`, and `--resume`.

Recipes are valid in headless mode, but the headless tutorial states that a recipe must include a `prompt` field. `--params` and `--sub-recipe` can parameterize a recipe run. `--system` adds system instructions for non-recipe runs. Tool surfaces are selected with config plus `--with-builtin`, `--with-extension`, `--with-streamable-http-extension`, and `--no-profile`.

Avoid `--interactive` for Claudine-managed non-interactive execution. It intentionally continues into an interactive session after the initial input and changes the blocking model.

## Output Formats

Goose exposes three `goose run --output-format` values:

| CLI value | Format | Streams | Claudine use |
| --- | --- | --- | --- |
| `text` | Human terminal output | Yes, but not structured | Avoid for parsing. It can contain prose, markdown rendering, progress display, ANSI styling, and optional stats. |
| `json` | One final JSON object | No | Useful for batch logs after completion, but weak for live orchestration. |
| `stream-json` | NDJSON | Yes | Preferred. It emits JSON records while the run is active. |

`json` mode emits a final object like:

```json
{
  "messages": [],
  "metadata": {
    "total_tokens": 123,
    "input_tokens": 45,
    "output_tokens": 78,
    "status": "completed"
  }
}
```

`stream-json` mode emits compact stdout lines shaped by the Rust `StreamEvent` enum:

```json
{"type":"message","message":{"role":"assistant","created":1760000000,"content":[]}}
{"type":"notification","extension_id":"developer","message":"..."}
{"type":"notification","extension_id":"developer","progress":0.5,"total":1.0,"message":"..."}
{"type":"error","error":"..."}
{"type":"complete","total_tokens":123,"input_tokens":45,"output_tokens":78}
```

The streaming format is better than final JSON because Claudine can render live assistant output, observe tool requests/results as nested message content, classify errors before process exit, and detect the terminal `complete` event. There is no separate CLI log stream that Claudine must parse for normal operation. Stderr is still useful as diagnostics, especially if the process exits before a terminal stream event.

## Schema Sources

Goose's exact CLI stream schema is source-backed rather than formally published. The strongest source for the outer envelope is `crates/goose-cli/src/session/mod.rs`, where `StreamEvent` is a Rust `serde` enum tagged by top-level `type` and `NotificationData` is flattened into the notification event.

Nested `message` payloads are stronger. `crates/goose-provider-types/src/conversation/message.rs` defines `Message`, `MessageMetadata`, `InferenceMetadata`, `MessageContent`, `ToolRequest`, `ToolResponse`, `ActionRequiredData`, `SystemNotificationContent`, and related types. Those nested types use camelCase `serde` names. Goose also publishes an OpenAPI document for the server/Desktop API and generated TypeScript SDK types. Those are formal, but they are broader than `goose run` stdout and should not be treated as a formal schema for the outer CLI stream.

The practical confidence model is:

| Layer | Best source | Confidence | Caveat |
| --- | --- | --- | --- |
| CLI outer stream | Rust `StreamEvent` source | High for current versions | Not a published compatibility contract. |
| Final JSON output | Rust `JsonOutput`/`JsonMetadata` source | High for current versions | No dedicated public JSON Schema. |
| Nested messages | Rust message types plus OpenAPI | High | Server OpenAPI may include message variants broader than current CLI emission. |
| Docs examples | Running Tasks and CLI Commands docs | Medium | Good for flags and intent, not complete field-level schema. |

## IO Contract

With `--output-format stream-json`, stdout is the machine channel. Each line is independently parseable JSON. Goose emits the line with `serde_json::to_string` and `println!`, so Claudine can use a normal line-oriented NDJSON parser.

Stdin is only prompt input when using `-i -`; it is not a JSON-RPC or bidirectional protocol. Goose headless mode does not ask Claudine to answer protocol requests over stdin.

Stderr should be treated as diagnostics and fallback lifecycle evidence. It may contain command failures, warnings, tracing/logging output, or human-oriented errors if a failure happens outside the stream path. In normal stream-json operation, the structured lifecycle source is stdout.

## Stream Contract

The top-level discriminator is `type`. Current top-level events are:

| Event | Payload | Meaning |
| --- | --- | --- |
| `message` | `message` | A completed Goose `Message` object from the agent stream. |
| `notification` | `extension_id` plus flattened `message` or `progress` fields | MCP/server log or progress notification. |
| `error` | `error` string | Agent error. |
| `complete` | optional `total_tokens`, `input_tokens`, `output_tokens` | Terminal success event emitted during finalization. |

Nested message content uses a second discriminator at `message.content[].type`. Important values include `text`, `image`, `toolRequest`, `toolResponse`, `toolConfirmationRequest`, `actionRequired`, `frontendToolRequest`, `thinking`, `redactedThinking`, and `systemNotification`. This casing split is a parser requirement: top-level events use snake_case, while nested content names and fields are camelCase.

Tool calls and results should be correlated by `message.content[].id` on `toolRequest` and `toolResponse`. If a tool integration omits or mutates ids, the parser can fall back to ordered heuristics, but that should be logged as degraded correlation. Top-level `notification` events are correlated to an extension by `extension_id`, not by a tool call id.

Assistant messages are emitted as completed message objects, not token deltas. There is no verified partial-message event flag equivalent. Unknown top-level events should be skipped and logged at trace or warning level; unknown nested content should be preserved in captured raw JSON when possible because the formal OpenAPI/server model can move ahead of the CLI adapter.

## Session Metadata

`stream-json` does not include an init event. It does not reliably expose session id, cwd, project root, Goose version, permission mode, sandbox mode, root list, or configured extension inventory at startup. Claudine should synthesize that launch record itself from the wrapper context: cwd, argv, selected prompt mode, environment overrides, and an out-of-band `goose --version` probe when needed.

Model/provider identity is only partially visible in the stream. The nested `MessageMetadata` type can carry `metadata.inference.provider`, `metadata.inference.requestedModel`, and `metadata.inference.resolvedModel`, but there is no guarantee this appears before the first meaningful output or on every message. Claudine should record requested `--provider`/`--model` and relevant `GOOSE_PROVIDER`/`GOOSE_MODEL` values before launch, then treat nested inference metadata as runtime confirmation.

Token usage is better. Current source emits optional `total_tokens`, `input_tokens`, and `output_tokens` in the final JSON metadata and the stream `complete` event. These come from accumulated session usage when available, falling back to current usage.

## Event Families

The stream has a small top-level event family and a richer nested message family:

| Family | Where | Notes |
| --- | --- | --- |
| Assistant/user text | `message.content[].type=text` | Final text blocks, not token deltas. |
| Tool call | `message.content[].type=toolRequest` | Includes `id`, `toolCall.name`, `toolCall.arguments`, optional provider metadata, and optional `_meta`. |
| Tool result | `message.content[].type=toolResponse` | Includes `id`, `toolResult`, and optional metadata. Errors live inside `toolResult` or provider-specific structures. |
| Permission/user action | `toolConfirmationRequest` and `actionRequired` | Nested model for tool confirmations and MCP elicitation, but headless code often handles these before normal rendering. |
| Reasoning | `thinking` and `redactedThinking` | Depends on provider support and configuration such as `GOOSE_CLI_SHOW_THINKING`. |
| System notification | `systemNotification` | Includes `notificationType`, `msg`, and optional `data`; `creditsExhausted` is the strongest quota/no-funds signal. |
| MCP notifications | top-level `notification` | Flattened log/progress from extensions, keyed by `extension_id`. |
| Error | top-level `error` | Stringified agent error. |
| Completion | top-level `complete` | Terminal success event with optional token totals. |

## Tools

Goose tools are exposed through the same nested message model regardless of whether they come from builtin extensions or configured MCP extensions. For Claudine, this means there is no separate `file_change`, `command_started`, or `command_finished` top-level event. A shell command, file edit, read, or extension operation must be inferred from the `toolRequest.toolCall.name`, `toolRequest.toolCall.arguments`, and `toolResponse.toolResult` payload.

MCP/server logging and progress notifications can appear as top-level `notification` events. These are useful for status rendering, but they are not a complete lifecycle stream for all tools. A progress notification has `progress`, optional `total`, and optional `message`. A log notification has `message`.

`--debug` is documented as showing complete tool responses, detailed parameters, and full paths in CLI output. In structured mode the stream still serializes the same message objects; Claudine should not rely on `--debug` as a schema-expansion switch unless a fixture proves additional structured fields for the specific Goose version.

## Completion and Exit Status

For success, Claudine should require both a `complete` event and process exit success. `complete` carries optional `total_tokens`, `input_tokens`, and `output_tokens`; it does not carry final answer text. Final answer text must be assembled from `message.content[].type=text` events.

For failure, Goose emits top-level `error` with an error string in the stream-json path, cancels processing, performs interruption cleanup, and returns an error. The process exit code should be treated as reliable for command success/failure, but not rich enough for classification. Claudine should classify failure kind from the `error.error` string, nested `toolResponse.toolResult` errors, and stderr fallback text.

Cancellation and user interruption do not have a dedicated structured terminal event in the verified stream contract. If the process exits without `complete`, Claudine should mark the run ambiguous or failed depending on exit status and stderr.

## Blocking Behavior

Goose's headless tutorial states that headless mode has no user interaction capability and uses default or configured behavior for decisions. Current source makes the important cases concrete:

| Situation | Headless behavior |
| --- | --- |
| Tool confirmation in `GOOSE_MODE=approve` or `smart_approve` | Goose cancels and returns an error saying approval modes require an interactive terminal and to use Auto for headless sessions. |
| Tool confirmation in `GOOSE_MODE=auto` | Goose logs a warning and returns `AllowOnce`. |
| MCP elicitation/user input request | Goose cancels because it cannot collect user input non-interactively. |
| Context limit decision | Docs say headless defaults to summarization, configurable with `GOOSE_CONTEXT_STRATEGY`. |

That behavior is automation-friendly but not a sandbox. If Claudine needs deterministic safety, it should enforce its own policy before launch, set `GOOSE_MODE` deliberately, and avoid MCP tools that can require elicitation or OAuth/user input mid-run.

## Subagents

Goose supports subagents, background tasks, and sub-recipes, and environment variables such as `GOOSE_SUBAGENT_MAX_TURNS` and `GOOSE_MAX_BACKGROUND_TASKS` affect their behavior. However, `goose run --output-format stream-json` does not expose a normalized subagent lifecycle family with child session ids, start/stop events, child model, or nested tool stream boundaries.

Subagent activity may be visible as generic tool requests/results or as formatted top-level notification messages. Claudine can show these as progress, but should not claim complete parent/child observability. Prompt injection for non-interactive behavior is possible through recipes, sub-recipes, and custom prompt templates under the Goose config `prompts/` directory.

## Use Case Detection

| Use case | Detection | Fields | Caveat |
| --- | --- | --- | --- |
| `tokens_consumed` | Yes | `complete.total_tokens`, `complete.input_tokens`, `complete.output_tokens` | Optional session totals; units are tokens. |
| `model_used` | Partial | `message.metadata.inference.provider`, `requestedModel`, `resolvedModel` | Not guaranteed early; supplement with argv/env/config. |
| `model_fallback` | Partial | Compare `requestedModel` and `resolvedModel` | Only when inference metadata is present. |
| `auth` | Heuristic | `error.error`, stderr | No typed auth error envelope. |
| `no_funds` / `plan_capped` | Partial | `systemNotification.notificationType=creditsExhausted`, `msg`, `data` | Reset windows and upgrade URLs are not guaranteed. |
| `permission_write_denied` | Partial | `error.error`, `toolResponse.toolResult` | Approval-mode failures are clear strings; tool denials are provider/tool specific. |
| `permission_read_denied` | Weak | `toolResponse.toolResult`, `error.error` | No normalized read-denied type. |
| `human_in_loop` | Yes for attempted interaction | `actionRequired.data.actionType`, `toolConfirmationRequest.prompt`, `error.error` | Headless may cancel before normal message emission. |
| `session_resumable` | Not from stream | wrapper-selected `--name`/`--session-id` | No stream session id. |
| `subagent_prompt_injection` | Config/prompt feature | recipes, sub-recipes, prompt templates | Not a stream event. |

No structured `plan_cap_approaching` event was verified in the CLI stream. Claudine should not infer quota windows from generic text unless it clearly labels the result heuristic.

## Headless Constraints

The most important headless constraint is that structured output does not equal complete operational observability. Goose's `stream-json` gives live messages, notifications, errors, and terminal token usage, but it omits launch metadata, normalized permission denials, file changes, sandbox/root information, and a stable schema version.

The second constraint is permission mode. Official docs recommend `GOOSE_MODE=auto` for headless success. Current source confirms why: approval modes cannot ask a human in non-interactive mode and fail when confirmation is needed, while Auto mode auto-allows confirmations. Claudine should make that choice explicit instead of inheriting a surprising user config.

The third constraint is auth and keyring behavior. Goose prefers OS keyrings but can fall back to `secrets.yaml` when keyring access is unavailable or disabled. CI and containers should set provider auth through environment variables or deterministic file-based config and should not run interactive `goose configure`.

## Timeline

- 2025-08-29: A public feature request asked for structured `goose run --output-format json` output because plain text was hard to parse programmatically.
- 2025-11: Goose added JSON output support for `goose run` in the public issue/PR stream.
- 2026-04-07: Goose announced the repository move from `block/goose` to `aaif-goose/goose`.
- 2026-07-02: Current docs and source verify `text`, `json`, and `stream-json`; `complete` now includes optional total, input, and output token fields.

## Quirks and Gaps

The strongest quirk is the mixed casing. A parser must handle top-level snake_case events and nested camelCase content in the same stdout line. `notification` is also flattened, so there is no stable nested `data` object at the top level.

The largest gap is the absence of a formal CLI stream schema. The OpenAPI schema is useful for nested server/Desktop objects, but the exact `goose run` stdout contract is currently the Rust source. There is also no verified stream schema version, no local fixture captured with real credentials in this update, no structured cost field, and no exact documented exit-code taxonomy for auth, rate limit, context overflow, max-turn, or cancellation cases.

## Claudine Integration Notes

Use:

```bash
goose run --output-format stream-json --no-session -i -
```

Parse stdout only as NDJSON. Treat stderr as diagnostics, not the primary lifecycle stream. The top-level parser should dispatch on `type`; the nested parser should dispatch on `message.content[].type`. Join tool requests and responses by `id`, and treat `complete` as the stream terminal event. If the process exits without `complete`, classify from exit status, stderr, and any prior `error` event.

Before launch, Claudine should capture the cwd, argv, selected provider/model, relevant Goose env vars, and Goose version because the stream does not emit them. It should pass `--output-format stream-json` every time because no persistent output-format config was verified. It should avoid `--interactive`, avoid inheriting approval modes accidentally, and either set `GOOSE_MODE=auto` for automation or enforce a stricter Claudine policy layer before Goose starts.

For reporting, use `complete.total_tokens`, `complete.input_tokens`, and `complete.output_tokens` as session token totals when present. Cost must be computed externally. File changes should be detected through tool payload interpretation or by filesystem diffing around the process.

## Changelog

- 2026-07-02: Rewrote the Goose non-interactive research file against current `aaif-goose/goose` docs and source, normalized the frontmatter, and added parser-specific notes for `stream-json`, token fields, missing init metadata, and headless permission behavior.

## Sources

- [Running Tasks](https://goose-docs.ai/docs/guides/running-tasks/)
- [CLI Commands](https://goose-docs.ai/docs/guides/goose-cli-commands/)
- [Using goose in Headless Mode for Automation](https://goose-docs.ai/docs/tutorials/headless-goose/)
- [Environment Variables](https://goose-docs.ai/docs/guides/environment-variables/)
- [Configuration Files](https://goose-docs.ai/docs/guides/config-files/)
- [goose Permission Modes](https://goose-docs.ai/docs/guides/managing-tools/goose-permissions/)
- [`crates/goose-cli/src/cli.rs`](https://github.com/aaif-goose/goose/blob/main/crates/goose-cli/src/cli.rs)
- [`crates/goose-cli/src/session/mod.rs`](https://github.com/aaif-goose/goose/blob/main/crates/goose-cli/src/session/mod.rs)
- [`crates/goose-provider-types/src/conversation/message.rs`](https://github.com/aaif-goose/goose/blob/main/crates/goose-provider-types/src/conversation/message.rs)
- [Goose server OpenAPI document](https://raw.githubusercontent.com/aaif-goose/goose/main/crates/goose-server/ui/desktop/openapi.json)
- [Generated TypeScript SDK types](https://github.com/aaif-goose/goose/blob/main/ui/sdk/src/generated/types.gen.ts)
- [Goose moved to AAIF](https://goose-docs.ai/blog/2026/04/07/goose-moves-to-aaif/)
- [Structured output feature request](https://github.com/block/goose/issues/4419)
