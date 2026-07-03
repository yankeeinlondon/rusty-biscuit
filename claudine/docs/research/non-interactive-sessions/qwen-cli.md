---
$schema: ./_schema.yaml
created: 2026-07-02
last_updated: 2026-07-02
agent: codex
model: default
docs: https://qwenlm.github.io/qwen-code-docs/en/users/features/headless/
invocation:
  - command: 'qwen -p "prompt" --output-format stream-json --include-partial-messages'
    stdin_support: true
    prompt_arg: "--prompt/-p string; piped stdin can provide additional prompt/context in text input mode"
    notes: "Starts a fresh one-shot headless session and emits one JSON object per stdout line."
  - command: 'cat input.txt | qwen -p "prompt" --output-format stream-json --include-partial-messages'
    stdin_support: true
    prompt_arg: "--prompt/-p plus text stdin"
    notes: "Fresh one-shot headless session with prompt text plus piped input."
  - command: 'qwen --continue -p "prompt" --output-format stream-json --include-partial-messages'
    stdin_support: true
    prompt_arg: "--prompt/-p string"
    notes: "Continues the most recent project-scoped session from saved JSONL history."
  - command: 'qwen --resume <session-id> -p "prompt" --output-format stream-json --include-partial-messages'
    stdin_support: true
    prompt_arg: "--prompt/-p string; --resume without an ID can open a picker and should be avoided in automation"
    notes: "Resumes a specific project-scoped saved session."
  - command: 'qwen --session-id <uuid> -p "prompt" --output-format stream-json --include-partial-messages'
    stdin_support: true
    prompt_arg: "--prompt/-p string"
    notes: "Starts a new session using a caller-supplied UUID; cannot be combined with --continue or --resume."
  - command: 'qwen --input-format stream-json --output-format stream-json'
    stdin_support: true
    prompt_arg: "stdin is a bidirectional JSON-line protocol, not plain prompt text"
    notes: "SDK/control-plane mode; supports multi-turn input and control requests/responses on the stream."
  - command: 'qwen -p "prompt" --output-format json'
    stdin_support: true
    prompt_arg: "--prompt/-p string or piped stdin"
    notes: "One-shot headless session with a final JSON array only after process completion."
  - command: 'qwen -p "prompt" --output-format text'
    stdin_support: true
    prompt_arg: "--prompt/-p string or piped stdin"
    notes: "Human-readable one-shot mode; not suitable for Claudine lifecycle parsing."
output_formats:
  - name: "text"
    cli_value: "text"
    stream: true
    format: text
    description: "Default human-readable response text. With --json-schema, successful text mode stdout becomes the validated JSON payload only."
    side_effects: "Not parser-safe for general supervision; errors and diagnostics go to stderr."
  - name: "json"
    cli_value: "json"
    stream: false
    format: json
    description: "A single JSON array of message objects emitted after completion. The final result can include stats and structured_result."
    side_effects: "No live progress; richer stats are present here but not in stream-json."
  - name: "stream-json"
    cli_value: "stream-json"
    stream: true
    format: jsonl
    description: "Line-delimited JSON objects on stdout. Without --include-partial-messages it emits completed system/assistant/user/result messages; with the flag it also emits stream_event records."
    side_effects: "Best Claudine mode. Stdout is parseable JSONL; stderr remains necessary for diagnostics and retry heartbeats."
  - name: "stream-json input/control"
    cli_value: "stream-json"
    stream: true
    format: jsonl
    description: "When selected via --input-format stream-json together with --output-format stream-json, stdin/stdout form a bidirectional JSON-line control protocol."
    side_effects: "Stdin is no longer prompt text. The host may need to answer control_request messages, including permission requests."
schema_sources:
  - url: "https://github.com/QwenLM/qwen-code/blob/main/packages/cli/src/nonInteractive/types.ts"
    schema_type: typescript
    formal: false
    notes: "Closest source-level union for CLIMessage, StreamEvent, ControlMessage, result messages, permission denials, and metadata."
  - url: "https://github.com/QwenLM/qwen-code/blob/main/packages/sdk-typescript/src/types/protocol.ts"
    schema_type: typescript
    formal: false
    notes: "SDK-facing protocol types; useful for integration shape, but may lag or be narrower than CLI-local types."
  - url: "https://github.com/QwenLM/qwen-code/blob/main/packages/cli/src/nonInteractive/io/BaseJsonOutputAdapter.ts"
    schema_type: typescript
    formal: false
    notes: "Authoritative implementation for result construction, permission_denials, tool_result messages, and structured_result."
  - url: "https://github.com/QwenLM/qwen-code/blob/main/packages/cli/src/nonInteractive/io/StreamJsonOutputAdapter.ts"
    schema_type: typescript
    formal: false
    notes: "Authoritative implementation for JSONL framing and optional stream_event emission."
  - url: "https://qwenlm.github.io/qwen-code-docs/en/users/features/headless/"
    schema_type: examples
    formal: false
    notes: "Official docs for headless entry points and output formats; examples still show session_start while current source emits init."
  - url: "https://qwenlm.github.io/qwen-code-docs/en/developers/sdk-typescript/"
    schema_type: examples
    formal: false
    notes: "Documents SDK query iteration, session methods, permission handler behavior, and timeout behavior."
