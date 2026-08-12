---
$schema: ./_schema.yaml
created: 2026-04-10
last_updated: 2026-07-03
agent: codex
model: default
docs: https://goose-docs.ai/docs/guides/running-tasks/
invocation:
  - command: "goose run --quiet --output-format stream-json --name <stable-name> -t \"<prompt>\""
    stdin_support: false
    prompt_arg: "--text/-t TEXT"
    notes: "Starts a fresh headless run; use a wrapper-generated name because the stream does not emit a session id."
  - command: "goose run --quiet --output-format stream-json --name <stable-name> -i -"
    stdin_support: true
    prompt_arg: "--instructions/-i -"
    notes: "Reads the complete prompt from stdin; stdin is not a bidirectional protocol."
  - command: "goose run --quiet --output-format stream-json --name <stable-name> -i <file>"
    stdin_support: false
    prompt_arg: "--instructions/-i FILE"
    notes: "Reads instructions from a file and exits when the headless run completes."
  - command: "goose run --quiet --output-format stream-json --recipe <recipe.yaml> --params key=value"
    stdin_support: false
    prompt_arg: "--recipe plus optional --params and --sub-recipe"
    notes: "Runs a recipe headlessly; recipe settings can override provider/model and extension configuration."
  - command: "goose run --quiet --output-format stream-json --resume --name <name> -t \"<prompt>\""
    stdin_support: false
    prompt_arg: "--text/-t TEXT"
    notes: "Resumes an existing stored session by name; --session-id or legacy --path can also identify a session."
  - command: "goose run --quiet --output-format stream-json --no-session -t \"<prompt>\""
    stdin_support: false
    prompt_arg: "--text/-t TEXT"
    notes: "Runs without retaining a reusable transcript; not useful when Claudine needs resume/recovery."
output_formats:
  - name: text
    cli_value: text
    stream: true
    format: text
    description: "Default human terminal rendering with markdown, tool display, status text, and optional stats."
    side_effects: "Not parse-safe; stdout can contain the Goose banner, prose, tool rendering, progress, and ANSI styling."
  - name: json
    cli_value: json
    stream: false
    format: json
    description: "One final pretty-printed JSON object with messages and metadata after the run finishes."
    side_effects: "Good for archival batch output, but not live progress; use --quiet to avoid the pre-run banner."
  - name: stream-json
    cli_value: stream-json
    stream: true
    format: ndjson
    description: "One compact JSON event per stdout line for message, notification, error, and complete events."
    side_effects: "Best for Claudine, but only parse-safe with --quiet; tool-call input is emitted as complete blocks rather than partial JSON deltas."
schema_sources:
  - url: https://github.com/aaif-goose/goose/blob/main/crates/goose-cli/src/session/mod.rs
    schema_type: rust
    formal: false
    notes: "Authoritative source for JsonOutput, JsonMetadata, StreamEvent, NotificationData, stream emission, completion, and headless blocking behavior."
  - url: https://github.com/aaif-goose/goose/blob/main/crates/goose-provider-types/src/conversation/message.rs
    schema_type: rust
    formal: false
    notes: "Authoritative serde model for nested Message, MessageContent, ToolRequest, ToolResponse, ActionRequired, SystemNotification, timestamps, and inference metadata."
  - url: https://raw.githubusercontent.com/aaif-goose/goose/main/crates/goose-server/ui/desktop/openapi.json
    schema_type: openapi
    formal: true
    notes: "Formal server/Desktop API schema; useful for nested message objects, broader than the CLI stdout envelope."
  - url: https://github.com/aaif-goose/goose/blob/main/ui/sdk/src/generated/types.gen.ts
    schema_type: typescript
    formal: true
    notes: "Generated SDK types from OpenAPI; corroborates nested server objects but is not the CLI stream schema."
  - url: https://goose-docs.ai/docs/guides/running-tasks/
    schema_type: examples
    formal: false
    notes: "Official docs for invocation, stdin, model/provider flags, extensions, debug, session management, and json/stream-json behavior."
cli_params:
  - flag: "--quiet"
    value: "boolean"
    description: "Suppresses the Goose session banner; required for parse-safe stdout with json or stream-json."
    example: "goose run --quiet --output-format stream-json -t \"inspect\""
  - flag: "--output-format"
    value: "text|json|stream-json"
    description: "Selects human text, final JSON, or streaming NDJSON."
    example: "goose run --quiet --output-format stream-json -t \"summarize\""
  - flag: "-t, --text"
    value: "TEXT"
    description: "Supplies prompt text on argv."
    example: "goose run -t \"fix the tests\""
  - flag: "-i, --instructions"
    value: "FILE|-"
    description: "Reads prompt text from a file or from stdin when the value is '-'."
    example: "cat prompt.md | goose run --quiet --output-format stream-json -i -"
  - flag: "--recipe"
    value: "RECIPE_NAME or PATH"
    description: "Runs a recipe; conflicts with --text, --instructions, and --system."
    example: "goose run --quiet --output-format stream-json --recipe audit.yaml"
  - flag: "--params"
    value: "KEY=VALUE"
    description: "Passes dynamic recipe parameters; can be repeated."
    example: "goose run --recipe deploy.yaml --params env=staging"
  - flag: "--sub-recipe"
    value: "RECIPE"
    description: "Includes additional local or configured sub-recipes."
    example: "goose run --recipe main.yaml --sub-recipe checks.yaml"
  - flag: "--system"
    value: "TEXT"
    description: "Adds system instructions for non-recipe runs."
    example: "goose run --system \"Be concise\" -t \"status\""
  - flag: "--provider"
    value: "PROVIDER"
    description: "Overrides configured provider for this run."
    example: "goose run --provider anthropic --model claude-sonnet-4-5-20250929 -t \"review\""
  - flag: "--model"
    value: "MODEL"
    description: "Overrides configured model for this run."
    example: "goose run --model gpt-5 -t \"review\""
  - flag: "--with-builtin"
    value: "NAME[,NAME]"
    description: "Adds bundled extension tools such as developer or memory."
    example: "goose run --with-builtin developer -t \"edit file\""
  - flag: "--with-extension"
    value: "COMMAND"
    description: "Adds a stdio MCP extension command; can be repeated."
    example: "goose run --with-extension \"ENV=1 mcp-server\" -t \"use tool\""
  - flag: "--with-streamable-http-extension"
    value: "URL [timeout=N]"
    description: "Adds a streamable HTTP MCP extension; can be repeated."
    example: "goose run --with-streamable-http-extension \"https://example/mcp timeout=100\" -t \"use tool\""
  - flag: "--no-profile"
    value: "boolean"
    description: "Prevents loading default configured extensions; useful for deterministic wrappers."
    example: "goose run --no-profile --with-builtin developer -t \"inspect\""
  - flag: "--debug"
    value: "boolean"
    description: "Shows full tool parameters/responses in human text mode and sets debug mode for session logic."
    example: "goose run --debug -t \"debug extension\""
  - flag: "--max-turns"
    value: "NUMBER"
    description: "Limits turns allowed without user input; CLI flag overrides default/session option."
    example: "goose run --max-turns 25 -t \"bounded task\""
  - flag: "--max-tool-repetitions"
    value: "NUMBER"
    description: "Limits repeated identical tool calls."
    example: "goose run --max-tool-repetitions 3 -t \"inspect loop\""
  - flag: "--container"
    value: "CONTAINER_ID"
    description: "Runs stdio and built-in extensions inside a Docker container."
    example: "goose run --container devbox --with-builtin developer -t \"run tests\""
  - flag: "--name, --session-id, --path"
    value: "IDENTIFIER"
    description: "Identifies sessions; --session-id and --path are only valid for resume paths."
    example: "goose run --resume --name claudine-123 -t \"continue\""
  - flag: "--resume"
    value: "boolean"
    description: "Continues an existing session rather than starting a new one."
    example: "goose run --resume --name claudine-123 -t \"continue\""
  - flag: "--no-session"
    value: "boolean"
    description: "Uses hidden/null session storage and discards the transcript when complete."
    example: "goose run --no-session -t \"one-shot\""
  - flag: "--interactive"
    value: "boolean"
    description: "Continues into interactive mode after the initial command; avoid for Claudine automation."
    example: "goose run -i instructions.md --interactive"
  - flag: "--stats"
    value: "boolean"
    description: "Prints generation stats after text-mode headless runs; not part of stream-json."
    example: "goose run --stats -t \"measure\""
