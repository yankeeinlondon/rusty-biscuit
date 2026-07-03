---
$schema: ./_schema.yaml
created: 2026-07-02
last_updated: 2026-07-02
agent: codex
model: default
docs: https://kilo.ai/docs/code-with-ai/platforms/cli#autonomous-mode-non-interactive
invocation:
  - command: "kilo run --auto --format json \"<prompt>\""
    stdin_support: true
    prompt_arg: "Message words are joined into the prompt; non-TTY stdin is appended to the prompt."
    notes: "Fresh autonomous session unless --continue or --session is supplied. Preferred subprocess mode for Claudine."
  - command: "kilo run --auto --format json -- \"<prompt beginning with dash>\""
    stdin_support: true
    prompt_arg: "Words after -- are raw positional prompt atoms; useful for prompts beginning with '-' or shell-like text."
    notes: "Same session behavior as normal run."
  - command: "kilo run --auto --format json --file <path> \"<prompt>\""
    stdin_support: true
    prompt_arg: "Prompt from argv/stdin plus file attachments represented as file URL parts."
    notes: "Local files are resolved from --dir/current directory; attach mode resolves against the remote server directory."
  - command: "kilo run --auto --format json --continue \"<prompt>\""
    stdin_support: true
    prompt_arg: "Same prompt surfaces as normal run."
    notes: "Continues the last top-level session."
  - command: "kilo run --auto --format json --session <session-id> \"<prompt>\""
    stdin_support: true
    prompt_arg: "Same prompt surfaces as normal run."
    notes: "Continues a specific session ID."
  - command: "kilo run --auto --format json --session <session-id> --fork \"<prompt>\""
    stdin_support: true
    prompt_arg: "Same prompt surfaces as normal run."
    notes: "Forks an existing local session before continuing."
  - command: "kilo run --auto --format json --session <cloud-session-id> --cloud-fork \"<prompt>\""
    stdin_support: true
    prompt_arg: "Same prompt surfaces as normal run."
    notes: "Imports a cloud session and continues it locally."
  - command: "kilo serve --port <port>; kilo run --auto --format json --attach http://127.0.0.1:<port> \"<prompt>\""
    stdin_support: true
    prompt_arg: "Prompt is sent to the long-running server over the SDK HTTP API."
    notes: "Useful when Claudine wants a reusable server. Basic auth may be required through --username/--password or KILO_SERVER_*."
  - command: "kilo acp --cwd <dir>"
    stdin_support: false
    prompt_arg: "ACP client protocol, not plain prompt stdin."
    notes: "Starts an Agent Client Protocol server. This is a structured server mode, but it is a different integration surface from kilo run."
output_formats:
  - name: "formatted run output"
    cli_value: "default"
    stream: true
    format: text
    description: "Human-oriented formatted output. Final text is printed to stdout when stdout is not a TTY; TTY output includes formatted status and tool summaries."
    side_effects: "Not safe for parsing: banners, status text, ANSI styling, and logs may appear."
  - name: "raw JSON events"
    cli_value: "--format json"
    stream: true
    format: ndjson
    description: "One JSON object per stdout line from kilo run. The CLI emits selected event records: tool_use, step_start, step_finish, text, reasoning, and error."
    side_effects: "Suppresses the human formatter for forwarded records, but global startup/help logs can still be noisy outside normal run execution. No explicit terminal complete event is emitted."
  - name: "server SSE"
    cli_value: "kilo serve / SDK event.subscribe"
    stream: true
    format: sse
    description: "The local Kilo server exposes text/event-stream events. Data payloads are JSON with richer SDK/global event unions."
    side_effects: "Requires managing a server, optional Basic Auth, directory scoping, and generated SDK/API compatibility. Richer than run JSON but not the simplest subprocess wrapper."
  - name: "ACP server"
    cli_value: "kilo acp"
    stream: true
    format: other
    description: "Agent Client Protocol server mode."
    side_effects: "Protocol integration rather than a one-shot stdout stream; stdin/stdout prompt assumptions do not apply."
schema_sources:
  - url: "https://kilo.ai/docs/code-with-ai/platforms/cli"
    schema_type: examples
    formal: false
    notes: "Official CLI docs describe autonomous mode, run examples, --format json in the generated command reference, exit codes, config files, and env overrides. They do not define the JSON event schema."
  - url: "https://github.com/Kilo-Org/kilocode/blob/4c07a1db51d121b60129c3858f035da3f12df39c/packages/opencode/src/cli/cmd/run.ts"
    schema_type: typescript
    formal: false
    notes: "Authoritative source for kilo run flags, stdin merging, --format json emission, non-interactive permission replies, network retry behavior, and event names forwarded to stdout."
  - url: "https://github.com/Kilo-Org/kilocode/blob/4c07a1db51d121b60129c3858f035da3f12df39c/packages/opencode/test/cli/run/run-process.test.ts"
    schema_type: examples
    formal: false
    notes: "Subprocess tests lock in parseable line-delimited JSON, required type/sessionID fields, and the current mid-stream error exit-code behavior."
  - url: "https://github.com/Kilo-Org/kilocode/blob/4c07a1db51d121b60129c3858f035da3f12df39c/packages/opencode/src/session/message-v2.ts"
    schema_type: typescript
    formal: false
    notes: "Effect Schema source for Message, Part, ToolPart, StepFinishPart, tokens, cost, model, provider, cwd/root, and assistant error fields carried inside JSON part payloads."
  - url: "https://github.com/Kilo-Org/kilocode/blob/4c07a1db51d121b60129c3858f035da3f12df39c/packages/core/src/session-event.ts"
    schema_type: typescript
    formal: false
    notes: "Richer server-side EventV2 union for session.next.* events. Useful if Claudine later integrates through serve/SDK rather than kilo run."
  - url: "https://github.com/Kilo-Org/kilocode/blob/4c07a1db51d121b60129c3858f035da3f12df39c/packages/sdk/js/src/v2/gen/client/types.gen.ts"
    schema_type: typescript
    formal: false
    notes: "Generated TypeScript SDK types for the HTTP/SSE API. Broader than the CLI NDJSON stream."
  - url: "https://github.com/Kilo-Org/kilocode/blob/4c07a1db51d121b60129c3858f035da3f12df39c/packages/opencode/src/server/routes/instance/httpapi/handlers/event.ts"
    schema_type: typescript
    formal: false
    notes: "Server event endpoint encodes SSE message events with JSON.stringify(data)."
