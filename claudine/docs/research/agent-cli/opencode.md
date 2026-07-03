---
$schema: ./_schema.yaml
created: 2026-05-12
last_updated: 2026-07-02
agent: codex
model: default
latest_version: "1.17.13"
homepage: https://opencode.ai
repo: https://github.com/anomalyco/opencode
docs: https://opencode.ai/docs/
cli_docs: https://opencode.ai/docs/cli/
binaries:
  - os: all
    binary: opencode
    alt_binaries: []
    notes: "Primary command documented for macOS, Linux, and Windows package-manager installs. Local macOS inspection found /Users/ken/.opencode/bin/opencode reporting 1.17.13."
  - os: windows
    binary: opencode.exe
    alt_binaries: ["opencode.cmd"]
    notes: "The npm package exposes bin.opencode as bin/opencode.exe. Windows package managers install the opencode command; shim names depend on npm, Scoop, or Chocolatey."
install_methods:
  - os: macos
    method: standalone_binary
    command: "curl -fsSL https://opencode.ai/install | bash"
    notes: "Official installer defaults to $HOME/.opencode/bin and can install a specific version with --version."
  - os: linux
    method: standalone_binary
    command: "curl -fsSL https://opencode.ai/install | bash"
    notes: "Official installer defaults to $HOME/.opencode/bin and downloads linux-x64/linux-arm64 archives, with baseline and musl variants when needed."
  - os: windows
    method: scoop
    command: "scoop install opencode"
    notes: "Documented Windows package-manager install."
  - os: windows
    method: chocolatey
    command: "choco install opencode"
    notes: "Documented Windows package-manager install. Chocolatey upgrades/uninstalls may require an elevated shell."
  - os: macos
    method: brew
    command: "brew install anomalyco/tap/opencode"
    notes: "README calls the tap formula recommended and always up to date."
  - os: linux
    method: brew
    command: "brew install anomalyco/tap/opencode"
    notes: "Homebrew on Linux is documented in the README."
  - os: macos
    method: brew
    command: "brew install opencode"
    notes: "Official Homebrew formula is documented but noted as updated less often than anomalyco/tap/opencode."
  - os: linux
    method: brew
    command: "brew install opencode"
    notes: "Official Homebrew formula is documented but noted as updated less often than anomalyco/tap/opencode."
  - os: all
    method: npm
    command: "npm i -g opencode-ai@latest"
    notes: "README says npm, bun, pnpm, or yarn can install the opencode-ai package globally."
  - os: linux
    method: package_manager
    command: "sudo pacman -S opencode"
    notes: "Documented Arch Linux stable package."
  - os: linux
    method: package_manager
    command: "paru -S opencode-bin"
    notes: "Documented Arch Linux AUR package for the latest release."
subcommands:
  - name: tui
    description: "Default mode when no subcommand is supplied; starts the terminal UI for an optional project path."
    non_interactive: false
    notes: "Documented as opencode [project]."
  - name: run
    description: "Runs OpenCode with a message and exits unless direct interactive mode is requested."
    non_interactive: true
    notes: "Primary automation entry point. Supports --format json NDJSON output."
  - name: attach
    description: "Attaches a TUI or mini session to a running OpenCode server."
    non_interactive: false
    notes: "Requires a server URL and usually a TTY."
  - name: acp
    description: "Starts an ACP Agent Client Protocol server over stdin/stdout."
    non_interactive: true
    notes: "Uses NDJSON framing from the ACP SDK."
  - name: mcp
    description: "Manages MCP servers."
    non_interactive: false
    notes: "Subcommands include add, list/ls, auth, logout, and debug. OAuth flows are interactive."
  - name: agent
    description: "Creates and lists OpenCode agents."
    non_interactive: false
    notes: "agent create can be fully non-interactive only when path, description, mode, and permissions are supplied."
  - name: plugin
    description: "Installs a plugin and updates config."
    non_interactive: false
    notes: "Alias: plug. Mutates global or project config and may run package installation."
  - name: pr
    description: "Fetches and checks out a GitHub PR branch, then runs OpenCode."
    non_interactive: false
    notes: "Uses gh CLI and then launches the TUI."
  - name: db
    description: "Runs database tools or prints the database path."
    non_interactive: true
    notes: "db without a query opens sqlite3 interactively; db path and db <query> are suitable for automation."
  - name: debug
    description: "Runs debugging and troubleshooting commands."
    non_interactive: true
    notes: "Subcommands include config, paths, info, skill, scrap, lsp, rg, file, snapshot, startup, agent, v2, and wait."
  - name: session
    description: "Lists and deletes saved sessions."
    non_interactive: true
    notes: "session list supports --format json; delete mutates local state."
  - name: models
    description: "Lists available models, optionally for one provider."
    non_interactive: true
    notes: "Outputs provider/model lines; --verbose prints JSON metadata after each line."
  - name: providers
    description: "Manages provider authentication and configuration."
    non_interactive: false
    notes: "Source exposes provider auth flows; most login flows prompt."
  - name: console
    description: "Manages OpenCode console account login, logout, org switch, org listing, and opening the console."
    non_interactive: false
    notes: "Login uses browser/device-code style prompts."
  - name: serve
    description: "Starts a headless OpenCode HTTP server."
    non_interactive: true
    notes: "Long-running process. Warns when OPENCODE_SERVER_PASSWORD is unset."
  - name: web
    description: "Starts a server and opens the web interface."
    non_interactive: false
    notes: "Long-running and browser-opening."
  - name: generate
    description: "Generates the OpenCode OpenAPI document as JSON."
    non_interactive: true
    notes: "Useful for SDK/API schema inspection."
  - name: stats
    description: "Shows token usage and cost statistics."
    non_interactive: true
    notes: "Human-readable output; source does not expose a JSON flag."
  - name: export
    description: "Exports session data as JSON."
    non_interactive: true
    notes: "Optional --sanitize redacts sensitive transcript and file data."
  - name: import
    description: "Imports session data from a JSON file or share URL."
    non_interactive: true
    notes: "Mutates local session storage."
  - name: github
    description: "Manages the GitHub agent."
    non_interactive: false
    notes: "Subcommands include install and run; github run accepts --event and --token."
  - name: completion
    description: "Generates shell completion scripts."
    non_interactive: true
    notes: "Exposed by yargs completion."
  - name: upgrade
    description: "Upgrades OpenCode to the latest or a specified version."
    non_interactive: false
    notes: "Mutates installation and may prompt when the installation method is unknown."
  - name: uninstall
    description: "Uninstalls OpenCode and related files."
    non_interactive: false
    notes: "Use --force for non-interactive removal; destructive."
