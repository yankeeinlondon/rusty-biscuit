---
$schema: ./_schema.yaml
created: 2026-07-02
last_updated: 2026-07-03
agent: codex
model: default
latest_version: "7.4.1"
homepage: https://kilo.ai/
repo: https://github.com/Kilo-Org/kilocode
docs: https://kilo.ai/docs
cli_docs: https://kilo.ai/docs/code-with-ai/platforms/cli-reference
binaries:
  - os: macos
    binary: kilo
    alt_binaries: ["kilocode"]
    notes: "npm global install exposes both symlinks to @kilocode/cli/bin/kilo; standalone release assets are named kilo-*."
  - os: linux
    binary: kilo
    alt_binaries: ["kilocode"]
    notes: "npm global install exposes both package bins; standalone release assets are named kilo-*."
  - os: windows
    binary: kilo.cmd
    alt_binaries: ["kilocode.cmd", "kilo.ps1", "kilocode.ps1", "kilo.exe"]
    notes: "npm global installs normally expose .cmd/.ps1 shims; standalone release archives contain the platform binary."
install_methods:
  - os: macos
    method: npm
    command: "npm install -g @kilocode/cli"
    notes: "Official CLI docs and README list npm as the primary install method."
  - os: linux
    method: npm
    command: "npm install -g @kilocode/cli"
    notes: "Official CLI docs and README list npm as the primary install method."
  - os: windows
    method: npm
    command: "npm install -g @kilocode/cli"
    notes: "Official package manager method; creates Windows npm shims."
  - os: macos
    method: brew
    command: "brew install Kilo-Org/tap/kilo"
    notes: "README documents Homebrew for macOS and Linux."
  - os: linux
    method: brew
    command: "brew install Kilo-Org/tap/kilo"
    notes: "README documents Homebrew for macOS and Linux."
  - os: macos
    method: other
    command: "curl -fsSL https://kilo.ai/cli/install | bash"
    notes: "README documents curl installer."
  - os: linux
    method: other
    command: "curl -fsSL https://kilo.ai/cli/install | bash"
    notes: "README documents curl installer."
  - os: macos
    method: other
    command: "pnpm add -g @kilocode/cli"
    notes: "README documents pnpm global install."
  - os: linux
    method: other
    command: "pnpm add -g @kilocode/cli"
    notes: "README documents pnpm global install."
  - os: windows
    method: other
    command: "pnpm add -g @kilocode/cli"
    notes: "README documents pnpm global install."
  - os: macos
    method: other
    command: "bun add -g @kilocode/cli"
    notes: "README documents Bun global install."
  - os: linux
    method: other
    command: "bun add -g @kilocode/cli"
    notes: "README documents Bun global install."
  - os: windows
    method: other
    command: "bun add -g @kilocode/cli"
    notes: "README documents Bun global install."
  - os: macos
    method: standalone_binary
    command: "download kilo-darwin-arm64.zip or kilo-darwin-x64.zip from GitHub Releases"
    notes: "Use kilo-darwin-x64-baseline.zip for older x64 CPUs without AVX."
  - os: linux
    method: standalone_binary
    command: "download kilo-linux-x64.tar.gz or kilo-linux-arm64.tar.gz from GitHub Releases"
    notes: "README also notes musl builds for Alpine/minimal containers and baseline builds for older x64 CPUs."
  - os: windows
    method: standalone_binary
    command: "download kilo-windows-x64.zip from GitHub Releases"
    notes: "Use kilo-windows-x64-baseline.zip for older x64 CPUs without AVX."
  - os: linux
    method: other
    command: "paru -S kilo-bin"
    notes: "README documents Arch Linux AUR install."
subcommands:
  - name: "(default TUI)"
    description: "Starts the Kilo terminal UI, optionally in a project path."
    non_interactive: false
    notes: "Usage is `kilo [project]`; first-time provider setup uses interactive `/connect`."
  - name: completion
    description: "Generates a yargs shell completion script."
    non_interactive: true
    notes: "Observed output is zsh-style and calls `kilo --get-yargs-completions`."
  - name: acp
    description: "Starts an Agent Client Protocol server."
    non_interactive: true
    notes: "Long-running server command."
  - name: mcp
    description: "Manages MCP servers and OAuth flows."
    non_interactive: false
    notes: "`mcp list` is inspectable; auth/debug flows can require OAuth/browser interaction."
  - name: attach
    description: "Attaches to a running Kilo server URL."
    non_interactive: false
    notes: "Requires a server and may attach to interactive session state."
  - name: run
    description: "Runs Kilo with a message."
    non_interactive: true
    notes: "Automation entry point; use `--auto` and usually `--format json`."
  - name: debug
    description: "Troubleshooting tools for config, paths, skills, provider catalog, files, LSP, ripgrep, agents, snapshots, and startup."
    non_interactive: true
    notes: "`debug wait` intentionally waits forever; most other debug commands print inspectable state."
  - name: auth
    description: "Lists, logs into, or logs out of AI providers and credentials."
    non_interactive: false
    notes: "`auth list` is inspectable; `auth login` prompts or launches provider flows."
  - name: agent
    description: "Creates or lists agents."
    non_interactive: false
    notes: "`agent list` is inspectable; `agent create` may prompt/generate and writes agent files."
  - name: upgrade
    description: "Upgrades Kilo to latest or a target version."
    non_interactive: false
    notes: "Can invoke package managers; target and `--method` can reduce prompting."
  - name: uninstall
    description: "Uninstalls Kilo and optionally removes config/data."
    non_interactive: false
    notes: "Use `--dry-run` for inspection and `--force` to avoid confirmation."
  - name: serve
    description: "Starts a headless Kilo HTTP server."
    non_interactive: true
    notes: "Long-running server command."
  - name: web
    description: "Starts a Kilo server and opens the web interface."
    non_interactive: false
    notes: "Browser/open side effect."
  - name: models
    description: "Lists available models, optionally filtered by provider."
    non_interactive: true
    notes: "`--verbose` emits JSON object blocks in text, but there is no JSON-array mode."
  - name: stats
    description: "Shows token usage and cost statistics."
    non_interactive: true
    notes: "Observed help exposes filters but no JSON mode."
  - name: export
    description: "Exports session data as JSON."
    non_interactive: true
    notes: "Use `--sanitize` to redact transcript/file data."
  - name: import
    description: "Imports session data from a JSON file or share URL."
    non_interactive: false
    notes: "Mutates local session storage."
  - name: github
    description: "Installs or runs the GitHub agent."
    non_interactive: false
    notes: "`github run` can take `--event` and `--token`; install mutates local config."
  - name: pr
    description: "Fetches and checks out a GitHub PR branch, then runs Kilo."
    non_interactive: false
    notes: "Mutates git working tree state."
  - name: session
    description: "Lists or deletes local sessions."
    non_interactive: true
    notes: "`session list --format json` is machine-readable; delete mutates storage."
  - name: plugin
    description: "Installs a plugin and updates config."
    non_interactive: false
    notes: "Runs package installation and mutates config; alias `plug`."
  - name: db
    description: "Runs database tools, prints DB path, or migrates JSON data to SQLite."
    non_interactive: true
    notes: "Bare `kilo db` opens sqlite shell; pass a query and `--format json` for automation."
  - name: console
    description: "Opens the local Kilo Console."
    non_interactive: false
    notes: "Browser/UI oriented; `console stop --json` exists in docs but was not present in 7.3.45 top help."
  - name: roll-call
    description: "Batch-tests text models matching a regex."
    non_interactive: true
    notes: "Machine-readable JSON is available, but it performs live model calls."
  - name: profile
    description: "Shows Kilo account profile."
    non_interactive: true
    notes: "`--json` exits non-zero when not authenticated."
  - name: remote
    description: "Enables remote connection for real-time session relay."
    non_interactive: false
    notes: "Requires Kilo Gateway authentication."
  - name: daemon
    description: "Starts, stops, restarts, or checks the local daemon."
    non_interactive: true
    notes: "`daemon status --json` is machine-readable."
  - name: config
    description: "Runs configuration tools."
    non_interactive: true
    notes: "`config check` prints warnings/errors."
  - name: help
    description: "Shows CLI reference."
    non_interactive: true
    notes: "Docs advertise `--all` and `--format`; local 7.3.45 `kilo help --all --format md` emitted only top-level help."