cli_params:
  - flag: "-p, --prompt"
    value: "string"
    description: "Pass prompt text and enter non-interactive mode."
    example: 'qwen -p "review this repo" --output-format stream-json'
  - flag: "--output-format, -o"
    value: "text | json | stream-json"
    description: "Select output mode. Claudine should pass stream-json explicitly."
    example: 'qwen -p "run" --output-format stream-json'
  - flag: "--input-format"
    value: "text | stream-json"
    description: "Select stdin mode. stream-json requires --output-format stream-json and turns stdin into protocol messages."
    example: "qwen --input-format stream-json --output-format stream-json"
  - flag: "--include-partial-messages"
    value: "boolean"
    description: "Emit stream_event records such as message_start, content_block_delta, tool_progress, and active_goal in stream-json mode."
    example: 'qwen -p "run" --output-format stream-json --include-partial-messages'
  - flag: "--system-prompt"
    value: "string"
    description: "Replace the built-in main-session system prompt for the run."
    example: 'qwen -p "review" --system-prompt "You are a terse reviewer."'
  - flag: "--append-system-prompt"
    value: "string"
    description: "Append run-specific instructions after the built-in prompt and loaded context."
    example: 'qwen -p "run" --append-system-prompt "Do not ask the user questions."'
  - flag: "--approval-mode"
    value: "plan | default | auto-edit | auto | yolo"
    description: "Select tool approval behavior. Use yolo only with an external sandbox; use plan/read-only or explicit permissions for safer automation."
    example: 'qwen -p "inspect" --approval-mode plan --output-format stream-json'
  - flag: "--yolo, -y"
    value: "boolean"
    description: "Legacy/convenience auto-approval for all tool calls. Cannot be combined with --approval-mode."
    example: 'qwen -p "fix tests" --yolo --sandbox --output-format stream-json'
  - flag: "--sandbox, -s"
    value: "boolean"
    description: "Enable sandbox mode for shell/edit/write execution."
    example: 'qwen -p "fix" --sandbox --approval-mode yolo --output-format stream-json'
  - flag: "--safe-mode"
    value: "boolean"
    description: "Disable settings-sourced customizations, context files, hooks, extensions, skills, MCP servers, custom subagents, permission rules, memory features, and sandbox settings; CLI yolo/approval-mode still apply."
    example: 'qwen -p "diagnose" --safe-mode --output-format stream-json'
  - flag: "--allowed-tools"
    value: "tool pattern list"
    description: "Bypass confirmation for matching tools."
    example: 'qwen -p "inspect" --allowed-tools "Shell(git status)"'
  - flag: "--core-tools"
    value: "tool list"
    description: "Limit built-in/core tool registration; parser-relevant because it changes tools emitted in the init message."
    example: 'qwen -p "read only" --core-tools read_file,grep_search'
  - flag: "--exclude-tools"
    value: "tool list or patterns"
    description: "Deny tools; useful to disable shell/write/edit/agent in unattended runs."
    example: 'qwen -p "audit" --exclude-tools shell,write,edit,agent'
  - flag: "--include-directories"
    value: "directory list"
    description: "Add up to five extra workspace directories."
    example: 'qwen -p "inspect" --include-directories ../shared'
  - flag: "--all-files, -a"
    value: "boolean"
    description: "Recursively include files under the current directory as context."
    example: 'qwen -p "summarize" --all-files'
  - flag: "--model, -m"
    value: "model id"
    description: "Requested model for the run; init.model reports config.getModel()."
    example: 'qwen -p "run" --model qwen3-coder-plus'
  - flag: "--auth-type"
    value: "openai | anthropic | qwen-oauth | gemini | vertex-ai"
    description: "Select auth/provider protocol when configured."
    example: 'qwen -p "run" --auth-type openai'
  - flag: "--continue"
    value: "boolean"
    description: "Continue the most recent saved session for the project."
    example: 'qwen --continue -p "continue" --output-format stream-json'
  - flag: "--resume, -r"
    value: "session id or title"
    description: "Resume a saved project session. Avoid bare --resume in automation because it can require interactive selection."
    example: 'qwen --resume 123e4567-e89b-12d3-a456-426614174000 -p "continue"'
  - flag: "--session-id"
    value: "uuid"
    description: "Start a fresh session with a caller-supplied UUID; mutually exclusive with resume/continue."
    example: 'qwen --session-id 123e4567-e89b-12d3-a456-426614174000 -p "run"'
  - flag: "--max-session-turns"
    value: "number"
    description: "Abort when turn cap is exceeded; documented exit code is 53."
    example: 'qwen -p "run" --max-session-turns 20'
  - flag: "--max-wall-time"
    value: "duration"
    description: "Abort an unattended one-shot or stream-json-input run on wall-clock budget; documented exit code is 55."
    example: 'qwen -p "run" --max-wall-time 10m'
  - flag: "--max-tool-calls"
    value: "number"
    description: "Abort after cumulative top-level tool-call budget; subagent inner calls are not counted."
    example: 'qwen -p "run" --max-tool-calls 50'
  - flag: "--max-subagent-depth"
    value: "number"
    description: "Limit subagent nesting depth."
    example: 'qwen -p "run" --max-subagent-depth 1'
  - flag: "--json-schema"
    value: "JSON literal or @path"
    description: "Constrain final answer to JSON Schema. In stream-json, final result has structured_result; incompatible with stream-json input and ACP."
    example: 'qwen -p "extract" --json-schema @schema.json --output-format stream-json'
  - flag: "--json-file"
    value: "path"
    description: "Dual-output JSON file path; can duplicate structured records outside stdout."
    example: 'qwen -p "run" --json-file out.jsonl'
  - flag: "--json-fd"
    value: "fd"
    description: "Dual-output JSON file descriptor; parser-relevant as a secondary structured stream."
    example: 'qwen -p "run" --json-fd 3'
  - flag: "--input-file"
    value: "path | FIFO | /dev/fd/N"
    description: "Read prompt/input from a file-like source."
    example: 'qwen -p "summarize" --input-file prompt.txt'
  - flag: "--debug, -d"
    value: "boolean"
    description: "Enable verbose diagnostic logging; do not treat stderr as parse-only."
    example: 'qwen -p "run" --debug --output-format stream-json'
  - flag: "--openai-logging"
    value: "boolean"
    description: "Enable API call logging for debugging; may write logs outside the stdout stream."
    example: 'qwen -p "run" --openai-logging'
  - flag: "--openai-logging-dir"
    value: "path"
    description: "Directory for OpenAI API logs."
    example: 'qwen -p "run" --openai-logging --openai-logging-dir ~/qwen-logs'
  - flag: "--allowed-mcp-server-names"
    value: "server names"
    description: "Restrict configured MCP servers made available during the run."
    example: 'qwen -p "run" --allowed-mcp-server-names github'
  - flag: "--mcp-config"
    value: "JSON string or path-like config"
    description: "Inject MCP server configuration at top precedence."
    example: 'qwen -p "run" --mcp-config ''{"servers":{}}'''
  - flag: "--acp"
    value: "boolean"
    description: "Starts ACP integration mode, not the preferred Claudine stream-json CLI wrapper."
    example: "qwen --acp"
  - flag: "--prompt-interactive, -i"
    value: "string"
    description: "Starts the interactive UI; conflicts with headless automation and --json-schema."
    example: 'qwen -i "interactive prompt"'
config_files:
  - os: linux
    scope: system
    path: "/etc/qwen-code/system-defaults.json"
    format: json
    effect: "Lowest-precedence system defaults for settings, including model, output, permissions, MCP, telemetry, sandbox, and tools."
    notes: "Can be overridden by QWEN_CODE_SYSTEM_DEFAULTS_PATH."
  - os: macos
    scope: system
    path: "/Library/Application Support/QwenCode/system-defaults.json"
    format: json
    effect: "Lowest-precedence system defaults."
    notes: "Can be overridden by QWEN_CODE_SYSTEM_DEFAULTS_PATH."
  - os: windows
    scope: system
    path: "C:\\ProgramData\\qwen-code\\system-defaults.json"
    format: json
    effect: "Lowest-precedence system defaults."
    notes: "Can be overridden by QWEN_CODE_SYSTEM_DEFAULTS_PATH."
  - os: all
    scope: user
    path: "~/.qwen/settings.json or $QWEN_HOME/settings.json"
    format: json
    effect: "User settings for model, output, permissions, auth selection, MCP, tools, telemetry, hooks, extensions, memory, and context."
    notes: "Overrides system defaults; overridden by trusted project settings, system settings, env vars, and CLI flags."
  - os: all
    scope: repo
    path: ".qwen/settings.json"
    format: json
    effect: "Project settings for model, output, permissions, MCP, tools, hooks, context, and other behavior."
    notes: "Loaded only when workspace settings are active/trusted; overrides user settings and is overridden by system settings, env vars, and CLI flags."
  - os: linux
    scope: system
    path: "/etc/qwen-code/settings.json"
    format: json
    effect: "Highest-precedence system settings override."
    notes: "Can be overridden by QWEN_CODE_SYSTEM_SETTINGS_PATH."
  - os: macos
    scope: system
    path: "/Library/Application Support/QwenCode/settings.json"
    format: json
    effect: "Highest-precedence system settings override."
    notes: "Can be overridden by QWEN_CODE_SYSTEM_SETTINGS_PATH."
  - os: windows
    scope: system
    path: "C:\\ProgramData\\qwen-code\\settings.json"
    format: json
    effect: "Highest-precedence system settings override."
    notes: "Can be overridden by QWEN_CODE_SYSTEM_SETTINGS_PATH."
  - os: all
    scope: repo
    path: ".mcp.json"
    format: json
    effect: "Project MCP server definitions can affect init.mcp_servers and available tools."
    notes: "Source comments describe precedence below CLI/session MCP config and system/workspace settings; project/workspace MCP servers may be gated by trust."
  - os: all
    scope: user
    path: "<QWEN_HOME>/.env or ~/.qwen/.env"
    format: text
    effect: "Qwen-specific environment overrides for auth, logging, sandbox, retry, color, and other behavior."
    notes: "Loaded before ~/.env and wins over ~/.env when both user-level env files define the same variable; existing process env values are not overwritten."
  - os: all
    scope: user
    path: "~/.env"
    format: text
    effect: "General user environment variables."
    notes: "Lower precedence than <QWEN_HOME>/.env for user-level env loading."
  - os: all
    scope: repo
    path: ".qwen/.env"
    format: text
    effect: "Project-specific Qwen environment variables."
    notes: "Project env loading can exclude variables such as DEBUG and DEBUG_MODE unless placed in .qwen/.env."
  - os: all
    scope: user
    path: "~/.qwen/projects/<sanitized-cwd>/chats/*.jsonl"
    format: json
    effect: "Project-scoped saved session history for --continue and --resume."
    notes: "The exact sanitized path is derived from cwd; resumes restore history, tool outputs, and compression checkpoints."