cli_params:
  - flag: "--format"
    value: "default | json"
    description: "Selects formatted text or raw JSON event output for kilo run."
    example: "kilo run --format json --auto \"fix tests\""
  - flag: "--auto"
    value: ""
    description: "Autonomous/pipeline mode. Auto-approves permissions for the root session and tracked task child sessions; questions and interactive terminal are denied."
    example: "kilo run --auto --format json \"task\""
  - flag: "--dangerously-skip-permissions"
    value: ""
    description: "Approves permission requests that are not explicitly denied. More dangerous and less deterministic than explicit permission config plus --auto."
    example: "kilo run --dangerously-skip-permissions --format json \"task\""
  - flag: "--interactive, -i"
    value: ""
    description: "Starts direct interactive split-footer mode. Conflicts with --format json and requires TTY stdout."
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
    description: "Runs a built-in/slash command with the message as arguments. Some built-ins require --continue or --session."
    example: "kilo run --command summarize --session ses_x --auto --format json"
  - flag: "--file, -f"
    value: "path"
    description: "Attaches one or more local files/directories to the prompt."
    example: "kilo run -f README.md --auto --format json \"summarize\""
  - flag: "--continue, -c"
    value: ""
    description: "Continues the last top-level session."
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
  - flag: "--share"
    value: ""
    description: "Shares the session if sharing is enabled."
    example: "kilo run --share --auto --format json \"task\""
  - flag: "--title"
    value: "title"
    description: "Sets the new session title; empty value derives a title from the prompt."
    example: "kilo run --title \"CI repair\" --auto --format json \"task\""
  - flag: "--dir"
    value: "path"
    description: "Runs in a local directory, or a remote server directory when --attach is used."
    example: "kilo run --dir packages/app --auto --format json \"task\""
  - flag: "--attach"
    value: "url"
    description: "Uses an existing Kilo server instead of starting an in-process server."
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
    description: "Prints logs to stderr according to CLI help; parser-relevant because logs can explain startup/auth failures."
    example: "kilo --print-logs --log-level INFO run --auto --format json \"task\""
  - flag: "--log-level"
    value: "DEBUG | INFO | WARN | ERROR"
    description: "Selects log verbosity."
    example: "kilo --print-logs --log-level DEBUG run --auto --format json \"task\""
  - flag: "--pure"
    value: ""
    description: "Runs without external plugins. Useful for reproducible automation."
    example: "kilo --pure run --auto --format json \"task\""
config_files:
  - os: all
    scope: user
    path: "~/.config/kilo/kilo.json"
    format: json
    effect: "Recommended global Kilo config; can set providers, model defaults, permission rules, MCP servers, plugins, skills, sharing, and server settings."
    notes: "Source order includes config.json, kilo.json, kilo.jsonc, opencode.json, opencode.jsonc. Later files merge over earlier files."
  - os: all
    scope: user
    path: "~/.config/kilo/kilo.jsonc"
    format: jsonc
    effect: "Recommended human-editable global config with comments."
    notes: "Documented as the global config for agents and many customization features."
  - os: all
    scope: user
    path: "~/.config/kilo/config.json"
    format: json
    effect: "Supported legacy/compat global config."
    notes: "Loaded before kilo.json/kilo.jsonc in current source."
  - os: all
    scope: user
    path: "~/.config/kilo/opencode.json | ~/.config/kilo/opencode.jsonc"
    format: jsonc
    effect: "Supported compatibility global config, including plugin config."
    notes: "Loaded after kilo config files; can affect tools, plugins, model/provider behavior, and output-side logs."
  - os: all
    scope: repo
    path: "./kilo.json | ./kilo.jsonc"
    format: jsonc
    effect: "Project config. Can override/merge permission, agent, MCP, plugin, skill, and provider settings for the current project."
    notes: "Project-level configuration takes precedence over global settings unless KILO_DISABLE_PROJECT_CONFIG is set."
  - os: all
    scope: repo
    path: "./.kilo/kilo.json | ./.kilo/kilo.jsonc"
    format: jsonc
    effect: "Project config under the recommended .kilo directory."
    notes: ".kilo directory resources load after root project config in the documented agent precedence."
  - os: all
    scope: repo
    path: "./.kilocode/kilo.json | ./.kilocode/opencode.json"
    format: jsonc
    effect: "Legacy project config directories and resources."
    notes: "Source walks both .kilocode and .kilo; .kilo generally wins when both exist."
  - os: all
    scope: user
    path: "~/.config/kilo/agents/*.md"
    format: other
    effect: "Global agent/subagent definitions, including permission and model defaults."
    notes: "Agent docs say global agent Markdown files merge after global/project config."
  - os: all
    scope: repo
    path: "./.kilo/agents/*.md"
    format: other
    effect: "Project agent/subagent definitions and overrides."
    notes: "Project agent files override same-name global/built-in agent fields by merge."
  - os: all
    scope: user
    path: "~/.config/kilo/tui.json | ~/.config/kilo/tui.jsonc"
    format: jsonc
    effect: "TUI and notification configuration."
    notes: "Mostly irrelevant to --format json, but can affect interactive/attention behavior."
  - os: all
    scope: repo
    path: "./.kilo/tui.json | ./.kilo/tui.jsonc"
    format: jsonc
    effect: "Project TUI/notification configuration."
    notes: "Not a structured stream selector."
  - os: all
    scope: user
    path: "~/.local/state/kilo/model.json"
    format: json
    effect: "Stores last selected model per agent."
    notes: "Docs say this remembered pick can override config-pinned model defaults until reset."
  - os: macos
    scope: other
    path: "~/Library/Application Support/Code/User/globalStorage/kilocode.kilo-code"
    format: other
    effect: "VS Code extension storage that can be migrated/read by CLI-related setup."
    notes: "Source documents this path for Kilo extension global storage."
  - os: windows
    scope: other
    path: "%APPDATA%/Code/User/globalStorage/kilocode.kilo-code"
    format: other
    effect: "VS Code extension storage that can be migrated/read by CLI-related setup."
    notes: "Source documents this path for Kilo extension global storage."
  - os: linux
    scope: other
    path: "~/.config/Code/User/globalStorage/kilocode.kilo-code"
    format: other
    effect: "VS Code extension storage that can be migrated/read by CLI-related setup."
    notes: "Source documents this path for Kilo extension global storage."