config_files:
  - os: macos
    scope: user
    path: "~/.config/goose/config.yaml"
    format: yaml
    effect: "Sets provider, model, Goose mode, max turns, extensions, search paths, prompt/security settings, telemetry, and other defaults."
    notes: "Environment variables override config file values; CLI flags override effective provider/model/session behavior for the run."
  - os: linux
    scope: user
    path: "~/.config/goose/config.yaml"
    format: yaml
    effect: "Sets provider, model, Goose mode, max turns, extensions, search paths, prompt/security settings, telemetry, and other defaults."
    notes: "Environment variables override config file values; CLI flags override effective provider/model/session behavior for the run."
  - os: windows
    scope: user
    path: "%APPDATA%\\Block\\goose\\config\\config.yaml"
    format: yaml
    effect: "Sets provider, model, Goose mode, max turns, extensions, search paths, prompt/security settings, telemetry, and other defaults."
    notes: "Environment variables override config file values; CLI flags override effective provider/model/session behavior for the run."
  - os: macos
    scope: user
    path: "~/.config/goose/permission.yaml"
    format: yaml
    effect: "Stores tool permission configuration written by goose configure."
    notes: "Can make headless runs fail if effective GooseMode is Approve or SmartApprove and a tool confirmation is required."
  - os: linux
    scope: user
    path: "~/.config/goose/permission.yaml"
    format: yaml
    effect: "Stores tool permission configuration written by goose configure."
    notes: "Can make headless runs fail if effective GooseMode is Approve or SmartApprove and a tool confirmation is required."
  - os: windows
    scope: user
    path: "%APPDATA%\\Block\\goose\\config\\permission.yaml"
    format: yaml
    effect: "Stores tool permission configuration written by goose configure."
    notes: "Can make headless runs fail if effective GooseMode is Approve or SmartApprove and a tool confirmation is required."
  - os: macos
    scope: user
    path: "~/.config/goose/secrets.yaml"
    format: yaml
    effect: "Fallback file-based secret storage when keyring is disabled or unavailable."
    notes: "Headless CI often needs environment credentials or GOOSE_DISABLE_KEYRING because desktop keyrings can be unavailable."
  - os: linux
    scope: user
    path: "~/.config/goose/secrets.yaml"
    format: yaml
    effect: "Fallback file-based secret storage when keyring is disabled or unavailable."
    notes: "Headless CI often needs environment credentials or GOOSE_DISABLE_KEYRING because desktop keyrings can be unavailable."
  - os: windows
    scope: user
    path: "%APPDATA%\\Block\\goose\\config\\secrets.yaml"
    format: yaml
    effect: "Fallback file-based secret storage when keyring is disabled or unavailable."
    notes: "Headless CI often needs environment credentials or GOOSE_DISABLE_KEYRING because desktop keyrings can be unavailable."
  - os: macos
    scope: user
    path: "~/.local/share/goose/sessions/sessions.db"
    format: other
    effect: "SQLite session records with IDs, working directories, messages, tool calls/results, token usage, and extension state."
    notes: "Useful for post-run recovery, but the stream-json envelope itself does not emit the session id."
  - os: linux
    scope: user
    path: "~/.local/share/goose/sessions/sessions.db"
    format: other
    effect: "SQLite session records with IDs, working directories, messages, tool calls/results, token usage, and extension state."
    notes: "Useful for post-run recovery, but the stream-json envelope itself does not emit the session id."
  - os: windows
    scope: user
    path: "%APPDATA%\\Block\\goose\\data\\sessions\\sessions.db"
    format: other
    effect: "SQLite session records with IDs, working directories, messages, tool calls/results, token usage, and extension state."
    notes: "Useful for post-run recovery, but the stream-json envelope itself does not emit the session id."
