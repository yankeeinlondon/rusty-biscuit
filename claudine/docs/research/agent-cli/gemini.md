---
$schema: ./_schema.yaml
created: 2026-07-02
last_updated: 2026-07-02
agent: codex
model: default
latest_version: "0.49.0"
homepage: https://geminicli.com/
repo: https://github.com/google-gemini/gemini-cli
docs: https://geminicli.com/docs/
cli_docs: https://geminicli.com/docs/cli/cli-reference/
binaries:
  - os: all
    binary: gemini
    alt_binaries: []
    notes: "Primary executable declared by the @google/gemini-cli npm package and used by official docs."
  - os: windows
    binary: gemini.cmd
    alt_binaries: ["gemini.ps1"]
    notes: "Expected npm command shims on Windows; official examples still invoke the command as `gemini`."
install_methods:
  - os: all
    method: npm
    command: "npm install -g @google/gemini-cli"
    notes: "Official global install. Requires Node.js 20.0.0+."
  - os: all
    method: npm
    command: "npx @google/gemini-cli"
    notes: "Official no-permanent-install execution path."
  - os: macos
    method: brew
    command: "brew install gemini-cli"
    notes: "Official Homebrew install; docs describe Homebrew as macOS/Linux."
  - os: linux
    method: brew
    command: "brew install gemini-cli"
    notes: "Official Homebrew install; requires Linuxbrew/Homebrew on Linux."
  - os: macos
    method: package_manager
    command: "sudo port install gemini-cli"
    notes: "Official MacPorts install."
  - os: all
    method: other
    command: "conda create -y -n gemini_env -c conda-forge nodejs && conda activate gemini_env && npm install -g @google/gemini-cli"
    notes: "Official Anaconda path for restricted environments; install is still npm inside the conda environment."
  - os: all
    method: other
    command: "docker run --rm -it us-docker.pkg.dev/gemini-code-dev/gemini-cli/sandbox:<version>"
    notes: "Official container execution path; docs show a versioned sandbox image rather than a stable `latest` image command."
  - os: all
    method: source
    command: "npm run start"
    notes: "Official source-tree development command from the repository root."
subcommands:
  - name: default
    description: "Launches the interactive REPL when no non-interactive prompt mode is selected; accepts a variadic positional query."
    non_interactive: false
    notes: "A positional query starts/continues an interactive session by default; use `-p/--prompt` for headless execution."
  - name: mcp
    description: "Manages configured MCP servers."
    non_interactive: false
    notes: "Top-level group includes add, remove, list, enable, and disable in local 0.46.0 inspection."
  - name: extensions
    description: "Manages Gemini CLI extensions."
    non_interactive: false
    notes: "Alias: `extension`. Includes install, uninstall, list, update, enable, disable, link, new, and validate in docs/source."
  - name: skills
    description: "Manages agent skills."
    non_interactive: false
    notes: "Alias: `skill`. Includes list, enable, disable, install, link, and uninstall in local 0.46.0 inspection."
  - name: hooks
    description: "Manages Gemini CLI hooks."
    non_interactive: false
    notes: "Alias: `hook`. Local 0.46.0 exposes `hooks migrate`."
  - name: gemma
    description: "Manages local Gemma model routing."
    non_interactive: false
    notes: "Local 0.46.0 exposes setup, start, stop, status, and logs."
  - name: update
    description: "Updates the CLI to the latest version."
    non_interactive: false
    notes: "Documented in the official CLI cheatsheet, but not shown by local 0.46.0 top-level help."
