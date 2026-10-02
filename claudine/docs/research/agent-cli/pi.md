---
$schema: ./_schema.yaml
schema_revision: 2
provider: pi
created: 2026-07-02
last_updated: 2026-10-01
agent: claude
model: sonnet
reasoning_effort: high
latest_version: 1.0.0
versions_examined:
- 0.87.1
- 1.0.0
evidence:
- claim: The latest dist-tag is 1.0.0, published 2026-10-01; 0.87.1 was published 2026-09-22, and 0.99.0 through 0.99.2 followed on 2026-09-29 and 2026-09-30.
  id: pi-npm-registry
  limitations: Reports the registry on the research date only.
  location: npm view @earendil-works/pi-coding-agent version dist-tags time --json
  method: local_inspection
  observed_on: 2026-10-01
  version: 1.0.0
- claim: The v1.0.0 release ships standalone archives for darwin, linux, and windows on x64 and arm64 besides the npm package.
  id: pi-github-release-1-0-0
  limitations: Asset names only; the archives were inspected separately.
  location: https://github.com/earendil-works/pi/releases/tag/v1.0.0
  method: official_docs
  observed_on: 2026-10-01
  version: 1.0.0
- claim: The macOS archive contains an executable named pi and the Windows archive contains pi.exe, each beside the docs, examples, and a native prebuild.
  id: pi-release-archives-1-0-0
  limitations: Archive listings only; the standalone executables were not run.
  location: tar tzf pi-darwin-arm64.tar.gz; unzip -l pi-windows-x64.zip (GitHub release v1.0.0 assets)
  method: local_inspection
  observed_on: 2026-10-01
  version: 1.0.0
- claim: Every root switch is matched by exact string equality, so only the space-separated form is recognized; the declarations fix which switches take a value, which are repeatable, and how a missing value is handled.
  id: pi-args-source-1-0-0
  limitations: Reads the unbundled source of the published package; extension-registered flags are not part of it.
  location: npm package @earendil-works/pi-coding-agent@1.0.0, dist/cli/args.js (parseArgs, a hand-written loop with no parsing library)
  method: source_code
  observed_on: 2026-10-01
  version: 1.0.0
- claim: 'The 0.87.1 parser declares the same switches as 1.0.0; only help text differs (the --provider description, the --tui-mode default, and builtin: extension names).'
  id: pi-args-source-0-87-1
  limitations: Reads the installed package; says nothing about older or newer releases.
  location: ~/.bun/install/global/node_modules/@earendil-works/pi-coding-agent/dist/cli/args.js (installed 0.87.1)
  method: source_code
  observed_on: 2026-10-01
  version: 0.87.1
- claim: Dispatch order is package commands, config, mcp, then the root parser; stdin or stdout that is not a terminal selects print mode; --export reads its output path from the first positional message; --offline is detected anywhere in argv.
  id: pi-main-source-1-0-0
  limitations: Source reading; no model session was started.
  location: npm package @earendil-works/pi-coding-agent@1.0.0, dist/main.js (main, resolveAppMode, runAuthCommand, --export handling, offline detection)
  method: source_code
  observed_on: 2026-10-01
  version: 1.0.0
- claim: install, remove, uninstall, update, list, and config accept only the listed switches in space-separated form, and --extension on update requires a next token that does not start with a dash.
  id: pi-pkg-source-1-0-0
  limitations: Source reading; install and update network behavior was not exercised.
  location: npm package @earendil-works/pi-coding-agent@1.0.0, dist/package-manager-cli.js (parsePackageCommand, handleConfigCommand)
  method: source_code
  observed_on: 2026-10-01
  version: 1.0.0
- claim: auth check, auth print-api-key, and auth print-bearer-token accept --provider and --model through the root parser, plus --json, --credentials, --no-refresh, and --min-expiry in space-separated form.
  id: pi-auth-source-1-0-0
  limitations: Source reading; credential printing was not run against real credentials.
  location: npm package @earendil-works/pi-coding-agent@1.0.0, dist/cli/auth-command.js and dist/cli/auth-check.js
  method: source_code
  observed_on: 2026-10-01
  version: 1.0.0
- claim: pi mcp declares each option as a flag, a single value, or a repeatable value, maps -l to --local, and accepts values only as the next argument.
  id: pi-mcp-source-1-0-0
  limitations: Source reading; pi mcp login was not run because it opens a browser.
  location: npm package @earendil-works/pi-coding-agent@1.0.0, dist/extensions/mcp/cli.js (parseOptions, add, remove, runMcpCommand)
  method: source_code
  observed_on: 2026-10-01
  version: 1.0.0
- claim: The installed binary reports 0.87.1 and lists every root switch with its short spellings and the install, remove, uninstall, update, list, config, and auth commands.
  id: pi-help-0-87-1
  limitations: Help text is a summary; the parser source decides value consumption.
  location: pi --version; pi --help (installed binary, 0.87.1)
  method: local_inspection
  observed_on: 2026-10-01
  version: 0.87.1
- claim: The 1.0.0 package reports 1.0.0 and lists the same root switches plus the mcp command and its options.
  id: pi-help-1-0-0
  limitations: Help text is a summary; the parser source decides value consumption.
  location: node dist/bundle/cli.js --version; --help; mcp --help (unpacked 1.0.0 package)
  method: local_inspection
  observed_on: 2026-10-01
  version: 1.0.0
- claim: Pairing every root switch with a following --version showed that each value switch consumes the next token even when it starts with a dash and no boolean switch does; --mode, --thinking, and --tui-mode rejected or warned about an invalid value with a message naming the switch; --name=foo, --mode=json, --thinking=high, --tui-mode=regular, and --list-models=x were not read as the known switch; -tread was rejected as an unknown option; package and auth commands rejected equals forms and missing values with errors naming the switch.
  id: pi-parse-tests-0-87-1
  limitations: Error and warning messages prove parsing; boolean switches are proven only by the absence of consumption. Nothing was sent to a model.
  location: Throwaway runs of the installed 0.87.1 binary under env -i with a temporary PI_CODING_AGENT_DIR, PI_OFFLINE=1, stdin from /dev/null, and no provider credentials
  method: disposable_test
  observed_on: 2026-10-01
  version: 0.87.1
- claim: The same value-consumption and equals-form results as 0.87.1 held; --provider without --model now fails with an error; pi mcp rejected --url=, --env=, and --json=1, rejected a value-less --url and --timeout, and accepted space-separated options and the -- command separator; mcp add rejected --env with --url.
  id: pi-parse-tests-1-0-0
  limitations: pi mcp login was not run. The --extension runs failed to load a module in the unbundled package, which still proves the value was consumed.
  location: Throwaway runs of the unpacked 1.0.0 bundle (node dist/bundle/cli.js) under the same isolated environment, including pi mcp add, remove, and list against a temporary agent directory
  method: disposable_test
  observed_on: 2026-10-01
  version: 1.0.0
- claim: Documents invocation modes, package commands, credential commands, MCP commands, the -- separator, and --export <input> [output].
  id: pi-docs-cli-1-0-0
  limitations: Does not state which forms of a value the parser accepts.
  location: https://github.com/earendil-works/pi/blob/main/packages/coding-agent/docs/cli.md
  method: official_docs
  observed_on: 2026-10-01
  version: 1.0.0
- claim: Lists the process-configuration variables, the AI_AGENT and PI_CODING_AGENT process markers, and the session variables injected into shell tools.
  id: pi-docs-env-1-0-0
  limitations: Provider API-key variables are documented elsewhere and recorded in model-config.
  location: https://github.com/earendil-works/pi/blob/main/packages/coding-agent/docs/environment-variables.md
  method: official_docs
  observed_on: 2026-10-01
  version: 1.0.0
- claim: Lists the files under the agent directory and the project .pi directory and the context-file discovery rules.
  id: pi-docs-config-1-0-0
  limitations: Does not describe trust.json, which docs/security.md names.
  location: https://github.com/earendil-works/pi/blob/main/packages/coding-agent/docs/configuration.md
  method: official_docs
  observed_on: 2026-10-01
  version: 1.0.0
- claim: Names ~/.pi/agent/trust.json as the saved project trust decisions.
  id: pi-docs-security-1-0-0
  limitations: Names the file only.
  location: https://github.com/earendil-works/pi/blob/main/packages/coding-agent/docs/security.md
  method: official_docs
  observed_on: 2026-10-01
  version: 1.0.0
- claim: The homepage documents the curl installer, the PowerShell installer, and npm, pnpm, and bun global installs; the quickstart requires Node.js 22.19 or newer for npm.
  id: pi-docs-install-1-0-0
  limitations: The installer scripts were read, not run.
  location: https://pi.dev/ and https://github.com/earendil-works/pi/blob/main/packages/coding-agent/docs/quickstart.md
  method: official_docs
  observed_on: 2026-10-01
  version: 1.0.0
- claim: Native Windows runs Bash tools through shellPath, Git Bash, or bash.exe on PATH.
  id: pi-docs-windows-1-0-0
  limitations: Does not name the Windows executable.
  location: https://github.com/earendil-works/pi/blob/main/packages/coding-agent/docs/windows.md
  method: official_docs
  observed_on: 2026-10-01
  version: 1.0.0
- claim: 1.0.0 made the TUI fullscreen by default and made --provider without --model an error; 0.99.0 added built-in MCP support with pi mcp; auth check and the credential printing commands arrived before 0.87.1.
  id: pi-changelog-1-0-0
  limitations: Release notes are authored by the maintainers and were checked against the source.
  location: https://github.com/earendil-works/pi/blob/main/packages/coding-agent/CHANGELOG.md
  method: official_docs
  observed_on: 2026-10-01
  version: 1.0.0
homepage: https://pi.dev/
repo: https://github.com/earendil-works/pi
docs: https://pi.dev/docs/latest
cli_docs: https://github.com/earendil-works/pi/blob/main/packages/coding-agent/docs/cli.md
binaries:
- binary: pi
  notes: npm and Bun global installs link a pi shim to dist/bundle/cli.js, a node script; this host has /Users/ken/.bun/bin/pi at 0.87.1. The 1.0.0 release archive contains a standalone pi executable.
  os: macos
- binary: pi
  notes: Same npm shim; the release archives for linux x64 and arm64 ship a standalone pi executable.
  os: linux
- alt_binaries:
  - pi.cmd
  - pi.ps1
  - pi.exe
  binary: pi
  notes: npm-compatible global installs create pi.cmd and pi.ps1 shims; the release archive pi-windows-x64.zip contains pi.exe.
  os: windows
install_methods:
- command: npm install -g --ignore-scripts @earendil-works/pi-coding-agent
  method: npm
  notes: Requires Node.js 22.19 or newer; Pi needs no dependency lifecycle scripts.
  os: macos
- command: npm install -g --ignore-scripts @earendil-works/pi-coding-agent
  method: npm
  notes: Requires Node.js 22.19 or newer; Pi needs no dependency lifecycle scripts.
  os: linux
