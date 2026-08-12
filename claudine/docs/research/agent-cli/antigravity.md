---
$schema: ./_schema.yaml
created: 2026-07-08
last_updated: 2026-07-08
agent: codex
model: default
latest_version: "1.1.0"
homepage: https://antigravity.google/product/antigravity-cli
repo: https://github.com/google-antigravity/antigravity-cli
docs: https://antigravity.google/docs/cli-overview
cli_docs: https://antigravity.google/docs/cli-reference
binaries:
  - os: macos
    binary: agy
    alt_binaries: []
    notes: "Official installer writes `agy` to `$HOME/.local/bin` by default. Local inspection found `/Users/ken/.local/bin/agy`."
  - os: linux
    binary: agy
    alt_binaries: []
    notes: "Official Unix installer writes `agy` to `$HOME/.local/bin` by default."
  - os: windows
    binary: agy.exe
    alt_binaries: ["agy"]
    notes: "Official PowerShell and CMD installers write `agy.exe` to `%LOCALAPPDATA%\\agy\\bin` by default; users normally type `agy` from PowerShell or CMD after PATH setup."
install_methods:
  - os: macos
    method: standalone_binary
    command: "curl -fsSL https://antigravity.google/cli/install.sh | bash"
    notes: "Official macOS install/upgrade path. The script supports `--dir <path>` for a custom target."
  - os: linux
    method: standalone_binary
    command: "curl -fsSL https://antigravity.google/cli/install.sh | bash"
    notes: "Official Linux install/upgrade path. The script detects glibc versus musl and supports `--dir <path>`."
  - os: windows
    method: standalone_binary
    command: "irm https://antigravity.google/cli/install.ps1 | iex"
    notes: "Official Windows PowerShell install/upgrade path. The script supports `-d`/`--dir`."
  - os: windows
    method: standalone_binary
    command: "curl -fsSL https://antigravity.google/cli/install.cmd -o install.cmd && install.cmd && del install.cmd"
    notes: "Official Windows CMD install/upgrade path."
subcommands:
  - name: default
    description: "Launches the Antigravity CLI TUI, or runs print mode when `--print`/`--prompt` is supplied."
    non_interactive: true
    notes: "Automation entry point is the default command with `--print <prompt>` or `--prompt <prompt>`. Without print mode it needs a TTY and may start browser/auth onboarding."
  - name: changelog
    description: "Shows cached changelog and release notes."
    non_interactive: true
    notes: "Observed output is human text, not JSON."
  - name: help
    description: "Shows top-level help or plugin help."
    non_interactive: true
    notes: "Equivalent to `agy --help` for top-level help."
  - name: install
    description: "Configures shell PATH and shell profile settings after the binary is installed."
    non_interactive: true
    notes: "Installer scripts call this command automatically; it can mutate shell profile files unless skipped."
  - name: models
    description: "Lists available models for the signed-in account."
    non_interactive: true
    notes: "Requires authentication. Local unsigned-in probe printed an error but exited 0."
  - name: plugin
    description: "Manages plugins: list, import, install, uninstall, enable, disable, validate, and marketplace links."
    non_interactive: false
    notes: "`agy plugin list` works without a TTY, but `agy plugin <subcommand> --help` attempted to open Bubble Tea and failed without `/dev/tty`."
  - name: plugins
    description: "Alias for `plugin`."
    non_interactive: false
    notes: "Same caveats as `plugin`."
  - name: update
    description: "Updates the CLI."
    non_interactive: false
    notes: "Mutates the installed binary/state; help output is minimal."
