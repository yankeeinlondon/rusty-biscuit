---
$schema: ./_schema.yaml
created: 2026-07-02
last_updated: 2026-07-03
agent: codex
model: default
latest_version: "0.80.3"
homepage: https://pi.dev/
repo: https://github.com/earendil-works/pi
docs: https://pi.dev/docs/latest
cli_docs: https://github.com/earendil-works/pi/blob/main/packages/coding-agent/docs/usage.md#cli-reference
binaries:
  - os: macos
    binary: pi
    alt_binaries: []
    notes: "The npm package exposes `bin.pi` as `dist/cli.js`; local macOS host currently has `/Users/ken/.bun/bin/pi`, but it points to the older `@mariozechner/pi-coding-agent@0.73.1` namespace."
  - os: linux
    binary: pi
    alt_binaries: []
    notes: "The current official npm package exposes the same `pi` bin on Linux."
  - os: windows
    binary: pi.cmd
    alt_binaries: ["pi.ps1", "pi"]
    notes: "The package bin is still named `pi`; npm-compatible Windows global installs normally create command and PowerShell shims."
install_methods:
  - os: macos
    method: npm
    command: "npm install -g --ignore-scripts @earendil-works/pi-coding-agent"
    notes: "Primary documented install."
  - os: linux
    method: npm
    command: "npm install -g --ignore-scripts @earendil-works/pi-coding-agent"
    notes: "Primary documented install."
  - os: windows
    method: npm
    command: "npm install -g --ignore-scripts @earendil-works/pi-coding-agent"
    notes: "Primary documented install."
  - os: macos
    method: standalone_binary
    command: "curl -fsSL https://pi.dev/install.sh | sh"
    notes: "Official homepage installer alternative; quickstart says curl installs are removed with npm."
  - os: linux
    method: standalone_binary
    command: "curl -fsSL https://pi.dev/install.sh | sh"
    notes: "Official homepage installer alternative; quickstart says curl installs are removed with npm."
  - os: windows
    method: other
    command: "powershell -c \"irm https://pi.dev/install.ps1 | iex\""
    notes: "Official homepage PowerShell installer."
  - os: macos
    method: other
    command: "pnpm add -g --ignore-scripts @earendil-works/pi-coding-agent"
    notes: "Official homepage also documents pnpm global install."
  - os: linux
    method: other
    command: "pnpm add -g --ignore-scripts @earendil-works/pi-coding-agent"
    notes: "Official homepage also documents pnpm global install."
  - os: windows
    method: other
    command: "pnpm add -g --ignore-scripts @earendil-works/pi-coding-agent"
    notes: "Official homepage also documents pnpm global install."
  - os: macos
    method: other
    command: "bun add -g --ignore-scripts @earendil-works/pi-coding-agent"
    notes: "Official homepage also documents Bun global install."
  - os: linux
    method: other
    command: "bun add -g --ignore-scripts @earendil-works/pi-coding-agent"
    notes: "Official homepage also documents Bun global install."
  - os: windows
    method: other
    command: "bun add -g --ignore-scripts @earendil-works/pi-coding-agent"
    notes: "Official homepage also documents Bun global install."
subcommands:
  - name: "(interactive)"
    description: "Starts the terminal coding agent when no package command or non-interactive mode is supplied."
    non_interactive: false
    notes: "Usage is `pi [options] [@files...] [messages...]`; optional messages seed the first prompt."
  - name: "(print mode)"
    description: "Processes a prompt or stdin non-interactively and exits."
    non_interactive: true
    notes: "Activated by `--print` or `-p`; supports text output by default and JSON event output when combined with `--mode json`."
  - name: "(JSON event mode)"
    description: "Runs a prompt and streams session events as JSON Lines to stdout."
    non_interactive: true
    notes: "Activated with `--mode json`; docs describe a session header followed by agent, message, turn, tool, queue, compaction, and retry events."
  - name: "(RPC mode)"
    description: "Starts a headless JSONL RPC process over stdin/stdout."
    non_interactive: true
    notes: "Activated with `--mode rpc`; docs require strict LF-delimited JSONL framing."
  - name: install
    description: "Installs an extension/package source and adds it to settings."
    non_interactive: false
    notes: "Accepts npm, git, HTTPS, SSH, and local paths; may run package-manager commands and mutate user or project settings."
  - name: remove
    description: "Removes an extension/package source from settings."
    non_interactive: false
    notes: "Supports project-local removal with `--local`."
  - name: uninstall
    description: "Alias for `remove`."
    non_interactive: false
    notes: "Removes package source configuration; not the same as uninstalling Pi itself with npm/pnpm/yarn/bun."
  - name: update
    description: "Updates Pi itself, installed packages, or one package source."
    non_interactive: false
    notes: "`pi update` defaults to Pi itself; `--all` updates Pi and packages. It can invoke package managers and network operations."
  - name: list
    description: "Lists installed packages from user and project settings."
    non_interactive: true
    notes: "Local inspection showed human-readable text output only."
  - name: config
    description: "Opens a TUI to enable or disable package resources."
    non_interactive: false
    notes: "`pi config --help` entered the TUI in both local 0.73.1 and unpacked 0.80.3 inspection; wrappers should treat it as interactive."
