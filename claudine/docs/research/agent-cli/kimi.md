---
$schema: ./_schema.yaml
created: 2026-07-02
last_updated: 2026-07-03
agent: codex
model: default
latest_version: "0.22.2"
homepage: https://moonshotai.github.io/kimi-code/
repo: https://github.com/MoonshotAI/kimi-code
docs: https://moonshotai.github.io/kimi-code/en/
cli_docs: https://moonshotai.github.io/kimi-code/en/reference/kimi-command
binaries:
  - os: macos
    binary: kimi
    alt_binaries: ["kimi-code", "kimi-cli"]
    notes: "Current Kimi Code installer places a single `kimi` executable on PATH. Homebrew formula is named `kimi-code`. A legacy Python `kimi-cli` command may still be installed separately and exposes a different CLI surface."
  - os: linux
    binary: kimi
    alt_binaries: ["kimi-code", "kimi-cli"]
    notes: "Current Kimi Code installer places a single `kimi` executable on PATH. Homebrew-on-Linux formula is named `kimi-code`. A legacy Python `kimi-cli` command may still be installed separately and exposes a different CLI surface."
  - os: windows
    binary: kimi.exe
    alt_binaries: ["kimi.cmd", "kimi-code", "kimi-cli.exe", "kimi-cli.cmd"]
    notes: "Official PowerShell installer exposes `kimi`; archive/npm installs may create `.exe` or `.cmd` shims. Git for Windows is required because the CLI uses Git Bash as its shell environment."
install_methods:
  - os: macos
    method: standalone_binary
    command: "curl -fsSL https://code.kimi.com/kimi-code/install.sh | bash"
    notes: "Official recommended script; downloads the latest release, verifies checksum, and places `kimi` on PATH."
  - os: macos
    method: brew
    command: "brew install kimi-code"
    notes: "Official README documents Homebrew for macOS/Linux. Homebrew reported formula version 0.22.1 while npm/GitHub latest was 0.22.2 on 2026-07-03."
  - os: macos
    method: npm
    command: "npm install -g @moonshot-ai/kimi-code"
    notes: "Requires Node.js 22.19.0 or later. pnpm alternative is `pnpm add -g @moonshot-ai/kimi-code`."
  - os: linux
    method: standalone_binary
    command: "curl -fsSL https://code.kimi.com/kimi-code/install.sh | bash"
    notes: "Official recommended script; downloads the latest release, verifies checksum, and places `kimi` on PATH."
  - os: linux
    method: brew
    command: "brew install kimi-code"
    notes: "Official README documents Homebrew for macOS/Linux."
  - os: linux
    method: npm
    command: "npm install -g @moonshot-ai/kimi-code"
    notes: "Requires Node.js 22.19.0 or later. pnpm alternative is `pnpm add -g @moonshot-ai/kimi-code`."
  - os: windows
    method: standalone_binary
    command: "irm https://code.kimi.com/kimi-code/install.ps1 | iex"
    notes: "Official PowerShell installer. Install Git for Windows before first launch, or set `KIMI_SHELL_PATH` to the absolute path of `bash.exe` if Git Bash is custom-installed."
  - os: windows
    method: npm
    command: "npm install -g @moonshot-ai/kimi-code"
    notes: "Requires Node.js 22.19.0 or later; package-manager shims may expose `kimi.cmd`."
subcommands:
  - name: "(default TUI)"
    description: "Starts an interactive terminal UI session in the current working directory."
    non_interactive: false
    notes: "Requires a TTY. First launch normally requires `/login` in the UI unless credentials/config already exist."
  - name: "(prompt mode)"
    description: "Runs one prompt non-interactively and exits."
    non_interactive: true
    notes: "Activated with `-p` or `--prompt`; supports `--output-format text|stream-json`."
  - name: "login"
    description: "Runs Kimi Code OAuth device-code login without opening the TUI."
    non_interactive: false
    notes: "No flags. It prints a verification URL/code and polls until browser-side authorization completes; user interaction is still required."
  - name: "acp"
    description: "Runs the Agent Client Protocol server over stdio for IDEs."
    non_interactive: true
    notes: "JSON-RPC protocol server. `--login` runs device-code login then exits for ACP terminal-auth."
  - name: "server"
    description: "Runs, installs, and manages the local REST/WebSocket/web service."
    non_interactive: true
    notes: "`server run` starts or reuses a background loopback daemon by default; `--foreground` stays attached. Service lifecycle commands may write OS service definitions."
  - name: "web"
    description: "Starts the local server and opens the browser web UI."
    non_interactive: false
    notes: "Alias-like convenience for `kimi server run --open`; `--no-open` avoids browser launch, but the web UI is still user-facing."
  - name: "doctor"
    description: "Validates `config.toml` and `tui.toml`."
    non_interactive: true
    notes: "Current docs include `doctor config [path]` and `doctor tui [path]`; installed 0.14.0 validated defaults but rejected the nested forms."
  - name: "export"
    description: "Exports a session as a ZIP archive."
    non_interactive: true
    notes: "Use `--yes` when omitting a session id to skip previous-session confirmation."
  - name: "migrate"
    description: "Migrates local data from the legacy Python `kimi-cli` installation."
    non_interactive: false
    notes: "Documented as entirely interactive."
  - name: "upgrade"
    description: "Checks for the latest Kimi Code version and offers an update path."
    non_interactive: false
    notes: "`update` is documented as an alias. It displays update choices and may run package-manager commands."
  - name: "provider"
    description: "Manages configured providers and imports catalog providers."
    non_interactive: true
    notes: "Includes `add`, `remove`, `list`, `catalog list`, and `catalog add`; JSON output is available for list/catalog commands in current docs."
  - name: "vis"
    description: "Runs the session trace visualizer web server."
    non_interactive: false
    notes: "Starts an in-process server, prints a URL, opens a browser unless `--no-open` is passed, and runs until interrupted."
