---
$schema: ./_schema.yaml
schema_revision: 2
provider: claude
created: 2026-07-02
last_updated: 2026-10-01
agent: opencode
model: zai-coding-plan/glm-5.3
reasoning_effort: provider_default
latest_version: 2.1.287
versions_examined:
- 2.1.287
evidence:
- claim: Root usage line, the visible subcommand list, and every root switch spelling, alias, value placeholder, and default that help renders; commander.js renders a required value as `<x>`, an optional value as `[x]`, and a variadic list as `<x...>`.
  id: help-root-287
  limitations: Help is incomplete by the provider's own statement, so hidden and docs-only switches do not appear, and help alone does not prove that a value binds.
  location: local `claude --help` output; binary /Users/ken/.local/share/claude/versions/2.1.287 via symlink /Users/ken/.local/bin/claude
  method: local_inspection
  observed_on: 2026-10-01
  version: 2.1.287
- claim: Subcommand trees below each command group and their option spellings, placeholders, defaults, and aliases.
  id: help-subcommands-287
  limitations: Hidden commands must be probed by name because help does not list them; `rm` and `daemon` render some options in usage prose instead of an options block.
  location: local `claude <path> --help` output for every command family listed under Subcommands, including the hidden daemon, remote-control, and self-hosted-runner
  method: local_inspection
  observed_on: 2026-10-01
  version: 2.1.287
- claim: The documented command and flag tables, including flags absent from local help, their aliases, value notes, version gates, and removed-flag history.
  id: docs-cli-reference
  limitations: Describes an unspecified current version; `--exec`, documented there, is rejected by the 2.1.287 parser.
  location: https://code.claude.com/docs/en/cli-reference
  method: official_docs
  observed_on: 2026-10-01
  version: unknown
- claim: The general runtime environment variables, their on/off spellings, and env-versus-settings precedence.
  id: docs-env-vars
  limitations: Model-endpoint, permission-policy, MCP-specific, and logging variables belong to their narrower topics and are not recorded here.
  location: https://code.claude.com/docs/en/env-vars
  method: official_docs
  observed_on: 2026-10-01
  version: unknown
- claim: Install commands per operating system and channel, auto-update behavior per install method, and the Node 22+ npm engine requirement.
  id: docs-setup
  limitations: Install commands were not executed on this host beyond the already-installed native build.
  location: https://code.claude.com/docs/en/setup
  method: official_docs
  observed_on: 2026-10-01
  version: unknown
- claim: Managed-settings file locations per operating system, the managed-settings.d drop-in directory, and the MDM and registry delivery paths.
  id: docs-managed-settings
  limitations: Enterprise paths were not locally exercised because this host has no managed deployment.
  location: https://code.claude.com/docs/en/managed-settings
  method: official_docs
  observed_on: 2026-10-01
  version: unknown
- claim: The installed command is a native darwin-arm64 binary managed by the native installer, version 2.1.287, auto-update channel latest.
  id: local-binary-287
  limitations: Single host, macOS arm64 only.
  location: command -v claude; ls -l of the symlink; claude --version; claude doctor install report
  method: local_inspection
  observed_on: 2026-10-01
  version: 2.1.287
- claim: The npm registry reports version 2.1.287 with dist-tags latest 2.1.287, stable 2.1.285, and next 2.1.287, so 2.1.287 is the newest release.
  id: npm-dist-tags-287
  limitations: Registry snapshot at observation time.
  location: npm view @anthropic-ai/claude-code version dist-tags --json
  method: local_inspection
  observed_on: 2026-10-01
  version: 2.1.287
- claim: The parser is commander.js and validates choice-valued options at parse time; a value binds through both the space form and the equals form for required-value and optional-value options alike; single-dash spellings accept a short-attached value (-nfoo and -rbar parse instead of erroring as unknown); an optional-value switch does not consume a following token that looks like an option; commander applies these forms uniformly to options declared with the same shape.
  id: test-parser-forms-287
  limitations: Forms were proven directly only for the switches named here; other switches rest on the parser's uniform behavior for the same declaration shape.
  location: temp directory; claude -p --output-format bogus 'q'; --output-format=bogus; --permission-mode bogus and =bogus; --input-format=bogus; --system-prompt-snapshot=bogus; --permission-prompts bogus; --autocompact bogus; --prompt-suggestions bogus and =bogus; -nfoo and -rbar before --setting-sources bogus; --from-pr --setting-sources bogus
  method: disposable_test
  observed_on: 2026-10-01
  version: 2.1.287
- claim: A variadic option grabs the immediately following token unconditionally, even when it looks like an option, and keeps consuming non-option tokens after it; equals binds a variadic value; at least one value is taken, so the variadic minimum is 1; with every token consumed and no prompt remaining, the run exits with an input-required error instead of starting a session.
  id: test-parser-variadic-287
  limitations: Established with --add-dir at the root entrypoint; other variadic switches share the parser.
  location: temp directory; claude --add-dir /nonexistent-dir-xyz --setting-sources bogus; --add-dir=/nonexistent-dir-xyz alone; --add-dir --setting-sources bogus
  method: disposable_test
  observed_on: 2026-10-01
  version: 2.1.287
- claim: 'The parser rejects unknown long and short options with `error: unknown option` and a non-zero exit, and `--exec`, documented in the CLI reference, is rejected as unknown in 2.1.287.'
  id: test-unknown-strict-287
  limitations: Probed at the root entrypoint and for one documented flag.
  location: temp directory; claude --bogus-flag; claude -Z; claude --exec --setting-sources bogus
  method: disposable_test
  observed_on: 2026-10-01
  version: 2.1.287
- claim: Each probed flag is accepted by the 2.1.287 root parser; required-value flags error with `option '--flag <placeholder>' argument missing`, which also gives the exact placeholder; `<servers...>` in those errors proves --channels and --dangerously-load-development-channels take variadic values; the boolean docs-only flags and the removed --enable-auto-mode parse without an unknown-option error.
  id: test-missing-arg-287
  limitations: Proves acceptance and arity, not semantics.
  location: temp directory; bare claude --max-turns, --permission-prompt-tool, --advisor, --append-system-prompt-file, --append-subagent-system-prompt, --append-subagent-system-prompt-file, --teammate-mode, --ref, --fallback-model, --json-schema, --file, --agents, --settings, --environment, --model, --system-prompt, --autocompact, --effort, --sdk-url, --channels, --dangerously-load-development-channels, --init, --init-only, --maintenance, --enable-auto-mode, --remote, --rc
  method: disposable_test
  observed_on: 2026-10-01
  version: 2.1.287
- claim: '`claude --remote` fails with an error that names `--cloud`, proving --remote is an accepted alias of --cloud; the same error says piped stdout alone selects non-interactive mode and names the hidden --sdk-url flag as another non-interactive entry.'
  id: test-remote-alias-287
  limitations: Alias direction only; --cloud itself needs an interactive terminal to run.
  location: temp directory; claude --remote with piped stdout
  method: disposable_test
  observed_on: 2026-10-01
  version: 2.1.287
- claim: A non-UUID --session-id value is consumed from the space form and rejected app-side with `Invalid session ID. Must be a valid UUID.` before any session starts.
  id: test-session-id-287
  limitations: Establishes the space form and the UUID check, not the equals form.
  location: temp directory; claude --session-id not-a-uuid
  method: disposable_test
  observed_on: 2026-10-01
  version: 2.1.287
- claim: The space form binds the --effort value; an invalid value prints a warning naming the value and the valid set (low, medium, high, xhigh, max) and falls back to the default effort instead of exiting.
  id: test-effort-287
  limitations: Warning behavior only; no model call was made.
  location: temp directory; claude --effort bogus
  method: disposable_test
  observed_on: 2026-10-01
  version: 2.1.287
- claim: '`mcp add` validates the transport app-side and rejects an invalid one naming stdio, sse, http, and the help-undocumented streamable-http; space, equals, and short-attached forms all bind the value; the failed probe wrote no server configuration.'
  id: test-mcp-add-287
  limitations: Transport only; scope and env values were not probed for write side effects.
  location: temp directory; claude mcp add -t bogus probe-name probe-cmd; claude mcp add -tbogus probe-name probe-cmd; claude mcp add --transport=bogus probe-name probe-cmd
  method: disposable_test
  observed_on: 2026-10-01
  version: 2.1.287
- claim: Prints JSON with loggedIn, authMethod, apiProvider, projectsDirectory, configDirectory, forcedLoginMethod, email, orgId, orgName, and subscriptionType, and exits 0 when logged in.
  id: probe-auth-status-287
  limitations: The logged-out exit-1 behavior is documented, not observed on this host.
  location: claude auth status with piped stdout
  method: local_inspection
  observed_on: 2026-10-01
  version: 2.1.287
- claim: Prints a JSON array of interactive and background sessions with id, cwd, kind, startedAt, sessionId, name, and status or state fields, and exits 0 without a TTY.
  id: probe-agents-json-287
  limitations: Snapshot of live sessions on one host.
  location: claude agents --json with piped stdout
  method: local_inspection
  observed_on: 2026-10-01
  version: 2.1.287
- claim: 'Both print JSON: defaults emits the built-in environment, allow, soft_deny, and hard_deny rules; config emits the effective rules after settings.'
  id: probe-auto-mode-287
  limitations: Rule content is policy data, not CLI surface.
  location: claude auto-mode defaults and claude auto-mode config with piped stdout
  method: local_inspection
  observed_on: 2026-10-01
  version: 2.1.287
- claim: Prints a JSON array of installed plugins with id, version, scope, enabled, installPath, installedAt, lastUpdated, and projectEnabled.
  id: probe-plugin-list-287
  limitations: Reflects this host's plugin set.
  location: claude plugin list --json with piped stdout
  method: local_inspection
  observed_on: 2026-10-01
  version: 2.1.287
- claim: Prints human-readable supervisor state (not running, socket directory, roster age, log path) and exits 1 when no supervisor is running.
  id: probe-daemon-status-287
  limitations: Text output only; the running-state format was not observed.
  location: claude daemon status with piped stdout
  method: local_inspection
  observed_on: 2026-10-01
  version: 2.1.287
- claim: Runs to completion without a TTY in well under a minute, prints install method, version, commit, platform, search, auto-update, and policy lines, and exits 0.
  id: probe-doctor-287
  limitations: Single healthy installation; the 2.1.200-era hang was not reproduced.
  location: claude doctor with piped stdout and stdin closed
  method: local_inspection
  observed_on: 2026-10-01
  version: 2.1.287
- claim: Prints human-readable server names with live health-check results and performs real network connections during the listing.
  id: probe-mcp-list-287
  limitations: Output is text; MCP semantics belong to the MCP topic.
  location: claude mcp list with piped stdout
  method: local_inspection
  observed_on: 2026-10-01
  version: 2.1.287
- claim: '`claude import` replies `not yet available in this build` in 2.1.287 and exits without importing, so the documented command is disabled in this build.'
  id: probe-import-287
  limitations: One source (codex) probed; availability may differ by channel.
  location: claude import codex --dry-run and claude import codex --yes=bogusdigest --dry-run with piped stdout
  method: local_inspection
  observed_on: 2026-10-01
  version: 2.1.287
- claim: The user settings file holds hooks, permissions, model, statusLine, enabledPlugins, extraKnownMarketplaces, effortLevel, and tui keys; ~/.claude.json holds mutable state (installMethod, projects, oauthAccount, caches, onboarding flags); the ~/.claude tree also contains projects, sessions, plugins, skills, agents, debug, and daemon state.
  id: local-config-keys-287
  limitations: Values were not read; only key names and file presence.
  location: key-only listing of ~/.claude/settings.json, ~/.claude.json, and ls of ~/.claude
  method: local_inspection
  observed_on: 2026-10-01
  version: 2.1.287
homepage: https://claude.ai/code
docs: https://code.claude.com/docs/en/overview
cli_docs: https://code.claude.com/docs/en/cli-reference
binaries:
- alt_binaries: []
  binary: claude
  notes: Native, Homebrew, and npm installs expose `claude`. Local inspection found /Users/ken/.local/bin/claude, a native-installer symlink into /Users/ken/.local/share/claude/versions/.
  os: macos
- alt_binaries: []
  binary: claude
  notes: Native, apt, dnf, apk, and npm installs expose `claude`; the same name is used inside WSL.
  os: linux
- alt_binaries:
  - claude
  - claude.cmd
  binary: claude.exe
  notes: Native PowerShell/CMD installer and WinGet provide claude.exe invoked as `claude`; npm installs link a per-platform optional dependency and create command shims such as claude.cmd.
  os: windows
install_methods:
- command: curl -fsSL https://claude.ai/install.sh | bash
  method: other
  notes: Recommended native installer; installs under ~/.local/bin and ~/.local/share/claude/versions and auto-updates in the background; accepts stable, latest, or a specific version as an argument.
  os: macos
- command: curl -fsSL https://claude.ai/install.sh | bash
  method: other
  notes: Recommended native installer for Linux and WSL; auto-updates; same channel and version arguments as macOS.
  os: linux
- command: irm https://claude.ai/install.ps1 | iex
  method: other
  notes: Native PowerShell installer; the CMD form is `curl -fsSL https://claude.ai/install.cmd -o install.cmd && install.cmd && del install.cmd`; auto-updates.
  os: windows
- command: brew install --cask claude-code
  method: brew
  notes: Stable-channel cask, typically about a week behind; `brew install --cask claude-code@latest` tracks the latest channel. Homebrew installs do not auto-update through Claude Code.
  os: macos
- command: winget install Anthropic.ClaudeCode
  method: winget
  notes: WinGet installs do not auto-update; run `winget upgrade Anthropic.ClaudeCode` manually or opt in with CLAUDE_CODE_PACKAGE_MANAGER_AUTO_UPDATE=1.
  os: windows
- command: sudo apt install claude-code
  method: package_manager
  notes: Debian/Ubuntu after adding Anthropic's signed apt repository (stable or latest channel); does not auto-update.
  os: linux
- command: sudo dnf install claude-code
  method: package_manager
  notes: Fedora/RHEL after adding the signed rpm repository; does not auto-update.
  os: linux
- command: apk add claude-code
  method: package_manager
  notes: Alpine after adding the signed apk repository; requires bash, curl, libgcc, libstdc++, and ripgrep at runtime.
  os: linux
- command: npm install -g @anthropic-ai/claude-code
  method: npm
  notes: Requires Node.js 22+ for install-time engine checks as of v2.1.198; installs the same native binary via per-platform optional dependencies; the installed claude does not invoke Node at runtime.
  os: macos
- command: npm install -g @anthropic-ai/claude-code
  method: npm
  notes: Supported platforms include linux-x64, linux-arm64, and the musl variants; optional dependencies must be allowed.
  os: linux
- command: npm install -g @anthropic-ai/claude-code
  method: npm
  notes: Supported platforms include win32-x64 and win32-arm64; npm creates command shims; do not elevate.
  os: windows
subcommands:
- description: Opens agent view to monitor and dispatch background sessions.
  name: agents
  non_interactive: false
  notes: Opening the view requires an interactive terminal; `--json` prints sessions as a JSON array and exits without a TTY. Accepts root-level session flags to set defaults for dispatched sessions.
- description: Opens a background session in the current terminal.
  name: attach
  non_interactive: false
  notes: Takes the short id that `--bg` prints and `claude agents` lists; requires an interactive terminal; Ctrl+Z or the left arrow returns to agent view.
- description: Signs in to an Anthropic account.
  name: auth login
  non_interactive: false
  notes: Browser or SSO flow; --email pre-fills the address, --console selects Console API billing, --sso forces SSO.
- description: Logs out from the Anthropic account.
  name: auth logout
  non_interactive: true
  notes: Mutates local auth state but prompts nobody.
- description: Prints authentication status as JSON by default.
  name: auth status
  non_interactive: true
  notes: Exits 0 when logged in and 1 when not; `--text` switches to human-readable output; the JSON includes configDirectory since v2.1.268.
- description: Prints the effective auto mode classifier configuration as JSON.
  name: auto-mode config
  non_interactive: true
  notes: User settings where set, defaults otherwise.
- description: Gets AI feedback on custom auto mode rules.
  name: auto-mode critique
  non_interactive: true
  notes: Makes a real model call that costs money and needs auth; `--model` overrides the model used.