env_vars:
  - name: GOOSE_PROVIDER
    effect: "Sets default LLM provider when not supplied by CLI, resumed session, or recipe settings."
    notes: "Config docs say environment variables have higher precedence than config.yaml."
  - name: GOOSE_MODEL
    effect: "Sets default model when not supplied by CLI, resumed session, or recipe settings."
    notes: "Stream messages can expose requested/resolved model in message.metadata.inference, not in a start event."
  - name: GOOSE_PROVIDER__API_KEY
    effect: "Supplies provider API key and can avoid interactive/keyring credential setup."
    notes: "Provider-specific secret variables also exist; exact names depend on provider configuration."
  - name: GOOSE_MODE
    effect: "Controls tool execution behavior: auto, approve, chat, or smart_approve."
    notes: "Headless tool confirmation fails in Approve/SmartApprove; Auto can auto-allow confirmation requests."
  - name: GOOSE_MAX_TURNS
    effect: "Sets default max turns without user input."
    notes: "--max-turns provides a per-run bound."
  - name: GOOSE_CONTEXT_STRATEGY
    effect: "Controls context-limit handling; docs state headless defaults to summarize rather than prompt."
    notes: "Use summarize/truncate/clear for automation; prompt is risky in non-interactive runs."
  - name: GOOSE_DEBUG
    effect: "Enables debug mode with full parameters and responses."
    notes: "Can affect human rendering and logs; stream-json nested tool payloads are still structured messages."
  - name: GOOSE_CLI_SHOW_THINKING
    effect: "Shows reasoning/thinking in CLI output for supported models."
    notes: "Nested thinking content can appear as message content when providers expose it."
  - name: GOOSE_CLI_SHOW_COST
    effect: "Toggles estimated cost display in CLI output."
    notes: "Cost is not present in StreamEvent::Complete."
  - name: GOOSE_DISABLE_KEYRING
    effect: "Disables system keyring and uses file-based secrets."
    notes: "Important for CI/containers where keyring access can fail."
  - name: GOOSE_MAX_TOOL_RESPONSE_SIZE
    effect: "Limits a single tool response before writing it to a temporary file instead of inline conversation content."
    notes: "Affects whether Claudine sees raw output inline in toolResponse content."
  - name: GOOSE_SHELL
    effect: "Overrides shell used for Developer extension shell commands."
    notes: "Cross-platform behavior differs; Windows default is cmd."
  - name: GOOSE_SUBAGENT_MAX_TURNS
    effect: "Sets default maximum turns for subagents."
    notes: "Subagent recipes or tool calls can override it."
  - name: GOOSE_MAX_BACKGROUND_TASKS
    effect: "Limits concurrent background subagent tasks."
    notes: "Relevant when delegate/subagent tools are enabled."
  - name: GOOSE_DISABLE_SESSION_NAMING
    effect: "Disables AI-generated session naming."
    notes: "Useful in CI to avoid extra model calls and keep wrapper-generated names stable."
  - name: OTEL_EXPORTER_OTLP_ENDPOINT
    effect: "Enables OpenTelemetry export for observability."
    notes: "Secondary telemetry stream; not equivalent to the CLI NDJSON stream."
  - name: NO_COLOR
    effect: "Disables color in human terminal rendering via console behavior."
    notes: "Prefer --quiet plus stream-json instead of relying on color suppression."
io_contract:
  stdout: structured_only
  stderr: diagnostics_only
  stdin: prompt
  framing: ndjson
  noise_handling: "For stream-json, pass --quiet and parse stdout line-by-line as JSON. Treat stderr as diagnostics and setup warnings. If --quiet is omitted, stdout starts with a human Goose banner before JSON."
  notes: "Early setup failures can still print human errors and exit before any stream event; stdin is consumed as prompt text only for -i -."
stream_contract:
  discriminator: "type"
  event_ordering: "No start event. Zero or more message/notification/error events are followed by complete only when the execution loop reaches normal teardown."
  correlation_fields: ["message.id", "message.content[].id", "message.content[].toolCall.value.name", "message.content[].toolResult.id", "extension_id"]
  terminal_event: "complete"
  partial_message_events: true
  unknown_event_policy: "Skip unknown top-level types and unknown message.content[].type values after logging at trace; preserve raw JSON for drift analysis."
  notes: "Top-level event names are snake_case; nested message and action tags are camelCase. Text can stream as multiple message events, but toolRequest input is emitted as a complete block rather than JSON argument deltas."
session_metadata:
  session_id: "Not emitted in stream-json; human banner shows it when --quiet is omitted; wrapper should provide --name and use session list/storage for recovery."
  cwd: "Not emitted in stream-json; human banner and session database record the working directory; resumed headless runs warn on stderr if cwd differs."
  model: "message.metadata.inference.requestedModel and resolvedModel can appear on model-generated messages; setup also logs model via tracing."
  provider: "message.metadata.inference.provider can appear on model-generated messages; provider can be set by CLI, recipe, config, or environment."
  auth: "Not emitted as structured metadata; auth/keyring failures usually occur as setup errors before stream events."
  version: "Not emitted in stream-json; available from goose --version outside the run."
  mcp_servers: "Configured extensions are not listed in a start event; tool calls and notification.extension_id reveal active extensions as they are used."
  permission_mode: "Not emitted in stream-json; effective GooseMode comes from config/env and affects non-interactive confirmation behavior."
  notes: "The stream lacks an init envelope, so Claudine must add wrapper-side metadata for cwd, invocation, Goose version, and requested provider/model."
stream_events:
  - event: message
    category: assistant
    fields: ["message.id", "message.role", "message.created", "message.content", "message.metadata"]
    notes: "Carries assistant text, thinking, toolRequest, toolResponse, systemNotification, and actionRequired content."
  - event: notification
    category: other
    fields: ["extension_id", "log.message", "progress.progress", "progress.total", "progress.message"]
    notes: "MCP logging/progress notifications; subagent tool request notices are converted to log messages."
  - event: error
    category: error
    fields: ["error"]
    notes: "Emitted for AgentEvent errors in stream-json mode; a following complete event can still be emitted."
  - event: complete
    category: usage
    fields: ["total_tokens", "input_tokens", "output_tokens"]
    notes: "Terminal stream event for normal loop teardown; lacks status, result text, session id, provider, model, and cost."
  - event: message.content[].text
    category: assistant
    fields: ["text", "_meta"]
    notes: "Nested camelCase content type; assistant text can arrive in multiple message events."
  - event: message.content[].thinking
    category: reasoning
    fields: ["thinking", "signature"]
    notes: "Nested reasoning content when provider/model exposes it."
  - event: message.content[].redactedThinking
    category: reasoning
    fields: ["data"]
    notes: "Nested redacted reasoning content."
  - event: message.content[].toolRequest
    category: tool_call
    fields: ["id", "toolCall", "metadata", "_meta"]
    notes: "Tool call start/input; id joins to toolResponse.id."
  - event: message.content[].toolResponse
    category: tool_result
    fields: ["id", "toolResult", "metadata"]
    notes: "Tool result; success/error is encoded inside toolResult."
  - event: message.content[].actionRequired
    category: permission
    fields: ["data.actionType", "data.id", "data.toolName", "data.arguments", "data.prompt", "data.message", "data.requestedSchema"]
    notes: "Tool confirmation or MCP elicitation request; headless handling may auto-allow, fail, or cancel before this is useful to a wrapper."
  - event: message.content[].systemNotification
    category: other
    fields: ["notificationType", "msg", "data"]
    notes: "Can include thinkingMessage, inlineMessage, or creditsExhausted."