env_vars:
  - name: "QWEN_HOME"
    effect: "Overrides the global configuration directory."
    notes: "Affects user settings, credentials, memory, skills, and other global state."
  - name: "QWEN_RUNTIME_DIR"
    effect: "Overrides runtime output directory for conversations, logs, and todos."
    notes: "Useful for separating ephemeral runtime data from persistent config."
  - name: "QWEN_CODE_SYSTEM_DEFAULTS_PATH"
    effect: "Overrides system defaults settings path."
    notes: "Can change effective settings for every run."
  - name: "QWEN_CODE_SYSTEM_SETTINGS_PATH"
    effect: "Overrides high-precedence system settings path."
    notes: "Can force managed settings that users/projects cannot override except by CLI/env where applicable."
  - name: "QWEN_TELEMETRY_ENABLED"
    effect: "Overrides telemetry.enabled."
    notes: "Parser-adjacent because telemetry/logging may create side-channel records."
  - name: "QWEN_TELEMETRY_TARGET"
    effect: "Sets telemetry target label."
    notes: "Use endpoint/outfile variables for routing."
  - name: "QWEN_TELEMETRY_OTLP_ENDPOINT"
    effect: "Configures telemetry OTLP endpoint."
    notes: "Overrides telemetry settings."
  - name: "QWEN_TELEMETRY_OTLP_PROTOCOL"
    effect: "Configures telemetry protocol."
    notes: "Overrides telemetry settings."
  - name: "QWEN_TELEMETRY_OUTFILE"
    effect: "Routes telemetry to a file."
    notes: "Useful secondary evidence stream, but not equivalent to stdout stream-json."
  - name: "QWEN_CODE_UNATTENDED_RETRY"
    effect: "When set to true or 1, retries transient HTTP 429/529 errors indefinitely with stderr heartbeats."
    notes: "CI=true alone does not enable it; combine with --max-wall-time."
  - name: "QWEN_CODE_SUPPRESS_YOLO_WARNING"
    effect: "Suppresses the stderr warning for YOLO without sandbox."
    notes: "Does not make YOLO safe."
  - name: "QWEN_SANDBOX"
    effect: "Enables sandbox mode."
    notes: "Docs recommend this for local/shared unattended yolo runs."
  - name: "QWEN_SANDBOX_IMAGE"
    effect: "Selects sandbox image."
    notes: "Precedence is --sandbox-image, QWEN_SANDBOX_IMAGE, tools.sandboxImage, built-in default."
  - name: "SEATBELT_PROFILE"
    effect: "Selects macOS sandbox-exec profile."
    notes: "macOS-specific; custom profiles live under project .qwen/."
  - name: "DEBUG"
    effect: "Enables verbose debug logging when true or 1."
    notes: "Can add stderr noise; excluded from project env by default unless in .qwen/.env."
  - name: "DEBUG_MODE"
    effect: "Enables verbose debug logging when true or 1."
    notes: "Same parser warning as DEBUG."
  - name: "NO_COLOR"
    effect: "Disables colored output."
    notes: "Structured stdout is already JSON; useful for text/stderr cleanliness."
  - name: "FORCE_HYPERLINK"
    effect: "Forces or disables OSC 8 hyperlinks in markdown rendering."
    notes: "Text-mode/stderr concern."
  - name: "QWEN_DISABLE_HYPERLINKS"
    effect: "Disables OSC 8 hyperlinks."
    notes: "Text-mode/stderr concern."
  - name: "CLI_TITLE"
    effect: "Customizes terminal title."
    notes: "TUI-oriented; not a stream field."
  - name: "QWEN_CODE_MAX_OUTPUT_TOKENS"
    effect: "Overrides default maximum output tokens per response."
    notes: "Can change truncation/escalation behavior and token usage."
  - name: "QWEN_CODE_LEGACY_MCP_BLOCKING"
    effect: "Restores synchronous MCP discovery during initialization."
    notes: "Can delay init/tool availability; modern default discovers progressively."
  - name: "QWEN_DISABLED_SLASH_COMMANDS"
    effect: "Adds disabled slash commands."
    notes: "Unioned with settings and CLI flag."
  - name: "QWEN_CODE_SAFE_MODE"
    effect: "Enables safe mode."
    notes: "Disables many settings-sourced customizations; CLI approval flags still apply."
  - name: "QWEN_CODE_CLI_PATH"
    effect: "SDK executable discovery override."
    notes: "SDK integration only."
  - name: "OPENAI_API_KEY"
    effect: "Auth credential for OpenAI-compatible providers."
    notes: "Do not log value; auth failures surface as errors, not a dedicated auth event."
  - name: "OPENAI_BASE_URL"
    effect: "OpenAI-compatible base URL."
    notes: "Can redirect provider/backend."
  - name: "OPENAI_MODEL"
    effect: "Default model for OpenAI-compatible provider."
    notes: "QWEN_MODEL is an alias according to auth docs."
  - name: "QWEN_MODEL"
    effect: "Alias for OpenAI-compatible model selection."
    notes: "May be superseded by CLI --model."
  - name: "ANTHROPIC_API_KEY"
    effect: "Auth credential for Anthropic provider."
    notes: "Do not log value."
  - name: "GEMINI_API_KEY"
    effect: "Auth credential for Gemini provider."
    notes: "Do not log value."
io_contract:
  stdout: structured_only
  stderr: mixed
  stdin: prompt
  framing: jsonl
  noise_handling: "In stream-json output mode, parse stdout line-by-line as JSON. Keep stderr as diagnostics/lifecycle text for startup warnings, API errors, budget/auth messages, debug logs, and persistent-retry heartbeats."
  notes: "When --input-format stream-json is also used, stdin becomes a bidirectional protocol instead of prompt text."
stream_contract:
  discriminator: "type; for stream_event use event.type; for control_request use request.subtype; for control_response use response.subtype; content blocks use message.content[].type"
  event_ordering: "Initial system init message is emitted before run output; assistant/tool messages follow execution order; result is terminal for one-shot runs when emitted, but some process-level failures can exit without a result."
  correlation_fields: ["session_id", "uuid", "message.id", "parent_tool_use_id", "message.content[].id", "message.content[].tool_use_id", "request_id", "event.tool_use_id"]
  terminal_event: "type=result"
  partial_message_events: true
  unknown_event_policy: "Skip unknown type/event.type values, preserve raw JSON for drift review, and continue parsing known terminal result records."
  notes: "Partial events require --include-partial-messages. Tool inputs in partial mode arrive as input_json_delta.partial_json, but current source emits the full JSON-stringified input rather than fine-grained deltas."
session_metadata:
  session_id: "system.session_id and result.session_id; system.uuid equals session id in current source"
  cwd: "system.cwd"
  model: "system.model and assistant.message.model"
  provider: "not emitted directly; infer from auth/config/modelProviders when available"
  auth: "not emitted directly; selected auth type is config-driven but absent from stream records"
  version: "system.qwen_code_version"
  mcp_servers: "system.mcp_servers[].name/status"
  permission_mode: "system.permission_mode"
  notes: "Current source emits system.subtype=init with tools, MCP server statuses, slash_commands, qwen_code_version, and agents. Docs examples still show subtype=session_start."