- description: Prints the built-in auto mode rules as JSON.
  name: auto-mode defaults
  non_interactive: true
  notes: Environment, allow, soft_deny, and hard_deny rule catalogs; `--label <prefix>` filters by label prefix, case-insensitively. Requires v2.1.208+.
- description: Resets auto mode configuration to shipped defaults.
  name: auto-mode reset
  non_interactive: false
  notes: Prompts for confirmation before removing the autoMode section from user settings; `-y`/`--yes` skips the prompt. Requires v2.1.212+.
- description: Manages the background-session supervisor.
  name: daemon
  non_interactive: true
  notes: Hidden from top-level help; with piped input the bare command runs the supervisor in the foreground, and service install is disabled in this version. Carries the umbrella --json-path and --log-file options.
- description: Tails the background-session supervisor log.
  name: daemon logs
  non_interactive: true
  notes: Hidden from top-level help; runs until interrupted.
- description: Runs the background-session supervisor in the foreground.
  name: daemon run
  non_interactive: true
  notes: Hidden; takes an optional json-path positional; service install is disabled in this version, so the daemon runs on demand.
- description: Prints supervisor pid, version, uptime, and socket state.
  name: daemon status
  non_interactive: true
  notes: Hidden; exits 1 when the supervisor is not running, which is a normal state.
- description: Shuts down the supervisor and terminates background sessions.
  name: daemon stop
  non_interactive: true
  notes: Hidden; `--any` also stops a transient daemon and `--keep-workers` leaves detached sessions running.
- description: Removes the background service integration.
  name: daemon uninstall
  non_interactive: true
  notes: Hidden; destructive to service registration.
- description: Prints read-only installation and settings diagnostics without starting a session.
  name: doctor
  non_interactive: true
  notes: Ran to completion with exit 0 under piped stdout in 2.1.287, unlike the 2.1.200-era hang; reports install method, search, auto-update state, and settings validation errors. The fuller checkup is /doctor inside a session.
- description: Runs the enterprise auth and telemetry gateway server.
  name: gateway
  non_interactive: true
  notes: Long-running; requires `--config` pointing at a gateway.yaml. Available since v2.1.195.
- description: Imports configuration from another coding agent.
  name: import
  non_interactive: false
  notes: Documented (v2.1.213+) to start an interactive session running /import with codex, gemini, or cursor sources; in this 2.1.287 build the command prints `not yet available in this build` instead.
- description: Installs or reinstalls the native binary.
  name: install
  non_interactive: true
  notes: Positional target accepts stable, latest, or a specific version; `--force` reinstalls even when installed.
- description: Prints a background session's recent terminal output.
  name: logs
  non_interactive: true
  notes: Takes the short session id printed by `--bg`.
- description: Adds an MCP server to Claude Code configuration.
  name: mcp add
  non_interactive: true
  notes: stdio servers take a command and args after `--`; HTTP/SSE servers take a URL. Transport is validated app-side and an invalid value is refused before any write.
- description: Imports MCP servers from Claude Desktop.
  name: mcp add-from-claude-desktop
  non_interactive: true
  notes: Mac and WSL only.
- description: Adds an MCP server from a JSON string.
  name: mcp add-json
  non_interactive: true
  notes: Supports stdio, SSE, HTTP, and WebSocket server definitions.
- description: Prints details about one configured MCP server.
  name: mcp get
  non_interactive: true
  notes: Unapproved .mcp.json servers show as pending and are not connected to.
- description: Lists configured MCP servers with health checks.
  name: mcp list
  non_interactive: true
  notes: Performs live network health checks, so timing depends on connectivity; text output only.
- description: Runs a configured MCP server's OAuth flow.
  name: mcp login
  non_interactive: false
  notes: Opens a browser by default; `--no-browser` prints the authorization URL for SSH or headless use and asks for the redirect URL back at a prompt.
- description: Clears stored OAuth credentials for an MCP server.
  name: mcp logout
  non_interactive: true
  notes: Mutates stored credentials.
- description: Removes an MCP server.
  name: mcp remove
  non_interactive: true
  notes: Without `--scope`, removes from whichever scope the server exists in.
- description: Resets approvals for project-scoped .mcp.json servers.
  name: mcp reset-project-choices
  non_interactive: true
  notes: Clears both approvals and rejections in this project.
- description: Starts the Claude Code MCP server.
  name: mcp serve
  non_interactive: true
  notes: Long-running stdio server for another MCP client; accepts `--debug` and `--verbose`.
- description: Shows a plugin's options or saves values from stdin.
  name: plugin configure
  non_interactive: true
  notes: Prints which options are unset; `--values-stdin` reads a JSON object so no prompt is needed; `--json` machine-reads the option list.
- description: Shows a plugin's component inventory and projected token cost.
  name: plugin details
  non_interactive: true
  notes: Read-only.
- description: Disables an enabled plugin.
  name: plugin disable
  non_interactive: true
  notes: Takes a plugin name or `-a`/`--all` for every enabled plugin; scope auto-detects unless `--scope` is given.
- description: Enables a disabled plugin.
  name: plugin enable
  non_interactive: true
  notes: Scope auto-detects unless `--scope` is given.
- description: Runs a plugin's eval suite and reports scored results.
  name: plugin eval
  non_interactive: false
  notes: Large operator flag surface (--mocks, --judge-model, --concurrency, --max-cost-usd, and more) not inventoried here; the first run in an untrusted plugin directory asks for confirmation unless `--trust-plugin` is passed; makes real model calls.
- description: Scaffolds a new plugin under ~/.claude/skills/.
  name: plugin init
  non_interactive: true
  notes: Alias `new`; `--with` selects extra component scaffolds; auto-loads next session as <name>@skills-dir.
- description: Installs a plugin from a marketplace.
  name: plugin install
  non_interactive: false
  notes: Alias `i`; a marketplace-declared command must be confirmed by a person, so `-y`/`--yes` (or a pinned `--accept-command` sha256) is required when stdin or stdout is not a TTY; `--json` prints one machine-readable result line.
- description: Lists installed plugins.
  name: plugin list
  non_interactive: true
  notes: '`--json` prints a JSON array; `--available` adds marketplace plugins and `--data-size` measures saved data, both requiring `--json`.'
- description: Adds a marketplace from a URL, path, or GitHub repo.
  name: plugin marketplace add
  non_interactive: true
  notes: '`--claudeai` adds a claude.ai-hosted marketplace by name; `--sparse` limits checkout paths.'
- description: Lists configured marketplaces.
  name: plugin marketplace list
  non_interactive: true
  notes: '`--json` prints machine-readable output.'
- description: Removes a configured marketplace.
  name: plugin marketplace remove
  non_interactive: true
  notes: Alias `rm`; without `--scope`, removes the declaration from every scope.
- description: Updates marketplace definitions from their source.
  name: plugin marketplace update
  non_interactive: true
  notes: Updates all marketplaces when no name is given.
- description: Removes auto-installed plugin dependencies no longer needed.
  name: plugin prune
  non_interactive: false
  notes: Alias `autoremove`; prompts for confirmation unless `-y`/`--yes`; `--dry-run` lists what would go.
- description: Creates a validated git tag for a plugin release.
  name: plugin tag
  non_interactive: true
  notes: Validates that plugin.json and any enclosing marketplace entry agree; `--dry-run` previews and `--push` pushes to a remote.
- description: Runs a mod's TypeScript test suite.
  name: plugin test
  non_interactive: true
  notes: Runs every *.test.ts and *.test.tsx under the directory in a child of this binary; exits 1 on failure.
- description: Uninstalls an installed plugin.
  name: plugin uninstall
  non_interactive: true
  notes: Alias `remove`; `--keep-data` preserves the plugin's data directory; `--prune` also removes unneeded auto-installed dependencies but prompts, requiring `-y` in non-interactive contexts.
- description: Updates an installed plugin to its latest version.
  name: plugin update
  non_interactive: false
  notes: Same marketplace-declared-command confirmation flow as install; `-y` or `--accept-command` answers it headlessly; restart required to apply.
- description: Validates a plugin or marketplace manifest.
  name: plugin validate
  non_interactive: true
  notes: '`--json` emits the validation report; `--strict` turns warnings into exit 1 for CI.'
- description: Deletes all Claude Code state for a project.
  name: project purge
  non_interactive: false
  notes: Prompts by default; `-y`/`--yes` skips, `-i`/`--interactive` confirms each item, `--dry-run` previews, `--all` purges every project and is mutually exclusive with a path.
- description: Starts a Remote Control server for claude.ai or the mobile app.
  name: remote-control
  non_interactive: false
  notes: Hidden from top-level help; runs as a persistent server in the current directory and requires a subscribed Claude account; its own flag surface (--spawn, --capacity, --session-id, -c, --[no-]chrome) is documented in `claude remote-control --help` and not inventoried here; the root `--remote-control` flag is the interactive-session form.
- description: Restarts a background session with its conversation intact.
  name: respawn
  non_interactive: true
  notes: Takes a session id or `--all` for every running session, for example to pick up an updated binary.
- description: Deletes a background session and its worktree.
  name: rm
  non_interactive: true
  notes: Works on already-exited sessions; a refused removal prints the exact `--discard-unpushed <commit>@<worktree-id>` or `--force-remove-worktree <worktree-id>` value to re-run with; the transcript stays on disk.
- description: Registers this machine as a self-hosted cloud-session runner.
  name: self-hosted-runner
  non_interactive: false
  notes: Hidden; long-running operator surface (v2.1.224+) with setup, doctor, and orchestrator subcommands and dozens of flags; see `claude self-hosted-runner --help`; not inventoried here.
- description: Generates a long-lived OAuth token for CI and scripts.
  name: setup-token
  non_interactive: false
  notes: Requires a Claude subscription and prints a secret to the terminal without saving it; treat as an interactive secrets flow.
- description: Stops a background session while keeping it resumable.
  name: stop
  non_interactive: true
  notes: Alias `kill`; the conversation is kept and reopens with `claude attach <id>`.
- description: Runs a cloud-hosted multi-agent code review and prints findings.
  name: ultrareview
  non_interactive: true
  notes: Takes a PR number or base branch as target; exits 0 on success and 1 on failure; `--json` prints the raw payload, `--timeout` caps the wait (default 45 minutes), `--post` posts findings to a github.com PR.
- description: Checks for updates and installs if available.
  name: update
  non_interactive: true
  notes: Alias `upgrade`; behavior depends on install method and channel; reports up-to-date for package-manager installs it cannot upgrade.
cli_switches:
- attachment:
  - space
  - equals
  description: Adds directories Claude may read and edit in addition to the working directory.
  evidence_ids:
  - help-root-287
  - docs-cli-reference
  - test-parser-variadic-287
  example: claude --add-dir ../apps ../lib
  flag: --add-dir
  invocation_scope:
  - applies_to: command
    command: []
  notes: Grants file access; most .claude/ configuration is not discovered from added directories; paths are validated to exist. The first token after the switch is consumed unconditionally, even when it looks like an option, and non-option tokens keep being consumed.
  scope:
  - filesystem
  - sessions
  value: <directories...>
  value_type: variadic
  variadic_min: 1
- attachment:
  - space
  - equals
  description: Adds one directory tool calls may reach in sessions dispatched from agent view.
  evidence_ids:
  - help-subcommands-287
  example: claude agents --add-dir ../shared
  flag: --add-dir
  invocation_scope:
  - applies_to: command
    command:
    - agents
  notes: Single value per occurrence at this path, repeatable per occurrence, unlike the variadic root spelling.
  scope:
  - filesystem
  - agents
  value: <directory>
  value_optional: false
  value_type: string
- attachment:
  - space
  - equals
  description: Enables the server-side advisor tool for this session with a model alias or full model ID.
  evidence_ids:
  - docs-cli-reference
  - test-missing-arg-287
  example: claude --advisor opus
  flag: --advisor
  invocation_scope:
  - applies_to: command
    command: []
  notes: Omitted from local 2.1.287 help but present in the parser and documented; overrides the advisorModel setting.
  scope:
  - model_selection
  value: <model>
  value_optional: false
  value_type: string
- attachment:
  - space
  - equals
  description: Selects the agent for the session, overriding the agent setting.
  evidence_ids:
  - help-root-287
  - help-subcommands-287
  example: claude --agent my-custom-agent
  flag: --agent
  invocation_scope:
  - applies_to: command
    command: []
  - applies_to: command
    command:
    - agents
  notes: At `agents`, sets the default agent for dispatched sessions.
  scope:
  - subagents
  value: <agent>
  value_optional: false
  value_type: string
- attachment:
  - space
  - equals
  description: Defines custom subagents dynamically from a JSON object, or with --print the path to a file holding one.
  evidence_ids:
  - help-root-287
  - docs-cli-reference
  - test-missing-arg-287
  example: claude --agents '{"reviewer":{"description":"Reviews code","prompt":"You are a code reviewer"}}'
  flag: --agents
  invocation_scope:
  - applies_to: command
    command: []
  notes: Validated at startup since v2.1.242; the file form requires v2.1.281+.
  scope:
  - subagents
  value: <json-or-file>
  value_optional: false
  value_type: string
- attachment: []
  default: 'false'
  description: Makes bypassPermissions available in the mode cycle without starting in it.
  evidence_ids:
  - help-root-287
  - help-subcommands-287
  - docs-cli-reference
  example: claude --permission-mode plan --allow-dangerously-skip-permissions
  flag: --allow-dangerously-skip-permissions
  invocation_scope:
  - applies_to: command
    command: []
  - applies_to: command
    command:
    - agents
  notes: A leading occurrence of this switch also routes `daemon <subcommand>` to the daemon command since v2.1.199.
  scope:
  - permissions
  value_type: none
- aliases:
  - --allowed-tools
  attachment:
  - space
  - equals
  description: Tools that execute without prompting for permission.
  evidence_ids:
  - help-root-287
  - docs-cli-reference
  example: claude --allowedTools "Bash(git log *)" Read
  flag: --allowedTools
  invocation_scope:
  - applies_to: command
    command: []
  notes: Comma or space separated within and across values; use --tools to restrict which tools exist at all.
  scope:
  - permissions
  value: <tools...>
  value_type: variadic
  variadic_min: 1
- attachment:
  - space
  - equals
  description: Appends text to every subagent's system prompt in non-interactive mode.
  evidence_ids:
  - docs-cli-reference
  - test-missing-arg-287
  example: claude -p --append-subagent-system-prompt "Cite file paths" "query"
  flag: --append-subagent-system-prompt
  invocation_scope:
  - applies_to: command
    command: []
  notes: Only applies with -p; requires v2.1.205+; cannot be combined with its -file form before v2.1.283.
  scope:
  - system_prompt
  - subagents
  value: <prompt>
  value_optional: false
  value_type: string
- attachment:
  - space
  - equals
  description: Appends file contents to subagent system prompts in non-interactive mode.
  evidence_ids:
  - docs-cli-reference
  - test-missing-arg-287
  example: claude -p --append-subagent-system-prompt-file ./subagent-rules.txt "query"
  flag: --append-subagent-system-prompt-file
  invocation_scope:
  - applies_to: command
    command: []
  notes: Only applies with -p; requires v2.1.261+.
  scope:
  - system_prompt
  - subagents
  value: <file>
  value_optional: false
  value_type: string
- attachment:
  - space
  - equals
  description: Appends text to the default system prompt.
  evidence_ids:
  - help-root-287
  - docs-cli-reference
  example: claude --append-system-prompt "Always use TypeScript"
  flag: --append-system-prompt
  invocation_scope:
  - applies_to: command
    command: []
  notes: Works in interactive and non-interactive modes; semantics belong to the system-prompt topic.
  scope:
  - system_prompt
  value: <prompt>
  value_optional: false
  value_type: string
- attachment:
  - space
  - equals
  description: Appends file contents to the default system prompt.
  evidence_ids:
  - docs-cli-reference
  - test-missing-arg-287
  example: claude --append-system-prompt-file ./extra-rules.txt
  flag: --append-system-prompt-file
  invocation_scope:
  - applies_to: command
    command: []
  notes: Omitted from local help but present in the parser and documented; semantics belong to the system-prompt topic.
  scope:
  - system_prompt
  value: <file>
  value_optional: false
  value_type: string
