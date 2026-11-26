---
$schema: ./_schema.yaml
schema_revision: 2
provider: kilo
created: 2026-07-02
last_updated: 2026-10-01
agent: codex
model: gpt-6.1-sol
reasoning_effort: medium
latest_version: 7.8.3
versions_examined:
- 7.3.45
- 7.8.3
evidence:
- claim: yargs-parser 22 defaults and parse/eatArray/defaultValue/extendAliases establish space, equals, restricted short attachments, camel-case aliases, boolean negation, and empty arrays.
  id: parser-v22
  limitations: Library source alone does not register Kilo options; Kilo package manifests and lockfiles pin yargs 18 and yargs-parser 22.
  location: https://github.com/yargs/yargs-parser/blob/v22.0.0/lib/yargs-parser.ts
  method: source_code
  observed_on: 2026-10-01
  version: unknown
- claim: Registered command and switch declarations in src/index.ts establish spelling, declared type, aliases, and command scope.
  id: src-index
  limitations: Attachment behavior and omitted values are defined by yargs-parser; no paid or interactive handler was run.
  location: https://github.com/Kilo-Org/kilocode/blob/59f1428abb5fe782ee7bd4d258e72a08b74aadb4/packages/opencode/src/index.ts
  method: source_code
  observed_on: 2026-10-01
  version: 7.8.3
- claim: Registered command and switch declarations in src/cli/cmd/tui.ts establish spelling, declared type, aliases, and command scope.
  id: src-cli-cmd-tui
  limitations: Attachment behavior and omitted values are defined by yargs-parser; no paid or interactive handler was run.
  location: https://github.com/Kilo-Org/kilocode/blob/59f1428abb5fe782ee7bd4d258e72a08b74aadb4/packages/opencode/src/cli/cmd/tui.ts
  method: source_code
  observed_on: 2026-10-01
  version: 7.8.3
- claim: Registered command and switch declarations in src/kilocode/cli/cmd/cloud.ts establish spelling, declared type, aliases, and command scope.
  id: src-kilocode-cli-cmd-cloud
  limitations: Attachment behavior and omitted values are defined by yargs-parser; no paid or interactive handler was run.
  location: https://github.com/Kilo-Org/kilocode/blob/59f1428abb5fe782ee7bd4d258e72a08b74aadb4/packages/opencode/src/kilocode/cli/cmd/cloud.ts
  method: source_code
  observed_on: 2026-10-01
  version: 7.8.3
- claim: Registered command and switch declarations in src/kilocode/cli/cmd/console.ts establish spelling, declared type, aliases, and command scope.
  id: src-kilocode-cli-cmd-console
  limitations: Attachment behavior and omitted values are defined by yargs-parser; no paid or interactive handler was run.
  location: https://github.com/Kilo-Org/kilocode/blob/59f1428abb5fe782ee7bd4d258e72a08b74aadb4/packages/opencode/src/kilocode/cli/cmd/console.ts
  method: source_code
  observed_on: 2026-10-01
  version: 7.8.3
- claim: Registered command and switch declarations in src/kilocode/cli/cmd/daemon.ts establish spelling, declared type, aliases, and command scope.
  id: src-kilocode-cli-cmd-daemon
  limitations: Attachment behavior and omitted values are defined by yargs-parser; no paid or interactive handler was run.
  location: https://github.com/Kilo-Org/kilocode/blob/59f1428abb5fe782ee7bd4d258e72a08b74aadb4/packages/opencode/src/kilocode/cli/cmd/daemon.ts
  method: source_code
  observed_on: 2026-10-01
  version: 7.8.3
- claim: Registered command and switch declarations in src/cli/cmd/db.ts establish spelling, declared type, aliases, and command scope.
  id: src-cli-cmd-db
  limitations: Attachment behavior and omitted values are defined by yargs-parser; no paid or interactive handler was run.
  location: https://github.com/Kilo-Org/kilocode/blob/59f1428abb5fe782ee7bd4d258e72a08b74aadb4/packages/opencode/src/cli/cmd/db.ts
  method: source_code
  observed_on: 2026-10-01
  version: 7.8.3
- claim: Registered command and switch declarations in src/cli/cmd/debug/agent.ts establish spelling, declared type, aliases, and command scope.
  id: src-cli-cmd-debug-agent
  limitations: Attachment behavior and omitted values are defined by yargs-parser; no paid or interactive handler was run.
  location: https://github.com/Kilo-Org/kilocode/blob/59f1428abb5fe782ee7bd4d258e72a08b74aadb4/packages/opencode/src/cli/cmd/debug/agent.ts
  method: source_code
  observed_on: 2026-10-01
  version: 7.8.3
- claim: Registered command and switch declarations in src/cli/cmd/debug/ripgrep.ts establish spelling, declared type, aliases, and command scope.
  id: src-cli-cmd-debug-ripgrep
  limitations: Attachment behavior and omitted values are defined by yargs-parser; no paid or interactive handler was run.
  location: https://github.com/Kilo-Org/kilocode/blob/59f1428abb5fe782ee7bd4d258e72a08b74aadb4/packages/opencode/src/cli/cmd/debug/ripgrep.ts
  method: source_code
  observed_on: 2026-10-01
  version: 7.8.3
- claim: Registered command and switch declarations in src/cli/cmd/debug/ripgrep.ts establish spelling, declared type, aliases, and command scope.
  id: src-old-cli-cmd-debug-ripgrep
  limitations: Attachment behavior and omitted values are defined by yargs-parser; no paid or interactive handler was run.
  location: https://github.com/Kilo-Org/kilocode/blob/67b815466c9ab3e022f16692988673712437b881/packages/opencode/src/cli/cmd/debug/ripgrep.ts
  method: source_code
  observed_on: 2026-10-01
  version: 7.3.45
- claim: Registered command and switch declarations in src/cli/cmd/export.ts establish spelling, declared type, aliases, and command scope.
  id: src-cli-cmd-export
  limitations: Attachment behavior and omitted values are defined by yargs-parser; no paid or interactive handler was run.
  location: https://github.com/Kilo-Org/kilocode/blob/59f1428abb5fe782ee7bd4d258e72a08b74aadb4/packages/opencode/src/cli/cmd/export.ts
  method: source_code
  observed_on: 2026-10-01
  version: 7.8.3
- claim: Registered command and switch declarations in src/cli/cmd/github.ts establish spelling, declared type, aliases, and command scope.
  id: src-cli-cmd-github
  limitations: Attachment behavior and omitted values are defined by yargs-parser; no paid or interactive handler was run.
  location: https://github.com/Kilo-Org/kilocode/blob/59f1428abb5fe782ee7bd4d258e72a08b74aadb4/packages/opencode/src/cli/cmd/github.ts
  method: source_code
  observed_on: 2026-10-01
  version: 7.8.3
- claim: Registered command and switch declarations in src/kilocode/help-command.ts establish spelling, declared type, aliases, and command scope.
  id: src-kilocode-help-command
  limitations: Attachment behavior and omitted values are defined by yargs-parser; no paid or interactive handler was run.
  location: https://github.com/Kilo-Org/kilocode/blob/59f1428abb5fe782ee7bd4d258e72a08b74aadb4/packages/opencode/src/kilocode/help-command.ts
  method: source_code
  observed_on: 2026-10-01
  version: 7.8.3
- claim: Registered command and switch declarations in src/cli/cmd/mcp.ts establish spelling, declared type, aliases, and command scope.
  id: src-cli-cmd-mcp
  limitations: Attachment behavior and omitted values are defined by yargs-parser; no paid or interactive handler was run.
  location: https://github.com/Kilo-Org/kilocode/blob/59f1428abb5fe782ee7bd4d258e72a08b74aadb4/packages/opencode/src/cli/cmd/mcp.ts
  method: source_code
  observed_on: 2026-10-01
  version: 7.8.3
- claim: Registered command and switch declarations in src/cli/cmd/models.ts establish spelling, declared type, aliases, and command scope.
  id: src-cli-cmd-models
  limitations: Attachment behavior and omitted values are defined by yargs-parser; no paid or interactive handler was run.
  location: https://github.com/Kilo-Org/kilocode/blob/59f1428abb5fe782ee7bd4d258e72a08b74aadb4/packages/opencode/src/cli/cmd/models.ts
  method: source_code
  observed_on: 2026-10-01
  version: 7.8.3
- claim: Registered command and switch declarations in src/cli/cmd/plug.ts establish spelling, declared type, aliases, and command scope.
  id: src-cli-cmd-plug
  limitations: Attachment behavior and omitted values are defined by yargs-parser; no paid or interactive handler was run.
  location: https://github.com/Kilo-Org/kilocode/blob/59f1428abb5fe782ee7bd4d258e72a08b74aadb4/packages/opencode/src/cli/cmd/plug.ts
  method: source_code
  observed_on: 2026-10-01
  version: 7.8.3
- claim: Registered command and switch declarations in src/cli/cmd/pr.ts establish spelling, declared type, aliases, and command scope.
  id: src-cli-cmd-pr
  limitations: Attachment behavior and omitted values are defined by yargs-parser; no paid or interactive handler was run.
  location: https://github.com/Kilo-Org/kilocode/blob/59f1428abb5fe782ee7bd4d258e72a08b74aadb4/packages/opencode/src/cli/cmd/pr.ts
  method: source_code
  observed_on: 2026-10-01
  version: 7.8.3
- claim: Registered command and switch declarations in src/kilocode/cli/cmd/profile.ts establish spelling, declared type, aliases, and command scope.
  id: src-kilocode-cli-cmd-profile
  limitations: Attachment behavior and omitted values are defined by yargs-parser; no paid or interactive handler was run.
  location: https://github.com/Kilo-Org/kilocode/blob/59f1428abb5fe782ee7bd4d258e72a08b74aadb4/packages/opencode/src/kilocode/cli/cmd/profile.ts
  method: source_code
  observed_on: 2026-10-01
  version: 7.8.3
- claim: Registered command and switch declarations in src/kilocode/cli/cmd/roll-call.ts establish spelling, declared type, aliases, and command scope.
  id: src-kilocode-cli-cmd-roll-call
  limitations: Attachment behavior and omitted values are defined by yargs-parser; no paid or interactive handler was run.
  location: https://github.com/Kilo-Org/kilocode/blob/59f1428abb5fe782ee7bd4d258e72a08b74aadb4/packages/opencode/src/kilocode/cli/cmd/roll-call.ts
  method: source_code
  observed_on: 2026-10-01
  version: 7.8.3
- claim: Registered command and switch declarations in src/cli/cmd/run.ts establish spelling, declared type, aliases, and command scope.
  id: src-cli-cmd-run
  limitations: Attachment behavior and omitted values are defined by yargs-parser; no paid or interactive handler was run.
  location: https://github.com/Kilo-Org/kilocode/blob/59f1428abb5fe782ee7bd4d258e72a08b74aadb4/packages/opencode/src/cli/cmd/run.ts
  method: source_code
  observed_on: 2026-10-01
  version: 7.8.3
- claim: Registered command and switch declarations in src/cli/cmd/session.ts establish spelling, declared type, aliases, and command scope.
  id: src-cli-cmd-session
  limitations: Attachment behavior and omitted values are defined by yargs-parser; no paid or interactive handler was run.
  location: https://github.com/Kilo-Org/kilocode/blob/59f1428abb5fe782ee7bd4d258e72a08b74aadb4/packages/opencode/src/cli/cmd/session.ts
  method: source_code
  observed_on: 2026-10-01
  version: 7.8.3
- claim: Registered command and switch declarations in src/cli/cmd/stats.ts establish spelling, declared type, aliases, and command scope.
  id: src-cli-cmd-stats
  limitations: Attachment behavior and omitted values are defined by yargs-parser; no paid or interactive handler was run.
  location: https://github.com/Kilo-Org/kilocode/blob/59f1428abb5fe782ee7bd4d258e72a08b74aadb4/packages/opencode/src/cli/cmd/stats.ts
  method: source_code
  observed_on: 2026-10-01
  version: 7.8.3
- claim: Registered command and switch declarations in src/cli/cmd/uninstall.ts establish spelling, declared type, aliases, and command scope.
  id: src-cli-cmd-uninstall
  limitations: Attachment behavior and omitted values are defined by yargs-parser; no paid or interactive handler was run.
  location: https://github.com/Kilo-Org/kilocode/blob/59f1428abb5fe782ee7bd4d258e72a08b74aadb4/packages/opencode/src/cli/cmd/uninstall.ts
  method: source_code
  observed_on: 2026-10-01
  version: 7.8.3
- claim: Replayed the extracted run option builder with the exact yargs 18 dependency graph, confirming greedy arrays, empty arrays, scalar omission/defaults, aliases, negation, and restricted short attachment without running any provider handler.
  id: parser-replay
  limitations: Parser-only fixture; arbitrary extra positional words are not mapped by a native run command registration in this fixture. Native missing-file probes separately confirm file consumption.
  location: /tmp/kilo-cli-research-20261001/yargs-replay/results.json
  method: disposable_test
  observed_on: 2026-10-01
  version: 7.8.3
- claim: Recursive native command help confirms public spellings, aliases, command paths and displayed types for this exact released binary.
  id: help-7-3-45
  limitations: Help early exit does not establish value consumption; parser declarations and library source supply that evidence. Temporary artifact is local to this research session.
  location: /tmp/kilo-cli-research-20261001/help-7.3.45.json
  method: local_inspection
  observed_on: 2026-10-01
  version: 7.3.45
- claim: Recursive native command help confirms public spellings, aliases, command paths and displayed types for this exact released binary.
  id: help-7-8-3
  limitations: Help early exit does not establish value consumption; parser declarations and library source supply that evidence. Temporary artifact is local to this research session.
  location: /tmp/kilo-cli-research-20261001/help-7.8.3.json
  method: local_inspection
  observed_on: 2026-10-01
  version: 7.8.3
homepage: https://kilo.ai/
repo: https://github.com/Kilo-Org/kilocode
docs: https://kilo.ai/docs
cli_docs: https://kilo.ai/docs/code-with-ai/platforms/cli-reference
binaries:
- alt_binaries:
  - kilocode
  binary: kilo
  notes: npm bin mapping exposes kilo and kilocode; standalone release binary is kilo.
  os: macos
- alt_binaries:
  - kilocode
  binary: kilo
  notes: npm bin mapping exposes kilo and kilocode; standalone release binary is kilo.
  os: linux
- alt_binaries:
  - kilocode
  - kilo.cmd
  - kilocode.cmd
  - kilo.ps1
  - kilocode.ps1
  - kilo.exe
  binary: kilo
  notes: npm bin mapping installs kilo and kilocode; standalone Windows artifact supplies kilo.exe.
  os: windows
install_methods:
- command: npm install -g @kilocode/cli
  method: npm
  notes: Official package README; platform package supplies native executable.
  os: macos
- command: npx --package @kilocode/cli kilo
  method: other
  notes: Official README direct-run alternative; may download a package and prompt unless caller arranges non-interactive npm behavior.
  os: macos
- method: standalone_binary
  notes: Download and extract the matching release asset; baseline x64 builds support older CPUs without AVX.
  os: macos
- command: brew install Kilo-Org/tap/kilo
  method: brew
  notes: Official package README Homebrew alternative.
  os: macos
- command: npm install -g @kilocode/cli
  method: npm
  notes: Official package README; platform package supplies native executable.
  os: linux
- command: npx --package @kilocode/cli kilo
  method: other
  notes: Official README direct-run alternative; may download a package and prompt unless caller arranges non-interactive npm behavior.
  os: linux
- method: standalone_binary
  notes: Download and extract the matching release asset; baseline x64 builds support older CPUs without AVX.
  os: linux
- command: brew install Kilo-Org/tap/kilo
  method: brew
  notes: Official package README Homebrew alternative.
  os: linux
- command: npm install -g @kilocode/cli
  method: npm
  notes: Official package README; platform package supplies native executable.
  os: windows
- command: npx --package @kilocode/cli kilo
  method: other
  notes: Official README direct-run alternative; may download a package and prompt unless caller arranges non-interactive npm behavior.
  os: windows
- method: standalone_binary
  notes: Download and extract the matching release asset; baseline x64 builds support older CPUs without AVX.
  os: windows
subcommands:
- description: Start ACP (Agent Client Protocol) server.
  name: acp
  non_interactive: false
  notes: Protocol server remains active; it does not run to completion.
- description: Manage agents.
  name: agent
  non_interactive: false
  notes: Command group; select a child path.
- description: Create a new agent.
  name: agent create
  non_interactive: false
  notes: Finishes with explicit arguments and closed stdin; help and source examined, handler not necessarily executed.
- description: List all available agents.
  name: agent list
  non_interactive: true
  notes: Finishes with explicit arguments and closed stdin; help and source examined, handler not necessarily executed.
- description: Attach to a running kilo server.
  name: attach
  non_interactive: false
  notes: Finishes with explicit arguments and closed stdin; help and source examined, handler not necessarily executed.
- description: Manage providers and credentials.
  name: auth
  non_interactive: false
  notes: Command group; select a child path.
- description: List providers and credentials.
  name: auth list
  non_interactive: true
  notes: Finishes with explicit arguments and closed stdin; help and source examined, handler not necessarily executed.
- description: Log in to a provider.
  name: auth login
  non_interactive: false
  notes: Finishes with explicit arguments and closed stdin; help and source examined, handler not necessarily executed.
- description: Log out from a configured provider.
  name: auth logout
  non_interactive: true
  notes: 7.8.3 accepts an explicit provider to avoid selection; 7.3.45 has an interactive provider picker.
- description: List providers and credentials.
  name: auth ls
  non_interactive: true
  notes: Alias of auth list. Finishes with explicit arguments and closed stdin; help and source examined, handler not necessarily executed.
- description: Run Cloud Agent tasks.
  name: cloud
  non_interactive: false
  notes: Command group; select a child path.
- description: Show a Cloud Agent task result.
  name: cloud result
  non_interactive: true
  notes: 7.8.3 only; requires --session-id and --message-id and authenticated network access.
- description: Send a follow-up prompt to a Cloud Agent task.
  name: cloud send
  non_interactive: true
  notes: 7.8.3 only; sends a follow-up to a paid task, requiring --session-id and a prompt input. Not executed.
- description: Start a Cloud Agent task.
  name: cloud start
  non_interactive: true
  notes: 7.8.3 only; starts a paid remote task. Provide --prompt or --prompt-stdin; --stream follows JSONL events. Not executed.
- description: Show Cloud Agent task status.
  name: cloud status
  non_interactive: true
  notes: 7.8.3 only; requires --session-id and --message-id and authenticated network access.
- description: Generate shell completion script.
  name: completion
  non_interactive: true
  notes: Shell completion generator; internal --get-yargs-completions is a shell callback, not an agent session.
- description: Configuration tools.
  name: config
  non_interactive: false
  notes: Command group; select a child path.
- description: Check configuration for warnings and errors.
  name: config check
  non_interactive: true
  notes: Finishes with explicit arguments and closed stdin; help and source examined, handler not necessarily executed.
- description: Open or stop the local Kilo Console (deprecated).
  name: console
  non_interactive: false
  notes: Finishes with explicit arguments and closed stdin; help and source examined, handler not necessarily executed.
- description: Stop the daemon behind Kilo Console.
  name: console stop
  non_interactive: true
  notes: 7.8.3 only; stops the daemon behind the deprecated Console without opening a browser.
- description: Manage the local kilo daemon.
  name: daemon
  non_interactive: true
  notes: 7.8.3 defaults to starting a detached daemon and returns; --foreground/-f keeps the caller active. 7.3.45 required a child command.
- description: Restart the local kilo daemon.
  name: daemon restart
  non_interactive: true
  notes: Returns after restarting detached daemon; --foreground stays active. Mutates daemon state.
- description: Start or reuse the detached local daemon.
  name: daemon start
  non_interactive: true
  notes: Returns after starting/reusing a detached daemon; --foreground (7.8.3 alias -f) stays active. Mutates daemon state.
- description: Show local kilo daemon status.
  name: daemon status
  non_interactive: true
  notes: Reports daemon state; --json is suitable for wrappers.
- description: Stop the local kilo daemon.
  name: daemon stop
  non_interactive: true
  notes: Stops the daemon and returns; --json added in 7.8.3.
- description: Database tools.
  name: db
  non_interactive: true
  notes: Supply a nonempty SQL query; omission spawns the interactive sqlite3 shell. --format json is available.
- description: Migrate JSON data to SQLite (merges with existing data).
  name: db migrate
  non_interactive: true
  notes: 7.3.45 only; removed from the 7.8.3 registered command surface. Finishes with explicit arguments and closed stdin; help and source examined, handler not necessarily executed.
- description: Print the database path.
  name: db path
  non_interactive: true
  notes: Finishes with explicit arguments and closed stdin; help and source examined, handler not necessarily executed.
- description: Debugging and troubleshooting tools.
  name: debug
  non_interactive: false
  notes: Command group; select a child path.
- description: Show agent configuration details.
  name: debug agent
  non_interactive: true
  notes: Prints agent configuration; --tool executes a real tool using --params and may mutate files.
- description: Show resolved configuration.
  name: debug config
  non_interactive: true
  notes: Finishes with explicit arguments and closed stdin; help and source examined, handler not necessarily executed.
- description: File system debugging utilities.
  name: debug file
  non_interactive: false
  notes: Command group; select a child path.
- description: List files in a directory.
  name: debug file list
  non_interactive: true
  notes: Finishes with explicit arguments and closed stdin; help and source examined, handler not necessarily executed.
- description: Read file contents as JSON.
  name: debug file read
  non_interactive: true
  notes: Finishes with explicit arguments and closed stdin; help and source examined, handler not necessarily executed.
- description: Search files by query.
  name: debug file search
  non_interactive: true
  notes: Finishes with explicit arguments and closed stdin; help and source examined, handler not necessarily executed.
- description: Show file status information.
  name: debug file status
  non_interactive: true
  notes: 7.3.45 only; removed from the 7.8.3 registered command surface. Finishes with explicit arguments and closed stdin; help and source examined, handler not necessarily executed.
- description: Show directory tree.
  name: debug file tree
  non_interactive: true
  notes: 7.3.45 only; removed from the 7.8.3 registered command surface. Finishes with explicit arguments and closed stdin; help and source examined, handler not necessarily executed.
- description: Show debug information.
  name: debug info
  non_interactive: true
  notes: Finishes with explicit arguments and closed stdin; help and source examined, handler not necessarily executed.
- description: LSP debugging utilities.
  name: debug lsp
  non_interactive: false
  notes: Command group; select a child path.
- description: Get diagnostics for a file.
  name: debug lsp diagnostics
  non_interactive: true
  notes: Finishes with explicit arguments and closed stdin; help and source examined, handler not necessarily executed.
- description: Get symbols from a document.
  name: debug lsp document-symbols
  non_interactive: true
  notes: Finishes with explicit arguments and closed stdin; help and source examined, handler not necessarily executed.
- description: Search workspace symbols.
  name: debug lsp symbols
  non_interactive: true
  notes: Finishes with explicit arguments and closed stdin; help and source examined, handler not necessarily executed.
- description: Show global paths (data, config, cache, state).
  name: debug paths
  non_interactive: true
  notes: Finishes with explicit arguments and closed stdin; help and source examined, handler not necessarily executed.
- description: Ripgrep debugging utilities.
  name: debug rg
  non_interactive: false
  notes: Command group; select a child path.
- description: List files using ripgrep.
  name: debug rg files
  non_interactive: true
  notes: Finishes with explicit arguments and closed stdin; help and source examined, handler not necessarily executed.
- description: Search file contents using ripgrep.
  name: debug rg search
  non_interactive: true
  notes: Finishes with explicit arguments and closed stdin; help and source examined, handler not necessarily executed.
- description: Show file tree using ripgrep.
  name: debug rg tree
  non_interactive: true
  notes: 7.3.45 only; removed from the 7.8.3 registered command surface. Finishes with explicit arguments and closed stdin; help and source examined, handler not necessarily executed.
- description: List all known projects.
  name: debug scrap
  non_interactive: true
  notes: Finishes with explicit arguments and closed stdin; help and source examined, handler not necessarily executed.
- description: List all available skills.
  name: debug skill
  non_interactive: true
  notes: Finishes with explicit arguments and closed stdin; help and source examined, handler not necessarily executed.
- description: Snapshot debugging utilities.
  name: debug snapshot
  non_interactive: false
  notes: Command group; select a child path.
- description: Show diff for a snapshot hash.
  name: debug snapshot diff
  non_interactive: true
  notes: Finishes with explicit arguments and closed stdin; help and source examined, handler not necessarily executed.
- description: Show patch for a snapshot hash.
  name: debug snapshot patch
  non_interactive: true
  notes: Finishes with explicit arguments and closed stdin; help and source examined, handler not necessarily executed.
- description: Track current snapshot state.
  name: debug snapshot track
  non_interactive: true
  notes: Finishes with explicit arguments and closed stdin; help and source examined, handler not necessarily executed.
- description: Print startup timing.
  name: debug startup
  non_interactive: true
  notes: Finishes with explicit arguments and closed stdin; help and source examined, handler not necessarily executed.
- description: Debug v2 catalog and built-in plugins.
  name: debug v2
  non_interactive: true
  notes: Finishes with explicit arguments and closed stdin; help and source examined, handler not necessarily executed.
- description: Wait indefinitely (for debugging).
  name: debug wait
  non_interactive: false
  notes: Waits indefinitely (source sleeps for a day); excluded from completion-based automation.
- description: Export session data as JSON.
  name: export
  non_interactive: true
  notes: Supply sessionID; omission opens an interactive session picker when sessions exist.
- description: Emit the HTTP OpenAPI description as JSON.
  name: generate
  non_interactive: true
  notes: Registered but hidden from root help; emits HTTP OpenAPI JSON, not the CLI/configuration schema.
- description: Manage GitHub agent.
  name: github
  non_interactive: false
  notes: Command group; select a child path.
- description: Install the GitHub agent.
  name: github install
  non_interactive: false
  notes: Finishes with explicit arguments and closed stdin; help and source examined, handler not necessarily executed.
- description: Run the GitHub agent.
  name: github run
  non_interactive: true
  notes: Runs the CI GitHub agent with existing credentials/event context; may execute models and modify a checkout. Not executed.
- description: Show full CLI reference.
  name: help
  non_interactive: true
  notes: Source declares --all and --format md|text; both releases actually returned only root help in the tested full-reference invocation.
- description: Import session data from JSON file or URL.
  name: import
  non_interactive: true
  notes: Imports supplied file/share URL into session storage without a picker; writes local data.
- description: Manage MCP (Model Context Protocol) servers.
  name: mcp
  non_interactive: false
  notes: Command group; select a child path.
- description: Add an MCP server.
  name: mcp add
  non_interactive: true
  notes: 7.8.3 supports explicit name plus --url or a local command after -- without prompts; otherwise uses a setup wizard. Earlier version uses a wizard.
- description: Authenticate with an OAuth-enabled MCP server.
  name: mcp auth
  non_interactive: false
  notes: Finishes with explicit arguments and closed stdin; help and source examined, handler not necessarily executed.
- description: List OAuth-capable MCP servers and their authentication state.
  name: mcp auth list
  non_interactive: true
  notes: Text OAuth status report; does not perform the browser authorization flow.
- description: List OAuth-capable MCP servers and their authentication state.
  name: mcp auth ls
  non_interactive: true
  notes: Alias of mcp auth list. Text OAuth status report; does not perform the browser authorization flow.
- description: Debug OAuth connection for an MCP server.
  name: mcp debug
  non_interactive: false
  notes: Finishes with explicit arguments and closed stdin; help and source examined, handler not necessarily executed.
- description: List MCP servers and their status.
  name: mcp list
  non_interactive: true
  notes: Text status report; configured servers may be contacted or started.
- description: Remove OAuth credentials for an MCP server.
  name: mcp logout
  non_interactive: true
  notes: Supply a server name to avoid selection; removes OAuth credentials.
- description: List MCP servers and their status.
  name: mcp ls
  non_interactive: true
  notes: Alias of mcp list. Text status report; configured servers may be contacted or started.
- description: List all available models.
  name: models
  non_interactive: true
  notes: Finishes with explicit arguments and closed stdin; help and source examined, handler not necessarily executed.
- description: Install plugin and update config.
  name: plug
  non_interactive: true
  notes: Alias of plugin. Installs packages and edits configuration; no confirmation in the source handler. Alias plug.
- description: Install plugin and update config.
  name: plugin
  non_interactive: true
  notes: Installs packages and edits configuration; no confirmation in the source handler. Alias plug.
- description: Manage pull requests.
  name: pr
  non_interactive: false
  notes: 7.8.3 command group; 7.3.45 pr <number> checked out a branch and launched the TUI.
- description: Fetch and checkout a GitHub PR branch, then run kilo.
  name: pr checkout
  non_interactive: false
  notes: Finishes with explicit arguments and closed stdin; help and source examined, handler not necessarily executed.
- description: Link a session to a pull request.
  name: pr link
  non_interactive: true
  notes: 7.8.3 only; sets PR metadata for --session/-s or a resolved session.
- description: Show a session's linked pull request.
  name: pr status
  non_interactive: true
  notes: 7.8.3 only; reports linked PR metadata for --session/-s or a resolved session.
- description: Clear a session's linked pull request.
  name: pr unlink
  non_interactive: true
  notes: 7.8.3 only; clears PR metadata for --session/-s or a resolved session.
- description: Show Kilo account profile.
  name: profile
  non_interactive: true
  notes: --json emits JSON on success; unauthenticated state exits 1 with styled stderr.
- description: Manage providers and credentials.
  name: providers
  non_interactive: false
  notes: Alias of auth. Command group; select a child path.