tools:
  - name: MCP extensions
    call_visible: true
    result_visible: true
    metadata: ["toolRequest.id", "toolRequest.toolCall", "toolResponse.id", "toolResponse.toolResult", "notification.extension_id"]
    notes: "Native and external MCP tools share nested message content; notifications are separate top-level notification events."
  - name: developer shell/command tools
    call_visible: true
    result_visible: true
    metadata: ["toolCall.value.name", "toolCall.value.arguments", "toolResult.value.content", "toolResult.error"]
    notes: "Command stdout/stderr/exit details are visible only insofar as the tool result includes them; no dedicated command event exists."
  - name: developer file editing tools
    call_visible: true
    result_visible: true
    metadata: ["path arguments", "tool result content", "tool result error"]
    notes: "File changes are not separate file_change events; infer them from tool names, arguments, and results."
  - name: delegate/subagent tools
    call_visible: true
    result_visible: true
    metadata: ["toolCall.value.name", "toolCall.value.arguments", "notification.log.message"]
    notes: "Parent stream may show delegate/subagent tool calls and summarized subagent tool notifications, but not a full nested child event stream."
  - name: prompt/elicitation tools
    call_visible: true
    result_visible: false
    metadata: ["actionRequired.data.actionType", "actionRequired.data.requestedSchema"]
    notes: "MCP elicitation is not collected in headless mode; it fails rather than reading stdin mid-run."
completion:
  success_event: "complete"
  failure_event: "error"
  exit_code_reliable: false
  result_fields: ["message.content[].text", "json.messages", "json.metadata.status"]
  cost_fields: []
  usage_fields: ["complete.total_tokens", "complete.input_tokens", "complete.output_tokens", "json.metadata.total_tokens", "json.metadata.input_tokens", "json.metadata.output_tokens"]
  notes: "A stream error can be followed by complete, and complete has no status field. Early setup failures may produce no JSON. Claudine should combine error events, process exit, and complete presence."
blocking_behavior:
  permissions: configurable
  questions: fail
  tool_approvals: configurable
  notes: "In headless mode, tool confirmations auto-allow under GooseMode Auto, but fail in Approve/SmartApprove. MCP elicitation fails because no interactive terminal is available. Auth/OAuth/keyring setup can fail before structured output."
subagents:
  supported: true
  start_visible: false
  stop_visible: false
  nested_events_visible: false
  prompt_injection_supported: true
  metadata_fields: ["toolRequest.toolCall.value.name", "toolRequest.toolCall.value.arguments", "notification.log.message", "GOOSE_SUBAGENT_MAX_TURNS"]
  notes: "Subagents are available through delegate/subagent tools and recipes; the parent stream exposes tool calls/results and limited log notifications, not full child session streams."
use_cases:
  - name: plan_cap_approaching
    detectable: false
    event_types: []
    fields: []
    hook_parity: "unknown"
    notes: "No structured plan/quota cap warning was verified in stream-json."
  - name: plan_capped
    detectable: false
    event_types: ["message.content[].systemNotification"]
    fields: ["notificationType", "msg", "data"]
    hook_parity: "unknown"
    notes: "creditsExhausted may signal provider credits, but no general plan cap schema is defined."
  - name: no_funds
    detectable: true
    event_types: ["message.content[].systemNotification", "error"]
    fields: ["notificationType=creditsExhausted", "msg", "data.top_up_url", "error"]
    hook_parity: "unknown"
    notes: "Credits exhaustion can appear as a system notification; provider billing failures may only be error strings."
  - name: auth
    detectable: true
    event_types: ["error", "process_exit"]
    fields: ["error", "stderr", "exit_code"]
    hook_parity: "logs may contain auth/keyring diagnostics"
    notes: "Auth/keyring failures often happen during provider creation before stream-json starts."
  - name: permission_read_denied
    detectable: true
    event_types: ["message.content[].toolResponse", "error"]
    fields: ["toolResponse.id", "toolResult.error", "error"]
    hook_parity: "logs may contain tool details"
    notes: "No dedicated read-denied event; classify from tool result/error text and tool name."
  - name: permission_write_denied
    detectable: true
    event_types: ["message.content[].toolResponse", "error"]
    fields: ["toolResponse.id", "toolResult.error", "error"]
    hook_parity: "logs may contain tool details"
    notes: "No dedicated write-denied event; classify from tool result/error text and tool name."
  - name: tokens_consumed
    detectable: true
    event_types: ["complete", "json"]
    fields: ["total_tokens", "input_tokens", "output_tokens", "metadata.total_tokens", "metadata.input_tokens", "metadata.output_tokens"]
    hook_parity: "LLM request logs include token usage"
    notes: "Units are tokens; stream values are cumulative session/database usage when available."
  - name: model_used
    detectable: true
    event_types: ["message"]
    fields: ["message.metadata.inference.provider", "message.metadata.inference.requestedModel", "message.metadata.inference.resolvedModel"]
    hook_parity: "LLM request logs include model configuration"
    notes: "Not guaranteed on every message and no init event repeats the requested CLI values."
  - name: model_fallback
    detectable: true
    event_types: ["stderr", "message"]
    fields: ["stderr warning", "message.metadata.inference.provider", "message.metadata.inference.requestedModel"]
    hook_parity: "logs may include fallback warning"
    notes: "Resume can fall back from an unavailable original provider to the default provider and prints a warning."
  - name: human_in_loop
    detectable: true
    event_types: ["message.content[].actionRequired", "error"]
    fields: ["actionRequired.data.actionType", "actionRequired.data.prompt", "actionRequired.data.requestedSchema", "error"]
    hook_parity: "unknown"
    notes: "Tool confirmation and elicitation are represented as actionRequired content; headless handling may auto-allow or fail."
  - name: session_resumable
    detectable: false
    event_types: []
    fields: []
    hook_parity: "session database contains id and name"
    notes: "The stream does not emit a session id; Claudine should pass --name and query/manage sessions separately."
  - name: subagent_prompt_injection
    detectable: true
    event_types: ["message.content[].toolRequest"]
    fields: ["toolCall.value.name", "toolCall.value.arguments.instructions", "toolCall.value.arguments.parameters"]
    hook_parity: "unknown"
    notes: "Caller can steer subagents through recipes/tool arguments; no dedicated child prompt-injection event exists."
