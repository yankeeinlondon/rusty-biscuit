---
$schema: ./_schema.yaml
created: 2026-05-12
last_updated: 2026-07-03
agent: codex
model: default
latest_version: "1.17.13"
homepage: https://opencode.ai
repo: https://github.com/anomalyco/opencode
docs: https://opencode.ai/docs/
cli_docs: https://opencode.ai/docs/cli/
binaries:
  - os: macos
    binary: opencode
    alt_binaries: []
    notes: "Official command name. Local macOS inspection found /Users/ken/.opencode/bin/opencode as a Mach-O arm64 standalone binary reporting 1.17.13."
  - os: linux
    binary: opencode
    alt_binaries: []
    notes: "Official command name for the install script, Homebrew, npm, Arch, Docker image entrypoint, and release archives."
  - os: windows
    binary: opencode
    alt_binaries: ["opencode.exe", "opencode.cmd"]
    notes: "Docs show the command as opencode. npm package metadata exposes bin.opencode as bin/opencode.exe; npm and package-manager shims may add .cmd/.ps1 launchers. Official docs recommend WSL for best Windows behavior."
install_methods:
  - os: macos
    method: standalone_binary
    command: "curl -fsSL https://opencode.ai/install | bash"
    notes: "Official install script. Local standalone install placed the binary under ~/.opencode/bin."
  - os: linux
    method: standalone_binary
    command: "curl -fsSL https://opencode.ai/install | bash"
    notes: "Official install script for Linux release archives."
  - os: macos
    method: npm
    command: "npm install -g opencode-ai"
    notes: "Official Node.js install; download page also shows npm i -g opencode-ai."
  - os: linux
    method: npm
    command: "npm install -g opencode-ai"
    notes: "Official Node.js install; bun, pnpm, and yarn are also documented."
  - os: windows
    method: npm
    command: "npm install -g opencode-ai"
    notes: "Official Windows npm install. Docs say Windows Bun support is still in progress."
  - os: macos
    method: other
    command: "bun install -g opencode-ai"
    notes: "Official Node-runtime alternative."
  - os: linux
    method: other
    command: "bun install -g opencode-ai"
    notes: "Official Node-runtime alternative."
  - os: macos
    method: other
    command: "pnpm install -g opencode-ai"
    notes: "Official Node-runtime alternative."
  - os: linux
    method: other
    command: "pnpm install -g opencode-ai"
    notes: "Official Node-runtime alternative."
  - os: windows
    method: other
    command: "pnpm install -g opencode-ai"
    notes: "Official Node-runtime alternative."
  - os: macos
    method: other
    command: "yarn global add opencode-ai"
    notes: "Official Node-runtime alternative."
  - os: linux
    method: other
    command: "yarn global add opencode-ai"
    notes: "Official Node-runtime alternative."
  - os: windows
    method: other
    command: "yarn global add opencode-ai"
    notes: "Official Node-runtime alternative."
  - os: macos
    method: brew
    command: "brew install anomalyco/tap/opencode"
    notes: "Official docs recommend the OpenCode tap as most up to date."
  - os: linux
    method: brew
    command: "brew install anomalyco/tap/opencode"
    notes: "Homebrew on Linux is documented."
  - os: linux
    method: package_manager
    command: "sudo pacman -S opencode"
    notes: "Official Arch Linux stable package command."
  - os: linux
    method: package_manager
    command: "paru -S opencode-bin"
    notes: "Official Arch AUR latest-release command; download page currently shows paru -S opencode."
  - os: windows
    method: chocolatey
    command: "choco install opencode"
    notes: "Official Windows package-manager install."
  - os: windows
    method: scoop
    command: "scoop install opencode"
    notes: "Official Windows package-manager install."
  - os: windows
    method: other
    command: "mise use -g github:anomalyco/opencode"
    notes: "Official Windows docs list Mise."
  - os: linux
    method: other
    command: "docker run -it --rm ghcr.io/anomalyco/opencode"
    notes: "Official Docker invocation; interactive by default."
