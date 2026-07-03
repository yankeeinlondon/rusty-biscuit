---
$schema: ./_schema.yaml
created: 2026-07-02
last_updated: 2026-07-03
agent: codex
model: default
docs: https://opencode.ai/docs/cli/
invocation:
  - command: "opencode run --format json --print-logs --log-level INFO -- \"<prompt>\""
    stdin_support: true
    prompt_arg: "Positional `[message..]`; non-TTY stdin is appended to the positional prompt text."
    notes: "Fresh non-interactive local session unless `--continue` or `--session` is supplied. This is Claudine's preferred subprocess entry point."
  - command: "opencode run --format json --print-logs --log-level INFO --continue -- \"<prompt>\""
    stdin_support: true
    prompt_arg: "Positional `[message..]` plus appended stdin."
    notes: "Continues the most recent parent session; `--fork` can fork before continuing."
  - command: "opencode run --format json --print-logs --log-level INFO --session <session-id> -- \"<prompt>\""
    stdin_support: true
    prompt_arg: "Positional `[message..]` plus appended stdin."
    notes: "Resumes a named session; exits 1 with `Session not found` if the ID cannot be loaded."
  - command: "opencode run --attach http://host:4096 --format json --dir <remote-path> -- \"<prompt>\""
    stdin_support: true
    prompt_arg: "Positional `[message..]` plus appended stdin."
    notes: "Talks to a long-running `opencode serve` backend; `--password`/`--username` or server env vars provide basic auth."
  - command: "opencode serve --port 4096 --hostname 127.0.0.1"
    stdin_support: false
    prompt_arg: "HTTP API/SDK requests such as `POST /session/:id/message`; not argv prompt text."
    notes: "Starts a long-running headless HTTP server with OpenAPI and SSE event endpoints."
  - command: "opencode acp --cwd <path>"
    stdin_support: true
    prompt_arg: "ACP client requests over newline-delimited JSON on stdin/stdout."
    notes: "Starts an Agent Client Protocol server; richer protocol surface, but Claudine's OpenCode wrapper does not currently use it."
output_formats:
  - name: "run default"
    cli_value: "default"
    stream: true
    format: text
    description: "Human-formatted stdout/stderr for `opencode run`."
    side_effects: "Text, banners, warnings, and status lines are not a stable parser contract."
  - name: "run json"
    cli_value: "json"
    stream: true
    format: ndjson
    description: "One JSON object per stdout line from `opencode run`; top-level `type` is a filtered CLI envelope over selected session events."
    side_effects: "Suppresses most human stdout, but omits user prompt events, permission events, tool start events, and a terminal completion event."
  - name: "run json plus printed logs"
    cli_value: "json + --print-logs --log-level INFO"
    stream: true
    format: other
    description: "Preferred dual stream: stdout NDJSON plus structured stderr lifecycle logs."
    side_effects: "stderr is no longer ignorable; it carries model/provider, session/subagent, permission, HTTP, retry, cap, and auth signals."
  - name: "server events"
    cli_value: "opencode serve; GET /event or /global/event"
    stream: true
    format: sse
    description: "Server-sent events. `/event` starts with `server.connected` and then bus events; `/global/event` exposes global events."
    side_effects: "Requires a long-running server and HTTP/SSE client instead of simple subprocess parsing."
  - name: "server OpenAPI responses"
    cli_value: "opencode serve; HTTP routes"
    stream: false
    format: json
    description: "Request/reply JSON for sessions, messages, config, tools, files, and auth."
    side_effects: "Better schema coverage than CLI NDJSON, but not a live progress stream unless combined with SSE."
  - name: "ACP"
    cli_value: "opencode acp"
    stream: true
    format: ndjson
    description: "Agent Client Protocol over newline-delimited JSON on stdin/stdout."
    side_effects: "Bidirectional protocol; stdout is protocol traffic, not a one-way run event log."
schema_sources:
  - url: "https://opencode.ai/docs/cli/"
    schema_type: examples
    formal: false
    notes: "Official command and flag docs for `opencode run`; documents `--format json` but not the exact line schema."
  - url: "https://github.com/anomalyco/opencode/blob/dev/packages/opencode/src/cli/cmd/run.ts"
    schema_type: typescript
    formal: false
    notes: "Implementation source for the exact `run --format json` stdout envelope and permission auto-reply behavior."
  - url: "https://opencode.ai/docs/server/"
    schema_type: openapi
    formal: true
    notes: "Official server docs; the server exposes OpenAPI 3.1 at `/doc` and SSE at `/event` / `/global/event`."
  - url: "https://raw.githubusercontent.com/anomalyco/opencode/dev/packages/sdk/openapi.json"
    schema_type: openapi
    formal: true
    notes: "Generated OpenAPI spec for server routes and underlying session/message/event models; broader than CLI stdout NDJSON."
  - url: "https://github.com/anomalyco/opencode/blob/dev/packages/sdk/js/src/v2/gen/types.gen.ts"
    schema_type: typescript
    formal: true
    notes: "Generated SDK TypeScript types for `Part`, `ToolPart`, `StepFinishPart`, raw bus events, permissions, sessions, tokens, and cost."
  - url: "https://github.com/anomalyco/opencode/blob/dev/packages/opencode/src/session/message-v2.ts"
    schema_type: typescript
    formal: false
    notes: "Provider source for session/message hydration and runtime event constants; useful for semantics behind generated types."
  - url: "https://github.com/anomalyco/opencode/blob/dev/packages/opencode/src/cli/cmd/acp.ts"
    schema_type: other
    formal: true
    notes: "ACP command implementation delegates to the Agent Client Protocol SDK's NDJSON stream."
cli_params:
  - flag: "run"
    value: "[message..]"
    description: "Runs one non-interactive prompt unless `--mini`/interactive mode is selected."
    example: "opencode run -- \"fix the tests\""
  - flag: "--format"
    value: "default | json"
    description: "Selects human output or the CLI NDJSON stream."
    example: "opencode run --format json -- \"summarize\""
  - flag: "--print-logs"
    value: ""
    description: "Prints OpenCode logs to stderr instead of only the log file; parser-relevant for lifecycle visibility."
    example: "opencode run --format json --print-logs --log-level INFO -- \"task\""
  - flag: "--log-level"
    value: "DEBUG | INFO | WARN | ERROR"
    description: "Controls printed log verbosity; INFO is the minimum useful lifecycle level for Claudine."
    example: "opencode run --format json --print-logs --log-level INFO -- \"task\""
  - flag: "--model, -m"
    value: "provider/model"
    description: "Requested provider/model ID for the run."
    example: "opencode run -m opencode/gpt-5 -- \"task\""
  - flag: "--variant"
    value: "STRING"
    description: "Provider-specific model variant such as reasoning effort."
    example: "opencode run --variant high -- \"task\""
  - flag: "--agent"
    value: "NAME"
    description: "Selects a primary agent; subagent names are rejected for direct run selection and fall back to the default."
    example: "opencode run --agent build -- \"task\""
  - flag: "--file, -f"
    value: "PATH"
    description: "Attaches one or more local files/directories to the prompt; attach-to-server mode uploads local files up to 10 MiB and refuses local directories."
    example: "opencode run -f screenshot.png -- \"inspect this\""
  - flag: "--dir"
    value: "PATH"
    description: "Sets local working directory, or the remote directory when using `--attach`."
    example: "opencode run --dir /repo -- \"task\""
  - flag: "--continue, -c"
    value: ""
    description: "Continues the most recent parent session."
    example: "opencode run --continue -- \"next step\""
  - flag: "--session, -s"
    value: "SESSION_ID"
    description: "Resumes an existing session by ID."
    example: "opencode run --session ses_abc -- \"continue\""
  - flag: "--fork"
    value: ""
    description: "Forks before continuing or resuming; requires `--continue` or `--session`."
    example: "opencode run --session ses_abc --fork -- \"try another approach\""
  - flag: "--command"
    value: "COMMAND"
    description: "Runs a slash/custom command with message text as arguments."
    example: "opencode run --format json --command test -- \"unit tests\""
  - flag: "--title"
    value: "TITLE"
    description: "Sets the session title; empty string uses a truncated prompt."
    example: "opencode run --title \"CI repair\" -- \"task\""
  - flag: "--share"
    value: ""
    description: "Requests session sharing in addition to config/env auto-share behavior."
    example: "opencode run --share -- \"task\""
  - flag: "--auto"
    value: ""
    description: "Auto-approves permission requests that are not explicitly denied."
    example: "opencode run --auto --format json -- \"task\""
  - flag: "--dangerously-skip-permissions"
    value: ""
    description: "Hidden alias accepted by current source; equivalent to auto approval for run-loop permission replies."
    example: "opencode run --dangerously-skip-permissions --format json -- \"task\""
  - flag: "--attach"
    value: "URL"
    description: "Uses a running OpenCode server instead of the in-process server."
    example: "opencode run --attach http://localhost:4096 --format json -- \"task\""
  - flag: "--password, -p"
    value: "PASSWORD"
    description: "Basic auth password for `--attach`; default can come from `OPENCODE_SERVER_PASSWORD`."
    example: "opencode run --attach http://localhost:4096 --password \"$OPENCODE_SERVER_PASSWORD\" -- \"task\""
  - flag: "--username, -u"
    value: "USERNAME"
    description: "Basic auth username for `--attach`; default is `OPENCODE_SERVER_USERNAME` or `opencode`."
    example: "opencode run --attach http://localhost:4096 --username opencode -- \"task\""
  - flag: "serve"
    value: "--port --hostname --cors"
    description: "Starts the headless HTTP/OpenAPI/SSE server."
    example: "OPENCODE_SERVER_PASSWORD=secret opencode serve --port 4096"
  - flag: "acp"
    value: "--cwd --port --hostname"
    description: "Starts an ACP NDJSON protocol server over stdin/stdout after launching an internal server."
    example: "opencode acp --cwd /repo"