cli_switches:
  - flag: --debug
    value: ""
    scope: ["global", "diagnostics"]
    default: "false"
    description: "Runs in debug mode with verbose logging."
    example: "gemini --debug"
    notes: "Alias: `-d`."
  - flag: --version
    value: ""
    scope: ["global", "metadata"]
    default: "false"
    description: "Shows the CLI version number and exits."
    example: "gemini --version"
    notes: "Alias: `-v`; local inspection returned 0.46.0 while npm latest was 0.49.0."
  - flag: --help
    value: ""
    scope: ["global", "help"]
    default: "false"
    description: "Shows help information."
    example: "gemini --help"
    notes: "Alias: `-h`."
  - flag: --model
    value: "<MODEL>"
    scope: ["global", "model_selection"]
    default: "auto"
    description: "Selects the model or model alias for the session."
    example: "gemini --model flash -p \"summarize README.md\""
    notes: "Alias: `-m`. Model-specific details belong in model-config research."
  - flag: --prompt
    value: "<PROMPT>"
    scope: ["global", "non_interactive"]
    default: ""
    description: "Passes prompt text directly and forces non-interactive headless mode."
    example: "gemini --prompt \"summarize README.md\""
    notes: "Alias: `-p`. Official docs say prompt text is appended to stdin input if stdin is provided."
  - flag: --prompt-interactive
    value: "<PROMPT>"
    scope: ["global", "interactive"]
    default: ""
    description: "Starts an interactive session with the provided prompt."
    example: "gemini --prompt-interactive \"explain this project\""
    notes: "Alias: `-i`. Cannot be used when piping input from stdin."
  - flag: --worktree
    value: "[NAME]"
    scope: ["global", "workspace"]
    default: ""
    description: "Starts Gemini in a new git worktree, generating a name when no name is provided."
    example: "gemini --worktree feature-a"
    notes: "Alias: `-w`. Docs say it requires `experimental.worktrees: true`."
  - flag: --sandbox
    value: ""
    scope: ["global", "execution"]
    default: "false"
    description: "Runs in a sandboxed environment for safer execution."
    example: "gemini --sandbox -p \"run tests\""
    notes: "Alias: `-s`. Can also be controlled by `GEMINI_SANDBOX` and settings."
  - flag: --skip-trust
    value: ""
    scope: ["global", "workspace_trust"]
    default: "false"
    description: "Trusts the current workspace for this session and skips the folder trust check."
    example: "gemini --skip-trust"
    notes: "Only relevant when folder trust is enabled."
  - flag: --approval-mode
    value: "<default|auto_edit|yolo|plan>"
    scope: ["global", "tool_approval"]
    default: "default"
    description: "Sets the approval mode for tool execution."
    example: "gemini --approval-mode auto_edit"
    notes: "Cannot be combined with `--yolo`; docs mark `plan` as under development/not fully functional."
  - flag: --yolo
    value: ""
    scope: ["global", "tool_approval"]
    default: "false"
    description: "Deprecated shortcut that automatically approves all actions."
    example: "gemini --yolo -p \"fix lint\""
    notes: "Alias: `-y`. Use `--approval-mode=yolo` instead."
  - flag: --acp
    value: ""
    scope: ["global", "protocol"]
    default: "false"
    description: "Starts the agent in Agent Communication Protocol mode."
    example: "gemini --acp"
    notes: "Present in current configuration docs and local 0.46.0 source."
  - flag: --experimental-acp
    value: ""
    scope: ["global", "protocol"]
    default: "false"
    description: "Deprecated experimental ACP flag."
    example: "gemini --experimental-acp"
    notes: "Local 0.46.0 source says to use `--acp` instead."
  - flag: --experimental-zed-integration
    value: ""
    scope: ["global", "ide"]
    default: ""
    description: "Runs in Zed editor integration mode."
    example: "gemini --experimental-zed-integration"
    notes: "Listed in the bundled CLI reference docs, but not observed in local 0.46.0 source option builder."
  - flag: --policy
    value: "<PATH>"
    scope: ["global", "policy"]
    default: ""
    description: "Loads additional policy files or directories."
    example: "gemini --policy ./policy.toml"
    notes: "Local 0.46.0 source accepts comma-separated values or multiple flags."
  - flag: --admin-policy
    value: "<PATH>"
    scope: ["global", "policy"]
    default: ""
    description: "Loads additional admin policy files or directories."
    example: "gemini --admin-policy /etc/gemini-cli/policy.toml"
    notes: "Local 0.46.0 source accepts comma-separated values or multiple flags."
  - flag: --allowed-mcp-server-names
    value: "<NAME,...>"
    scope: ["global", "mcp"]
    default: ""
    description: "Restricts allowed MCP server names for the session."
    example: "gemini --allowed-mcp-server-names github,filesystem"
    notes: "Accepts comma-separated values or multiple flags."
  - flag: --allowed-tools
    value: "<TOOL,...>"
    scope: ["global", "tool_approval"]
    default: ""
    description: "Deprecated list of tools allowed to run without confirmation."
    example: "gemini --allowed-tools \"ShellTool(git status)\""
    notes: "Docs direct users to the Policy Engine instead."
  - flag: --extensions
    value: "<NAME,...>"
    scope: ["global", "extensions"]
    default: "all extensions enabled"
    description: "Selects extensions to use for the session."
    example: "gemini --extensions my-extension"
    notes: "Alias: `-e`. Docs say `gemini -e none` disables all extensions."
  - flag: --list-extensions
    value: ""
    scope: ["global", "extensions", "introspection"]
    default: "false"
    description: "Lists all available extensions and exits."
    example: "gemini --list-extensions"
    notes: "Alias: `-l`."
  - flag: --resume
    value: "[SESSION]"
    scope: ["global", "sessions"]
    default: ""
    description: "Resumes a previous chat session by latest, index, or UUID."
    example: "gemini --resume latest \"continue the fix\""
    notes: "Alias: `-r`. If provided with no value, docs say it defaults to `latest`."
  - flag: --session-file
    value: "<PATH>"
    scope: ["global", "sessions"]
    default: ""
    description: "Loads a session from a JSON file."
    example: "gemini --session-file ./session.json"
    notes: "Observed in local 0.46.0 source; not present in bundled CLI reference table."
  - flag: --session-id
    value: "<ID>"
    scope: ["global", "sessions"]
    default: ""
    description: "Starts a new session with a manually provided session id."
    example: "gemini --session-id run_123"
    notes: "Observed in local 0.46.0 source; accepted characters are alphanumeric, dash, and underscore."
  - flag: --list-sessions
    value: ""
    scope: ["global", "sessions", "introspection"]
    default: "false"
    description: "Lists available sessions for the current project and exits."
    example: "gemini --list-sessions"
    notes: "Output is human text, not documented as JSON."
  - flag: --delete-session
    value: "<INDEX_OR_UUID>"
    scope: ["global", "sessions"]
    default: ""
    description: "Deletes a session by index number or UUID."
    example: "gemini --delete-session 3"
    notes: "Use `--list-sessions` first."
  - flag: --include-directories
    value: "<DIR,...>"
    scope: ["global", "workspace"]
    default: ""
    description: "Adds directories to include in the workspace context."
    example: "gemini --include-directories ../lib,../docs"
    notes: "Docs say it can be repeated or comma-separated and is limited to 5 directories."
  - flag: --screen-reader
    value: ""
    scope: ["global", "accessibility"]
    default: "false"
    description: "Enables screen reader mode."
    example: "gemini --screen-reader"
    notes: ""
  - flag: --output-format
    value: "<text|json|stream-json>"
    scope: ["global", "non_interactive", "output"]
    default: "text"
    description: "Selects text, JSON, or streaming JSON output for non-interactive mode."
    example: "gemini -p \"summarize README.md\" --output-format stream-json"
    notes: "Alias: `-o`. `stream-json` emits newline-delimited events."
  - flag: --fake-responses
    value: "<PATH>"
    scope: ["global", "testing"]
    default: ""
    description: "Uses fake model responses for testing."
    example: "gemini --fake-responses ./responses.json"
    notes: "Hidden local 0.46.0 option."
  - flag: --fake-responses-non-strict
    value: "<PATH>"
    scope: ["global", "testing"]
    default: ""
    description: "Uses fake model responses for testing in non-strict mode."
    example: "gemini --fake-responses-non-strict ./responses.json"
    notes: "Hidden local 0.46.0 option."
  - flag: --record-responses
    value: "<PATH>"
    scope: ["global", "testing"]
    default: ""
    description: "Records model responses to a file for testing."
    example: "gemini --record-responses ./responses.jsonl"
    notes: "Documented in configuration reference; hidden in local 0.46.0 source."
  - flag: --scope
    value: "<user|project>"
    scope: ["mcp add", "mcp remove"]
    default: "project"
    description: "Selects configuration scope for MCP server changes."
    example: "gemini mcp add db node db-server.js --scope user"
    notes: "Alias: `-s` for MCP commands."
  - flag: --transport
    value: "<stdio|sse|http>"
    scope: ["mcp add"]
    default: "stdio"
    description: "Selects MCP server transport."
    example: "gemini mcp add api-server http://localhost:3000 --transport http"
    notes: "Aliases: `-t`, `--type`."
  - flag: --env
    value: "<KEY=VALUE>"
    scope: ["mcp add"]
    default: ""
    description: "Sets environment variables for an MCP server."
    example: "gemini mcp add slack node server.js --env SLACK_TOKEN=xoxb-xxx"
    notes: "Alias: `-e`. Can be repeated."
  - flag: --header
    value: "<HEADER: VALUE>"
    scope: ["mcp add"]
    default: ""
    description: "Sets HTTP headers for SSE and HTTP MCP transports."
    example: "gemini mcp add --transport http --header \"Authorization: Bearer abc123\" secure https://api.example.com/mcp/"
    notes: "Alias: `-H`. Can be repeated."
  - flag: --timeout
    value: "<MILLISECONDS>"
    scope: ["mcp add"]
    default: ""
    description: "Sets the MCP connection timeout in milliseconds."
    example: "gemini mcp add server node server.js --timeout 30000"
    notes: ""
  - flag: --trust
    value: ""
    scope: ["mcp add"]
    default: "false"
    description: "Trusts the MCP server and bypasses all tool-call confirmation prompts for that server."
    example: "gemini mcp add server node server.js --trust"
    notes: "Wrapper-relevant because it changes approval behavior persisted in settings."
  - flag: --description
    value: "<TEXT>"
    scope: ["mcp add"]
    default: ""
    description: "Sets the MCP server description."
    example: "gemini mcp add server node server.js --description \"Local tools\""
    notes: ""
  - flag: --include-tools
    value: "<TOOL,...>"
    scope: ["mcp add"]
    default: ""
    description: "Limits the MCP server to specific tools."
    example: "gemini mcp add github npx -y @modelcontextprotocol/server-github --include-tools list_repos,get_pr"
    notes: ""
  - flag: --exclude-tools
    value: "<TOOL,...>"
    scope: ["mcp add"]
    default: ""
    description: "Excludes specific tools from the MCP server."
    example: "gemini mcp add server node server.js --exclude-tools delete_repo"
    notes: ""
  - flag: --session
    value: ""
    scope: ["mcp enable", "mcp disable"]
    default: "false"
    description: "Applies MCP enable/disable state only to the current session."
    example: "gemini mcp disable github --session"
    notes: "For `enable`, local help describes this as clearing a session-only disable."
  - flag: --ref
    value: "<GIT_REF>"
    scope: ["extensions install"]
    default: ""
    description: "Installs an extension from a specific git ref."
    example: "gemini extensions install https://github.com/user/my-extension --ref develop"
    notes: ""
  - flag: --auto-update
    value: ""
    scope: ["extensions install"]
    default: "false"
    description: "Enables auto-update for an installed extension."
    example: "gemini extensions install https://github.com/user/my-extension --auto-update"
    notes: ""
  - flag: --pre-release
    value: ""
    scope: ["extensions install"]
    default: "false"
    description: "Enables pre-release extension versions."
    example: "gemini extensions install https://github.com/user/my-extension --pre-release"
    notes: ""
  - flag: --consent
    value: ""
    scope: ["extensions install", "extensions link", "skills install", "skills link", "gemma setup"]
    default: "false"
    description: "Acknowledges security/installation consent and skips the confirmation prompt."
    example: "gemini skills install https://github.com/user/repo.git --consent"
    notes: "Wrapper-relevant for non-interactive installation flows."
  - flag: --skip-settings
    value: ""
    scope: ["extensions install"]
    default: "false"
    description: "Skips extension install-time settings configuration."
    example: "gemini extensions install ./extension --skip-settings"
    notes: "Observed in local 0.46.0 source."
  - flag: --all
    value: ""
    scope: ["extensions uninstall", "extensions update", "skills list"]
    default: "false"
    description: "Applies the command to all applicable items or includes built-in skills in listings."
    example: "gemini extensions update --all"
    notes: "Meaning is command-specific."
  - flag: --scope
    value: "<user|workspace>"
    scope: ["extensions enable", "extensions disable", "skills disable", "skills install", "skills link", "skills uninstall"]
    default: "command-specific"
    description: "Selects user/workspace scope for extension or skill state."
    example: "gemini skills install ./skill --scope workspace"
    notes: "Extension code uses setting scopes; skill commands use user/workspace."
  - flag: --path
    value: "<SUBPATH>"
    scope: ["skills install"]
    default: ""
    description: "Installs a skill from a subdirectory inside a git repository source."
    example: "gemini skills install https://github.com/user/repo.git --path skills/security"
    notes: ""
  - flag: --port
    value: "<PORT>"
    scope: ["gemma setup"]
    default: "unknown"
    description: "Sets the LiteRT server port for local Gemma routing."
    example: "gemini gemma setup --port 8080"
    notes: "Local source default is a constant; the numeric value was not proven from docs/help."
  - flag: --skip-model
    value: ""
    scope: ["gemma setup"]
    default: "false"
    description: "Skips model download during Gemma setup."
    example: "gemini gemma setup --skip-model"
    notes: ""
  - flag: --start
    value: ""
    scope: ["gemma setup"]
    default: "true"
    description: "Starts the LiteRT server after Gemma setup."
    example: "gemini gemma setup --start"
    notes: ""
  - flag: --force
    value: ""
    scope: ["gemma setup"]
    default: "false"
    description: "Re-downloads the Gemma binary and model even if already present."
    example: "gemini gemma setup --force"
    notes: ""