- attachment:
  - space
  - equals
  description: Sets the auto-compact window for the session without changing saved settings.
  evidence_ids:
  - help-root-287
  - docs-cli-reference
  - test-parser-forms-287
  example: claude --autocompact 500k
  flag: --autocompact
  invocation_scope:
  - applies_to: command
    command: []
  notes: 'Validated at parse time: auto, or between 100k and 1M tokens; requires v2.1.221+.'
  scope:
  - model_selection
  value: <auto|tokens>
  value_optional: false
  value_type: string
- attachment: []
  default: 'false'
  description: Renders screen-reader friendly flat output.
  evidence_ids:
  - help-root-287
  - docs-cli-reference
  example: claude --ax-screen-reader
  flag: --ax-screen-reader
  invocation_scope:
  - applies_to: command
    command: []
  notes: Forces the classic renderer; overrides CLAUDE_AX_SCREEN_READER and the axScreenReader setting; requires v2.1.181+.
  scope:
  - accessibility
  value_type: none
- aliases:
  - --background
  attachment: []
  default: 'false'
  description: Starts the session as a background agent and returns immediately, printing the id that attach, logs, stop, and rm take.
  evidence_ids:
  - help-root-287
  - docs-cli-reference
  example: claude --bg "investigate the flaky test"
  flag: --bg
  invocation_scope:
  - applies_to: command
    command: []
  notes: Cannot be combined with -p; checks workspace trust before starting; with --resume, continues that session in the background under the same id.
  scope:
  - background_agents
  value_type: none
- attachment: []
  default: 'false'
  description: Minimal mode that skips most customization discovery so scripted calls start faster.
  evidence_ids:
  - help-root-287
  - docs-cli-reference
  example: claude --bare -p "query"
  flag: --bare
  invocation_scope:
  - applies_to: command
    command: []
  notes: Sets CLAUDE_CODE_SIMPLE=1; Anthropic auth is strictly ANTHROPIC_API_KEY or apiKeyHelper via --settings, so OAuth and keychain are never read.
  scope:
  - config
  - print_mode
  value_type: none
- attachment:
  - space
  - equals
  description: Adds beta headers to API requests.
  evidence_ids:
  - help-root-287
  - docs-cli-reference
  example: claude --betas interleaved-thinking
  flag: --betas
  invocation_scope:
  - applies_to: command
    command: []
  notes: API key users only; the ANTHROPIC_BETAS variable works with all auth methods.
  scope:
  - api
  value: <betas...>
  value_type: variadic
  variadic_min: 1
- attachment: []
  default: 'false'
  description: Enables the SendUserMessage tool for agent-to-user communication.
  evidence_ids:
  - help-root-287
  example: claude --brief
  flag: --brief
  invocation_scope:
  - applies_to: command
    command: []
  notes: Observed in local help but not in the CLI-reference flag table.
  scope:
  - output
  value_type: none
- attachment:
  - space
  - equals
  description: Subscribes to MCP channel notifications for this session.
  evidence_ids:
  - docs-cli-reference
  - test-missing-arg-287
  example: claude --channels plugin:my-notifier@my-marketplace
  flag: --channels
  invocation_scope:
  - applies_to: command
    command: []
  notes: Research preview; space-separated plugin:<name>@<marketplace> entries; requires Anthropic authentication. Omitted from local help but variadic in the parser's missing-argument error.
  scope:
  - channels
  value: <servers...>
  value_type: variadic
  variadic_min: 1
- attachment: []
  default: 'false'
  description: Enables Claude in Chrome integration.
  evidence_ids:
  - help-root-287
  - docs-cli-reference
  example: claude --chrome
  flag: --chrome
  invocation_scope:
  - applies_to: command
    command: []
  notes: Commander also exposes the automatic --no-chrome negation.
  scope:
  - browser
  value_type: none
- aliases:
  - --remote
  attachment:
  - space
  - equals
  description: Creates a cloud session with a task description, or attaches to one by session ID or claude.ai/code URL.
  evidence_ids:
  - help-root-287
  - docs-cli-reference
  - test-remote-alias-287
  example: claude --cloud "Fix the login bug"
  flag: --cloud
  invocation_scope:
  - applies_to: command
    command: []
  notes: With -p and a session id or URL, queues a message into that existing session; --remote is the deprecated alias, and an error raised by --remote names --cloud; without -p it requires an interactive terminal.
  scope:
  - remote
  value: '[description|session_id|url]'
  value_optional: true
  value_type: string
- aliases:
  - -c
  attachment: []
  default: 'false'
  description: Continues the most recent conversation in the current directory.
  evidence_ids:
  - help-root-287
  - docs-cli-reference
  example: claude -c -p "Check for type errors"
  flag: --continue
  invocation_scope:
  - applies_to: command
    command: []
  notes: Includes finished background sessions since v2.1.257; -p --continue also includes -p, SDK, and /loop sessions.
  scope:
  - sessions
  value_type: none
- attachment:
  - space
  - equals
  description: Enables channels that are not on the approved allowlist, for local development.
  evidence_ids:
  - docs-cli-reference
  - test-missing-arg-287
  example: claude --dangerously-load-development-channels server:webhook
  flag: --dangerously-load-development-channels
  invocation_scope:
  - applies_to: command
    command: []
  notes: Accepts plugin:<name>@<marketplace> and server:<name> entries; prompts for confirmation; omitted from local help but variadic in the parser's missing-argument error.
  scope:
  - channels
  value: <servers...>
  value_type: variadic
  variadic_min: 1
- attachment: []
  default: 'false'
  description: Starts with permission checks bypassed.
  evidence_ids:
  - help-root-287
  - help-subcommands-287
  - docs-cli-reference
  example: claude --dangerously-skip-permissions
  flag: --dangerously-skip-permissions
  invocation_scope:
  - applies_to: command
    command: []
  - applies_to: command
    command:
    - agents
  notes: Equivalent to --permission-mode bypassPermissions; a leading occurrence also routes `daemon <subcommand>` to the daemon command since v2.1.199.
  scope:
  - permissions
  value_type: none
- aliases:
  - -d
  attachment:
  - equals
  default: 'false'
  description: Enables debug mode, optionally filtered by category.
  evidence_ids:
  - help-root-287
  - docs-cli-reference
  - help-subcommands-287
  example: claude --debug='api,hooks'
  flag: --debug
  invocation_scope:
  - applies_to: command
    command: []
  notes: The filter binds only in the = form; a space-separated filter enables debug mode without filtering, so a wrapper must always write --debug=filter. The hidden remote-control help spells the same shape --debug[=<filter>].
  scope:
  - diagnostics
  value: '[filter]'
  value_optional: true
  value_type: string
- attachment:
  - space
  - equals
  description: Writes debug logs to a file, implicitly enabling debug mode.
  evidence_ids:
  - help-root-287
  - docs-cli-reference
  example: claude --debug-file /tmp/claude-debug.log
  flag: --debug-file
  invocation_scope:
  - applies_to: command
    command: []
  notes: Takes precedence over CLAUDE_CODE_DEBUG_LOGS_DIR.
  scope:
  - diagnostics
  value: <path>
  value_optional: false
  value_type: string
- attachment: []
  default: 'false'
  description: Opens the Claude Desktop app on the current directory instead of starting a terminal session.
  evidence_ids:
  - help-root-287
  - docs-cli-reference
  example: claude --desktop
  flag: --desktop
  invocation_scope:
  - applies_to: command
    command: []
  notes: Takes no prompt and no other flags except --verbose and the --debug flags; available on macOS and x64 Windows with a Claude subscription; requires v2.1.285+.
  scope:
  - desktop
  value_type: none
- attachment: []
  default: 'false'
  description: Disables all skills and slash commands for the session.
  evidence_ids:
  - help-root-287
  - docs-cli-reference
  example: claude --disable-slash-commands
  flag: --disable-slash-commands
  invocation_scope:
  - applies_to: command
    command: []
  notes: Local help phrases it as disabling all skills.
  scope:
  - skills
  value_type: none
- aliases:
  - --disallowed-tools
  attachment:
  - space
  - equals
  description: Deny rules for tool calls; a bare tool name removes the tool from context.
  evidence_ids:
  - help-root-287
  - docs-cli-reference
  example: claude --disallowedTools "Bash(rm *)" Edit
  flag: --disallowedTools
  invocation_scope:
  - applies_to: command
    command: []
  notes: A scoped rule such as Bash(rm *) leaves the tool available and denies matching calls; mcp__* removes every MCP tool.
  scope:
  - permissions
  value: <tools...>
  value_type: variadic
  variadic_min: 1
- attachment:
  - space
  - equals
  description: Sets the effort level for the current session.
  evidence_ids:
  - help-root-287
  - help-subcommands-287
  - test-effort-287
  example: claude --effort high
  flag: --effort
  invocation_scope:
  - applies_to: command
    command: []
  - applies_to: command
    command:
    - agents
  notes: Help lists low, medium, high, xhigh, max and docs add ultracode; an invalid value warns, is ignored, and the default effort applies. At `agents`, sets the default for dispatched sessions. CLAUDE_CODE_EFFORT_LEVEL overrides the flag.
  scope:
  - model_selection
  value: <level>
  value_optional: false
  value_type: string
- attachment: []
  default: removed
  description: Removed flag; auto mode is now in the mode cycle by default.
  evidence_ids:
  - docs-cli-reference
  - test-missing-arg-287
  - test-unknown-strict-287
  example: claude --permission-mode auto
  flag: --enable-auto-mode
  invocation_scope:
  - applies_to: command
    command: []
  notes: Removed in v2.1.111; the 2.1.287 parser still accepts it as a no-op instead of erroring, and the parser rejects genuinely unknown options, so the quiet parse means real acceptance; wrappers must not emit it; use --permission-mode auto.
  scope:
  - permissions
  value_type: none
- attachment:
  - space
  - equals
  description: Creates a new cloud session on the named self-hosted environment.
  evidence_ids:
  - help-root-287
  - docs-cli-reference
  - test-missing-arg-287
  example: claude -p "Run the smoke test" --environment ccpool_abc123
  flag: --environment
  invocation_scope:
  - applies_to: command
    command: []
  notes: Environment IDs start with ccpool_; requires v2.1.224+; combine with --ref to base the checkout on a named ref.
  scope:
  - remote
  value: <environment_id>
  value_optional: false
  value_type: string
- attachment: []
  default: 'false'
  description: Moves per-machine system prompt sections into the first user message.
  evidence_ids:
  - help-root-287
  - docs-cli-reference
  example: claude -p --exclude-dynamic-system-prompt-sections "query"
  flag: --exclude-dynamic-system-prompt-sections
  invocation_scope:
  - applies_to: command
    command: []
  notes: Only applies with the default system prompt; semantics belong to the system-prompt topic.
  scope:
  - system_prompt
  value_type: none
- attachment:
  - space
  - equals
  description: Enables automatic fallback to the named model or models when the primary is overloaded or unavailable.
  evidence_ids:
  - help-root-287
  - docs-cli-reference
  - test-missing-arg-287
  example: claude --fallback-model sonnet,haiku
  flag: --fallback-model
  invocation_scope:
  - applies_to: command
    command: []
  notes: One value holding a comma-separated chain tried in order; overrides the fallbackModel setting.
  scope:
  - model_selection
  value: <model>
  value_optional: false
  value_type: string
- attachment:
  - space
  - equals
  description: Downloads file resources at startup.
  evidence_ids:
  - help-root-287
  example: claude --file file_abc:doc.txt file_def:img.png
  flag: --file
  invocation_scope:
  - applies_to: command
    command: []
  notes: Format file_id:relative_path; observed in local help but not in the CLI-reference flag table.
  scope:
  - input
  value: <specs...>
  value_type: variadic
  variadic_min: 1
- attachment: []
  default: 'false'
  description: Creates a new session ID when resuming instead of reusing the original.
  evidence_ids:
  - help-root-287
  - docs-cli-reference
  example: claude --resume abc123 --fork-session
  flag: --fork-session
  invocation_scope:
  - applies_to: command
    command: []
  notes: Use with --resume or --continue.
  scope:
  - sessions
  value_type: none
- attachment: []
  default: 'false'
  description: Emits subagent text and thinking blocks in the output stream with parent_tool_use_id set.
  evidence_ids:
  - help-root-287
  - docs-cli-reference
  example: claude -p --output-format stream-json --verbose --forward-subagent-text "query"
  flag: --forward-subagent-text
  invocation_scope:
  - applies_to: command
    command: []
  notes: Requires --print and --output-format stream-json; CLAUDE_CODE_FORWARD_SUBAGENT_TEXT does the same; requires v2.1.211+.
  scope:
  - print_mode
  - subagents
  value_type: none
- attachment:
  - space
  - equals
  description: Resumes a session linked to a pull request, or opens the picker filtered to it.
  evidence_ids:
  - help-root-287
  - docs-cli-reference
  - test-parser-forms-287
  example: claude --from-pr 123
  flag: --from-pr
  invocation_scope:
  - applies_to: command
    command: []
  notes: Accepts a PR number or a GitHub, GitHub Enterprise, GitLab, or Bitbucket URL; without a value opens an interactive picker; does not consume a following token that looks like an option.
  scope:
  - sessions
  value: '[value]'
  value_optional: true
  value_type: string
- aliases:
  - -h
  attachment: []
  description: Displays help for the command.
  evidence_ids:
  - help-root-287
  - help-subcommands-287
  example: claude --help
  flag: --help
  invocation_scope:
  - applies_to: global
  notes: Present at every command path probed; help is incomplete by design, so absence from help does not mean a flag is unavailable.
  scope:
  - diagnostics
  value_type: none
- attachment: []
  default: 'false'
  description: Connects to an IDE on startup when exactly one valid IDE is available.
  evidence_ids:
  - help-root-287
  - docs-cli-reference
  example: claude --ide
  flag: --ide
  invocation_scope:
  - applies_to: command
    command: []
  notes: CLAUDE_CODE_AUTO_CONNECT_IDE overrides auto-detection.
  scope:
  - ide
  value_type: none
- attachment: []
  default: 'false'
  description: Includes hook lifecycle events in the output stream.
  evidence_ids:
  - help-root-287
  - docs-cli-reference
  example: claude -p --output-format stream-json --verbose --include-hook-events "query"
  flag: --include-hook-events
  invocation_scope:
  - applies_to: command
    command: []
  notes: Requires --output-format stream-json; SessionStart and Setup events are always included; some events never produce hook_started even with the flag.
  scope:
  - print_mode
  - hooks
  value_type: none
- attachment: []
  default: 'false'
  description: Includes partial streaming events in output as they arrive.
  evidence_ids:
  - help-root-287
  - docs-cli-reference
  example: claude -p --output-format stream-json --verbose --include-partial-messages "query"
  flag: --include-partial-messages
  invocation_scope:
  - applies_to: command
    command: []
  notes: Requires --print and --output-format stream-json.
  scope:
  - print_mode
  value_type: none
- attachment: []
  default: 'false'
  description: Runs Setup hooks with the init matcher before a print-mode session.
  evidence_ids:
  - docs-cli-reference
  - test-missing-arg-287
  example: claude -p --init "query"
  flag: --init
  invocation_scope:
  - applies_to: command
    command: []
  notes: Omitted from local help but accepted by the parser; print mode only.
  scope:
  - hooks
  - print_mode
  value_type: none
- attachment: []
  default: 'false'
  description: Runs Setup and SessionStart hooks, then exits without starting a conversation.
  evidence_ids:
  - docs-cli-reference
  - test-missing-arg-287
  example: claude --init-only
  flag: --init-only
  invocation_scope:
  - applies_to: command
    command: []
  notes: Omitted from local help but accepted; a local bare run completed silently with exit 0; also counts as a non-interactive invocation that refuses --cloud.
  scope:
  - hooks
  value_type: none
- attachment:
  - space
  - equals
  default: text
  description: Selects the print-mode input format.
  evidence_ids:
  - help-root-287
  - test-parser-forms-287
  example: claude -p --output-format json --input-format stream-json
  flag: --input-format
  invocation_scope:
  - applies_to: command
    command: []
  notes: Choices text and stream-json validated at parse time in both space and equals forms; only works with --print.
  scope:
  - print_mode
  value: <format>
  value_optional: false
  value_type: string