cli_switches:
  - flag: --provider
    value: "<name>"
    scope: ["global", "model_selection"]
    default: "google"
    description: "Selects the provider name."
    example: "pi --provider openai --model gpt-4o-mini"
    notes: "Can be bypassed by using `--model provider/model`."
  - flag: --model
    value: "<pattern>"
    scope: ["global", "model_selection"]
    default: "settings or provider default"
    description: "Selects a model pattern or id, including `provider/id` and optional `:<thinking>` shorthand."
    example: "pi --model openai/gpt-4o"
    notes: ""
  - flag: --api-key
    value: "<key>"
    scope: ["global", "auth"]
    default: "environment variables or auth.json"
    description: "Supplies a runtime API key."
    example: "pi --model openai/gpt-4o --api-key sk-..."
    notes: "Requires a model selection so Pi can associate the key with a provider."
  - flag: --system-prompt
    value: "<text>"
    scope: ["global", "prompt"]
    default: "coding assistant prompt"
    description: "Replaces the system prompt."
    example: "pi --system-prompt 'You are concise.'"
    notes: "System-prompt semantics are owned by the sibling `system-prompt` topic."
  - flag: --append-system-prompt
    value: "<text-or-file>"
    scope: ["global", "prompt"]
    default: "[]"
    description: "Appends text or file contents to the system prompt."
    example: "pi --append-system-prompt .pi/extra-system.md"
    notes: "Repeatable. System-prompt semantics are owned by the sibling `system-prompt` topic."
  - flag: --mode
    value: "text | json | rpc"
    scope: ["global", "output", "automation"]
    default: "text"
    description: "Selects text output, JSON event stream output, or JSONL RPC mode."
    example: "pi --mode json -p 'Summarize this repo'"
    notes: "`--mode rpc` cannot accept `@file` arguments according to RPC docs."
  - flag: --print
    value: ""
    scope: ["global", "automation"]
    default: "false"
    description: "Runs non-interactively, processes a prompt, and exits."
    example: "pi -p 'Summarize this codebase'"
    notes: "Short form: `-p`; parser also accepts the next non-flag token as the prompt."
  - flag: --continue
    value: ""
    scope: ["global", "sessions"]
    default: "false"
    description: "Continues the previous session."
    example: "pi --continue 'What did we discuss?'"
    notes: "Short form: `-c`."
  - flag: --resume
    value: ""
    scope: ["global", "sessions"]
    default: "false"
    description: "Selects a session to resume."
    example: "pi --resume"
    notes: "Short form: `-r`; interactive session picker."
  - flag: --session
    value: "<path|id>"
    scope: ["global", "sessions"]
    default: ""
    description: "Uses a specific session file or partial UUID."
    example: "pi --session ~/.pi/agent/sessions/project/session.jsonl"
    notes: ""
  - flag: --session-id
    value: "<id>"
    scope: ["global", "sessions"]
    default: ""
    description: "Uses an exact project session id, creating it if missing."
    example: "pi --session-id my-ci-run --no-session"
    notes: "Present in parser/help for 0.80.3; not listed in the docs CLI table."
  - flag: --fork
    value: "<path|id>"
    scope: ["global", "sessions"]
    default: ""
    description: "Forks a specific session file or partial UUID into a new session."
    example: "pi --fork old-session-id"
    notes: ""
  - flag: --session-dir
    value: "<dir>"
    scope: ["global", "sessions"]
    default: "~/.pi/agent/sessions or settings"
    description: "Sets the directory for session storage and lookup."
    example: "pi --session-dir .pi/sessions"
    notes: "Precedence is CLI flag, then `PI_CODING_AGENT_SESSION_DIR`, then `sessionDir` in settings."
  - flag: --no-session
    value: ""
    scope: ["global", "sessions"]
    default: "false"
    description: "Disables session persistence for an ephemeral run."
    example: "pi --no-session -p 'Review this diff'"
    notes: ""
  - flag: --name
    value: "<name>"
    scope: ["global", "sessions"]
    default: ""
    description: "Sets the session display name."
    example: "pi --name 'Refactor auth module'"
    notes: "Short form: `-n`."
  - flag: --models
    value: "<patterns>"
    scope: ["global", "model_selection"]
    default: "settings enabledModels"
    description: "Sets comma-separated model patterns for Ctrl+P cycling."
    example: "pi --models claude-sonnet,claude-haiku,gpt-4o"
    notes: "Supports globs and fuzzy matching."
  - flag: --no-tools
    value: ""
    scope: ["global", "tools"]
    default: "false"
    description: "Disables all built-in, extension, and custom tools by default."
    example: "pi --no-tools -p 'Think through this API design'"
    notes: "Short form: `-nt`."
  - flag: --no-builtin-tools
    value: ""
    scope: ["global", "tools"]
    default: "false"
    description: "Disables built-in tools but keeps extension and custom tools enabled."
    example: "pi --no-builtin-tools"
    notes: "Short form: `-nbt`."
  - flag: --tools
    value: "<tools>"
    scope: ["global", "tools"]
    default: "built-in defaults"
    description: "Enables a comma-separated allowlist of tool names."
    example: "pi --tools read,grep,find,ls -p 'Review the code in src/'"
    notes: "Short form: `-t`; applies to built-in, extension, and custom tools."
  - flag: --exclude-tools
    value: "<tools>"
    scope: ["global", "tools"]
    default: "[]"
    description: "Disables a comma-separated denylist of tool names."
    example: "pi --exclude-tools ask_question"
    notes: "Short form: `-xt`; applies to built-in, extension, and custom tools."
  - flag: --thinking
    value: "off | minimal | low | medium | high | xhigh"
    scope: ["global", "model_behavior"]
    default: "settings or model default"
    description: "Sets the reasoning/thinking level."
    example: "pi --thinking high 'Solve this complex problem'"
    notes: "`xhigh` is documented in RPC docs as only supported by OpenAI codex-max models."
  - flag: --extension
    value: "<path|source>"
    scope: ["global", "resources"]
    default: "[]"
    description: "Loads an extension file or package source."
    example: "pi --extension ./my-extension.ts"
    notes: "Short form: `-e`; repeatable. Package docs also show npm/git sources for temporary extension loading."
  - flag: --no-extensions
    value: ""
    scope: ["global", "resources"]
    default: "false"
    description: "Disables extension discovery while still allowing explicit `--extension` paths."
    example: "pi --no-extensions"
    notes: "Short form: `-ne`."
  - flag: --skill
    value: "<path>"
    scope: ["global", "resources"]
    default: "[]"
    description: "Loads a skill file or directory."
    example: "pi --skill ./skills/reviewer.md"
    notes: "Repeatable."
  - flag: --no-skills
    value: ""
    scope: ["global", "resources"]
    default: "false"
    description: "Disables skill discovery and loading."
    example: "pi --no-skills"
    notes: "Short form: `-ns`."
  - flag: --prompt-template
    value: "<path>"
    scope: ["global", "resources"]
    default: "[]"
    description: "Loads a prompt template file or directory."
    example: "pi --prompt-template ./prompts"
    notes: "Repeatable."
  - flag: --no-prompt-templates
    value: ""
    scope: ["global", "resources"]
    default: "false"
    description: "Disables prompt template discovery and loading."
    example: "pi --no-prompt-templates"
    notes: "Short form: `-np`."
  - flag: --theme
    value: "<path>"
    scope: ["global", "resources", "ui"]
    default: "settings theme"
    description: "Loads a theme file or directory."
    example: "pi --theme ./theme.json"
    notes: "Repeatable."
  - flag: --no-themes
    value: ""
    scope: ["global", "resources", "ui"]
    default: "false"
    description: "Disables theme discovery and loading."
    example: "pi --no-themes"
    notes: ""
  - flag: --no-context-files
    value: ""
    scope: ["global", "context"]
    default: "false"
    description: "Disables AGENTS.md and CLAUDE.md discovery and loading."
    example: "pi --no-context-files"
    notes: "Short form: `-nc`."
  - flag: --export
    value: "<file>"
    scope: ["global", "sessions"]
    default: ""
    description: "Exports a session file to HTML and exits."
    example: "pi --export ~/.pi/agent/sessions/project/session.jsonl"
    notes: "Docs examples also show `pi --export session.jsonl output.html`; parser inspection records only the flagged input value and treats the trailing output path as a message argument."
  - flag: --list-models
    value: "[search]"
    scope: ["global", "models", "introspection"]
    default: "false"
    description: "Lists available models, optionally filtered by a fuzzy search."
    example: "pi --list-models sonnet"
    notes: "Human-readable table; 0.80.3 did not expose a JSON variant."
  - flag: --verbose
    value: ""
    scope: ["global", "diagnostics"]
    default: "false"
    description: "Forces verbose startup, overriding the `quietStartup` setting."
    example: "pi --verbose"
    notes: ""
  - flag: --approve
    value: ""
    scope: ["global", "project_trust", "install", "remove", "list", "update"]
    default: "settings defaultProjectTrust"
    description: "Trusts project-local files for this run or package command."
    example: "pi --approve -p 'Use project-local extensions'"
    notes: "Short form: `-a`."
  - flag: --no-approve
    value: ""
    scope: ["global", "project_trust", "install", "remove", "list", "update"]
    default: "settings defaultProjectTrust"
    description: "Ignores project-local files for this run or package command."
    example: "pi --no-approve -p 'Ignore repo-local Pi settings'"
    notes: "Short form: `-na`."
  - flag: --offline
    value: ""
    scope: ["global", "network"]
    default: "false"
    description: "Disables startup network operations."
    example: "pi --offline --list-models"
    notes: "Equivalent to `PI_OFFLINE=1`; source inspection also sets `PI_SKIP_VERSION_CHECK=1` internally."
  - flag: --help
    value: ""
    scope: ["global", "meta"]
    default: "false"
    description: "Shows help."
    example: "pi --help"
    notes: "Short form: `-h`."
  - flag: --version
    value: ""
    scope: ["global", "meta"]
    default: "false"
    description: "Shows the version number."
    example: "pi --version"
    notes: "Short form: `-v`."
  - flag: --local
    value: ""
    scope: ["install", "remove", "uninstall", "packages"]
    default: "false"
    description: "Installs or removes a package in project-local `.pi/settings.json`."
    example: "pi install npm:@foo/bar --local"
    notes: "Short form: `-l`."
  - flag: --self
    value: ""
    scope: ["update", "packages"]
    default: "false"
    description: "Updates Pi only."
    example: "pi update --self"
    notes: "Default when no update target is supplied."
  - flag: --extensions
    value: ""
    scope: ["update", "packages"]
    default: "false"
    description: "Updates installed Pi packages only."
    example: "pi update --extensions"
    notes: ""
  - flag: --all
    value: ""
    scope: ["update", "packages"]
    default: "false"
    description: "Updates Pi and installed packages."
    example: "pi update --all"
    notes: "Added relative to the older local 0.73.1 `pi update` help."
  - flag: --extension
    value: "<source>"
    scope: ["update", "packages"]
    default: ""
    description: "Updates one package source."
    example: "pi update --extension npm:@foo/bar"
    notes: "This update-specific use shares the same flag spelling as global extension loading but has package-source semantics."
  - flag: --force
    value: ""
    scope: ["update", "packages"]
    default: "false"
    description: "Reinstalls Pi even if the current version is latest."
    example: "pi update --self --force"
    notes: ""