stream_events:
  - event: "system/init"
    category: session
    fields: ["type", "subtype", "uuid", "session_id", "cwd", "tools", "mcp_servers", "model", "permission_mode", "slash_commands", "qwen_code_version", "agents"]
    notes: "First current-source system record for a run."
  - event: "system/worktree_started"
    category: session
    fields: ["type", "subtype", "uuid", "session_id", "parent_tool_use_id", "data.notice"]
    notes: "Emitted when startup worktree context is injected."
  - event: "system/worktree_restored"
    category: session
    fields: ["type", "subtype", "uuid", "session_id", "data.notice"]
    notes: "Emitted when resumed worktree context is restored."
  - event: "assistant"
    category: assistant
    fields: ["type", "uuid", "session_id", "parent_tool_use_id", "message.id", "message.model", "message.content", "message.stop_reason", "message.usage"]
    notes: "Completed assistant message. content blocks are one of text, thinking, tool_use, or tool_result-shaped data depending on message category."
  - event: "user"
    category: tool_result
    fields: ["type", "uuid", "session_id", "parent_tool_use_id", "message.role", "message.content[].tool_use_id", "message.content[].content", "message.content[].is_error"]
    notes: "Used for tool_result content and subagent prompt/user messages."
  - event: "stream_event/message_start"
    category: assistant
    fields: ["type", "uuid", "session_id", "parent_tool_use_id", "event.type", "event.message.id", "event.message.role", "event.message.model"]
    notes: "Partial message start; requires --include-partial-messages."
  - event: "stream_event/content_block_start"
    category: assistant
    fields: ["event.index", "event.content_block.type", "event.content_block.id", "event.content_block.name", "event.content_block.input", "parent_tool_use_id"]
    notes: "Starts text, thinking, or tool_use content block."
  - event: "stream_event/content_block_delta"
    category: assistant
    fields: ["event.index", "event.delta.type", "event.delta.text", "event.delta.thinking", "event.delta.partial_json"]
    notes: "Partial text/thinking/tool-input updates."
  - event: "stream_event/content_block_stop"
    category: assistant
    fields: ["event.index"]
    notes: "Closes a partial content block."
  - event: "stream_event/message_stop"
    category: assistant
    fields: ["event.type"]
    notes: "Closes a partial assistant message."
  - event: "stream_event/tool_progress"
    category: tool_call
    fields: ["event.tool_use_id", "event.content"]
    notes: "MCP progress only; requires --include-partial-messages and a tool that emits McpToolProgressData."
  - event: "stream_event/active_goal"
    category: plan
    fields: ["event.active_goal"]
    notes: "Session-level goal state update; current source emits only when partial stream events are enabled."
  - event: "control_request/can_use_tool"
    category: permission
    fields: ["request_id", "request.tool_name", "request.tool_use_id", "request.input", "request.permission_suggestions", "request.blocked_path"]
    notes: "Only in stream-json input/control mode or SDK-backed permission path."
  - event: "control_request/initialize"
    category: session
    fields: ["request_id", "request.hooks", "request.timeout.canUseTool", "request.sdkMcpServers", "request.mcpServers", "request.agents"]
    notes: "Control-plane initialization request shape."
  - event: "control_request/set_permission_mode"
    category: permission
    fields: ["request_id", "request.mode"]
    notes: "Control-plane permission mode change."
  - event: "control_request/set_model"
    category: session
    fields: ["request_id", "request.model"]
    notes: "Control-plane model change."
  - event: "control_request/get_context_usage"
    category: usage
    fields: ["request_id", "request.show_details"]
    notes: "Control-plane context usage query."
  - event: "control_response/success"
    category: other
    fields: ["response.request_id", "response.response"]
    notes: "Acknowledges control requests."
  - event: "control_response/error"
    category: error
    fields: ["response.request_id", "response.error"]
    notes: "Reports control-plane request errors."
  - event: "control_cancel_request"
    category: other
    fields: ["request_id"]
    notes: "Control-plane cancellation."
  - event: "result/success"
    category: session
    fields: ["type", "subtype", "uuid", "session_id", "is_error", "duration_ms", "duration_api_ms", "num_turns", "result", "usage", "modelUsage", "permission_denials", "structured_result", "stats"]
    notes: "Terminal success for one-shot JSON/stream-json runs. stats is only emitted by json output in current source."
  - event: "result/error_during_execution"
    category: error
    fields: ["type", "subtype", "uuid", "session_id", "is_error", "duration_ms", "duration_api_ms", "num_turns", "usage", "permission_denials", "error.message"]
    notes: "Terminal structured error when adapter emission succeeds."
  - event: "result/error_max_turns"
    category: error
    fields: ["type", "subtype", "uuid", "session_id", "is_error", "duration_ms", "duration_api_ms", "num_turns", "usage", "permission_denials", "error.message"]
    notes: "Typed in source; docs say max-session-turns can also exit 53 without a stdout result in some paths."
tools:
  - name: "read_file"
    call_visible: true
    result_visible: true
    metadata: ["tool_use.id", "tool_use.name", "tool_use.input", "tool_result.tool_use_id", "tool_result.content", "tool_result.is_error"]
    notes: "Read results are user/tool_result messages. Permission denials also aggregate in final result.permission_denials."
  - name: "list_directory"
    call_visible: true
    result_visible: true
    metadata: ["input", "content", "is_error"]
    notes: "Same general tool_use/tool_result shape."
  - name: "grep_search"
    call_visible: true
    result_visible: true
    metadata: ["input", "content", "is_error"]
    notes: "Same general tool_use/tool_result shape."
  - name: "glob"
    call_visible: true
    result_visible: true
    metadata: ["input", "content", "is_error"]
    notes: "Same general tool_use/tool_result shape."
  - name: "edit"
    call_visible: true
    result_visible: true
    metadata: ["path in input when present", "tool_result.content", "permission_denials"]
    notes: "No dedicated file_change event; infer writes from tool name/input/result."
  - name: "write_file"
    call_visible: true
    result_visible: true
    metadata: ["path in input when present", "tool_result.content", "permission_denials"]
    notes: "No dedicated file_change event; infer writes from tool name/input/result."
  - name: "run_shell_command"
    call_visible: true
    result_visible: true
    metadata: ["command input", "tool_result.content", "is_error"]
    notes: "Exit code/stdout/stderr are not guaranteed as separately typed fields in the CLI stream; they may be embedded in content/resultDisplay."
  - name: "agent"
    call_visible: true
    result_visible: true
    metadata: ["parent_tool_use_id", "subagent user/assistant/result messages", "task notifications where surfaced"]
    notes: "Subagent messages are linked to the parent agent tool call by parent_tool_use_id."
  - name: "skill"
    call_visible: true
    result_visible: true
    metadata: ["tool_use", "tool_result"]
    notes: "Can affect model override internally; stream should still be parsed as ordinary tool records."
  - name: "todo_write"
    call_visible: true
    result_visible: true
    metadata: ["tool_use", "tool_result"]
    notes: "Planning/todo state is not a dedicated normalized plan event except active_goal when emitted."
  - name: "ask_user_question"
    call_visible: true
    result_visible: true
    metadata: ["tool_use", "tool_result", "permission/control behavior unknown"]
    notes: "Human answer behavior in plain one-shot headless mode was not fully verified; treat as human-in-loop risk."
  - name: "exit_plan_mode"
    call_visible: true
    result_visible: true
    metadata: ["tool_use", "tool_result"]
    notes: "Relevant when plan workflow is active."
  - name: "web_fetch"
    call_visible: true
    result_visible: true
    metadata: ["tool_use", "tool_result", "permission_denials"]
    notes: "Network/tool permission policy can deny."
  - name: "web_search"
    call_visible: true
    result_visible: true
    metadata: ["tool_use", "tool_result", "usage.server_tool_use.web_search_requests"]
    notes: "Usage may include web_search_requests in ExtendedUsage."
  - name: "MCP tools"
    call_visible: true
    result_visible: true
    metadata: ["system.mcp_servers", "tool_use.name", "tool_progress", "tool_result"]
    notes: "tool_progress is visible only as stream_event/tool_progress with --include-partial-messages."
  - name: "structured_output"
    call_visible: true
    result_visible: true
    metadata: ["result.structured_result", "result.result"]
    notes: "Synthetic tool for --json-schema. Final result carries structured_result; this tool is exempt from --max-tool-calls but not --max-session-turns."
completion:
  success_event: "result with subtype=success and is_error=false"
  failure_event: "result with is_error=true when emitted; otherwise rely on process exit code and stderr"
  exit_code_reliable: false
  result_fields: ["result.result", "result.structured_result", "result.error.message", "result.permission_denials", "result.num_turns", "result.duration_ms", "result.duration_api_ms"]
  cost_fields: ["unknown; stats/modelUsage may help but no stable cost field verified"]
  usage_fields: ["result.usage.input_tokens", "result.usage.output_tokens", "result.usage.cache_read_input_tokens", "result.usage.cache_creation_input_tokens", "result.usage.total_tokens", "result.usage.server_tool_use.web_search_requests", "result.modelUsage", "result.stats.models"]
  notes: "Docs state some failures such as max-session-turns exit 53 and signal interrupts exit 130 with stderr only. Budget errors are documented as exit 55 and source tries to emit a terminal result before exiting when possible."
blocking_behavior:
  permissions: configurable
  questions: unknown
  tool_approvals: configurable
  notes: "Plain one-shot headless mode cannot prompt through a TUI; non-stream-json teammate approvals auto-cancel unless YOLO. SDK docs say default SDK behavior auto-denies write tools unless canUseTool/allowedTools/yolo approves, and canUseTool has a timeout. stream-json input/control mode can emit can_use_tool requests that a host must answer."
subagents:
  supported: true
  start_visible: true
  stop_visible: true
  nested_events_visible: true
  prompt_injection_supported: true
  metadata_fields: ["parent_tool_use_id", "system.agents", "subagent user messages", "subagent assistant messages", "subagent result errors"]
  notes: "The agent tool is visible as a normal tool call; child messages and tool calls are linked by parent_tool_use_id. --max-tool-calls counts the top-level agent call but not inner subagent calls; use --exclude-tools agent or --max-subagent-depth for tighter automation bounds."