- command: npm install -g --ignore-scripts @earendil-works/pi-coding-agent
  method: npm
  notes: Requires Node.js 22.19 or newer; Pi needs no dependency lifecycle scripts.
  os: windows
- command: curl -fsSL https://pi.dev/install.sh | sh
  method: standalone_binary
  notes: 'Homepage installer: a managed, npm-based installation that may install Node.js itself; release archives named pi-darwin-*.tar.gz and pi-linux-*.tar.gz are the other standalone route.'
  os: macos
- command: curl -fsSL https://pi.dev/install.sh | sh
  method: standalone_binary
  notes: 'Homepage installer: a managed, npm-based installation that may install Node.js itself; release archives named pi-darwin-*.tar.gz and pi-linux-*.tar.gz are the other standalone route.'
  os: linux
- command: powershell -c "irm https://pi.dev/install.ps1 | iex"
  method: other
  notes: Homepage PowerShell installer.
  os: windows
- method: standalone_binary
  notes: Unzip pi-windows-x64.zip or pi-windows-arm64.zip from the GitHub release; it contains pi.exe.
  os: windows
- command: pnpm add -g --ignore-scripts @earendil-works/pi-coding-agent
  method: other
  notes: Documented on the homepage.
  os: macos
- command: bun add -g --ignore-scripts @earendil-works/pi-coding-agent
  method: other
  notes: Documented on the homepage; this host's install.
  os: macos
- command: pnpm add -g --ignore-scripts @earendil-works/pi-coding-agent
  method: other
  notes: Documented on the homepage.
  os: linux
- command: bun add -g --ignore-scripts @earendil-works/pi-coding-agent
  method: other
  notes: Documented on the homepage; this host's install.
  os: linux
- command: pnpm add -g --ignore-scripts @earendil-works/pi-coding-agent
  method: other
  notes: Documented on the homepage.
  os: windows
- command: bun add -g --ignore-scripts @earendil-works/pi-coding-agent
  method: other
  notes: Documented on the homepage; this host's install.
  os: windows
subcommands:
- description: Installs an extension package source and adds it to settings.
  name: install
  non_interactive: true
  notes: Sources are npm:, git:, https, ssh, or a local path; runs package-manager commands and writes user or project settings. In a terminal with an undecided project trust it may prompt; a run with no terminal never prompts.
- description: Removes a package source from settings.
  name: remove
  non_interactive: true
  notes: Same trust behavior as install.
- description: Alias for remove.
  name: uninstall
  non_interactive: true
  notes: Removes package configuration only; it does not uninstall Pi itself.
- description: Updates Pi itself, installed packages, one package source, or model catalogs.
  name: update
  non_interactive: true
  notes: Defaults to Pi itself; self and pi are accepted as positional aliases. Never prompts for project trust and uses only the saved decision or --approve. Runs the package manager and the network.
- description: Lists installed packages from user and project settings.
  name: list
  non_interactive: true
  notes: Plain text only; no JSON switch.
- description: Opens a terminal UI to enable or disable package resources.
  name: config
  non_interactive: false
  notes: Requires a terminal; accepts -l/--local, --approve, and --no-approve.
- description: Prints usage for the credential commands.
  name: auth
  non_interactive: true
  notes: 'Group command: with no subcommand or with help it prints usage and exits 0.'
- description: Reports whether a provider or model has usable credentials.
  name: auth check
  non_interactive: true
  notes: Prints ready, not_ready, or invalid and exits 0, 1, or 2; --json gives a structured result. May refresh OAuth credentials over the network unless --no-refresh.
- description: Prints the resolved API key for a provider or model.
  name: auth print-api-key
  non_interactive: true
  notes: Prints a secret to stdout.
- description: Prints a resolved OAuth bearer token, refreshing it when expired.
  name: auth print-bearer-token
  non_interactive: true
  notes: Prints a secret to stdout; times out after 15 seconds.
- description: Prints usage for the MCP commands.
  name: mcp
  non_interactive: true
  notes: 'Group command: with no subcommand or with help it prints usage and exits 0. Added in 0.99.0.'
- description: Adds or replaces an MCP server in mcp.json.
  name: mcp add
  non_interactive: true
  notes: Stdio form is mcp add <server> [options] -- <command> [args...]; URL form uses --url. Options stop being parsed after the server name and first command token.
- description: Removes an MCP server from mcp.json.
  name: mcp remove
  non_interactive: true
  notes: Stored OAuth credentials are kept.
- description: Connects to every enabled MCP server and reports state, tools, and errors.
  name: mcp list
  non_interactive: true
  notes: Exits 1 when an entry is invalid or an enabled server is not connected; --json prints JSON.
- description: Signs in to an OAuth MCP server through the browser.
  name: mcp login
  non_interactive: false
  notes: Opens a browser and waits (default 300 seconds); accepts --timeout.
- description: Deletes the stored OAuth credentials of an MCP server.
  name: mcp logout
  non_interactive: true
  notes: Exits 1 for a server that does not use OAuth.
cli_switches:
- attachment:
  - space
  description: Selects the provider that --model is searched in.
  evidence_ids:
  - pi-args-source-1-0-0
  - pi-args-source-0-87-1
  - pi-help-0-87-1
  - pi-help-1-0-0
  - pi-parse-tests-0-87-1
  - pi-parse-tests-1-0-0
  example: pi --provider openai --model gpt-4o-mini -p 'hi'
  flag: --provider
  invocation_scope:
  - applies_to: command
    command: []
  notes: 'In 1.0.0 the switch without --model exits 1 with an error; 0.87.1 ignored it. A value-less trailing --provider is read as an unknown flag (Unknown option: --provider).'
  scope:
  - model_selection
  value: <name>
  value_optional: false
  value_type: string
- attachment:
  - space
  description: Selects a model pattern or id, including provider/id and an optional :<thinking> suffix.
  evidence_ids:
  - pi-args-source-1-0-0
  - pi-args-source-0-87-1
  - pi-help-0-87-1
  - pi-help-1-0-0
  - pi-parse-tests-0-87-1
  - pi-parse-tests-1-0-0
  example: pi --model openai/gpt-4o -p 'hi'
  flag: --model
  invocation_scope:
  - applies_to: command
    command: []
  notes: A value-less trailing --model is read as an unknown flag.
  scope:
  - model_selection
  value: <pattern>
  value_optional: false
  value_type: string
- attachment:
  - space
  description: Supplies a runtime API key for the selected model's provider.
  evidence_ids:
  - pi-args-source-1-0-0
  - pi-args-source-0-87-1
  - pi-help-0-87-1
  - pi-help-1-0-0
  - pi-parse-tests-0-87-1
  - pi-parse-tests-1-0-0
  example: pi --model openai/gpt-4o --api-key sk-... -p 'hi'
  flag: --api-key
  invocation_scope:
  - applies_to: command
    command: []
  notes: Requires --model, --provider with --model, or --models, else exits 1. The key is visible in the process list; redact it in logs.
  scope:
  - auth
  value: <key>
  value_optional: false
  value_type: string
- attachment:
  - space
  description: Replaces the system prompt; delivery semantics belong to the system-prompt topic.
  evidence_ids:
  - pi-args-source-1-0-0
  - pi-args-source-0-87-1
  - pi-help-0-87-1
  - pi-help-1-0-0
  - pi-parse-tests-0-87-1
  - pi-parse-tests-1-0-0
  flag: --system-prompt
  invocation_scope:
  - applies_to: command
    command: []
  notes: Existence, spelling, and value type only; semantics are owned by the system-prompt topic.
  scope:
  - prompt
  value: <text>
  value_optional: false
  value_type: string
- attachment:
  - space
  description: Appends text or file contents to the system prompt; may be repeated.
  evidence_ids:
  - pi-args-source-1-0-0
  - pi-args-source-0-87-1
  - pi-help-0-87-1
  - pi-help-1-0-0
  - pi-parse-tests-0-87-1
  - pi-parse-tests-1-0-0
  flag: --append-system-prompt
  invocation_scope:
  - applies_to: command
    command: []
  notes: Each occurrence takes one value and is collected into a list, so it is string, not variadic. Semantics are owned by the system-prompt topic.
  scope:
  - prompt
  value: <text-or-file>
  value_optional: false
  value_type: string
- attachment:
  - space
  default: text
  description: Selects text, json (JSONL events), or rpc output.
  evidence_ids:
  - pi-args-source-1-0-0
  - pi-args-source-0-87-1
  - pi-help-0-87-1
  - pi-help-1-0-0
  - pi-parse-tests-0-87-1
  - pi-parse-tests-1-0-0
  example: pi --mode json -p 'hi'
  flag: --mode
  invocation_scope:
  - applies_to: command
    command: []
  notes: Accepted values are text, json, and rpc. A missing value or a next token starting with a dash is an error and is not consumed; any other value is an error exit 1. --mode rpc rejects @file arguments.
  scope:
  - output
  - automation
  value: <mode>
  value_optional: false
  value_type: string
- attachment:
  - space
  description: Uses a specific session file or partial session id.
  evidence_ids:
  - pi-args-source-1-0-0
  - pi-args-source-0-87-1
  - pi-help-0-87-1
  - pi-help-1-0-0
  - pi-parse-tests-0-87-1
  - pi-parse-tests-1-0-0
  flag: --session
  invocation_scope:
  - applies_to: command
    command: []
  scope:
  - sessions
  value: <path|id>
  value_optional: false
  value_type: string
- attachment:
  - space
  description: Uses an exact project session id, creating it when missing.
  evidence_ids:
  - pi-args-source-1-0-0
  - pi-args-source-0-87-1
  - pi-help-0-87-1
  - pi-help-1-0-0
  - pi-parse-tests-0-87-1
  - pi-parse-tests-1-0-0
  flag: --session-id
  invocation_scope:
  - applies_to: command
    command: []
  notes: The id must be alphanumeric with -, _, and . inside and alphanumeric at both ends.
  scope:
  - sessions
  value: <id>
  value_optional: false
  value_type: string
- attachment:
  - space
  description: Forks a session file or partial session id into a new session.
  evidence_ids:
  - pi-args-source-1-0-0
  - pi-args-source-0-87-1
  - pi-help-0-87-1
  - pi-help-1-0-0
  - pi-parse-tests-0-87-1
  - pi-parse-tests-1-0-0
  flag: --fork
  invocation_scope:
  - applies_to: command
    command: []
  scope:
  - sessions
  value: <path|id>
  value_optional: false
  value_type: string
- attachment:
  - space
  default: ~/.pi/agent/sessions or the sessionDir setting
  description: Sets the directory for session storage and lookup.
  evidence_ids:
  - pi-args-source-1-0-0
  - pi-args-source-0-87-1
  - pi-help-0-87-1
  - pi-help-1-0-0
  - pi-parse-tests-0-87-1
  - pi-parse-tests-1-0-0
  flag: --session-dir
  invocation_scope:
  - applies_to: command
    command: []
  notes: Wins over PI_CODING_AGENT_SESSION_DIR and the sessionDir setting.
  scope:
  - sessions
  value: <dir>
  value_optional: false
  value_type: string