cli_switches:
  - flag: --help
    value: ""
    scope: ["global"]
    default: "false"
    description: "Prints top-level help and exits."
    example: "agy --help"
    notes: "Alias-like `agy help` returns the same top-level help."
  - flag: --version
    value: ""
    scope: ["global"]
    default: "false"
    description: "Prints the installed CLI version and exits."
    example: "agy --version"
    notes: "Accepted locally even though omitted from `agy --help`; returned `1.1.0`."
  - flag: --add-dir
    value: "<PATH>"
    scope: ["global", "default"]
    default: "[]"
    description: "Adds a directory to the workspace; repeatable."
    example: "agy --add-dir ../other-package"
    notes: "Value-taking repeatable flag."
  - flag: --continue
    value: ""
    scope: ["global", "default", "resume"]
    default: "false"
    description: "Continues the most recent conversation."
    example: "agy --continue --print \"summarize the latest state\""
    notes: "Alias: `-c`."
  - flag: -c
    value: ""
    scope: ["global", "default", "resume"]
    default: "false"
    description: "Short alias for `--continue`."
    example: "agy -c -p \"continue\""
    notes: "Boolean."
  - flag: --conversation
    value: "<ID>"
    scope: ["global", "default", "resume"]
    default: ""
    description: "Resumes a previous conversation by ID."
    example: "agy --conversation 01234567-89ab-cdef-0123-456789abcdef --print \"continue\""
    notes: "Help text does not show a metavariable, but the description says it takes a conversation ID."
  - flag: --dangerously-skip-permissions
    value: ""
    scope: ["global", "default", "permissions"]
    default: "false"
    description: "Auto-approves all tool permission requests without prompting."
    example: "agy --dangerously-skip-permissions --print \"run the formatter\""
    notes: "Wrapper-impacting safety flag; permission semantics belong to the agent-permissions topic."
  - flag: --prompt-interactive
    value: "<PROMPT>"
    scope: ["global", "default", "interactive"]
    default: ""
    description: "Runs an initial prompt interactively and continues the session."
    example: "agy --prompt-interactive \"review this repo\""
    notes: "Alias: `-i`; requires a TTY after the initial prompt."
  - flag: -i
    value: "<PROMPT>"
    scope: ["global", "default", "interactive"]
    default: ""
    description: "Short alias for `--prompt-interactive`."
    example: "agy -i \"review this repo\""
    notes: "Value-taking."
  - flag: --log-file
    value: "<PATH>"
    scope: ["global", "default", "logging"]
    default: ""
    description: "Overrides the CLI log file path."
    example: "agy --log-file /tmp/agy.log --print \"hello\""
    notes: "General wrapper note only; detailed logging belongs to agent-logging."
  - flag: --mode
    value: "<MODE>"
    scope: ["global", "default", "execution"]
    default: "request-review"
    description: "Sets the agent execution mode for this session."
    example: "agy --mode plan --print \"plan a migration\""
    notes: "Help lists `accept-edits` and `plan`; changelog says 1.1.0 also has default/request-review behavior."
  - flag: --model
    value: "<MODEL>"
    scope: ["global", "default", "model"]
    default: ""
    description: "Selects the model for the current CLI session."
    example: "agy --model \"Gemini 3.1 Pro (High)\" --print \"summarize\""
    notes: "Model catalog and endpoint semantics belong to model-config."
  - flag: --new-project
    value: ""
    scope: ["global", "default", "project"]
    default: "false"
    description: "Creates a new project for this session."
    example: "agy --new-project --print \"start fresh\""
    notes: "Boolean."
  - flag: --project
    value: "<PROJECT_ID>"
    scope: ["global", "default", "project"]
    default: ""
    description: "Selects the project ID for the current CLI session."
    example: "agy --project default-cli-project --print \"status\""
    notes: "Value-taking."
  - flag: --print
    value: "<PROMPT>"
    scope: ["global", "default", "non_interactive"]
    default: ""
    description: "Runs a single prompt non-interactively and prints the response."
    example: "agy --print \"Say OK\""
    notes: "Alias: `-p`; local unsigned-in probe returned `Error: authentication failed or timed out` and exit 0."
  - flag: -p
    value: "<PROMPT>"
    scope: ["global", "default", "non_interactive"]
    default: ""
    description: "Short alias for `--print`."
    example: "agy -p \"Say OK\""
    notes: "Value-taking."
  - flag: --prompt
    value: "<PROMPT>"
    scope: ["global", "default", "non_interactive"]
    default: ""
    description: "Alias for `--print`."
    example: "agy --prompt \"Say OK\""
    notes: "Value-taking."
  - flag: --print-timeout
    value: "<DURATION>"
    scope: ["global", "default", "non_interactive"]
    default: "5m0s"
    description: "Sets the wait timeout for print mode."
    example: "agy --print \"Say OK\" --print-timeout 30s"
    notes: "Go duration syntax was accepted locally with `3s`."
  - flag: --sandbox
    value: ""
    scope: ["global", "default", "execution"]
    default: "false"
    description: "Runs in a sandbox with terminal restrictions enabled."
    example: "agy --sandbox --print \"inspect this repo\""
    notes: "Boolean."
  - flag: --output-format
    value: "<FORMAT>"
    scope: ["global", "default", "non_interactive"]
    default: "text"
    description: "Selects print-mode output format."
    example: "agy --print \"Say OK\" --output-format json"
    notes: "Hidden from `agy --help`, but accepted locally. `json` returned an object with `conversation_id`, `status`, `response`, `error`, `duration_seconds`, `num_turns`, and `usage`. `--json` and `--format json` were rejected."
  - flag: --dir
    value: "<PATH>"
    scope: ["install"]
    default: ""
    description: "Custom directory target to configure PATH for."
    example: "agy install --dir ~/.local/bin"
    notes: "Installer scripts also accept `--dir`/`-d` before calling `agy install`."
  - flag: --skip-aliases
    value: ""
    scope: ["install"]
    default: "false"
    description: "Bypasses shell profile alias purging."
    example: "agy install --skip-aliases"
    notes: "Boolean install flag."
  - flag: --skip-path
    value: ""
    scope: ["install"]
    default: "false"
    description: "Bypasses shell profile PATH appending."
    example: "agy install --skip-path"
    notes: "Boolean install flag."
  - flag: -h
    value: ""
    scope: ["install", "models", "changelog"]
    default: "false"
    description: "Shows subcommand help."
    example: "agy models -h"
    notes: "Shown for `install`, `models`, and `changelog`; plugin subcommand help tried to open the TUI in non-TTY probes."