cli_switches:
  - flag: --version
    value: ""
    scope: ["global", "meta"]
    default: "false"
    description: "Prints the version number and exits."
    example: "kimi --version"
    notes: "Short alias: `-V`. Local installed `kimi` returned `0.14.0`; npm/GitHub latest was `0.22.2`."
  - flag: --help
    value: ""
    scope: ["global", "meta"]
    default: "false"
    description: "Shows help information and exits."
    example: "kimi --help"
    notes: "Short alias: `-h`. Local 0.14.0 help truncated or fell back to root help for several nested commands, so current docs were used for nested switch inventory."
  - flag: --session
    value: "[id]"
    scope: ["global", "session"]
    default: "none"
    description: "Resumes a session by id, or opens an interactive selector when no id is supplied."
    example: "kimi --session 01HZ...XYZ"
    notes: "Short alias: `-S`; hidden aliases: `--resume`, `-r`. Mutually exclusive with `--continue`."
  - flag: --continue
    value: ""
    scope: ["global", "session"]
    default: "false"
    description: "Continues the most recent session for the current working directory."
    example: "kimi --continue"
    notes: "Short alias in current docs is `-c`; local 0.14.0 help showed `-C`. Mutually exclusive with `--session`."
  - flag: --model
    value: "<model>"
    scope: ["global", "model_selection"]
    default: "default_model from config.toml"
    description: "Uses a model alias for this invocation."
    example: "kimi -m kimi-code/kimi-for-coding -p 'Explain the latest diff'"
    notes: "Short alias: `-m`."
  - flag: --prompt
    value: "<prompt>"
    scope: ["global", "automation"]
    default: "interactive TUI"
    description: "Runs one prompt non-interactively and streams assistant output to stdout."
    example: "kimi -p 'Summarize the current repository status'"
    notes: "Short alias: `-p`. Current docs say `--prompt` cannot be combined with `--yolo`, `--auto`, or `--plan`; installed 0.14.0 rejected those combinations."
  - flag: --output-format
    value: "<text|stream-json>"
    scope: ["global", "automation"]
    default: "text"
    description: "Selects non-interactive prompt output format."
    example: "kimi -p 'List changed files' --output-format stream-json"
    notes: "Only valid with `--prompt`; installed 0.14.0 rejected `--output-format stream-json` without `-p`."
  - flag: --yolo
    value: ""
    scope: ["global", "permissions"]
    default: "false"
    description: "Auto-approves regular tool calls, including file writes and shell commands."
    example: "kimi --yolo"
    notes: "Short alias: `-y`; hidden aliases: `--yes`, `--auto-approve`. Cannot be combined with `--auto`; current docs say prompt mode uses auto permission by default instead."
  - flag: --auto
    value: ""
    scope: ["global", "permissions"]
    default: "false"
    description: "Starts with auto permission mode so tool approvals are automatic and the agent does not ask the user questions."
    example: "kimi --auto"
    notes: "Cannot be combined with `--yolo`; installed 0.14.0 rejected `-p hi --auto`."
  - flag: --plan
    value: ""
    scope: ["global", "planning"]
    default: "false"
    description: "Starts in Plan mode, prioritizing read-only exploration and planning."
    example: "kimi --plan"
    notes: "Installed 0.14.0 rejected `-p hi --plan`. Plan-mode exit approval is not bypassed by `--yolo` in current docs."
  - flag: --skills-dir
    value: "<dir>"
    scope: ["global", "skills"]
    default: "auto-discovered user and project skills"
    description: "Loads Skills from the specified directory and replaces automatic discovery for this launch."
    example: "kimi --skills-dir /path/to/team-skills --skills-dir ./local-skills"
    notes: "Repeatable. Persistent additions use `extra_skill_dirs` in `config.toml`."
  - flag: --add-dir
    value: "<dir>"
    scope: ["global", "workspace"]
    default: "[]"
    description: "Adds an extra workspace directory for this session."
    example: "kimi --add-dir ../shared"
    notes: "Repeatable. Relative paths resolve against the current working directory."
  - flag: --login
    value: ""
    scope: ["acp", "auth"]
    default: "false"
    description: "Runs the device-code login flow from the ACP subcommand and exits."
    example: "kimi acp --login"
    notes: "Observed in local `kimi acp --help`; intended for ACP terminal-auth."
  - flag: --port
    value: "<port>"
    scope: ["server run", "server install", "web", "vis"]
    default: "58627 for server; available port for vis"
    description: "Sets the local server or visualizer port."
    example: "kimi server run --port 58627"
    notes: "For supervised install, records the chosen port in `$KIMI_CODE_HOME/server/install.json`."
  - flag: --log-level
    value: "<level>"
    scope: ["server run", "server install", "web"]
    default: "omitted"
    description: "Enables server logs at the selected level."
    example: "kimi server run --log-level debug"
    notes: "Documented for server commands."
  - flag: --debug-endpoints
    value: ""
    scope: ["server run", "web"]
    default: "false"
    description: "Mounts `/api/v1/debug/*` routes."
    example: "kimi server run --debug-endpoints"
    notes: "Use only for diagnostics."
  - flag: --foreground
    value: ""
    scope: ["server run", "web"]
    default: "false"
    description: "Runs the server attached to the current terminal instead of spawning/reusing a background daemon."
    example: "kimi server run --foreground"
    notes: "Foreground mode stays running until interrupted."
  - flag: --open
    value: ""
    scope: ["server run", "web"]
    default: "false for server run; true for web"
    description: "Opens the web UI in the default browser once the local server is healthy."
    example: "kimi server run --open"
    notes: "`kimi web` enables browser opening by default."
  - flag: --no-open
    value: ""
    scope: ["web", "vis"]
    default: "false"
    description: "Suppresses automatic browser launch."
    example: "kimi web --no-open"
    notes: "Useful for wrappers that want to present the URL themselves."
  - flag: --force
    value: ""
    scope: ["server install"]
    default: "false"
    description: "Replaces an existing OS service install instead of failing."
    example: "kimi server install --force"
    notes: "Service install writes launchd/systemd/schtasks definitions."
  - flag: --json
    value: ""
    scope: ["server install", "server status", "provider list", "provider catalog list"]
    default: "false"
    description: "Outputs JSON for automation where supported."
    example: "kimi provider list --json"
    notes: "Current docs list `server status --json`, but installed 0.14.0 rejected that flag. `provider list --json` and `provider catalog list --json` worked locally."
  - flag: --output
    value: "<path>"
    scope: ["export"]
    default: "session-derived ZIP path in current directory"
    description: "Sets the output ZIP path for a session export."
    example: "kimi export 01HZ...XYZ -o ./bug-report.zip"
    notes: "Short alias: `-o`."
  - flag: --yes
    value: ""
    scope: ["export"]
    default: "false"
    description: "Skips confirmation when exporting the default previous session."
    example: "kimi export -y"
    notes: "Short alias: `-y`."
  - flag: --no-include-global-log
    value: ""
    scope: ["export"]
    default: "false"
    description: "Excludes the global diagnostic log from the export."
    example: "kimi export 01HZ...XYZ -o ./bug-report.zip --no-include-global-log"
    notes: "By default, `~/.kimi-code/logs/kimi-code.log` is included."
  - flag: --host
    value: "<host>"
    scope: ["vis"]
    default: "127.0.0.1"
    description: "Sets the visualizer bind host."
    example: "kimi vis --host 0.0.0.0 --port 8123 --no-open"
    notes: "Current command reference documents this for `vis`; server run binds loopback only."
  - flag: --api-key
    value: "<key>"
    scope: ["provider add", "provider catalog add"]
    default: "KIMI_REGISTRY_API_KEY fallback"
    description: "Supplies the bearer token used to import providers from a registry/catalog."
    example: "kimi provider add https://registry.example.com/v1/models/api.json --api-key YOUR_KEY"
    notes: "Avoid logging the value. This is scoped to provider import, not ordinary model credential fallback."
  - flag: --filter
    value: "<substring>"
    scope: ["provider catalog list"]
    default: "none"
    description: "Filters catalog providers or models by case-insensitive substring."
    example: "kimi provider catalog list --filter anthropic"
    notes: "Useful for catalog browsing."
  - flag: --url
    value: "<url>"
    scope: ["provider catalog list", "provider catalog add"]
    default: "https://models.dev/api.json"
    description: "Overrides the provider catalog URL."
    example: "kimi provider catalog list --url https://models.dev/api.json --json"
    notes: "Also used when importing a known catalog provider."
  - flag: --default-model
    value: "<modelId>"
    scope: ["provider catalog add"]
    default: "none"
    description: "Sets `default_model` after importing a catalog provider."
    example: "kimi provider catalog add anthropic --api-key sk-ant-... --default-model claude-opus-4-7"
    notes: "Current docs document this under `provider catalog add`."