- aliases:
  - -n
  attachment:
  - space
  description: Sets the session display name.
  evidence_ids:
  - pi-args-source-1-0-0
  - pi-args-source-0-87-1
  - pi-help-0-87-1
  - pi-help-1-0-0
  - pi-parse-tests-0-87-1
  - pi-parse-tests-1-0-0
  flag: --name
  invocation_scope:
  - applies_to: command
    command: []
  notes: A missing value is an error; the next token is consumed even when it starts with a dash.
  scope:
  - sessions
  value: <name>
  value_optional: false
  value_type: string
- attachment:
  - space
  description: Sets comma-separated model patterns for Ctrl+P cycling.
  evidence_ids:
  - pi-args-source-1-0-0
  - pi-args-source-0-87-1
  - pi-help-0-87-1
  - pi-help-1-0-0
  - pi-parse-tests-0-87-1
  - pi-parse-tests-1-0-0
  example: pi --models claude-sonnet,claude-haiku
  flag: --models
  invocation_scope:
  - applies_to: command
    command: []
  notes: One value split on commas by Pi; supports globs and :<thinking> suffixes.
  scope:
  - model_selection
  value: <patterns>
  value_optional: false
  value_type: string
- aliases:
  - -t
  attachment:
  - space
  description: Enables a comma-separated allowlist of tool names.
  evidence_ids:
  - pi-args-source-1-0-0
  - pi-args-source-0-87-1
  - pi-help-0-87-1
  - pi-help-1-0-0
  - pi-parse-tests-0-87-1
  - pi-parse-tests-1-0-0
  example: pi --tools read,grep,find,ls -p 'Review src/'
  flag: --tools
  invocation_scope:
  - applies_to: command
    command: []
  notes: One value split on commas; applies to built-in, extension, and custom tools.
  scope:
  - tools
  value: <tools>
  value_optional: false
  value_type: string
- aliases:
  - -xt
  attachment:
  - space
  description: Disables a comma-separated denylist of tool names.
  evidence_ids:
  - pi-args-source-1-0-0
  - pi-args-source-0-87-1
  - pi-help-0-87-1
  - pi-help-1-0-0
  - pi-parse-tests-0-87-1
  - pi-parse-tests-1-0-0
  flag: --exclude-tools
  invocation_scope:
  - applies_to: command
    command: []
  notes: One value split on commas.
  scope:
  - tools
  value: <tools>
  value_optional: false
  value_type: string
- attachment:
  - space
  description: 'Sets the reasoning level: off, minimal, low, medium, high, xhigh, or max.'
  evidence_ids:
  - pi-args-source-1-0-0
  - pi-args-source-0-87-1
  - pi-help-0-87-1
  - pi-help-1-0-0
  - pi-parse-tests-0-87-1
  - pi-parse-tests-1-0-0
  flag: --thinking
  invocation_scope:
  - applies_to: command
    command: []
  notes: An invalid level produces a warning and the run continues with the default level; the token is still consumed.
  scope:
  - model_behavior
  value: <level>
  value_optional: false
  value_type: string
- aliases:
  - -e
  attachment:
  - space
  description: Loads an extension file or package source; may be repeated.
  evidence_ids:
  - pi-args-source-1-0-0
  - pi-args-source-0-87-1
  - pi-help-0-87-1
  - pi-help-1-0-0
  - pi-parse-tests-0-87-1
  - pi-parse-tests-1-0-0
  flag: --extension
  invocation_scope:
  - applies_to: command
    command: []
  notes: Each occurrence takes one value. 1.0.0 also accepts builtin:<name>.
  scope:
  - resources
  value: <path>
  value_optional: false
  value_type: string
- attachment:
  - space
  description: Loads a skill file or directory; may be repeated.
  evidence_ids:
  - pi-args-source-1-0-0
  - pi-args-source-0-87-1
  - pi-help-0-87-1
  - pi-help-1-0-0
  - pi-parse-tests-0-87-1
  - pi-parse-tests-1-0-0
  flag: --skill
  invocation_scope:
  - applies_to: command
    command: []
  scope:
  - resources
  value: <path>
  value_optional: false
  value_type: string
- attachment:
  - space
  description: Loads a prompt template file or directory; may be repeated.
  evidence_ids:
  - pi-args-source-1-0-0
  - pi-args-source-0-87-1
  - pi-help-0-87-1
  - pi-help-1-0-0
  - pi-parse-tests-0-87-1
  - pi-parse-tests-1-0-0
  flag: --prompt-template
  invocation_scope:
  - applies_to: command
    command: []
  scope:
  - resources
  value: <path>
  value_optional: false
  value_type: string
- attachment:
  - space
  description: Loads a theme file or directory; may be repeated.
  evidence_ids:
  - pi-args-source-1-0-0
  - pi-args-source-0-87-1
  - pi-help-0-87-1
  - pi-help-1-0-0
  - pi-parse-tests-0-87-1
  - pi-parse-tests-1-0-0
  flag: --theme
  invocation_scope:
  - applies_to: command
    command: []
  scope:
  - resources
  - ui
  value: <path>
  value_optional: false
  value_type: string
- attachment:
  - space
  description: Sets the initial interactive theme for this run.
  evidence_ids:
  - pi-args-source-1-0-0
  - pi-args-source-0-87-1
  - pi-help-0-87-1
  - pi-help-1-0-0
  - pi-parse-tests-0-87-1
  - pi-parse-tests-1-0-0
  flag: --use-theme
  invocation_scope:
  - applies_to: command
    command: []
  notes: A missing value or a next token starting with a dash is an error and is not consumed.
  scope:
  - ui
  value: <name[/name]>
  value_optional: false
  value_type: string
- attachment:
  - space
  description: Exports a session file to HTML and exits.
  evidence_ids:
  - pi-args-source-1-0-0
  - pi-args-source-0-87-1
  - pi-help-0-87-1
  - pi-help-1-0-0
  - pi-parse-tests-0-87-1
  - pi-parse-tests-1-0-0
  - pi-main-source-1-0-0
  example: pi --export session.jsonl out.html
  flag: --export
  invocation_scope:
  - applies_to: command
    command: []
  notes: The optional output path is the first positional message, not a second value of the switch, so Claudine must leave the following positional alone.
  scope:
  - sessions
  value: <file>
  value_optional: false
  value_type: string
- attachment:
  - space
  default: fullscreen in 1.0.0; regular in 0.87.1
  description: Selects the interactive TUI mode, regular or fullscreen.
  evidence_ids:
  - pi-args-source-1-0-0
  - pi-args-source-0-87-1
  - pi-help-0-87-1
  - pi-help-1-0-0
  - pi-parse-tests-0-87-1
  - pi-parse-tests-1-0-0
  flag: --tui-mode
  invocation_scope:
  - applies_to: command
    command: []
  notes: A missing value, a dash-prefixed next token, or a value other than regular and fullscreen exits 1.
  scope:
  - ui
  value: <mode>
  value_optional: false
  value_type: string
- attachment:
  - space
  description: Lists available models, optionally filtered by a fuzzy search term, and exits.
  evidence_ids:
  - pi-args-source-1-0-0
  - pi-args-source-0-87-1
  - pi-help-0-87-1
  - pi-help-1-0-0
  - pi-parse-tests-0-87-1
  - pi-parse-tests-1-0-0
  example: pi --list-models sonnet
  flag: --list-models
  invocation_scope:
  - applies_to: command
    command: []
  notes: The next token is the search term unless it starts with a dash or @; the equals form is not read.
  scope:
  - models
  - introspection
  value: '[search]'
  value_optional: true
  value_type: string
- aliases:
  - -p
  attachment: []
  description: Runs non-interactively, processes the prompt, and exits.
  evidence_ids:
  - pi-args-source-1-0-0
  - pi-args-source-0-87-1
  - pi-help-0-87-1
  - pi-help-1-0-0
  - pi-parse-tests-0-87-1
  - pi-parse-tests-1-0-0
  example: pi -p 'Summarize this repo'
  flag: --print
  invocation_scope:
  - applies_to: command
    command: []
  notes: The parser also takes the next token as a prompt message unless it starts with @ or with - (three dashes still count as a message). That token is a message, equivalent to a positional, not a value of the switch, so the switch takes none.
  scope:
  - automation
  value_type: none
- aliases:
  - -c
  attachment: []
  description: Continues the previous session.
  evidence_ids:
  - pi-args-source-1-0-0
  - pi-args-source-0-87-1
  - pi-help-0-87-1
  - pi-help-1-0-0
  - pi-parse-tests-0-87-1
  - pi-parse-tests-1-0-0
  flag: --continue
  invocation_scope:
  - applies_to: command
    command: []
  scope:
  - sessions
  value_type: none
- aliases:
  - -r
  attachment: []
  description: Opens the session picker to resume a session.
  evidence_ids:
  - pi-args-source-1-0-0
  - pi-args-source-0-87-1
  - pi-help-0-87-1
  - pi-help-1-0-0
  - pi-parse-tests-0-87-1
  - pi-parse-tests-1-0-0
  flag: --resume
  invocation_scope:
  - applies_to: command
    command: []
  notes: Interactive picker; not suitable without a terminal.
  scope:
  - sessions
  value_type: none
- attachment: []
  description: Does not save the session (ephemeral run).
  evidence_ids:
  - pi-args-source-1-0-0
  - pi-args-source-0-87-1
  - pi-help-0-87-1
  - pi-help-1-0-0
  - pi-parse-tests-0-87-1
  - pi-parse-tests-1-0-0
  flag: --no-session
  invocation_scope:
  - applies_to: command
    command: []
  scope:
  - sessions
  value_type: none
- aliases:
  - -nt
  attachment: []
  description: Disables all tools by default, built-in and extension.
  evidence_ids:
  - pi-args-source-1-0-0
  - pi-args-source-0-87-1
  - pi-help-0-87-1
  - pi-help-1-0-0
  - pi-parse-tests-0-87-1
  - pi-parse-tests-1-0-0
  flag: --no-tools
  invocation_scope:
  - applies_to: command
    command: []
  scope:
  - tools
  value_type: none
- aliases:
  - -nbt
  attachment: []
  description: Disables built-in tools but keeps extension and custom tools.
  evidence_ids:
  - pi-args-source-1-0-0
  - pi-args-source-0-87-1
  - pi-help-0-87-1
  - pi-help-1-0-0
  - pi-parse-tests-0-87-1
  - pi-parse-tests-1-0-0
  flag: --no-builtin-tools
  invocation_scope:
  - applies_to: command
    command: []
  scope:
  - tools
  value_type: none
