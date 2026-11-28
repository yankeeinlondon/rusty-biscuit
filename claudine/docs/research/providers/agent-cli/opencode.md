---
$schema: ./_schema.yaml
schema_revision: 2
provider: opencode
created: 2026-05-12
last_updated: 2026-10-01
agent: codex
model: gpt-6.1-sol
reasoning_effort: medium
latest_version: 1.18.34
versions_examined:
- 1.18.33
evidence:
- claim: OpenCode locks yargs 18.0.0 and its yargs-parser 22.0.0. Default greedy arrays, omitted scalar values, camel-case aliases, boolean negation, space/equals and restricted short attachment are established by parse/eatArray/defaultValue.
  id: parser
  limitations: Dependency source, plus disposable direct-parser probes; short attachment is conditional on the value syntax, and array minimum zero is not representable in the research schema.
  location: https://github.com/yargs/yargs-parser/blob/v22.0.0/lib/yargs-parser.ts
  method: source_code
  observed_on: 2026-10-01
  version: 1.18.33
- claim: Installed 1.18.33 help inspected at all listed paths, including hidden generate/console and aliases.
  id: local-help
  limitations: Help establishes advertised spellings and command registration; help alone does not prove value consumption.
  location: /tmp/opencode-cli-research/helps.json
  method: local_inspection
  observed_on: 2026-10-01
  version: 1.18.33
- claim: Global help/version aliases, print-logs, log-level, pure, command registration, strict parsing and populate-- configuration.
  id: entry
  limitations: Source establishes parser declarations, not successful runtime execution of model or management commands.
  location: https://github.com/anomalyco/opencode/blob/v1.18.33/packages/opencode/src/index.ts
  method: source_code
  observed_on: 2026-10-01
  version: 1.18.33
- claim: Typed switch and alias declarations for the root TUI.
  id: cmd-tui
  limitations: Source establishes parser declarations, not successful runtime execution of model or management commands.
  location: https://github.com/anomalyco/opencode/blob/v1.18.33/packages/opencode/src/cli/cmd/tui.ts
  method: source_code
  observed_on: 2026-10-01
  version: 1.18.33
- claim: Typed switch and alias declarations for run.
  id: cmd-run
  limitations: Source establishes parser declarations, not successful runtime execution of model or management commands.
  location: https://github.com/anomalyco/opencode/blob/v1.18.33/packages/opencode/src/cli/cmd/run.ts
  method: source_code
  observed_on: 2026-10-01
  version: 1.18.33
- claim: Typed switch and alias declarations for models.
  id: cmd-models
  limitations: Source establishes parser declarations, not successful runtime execution of model or management commands.
  location: https://github.com/anomalyco/opencode/blob/v1.18.33/packages/opencode/src/cli/cmd/models.ts
  method: source_code
  observed_on: 2026-10-01
  version: 1.18.33
- claim: Typed switch and alias declarations for stats.
  id: cmd-stats
  limitations: Source establishes parser declarations, not successful runtime execution of model or management commands.
  location: https://github.com/anomalyco/opencode/blob/v1.18.33/packages/opencode/src/cli/cmd/stats.ts
  method: source_code
  observed_on: 2026-10-01
  version: 1.18.33
- claim: Typed switch and alias declarations for agent.
  id: cmd-agent
  limitations: Source establishes parser declarations, not successful runtime execution of model or management commands.
  location: https://github.com/anomalyco/opencode/blob/v1.18.33/packages/opencode/src/cli/cmd/agent.ts
  method: source_code
  observed_on: 2026-10-01
  version: 1.18.33
- claim: Typed switch and alias declarations for uninstall.
  id: cmd-uninstall
  limitations: Source establishes parser declarations, not successful runtime execution of model or management commands.
  location: https://github.com/anomalyco/opencode/blob/v1.18.33/packages/opencode/src/cli/cmd/uninstall.ts
  method: source_code
  observed_on: 2026-10-01
  version: 1.18.33
- claim: Typed switch and alias declarations for plugin.
  id: cmd-plug
  limitations: Source establishes parser declarations, not successful runtime execution of model or management commands.
  location: https://github.com/anomalyco/opencode/blob/v1.18.33/packages/opencode/src/cli/cmd/plug.ts
  method: source_code
  observed_on: 2026-10-01
  version: 1.18.33
- claim: Typed switch and alias declarations for mcp.
  id: cmd-mcp
  limitations: Source establishes parser declarations, not successful runtime execution of model or management commands.
  location: https://github.com/anomalyco/opencode/blob/v1.18.33/packages/opencode/src/cli/cmd/mcp.ts
  method: source_code
  observed_on: 2026-10-01
  version: 1.18.33
- claim: Typed switch and alias declarations for acp.
  id: cmd-acp
  limitations: Source establishes parser declarations, not successful runtime execution of model or management commands.
  location: https://github.com/anomalyco/opencode/blob/v1.18.33/packages/opencode/src/cli/cmd/acp.ts
  method: source_code
  observed_on: 2026-10-01
  version: 1.18.33
- claim: Typed switch and alias declarations for providers.
  id: cmd-providers
  limitations: Source establishes parser declarations, not successful runtime execution of model or management commands.
  location: https://github.com/anomalyco/opencode/blob/v1.18.33/packages/opencode/src/cli/cmd/providers.ts
  method: source_code
  observed_on: 2026-10-01
  version: 1.18.33
- claim: Typed switch and alias declarations for upgrade.
  id: cmd-upgrade
  limitations: Source establishes parser declarations, not successful runtime execution of model or management commands.
  location: https://github.com/anomalyco/opencode/blob/v1.18.33/packages/opencode/src/cli/cmd/upgrade.ts
  method: source_code
  observed_on: 2026-10-01
  version: 1.18.33
- claim: Typed switch and alias declarations for export.
  id: cmd-export
  limitations: Source establishes parser declarations, not successful runtime execution of model or management commands.
  location: https://github.com/anomalyco/opencode/blob/v1.18.33/packages/opencode/src/cli/cmd/export.ts
  method: source_code
  observed_on: 2026-10-01
  version: 1.18.33
- claim: Typed switch and alias declarations for session.
  id: cmd-session
  limitations: Source establishes parser declarations, not successful runtime execution of model or management commands.
  location: https://github.com/anomalyco/opencode/blob/v1.18.33/packages/opencode/src/cli/cmd/session.ts
  method: source_code
  observed_on: 2026-10-01
  version: 1.18.33
- claim: Typed switch and alias declarations for attach.
  id: cmd-attach
  limitations: Source establishes parser declarations, not successful runtime execution of model or management commands.
  location: https://github.com/anomalyco/opencode/blob/v1.18.33/packages/opencode/src/cli/cmd/attach.ts
  method: source_code
  observed_on: 2026-10-01
  version: 1.18.33
- claim: Typed switch and alias declarations for github.
  id: cmd-github
  limitations: Source establishes parser declarations, not successful runtime execution of model or management commands.
  location: https://github.com/anomalyco/opencode/blob/v1.18.33/packages/opencode/src/cli/cmd/github.ts
  method: source_code
  observed_on: 2026-10-01
  version: 1.18.33
- claim: Typed switch and alias declarations for db.
  id: cmd-db
  limitations: Source establishes parser declarations, not successful runtime execution of model or management commands.
  location: https://github.com/anomalyco/opencode/blob/v1.18.33/packages/opencode/src/cli/cmd/db.ts
  method: source_code
  observed_on: 2026-10-01
  version: 1.18.33
- claim: Typed switch and alias declarations for debug.
  id: cmd-debug-agent
  limitations: Source establishes parser declarations, not successful runtime execution of model or management commands.
  location: https://github.com/anomalyco/opencode/blob/v1.18.33/packages/opencode/src/cli/cmd/debug/agent.ts
  method: source_code
  observed_on: 2026-10-01
  version: 1.18.33
- claim: Typed switch and alias declarations for debug rg.
  id: cmd-debug-ripgrep
  limitations: Source establishes parser declarations, not successful runtime execution of model or management commands.
  location: https://github.com/anomalyco/opencode/blob/v1.18.33/packages/opencode/src/cli/cmd/debug/ripgrep.ts
  method: source_code
  observed_on: 2026-10-01
  version: 1.18.33
- claim: Shared network options on root, acp, serve and web; cors is a string array.
  id: network
  limitations: Source establishes parser declarations, not successful runtime execution of model or management commands.
  location: https://github.com/anomalyco/opencode/blob/v1.18.33/packages/opencode/src/cli/network.ts
  method: source_code
  observed_on: 2026-10-01
  version: 1.18.33
- claim: The yargs completion helper accepts --get-yargs-completions and passes subsequent raw arguments to the completion engine.
  id: completion-parser
  limitations: Internal shell-completion protocol, not an ordinary typed option; no arity guarantee for wrappers.
  location: https://github.com/yargs/yargs/blob/v18.0.0/lib/yargs-factory.ts
  method: source_code
  observed_on: 2026-10-01
  version: 1.18.33
- claim: Disposable locked-parser probe records consumed values, greedy arrays, omitted values, literal booleans and restricted short attachment.
  id: parser-probe
  limitations: Equivalent option declarations in the actual parser library; does not exercise OpenCode handlers or full-yargs choice validation.
  location: /tmp/opencode-cli-research/parser-probes.json
  method: disposable_test
  observed_on: 2026-10-01
  version: 1.18.33
homepage: https://opencode.ai
repo: https://github.com/anomalyco/opencode
docs: https://opencode.ai/docs/
cli_docs: https://opencode.ai/docs/cli/
binaries:
- alt_binaries: []
  binary: opencode
  notes: Official install documentation and release packaging use opencode; Windows npm creates platform shell shims. Installed macOS binary is /Users/ken/.opencode/bin/opencode, version 1.18.33.
  os: macos
- alt_binaries: []
  binary: opencode
  notes: Official install documentation and release packaging use opencode; Windows npm creates platform shell shims. Installed macOS binary is /Users/ken/.opencode/bin/opencode, version 1.18.33.
  os: linux
- alt_binaries:
  - opencode.exe
  - opencode.cmd
  - opencode.ps1
  binary: opencode
  notes: Official install documentation and release packaging use opencode; Windows npm creates platform shell shims. Installed macOS binary is /Users/ken/.opencode/bin/opencode, version 1.18.33.
  os: windows
install_methods:
- command: npm install -g opencode-ai
  method: npm
  notes: Official installation documentation; no installation performed.
  os: macos
- command: pnpm install -g opencode-ai
  method: other
  notes: Official installation documentation; no installation performed.
  os: macos
- command: yarn global add opencode-ai
  method: other
  notes: Official installation documentation; no installation performed.
  os: macos
- command: curl -fsSL https://opencode.ai/install | bash
  method: standalone_binary
  notes: Official install documentation; standalone script normally uses ~/.opencode/bin.
  os: macos
- command: bun install -g opencode-ai
  method: other
  notes: Official install documentation; standalone script normally uses ~/.opencode/bin.
  os: macos
- command: brew install anomalyco/tap/opencode
  method: brew
  notes: Official install documentation; standalone script normally uses ~/.opencode/bin.
  os: macos
- command: npm install -g opencode-ai
  method: npm
  notes: Official installation documentation; no installation performed.
  os: linux
- command: pnpm install -g opencode-ai
  method: other
  notes: Official installation documentation; no installation performed.
  os: linux
- command: yarn global add opencode-ai
  method: other
  notes: Official installation documentation; no installation performed.
  os: linux
- command: curl -fsSL https://opencode.ai/install | bash
  method: standalone_binary
  notes: Official install documentation; standalone script normally uses ~/.opencode/bin.
  os: linux
- command: bun install -g opencode-ai
  method: other
  notes: Official install documentation; standalone script normally uses ~/.opencode/bin.
  os: linux
- command: brew install anomalyco/tap/opencode
  method: brew
  notes: Official install documentation; standalone script normally uses ~/.opencode/bin.
  os: linux
- command: sudo pacman -S opencode
  method: package_manager
  notes: Official installation documentation; Docker example is interactive.
  os: linux
- command: paru -S opencode-bin
  method: package_manager
  notes: Official installation documentation; Docker example is interactive.
  os: linux
- command: docker run -it --rm ghcr.io/anomalyco/opencode
  method: other
  notes: Official installation documentation; Docker example is interactive.
  os: linux
- command: npm install -g opencode-ai
  method: npm
  notes: Official installation documentation; no installation performed.
  os: windows
- command: pnpm install -g opencode-ai
  method: other
  notes: Official installation documentation; no installation performed.
  os: windows
- command: yarn global add opencode-ai
  method: other
  notes: Official installation documentation; no installation performed.
  os: windows
- command: choco install opencode
  method: chocolatey
  notes: Official Windows install documentation; WSL is recommended, Windows Bun installation remains in progress.
  os: windows
- command: scoop install opencode
  method: scoop
  notes: Official Windows install documentation; WSL is recommended, Windows Bun installation remains in progress.
  os: windows
- command: mise use -g github:anomalyco/opencode
  method: other
  notes: Official Windows install documentation; WSL is recommended, Windows Bun installation remains in progress.
  os: windows
subcommands:
- description: start ACP (Agent Client Protocol) server
  name: acp
  non_interactive: false
  notes: Long-running service; does not run to completion.
- description: Groups agent creation and listing.
  name: agent
  non_interactive: false
  notes: Command group; select a leaf command.
- description: create a new agent
  name: agent create
  non_interactive: true
  notes: Only non-interactive with nonempty --path, --description, --mode and supplied --permissions (alias --tools). Generates an agent through an LLM and writes <path>/agents/<identifier>.md; not executed in this research.
- description: list all available agents
  name: agent list
  non_interactive: true
  notes: Finite command without user input according to its registered handler; help inspected, handler not executed unless noted below.
- description: attach to a running opencode server
  name: attach
  non_interactive: false
  notes: Requires a terminal/browser, or can prompt during installation.
- description: Groups auth operations.
  name: auth
  non_interactive: false
  notes: Command group; select a leaf command.
- description: list providers and credentials
  name: auth list
  non_interactive: true
  notes: Finite command without user input according to its registered handler; help inspected, handler not executed unless noted below.
- description: log in to a provider
  name: auth login
  non_interactive: false
  notes: Can request user selection, credentials or browser authorization; only help inspected.
- description: log out from a configured provider
  name: auth logout
  non_interactive: false
  notes: Can request user selection, credentials or browser authorization; only help inspected.
- description: list providers and credentials
  name: auth ls
  non_interactive: true
  notes: Finite command without user input according to its registered handler; help inspected, handler not executed unless noted below.
- description: Generate a shell completion script.
  name: completion
  non_interactive: true
  notes: Finite command without user input according to its registered handler; help inspected, handler not executed unless noted below.
- description: Groups console account operations.
  name: console
  non_interactive: false
  notes: 'Hidden registered command: absent from root help, present in console --help and index.ts.'
- description: log in to console
  name: console login
  non_interactive: false
  notes: Can request user selection, credentials or browser authorization; only help inspected.
- description: log out from console
  name: console logout
  non_interactive: false
  notes: Can request user selection, credentials or browser authorization; only help inspected.
- description: open active console account
  name: console open
  non_interactive: false
  notes: Can request user selection, credentials or browser authorization; only help inspected.
- description: list orgs
  name: console orgs
  non_interactive: true
  notes: Lists account organizations without selecting one; can perform account/network reads.
- description: switch active org
  name: console switch
  non_interactive: false
  notes: Can request user selection, credentials or browser authorization; only help inspected.
- description: database tools
  name: db
  non_interactive: true
  notes: Finite with a SQL query positional; bare db opens interactive sqlite3. Read-only SQL is a safe probe; arbitrary SQL can mutate state.
- description: print the database path
  name: db path
  non_interactive: true
  notes: Finite command without user input according to its registered handler; help inspected, handler not executed unless noted below.
- description: Groups troubleshooting tools.
  name: debug
  non_interactive: false
  notes: Command group; select a leaf command.
- description: show agent configuration details
  name: debug agent
  non_interactive: true
  notes: Shows agent configuration; --tool executes a tool with --params, so this diagnostic can have side effects.
- description: show resolved configuration
  name: debug config
  non_interactive: true
  notes: Finite command without user input according to its registered handler; help inspected, handler not executed unless noted below.
- description: Groups debug file operations.
  name: debug file
  non_interactive: false
  notes: Command group; select a leaf command.
- description: list files in a directory
  name: debug file list
  non_interactive: true
  notes: Finite command without user input according to its registered handler; help inspected, handler not executed unless noted below.
- description: read file contents as JSON
  name: debug file read
  non_interactive: true
  notes: Finite command without user input according to its registered handler; help inspected, handler not executed unless noted below.
- description: search files by query
  name: debug file search
  non_interactive: true
  notes: Finite command without user input according to its registered handler; help inspected, handler not executed unless noted below.
- description: show debug information
  name: debug info
  non_interactive: true
  notes: Finite command without user input according to its registered handler; help inspected, handler not executed unless noted below.
- description: Groups debug lsp operations.
  name: debug lsp
  non_interactive: false
  notes: Command group; select a leaf command.
- description: get diagnostics for a file
  name: debug lsp diagnostics
  non_interactive: true
  notes: Finite command without user input according to its registered handler; help inspected, handler not executed unless noted below.
- description: get symbols from a document
  name: debug lsp document-symbols
  non_interactive: true
  notes: Finite command without user input according to its registered handler; help inspected, handler not executed unless noted below.
- description: search workspace symbols
  name: debug lsp symbols
  non_interactive: true
  notes: Finite command without user input according to its registered handler; help inspected, handler not executed unless noted below.
- description: show global paths (data, config, cache, state)
  name: debug paths
  non_interactive: true
  notes: Finite command without user input according to its registered handler; help inspected, handler not executed unless noted below.
- description: Groups debug rg operations.
  name: debug rg
  non_interactive: false
  notes: Command group; select a leaf command.
- description: list files using ripgrep
  name: debug rg files
  non_interactive: true
  notes: Finite command without user input according to its registered handler; help inspected, handler not executed unless noted below.
- description: search file contents using ripgrep
  name: debug rg search
  non_interactive: true
  notes: Finite command without user input according to its registered handler; help inspected, handler not executed unless noted below.
- description: list all known projects
  name: debug scrap
  non_interactive: true
  notes: Finite command without user input according to its registered handler; help inspected, handler not executed unless noted below.
- description: list all available skills
  name: debug skill
  non_interactive: true
  notes: Finite command without user input according to its registered handler; help inspected, handler not executed unless noted below.
- description: Groups debug snapshot operations.
  name: debug snapshot
  non_interactive: false
  notes: Command group; select a leaf command.
- description: show diff for a snapshot hash
  name: debug snapshot diff
  non_interactive: true
  notes: Finite command without user input according to its registered handler; help inspected, handler not executed unless noted below.
- description: show patch for a snapshot hash
  name: debug snapshot patch
  non_interactive: true
  notes: Finite command without user input according to its registered handler; help inspected, handler not executed unless noted below.
- description: track current snapshot state
  name: debug snapshot track
  non_interactive: true
  notes: Creates/tracks a repository snapshot; may update snapshot storage.
- description: print startup timing
  name: debug startup
  non_interactive: true
  notes: Prints process performance.now() timing and exits; not a resolved-configuration benchmark.
- description: debug v2 catalog and built-in plugins
  name: debug v2
  non_interactive: true
  notes: Finite command without user input according to its registered handler; help inspected, handler not executed unless noted below.
- description: wait indefinitely (for debugging)
  name: debug wait
  non_interactive: false
  notes: Long-running service; does not run to completion.
- description: export session data as JSON
  name: export
  non_interactive: true
  notes: Supply sessionID; omitting it invokes a session selector. Exports JSON and --sanitize redacts transcript/file data.
- description: Emit the HTTP API OpenAPI document as JSON.
  name: generate
  non_interactive: true
  notes: Hidden registered command; emits OpenAPI JSON and is accepted by installed binary.
- description: Groups GitHub setup and agent execution.
  name: github
  non_interactive: false
  notes: Command group; select a leaf command.
- description: install the GitHub agent
  name: github install
  non_interactive: false
  notes: Can request user selection, credentials or browser authorization; only help inspected.
- description: run the GitHub agent
  name: github run
  non_interactive: true
  notes: CI-oriented; may call paid models, tools, git and GitHub. Not executed.
- description: import session data from JSON file or URL
  name: import
  non_interactive: true
  notes: Finite command without user input according to its registered handler; help inspected, handler not executed unless noted below.
- description: Groups MCP server operations.
  name: mcp
  non_interactive: false
  notes: Command group; select a leaf command.
- description: add an MCP server
  name: mcp add
  non_interactive: false
  notes: Can request user selection, credentials or browser authorization; only help inspected.
- description: authenticate with an OAuth-enabled MCP server
  name: mcp auth
  non_interactive: false
  notes: Can request user selection, credentials or browser authorization; only help inspected.
- description: list OAuth-capable MCP servers and their auth status
  name: mcp auth list
  non_interactive: true
  notes: Finite command without user input according to its registered handler; help inspected, handler not executed unless noted below.
- description: list OAuth-capable MCP servers and their auth status
  name: mcp auth ls
  non_interactive: true
  notes: Finite command without user input according to its registered handler; help inspected, handler not executed unless noted below.
- description: debug OAuth connection for an MCP server
  name: mcp debug
  non_interactive: true
  notes: Performs network/OAuth diagnostics without completing authorization; can attempt client registration.
- description: list MCP servers and their status
  name: mcp list
  non_interactive: true
  notes: Finite command without user input according to its registered handler; help inspected, handler not executed unless noted below.
- description: remove OAuth credentials for an MCP server
  name: mcp logout
  non_interactive: false
  notes: Can request user selection, credentials or browser authorization; only help inspected.
- description: list MCP servers and their status
  name: mcp ls
  non_interactive: true
  notes: Finite command without user input according to its registered handler; help inspected, handler not executed unless noted below.
- description: list all available models
  name: models
  non_interactive: true
  notes: Finite command without user input according to its registered handler; help inspected, handler not executed unless noted below.
- description: install plugin and update config
  name: plug
  non_interactive: true
  notes: Alias of plugin; same installation/configuration side effects.
- description: install plugin and update config
  name: plugin
  non_interactive: true
  notes: Alias plug; installs dependencies and updates opencode/tui configuration. No user selection in its command handler; not executed.
- description: fetch and checkout a GitHub PR branch, then run opencode
  name: pr
  non_interactive: false
  notes: Requires a terminal/browser, or can prompt during installation.
- description: Groups providers operations.
  name: providers
  non_interactive: false
  notes: Command group; select a leaf command.
- description: list providers and credentials
  name: providers list
  non_interactive: true
  notes: Finite command without user input according to its registered handler; help inspected, handler not executed unless noted below.
- description: log in to a provider
  name: providers login
  non_interactive: false
  notes: Can request user selection, credentials or browser authorization; only help inspected.
- description: log out from a configured provider
  name: providers logout
  non_interactive: false
  notes: Can request user selection, credentials or browser authorization; only help inspected.
- description: list providers and credentials
  name: providers ls
  non_interactive: true
  notes: Finite command without user input according to its registered handler; help inspected, handler not executed unless noted below.
- description: run opencode with a message
  name: run
  non_interactive: true
  notes: Finishes by default; --interactive/-i and --mini request a terminal. Resume uses --continue/-c or --session/-s on this same path, with optional --fork; there is no resume subcommand.