cli_switches:
  - flag: --help
    value: ""
    scope: ["global"]
    default: "false"
    description: "Shows help."
    example: "kilo --help"
    notes: "Short form `-h`."
  - flag: --version
    value: ""
    scope: ["global"]
    default: "false"
    description: "Shows version number."
    example: "kilo --version"
    notes: "Short form `-v`."
  - flag: --print-logs
    value: ""
    scope: ["global", "logging"]
    default: "false"
    description: "Prints logs to stderr."
    example: "kilo --print-logs --help"
    notes: "Observed help commands print INFO lifecycle logs even without this flag."
  - flag: --log-level
    value: "DEBUG | INFO | WARN | ERROR"
    scope: ["global", "logging"]
    default: "unknown"
    description: "Sets log level."
    example: "kilo --log-level DEBUG --help"
    notes: ""
  - flag: --pure
    value: ""
    scope: ["global", "plugins"]
    default: "false"
    description: "Runs without external plugins."
    example: "kilo --pure run --auto 'summarize'"
    notes: "Equivalent intent to `KILO_PURE=1`."
  - flag: --port
    value: "<number>"
    scope: ["global", "acp", "serve", "web", "console", "server"]
    default: "0"
    description: "Sets server listen port."
    example: "kilo serve --port 4096"
    notes: "For `run`, local server port defaults to a random port if no value is provided."
  - flag: --hostname
    value: "<host>"
    scope: ["global", "acp", "serve", "web", "console", "server"]
    default: "127.0.0.1"
    description: "Sets server listen host."
    example: "kilo serve --hostname 127.0.0.1"
    notes: ""
  - flag: --mdns
    value: ""
    scope: ["global", "acp", "serve", "web", "console", "server"]
    default: "false"
    description: "Enables mDNS service discovery."
    example: "kilo serve --mdns"
    notes: "Help says this defaults hostname to 0.0.0.0."
  - flag: --mdns-domain
    value: "<domain>"
    scope: ["global", "acp", "serve", "web", "console", "server"]
    default: "kilo.local"
    description: "Sets custom mDNS service domain."
    example: "kilo serve --mdns-domain kilo.local"
    notes: ""
  - flag: --cors
    value: "<domain>"
    scope: ["global", "acp", "serve", "web", "console", "server"]
    default: "[]"
    description: "Adds allowed CORS domains."
    example: "kilo serve --cors https://example.com"
    notes: "Array/repeatable."
  - flag: --model
    value: "<provider/model>"
    scope: ["global", "run", "model_selection"]
    default: "config/provider default"
    description: "Selects model in provider/model format."
    example: "kilo run --model kilo/~anthropic/claude-haiku-latest --auto 'review'"
    notes: "Short form `-m`; top-level help also exposes it."
  - flag: --continue
    value: ""
    scope: ["global", "run", "attach", "sessions"]
    default: "false"
    description: "Continues the last session."
    example: "kilo run --continue"
    notes: "Short form `-c`; docs say it cannot be combined with autonomous mode or a prompt."
  - flag: --session
    value: "<id>"
    scope: ["global", "run", "attach", "sessions"]
    default: ""
    description: "Session id to continue."
    example: "kilo run --session ses_123"
    notes: "Short form `-s`."
  - flag: --fork
    value: ""
    scope: ["global", "run", "attach", "sessions"]
    default: "false"
    description: "Forks a session before continuing."
    example: "kilo run --session ses_123 --fork"
    notes: "Requires `--continue` or `--session`."
  - flag: --cloud-fork
    value: ""
    scope: ["global", "run", "attach", "sessions"]
    default: "false"
    description: "Fetches a cloud session and continues it locally."
    example: "kilo run --session ses_123 --cloud-fork"
    notes: "Requires `--session`."
  - flag: --prompt
    value: "<text>"
    scope: ["global", "roll-call"]
    default: "roll-call: Hello"
    description: "Top-level prompt to use, or roll-call prompt sent to each model."
    example: "kilo --prompt 'inspect this repo'"
    notes: "This is not a system-prompt delivery flag."
  - flag: --agent
    value: "<name>"
    scope: ["global", "run"]
    default: "config default"
    description: "Selects active agent."
    example: "kilo run --agent plan 'design migration'"
    notes: ""
  - flag: --command
    value: "<command>"
    scope: ["run"]
    default: ""
    description: "Runs a named command; message supplies args."
    example: "kilo run --command review"
    notes: ""
  - flag: --share
    value: ""
    scope: ["run", "sessions"]
    default: "false"
    description: "Shares the session."
    example: "kilo run --share 'summarize this repo'"
    notes: ""
  - flag: --format
    value: "default | json"
    scope: ["run"]
    default: "default"
    description: "Selects formatted output or raw JSON events."
    example: "kilo run --format json --auto 'summarize this repo'"
    notes: "Wrapper-grade event output; unsupported values exit 1 and print help."
  - flag: --file
    value: "<path>"
    scope: ["run", "context"]
    default: "[]"
    description: "Attaches file(s) to the message."
    example: "kilo run --file src/lib.rs 'explain this file'"
    notes: "Short form `-f`; array/repeatable."
  - flag: --title
    value: "<title>"
    scope: ["run", "sessions"]
    default: "truncated prompt"
    description: "Sets session title."
    example: "kilo run --title 'CI review' 'review diff'"
    notes: ""
  - flag: --attach
    value: "<url>"
    scope: ["run", "server"]
    default: ""
    description: "Attaches a run to a running Kilo server."
    example: "kilo run --attach http://localhost:4096 'continue work'"
    notes: ""
  - flag: --password
    value: "<password>"
    scope: ["run", "attach", "server_auth"]
    default: "KILO_SERVER_PASSWORD"
    description: "Basic auth password for server attachment."
    example: "kilo attach http://localhost:4096 --password \"$KILO_SERVER_PASSWORD\""
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
    description: "Directory to run in, or remote path when attaching."
    example: "kilo run --dir /repo 'inspect this project'"
    notes: ""
  - flag: --variant
    value: "<variant>"
    scope: ["run", "model_selection"]
    default: ""
    description: "Selects provider-specific reasoning/model variant."
    example: "kilo run --variant high 'solve this bug'"
    notes: "Examples in help: high, max, minimal."
  - flag: --thinking
    value: ""
    scope: ["run", "output"]
    default: "false"
    description: "Shows thinking blocks."
    example: "kilo run --thinking 'debug this failure'"
    notes: ""
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
    description: "Auto-approves permissions not explicitly denied."
    example: "kilo run --dangerously-skip-permissions 'fix lint'"
    notes: "Dangerous for wrappers unless paired with restrictive config."
  - flag: --auto
    value: ""
    scope: ["run", "automation", "permissions"]
    default: "false"
    description: "Auto-approves all permissions for autonomous/pipeline usage."
    example: "kilo run --auto 'implement feature X'"
    notes: "Official autonomous mode entry point."
  - flag: --demo
    value: ""
    scope: ["run"]
    default: "false"
    description: "Enables direct interactive demo slash commands."
    example: "kilo run --demo"
    notes: ""
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
    description: "Provider id or name to log into."
    example: "kilo auth login --provider openai"
    notes: "Short form `-p`."
  - flag: --method
    value: "curl | npm | pnpm | bun | brew | choco | scoop"
    scope: ["auth login", "upgrade"]
    default: ""
    description: "Selects login method or upgrade install method."
    example: "kilo upgrade --method npm"
    notes: "Local upgrade help omits yarn from older draft research."
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
    description: "Shows uninstall targets without removing them."
    example: "kilo uninstall --dry-run"
    notes: ""
  - flag: --force
    value: ""
    scope: ["uninstall", "plugin"]
    default: "false"
    description: "Skips uninstall confirmation or replaces existing plugin version."
    example: "kilo uninstall --force"
    notes: "Short form `-f`."
  - flag: --verbose
    value: ""
    scope: ["models", "roll-call"]
    default: "false"
    description: "Shows verbose output."
    example: "kilo models --verbose"
    notes: "For models, includes JSON object metadata per model."
  - flag: --refresh
    value: ""
    scope: ["models"]
    default: "false"
    description: "Refreshes model cache from models.dev."
    example: "kilo models --refresh"
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
    example: "kilo roll-call 'kilo/.*' --parallel 3"
    notes: ""
  - flag: --quiet
    value: ""
    scope: ["roll-call"]
    default: "false"
    description: "Suppresses progress and decoration."
    example: "kilo roll-call 'kilo/.*' --output json --quiet"
    notes: ""
  - flag: --output
    value: "table | json | md"
    scope: ["roll-call"]
    default: "table"
    description: "Selects roll-call output format."
    example: "kilo roll-call 'kilo/.*' --output json"
    notes: ""
  - flag: --json
    value: ""
    scope: ["profile", "daemon status"]
    default: "false"
    description: "Prints selected command output as JSON."
    example: "kilo daemon status --json"
    notes: "Local 7.3.45 observed this on profile and daemon status."
  - flag: --days
    value: "<number>"
    scope: ["stats"]
    default: "all time"
    description: "Shows stats for last N days."
    example: "kilo stats --days 7"
    notes: ""
  - flag: --tools
    value: "<number-or-permissions>"
    scope: ["stats", "agent create"]
    default: "stats: all; agent create: all"
    description: "Shows top N tools in stats, or aliases agent-create permissions."
    example: "kilo stats --tools 10"
    notes: "Context-dependent flag."
  - flag: --models
    value: "<number>"
    scope: ["stats"]
    default: "hidden"
    description: "Shows model statistics, optionally top N."
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
    description: "GitHub token."
    example: "kilo github run --token github_pat_..."
    notes: "Credential-bearing flag; wrappers should avoid logging."
  - flag: --max-count
    value: "<number>"
    scope: ["session list"]
    default: ""
    description: "Limits session list to N sessions."
    example: "kilo session list --max-count 20 --format json"
    notes: "Short form `-n`."
  - flag: --search
    value: "<text>"
    scope: ["session list"]
    default: ""
    description: "Filters sessions by title."
    example: "kilo session list --search migration"
    notes: "Short form `-s`."
  - flag: --all
    value: ""
    scope: ["session list", "help"]
    default: "false"
    description: "Includes all projects for session list, or all commands for help."
    example: "kilo session list --all --format json"
    notes: "In local 7.3.45, `kilo help --all --format md` still emitted only top-level help."
  - flag: --foreground
    value: ""
    scope: ["daemon start", "daemon restart", "console"]
    default: "false"
    description: "Keeps command active until interrupted."
    example: "kilo daemon start --foreground"
    notes: "Short form `-f`."
  - flag: --global
    value: ""
    scope: ["plugin"]
    default: "false"
    description: "Installs plugin in global config."
    example: "kilo plugin @scope/plugin --global"
    notes: "Short form `-g`."
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
    description: "Agent description."
    example: "kilo agent create --description 'Reviews migrations'"
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
    description: "Comma-separated permissions to allow."
    example: "kilo agent create --permissions read,grep --mode subagent"
    notes: "Alias: `--tools`."
  - flag: --query
    value: "<query>"
    scope: ["debug rg files"]
    default: ""
    description: "Filters debug file listing by query."
    example: "kilo debug rg files --query cli"
    notes: ""
  - flag: --glob
    value: "<glob>"
    scope: ["debug rg files", "debug rg search"]
    default: ""
    description: "Filters files/search by glob."
    example: "kilo debug rg search TODO --glob '*.rs'"
    notes: "Array for search."
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
    description: "Tool params as JSON or JavaScript object literal."
    example: "kilo debug agent build --tool read --params '{path:\"README.md\"}'"
    notes: "Shell quoting is wrapper-sensitive."
  - flag: --system-prompt
    value: "<text-or-path>"
    scope: ["run", "system-prompt"]
    default: "unsupported"
    description: "No supported system-prompt delivery flag was found in installed 7.3.45."
    example: "kilo run --system-prompt 'x' 'task'"
    notes: "Negative probe: `--system-prompt`, `--append-system-prompt`, and `--replace-system-prompt` all exited 1 and printed help. Defer semantics to the sibling system-prompt topic if Kilo adds these flags later."
