---
$schema: ./_schema.yaml
created: 2026-07-02
last_updated: 2026-07-02
agent: codex
model: default
latest_version: "0.19.5"
homepage: https://qwen.ai/qwencode
repo: https://github.com/QwenLM/qwen-code
docs: https://qwenlm.github.io/qwen-code-docs/
cli_docs: https://qwenlm.github.io/qwen-code-docs/en/users/overview/
binaries:
  - os: all
    binary: qwen
    alt_binaries: []
    notes: "npm package bin map exposes qwen. Standalone installers and Homebrew docs also use qwen."
  - os: windows
    binary: qwen
    alt_binaries: ["qwen.cmd", "qwen.ps1"]
    notes: "npm on Windows normally creates .cmd and PowerShell shims for package bins; official docs do not prove standalone shim filenames beyond the qwen command."
install_methods:
  - os: macos
    method: standalone_binary
    command: "curl -fsSL https://qwen-code-assets.oss-cn-hangzhou.aliyuncs.com/installation/install-qwen-standalone.sh | bash"
    notes: "Official README documents this as the primary Linux/macOS installer."
  - os: linux
    method: standalone_binary
    command: "curl -fsSL https://qwen-code-assets.oss-cn-hangzhou.aliyuncs.com/installation/install-qwen-standalone.sh | bash"
    notes: "Official README documents this as the primary Linux/macOS installer."
  - os: windows
    method: standalone_binary
    command: "irm https://qwen-code-assets.oss-cn-hangzhou.aliyuncs.com/installation/install-qwen-standalone.ps1 | iex"
    notes: "Official README documents this PowerShell installer for Windows."
  - os: all
    method: npm
    command: "npm install -g @qwen-code/qwen-code@latest"
    notes: "Requires Node.js 22+ as of npm package 0.19.5."
  - os: macos
    method: brew
    command: "brew install qwen-code"
    notes: "Official README documents Homebrew for macOS and Linux."
  - os: linux
    method: brew
    command: "brew install qwen-code"
    notes: "Official README documents Homebrew for macOS and Linux."
  - os: all
    method: source
    command: "git clone https://github.com/QwenLM/qwen-code.git && cd qwen-code && npm install && npm install -g ."
    notes: "Common source install path; useful for wrapper inspection, but official quick install paths are standalone, npm, and Homebrew."
subcommands:
  - name: "[query..]"
    description: "Default command. Launches the interactive CLI with no prompt, or runs a positional prompt as a one-shot task unless --prompt-interactive is used."
    non_interactive: true
    notes: "The same command is also the interactive TUI entry point when no prompt is supplied and stdin is a TTY."
  - name: "mcp"
    description: "Manages MCP servers."
    non_interactive: false
    notes: "Subcommands: add, remove, list, reconnect, approve, reject. add/remove/approve/reject mutate settings or approval state."
  - name: "extensions"
    description: "Manages Qwen Code extensions."
    non_interactive: false
    notes: "Subcommands include install, uninstall, list, update, disable, enable, link, new, settings, and sources."
  - name: "auth"
    description: "Removed legacy command that prints migration guidance."
    non_interactive: true
    notes: "Use interactive /auth, environment variables, CLI auth/model flags, or edit ~/.qwen/settings.json."
  - name: "hooks"
    description: "Placeholder command for hook management; directs users to /hooks in interactive mode."
    non_interactive: false
    notes: "Alias: hook. No useful non-interactive output was observed."
  - name: "channel"
    description: "Manages messaging channel daemon integration."
    non_interactive: false
    notes: "Subcommands: start, stop, status, pairing, configure-weixin. Channel flows may be long-running or interactive."
  - name: "review"
    description: "Internal helpers used by the /review skill."
    non_interactive: true
    notes: "Subcommands include fetch-pr, pr-context, load-rules, presubmit, and cleanup. Intended as implementation detail, not general wrapper entry point."
  - name: "serve"
    description: "Runs Qwen Code as a local HTTP daemon."
    non_interactive: true
    notes: "Long-running experimental daemon. Bearer token is optional for loopback by default and controlled by --token/QWEN_SERVER_TOKEN."
  - name: "sessions"
    description: "Manages saved Qwen Code sessions."
    non_interactive: true
    notes: "sessions list supports JSON Lines output with --json."
