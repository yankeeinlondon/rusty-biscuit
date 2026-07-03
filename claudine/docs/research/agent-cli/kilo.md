---
$schema: ./_schema.yaml
created: 2026-07-02
last_updated: 2026-07-02
agent: codex
model: default
latest_version: "7.3.54"
homepage: https://kilo.ai/
repo: https://github.com/Kilo-Org/kilocode
docs: https://kilo.ai/docs
cli_docs: https://kilo.ai/docs/code-with-ai/platforms/cli-reference
binaries:
  - os: all
    binary: kilo
    alt_binaries: ["kilocode"]
    notes: "The npm package @kilocode/cli exposes both `kilo` and `kilocode`, with both mapped to bin/kilo. Official docs use `kilo`."
  - os: windows
    binary: kilo.cmd
    alt_binaries: ["kilo.ps1", "kilocode.cmd", "kilocode.ps1"]
    notes: "Windows npm global installs normally expose command and PowerShell shims for package bins; no separate upstream command name is documented."
install_methods:
  - os: all
    method: npm
    command: "npm install -g @kilocode/cli"
    notes: "Primary documented install method for Kilo CLI 1.0 and later."
  - os: macos
    method: standalone_binary
    command: "download kilo-darwin-x64-baseline.zip from GitHub releases"
    notes: "Documented fallback for older x64 CPUs without AVX support; arm64 baseline package was not documented."
  - os: linux
    method: standalone_binary
    command: "download kilo-linux-x64-baseline.tar.gz from GitHub releases"
    notes: "Documented fallback for older x64 CPUs without AVX support."
  - os: windows
    method: standalone_binary
    command: "download kilo-windows-x64-baseline.zip from GitHub releases"
    notes: "Documented fallback for older x64 CPUs without AVX support."
  - os: all
    method: source
    command: "unknown"
    notes: "Repository is public, but the public CLI install docs do not provide a source-build install command."
subcommands:
  - name: "(default TUI)"
    description: "Starts the Kilo terminal UI, optionally scoped to a project path."
    non_interactive: false
    notes: "Usage shown as `kilo [project]`."
  - name: acp
    description: "Starts an ACP server."
    non_interactive: true
    notes: "Server command; remains running until stopped."
  - name: mcp
    description: "Manages MCP servers, OAuth auth, logout, and debug flows."
    non_interactive: false
    notes: "`mcp list` is inspectable; `mcp auth` can require browser/OAuth interaction."
  - name: attach
    description: "Attaches to a running Kilo server URL."
    non_interactive: false
    notes: "Can continue, resume, or fork remote sessions."
  - name: run
    description: "Runs Kilo with a message."
    non_interactive: true
    notes: "Use `--auto` for autonomous/pipeline usage; `--format json` emits raw JSON events."
  - name: debug
    description: "Exposes troubleshooting tools for config, LSP, ripgrep, files, paths, agents, snapshots, startup, and debug info."
    non_interactive: true
    notes: "Most debug subcommands print inspectable state; `debug wait` intentionally waits forever."
  - name: auth
    description: "Manages AI provider and Kilo Gateway credentials."
    non_interactive: false
    notes: "`auth list` is inspectable; `auth login` may prompt or launch browser flows."
  - name: agent
    description: "Creates or lists agents."
    non_interactive: true
    notes: "`agent create` mutates agent files; `agent list` is an inventory command."
  - name: upgrade
    description: "Upgrades Kilo to the latest or a specific version."
    non_interactive: false
    notes: "Can invoke package managers; supports explicit install method selection."
  - name: uninstall
    description: "Uninstalls Kilo and optionally removes related config/data."
    non_interactive: false
    notes: "Use `--dry-run` and `--force` for automation."
  - name: serve
    description: "Starts a headless Kilo HTTP server."
    non_interactive: true
    notes: "Long-running server command."
  - name: web
    description: "Starts a Kilo server and opens the web interface."
    non_interactive: false
    notes: "Wrapper should expect browser/open side effects."
  - name: models
    description: "Lists available models, optionally filtered by provider."
    non_interactive: true
    notes: "Supports verbose metadata and cache refresh."
  - name: roll-call
    description: "Batch-tests text models matching a regex filter."
    non_interactive: true
    notes: "Supports JSON output but performs live provider calls."
  - name: profile
    description: "Shows the Kilo account profile."
    non_interactive: true
    notes: "Supports JSON output; exits non-zero when not authenticated."
  - name: stats
    description: "Shows token usage and cost statistics."
    non_interactive: true
    notes: "Human output only in current public help."
  - name: export
    description: "Exports session data as JSON."
    non_interactive: true
    notes: "Can sanitize transcript and file data."
  - name: import
    description: "Imports session data from a JSON file or share URL."
    non_interactive: false
    notes: "Mutates local session storage."
  - name: github
    description: "Installs or runs the GitHub agent."
    non_interactive: false
    notes: "`github run` accepts a token and mock event."
  - name: pr
    description: "Fetches and checks out a GitHub PR branch, then runs Kilo."
    non_interactive: false
    notes: "Mutates git working tree state."
  - name: session
    description: "Lists and deletes local sessions."
    non_interactive: true
    notes: "`session list --format json` is machine-readable; delete mutates storage."
  - name: remote
    description: "Enables remote connection for real-time session relay."
    non_interactive: false
    notes: "Requires Kilo Gateway authentication."
  - name: daemon
    description: "Starts, stops, restarts, or checks the local daemon."
    non_interactive: true
    notes: "Status and lifecycle commands support JSON output."
  - name: console
    description: "Opens or stops the local Kilo Console."
    non_interactive: false
    notes: "Default command opens UI; `console stop --json` is inspectable."
  - name: db
    description: "Runs database tools, prints the database path, or migrates JSON data to SQLite."
    non_interactive: true
    notes: "Bare `kilo db` opens an interactive sqlite shell; pass a query and `--format json` for automation."
  - name: config
    description: "Runs configuration tools."
    non_interactive: true
    notes: "`config check` prints configuration warnings and errors."
  - name: plugin
    description: "Installs a plugin and updates config."
    non_interactive: false
    notes: "Runs npm package installation and mutates config."
  - name: help
    description: "Shows full CLI reference."
    non_interactive: true
    notes: "Supports text and Markdown formats."
  - name: completion
    description: "Generates shell completion scripts."
    non_interactive: true
    notes: "Current public help does not list a shell selector flag."
