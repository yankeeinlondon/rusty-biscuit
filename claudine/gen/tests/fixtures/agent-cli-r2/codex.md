---
$schema: ./_schema.yaml
schema_revision: 2
created: 2026-07-02
last_updated: 2026-07-03
agent: codex
model: default
latest_version: "0.142.5"
versions_examined:
  - "0.142.5"
evidence:
  - id: codex-cli-reference
    method: official_docs
    location: https://developers.openai.com/codex/cli/reference
    version: "0.142.5"
    observed_on: 2026-07-03
    claim: "Documents each switch spelling, its short form, and its value placeholder."
    limitations: "Does not state which attached value forms the parser accepts."
  - id: codex-clap-parser
    method: source_code
    location: https://github.com/openai/codex/tree/rust-v0.142.5/codex-rs/cli
    version: "0.142.5"
    observed_on: 2026-07-03
    claim: "The CLI parses switches with clap, which accepts a long value after = and a short value attached to its switch."
    limitations: "Read from the parser definition, not from a run of the binary."
homepage: https://developers.openai.com/codex/cli
repo: https://github.com/openai/codex
docs: https://developers.openai.com/codex/
cli_docs: https://developers.openai.com/codex/cli/reference
binaries:
  - os: macos
    binary: codex
    alt_binaries: []
    notes: "Official command name. Local macOS inspection found /Users/ken/.bun/bin/codex, managed by bun, dispatching to a darwin-arm64 native binary."
  - os: linux
    binary: codex
    alt_binaries: []
    notes: "Official command name. GitHub release archives contain platform-named executables such as codex-x86_64-unknown-linux-musl that users normally rename to codex."
  - os: windows
    binary: codex
    alt_binaries: ["codex.exe", "codex.cmd"]
    notes: "Official docs say to run codex natively in PowerShell. Native executable and package-manager shims were not locally inspected."
install_methods:
  - os: macos
    method: standalone_binary
    command: "curl -fsSL https://chatgpt.com/codex/install.sh | sh"
    notes: "Official standalone installer; rerun to upgrade. CODEX_INSTALL_DIR defaults to ~/.local/bin for macOS/Linux."
  - os: linux
    method: standalone_binary
    command: "curl -fsSL https://chatgpt.com/codex/install.sh | sh"
    notes: "Official standalone installer; rerun to upgrade. CODEX_INSTALL_DIR defaults to ~/.local/bin for macOS/Linux."
  - os: windows
    method: standalone_binary
    command: "powershell -ExecutionPolicy ByPass -c \"irm https://chatgpt.com/codex/install.ps1 | iex\""
    notes: "Official standalone installer. CODEX_INSTALL_DIR defaults to %LOCALAPPDATA%\\Programs\\OpenAI\\Codex\\bin."
  - os: macos
    method: npm
    command: "npm install -g @openai/codex"
    notes: "Official package-manager install. bun can also install the npm package; local install is bun-managed."
  - os: linux
    method: npm
    command: "npm install -g @openai/codex"
    notes: "Official package-manager install."
  - os: windows
    method: npm
    command: "npm install -g @openai/codex"
    notes: "Official package-manager install; expected to expose Windows command shims."
  - os: macos
    method: brew
    command: "brew install --cask codex"
    notes: "Official Homebrew cask install."
  - os: macos
    method: standalone_binary
    command: "download from https://github.com/openai/codex/releases/latest"
    notes: "Download the macOS Apple Silicon or Intel archive and rename the extracted platform-named binary to codex."
  - os: linux
    method: standalone_binary
    command: "download from https://github.com/openai/codex/releases/latest"
    notes: "Download the Linux x86_64 or arm64 archive and rename the extracted platform-named binary to codex."
  - os: windows
    method: standalone_binary
    command: "download from https://github.com/openai/codex/releases/latest"
    notes: "Release assets include Windows targets, but the exact local shim layout was not inspected."
