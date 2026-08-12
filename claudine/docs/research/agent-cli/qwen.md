---
$schema: ./_schema.yaml
created: 2026-07-02
last_updated: 2026-07-03
agent: codex
model: default
latest_version: "0.19.6"
homepage: https://qwen.ai/qwencode
repo: https://github.com/QwenLM/qwen-code
docs: https://qwenlm.github.io/qwen-code-docs/
cli_docs: https://qwenlm.github.io/qwen-code-docs/en/users/overview/
binaries:
  - os: macos
    binary: qwen
    alt_binaries: []
    notes: "Homebrew links /opt/homebrew/bin/qwen to the Cellar package on this host. npm exposes bin.qwen. Standalone docs also use qwen."
  - os: linux
    binary: qwen
    alt_binaries: []
    notes: "npm exposes bin.qwen; official standalone and Homebrew docs also use qwen."
  - os: windows
    binary: qwen
    alt_binaries: ["qwen.cmd", "qwen.ps1"]
    notes: "Official examples invoke qwen. npm package bins normally create .cmd and PowerShell shims on Windows; standalone shim names were not independently observed on this macOS host."
install_methods:
  - os: macos
    method: standalone_binary
    command: "curl -fsSL https://qwen-code-assets.oss-cn-hangzhou.aliyuncs.com/installation/install-qwen-standalone.sh | bash"
    notes: "Official quick install path."
  - os: linux
    method: standalone_binary
    command: "curl -fsSL https://qwen-code-assets.oss-cn-hangzhou.aliyuncs.com/installation/install-qwen-standalone.sh | bash"
    notes: "Official quick install path."
  - os: windows
    method: standalone_binary
    command: "irm https://qwen-code-assets.oss-cn-hangzhou.aliyuncs.com/installation/install-qwen-standalone.ps1 | iex"
    notes: "Official quick install path."
  - os: macos
    method: npm
    command: "npm install -g @qwen-code/qwen-code@latest"
    notes: "npm latest 0.19.6 declares Node.js >=22.0.0."
  - os: linux
    method: npm
    command: "npm install -g @qwen-code/qwen-code@latest"
    notes: "npm latest 0.19.6 declares Node.js >=22.0.0."
  - os: windows
    method: npm
    command: "npm install -g @qwen-code/qwen-code@latest"
    notes: "npm latest 0.19.6 declares Node.js >=22.0.0 and creates Windows command shims."
  - os: macos
    method: brew
    command: "brew install qwen-code"
    notes: "Homebrew formula stable was 0.19.5 on 2026-07-03; the installed keg on this host was older, 0.15.6."
  - os: linux
    method: brew
    command: "brew install qwen-code"
    notes: "Official README and Homebrew formula support Linux bottles."
subcommands:
  - name: "[query..]"
    description: "Default command. With no prompt it launches the interactive TUI; with a positional prompt or --prompt it runs a headless one-shot task."
    non_interactive: true
    notes: "Automation entry point. --prompt-interactive runs an initial prompt and then stays interactive."
  - name: "mcp"
    description: "Manage MCP servers."
    non_interactive: false
    notes: "Subcommands: add, remove, list, reconnect, approve, reject. list is scriptable but text-only; add/remove/approve/reject mutate settings or approval state."
  - name: "extensions"
    description: "Manage Qwen Code extensions."
    non_interactive: false
    notes: "Subcommands include install, uninstall, list, update, disable, enable, link, new, settings, and sources. install may require --consent to avoid a confirmation prompt."
  - name: "auth"
    description: "Removed legacy authentication command."
    non_interactive: true
    notes: "0.19.6 top-level help describes it as removed; official setup uses interactive /auth or settings/env/CLI flags."
  - name: "hooks"
    description: "Hook management placeholder."
    non_interactive: false
    notes: "Alias: hook. Help directs users to /hooks in interactive mode."
  - name: "channel"
    description: "Manage messaging channel integrations."
    non_interactive: false
    notes: "Channel flows can be long-running daemon or pairing flows."
  - name: "review"
    description: "Internal helpers used by the /review skill."
    non_interactive: true
    notes: "Subcommands handle PR worktree setup, context fetch, rule loading, presubmit checks, and cleanup."
  - name: "serve"
    description: "Run Qwen Code as a local HTTP daemon."
    non_interactive: true
    notes: "Long-running daemon; loopback is auth-free unless --require-auth is set. Non-loopback requires a bearer token."
  - name: "sessions"
    description: "Manage saved Qwen Code sessions."
    non_interactive: true
    notes: "sessions list supports JSON Lines output with --json."