config_paths:
  - os: macos
    scope: user
    path: /Users/<name>/.kimi-code/config.toml
    format: toml
    notes: "Main runtime configuration. Can be relocated by `KIMI_CODE_HOME`; local host had `/Users/ken/.kimi-code/config.toml`."
  - os: linux
    scope: user
    path: /home/<name>/.kimi-code/config.toml
    format: toml
    notes: "Main runtime configuration. Can be relocated by `KIMI_CODE_HOME`."
  - os: windows
    scope: user
    path: 'C:\Users\<name>\.kimi-code\config.toml'
    format: toml
    notes: "Main runtime configuration. Can be relocated by `KIMI_CODE_HOME`."
  - os: macos
    scope: user
    path: /Users/<name>/.kimi-code/tui.toml
    format: toml
    notes: "Terminal UI preferences, including theme, editor, notifications, and auto-update settings."
  - os: linux
    scope: user
    path: /home/<name>/.kimi-code/tui.toml
    format: toml
    notes: "Terminal UI preferences, including theme, editor, notifications, and auto-update settings."
  - os: windows
    scope: user
    path: 'C:\Users\<name>\.kimi-code\tui.toml'
    format: toml
    notes: "Terminal UI preferences, including theme, editor, notifications, and auto-update settings."
  - os: macos
    scope: user
    path: /Users/<name>/.kimi-code/mcp.json
    format: json
    notes: "User-level MCP server declarations; project `.kimi-code/mcp.json` overrides entries with the same name."
  - os: linux
    scope: user
    path: /home/<name>/.kimi-code/mcp.json
    format: json
    notes: "User-level MCP server declarations; project `.kimi-code/mcp.json` overrides entries with the same name."
  - os: windows
    scope: user
    path: 'C:\Users\<name>\.kimi-code\mcp.json'
    format: json
    notes: "User-level MCP server declarations; project `.kimi-code/mcp.json` overrides entries with the same name."
  - os: macos
    scope: repo
    path: <repo>/.kimi-code/mcp.json
    format: json
    notes: "Project-local MCP declarations. Stdio entries can execute local commands when a session starts."
  - os: linux
    scope: repo
    path: <repo>/.kimi-code/mcp.json
    format: json
    notes: "Project-local MCP declarations. Stdio entries can execute local commands when a session starts."
  - os: windows
    scope: repo
    path: '<repo>\.kimi-code\mcp.json'
    format: json
    notes: "Project-local MCP declarations. Stdio entries can execute local commands when a session starts."
  - os: macos
    scope: user
    path: /Users/<name>/.kimi-code/AGENTS.md
    format: text
    notes: "Optional global Kimi-specific agent instructions; moves with `KIMI_CODE_HOME`."
  - os: linux
    scope: user
    path: /home/<name>/.kimi-code/AGENTS.md
    format: text
    notes: "Optional global Kimi-specific agent instructions; moves with `KIMI_CODE_HOME`."
  - os: windows
    scope: user
    path: 'C:\Users\<name>\.kimi-code\AGENTS.md'
    format: text
    notes: "Optional global Kimi-specific agent instructions; moves with `KIMI_CODE_HOME`."
  - os: macos
    scope: user
    path: /Users/<name>/.kimi-code/updates/latest.json
    format: json
    notes: "Auto-update metadata. Local file recorded latest `0.22.2` on 2026-07-03."
  - os: linux
    scope: user
    path: /home/<name>/.kimi-code/updates/latest.json
    format: json
    notes: "Auto-update metadata."
  - os: windows
    scope: user
    path: 'C:\Users\<name>\.kimi-code\updates\latest.json'
    format: json
    notes: "Auto-update metadata."
  - os: macos
    scope: system
    path: /Users/<name>/Library/LaunchAgents/ai.moonshot.kimi-server.plist
    format: other
    notes: "`kimi server install` writes this LaunchAgent plist on macOS."
  - os: linux
    scope: system
    path: /home/<name>/.config/systemd/user/kimi-server.service
    format: other
    notes: "`kimi server install` writes this user systemd unit on Linux."
  - os: windows
    scope: system
    path: KimiServer scheduled task
    format: other
    notes: "`kimi server install` registers a scheduled task named `KimiServer` via `schtasks /Create /XML`."
env_vars:
  - name: KIMI_CODE_HOME
    effect: "Relocates the entire Kimi Code data root; config, sessions, logs, OAuth credentials, updates, Kimi-specific skills, and AGENTS.md are read/written under this directory instead of `~/.kimi-code`."
  - name: KIMI_DISABLE_TELEMETRY
    effect: "Truthy values (`1`, `true`, `yes`, `y`) disable anonymous telemetry even when `telemetry = true` in config."
  - name: KIMI_CODE_BACKGROUND_KEEP_ALIVE_ON_EXIT
    effect: "Overrides `[background].keep_alive_on_exit`; in prompt mode, a true value makes the process wait for background tasks to finish before exit, bounded by `print_wait_ceiling_s`."
  - name: KIMI_CODE_PLUGIN_MARKETPLACE_URL
    effect: "Overrides the plugin marketplace JSON URL used by `/plugins`; accepts HTTP(S), file URLs, and local paths."
  - name: KIMI_CODE_AGENT_SWARM_MAX_CONCURRENCY
    effect: "Caps how many AgentSwarm subagents run concurrently during initial ramp; invalid non-positive values fail fast."
  - name: KIMI_CODE_EXPERIMENTAL_FLAG
    effect: "Truthy values enable all registered experimental features for the process."
  - name: KIMI_SHELL_PATH
    effect: "On Windows, overrides Git Bash auto-detection with an absolute path to `bash.exe`."
  - name: KIMI_CODE_NO_AUTO_UPDATE
    effect: "Truthy values fully disable update preflight: no check, background install, or prompt."
  - name: KIMI_CLI_NO_AUTO_UPDATE
    effect: "Legacy alias honored for `KIMI_CODE_NO_AUTO_UPDATE`."
  - name: KIMI_DISABLE_CRON
    effect: "Set to `1` to disable the scheduled-task tool; new CronCreate requests are rejected and existing tasks do not fire."
  - name: HOME
    effect: "Used to resolve the default `~/.kimi-code` data path."
  - name: VISUAL
    effect: "External editor command; takes precedence over `EDITOR`."
  - name: EDITOR
    effect: "Fallback external editor command when `VISUAL` is unset."
  - name: PATH
    effect: "Used to locate dependencies such as `rg`, `fd`/`fdfind`, `git`, and Windows Git Bash candidates."
  - name: NO_COLOR
    effect: "Disables color output according to the no-color convention."
  - name: FORCE_COLOR
    effect: "Forces color output where supported."
  - name: CI
    effect: "When non-empty and not `0`, disables theme detection and falls back to the dark theme."
  - name: HTTP_PROXY
    effect: "Standard proxy variable honored for HTTP outbound traffic, including model API calls, MCP servers, web tools, telemetry, sign-in, and update checks."
  - name: HTTPS_PROXY
    effect: "Standard proxy variable honored for HTTPS outbound traffic."
  - name: ALL_PROXY
    effect: "Fallback proxy used when scheme-specific proxy variables are unset; supports SOCKS schemes."
  - name: NO_PROXY
    effect: "Comma-separated hosts that bypass proxy handling; loopback hosts always bypass."
