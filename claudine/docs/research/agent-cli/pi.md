---
$schema: ./_schema.yaml
created: 2026-07-02
last_updated: 2026-07-02
agent: codex
model: default
latest_version: "0.80.3"
homepage: https://pi.dev/
repo: https://github.com/earendil-works/pi
docs: https://pi.dev/docs/latest
cli_docs: https://github.com/earendil-works/pi/tree/main/packages/coding-agent#cli-reference
binaries:
  - os: all
    binary: pi
    alt_binaries: []
    notes: "The npm package exposes bin.pi as dist/cli.js. Official docs and local 0.80.3 package inspection use `pi` on all platforms."
  - os: windows
    binary: pi.cmd
    alt_binaries: ["pi.ps1", "pi.exe"]
    notes: "Windows npm installs commonly expose .cmd and PowerShell shims for the package bin; no separate upstream Windows binary name was documented."
install_methods:
  - os: all
    method: npm
    command: "npm install -g --ignore-scripts @earendil-works/pi-coding-agent"
    notes: "Primary documented install. `--ignore-scripts` is recommended because normal npm installs do not need lifecycle scripts."
  - os: macos
    method: standalone_binary
    command: "curl -fsSL https://pi.dev/install.sh | sh"
    notes: "Official site and README document this installer alternative; docs say curl-installed Pi is still uninstalled with npm."
  - os: linux
    method: standalone_binary
    command: "curl -fsSL https://pi.dev/install.sh | sh"
    notes: "Official site and README document this installer alternative; docs say curl-installed Pi is still uninstalled with npm."
  - os: windows
    method: other
    command: "powershell -c \"irm https://pi.dev/install.ps1 | iex\""
    notes: "Official homepage documents the PowerShell installer."
  - os: all
    method: other
    command: "pnpm add -g --ignore-scripts @earendil-works/pi-coding-agent"
    notes: "Official homepage documents pnpm global install."
  - os: all
    method: other
    command: "bun add -g --ignore-scripts @earendil-works/pi-coding-agent"
    notes: "Official homepage documents Bun global install."
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
    description: "Starts a headless JSONL RPC server over stdin/stdout."
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
    notes: "Can mutate the global Pi install or installed packages; `pi update` defaults to Pi itself."
  - name: list
    description: "Lists installed packages from user and project settings."
    non_interactive: true
    notes: "Local inspection showed human-readable text output only."
  - name: config
    description: "Opens a TUI to enable or disable package resources."
    non_interactive: false
    notes: "`pi config --help` in local inspection still entered the TUI; wrappers should treat it as interactive."
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
    notes: ""
  - flag: --append-system-prompt
    value: "<text-or-file>"
    scope: ["global", "prompt"]
    default: "[]"
    description: "Appends text or file contents to the system prompt."
    example: "pi --append-system-prompt .pi/extra-system.md"
    notes: "Repeatable."
  - flag: --mode
    value: "text | json | rpc"
    scope: ["global", "output", "automation"]
    default: "text"
    description: "Selects text output, JSON event stream output, or JSONL RPC mode."
    example: "pi --mode json -p 'Summarize this repo'"
    notes: "`--mode rpc` cannot accept `@file` arguments."
  - flag: --print
    value: ""
    scope: ["global", "automation"]
    default: "false"
    description: "Runs non-interactively, processes a prompt, and exits."
    example: "pi -p 'Summarize this codebase'"
    notes: "Short form: `-p`; local parser also accepts the next non-flag token as the prompt."
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
    notes: ""
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
    value: "<path>"
    scope: ["global", "resources"]
    default: "[]"
    description: "Loads an extension file."
    example: "pi --extension ./my-extension.ts"
    notes: "Short form: `-e`; repeatable."
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
    notes: "Help examples also show `pi --export session.jsonl output.html`, but parser inspection only proved one value for the flag."
  - flag: --list-models
    value: "[search]"
    scope: ["global", "models", "introspection"]
    default: "false"
    description: "Lists available models, optionally filtered by a fuzzy search."
    example: "pi --list-models sonnet"
    notes: "Human-readable table; local 0.80.3 inspection did not expose a JSON variant."
  - flag: --verbose
    value: ""
    scope: ["global", "diagnostics"]
    default: "false"
    description: "Forces verbose startup, overriding the `quietStartup` setting."
    example: "pi --verbose"
    notes: ""
  - flag: --approve
    value: ""
    scope: ["global", "project_trust"]
    default: "settings defaultProjectTrust"
    description: "Trusts project-local files for this run or package command."
    example: "pi --approve -p 'Use project-local extensions'"
    notes: "Short form: `-a`."
  - flag: --no-approve
    value: ""
    scope: ["global", "project_trust"]
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
    notes: ""
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
config_files:
  - os: all
    scope: user
    path: "~/.pi/agent/settings.json"
    format: json
    notes: "Global settings file; paths inside it resolve relative to `~/.pi/agent`."
  - os: all
    scope: repo
    path: ".pi/settings.json"
    format: json
    notes: "Project settings override global settings; paths inside it resolve relative to `.pi`."
  - os: all
    scope: user
    path: "~/.pi/agent/auth.json"
    format: json
    notes: "Stores API keys or OAuth credentials when configured through `/login`; API key env vars take precedence."
  - os: all
    scope: user
    path: "~/.pi/agent/models.json"
    format: json
    notes: "Custom provider/model definitions for APIs compatible with Pi's supported standards."
  - os: all
    scope: user
    path: "~/.pi/agent/trust.json"
    format: json
    notes: "Stores project trust decisions; non-interactive modes do not prompt and consult trust/defaultProjectTrust instead."
  - os: all
    scope: user
    path: "~/.pi/agent/AGENTS.md"
    format: text
    notes: "Global context instructions loaded at startup unless context files are disabled."
  - os: all
    scope: repo
    path: "AGENTS.md"
    format: text
    notes: "Project context instructions discovered from parent directories and cwd unless `--no-context-files` is set."
  - os: all
    scope: repo
    path: "CLAUDE.md"
    format: text
    notes: "Alternative project context instructions discovered from parent directories and cwd unless `--no-context-files` is set."
  - os: all
    scope: user
    path: "~/.pi/agent/SYSTEM.md"
    format: text
    notes: "Global replacement system prompt documented in usage docs."
  - os: all
    scope: repo
    path: ".pi/SYSTEM.md"
    format: text
    notes: "Project replacement system prompt documented in usage docs."
  - os: all
    scope: user
    path: "~/.pi/agent/APPEND_SYSTEM.md"
    format: text
    notes: "Global appended system prompt documented in usage docs."
  - os: all
    scope: repo
    path: ".pi/APPEND_SYSTEM.md"
    format: text
    notes: "Project appended system prompt documented in usage docs."
  - os: all
    scope: user
    path: "~/.pi/agent/keybindings.json"
    format: json
    notes: "Interactive keybinding customization file referenced by README and docs."
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
    effect: "Disables the Pi version update check."
  - name: PI_TELEMETRY
    effect: "Overrides install/update telemetry opt-in with 1/true/yes or 0/false/no."
  - name: PI_SHARE_VIEWER_URL
    effect: "Sets the base URL for the `/share` command; default is `https://pi.dev/session/`."
  - name: PI_STARTUP_BENCHMARK
    effect: "Source inspection shows this enables startup benchmarking only in interactive mode; it exits with an error in non-interactive modes."
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
    notes: "Lists available models with provider, model, context, max output, thinking, and image columns. Local 0.80.3 inspection found no JSON flag, so wrappers should prefer RPC for machine parsing."
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
    notes: "Lists installed Pi packages from user and project settings. Local 0.80.3 inspection returned plain text (`No packages installed.`) with no JSON switch."