subcommands:
  - name: interactive
    description: "Default mode when no subcommand is supplied; launches the terminal UI, optionally with an initial prompt."
    non_interactive: false
    notes: "Requires a TTY for normal use and prompts for first-run authentication."
  - name: exec
    description: "Runs Codex non-interactively and exits."
    non_interactive: true
    notes: "Alias: e. Reads prompt from argv, stdin, or '-' and can emit JSONL with --json."
  - name: exec resume
    description: "Continues a prior exec session non-interactively."
    non_interactive: true
    notes: "Use --last to avoid the picker; otherwise an omitted session can become interactive."
  - name: exec review
    description: "Runs the exec-mode reviewer against the current repository."
    non_interactive: true
    notes: "Supports review scope flags and JSONL output."
  - name: review
    description: "Runs a code review non-interactively."
    non_interactive: true
    notes: "Top-level review command for staged, uncommitted, base-branch, or commit review."
  - name: login
    description: "Manages authentication."
    non_interactive: false
    notes: "Default and device flows require user interaction; --with-api-key and --with-access-token read secrets from stdin."
  - name: login status
    description: "Shows login status."
    non_interactive: true
    notes: "Local help exposes text output only; doctor --json is better for machine-readable auth state."
  - name: logout
    description: "Removes stored authentication credentials."
    non_interactive: false
    notes: "Mutates CODEX_HOME auth state."
  - name: mcp
    description: "Manages external MCP servers."
    non_interactive: false
    notes: "list/get are scriptable with --json; add/remove mutate config; login/logout may require OAuth interaction."
  - name: plugin
    description: "Manages Codex plugins and marketplaces."
    non_interactive: false
    notes: "add/list/remove and marketplace operations support JSON on selected subcommands; add/remove mutate config and cache."
  - name: mcp-server
    description: "Starts Codex as an MCP server over stdio."
    non_interactive: true
    notes: "Intended for another agent or MCP client to consume Codex."
  - name: app-server
    description: "Runs the experimental local app server or related tooling."
    non_interactive: true
    notes: "Can listen on stdio, WebSocket, Unix socket, or off; daemon controls mutate app-server state."
  - name: remote-control
    description: "Manages the app-server daemon with remote control enabled."
    non_interactive: true
    notes: "start/stop support --json but may start or stop a background daemon."
  - name: app
    description: "Launches the Codex desktop app or opens the app installer if missing."
    non_interactive: false
    notes: "macOS/Windows desktop-oriented command; not useful for headless wrappers."
  - name: completion
    description: "Generates shell completion scripts."
    non_interactive: true
    notes: "Supported shells: bash, zsh, fish, powershell, and elvish."
  - name: update
    description: "Checks for and applies a Codex CLI update when supported."
    non_interactive: false
    notes: "Mutates the installation and can invoke package-manager update behavior."
  - name: doctor
    description: "Generates diagnostic reports for installation, config, auth, runtime, Git, terminal, app-server, and thread inventory."
    non_interactive: true
    notes: "Use --json for a redacted machine-readable report."
  - name: sandbox
    description: "Runs arbitrary commands inside a Codex-provided sandbox."
    non_interactive: true
    notes: "Platform behavior differs: macOS Seatbelt, Linux Landlock/seccomp, Windows native sandbox."
  - name: debug
    description: "Debugging tools."
    non_interactive: true
    notes: "debug models prints the raw model catalog as JSON; debug app-server sends app-server test messages."
  - name: apply
    description: "Applies the latest diff produced by a Codex Cloud task to the local working tree."
    non_interactive: true
    notes: "Alias: a. Mutates the working tree."
  - name: resume
    description: "Resumes a previous interactive session."
    non_interactive: false
    notes: "Picker by default; --last avoids the picker but still launches interactive TUI mode."
  - name: archive
    description: "Archives a saved session by id or session name."
    non_interactive: true
    notes: "Mutates saved session state."
  - name: delete
    description: "Permanently deletes a saved session by id or session name."
    non_interactive: true
    notes: "--force avoids prompting, but only when SESSION is a UUID."
  - name: unarchive
    description: "Restores an archived session by id or session name."
    non_interactive: true
    notes: "Mutates saved session state."
  - name: fork
    description: "Forks a previous interactive session into a new thread."
    non_interactive: false
    notes: "Picker by default; --last avoids the picker but still launches interactive TUI mode."
  - name: cloud
    description: "Browses or executes Codex Cloud tasks from the terminal."
    non_interactive: true
    notes: "cloud exec submits directly; cloud list supports --json; apply mutates the working tree."
  - name: exec-server
    description: "Runs the experimental standalone exec-server service."
    non_interactive: true
    notes: "Can listen on WebSocket or stdio and can register as a remote environment."
  - name: features
    description: "Lists or mutates feature flags."
    non_interactive: true
    notes: "list is read-only text; enable/disable persist changes in config.toml."
  - name: execpolicy
    description: "Evaluates execpolicy rule files against command tokens."
    non_interactive: true
    notes: "Accepted by local 0.142.5 and documented, but omitted from local top-level help; check emits JSON."