config_files:
  - os: linux
    scope: system
    path: /etc/gemini-cli/system-defaults.json
    format: json
    notes: "System defaults file; can be overridden with `GEMINI_CLI_SYSTEM_DEFAULTS_PATH`."
  - os: windows
    scope: system
    path: C:\ProgramData\gemini-cli\system-defaults.json
    format: json
    notes: "System defaults file; can be overridden with `GEMINI_CLI_SYSTEM_DEFAULTS_PATH`."
  - os: macos
    scope: system
    path: /Library/Application Support/GeminiCli/system-defaults.json
    format: json
    notes: "System defaults file; can be overridden with `GEMINI_CLI_SYSTEM_DEFAULTS_PATH`."
  - os: all
    scope: user
    path: ~/.gemini/settings.json
    format: json
    notes: "Primary user settings file."
  - os: all
    scope: repo
    path: .gemini/settings.json
    format: json
    notes: "Project settings file under the project root."
  - os: linux
    scope: system
    path: /etc/gemini-cli/settings.json
    format: json
    notes: "System override settings; can be overridden with `GEMINI_CLI_SYSTEM_SETTINGS_PATH`."
  - os: windows
    scope: system
    path: C:\ProgramData\gemini-cli\settings.json
    format: json
    notes: "System override settings; can be overridden with `GEMINI_CLI_SYSTEM_SETTINGS_PATH`."
  - os: macos
    scope: system
    path: /Library/Application Support/GeminiCli/settings.json
    format: json
    notes: "System override settings; can be overridden with `GEMINI_CLI_SYSTEM_SETTINGS_PATH`."
  - os: all
    scope: user
    path: ~/.gemini/GEMINI.md
    format: text
    notes: "Global context/memory file; filename is configurable via `context.fileName`."
  - os: all
    scope: repo
    path: GEMINI.md
    format: text
    notes: "Project context/memory file discovered hierarchically; filename is configurable via `context.fileName`."
  - os: all
    scope: repo
    path: .geminiignore
    format: text
    notes: "Project ignore file for Gemini file discovery when `context.fileFiltering.respectGeminiIgnore` is true."
  - os: all
    scope: repo
    path: .env
    format: text
    notes: "Environment variables may be loaded from `.env` files; project `.env` excludes DEBUG/DEBUG_MODE and other configured variables."
  - os: all
    scope: user
    path: ~/.gemini/skills/
    format: other
    notes: "User agent skill discovery directory."
  - os: all
    scope: repo
    path: .gemini/skills/
    format: other
    notes: "Workspace agent skill discovery directory."