- aliases:
  - -ne
  attachment: []
  description: Disables extension discovery; explicit -e paths still load.
  evidence_ids:
  - pi-args-source-1-0-0
  - pi-args-source-0-87-1
  - pi-help-0-87-1
  - pi-help-1-0-0
  - pi-parse-tests-0-87-1
  - pi-parse-tests-1-0-0
  flag: --no-extensions
  invocation_scope:
  - applies_to: command
    command: []
  notes: In 1.0.0 it also disables built-in extensions.
  scope:
  - resources
  value_type: none
- aliases:
  - -ns
  attachment: []
  description: Disables skill discovery and loading.
  evidence_ids:
  - pi-args-source-1-0-0
  - pi-args-source-0-87-1
  - pi-help-0-87-1
  - pi-help-1-0-0
  - pi-parse-tests-0-87-1
  - pi-parse-tests-1-0-0
  flag: --no-skills
  invocation_scope:
  - applies_to: command
    command: []
  scope:
  - resources
  value_type: none
- aliases:
  - -np
  attachment: []
  description: Disables prompt template discovery and loading.
  evidence_ids:
  - pi-args-source-1-0-0
  - pi-args-source-0-87-1
  - pi-help-0-87-1
  - pi-help-1-0-0
  - pi-parse-tests-0-87-1
  - pi-parse-tests-1-0-0
  flag: --no-prompt-templates
  invocation_scope:
  - applies_to: command
    command: []
  scope:
  - resources
  value_type: none
- attachment: []
  description: Disables theme discovery and loading.
  evidence_ids:
  - pi-args-source-1-0-0
  - pi-args-source-0-87-1
  - pi-help-0-87-1
  - pi-help-1-0-0
  - pi-parse-tests-0-87-1
  - pi-parse-tests-1-0-0
  flag: --no-themes
  invocation_scope:
  - applies_to: command
    command: []
  scope:
  - resources
  - ui
  value_type: none
- aliases:
  - -nc
  attachment: []
  description: Disables AGENTS.md and CLAUDE.md discovery and loading.
  evidence_ids:
  - pi-args-source-1-0-0
  - pi-args-source-0-87-1
  - pi-help-0-87-1
  - pi-help-1-0-0
  - pi-parse-tests-0-87-1
  - pi-parse-tests-1-0-0
  flag: --no-context-files
  invocation_scope:
  - applies_to: command
    command: []
  scope:
  - context
  value_type: none
- attachment: []
  description: Forces verbose startup, overriding the quietStartup setting.
  evidence_ids:
  - pi-args-source-1-0-0
  - pi-args-source-0-87-1
  - pi-help-0-87-1
  - pi-help-1-0-0
  - pi-parse-tests-0-87-1
  - pi-parse-tests-1-0-0
  flag: --verbose
  invocation_scope:
  - applies_to: command
    command: []
  scope:
  - diagnostics
  value_type: none
- aliases:
  - -a
  attachment: []
  description: Trusts project-local files for this run or package command.
  evidence_ids:
  - pi-args-source-1-0-0
  - pi-args-source-0-87-1
  - pi-help-0-87-1
  - pi-help-1-0-0
  - pi-parse-tests-0-87-1
  - pi-parse-tests-1-0-0
  - pi-pkg-source-1-0-0
  - pi-help-1-0-0
  - pi-parse-tests-0-87-1
  - pi-parse-tests-1-0-0
  - pi-docs-cli-1-0-0
  example: pi --approve -p 'Use project-local extensions'
  flag: --approve
  invocation_scope:
  - applies_to: command
    command: []
  - applies_to: command
    command:
    - install
  - applies_to: command
    command:
    - remove
  - applies_to: command
    command:
    - uninstall
  - applies_to: command
    command:
    - update
  - applies_to: command
    command:
    - list
  notes: Without it or --no-approve, a non-terminal run does not prompt and falls back to the saved trust decision or defaultProjectTrust.
  scope:
  - project_trust
  value_type: none
- aliases:
  - -na
  attachment: []
  description: Ignores project-local files for this run or package command.
  evidence_ids:
  - pi-args-source-1-0-0
  - pi-args-source-0-87-1
  - pi-help-0-87-1
  - pi-help-1-0-0
  - pi-parse-tests-0-87-1
  - pi-parse-tests-1-0-0
  - pi-pkg-source-1-0-0
  - pi-help-1-0-0
  - pi-parse-tests-0-87-1
  - pi-parse-tests-1-0-0
  - pi-docs-cli-1-0-0
  flag: --no-approve
  invocation_scope:
  - applies_to: command
    command: []
  - applies_to: command
    command:
    - install
  - applies_to: command
    command:
    - remove
  - applies_to: command
    command:
    - uninstall
  - applies_to: command
    command:
    - update
  - applies_to: command
    command:
    - list
  scope:
  - project_trust
  value_type: none
- attachment: []
  description: Disables startup network operations; same as PI_OFFLINE=1.
  evidence_ids:
  - pi-args-source-1-0-0
  - pi-args-source-0-87-1
  - pi-help-0-87-1
  - pi-help-1-0-0
  - pi-parse-tests-0-87-1
  - pi-parse-tests-1-0-0
  - pi-main-source-1-0-0
  flag: --offline
  invocation_scope:
  - applies_to: command
    command: []
  notes: main() looks for the literal token anywhere in argv, including after --, and then sets PI_OFFLINE and PI_SKIP_VERSION_CHECK.
  scope:
  - network
  value_type: none
- aliases:
  - -v
  attachment: []
  description: Prints the version and exits.
  evidence_ids:
  - pi-args-source-1-0-0
  - pi-args-source-0-87-1
  - pi-help-0-87-1
  - pi-help-1-0-0
  - pi-parse-tests-0-87-1
  - pi-parse-tests-1-0-0
  flag: --version
  invocation_scope:
  - applies_to: command
    command: []
  notes: Takes effect after parse-time errors; accepted at the root only.
  scope:
  - meta
  value_type: none
- aliases:
  - -h
  attachment: []
  description: Shows help for the root or the command path.
  evidence_ids:
  - pi-args-source-1-0-0
  - pi-args-source-0-87-1
  - pi-help-0-87-1
  - pi-help-1-0-0
  - pi-parse-tests-0-87-1
  - pi-parse-tests-1-0-0
  - pi-pkg-source-1-0-0
  - pi-help-1-0-0
  - pi-parse-tests-0-87-1
  - pi-parse-tests-1-0-0
  - pi-docs-cli-1-0-0
  - pi-auth-source-1-0-0
  - pi-mcp-source-1-0-0
  flag: --help
  invocation_scope:
  - applies_to: global
  notes: Accepted at the root and after every command path; with auth and mcp it is detected anywhere in the arguments.
  scope:
  - meta
  value_type: none
- aliases:
  - -l
  attachment: []
  description: Installs or removes a package in project settings instead of global settings.
  evidence_ids:
  - pi-pkg-source-1-0-0
  - pi-help-1-0-0
  - pi-parse-tests-0-87-1
  - pi-parse-tests-1-0-0
  - pi-docs-cli-1-0-0
  example: pi install npm:@foo/bar --local
  flag: --local
  invocation_scope:
  - applies_to: command
    command:
    - install
  - applies_to: command
    command:
    - remove
  - applies_to: command
    command:
    - uninstall
  notes: Writing project settings requires trust; without --approve a non-trusted project exits 1. Also accepted by config, which is interactive and not inventoried.
  scope:
  - packages
  value_type: none
- attachment: []
  description: Updates Pi only.
  evidence_ids:
  - pi-pkg-source-1-0-0
  - pi-help-1-0-0
  - pi-parse-tests-0-87-1
  - pi-parse-tests-1-0-0
  - pi-docs-cli-1-0-0
  flag: --self
  invocation_scope:
  - applies_to: command
    command:
    - update
  notes: Default when no target is given.
  scope:
  - packages
  value_type: none
- attachment: []
  description: Updates installed packages only.
  evidence_ids:
  - pi-pkg-source-1-0-0
  - pi-help-1-0-0
  - pi-parse-tests-0-87-1
  - pi-parse-tests-1-0-0
  - pi-docs-cli-1-0-0
  flag: --extensions
  invocation_scope:
  - applies_to: command
    command:
    - update
  scope:
  - packages
  value_type: none
- attachment: []
  description: Refreshes model catalogs only.
  evidence_ids:
  - pi-pkg-source-1-0-0
  - pi-help-1-0-0
  - pi-parse-tests-0-87-1
  - pi-parse-tests-1-0-0
  - pi-docs-cli-1-0-0
  flag: --models
  invocation_scope:
  - applies_to: command
    command:
    - update
  notes: Same spelling as the root --models, which takes a value; at update it takes none. Cannot be combined with --self, --extensions, --all, or --extension.
  scope:
  - packages
  - models
  value_type: none
- attachment: []
  description: Updates Pi and installed packages.
  evidence_ids:
  - pi-pkg-source-1-0-0
  - pi-help-1-0-0
  - pi-parse-tests-0-87-1
  - pi-parse-tests-1-0-0
  - pi-docs-cli-1-0-0
  flag: --all
  invocation_scope:
  - applies_to: command
    command:
    - update
  notes: Cannot be combined with --self, --extensions, --models, --extension, or a positional source.
  scope:
  - packages
  value_type: none
- attachment: []
  description: Reinstalls Pi even when the current version is latest.
  evidence_ids:
  - pi-pkg-source-1-0-0
  - pi-help-1-0-0
  - pi-parse-tests-0-87-1
  - pi-parse-tests-1-0-0
  - pi-docs-cli-1-0-0
  flag: --force
  invocation_scope:
  - applies_to: command
    command:
    - update
  scope:
  - packages
  value_type: none
- attachment:
  - space
  description: Updates one package source.
  evidence_ids:
  - pi-pkg-source-1-0-0
  - pi-help-1-0-0
  - pi-parse-tests-0-87-1
  - pi-parse-tests-1-0-0
  - pi-docs-cli-1-0-0
  example: pi update --extension npm:@foo/bar
  flag: --extension
  invocation_scope:
  - applies_to: command
    command:
    - update
  notes: Same spelling as the root --extension but with no -e alias and package-source meaning; the next token must not start with a dash, may be given once, and equals a positional source.
  scope:
  - packages
  value: <source>
  value_optional: false
  value_type: string
- attachment:
  - space
  description: Resolves credentials for a provider.
  evidence_ids:
  - pi-auth-source-1-0-0
  - pi-main-source-1-0-0
  - pi-parse-tests-0-87-1
  - pi-parse-tests-1-0-0
  - pi-docs-cli-1-0-0
  example: pi auth check --provider openai --json
  flag: --provider
  invocation_scope:
  - applies_to: command
    command:
    - auth
    - check
  - applies_to: command
    command:
    - auth
    - print-api-key
  - applies_to: command
    command:
    - auth
    - print-bearer-token
  notes: At least one of --provider and --model is required (exit 2 for check, 1 for the print commands). A trailing value-less --provider is reported as an unknown option.
  scope:
  - auth
  value: <provider>
  value_optional: false
  value_type: string