wrapper_notes:
  - "Use `pi -p` for one-shot non-interactive text runs; use `--mode json` for JSONL session events; use `--mode rpc` for bidirectional JSONL integration."
  - "JSON event mode writes machine-readable JSON Lines to stdout; docs recommend redirecting stderr when piping to tools such as jq."
  - "RPC mode uses strict LF-delimited JSONL over stdin/stdout; clients must not use line readers that split on Unicode line separators."
  - "`pi config` is a TUI command. Local 0.80.3 inspection showed `pi config --help` entered the TUI instead of printing command help."
  - "Non-interactive modes do not prompt for project trust. Without saved trust, `defaultProjectTrust: ask` behaves like ignoring project resources; use `--approve` or `--no-approve` for deterministic wrapper behavior."
  - "Project-local settings, extensions, package resources, and `.agents/skills` can execute or influence behavior after trust; wrappers should set trust flags and consider `--no-extensions`, `--no-skills`, `--no-prompt-templates`, and `--no-context-files` when isolation matters."
  - "Set `PI_OFFLINE=1` or pass `--offline` to avoid startup network operations such as version checks, package update checks, and install/update telemetry."
  - "Windows tool execution requires a bash shell; docs say Pi checks custom `shellPath`, Git Bash, then `bash.exe` on PATH."
  - "No MCP support is built in according to the README philosophy section; MCP-like behavior requires an extension."
  - "Provider API-key environment variables are numerous and model-config-owned; this document records only Pi-owned general runtime env vars."