- description: List providers and credentials.
  name: providers list
  non_interactive: true
  notes: Alias of auth list. Finishes with explicit arguments and closed stdin; help and source examined, handler not necessarily executed.
- description: Log in to a provider.
  name: providers login
  non_interactive: false
  notes: Alias of auth login. Finishes with explicit arguments and closed stdin; help and source examined, handler not necessarily executed.
- description: Log out from a configured provider.
  name: providers logout
  non_interactive: true
  notes: Alias of auth logout. 7.8.3 accepts an explicit provider to avoid selection; 7.3.45 has an interactive provider picker.
- description: List providers and credentials.
  name: providers ls
  non_interactive: true
  notes: Alias of auth list. Finishes with explicit arguments and closed stdin; help and source examined, handler not necessarily executed.
- description: Enable remote connection for real-time session relay.
  name: remote
  non_interactive: false
  notes: Finishes with explicit arguments and closed stdin; help and source examined, handler not necessarily executed.
- description: Batch-test text models matching a filter for connectivity and latency.
  name: roll-call
  non_interactive: true
  notes: Runs live model calls and may incur charges; not executed during this research.
- description: Run kilo with a message.
  name: run
  non_interactive: true
  notes: One-shot mode with closed stdin; --interactive/-i requires a TTY. Resume via --continue/-c or --session/-s; no native resume subcommand.
- description: Starts a headless kilo server.
  name: serve
  non_interactive: false
  notes: HTTP server remains active until terminated.
- description: Manage sessions.
  name: session
  non_interactive: false
  notes: Command group; select a child path.
- description: Delete a session.
  name: session delete
  non_interactive: true
  notes: Deletes the explicitly named session without confirmation.
- description: List sessions.
  name: session list
  non_interactive: true
  notes: Use --format json or pipe stdout to avoid the table pager; an empty result is empty stdout, not [].
- description: Show token usage and cost statistics.
  name: stats
  non_interactive: true
  notes: Finishes with explicit arguments and closed stdin; help and source examined, handler not necessarily executed.
- description: Uninstall kilo and remove all related files.
  name: uninstall
  non_interactive: true
  notes: Use --dry-run or --force to avoid the confirmation prompt. Destructive without --dry-run.
- description: Upgrade kilo to the latest or a specific version.
  name: upgrade
  non_interactive: false
  notes: Pass --method to avoid install-method selection; package managers may still prompt or require elevated privileges.
- description: Start kilo server and open web interface.
  name: web
  non_interactive: false
  notes: 7.3.45 only; removed from the 7.8.3 registered command surface. Finishes with explicit arguments and closed stdin; help and source examined, handler not necessarily executed.
- description: Manage git worktrees.
  name: worktree
  non_interactive: false
  notes: Command group; select a child path.
- description: Create (or reuse) a git worktree by name.
  name: worktree create
  non_interactive: true
  notes: 7.8.3 only; creates/reuses a named git worktree without a prompt.
- description: List git worktrees for the current project.
  name: worktree list
  non_interactive: true
  notes: 7.8.3 only; prints worktree state.
- description: Remove a named git worktree and its branch.
  name: worktree remove
  non_interactive: true
  notes: 7.8.3 only; removes the worktree and its branch without a confirmation. Not executed.
cli_switches:
- aliases:
  - -h
  - --h
  attachment: []
  description: Show help.
  evidence_ids:
  - src-index
  - parser-v22
  - help-7-8-3
  flag: --help
  invocation_scope:
  - applies_to: global
  value_type: none
- aliases:
  - -v
  - --v
  attachment: []
  description: Show version number.
  evidence_ids:
  - src-index
  - parser-v22
  - help-7-8-3
  flag: --version
  invocation_scope:
  - applies_to: global
  value_type: none
- aliases:
  - --printLogs
  attachment: []
  description: Print logs to stderr.
  evidence_ids:
  - src-index
  - parser-v22
  - help-7-8-3
  flag: --print-logs
  invocation_scope:
  - applies_to: global
  value_type: none
- aliases:
  - --logLevel
  attachment:
  - space
  - equals
  description: Set log level to DEBUG, INFO, WARN, or ERROR.
  evidence_ids:
  - src-index
  - parser-v22
  - help-7-8-3
  flag: --log-level
  invocation_scope:
  - applies_to: global
  value_optional: false
  value_type: string
- attachment: []
  description: Run without external plugins.
  evidence_ids:
  - src-index
  - parser-v22
  - help-7-8-3
  flag: --pure
  invocation_scope:
  - applies_to: global
  value_type: none
- attachment:
  - space
  - equals
  description: port to listen on
  evidence_ids:
  - src-cli-cmd-tui
  - parser-v22
  - src-kilocode-cli-cmd-daemon
  - help-7-8-3
  flag: --port
  invocation_scope:
  - applies_to: command
    command: []
  - applies_to: command
    command:
    - completion
  - applies_to: command
    command:
    - daemon
  - applies_to: command
    command:
    - daemon
    - restart
  - applies_to: command
    command:
    - daemon
    - start
  notes: Parser permits omission unless a choice/check rejects the empty or undefined value; this does not make the command semantically valid.
  value_optional: true
  value_type: number
- attachment:
  - space
  - equals
  description: hostname to listen on
  evidence_ids:
  - src-cli-cmd-tui
  - parser-v22
  - src-kilocode-cli-cmd-daemon
  - help-7-8-3
  flag: --hostname
  invocation_scope:
  - applies_to: command
    command: []
  - applies_to: command
    command:
    - completion
  - applies_to: command
    command:
    - daemon
  - applies_to: command
    command:
    - daemon
    - restart
  - applies_to: command
    command:
    - daemon
    - start
  notes: Parser permits omission unless a choice/check rejects the empty or undefined value; this does not make the command semantically valid.
  value_optional: true
  value_type: string
- attachment: []
  description: enable mDNS service discovery (defaults hostname to 0.0.0.0)
  evidence_ids:
  - src-cli-cmd-tui
  - parser-v22
  - src-kilocode-cli-cmd-daemon
  - help-7-8-3
  flag: --mdns
  invocation_scope:
  - applies_to: command
    command: []
  - applies_to: command
    command:
    - completion
  - applies_to: command
    command:
    - daemon
  - applies_to: command
    command:
    - daemon
    - restart
  - applies_to: command
    command:
    - daemon
    - start
  value_type: none
- aliases:
  - --mdnsDomain
  attachment:
  - space
  - equals
  description: 'custom domain name for mDNS service (default: kilo.local)'
  evidence_ids:
  - src-cli-cmd-tui
  - parser-v22
  - src-kilocode-cli-cmd-daemon
  - help-7-8-3
  flag: --mdns-domain
  invocation_scope:
  - applies_to: command
    command: []
  - applies_to: command
    command:
    - completion
  - applies_to: command
    command:
    - daemon
  - applies_to: command
    command:
    - daemon
    - restart
  - applies_to: command
    command:
    - daemon
    - start
  notes: Parser permits omission unless a choice/check rejects the empty or undefined value; this does not make the command semantically valid.
  value_optional: true
  value_type: string
- attachment:
  - space
  - equals
  description: additional domains to allow for CORS
  evidence_ids:
  - src-cli-cmd-tui
  - parser-v22
  - src-kilocode-cli-cmd-daemon
  - help-7-8-3
  flag: --cors
  gap: Source eatArray and a parser replay establish a minimum of zero (bare --cors produces []), but revision 2 allows only integers >=1. Recheck the same declaration and zero-value parse after the contract can represent zero; do not assume a minimum of one.
  invocation_scope:
  - applies_to: command
    command: []
  - applies_to: command
    command:
    - completion
  - applies_to: command
    command:
    - daemon
  - applies_to: command
    command:
    - daemon
    - restart
  - applies_to: command
    command:
    - daemon
    - start
  notes: Greedily consumes following non-option words even after --name=value; use -- before the prompt. Repeated occurrences concatenate arrays.
  value_type: variadic
  variadic_min: unknown
- aliases:
  - -m
  - --m
  attachment:
  - space
  - equals
  - short_attached
  description: model to use in the format of provider/model
  evidence_ids:
  - src-cli-cmd-tui
  - parser-v22
  - src-cli-cmd-run
  - parser-replay
  - help-7-8-3
  flag: --model
  invocation_scope:
  - applies_to: command
    command: []
  - applies_to: command
    command:
    - completion
  - applies_to: command
    command:
    - run
  notes: Parser permits omission unless a choice/check rejects the empty or undefined value; this does not make the command semantically valid. Short attachment works for numeric or punctuation-leading values (for example -m123 or -f/path); alphabetic -mfoo is parsed as a short-option group and is not a general value form. Use -m foo or -m=foo.
  value_optional: true
  value_type: string
- aliases:
  - -c
  - --c
  attachment: []
  description: continue the last session
  evidence_ids:
  - src-cli-cmd-tui
  - parser-v22
  - src-cli-cmd-run
  - parser-replay
  - help-7-8-3
  flag: --continue
  invocation_scope:
  - applies_to: command
    command: []
  - applies_to: command
    command:
    - completion
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
  - src-cli-cmd-tui
  - parser-v22
  - src-cli-cmd-run
  - parser-replay
  - help-7-8-3
  flag: --session
  invocation_scope:
  - applies_to: command
    command: []
  - applies_to: command
    command:
    - completion
  - applies_to: command
    command:
    - run
  notes: Parser permits omission unless a choice/check rejects the empty or undefined value; this does not make the command semantically valid. Short attachment works for numeric or punctuation-leading values (for example -m123 or -f/path); alphabetic -mfoo is parsed as a short-option group and is not a general value form. Use -m foo or -m=foo.
  value_optional: true
  value_type: string
- attachment: []
  description: fork the session when continuing (use with --continue or --session)
  evidence_ids:
  - src-cli-cmd-tui
  - parser-v22
  - help-7-8-3
  flag: --fork
  invocation_scope:
  - applies_to: command
    command: []
  - applies_to: command
    command:
    - completion
  value_type: none
- aliases:
  - --cloudFork
  attachment: []
  description: fetch session from cloud and continue locally (use with --session)
  evidence_ids:
  - src-cli-cmd-tui
  - parser-v22
  - src-cli-cmd-run
  - parser-replay
  - help-7-8-3
  flag: --cloud-fork
  invocation_scope:
  - applies_to: command
    command: []
  - applies_to: command
    command:
    - completion
  - applies_to: command
    command:
    - run
  value_type: none
- attachment:
  - space
  - equals
  description: create (or reuse) a git worktree with this name and start kilo there
  evidence_ids:
  - src-cli-cmd-tui
  - parser-v22
  - help-7-8-3
  flag: --worktree
  invocation_scope:
  - applies_to: command
    command: []
  - applies_to: command
    command:
    - completion
  notes: Parser permits omission unless a choice/check rejects the empty or undefined value; this does not make the command semantically valid.
  value_optional: true
  value_type: string
- attachment:
  - space
  - equals
  description: prompt to use
  evidence_ids:
  - src-cli-cmd-tui
  - parser-v22
  - help-7-8-3
  flag: --prompt
  invocation_scope:
  - applies_to: command
    command: []
  - applies_to: command
    command:
    - completion
  notes: Parser permits omission unless a choice/check rejects the empty or undefined value; this does not make the command semantically valid.
  value_optional: true
  value_type: string
- attachment:
  - space
  - equals
  description: agent to use
  evidence_ids:
  - src-cli-cmd-tui
  - parser-v22
  - src-cli-cmd-run
  - parser-replay
  - help-7-8-3
  flag: --agent
  invocation_scope:
  - applies_to: command
    command: []
  - applies_to: command
    command:
    - completion
  - applies_to: command
    command:
    - run
  notes: Parser permits omission unless a choice/check rejects the empty or undefined value; this does not make the command semantically valid.
  value_optional: true
  value_type: string
- attachment: []
  description: auto-approve permissions that are not explicitly denied (dangerous!)
  evidence_ids:
  - src-cli-cmd-tui
  - parser-v22
  - src-cli-cmd-run
  - parser-replay
  - help-7-8-3
  flag: --auto
  invocation_scope:
  - applies_to: command
    command: []
  - applies_to: command
    command:
    - completion
  - applies_to: command
    command:
    - run
  value_type: none
- attachment: []
  description: start the minimal interactive interface
  evidence_ids:
  - src-cli-cmd-tui
  - parser-v22
  - help-7-8-3
  flag: --mini
  invocation_scope:
  - applies_to: command
    command: []
  - applies_to: command
    command:
    - completion
  notes: Run scope retains compatibility switches; mini/replay settings may be rejected by the handler outside interactive mode.
  value_type: none
- aliases:
  - --noReplay
  attachment: []
  description: disable mini session history replay on resume and after resize
  evidence_ids:
  - src-cli-cmd-tui
  - parser-v22
  - help-7-8-3
  flag: --no-replay
  invocation_scope:
  - applies_to: command
    command: []
  notes: Run scope retains compatibility switches; mini/replay settings may be rejected by the handler outside interactive mode.
  value_type: none
- aliases:
  - --replayLimit
  attachment:
  - space
  - equals
  description: cap visible mini replay to the newest N messages
  evidence_ids:
  - src-cli-cmd-tui
  - parser-v22
  - help-7-8-3
  flag: --replay-limit
  invocation_scope:
  - applies_to: command
    command: []
  - applies_to: command
    command:
    - completion
  notes: Parser permits omission unless a choice/check rejects the empty or undefined value; this does not make the command semantically valid. Run scope retains compatibility switches; mini/replay settings may be rejected by the handler outside interactive mode.
  value_optional: true
  value_type: number
- attachment: []
  description: Compatibility permission bypass switch.
  evidence_ids:
  - src-cli-cmd-tui
  - parser-v22
  - src-cli-cmd-run
  - parser-replay
  - help-7-8-3
  flag: --yolo
  invocation_scope:
  - applies_to: command
    command: []
  - applies_to: command
    command:
    - run
  notes: Hidden but accepted by the 7.8.3 source parser.
  value_type: none
- aliases:
  - --dangerouslySkipPermissions
  attachment: []
  description: Compatibility permission bypass switch.
  evidence_ids:
  - src-cli-cmd-tui
  - parser-v22
  - src-cli-cmd-run
  - parser-replay
  - help-7-8-3
  flag: --dangerously-skip-permissions
  invocation_scope:
  - applies_to: command
    command: []
  - applies_to: command
    command:
    - run
  notes: Hidden but accepted by the 7.8.3 source parser.
  value_type: none
- attachment: []
  description: Control interactive history replay.
  evidence_ids:
  - src-cli-cmd-tui
  - parser-v22
  - src-cli-cmd-run
  - parser-replay
  - help-7-8-3
  flag: --replay
  invocation_scope:
  - applies_to: command
    command: []
  - applies_to: command
    command:
    - run
  notes: Hidden but accepted by the 7.8.3 source parser. Run scope retains compatibility switches; mini/replay settings may be rejected by the handler outside interactive mode.
  value_type: none
- attachment: []
  description: Enable direct interactive demo slash commands.
  evidence_ids:
  - src-cli-cmd-tui
  - parser-v22
  - src-cli-cmd-run
  - parser-replay
  - help-7-8-3
  flag: --demo
  invocation_scope:
  - applies_to: command
    command: []
  - applies_to: command
    command:
    - run
  notes: Hidden but accepted by the 7.8.3 source parser.
  value_type: none
- aliases:
  - --sessionId
  attachment:
  - space
  - equals
  description: Cloud Agent session ID
  evidence_ids:
  - src-kilocode-cli-cmd-cloud
  - parser-v22
  - help-7-8-3
  flag: --session-id
  invocation_scope:
  - applies_to: command
    command:
    - cloud
    - result
  - applies_to: command
    command:
    - cloud
    - send
  - applies_to: command
    command:
    - cloud
    - status
  notes: Parser permits omission unless a choice/check rejects the empty or undefined value; this does not make the command semantically valid.
  value_optional: true
  value_type: string
- aliases:
  - --messageId
  attachment:
  - space
  - equals
  description: Cloud Agent message ID
  evidence_ids:
  - src-kilocode-cli-cmd-cloud
  - parser-v22
  - help-7-8-3
  flag: --message-id
  invocation_scope:
  - applies_to: command
    command:
    - cloud
    - result
  - applies_to: command
    command:
    - cloud
    - status
  notes: Parser permits omission unless a choice/check rejects the empty or undefined value; this does not make the command semantically valid.
  value_optional: true
  value_type: string
- attachment:
  - space
  - equals
  description: prompt for the Cloud Agent
  evidence_ids:
  - src-kilocode-cli-cmd-cloud
  - parser-v22
  - help-7-8-3
  flag: --prompt
  invocation_scope:
  - applies_to: command
    command:
    - cloud
    - send
  - applies_to: command
    command:
    - cloud
    - start
  notes: Parser permits omission unless a choice/check rejects the empty or undefined value; this does not make the command semantically valid.
  value_optional: true
  value_type: string
- aliases:
  - --promptStdin
  attachment: []
  description: read the prompt from standard input
  evidence_ids:
  - src-kilocode-cli-cmd-cloud
  - parser-v22
  - help-7-8-3
  flag: --prompt-stdin
  invocation_scope:
  - applies_to: command
    command:
    - cloud
    - send
  - applies_to: command
    command:
    - cloud
    - start
  value_type: none
- attachment:
  - space
  - equals
  description: repository shorthand or URL
  evidence_ids:
  - src-kilocode-cli-cmd-cloud
  - parser-v22
  - help-7-8-3
  flag: --repo
  invocation_scope:
  - applies_to: command
    command:
    - cloud
    - start
  notes: Parser permits omission unless a choice/check rejects the empty or undefined value; this does not make the command semantically valid.
  value_optional: true
  value_type: string
- aliases:
  - --repoType
  attachment:
  - space
  - equals
  description: repository provider type
  evidence_ids:
  - src-kilocode-cli-cmd-cloud
  - parser-v22
  - help-7-8-3
  flag: --repo-type
  invocation_scope:
  - applies_to: command
    command:
    - cloud
    - start
  notes: Parser permits omission unless a choice/check rejects the empty or undefined value; this does not make the command semantically valid.
  value_optional: false
  value_type: string
- attachment:
  - space
  - equals
  description: repository branch
  evidence_ids:
  - src-kilocode-cli-cmd-cloud
  - parser-v22
  - help-7-8-3
  flag: --branch
  invocation_scope:
  - applies_to: command
    command:
    - cloud
    - start
  notes: Parser permits omission unless a choice/check rejects the empty or undefined value; this does not make the command semantically valid.
  value_optional: true
  value_type: string
- attachment:
  - space
  - equals
  description: Cloud Agent model
  evidence_ids:
  - src-kilocode-cli-cmd-cloud
  - parser-v22
  - help-7-8-3
  flag: --model
  invocation_scope:
  - applies_to: command
    command:
    - cloud
    - start
  notes: Parser permits omission unless a choice/check rejects the empty or undefined value; this does not make the command semantically valid.
  value_optional: true
  value_type: string
- attachment:
  - space
  - equals
  description: Cloud Agent mode
  evidence_ids:
  - src-kilocode-cli-cmd-cloud
  - parser-v22
  - help-7-8-3
  flag: --mode
  invocation_scope:
  - applies_to: command
    command:
    - cloud
    - start
  notes: Parser permits omission unless a choice/check rejects the empty or undefined value; this does not make the command semantically valid.
  value_optional: true
  value_type: string
- aliases:
  - --orgId
  attachment:
  - space
  - equals
  description: Kilo organization ID
  evidence_ids:
  - src-kilocode-cli-cmd-cloud
  - parser-v22
  - help-7-8-3
  flag: --org-id
  invocation_scope:
  - applies_to: command
    command:
    - cloud
    - start
  notes: Parser permits omission unless a choice/check rejects the empty or undefined value; this does not make the command semantically valid.
  value_optional: true
  value_type: string
- attachment: []
  description: connect to the WebSocket stream and print events as JSONL
  evidence_ids:
  - src-kilocode-cli-cmd-cloud
  - parser-v22
  - help-7-8-3
  flag: --stream
  invocation_scope:
  - applies_to: command
    command:
    - cloud
    - start
  value_type: none
- attachment: []
  description: disable mini session history replay on resume and after resize
  evidence_ids:
  - src-cli-cmd-tui
  - parser-v22
  - help-7-8-3
  flag: --no-replay
  invocation_scope:
  - applies_to: command
    command:
    - completion
  notes: Run scope retains compatibility switches; mini/replay settings may be rejected by the handler outside interactive mode.
  value_type: none
- attachment: []
  description: print daemon details as JSON
  evidence_ids:
  - src-kilocode-cli-cmd-console
  - parser-v22
  - src-kilocode-cli-cmd-daemon
  - help-7-8-3
  flag: --json
  invocation_scope:
  - applies_to: command
    command:
    - console
    - stop
  - applies_to: command
    command:
    - daemon
  - applies_to: command
    command:
    - daemon
    - restart
  - applies_to: command
    command:
    - daemon
    - start
  - applies_to: command
    command:
    - daemon
    - status
  - applies_to: command
    command:
    - daemon
    - stop
  value_type: none
- aliases:
  - -f
  - --f
  attachment: []
  description: keep the command active until interrupted
  evidence_ids:
  - src-kilocode-cli-cmd-daemon
  - parser-v22
  - help-7-8-3
  flag: --foreground
  invocation_scope:
  - applies_to: command
    command:
    - daemon
  - applies_to: command
    command:
    - daemon
    - restart
  - applies_to: command
    command:
    - daemon
    - start
  value_type: none
- attachment:
  - space
  - equals
  description: Output format
  evidence_ids:
  - src-cli-cmd-db
  - parser-v22
  - help-7-8-3
  flag: --format
  invocation_scope:
  - applies_to: command
    command:
    - db
  notes: Parser permits omission unless a choice/check rejects the empty or undefined value; this does not make the command semantically valid.
  value_optional: true
  value_type: string
- attachment:
  - space
  - equals
  description: Tool id to execute
  evidence_ids:
  - src-cli-cmd-debug-agent
  - parser-v22
  - help-7-8-3
  flag: --tool
  invocation_scope:
  - applies_to: command
    command:
    - debug
    - agent
  notes: Parser permits omission unless a choice/check rejects the empty or undefined value; this does not make the command semantically valid.
  value_optional: true
  value_type: string
- attachment:
  - space
  - equals
  description: Tool params as JSON or a JS object literal
  evidence_ids:
  - src-cli-cmd-debug-agent
  - parser-v22
  - help-7-8-3
  flag: --params
  invocation_scope:
  - applies_to: command
    command:
    - debug
    - agent
  notes: Parser permits omission unless a choice/check rejects the empty or undefined value; this does not make the command semantically valid.
  value_optional: true
  value_type: string
- attachment:
  - space
  - equals
  description: Filter files by query
  evidence_ids:
  - src-cli-cmd-debug-ripgrep
  - parser-v22
  - help-7-8-3
  flag: --query
  invocation_scope:
  - applies_to: command
    command:
    - debug
    - rg
    - files
  notes: Parser permits omission unless a choice/check rejects the empty or undefined value; this does not make the command semantically valid.
  value_optional: true
  value_type: string
- attachment:
  - space
  - equals
  description: Glob pattern to match files
  evidence_ids:
  - src-cli-cmd-debug-ripgrep
  - parser-v22
  - help-7-8-3
  flag: --glob
  invocation_scope:
  - applies_to: command
    command:
    - debug
    - rg
    - files
  notes: Parser permits omission unless a choice/check rejects the empty or undefined value; this does not make the command semantically valid.
  value_optional: true
  value_type: string
- attachment:
  - space
  - equals
  description: Limit number of results
  evidence_ids:
  - src-cli-cmd-debug-ripgrep
  - parser-v22
  - help-7-8-3
  flag: --limit
  invocation_scope:
  - applies_to: command
    command:
    - debug
    - rg
    - files
  - applies_to: command
    command:
    - debug
    - rg
    - search
  notes: Parser permits omission unless a choice/check rejects the empty or undefined value; this does not make the command semantically valid.
  value_optional: true
  value_type: number
- attachment:
  - space
  - equals
  description: File glob patterns
  evidence_ids:
  - src-cli-cmd-debug-ripgrep
  - parser-v22
  - help-7-8-3
  flag: --glob
  gap: Source eatArray and a parser replay establish a minimum of zero (bare --glob produces []), but revision 2 allows only integers >=1. Recheck the same declaration and zero-value parse after the contract can represent zero; do not assume a minimum of one.
  invocation_scope:
  - applies_to: command
    command:
    - debug
    - rg
    - search
  notes: Greedily consumes following non-option words even after --name=value; use -- before the prompt. Repeated occurrences concatenate arrays.
  value_type: variadic
  variadic_min: unknown
- attachment:
  - space
  - equals
  description: Provider CLI switch.
  evidence_ids:
  - src-old-cli-cmd-debug-ripgrep
  - parser-v22
  - help-7-3-45
  flag: --limit
  invocation_scope:
  - applies_to: command
    command:
    - debug
    - rg
    - tree
  notes: 7.3.45-specific command; not registered in 7.8.3. Parser permits omission unless a choice/check rejects the empty or undefined value; this does not make the command semantically valid.
  value_optional: true
  value_type: number
- attachment: []
  description: redact sensitive transcript and file data
  evidence_ids:
  - src-cli-cmd-export
  - parser-v22
  - help-7-8-3
  flag: --sanitize
  invocation_scope:
  - applies_to: command
    command:
    - export
  value_type: none
- attachment:
  - space
  - equals
  description: GitHub mock event to run the agent for
  evidence_ids:
  - src-cli-cmd-github
  - parser-v22
  - help-7-8-3
  flag: --event
  invocation_scope:
  - applies_to: command
    command:
    - github
    - run
  notes: Parser permits omission unless a choice/check rejects the empty or undefined value; this does not make the command semantically valid.
  value_optional: true
  value_type: string
- attachment:
  - space
  - equals
  description: GitHub personal access token (github_pat_********)
  evidence_ids:
  - src-cli-cmd-github
  - parser-v22
  - help-7-8-3
  flag: --token
  invocation_scope:
  - applies_to: command
    command:
    - github
    - run
  notes: Parser permits omission unless a choice/check rejects the empty or undefined value; this does not make the command semantically valid.
  value_optional: true
  value_type: string
- attachment: []
  description: Show help for all commands.
  evidence_ids:
  - src-kilocode-help-command
  - parser-v22
  - help-7-8-3
  flag: --all
  invocation_scope:
  - applies_to: command
    command:
    - help
  value_type: none
- attachment:
  - space
  - equals
  description: Select help output format md or text.
  evidence_ids:
  - src-kilocode-help-command
  - parser-v22
  - help-7-8-3
  flag: --format
  invocation_scope:
  - applies_to: command
    command:
    - help
  notes: Parser permits omission unless a choice/check rejects the empty or undefined value; this does not make the command semantically valid.
  value_optional: true
  value_type: string
- attachment:
  - space
  - equals
  description: URL for a remote MCP server
  evidence_ids:
  - src-cli-cmd-mcp
  - parser-v22
  - help-7-8-3
  flag: --url
  invocation_scope:
  - applies_to: command
    command:
    - mcp
    - add
  notes: Parser permits omission unless a choice/check rejects the empty or undefined value; this does not make the command semantically valid.
  value_optional: true
  value_type: string
- attachment:
  - space
  - equals
  description: environment variable for a local MCP server (KEY=VALUE)
  evidence_ids:
  - src-cli-cmd-mcp
  - parser-v22
  - help-7-8-3
  flag: --env
  gap: Source eatArray and a parser replay establish a minimum of zero (bare --env produces []), but revision 2 allows only integers >=1. Recheck the same declaration and zero-value parse after the contract can represent zero; do not assume a minimum of one.
  invocation_scope:
  - applies_to: command
    command:
    - mcp
    - add
  notes: Greedily consumes following non-option words even after --name=value; use -- before the prompt. Repeated occurrences concatenate arrays.
  value_type: variadic
  variadic_min: unknown
- attachment:
  - space
  - equals
  description: HTTP header for a remote MCP server (KEY=VALUE)
  evidence_ids:
  - src-cli-cmd-mcp
  - parser-v22
  - help-7-8-3
  flag: --header
  gap: Source eatArray and a parser replay establish a minimum of zero (bare --header produces []), but revision 2 allows only integers >=1. Recheck the same declaration and zero-value parse after the contract can represent zero; do not assume a minimum of one.
  invocation_scope:
  - applies_to: command
    command:
    - mcp
    - add
  notes: Greedily consumes following non-option words even after --name=value; use -- before the prompt. Repeated occurrences concatenate arrays.
  value_type: variadic
  variadic_min: unknown
- attachment: []
  description: use more verbose model output (includes metadata like costs)
  evidence_ids:
  - src-cli-cmd-models
  - parser-v22
  - help-7-8-3
  flag: --verbose
  invocation_scope:
  - applies_to: command
    command:
    - models
  value_type: none
- attachment: []
  description: refresh the models cache from models.dev
  evidence_ids:
  - src-cli-cmd-models
  - parser-v22
  - help-7-8-3
  flag: --refresh
  invocation_scope:
  - applies_to: command
    command:
    - models
  value_type: none
- aliases:
  - -g
  - --g
  attachment: []
  description: install in global config
  evidence_ids:
  - src-cli-cmd-plug
  - parser-v22
  - help-7-8-3
  flag: --global
  invocation_scope:
  - applies_to: command
    command:
    - plug
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
  - src-cli-cmd-plug
  - parser-v22
  - help-7-8-3
  flag: --force
  invocation_scope:
  - applies_to: command
    command:
    - plug
  - applies_to: command
    command:
    - plugin
  value_type: none