config_files:
  - os: macos
    scope: user
    path: "~/.config/opencode/opencode.json"
    format: json
    effect: "Global model, provider, permission, agent, shell, formatter, LSP, MCP, plugin, instruction, autoupdate, and experimental settings."
    notes: "Loaded after remote config and before `OPENCODE_CONFIG`; merged with later layers."
  - os: linux
    scope: user
    path: "~/.config/opencode/opencode.json"
    format: json
    effect: "Global model, provider, permission, agent, shell, formatter, LSP, MCP, plugin, instruction, autoupdate, and experimental settings."
    notes: "Loaded after remote config and before `OPENCODE_CONFIG`; merged with later layers."
  - os: windows
    scope: user
    path: "%USERPROFILE%\\.config\\opencode\\opencode.json"
    format: json
    effect: "Global model, provider, permission, agent, shell, formatter, LSP, MCP, plugin, instruction, autoupdate, and experimental settings."
    notes: "Source uses XDG config resolution; Windows paths differ from POSIX paths."
  - os: macos
    scope: user
    path: "~/.config/opencode/opencode.jsonc"
    format: json
    effect: "Same as the global JSON config, with comments."
    notes: "JSONC file; schema records it as json because the research schema has no jsonc enum."
  - os: linux
    scope: user
    path: "~/.config/opencode/opencode.jsonc"
    format: json
    effect: "Same as the global JSON config, with comments."
    notes: "JSONC file; schema records it as json because the research schema has no jsonc enum."
  - os: windows
    scope: user
    path: "%USERPROFILE%\\.config\\opencode\\opencode.jsonc"
    format: json
    effect: "Same as the global JSON config, with comments."
    notes: "JSONC file; Windows path is the XDG-style config path used by OpenCode."
  - os: macos
    scope: repo
    path: "opencode.json"
    format: json
    effect: "Project-local model, provider, permission, agent, MCP, plugin, command, instruction, and tool settings."
    notes: "OpenCode searches from cwd upward to the nearest Git directory; project config overrides global/custom config on conflicting keys."
  - os: linux
    scope: repo
    path: "opencode.json"
    format: json
    effect: "Project-local model, provider, permission, agent, MCP, plugin, command, instruction, and tool settings."
    notes: "OpenCode searches from cwd upward to the nearest Git directory; project config overrides global/custom config on conflicting keys."
  - os: windows
    scope: repo
    path: "opencode.json"
    format: json
    effect: "Project-local model, provider, permission, agent, MCP, plugin, command, instruction, and tool settings."
    notes: "Same repo-relative file name; path separators differ when resolved by the host filesystem."
  - os: macos
    scope: repo
    path: "opencode.jsonc"
    format: json
    effect: "Project-local JSONC equivalent."
    notes: "JSONC file; merged, not replaced. Later sources override only conflicting keys."
  - os: linux
    scope: repo
    path: "opencode.jsonc"
    format: json
    effect: "Project-local JSONC equivalent."
    notes: "JSONC file; merged, not replaced. Later sources override only conflicting keys."
  - os: windows
    scope: repo
    path: "opencode.jsonc"
    format: json
    effect: "Project-local JSONC equivalent."
    notes: "JSONC file; same repo-relative file name with Windows path resolution."
  - os: macos
    scope: repo
    path: ".opencode/"
    format: other
    effect: "Project resource directory for agents, commands, plugins, skills, tools, themes, and config-side assets."
    notes: "Loaded after project config; plural subdirectories are preferred, singular names remain supported."
  - os: linux
    scope: repo
    path: ".opencode/"
    format: other
    effect: "Project resource directory for agents, commands, plugins, skills, tools, themes, and config-side assets."
    notes: "Loaded after project config; plural subdirectories are preferred, singular names remain supported."
  - os: windows
    scope: repo
    path: ".opencode\\"
    format: other
    effect: "Project resource directory for agents, commands, plugins, skills, tools, themes, and config-side assets."
    notes: "Loaded after project config; plural subdirectories are preferred, singular names remain supported."
  - os: macos
    scope: user
    path: "~/.config/opencode/{agents,commands,plugins,skills,tools,themes}/"
    format: other
    effect: "User resource directories that can add agents/subagents, slash commands, plugins/hooks, skills, tools, and themes."
    notes: "Can affect tool availability, permissions, subagent behavior, and emitted tool names."
  - os: linux
    scope: user
    path: "~/.config/opencode/{agents,commands,plugins,skills,tools,themes}/"
    format: other
    effect: "User resource directories that can add agents/subagents, slash commands, plugins/hooks, skills, tools, and themes."
    notes: "Can affect tool availability, permissions, subagent behavior, and emitted tool names."
  - os: windows
    scope: user
    path: "%USERPROFILE%\\.config\\opencode\\{agents,commands,plugins,skills,tools,themes}\\"
    format: other
    effect: "User resource directories that can add agents/subagents, slash commands, plugins/hooks, skills, tools, and themes."
    notes: "Can affect tool availability, permissions, subagent behavior, and emitted tool names."
  - os: macos
    scope: other
    path: "$OPENCODE_CONFIG"
    format: json
    effect: "Custom config path."
    notes: "Loaded after global config and before project config."
  - os: linux
    scope: other
    path: "$OPENCODE_CONFIG"
    format: json
    effect: "Custom config path."
    notes: "Loaded after global config and before project config."
  - os: windows
    scope: other
    path: "%OPENCODE_CONFIG%"
    format: json
    effect: "Custom config path."
    notes: "Loaded after global config and before project config."
  - os: macos
    scope: other
    path: "$OPENCODE_CONFIG_DIR"
    format: other
    effect: "Custom config directory for user resources."
    notes: "Can redirect resource discovery and plugin loading."
  - os: linux
    scope: other
    path: "$OPENCODE_CONFIG_DIR"
    format: other
    effect: "Custom config directory for user resources."
    notes: "Can redirect resource discovery and plugin loading."
  - os: windows
    scope: other
    path: "%OPENCODE_CONFIG_DIR%"
    format: other
    effect: "Custom config directory for user resources."
    notes: "Can redirect resource discovery and plugin loading."
  - os: macos
    scope: other
    path: "$OPENCODE_CONFIG_CONTENT"
    format: json
    effect: "Inline runtime config override."
    notes: "Loaded after `.opencode` directories and before managed settings; useful for wrapper-injected MCP/instructions."
  - os: linux
    scope: other
    path: "$OPENCODE_CONFIG_CONTENT"
    format: json
    effect: "Inline runtime config override."
    notes: "Loaded after `.opencode` directories and before managed settings; useful for wrapper-injected MCP/instructions."
  - os: windows
    scope: other
    path: "%OPENCODE_CONFIG_CONTENT%"
    format: json
    effect: "Inline runtime config override."
    notes: "Loaded after `.opencode` directories and before managed settings; useful for wrapper-injected MCP/instructions."
  - os: macos
    scope: managed
    path: "/Library/Application Support/opencode/"
    format: json
    effect: "Admin-managed settings."
    notes: "Docs list this after inline config; managed files override ordinary user/repo/runtime settings."
  - os: macos
    scope: managed
    path: "MDM .mobileconfig managed preferences"
    format: other
    effect: "Highest-priority non-user-overridable managed preferences."
    notes: "Can override model, permissions, tools, or provider settings in enterprise deployments."