cli_switches:
  - flag: --help
    value: ""
    scope: ["global"]
    default: "false"
    description: "Show help."
    example: "qwen --help"
    notes: "Short form: -h. 0.19.6 top-level help is compact and omits many default-command flags still present in the full parser."
  - flag: --version
    value: ""
    scope: ["global"]
    default: "false"
    description: "Show version number."
    example: "qwen --version"
    notes: "Short form: -v."
  - flag: --telemetry
    value: ""
    scope: ["global", "telemetry"]
    default: "settings/env dependent"
    description: "Enable telemetry."
    example: "qwen --telemetry \"summarize\""
    notes: "Deprecated in favor of telemetry.enabled in settings.json."
  - flag: --telemetry-target
    value: "local | gcp"
    scope: ["global", "telemetry"]
    default: "settings/env dependent"
    description: "Set telemetry target."
    example: "qwen --telemetry-target local"
    notes: "Deprecated in favor of telemetry.target in settings.json."
  - flag: --telemetry-otlp-endpoint
    value: "<URL>"
    scope: ["global", "telemetry"]
    default: "settings/env dependent"
    description: "Set OTLP endpoint."
    example: "qwen --telemetry-otlp-endpoint http://localhost:4317"
    notes: "Deprecated in favor of telemetry.otlpEndpoint in settings.json."
  - flag: --telemetry-otlp-protocol
    value: "grpc | http"
    scope: ["global", "telemetry"]
    default: "grpc"
    description: "Set OTLP protocol."
    example: "qwen --telemetry-otlp-protocol http"
    notes: "Deprecated in favor of telemetry.otlpProtocol in settings.json."
  - flag: --telemetry-log-prompts
    value: ""
    scope: ["global", "telemetry"]
    default: "settings/env dependent"
    description: "Enable telemetry prompt logging."
    example: "qwen --telemetry-log-prompts"
    notes: "Deprecated in favor of telemetry.logPrompts in settings.json."
  - flag: --telemetry-outfile
    value: "<PATH>"
    scope: ["global", "telemetry"]
    default: "settings/env dependent"
    description: "Write telemetry output to a file."
    example: "qwen --telemetry-outfile ./qwen-telemetry.jsonl"
    notes: "Deprecated in favor of telemetry.outfile in settings.json."
  - flag: --debug
    value: ""
    scope: ["global", "diagnostics"]
    default: "false"
    description: "Run in debug mode."
    example: "qwen --debug \"diagnose\""
    notes: "Short form: -d."
  - flag: --bare
    value: ""
    scope: ["global", "isolation"]
    default: "false"
    description: "Skip implicit startup auto-discovery and honor only explicit CLI inputs."
    example: "qwen --bare \"summarize this repo\""
    notes: "Useful for wrappers that want fewer config/context side effects."
  - flag: --safe-mode
    value: ""
    scope: ["global", "isolation"]
    default: "false"
    description: "Disable customizations such as context files, hooks, extensions, skills, and MCP servers."
    example: "qwen --safe-mode \"reproduce this bug\""
    notes: "Also settable with QWEN_CODE_SAFE_MODE=true."
  - flag: --proxy
    value: "<URL>"
    scope: ["global", "network"]
    default: "settings/env dependent"
    description: "Set proxy."
    example: "qwen --proxy http://localhost:7890"
    notes: "Deprecated in favor of proxy in settings.json; HTTPS_PROXY/HTTP_PROXY are also honored."
  - flag: --insecure
    value: ""
    scope: ["global", "network"]
    default: "false"
    description: "Skip TLS certificate verification for API connections."
    example: "qwen --insecure \"test local provider\""
    notes: "Equivalent to QWEN_TLS_INSECURE=1; affects API, OAuth, MCP, and child-process HTTPS."
  - flag: --chat-recording
    value: ""
    scope: ["global", "sessions"]
    default: "true"
    description: "Enable chat recording to disk."
    example: "qwen --no-chat-recording \"one-off\""
    notes: "Boolean yargs option supports --no-chat-recording. Disabling it prevents --continue/--resume."
  - flag: --model
    value: "<MODEL>"
    scope: ["default", "model_selection"]
    default: "auth/settings/env dependent"
    description: "Select model."
    example: "qwen --model qwen3-coder-plus \"inspect this repo\""
    notes: "Short form: -m."
  - flag: --prompt
    value: "<PROMPT>"
    scope: ["default", "non_interactive"]
    default: ""
    description: "Supply a prompt; appended to stdin content if stdin is provided."
    example: "qwen --prompt \"summarize\""
    notes: "Short form: -p. Deprecated in favor of positional prompt."
  - flag: --prompt-interactive
    value: "<PROMPT>"
    scope: ["default", "interactive"]
    default: ""
    description: "Execute the prompt and continue in interactive mode."
    example: "qwen --prompt-interactive \"start by reading README\""
    notes: "Short form: -i. Cannot be combined with --prompt or --json-schema."
  - flag: --system-prompt
    value: "<TEXT>"
    scope: ["default", "prompting"]
    default: ""
    description: "Override the main session system prompt for this run."
    example: "qwen \"review\" --system-prompt \"You are a strict reviewer.\""
    notes: "Existence recorded only; detailed semantics belong to the system-prompt research topic."
  - flag: --append-system-prompt
    value: "<TEXT>"
    scope: ["default", "prompting"]
    default: ""
    description: "Append instructions to the main session system prompt for this run."
    example: "qwen \"review\" --append-system-prompt \"Focus on regressions.\""
    notes: "Existence recorded only; detailed semantics belong to the system-prompt research topic."
  - flag: --sandbox
    value: ""
    scope: ["default", "sandbox"]
    default: "false"
    description: "Run in a sandbox."
    example: "qwen --sandbox \"run tests\""
    notes: "Short form: -s. QWEN_SANDBOX can override CLI/settings."
  - flag: --sandbox-image
    value: "<IMAGE>"
    scope: ["default", "sandbox"]
    default: "package/settings dependent"
    description: "Select sandbox image URI."
    example: "qwen --sandbox --sandbox-image ghcr.io/qwenlm/qwen-code:0.19.6"
    notes: "Deprecated in favor of tools.sandboxImage in settings.json."
  - flag: --yolo
    value: ""
    scope: ["default", "permissions"]
    default: "false"
    description: "Auto-approve all actions."
    example: "qwen --yolo \"apply the fix\""
    notes: "Short form: -y. Does not enable sandboxing."
  - flag: --approval-mode
    value: "plan | default | auto-edit | auto | yolo"
    scope: ["default", "permissions"]
    default: "default"
    description: "Set approval behavior for tool execution."
    example: "qwen --approval-mode plan \"make a plan\""
    notes: "Cannot be combined with --yolo. auto is present in the 0.19.6 source parser but omitted from compact top-level help."
  - flag: --acp
    value: ""
    scope: ["default", "protocol"]
    default: "false"
    description: "Start in Agent Client Protocol mode."
    example: "qwen --acp"
    notes: "Not compatible with --json-schema."
  - flag: --experimental-lsp
    value: ""
    scope: ["default", "lsp"]
    default: "false"
    description: "Enable experimental LSP code intelligence."
    example: "qwen --experimental-lsp"
    notes: "serve also has a scoped --experimental-lsp that forwards the opt-in to spawned sessions."
  - flag: --channel
    value: "VSCode | ACP | SDK | CI | desktop"
    scope: ["default", "integration"]
    default: ""
    description: "Set channel identifier."
    example: "qwen --channel CI \"run checks\""
    notes: "serve has a different repeatable --channel option for daemon-managed channel workers."
  - flag: --allowed-mcp-server-names
    value: "<NAME>[,<NAME>...]"
    scope: ["default", "mcp"]
    default: ""
    description: "Restrict allowed MCP server names."
    example: "qwen --allowed-mcp-server-names filesystem,github \"work\""
    notes: "Can be repeated or comma-separated."
  - flag: --mcp-config
    value: "<JSON_OR_PATH>"
    scope: ["default", "mcp"]
    default: ""
    description: "Inject MCP server configuration as inline JSON or a JSON file path."
    example: "qwen --mcp-config ./mcp.json \"use tools\""
    notes: "Expected shape includes {\"mcpServers\": {...}}."
  - flag: --allowed-tools
    value: "<TOOL>[,<TOOL>...]"
    scope: ["default", "permissions"]
    default: ""
    description: "Allow tools to run without confirmation."
    example: "qwen --allowed-tools read_file,grep \"inspect\""
    notes: "Can be repeated or comma-separated."
  - flag: --extensions
    value: "<EXT>[,<EXT>...]"
    scope: ["default", "extensions"]
    default: "all extensions"
    description: "Select extensions to use for the session."
    example: "qwen --extensions my-extension \"use it\""
    notes: "Short form: -e."
  - flag: --list-extensions
    value: ""
    scope: ["default", "extensions"]
    default: "false"
    description: "List available extensions and exit."
    example: "qwen --list-extensions"
    notes: "Short form: -l. No JSON mode found."
  - flag: --include-directories
    value: "<PATH>[,<PATH>...]"
    scope: ["default", "context"]
    default: ""
    description: "Include additional directories in workspace context."
    example: "qwen --include-directories ../lib,../cli \"inspect both\""
    notes: "Alias: --add-dir."
  - flag: --openai-logging
    value: ""
    scope: ["default", "diagnostics"]
    default: "settings dependent"
    description: "Enable OpenAI API call logging."
    example: "qwen --openai-logging \"debug provider\""
    notes: "General logging switch; detailed logging belongs to agent-logging."
  - flag: --openai-logging-dir
    value: "<PATH>"
    scope: ["default", "diagnostics"]
    default: "settings dependent"
    description: "Set OpenAI API log directory."
    example: "qwen --openai-logging --openai-logging-dir ~/qwen-logs"
    notes: "Overrides settings files."
  - flag: --openai-api-key
    value: "<KEY>"
    scope: ["default", "auth"]
    default: "env/settings dependent"
    description: "Set OpenAI-compatible API key."
    example: "qwen --openai-api-key \"$OPENAI_API_KEY\" \"run\""
    notes: "Provider endpoint detail; included because it is public CLI surface."
  - flag: --openai-base-url
    value: "<URL>"
    scope: ["default", "auth"]
    default: "env/settings dependent"
    description: "Set OpenAI-compatible base URL."
    example: "qwen --openai-base-url http://localhost:11434/v1 --model qwen3-coder"
    notes: "Provider endpoint detail; included because it is public CLI surface."
  - flag: --screen-reader
    value: ""
    scope: ["default", "accessibility"]
    default: "false"
    description: "Enable screen reader mode."
    example: "qwen --screen-reader"
    notes: "Adjusts TUI behavior."
  - flag: --input-format
    value: "text | stream-json"
    scope: ["default", "io"]
    default: "text"
    description: "Select stdin input protocol."
    example: "qwen --input-format stream-json --output-format stream-json"
    notes: "stream-json input requires --output-format stream-json."
  - flag: --output-format
    value: "text | json | stream-json"
    scope: ["default", "io"]
    default: "text"
    description: "Select CLI output format."
    example: "qwen --output-format stream-json \"run task\""
    notes: "Short form: -o. json buffers; stream-json emits line-delimited JSON events."
  - flag: --include-partial-messages
    value: ""
    scope: ["default", "io"]
    default: "false"
    description: "Include partial assistant messages in stream-json output."
    example: "qwen -o stream-json --include-partial-messages \"write\""
    notes: "Requires --output-format stream-json."
  - flag: --json-fd
    value: "<FD>"
    scope: ["default", "dual_output"]
    default: ""
    description: "Write structured JSON event output to a supplied file descriptor while the TUI renders normally."
    example: "spawn qwen with fd 3 and pass --json-fd 3"
    notes: "Mutually exclusive with --json-file; caller must configure spawn stdio."
  - flag: --json-file
    value: "<PATH>"
    scope: ["default", "dual_output"]
    default: ""
    description: "Write structured JSON event output to a file, FIFO, or /dev/fd/N."
    example: "qwen --json-file ./events.jsonl"
    notes: "Mutually exclusive with --json-fd."
  - flag: --json-schema
    value: "<JSON_OR_@PATH>"
    scope: ["default", "structured_output"]
    default: ""
    description: "Require final headless output to conform to a JSON Schema."
    example: "qwen \"summarize\" --json-schema @./schema.json"
    notes: "Headless only; rejected with --prompt-interactive, --input-format stream-json, --acp, or no prompt/stdin."
  - flag: --input-file
    value: "<PATH>"
    scope: ["default", "dual_output"]
    default: ""
    description: "Read remote JSONL input commands from a file for bidirectional sync."
    example: "qwen --input-file ./commands.jsonl"
    notes: "The TUI watches the file and processes external commands."
  - flag: --continue
    value: ""
    scope: ["default", "sessions"]
    default: "false"
    description: "Resume the most recent session for the current project."
    example: "qwen --continue \"next\""
    notes: "Short form: -c. Cannot be combined with --resume or --session-id."
  - flag: --resume
    value: "<SESSION_ID>"
    scope: ["default", "sessions"]
    default: ""
    description: "Resume a specific session by ID, or show a picker when used without an ID."
    example: "qwen --resume 123e4567-e89b-12d3-a456-426614174000 \"continue\""
    notes: "Short form: -r. Picker requires interaction."
  - flag: --session-id
    value: "<UUID>"
    scope: ["default", "sessions"]
    default: "generated"
    description: "Specify a session ID for a new run."
    example: "qwen --session-id 123e4567-e89b-12d3-a456-426614174000 \"run\""
    notes: "Cannot be combined with --continue or --resume."
  - flag: --fork-session
    value: ""
    scope: ["default", "sessions"]
    default: "false"
    description: "Create a forked session from a resumed session."
    example: "qwen --continue --fork-session \"try alternate fix\""
    notes: "Requires --continue or --resume."
  - flag: --worktree
    value: "[SLUG_OR_PR]"
    scope: ["default", "git"]
    default: ""
    description: "Start the session inside a git worktree under <repoRoot>/.qwen/worktrees/<slug>/."
    example: "qwen --worktree my-feature \"implement\""
    notes: "Accepts a slug, bare flag, #123, or a GitHub pull-request URL. Exit dialog may prompt."
  - flag: --max-session-turns
    value: "<N>"
    scope: ["default", "limits"]
    default: "settings dependent or unlimited"
    description: "Limit session turns."
    example: "qwen --max-session-turns 8 \"run bounded task\""
    notes: "Applies to structured-output terminal turn too."
  - flag: --max-wall-time
    value: "<DURATION>"
    scope: ["default", "limits"]
    default: "settings dependent or unlimited"
    description: "Set a wall-clock budget for headless or unattended runs."
    example: "qwen --max-wall-time 10m \"run bounded task\""
    notes: "Accepts seconds or duration strings; abort exits 55."
  - flag: --max-tool-calls
    value: "<N>"
    scope: ["default", "limits"]
    default: "-1"
    description: "Limit cumulative tool calls for a run."
    example: "qwen --max-tool-calls 20 \"inspect\""
    notes: "-1/unset means unlimited; 0 means no tool calls; abort exits 55."
  - flag: --max-subagent-depth
    value: "<N>"
    scope: ["default", "limits"]
    default: "5"
    description: "Limit sub-agent nesting depth."
    example: "qwen --max-subagent-depth 1 \"do not nest subagents\""
    notes: "1 keeps subagents available but disables nesting."
  - flag: --core-tools
    value: "<TOOL>[,<TOOL>...]"
    scope: ["default", "tools"]
    default: "settings dependent"
    description: "Restrict registered core tools."
    example: "qwen --core-tools read_file,grep"
    notes: "Whitelist semantics; not the same as auto-approval."
  - flag: --exclude-tools
    value: "<TOOL>[,<TOOL>...]"
    scope: ["default", "tools"]
    default: "settings dependent"
    description: "Exclude tools."
    example: "qwen --exclude-tools shell,write_file \"inspect only\""
    notes: "Can be repeated or comma-separated."
  - flag: --disabled-slash-commands
    value: "<NAME>[,<NAME>...]"
    scope: ["default", "slash_commands"]
    default: "settings/env dependent"
    description: "Hide or disable slash commands."
    example: "qwen --disabled-slash-commands auth,mcp,extensions"
    notes: "Merged with settings and QWEN_DISABLED_SLASH_COMMANDS."
  - flag: --auth-type
    value: "openai | anthropic | qwen-oauth | gemini | vertex-ai"
    scope: ["default", "auth"]
    default: "env/settings dependent"
    description: "Select authentication/provider protocol."
    example: "qwen --auth-type openai --model qwen3-coder-plus"
    notes: "Model endpoint semantics belong to model-config."
  - flag: --scope
    value: "user | project"
    scope: ["mcp add", "extensions install", "extensions enable", "extensions disable"]
    default: "command dependent"
    description: "Select user/project scope for MCP or extension changes."
    example: "qwen mcp add --scope project local python -m server"
    notes: "mcp add short form: -s. extensions install also accepts workspace as an alias for project."
  - flag: --transport
    value: "stdio | sse | http"
    scope: ["mcp add"]
    default: "auto-detected"
    description: "Select MCP transport."
    example: "qwen mcp add --transport http my-server http://localhost:3000/mcp"
    notes: "Short form: -t."
  - flag: --env
    value: "KEY=value"
    scope: ["mcp add"]
    default: ""
    description: "Add environment variables for an MCP server."
    example: "qwen mcp add -e TOKEN=abc local node server.js"
    notes: "Short form: -e."
  - flag: --header
    value: "NAME: value"
    scope: ["mcp add"]
    default: ""
    description: "Add HTTP headers for SSE/HTTP MCP transports."
    example: "qwen mcp add -H \"X-Api-Key: abc\" --transport http remote http://localhost:3000/mcp"
    notes: "Short form: -H."
  - flag: --timeout
    value: "<MS>"
    scope: ["mcp add"]
    default: ""
    description: "Set MCP server connection timeout in milliseconds."
    example: "qwen mcp add --timeout 30000 local python -m server"
    notes: ""
  - flag: --trust
    value: ""
    scope: ["mcp add"]
    default: "false"
    description: "Trust an MCP server and bypass tool-call confirmation prompts for it."
    example: "qwen mcp add --trust local python -m server"
    notes: "Mutates persistent trust/permission behavior."
  - flag: --description
    value: "<TEXT>"
    scope: ["mcp add"]
    default: ""
    description: "Set MCP server description."
    example: "qwen mcp add --description \"Local tools\" local python -m server"
    notes: ""
  - flag: --include-tools
    value: "<TOOL>[,<TOOL>...]"
    scope: ["mcp add"]
    default: "all tools"
    description: "Include only selected MCP tools."
    example: "qwen mcp add --include-tools search,fetch remote http://localhost:3000/mcp"
    notes: ""
  - flag: --oauth-client-id
    value: "<ID>"
    scope: ["mcp add"]
    default: ""
    description: "Set OAuth client ID for MCP authentication."
    example: "qwen mcp add --transport http --oauth-client-id id remote http://localhost:3000/mcp"
    notes: "Only for sse/http transports."
  - flag: --oauth-client-secret
    value: "<SECRET>"
    scope: ["mcp add"]
    default: ""
    description: "Set OAuth client secret for MCP authentication."
    example: "qwen mcp add --transport http --oauth-client-secret secret remote http://localhost:3000/mcp"
    notes: "Only for sse/http transports."
  - flag: --oauth-redirect-uri
    value: "<URI>"
    scope: ["mcp add"]
    default: "localhost callback"
    description: "Set OAuth redirect URI for MCP authentication."
    example: "qwen mcp add --transport sse --oauth-redirect-uri https://example.com/oauth/callback remote https://example.com/sse"
    notes: ""
  - flag: --oauth-authorization-url
    value: "<URL>"
    scope: ["mcp add"]
    default: ""
    description: "Set OAuth authorization URL for MCP authentication."
    example: "qwen mcp add --transport http --oauth-authorization-url https://provider.example.com/authorize remote http://localhost:3000/mcp"
    notes: ""
  - flag: --oauth-token-url
    value: "<URL>"
    scope: ["mcp add"]
    default: ""
    description: "Set OAuth token URL for MCP authentication."
    example: "qwen mcp add --transport http --oauth-token-url https://provider.example.com/token remote http://localhost:3000/mcp"
    notes: ""
  - flag: --oauth-scopes
    value: "<SCOPE>[,<SCOPE>...]"
    scope: ["mcp add"]
    default: ""
    description: "Set OAuth scopes for MCP authentication."
    example: "qwen mcp add --transport http --oauth-scopes scope1,scope2 remote http://localhost:3000/mcp"
    notes: ""
  - flag: --all
    value: ""
    scope: ["mcp approve", "mcp reject", "extensions update"]
    default: "false"
    description: "Apply the operation to all matching items."
    example: "qwen mcp approve --all"
    notes: "For extensions update, updates all extensions."
  - flag: --json
    value: ""
    scope: ["sessions list"]
    default: "false"
    description: "Output sessions as JSON Lines."
    example: "qwen sessions list --json"
    notes: "Primary machine-readable state command found."
  - flag: --limit
    value: "<N>"
    scope: ["sessions list"]
    default: "20"
    description: "Limit sessions shown."
    example: "qwen sessions list --json --limit 100"
    notes: ""
  - flag: --ref
    value: "<GIT_REF>"
    scope: ["extensions install"]
    default: ""
    description: "Install an extension from a specific git ref."
    example: "qwen extensions install https://github.com/org/ext --ref main --consent"
    notes: "Not applicable to npm, archive URL, or marketplace extensions."
  - flag: --auto-update
    value: ""
    scope: ["extensions install"]
    default: "false"
    description: "Enable auto-update for an extension."
    example: "qwen extensions install https://github.com/org/ext --auto-update --consent"
    notes: "Not applicable to marketplace extensions."
  - flag: --pre-release
    value: ""
    scope: ["extensions install"]
    default: "false"
    description: "Allow pre-release extension versions."
    example: "qwen extensions install @scope/ext --pre-release --consent"
    notes: ""
  - flag: --registry
    value: "<URL>"
    scope: ["extensions install"]
    default: "npm default"
    description: "Use a custom npm registry for npm extensions."
    example: "qwen extensions install @scope/ext --registry https://registry.npmjs.org --consent"
    notes: "Only for npm extension sources."
  - flag: --consent
    value: ""
    scope: ["extensions install"]
    default: "false"
    description: "Acknowledge extension security risks and skip confirmation prompt."
    example: "qwen extensions install https://github.com/org/ext --consent"
    notes: "Wrapper-relevant for non-interactive installs."
  - flag: --port
    value: "<PORT>"
    scope: ["serve"]
    default: "4170"
    description: "Bind daemon TCP port."
    example: "qwen serve --port 4170"
    notes: "Use 0 for an OS-assigned ephemeral port."
  - flag: --hostname
    value: "<HOST>"
    scope: ["serve"]
    default: "127.0.0.1"
    description: "Bind daemon interface."
    example: "qwen serve --hostname 127.0.0.1"
    notes: "Non-loopback requires token."
  - flag: --token
    value: "<TOKEN>"
    scope: ["serve"]
    default: "QWEN_SERVER_TOKEN or none on loopback"
    description: "Set bearer token for the daemon."
    example: "qwen serve --token \"$QWEN_SERVER_TOKEN\""
    notes: "Visible in process argv; prefer env on shared hosts."
  - flag: --require-auth
    value: ""
    scope: ["serve"]
    default: "false"
    description: "Require bearer auth even on loopback."
    example: "qwen serve --require-auth --token \"$QWEN_SERVER_TOKEN\""
    notes: "Hardens shared dev hosts and CI runners."
  - flag: --workspace
    value: "<PATH>"
    scope: ["serve"]
    default: "process.cwd()"
    description: "Bind daemon to an absolute workspace path."
    example: "qwen serve --workspace /repo"
    notes: "Mismatched POST /session cwd returns workspace_mismatch."
  - flag: --web
    value: ""
    scope: ["serve"]
    default: "true"
    description: "Serve Web Shell UI."
    example: "qwen serve --no-web"
    notes: "Boolean option supports --no-web."
  - flag: --open
    value: ""
    scope: ["serve"]
    default: "false"
    description: "Open Web Shell in a browser once listening."
    example: "qwen serve --open"
    notes: "With token configured, launch URL can expose token in process list."
  - flag: --http-bridge
    value: ""
    scope: ["serve"]
    default: "true"
    description: "Use ACP child bridge mode."
    example: "qwen serve --http-bridge"
    notes: "--no-http-bridge is not implemented in 0.19.6 and falls back to bridge mode."
  - flag: --rate-limit
    value: ""
    scope: ["serve"]
    default: "false"
    description: "Enable per-tier HTTP rate limiting."
    example: "qwen serve --rate-limit"
    notes: "Related numeric flags: --rate-limit-prompt, --rate-limit-mutation, --rate-limit-read, --rate-limit-window-ms."