config_paths:
  - os: macos
    scope: user
    path: "~/.gemini/antigravity-cli/settings.json"
    format: json
    notes: "CLI-specific settings. Local file contained telemetry, model, and trusted workspace settings."
  - os: linux
    scope: user
    path: "~/.gemini/antigravity-cli/settings.json"
    format: json
    notes: "Same home-relative path is used by the cross-platform CLI docs and embedded help."
  - os: windows
    scope: user
    path: "%USERPROFILE%\\.gemini\\antigravity-cli\\settings.json"
    format: json
    notes: "Windows home-relative spelling of the same CLI settings file."
  - os: macos
    scope: user
    path: "~/.gemini/config/config.json"
    format: json
    notes: "Shared Antigravity configuration root. Local file contained `userSettings`."
  - os: linux
    scope: user
    path: "~/.gemini/config/config.json"
    format: json
    notes: "Shared Antigravity configuration root."
  - os: windows
    scope: user
    path: "%USERPROFILE%\\.gemini\\config\\config.json"
    format: json
    notes: "Windows home-relative spelling of the shared Antigravity configuration root."
  - os: macos
    scope: user
    path: "~/.gemini/config/mcp_config.json"
    format: json
    notes: "Global MCP server configuration; MCP details belong to the mcp topic."
  - os: linux
    scope: user
    path: "~/.gemini/config/mcp_config.json"
    format: json
    notes: "Global MCP server configuration."
  - os: windows
    scope: user
    path: "%USERPROFILE%\\.gemini\\config\\mcp_config.json"
    format: json
    notes: "Windows home-relative spelling of the global MCP server configuration."
  - os: macos
    scope: user
    path: "~/.gemini/config/projects/<project-id>.json"
    format: json
    notes: "Project settings; local file maps folder URIs to project resources."
  - os: linux
    scope: user
    path: "~/.gemini/config/projects/<project-id>.json"
    format: json
    notes: "Project settings."
  - os: windows
    scope: user
    path: "%USERPROFILE%\\.gemini\\config\\projects\\<project-id>.json"
    format: json
    notes: "Windows home-relative spelling of project settings."
  - os: macos
    scope: user
    path: "~/.gemini/antigravity-cli/keybindings.json"
    format: json
    notes: "CLI keybindings; local file contains command-to-key arrays."
  - os: linux
    scope: user
    path: "~/.gemini/antigravity-cli/keybindings.json"
    format: json
    notes: "CLI keybindings."
  - os: windows
    scope: user
    path: "%USERPROFILE%\\.gemini\\antigravity-cli\\keybindings.json"
    format: json
    notes: "Windows home-relative spelling of CLI keybindings."
  - os: macos
    scope: repo
    path: "<repo>/.agents/"
    format: other
    notes: "Workspace customization root; alternatives `.agent/`, `_agents/`, and `_agent/` are also discovered."
  - os: linux
    scope: repo
    path: "<repo>/.agents/"
    format: other
    notes: "Workspace customization root; alternatives `.agent/`, `_agents/`, and `_agent/` are also discovered."
  - os: windows
    scope: repo
    path: "<repo>\\.agents\\"
    format: other
    notes: "Workspace customization root; alternatives `.agent\\`, `_agents\\`, and `_agent\\` are also discovered."
  - os: macos
    scope: user
    path: "~/.gemini/config/skills.json"
    format: json
    notes: "Optional explicit skills registry."
  - os: linux
    scope: user
    path: "~/.gemini/config/skills.json"
    format: json
    notes: "Optional explicit skills registry."
  - os: windows
    scope: user
    path: "%USERPROFILE%\\.gemini\\config\\skills.json"
    format: json
    notes: "Optional explicit skills registry."
  - os: macos
    scope: user
    path: "~/.gemini/config/plugins.json"
    format: json
    notes: "Optional explicit plugin registry."
  - os: linux
    scope: user
    path: "~/.gemini/config/plugins.json"
    format: json
    notes: "Optional explicit plugin registry."
  - os: windows
    scope: user
    path: "%USERPROFILE%\\.gemini\\config\\plugins.json"
    format: json
    notes: "Optional explicit plugin registry."
  - os: macos
    scope: other
    path: "~/.gemini/antigravity-cli/cache/projects.json"
    format: json
    notes: "Workspace-to-project cache; local print-mode/changelog probes wrote or used cache files under the CLI state root."
  - os: linux
    scope: other
    path: "~/.gemini/antigravity-cli/cache/projects.json"
    format: json
    notes: "Workspace-to-project cache."
  - os: windows
    scope: other
    path: "%USERPROFILE%\\.gemini\\antigravity-cli\\cache\\projects.json"
    format: json
    notes: "Windows home-relative spelling of the workspace-to-project cache."