env_vars:
  - name: "OPENCODE_CONFIG"
    effect: "Loads a custom config file layer."
    notes: "Parser-relevant when it changes model, permissions, tools, MCP, plugins, or agents."
  - name: "OPENCODE_CONFIG_DIR"
    effect: "Changes the config/resource directory."
    notes: "Can change discovered plugins, agents, commands, skills, tools, and themes."
  - name: "OPENCODE_CONFIG_CONTENT"
    effect: "Inline JSON config layer with high precedence."
    notes: "Claudine uses this surface for runtime injection such as MCP defaults or instruction files."
  - name: "OPENCODE_PERMISSION"
    effect: "Inline JSON permissions config."
    notes: "Listed by the CLI docs; can alter allow/ask/deny behavior in non-interactive runs."
  - name: "OPENCODE_SERVER_PASSWORD"
    effect: "Enables basic auth for `serve`, `web`, and `run --attach` clients."
    notes: "Missing password leaves `serve` unsecured and prints a warning."
  - name: "OPENCODE_SERVER_USERNAME"
    effect: "Overrides the basic auth username for server/attach mode."
    notes: "Default username is `opencode`."
  - name: "OPENCODE_PRINT_LOGS"
    effect: "Set internally by `--print-logs`; routes logs to stderr."
    notes: "Wrapper should prefer the CLI flag over setting this directly."
  - name: "OPENCODE_LOG_LEVEL"
    effect: "Set internally by `--log-level`; controls printed log severity."
    notes: "Wrapper should prefer `--log-level INFO`."
  - name: "OPENCODE_PURE"
    effect: "Set by `--pure`; disables external plugins."
    notes: "Can make stream shape more deterministic by removing plugin side effects."
  - name: "OPENCODE_DISABLE_DEFAULT_PLUGINS"
    effect: "Disables default plugins."
    notes: "Can change hook/tool behavior and side-effect noise."
  - name: "OPENCODE_DISABLE_CLAUDE_CODE"
    effect: "Disables reading Claude Code prompt and skills compatibility surfaces."
    notes: "Can alter effective instructions/skills."
  - name: "OPENCODE_DISABLE_CLAUDE_CODE_PROMPT"
    effect: "Disables reading Claude Code prompt files."
    notes: "Affects behavior, not framing."
  - name: "OPENCODE_DISABLE_CLAUDE_CODE_SKILLS"
    effect: "Disables loading `.claude/skills`."
    notes: "Affects skills and tool/resource availability."
  - name: "OPENCODE_DISABLE_MODELS_FETCH"
    effect: "Disables fetching models from remote sources."
    notes: "Can change model availability and fallback errors."
  - name: "OPENCODE_ENABLE_EXPERIMENTAL_MODELS"
    effect: "Enables experimental models."
    notes: "Can change selectable models."
  - name: "OPENCODE_ENABLE_EXA"
    effect: "Enables Exa web search tools."
    notes: "Can add tool families visible as completed `tool_use` records."
  - name: "OPENCODE_EXPERIMENTAL"
    effect: "Enables experimental feature umbrella."
    notes: "Can alter tools, event system, background subagents, LSP, and runtime behavior; avoid relying on experimental stream fields as stable."
  - name: "OPENCODE_EXPERIMENTAL_BACKGROUND_SUBAGENTS"
    effect: "Enables background subagent tasks."
    notes: "Can alter subagent timing and visibility."
  - name: "OPENCODE_EXPERIMENTAL_EVENT_SYSTEM"
    effect: "Enables experimental event system."
    notes: "Potential stream/event drift risk."
  - name: "OPENCODE_EXPERIMENTAL_OUTPUT_TOKEN_MAX"
    effect: "Sets max output tokens for LLM responses."
    notes: "Can produce output-length failures."
  - name: "OPENCODE_EXPERIMENTAL_BASH_DEFAULT_TIMEOUT_MS"
    effect: "Sets default bash tool timeout in milliseconds."
    notes: "Affects command execution failure timing."
  - name: "OPENCODE_CLIENT"
    effect: "Client identifier; `acp` command sets it to `acp`."
    notes: "May appear in logs/telemetry and affect client-specific behavior."
io_contract:
  stdout: structured_only
  stderr: mixed
  stdin: prompt
  framing: ndjson
  noise_handling: "Parse stdout as NDJSON only when `--format json` is supplied. Parse selected structured stderr log lines when `--print-logs --log-level INFO` is supplied; ignore known human chrome and unclassified stderr unless exit/failure classification needs it."
  notes: "For `opencode run --format json`, stdout is one JSON object per line with no terminal completion line. Non-TTY stdin is consumed once as prompt text and appended to argv prompt text. ACP changes stdin/stdout into a bidirectional NDJSON protocol and should be treated separately."
stream_contract:
  discriminator: "stdout: type; stderr logs: header + service/message classification"
  event_ordering: "stdout follows subscribed server events for the active session and generally emits step_start before text/tool_use/step_finish, but open issues show session.status idle can race ahead of late part events in some environments."
  correlation_fields: ["sessionID", "part.id", "part.messageID", "part.callID", "stderr session.id", "stderr parentID", "stderr providerID", "stderr modelID"]
  terminal_event: "none in stdout NDJSON; infer from process exit plus absence/presence of error, or from stderr `session.prompt ... exiting loop`; server/plugin streams expose `session.idle`/`session.status`."
  partial_message_events: false
  unknown_event_policy: "Preserve unknown stdout `type` records as provider extensions and log at trace; preserve unclassified stderr as raw diagnostics."
  notes: "CLI JSON emits completed text/reasoning blocks only after `part.time.end`, not token deltas. Raw server/SSE and generated SDK types include richer `session.next.*` delta events, but those are not the `run --format json` stdout contract."
session_metadata:
  session_id: "stdout `sessionID` on every emitted JSON line; stderr `service=session id=... created`; server `Session.id`."
  cwd: "Not in stdout NDJSON. `run --dir` selects cwd; `--attach --dir` selects remote cwd. Server `/path` and stderr/config logs can reveal directory."
  model: "Not reliably in stdout NDJSON. Requested model comes from `--model`; resolved provider/model appears on stderr `service=llm providerID=... modelID=... mode=primary stream` and raw server events."
  provider: "Not in stdout NDJSON except embedded provider metadata on some parts. stderr LLM-call tags expose `providerID`; config/provider APIs expose provider state."
  auth: "Not in stdout NDJSON. Auth failures surface as `error` records or stderr `AuthFailure`/provider error text; server `/provider/auth` lists methods."
  version: "Not in stdout NDJSON. `opencode --version` prints version; `--print-logs` boot banner includes `version`; server health returns version."
  mcp_servers: "Not in stdout NDJSON. Config/server `/mcp` expose MCP status; MCP tools appear as normal tool names when called."
  permission_mode: "Not directly in stdout NDJSON. Config `permission`, `--auto`, `OPENCODE_PERMISSION`, and stderr permission-evaluated records reveal behavior."
  notes: "For Claudine, stdout provides early session ID only after the first emitted event. stderr printed logs provide earlier boot/session/model context and should backfill the summary."
stream_events:
  - event: "tool_use"
    category: tool_result
    fields: ["type", "timestamp", "sessionID", "part.id", "part.messageID", "part.callID", "part.tool", "part.state.status", "part.state.input", "part.state.output", "part.state.error", "part.state.metadata", "part.state.time"]
    notes: "Emitted only when a tool reaches `completed` or `error`; there is no stdout call-start event."
  - event: "step_start"
    category: session
    fields: ["type", "timestamp", "sessionID", "part.id", "part.messageID", "part.type", "part.snapshot"]
    notes: "Start marker for an assistant step."
  - event: "step_finish"
    category: usage
    fields: ["type", "timestamp", "sessionID", "part.id", "part.messageID", "part.reason", "part.snapshot", "part.cost", "part.tokens.total", "part.tokens.input", "part.tokens.output", "part.tokens.reasoning", "part.tokens.cache.read", "part.tokens.cache.write"]
    notes: "Best stdout source for token and cost accounting."
  - event: "text"
    category: assistant
    fields: ["type", "timestamp", "sessionID", "part.id", "part.messageID", "part.text", "part.time.start", "part.time.end", "part.metadata"]
    notes: "Completed assistant text block; user prompt text is not emitted."
  - event: "reasoning"
    category: reasoning
    fields: ["type", "timestamp", "sessionID", "part.id", "part.messageID", "part.text", "part.time.start", "part.time.end", "part.metadata"]
    notes: "Only emitted when `--thinking` is enabled and the reasoning part has ended."
  - event: "error"
    category: error
    fields: ["type", "timestamp", "sessionID", "error.name", "error.data", "error.message"]
    notes: "Session or immediate prompt/command error; exit code is set non-zero for accumulated session errors."
  - event: "stderr BootBanner"
    category: session
    fields: ["level", "timestamp", "service", "version", "args", "process_role", "run_id"]
    notes: "Printed only with `--print-logs`; useful stream anchor and version source."
  - event: "stderr SessionCreated"
    category: session
    fields: ["level", "timestamp", "service=session", "id", "parentID", "title"]
    notes: "Parent session maps to session start; child session with `parentID` maps to subagent start."
  - event: "stderr LlmCall"
    category: session
    fields: ["level", "timestamp", "service=llm", "providerID", "modelID", "mode", "stream", "agent", "small", "session.id"]
    notes: "Best live source for resolved provider/model identity."
  - event: "stderr StepLoop"
    category: session
    fields: ["level", "timestamp", "service=session.prompt", "session.id", "step"]
    notes: "Progress heartbeat while stdout may be silent."
  - event: "stderr StepExit"
    category: session
    fields: ["level", "timestamp", "service=session.prompt", "session.id", "message"]
    notes: "Loop closure; for tracked child sessions Claudine synthesizes subagent stop."
  - event: "stderr PermissionEvaluated"
    category: permission
    fields: ["level", "timestamp", "service=permission", "permission", "pattern", "action"]
    notes: "Shows permission policy evaluation; stdout does not expose permission.asked/replied."
  - event: "stderr HttpResponse"
    category: other
    fields: ["level", "timestamp", "service=default", "http.method", "http.url", "http.status", "duration_ms"]
    notes: "Useful for live activity and provider/API failure timing."
  - event: "server.connected"
    category: session
    fields: ["type", "properties"]
    notes: "First `/event` SSE event from the server API; not emitted by `run --format json` stdout."
  - event: "message.part.updated"
    category: other
    fields: ["id", "type", "properties.sessionID", "properties.part", "properties.time"]
    notes: "Raw bus event used internally by `run.ts`; the CLI converts only selected completed parts to stdout NDJSON."
  - event: "permission.asked"
    category: permission
    fields: ["id", "type", "properties.sessionID", "properties.permission", "properties.patterns"]
    notes: "Raw bus/plugin/SSE event; `run.ts` replies auto-approve or reject but does not forward it to stdout."
  - event: "session.error"
    category: error
    fields: ["id", "type", "properties.sessionID", "properties.error"]
    notes: "Raw bus event converted to stdout `error` for the active session."
  - event: "session.status"
    category: session
    fields: ["id", "type", "properties.sessionID", "properties.status.type"]
    notes: "Raw bus event that makes `run.ts` break when status is `idle`; no stdout terminal record."