cli_switches:
  - flag: --help
    value: ""
    scope: ["global"]
    default: "false"
    description: "Shows help."
    example: "kilo --help"
    notes: "Short form `-h` is documented in the overview; generated command help lists `--help`."
  - flag: --version
    value: ""
    scope: ["global"]
    default: "false"
    description: "Shows version number."
    example: "kilo --version"
    notes: "Short form `-v` is documented in the overview; generated command help lists `--version`."
  - flag: --print-logs
    value: ""
    scope: ["global", "logging"]
    default: "false"
    description: "Prints logs to stderr."
    example: "kilo --print-logs"
    notes: "Documented in overview global options, not repeated in generated command help."
  - flag: --log-level
    value: "DEBUG | INFO | WARN | ERROR"
    scope: ["global", "logging"]
    default: "unknown"
    description: "Sets CLI log level."
    example: "kilo --log-level DEBUG"
    notes: "Config schema defines the same enum."
  - flag: --command
    value: "<command>"
    scope: ["run"]
    default: ""
    description: "Command to run; use message for args."
    example: "kilo run --command review"
    notes: ""
  - flag: --continue
    value: ""
    scope: ["run", "attach", "sessions"]
    default: "false"
    description: "Continues the last session."
    example: "kilo run --continue"
    notes: "Short form `-c`."
  - flag: --session
    value: "<id>"
    scope: ["run", "attach", "sessions"]
    default: ""
    description: "Session id to continue."
    example: "kilo run --session ses_123"
    notes: "Short form `-s`."
  - flag: --fork
    value: ""
    scope: ["run", "attach", "sessions"]
    default: "false"
    description: "Forks the session before continuing."
    example: "kilo run --session ses_123 --fork"
    notes: "Requires `--continue` or `--session`."
  - flag: --cloud-fork
    value: ""
    scope: ["run", "attach", "sessions"]
    default: "false"
    description: "Fetches a cloud session and continues it locally."
    example: "kilo run --session ses_123 --cloud-fork"
    notes: "Requires `--session`."
  - flag: --share
    value: ""
    scope: ["run", "sessions"]
    default: "false"
    description: "Shares the session."
    example: "kilo run --share 'summarize this repo'"
    notes: ""
  - flag: --model
    value: "<provider/model>"
    scope: ["run", "agent create", "model_selection"]
    default: "config or provider default"
    description: "Selects the model in provider/model format."
    example: "kilo run --model anthropic/claude-sonnet-4.6 'review this diff'"
    notes: "Short form `-m`."
  - flag: --agent
    value: "<name>"
    scope: ["run"]
    default: "config default"
    description: "Selects the active agent."
    example: "kilo run --agent plan 'design this migration'"
    notes: ""
  - flag: --format
    value: "default | json"
    scope: ["run", "output"]
    default: "default"
    description: "Selects formatted output or raw JSON events."
    example: "kilo run --format json --auto 'summarize this repo'"
    notes: "`json` is the wrapper-grade event stream."
  - flag: --file
    value: "<path>"
    scope: ["run", "context"]
    default: "[]"
    description: "Attaches file(s) to the message."
    example: "kilo run --file src/lib.rs 'explain this file'"
    notes: "Short form `-f`; repeatable/array."
  - flag: --title
    value: "<title>"
    scope: ["run", "sessions"]
    default: "truncated prompt"
    description: "Sets the session title."
    example: "kilo run --title 'CI review' 'review this diff'"
    notes: ""
  - flag: --attach
    value: "<url>"
    scope: ["run", "server"]
    default: ""
    description: "Attaches the run to a running Kilo server."
    example: "kilo run --attach http://localhost:4096 'continue work'"
    notes: ""
  - flag: --password
    value: "<password>"
    scope: ["run", "attach", "server_auth"]
    default: "KILO_SERVER_PASSWORD"
    description: "Basic auth password for server attachment."
    example: "kilo run --attach http://localhost:4096 --password \"$KILO_SERVER_PASSWORD\" 'status'"
    notes: "Short form `-p`."
  - flag: --username
    value: "<username>"
    scope: ["run", "attach", "server_auth"]
    default: "KILO_SERVER_USERNAME or kilo"
    description: "Basic auth username for server attachment."
    example: "kilo attach http://localhost:4096 --username kilo"
    notes: "Short form `-u`."
  - flag: --dir
    value: "<path>"
    scope: ["run", "attach"]
    default: "current directory"
    description: "Directory to run in; when attaching, this is the path on the remote server."
    example: "kilo run --dir /repo 'inspect this project'"
    notes: ""
  - flag: --port
    value: "<number>"
    scope: ["run", "acp", "serve", "web", "daemon", "console", "server"]
    default: "0 for server commands; random for run when no value is provided"
    description: "Sets the server listen port."
    example: "kilo serve --port 4096"
    notes: "Default 0 means choose a free port for server commands."
  - flag: --variant
    value: "<variant>"
    scope: ["run", "model_selection"]
    default: ""
    description: "Selects a provider-specific model variant or reasoning effort."
    example: "kilo run --variant high 'solve this bug'"
    notes: "Examples include high, max, and minimal."
  - flag: --thinking
    value: ""
    scope: ["run", "output"]
    default: "false"
    description: "Shows thinking blocks."
    example: "kilo run --thinking 'debug this failure'"
    notes: ""
  - flag: --replay
    value: ""
    scope: ["run", "sessions"]
    default: "false"
    description: "Replays visible session history on interactive resume."
    example: "kilo run --continue --interactive --replay"
    notes: "Present in @kilocode/cli 7.3.54 help."
  - flag: --replay-limit
    value: "<number>"
    scope: ["run", "sessions"]
    default: ""
    description: "Caps visible interactive replay to the newest N messages."
    example: "kilo run --continue --interactive --replay-limit 20"
    notes: "Present in @kilocode/cli 7.3.54 help."
  - flag: --interactive
    value: ""
    scope: ["run"]
    default: "false"
    description: "Runs in direct interactive split-footer mode."
    example: "kilo run --interactive 'start here'"
    notes: "Short form `-i`."
  - flag: --dangerously-skip-permissions
    value: ""
    scope: ["run", "permissions"]
    default: "false"
    description: "Auto-approves permissions that are not explicitly denied."
    example: "kilo run --dangerously-skip-permissions 'fix lint'"
    notes: "Dangerous for wrappers unless paired with restrictive config."
  - flag: --auto
    value: ""
    scope: ["run", "automation", "permissions"]
    default: "false"
    description: "Auto-approves all permissions for autonomous/pipeline usage."
    example: "kilo run --auto 'implement feature X'"
    notes: "Official non-interactive mode."
  - flag: --demo
    value: ""
    scope: ["run"]
    default: "false"
    description: "Enables direct interactive demo slash commands."
    example: "kilo run --demo"
    notes: "If a demo command is passed as the message, it runs immediately."
  - flag: --hostname
    value: "<host>"
    scope: ["acp", "serve", "web", "daemon", "console", "server"]
    default: "127.0.0.1"
    description: "Sets server listen hostname."
    example: "kilo serve --hostname 127.0.0.1"
    notes: ""
  - flag: --mdns
    value: ""
    scope: ["acp", "serve", "web", "daemon", "console", "server"]
    default: "false"
    description: "Enables mDNS service discovery."
    example: "kilo serve --mdns"
    notes: "Generated help says this defaults hostname to 0.0.0.0."
  - flag: --mdns-domain
    value: "<domain>"
    scope: ["acp", "serve", "web", "daemon", "console", "server"]
    default: "kilo.local"
    description: "Sets custom mDNS service domain."
    example: "kilo serve --mdns-domain kilo.local"
    notes: ""
  - flag: --cors
    value: "<domain>"
    scope: ["acp", "serve", "web", "daemon", "console", "server"]
    default: "[]"
    description: "Adds domains allowed for CORS."
    example: "kilo serve --cors https://example.com"
    notes: "Array option."
  - flag: --cwd
    value: "<path>"
    scope: ["acp"]
    default: "current directory"
    description: "Sets ACP working directory."
    example: "kilo acp --cwd /repo"
    notes: ""
  - flag: --provider
    value: "<id-or-name>"
    scope: ["auth login"]
    default: ""
    description: "Provider id or name to log into, skipping provider selection."
    example: "kilo auth login --provider anthropic"
    notes: "Short form `-p`."
  - flag: --method
    value: "<label>"
    scope: ["auth login", "upgrade"]
    default: ""
    description: "For auth login, selects login method; for upgrade, selects installation method."
    example: "kilo upgrade --method npm"
    notes: "Upgrade choices: curl, npm, yarn, pnpm, bun, brew, choco, scoop."
  - flag: --query
    value: "<query>"
    scope: ["debug rg files"]
    default: ""
    description: "Filters files by query."
    example: "kilo debug rg files --query cli"
    notes: ""
  - flag: --glob
    value: "<glob>"
    scope: ["debug rg files", "debug rg search"]
    default: ""
    description: "Filters files or searches by glob pattern."
    example: "kilo debug rg search TODO --glob '*.rs'"
    notes: "Array for `debug rg search`."
  - flag: --limit
    value: "<number>"
    scope: ["debug rg tree", "debug rg files", "debug rg search"]
    default: ""
    description: "Limits debug ripgrep output."
    example: "kilo debug rg files --limit 100"
    notes: ""
  - flag: --tool
    value: "<id>"
    scope: ["debug agent"]
    default: ""
    description: "Tool id to execute while debugging an agent."
    example: "kilo debug agent build --tool read"
    notes: ""
  - flag: --params
    value: "<json-or-js-object>"
    scope: ["debug agent"]
    default: ""
    description: "Tool params as JSON or a JavaScript object literal."
    example: "kilo debug agent build --tool read --params '{path:\"README.md\"}'"
    notes: ""
  - flag: --path
    value: "<path>"
    scope: ["agent create"]
    default: ""
    description: "Directory path where the agent file is generated."
    example: "kilo agent create --path .kilo/agent"
    notes: ""
  - flag: --description
    value: "<text>"
    scope: ["agent create"]
    default: ""
    description: "Description of what the agent should do."
    example: "kilo agent create --description 'Reviews database migrations'"
    notes: ""
  - flag: --mode
    value: "all | primary | subagent"
    scope: ["agent create"]
    default: ""
    description: "Sets agent mode."
    example: "kilo agent create --mode subagent"
    notes: ""
  - flag: --permissions
    value: "<permissions>"
    scope: ["agent create"]
    default: "all"
    description: "Comma-separated list of permissions to allow."
    example: "kilo agent create --permissions read,grep --mode subagent"
    notes: "Alias: `--tools`."
  - flag: --tools
    value: "<permissions-or-count>"
    scope: ["agent create", "stats"]
    default: "agent create: all; stats: all"
    description: "Alias for agent-create permissions, or number of tools to show in stats."
    example: "kilo stats --tools 10"
    notes: "Context-dependent flag."
  - flag: --keep-config
    value: ""
    scope: ["uninstall"]
    default: "false"
    description: "Keeps configuration files during uninstall."
    example: "kilo uninstall --keep-config"
    notes: "Short form `-c`."
  - flag: --keep-data
    value: ""
    scope: ["uninstall"]
    default: "false"
    description: "Keeps session data and snapshots during uninstall."
    example: "kilo uninstall --keep-data"
    notes: "Short form `-d`."
  - flag: --dry-run
    value: ""
    scope: ["uninstall"]
    default: "false"
    description: "Shows what uninstall would remove without removing it."
    example: "kilo uninstall --dry-run"
    notes: ""
  - flag: --force
    value: ""
    scope: ["uninstall", "plugin"]
    default: "false"
    description: "Skips confirmation prompts for uninstall, or replaces an existing plugin version."
    example: "kilo uninstall --force"
    notes: "Short form `-f` for uninstall and plugin."
  - flag: --verbose
    value: ""
    scope: ["models", "roll-call"]
    default: "false"
    description: "Shows more verbose output."
    example: "kilo models anthropic --verbose"
    notes: "For models, includes metadata like costs."
  - flag: --refresh
    value: ""
    scope: ["models"]
    default: "false"
    description: "Refreshes the model cache from models.dev."
    example: "kilo models --refresh"
    notes: ""
  - flag: --prompt
    value: "<text>"
    scope: ["roll-call"]
    default: "Hello"
    description: "Prompt sent to each model during roll-call."
    example: "kilo roll-call 'anthropic/.*' --prompt 'ping'"
    notes: ""
  - flag: --timeout
    value: "<milliseconds>"
    scope: ["roll-call"]
    default: "25000"
    description: "Timeout for each model call."
    example: "kilo roll-call 'openai/.*' --timeout 10000"
    notes: ""
  - flag: --parallel
    value: "<number>"
    scope: ["roll-call"]
    default: "5"
    description: "Number of parallel model calls."
    example: "kilo roll-call 'google/.*' --parallel 3"
    notes: ""
  - flag: --quiet
    value: ""
    scope: ["roll-call"]
    default: "false"
    description: "Suppresses progress and decoration."
    example: "kilo roll-call 'anthropic/.*' --output json --quiet"
    notes: ""
  - flag: --output
    value: "table | json | md"
    scope: ["roll-call"]
    default: "table"
    description: "Selects roll-call output format."
    example: "kilo roll-call 'anthropic/.*' --output json"
    notes: ""
  - flag: --json
    value: ""
    scope: ["profile", "daemon", "daemon status", "daemon stop", "console stop"]
    default: "false"
    description: "Prints command output as JSON."
    example: "kilo daemon status --json"
    notes: "Available on selected introspection/status commands."
  - flag: --days
    value: "<number>"
    scope: ["stats"]
    default: "all time"
    description: "Shows stats for the last N days."
    example: "kilo stats --days 7"
    notes: ""
  - flag: --models
    value: "<number>"
    scope: ["stats"]
    default: "hidden"
    description: "Shows model statistics; pass a number for top N, otherwise all."
    example: "kilo stats --models 10"
    notes: ""
  - flag: --project
    value: "<project>"
    scope: ["stats"]
    default: "all projects"
    description: "Filters stats by project."
    example: "kilo stats --project ''"
    notes: "Empty string means current project."
  - flag: --sanitize
    value: ""
    scope: ["export"]
    default: "false"
    description: "Redacts sensitive transcript and file data."
    example: "kilo export ses_123 --sanitize"
    notes: ""
  - flag: --event
    value: "<event>"
    scope: ["github run"]
    default: ""
    description: "GitHub mock event to run the agent for."
    example: "kilo github run --event pull_request"
    notes: ""
  - flag: --token
    value: "<github_pat>"
    scope: ["github run"]
    default: ""
    description: "GitHub personal access token."
    example: "kilo github run --token github_pat_..."
    notes: "Credential-bearing flag; wrappers should avoid logging."
  - flag: --max-count
    value: "<number>"
    scope: ["session list"]
    default: ""
    description: "Limits session list to N most recent sessions."
    example: "kilo session list --max-count 20 --format json"
    notes: "Short form `-n`."
  - flag: --format
    value: "table | json"
    scope: ["session list"]
    default: "table"
    description: "Selects session-list output format."
    example: "kilo session list --format json"
    notes: ""
  - flag: --all
    value: ""
    scope: ["session list", "help"]
    default: "false"
    description: "For session list, includes all projects; for help, shows all commands."
    example: "kilo session list --all --format json"
    notes: "Short form `-a` for session list."
  - flag: --search
    value: "<text>"
    scope: ["session list"]
    default: ""
    description: "Filters sessions by title."
    example: "kilo session list --search migration"
    notes: "Short form `-s`."
  - flag: --foreground
    value: ""
    scope: ["daemon", "daemon start", "daemon restart", "console"]
    default: "false"
    description: "Keeps the command active until interrupted."
    example: "kilo daemon start --foreground"
    notes: "Short form `-f`."
  - flag: --format
    value: "json | tsv"
    scope: ["db"]
    default: "tsv"
    description: "Selects database query output format."
    example: "kilo db 'select * from session limit 5' --format json"
    notes: "Only applies when a SQL query is provided."
  - flag: --global
    value: ""
    scope: ["plugin"]
    default: "false"
    description: "Installs plugin in global config."
    example: "kilo plugin @scope/plugin --global"
    notes: "Short form `-g`."
  - flag: --format
    value: "md | text"
    scope: ["help"]
    default: "md"
    description: "Selects help output format."
    example: "kilo help run --format text"
    notes: ""
