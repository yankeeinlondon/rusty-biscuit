---
$schema: ./_schema.yaml
created: 2026-07-02
last_updated: 2026-07-02
agent: codex
model: default
latest_version: "1.48.0"
homepage: https://www.kimi.com/code/
repo: https://github.com/MoonshotAI/kimi-cli
docs: https://moonshotai.github.io/kimi-cli/en/
cli_docs: https://moonshotai.github.io/kimi-cli/en/reference/kimi-command.html
binaries:
  - os: all
    binary: kimi
    alt_binaries: ["kimi-cli"]
    notes: "PyPI console scripts expose both `kimi` and `kimi-cli`, both mapped to the same entrypoint. Official docs use `kimi`."
  - os: windows
    binary: kimi.exe
    alt_binaries: ["kimi-cli.exe", "kimi.cmd", "kimi-cli.cmd"]
    notes: "Windows Python/uv installs commonly expose executable launchers or command shims for console scripts; official examples still invoke `kimi` from PowerShell."
install_methods:
  - os: macos
    method: other
    command: "curl -LsSf https://code.kimi.com/install.sh | bash"
    notes: "Official installer installs uv if needed, then runs `uv tool install --python 3.13 kimi-cli`."
  - os: linux
    method: other
    command: "curl -LsSf https://code.kimi.com/install.sh | bash"
    notes: "Official installer installs uv if needed, then runs `uv tool install --python 3.13 kimi-cli`."
  - os: windows
    method: other
    command: "Invoke-RestMethod https://code.kimi.com/install.ps1 | Invoke-Expression"
    notes: "Official PowerShell installer installs uv if needed, then runs `uv tool install --python 3.13 kimi-cli`."
  - os: all
    method: other
    command: "uv tool install --python 3.13 kimi-cli"
    notes: "Documented alternative when uv is already installed. Kimi Code CLI supports Python 3.12-3.14, with 3.13 recommended; `kimi term` requires Python 3.14+."
subcommands:
  - name: "(default shell)"
    description: "Starts the interactive terminal agent session when no subcommand or non-interactive mode flag is supplied."
    non_interactive: false
    notes: "Accepts an optional `--prompt` prefill but otherwise stays interactive."
  - name: "(print mode)"
    description: "Runs a prompt or stdin task non-interactively and exits."
    non_interactive: true
    notes: "Activated by `--print` or `--quiet`; print mode implicitly enables AFK behavior."
  - name: "(wire mode)"
    description: "Runs a JSON-RPC 2.0 Wire server over stdin/stdout."
    non_interactive: true
    notes: "Activated by `--wire`; intended for custom UIs, embedding, and automated tests."
  - name: "login"
    description: "Logs in to a Kimi account."
    non_interactive: false
    notes: "Opens or waits for browser OAuth unless `--json` is used to emit event JSONL; still requires user authorization."
  - name: "logout"
    description: "Logs out from a Kimi account."
    non_interactive: false
    notes: "Supports `--json` event JSONL."
  - name: "info"
    description: "Displays version and protocol information."
    non_interactive: true
    notes: "Supports `--json` for machine-readable output."
  - name: "acp"
    description: "Starts the multi-session Agent Client Protocol server."
    non_interactive: true
    notes: "ACP clients receive `AUTH_REQUIRED` if the user has not logged in."
  - name: "mcp"
    description: "Manages MCP server configuration."
    non_interactive: false
    notes: "Includes add, list, remove, auth, reset-auth, and test; OAuth auth opens a browser."
  - name: "plugin"
    description: "Manages local plugins."
    non_interactive: false
    notes: "Includes install, list, info, and remove. Git install may perform network clone; plugin installation may read host credentials for injection."
  - name: "term"
    description: "Launches the Toad terminal UI backed by an internal ACP server."
    non_interactive: false
    notes: "Passes extra arguments through to the internal `kimi acp`; requires Python 3.14+."
  - name: "export"
    description: "Exports session data and diagnostic logs as a ZIP archive."
    non_interactive: false
    notes: "Prompts before exporting the default previous session unless `--yes` is passed."
  - name: "vis"
    description: "Starts the Agent Tracing Visualizer web server."
    non_interactive: false
    notes: "Technical preview; opens a browser by default."
  - name: "web"
    description: "Starts the browser-based Web UI server."
    non_interactive: false
    notes: "Opens a browser by default and auto-selects the next port in the 5494-5503 range if the default is occupied."