tools:
  - name: "bash"
    call_visible: false
    result_visible: true
    metadata: ["part.tool", "part.callID", "part.state.input.command", "part.state.output", "part.state.error", "part.state.time"]
    notes: "Command stdout/stderr are embedded in completed tool output/error text, not separate structured streams."
  - name: "edit"
    call_visible: false
    result_visible: true
    metadata: ["part.state.input", "part.state.output", "part.state.error", "part.state.metadata", "file paths in tool payload"]
    notes: "File changes are visible as completed tool results and snapshots, not as dedicated stdout file-change events."
  - name: "write"
    call_visible: false
    result_visible: true
    metadata: ["part.state.input", "part.state.output", "part.state.error"]
    notes: "Controlled by the `edit` permission."
  - name: "apply_patch"
    call_visible: false
    result_visible: true
    metadata: ["part.state.input.patchText", "part.state.output", "part.state.error"]
    notes: "Docs state paths are embedded in patch marker lines and the tool is controlled by `edit` permission."
  - name: "read"
    call_visible: false
    result_visible: true
    metadata: ["part.state.input", "part.state.output", "part.state.error"]
    notes: "Permission-denied reads appear as errored `tool_use` records when the call reaches a terminal state."
  - name: "grep"
    call_visible: false
    result_visible: true
    metadata: ["part.state.input", "part.state.output", "part.state.error"]
    notes: "Completed search output is visible; no live progress event."
  - name: "glob"
    call_visible: false
    result_visible: true
    metadata: ["part.state.input", "part.state.output", "part.state.error"]
    notes: "Completed file matching output is visible."
  - name: "lsp"
    call_visible: false
    result_visible: true
    metadata: ["part.state.input", "part.state.output", "part.state.error"]
    notes: "Experimental; availability depends on `OPENCODE_EXPERIMENTAL_LSP_TOOL` or `OPENCODE_EXPERIMENTAL`."
  - name: "skill"
    call_visible: false
    result_visible: true
    metadata: ["part.state.input", "part.state.output", "part.state.error"]
    notes: "Loads skill content into the conversation; skill discovery can be affected by config dirs and Claude Code compatibility env vars."
  - name: "todowrite"
    call_visible: false
    result_visible: true
    metadata: ["part.state.input", "part.state.output", "part.state.error"]
    notes: "Plan/todo state is only visible through completed tool results in CLI JSON; subagents disable this by default unless configured."
  - name: "webfetch"
    call_visible: false
    result_visible: true
    metadata: ["part.state.input", "part.state.output", "part.state.error"]
    notes: "Completed fetch result is visible as a `tool_use` record."
  - name: "websearch"
    call_visible: false
    result_visible: true
    metadata: ["part.state.input", "part.state.output", "part.state.error"]
    notes: "Availability can change with Exa/parallel experimental flags and provider configuration."
  - name: "question"
    call_visible: false
    result_visible: false
    metadata: ["raw permission/question bus events only outside CLI stdout"]
    notes: "`opencode run` creates fresh sessions with `question` denied, so human questions should be blocked rather than hanging."
  - name: "task"
    call_visible: false
    result_visible: true
    metadata: ["part.tool=task", "part.state.metadata.sessionId", "stderr child session id", "stderr parentID"]
    notes: "CLI stdout shows final task tool result; live subagent start/stop requires stderr logs or raw server/plugin events."
  - name: "MCP tools"
    call_visible: false
    result_visible: true
    metadata: ["part.tool", "part.callID", "part.state.input", "part.state.output", "part.state.error"]
    notes: "MCP calls appear as normal tool parts; MCP server status is available through config/server surfaces, not stdout NDJSON."
completion:
  success_event: "none"
  failure_event: "stdout `error` or classified stderr fatal error"
  exit_code_reliable: false
  result_fields: ["text.part.text", "reasoning.part.text", "tool_use.part.state.output", "step_finish.part.reason"]
  cost_fields: ["step_finish.part.cost", "Session.cost in server/SDK responses"]
  usage_fields: ["step_finish.part.tokens.input", "step_finish.part.tokens.output", "step_finish.part.tokens.reasoning", "step_finish.part.tokens.cache.read", "step_finish.part.tokens.cache.write", "step_finish.part.tokens.total"]
  notes: "Exit code is useful but not sufficient: stdout has no terminal success event, attach mode does not await the subscription loop before returning, and open issues report successful exits with incomplete or empty stdout for some sessions/environments."
blocking_behavior:
  permissions: configurable
  questions: fail
  tool_approvals: configurable
  notes: "`opencode run` creates non-interactive sessions with `question`, `plan_enter`, and `plan_exit` denied. For permission.asked, current source replies `once` when `--auto`, `--yolo`, or `--dangerously-skip-permissions` is set; otherwise it prints a warning and replies `reject`. Explicit deny rules still win."
subagents:
  supported: true
  start_visible: true
  stop_visible: true
  nested_events_visible: false
  prompt_injection_supported: true
  metadata_fields: ["stdout `tool_use.part.tool=task`", "stdout `part.state.metadata.sessionId`", "stderr `service=session id=... parentID=... created`", "stderr `service=session.prompt session.id=... exiting loop`", "agent config `mode`, `prompt`, `permission.task`"]
  notes: "Subagents are invoked through the Task tool and configured as agents with `mode: subagent` or `all`. CLI stdout sees final task completion; Claudine needs stderr or server/plugin events for live child-session lifecycle."