- attachment:
  - space
  description: Resolves credentials from a model; may be combined with --provider.
  evidence_ids:
  - pi-auth-source-1-0-0
  - pi-main-source-1-0-0
  - pi-parse-tests-0-87-1
  - pi-parse-tests-1-0-0
  - pi-docs-cli-1-0-0
  flag: --model
  invocation_scope:
  - applies_to: command
    command:
    - auth
    - check
  - applies_to: command
    command:
    - auth
    - print-api-key
  - applies_to: command
    command:
    - auth
    - print-bearer-token
  scope:
  - auth
  value: <model>
  value_optional: false
  value_type: string
- attachment: []
  description: Writes the structured result as JSON.
  evidence_ids:
  - pi-auth-source-1-0-0
  - pi-main-source-1-0-0
  - pi-parse-tests-0-87-1
  - pi-parse-tests-1-0-0
  - pi-docs-cli-1-0-0
  flag: --json
  invocation_scope:
  - applies_to: command
    command:
    - auth
    - check
  notes: Any other auth path rejects it. The same spelling at mcp list is a separate switch.
  scope:
  - auth
  value_type: none
- attachment: []
  description: Emits the resolved credential when the provider is ready.
  evidence_ids:
  - pi-auth-source-1-0-0
  - pi-main-source-1-0-0
  - pi-parse-tests-0-87-1
  - pi-parse-tests-1-0-0
  - pi-docs-cli-1-0-0
  flag: --credentials
  invocation_scope:
  - applies_to: command
    command:
    - auth
    - check
  notes: Prints a secret to stdout; do not capture it into logs.
  scope:
  - auth
  value_type: none
- attachment: []
  description: Does not refresh expired OAuth credentials.
  evidence_ids:
  - pi-auth-source-1-0-0
  - pi-main-source-1-0-0
  - pi-parse-tests-0-87-1
  - pi-parse-tests-1-0-0
  - pi-docs-cli-1-0-0
  flag: --no-refresh
  invocation_scope:
  - applies_to: command
    command:
    - auth
    - check
  scope:
  - auth
  value_type: none
- attachment:
  - space
  description: Requires the bearer token to stay valid for at least the given duration.
  evidence_ids:
  - pi-auth-source-1-0-0
  - pi-main-source-1-0-0
  - pi-parse-tests-0-87-1
  - pi-parse-tests-1-0-0
  - pi-docs-cli-1-0-0
  example: pi auth print-bearer-token --provider openai-codex --min-expiry 30m
  flag: --min-expiry
  invocation_scope:
  - applies_to: command
    command:
    - auth
    - print-bearer-token
  notes: The duration is an integer with a unit of ms, s, m, or h; anything else exits 1.
  scope:
  - auth
  value: <duration>
  value_optional: false
  value_type: string
- attachment: []
  description: Prints the server list as JSON.
  evidence_ids:
  - pi-mcp-source-1-0-0
  - pi-help-1-0-0
  - pi-parse-tests-1-0-0
  - pi-docs-cli-1-0-0
  example: pi mcp list --json
  flag: --json
  invocation_scope:
  - applies_to: command
    command:
    - mcp
    - list
  scope:
  - mcp
  value_type: none
- aliases:
  - -l
  attachment: []
  description: Uses .pi/mcp.json in the current project instead of the global file.
  evidence_ids:
  - pi-mcp-source-1-0-0
  - pi-help-1-0-0
  - pi-parse-tests-1-0-0
  - pi-docs-cli-1-0-0
  flag: --local
  invocation_scope:
  - applies_to: command
    command:
    - mcp
    - add
  - applies_to: command
    command:
    - mcp
    - remove
  notes: Introduced with built-in MCP in 0.99.0.
  scope:
  - mcp
  value_type: none
- attachment:
  - space
  description: Adds a streamable HTTP server at the given URL instead of a stdio command.
  evidence_ids:
  - pi-mcp-source-1-0-0
  - pi-help-1-0-0
  - pi-parse-tests-1-0-0
  - pi-docs-cli-1-0-0
  flag: --url
  invocation_scope:
  - applies_to: command
    command:
    - mcp
    - add
  scope:
  - mcp
  value: <url>
  value_optional: false
  value_type: string
- attachment:
  - space
  description: Sets an environment variable for a stdio server; may be repeated.
  evidence_ids:
  - pi-mcp-source-1-0-0
  - pi-help-1-0-0
  - pi-parse-tests-1-0-0
  - pi-docs-cli-1-0-0
  flag: --env
  invocation_scope:
  - applies_to: command
    command:
    - mcp
    - add
  notes: Rejected together with --url.
  scope:
  - mcp
  value: <KEY=VALUE>
  value_optional: false
  value_type: string
- attachment:
  - space
  description: Sets the working directory of a stdio server.
  evidence_ids:
  - pi-mcp-source-1-0-0
  - pi-help-1-0-0
  - pi-parse-tests-1-0-0
  - pi-docs-cli-1-0-0
  flag: --cwd
  invocation_scope:
  - applies_to: command
    command:
    - mcp
    - add
  scope:
  - mcp
  value: <dir>
  value_optional: false
  value_type: string
- attachment:
  - space
  description: Sets an HTTP header for a URL server; may be repeated.
  evidence_ids:
  - pi-mcp-source-1-0-0
  - pi-help-1-0-0
  - pi-parse-tests-1-0-0
  - pi-docs-cli-1-0-0
  flag: --header
  invocation_scope:
  - applies_to: command
    command:
    - mcp
    - add
  scope:
  - mcp
  value: <KEY=VALUE>
  value_optional: false
  value_type: string
- attachment:
  - space
  description: 'Sends Authorization: Bearer ${NAME} for a URL server.'
  evidence_ids:
  - pi-mcp-source-1-0-0
  - pi-help-1-0-0
  - pi-parse-tests-1-0-0
  - pi-docs-cli-1-0-0
  flag: --bearer-token-env-var
  invocation_scope:
  - applies_to: command
    command:
    - mcp
    - add
  scope:
  - mcp
  value: <NAME>
  value_optional: false
  value_type: string
- attachment:
  - space
  description: Sets a pre-registered OAuth client id.
  evidence_ids:
  - pi-mcp-source-1-0-0
  - pi-help-1-0-0
  - pi-parse-tests-1-0-0
  - pi-docs-cli-1-0-0
  flag: --oauth-client-id
  invocation_scope:
  - applies_to: command
    command:
    - mcp
    - add
  scope:
  - mcp
  value: <id>
  value_optional: false
  value_type: string
- attachment:
  - space
  description: Sets the OAuth client secret, which may be ${NAME} or !command.
  evidence_ids:
  - pi-mcp-source-1-0-0
  - pi-help-1-0-0
  - pi-parse-tests-1-0-0
  - pi-docs-cli-1-0-0
  flag: --oauth-client-secret
  invocation_scope:
  - applies_to: command
    command:
    - mcp
    - add
  scope:
  - mcp
  value: <secret>
  value_optional: false
  value_type: string
- attachment:
  - space
  description: Sets a fixed OAuth callback port.
  evidence_ids:
  - pi-mcp-source-1-0-0
  - pi-help-1-0-0
  - pi-parse-tests-1-0-0
  - pi-docs-cli-1-0-0
  flag: --oauth-callback-port
  invocation_scope:
  - applies_to: command
    command:
    - mcp
    - add
  notes: The parser takes one token and converts it with Number().
  scope:
  - mcp
  value: <port>
  value_optional: false
  value_type: number
- attachment:
  - space
  description: Sets the client name sent when registering with the OAuth server.
  evidence_ids:
  - pi-mcp-source-1-0-0
  - pi-help-1-0-0
  - pi-parse-tests-1-0-0
  - pi-docs-cli-1-0-0
  flag: --oauth-client-name
  invocation_scope:
  - applies_to: command
    command:
    - mcp
    - add
  scope:
  - mcp
  value: <name>
  value_optional: false
  value_type: string
- attachment:
  - space
  default: codemode
  description: 'Sets how the server''s tools are exposed: codemode, deferred, direct, or hidden.'
  evidence_ids:
  - pi-mcp-source-1-0-0
  - pi-help-1-0-0
  - pi-parse-tests-1-0-0
  - pi-docs-cli-1-0-0
  flag: --exposure
  invocation_scope:
  - applies_to: command
    command:
    - mcp
    - add
  scope:
  - mcp
  value: <mode>
  value_optional: false
  value_type: string
- attachment:
  - space
  description: Describes what the server offers, shown in the system prompt.
  evidence_ids:
  - pi-mcp-source-1-0-0
  - pi-help-1-0-0
  - pi-parse-tests-1-0-0
  - pi-docs-cli-1-0-0
  flag: --description
  invocation_scope:
  - applies_to: command
    command:
    - mcp
    - add
  scope:
  - mcp
  value: <text>
  value_optional: false
  value_type: string
config_paths:
- format: other
  notes: Replaces the agent directory; every user-scope file above is read from it.
  os: macos
  path: $PI_CODING_AGENT_DIR
  scope: env
- format: json
  notes: User settings; project settings override it.
  os: macos
  path: ~/.pi/agent/settings.json
  scope: user
- format: json
  notes: Interactive keybinding customization.
  os: macos
  path: ~/.pi/agent/keybindings.json
  scope: user
- format: json
  notes: MCP servers for every project; written by pi mcp add and remove.
  os: macos
  path: ~/.pi/agent/mcp.json
  scope: user
- format: json
  notes: Custom endpoints, models, and model overrides.
  os: macos
  path: ~/.pi/agent/models.json
  scope: user
- format: json
  notes: Saved API keys and OAuth credentials; written by /login.
  os: macos
  path: ~/.pi/agent/auth.json
  scope: user
- format: json
  notes: Saved project trust decisions; non-terminal runs read it and never prompt.
  os: macos
  path: ~/.pi/agent/trust.json
  scope: user
- format: text
  notes: User context instructions; AGENTS.override.md, AGENTS.MD, CLAUDE.md, and CLAUDE.MD are also read.
  os: macos
  path: ~/.pi/agent/AGENTS.md
  scope: user
- format: text
  notes: Replaces the default system prompt; semantics belong to the system-prompt topic.
  os: macos
  path: ~/.pi/agent/SYSTEM.md
  scope: user
- format: text
  notes: Appends to the system prompt; semantics belong to the system-prompt topic.
  os: macos
  path: ~/.pi/agent/APPEND_SYSTEM.md
  scope: user
- format: other
  notes: Session JSONL files grouped by working directory; moved by --session-dir, PI_CODING_AGENT_SESSION_DIR, or sessionDir.
  os: macos
  path: ~/.pi/agent/sessions
  scope: user
- format: text
  notes: Log of MCP server connections; written by Pi.
  os: macos
  path: ~/.pi/agent/mcp.log
  scope: user
- format: json
  notes: Project settings, read only once the project is trusted; override user settings.
  os: macos
  path: .pi/settings.json
  scope: repo
- format: json
  notes: Project MCP servers, read only once the project is trusted.
  os: macos
  path: .pi/mcp.json
  scope: repo