subcommands:
  - name: tui
    description: "Default mode, opencode [project], starts the terminal UI in the current or supplied project path."
    non_interactive: false
    notes: "Requires a TTY. Local help exposes TUI flags on the root command."
  - name: run
    description: "Runs a prompt/task from argv or stdin and exits unless direct interactive mode is requested."
    non_interactive: true
    notes: "Primary automation entry point; --format json emits NDJSON. --interactive/--mini require TTY."
  - name: attach
    description: "Attaches a terminal UI to an already running OpenCode backend server."
    non_interactive: false
    notes: "Requires a server URL and TTY for useful operation."
  - name: acp
    description: "Starts an ACP Agent Client Protocol server over stdin/stdout."
    non_interactive: true
    notes: "Docs describe nd-JSON framing. It is a long-running protocol server."
  - name: mcp
    description: "Manages MCP servers: add, list/ls, auth, logout, and debug."
    non_interactive: false
    notes: "list/debug are automation-friendly; add and OAuth auth/logout can prompt or require browser/auth flows."
  - name: providers
    description: "Manages AI provider credentials: list/ls, login, and logout."
    non_interactive: false
    notes: "Local binary exposes providers as canonical and auth as alias; official CLI docs describe auth. list is non-interactive, login/logout can prompt."
  - name: auth
    description: "Alias for providers credential management."
    non_interactive: false
    notes: "Accepted by local 1.17.13; help heading still says opencode providers."
  - name: console
    description: "Manages OpenCode console login, logout, org switching/listing, and opening the active account."
    non_interactive: false
    notes: "Present in local 1.17.13 help/source but omitted from the official CLI docs page; login/open require browser or account interaction."
  - name: agent
    description: "Creates and lists OpenCode agents."
    non_interactive: false
    notes: "agent list is non-interactive; agent create becomes non-interactive only when path, description, mode, and permissions are supplied."
  - name: plugin
    description: "Installs an npm plugin and updates config."
    non_interactive: false
    notes: "Alias: plug. Mutates global or project config and may install package dependencies."
  - name: pr
    description: "Fetches and checks out a GitHub PR branch, then runs OpenCode."
    non_interactive: false
    notes: "Uses GitHub/git state and then launches OpenCode."
  - name: db
    description: "Opens sqlite3, runs a SQL query, or prints the local database path."
    non_interactive: true
    notes: "db without query opens an interactive sqlite shell; db path and db <query> --format json are automation-friendly."
  - name: debug
    description: "Runs troubleshooting commands such as config, paths, skill, lsp, rg, file, scrap, snapshot, and startup."
    non_interactive: true
    notes: "Several subcommands produce machine-usable output."
  - name: session
    description: "Lists and deletes saved sessions."
    non_interactive: true
    notes: "session list --format json is machine-readable; delete mutates local state."
  - name: models
    description: "Lists available models, optionally filtered by provider."
    non_interactive: true
    notes: "Line-oriented text by default; --verbose interleaves model ids and JSON metadata."
  - name: serve
    description: "Starts a headless OpenCode HTTP server."
    non_interactive: true
    notes: "Long-running process; set OPENCODE_SERVER_PASSWORD for basic auth."
  - name: web
    description: "Starts an OpenCode server and opens the web interface."
    non_interactive: false
    notes: "Long-running and browser-opening."
  - name: generate
    description: "Generates the OpenCode server OpenAPI document as JSON."
    non_interactive: true
    notes: "Useful for API/schema code generation."
  - name: stats
    description: "Shows token usage and cost statistics."
    non_interactive: true
    notes: "Human-readable output; no JSON flag observed."
  - name: export
    description: "Exports session data as JSON."
    non_interactive: true
    notes: "Prompts for a session if sessionID is omitted; --sanitize redacts sensitive transcript/file data."
  - name: import
    description: "Imports session data from a JSON file or share URL."
    non_interactive: true
    notes: "Mutates local session storage."
  - name: github
    description: "Manages the GitHub agent."
    non_interactive: false
    notes: "github run is intended for GitHub Actions and accepts --event/--token; github install is interactive setup."
  - name: completion
    description: "Generates shell completion scripts."
    non_interactive: true
    notes: "Exposed by yargs completion."
  - name: upgrade
    description: "Upgrades OpenCode to latest or a specific version."
    non_interactive: false
    notes: "Mutates installation; --method can select curl/npm/pnpm/bun/brew."
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
    default: "unset"
    description: "Selects log verbosity."
    example: "opencode --log-level DEBUG debug paths"
    notes: "Sets OPENCODE_LOG_LEVEL internally."
  - flag: --pure
    value: ""
    scope: ["global", "plugins"]
    default: "false"
    description: "Runs without external plugins."
    example: "opencode --pure run \"inspect this repo\""
    notes: "Sets OPENCODE_PURE=1 internally."
  - flag: --port
    value: "<PORT>"
    scope: ["tui", "run", "serve", "web", "acp", "network"]
    default: "0/random"
    description: "Port for the local server."
    example: "opencode serve --port 4096"
    notes: "Root help shows default 0; run docs say random port when omitted."
  - flag: --hostname
    value: "<HOST>"
    scope: ["tui", "serve", "web", "acp", "network"]
    default: "127.0.0.1"
    description: "Hostname for the local server."
    example: "opencode serve --hostname 0.0.0.0"
    notes: "Config docs say mDNS can default hostname to 0.0.0.0."
  - flag: --mdns
    value: ""
    scope: ["tui", "serve", "web", "acp", "network"]
    default: "false"
    description: "Enables mDNS service discovery."
    example: "opencode web --mdns"
    notes: "Can be disabled with yargs --no-mdns."
  - flag: --mdns-domain
    value: "<DOMAIN>"
    scope: ["tui", "serve", "web", "acp", "network"]
    default: "opencode.local"
    description: "Sets the custom mDNS service domain."
    example: "opencode serve --mdns --mdns-domain workstation.local"
    notes: "Config key is mdnsDomain."
  - flag: --cors
    value: "<ORIGIN>"
    scope: ["tui", "serve", "web", "acp", "network"]
    default: "[]"
    description: "Adds browser origins to allow for CORS."
    example: "opencode serve --cors http://localhost:3000"
    notes: "Array/repeatable according to local root help."
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
    default: "unset"
    description: "Continues a specific session."
    example: "opencode run --session ses_abc \"continue\""
    notes: "Short form: -s."
  - flag: --fork
    value: ""
    scope: ["tui", "run", "attach", "session"]
    default: "false"
    description: "Forks before continuing a session."
    example: "opencode run --session ses_abc --fork \"try another approach\""
    notes: "Requires --continue or --session; installed version rejects --fork alone."
  - flag: --prompt
    value: "<TEXT>"
    scope: ["tui", "input"]
    default: "unset"
    description: "Initial prompt to use in TUI mode."
    example: "opencode --prompt \"review this repo\""
    notes: "Root/TUI-scoped, not run-scoped."
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
    default: "config/default_agent"
    description: "Selects the primary agent."
    example: "opencode run --agent build \"implement feature\""
    notes: "Source rejects subagents for primary run selection and falls back to default."
  - flag: --auto
    value: ""
    scope: ["tui", "run", "permissions"]
    default: "false"
    description: "Auto-approves permissions that are not explicitly denied."
    example: "opencode run --auto \"apply the change\""
    notes: "Dangerous for wrappers unless policy is constrained elsewhere."
  - flag: --yolo
    value: ""
    scope: ["run", "permissions"]
    default: "false"
    description: "Hidden permission bypass used by source to enable auto approval."
    example: "opencode run --yolo \"apply the change\""
    notes: "Hidden; prefer --auto for user-facing behavior."
  - flag: --dangerously-skip-permissions
    value: ""
    scope: ["run", "permissions"]
    default: "false"
    description: "Hidden permission bypass used by source to enable auto approval."
    example: "opencode run --dangerously-skip-permissions \"apply the change\""
    notes: "Hidden and unsafe for general wrapper defaults."
  - flag: --command
    value: "<COMMAND>"
    scope: ["run", "input"]
    default: "unset"
    description: "Runs a slash command, using the message as arguments."
    example: "opencode run --command test -- \"unit only\""
    notes: "Cannot be used with --mini."
  - flag: --share
    value: ""
    scope: ["run", "sharing"]
    default: "false/config"
    description: "Shares the session."
    example: "opencode run --share \"summarize\""
    notes: "Also affected by config share and OPENCODE_AUTO_SHARE."
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
    default: "unset"
    description: "Sets the session title."
    example: "opencode run --title \"CI failure\" \"debug tests\""
    notes: "Empty value uses a truncated prompt."
  - flag: --attach
    value: "<URL>"
    scope: ["run", "remote"]
    default: "unset"
    description: "Runs against a running OpenCode server."
    example: "opencode run --attach http://localhost:4096 \"continue\""
    notes: "Skips local instance loading."
  - flag: --password
    value: "<PASSWORD>"
    scope: ["run", "attach", "auth"]
    default: "OPENCODE_SERVER_PASSWORD"
    description: "Basic auth password for a remote server."
    example: "opencode attach http://localhost:4096 --password \"$OPENCODE_SERVER_PASSWORD\""
    notes: "Short form: -p. Avoid secrets in argv when possible."
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
    default: "unset"
    description: "Sets a provider-specific model variant such as reasoning effort."
    example: "opencode run --variant high \"solve\""
    notes: "Source examples include high, max, and minimal."
  - flag: --thinking
    value: ""
    scope: ["run", "output"]
    default: "false for non-interactive run; true for mini"
    description: "Shows thinking/reasoning blocks."
    example: "opencode run --thinking --format json \"solve\""
    notes: "Reasoning events in run JSON are opt-in."
  - flag: --interactive
    value: ""
    scope: ["run", "tui"]
    default: "false"
    description: "Runs in direct interactive split-footer mode."
    example: "opencode run --interactive"
    notes: "Short form: -i. Source marks direct interactive mode; requires TTY."
  - flag: --mini
    value: ""
    scope: ["tui", "run", "attach"]
    default: "false"
    description: "Starts the minimal interactive interface."
    example: "opencode --mini"
    notes: "Hidden in run source but visible in root help; requires TTY stdout and cannot be used with --format json."
  - flag: --no-replay
    value: ""
    scope: ["tui", "attach"]
    default: "false"
    description: "Disables mini session history replay on resume and resize."
    example: "opencode attach http://localhost:4096 --mini --no-replay"
    notes: "Root help exposes this as --no-replay; source option is replay default true."
  - flag: --replay-limit
    value: "<N>"
    scope: ["tui", "run", "attach"]
    default: "unset"
    description: "Caps visible mini replay to the newest N messages."
    example: "opencode --mini --replay-limit 20"
    notes: "Requires --mini and a positive integer."
  - flag: --demo
    value: ""
    scope: ["run", "tui"]
    default: "false"
    description: "Hidden direct-interactive demo slash-command mode."
    example: "opencode --mini --demo"
    notes: "Source requires --mini; not a wrapper-stable flag."
  - flag: --cwd
    value: "<PATH>"
    scope: ["acp"]
    default: "process.cwd()"
    description: "Working directory for the ACP server."
    example: "opencode acp --cwd /repo"
    notes: "Documented ACP flag."
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
    notes: "Supplying all create inputs avoids the interactive wizard."
  - flag: --mode
    value: "all | primary | subagent"
    scope: ["agent create"]
    default: "prompted"
    description: "Sets the agent mode."
    example: "opencode agent create --mode primary --description \"Build features\" --permissions bash,read,edit"
    notes: "Documented values: all, primary, subagent."
  - flag: --permissions
    value: "<CSV>"
    scope: ["agent create"]
    default: "all"
    description: "Comma-separated permissions to allow."
    example: "opencode agent create --permissions read,grep,glob"
    notes: "Alias: --tools. Available permissions include bash, read, edit, glob, grep, webfetch, task, todowrite, websearch, lsp, and skill."
  - flag: --tools
    value: "<CSV>"
    scope: ["agent create"]
    default: "all"
    description: "Alias for --permissions."
    example: "opencode agent create --tools read,grep,glob"
    notes: "Documented as an alias."
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
    default: "unset"
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
    notes: "0 means today in source/older research."
  - flag: --models
    value: "[N]"
    scope: ["stats"]
    default: "hidden"
    description: "Shows model statistics, optionally top N."
    example: "opencode stats --models 10"
    notes: "Stats output is human-readable."
  - flag: --project
    value: "<PROJECT_ID>"
    scope: ["stats"]
    default: "all projects"
    description: "Filters stats by project; empty string means current project."
    example: "opencode stats --project \"\""
    notes: "Documented stats flag."
  - flag: --sanitize
    value: ""
    scope: ["export"]
    default: "false"
    description: "Redacts sensitive transcript and file data."
    example: "opencode export ses_abc --sanitize"
    notes: "Use for diagnostics."
  - flag: --event
    value: "<EVENT>"
    scope: ["github run"]
    default: "unset"
    description: "GitHub mock event to run the agent for."
    example: "opencode github run --event pull_request"
    notes: "Documented GitHub run flag."
  - flag: --token
    value: "<TOKEN>"
    scope: ["github run"]
    default: "unset"
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
    notes: "Safe diagnostic mode."
  - flag: --method
    value: "curl | npm | pnpm | bun | brew"
    scope: ["upgrade"]
    default: "detected"
    description: "Selects the installation method used for upgrade."
    example: "opencode upgrade --method brew"
    notes: "Short form: -m. Local docs list these values; old research also mentioned choco/scoop, but current docs do not."
  - flag: "system-prompt delivery flags"
    value: "none observed"
    scope: ["run", "system_prompt"]
    default: "not applicable"
    description: "No native opencode run CLI flag for inline/file system-prompt replacement or append was observed in local help, official CLI docs, or v1.17.13 run source."
    example: "opencode run \"prompt\""
    notes: "OpenCode uses agents and config instructions for prompt/instruction delivery. Defer Claudine wrapper semantics to the sibling system-prompt topic."