use_cases:
  - name: plan_cap_approaching
    detectable: false
    event_types: []
    fields: []
    hook_parity: "unknown"
    notes: "No first-class approaching-cap event was verified in CLI JSON or printed logs."
  - name: plan_capped
    detectable: true
    event_types: ["stdout error", "stderr ProviderLimit UsageCap", "stderr ProviderLimit RetriesExhausted"]
    fields: ["error.name", "error.data", "stderr status_code", "stderr reset_at", "stderr provider_id", "stderr model_id", "stderr provider_error"]
    hook_parity: "Raw `session.error` and provider/plugin events may see the underlying error."
    notes: "Claudine's stderr classifier distinguishes usage caps from transient rate limits/overload when provider error context proves it."
  - name: no_funds
    detectable: true
    event_types: ["stdout error", "stderr ApiFailure", "stderr ProviderLimit UsageCap"]
    fields: ["error.name", "error.data.message", "stderr provider_error", "stderr status_code"]
    hook_parity: "Raw `session.error` may expose the same provider error."
    notes: "No dedicated no-funds event; classify from provider-specific quota/billing text and status codes."
  - name: auth
    detectable: true
    event_types: ["stdout error", "stderr AuthFailure", "stderr ApiFailure"]
    fields: ["error.name", "error.data.message", "stderr message"]
    hook_parity: "Raw `session.error`; server `/provider/auth` lists configured auth methods but not secrets."
    notes: "Auth source/kind is not reliably exposed by `run --format json`."
  - name: permission_read_denied
    detectable: true
    event_types: ["tool_use", "stderr PermissionEvaluated"]
    fields: ["part.tool", "part.state.status", "part.state.error", "part.state.input.path", "stderr permission", "stderr pattern", "stderr action"]
    hook_parity: "`permission.asked`/`permission.replied` and tool hooks are richer than CLI stdout."
    notes: "Prefer errored `tool_use` for concrete path; stderr permission logs may only show permission/pattern/action."
  - name: permission_write_denied
    detectable: true
    event_types: ["tool_use", "stderr PermissionEvaluated"]
    fields: ["part.tool", "part.state.status", "part.state.error", "part.state.input.path", "part.state.input.patchText", "stderr permission", "stderr pattern", "stderr action"]
    hook_parity: "`permission.asked`/`permission.replied` and tool hooks are richer than CLI stdout."
    notes: "Write-like tools are `edit`, `write`, and `apply_patch`; `apply_patch` paths are inside patch text."
  - name: tokens_consumed
    detectable: true
    event_types: ["step_finish"]
    fields: ["part.cost", "part.tokens.input", "part.tokens.output", "part.tokens.reasoning", "part.tokens.cache.read", "part.tokens.cache.write", "part.tokens.total"]
    hook_parity: "Server/SDK AssistantMessage and Session also include cost/tokens."
    notes: "Units are token counts and cost numeric values as emitted by OpenCode; sum per `step_finish` for session totals."
  - name: model_used
    detectable: true
    event_types: ["stderr LlmCall", "raw message.updated", "server/SDK AssistantMessage"]
    fields: ["stderr providerID", "stderr modelID", "stderr mode", "message.info.providerID", "message.info.modelID"]
    hook_parity: "Raw server/plugin events expose message/provider metadata."
    notes: "Requested `--model` is an input; resolved model is best observed from stderr LLM-call logs."
  - name: model_fallback
    detectable: false
    event_types: []
    fields: []
    hook_parity: "unknown"
    notes: "No explicit fallback event verified; compare requested model to stderr/raw resolved model as an inference only."
  - name: human_in_loop
    detectable: true
    event_types: ["stderr PermissionEvaluated", "raw permission.asked", "raw question events"]
    fields: ["permission", "pattern", "action", "question request fields in raw events"]
    hook_parity: "Plugin/SSE event stream is richer; CLI stdout omits permission and question events."
    notes: "In `run`, human questions/plan approvals are denied up front and permissions are auto-approved only with `--auto`/hidden yolo flags."
  - name: session_resumable
    detectable: true
    event_types: ["stdout any event", "stderr SessionCreated", "server Session"]
    fields: ["sessionID", "stderr id", "Session.id"]
    hook_parity: "Server and plugin session events expose IDs."
    notes: "No final stdout completion record, but any stdout event carries a resumable session ID."
  - name: subagent_prompt_injection
    detectable: true
    event_types: ["agent config", "stdout task tool", "stderr child session"]
    fields: ["agent.<name>.prompt", "agent.<name>.mode", "permission.task", "part.state.metadata.sessionId", "stderr parentID"]
    hook_parity: "Agent config and plugin hooks can see more than CLI stdout."
    notes: "Use `.opencode/agents/*.md` or `opencode.json` agent definitions to steer subagents away from interactive behavior."
headless_constraints:
  - constraint: "No stdout terminal completion event."
    mitigation: "Use process exit plus accumulated stdout/stderr state; parse stderr `StepExit` and fatal classifications for better lifecycle."
    notes: "Open issues request a final session/completion event."
  - constraint: "Tool calls are DONE-only on stdout."
    mitigation: "Use stderr StepLoop/LlmCall for live progress and raw server/plugin events if true tool-start visibility is required."
    notes: "`tool_use` only appears after completed/error state."
  - constraint: "User prompt is not emitted in JSON mode."
    mitigation: "Claudine must retain the submitted prompt in its own run metadata if transcript reconstruction needs it."
    notes: "Open issue reports `run --format json` never emits the user prompt."
  - constraint: "stdout can be incomplete or empty in race/resume edge cases."
    mitigation: "Classify missing terminal records as ambiguous; consider server/SDK transcript lookup by session ID after exit when available."
    notes: "Recent issues report dropped `text`/`step_finish` in containers and empty stdout for some resumed sessions."
  - constraint: "Permission questions are not programmable through stdout."
    mitigation: "Set explicit permissions and use `--auto` only when policy allows; parse stderr permission evaluations."
    notes: "Without auto, current source auto-rejects permission requests after warning."
  - constraint: "Config layers can change model, tools, permissions, plugins, MCP, agents, and output-adjacent behavior."
    mitigation: "Capture relevant env/config context in Claudine reports; use `--pure` for reduced plugin drift when appropriate."
    notes: "OpenCode merges remote, global, custom, project, `.opencode`, inline, and managed settings."
  - constraint: "ACP is bidirectional, not a simple event stream."
    mitigation: "Do not parse ACP stdout with the `run --format json` parser."
    notes: "ACP may be useful for a future protocol adapter."
quirks:
  - "`--format json` means NDJSON stdout, not a single JSON result and not the raw full event bus."
  - "The top-level discriminator is `type`, but embedded part types use hyphenated names such as `step-start` and `step-finish` while CLI event types use underscores such as `step_start` and `step_finish`."
  - "Timestamps from the CLI JSON envelope are Unix epoch milliseconds from `Date.now()`; stderr printed logs use formatted timestamps parsed as UTC by Claudine."
  - "Raw server/SSE events include richer `session.next.*` delta events, but `run --format json` emits only completed text/reasoning blocks."
  - "The JSON stdout stream does not include user messages, permission.asked, permission.replied, session.status, or session.idle."
  - "`--command` uses a different server route and has had historical JSON-output bugs; treat command-mode fixtures separately from prompt-mode fixtures."
  - "In attach mode, current source returns without awaiting the event loop, so exit semantics may differ from local in-process runs."
  - "OpenCode config files are merged, not replaced; later layers override only conflicting keys."
  - "The old `tools` boolean config remains supported but permissions are now controlled by `permission`."
gaps:
  - "No provider-published formal schema was found for the exact `opencode run --format json` line envelope."
  - "No verified machine-readable final success event exists in CLI stdout."
  - "No direct CLI JSON field for cwd/project root, auth kind/source, MCP server list, sandbox mode, or complete permission mode was verified."
  - "No dedicated plan/quota approaching-cap event was verified."
  - "No explicit model-fallback event was verified; fallback can only be inferred by comparing requested and observed model IDs."
  - "The ACP protocol shape was not expanded here beyond entry point and framing; use the ACP skill before implementing an ACP adapter."
  - "Local execution of a real model run was not performed because it may require credentials and tool approvals; source and docs were used instead."
claudine_strategy:
  preferred_invocation: "opencode run --format json --print-logs --log-level INFO -- \"<prompt>\""
  required_flags: ["--format json", "--print-logs", "--log-level INFO", "--"]
  conflicting_flags: ["--mini", "--interactive", "ACP parser on run stdout", "text/default output parsing"]
  parser_notes: "Parse stdout as NDJSON with top-level `type`; parse stderr structured logs as a secondary lifecycle stream. Treat `tool_use` as completed/error only, `step_finish` as usage/cost source, and process exit without error as success only after accounting for missing terminal event risk."
  wrapper_notes: "Use argv prompt delivery separated by `--`; retain prompt text yourself because OpenCode does not emit user prompt events. Use `--auto` or explicit permissions only under a caller-approved policy. Keep server/SSE and ACP as future richer adapters, not the current subprocess parser."
data_format: ndjson
changes:
  - "2026-07-03: Reverified official docs and current source, observed local OpenCode 1.17.13 invalid-model failure framing, set `last_updated`, and normalized `config_files` frontmatter to OS-specific records."
  - "2026-07-02: Replaced the older ad hoc research note with schema-backed frontmatter, current OpenCode CLI/source findings, and Claudine's dual stdout/stderr parsing strategy."
requires_claudine_update: false
reason: "No immediate code change is required: Claudine's current OpenCode wrapper already uses `--format json --print-logs --log-level INFO` and has an OpenCode stderr bridge. This document refreshes provider metadata and records remaining parser risks."
---

# OpenCode Non-Interactive Sessions

## Summary

Claudine can run OpenCode non-interactively through `opencode run --format json`, which emits newline-delimited JSON records on stdout. That stream is the best subprocess-friendly primary format, but it is not enough by itself for wrapper-grade live status. The CLI JSON stream is a filtered projection of selected session events: completed text, completed reasoning, step start/finish, completed or errored tools, and errors. It does not emit the user prompt, raw permission events, tool-start events, model/provider identity in a stable init record, or a terminal success event.

The best Claudine strategy is therefore a dual-source contract: run `opencode run --format json --print-logs --log-level INFO -- "<prompt>"`, parse stdout as NDJSON, and parse selected structured stderr logs as the live lifecycle stream. stdout remains canonical for final assistant text, completed tool payloads, and `step_finish` token/cost accounting. stderr supplies the signals stdout lacks while the run is active: boot/version, session and child-session creation, resolved provider/model per LLM call, step-loop heartbeat, permission evaluations, HTTP spans, retry/cap/auth failures, and subagent lifecycle. The main risks are the missing terminal event, DONE-only tool visibility, config/plugin drift, and recent upstream reports of incomplete stdout in container/resume edge cases.

## Non-Interactive Entry Points

The documented simple entry point is `opencode run "Explain how closures work in JavaScript"`; the CLI page says OpenCode starts the TUI when run without arguments but also provides commands for programmatic use. The `run` source currently defines positional `message`, `--format default|json`, `--model`, `--agent`, `--file`, `--dir`, `--continue`, `--session`, `--fork`, `--command`, `--attach`, `--auto`, `--thinking`, and related flags. In non-TTY mode it reads all stdin once with `Bun.stdin.text()` and appends that text to the positional message. Claudine still should deliver the prompt as argv separated by `--`, because that is the native command contract and avoids yargs interpreting a prompt that begins with `-` as flags.