cli_switches:
  - flag: --help
    value: ""
    scope: ["global"]
    default: "false"
    description: "Shows help."
    example: "qwen --help"
    notes: "Short form: -h."
  - flag: --version
    value: ""
    scope: ["global"]
    default: "false"
    description: "Prints the CLI version."
    example: "qwen --version"
    notes: "Short form: -v."
  - flag: --telemetry
    value: ""
    scope: ["global", "telemetry"]
    default: "settings/env dependent"
    description: "Enables telemetry."
    example: "qwen --telemetry \"summarize\""
    notes: "Deprecated in favor of telemetry.enabled in settings.json."
  - flag: --telemetry-target
    value: "local | gcp"
    scope: ["global", "telemetry"]
    default: "settings/env dependent"
    description: "Sets the telemetry target label."
    example: "qwen --telemetry-target local"
    notes: "Deprecated in favor of telemetry.target in settings.json."
  - flag: --telemetry-otlp-endpoint
    value: "<URL>"
    scope: ["global", "telemetry"]
    default: "settings/env dependent"
    description: "Sets the OTLP endpoint for telemetry."
    example: "qwen --telemetry-otlp-endpoint http://localhost:4317"
    notes: "Deprecated in favor of telemetry.otlpEndpoint in settings.json."
  - flag: --telemetry-otlp-protocol
    value: "grpc | http"
    scope: ["global", "telemetry"]
    default: "grpc"
    description: "Sets the OTLP protocol."
    example: "qwen --telemetry-otlp-protocol http"
    notes: "Deprecated in favor of telemetry.otlpProtocol in settings.json."
  - flag: --telemetry-log-prompts
    value: ""
    scope: ["global", "telemetry"]
    default: "settings/env dependent"
    description: "Enables or disables logging prompts for telemetry."
    example: "qwen --telemetry-log-prompts"
    notes: "Deprecated in favor of telemetry.logPrompts in settings.json."
  - flag: --telemetry-outfile
    value: "<PATH>"
    scope: ["global", "telemetry"]
    default: "settings/env dependent"
    description: "Writes telemetry output to a file."
    example: "qwen --telemetry-outfile ./qwen-telemetry.jsonl"
    notes: "Deprecated in favor of telemetry.outfile in settings.json."
  - flag: --debug
    value: ""
    scope: ["global", "diagnostics"]
    default: "false"
    description: "Runs in debug mode."
    example: "qwen --debug \"diagnose\""
    notes: "Short form: -d."
  - flag: --bare
    value: ""
    scope: ["global", "isolation"]
    default: "false"
    description: "Skips implicit startup auto-discovery and honors only explicit CLI inputs."
    example: "qwen --bare \"summarize\""
    notes: "Useful for wrappers that want to suppress context, extensions, MCP, and other implicit state."
  - flag: --safe-mode
    value: ""
    scope: ["global", "isolation"]
    default: "false"
    description: "Disables customizations such as context files, hooks, extensions, skills, MCP servers, custom subagents, permission rules, and settings-sourced approval overrides."
    example: "qwen --safe-mode \"reproduce this bug\""
    notes: "Also documented as settable via QWEN_CODE_SAFE_MODE=true."
  - flag: --proxy
    value: "<URL>"
    scope: ["global", "network"]
    default: "settings/env dependent"
    description: "Sets the proxy for Qwen Code."
    example: "qwen --proxy http://localhost:7890"
    notes: "Deprecated in favor of proxy in settings.json; HTTPS_PROXY/HTTP_PROXY are also honored."
  - flag: --insecure
    value: ""
    scope: ["global", "network"]
    default: "false"
    description: "Skips TLS certificate verification for API connections."
    example: "qwen --insecure \"test local provider\""
    notes: "Equivalent to QWEN_TLS_INSECURE=1; weakens TLS security for API, OAuth, MCP, and child-process HTTPS."
  - flag: --chat-recording
    value: ""
    scope: ["global", "sessions"]
    default: "true"
    description: "Enables chat recording to disk; when false, history is not saved and --continue/--resume will not work."
    example: "qwen --no-chat-recording \"one-off\""
    notes: "Boolean yargs option supports --no-chat-recording."
  - flag: --model
    value: "<MODEL>"
    scope: ["default", "model_selection"]
    default: "auth/settings/env dependent"
    description: "Selects the model."
    example: "qwen --model qwen3-coder-plus \"inspect this repo\""
    notes: "Short form: -m."
  - flag: --prompt
    value: "<PROMPT>"
    scope: ["default", "non_interactive"]
    default: ""
    description: "Supplies a prompt; appended to stdin content if stdin is provided."
    example: "qwen --prompt \"summarize\""
    notes: "Short form: -p. Deprecated in favor of positional prompt, but still present in 0.19.5."
  - flag: --prompt-interactive
    value: "<PROMPT>"
    scope: ["default", "interactive"]
    default: ""
    description: "Executes the provided prompt and continues in interactive mode."
    example: "qwen --prompt-interactive \"start by reading README\""
    notes: "Short form: -i. Cannot be combined with --prompt or --json-schema."
  - flag: --system-prompt
    value: "<TEXT>"
    scope: ["default", "prompting"]
    default: ""
    description: "Overrides the main session system prompt for this run."
    example: "qwen \"review\" --system-prompt \"You are a strict reviewer.\""
    notes: "Loaded context files such as QWEN.md are still appended after this override."
  - flag: --append-system-prompt
    value: "<TEXT>"
    scope: ["default", "prompting"]
    default: ""
    description: "Appends instructions to the main session system prompt for this run."
    example: "qwen \"review\" --append-system-prompt \"Focus on regressions.\""
    notes: "Can be combined with --system-prompt."
  - flag: --sandbox
    value: ""
    scope: ["default", "sandbox"]
    default: "false"
    description: "Runs in a sandbox."
    example: "qwen --sandbox \"run tests\""
    notes: "Short form: -s. QWEN_SANDBOX overrides the CLI flag and settings."
  - flag: --sandbox-image
    value: "<IMAGE>"
    scope: ["default", "sandbox"]
    default: "ghcr.io/qwenlm/qwen-code:0.19.5"
    description: "Selects the sandbox image URI."
    example: "qwen --sandbox --sandbox-image ghcr.io/qwenlm/qwen-code:0.19.5"
    notes: "Deprecated in favor of tools.sandboxImage in settings.json."
  - flag: --yolo
    value: ""
    scope: ["default", "permissions"]
    default: "false"
    description: "Auto-approves all actions."
    example: "qwen --yolo \"apply the fix\""
    notes: "Short form: -y. Does not enable sandboxing; wrappers should prefer --approval-mode=yolo if they need an explicit mode value."
  - flag: --approval-mode
    value: "plan | default | auto-edit | auto | yolo"
    scope: ["default", "permissions"]
    default: "default"
    description: "Sets approval behavior for tool execution."
    example: "qwen --approval-mode plan \"make a plan\""
    notes: "Cannot be combined with --yolo. Untrusted folders force modes other than default/plan back to default."
  - flag: --acp
    value: ""
    scope: ["default", "protocol"]
    default: "false"
    description: "Starts the agent in ACP mode."
    example: "qwen --acp"
    notes: "Stable replacement for hidden --experimental-acp. Not compatible with --json-schema."
  - flag: --experimental-lsp
    value: ""
    scope: ["default", "lsp"]
    default: "false"
    description: "Enables experimental LSP features for code intelligence."
    example: "qwen --experimental-lsp"
    notes: "Requires language servers."
  - flag: --channel
    value: "VSCode | ACP | SDK | CI | desktop"
    scope: ["default", "integration"]
    default: ""
    description: "Sets a channel identifier."
    example: "qwen --channel CI \"run checks\""
    notes: "Useful for integration identity."
  - flag: --allowed-mcp-server-names
    value: "<NAME>[,<NAME>...]"
    scope: ["default", "mcp"]
    default: ""
    description: "Restricts allowed MCP server names for the session."
    example: "qwen --allowed-mcp-server-names filesystem,github \"work\""
    notes: "Can be repeated or comma-separated."
  - flag: --mcp-config
    value: "<JSON_OR_PATH>"
    scope: ["default", "mcp"]
    default: ""
    description: "Injects MCP server configuration as inline JSON or a path to a JSON file."
    example: "qwen --mcp-config ./mcp.json \"use tools\""
    notes: "Expected shape includes {\"mcpServers\": {...}}."
  - flag: --allowed-tools
    value: "<TOOL>[,<TOOL>...]"
    scope: ["default", "permissions"]
    default: ""
    description: "Allows tools to run without confirmation."
    example: "qwen --allowed-tools read_file,grep \"inspect\""
    notes: "Declared twice in source with equivalent array/coerce behavior; can be repeated or comma-separated."
  - flag: --extensions
    value: "<EXT>[,<EXT>...]"
    scope: ["default", "extensions"]
    default: "all extensions"
    description: "Selects extensions to use for the session."
    example: "qwen --extensions none \"ignore extensions\""
    notes: "Short form: -e. Docs mention special value none to disable all extensions."
  - flag: --list-extensions
    value: ""
    scope: ["default", "extensions"]
    default: "false"
    description: "Lists available extensions and exits."
    example: "qwen --list-extensions"
    notes: "Short form: -l. Output format is human text, not documented JSON."
  - flag: --include-directories
    value: "<PATH>[,<PATH>...]"
    scope: ["default", "context"]
    default: ""
    description: "Adds directories to include in workspace context."
    example: "qwen --include-directories ../lib,../cli \"inspect both\""
    notes: "Alias: --add-dir. Docs state a maximum of 5 directories."
  - flag: --openai-logging
    value: ""
    scope: ["default", "diagnostics"]
    default: "settings dependent"
    description: "Enables OpenAI API call logging."
    example: "qwen --openai-logging \"debug provider\""
    notes: "Overrides the enableOpenAILogging setting."
  - flag: --openai-logging-dir
    value: "<PATH>"
    scope: ["default", "diagnostics"]
    default: "logs/openai"
    description: "Sets the OpenAI API log directory."
    example: "qwen --openai-logging --openai-logging-dir ~/qwen-logs"
    notes: "Supports absolute paths, relative paths, and ~ expansion."
  - flag: --openai-api-key
    value: "<KEY>"
    scope: ["default", "auth"]
    default: "env/settings dependent"
    description: "Sets an OpenAI API key for authentication."
    example: "qwen --openai-api-key \"$OPENAI_API_KEY\" \"run\""
    notes: "Model-provider detail; included because source exposes it as CLI surface."
  - flag: --openai-base-url
    value: "<URL>"
    scope: ["default", "auth"]
    default: "env/settings dependent"
    description: "Sets an OpenAI-compatible base URL."
    example: "qwen --openai-base-url http://localhost:11434/v1 --model qwen3-coder"
    notes: "Model-provider detail; included because source exposes it as CLI surface."
  - flag: --screen-reader
    value: ""
    scope: ["default", "accessibility"]
    default: "false"
    description: "Enables screen reader mode."
    example: "qwen --screen-reader"
    notes: "Adjusts TUI behavior."
  - flag: --input-format
    value: "text | stream-json"
    scope: ["default", "io"]
    default: "text"
    description: "Selects stdin input protocol."
    example: "qwen --input-format stream-json --output-format stream-json"
    notes: "stream-json input requires --output-format stream-json."
  - flag: --output-format
    value: "text | json | stream-json"
    scope: ["default", "io"]
    default: "text"
    description: "Selects CLI output format."
    example: "qwen --output-format stream-json \"run task\""
    notes: "Short form: -o. json emits a buffered JSON array; stream-json emits JSONL events."
  - flag: --include-partial-messages
    value: ""
    scope: ["default", "io"]
    default: "false"
    description: "Includes partial assistant messages in stream-json output."
    example: "qwen -o stream-json --include-partial-messages \"write\""
    notes: "Requires --output-format stream-json."
  - flag: --json-fd
    value: "<FD>"
    scope: ["default", "dual_output"]
    default: ""
    description: "Writes structured JSON event output to a supplied file descriptor while the TUI renders normally."
    example: "spawn qwen with fd 3 and pass --json-fd 3"
    notes: "Mutually exclusive with --json-file; caller must configure spawn stdio."
  - flag: --json-file
    value: "<PATH>"
    scope: ["default", "dual_output"]
    default: "settings dependent"
    description: "Writes structured JSON event output to a file, FIFO, or /dev/fd/N."
    example: "qwen --json-file ./events.jsonl"
    notes: "Mutually exclusive with --json-fd."
  - flag: --json-schema
    value: "<JSON_OR_@PATH>"
    scope: ["default", "structured_output"]
    default: ""
    description: "Requires the final headless output to conform to a JSON Schema."
    example: "qwen \"summarize\" --json-schema @./schema.json"
    notes: "Headless only. Rejected with --prompt-interactive, --input-format stream-json, --acp, or no prompt/stdin."
  - flag: --input-file
    value: "<PATH>"
    scope: ["default", "dual_output"]
    default: "settings dependent"
    description: "Reads remote JSONL input commands from a file for bidirectional sync."
    example: "qwen --input-file ./commands.jsonl"
    notes: "The TUI watches the file and processes external commands."
  - flag: --continue
    value: ""
    scope: ["default", "sessions"]
    default: "false"
    description: "Resumes the most recent session for the current project."
    example: "qwen --continue \"next\""
    notes: "Short form: -c. Cannot be combined with --resume or --session-id."
  - flag: --resume
    value: "<SESSION_ID>"
    scope: ["default", "sessions"]
    default: ""
    description: "Resumes a specific session by ID, or shows a picker when used without an ID."
    example: "qwen --resume 123e4567-e89b-12d3-a456-426614174000 \"continue\""
    notes: "Short form: -r. Cannot be combined with --continue or --session-id."
  - flag: --session-id
    value: "<UUID>"
    scope: ["default", "sessions"]
    default: "generated"
    description: "Specifies a session ID for a new run."
    example: "qwen --session-id 123e4567-e89b-12d3-a456-426614174000 \"run\""
    notes: "Must be UUID-shaped; not valid with --continue or --resume."
  - flag: --fork-session
    value: ""
    scope: ["default", "sessions"]
    default: "false"
    description: "Creates a forked session from a resumed session."
    example: "qwen --continue --fork-session \"try alternate fix\""
    notes: "Requires --continue or --resume."
  - flag: --worktree
    value: "[SLUG_OR_PR]"
    scope: ["default", "git"]
    default: ""
    description: "Starts the session inside a git worktree under <repoRoot>/.qwen/worktrees/<slug>/."
    example: "qwen --worktree my-feature \"implement\""
    notes: "Accepts a slug, bare flag for auto-generation, #123, or a GitHub pull-request URL. Interactive exit dialog may prompt to keep/remove."
  - flag: --max-session-turns
    value: "<N>"
    scope: ["default", "limits"]
    default: "settings dependent or unlimited"
    description: "Limits session turns."
    example: "qwen --max-session-turns 8 \"run bounded task\""
    notes: "Useful for CI budgets."
  - flag: --max-wall-time
    value: "<DURATION>"
    scope: ["default", "limits"]
    default: "settings dependent or unlimited"
    description: "Sets a run-level wall-clock budget for headless or unattended runs."
    example: "qwen --max-wall-time 10m \"run bounded task\""
    notes: "Accepts seconds or duration strings such as 30s, 5m, 1h. Aborts with exit code 55 when exceeded."
  - flag: --max-tool-calls
    value: "<N>"
    scope: ["default", "limits"]
    default: "-1"
    description: "Limits cumulative tool calls for a run."
    example: "qwen --max-tool-calls 20 \"inspect\""
    notes: "-1/unset means unlimited; 0 means no tool calls. Aborts with exit code 55 when exceeded."
  - flag: --core-tools
    value: "<TOOL>[,<TOOL>...]"
    scope: ["default", "tools"]
    default: "settings dependent"
    description: "Restricts registered core tools."
    example: "qwen --core-tools read_file,grep"
    notes: "Whitelist semantics; not the same as auto-approval."
  - flag: --exclude-tools
    value: "<TOOL>[,<TOOL>...]"
    scope: ["default", "tools"]
    default: "settings dependent"
    description: "Excludes tools."
    example: "qwen --exclude-tools shell,write_file \"inspect only\""
    notes: "Can be repeated or comma-separated."
  - flag: --disabled-slash-commands
    value: "<NAME>[,<NAME>...]"
    scope: ["default", "slash_commands"]
    default: "settings/env dependent"
    description: "Hides or disables slash commands."
    example: "qwen --disabled-slash-commands auth,mcp,extensions"
    notes: "Merged with settings and QWEN_DISABLED_SLASH_COMMANDS; matched case-insensitively."
  - flag: --auth-type
    value: "openai | anthropic | qwen-oauth | gemini | vertex-ai"
    scope: ["default", "auth"]
    default: "env/settings dependent"
    description: "Selects authentication/provider protocol."
    example: "qwen --auth-type openai --model qwen3-coder-plus"
    notes: "Qwen OAuth is configured interactively; CI/headless should use API-key/provider environment variables."
  - flag: --scope
    value: "user | project"
    scope: ["mcp add"]
    default: "user"
    description: "Selects where an MCP server is stored."
    example: "qwen mcp add --scope project local python -m server"
    notes: "Short form: -s."
  - flag: --transport
    value: "stdio | sse | http"
    scope: ["mcp add"]
    default: "stdio"
    description: "Selects MCP transport."
    example: "qwen mcp add --transport http my-server http://localhost:3000/mcp"
    notes: "Short form: -t."
  - flag: --env
    value: "KEY=value"
    scope: ["mcp add"]
    default: ""
    description: "Adds environment variables for an MCP server."
    example: "qwen mcp add -e TOKEN=abc local node server.js"
    notes: "Short form: -e."
  - flag: --header
    value: "NAME: value"
    scope: ["mcp add"]
    default: ""
    description: "Adds HTTP headers for SSE/HTTP MCP transports."
    example: "qwen mcp add -H \"X-Api-Key: abc\" --transport http remote http://localhost:3000/mcp"
    notes: "Short form: -H."
  - flag: --timeout
    value: "<MS>"
    scope: ["mcp add"]
    default: ""
    description: "Sets MCP server connection timeout in milliseconds."
    example: "qwen mcp add --timeout 30000 local python -m server"
    notes: ""
  - flag: --trust
    value: ""
    scope: ["mcp add"]
    default: "false"
    description: "Trusts an MCP server and bypasses all tool-call confirmation prompts for it."
    example: "qwen mcp add --trust local python -m server"
    notes: "Wrapper caveat: mutates persistent trust/permission behavior."
  - flag: --description
    value: "<TEXT>"
    scope: ["mcp add"]
    default: ""
    description: "Sets the MCP server description."
    example: "qwen mcp add --description \"Local tools\" local python -m server"
    notes: ""
  - flag: --include-tools
    value: "<TOOL>[,<TOOL>...]"
    scope: ["mcp add"]
    default: "all tools"
    description: "Includes only selected MCP tools."
    example: "qwen mcp add --include-tools search,fetch remote http://localhost:3000/mcp"
    notes: ""
  - flag: --exclude-tools
    value: "<TOOL>[,<TOOL>...]"
    scope: ["mcp add"]
    default: "none"
    description: "Excludes selected MCP tools."
    example: "qwen mcp add --exclude-tools write_file local python -m server"
    notes: "Also exists as a default-command tool exclusion flag."
  - flag: --oauth-client-id
    value: "<ID>"
    scope: ["mcp add"]
    default: ""
    description: "Sets OAuth client ID for MCP server authentication."
    example: "qwen mcp add --transport http --oauth-client-id id remote http://localhost:3000/mcp"
    notes: "Only for sse/http transports."
  - flag: --oauth-client-secret
    value: "<SECRET>"
    scope: ["mcp add"]
    default: ""
    description: "Sets OAuth client secret for MCP server authentication."
    example: "qwen mcp add --transport http --oauth-client-secret secret remote http://localhost:3000/mcp"
    notes: "Only for sse/http transports."
  - flag: --oauth-redirect-uri
    value: "<URI>"
    scope: ["mcp add"]
    default: "http://localhost:7777/oauth/callback"
    description: "Sets OAuth redirect URI for MCP authentication."
    example: "qwen mcp add --transport sse --oauth-redirect-uri https://example.com/oauth/callback remote https://example.com/sse"
    notes: "Remote/cloud environments must configure a public callback; localhost will not work there."
  - flag: --oauth-authorization-url
    value: "<URL>"
    scope: ["mcp add"]
    default: ""
    description: "Sets OAuth authorization URL for MCP authentication."
    example: "qwen mcp add --transport http --oauth-authorization-url https://provider.example.com/authorize remote http://localhost:3000/mcp"
    notes: "Only for sse/http transports."
  - flag: --oauth-token-url
    value: "<URL>"
    scope: ["mcp add"]
    default: ""
    description: "Sets OAuth token URL for MCP authentication."
    example: "qwen mcp add --transport http --oauth-token-url https://provider.example.com/token remote http://localhost:3000/mcp"
    notes: "Only for sse/http transports."
  - flag: --oauth-scopes
    value: "<SCOPE>[,<SCOPE>...]"
    scope: ["mcp add"]
    default: ""
    description: "Sets OAuth scopes for MCP authentication."
    example: "qwen mcp add --transport http --oauth-scopes scope1,scope2 remote http://localhost:3000/mcp"
    notes: "Only for sse/http transports."
  - flag: --all
    value: ""
    scope: ["mcp approve", "mcp reject", "extensions update"]
    default: "false"
    description: "Applies the operation to all matching items."
    example: "qwen mcp approve --all"
    notes: "For extensions update, updates all extensions; for MCP approval, approves/rejects all gated servers."
  - flag: --json
    value: ""
    scope: ["sessions list"]
    default: "false"
    description: "Outputs sessions as JSON Lines."
    example: "qwen sessions list --json"
    notes: "Primary machine-readable state command found in the CLI."
  - flag: --limit
    value: "<N>"
    scope: ["sessions list"]
    default: "20"
    description: "Limits the number of sessions shown."
    example: "qwen sessions list --json --limit 100"
    notes: "Invalid values coerce back to 20."
  - flag: --token
    value: "<TOKEN>"
    scope: ["serve"]
    default: "QWEN_SERVER_TOKEN or none on loopback"
    description: "Sets bearer token for the local HTTP daemon."
    example: "qwen serve --token \"$QWEN_SERVER_TOKEN\""
    notes: "serve-specific help/docs expose this; token can also come from QWEN_SERVER_TOKEN."
  - flag: --require-auth
    value: ""
    scope: ["serve"]
    default: "false"
    description: "Requires bearer auth even on loopback."
    example: "qwen serve --require-auth --token \"$QWEN_SERVER_TOKEN\""
    notes: "Useful on shared developer hosts and CI runners."