cli_switches:
  - flag: --config
    aliases: ["-c"]
    value: "<key=value>"
    value_type: string
    value_optional: false
    attachment: [space, equals, short_attached]
    invocation_scope:
      - applies_to: global
    scope: ["global", "config"]
    description: "Override a configuration value for this invocation; dotted paths are supported and values parse as TOML when possible."
    example: "codex -c model='gpt-5.5'"
    notes: "Overrides take precedence over config.toml."
    evidence_ids: [codex-cli-reference, codex-clap-parser]
  - flag: --image
    aliases: ["-i"]
    value: "<FILE>..."
    value_type: variadic
    variadic_min: 1
    attachment: [space, equals, short_attached]
    invocation_scope:
      - applies_to: command
        command: []
      - applies_to: command
        command: [exec]
    scope: ["interactive", "exec", "input"]
    description: "Attach one or more image files to the initial prompt."
    example: "codex -i shot.png diagram.png 'explain these'"
    evidence_ids: [codex-cli-reference, codex-clap-parser]
  - flag: --model
    aliases: ["-m"]
    value: "<MODEL>"
    value_type: string
    value_optional: false
    attachment: [space, equals, short_attached]
    invocation_scope:
      - applies_to: command
        command: []
      - applies_to: command
        command: [exec]
    scope: ["interactive", "exec", "model_selection"]
    description: "Select the model for this session."
    example: "codex exec -m gpt-5.5 'summarize'"
    evidence_ids: [codex-cli-reference, codex-clap-parser]
  - flag: --oss
    value_type: none
    attachment: []
    invocation_scope:
      - applies_to: command
        command: []
      - applies_to: command
        command: [exec]
    scope: ["interactive", "exec", "model_selection"]
    description: "Use a local open-source model provider."
    example: "codex --oss"
    evidence_ids: [codex-cli-reference]
  - flag: --json
    value_type: none
    attachment: []
    invocation_scope:
      - applies_to: command
        command: [exec]
    scope: ["exec", "output"]
    description: "Print events to stdout as JSON Lines."
    example: "codex exec --json 'summarize'"
    evidence_ids: [codex-cli-reference]
  - flag: --remote
    value: "<ADDR>"
    value_type: unknown
    attachment: []
    invocation_scope:
      - applies_to: command
        command: []
    scope: ["interactive", "remote"]
    description: "Connect the TUI to a remote app-server endpoint."
    example: "codex --remote ws://127.0.0.1:1455"
    evidence_ids: [codex-cli-reference]
    gap: "The reference shows a placeholder but not whether the address may be omitted; run codex --remote with no value to check."