cli_switches:
  - flag: --version
    value: ""
    scope: ["global", "meta"]
    default: "false"
    description: "Shows the version and exits."
    example: "kimi --version"
    notes: "Short alias: `-V`."
  - flag: --help
    value: ""
    scope: ["global", "meta"]
    default: "false"
    description: "Shows help and exits."
    example: "kimi --help"
    notes: "Short alias: `-h`."
  - flag: --verbose
    value: ""
    scope: ["global", "diagnostics"]
    default: "false"
    description: "Prints verbose runtime information."
    example: "kimi --verbose"
    notes: ""
  - flag: --debug
    value: ""
    scope: ["global", "diagnostics"]
    default: "false"
    description: "Enables debug logging to the Kimi log file."
    example: "kimi --debug"
    notes: "Docs describe debug logs at `~/.kimi/logs/kimi.log`."
  - flag: --work-dir
    value: "<PATH>"
    scope: ["global", "workspace"]
    default: "current directory"
    description: "Sets the working directory for agent file operations."
    example: "kimi --work-dir /path/to/project"
    notes: "Short alias: `-w`."
  - flag: --add-dir
    value: "<PATH>"
    scope: ["global", "workspace"]
    default: "[]"
    description: "Adds an additional directory to the workspace scope."
    example: "kimi --add-dir ../shared"
    notes: "Repeatable; added directories persist in session state."
  - flag: --session
    value: "[ID]"
    scope: ["global", "sessions"]
    default: "none"
    description: "Resumes a session by ID, or opens an interactive session picker when no ID is supplied."
    example: "kimi --session abc123"
    notes: "Aliases: `--resume`, `-S`, `-r`. Mutually exclusive with `--continue`."
  - flag: --continue
    value: ""
    scope: ["global", "sessions"]
    default: "false"
    description: "Continues the previous session for the working directory."
    example: "kimi --continue"
    notes: "Short alias: `-C`; mutually exclusive with `--session`/`--resume`."
  - flag: --config
    value: "<TOML_OR_JSON>"
    scope: ["global", "configuration"]
    default: "none"
    description: "Loads configuration from an inline TOML or JSON string."
    example: "kimi --config '{\"default_model\":\"kimi-for-coding\"}'"
    notes: "Mutually exclusive with `--config-file`."
  - flag: --config-file
    value: "<PATH>"
    scope: ["global", "configuration"]
    default: "~/.kimi/config.toml"
    description: "Loads configuration from a TOML or JSON file."
    example: "kimi --config-file /path/to/config.toml"
    notes: "Mutually exclusive with `--config`."
  - flag: --model
    value: "<NAME>"
    scope: ["global", "model_selection"]
    default: "config default_model"
    description: "Selects the LLM model defined in config."
    example: "kimi --model kimi-for-coding"
    notes: "Short alias: `-m`."
  - flag: --thinking
    value: ""
    scope: ["global", "model_behavior"]
    default: "last session or config default"
    description: "Enables thinking mode."
    example: "kimi --thinking"
    notes: "Paired boolean with `--no-thinking`; model must support thinking."
  - flag: --no-thinking
    value: ""
    scope: ["global", "model_behavior"]
    default: "last session or config default"
    description: "Disables thinking mode."
    example: "kimi --no-thinking"
    notes: "Paired boolean with `--thinking`."
  - flag: --yolo
    value: ""
    scope: ["global", "approval"]
    default: "false"
    description: "Automatically approves tool calls while the user remains reachable for questions."
    example: "kimi --yolo"
    notes: "Aliases: `--yes`, `--auto-approve`, `-y`."
  - flag: --afk
    value: ""
    scope: ["global", "approval"]
    default: "false"
    description: "Runs away-from-keyboard: auto-approves tool calls and auto-dismisses user questions."
    example: "kimi --afk"
    notes: "Use when no user will be present."
  - flag: --plan
    value: ""
    scope: ["global", "planning"]
    default: "false"
    description: "Starts or forces plan mode."
    example: "kimi --plan"
    notes: "New sessions can default to plan mode via `default_plan_mode = true`."
  - flag: --prompt
    value: "<TEXT>"
    scope: ["global", "input"]
    default: "interactive prompt"
    description: "Passes a user prompt to the agent."
    example: "kimi --print --prompt 'Summarize this repo'"
    notes: "Aliases: `--command`, `-p`, `-c`. In shell mode this can prefill or run before continuing; in print mode it is the non-interactive task."
  - flag: --print
    value: ""
    scope: ["global", "ui_mode", "automation"]
    default: "false"
    description: "Runs in non-interactive print mode."
    example: "kimi --print -p 'List Python files'"
    notes: "Mutually exclusive with `--acp` and `--wire`; implicitly enables runtime AFK behavior."
  - flag: --quiet
    value: ""
    scope: ["global", "ui_mode", "automation"]
    default: "false"
    description: "Shortcut for print mode with text output and only the final assistant message."
    example: "kimi --quiet -p 'Write a commit message'"
    notes: "Equivalent to `--print --output-format text --final-message-only`."
  - flag: --acp
    value: ""
    scope: ["global", "ui_mode", "protocol"]
    default: "false"
    description: "Runs the ACP server from the root command."
    example: "kimi --acp"
    notes: "Deprecated in favor of `kimi acp`; mutually exclusive with `--print` and `--wire`."
  - flag: --wire
    value: ""
    scope: ["global", "ui_mode", "protocol"]
    default: "false"
    description: "Runs the experimental Wire JSON-RPC server over stdin/stdout."
    example: "kimi --wire"
    notes: "Mutually exclusive with `--print` and `--acp`."
  - flag: --input-format
    value: "<text|stream-json>"
    scope: ["print", "automation"]
    default: "text"
    description: "Selects print-mode stdin input format."
    example: "echo '{\"role\":\"user\",\"content\":\"Hello\"}' | kimi --print --input-format stream-json --output-format stream-json"
    notes: "Only valid with `--print`; `stream-json` is JSONL."
  - flag: --output-format
    value: "<text|stream-json>"
    scope: ["print", "automation"]
    default: "text"
    description: "Selects print-mode output format."
    example: "kimi --print -p 'Hello' --output-format stream-json"
    notes: "Only valid with `--print`; `stream-json` is JSONL."
  - flag: --final-message-only
    value: ""
    scope: ["print", "automation"]
    default: "false"
    description: "Prints only the final assistant message."
    example: "kimi --print -p 'Give me a commit message' --final-message-only"
    notes: "Only valid with `--print`."
  - flag: --agent
    value: "<default|okabe>"
    scope: ["global", "agent"]
    default: "default"
    description: "Uses a built-in agent specification."
    example: "kimi --agent okabe"
    notes: "Mutually exclusive with `--agent-file`."
  - flag: --agent-file
    value: "<PATH>"
    scope: ["global", "agent"]
    default: "built-in default agent"
    description: "Loads a custom YAML agent specification file."
    example: "kimi --agent-file /path/to/agent.yaml"
    notes: "Mutually exclusive with `--agent`."
  - flag: --mcp-config-file
    value: "<PATH>"
    scope: ["global", "mcp"]
    default: "~/.kimi/mcp.json if it exists"
    description: "Loads an MCP config file."
    example: "kimi --mcp-config-file /path/to/mcp.json"
    notes: "Repeatable; loaded config files use the common `mcpServers` JSON shape."
  - flag: --mcp-config
    value: "<JSON>"
    scope: ["global", "mcp"]
    default: "none"
    description: "Loads MCP config from an inline JSON string."
    example: "kimi --mcp-config '{\"mcpServers\":{\"test\":{\"url\":\"https://example.invalid\"}}}'"
    notes: "Repeatable."
  - flag: --skills-dir
    value: "<PATH>"
    scope: ["global", "skills"]
    default: "automatic discovery"
    description: "Appends additional skills directories."
    example: "kimi --skills-dir /path/to/skills"
    notes: "Repeatable; does not override `KIMI_SHARE_DIR`."
  - flag: --max-steps-per-turn
    value: "<N>"
    scope: ["global", "loop_control"]
    default: "config loop_control.max_steps_per_turn"
    description: "Overrides the maximum number of agent steps in one turn."
    example: "kimi --max-steps-per-turn 100"
    notes: "Minimum 1."
  - flag: --max-retries-per-step
    value: "<N>"
    scope: ["global", "loop_control"]
    default: "config loop_control.max_retries_per_step"
    description: "Overrides maximum retries for one step."
    example: "kimi --max-retries-per-step 3"
    notes: "Minimum 1."
  - flag: --max-ralph-iterations
    value: "<N>"
    scope: ["global", "loop_control"]
    default: "config loop_control.max_ralph_iterations"
    description: "Sets extra Ralph-loop iterations after the first turn; `0` disables and `-1` is unlimited."
    example: "kimi --max-ralph-iterations 5"
    notes: "Ralph mode repeats the same prompt until stop or limit."
  - flag: --json
    value: ""
    scope: ["login", "logout", "info"]
    default: "false"
    description: "Emits JSON or JSONL output for supported commands."
    example: "kimi info --json"
    notes: "`info --json` emits one JSON object; `login --json` and `logout --json` emit OAuth event JSON lines."
  - flag: --output
    value: "<PATH>"
    scope: ["export"]
    default: "session-<id>.zip"
    description: "Sets the export ZIP output path."
    example: "kimi export abc123 --output session.zip"
    notes: "Short alias: `-o`."
  - flag: --yes
    value: ""
    scope: ["export"]
    default: "false"
    description: "Skips confirmation when exporting the default previous session."
    example: "kimi export --yes"
    notes: "Short alias: `-y`; separate from global `--yes` alias for YOLO."
  - flag: --transport
    value: "<stdio|http>"
    scope: ["mcp add"]
    default: "stdio"
    description: "Selects MCP server transport."
    example: "kimi mcp add --transport http context7 https://mcp.context7.com/mcp"
    notes: "Short alias: `-t`."
  - flag: --env
    value: "<KEY=VALUE>"
    scope: ["mcp add"]
    default: "[]"
    description: "Adds an environment variable for stdio MCP servers."
    example: "kimi mcp add --transport stdio myserver --env FOO=bar -- command"
    notes: "Short alias: `-e`; repeatable; stdio only."
  - flag: --header
    value: "<KEY:VALUE>"
    scope: ["mcp add"]
    default: "[]"
    description: "Adds an HTTP header for HTTP MCP servers."
    example: "kimi mcp add --transport http context7 https://mcp.context7.com/mcp --header 'CONTEXT7_API_KEY: token'"
    notes: "Short alias: `-H`; repeatable; HTTP only."
  - flag: --auth
    value: "<TYPE>"
    scope: ["mcp add"]
    default: "none"
    description: "Sets MCP HTTP authorization type."
    example: "kimi mcp add --transport http --auth oauth linear https://mcp.linear.app/mcp"
    notes: "Short alias: `-a`; docs show `oauth`."
  - flag: --host
    value: "<TEXT>"
    scope: ["web", "vis"]
    default: "127.0.0.1 unless --network is used"
    description: "Binds the web server to a specific IP address."
    example: "kimi web --host 192.168.1.100"
    notes: "Short alias: `-h`."
  - flag: --network
    value: ""
    scope: ["web", "vis"]
    default: "false"
    description: "Binds the server to all network interfaces."
    example: "kimi web --network"
    notes: "Short alias: `-n`; use web access-control flags when exposing beyond localhost."
  - flag: --port
    value: "<INTEGER>"
    scope: ["web", "vis"]
    default: "5494 for web, 5495 for vis"
    description: "Sets the web server port."
    example: "kimi web --port 8080"
    notes: "Short alias: `-p`."
  - flag: --open
    value: ""
    scope: ["web", "vis"]
    default: "true"
    description: "Opens the browser automatically."
    example: "kimi web --open"
    notes: "Paired with `--no-open`."
  - flag: --no-open
    value: ""
    scope: ["web", "vis"]
    default: "false"
    description: "Prevents automatically opening a browser."
    example: "kimi web --no-open"
    notes: "Paired with `--open`."
  - flag: --reload
    value: ""
    scope: ["web", "vis"]
    default: "false"
    description: "Enables auto-reload development mode."
    example: "kimi web --reload"
    notes: ""
  - flag: --auth-token
    value: "<TEXT>"
    scope: ["web", "access_control"]
    default: "none"
    description: "Sets a Bearer token for Web UI API authentication."
    example: "kimi web --network --auth-token my-secret-token"
    notes: "Use a long random token when enabling network access."
  - flag: --allowed-origins
    value: "<TEXT>"
    scope: ["web", "access_control"]
    default: "none"
    description: "Sets a comma-separated allowlist of Origin values."
    example: "kimi web --network --allowed-origins 'https://example.com'"
    notes: ""
  - flag: --lan-only
    value: ""
    scope: ["web", "access_control"]
    default: "true"
    description: "Allows only LAN access."
    example: "kimi web --lan-only"
    notes: "Paired with `--public`."
  - flag: --public
    value: ""
    scope: ["web", "access_control"]
    default: "false"
    description: "Allows public network access."
    example: "kimi web --network --public --auth-token token"
    notes: "Public access should be paired with authentication and origin restrictions."
  - flag: --restrict-sensitive-apis
    value: ""
    scope: ["web", "access_control"]
    default: "enabled in public mode, disabled in LAN-only mode"
    description: "Restricts sensitive Web UI APIs such as config writes, open-in, and file access limits."
    example: "kimi web --network --restrict-sensitive-apis"
    notes: "Paired with `--no-restrict-sensitive-apis`."
  - flag: --no-restrict-sensitive-apis
    value: ""
    scope: ["web", "access_control"]
    default: "see --restrict-sensitive-apis"
    description: "Disables sensitive API restrictions."
    example: "kimi web --no-restrict-sensitive-apis"
    notes: "Paired with `--restrict-sensitive-apis`."
  - flag: --dangerously-omit-auth
    value: ""
    scope: ["web", "access_control"]
    default: "false"
    description: "Disables Web UI authentication checks."
    example: "kimi web --dangerously-omit-auth"
    notes: "Dangerous; intended only for fully trusted networks."