env_vars:
  - name: AGY_CLI_CMD_OUTPUT_PERCENTAGE
    effect: "Customizes the maximum height of command outputs in the TUI as a percentage of terminal height."
  - name: AGY_CLI_DISABLE_LATEX
    effect: "Disables LaTeX/math rendering globally in the CLI."
  - name: AGY_CLI_HIDE_ACCOUNT_INFO
    effect: "Hides account email and plan tier from the TUI header."
  - name: AGY_CLI_DISABLE_AUTO_UPDATE
    effect: "Appears in the installed binary as an auto-update disable switch; exact accepted values were not documented in help output."
  - name: AGY_CLI_FORCE_OSC8
    effect: "Appears in the installed binary and is consistent with forced OSC8 hyperlink rendering; exact accepted values were not documented in help output."
  - name: AGY_CLI_EXPERIMENTAL_RENDERING
    effect: "Appears in the installed binary as a rendering feature toggle; exact accepted values were not documented in help output."
  - name: AGY_CLI_LOGO_STYLE
    effect: "Appears in the installed binary as a logo style override; exact accepted values were not documented in help output."
  - name: AGY_CLI_NEW_HARNESS
    effect: "Appears in the installed binary as a harness feature toggle; exact accepted values were not documented in help output."
machine_introspection:
  - command: "agy --print \"<prompt>\" --output-format json"
    purpose: other
    machine_readable: true
    output_format: json
    useful_for_codegen: false
    notes: "Hidden but accepted. Useful for wrappers to parse `status`, `response`, `error`, and `usage`; not a schema/catalog endpoint."
  - command: "agy models"
    purpose: models
    machine_readable: false
    output_format: text
    useful_for_codegen: false
    notes: "Requires authentication. Local unsigned-in probe printed `Error: Please sign in to view available models. Launch the CLI without arguments to sign in.` and exited 0."
  - command: "agy plugin list"
    purpose: plugins
    machine_readable: false
    output_format: text
    useful_for_codegen: false
    notes: "Local empty-state output was `No imported plugins.` No JSON mode was discovered."
  - command: "agy changelog"
    purpose: other
    machine_readable: false
    output_format: text
    useful_for_codegen: false
    notes: "Prints cached release notes as human text. Useful for diagnostics, not code generation."
wrapper_notes:
  - "Use `/Users/ken/.local/bin/agy` or PATH lookup for the real CLI binary; the macOS `/Applications/Antigravity.app` executable is the desktop app, not the CLI."
  - "`--print`/`--prompt` is the wrapper automation entry point. Plain `agy` launches a TUI and may initiate browser-based sign-in."
  - "Local unsigned-in print-mode failures exit 0. Wrappers must parse stderr or `--output-format json` `status`/`error` instead of trusting the exit code."
  - "`--output-format json` is hidden from help but accepted; `--json`, `--format json`, and `--verbose` are rejected."
  - "No direct system-prompt delivery flag was discovered. Negative probes for `--system-prompt`, `--append-system-prompt`, `--replace-system-prompt`, and `--instruction` were rejected by installed 1.1.0. The deeper system-prompt topic should own semantics if a future release adds such flags."
  - "`agy plugin <subcommand> --help` attempted to open Bubble Tea and failed without `/dev/tty`; do not assume every help path is non-interactive."
  - "`agy models` and unauthenticated `agy --print` report auth failures but exit 0."
  - "The CLI writes state under `~/.gemini/antigravity-cli`, including logs, changelog cache, project cache, updater status, conversations, and built-in skills."
  - "The installer scripts create staging directories under `$HOME/.cache/antigravity/staging` on Unix and `%LOCALAPPDATA%\\antigravity\\staging` on Windows, then call `agy install` to mutate shell environment configuration."
  - "The Windows CMD installer rejects command lines containing shell metacharacters such as `&`, `|`, `;`, `<`, `>`, and `^`."
changes: []
requires_claudine_update: true
reason: "Antigravity CLI is a newly researched provider with a distinct binary (`agy`/`agy.exe`), hidden JSON print mode, zero-exit auth failures, and config/state paths not covered by the existing Claudine provider enum."
---

# Antigravity CLI Surface

## Overview

Antigravity CLI is Google's terminal-first interface for the Antigravity agent platform. The public repository describes it as the terminal surface for Antigravity's shared agent engine, and the primary command a user types is `agy`.

The current upstream version verified for this research is `1.1.0`. I verified it three ways on 2026-07-08: local `agy --version` returned `1.1.0`; the official updater manifest for `darwin_arm64` returned `"version": "1.1.0"`; and the GitHub latest release API returned tag `1.1.0`, published on 2026-07-08.

Primary official URLs:

| Resource | URL |
| --- | --- |
| Homepage | <https://antigravity.google/product/antigravity-cli> |
| Repository | <https://github.com/google-antigravity/antigravity-cli> |
| General docs | <https://antigravity.google/docs/cli-overview> |
| CLI reference | <https://antigravity.google/docs/cli-reference> |
| Install docs | <https://antigravity.google/docs/cli-install> |

## Installation and Binaries

The public command is `agy` on macOS and Linux and `agy.exe` on Windows. On Windows, users normally type `agy` after `%LOCALAPPDATA%\agy\bin` is added to `PATH`, but the installed file is `agy.exe`.