config_paths:
  - os: macos
    scope: user
    path: "$CODEX_HOME/config.toml; default /Users/<user>/.codex/config.toml"
    format: toml
    notes: "Primary durable user config. Local wrapper environment used /Users/ken/.claudine/.codex/config.toml, symlinked to /Users/ken/.codex/config.toml."
  - os: linux
    scope: user
    path: "$CODEX_HOME/config.toml; default /home/<user>/.codex/config.toml"
    format: toml
    notes: "Primary durable user config."
  - os: windows
    scope: user
    path: "%USERPROFILE%\\.codex\\config.toml or %CODEX_HOME%\\config.toml"
    format: toml
    notes: "Primary durable user config; exact default expansion on Windows was not locally inspected."
  - os: macos
    scope: user
    path: "$CODEX_HOME/<profile-name>.config.toml"
    format: toml
    notes: "Profile layer selected with --profile/-p."
  - os: linux
    scope: user
    path: "$CODEX_HOME/<profile-name>.config.toml"
    format: toml
    notes: "Profile layer selected with --profile/-p."
  - os: windows
    scope: user
    path: "%CODEX_HOME%\\<profile-name>.config.toml"
    format: toml
    notes: "Profile layer selected with --profile/-p."
  - os: macos
    scope: repo
    path: ".codex/config.toml"
    format: toml
    notes: "Project-scoped override loaded only for trusted projects."
  - os: linux
    scope: repo
    path: ".codex/config.toml"
    format: toml
    notes: "Project-scoped override loaded only for trusted projects."
  - os: windows
    scope: repo
    path: ".codex\\config.toml"
    format: toml
    notes: "Project-scoped override loaded only for trusted projects."
  - os: macos
    scope: system
    path: "/etc/codex/config.toml"
    format: toml
    notes: "Official config precedence lists this Unix system config if present."
  - os: linux
    scope: system
    path: "/etc/codex/config.toml"
    format: toml
    notes: "Official config precedence lists this Unix system config if present."
  - os: windows
    scope: system
    path: "unknown"
    format: toml
    notes: "Official config basics page cites Unix /etc path only; Windows system config path was not verified."
  - os: macos
    scope: user
    path: "$CODEX_HOME/auth.json"
    format: json
    notes: "Stored authentication state; not a normal user-edited config file."
  - os: linux
    scope: user
    path: "$CODEX_HOME/auth.json"
    format: json
    notes: "Stored authentication state; not a normal user-edited config file."
  - os: windows
    scope: user
    path: "%CODEX_HOME%\\auth.json"
    format: json
    notes: "Stored authentication state; not a normal user-edited config file."
  - os: macos
    scope: user
    path: "$CODEX_HOME/rules/default.rules"
    format: other
    notes: "User execpolicy rules in Starlark syntax."
  - os: linux
    scope: user
    path: "$CODEX_HOME/rules/default.rules"
    format: other
    notes: "User execpolicy rules in Starlark syntax."
  - os: windows
    scope: user
    path: "%CODEX_HOME%\\rules\\default.rules"
    format: other
    notes: "User execpolicy rules in Starlark syntax."
  - os: macos
    scope: repo
    path: ".codex/rules/"
    format: other
    notes: "Project execpolicy rules directory; ignored for untrusted projects."
  - os: linux
    scope: repo
    path: ".codex/rules/"
    format: other
    notes: "Project execpolicy rules directory; ignored for untrusted projects."
  - os: windows
    scope: repo
    path: ".codex\\rules\\"
    format: other
    notes: "Project execpolicy rules directory; ignored for untrusted projects."
  - os: macos
    scope: user
    path: "$CODEX_HOME/AGENTS.override.md or $CODEX_HOME/AGENTS.md"
    format: text
    notes: "Global instruction files; override wins over AGENTS.md."
  - os: linux
    scope: user
    path: "$CODEX_HOME/AGENTS.override.md or $CODEX_HOME/AGENTS.md"
    format: text
    notes: "Global instruction files; override wins over AGENTS.md."
  - os: windows
    scope: user
    path: "%CODEX_HOME%\\AGENTS.override.md or %CODEX_HOME%\\AGENTS.md"
    format: text
    notes: "Global instruction files; override wins over AGENTS.md."
  - os: macos
    scope: repo
    path: "AGENTS.override.md, AGENTS.md, or configured fallback filenames along the project path"
    format: text
    notes: "Project instruction discovery walks from project root to cwd and includes at most one instruction file per directory."
  - os: linux
    scope: repo
    path: "AGENTS.override.md, AGENTS.md, or configured fallback filenames along the project path"
    format: text
    notes: "Project instruction discovery walks from project root to cwd and includes at most one instruction file per directory."
  - os: windows
    scope: repo
    path: "AGENTS.override.md, AGENTS.md, or configured fallback filenames along the project path"
    format: text
    notes: "Project instruction discovery walks from project root to cwd and includes at most one instruction file per directory."
  - os: macos
    scope: user
    path: "$CODEX_HOME/agents/*.toml"
    format: toml
    notes: "Custom subagent definitions."
  - os: linux
    scope: user
    path: "$CODEX_HOME/agents/*.toml"
    format: toml
    notes: "Custom subagent definitions."
  - os: windows
    scope: user
    path: "%CODEX_HOME%\\agents\\*.toml"
    format: toml
    notes: "Custom subagent definitions."
  - os: macos
    scope: repo
    path: ".codex/agents/*.toml"
    format: toml
    notes: "Project-scoped custom subagent definitions."
  - os: linux
    scope: repo
    path: ".codex/agents/*.toml"
    format: toml
    notes: "Project-scoped custom subagent definitions."
  - os: windows
    scope: repo
    path: ".codex\\agents\\*.toml"
    format: toml
    notes: "Project-scoped custom subagent definitions."
  - os: macos
    scope: user
    path: "$CODEX_HOME/prompts/*.md"
    format: text
    notes: "Deprecated custom prompt files invoked as slash commands."
  - os: linux
    scope: user
    path: "$CODEX_HOME/prompts/*.md"
    format: text
    notes: "Deprecated custom prompt files invoked as slash commands."
  - os: windows
    scope: user
    path: "%CODEX_HOME%\\prompts\\*.md"
    format: text
    notes: "Deprecated custom prompt files invoked as slash commands."
  - os: macos
    scope: user
    path: "$CODEX_HOME/state_5.sqlite, logs_2.sqlite, memories_1.sqlite, goals_1.sqlite"
    format: other
    notes: "Observed local SQLite-backed state files. CODEX_SQLITE_HOME or sqlite_home can move SQLite state."
  - os: linux
    scope: user
    path: "$CODEX_HOME/state_5.sqlite, logs_2.sqlite, memories_1.sqlite, goals_1.sqlite"
    format: other
    notes: "Observed local SQLite-backed state file names; versioned names may change."
  - os: windows
    scope: user
    path: "%CODEX_HOME%\\state_5.sqlite, logs_2.sqlite, memories_1.sqlite, goals_1.sqlite"
    format: other
    notes: "Observed local SQLite-backed state names on macOS; Windows names were not locally inspected."