config_files:
  - os: all
    scope: user
    path: "~/.qwen/settings.json"
    format: json
    notes: "Primary user settings file. QWEN_HOME changes the global config directory, so this becomes <QWEN_HOME>/settings.json."
  - os: all
    scope: repo
    path: ".qwen/settings.json"
    format: json
    notes: "Project settings file in the project root. Ignored when the workspace is untrusted."
  - os: linux
    scope: system
    path: "/etc/qwen-code/settings.json"
    format: json
    notes: "System settings file. QWEN_CODE_SYSTEM_SETTINGS_PATH can override this path."
  - os: windows
    scope: system
    path: "C:\\ProgramData\\qwen-code\\settings.json"
    format: json
    notes: "System settings file. QWEN_CODE_SYSTEM_SETTINGS_PATH can override this path."
  - os: macos
    scope: system
    path: "/Library/Application Support/QwenCode/settings.json"
    format: json
    notes: "System settings file. QWEN_CODE_SYSTEM_SETTINGS_PATH can override this path."
  - os: linux
    scope: system
    path: "/etc/qwen-code/system-defaults.json"
    format: json
    notes: "System defaults file. QWEN_CODE_SYSTEM_DEFAULTS_PATH can override this path."
  - os: windows
    scope: system
    path: "C:\\ProgramData\\qwen-code\\system-defaults.json"
    format: json
    notes: "System defaults file. QWEN_CODE_SYSTEM_DEFAULTS_PATH can override this path."
  - os: macos
    scope: system
    path: "/Library/Application Support/QwenCode/system-defaults.json"
    format: json
    notes: "System defaults file. QWEN_CODE_SYSTEM_DEFAULTS_PATH can override this path."
  - os: all
    scope: env
    path: "~/.qwen/.env"
    format: text
    notes: "Global dotenv file loaded from QWEN_HOME when set; wins over project .env files."
  - os: all
    scope: repo
    path: ".qwen/.env"
    format: text
    notes: "Project-scoped dotenv file. Docs recommend this over .env to avoid conflicts."
  - os: all
    scope: repo
    path: ".env"
    format: text
    notes: "Project dotenv file; some variables such as DEBUG and DEBUG_MODE are excluded by default."
  - os: all
    scope: repo
    path: ".mcp.json"
    format: json
    notes: "Project MCP server file; gated servers require qwen mcp approve/reject state."
  - os: all
    scope: user
    path: "~/.qwen/QWEN.md"
    format: text
    notes: "Global context file. Actual filename can be changed by context.fileName."
  - os: all
    scope: repo
    path: "QWEN.md"
    format: text
    notes: "Hierarchical project context file loaded from current directory and parents according to context settings."
  - os: all
    scope: repo
    path: ".qwen/sandbox.Dockerfile"
    format: text
    notes: "Custom project sandbox image Dockerfile used when BUILD_SANDBOX=1."
  - os: macos
    scope: repo
    path: ".qwen/sandbox-macos-<profile>.sb"
    format: text
    notes: "Custom macOS Seatbelt profile selected with SEATBELT_PROFILE."
  - os: all
    scope: user
    path: "~/.qwen/mcp-oauth-tokens.json"
    format: json
    notes: "Default plaintext MCP OAuth token store, mode 0600."
  - os: all
    scope: user
    path: "~/.qwen/mcp-oauth-tokens-v2.json"
    format: json
    notes: "Encrypted MCP OAuth token store used when QWEN_CODE_FORCE_ENCRYPTED_FILE_STORAGE=true and keychain-backed storage is unavailable."