- description: starts a headless opencode server
  name: serve
  non_interactive: false
  notes: Long-running service; does not run to completion.
- description: Groups session listing and deletion.
  name: session
  non_interactive: false
  notes: Command group; select a leaf command.
- description: delete a session
  name: session delete
  non_interactive: true
  notes: Finite command without user input according to its registered handler; help inspected, handler not executed unless noted below.
- description: list sessions
  name: session list
  non_interactive: true
  notes: Pipes complete without a pager; --format json returns an array for nonempty results, but empty stdout for no sessions. TTY table output without --max-count opens a pager.
- description: show token usage and cost statistics
  name: stats
  non_interactive: true
  notes: Finite command without user input according to its registered handler; help inspected, handler not executed unless noted below.
- description: uninstall opencode and remove all related files
  name: uninstall
  non_interactive: true
  notes: Finite with --force; otherwise confirmation prompts. Destructive; not executed.
- description: upgrade opencode to the latest or a specific version
  name: upgrade
  non_interactive: false
  notes: Requires a terminal/browser, or can prompt during installation.
- description: start opencode server and open web interface
  name: web
  non_interactive: false
  notes: Requires a terminal/browser, or can prompt during installation.
cli_switches:
- aliases:
  - -h
  - --h
  attachment: []
  description: Show help.
  evidence_ids:
  - entry
  - parser
  - local-help
  - parser-probe
  flag: --help
  invocation_scope:
  - applies_to: global
  value_type: none
- aliases:
  - --no-h
  attachment: []
  description: Set --help to false through yargs boolean negation.
  evidence_ids:
  - entry
  - parser
  - parser-probe
  flag: --no-help
  invocation_scope:
  - applies_to: global
  value_type: none
- aliases:
  - -v
  - --v
  attachment: []
  description: Show version.
  evidence_ids:
  - entry
  - parser
  - local-help
  - parser-probe
  flag: --version
  invocation_scope:
  - applies_to: global
  value_type: none
- aliases:
  - --no-v
  attachment: []
  description: Set --version to false through yargs boolean negation.
  evidence_ids:
  - entry
  - parser
  - parser-probe
  flag: --no-version
  invocation_scope:
  - applies_to: global
  value_type: none
- aliases:
  - --printLogs
  attachment: []
  description: print logs to stderr
  evidence_ids:
  - entry
  - parser
  - local-help
  - parser-probe
  flag: --print-logs
  invocation_scope:
  - applies_to: global
  value_type: none
- aliases:
  - --no-printLogs
  attachment: []
  description: Set --print-logs to false through yargs boolean negation.
  evidence_ids:
  - entry
  - parser
  - parser-probe
  flag: --no-print-logs
  invocation_scope:
  - applies_to: global
  value_type: none
- aliases:
  - --logLevel
  attachment:
  - space
  - equals
  description: log level
  evidence_ids:
  - entry
  - parser
  - local-help
  - parser-probe
  flag: --log-level
  invocation_scope:
  - applies_to: global
  notes: 'No nargs/requiresArg declaration: omitted strings become empty strings, numbers undefined or their declared default. This describes parser acceptance; the handler may reject empty values.'
  value_optional: true
  value_type: string
- attachment: []
  description: run without external plugins
  evidence_ids:
  - entry
  - parser
  - local-help
  - parser-probe
  flag: --pure
  invocation_scope:
  - applies_to: global
  value_type: none
- attachment: []
  description: Set --pure to false through yargs boolean negation.
  evidence_ids:
  - entry
  - parser
  - parser-probe
  flag: --no-pure
  invocation_scope:
  - applies_to: global
  value_type: none
- aliases:
  - -m
  - --m
  attachment:
  - space
  - equals
  - short_attached
  description: model to use in the format of provider/model
  evidence_ids:
  - cmd-tui
  - parser
  - local-help
  - parser-probe
  flag: --model
  invocation_scope:
  - applies_to: command
    command: []
  notes: 'No nargs/requiresArg declaration: omitted strings become empty strings, numbers undefined or their declared default. This describes parser acceptance; the handler may reject empty values. Short attachment is restricted: a numeric suffix or punctuation immediately after the short letter is consumed; an alphabetic suffix is grouped as switches. Use space or equals for arbitrary strings.'
  value_optional: true
  value_type: string
- aliases:
  - -c
  - --c
  attachment: []
  description: continue the last session
  evidence_ids:
  - cmd-tui
  - parser
  - local-help
  - parser-probe
  flag: --continue
  invocation_scope:
  - applies_to: command
    command: []
  value_type: none
- aliases:
  - --no-c
  attachment: []
  description: Set --continue to false through yargs boolean negation.
  evidence_ids:
  - cmd-tui
  - parser
  - parser-probe
  flag: --no-continue
  invocation_scope:
  - applies_to: command
    command: []
  value_type: none
- aliases:
  - -s
  - --s
  attachment:
  - space
  - equals
  - short_attached
  description: session id to continue
  evidence_ids:
  - cmd-tui
  - parser
  - local-help
  - parser-probe
  flag: --session
  invocation_scope:
  - applies_to: command
    command: []
  notes: 'No nargs/requiresArg declaration: omitted strings become empty strings, numbers undefined or their declared default. This describes parser acceptance; the handler may reject empty values. Short attachment is restricted: a numeric suffix or punctuation immediately after the short letter is consumed; an alphabetic suffix is grouped as switches. Use space or equals for arbitrary strings.'
  value_optional: true
  value_type: string
- attachment: []
  description: fork the session when continuing (use with --continue or --session)
  evidence_ids:
  - cmd-tui
  - parser
  - local-help
  - parser-probe
  flag: --fork
  invocation_scope:
  - applies_to: command
    command: []
  value_type: none
- attachment: []
  description: Set --fork to false through yargs boolean negation.
  evidence_ids:
  - cmd-tui
  - parser
  - parser-probe
  flag: --no-fork
  invocation_scope:
  - applies_to: command
    command: []
  value_type: none
- attachment:
  - space
  - equals
  description: prompt to use
  evidence_ids:
  - cmd-tui
  - parser
  - local-help
  - parser-probe
  flag: --prompt
  invocation_scope:
  - applies_to: command
    command: []
  notes: 'No nargs/requiresArg declaration: omitted strings become empty strings, numbers undefined or their declared default. This describes parser acceptance; the handler may reject empty values. This is the TUI initial user prompt; no run-level system-prompt delivery flag was found. System-prompt semantics belong to that topic.'
  value_optional: true
  value_type: string
- attachment:
  - space
  - equals
  description: agent to use
  evidence_ids:
  - cmd-tui
  - parser
  - local-help
  - parser-probe
  flag: --agent
  invocation_scope:
  - applies_to: command
    command: []
  notes: 'No nargs/requiresArg declaration: omitted strings become empty strings, numbers undefined or their declared default. This describes parser acceptance; the handler may reject empty values.'
  value_optional: true
  value_type: string
- attachment: []
  description: auto-approve permissions that are not explicitly denied (dangerous!)
  evidence_ids:
  - cmd-tui
  - parser
  - local-help
  - parser-probe
  flag: --auto
  invocation_scope:
  - applies_to: command
    command: []
  value_type: none
- attachment: []
  description: Set --auto to false through yargs boolean negation.
  evidence_ids:
  - cmd-tui
  - parser
  - parser-probe
  flag: --no-auto
  invocation_scope:
  - applies_to: command
    command: []
  value_type: none
- attachment: []
  description: Hidden automatic approval switch.
  evidence_ids:
  - cmd-tui
  - parser
  - local-help
  - parser-probe
  flag: --yolo
  invocation_scope:
  - applies_to: command
    command: []
  notes: ' Hidden in ordinary help; declared in version-pinned source.'
  value_type: none
- attachment: []
  description: Set --yolo to false through yargs boolean negation.
  evidence_ids:
  - cmd-tui
  - parser
  - parser-probe
  flag: --no-yolo
  invocation_scope:
  - applies_to: command
    command: []
  value_type: none
- aliases:
  - --dangerouslySkipPermissions
  attachment: []
  description: Hidden automatic approval switch.
  evidence_ids:
  - cmd-tui
  - parser
  - local-help
  - parser-probe
  flag: --dangerously-skip-permissions
  invocation_scope:
  - applies_to: command
    command: []
  notes: ' Hidden in ordinary help; declared in version-pinned source.'
  value_type: none
- aliases:
  - --no-dangerouslySkipPermissions
  attachment: []
  description: Set --dangerously-skip-permissions to false through yargs boolean negation.
  evidence_ids:
  - cmd-tui
  - parser
  - parser-probe
  flag: --no-dangerously-skip-permissions
  invocation_scope:
  - applies_to: command
    command: []
  value_type: none
- attachment: []
  description: start the minimal interactive interface
  evidence_ids:
  - cmd-tui
  - parser
  - local-help
  - parser-probe
  flag: --mini
  invocation_scope:
  - applies_to: command
    command: []
  value_type: none
- attachment: []
  description: Set --mini to false through yargs boolean negation.
  evidence_ids:
  - cmd-tui
  - parser
  - parser-probe
  flag: --no-mini
  invocation_scope:
  - applies_to: command
    command: []
  value_type: none
- attachment: []
  description: Enable interactive session-history replay.
  evidence_ids:
  - cmd-tui
  - parser
  - local-help
  - parser-probe
  flag: --replay
  invocation_scope:
  - applies_to: command
    command: []
  notes: ' Hidden in ordinary help; declared in version-pinned source.'
  value_type: none
- aliases:
  - --noReplay
  attachment: []
  description: disable mini session history replay on resume and after resize
  evidence_ids:
  - cmd-tui
  - parser
  - local-help
  - parser-probe
  flag: --no-replay
  invocation_scope:
  - applies_to: command
    command: []
  value_type: none
- aliases:
  - --replayLimit
  attachment:
  - space
  - equals
  description: cap visible mini replay to the newest N messages
  evidence_ids:
  - cmd-tui
  - parser
  - local-help
  - parser-probe
  flag: --replay-limit
  invocation_scope:
  - applies_to: command
    command: []
  notes: 'No nargs/requiresArg declaration: omitted strings become empty strings, numbers undefined or their declared default. This describes parser acceptance; the handler may reject empty values.'
  value_optional: true
  value_type: number
- attachment: []
  description: Enable interactive demo commands.
  evidence_ids:
  - cmd-tui
  - parser
  - local-help
  - parser-probe
  flag: --demo
  invocation_scope:
  - applies_to: command
    command: []
  notes: ' Hidden in ordinary help; declared in version-pinned source.'
  value_type: none
- attachment: []
  description: Set --demo to false through yargs boolean negation.
  evidence_ids:
  - cmd-tui
  - parser
  - parser-probe
  flag: --no-demo
  invocation_scope:
  - applies_to: command
    command: []
  value_type: none
- attachment:
  - space
  - equals
  description: the command to run, use message for args
  evidence_ids:
  - cmd-run
  - parser
  - local-help
  - parser-probe
  flag: --command
  invocation_scope:
  - applies_to: command
    command:
    - run
  notes: 'No nargs/requiresArg declaration: omitted strings become empty strings, numbers undefined or their declared default. This describes parser acceptance; the handler may reject empty values.'
  value_optional: true
  value_type: string
- aliases:
  - -c
  - --c
  attachment: []
  description: continue the last session
  evidence_ids:
  - cmd-run
  - parser
  - local-help
  - parser-probe
  flag: --continue
  invocation_scope:
  - applies_to: command
    command:
    - run
  value_type: none
- aliases:
  - --no-c
  attachment: []
  description: Set --continue to false through yargs boolean negation.
  evidence_ids:
  - cmd-run
  - parser
  - parser-probe
  flag: --no-continue
  invocation_scope:
  - applies_to: command
    command:
    - run
  value_type: none
- aliases:
  - -s
  - --s
  attachment:
  - space
  - equals
  - short_attached
  description: session id to continue
  evidence_ids:
  - cmd-run
  - parser
  - local-help
  - parser-probe
  flag: --session
  invocation_scope:
  - applies_to: command
    command:
    - run
  notes: 'No nargs/requiresArg declaration: omitted strings become empty strings, numbers undefined or their declared default. This describes parser acceptance; the handler may reject empty values. Short attachment is restricted: a numeric suffix or punctuation immediately after the short letter is consumed; an alphabetic suffix is grouped as switches. Use space or equals for arbitrary strings.'
  value_optional: true
  value_type: string
- attachment: []
  description: fork the session before continuing (requires --continue or --session)
  evidence_ids:
  - cmd-run
  - parser
  - local-help
  - parser-probe
  flag: --fork
  invocation_scope:
  - applies_to: command
    command:
    - run
  value_type: none
- attachment: []
  description: Set --fork to false through yargs boolean negation.
  evidence_ids:
  - cmd-run
  - parser
  - parser-probe
  flag: --no-fork
  invocation_scope:
  - applies_to: command
    command:
    - run
  value_type: none
- attachment: []
  description: share the session
  evidence_ids:
  - cmd-run
  - parser
  - local-help
  - parser-probe
  flag: --share
  invocation_scope:
  - applies_to: command
    command:
    - run
  value_type: none
- attachment: []
  description: Set --share to false through yargs boolean negation.
  evidence_ids:
  - cmd-run
  - parser
  - parser-probe
  flag: --no-share
  invocation_scope:
  - applies_to: command
    command:
    - run
  value_type: none
- aliases:
  - -m
  - --m
  attachment:
  - space
  - equals
  - short_attached
  description: model to use in the format of provider/model
  evidence_ids:
  - cmd-run
  - parser
  - local-help
  - parser-probe
  flag: --model
  invocation_scope:
  - applies_to: command
    command:
    - run
  notes: 'No nargs/requiresArg declaration: omitted strings become empty strings, numbers undefined or their declared default. This describes parser acceptance; the handler may reject empty values. Short attachment is restricted: a numeric suffix or punctuation immediately after the short letter is consumed; an alphabetic suffix is grouped as switches. Use space or equals for arbitrary strings.'
  value_optional: true
  value_type: string
- attachment:
  - space
  - equals
  description: agent to use
  evidence_ids:
  - cmd-run
  - parser
  - local-help
  - parser-probe
  flag: --agent
  invocation_scope:
  - applies_to: command
    command:
    - run
  notes: 'No nargs/requiresArg declaration: omitted strings become empty strings, numbers undefined or their declared default. This describes parser acceptance; the handler may reject empty values.'
  value_optional: true
  value_type: string
- attachment:
  - space
  - equals
  default: default
  description: 'format: default (formatted) or json (raw JSON events)'
  evidence_ids:
  - cmd-run
  - parser
  - local-help
  - parser-probe
  flag: --format
  invocation_scope:
  - applies_to: command
    command:
    - run
  notes: 'No nargs/requiresArg declaration: omitted strings become empty strings, numbers undefined or their declared default. This describes parser acceptance; the handler may reject empty values.'
  value_optional: true
  value_type: string
- aliases:
  - -f
  - --f
  attachment:
  - space
  - equals
  - short_attached
  description: file(s) to attach to message
  evidence_ids:
  - cmd-run
  - parser
  - local-help
  - parser-probe
  flag: --file
  gap: Parser eatArray and the direct parser probe establish a zero-value minimum. Revision 2 allows only positive minima or unknown; a schema revision supporting zero and rerunning the same empty-array probe would settle a faithfully encodable minimum.
  invocation_scope:
  - applies_to: command
    command:
    - run
  notes: 'No nargs/requiresArg declaration: omitted strings become empty strings, numbers undefined or their declared default. This describes parser acceptance; the handler may reject empty values. Short attachment is restricted: a numeric suffix or punctuation immediately after the short letter is consumed; an alphabetic suffix is grouped as switches. Use space or equals for arbitrary strings. Arrays greedily consume following non-switch tokens even after equals. -f/path consumes the attached path but leaves the next token positional; -f=path uses greedy array parsing.'
  value_type: variadic
  variadic_min: unknown
- attachment:
  - space
  - equals
  description: title for the session (uses truncated prompt if no value provided)
  evidence_ids:
  - cmd-run
  - parser
  - local-help
  - parser-probe
  flag: --title
  invocation_scope:
  - applies_to: command
    command:
    - run
  notes: 'No nargs/requiresArg declaration: omitted strings become empty strings, numbers undefined or their declared default. This describes parser acceptance; the handler may reject empty values.'
  value_optional: true
  value_type: string