env_vars:
  - name: GEMINI_CLI_HOME
    effect: "Overrides the Gemini CLI user home/config directory, useful for wrapper isolation."
  - name: GEMINI_CLI_SYSTEM_DEFAULTS_PATH
    effect: "Overrides the system defaults JSON path."
  - name: GEMINI_CLI_SYSTEM_SETTINGS_PATH
    effect: "Overrides the system settings JSON path."
  - name: GEMINI_CLI_TRUST_WORKSPACE
    effect: "Trusts the current workspace when set true, bypassing folder trust prompts."
  - name: GEMINI_CLI_TRUSTED_FOLDERS_PATH
    effect: "Overrides the trusted-folders state path."
  - name: GEMINI_SANDBOX
    effect: "Enables/selects sandbox execution, such as true, docker, podman, sandbox-exec, runsc, or lxc."
  - name: GEMINI_SANDBOX_IMAGE
    effect: "Overrides the sandbox container image."
  - name: BUILD_SANDBOX
    effect: "Builds a custom sandbox image when used with sandbox mode and a custom sandbox Dockerfile."
  - name: SANDBOX_MOUNTS
    effect: "Adds container sandbox host/container mounts."
  - name: SANDBOX_FLAGS
    effect: "Passes additional flags to Docker or Podman sandbox commands."
  - name: SANDBOX_SET_UID_GID
    effect: "Controls host UID/GID mapping in container sandboxes."
  - name: GEMINI_SYSTEM_MD
    effect: "Enables or points to an external Markdown system prompt override."
  - name: GEMINI_WRITE_SYSTEM_MD
    effect: "Writes the effective system prompt Markdown for inspection/debugging."
  - name: GEMINI_CLI_SURFACE
    effect: "Sets an optional custom surface label for CLI traffic/reporting."
  - name: GEMINI_CLI_IDE_PID
    effect: "Connects the CLI to a specific IDE companion process."
  - name: GEMINI_CLI_IDE_WORKSPACE_PATH
    effect: "Helps IDE integration identify the workspace to connect."
  - name: GEMINI_CLI_IDE_SERVER_PORT
    effect: "Helps IDE integration identify the companion server port."
  - name: NO_COLOR
    effect: "Disables ANSI color output."
  - name: DEBUG
    effect: "Enables debug behavior in supported paths; project `.env` values are intentionally excluded."
  - name: DEBUG_MODE
    effect: "Enables debug behavior in supported paths; project `.env` values are intentionally excluded."