- aliases:
  - -s
  - --s
  attachment:
  - space
  - equals
  - short_attached
  description: session id to apply the PR link to
  evidence_ids:
  - src-cli-cmd-pr
  - parser-v22
  - help-7-8-3
  flag: --session
  invocation_scope:
  - applies_to: command
    command:
    - pr
    - link
  - applies_to: command
    command:
    - pr
    - status
  - applies_to: command
    command:
    - pr
    - unlink
  notes: Parser permits omission unless a choice/check rejects the empty or undefined value; this does not make the command semantically valid. Short attachment works for numeric or punctuation-leading values (for example -m123 or -f/path); alphabetic -mfoo is parsed as a short-option group and is not a general value form. Use -m foo or -m=foo.
  value_optional: true
  value_type: string
- attachment: []
  description: output profile as JSON
  evidence_ids:
  - src-kilocode-cli-cmd-profile
  - parser-v22
  - help-7-8-3
  flag: --json
  invocation_scope:
  - applies_to: command
    command:
    - profile
  value_type: none
- attachment:
  - space
  - equals
  description: Prompt to send to each model
  evidence_ids:
  - src-kilocode-cli-cmd-roll-call
  - parser-v22
  - help-7-8-3
  flag: --prompt
  invocation_scope:
  - applies_to: command
    command:
    - roll-call
  notes: Parser permits omission unless a choice/check rejects the empty or undefined value; this does not make the command semantically valid.
  value_optional: true
  value_type: string
- attachment:
  - space
  - equals
  description: Timeout for each model call in milliseconds
  evidence_ids:
  - src-kilocode-cli-cmd-roll-call
  - parser-v22
  - help-7-8-3
  flag: --timeout
  invocation_scope:
  - applies_to: command
    command:
    - roll-call
  notes: Parser permits omission unless a choice/check rejects the empty or undefined value; this does not make the command semantically valid.
  value_optional: true
  value_type: number
- attachment:
  - space
  - equals
  description: Number of parallel model calls
  evidence_ids:
  - src-kilocode-cli-cmd-roll-call
  - parser-v22
  - help-7-8-3
  flag: --parallel
  invocation_scope:
  - applies_to: command
    command:
    - roll-call
  notes: Parser permits omission unless a choice/check rejects the empty or undefined value; this does not make the command semantically valid.
  value_optional: true
  value_type: number
- attachment: []
  description: Show verbose output
  evidence_ids:
  - src-kilocode-cli-cmd-roll-call
  - parser-v22
  - help-7-8-3
  flag: --verbose
  invocation_scope:
  - applies_to: command
    command:
    - roll-call
  value_type: none
- attachment: []
  description: Suppress progress and decoration
  evidence_ids:
  - src-kilocode-cli-cmd-roll-call
  - parser-v22
  - help-7-8-3
  flag: --quiet
  invocation_scope:
  - applies_to: command
    command:
    - roll-call
  value_type: none
- attachment:
  - space
  - equals
  description: Output format (table, json, or md)
  evidence_ids:
  - src-kilocode-cli-cmd-roll-call
  - parser-v22
  - help-7-8-3
  flag: --output
  invocation_scope:
  - applies_to: command
    command:
    - roll-call
  notes: Parser permits omission unless a choice/check rejects the empty or undefined value; this does not make the command semantically valid.
  value_optional: true
  value_type: string
- attachment:
  - space
  - equals
  description: the command to run, use message for args
  evidence_ids:
  - src-cli-cmd-run
  - parser-v22
  - parser-replay
  - help-7-8-3
  flag: --command
  invocation_scope:
  - applies_to: command
    command:
    - run
  notes: Parser permits omission unless a choice/check rejects the empty or undefined value; this does not make the command semantically valid.
  value_optional: true
  value_type: string
- attachment: []
  description: fork the session before continuing (requires --continue or --session)
  evidence_ids:
  - src-cli-cmd-run
  - parser-v22
  - parser-replay
  - help-7-8-3
  flag: --fork
  invocation_scope:
  - applies_to: command
    command:
    - run
  value_type: none
- attachment: []
  description: share the session
  evidence_ids:
  - src-cli-cmd-run
  - parser-v22
  - parser-replay
  - help-7-8-3
  flag: --share
  invocation_scope:
  - applies_to: command
    command:
    - run
  value_type: none
- attachment:
  - space
  - equals
  description: 'format: default (formatted) or json (raw JSON events)'
  evidence_ids:
  - src-cli-cmd-run
  - parser-v22
  - parser-replay
  - help-7-8-3
  flag: --format
  invocation_scope:
  - applies_to: command
    command:
    - run
  notes: Parser permits omission unless a choice/check rejects the empty or undefined value; this does not make the command semantically valid.
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
  - src-cli-cmd-run
  - parser-v22
  - parser-replay
  - help-7-8-3
  flag: --file
  gap: Source eatArray and a parser replay establish a minimum of zero (bare --file produces []), but revision 2 allows only integers >=1. Recheck the same declaration and zero-value parse after the contract can represent zero; do not assume a minimum of one.
  invocation_scope:
  - applies_to: command
    command:
    - run
  notes: Short attachment works for numeric or punctuation-leading values (for example -m123 or -f/path); alphabetic -mfoo is parsed as a short-option group and is not a general value form. Use -m foo or -m=foo. Greedily consumes following non-option words even after --name=value; use -- before the prompt. Repeated occurrences concatenate arrays.
  value_type: variadic
  variadic_min: unknown
- attachment:
  - space
  - equals
  description: title for the session (uses truncated prompt if no value provided)
  evidence_ids:
  - src-cli-cmd-run
  - parser-v22
  - parser-replay
  - help-7-8-3
  flag: --title
  invocation_scope:
  - applies_to: command
    command:
    - run
  notes: Parser permits omission unless a choice/check rejects the empty or undefined value; this does not make the command semantically valid.
  value_optional: true
  value_type: string