machine_introspection:
  - command: "kimi provider list --json"
    purpose: config_dump
    machine_readable: true
    output_format: json
    useful_for_codegen: true
    notes: "Prints raw configured `providers` and `models` tables. Worked locally on installed 0.14.0, returning empty objects under the worktree-scoped home."
  - command: "kimi provider catalog list --json"
    purpose: models
    machine_readable: true
    output_format: json
    useful_for_codegen: true
    notes: "Downloads and prints the public models.dev provider/model catalog as JSON. Worked locally on installed 0.14.0."
  - command: "kimi provider catalog list <providerId> --json"
    purpose: models
    machine_readable: true
    output_format: json
    useful_for_codegen: true
    notes: "Narrows the models.dev catalog to one provider."
  - command: "kimi doctor"
    purpose: doctor
    machine_readable: false
    output_format: text
    useful_for_codegen: false
    notes: "Validates default `config.toml` and `tui.toml`; exits 0 when files are valid or missing defaults are skipped."
  - command: "kimi doctor config [path]"
    purpose: doctor
    machine_readable: false
    output_format: text
    useful_for_codegen: false
    notes: "Current docs document this nested validator; installed 0.14.0 rejected the nested form, so wrappers should probe version/help before relying on it."
  - command: "kimi doctor tui [path]"
    purpose: doctor
    machine_readable: false
    output_format: text
    useful_for_codegen: false
    notes: "Current docs document this nested validator; installed 0.14.0 rejected the nested form."
  - command: "kimi server status --json"
    purpose: doctor
    machine_readable: true
    output_format: json
    useful_for_codegen: false
    notes: "Current docs document JSON status for installed/running/pid/port/log-path. Installed 0.14.0 rejected `--json`, so treat as version-gated."
  - command: "GET http://127.0.0.1:<port>/openapi.json"
    purpose: capabilities
    machine_readable: true
    output_format: json
    useful_for_codegen: true
    notes: "Available when `kimi server run` has started the local server; returns the REST OpenAPI document."
  - command: "GET http://127.0.0.1:<port>/asyncapi.json"
    purpose: capabilities
    machine_readable: true
    output_format: json
    useful_for_codegen: true
    notes: "Available when `kimi server run` has started the local server; returns the local WebSocket AsyncAPI document."
  - command: "kimi acp"
    purpose: capabilities
    machine_readable: true
    output_format: json
    useful_for_codegen: true
    notes: "Long-running JSON-RPC server over stdin/stdout. `initialize` returns agent info, auth methods, and capability matrix; logs go to stderr and diagnostic files."
wrapper_notes:
  - "Do not conflate current Kimi Code CLI (`@moonshot-ai/kimi-code`, `kimi --version` style `0.x`) with the legacy Python `kimi-cli` (`kimi-cli --version` style `1.x`). This host has both: `kimi` at `/Users/ken/.kimi-code/bin/kimi` returned `0.14.0`, while `/Users/ken/.local/bin/kimi-cli` returned `1.47.0`."
  - "Current upstream latest was verified as `0.22.2` from npm and GitHub releases on 2026-07-03. The installed `kimi` was older (`0.14.0`) and its help/flags lagged current docs."
  - "For non-interactive wrapper execution, use `kimi -p <prompt>` and optionally `--output-format stream-json`; prompt mode does not open the TUI and uses auto permission behavior by default."
  - "In prompt mode, assistant text goes to stdout, while thinking, tool progress, and resuming notices go to stderr. In `stream-json`, thinking content is not written to JSONL and tool progress still goes to stderr."
  - "Installed 0.14.0 rejects `-p` combined with `--yolo`, `--auto`, or `--plan`; current docs document the same conflict and say prompt mode uses auto permission by default."
  - "The default data root is `~/.kimi-code`, but `KIMI_CODE_HOME` relocates all config, credentials, sessions, logs, updates, Kimi-specific skills, and AGENTS.md. In this worktree, `kimi doctor` without explicit `KIMI_CODE_HOME` checked `/Users/ken/.claudine/.kimi-code`, so wrappers should set `KIMI_CODE_HOME` deliberately when isolation matters."
  - "Config files can contain plaintext provider API keys. Wrappers and diagnostics must redact `config.toml`, provider imports, and command lines containing `--api-key`."
  - "The CLI writes side-effect state on normal use: `config.toml`, `tui.toml`, `credentials/`, `oauth/`, `session_index.jsonl`, `sessions/`, `logs/`, `updates/`, and `user-history/` under `KIMI_CODE_HOME`."
  - "OAuth login (`kimi login`, `kimi acp --login`, and TUI `/login`) is not unattended despite being outside the TUI; it requires a browser/device-code authorization flow."
  - "`kimi acp` is the clean protocol entry point for IDE-style integration. It keeps JSON-RPC on stdout/stdin and writes logs to stderr/diagnostic files."
  - "The local server binds loopback in documented server mode and exposes machine-readable OpenAPI/AsyncAPI documents only after startup. `web` and `vis` open browsers by default unless suppressed."
  - "`kimi server install` writes OS-managed service definitions: launchd on macOS, user systemd on Linux, and a Windows scheduled task. Avoid running it from wrappers unless the user explicitly requested service installation."
  - "Windows shell execution depends on Git Bash. Require Git for Windows or set `KIMI_SHELL_PATH` to `bash.exe`."
  - "The root help output from installed 0.14.0 truncated at `--auto` in this non-interactive terminal, and several nested `--help` calls fell back to root help. Current docs were used for nested switch inventory where local help was incomplete."