cli_switches:
  - flag: --help
    value: ""
    scope: ["global"]
    default: "false"
    description: "Shows help."
    example: "opencode --help"
    notes: "Short form: -h."
  - flag: --version
    value: ""
    scope: ["global"]
    default: "false"
    description: "Prints the OpenCode version."
    example: "opencode --version"
    notes: "Short form: -v."
  - flag: --print-logs
    value: ""
    scope: ["global", "logging"]
    default: "false"
    description: "Prints structured logs to stderr."
    example: "opencode run --format json --print-logs --log-level INFO \"summarize\""
    notes: "Sets OPENCODE_PRINT_LOGS=1 internally."
  - flag: --log-level
    value: "DEBUG | INFO | WARN | ERROR"
    scope: ["global", "logging"]
    default: "unknown"
    description: "Selects log verbosity."
    example: "opencode --log-level DEBUG debug info"
    notes: "Sets OPENCODE_LOG_LEVEL internally."
  - flag: --pure
    value: ""
    scope: ["global", "plugins"]
    default: "false"
    description: "Runs without external plugins."
    example: "opencode --pure run \"inspect this repo\""
    notes: "Sets OPENCODE_PURE=1 internally."
  - flag: --continue
    value: ""
    scope: ["tui", "run", "attach", "session"]
    default: "false"
    description: "Continues the last session."
    example: "opencode run --continue \"next step\""
    notes: "Short form: -c."
  - flag: --session
    value: "<SESSION_ID>"
    scope: ["tui", "run", "attach", "session"]
    default: ""
    description: "Continues a specific session."
    example: "opencode run --session ses_abc \"continue\""
    notes: "Short form: -s."
  - flag: --fork
    value: ""
    scope: ["tui", "run", "attach", "session"]
    default: "false"
    description: "Forks before continuing a session."
    example: "opencode run --session ses_abc --fork \"try another approach\""
    notes: "Requires --continue or --session."
  - flag: --prompt
    value: "<TEXT>"
    scope: ["tui", "input"]
    default: ""
    description: "Initial prompt to use in TUI mode."
    example: "opencode --prompt \"review this repo\""
    notes: "Documented on the TUI command."
  - flag: --model
    value: "<PROVIDER>/<MODEL>"
    scope: ["tui", "run", "agent create", "model_selection"]
    default: "config default"
    description: "Selects the model."
    example: "opencode run --model anthropic/claude-sonnet-4-5 \"fix tests\""
    notes: "Short form: -m."
  - flag: --agent
    value: "<AGENT>"
    scope: ["tui", "run"]
    default: "config/default"
    description: "Selects the primary agent."
    example: "opencode run --agent build \"implement feature\""
    notes: "Subagents are rejected for primary run selection."
  - flag: --auto
    value: ""
    scope: ["tui", "run", "permissions"]
    default: "false"
    description: "Auto-approves permissions that are not explicitly denied."
    example: "opencode run --auto \"apply the change\""
    notes: "Dangerous for wrappers unless policy is constrained elsewhere."
  - flag: --yolo
    value: ""
    scope: ["tui", "run", "permissions"]
    default: "false"
    description: "Hidden alias-like permission bypass used by source to enable auto approval."
    example: "opencode run --yolo \"apply the change\""
    notes: "Hidden; prefer --auto when documenting user-facing behavior."
  - flag: --dangerously-skip-permissions
    value: ""
    scope: ["tui", "run", "permissions"]
    default: "false"
    description: "Hidden permission bypass used by source to enable auto approval."
    example: "opencode run --dangerously-skip-permissions \"apply the change\""
    notes: "Hidden and unsafe for general wrapper defaults."
  - flag: --port
    value: "<PORT>"
    scope: ["tui", "run", "serve", "web", "acp", "network"]
    default: "0"
    description: "Port for the local server."
    example: "opencode serve --port 4096"
    notes: "Config server.port is used unless the CLI arg is explicitly set."
  - flag: --hostname
    value: "<HOST>"
    scope: ["tui", "serve", "web", "acp", "network"]
    default: "127.0.0.1"
    description: "Hostname for the local server."
    example: "opencode serve --hostname 0.0.0.0"
    notes: "mDNS can default hostname to 0.0.0.0."
  - flag: --mdns
    value: ""
    scope: ["tui", "serve", "web", "acp", "network"]
    default: "false"
    description: "Enables mDNS service discovery."
    example: "opencode web --mdns"
    notes: "Can be disabled with --no-mdns."
  - flag: --mdns-domain
    value: "<DOMAIN>"
    scope: ["serve", "web", "acp", "network"]
    default: "opencode.local"
    description: "Sets the custom mDNS domain."
    example: "opencode serve --mdns --mdns-domain workstation.local"
    notes: ""
  - flag: --cors
    value: "<ORIGIN>"
    scope: ["serve", "web", "acp", "network"]
    default: "[]"
    description: "Adds browser origins to allow for CORS."
    example: "opencode serve --cors http://localhost:3000"
    notes: "Repeatable."
  - flag: --command
    value: "<COMMAND>"
    scope: ["run", "input"]
    default: ""
    description: "Runs a slash command, using the message as arguments."
    example: "opencode run --command test"
    notes: "Cannot be used with --mini."
  - flag: --share
    value: ""
    scope: ["run", "sharing"]
    default: "false"
    description: "Shares the session."
    example: "opencode run --share \"summarize\""
    notes: "Also affected by config and OPENCODE_AUTO_SHARE."
  - flag: --format
    value: "default | json"
    scope: ["run", "db", "session list", "output"]
    default: "run: default; db: tsv; session list: table"
    description: "Selects output format."
    example: "opencode run --format json \"summarize\""
    notes: "For run, json means NDJSON event lines. For db/session list, json means a JSON document."
  - flag: --file
    value: "<FILE>"
    scope: ["run", "input"]
    default: "[]"
    description: "Attaches files to the message."
    example: "opencode run --file screenshot.png \"analyze this\""
    notes: "Short form: -f. Repeatable. Remote attach uploads local files up to 10 MiB and rejects local directories."
  - flag: --title
    value: "<TITLE>"
    scope: ["run", "session"]
    default: ""
    description: "Sets the session title."
    example: "opencode run --title \"CI failure\" \"debug tests\""
    notes: "Empty value uses a truncated prompt."
  - flag: --attach
    value: "<URL>"
    scope: ["run", "remote"]
    default: ""
    description: "Runs against a running OpenCode server."
    example: "opencode run --attach http://localhost:4096 \"continue\""
    notes: "Skips local instance loading."
  - flag: --password
    value: "<PASSWORD>"
    scope: ["run", "attach", "auth"]
    default: "OPENCODE_SERVER_PASSWORD"
    description: "Basic auth password for a remote server."
    example: "opencode attach http://localhost:4096 --password \"$OPENCODE_SERVER_PASSWORD\""
    notes: "Short form: -p."
  - flag: --username
    value: "<USERNAME>"
    scope: ["run", "attach", "auth"]
    default: "OPENCODE_SERVER_USERNAME or opencode"
    description: "Basic auth username for a remote server."
    example: "opencode attach http://localhost:4096 --username opencode"
    notes: "Short form: -u."
  - flag: --dir
    value: "<PATH>"
    scope: ["run", "attach", "working_directory"]
    default: "current directory"
    description: "Directory to run in; interpreted as remote server path when attaching."
    example: "opencode run --dir packages/app \"inspect\""
    notes: "For local runs, OpenCode changes into the directory before running."
  - flag: --variant
    value: "<VARIANT>"
    scope: ["run", "model_selection"]
    default: ""
    description: "Sets a provider-specific model variant such as reasoning effort."
    example: "opencode run --variant high \"solve\""
    notes: "Examples in source include high, max, and minimal."
  - flag: --thinking
    value: ""
    scope: ["run", "output"]
    default: "false for non-interactive run; true for mini"
    description: "Shows thinking blocks."
    example: "opencode run --thinking --format json \"solve\""
    notes: "Reasoning events in run JSON are opt-in."
  - flag: --interactive
    value: ""
    scope: ["run", "tui"]
    default: "false"
    description: "Runs in direct interactive split-footer mode."
    example: "opencode run --interactive"
    notes: "Short form: -i."
  - flag: --mini
    value: ""
    scope: ["tui", "run", "attach"]
    default: "false"
    description: "Starts the minimal interactive interface."
    example: "opencode --mini"
    notes: "Hidden or restricted in some contexts; requires TTY stdout and cannot be used with --format json."
  - flag: --no-replay
    value: ""
    scope: ["tui", "attach"]
    default: "false"
    description: "Disables mini session history replay on resume and resize."
    example: "opencode attach http://localhost:4096 --mini --no-replay"
    notes: "Requires --mini."
  - flag: --replay-limit
    value: "<N>"
    scope: ["tui", "run", "attach"]
    default: ""
    description: "Caps visible mini replay to the newest N messages."
    example: "opencode --mini --replay-limit 20"
    notes: "Requires --mini and must be a positive integer."
  - flag: --cwd
    value: "<PATH>"
    scope: ["acp"]
    default: "process.cwd()"
    description: "Working directory for the ACP server."
    example: "opencode acp --cwd /repo"
    notes: ""
  - flag: --path
    value: "<DIR>"
    scope: ["agent create"]
    default: "prompted global/project location"
    description: "Directory path where the agent file should be generated."
    example: "opencode agent create --path .opencode --description \"Review Rust\" --mode subagent --permissions read,grep"
    notes: "The command appends agents/ under the supplied path."
  - flag: --description
    value: "<TEXT>"
    scope: ["agent create"]
    default: "prompted"
    description: "Describes what the generated agent should do."
    example: "opencode agent create --description \"Review Rust code\" --mode subagent --permissions read,grep"
    notes: ""
  - flag: --mode
    value: "all | primary | subagent"
    scope: ["agent create"]
    default: "prompted"
    description: "Sets the agent mode."
    example: "opencode agent create --mode primary --description \"Build features\" --permissions bash,read,edit"
    notes: ""
  - flag: --permissions
    value: "<CSV>"
    scope: ["agent create"]
    default: "all"
    description: "Comma-separated permissions to allow."
    example: "opencode agent create --permissions read,grep,glob"
    notes: "Alias: --tools. Available permissions include bash, read, edit, glob, grep, webfetch, task, todowrite, websearch, lsp, and skill."
  - flag: --global
    value: ""
    scope: ["plugin"]
    default: "false"
    description: "Installs a plugin in global config."
    example: "opencode plugin opencode-wakatime --global"
    notes: "Short form: -g."
  - flag: --force
    value: ""
    scope: ["plugin", "uninstall"]
    default: "false"
    description: "For plugin, replaces an existing plugin version. For uninstall, skips confirmation prompts."
    example: "opencode plugin opencode-wakatime --force"
    notes: "Short form: -f."
  - flag: --max-count
    value: "<N>"
    scope: ["session list"]
    default: ""
    description: "Limits session list to N most recent sessions."
    example: "opencode session list --max-count 10 --format json"
    notes: "Short form: -n."
  - flag: --verbose
    value: ""
    scope: ["models"]
    default: "false"
    description: "Prints verbose model metadata."
    example: "opencode models anthropic --verbose"
    notes: "Model ids remain line-oriented; metadata is pretty JSON following each id."
  - flag: --refresh
    value: ""
    scope: ["models"]
    default: "false"
    description: "Refreshes the models cache from models.dev."
    example: "opencode models --refresh"
    notes: "May use network and mutate cache."
  - flag: --days
    value: "<N>"
    scope: ["stats"]
    default: "all time"
    description: "Shows stats for the last N days."
    example: "opencode stats --days 7"
    notes: "0 means today in source."
  - flag: --tools
    value: "<N>"
    scope: ["stats"]
    default: "all"
    description: "Limits the number of tools displayed."
    example: "opencode stats --tools 20"
    notes: ""
  - flag: --models
    value: "[N]"
    scope: ["stats"]
    default: "hidden"
    description: "Shows model statistics, optionally top N."
    example: "opencode stats --models 10"
    notes: ""
  - flag: --project
    value: "<PROJECT_ID>"
    scope: ["stats"]
    default: "all projects"
    description: "Filters stats by project; empty string means current project."
    example: "opencode stats --project \"\""
    notes: ""
  - flag: --sanitize
    value: ""
    scope: ["export"]
    default: "false"
    description: "Redacts sensitive transcript and file data."
    example: "opencode export ses_abc --sanitize"
    notes: ""
  - flag: --event
    value: "<EVENT>"
    scope: ["github run"]
    default: ""
    description: "GitHub mock event to run the agent for."
    example: "opencode github run --event pull_request"
    notes: ""
  - flag: --token
    value: "<TOKEN>"
    scope: ["github run"]
    default: ""
    description: "GitHub personal access token."
    example: "opencode github run --token github_pat_..."
    notes: "Avoid passing secrets in argv from wrappers."
  - flag: --keep-config
    value: ""
    scope: ["uninstall"]
    default: "false"
    description: "Keeps configuration files during uninstall."
    example: "opencode uninstall --keep-config --force"
    notes: "Short form: -c."
  - flag: --keep-data
    value: ""
    scope: ["uninstall"]
    default: "false"
    description: "Keeps session data and snapshots during uninstall."
    example: "opencode uninstall --keep-data --force"
    notes: "Short form: -d."
  - flag: --dry-run
    value: ""
    scope: ["uninstall"]
    default: "false"
    description: "Shows what uninstall would remove without removing it."
    example: "opencode uninstall --dry-run"
    notes: ""
  - flag: --method
    value: "curl | npm | pnpm | bun | brew | choco | scoop"
    scope: ["upgrade"]
    default: "detected"
    description: "Selects the installation method used for upgrade."
    example: "opencode upgrade --method brew"
    notes: "Short form: -m."