env_vars:
  - name: QWEN_HOME
    effect: "Overrides the global configuration directory, defaulting to ~/.qwen."
  - name: QWEN_RUNTIME_DIR
    effect: "Overrides runtime output directory for conversations, logs, todos, and daemon debug files."
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
  - name: SANDBOX_SET_UID_GID
    effect: "Controls host UID/GID mapping in container sandbox mode."
  - name: QWEN_SANDBOX_PROXY_COMMAND
    effect: "Configures sandbox proxy command behavior."
  - name: QWEN_CODE_SAFE_MODE
    effect: "Enables safe mode when CLI flags cannot be passed."
  - name: QWEN_CODE_SUPPRESS_YOLO_WARNING
    effect: "Suppresses the warning emitted for headless YOLO runs without sandboxing."
  - name: QWEN_CODE_UNATTENDED_RETRY
    effect: "Enables persistent retry for transient 429/529 API capacity errors, with stderr heartbeat keepalives."
  - name: QWEN_CODE_MAX_OUTPUT_TOKENS
    effect: "Overrides the default maximum output tokens per model response."
  - name: QWEN_DISABLED_SLASH_COMMANDS
    effect: "Adds comma-separated slash commands to hide/disable."
  - name: QWEN_CODE_LANG
    effect: "Overrides UI language."
  - name: QWEN_CODE_LEGACY_MCP_BLOCKING
    effect: "Restores synchronous MCP discovery before UI/prompt startup."
  - name: QWEN_CODE_FORCE_ENCRYPTED_FILE_STORAGE
    effect: "Uses encrypted/keychain-backed storage for MCP OAuth tokens where available."
  - name: QWEN_CODE_MCP_APPROVALS_PATH
    effect: "Overrides path for MCP approval state."
  - name: QWEN_TLS_INSECURE
    effect: "Disables TLS certificate verification when set to 1."
  - name: NO_COLOR
    effect: "Disables color output."
  - name: FORCE_HYPERLINK
    effect: "Overrides OSC 8 hyperlink auto-detection."
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
  - name: NO_BROWSER
    effect: "Prevents browser-opening behavior in integrations that honor the noBrowser config."
  - name: CLI_TITLE
    effect: "Customizes the CLI title."
  - name: QWEN_SERVER_TOKEN
    effect: "Bearer token source for qwen serve and SDK clients."
  - name: QWEN_SERVE_PROMPT_DEADLINE_MS
    effect: "Default server-side deadline for qwen serve prompt requests."
  - name: QWEN_SERVE_WRITER_IDLE_TIMEOUT_MS
    effect: "Idle deadline for qwen serve SSE writers."
  - name: QWEN_SERVE_DEBUG
    effect: "Enables extra qwen serve bridge debug breadcrumbs."
  - name: QWEN_DAEMON_LOG_FILE
    effect: "Disables qwen serve daemon file logging when set to 0/false/off/no."
  - name: QWEN_SERVE_CDP_TUNNEL_OVER_WS
    effect: "Signals that browser automation is routed through the CDP tunnel, disabling computer-use tools."