headless_constraints:
  - constraint: "--quiet is required for parse-safe stdout."
    mitigation: "Always include --quiet with --output-format stream-json or json."
    notes: "Without --quiet, build_session prints a human session banner to stdout before the JSON stream."
  - constraint: "No stream init event or session id."
    mitigation: "Generate a unique --name, record cwd/provider/model/version wrapper-side, and use session list/storage for recovery."
    notes: "Do not scrape the non-quiet banner unless accepting mixed stdout."
  - constraint: "Approve and SmartApprove modes are invalid for headless tool confirmations."
    mitigation: "Use GOOSE_MODE=auto for deterministic automation or preconfigure allowed tools."
    notes: "Headless confirmation in those modes returns an error instead of waiting for a TTY."
  - constraint: "MCP elicitation fails in non-interactive mode."
    mitigation: "Disable eliciting tools, pre-answer via recipe/config, or use an integration path that can answer MCP elicitation."
    notes: "stdin remains prompt text, not a mid-run answer channel."
  - constraint: "Tool input deltas are not exposed."
    mitigation: "Show heartbeat/quiet timers while waiting for complete toolRequest blocks."
    notes: "An upstream issue documents buffering of complete tool-use blocks in stream-json."
  - constraint: "Early setup/auth/keyring failures can happen before JSON."
    mitigation: "Set provider/model/secrets non-interactively and classify non-JSON stdout/stderr plus exit code."
    notes: "Provider creation errors mention keychain/keyring troubleshooting."
quirks:
  - "The repository and docs have moved from Block branding to AAIF; source URLs now live under aaif-goose/goose."
  - "Top-level stream event names use snake_case, while nested message content and action tags use camelCase."
  - "The complete event contains token usage but no status, final answer, session id, provider, model, or cost."
  - "In-loop agent errors can emit error and then complete, so complete alone does not prove success."
  - "The source uses stdout for human render_error paths, so setup failures can break a pure JSON parser before stream setup."
  - "json mode is pretty-printed final JSON, not JSONL; stream-json is compact one-object-per-line NDJSON."
gaps:
  - "No local Goose binary was available in this environment, so no live fixture was captured."
  - "No formal JSON Schema for goose run --output-format stream-json was found."
  - "Exact stderr/stdout behavior of extension startup spinners under non-TTY pipes was not verified locally."
  - "Exact exit codes for provider auth failures, quota exhaustion, max turns, cancellation, and context overflow were not exhaustively verified."
  - "No stable structured schema was verified for costs, rate-limit reset windows, or plan caps."
  - "No full nested subagent event stream schema was found for the CLI stream-json mode."
claudine_strategy:
  preferred_invocation: "goose run --quiet --output-format stream-json --name <claudine-run-id> -i -"
  required_flags: ["--quiet", "--output-format stream-json", "--name <claudine-run-id>", "-i - or -t TEXT"]
  conflicting_flags: ["--interactive", "--no-session when resume/recovery is needed", "GOOSE_MODE=approve", "GOOSE_MODE=smart_approve"]
  parser_notes: "Parse stdout as NDJSON only when --quiet is supplied. Use top-level type, then nested message.content[].type/actionRequired.data.actionType. Join toolRequest/toolResponse by content id. Treat error as failure even if complete follows."
  wrapper_notes: "Record Goose version, cwd, requested provider/model, GOOSE_MODE, generated session name, and environment/config provenance wrapper-side because the stream has no init event."
data_format: ndjson
changes:
  - "2026-07-03: Re-researched Goose CLI stream-json from official docs and current aaif-goose source; added --quiet parse-safety requirement, headless approval behavior, and stream schema details."
requires_claudine_update: true
reason: "Claudine should launch Goose stream-json with --quiet and a wrapper-generated --name, and its parser must handle mixed casing, absent init/session id, error-plus-complete failure semantics, and complete toolRequest blocks."
---

# Goose CLI Non-Interactive Sessions

## Summary

Claudine can run Goose non-interactively with structured output through `goose run --output-format stream-json`. That mode emits one compact JSON object per stdout line while the run is active, and it is the best Goose output for wrapper use because it exposes assistant messages, tool requests, tool responses, MCP notifications, errors, and final token usage before process exit.

The recommended launch form is `goose run --quiet --output-format stream-json --name <claudine-run-id> -i -`. The `--quiet` flag matters: current source prints the human Goose session banner from `build_session` unless quiet mode is enabled, so `stream-json` is only parse-safe on stdout when `--quiet` is supplied. The main parser risks are the lack of an init/session-start event, no session id in the stream, snake_case top-level events with camelCase nested content, a `complete` event that has token usage but no status, and headless approval/elicitation paths that can fail before or during the stream.

## Non-Interactive Entry Points

The official task-running docs describe `goose run` as the non-interactive command: it starts a session, executes supplied arguments, and exits automatically when the task completes. Prompt text can come from argv with `-t/--text`, from a file with `-i/--instructions <file>`, or from stdin with `-i -`. Recipes run headlessly with `--recipe`, optional `--params`, and optional `--sub-recipe`.

The run command also supports session identity and lifecycle controls. `--name` creates or identifies a named session, `--resume` resumes a previous run, `--session-id` can identify a session only with resume, and `--no-session` discards retained session state. For Claudine, a wrapper-generated `--name` is more useful than `--no-session` because Goose's structured stream does not emit the session id. A stable name gives Claudine an external correlation key and leaves the session database available for recovery.

Provider and model can be selected per run with `--provider` and `--model`. Tools/extensions can be shaped with `--with-builtin`, `--with-extension`, `--with-streamable-http-extension`, and `--no-profile`. `--system` adds extra system instructions for non-recipe runs. `--max-turns` and `--max-tool-repetitions` bound autonomous execution. `--interactive` conflicts with Claudine's non-interactive goal because it keeps a human session open after the initial prompt.

## Output Formats

Goose documents two structured output formats for `goose run`: `json` and `stream-json`. The Clap definition in `crates/goose-cli/src/cli.rs` accepts exactly `text`, `json`, and `stream-json` for `--output-format`.