- attachment:
  - space
  - equals
  description: Validates final structured output against a JSON Schema.
  evidence_ids:
  - help-root-287
  - docs-cli-reference
  - test-missing-arg-287
  example: claude -p --json-schema '{"type":"object"}' "query"
  flag: --json-schema
  invocation_scope:
  - applies_to: command
    command: []
  notes: Print mode only; an invalid schema exits with an error.
  scope:
  - print_mode
  value: <schema>
  value_optional: false
  value_type: string
- attachment: []
  default: 'false'
  description: Runs Setup hooks with the maintenance matcher before a print-mode session.
  evidence_ids:
  - docs-cli-reference
  - test-missing-arg-287
  example: claude -p --maintenance "query"
  flag: --maintenance
  invocation_scope:
  - applies_to: command
    command: []
  notes: Omitted from local help but accepted by the parser; print mode only.
  scope:
  - hooks
  - print_mode
  value_type: none
- attachment:
  - space
  - equals
  description: Stops print-mode execution after a dollar budget is reached.
  evidence_ids:
  - help-root-287
  - docs-cli-reference
  example: claude -p --max-budget-usd 5.00 "query"
  flag: --max-budget-usd
  invocation_scope:
  - applies_to: command
    command: []
  notes: Subagent spend counts toward the cap; only works with --print.
  scope:
  - print_mode
  - costs
  value: <amount>
  value_optional: false
  value_type: number
- attachment:
  - space
  - equals
  default: unlimited
  description: Limits the number of agentic turns and exits with an error when the limit is reached.
  evidence_ids:
  - docs-cli-reference
  - test-missing-arg-287
  example: claude -p --max-turns 3 "query"
  flag: --max-turns
  invocation_scope:
  - applies_to: command
    command: []
  notes: Omitted from local help but accepted by the parser; print mode only; with stream-json input a still-queued message starts a new turn with its own limit.
  scope:
  - print_mode
  value: <turns>
  value_optional: false
  value_type: number
- attachment:
  - space
  - equals
  description: Loads MCP servers from JSON files or inline JSON strings.
  evidence_ids:
  - help-root-287
  - docs-cli-reference
  example: claude --mcp-config ./mcp.json
  flag: --mcp-config
  invocation_scope:
  - applies_to: command
    command: []
  notes: Space-separated; with -p, waits for pending servers up to MCP_TIMEOUT (30 s default) before the first turn; use --strict-mcp-config to ignore discovered MCP configuration.
  scope:
  - mcp
  value: <configs...>
  value_type: variadic
  variadic_min: 1
- attachment:
  - space
  - equals
  description: Loads one MCP configuration for sessions dispatched from agent view.
  evidence_ids:
  - help-subcommands-287
  example: claude agents --mcp-config ./mcp.json
  flag: --mcp-config
  invocation_scope:
  - applies_to: command
    command:
    - agents
  notes: Single value per occurrence at this path, repeatable per occurrence, unlike the variadic root spelling.
  scope:
  - mcp
  - agents
  value: <config>
  value_optional: false
  value_type: string
- attachment:
  - space
  - equals
  description: Sets the model alias or full model ID for the session.
  evidence_ids:
  - help-root-287
  - help-subcommands-287
  example: claude --model claude-sonnet-5
  flag: --model
  invocation_scope:
  - applies_to: command
    command: []
  - applies_to: command
    command:
    - agents
  - applies_to: command
    command:
    - auto-mode
    - critique
  notes: No short alias; overrides the model setting and ANTHROPIC_MODEL; at `agents` sets the dispatched-session default; at `auto-mode critique` overrides the critique model.
  scope:
  - model_selection
  value: <model>
  value_optional: false
  value_type: string
- aliases:
  - -n
  attachment:
  - space
  - equals
  - short_attached
  description: Sets a display name for the session.
  evidence_ids:
  - help-root-287
  - docs-cli-reference
  - test-parser-forms-287
  example: claude -n my-feature-work
  flag: --name
  invocation_scope:
  - applies_to: command
    command: []
  notes: Shown in /resume and the terminal title; a live duplicate name gets a variant; the short-attached form was proven locally (-nfoo parsed as a name).
  scope:
  - sessions
  value: <name>
  value_optional: false
  value_type: string
- attachment: []
  default: 'false'
  description: Disables Claude in Chrome integration for the session.
  evidence_ids:
  - help-root-287
  - docs-cli-reference
  example: claude --no-chrome
  flag: --no-chrome
  invocation_scope:
  - applies_to: command
    command: []
  notes: Commander's automatic negation of --chrome.
  scope:
  - browser
  value_type: none
- attachment: []
  default: 'false'
  description: Disables saving sessions to disk so they cannot be resumed.
  evidence_ids:
  - help-root-287
  - docs-cli-reference
  example: claude -p --no-session-persistence "query"
  flag: --no-session-persistence
  invocation_scope:
  - applies_to: command
    command: []
  notes: Print mode only; CLAUDE_CODE_SKIP_PROMPT_HISTORY does the same in any mode.
  scope:
  - print_mode
  - sessions
  value_type: none
- attachment:
  - space
  - equals
  default: text
  description: Selects the print-mode output format.
  evidence_ids:
  - help-root-287
  - test-parser-forms-287
  example: claude -p "query" --output-format json
  flag: --output-format
  invocation_scope:
  - applies_to: command
    command: []
  notes: Choices text, json, stream-json validated at parse time in both space and equals forms; use stream-json for structured streaming wrappers.
  scope:
  - print_mode
  value: <format>
  value_optional: false
  value_type: string
- attachment:
  - space
  - equals
  description: Sets the starting permission mode.
  evidence_ids:
  - help-root-287
  - help-subcommands-287
  - test-parser-forms-287
  example: claude --permission-mode plan
  flag: --permission-mode
  invocation_scope:
  - applies_to: command
    command: []
  - applies_to: command
    command:
    - agents
  notes: 'Choices validated at parse time in both forms: acceptEdits, auto, bypassPermissions, manual, dontAsk, plan; manual is an alias of default (v2.1.200+); overrides defaultMode from settings.'
  scope:
  - permissions
  value: <mode>
  value_optional: false
  value_type: string
- attachment:
  - space
  - equals
  description: Delegates non-interactive permission prompts to an MCP tool.
  evidence_ids:
  - docs-cli-reference
  - test-missing-arg-287
  example: claude -p --permission-prompt-tool mcp_auth_tool "query"
  flag: --permission-prompt-tool
  invocation_scope:
  - applies_to: command
    command: []
  notes: Omitted from local help but accepted by the parser; waits for the tool's server up to MCP_TIMEOUT.
  scope:
  - print_mode
  - permissions
  value: <tool>
  value_optional: false
  value_type: string
- attachment:
  - space
  - equals
  default: host
  description: Sets who answers permission prompts in print mode.
  evidence_ids:
  - help-root-287
  - test-parser-forms-287
  example: claude -p --permission-prompts none "query"
  flag: --permission-prompts
  invocation_scope:
  - applies_to: command
    command: []
  notes: Choices host and none validated at parse time; none denies anything that would prompt; requires v2.1.259+.
  scope:
  - print_mode
  - permissions
  value: <target>
  value_optional: false
  value_type: string
- attachment:
  - space
  - equals
  default: '[]'
  description: Loads a plugin from a directory or .zip for this session only.
  evidence_ids:
  - help-root-287
  - help-subcommands-287
  - docs-cli-reference
  example: claude --plugin-dir ./my-plugin
  flag: --plugin-dir
  invocation_scope:
  - applies_to: command
    command: []
  - applies_to: command
    command:
    - agents
  notes: One path per occurrence; repeat the flag for more; a folder of plugins loads each child (v2.1.265+).
  scope:
  - plugins
  value: <path>
  value_optional: false
  value_type: string
- attachment:
  - space
  - equals
  default: '[]'
  description: Fetches a plugin .zip from a URL for this session only.
  evidence_ids:
  - help-root-287
  - docs-cli-reference
  example: claude --plugin-url https://example.com/plugin.zip
  flag: --plugin-url
  invocation_scope:
  - applies_to: command
    command: []
  notes: Repeat the flag for multiple plugins or pass space-separated URLs in one quoted value.
  scope:
  - plugins
  value: <url>
  value_optional: false
  value_type: string
- aliases:
  - -p
  attachment: []
  default: 'false'
  description: Prints a response and exits without interactive mode.
  evidence_ids:
  - help-root-287
  - docs-cli-reference
  - test-remote-alias-287
  example: claude -p "query"
  flag: --print
  invocation_scope:
  - applies_to: command
    command: []
  notes: Skips the workspace trust dialog and silently ignores settings files that fail validation; piped stdout alone also selects the same non-interactive input requirement even without -p.
  scope:
  - print_mode
  value_type: none
- attachment:
  - space
  - equals
  default: 'true'
  description: Emits prompt_suggestion messages after turns that generate one.
  evidence_ids:
  - help-root-287
  - test-parser-forms-287
  example: claude -p --prompt-suggestions --output-format stream-json --verbose "query"
  flag: --prompt-suggestions
  invocation_scope:
  - applies_to: command
    command: []
  notes: Optional value with choices true, false, 1, 0, yes, no, on, off validated at parse time in both space and equals forms; requires --print, stream-json output, and --verbose.
  scope:
  - print_mode
  value: '[value]'
  value_optional: true
  value_type: string
- attachment:
  - space
  - equals
  description: Bases a new --environment session's checkout on a named ref instead of local HEAD.
  evidence_ids:
  - docs-cli-reference
  - test-missing-arg-287
  example: claude -p "Run the smoke test" --environment ccpool_abc123 --ref main
  flag: --ref
  invocation_scope:
  - applies_to: command
    command: []
  notes: Documented alongside --environment; omitted from local help but accepted by the parser.
  scope:
  - remote
  value: <ref>
  value_optional: false
  value_type: string
- aliases:
  - --rc
  attachment:
  - space
  - equals
  description: Starts an interactive session with Remote Control enabled, optionally named.
  evidence_ids:
  - help-root-287
  - docs-cli-reference
  - test-missing-arg-287
  example: claude --remote-control "My Project"
  flag: --remote-control
  invocation_scope:
  - applies_to: command
    command: []
  notes: The hidden `claude remote-control` subcommand is the server-mode form with its own flags; --rc parses as this flag's alias.
  scope:
  - remote
  value: '[name]'
  value_optional: true
  value_type: string
- attachment:
  - space
  - equals
  default: hostname
  description: Sets the prefix for auto-generated Remote Control session names.
  evidence_ids:
  - help-root-287
  - docs-cli-reference
  example: claude --remote-control-session-name-prefix dev-box
  flag: --remote-control-session-name-prefix
  invocation_scope:
  - applies_to: command
    command: []
  notes: CLAUDE_REMOTE_CONTROL_SESSION_NAME_PREFIX does the same; also accepted at the hidden remote-control subcommand.
  scope:
  - remote
  value: <prefix>
  value_optional: false
  value_type: string
- attachment: []
  default: 'false'
  description: Re-emits stdin user messages to stdout for acknowledgement.
  evidence_ids:
  - help-root-287
  - docs-cli-reference
  example: claude -p --input-format stream-json --output-format stream-json --verbose --replay-user-messages
  flag: --replay-user-messages
  invocation_scope:
  - applies_to: command
    command: []
  notes: Requires stream-json input and output.
  scope:
  - print_mode
  value_type: none
- attachment: []
  default: 'false'
  description: Starts in restricted mode, removing built-in tools that run commands or code and ignoring user, project, and local settings.
  evidence_ids:
  - help-root-287
  - help-subcommands-287
  - docs-cli-reference
  example: claude --restricted -p "query"
  flag: --restricted
  invocation_scope:
  - applies_to: command
    command: []
  - applies_to: command
    command:
    - agents
  notes: Confines file tools to working directories, refuses bypassPermissions, and refuses cloud sessions; managed settings and --settings still apply; requires v2.1.248+.
  scope:
  - permissions
  value_type: none
- aliases:
  - -r
  attachment:
  - space
  - equals
  - short_attached
  description: Resumes a session by ID, name, or transcript path, or opens an interactive picker.
  evidence_ids:
  - help-root-287
  - docs-cli-reference
  - test-parser-forms-287
  example: claude --resume auth-refactor
  flag: --resume
  invocation_scope:
  - applies_to: command
    command: []
  notes: The picker needs interaction; a value may also be an absolute path to a .jsonl transcript; resuming a running background session attaches to it (v2.1.285+); the short-attached form was proven locally (-rbar parsed as a value).
  scope:
  - sessions
  value: '[value]'
  value_optional: true
  value_type: string
- attachment: []
  default: 'false'
  description: Starts with all customizations disabled for troubleshooting.
  evidence_ids:
  - help-root-287
  - docs-cli-reference
  example: claude --safe-mode
  flag: --safe-mode
  invocation_scope:
  - applies_to: command
    command: []
  notes: Sets CLAUDE_CODE_SAFE_MODE=1; auth, model selection, built-in tools, and permissions work normally; managed policy still partly applies.
  scope:
  - config
  value_type: none
- attachment:
  - space
  - equals
  description: Puts the CLI into the SDK's non-interactive invocation mode.
  evidence_ids:
  - test-remote-alias-287
  - test-missing-arg-287
  flag: --sdk-url
  invocation_scope:
  - applies_to: command
    command: []
  notes: 'Hidden: absent from help and the CLI reference; revealed by a --cloud error message listing non-interactive entry modes (piped stdout, --init-only, --sdk-url) and by the parser''s missing-argument error.'
  scope:
  - sdk
  value: <url>
  value_optional: false
  value_type: string
- attachment:
  - space
  - equals
  description: Uses a specific session ID for the conversation.
  evidence_ids:
  - help-root-287
  - test-session-id-287
  example: claude --session-id 550e8400-e29b-41d4-a716-446655440000
  flag: --session-id
  invocation_scope:
  - applies_to: command
    command: []
  notes: Must be a valid UUID; an invalid value is rejected app-side before any session starts.
  scope:
  - sessions
  value: <uuid>
  value_optional: false
  value_type: string
- attachment:
  - space
  - equals
  default: user,project,local
  description: Restricts which setting sources are loaded.
  evidence_ids:
  - help-root-287
  - help-subcommands-287
  - test-parser-variadic-287
  example: claude --setting-sources user,project
  flag: --setting-sources
  invocation_scope:
  - applies_to: command
    command: []
  - applies_to: command
    command:
    - agents
  notes: One comma-separated value; validated app-side with a fatal error naming the valid set; inherited by sessions started from this one.
  scope:
  - config
  value: <sources>
  value_optional: false
  value_type: string
- attachment:
  - space
  - equals
  description: Loads additional settings from a JSON file or inline JSON string.
  evidence_ids:
  - help-root-287
  - help-subcommands-287
  - docs-cli-reference
  example: claude --settings ./settings.json
  flag: --settings
  invocation_scope:
  - applies_to: command
    command: []
  - applies_to: command
    command:
    - agents
  notes: Overrides same keys in settings files for this session; the file must be a regular file no larger than 2 MiB; managed values still win.
  scope:
  - config
  value: <file-or-json>
  value_optional: false
  value_type: string
- attachment: []
  default: 'false'
  description: Uses only MCP servers from --mcp-config, ignoring all other MCP configurations.
  evidence_ids:
  - help-root-287
  - help-subcommands-287
  example: claude --strict-mcp-config --mcp-config ./mcp.json
  flag: --strict-mcp-config
  invocation_scope:
  - applies_to: command
    command: []
  - applies_to: command
    command:
    - agents
  notes: At `agents`, applies to dispatched sessions.
  scope:
  - mcp
  value_type: none
- attachment:
  - space
  - equals
  description: Replaces the default system prompt with inline text.
  evidence_ids:
  - help-root-287
  - docs-cli-reference
  example: claude --system-prompt "You are a Python expert"
  flag: --system-prompt
  invocation_scope:
  - applies_to: command
    command: []
  notes: Semantics belong to the system-prompt topic.
  scope:
  - system_prompt
  value: <prompt>
  value_optional: false
  value_type: string