config_paths:
  - os: macos
    scope: env
    path: "$PI_CODING_AGENT_DIR"
    format: other
    notes: "Overrides the agent config directory; default is `/Users/<user>/.pi/agent`."
  - os: linux
    scope: env
    path: "$PI_CODING_AGENT_DIR"
    format: other
    notes: "Overrides the agent config directory; default is `/home/<user>/.pi/agent`."
  - os: windows
    scope: env
    path: "%PI_CODING_AGENT_DIR%"
    format: other
    notes: "Overrides the agent config directory; default is `%USERPROFILE%\\.pi\\agent`."
  - os: macos
    scope: user
    path: "~/.pi/agent/settings.json"
    format: json
    notes: "Global settings file; paths inside it resolve relative to `~/.pi/agent`."
  - os: linux
    scope: user
    path: "~/.pi/agent/settings.json"
    format: json
    notes: "Global settings file; paths inside it resolve relative to `~/.pi/agent`."
  - os: windows
    scope: user
    path: "%USERPROFILE%\\.pi\\agent\\settings.json"
    format: json
    notes: "Global settings file; paths inside it resolve relative to `%USERPROFILE%\\.pi\\agent`."
  - os: macos
    scope: repo
    path: ".pi/settings.json"
    format: json
    notes: "Project settings override global settings; paths inside it resolve relative to `.pi`."
  - os: linux
    scope: repo
    path: ".pi/settings.json"
    format: json
    notes: "Project settings override global settings; paths inside it resolve relative to `.pi`."
  - os: windows
    scope: repo
    path: ".pi\\settings.json"
    format: json
    notes: "Project settings override global settings; paths inside it resolve relative to `.pi`."
  - os: macos
    scope: user
    path: "~/.pi/agent/auth.json"
    format: json
    notes: "Stores API keys or OAuth credentials when configured through `/login`; API key env vars take precedence."
  - os: linux
    scope: user
    path: "~/.pi/agent/auth.json"
    format: json
    notes: "Stores API keys or OAuth credentials when configured through `/login`; API key env vars take precedence."
  - os: windows
    scope: user
    path: "%USERPROFILE%\\.pi\\agent\\auth.json"
    format: json
    notes: "Stores API keys or OAuth credentials when configured through `/login`; API key env vars take precedence."
  - os: macos
    scope: user
    path: "~/.pi/agent/models.json"
    format: json
    notes: "Custom provider/model definitions for supported API standards."
  - os: linux
    scope: user
    path: "~/.pi/agent/models.json"
    format: json
    notes: "Custom provider/model definitions for supported API standards."
  - os: windows
    scope: user
    path: "%USERPROFILE%\\.pi\\agent\\models.json"
    format: json
    notes: "Custom provider/model definitions for supported API standards."
  - os: macos
    scope: user
    path: "~/.pi/agent/trust.json"
    format: json
    notes: "Stores project trust decisions; non-interactive modes do not prompt and consult trust/defaultProjectTrust instead."
  - os: linux
    scope: user
    path: "~/.pi/agent/trust.json"
    format: json
    notes: "Stores project trust decisions; non-interactive modes do not prompt and consult trust/defaultProjectTrust instead."
  - os: windows
    scope: user
    path: "%USERPROFILE%\\.pi\\agent\\trust.json"
    format: json
    notes: "Stores project trust decisions; non-interactive modes do not prompt and consult trust/defaultProjectTrust instead."
  - os: macos
    scope: user
    path: "~/.pi/agent/AGENTS.md"
    format: text
    notes: "Global context instructions loaded at startup unless context files are disabled."
  - os: linux
    scope: user
    path: "~/.pi/agent/AGENTS.md"
    format: text
    notes: "Global context instructions loaded at startup unless context files are disabled."
  - os: windows
    scope: user
    path: "%USERPROFILE%\\.pi\\agent\\AGENTS.md"
    format: text
    notes: "Global context instructions loaded at startup unless context files are disabled."
  - os: macos
    scope: repo
    path: "AGENTS.md"
    format: text
    notes: "Project context instructions discovered from parent directories and cwd unless `--no-context-files` is set."
  - os: linux
    scope: repo
    path: "AGENTS.md"
    format: text
    notes: "Project context instructions discovered from parent directories and cwd unless `--no-context-files` is set."
  - os: windows
    scope: repo
    path: "AGENTS.md"
    format: text
    notes: "Project context instructions discovered from parent directories and cwd unless `--no-context-files` is set."
  - os: macos
    scope: repo
    path: "CLAUDE.md"
    format: text
    notes: "Alternative project context instructions discovered from parent directories and cwd unless `--no-context-files` is set."
  - os: linux
    scope: repo
    path: "CLAUDE.md"
    format: text
    notes: "Alternative project context instructions discovered from parent directories and cwd unless `--no-context-files` is set."
  - os: windows
    scope: repo
    path: "CLAUDE.md"
    format: text
    notes: "Alternative project context instructions discovered from parent directories and cwd unless `--no-context-files` is set."
  - os: macos
    scope: user
    path: "~/.pi/agent/SYSTEM.md"
    format: text
    notes: "Global replacement system prompt; semantics deferred to the sibling `system-prompt` topic."
  - os: linux
    scope: user
    path: "~/.pi/agent/SYSTEM.md"
    format: text
    notes: "Global replacement system prompt; semantics deferred to the sibling `system-prompt` topic."
  - os: windows
    scope: user
    path: "%USERPROFILE%\\.pi\\agent\\SYSTEM.md"
    format: text
    notes: "Global replacement system prompt; semantics deferred to the sibling `system-prompt` topic."
  - os: macos
    scope: repo
    path: ".pi/SYSTEM.md"
    format: text
    notes: "Project replacement system prompt; semantics deferred to the sibling `system-prompt` topic."
  - os: linux
    scope: repo
    path: ".pi/SYSTEM.md"
    format: text
    notes: "Project replacement system prompt; semantics deferred to the sibling `system-prompt` topic."
  - os: windows
    scope: repo
    path: ".pi\\SYSTEM.md"
    format: text
    notes: "Project replacement system prompt; semantics deferred to the sibling `system-prompt` topic."
  - os: macos
    scope: user
    path: "~/.pi/agent/APPEND_SYSTEM.md"
    format: text
    notes: "Global appended system prompt; semantics deferred to the sibling `system-prompt` topic."
  - os: linux
    scope: user
    path: "~/.pi/agent/APPEND_SYSTEM.md"
    format: text
    notes: "Global appended system prompt; semantics deferred to the sibling `system-prompt` topic."
  - os: windows
    scope: user
    path: "%USERPROFILE%\\.pi\\agent\\APPEND_SYSTEM.md"
    format: text
    notes: "Global appended system prompt; semantics deferred to the sibling `system-prompt` topic."
  - os: macos
    scope: repo
    path: ".pi/APPEND_SYSTEM.md"
    format: text
    notes: "Project appended system prompt; semantics deferred to the sibling `system-prompt` topic."
  - os: linux
    scope: repo
    path: ".pi/APPEND_SYSTEM.md"
    format: text
    notes: "Project appended system prompt; semantics deferred to the sibling `system-prompt` topic."
  - os: windows
    scope: repo
    path: ".pi\\APPEND_SYSTEM.md"
    format: text
    notes: "Project appended system prompt; semantics deferred to the sibling `system-prompt` topic."
  - os: macos
    scope: user
    path: "~/.pi/agent/keybindings.json"
    format: json
    notes: "Interactive keybinding customization file."
  - os: linux
    scope: user
    path: "~/.pi/agent/keybindings.json"
    format: json
    notes: "Interactive keybinding customization file."
  - os: windows
    scope: user
    path: "%USERPROFILE%\\.pi\\agent\\keybindings.json"
    format: json
    notes: "Interactive keybinding customization file."