machine_introspection:
  - command: "gemini --list-sessions"
    purpose: other
    machine_readable: false
    output_format: text
    useful_for_codegen: false
    notes: "Lists session indices, dates, message counts, and previews for the current project; useful for wrappers that offer resume UI, but not structured."
  - command: "gemini --list-extensions"
    purpose: plugins
    machine_readable: false
    output_format: text
    useful_for_codegen: false
    notes: "Lists available extensions and exits; no JSON mode was documented."
  - command: "gemini extensions list"
    purpose: plugins
    machine_readable: false
    output_format: text
    useful_for_codegen: false
    notes: "Lists installed extensions; no JSON mode was documented."
  - command: "gemini skills list --all"
    purpose: tools
    machine_readable: false
    output_format: text
    useful_for_codegen: false
    notes: "Lists discovered skills, including built-ins; local output is human text with status and locations."
  - command: "gemini mcp list"
    purpose: mcp
    machine_readable: false
    output_format: text
    useful_for_codegen: false
    notes: "Lists configured MCP servers; no JSON mode was documented."
  - command: "gemini gemma status"
    purpose: doctor
    machine_readable: false
    output_format: text
    useful_for_codegen: false
    notes: "Checks local Gemma routing/LiteRT status; useful diagnostically, not for static provider metadata."