config_files:
  - os: macos
    scope: user
    path: "~/.config/kilo/kilo.jsonc"
    format: jsonc
    notes: "Official CLI 1.0 docs document ~/.config/kilo/kilo.json[c]. Local macOS inspection with HOME=/Users/ken/.claudine resolved config to $HOME/.config/kilo."
  - os: linux
    scope: user
    path: "~/.config/kilo/kilo.jsonc"
    format: jsonc
    notes: "Primary user config path documented for Kilo CLI 1.0."
  - os: windows
    scope: user
    path: "%APPDATA%\\kilo\\kilo.jsonc"
    format: jsonc
    notes: "Official docs say Windows config dir may vary; this is the expected app-data equivalent from sibling Kilo permission research."
  - os: macos
    scope: user
    path: "~/.config/kilo/tui.jsonc"
    format: jsonc
    notes: "TUI notifications, sounds, themes, and keybindings; `.json` is also supported."
  - os: linux
    scope: user
    path: "~/.config/kilo/tui.jsonc"
    format: jsonc
    notes: "TUI notifications, sounds, themes, and keybindings; `.json` is also supported."
  - os: windows
    scope: user
    path: "%APPDATA%\\kilo\\tui.jsonc"
    format: jsonc
    notes: "Expected Windows equivalent for TUI config; docs say Windows config dir may vary."
  - os: macos
    scope: repo
    path: "./kilo.jsonc"
    format: jsonc
    notes: "Project config; project-level configuration takes precedence over global settings. `.json` is also supported."
  - os: linux
    scope: repo
    path: "./kilo.jsonc"
    format: jsonc
    notes: "Project config; project-level configuration takes precedence over global settings. `.json` is also supported."
  - os: windows
    scope: repo
    path: ".\\kilo.jsonc"
    format: jsonc
    notes: "Project config; project-level configuration takes precedence over global settings. `.json` is also supported."
  - os: macos
    scope: repo
    path: "./.kilo/"
    format: other
    notes: "Project directory for config/resources; legacy `./.kilocode/` and `./.opencode/` are also read."
  - os: linux
    scope: repo
    path: "./.kilo/"
    format: other
    notes: "Project directory for config/resources; legacy `./.kilocode/` and `./.opencode/` are also read."
  - os: windows
    scope: repo
    path: ".\\.kilo\\"
    format: other
    notes: "Project directory for config/resources; legacy `.kilocode` and `.opencode` are also read."
  - os: macos
    scope: user
    path: "~/.config/kilo/opencode.jsonc"
    format: jsonc
    notes: "Legacy global config file read for OpenCode compatibility; `.json` is also supported."
  - os: linux
    scope: user
    path: "~/.config/kilo/opencode.jsonc"
    format: jsonc
    notes: "Legacy global config file read for OpenCode compatibility; `.json` is also supported."
  - os: windows
    scope: user
    path: "%APPDATA%\\kilo\\opencode.jsonc"
    format: jsonc
    notes: "Expected Windows legacy config equivalent; `.json` is also supported."