- attachment:
  - space
  - equals
  description: Replaces the default system prompt with file contents.
  evidence_ids:
  - docs-cli-reference
  example: claude --system-prompt-file ./custom-prompt.txt
  flag: --system-prompt-file
  invocation_scope:
  - applies_to: command
    command: []
  notes: Omitted from local help; documented in the CLI reference; semantics belong to the system-prompt topic.
  scope:
  - system_prompt
  value: <path>
  value_optional: false
  value_type: string
- attachment:
  - space
  - equals
  default: on
  description: Controls whether a conversation records its system prompt once and reuses it.
  evidence_ids:
  - help-root-287
  - test-parser-forms-287
  example: claude --append-system-prompt "Draft rules" --system-prompt-snapshot off
  flag: --system-prompt-snapshot
  invocation_scope:
  - applies_to: command
    command: []
  notes: Choices on and off validated at parse time; requires v2.1.257+; semantics belong to the system-prompt topic.
  scope:
  - system_prompt
  value: <on|off>
  value_optional: false
  value_type: string
- attachment:
  - space
  - equals
  description: Resumes a cloud session in the local terminal, optionally by session ID.
  evidence_ids:
  - help-root-287
  - docs-cli-reference
  example: claude --teleport
  flag: --teleport
  invocation_scope:
  - applies_to: command
    command: []
  notes: Requires a claude.ai subscription.
  scope:
  - remote
  value: '[session]'
  value_optional: true
  value_type: string
- attachment:
  - space
  - equals
  default: in-process
  description: Sets how agent team teammates display.
  evidence_ids:
  - docs-cli-reference
  - test-missing-arg-287
  example: claude --teammate-mode auto
  flag: --teammate-mode
  invocation_scope:
  - applies_to: command
    command: []
  notes: Values in-process, auto, tmux, iterm2; overrides the teammateMode setting; omitted from local help but accepted by the parser.
  scope:
  - agents
  value: <mode>
  value_optional: false
  value_type: string
- attachment:
  - equals
  default: 'false'
  description: Creates a tmux session for the worktree, using iTerm2 native panes when available.
  evidence_ids:
  - help-root-287
  - docs-cli-reference
  example: claude -w feature-auth --tmux=classic
  flag: --tmux
  invocation_scope:
  - applies_to: command
    command: []
  notes: Requires --worktree; the classic spelling is documented only in the equals form, which is why attachment records equals alone.
  scope:
  - worktree
  value: classic
  value_optional: true
  value_type: string
- attachment:
  - space
  - equals
  default: default
  description: Restricts which built-in tools are available.
  evidence_ids:
  - help-root-287
  - docs-cli-reference
  example: claude --tools "Bash,Edit,Read"
  flag: --tools
  invocation_scope:
  - applies_to: command
    command: []
  notes: Use "" to disable all and "default" for the default set; does not affect MCP tools, which --disallowedTools governs.
  scope:
  - permissions
  value: <tools...>
  value_type: variadic
  variadic_min: 1
- attachment: []
  default: 'false'
  description: Enables verbose turn-by-turn output, overriding the viewMode setting.
  evidence_ids:
  - help-root-287
  - help-subcommands-287
  example: claude --verbose
  flag: --verbose
  invocation_scope:
  - applies_to: command
    command: []
  - applies_to: command
    command:
    - mcp
    - serve
  notes: Required by several stream-json extensions; no short alias, since -v is --version; also accepted at `mcp serve`.
  scope:
  - print_mode
  - diagnostics
  value_type: none
- aliases:
  - -v
  attachment: []
  description: Prints the version number.
  evidence_ids:
  - help-root-287
  - local-binary-287
  example: claude --version
  flag: --version
  invocation_scope:
  - applies_to: command
    command: []
  notes: 'Output form: 2.1.287 (Claude Code).'
  scope:
  - diagnostics
  value_type: none
- aliases:
  - -w
  attachment:
  - space
  - equals
  - short_attached
  description: Starts Claude in an isolated git worktree.
  evidence_ids:
  - help-root-287
  - docs-cli-reference
  - test-parser-forms-287
  example: claude -w feature-auth
  flag: --worktree
  invocation_scope:
  - applies_to: command
    command: []
  notes: 'Without a name one is generated; #<number> or a PR or MR URL branches the worktree from it; the short-attached form rests on the parser''s uniform short-flag behavior.'
  scope:
  - worktree
  value: '[name]'
  value_optional: true
  value_type: string
- attachment: []
  default: 'false'
  description: Applies the command to all items instead of one.
  evidence_ids:
  - help-subcommands-287
  - help-root-287
  - docs-cli-reference
  example: claude agents --json --all
  flag: --all
  invocation_scope:
  - applies_to: command
    command:
    - agents
  - applies_to: command
    command:
    - respawn
  - applies_to: command
    command:
    - project
    - purge
  notes: At `agents`, includes completed background sessions in --json output; at `respawn`, restarts every running session; at `project purge`, purges every project and is mutually exclusive with a path.
  scope:
  - background_agents
  - sessions
  value_type: none
- aliases:
  - -a
  attachment: []
  default: 'false'
  description: Disables all enabled plugins.
  evidence_ids:
  - help-subcommands-287
  example: claude plugin disable --all
  flag: --all
  invocation_scope:
  - applies_to: command
    command:
    - plugin
    - disable
  notes: The -a short alias is specific to this path.
  scope:
  - plugins
  value_type: none
- attachment:
  - space
  - equals
  description: Filters the agent view or listing to sessions started under a path.
  evidence_ids:
  - help-subcommands-287
  example: claude agents --json --cwd .
  flag: --cwd
  invocation_scope:
  - applies_to: command
    command:
    - agents
  notes: Scoped to `agents`.
  scope:
  - agents
  value: <path>
  value_optional: false
  value_type: string
- attachment: []
  description: Requests JSON output for commands that support it.
  evidence_ids:
  - help-subcommands-287
  - probe-auth-status-287
  - probe-agents-json-287
  - probe-plugin-list-287
  example: claude plugin list --json
  flag: --json
  invocation_scope:
  - applies_to: command
    command:
    - auth
    - status
  - applies_to: command
    command:
    - agents
  - applies_to: command
    command:
    - plugin
    - list
  - applies_to: command
    command:
    - plugin
    - configure
  - applies_to: command
    command:
    - plugin
    - uninstall
  - applies_to: command
    command:
    - plugin
    - enable
  - applies_to: command
    command:
    - plugin
    - disable
  - applies_to: command
    command:
    - plugin
    - validate
  - applies_to: command
    command:
    - plugin
    - marketplace
    - add
  - applies_to: command
    command:
    - plugin
    - marketplace
    - list
  - applies_to: command
    command:
    - plugin
    - marketplace
    - remove
  - applies_to: command
    command:
    - plugin
    - marketplace
    - update
  - applies_to: command
    command:
    - ultrareview
  notes: At `auth status` JSON is the default and --text opts out; mutating plugin paths print one machine-readable result line with the same exit codes; plugin install and plugin update also accept --json per their help.
  scope:
  - output
  value_type: none
- attachment: []
  default: 'false'
  description: Prints human-readable authentication status instead of JSON.
  evidence_ids:
  - help-subcommands-287
  example: claude auth status --text
  flag: --text
  invocation_scope:
  - applies_to: command
    command:
    - auth
    - status
  notes: Scoped to `auth status`.
  scope:
  - output
  value_type: none
- attachment:
  - space
  - equals
  description: Filters printed auto mode rules by label prefix, case-insensitively.
  evidence_ids:
  - help-subcommands-287
  - docs-cli-reference
  example: claude auto-mode defaults --label 'Git Destructive'
  flag: --label
  invocation_scope:
  - applies_to: command
    command:
    - auto-mode
    - defaults
  notes: Scoped to `auto-mode defaults`.
  scope:
  - permissions
  value: <prefix>
  value_optional: false
  value_type: string
- aliases:
  - -y
  attachment: []
  default: 'false'
  description: Skips the command's confirmation prompt.
  evidence_ids:
  - help-subcommands-287
  - docs-cli-reference
  example: claude auto-mode reset --yes
  flag: --yes
  invocation_scope:
  - applies_to: command
    command:
    - auto-mode
    - reset
  - applies_to: command
    command:
    - project
    - purge
  - applies_to: command
    command:
    - plugin
    - uninstall
  - applies_to: command
    command:
    - plugin
    - prune
  notes: Required for non-interactive runs of these paths when a confirmation would otherwise appear; plugin install and plugin update take the same flag plus --accept-command but prompt by design; `import` documents a --yes=<digest> value form.
  scope:
  - prompts
  value_type: none
- attachment:
  - space
  - equals
  default: ~/.claude/daemon.json
  description: Points the background-session supervisor at a JSON config file.
  evidence_ids:
  - help-subcommands-287
  example: claude daemon run --json-path ./daemon.json
  flag: --json-path
  invocation_scope:
  - applies_to: command
    command:
    - daemon
  notes: Umbrella option of the hidden `daemon` command; `daemon run` also takes the same value as a positional.
  scope:
  - background_agents
  value: <p>
  value_optional: false
  value_type: string
- attachment:
  - space
  - equals
  default: ~/.claude/daemon.log
  description: Points the background-session supervisor at a log file.
  evidence_ids:
  - help-subcommands-287
  example: claude daemon run --log-file ./daemon.log
  flag: --log-file
  invocation_scope:
  - applies_to: command
    command:
    - daemon
  notes: Umbrella option of the hidden `daemon` command.
  scope:
  - background_agents
  - diagnostics
  value: <p>
  value_optional: false
  value_type: string
- attachment: []
  default: 'false'
  description: Also stops a transient, non-service daemon.
  evidence_ids:
  - help-subcommands-287
  - docs-cli-reference
  example: claude daemon stop --any
  flag: --any
  invocation_scope:
  - applies_to: command
    command:
    - daemon
    - stop
  notes: Scoped to `daemon stop`.
  scope:
  - background_agents
  value_type: none
- attachment: []
  default: 'false'
  description: Leaves detached background sessions running when the supervisor stops.
  evidence_ids:
  - help-subcommands-287
  - docs-cli-reference
  example: claude daemon stop --keep-workers
  flag: --keep-workers
  invocation_scope:
  - applies_to: command
    command:
    - daemon
    - stop
  notes: Scoped to `daemon stop`.
  scope:
  - background_agents
  value_type: none
- attachment:
  - space
  - equals
  description: Points the gateway at its YAML configuration.
  evidence_ids:
  - help-subcommands-287
  - docs-cli-reference
  example: claude gateway --config gateway.yaml
  flag: --config
  invocation_scope:
  - applies_to: command
    command:
    - gateway
  notes: Required for gateway server mode.
  scope:
  - gateway
  value: <path>
  value_optional: false
  value_type: string
- attachment: []
  default: 'false'
  description: Forces native binary installation even if already installed.
  evidence_ids:
  - help-subcommands-287
  example: claude install --force latest
  flag: --force
  invocation_scope:
  - applies_to: command
    command:
    - install
  notes: Scoped to `install`; no short alias here, unlike plugin init and plugin tag.
  scope:
  - install
  value_type: none
- aliases:
  - -f
  attachment: []
  default: 'false'
  description: Forces the operation past its overwrite or pre-flight checks.
  evidence_ids:
  - help-subcommands-287
  example: claude plugin init -f my-plugin
  flag: --force
  invocation_scope:
  - applies_to: command
    command:
    - plugin
    - init
  - applies_to: command
    command:
    - plugin
    - tag
  notes: At `plugin init`, overwrites an existing .claude-plugin; at `plugin tag`, skips the dirty-tree and existing-tag checks.
  scope:
  - plugins
  value_type: none
- attachment:
  - space
  - equals
  default: git config user.name
  description: Sets the author name written to the scaffolded manifest.
  evidence_ids:
  - help-subcommands-287
  example: claude plugin init --author "Ken Snyder" my-plugin
  flag: --author
  invocation_scope:
  - applies_to: command
    command:
    - plugin
    - init
  notes: Scoped to `plugin init`.
  scope:
  - plugins
  value: <name>
  value_optional: false
  value_type: string
- attachment:
  - space
  - equals
  default: git config user.email
  description: Sets the author email written to the scaffolded manifest.
  evidence_ids:
  - help-subcommands-287
  example: claude plugin init --author-email ken@example.com my-plugin
  flag: --author-email
  invocation_scope:
  - applies_to: command
    command:
    - plugin
    - init
  notes: Scoped to `plugin init`.
  scope:
  - plugins
  value: <email>
  value_optional: false
  value_type: string
- attachment:
  - space
  - equals
  description: Sets the manifest description of a scaffolded plugin.
  evidence_ids:
  - help-subcommands-287
  example: claude plugin init --description "Code review helpers" my-plugin
  flag: --description
  invocation_scope:
  - applies_to: command
    command:
    - plugin
    - init
  notes: Scoped to `plugin init`.
  scope:
  - plugins
  value: <text>
  value_optional: false
  value_type: string
- attachment:
  - space
  - equals
  description: Also scaffolds the named components.
  evidence_ids:
  - help-subcommands-287
  example: claude plugin init --with skills hooks my-plugin
  flag: --with
  invocation_scope:
  - applies_to: command
    command:
    - plugin
    - init
  notes: 'Component names: skills, agents, hooks, mcp, lsp, output-style, channel.'
  scope:
  - plugins
  value: <components...>
  value_type: variadic
  variadic_min: 1
- attachment: []
  default: 'false'
  description: Includes available marketplace plugins in the listing.
  evidence_ids:
  - help-subcommands-287
  example: claude plugin list --json --available
  flag: --available
  invocation_scope:
  - applies_to: command
    command:
    - plugin
    - list
  notes: Requires --json.
  scope:
  - plugins
  value_type: none
- attachment:
  - space
  - equals
  description: Measures each installed plugin's saved data directory, or only the named plugin's.
  evidence_ids:
  - help-subcommands-287
  example: claude plugin list --json --data-size
  flag: --data-size
  invocation_scope:
  - applies_to: command
    command:
    - plugin
    - list
  notes: Requires --json.
  scope:
  - plugins
  value: '[plugin]'
  value_optional: true
  value_type: string
- attachment: []
  default: 'false'
  description: Reads option values from stdin as a JSON object of single-line strings.
  evidence_ids:
  - help-subcommands-287
  example: claude plugin configure --values-stdin my-plugin < values.json
  flag: --values-stdin
  invocation_scope:
  - applies_to: command
    command:
    - plugin
    - configure
  notes: Options left out keep their values; scoped to `plugin configure`.
  scope:
  - plugins
  value_type: none
- attachment: []
  default: 'false'
  description: Preserves the plugin's persistent data directory on uninstall.
  evidence_ids:
  - help-subcommands-287
  example: claude plugin uninstall --keep-data my-plugin
  flag: --keep-data
  invocation_scope:
  - applies_to: command
    command:
    - plugin
    - uninstall
  notes: Data lives under ~/.claude/plugins/data/{id}/.
  scope:
  - plugins
  value_type: none
- attachment: []
  default: 'false'
  description: Also removes auto-installed dependencies that are no longer needed.
  evidence_ids:
  - help-subcommands-287
  example: claude plugin uninstall --prune my-plugin
  flag: --prune
  invocation_scope:
  - applies_to: command
    command:
    - plugin
    - uninstall
  notes: Requires -y in non-interactive contexts; not valid with --json.
  scope:
  - plugins
  value_type: none
- attachment: []
  default: 'false'
  description: Adds the claude.ai-hosted marketplace of the given name.
  evidence_ids:
  - help-subcommands-287
  example: claude plugin marketplace add --claudeai my-marketplace
  flag: --claudeai
  invocation_scope:
  - applies_to: command
    command:
    - plugin
    - marketplace
    - add
  notes: Scoped to `plugin marketplace add`.
  scope:
  - plugins
  value_type: none
- attachment:
  - space
  - equals
  description: Limits a marketplace checkout to specific directories via git sparse-checkout.
  evidence_ids:
  - help-subcommands-287
  example: claude plugin marketplace add --sparse .claude-plugin plugins owner/repo
  flag: --sparse
  invocation_scope:
  - applies_to: command
    command:
    - plugin
    - marketplace
    - add
  notes: For monorepos; scoped to `plugin marketplace add`.
  scope:
  - plugins
  value: <paths...>
  value_type: variadic
  variadic_min: 1