wrapper_notes:
  - "Use `gemini -p/--prompt` for non-interactive runs; a bare positional query defaults to interactive continuation in a TTY."
  - "Headless output can be `text`, `json`, or newline-delimited `stream-json`; stream-json event types include init, message, tool_use, tool_result, error, and result."
  - "JSON/stream-json are model-run output modes, not general config/introspection modes; management commands inspected here emit human text."
  - "Official docs and local 0.46.0 source differ on some flags; wrappers should probe `gemini --version` and tolerate option drift."
  - "Local installed version inspected was 0.46.0, while npm `latest` on 2026-07-02/03 was 0.49.0."
  - "`--approval-mode=yolo`, `--yolo`, MCP `--trust`, and `GEMINI_CLI_TRUST_WORKSPACE=true` materially reduce prompts/approvals and should not be injected silently."
  - "Extension and skill install/link flows may prompt for security consent; use command-specific `--consent` only when the wrapper has explicit user authorization."
  - "Set `GEMINI_CLI_HOME` for wrapper-isolated config/auth state; docs explicitly recommend this for ephemeral/automation setups."
  - "System settings/defaults can be redirected with `GEMINI_CLI_SYSTEM_SETTINGS_PATH` and `GEMINI_CLI_SYSTEM_DEFAULTS_PATH`, which can affect all effective behavior."
  - "On Windows, invoke through normal command resolution (`gemini`) rather than hard-coding POSIX paths; npm commonly exposes `.cmd`/`.ps1` shims."
  - "Exit codes documented for headless execution: 0 success, 1 general/API failure, 42 input error, 53 turn limit exceeded."
  - "The website banner says unpaid tier and Google One users will be replaced by Antigravity CLI on June 18; wrappers should expect future auth/product behavior changes for those user classes."