| Format | CLI value | Framing | Streams | Claudine preference | Notes |
| --- | --- | --- | --- | --- | --- |
| Human text | `text` | Text | Yes | No | Default output. It can include banners, markdown, tool rendering, progress, ANSI styling, and optional stats. |
| Final JSON | `json` | Single pretty JSON object | No | Sometimes for archive-only batch logs | Emitted after completion as `{ "messages": [...], "metadata": { "total_tokens", "input_tokens", "output_tokens", "status" } }`. It does not provide live progress. |
| Streaming JSON | `stream-json` | NDJSON | Yes | Yes | Emits `message`, `notification`, `error`, and `complete` events as the run proceeds. Use `--quiet` to keep stdout parse-safe. |

`stream-json` is the right primary format for Claudine because it is the only documented Goose mode that provides live machine-readable progress. `json` is easier to store but loses the important wrapper signals that arrive mid-run: tool calls, tool results, notifications, and errors. Goose also has local logs and optional OpenTelemetry export, but those are secondary observability streams. Claudine should parse stdout NDJSON as the authoritative live run stream and use logs only for troubleshooting or after-the-fact enrichment.

There is one important caveat: without `--quiet`, source inspection shows `build_session` calls `display_session_info`, which prints a human banner and session details to stdout before the agent loop. That makes stdout mixed text and JSON. With `--quiet`, the normal banner is suppressed, and stream events are emitted through `serde_json::to_string(event)` followed by `println!`.

## Schema Sources