config_paths:
  - os: macos
    scope: user
    path: "~/.qwen/settings.json"
    format: json
    notes: "Primary user settings file; QWEN_HOME changes the global config directory."
  - os: linux
    scope: user
    path: "~/.qwen/settings.json"
    format: json
    notes: "Primary user settings file; QWEN_HOME changes the global config directory."
  - os: windows
    scope: user
    path: "%USERPROFILE%\\.qwen\\settings.json"
    format: json
    notes: "Primary user settings file inferred from Node homedir plus .qwen; QWEN_HOME changes the global config directory."
  - os: macos
    scope: repo
    path: ".qwen/settings.json"
    format: json
    notes: "Project settings file in project root; ignored when workspace is untrusted."
  - os: linux
    scope: repo
    path: ".qwen/settings.json"
    format: json
    notes: "Project settings file in project root; ignored when workspace is untrusted."
  - os: windows
    scope: repo
    path: ".qwen\\settings.json"
    format: json
    notes: "Project settings file in project root; ignored when workspace is untrusted."
  - os: macos
    scope: system
    path: "/Library/Application Support/QwenCode/settings.json"
    format: json
    notes: "System override settings; QWEN_CODE_SYSTEM_SETTINGS_PATH can override."
  - os: linux
    scope: system
    path: "/etc/qwen-code/settings.json"
    format: json
    notes: "System override settings; QWEN_CODE_SYSTEM_SETTINGS_PATH can override."
  - os: windows
    scope: system
    path: "C:\\ProgramData\\qwen-code\\settings.json"
    format: json
    notes: "System override settings; QWEN_CODE_SYSTEM_SETTINGS_PATH can override."
  - os: macos
    scope: system
    path: "/Library/Application Support/QwenCode/system-defaults.json"
    format: json
    notes: "System defaults; QWEN_CODE_SYSTEM_DEFAULTS_PATH can override."
  - os: linux
    scope: system
    path: "/etc/qwen-code/system-defaults.json"
    format: json
    notes: "System defaults; QWEN_CODE_SYSTEM_DEFAULTS_PATH can override."
  - os: windows
    scope: system
    path: "C:\\ProgramData\\qwen-code\\system-defaults.json"
    format: json
    notes: "System defaults; QWEN_CODE_SYSTEM_DEFAULTS_PATH can override."
  - os: macos
    scope: user
    path: "~/.qwen/installation_id"
    format: text
    notes: "Written on first run. Observed on this host."
  - os: linux
    scope: user
    path: "~/.qwen/installation_id"
    format: text
    notes: "Written on first run."
  - os: windows
    scope: user
    path: "%USERPROFILE%\\.qwen\\installation_id"
    format: text
    notes: "Written on first run."
  - os: macos
    scope: user
    path: "~/.qwen/debug/*.txt"
    format: text
    notes: "Runtime debug logs; QWEN_RUNTIME_DIR can relocate runtime output."
  - os: linux
    scope: user
    path: "~/.qwen/debug/*.txt"
    format: text
    notes: "Runtime debug logs; QWEN_RUNTIME_DIR can relocate runtime output."
  - os: windows
    scope: user
    path: "%USERPROFILE%\\.qwen\\debug\\*.txt"
    format: text
    notes: "Runtime debug logs; QWEN_RUNTIME_DIR can relocate runtime output."
  - os: macos
    scope: repo
    path: "QWEN.md"
    format: text
    notes: "Default hierarchical context file; filename can be changed by context.fileName."
  - os: linux
    scope: repo
    path: "QWEN.md"
    format: text
    notes: "Default hierarchical context file; filename can be changed by context.fileName."
  - os: windows
    scope: repo
    path: "QWEN.md"
    format: text
    notes: "Default hierarchical context file; filename can be changed by context.fileName."
  - os: macos
    scope: repo
    path: ".mcp.json"
    format: json
    notes: "Project MCP server file; gated servers require approval state."
  - os: linux
    scope: repo
    path: ".mcp.json"
    format: json
    notes: "Project MCP server file; gated servers require approval state."
  - os: windows
    scope: repo
    path: ".mcp.json"
    format: json
    notes: "Project MCP server file; gated servers require approval state."