- attachment:
  - space
  - equals
  description: attach to a running kilo server (e.g., http://localhost:4096)
  evidence_ids:
  - src-cli-cmd-run
  - parser-v22
  - parser-replay
  - help-7-8-3
  flag: --attach
  invocation_scope:
  - applies_to: command
    command:
    - run
  notes: Parser permits omission unless a choice/check rejects the empty or undefined value; this does not make the command semantically valid.
  value_optional: true
  value_type: string
- aliases:
  - -p
  - --p
  attachment:
  - space
  - equals
  - short_attached
  description: basic auth password (defaults to KILO_SERVER_PASSWORD)
  evidence_ids:
  - src-cli-cmd-run
  - parser-v22
  - parser-replay
  - help-7-8-3
  flag: --password
  invocation_scope:
  - applies_to: command
    command:
    - run
  notes: Parser permits omission unless a choice/check rejects the empty or undefined value; this does not make the command semantically valid. Short attachment works for numeric or punctuation-leading values (for example -m123 or -f/path); alphabetic -mfoo is parsed as a short-option group and is not a general value form. Use -m foo or -m=foo.
  value_optional: true
  value_type: string
- aliases:
  - -u
  - --u
  attachment:
  - space
  - equals
  - short_attached
  description: basic auth username (defaults to KILO_SERVER_USERNAME or 'kilo')
  evidence_ids:
  - src-cli-cmd-run
  - parser-v22
  - parser-replay
  - help-7-8-3
  flag: --username
  invocation_scope:
  - applies_to: command
    command:
    - run
  notes: Parser permits omission unless a choice/check rejects the empty or undefined value; this does not make the command semantically valid. Short attachment works for numeric or punctuation-leading values (for example -m123 or -f/path); alphabetic -mfoo is parsed as a short-option group and is not a general value form. Use -m foo or -m=foo.
  value_optional: true
  value_type: string
- attachment:
  - space
  - equals
  description: directory to run in, path on remote server if attaching
  evidence_ids:
  - src-cli-cmd-run
  - parser-v22
  - parser-replay
  - help-7-8-3
  flag: --dir
  invocation_scope:
  - applies_to: command
    command:
    - run
  notes: Parser permits omission unless a choice/check rejects the empty or undefined value; this does not make the command semantically valid.
  value_optional: true
  value_type: string
- attachment:
  - space
  - equals
  description: port for the local server (defaults to random port if no value provided)
  evidence_ids:
  - src-cli-cmd-run
  - parser-v22
  - parser-replay
  - help-7-8-3
  flag: --port
  invocation_scope:
  - applies_to: command
    command:
    - run
  notes: Parser permits omission unless a choice/check rejects the empty or undefined value; this does not make the command semantically valid.
  value_optional: true
  value_type: number
- attachment:
  - space
  - equals
  description: model variant (provider-specific reasoning effort, e.g., high, max, minimal)
  evidence_ids:
  - src-cli-cmd-run
  - parser-v22
  - parser-replay
  - help-7-8-3
  flag: --variant
  invocation_scope:
  - applies_to: command
    command:
    - run
  notes: Parser permits omission unless a choice/check rejects the empty or undefined value; this does not make the command semantically valid.
  value_optional: true
  value_type: string
- attachment: []
  description: show thinking blocks
  evidence_ids:
  - src-cli-cmd-run
  - parser-v22
  - parser-replay
  - help-7-8-3
  flag: --thinking
  invocation_scope:
  - applies_to: command
    command:
    - run
  value_type: none
- aliases:
  - -i
  - --i
  attachment: []
  description: run in direct interactive split-footer mode
  evidence_ids:
  - src-cli-cmd-run
  - parser-v22
  - parser-replay
  - help-7-8-3
  flag: --interactive
  invocation_scope:
  - applies_to: command
    command:
    - run
  value_type: none
- attachment: []
  description: Internal minimal interactive interface selector.
  evidence_ids:
  - src-cli-cmd-run
  - parser-v22
  - parser-replay
  - help-7-8-3
  flag: --mini
  invocation_scope:
  - applies_to: command
    command:
    - run
  notes: Hidden but accepted by the 7.8.3 source parser. Run scope retains compatibility switches; mini/replay settings may be rejected by the handler outside interactive mode.
  value_type: none
- aliases:
  - --replayLimit
  attachment:
  - space
  - equals
  description: Limit interactive replay messages.
  evidence_ids:
  - src-cli-cmd-run
  - parser-v22
  - parser-replay
  - help-7-8-3
  flag: --replay-limit
  invocation_scope:
  - applies_to: command
    command:
    - run
  notes: Hidden but accepted by the 7.8.3 source parser. Parser permits omission unless a choice/check rejects the empty or undefined value; this does not make the command semantically valid. Run scope retains compatibility switches; mini/replay settings may be rejected by the handler outside interactive mode.
  value_optional: true
  value_type: number
- aliases:
  - -n
  - --maxCount
  - --n
  attachment:
  - space
  - equals
  - short_attached
  description: limit to N most recent sessions
  evidence_ids:
  - src-cli-cmd-session
  - parser-v22
  - help-7-8-3
  flag: --max-count
  invocation_scope:
  - applies_to: command
    command:
    - session
    - list
  notes: Parser permits omission unless a choice/check rejects the empty or undefined value; this does not make the command semantically valid. Short attachment works for numeric or punctuation-leading values (for example -m123 or -f/path); alphabetic -mfoo is parsed as a short-option group and is not a general value form. Use -m foo or -m=foo.
  value_optional: true
  value_type: number
- attachment:
  - space
  - equals
  description: output format
  evidence_ids:
  - src-cli-cmd-session
  - parser-v22
  - help-7-8-3
  flag: --format
  invocation_scope:
  - applies_to: command
    command:
    - session
    - list
  notes: Parser permits omission unless a choice/check rejects the empty or undefined value; this does not make the command semantically valid.
  value_optional: true
  value_type: string
- aliases:
  - -a
  - --a
  attachment: []
  description: list sessions from all projects
  evidence_ids:
  - src-cli-cmd-session
  - parser-v22
  - help-7-8-3
  flag: --all
  invocation_scope:
  - applies_to: command
    command:
    - session
    - list
  value_type: none
- aliases:
  - -s
  - --s
  attachment:
  - space
  - equals
  - short_attached
  description: filter sessions by title
  evidence_ids:
  - src-cli-cmd-session
  - parser-v22
  - help-7-8-3
  flag: --search
  invocation_scope:
  - applies_to: command
    command:
    - session
    - list
  notes: Parser permits omission unless a choice/check rejects the empty or undefined value; this does not make the command semantically valid. Short attachment works for numeric or punctuation-leading values (for example -m123 or -f/path); alphabetic -mfoo is parsed as a short-option group and is not a general value form. Use -m foo or -m=foo.
  value_optional: true
  value_type: string
- attachment:
  - space
  - equals
  description: 'show stats for the last N days (default: all time)'
  evidence_ids:
  - src-cli-cmd-stats
  - parser-v22
  - help-7-8-3
  flag: --days
  invocation_scope:
  - applies_to: command
    command:
    - stats
  notes: Parser permits omission unless a choice/check rejects the empty or undefined value; this does not make the command semantically valid.
  value_optional: true
  value_type: number
- attachment:
  - space
  - equals
  description: 'number of tools to show (default: all)'
  evidence_ids:
  - src-cli-cmd-stats
  - parser-v22
  - help-7-8-3
  flag: --tools
  invocation_scope:
  - applies_to: command
    command:
    - stats
  notes: Parser permits omission unless a choice/check rejects the empty or undefined value; this does not make the command semantically valid.
  value_optional: true
  value_type: number
- attachment: []
  description: 'show model statistics (default: hidden). Pass a number to show top N, otherwise shows all'
  evidence_ids:
  - src-cli-cmd-stats
  - parser-v22
  - help-7-8-3
  flag: --models
  gap: The declaration has no explicit type and yargs infers boolean when bare, number for numeric input, and string otherwise. Replaying stats --models with omitted, numeric, and text values confirms the polymorphism; a contract capable of expressing those alternatives would settle its wrapper representation.
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
  - src-cli-cmd-stats
  - parser-v22
  - help-7-8-3
  flag: --project
  invocation_scope:
  - applies_to: command
    command:
    - stats
  notes: Parser permits omission unless a choice/check rejects the empty or undefined value; this does not make the command semantically valid.
  value_optional: true
  value_type: string
- aliases:
  - -c
  - --keepConfig
  - --c
  attachment: []
  description: keep configuration files
  evidence_ids:
  - src-cli-cmd-uninstall
  - parser-v22
  - help-7-8-3
  flag: --keep-config
  invocation_scope:
  - applies_to: command
    command:
    - uninstall
  value_type: none
- aliases:
  - -d
  - --keepData
  - --d
  attachment: []
  description: keep session data and snapshots
  evidence_ids:
  - src-cli-cmd-uninstall
  - parser-v22
  - help-7-8-3
  flag: --keep-data
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
  - src-cli-cmd-uninstall
  - parser-v22
  - help-7-8-3
  flag: --dry-run
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
  - src-cli-cmd-uninstall
  - parser-v22
  - help-7-8-3
  flag: --force
  invocation_scope:
  - applies_to: command
    command:
    - uninstall
  value_type: none
- aliases:
  - --no-h
  attachment: []
  description: Use parser-generated negation of --help.
  evidence_ids:
  - src-index
  - parser-v22
  - help-7-8-3
  flag: --no-help
  invocation_scope:
  - applies_to: global
  notes: Parser-generated negation consumes no following argument. Underlying value becomes false, zero for a number, or [false] for an array; choices or the handler may reject it. This is a recognized parser spelling, not a recommended invocation.
  value_type: none
- aliases:
  - --no-v
  attachment: []
  description: Use parser-generated negation of --version.
  evidence_ids:
  - src-index
  - parser-v22
  - help-7-8-3
  flag: --no-version
  invocation_scope:
  - applies_to: global
  notes: Parser-generated negation consumes no following argument. Underlying value becomes false, zero for a number, or [false] for an array; choices or the handler may reject it. This is a recognized parser spelling, not a recommended invocation.
  value_type: none
- aliases:
  - --no-printLogs
  attachment: []
  description: Use parser-generated negation of --print-logs.
  evidence_ids:
  - src-index
  - parser-v22
  - help-7-8-3
  flag: --no-print-logs
  invocation_scope:
  - applies_to: global
  notes: Parser-generated negation consumes no following argument. Underlying value becomes false, zero for a number, or [false] for an array; choices or the handler may reject it. This is a recognized parser spelling, not a recommended invocation.
  value_type: none
- aliases:
  - --no-logLevel
  attachment: []
  description: Use parser-generated negation of --log-level.
  evidence_ids:
  - src-index
  - parser-v22
  - help-7-8-3
  flag: --no-log-level
  invocation_scope:
  - applies_to: global
  notes: Parser-generated negation consumes no following argument. Underlying value becomes false, zero for a number, or [false] for an array; choices or the handler may reject it. This is a recognized parser spelling, not a recommended invocation.
  value_type: none
- attachment: []
  description: Use parser-generated negation of --pure.
  evidence_ids:
  - src-index
  - parser-v22
  - help-7-8-3
  flag: --no-pure
  invocation_scope:
  - applies_to: global
  notes: Parser-generated negation consumes no following argument. Underlying value becomes false, zero for a number, or [false] for an array; choices or the handler may reject it. This is a recognized parser spelling, not a recommended invocation.
  value_type: none
- attachment: []
  description: Use parser-generated negation of --port.
  evidence_ids:
  - src-cli-cmd-tui
  - parser-v22
  - src-kilocode-cli-cmd-daemon
  - src-cli-cmd-run
  - parser-replay
  - help-7-8-3
  flag: --no-port
  invocation_scope:
  - applies_to: command
    command: []
  - applies_to: command
    command:
    - completion
  - applies_to: command
    command:
    - daemon
  - applies_to: command
    command:
    - daemon
    - restart
  - applies_to: command
    command:
    - daemon
    - start
  - applies_to: command
    command:
    - run
  notes: Parser-generated negation consumes no following argument. Underlying value becomes false, zero for a number, or [false] for an array; choices or the handler may reject it. This is a recognized parser spelling, not a recommended invocation.
  value_type: none
- attachment: []
  description: Use parser-generated negation of --hostname.
  evidence_ids:
  - src-cli-cmd-tui
  - parser-v22
  - src-kilocode-cli-cmd-daemon
  - help-7-8-3
  flag: --no-hostname
  invocation_scope:
  - applies_to: command
    command: []
  - applies_to: command
    command:
    - completion
  - applies_to: command
    command:
    - daemon
  - applies_to: command
    command:
    - daemon
    - restart
  - applies_to: command
    command:
    - daemon
    - start
  notes: Parser-generated negation consumes no following argument. Underlying value becomes false, zero for a number, or [false] for an array; choices or the handler may reject it. This is a recognized parser spelling, not a recommended invocation.
  value_type: none
- attachment: []
  description: Use parser-generated negation of --mdns.
  evidence_ids:
  - src-cli-cmd-tui
  - parser-v22
  - src-kilocode-cli-cmd-daemon
  - help-7-8-3
  flag: --no-mdns
  invocation_scope:
  - applies_to: command
    command: []
  - applies_to: command
    command:
    - completion
  - applies_to: command
    command:
    - daemon
  - applies_to: command
    command:
    - daemon
    - restart
  - applies_to: command
    command:
    - daemon
    - start
  notes: Parser-generated negation consumes no following argument. Underlying value becomes false, zero for a number, or [false] for an array; choices or the handler may reject it. This is a recognized parser spelling, not a recommended invocation.
  value_type: none
- aliases:
  - --no-mdnsDomain
  attachment: []
  description: Use parser-generated negation of --mdns-domain.
  evidence_ids:
  - src-cli-cmd-tui
  - parser-v22
  - src-kilocode-cli-cmd-daemon
  - help-7-8-3
  flag: --no-mdns-domain
  invocation_scope:
  - applies_to: command
    command: []
  - applies_to: command
    command:
    - completion
  - applies_to: command
    command:
    - daemon
  - applies_to: command
    command:
    - daemon
    - restart
  - applies_to: command
    command:
    - daemon
    - start
  notes: Parser-generated negation consumes no following argument. Underlying value becomes false, zero for a number, or [false] for an array; choices or the handler may reject it. This is a recognized parser spelling, not a recommended invocation.
  value_type: none
- attachment: []
  description: Use parser-generated negation of --cors.
  evidence_ids:
  - src-cli-cmd-tui
  - parser-v22
  - src-kilocode-cli-cmd-daemon
  - help-7-8-3
  flag: --no-cors
  invocation_scope:
  - applies_to: command
    command: []
  - applies_to: command
    command:
    - completion
  - applies_to: command
    command:
    - daemon
  - applies_to: command
    command:
    - daemon
    - restart
  - applies_to: command
    command:
    - daemon
    - start
  notes: Parser-generated negation consumes no following argument. Underlying value becomes false, zero for a number, or [false] for an array; choices or the handler may reject it. This is a recognized parser spelling, not a recommended invocation.
  value_type: none
- aliases:
  - --no-m
  attachment: []
  description: Use parser-generated negation of --model.
  evidence_ids:
  - src-cli-cmd-tui
  - parser-v22
  - src-cli-cmd-run
  - parser-replay
  - help-7-8-3
  flag: --no-model
  invocation_scope:
  - applies_to: command
    command: []
  - applies_to: command
    command:
    - completion
  - applies_to: command
    command:
    - run
  notes: Parser-generated negation consumes no following argument. Underlying value becomes false, zero for a number, or [false] for an array; choices or the handler may reject it. This is a recognized parser spelling, not a recommended invocation.
  value_type: none
- aliases:
  - --no-c
  attachment: []
  description: Use parser-generated negation of --continue.
  evidence_ids:
  - src-cli-cmd-tui
  - parser-v22
  - src-cli-cmd-run
  - parser-replay
  - help-7-8-3
  flag: --no-continue
  invocation_scope:
  - applies_to: command
    command: []
  - applies_to: command
    command:
    - completion
  - applies_to: command
    command:
    - run
  notes: Parser-generated negation consumes no following argument. Underlying value becomes false, zero for a number, or [false] for an array; choices or the handler may reject it. This is a recognized parser spelling, not a recommended invocation.
  value_type: none
- aliases:
  - --no-s
  attachment: []
  description: Use parser-generated negation of --session.
  evidence_ids:
  - src-cli-cmd-tui
  - parser-v22
  - src-cli-cmd-pr
  - src-cli-cmd-run
  - parser-replay
  - help-7-8-3
  flag: --no-session
  invocation_scope:
  - applies_to: command
    command: []
  - applies_to: command
    command:
    - completion
  - applies_to: command
    command:
    - pr
    - link
  - applies_to: command
    command:
    - pr
    - status
  - applies_to: command
    command:
    - pr
    - unlink
  - applies_to: command
    command:
    - run
  notes: Parser-generated negation consumes no following argument. Underlying value becomes false, zero for a number, or [false] for an array; choices or the handler may reject it. This is a recognized parser spelling, not a recommended invocation.
  value_type: none
- attachment: []
  description: Use parser-generated negation of --fork.
  evidence_ids:
  - src-cli-cmd-tui
  - parser-v22
  - src-cli-cmd-run
  - parser-replay
  - help-7-8-3
  flag: --no-fork
  invocation_scope:
  - applies_to: command
    command: []
  - applies_to: command
    command:
    - completion
  - applies_to: command
    command:
    - run
  notes: Parser-generated negation consumes no following argument. Underlying value becomes false, zero for a number, or [false] for an array; choices or the handler may reject it. This is a recognized parser spelling, not a recommended invocation.
  value_type: none
- aliases:
  - --no-cloudFork
  attachment: []
  description: Use parser-generated negation of --cloud-fork.
  evidence_ids:
  - src-cli-cmd-tui
  - parser-v22
  - src-cli-cmd-run
  - parser-replay
  - help-7-8-3
  flag: --no-cloud-fork
  invocation_scope:
  - applies_to: command
    command: []
  - applies_to: command
    command:
    - completion
  - applies_to: command
    command:
    - run
  notes: Parser-generated negation consumes no following argument. Underlying value becomes false, zero for a number, or [false] for an array; choices or the handler may reject it. This is a recognized parser spelling, not a recommended invocation.
  value_type: none
- attachment: []
  description: Use parser-generated negation of --worktree.
  evidence_ids:
  - src-cli-cmd-tui
  - parser-v22
  - help-7-8-3
  flag: --no-worktree
  invocation_scope:
  - applies_to: command
    command: []
  - applies_to: command
    command:
    - completion
  notes: Parser-generated negation consumes no following argument. Underlying value becomes false, zero for a number, or [false] for an array; choices or the handler may reject it. This is a recognized parser spelling, not a recommended invocation.
  value_type: none
- attachment: []
  description: Use parser-generated negation of --prompt.
  evidence_ids:
  - src-cli-cmd-tui
  - parser-v22
  - src-kilocode-cli-cmd-cloud
  - src-kilocode-cli-cmd-roll-call
  - help-7-8-3
  flag: --no-prompt
  invocation_scope:
  - applies_to: command
    command: []
  - applies_to: command
    command:
    - cloud
    - send
  - applies_to: command
    command:
    - cloud
    - start
  - applies_to: command
    command:
    - completion
  - applies_to: command
    command:
    - roll-call
  notes: Parser-generated negation consumes no following argument. Underlying value becomes false, zero for a number, or [false] for an array; choices or the handler may reject it. This is a recognized parser spelling, not a recommended invocation.
  value_type: none
- attachment: []
  description: Use parser-generated negation of --agent.
  evidence_ids:
  - src-cli-cmd-tui
  - parser-v22
  - src-cli-cmd-run
  - parser-replay
  - help-7-8-3
  flag: --no-agent
  invocation_scope:
  - applies_to: command
    command: []
  - applies_to: command
    command:
    - completion
  - applies_to: command
    command:
    - run
  notes: Parser-generated negation consumes no following argument. Underlying value becomes false, zero for a number, or [false] for an array; choices or the handler may reject it. This is a recognized parser spelling, not a recommended invocation.
  value_type: none
- attachment: []
  description: Use parser-generated negation of --auto.
  evidence_ids:
  - src-cli-cmd-tui
  - parser-v22
  - src-cli-cmd-run
  - parser-replay
  - help-7-8-3
  flag: --no-auto
  invocation_scope:
  - applies_to: command
    command: []
  - applies_to: command
    command:
    - completion
  - applies_to: command
    command:
    - run
  notes: Parser-generated negation consumes no following argument. Underlying value becomes false, zero for a number, or [false] for an array; choices or the handler may reject it. This is a recognized parser spelling, not a recommended invocation.
  value_type: none
- attachment: []
  description: Use parser-generated negation of --mini.
  evidence_ids:
  - src-cli-cmd-tui
  - parser-v22
  - src-cli-cmd-run
  - parser-replay
  - help-7-8-3
  flag: --no-mini
  invocation_scope:
  - applies_to: command
    command: []
  - applies_to: command
    command:
    - completion
  - applies_to: command
    command:
    - run
  notes: Parser-generated negation consumes no following argument. Underlying value becomes false, zero for a number, or [false] for an array; choices or the handler may reject it. This is a recognized parser spelling, not a recommended invocation.
  value_type: none
- aliases:
  - --no-replayLimit
  attachment: []
  description: Use parser-generated negation of --replay-limit.
  evidence_ids:
  - src-cli-cmd-tui
  - parser-v22
  - src-cli-cmd-run
  - parser-replay
  - help-7-8-3
  flag: --no-replay-limit
  invocation_scope:
  - applies_to: command
    command: []
  - applies_to: command
    command:
    - completion
  - applies_to: command
    command:
    - run
  notes: Parser-generated negation consumes no following argument. Underlying value becomes false, zero for a number, or [false] for an array; choices or the handler may reject it. This is a recognized parser spelling, not a recommended invocation.
  value_type: none
- attachment: []
  description: Use parser-generated negation of --yolo.
  evidence_ids:
  - src-cli-cmd-tui
  - parser-v22
  - src-cli-cmd-run
  - parser-replay
  - help-7-8-3
  flag: --no-yolo
  invocation_scope:
  - applies_to: command
    command: []
  - applies_to: command
    command:
    - run
  notes: Parser-generated negation consumes no following argument. Underlying value becomes false, zero for a number, or [false] for an array; choices or the handler may reject it. This is a recognized parser spelling, not a recommended invocation.
  value_type: none
- aliases:
  - --no-dangerouslySkipPermissions
  attachment: []
  description: Use parser-generated negation of --dangerously-skip-permissions.
  evidence_ids:
  - src-cli-cmd-tui
  - parser-v22
  - src-cli-cmd-run
  - parser-replay
  - help-7-8-3
  flag: --no-dangerously-skip-permissions
  invocation_scope:
  - applies_to: command
    command: []
  - applies_to: command
    command:
    - run
  notes: Parser-generated negation consumes no following argument. Underlying value becomes false, zero for a number, or [false] for an array; choices or the handler may reject it. This is a recognized parser spelling, not a recommended invocation.
  value_type: none
- attachment: []
  description: Use parser-generated negation of --demo.
  evidence_ids:
  - src-cli-cmd-tui
  - parser-v22
  - src-cli-cmd-run
  - parser-replay
  - help-7-8-3
  flag: --no-demo
  invocation_scope:
  - applies_to: command
    command: []
  - applies_to: command
    command:
    - run
  notes: Parser-generated negation consumes no following argument. Underlying value becomes false, zero for a number, or [false] for an array; choices or the handler may reject it. This is a recognized parser spelling, not a recommended invocation.
  value_type: none
- aliases:
  - --no-sessionId
  attachment: []
  description: Use parser-generated negation of --session-id.
  evidence_ids:
  - src-kilocode-cli-cmd-cloud
  - parser-v22
  - help-7-8-3
  flag: --no-session-id
  invocation_scope:
  - applies_to: command
    command:
    - cloud
    - result
  - applies_to: command
    command:
    - cloud
    - send
  - applies_to: command
    command:
    - cloud
    - status
  notes: Parser-generated negation consumes no following argument. Underlying value becomes false, zero for a number, or [false] for an array; choices or the handler may reject it. This is a recognized parser spelling, not a recommended invocation.
  value_type: none
- aliases:
  - --no-messageId
  attachment: []
  description: Use parser-generated negation of --message-id.
  evidence_ids:
  - src-kilocode-cli-cmd-cloud
  - parser-v22
  - help-7-8-3
  flag: --no-message-id
  invocation_scope:
  - applies_to: command
    command:
    - cloud
    - result
  - applies_to: command
    command:
    - cloud
    - status
  notes: Parser-generated negation consumes no following argument. Underlying value becomes false, zero for a number, or [false] for an array; choices or the handler may reject it. This is a recognized parser spelling, not a recommended invocation.
  value_type: none
- aliases:
  - --no-promptStdin
  attachment: []
  description: Use parser-generated negation of --prompt-stdin.
  evidence_ids:
  - src-kilocode-cli-cmd-cloud
  - parser-v22
  - help-7-8-3
  flag: --no-prompt-stdin
  invocation_scope:
  - applies_to: command
    command:
    - cloud
    - send
  - applies_to: command
    command:
    - cloud
    - start
  notes: Parser-generated negation consumes no following argument. Underlying value becomes false, zero for a number, or [false] for an array; choices or the handler may reject it. This is a recognized parser spelling, not a recommended invocation.
  value_type: none
- attachment: []
  description: Use parser-generated negation of --repo.
  evidence_ids:
  - src-kilocode-cli-cmd-cloud
  - parser-v22
  - help-7-8-3
  flag: --no-repo
  invocation_scope:
  - applies_to: command
    command:
    - cloud
    - start
  notes: Parser-generated negation consumes no following argument. Underlying value becomes false, zero for a number, or [false] for an array; choices or the handler may reject it. This is a recognized parser spelling, not a recommended invocation.
  value_type: none
- aliases:
  - --no-repoType
  attachment: []
  description: Use parser-generated negation of --repo-type.
  evidence_ids:
  - src-kilocode-cli-cmd-cloud
  - parser-v22
  - help-7-8-3
  flag: --no-repo-type
  invocation_scope:
  - applies_to: command
    command:
    - cloud
    - start
  notes: Parser-generated negation consumes no following argument. Underlying value becomes false, zero for a number, or [false] for an array; choices or the handler may reject it. This is a recognized parser spelling, not a recommended invocation.
  value_type: none
- attachment: []
  description: Use parser-generated negation of --branch.
  evidence_ids:
  - src-kilocode-cli-cmd-cloud
  - parser-v22
  - help-7-8-3
  flag: --no-branch
  invocation_scope:
  - applies_to: command
    command:
    - cloud
    - start
  notes: Parser-generated negation consumes no following argument. Underlying value becomes false, zero for a number, or [false] for an array; choices or the handler may reject it. This is a recognized parser spelling, not a recommended invocation.
  value_type: none
- attachment: []
  description: Use parser-generated negation of --model.
  evidence_ids:
  - src-kilocode-cli-cmd-cloud
  - parser-v22
  - help-7-8-3
  flag: --no-model
  invocation_scope:
  - applies_to: command
    command:
    - cloud
    - start
  notes: Parser-generated negation consumes no following argument. Underlying value becomes false, zero for a number, or [false] for an array; choices or the handler may reject it. This is a recognized parser spelling, not a recommended invocation.
  value_type: none
- attachment: []
  description: Use parser-generated negation of --mode.
  evidence_ids:
  - src-kilocode-cli-cmd-cloud
  - parser-v22
  - help-7-8-3
  flag: --no-mode
  invocation_scope:
  - applies_to: command
    command:
    - cloud
    - start
  notes: Parser-generated negation consumes no following argument. Underlying value becomes false, zero for a number, or [false] for an array; choices or the handler may reject it. This is a recognized parser spelling, not a recommended invocation.
  value_type: none
- aliases:
  - --no-orgId
  attachment: []
  description: Use parser-generated negation of --org-id.
  evidence_ids:
  - src-kilocode-cli-cmd-cloud
  - parser-v22
  - help-7-8-3
  flag: --no-org-id
  invocation_scope:
  - applies_to: command
    command:
    - cloud
    - start
  notes: Parser-generated negation consumes no following argument. Underlying value becomes false, zero for a number, or [false] for an array; choices or the handler may reject it. This is a recognized parser spelling, not a recommended invocation.
  value_type: none
- attachment: []
  description: Use parser-generated negation of --stream.
  evidence_ids:
  - src-kilocode-cli-cmd-cloud
  - parser-v22
  - help-7-8-3
  flag: --no-stream
  invocation_scope:
  - applies_to: command
    command:
    - cloud
    - start
  notes: Parser-generated negation consumes no following argument. Underlying value becomes false, zero for a number, or [false] for an array; choices or the handler may reject it. This is a recognized parser spelling, not a recommended invocation.
  value_type: none
- attachment: []
  description: Use parser-generated negation of --json.
  evidence_ids:
  - src-kilocode-cli-cmd-console
  - parser-v22
  - src-kilocode-cli-cmd-daemon
  - src-kilocode-cli-cmd-profile
  - help-7-8-3
  flag: --no-json
  invocation_scope:
  - applies_to: command
    command:
    - console
    - stop
  - applies_to: command
    command:
    - daemon
  - applies_to: command
    command:
    - daemon
    - restart
  - applies_to: command
    command:
    - daemon
    - start
  - applies_to: command
    command:
    - daemon
    - status
  - applies_to: command
    command:
    - daemon
    - stop
  - applies_to: command
    command:
    - profile
  notes: Parser-generated negation consumes no following argument. Underlying value becomes false, zero for a number, or [false] for an array; choices or the handler may reject it. This is a recognized parser spelling, not a recommended invocation.
  value_type: none
- aliases:
  - --no-f
  attachment: []
  description: Use parser-generated negation of --foreground.
  evidence_ids:
  - src-kilocode-cli-cmd-daemon
  - parser-v22
  - help-7-8-3
  flag: --no-foreground
  invocation_scope:
  - applies_to: command
    command:
    - daemon
  - applies_to: command
    command:
    - daemon
    - restart
  - applies_to: command
    command:
    - daemon
    - start
  notes: Parser-generated negation consumes no following argument. Underlying value becomes false, zero for a number, or [false] for an array; choices or the handler may reject it. This is a recognized parser spelling, not a recommended invocation.
  value_type: none
- attachment: []
  description: Use parser-generated negation of --format.
  evidence_ids:
  - src-cli-cmd-db
  - parser-v22
  - src-kilocode-help-command
  - src-cli-cmd-run
  - src-cli-cmd-session
  - parser-replay
  - help-7-8-3
  flag: --no-format
  invocation_scope:
  - applies_to: command
    command:
    - db
  - applies_to: command
    command:
    - help
  - applies_to: command
    command:
    - run
  - applies_to: command
    command:
    - session
    - list
  notes: Parser-generated negation consumes no following argument. Underlying value becomes false, zero for a number, or [false] for an array; choices or the handler may reject it. This is a recognized parser spelling, not a recommended invocation.
  value_type: none
- attachment: []
  description: Use parser-generated negation of --tool.
  evidence_ids:
  - src-cli-cmd-debug-agent
  - parser-v22
  - help-7-8-3
  flag: --no-tool
  invocation_scope:
  - applies_to: command
    command:
    - debug
    - agent
  notes: Parser-generated negation consumes no following argument. Underlying value becomes false, zero for a number, or [false] for an array; choices or the handler may reject it. This is a recognized parser spelling, not a recommended invocation.
  value_type: none
- attachment: []
  description: Use parser-generated negation of --params.
  evidence_ids:
  - src-cli-cmd-debug-agent
  - parser-v22
  - help-7-8-3
  flag: --no-params
  invocation_scope:
  - applies_to: command
    command:
    - debug
    - agent
  notes: Parser-generated negation consumes no following argument. Underlying value becomes false, zero for a number, or [false] for an array; choices or the handler may reject it. This is a recognized parser spelling, not a recommended invocation.
  value_type: none
- attachment: []
  description: Use parser-generated negation of --query.
  evidence_ids:
  - src-cli-cmd-debug-ripgrep
  - parser-v22
  - help-7-8-3
  flag: --no-query
  invocation_scope:
  - applies_to: command
    command:
    - debug
    - rg
    - files
  notes: Parser-generated negation consumes no following argument. Underlying value becomes false, zero for a number, or [false] for an array; choices or the handler may reject it. This is a recognized parser spelling, not a recommended invocation.
  value_type: none
- attachment: []
  description: Use parser-generated negation of --glob.
  evidence_ids:
  - src-cli-cmd-debug-ripgrep
  - parser-v22
  - help-7-8-3
  flag: --no-glob
  invocation_scope:
  - applies_to: command
    command:
    - debug
    - rg
    - files
  - applies_to: command
    command:
    - debug
    - rg
    - search
  notes: Parser-generated negation consumes no following argument. Underlying value becomes false, zero for a number, or [false] for an array; choices or the handler may reject it. This is a recognized parser spelling, not a recommended invocation.
  value_type: none
- attachment: []
  description: Use parser-generated negation of --limit.
  evidence_ids:
  - src-cli-cmd-debug-ripgrep
  - parser-v22
  - src-old-cli-cmd-debug-ripgrep
  - help-7-3-45
  - help-7-8-3
  flag: --no-limit
  invocation_scope:
  - applies_to: command
    command:
    - debug
    - rg
    - files
  - applies_to: command
    command:
    - debug
    - rg
    - search
  - applies_to: command
    command:
    - debug
    - rg
    - tree
  notes: Parser-generated negation consumes no following argument. Underlying value becomes false, zero for a number, or [false] for an array; choices or the handler may reject it. This is a recognized parser spelling, not a recommended invocation.
  value_type: none
- attachment: []
  description: Use parser-generated negation of --sanitize.
  evidence_ids:
  - src-cli-cmd-export
  - parser-v22
  - help-7-8-3
  flag: --no-sanitize
  invocation_scope:
  - applies_to: command
    command:
    - export
  notes: Parser-generated negation consumes no following argument. Underlying value becomes false, zero for a number, or [false] for an array; choices or the handler may reject it. This is a recognized parser spelling, not a recommended invocation.
  value_type: none
- attachment: []
  description: Use parser-generated negation of --event.
  evidence_ids:
  - src-cli-cmd-github
  - parser-v22
  - help-7-8-3
  flag: --no-event
  invocation_scope:
  - applies_to: command
    command:
    - github
    - run
  notes: Parser-generated negation consumes no following argument. Underlying value becomes false, zero for a number, or [false] for an array; choices or the handler may reject it. This is a recognized parser spelling, not a recommended invocation.
  value_type: none
- attachment: []
  description: Use parser-generated negation of --token.
  evidence_ids:
  - src-cli-cmd-github
  - parser-v22
  - help-7-8-3
  flag: --no-token
  invocation_scope:
  - applies_to: command
    command:
    - github
    - run
  notes: Parser-generated negation consumes no following argument. Underlying value becomes false, zero for a number, or [false] for an array; choices or the handler may reject it. This is a recognized parser spelling, not a recommended invocation.
  value_type: none
- attachment: []
  description: Use parser-generated negation of --all.
  evidence_ids:
  - src-kilocode-help-command
  - parser-v22
  - help-7-8-3
  flag: --no-all
  invocation_scope:
  - applies_to: command
    command:
    - help
  notes: Parser-generated negation consumes no following argument. Underlying value becomes false, zero for a number, or [false] for an array; choices or the handler may reject it. This is a recognized parser spelling, not a recommended invocation.
  value_type: none
- attachment: []
  description: Use parser-generated negation of --url.
  evidence_ids:
  - src-cli-cmd-mcp
  - parser-v22
  - help-7-8-3
  flag: --no-url
  invocation_scope:
  - applies_to: command
    command:
    - mcp
    - add
  notes: Parser-generated negation consumes no following argument. Underlying value becomes false, zero for a number, or [false] for an array; choices or the handler may reject it. This is a recognized parser spelling, not a recommended invocation.
  value_type: none
- attachment: []
  description: Use parser-generated negation of --env.
  evidence_ids:
  - src-cli-cmd-mcp
  - parser-v22
  - help-7-8-3
  flag: --no-env
  invocation_scope:
  - applies_to: command
    command:
    - mcp
    - add
  notes: Parser-generated negation consumes no following argument. Underlying value becomes false, zero for a number, or [false] for an array; choices or the handler may reject it. This is a recognized parser spelling, not a recommended invocation.
  value_type: none
- attachment: []
  description: Use parser-generated negation of --header.
  evidence_ids:
  - src-cli-cmd-mcp
  - parser-v22
  - help-7-8-3
  flag: --no-header
  invocation_scope:
  - applies_to: command
    command:
    - mcp
    - add
  notes: Parser-generated negation consumes no following argument. Underlying value becomes false, zero for a number, or [false] for an array; choices or the handler may reject it. This is a recognized parser spelling, not a recommended invocation.
  value_type: none
- attachment: []
  description: Use parser-generated negation of --verbose.
  evidence_ids:
  - src-cli-cmd-models
  - parser-v22
  - src-kilocode-cli-cmd-roll-call
  - help-7-8-3
  flag: --no-verbose
  invocation_scope:
  - applies_to: command
    command:
    - models
  - applies_to: command
    command:
    - roll-call
  notes: Parser-generated negation consumes no following argument. Underlying value becomes false, zero for a number, or [false] for an array; choices or the handler may reject it. This is a recognized parser spelling, not a recommended invocation.
  value_type: none
- attachment: []
  description: Use parser-generated negation of --refresh.
  evidence_ids:
  - src-cli-cmd-models
  - parser-v22
  - help-7-8-3
  flag: --no-refresh
  invocation_scope:
  - applies_to: command
    command:
    - models
  notes: Parser-generated negation consumes no following argument. Underlying value becomes false, zero for a number, or [false] for an array; choices or the handler may reject it. This is a recognized parser spelling, not a recommended invocation.
  value_type: none
- aliases:
  - --no-g
  attachment: []
  description: Use parser-generated negation of --global.
  evidence_ids:
  - src-cli-cmd-plug
  - parser-v22
  - help-7-8-3
  flag: --no-global
  invocation_scope:
  - applies_to: command
    command:
    - plug
  - applies_to: command
    command:
    - plugin
  notes: Parser-generated negation consumes no following argument. Underlying value becomes false, zero for a number, or [false] for an array; choices or the handler may reject it. This is a recognized parser spelling, not a recommended invocation.
  value_type: none
- aliases:
  - --no-f
  attachment: []
  description: Use parser-generated negation of --force.
  evidence_ids:
  - src-cli-cmd-plug
  - parser-v22
  - src-cli-cmd-uninstall
  - help-7-8-3
  flag: --no-force
  invocation_scope:
  - applies_to: command
    command:
    - plug
  - applies_to: command
    command:
    - plugin
  - applies_to: command
    command:
    - uninstall
  notes: Parser-generated negation consumes no following argument. Underlying value becomes false, zero for a number, or [false] for an array; choices or the handler may reject it. This is a recognized parser spelling, not a recommended invocation.
  value_type: none
- attachment: []
  description: Use parser-generated negation of --timeout.
  evidence_ids:
  - src-kilocode-cli-cmd-roll-call
  - parser-v22
  - help-7-8-3
  flag: --no-timeout
  invocation_scope:
  - applies_to: command
    command:
    - roll-call
  notes: Parser-generated negation consumes no following argument. Underlying value becomes false, zero for a number, or [false] for an array; choices or the handler may reject it. This is a recognized parser spelling, not a recommended invocation.
  value_type: none
- attachment: []
  description: Use parser-generated negation of --parallel.
  evidence_ids:
  - src-kilocode-cli-cmd-roll-call
  - parser-v22
  - help-7-8-3
  flag: --no-parallel
  invocation_scope:
  - applies_to: command
    command:
    - roll-call
  notes: Parser-generated negation consumes no following argument. Underlying value becomes false, zero for a number, or [false] for an array; choices or the handler may reject it. This is a recognized parser spelling, not a recommended invocation.
  value_type: none
- attachment: []
  description: Use parser-generated negation of --quiet.
  evidence_ids:
  - src-kilocode-cli-cmd-roll-call
  - parser-v22
  - help-7-8-3
  flag: --no-quiet
  invocation_scope:
  - applies_to: command
    command:
    - roll-call
  notes: Parser-generated negation consumes no following argument. Underlying value becomes false, zero for a number, or [false] for an array; choices or the handler may reject it. This is a recognized parser spelling, not a recommended invocation.
  value_type: none
- attachment: []
  description: Use parser-generated negation of --output.
  evidence_ids:
  - src-kilocode-cli-cmd-roll-call
  - parser-v22
  - help-7-8-3
  flag: --no-output
  invocation_scope:
  - applies_to: command
    command:
    - roll-call
  notes: Parser-generated negation consumes no following argument. Underlying value becomes false, zero for a number, or [false] for an array; choices or the handler may reject it. This is a recognized parser spelling, not a recommended invocation.
  value_type: none
- attachment: []
  description: Use parser-generated negation of --command.
  evidence_ids:
  - src-cli-cmd-run
  - parser-v22
  - parser-replay
  - help-7-8-3
  flag: --no-command
  invocation_scope:
  - applies_to: command
    command:
    - run
  notes: Parser-generated negation consumes no following argument. Underlying value becomes false, zero for a number, or [false] for an array; choices or the handler may reject it. This is a recognized parser spelling, not a recommended invocation.
  value_type: none
- attachment: []
  description: Use parser-generated negation of --share.
  evidence_ids:
  - src-cli-cmd-run
  - parser-v22
  - parser-replay
  - help-7-8-3
  flag: --no-share
  invocation_scope:
  - applies_to: command
    command:
    - run
  notes: Parser-generated negation consumes no following argument. Underlying value becomes false, zero for a number, or [false] for an array; choices or the handler may reject it. This is a recognized parser spelling, not a recommended invocation.
  value_type: none
- aliases:
  - --no-f
  attachment: []
  description: Use parser-generated negation of --file.
  evidence_ids:
  - src-cli-cmd-run
  - parser-v22
  - parser-replay
  - help-7-8-3
  flag: --no-file
  invocation_scope:
  - applies_to: command
    command:
    - run
  notes: Parser-generated negation consumes no following argument. Underlying value becomes false, zero for a number, or [false] for an array; choices or the handler may reject it. This is a recognized parser spelling, not a recommended invocation.
  value_type: none
- attachment: []
  description: Use parser-generated negation of --title.
  evidence_ids:
  - src-cli-cmd-run
  - parser-v22
  - parser-replay
  - help-7-8-3
  flag: --no-title
  invocation_scope:
  - applies_to: command
    command:
    - run
  notes: Parser-generated negation consumes no following argument. Underlying value becomes false, zero for a number, or [false] for an array; choices or the handler may reject it. This is a recognized parser spelling, not a recommended invocation.
  value_type: none
- attachment: []
  description: Use parser-generated negation of --attach.
  evidence_ids:
  - src-cli-cmd-run
  - parser-v22
  - parser-replay
  - help-7-8-3
  flag: --no-attach
  invocation_scope:
  - applies_to: command
    command:
    - run
  notes: Parser-generated negation consumes no following argument. Underlying value becomes false, zero for a number, or [false] for an array; choices or the handler may reject it. This is a recognized parser spelling, not a recommended invocation.
  value_type: none
- aliases:
  - --no-p
  attachment: []
  description: Use parser-generated negation of --password.
  evidence_ids:
  - src-cli-cmd-run
  - parser-v22
  - parser-replay
  - help-7-8-3
  flag: --no-password
  invocation_scope:
  - applies_to: command
    command:
    - run
  notes: Parser-generated negation consumes no following argument. Underlying value becomes false, zero for a number, or [false] for an array; choices or the handler may reject it. This is a recognized parser spelling, not a recommended invocation.
  value_type: none
- aliases:
  - --no-u
  attachment: []
  description: Use parser-generated negation of --username.
  evidence_ids:
  - src-cli-cmd-run
  - parser-v22
  - parser-replay
  - help-7-8-3
  flag: --no-username
  invocation_scope:
  - applies_to: command
    command:
    - run
  notes: Parser-generated negation consumes no following argument. Underlying value becomes false, zero for a number, or [false] for an array; choices or the handler may reject it. This is a recognized parser spelling, not a recommended invocation.
  value_type: none
- attachment: []
  description: Use parser-generated negation of --dir.
  evidence_ids:
  - src-cli-cmd-run
  - parser-v22
  - parser-replay
  - help-7-8-3
  flag: --no-dir
  invocation_scope:
  - applies_to: command
    command:
    - run
  notes: Parser-generated negation consumes no following argument. Underlying value becomes false, zero for a number, or [false] for an array; choices or the handler may reject it. This is a recognized parser spelling, not a recommended invocation.
  value_type: none
- attachment: []
  description: Use parser-generated negation of --variant.
  evidence_ids:
  - src-cli-cmd-run
  - parser-v22
  - parser-replay
  - help-7-8-3
  flag: --no-variant
  invocation_scope:
  - applies_to: command
    command:
    - run
  notes: Parser-generated negation consumes no following argument. Underlying value becomes false, zero for a number, or [false] for an array; choices or the handler may reject it. This is a recognized parser spelling, not a recommended invocation.
  value_type: none
- attachment: []
  description: Use parser-generated negation of --thinking.
  evidence_ids:
  - src-cli-cmd-run
  - parser-v22
  - parser-replay
  - help-7-8-3
  flag: --no-thinking
  invocation_scope:
  - applies_to: command
    command:
    - run
  notes: Parser-generated negation consumes no following argument. Underlying value becomes false, zero for a number, or [false] for an array; choices or the handler may reject it. This is a recognized parser spelling, not a recommended invocation.
  value_type: none
- aliases:
  - --no-i
  attachment: []
  description: Use parser-generated negation of --interactive.
  evidence_ids:
  - src-cli-cmd-run
  - parser-v22
  - parser-replay
  - help-7-8-3
  flag: --no-interactive
  invocation_scope:
  - applies_to: command
    command:
    - run
  notes: Parser-generated negation consumes no following argument. Underlying value becomes false, zero for a number, or [false] for an array; choices or the handler may reject it. This is a recognized parser spelling, not a recommended invocation.
  value_type: none
- attachment: []
  description: Use parser-generated negation of --replay.
  evidence_ids:
  - src-cli-cmd-run
  - parser-v22
  - parser-replay
  - help-7-8-3
  flag: --no-replay
  invocation_scope:
  - applies_to: command
    command:
    - run
  notes: Parser-generated negation consumes no following argument. Underlying value becomes false, zero for a number, or [false] for an array; choices or the handler may reject it. This is a recognized parser spelling, not a recommended invocation.
  value_type: none
- aliases:
  - --no-n
  - --no-maxCount
  attachment: []
  description: Use parser-generated negation of --max-count.
  evidence_ids:
  - src-cli-cmd-session
  - parser-v22
  - help-7-8-3
  flag: --no-max-count
  invocation_scope:
  - applies_to: command
    command:
    - session
    - list
  notes: Parser-generated negation consumes no following argument. Underlying value becomes false, zero for a number, or [false] for an array; choices or the handler may reject it. This is a recognized parser spelling, not a recommended invocation.
  value_type: none
- aliases:
  - --no-a
  attachment: []
  description: Use parser-generated negation of --all.
  evidence_ids:
  - src-cli-cmd-session
  - parser-v22
  - help-7-8-3
  flag: --no-all
  invocation_scope:
  - applies_to: command
    command:
    - session
    - list
  notes: Parser-generated negation consumes no following argument. Underlying value becomes false, zero for a number, or [false] for an array; choices or the handler may reject it. This is a recognized parser spelling, not a recommended invocation.
  value_type: none
- aliases:
  - --no-s
  attachment: []
  description: Use parser-generated negation of --search.
  evidence_ids:
  - src-cli-cmd-session
  - parser-v22
  - help-7-8-3
  flag: --no-search
  invocation_scope:
  - applies_to: command
    command:
    - session
    - list
  notes: Parser-generated negation consumes no following argument. Underlying value becomes false, zero for a number, or [false] for an array; choices or the handler may reject it. This is a recognized parser spelling, not a recommended invocation.
  value_type: none
- attachment: []
  description: Use parser-generated negation of --days.
  evidence_ids:
  - src-cli-cmd-stats
  - parser-v22
  - help-7-8-3
  flag: --no-days
  invocation_scope:
  - applies_to: command
    command:
    - stats
  notes: Parser-generated negation consumes no following argument. Underlying value becomes false, zero for a number, or [false] for an array; choices or the handler may reject it. This is a recognized parser spelling, not a recommended invocation.
  value_type: none
- attachment: []
  description: Use parser-generated negation of --tools.
  evidence_ids:
  - src-cli-cmd-stats
  - parser-v22
  - help-7-8-3
  flag: --no-tools
  invocation_scope:
  - applies_to: command
    command:
    - stats
  notes: Parser-generated negation consumes no following argument. Underlying value becomes false, zero for a number, or [false] for an array; choices or the handler may reject it. This is a recognized parser spelling, not a recommended invocation.
  value_type: none
- attachment: []
  description: Use parser-generated negation of --models.
  evidence_ids:
  - src-cli-cmd-stats
  - parser-v22
  - help-7-8-3
  flag: --no-models
  invocation_scope:
  - applies_to: command
    command:
    - stats
  notes: Parser-generated negation consumes no following argument. Underlying value becomes false, zero for a number, or [false] for an array; choices or the handler may reject it. This is a recognized parser spelling, not a recommended invocation.
  value_type: none
- attachment: []
  description: Use parser-generated negation of --project.
  evidence_ids:
  - src-cli-cmd-stats
  - parser-v22
  - help-7-8-3
  flag: --no-project
  invocation_scope:
  - applies_to: command
    command:
    - stats
  notes: Parser-generated negation consumes no following argument. Underlying value becomes false, zero for a number, or [false] for an array; choices or the handler may reject it. This is a recognized parser spelling, not a recommended invocation.
  value_type: none
- aliases:
  - --no-c
  - --no-keepConfig
  attachment: []
  description: Use parser-generated negation of --keep-config.
  evidence_ids:
  - src-cli-cmd-uninstall
  - parser-v22
  - help-7-8-3
  flag: --no-keep-config
  invocation_scope:
  - applies_to: command
    command:
    - uninstall
  notes: Parser-generated negation consumes no following argument. Underlying value becomes false, zero for a number, or [false] for an array; choices or the handler may reject it. This is a recognized parser spelling, not a recommended invocation.
  value_type: none
- aliases:
  - --no-d
  - --no-keepData
  attachment: []
  description: Use parser-generated negation of --keep-data.
  evidence_ids:
  - src-cli-cmd-uninstall
  - parser-v22
  - help-7-8-3
  flag: --no-keep-data
  invocation_scope:
  - applies_to: command
    command:
    - uninstall
  notes: Parser-generated negation consumes no following argument. Underlying value becomes false, zero for a number, or [false] for an array; choices or the handler may reject it. This is a recognized parser spelling, not a recommended invocation.
  value_type: none
- aliases:
  - --no-dryRun
  attachment: []
  description: Use parser-generated negation of --dry-run.
  evidence_ids:
  - src-cli-cmd-uninstall
  - parser-v22
  - help-7-8-3
  flag: --no-dry-run
  invocation_scope:
  - applies_to: command
    command:
    - uninstall
  notes: Parser-generated negation consumes no following argument. Underlying value becomes false, zero for a number, or [false] for an array; choices or the handler may reject it. This is a recognized parser spelling, not a recommended invocation.
  value_type: none
config_paths:
- format: jsonc
  notes: Default XDG_CONFIG_HOME/kilo directory on every OS; Windows source uses home/.config, not APPDATA. Main global load order is config.json, kilo.json, kilo.jsonc, opencode.json, opencode.jsonc; TUI files are a separate layer.
  os: macos
  path: ~/.config/kilo/config.json
  scope: user
- format: jsonc
  notes: Default XDG_CONFIG_HOME/kilo directory on every OS; Windows source uses home/.config, not APPDATA. Main global load order is config.json, kilo.json, kilo.jsonc, opencode.json, opencode.jsonc; TUI files are a separate layer.
  os: macos
  path: ~/.config/kilo/kilo.json
  scope: user
- format: jsonc
  notes: Default XDG_CONFIG_HOME/kilo directory on every OS; Windows source uses home/.config, not APPDATA. Main global load order is config.json, kilo.json, kilo.jsonc, opencode.json, opencode.jsonc; TUI files are a separate layer.
  os: macos
  path: ~/.config/kilo/kilo.jsonc
  scope: user
- format: jsonc
  notes: Default XDG_CONFIG_HOME/kilo directory on every OS; Windows source uses home/.config, not APPDATA. Main global load order is config.json, kilo.json, kilo.jsonc, opencode.json, opencode.jsonc; TUI files are a separate layer.
  os: macos
  path: ~/.config/kilo/opencode.json
  scope: user
- format: jsonc
  notes: Default XDG_CONFIG_HOME/kilo directory on every OS; Windows source uses home/.config, not APPDATA. Main global load order is config.json, kilo.json, kilo.jsonc, opencode.json, opencode.jsonc; TUI files are a separate layer.
  os: macos
  path: ~/.config/kilo/opencode.jsonc
  scope: user
- format: jsonc
  notes: Default XDG_CONFIG_HOME/kilo directory on every OS; Windows source uses home/.config, not APPDATA. Main global load order is config.json, kilo.json, kilo.jsonc, opencode.json, opencode.jsonc; TUI files are a separate layer.
  os: macos
  path: ~/.config/kilo/tui.json
  scope: user
- format: jsonc
  notes: Default XDG_CONFIG_HOME/kilo directory on every OS; Windows source uses home/.config, not APPDATA. Main global load order is config.json, kilo.json, kilo.jsonc, opencode.json, opencode.jsonc; TUI files are a separate layer.
  os: macos
  path: ~/.config/kilo/tui.jsonc
  scope: user
- format: jsonc
  notes: Walked from launch directory to worktree boundary and merged ancestor-first; disabled by KILO_DISABLE_PROJECT_CONFIG.
  os: macos
  path: ./kilo.json
  scope: repo
- format: jsonc
  notes: Supported config-directory discovery; also examines the same directory names in the user home and primary checkout fallbacks for linked worktrees.
  os: macos
  path: ./.kilo/kilo.json
  scope: repo
- format: jsonc
  notes: Supported config-directory discovery; also examines the same directory names in the user home and primary checkout fallbacks for linked worktrees.
  os: macos
  path: ./.kilocode/kilo.json
  scope: repo
- format: jsonc
  notes: Walked from launch directory to worktree boundary and merged ancestor-first; disabled by KILO_DISABLE_PROJECT_CONFIG.
  os: macos
  path: ./kilo.jsonc
  scope: repo
- format: jsonc
  notes: Supported config-directory discovery; also examines the same directory names in the user home and primary checkout fallbacks for linked worktrees.
  os: macos
  path: ./.kilo/kilo.jsonc
  scope: repo
- format: jsonc
  notes: Supported config-directory discovery; also examines the same directory names in the user home and primary checkout fallbacks for linked worktrees.
  os: macos
  path: ./.kilocode/kilo.jsonc
  scope: repo
- format: jsonc
  notes: Walked from launch directory to worktree boundary and merged ancestor-first; disabled by KILO_DISABLE_PROJECT_CONFIG.
  os: macos
  path: ./opencode.json
  scope: repo
- format: jsonc
  notes: Supported config-directory discovery; also examines the same directory names in the user home and primary checkout fallbacks for linked worktrees.
  os: macos
  path: ./.kilo/opencode.json
  scope: repo
- format: jsonc
  notes: Supported config-directory discovery; also examines the same directory names in the user home and primary checkout fallbacks for linked worktrees.
  os: macos
  path: ./.kilocode/opencode.json
  scope: repo
- format: jsonc
  notes: Walked from launch directory to worktree boundary and merged ancestor-first; disabled by KILO_DISABLE_PROJECT_CONFIG.
  os: macos
  path: ./opencode.jsonc
  scope: repo
- format: jsonc
  notes: Supported config-directory discovery; also examines the same directory names in the user home and primary checkout fallbacks for linked worktrees.
  os: macos
  path: ./.kilo/opencode.jsonc
  scope: repo
- format: jsonc
  notes: Supported config-directory discovery; also examines the same directory names in the user home and primary checkout fallbacks for linked worktrees.
  os: macos
  path: ./.kilocode/opencode.jsonc
  scope: repo
- format: jsonc
  notes: Walked from launch directory to worktree boundary and merged ancestor-first; disabled by KILO_DISABLE_PROJECT_CONFIG.
  os: macos
  path: ./tui.json
  scope: repo
- format: jsonc
  notes: Supported config-directory discovery; also examines the same directory names in the user home and primary checkout fallbacks for linked worktrees.
  os: macos
  path: ./.kilo/tui.json
  scope: repo
- format: jsonc
  notes: Supported config-directory discovery; also examines the same directory names in the user home and primary checkout fallbacks for linked worktrees.
  os: macos
  path: ./.kilocode/tui.json
  scope: repo
- format: jsonc
  notes: Walked from launch directory to worktree boundary and merged ancestor-first; disabled by KILO_DISABLE_PROJECT_CONFIG.
  os: macos
  path: ./tui.jsonc
  scope: repo
- format: jsonc
  notes: Supported config-directory discovery; also examines the same directory names in the user home and primary checkout fallbacks for linked worktrees.
  os: macos
  path: ./.kilo/tui.jsonc
  scope: repo
- format: jsonc
  notes: Supported config-directory discovery; also examines the same directory names in the user home and primary checkout fallbacks for linked worktrees.
  os: macos
  path: ./.kilocode/tui.jsonc
  scope: repo
- format: jsonc
  notes: Enterprise-managed files override ordinary file and inline-content layers; directory can be redirected by KILO_TEST_MANAGED_CONFIG_DIR.
  os: macos
  path: /Library/Application Support/kilo/kilo.jsonc
  scope: system
- format: jsonc
  notes: Enterprise-managed files override ordinary file and inline-content layers; directory can be redirected by KILO_TEST_MANAGED_CONFIG_DIR.
  os: macos
  path: /Library/Application Support/kilo/kilo.json
  scope: system
- format: jsonc
  notes: Enterprise-managed files override ordinary file and inline-content layers; directory can be redirected by KILO_TEST_MANAGED_CONFIG_DIR.
  os: macos
  path: /Library/Application Support/kilo/opencode.jsonc
  scope: system
- format: jsonc
  notes: Enterprise-managed files override ordinary file and inline-content layers; directory can be redirected by KILO_TEST_MANAGED_CONFIG_DIR.
  os: macos
  path: /Library/Application Support/kilo/opencode.json
  scope: system
- format: jsonc
  notes: Explicit JSONC file loaded after global config, before project config.
  os: macos
  path: $KILO_CONFIG
  scope: env
- format: jsonc
  notes: Explicit TUI JSONC file loaded after global TUI, before project TUI.
  os: macos
  path: $KILO_TUI_CONFIG
  scope: env
- format: jsonc
  notes: Default XDG_CONFIG_HOME/kilo directory on every OS; Windows source uses home/.config, not APPDATA. Main global load order is config.json, kilo.json, kilo.jsonc, opencode.json, opencode.jsonc; TUI files are a separate layer.
  os: linux
  path: ~/.config/kilo/config.json
  scope: user
- format: jsonc
  notes: Default XDG_CONFIG_HOME/kilo directory on every OS; Windows source uses home/.config, not APPDATA. Main global load order is config.json, kilo.json, kilo.jsonc, opencode.json, opencode.jsonc; TUI files are a separate layer.
  os: linux
  path: ~/.config/kilo/kilo.json
  scope: user
- format: jsonc
  notes: Default XDG_CONFIG_HOME/kilo directory on every OS; Windows source uses home/.config, not APPDATA. Main global load order is config.json, kilo.json, kilo.jsonc, opencode.json, opencode.jsonc; TUI files are a separate layer.
  os: linux
  path: ~/.config/kilo/kilo.jsonc
  scope: user
- format: jsonc
  notes: Default XDG_CONFIG_HOME/kilo directory on every OS; Windows source uses home/.config, not APPDATA. Main global load order is config.json, kilo.json, kilo.jsonc, opencode.json, opencode.jsonc; TUI files are a separate layer.
  os: linux
  path: ~/.config/kilo/opencode.json
  scope: user
- format: jsonc
  notes: Default XDG_CONFIG_HOME/kilo directory on every OS; Windows source uses home/.config, not APPDATA. Main global load order is config.json, kilo.json, kilo.jsonc, opencode.json, opencode.jsonc; TUI files are a separate layer.
  os: linux
  path: ~/.config/kilo/opencode.jsonc
  scope: user
- format: jsonc
  notes: Default XDG_CONFIG_HOME/kilo directory on every OS; Windows source uses home/.config, not APPDATA. Main global load order is config.json, kilo.json, kilo.jsonc, opencode.json, opencode.jsonc; TUI files are a separate layer.
  os: linux
  path: ~/.config/kilo/tui.json
  scope: user
- format: jsonc
  notes: Default XDG_CONFIG_HOME/kilo directory on every OS; Windows source uses home/.config, not APPDATA. Main global load order is config.json, kilo.json, kilo.jsonc, opencode.json, opencode.jsonc; TUI files are a separate layer.
  os: linux
  path: ~/.config/kilo/tui.jsonc
  scope: user
- format: jsonc
  notes: Walked from launch directory to worktree boundary and merged ancestor-first; disabled by KILO_DISABLE_PROJECT_CONFIG.
  os: linux
  path: ./kilo.json
  scope: repo
- format: jsonc
  notes: Supported config-directory discovery; also examines the same directory names in the user home and primary checkout fallbacks for linked worktrees.
  os: linux
  path: ./.kilo/kilo.json
  scope: repo
- format: jsonc
  notes: Supported config-directory discovery; also examines the same directory names in the user home and primary checkout fallbacks for linked worktrees.
  os: linux
  path: ./.kilocode/kilo.json
  scope: repo
- format: jsonc
  notes: Walked from launch directory to worktree boundary and merged ancestor-first; disabled by KILO_DISABLE_PROJECT_CONFIG.
  os: linux
  path: ./kilo.jsonc
  scope: repo
- format: jsonc
  notes: Supported config-directory discovery; also examines the same directory names in the user home and primary checkout fallbacks for linked worktrees.
  os: linux
  path: ./.kilo/kilo.jsonc
  scope: repo
- format: jsonc
  notes: Supported config-directory discovery; also examines the same directory names in the user home and primary checkout fallbacks for linked worktrees.
  os: linux
  path: ./.kilocode/kilo.jsonc
  scope: repo
- format: jsonc
  notes: Walked from launch directory to worktree boundary and merged ancestor-first; disabled by KILO_DISABLE_PROJECT_CONFIG.
  os: linux
  path: ./opencode.json
  scope: repo
- format: jsonc
  notes: Supported config-directory discovery; also examines the same directory names in the user home and primary checkout fallbacks for linked worktrees.
  os: linux
  path: ./.kilo/opencode.json
  scope: repo
- format: jsonc
  notes: Supported config-directory discovery; also examines the same directory names in the user home and primary checkout fallbacks for linked worktrees.
  os: linux
  path: ./.kilocode/opencode.json
  scope: repo
- format: jsonc
  notes: Walked from launch directory to worktree boundary and merged ancestor-first; disabled by KILO_DISABLE_PROJECT_CONFIG.
  os: linux
  path: ./opencode.jsonc
  scope: repo
- format: jsonc
  notes: Supported config-directory discovery; also examines the same directory names in the user home and primary checkout fallbacks for linked worktrees.
  os: linux
  path: ./.kilo/opencode.jsonc
  scope: repo
- format: jsonc
  notes: Supported config-directory discovery; also examines the same directory names in the user home and primary checkout fallbacks for linked worktrees.
  os: linux
  path: ./.kilocode/opencode.jsonc
  scope: repo
- format: jsonc
  notes: Walked from launch directory to worktree boundary and merged ancestor-first; disabled by KILO_DISABLE_PROJECT_CONFIG.
  os: linux
  path: ./tui.json
  scope: repo
- format: jsonc
  notes: Supported config-directory discovery; also examines the same directory names in the user home and primary checkout fallbacks for linked worktrees.
  os: linux
  path: ./.kilo/tui.json
  scope: repo
- format: jsonc
  notes: Supported config-directory discovery; also examines the same directory names in the user home and primary checkout fallbacks for linked worktrees.
  os: linux
  path: ./.kilocode/tui.json
  scope: repo
- format: jsonc
  notes: Walked from launch directory to worktree boundary and merged ancestor-first; disabled by KILO_DISABLE_PROJECT_CONFIG.
  os: linux
  path: ./tui.jsonc
  scope: repo
- format: jsonc
  notes: Supported config-directory discovery; also examines the same directory names in the user home and primary checkout fallbacks for linked worktrees.
  os: linux
  path: ./.kilo/tui.jsonc
  scope: repo
- format: jsonc
  notes: Supported config-directory discovery; also examines the same directory names in the user home and primary checkout fallbacks for linked worktrees.
  os: linux
  path: ./.kilocode/tui.jsonc
  scope: repo
- format: jsonc
  notes: Enterprise-managed files override ordinary file and inline-content layers; directory can be redirected by KILO_TEST_MANAGED_CONFIG_DIR.
  os: linux
  path: /etc/kilo/kilo.jsonc
  scope: system
- format: jsonc
  notes: Enterprise-managed files override ordinary file and inline-content layers; directory can be redirected by KILO_TEST_MANAGED_CONFIG_DIR.
  os: linux
  path: /etc/kilo/kilo.json
  scope: system
- format: jsonc
  notes: Enterprise-managed files override ordinary file and inline-content layers; directory can be redirected by KILO_TEST_MANAGED_CONFIG_DIR.
  os: linux
  path: /etc/kilo/opencode.jsonc
  scope: system
- format: jsonc
  notes: Enterprise-managed files override ordinary file and inline-content layers; directory can be redirected by KILO_TEST_MANAGED_CONFIG_DIR.
  os: linux
  path: /etc/kilo/opencode.json
  scope: system
- format: jsonc
  notes: Explicit JSONC file loaded after global config, before project config.
  os: linux
  path: $KILO_CONFIG
  scope: env
- format: jsonc
  notes: Explicit TUI JSONC file loaded after global TUI, before project TUI.
  os: linux
  path: $KILO_TUI_CONFIG
  scope: env
- format: jsonc
  notes: Default XDG_CONFIG_HOME/kilo directory on every OS; Windows source uses home/.config, not APPDATA. Main global load order is config.json, kilo.json, kilo.jsonc, opencode.json, opencode.jsonc; TUI files are a separate layer.
  os: windows
  path: '%USERPROFILE%\.config\kilo\config.json'
  scope: user
- format: jsonc
  notes: Default XDG_CONFIG_HOME/kilo directory on every OS; Windows source uses home/.config, not APPDATA. Main global load order is config.json, kilo.json, kilo.jsonc, opencode.json, opencode.jsonc; TUI files are a separate layer.
  os: windows
  path: '%USERPROFILE%\.config\kilo\kilo.json'
  scope: user
- format: jsonc
  notes: Default XDG_CONFIG_HOME/kilo directory on every OS; Windows source uses home/.config, not APPDATA. Main global load order is config.json, kilo.json, kilo.jsonc, opencode.json, opencode.jsonc; TUI files are a separate layer.
  os: windows
  path: '%USERPROFILE%\.config\kilo\kilo.jsonc'
  scope: user
- format: jsonc
  notes: Default XDG_CONFIG_HOME/kilo directory on every OS; Windows source uses home/.config, not APPDATA. Main global load order is config.json, kilo.json, kilo.jsonc, opencode.json, opencode.jsonc; TUI files are a separate layer.
  os: windows
  path: '%USERPROFILE%\.config\kilo\opencode.json'
  scope: user
- format: jsonc
  notes: Default XDG_CONFIG_HOME/kilo directory on every OS; Windows source uses home/.config, not APPDATA. Main global load order is config.json, kilo.json, kilo.jsonc, opencode.json, opencode.jsonc; TUI files are a separate layer.
  os: windows
  path: '%USERPROFILE%\.config\kilo\opencode.jsonc'
  scope: user
- format: jsonc
  notes: Default XDG_CONFIG_HOME/kilo directory on every OS; Windows source uses home/.config, not APPDATA. Main global load order is config.json, kilo.json, kilo.jsonc, opencode.json, opencode.jsonc; TUI files are a separate layer.
  os: windows
  path: '%USERPROFILE%\.config\kilo\tui.json'
  scope: user
- format: jsonc
  notes: Default XDG_CONFIG_HOME/kilo directory on every OS; Windows source uses home/.config, not APPDATA. Main global load order is config.json, kilo.json, kilo.jsonc, opencode.json, opencode.jsonc; TUI files are a separate layer.
  os: windows
  path: '%USERPROFILE%\.config\kilo\tui.jsonc'
  scope: user
- format: jsonc
  notes: Walked from launch directory to worktree boundary and merged ancestor-first; disabled by KILO_DISABLE_PROJECT_CONFIG.
  os: windows
  path: .\kilo.json
  scope: repo
- format: jsonc
  notes: Supported config-directory discovery; also examines the same directory names in the user home and primary checkout fallbacks for linked worktrees.
  os: windows
  path: .\.kilo\kilo.json
  scope: repo
- format: jsonc
  notes: Supported config-directory discovery; also examines the same directory names in the user home and primary checkout fallbacks for linked worktrees.
  os: windows
  path: .\.kilocode\kilo.json
  scope: repo
- format: jsonc
  notes: Walked from launch directory to worktree boundary and merged ancestor-first; disabled by KILO_DISABLE_PROJECT_CONFIG.
  os: windows
  path: .\kilo.jsonc
  scope: repo
- format: jsonc
  notes: Supported config-directory discovery; also examines the same directory names in the user home and primary checkout fallbacks for linked worktrees.
  os: windows
  path: .\.kilo\kilo.jsonc
  scope: repo
- format: jsonc
  notes: Supported config-directory discovery; also examines the same directory names in the user home and primary checkout fallbacks for linked worktrees.
  os: windows
  path: .\.kilocode\kilo.jsonc
  scope: repo
- format: jsonc
  notes: Walked from launch directory to worktree boundary and merged ancestor-first; disabled by KILO_DISABLE_PROJECT_CONFIG.
  os: windows
  path: .\opencode.json
  scope: repo
- format: jsonc
  notes: Supported config-directory discovery; also examines the same directory names in the user home and primary checkout fallbacks for linked worktrees.
  os: windows
  path: .\.kilo\opencode.json
  scope: repo
- format: jsonc
  notes: Supported config-directory discovery; also examines the same directory names in the user home and primary checkout fallbacks for linked worktrees.
  os: windows
  path: .\.kilocode\opencode.json
  scope: repo
- format: jsonc
  notes: Walked from launch directory to worktree boundary and merged ancestor-first; disabled by KILO_DISABLE_PROJECT_CONFIG.
  os: windows
  path: .\opencode.jsonc
  scope: repo
- format: jsonc
  notes: Supported config-directory discovery; also examines the same directory names in the user home and primary checkout fallbacks for linked worktrees.
  os: windows
  path: .\.kilo\opencode.jsonc
  scope: repo
- format: jsonc
  notes: Supported config-directory discovery; also examines the same directory names in the user home and primary checkout fallbacks for linked worktrees.
  os: windows
  path: .\.kilocode\opencode.jsonc
  scope: repo
- format: jsonc
  notes: Walked from launch directory to worktree boundary and merged ancestor-first; disabled by KILO_DISABLE_PROJECT_CONFIG.
  os: windows
  path: .\tui.json
  scope: repo
- format: jsonc
  notes: Supported config-directory discovery; also examines the same directory names in the user home and primary checkout fallbacks for linked worktrees.
  os: windows
  path: .\.kilo\tui.json
  scope: repo
- format: jsonc
  notes: Supported config-directory discovery; also examines the same directory names in the user home and primary checkout fallbacks for linked worktrees.
  os: windows
  path: .\.kilocode\tui.json
  scope: repo
- format: jsonc
  notes: Walked from launch directory to worktree boundary and merged ancestor-first; disabled by KILO_DISABLE_PROJECT_CONFIG.
  os: windows
  path: .\tui.jsonc
  scope: repo
- format: jsonc
  notes: Supported config-directory discovery; also examines the same directory names in the user home and primary checkout fallbacks for linked worktrees.
  os: windows
  path: .\.kilo\tui.jsonc
  scope: repo
- format: jsonc
  notes: Supported config-directory discovery; also examines the same directory names in the user home and primary checkout fallbacks for linked worktrees.
  os: windows
  path: .\.kilocode\tui.jsonc
  scope: repo
- format: jsonc
  notes: Enterprise-managed files override ordinary file and inline-content layers; directory can be redirected by KILO_TEST_MANAGED_CONFIG_DIR.
  os: windows
  path: '%ProgramData%\kilo\kilo.jsonc'
  scope: system
- format: jsonc
  notes: Enterprise-managed files override ordinary file and inline-content layers; directory can be redirected by KILO_TEST_MANAGED_CONFIG_DIR.
  os: windows
  path: '%ProgramData%\kilo\kilo.json'
  scope: system
- format: jsonc
  notes: Enterprise-managed files override ordinary file and inline-content layers; directory can be redirected by KILO_TEST_MANAGED_CONFIG_DIR.
  os: windows
  path: '%ProgramData%\kilo\opencode.jsonc'
  scope: system
- format: jsonc
  notes: Enterprise-managed files override ordinary file and inline-content layers; directory can be redirected by KILO_TEST_MANAGED_CONFIG_DIR.
  os: windows
  path: '%ProgramData%\kilo\opencode.json'
  scope: system
- format: jsonc
  notes: Explicit JSONC file loaded after global config, before project config.
  os: windows
  path: '%KILO_CONFIG%'
  scope: env
- format: jsonc
  notes: Explicit TUI JSONC file loaded after global TUI, before project TUI.
  os: windows
  path: '%KILO_TUI_CONFIG%'
  scope: env
- format: jsonc
  notes: Home compatibility config directory, discovered separately from the default XDG user-config directory.
  os: macos
  path: ~/.kilo/kilo.json
  scope: user
- format: jsonc
  notes: Home compatibility config directory, discovered separately from the default XDG user-config directory.
  os: macos
  path: ~/.kilocode/kilo.json
  scope: user
- format: jsonc
  notes: Explicit config directory is an additional discovery layer for main/TUI configuration and shared resources.
  os: macos
  path: $KILO_CONFIG_DIR/kilo.json
  scope: env
- format: jsonc
  notes: Home compatibility config directory, discovered separately from the default XDG user-config directory.
  os: macos
  path: ~/.kilo/kilo.jsonc
  scope: user
- format: jsonc
  notes: Home compatibility config directory, discovered separately from the default XDG user-config directory.
  os: macos
  path: ~/.kilocode/kilo.jsonc
  scope: user
- format: jsonc
  notes: Explicit config directory is an additional discovery layer for main/TUI configuration and shared resources.
  os: macos
  path: $KILO_CONFIG_DIR/kilo.jsonc
  scope: env
- format: jsonc
  notes: Home compatibility config directory, discovered separately from the default XDG user-config directory.
  os: macos
  path: ~/.kilo/opencode.json
  scope: user
- format: jsonc
  notes: Home compatibility config directory, discovered separately from the default XDG user-config directory.
  os: macos
  path: ~/.kilocode/opencode.json
  scope: user
- format: jsonc
  notes: Explicit config directory is an additional discovery layer for main/TUI configuration and shared resources.
  os: macos
  path: $KILO_CONFIG_DIR/opencode.json
  scope: env
- format: jsonc
  notes: Home compatibility config directory, discovered separately from the default XDG user-config directory.
  os: macos
  path: ~/.kilo/opencode.jsonc
  scope: user
- format: jsonc
  notes: Home compatibility config directory, discovered separately from the default XDG user-config directory.
  os: macos
  path: ~/.kilocode/opencode.jsonc
  scope: user
- format: jsonc
  notes: Explicit config directory is an additional discovery layer for main/TUI configuration and shared resources.
  os: macos
  path: $KILO_CONFIG_DIR/opencode.jsonc
  scope: env
- format: jsonc
  notes: Home compatibility config directory, discovered separately from the default XDG user-config directory.
  os: macos
  path: ~/.kilo/tui.json
  scope: user
- format: jsonc
  notes: Home compatibility config directory, discovered separately from the default XDG user-config directory.
  os: macos
  path: ~/.kilocode/tui.json
  scope: user
- format: jsonc
  notes: Explicit config directory is an additional discovery layer for main/TUI configuration and shared resources.
  os: macos
  path: $KILO_CONFIG_DIR/tui.json
  scope: env
- format: jsonc
  notes: Home compatibility config directory, discovered separately from the default XDG user-config directory.
  os: macos
  path: ~/.kilo/tui.jsonc
  scope: user
- format: jsonc
  notes: Home compatibility config directory, discovered separately from the default XDG user-config directory.
  os: macos
  path: ~/.kilocode/tui.jsonc
  scope: user
- format: jsonc
  notes: Explicit config directory is an additional discovery layer for main/TUI configuration and shared resources.
  os: macos
  path: $KILO_CONFIG_DIR/tui.jsonc
  scope: env
- format: json
  notes: Legacy CLI authentication config may be read/migrated at bootstrap; not the current main config store.
  os: macos
  path: ~/.kilocode/cli/config.json
  scope: user
- format: json
  notes: Persisted provider authentication state; credentials are not included in this research. XDG_DATA_HOME redirects the base.
  os: macos
  path: ~/.local/share/kilo/auth.json
  scope: user
- format: jsonc
  notes: Home compatibility config directory, discovered separately from the default XDG user-config directory.
  os: linux
  path: ~/.kilo/kilo.json
  scope: user
- format: jsonc
  notes: Home compatibility config directory, discovered separately from the default XDG user-config directory.
  os: linux
  path: ~/.kilocode/kilo.json
  scope: user
- format: jsonc
  notes: Explicit config directory is an additional discovery layer for main/TUI configuration and shared resources.
  os: linux
  path: $KILO_CONFIG_DIR/kilo.json
  scope: env
- format: jsonc
  notes: Home compatibility config directory, discovered separately from the default XDG user-config directory.
  os: linux
  path: ~/.kilo/kilo.jsonc
  scope: user
- format: jsonc
  notes: Home compatibility config directory, discovered separately from the default XDG user-config directory.
  os: linux
  path: ~/.kilocode/kilo.jsonc
  scope: user
- format: jsonc
  notes: Explicit config directory is an additional discovery layer for main/TUI configuration and shared resources.
  os: linux
  path: $KILO_CONFIG_DIR/kilo.jsonc
  scope: env
- format: jsonc
  notes: Home compatibility config directory, discovered separately from the default XDG user-config directory.
  os: linux
  path: ~/.kilo/opencode.json
  scope: user
- format: jsonc
  notes: Home compatibility config directory, discovered separately from the default XDG user-config directory.
  os: linux
  path: ~/.kilocode/opencode.json
  scope: user
- format: jsonc
  notes: Explicit config directory is an additional discovery layer for main/TUI configuration and shared resources.
  os: linux
  path: $KILO_CONFIG_DIR/opencode.json
  scope: env
- format: jsonc
  notes: Home compatibility config directory, discovered separately from the default XDG user-config directory.
  os: linux
  path: ~/.kilo/opencode.jsonc
  scope: user
- format: jsonc
  notes: Home compatibility config directory, discovered separately from the default XDG user-config directory.
  os: linux
  path: ~/.kilocode/opencode.jsonc
  scope: user
- format: jsonc
  notes: Explicit config directory is an additional discovery layer for main/TUI configuration and shared resources.
  os: linux
  path: $KILO_CONFIG_DIR/opencode.jsonc
  scope: env
- format: jsonc
  notes: Home compatibility config directory, discovered separately from the default XDG user-config directory.
  os: linux
  path: ~/.kilo/tui.json
  scope: user
- format: jsonc
  notes: Home compatibility config directory, discovered separately from the default XDG user-config directory.
  os: linux
  path: ~/.kilocode/tui.json
  scope: user
- format: jsonc
  notes: Explicit config directory is an additional discovery layer for main/TUI configuration and shared resources.
  os: linux
  path: $KILO_CONFIG_DIR/tui.json
  scope: env
- format: jsonc
  notes: Home compatibility config directory, discovered separately from the default XDG user-config directory.
  os: linux
  path: ~/.kilo/tui.jsonc
  scope: user
- format: jsonc
  notes: Home compatibility config directory, discovered separately from the default XDG user-config directory.
  os: linux
  path: ~/.kilocode/tui.jsonc
  scope: user
- format: jsonc
  notes: Explicit config directory is an additional discovery layer for main/TUI configuration and shared resources.
  os: linux
  path: $KILO_CONFIG_DIR/tui.jsonc
  scope: env
- format: json
  notes: Legacy CLI authentication config may be read/migrated at bootstrap; not the current main config store.
  os: linux
  path: ~/.kilocode/cli/config.json
  scope: user
- format: json
  notes: Persisted provider authentication state; credentials are not included in this research. XDG_DATA_HOME redirects the base.
  os: linux
  path: ~/.local/share/kilo/auth.json
  scope: user
- format: jsonc
  notes: Home compatibility config directory, discovered separately from the default XDG user-config directory.
  os: windows
  path: '%USERPROFILE%\.kilo\kilo.json'
  scope: user
- format: jsonc
  notes: Home compatibility config directory, discovered separately from the default XDG user-config directory.
  os: windows
  path: '%USERPROFILE%\.kilocode\kilo.json'
  scope: user
- format: jsonc
  notes: Explicit config directory is an additional discovery layer for main/TUI configuration and shared resources.
  os: windows
  path: '%KILO_CONFIG_DIR%\kilo.json'
  scope: env
- format: jsonc
  notes: Home compatibility config directory, discovered separately from the default XDG user-config directory.
  os: windows
  path: '%USERPROFILE%\.kilo\kilo.jsonc'
  scope: user
- format: jsonc
  notes: Home compatibility config directory, discovered separately from the default XDG user-config directory.
  os: windows
  path: '%USERPROFILE%\.kilocode\kilo.jsonc'
  scope: user
- format: jsonc
  notes: Explicit config directory is an additional discovery layer for main/TUI configuration and shared resources.
  os: windows
  path: '%KILO_CONFIG_DIR%\kilo.jsonc'
  scope: env
- format: jsonc
  notes: Home compatibility config directory, discovered separately from the default XDG user-config directory.
  os: windows
  path: '%USERPROFILE%\.kilo\opencode.json'
  scope: user
- format: jsonc
  notes: Home compatibility config directory, discovered separately from the default XDG user-config directory.
  os: windows
  path: '%USERPROFILE%\.kilocode\opencode.json'
  scope: user
- format: jsonc
  notes: Explicit config directory is an additional discovery layer for main/TUI configuration and shared resources.
  os: windows
  path: '%KILO_CONFIG_DIR%\opencode.json'
  scope: env
- format: jsonc
  notes: Home compatibility config directory, discovered separately from the default XDG user-config directory.
  os: windows
  path: '%USERPROFILE%\.kilo\opencode.jsonc'
  scope: user
- format: jsonc
  notes: Home compatibility config directory, discovered separately from the default XDG user-config directory.
  os: windows
  path: '%USERPROFILE%\.kilocode\opencode.jsonc'
  scope: user
- format: jsonc
  notes: Explicit config directory is an additional discovery layer for main/TUI configuration and shared resources.
  os: windows
  path: '%KILO_CONFIG_DIR%\opencode.jsonc'
  scope: env
- format: jsonc
  notes: Home compatibility config directory, discovered separately from the default XDG user-config directory.
  os: windows
  path: '%USERPROFILE%\.kilo\tui.json'
  scope: user
- format: jsonc
  notes: Home compatibility config directory, discovered separately from the default XDG user-config directory.
  os: windows
  path: '%USERPROFILE%\.kilocode\tui.json'
  scope: user
- format: jsonc
  notes: Explicit config directory is an additional discovery layer for main/TUI configuration and shared resources.
  os: windows
  path: '%KILO_CONFIG_DIR%\tui.json'
  scope: env
- format: jsonc
  notes: Home compatibility config directory, discovered separately from the default XDG user-config directory.
  os: windows
  path: '%USERPROFILE%\.kilo\tui.jsonc'
  scope: user
- format: jsonc
  notes: Home compatibility config directory, discovered separately from the default XDG user-config directory.
  os: windows
  path: '%USERPROFILE%\.kilocode\tui.jsonc'
  scope: user
- format: jsonc
  notes: Explicit config directory is an additional discovery layer for main/TUI configuration and shared resources.
  os: windows
  path: '%KILO_CONFIG_DIR%\tui.jsonc'
  scope: env
- format: json
  notes: Legacy CLI authentication config may be read/migrated at bootstrap; not the current main config store.
  os: windows
  path: '%USERPROFILE%\.kilocode\cli\config.json'
  scope: user
- format: json
  notes: Persisted provider authentication state; credentials are not included in this research. XDG_DATA_HOME redirects the base.
  os: windows
  path: '%USERPROFILE%\.local\share\kilo\auth.json'
  scope: user
- format: other
  notes: macOS managed preference plist; first readable user-specific/system file is converted using plutil and applied last.
  os: macos
  path: /Library/Managed Preferences/<username>/ai.opencode.managed.plist
  scope: system
- format: other
  notes: macOS managed preference plist; first readable user-specific/system file is converted using plutil and applied last.
  os: macos
  path: /Library/Managed Preferences/ai.opencode.managed.plist
  scope: system
env_vars:
- effect: Unix home-directory input used by Node/Bun and XDG defaults.
  name: HOME
- effect: Windows home-directory input; XDG defaults are under this home.
  name: USERPROFILE
- effect: Choose the base directory containing kilo user configuration.
  name: XDG_CONFIG_HOME
- effect: Choose the base directory containing kilo auth, database, sessions, and logs.
  name: XDG_DATA_HOME
- effect: Choose the base directory containing kilo caches and helper binaries.
  name: XDG_CACHE_HOME
- effect: Choose the base directory containing kilo state and daemon metadata.
  name: XDG_STATE_HOME
- effect: Windows enterprise-managed configuration base; defaults to C:\ProgramData.
  name: ProgramData
- effect: Load an explicitly named configuration file after global configuration.
  name: KILO_CONFIG
- effect: Add an explicit directory for config, agent, command, skill, and plugin discovery; does not erase the normal global layer.
  name: KILO_CONFIG_DIR
- effect: Merge an inline JSONC config string after project/config-directory layers, before organization and managed policy layers.
  name: KILO_CONFIG_CONTENT
- effect: Load an explicitly named TUI config file.
  name: KILO_TUI_CONFIG
- effect: Skip project config files and project .kilo/.kilocode discovery when true or 1.
  name: KILO_DISABLE_PROJECT_CONFIG
- effect: Disable external plugins when true or 1; built-in plugins still appear in resolved config.
  name: KILO_PURE
- effect: npm launcher selects the supplied native executable instead of normal binary discovery.
  name: KILO_BIN_PATH
- effect: Choose tree-sitter WASM resource directory; npm launcher fills it from bundled resources if unset.
  name: KILO_TREE_SITTER_WASM_DIR
- effect: HTTP server/client Basic authentication password default.
  name: KILO_SERVER_PASSWORD
- effect: HTTP Basic authentication username default; client falls back to kilo.
  name: KILO_SERVER_USERNAME
- effect: Disable automatic updates when true or 1.
  name: KILO_DISABLE_AUTOUPDATE
- effect: Always show update notification when true or 1.
  name: KILO_ALWAYS_NOTIFY_UPDATE
- effect: Automatically share newly created sessions when true or 1.
  name: KILO_AUTO_SHARE
- effect: Enable Exa search; also enabled by KILO_EXPERIMENTAL_EXA or the grouped experimental switch.
  name: KILO_ENABLE_EXA
- effect: Compatibility experimental Exa search switch.
  name: KILO_EXPERIMENTAL_EXA
- effect: Enable experimental icon discovery.
  name: KILO_EXPERIMENTAL_ICON_DISCOVERY
- effect: Enable experimental oxfmt integration.
  name: KILO_EXPERIMENTAL_OXFMT
- effect: Enable experimental ty language-server integration.
  name: KILO_EXPERIMENTAL_LSP_TY
- effect: Enable experimental LSP tool.
  name: KILO_EXPERIMENTAL_LSP_TOOL
- effect: Enable experimental scout behavior.
  name: KILO_EXPERIMENTAL_SCOUT
- effect: The CLI sets this to 1 before command bootstrap so children can recognize an agent context.
  name: AGENT
- effect: The CLI sets this compatibility marker to 1 before bootstrap.
  name: OPENCODE
- effect: The CLI sets this marker to 1 during normal Kilo bootstrap.
  name: KILO
- effect: Disable automatic pruning of older tool outputs.
  name: KILO_DISABLE_PRUNE
- effect: Disable automatic context compaction.
  name: KILO_DISABLE_AUTOCOMPACT
- effect: Prevent terminal title changes.
  name: KILO_DISABLE_TERMINAL_TITLE
- effect: Disable TUI mouse handling.
  name: KILO_DISABLE_MOUSE
- effect: Disable automatic LSP downloads.
  name: KILO_DISABLE_LSP_DOWNLOAD
- effect: Disable default external plugin loading.
  name: KILO_DISABLE_DEFAULT_PLUGINS
- effect: Disable external skill discovery.
  name: KILO_DISABLE_EXTERNAL_SKILLS
- effect: Disable Claude Code compatibility prompt/skill discovery.
  name: KILO_DISABLE_CLAUDE_CODE
- effect: Disable Claude Code skill discovery independently.
  name: KILO_DISABLE_CLAUDE_CODE_SKILLS
- effect: Disable Claude Code prompt discovery independently; semantics belong to system-prompt.
  name: KILO_DISABLE_CLAUDE_CODE_PROMPT
- effect: Specify Windows Git Bash executable path; also used to locate less for session table paging.
  name: KILO_GIT_BASH_PATH
- effect: Override the SQLite database path.
  name: KILO_DB
- effect: Disable installation-channel-specific database naming.
  name: KILO_DISABLE_CHANNEL_DB
- effect: Skip database migrations; unsafe with incompatible databases.
  name: KILO_SKIP_MIGRATIONS
- effect: Enable strict configuration dependency handling.
  name: KILO_STRICT_CONFIG_DEPS
- effect: Override plugin metadata file location.
  name: KILO_PLUGIN_META_FILE
- effect: Set client identity; defaults to cli.
  name: KILO_CLIENT
- effect: Override retry limit with a positive integer; invalid/nonpositive values are ignored.
  name: KILO_SESSION_RETRY_LIMIT
- effect: Identify the workspace for runtime services.
  name: KILO_WORKSPACE_ID
- effect: Disable the embedded web UI.
  name: KILO_DISABLE_EMBEDDED_WEB_UI
- effect: Disable fff file finder; defaults disabled on Windows in 7.8.3.
  name: KILO_DISABLE_FFF
- effect: Enable the question tool.
  name: KILO_ENABLE_QUESTION_TOOL
- effect: Enable parallel execution; KILO_EXPERIMENTAL_PARALLEL also enables it.
  name: KILO_ENABLE_PARALLEL
- effect: Compatibility experimental switch for parallel execution.
  name: KILO_EXPERIMENTAL_PARALLEL
- effect: Enable grouped experimental features.
  name: KILO_EXPERIMENTAL
- effect: Enable experimental file watching (Effect boolean config).
  name: KILO_EXPERIMENTAL_FILEWATCHER
- effect: Disable experimental file watching (Effect boolean config).
  name: KILO_EXPERIMENTAL_DISABLE_FILEWATCHER
- effect: Disable copy-on-select; default true on Windows.
  name: KILO_EXPERIMENTAL_DISABLE_COPY_ON_SELECT
- effect: Override bash default timeout with a positive integer in milliseconds.
  name: KILO_EXPERIMENTAL_BASH_DEFAULT_TIMEOUT_MS
- effect: Override output-token ceiling with a positive integer.
  name: KILO_EXPERIMENTAL_OUTPUT_TOKEN_MAX
- effect: Enable experimental Markdown rendering unless false or 0.
  name: KILO_EXPERIMENTAL_MARKDOWN
- effect: Enable skill customization; defaults enabled for dev/beta/local channels unless false or 0.
  name: KILO_EXPERIMENTAL_CUSTOMIZE_SKILL
- effect: Enable experimental Claude migration.
  name: KILO_EXPERIMENTAL_CLAUDE_MIGRATION
- effect: Enable experimental workspaces; inherits KILO_EXPERIMENTAL unless explicitly set.
  name: KILO_EXPERIMENTAL_WORKSPACES
- effect: Enable experimental event system.
  name: KILO_EXPERIMENTAL_EVENT_SYSTEM
- effect: Enable experimental session switching.
  name: KILO_EXPERIMENTAL_SESSION_SWITCHING
- effect: Enable experimental session switcher; inherits KILO_EXPERIMENTAL unless explicitly set.
  name: KILO_EXPERIMENTAL_SESSION_SWITCHER
- effect: Enable experimental references; inherits KILO_EXPERIMENTAL unless explicitly set.
  name: KILO_EXPERIMENTAL_REFERENCES
- effect: Override Global.Path.home for compatibility discovery; does not move XDG bases already initialized from the real home.
  name: KILO_TEST_HOME
- effect: Override system-managed config directory; named as a test override in source.
  name: KILO_TEST_MANAGED_CONFIG_DIR
- effect: Used by run to resolve its working directory; wrappers should keep it consistent with the actual child cwd.
  name: PWD
machine_introspection:
- command: kilo debug config
  machine_readable: true
  notes: Observed valid JSON; contains merged configuration and may expose sensitive configuration.
  output_format: json
  purpose: config_dump
  useful_for_codegen: true
- command: kilo debug v2
  machine_readable: true
  notes: Observed providers/default/small JSON. In 7.8.3 default serialized an Effect object rather than a resolved model ID; do not trust that field.
  output_format: json
  purpose: models
  useful_for_codegen: true
- command: kilo debug skill
  machine_readable: true
  notes: Observed JSON array including full skill content; potentially sensitive and large.
  output_format: json
  purpose: capabilities
  useful_for_codegen: true
- command: kilo generate
  machine_readable: true
  notes: Hidden source-registered command emits HTTP OpenAPI, not JSON Schema for kilo.json and not CLI switches. Handler not run.
  output_format: json
  purpose: other
  useful_for_codegen: true
- command: kilo models --verbose
  machine_readable: false
  notes: Source prints model IDs plus JSON metadata blocks; not one JSON document.
  output_format: text
  purpose: models
  useful_for_codegen: true
- command: kilo session list --format json
  machine_readable: true
  notes: Observed empty stdout with exit 0 when no sessions; otherwise source emits JSON array. Avoid the table pager.
  output_format: json
  purpose: other
  useful_for_codegen: false
- command: kilo daemon status --json
  machine_readable: true
  notes: Observed JSON with running, stale, file, reason; safe state projection omits password.
  output_format: json
  purpose: other
  useful_for_codegen: false
- command: kilo db 'SELECT 1 AS probe' --format json
  machine_readable: true
  notes: Observed [{"probe":1}]; query mode accesses local SQLite. Keep wrapper queries fixed/read-only.
  output_format: json
  purpose: other
  useful_for_codegen: false
- command: kilo db path
  machine_readable: false
  notes: Observed resolved database path.
  output_format: text
  purpose: env
  useful_for_codegen: true
- command: kilo debug paths
  machine_readable: false
  notes: Observed text key/path lines after initialization; first simultaneous boot raced with other probes and failed a migration, sequential rerun succeeded.
  output_format: text
  purpose: env
  useful_for_codegen: true
- command: kilo config check
  machine_readable: false
  notes: Observed styled diagnostics and exit 1; not a JSON diagnostics protocol.
  output_format: text
  purpose: doctor
  useful_for_codegen: false
- command: kilo auth list
  machine_readable: false
  notes: Source renders credentials/provider names and environment-variable names; no JSON switch.
  output_format: text
  purpose: capabilities
  useful_for_codegen: false
- command: kilo mcp list
  machine_readable: false
  notes: Source renders status text; configured servers may connect/spawn.
  output_format: text
  purpose: mcp
  useful_for_codegen: false
- command: kilo profile --json
  machine_readable: true
  notes: Observed unauthenticated exit 1, empty stdout, styled stderr; JSON only on success.
  output_format: json
  purpose: other
  useful_for_codegen: false
- command: kilo cloud status --session-id ID --message-id ID
  machine_readable: true
  notes: Source prints authenticated remote-task status JSON; not executed.
  output_format: json
  purpose: other
  useful_for_codegen: false
- command: kilo cloud result --session-id ID --message-id ID
  machine_readable: true
  notes: Source prints authenticated remote-task result JSON; not executed.
  output_format: json
  purpose: other
  useful_for_codegen: false
wrapper_notes:
- Installed kilo --version is exactly 7.3.45; separately downloaded 7.8.3 native binary also reports 7.8.3. Gate compatibility by version; latest npm/GitHub release is 7.8.3.
- 'Use run with closed stdin and --format json for one-shot execution; --interactive/-i and root --mini require a TTY. No resume command exists: use run --continue/-c or --session/-s, with --fork or --cloud-fork as appropriate.'
- run --file/-f is greedy variadic, including after equals. Put -- before positional prompt text. Minimum zero is a confirmed fact that schema revision 2 cannot encode; variadic_min is conservatively unknown with an explicit gap.
- yargs short-option groups do not support arbitrary -mfoo values. Numeric or punctuation-leading attached values work; space or equals avoids grouped-letter ambiguity.
- Scalar flags usually allow parser-level omission; empty strings/undefined can still fail choices or handlers. Boolean flags can consume a following literal true/false or accept =true/=false; the none records describe standalone toggles.
- Boolean negations and camel-case aliases are accepted by the parser. Some handlers inspect literal argv spelling (for example network explicitness and print-logs), so parser acceptance does not guarantee equivalent downstream behavior.
- Bare root positional is a project path; --prompt supplies the initial user prompt. No system-prompt, append-system-prompt, or replace-system-prompt options occur in the examined declarations; dedicated semantics belong to system-prompt.
- Help is normally on stderr with branding and can include INFO lines on successful exit. NO_COLOR did not remove all styled diagnostic output. help --all --format md returned root help in both releases.
- Configuration/data initialization writes directories, database migrations, logs and telemetry state. Concurrent first boots into one empty home caused a migration failure; sequential initialization succeeded. Use a disposable home/XDG sandbox for probing.
- Windows uses XDG defaults under USERPROFILE/.config and .local/share, rather than APPDATA/LOCALAPPDATA. Enterprise configuration is separate under ProgramData/kilo.
- session list JSON mode can return empty stdout. debug v2 default in 7.8.3 is an unevaluated Effect object; do not assume it is a model ID.
- Long-running acp/serve/debug wait do not meet this contract's run-to-completion non_interactive criterion. Daemon launch returns unless foreground is selected; export/db require explicit arguments to avoid a picker/shell.
- cloud start/send, roll-call, github run and actual model sessions can incur charges. Plugin/import/session deletion/worktree commands mutate local state. None of these handlers was executed for research.
- npm launcher adds a Node process, forwards SIGINT/SIGTERM/SIGHUP and propagates exit status/signals; KILO_BIN_PATH can redirect it to another binary.
- ~/.kilo was checked and is absent on this host; home .kilo is only one config-discovery location, not the default user config store.
changes:
- Migrated the previous untyped revision-1 document to revision 2 with evidence-backed switch types, attachments, exact scopes, and explicit gaps.
- Verified installed 7.3.45 and current release 7.8.3; inventoried nested native commands and aliases and documented removed/added paths.
- Corrected file attachment from repeatable scalar to greedy variadic, Windows XDG paths, and long-running server classification.
- Recorded hidden compatibility switches, parser omission/short-attachment behavior, empty JSON session output, help-reference discrepancy, and configuration initialization caveats.
- Removed unsupported generic KILO_PROVIDER/KILO_<FIELD_NAME>/KILOCODE_<FIELD_NAME> claims; provider-endpoint variables belong to model-config.
requires_claudine_update: true
reason: Regenerate the Kilo switch catalog from revision 2; preserve greedy array semantics and the explicit zero-minimum/polymorphic gaps. Existing scalar-file assumptions would steal prompt arguments; latest-release command and config discovery differ from the previous research.
contract_checked: 2026-10-01
---

# Kilo Code CLI: Commands, Switch Parsing, and Wrapper Integration

## Overview

Kilo Code is an open-source coding agent shipped by Kilo Org for IDEs and the terminal. The terminal CLI is published as `@kilocode/cli` from the [Kilo repository](https://github.com/Kilo-Org/kilocode). Its primary entrypoint is `kilo`; npm also exposes `kilocode`. See the [product site](https://kilo.ai/), [documentation](https://kilo.ai/docs), [CLI guide](https://kilo.ai/docs/code-with-ai/platforms/cli), and [command reference](https://kilo.ai/docs/code-with-ai/platforms/cli-reference).

On 2026-10-01, the installed `kilo --version` returned exactly `7.3.45`. `npm view @kilocode/cli version repository bin --json` and the GitHub latest-release API both reported `7.8.3` (published 2026-10-01). A separately downloaded npm platform binary, without changing the installed package, returned `7.8.3`. Help was examined recursively for both releases; source was pinned to commits `67b815466c9ab3e022f16692988673712437b881` and `59f1428abb5fe782ee7bd4d258e72a08b74aadb4`. All execution probes used closed stdin and isolated home/XDG directories; no model sessions were started.

## Installation and Binaries

| OS | Command | Other installed names |
| --- | --- | --- |
| macos | kilo | `kilocode` |
| linux | kilo | `kilocode` |
| windows | kilo | `kilocode`, `kilo.cmd`, `kilocode.cmd`, `kilo.ps1`, `kilocode.ps1`, `kilo.exe` |

The [release package README](https://github.com/Kilo-Org/kilocode/blob/59f1428abb5fe782ee7bd4d258e72a08b74aadb4/packages/opencode/README.md) documents npm, direct npx execution, Homebrew, and prebuilt releases. The [npm launcher](https://github.com/Kilo-Org/kilocode/blob/59f1428abb5fe782ee7bd4d258e72a08b74aadb4/packages/opencode/bin/kilo) locates the platform binary and forwards signals. Windows npm shims and standalone `kilo.exe` are different launch layers.

| OS | Method | Invocation |
| --- | --- | --- |
| macos | npm | `npm install -g @kilocode/cli` |
| macos | other | `npx --package @kilocode/cli kilo` |
| macos | standalone_binary | `Download the matching asset from GitHub Releases` |
| macos | brew | `brew install Kilo-Org/tap/kilo` |
| linux | npm | `npm install -g @kilocode/cli` |
| linux | other | `npx --package @kilocode/cli kilo` |
| linux | standalone_binary | `Download the matching asset from GitHub Releases` |
| linux | brew | `brew install Kilo-Org/tap/kilo` |
| windows | npm | `npm install -g @kilocode/cli` |
| windows | other | `npx --package @kilocode/cli kilo` |
| windows | standalone_binary | `Download the matching asset from GitHub Releases` |

Release assets include macOS arm64/x64 ZIPs, Linux arm64/x64 tarballs with musl variants, and Windows arm64/x64 ZIPs. Baseline x64 builds support older CPUs without AVX. The earlier document listed curl, pnpm, Bun, and AUR installs; those were not established by the examined release README and are not retained as current verified methods.

## Subcommands

The following table lists native command paths and accepted aliases across the two examined releases. The bare executable is the interactive TUI and is deliberately absent. **Yes** means an explicit automation invocation can finish with no TTY, browser, or prompt; the notes identify arguments that are required to obtain that behavior. It does not mean read-only, free, or guaranteed success. Group-only paths are marked No, as are persistent servers. Release-gated developer commands and the internal background-process runner are not part of this public release inventory.

| Path after kilo | Finishes unattended | Purpose and constraints |
| --- | --- | --- |
| acp | No | Start ACP (Agent Client Protocol) server. Protocol server remains active; it does not run to completion. |
| agent | No | Manage agents. Command group; select a child path. |
| agent create | No | Create a new agent. Finishes with explicit arguments and closed stdin; help and source examined, handler not necessarily executed. |
| agent list | Yes | List all available agents. Finishes with explicit arguments and closed stdin; help and source examined, handler not necessarily executed. |
| attach | No | Attach to a running kilo server. Finishes with explicit arguments and closed stdin; help and source examined, handler not necessarily executed. |
| auth | No | Manage providers and credentials. Command group; select a child path. |
| auth list | Yes | List providers and credentials. Finishes with explicit arguments and closed stdin; help and source examined, handler not necessarily executed. |
| auth login | No | Log in to a provider. Finishes with explicit arguments and closed stdin; help and source examined, handler not necessarily executed. |
| auth logout | Yes | Log out from a configured provider. 7.8.3 accepts an explicit provider to avoid selection; 7.3.45 has an interactive provider picker. |
| auth ls | Yes | List providers and credentials. Alias of auth list. Finishes with explicit arguments and closed stdin; help and source examined, handler not necessarily executed. |
| cloud | No | Run Cloud Agent tasks. Command group; select a child path. |
| cloud result | Yes | Show a Cloud Agent task result. 7.8.3 only; requires --session-id and --message-id and authenticated network access. |
| cloud send | Yes | Send a follow-up prompt to a Cloud Agent task. 7.8.3 only; sends a follow-up to a paid task, requiring --session-id and a prompt input. Not executed. |
| cloud start | Yes | Start a Cloud Agent task. 7.8.3 only; starts a paid remote task. Provide --prompt or --prompt-stdin; --stream follows JSONL events. Not executed. |
| cloud status | Yes | Show Cloud Agent task status. 7.8.3 only; requires --session-id and --message-id and authenticated network access. |
| completion | Yes | Generate shell completion script. Shell completion generator; internal --get-yargs-completions is a shell callback, not an agent session. |
| config | No | Configuration tools. Command group; select a child path. |
| config check | Yes | Check configuration for warnings and errors. Finishes with explicit arguments and closed stdin; help and source examined, handler not necessarily executed. |
| console | No | Open or stop the local Kilo Console (deprecated). Finishes with explicit arguments and closed stdin; help and source examined, handler not necessarily executed. |
| console stop | Yes | Stop the daemon behind Kilo Console. 7.8.3 only; stops the daemon behind the deprecated Console without opening a browser. |
| daemon | Yes | Manage the local kilo daemon. 7.8.3 defaults to starting a detached daemon and returns; --foreground/-f keeps the caller active. 7.3.45 required a child command. |
| daemon restart | Yes | Restart the local kilo daemon. Returns after restarting detached daemon; --foreground stays active. Mutates daemon state. |
| daemon start | Yes | Start or reuse the detached local daemon. Returns after starting/reusing a detached daemon; --foreground (7.8.3 alias -f) stays active. Mutates daemon state. |
| daemon status | Yes | Show local kilo daemon status. Reports daemon state; --json is suitable for wrappers. |
| daemon stop | Yes | Stop the local kilo daemon. Stops the daemon and returns; --json added in 7.8.3. |
| db | Yes | Database tools. Supply a nonempty SQL query; omission spawns the interactive sqlite3 shell. --format json is available. |
| db migrate | Yes | Migrate JSON data to SQLite (merges with existing data). 7.3.45 only; removed from the 7.8.3 registered command surface. Finishes with explicit arguments and closed stdin; help and source examined, handler not necessarily executed. |
| db path | Yes | Print the database path. Finishes with explicit arguments and closed stdin; help and source examined, handler not necessarily executed. |
| debug | No | Debugging and troubleshooting tools. Command group; select a child path. |
| debug agent | Yes | Show agent configuration details. Prints agent configuration; --tool executes a real tool using --params and may mutate files. |
| debug config | Yes | Show resolved configuration. Finishes with explicit arguments and closed stdin; help and source examined, handler not necessarily executed. |
| debug file | No | File system debugging utilities. Command group; select a child path. |
| debug file list | Yes | List files in a directory. Finishes with explicit arguments and closed stdin; help and source examined, handler not necessarily executed. |
| debug file read | Yes | Read file contents as JSON. Finishes with explicit arguments and closed stdin; help and source examined, handler not necessarily executed. |
| debug file search | Yes | Search files by query. Finishes with explicit arguments and closed stdin; help and source examined, handler not necessarily executed. |
| debug file status | Yes | Show file status information. 7.3.45 only; removed from the 7.8.3 registered command surface. Finishes with explicit arguments and closed stdin; help and source examined, handler not necessarily executed. |
| debug file tree | Yes | Show directory tree. 7.3.45 only; removed from the 7.8.3 registered command surface. Finishes with explicit arguments and closed stdin; help and source examined, handler not necessarily executed. |
| debug info | Yes | Show debug information. Finishes with explicit arguments and closed stdin; help and source examined, handler not necessarily executed. |
| debug lsp | No | LSP debugging utilities. Command group; select a child path. |
| debug lsp diagnostics | Yes | Get diagnostics for a file. Finishes with explicit arguments and closed stdin; help and source examined, handler not necessarily executed. |
| debug lsp document-symbols | Yes | Get symbols from a document. Finishes with explicit arguments and closed stdin; help and source examined, handler not necessarily executed. |
| debug lsp symbols | Yes | Search workspace symbols. Finishes with explicit arguments and closed stdin; help and source examined, handler not necessarily executed. |
| debug paths | Yes | Show global paths (data, config, cache, state). Finishes with explicit arguments and closed stdin; help and source examined, handler not necessarily executed. |
| debug rg | No | Ripgrep debugging utilities. Command group; select a child path. |
| debug rg files | Yes | List files using ripgrep. Finishes with explicit arguments and closed stdin; help and source examined, handler not necessarily executed. |
| debug rg search | Yes | Search file contents using ripgrep. Finishes with explicit arguments and closed stdin; help and source examined, handler not necessarily executed. |
| debug rg tree | Yes | Show file tree using ripgrep. 7.3.45 only; removed from the 7.8.3 registered command surface. Finishes with explicit arguments and closed stdin; help and source examined, handler not necessarily executed. |
| debug scrap | Yes | List all known projects. Finishes with explicit arguments and closed stdin; help and source examined, handler not necessarily executed. |
| debug skill | Yes | List all available skills. Finishes with explicit arguments and closed stdin; help and source examined, handler not necessarily executed. |
| debug snapshot | No | Snapshot debugging utilities. Command group; select a child path. |
| debug snapshot diff | Yes | Show diff for a snapshot hash. Finishes with explicit arguments and closed stdin; help and source examined, handler not necessarily executed. |
| debug snapshot patch | Yes | Show patch for a snapshot hash. Finishes with explicit arguments and closed stdin; help and source examined, handler not necessarily executed. |
| debug snapshot track | Yes | Track current snapshot state. Finishes with explicit arguments and closed stdin; help and source examined, handler not necessarily executed. |
| debug startup | Yes | Print startup timing. Finishes with explicit arguments and closed stdin; help and source examined, handler not necessarily executed. |
| debug v2 | Yes | Debug v2 catalog and built-in plugins. Finishes with explicit arguments and closed stdin; help and source examined, handler not necessarily executed. |
| debug wait | No | Wait indefinitely (for debugging). Waits indefinitely (source sleeps for a day); excluded from completion-based automation. |
| export | Yes | Export session data as JSON. Supply sessionID; omission opens an interactive session picker when sessions exist. |
| generate | Yes | Emit the HTTP OpenAPI description as JSON. Registered but hidden from root help; emits HTTP OpenAPI JSON, not the CLI/configuration schema. |
| github | No | Manage GitHub agent. Command group; select a child path. |
| github install | No | Install the GitHub agent. Finishes with explicit arguments and closed stdin; help and source examined, handler not necessarily executed. |
| github run | Yes | Run the GitHub agent. Runs the CI GitHub agent with existing credentials/event context; may execute models and modify a checkout. Not executed. |
| help | Yes | Show full CLI reference. Source declares --all and --format md\|text; both releases actually returned only root help in the tested full-reference invocation. |
| import | Yes | Import session data from JSON file or URL. Imports supplied file/share URL into session storage without a picker; writes local data. |
| mcp | No | Manage MCP (Model Context Protocol) servers. Command group; select a child path. |
| mcp add | Yes | Add an MCP server. 7.8.3 supports explicit name plus --url or a local command after -- without prompts; otherwise uses a setup wizard. Earlier version uses a wizard. |
| mcp auth | No | Authenticate with an OAuth-enabled MCP server. Finishes with explicit arguments and closed stdin; help and source examined, handler not necessarily executed. |
| mcp auth list | Yes | List OAuth-capable MCP servers and their authentication state. Text OAuth status report; does not perform the browser authorization flow. |
| mcp auth ls | Yes | List OAuth-capable MCP servers and their authentication state. Alias of mcp auth list. Text OAuth status report; does not perform the browser authorization flow. |
| mcp debug | No | Debug OAuth connection for an MCP server. Finishes with explicit arguments and closed stdin; help and source examined, handler not necessarily executed. |
| mcp list | Yes | List MCP servers and their status. Text status report; configured servers may be contacted or started. |
| mcp logout | Yes | Remove OAuth credentials for an MCP server. Supply a server name to avoid selection; removes OAuth credentials. |
| mcp ls | Yes | List MCP servers and their status. Alias of mcp list. Text status report; configured servers may be contacted or started. |
| models | Yes | List all available models. Finishes with explicit arguments and closed stdin; help and source examined, handler not necessarily executed. |
| plug | Yes | Install plugin and update config. Alias of plugin. Installs packages and edits configuration; no confirmation in the source handler. Alias plug. |
| plugin | Yes | Install plugin and update config. Installs packages and edits configuration; no confirmation in the source handler. Alias plug. |
| pr | No | Manage pull requests. 7.8.3 command group; 7.3.45 pr <number> checked out a branch and launched the TUI. |
| pr checkout | No | Fetch and checkout a GitHub PR branch, then run kilo. Finishes with explicit arguments and closed stdin; help and source examined, handler not necessarily executed. |
| pr link | Yes | Link a session to a pull request. 7.8.3 only; sets PR metadata for --session/-s or a resolved session. |
| pr status | Yes | Show a session's linked pull request. 7.8.3 only; reports linked PR metadata for --session/-s or a resolved session. |
| pr unlink | Yes | Clear a session's linked pull request. 7.8.3 only; clears PR metadata for --session/-s or a resolved session. |
| profile | Yes | Show Kilo account profile. --json emits JSON on success; unauthenticated state exits 1 with styled stderr. |
| providers | No | Manage providers and credentials. Alias of auth. Command group; select a child path. |
| providers list | Yes | List providers and credentials. Alias of auth list. Finishes with explicit arguments and closed stdin; help and source examined, handler not necessarily executed. |
| providers login | No | Log in to a provider. Alias of auth login. Finishes with explicit arguments and closed stdin; help and source examined, handler not necessarily executed. |
| providers logout | Yes | Log out from a configured provider. Alias of auth logout. 7.8.3 accepts an explicit provider to avoid selection; 7.3.45 has an interactive provider picker. |
| providers ls | Yes | List providers and credentials. Alias of auth list. Finishes with explicit arguments and closed stdin; help and source examined, handler not necessarily executed. |
| remote | No | Enable remote connection for real-time session relay. Finishes with explicit arguments and closed stdin; help and source examined, handler not necessarily executed. |
| roll-call | Yes | Batch-test text models matching a filter for connectivity and latency. Runs live model calls and may incur charges; not executed during this research. |
| run | Yes | Run kilo with a message. One-shot mode with closed stdin; --interactive/-i requires a TTY. Resume via --continue/-c or --session/-s; no native resume subcommand. |
| serve | No | Starts a headless kilo server. HTTP server remains active until terminated. |
| session | No | Manage sessions. Command group; select a child path. |
| session delete | Yes | Delete a session. Deletes the explicitly named session without confirmation. |
| session list | Yes | List sessions. Use --format json or pipe stdout to avoid the table pager; an empty result is empty stdout, not []. |
| stats | Yes | Show token usage and cost statistics. Finishes with explicit arguments and closed stdin; help and source examined, handler not necessarily executed. |
| uninstall | Yes | Uninstall kilo and remove all related files. Use --dry-run or --force to avoid the confirmation prompt. Destructive without --dry-run. |
| upgrade | No | Upgrade kilo to the latest or a specific version. Pass --method to avoid install-method selection; package managers may still prompt or require elevated privileges. |
| web | No | Start kilo server and open web interface. 7.3.45 only; removed from the 7.8.3 registered command surface. Finishes with explicit arguments and closed stdin; help and source examined, handler not necessarily executed. |
| worktree | No | Manage git worktrees. Command group; select a child path. |
| worktree create | Yes | Create (or reuse) a git worktree by name. 7.8.3 only; creates/reuses a named git worktree without a prompt. |
| worktree list | Yes | List git worktrees for the current project. 7.8.3 only; prints worktree state. |
| worktree remove | Yes | Remove a named git worktree and its branch. 7.8.3 only; removes the worktree and its branch without a confirmation. Not executed. |

Every path in this table has its canonical help examined; alias resolution was checked through help and declarations. `generate` is registered in source despite being absent from displayed root help. Removed paths were confirmed by source registration, because asking an unknown nested path for help can misleadingly print parent help with exit 0. There is no `resume` path; resumption uses `run` switches.

## CLI Switch Inventory

The inventory covers the root and every path marked Yes above, including aliases, query/export automation forms, and run resumption. Interactive paths additionally receive the five global switches; their other switches are outside the required inventory. Records for removed commands explicitly apply to 7.3.45 only. Root and run hidden compatibility options were also read from source.

Parsing is [yargs 18.0.0](https://github.com/yargs/yargs/tree/v18.0.0) with [yargs-parser 22.0.0](https://github.com/yargs/yargs-parser/blob/v22.0.0/lib/yargs-parser.ts), pinned in the release manifests and lockfile. Kilo sets `populate--: true` and enables strict validation. Parser defaults retain greedy arrays, short-option groups, boolean negation, and camel-case expansion. The Kilo declaration establishes each switch type; parser source establishes its written forms.

| Type/form | Established behavior |
| --- | --- |
| none | Standalone boolean toggle; also accepts an explicit =true/=false or consumes a following literal true/false. Negations are recorded separately. |
| string / number | At most one following value, or --flag=value. No requiresArg/nargs is declared for the inventoried scalar options; defaultValue permits omission, but choices without a default reject it. Other handlers can also reject empty values. |
| variadic | One occurrence consumes multiple non-option words. Equals does not stop greediness. Bare array options produce [], a zero minimum that revision 2 cannot encode; variadic_min remains unknown with the exact limitation. |
| short_attached | Accepted only for compatible numeric/punctuation-leading values, not arbitrary alphabetic text; -mfoo is a grouped-letter parse, while -m123 and -f/path consume attached values. -m=foo is equals. |
| unknown | stats --models is undeclared/polymorphic: bare true, numeric input number, other input string. The schema cannot encode that union, so it remains unknown. |

A disposable fixture replayed the extracted `run` builder with the exact yargs 18.0.0 dependency graph and no provider handler. It produced `file:["a","b","prompt"]` from `--file a b prompt`, `file:["a","b"]` from `--file=a b`, and `file:[]` from bare `--file`. Native `7.8.3` rejected missing-file probes by naming the consumed path for space, equals, and `-f/path`. Native `run --replayLimit=-2` reached the handler and produced `--replay-limit requires --mini`, confirming the camel-case spelling and equals number parse without a model call. Help/version early exits were not used to prove consumption.

### Global switches

| Switch | Aliases | Value |
| --- | --- | --- |
| --help | -h, --h | none |
| --version | -v, --v | none |
| --print-logs | --printLogs | none |
| --log-level | --logLevel | string |
| --pure | — | none |
| --no-help | --no-h | none |
| --no-version | --no-v | none |
| --no-print-logs | --no-printLogs | none |
| --no-log-level | --no-logLevel | none |
| --no-pure | — | none |

### Command-scoped switches

Each row cites a pinned declaration. Space/equals are accepted for scalar/array rows; short attachments are listed only where a one-character alias exists and have the restrictions above. Scalar omission is accepted unless choices without a default reject it: --log-level and --repo-type require a value. Frontmatter records value_optional individually. Parser-generated boolean negations are listed separately below.

| Exact command path | Switch and aliases | Type | Attachment | Purpose and declaration |
| --- | --- | --- | --- | --- |
| root, completion, daemon, daemon restart, daemon start | --port | number | space, equals | port to listen on [src/cli/cmd/tui.ts](https://github.com/Kilo-Org/kilocode/blob/59f1428abb5fe782ee7bd4d258e72a08b74aadb4/packages/opencode/src/cli/cmd/tui.ts) |
| root, completion, daemon, daemon restart, daemon start | --hostname | string | space, equals | hostname to listen on [src/cli/cmd/tui.ts](https://github.com/Kilo-Org/kilocode/blob/59f1428abb5fe782ee7bd4d258e72a08b74aadb4/packages/opencode/src/cli/cmd/tui.ts) |
| root, completion, daemon, daemon restart, daemon start | --mdns | none | — | enable mDNS service discovery (defaults hostname to 0.0.0.0) [src/cli/cmd/tui.ts](https://github.com/Kilo-Org/kilocode/blob/59f1428abb5fe782ee7bd4d258e72a08b74aadb4/packages/opencode/src/cli/cmd/tui.ts) |
| root, completion, daemon, daemon restart, daemon start | --mdns-domain / --mdnsDomain | string | space, equals | custom domain name for mDNS service (default: kilo.local) [src/cli/cmd/tui.ts](https://github.com/Kilo-Org/kilocode/blob/59f1428abb5fe782ee7bd4d258e72a08b74aadb4/packages/opencode/src/cli/cmd/tui.ts) |
| root, completion, daemon, daemon restart, daemon start | --cors | variadic | space, equals | additional domains to allow for CORS [src/cli/cmd/tui.ts](https://github.com/Kilo-Org/kilocode/blob/59f1428abb5fe782ee7bd4d258e72a08b74aadb4/packages/opencode/src/cli/cmd/tui.ts) |
| root, completion, run | --model / -m, --m | string | space, equals, short_attached | model to use in the format of provider/model [src/cli/cmd/tui.ts](https://github.com/Kilo-Org/kilocode/blob/59f1428abb5fe782ee7bd4d258e72a08b74aadb4/packages/opencode/src/cli/cmd/tui.ts) |
| root, completion, run | --continue / -c, --c | none | — | continue the last session [src/cli/cmd/tui.ts](https://github.com/Kilo-Org/kilocode/blob/59f1428abb5fe782ee7bd4d258e72a08b74aadb4/packages/opencode/src/cli/cmd/tui.ts) |
| root, completion, run | --session / -s, --s | string | space, equals, short_attached | session id to continue [src/cli/cmd/tui.ts](https://github.com/Kilo-Org/kilocode/blob/59f1428abb5fe782ee7bd4d258e72a08b74aadb4/packages/opencode/src/cli/cmd/tui.ts) |
| root, completion | --fork | none | — | fork the session when continuing (use with --continue or --session) [src/cli/cmd/tui.ts](https://github.com/Kilo-Org/kilocode/blob/59f1428abb5fe782ee7bd4d258e72a08b74aadb4/packages/opencode/src/cli/cmd/tui.ts) |
| root, completion, run | --cloud-fork / --cloudFork | none | — | fetch session from cloud and continue locally (use with --session) [src/cli/cmd/tui.ts](https://github.com/Kilo-Org/kilocode/blob/59f1428abb5fe782ee7bd4d258e72a08b74aadb4/packages/opencode/src/cli/cmd/tui.ts) |
| root, completion | --worktree | string | space, equals | create (or reuse) a git worktree with this name and start kilo there [src/cli/cmd/tui.ts](https://github.com/Kilo-Org/kilocode/blob/59f1428abb5fe782ee7bd4d258e72a08b74aadb4/packages/opencode/src/cli/cmd/tui.ts) |
| root, completion | --prompt | string | space, equals | prompt to use [src/cli/cmd/tui.ts](https://github.com/Kilo-Org/kilocode/blob/59f1428abb5fe782ee7bd4d258e72a08b74aadb4/packages/opencode/src/cli/cmd/tui.ts) |
| root, completion, run | --agent | string | space, equals | agent to use [src/cli/cmd/tui.ts](https://github.com/Kilo-Org/kilocode/blob/59f1428abb5fe782ee7bd4d258e72a08b74aadb4/packages/opencode/src/cli/cmd/tui.ts) |
| root, completion, run | --auto | none | — | auto-approve permissions that are not explicitly denied (dangerous!) [src/cli/cmd/tui.ts](https://github.com/Kilo-Org/kilocode/blob/59f1428abb5fe782ee7bd4d258e72a08b74aadb4/packages/opencode/src/cli/cmd/tui.ts) |
| root, completion | --mini | none | — | start the minimal interactive interface [src/cli/cmd/tui.ts](https://github.com/Kilo-Org/kilocode/blob/59f1428abb5fe782ee7bd4d258e72a08b74aadb4/packages/opencode/src/cli/cmd/tui.ts) |
| root | --no-replay / --noReplay | none | — | disable mini session history replay on resume and after resize [src/cli/cmd/tui.ts](https://github.com/Kilo-Org/kilocode/blob/59f1428abb5fe782ee7bd4d258e72a08b74aadb4/packages/opencode/src/cli/cmd/tui.ts) |
| root, completion | --replay-limit / --replayLimit | number | space, equals | cap visible mini replay to the newest N messages [src/cli/cmd/tui.ts](https://github.com/Kilo-Org/kilocode/blob/59f1428abb5fe782ee7bd4d258e72a08b74aadb4/packages/opencode/src/cli/cmd/tui.ts) |
| root, run | --yolo | none | — | Compatibility permission bypass switch. [src/cli/cmd/tui.ts](https://github.com/Kilo-Org/kilocode/blob/59f1428abb5fe782ee7bd4d258e72a08b74aadb4/packages/opencode/src/cli/cmd/tui.ts) |
| root, run | --dangerously-skip-permissions / --dangerouslySkipPermissions | none | — | Compatibility permission bypass switch. [src/cli/cmd/tui.ts](https://github.com/Kilo-Org/kilocode/blob/59f1428abb5fe782ee7bd4d258e72a08b74aadb4/packages/opencode/src/cli/cmd/tui.ts) |
| root, run | --replay | none | — | Control interactive history replay. [src/cli/cmd/tui.ts](https://github.com/Kilo-Org/kilocode/blob/59f1428abb5fe782ee7bd4d258e72a08b74aadb4/packages/opencode/src/cli/cmd/tui.ts) |
| root, run | --demo | none | — | Enable direct interactive demo slash commands. [src/cli/cmd/tui.ts](https://github.com/Kilo-Org/kilocode/blob/59f1428abb5fe782ee7bd4d258e72a08b74aadb4/packages/opencode/src/cli/cmd/tui.ts) |
| cloud result, cloud send, cloud status | --session-id / --sessionId | string | space, equals | Cloud Agent session ID [src/kilocode/cli/cmd/cloud.ts](https://github.com/Kilo-Org/kilocode/blob/59f1428abb5fe782ee7bd4d258e72a08b74aadb4/packages/opencode/src/kilocode/cli/cmd/cloud.ts) |
| cloud result, cloud status | --message-id / --messageId | string | space, equals | Cloud Agent message ID [src/kilocode/cli/cmd/cloud.ts](https://github.com/Kilo-Org/kilocode/blob/59f1428abb5fe782ee7bd4d258e72a08b74aadb4/packages/opencode/src/kilocode/cli/cmd/cloud.ts) |
| cloud send, cloud start | --prompt | string | space, equals | prompt for the Cloud Agent [src/kilocode/cli/cmd/cloud.ts](https://github.com/Kilo-Org/kilocode/blob/59f1428abb5fe782ee7bd4d258e72a08b74aadb4/packages/opencode/src/kilocode/cli/cmd/cloud.ts) |
| cloud send, cloud start | --prompt-stdin / --promptStdin | none | — | read the prompt from standard input [src/kilocode/cli/cmd/cloud.ts](https://github.com/Kilo-Org/kilocode/blob/59f1428abb5fe782ee7bd4d258e72a08b74aadb4/packages/opencode/src/kilocode/cli/cmd/cloud.ts) |
| cloud start | --repo | string | space, equals | repository shorthand or URL [src/kilocode/cli/cmd/cloud.ts](https://github.com/Kilo-Org/kilocode/blob/59f1428abb5fe782ee7bd4d258e72a08b74aadb4/packages/opencode/src/kilocode/cli/cmd/cloud.ts) |
| cloud start | --repo-type / --repoType | string | space, equals | repository provider type [src/kilocode/cli/cmd/cloud.ts](https://github.com/Kilo-Org/kilocode/blob/59f1428abb5fe782ee7bd4d258e72a08b74aadb4/packages/opencode/src/kilocode/cli/cmd/cloud.ts) |
| cloud start | --branch | string | space, equals | repository branch [src/kilocode/cli/cmd/cloud.ts](https://github.com/Kilo-Org/kilocode/blob/59f1428abb5fe782ee7bd4d258e72a08b74aadb4/packages/opencode/src/kilocode/cli/cmd/cloud.ts) |
| cloud start | --model | string | space, equals | Cloud Agent model [src/kilocode/cli/cmd/cloud.ts](https://github.com/Kilo-Org/kilocode/blob/59f1428abb5fe782ee7bd4d258e72a08b74aadb4/packages/opencode/src/kilocode/cli/cmd/cloud.ts) |
| cloud start | --mode | string | space, equals | Cloud Agent mode [src/kilocode/cli/cmd/cloud.ts](https://github.com/Kilo-Org/kilocode/blob/59f1428abb5fe782ee7bd4d258e72a08b74aadb4/packages/opencode/src/kilocode/cli/cmd/cloud.ts) |
| cloud start | --org-id / --orgId | string | space, equals | Kilo organization ID [src/kilocode/cli/cmd/cloud.ts](https://github.com/Kilo-Org/kilocode/blob/59f1428abb5fe782ee7bd4d258e72a08b74aadb4/packages/opencode/src/kilocode/cli/cmd/cloud.ts) |
| cloud start | --stream | none | — | connect to the WebSocket stream and print events as JSONL [src/kilocode/cli/cmd/cloud.ts](https://github.com/Kilo-Org/kilocode/blob/59f1428abb5fe782ee7bd4d258e72a08b74aadb4/packages/opencode/src/kilocode/cli/cmd/cloud.ts) |
| completion | --no-replay | none | — | disable mini session history replay on resume and after resize [src/cli/cmd/tui.ts](https://github.com/Kilo-Org/kilocode/blob/59f1428abb5fe782ee7bd4d258e72a08b74aadb4/packages/opencode/src/cli/cmd/tui.ts) |
| console stop, daemon, daemon restart, daemon start, daemon status, daemon stop | --json | none | — | print daemon details as JSON [src/kilocode/cli/cmd/console.ts](https://github.com/Kilo-Org/kilocode/blob/59f1428abb5fe782ee7bd4d258e72a08b74aadb4/packages/opencode/src/kilocode/cli/cmd/console.ts) |
| daemon, daemon restart, daemon start | --foreground / -f, --f | none | — | keep the command active until interrupted [src/kilocode/cli/cmd/daemon.ts](https://github.com/Kilo-Org/kilocode/blob/59f1428abb5fe782ee7bd4d258e72a08b74aadb4/packages/opencode/src/kilocode/cli/cmd/daemon.ts) |
| db | --format | string | space, equals | Output format [src/cli/cmd/db.ts](https://github.com/Kilo-Org/kilocode/blob/59f1428abb5fe782ee7bd4d258e72a08b74aadb4/packages/opencode/src/cli/cmd/db.ts) |
| debug agent | --tool | string | space, equals | Tool id to execute [src/cli/cmd/debug/agent.ts](https://github.com/Kilo-Org/kilocode/blob/59f1428abb5fe782ee7bd4d258e72a08b74aadb4/packages/opencode/src/cli/cmd/debug/agent.ts) |
| debug agent | --params | string | space, equals | Tool params as JSON or a JS object literal [src/cli/cmd/debug/agent.ts](https://github.com/Kilo-Org/kilocode/blob/59f1428abb5fe782ee7bd4d258e72a08b74aadb4/packages/opencode/src/cli/cmd/debug/agent.ts) |
| debug rg files | --query | string | space, equals | Filter files by query [src/cli/cmd/debug/ripgrep.ts](https://github.com/Kilo-Org/kilocode/blob/59f1428abb5fe782ee7bd4d258e72a08b74aadb4/packages/opencode/src/cli/cmd/debug/ripgrep.ts) |
| debug rg files | --glob | string | space, equals | Glob pattern to match files [src/cli/cmd/debug/ripgrep.ts](https://github.com/Kilo-Org/kilocode/blob/59f1428abb5fe782ee7bd4d258e72a08b74aadb4/packages/opencode/src/cli/cmd/debug/ripgrep.ts) |
| debug rg files, debug rg search | --limit | number | space, equals | Limit number of results [src/cli/cmd/debug/ripgrep.ts](https://github.com/Kilo-Org/kilocode/blob/59f1428abb5fe782ee7bd4d258e72a08b74aadb4/packages/opencode/src/cli/cmd/debug/ripgrep.ts) |
| debug rg search | --glob | variadic | space, equals | File glob patterns [src/cli/cmd/debug/ripgrep.ts](https://github.com/Kilo-Org/kilocode/blob/59f1428abb5fe782ee7bd4d258e72a08b74aadb4/packages/opencode/src/cli/cmd/debug/ripgrep.ts) |
| debug rg tree | --limit | number | space, equals | Provider CLI switch. [src/cli/cmd/debug/ripgrep.ts](https://github.com/Kilo-Org/kilocode/blob/67b815466c9ab3e022f16692988673712437b881/packages/opencode/src/cli/cmd/debug/ripgrep.ts) |
| export | --sanitize | none | — | redact sensitive transcript and file data [src/cli/cmd/export.ts](https://github.com/Kilo-Org/kilocode/blob/59f1428abb5fe782ee7bd4d258e72a08b74aadb4/packages/opencode/src/cli/cmd/export.ts) |
| github run | --event | string | space, equals | GitHub mock event to run the agent for [src/cli/cmd/github.ts](https://github.com/Kilo-Org/kilocode/blob/59f1428abb5fe782ee7bd4d258e72a08b74aadb4/packages/opencode/src/cli/cmd/github.ts) |
| github run | --token | string | space, equals | GitHub personal access token (github_pat_********) [src/cli/cmd/github.ts](https://github.com/Kilo-Org/kilocode/blob/59f1428abb5fe782ee7bd4d258e72a08b74aadb4/packages/opencode/src/cli/cmd/github.ts) |
| help | --all | none | — | Show help for all commands. [src/kilocode/help-command.ts](https://github.com/Kilo-Org/kilocode/blob/59f1428abb5fe782ee7bd4d258e72a08b74aadb4/packages/opencode/src/kilocode/help-command.ts) |
| help | --format | string | space, equals | Select help output format md or text. [src/kilocode/help-command.ts](https://github.com/Kilo-Org/kilocode/blob/59f1428abb5fe782ee7bd4d258e72a08b74aadb4/packages/opencode/src/kilocode/help-command.ts) |
| mcp add | --url | string | space, equals | URL for a remote MCP server [src/cli/cmd/mcp.ts](https://github.com/Kilo-Org/kilocode/blob/59f1428abb5fe782ee7bd4d258e72a08b74aadb4/packages/opencode/src/cli/cmd/mcp.ts) |
| mcp add | --env | variadic | space, equals | environment variable for a local MCP server (KEY=VALUE) [src/cli/cmd/mcp.ts](https://github.com/Kilo-Org/kilocode/blob/59f1428abb5fe782ee7bd4d258e72a08b74aadb4/packages/opencode/src/cli/cmd/mcp.ts) |
| mcp add | --header | variadic | space, equals | HTTP header for a remote MCP server (KEY=VALUE) [src/cli/cmd/mcp.ts](https://github.com/Kilo-Org/kilocode/blob/59f1428abb5fe782ee7bd4d258e72a08b74aadb4/packages/opencode/src/cli/cmd/mcp.ts) |
| models | --verbose | none | — | use more verbose model output (includes metadata like costs) [src/cli/cmd/models.ts](https://github.com/Kilo-Org/kilocode/blob/59f1428abb5fe782ee7bd4d258e72a08b74aadb4/packages/opencode/src/cli/cmd/models.ts) |
| models | --refresh | none | — | refresh the models cache from models.dev [src/cli/cmd/models.ts](https://github.com/Kilo-Org/kilocode/blob/59f1428abb5fe782ee7bd4d258e72a08b74aadb4/packages/opencode/src/cli/cmd/models.ts) |
| plug, plugin | --global / -g, --g | none | — | install in global config [src/cli/cmd/plug.ts](https://github.com/Kilo-Org/kilocode/blob/59f1428abb5fe782ee7bd4d258e72a08b74aadb4/packages/opencode/src/cli/cmd/plug.ts) |
| plug, plugin | --force / -f, --f | none | — | replace existing plugin version [src/cli/cmd/plug.ts](https://github.com/Kilo-Org/kilocode/blob/59f1428abb5fe782ee7bd4d258e72a08b74aadb4/packages/opencode/src/cli/cmd/plug.ts) |
| pr link, pr status, pr unlink | --session / -s, --s | string | space, equals, short_attached | session id to apply the PR link to [src/cli/cmd/pr.ts](https://github.com/Kilo-Org/kilocode/blob/59f1428abb5fe782ee7bd4d258e72a08b74aadb4/packages/opencode/src/cli/cmd/pr.ts) |
| profile | --json | none | — | output profile as JSON [src/kilocode/cli/cmd/profile.ts](https://github.com/Kilo-Org/kilocode/blob/59f1428abb5fe782ee7bd4d258e72a08b74aadb4/packages/opencode/src/kilocode/cli/cmd/profile.ts) |
| roll-call | --prompt | string | space, equals | Prompt to send to each model [src/kilocode/cli/cmd/roll-call.ts](https://github.com/Kilo-Org/kilocode/blob/59f1428abb5fe782ee7bd4d258e72a08b74aadb4/packages/opencode/src/kilocode/cli/cmd/roll-call.ts) |
| roll-call | --timeout | number | space, equals | Timeout for each model call in milliseconds [src/kilocode/cli/cmd/roll-call.ts](https://github.com/Kilo-Org/kilocode/blob/59f1428abb5fe782ee7bd4d258e72a08b74aadb4/packages/opencode/src/kilocode/cli/cmd/roll-call.ts) |
| roll-call | --parallel | number | space, equals | Number of parallel model calls [src/kilocode/cli/cmd/roll-call.ts](https://github.com/Kilo-Org/kilocode/blob/59f1428abb5fe782ee7bd4d258e72a08b74aadb4/packages/opencode/src/kilocode/cli/cmd/roll-call.ts) |
| roll-call | --verbose | none | — | Show verbose output [src/kilocode/cli/cmd/roll-call.ts](https://github.com/Kilo-Org/kilocode/blob/59f1428abb5fe782ee7bd4d258e72a08b74aadb4/packages/opencode/src/kilocode/cli/cmd/roll-call.ts) |
| roll-call | --quiet | none | — | Suppress progress and decoration [src/kilocode/cli/cmd/roll-call.ts](https://github.com/Kilo-Org/kilocode/blob/59f1428abb5fe782ee7bd4d258e72a08b74aadb4/packages/opencode/src/kilocode/cli/cmd/roll-call.ts) |
| roll-call | --output | string | space, equals | Output format (table, json, or md) [src/kilocode/cli/cmd/roll-call.ts](https://github.com/Kilo-Org/kilocode/blob/59f1428abb5fe782ee7bd4d258e72a08b74aadb4/packages/opencode/src/kilocode/cli/cmd/roll-call.ts) |
| run | --command | string | space, equals | the command to run, use message for args [src/cli/cmd/run.ts](https://github.com/Kilo-Org/kilocode/blob/59f1428abb5fe782ee7bd4d258e72a08b74aadb4/packages/opencode/src/cli/cmd/run.ts) |
| run | --fork | none | — | fork the session before continuing (requires --continue or --session) [src/cli/cmd/run.ts](https://github.com/Kilo-Org/kilocode/blob/59f1428abb5fe782ee7bd4d258e72a08b74aadb4/packages/opencode/src/cli/cmd/run.ts) |
| run | --share | none | — | share the session [src/cli/cmd/run.ts](https://github.com/Kilo-Org/kilocode/blob/59f1428abb5fe782ee7bd4d258e72a08b74aadb4/packages/opencode/src/cli/cmd/run.ts) |
| run | --format | string | space, equals | format: default (formatted) or json (raw JSON events) [src/cli/cmd/run.ts](https://github.com/Kilo-Org/kilocode/blob/59f1428abb5fe782ee7bd4d258e72a08b74aadb4/packages/opencode/src/cli/cmd/run.ts) |
| run | --file / -f, --f | variadic | space, equals, short_attached | file(s) to attach to message [src/cli/cmd/run.ts](https://github.com/Kilo-Org/kilocode/blob/59f1428abb5fe782ee7bd4d258e72a08b74aadb4/packages/opencode/src/cli/cmd/run.ts) |
| run | --title | string | space, equals | title for the session (uses truncated prompt if no value provided) [src/cli/cmd/run.ts](https://github.com/Kilo-Org/kilocode/blob/59f1428abb5fe782ee7bd4d258e72a08b74aadb4/packages/opencode/src/cli/cmd/run.ts) |
| run | --attach | string | space, equals | attach to a running kilo server (e.g., http://localhost:4096) [src/cli/cmd/run.ts](https://github.com/Kilo-Org/kilocode/blob/59f1428abb5fe782ee7bd4d258e72a08b74aadb4/packages/opencode/src/cli/cmd/run.ts) |
| run | --password / -p, --p | string | space, equals, short_attached | basic auth password (defaults to KILO_SERVER_PASSWORD) [src/cli/cmd/run.ts](https://github.com/Kilo-Org/kilocode/blob/59f1428abb5fe782ee7bd4d258e72a08b74aadb4/packages/opencode/src/cli/cmd/run.ts) |
| run | --username / -u, --u | string | space, equals, short_attached | basic auth username (defaults to KILO_SERVER_USERNAME or 'kilo') [src/cli/cmd/run.ts](https://github.com/Kilo-Org/kilocode/blob/59f1428abb5fe782ee7bd4d258e72a08b74aadb4/packages/opencode/src/cli/cmd/run.ts) |
| run | --dir | string | space, equals | directory to run in, path on remote server if attaching [src/cli/cmd/run.ts](https://github.com/Kilo-Org/kilocode/blob/59f1428abb5fe782ee7bd4d258e72a08b74aadb4/packages/opencode/src/cli/cmd/run.ts) |
| run | --port | number | space, equals | port for the local server (defaults to random port if no value provided) [src/cli/cmd/run.ts](https://github.com/Kilo-Org/kilocode/blob/59f1428abb5fe782ee7bd4d258e72a08b74aadb4/packages/opencode/src/cli/cmd/run.ts) |
| run | --variant | string | space, equals | model variant (provider-specific reasoning effort, e.g., high, max, minimal) [src/cli/cmd/run.ts](https://github.com/Kilo-Org/kilocode/blob/59f1428abb5fe782ee7bd4d258e72a08b74aadb4/packages/opencode/src/cli/cmd/run.ts) |
| run | --thinking | none | — | show thinking blocks [src/cli/cmd/run.ts](https://github.com/Kilo-Org/kilocode/blob/59f1428abb5fe782ee7bd4d258e72a08b74aadb4/packages/opencode/src/cli/cmd/run.ts) |
| run | --interactive / -i, --i | none | — | run in direct interactive split-footer mode [src/cli/cmd/run.ts](https://github.com/Kilo-Org/kilocode/blob/59f1428abb5fe782ee7bd4d258e72a08b74aadb4/packages/opencode/src/cli/cmd/run.ts) |
| run | --mini | none | — | Internal minimal interactive interface selector. [src/cli/cmd/run.ts](https://github.com/Kilo-Org/kilocode/blob/59f1428abb5fe782ee7bd4d258e72a08b74aadb4/packages/opencode/src/cli/cmd/run.ts) |
| run | --replay-limit / --replayLimit | number | space, equals | Limit interactive replay messages. [src/cli/cmd/run.ts](https://github.com/Kilo-Org/kilocode/blob/59f1428abb5fe782ee7bd4d258e72a08b74aadb4/packages/opencode/src/cli/cmd/run.ts) |
| session list | --max-count / -n, --maxCount, --n | number | space, equals, short_attached | limit to N most recent sessions [src/cli/cmd/session.ts](https://github.com/Kilo-Org/kilocode/blob/59f1428abb5fe782ee7bd4d258e72a08b74aadb4/packages/opencode/src/cli/cmd/session.ts) |
| session list | --format | string | space, equals | output format [src/cli/cmd/session.ts](https://github.com/Kilo-Org/kilocode/blob/59f1428abb5fe782ee7bd4d258e72a08b74aadb4/packages/opencode/src/cli/cmd/session.ts) |
| session list | --all / -a, --a | none | — | list sessions from all projects [src/cli/cmd/session.ts](https://github.com/Kilo-Org/kilocode/blob/59f1428abb5fe782ee7bd4d258e72a08b74aadb4/packages/opencode/src/cli/cmd/session.ts) |
| session list | --search / -s, --s | string | space, equals, short_attached | filter sessions by title [src/cli/cmd/session.ts](https://github.com/Kilo-Org/kilocode/blob/59f1428abb5fe782ee7bd4d258e72a08b74aadb4/packages/opencode/src/cli/cmd/session.ts) |
| stats | --days | number | space, equals | show stats for the last N days (default: all time) [src/cli/cmd/stats.ts](https://github.com/Kilo-Org/kilocode/blob/59f1428abb5fe782ee7bd4d258e72a08b74aadb4/packages/opencode/src/cli/cmd/stats.ts) |
| stats | --tools | number | space, equals | number of tools to show (default: all) [src/cli/cmd/stats.ts](https://github.com/Kilo-Org/kilocode/blob/59f1428abb5fe782ee7bd4d258e72a08b74aadb4/packages/opencode/src/cli/cmd/stats.ts) |
| stats | --models | unknown | — | show model statistics (default: hidden). Pass a number to show top N, otherwise shows all [src/cli/cmd/stats.ts](https://github.com/Kilo-Org/kilocode/blob/59f1428abb5fe782ee7bd4d258e72a08b74aadb4/packages/opencode/src/cli/cmd/stats.ts) |
| stats | --project | string | space, equals | filter by project (default: all projects, empty string: current project) [src/cli/cmd/stats.ts](https://github.com/Kilo-Org/kilocode/blob/59f1428abb5fe782ee7bd4d258e72a08b74aadb4/packages/opencode/src/cli/cmd/stats.ts) |
| uninstall | --keep-config / -c, --keepConfig, --c | none | — | keep configuration files [src/cli/cmd/uninstall.ts](https://github.com/Kilo-Org/kilocode/blob/59f1428abb5fe782ee7bd4d258e72a08b74aadb4/packages/opencode/src/cli/cmd/uninstall.ts) |
| uninstall | --keep-data / -d, --keepData, --d | none | — | keep session data and snapshots [src/cli/cmd/uninstall.ts](https://github.com/Kilo-Org/kilocode/blob/59f1428abb5fe782ee7bd4d258e72a08b74aadb4/packages/opencode/src/cli/cmd/uninstall.ts) |
| uninstall | --dry-run / --dryRun | none | — | show what would be removed without removing [src/cli/cmd/uninstall.ts](https://github.com/Kilo-Org/kilocode/blob/59f1428abb5fe782ee7bd4d258e72a08b74aadb4/packages/opencode/src/cli/cmd/uninstall.ts) |
| uninstall | --force / -f, --f | none | — | skip confirmation prompts [src/cli/cmd/uninstall.ts](https://github.com/Kilo-Org/kilocode/blob/59f1428abb5fe782ee7bd4d258e72a08b74aadb4/packages/opencode/src/cli/cmd/uninstall.ts) |
| run | --no-replay | none | — | Use parser-generated negation of --replay. [src/cli/cmd/run.ts](https://github.com/Kilo-Org/kilocode/blob/59f1428abb5fe782ee7bd4d258e72a08b74aadb4/packages/opencode/src/cli/cmd/run.ts) |

### Parser-generated negation switches

These recognized parser spellings negate their underlying registered option in the same exact scope. They consume no following argument and have empty attachment lists. Scalars become false (numbers zero), arrays [false]; choice validation or a handler may reject the resulting value. They are recorded for argument ownership, not recommended as useful command invocations. Boolean affirmative flags can also consume a following literal true/false; their explicit-value semantics do not fit a separate typed value field in this contract.

| Spelling | Aliases | Exact scope |
| --- | --- | --- |
| --no-help | --no-h | global |
| --no-version | --no-v | global |
| --no-print-logs | --no-printLogs | global |
| --no-log-level | --no-logLevel | global |
| --no-pure | — | global |
| --no-port | — | root, completion, daemon, daemon restart, daemon start, run |
| --no-hostname | — | root, completion, daemon, daemon restart, daemon start |
| --no-mdns | — | root, completion, daemon, daemon restart, daemon start |
| --no-mdns-domain | --no-mdnsDomain | root, completion, daemon, daemon restart, daemon start |
| --no-cors | — | root, completion, daemon, daemon restart, daemon start |
| --no-model | --no-m | root, completion, run |
| --no-continue | --no-c | root, completion, run |
| --no-session | --no-s | root, completion, pr link, pr status, pr unlink, run |
| --no-fork | — | root, completion, run |
| --no-cloud-fork | --no-cloudFork | root, completion, run |
| --no-worktree | — | root, completion |
| --no-prompt | — | root, cloud send, cloud start, completion, roll-call |
| --no-agent | — | root, completion, run |
| --no-auto | — | root, completion, run |
| --no-mini | — | root, completion, run |
| --no-replay-limit | --no-replayLimit | root, completion, run |
| --no-yolo | — | root, run |
| --no-dangerously-skip-permissions | --no-dangerouslySkipPermissions | root, run |
| --no-demo | — | root, run |
| --no-session-id | --no-sessionId | cloud result, cloud send, cloud status |
| --no-message-id | --no-messageId | cloud result, cloud status |
| --no-prompt-stdin | --no-promptStdin | cloud send, cloud start |
| --no-repo | — | cloud start |
| --no-repo-type | --no-repoType | cloud start |
| --no-branch | — | cloud start |
| --no-model | — | cloud start |
| --no-mode | — | cloud start |
| --no-org-id | --no-orgId | cloud start |
| --no-stream | — | cloud start |
| --no-json | — | console stop, daemon, daemon restart, daemon start, daemon status, daemon stop, profile |
| --no-foreground | --no-f | daemon, daemon restart, daemon start |
| --no-format | — | db, help, run, session list |
| --no-tool | — | debug agent |
| --no-params | — | debug agent |
| --no-query | — | debug rg files |
| --no-glob | — | debug rg files, debug rg search |
| --no-limit | — | debug rg files, debug rg search, debug rg tree |
| --no-sanitize | — | export |
| --no-event | — | github run |
| --no-token | — | github run |
| --no-all | — | help |
| --no-url | — | mcp add |
| --no-env | — | mcp add |
| --no-header | — | mcp add |
| --no-verbose | — | models, roll-call |
| --no-refresh | — | models |
| --no-global | --no-g | plug, plugin |
| --no-force | --no-f | plug, plugin, uninstall |
| --no-timeout | — | roll-call |
| --no-parallel | — | roll-call |
| --no-quiet | — | roll-call |
| --no-output | — | roll-call |
| --no-command | — | run |
| --no-share | — | run |
| --no-file | --no-f | run |
| --no-title | — | run |
| --no-attach | — | run |
| --no-password | --no-p | run |
| --no-username | --no-u | run |
| --no-dir | — | run |
| --no-variant | — | run |
| --no-thinking | — | run |
| --no-interactive | --no-i | run |
| --no-max-count | --no-n, --no-maxCount | session list |
| --no-all | --no-a | session list |
| --no-search | --no-s | session list |
| --no-days | — | stats |
| --no-tools | — | stats |
| --no-models | — | stats |
| --no-project | — | stats |
| --no-keep-config | --no-c, --no-keepConfig | uninstall |
| --no-keep-data | --no-d, --no-keepData | uninstall |
| --no-dry-run | --no-dryRun | uninstall |

System-prompt delivery: no dedicated `--system-prompt`, `--append-system-prompt`, or `--replace-system-prompt` declaration was found in either release, and standalone native probes rejected those spellings. `--prompt` is a string switch at the TUI, cloud start/send, and roll-call scopes; it is not a verified system-prompt delivery flag. Detailed semantics belong to the system-prompt topic. `--` is an end-of-options delimiter, not a switch record; place it before prompt text to preserve dash-leading content and stop array consumption.

## Configuration Discovery

The [global path module](https://github.com/Kilo-Org/kilocode/blob/59f1428abb5fe782ee7bd4d258e72a08b74aadb4/packages/core/src/global.ts), [config loader](https://github.com/Kilo-Org/kilocode/blob/59f1428abb5fe782ee7bd4d258e72a08b74aadb4/packages/opencode/src/config/config.ts), [directory walker](https://github.com/Kilo-Org/kilocode/blob/59f1428abb5fe782ee7bd4d258e72a08b74aadb4/packages/opencode/src/config/paths.ts), [TUI loader](https://github.com/Kilo-Org/kilocode/blob/59f1428abb5fe782ee7bd4d258e72a08b74aadb4/packages/opencode/src/config/tui.ts), and [managed config loader](https://github.com/Kilo-Org/kilocode/blob/59f1428abb5fe782ee7bd4d258e72a08b74aadb4/packages/opencode/src/config/managed.ts) define discovery. `xdg-basedir` 5.1.0 uses home-relative defaults on **all three operating systems**, including Windows. The earlier APPDATA/LOCALAPPDATA description was incorrect.

| Kind | macOS / Linux default | Windows default |
| --- | --- | --- |
| User config | `~/.config/kilo/` | `%USERPROFILE%\.config\kilo\` |
| Data/auth/database | `~/.local/share/kilo/` | `%USERPROFILE%\.local\share\kilo\` |
| State | `~/.local/state/kilo/` | `%USERPROFILE%\.local\state\kilo\` |
| Cache | `~/.cache/kilo/` | `%USERPROFILE%\.cache\kilo\` |
| System-managed | `/Library/Application Support/kilo/ (macOS); /etc/kilo/ (Linux)` | `%ProgramData%\kilo\` |

Main global configuration merges `config.json`, `kilo.json`, `kilo.jsonc`, `opencode.json`, then `opencode.jsonc`. Files are parsed as JSONC even when named `.json`. `KILO_CONFIG` follows that global layer. Project root `kilo.json[c]` and `opencode.json[c]` files are walked ancestor-first; `.kilo` and legacy `.kilocode` directories load the same four main filenames. Home config directories, linked-worktree primary checkout fallbacks, and `KILO_CONFIG_DIR` also participate. `.opencode` directory discovery is not present in the examined walker. `KILO_CONFIG_CONTENT` is merged after directory layers. Organization configuration, enterprise files, and macOS managed preferences may override it.

TUI configuration is separate: global `tui.json`, `tui.jsonc`, explicit `KILO_TUI_CONFIG`, project `tui.json[c]`, then discovered `.kilo`/`.kilocode` directory TUI files. The macOS managed preference domain remains `ai.opencode.managed`, in a user-specific or system `/Library/Managed Preferences/` plist. Explicit test overrides can redirect managed directories. The frontmatter enumerates individual file/OS records rather than conflating these layers.

`~/.kilo` was checked and was absent on this host. Native `debug paths` in the sandbox resolved the requested XDG paths. Bootstrap can initialize configuration/data directories, `kilo.db` and migrations, logs, telemetry state, and `auth.json`; startup can migrate legacy `~/.kilocode/cli/config.json` authentication. Config loading can write `$schema`, `.gitignore`, migrate legacy rules/modes/workflows, or install plugin dependencies. Isolate probes and do not treat a diagnostic command as filesystem-pure. Concurrent first-start probes sharing an empty home produced one database migration error; the sequential rerun succeeded.

## Environment Variables

The general runtime variables below come from the [core flags](https://github.com/Kilo-Org/kilocode/blob/59f1428abb5fe782ee7bd4d258e72a08b74aadb4/packages/core/src/flag/flag.ts), global/config/managed loaders, launcher, and [session pager](https://github.com/Kilo-Org/kilocode/blob/59f1428abb5fe782ee7bd4d258e72a08b74aadb4/packages/opencode/src/cli/cmd/session.ts). Unless otherwise noted, core booleans accept case-insensitive `true` or `1`; positive-number helpers ignore invalid/nonpositive values. Experimental variables are implementation controls, not stable wrapper guarantees.

| Variable | Effect |
| --- | --- |
| HOME | Unix home-directory input used by Node/Bun and XDG defaults. |
| USERPROFILE | Windows home-directory input; XDG defaults are under this home. |
| XDG_CONFIG_HOME | Choose the base directory containing kilo user configuration. |
| XDG_DATA_HOME | Choose the base directory containing kilo auth, database, sessions, and logs. |
| XDG_CACHE_HOME | Choose the base directory containing kilo caches and helper binaries. |
| XDG_STATE_HOME | Choose the base directory containing kilo state and daemon metadata. |
| ProgramData | Windows enterprise-managed configuration base; defaults to C:\ProgramData. |
| KILO_CONFIG | Load an explicitly named configuration file after global configuration. |
| KILO_CONFIG_DIR | Add an explicit directory for config, agent, command, skill, and plugin discovery; does not erase the normal global layer. |
| KILO_CONFIG_CONTENT | Merge an inline JSONC config string after project/config-directory layers, before organization and managed policy layers. |
| KILO_TUI_CONFIG | Load an explicitly named TUI config file. |
| KILO_DISABLE_PROJECT_CONFIG | Skip project config files and project .kilo/.kilocode discovery when true or 1. |
| KILO_PURE | Disable external plugins when true or 1; built-in plugins still appear in resolved config. |
| KILO_BIN_PATH | npm launcher selects the supplied native executable instead of normal binary discovery. |
| KILO_TREE_SITTER_WASM_DIR | Choose tree-sitter WASM resource directory; npm launcher fills it from bundled resources if unset. |
| KILO_SERVER_PASSWORD | HTTP server/client Basic authentication password default. |
| KILO_SERVER_USERNAME | HTTP Basic authentication username default; client falls back to kilo. |
| KILO_DISABLE_AUTOUPDATE | Disable automatic updates when true or 1. |
| KILO_ALWAYS_NOTIFY_UPDATE | Always show update notification when true or 1. |
| KILO_AUTO_SHARE | Automatically share newly created sessions when true or 1. |
| KILO_ENABLE_EXA | Enable Exa search; also enabled by KILO_EXPERIMENTAL_EXA or the grouped experimental switch. |
| KILO_EXPERIMENTAL_EXA | Compatibility experimental Exa search switch. |
| KILO_EXPERIMENTAL_ICON_DISCOVERY | Enable experimental icon discovery. |
| KILO_EXPERIMENTAL_OXFMT | Enable experimental oxfmt integration. |
| KILO_EXPERIMENTAL_LSP_TY | Enable experimental ty language-server integration. |
| KILO_EXPERIMENTAL_LSP_TOOL | Enable experimental LSP tool. |
| KILO_EXPERIMENTAL_SCOUT | Enable experimental scout behavior. |
| AGENT | The CLI sets this to 1 before command bootstrap so children can recognize an agent context. |
| OPENCODE | The CLI sets this compatibility marker to 1 before bootstrap. |
| KILO | The CLI sets this marker to 1 during normal Kilo bootstrap. |
| KILO_DISABLE_PRUNE | Disable automatic pruning of older tool outputs. |
| KILO_DISABLE_AUTOCOMPACT | Disable automatic context compaction. |
| KILO_DISABLE_TERMINAL_TITLE | Prevent terminal title changes. |
| KILO_DISABLE_MOUSE | Disable TUI mouse handling. |
| KILO_DISABLE_LSP_DOWNLOAD | Disable automatic LSP downloads. |
| KILO_DISABLE_DEFAULT_PLUGINS | Disable default external plugin loading. |
| KILO_DISABLE_EXTERNAL_SKILLS | Disable external skill discovery. |
| KILO_DISABLE_CLAUDE_CODE | Disable Claude Code compatibility prompt/skill discovery. |
| KILO_DISABLE_CLAUDE_CODE_SKILLS | Disable Claude Code skill discovery independently. |
| KILO_DISABLE_CLAUDE_CODE_PROMPT | Disable Claude Code prompt discovery independently; semantics belong to system-prompt. |
| KILO_GIT_BASH_PATH | Specify Windows Git Bash executable path; also used to locate less for session table paging. |
| KILO_DB | Override the SQLite database path. |
| KILO_DISABLE_CHANNEL_DB | Disable installation-channel-specific database naming. |
| KILO_SKIP_MIGRATIONS | Skip database migrations; unsafe with incompatible databases. |
| KILO_STRICT_CONFIG_DEPS | Enable strict configuration dependency handling. |
| KILO_PLUGIN_META_FILE | Override plugin metadata file location. |
| KILO_CLIENT | Set client identity; defaults to cli. |
| KILO_SESSION_RETRY_LIMIT | Override retry limit with a positive integer; invalid/nonpositive values are ignored. |
| KILO_WORKSPACE_ID | Identify the workspace for runtime services. |
| KILO_DISABLE_EMBEDDED_WEB_UI | Disable the embedded web UI. |
| KILO_DISABLE_FFF | Disable fff file finder; defaults disabled on Windows in 7.8.3. |
| KILO_ENABLE_QUESTION_TOOL | Enable the question tool. |
| KILO_ENABLE_PARALLEL | Enable parallel execution; KILO_EXPERIMENTAL_PARALLEL also enables it. |
| KILO_EXPERIMENTAL_PARALLEL | Compatibility experimental switch for parallel execution. |
| KILO_EXPERIMENTAL | Enable grouped experimental features. |
| KILO_EXPERIMENTAL_FILEWATCHER | Enable experimental file watching (Effect boolean config). |
| KILO_EXPERIMENTAL_DISABLE_FILEWATCHER | Disable experimental file watching (Effect boolean config). |
| KILO_EXPERIMENTAL_DISABLE_COPY_ON_SELECT | Disable copy-on-select; default true on Windows. |
| KILO_EXPERIMENTAL_BASH_DEFAULT_TIMEOUT_MS | Override bash default timeout with a positive integer in milliseconds. |
| KILO_EXPERIMENTAL_OUTPUT_TOKEN_MAX | Override output-token ceiling with a positive integer. |
| KILO_EXPERIMENTAL_MARKDOWN | Enable experimental Markdown rendering unless false or 0. |
| KILO_EXPERIMENTAL_CUSTOMIZE_SKILL | Enable skill customization; defaults enabled for dev/beta/local channels unless false or 0. |
| KILO_EXPERIMENTAL_CLAUDE_MIGRATION | Enable experimental Claude migration. |
| KILO_EXPERIMENTAL_WORKSPACES | Enable experimental workspaces; inherits KILO_EXPERIMENTAL unless explicitly set. |
| KILO_EXPERIMENTAL_EVENT_SYSTEM | Enable experimental event system. |
| KILO_EXPERIMENTAL_SESSION_SWITCHING | Enable experimental session switching. |
| KILO_EXPERIMENTAL_SESSION_SWITCHER | Enable experimental session switcher; inherits KILO_EXPERIMENTAL unless explicitly set. |
| KILO_EXPERIMENTAL_REFERENCES | Enable experimental references; inherits KILO_EXPERIMENTAL unless explicitly set. |
| KILO_TEST_HOME | Override Global.Path.home for compatibility discovery; does not move XDG bases already initialized from the real home. |
| KILO_TEST_MANAGED_CONFIG_DIR | Override system-managed config directory; named as a test override in source. |
| PWD | Used by run to resolve its working directory; wrappers should keep it consistent with the actual child cwd. |

Core flags also include experimental icon discovery, Exa search, oxfmt, LSP ty/tool, and scout controls; those feature-specific controls do not establish argument ownership. Model-endpoint/catalog variables belong to model-config, permission variables to agent-permissions, MCP variables to mcp, and log/telemetry/process-correlation variables to agent-logging. No generic `KILO_PROVIDER`, `KILO_<FIELD_NAME>`, or `KILOCODE_<FIELD_NAME>` override mechanism was found in the examined CLI config source; the previous claim is removed.

## Machine Introspection

| Command | Format | Use and limitations |
| --- | --- | --- |
| kilo debug config | json | Observed valid JSON; contains merged configuration and may expose sensitive configuration. |
| kilo debug v2 | json | Observed providers/default/small JSON. In 7.8.3 default serialized an Effect object rather than a resolved model ID; do not trust that field. |
| kilo debug skill | json | Observed JSON array including full skill content; potentially sensitive and large. |
| kilo generate | json | Hidden source-registered command emits HTTP OpenAPI, not JSON Schema for kilo.json and not CLI switches. Handler not run. |
| kilo models --verbose | text (human output) | Source prints model IDs plus JSON metadata blocks; not one JSON document. |
| kilo session list --format json | json | Observed empty stdout with exit 0 when no sessions; otherwise source emits JSON array. Avoid the table pager. |
| kilo daemon status --json | json | Observed JSON with running, stale, file, reason; safe state projection omits password. |
| kilo db 'SELECT 1 AS probe' --format json | json | Observed [{"probe":1}]; query mode accesses local SQLite. Keep wrapper queries fixed/read-only. |
| kilo db path | text (human output) | Observed resolved database path. |
| kilo debug paths | text (human output) | Observed text key/path lines after initialization; first simultaneous boot raced with other probes and failed a migration, sequential rerun succeeded. |
| kilo config check | text (human output) | Observed styled diagnostics and exit 1; not a JSON diagnostics protocol. |
| kilo auth list | text (human output) | Source renders credentials/provider names and environment-variable names; no JSON switch. |
| kilo mcp list | text (human output) | Source renders status text; configured servers may connect/spawn. |
| kilo profile --json | json | Observed unauthenticated exit 1, empty stdout, styled stderr; JSON only on success. |
| kilo cloud status --session-id ID --message-id ID | json | Source prints authenticated remote-task status JSON; not executed. |
| kilo cloud result --session-id ID --message-id ID | json | Source prints authenticated remote-task result JSON; not executed. |

No machine-readable CLI option manifest or CLI JSON-help mode was found. `completion` generates a shell script; it is not a typed option schema. `generate` exposes HTTP OpenAPI only. `run --format json` is an execution event stream, not provider-state introspection. `models --verbose` mixes IDs and JSON object blocks rather than emitting a JSON array. Cloud output is documented from source, not authenticated execution.

## Wrapper Notes

- Installed kilo --version is exactly 7.3.45; separately downloaded 7.8.3 native binary also reports 7.8.3. Gate compatibility by version; latest npm/GitHub release is 7.8.3.
- Use run with closed stdin and --format json for one-shot execution; --interactive/-i and root --mini require a TTY. No resume command exists: use run --continue/-c or --session/-s, with --fork or --cloud-fork as appropriate.
- run --file/-f is greedy variadic, including after equals. Put -- before positional prompt text. Minimum zero is a confirmed fact that schema revision 2 cannot encode; variadic_min is conservatively unknown with an explicit gap.
- yargs short-option groups do not support arbitrary -mfoo values. Numeric or punctuation-leading attached values work; space or equals avoids grouped-letter ambiguity.
- Scalar flags usually allow parser-level omission; empty strings/undefined can still fail choices or handlers. Boolean flags can consume a following literal true/false or accept =true/=false; the none records describe standalone toggles.
- Boolean negations and camel-case aliases are accepted by the parser. Some handlers inspect literal argv spelling (for example network explicitness and print-logs), so parser acceptance does not guarantee equivalent downstream behavior.
- Bare root positional is a project path; --prompt supplies the initial user prompt. No system-prompt, append-system-prompt, or replace-system-prompt options occur in the examined declarations; dedicated semantics belong to system-prompt.
- Help is normally on stderr with branding and can include INFO lines on successful exit. NO_COLOR did not remove all styled diagnostic output. help --all --format md returned root help in both releases.
- Configuration/data initialization writes directories, database migrations, logs and telemetry state. Concurrent first boots into one empty home caused a migration failure; sequential initialization succeeded. Use a disposable home/XDG sandbox for probing.
- Windows uses XDG defaults under USERPROFILE/.config and .local/share, rather than APPDATA/LOCALAPPDATA. Enterprise configuration is separate under ProgramData/kilo.
- session list JSON mode can return empty stdout. debug v2 default in 7.8.3 is an unevaluated Effect object; do not assume it is a model ID.
- Long-running acp/serve/debug wait do not meet this contract's run-to-completion non_interactive criterion. Daemon launch returns unless foreground is selected; export/db require explicit arguments to avoid a picker/shell.
- cloud start/send, roll-call, github run and actual model sessions can incur charges. Plugin/import/session deletion/worktree commands mutate local state. None of these handlers was executed for research.
- npm launcher adds a Node process, forwards SIGINT/SIGTERM/SIGHUP and propagates exit status/signals; KILO_BIN_PATH can redirect it to another binary.
- ~/.kilo was checked and is absent on this host; home .kilo is only one config-discovery location, not the default user config store.

## Sources

- [Kilo homepage](https://kilo.ai/)
- [CLI guide](https://kilo.ai/docs/code-with-ai/platforms/cli)
- [CLI command reference](https://kilo.ai/docs/code-with-ai/platforms/cli-reference)
- [npm latest package metadata](https://registry.npmjs.org/@kilocode/cli/latest)
- [GitHub 7.8.3 release](https://github.com/Kilo-Org/kilocode/releases/tag/v7.8.3)
- [7.3.45 source](https://github.com/Kilo-Org/kilocode/tree/67b815466c9ab3e022f16692988673712437b881) and [7.8.3 source](https://github.com/Kilo-Org/kilocode/tree/59f1428abb5fe782ee7bd4d258e72a08b74aadb4)
- [yargs parser defaults and consumption](https://github.com/yargs/yargs-parser/blob/v22.0.0/lib/yargs-parser.ts)
- [yargs option factory](https://github.com/yargs/yargs/blob/v18.0.0/lib/yargs-factory.ts)
- [xdg-basedir defaults](https://github.com/sindresorhus/xdg-basedir/blob/v5.1.0/index.js)
- Pinned per-command declaration links appear in the switch table and evidence records.
- Local observations: `kilo --version`; downloaded platform binary `--version`; recursive `--help`; isolated paths/config/catalog/skills/session/db/daemon/profile/config-check commands; missing-file and rejected-switch probes. Sanitized temporary help/probe artifacts are under `/tmp/kilo-cli-research-20261001/`; these paths are session artifacts, not published source URLs.

## Changelog

- 2026-10-01: Migrated the previous untyped revision-1 document to revision 2 with evidence-backed switch types, attachments, exact scopes, and explicit gaps.
- 2026-10-01: Verified installed 7.3.45 and current release 7.8.3; inventoried nested native commands and aliases and documented removed/added paths.
- 2026-10-01: Corrected file attachment from repeatable scalar to greedy variadic, Windows XDG paths, and long-running server classification.
- 2026-10-01: Recorded hidden compatibility switches, parser omission/short-attachment behavior, empty JSON session output, help-reference discrepancy, and configuration initialization caveats.
- 2026-10-01: Removed unsupported generic KILO_PROVIDER/KILO_<FIELD_NAME>/KILOCODE_<FIELD_NAME> claims; provider-endpoint variables belong to model-config.