env_vars:
  - name: "KILO_CONFIG_CONTENT"
    effect: "Inline JSON/JSONC config override loaded after file config."
    notes: "Highest documented precedence; useful for deterministic CI wrapper config."
  - name: "KILO_CONFIG"
    effect: "Loads an explicit config file."
    notes: "Source loads it in addition to global config, before project directories and KILO_CONFIG_CONTENT."
  - name: "KILO_CONFIG_DIR"
    effect: "Adds/preferentially loads a config directory and AGENTS.md/instructions from that profile."
    notes: "Useful for isolated Claudine provider profiles."
  - name: "KILO_DISABLE_PROJECT_CONFIG"
    effect: "Disables project-local config/resource loading."
    notes: "Reduces repo-driven stream/tool surprises."
  - name: "KILO_PERMISSION"
    effect: "JSON permission override merged into resolved permission config."
    notes: "Can preconfigure allow/deny policy for non-interactive runs."
  - name: "KILO_PROVIDER"
    effect: "Official docs say it overrides active provider ID."
    notes: "Parser-relevant because it changes model/provider identity."
  - name: "KILO_<FIELD_NAME>"
    effect: "Official docs say non-kilocode provider fields can be overridden through env."
    notes: "Examples include KILO_API_KEY -> apiKey."
  - name: "KILOCODE_<FIELD_NAME>"
    effect: "Official docs say kilocode provider fields can be overridden through env."
    notes: "Example: KILOCODE_MODEL -> kilocodeModel."
  - name: "KILO_API_KEY"
    effect: "Kilo gateway/API auth and provider plugin fallback."
    notes: "Do not log raw value."
  - name: "KILO_ORG_ID"
    effect: "Selects organization for non-interactive kilo run; higher priority than persisted /teams selection."
    notes: "Official docs say there is no --org/--team flag for kilo run."
  - name: "KILO_AUTH_CONTENT"
    effect: "Process-local auth JSON override."
    notes: "Useful for isolated automation; do not log raw value."
  - name: "KILO_SERVER_PASSWORD"
    effect: "Basic auth password default for attach/server clients."
    notes: "Server auth is optional unless this is set."
  - name: "KILO_SERVER_USERNAME"
    effect: "Basic auth username default for attach/server clients."
    notes: "Defaults to kilo when unset."
  - name: "KILO_PURE"
    effect: "Skips external plugins."
    notes: "Same practical purpose as --pure for reproducible CI."
  - name: "KILO_DISABLE_DEFAULT_PLUGINS"
    effect: "Disables Kilo default plugins."
    notes: "Can change providers/auth/tools loaded."
  - name: "KILO_DIRECT_TRACE"
    effect: "Enables dev-only direct interactive JSONL trace under ~/.local/share/kilo/log/direct."
    notes: "Applies to direct interactive mode, not normal kilo run --format json."
  - name: "KILO_DISABLE_AUTOCOMPACT"
    effect: "Forces compaction.auto false."
    notes: "Can affect context behavior and token use."
  - name: "KILO_DISABLE_PRUNE"
    effect: "Forces compaction.prune false."
    notes: "Can affect context behavior and token use."
  - name: "KILO_EXPERIMENTAL_OUTPUT_TOKEN_MAX"
    effect: "Overrides default output token ceiling."
    notes: "Useful for cap/failure analysis."
  - name: "KILO_DISABLE_MODELS_FETCH"
    effect: "Disables startup model catalog fetch."
    notes: "Reduces network activity; may change model availability."
  - name: "KILO_MODELS_URL"
    effect: "Overrides model catalog source URL."
    notes: "Can alter available model metadata."
  - name: "KILO_MODELS_PATH"
    effect: "Loads model catalog from disk."
    notes: "Can alter model/provider availability."
  - name: "KILO_DB"
    effect: "Overrides database path; relative paths resolve under data directory; :memory: accepted."
    notes: "Useful for isolated test/CI runs."
  - name: "KILO_NO_DAEMON"
    effect: "Disables automatic daemon attach by clients."
    notes: "Documented in architecture docs; useful for deterministic subprocess ownership."
  - name: "KILO_REMOTE"
    effect: "Enables remote session relay behavior."
    notes: "Can add network/remote lifecycle behavior."
io_contract:
  stdout: structured_only
  stderr: mixed
  stdin: prompt
  framing: ndjson
  noise_handling: "For kilo run --format json, parse stdout line by line as JSON and treat non-JSON stdout as a wrapper error/noise condition. Keep stderr for diagnostics and startup/auth/log classification."
  notes: "Normal prompt stdin is one-shot text, not a bidirectional protocol. Help output and some startup logs observed locally can go to stdout, so Claudine should only assume parse-only stdout after launching the exact run command and seeing JSON records."
stream_contract:
  discriminator: "type"
  event_ordering: "Records are emitted in SDK event order for selected completed parts and errors. There is no session_start or terminal complete record."
  correlation_fields: ["sessionID", "part.id", "part.sessionID", "part.messageID", "part.callID", "part.tool"]
  terminal_event: ""
  partial_message_events: false
  unknown_event_policy: "Skip unknown type values after logging at trace; preserve raw record for drift analysis."
  notes: "Every JSON record has type, timestamp as Date.now() Unix milliseconds, and sessionID. text/reasoning/tool/step records carry a nested part object. The stream contains completed text/reasoning parts, completed/error tool parts, step-start/step-finish parts, and session.error records."
session_metadata:
  session_id: "Top-level sessionID field on every --format json record; emitted only once the first forwarded event occurs, not as a startup header."
  cwd: "Nested part.path.cwd/root only appears indirectly in assistant message schemas and server/SSE message.updated events; not emitted as a top-level kilo run JSON header."
  model: "step_finish.part.model.providerID/modelID when present; assistant message info in server/SSE includes modelID/providerID; requested --model is not echoed in a startup event."
  provider: "step_finish.part.model.providerID when present; tool/provider metadata can appear in nested tool state metadata."
  auth: "Not emitted in kilo run JSON. Auth failures surface as error records or stderr/human error text."
  version: "Not emitted in stream. Use kilo --version separately."
  mcp_servers: "Not emitted in kilo run JSON. Config/SDK may expose MCP server lists separately."
  permission_mode: "Inferred from invocation/config: --auto, --dangerously-skip-permissions, session permission rules, or KILO_PERMISSION. Permission requests are not emitted as JSON records."
  notes: "The preferred stream lacks an init envelope. Claudine must join static invocation/config facts with later part payloads."