env_vars:
  - name: KILO_CONFIG
    effect: "Points the CLI at a custom config file."
  - name: KILO_CONFIG_CONTENT
    effect: "Provides inline JSON/JSONC config content for the current process."
  - name: KILO_DISABLE_PROJECT_CONFIG
    effect: "Skips project-level config discovery."
  - name: KILO_PROVIDER
    effect: "Overrides the active provider ID."
  - name: KILO_ORG_ID
    effect: "Selects a Kilo organization for non-interactive `kilo run`."
  - name: KILO_SERVER_PASSWORD
    effect: "Default basic auth password for `run --attach` and `attach`."
  - name: KILO_SERVER_USERNAME
    effect: "Default basic auth username for `run --attach` and `attach`; falls back to `kilo`."
  - name: KILO_<FIELD_NAME>
    effect: "Overrides provider/config fields for non-kilocode providers, for example KILO_API_KEY -> apiKey."
  - name: KILOCODE_<FIELD_NAME>
    effect: "Overrides fields for the `kilocode` provider, for example KILOCODE_MODEL -> kilocodeModel."
  - name: NO_COLOR
    effect: "Disables color in common terminal tooling; local Kilo inspection honored the environment by producing plain command output except for error styling."
machine_introspection:
  - command: "kilo models [provider] --verbose"
    purpose: models
    machine_readable: false
    output_format: text
    useful_for_codegen: true
    notes: "Lists provider/model catalog and optional metadata. Current help exposes no JSON mode; provider filters can fail non-zero when unavailable."
  - command: "kilo session list --format json"
    purpose: other
    machine_readable: true
    output_format: json
    useful_for_codegen: false
    notes: "Lists sessions, optionally with `--all`, `--max-count`, and `--search`."
  - command: "kilo daemon status --json"
    purpose: doctor
    machine_readable: true
    output_format: json
    useful_for_codegen: false
    notes: "Reports daemon running/stale state and state-file path."
  - command: "kilo daemon start --json"
    purpose: doctor
    machine_readable: true
    output_format: json
    useful_for_codegen: false
    notes: "Starts daemon and prints details; long-running behavior depends on foreground/background mode."
  - command: "kilo debug config"
    purpose: config_dump
    machine_readable: true
    output_format: json
    useful_for_codegen: true
    notes: "Prints resolved configuration as JSON; may include user-authored agent prompts and should be treated as sensitive."
  - command: "kilo debug paths"
    purpose: env
    machine_readable: false
    output_format: table
    useful_for_codegen: false
    notes: "Shows home, data, bin, log, repo, cache, config, state, and temp directories."
  - command: "kilo config check"
    purpose: doctor
    machine_readable: false
    output_format: text
    useful_for_codegen: false
    notes: "Reports configuration warnings/errors; observed success text: `No config warnings.`"
  - command: "kilo db path"
    purpose: env
    machine_readable: false
    output_format: text
    useful_for_codegen: false
    notes: "Prints the SQLite database path."
  - command: "kilo db '<SQL>' --format json"
    purpose: other
    machine_readable: true
    output_format: json
    useful_for_codegen: false
    notes: "Runs a SQL query against the local Kilo SQLite database. Bare `kilo db` opens an interactive sqlite shell and should be avoided by wrappers."
  - command: "kilo profile --json"
    purpose: env
    machine_readable: true
    output_format: json
    useful_for_codegen: false
    notes: "Prints Kilo account profile when authenticated; exits non-zero when not authenticated."
  - command: "kilo export <sessionID> [--sanitize]"
    purpose: other
    machine_readable: true
    output_format: json
    useful_for_codegen: false
    notes: "Exports a session transcript/state bundle; `--sanitize` redacts sensitive transcript and file data."
  - command: "kilo roll-call <filter> --output json --quiet"
    purpose: models
    machine_readable: true
    output_format: json
    useful_for_codegen: false
    notes: "Performs live provider calls to test connectivity and latency; useful for diagnostics, not static metadata."
  - command: "kilo help --all --format md"
    purpose: help
    machine_readable: false
    output_format: text
    useful_for_codegen: false
    notes: "Documents the CLI surface. Included because the generated Markdown help is the closest available command schema, but it is not machine-readable."