config_paths:
  - os: macos
    scope: user
    path: "~/.config/opencode/opencode.json"
    format: json
    notes: "Global config; JSONC also supported. Local debug paths show this host's effective config root as /Users/ken/.claudine/.config/opencode because HOME is redirected in this environment."
  - os: linux
    scope: user
    path: "~/.config/opencode/opencode.json"
    format: json
    notes: "Global config; JSONC also supported."
  - os: windows
    scope: user
    path: "%USERPROFILE%\\.config\\opencode\\opencode.json"
    format: json
    notes: "Docs describe ~/.config/opencode; Windows path expansion is home-relative."
  - os: macos
    scope: user
    path: "~/.config/opencode/opencode.jsonc"
    format: jsonc
    notes: "Global JSONC config. Local file contains only the schema URL."
  - os: linux
    scope: user
    path: "~/.config/opencode/opencode.jsonc"
    format: jsonc
    notes: "Global JSONC config."
  - os: windows
    scope: user
    path: "%USERPROFILE%\\.config\\opencode\\opencode.jsonc"
    format: jsonc
    notes: "Global JSONC config."
  - os: macos
    scope: user
    path: "~/.config/opencode/tui.json"
    format: json
    notes: "Global TUI-specific config; may also be JSONC."
  - os: linux
    scope: user
    path: "~/.config/opencode/tui.json"
    format: json
    notes: "Global TUI-specific config; may also be JSONC."
  - os: windows
    scope: user
    path: "%USERPROFILE%\\.config\\opencode\\tui.json"
    format: json
    notes: "Global TUI-specific config; may also be JSONC."
  - os: macos
    scope: repo
    path: "opencode.json"
    format: json
    notes: "Project config. OpenCode starts in the current directory and traverses up to the nearest Git directory."
  - os: linux
    scope: repo
    path: "opencode.json"
    format: json
    notes: "Project config. OpenCode starts in the current directory and traverses up to the nearest Git directory."
  - os: windows
    scope: repo
    path: "opencode.json"
    format: json
    notes: "Project config. OpenCode starts in the current directory and traverses up to the nearest Git directory."
  - os: macos
    scope: repo
    path: "opencode.jsonc"
    format: jsonc
    notes: "Project JSONC config."
  - os: linux
    scope: repo
    path: "opencode.jsonc"
    format: jsonc
    notes: "Project JSONC config."
  - os: windows
    scope: repo
    path: "opencode.jsonc"
    format: jsonc
    notes: "Project JSONC config."
  - os: macos
    scope: repo
    path: "tui.json"
    format: json
    notes: "Project TUI-specific config alongside opencode.json."
  - os: linux
    scope: repo
    path: "tui.json"
    format: json
    notes: "Project TUI-specific config alongside opencode.json."
  - os: windows
    scope: repo
    path: "tui.json"
    format: json
    notes: "Project TUI-specific config alongside opencode.json."
  - os: macos
    scope: repo
    path: ".opencode/"
    format: other
    notes: "Project directory for agents, commands, modes, plugins, skills, tools, and themes. Plural names are preferred; singular names are backward-compatible."
  - os: linux
    scope: repo
    path: ".opencode/"
    format: other
    notes: "Project directory for agents, commands, modes, plugins, skills, tools, and themes."
  - os: windows
    scope: repo
    path: ".opencode\\"
    format: other
    notes: "Project directory for agents, commands, modes, plugins, skills, tools, and themes."
  - os: macos
    scope: user
    path: "~/.config/opencode/"
    format: other
    notes: "Global directory for agents, commands, modes, plugins, skills, tools, themes, package.json, and dependencies."
  - os: linux
    scope: user
    path: "~/.config/opencode/"
    format: other
    notes: "Global directory for agents, commands, modes, plugins, skills, tools, themes, package.json, and dependencies."
  - os: windows
    scope: user
    path: "%USERPROFILE%\\.config\\opencode\\"
    format: other
    notes: "Global directory for agents, commands, modes, plugins, skills, tools, themes, package.json, and dependencies."
  - os: macos
    scope: env
    path: "$OPENCODE_CONFIG"
    format: json
    notes: "Custom config file loaded between global and project config; JSON or JSONC by content/file."
  - os: linux
    scope: env
    path: "$OPENCODE_CONFIG"
    format: json
    notes: "Custom config file loaded between global and project config; JSON or JSONC by content/file."
  - os: windows
    scope: env
    path: "%OPENCODE_CONFIG%"
    format: json
    notes: "Custom config file loaded between global and project config; JSON or JSONC by content/file."
  - os: macos
    scope: env
    path: "$OPENCODE_CONFIG_DIR"
    format: other
    notes: "Custom config directory searched for agents, commands, modes, and plugins after global and .opencode directories."
  - os: linux
    scope: env
    path: "$OPENCODE_CONFIG_DIR"
    format: other
    notes: "Custom config directory searched for agents, commands, modes, and plugins after global and .opencode directories."
  - os: windows
    scope: env
    path: "%OPENCODE_CONFIG_DIR%"
    format: other
    notes: "Custom config directory searched for agents, commands, modes, and plugins after global and .opencode directories."
  - os: macos
    scope: env
    path: "$OPENCODE_CONFIG_CONTENT"
    format: json
    notes: "Inline JSON config content loaded as a high-precedence runtime override before managed settings."
  - os: linux
    scope: env
    path: "$OPENCODE_CONFIG_CONTENT"
    format: json
    notes: "Inline JSON config content loaded as a high-precedence runtime override before managed settings."
  - os: windows
    scope: env
    path: "%OPENCODE_CONFIG_CONTENT%"
    format: json
    notes: "Inline JSON config content loaded as a high-precedence runtime override before managed settings."
  - os: macos
    scope: system
    path: "/Library/Application Support/opencode/opencode.json"
    format: json
    notes: "Managed settings file; admin-controlled and higher priority than inline config."
  - os: macos
    scope: system
    path: "/Library/Application Support/opencode/opencode.jsonc"
    format: jsonc
    notes: "Managed settings file; admin-controlled and higher priority than inline config."
  - os: linux
    scope: system
    path: "/etc/opencode/opencode.json"
    format: json
    notes: "Managed settings file; admin/root-controlled."
  - os: linux
    scope: system
    path: "/etc/opencode/opencode.jsonc"
    format: jsonc
    notes: "Managed settings file; admin/root-controlled."
  - os: windows
    scope: system
    path: "%ProgramData%\\opencode\\opencode.json"
    format: json
    notes: "Managed settings file; admin-controlled."
  - os: windows
    scope: system
    path: "%ProgramData%\\opencode\\opencode.jsonc"
    format: jsonc
    notes: "Managed settings file; admin-controlled."
  - os: macos
    scope: system
    path: "/Library/Managed Preferences/<user>/ai.opencode.managed.plist"
    format: other
    notes: "macOS MDM managed preferences; highest priority and not user-overridable."
  - os: macos
    scope: system
    path: "/Library/Managed Preferences/ai.opencode.managed.plist"
    format: other
    notes: "macOS MDM managed preferences fallback path; highest priority."
  - os: macos
    scope: user
    path: "~/.local/share/opencode/auth.json"
    format: json
    notes: "Provider credentials stored after /connect or opencode providers/auth login."
  - os: linux
    scope: user
    path: "~/.local/share/opencode/auth.json"
    format: json
    notes: "Provider credentials stored after /connect or opencode providers/auth login."
  - os: windows
    scope: user
    path: "%USERPROFILE%\\.local\\share\\opencode\\auth.json"
    format: json
    notes: "Provider credentials stored after /connect or opencode providers/auth login."
  - os: macos
    scope: user
    path: "~/.local/share/opencode/opencode.db"
    format: other
    notes: "SQLite database path observed through opencode db path, with HOME redirected to /Users/ken/.claudine on this host."
  - os: linux
    scope: user
    path: "~/.local/share/opencode/opencode.db"
    format: other
    notes: "SQLite database path exposed by opencode db path."
  - os: windows
    scope: user
    path: "%USERPROFILE%\\.local\\share\\opencode\\opencode.db"
    format: other
    notes: "Home-relative SQLite database path inferred from documented ~/.local/share path pattern; verify on native Windows before hard-coding."