`opencode run` has three relevant launch shapes:

| Entry point | Fresh/resume/server | Prompt input | Claudine value |
| --- | --- | --- | --- |
| `opencode run --format json -- "<prompt>"` | Fresh local session with an in-process server | argv plus appended stdin | Best subprocess wrapper mode |
| `opencode run --continue` / `--session <id>` | Resume or fork an existing session | argv plus appended stdin | Useful when Claudine wants resumability |
| `opencode run --attach <url>` | Talks to a long-running server | argv plus appended stdin | Useful if Claudine manages a server separately |

OpenCode also exposes two richer automation surfaces. `opencode serve` starts a headless HTTP server with an OpenAPI endpoint and SSE event streams; the server docs describe `/doc` as the OpenAPI 3.1 spec, `/event` as an SSE stream whose first event is `server.connected`, and `/global/event` as a global event stream. `opencode acp` starts an Agent Client Protocol server over newline-delimited JSON on stdin/stdout. Those are real non-interactive modes, but they are different integration shapes: server mode is HTTP/SSE request-reply plus streams, and ACP is a bidirectional protocol, not a one-way run log.

Attachments are supported with repeated `--file`. For local `run`, file parts are file URLs or directory markers. In `--attach` mode, source reads local files and sends data URLs, refuses local directories, and rejects files larger than 10 MiB or special files. `--dir` changes the local process cwd for non-attach runs and selects the remote directory for attach mode.

## Output Formats

For `opencode run`, current source enumerates exactly two values for `--format`: `default` and `json`. Other OpenCode commands use their own `--format json` contracts; for example list/database-style commands can return ordinary JSON documents or TSV. Do not reuse the `run` stream parser for those commands.

| Format | Selector | Framing | Streams? | Claudine preference | Notes |
| --- | --- | --- | --- | --- | --- |
| Human text | `--format default` | Text | Yes | No | Human stdout/stderr, status lines, and ANSI-style UI are not stable. |
| CLI JSON | `--format json` | NDJSON on stdout | Yes | Primary | Best canonical subprocess stream, but filtered and no terminal success event. |
| CLI JSON + logs | `--format json --print-logs --log-level INFO` | NDJSON stdout + structured/plain stderr | Yes | Preferred | This is the wrapper-grade mode Claudine should use. |
| Server API | `opencode serve` | JSON request/reply | Usually no | Secondary | Strong schema, but not a live stream unless paired with SSE. |
| Server events | `GET /event` / `/global/event` | SSE | Yes | Future richer adapter | Full bus events are broader than CLI JSON. |
| ACP | `opencode acp` | NDJSON protocol | Yes, bidirectional | Future protocol adapter | Requires ACP client behavior and request handling. |

`run --format json` writes one JSON object per line to stdout through an internal `emit(type, data)` helper. Each line has `type`, `timestamp`, `sessionID`, and then either `part` or `error`. The currently emitted stdout `type` values are `tool_use`, `step_start`, `step_finish`, `text`, `reasoning`, and `error`. This is a filtered CLI envelope over server events from `client.event.subscribe()`, not the raw bus. For example, `message.part.updated` is converted only if it is an active-session part that satisfies the CLI filters.

The stream choice changes behavior. Text output is no longer a human transcript; stdout becomes parseable NDJSON. But JSON mode also suppresses useful human-only lines and omits several raw event families. Tool calls are visible only after a tool reaches `completed` or `error`. Text and reasoning are emitted only when their part has an end time; there are no token deltas. Permission requests are handled internally and never forwarded to stdout. Completion is inferred from process exit or from raw/stderr lifecycle, not from a final stdout record.

The server/SSE stream is richer. The generated SDK types include raw events such as `session.created`, `message.updated`, `message.part.updated`, `permission.asked`, `session.status`, and `session.next.*` delta events. That richer stream is valuable context for future adapters, but Claudine's current wrapper needs a robust subprocess mode, so stdout NDJSON plus printed stderr logs remains the pragmatic recommendation.

## Schema Sources

There is no official JSON Schema, OpenAPI component, or named TypeScript union for the exact `opencode run --format json` stdout line envelope. The authoritative source for that envelope is the CLI implementation in `packages/opencode/src/cli/cmd/run.ts`, which constructs each line with `JSON.stringify({ type, timestamp: Date.now(), sessionID, ...data })`.

OpenCode does publish strong schemas for the underlying model:

| Source | Formality | Scope | Parser relevance |
| --- | --- | --- | --- |
| CLI docs | Official prose/examples | Commands and flags | Confirms `run` and global log flags, but not event shape. |
| `run.ts` | Source implementation | Exact CLI stdout envelope | Best evidence for `type` values, filters, stdin, permissions, and exit behavior. |
| Server docs + `/doc` | OpenAPI 3.1 | HTTP routes and SSE surfaces | Strong formal source for server/SDK adapters. |
| `packages/sdk/js/src/v2/gen/types.gen.ts` | Generated TypeScript | Session, message, part, event, permission, tokens, cost | Best readable schema for embedded `part` payloads. |
| `message-v2.ts` | Source/runtime semantics | Session/message hydration and error handling | Useful for understanding what generated types mean. |
| Plugin docs | Official event list | Raw event and hook names | Confirms richer events than CLI stdout exposes. |

The important confidence distinction is that `ToolPart`, `StepFinishPart`, `TextPart`, `ReasoningPart`, and raw event types are formally generated SDK types, while the CLI line wrapper is an implementation convention. Claudine should make its parser permissive: require only `type` and valid JSON per line, treat unknown `type` as provider extensions, and avoid assuming the underlying `part` object is exhaustive.

## IO Contract

With `--format json`, stdout is parse-only NDJSON for the `run` command. Each stdout line is independently parseable JSON. The parser discriminator is top-level `type`. The embedded payload usually has a `part` object whose own discriminator is `part.type`, using OpenCode's internal hyphenated names such as `step-start`, `step-finish`, `text`, `reasoning`, and `tool`.

stderr is mixed. Without `--print-logs`, stderr can contain human warnings, share URLs, and fatal errors while structured logs go to OpenCode's log files. With `--print-logs --log-level INFO`, stderr additionally carries structured log lines. Claudine's OpenCode bridge parses both legacy `LEVEL TIMESTAMP +Nms ...` lines and newer `timestamp=... level=...` lines, then classifies selected records. It must still tolerate raw text and UI chrome because OpenCode can write warnings or top-level fatal errors outside the structured logger.

stdin is prompt text for `run`, not an interactive protocol. Source checks `process.stdin.isTTY`; when stdin is not a TTY, it reads all stdin and appends it to the positional message. That means wrappers must close stdin and should not expect mid-run replies. ACP is the exception: `opencode acp` converts stdin/stdout into an ACP NDJSON protocol and cannot be parsed with the `run` stream parser.

## Stream Contract

The stdout event ordering is good enough for a single-pass parser but not strong enough to infer completion from an event. `run.ts` subscribes to server events, sends the prompt/command, and breaks the loop when it observes `session.status` with `status.type === "idle"` for the active session. It does not emit that idle event to stdout. Recent upstream issues report that in some environments the idle event can race ahead of late `text`/`step_finish` events, causing incomplete stdout, and that some resumed sessions can persist assistant output while emitting empty stdout. Claudine should therefore classify "process exited 0 but no terminal event" as normal only when enough expected records were observed, and keep a path for ambiguous/incomplete-output diagnostics.

The stdout contract by event:

| stdout `type` | Source filter | Important fields | Completion behavior |
| --- | --- | --- | --- |
| `step_start` | Any active-session `part.type === "step-start"` | `sessionID`, `part.id`, `part.messageID`, `part.snapshot` | Activity marker only |
| `text` | `part.type === "text" && part.time?.end` | `part.text`, `part.time`, `part.metadata` | Completed block, not delta |
| `reasoning` | `part.type === "reasoning" && part.time?.end && --thinking` | `part.text`, `part.metadata`, `part.time` | Completed block, opt-in |
| `tool_use` | `part.type === "tool"` and status `completed` or `error` | `part.callID`, `part.tool`, `part.state.*` | Tool terminal state only |
| `step_finish` | `part.type === "step-finish"` | `part.reason`, `part.cost`, `part.tokens` | Usage/cost marker |
| `error` | `session.error` or immediate prompt/command route error | `error.name`, `error.data`, `error.message` | Failure marker |

Tool correlation uses `part.callID` for model tool calls and `part.id` for the part itself. Tool inputs/results live under `part.state`: `pending`/`running` states exist in generated types, but the CLI only emits `completed` and `error` states as `tool_use`. Completed tools have `part.state.input`, `part.state.output`, `part.state.title`, `part.state.metadata`, `part.state.time.start`, and `part.state.time.end`; errored tools have `part.state.input`, `part.state.error`, optional metadata, and time bounds.