machine_introspection:
  - command: "qwen sessions list --json --limit 100"
    purpose: other
    machine_readable: true
    output_format: jsonl
    useful_for_codegen: false
    notes: "Lists saved sessions as JSON Lines with sessionId, startTime, mtime, prompt, gitBranch, customTitle, titleSource, filePath, and cwd."
  - command: "qwen --list-extensions"
    purpose: plugins
    machine_readable: false
    output_format: text
    useful_for_codegen: false
    notes: "Lists available extensions and exits, but no JSON mode was found."
  - command: "qwen mcp list"
    purpose: mcp
    machine_readable: false
    output_format: text
    useful_for_codegen: false
    notes: "Lists configured MCP servers. No JSON mode was found in the 0.19.5 command builder."
  - command: "qwen serve + GET /capabilities"
    purpose: capabilities
    machine_readable: true
    output_format: json
    useful_for_codegen: true
    notes: "Not a single CLI introspection command; requires starting the HTTP daemon and calling its API with bearer auth when configured."
wrapper_notes:
  - "Use the positional prompt for headless runs; --prompt still works in 0.19.5 but is deprecated."
  - "For stream parsing, prefer --output-format stream-json. --output-format json buffers a JSON array until completion."
  - "--json-schema changes headless text output to a single JSON.stringify(payload) line and exposes structured_result in json/stream-json result events."
  - "--json-schema is rejected with --prompt-interactive, --input-format stream-json, --acp, or no prompt/stdin."
  - "--json-fd and --json-file provide dual structured event output while the normal UI remains on stdout; --json-fd requires explicit fd plumbing in the spawn call."
  - "Non-interactive runs wait for MCP discovery to settle before sending the first prompt; interactive runs let MCP servers come online progressively."
  - "--bare and --safe-mode are useful wrapper isolation controls. safe-mode disables settings-sourced approval mode and sandbox settings, while explicit --approval-mode/--yolo still apply."
  - "--yolo or --approval-mode=yolo does not enable sandboxing. Headless YOLO without sandbox prints a stderr warning unless QWEN_CODE_SUPPRESS_YOLO_WARNING=1."
  - "Untrusted folders force approval modes other than default/plan back to default and ignore project .qwen/settings.json."
  - "--worktree can prompt on exit to keep or remove the worktree; avoid it for unattended wrappers unless that behavior is acceptable."
  - "auth is a removed legacy command. Use interactive /auth, env vars, CLI OpenAI flags, or settings.json for scripted setup."
  - "Qwen OAuth cannot be fully configured by env vars alone; CI/headless should use API-key auth such as OpenAI-compatible settings."
  - "The latest npm package requires Node.js >=22.0.0; older docs and installations may still mention Node.js 20+."
  - "Local PATH inspection on this host found qwen 0.15.6, older than npm latest 0.19.5, so wrappers should not infer current surface from an arbitrary installed binary."