env_vars:
  - name: OPENCODE_AUTO_SHARE
    effect: "Automatically shares sessions."
  - name: OPENCODE_GIT_BASH_PATH
    effect: "Path to Git Bash on Windows; used by Windows shell/pager integration."
  - name: OPENCODE_CONFIG
    effect: "Sets a custom config file path loaded between global and project config."
  - name: OPENCODE_TUI_CONFIG
    effect: "Sets a custom TUI config file path."
  - name: OPENCODE_CONFIG_DIR
    effect: "Sets a custom config directory for agents, commands, modes, and plugins."
  - name: OPENCODE_CONFIG_CONTENT
    effect: "Provides inline JSON config content as a high-precedence runtime override."
  - name: OPENCODE_DISABLE_AUTOUPDATE
    effect: "Disables automatic update checks."
  - name: OPENCODE_DISABLE_PRUNE
    effect: "Disables pruning of old data."
  - name: OPENCODE_DISABLE_TERMINAL_TITLE
    effect: "Disables automatic terminal title updates."
  - name: OPENCODE_PERMISSION
    effect: "Provides inline JSON permissions config; detailed permission semantics belong to agent-permissions."
  - name: OPENCODE_DISABLE_DEFAULT_PLUGINS
    effect: "Disables default plugins."
  - name: OPENCODE_DISABLE_LSP_DOWNLOAD
    effect: "Disables automatic LSP server downloads."
  - name: OPENCODE_ENABLE_EXPERIMENTAL_MODELS
    effect: "Enables experimental models in the model catalog."
  - name: OPENCODE_DISABLE_AUTOCOMPACT
    effect: "Disables automatic context compaction."
  - name: OPENCODE_DISABLE_CLAUDE_CODE
    effect: "Disables reading from .claude prompt and skills sources."
  - name: OPENCODE_DISABLE_CLAUDE_CODE_PROMPT
    effect: "Disables reading ~/.claude/CLAUDE.md."
  - name: OPENCODE_DISABLE_CLAUDE_CODE_SKILLS
    effect: "Disables loading .claude/skills."
  - name: OPENCODE_DISABLE_MODELS_FETCH
    effect: "Disables fetching model lists from remote sources."
  - name: OPENCODE_DISABLE_MOUSE
    effect: "Disables mouse capture in the TUI."
  - name: OPENCODE_FAKE_VCS
    effect: "Fakes the VCS provider for testing."
  - name: OPENCODE_CLIENT
    effect: "Sets the client identifier; defaults to cli and ACP sets it to acp."
  - name: OPENCODE_ENABLE_EXA
    effect: "Enables Exa web search tools."
  - name: OPENCODE_SERVER_PASSWORD
    effect: "Enables/supplies HTTP basic auth password for serve/web and remote attach/run."
  - name: OPENCODE_SERVER_USERNAME
    effect: "Overrides the basic auth username; default is opencode."
  - name: OPENCODE_MODELS_URL
    effect: "Sets a custom URL for fetching model configuration."
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
  - name: OPENCODE_EXPERIMENTAL_FILEWATCHER
    effect: "Enables file watcher for the entire directory."
  - name: OPENCODE_EXPERIMENTAL_OXFMT
    effect: "Enables the experimental oxfmt formatter."
  - name: OPENCODE_EXPERIMENTAL_LSP_TOOL
    effect: "Enables the experimental LSP tool."
  - name: OPENCODE_EXPERIMENTAL_DISABLE_FILEWATCHER
    effect: "Disables the experimental file watcher."
  - name: OPENCODE_EXPERIMENTAL_EXA
    effect: "Enables experimental Exa features."
  - name: OPENCODE_EXPERIMENTAL_LSP_TY
    effect: "Enables TY LSP for Python files."
  - name: OPENCODE_EXPERIMENTAL_PLAN_MODE
    effect: "Enables experimental plan mode."
  - name: OPENCODE_EXPERIMENTAL_BACKGROUND_SUBAGENTS
    effect: "Enables background subagent tasks."
  - name: OPENCODE_EXPERIMENTAL_EVENT_SYSTEM
    effect: "Enables the experimental event system."
  - name: OPENCODE_EXPERIMENTAL_NATIVE_LLM
    effect: "Enables the native LLM request path."
  - name: OPENCODE_EXPERIMENTAL_PARALLEL
    effect: "Enables parallel web search execution."
  - name: OPENCODE_EXPERIMENTAL_SCOUT
    effect: "Enables the Scout subagent."
  - name: OPENCODE_EXPERIMENTAL_WORKSPACES
    effect: "Enables workspace support."
  - name: OPENCODE
    effect: "Set to 1 by the CLI process for child/runtime detection."
  - name: OPENCODE_PID
    effect: "Set by the CLI to the current process id for child/runtime detection."
  - name: OPENCODE_PRINT_LOGS
    effect: "Set by --print-logs; causes logs to print to stderr."
  - name: OPENCODE_LOG_LEVEL
    effect: "Set by --log-level; controls log verbosity."
  - name: OPENCODE_PURE
    effect: "Set by --pure; disables external plugins."
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
    notes: "Prints resolved home, data, bin, log, repos, cache, config, state, and tmp paths."
  - command: "opencode debug skill"
    purpose: capabilities
    machine_readable: true
    output_format: json
    useful_for_codegen: false
    notes: "Lists available skills as JSON; useful for diagnostics and reports."
  - command: "opencode models"
    purpose: models
    machine_readable: false
    output_format: text
    useful_for_codegen: true
    notes: "Prints provider/model ids one per line. Parseable but not a JSON contract."
  - command: "opencode models --verbose"
    purpose: models
    machine_readable: false
    output_format: text
    useful_for_codegen: true
    notes: "Prints provider/model ids plus pretty-printed JSON metadata blocks; useful but awkward because ids and JSON are interleaved."
  - command: "opencode session list --format json"
    purpose: other
    machine_readable: true
    output_format: json
    useful_for_codegen: false
    notes: "Lists saved sessions with id, title, timestamps, project id, and directory. Local inspection confirmed JSON output."
  - command: "opencode export <sessionID> --sanitize"
    purpose: other
    machine_readable: true
    output_format: json
    useful_for_codegen: false
    notes: "Exports redacted session data for diagnostics or replay tooling. Omitting sessionID can prompt."
  - command: "opencode db <query> --format json"
    purpose: other
    machine_readable: true
    output_format: json
    useful_for_codegen: false
    notes: "Runs arbitrary SQL against OpenCode's local database; local probe of select 1 returned a JSON array."
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
    notes: "Generates the OpenCode server OpenAPI document as JSON. Local inspection showed OpenAPI 3.1.0."
  - command: "opencode providers list"
    purpose: env
    machine_readable: false
    output_format: text
    useful_for_codegen: false
    notes: "Lists credentials and environment-provided provider keys; local output was styled human text, not JSON."
  - command: "opencode auth list"
    purpose: env
    machine_readable: false
    output_format: text
    useful_for_codegen: false
    notes: "Alias for providers list; local output showed credential file path and environment variables."