use_cases:
  - name: "plan_cap_approaching"
    detectable: false
    event_types: []
    fields: []
    hook_parity: "unknown"
    notes: "No verified near-cap event for plan/quota."
  - name: "plan_capped"
    detectable: true
    event_types: ["result/error_during_execution", "process exit 55", "process exit 53", "stderr"]
    fields: ["result.error.message", "exit_code"]
    hook_parity: "unknown"
    notes: "Run budgets produce exit 55; max turns produce exit 53. Provider billing quota exhaustion is not a typed stream event."
  - name: "no_funds"
    detectable: false
    event_types: ["result/error_during_execution", "stderr"]
    fields: ["error.message"]
    hook_parity: "unknown"
    notes: "Only generic provider error text was verified."
  - name: "auth"
    detectable: true
    event_types: ["result/error_during_execution", "stderr", "process exit nonzero"]
    fields: ["result.error.message"]
    hook_parity: "unknown"
    notes: "Auth source/kind is not emitted as metadata; classify from error text and configured auth."
  - name: "permission_read_denied"
    detectable: true
    event_types: ["user tool_result", "result"]
    fields: ["message.content[].is_error", "message.content[].content", "result.permission_denials[].tool_name", "result.permission_denials[].tool_input"]
    hook_parity: "unknown"
    notes: "Final permission_denials include tool name, tool_use_id, and full input; no dedicated denied-read event verified."
  - name: "permission_write_denied"
    detectable: true
    event_types: ["user tool_result", "result"]
    fields: ["message.content[].is_error", "result.permission_denials[].tool_name", "result.permission_denials[].tool_input"]
    hook_parity: "unknown"
    notes: "Same as read denial. blocked_path exists in control_request but current source default emits null unless caller supplies it."
  - name: "tokens_consumed"
    detectable: true
    event_types: ["assistant", "result"]
    fields: ["assistant.message.usage", "result.usage", "result.modelUsage", "result.stats.models"]
    hook_parity: "unknown"
    notes: "Assistant usage is per completed assistant message; result.usage is aggregate. stats only appears in json mode in current source."
  - name: "model_used"
    detectable: true
    event_types: ["system/init", "assistant", "stream_event/message_start"]
    fields: ["system.model", "assistant.message.model", "event.message.model"]
    hook_parity: "unknown"
    notes: "Reports configured model string, not necessarily a fully resolved backend route."
  - name: "model_fallback"
    detectable: false
    event_types: []
    fields: []
    hook_parity: "unknown"
    notes: "No dedicated fallback event verified; compare requested --model/env with emitted model as a weak inference only."
  - name: "human_in_loop"
    detectable: true
    event_types: ["assistant tool_use", "control_request/can_use_tool", "stderr"]
    fields: ["message.content[].name", "request.subtype", "request.tool_name"]
    hook_parity: "unknown"
    notes: "ask_user_question tool calls and control permission requests indicate human-in-loop risk; plain one-shot answer behavior remains a gap."
  - name: "session_resumable"
    detectable: true
    event_types: ["system/init", "result"]
    fields: ["session_id"]
    hook_parity: "none"
    notes: "Session ID is emitted at run start and can be used with --resume if chat recording remains enabled."
  - name: "subagent_prompt_injection"
    detectable: true
    event_types: ["system/init", "assistant tool_use agent", "user subagent prompt"]
    fields: ["system.agents", "message.content[].name", "parent_tool_use_id", "message.content"]
    hook_parity: "unknown"
    notes: "Top-level --append-system-prompt can instruct subagents indirectly; SDK/control initialize can provide agents. Direct per-subagent injection surface needs more verification."
headless_constraints:
  - constraint: "Use --output-format stream-json explicitly; text mode is not parser-safe."
    mitigation: "Always pass --output-format stream-json --include-partial-messages for Claudine-supervised runs."
    notes: "json mode is useful only for post-run audit/stats."
  - constraint: "stream-json input mode is a bidirectional protocol."
    mitigation: "Do not use --input-format stream-json unless Claudine is prepared to send/answer protocol messages."
    notes: "The official docs describe it as under construction and intended for SDK integration."
  - constraint: "Default/ask permissions can require approval that plain headless mode cannot supply."
    mitigation: "Use plan/read-only constraints, explicit allow/deny rules, SDK canUseTool, or yolo inside an external sandbox."
    notes: "Do not use --yolo without sandbox for untrusted work."
  - constraint: "YOLO does not imply sandboxing."
    mitigation: "Use --sandbox or QWEN_SANDBOX=1 when auto-approving tools."
    notes: "Qwen prints a startup warning to stderr when yolo has no sandbox."
  - constraint: "Some failures do not emit a terminal result record."
    mitigation: "Treat process exit code and stderr as required lifecycle inputs in addition to stdout JSONL."
    notes: "Structured-output docs call out max-session-turns and signal interrupts."
  - constraint: "stats are missing from stream-json result in current source."
    mitigation: "Use result.usage/modelUsage for live runs, or run json mode for post-run stats when live progress is not needed."
    notes: "Do not require result.stats in Claudine's stream-json parser."
  - constraint: "Subagent inner tool calls are outside --max-tool-calls."
    mitigation: "Set --max-subagent-depth 1 or disable agent tool for strict tool-call caps."
    notes: "Docs explicitly scope tool-call budget to top-level dispatches."
  - constraint: "--json-schema conflicts with --input-format stream-json and ACP."
    mitigation: "Use one-shot stream-json/json output for schema-constrained final payloads."
    notes: "Final result carries structured_result when supported."
  - constraint: "Docs examples show system.subtype=session_start but current source emits init."
    mitigation: "Accept both subtypes and prefer source-backed init for current parser fixtures."
    notes: "Parser should key primarily on type=system and known metadata fields."
quirks:
  - "The best stream schema is TypeScript source, not JSON Schema or OpenAPI."
  - "Official docs examples still show system.subtype=session_start; current source emits system.subtype=init."
  - "Current stream-json partial input_json_delta contains JSON.stringify(input), not necessarily tiny incremental chunks."
  - "result.stats is emitted only for --output-format json in current source; stream-json result has usage and may have modelUsage but not stats."
  - "Permission denied path detail is best recovered from result.permission_denials[].tool_input; control_request.blocked_path exists but defaults to null in the base emitter."
  - "stderr is not ignorable: yolo warnings, API errors, persistent retry heartbeats, debug logs, and some no-result failures appear there."
  - "Qwen OAuth free tier was discontinued on 2026-04-15; unattended runs should use configured Alibaba Cloud/API-key providers."
  - "The local machine inspected during research had qwen 0.15.6, while the document follows current official docs/source as of 2026-07-02."
gaps:
  - "No formal JSON Schema, OpenAPI, AsyncAPI, or versioned schema marker for the CLI stream was found."
  - "Exact behavior of ask_user_question in plain one-shot non-interactive mode was not verified with a live run."
  - "Exact auth/rate-limit/no-funds error payload taxonomy is generic and needs captured fixtures."
  - "Whether all built-in tool outputs preserve command exit code/stdout/stderr as structured fields was not verified; current evidence shows content/resultDisplay projection."
  - "Exact dual-output --json-file/--json-fd framing and parity with stdout stream-json needs separate fixture capture."
  - "Timestamp fields were not found in the CLI stream union."
  - "Cost fields were not verified as stable stream fields."
  - "Direct per-subagent prompt injection controls outside general prompt/agent config need more verification."
claudine_strategy:
  preferred_invocation: 'qwen -p "$PROMPT" --output-format stream-json --include-partial-messages --approval-mode plan --max-session-turns <n> --max-wall-time <duration>'
  required_flags: ["--output-format stream-json", "--include-partial-messages", "--max-session-turns", "--max-wall-time", "one of --approval-mode plan/default/auto-edit/auto/yolo chosen by policy"]
  conflicting_flags: ["--prompt-interactive", "--input-format stream-json unless Claudine implements the control protocol", "--json-schema together with --input-format stream-json", "--acp for normal CLI stream parsing", "--resume without an explicit session id"]
  parser_notes: "Parse stdout as JSONL using type as the top-level discriminator. For stream_event records, branch on event.type. Join tool_use to tool_result by content id/tool_use_id and join subagent activity by parent_tool_use_id. Preserve stderr and exit code for lifecycle classification."
  wrapper_notes: "Prefer plan/read-only for inspection, yolo only under a sandbox. Keep stderr visible in reports. Accept both system subtype init and session_start. Do not assume terminal result on every failure."
data_format: jsonl
changes: []
requires_claudine_update: true
reason: "Qwen's current source-backed stream includes active_goal and control-plane event shapes, uses system subtype init rather than the documented session_start example, and lacks stream-json stats; Claudine parser metadata should reflect those details."
---

## Summary