changes: []
requires_claudine_update: true
reason: "The existing Qwen research file used legacy frontmatter and lacked the current 0.19.5 CLI surface, including qwen as the binary, Node >=22 npm requirement, standalone installers, --bare/--safe-mode, JSON/stream-json output, dual-output flags, structured-output flags, sessions JSONL introspection, serve mode, and changed auth behavior."
---

# Qwen Code CLI

## Overview

Qwen Code is Alibaba/Qwen's terminal coding agent. It is distributed as the `@qwen-code/qwen-code` npm package, standalone installers, and a Homebrew formula, and exposes the `qwen` command. The latest npm package observed on 2026-07-02 is `0.19.5`; local PATH inspection found an older `qwen 0.15.6`, so the structured inventory above is based on the latest npm package metadata and unpacked `0.19.5` CLI bundle, with official docs used as cross-checks.

The default command is both the TUI entry point and the headless execution entry point. Supplying a positional prompt or `--prompt` runs one-shot non-interactively; `--prompt-interactive` starts with a prompt and then remains interactive.

## Installation and Binaries

The public binary is `qwen`. npm exposes `bin.qwen`, and official standalone/Homebrew examples also use `qwen`.

Official install paths:

- Linux/macOS standalone: `curl -fsSL https://qwen-code-assets.oss-cn-hangzhou.aliyuncs.com/installation/install-qwen-standalone.sh | bash`
- Windows standalone: `irm https://qwen-code-assets.oss-cn-hangzhou.aliyuncs.com/installation/install-qwen-standalone.ps1 | iex`
- npm: `npm install -g @qwen-code/qwen-code@latest`
- Homebrew: `brew install qwen-code`