wrapper_notes:
  - "Primary wrapper entry point is `kilo run --auto`; without `--auto` the CLI can prompt for approvals and follow-up questions."
  - "`kilo run --format json` emits raw JSON events and is the best real-time stream for Claudine wrappers."
  - "`--auto` and `--dangerously-skip-permissions` change approval behavior; wrappers should pair them with explicit Kilo permission config rather than assuming a safe default."
  - "Authentication commands and `/connect` are interactive; `profile --json` is a safe auth-state probe but exits non-zero when unauthenticated."
  - "The CLI stores sessions in SQLite (`kilo.db`) rather than per-session JSONL files; read it via read-only queries or Kilo export commands, not by copying live WAL files."
  - "Local inspection was run with HOME=/Users/ken/.claudine, and Kilo resolved its XDG-style paths under that HOME; wrapper HOME shadowing changes config/data discovery."
  - "Official docs list more top-level commands than `kilo help --format md` printed locally; per-command help and the public CLI reference are more complete."
  - "@kilocode/cli 7.3.54 added `kilo run --replay` and `--replay-limit` compared with the locally installed 7.3.45."
  - "Older x64 CPUs without AVX may need the documented baseline release assets; wrappers should not assume the normal native package works on all x64 hosts."
  - "`kilo db` without a SQL query opens an interactive sqlite shell; wrappers must pass a query and `--format json` or avoid it."
  - "Commands that open UI or browsers (`web`, `console`, some auth flows) are not suitable for non-interactive wrapper runs."