stderr classification provides a second discriminator set. Claudine maps boot banners, session creation, LLM calls, step loops/exits, permission evaluations, HTTP responses, snapshots, malformed assets, auth failures, API failures, provider limits, and uncaught errors into semantic events. These are not official OpenCode schemas, but they are essential for live rendering because stdout can remain silent during long generations, retries, and subagent work.

## Session Metadata

stdout gives `sessionID` on every emitted line, but not before the first event and not in a final completion record. For a fresh local run, that is usually early enough to recover/resume after the first `step_start`; for a failure before session creation, Claudine may only have stderr or process state.

Model and provider identity are weaker in stdout. `--model provider/model` is the requested model, but the resolved backend model should be read from printed stderr logs, especially the first primary `service=llm providerID=... modelID=... mode=primary ... stream` line. Raw server/SDK `AssistantMessage` records also carry provider/model fields. The generated SDK types define assistant messages with cost and tokens, and raw events include `session.next.step.started` with agent and model, but `run --format json` does not forward that raw start event.

cwd/project/worktree metadata is mostly outside the stdout stream. `--dir` controls the local working directory for non-attach runs; `--attach --dir` controls the remote path. The server has `/path`, `/project`, `/project/current`, and `/vcs` endpoints. The plugin context receives project, directory, and worktree. Claudine should report the cwd it launched with rather than expecting OpenCode stdout to repeat it.

Auth source, MCP server list, sandbox mode, roots, and complete permission mode are not emitted in stdout NDJSON. Some are available from server/config endpoints or config files. Permission evaluations appear in stderr logs when printed, and provider auth failures appear as stdout/stderr errors.

## Event Families

OpenCode has three event layers relevant to Claudine:

| Layer | Examples | Claudine role |
| --- | --- | --- |
| CLI stdout NDJSON | `step_start`, `text`, `reasoning`, `tool_use`, `step_finish`, `error` | Primary parse stream for result, tools, usage, and errors |
| Printed stderr logs | boot banner, session created, LLM call, step loop, permission evaluated, HTTP response, provider limit/auth/API failure | Live lifecycle, model identity, subagent visibility, retry/cap/auth classification |
| Raw server/plugin/SSE bus | `server.connected`, `session.created`, `message.part.updated`, `permission.asked`, `session.status`, `session.idle`, `tool.execute.before/after`, `session.next.*` | Richer future adapter and schema evidence |

The plugin docs list raw event names including `message.part.updated`, `permission.asked`, `permission.replied`, `session.created`, `session.error`, `session.idle`, `session.status`, `tool.execute.before`, and `tool.execute.after`. The server docs expose `/event` and `/global/event` SSE streams. The generated SDK types go further and include `session.next.text.delta`, `session.next.reasoning.delta`, `session.next.tool.input.delta`, `session.next.tool.called`, and step-ended token/cost records. None of those richer raw events should be assumed to appear on `run --format json` stdout unless source starts forwarding them.

## Tools

OpenCode's built-in tool families include `bash`, `edit`, `write`, `read`, `grep`, `glob`, experimental `lsp`, `apply_patch`, `skill`, `todowrite`, `webfetch`, `websearch`, and `question`. It also supports custom tools and MCP server tools. The tools docs state that all tools are enabled by default and controlled through `permission`; `write` and `apply_patch` are controlled by the `edit` permission, and `apply_patch` paths are embedded in the patch text.

In CLI JSON mode, tool calls are DONE-only. There is no stdout record when a normal tool begins running. The only stdout `tool_use` record is emitted after `part.state.status` is `completed` or `error`. That means Claudine cannot show exact "bash started" or "edit started" progress from stdout alone. It can show step loops, LLM calls, and permission evaluations from stderr, and it can render completed tool results once `tool_use` arrives.

Command execution is represented structurally as a tool part, not as separate stdout/stderr streams. The command, exit status, stdout, stderr, and truncation details depend on the tool's input/output text and metadata. Claudine should treat `part.state.output` as provider-formatted tool output, not raw process stdout. File changes are likewise represented through tool outputs, patch text, snapshots, and server diff endpoints, not dedicated `file_change` stdout events.

MCP tools are surfaced as tool parts with ordinary tool names and `part.state.*`. MCP server status is available through config and server `/mcp`, not the stdout stream.

## Completion and Exit Status

Normal completion has no stdout event. `run.ts` stops its event loop when the raw subscribed stream reports `session.status` idle for the active session, then returns. If accumulated session errors were observed, local non-attach mode sets `process.exitCode = 1`. Immediate prompt/command route errors also emit an `error` record in JSON mode and set the exit code to 1.

Exit code is useful but not sufficient for Claudine's lifecycle model. The CLI can exit 0 without a terminal success record. Open issues also report successful exits with incomplete or empty stdout in some cases. Conversely, stderr can carry retry/cap/auth failures before stdout reports a final error. Claudine should combine signals:

- success: process exited 0, no stdout/stderr fatal error, and a plausible final assistant/tool/step state was observed;
- failure: stdout `error`, classified fatal stderr error, or non-zero exit;
- ambiguous: process exited but stdout was empty/incomplete, no terminal event was observed, or attach-mode loop semantics prevented clean completion inference;
- cancellation/interruption: process termination by wrapper/user signal, mapped by Claudine outside OpenCode's stdout schema.

Usage and cost are best read from each `step_finish.part`: `cost`, `tokens.input`, `tokens.output`, `tokens.reasoning`, `tokens.cache.read`, `tokens.cache.write`, and optional `tokens.total`. These are per-step records; Claudine should sum them for session totals. Server/SDK session and assistant message types also expose cost/tokens for post-run reconciliation.

## Blocking Behavior

OpenCode's non-interactive `run` mode is designed not to ask a human mid-run. Current source creates fresh non-interactive sessions with three explicit deny rules: `question`, `plan_enter`, and `plan_exit`. That means user questions and plan approval transitions should be blocked rather than hanging automation.

Regular permission requests are handled in the event loop. When `--auto`, hidden `--yolo`, or hidden `--dangerously-skip-permissions` is set, `run.ts` replies `once` to `permission.asked` for the active session. Otherwise it prints a warning and replies `reject`. The official permissions docs describe `--auto` as approving requests that are not explicitly denied, with explicit `deny` still enforced. This is configurable through the `permission` config and `OPENCODE_PERMISSION`, so Claudine should not assume `--auto` bypasses policy.

Auth and OAuth are different. Missing/expired provider auth can still fail the run. Server auth for `serve`/`web`/`attach` is HTTP basic auth controlled by `OPENCODE_SERVER_PASSWORD` and `OPENCODE_SERVER_USERNAME`; `serve` warns when no password is set. MCP OAuth or provider OAuth flows were not verified as non-interactive-safe through `run`; wrappers should treat auth setup as a preflight concern.

## Subagents

OpenCode supports subagents through agent configuration and the Task tool. Agent config can live in `opencode.json` or agent markdown files under `.opencode/agents/` or the user config directory. The agents docs define `mode` values `primary`, `subagent`, and `all`; direct `opencode run --agent <name>` rejects subagent-only agents and falls back to the default, while primary/all agents can invoke subagents through `permission.task`.

CLI stdout shows subagent work mainly as a final `tool_use` record for `part.tool === "task"`. That record can include metadata such as a child `sessionId`, but it arrives after the subagent completes or errors. Current Claudine behavior no longer treats the completed task tool as the source of truth for subagent lifecycle; instead, it parses printed stderr logs. A `service=session id=... parentID=... created` line marks child start, and the child's `service=session.prompt session.id=... exiting loop` marks child stop. Nested child tool calls are not forwarded as separate parent stdout events.

Prompt injection into subagents is supported operationally through agent definitions: put non-interactive instructions in the subagent prompt/config and constrain task permissions. There is no special `run --format json` field that injects subagent instructions per task.

## Use Case Detection

| Use case | Detectable? | Best signal | Notes |
| --- | --- | --- | --- |
| `plan_cap_approaching` | No | none verified | No first-class approaching-cap signal found. |
| `plan_capped` | Yes | stderr provider limit or stdout error | Claudine's classifier distinguishes usage cap from rate-limit/overload when error context is present. |
| `no_funds` | Partial | provider quota/billing error text | No dedicated event; classify provider-specific errors. |
| `auth` | Yes | stdout `error`, stderr `AuthFailure`, server auth endpoints | Auth kind/source is not reliably emitted by stdout. |
| `permission_read_denied` | Yes | errored `tool_use` and stderr permission evaluation | Path is most likely in tool input/error; stderr may show only pattern/action. |
| `permission_write_denied` | Yes | errored write/edit/apply_patch `tool_use` | `apply_patch` paths are embedded in patch text. |
| `tokens_consumed` | Yes | `step_finish.part.tokens` | Sum per step for session totals; token units are counts. |
| `model_used` | Yes | stderr `service=llm providerID/modelID` | stdout alone is weak; compare with requested `--model`. |
| `model_fallback` | Inference only | requested vs observed model | No explicit fallback event verified. |
| `human_in_loop` | Yes | raw permission/question events or stderr permission evaluation | stdout omits `permission.asked`; `run` auto-replies or denies. |
| `session_resumable` | Yes | `sessionID` on stdout/stderr/server session | Available after first stdout event or session-created stderr line. |
| `subagent_prompt_injection` | Yes | agent config | Use agent prompt and `permission.task`, not a stream field. |