config_files:
  - os: all
    scope: user
    path: "~/.config/opencode/opencode.json"
    format: json
    notes: "Global config. OpenCode also accepts opencode.jsonc in config directories."
  - os: all
    scope: user
    path: "~/.config/opencode/opencode.jsonc"
    format: jsonc
    notes: "Global JSONC config; docs show JSON/JSONC are supported."
  - os: all
    scope: user
    path: "~/.config/opencode/tui.json"
    format: json
    notes: "Global TUI-specific config."
  - os: all
    scope: user
    path: "~/.config/opencode/tui.jsonc"
    format: jsonc
    notes: "Global TUI-specific JSONC config."
  - os: all
    scope: repo
    path: "opencode.json"
    format: json
    notes: "Project config. OpenCode starts in the current directory and traverses up to the nearest Git directory."
  - os: all
    scope: repo
    path: "opencode.jsonc"
    format: jsonc
    notes: "Project JSONC config."
  - os: all
    scope: repo
    path: "tui.json"
    format: json
    notes: "Project TUI-specific config alongside opencode.json."
  - os: all
    scope: repo
    path: "tui.jsonc"
    format: jsonc
    notes: "Project TUI-specific JSONC config alongside opencode.json."
  - os: all
    scope: repo
    path: ".opencode/"
    format: other
    notes: "Project directory for agents, commands, plugins, package.json dependencies, and related assets."
  - os: all
    scope: user
    path: "~/.config/opencode/"
    format: other
    notes: "Global directory for agents, commands, plugins, package.json dependencies, and config files."
  - os: all
    scope: env
    path: "$OPENCODE_CONFIG"
    format: json
    notes: "Custom config file loaded between global and project config; format may be JSON or JSONC based on file content."
  - os: all
    scope: env
    path: "$OPENCODE_CONFIG_DIR"
    format: other
    notes: "Custom config directory searched for agents, commands, modes, and plugins."
  - os: all
    scope: env
    path: "$OPENCODE_CONFIG_CONTENT"
    format: json
    notes: "Inline JSON config content loaded with highest normal precedence before managed settings."
  - os: macos
    scope: system
    path: "/Library/Application Support/opencode/opencode.json"
    format: json
    notes: "Managed settings directory."
  - os: macos
    scope: system
    path: "/Library/Application Support/opencode/opencode.jsonc"
    format: jsonc
    notes: "Managed settings directory."
  - os: linux
    scope: system
    path: "/etc/opencode/opencode.json"
    format: json
    notes: "Managed settings directory."
  - os: linux
    scope: system
    path: "/etc/opencode/opencode.jsonc"
    format: jsonc
    notes: "Managed settings directory."
  - os: windows
    scope: system
    path: "%ProgramData%\\opencode\\opencode.json"
    format: json
    notes: "Managed settings directory."
  - os: windows
    scope: system
    path: "%ProgramData%\\opencode\\opencode.jsonc"
    format: jsonc
    notes: "Managed settings directory."
  - os: macos
    scope: system
    path: "/Library/Managed Preferences/<user>/ai.opencode.managed.plist"
    format: other
    notes: "macOS MDM managed preferences; keys map to opencode.json fields."
  - os: macos
    scope: system
    path: "/Library/Managed Preferences/ai.opencode.managed.plist"
    format: other
    notes: "macOS MDM managed preferences; keys map to opencode.json fields."
  - os: all
    scope: user
    path: "~/.local/share/opencode/auth.json"
    format: json
    notes: "Provider credentials are stored here after /connect according to provider docs."