changes: []
requires_claudine_update: true
reason: "Gemini CLI exposes wrapper-relevant drift beyond older metadata: npm latest is 0.49.0, top-level management commands include extensions/skills/hooks/gemma, headless JSON modes are available, `GEMINI_CLI_HOME` supports config isolation, and current source/docs include ACP/session/policy flags that wrappers should either support or intentionally ignore."
---

## Overview

Gemini CLI is Google's open-source terminal agent. The public command is `gemini`; it defaults to an interactive REPL, supports headless execution with `-p/--prompt`, and exposes management command groups for MCP servers, extensions, skills, hooks, and local Gemma routing.

The latest npm `latest` dist-tag observed during research was `0.49.0`. The locally installed CLI available for inspection was `0.46.0`, so this file records both official documentation and local inspection where they differ.

## Installation and Binaries

Official install paths are npm, npx, Homebrew, MacPorts, Anaconda-with-npm, container execution, and source execution. The npm package declares the `gemini` bin. On Windows, npm installations are expected to expose normal command shims such as `gemini.cmd` and `gemini.ps1`, but official examples use `gemini` consistently.

Official runtime requirements include Node.js 20.0.0+, Bash/Zsh/PowerShell, internet access, and current macOS/Windows/Ubuntu baselines.

## Subcommands

Local `gemini --help` for 0.46.0 showed these top-level command groups:

- `gemini mcp`
- `gemini extensions` / `gemini extension`
- `gemini skills` / `gemini skill`
- `gemini hooks` / `gemini hook`
- `gemini gemma`
- `gemini [query..]`

The official CLI cheatsheet also documents `gemini update`. That command was not shown by the locally installed 0.46.0 help, so wrappers should not assume it exists on every installed version.

## CLI Switch Inventory

The structured frontmatter contains the switch inventory. Important wrapper-facing groups:

- Headless execution: `-p/--prompt`, `-o/--output-format`, `--include-directories`, `--resume`, `--list-sessions`, `--delete-session`.
- Approval and execution posture: `--approval-mode`, `--yolo`, `--sandbox`, `--skip-trust`, `--allowed-tools`, `--policy`, `--admin-policy`.
- Provider/runtime selection: `--model`, `--extensions`, `--allowed-mcp-server-names`.
- Management command flags: MCP `--scope`, `--transport`, `--env`, `--header`, `--timeout`, `--trust`, `--include-tools`, `--exclude-tools`; extension/skill `--scope`, `--consent`, `--all`, `--ref`, `--auto-update`, `--pre-release`, `--path`; Gemma setup `--port`, `--skip-model`, `--start`, `--force`, `--consent`.

The main caveat is version drift: bundled docs list `--experimental-zed-integration`, while local 0.46.0 source exposes newer `--acp`, `--policy`, `--admin-policy`, `--session-file`, and `--session-id` flags that are not all present in the bundled CLI reference table.

## Configuration Discovery

Gemini CLI uses JSON settings files with this precedence: defaults, system defaults, user settings, project settings, system settings, environment variables, then CLI arguments.

Primary settings files:

- System defaults: `/etc/gemini-cli/system-defaults.json`, `C:\ProgramData\gemini-cli\system-defaults.json`, or `/Library/Application Support/GeminiCli/system-defaults.json`.
- User settings: `~/.gemini/settings.json`.
- Project settings: `.gemini/settings.json`.
- System override settings: `/etc/gemini-cli/settings.json`, `C:\ProgramData\gemini-cli\settings.json`, or `/Library/Application Support/GeminiCli/settings.json`.

Context/memory is supplied by `GEMINI.md` files by default, with the filename configurable through `context.fileName`. Project file discovery also respects `.geminiignore` when enabled.

## Environment Variables

The frontmatter records general wrapper-relevant variables. It intentionally omits authentication/model endpoint variables and telemetry-specific variables except for `GEMINI_CLI_SURFACE`, which can affect wrapper attribution. For isolated wrapper runs, `GEMINI_CLI_HOME` is the most important general variable because it redirects the CLI home/config directory.

## Machine Introspection

Gemini CLI exposes useful state-listing commands, but current public docs and local inspection did not prove JSON output for those management commands. Claudine can still call these for reports or diagnostics, but parsers should treat them as human text unless a future version adds structured output.

The one structured output surface is headless model execution through `--output-format json` or `--output-format stream-json`; that is run output, not provider-state introspection.

## Wrapper Notes

For non-interactive wrappers, prefer `gemini -p "<prompt>" --output-format stream-json` when streaming events are needed, or `--output-format json` when a single response object is enough. Avoid bare positional prompts for automation in a TTY because the documented behavior is interactive continuation.

Do not silently add `--yolo`, `--approval-mode=yolo`, MCP `--trust`, or trust-bypass environment variables. These change confirmation and workspace-trust posture. Install/link flows for extensions and skills may also require consent; wrappers should use `--consent` only after explicit user authorization.

## Sources

- [Gemini CLI homepage](https://geminicli.com/)
- [Gemini CLI GitHub repository](https://github.com/google-gemini/gemini-cli)
- [Gemini CLI installation, execution, and releases](https://geminicli.com/docs/get-started/installation/)
- [Gemini CLI cheatsheet / CLI reference](https://geminicli.com/docs/cli/cli-reference/)
- [Gemini CLI configuration reference](https://geminicli.com/docs/reference/configuration/)
- [Gemini CLI headless mode](https://geminicli.com/docs/cli/headless/)
- [Gemini CLI Agent Skills](https://geminicli.com/docs/cli/skills/)
- [@google/gemini-cli npm package](https://www.npmjs.com/package/@google/gemini-cli)
- Local inspection on 2026-07-02/03: `gemini --version` returned `0.46.0`; `npm view @google/gemini-cli version --json` returned `0.49.0`; `npm view @google/gemini-cli bin --json` returned `{"gemini":"bundle/gemini.js"}`.