changes:
  - "Retargeted the research from the legacy Python `MoonshotAI/kimi-cli` package to the current `MoonshotAI/kimi-code` Kimi Code CLI successor."
  - "Updated latest upstream version from legacy `1.48.0` to current Kimi Code `0.22.2`, verified from npm and GitHub releases; recorded local installed `kimi` as `0.14.0` and legacy `kimi-cli` as `1.47.0`."
  - "Replaced uv/Python install details with standalone install scripts, Homebrew `kimi-code`, and npm package `@moonshot-ai/kimi-code`."
  - "Replaced legacy `--print`/`--quiet` automation with current `-p`/`--prompt` prompt mode and `--output-format text|stream-json`."
  - "Replaced legacy `~/.kimi` configuration discovery with current `~/.kimi-code` / `KIMI_CODE_HOME` data layout."
  - "Added current machine-readable provider catalog/config commands and local server OpenAPI/AsyncAPI introspection."
  - "Recorded version-gated gaps where installed 0.14.0 rejects current documented nested doctor/server JSON forms."
requires_claudine_update: true
reason: "Claudine's Kimi provider metadata and wrapper need to distinguish current Kimi Code (`kimi`, @moonshot-ai/kimi-code, ~/.kimi-code, -p prompt mode) from the legacy Python `kimi-cli` surface, update non-interactive flags/output handling, and add current config/introspection paths."
---

# Kimi Code CLI

## Overview

Kimi Code CLI is Moonshot AI's terminal AI coding agent. It can read and edit code, run shell commands, search files, fetch web pages, use MCP tools, and run either as an interactive TUI or as protocol/server entry points. The primary command a user types is `kimi`.

The current upstream version I verified is `0.22.2`. I verified it from `npm view @moonshot-ai/kimi-code version --json`, the GitHub releases page for `MoonshotAI/kimi-code`, and the local update cache at `~/.kimi-code/updates/latest.json`. The installed primary binary on this host is older: `kimi --version` returned `0.14.0`. A separate legacy Python command is also present: `kimi-cli --version` returned `kimi, version 1.47.0`.

Primary URLs:

- Homepage: [Kimi Code CLI](https://moonshotai.github.io/kimi-code/)
- Repository: [MoonshotAI/kimi-code](https://github.com/MoonshotAI/kimi-code)
- General docs: [Kimi Code CLI Docs](https://moonshotai.github.io/kimi-code/en/)
- CLI reference: [`kimi` Command](https://moonshotai.github.io/kimi-code/en/reference/kimi-command)

The user-provided legacy site and repository, [moonshotai.github.io/kimi-cli](https://moonshotai.github.io/kimi-cli/) and [MoonshotAI/kimi-cli](https://github.com/MoonshotAI/kimi-cli), still describe the Python `kimi-cli` line. Current official docs and the repository README point new users to the `kimi-code` successor, and this host's `kimi` command is the new single-binary Kimi Code CLI.

## Installation and Binaries

The current official command is `kimi` on macOS, Linux, and Windows. The current project/package is named `kimi-code` for Homebrew and `@moonshot-ai/kimi-code` for npm, but those package names are not the primary runtime command. On Windows, installers or package managers may expose `.exe` or `.cmd` shims such as `kimi.exe` or `kimi.cmd`.

Official install commands:

```sh
# macOS / Linux recommended installer
curl -fsSL https://code.kimi.com/kimi-code/install.sh | bash

# macOS / Linux Homebrew
brew install kimi-code

# npm, all platforms with Node.js 22.19.0+
npm install -g @moonshot-ai/kimi-code

# pnpm alternative
pnpm add -g @moonshot-ai/kimi-code
```

```powershell
# Windows PowerShell recommended installer
irm https://code.kimi.com/kimi-code/install.ps1 | iex
```

Windows has an extra runtime prerequisite: install Git for Windows before first launch because Kimi Code CLI uses the bundled Git Bash as its shell environment. If Git Bash is installed somewhere non-standard, set `KIMI_SHELL_PATH` to the absolute path of `bash.exe`.

Local observations:

- `command -v kimi` resolved to `/Users/ken/.kimi-code/bin/kimi`, a Mach-O arm64 executable.
- `kimi --version` returned `0.14.0`.
- `command -v kimi-cli` resolved to `/Users/ken/.local/bin/kimi-cli`, a Python/uv console script importing `kimi_cli.__main__`.
- `kimi-cli --version` returned `kimi, version 1.47.0`.
- Local `~/.kimi-code/updates/latest.json` recorded `"latest": "0.22.2"`.

Wrapper detection should prefer `kimi` for Kimi Code. It may still be useful to detect `kimi-cli` as a legacy collision and warn rather than silently applying the wrong flag surface.

## Subcommands

The default command, `kimi`, starts the interactive TUI in the current working directory. This requires a terminal and is not the automation entry point.

Automation and protocol entry points:

| Command or mode | Description | Non-interactive suitability |
| --- | --- | --- |
| `kimi -p <prompt>` / `kimi --prompt <prompt>` | Runs one prompt, streams output, and exits without opening the TUI. | Primary one-shot wrapper path. |
| `kimi acp` | Runs an Agent Client Protocol JSON-RPC server over stdin/stdout. | Suitable for IDE/protocol wrappers. |
| `kimi server run` | Starts or reuses a local REST/WebSocket/web daemon and returns when healthy unless `--foreground` is used. | Suitable for local API/web integration when a resident service is intended. |
| `kimi doctor` | Validates config files. | Suitable for diagnostics. |
| `kimi provider list --json` | Dumps configured providers/models. | Suitable for metadata inspection. |
| `kimi provider catalog list --json` | Dumps the public models.dev provider/model catalog. | Suitable for catalog/codegen. |

Interactive or user-mediated commands:

| Command | Description | Interaction caveat |
| --- | --- | --- |
| `kimi login` | Device-code OAuth login outside the TUI. | Requires browser/device authorization and polls until completion. |
| `kimi acp --login` | ACP terminal-auth login flow. | Requires browser/device authorization. |
| `kimi web` | Starts local server and opens the web UI. | Browser-facing; use `--no-open` if a wrapper presents the URL. |
| `kimi vis` | Starts the session visualizer. | Opens a browser by default and runs until interrupted. |
| `kimi export` | Exports a session as ZIP. | Use `--yes` when omitting session id to avoid confirmation. |
| `kimi migrate` | Migrates data from legacy `kimi-cli`. | Documented as entirely interactive. |
| `kimi upgrade` / `kimi update` | Checks for updates and offers install choices. | Presents choices and may run package-manager commands. |
| `kimi server install` | Installs an OS-managed server service. | Writes launchd/systemd/schtasks service definitions. |

Current docs list these top-level subcommands: `login`, `acp`, `server`, `web`, `doctor`, `export`, `migrate`, `upgrade`/`update`, `provider`, and `vis`.

## CLI Switch Inventory

Global/root switches observed in local help or documented in current official docs:

| Flag | Type | Default | Scope | Example | Notes |
| --- | --- | --- | --- | --- | --- |
| `--version`, `-V` | boolean | `false` | global | `kimi --version` | Local `kimi` returned `0.14.0`; upstream latest was `0.22.2`. |
| `--help`, `-h` | boolean | `false` | global | `kimi --help` | Local 0.14.0 help truncated in this non-interactive terminal. |
| `--session [id]`, `-S` | optional value | none | global/session | `kimi --session 01HZ...XYZ` | Hidden aliases: `--resume`, `-r`; no id opens interactive selector. |
| `--continue`, `-c` | boolean | `false` | global/session | `kimi --continue` | Local 0.14.0 help showed `-C`; current docs show `-c`. |
| `--model <model>`, `-m` | value | `default_model` | global/model | `kimi -m kimi-code/kimi-for-coding -p "Explain the diff"` | Uses model aliases from config. |
| `--prompt <prompt>`, `-p` | value | TUI mode | global/automation | `kimi -p "Summarize the repo"` | Primary one-shot path. |
| `--output-format <format>` | value enum | `text` | prompt mode | `kimi -p "List files" --output-format stream-json` | `text` or `stream-json`; only valid with `--prompt`. |
| `--yolo`, `-y` | boolean | `false` | global/permissions | `kimi --yolo` | Hidden aliases: `--yes`, `--auto-approve`; cannot combine with `--auto` or prompt mode. |
| `--auto` | boolean | `false` | global/permissions | `kimi --auto` | Auto permission mode; cannot combine with `--yolo` or prompt mode. |
| `--plan` | boolean | `false` | global/planning | `kimi --plan` | Cannot combine with prompt mode. |
| `--skills-dir <dir>` | repeatable value | automatic discovery | global/skills | `kimi --skills-dir /team/skills --skills-dir ./skills` | Replaces automatic skill discovery for this launch. |
| `--add-dir <dir>` | repeatable value | none | global/workspace | `kimi --add-dir ../shared` | Adds workspace directories. |

Prompt-mode behavior is wrapper-critical. `kimi -p` writes assistant text to stdout. Thinking, tool progress, and "resuming session" notices go to stderr. With `--output-format stream-json`, stdout is JSONL; thinking content is not written to JSONL, and progress still goes to stderr. Installed 0.14.0 rejected these documented invalid combinations:

```sh
kimi -p hi --yolo
kimi -p hi --auto
kimi -p hi --plan
kimi --output-format stream-json
```

`kimi acp`:

| Flag | Type | Default | Example | Notes |
| --- | --- | --- | --- | --- |
| `--login` | boolean | `false` | `kimi acp --login` | Runs device-code login then exits. |
| `--help`, `-h` | boolean | `false` | `kimi acp --help` | Local help worked. |

`kimi server run` and `kimi web`:

| Flag | Type | Default | Example | Notes |
| --- | --- | --- | --- | --- |
| `--port <port>` | value | `58627` | `kimi server run --port 58627` | Server loopback port. |
| `--log-level <level>` | value | omitted | `kimi server run --log-level debug` | Enables server logs. |
| `--debug-endpoints` | boolean | `false` | `kimi server run --debug-endpoints` | Mounts debug routes. |
| `--foreground` | boolean | `false` | `kimi server run --foreground` | Keeps server attached. |
| `--open` | boolean | `false` for `server run`, enabled by `web` | `kimi server run --open` | Opens browser. |
| `--no-open` | boolean | `false` | `kimi web --no-open` | Suppresses browser launch. |

`kimi server install`:

| Flag | Type | Default | Example | Notes |
| --- | --- | --- | --- | --- |
| `--port <port>` | value | `58627` | `kimi server install --port 58627` | Stored in server install metadata. |
| `--log-level <level>` | value | omitted | `kimi server install --log-level info` | Stored in generated service. |
| `--force` | boolean | `false` | `kimi server install --force` | Replaces an existing install. |
| `--json` | boolean | `false` | `kimi server install --json` | JSON output according to docs. |

`kimi server status`:

| Flag | Type | Default | Example | Notes |
| --- | --- | --- | --- | --- |
| `--json` | boolean | `false` | `kimi server status --json` | Current docs document this, but installed 0.14.0 rejected it with `unknown option '--json'`. |

`kimi doctor`:

| Command | Description | Notes |
| --- | --- | --- |
| `kimi doctor` | Validates default `config.toml` and `tui.toml`. | Installed 0.14.0 worked and returned 0 for valid/skipped files. |
| `kimi doctor config [path]` | Validates only runtime config. | Current docs document it; installed 0.14.0 rejected the nested form. |
| `kimi doctor tui [path]` | Validates only TUI config. | Current docs document it; installed 0.14.0 rejected the nested form. |

`kimi export`:

| Flag | Type | Default | Example | Notes |
| --- | --- | --- | --- | --- |
| `--output <path>`, `-o` | value | derived ZIP name | `kimi export 01HZ...XYZ -o ./bug-report.zip` | Output path. |
| `--yes`, `-y` | boolean | `false` | `kimi export -y` | Skips confirmation for default previous session. |
| `--no-include-global-log` | boolean | `false` | `kimi export 01HZ...XYZ --no-include-global-log` | Excludes global diagnostic log. |

`kimi vis`:

| Flag | Type | Default | Example | Notes |
| --- | --- | --- | --- | --- |
| `--port <number>` | value | available port | `kimi vis --port 8123` | Visualizer server port. |
| `--host <host>` | value | `127.0.0.1` | `kimi vis --host 0.0.0.0 --no-open` | Visualizer bind host. |
| `--no-open` | boolean | `false` | `kimi vis --no-open` | Suppresses browser launch. |

`kimi provider`:

| Command or flag | Type | Default | Example | Notes |
| --- | --- | --- | --- | --- |
| `provider add <url>` | command | none | `kimi provider add https://registry.example.com/v1/models/api.json --api-key YOUR_KEY` | Imports every provider in a custom registry. |
| `--api-key <key>` | value | `KIMI_REGISTRY_API_KEY` fallback | `kimi provider add <url> --api-key YOUR_KEY` | Redact in logs. |
| `provider remove <providerId>` | command | none | `kimi provider remove kohub` | Removes provider and referenced model aliases. |
| `provider list --json` | boolean flag | text | `kimi provider list --json` | Worked locally and returned `{"providers":{},"models":{}}` under the worktree-scoped home. |
| `provider catalog list [providerId]` | command | all providers | `kimi provider catalog list anthropic` | Reads models.dev catalog. |
| `--filter <substring>` | value | none | `kimi provider catalog list --filter anthropic` | Catalog filter. |
| `--url <url>` | value | `https://models.dev/api.json` | `kimi provider catalog list --url https://models.dev/api.json --json` | Catalog override. |
| `--json` | boolean | text | `kimi provider catalog list --json` | Worked locally and emitted JSON. |
| `provider catalog add <providerId>` | command | none | `kimi provider catalog add anthropic --api-key sk-ant-...` | Imports provider from catalog. |
| `--default-model <modelId>` | value | none | `kimi provider catalog add anthropic --api-key sk-ant-... --default-model claude-opus-4-7` | Optionally sets `default_model`. |

System-prompt delivery flags: no current Kimi Code root CLI flag equivalent to `--append-system-prompt` or `--replace-system-prompt` was observed in `kimi --help` or the current `kimi` command reference. Kimi has instruction/config surfaces such as `AGENTS.md`, Skills, agents/subagents, and agent files; delivery semantics belong to the sibling `system-prompt` topic, not this CLI surface document.

When help output and official docs disagree, I trusted local installed output for installed-version behavior and current official docs for current upstream behavior. The installed binary is `0.14.0` while upstream latest is `0.22.2`; several documented current commands are version-gated relative to this host.

## Configuration Discovery

Kimi Code stores all runtime data under `~/.kimi-code/` by default. The per-OS defaults are:

| OS | Default data root |
| --- | --- |
| macOS | `/Users/<name>/.kimi-code` |
| Linux | `/home/<name>/.kimi-code` |
| Windows | `C:\Users\<name>\.kimi-code` |

Set `KIMI_CODE_HOME` to move the data root. Once set, config, sessions, logs, OAuth credentials, Kimi-specific user Skills, global Kimi-specific `AGENTS.md`, and update state all land under that directory.

Important files and directories:

| Path under `KIMI_CODE_HOME` | Format | Purpose |
| --- | --- | --- |
| `config.toml` | TOML | Main runtime config: providers, models, permissions, hooks, loop control, background behavior, services, default model/mode. |
| `tui.toml` | TOML | TUI/client preferences: theme, editor, notifications, auto-update settings. |
| `AGENTS.md` | Markdown/text | Optional global Kimi-specific agent instructions. |
| `mcp.json` | JSON | User-level MCP server declarations. |
| `skills/` | directory | Kimi-specific user-level Skills. |
| `plugins/installed.json` | JSON | Installed plugin records and enabled state. |
| `credentials/` | JSON files | OAuth credentials; docs state directory `0700` and files `0600`. |
| `session_index.jsonl` | JSONL | Session index. |
| `sessions/<workDirKey>/<sessionId>/` | mixed | Session state, wire logs, plans, background task state, cron state, and session logs. |
| `logs/kimi-code.log` | text log | Global diagnostic log. |
| `updates/latest.json`, `updates/install.json` | JSON | Auto-update state and install metadata. |
| `user-history/<md5(workDir)>.jsonl` | JSONL | Per-working-directory input history. |

Project-local MCP config is discovered at `.kimi-code/mcp.json` in the working directory. Entries with the same server name override user-level entries. Stdio MCP entries can execute local commands when a session starts, so wrappers should treat project-local MCP as trust-sensitive.

The current docs say the CLI reads a single user-level `config.toml` and has no project-level config file mechanism for ordinary runtime config. Use different `KIMI_CODE_HOME` values to isolate config per project.

Local side effects observed under `/Users/ken/.kimi-code/` included:

- `config.toml`
- `tui.toml`
- `credentials/kimi-code.json`
- `oauth/kimi-code`
- `session_index.jsonl`
- `sessions/...`
- `logs/kimi-code.log`
- `updates/install.json`
- `updates/latest.json`
- `user-history/*.jsonl`

One wrapper-impacting local observation: running `kimi doctor` from this Claudine worktree without setting `KIMI_CODE_HOME` checked `/Users/ken/.claudine/.kimi-code/config.toml` and `/Users/ken/.claudine/.kimi-code/tui.toml`, while `KIMI_CODE_HOME=/Users/ken/.kimi-code kimi doctor` checked the expected user data root. Claudine wrappers should set `KIMI_CODE_HOME` deliberately for isolation and predictability.

## Environment Variables

General CLI/runtime variables:

| Variable | Effect |
| --- | --- |
| `KIMI_CODE_HOME` | Relocates the whole data root from `~/.kimi-code`. |
| `KIMI_DISABLE_TELEMETRY` | Truthy values disable anonymous telemetry. |
| `KIMI_CODE_BACKGROUND_KEEP_ALIVE_ON_EXIT` | Overrides background-task shutdown behavior and can make prompt mode wait for background tasks. |
| `KIMI_CODE_PLUGIN_MARKETPLACE_URL` | Overrides the plugin marketplace JSON loaded by `/plugins`. |
| `KIMI_CODE_AGENT_SWARM_MAX_CONCURRENCY` | Caps initial AgentSwarm subagent concurrency. |
| `KIMI_CODE_EXPERIMENTAL_FLAG` | Enables all registered experimental features for the process. |
| `KIMI_SHELL_PATH` | Windows-only Git Bash path override. |
| `KIMI_CODE_NO_AUTO_UPDATE` | Disables update checks, background installs, and prompts. |
| `KIMI_CLI_NO_AUTO_UPDATE` | Legacy alias for disabling auto-update. |
| `KIMI_DISABLE_CRON` | Disables scheduled-task creation and firing. |

Standard environment variables Kimi Code uses for general runtime behavior:

| Variable | Effect |
| --- | --- |
| `HOME` | Resolves the default data path. |
| `VISUAL`, `EDITOR` | Select external editor command; `VISUAL` wins. |
| `PATH` | Locates `rg`, `fd`/`fdfind`, `git`, and Git Bash candidates. |
| `NO_COLOR`, `FORCE_COLOR` | Control color output. |
| `CI` | Disables theme detection and falls back to dark theme when set and not `0`. |
| `TERM_PROGRAM`, `TERM`, `TMUX` | Detect terminal capabilities and notification support. |
| `DISPLAY`, `WAYLAND_DISPLAY`, `XDG_SESSION_TYPE` | Detect Linux graphical sessions for clipboard/image features. |
| `WSL_DISTRO_NAME`, `WSLENV` | Detect WSL for clipboard bridging. |
| `LOCALAPPDATA` | Windows fallback location while probing Git Bash. |
| `HTTP_PROXY`, `HTTPS_PROXY`, `ALL_PROXY`, `NO_PROXY` and lowercase variants | Configure outbound proxy behavior for Kimi Code traffic. Loopback hosts bypass the proxy. |

Model endpoint and provider credential variables are intentionally not duplicated here. Current docs emphasize that ordinary provider credentials such as `KIMI_API_KEY`, `ANTHROPIC_API_KEY`, and `OPENAI_API_KEY` are not read automatically from the shell; they must be written in `config.toml` or the `[providers.<name>.env]` sub-table. The explicit `KIMI_MODEL_*` family belongs to model configuration, not this general CLI surface, except that wrappers should know it is an in-memory temporary model channel and does not persist.

## Machine Introspection

| Command | Machine-readable | Format | Wrapper/codegen use |
| --- | --- | --- | --- |
| `kimi provider list --json` | yes | JSON | Dumps configured `providers` and `models` tables. Useful for reporting/provider metadata. |
| `kimi provider catalog list --json` | yes | JSON | Dumps the models.dev provider/model catalog. Useful for model/provider code generation. |
| `kimi provider catalog list <providerId> --json` | yes | JSON | Narrows catalog introspection to one provider. |
| `kimi doctor` | no | text | Validates active default config files. Useful for diagnostics, not codegen. |
| `kimi doctor config [path]` | no | text | Current-doc validator for config candidates; installed 0.14.0 rejected the nested form. |
| `kimi doctor tui [path]` | no | text | Current-doc validator for TUI config candidates; installed 0.14.0 rejected the nested form. |
| `kimi server status --json` | yes, current docs | JSON | Should report installed/running/pid/port/log path, but installed 0.14.0 rejected `--json`. |
| `GET /openapi.json` from the local server | yes | JSON/OpenAPI | REST API schema for code generation once `kimi server run` is healthy. |
| `GET /asyncapi.json` from the local server | yes | JSON/AsyncAPI | WebSocket API schema for code generation once `kimi server run` is healthy. |
| `kimi acp` + JSON-RPC `initialize` | yes | JSON-RPC | Returns agent info, auth methods, and capability matrix for ACP clients. |

Negative probes are useful evidence:

- Installed 0.14.0 accepted `kimi provider list --json` and emitted JSON.
- Installed 0.14.0 accepted `kimi provider catalog list --json` and emitted JSON from models.dev.
- Installed 0.14.0 rejected `kimi server status --json` with `unknown option '--json'`.
- Installed 0.14.0 rejected `kimi doctor config` and `kimi doctor tui` even though current docs document them.

Generic `--help` and `--version` are not listed as machine introspection in frontmatter because they do not expose structured state.

## Wrapper Notes

Use `kimi -p <prompt>` for one-shot execution. Add `--output-format stream-json` when Claudine needs machine-readable streaming. In text output, plan for stderr noise during successful runs because thinking, tool progress, and resume notices go to stderr.

Prompt mode uses auto permission behavior by default. Do not add `--yolo`, `--auto`, or `--plan` to prompt-mode launches; installed 0.14.0 rejects those combinations, and current docs document them as conflicts.

Set `KIMI_CODE_HOME` explicitly when a wrapper needs a known config/state root. This prevents accidental reads/writes to a user or worktree-specific data directory and makes session/log cleanup predictable.

Redact config and command lines. `config.toml` can contain plaintext provider API keys, OAuth references, custom headers, and provider import credentials. `kimi provider add --api-key ...` and `provider catalog add --api-key ...` should never be logged verbatim.

Avoid unattended login and upgrade flows. `kimi login`, `kimi acp --login`, TUI `/login`, `kimi migrate`, and `kimi upgrade` require user interaction or can invoke package-manager actions.

Avoid `kimi server install` unless the user explicitly requested a persistent service. It writes OS service definitions: launchd on macOS, user systemd on Linux, and a scheduled task on Windows.

For web/visualizer use, pass `--no-open` if the wrapper should not open a browser. Treat local web services as user-facing stateful processes, not normal one-shot commands.

On Windows, verify Git for Windows or `KIMI_SHELL_PATH` before relying on shell tool execution.

Keep legacy detection explicit. A `kimi-cli` binary on PATH is likely the Python legacy project and does not support the same flags/config paths as current Kimi Code. On this host, both are installed.

No current Kimi Code system-prompt CLI switch was found. System-prompt behavior should be handled by the sibling system-prompt research topic, especially for `AGENTS.md`, Skills, agent/subagent definitions, and any future prompt override flags.

## Changelog

- 2026-07-03: Retargeted the document from legacy Python `MoonshotAI/kimi-cli` to current `MoonshotAI/kimi-code`, because the primary installed `kimi` binary and current official docs now represent the successor CLI.
- 2026-07-03: Updated upstream latest version to `0.22.2`, verified from npm, GitHub releases, and local update metadata; recorded local installed `kimi` as `0.14.0` and legacy `kimi-cli` as `1.47.0`.
- 2026-07-03: Replaced uv/Python installation details with standalone install scripts, Homebrew `kimi-code`, and npm `@moonshot-ai/kimi-code`.
- 2026-07-03: Replaced legacy `--print`/`--quiet` automation with current `-p`/`--prompt` prompt mode and `--output-format text|stream-json`.
- 2026-07-03: Replaced legacy `~/.kimi` config discovery with `~/.kimi-code` / `KIMI_CODE_HOME`, including config, credentials, sessions, logs, updates, and user-history side effects.
- 2026-07-03: Added current provider catalog/list JSON introspection, local server OpenAPI/AsyncAPI introspection, and version-gated notes for `doctor` and `server status --json`.

## Sources

- [Kimi Code CLI homepage](https://moonshotai.github.io/kimi-code/)
- [MoonshotAI/kimi-code repository](https://github.com/MoonshotAI/kimi-code)
- [Kimi Code CLI docs](https://moonshotai.github.io/kimi-code/en/)
- [`kimi` command reference](https://moonshotai.github.io/kimi-code/en/reference/kimi-command)
- [Getting started](https://moonshotai.github.io/kimi-code/en/guides/getting-started)
- [Configuration files](https://moonshotai.github.io/kimi-code/en/configuration/config-files.md)
- [Config overrides](https://moonshotai.github.io/kimi-code/en/configuration/overrides.md)
- [Environment variables](https://moonshotai.github.io/kimi-code/en/configuration/env-vars.md)
- [Data locations](https://moonshotai.github.io/kimi-code/en/configuration/data-locations.md)
- [Model Context Protocol](https://moonshotai.github.io/kimi-code/en/customization/mcp.md)
- [`kimi acp` subcommand](https://moonshotai.github.io/kimi-code/en/reference/kimi-acp.md)
- [MoonshotAI/kimi-code releases](https://github.com/MoonshotAI/kimi-code/releases)
- [Legacy MoonshotAI/kimi-cli repository](https://github.com/MoonshotAI/kimi-cli)
- [Legacy Kimi CLI command reference](https://moonshotai.github.io/kimi-cli/en/reference/kimi-command.html)
- Local command: `kimi --version` -> `0.14.0`
- Local command: `kimi --help`
- Local command: `kimi acp --help`
- Local command: `kimi export --help`
- Local command: `kimi doctor`
- Local command: `kimi provider list --json`
- Local command: `kimi provider catalog list --json`
- Local command: `kimi server status --json` -> rejected by installed 0.14.0
- Local command: `kimi-cli --version` -> `kimi, version 1.47.0`
- Local inspection: `/Users/ken/.kimi-code/` file layout and update metadata
- Local command: `npm view @moonshot-ai/kimi-code version --json` -> `"0.22.2"`
- Local command: `brew info kimi-code --json=v2` -> Homebrew formula version `0.22.1` on 2026-07-03