wrapper_notes:
  - "Use opencode run for non-interactive prompts; opencode without a subcommand starts the TUI and expects a terminal."
  - "opencode run --format json emits NDJSON, not a single JSON document."
  - "The run JSON stream is completion-oriented, not a complete lifecycle event bus. Treat process exit as terminal state."
  - "Reasoning records in run JSON require --thinking."
  - "For richer progress and provider/model lifecycle signals, pair --format json with --print-logs --log-level INFO and parse stderr carefully."
  - "Successful runs can write human/status output to stderr, especially share URLs, warnings, and logs when --print-logs is enabled."
  - "Non-interactive run denies question and plan-enter/plan-exit permissions by default in source; --auto, hidden --yolo, and hidden --dangerously-skip-permissions bypass approval prompts and should not be wrapper defaults."
  - "Interactive provider/console login, mcp auth, agent create without all required args, web, attach TUI, and default TUI modes require a TTY, browser, or prompt flow."
  - "serve and web are long-running. serve/web are unauthenticated unless OPENCODE_SERVER_PASSWORD is set."
  - "Config files are merged by precedence; wrappers that inject OPENCODE_CONFIG_CONTENT intentionally override normal config at a high precedence layer, but managed settings can still override it."
  - "OpenCode loads plugins from global/project config and plugin directories unless --pure or disable env vars are used."
  - "Project .opencode directories can trigger plugin/dependency behavior, including package installation for plugin/custom-tool dependencies."
  - "The local 1.17.13 binary exposes providers as canonical and auth as alias, while the official CLI docs show auth; wrappers should accept/probe both names."
  - "No native run-level system-prompt delivery flag was observed; use config instructions or generated agents, and defer Claudine append/replace semantics to the system-prompt topic."