The official install commands are:

| OS | Binary | Default install path | Official command |
| --- | --- | --- | --- |
| macOS | `agy` | `$HOME/.local/bin/agy` | `curl -fsSL https://antigravity.google/cli/install.sh \| bash` |
| Linux | `agy` | `$HOME/.local/bin/agy` | `curl -fsSL https://antigravity.google/cli/install.sh \| bash` |
| Windows PowerShell | `agy.exe` | `%LOCALAPPDATA%\agy\bin\agy.exe` | `irm https://antigravity.google/cli/install.ps1 \| iex` |
| Windows CMD | `agy.exe` | `%LOCALAPPDATA%\agy\bin\agy.exe` | `curl -fsSL https://antigravity.google/cli/install.cmd -o install.cmd && install.cmd && del install.cmd` |

The Unix installer accepts `-d, --dir <path>` to install to a custom directory. It detects `darwin_amd64`, `darwin_arm64`, `linux_amd64`, `linux_arm64`, and musl Linux variants, downloads a manifest from `https://antigravity-cli-auto-updater-974169037036.us-central1.run.app/manifests/<platform>.json`, verifies SHA-512, copies the binary, clears macOS quarantine attributes, and calls `agy install`.

The Windows PowerShell and CMD installers use `%LOCALAPPDATA%\agy\bin` by default, query the same updater service, verify SHA-512, copy `agy.exe`, and call `agy.exe install`. The CMD installer additionally rejects command lines containing shell metacharacters before processing arguments.

Local host evidence:

- `/Users/ken/.local/bin/agy` exists and returned `1.1.0` for `--version`.
- `/Applications/Antigravity.app` exists and reports desktop app version `2.2.1`; this is not the CLI binary.
- `/Applications/Antigravity IDE.app` also exists; this is an IDE app bundle, not the CLI binary.

## Subcommands

Top-level help from installed `agy` 1.1.0 exposes these subcommands and modes:

| Command or mode | Description | Non-interactive suitability |
| --- | --- | --- |
| default invocation | Launches the TUI, or runs a prompt when `--print`/`--prompt` is supplied. | Suitable for automation only with `--print`/`--prompt`; otherwise needs a TTY and may start sign-in. |
| `changelog` | Shows cached changelog and release notes. | Non-interactive text output. |
| `help` | Shows top-level help or plugin help. | Top-level help is non-interactive; plugin subcommand help can require a TTY. |
| `install` | Configures environment paths and shell settings. | Non-interactive, but mutates shell configuration unless skipped. |
| `models` | Lists available models. | Non-interactive text output, but requires sign-in; unsigned-in local probe exited 0 with an error message. |
| `plugin` | Manages plugins: `list`, `import`, `install`, `uninstall`, `enable`, `disable`, `validate`, and `link`. | Mixed. `plugin list` worked without TTY in the empty state; `plugin <subcommand> --help` attempted Bubble Tea and failed without `/dev/tty`. |
| `plugins` | Alias for `plugin`. | Same as `plugin`. |
| `update` | Updates the CLI. | Mutating operation; not appropriate for wrapper discovery unless explicitly requested. |

The `plugin` subcommand exposes these second-level commands in `agy plugin --help`:

| Plugin command | Description |
| --- | --- |
| `list` | Lists imported plugins. |
| `import [source]` | Imports plugins from Gemini or Claude. |
| `install <target>` | Installs a plugin, including marketplace targets. |
| `uninstall <name>` | Uninstalls a plugin. |
| `enable <name>` | Enables a plugin. |
| `disable <name>` | Disables a plugin. |
| `validate [path]` | Validates a plugin. |
| `link <mp> <target>` | Generates a marketplace link. |
| `help` | Shows plugin help. |

## CLI Switch Inventory

Installed `agy --help` lists these global/default-mode switches:

| Flag | Type | Scope | Default | Description | Example |
| --- | --- | --- | --- | --- | --- |
| `--add-dir <path>` | value, repeatable | global/default | `[]` | Adds a directory to the workspace. | `agy --add-dir ../shared` |
| `-c`, `--continue` | boolean | global/default | `false` | Continues the most recent conversation. | `agy -c -p "continue"` |
| `--conversation <id>` | value | global/default | empty | Resumes a previous conversation by ID. | `agy --conversation <id> --print "continue"` |
| `--dangerously-skip-permissions` | boolean | global/default | `false` | Auto-approves all tool permission requests. | `agy --dangerously-skip-permissions --print "run checks"` |
| `-i`, `--prompt-interactive <prompt>` | value | global/default | empty | Runs an initial prompt interactively and continues the session. | `agy -i "review this repo"` |
| `--log-file <path>` | value | global/default | default log path | Overrides CLI log file path. | `agy --log-file /tmp/agy.log --print "hello"` |
| `--mode <mode>` | value | global/default | `request-review` behavior in 1.1.0 | Sets agent execution mode for the session. Help lists `accept-edits` and `plan`; changelog describes default/request-review behavior. | `agy --mode plan --print "plan the change"` |
| `--model <model>` | value | global/default | from settings/account | Selects the model for the current CLI session. | `agy --model "Gemini 3.1 Pro (High)" --print "summarize"` |
| `--new-project` | boolean | global/default | `false` | Creates a new project for this session. | `agy --new-project --print "start fresh"` |
| `-p`, `--print <prompt>` | value | global/default | empty | Runs one prompt non-interactively and prints the response. | `agy -p "Say OK"` |
| `--print-timeout <duration>` | value | global/default | `5m0s` | Sets print-mode wait timeout. | `agy --print "Say OK" --print-timeout 30s` |
| `--project <project-id>` | value | global/default | inferred/default project | Selects the project ID for the current CLI session. | `agy --project default-cli-project --print "status"` |
| `--prompt <prompt>` | value | global/default | empty | Alias for `--print`. | `agy --prompt "Say OK"` |
| `--sandbox` | boolean | global/default | `false` | Runs in a sandbox with terminal restrictions enabled. | `agy --sandbox --print "inspect this repo"` |
| `--help` | boolean | global | `false` | Prints help. | `agy --help` |
| `--version` | boolean | global | `false` | Prints the installed version. | `agy --version` |

Hidden or help-omitted switches observed by negative and positive probes:

| Flag | Type | Scope | Default | Finding | Example |
| --- | --- | --- | --- | --- | --- |
| `--output-format <format>` | value | print mode | `text` | Accepted by installed 1.1.0 although omitted from help. `json` returns a JSON object with `conversation_id`, `status`, `response`, `error`, `duration_seconds`, `num_turns`, and `usage`. | `agy --print "Say OK" --output-format json` |
| `--json` | boolean? | unknown | none | Rejected: `flags provided but not defined: -json`. | Not supported. |
| `--format json` | value | unknown | none | Rejected: `flags provided but not defined: -format`. | Not supported. |
| `--verbose` | boolean | unknown | none | Rejected. | Not supported. |
| `--system-prompt <text>` | value | system prompt | none | Rejected by installed 1.1.0. | Not supported. |
| `--append-system-prompt <text>` | value | system prompt | none | Rejected by installed 1.1.0. | Not supported. |
| `--replace-system-prompt <text>` | value | system prompt | none | Rejected by installed 1.1.0. | Not supported. |
| `--instruction <text>` | value | system prompt | none | Rejected by installed 1.1.0. | Not supported. |

This topic records only that no direct system-prompt delivery switch was discovered in the 1.1.0 CLI surface. If a future release adds such flags, the replace/append/file/inline semantics should be documented in the `system-prompt` topic, not duplicated here.

Subcommand-specific flags:

| Command | Flag | Type | Default | Description | Example |
| --- | --- | --- | --- | --- | --- |
| `install` | `--dir <path>` | value | default binary dir | Custom directory target to configure PATH for. | `agy install --dir ~/.local/bin` |
| `install` | `--skip-aliases` | boolean | `false` | Bypasses shell profile alias purging. | `agy install --skip-aliases` |
| `install` | `--skip-path` | boolean | `false` | Bypasses shell profile PATH appending. | `agy install --skip-path` |
| `install` | `-h`, `--help` | boolean | `false` | Shows install help. | `agy install --help` |
| `models` | `-h`, `--help` | boolean | `false` | Shows models help. | `agy models --help` |
| `changelog` | `-h`, `--help` | boolean | `false` | Shows changelog help. | `agy changelog --help` |

When help output and observed behavior disagree, I trust observed behavior for wrapper implementation. The most important disagreement is `--output-format json`: it is hidden from help, but installed 1.1.0 accepted it and returned parseable JSON in print mode.

## Configuration Discovery

Observed and embedded documentation point to two user-level roots:

- CLI-private state and settings: `~/.gemini/antigravity-cli/`
- Shared Antigravity configuration: `~/.gemini/config/`

Per-OS path spellings:

| OS | Scope | Path | Format | Notes |
| --- | --- | --- | --- | --- |
| macOS | user | `~/.gemini/antigravity-cli/settings.json` | JSON | CLI-specific settings. Local file contained telemetry, model, and trusted workspace values. |
| Linux | user | `~/.gemini/antigravity-cli/settings.json` | JSON | Same home-relative path. |
| Windows | user | `%USERPROFILE%\.gemini\antigravity-cli\settings.json` | JSON | Windows spelling of the same home-relative path. |
| macOS | user | `~/.gemini/config/config.json` | JSON | Shared user settings. |
| Linux | user | `~/.gemini/config/config.json` | JSON | Shared user settings. |
| Windows | user | `%USERPROFILE%\.gemini\config\config.json` | JSON | Windows spelling of the same shared settings file. |
| macOS | user | `~/.gemini/config/mcp_config.json` | JSON | Global MCP servers. |
| Linux | user | `~/.gemini/config/mcp_config.json` | JSON | Global MCP servers. |
| Windows | user | `%USERPROFILE%\.gemini\config\mcp_config.json` | JSON | Windows spelling of global MCP config. |
| macOS | user | `~/.gemini/config/projects/<project-id>.json` | JSON | Project resources and folder URIs. |
| Linux | user | `~/.gemini/config/projects/<project-id>.json` | JSON | Project resources and folder URIs. |
| Windows | user | `%USERPROFILE%\.gemini\config\projects\<project-id>.json` | JSON | Windows spelling of project resources. |
| macOS | user | `~/.gemini/antigravity-cli/keybindings.json` | JSON | CLI keybinding map. Changelog says it is created when the user customizes keybindings. |
| Linux | user | `~/.gemini/antigravity-cli/keybindings.json` | JSON | CLI keybinding map. |
| Windows | user | `%USERPROFILE%\.gemini\antigravity-cli\keybindings.json` | JSON | Windows spelling of CLI keybindings. |
| macOS | repo | `<repo>/.agents/` | directory | Workspace customization root. Alternatives: `.agent/`, `_agents/`, `_agent/`. |
| Linux | repo | `<repo>/.agents/` | directory | Workspace customization root. |
| Windows | repo | `<repo>\.agents\` | directory | Workspace customization root. |
| macOS | user | `~/.gemini/config/skills.json` | JSON | Optional explicit skills registry. |
| Linux | user | `~/.gemini/config/skills.json` | JSON | Optional explicit skills registry. |
| Windows | user | `%USERPROFILE%\.gemini\config\skills.json` | JSON | Windows spelling of explicit skills registry. |
| macOS | user | `~/.gemini/config/plugins.json` | JSON | Optional explicit plugin registry. |
| Linux | user | `~/.gemini/config/plugins.json` | JSON | Optional explicit plugin registry. |
| Windows | user | `%USERPROFILE%\.gemini\config\plugins.json` | JSON | Windows spelling of explicit plugin registry. |
| macOS | other | `~/.gemini/antigravity-cli/cache/projects.json` | JSON | Workspace-to-project cache. |
| Linux | other | `~/.gemini/antigravity-cli/cache/projects.json` | JSON | Workspace-to-project cache. |
| Windows | other | `%USERPROFILE%\.gemini\antigravity-cli\cache\projects.json` | JSON | Windows spelling of workspace-to-project cache. |

Local inspection also found CLI-side state files and directories under `/Users/ken/.claudine/.gemini/antigravity-cli/`: `conversation_summaries.db`, `jetski_state.pbtxt`, `last_check.timestamp`, `installation_id`, `cache/CHANGELOG.md`, `cache/projects.json`, `updater/update_status.json`, `cli.log`, `log/`, `conversations/`, `brain/`, `knowledge/`, and `builtin/skills/`.

Config side effects wrappers should expect:

- First interactive or print-mode runs may create or update CLI cache, logs, project cache, updater status, and conversation state under `~/.gemini/antigravity-cli/`.
- `agy install` can mutate shell profile files to add PATH and purge aliases unless `--skip-path` and `--skip-aliases` are used.
- Trust and permission prompts are interactive. Local `settings.json` included `trustedWorkspaces`, indicating trust state is persisted.
- Authentication is stored via OS keyring according to the README; unsigned-in CLI probes ask the user to launch the CLI without arguments to sign in.

## Environment Variables

General CLI/runtime variables found in changelog text or installed binary strings:

| Variable | Effect |
| --- | --- |
| `AGY_CLI_CMD_OUTPUT_PERCENTAGE` | Customizes the maximum height of command outputs in the TUI as a percentage of terminal height. |
| `AGY_CLI_DISABLE_LATEX` | Disables LaTeX/math rendering globally in the CLI. |
| `AGY_CLI_HIDE_ACCOUNT_INFO` | Hides account email and plan tier from the TUI header. |
| `AGY_CLI_DISABLE_AUTO_UPDATE` | Appears in the installed binary as an auto-update disable switch; exact accepted values were not documented in help output. |
| `AGY_CLI_FORCE_OSC8` | Appears in the installed binary and is consistent with forced OSC8 hyperlink rendering; exact accepted values were not documented in help output. |
| `AGY_CLI_EXPERIMENTAL_RENDERING` | Appears in the installed binary as a rendering feature toggle; exact accepted values were not documented in help output. |
| `AGY_CLI_LOGO_STYLE` | Appears in the installed binary as a logo style override; exact accepted values were not documented in help output. |
| `AGY_CLI_NEW_HARNESS` | Appears in the installed binary as a harness feature toggle; exact accepted values were not documented in help output. |

Variables for model endpoints, permissions, MCP, detailed logging, and streaming are intentionally left to their topic-specific research files.

## Machine Introspection

Antigravity CLI has limited machine-readable discovery in the public surface observed here.

| Command | Purpose | Machine-readable | Format | Useful for codegen | Notes |
| --- | --- | --- | --- | --- | --- |
| `agy --print "<prompt>" --output-format json` | Print-mode run result | Yes | JSON | No | Hidden from help but accepted. Returns `conversation_id`, `status`, `response`, `error`, `duration_seconds`, `num_turns`, and `usage`. Useful for wrappers, not provider metadata generation. |
| `agy models` | Model listing | No | Text | No | Requires sign-in. Local unsigned-in probe printed an error and exited 0. No JSON flag was discovered. |
| `agy plugin list` | Plugin listing | No | Text | No | Local empty-state output was `No imported plugins.` No JSON flag was discovered. |
| `agy changelog` | Release notes | No | Text | No | Prints human release notes from cache/upstream. Useful for diagnostics only. |

`--help` and `--version` are useful for smoke checks but are not included as machine introspection in the frontmatter because they do not expose structured provider state.

## Wrapper Notes

- Resolve the CLI binary as `agy` on macOS/Linux and `agy.exe` on Windows. Do not confuse the CLI with `/Applications/Antigravity.app` or `/Applications/Antigravity IDE.app`.
- Use `agy --print <prompt>` or `agy --prompt <prompt>` for non-interactive sessions. Plain `agy` is a TUI and may initiate browser or remote URL sign-in.
- Prefer `agy --print <prompt> --output-format json` for wrappers. This hidden flag returns structured JSON even for local auth failures.
- Do not trust exit code alone. Local unsigned-in `agy --print "Say OK" --print-timeout 3s` printed `Error: authentication failed or timed out` and exited 0. `agy models` also printed a sign-in error and exited 0.
- `--json`, `--format json`, and `--verbose` are rejected by installed 1.1.0.
- No direct system-prompt flag was discovered. Installed 1.1.0 rejected `--system-prompt`, `--append-system-prompt`, `--replace-system-prompt`, and `--instruction`.
- `agy plugin <subcommand> --help` attempted to open Bubble Tea and failed without `/dev/tty`; wrappers should not blindly recurse through help paths in a non-TTY.
- The CLI writes state under `~/.gemini/antigravity-cli/`, including logs, project caches, update status, conversations, and built-in skills.
- Installer scripts stage downloads under `$HOME/.cache/antigravity/staging` on Unix and `%LOCALAPPDATA%\antigravity\staging` on Windows.
- Windows CMD installer argument sanitization rejects command lines containing `&`, `|`, `;`, `<`, `>`, or `^`.

## Changelog

This is the initial Antigravity CLI research document for this topic. `changes` is `[]`.

## Sources

- [Antigravity CLI product page](https://antigravity.google/product/antigravity-cli)
- [Antigravity CLI repository](https://github.com/google-antigravity/antigravity-cli)
- [Antigravity CLI README](https://github.com/google-antigravity/antigravity-cli/blob/main/README.md)
- [Antigravity CLI install docs](https://antigravity.google/docs/cli-install)
- [Antigravity CLI reference docs](https://antigravity.google/docs/cli-reference)
- [Antigravity CLI latest GitHub release API](https://api.github.com/repos/google-antigravity/antigravity-cli/releases/latest)
- [Unix installer script](https://antigravity.google/cli/install.sh)
- [Windows PowerShell installer script](https://antigravity.google/cli/install.ps1)
- [Windows CMD installer script](https://antigravity.google/cli/install.cmd)
- [Google Developers Blog: Transitioning Gemini CLI to Antigravity CLI](https://developers.googleblog.com/an-important-update-transitioning-gemini-cli-to-antigravity-cli/)
- Local command: `/Users/ken/.local/bin/agy --version`
- Local command: `/Users/ken/.local/bin/agy --help`
- Local command: `/Users/ken/.local/bin/agy install --help`
- Local command: `/Users/ken/.local/bin/agy models --help`
- Local command: `/Users/ken/.local/bin/agy plugin --help`
- Local command: `/Users/ken/.local/bin/agy changelog --help`
- Local command: `/Users/ken/.local/bin/agy models`
- Local command: `/Users/ken/.local/bin/agy plugin list`
- Local command: `/Users/ken/.local/bin/agy --print "Say OK" --print-timeout 3s`
- Local command: `/Users/ken/.local/bin/agy --print "x" --output-format json`
- Local negative probes: `--json`, `--format json`, `--verbose`, `--system-prompt`, `--append-system-prompt`, `--replace-system-prompt`, and `--instruction`
- Local files inspected: `/Users/ken/.claudine/.gemini/antigravity-cli/`, `/Users/ken/.gemini/antigravity-cli/`, `/Users/ken/.gemini/config/`, and `/Users/ken/.antigravity/`
- Local file inspected: `/Users/ken/.claudine/.gemini/antigravity-cli/builtin/skills/agy-customizations/SKILL.md`
- Local file inspected: `/Users/ken/.claudine/.gemini/antigravity-cli/builtin/skills/agy-customizations/docs/json_configs.md`
- Local file inspected: `/Users/ken/.claudine/.gemini/antigravity-cli/builtin/skills/agy-customizations/docs/plugins.md`
- Local file inspected: `/Users/ken/.claudine/.gemini/antigravity-cli/builtin/skills/agy-customizations/docs/mcp_servers.md`