env_vars:
  - name: CODEX_HOME
    effect: "Sets the root for Codex state, including config, auth, logs, sessions, skills, and standalone package metadata. If set, the directory must already exist."
  - name: CODEX_SQLITE_HOME
    effect: "Sets where SQLite-backed state is stored. The sqlite_home config option takes precedence; relative paths resolve from the current working directory."
  - name: CODEX_NON_INTERACTIVE
    effect: "For standalone install scripts, 1/true/yes skips installer prompts and accepts defaults."
  - name: CODEX_INSTALL_DIR
    effect: "For standalone installers, changes where the visible codex command is installed; defaults to ~/.local/bin on macOS/Linux and %LOCALAPPDATA%\\Programs\\OpenAI\\Codex\\bin on Windows."
  - name: CODEX_API_KEY
    effect: "Provides an API key for a single codex exec run; official docs recommend setting it inline rather than job-wide when running repository-controlled code."
  - name: CODEX_ACCESS_TOKEN
    effect: "Provides a ChatGPT or Codex access token for trusted automation; can also be piped to codex login --with-access-token for persisted login."
  - name: CODEX_CA_CERTIFICATE
    effect: "Points HTTPS, login, and WebSocket clients at a PEM CA bundle; takes precedence over SSL_CERT_FILE."
  - name: SSL_CERT_FILE
    effect: "Fallback PEM CA bundle path for HTTPS, login, and WebSocket clients when CODEX_CA_CERTIFICATE is unset."
  - name: RUST_LOG
    effect: "Controls Rust log filtering and verbosity. codex exec defaults to error output unless a more verbose value is set."