- format: text
  notes: Project system prompt replacement; wins over the user file.
  os: macos
  path: .pi/SYSTEM.md
  scope: repo
- format: text
  notes: Project system prompt addition; wins over the user file.
  os: macos
  path: .pi/APPEND_SYSTEM.md
  scope: repo
- format: text
  notes: Context file discovered from the working directory and its parents; CLAUDE.md is also read; disabled by --no-context-files; needs no trust.
  os: macos
  path: AGENTS.md
  scope: repo
- format: other
  notes: Replaces the agent directory; every user-scope file above is read from it.
  os: linux
  path: $PI_CODING_AGENT_DIR
  scope: env
- format: json
  notes: User settings; project settings override it.
  os: linux
  path: ~/.pi/agent/settings.json
  scope: user
- format: json
  notes: Interactive keybinding customization.
  os: linux
  path: ~/.pi/agent/keybindings.json
  scope: user
- format: json
  notes: MCP servers for every project; written by pi mcp add and remove.
  os: linux
  path: ~/.pi/agent/mcp.json
  scope: user
- format: json
  notes: Custom endpoints, models, and model overrides.
  os: linux
  path: ~/.pi/agent/models.json
  scope: user
- format: json
  notes: Saved API keys and OAuth credentials; written by /login.
  os: linux
  path: ~/.pi/agent/auth.json
  scope: user
- format: json
  notes: Saved project trust decisions; non-terminal runs read it and never prompt.
  os: linux
  path: ~/.pi/agent/trust.json
  scope: user
- format: text
  notes: User context instructions; AGENTS.override.md, AGENTS.MD, CLAUDE.md, and CLAUDE.MD are also read.
  os: linux
  path: ~/.pi/agent/AGENTS.md
  scope: user
- format: text
  notes: Replaces the default system prompt; semantics belong to the system-prompt topic.
  os: linux
  path: ~/.pi/agent/SYSTEM.md
  scope: user
- format: text
  notes: Appends to the system prompt; semantics belong to the system-prompt topic.
  os: linux
  path: ~/.pi/agent/APPEND_SYSTEM.md
  scope: user
- format: other
  notes: Session JSONL files grouped by working directory; moved by --session-dir, PI_CODING_AGENT_SESSION_DIR, or sessionDir.
  os: linux
  path: ~/.pi/agent/sessions
  scope: user
- format: text
  notes: Log of MCP server connections; written by Pi.
  os: linux
  path: ~/.pi/agent/mcp.log
  scope: user
- format: json
  notes: Project settings, read only once the project is trusted; override user settings.
  os: linux
  path: .pi/settings.json
  scope: repo
- format: json
  notes: Project MCP servers, read only once the project is trusted.
  os: linux
  path: .pi/mcp.json
  scope: repo
- format: text
  notes: Project system prompt replacement; wins over the user file.
  os: linux
  path: .pi/SYSTEM.md
  scope: repo
- format: text
  notes: Project system prompt addition; wins over the user file.
  os: linux
  path: .pi/APPEND_SYSTEM.md
  scope: repo
- format: text
  notes: Context file discovered from the working directory and its parents; CLAUDE.md is also read; disabled by --no-context-files; needs no trust.
  os: linux
  path: AGENTS.md
  scope: repo
- format: other
  notes: Replaces the agent directory; every user-scope file above is read from it.
  os: windows
  path: '%PI_CODING_AGENT_DIR%'
  scope: env
- format: json
  notes: User settings; project settings override it.
  os: windows
  path: '%USERPROFILE%\.pi\agent\settings.json'
  scope: user
- format: json
  notes: Interactive keybinding customization.
  os: windows
  path: '%USERPROFILE%\.pi\agent\keybindings.json'
  scope: user
- format: json
  notes: MCP servers for every project; written by pi mcp add and remove.
  os: windows
  path: '%USERPROFILE%\.pi\agent\mcp.json'
  scope: user
- format: json
  notes: Custom endpoints, models, and model overrides.
  os: windows
  path: '%USERPROFILE%\.pi\agent\models.json'
  scope: user
- format: json
  notes: Saved API keys and OAuth credentials; written by /login.
  os: windows
  path: '%USERPROFILE%\.pi\agent\auth.json'
  scope: user
- format: json
  notes: Saved project trust decisions; non-terminal runs read it and never prompt.
  os: windows
  path: '%USERPROFILE%\.pi\agent\trust.json'
  scope: user
- format: text
  notes: User context instructions; AGENTS.override.md, AGENTS.MD, CLAUDE.md, and CLAUDE.MD are also read.
  os: windows
  path: '%USERPROFILE%\.pi\agent\AGENTS.md'
  scope: user
- format: text
  notes: Replaces the default system prompt; semantics belong to the system-prompt topic.
  os: windows
  path: '%USERPROFILE%\.pi\agent\SYSTEM.md'
  scope: user
- format: text
  notes: Appends to the system prompt; semantics belong to the system-prompt topic.
  os: windows
  path: '%USERPROFILE%\.pi\agent\APPEND_SYSTEM.md'
  scope: user
- format: other
  notes: Session JSONL files grouped by working directory; moved by --session-dir, PI_CODING_AGENT_SESSION_DIR, or sessionDir.
  os: windows
  path: '%USERPROFILE%\.pi\agent\sessions'
  scope: user
- format: text
  notes: Log of MCP server connections; written by Pi.
  os: windows
  path: '%USERPROFILE%\.pi\agent\mcp.log'
  scope: user
- format: json
  notes: Project settings, read only once the project is trusted; override user settings.
  os: windows
  path: .pi\settings.json
  scope: repo
- format: json
  notes: Project MCP servers, read only once the project is trusted.
  os: windows
  path: .pi\mcp.json
  scope: repo
- format: text
  notes: Project system prompt replacement; wins over the user file.
  os: windows
  path: .pi\SYSTEM.md
  scope: repo
- format: text
  notes: Project system prompt addition; wins over the user file.
  os: windows
  path: .pi\APPEND_SYSTEM.md
  scope: repo
- format: text
  notes: Context file discovered from the working directory and its parents; CLAUDE.md is also read; disabled by --no-context-files; needs no trust.
  os: windows
  path: AGENTS.md
  scope: repo
env_vars:
- effect: Replaces the agent directory, default ~/.pi/agent.
  name: PI_CODING_AGENT_DIR
- effect: Replaces the session directory unless --session-dir is given.
  name: PI_CODING_AGENT_SESSION_DIR
- effect: Overrides the package directory, for immutable store paths such as Nix or Guix.
  name: PI_PACKAGE_DIR
- effect: With 1, true, or yes, disables automatic network activity including update checks and model catalog refreshes.
  name: PI_OFFLINE
- effect: Disables the pi.dev latest-version request; --offline and PI_OFFLINE set it.
  name: PI_SKIP_VERSION_CHECK
- effect: Overrides install and update telemetry and provider attribution headers with 1/true/yes or 0/false/no; it does not disable update checks.
  name: PI_TELEMETRY
- effect: Set to long for extended provider prompt caching where supported.
  name: PI_CACHE_RETENTION
- effect: Overrides the base URL used by the /share command.
  name: PI_SHARE_VIEWER_URL
- effect: Overrides the Radius gateway origin used by /bug uploads and Radius relay connections.
  name: PI_RADIUS_GATEWAY
- effect: Binds the local OAuth callback server of built-in login flows to a chosen interface instead of 127.0.0.1.
  name: PI_OAUTH_CALLBACK_HOST
- effect: With 1, shows the terminal hardware cursor.
  name: PI_HARDWARE_CURSOR
- effect: Overrides OSC 8 hyperlink detection with 1, 0, or auto.
  name: PI_HYPERLINKS
- effect: Overrides inline image detection with kitty, iterm2, none, or auto.
  name: PI_IMAGE_PROTOCOL
- effect: Overrides truecolor detection with 1, 0, or auto.
  name: PI_TRUE_COLOR
- effect: Milliseconds to wait after a lone ESC before treating it as Escape; 100 over SSH and 10 otherwise.
  name: PI_TUI_ESC_TIMEOUT
- effect: With a truthy value, benchmarks startup in interactive mode; any non-interactive mode exits 1 with an error.
  name: PI_STARTUP_BENCHMARK
- effect: With 1, enables startup timing output.
  name: PI_TIMING
- effect: With 1, enables experimental features.
  name: PI_EXPERIMENTAL
- effect: External editor, tried before EDITOR when the externalEditor setting is unset.
  name: VISUAL
- effect: External editor fallback after VISUAL.
  name: EDITOR
- effect: Proxies Pi's outbound HTTP requests.
  name: HTTP_PROXY
- effect: Proxies Pi's outbound HTTPS requests.
  name: HTTPS_PROXY
- effect: Set to pi by Pi in its own process so child processes can identify the launching agent.
  name: AI_AGENT
- effect: Set to true by Pi so child processes can detect that they run inside Pi.
  name: PI_CODING_AGENT
- effect: 'Set by Pi in commands run by its bash and powershell tools: the current session id.'
  name: PI_SESSION_ID
- effect: 'Set by Pi in tool shells: absolute path of the session JSONL file, unset for ephemeral sessions.'
  name: PI_SESSION_FILE
- effect: 'Set by Pi in tool shells: the selected model provider.'
  name: PI_PROVIDER
- effect: 'Set by Pi in tool shells: the selected model id.'
  name: PI_MODEL
- effect: 'Set by Pi in tool shells: the effective reasoning level.'
  name: PI_REASONING_LEVEL
machine_introspection:
- command: pi --version
  machine_readable: false
  notes: Prints the bare version, for example 0.87.1.
  output_format: text
  purpose: version
  useful_for_codegen: false
- command: pi --list-models [search]
  machine_readable: false
  notes: Columns provider, model, context, max-out, thinking, images; reads the local catalog and credentials and needs no network with PI_OFFLINE=1. With no credentials it prints a No models available notice and exits 0.
  output_format: table
  purpose: models
  useful_for_codegen: false
- command: pi auth check --provider <provider> --json
  machine_readable: true
  notes: 'One JSON object with status ready, not_ready, or invalid; exit 0, 1, or 2. Verified shape: {"status":"not_ready","provider":"x","reason":"provider_not_found"}. --credentials adds the secret.'
  output_format: json
  purpose: doctor
  useful_for_codegen: false
- command: pi mcp list --json
  machine_readable: true
  notes: Connects to each enabled MCP server; prints servers and errors arrays; exits 1 on an invalid entry or a disconnected server. Needs 0.99.0 or newer.
  output_format: json
  purpose: mcp
  useful_for_codegen: false
- command: pi list
  machine_readable: false
  notes: Installed packages from user and project settings; no JSON switch.
  output_format: text
  purpose: plugins
  useful_for_codegen: false
- command: pi --mode rpc --no-session
  machine_readable: true
  notes: Send {"type":"get_available_models"}, get_state, or get_commands as LF-delimited JSON on stdin; documented in docs/rpc.md and not exercised here.
  output_format: jsonl
  purpose: capabilities
  useful_for_codegen: true