Qwen Code can run non-interactively and has useful structured output. Claudine should prefer `qwen -p ... --output-format stream-json --include-partial-messages` for ordinary supervised runs because it emits line-delimited JSON on stdout while the process is still active. The stream exposes session initialization, completed assistant messages, tool calls, tool results, optional partial deltas, MCP progress, subagent-linked records, aggregate usage, permission denials, and a terminal `result` record when the run reaches the adapter's normal completion path.

The main risks are schema drift and incomplete failure coverage. Qwen does not publish a formal JSON Schema for the CLI stream; the most reliable schema is the TypeScript union and output-adapter implementation in the repository. Some failures can exit with only stderr and an exit code, `result.stats` currently appears in buffered `json` output but not `stream-json`, and the official Headless docs still show `system.subtype: "session_start"` while current source emits `system.subtype: "init"`. Claudine should parse stdout JSONL, preserve stderr as lifecycle evidence, and treat process exit as necessary but not sufficient.

## Non-Interactive Entry Points

The documented headless entry point is `qwen --prompt/-p`, with prompt text supplied by argv, piped stdin, or both. Headless mode is explicitly described as intended for scripting, automation, CI/CD, and tool-building. It can also resume saved project-scoped sessions using `--continue` or `--resume <session-id>`; the docs say session data is JSONL under `~/.qwen/projects/<sanitized-cwd>/chats`, and that resume restores conversation history, tool outputs, and compression checkpoints.

The safe fresh-run command shape for Claudine is:

```bash
qwen -p "$PROMPT" \
  --output-format stream-json \
  --include-partial-messages \
  --max-session-turns 20 \
  --max-wall-time 10m
```

For read-only or low-risk inspection, add `--approval-mode plan` or explicit `--exclude-tools shell,write,edit,agent`. For autonomous mutation, use `--approval-mode yolo` or `--yolo` only when Claudine has already provided an external sandbox or Qwen's own `--sandbox`/`QWEN_SANDBOX=1` is enabled. Qwen's docs call out that YOLO auto-approves shell/write/edit but does not enable sandboxing by itself.

Qwen also has a bidirectional non-interactive mode:

```bash
qwen --input-format stream-json --output-format stream-json
```

That mode is not just "prompt via JSON." Stdin becomes a JSON-line protocol for SDK/control messages, and stdout can include control requests/responses alongside agent messages. The Headless docs call stream-json input "under construction" and intended for SDK integration, so Claudine should not use it for the basic wrapper until it implements the control protocol.

## Output Formats

| Mode | Selector | Transport | Streams? | Claudine use |
| --- | --- | --- | --- | --- |
| Text | `--output-format text` or default | Plain text | Human output streams as text | Avoid for lifecycle parsing |
| JSON | `--output-format json` | One JSON array after completion | No | Useful for post-run audit and `stats`, not live supervision |
| Stream JSON | `--output-format stream-json` | JSONL/NDJSON on stdout | Yes | Preferred for Claudine |
| Stream JSON control | `--input-format stream-json --output-format stream-json` | Bidirectional JSON-line protocol | Yes | Use only if Claudine hosts the protocol |

`stream-json` is the right default because it gives Claudine progress before process exit. Without `--include-partial-messages`, it still emits completed message envelopes such as `system`, `assistant`, `user`, and `result`. With `--include-partial-messages`, it also emits `stream_event` records such as `message_start`, `content_block_delta`, `tool_progress`, and `active_goal`. This matters for terminal status: Claudine can show that the model is thinking, starting a tool call, receiving tool progress, or producing text instead of waiting for a final JSON array.

Buffered `json` is not useless. Current source adds `stats` only when the output format is `json`, and the Headless docs include jq examples against `.stats.models` and `.stats.tools`. The tradeoff is that Claudine sees nothing live. If the wrapper's goal is a report after completion, `json` can be attractive; if the goal is autonomous-process supervision, `stream-json` wins.

`--json-schema` is a separate structured-output feature. In text mode, successful stdout is the validated JSON payload. In `json`, the final array element is a `result` message carrying both `result` as a stringified payload and `structured_result` as the raw object. In `stream-json`, the terminal `result` line carries the same fields. The docs say `--json-schema` is rejected with `--input-format stream-json` and with ACP, so schema-constrained outputs belong to one-shot text/json/stream-json runs, not the long-lived control protocol.

## Schema Sources

There is no verified formal JSON Schema, OpenAPI, or AsyncAPI document for Qwen CLI's non-interactive stream. The best schema evidence is provider-authored TypeScript:

| Source | Role | Confidence |
| --- | --- | --- |
| [`packages/cli/src/nonInteractive/types.ts`](https://github.com/QwenLM/qwen-code/blob/main/packages/cli/src/nonInteractive/types.ts) | CLI-local unions for `CLIMessage`, `StreamEvent`, `ControlMessage`, results, and permission denials | Highest for the CLI |
| [`packages/cli/src/nonInteractive/io/BaseJsonOutputAdapter.ts`](https://github.com/QwenLM/qwen-code/blob/main/packages/cli/src/nonInteractive/io/BaseJsonOutputAdapter.ts) | Actual construction of assistant/user/result records, permission denials, and structured results | Highest for emitted fields |
| [`packages/cli/src/nonInteractive/io/StreamJsonOutputAdapter.ts`](https://github.com/QwenLM/qwen-code/blob/main/packages/cli/src/nonInteractive/io/StreamJsonOutputAdapter.ts) | JSONL framing and partial stream event emission | Highest for streaming behavior |
| [`packages/sdk-typescript/src/types/protocol.ts`](https://github.com/QwenLM/qwen-code/blob/main/packages/sdk-typescript/src/types/protocol.ts) | SDK-facing protocol types | Useful but not exact; currently narrower than CLI-local source for some stream events |
| [Headless Mode docs](https://qwenlm.github.io/qwen-code-docs/en/users/features/headless/) | Public feature and command documentation | Good for user-facing behavior; examples lag source |
| [TypeScript SDK docs](https://qwenlm.github.io/qwen-code-docs/en/developers/sdk-typescript/) | SDK query and permission behavior | Useful for control-plane behavior |

The schema source mismatch is important. The SDK protocol file is valuable, but the CLI-local type file currently includes `tool_progress` and `active_goal` stream events that are not present in the SDK type excerpt I inspected. Claudine should generate parser fixtures from CLI-local source and use SDK docs/source for the bidirectional control layer.

## IO Contract

In `stream-json` output mode, stdout is one JSON object per line. Qwen's `StreamJsonOutputAdapter` writes `JSON.stringify(message) + "\n"` to stdout for each message. Claudine can parse stdout line-by-line and should treat each line as an independent JSON envelope.

Stderr is not ignorable. Qwen writes startup warnings, API errors, debug logs, YOLO-without-sandbox warnings, and persistent-retry heartbeat lines to stderr. Persistent retry mode is explicitly documented to print heartbeat messages every 30 seconds while retrying transient HTTP 429/529 errors, and the docs say those messages do not appear on stdout, preserving JSON cleanliness. For failure classification, stderr is a secondary lifecycle stream.

Stdin depends on `--input-format`. In the default `text` mode, stdin can be prompt/context text. With `--input-format stream-json`, stdin is reserved for protocol messages and requires `--output-format stream-json`. Claudine should treat that mode as a bidirectional protocol, not as a prettier prompt channel.

## Stream Contract

The top-level discriminator is `type`. Important top-level values are:

| Top-level `type` | Meaning |
| --- | --- |
| `system` | Session/init or runtime system message |
| `assistant` | Completed assistant message |
| `user` | User/tool-result message |
| `stream_event` | Optional partial event, only with `--include-partial-messages` |
| `result` | Terminal success/error envelope when emitted |
| `control_request` | Bidirectional protocol request |
| `control_response` | Bidirectional protocol response |
| `control_cancel_request` | Bidirectional protocol cancellation |

Nested discriminators matter. `stream_event` records branch on `event.type`; control requests branch on `request.subtype`; control responses branch on `response.subtype`; assistant content blocks branch on `message.content[].type`.

Tool calls and results are correlated by the assistant `tool_use` block's `id` and the user `tool_result` block's `tool_use_id`. Subagent activity is linked by `parent_tool_use_id`, which points back to the parent agent tool call. Control-plane request/response records use `request_id`.

Partial-message semantics are optional. With `--include-partial-messages`, Qwen emits `message_start`, `content_block_start`, `content_block_delta`, `content_block_stop`, and `message_stop`. For tool inputs, current source emits an `input_json_delta` with `partial_json: JSON.stringify(input)`; parsers should not assume tiny fragments. Completed `assistant` messages still arrive after partials, so Claudine can use partials for live UI and completed messages for stable transcript records.

The terminal record for ordinary one-shot success is `type: "result", subtype: "success", is_error: false`. Structured failures often emit `type: "result", is_error: true`, but not all failures do. The Structured Output docs explicitly say max-session-turns and signal interrupts can exit with stderr output only. Unknown events should be skipped and preserved for drift review, not treated as fatal parser errors.

## Session Metadata

Current source emits a system init record before the run's substantive output. The fields include:

| Field | Meaning | Presence |
| --- | --- | --- |
| `session_id` | Stable session identifier for logs/resume | Present in system/result records |
| `uuid` | For system init, current source sets this to the session ID | Present |
| `cwd` | Target working directory | Present in init |
| `tools` | Registered tool names | Present in init |
| `mcp_servers[].name/status` | Configured MCP servers and current status | Present in init, possibly empty |
| `model` | Configured active model | Present in init and assistant messages |
| `permission_mode` | Approval mode | Present in init |
| `slash_commands` | Available slash command names | Present in init |
| `qwen_code_version` | CLI version | Present in init, or `unknown` fallback |
| `agents` | Available subagent names | Present in init, possibly empty |

The official Headless examples still show `subtype: "session_start"`. Current source-backed `buildSystemMessage` emits `subtype: "init"`. Claudine should accept both for compatibility, but parser fixtures should use `init` for current Qwen.

Auth source is not emitted directly. The auth docs explain that `security.auth.selectedType`, model provider config, and provider environment variables choose OpenAI-compatible, Anthropic, Gemini, Vertex AI, Alibaba Cloud Coding Plan, or other providers. To detect auth kind, Claudine must combine configured inputs with generic error text; the stream itself does not provide a safe `auth_type` field.

## Event Families

The stream is transcript-oriented rather than a fully normalized lifecycle API. The main event families are:

| Family | Records | Notes |
| --- | --- | --- |
| Session | `system/init`, `system/worktree_started`, `system/worktree_restored`, `result` | Init arrives early enough to identify session/model/cwd/tools. |
| Assistant text/reasoning | `assistant`, `stream_event/content_block_delta` | Reasoning appears as `thinking` blocks/deltas when surfaced. |
| Tools | Assistant `tool_use` blocks, user `tool_result` blocks, `stream_event/tool_progress` | No dedicated file-change event; infer from tool names and inputs. |
| Permissions | `result.permission_denials`, `control_request/can_use_tool` | Plain stream has aggregate denials; control mode can request host decisions. |
| Usage | `assistant.message.usage`, `result.usage`, `result.modelUsage`, `result.stats` in JSON mode | Units are tokens; no cost field verified. |
| Subagents | `agent` tool call plus child messages with `parent_tool_use_id` | Inner tool calls are visible when subagent progress is projected. |
| Control plane | `control_request`, `control_response`, `control_cancel_request` | Only relevant for stream-json input/SDK-style use. |

`active_goal` deserves special mention because it is parser-significant but easy to miss. It is emitted as `stream_event` with `event.type: "active_goal"` only when partial stream events are enabled. That makes `--include-partial-messages` valuable even when Claudine does not need token-by-token text.

## Tools

Qwen's stream exposes tool calls as assistant content blocks:

```json
{"type":"assistant","message":{"content":[{"type":"tool_use","id":"...","name":"read_file","input":{"path":"src/lib.rs"}}],"stop_reason":"tool_use"}}
```

Tool results are user messages with `tool_result` blocks:

```json
{"type":"user","message":{"content":[{"type":"tool_result","tool_use_id":"...","content":"...","is_error":false}]}}
```

The result envelope also aggregates execution-denied tool calls:

```json
{"type":"result","permission_denials":[{"tool_name":"edit","tool_use_id":"...","tool_input":{"path":"..."}}]}
```

This is enough for Claudine to detect tool families, inputs, results, and denials. It is not enough to treat file changes as first-class events. Edits and writes must be inferred from `tool_use.name` (`edit`, `write_file`) and their inputs/results. Shell command stdout/stderr/exit status may be present inside `tool_result.content` or the tool's display payload, but I did not verify stable separate `stdout`, `stderr`, and `exit_code` fields in the CLI stream.

MCP progress is a special case. Tools that produce `McpToolProgressData` can emit `stream_event/tool_progress`, but only in stream-json mode with partial messages enabled.

Subagents are represented through the `agent` tool and child messages. The helper code emits the subagent prompt as a `user` message with `parent_tool_use_id` set to the parent agent call. It also emits child tool calls/results through the same adapter APIs, preserving the parent linkage. This is useful for Claudine reports, but budget semantics are a trap: the Headless docs state `--max-tool-calls` counts the top-level `agent` dispatch as one and does not count inner subagent tool calls.

## Completion and Exit Status

For normal success, trust the final `result` record:

```json
{"type":"result","subtype":"success","is_error":false,"result":"...","usage":{...}}
```

For structured-output success under `--json-schema`, prefer `result.structured_result` over `result.result`; the latter is stringified for consumers expecting a string. For ordinary text answers, `result.result` contains the final assistant text assembled from the last assistant message.

For structured failures, Qwen can emit:

```json
{"type":"result","subtype":"error_during_execution","is_error":true,"error":{"message":"..."}}
```

Source types also include `error_max_turns`, and docs distinguish exit codes: budget aborts are exit 55, max-session-turns is exit 53, and SIGINT is 130. The Structured Output docs warn that max-session-turns and signal interrupts can exit with stderr output only. Therefore Claudine should not rely solely on a terminal JSON event or solely on exit code. The robust rule is:

1. If a `result` record is present, use `is_error`, `subtype`, `error.message`, usage, and permission denials.
2. Always record process exit code.
3. If the process exits non-zero without a terminal result, classify from exit code and stderr.
4. If stdout JSONL ends without a `result` and exit code is zero, mark the state ambiguous and preserve raw logs.

## Blocking Behavior

Qwen has configurable approval modes: `plan`, `default`, `auto-edit`, `auto`, and `yolo`. The Approval Mode docs say default/ask requires manual approval for edits and shell commands; auto-edit approves edit tools but still prompts for shell; auto uses a classifier; yolo auto-approves everything.

In plain one-shot headless mode, there is no TUI available to answer a prompt. Source comments around teammate approvals say that in non-stream-json mode the only safe options are YOLO or cancel; otherwise Qwen auto-cancels teammate tool requests and writes a clear reason to stderr. SDK docs say default SDK behavior auto-denies write tools unless a `canUseTool` callback or allow rule approves them, and that custom permission handling has a timeout.

For Claudine this means:

| Situation | Expected automation behavior |
| --- | --- |
| Read-only/plan mode | Deterministic, but write/shell tools blocked |
| Default/ask in one-shot headless | Risk of denial/cancellation because no human prompt path exists |
| SDK/control stream with `can_use_tool` | Host can answer programmatically |
| YOLO without sandbox | Deterministic but unsafe; Qwen warns on stderr |
| YOLO with sandbox | Best fit for trusted autonomous mutation |

`ask_user_question` exists as a first-party tool, but I did not verify its exact one-shot headless behavior. Claudine should treat any `ask_user_question` tool call as a human-in-loop risk unless it is hosting the control protocol or has provider-specific evidence that Qwen auto-fails/auto-answers.

## Subagents

Subagents can run non-interactively. The init message includes an `agents` list, and subagent work is linked to the parent `agent` tool call via `parent_tool_use_id`. The non-interactive helper code projects subagent prompts, child tool calls, child tool results, and subagent error results through the same JSON adapters used for main-agent messages.

Visibility is good enough for Claudine to build a nested activity report:

```text
assistant tool_use id=A name=agent
  user parent_tool_use_id=A content=<subagent prompt>
  assistant parent_tool_use_id=A tool_use id=B name=read_file
  user parent_tool_use_id=A tool_result tool_use_id=B
```

The caveat is control and budgeting. Inner subagent tool calls are visible when projected, but they are not counted by the top-level `--max-tool-calls` budget according to the docs. Use `--max-subagent-depth 1` to prevent nesting and `--exclude-tools agent` when a strict top-level tool budget must also prevent delegated work.

## Use Case Detection

| Use case | Detectable? | Best signal | Notes |
| --- | --- | --- | --- |
| `plan_cap_approaching` | No | None verified | No near-cap quota/plan event found. |
| `plan_capped` | Partly | Exit 55, exit 53, `result.error.message`, stderr | Run budgets and turn caps are detectable; provider billing quota is generic. |
| `no_funds` | Weak | Generic error text | No stable no-funds event verified. |
| `auth` | Weak | Generic error text plus configured auth | Stream does not emit auth source/kind. |
| `permission_read_denied` | Yes | `result.permission_denials[]`, tool_result `is_error` | Path/policy usually recovered from tool input/content. |
| `permission_write_denied` | Yes | Same as read denial | No dedicated write-denied event. |
| `tokens_consumed` | Yes | `assistant.message.usage`, `result.usage`, `result.modelUsage` | `stats` requires buffered `json` in current source. |
| `model_used` | Yes | `system.model`, `assistant.message.model` | This is the configured model string. |
| `model_fallback` | Weak | Compare requested model to emitted model | No explicit fallback event verified. |
| `human_in_loop` | Yes | `ask_user_question` tool call, `control_request/can_use_tool`, stderr cancellation text | Exact one-shot question behavior remains a gap. |
| `session_resumable` | Yes | `session_id` in initial system record | Use with `--resume` if chat recording is enabled. |
| `subagent_prompt_injection` | Partly | `system.agents`, agent tool input, child prompt records | Top-level prompt can instruct subagents indirectly; direct per-subagent injection needs more evidence. |

Token units are tokens. `duration_ms` and `duration_api_ms` are milliseconds. I did not find timestamps in the CLI stream records, so Claudine should timestamp receipt time itself if it needs event timing.

## Headless Constraints

The strongest constraints for automation are:

- Always pass an output format. Config can influence output, but Claudine should not rely on persistent output settings because a repo/system/user setting can drift.
- Do not treat stderr as noise. It carries diagnostics and some lifecycle-only states.
- Do not use bare `--resume` in automation; it can open a picker. Use `--continue` or an explicit session ID.
- Do not use `--input-format stream-json` unless Claudine implements the control protocol.
- Do not combine `--json-schema` with stream-json input or ACP.
- Use explicit budgets. `--max-wall-time` and `--max-session-turns` are the minimum useful pair for unattended mutation.
- Treat `--max-tool-calls` as top-level only. Disable `agent` or cap subagent depth when necessary.
- Use sandboxing if auto-approving tools.

## Timeline

| Date | Event | Evidence |
| --- | --- | --- |
| 2025-10-11 | Community issue requested `json`/`stream-json` output for integration | [Qwen issue #795](https://github.com/QwenLM/qwen-code/issues/795) |
| 2026-02-09 | Session export to Markdown, JSONL, and HTML announced | [Weekly update](https://qwenlm.github.io/qwen-code-docs/en/blog/weekly-update-2026-02-09/) |
| 2026-03-13 | Hooks and proactive questions announced | [Weekly update](https://qwenlm.github.io/qwen-code-docs/en/blog/weekly-update-2026-03-13/) |
| 2026-04-15 | Qwen OAuth free tier discontinued | [Authentication docs](https://qwenlm.github.io/qwen-code-docs/en/users/configuration/auth/) |
| 2026-05-28 | Runtime budgets `--max-wall-time` and `--max-tool-calls` highlighted | [Weekly update](https://qwenlm.github.io/qwen-code-docs/en/blog/weekly-update-2026-05-28/) |
| 2026-07-02 | Headless docs list current output formats, budgets, retry behavior, and safety recommendations | [Headless docs](https://qwenlm.github.io/qwen-code-docs/en/users/features/headless/) |

## Quirks and Gaps

The biggest quirk is documentation/source drift. The docs examples show a `system` message with `subtype: "session_start"`, while current source's `buildSystemMessage` uses `subtype: "init"`. Claudine should accept both and avoid making the subtype the only session-start detector.

The second quirk is that `stream-json` looks complete but omits `stats` in current source. The source computes `stats = outputFormat === OutputFormat.JSON ? uiTelemetryService.getMetrics() : undefined`. For live parsing, use `usage` and `modelUsage`; for detailed post-run stats, run buffered `json`.

The third quirk is that the control stream shares the stdout transport. In stream-json input mode, stdout can carry ordinary data messages and control messages. That is a protocol, not a log. A wrapper that reads without answering permission/control requests can deadlock or cause auto-denials.

Verified gaps:

- No formal schema or version marker for stream records.
- No stable cost field.
- No timestamps in the stream union.
- No dedicated file-change event.
- No fully verified taxonomy for auth, rate-limit, quota, or no-funds failures.
- No live fixture for `ask_user_question` in plain one-shot headless mode.
- No verified dual-output framing for `--json-file`/`--json-fd`.

## Claudine Integration Notes

Recommended default:

```bash
qwen -p "$PROMPT" \
  --output-format stream-json \
  --include-partial-messages \
  --approval-mode plan \
  --max-session-turns 20 \
  --max-wall-time 10m
```

For trusted mutation inside a sandbox:

```bash
qwen -p "$PROMPT" \
  --output-format stream-json \
  --include-partial-messages \
  --approval-mode yolo \
  --sandbox \
  --max-session-turns 40 \
  --max-wall-time 20m
```

Parser rules:

- Parse stdout as JSONL.
- Discriminate first on `type`.
- For `stream_event`, discriminate on `event.type`.
- Join tool calls/results by `tool_use.id` and `tool_result.tool_use_id`.
- Join subagent activity by `parent_tool_use_id`.
- Treat `result` as terminal only when present.
- Preserve stderr and exit code as lifecycle evidence.
- Accept `system.subtype` of both `init` and `session_start`.
- Do not require `stats` in stream-json.

Wrapper rules:

- Supply output flags every run; do not trust config defaults.
- Avoid `--prompt-interactive`, bare `--resume`, and stream-json input unless explicitly implementing those modes.
- Set budgets for unattended runs.
- Decide permission mode explicitly from Claudine policy.
- Keep stderr attached to logs/reports even when stdout parsing succeeds.
- Mark no-result non-zero exits from codes 53, 55, and 130 using exit code plus stderr.

## Sources

- [Qwen Code Headless Mode](https://qwenlm.github.io/qwen-code-docs/en/users/features/headless/)
- [Qwen Code Configuration Settings](https://qwenlm.github.io/qwen-code-docs/en/users/configuration/settings/)
- [Qwen Code Structured Output](https://qwenlm.github.io/qwen-code-docs/en/users/features/structured-output/)
- [Qwen Code Authentication](https://qwenlm.github.io/qwen-code-docs/en/users/configuration/auth/)
- [Qwen Code Approval Mode](https://qwenlm.github.io/qwen-code-docs/en/users/features/approval-mode/)
- [Qwen Code TypeScript SDK](https://qwenlm.github.io/qwen-code-docs/en/developers/sdk-typescript/)
- [QwenLM/qwen-code `packages/cli/src/nonInteractive/types.ts`](https://github.com/QwenLM/qwen-code/blob/main/packages/cli/src/nonInteractive/types.ts)
- [QwenLM/qwen-code `packages/sdk-typescript/src/types/protocol.ts`](https://github.com/QwenLM/qwen-code/blob/main/packages/sdk-typescript/src/types/protocol.ts)
- [QwenLM/qwen-code `packages/cli/src/nonInteractive/io/BaseJsonOutputAdapter.ts`](https://github.com/QwenLM/qwen-code/blob/main/packages/cli/src/nonInteractive/io/BaseJsonOutputAdapter.ts)
- [QwenLM/qwen-code `packages/cli/src/nonInteractive/io/StreamJsonOutputAdapter.ts`](https://github.com/QwenLM/qwen-code/blob/main/packages/cli/src/nonInteractive/io/StreamJsonOutputAdapter.ts)
- [QwenLM/qwen-code `packages/cli/src/nonInteractiveCli.ts`](https://github.com/QwenLM/qwen-code/blob/main/packages/cli/src/nonInteractiveCli.ts)
- [QwenLM/qwen-code `packages/cli/src/utils/nonInteractiveHelpers.ts`](https://github.com/QwenLM/qwen-code/blob/main/packages/cli/src/utils/nonInteractiveHelpers.ts)
- [Qwen issue #795: output-format json/stream-json request](https://github.com/QwenLM/qwen-code/issues/795)
- [Qwen Code Weekly: Parallel Agent Panel, Auto-Memory On by Default, Worktree Phase D](https://qwenlm.github.io/qwen-code-docs/en/blog/weekly-update-2026-05-28/)