- attachment: []
  default: 'false'
  description: Treats validation warnings as errors for CI.
  evidence_ids:
  - help-subcommands-287
  example: claude plugin validate --strict ./my-plugin
  flag: --strict
  invocation_scope:
  - applies_to: command
    command:
    - plugin
    - validate
  notes: Exits 1 on unrecognized fields and missing metadata the runtime tolerates.
  scope:
  - plugins
  value_type: none
- attachment:
  - space
  - equals
  default: '45'
  description: Caps how long ultrareview waits for the review to finish.
  evidence_ids:
  - help-subcommands-287
  example: claude ultrareview --timeout 10 --json 1234
  flag: --timeout
  invocation_scope:
  - applies_to: command
    command:
    - ultrareview
  notes: Measured in minutes; scoped to `ultrareview`.
  scope:
  - ultrareview
  value: <minutes>
  value_optional: false
  value_type: number
- attachment: []
  default: 'false'
  description: Posts the finished review's findings to the PR as one plain comment.
  evidence_ids:
  - help-subcommands-287
  - docs-cli-reference
  example: claude ultrareview --post 1234
  flag: --post
  invocation_scope:
  - applies_to: command
    command:
    - ultrareview
  notes: PR targets on github.com only; requires v2.1.227+.
  scope:
  - ultrareview
  value_type: none
- attachment: []
  default: 'true'
  description: Does not post findings to the PR; the default.
  evidence_ids:
  - help-subcommands-287
  - docs-cli-reference
  example: claude ultrareview --no-post 1234
  flag: --no-post
  invocation_scope:
  - applies_to: command
    command:
    - ultrareview
  notes: Accepted for parity with the /ultrareview and /code-review ultra flags; requires v2.1.227+.
  scope:
  - ultrareview
  value_type: none
- attachment: []
  default: 'false'
  description: Previews what the command would change without changing anything.
  evidence_ids:
  - help-subcommands-287
  - docs-cli-reference
  example: claude project purge --dry-run ~/work/repo
  flag: --dry-run
  invocation_scope:
  - applies_to: command
    command:
    - project
    - purge
  - applies_to: command
    command:
    - plugin
    - tag
  - applies_to: command
    command:
    - plugin
    - prune
  - applies_to: command
    command:
    - import
  notes: At `project purge`, lists what would be deleted; at `plugin tag`, prints what would be tagged; at `plugin prune`, lists what would be removed; at `import`, shows what would be imported.
  scope:
  - prompts
  value_type: none
- aliases:
  - -i
  attachment: []
  default: 'false'
  description: Prompts for each item before deleting during a project purge.
  evidence_ids:
  - help-subcommands-287
  - docs-cli-reference
  example: claude project purge -i ~/work/repo
  flag: --interactive
  invocation_scope:
  - applies_to: command
    command:
    - project
    - purge
  notes: Scoped to `project purge`.
  scope:
  - prompts
  value_type: none
- attachment:
  - space
  - equals
  description: Discards a worktree's unpushed commits while removing a session.
  evidence_ids:
  - help-subcommands-287
  - docs-cli-reference
  example: claude rm 7c5dcf5d --discard-unpushed abc123@wt-9
  flag: --discard-unpushed
  invocation_scope:
  - applies_to: command
    command:
    - rm
  notes: Pass the value a previous refused `claude rm` reported; requires v2.1.260+.
  scope:
  - background_agents
  value: <commit>@<worktree-id>
  value_optional: false
  value_type: string
- attachment:
  - space
  - equals
  description: Deletes a worktree directory that git or the WorktreeRemove hook could not remove.
  evidence_ids:
  - help-subcommands-287
  - docs-cli-reference
  example: claude rm 7c5dcf5d --force-remove-worktree wt-9
  flag: --force-remove-worktree
  invocation_scope:
  - applies_to: command
    command:
    - rm
  notes: Pass the value a previous refused `claude rm` reported; the branch is kept; requires v2.1.268+.
  scope:
  - background_agents
  value: <worktree-id>
  value_optional: false
  value_type: string
- aliases:
  - -e
  attachment:
  - space
  - equals
  - short_attached
  description: Sets environment variables for a stdio MCP server being added.
  evidence_ids:
  - help-subcommands-287
  - test-parser-forms-287
  example: claude mcp add my-server -e API_KEY=xxx -- npx my-mcp-server
  flag: --env
  invocation_scope:
  - applies_to: command
    command:
    - mcp
    - add
  notes: KEY=value entries; scoped to `mcp add`.
  scope:
  - mcp
  value: <env...>
  value_type: variadic
  variadic_min: 1
- aliases:
  - -H
  attachment:
  - space
  - equals
  - short_attached
  description: Sets headers for an HTTP or SSE MCP server being added.
  evidence_ids:
  - help-subcommands-287
  - test-parser-forms-287
  example: 'claude mcp add --transport http corridor https://app.corridor.dev/api/mcp --header "Authorization: Bearer ..."'
  flag: --header
  invocation_scope:
  - applies_to: command
    command:
    - mcp
    - add
  notes: Repeatable and space-separated; scoped to `mcp add`.
  scope:
  - mcp
  value: <header...>
  value_type: variadic
  variadic_min: 1
- aliases:
  - -s
  attachment:
  - space
  - equals
  - short_attached
  default: local
  description: Chooses which configuration scope an MCP server is added to.
  evidence_ids:
  - help-subcommands-287
  - test-parser-forms-287
  example: claude mcp add --scope user my-server -- npx my-mcp-server
  flag: --scope
  invocation_scope:
  - applies_to: command
    command:
    - mcp
    - add
  - applies_to: command
    command:
    - mcp
    - add-json
  - applies_to: command
    command:
    - mcp
    - add-from-claude-desktop
  notes: Values local, user, project.
  scope:
  - mcp
  value: <scope>
  value_optional: false
  value_type: string
- aliases:
  - -s
  attachment:
  - space
  - equals
  - short_attached
  description: Removes an MCP server from a specific configuration scope.
  evidence_ids:
  - help-subcommands-287
  example: claude mcp remove --scope user my-server
  flag: --scope
  invocation_scope:
  - applies_to: command
    command:
    - mcp
    - remove
  notes: Without the flag, removes from whichever scope the server exists in.
  scope:
  - mcp
  value: <scope>
  value_optional: false
  value_type: string
- aliases:
  - -s
  attachment:
  - space
  - equals
  - short_attached
  default: auto-detect
  description: Chooses the installation scope a plugin is enabled or disabled in.
  evidence_ids:
  - help-subcommands-287
  example: claude plugin enable --scope user my-plugin
  flag: --scope
  invocation_scope:
  - applies_to: command
    command:
    - plugin
    - enable
  - applies_to: command
    command:
    - plugin
    - disable
  notes: Values user, project, local; auto-detects when omitted.
  scope:
  - plugins
  value: <scope>
  value_optional: false
  value_type: string
- aliases:
  - -s
  attachment:
  - space
  - equals
  - short_attached
  default: user
  description: Chooses the installation scope a plugin is uninstalled from.
  evidence_ids:
  - help-subcommands-287
  example: claude plugin uninstall --scope user my-plugin
  flag: --scope
  invocation_scope:
  - applies_to: command
    command:
    - plugin
    - uninstall
  notes: plugin install and plugin update take the same flag with their own defaults.
  scope:
  - plugins
  value: <scope>
  value_optional: false
  value_type: string
- aliases:
  - -t
  attachment:
  - space
  - equals
  - short_attached
  default: stdio
  description: Selects the transport type of an MCP server being added.
  evidence_ids:
  - help-subcommands-287
  - test-mcp-add-287
  example: claude mcp add --transport http sentry https://mcp.sentry.dev/mcp
  flag: --transport
  invocation_scope:
  - applies_to: command
    command:
    - mcp
    - add
  notes: Validated app-side as stdio, sse, http, or the help-undocumented streamable-http; all three attachment forms proven locally.
  scope:
  - mcp
  value: <transport>
  value_optional: false
  value_type: string
- attachment:
  - space
  - equals
  description: Fixes the OAuth callback port for servers requiring pre-registered redirect URIs.
  evidence_ids:
  - help-subcommands-287
  example: claude mcp add --transport http --callback-port 9090 my-server https://example.com/mcp
  flag: --callback-port
  invocation_scope:
  - applies_to: command
    command:
    - mcp
    - add
  notes: Scoped to `mcp add`.
  scope:
  - mcp
  value: <port>
  value_optional: false
  value_type: string
- attachment:
  - space
  - equals
  description: Sets the OAuth client ID for an HTTP or SSE MCP server being added.
  evidence_ids:
  - help-subcommands-287
  example: claude mcp add --transport http --client-id abc my-server https://example.com/mcp
  flag: --client-id
  invocation_scope:
  - applies_to: command
    command:
    - mcp
    - add
  notes: Scoped to `mcp add`.
  scope:
  - mcp
  value: <clientId>
  value_optional: false
  value_type: string
- attachment: []
  default: 'false'
  description: Prompts for the OAuth client secret instead of taking it on the command line.
  evidence_ids:
  - help-subcommands-287
  example: claude mcp add --transport http --client-secret my-server https://example.com/mcp
  flag: --client-secret
  invocation_scope:
  - applies_to: command
    command:
    - mcp
    - add
  - applies_to: command
    command:
    - mcp
    - add-json
  notes: Prompts, so it is interactive; the MCP_CLIENT_SECRET variable is the non-interactive alternative.
  scope:
  - mcp
  value_type: none
- attachment: []
  default: 'false'
  description: Prints the MCP OAuth authorization URL instead of opening a browser.
  evidence_ids:
  - help-subcommands-287
  - docs-cli-reference
  example: claude mcp login --no-browser sentry
  flag: --no-browser
  invocation_scope:
  - applies_to: command
    command:
    - mcp
    - login
  notes: For SSH and headless sessions; the redirect URL is pasted back at a prompt, so the flow still needs a person.
  scope:
  - mcp
  value_type: none
- aliases:
  - -d
  attachment: []
  default: 'false'
  description: Enables debug mode for the Claude Code MCP server.
  evidence_ids:
  - help-subcommands-287
  example: claude mcp serve --debug
  flag: --debug
  invocation_scope:
  - applies_to: command
    command:
    - mcp
    - serve
  notes: Boolean at this path, unlike the root --debug whose optional filter binds only in the equals form.
  scope:
  - diagnostics
  - mcp
  value_type: none
config_paths:
- format: json
  notes: User settings; local file exists and holds hooks, permissions, model, statusLine, enabledPlugins, extraKnownMarketplaces, effortLevel, and tui keys. Written by Claude Code when a setting is saved.
  os: macos
  path: ~/.claude/settings.json
  scope: user
- format: json
  notes: User settings under the Linux home directory; same shape as macOS.
  os: linux
  path: ~/.claude/settings.json
  scope: user
- format: json
  notes: Windows expansion of the user settings path.
  os: windows
  path: '%USERPROFILE%\.claude\settings.json'
  scope: user
- format: json
  notes: Mutable user state and cache; local file exists and holds installMethod, projects, oauthAccount, caches, and onboarding flags. Claude Code rewrites this file frequently.
  os: macos
  path: ~/.claude.json
  scope: user
- format: json
  notes: Mutable user state and cache next to the home config directory.
  os: linux
  path: ~/.claude.json
  scope: user
- format: json
  notes: Windows mutable user state and cache path.
  os: windows
  path: '%USERPROFILE%\.claude.json'
  scope: user
- format: json
  notes: Project settings checked into source control when present.
  os: macos
  path: .claude/settings.json
  scope: repo
- format: json
  notes: Project settings checked into source control when present.
  os: linux
  path: .claude/settings.json
  scope: repo
- format: json
  notes: Project settings checked into source control when present.
  os: windows
  path: .claude\settings.json
  scope: repo
- format: json
  notes: Local project settings; gitignored when Claude Code saves a setting to it.
  os: macos
  path: .claude/settings.local.json
  scope: repo
- format: json
  notes: Local project settings; gitignored when Claude Code saves a setting to it.
  os: linux
  path: .claude/settings.local.json
  scope: repo
- format: json
  notes: Local project settings; gitignored when Claude Code saves a setting to it.
  os: windows
  path: .claude\settings.local.json
  scope: repo
- format: json
  notes: Project-scoped MCP servers; unapproved servers are shown as pending.
  os: macos
  path: .mcp.json
  scope: repo
- format: json
  notes: Project-scoped MCP servers; unapproved servers are shown as pending.
  os: linux
  path: .mcp.json
  scope: repo
- format: json
  notes: Project-scoped MCP servers; unapproved servers are shown as pending.
  os: windows
  path: .mcp.json
  scope: repo
- format: json
  notes: Enterprise managed settings; a managed-settings.d/ drop-in directory beside it merges in alphabetical order; also deliverable as a com.anthropic.claudecode MDM profile.
  os: macos
  path: /Library/Application Support/ClaudeCode/managed-settings.json
  scope: system
- format: json
  notes: Enterprise managed settings for Linux and WSL; managed-settings.d/ drop-ins merge in alphabetical order.
  os: linux
  path: /etc/claude-code/managed-settings.json
  scope: system
- format: json
  notes: Enterprise managed settings; the legacy C:\ProgramData\ClaudeCode path is not read; an HKLM\SOFTWARE\Policies\ClaudeCode Settings value is the registry delivery.
  os: windows
  path: C:\Program Files\ClaudeCode\managed-settings.json
  scope: system
- format: other
  notes: Relocates the ~/.claude tree; all settings, session history, and plugins move under it; ignored in project and local settings env blocks.
  os: macos
  path: CLAUDE_CONFIG_DIR
  scope: env
- format: other
  notes: Relocates the ~/.claude tree; ignored in project and local settings env blocks.
  os: linux
  path: CLAUDE_CONFIG_DIR
  scope: env
- format: other
  notes: Relocates the %USERPROFILE%\.claude tree; ignored in project and local settings env blocks.
  os: windows
  path: CLAUDE_CONFIG_DIR
  scope: env
env_vars:
- effect: Overrides the configuration directory (default ~/.claude); settings, session history, and plugins all live under it.
  name: CLAUDE_CONFIG_DIR
- effect: Set to 1 to start in safe mode with customizations disabled for troubleshooting; the --safe-mode flag sets it.
  name: CLAUDE_CODE_SAFE_MODE
- effect: Set to 1 for minimal mode with a minimal system prompt and only Bash, file read, and file edit tools; the --bare flag sets it.
  name: CLAUDE_CODE_SIMPLE
- effect: Set to 1 to use a shorter system prompt and abbreviated tool descriptions; set to a false spelling to opt out.
  name: CLAUDE_CODE_SIMPLE_SYSTEM_PROMPT
- effect: Set to 1 to skip writing prompt history and session transcripts; sessions do not appear in resume, continue, or up-arrow history.
  name: CLAUDE_CODE_SKIP_PROMPT_HISTORY
- effect: Overrides the temp directory for internal temp files; /claude-{uid} is appended on Unix.
  name: CLAUDE_CODE_TMPDIR
- effect: Any non-empty value disables nonessential network traffic (auto-updates, telemetry, error reporting, release notes, availability checks); 0 and false still disable it.
  name: CLAUDE_CODE_DISABLE_NONESSENTIAL_TRAFFIC
- effect: Set to 1 to disable automatic background updates; manual claude update still works.
  name: DISABLE_AUTOUPDATER
- effect: Blocks all update paths including manual claude update and claude install, for distributing Claude Code through your own channels.
  name: DISABLE_UPDATES
- effect: Set to 1 to disable cost warning messages.
  name: DISABLE_COST_WARNINGS
- effect: Any non-empty value opts out of error reporting; 0 and false still opt out.
  name: DISABLE_ERROR_REPORTING
- effect: Any non-empty value opts out of telemetry and also disables feature-flag fetching.
  name: DISABLE_TELEMETRY
- effect: Same effect as DISABLE_TELEMETRY, including on feature-flag fetching.
  name: DO_NOT_TRACK
- effect: Set to 1 to disable the /feedback command; the older name DISABLE_BUG_COMMAND is also accepted.
  name: DISABLE_FEEDBACK_COMMAND