Timestamps differ by stream. stdout `timestamp` is epoch milliseconds from `Date.now()`. Stderr printed logs use formatted timestamps; Claudine parses both observed header formats into UTC `DateTime`. Provider reset times, when detected from error text, should be treated as provider-supplied and possibly timezone-ambiguous unless the parsed text includes enough context.

## Headless Constraints

OpenCode is scriptable, but there are practical constraints:

- Use `--format json` every run; there is no persistent config key documented for making `run` JSON output the default.
- Use `--print-logs --log-level INFO` if Claudine needs live status beyond completed stdout records.
- Do not parse default text output.
- Do not treat stderr as noise once printed logs are enabled.
- Do not expect a terminal success event.
- Do not expect user prompt replay on stdout.
- Do not expect live tool-start events on stdout.
- Do not rely on `--auto` to override explicit deny rules.
- Treat config layers and plugins as stream-affecting because they can change permissions, tools, agents, MCP, commands, and hooks.
- Treat ACP as a separate protocol adapter.

The biggest automation hazard is ambiguity at the end of a run. `session.status idle` is consumed internally but not emitted. If the process exits 0 after only `step_start`, or exits 0 with no stdout for a resumed session, Claudine cannot confidently produce a normal success report without another evidence source such as stderr, server transcript lookup, or a provider-specific reconciliation step.

## Timeline

| Date | Event | Why it matters |
| --- | --- | --- |
| 2025-06-29 | PR `#533` proposed an earlier `run --print` mode with `json` and `stream-json` outputs. | Shows machine-readable non-interactive output was actively designed before the current shape. |
| 2025-10-31 | PR `#3638` aligned docs/source around `opencode run --format json`. | Earliest clear upstream evidence for the current `--format json` contract found in prior research. |
| 2026-02-12 | SDK structured output shipped in release `v1.1.60`. | Adds JSON-schema-constrained model output in SDK/server paths, separate from CLI stream framing. |
| 2026-03-19 | PR `#18249` proposed running `tool_use` events in JSON mode. | Evidence that current DONE-only tool visibility is a known integration weakness. |
| 2026-04-06 | Release `v1.3.16` fixed output token totals when reasoning tokens are separated. | Token accounting from `step_finish` can drift with provider/runtime changes. |
| 2026-06-08 | Issue `#31435` reported containerized `run --format json` losing `text` and `step_finish` after `step_start`. | Confirms the missing terminal event and idle-race risk are operationally relevant. |
| 2026-06-09 | Issue `#31482` reported resumed-session JSON mode exiting successfully with empty stdout while DB contained the answer. | Confirms successful exit is not enough to prove the stdout stream was complete. |
| 2026-07-03 | Official docs observed with `run`, `serve`, `acp`, global `--print-logs`, and current config precedence. | Current research baseline for this document. |
| 2026-07-03 | Local OpenCode `1.17.13` no-quota probe with an invalid model emitted stdout `error` plus printed stderr lifecycle logs. | Confirms current process/error framing without running a real model prompt. |

## Quirks and Gaps

The central quirk is that OpenCode's most convenient machine-readable mode is both useful and intentionally narrow. It is NDJSON, but not a complete event bus. It emits enough to reconstruct final assistant text, completed tools, and usage, but it drops the exact signals a wrapper most wants while the run is active. That is why Claudine's stderr bridge is not optional for OpenCode.

The second quirk is casing and naming. CLI stdout uses snake_case `type` values such as `step_finish`, while embedded `part.type` values use hyphenated OpenCode names such as `step-finish`. Raw server events use dotted names such as `message.part.updated` and `session.next.text.delta`. A parser must keep these namespaces separate.

The biggest gaps are the absence of a formal CLI line schema, absence of a stdout terminal success event, incomplete stdout metadata for model/cwd/auth/MCP/permissions, and unverified behavior around MCP OAuth or provider OAuth in fully non-interactive runs. Local execution of a real model prompt was not performed for this refresh because it could require credentials and mutate files or consume quota; source, docs, local wrapper code, and public issue evidence were used instead.

A local July 3, 2026 probe with OpenCode `1.17.13` used `OPENCODE_CONFIG_CONTENT` to select an invalid model and ran `opencode run --format json --print-logs --log-level INFO` with both argv and stdin prompt forms. It exited non-zero, printed lifecycle/config/session logs on stderr, and emitted a stdout JSON error with `type: "error"`, epoch-millisecond `timestamp`, `sessionID`, and nested `error.name` / `error.data`. That probe verifies framing and failure behavior only; it does not prove successful-run tool or token payloads.

## Claudine Integration Notes

Use:

```bash
opencode run --format json --print-logs --log-level INFO -- "<prompt>"
```

Parse stdout line-by-line as NDJSON. Use top-level `type` as the discriminator. Treat `text` and `reasoning` as complete blocks, not deltas. Treat `tool_use` as both the call and terminal result because OpenCode does not emit a separate stdout start event. Join tool information by `part.callID`; preserve `part.id` and `part.messageID` for transcript/debug correlation. Sum usage from `step_finish.part.tokens` and cost from `step_finish.part.cost`.

Parse stderr as a secondary stream when printed logs are enabled. Promote only recognized structured records: boot/version, parent session start, child session start/stop, primary LLM calls, genuine step transitions, permission evaluations, HTTP responses, malformed assets, auth failures, provider limits, API failures, and uncaught fatal errors. Keep unknown stderr as diagnostics, not as assistant output.

Do not wait for a stdout completion record; none exists. Completion classification must combine process exit, stdout errors, stderr fatal classifications, and observed progress. A zero exit with missing `text`/`step_finish` should be reportable as ambiguous because upstream issues show incomplete stdout can happen.

Prefer explicit permissions in config for deterministic automation. Use `--auto` only when the caller policy permits auto-approval. `--dangerously-skip-permissions` is accepted by current source and used by Claudine's yolo path, but the public documented flag is `--auto`.

Keep server/SSE and ACP separate in the adapter design. They are promising richer surfaces, especially for raw permission events and deltas, but their IO contracts differ from the subprocess NDJSON mode.

## Changelog

- 2026-07-03: Reverified official docs and current source, observed local OpenCode `1.17.13` invalid-model failure framing, set `last_updated`, and normalized `config_files` frontmatter to the topic schema's OS-specific records.
- 2026-07-02: Replaced the older ad hoc research note with schema-backed frontmatter, current OpenCode CLI/source findings, and Claudine's dual stdout/stderr parsing strategy.

## Sources

- OpenCode CLI docs: <https://opencode.ai/docs/cli/>
- OpenCode config docs: <https://opencode.ai/docs/config/>
- OpenCode server docs: <https://opencode.ai/docs/server/>
- OpenCode SDK docs: <https://opencode.ai/docs/sdk/>
- OpenCode tools docs: <https://opencode.ai/docs/tools/>
- OpenCode permissions docs: <https://opencode.ai/docs/permissions/>
- OpenCode agents docs: <https://opencode.ai/docs/agents/>
- OpenCode plugins docs: <https://opencode.ai/docs/plugins/>
- OpenCode `run` source: <https://github.com/anomalyco/opencode/blob/dev/packages/opencode/src/cli/cmd/run.ts>
- OpenCode `serve` source: <https://github.com/anomalyco/opencode/blob/dev/packages/opencode/src/cli/cmd/serve.ts>
- OpenCode `acp` source: <https://github.com/anomalyco/opencode/blob/dev/packages/opencode/src/cli/cmd/acp.ts>
- OpenCode CLI bootstrap source: <https://github.com/anomalyco/opencode/blob/dev/packages/opencode/src/index.ts>
- OpenCode generated SDK types: <https://github.com/anomalyco/opencode/blob/dev/packages/sdk/js/src/v2/gen/types.gen.ts>
- OpenCode session/message source: <https://github.com/anomalyco/opencode/blob/dev/packages/opencode/src/session/message-v2.ts>
- OpenCode OpenAPI spec: <https://raw.githubusercontent.com/anomalyco/opencode/dev/packages/sdk/openapi.json>
- OpenCode issue `#29997`, user prompt missing from JSON stream: <https://github.com/anomalyco/opencode/issues/29997>
- OpenCode issue `#31435`, container JSON stream drops late events: <https://github.com/anomalyco/opencode/issues/31435>
- OpenCode issue `#31482`, resumed session exits with empty stdout: <https://github.com/anomalyco/opencode/issues/31482>
- Claudine OpenCode event-source design note: `.claude/skills/claudine/opencode-event-sources.md`
- Claudine OpenCode wrapper profile: `claudine/cli/src/commands/wrap/profile/opencode.rs`
- Claudine OpenCode stdout parser: `claudine/lib/src/stream/providers/opencode.rs`
- Claudine OpenCode stderr log parser/classifier: `claudine/lib/src/stream/logs/opencode/`