wrapper_notes:
- 'Pass every value as a separate argument: the parser matches switches by exact string, so --name=foo and --mode=json are not read as the known switch and become unknown extension flags (Unknown option: --mode, exit 1). Package, auth, and mcp commands reject equals forms the same way.'
- A value switch consumes the next argument even when it starts with a dash (--model --version takes --version as the model); boolean switches never consume. --mode, --tui-mode, and --use-theme are the exceptions that refuse a dash-prefixed value with an error.
- 'A value switch written last, with nothing after it, exits 1: --provider, --model, and most others report Unknown option, while --mode, --tui-mode, --use-theme, and --name report a missing value.'
- An unknown --flag (an extension flag) swallows the next token unless that token starts with - or @, so an unrecognized switch can take a user's prompt as its value.
- -p also consumes the next token as the prompt unless it starts with @ or -; put a prompt that may begin with a dash after -- (pi -p -- "- item"). A token starting with @ is a file argument even after --.
- Stdin or stdout that is not a terminal selects print mode, and piped stdin is prepended to the first prompt, so stdin cannot seed an interactive session; for an interactive session with a first message, a terminal and a positional message after -- are needed.
- --offline anywhere in argv, even after --, turns offline mode on, so a forwarded user prompt consisting of that token changes behavior.
- 'In 1.0.0 the TUI is fullscreen by default (0.87.1: regular); a wrapper that depends on terminal scrollback should pass --tui-mode regular, which both 0.87.1 and 1.0.0 accept.'
- In 1.0.0 --provider without --model exits 1; 0.87.1 silently ran the default model of another provider.
- 'Project trust is never prompted without a terminal: without a saved decision the project-local settings, extensions, and MCP file are ignored unless --approve is given; pass --approve or --no-approve for deterministic runs and consider --no-extensions, --no-skills, --no-prompt-templates, and --no-context-files for isolation.'
- pi config and pi mcp login need a terminal or browser; pi update and pi install run the package manager and change the installation or settings, so a wrapper should not call them as a side effect.
- pi auth print-api-key, print-bearer-token, and auth check --credentials write secrets to stdout; pi auth check exits 0, 1, or 2 for ready, not_ready, and invalid.
- Pi sets AI_AGENT=pi and PI_CODING_AGENT=true for its children and injects PI_SESSION_ID, PI_SESSION_FILE, PI_PROVIDER, PI_MODEL, and PI_REASONING_LEVEL into its tool shells.
- On native Windows, Pi's bash tool needs Git Bash or another bash.exe; shellPath in settings.json overrides discovery.
- Extensions can register extra flags, so the inventory here is the baseline of an uncustomized Pi; help output lists registered extension flags when any are loaded.
changes:
- 'Rewrote the document for contract revision 2: typed cli_switches with value type, optional-value flag, attachment forms, and invocation scopes, plus cited evidence.'
- Verified installed Pi 0.87.1 and unpacked 1.0.0 (published 2026-10-01); the previous document described 0.80.3 and a stale 0.73.1 install.
- Added the auth commands (auth check, print-api-key, print-bearer-token), the --use-theme and --tui-mode switches, and the 1.0.0 mcp commands (add, remove, list, login, logout).
- 'Corrected --export: the optional output path is read from the first positional message, so the two-argument form in the docs works; the earlier note that it was unsupported is withdrawn.'
- Recorded that known switches accept only the space-separated form, and that value switches consume a following dash-prefixed token.
- 'Recorded the 1.0.0 behavior changes: fullscreen is the default TUI mode, --provider requires --model, and --no-extensions also disables built-in extensions.'
- 'Replaced the claim that Pi has no built-in MCP: 0.99.0 added MCP servers, codemode, and tool search as built-in extensions.'
- Added standalone release archives (pi, pi.exe), the PowerShell installer, mcp.json, mcp.log, and the session environment variables.
requires_claudine_update: true
reason: Claudine's Pi wrapper must treat known Pi switches as space-separated only and must not take a token that follows -p or --export away from the user; the 1.0.0 fullscreen default and the --provider-requires---model change affect wrapper launch arguments.
contract_checked: 2026-10-01
---

# Pi Agent CLI Surface

## Overview

Pi is a minimal, extensible terminal coding agent from Earendil Works. One `pi` command offers an interactive terminal UI, a print mode (`-p`), a JSON Lines event mode (`--mode json`), and a JSONL RPC mode (`--mode rpc`). From 0.99.0 it also bundles MCP support, codemode, and tool search as built-in extensions, with `pi mcp` commands to manage servers.

The version installed on this host is **0.87.1**, confirmed with `pi --version` on the Bun-global install at `/Users/ken/.bun/bin/pi` (package `@earendil-works/pi-coding-agent`). The newest release is **1.0.0**, published 2026-10-01 according to `npm view @earendil-works/pi-coding-agent version dist-tags time --json` and the GitHub release. The 1.0.0 package was unpacked into a scratch directory and run with `node dist/bundle/cli.js`, so both versions were exercised, and the parser source of both was read.