config_paths:
  - os: macos
    scope: user
    path: "~/.config/kilo/kilo.jsonc"
    format: jsonc
    notes: "Primary user config observed locally and documented; `.json` and `config.json` are also supported."
  - os: linux
    scope: user
    path: "~/.config/kilo/kilo.jsonc"
    format: jsonc
    notes: "Primary user config documented for XDG Linux; `.json` and `config.json` are also supported."
  - os: windows
    scope: user
    path: "%APPDATA%\\kilo\\kilo.jsonc"
    format: jsonc
    notes: "Docs say Windows config dir may vary; this is the expected roaming app-data equivalent."
  - os: macos
    scope: user
    path: "~/.config/kilo/tui.jsonc"
    format: jsonc
    notes: "TUI notifications, sounds, themes, and keybindings; `.json` also supported."
  - os: linux
    scope: user
    path: "~/.config/kilo/tui.jsonc"
    format: jsonc
    notes: "TUI notifications, sounds, themes, and keybindings; `.json` also supported."
  - os: windows
    scope: user
    path: "%APPDATA%\\kilo\\tui.jsonc"
    format: jsonc
    notes: "Expected Windows equivalent; docs say Windows config dir may vary."
  - os: macos
    scope: repo
    path: "./kilo.jsonc"
    format: jsonc
    notes: "Project config takes precedence over global settings; `.json` supported."
  - os: linux
    scope: repo
    path: "./kilo.jsonc"
    format: jsonc
    notes: "Project config takes precedence over global settings; `.json` supported."
  - os: windows
    scope: repo
    path: ".\\kilo.jsonc"
    format: jsonc
    notes: "Project config takes precedence over global settings; `.json` supported."
  - os: macos
    scope: repo
    path: "./.kilo/kilo.jsonc"
    format: jsonc
    notes: "Project directory config; docs also mention legacy `.kilocode` and `.opencode` discovery."
  - os: linux
    scope: repo
    path: "./.kilo/kilo.jsonc"
    format: jsonc
    notes: "Project directory config; docs also mention legacy `.kilocode` and `.opencode` discovery."
  - os: windows
    scope: repo
    path: ".\\.kilo\\kilo.jsonc"
    format: jsonc
    notes: "Project directory config; docs also mention legacy `.kilocode` and `.opencode` discovery."
  - os: macos
    scope: user
    path: "~/.local/share/kilo/kilo.db"
    format: other
    notes: "Local inspection showed SQLite session/state DB, WAL/SHM sidecars, session-export DB, logs, repos, and telemetry-id under data dir."
  - os: linux
    scope: user
    path: "~/.local/share/kilo/kilo.db"
    format: other
    notes: "XDG data path equivalent reported by local `kilo debug paths` on macOS."
  - os: windows
    scope: user
    path: "%LOCALAPPDATA%\\kilo\\kilo.db"
    format: other
    notes: "Expected Windows local data equivalent; not locally verified."