config_files:
  - os: all
    scope: user
    path: "~/.kimi/config.toml"
    format: toml
    notes: "Default user configuration file; Kimi creates it on first run if missing. Kimi also supports JSON configuration via `--config-file`."
  - os: all
    scope: user
    path: "~/.kimi/mcp.json"
    format: json
    notes: "Default MCP server configuration file; loaded automatically if it exists and no explicit MCP config file is passed."
  - os: all
    scope: user
    path: "~/.kimi/kimi.json"
    format: json
    notes: "CLI-managed metadata storing work directory/session state and the last thinking-mode state."
  - os: all
    scope: other
    path: "--config-file <PATH>"
    format: other
    notes: "Explicit TOML or JSON config file path."
  - os: all
    scope: other
    path: "--mcp-config-file <PATH>"
    format: json
    notes: "Explicit MCP config file path; repeatable."
  - os: all
    scope: user
    path: "~/.kimi/credentials/<provider>.json"
    format: json
    notes: "OAuth credentials written by login; not a user-authored config file."
  - os: all
    scope: user
    path: "~/.kimi/mcp-oauth/"
    format: other
    notes: "MCP OAuth token storage for remote MCP servers."
env_vars:
  - name: KIMI_SHARE_DIR
    effect: "Overrides the CLI share/runtime data directory, defaulting to `~/.kimi`; does not change skills search paths."
  - name: KIMI_CLI_NO_AUTO_UPDATE
    effect: "Disables update-related features when set to one of `1`, `true`, `t`, `yes`, or `y` case-insensitively."
  - name: KIMI_CLI_PASTE_CHAR_THRESHOLD
    effect: "Controls the character threshold for folding pasted text in agent mode; default is `1000`."
  - name: KIMI_CLI_PASTE_LINE_THRESHOLD
    effect: "Controls the line threshold for folding pasted text in agent mode; default is `15`."
  - name: KIMI_CLI_GIT_BASH_PATH
    effect: "Windows-only override for locating Git Bash, which the Shell tool uses as the Windows shell backend."