stream_events:
  - event: "tool_use"
    category: tool_result
    fields: ["type", "timestamp", "sessionID", "part"]
    notes: "Emitted only for tool parts whose state.status is completed or error. part.callID joins call/result; part.state.input/output/error/metadata/time carry tool details."
  - event: "step_start"
    category: assistant
    fields: ["type", "timestamp", "sessionID", "part"]
    notes: "Emitted for part.type == step-start. Carries part.id/sessionID/messageID and optional snapshot."
  - event: "step_finish"
    category: usage
    fields: ["type", "timestamp", "sessionID", "part"]
    notes: "Emitted for part.type == step-finish. Carries reason, optional model.providerID/modelID, cost, and tokens."
  - event: "text"
    category: assistant
    fields: ["type", "timestamp", "sessionID", "part"]
    notes: "Emitted only for completed text parts with part.time.end. No partial deltas."
  - event: "reasoning"
    category: reasoning
    fields: ["type", "timestamp", "sessionID", "part"]
    notes: "Emitted only for completed reasoning parts with --thinking enabled."
  - event: "error"
    category: error
    fields: ["type", "timestamp", "sessionID", "error"]
    notes: "Emitted for session.error events or immediate SDK call errors. Mid-stream LLM errors currently can still exit 0."
  - event: "session.status"
    category: session
    fields: ["properties.sessionID", "properties.status.type"]
    notes: "Internal/SSE event used by run loop to break on idle; not forwarded as --format json."
  - event: "permission.asked"
    category: permission
    fields: ["properties.id", "properties.sessionID", "properties.permission", "properties.patterns", "properties.metadata", "properties.tool"]
    notes: "Internal/SSE event. In non-interactive run it is auto-replied once under --auto for allowed sessions, auto-rejected without --auto unless --dangerously-skip-permissions is set; not forwarded as JSON."
  - event: "question.asked"
    category: permission
    fields: ["properties.id", "properties.sessionID", "properties.questions"]
    notes: "Internal/SSE event. Non-interactive sessions create deny rules for question permission; not forwarded as JSON."
  - event: "session.network.asked"
    category: error
    fields: ["properties.id", "properties.sessionID"]
    notes: "Internal/SSE event. Non-interactive loop retries up to 3 times with exponential delay then rejects; not forwarded as JSON."
tools:
  - name: "bash/shell"
    call_visible: false
    result_visible: true
    metadata: ["part.callID", "part.tool", "part.state.status", "part.state.input", "part.state.output", "part.state.error", "part.state.time", "part.state.metadata"]
    notes: "CLI JSON emits tool_use only after completed or error tool state. Server/SSE also has session.next.shell.started/ended."
  - name: "read/glob/grep"
    call_visible: false
    result_visible: true
    metadata: ["part.callID", "part.tool", "part.state.input", "part.state.output", "part.state.error"]
    notes: "No separate file-read-denied JSON event; denied/failed access appears as tool error or session error."
  - name: "edit/write/apply_patch"
    call_visible: false
    result_visible: true
    metadata: ["part.callID", "part.tool", "part.state.input", "part.state.output", "part.state.metadata", "part.state.attachments"]
    notes: "No dedicated file_change event in kilo run JSON. Infer changed files from tool name, input, output, metadata, or external filesystem diff."
  - name: "task"
    call_visible: true
    result_visible: true
    metadata: ["part.callID", "part.tool", "part.state.status", "part.state.metadata.sessionId", "part.state.output"]
    notes: "Running task tool parts are used internally to track child sessions for --auto, but kilo run JSON only emits completed/error tool_use records. Child session ID can appear in metadata.sessionId."
  - name: "question"
    call_visible: false
    result_visible: false
    metadata: ["internal permission/question events only"]
    notes: "Non-interactive default rules deny question permission; no JSON event is emitted for the attempted human question."
  - name: "MCP tools"
    call_visible: false
    result_visible: true
    metadata: ["part.tool", "part.callID", "part.state.input", "part.state.output", "part.state.error", "part.state.metadata"]
    notes: "MCP tools use namespaced permission keys and appear as normal tool parts; MCP server lists are not in the run JSON envelope."
completion:
  success_event: ""
  failure_event: "error"
  exit_code_reliable: false
  result_fields: ["text.part.text", "step_finish.part.reason", "error.error"]
  cost_fields: ["step_finish.part.cost"]
  usage_fields: ["step_finish.part.tokens.input", "step_finish.part.tokens.output", "step_finish.part.tokens.reasoning", "step_finish.part.tokens.cache.read", "step_finish.part.tokens.cache.write", "step_finish.part.tokens.total"]
  notes: "Official docs list exit 0 success, 124 timeout, 1 initialization/execution failure. Source tests lock in that mid-stream LLM errors emit session.error but currently exit 0, so Claudine must parse error records and not rely only on exit code."
blocking_behavior:
  permissions: configurable
  questions: auto_deny
  tool_approvals: configurable
  notes: "Without --auto or --dangerously-skip-permissions, non-interactive permission requests for the root session are rejected. --auto replies once for root and tracked task child sessions. question, interactive_terminal, plan_enter, and plan_exit are denied in non-interactive sessions; network retry prompts are retried up to three times and then rejected."
subagents:
  supported: true
  start_visible: false
  stop_visible: false
  nested_events_visible: false
  prompt_injection_supported: false
  metadata_fields: ["tool_use.part.state.metadata.sessionId", "tool_use.part.state.output"]
  notes: "The task tool can create child sessions and --auto tracks task metadata.sessionId for permission replies. The CLI JSON stream does not forward nested child-session events except what is summarized in the parent task tool result. Server/SSE direct interactive transport has richer subagent state, but not the preferred run JSON stream."