The current npm package declares Node.js `>=22.0.0`. Older local installations or third-party docs may mention Node.js 20+, but npm metadata for `0.19.5` is stricter.

## Subcommands

Top-level commands in `0.19.5` are:

- `qwen [query..]`: default TUI/headless command.
- `qwen mcp`: MCP server management.
- `qwen extensions <command>`: extension management.
- `qwen auth`: removed legacy command that prints migration guidance.
- `qwen hooks` / `qwen hook`: hook management placeholder; use `/hooks` interactively.
- `qwen channel`: messaging channel daemon management.
- `qwen review`: internal helpers for the `/review` skill.
- `qwen serve`: local HTTP daemon.
- `qwen sessions`: saved session management.

ACP is a default-command mode via `qwen --acp`, not a top-level subcommand.

## CLI Switch Inventory

The complete switch inventory captured for Claudine is in frontmatter. The most wrapper-relevant switches are:

- Execution and model: `--model`, positional prompt, `--prompt`, `--prompt-interactive`, `--auth-type`.
- Output protocols: `--output-format text|json|stream-json`, `--input-format text|stream-json`, `--include-partial-messages`, `--json-schema`, `--json-fd`, `--json-file`, `--input-file`.
- Isolation and permissions: `--bare`, `--safe-mode`, `--sandbox`, `--approval-mode`, `--yolo`, `--allowed-tools`, `--exclude-tools`, `--core-tools`.
- State and sessions: `--continue`, `--resume`, `--session-id`, `--fork-session`, `--chat-recording`, `qwen sessions list --json`.
- Limits: `--max-session-turns`, `--max-wall-time`, `--max-tool-calls`.
- MCP: `--mcp-config`, `--allowed-mcp-server-names`, plus `qwen mcp add` flags.