changes: []
requires_claudine_update: true
reason: "Claudine's compiled provider enum does not yet support Kilo Code. Kilo is now a first-class CLI with an OpenCode-like JSON event stream, distinct binary aliases, config discovery, model/provider catalog commands, SQLite session storage, and permission/auth semantics that require provider metadata and wrapper implementation."
---

# Kilo Code CLI

## Overview

Kilo Code is an agentic coding platform available in IDEs and as a terminal CLI. The public CLI package is `@kilocode/cli`; npm reported `7.3.54` as the latest stable version on 2026-07-02, while the local host had `7.3.45` installed. The package exposes both `kilo` and `kilocode`, but official documentation uses `kilo`.

For Claudine, Kilo should be treated as an OpenCode-family CLI rather than a Roo-style IDE-only provider. The wrapper-grade path is `kilo run --auto --format json`, with Kilo config and permissions supplied through config files or environment overrides.

## Installation and Binaries

Official docs document npm as the primary install:

```sh
npm install -g @kilocode/cli
```

The current npm package exposes two command names:

| Binary | Notes |
|--------|-------|
| `kilo` | Primary documented command. |
| `kilocode` | npm alias mapped to the same `bin/kilo` entry. |

For older x64 CPUs without AVX support, the docs instruct users to download baseline release assets directly from GitHub releases:

| OS | Baseline asset documented |
|----|---------------------------|
| Linux x64 | `kilo-linux-x64-baseline.tar.gz` |
| macOS x64 | `kilo-darwin-x64-baseline.zip` |
| Windows x64 | `kilo-windows-x64-baseline.zip` |

## Subcommands

The public CLI reference documents these top-level commands: `acp`, `mcp`, default TUI (`kilo [project]`), `attach`, `run`, `debug`, `auth`, `agent`, `upgrade`, `uninstall`, `serve`, `web`, `models`, `roll-call`, `profile`, `stats`, `export`, `import`, `github`, `pr`, `session`, `remote`, `daemon`, `console`, `db`, `config`, `plugin`, `help`, and `completion`.

The locally installed and npx-inspected help output printed only the older core command set at the top level, but per-command help for current `7.3.54` confirmed newer commands such as `models`, `profile`, `stats`, `session`, `daemon`, `db`, `agent`, `upgrade`, `uninstall`, `serve`, `web`, `github`, `remote`, `console`, and `plugin`.

## CLI Switch Inventory

The full switch inventory is captured in frontmatter. The wrapper-critical subset is:

| Scope | Switches |
|-------|----------|
| non-interactive run | `kilo run --auto --format json [--model provider/model] [--agent name] [--file path] [--dir path] <message>` |
| permission bypass | `--auto`, `--dangerously-skip-permissions` |
| session continuation | `--continue`, `--session`, `--fork`, `--cloud-fork`, `--replay`, `--replay-limit` |
| remote attach | `--attach`, `--username`, `--password`, `--dir` |
| server commands | `--port`, `--hostname`, `--mdns`, `--mdns-domain`, `--cors`, `--json` where supported |
| introspection | `models --verbose`, `session list --format json`, `daemon status --json`, `debug config`, `debug paths`, `db '<SQL>' --format json` |