env_vars:
  - name: OPENCODE_AUTO_SHARE
    effect: "Automatically shares sessions."
  - name: OPENCODE_GIT_BASH_PATH
    effect: "Path to Git Bash on Windows; used to locate less.exe for paging and by Windows shell integration."
  - name: OPENCODE_CONFIG
    effect: "Sets a custom config file path."
  - name: OPENCODE_TUI_CONFIG
    effect: "Sets a custom TUI config file path."
  - name: OPENCODE_CONFIG_DIR
    effect: "Sets a custom config directory and overrides the global config path used by runtime services."
  - name: OPENCODE_CONFIG_CONTENT
    effect: "Provides inline JSON config content."
  - name: OPENCODE_DISABLE_AUTOUPDATE
    effect: "Disables automatic update checks."
  - name: OPENCODE_DISABLE_PRUNE
    effect: "Disables pruning of old data."
  - name: OPENCODE_DISABLE_TERMINAL_TITLE
    effect: "Disables automatic terminal title updates."
  - name: OPENCODE_DISABLE_DEFAULT_PLUGINS
    effect: "Disables default plugins."
  - name: OPENCODE_DISABLE_LSP_DOWNLOAD
    effect: "Disables automatic LSP server downloads."
  - name: OPENCODE_DISABLE_AUTOCOMPACT
    effect: "Disables automatic context compaction."
  - name: OPENCODE_DISABLE_CLAUDE_CODE
    effect: "Disables reading from .claude prompt and skills sources."
  - name: OPENCODE_DISABLE_CLAUDE_CODE_PROMPT
    effect: "Disables reading ~/.claude/CLAUDE.md."
  - name: OPENCODE_DISABLE_CLAUDE_CODE_SKILLS
    effect: "Disables loading .claude/skills."
  - name: OPENCODE_DISABLE_MOUSE
    effect: "Disables mouse capture in the TUI."
  - name: OPENCODE_FAKE_VCS
    effect: "Fakes the VCS provider for testing."
  - name: OPENCODE_CLIENT
    effect: "Sets the client identifier; defaults to cli and is set to acp by the ACP command."
  - name: OPENCODE_ENABLE_EXA
    effect: "Enables Exa web search tools."
  - name: OPENCODE_SERVER_PASSWORD
    effect: "Enables basic auth for serve/web and supplies the default remote password for attach/run."
  - name: OPENCODE_SERVER_USERNAME
    effect: "Overrides the basic auth username; default is opencode."
  - name: OPENCODE_EXPERIMENTAL
    effect: "Enables the experimental umbrella flag."
  - name: OPENCODE_EXPERIMENTAL_ICON_DISCOVERY
    effect: "Enables experimental icon discovery."
  - name: OPENCODE_EXPERIMENTAL_DISABLE_COPY_ON_SELECT
    effect: "Disables copy-on-select in the TUI."
  - name: OPENCODE_EXPERIMENTAL_BASH_DEFAULT_TIMEOUT_MS
    effect: "Sets the experimental default timeout for bash commands in milliseconds."
  - name: OPENCODE_EXPERIMENTAL_OUTPUT_TOKEN_MAX
    effect: "Sets the experimental maximum output tokens for LLM responses."