env_vars:
  - name: PI_CODING_AGENT_DIR
    effect: "Overrides the Pi agent config directory; default is `~/.pi/agent`."
  - name: PI_CODING_AGENT_SESSION_DIR
    effect: "Overrides session storage directory unless `--session-dir` is supplied."
  - name: PI_PACKAGE_DIR
    effect: "Overrides the package directory, mainly for immutable store paths such as Nix or Guix."
  - name: PI_OFFLINE
    effect: "When set to 1/true/yes, disables startup network operations including update checks, package update checks, and install/update telemetry."
  - name: PI_SKIP_VERSION_CHECK
    effect: "Disables the Pi version update check and prevents the `pi.dev` latest-version request."
  - name: PI_TELEMETRY
    effect: "Overrides install/update telemetry and provider attribution headers with 1/true/yes or 0/false/no; it does not disable update checks."
  - name: PI_CACHE_RETENTION
    effect: "Set to `long` for extended prompt cache where supported, such as Anthropic 1h or OpenAI 24h."
  - name: PI_SHARE_VIEWER_URL
    effect: "Sets the base URL for the `/share` command; default is `https://pi.dev/session/`."
  - name: PI_STARTUP_BENCHMARK
    effect: "Source inspection shows this enables startup benchmarking only in interactive mode; it exits with an error in non-interactive modes."
  - name: PI_HARDWARE_CURSOR
    effect: "Makes the TUI show the terminal hardware cursor, useful for some IME/terminal combinations."
  - name: PI_TUI_WRITE_LOG
    effect: "Captures raw TUI ANSI output to the configured log path for TUI diagnostics."
  - name: VISUAL
    effect: "Used as the external editor fallback before `EDITOR` when `externalEditor` is not set."
  - name: EDITOR
    effect: "Used as the external editor fallback after `VISUAL` when `externalEditor` is not set."