machine_introspection:
  - command: "kimi info --json"
    purpose: version
    machine_readable: true
    output_format: json
    useful_for_codegen: true
    notes: "Returns `kimi_cli_version`, supported agent spec versions, Wire protocol version, and Python runtime version."
  - command: "printf '%s\n' '{\"jsonrpc\":\"2.0\",\"method\":\"initialize\",\"id\":\"1\",\"params\":{\"protocol_version\":\"1.10\",\"client\":{\"name\":\"claudine\",\"version\":\"0\"}}}' | kimi --wire"
    purpose: capabilities
    machine_readable: true
    output_format: jsonl
    useful_for_codegen: true
    notes: "Wire initialize returns server info, protocol version, available slash commands, and capabilities. Requires a configured provider/session environment and process management because `kimi --wire` is a long-running stdio server."
  - command: "kimi login --json"
    purpose: env
    machine_readable: true
    output_format: jsonl
    useful_for_codegen: false
    notes: "Emits OAuth event JSONL but still requires browser/user authorization; useful for wrappers that need to present login progress, not for static provider metadata."
  - command: "kimi logout --json"
    purpose: env
    machine_readable: true
    output_format: jsonl
    useful_for_codegen: false
    notes: "Emits logout event JSONL; useful for wrapper status handling."
  - command: "kimi mcp list"
    purpose: mcp
    machine_readable: false
    output_format: text
    useful_for_codegen: false
    notes: "Lists configured MCP servers and OAuth authorization status in human-readable text; no JSON flag is documented or present in the 1.48.0 source."
  - command: "kimi mcp test <name>"
    purpose: tools
    machine_readable: false
    output_format: text
    useful_for_codegen: false
    notes: "Connects to one MCP server and lists available tools in text; useful for diagnostics but not stable codegen input."