machine_introspection:
  - command: "opencode debug config"
    purpose: config_dump
    machine_readable: true
    output_format: json
    useful_for_codegen: true
    notes: "Prints resolved merged configuration as JSON. Local inspection confirmed JSON output."
  - command: "opencode debug paths"
    purpose: env
    machine_readable: false
    output_format: text
    useful_for_codegen: false
    notes: "Prints resolved global paths for data, config, cache, state, logs, repos, temp, and bin; easy to parse but not structured JSON."
  - command: "opencode debug skill"
    purpose: capabilities
    machine_readable: true
    output_format: json
    useful_for_codegen: false
    notes: "Lists available skills as JSON; useful for reports and wrapper diagnostics."
  - command: "opencode models"
    purpose: models
    machine_readable: false
    output_format: text
    useful_for_codegen: true
    notes: "Prints provider/model ids one per line. It is parseable but not a JSON contract."
  - command: "opencode models --verbose"
    purpose: models
    machine_readable: false
    output_format: text
    useful_for_codegen: true
    notes: "Prints provider/model ids plus pretty-printed JSON metadata; useful but awkward because ids and JSON blocks are interleaved."
  - command: "opencode session list --format json"
    purpose: other
    machine_readable: true
    output_format: json
    useful_for_codegen: false
    notes: "Lists saved sessions with id, title, timestamps, project id, and directory."
  - command: "opencode export <sessionID> --sanitize"
    purpose: other
    machine_readable: true
    output_format: json
    useful_for_codegen: false
    notes: "Exports redacted session data for diagnostics or replay tooling."
  - command: "opencode db <query> --format json"
    purpose: other
    machine_readable: true
    output_format: json
    useful_for_codegen: false
    notes: "Runs arbitrary SQL against OpenCode's local database; useful for diagnostics but too unconstrained for normal wrapper metadata."
  - command: "opencode db path"
    purpose: other
    machine_readable: false
    output_format: text
    useful_for_codegen: false
    notes: "Prints the SQLite database path."
  - command: "opencode generate"
    purpose: config_schema
    machine_readable: true
    output_format: json
    useful_for_codegen: true
    notes: "Generates the OpenCode server OpenAPI document as JSON."