machine_introspection:
  - command: "pi --list-models [search]"
    purpose: models
    machine_readable: false
    output_format: table
    useful_for_codegen: false
    notes: "Lists available models with provider, model, context, max output, thinking, and image columns. Local inspection found no JSON flag, so wrappers should prefer RPC for machine parsing."
  - command: "printf '%s\n' '{\"id\":\"models\",\"type\":\"get_available_models\"}' | pi --mode rpc --no-session"
    purpose: models
    machine_readable: true
    output_format: jsonl
    useful_for_codegen: true
    notes: "RPC docs define `get_available_models` returning full Model objects. Requires spawning Pi in RPC mode and sending LF-delimited JSON."
  - command: "printf '%s\n' '{\"id\":\"state\",\"type\":\"get_state\"}' | pi --mode rpc --no-session"
    purpose: env
    machine_readable: true
    output_format: jsonl
    useful_for_codegen: false
    notes: "RPC docs define `get_state` for current model, thinking level, streaming state, session file/id/name, compaction, message counts, and queue modes."
  - command: "printf '%s\n' '{\"id\":\"messages\",\"type\":\"get_messages\"}' | pi --mode rpc --no-session"
    purpose: other
    machine_readable: true
    output_format: jsonl
    useful_for_codegen: false
    notes: "RPC docs define `get_messages` for full conversation messages; useful for wrapper diagnostics, not static metadata."
  - command: "pi list"
    purpose: plugins
    machine_readable: false
    output_format: text
    useful_for_codegen: false
    notes: "Lists installed Pi packages from user and project settings. Local inspection returned plain text (`No packages installed.`) with no JSON switch."