env_vars:
  - name: KILO_PROVIDER
    effect: "Overrides the active provider ID."
  - name: KILO_<FIELD_NAME>
    effect: "Overrides provider/config fields for non-kilocode providers, for example KILO_API_KEY maps to apiKey."
  - name: KILOCODE_<FIELD_NAME>
    effect: "Overrides fields for the `kilocode` provider, for example KILOCODE_MODEL maps to kilocodeModel."
  - name: KILO_PURE
    effect: "When set to 1, skips external plugins; useful for reproducible CI or debugging."
  - name: KILO_SERVER_PASSWORD
    effect: "Default basic auth password for `kilo run --attach` and `kilo attach`."
  - name: KILO_SERVER_USERNAME
    effect: "Default basic auth username for `kilo run --attach` and `kilo attach`; falls back to `kilo`."
  - name: KILO_TREE_SITTER_WASM_DIR
    effect: "Overrides the tree-sitter WASM resource directory; npm launcher sets it to the co-located package resource directory when absent."
machine_introspection:
  - command: "kilo debug paths"
    purpose: env
    machine_readable: false
    output_format: table
    useful_for_codegen: true
    notes: "Prints resolved home, data, bin, log, repos, cache, config, state, and tmp paths."
  - command: "kilo debug config"
    purpose: config_dump
    machine_readable: true
    output_format: json
    useful_for_codegen: true
    notes: "Prints resolved configuration; can be very large because built-in/custom agents include full prompts."
  - command: "kilo config check"
    purpose: doctor
    machine_readable: false
    output_format: text
    useful_for_codegen: false
    notes: "Prints configuration warnings/errors; local clean output was `No config warnings.`"
  - command: "kilo debug info"
    purpose: doctor
    machine_readable: false
    output_format: text
    useful_for_codegen: false
    notes: "Prints version, OS, terminal, and plugin summary."
  - command: "kilo debug v2"
    purpose: capabilities
    machine_readable: true
    output_format: json
    useful_for_codegen: true
    notes: "Prints enabled providers, defaults, provider endpoint metadata, and model maps."
  - command: "kilo debug skill"
    purpose: tools
    machine_readable: true
    output_format: json
    useful_for_codegen: true
    notes: "Prints available skills with name, description, location, and content; large output."
  - command: "kilo models --verbose"
    purpose: models
    machine_readable: false
    output_format: text
    useful_for_codegen: true
    notes: "Emits repeated `provider/model` labels followed by JSON objects; no JSON array mode in help."
  - command: "kilo session list --format json --max-count N"
    purpose: other
    machine_readable: true
    output_format: json
    useful_for_codegen: false
    notes: "Lists local sessions; empty local result produced empty stdout with exit 0."
  - command: "kilo daemon status --json"
    purpose: doctor
    machine_readable: true
    output_format: json
    useful_for_codegen: false
    notes: "Reports daemon running/stale/file/reason state."
  - command: "kilo db path"
    purpose: env
    machine_readable: false
    output_format: text
    useful_for_codegen: false
    notes: "Prints resolved SQLite database path."
  - command: "kilo db '<query>' --format json"
    purpose: config_dump
    machine_readable: true
    output_format: json
    useful_for_codegen: false
    notes: "Runs arbitrary SQLite query against Kilo DB; useful for diagnostics, risky for wrappers unless query is fixed/read-only."
  - command: "kilo auth list"
    purpose: env
    machine_readable: false
    output_format: text
    useful_for_codegen: false
    notes: "Shows credential file and environment-sourced providers."
  - command: "kilo mcp list"
    purpose: mcp
    machine_readable: false
    output_format: text
    useful_for_codegen: false
    notes: "Lists MCP server status; local empty state exited 0 with human text."
  - command: "kilo profile --json"
    purpose: env
    machine_readable: true
    output_format: json
    useful_for_codegen: false
    notes: "Only useful when authenticated; local unauthenticated run exited 1 with a styled error."