- effect: Set to 1 to let Claude Code run the package manager's upgrade command in the background for Homebrew and WinGet installs.
  name: CLAUDE_CODE_PACKAGE_MANAGER_AUTO_UPDATE
- effect: Set to 1 to disable automatic terminal title updates and skip the title-generating model request.
  name: CLAUDE_CODE_DISABLE_TERMINAL_TITLE
- effect: Set to 1 to disable fullscreen rendering and use the classic main-screen renderer.
  name: CLAUDE_CODE_DISABLE_ALTERNATE_SCREEN
- effect: 'Command prefix that wraps shell commands Claude Code spawns: Bash tool calls, hook commands, status line commands, and stdio MCP server startup.'
  name: CLAUDE_CODE_SHELL_PREFIX
- effect: 'Controls the PowerShell tool: on Windows without Git Bash it is on by default, on other platforms set to 1 to enable it, which requires pwsh on PATH.'
  name: CLAUDE_CODE_USE_POWERSHELL_TOOL
- effect: 'Windows only: path to Git Bash''s bash.exe used when auto-discovery fails; an invalid path is ignored.'
  name: CLAUDE_CODE_GIT_BASH_PATH
- effect: Set to 1 in print mode to wait for plugin installation to finish before the first query.
  name: CLAUDE_CODE_SYNC_PLUGIN_INSTALL
- effect: Set to 1 in print mode to download claude.ai-enabled skills and wait for the list before the first query.
  name: CLAUDE_CODE_SYNC_SKILLS
- effect: Set to 1 to strip credentials from subprocess environments (Bash tool, hooks, stdio MCP servers).
  name: CLAUDE_CODE_SUBPROCESS_ENV_SCRUB
- effect: Set to false to disable syntax highlighting in diff output.
  name: CLAUDE_CODE_SYNTAX_HIGHLIGHT
- effect: Any non-empty value allows 24-bit truecolor inside tmux; 0 and false still allow it.
  name: CLAUDE_CODE_TMUX_TRUECOLOR
- effect: Set to 1 to clone GitHub shorthand plugin sources over HTTPS instead of SSH.
  name: CLAUDE_CODE_PLUGIN_PREFER_HTTPS
- effect: Prefix for auto-generated Remote Control session names; defaults to the hostname.
  name: CLAUDE_REMOTE_CONTROL_SESSION_NAME_PREFIX
- effect: Timeout for API requests in milliseconds (default 600000, maximum 2147483647).
  name: API_TIMEOUT_MS
- effect: Default timeout for a foreground Bash or PowerShell tool command in milliseconds (default 120000).
  name: BASH_DEFAULT_TIMEOUT_MS
- effect: Set to 0 to use system-installed rg instead of the bundled ripgrep.
  name: USE_BUILTIN_RIPGREP
- effect: Set to 1 to emit subagent text and thinking blocks in print-mode stream-json output, matching --forward-subagent-text.
  name: CLAUDE_CODE_FORWARD_SUBAGENT_TEXT
- effect: Overrides the debug log file path (a file path despite the name); debug mode must be enabled separately.
  name: CLAUDE_CODE_DEBUG_LOGS_DIR
- effect: 'Set to 1 to turn off background agents and agent view: claude agents, --bg, /background, and the on-demand supervisor.'
  name: CLAUDE_CODE_DISABLE_AGENT_VIEW
- effect: 'Overrides automatic IDE connection: false prevents it, true forces an attempt when auto-detection fails.'
  name: CLAUDE_CODE_AUTO_CONNECT_IDE
- effect: Set to 1 to prevent loading any CLAUDE.md memory files into context.
  name: CLAUDE_CODE_DISABLE_CLAUDE_MDS
- effect: Set to 1 to disable auto memory; 0 forces it on even in --bare mode.
  name: CLAUDE_CODE_DISABLE_AUTO_MEMORY
- effect: Set to 1 to disable attachment processing; @ file mentions are sent as plain text.
  name: CLAUDE_CODE_DISABLE_ATTACHMENTS
- effect: Set to 1 in subprocesses Claude Code spawns (Bash and PowerShell tools, tmux sessions, hook and status line commands, stdio MCP servers); detect it to know a script runs inside Claude Code.
  name: CLAUDECODE
- effect: Set to 1 in direct tool, hook, and status line subprocesses but not stdio MCP servers; distinguishes a nested session from a top-level claude in an IDE terminal.
  name: CLAUDE_CODE_CHILD_SESSION
machine_introspection:
- command: claude --version
  machine_readable: false
  notes: Prints `2.1.287 (Claude Code)`; one line, trivially parseable, but not a structured format.
  output_format: text
  purpose: version
  useful_for_codegen: false
- command: claude auth status
  machine_readable: true
  notes: JSON with loggedIn, authMethod, apiProvider, configDirectory, email, orgId, subscriptionType; exits 0 logged in, 1 logged out; --text switches to human-readable.
  output_format: json
  purpose: env
  useful_for_codegen: false
- command: claude agents --json
  machine_readable: true
  notes: JSON array of interactive and background sessions (id, cwd, kind, startedAt, sessionId, name, status or state); does not require a TTY; --all adds completed background sessions.
  output_format: json
  purpose: other
  useful_for_codegen: false
- command: claude auto-mode defaults
  machine_readable: true
  notes: Built-in auto mode environment, allow, soft_deny, and hard_deny rules as JSON; --label filters by prefix.
  output_format: json
  purpose: config_schema
  useful_for_codegen: true
- command: claude auto-mode config
  machine_readable: true
  notes: Effective auto mode configuration after settings are applied.
  output_format: json
  purpose: config_dump
  useful_for_codegen: true
- command: claude plugin list --json
  machine_readable: true
  notes: JSON array of installed plugins with id, version, scope, enabled, installPath; --available adds marketplace plugins; --data-size measures saved data.
  output_format: json
  purpose: plugins
  useful_for_codegen: false
- command: claude daemon status
  machine_readable: false
  notes: Supervisor pid, version, socket directory, worker roster, and log state; exits 1 when not running, which is a normal state.
  output_format: text
  purpose: doctor
  useful_for_codegen: false
- command: claude doctor
  machine_readable: false
  notes: Read-only install and settings diagnostics including settings-file validation errors; ran to completion with exit 0 without a TTY in 2.1.287.
  output_format: text
  purpose: doctor
  useful_for_codegen: false
- command: claude mcp list
  machine_readable: false
  notes: Human-readable server list with live health checks over the network; MCP details belong to the MCP topic.
  output_format: text
  purpose: mcp
  useful_for_codegen: false
wrapper_notes:
- Use `claude -p` for non-interactive wrapper runs; plain `claude "query"` starts an interactive session seeded with the prompt. Piped stdout alone also selects the non-interactive input requirement even without -p, so a wrapper that captures stdout changes the CLI's behavior.
- 'The parser is strict: an unrecognized long or short option exits 1 with `error: unknown option`, so Claudine must not forward switches it cannot classify, and a user argument that looks like an unknown switch genuinely fails rather than passing through.'
- 'Documentation can drift from the binary: `--exec` is documented in the CLI reference but rejected as unknown by the 2.1.287 parser, and `claude import` documents an interactive flow but prints `not yet available in this build`. Verify flags against the running binary, not the docs alone.'
- A variadic root switch (--add-dir, --allowedTools, --disallowedTools, --betas, --file, --mcp-config, --tools, --channels, --dangerously-load-development-channels) grabs the immediately following token unconditionally, even when it looks like an option, and keeps consuming non-option tokens. Never place another flag directly after a variadic switch in a mixed command line.
- An optional-value root switch (--resume, --from-pr, --worktree, --teleport, --cloud, --remote-control, --prompt-suggestions) does not consume a following token that looks like an option, but does consume a plain one, which would steal a user's positional prompt.
- The --debug filter binds only in the equals form; `--debug api` enables debug mode without filtering, so wrappers must emit --debug=api.
- 'Invalid values fail at three different layers: commander choice validation exits at parse time (--output-format, --permission-mode, --input-format, --system-prompt-snapshot, --permission-prompts, --autocompact, --prompt-suggestions); app-level validation exits after parsing (--setting-sources, --session-id UUID, mcp add transport); and --effort only warns and falls back to the default.'
- '`claude auth status` exits 1 when logged out and `claude daemon status` exits 1 when no supervisor runs; both are normal states, so parse stdout before classifying the exit as a crash.'
- '`claude mcp list` performs live network health checks; expect slow or offline-sensitive runs.'
- '`claude doctor` now completes without a TTY; the 2.1.200-era 20-second hang did not reproduce in 2.1.287, but a timeout guard is still cheap insurance.'
- Top-level help omits hidden commands (daemon, remote-control, self-hosted-runner) and hidden flags (--sdk-url, and docs-only flags like --max-turns, --init, --init-only, --maintenance, --advisor, --teammate-mode); help incompleteness is documented policy, so absence from help is not evidence a flag is unavailable.
- --bg cannot be combined with -p and prints a session id that `claude attach`, `logs`, `stop`, and `rm` take; a leading --dangerously-skip-permissions or --allow-dangerously-skip-permissions routes `daemon <subcommand>` to the daemon command since v2.1.199.
- Print mode skips the workspace trust dialog and silently ignores settings files that fail validation, so wrappers should only launch it in trusted directories and cannot rely on settings errors surfacing.
- Native installs auto-update in the background (channel latest by default), so the binary a wrapper resolved can change between runs; the launcher is a symlink into ~/.local/share/claude/versions/.
- CLAUDE_CONFIG_DIR is the broadest public isolation knob, moving settings, session history, and plugins together; --bare is stronger but restricts auth to ANTHROPIC_API_KEY or an apiKeyHelper, never reading OAuth or keychain credentials.
- On Windows without Git for Windows the shell tool is PowerShell; with it, Bash via Git Bash; CLAUDE_CODE_GIT_BASH_PATH points at bash.exe when discovery fails.
- --settings values override settings files for the session but managed policy still wins; --restricted ignores user, project, and local settings entirely.
- System-prompt flags (--system-prompt, --system-prompt-file, --append-system-prompt, --append-system-prompt-file, --system-prompt-snapshot, --exclude-dynamic-system-prompt-sections, and the subagent variants) work in both interactive and non-interactive modes; this topic records their spellings and value shapes only, with semantics in the system-prompt topic.
changes:
- 'Rewrote the document to research contract revision 2: every switch record now carries value type, optional-value and variadic-minimum facts, attachment forms, invocation scope, and evidence citations.'
- Updated the verified version from 2.1.200 to 2.1.287 via local `claude --version`, `claude doctor`, and npm dist-tags (latest 2.1.287, stable 2.1.285).
- 'Established the parser as commander.js and its attachment behavior from disposable tests: space and equals bind single and optional values, short-attached binds for one-dash spellings, equals binds variadic values, and optional-value switches do not grab option-looking tokens.'
- 'Recorded the variadic hazard: a variadic switch consumes the immediately following token unconditionally, even an option-looking one.'
- Recorded that unknown options are rejected with a non-zero exit, and the docs-versus-binary drift for --exec (documented, rejected in 2.1.287) and `claude import` (documented, disabled in this build).
- 'Added newly verified root switches: --cloud (with deprecated --remote alias), --desktop, --environment, --ref, --restricted, --permission-prompts, --forward-subagent-text, --autocompact, --system-prompt-snapshot, --append-subagent-system-prompt[-file], --sdk-url (hidden), and the variadic --channels and --dangerously-load-development-channels.'
- Dropped --exec, --teammate-mode help presence, --remote and --teleport help absence notes, and other 2.1.200-era observations that no longer hold; --teammate-mode and --teleport are now parser-verified with current placeholder spellings.
- Expanded the subcommand list with import, the full mcp and plugin leaf paths, plugin marketplace, and the hidden remote-control and self-hosted-runner commands; marked doctor as completing without a TTY in 2.1.287.
- Refreshed configuration discovery with exact managed-settings paths per OS, the managed-settings.d drop-in directory, and the registry and MDM delivery channels.
- 'Refreshed the environment variable list from the current env-vars page: added DISABLE_UPDATES, DISABLE_FEEDBACK_COMMAND, DO_NOT_TRACK, CLAUDE_CODE_FORWARD_SUBAGENT_TEXT, CLAUDE_CODE_DISABLE_AGENT_VIEW, CLAUDECODE, and CLAUDE_CODE_CHILD_SESSION; removed the removed FORCE_AUTOUPDATER and the no-longer-documented DISABLE_NON_ESSENTIAL_MODEL_CALLS; renamed the alternate-screen variable to CLAUDE_CODE_DISABLE_ALTERNATE_SCREEN.'
- 'Refreshed machine introspection: auth status JSON now includes configDirectory; agents --json reports both interactive and background sessions with status or state; doctor behavior re-verified.'
requires_claudine_update: true
reason: 'Claudine''s Claude wrapper and provider metadata should consume the revision-2 switch inventory (ProviderInfo::cli_switches can now be Researched instead of Unknown), and the parser findings change partitioning behavior: variadic root switches must never be followed directly by another flag, optional-value switches steal plain positionals, unknown options hard-fail, --debug filters bind only via =, and several new or renamed switches (--permission-prompts, --cloud/--remote, --restricted, --system-prompt-snapshot) plus the --exec removal affect how a mixed command line should be forwarded.'
contract_checked: 2026-10-01
---

# Claude Code CLI Surface

## Overview

Claude Code is Anthropic's official agentic coding CLI. The public terminal command is `claude`: running `claude` starts an interactive session, `claude "query"` starts an interactive session with an initial prompt, and `claude -p "query"` runs print/SDK mode, answers, and exits. A wrapper that wants one-shot execution always names `-p`/`--print`; nothing else is safe to assume.

The verified version for this research is `2.1.287`. It was verified on 2026-10-01 three ways: local `claude --version` printed `2.1.287 (Claude Code)`; `claude doctor` reported `Running: native (2.1.287)` from `/Users/ken/.local/share/claude/versions/2.1.287`; and `npm view @anthropic-ai/claude-code version dist-tags --json` reported `latest: 2.1.287` and `stable: 2.1.285`. The CLI is closed source; there is no public source repository, so the argument parser was characterized from its error output and disposable runs rather than from declarations in source.

The binary embeds a **commander.js** argument parser. Its error format (`error: option '--output-format <format>' argument 'bogus' is invalid. Allowed choices are text, json, stream-json.`), its `argument missing` errors, and its help rendering (`-h, --help  Display help for command`, `(default: ...)`) are all commander's. The placeholder grammar in help is therefore the parser's own declaration: `<x>` takes one required value, `[x]` takes one optional value, and `<x...>` takes a variadic list.

Primary links:

| Resource | URL |
| --- | --- |
| Homepage | [https://claude.ai/code](https://claude.ai/code) |
| Repository | none; the CLI is closed source |
| General docs | [https://code.claude.com/docs/en/overview](https://code.claude.com/docs/en/overview) |
| CLI reference | [https://code.claude.com/docs/en/cli-reference](https://code.claude.com/docs/en/cli-reference) |

## Installation and Binaries

The installed macOS command is `/Users/ken/.local/bin/claude`, a native-installer symlink into `/Users/ken/.local/share/claude/versions/`; `claude doctor` identifies it as a native darwin-arm64 build on the `latest` auto-update channel. Native and npm installs both ship a per-platform native binary — the npm package pulls it through an optional dependency such as `@anthropic-ai/claude-code-darwin-arm64`, and the installed `claude` does not invoke Node at runtime.

| OS | Binary | Alternate shims | Install methods |
| --- | --- | --- | --- |
| macOS | `claude` | none observed | Native installer, Homebrew cask, npm |
| Linux | `claude` | none documented | Native installer, apt, dnf, apk, npm |
| Windows | `claude.exe` | `claude`, npm shims such as `claude.cmd` | Native PowerShell/CMD installer, WinGet, npm |

Official install commands:

```sh
curl -fsSL https://claude.ai/install.sh | bash            # macOS, Linux, WSL
irm https://claude.ai/install.ps1 | iex                    # Windows PowerShell
brew install --cask claude-code                            # stable channel
brew install --cask claude-code@latest                     # latest channel
winget install Anthropic.ClaudeCode
sudo apt install claude-code
sudo dnf install claude-code
apk add claude-code
npm install -g @anthropic-ai/claude-code
```

The apt, dnf, and apk commands require adding Anthropic's signed package repository first; each offers `stable` and `latest` channels. The native installer accepts a channel or version argument (`curl -fsSL https://claude.ai/install.sh | bash -s stable`). Native installs auto-update in the background; Homebrew, WinGet, apt, dnf, and apk installs do not, though Homebrew and WinGet can opt in with `CLAUDE_CODE_PACKAGE_MANAGER_AUTO_UPDATE=1`. The npm package requires Node.js 22+ for its install-time engine checks (as of v2.1.198) but runs the native binary regardless.

## Subcommands

The frontmatter `subcommands` list records every native command path below the executable with its non-interactive marking. Highlights for a wrapper:

| Command path | Non-interactive | Wrapper use |
| --- | --- | --- |
| `claude -p` (root flag, not a subcommand) | Yes | The one-shot execution path |
| `claude agents --json` | Yes (the `--json` form) | Live session discovery; the bare view needs a TTY |
| `claude auth status` | Yes | Readiness probe; exits 1 when logged out |
| `claude auto-mode defaults` / `config` | Yes | JSON rule catalogs |
| `claude daemon status` | Yes | Supervisor diagnostics; exits 1 when not running |
| `claude doctor` | Yes | Install and settings diagnostics; completes without a TTY in 2.1.287 |
| `claude install`, `update`, `respawn`, `rm`, `stop`, `logs` | Yes | Binary and background-session management |
| `claude mcp add` / `add-json` / `list` / `get` / `remove` / `serve` | Yes | MCP configuration (semantics in the MCP topic) |
| `claude plugin list --json`, `plugin validate --json` | Yes | Plugin inventory and validation |
| `claude ultrareview` | Yes | Cloud review; `--json` for the raw payload |
| `claude import`, `plugin install`, `plugin update`, `project purge`, `auto-mode reset` | No | Prompt or are disabled; see notes for the flags that make them promptless |
| `claude attach`, `auth login`, `mcp login`, `remote-control`, `setup-token` | No | Terminal, browser, or secrets flows |

Three commands are hidden from top-level help but answer `--help`: `daemon`, `remote-control`, and `self-hosted-runner`. Since v2.1.199, a leading `--dangerously-skip-permissions` or `--allow-dangerously-skip-permissions` routes `daemon <subcommand>` to the daemon command; any other leading flag leaves `daemon ...` to start an interactive session with those words as the prompt. The `remote-control` and `self-hosted-runner` flag surfaces are large operator interfaces documented in their own `--help` output and are not inventoried here.

## CLI Switch Inventory

Inventoried paths: the root entrypoint (including its resume forms `-c`/`--continue`, `-r`/`--resume`, `--from-pr`, `--fork-session`, `--session-id`) and every command path marked non-interactive in `subcommands`. The `agents` command's switches are also recorded because its `--json` form is a scriptable path even though the bare view is interactive. Switches of the interactive and operator paths (`auth login`, `plugin install`, `plugin update`, `plugin eval`, `remote-control`, `self-hosted-runner`) are summarized in their `subcommands` notes rather than recorded as switch records.

How the facts were established:

- **Spellings, aliases, placeholders, and defaults** come from local 2.1.287 help at every inventoried path, cross-checked against the [CLI reference](https://code.claude.com/docs/en/cli-reference). Where help and docs disagree, the parser wins: `--exec` is documented but rejected as unknown by the 2.1.287 binary.
- **Value types** come from commander's placeholder grammar (`<x>`, `[x]`, `<x...>`) confirmed by parse errors: a bare required-value switch errors with `option '--flag <placeholder>' argument missing`, and those errors exposed both the exact placeholder and the variadic `<servers...>` shape of `--channels` and `--dangerously-load-development-channels`.
- **Attachment forms** were proven by disposable tests: choice-validated switches reject invalid values naming the switch in both the space and equals forms (`--output-format bogus` / `--output-format=bogus`); short-attached values parse (`-nfoo`, `-rbar`, and `mcp add -tbogus`); an optional-value switch does not consume an option-looking next token (`--from-pr --setting-sources bogus` still parses `--setting-sources`); and equals binds variadic values (`--add-dir=/nonexistent` consumed the path).

Wrapper-critical groups at the root entrypoint:

| Scope | Switches |
| --- | --- |
| Non-interactive execution | `-p`/`--print`, `--input-format`, `--output-format`, `--json-schema`, `--max-turns`, `--max-budget-usd`, `--permission-prompts`, `--no-session-persistence` |
| Structured stream extensions | `--include-hook-events`, `--include-partial-messages`, `--prompt-suggestions`, `--replay-user-messages`, `--forward-subagent-text`, `--verbose` |
| Resume and session identity | `-c`/`--continue`, `-r`/`--resume`, `--from-pr`, `--fork-session`, `--session-id`, `-n`/`--name` |
| Config isolation | `--settings`, `--setting-sources`, `--bare`, `--safe-mode`, `--restricted`, `--mcp-config`, `--strict-mcp-config`, `CLAUDE_CONFIG_DIR` |
| Background sessions | `--bg`/`--background`, `-w`/`--worktree`, `--tmux`, `--agents` |
| Cloud and remote entry points | `--cloud` (alias `--remote`), `--environment`, `--ref`, `--teleport`, `--remote-control` (alias `--rc`), `--remote-control-session-name-prefix` |
| Permissions | `--permission-mode`, `--allowedTools`/`--allowed-tools`, `--disallowedTools`/`--disallowed-tools`, `--tools`, `--dangerously-skip-permissions`, `--allow-dangerously-skip-permissions` |
| System-prompt delivery | `--system-prompt`, `--system-prompt-file`, `--append-system-prompt`, `--append-system-prompt-file`, `--system-prompt-snapshot`, `--exclude-dynamic-system-prompt-sections`, `--append-subagent-system-prompt[-file]` |

The system-prompt flags exist in both interactive and non-interactive modes; this topic records their spellings and value shapes only, and their semantics belong to the `system-prompt` topic.

The parsing behavior a mixed-command-line reader must reproduce:

```mermaid
flowchart TD
    start[Token after a claude switch] --> known{Known spelling?}
    known -- no --> fail[exit 1: unknown option]
    known -- yes --> type{Value type}
    type -- none --> next[consumes nothing]
    type -- string/number --> grab[grabs the next token\nspace or =form, -xvalue for one-dash]
    type -- optional --> opt{Next token looks like an option?}
    opt -- yes --> next
    opt -- no --> grab
    type -- variadic --> var[grabs the next token unconditionally,\nthen keeps taking non-option tokens]
```

## Configuration Discovery

Claude Code reads hierarchical JSON settings and keeps mutable state beside them:

| Scope | macOS / Linux path | Windows path | Notes |
| --- | --- | --- | --- |
| User settings | `~/.claude/settings.json` | `%USERPROFILE%\.claude\settings.json` | Local file exists with hooks, permissions, model, statusLine, enabledPlugins, and related keys. |
| Mutable user state | `~/.claude.json` | `%USERPROFILE%\.claude.json` | Local file exists with installMethod, projects, oauthAccount, caches, and onboarding flags; rewritten frequently. |
| Project settings | `.claude/settings.json` | `.claude\settings.json` | Checked into source control when present. |
| Local project settings | `.claude/settings.local.json` | `.claude\settings.local.json` | Gitignored when Claude Code saves a setting to it. |
| Project MCP | `.mcp.json` | `.mcp.json` | Unapproved servers are shown as pending. |
| Managed settings | `/Library/Application Support/ClaudeCode/managed-settings.json` (macOS), `/etc/claude-code/managed-settings.json` (Linux/WSL) | `C:\Program Files\ClaudeCode\managed-settings.json` | A `managed-settings.d/` drop-in directory merges in alphabetical order; also deliverable as an MDM profile or a `HKLM\SOFTWARE\Policies\ClaudeCode` registry value. The legacy `C:\ProgramData\ClaudeCode` path is not read. |
| Config-dir override | `CLAUDE_CONFIG_DIR` | `CLAUDE_CONFIG_DIR` | Moves settings, session history, and plugins together. |

Settings precedence is managed policy, command-line session values (`--settings`), local project, project, then user. Permission rules merge rather than override. Claude Code also writes transcripts, prompt history, file snapshots, caches, plugin data, and logs under `~/.claude` (relocated by `CLAUDE_CONFIG_DIR`); the local tree contains `projects`, `sessions`, `plugins`, `skills`, `agents`, `debug`, and daemon state. First use in a project may create or update these files.

Trust and validation caveats: print mode skips the workspace trust dialog; settings files that fail validation are silently ignored in print mode; `--bare` skips most customization discovery and never reads OAuth or keychain credentials; `--restricted` ignores user, project, and local settings but still applies managed settings and `--settings`.

## Environment Variables

The frontmatter `env_vars` list records general CLI and runtime variables only. Model-endpoint variables (`ANTHROPIC_*`), permission policy variables, MCP variables, and logging/telemetry variables belong to their narrower topics. Variables can be set in the shell or under the `env` key of any settings file; a settings-file `env` entry replaces the inherited shell value in most sessions, and `CLAUDE_CONFIG_DIR` plus the OpenTelemetry exporters cannot be set from project or local settings.

On/off variables accept `1`, `true`, `yes`, or `on` in any casing to turn on and the false spellings to turn off — with a documented exception class (`CLAUDE_CODE_DISABLE_NONESSENTIAL_TRAFFIC`, `DISABLE_TELEMETRY`, `DISABLE_ERROR_REPORTING`, `CLAUDE_CODE_TMUX_TRUECOLOR`) where *any* non-empty value, including `0`, turns the behavior on and only unsetting turns it off. Numeric variables accept scientific notation and digit separators. `CLAUDECODE` and `CLAUDE_CODE_CHILD_SESSION` are set by Claude Code itself in the subprocesses it spawns, which a wrapper can use to detect nesting.

## Machine Introspection

| Command | Format | Machine-readable | Wrapper use |
| --- | --- | --- | --- |
| `claude --version` | one line of text | No | Exact version; `2.1.287 (Claude Code)`. |
| `claude auth status` | JSON | Yes | Readiness: `loggedIn`, `authMethod`, `configDirectory`, subscription tier; exits 1 when logged out. |
| `claude agents --json [--all]` | JSON array | Yes | Live interactive and background sessions with pid, cwd, kind, state, sessionId, name. |
| `claude auto-mode defaults [--label x]` | JSON | Yes | Built-in classifier rule catalogs. |
| `claude auto-mode config` | JSON | Yes | Effective classifier configuration. |
| `claude plugin list --json [--available] [--data-size]` | JSON array | Yes | Installed plugin inventory with versions, scopes, and paths. |
| `claude daemon status` | text | No | Supervisor state; exits 1 when not running. |
| `claude doctor` | text | No | Install and settings diagnostics; completes without a TTY in 2.1.287. |
| `claude mcp list` | text | No | Server list with live network health checks. |

`--help` output is not counted as machine introspection, but it remains research evidence: targeted `--help` probes revealed the hidden commands and the per-path option sets recorded in the inventory.

## Wrapper Notes

The frontmatter `wrapper_notes` list carries the full caveats. The ones that most affect a program that mixes its own inputs with Claude Code switches:

1. **Non-interactive is not just `-p`.** Piped stdout alone imposes the same input requirement, so a wrapper that captures output has already changed the CLI's mode.
2. **Unknown options hard-fail.** The parser rejects them with exit 1; a wrapper cannot rely on pass-through for switches it does not know, and the docs themselves drift (`--exec` documented but absent from the 2.1.287 parser; `import` documented but disabled in this build).
3. **Variadic switches swallow neighbors.** The token after `--add-dir` is consumed even when it looks like an option, so a variadic switch must never sit directly before a flag meant for Claude Code.
4. **Optional-value switches steal plain positionals.** `-r`, `--from-pr`, `-w`, `--teleport`, `--cloud`, and `--remote-control` do not take option-looking tokens but do take the next plain one, which would swallow a user's prompt.
5. **`--debug` filters bind only via `=`.** Emit `--debug=api,hooks`, never `--debug api`.
6. **Normal states exit non-zero.** `auth status` logged out and `daemon status` without a supervisor both exit 1 with useful output.
7. **Help is incomplete by policy**, and three commands plus at least one flag are hidden; probe the binary, do not infer availability from `--help` alone.
8. **The binary moves under you.** Native installs auto-update in the background; the resolved version can change between wrapper runs.

## Sources

- [Claude Code overview](https://code.claude.com/docs/en/overview)
- [CLI reference](https://code.claude.com/docs/en/cli-reference)
- [Advanced setup](https://code.claude.com/docs/en/setup)
- [Environment variables](https://code.claude.com/docs/en/env-vars)
- [Deploy managed settings](https://code.claude.com/docs/en/managed-settings)
- Local inspection on 2026-10-01: `command -v claude`, symlink target, `claude --version`, `claude doctor`, `npm view @anthropic-ai/claude-code version dist-tags --json`, and `claude <path> --help` for every command family in the inventory (agents, attach, auth, auto-mode, daemon, doctor, gateway, import, install, logs, mcp and its leaves, plugin and its leaves including marketplace, project, respawn, rm, setup-token, stop, ultrareview, update, remote-control, self-hosted-runner).
- Disposable parse tests on 2026-10-01 in a temp directory, each with piped stdout and closed stdin: invalid-value probes for `--output-format`, `--permission-mode`, `--input-format`, `--system-prompt-snapshot`, `--permission-prompts`, `--autocompact`, `--prompt-suggestions` (space and equals forms); `--effort bogus`; `--session-id not-a-uuid`; `--add-dir` with space, equals, and option-token forms before a `--setting-sources bogus` sentinel; `-nfoo`, `-rbar`, `-Z`, `--bogus-flag`, `--exec`; bare `--max-turns`, `--permission-prompt-tool`, `--advisor`, `--append-system-prompt-file`, `--append-subagent-system-prompt[-file]`, `--teammate-mode`, `--ref`, `--fallback-model`, `--json-schema`, `--file`, `--agents`, `--settings`, `--environment`, `--model`, `--system-prompt`, `--autocompact`, `--effort`, `--sdk-url`, `--channels`, `--dangerously-load-development-channels`, `--init`, `--init-only`, `--maintenance`, `--enable-auto-mode`, `--remote`, `--rc`; `--teleport`; `mcp add -t bogus`, `-tbogus`, `--transport=bogus`; `import codex --dry-run` and `--yes=<digest> --dry-run`.
- Behavior probes on 2026-10-01: `claude auth status`, `claude agents --json`, `claude auto-mode defaults`, `claude auto-mode config`, `claude plugin list --json`, `claude daemon status`, `claude doctor`, `claude mcp list`; key-only inspection of `~/.claude/settings.json`, `~/.claude.json`, and `ls ~/.claude`.

## Changelog

- 2026-10-01: Rewrote for research contract revision 2 — typed switch inventory with value types, attachment forms, invocation scopes, and evidence citations for every record.
- 2026-10-01: Verified version 2.1.287 (local binary, doctor, npm dist-tags; stable channel 2.1.285).
- 2026-10-01: Identified the parser as commander.js and established attachment behavior from disposable tests, including the variadic grab hazard and the `--debug` equals-only filter binding.
- 2026-10-01: Recorded docs-versus-binary drift: `--exec` rejected by the 2.1.287 parser despite being documented, and `claude import` disabled in this build.
- 2026-10-01: Added newly verified switches (`--cloud`/`--remote`, `--desktop`, `--environment`, `--ref`, `--restricted`, `--permission-prompts`, `--forward-subagent-text`, `--autocompact`, `--system-prompt-snapshot`, subagent system-prompt flags, hidden `--sdk-url`, variadic `--channels` and `--dangerously-load-development-channels`) and the full mcp/plugin command-scoped inventories.
- 2026-10-01: Expanded subcommands with import, plugin marketplace, and the hidden remote-control and self-hosted-runner; re-verified `doctor` as completing without a TTY.
- 2026-10-01: Refreshed configuration discovery (exact managed-settings paths per OS, drop-in directory, registry and MDM channels) and the environment variable list against the current docs pages.
- 2026-07-03: Previous revision — version 2.1.200, hidden background-session commands, first switch inventory under revision 1.