use_cases:
  - name: "plan_cap_approaching"
    detectable: false
    event_types: []
    fields: []
    hook_parity: "unknown"
    notes: "No plan/quota approaching event was found in kilo run JSON."
  - name: "plan_capped"
    detectable: true
    event_types: ["error", "step_finish"]
    fields: ["error.error.name", "error.error.data.message", "step_finish.part.reason"]
    hook_parity: "unknown"
    notes: "Only detectable if provider emits an error/final reason. Reset windows or upgrade URLs are not structured in the CLI stream."
  - name: "no_funds"
    detectable: true
    event_types: ["error"]
    fields: ["error.error.name", "error.error.data.message"]
    hook_parity: "unknown"
    notes: "Detect by provider/Kilo billing error text or typed error name when present; no dedicated no_funds event."
  - name: "auth"
    detectable: true
    event_types: ["error"]
    fields: ["error.error.name", "error.error.data.message"]
    hook_parity: "unknown"
    notes: "Auth source is not emitted. Missing/expired auth must be classified from error payload or stderr."
  - name: "permission_read_denied"
    detectable: true
    event_types: ["tool_use", "error"]
    fields: ["part.tool", "part.state.status", "part.state.error", "part.state.input", "error.error"]
    hook_parity: "internal permission.asked/replied events exist in SSE but are not forwarded"
    notes: "No dedicated denial record in CLI JSON; distinguish read denial by tool name and error text."
  - name: "permission_write_denied"
    detectable: true
    event_types: ["tool_use", "error"]
    fields: ["part.tool", "part.state.status", "part.state.error", "part.state.input", "error.error"]
    hook_parity: "internal permission.asked/replied events exist in SSE but are not forwarded"
    notes: "No dedicated denial record in CLI JSON; distinguish edit/write denial by tool name and error text."
  - name: "tokens_consumed"
    detectable: true
    event_types: ["step_finish"]
    fields: ["part.tokens.total", "part.tokens.input", "part.tokens.output", "part.tokens.reasoning", "part.tokens.cache.read", "part.tokens.cache.write"]
    hook_parity: "server/SSE step events expose similar fields"
    notes: "Units are tokens. Granularity is per step_finish part, not explicitly session-total."
  - name: "model_used"
    detectable: true
    event_types: ["step_finish", "message.updated"]
    fields: ["part.model.providerID", "part.model.modelID", "properties.info.providerID", "properties.info.modelID"]
    hook_parity: "SSE/message.updated has richer metadata"
    notes: "CLI JSON only exposes model on step_finish when present; no early init model record."
  - name: "model_fallback"
    detectable: false
    event_types: []
    fields: []
    hook_parity: "unknown"
    notes: "No explicit fallback event found in kilo run JSON."
  - name: "human_in_loop"
    detectable: false
    event_types: []
    fields: []
    hook_parity: "internal question.asked/permission.asked/interactive_terminal events exist in SSE"
    notes: "The preferred run JSON does not expose attempted questions or permission prompts. Infer from denial/tool errors unless Claudine uses SSE."
  - name: "session_resumable"
    detectable: true
    event_types: ["tool_use", "step_start", "step_finish", "text", "reasoning", "error"]
    fields: ["sessionID"]
    hook_parity: "SSE has session IDs in most events"
    notes: "sessionID appears on every JSON record, but not before first forwarded event."
  - name: "subagent_prompt_injection"
    detectable: false
    event_types: []
    fields: []
    hook_parity: "unknown"
    notes: "No general subagent prompt injection surface found. Use configured agent prompts/permissions rather than runtime parent-stream injection."
headless_constraints:
  - constraint: "No terminal complete event in --format json."
    mitigation: "Treat process exit plus absence/presence of error records as completion; retain all final text records."
    notes: "The run loop breaks on internal session.status idle but does not emit that event."
  - constraint: "Mid-stream LLM errors currently can exit 0."
    mitigation: "Classify any error record as run failure or at least ambiguous even when exit code is 0."
    notes: "Locked in by subprocess test."
  - constraint: "Permission requests are not visible in CLI JSON."
    mitigation: "Use --auto with explicit permission config; classify permission failures from tool_use/error payloads."
    notes: "Use server/SSE if Claudine needs direct permission prompt telemetry."
  - constraint: "Tool call start/progress is mostly absent in CLI JSON."
    mitigation: "Render tools when completed/error tool_use arrives; use step_start as coarse progress."
    notes: "Server/SSE has richer session.next.tool.* events."
  - constraint: "Startup/help/log output may be human text."
    mitigation: "Launch only the exact run command for parsing and treat non-JSON stdout as noise/error."
    notes: "Local help output showed banner and INFO lines on stdout."
  - constraint: "Project config and plugins can change tools, providers, permissions, and stream-adjacent behavior."
    mitigation: "Use --pure, KILO_CONFIG_CONTENT, KILO_CONFIG_DIR, and/or KILO_DISABLE_PROJECT_CONFIG for deterministic wrapper profiles."
    notes: "Project config normally takes precedence over global config."
  - constraint: "MCP OAuth/auth commands are interactive surfaces."
    mitigation: "Pre-authenticate MCP servers or disable project MCP for CI; do not run mcp auth inside a non-interactive wrapper."
    notes: "MCP tools share normal permission system after configured."
quirks:
  - "The JSON format is selected with --format json, not --json."
  - "The JSON stream is sparse: no init, status, permission, question, partial delta, file_change, or complete event."
  - "Each JSON record includes timestamp from Date.now(), so it is Unix milliseconds in the local process clock."
  - "text and reasoning are emitted only for completed parts; reasoning additionally requires --thinking."
  - "tool_use is emitted only after tool completion/error, so live tool start/progress is invisible in the preferred stream."
  - "--auto does not mean every human interaction is answered; question and interactive_terminal are explicitly denied."
  - "--dangerously-skip-permissions can make automation proceed, but it is intentionally unsafe and still does not expose approval telemetry."
  - "The run command may automatically attach to a local daemon unless disabled; use KILO_NO_DAEMON when subprocess ownership matters."
  - "Kilo inherits substantial OpenCode internals; source paths and some docs/tests still use opencode names."
gaps:
  - "No official JSON schema or version marker for kilo run --format json was found."
  - "No captured real authenticated Kilo run was executed; findings rely on official docs, source, package help, and tests."
  - "Exact stderr/stdout split for all startup/auth failures was not exhaustively observed."
  - "Whether every provider populates step_finish.part.model consistently is unverified."
  - "Exact MCP tool payload metadata and OAuth failure payloads in --format json need fixtures."
  - "ACP protocol details were not researched beyond command availability."
  - "Server/SDK SSE schema is richer, but equivalence and stability relative to CLI JSON need a separate integration spike."
claudine_strategy:
  preferred_invocation: "kilo run --auto --format json --dir <cwd> --model <provider/model> \"<prompt>\""
  required_flags: ["run", "--auto", "--format json"]
  conflicting_flags: ["--interactive", "--replay", "--replay-limit"]
  parser_notes: "Parse stdout as NDJSON with discriminator type. Accept tool_use, step_start, step_finish, text, reasoning, and error; skip unknown records with trace logging. Treat non-JSON stdout as noise/error. Keep stderr for diagnostics. Use error records as failure evidence even with exit code 0."
  wrapper_notes: "Prefer explicit --model and isolated config through KILO_CONFIG_CONTENT or KILO_CONFIG_DIR. Use --pure and KILO_DISABLE_PROJECT_CONFIG when repository plugins/config must not affect automation. Add --thinking only if reasoning capture is desired. Do not use --dangerously-skip-permissions unless the caller explicitly accepts the risk."