wrapper_notes:
  - "The legacy `MoonshotAI/kimi-cli` project is being wound down in favor of the newer `MoonshotAI/kimi-code`; the hosted Kimi CLI docs explicitly recommend new users install Kimi Code directly."
  - "The current upstream package version verified from PyPI and `pyproject.toml` is 1.48.0; the hosted changelog content fetched during this pass listed 1.47.0 as the latest documented release."
  - "Official install scripts install uv if necessary and then install `kimi-cli` with Python 3.13; `kimi term` separately requires Python 3.14+."
  - "The package exposes both `kimi` and `kimi-cli` console scripts; wrapper detection should accept either but prefer `kimi`."
  - "Print mode (`--print`) is the primary non-interactive path, exits automatically, writes assistant output to stdout, and implicitly enables AFK behavior that auto-approves tools and auto-dismisses questions."
  - "`--quiet` is useful for wrappers that need final text only: it maps to `--print --output-format text --final-message-only`."
  - "Structured print output is JSONL via `--output-format stream-json`; stdin JSONL is selected with `--input-format stream-json`."
  - "Print-mode exit codes are documented as 0 success, 1 permanent failure, and 75 retryable/transient failure."
  - "The CLI redirects runtime stderr noise to `~/.kimi/logs/kimi.log` after startup; fatal startup/runtime errors may still be written to the original stderr."
  - "MCP server loading is asynchronous in shell mode; wrapper code should not assume all MCP tools are ready immediately after interactive shell startup."
  - "`kimi mcp auth` and `kimi login` can open a browser and require user action; ACP returns an `AUTH_REQUIRED` JSON-RPC error when login is missing."
  - "Windows Shell tool execution uses Git Bash in recent releases and requires Git for Windows; `KIMI_CLI_GIT_BASH_PATH` can override discovery."
  - "The Web UI opens a browser by default and may expose sensitive local APIs; wrappers that start `kimi web` should pass explicit host/auth flags instead of relying on defaults for remote access."