machine_introspection:
  - command: "codex doctor --json"
    purpose: doctor
    machine_readable: true
    output_format: json
    useful_for_codegen: true
    notes: "Redacted diagnostic report with schemaVersion, codexVersion, install paths, CODEX_HOME, config path, auth mode, feature flags, model/provider, MCP server count, runtime, Git, terminal, app-server, update status, and state DB checks."
  - command: "codex debug models [--bundled]"
    purpose: models
    machine_readable: true
    output_format: json
    useful_for_codegen: true
    notes: "Raw model catalog. Local --bundled output included model slugs, display names, reasoning levels, service tiers, shell type, visibility, API support, and embedded instruction metadata."
  - command: "codex mcp list --json"
    purpose: mcp
    machine_readable: true
    output_format: json
    useful_for_codegen: true
    notes: "Lists configured MCP servers with name, enabled state, transport, env token references, timeouts, and auth_status. Local output showed one streamable HTTP github server."
  - command: "codex mcp get --json <name>"
    purpose: mcp
    machine_readable: true
    output_format: json
    useful_for_codegen: true
    notes: "Shows one raw MCP server configuration."
  - command: "codex plugin list --json [--available]"
    purpose: plugins
    machine_readable: true
    output_format: json
    useful_for_codegen: true
    notes: "Lists installed and available plugins. Local output showed gmail@openai-curated and github@openai-curated installed and enabled."
  - command: "codex plugin add --json <plugin> and codex plugin remove --json <plugin>"
    purpose: plugins
    machine_readable: true
    output_format: json
    useful_for_codegen: false
    notes: "Machine-readable mutation result; useful for wrapper UX but mutates local config/cache."
  - command: "codex plugin marketplace list --json"
    purpose: plugins
    machine_readable: true
    output_format: json
    useful_for_codegen: true
    notes: "Official reference documents JSON output for marketplace source inventory."
  - command: "codex features list"
    purpose: capabilities
    machine_readable: false
    output_format: table
    useful_for_codegen: true
    notes: "Text table of feature key, stage, and effective state. Useful but requires parsing; no --json in local 0.142.5 help."
  - command: "codex app-server generate-json-schema --out <dir> [--experimental]"
    purpose: config_schema
    machine_readable: true
    output_format: json
    useful_for_codegen: true
    notes: "Generates app-server protocol JSON Schema bundles to a directory."
  - command: "codex app-server generate-ts --out <dir> [--experimental]"
    purpose: other
    machine_readable: true
    output_format: text
    useful_for_codegen: true
    notes: "Generates TypeScript protocol bindings; not introspection of user state."
  - command: "codex app-server daemon version"
    purpose: version
    machine_readable: true
    output_format: json
    useful_for_codegen: false
    notes: "Local help says it prints local CLI and running app-server versions as JSON."
  - command: "codex remote-control start --json and codex remote-control stop --json"
    purpose: other
    machine_readable: true
    output_format: json
    useful_for_codegen: false
    notes: "Machine-readable daemon control results; mutates daemon state."
  - command: "codex cloud list --json"
    purpose: other
    machine_readable: true
    output_format: json
    useful_for_codegen: false
    notes: "Lists Codex Cloud tasks with task metadata and cursor. Requires cloud auth/state."
  - command: "codex execpolicy check --rules <file> [--pretty] -- <command>..."
    purpose: tools
    machine_readable: true
    output_format: json
    useful_for_codegen: true
    notes: "Evaluates rule files and emits the strictest decision and matching rules. Useful for PolicyEngine comparison."