data_format: ndjson
changes: []
requires_claudine_update: true
reason: "Kilo is not currently in Claudine's compiled provider enum, and its preferred stream uses a sparse NDJSON contract with no terminal event; supporting it needs provider metadata, a stream parser, and completion/error classification."
---

## Summary

Kilo Code can run non-interactively through `kilo run`. For Claudine, the best subprocess format is `kilo run --auto --format json`, which emits line-delimited JSON objects on stdout while the session is active. It is enough to render assistant text, completed tool results, step usage/cost, and session errors without scraping prose.

The main caveat is that Kilo's CLI JSON stream is intentionally sparse. It has no initialization event, no terminal completion event, no permission/question events, no partial assistant deltas, and no tool-start event for most tools. Claudine should parse the NDJSON stream for progress and final text, but it must use process exit plus parsed `error` records for completion classification. Source tests currently lock in that a mid-stream LLM error can emit an error event and still exit `0`, so exit code alone is not reliable.

## Non-Interactive Entry Points

The official CLI page documents autonomous mode as the mode for CI and other automation. The command shape is:

```sh
kilo run --auto "Implement feature X"
```

Current package help for `@kilocode/cli` 7.3.54 adds the parser-relevant flag:

```sh
kilo run --auto --format json "Implement feature X"
```

The prompt can come from positional argv, from raw args after `--`, and from non-TTY stdin. Source code joins positional `message` values into a prompt and appends piped stdin. Files can be attached with `--file`; each file becomes a `file://` part with a filename and MIME-like marker.

Session control is scriptable:

| Form | Behavior |
|---|---|
| `kilo run --auto --format json "<prompt>"` | Creates a fresh session unless a reusable daemon/session path is involved. |
| `kilo run --continue --auto --format json "<prompt>"` | Continues the latest top-level session. |
| `kilo run --session <id> --auto --format json "<prompt>"` | Continues a specific session. |
| `kilo run --session <id> --fork --auto --format json "<prompt>"` | Forks before continuing. |
| `kilo run --attach <url> --auto --format json "<prompt>"` | Talks to an existing `kilo serve` HTTP server. |

`kilo serve` is also a headless entry point, but it is a server integration rather than a one-shot subprocess output format. It exposes SDK/SSE surfaces that are richer than `kilo run --format json`, at the cost of lifecycle ownership, server auth, and directory scoping.

Kilo also exposes `kilo acp`, an Agent Client Protocol server. That is a plausible future Claudine integration for protocol-level control, but it is not the same as a plain stdout stream and was not the preferred path for this document.

## Output Formats

`kilo run` has two documented formats in current help: `default` and `json`.

| Format | Selector | Framing | Streams? | Claudine recommendation |
|---|---|---:|---:|---|
| Human formatted | default | text | yes | Avoid for wrappers; stdout can contain human text, status, and formatting. |
| Raw JSON events | `--format json` | NDJSON | yes | Prefer for subprocess wrapping. |
| Server events | `kilo serve` plus SDK `event.subscribe` | SSE | yes | Consider later when Claudine needs richer permissions/tool progress. |
| ACP | `kilo acp` | ACP protocol | yes | Separate protocol integration, not a one-shot run stream. |

The preferred stream is not a complete low-level event log. In `--format json`, the run loop emits JSON records only for selected events:

```ts
{
  type,
  timestamp: Date.now(),
  sessionID,
  ...data,
}
```

The concrete CLI event names are `tool_use`, `step_start`, `step_finish`, `text`, `reasoning`, and `error`. Each line is independently parseable JSON. `timestamp` is Unix milliseconds from the local process clock.

The server/SSE stream is richer. The server endpoint returns `text/event-stream`; each SSE `data` value is `JSON.stringify(data)`. It includes internal events such as `session.status`, `permission.asked`, `question.asked`, and the `session.next.*` event family. That richer stream is valuable, but it requires Claudine to manage or attach to a server and use the generated SDK/API contract. For a first Kilo provider, the simpler subprocess NDJSON stream is the right default.

## Schema Sources

Kilo does not publish a formal JSON Schema for `kilo run --format json`. The strongest evidence is source code and tests:

- `packages/opencode/src/cli/cmd/run.ts` defines the `--format` flag and the exact `emit()` envelope for CLI JSON records.
- `packages/opencode/test/cli/run/run-process.test.ts` asserts that `--format json` emits parseable line-delimited JSON and that every event has string `type` and `sessionID`.
- `packages/opencode/src/session/message-v2.ts` defines the nested `part` payload schemas for text, reasoning, tool, step-start, and step-finish parts using Effect Schema.
- `packages/core/src/session-event.ts` defines the richer `session.next.*` server event union. This is not identical to the CLI stream, but it explains fields that may appear in server/SSE mode or future CLI changes.
- `packages/sdk/js/src/v2/gen/client/types.gen.ts` contains generated TypeScript client types for the HTTP/SSE API. It is broader than the CLI NDJSON stream.

This means Claudine should treat the CLI stream schema as source-defined and version-sensitive. Unknown event names should be skipped and logged, not fatal, because the format is not formally versioned.

## IO Contract

For the exact command `kilo run --auto --format json`, stdout is intended to be NDJSON. Claudine should parse stdout line by line and keep the raw line for drift diagnostics. Any non-JSON stdout line during a parsed run should be treated as noise or a wrapper error.

stderr should not be discarded. Kilo has global `--print-logs` and `--log-level` controls, and startup/auth/provider failures may be clearer in stderr than in the JSON stream. Local package inspection also showed help output with banner and INFO lines on stdout, so the parse-only stdout assumption applies only to the exact run command once JSON records begin.

stdin is one-shot prompt text when it is not a TTY. It is not a bidirectional protocol in `kilo run`. ACP and SDK/server modes are separate protocol surfaces.

## Stream Contract

The top-level discriminator is `type`. The common envelope is:

```json
{"type":"text","timestamp":1760000000000,"sessionID":"ses_x","part":{}}
```

Important nested discriminators:

| Record | Nested discriminator | Correlation |
|---|---|---|
| `tool_use` | `part.type == "tool"` and `part.state.status` | `part.callID`, `part.id`, `part.messageID` |
| `step_start` | `part.type == "step-start"` | `part.id`, `part.messageID` |
| `step_finish` | `part.type == "step-finish"` | `part.id`, `part.messageID` |
| `text` | `part.type == "text"` | `part.id`, `part.messageID` |
| `reasoning` | `part.type == "reasoning"` | `part.id`, `part.messageID` |
| `error` | `error.name` or nested provider fields | top-level `sessionID` |

Assistant text and reasoning are complete snapshots, not deltas. `text` is emitted only when a text part has `time.end`; `reasoning` is emitted only when reasoning has ended and `--thinking` is enabled. Tool records are emitted only when the tool part is completed or errored.

There is no terminal event. Internally, the run loop breaks when it sees `session.status` become `idle`, but that record is not forwarded to `--format json`.

## Session Metadata

The stream exposes `sessionID` on every emitted JSON record. It does not emit a startup record, so Claudine does not learn the session ID until the first forwarded event. A run that fails before a forwarded event may have no session ID in stdout.

Model and provider are best extracted from `step_finish.part.model.providerID` and `step_finish.part.model.modelID` when present. The richer server message schema also carries `providerID`, `modelID`, `agent`, `path.cwd`, `path.root`, `cost`, and token usage on assistant messages. The CLI JSON stream does not emit that as an init envelope.

Auth source, CLI version, MCP server list, permission mode, sandbox mode, and project roots are not emitted in the preferred stream. Claudine should treat those as invocation/config facts or collect them through separate commands/config inspection when needed.

## Event Families

`tool_use` represents completed or failed tools. The nested `part` carries `tool`, `callID`, and `state`. Completed states include `input`, `output`, `title`, `metadata`, `time.start`, `time.end`, and optional `attachments`. Error states include `input`, `error`, optional `metadata`, and time fields.

`step_start` and `step_finish` bracket model steps at a coarse level. `step_finish` is the most important usage event because it carries `reason`, optional `model`, `cost`, and tokens:

```json
{
  "type": "step_finish",
  "part": {
    "type": "step-finish",
    "reason": "stop",
    "model": {"providerID": "openai", "modelID": "gpt-5"},
    "cost": 0.01,
    "tokens": {
      "input": 100,
      "output": 50,
      "reasoning": 0,
      "cache": {"read": 0, "write": 0}
    }
  }
}
```

`text` carries final assistant text part snapshots. Claudine should concatenate final text parts by stream order for a human answer, while preserving part IDs for reports.

`reasoning` carries completed reasoning only when `--thinking` is supplied. It should not be assumed present.

`error` carries session or immediate SDK errors. Source code normalizes some `session.error` payloads to human text for formatted mode, but JSON mode emits the raw `error` object.

## Tools

Kilo's built-in tool registry includes shell/bash, read, glob, grep, edit, write, task, fetch/search, todo, skill, patch, plan, and optional experimental tools. MCP tools are also available when configured and use normal permission keys.

For Claudine's preferred stream, the important behavior is visibility:

| Tool signal | `kilo run --format json` |
|---|---|
| Call start | Usually absent. |
| Input | Visible in `tool_use.part.state.input` after completion/error. |
| Progress | Usually absent. |
| Result | Visible in `tool_use.part.state.output` for completed tools. |
| Error | Visible in `tool_use.part.state.error` or top-level `error`. |
| File changes | No dedicated event; infer from edit/write/patch tool payloads or filesystem diff. |
| stdout/stderr | Usually embedded as tool output text, not separate streams. |

The `task` tool is special. In `--auto`, Kilo tracks task child sessions through `part.state.metadata.sessionId` so that permissions for child sessions can be auto-replied. The preferred JSON stream still does not forward full nested child-session event streams.

## Completion and Exit Status

Official docs list:

| Code | Meaning |
|---:|---|
| `0` | Success/task completed |
| `124` | Timeout |
| `1` | Initialization or execution failure |

Claudine should not rely on this table alone. Kilo's subprocess tests explicitly lock in the current behavior that a mid-stream LLM error emits a `session.error` event but exits `0`. Therefore:

- If the process exits non-zero, classify failure from exit code plus stderr/stdout context.
- If any `error` JSON record appears, classify the run as failure or ambiguous even if exit code is `0`.
- If there is no `error` record and exit code is `0`, treat the final answer as the ordered set of `text.part.text` records.
- Usage and cost are accumulated from `step_finish.part.tokens` and `step_finish.part.cost`.

There is no JSON completion record with final status, final answer, usage total, or cost total.

## Blocking Behavior

Autonomous mode is designed to avoid a human TTY mid-run. Official docs say approval requests are handled automatically based on configuration, follow-up questions receive an autonomous-decision instruction, and the CLI exits when the task completes or times out.

Current source behavior is more specific:

- Non-interactive sessions add deny rules for `question`, `interactive_terminal`, `plan_enter`, and `plan_exit`.
- Without `--auto`, permission requests for the root session are auto-rejected unless `--dangerously-skip-permissions` is set.
- With `--auto`, Kilo replies `once` to permissions for the root session and tracked `task` child sessions.
- `--dangerously-skip-permissions` approves permission requests that are not explicitly denied.
- `session.network.asked` is retried up to three times with exponential delay and then rejected.

Permission and question attempts are not forwarded in CLI JSON, so Claudine cannot directly render "awaiting permission" from the preferred stream. It should preconfigure permission policy and classify denials from tool/error payloads.

## Subagents

Kilo supports custom subagents and task child sessions. Project/global agent files can define prompts, models, and permissions. Subagents can run during non-interactive sessions through the task tool.

The preferred CLI JSON stream does not expose subagent start/stop as first-class events. Parent `task` tool results may include child-session metadata and summarized output. Source helper `KiloRunAuto` tracks `task` tool `metadata.sessionId` so that `--auto` can approve permissions for those child sessions, but nested tool calls and child status events are not forwarded to the parent JSON stream.

If Claudine needs rich nested subagent telemetry, it should investigate the server/SSE integration rather than the sparse subprocess stream.

## Use Case Detection