changes: []
requires_claudine_update: true
reason: "Claudine provider metadata should recognize the `kimi-cli` alias, Kimi print-mode JSONL/non-interactive flags, `kimi info --json` introspection, and the upstream migration caveat from `MoonshotAI/kimi-cli` to `MoonshotAI/kimi-code`."
---

# Kimi Code CLI

## Overview

Kimi Code CLI is Moonshot AI's terminal AI coding agent. The legacy public CLI researched here is the Python package and repository at `MoonshotAI/kimi-cli`, installed as `kimi-cli` on PyPI and exposed mainly through the `kimi` command. The official README and getting-started docs now say this project is evolving into the newer standalone Kimi Code project and that new users are encouraged to install Kimi Code directly, while the legacy docs and package remain available.

The latest package version verified during this pass is `1.48.0` from PyPI and the repository `pyproject.toml`. The hosted changelog content fetched during the same pass listed `1.47.0` as the latest documented release, so wrappers should treat PyPI/package metadata as the version source and docs as slightly lagging.

## Installation and Binaries

Official installation uses uv. The Linux/macOS script at `https://code.kimi.com/install.sh` installs uv if needed and then runs `uv tool install --python 3.13 kimi-cli`; the Windows PowerShell script does the same through `install.ps1`. If uv is already installed, the docs show direct installation with:

```sh
uv tool install --python 3.13 kimi-cli
```

The package declares two console scripts, `kimi` and `kimi-cli`, both mapped to `kimi_cli.__main__:main`. Official documentation and examples consistently use `kimi`. Windows installs may materialize `.exe` launchers or command shims for those console scripts.

Python support is documented as 3.12-3.14 with Python 3.13 recommended. The `kimi term` Toad UI is a special case that requires Python 3.14+.

## Subcommands

The root command starts an interactive terminal session by default. For automation, use `kimi --print` with `--prompt`/`-p` or stdin; `--quiet` is the compact final-message-only shortcut. Protocol modes are available through `kimi acp`, deprecated root `--acp`, and experimental `--wire`.

Top-level commands verified from docs and the 1.48.0 source are `login`, `logout`, `info`, `acp`, `mcp`, `plugin`, `term`, `export`, `vis`, and `web`. Hidden internal worker commands exist in source but are not public wrapper targets.

## CLI Switch Inventory

The structured frontmatter records the wrapper-relevant switch inventory. Important automation switches are:

- `--print`, `--quiet`, `--input-format`, `--output-format`, and `--final-message-only` for non-interactive execution.
- `--prompt`/`--command` with `-p`/`-c` for direct task input.
- `--config`, `--config-file`, `--model`, `--thinking`/`--no-thinking`, and loop-control flags for startup behavior.
- `--yolo` and `--afk` for approval posture; `--print` implicitly uses runtime AFK behavior.
- `--mcp-config-file` and `--mcp-config` for ad hoc MCP config injection.
- `kimi info --json`, `kimi login --json`, and `kimi logout --json` for structured status/event output.

The docs state print-mode exit codes are `0` for success, `1` for non-retryable failures, and `75` for retryable transient failures.

## Configuration Discovery

The default config file is `~/.kimi/config.toml`, and Kimi creates it on first run if missing. Config files support TOML and JSON, and callers can override config with `--config-file <PATH>` or inline `--config <TOML_OR_JSON>`. The documented precedence is environment variables first, then CLI flags, then config file.

Runtime data lives under `~/.kimi/` unless `KIMI_SHARE_DIR` is set. Important wrapper-visible files include `config.toml`, `kimi.json`, `mcp.json`, `credentials/<provider>.json`, `mcp-oauth/`, `sessions/`, `plans/`, `user-history/`, and `logs/kimi.log`.

MCP config is stored in `~/.kimi/mcp.json` and follows the common `mcpServers` JSON shape. If no explicit MCP config is passed, the CLI loads this file when it exists.

## Environment Variables