wrapper_notes:
  - "Use opencode run for non-interactive prompts; opencode without a subcommand starts the TUI and expects a terminal."
  - "opencode run --format json emits NDJSON, not a single JSON document."
  - "The run JSON stream does not expose every internal lifecycle event. It is known to omit tool-start style events and a dedicated session-complete event; wrappers must treat process exit as terminal state."
  - "Reasoning records in run JSON require --thinking."
  - "For richer progress and provider/model lifecycle signals, pair --format json with --print-logs --log-level INFO and parse stderr carefully."
  - "Successful runs can still write human/status output to stderr, especially share URLs, warnings, and logs when --print-logs is enabled."
  - "Non-interactive run denies question and plan-enter/plan-exit permissions by default in source; --auto, hidden --yolo, and hidden --dangerously-skip-permissions bypass approval prompts and should not be wrapper defaults."
  - "Interactive console/provider login, mcp auth, agent create without all required args, web, attach TUI, and default TUI modes require a TTY or browser/prompt flow."
  - "serve and web are long-running. serve prints a warning and starts unsecured unless OPENCODE_SERVER_PASSWORD is set."
  - "Config files are merged by precedence; wrappers that inject OPENCODE_CONFIG_CONTENT are intentionally overriding normal config at a high precedence layer."
  - "OpenCode loads plugins from global/project config and plugin directories unless --pure or related disable env vars are used."
  - "Project .opencode directories can trigger plugin/dependency behavior, including Bun package installation for plugin/custom-tool dependencies."
  - "The standalone installer writes to ~/.opencode/bin by default, while XDG config/data/cache paths are used for runtime state."