changes: []
requires_claudine_update: true
reason: "Pi is researched but not yet represented in Claudine's compiled provider enum or wrapper metadata; this CLI surface supplies the binary, install, modes, config, and machine-introspection facts needed for future support."
---

## Overview

Pi is a minimal, extensible terminal coding harness from Earendil Works. The current public repository is `earendil-works/pi`, and the CLI package is `@earendil-works/pi-coding-agent`. The package exposes a single `pi` command and supports four practical wrapper surfaces: interactive TUI, print mode, JSON event stream mode, and JSONL RPC mode.

Local inspection used the npm package `@earendil-works/pi-coding-agent@0.80.3` installed into `/tmp` with lifecycle scripts disabled. `pi --version` reported `0.80.3`, matching npm metadata and the `v0.80.3` Git tag observed on 2026-07-02.

## Installation and Binaries

The primary documented install is:

```bash
npm install -g --ignore-scripts @earendil-works/pi-coding-agent
```

The official docs say `--ignore-scripts` is recommended because Pi does not need package lifecycle scripts for normal npm installs. The official homepage and README also document shell installers and alternative JavaScript package managers:

```bash
curl -fsSL https://pi.dev/install.sh | sh
powershell -c "irm https://pi.dev/install.ps1 | iex"
pnpm add -g --ignore-scripts @earendil-works/pi-coding-agent
bun add -g --ignore-scripts @earendil-works/pi-coding-agent
```

The npm package's `bin` map exposes only `pi`. Windows npm installs are expected to create package-manager shims such as `pi.cmd` and possibly `pi.ps1`; no upstream separate Windows executable name was documented.

## Subcommands

`pi --help` in 0.80.3 lists package-management commands: `install`, `remove`, `uninstall`, `update`, `list`, and `config`. `uninstall` is an alias for `remove`, not a Pi self-uninstaller. Pi itself should be uninstalled with the package manager that installed it.

The main agent invocation is mode-driven rather than subcommand-driven:

- `pi` starts the interactive TUI.
- `pi -p "prompt"` runs print mode and exits.
- `pi --mode json -p "prompt"` streams JSON Lines session events.
- `pi --mode rpc` starts a JSONL RPC process over stdin/stdout.

`pi config` is interactive. Local inspection of `pi config --help` still entered the resource-configuration TUI, so wrappers should not treat it as a non-interactive help command.

## CLI Switch Inventory

The frontmatter `cli_switches` array contains the full switch inventory observed from `pi --help`, `pi install --help`, `pi remove --help`, `pi update --help`, and `pi list --help` for 0.80.3.

Important grouping notes:

- `--mode` accepts `text`, `json`, or `rpc`.
- `--print`/`-p` is the one-shot non-interactive entry point.
- `--list-models [search]` emits a human-readable model table, not JSON.
- `--approve` and `--no-approve` matter for deterministic non-interactive project-resource loading.
- `--extension` has two meanings by scope: global explicit extension loading, and `pi update --extension <source>` package update selection.
- Extensions can register additional flags, so the inventory above is the built-in baseline, not a guarantee for a customized Pi environment.

## Configuration Discovery

Pi uses JSON settings with global and project scopes:

- `~/.pi/agent/settings.json`
- `.pi/settings.json`

Project settings override global settings, and nested objects are merged. Paths in global settings resolve relative to `~/.pi/agent`; paths in project settings resolve relative to `.pi`.

Other wrapper-relevant files include:

- `~/.pi/agent/auth.json` for stored API keys and OAuth credentials.
- `~/.pi/agent/models.json` for custom providers/models.
- `~/.pi/agent/trust.json` for saved project trust decisions.
- `~/.pi/agent/AGENTS.md`, plus `AGENTS.md` or `CLAUDE.md` discovered from parent directories and the current directory.
- `.pi/SYSTEM.md` / `~/.pi/agent/SYSTEM.md` for replacement system prompts.
- `.pi/APPEND_SYSTEM.md` / `~/.pi/agent/APPEND_SYSTEM.md` for appended system prompts.
- `~/.pi/agent/keybindings.json` for interactive keybindings.

Non-interactive modes do not show the project trust prompt. Without a saved trust decision, `defaultProjectTrust: "ask"` and `"never"` ignore project resources; `"always"` trusts them. Use `--approve` or `--no-approve` for one run.

## Environment Variables

The frontmatter records Pi-owned general runtime variables. Provider API-key variables such as `ANTHROPIC_API_KEY`, `OPENAI_API_KEY`, `GEMINI_API_KEY`, `KIMI_API_KEY`, and many others are intentionally omitted from `env_vars` because they belong to model configuration rather than the generic CLI surface.

Additional general environment behavior:

- `VISUAL` and `EDITOR` are external-editor fallbacks when `externalEditor` is not set.
- A configured `httpProxy` setting applies `HTTP_PROXY` and `HTTPS_PROXY` for Pi's process.
- `--offline` sets offline behavior for startup network operations and source inspection showed it also sets `PI_SKIP_VERSION_CHECK=1` internally.

## Machine Introspection

The most useful machine-readable introspection path is RPC mode. The docs define JSONL commands including:

- `get_available_models`: returns full model objects.
- `get_state`: returns current session state, model, thinking level, stream status, session identifiers, queue modes, and counts.
- `get_messages`: returns conversation messages.

`pi --list-models [search]` is useful to humans and local inspection confirmed it works offline against bundled model definitions, but it outputs a text table. `pi list` exposes installed packages as text only in 0.80.3.

## Wrapper Notes

Pi is highly extensible. Extensions can register flags, commands, tools, providers, UI, compaction behavior, and other runtime hooks. A wrapper that needs deterministic behavior should explicitly control resource loading and trust:

```bash
PI_OFFLINE=1 pi --no-extensions --no-skills --no-prompt-templates --no-context-files --no-approve -p "..."
```

That isolation also disables useful project instructions and user customization, so Claudine should expose the trade-off rather than silently forcing it for every run.

For JSON integrations, prefer:

```bash
pi --mode json -p "Summarize this repo"
```

For bidirectional process integrations, prefer:

```bash
pi --mode rpc --no-session
```

RPC clients must split on LF only. The docs specifically warn that generic line readers such as Node `readline` are not protocol-compliant because they split on additional Unicode line separators.

Pi does not provide built-in MCP support; the README states MCP should be added through an extension if needed. Pi also does not provide built-in permission popups; confirmation, sandboxing, or policy gates are expected to be implemented through containers or extensions.

On Windows, Pi requires a bash shell for shell tool execution. The documented search order is custom `shellPath`, Git Bash, then `bash.exe` on PATH.

## Sources

- [Pi homepage](https://pi.dev/)
- [Pi documentation](https://pi.dev/docs/latest)
- [Quickstart](https://pi.dev/docs/latest/quickstart)
- [Settings](https://pi.dev/docs/latest/settings)
- [JSON Event Stream Mode](https://pi.dev/docs/latest/json)
- [RPC Mode](https://pi.dev/docs/latest/rpc)
- [Windows Setup](https://pi.dev/docs/latest/windows)
- [GitHub repository](https://github.com/earendil-works/pi)
- [Coding agent README / CLI reference](https://github.com/earendil-works/pi/tree/main/packages/coding-agent#cli-reference)
- [npm package](https://www.npmjs.com/package/@earendil-works/pi-coding-agent)
- Local inspection: `npm view @earendil-works/pi-coding-agent version bin dist.tarball --json`; isolated install of `@earendil-works/pi-coding-agent@0.80.3`; `pi --version`; `pi --help`; `pi install --help`; `pi remove --help`; `pi update --help`; `pi list --help`; `pi --list-models`.