No formal JSON Schema for `goose run --output-format stream-json` was found. The authoritative stream envelope is the Rust enum in [`crates/goose-cli/src/session/mod.rs`](https://github.com/aaif-goose/goose/blob/main/crates/goose-cli/src/session/mod.rs): `StreamEvent` is serde-tagged with top-level `type` and `rename_all = "snake_case"`. Its variants are `Message { message }`, `Notification { extension_id, ... }`, `Error { error }`, and `Complete { total_tokens, input_tokens, output_tokens }`.

The nested message schema is defined in [`crates/goose-provider-types/src/conversation/message.rs`](https://github.com/aaif-goose/goose/blob/main/crates/goose-provider-types/src/conversation/message.rs). `Message` has `id`, `role`, `created`, `content`, and `metadata`. `created` uses `Utc::now().timestamp()`, so timestamps are Unix seconds. `MessageContent` is serde-tagged on nested `type` with camelCase names such as `text`, `toolRequest`, `toolResponse`, `actionRequired`, `thinking`, `redactedThinking`, `frontendToolRequest`, and `systemNotification`.

Goose also publishes a Desktop/server OpenAPI document and generated TypeScript SDK types. Those are formal schema artifacts for the server API and are useful corroboration for nested message objects, but they are broader than the CLI stdout stream and should not be treated as the exact `goose run` stream contract.

## IO Contract

With `--quiet --output-format stream-json`, stdout should be parsed as newline-delimited JSON. Each stdout line is an independent JSON object. Stdin is prompt input only when `-i -` is used; it is not a bidirectional control protocol and cannot be used to approve tools or answer MCP elicitation mid-run.

Stderr is diagnostics. Source paths print warnings and setup diagnostics to stderr, including invalid extensions, extension startup failures, resumed-session working-directory mismatch, and provider fallback warnings. Text-mode stats also go to stderr. Claudine should capture stderr for diagnostics and classification, but it should not expect structured lifecycle events there.

Early failures can happen before the stream exists. Provider creation errors, missing configuration, keyring failures, invalid session identifiers, and similar setup problems can print human text and exit without any `error` or `complete` event. Claudine therefore needs a fallback path: if stdout is not valid NDJSON or no terminal event arrives, classify using exit code, stderr, and any non-JSON stdout.

## Stream Contract

The top-level discriminator is `type`. Valid current top-level values from source are:

| Top-level `type` | Fields | Meaning |
| --- | --- | --- |
| `message` | `message` | A Goose conversation message containing text, thinking, tool calls, tool results, or action-required content. |
| `notification` | `extension_id` plus `log` or `progress` fields | MCP extension logging/progress notification. |
| `error` | `error` | Agent event error string. |
| `complete` | `total_tokens`, `input_tokens`, `output_tokens` | Normal loop teardown and cumulative token usage when available. |

Nested message content has a separate discriminator at `message.content[].type`. Parser code must preserve the casing difference: top-level stream events are snake_case, while nested content is camelCase. `actionRequired` has another nested discriminator at `data.actionType`, with `toolConfirmation`, `elicitation`, and `elicitationResponse`.

Tool calls and results correlate by id. A `toolRequest` content block has `id`; a later `toolResponse` has the same `id`. The tool name and arguments are inside `toolRequest.toolCall` when the call parsed successfully. Because `toolCall` and `toolResult` are serialized result-like wrappers, parsers should allow either success/value or error shapes.

There is no `start` or `init` event. The first parseable event may be an assistant text message, a tool request, a notification, or an error. The `complete` event is terminal only for the execution loop path that reaches teardown. A stream can emit `error` and later `complete`; that combination should be treated as failed or at least ambiguous, not successful. Unknown top-level or nested event types should be skipped after logging and retaining raw JSON for drift analysis.

Assistant text can arrive in multiple `message` events. Tool-request JSON arguments, however, should be treated as complete blocks. A 2026 upstream issue about partial tool-use deltas documents the current wrapper problem: long tool inputs can be buffered until a complete tool block is emitted, causing quiet periods in the stream. Claudine should show elapsed-time heartbeats during these gaps rather than expecting partial tool argument deltas.

## Session Metadata

The stream lacks a session-start envelope. It does not emit Goose version, cwd, session id, permission mode, configured MCP servers, or requested provider/model at start. If `--quiet` is omitted, the human banner includes session information, but that makes stdout mixed and is the wrong tradeoff for a parser.

Some model metadata can appear on generated messages. `message.metadata.inference` has `provider`, `requestedModel`, and optional `resolvedModel`. This is useful for `model_used` detection but is not a complete init record and is not guaranteed to appear before other operational events. The `complete` event contains token counts only.

Goose stores richer session data locally. The logging docs describe SQLite session records at `~/.local/share/goose/sessions/sessions.db` on macOS/Linux and `%APPDATA%\Block\goose\data\sessions\sessions.db` on Windows. Those records include metadata, working directory, messages, tool calls/results, token usage, and extension data. Claudine can use a wrapper-generated `--name` plus Goose session commands/storage for recovery, but the live parser should not depend on a session id in the stream.

## Event Families

`message` is the central event family. It covers assistant prose, user/tool-response messages, reasoning content, system notifications, tool calls, tool results, and action-required requests. Important nested content types include:

| Nested type | Category | Parser notes |
| --- | --- | --- |
| `text` | Assistant/user text | `text` field contains the text block. Multiple events can form a full answer. |
| `thinking` | Reasoning | Contains `thinking` and `signature` when exposed. |
| `redactedThinking` | Reasoning | Contains redacted reasoning data. |
| `toolRequest` | Tool call | Contains `id`, `toolCall`, optional `metadata`, and optional `_meta`. |
| `toolResponse` | Tool result | Contains `id`, `toolResult`, and optional `metadata`. |
| `actionRequired` | Permission or human input | Contains `data.actionType`; headless mode handles this specially. |
| `systemNotification` | Runtime notification | `notificationType` can include `thinkingMessage`, `inlineMessage`, or `creditsExhausted`. |

`notification` events come from MCP server notifications. Logging notifications become `notification` events with `extension_id` and `log.message`. Progress notifications become `notification` events with `progress.progress`, optional `progress.total`, and optional `progress.message`. Subagent tool request notices are downgraded to formatted log messages in stream-json rather than a fully structured child event.

`error` is a string-only event. It is valuable but not rich enough to classify all failures without string matching and stderr. `complete` carries final token counts but does not include a status, final answer, session id, or cost.

## Tools

Goose tools are represented through the same nested message schema whether they come from built-in extensions or external MCP extensions. A tool call is visible before execution as `message.content[].toolRequest`; the result is visible later as `message.content[].toolResponse`. Input is visible in `toolRequest.toolCall` when the provider emitted a valid call. Output is visible in `toolResponse.toolResult`, subject to Goose's tool response size limits and any tool-specific summarization.

Command execution does not have a dedicated command event. Shell commands from the developer extension appear as tool requests/results. Exit codes, stdout, stderr, and file paths are only as structured as the tool result content makes them. File edits likewise have no dedicated `file_change` event; Claudine must infer reads/writes/deletes from tool names, arguments, and results.

MCP progress/logging is a separate `notification` family. This is useful for live progress, but it is not a substitute for tool result parsing. `notification.extension_id` identifies the extension producing the notification.

Subagents exist through delegate/subagent tool paths and recipes. The parent stream can show the parent tool request/result and some formatted subagent tool notifications, but source inspection did not find a full nested child stream with child session ids, child model metadata, or child lifecycle events.

## Completion and Exit Status

For successful stream teardown, Goose emits:

```json
{"type":"complete","total_tokens":1234,"input_tokens":1000,"output_tokens":234}
```

The token fields are optional and represent token counts from stored session usage when available. They are token units, not dollars. Cost is not present in `complete`; `GOOSE_CLI_SHOW_COST` affects human CLI cost display, not the stream schema.

`complete` alone is not a reliable success marker. In the agent response loop, `AgentEvent` errors are handled by emitting `{"type":"error","error":"..."}` and then breaking to normal teardown, which can still emit `complete`. Claudine should treat any `error` event as failure or ambiguity even if `complete` follows. Conversely, setup errors can occur before any stream event and can exit nonzero with human diagnostics only.

The final answer text is not a single `result` field. Claudine must assemble visible text from `message.content[].text` events or use final `json` mode for archive-only workflows. For live operation, parse `stream-json` and maintain state until process exit.

## Blocking Behavior

Headless Goose does not read stdin for mid-run approvals. Tool confirmation and MCP elicitation are handled in the agent loop:

| Situation | Headless behavior | Claudine implication |
| --- | --- | --- |
| Tool confirmation with GooseMode `Auto` | Auto-allows once | Deterministic but permissive; use only with an intentional automation policy. |
| Tool confirmation with GooseMode `Approve` or `SmartApprove` | Fails with an invalid headless configuration error | Avoid these modes for non-interactive runs unless all confirmations are precluded. |
| MCP elicitation | Fails because no interactive terminal can collect input | Disable eliciting tools or use a different integration path that can answer elicitation. |
| Auth/keyring/OAuth setup | Can fail before structured output | Supply non-interactive credentials and avoid first-run configuration in CI. |

The official configuration docs list `GOOSE_MODE` as the tool execution behavior setting and environment docs list `GOOSE_DISABLE_KEYRING` for disabling system keyring use. This matters in containers and CI where a desktop keyring may not be available. If the keyring is disabled or inaccessible, Goose can fall back to `secrets.yaml`.

## Subagents

Subagents can run non-interactively when the relevant tools/recipes are available, and `GOOSE_SUBAGENT_MAX_TURNS` controls their default turn cap. Claudine can inject non-interactive instructions through the original prompt, recipes, or delegate/subagent tool arguments, but there is no separate structured subagent control channel in the CLI stream.

Visibility is limited. Parent-level `toolRequest` and `toolResponse` blocks show delegate/subagent tool calls. MCP logging notifications may include formatted messages that a subagent made a tool call. Source inspection did not verify child start/stop events, child session ids, child token usage, or nested child tool streams as first-class events in the parent `stream-json` output.

## Use Case Detection

| Use case | Detectable | Fields/events | Caveats |
| --- | --- | --- | --- |
| `tokens_consumed` | Yes | `complete.total_tokens`, `complete.input_tokens`, `complete.output_tokens` | Optional fields; cumulative session usage when available. |
| `model_used` | Partially | `message.metadata.inference.provider`, `requestedModel`, `resolvedModel` | Not a start event and may be absent from some messages. |
| `model_fallback` | Partially | stderr warning plus inference metadata | Resume fallback prints a human warning when original provider is unavailable. |
| `auth` | Partially | `error.error`, stderr, process exit | Many auth/keyring failures occur before stream-json events. |
| `no_funds` | Partially | `systemNotification.notificationType=creditsExhausted`, `msg`, `data.top_up_url`; provider error strings | No general billing schema or reset window. |
| `permission_read_denied` | Partially | `toolResponse.toolResult` errors and tool name/path arguments | No dedicated read-denied event. |
| `permission_write_denied` | Partially | `toolResponse.toolResult` errors and tool name/path arguments | No dedicated write-denied event. |
| `human_in_loop` | Yes | `actionRequired.data.actionType`, `prompt`, `requestedSchema`, headless error strings | Tool confirmations may auto-allow in Auto mode; elicitation fails. |
| `session_resumable` | Not from stream | Wrapper-supplied `--name`; session database | Stream lacks session id. |
| `subagent_prompt_injection` | Partially | delegate/subagent `toolRequest.toolCall` arguments and recipe content | No dedicated subagent prompt event. |
| `plan_cap_approaching` | No | None verified | No structured plan/quota warning found. |
| `plan_capped` | No general signal | Possible `creditsExhausted` near-miss | Credits exhaustion is not the same as a plan cap. |

## Headless Constraints

The strongest constraint is stdout purity. `--output-format stream-json` alone is not enough for Claudine because source inspection shows the session banner is printed before the run unless `--quiet` is set. The parser contract should require both flags.

The second constraint is missing init metadata. Goose does not tell the stream consumer the session id, cwd, provider, model, Goose version, effective permission mode, or enabled extensions in a first event. Claudine must record those from its own invocation, environment, and `goose --version`, and should pass `--name <claudine-run-id>` for recovery.

The third constraint is interactive safety behavior. `Approve` and `SmartApprove` are explicitly invalid for headless tool confirmations in current source. MCP elicitation also fails in headless mode. This is better than silently hanging, but it means Claudine must treat those configurations as automation blockers.

Finally, tool input can be silent while the provider is generating a large JSON argument. Current source emits `toolRequest` as a complete nested message block, and an upstream issue describes long quiet gaps before complete tool-use blocks. Claudine should render time-based liveness indicators during stream silence.

## Timeline

| Date | Evidence | Notes |
| --- | --- | --- |
| 2025-08-29 | GitHub issue #4419 requested structured `goose run --output-format json` output | Shows the feature was added to solve script parsing. |
| 2026-04-30 | GitHub issue #8933 discussed missing partial tool-use deltas in `stream-json` | Confirms wrapper-visible quiet gaps around long tool arguments. |
| 2026-07-03 | Source and docs inspection | Current docs expose `json` and `stream-json`; current source defines `StreamEvent` and headless approval/elicitation behavior. |

## Quirks and Gaps

The main provider-specific parser footgun is casing: top-level `StreamEvent` variants use snake_case, while nested `MessageContent` variants use camelCase. Parsers that normalize all event names will lose important distinctions such as `toolRequest` and `actionRequired.data.actionType`.

The stream's terminal event is intentionally small. It is useful for token usage, but it is not a `result` record. It does not say success or failure, does not repeat the final answer, and does not identify the session. Any prior `error` event should taint the run even if `complete` appears.

This research did not capture a live local fixture because no `goose` binary was installed in the environment. The stream schema and behavior above are source-backed, but exact process exit codes and exact spinner behavior under piped non-TTY stdout/stderr remain gaps.

## Claudine Integration Notes

Use this command shape for normal wrapper execution:

```sh
goose run --quiet --output-format stream-json --name "$CLAUDINE_RUN_ID" -i -
```

Add `--provider`, `--model`, `--max-turns`, `--max-tool-repetitions`, `--no-profile`, and explicit extension flags when Claudine needs deterministic runtime behavior. Avoid `--interactive`. Avoid `--no-session` when resume/recovery matters. Avoid effective `GOOSE_MODE=approve` and `GOOSE_MODE=smart_approve` for headless automation unless the wrapper can guarantee no confirmation will be requested.

Parse stdout as NDJSON only when `--quiet` is present. Use `type` for the top-level event. For `message`, inspect `message.content[].type`; for `actionRequired`, inspect `message.content[].data.actionType`. Join tool requests and responses by content `id`. Treat `notification` as useful progress/log data, but do not rely on it for completion. Treat any `error` event as failure or ambiguous failure even if `complete` follows. If no valid JSON arrives, fall back to stderr, non-JSON stdout, and process exit.

Claudine should add its own run metadata record before process start: Goose version, cwd, command, requested provider/model, generated session name, effective known environment overrides, and intended permission mode. Goose's stream does not provide those fields early enough for robust lifecycle reporting.

## Changelog

- 2026-07-03: Re-researched Goose CLI from official docs and current AAIF source. Updated the recommended invocation to require `--quiet`, documented the Rust stream schema, headless approval behavior, missing init/session metadata, and error-plus-complete semantics.

## Sources

- [Running Tasks](https://goose-docs.ai/docs/guides/running-tasks/) - official `goose run` invocation, stdin, recipes, session management, provider/model flags, extensions, debug, and JSON output docs.
- [Configuration Files](https://goose-docs.ai/docs/guides/config-files/) - official config paths, config precedence, global settings, extension config, and keyring fallback notes.
- [Environment Variables](https://goose-docs.ai/docs/guides/environment-variables/) - official provider/model, headless context strategy, `GOOSE_MODE`, debug, keyring, shell, token/context, subagent, and observability variables.
- [Logging System](https://goose-docs.ai/docs/guides/logs/) - official session database, CLI logs, request logs, and local storage paths.
- [MCP Elicitation](https://goose-docs.ai/docs/guides/mcp-elicitation/) - official MCP elicitation behavior context.
- [MCP Roots](https://goose-docs.ai/docs/guides/mcp-roots/) - official MCP roots context.
- [`crates/goose-cli/src/cli.rs`](https://github.com/aaif-goose/goose/blob/main/crates/goose-cli/src/cli.rs) - Clap command/flag definitions and `handle_run_command`.
- [`crates/goose-cli/src/session/mod.rs`](https://github.com/aaif-goose/goose/blob/main/crates/goose-cli/src/session/mod.rs) - `JsonOutput`, `JsonMetadata`, `StreamEvent`, `NotificationData`, event emission, headless approval/elicitation handling, error handling, and completion.
- [`crates/goose-cli/src/session/builder.rs`](https://github.com/aaif-goose/goose/blob/main/crates/goose-cli/src/session/builder.rs) - provider/model precedence, session id resolution, extension loading, fallback behavior, and session banner emission.
- [`crates/goose-provider-types/src/conversation/message.rs`](https://github.com/aaif-goose/goose/blob/main/crates/goose-provider-types/src/conversation/message.rs) - nested message, content, tool, action-required, timestamp, and inference metadata serde types.
- [Issue #8933: partial tool-use deltas in stream-json](https://github.com/aaif-goose/goose/issues/8933) - upstream discussion of `stream-json` buffering complete tool-use blocks and the wrapper-visible liveness gap.