## Configuration Discovery

Official docs say Kilo CLI 1.0 uses `~/.config/kilo/kilo.json[c]` globally and `./kilo.json[c]` for project config. Legacy OpenCode names are also read: `opencode.json[c]` and `.opencode/`. Kilo also reads project resources from `.kilo/`, with legacy `.kilocode/` support.

TUI-specific settings such as attention notifications, sounds, themes, and keybindings live in `tui.jsonc` or `tui.json` under the global config directory or project `.kilo/` directory.

Local inspection with `HOME=/Users/ken/.claudine` showed Kilo resolving paths under that HOME:

| Kind | Observed path |
|------|---------------|
| config | `/Users/ken/.claudine/.config/kilo` |
| data | `/Users/ken/.claudine/.local/share/kilo` |
| cache | `/Users/ken/.claudine/.cache/kilo` |
| state | `/Users/ken/.claudine/.local/state/kilo` |
| logs | `/Users/ken/.claudine/.local/share/kilo/log` |

That matters for Claudine because a shadow HOME changes Kilo's config, credential, model cache, and database discovery.

## Environment Variables

Kilo supports config and provider overrides from environment variables. The current general wrapper-relevant set is in frontmatter. Environment variables that are better owned by narrower topics, such as logging/OTel and detailed permission sandboxing, are not exhaustively duplicated here.

For non-interactive organization routing, official docs say there is no `--org` or `--team` flag on `kilo run`; use `KILO_ORG_ID` or a previously persisted `/teams` selection.

## Machine Introspection

The useful machine-facing commands are mixed quality:

| Command | Output | Notes |
|---------|--------|-------|
| `kilo run --format json --auto ...` | JSON events | Primary live wrapper stream. |
| `kilo debug config` | JSON | Resolved config; sensitive. |
| `kilo session list --format json` | JSON | Session inventory. |
| `kilo daemon status --json` | JSON | Daemon state. |
| `kilo db '<SQL>' --format json` | JSON | Direct SQLite query. Avoid bare `kilo db`. |
| `kilo export <sessionID> [--sanitize]` | JSON | Session export. |
| `kilo profile --json` | JSON | Authenticated account state; non-zero when unauthenticated. |
| `kilo models [provider] --verbose` | text | Model catalog; useful but not structured. |
| `kilo roll-call <filter> --output json --quiet` | JSON | Live connectivity diagnostics; makes model calls. |

## Wrapper Notes

Kilo is promising for Claudine because it has a documented non-interactive autonomous mode and a JSON event stream. The main wrapper risks are permission posture, auth interactivity, HOME/config shadowing, and SQLite-backed session storage.

Do not launch `kilo db` without a query; it opens an interactive sqlite shell. Do not use `web`, default `console`, or login flows in a non-interactive wrapper. Treat `--auto` as an automation mode, not a security mode.

## Sources

- [Kilo Code homepage](https://kilo.ai/)
- [Kilo Code CLI docs](https://kilo.ai/docs/code-with-ai/platforms/cli)
- [Kilo CLI command reference](https://kilo.ai/docs/code-with-ai/platforms/cli-reference)
- [Kilo Code repository](https://github.com/Kilo-Org/kilocode)
- [npm package: @kilocode/cli](https://www.npmjs.com/package/@kilocode/cli)
- [Kilo config JSON Schema](https://app.kilo.ai/config.json)
- Local CLI inspection on 2026-07-02: `kilo --version` = `7.3.45`; `npx --yes @kilocode/cli@7.3.54 --version` = `7.3.54`; `npm view @kilocode/cli version dist-tags --json` reported latest `7.3.54` and rc `7.3.63`.