changes: []
requires_claudine_update: true
reason: "OpenCode current CLI surface differs from the stale agent-cli document: version is 1.17.13, schema frontmatter was missing, opencode run has additional wrapper-impacting flags, and machine introspection commands such as debug config, debug paths, models, session list, export, db, and generate should be reflected in Claudine provider metadata/wrappers."
---

## Overview

OpenCode is an open-source agentic coding CLI. The public command is `opencode`; running it without a subcommand starts the TUI, while `opencode run` is the primary non-interactive prompt mode documented for programmatic use.

The latest release verified for this file is `1.17.13`, from npm and GitHub releases. Local macOS inspection found `/Users/ken/.opencode/bin/opencode` reporting `1.17.13`.

The most wrapper-relevant surface is `opencode run --format json`, which emits newline-delimited JSON events on stdout. It is useful for automation but not a complete event bus. Wrappers should pair it with process-exit handling and, when richer lifecycle signals are needed, `--print-logs --log-level INFO` on stderr.

## Installation and Binaries

The canonical binary name is `opencode` on all platforms. The npm package exposes `opencode`; Windows installations may surface `opencode.exe` or command shims depending on the installer.

Documented install paths and methods include:

- Standalone installer: `curl -fsSL https://opencode.ai/install | bash`.
- npm package: `npm i -g opencode-ai@latest`; README also mentions bun, pnpm, and yarn.
- macOS/Linux Homebrew: `brew install anomalyco/tap/opencode` or `brew install opencode`.
- Windows: `scoop install opencode` or `choco install opencode`.
- Arch Linux: `sudo pacman -S opencode` or `paru -S opencode-bin`.

The standalone installer defaults to `$HOME/.opencode/bin`, supports `--version`, `--binary`, and `--no-modify-path`, and downloads OS/architecture-specific release archives.

## Subcommands

Public and source-registered top-level commands verified from docs, local help, and source are:

- `opencode [project]`: starts the TUI.
- `opencode run [message..]`: non-interactive prompt execution, with optional direct interactive mode.
- `opencode attach <url>`: attaches to a running OpenCode server.
- `opencode acp`: starts an ACP server over stdin/stdout.
- `opencode mcp`: manages MCP servers.
- `opencode agent`: creates or lists agents.
- `opencode plugin <module>` / `opencode plug <module>`: installs plugins and updates config.
- `opencode session`: lists or deletes sessions.
- `opencode models`: lists available provider/model ids.
- `opencode providers`: manages provider auth/config flows.
- `opencode console`: manages OpenCode console login, logout, org switching, org listing, and opening.
- `opencode serve`: starts a headless HTTP server.
- `opencode web`: starts a server and opens the web UI.
- `opencode db`: queries or locates the local database.
- `opencode debug`: troubleshooting commands.
- `opencode generate`: emits the OpenAPI document.
- `opencode stats`: token/cost statistics.
- `opencode export` / `opencode import`: session data transfer.
- `opencode github`: GitHub agent management.
- `opencode pr`: checks out a GitHub PR then launches OpenCode.
- `opencode completion`: shell completion generation.
- `opencode upgrade` and `opencode uninstall`: installation management.

## CLI Switch Inventory

The structured frontmatter contains the switch inventory that matters to wrappers. The most important groups are:

- Global: `--help`, `--version`, `--print-logs`, `--log-level`, `--pure`.
- Non-interactive run: `--command`, `--continue`, `--session`, `--fork`, `--share`, `--model`, `--agent`, `--format`, `--file`, `--title`, `--attach`, `--password`, `--username`, `--dir`, `--port`, `--variant`, `--thinking`, `--interactive`, `--auto`.
- Hidden or compatibility run flags: `--mini`, `--yolo`, `--dangerously-skip-permissions`, `--replay`, `--replay-limit`, `--demo`.
- Server/network: `--port`, `--hostname`, `--mdns`, `--mdns-domain`, `--cors`, plus `--cwd` for `acp`.
- Machine-readable state: `session list --format json`, `db --format json`, `export --sanitize`, and `generate`.

`opencode run --format json` is the machine-output mode for prompt runs. `db --format json` and `session list --format json` emit regular JSON documents, not NDJSON.

## Configuration Discovery

OpenCode uses JSON or JSONC config. Documented config precedence is:

1. Remote config from `.well-known/opencode`.
2. Global config, normally `~/.config/opencode/opencode.json`.
3. Custom config from `OPENCODE_CONFIG`.
4. Project config, `opencode.json` in the project.
5. `.opencode` directories for agents, commands, and plugins.
6. Inline config from `OPENCODE_CONFIG_CONTENT`.

Config files are merged rather than replaced. Later sources override conflicting keys while preserving non-conflicting settings.