wrapper_notes:
  - "Use `pi -p` for one-shot non-interactive text runs; use `--mode json` for JSONL session events; use `--mode rpc` for bidirectional JSONL integration."
  - "The host `pi` binary is stale: `/Users/ken/.bun/bin/pi` points to `@mariozechner/pi-coding-agent@0.73.1`, while the current official package is `@earendil-works/pi-coding-agent@0.80.3`."
  - "The 0.80.3 top-level `--help` output truncates in non-TTY capture after the `pi config` command line; the parser source and docs include the full flag inventory."
  - "JSON event mode writes machine-readable JSON Lines to stdout; docs recommend redirecting stderr when piping to tools such as jq."
  - "RPC mode uses strict LF-delimited JSONL over stdin/stdout; clients must not use line readers that split on Unicode line separators."
  - "`pi config` is a TUI command. Local 0.73.1 and unpacked 0.80.3 inspection showed `pi config --help` entering the TUI instead of printing command help."
  - "Non-interactive modes do not prompt for project trust. Without saved trust, `defaultProjectTrust: ask` behaves like ignoring project resources; use `--approve` or `--no-approve` for deterministic wrapper behavior."
  - "Project-local settings, extensions, package resources, and `.agents/skills` can execute or influence behavior after trust; wrappers should set trust flags and consider `--no-extensions`, `--no-skills`, `--no-prompt-templates`, and `--no-context-files` when isolation matters."
  - "Set `PI_OFFLINE=1` or pass `--offline` to avoid startup network operations such as version checks, package update checks, and install/update telemetry."
  - "Windows tool execution requires a bash shell; docs say Pi checks custom `shellPath`, Git Bash, then `bash.exe` on PATH."
  - "No MCP support is built in according to the README philosophy section; MCP-like behavior requires an extension such as a package bridge."
  - "Provider API-key environment variables are numerous and model-config-owned; this document records only Pi-owned general runtime env vars."
changes:
  - "Verified current upstream npm package `@earendil-works/pi-coding-agent` remains at 0.80.3 while the host-installed `pi` is the stale `@mariozechner/pi-coding-agent@0.73.1` shim."
  - "Replaced schema-invalid `os: all` frontmatter entries with separate macOS, Linux, and Windows records."
  - "Recorded 0.80.3 `--all` update flag and package-command `--approve`/`--no-approve` support."
  - "Added Pi-owned runtime variables present in current docs/source: `PI_CACHE_RETENTION`, `PI_HARDWARE_CURSOR`, and `PI_TUI_WRITE_LOG`."
  - "Documented that current top-level help capture truncates mid-command, so parser source and official docs were used for full switch inventory."
requires_claudine_update: true
reason: "Pi is researched but not yet represented in Claudine's compiled provider enum or wrapper metadata; current research also shows stale old-namespace installs may need detection/migration handling."
---

# Pi Agent CLI Surface

## Overview

Pi is a minimal, extensible terminal coding harness from Earendil Works. The public repository is `earendil-works/pi`, and the current CLI package is `@earendil-works/pi-coding-agent`. The package exposes a primary `pi` command and supports four wrapper-relevant surfaces: interactive TUI, print mode, JSON event stream mode, and JSONL RPC mode.

The current upstream version verified on 2026-07-03 is `0.80.3`. I verified that with `npm view @earendil-works/pi-coding-agent version dist-tags bin repository homepage --json`, by unpacking `@earendil-works/pi-coding-agent@0.80.3` with `npm pack`, and by running `node dist/cli.js --version` from the unpacked package after dependency installation with lifecycle scripts disabled. The host-installed `pi --version` reports `0.73.1`; that local binary is `/Users/ken/.bun/bin/pi`, a Bun global shim pointing at the old `@mariozechner/pi-coding-agent` package namespace, so it is useful compatibility evidence but not the latest public release.

Primary URLs:

- Homepage: [https://pi.dev/](https://pi.dev/)
- Repository: [https://github.com/earendil-works/pi](https://github.com/earendil-works/pi)
- General docs: [https://pi.dev/docs/latest](https://pi.dev/docs/latest)
- CLI reference: [packages/coding-agent/docs/usage.md#cli-reference](https://github.com/earendil-works/pi/blob/main/packages/coding-agent/docs/usage.md#cli-reference)

## Installation and Binaries

The current official npm package exposes one bin, `pi`, mapped to `dist/cli.js`. On macOS and Linux the command name is `pi`. On Windows, npm-compatible global installs normally expose `pi.cmd` and `pi.ps1` shims for the same package bin; upstream does not document a separate Windows executable name.

The primary documented install is:

```bash
npm install -g --ignore-scripts @earendil-works/pi-coding-agent
```

The docs recommend `--ignore-scripts` because Pi does not require dependency lifecycle scripts for normal npm installs. The homepage also documents:

```bash
curl -fsSL https://pi.dev/install.sh | sh
powershell -c "irm https://pi.dev/install.ps1 | iex"
pnpm add -g --ignore-scripts @earendil-works/pi-coding-agent
bun add -g --ignore-scripts @earendil-works/pi-coding-agent
```

The quickstart says the curl installer uses npm globally, so curl-installed Pi is uninstalled with npm. Uninstalling Pi leaves settings, credentials, sessions, and installed Pi packages under `~/.pi/agent`.

Local host evidence:

- `command -v pi` resolved to `/Users/ken/.bun/bin/pi`.
- That symlink points to `../install/global/node_modules/@mariozechner/pi-coding-agent/dist/cli.js`.
- `pi --version` returned `0.73.1`.
- `npm view @mariozechner/pi-coding-agent version` returned `0.73.1`, while `npm view @earendil-works/pi-coding-agent version` returned `0.80.3`.

## Subcommands

Pi is mostly mode-driven rather than subcommand-driven.

| Command or mode | Description | Automation suitability |
| --- | --- | --- |
| `pi` | Starts the interactive terminal coding agent. | Requires TTY and user interaction. |
| `pi -p`, `pi --print` | Processes a prompt or piped stdin non-interactively and exits. | Primary one-shot automation entry point. |
| `pi --mode json` | Streams session events as JSON Lines. | Automation entry point when paired with an initial prompt or `-p`. |
| `pi --mode rpc` | Starts a JSONL RPC process over stdin/stdout. | Primary bidirectional process-integration entry point. |
| `pi install <source>` | Installs an extension/package source and writes settings. | Can run package-manager commands and mutate config; treat as user-initiated. |
| `pi remove <source>` | Removes an extension/package source from settings. | Non-TTY-capable in principle, but mutates config. |
| `pi uninstall <source>` | Alias for `remove`; not a Pi self-uninstaller. | Non-TTY-capable in principle, but mutates config. |
| `pi update [source\|self\|pi]` | Updates Pi itself, installed packages, or one package source. | Can perform network/package-manager work; `pi update` never prompts for project trust. |
| `pi list` | Lists installed packages from user and project settings. | Non-interactive, but text only. |
| `pi config` | Opens a TUI to enable or disable package resources. | Requires TTY. Local `pi config --help` entered the TUI. |

Interactive slash commands exist inside the TUI, including `/login`, `/logout`, `/model`, `/settings`, `/resume`, `/new`, `/session`, `/tree`, `/trust`, `/fork`, `/clone`, `/compact`, `/copy`, `/export`, `/import`, `/share`, `/reload`, `/hotkeys`, `/changelog`, and `/quit`. They are not top-level process subcommands.

## CLI Switch Inventory

The frontmatter `cli_switches` array contains the full built-in switch inventory observed from `dist/cli/args.js`, official docs, and subcommand help for `@earendil-works/pi-coding-agent@0.80.3`. The host-installed 0.73.1 binary was also probed for compatibility.

Important grouping notes:

- `--mode` accepts `text`, `json`, or `rpc`.
- `--print`/`-p` is the one-shot non-interactive entry point.
- `--list-models [search]` emits a human-readable model table, not JSON.
- `--approve` and `--no-approve` matter for deterministic non-interactive project-resource loading.
- `--extension` has two meanings by scope: global explicit extension/package loading, and `pi update --extension <source>` package update selection.
- `--system-prompt` and `--append-system-prompt` exist; their replace/append/file/inline semantics are intentionally deferred to the sibling `system-prompt` topic.
- Extensions can register additional flags. The inventory here is the built-in baseline for an uncustomized Pi environment.

Help/docs disagreement:

- In non-TTY capture, both the installed 0.73.1 binary and unpacked 0.80.3 package truncated top-level `--help` output around the `pi config` line. I trusted the parser source (`dist/cli/args.js`) and official CLI reference for the full global switch inventory, then confirmed subcommand flags with `install --help`, `remove --help`, `update --help`, and `list --help`.
- The CLI reference shows `pi --export session.jsonl output.html`, but parser inspection only records one value for `--export`; a trailing output path is parsed as a message argument. Wrappers should avoid depending on the two-argument export form unless separately verified.

## Configuration Discovery

Pi uses JSON settings with global and project scopes:

- Global settings: `~/.pi/agent/settings.json`
- Project settings: `.pi/settings.json`

Project settings override global settings, and nested objects are merged. Paths in global settings resolve relative to `~/.pi/agent`; paths in project settings resolve relative to `.pi`. The environment variable `PI_CODING_AGENT_DIR` overrides the global agent directory.

Other wrapper-relevant files and directories include:

- `~/.pi/agent/auth.json` for API-key and OAuth credentials.
- `~/.pi/agent/models.json` for custom providers and models.
- `~/.pi/agent/trust.json` for saved project trust decisions.
- `~/.pi/agent/AGENTS.md`, plus `AGENTS.md` or `CLAUDE.md` discovered from parent directories and the current directory.
- `.pi/SYSTEM.md` / `~/.pi/agent/SYSTEM.md` for replacement system prompts.
- `.pi/APPEND_SYSTEM.md` / `~/.pi/agent/APPEND_SYSTEM.md` for appended system prompts.
- `~/.pi/agent/keybindings.json` for interactive keybindings.
- `~/.pi/agent/sessions/` by default for session JSONL files, unless overridden.
- `~/.pi/agent/npm/` and `~/.pi/agent/git/` for user-scoped package installs; `.pi/npm/` and `.pi/git/` for project-scoped package installs.

Non-interactive modes do not show the project trust prompt. Without a saved trust decision, `defaultProjectTrust: "ask"` and `"never"` ignore project resources; `"always"` trusts them. Use `--approve` or `--no-approve` for deterministic wrapper behavior.

Local config inspection found `~/.pi/agent/auth.json` and backup settings/model files. Some local model backup files contained API credentials, so the research records only path behavior and not local credential contents.

## Environment Variables

The frontmatter records Pi-owned general runtime variables. Provider API-key variables such as `ANTHROPIC_API_KEY`, `OPENAI_API_KEY`, `GEMINI_API_KEY`, `KIMI_API_KEY`, and cloud-specific variables are intentionally omitted from `env_vars` because they belong to model configuration rather than the generic CLI surface.

Additional general environment behavior:

- `VISUAL` and `EDITOR` are external-editor fallbacks when `externalEditor` is not set.
- A configured `httpProxy` setting applies `HTTP_PROXY` and `HTTPS_PROXY` for Pi's process.
- `--offline` sets offline behavior for startup network operations and source inspection showed it also sets `PI_SKIP_VERSION_CHECK=1` internally.
- `PI_TELEMETRY=0` disables install/update telemetry and provider attribution headers, but does not disable update checks.

## Machine Introspection

The most useful machine-readable introspection path is RPC mode. The docs define JSONL commands including:

- `get_available_models`: returns full model objects.
- `get_state`: returns current session state, model, thinking level, stream status, session identifiers, queue modes, and counts.
- `get_messages`: returns conversation messages.

`pi --list-models [search]` is useful to humans and works from bundled plus configured model definitions, but it outputs a text table. `pi list` exposes installed packages as text only in inspected versions.

No built-in command was found for a machine-readable config dump, config schema dump, doctor report, MCP server list, or tool/capability list. Pi intentionally omits built-in MCP; MCP-like behavior is provided by extensions/packages.

## Wrapper Notes

Use `pi -p` for simple one-shot wrapper runs, `--mode json` for event streaming, and `--mode rpc` for full process integration. In JSON modes, stdout is protocol data and stderr should be treated as diagnostics/noise.

For deterministic wrapper runs, set project trust explicitly with `--approve` or `--no-approve`. When isolation matters, also consider `--no-extensions`, `--no-skills`, `--no-prompt-templates`, `--no-context-files`, `--no-session`, and a controlled `PI_CODING_AGENT_DIR` or `--session-dir`.

The host install is stale and old-namespace: `/Users/ken/.bun/bin/pi` is `@mariozechner/pi-coding-agent@0.73.1`. A Claudine wrapper should not assume the binary on PATH is the current official package without checking `pi --version` and, where possible, the resolved package path/name.

`pi config` is interactive. In both installed 0.73.1 and unpacked 0.80.3 inspection, `pi config --help` entered the TUI rather than printing command help.

Package install/update commands can invoke npm/git/ssh and mutate user or project settings. For non-interactive CI-style package operations involving git sources, upstream package docs recommend disabling credential prompts with `GIT_TERMINAL_PROMPT=0` and using batch SSH options.

Windows bash execution requires a bash-compatible shell. Docs say Pi checks a custom `shellPath`, Git Bash, and then `bash.exe` on `PATH`.

## Changelog

- 2026-07-03: Verified latest upstream package as `@earendil-works/pi-coding-agent@0.80.3`; recorded that the host-installed binary is the older `@mariozechner/pi-coding-agent@0.73.1` namespace.
- 2026-07-03: Reworked frontmatter to use separate macOS, Linux, and Windows records for binaries, install methods, and config files.
- 2026-07-03: Added `pi update --all`, package-command trust flags, current Pi-owned runtime variables, and help-output truncation caveat.
- 2026-07-03: Preserved the prior conclusion that Claudine needs a future provider/wrapper update for Pi support.

## Sources

- [Pi homepage](https://pi.dev/)
- [Pi repository](https://github.com/earendil-works/pi)
- [Pi documentation](https://pi.dev/docs/latest)
- [Pi CLI reference](https://github.com/earendil-works/pi/blob/main/packages/coding-agent/docs/usage.md#cli-reference)
- [Pi quickstart](https://github.com/earendil-works/pi/blob/main/packages/coding-agent/docs/quickstart.md)
- [Pi settings docs](https://github.com/earendil-works/pi/blob/main/packages/coding-agent/docs/settings.md)
- [Pi package docs](https://github.com/earendil-works/pi/blob/main/packages/coding-agent/docs/packages.md)
- [Pi JSON mode docs](https://github.com/earendil-works/pi/blob/main/packages/coding-agent/docs/json.md)
- [Pi RPC mode docs](https://github.com/earendil-works/pi/blob/main/packages/coding-agent/docs/rpc.md)
- [Pi ownership/package migration news](https://pi.dev/news/2026/5/7/pi-has-a-new-home)
- [npm package: @earendil-works/pi-coding-agent](https://www.npmjs.com/package/@earendil-works/pi-coding-agent)
- Local command: `npm view @earendil-works/pi-coding-agent version dist-tags bin repository homepage --json`
- Local command: `npm view @mariozechner/pi-coding-agent version dist-tags bin repository homepage --json`
- Local command: `command -v pi; pi --version; pi --help; pi install --help; pi remove --help; pi update --help; pi list --help; pi config --help`
- Local command: `npm pack @earendil-works/pi-coding-agent@0.80.3 --json`
- Local command: `cd /tmp/pi-research-0.80.3/package && npm install --ignore-scripts --no-audit --no-fund --color=false && node dist/cli.js --version && node dist/cli.js --help`
- Local source inspection: `/tmp/pi-research-0.80.3/package/dist/cli/args.js`
- Local source inspection: `/tmp/pi-research-0.80.3/package/dist/config.js`
- Local source inspection: `/tmp/pi-research-0.80.3/package/docs/`