- Homepage: [pi.dev](https://pi.dev/)
- Repository: [earendil-works/pi](https://github.com/earendil-works/pi)
- Documentation: [pi.dev/docs/latest](https://pi.dev/docs/latest)
- CLI reference: [docs/cli.md](https://github.com/earendil-works/pi/blob/main/packages/coding-agent/docs/cli.md)

## Installation and Binaries

| OS | Command name | Install |
| --- | --- | --- |
| macOS | `pi` | `npm install -g --ignore-scripts @earendil-works/pi-coding-agent` (Node.js 22.19 or newer), `curl -fsSL https://pi.dev/install.sh \| sh`, `pnpm add -g …`, `bun add -g …`, or the `pi-darwin-arm64.tar.gz` / `pi-darwin-x64.tar.gz` release archive |
| Linux (and WSL) | `pi` | The same npm, pnpm, bun, and curl routes; `pi-linux-x64.tar.gz` or `pi-linux-arm64.tar.gz` release archive |
| Windows | `pi` (shims `pi.cmd`, `pi.ps1`); `pi.exe` in the release archive | npm, pnpm, or bun global install; `powershell -c "irm https://pi.dev/install.ps1 \| iex"`; `pi-windows-x64.zip` or `pi-windows-arm64.zip` release archive |

The npm package declares `bin.pi` as `dist/bundle/cli.js`, a Node script. The release archives contain a standalone `pi` (macOS) or `pi.exe` (Windows) next to the docs and a native prebuild; they were listed, not run. The curl installer is a managed, npm-based installation that may install Node.js itself. Uninstalling Pi leaves settings, credentials, sessions, and packages under `~/.pi/agent`.

## Subcommands

| Command path | Runs without a terminal | Purpose |
| --- | --- | --- |
| `install <source>` | yes | Install a package source and add it to settings |
| `remove <source>` | yes | Remove a package source |
| `uninstall <source>` | yes | Alias of `remove` |
| `update [source\|self\|pi]` | yes | Update Pi, packages, one source, or model catalogs |
| `list` | yes | List installed packages |
| `config` | no | Terminal UI to enable or disable package resources |
| `auth` | yes | Print usage |
| `auth check` | yes | Report credential readiness (exit 0, 1, or 2) |
| `auth print-api-key` | yes | Print a resolved API key |
| `auth print-bearer-token` | yes | Print a resolved OAuth bearer token |
| `mcp` (0.99.0+) | yes | Print usage |
| `mcp add` | yes | Add or replace an MCP server |
| `mcp remove` | yes | Remove an MCP server |
| `mcp list` | yes | Connect to servers and report their state |
| `mcp login` | no | Browser sign-in to an OAuth server |
| `mcp logout` | yes | Delete stored OAuth credentials |

`install`, `remove`, and `list` can show a project-trust prompt only when stdin and stdout are terminals and trust is undecided; with no terminal they never prompt, and `--approve` or `--no-approve` removes the prompt. `update` never prompts and uses only the saved trust decision. Slash commands such as `/login` and `/model` exist inside the terminal UI and are not process subcommands.

```mermaid
flowchart TD
    A["pi argv"] --> B{"first word"}
    B -->|"install, remove, uninstall, update, list"| C["package command parser"]
    B -->|"config"| D["config UI (terminal)"]
    B -->|"auth"| E["auth parser, then root parser for --provider and --model"]
    B -->|"mcp (0.99.0+)"| F["mcp parser"]
    B -->|"anything else"| G["root parser"]
    G --> H{"stdout and stdin are terminals and no -p or --mode"}
    H -->|"yes"| I["interactive TUI"]
    H -->|"no"| J["print, json, or rpc mode"]
```

## CLI Switch Inventory

Inventoried paths: the root entrypoint, and every command path marked non-interactive above (`install`, `remove`, `uninstall`, `update`, `list`, `auth check`, `auth print-api-key`, `auth print-bearer-token`, `mcp add`, `mcp remove`, `mcp list`, `mcp logout`). `config` and `mcp login` are interactive and left out. The frontmatter holds the 66 records; this section summarizes them.

**How the parser reads values.** Pi uses no parsing library. The root parser is a hand-written loop in `dist/cli/args.js` that compares each argument to a switch spelling by exact equality, in both 0.87.1 and 1.0.0; the package commands, `auth`, and `mcp` have their own small loops in `package-manager-cli.js`, `cli/auth-command.js`, and `extensions/mcp/cli.js`. Consequences, all confirmed with disposable runs of both versions under an isolated environment with no credentials:

| Behavior | Evidence |
| --- | --- |
| Only the space-separated form is accepted. `--name=foo`, `--mode=json`, `--thinking=high`, `--tui-mode=regular`, `--list-models=x` are not read as the known switch; they become unknown (extension) flags and `--mode=json` exits 1 with `Unknown option: --mode`. | `--mode=json`, `--tui-mode=regular`, `--list-models=zzz`, and `pi install --local=1 foo`, `pi update --extension=foo`, `pi mcp list --json=1`, `pi mcp add s1 --url=…` runs |
| No spelling accepts an attached value: `-tread` exits 1 with `Unknown option: -tread`, so no `short_attached` form exists. | `-tread --version` run |
| A value switch consumes the next argument even when it starts with a dash. | Every value switch followed by `--version` printed no version; boolean switches followed by `--version` printed it |
| Short spellings are exact words, including multi-character ones: `-nt`, `-nbt`, `-ne`, `-ns`, `-np`, `-nc`, `-xt`, `-na`. | Help output and source |
| `--list-models` takes an optional search term; the term is the next argument unless it starts with `-` or `@`. | Source and run |
| `-p`/`--print` takes no value, but also moves the next argument into the prompt messages unless it starts with `@` or `-`. | Source; recorded as `none` because that argument is a message |

**Root switches.**

| Switch | Value | Notes |
| --- | --- | --- |
| `--provider` | string | 1.0.0: error without `--model` |
| `--model`, `--api-key`, `--session`, `--session-id`, `--fork`, `--session-dir` | string | |
| `--system-prompt`, `--append-system-prompt` | string | Spellings and type only; semantics belong to the `system-prompt` topic. `--append-system-prompt` repeats |
| `--mode` | string | `text`, `json`, or `rpc` |
| `--name`, `-n` | string | |
| `--models`, `--tools`/`-t`, `--exclude-tools`/`-xt` | string | One comma-separated value |
| `--thinking` | string | `off`, `minimal`, `low`, `medium`, `high`, `xhigh`, `max`; invalid value warns |
| `--extension`/`-e`, `--skill`, `--prompt-template`, `--theme` | string | Repeatable, one value each |
| `--use-theme`, `--tui-mode`, `--export` | string | `--export` takes an optional output path as the next positional message |
| `--list-models` | optional string | |
| `--print`/`-p`, `--continue`/`-c`, `--resume`/`-r`, `--no-session`, `--no-tools`/`-nt`, `--no-builtin-tools`/`-nbt`, `--no-extensions`/`-ne`, `--no-skills`/`-ns`, `--no-prompt-templates`/`-np`, `--no-themes`, `--no-context-files`/`-nc`, `--verbose`, `--approve`/`-a`, `--no-approve`/`-na`, `--offline`, `--version`/`-v` | none | |
| `--help`/`-h` | none | Accepted at every path |

Nothing in Pi's switch set is variadic. `--` ends option parsing; the remaining arguments become messages, except those starting with `@`, which are files.

**Package commands.** `-l`/`--local` (`install`, `remove`, `uninstall`); `--approve`/`-a` and `--no-approve`/`-na` (also `update`, `list`); `update` takes `--self`, `--extensions`, `--models`, `--all`, `--force` (no value) and `--extension <source>` (one value that must not start with a dash). `--models` takes a value at the root and none at `update`; `--extension` has an `-e` alias at the root and none at `update`.

**Auth commands.** `--provider <provider>` and `--model <model>` on all three, with at least one required; `--json`, `--credentials`, `--no-refresh` on `auth check`; `--min-expiry <duration>` (`30m`, `1h`, units `ms`, `s`, `m`, `h`) on `auth print-bearer-token`.

**MCP commands** (0.99.0+). `mcp list --json`; `mcp add` and `mcp remove` take `-l`/`--local`; `mcp add` takes `--url`, `--cwd`, `--bearer-token-env-var`, `--oauth-client-id`, `--oauth-client-secret`, `--oauth-client-name`, `--exposure`, `--description`, `--oauth-callback-port` (numeric), and repeatable `--env` and `--header` (`KEY=VALUE`). Option parsing in `mcp add` stops after the server name and the first command token, so a stdio command's own flags pass through.

Gaps: none of the switches is `unknown`. Extension-registered flags are outside this inventory.

## Configuration Discovery

The agent directory defaults to `~/.pi/agent` (`%USERPROFILE%\.pi\agent` on Windows) and moves with `PI_CODING_AGENT_DIR`. Project files live in `.pi/` under the working directory and load only after the project is trusted.

| File | Scope | Role |
| --- | --- | --- |
| `<agent-dir>/settings.json`, `.pi/settings.json` | user, project | Settings; project overrides user |
| `<agent-dir>/mcp.json`, `.pi/mcp.json` | user, project | MCP servers; written by `pi mcp add` and `remove` |
| `<agent-dir>/auth.json` | user | Credentials; written by `/login` |
| `<agent-dir>/models.json` | user | Custom endpoints and models |
| `<agent-dir>/trust.json` | user | Saved project trust decisions |
| `<agent-dir>/keybindings.json` | user | Keybindings |
| `<agent-dir>/SYSTEM.md`, `APPEND_SYSTEM.md`; `.pi/SYSTEM.md`, `APPEND_SYSTEM.md` | user, project | System prompt files; semantics in the `system-prompt` topic |
| `AGENTS.md` / `CLAUDE.md` (and `AGENTS.override.md`) in the agent directory, working directory, and parents | user, repo | Context files; need no trust; disabled by `--no-context-files` |
| `<agent-dir>/sessions/` | user | Session JSONL files |
| `<agent-dir>/mcp.log` | user | MCP connection log |

On this host the agent directory also holds extension, skill, npm, and backup files; credential contents were not read.

## Environment Variables

| Variable | Effect |
| --- | --- |
| `PI_CODING_AGENT_DIR` | Replaces the agent directory |
| `PI_CODING_AGENT_SESSION_DIR` | Replaces the session directory (`--session-dir` wins) |
| `PI_PACKAGE_DIR` | Overrides the package directory for immutable stores |
| `PI_OFFLINE` | Disables automatic network activity |
| `PI_SKIP_VERSION_CHECK` | Disables the latest-version request; set by `--offline` |
| `PI_TELEMETRY` | Overrides install and update telemetry and attribution headers |
| `PI_CACHE_RETENTION` | `long` requests extended prompt caching |
| `PI_SHARE_VIEWER_URL` | Base URL of `/share` |
| `PI_RADIUS_GATEWAY` | Radius gateway origin |
| `PI_OAUTH_CALLBACK_HOST` | Interface for the OAuth callback server |
| `PI_HARDWARE_CURSOR`, `PI_HYPERLINKS`, `PI_IMAGE_PROTOCOL`, `PI_TRUE_COLOR`, `PI_TUI_ESC_TIMEOUT` | Terminal rendering and input overrides |
| `PI_STARTUP_BENCHMARK`, `PI_TIMING`, `PI_EXPERIMENTAL` | Diagnostics; the benchmark exits 1 outside interactive mode |
| `VISUAL`, `EDITOR`, `HTTP_PROXY`, `HTTPS_PROXY` | External editor and proxies |
| `AI_AGENT=pi`, `PI_CODING_AGENT=true` | Set by Pi for child processes |
| `PI_SESSION_ID`, `PI_SESSION_FILE`, `PI_PROVIDER`, `PI_MODEL`, `PI_REASONING_LEVEL` | Set by Pi in its bash and powershell tool shells |

Provider API-key variables (`ANTHROPIC_API_KEY`, `OPENAI_API_KEY`, `AWS_*`, and many others) belong to the `model-config` topic.

## Machine Introspection

| Command | Output | Use |
| --- | --- | --- |
| `pi --version` | text | Bare version string |
| `pi --list-models [search]` | table | Models the local catalog and credentials offer; no JSON variant |
| `pi auth check --provider <p> --json` | JSON, exit 0/1/2 | Credential readiness; observed `{"status":"not_ready","provider":"nonexistent-zz","reason":"provider_not_found"}` |
| `pi mcp list --json` | JSON | MCP server state; connects to servers |
| `pi list` | text | Installed packages |
| `pi --mode rpc --no-session` | JSONL | `get_available_models`, `get_state`, `get_commands` per the RPC docs; not exercised |

No command was found that dumps configuration or its schema; checked `--help` of the root, package, auth, and mcp commands.

## Wrapper Notes

- Send each value as its own argument; equals forms silently turn into unknown extension flags and exit 1.
- A value switch swallows a following dash-prefixed token; an unknown `--flag` swallows the next token unless it starts with `-` or `@`.
- `-p` takes the next token as the prompt unless it starts with `@` or `-`; use `--` for prompts that may start with a dash. After `--`, a token starting with `@` is still a file argument.
- A non-terminal stdin or stdout forces print mode, and piped stdin is prepended to the first prompt, so only a terminal with `pi -- "<prompt>"` starts an interactive session with a first message.
- `--offline` is detected anywhere in argv, even after `--`.
- 1.0.0 defaults to a fullscreen TUI (pass `--tui-mode regular` for scrollback), makes `--provider` without `--model` an error, and `--no-extensions` also disables built-in extensions.
- Without a terminal Pi never prompts for project trust; pass `--approve` or `--no-approve`. Add `--no-extensions`, `--no-skills`, `--no-prompt-templates`, and `--no-context-files` for isolation.
- `pi config` and `pi mcp login` need a terminal or browser; `pi install` and `pi update` run package managers and change the installation or settings.
- `auth print-*` and `auth check --credentials` print secrets to stdout; `--api-key` puts a secret in the process list.
- On native Windows the bash tool needs Git Bash or another `bash.exe`; `shellPath` overrides discovery.
- Pi sets `AI_AGENT=pi` and `PI_CODING_AGENT=true` for children.

## Sources

- [Pi homepage](https://pi.dev/)
- [Pi repository](https://github.com/earendil-works/pi)
- [Pi documentation](https://pi.dev/docs/latest)
- [CLI reference](https://github.com/earendil-works/pi/blob/main/packages/coding-agent/docs/cli.md)
- [Environment variables](https://github.com/earendil-works/pi/blob/main/packages/coding-agent/docs/environment-variables.md)
- [Configuration](https://github.com/earendil-works/pi/blob/main/packages/coding-agent/docs/configuration.md)
- [Security and project trust](https://github.com/earendil-works/pi/blob/main/packages/coding-agent/docs/security.md)
- [Windows setup](https://github.com/earendil-works/pi/blob/main/packages/coding-agent/docs/windows.md)
- [Changelog](https://github.com/earendil-works/pi/blob/main/packages/coding-agent/CHANGELOG.md)
- [GitHub release v1.0.0](https://github.com/earendil-works/pi/releases/tag/v1.0.0)
- [npm package](https://www.npmjs.com/package/@earendil-works/pi-coding-agent)
- Local: `pi --version`, `pi --help` (0.87.1); `npm view` and `npm pack` of `@earendil-works/pi-coding-agent@1.0.0`; source files `dist/cli/args.js`, `dist/main.js`, `dist/package-manager-cli.js`, `dist/cli/auth-command.js`, `dist/extensions/mcp/cli.js`; disposable runs of both versions

## Changelog

- 2026-10-01: Rewritten for contract revision 2; verified 0.87.1 (installed) and 1.0.0 (latest).
- 2026-10-01: Added the `auth` commands, `--use-theme`, `--tui-mode`, and the 1.0.0 `mcp` commands; recorded that built-in MCP exists since 0.99.0.
- 2026-10-01: Corrected `--export`: the output path is the first positional message, so `--export <input> <output>` works.
- 2026-10-01: Recorded space-only attachment, no attached short values, and dash-prefixed value consumption.
- 2026-10-01: Recorded 1.0.0 changes: fullscreen default, `--provider` requires `--model`, `--no-extensions` also disables built-in extensions.
- 2026-10-01: Added standalone release archives (`pi`, `pi.exe`), the PowerShell installer, `mcp.json`, `mcp.log`, and session environment variables.
- 2026-09-28: Recorded the interactive startup-prompt form `pi -- "<prompt>"`, which needs 0.84.3 or later (earlier entry, kept; the `--` branch was re-read in the 0.87.1 and 1.0.0 parsers).
- 2026-07-03: Previous research against 0.80.3 and a stale 0.73.1 install.