Global and project TUI-specific config use `tui.json` or `tui.jsonc`. Managed settings can be deployed under `/Library/Application Support/opencode/`, `/etc/opencode/`, or `%ProgramData%\\opencode`, and macOS can also use the `ai.opencode.managed` managed-preferences domain.

Provider credentials added through `/connect` are documented as stored in `~/.local/share/opencode/auth.json`.

## Environment Variables

General wrapper-relevant variables include:

- `OPENCODE_CONFIG`, `OPENCODE_CONFIG_DIR`, and `OPENCODE_CONFIG_CONTENT` for config injection and discovery.
- `OPENCODE_TUI_CONFIG` for TUI-specific config.
- `OPENCODE_SERVER_PASSWORD` and `OPENCODE_SERVER_USERNAME` for server/attach authentication.
- `OPENCODE_CLIENT` for client identity.
- `OPENCODE_DISABLE_AUTOUPDATE`, `OPENCODE_DISABLE_PRUNE`, `OPENCODE_DISABLE_TERMINAL_TITLE`, `OPENCODE_DISABLE_AUTOCOMPACT`, `OPENCODE_DISABLE_MOUSE`, and `OPENCODE_GIT_BASH_PATH` for runtime behavior.
- `OPENCODE_DISABLE_CLAUDE_CODE`, `OPENCODE_DISABLE_CLAUDE_CODE_PROMPT`, and `OPENCODE_DISABLE_CLAUDE_CODE_SKILLS` for cross-provider prompt/skill loading behavior.
- `OPENCODE_DISABLE_DEFAULT_PLUGINS`, `OPENCODE_ENABLE_EXA`, and the experimental variables listed in frontmatter for plugin/tool/UI behavior.

Variables primarily owned by narrower topics, such as provider endpoint/model variables, permission policy injection, logging, MCP, and streaming, are not exhaustively duplicated here.

## Machine Introspection

Useful commands for Claudine wrappers and reports:

- `opencode debug config`: resolved config as JSON. Useful for provider metadata and wrapper diagnostics.
- `opencode debug paths`: resolved data/config/cache/state/log/bin paths as text.
- `opencode models`: provider/model ids, one per line.
- `opencode models --verbose`: model ids plus JSON metadata blocks.
- `opencode session list --format json`: saved sessions as JSON.
- `opencode export <sessionID> --sanitize`: redacted session export as JSON.
- `opencode db <query> --format json`: arbitrary database query output as JSON.
- `opencode db path`: database path.
- `opencode generate`: OpenAPI document as JSON.
- `opencode debug skill`: available skills as JSON.

Avoid treating `--help` and `--version` as machine introspection except for diagnostics; they do not expose stable provider state beyond version text.

## Wrapper Notes

Wrappers should invoke `opencode run` for non-interactive prompts. Running `opencode` without a subcommand starts the TUI.

`opencode run --format json` writes NDJSON events to stdout. The stream is not a formal complete schema for every OpenCode lifecycle signal. Current implementation-level behavior from the existing Claudine research and source inspection: tool events are completion-oriented, there is no dedicated completion event, and reasoning output requires `--thinking`.

Use process exit as the terminal signal. For richer lifecycle and provider/model progress, add `--print-logs --log-level INFO` and parse stderr as a second source. This makes stderr intentionally noisy even for successful runs.

Non-interactive `run` adds deny rules for question and plan enter/exit permissions by default. `--auto`, hidden `--yolo`, and hidden `--dangerously-skip-permissions` remove permission friction and should be explicit opt-ins, not wrapper defaults.

OpenCode loads global/project plugins and `.opencode` assets unless disabled. For deterministic wrapper runs, consider `--pure` and explicit config injection. Be aware that plugin/custom-tool dependencies can trigger Bun package installation.

`serve` and `web` are long-running. `serve` warns when `OPENCODE_SERVER_PASSWORD` is missing; wrappers exposing remote attachment should set server credentials.

## Sources

- [OpenCode CLI docs](https://opencode.ai/docs/cli/)
- [OpenCode config docs](https://opencode.ai/docs/config/)
- [OpenCode providers docs](https://opencode.ai/docs/providers/)
- [OpenCode MCP docs](https://opencode.ai/docs/mcp-servers/)
- [OpenCode plugins docs](https://opencode.ai/docs/plugins/)
- [OpenCode troubleshooting docs](https://opencode.ai/docs/troubleshooting/)
- [OpenCode GitHub repository](https://github.com/anomalyco/opencode)
- [OpenCode releases](https://github.com/anomalyco/opencode/releases)
- [OpenCode npm package metadata](https://www.npmjs.com/package/opencode-ai)
- [OpenCode CLI entry source](https://github.com/anomalyco/opencode/blob/dev/packages/opencode/src/index.ts)
- [OpenCode run command source](https://github.com/anomalyco/opencode/blob/dev/packages/opencode/src/cli/cmd/run.ts)
- [OpenCode config path source](https://github.com/anomalyco/opencode/blob/dev/packages/opencode/src/config/paths.ts)
- [OpenCode global path source](https://github.com/anomalyco/opencode/blob/dev/packages/core/src/global.ts)
- [OpenCode flag source](https://github.com/anomalyco/opencode/blob/dev/packages/core/src/flag/flag.ts)