wrapper_notes:
  - "Use codex exec as the primary non-interactive automation entry point. Default codex, resume, fork, login, app, and most OAuth flows are interactive or desktop/browser oriented."
  - "Prefer local help over docs for argv accepted by the installed binary. Local 0.142.5 accepts execpolicy but omits it from top-level help; docs include it. Local help omits documented --full-auto and --experimental-json."
  - "For machine output, codex exec --json emits JSONL on stdout. Pair it with --output-last-message when a wrapper needs both event streaming and the final assistant text."
  - "codex exec reads stdin when the prompt is omitted or set to '-'. If stdin is piped and a prompt argument is also supplied, Codex appends stdin as a <stdin> context block."
  - "First run can prompt for auth. For non-interactive persisted login, pipe secrets into codex login --with-api-key or codex login --with-access-token; for one-shot exec API auth, use CODEX_API_KEY."
  - "CODEX_HOME is a major wrapper lever. It controls config, auth, logs, sessions, skills, plugin cache, standalone package metadata, and observed SQLite state. The directory must already exist when overridden."
  - "Local inspection was inside a wrapped environment where CODEX_HOME was /Users/ken/.claudine/.codex and many entries were symlinks to /Users/ken/.codex. Do not assume ~/.codex is the only physical state root."
  - "Project .codex/config.toml, .codex/rules, hooks, and project AGENTS files are loaded only for trusted projects. Trust state is stored in config.toml under [projects.<path>]."
  - "Config precedence is CLI flags and -c overrides, trusted project .codex/config.toml layers, selected profile file, user config, Unix system config, then built-ins."
  - "Project-local config cannot override selected machine-local provider, auth, app request metadata, notification, profile selection, or telemetry routing keys; wrappers should put those in user config or -c overrides."
  - "No dedicated system-prompt CLI flags were found in local 0.142.5 help. The wrapper-relevant instruction surfaces are -c developer_instructions=..., -c model_instructions_file=..., and AGENTS.md discovery; semantics belong to the sibling system-prompt topic."
  - "Use --ephemeral for exec runs that should avoid persisted session files, but auth and other CODEX_HOME state may still be read."
  - "Use --ignore-user-config and --ignore-rules for controlled automation where inherited user config or execpolicy would make behavior non-deterministic."
  - "Use --dangerously-bypass-approvals-and-sandbox or --yolo only inside an external sandbox. The flag disables Codex's approval and sandbox safety rails."
  - "codex doctor --json is the best single probe for install provenance, update status, effective CODEX_HOME, auth mode, model/provider, feature flags, and state integrity."
  - "codex features list is useful but text-only in local 0.142.5; wrappers must parse a fixed-width table or avoid depending on it."
  - "debug models emits large JSON and may include embedded instruction text. Treat it as sensitive diagnostic/model metadata, not a casual log payload."
  - "codex sandbox has OS-specific behavior. Local macOS help exposes --allow-unix-socket and --log-denials; Linux and Windows flags should be inspected on those platforms before hard-coding."
  - "Plugin and MCP commands can mutate config/cache or start OAuth flows. Use list/get JSON commands for read-only discovery."
changes:
  - "Refreshed verification date to 2026-07-03 and revalidated installed codex-cli 0.142.5 against npm latest and GitHub stable release metadata; alpha prereleases are newer but not the npm latest tag."
  - "Expanded subcommand inventory to include exec resume, exec review, login status, execpolicy, and app-server/debug/plugin/cloud subordinate automation surfaces."
  - "Recorded that execpolicy is accepted and documented but omitted from local top-level help."
  - "Updated CLI switches with documented-but-hidden --full-auto and --experimental-json, exec resume/review flags, delete --force, app --download-url, app-server generation flags, cloud flags, execpolicy flags, and config-based instruction surfaces."
  - "Reworked config discovery into per-OS records required by the schema and added system config, AGENTS discovery, custom agents, prompts, rules, and observed SQLite state."
  - "Updated environment variables from official environment-variable docs, including CODEX_SQLITE_HOME, installer variables, TLS certificate variables, and RUST_LOG behavior."
  - "Expanded machine introspection with doctor --json, debug models, MCP/plugin JSON, app-server schema/binding generation, app-server daemon version, cloud list, and execpolicy check."
requires_claudine_update: true
reason: "Claudine provider metadata should account for the newly verified execpolicy command, exec resume/review automation surfaces, per-OS config path records, documented hidden exec flags, and config-based instruction delivery surfaces."
---
# Agent CLI research fixture for Codex, revision 2

Test fixture for `claudine-gen`: the committed `docs/research/agent-cli/codex.md`
frontmatter with its switch inventory narrowed to six records typed under
contract revision 2. It is not research; do not cite it.