env_vars:
  - name: QWEN_HOME
    effect: "Overrides the global configuration directory, defaulting to ~/.qwen."
  - name: QWEN_RUNTIME_DIR
    effect: "Overrides runtime output directory for temp files, debug logs, session data, todos, and similar runtime artifacts; config files remain under QWEN_HOME/default global dir."
  - name: QWEN_CODE_SYSTEM_SETTINGS_PATH
    effect: "Overrides the system settings file path."
  - name: QWEN_CODE_SYSTEM_DEFAULTS_PATH
    effect: "Overrides the system defaults file path."
  - name: QWEN_SANDBOX
    effect: "Enables or selects sandbox provider; accepts true, false, docker, podman, sandbox-exec, or a custom command."
  - name: QWEN_SANDBOX_IMAGE
    effect: "Overrides sandbox image selection."
  - name: SEATBELT_PROFILE
    effect: "Selects the macOS sandbox-exec Seatbelt profile."
  - name: BUILD_SANDBOX
    effect: "Builds a custom sandbox image from .qwen/sandbox.Dockerfile when set."
  - name: SANDBOX_FLAGS
    effect: "Injects extra flags into docker or podman sandbox commands."
  - name: QWEN_CODE_SAFE_MODE
    effect: "Enables safe mode when CLI flags cannot be passed."
  - name: QWEN_CODE_SUPPRESS_YOLO_WARNING
    effect: "Suppresses the warning emitted for headless YOLO runs without sandboxing."
  - name: QWEN_CODE_UNATTENDED_RETRY
    effect: "Enables persistent retry for transient 429/529 API capacity errors, with stderr heartbeat keepalives."
  - name: QWEN_DISABLED_SLASH_COMMANDS
    effect: "Adds comma-separated slash commands to hide or disable."
  - name: QWEN_CODE_LANG
    effect: "Overrides UI language."
  - name: QWEN_TLS_INSECURE
    effect: "Disables TLS certificate verification when set to 1."
  - name: NO_COLOR
    effect: "Disables color output and theme configuration surfaces."
  - name: QWEN_DISABLE_HYPERLINKS
    effect: "Disables OSC 8 clickable hyperlinks."
  - name: DEBUG
    effect: "Enables debug mode when true/1; excluded from project .env by default."
  - name: DEBUG_MODE
    effect: "Enables debug mode when true/1; excluded from project .env by default."
  - name: HTTPS_PROXY
    effect: "Proxy fallback when proxy is not set by CLI/settings."
  - name: HTTP_PROXY
    effect: "Proxy fallback when proxy is not set by CLI/settings."
  - name: QWEN_SERVER_TOKEN
    effect: "Bearer token source for qwen serve and SDK clients."
  - name: QWEN_SERVE_PROMPT_DEADLINE_MS
    effect: "Default server-side deadline for qwen serve prompt requests."
  - name: QWEN_SERVE_WRITER_IDLE_TIMEOUT_MS
    effect: "Idle deadline for qwen serve SSE writers."
  - name: QWEN_SERVE_RATE_LIMIT
    effect: "Enables qwen serve HTTP rate limiting when set to 1 or true."
  - name: QWEN_SERVE_RATE_LIMIT_PROMPT
    effect: "Sets qwen serve prompt request rate limit when rate limiting is enabled."
  - name: QWEN_SERVE_RATE_LIMIT_MUTATION
    effect: "Sets qwen serve mutation request rate limit when rate limiting is enabled."
  - name: QWEN_SERVE_RATE_LIMIT_READ
    effect: "Sets qwen serve read request rate limit when rate limiting is enabled."
  - name: QWEN_SERVE_RATE_LIMIT_WINDOW_MS
    effect: "Sets qwen serve rate-limit window in milliseconds."
  - name: QWEN_SERVE_DEBUG
    effect: "Enables extra qwen serve bridge debug breadcrumbs."