Unknowns: the official docs do not publish a stable JSON schema for stream-json event objects in the user CLI docs. The bundled structured-output docs document `structured_result` for `--json-schema`, but general stream event typing should be re-verified from source before codegen.

## Configuration Discovery

Qwen discovers user, project, and system JSON settings. The documented files are `~/.qwen/settings.json`, project `.qwen/settings.json`, and per-OS system settings/defaults under `/etc/qwen-code`, `C:\ProgramData\qwen-code`, or `/Library/Application Support/QwenCode`. `QWEN_HOME` relocates global state, and `QWEN_CODE_SYSTEM_SETTINGS_PATH` / `QWEN_CODE_SYSTEM_DEFAULTS_PATH` override system paths.

Project MCP can also come from `.mcp.json`; gated project/workspace MCP servers require `qwen mcp approve` or `qwen mcp reject`. Context is supplied through `QWEN.md` files by default, with the filename configurable in settings.

## Environment Variables

The frontmatter lists general CLI/runtime variables that affect wrapper behavior. Model-provider secrets such as `OPENAI_API_KEY`, `OPENAI_BASE_URL`, `OPENAI_MODEL`, provider-specific keys, and GitHub Action-only variables are intentionally not exhaustively duplicated here because those belong in model-config or integration-specific research.

Important wrapper variables include `QWEN_HOME`, `QWEN_RUNTIME_DIR`, `QWEN_SANDBOX`, `QWEN_CODE_SAFE_MODE`, `QWEN_CODE_UNATTENDED_RETRY`, `QWEN_DISABLED_SLASH_COMMANDS`, `QWEN_CODE_LEGACY_MCP_BLOCKING`, `QWEN_TLS_INSECURE`, `NO_COLOR`, proxy variables, and `QWEN_SERVER_TOKEN`.

## Machine Introspection

Useful machine-readable CLI state is limited. `qwen sessions list --json` emits JSON Lines and is the clearest direct introspection command found. `qwen mcp list` and `qwen --list-extensions` expose useful state but only as human-readable text in the observed command builders. `qwen serve` exposes HTTP JSON endpoints such as capabilities, but that requires launching the daemon and is not a single fire-and-exit CLI probe.

Generic `qwen --help` and `qwen --version` were used for verification but are not listed as machine-introspection entries because they do not expose durable provider state.

## Wrapper Notes

Use positional prompts for new headless wrapper calls because `--prompt` is deprecated. Use `--output-format stream-json` for streaming wrappers, or `--output-format json` only when buffered output is acceptable. For strict structured final output, `--json-schema` is useful but has important mode conflicts listed above.

For isolation, `--bare` and `--safe-mode` are valuable. Be careful with `--yolo`: it does not imply sandboxing, and Qwen warns on stderr in headless YOLO without sandbox. In untrusted folders, Qwen ignores project settings and forces risky approval modes back to default.

Auth setup changed: `qwen auth` is removed and prints instructions. Headless wrappers should use provider environment variables, CLI OpenAI-compatible flags, or settings files; Qwen OAuth needs interactive `/auth`.

## Sources

- [Qwen Code overview](https://qwenlm.github.io/qwen-code-docs/en/users/overview/)
- [Qwen Code GitHub repository](https://github.com/QwenLM/qwen-code)
- [@qwen-code/qwen-code npm package](https://www.npmjs.com/package/@qwen-code/qwen-code)
- [Qwen Code configuration docs](https://qwenlm.github.io/qwen-code-docs/en/users/configuration/settings/)
- [Qwen Code headless mode docs](https://qwenlm.github.io/qwen-code-docs/en/users/features/headless/)
- [Qwen Code commands docs](https://qwenlm.github.io/qwen-code-docs/en/users/features/commands/)
- Local inspection of npm package `@qwen-code/qwen-code@0.19.5` and its bundled CLI/docs on 2026-07-02.