General CLI/runtime variables captured in frontmatter are `KIMI_SHARE_DIR`, `KIMI_CLI_NO_AUTO_UPDATE`, `KIMI_CLI_PASTE_CHAR_THRESHOLD`, `KIMI_CLI_PASTE_LINE_THRESHOLD`, and the Windows shell-discovery override `KIMI_CLI_GIT_BASH_PATH`.

Model/provider environment variables such as `KIMI_API_KEY`, `KIMI_BASE_URL`, `KIMI_MODEL_NAME`, `KIMI_MODEL_*`, `OPENAI_API_KEY`, and `OPENAI_BASE_URL` are documented upstream, but they belong primarily to model/provider configuration rather than this general CLI surface.

## Machine Introspection

`kimi info --json` is the cleanest machine-readable command. It returns version and protocol fields including `kimi_cli_version`, `agent_spec_versions`, `wire_protocol_version`, and `python_version`.

`kimi --wire` can be introspected by sending a JSON-RPC `initialize` request over stdin/stdout. The response includes protocol version, server info, slash commands, and capabilities, but it is a long-running protocol server and requires normal process lifecycle management.

`kimi login --json` and `kimi logout --json` emit OAuth event JSONL, but login still depends on browser/user authorization. MCP and plugin listing commands are text-only in the public docs and 1.48.0 source inspected here.

## Wrapper Notes

Use `kimi --print` or `kimi --quiet` for one-shot wrapper execution. Prefer `--output-format stream-json` when Claudine needs event-like output; prefer `--quiet` when only final text is needed. Do not use interactive shell, `web`, `vis`, `term`, `login`, or MCP OAuth auth commands in unattended runs unless the wrapper is explicitly presenting an interactive flow.

Wrapper detection should accept both `kimi` and `kimi-cli` and prefer `kimi`. On Windows, account for Python/uv launcher suffixes and the Git Bash runtime requirement for shell tool execution.

The Kimi CLI docs and source show a migration boundary: `MoonshotAI/kimi-cli` remains usable and documented, but the team points new users toward `MoonshotAI/kimi-code`. Claudine metadata should keep that distinction visible so users and wrappers do not conflate the Python legacy CLI with the newer standalone Kimi Code implementation.

## Sources

- [Kimi CLI repository README](https://github.com/MoonshotAI/kimi-cli)
- [Kimi Code CLI docs](https://moonshotai.github.io/kimi-cli/en/)
- [`kimi` command reference](https://moonshotai.github.io/kimi-cli/en/reference/kimi-command.html)
- [Getting started and installation](https://moonshotai.github.io/kimi-cli/en/guides/getting-started.html)
- [Config files](https://moonshotai.github.io/kimi-cli/en/configuration/config-files.html)
- [Config overrides](https://moonshotai.github.io/kimi-cli/en/configuration/overrides.html)
- [Environment variables](https://moonshotai.github.io/kimi-cli/en/configuration/env-vars.html)
- [Data locations](https://moonshotai.github.io/kimi-cli/en/configuration/data-locations.html)
- [`kimi info` reference](https://moonshotai.github.io/kimi-cli/en/reference/kimi-info.html)
- [`kimi mcp` reference](https://moonshotai.github.io/kimi-cli/en/reference/kimi-mcp.html)
- [`kimi acp` reference](https://moonshotai.github.io/kimi-cli/en/reference/kimi-acp.html)
- [`kimi web` reference](https://moonshotai.github.io/kimi-cli/en/reference/kimi-web.html)
- [`kimi term` reference](https://moonshotai.github.io/kimi-cli/en/reference/kimi-term.html)
- [Print mode](https://moonshotai.github.io/kimi-cli/en/customization/print-mode.html)
- [Wire mode](https://moonshotai.github.io/kimi-cli/en/customization/wire-mode.html)
- [Plugins](https://moonshotai.github.io/kimi-cli/en/customization/plugins.html)
- [PyPI `kimi-cli` package](https://pypi.org/project/kimi-cli/)
- [1.48.0 `pyproject.toml`](https://raw.githubusercontent.com/MoonshotAI/kimi-cli/main/pyproject.toml)