changes:
  - "Verified latest upstream and local installed OpenCode version remains 1.17.13 on 2026-07-03."
  - "Recorded local help/docs discrepancy: installed CLI exposes providers with auth alias, while official docs list auth."
  - "Added locally observed console command, which is present in the installed binary/source but omitted from the official CLI docs page."
  - "Expanded environment variable inventory to match the 2026-07-03 official CLI docs, including model-fetch and experimental feature variables."
  - "Reworked config_paths frontmatter into per-OS records required by the sidecar schema instead of invalid os: all records."
  - "Confirmed local machine-readable probes: debug config JSON, debug paths text, debug skill JSON, session list JSON, db JSON, db path text, generate OpenAPI JSON, and auth/providers list text."
requires_claudine_update: true
reason: "Claudine provider metadata/wrappers should reflect the current OpenCode command aliases (providers/auth), local console command, expanded env-var surface, per-OS config path records, and confirmed machine-introspection commands."
---

# OpenCode CLI Surface

## Overview

OpenCode is an open-source AI coding agent shipped by Anomaly. The primary terminal command is `opencode`. Running `opencode` without a subcommand starts the TUI; running `opencode run "..."` is the primary non-interactive automation path.

The current upstream version verified for this research is `1.17.13`. I verified it three ways on 2026-07-03: the local macOS binary `/Users/ken/.opencode/bin/opencode --version` printed `1.17.13`; `npm view opencode-ai version` returned `1.17.13`; and GitHub releases marked `v1.17.13` as latest.

Primary URLs:

- Homepage: [https://opencode.ai](https://opencode.ai)
- Repository: [https://github.com/anomalyco/opencode](https://github.com/anomalyco/opencode)
- General docs: [https://opencode.ai/docs/](https://opencode.ai/docs/)
- CLI reference: [https://opencode.ai/docs/cli/](https://opencode.ai/docs/cli/)

## Installation and Binaries

The documented command name is `opencode` on macOS, Linux, and Windows. Local macOS inspection found `/Users/ken/.opencode/bin/opencode`, a Mach-O arm64 standalone binary, reporting version `1.17.13`. npm package metadata exposes the bin entry as `opencode: bin/opencode.exe`, so Windows npm installs can also surface `opencode.exe` and shell shims such as `opencode.cmd` depending on npm/PowerShell/cmd setup.

Official install commands:

| OS | Method | Command | Notes |
| --- | --- | --- | --- |
| macOS/Linux | Install script | `curl -fsSL https://opencode.ai/install | bash` | Local install uses `~/.opencode/bin`. |
| macOS/Linux/Windows | npm | `npm install -g opencode-ai` | The download page also shows `npm i -g opencode-ai`. |
| macOS/Linux | Bun | `bun install -g opencode-ai` | Windows Bun support is documented as in progress. |
| macOS/Linux/Windows | pnpm | `pnpm install -g opencode-ai` | Official Node-runtime alternative. |
| macOS/Linux/Windows | Yarn | `yarn global add opencode-ai` | Official Node-runtime alternative. |
| macOS/Linux | Homebrew tap | `brew install anomalyco/tap/opencode` | Official docs recommend the OpenCode tap as most up to date. |
| Linux | Arch stable | `sudo pacman -S opencode` | Official docs. |
| Linux | Arch AUR/latest | `paru -S opencode-bin` | Intro docs; download page currently shows `paru -S opencode`. |
| Windows | Chocolatey | `choco install opencode` | Official Windows package-manager install. |
| Windows | Scoop | `scoop install opencode` | Official Windows package-manager install. |
| Windows | Mise | `mise use -g github:anomalyco/opencode` | Official Windows docs. |
| Linux/containers | Docker | `docker run -it --rm ghcr.io/anomalyco/opencode` | Interactive by default. |

OpenCode's docs recommend WSL for the best Windows experience.

## Subcommands

| Command | Description | Non-interactive? | Notes |
| --- | --- | --- | --- |
| `opencode [project]` | Starts the TUI. | No | Requires a TTY. |
| `opencode run [message..]` | Runs a prompt/task and exits by default. | Yes | Primary automation entry point; `--format json` emits NDJSON. |
| `opencode attach <url>` | Attaches a terminal UI to a running backend. | No | Needs server URL and TTY. |
| `opencode acp` | Starts an ACP server over stdin/stdout. | Yes | Long-running nd-JSON protocol server. |
| `opencode mcp` | Manages MCP servers. | Mixed | `list`/`debug` are automation-friendly; `add`/`auth` can prompt or use OAuth. |
| `opencode providers` | Manages provider credentials. | Mixed | Local canonical name; subcommands are `list`, `login`, `logout`. |
| `opencode auth` | Alias for `providers`. | Mixed | Official CLI docs list `auth`; local help heading still says `providers`. |
| `opencode console` | Manages OpenCode console account/org state. | No | Present locally but omitted from the official CLI page; login/open need browser/account interaction. |
| `opencode agent` | Creates and lists agents. | Mixed | `agent list` is non-interactive; `agent create` is non-interactive only with all required flags. |
| `opencode plugin` / `opencode plug` | Installs an npm plugin and updates config. | Mixed | Mutates config and may install dependencies. |
| `opencode pr <number>` | Fetches and checks out a GitHub PR, then runs OpenCode. | No | Depends on GitHub/git state and launches OpenCode. |
| `opencode db` | Opens sqlite3, runs SQL, or prints the database path. | Mixed | `db path` and `db <query> --format json` are automation-friendly; bare `db` is interactive. |
| `opencode debug` | Troubleshooting commands. | Yes | Includes `config`, `paths`, `skill`, `lsp`, `rg`, `file`, `scrap`, `snapshot`, and `startup`. |
| `opencode session` | Lists/deletes saved sessions. | Yes | `session list --format json` is machine-readable; delete mutates state. |
| `opencode models [provider]` | Lists model ids. | Yes | Text by default; `--verbose` interleaves ids and JSON metadata. |
| `opencode serve` | Starts a headless HTTP server. | Yes | Long-running; set `OPENCODE_SERVER_PASSWORD` for auth. |
| `opencode web` | Starts a server and opens the web UI. | No | Long-running and browser-opening. |
| `opencode generate` | Emits the OpenAPI document. | Yes | JSON output. |
| `opencode stats` | Shows token/cost stats. | Yes | Human-readable output. |
| `opencode export [sessionID]` | Exports session JSON. | Yes | Omitting `sessionID` can prompt. |
| `opencode import <file>` | Imports session JSON/share URL. | Yes | Mutates local state. |
| `opencode github` | Manages the GitHub agent. | Mixed | `github run` is CI-oriented; `github install` is setup-oriented. |
| `opencode completion` | Generates shell completions. | Yes | yargs completion command. |
| `opencode upgrade [target]` | Upgrades OpenCode. | Mixed | Mutates installation. |
| `opencode uninstall` | Removes OpenCode files. | Mixed | `--force` skips prompts; destructive. |

## CLI Switch Inventory

The frontmatter contains the full switch inventory captured for wrappers. Summary by scope:

- Global: `--help`, `--version`, `--print-logs`, `--log-level`, `--pure`.
- Root/TUI/network: `--port`, `--hostname`, `--mdns`, `--mdns-domain`, `--cors`, `--model`, `--continue`, `--session`, `--fork`, `--prompt`, `--agent`, `--auto`, `--mini`, `--no-replay`, `--replay-limit`.
- `run`: `--command`, `--continue`, `--session`, `--fork`, `--share`, `--model`, `--agent`, `--format`, `--file`, `--title`, `--attach`, `--password`, `--username`, `--dir`, `--port`, `--variant`, `--thinking`, `--interactive`, `--auto`, plus hidden `--mini`, `--replay`, `--replay-limit`, `--yolo`, `--dangerously-skip-permissions`, and `--demo` in source.
- Server/web/ACP: `--port`, `--hostname`, `--mdns`, `--mdns-domain`, `--cors`; ACP also has `--cwd`.
- `agent create`: `--path`, `--description`, `--mode`, `--permissions`, `--tools`, `--model`.
- `plugin`: `--global`, `--force`.
- `models`: `--refresh`, `--verbose`.
- `session list`: `--max-count`, `--format`.
- `db`: `--format`.
- `stats`: `--days`, `--tools`, `--models`, `--project`.
- `export`: `--sanitize`.
- `github run`: `--event`, `--token`.
- `uninstall`: `--keep-config`, `--keep-data`, `--dry-run`, `--force`.
- `upgrade`: `--method`.

`opencode run --format json` emits newline-delimited JSON events on stdout. `opencode db <query> --format json` and `opencode session list --format json` emit regular JSON documents.

Help/docs disagreement: I trusted local `1.17.13` help and v1.17.13 source for executable behavior, and official docs for prose descriptions and environment/config lists. The important discrepancies are that local help exposes `providers` with alias `auth` while official docs list `auth`, and local help/source expose `console` while the official CLI page omits it.

System-prompt delivery flags: no native `opencode run` CLI flag for inline or file-backed system-prompt append/replace was observed in local help, official docs, or v1.17.13 `run.ts`. OpenCode's relevant surface is config `instructions` and agent prompts/files. Claudine's provider-specific append/replace semantics are owned by the sibling `system-prompt` topic.

## Configuration Discovery

OpenCode uses JSON or JSONC config. Config files are merged, not replaced. Current docs list this precedence order, later overriding earlier conflicting keys:

1. Remote config from `.well-known/opencode`.
2. Global config, normally `~/.config/opencode/opencode.json` or `.jsonc`.
3. Custom config from `OPENCODE_CONFIG`.
4. Project config, `opencode.json` or `.jsonc` in the project.
5. `.opencode` directories for agents, commands, modes, plugins, skills, tools, and themes.
6. Inline config from `OPENCODE_CONFIG_CONTENT`.
7. Managed config files such as `/Library/Application Support/opencode/`, `/etc/opencode/`, or `%ProgramData%\opencode`.
8. macOS managed preferences under the `ai.opencode.managed` MDM domain.

Global and project TUI-specific config use `tui.json` or `tui.jsonc`. `OPENCODE_TUI_CONFIG` can point to a custom TUI config.

Local inspection of `~/.config/opencode` on this host found `opencode.jsonc` containing only:

```json
{
  "$schema": "https://opencode.ai/config.json"
}
```

The same directory has a `package.json` declaring `@opencode-ai/plugin` and `node_modules`, which confirms the config directory can carry plugin dependencies. `opencode debug paths` showed this session's effective HOME is redirected to `/Users/ken/.claudine`, so local OpenCode paths resolve under `/Users/ken/.claudine/.config`, `.local/share`, `.cache`, and `.local/state` even though the binary itself is `/Users/ken/.opencode/bin/opencode`.

Provider credentials are stored in `~/.local/share/opencode/auth.json`. `opencode auth list` locally showed zero stored credentials and one environment-provided OpenAI credential, with styled human output rather than JSON.

## Environment Variables

General CLI/runtime variables are listed in frontmatter. Wrapper-relevant groups:

- Config and discovery: `OPENCODE_CONFIG`, `OPENCODE_TUI_CONFIG`, `OPENCODE_CONFIG_DIR`, `OPENCODE_CONFIG_CONTENT`, `OPENCODE_MODELS_URL`, `OPENCODE_DISABLE_MODELS_FETCH`.
- Runtime behavior: `OPENCODE_AUTO_SHARE`, `OPENCODE_GIT_BASH_PATH`, `OPENCODE_DISABLE_AUTOUPDATE`, `OPENCODE_DISABLE_PRUNE`, `OPENCODE_DISABLE_TERMINAL_TITLE`, `OPENCODE_DISABLE_AUTOCOMPACT`, `OPENCODE_DISABLE_MOUSE`.
- Cross-provider prompt/skill loading: `OPENCODE_DISABLE_CLAUDE_CODE`, `OPENCODE_DISABLE_CLAUDE_CODE_PROMPT`, `OPENCODE_DISABLE_CLAUDE_CODE_SKILLS`.
- Server/remote auth: `OPENCODE_SERVER_PASSWORD`, `OPENCODE_SERVER_USERNAME`.
- Plugin/tool behavior: `OPENCODE_DISABLE_DEFAULT_PLUGINS`, `OPENCODE_DISABLE_LSP_DOWNLOAD`, `OPENCODE_ENABLE_EXA`, `OPENCODE_ENABLE_EXPERIMENTAL_MODELS`.
- CLI-set child/runtime markers: `OPENCODE`, `OPENCODE_PID`, `OPENCODE_PRINT_LOGS`, `OPENCODE_LOG_LEVEL`, `OPENCODE_PURE`.
- Experimental toggles: `OPENCODE_EXPERIMENTAL`, `OPENCODE_EXPERIMENTAL_ICON_DISCOVERY`, `OPENCODE_EXPERIMENTAL_DISABLE_COPY_ON_SELECT`, `OPENCODE_EXPERIMENTAL_BASH_DEFAULT_TIMEOUT_MS`, `OPENCODE_EXPERIMENTAL_OUTPUT_TOKEN_MAX`, `OPENCODE_EXPERIMENTAL_FILEWATCHER`, `OPENCODE_EXPERIMENTAL_OXFMT`, `OPENCODE_EXPERIMENTAL_LSP_TOOL`, `OPENCODE_EXPERIMENTAL_DISABLE_FILEWATCHER`, `OPENCODE_EXPERIMENTAL_EXA`, `OPENCODE_EXPERIMENTAL_LSP_TY`, `OPENCODE_EXPERIMENTAL_PLAN_MODE`, `OPENCODE_EXPERIMENTAL_BACKGROUND_SUBAGENTS`, `OPENCODE_EXPERIMENTAL_EVENT_SYSTEM`, `OPENCODE_EXPERIMENTAL_NATIVE_LLM`, `OPENCODE_EXPERIMENTAL_PARALLEL`, `OPENCODE_EXPERIMENTAL_SCOUT`, and `OPENCODE_EXPERIMENTAL_WORKSPACES`.

Narrower model-endpoint/provider credentials, detailed permission policy behavior, MCP runtime injection, logging, and streaming semantics belong to their sibling research topics.

## Machine Introspection

| Command | Machine-readable? | Format | Useful for codegen? | Notes |
| --- | --- | --- | --- | --- |
| `opencode debug config` | Yes | JSON | Yes | Local probe confirmed resolved merged config JSON. |
| `opencode debug paths` | No | Text | No | Prints resolved home/data/bin/log/repos/cache/config/state/tmp paths. |
| `opencode debug skill` | Yes | JSON | No | Lists available skills. |
| `opencode models` | No | Text | Yes | Provider/model ids, one per line; parseable but not a JSON contract. |
| `opencode models --verbose` | No | Text + JSON blocks | Yes | Interleaves ids with pretty JSON metadata. |
| `opencode session list --format json` | Yes | JSON | No | Local probe confirmed session array JSON. |
| `opencode export <sessionID> --sanitize` | Yes | JSON | No | Redacted session export; omitting sessionID can prompt. |
| `opencode db <query> --format json` | Yes | JSON | No | Local `select 1 as one` probe returned a JSON array. |
| `opencode db path` | No | Text | No | Prints SQLite database path. |
| `opencode generate` | Yes | JSON | Yes | Local probe emitted OpenAPI 3.1.0 JSON. |
| `opencode providers list` / `opencode auth list` | No | Text | No | Shows credential file path and env-provided credentials; local output is styled human text. |

Generic `--help` and `--version` are not counted as machine introspection except as diagnostics.

## Wrapper Notes

Wrappers should invoke `opencode run` for non-interactive prompts. Running `opencode` without a subcommand starts the TUI.

`opencode run --format json` writes NDJSON events to stdout. The stream is not a complete lifecycle event bus; use process exit as the terminal signal. Reasoning output requires `--thinking`.

For richer progress and provider/model lifecycle signals, add `--print-logs --log-level INFO` and parse stderr as a second source. That intentionally makes stderr noisy even for successful runs.

Non-interactive `run` adds deny rules for question and plan enter/exit permissions by default. `--auto`, hidden `--yolo`, and hidden `--dangerously-skip-permissions` remove permission friction and should be explicit opt-ins, not wrapper defaults.

OpenCode loads global/project plugins and `.opencode` assets unless disabled. For deterministic wrapper runs, consider `--pure` and explicit config injection. Plugin/custom-tool dependencies can trigger package installation from the config directory.

`serve` and `web` are long-running. They are unauthenticated unless `OPENCODE_SERVER_PASSWORD` is set. Avoid passing credentials through `--password`/`--token` argv when an environment variable or config path can be used instead.

The local `1.17.13` binary exposes `providers` as canonical with `auth` as alias, while the official CLI page documents `auth`. Wrappers should probe or support both names. The local binary also exposes `console`, which the official CLI page currently omits.

No native run-level system-prompt delivery flag was observed. Use OpenCode config `instructions` or generated agents for provider-native prompt shaping, and defer Claudine append/replace semantics to the `system-prompt` research topic.

## Changelog

- 2026-07-03: Verified latest upstream and local installed OpenCode version remains `1.17.13`.
- 2026-07-03: Recorded local help/docs discrepancy for `providers` versus `auth`, and documented `console` as locally present but omitted from the official CLI page.
- 2026-07-03: Expanded the environment variable inventory to match the 2026-07-03 official CLI docs, including model-fetch and experimental feature variables.
- 2026-07-03: Reworked `config_paths` frontmatter into per-OS records required by `_schema.yaml`; removed invalid `os: all` records from the prior version.
- 2026-07-03: Confirmed local machine-readable probes for config, paths, skills, sessions, database, OpenAPI generation, and provider/auth listing.

## Sources

- [OpenCode homepage](https://opencode.ai)
- [OpenCode general docs](https://opencode.ai/docs/)
- [OpenCode CLI reference](https://opencode.ai/docs/cli/)
- [OpenCode config docs](https://opencode.ai/docs/config/)
- [OpenCode download page](https://opencode.ai/download)
- [OpenCode server docs](https://opencode.ai/docs/server/)
- [OpenCode plugins docs](https://opencode.ai/docs/plugins/)
- [OpenCode GitHub repository](https://github.com/anomalyco/opencode)
- [OpenCode GitHub releases](https://github.com/anomalyco/opencode/releases)
- [OpenCode v1.17.13 run command source](https://raw.githubusercontent.com/anomalyco/opencode/v1.17.13/packages/opencode/src/cli/cmd/run.ts)
- [OpenCode v1.17.13 CLI entry source](https://raw.githubusercontent.com/anomalyco/opencode/v1.17.13/packages/opencode/src/index.ts)
- Local command: `opencode --version` -> `1.17.13`
- Local command: `opencode --help`
- Local command: `opencode run --help`
- Local command: `opencode providers --help` and `opencode auth --help`
- Local command: `opencode console --help`
- Local command: `opencode debug config`
- Local command: `opencode debug paths`
- Local command: `opencode debug skill`
- Local command: `opencode models` and `opencode models --help`
- Local command: `opencode session list --format json`
- Local command: `opencode db path` and `opencode db 'select 1 as one' --format json`
- Local command: `opencode generate`
- Local command: `npm view opencode-ai version bin dist-tags --json --no-audit --no-fund`
- Local config inspection: `~/.config/opencode/opencode.jsonc` and `~/.config/opencode/package.json`