machine_introspection:
  - command: "qwen sessions list --json --limit 100"
    purpose: other
    machine_readable: true
    output_format: jsonl
    useful_for_codegen: false
    notes: "Lists saved sessions as JSON Lines. This is the clearest direct machine-readable CLI state command found."
  - command: "qwen --list-extensions"
    purpose: plugins
    machine_readable: false
    output_format: text
    useful_for_codegen: false
    notes: "Lists extensions and exits, but no JSON mode was found."
  - command: "qwen mcp list"
    purpose: mcp
    machine_readable: false
    output_format: text
    useful_for_codegen: false
    notes: "Lists configured MCP servers. No JSON mode was found in help/source."
  - command: "qwen serve + GET /capabilities"
    purpose: capabilities
    machine_readable: true
    output_format: json
    useful_for_codegen: true
    notes: "Requires starting the HTTP daemon and calling its API; not a fire-and-exit CLI probe."
wrapper_notes:
  - "Use positional prompts for new headless wrapper calls; --prompt still works but is documented as deprecated."
  - "Prefer --output-format stream-json for streaming wrappers. --output-format json buffers a JSON array until completion."
  - "--json-schema is useful for strict final output, but is rejected with --prompt-interactive, --input-format stream-json, --acp, or no prompt/stdin."
  - "--json-fd and --json-file provide dual structured event output while the normal UI remains on stdout; --json-fd requires explicit fd plumbing in the spawn call."
  - "--bare and --safe-mode are useful wrapper isolation controls. safe-mode disables settings-sourced customizations such as context files, hooks, extensions, skills, and MCP."
  - "--yolo or --approval-mode=yolo does not enable sandboxing. Headless YOLO without sandbox prints a stderr warning unless QWEN_CODE_SUPPRESS_YOLO_WARNING=1."
  - "Untrusted folders ignore project .qwen/settings.json and force risky approval modes back toward default behavior."
  - "--worktree can prompt on exit to keep or remove the worktree; avoid it for unattended wrappers unless that behavior is acceptable."
  - "auth is a removed legacy command in 0.19.6. Use interactive /auth, env vars, CLI OpenAI-compatible flags, or settings.json for scripted setup."
  - "Qwen OAuth cannot be fully configured by env vars alone; CI/headless should use API-key auth such as OpenAI-compatible settings."
  - "The latest npm package requires Node.js >=22.0.0; older docs/installations may still mention older Node versions."
  - "Local PATH inspection on this host found qwen 0.15.6, older than npm latest 0.19.6 and Homebrew stable 0.19.5, so wrappers should not infer current surface from an arbitrary installed binary."