wrapper_notes:
  - "Installed local version is 7.3.45, but npm latest on 2026-07-03 is 7.4.1; wrappers should not assume local and upstream latest match."
  - "The npm package exposes both `kilo` and `kilocode`; both local symlinks target the same launcher."
  - "The npm launcher spawns a platform binary and forwards SIGINT, SIGTERM, and SIGHUP; process-tree handling should account for this wrapper layer."
  - "Most `--help` commands initialize file/db services and print INFO lines to stderr after help text, even without `--print-logs`; help capture should tolerate noisy stderr."
  - "`kilo run --auto --format json` is the main non-interactive execution shape. Without `--auto`, Kilo can prompt for approvals."
  - "The installed 7.3.45 binary rejects `--system-prompt`, `--append-system-prompt`, and `--replace-system-prompt`; system-prompt customization appears to be config/agent based, not a run flag."
  - "`kilo models --verbose` is useful but not clean JSON; it emits labels plus JSON object blocks and provider filters can exit 1 when the provider id is unknown."
  - "`kilo help --all --format md` did not provide a full all-command reference locally despite official docs listing `--all` and `--format`; prefer per-command help or official CLI reference for full inventories."
  - "Local first inspection created/used `~/.config/kilo/kilo.jsonc`, `~/.local/share/kilo/kilo.db`, WAL/SHM sidecars, log files, `session-export.db`, and `telemetry-id`."
  - "Commands that open UI or mutate host state include default TUI, `web`, `console`, `plugin`, `import`, `pr`, `github install`, `upgrade`, and `uninstall`."
  - "`profile --json` exits 1 when not authenticated; this is an expected state, not necessarily a wrapper failure."
changes:
  - "Updated upstream latest version from 7.3.54 to npm latest 7.4.1 while recording local installed 7.3.45."
  - "Replaced invalid schema records using `os: all` with explicit macOS, Linux, and Windows records."
  - "Added observed global `--pure`, server flags, root `--prompt`, noisy help stderr, npm launcher behavior, and local XDG state paths."
  - "Added negative probes showing Kilo 7.3.45 rejects system-prompt delivery flags."
  - "Expanded machine-introspection coverage for debug paths/config/v2/skill, daemon status JSON, DB query JSON, auth, MCP, profile, and models."
requires_claudine_update: true
reason: "Claudine provider metadata for Kilo should account for npm latest 7.4.1, the `kilo`/`kilocode` binary aliases, `run --auto --format json` automation, noisy stderr during successful help/introspection, lack of system-prompt run flags in 7.3.45, and per-OS schema-valid install/config records."
---

# Kilo Code CLI Surface

## Overview

Kilo Code is Kilo's open source agentic coding product for VS Code, JetBrains, and the terminal. The public CLI is shipped from the `Kilo-Org/kilocode` repository and the npm package `@kilocode/cli`. The primary command users type is `kilo`; the npm package also installs `kilocode` as an alias to the same launcher.

The current upstream version I verified is `7.4.1`, from `npm view @kilocode/cli version dist-tags bin repository homepage --json` on 2026-07-03. The locally installed npm package and binary are `7.3.45`, verified with `kilo --version`, `kilocode --version`, and `/Users/ken/.nvm/versions/node/v22.20.0/lib/node_modules/@kilocode/cli/package.json`. This document treats `7.4.1` as the upstream latest and local `7.3.45` as the behavioral evidence for help output and config/state discovery.

Primary sources:

- Homepage: [https://kilo.ai/](https://kilo.ai/)
- Repository: [https://github.com/Kilo-Org/kilocode](https://github.com/Kilo-Org/kilocode)
- General docs: [https://kilo.ai/docs](https://kilo.ai/docs)
- CLI overview: [https://kilo.ai/docs/code-with-ai/platforms/cli](https://kilo.ai/docs/code-with-ai/platforms/cli)
- CLI reference: [https://kilo.ai/docs/code-with-ai/platforms/cli-reference](https://kilo.ai/docs/code-with-ai/platforms/cli-reference)

## Installation and Binaries

The npm package `@kilocode/cli` exposes two bin names:

| OS | Primary command | Aliases/shims | Notes |
| --- | --- | --- | --- |
| macOS | `kilo` | `kilocode` | Local npm install creates symlinks to `../lib/node_modules/@kilocode/cli/bin/kilo`. |
| Linux | `kilo` | `kilocode` | Same npm bin names; standalone assets are platform archives. |
| Windows | `kilo.cmd` | `kilocode.cmd`, `kilo.ps1`, `kilocode.ps1`, `kilo.exe` | npm creates command/PowerShell shims; standalone archives contain the Windows binary. |

Official install commands and release assets:

| OS | Method | Command or asset |
| --- | --- | --- |
| macOS/Linux/Windows | npm | `npm install -g @kilocode/cli` |
| macOS/Linux/Windows | pnpm | `pnpm add -g @kilocode/cli` |
| macOS/Linux/Windows | Bun | `bun add -g @kilocode/cli` |
| macOS/Linux | Homebrew | `brew install Kilo-Org/tap/kilo` |
| macOS/Linux | curl installer | `curl -fsSL https://kilo.ai/cli/install \| bash` |
| Linux | Arch AUR | `paru -S kilo-bin` |
| macOS | GitHub Releases | `kilo-darwin-arm64.zip`, `kilo-darwin-x64.zip`, or `kilo-darwin-x64-baseline.zip` |
| Linux | GitHub Releases | `kilo-linux-x64.tar.gz`, `kilo-linux-arm64.tar.gz`; docs also note musl/baseline variants |
| Windows | GitHub Releases | `kilo-windows-x64.zip` or `kilo-windows-x64-baseline.zip` |

The local npm launcher is a Node script that locates/spawns the packaged platform binary, sets `KILO_TREE_SITTER_WASM_DIR` to the co-located tree-sitter WASM directory when absent, and forwards `SIGINT`, `SIGTERM`, and `SIGHUP`.

## Subcommands

| Command | Description | Automation / interaction notes |
| --- | --- | --- |
| `kilo [project]` | Starts the terminal UI. | Interactive; first-time provider setup uses `/connect`. |
| `completion` | Generates shell completion script. | Non-interactive. |
| `acp` | Starts an ACP server. | Non-interactive but long-running. |
| `mcp` | Manages MCP servers, OAuth auth, logout, and debug flows. | `mcp list` is inspectable; auth/debug can require browser/OAuth interaction. |
| `attach <url>` | Attaches to a running Kilo server. | Usually interactive or server-dependent. |
| `run [message..]` | Runs Kilo with a message. | Main automation entry point; use `--auto --format json`. |
| `debug` | Troubleshooting tools for config, paths, skills, provider catalog, LSP, ripgrep, files, snapshots, agents, and startup. | Mostly non-interactive; `debug wait` intentionally waits indefinitely. |
| `auth` / `providers` | Manages AI providers and credentials. | `auth list` is inspectable; login/logout are interactive or mutating. |
| `agent` | Creates or lists agents. | `agent list` is inspectable; `agent create` may prompt/generate and writes files. |
| `upgrade [target]` | Upgrades Kilo. | Mutating; can invoke package managers. |
| `uninstall` | Removes Kilo and related files. | Mutating; use `--dry-run` and `--force` for automation. |
| `serve` | Starts a headless HTTP server. | Non-interactive but long-running. |
| `web` | Starts a server and opens the web UI. | Browser/open side effect. |
| `models [provider]` | Lists models. | Non-interactive; `--verbose` is structured-ish but not clean JSON. |
| `stats` | Shows usage/cost statistics. | Non-interactive human output. |
| `export [sessionID]` | Exports session data as JSON. | Non-interactive; `--sanitize` redacts sensitive data. |
| `import <file>` | Imports session JSON or share URL. | Mutates local session storage. |
| `github` | Installs or runs the GitHub agent. | Mutating/auth-sensitive. |
| `pr <number>` | Fetches/checks out a GitHub PR branch, then runs Kilo. | Mutates git state. |
| `session` | Lists or deletes sessions. | `session list --format json` is machine-readable; delete mutates. |
| `plugin` / `plug` | Installs a plugin and updates config. | Mutates config and may install packages. |
| `db` | Opens sqlite shell, prints DB path, migrates data, or runs a SQL query. | Query form with `--format json` is non-interactive; bare command is interactive. |
| `console` | Opens local Kilo Console. | UI/browser oriented. |
| `roll-call <filter>` | Batch-tests text models. | Non-interactive with `--output json`, but performs live model calls. |
| `profile` | Shows Kilo account profile. | `--json` is machine-readable when authenticated; exits 1 if not authenticated. |
| `remote` | Enables real-time remote connection. | Requires Kilo Gateway authentication. |
| `daemon` | Manages local daemon. | `daemon status --json` is machine-readable; start/restart are long-running with `--foreground`. |
| `config` | Configuration tools. | `config check` is non-interactive text diagnostics. |
| `help [command]` | Shows CLI reference. | Non-interactive, but local `--all --format md` did not expand all commands. |

## CLI Switch Inventory

Observed global options in local `7.3.45`: `-h, --help`; `-v, --version`; `--print-logs`; `--log-level DEBUG|INFO|WARN|ERROR`; `--pure`; server options `--port`, `--hostname`, `--mdns`, `--mdns-domain`, `--cors`; session/model shortcuts `-m, --model`, `-c, --continue`, `-s, --session`, `--fork`, `--cloud-fork`, `--prompt`, and `--agent`.

Wrapper-relevant `run` options:

| Flag | Type | Default | Example | Notes |
| --- | --- | --- | --- | --- |
| `--command <command>` | value | unset | `kilo run --command review` | Runs a named command; message supplies args. |
| `--continue`, `-c` | boolean | false | `kilo run --continue` | Docs say not with autonomous mode or prompt. |
| `--session <id>`, `-s` | value | unset | `kilo run --session ses_123` | Continue specific session. |
| `--fork` | boolean | false | `kilo run --session ses_123 --fork` | Requires `--continue` or `--session`. |
| `--cloud-fork` | boolean | false | `kilo run --session ses_123 --cloud-fork` | Requires `--session`. |
| `--share` | boolean | false | `kilo run --share 'summarize'` | Shares the session. |
| `--model <provider/model>`, `-m` | value | config/default | `kilo run --model kilo/~anthropic/claude-haiku-latest --auto 'review'` | Provider-prefixed model id. |
| `--agent <name>` | value | config/default | `kilo run --agent plan 'design this'` | Selects active agent/mode. |
| `--format default\|json` | value | `default` | `kilo run --format json --auto 'summarize'` | JSON is the wrapper-grade event stream. Unsupported values exit 1. |
| `--file <path>`, `-f` | repeatable value | `[]` | `kilo run --file src/lib.rs 'explain'` | Attaches files. |
| `--title <title>` | value | truncated prompt | `kilo run --title 'CI review' 'review diff'` | Sets session title. |
| `--attach <url>` | value | unset | `kilo run --attach http://localhost:4096 'continue'` | Attaches to server. |
| `--password <password>`, `-p` | value | `KILO_SERVER_PASSWORD` | `kilo attach http://localhost:4096 --password "$KILO_SERVER_PASSWORD"` | Server basic auth. |
| `--username <username>`, `-u` | value | `KILO_SERVER_USERNAME` or `kilo` | `kilo attach http://localhost:4096 --username kilo` | Server basic auth. |
| `--dir <path>` | value | current directory | `kilo run --dir /repo 'inspect'` | Working directory or remote path. |
| `--port <number>` | value | random for run server | `kilo run --port 4096 'task'` | Local server port. |
| `--variant <variant>` | value | unset | `kilo run --variant high 'solve'` | Provider-specific reasoning effort. |
| `--thinking` | boolean | false | `kilo run --thinking 'debug'` | Shows thinking blocks. |
| `--interactive`, `-i` | boolean | false | `kilo run --interactive 'start'` | Direct interactive split-footer mode. |
| `--dangerously-skip-permissions` | boolean | false | `kilo run --dangerously-skip-permissions 'fix lint'` | Auto-approves permissions not explicitly denied. |
| `--auto` | boolean | false | `kilo run --auto 'Implement feature X'` | Official autonomous mode. |
| `--demo` | boolean | false | `kilo run --demo` | Demo slash commands. |

Other observed scoped switches:

| Scope | Switches |
| --- | --- |
| `acp` | server flags plus `--cwd <path>` |
| `auth login` | `--provider <id-or-name>`, `--method <label>` |
| `upgrade` | `--method curl\|npm\|pnpm\|bun\|brew\|choco\|scoop` |
| `uninstall` | `--keep-config`, `--keep-data`, `--dry-run`, `--force` |
| `models` | `--verbose`, `--refresh` |
| `roll-call` | `--prompt <text>`, `--timeout <ms>`, `--parallel <n>`, `--verbose`, `--quiet`, `--output table\|json\|md` |
| `profile` | `--json` |
| `stats` | `--days <n>`, `--tools <n>`, `--models [n]`, `--project <project>` |
| `export` | `--sanitize` |
| `github run` | `--event <event>`, `--token <github_pat>` |
| `session list` | `--max-count <n>`, `--format table\|json`, `--all`, `--search <text>` |
| `daemon start/restart` | `--foreground` |
| `daemon status` | `--json` |
| `db` | `--format json\|tsv` for query mode |
| `plugin` | `--global`, `--force` |
| `agent create` | `--path <path>`, `--description <text>`, `--mode all\|primary\|subagent`, `--permissions <list>`, `--tools <list>` |
| `debug rg` | `--query`, `--glob`, `--limit` depending on subcommand |
| `debug agent` | `--tool <id>`, `--params <json-or-js-object>` |
| `help` | docs list `--all` and `--format md\|text`; local `7.3.45` did not expand all commands with `--all` |

System-prompt delivery flags: none were supported by the installed `7.3.45` binary. Negative probes for `kilo run --system-prompt test`, `kilo run --append-system-prompt x test`, and `kilo run --replace-system-prompt x test` all exited 1 and printed help. The plain `--prompt` flag is an initial user prompt, not a system-prompt override. If Kilo adds system-prompt flags later, semantics belong in the sibling `system-prompt` research topic.

When official docs and local help disagree, this document trusts local help for wrapper behavior and official docs for install paths and cross-platform release assets. The most important disagreement observed was `kilo help --all --format md`: official CLI reference documents those flags, but the local command emitted only top-level help.

## Configuration Discovery

Kilo uses XDG-style config and data directories on macOS/Linux. On this host, `HOME` was `/Users/ken/.claudine`; `kilo debug paths` resolved:

| Kind | Local resolved path |
| --- | --- |
| home | `/Users/ken/.claudine` |
| config | `/Users/ken/.claudine/.config/kilo` |
| data | `/Users/ken/.claudine/.local/share/kilo` |
| state | `/Users/ken/.claudine/.local/state/kilo` |
| cache | `/Users/ken/.claudine/.cache/kilo` |
| bin | `/Users/ken/.claudine/.cache/kilo/bin` |
| log | `/Users/ken/.claudine/.local/share/kilo/log` |
| repos | `/Users/ken/.claudine/.local/share/kilo/repos` |
| tmp | `/var/folders/.../T/kilo` |

Config files and scopes:

| Scope | macOS/Linux path | Windows path | Format | Notes |
| --- | --- | --- | --- | --- |
| User | `~/.config/kilo/kilo.jsonc` | `%APPDATA%\kilo\kilo.jsonc` | JSONC | Local file contained only `$schema: https://app.kilo.ai/config.json`; docs also allow `.json` and `config.json`. |
| User TUI | `~/.config/kilo/tui.jsonc` | `%APPDATA%\kilo\tui.jsonc` | JSONC | Notifications, sounds, themes, keybindings; `.json` also supported. |
| Project | `./kilo.jsonc` | `.\kilo.jsonc` | JSONC | Project config takes precedence over global. |
| Project directory | `./.kilo/kilo.jsonc` | `.\.kilo\kilo.jsonc` | JSONC | Docs also mention legacy `.kilocode` and `.opencode` discovery. |
| User data | `~/.local/share/kilo/kilo.db` | `%LOCALAPPDATA%\kilo\kilo.db` | SQLite | Local inspection showed DB, WAL/SHM sidecars, logs, repos, `session-export.db`, and `telemetry-id`. |

Side effects observed on first/local runs: Kilo initialized file and DB services during help/debug commands, opened/applied migrations to `~/.local/share/kilo/kilo.db`, wrote logs under `~/.local/share/kilo/log`, and maintained `telemetry-id`. `auth list` displayed credential storage at `~/.local/share/kilo/auth.json` even when the file did not yet contain credentials.

## Environment Variables

General CLI/runtime variables:

| Variable | Effect |
| --- | --- |
| `KILO_PROVIDER` | Overrides the active provider id. |
| `KILO_<FIELD_NAME>` | Overrides provider/config fields for non-`kilocode` providers, for example `KILO_API_KEY` maps to `apiKey`. |
| `KILOCODE_<FIELD_NAME>` | Overrides fields for the `kilocode` provider, for example `KILOCODE_MODEL` maps to `kilocodeModel`. |
| `KILO_PURE` | `KILO_PURE=1` skips external plugins; useful for reproducible CI or debugging. |
| `KILO_SERVER_PASSWORD` | Default basic auth password for `run --attach` and `attach`. |
| `KILO_SERVER_USERNAME` | Default basic auth username for `run --attach` and `attach`; falls back to `kilo`. |
| `KILO_TREE_SITTER_WASM_DIR` | Overrides tree-sitter WASM resource directory; the npm launcher sets it to the package's bundled directory when absent. |

Kilo also documents `{env:VARIABLE_NAME}` interpolation inside config files. OpenTelemetry variables are documented by Kilo, but those belong to the logging topic unless a wrapper needs to control telemetry explicitly.

## Machine Introspection

| Command | Machine-readable | Format | Codegen use | Notes |
| --- | --- | --- | --- | --- |
| `kilo debug paths` | No | table/text | Yes | Resolved home/data/config/cache/state/tmp paths. |
| `kilo debug config` | Yes | JSON | Yes | Resolved config; very large because agent prompts are included. |
| `kilo config check` | No | text | No | Doctor-style config warnings/errors. |
| `kilo debug info` | No | text | No | Version, OS, terminal, plugins. |
| `kilo debug v2` | Yes | JSON | Yes | Enabled providers, endpoints, defaults, model maps. |
| `kilo debug skill` | Yes | JSON | Yes | Available skills with full content; large and potentially sensitive. |
| `kilo models --verbose` | Partly | text with JSON object blocks | Yes | Model catalog and metadata, but no clean JSON-array mode. |
| `kilo session list --format json --max-count N` | Yes | JSON | No | Local empty state produced empty stdout and exit 0. |
| `kilo daemon status --json` | Yes | JSON | No | Daemon running/stale/file/reason. |
| `kilo db path` | No | text | No | Resolved SQLite DB path. |
| `kilo db '<query>' --format json` | Yes | JSON | No | Arbitrary DB query; wrappers should keep queries fixed/read-only. |
| `kilo auth list` | No | text | No | Credentials and environment-backed providers. |
| `kilo mcp list` | No | text | No | MCP server status; local empty state exited 0. |
| `kilo profile --json` | Yes | JSON or styled error | No | Exits 1 when unauthenticated. |

## Wrapper Notes

- Prefer `kilo run --auto --format json <prompt>` for non-interactive task execution.
- Capture and classify stderr carefully: help and diagnostics can succeed while still printing INFO service/file/db lines to stderr.
- Do not assume the locally installed version is current. On 2026-07-03, npm latest was `7.4.1` while local install was `7.3.45`.
- Account for the npm launcher as an extra process layer; it forwards signals to the packaged platform binary.
- `--auto` does not mean "ignore policy"; docs say autonomous mode still respects auto-approval configuration, and unapproved operations are not allowed.
- Kilo 7.3.45 does not support run-time system-prompt delivery flags. Use config/agent mechanisms or wait for the sibling system-prompt topic if later versions add flags.
- `kilo models --verbose` is useful for model metadata but is not directly machine-readable as a single JSON document.
- `profile --json` exiting 1 when unauthenticated is an expected state.
- UI/mutating commands include default TUI, `web`, `console`, `plugin`, `import`, `pr`, `github install`, `upgrade`, and `uninstall`.
- Shell quoting matters for JSON-ish values such as `debug agent --params '{path:"README.md"}'`.
- Local runs initialized config/data/log/telemetry files even for help/debug commands; wrappers should use isolated `HOME`/XDG directories when probing.

## Changelog

- 2026-07-03: Updated upstream latest to npm `7.4.1` and recorded local installed `7.3.45`.
- 2026-07-03: Reworked frontmatter to satisfy `_schema.yaml` with explicit macOS/Linux/Windows records instead of `os: all`.
- 2026-07-03: Added local evidence for binary aliases, npm launcher signal/tree-sitter behavior, noisy help stderr, XDG config/data paths, DB/log side effects, and machine-introspection commands.
- 2026-07-03: Added negative probes showing installed Kilo rejects `--system-prompt`, `--append-system-prompt`, and `--replace-system-prompt`.
- 2026-07-03: Expanded install methods from README/docs, including curl, pnpm, Bun, Homebrew, Arch AUR, and release assets.

## Sources

- [Kilo homepage](https://kilo.ai/)
- [Kilo docs](https://kilo.ai/docs)
- [Kilo CLI overview](https://kilo.ai/docs/code-with-ai/platforms/cli)
- [Kilo CLI command reference](https://kilo.ai/docs/code-with-ai/platforms/cli-reference)
- [Kilo GitHub repository](https://github.com/Kilo-Org/kilocode)
- [Kilo MCP CLI docs](https://kilo.ai/docs/automate/mcp/using-in-cli)
- [Kilo plugin docs](https://kilo.ai/docs/automate/extending/plugins)
- [npm package: `@kilocode/cli`](https://www.npmjs.com/package/@kilocode/cli)
- Local command: `npm view @kilocode/cli version dist-tags bin repository homepage --json`
- Local command: `kilo --version`; `kilocode --version`
- Local command: `kilo --help`; `kilo <subcommand> --help`; `kilo help --all --format md`
- Local command: `kilo debug paths`; `kilo debug config`; `kilo debug info`; `kilo debug v2`; `kilo debug skill`
- Local command: `kilo config check`; `kilo auth list`; `kilo mcp list`; `kilo daemon status --json`; `kilo db path`; `kilo db 'select name from sqlite_master limit 3' --format json`
- Local files inspected: `~/.config/kilo/kilo.jsonc`, `~/.config/kilo/.gitignore`, `~/.local/share/kilo/`, `/Users/ken/.nvm/versions/node/v22.20.0/lib/node_modules/@kilocode/cli/package.json`, and `/Users/ken/.nvm/versions/node/v22.20.0/lib/node_modules/@kilocode/cli/bin/kilo`