- attachment:
  - space
  - equals
  description: attach to a running opencode server (e.g., http://localhost:4096)
  evidence_ids:
  - cmd-run
  - parser
  - local-help
  - parser-probe
  flag: --attach
  invocation_scope:
  - applies_to: command
    command:
    - run
  notes: 'No nargs/requiresArg declaration: omitted strings become empty strings, numbers undefined or their declared default. This describes parser acceptance; the handler may reject empty values.'
  value_optional: true
  value_type: string
- aliases:
  - -p
  - --p
  attachment:
  - space
  - equals
  - short_attached
  description: basic auth password (defaults to OPENCODE_SERVER_PASSWORD)
  evidence_ids:
  - cmd-run
  - parser
  - local-help
  - parser-probe
  flag: --password
  invocation_scope:
  - applies_to: command
    command:
    - run
  notes: 'No nargs/requiresArg declaration: omitted strings become empty strings, numbers undefined or their declared default. This describes parser acceptance; the handler may reject empty values. Short attachment is restricted: a numeric suffix or punctuation immediately after the short letter is consumed; an alphabetic suffix is grouped as switches. Use space or equals for arbitrary strings.'
  value_optional: true
  value_type: string
- aliases:
  - -u
  - --u
  attachment:
  - space
  - equals
  - short_attached
  description: basic auth username (defaults to OPENCODE_SERVER_USERNAME or 'opencode')
  evidence_ids:
  - cmd-run
  - parser
  - local-help
  - parser-probe
  flag: --username
  invocation_scope:
  - applies_to: command
    command:
    - run
  notes: 'No nargs/requiresArg declaration: omitted strings become empty strings, numbers undefined or their declared default. This describes parser acceptance; the handler may reject empty values. Short attachment is restricted: a numeric suffix or punctuation immediately after the short letter is consumed; an alphabetic suffix is grouped as switches. Use space or equals for arbitrary strings.'
  value_optional: true
  value_type: string
- attachment:
  - space
  - equals
  description: directory to run in, path on remote server if attaching
  evidence_ids:
  - cmd-run
  - parser
  - local-help
  - parser-probe
  flag: --dir
  invocation_scope:
  - applies_to: command
    command:
    - run
  notes: 'No nargs/requiresArg declaration: omitted strings become empty strings, numbers undefined or their declared default. This describes parser acceptance; the handler may reject empty values.'
  value_optional: true
  value_type: string
- attachment:
  - space
  - equals
  description: port for the local server (defaults to random port if no value provided)
  evidence_ids:
  - cmd-run
  - parser
  - local-help
  - parser-probe
  flag: --port
  invocation_scope:
  - applies_to: command
    command:
    - run
  notes: 'No nargs/requiresArg declaration: omitted strings become empty strings, numbers undefined or their declared default. This describes parser acceptance; the handler may reject empty values.'
  value_optional: true
  value_type: number
- attachment:
  - space
  - equals
  description: model variant (provider-specific reasoning effort, e.g., high, max, minimal)
  evidence_ids:
  - cmd-run
  - parser
  - local-help
  - parser-probe
  flag: --variant
  invocation_scope:
  - applies_to: command
    command:
    - run
  notes: 'No nargs/requiresArg declaration: omitted strings become empty strings, numbers undefined or their declared default. This describes parser acceptance; the handler may reject empty values.'
  value_optional: true
  value_type: string
- attachment: []
  description: show thinking blocks
  evidence_ids:
  - cmd-run
  - parser
  - local-help
  - parser-probe
  flag: --thinking
  invocation_scope:
  - applies_to: command
    command:
    - run
  value_type: none
- attachment: []
  description: Set --thinking to false through yargs boolean negation.
  evidence_ids:
  - cmd-run
  - parser
  - parser-probe
  flag: --no-thinking
  invocation_scope:
  - applies_to: command
    command:
    - run
  value_type: none
- attachment: []
  description: Select the minimal interactive interface.
  evidence_ids:
  - cmd-run
  - parser
  - local-help
  - parser-probe
  flag: --mini
  invocation_scope:
  - applies_to: command
    command:
    - run
  notes: ' Hidden in ordinary help; declared in version-pinned source.'
  value_type: none
- attachment: []
  description: Set --mini to false through yargs boolean negation.
  evidence_ids:
  - cmd-run
  - parser
  - parser-probe
  flag: --no-mini
  invocation_scope:
  - applies_to: command
    command:
    - run
  value_type: none
- attachment: []
  description: replay interactive session history on resume and after resize (use --no-replay to disable)
  evidence_ids:
  - cmd-run
  - parser
  - local-help
  - parser-probe
  flag: --replay
  invocation_scope:
  - applies_to: command
    command:
    - run
  notes: ' Hidden in ordinary help; declared in version-pinned source.'
  value_type: none
- aliases:
  - --replayLimit
  attachment:
  - space
  - equals
  description: cap visible interactive replay to the newest N messages
  evidence_ids:
  - cmd-run
  - parser
  - local-help
  - parser-probe
  flag: --replay-limit
  invocation_scope:
  - applies_to: command
    command:
    - run
  notes: 'No nargs/requiresArg declaration: omitted strings become empty strings, numbers undefined or their declared default. This describes parser acceptance; the handler may reject empty values. Hidden in ordinary help; declared in version-pinned source.'
  value_optional: true
  value_type: number
- aliases:
  - -i
  - --i
  attachment: []
  description: run in direct interactive split-footer mode
  evidence_ids:
  - cmd-run
  - parser
  - local-help
  - parser-probe
  flag: --interactive
  invocation_scope:
  - applies_to: command
    command:
    - run
  value_type: none
- aliases:
  - --no-i
  attachment: []
  description: Set --interactive to false through yargs boolean negation.
  evidence_ids:
  - cmd-run
  - parser
  - parser-probe
  flag: --no-interactive
  invocation_scope:
  - applies_to: command
    command:
    - run
  value_type: none
- attachment: []
  description: auto-approve permissions that are not explicitly denied (dangerous!)
  evidence_ids:
  - cmd-run
  - parser
  - local-help
  - parser-probe
  flag: --auto
  invocation_scope:
  - applies_to: command
    command:
    - run
  value_type: none
- attachment: []
  description: Set --auto to false through yargs boolean negation.
  evidence_ids:
  - cmd-run
  - parser
  - parser-probe
  flag: --no-auto
  invocation_scope:
  - applies_to: command
    command:
    - run
  value_type: none
- attachment: []
  description: Hidden automatic approval switch.
  evidence_ids:
  - cmd-run
  - parser
  - local-help
  - parser-probe
  flag: --yolo
  invocation_scope:
  - applies_to: command
    command:
    - run
  notes: ' Hidden in ordinary help; declared in version-pinned source.'
  value_type: none
- attachment: []
  description: Set --yolo to false through yargs boolean negation.
  evidence_ids:
  - cmd-run
  - parser
  - parser-probe
  flag: --no-yolo
  invocation_scope:
  - applies_to: command
    command:
    - run
  value_type: none
- aliases:
  - --dangerouslySkipPermissions
  attachment: []
  description: Hidden automatic approval switch.
  evidence_ids:
  - cmd-run
  - parser
  - local-help
  - parser-probe
  flag: --dangerously-skip-permissions
  invocation_scope:
  - applies_to: command
    command:
    - run
  notes: ' Hidden in ordinary help; declared in version-pinned source.'
  value_type: none
- aliases:
  - --no-dangerouslySkipPermissions
  attachment: []
  description: Set --dangerously-skip-permissions to false through yargs boolean negation.
  evidence_ids:
  - cmd-run
  - parser
  - parser-probe
  flag: --no-dangerously-skip-permissions
  invocation_scope:
  - applies_to: command
    command:
    - run
  value_type: none
- attachment: []
  description: enable direct interactive demo slash commands; pass one as the message to run it immediately
  evidence_ids:
  - cmd-run
  - parser
  - local-help
  - parser-probe
  flag: --demo
  invocation_scope:
  - applies_to: command
    command:
    - run
  notes: ' Hidden in ordinary help; declared in version-pinned source.'
  value_type: none
- attachment: []
  description: Set --demo to false through yargs boolean negation.
  evidence_ids:
  - cmd-run
  - parser
  - parser-probe
  flag: --no-demo
  invocation_scope:
  - applies_to: command
    command:
    - run
  value_type: none
- attachment: []
  description: use more verbose model output (includes metadata like costs)
  evidence_ids:
  - cmd-models
  - parser
  - local-help
  - parser-probe
  flag: --verbose
  invocation_scope:
  - applies_to: command
    command:
    - models
  value_type: none
- attachment: []
  description: Set --verbose to false through yargs boolean negation.
  evidence_ids:
  - cmd-models
  - parser
  - parser-probe
  flag: --no-verbose
  invocation_scope:
  - applies_to: command
    command:
    - models
  value_type: none
- attachment: []
  description: refresh the models cache from models.dev
  evidence_ids:
  - cmd-models
  - parser
  - local-help
  - parser-probe
  flag: --refresh
  invocation_scope:
  - applies_to: command
    command:
    - models
  value_type: none
- attachment: []
  description: Set --refresh to false through yargs boolean negation.
  evidence_ids:
  - cmd-models
  - parser
  - parser-probe
  flag: --no-refresh
  invocation_scope:
  - applies_to: command
    command:
    - models
  value_type: none
- attachment:
  - space
  - equals
  description: 'show stats for the last N days (default: all time)'
  evidence_ids:
  - cmd-stats
  - parser
  - local-help
  - parser-probe
  flag: --days
  invocation_scope:
  - applies_to: command
    command:
    - stats
  notes: 'No nargs/requiresArg declaration: omitted strings become empty strings, numbers undefined or their declared default. This describes parser acceptance; the handler may reject empty values.'
  value_optional: true
  value_type: number
- attachment:
  - space
  - equals
  description: 'number of tools to show (default: all)'
  evidence_ids:
  - cmd-stats
  - parser
  - local-help
  - parser-probe
  flag: --tools
  invocation_scope:
  - applies_to: command
    command:
    - stats
  notes: 'No nargs/requiresArg declaration: omitted strings become empty strings, numbers undefined or their declared default. This describes parser acceptance; the handler may reject empty values.'
  value_optional: true
  value_type: number
- attachment: []
  description: 'show model statistics (default: hidden). Pass a number to show top N, otherwise shows all'
  evidence_ids:
  - cmd-stats
  - parser
  - local-help
  - parser-probe
  flag: --models
  gap: This option has no declared type and supports a bare flag or a count. A disposable full-yargs probe of stats --models with absent, numeric and textual values, followed by its handler, would establish a catalog value type; do not infer number from its description.
  invocation_scope:
  - applies_to: command
    command:
    - stats
  value_type: unknown
- attachment:
  - space
  - equals
  description: 'filter by project (default: all projects, empty string: current project)'
  evidence_ids:
  - cmd-stats
  - parser
  - local-help
  - parser-probe
  flag: --project
  invocation_scope:
  - applies_to: command
    command:
    - stats
  notes: 'No nargs/requiresArg declaration: omitted strings become empty strings, numbers undefined or their declared default. This describes parser acceptance; the handler may reject empty values.'
  value_optional: true
  value_type: string
- attachment:
  - space
  - equals
  description: directory path to generate the agent file
  evidence_ids:
  - cmd-agent
  - parser
  - local-help
  - parser-probe
  flag: --path
  invocation_scope:
  - applies_to: command
    command:
    - agent
    - create
  notes: 'No nargs/requiresArg declaration: omitted strings become empty strings, numbers undefined or their declared default. This describes parser acceptance; the handler may reject empty values.'
  value_optional: true
  value_type: string
- attachment:
  - space
  - equals
  description: what the agent should do
  evidence_ids:
  - cmd-agent
  - parser
  - local-help
  - parser-probe
  flag: --description
  invocation_scope:
  - applies_to: command
    command:
    - agent
    - create
  notes: 'No nargs/requiresArg declaration: omitted strings become empty strings, numbers undefined or their declared default. This describes parser acceptance; the handler may reject empty values.'
  value_optional: true
  value_type: string
- attachment:
  - space
  - equals
  description: agent mode
  evidence_ids:
  - cmd-agent
  - parser
  - local-help
  - parser-probe
  flag: --mode
  invocation_scope:
  - applies_to: command
    command:
    - agent
    - create
  notes: 'No nargs/requiresArg declaration: omitted strings become empty strings, numbers undefined or their declared default. This describes parser acceptance; the handler may reject empty values.'
  value_optional: true
  value_type: string
- aliases:
  - --tools
  attachment:
  - space
  - equals
  description: Comma-separated permissions allowed in the generated agent; --tools is the same switch.
  evidence_ids:
  - cmd-agent
  - parser
  - local-help
  - parser-probe
  flag: --permissions
  invocation_scope:
  - applies_to: command
    command:
    - agent
    - create
  notes: 'No nargs/requiresArg declaration: omitted strings become empty strings, numbers undefined or their declared default. This describes parser acceptance; the handler may reject empty values. --tools is a long alias; this is a single comma-separated value, not an array.'
  value_optional: true
  value_type: string
- aliases:
  - -m
  - --m
  attachment:
  - space
  - equals
  - short_attached
  description: model to use in the format of provider/model
  evidence_ids:
  - cmd-agent
  - parser
  - local-help
  - parser-probe
  flag: --model
  invocation_scope:
  - applies_to: command
    command:
    - agent
    - create
  notes: 'No nargs/requiresArg declaration: omitted strings become empty strings, numbers undefined or their declared default. This describes parser acceptance; the handler may reject empty values. Short attachment is restricted: a numeric suffix or punctuation immediately after the short letter is consumed; an alphabetic suffix is grouped as switches. Use space or equals for arbitrary strings.'
  value_optional: true
  value_type: string
- aliases:
  - -c
  - --c
  - --keepConfig
  attachment: []
  description: keep configuration files
  evidence_ids:
  - cmd-uninstall
  - parser
  - local-help
  - parser-probe
  flag: --keep-config
  invocation_scope:
  - applies_to: command
    command:
    - uninstall
  value_type: none
- aliases:
  - --no-keepConfig
  - --no-c
  attachment: []
  description: Set --keep-config to false through yargs boolean negation.
  evidence_ids:
  - cmd-uninstall
  - parser
  - parser-probe
  flag: --no-keep-config
  invocation_scope:
  - applies_to: command
    command:
    - uninstall
  value_type: none
- aliases:
  - -d
  - --d
  - --keepData
  attachment: []
  description: keep session data and snapshots
  evidence_ids:
  - cmd-uninstall
  - parser
  - local-help
  - parser-probe
  flag: --keep-data
  invocation_scope:
  - applies_to: command
    command:
    - uninstall
  value_type: none
- aliases:
  - --no-keepData
  - --no-d
  attachment: []
  description: Set --keep-data to false through yargs boolean negation.
  evidence_ids:
  - cmd-uninstall
  - parser
  - parser-probe
  flag: --no-keep-data
  invocation_scope:
  - applies_to: command
    command:
    - uninstall
  value_type: none
- aliases:
  - --dryRun
  attachment: []
  description: show what would be removed without removing
  evidence_ids:
  - cmd-uninstall
  - parser
  - local-help
  - parser-probe
  flag: --dry-run
  invocation_scope:
  - applies_to: command
    command:
    - uninstall
  value_type: none
- aliases:
  - --no-dryRun
  attachment: []
  description: Set --dry-run to false through yargs boolean negation.
  evidence_ids:
  - cmd-uninstall
  - parser
  - parser-probe
  flag: --no-dry-run
  invocation_scope:
  - applies_to: command
    command:
    - uninstall
  value_type: none
- aliases:
  - -f
  - --f
  attachment: []
  description: skip confirmation prompts
  evidence_ids:
  - cmd-uninstall
  - parser
  - local-help
  - parser-probe
  flag: --force
  invocation_scope:
  - applies_to: command
    command:
    - uninstall
  value_type: none
- aliases:
  - --no-f
  attachment: []
  description: Set --force to false through yargs boolean negation.
  evidence_ids:
  - cmd-uninstall
  - parser
  - parser-probe
  flag: --no-force
  invocation_scope:
  - applies_to: command
    command:
    - uninstall
  value_type: none
- aliases:
  - -g
  - --g
  attachment: []
  description: install in global config
  evidence_ids:
  - cmd-plug
  - parser
  - local-help
  - parser-probe
  flag: --global
  invocation_scope:
  - applies_to: command
    command:
    - plugin
  value_type: none
- aliases:
  - --no-g
  attachment: []
  description: Set --global to false through yargs boolean negation.
  evidence_ids:
  - cmd-plug
  - parser
  - parser-probe
  flag: --no-global
  invocation_scope:
  - applies_to: command
    command:
    - plugin
  value_type: none
- aliases:
  - -g
  - --g
  attachment: []
  description: install in global config
  evidence_ids:
  - cmd-plug
  - parser
  - local-help
  - parser-probe
  flag: --global
  invocation_scope:
  - applies_to: command
    command:
    - plug
  value_type: none
- aliases:
  - --no-g
  attachment: []
  description: Set --global to false through yargs boolean negation.
  evidence_ids:
  - cmd-plug
  - parser
  - parser-probe
  flag: --no-global
  invocation_scope:
  - applies_to: command
    command:
    - plug
  value_type: none
- aliases:
  - -f
  - --f
  attachment: []
  description: replace existing plugin version
  evidence_ids:
  - cmd-plug
  - parser
  - local-help
  - parser-probe
  flag: --force
  invocation_scope:
  - applies_to: command
    command:
    - plugin
  value_type: none
- aliases:
  - --no-f
  attachment: []
  description: Set --force to false through yargs boolean negation.
  evidence_ids:
  - cmd-plug
  - parser
  - parser-probe
  flag: --no-force
  invocation_scope:
  - applies_to: command
    command:
    - plugin
  value_type: none
- aliases:
  - -f
  - --f
  attachment: []
  description: replace existing plugin version
  evidence_ids:
  - cmd-plug
  - parser
  - local-help
  - parser-probe
  flag: --force
  invocation_scope:
  - applies_to: command
    command:
    - plug
  value_type: none
- aliases:
  - --no-f
  attachment: []
  description: Set --force to false through yargs boolean negation.
  evidence_ids:
  - cmd-plug
  - parser
  - parser-probe
  flag: --no-force
  invocation_scope:
  - applies_to: command
    command:
    - plug
  value_type: none
- attachment:
  - space
  - equals
  description: URL for a remote MCP server
  evidence_ids:
  - cmd-mcp
  - parser
  - local-help
  - parser-probe
  flag: --url
  invocation_scope:
  - applies_to: command
    command:
    - mcp
    - add
  notes: 'No nargs/requiresArg declaration: omitted strings become empty strings, numbers undefined or their declared default. This describes parser acceptance; the handler may reject empty values.'
  value_optional: true
  value_type: string
- attachment:
  - space
  - equals
  description: environment variable for a local MCP server (KEY=VALUE)
  evidence_ids:
  - cmd-mcp
  - parser
  - local-help
  - parser-probe
  flag: --env
  gap: Parser eatArray and the direct parser probe establish a zero-value minimum. Revision 2 allows only positive minima or unknown; a schema revision supporting zero and rerunning the same empty-array probe would settle a faithfully encodable minimum.
  invocation_scope:
  - applies_to: command
    command:
    - mcp
    - add
  notes: 'No nargs/requiresArg declaration: omitted strings become empty strings, numbers undefined or their declared default. This describes parser acceptance; the handler may reject empty values. Arrays greedily consume following non-switch tokens, including after equals; an occurrence without values produces an empty array.'
  value_type: variadic
  variadic_min: unknown
- attachment:
  - space
  - equals
  description: HTTP header for a remote MCP server (KEY=VALUE)
  evidence_ids:
  - cmd-mcp
  - parser
  - local-help
  - parser-probe
  flag: --header
  gap: Parser eatArray and the direct parser probe establish a zero-value minimum. Revision 2 allows only positive minima or unknown; a schema revision supporting zero and rerunning the same empty-array probe would settle a faithfully encodable minimum.
  invocation_scope:
  - applies_to: command
    command:
    - mcp
    - add
  notes: 'No nargs/requiresArg declaration: omitted strings become empty strings, numbers undefined or their declared default. This describes parser acceptance; the handler may reject empty values. Arrays greedily consume following non-switch tokens, including after equals; an occurrence without values produces an empty array.'
  value_type: variadic
  variadic_min: unknown
- attachment:
  - space
  - equals
  description: working directory
  evidence_ids:
  - cmd-acp
  - parser
  - local-help
  - parser-probe
  flag: --cwd
  invocation_scope:
  - applies_to: command
    command:
    - acp
  notes: 'No nargs/requiresArg declaration: omitted strings become empty strings, numbers undefined or their declared default. This describes parser acceptance; the handler may reject empty values.'
  value_optional: true
  value_type: string
- aliases:
  - -p
  - --p
  attachment:
  - space
  - equals
  - short_attached
  description: provider id or name to log in to (skips provider selection)
  evidence_ids:
  - cmd-providers
  - parser
  - local-help
  - parser-probe
  flag: --provider
  invocation_scope:
  - applies_to: command
    command:
    - providers
    - login
  notes: 'No nargs/requiresArg declaration: omitted strings become empty strings, numbers undefined or their declared default. This describes parser acceptance; the handler may reject empty values. Short attachment is restricted: a numeric suffix or punctuation immediately after the short letter is consumed; an alphabetic suffix is grouped as switches. Use space or equals for arbitrary strings.'
  value_optional: true
  value_type: string
- aliases:
  - -p
  - --p
  attachment:
  - space
  - equals
  - short_attached
  description: provider id or name to log in to (skips provider selection)
  evidence_ids:
  - cmd-providers
  - parser
  - local-help
  - parser-probe
  flag: --provider
  invocation_scope:
  - applies_to: command
    command:
    - auth
    - login
  notes: 'No nargs/requiresArg declaration: omitted strings become empty strings, numbers undefined or their declared default. This describes parser acceptance; the handler may reject empty values. Short attachment is restricted: a numeric suffix or punctuation immediately after the short letter is consumed; an alphabetic suffix is grouped as switches. Use space or equals for arbitrary strings.'
  value_optional: true
  value_type: string
- aliases:
  - -m
  - --m
  attachment:
  - space
  - equals
  - short_attached
  description: login method label (skips method selection)
  evidence_ids:
  - cmd-providers
  - parser
  - local-help
  - parser-probe
  flag: --method
  invocation_scope:
  - applies_to: command
    command:
    - providers
    - login
  notes: 'No nargs/requiresArg declaration: omitted strings become empty strings, numbers undefined or their declared default. This describes parser acceptance; the handler may reject empty values. Short attachment is restricted: a numeric suffix or punctuation immediately after the short letter is consumed; an alphabetic suffix is grouped as switches. Use space or equals for arbitrary strings.'
  value_optional: true
  value_type: string
- aliases:
  - -m
  - --m
  attachment:
  - space
  - equals
  - short_attached
  description: login method label (skips method selection)
  evidence_ids:
  - cmd-providers
  - parser
  - local-help
  - parser-probe
  flag: --method
  invocation_scope:
  - applies_to: command
    command:
    - auth
    - login
  notes: 'No nargs/requiresArg declaration: omitted strings become empty strings, numbers undefined or their declared default. This describes parser acceptance; the handler may reject empty values. Short attachment is restricted: a numeric suffix or punctuation immediately after the short letter is consumed; an alphabetic suffix is grouped as switches. Use space or equals for arbitrary strings.'
  value_optional: true
  value_type: string
- aliases:
  - -m
  - --m
  attachment:
  - space
  - equals
  - short_attached
  description: installation method to use
  evidence_ids:
  - cmd-upgrade
  - parser
  - local-help
  - parser-probe
  flag: --method
  invocation_scope:
  - applies_to: command
    command:
    - upgrade
  notes: 'No nargs/requiresArg declaration: omitted strings become empty strings, numbers undefined or their declared default. This describes parser acceptance; the handler may reject empty values. Short attachment is restricted: a numeric suffix or punctuation immediately after the short letter is consumed; an alphabetic suffix is grouped as switches. Use space or equals for arbitrary strings.'
  value_optional: true
  value_type: string
- attachment: []
  description: redact sensitive transcript and file data
  evidence_ids:
  - cmd-export
  - parser
  - local-help
  - parser-probe
  flag: --sanitize
  invocation_scope:
  - applies_to: command
    command:
    - export
  value_type: none
- attachment: []
  description: Set --sanitize to false through yargs boolean negation.
  evidence_ids:
  - cmd-export
  - parser
  - parser-probe
  flag: --no-sanitize
  invocation_scope:
  - applies_to: command
    command:
    - export
  value_type: none
- aliases:
  - -n
  - --n
  - --maxCount
  attachment:
  - space
  - equals
  - short_attached
  description: limit to N most recent sessions
  evidence_ids:
  - cmd-session
  - parser
  - local-help
  - parser-probe
  flag: --max-count
  invocation_scope:
  - applies_to: command
    command:
    - session
    - list
  notes: 'No nargs/requiresArg declaration: omitted strings become empty strings, numbers undefined or their declared default. This describes parser acceptance; the handler may reject empty values. Short attachment is restricted: a numeric suffix or punctuation immediately after the short letter is consumed; an alphabetic suffix is grouped as switches. Use space or equals for arbitrary strings.'
  value_optional: true
  value_type: number
- attachment:
  - space
  - equals
  default: table
  description: output format
  evidence_ids:
  - cmd-session
  - parser
  - local-help
  - parser-probe
  flag: --format
  invocation_scope:
  - applies_to: command
    command:
    - session
    - list
  notes: 'No nargs/requiresArg declaration: omitted strings become empty strings, numbers undefined or their declared default. This describes parser acceptance; the handler may reject empty values.'
  value_optional: true
  value_type: string
- attachment:
  - space
  - equals
  description: directory to run in
  evidence_ids:
  - cmd-attach
  - parser
  - local-help
  - parser-probe
  flag: --dir
  invocation_scope:
  - applies_to: command
    command:
    - attach
  notes: 'No nargs/requiresArg declaration: omitted strings become empty strings, numbers undefined or their declared default. This describes parser acceptance; the handler may reject empty values.'
  value_optional: true
  value_type: string
- aliases:
  - -c
  - --c
  attachment: []
  description: continue the last session
  evidence_ids:
  - cmd-attach
  - parser
  - local-help
  - parser-probe
  flag: --continue
  invocation_scope:
  - applies_to: command
    command:
    - attach
  value_type: none
- aliases:
  - --no-c
  attachment: []
  description: Set --continue to false through yargs boolean negation.
  evidence_ids:
  - cmd-attach
  - parser
  - parser-probe
  flag: --no-continue
  invocation_scope:
  - applies_to: command
    command:
    - attach
  value_type: none
- aliases:
  - -s
  - --s
  attachment:
  - space
  - equals
  - short_attached
  description: session id to continue
  evidence_ids:
  - cmd-attach
  - parser
  - local-help
  - parser-probe
  flag: --session
  invocation_scope:
  - applies_to: command
    command:
    - attach
  notes: 'No nargs/requiresArg declaration: omitted strings become empty strings, numbers undefined or their declared default. This describes parser acceptance; the handler may reject empty values. Short attachment is restricted: a numeric suffix or punctuation immediately after the short letter is consumed; an alphabetic suffix is grouped as switches. Use space or equals for arbitrary strings.'
  value_optional: true
  value_type: string
- attachment: []
  description: fork the session when continuing (use with --continue or --session)
  evidence_ids:
  - cmd-attach
  - parser
  - local-help
  - parser-probe
  flag: --fork
  invocation_scope:
  - applies_to: command
    command:
    - attach
  value_type: none
- attachment: []
  description: Set --fork to false through yargs boolean negation.
  evidence_ids:
  - cmd-attach
  - parser
  - parser-probe
  flag: --no-fork
  invocation_scope:
  - applies_to: command
    command:
    - attach
  value_type: none
- aliases:
  - -p
  - --p
  attachment:
  - space
  - equals
  - short_attached
  description: basic auth password (defaults to OPENCODE_SERVER_PASSWORD)
  evidence_ids:
  - cmd-attach
  - parser
  - local-help
  - parser-probe
  flag: --password
  invocation_scope:
  - applies_to: command
    command:
    - attach
  notes: 'No nargs/requiresArg declaration: omitted strings become empty strings, numbers undefined or their declared default. This describes parser acceptance; the handler may reject empty values. Short attachment is restricted: a numeric suffix or punctuation immediately after the short letter is consumed; an alphabetic suffix is grouped as switches. Use space or equals for arbitrary strings.'
  value_optional: true
  value_type: string
- aliases:
  - -u
  - --u
  attachment:
  - space
  - equals
  - short_attached
  description: basic auth username (defaults to OPENCODE_SERVER_USERNAME or 'opencode')
  evidence_ids:
  - cmd-attach
  - parser
  - local-help
  - parser-probe
  flag: --username
  invocation_scope:
  - applies_to: command
    command:
    - attach
  notes: 'No nargs/requiresArg declaration: omitted strings become empty strings, numbers undefined or their declared default. This describes parser acceptance; the handler may reject empty values. Short attachment is restricted: a numeric suffix or punctuation immediately after the short letter is consumed; an alphabetic suffix is grouped as switches. Use space or equals for arbitrary strings.'
  value_optional: true
  value_type: string
- attachment: []
  description: start the minimal interactive interface
  evidence_ids:
  - cmd-attach
  - parser
  - local-help
  - parser-probe
  flag: --mini
  invocation_scope:
  - applies_to: command
    command:
    - attach
  value_type: none
- attachment: []
  description: Set --mini to false through yargs boolean negation.
  evidence_ids:
  - cmd-attach
  - parser
  - parser-probe
  flag: --no-mini
  invocation_scope:
  - applies_to: command
    command:
    - attach
  value_type: none
- attachment: []
  description: Enable interactive session-history replay.
  evidence_ids:
  - cmd-attach
  - parser
  - local-help
  - parser-probe
  flag: --replay
  invocation_scope:
  - applies_to: command
    command:
    - attach
  notes: ' Hidden in ordinary help; declared in version-pinned source.'
  value_type: none
- aliases:
  - --noReplay
  attachment: []
  description: disable mini session history replay on resume and after resize
  evidence_ids:
  - cmd-attach
  - parser
  - local-help
  - parser-probe
  flag: --no-replay
  invocation_scope:
  - applies_to: command
    command:
    - attach
  value_type: none
- aliases:
  - --replayLimit
  attachment:
  - space
  - equals
  description: cap visible mini replay to the newest N messages
  evidence_ids:
  - cmd-attach
  - parser
  - local-help
  - parser-probe
  flag: --replay-limit
  invocation_scope:
  - applies_to: command
    command:
    - attach
  notes: 'No nargs/requiresArg declaration: omitted strings become empty strings, numbers undefined or their declared default. This describes parser acceptance; the handler may reject empty values.'
  value_optional: true
  value_type: number
- attachment:
  - space
  - equals
  description: GitHub mock event to run the agent for
  evidence_ids:
  - cmd-github
  - parser
  - local-help
  - parser-probe
  flag: --event
  invocation_scope:
  - applies_to: command
    command:
    - github
    - run
  notes: 'No nargs/requiresArg declaration: omitted strings become empty strings, numbers undefined or their declared default. This describes parser acceptance; the handler may reject empty values.'
  value_optional: true
  value_type: string
- attachment:
  - space
  - equals
  description: GitHub personal access token (github_pat_********)
  evidence_ids:
  - cmd-github
  - parser
  - local-help
  - parser-probe
  flag: --token
  invocation_scope:
  - applies_to: command
    command:
    - github
    - run
  notes: 'No nargs/requiresArg declaration: omitted strings become empty strings, numbers undefined or their declared default. This describes parser acceptance; the handler may reject empty values.'
  value_optional: true
  value_type: string
- attachment:
  - space
  - equals
  default: tsv
  description: Output format
  evidence_ids:
  - cmd-db
  - parser
  - local-help
  - parser-probe
  flag: --format
  invocation_scope:
  - applies_to: command
    command:
    - db
  notes: 'No nargs/requiresArg declaration: omitted strings become empty strings, numbers undefined or their declared default. This describes parser acceptance; the handler may reject empty values.'
  value_optional: true
  value_type: string
- attachment:
  - space
  - equals
  description: Tool id to execute
  evidence_ids:
  - cmd-debug-agent
  - parser
  - local-help
  - parser-probe
  flag: --tool
  invocation_scope:
  - applies_to: command
    command:
    - debug
    - agent
  notes: 'No nargs/requiresArg declaration: omitted strings become empty strings, numbers undefined or their declared default. This describes parser acceptance; the handler may reject empty values.'
  value_optional: true
  value_type: string
- attachment:
  - space
  - equals
  description: Tool params as JSON or a JS object literal
  evidence_ids:
  - cmd-debug-agent
  - parser
  - local-help
  - parser-probe
  flag: --params
  invocation_scope:
  - applies_to: command
    command:
    - debug
    - agent
  notes: 'No nargs/requiresArg declaration: omitted strings become empty strings, numbers undefined or their declared default. This describes parser acceptance; the handler may reject empty values.'
  value_optional: true
  value_type: string
- attachment:
  - space
  - equals
  description: Filter files by query
  evidence_ids:
  - cmd-debug-ripgrep
  - parser
  - local-help
  - parser-probe
  flag: --query
  invocation_scope:
  - applies_to: command
    command:
    - debug
    - rg
    - files
  notes: 'No nargs/requiresArg declaration: omitted strings become empty strings, numbers undefined or their declared default. This describes parser acceptance; the handler may reject empty values.'
  value_optional: true
  value_type: string
- attachment:
  - space
  - equals
  description: Glob pattern to match files
  evidence_ids:
  - cmd-debug-ripgrep
  - parser
  - local-help
  - parser-probe
  flag: --glob
  invocation_scope:
  - applies_to: command
    command:
    - debug
    - rg
    - files
  notes: 'No nargs/requiresArg declaration: omitted strings become empty strings, numbers undefined or their declared default. This describes parser acceptance; the handler may reject empty values.'
  value_optional: true
  value_type: string
- attachment:
  - space
  - equals
  description: Limit number of results
  evidence_ids:
  - cmd-debug-ripgrep
  - parser
  - local-help
  - parser-probe
  flag: --limit
  invocation_scope:
  - applies_to: command
    command:
    - debug
    - rg
    - files
  notes: 'No nargs/requiresArg declaration: omitted strings become empty strings, numbers undefined or their declared default. This describes parser acceptance; the handler may reject empty values.'
  value_optional: true
  value_type: number
- attachment:
  - space
  - equals
  description: File glob patterns
  evidence_ids:
  - cmd-debug-ripgrep
  - parser
  - local-help
  - parser-probe
  flag: --glob
  gap: Parser eatArray and the direct parser probe establish a zero-value minimum. Revision 2 allows only positive minima or unknown; a schema revision supporting zero and rerunning the same empty-array probe would settle a faithfully encodable minimum.
  invocation_scope:
  - applies_to: command
    command:
    - debug
    - rg
    - search
  notes: 'No nargs/requiresArg declaration: omitted strings become empty strings, numbers undefined or their declared default. This describes parser acceptance; the handler may reject empty values. Arrays greedily consume following non-switch tokens, including after equals; an occurrence without values produces an empty array.'
  value_type: variadic
  variadic_min: unknown
- attachment:
  - space
  - equals
  description: Limit number of results
  evidence_ids:
  - cmd-debug-ripgrep
  - parser
  - local-help
  - parser-probe
  flag: --limit
  invocation_scope:
  - applies_to: command
    command:
    - debug
    - rg
    - search
  notes: 'No nargs/requiresArg declaration: omitted strings become empty strings, numbers undefined or their declared default. This describes parser acceptance; the handler may reject empty values.'
  value_optional: true
  value_type: number
- attachment: []
  description: Disable interactive history replay through yargs negation.
  evidence_ids:
  - cmd-run
  - parser
  - local-help
  - parser-probe
  flag: --no-replay
  invocation_scope:
  - applies_to: command
    command:
    - run
  value_type: none
- attachment:
  - space
  - equals
  description: Port to listen on.
  evidence_ids:
  - network
  - parser
  - local-help
  - parser-probe
  flag: --port
  invocation_scope:
  - applies_to: command
    command: []
  notes: 'No nargs/requiresArg declaration: omitted strings become empty strings, numbers undefined or their declared default. This describes parser acceptance; the handler may reject empty values.'
  value_optional: true
  value_type: number
- attachment:
  - space
  - equals
  description: Hostname to listen on.
  evidence_ids:
  - network
  - parser
  - local-help
  - parser-probe
  flag: --hostname
  invocation_scope:
  - applies_to: command
    command: []
  notes: 'No nargs/requiresArg declaration: omitted strings become empty strings, numbers undefined or their declared default. This describes parser acceptance; the handler may reject empty values.'
  value_optional: true
  value_type: string
- attachment: []
  description: Enable mDNS discovery.
  evidence_ids:
  - network
  - parser
  - local-help
  - parser-probe
  flag: --mdns
  invocation_scope:
  - applies_to: command
    command: []
  value_type: none
- attachment: []
  description: Set --mdns to false through yargs boolean negation.
  evidence_ids:
  - network
  - parser
  - parser-probe
  flag: --no-mdns
  invocation_scope:
  - applies_to: command
    command: []
  value_type: none
- aliases:
  - --mdnsDomain
  attachment:
  - space
  - equals
  description: mDNS service domain.
  evidence_ids:
  - network
  - parser
  - local-help
  - parser-probe
  flag: --mdns-domain
  invocation_scope:
  - applies_to: command
    command: []
  notes: 'No nargs/requiresArg declaration: omitted strings become empty strings, numbers undefined or their declared default. This describes parser acceptance; the handler may reject empty values.'
  value_optional: true
  value_type: string
- attachment:
  - space
  - equals
  description: Additional CORS origins.
  evidence_ids:
  - network
  - parser
  - local-help
  - parser-probe
  flag: --cors
  gap: Parser eatArray and the direct parser probe establish a zero-value minimum. Revision 2 allows only positive minima or unknown; a schema revision supporting zero and rerunning the same empty-array probe would settle a faithfully encodable minimum.
  invocation_scope:
  - applies_to: command
    command: []
  notes: 'No nargs/requiresArg declaration: omitted strings become empty strings, numbers undefined or their declared default. This describes parser acceptance; the handler may reject empty values. Arrays greedily consume following non-switch tokens, including after equals; an occurrence without values produces an empty array.'
  value_type: variadic
  variadic_min: unknown
- attachment:
  - space
  - equals
  description: Port to listen on.
  evidence_ids:
  - network
  - parser
  - local-help
  - parser-probe
  flag: --port
  invocation_scope:
  - applies_to: command
    command:
    - acp
  notes: 'No nargs/requiresArg declaration: omitted strings become empty strings, numbers undefined or their declared default. This describes parser acceptance; the handler may reject empty values.'
  value_optional: true
  value_type: number
- attachment:
  - space
  - equals
  description: Hostname to listen on.
  evidence_ids:
  - network
  - parser
  - local-help
  - parser-probe
  flag: --hostname
  invocation_scope:
  - applies_to: command
    command:
    - acp
  notes: 'No nargs/requiresArg declaration: omitted strings become empty strings, numbers undefined or their declared default. This describes parser acceptance; the handler may reject empty values.'
  value_optional: true
  value_type: string
- attachment: []
  description: Enable mDNS discovery.
  evidence_ids:
  - network
  - parser
  - local-help
  - parser-probe
  flag: --mdns
  invocation_scope:
  - applies_to: command
    command:
    - acp
  value_type: none
- attachment: []
  description: Set --mdns to false through yargs boolean negation.
  evidence_ids:
  - network
  - parser
  - parser-probe
  flag: --no-mdns
  invocation_scope:
  - applies_to: command
    command:
    - acp
  value_type: none
- aliases:
  - --mdnsDomain
  attachment:
  - space
  - equals
  description: mDNS service domain.
  evidence_ids:
  - network
  - parser
  - local-help
  - parser-probe
  flag: --mdns-domain
  invocation_scope:
  - applies_to: command
    command:
    - acp
  notes: 'No nargs/requiresArg declaration: omitted strings become empty strings, numbers undefined or their declared default. This describes parser acceptance; the handler may reject empty values.'
  value_optional: true
  value_type: string
- attachment:
  - space
  - equals
  description: Additional CORS origins.
  evidence_ids:
  - network
  - parser
  - local-help
  - parser-probe
  flag: --cors
  gap: Parser eatArray and the direct parser probe establish a zero-value minimum. Revision 2 allows only positive minima or unknown; a schema revision supporting zero and rerunning the same empty-array probe would settle a faithfully encodable minimum.
  invocation_scope:
  - applies_to: command
    command:
    - acp
  notes: 'No nargs/requiresArg declaration: omitted strings become empty strings, numbers undefined or their declared default. This describes parser acceptance; the handler may reject empty values. Arrays greedily consume following non-switch tokens, including after equals; an occurrence without values produces an empty array.'
  value_type: variadic
  variadic_min: unknown
- attachment:
  - space
  - equals
  description: Port to listen on.
  evidence_ids:
  - network
  - parser
  - local-help
  - parser-probe
  flag: --port
  invocation_scope:
  - applies_to: command
    command:
    - serve
  notes: 'No nargs/requiresArg declaration: omitted strings become empty strings, numbers undefined or their declared default. This describes parser acceptance; the handler may reject empty values.'
  value_optional: true
  value_type: number
- attachment:
  - space
  - equals
  description: Hostname to listen on.
  evidence_ids:
  - network
  - parser
  - local-help
  - parser-probe
  flag: --hostname
  invocation_scope:
  - applies_to: command
    command:
    - serve
  notes: 'No nargs/requiresArg declaration: omitted strings become empty strings, numbers undefined or their declared default. This describes parser acceptance; the handler may reject empty values.'
  value_optional: true
  value_type: string
- attachment: []
  description: Enable mDNS discovery.
  evidence_ids:
  - network
  - parser
  - local-help
  - parser-probe
  flag: --mdns
  invocation_scope:
  - applies_to: command
    command:
    - serve
  value_type: none
- attachment: []
  description: Set --mdns to false through yargs boolean negation.
  evidence_ids:
  - network
  - parser
  - parser-probe
  flag: --no-mdns
  invocation_scope:
  - applies_to: command
    command:
    - serve
  value_type: none
- aliases:
  - --mdnsDomain
  attachment:
  - space
  - equals
  description: mDNS service domain.
  evidence_ids:
  - network
  - parser
  - local-help
  - parser-probe
  flag: --mdns-domain
  invocation_scope:
  - applies_to: command
    command:
    - serve
  notes: 'No nargs/requiresArg declaration: omitted strings become empty strings, numbers undefined or their declared default. This describes parser acceptance; the handler may reject empty values.'
  value_optional: true
  value_type: string
- attachment:
  - space
  - equals
  description: Additional CORS origins.
  evidence_ids:
  - network
  - parser
  - local-help
  - parser-probe
  flag: --cors
  gap: Parser eatArray and the direct parser probe establish a zero-value minimum. Revision 2 allows only positive minima or unknown; a schema revision supporting zero and rerunning the same empty-array probe would settle a faithfully encodable minimum.
  invocation_scope:
  - applies_to: command
    command:
    - serve
  notes: 'No nargs/requiresArg declaration: omitted strings become empty strings, numbers undefined or their declared default. This describes parser acceptance; the handler may reject empty values. Arrays greedily consume following non-switch tokens, including after equals; an occurrence without values produces an empty array.'
  value_type: variadic
  variadic_min: unknown
- attachment:
  - space
  - equals
  description: Port to listen on.
  evidence_ids:
  - network
  - parser
  - local-help
  - parser-probe
  flag: --port
  invocation_scope:
  - applies_to: command
    command:
    - web
  notes: 'No nargs/requiresArg declaration: omitted strings become empty strings, numbers undefined or their declared default. This describes parser acceptance; the handler may reject empty values.'
  value_optional: true
  value_type: number
- attachment:
  - space
  - equals
  description: Hostname to listen on.
  evidence_ids:
  - network
  - parser
  - local-help
  - parser-probe
  flag: --hostname
  invocation_scope:
  - applies_to: command
    command:
    - web
  notes: 'No nargs/requiresArg declaration: omitted strings become empty strings, numbers undefined or their declared default. This describes parser acceptance; the handler may reject empty values.'
  value_optional: true
  value_type: string
- attachment: []
  description: Enable mDNS discovery.
  evidence_ids:
  - network
  - parser
  - local-help
  - parser-probe
  flag: --mdns
  invocation_scope:
  - applies_to: command
    command:
    - web
  value_type: none
- attachment: []
  description: Set --mdns to false through yargs boolean negation.
  evidence_ids:
  - network
  - parser
  - parser-probe
  flag: --no-mdns
  invocation_scope:
  - applies_to: command
    command:
    - web
  value_type: none
- aliases:
  - --mdnsDomain
  attachment:
  - space
  - equals
  description: mDNS service domain.
  evidence_ids:
  - network
  - parser
  - local-help
  - parser-probe
  flag: --mdns-domain
  invocation_scope:
  - applies_to: command
    command:
    - web
  notes: 'No nargs/requiresArg declaration: omitted strings become empty strings, numbers undefined or their declared default. This describes parser acceptance; the handler may reject empty values.'
  value_optional: true
  value_type: string
- attachment:
  - space
  - equals
  description: Additional CORS origins.
  evidence_ids:
  - network
  - parser
  - local-help
  - parser-probe
  flag: --cors
  gap: Parser eatArray and the direct parser probe establish a zero-value minimum. Revision 2 allows only positive minima or unknown; a schema revision supporting zero and rerunning the same empty-array probe would settle a faithfully encodable minimum.
  invocation_scope:
  - applies_to: command
    command:
    - web
  notes: 'No nargs/requiresArg declaration: omitted strings become empty strings, numbers undefined or their declared default. This describes parser acceptance; the handler may reject empty values. Arrays greedily consume following non-switch tokens, including after equals; an occurrence without values produces an empty array.'
  value_type: variadic
  variadic_min: unknown
- attachment: []
  description: Internal yargs shell-completion request; generated completion scripts use this switch.
  evidence_ids:
  - entry
  - completion-parser
  flag: --get-yargs-completions
  gap: No ordinary typed option declaration exists for this completion protocol switch. A disposable full-yargs completion probe with trailing tokens and equals forms would establish consumption; do not treat it as a zero-value boolean.
  invocation_scope:
  - applies_to: global
  value_type: unknown
config_paths:
- format: json
  notes: XDG_CONFIG_HOME overrides ~/.config on all platforms. Legacy config.json merges before opencode.json, then opencode.jsonc; legacy config is migrated. TUI files have a separate loader and migration.
  os: macos
  path: ~/.config/opencode/config.json
  scope: user
- format: json
  notes: XDG_CONFIG_HOME overrides ~/.config on all platforms. Legacy config.json merges before opencode.json, then opencode.jsonc; legacy config is migrated. TUI files have a separate loader and migration.
  os: macos
  path: ~/.config/opencode/opencode.json
  scope: user
- format: jsonc
  notes: XDG_CONFIG_HOME overrides ~/.config on all platforms. Legacy config.json merges before opencode.json, then opencode.jsonc; legacy config is migrated. TUI files have a separate loader and migration.
  os: macos
  path: ~/.config/opencode/opencode.jsonc
  scope: user
- format: json
  notes: XDG_CONFIG_HOME overrides ~/.config on all platforms. Legacy config.json merges before opencode.json, then opencode.jsonc; legacy config is migrated. TUI files have a separate loader and migration.
  os: macos
  path: ~/.config/opencode/tui.json
  scope: user
- format: jsonc
  notes: XDG_CONFIG_HOME overrides ~/.config on all platforms. Legacy config.json merges before opencode.json, then opencode.jsonc; legacy config is migrated. TUI files have a separate loader and migration.
  os: macos
  path: ~/.config/opencode/tui.jsonc
  scope: user
- format: toml
  notes: XDG_CONFIG_HOME overrides ~/.config on all platforms. Legacy config.json merges before opencode.json, then opencode.jsonc; legacy config is migrated. TUI files have a separate loader and migration.
  os: macos
  path: ~/.config/opencode/config
  scope: user
- format: json
  notes: Discovered upward from working directory; project config can be disabled. .opencode also contains agents, commands, skills, plugins and tools.
  os: macos
  path: opencode.json
  scope: repo
- format: jsonc
  notes: Discovered upward from working directory; project config can be disabled. .opencode also contains agents, commands, skills, plugins and tools.
  os: macos
  path: opencode.jsonc
  scope: repo
- format: json
  notes: Discovered upward from working directory; project config can be disabled. .opencode also contains agents, commands, skills, plugins and tools.
  os: macos
  path: tui.json
  scope: repo
- format: jsonc
  notes: Discovered upward from working directory; project config can be disabled. .opencode also contains agents, commands, skills, plugins and tools.
  os: macos
  path: tui.jsonc
  scope: repo
- format: json
  notes: Discovered upward from working directory; project config can be disabled. .opencode also contains agents, commands, skills, plugins and tools.
  os: macos
  path: .opencode/opencode.json
  scope: repo
- format: jsonc
  notes: Discovered upward from working directory; project config can be disabled. .opencode also contains agents, commands, skills, plugins and tools.
  os: macos
  path: .opencode/opencode.jsonc
  scope: repo
- format: json
  notes: Discovered upward from working directory; project config can be disabled. .opencode also contains agents, commands, skills, plugins and tools.
  os: macos
  path: .opencode/tui.json
  scope: repo
- format: jsonc
  notes: Discovered upward from working directory; project config can be disabled. .opencode also contains agents, commands, skills, plugins and tools.
  os: macos
  path: .opencode/tui.jsonc
  scope: repo
- format: jsonc
  notes: Explicit file path; accepts JSON/JSONC. Main configuration and TUI configuration merge independently.
  os: macos
  path: $OPENCODE_CONFIG
  scope: env
- format: jsonc
  notes: Explicit file path; accepts JSON/JSONC. Main configuration and TUI configuration merge independently.
  os: macos
  path: $OPENCODE_TUI_CONFIG
  scope: env
- format: json
  notes: Additional discovery directory; its plugin/resource directories can also load.
  os: macos
  path: $OPENCODE_CONFIG_DIR/opencode.json
  scope: env
- format: jsonc
  notes: Additional discovery directory; its plugin/resource directories can also load.
  os: macos
  path: $OPENCODE_CONFIG_DIR/opencode.jsonc
  scope: env
- format: json
  notes: Additional discovery directory; its plugin/resource directories can also load.
  os: macos
  path: $OPENCODE_CONFIG_DIR/tui.json
  scope: env
- format: jsonc
  notes: Additional discovery directory; its plugin/resource directories can also load.
  os: macos
  path: $OPENCODE_CONFIG_DIR/tui.jsonc
  scope: env
- format: json
  notes: Managed settings merge after inline configuration. Windows fallback is C:\ProgramData.
  os: macos
  path: /Library/Application Support/opencode/opencode.json
  scope: system
- format: jsonc
  notes: Managed settings merge after inline configuration. Windows fallback is C:\ProgramData.
  os: macos
  path: /Library/Application Support/opencode/opencode.jsonc
  scope: system
- format: other
  notes: MDM plist converted through plutil; highest configuration precedence.
  os: macos
  path: /Library/Managed Preferences/ai.opencode.managed.plist
  scope: system
- format: other
  notes: MDM plist converted through plutil; highest configuration precedence.
  os: macos
  path: /Library/Managed Preferences/<username>/ai.opencode.managed.plist
  scope: system
- format: json
  notes: Home .opencode resource directory is discovered too; opencode.jsonc is its JSONC counterpart, and tui.json/tui.jsonc use separate TUI discovery.
  os: macos
  path: ~/.opencode/opencode.json
  scope: user
- format: json
  notes: Provider credential store; XDG_DATA_HOME relocates it. Login/logout write credentials.
  os: macos
  path: ~/.local/share/opencode/auth.json
  scope: user
- format: json
  notes: XDG_CONFIG_HOME overrides ~/.config on all platforms. Legacy config.json merges before opencode.json, then opencode.jsonc; legacy config is migrated. TUI files have a separate loader and migration.
  os: linux
  path: ~/.config/opencode/config.json
  scope: user
- format: json
  notes: XDG_CONFIG_HOME overrides ~/.config on all platforms. Legacy config.json merges before opencode.json, then opencode.jsonc; legacy config is migrated. TUI files have a separate loader and migration.
  os: linux
  path: ~/.config/opencode/opencode.json
  scope: user
- format: jsonc
  notes: XDG_CONFIG_HOME overrides ~/.config on all platforms. Legacy config.json merges before opencode.json, then opencode.jsonc; legacy config is migrated. TUI files have a separate loader and migration.
  os: linux
  path: ~/.config/opencode/opencode.jsonc
  scope: user
- format: json
  notes: XDG_CONFIG_HOME overrides ~/.config on all platforms. Legacy config.json merges before opencode.json, then opencode.jsonc; legacy config is migrated. TUI files have a separate loader and migration.
  os: linux
  path: ~/.config/opencode/tui.json
  scope: user
- format: jsonc
  notes: XDG_CONFIG_HOME overrides ~/.config on all platforms. Legacy config.json merges before opencode.json, then opencode.jsonc; legacy config is migrated. TUI files have a separate loader and migration.
  os: linux
  path: ~/.config/opencode/tui.jsonc
  scope: user
- format: toml
  notes: XDG_CONFIG_HOME overrides ~/.config on all platforms. Legacy config.json merges before opencode.json, then opencode.jsonc; legacy config is migrated. TUI files have a separate loader and migration.
  os: linux
  path: ~/.config/opencode/config
  scope: user
- format: json
  notes: Discovered upward from working directory; project config can be disabled. .opencode also contains agents, commands, skills, plugins and tools.
  os: linux
  path: opencode.json
  scope: repo
- format: jsonc
  notes: Discovered upward from working directory; project config can be disabled. .opencode also contains agents, commands, skills, plugins and tools.
  os: linux
  path: opencode.jsonc
  scope: repo
- format: json
  notes: Discovered upward from working directory; project config can be disabled. .opencode also contains agents, commands, skills, plugins and tools.
  os: linux
  path: tui.json
  scope: repo
- format: jsonc
  notes: Discovered upward from working directory; project config can be disabled. .opencode also contains agents, commands, skills, plugins and tools.
  os: linux
  path: tui.jsonc
  scope: repo
- format: json
  notes: Discovered upward from working directory; project config can be disabled. .opencode also contains agents, commands, skills, plugins and tools.
  os: linux
  path: .opencode/opencode.json
  scope: repo
- format: jsonc
  notes: Discovered upward from working directory; project config can be disabled. .opencode also contains agents, commands, skills, plugins and tools.
  os: linux
  path: .opencode/opencode.jsonc
  scope: repo
- format: json
  notes: Discovered upward from working directory; project config can be disabled. .opencode also contains agents, commands, skills, plugins and tools.
  os: linux
  path: .opencode/tui.json
  scope: repo
- format: jsonc
  notes: Discovered upward from working directory; project config can be disabled. .opencode also contains agents, commands, skills, plugins and tools.
  os: linux
  path: .opencode/tui.jsonc
  scope: repo
- format: jsonc
  notes: Explicit file path; accepts JSON/JSONC. Main configuration and TUI configuration merge independently.
  os: linux
  path: $OPENCODE_CONFIG
  scope: env
- format: jsonc
  notes: Explicit file path; accepts JSON/JSONC. Main configuration and TUI configuration merge independently.
  os: linux
  path: $OPENCODE_TUI_CONFIG
  scope: env
- format: json
  notes: Additional discovery directory; its plugin/resource directories can also load.
  os: linux
  path: $OPENCODE_CONFIG_DIR/opencode.json
  scope: env
- format: jsonc
  notes: Additional discovery directory; its plugin/resource directories can also load.
  os: linux
  path: $OPENCODE_CONFIG_DIR/opencode.jsonc
  scope: env
- format: json
  notes: Additional discovery directory; its plugin/resource directories can also load.
  os: linux
  path: $OPENCODE_CONFIG_DIR/tui.json
  scope: env
- format: jsonc
  notes: Additional discovery directory; its plugin/resource directories can also load.
  os: linux
  path: $OPENCODE_CONFIG_DIR/tui.jsonc
  scope: env
- format: json
  notes: Managed settings merge after inline configuration. Windows fallback is C:\ProgramData.
  os: linux
  path: /etc/opencode/opencode.json
  scope: system
- format: jsonc
  notes: Managed settings merge after inline configuration. Windows fallback is C:\ProgramData.
  os: linux
  path: /etc/opencode/opencode.jsonc
  scope: system
- format: json
  notes: Home .opencode resource directory is discovered too; opencode.jsonc is its JSONC counterpart, and tui.json/tui.jsonc use separate TUI discovery.
  os: linux
  path: ~/.opencode/opencode.json
  scope: user
- format: json
  notes: Provider credential store; XDG_DATA_HOME relocates it. Login/logout write credentials.
  os: linux
  path: ~/.local/share/opencode/auth.json
  scope: user
- format: json
  notes: XDG_CONFIG_HOME overrides ~/.config on all platforms. Legacy config.json merges before opencode.json, then opencode.jsonc; legacy config is migrated. TUI files have a separate loader and migration.
  os: windows
  path: ~/.config/opencode/config.json
  scope: user
- format: json
  notes: XDG_CONFIG_HOME overrides ~/.config on all platforms. Legacy config.json merges before opencode.json, then opencode.jsonc; legacy config is migrated. TUI files have a separate loader and migration.
  os: windows
  path: ~/.config/opencode/opencode.json
  scope: user
- format: jsonc
  notes: XDG_CONFIG_HOME overrides ~/.config on all platforms. Legacy config.json merges before opencode.json, then opencode.jsonc; legacy config is migrated. TUI files have a separate loader and migration.
  os: windows
  path: ~/.config/opencode/opencode.jsonc
  scope: user
- format: json
  notes: XDG_CONFIG_HOME overrides ~/.config on all platforms. Legacy config.json merges before opencode.json, then opencode.jsonc; legacy config is migrated. TUI files have a separate loader and migration.
  os: windows
  path: ~/.config/opencode/tui.json
  scope: user
- format: jsonc
  notes: XDG_CONFIG_HOME overrides ~/.config on all platforms. Legacy config.json merges before opencode.json, then opencode.jsonc; legacy config is migrated. TUI files have a separate loader and migration.
  os: windows
  path: ~/.config/opencode/tui.jsonc
  scope: user
- format: toml
  notes: XDG_CONFIG_HOME overrides ~/.config on all platforms. Legacy config.json merges before opencode.json, then opencode.jsonc; legacy config is migrated. TUI files have a separate loader and migration.
  os: windows
  path: ~/.config/opencode/config
  scope: user
- format: json
  notes: Discovered upward from working directory; project config can be disabled. .opencode also contains agents, commands, skills, plugins and tools.
  os: windows
  path: opencode.json
  scope: repo
- format: jsonc
  notes: Discovered upward from working directory; project config can be disabled. .opencode also contains agents, commands, skills, plugins and tools.
  os: windows
  path: opencode.jsonc
  scope: repo
- format: json
  notes: Discovered upward from working directory; project config can be disabled. .opencode also contains agents, commands, skills, plugins and tools.
  os: windows
  path: tui.json
  scope: repo
- format: jsonc
  notes: Discovered upward from working directory; project config can be disabled. .opencode also contains agents, commands, skills, plugins and tools.
  os: windows
  path: tui.jsonc
  scope: repo
- format: json
  notes: Discovered upward from working directory; project config can be disabled. .opencode also contains agents, commands, skills, plugins and tools.
  os: windows
  path: .opencode/opencode.json
  scope: repo
- format: jsonc
  notes: Discovered upward from working directory; project config can be disabled. .opencode also contains agents, commands, skills, plugins and tools.
  os: windows
  path: .opencode/opencode.jsonc
  scope: repo
- format: json
  notes: Discovered upward from working directory; project config can be disabled. .opencode also contains agents, commands, skills, plugins and tools.
  os: windows
  path: .opencode/tui.json
  scope: repo
- format: jsonc
  notes: Discovered upward from working directory; project config can be disabled. .opencode also contains agents, commands, skills, plugins and tools.
  os: windows
  path: .opencode/tui.jsonc
  scope: repo
- format: jsonc
  notes: Explicit file path; accepts JSON/JSONC. Main configuration and TUI configuration merge independently.
  os: windows
  path: $OPENCODE_CONFIG
  scope: env
- format: jsonc
  notes: Explicit file path; accepts JSON/JSONC. Main configuration and TUI configuration merge independently.
  os: windows
  path: $OPENCODE_TUI_CONFIG
  scope: env
- format: json
  notes: Additional discovery directory; its plugin/resource directories can also load.
  os: windows
  path: $OPENCODE_CONFIG_DIR/opencode.json
  scope: env
- format: jsonc
  notes: Additional discovery directory; its plugin/resource directories can also load.
  os: windows
  path: $OPENCODE_CONFIG_DIR/opencode.jsonc
  scope: env
- format: json
  notes: Additional discovery directory; its plugin/resource directories can also load.
  os: windows
  path: $OPENCODE_CONFIG_DIR/tui.json
  scope: env
- format: jsonc
  notes: Additional discovery directory; its plugin/resource directories can also load.
  os: windows
  path: $OPENCODE_CONFIG_DIR/tui.jsonc
  scope: env
- format: json
  notes: Managed settings merge after inline configuration. Windows fallback is C:\ProgramData.
  os: windows
  path: '%ProgramData%\opencode\opencode.json'
  scope: system
- format: jsonc
  notes: Managed settings merge after inline configuration. Windows fallback is C:\ProgramData.
  os: windows
  path: '%ProgramData%\opencode\opencode.jsonc'
  scope: system
- format: json
  notes: Home .opencode resource directory is discovered too; opencode.jsonc is its JSONC counterpart, and tui.json/tui.jsonc use separate TUI discovery.
  os: windows
  path: ~/.opencode/opencode.json
  scope: user
- format: json
  notes: Provider credential store; XDG_DATA_HOME relocates it. Login/logout write credentials.
  os: windows
  path: ~/.local/share/opencode/auth.json
  scope: user
env_vars:
- effect: Load an explicit main configuration file.
  name: OPENCODE_CONFIG
- effect: Load an explicit TUI configuration file.
  name: OPENCODE_TUI_CONFIG
- effect: Add a configuration/resource discovery directory.
  name: OPENCODE_CONFIG_DIR
- effect: Merge inline JSON configuration after ordinary discovery and before managed settings.
  name: OPENCODE_CONFIG_CONTENT
- effect: Disable project configuration and project .opencode discovery.
  name: OPENCODE_DISABLE_PROJECT_CONFIG
- effect: Disable external plugins; --pure sets this to 1.
  name: OPENCODE_PURE
- effect: Automatically share sessions.
  name: OPENCODE_AUTO_SHARE
- effect: Disable session sharing (literal true or 1).
  name: OPENCODE_DISABLE_SHARE
- effect: Select the Git Bash executable on Windows.
  name: OPENCODE_GIT_BASH_PATH
- effect: Disable automatic update checks.
  name: OPENCODE_DISABLE_AUTOUPDATE
- effect: Always notify about available updates.
  name: OPENCODE_ALWAYS_NOTIFY_UPDATE
- effect: Disable old-data pruning.
  name: OPENCODE_DISABLE_PRUNE
- effect: Disable terminal-title updates.
  name: OPENCODE_DISABLE_TERMINAL_TITLE
- effect: Disable automatic context compaction.
  name: OPENCODE_DISABLE_AUTOCOMPACT
- effect: Disable TUI mouse capture.
  name: OPENCODE_DISABLE_MOUSE
- effect: Disable default plugins.
  name: OPENCODE_DISABLE_DEFAULT_PLUGINS
- effect: Disable external skill discovery.
  name: OPENCODE_DISABLE_EXTERNAL_SKILLS
- effect: Disable Claude prompt and skill discovery.
  name: OPENCODE_DISABLE_CLAUDE_CODE
- effect: Disable Claude prompt discovery; detailed prompt semantics are owned by system-prompt.
  name: OPENCODE_DISABLE_CLAUDE_CODE_PROMPT
- effect: Disable Claude skill discovery.
  name: OPENCODE_DISABLE_CLAUDE_CODE_SKILLS
- effect: Disable automatic language-server downloads.
  name: OPENCODE_DISABLE_LSP_DOWNLOAD
- effect: Disable embedded web UI.
  name: OPENCODE_DISABLE_EMBEDDED_WEB_UI
- effect: Enable Exa search tools.
  name: OPENCODE_ENABLE_EXA
- effect: Enable Parallel search tools.
  name: OPENCODE_ENABLE_PARALLEL
- effect: Enable experimental model offerings.
  name: OPENCODE_ENABLE_EXPERIMENTAL_MODELS
- effect: Disable remote model-catalog fetching.
  name: OPENCODE_DISABLE_MODELS_FETCH
- effect: Override model-catalog URL, not a model inference endpoint.
  name: OPENCODE_MODELS_URL
- effect: Select a local model-catalog file.
  name: OPENCODE_MODELS_PATH
- effect: Client identifier, default cli.
  name: OPENCODE_CLIENT
- effect: Enable server HTTP basic authentication.
  name: OPENCODE_SERVER_PASSWORD
- effect: HTTP basic-auth username, default opencode.
  name: OPENCODE_SERVER_USERNAME
- effect: 'Database path: absolute and :memory: used directly; relative paths resolve under the data directory.'
  name: OPENCODE_DB
- effect: Use the unqualified database instead of a channel-specific database (literal 1 or true).
  name: OPENCODE_DISABLE_CHANNEL_DB
- effect: Disable native fast file finder; defaults to disabled on Windows.
  name: OPENCODE_DISABLE_FFF
- effect: Override detected VCS for testing.
  name: OPENCODE_FAKE_VCS
- effect: Select workspace context.
  name: OPENCODE_WORKSPACE_ID
- effect: Plugin metadata file path.
  name: OPENCODE_PLUGIN_META_FILE
- effect: CLI writes 1 as a child-process marker.
  name: OPENCODE
- effect: CLI writes its process ID for child processes.
  name: OPENCODE_PID
- effect: CLI writes 1 as a child-process agent marker.
  name: AGENT
- effect: Relocate default configuration directory (append opencode).
  name: XDG_CONFIG_HOME
- effect: Relocate persistent data and auth (append opencode).
  name: XDG_DATA_HOME
- effect: Relocate caches (append opencode).
  name: XDG_CACHE_HOME
- effect: Relocate state (append opencode).
  name: XDG_STATE_HOME
- effect: Enable the experimental umbrella; individual switches may opt out or combine with it.
  name: OPENCODE_EXPERIMENTAL
- effect: Set a positive integer default bash-tool timeout in milliseconds.
  name: OPENCODE_EXPERIMENTAL_BASH_DEFAULT_TIMEOUT_MS
- effect: Set a positive integer maximum model output token count.
  name: OPENCODE_EXPERIMENTAL_OUTPUT_TOKEN_MAX
- effect: Enable whole-directory file watching.
  name: OPENCODE_EXPERIMENTAL_FILEWATCHER
- effect: Disable file watching.
  name: OPENCODE_EXPERIMENTAL_DISABLE_FILEWATCHER
- effect: Disable copy on selection; default true on Windows.
  name: OPENCODE_EXPERIMENTAL_DISABLE_COPY_ON_SELECT
- effect: Legacy Exa search toggle; combines with umbrella and OPENCODE_ENABLE_EXA.
  name: OPENCODE_EXPERIMENTAL_EXA
- effect: Legacy Parallel search toggle; combines with OPENCODE_ENABLE_PARALLEL.
  name: OPENCODE_EXPERIMENTAL_PARALLEL
- effect: Enable native LLM request path.
  name: OPENCODE_EXPERIMENTAL_NATIVE_LLM
- effect: Enable experimental WebSocket transport.
  name: OPENCODE_EXPERIMENTAL_WEBSOCKETS
- effect: Enable references; defaults to the experimental umbrella when unset.
  name: OPENCODE_EXPERIMENTAL_REFERENCES
- effect: Enable background subagent tasks; defaults to the experimental umbrella when unset.
  name: OPENCODE_EXPERIMENTAL_BACKGROUND_SUBAGENTS
- effect: Enable LSP tools; defaults to the experimental umbrella when unset.
  name: OPENCODE_EXPERIMENTAL_LSP_TOOL
- effect: Enable oxfmt formatting; defaults to the experimental umbrella when unset.
  name: OPENCODE_EXPERIMENTAL_OXFMT
- effect: Enable plan mode; defaults to the experimental umbrella when unset.
  name: OPENCODE_EXPERIMENTAL_PLAN_MODE
- effect: Enable code mode; defaults to the experimental umbrella when unset.
  name: OPENCODE_EXPERIMENTAL_CODE_MODE
- effect: Enable event system; defaults to the experimental umbrella when unset.
  name: OPENCODE_EXPERIMENTAL_EVENT_SYSTEM
- effect: Enable workspaces; defaults to the experimental umbrella when unset.
  name: OPENCODE_EXPERIMENTAL_WORKSPACES
- effect: Enable icon discovery; defaults to the experimental umbrella when unset.
  name: OPENCODE_EXPERIMENTAL_ICON_DISCOVERY
- effect: Enable the TY Python language server.
  name: OPENCODE_EXPERIMENTAL_LSP_TY
machine_introspection:
- command: opencode debug config
  machine_readable: true
  notes: Resolved merged config; can expose secrets and load plugin dependencies. Isolated probe succeeded.
  output_format: json
  purpose: config_dump
  useful_for_codegen: true
- command: opencode debug skill
  machine_readable: true
  notes: Available skills including built-in entries; isolated probe succeeded.
  output_format: json
  purpose: other
  useful_for_codegen: false
- command: opencode debug scrap
  machine_readable: true
  notes: Known projects; established from JSON.stringify in source.
  output_format: json
  purpose: other
  useful_for_codegen: false
- command: opencode debug v2
  machine_readable: true
  notes: Experimental catalog/provider/default/small-model diagnostic; established from source, not executed.
  output_format: json
  purpose: models
  useful_for_codegen: true
- command: opencode debug agent <name>
  machine_readable: true
  notes: Agent configuration; --tool/--params execute a tool. Source, not executed.
  output_format: json
  purpose: other
  useful_for_codegen: false
- command: opencode generate
  machine_readable: true
  notes: OpenAPI 3.1.0 document confirmed locally; this is the HTTP API schema, not the configuration schema.
  output_format: json
  purpose: config_schema
  useful_for_codegen: true
- command: opencode session list --format json
  machine_readable: true
  notes: Nonempty session array; no sessions yields empty stdout, confirmed locally.
  output_format: json
  purpose: other
  useful_for_codegen: false
- command: opencode export <sessionID> --sanitize
  machine_readable: true
  notes: Sanitized session JSON; supplying session ID avoids selection. Source, not executed.
  output_format: json
  purpose: other
  useful_for_codegen: false
- command: opencode db "select 1 as one" --format json
  machine_readable: true
  notes: SQL result array [{"one":1}] confirmed locally; other SQL can mutate the database.
  output_format: json
  purpose: other
  useful_for_codegen: false
- command: opencode debug paths
  machine_readable: false
  notes: Effective paths, confirmed with isolated XDG environment.
  output_format: text
  purpose: env
  useful_for_codegen: false
- command: opencode debug info
  machine_readable: false
  notes: Version/OS/terminal/plugin list; text only.
  output_format: text
  purpose: doctor
  useful_for_codegen: false
- command: opencode models
  machine_readable: false
  notes: One provider/model ID per line; not a JSON output contract.
  output_format: text
  purpose: models
  useful_for_codegen: true
- command: opencode models --verbose
  machine_readable: false
  notes: Model IDs mixed with pretty JSON metadata; not a single JSON document.
  output_format: text
  purpose: models
  useful_for_codegen: true
- command: opencode providers list
  machine_readable: false
  notes: Alias auth list (and ls); styled credential-provider listing, no JSON mode.
  output_format: text
  purpose: other
  useful_for_codegen: false
- command: opencode db path
  machine_readable: false
  notes: Print database path; no JSON mode.
  output_format: text
  purpose: env
  useful_for_codegen: false
wrapper_notes:
- Installed version is 1.18.33; latest GitHub release is 1.18.34. Switch observations apply to 1.18.33, not unexamined latest-version behavior.
- Use run for finite prompt execution, with stdin closed or intentional piped content. Root starts a TUI. run reads piped stdin and combines it with argv message text.
- --file/-f and --cors are greedy arrays, including equals forms. Place prompt positionals before these switches or use -- to terminate switch parsing; run includes its populated -- arguments in the message.
- Array switches accept zero values, but revision 2 cannot encode minimum zero; variadic_min is unknown with an explicit schema gap. Do not silently substitute 1.
- 'Short-attached strings have restricted syntax: -s123 and -m/provider are consumed, but -sABC and -mfoo group letters as switches. Prefer space or equals.'
- Boolean switches also accept =true/=false, sometimes consume a following literal true/false, and have --no-* forms. The none catalog type cannot describe the optional literal-token consumption; preserve those tokens explicitly in wrapper parsing.
- No native run system-prompt flag was found in installed help, run.ts or CLI documentation. --prompt belongs to the root TUI initial user prompt.
- String and numeric options usually allow missing values syntactically; handler validation can still reject or ignore them. Stats --models is intentionally unknown because its declaration is untyped.
- Resume uses run --continue/-c or --session/-s; --fork forks before continuation. There is no separate resume command.
- run --format json emits NDJSON events; capture stderr separately, since human statuses, errors and share URLs can appear there even on successful runs.
- serve/acp are headless but long-running, so non_interactive is false under this contract’s runs-to-completion definition. web additionally opens a browser.
- Configuration/diagnostic startup can create directories, seed config, install plugin dependencies, and migrate TUI config. Use private HOME plus XDG roots, disable project discovery, supply explicit config, and disable external skill discovery for disposable probes; HOME alone does not suppress built-in skills.
- --pure disables external plugins; it does not constitute a complete filesystem/model isolation policy.
- providers/auth and plugin/plug are aliases. console and generate are hidden registered commands, despite absence from root help.
- session list --format json can return empty stdout with success for an empty database. Treat that as an empty result.
- agent create with all required settings is finite but calls an LLM and writes files. export without sessionID, db without SQL, uninstall without --force and management login flows can prompt.
changes:
- Refreshed installed version from 1.17.13 to 1.18.33; latest released version is 1.18.34.
- Migrated research to schema revision 2 with source-backed types, exact aliases, attachment forms and command scopes.
- Expanded command inventory to nested and alias paths; console/generate remain accepted hidden commands.
- Corrected headless service classification, empty session JSON behavior, greedy array minimum zero and restricted short attachment.
- Replaced old local config snapshot and environment inventory with current read-only inspection and versioned source.
requires_claudine_update: true
reason: Typed parser records replace the old untyped inventory. Claudine must handle greedy arrays, restricted short attachment, boolean literal consumption and explicit unknown/schema gaps without taking prompt arguments.
contract_checked: 2026-10-01
---

# OpenCode CLI Surface


## Overview


OpenCode is the open-source coding agent shipped by Anomaly. `opencode` starts a terminal UI; `opencode run "Explain this function"` is the finite automation entrypoint. On 2026-10-01, `opencode --version` returned **1.18.33**. `sniff software agents --json` located it at `/Users/ken/.opencode/bin/opencode`. The [latest GitHub release](https://github.com/anomalyco/opencode/releases/latest) was **v1.18.34**; its switch surface was not examined. Findings below use installed 1.18.33 help and [version-pinned source](https://github.com/anomalyco/opencode/blob/v1.18.33/packages/opencode/src/index.ts).

Main links: [homepage](https://opencode.ai), [repository](https://github.com/anomalyco/opencode), [documentation](https://opencode.ai/docs/), and [CLI reference](https://opencode.ai/docs/cli/).


## Installation and Binaries


The command is `opencode` on macOS, Linux and Windows. Native Windows packages supply `opencode.exe`; npm may supply `opencode.cmd` and `opencode.ps1` shims. [Installation documentation](https://github.com/anomalyco/opencode/blob/v1.18.33/packages/web/src/content/docs/index.mdx) recommends WSL for Windows and says Windows Bun installation is still in progress. No installer was executed.

| OS | Method | Command |
| --- | --- | --- |
| macos | npm | `npm install -g opencode-ai` |
| macos | other | `pnpm install -g opencode-ai` |
| macos | other | `yarn global add opencode-ai` |
| macos | standalone_binary | `curl -fsSL https://opencode.ai/install | bash` |
| macos | other | `bun install -g opencode-ai` |
| macos | brew | `brew install anomalyco/tap/opencode` |
| linux | npm | `npm install -g opencode-ai` |
| linux | other | `pnpm install -g opencode-ai` |
| linux | other | `yarn global add opencode-ai` |
| linux | standalone_binary | `curl -fsSL https://opencode.ai/install | bash` |
| linux | other | `bun install -g opencode-ai` |
| linux | brew | `brew install anomalyco/tap/opencode` |
| linux | package_manager | `sudo pacman -S opencode` |
| linux | package_manager | `paru -S opencode-bin` |
| linux | other | `docker run -it --rm ghcr.io/anomalyco/opencode` |
| windows | npm | `npm install -g opencode-ai` |
| windows | other | `pnpm install -g opencode-ai` |
| windows | other | `yarn global add opencode-ai` |
| windows | chocolatey | `choco install opencode` |
| windows | scoop | `scoop install opencode` |
| windows | other | `mise use -g github:anomalyco/opencode` |


## Subcommands


Every path below was inspected with `opencode <path> --help`. The root `$0 [project]` is an entrypoint, not a `tui` subcommand. Hidden `console` and `generate` were inspected explicitly. “Yes” means a finite invocation exists under the condition in the notes; it does not promise that the command is read-only or free. Group commands, services, and browser/prompt flows are “No.”

| Path | Finite without terminal/browser/input? | Purpose and conditions |
| --- | --- | --- |
| `acp` | No | start ACP (Agent Client Protocol) server Long-running service; does not run to completion. |
| `agent` | No | Groups agent creation and listing. Command group; select a leaf command. |
| `agent create` | Yes | create a new agent Only non-interactive with nonempty --path, --description, --mode and supplied --permissions (alias --tools). Generates an agent through an LLM and writes <path>/agents/<identifier>.md; not executed in this research. |
| `agent list` | Yes | list all available agents Finite command without user input according to its registered handler; help inspected, handler not executed unless noted below. |
| `attach` | No | attach to a running opencode server Requires a terminal/browser, or can prompt during installation. |
| `auth` | No | Groups auth operations. Command group; select a leaf command. |
| `auth list` | Yes | list providers and credentials Finite command without user input according to its registered handler; help inspected, handler not executed unless noted below. |
| `auth login` | No | log in to a provider Can request user selection, credentials or browser authorization; only help inspected. |
| `auth logout` | No | log out from a configured provider Can request user selection, credentials or browser authorization; only help inspected. |
| `auth ls` | Yes | list providers and credentials Finite command without user input according to its registered handler; help inspected, handler not executed unless noted below. |
| `completion` | Yes | Generate a shell completion script. Finite command without user input according to its registered handler; help inspected, handler not executed unless noted below. |
| `console` | No | Groups console account operations. Hidden registered command: absent from root help, present in console --help and index.ts. |
| `console login` | No | log in to console Can request user selection, credentials or browser authorization; only help inspected. |
| `console logout` | No | log out from console Can request user selection, credentials or browser authorization; only help inspected. |
| `console open` | No | open active console account Can request user selection, credentials or browser authorization; only help inspected. |
| `console orgs` | Yes | list orgs Lists account organizations without selecting one; can perform account/network reads. |
| `console switch` | No | switch active org Can request user selection, credentials or browser authorization; only help inspected. |
| `db` | Yes | database tools Finite with a SQL query positional; bare db opens interactive sqlite3. Read-only SQL is a safe probe; arbitrary SQL can mutate state. |
| `db path` | Yes | print the database path Finite command without user input according to its registered handler; help inspected, handler not executed unless noted below. |
| `debug` | No | Groups troubleshooting tools. Command group; select a leaf command. |
| `debug agent` | Yes | show agent configuration details Shows agent configuration; --tool executes a tool with --params, so this diagnostic can have side effects. |
| `debug config` | Yes | show resolved configuration Finite command without user input according to its registered handler; help inspected, handler not executed unless noted below. |
| `debug file` | No | Groups debug file operations. Command group; select a leaf command. |
| `debug file list` | Yes | list files in a directory Finite command without user input according to its registered handler; help inspected, handler not executed unless noted below. |
| `debug file read` | Yes | read file contents as JSON Finite command without user input according to its registered handler; help inspected, handler not executed unless noted below. |
| `debug file search` | Yes | search files by query Finite command without user input according to its registered handler; help inspected, handler not executed unless noted below. |
| `debug info` | Yes | show debug information Finite command without user input according to its registered handler; help inspected, handler not executed unless noted below. |
| `debug lsp` | No | Groups debug lsp operations. Command group; select a leaf command. |
| `debug lsp diagnostics` | Yes | get diagnostics for a file Finite command without user input according to its registered handler; help inspected, handler not executed unless noted below. |
| `debug lsp document-symbols` | Yes | get symbols from a document Finite command without user input according to its registered handler; help inspected, handler not executed unless noted below. |
| `debug lsp symbols` | Yes | search workspace symbols Finite command without user input according to its registered handler; help inspected, handler not executed unless noted below. |
| `debug paths` | Yes | show global paths (data, config, cache, state) Finite command without user input according to its registered handler; help inspected, handler not executed unless noted below. |
| `debug rg` | No | Groups debug rg operations. Command group; select a leaf command. |
| `debug rg files` | Yes | list files using ripgrep Finite command without user input according to its registered handler; help inspected, handler not executed unless noted below. |
| `debug rg search` | Yes | search file contents using ripgrep Finite command without user input according to its registered handler; help inspected, handler not executed unless noted below. |
| `debug scrap` | Yes | list all known projects Finite command without user input according to its registered handler; help inspected, handler not executed unless noted below. |
| `debug skill` | Yes | list all available skills Finite command without user input according to its registered handler; help inspected, handler not executed unless noted below. |
| `debug snapshot` | No | Groups debug snapshot operations. Command group; select a leaf command. |
| `debug snapshot diff` | Yes | show diff for a snapshot hash Finite command without user input according to its registered handler; help inspected, handler not executed unless noted below. |
| `debug snapshot patch` | Yes | show patch for a snapshot hash Finite command without user input according to its registered handler; help inspected, handler not executed unless noted below. |
| `debug snapshot track` | Yes | track current snapshot state Creates/tracks a repository snapshot; may update snapshot storage. |
| `debug startup` | Yes | print startup timing Prints process performance.now() timing and exits; not a resolved-configuration benchmark. |
| `debug v2` | Yes | debug v2 catalog and built-in plugins Finite command without user input according to its registered handler; help inspected, handler not executed unless noted below. |
| `debug wait` | No | wait indefinitely (for debugging) Long-running service; does not run to completion. |
| `export` | Yes | export session data as JSON Supply sessionID; omitting it invokes a session selector. Exports JSON and --sanitize redacts transcript/file data. |
| `generate` | Yes | Emit the HTTP API OpenAPI document as JSON. Hidden registered command; emits OpenAPI JSON and is accepted by installed binary. |
| `github` | No | Groups GitHub setup and agent execution. Command group; select a leaf command. |
| `github install` | No | install the GitHub agent Can request user selection, credentials or browser authorization; only help inspected. |
| `github run` | Yes | run the GitHub agent CI-oriented; may call paid models, tools, git and GitHub. Not executed. |
| `import` | Yes | import session data from JSON file or URL Finite command without user input according to its registered handler; help inspected, handler not executed unless noted below. |
| `mcp` | No | Groups MCP server operations. Command group; select a leaf command. |
| `mcp add` | No | add an MCP server Can request user selection, credentials or browser authorization; only help inspected. |
| `mcp auth` | No | authenticate with an OAuth-enabled MCP server Can request user selection, credentials or browser authorization; only help inspected. |
| `mcp auth list` | Yes | list OAuth-capable MCP servers and their auth status Finite command without user input according to its registered handler; help inspected, handler not executed unless noted below. |
| `mcp auth ls` | Yes | list OAuth-capable MCP servers and their auth status Finite command without user input according to its registered handler; help inspected, handler not executed unless noted below. |
| `mcp debug` | Yes | debug OAuth connection for an MCP server Performs network/OAuth diagnostics without completing authorization; can attempt client registration. |
| `mcp list` | Yes | list MCP servers and their status Finite command without user input according to its registered handler; help inspected, handler not executed unless noted below. |
| `mcp logout` | No | remove OAuth credentials for an MCP server Can request user selection, credentials or browser authorization; only help inspected. |
| `mcp ls` | Yes | list MCP servers and their status Finite command without user input according to its registered handler; help inspected, handler not executed unless noted below. |
| `models` | Yes | list all available models Finite command without user input according to its registered handler; help inspected, handler not executed unless noted below. |
| `plug` | Yes | install plugin and update config Alias of plugin; same installation/configuration side effects. |
| `plugin` | Yes | install plugin and update config Alias plug; installs dependencies and updates opencode/tui configuration. No user selection in its command handler; not executed. |
| `pr` | No | fetch and checkout a GitHub PR branch, then run opencode Requires a terminal/browser, or can prompt during installation. |
| `providers` | No | Groups providers operations. Command group; select a leaf command. |
| `providers list` | Yes | list providers and credentials Finite command without user input according to its registered handler; help inspected, handler not executed unless noted below. |
| `providers login` | No | log in to a provider Can request user selection, credentials or browser authorization; only help inspected. |
| `providers logout` | No | log out from a configured provider Can request user selection, credentials or browser authorization; only help inspected. |
| `providers ls` | Yes | list providers and credentials Finite command without user input according to its registered handler; help inspected, handler not executed unless noted below. |
| `run` | Yes | run opencode with a message Finishes by default; --interactive/-i and --mini request a terminal. Resume uses --continue/-c or --session/-s on this same path, with optional --fork; there is no resume subcommand. |
| `serve` | No | starts a headless opencode server Long-running service; does not run to completion. |
| `session` | No | Groups session listing and deletion. Command group; select a leaf command. |
| `session delete` | Yes | delete a session Finite command without user input according to its registered handler; help inspected, handler not executed unless noted below. |
| `session list` | Yes | list sessions Pipes complete without a pager; --format json returns an array for nonempty results, but empty stdout for no sessions. TTY table output without --max-count opens a pager. |
| `stats` | Yes | show token usage and cost statistics Finite command without user input according to its registered handler; help inspected, handler not executed unless noted below. |
| `uninstall` | Yes | uninstall opencode and remove all related files Finite with --force; otherwise confirmation prompts. Destructive; not executed. |
| `upgrade` | No | upgrade opencode to the latest or a specific version Requires a terminal/browser, or can prompt during installation. |
| `web` | No | start opencode server and open web interface Requires a terminal/browser, or can prompt during installation. |


## CLI Switch Inventory


The inventory covers the root and **all listed paths**, including switches of interactive management commands where source declarations are available. Shared globals apply everywhere. Paths without local options accept the globals. The internal `--get-yargs-completions` protocol is recorded as unknown because it lacks an ordinary typed declaration. Every declaration comes from yargs builders in the linked source files; help corroborates visible spellings. Hidden flags come from source.

OpenCode uses **yargs 18.0.0**, with **yargs-parser 22.0.0**, locked in [package.json](https://github.com/anomalyco/opencode/blob/v1.18.33/packages/opencode/package.json) and [bun.lock](https://github.com/anomalyco/opencode/blob/v1.18.33/bun.lock). `index.ts` changes only `populate--` in parser configuration. [Parser source](https://github.com/yargs/yargs-parser/blob/v22.0.0/lib/yargs-parser.ts) establishes space and equals forms, greedy arrays, missing-value defaults, camel-case expansion and negation. Exact long aliases for short keys (`--m`, `--s`, etc.) are also accepted.

`value_optional: true` reflects syntactic acceptance: undeclared `nargs`/`requiresArg` means an omitted string becomes empty, and an omitted number becomes undefined or its default. Choices or handlers may subsequently reject that value. Boolean `none` means no required value, but yargs can consume a following literal `true` or `false` and accepts `--flag=false`. That exceptional consumption requires wrapper handling outside this contract’s type vocabulary.

Disposable direct-parser probes used the downloaded locked parser with equivalent option declarations. They read actual values: `--file a b`, `--file=a b` and `-f=a b` produced `file: ["a","b"]`; bare `--file` produced `[]`; `-s123` produced session `"123"`; `-m/provider` produced model `"/provider"`; `-mfoo` produced grouped switch keys instead. `-f/tmp/a b` produced file `["/tmp/a"]` and positional `b`. Thus `short_attached` records only restricted numeric/punctuation attachment, **not arbitrary alphabetic strings**. Use space/equals for general values.

The schema cannot represent the observed array minimum **zero**. Array records retain `value_type: variadic` and `variadic_min: unknown` with a gap explaining the representational limit and the required schema/probe follow-up. `stats --models` has no declared type; its record remains `unknown` rather than guessing a number from help.

No run-level system-prompt delivery switch was found in `run.ts`, local help or official CLI docs. Root `--prompt` is a string initial user prompt. System-prompt delivery semantics belong to the system-prompt topic.


### Global


| Switch and exact aliases | Value | Accepted attachment | Purpose and evidence |
| --- | --- | --- | --- |
| `--help`, `-h`, `--h` | none | none | Show help. [declaration](https://github.com/anomalyco/opencode/blob/v1.18.33/packages/opencode/src/index.ts) |
| `--no-help`, `--no-h` | none | none | Set --help to false through yargs boolean negation. [declaration](https://github.com/anomalyco/opencode/blob/v1.18.33/packages/opencode/src/index.ts) |
| `--version`, `-v`, `--v` | none | none | Show version. [declaration](https://github.com/anomalyco/opencode/blob/v1.18.33/packages/opencode/src/index.ts) |
| `--no-version`, `--no-v` | none | none | Set --version to false through yargs boolean negation. [declaration](https://github.com/anomalyco/opencode/blob/v1.18.33/packages/opencode/src/index.ts) |
| `--print-logs`, `--printLogs` | none | none | print logs to stderr [declaration](https://github.com/anomalyco/opencode/blob/v1.18.33/packages/opencode/src/index.ts) |
| `--no-print-logs`, `--no-printLogs` | none | none | Set --print-logs to false through yargs boolean negation. [declaration](https://github.com/anomalyco/opencode/blob/v1.18.33/packages/opencode/src/index.ts) |
| `--log-level`, `--logLevel` | string (optional) | space, equals | log level [declaration](https://github.com/anomalyco/opencode/blob/v1.18.33/packages/opencode/src/index.ts) |
| `--pure` | none | none | run without external plugins [declaration](https://github.com/anomalyco/opencode/blob/v1.18.33/packages/opencode/src/index.ts) |
| `--no-pure` | none | none | Set --pure to false through yargs boolean negation. [declaration](https://github.com/anomalyco/opencode/blob/v1.18.33/packages/opencode/src/index.ts) |
| `--get-yargs-completions` | unknown | none | Internal yargs shell-completion request; generated completion scripts use this switch. [declaration](https://github.com/anomalyco/opencode/blob/v1.18.33/packages/opencode/src/index.ts) |


### Root entrypoint


| Switch and exact aliases | Value | Accepted attachment | Purpose and evidence |
| --- | --- | --- | --- |
| `--model`, `-m`, `--m` | string (optional) | space, equals, short_attached | model to use in the format of provider/model [declaration](https://github.com/anomalyco/opencode/blob/v1.18.33/packages/opencode/src/cli/cmd/tui.ts) |
| `--continue`, `-c`, `--c` | none | none | continue the last session [declaration](https://github.com/anomalyco/opencode/blob/v1.18.33/packages/opencode/src/cli/cmd/tui.ts) |
| `--no-continue`, `--no-c` | none | none | Set --continue to false through yargs boolean negation. [declaration](https://github.com/anomalyco/opencode/blob/v1.18.33/packages/opencode/src/cli/cmd/tui.ts) |
| `--session`, `-s`, `--s` | string (optional) | space, equals, short_attached | session id to continue [declaration](https://github.com/anomalyco/opencode/blob/v1.18.33/packages/opencode/src/cli/cmd/tui.ts) |
| `--fork` | none | none | fork the session when continuing (use with --continue or --session) [declaration](https://github.com/anomalyco/opencode/blob/v1.18.33/packages/opencode/src/cli/cmd/tui.ts) |
| `--no-fork` | none | none | Set --fork to false through yargs boolean negation. [declaration](https://github.com/anomalyco/opencode/blob/v1.18.33/packages/opencode/src/cli/cmd/tui.ts) |
| `--prompt` | string (optional) | space, equals | prompt to use [declaration](https://github.com/anomalyco/opencode/blob/v1.18.33/packages/opencode/src/cli/cmd/tui.ts) |
| `--agent` | string (optional) | space, equals | agent to use [declaration](https://github.com/anomalyco/opencode/blob/v1.18.33/packages/opencode/src/cli/cmd/tui.ts) |
| `--auto` | none | none | auto-approve permissions that are not explicitly denied (dangerous!) [declaration](https://github.com/anomalyco/opencode/blob/v1.18.33/packages/opencode/src/cli/cmd/tui.ts) |
| `--no-auto` | none | none | Set --auto to false through yargs boolean negation. [declaration](https://github.com/anomalyco/opencode/blob/v1.18.33/packages/opencode/src/cli/cmd/tui.ts) |
| `--yolo` | none | none | Hidden automatic approval switch. [declaration](https://github.com/anomalyco/opencode/blob/v1.18.33/packages/opencode/src/cli/cmd/tui.ts) |
| `--no-yolo` | none | none | Set --yolo to false through yargs boolean negation. [declaration](https://github.com/anomalyco/opencode/blob/v1.18.33/packages/opencode/src/cli/cmd/tui.ts) |
| `--dangerously-skip-permissions`, `--dangerouslySkipPermissions` | none | none | Hidden automatic approval switch. [declaration](https://github.com/anomalyco/opencode/blob/v1.18.33/packages/opencode/src/cli/cmd/tui.ts) |
| `--no-dangerously-skip-permissions`, `--no-dangerouslySkipPermissions` | none | none | Set --dangerously-skip-permissions to false through yargs boolean negation. [declaration](https://github.com/anomalyco/opencode/blob/v1.18.33/packages/opencode/src/cli/cmd/tui.ts) |
| `--mini` | none | none | start the minimal interactive interface [declaration](https://github.com/anomalyco/opencode/blob/v1.18.33/packages/opencode/src/cli/cmd/tui.ts) |
| `--no-mini` | none | none | Set --mini to false through yargs boolean negation. [declaration](https://github.com/anomalyco/opencode/blob/v1.18.33/packages/opencode/src/cli/cmd/tui.ts) |
| `--replay` | none | none | Enable interactive session-history replay. [declaration](https://github.com/anomalyco/opencode/blob/v1.18.33/packages/opencode/src/cli/cmd/tui.ts) |
| `--no-replay`, `--noReplay` | none | none | disable mini session history replay on resume and after resize [declaration](https://github.com/anomalyco/opencode/blob/v1.18.33/packages/opencode/src/cli/cmd/tui.ts) |
| `--replay-limit`, `--replayLimit` | number (optional) | space, equals | cap visible mini replay to the newest N messages [declaration](https://github.com/anomalyco/opencode/blob/v1.18.33/packages/opencode/src/cli/cmd/tui.ts) |
| `--demo` | none | none | Enable interactive demo commands. [declaration](https://github.com/anomalyco/opencode/blob/v1.18.33/packages/opencode/src/cli/cmd/tui.ts) |
| `--no-demo` | none | none | Set --demo to false through yargs boolean negation. [declaration](https://github.com/anomalyco/opencode/blob/v1.18.33/packages/opencode/src/cli/cmd/tui.ts) |
| `--port` | number (optional) | space, equals | Port to listen on. [declaration](https://github.com/anomalyco/opencode/blob/v1.18.33/packages/opencode/src/cli/network.ts) |
| `--hostname` | string (optional) | space, equals | Hostname to listen on. [declaration](https://github.com/anomalyco/opencode/blob/v1.18.33/packages/opencode/src/cli/network.ts) |
| `--mdns` | none | none | Enable mDNS discovery. [declaration](https://github.com/anomalyco/opencode/blob/v1.18.33/packages/opencode/src/cli/network.ts) |
| `--no-mdns` | none | none | Set --mdns to false through yargs boolean negation. [declaration](https://github.com/anomalyco/opencode/blob/v1.18.33/packages/opencode/src/cli/network.ts) |
| `--mdns-domain`, `--mdnsDomain` | string (optional) | space, equals | mDNS service domain. [declaration](https://github.com/anomalyco/opencode/blob/v1.18.33/packages/opencode/src/cli/network.ts) |
| `--cors` | variadic; minimum unknown (observed zero) | space, equals | Additional CORS origins. [declaration](https://github.com/anomalyco/opencode/blob/v1.18.33/packages/opencode/src/cli/network.ts) |


### `acp`


| Switch and exact aliases | Value | Accepted attachment | Purpose and evidence |
| --- | --- | --- | --- |
| `--cwd` | string (optional) | space, equals | working directory [declaration](https://github.com/anomalyco/opencode/blob/v1.18.33/packages/opencode/src/cli/cmd/acp.ts) |
| `--port` | number (optional) | space, equals | Port to listen on. [declaration](https://github.com/anomalyco/opencode/blob/v1.18.33/packages/opencode/src/cli/network.ts) |
| `--hostname` | string (optional) | space, equals | Hostname to listen on. [declaration](https://github.com/anomalyco/opencode/blob/v1.18.33/packages/opencode/src/cli/network.ts) |
| `--mdns` | none | none | Enable mDNS discovery. [declaration](https://github.com/anomalyco/opencode/blob/v1.18.33/packages/opencode/src/cli/network.ts) |
| `--no-mdns` | none | none | Set --mdns to false through yargs boolean negation. [declaration](https://github.com/anomalyco/opencode/blob/v1.18.33/packages/opencode/src/cli/network.ts) |
| `--mdns-domain`, `--mdnsDomain` | string (optional) | space, equals | mDNS service domain. [declaration](https://github.com/anomalyco/opencode/blob/v1.18.33/packages/opencode/src/cli/network.ts) |
| `--cors` | variadic; minimum unknown (observed zero) | space, equals | Additional CORS origins. [declaration](https://github.com/anomalyco/opencode/blob/v1.18.33/packages/opencode/src/cli/network.ts) |


### `agent create`


| Switch and exact aliases | Value | Accepted attachment | Purpose and evidence |
| --- | --- | --- | --- |
| `--path` | string (optional) | space, equals | directory path to generate the agent file [declaration](https://github.com/anomalyco/opencode/blob/v1.18.33/packages/opencode/src/cli/cmd/agent.ts) |
| `--description` | string (optional) | space, equals | what the agent should do [declaration](https://github.com/anomalyco/opencode/blob/v1.18.33/packages/opencode/src/cli/cmd/agent.ts) |
| `--mode` | string (optional) | space, equals | agent mode [declaration](https://github.com/anomalyco/opencode/blob/v1.18.33/packages/opencode/src/cli/cmd/agent.ts) |
| `--permissions`, `--tools` | string (optional) | space, equals | Comma-separated permissions allowed in the generated agent; --tools is the same switch. [declaration](https://github.com/anomalyco/opencode/blob/v1.18.33/packages/opencode/src/cli/cmd/agent.ts) |
| `--model`, `-m`, `--m` | string (optional) | space, equals, short_attached | model to use in the format of provider/model [declaration](https://github.com/anomalyco/opencode/blob/v1.18.33/packages/opencode/src/cli/cmd/agent.ts) |


### `attach`


| Switch and exact aliases | Value | Accepted attachment | Purpose and evidence |
| --- | --- | --- | --- |
| `--dir` | string (optional) | space, equals | directory to run in [declaration](https://github.com/anomalyco/opencode/blob/v1.18.33/packages/opencode/src/cli/cmd/attach.ts) |
| `--continue`, `-c`, `--c` | none | none | continue the last session [declaration](https://github.com/anomalyco/opencode/blob/v1.18.33/packages/opencode/src/cli/cmd/attach.ts) |
| `--no-continue`, `--no-c` | none | none | Set --continue to false through yargs boolean negation. [declaration](https://github.com/anomalyco/opencode/blob/v1.18.33/packages/opencode/src/cli/cmd/attach.ts) |
| `--session`, `-s`, `--s` | string (optional) | space, equals, short_attached | session id to continue [declaration](https://github.com/anomalyco/opencode/blob/v1.18.33/packages/opencode/src/cli/cmd/attach.ts) |
| `--fork` | none | none | fork the session when continuing (use with --continue or --session) [declaration](https://github.com/anomalyco/opencode/blob/v1.18.33/packages/opencode/src/cli/cmd/attach.ts) |
| `--no-fork` | none | none | Set --fork to false through yargs boolean negation. [declaration](https://github.com/anomalyco/opencode/blob/v1.18.33/packages/opencode/src/cli/cmd/attach.ts) |
| `--password`, `-p`, `--p` | string (optional) | space, equals, short_attached | basic auth password (defaults to OPENCODE_SERVER_PASSWORD) [declaration](https://github.com/anomalyco/opencode/blob/v1.18.33/packages/opencode/src/cli/cmd/attach.ts) |
| `--username`, `-u`, `--u` | string (optional) | space, equals, short_attached | basic auth username (defaults to OPENCODE_SERVER_USERNAME or 'opencode') [declaration](https://github.com/anomalyco/opencode/blob/v1.18.33/packages/opencode/src/cli/cmd/attach.ts) |
| `--mini` | none | none | start the minimal interactive interface [declaration](https://github.com/anomalyco/opencode/blob/v1.18.33/packages/opencode/src/cli/cmd/attach.ts) |
| `--no-mini` | none | none | Set --mini to false through yargs boolean negation. [declaration](https://github.com/anomalyco/opencode/blob/v1.18.33/packages/opencode/src/cli/cmd/attach.ts) |
| `--replay` | none | none | Enable interactive session-history replay. [declaration](https://github.com/anomalyco/opencode/blob/v1.18.33/packages/opencode/src/cli/cmd/attach.ts) |
| `--no-replay`, `--noReplay` | none | none | disable mini session history replay on resume and after resize [declaration](https://github.com/anomalyco/opencode/blob/v1.18.33/packages/opencode/src/cli/cmd/attach.ts) |
| `--replay-limit`, `--replayLimit` | number (optional) | space, equals | cap visible mini replay to the newest N messages [declaration](https://github.com/anomalyco/opencode/blob/v1.18.33/packages/opencode/src/cli/cmd/attach.ts) |


### `auth login`


| Switch and exact aliases | Value | Accepted attachment | Purpose and evidence |
| --- | --- | --- | --- |
| `--provider`, `-p`, `--p` | string (optional) | space, equals, short_attached | provider id or name to log in to (skips provider selection) [declaration](https://github.com/anomalyco/opencode/blob/v1.18.33/packages/opencode/src/cli/cmd/providers.ts) |
| `--method`, `-m`, `--m` | string (optional) | space, equals, short_attached | login method label (skips method selection) [declaration](https://github.com/anomalyco/opencode/blob/v1.18.33/packages/opencode/src/cli/cmd/providers.ts) |


### `db`


| Switch and exact aliases | Value | Accepted attachment | Purpose and evidence |
| --- | --- | --- | --- |
| `--format` | string (optional) | space, equals | Output format [declaration](https://github.com/anomalyco/opencode/blob/v1.18.33/packages/opencode/src/cli/cmd/db.ts) |


### `debug agent`


| Switch and exact aliases | Value | Accepted attachment | Purpose and evidence |
| --- | --- | --- | --- |
| `--tool` | string (optional) | space, equals | Tool id to execute [declaration](https://github.com/anomalyco/opencode/blob/v1.18.33/packages/opencode/src/cli/cmd/debug/agent.ts) |
| `--params` | string (optional) | space, equals | Tool params as JSON or a JS object literal [declaration](https://github.com/anomalyco/opencode/blob/v1.18.33/packages/opencode/src/cli/cmd/debug/agent.ts) |


### `debug rg files`


| Switch and exact aliases | Value | Accepted attachment | Purpose and evidence |
| --- | --- | --- | --- |
| `--query` | string (optional) | space, equals | Filter files by query [declaration](https://github.com/anomalyco/opencode/blob/v1.18.33/packages/opencode/src/cli/cmd/debug/ripgrep.ts) |
| `--glob` | string (optional) | space, equals | Glob pattern to match files [declaration](https://github.com/anomalyco/opencode/blob/v1.18.33/packages/opencode/src/cli/cmd/debug/ripgrep.ts) |
| `--limit` | number (optional) | space, equals | Limit number of results [declaration](https://github.com/anomalyco/opencode/blob/v1.18.33/packages/opencode/src/cli/cmd/debug/ripgrep.ts) |


### `debug rg search`


| Switch and exact aliases | Value | Accepted attachment | Purpose and evidence |
| --- | --- | --- | --- |
| `--glob` | variadic; minimum unknown (observed zero) | space, equals | File glob patterns [declaration](https://github.com/anomalyco/opencode/blob/v1.18.33/packages/opencode/src/cli/cmd/debug/ripgrep.ts) |
| `--limit` | number (optional) | space, equals | Limit number of results [declaration](https://github.com/anomalyco/opencode/blob/v1.18.33/packages/opencode/src/cli/cmd/debug/ripgrep.ts) |


### `export`


| Switch and exact aliases | Value | Accepted attachment | Purpose and evidence |
| --- | --- | --- | --- |
| `--sanitize` | none | none | redact sensitive transcript and file data [declaration](https://github.com/anomalyco/opencode/blob/v1.18.33/packages/opencode/src/cli/cmd/export.ts) |
| `--no-sanitize` | none | none | Set --sanitize to false through yargs boolean negation. [declaration](https://github.com/anomalyco/opencode/blob/v1.18.33/packages/opencode/src/cli/cmd/export.ts) |


### `github run`


| Switch and exact aliases | Value | Accepted attachment | Purpose and evidence |
| --- | --- | --- | --- |
| `--event` | string (optional) | space, equals | GitHub mock event to run the agent for [declaration](https://github.com/anomalyco/opencode/blob/v1.18.33/packages/opencode/src/cli/cmd/github.ts) |
| `--token` | string (optional) | space, equals | GitHub personal access token (github_pat_********) [declaration](https://github.com/anomalyco/opencode/blob/v1.18.33/packages/opencode/src/cli/cmd/github.ts) |


### `mcp add`


| Switch and exact aliases | Value | Accepted attachment | Purpose and evidence |
| --- | --- | --- | --- |
| `--url` | string (optional) | space, equals | URL for a remote MCP server [declaration](https://github.com/anomalyco/opencode/blob/v1.18.33/packages/opencode/src/cli/cmd/mcp.ts) |
| `--env` | variadic; minimum unknown (observed zero) | space, equals | environment variable for a local MCP server (KEY=VALUE) [declaration](https://github.com/anomalyco/opencode/blob/v1.18.33/packages/opencode/src/cli/cmd/mcp.ts) |
| `--header` | variadic; minimum unknown (observed zero) | space, equals | HTTP header for a remote MCP server (KEY=VALUE) [declaration](https://github.com/anomalyco/opencode/blob/v1.18.33/packages/opencode/src/cli/cmd/mcp.ts) |


### `models`


| Switch and exact aliases | Value | Accepted attachment | Purpose and evidence |
| --- | --- | --- | --- |
| `--verbose` | none | none | use more verbose model output (includes metadata like costs) [declaration](https://github.com/anomalyco/opencode/blob/v1.18.33/packages/opencode/src/cli/cmd/models.ts) |
| `--no-verbose` | none | none | Set --verbose to false through yargs boolean negation. [declaration](https://github.com/anomalyco/opencode/blob/v1.18.33/packages/opencode/src/cli/cmd/models.ts) |
| `--refresh` | none | none | refresh the models cache from models.dev [declaration](https://github.com/anomalyco/opencode/blob/v1.18.33/packages/opencode/src/cli/cmd/models.ts) |
| `--no-refresh` | none | none | Set --refresh to false through yargs boolean negation. [declaration](https://github.com/anomalyco/opencode/blob/v1.18.33/packages/opencode/src/cli/cmd/models.ts) |


### `plug`


| Switch and exact aliases | Value | Accepted attachment | Purpose and evidence |
| --- | --- | --- | --- |
| `--global`, `-g`, `--g` | none | none | install in global config [declaration](https://github.com/anomalyco/opencode/blob/v1.18.33/packages/opencode/src/cli/cmd/plug.ts) |
| `--no-global`, `--no-g` | none | none | Set --global to false through yargs boolean negation. [declaration](https://github.com/anomalyco/opencode/blob/v1.18.33/packages/opencode/src/cli/cmd/plug.ts) |
| `--force`, `-f`, `--f` | none | none | replace existing plugin version [declaration](https://github.com/anomalyco/opencode/blob/v1.18.33/packages/opencode/src/cli/cmd/plug.ts) |
| `--no-force`, `--no-f` | none | none | Set --force to false through yargs boolean negation. [declaration](https://github.com/anomalyco/opencode/blob/v1.18.33/packages/opencode/src/cli/cmd/plug.ts) |


### `plugin`


| Switch and exact aliases | Value | Accepted attachment | Purpose and evidence |
| --- | --- | --- | --- |
| `--global`, `-g`, `--g` | none | none | install in global config [declaration](https://github.com/anomalyco/opencode/blob/v1.18.33/packages/opencode/src/cli/cmd/plug.ts) |
| `--no-global`, `--no-g` | none | none | Set --global to false through yargs boolean negation. [declaration](https://github.com/anomalyco/opencode/blob/v1.18.33/packages/opencode/src/cli/cmd/plug.ts) |
| `--force`, `-f`, `--f` | none | none | replace existing plugin version [declaration](https://github.com/anomalyco/opencode/blob/v1.18.33/packages/opencode/src/cli/cmd/plug.ts) |
| `--no-force`, `--no-f` | none | none | Set --force to false through yargs boolean negation. [declaration](https://github.com/anomalyco/opencode/blob/v1.18.33/packages/opencode/src/cli/cmd/plug.ts) |


### `providers login`


| Switch and exact aliases | Value | Accepted attachment | Purpose and evidence |
| --- | --- | --- | --- |
| `--provider`, `-p`, `--p` | string (optional) | space, equals, short_attached | provider id or name to log in to (skips provider selection) [declaration](https://github.com/anomalyco/opencode/blob/v1.18.33/packages/opencode/src/cli/cmd/providers.ts) |
| `--method`, `-m`, `--m` | string (optional) | space, equals, short_attached | login method label (skips method selection) [declaration](https://github.com/anomalyco/opencode/blob/v1.18.33/packages/opencode/src/cli/cmd/providers.ts) |


### `run`


| Switch and exact aliases | Value | Accepted attachment | Purpose and evidence |
| --- | --- | --- | --- |
| `--command` | string (optional) | space, equals | the command to run, use message for args [declaration](https://github.com/anomalyco/opencode/blob/v1.18.33/packages/opencode/src/cli/cmd/run.ts) |
| `--continue`, `-c`, `--c` | none | none | continue the last session [declaration](https://github.com/anomalyco/opencode/blob/v1.18.33/packages/opencode/src/cli/cmd/run.ts) |
| `--no-continue`, `--no-c` | none | none | Set --continue to false through yargs boolean negation. [declaration](https://github.com/anomalyco/opencode/blob/v1.18.33/packages/opencode/src/cli/cmd/run.ts) |
| `--session`, `-s`, `--s` | string (optional) | space, equals, short_attached | session id to continue [declaration](https://github.com/anomalyco/opencode/blob/v1.18.33/packages/opencode/src/cli/cmd/run.ts) |
| `--fork` | none | none | fork the session before continuing (requires --continue or --session) [declaration](https://github.com/anomalyco/opencode/blob/v1.18.33/packages/opencode/src/cli/cmd/run.ts) |
| `--no-fork` | none | none | Set --fork to false through yargs boolean negation. [declaration](https://github.com/anomalyco/opencode/blob/v1.18.33/packages/opencode/src/cli/cmd/run.ts) |
| `--share` | none | none | share the session [declaration](https://github.com/anomalyco/opencode/blob/v1.18.33/packages/opencode/src/cli/cmd/run.ts) |
| `--no-share` | none | none | Set --share to false through yargs boolean negation. [declaration](https://github.com/anomalyco/opencode/blob/v1.18.33/packages/opencode/src/cli/cmd/run.ts) |
| `--model`, `-m`, `--m` | string (optional) | space, equals, short_attached | model to use in the format of provider/model [declaration](https://github.com/anomalyco/opencode/blob/v1.18.33/packages/opencode/src/cli/cmd/run.ts) |
| `--agent` | string (optional) | space, equals | agent to use [declaration](https://github.com/anomalyco/opencode/blob/v1.18.33/packages/opencode/src/cli/cmd/run.ts) |
| `--format` | string (optional) | space, equals | format: default (formatted) or json (raw JSON events) [declaration](https://github.com/anomalyco/opencode/blob/v1.18.33/packages/opencode/src/cli/cmd/run.ts) |
| `--file`, `-f`, `--f` | variadic; minimum unknown (observed zero) | space, equals, short_attached | file(s) to attach to message [declaration](https://github.com/anomalyco/opencode/blob/v1.18.33/packages/opencode/src/cli/cmd/run.ts) |
| `--title` | string (optional) | space, equals | title for the session (uses truncated prompt if no value provided) [declaration](https://github.com/anomalyco/opencode/blob/v1.18.33/packages/opencode/src/cli/cmd/run.ts) |
| `--attach` | string (optional) | space, equals | attach to a running opencode server (e.g., http://localhost:4096) [declaration](https://github.com/anomalyco/opencode/blob/v1.18.33/packages/opencode/src/cli/cmd/run.ts) |
| `--password`, `-p`, `--p` | string (optional) | space, equals, short_attached | basic auth password (defaults to OPENCODE_SERVER_PASSWORD) [declaration](https://github.com/anomalyco/opencode/blob/v1.18.33/packages/opencode/src/cli/cmd/run.ts) |
| `--username`, `-u`, `--u` | string (optional) | space, equals, short_attached | basic auth username (defaults to OPENCODE_SERVER_USERNAME or 'opencode') [declaration](https://github.com/anomalyco/opencode/blob/v1.18.33/packages/opencode/src/cli/cmd/run.ts) |
| `--dir` | string (optional) | space, equals | directory to run in, path on remote server if attaching [declaration](https://github.com/anomalyco/opencode/blob/v1.18.33/packages/opencode/src/cli/cmd/run.ts) |
| `--port` | number (optional) | space, equals | port for the local server (defaults to random port if no value provided) [declaration](https://github.com/anomalyco/opencode/blob/v1.18.33/packages/opencode/src/cli/cmd/run.ts) |
| `--variant` | string (optional) | space, equals | model variant (provider-specific reasoning effort, e.g., high, max, minimal) [declaration](https://github.com/anomalyco/opencode/blob/v1.18.33/packages/opencode/src/cli/cmd/run.ts) |
| `--thinking` | none | none | show thinking blocks [declaration](https://github.com/anomalyco/opencode/blob/v1.18.33/packages/opencode/src/cli/cmd/run.ts) |
| `--no-thinking` | none | none | Set --thinking to false through yargs boolean negation. [declaration](https://github.com/anomalyco/opencode/blob/v1.18.33/packages/opencode/src/cli/cmd/run.ts) |
| `--mini` | none | none | Select the minimal interactive interface. [declaration](https://github.com/anomalyco/opencode/blob/v1.18.33/packages/opencode/src/cli/cmd/run.ts) |
| `--no-mini` | none | none | Set --mini to false through yargs boolean negation. [declaration](https://github.com/anomalyco/opencode/blob/v1.18.33/packages/opencode/src/cli/cmd/run.ts) |
| `--replay` | none | none | replay interactive session history on resume and after resize (use --no-replay to disable) [declaration](https://github.com/anomalyco/opencode/blob/v1.18.33/packages/opencode/src/cli/cmd/run.ts) |
| `--replay-limit`, `--replayLimit` | number (optional) | space, equals | cap visible interactive replay to the newest N messages [declaration](https://github.com/anomalyco/opencode/blob/v1.18.33/packages/opencode/src/cli/cmd/run.ts) |
| `--interactive`, `-i`, `--i` | none | none | run in direct interactive split-footer mode [declaration](https://github.com/anomalyco/opencode/blob/v1.18.33/packages/opencode/src/cli/cmd/run.ts) |
| `--no-interactive`, `--no-i` | none | none | Set --interactive to false through yargs boolean negation. [declaration](https://github.com/anomalyco/opencode/blob/v1.18.33/packages/opencode/src/cli/cmd/run.ts) |
| `--auto` | none | none | auto-approve permissions that are not explicitly denied (dangerous!) [declaration](https://github.com/anomalyco/opencode/blob/v1.18.33/packages/opencode/src/cli/cmd/run.ts) |
| `--no-auto` | none | none | Set --auto to false through yargs boolean negation. [declaration](https://github.com/anomalyco/opencode/blob/v1.18.33/packages/opencode/src/cli/cmd/run.ts) |
| `--yolo` | none | none | Hidden automatic approval switch. [declaration](https://github.com/anomalyco/opencode/blob/v1.18.33/packages/opencode/src/cli/cmd/run.ts) |
| `--no-yolo` | none | none | Set --yolo to false through yargs boolean negation. [declaration](https://github.com/anomalyco/opencode/blob/v1.18.33/packages/opencode/src/cli/cmd/run.ts) |
| `--dangerously-skip-permissions`, `--dangerouslySkipPermissions` | none | none | Hidden automatic approval switch. [declaration](https://github.com/anomalyco/opencode/blob/v1.18.33/packages/opencode/src/cli/cmd/run.ts) |
| `--no-dangerously-skip-permissions`, `--no-dangerouslySkipPermissions` | none | none | Set --dangerously-skip-permissions to false through yargs boolean negation. [declaration](https://github.com/anomalyco/opencode/blob/v1.18.33/packages/opencode/src/cli/cmd/run.ts) |
| `--demo` | none | none | enable direct interactive demo slash commands; pass one as the message to run it immediately [declaration](https://github.com/anomalyco/opencode/blob/v1.18.33/packages/opencode/src/cli/cmd/run.ts) |
| `--no-demo` | none | none | Set --demo to false through yargs boolean negation. [declaration](https://github.com/anomalyco/opencode/blob/v1.18.33/packages/opencode/src/cli/cmd/run.ts) |
| `--no-replay` | none | none | Disable interactive history replay through yargs negation. [declaration](https://github.com/anomalyco/opencode/blob/v1.18.33/packages/opencode/src/cli/cmd/run.ts) |


### `serve`


| Switch and exact aliases | Value | Accepted attachment | Purpose and evidence |
| --- | --- | --- | --- |
| `--port` | number (optional) | space, equals | Port to listen on. [declaration](https://github.com/anomalyco/opencode/blob/v1.18.33/packages/opencode/src/cli/network.ts) |
| `--hostname` | string (optional) | space, equals | Hostname to listen on. [declaration](https://github.com/anomalyco/opencode/blob/v1.18.33/packages/opencode/src/cli/network.ts) |
| `--mdns` | none | none | Enable mDNS discovery. [declaration](https://github.com/anomalyco/opencode/blob/v1.18.33/packages/opencode/src/cli/network.ts) |
| `--no-mdns` | none | none | Set --mdns to false through yargs boolean negation. [declaration](https://github.com/anomalyco/opencode/blob/v1.18.33/packages/opencode/src/cli/network.ts) |
| `--mdns-domain`, `--mdnsDomain` | string (optional) | space, equals | mDNS service domain. [declaration](https://github.com/anomalyco/opencode/blob/v1.18.33/packages/opencode/src/cli/network.ts) |
| `--cors` | variadic; minimum unknown (observed zero) | space, equals | Additional CORS origins. [declaration](https://github.com/anomalyco/opencode/blob/v1.18.33/packages/opencode/src/cli/network.ts) |


### `session list`


| Switch and exact aliases | Value | Accepted attachment | Purpose and evidence |
| --- | --- | --- | --- |
| `--max-count`, `-n`, `--n`, `--maxCount` | number (optional) | space, equals, short_attached | limit to N most recent sessions [declaration](https://github.com/anomalyco/opencode/blob/v1.18.33/packages/opencode/src/cli/cmd/session.ts) |
| `--format` | string (optional) | space, equals | output format [declaration](https://github.com/anomalyco/opencode/blob/v1.18.33/packages/opencode/src/cli/cmd/session.ts) |


### `stats`


| Switch and exact aliases | Value | Accepted attachment | Purpose and evidence |
| --- | --- | --- | --- |
| `--days` | number (optional) | space, equals | show stats for the last N days (default: all time) [declaration](https://github.com/anomalyco/opencode/blob/v1.18.33/packages/opencode/src/cli/cmd/stats.ts) |
| `--tools` | number (optional) | space, equals | number of tools to show (default: all) [declaration](https://github.com/anomalyco/opencode/blob/v1.18.33/packages/opencode/src/cli/cmd/stats.ts) |
| `--models` | unknown | none | show model statistics (default: hidden). Pass a number to show top N, otherwise shows all [declaration](https://github.com/anomalyco/opencode/blob/v1.18.33/packages/opencode/src/cli/cmd/stats.ts) |
| `--project` | string (optional) | space, equals | filter by project (default: all projects, empty string: current project) [declaration](https://github.com/anomalyco/opencode/blob/v1.18.33/packages/opencode/src/cli/cmd/stats.ts) |


### `uninstall`


| Switch and exact aliases | Value | Accepted attachment | Purpose and evidence |
| --- | --- | --- | --- |
| `--keep-config`, `-c`, `--c`, `--keepConfig` | none | none | keep configuration files [declaration](https://github.com/anomalyco/opencode/blob/v1.18.33/packages/opencode/src/cli/cmd/uninstall.ts) |
| `--no-keep-config`, `--no-keepConfig`, `--no-c` | none | none | Set --keep-config to false through yargs boolean negation. [declaration](https://github.com/anomalyco/opencode/blob/v1.18.33/packages/opencode/src/cli/cmd/uninstall.ts) |
| `--keep-data`, `-d`, `--d`, `--keepData` | none | none | keep session data and snapshots [declaration](https://github.com/anomalyco/opencode/blob/v1.18.33/packages/opencode/src/cli/cmd/uninstall.ts) |
| `--no-keep-data`, `--no-keepData`, `--no-d` | none | none | Set --keep-data to false through yargs boolean negation. [declaration](https://github.com/anomalyco/opencode/blob/v1.18.33/packages/opencode/src/cli/cmd/uninstall.ts) |
| `--dry-run`, `--dryRun` | none | none | show what would be removed without removing [declaration](https://github.com/anomalyco/opencode/blob/v1.18.33/packages/opencode/src/cli/cmd/uninstall.ts) |
| `--no-dry-run`, `--no-dryRun` | none | none | Set --dry-run to false through yargs boolean negation. [declaration](https://github.com/anomalyco/opencode/blob/v1.18.33/packages/opencode/src/cli/cmd/uninstall.ts) |
| `--force`, `-f`, `--f` | none | none | skip confirmation prompts [declaration](https://github.com/anomalyco/opencode/blob/v1.18.33/packages/opencode/src/cli/cmd/uninstall.ts) |
| `--no-force`, `--no-f` | none | none | Set --force to false through yargs boolean negation. [declaration](https://github.com/anomalyco/opencode/blob/v1.18.33/packages/opencode/src/cli/cmd/uninstall.ts) |


### `upgrade`


| Switch and exact aliases | Value | Accepted attachment | Purpose and evidence |
| --- | --- | --- | --- |
| `--method`, `-m`, `--m` | string (optional) | space, equals, short_attached | installation method to use [declaration](https://github.com/anomalyco/opencode/blob/v1.18.33/packages/opencode/src/cli/cmd/upgrade.ts) |


### `web`


| Switch and exact aliases | Value | Accepted attachment | Purpose and evidence |
| --- | --- | --- | --- |
| `--port` | number (optional) | space, equals | Port to listen on. [declaration](https://github.com/anomalyco/opencode/blob/v1.18.33/packages/opencode/src/cli/network.ts) |
| `--hostname` | string (optional) | space, equals | Hostname to listen on. [declaration](https://github.com/anomalyco/opencode/blob/v1.18.33/packages/opencode/src/cli/network.ts) |
| `--mdns` | none | none | Enable mDNS discovery. [declaration](https://github.com/anomalyco/opencode/blob/v1.18.33/packages/opencode/src/cli/network.ts) |
| `--no-mdns` | none | none | Set --mdns to false through yargs boolean negation. [declaration](https://github.com/anomalyco/opencode/blob/v1.18.33/packages/opencode/src/cli/network.ts) |
| `--mdns-domain`, `--mdnsDomain` | string (optional) | space, equals | mDNS service domain. [declaration](https://github.com/anomalyco/opencode/blob/v1.18.33/packages/opencode/src/cli/network.ts) |
| `--cors` | variadic; minimum unknown (observed zero) | space, equals | Additional CORS origins. [declaration](https://github.com/anomalyco/opencode/blob/v1.18.33/packages/opencode/src/cli/network.ts) |


## Configuration Discovery


The [main loader](https://github.com/anomalyco/opencode/blob/v1.18.33/packages/opencode/src/config/config.ts), [path discovery](https://github.com/anomalyco/opencode/blob/v1.18.33/packages/opencode/src/config/paths.ts) and [TUI loader](https://github.com/anomalyco/opencode/blob/v1.18.33/packages/opencode/src/config/tui.ts) are the authority. Main settings merge in this order: authenticated `.well-known/opencode` remote configuration, global configuration, `OPENCODE_CONFIG`, upward-discovered project settings, resource directories, `OPENCODE_CONFIG_CONTENT`, system managed settings, then macOS managed preferences. Later keys override earlier ones; selected arrays such as instructions/plugins are combined.

Global files are read in order `config.json`, `opencode.json`, `opencode.jsonc`. A legacy TOML file named `config` has a migration path. XDG defaults are `~/.config/opencode`, `~/.local/share/opencode`, `~/.cache/opencode`, and `~/.local/state/opencode`; the implementation uses xdg-basedir on every OS. Do not assume native Windows uses APPDATA. `XDG_*_HOME` overrides those roots. Project `opencode.json`/`.jsonc` and `.opencode` directories are found upward from the working directory; the home `.opencode` and `OPENCODE_CONFIG_DIR` can also contribute resources.

TUI settings merge separately: global `tui.json`/`.jsonc`, explicit `OPENCODE_TUI_CONFIG`, project TUI files, then discovered `.opencode`/custom-directory TUI files. TUI startup may migrate old inline TUI keys into `tui.json`.

| OS | Managed directory | Extra policy source |
| --- | --- | --- |
| macOS | `/Library/Application Support/opencode` | `/Library/Managed Preferences[/<username>]/ai.opencode.managed.plist` |
| Linux / WSL | `/etc/opencode` | None in the managed loader |
| Windows | `%ProgramData%\opencode` (fallback `C:\ProgramData\opencode`) | None in the managed loader |

Both `opencode.json` and `opencode.jsonc` are read in each managed directory. [Managed source](https://github.com/anomalyco/opencode/blob/v1.18.33/packages/opencode/src/config/managed.ts) establishes those OS-specific paths. Provider credentials use `~/.local/share/opencode/auth.json`, relocated by `XDG_DATA_HOME`. Login/logout write the credential store; plugin installation updates main/TUI config and installs packages. Startup can seed a missing global configuration and create data/cache/state directories.

Read-only inspection of the actual `/Users/ken/.config/opencode` found **config.json, opencode.json and opencode.jsonc**, plus plugin/agent/command/skill directories and package dependency files. No `tui.json` or `tui.jsonc` was present. Contents and credentials were not published or changed. This replaces the previous document’s now-stale claim that configuration consisted only of a schema-only JSONC file.


## Environment Variables


The [core flag reader](https://github.com/anomalyco/opencode/blob/v1.18.33/packages/core/src/flag/flag.ts) and [runtime flags](https://github.com/anomalyco/opencode/blob/v1.18.33/packages/opencode/src/effect/runtime-flags.ts) establish the following runtime surface. Core `truthy()` accepts case-insensitive `true` or `1`; runtime flags use Effect config boolean parsing. Positive numeric runtime overrides require integers greater than zero.

| Variable | Effect |
| --- | --- | --- |
| `OPENCODE_CONFIG` | Load an explicit main configuration file. |
| `OPENCODE_TUI_CONFIG` | Load an explicit TUI configuration file. |
| `OPENCODE_CONFIG_DIR` | Add a configuration/resource discovery directory. |
| `OPENCODE_CONFIG_CONTENT` | Merge inline JSON configuration after ordinary discovery and before managed settings. |
| `OPENCODE_DISABLE_PROJECT_CONFIG` | Disable project configuration and project .opencode discovery. |
| `OPENCODE_PURE` | Disable external plugins; --pure sets this to 1. |
| `OPENCODE_AUTO_SHARE` | Automatically share sessions. |
| `OPENCODE_DISABLE_SHARE` | Disable session sharing (literal true or 1). |
| `OPENCODE_GIT_BASH_PATH` | Select the Git Bash executable on Windows. |
| `OPENCODE_DISABLE_AUTOUPDATE` | Disable automatic update checks. |
| `OPENCODE_ALWAYS_NOTIFY_UPDATE` | Always notify about available updates. |
| `OPENCODE_DISABLE_PRUNE` | Disable old-data pruning. |
| `OPENCODE_DISABLE_TERMINAL_TITLE` | Disable terminal-title updates. |
| `OPENCODE_DISABLE_AUTOCOMPACT` | Disable automatic context compaction. |
| `OPENCODE_DISABLE_MOUSE` | Disable TUI mouse capture. |
| `OPENCODE_DISABLE_DEFAULT_PLUGINS` | Disable default plugins. |
| `OPENCODE_DISABLE_EXTERNAL_SKILLS` | Disable external skill discovery. |
| `OPENCODE_DISABLE_CLAUDE_CODE` | Disable Claude prompt and skill discovery. |
| `OPENCODE_DISABLE_CLAUDE_CODE_PROMPT` | Disable Claude prompt discovery; detailed prompt semantics are owned by system-prompt. |
| `OPENCODE_DISABLE_CLAUDE_CODE_SKILLS` | Disable Claude skill discovery. |
| `OPENCODE_DISABLE_LSP_DOWNLOAD` | Disable automatic language-server downloads. |
| `OPENCODE_DISABLE_EMBEDDED_WEB_UI` | Disable embedded web UI. |
| `OPENCODE_ENABLE_EXA` | Enable Exa search tools. |
| `OPENCODE_ENABLE_PARALLEL` | Enable Parallel search tools. |
| `OPENCODE_ENABLE_EXPERIMENTAL_MODELS` | Enable experimental model offerings. |
| `OPENCODE_DISABLE_MODELS_FETCH` | Disable remote model-catalog fetching. |
| `OPENCODE_MODELS_URL` | Override model-catalog URL, not a model inference endpoint. |
| `OPENCODE_MODELS_PATH` | Select a local model-catalog file. |
| `OPENCODE_CLIENT` | Client identifier, default cli. |
| `OPENCODE_SERVER_PASSWORD` | Enable server HTTP basic authentication. |
| `OPENCODE_SERVER_USERNAME` | HTTP basic-auth username, default opencode. |
| `OPENCODE_DB` | Database path: absolute and :memory: used directly; relative paths resolve under the data directory. |
| `OPENCODE_DISABLE_CHANNEL_DB` | Use the unqualified database instead of a channel-specific database (literal 1 or true). |
| `OPENCODE_DISABLE_FFF` | Disable native fast file finder; defaults to disabled on Windows. |
| `OPENCODE_FAKE_VCS` | Override detected VCS for testing. |
| `OPENCODE_WORKSPACE_ID` | Select workspace context. |
| `OPENCODE_PLUGIN_META_FILE` | Plugin metadata file path. |
| `OPENCODE` | CLI writes 1 as a child-process marker. |
| `OPENCODE_PID` | CLI writes its process ID for child processes. |
| `AGENT` | CLI writes 1 as a child-process agent marker. |
| `XDG_CONFIG_HOME` | Relocate default configuration directory (append opencode). |
| `XDG_DATA_HOME` | Relocate persistent data and auth (append opencode). |
| `XDG_CACHE_HOME` | Relocate caches (append opencode). |
| `XDG_STATE_HOME` | Relocate state (append opencode). |
| `OPENCODE_EXPERIMENTAL` | Enable the experimental umbrella; individual switches may opt out or combine with it. |
| `OPENCODE_EXPERIMENTAL_BASH_DEFAULT_TIMEOUT_MS` | Set a positive integer default bash-tool timeout in milliseconds. |
| `OPENCODE_EXPERIMENTAL_OUTPUT_TOKEN_MAX` | Set a positive integer maximum model output token count. |
| `OPENCODE_EXPERIMENTAL_FILEWATCHER` | Enable whole-directory file watching. |
| `OPENCODE_EXPERIMENTAL_DISABLE_FILEWATCHER` | Disable file watching. |
| `OPENCODE_EXPERIMENTAL_DISABLE_COPY_ON_SELECT` | Disable copy on selection; default true on Windows. |
| `OPENCODE_EXPERIMENTAL_EXA` | Legacy Exa search toggle; combines with umbrella and OPENCODE_ENABLE_EXA. |
| `OPENCODE_EXPERIMENTAL_PARALLEL` | Legacy Parallel search toggle; combines with OPENCODE_ENABLE_PARALLEL. |
| `OPENCODE_EXPERIMENTAL_NATIVE_LLM` | Enable native LLM request path. |
| `OPENCODE_EXPERIMENTAL_WEBSOCKETS` | Enable experimental WebSocket transport. |
| `OPENCODE_EXPERIMENTAL_REFERENCES` | Enable references; defaults to the experimental umbrella when unset. |
| `OPENCODE_EXPERIMENTAL_BACKGROUND_SUBAGENTS` | Enable background subagent tasks; defaults to the experimental umbrella when unset. |
| `OPENCODE_EXPERIMENTAL_LSP_TOOL` | Enable LSP tools; defaults to the experimental umbrella when unset. |
| `OPENCODE_EXPERIMENTAL_OXFMT` | Enable oxfmt formatting; defaults to the experimental umbrella when unset. |
| `OPENCODE_EXPERIMENTAL_PLAN_MODE` | Enable plan mode; defaults to the experimental umbrella when unset. |
| `OPENCODE_EXPERIMENTAL_CODE_MODE` | Enable code mode; defaults to the experimental umbrella when unset. |
| `OPENCODE_EXPERIMENTAL_EVENT_SYSTEM` | Enable event system; defaults to the experimental umbrella when unset. |
| `OPENCODE_EXPERIMENTAL_WORKSPACES` | Enable workspaces; defaults to the experimental umbrella when unset. |
| `OPENCODE_EXPERIMENTAL_ICON_DISCOVERY` | Enable icon discovery; defaults to the experimental umbrella when unset. |
| `OPENCODE_EXPERIMENTAL_LSP_TY` | Enable the TY Python language server. |

Model inference endpoint and credential variables are owned by model-config; permission variables by agent-permissions; MCP variables by mcp; logging/trace variables by agent-logging. Model-catalog URL/path controls above concern discovery, not inference endpoints. `OPENCODE_ENABLE_QUESTION_TOOL` is omitted here because permissions owns it.

Source search also found test/build-only overrides such as `OPENCODE_TEST_HOME`, `OPENCODE_TEST_MANAGED_CONFIG_DIR`, `OPENCODE_VERSION` and `OPENCODE_WORKER_PATH`; they are not a supported general wrapper interface. The CLI documentation still lists `OPENCODE_EXPERIMENTAL_SCOUT`, but a search of 1.18.33 core/opencode runtime source found no reader; no effect is claimed for that variable.


## Machine Introspection


| Command | Format | Purpose and limits |
| --- | --- | --- |
| `opencode debug config` | json | Resolved merged config; can expose secrets and load plugin dependencies. Isolated probe succeeded. |
| `opencode debug skill` | json | Available skills including built-in entries; isolated probe succeeded. |
| `opencode debug scrap` | json | Known projects; established from JSON.stringify in source. |
| `opencode debug v2` | json | Experimental catalog/provider/default/small-model diagnostic; established from source, not executed. |
| `opencode debug agent <name>` | json | Agent configuration; --tool/--params execute a tool. Source, not executed. |
| `opencode generate` | json | OpenAPI 3.1.0 document confirmed locally; this is the HTTP API schema, not the configuration schema. |
| `opencode session list --format json` | json | Nonempty session array; no sessions yields empty stdout, confirmed locally. |
| `opencode export <sessionID> --sanitize` | json | Sanitized session JSON; supplying session ID avoids selection. Source, not executed. |
| `opencode db "select 1 as one" --format json` | json | SQL result array [{"one":1}] confirmed locally; other SQL can mutate the database. |
| `opencode debug paths` | text | Effective paths, confirmed with isolated XDG environment. |
| `opencode debug info` | text | Version/OS/terminal/plugin list; text only. |
| `opencode models` | text | One provider/model ID per line; not a JSON output contract. |
| `opencode models --verbose` | text | Model IDs mixed with pretty JSON metadata; not a single JSON document. |
| `opencode providers list` | text | Alias auth list (and ls); styled credential-provider listing, no JSON mode. |
| `opencode db path` | text | Print database path; no JSON mode. |

Local probes used a disposable directory with private HOME/XDG roots, empty inline config, disabled project discovery/model fetch, and pure mode. No paid model session or user-config write was performed. `debug config`, `debug skill`, `debug paths`, SQL JSON and `generate` succeeded; the empty session listing succeeded with empty stdout. Skills still included shipped built-in entries. Sanitized probe records are in `/tmp/opencode-cli-research/probes.json`; help records are in `/tmp/opencode-cli-research/helps.json`. Other output formats above were established from handlers, not asserted to have been run.


## Wrapper Notes


- Installed version is 1.18.33; latest GitHub release is 1.18.34. Switch observations apply to 1.18.33, not unexamined latest-version behavior.
- Use run for finite prompt execution, with stdin closed or intentional piped content. Root starts a TUI. run reads piped stdin and combines it with argv message text.
- --file/-f and --cors are greedy arrays, including equals forms. Place prompt positionals before these switches or use -- to terminate switch parsing; run includes its populated -- arguments in the message.
- Array switches accept zero values, but revision 2 cannot encode minimum zero; variadic_min is unknown with an explicit schema gap. Do not silently substitute 1.
- Short-attached strings have restricted syntax: -s123 and -m/provider are consumed, but -sABC and -mfoo group letters as switches. Prefer space or equals.
- Boolean switches also accept =true/=false, sometimes consume a following literal true/false, and have --no-* forms. The none catalog type cannot describe the optional literal-token consumption; preserve those tokens explicitly in wrapper parsing.
- No native run system-prompt flag was found in installed help, run.ts or CLI documentation. --prompt belongs to the root TUI initial user prompt.
- String and numeric options usually allow missing values syntactically; handler validation can still reject or ignore them. Stats --models is intentionally unknown because its declaration is untyped.
- Resume uses run --continue/-c or --session/-s; --fork forks before continuation. There is no separate resume command.
- run --format json emits NDJSON events; capture stderr separately, since human statuses, errors and share URLs can appear there even on successful runs.
- serve/acp are headless but long-running, so non_interactive is false under this contract’s runs-to-completion definition. web additionally opens a browser.
- Configuration/diagnostic startup can create directories, seed config, install plugin dependencies, and migrate TUI config. Use private HOME plus XDG roots, disable project discovery, supply explicit config, and disable external skill discovery for disposable probes; HOME alone does not suppress built-in skills.
- --pure disables external plugins; it does not constitute a complete filesystem/model isolation policy.
- providers/auth and plugin/plug are aliases. console and generate are hidden registered commands, despite absence from root help.
- session list --format json can return empty stdout with success for an empty database. Treat that as an empty result.
- agent create with all required settings is finite but calls an LLM and writes files. export without sessionID, db without SQL, uninstall without --force and management login flows can prompt.


## Sources


- [OpenCode homepage](https://opencode.ai)
- [Official CLI reference](https://opencode.ai/docs/cli/)
- [Official config reference](https://opencode.ai/docs/config/)
- [GitHub latest release metadata](https://api.github.com/repos/anomalyco/opencode/releases/latest)
- [Versioned install documentation](https://github.com/anomalyco/opencode/blob/v1.18.33/packages/web/src/content/docs/index.mdx)
- [Versioned main config loader](https://github.com/anomalyco/opencode/blob/v1.18.33/packages/opencode/src/config/config.ts)
- [Versioned resource path discovery](https://github.com/anomalyco/opencode/blob/v1.18.33/packages/opencode/src/config/paths.ts)
- [Versioned TUI config loader](https://github.com/anomalyco/opencode/blob/v1.18.33/packages/opencode/src/config/tui.ts)
- [Versioned managed policy loader](https://github.com/anomalyco/opencode/blob/v1.18.33/packages/opencode/src/config/managed.ts)
- [Versioned core runtime flags](https://github.com/anomalyco/opencode/blob/v1.18.33/packages/core/src/flag/flag.ts)
- [Versioned runtime flag service](https://github.com/anomalyco/opencode/blob/v1.18.33/packages/opencode/src/effect/runtime-flags.ts)
- [Versioned global XDG paths](https://github.com/anomalyco/opencode/blob/v1.18.33/packages/core/src/global.ts)
- [Versioned dependency lock](https://github.com/anomalyco/opencode/blob/v1.18.33/bun.lock)
- [parser](https://github.com/yargs/yargs-parser/blob/v22.0.0/lib/yargs-parser.ts)
- Local sanitized artifact: `/tmp/opencode-cli-research/helps.json`.
- [entry](https://github.com/anomalyco/opencode/blob/v1.18.33/packages/opencode/src/index.ts)
- [cmd-tui](https://github.com/anomalyco/opencode/blob/v1.18.33/packages/opencode/src/cli/cmd/tui.ts)
- [cmd-run](https://github.com/anomalyco/opencode/blob/v1.18.33/packages/opencode/src/cli/cmd/run.ts)
- [cmd-models](https://github.com/anomalyco/opencode/blob/v1.18.33/packages/opencode/src/cli/cmd/models.ts)
- [cmd-stats](https://github.com/anomalyco/opencode/blob/v1.18.33/packages/opencode/src/cli/cmd/stats.ts)
- [cmd-agent](https://github.com/anomalyco/opencode/blob/v1.18.33/packages/opencode/src/cli/cmd/agent.ts)
- [cmd-uninstall](https://github.com/anomalyco/opencode/blob/v1.18.33/packages/opencode/src/cli/cmd/uninstall.ts)
- [cmd-plug](https://github.com/anomalyco/opencode/blob/v1.18.33/packages/opencode/src/cli/cmd/plug.ts)
- [cmd-mcp](https://github.com/anomalyco/opencode/blob/v1.18.33/packages/opencode/src/cli/cmd/mcp.ts)
- [cmd-acp](https://github.com/anomalyco/opencode/blob/v1.18.33/packages/opencode/src/cli/cmd/acp.ts)
- [cmd-providers](https://github.com/anomalyco/opencode/blob/v1.18.33/packages/opencode/src/cli/cmd/providers.ts)
- [cmd-upgrade](https://github.com/anomalyco/opencode/blob/v1.18.33/packages/opencode/src/cli/cmd/upgrade.ts)
- [cmd-export](https://github.com/anomalyco/opencode/blob/v1.18.33/packages/opencode/src/cli/cmd/export.ts)
- [cmd-session](https://github.com/anomalyco/opencode/blob/v1.18.33/packages/opencode/src/cli/cmd/session.ts)
- [cmd-attach](https://github.com/anomalyco/opencode/blob/v1.18.33/packages/opencode/src/cli/cmd/attach.ts)
- [cmd-github](https://github.com/anomalyco/opencode/blob/v1.18.33/packages/opencode/src/cli/cmd/github.ts)
- [cmd-db](https://github.com/anomalyco/opencode/blob/v1.18.33/packages/opencode/src/cli/cmd/db.ts)
- [cmd-debug-agent](https://github.com/anomalyco/opencode/blob/v1.18.33/packages/opencode/src/cli/cmd/debug/agent.ts)
- [cmd-debug-ripgrep](https://github.com/anomalyco/opencode/blob/v1.18.33/packages/opencode/src/cli/cmd/debug/ripgrep.ts)
- [network](https://github.com/anomalyco/opencode/blob/v1.18.33/packages/opencode/src/cli/network.ts)
- [completion-parser](https://github.com/yargs/yargs/blob/v18.0.0/lib/yargs-factory.ts)
- Local sanitized artifact: `/tmp/opencode-cli-research/parser-probes.json`.
- [Versioned diagnostic command handlers](https://github.com/anomalyco/opencode/blob/v1.18.33/packages/opencode/src/cli/cmd/debug)
- [Versioned model catalog loader](https://github.com/anomalyco/opencode/blob/v1.18.33/packages/core/src/models-dev.ts)
- [Versioned database path selection](https://github.com/anomalyco/opencode/blob/v1.18.33/packages/core/src/database/database.ts)
- [Versioned skill/agent introspection handler](https://github.com/anomalyco/opencode/blob/v1.18.33/packages/opencode/src/cli/cmd/debug/agent.handler.ts)
- Local read-only directory inspection: `/Users/ken/.config/opencode`; file names and presence only.
- Local version/discovery: `opencode --version`, `sniff software agents --json`.


## Changelog


- 2026-10-01: Refreshed installed version from 1.17.13 to 1.18.33; latest released version is 1.18.34.
- 2026-10-01: Migrated research to schema revision 2 with source-backed types, exact aliases, attachment forms and command scopes.
- 2026-10-01: Expanded command inventory to nested and alias paths; console/generate remain accepted hidden commands.
- 2026-10-01: Corrected headless service classification, empty session JSON behavior, greedy array minimum zero and restricted short attachment.
- 2026-10-01: Replaced old local config snapshot and environment inventory with current read-only inspection and versioned source.