changes:
  - "Updated latest npm version from 0.19.5 to 0.19.6."
  - "Recorded that local Homebrew-installed qwen is 0.15.6 while Homebrew stable is 0.19.5 and npm latest is 0.19.6."
  - "Reconciled compact 0.19.6 top-level help with the full packaged command parser, including flags omitted from top-level help."
  - "Added schema-conformant per-OS binary, install, and config records instead of os: all records."
  - "Added newer wrapper-facing flags including --max-subagent-depth and qwen serve rate-limit controls."
requires_claudine_update: true
reason: "Claudine provider metadata should account for npm latest 0.19.6, the observed version skew with local/Homebrew installs, compact help that omits parser-supported flags, per-OS config path normalization, and newer wrapper-facing flags such as --max-subagent-depth and qwen serve rate-limit controls."
---

# Qwen Code CLI

## Overview

Qwen Code is the Qwen/Alibaba open-source terminal coding agent. It is shipped from the [QwenLM/qwen-code](https://github.com/QwenLM/qwen-code) repository as the `@qwen-code/qwen-code` npm package, standalone installers, and a Homebrew formula. The primary command users type is `qwen`.

The latest upstream version I verified on 2026-07-03 is `0.19.6`, from `npm view @qwen-code/qwen-code version` and `npx --yes @qwen-code/qwen-code@0.19.6 --version`. The locally installed `qwen` on PATH is older: `/opt/homebrew/bin/qwen` reports `0.15.6`. Homebrew's formula page and `brew info --json=v2 qwen-code` reported stable `0.19.5`. This version skew matters: the document uses npm `0.19.6` for current public surface, while calling out local negative/older observations where relevant.

Primary URLs:

- Homepage: [https://qwen.ai/qwencode](https://qwen.ai/qwencode)
- Repository: [https://github.com/QwenLM/qwen-code](https://github.com/QwenLM/qwen-code)
- General docs: [https://qwenlm.github.io/qwen-code-docs/](https://qwenlm.github.io/qwen-code-docs/)
- CLI/user overview: [https://qwenlm.github.io/qwen-code-docs/en/users/overview/](https://qwenlm.github.io/qwen-code-docs/en/users/overview/)

## Installation and Binaries

The public executable name is `qwen`. The npm package declares `bin: { "qwen": "cli-entry.js" }`. On this macOS host, Homebrew installed `/opt/homebrew/bin/qwen` as a symlink into the `qwen-code` Cellar package.

Official install commands:

| OS | Method | Command |
| --- | --- | --- |
| macOS | standalone | `curl -fsSL https://qwen-code-assets.oss-cn-hangzhou.aliyuncs.com/installation/install-qwen-standalone.sh \| bash` |
| Linux | standalone | `curl -fsSL https://qwen-code-assets.oss-cn-hangzhou.aliyuncs.com/installation/install-qwen-standalone.sh \| bash` |
| Windows | standalone | `irm https://qwen-code-assets.oss-cn-hangzhou.aliyuncs.com/installation/install-qwen-standalone.ps1 \| iex` |
| macOS/Linux/Windows | npm | `npm install -g @qwen-code/qwen-code@latest` |
| macOS/Linux | Homebrew | `brew install qwen-code` |

npm latest `0.19.6` requires Node.js `>=22.0.0`. Windows npm installs normally expose `qwen.cmd` and `qwen.ps1` shims in addition to the command name used in docs, but I did not inspect a Windows standalone install directly.

## Subcommands

| Command | Description | Non-interactive fit |
| --- | --- | --- |
| `qwen [query..]` | Default command. No prompt launches the TUI; a positional prompt or `--prompt` runs a one-shot headless task. | Yes, with prompt/stdin and noninteractive flags. |
| `qwen mcp` | Manage MCP servers: `add`, `remove`, `list`, `reconnect`, `approve`, `reject`. | Mixed. `list` is scriptable but text-only; mutation/approval commands change persistent state. |
| `qwen extensions <command>` | Manage extensions: `install`, `uninstall`, `list`, `update`, `disable`, `enable`, `link`, `new`, `settings`, `sources`. | Mixed. `install` needs `--consent` to avoid a confirmation prompt. |
| `qwen auth` | Removed legacy command. | Only prints migration guidance; real auth setup is interactive `/auth` or config/env. |
| `qwen hooks` / `qwen hook` | Hook management placeholder. | No useful non-interactive management observed; directs users to `/hooks`. |
| `qwen channel <command>` | Manage messaging channels. | Often daemon/pairing oriented; assume interactive or long-running unless proven otherwise. |
| `qwen review <command>` | Internal helpers for the `/review` skill. | Yes for helper automation, but not a general wrapper entry point. |
| `qwen serve` | Run a local HTTP daemon. | Long-running non-TTY daemon. |
| `qwen sessions <command>` | Manage saved sessions. | Yes; `sessions list --json` is JSON Lines. |

ACP is a mode flag (`qwen --acp`), not a top-level subcommand.

## CLI Switch Inventory

The full structured switch inventory is in frontmatter. Highlights for wrappers:

- Headless execution: positional prompt, `--prompt`, `--output-format`, `--input-format`, `--include-partial-messages`, `--json-schema`.
- Structured streaming: `--output-format stream-json`; JSON mode buffers until completion.
- Dual output: `--json-fd` and `--json-file` write structured events while the normal UI remains on stdout.
- Isolation: `--bare`, `--safe-mode`, `--sandbox`, `--approval-mode`, `--yolo`, `--allowed-tools`, `--exclude-tools`, `--core-tools`.
- State: `--continue`, `--resume`, `--session-id`, `--fork-session`, `--chat-recording`.
- Limits: `--max-session-turns`, `--max-wall-time`, `--max-tool-calls`, `--max-subagent-depth`.
- System-prompt delivery flags exist as `--system-prompt <TEXT>` and `--append-system-prompt <TEXT>`, for example `qwen "review" --append-system-prompt "Focus on regressions."`. Their replace-vs-append semantics are intentionally left to the sibling `system-prompt` topic.

Help mismatch: `qwen --help` in npm `0.19.6` is compact and omits many parser-supported default-command flags. I trusted the unpacked `0.19.6` command parser plus scoped subcommand help over the compact top-level help, because the source defines and validates those flags and scoped help confirms several of them.

## Configuration Discovery

Qwen applies configuration in layers: defaults, system defaults, user settings, project settings, system override settings, environment variables, then command-line arguments. Persistent settings are JSON.

Documented settings files:

| Scope | macOS | Linux | Windows |
| --- | --- | --- | --- |
| System defaults | `/Library/Application Support/QwenCode/system-defaults.json` | `/etc/qwen-code/system-defaults.json` | `C:\ProgramData\qwen-code\system-defaults.json` |
| User | `~/.qwen/settings.json` | `~/.qwen/settings.json` | `%USERPROFILE%\.qwen\settings.json` |
| Project | `.qwen/settings.json` | `.qwen/settings.json` | `.qwen\settings.json` |
| System override | `/Library/Application Support/QwenCode/settings.json` | `/etc/qwen-code/settings.json` | `C:\ProgramData\qwen-code\settings.json` |

`QWEN_HOME` relocates the global `.qwen` directory. `QWEN_RUNTIME_DIR` relocates runtime output such as debug logs and session data, not config files. `QWEN_CODE_SYSTEM_SETTINGS_PATH` and `QWEN_CODE_SYSTEM_DEFAULTS_PATH` override the system paths.

Observed local side effects: the installed CLI created `~/.qwen/installation_id`, `~/.qwen/output-language.md`, `~/.qwen/skills/`, and debug logs under a `.qwen/debug/` runtime tree. Project `.qwen/settings.json` is ignored in untrusted workspaces. Project MCP can also come from `.mcp.json`; gated MCP servers require approval/rejection state.

## Environment Variables

General wrapper-relevant variables are captured in frontmatter. This topic intentionally does not exhaustively duplicate model-provider secrets and endpoints, permission-policy specifics, MCP details, logging-only variables, or streaming-only variables owned by narrower topics.

The most important general variables are `QWEN_HOME`, `QWEN_RUNTIME_DIR`, `QWEN_CODE_SAFE_MODE`, `QWEN_SANDBOX`, `QWEN_TLS_INSECURE`, `QWEN_DISABLED_SLASH_COMMANDS`, `NO_COLOR`, `HTTP_PROXY`/`HTTPS_PROXY`, `QWEN_SERVER_TOKEN`, and the `QWEN_SERVE_*` daemon tuning variables.

## Machine Introspection

Useful machine-readable CLI introspection is limited:

| Command | Format | Usefulness |
| --- | --- | --- |
| `qwen sessions list --json --limit 100` | JSON Lines | Useful for session reports; not codegen ground truth. |
| `qwen --list-extensions` | Text | Lists extensions but lacks JSON output. |
| `qwen mcp list` | Text | Lists MCP servers but lacks JSON output. |
| `qwen serve` plus HTTP APIs such as `/capabilities` | JSON | Potentially useful for codegen/capability reports, but requires launching a daemon. |

`qwen --version` and help output were used as evidence but are not durable provider-state introspection commands.

## Wrapper Notes

Use positional prompts for new headless wrapper calls; `--prompt` still works but is deprecated. Use `--output-format stream-json` for streaming wrappers. Use `--output-format json` only when buffered output is acceptable.

`--json-schema` can enforce strict final output but conflicts with `--prompt-interactive`, `--input-format stream-json`, `--acp`, and no-prompt/no-stdin invocations. `--json-fd` requires fd setup by the parent process.

For isolation, prefer `--bare` or `--safe-mode` when appropriate. `--yolo` and `--approval-mode=yolo` do not enable sandboxing; Qwen prints a warning to stderr in headless YOLO without sandbox unless `QWEN_CODE_SUPPRESS_YOLO_WARNING=1`.

Avoid `--worktree` in unattended wrappers unless interactive exit handling is acceptable. Treat `qwen auth` as removed; scripted auth should use settings/env/CLI provider flags, while Qwen OAuth remains an interactive `/auth` flow.

## Changelog

- 2026-07-03: Updated npm latest from `0.19.5` to `0.19.6`; recorded local installed `0.15.6` and Homebrew stable `0.19.5` version skew.
- 2026-07-03: Rebuilt the frontmatter to satisfy `_schema.yaml`, including separate macOS/Linux/Windows binary, install, and config records.
- 2026-07-03: Reconciled compact `0.19.6` top-level help with the full packaged command parser and scoped subcommand help.
- 2026-07-03: Added newer wrapper-facing flags including `--max-subagent-depth` and `qwen serve` rate-limit controls.

## Sources

- [Qwen Code overview](https://qwenlm.github.io/qwen-code-docs/en/users/overview/)
- [Qwen Code GitHub repository](https://github.com/QwenLM/qwen-code)
- [Qwen Code quickstart](https://qwenlm.github.io/qwen-code-docs/en/users/quickstart/)
- [Qwen Code configuration docs](https://qwenlm.github.io/qwen-code-docs/en/users/configuration/settings/)
- [Qwen Code headless mode docs](https://qwenlm.github.io/qwen-code-docs/en/users/features/headless/)
- [Qwen Code extensions docs](https://qwenlm.github.io/qwen-code-docs/en/users/extension/introduction/)
- [Homebrew qwen-code formula](https://formulae.brew.sh/formula/qwen-code)
- [@qwen-code/qwen-code npm package](https://www.npmjs.com/package/@qwen-code/qwen-code)
- Local commands run on 2026-07-03: `qwen --version`, `qwen --help`, `npm view @qwen-code/qwen-code version bin engines dist-tags --json`, `brew info qwen-code --json=v2`, `npx --yes @qwen-code/qwen-code@0.19.6 --version`, `npx --yes @qwen-code/qwen-code@0.19.6 --help`, `npx --yes @qwen-code/qwen-code@0.19.6 mcp add --help`, `npx --yes @qwen-code/qwen-code@0.19.6 extensions install --help`, `npx --yes @qwen-code/qwen-code@0.19.6 sessions list --help`, `npx --yes @qwen-code/qwen-code@0.19.6 serve --help`, and unpacked `npm pack @qwen-code/qwen-code@0.19.6` source inspection.