| Use case | Detectable from `--format json`? | How |
|---|---:|---|
| `tokens_consumed` | Yes | Sum `step_finish.part.tokens.*`; units are tokens, per step. |
| `model_used` | Partially | Use `step_finish.part.model.*` when present; otherwise infer from requested `--model` or server messages. |
| `session_resumable` | Yes, after first event | Use top-level `sessionID`. |
| `auth` | Partially | Classify `error.error.name` or `error.error.data.message`; auth source is absent. |
| `no_funds` | Partially | Classify billing/quota text or typed provider errors in `error`. |
| `permission_read_denied` | Partially | Tool/error name plus `part.state.error`; no dedicated permission event. |
| `permission_write_denied` | Partially | Tool/error name plus `part.state.error`; no dedicated permission event. |
| `human_in_loop` | No | Internal question/permission events are not emitted in CLI JSON. |
| `model_fallback` | No | No explicit fallback event found. |
| `plan_cap_approaching` | No | No plan/quota approach event found. |
| `plan_capped` | Partially | Only if final reason/error text exposes it. |
| `subagent_prompt_injection` | No | Use configured agent prompts rather than runtime stream injection. |

## Headless Constraints

The highest-risk automation constraint is the absence of a terminal event. Claudine must treat process exit as the stream terminator and use parsed records for semantic classification.

The second major constraint is sparse live visibility. Tool input and result are visible only after completion/error. Permission prompts, questions, and MCP OAuth surfaces are not visible in the preferred stream. This is enough for simple reporting, but not enough for a high-fidelity live operations dashboard.

Repository and user config can significantly alter behavior. Project configs, `.kilo` resources, plugins, MCP servers, and agent definitions can change available tools, permission behavior, and provider/model selection. For deterministic runs, Claudine should prefer an isolated Kilo profile using `KILO_CONFIG_CONTENT` or `KILO_CONFIG_DIR`, and consider `--pure`, `KILO_DISABLE_PROJECT_CONFIG`, and `KILO_NO_DAEMON` depending on the desired trust boundary.

## Timeline

- Kilo CLI 1.0+ docs describe autonomous mode (`kilo run --auto`) for CI/pipeline use.
- Current npm package inspection on 2026-07-02 found `@kilocode/cli` 7.3.54 and help for `kilo run --format default|json`.
- Current source at commit `4c07a1db51d121b60129c3858f035da3f12df39c` defines the sparse JSON stream and its non-interactive permission behavior.

## Quirks and Gaps

Kilo's source still lives largely under `packages/opencode`, and several tests/comments refer to `opencode run`. That is expected for this fork, but future maintainers should cite Kilo-specific wrappers and docs when possible.

The generated SDK and server schemas are richer than the CLI stream. They are useful context, not proof that `kilo run --format json` exposes those fields. A future Claudine server integration should separately test the SSE stream, Basic Auth, session directory selection, and event ordering.

Unverified gaps remain around real provider auth failures, MCP OAuth failures, exact provider quota payloads, and model fallback behavior. No authenticated live Kilo task was run for this research.

## Claudine Integration Notes

Recommended default:

```sh
kilo run --auto --format json --dir "$PWD" --model "<provider/model>" "<prompt>"
```

Use `--thinking` only when Claudine wants completed reasoning blocks. Avoid `--interactive`, `--replay`, and `--replay-limit`; the source rejects those combinations outside interactive mode or with JSON format.

For deterministic wrapper behavior, launch with an explicit provider profile:

```sh
KILO_CONFIG_CONTENT='{"permission":{"*":"allow","question":"deny","interactive_terminal":"deny"}}' \
KILO_NO_DAEMON=1 \
kilo --pure run --auto --format json --model "<provider/model>" "<prompt>"
```

The parser should:

- Parse stdout as NDJSON and dispatch by top-level `type`.
- Preserve unknown records for drift reports.
- Treat `error` records as semantic failure evidence even if the process exits `0`.
- Sum usage from `step_finish.part.tokens`.
- Infer tool/file changes from `tool_use.part`, not from a dedicated file-change event.
- Use stderr for diagnostics, especially when the JSON stream never starts.

The stream is good enough for a first Claudine provider, but a high-fidelity Kilo integration should eventually investigate `kilo serve` plus SDK/SSE because that surface exposes internal status, permission, question, and `session.next.*` events that the CLI NDJSON stream drops.

## Sources

- [Kilo Code CLI docs](https://kilo.ai/docs/code-with-ai/platforms/cli)
- [CLI Command Reference](https://github.com/Kilo-Org/kilocode/blob/main/packages/kilo-docs/pages/code-with-ai/platforms/cli-reference.md)
- [Using MCP in the CLI](https://github.com/Kilo-Org/kilocode/blob/main/packages/kilo-docs/pages/automate/mcp/using-in-cli.md)
- [Custom Modes configuration precedence](https://github.com/Kilo-Org/kilocode/blob/main/packages/kilo-docs/pages/customize/custom-modes.md)
- [Custom Subagents precedence](https://github.com/Kilo-Org/kilocode/blob/main/packages/kilo-docs/pages/customize/custom-subagents.md)
- [Plugin load order](https://github.com/Kilo-Org/kilocode/blob/main/packages/kilo-docs/pages/automate/extending/plugins.md)
- [`packages/opencode/src/cli/cmd/run.ts`](https://github.com/Kilo-Org/kilocode/blob/4c07a1db51d121b60129c3858f035da3f12df39c/packages/opencode/src/cli/cmd/run.ts)
- [`packages/opencode/test/cli/run/run-process.test.ts`](https://github.com/Kilo-Org/kilocode/blob/4c07a1db51d121b60129c3858f035da3f12df39c/packages/opencode/test/cli/run/run-process.test.ts)
- [`packages/opencode/src/session/message-v2.ts`](https://github.com/Kilo-Org/kilocode/blob/4c07a1db51d121b60129c3858f035da3f12df39c/packages/opencode/src/session/message-v2.ts)
- [`packages/core/src/session-event.ts`](https://github.com/Kilo-Org/kilocode/blob/4c07a1db51d121b60129c3858f035da3f12df39c/packages/core/src/session-event.ts)
- [`packages/opencode/src/server/routes/instance/httpapi/handlers/event.ts`](https://github.com/Kilo-Org/kilocode/blob/4c07a1db51d121b60129c3858f035da3f12df39c/packages/opencode/src/server/routes/instance/httpapi/handlers/event.ts)
- [`packages/opencode/src/kilocode/cli/run-auto.ts`](https://github.com/Kilo-Org/kilocode/blob/4c07a1db51d121b60129c3858f035da3f12df39c/packages/opencode/src/kilocode/cli/run-auto.ts)
- Local package inspection: `@kilocode/cli` 7.3.54 and `@kilocode/cli-darwin-arm64` 7.3.54 help output on 2026-07-02.
