---
$schema: ./_schema.yaml
schema_revision: 2
provider: qwen
created: 2026-07-02
last_updated: 2026-10-01
agent: opencode
model: zai-coding-plan/glm-5.3
reasoning_effort: provider_default
latest_version: 0.24.7
versions_examined:
- 0.24.7
- 0.19.8
homepage: https://qwen.ai/qwencode
repo: https://github.com/QwenLM/qwen-code
docs: https://qwenlm.github.io/qwen-code-docs/
cli_docs: https://qwenlm.github.io/qwen-code-docs/en/users/features/commands/
evidence:
- claim: 0.24.7 root help prints the complete default-command option set with aliases, type markers, choices, and defaults, plus the full subcommand list.
  id: help-0247-root
  limitations: Help display alone does not prove a switch is accepted at subcommand paths; parse tests and source establish acceptance.
  location: npx --yes @qwen-code/qwen-code@0.24.7 --help (run 2026-10-01 during this research session)
  method: local_inspection
  observed_on: 2026-10-01
  version: 0.24.7
- claim: Scoped help output for every inventoried command path, including the roughly sixty qwen serve options and each leaf's own options.
  id: help-0247-subcommands
  limitations: Leaf helps also display the default-command options that strict parsing then rejects there; only the leaf-specific options are accepted at leaves.
  location: npx --yes @qwen-code/qwen-code@0.24.7 <subcommand> --help for auth, batch, board, channel, extensions, hooks, mcp, review, sandbox, serve, sessions, update, mcp add, mcp list, sessions list, sessions ps, sessions controllers (run 2026-10-01)
  method: local_inspection
  observed_on: 2026-10-01
  version: 0.24.7
- claim: Declarations of the thirteen root/global options (TOP_LEVEL_GLOBAL_OPTIONS) and every default-command option (DEFAULT_COMMAND_OPTIONS) with aliases, types, choices, and defaults.
  id: source-top-level-options
  limitations: Declarations state intent; runtime acceptance at each command path was verified separately by parse tests.
  location: https://github.com/QwenLM/qwen-code/blob/b12edec1401a28fc53cd9e714d5928b285071fc8/packages/cli/src/config/top-level-options.ts
  method: source_code
  observed_on: 2026-10-01
  version: 0.24.7
- claim: 'yargs wiring: globals registered at root before commands, default-command options nested in the $0 [query..] builder, strict(), version() with alias v, help() with alias h, the hidden flags, array options with comma-splitting coerce, and the mutual-exclusion checks (prompt vs positional, prompt vs prompt-interactive, yolo vs approval-mode, continue vs resume vs session-id, fork-session, json-fd vs json-file, json-schema constraints, bwrap migration guard).'
  id: source-config-parser
  limitations: Covers the root parser only; per-subcommand builders live in their own command files.
  location: https://github.com/QwenLM/qwen-code/blob/b12edec1401a28fc53cd9e714d5928b285071fc8/packages/cli/src/config/config.ts
  method: source_code
  observed_on: 2026-10-01
  version: 0.24.7
- claim: sessions list declares --json (boolean) and --limit (number, default 20, integer coerce); sessions ps declares --json; sessions controllers add declares --label and --json and its list declares --json; JSON Lines go to stdout with the hasMore hint kept on stderr.
  id: source-sessions-cmds
  limitations: Does not cover group-level sessions behavior, which yargs demandCommand governs.
  location: https://github.com/QwenLM/qwen-code/blob/b12edec1401a28fc53cd9e714d5928b285071fc8/packages/cli/src/commands/sessions/list.ts (and ps.ts, controllers.ts in the same directory)
  method: source_code
  observed_on: 2026-10-01
  version: 0.24.7
- claim: mcp add option declarations including nargs:1 arrays for --env and --header and plain arrays for --include-tools, --exclude-tools, and --oauth-scopes; mcp remove re-declares --scope with the same shape as mcp add; approve, reject, and reconnect declare --all.
  id: source-mcp-cmds
  limitations: Option declarations only; server registration behavior is out of scope for this topic.
  location: https://github.com/QwenLM/qwen-code/blob/b12edec1401a28fc53cd9e714d5928b285071fc8/packages/cli/src/commands/mcp/add.ts (and remove.ts, approve.ts, reconnect.ts in the same directory)
  method: source_code
  observed_on: 2026-10-01
  version: 0.24.7
- claim: The qwen serve command module with its roughly sixty-one option declarations, their types, defaults, and choices.
  id: source-serve-cmd
  limitations: Serve runtime behavior beyond option declarations is documented by the daemon docs, not this file alone.
  location: https://github.com/QwenLM/qwen-code/blob/b12edec1401a28fc53cd9e714d5928b285071fc8/packages/cli/src/commands/serve.ts
  method: source_code
  observed_on: 2026-10-01
  version: 0.24.7
- claim: qwen sandbox declares a variadic cmd positional, --verify (boolean), and re-declares --sandbox and --sandbox-image with the default-command shapes.
  id: source-sandbox-cmd
  limitations: Sandbox backend verification behavior is out of scope.
  location: https://github.com/QwenLM/qwen-code/blob/b12edec1401a28fc53cd9e714d5928b285071fc8/packages/cli/src/commands/sandbox.ts
  method: source_code
  observed_on: 2026-10-01
  version: 0.24.7
- claim: batch run declares --dry-run and --expect; batch collect --wait and --timeout; batch retry --max-output-tokens; batch clean --force; check, cancel, and list declare no own options.
  id: source-batch-cmd
  limitations: DashScope Batch API behavior is out of scope.
  location: https://github.com/QwenLM/qwen-code/blob/b12edec1401a28fc53cd9e714d5928b285071fc8/packages/cli/src/commands/batch.ts
  method: source_code
  observed_on: 2026-10-01
  version: 0.24.7
- claim: board declares --board, --as, and --json on the group builder before its subcommands; task declares --owner, done --note, ask --about/--wait/--timeout/--ttl, prune --older-than.
  id: source-board-cmd
  limitations: Board storage location and sharing semantics are out of scope.
  location: https://github.com/QwenLM/qwen-code/blob/b12edec1401a28fc53cd9e714d5928b285071fc8/packages/cli/src/commands/board.ts
  method: source_code
  observed_on: 2026-10-01
  version: 0.24.7
- claim: review run declares --effort (choices low, medium, high), --comment, --resume, --json, --fail-on (choices none, request-changes), --timeout-minutes (default 120), --approval-mode (default yolo), and --quiet.
  id: source-review-run-cmd
  limitations: The roughly forty-one internal /review helper subcommands each have their own file and are summarized, not inventoried.
  location: https://github.com/QwenLM/qwen-code/blob/b12edec1401a28fc53cd9e714d5928b285071fc8/packages/cli/src/commands/review/run.ts (EFFORT_LEVELS in review/parse-args.ts)
  method: source_code
  observed_on: 2026-10-01
  version: 0.24.7
- claim: extensions install declares --ref, --auto-update, --pre-release, --registry, --consent, and --scope (choices user, project, workspace); update declares --all; enable and disable declare a free-string --scope.
  id: source-extensions-cmds
  limitations: Consent prompt flow and marketplace handling are out of scope.
  location: https://github.com/QwenLM/qwen-code/blob/b12edec1401a28fc53cd9e714d5928b285071fc8/packages/cli/src/commands/extensions/install.ts (and update.ts, enable.ts, disable.ts in the same directory)
  method: source_code
  observed_on: 2026-10-01
  version: 0.24.7
- claim: Per-OS system settings and system defaults paths, their QWEN_CODE_SYSTEM_SETTINGS_PATH and QWEN_CODE_SYSTEM_DEFAULTS_PATH overrides, and QWEN_HOME-based global directory resolution.
  id: source-storage-paths
  limitations: Covers path resolution, not the settings schema.
  location: https://github.com/QwenLM/qwen-code/blob/b12edec1401a28fc53cd9e714d5928b285071fc8/packages/cli/src/config/storage-paths-lite.ts
  method: source_code
  observed_on: 2026-10-01
  version: 0.24.7
- claim: 'Space and equals attachment are accepted at the root (Invalid values errors naming the switch); -obogus is parsed as a flag cluster and rejected with Unknown arguments: b, g, u; strict mode rejects default-command options at subcommand paths with Unknown arguments while the global telemetry options are choice-validated there; --no- negation parses; --version works at group and leaf paths.'
  id: parse-test-0247
  limitations: Tests prove parse-time acceptance, not runtime semantics; one probe whose value passed validation started a short billed session and was not repeated.
  location: 'Parse-level probes run against npx @qwen-code/qwen-code@0.24.7 on 2026-10-01: --approval-mode bogus (space form), --approval-mode=bogus (equals form), -o bogus, -o=bogus, -obogus, --no-chat-recording, sessions list --approval-mode bogus, sessions list --telemetry-target bogus, sessions --approval-mode bogus, sessions list --version, sessions --version, serve --approval-mode bogus'
  method: disposable_test
  observed_on: 2026-10-01
  version: 0.24.7
- claim: yargs 17.7.2 accepts --opt value and --opt=value for long and short flags, rejects attached short values (-ojson fails choices validation as separate flags), array options greedily consume following non-flag arguments (--allowed-tools a b c yields a, b, c) and stop at the next flag, repeated occurrences accumulate, nargs:1 arrays take exactly one value per occurrence, a zero-value array occurrence parses to an empty list, and --no-boolean negation works.
  id: yargs-parser-test
  limitations: Isolated harness, not the qwen binary itself; qwen adds comma-splitting coerce functions on top.
  location: 'Disposable harness at /tmp/qwen-yargs-test using yargs@17.7.2, the version packages/cli/package.json of 0.24.7 depends on, configured like qwen: strict, aliases, choices, array+string options, nargs:1 arrays, $0 [query..] default command'
  method: disposable_test
  observed_on: 2026-10-01
  version: 0.24.7
- claim: Configuration layer precedence (defaults, system defaults, user, project, system override, environment, CLI), the four settings file locations per OS, the .env loading order and rejection lists, the hierarchical context file lookup (QWEN.md under ~/.qwen and project ancestors), and the environment variable and command-line argument tables.
  id: docs-settings-page
  limitations: Documentation prose; every CLI-table claim used here was cross-checked against source or a parse test.
  location: https://qwenlm.github.io/qwen-code-docs/en/users/configuration/settings/
  method: official_docs
  observed_on: 2026-10-01
  version: 0.24.7
- claim: Official install commands (standalone curl and irm, npm install -g @qwen-code/qwen-code@latest, brew install qwen-code), the Node.js 22 prerequisite, the first-run /auth flow, and that Qwen OAuth was discontinued on April 15, 2026.
  id: docs-quickstart
  limitations: Does not describe switch behavior.
  location: https://qwenlm.github.io/qwen-code-docs/en/users/quickstart/
  method: official_docs
  observed_on: 2026-10-01
  version: 0.24.7
- claim: npm latest dist-tag is 0.24.7 with engines node >=22.0.0; a 0.24.7-nightly channel exists.
  id: npm-registry-check
  limitations: Registry metadata only; no runtime claims.
  location: npm view @qwen-code/qwen-code version dist-tags engines --json (run 2026-10-01)
  method: local_inspection
  observed_on: 2026-10-01
  version: 0.24.7
- claim: The installed Homebrew qwen on this host is 0.19.8, older than npm latest 0.24.7; sessions list --json emits one JSON object per line with sessionId, startTime, mtime, prompt, gitBranch, customTitle, titleSource, filePath, and cwd.
  id: local-binary-0198
  limitations: One macOS host; version skew means the installed surface may lag the documented 0.24.7 surface.
  location: qwen --version on this host (Homebrew-linked /opt/homebrew/bin/qwen), plus qwen sessions list --json --limit 2 (run 2026-10-01)
  method: local_inspection
  observed_on: 2026-10-01
  version: 0.19.8
- claim: Homebrew stable for qwen-code is 0.24.7 (bottled); the installed keg on this host is 0.19.8.
  id: brew-formula-0247
  limitations: Homebrew channel only.
  location: brew info qwen-code (run 2026-10-01)
  method: local_inspection
  observed_on: 2026-10-01
  version: 0.24.7
- claim: sandbox prints a text policy report; --list-extensions prints a text list and exits; board show requires --board and errors without it; auth prints removal guidance and exits; hooks prints nothing and exits 0; channel status prints service status and exits 0; extension leaves complete without prompting when given a missing name.
  id: probes-0247
  limitations: Probes with empty local state; output shape may grow with configured state.
  location: 'Non-model probes against npx 0.24.7 on 2026-10-01: qwen sandbox, qwen --list-extensions, qwen board show --json, qwen auth, qwen hooks, qwen channel status, qwen extensions list, uninstall, enable, disable, update, and sources list with a nonexistent extension name'
  method: local_inspection
  observed_on: 2026-10-01
  version: 0.24.7
binaries:
- alt_binaries: []
  binary: qwen
  notes: Homebrew links /opt/homebrew/bin/qwen into the qwen-code Cellar keg (0.19.8 on this host while the formula is 0.24.7). The npm package also exposes bin.qwen. Confirmed with which -a qwen and qwen --version on 2026-10-01.
  os: macos
- alt_binaries: []
  binary: qwen
  notes: The npm package bin mapping and the standalone installer both install qwen; not run on Linux by this research.
  os: linux
- alt_binaries:
  - qwen.cmd
  - qwen.ps1
  binary: qwen
  notes: Official examples invoke qwen. npm installs normally create qwen.cmd and qwen.ps1 shims; the Windows standalone path uses the PowerShell installer. Not run on Windows by this research.
  os: windows
install_methods:
- command: curl -fsSL https://qwen-code-assets.oss-cn-hangzhou.aliyuncs.com/installation/install-qwen-standalone.sh | bash
  method: standalone_binary
  notes: Official quick-install path from the quickstart; installs a self-updating standalone build.
  os: macos
- command: curl -fsSL https://qwen-code-assets.oss-cn-hangzhou.aliyuncs.com/installation/install-qwen-standalone.sh | bash
  method: standalone_binary
  notes: Official quick-install path from the quickstart.
  os: linux
- command: irm https://qwen-code-assets.oss-cn-hangzhou.aliyuncs.com/installation/install-qwen-standalone.ps1 | iex
  method: standalone_binary
  notes: Official quick-install path from the quickstart (PowerShell).
  os: windows
- command: npm install -g @qwen-code/qwen-code@latest
  method: npm
  notes: npm latest is 0.24.7 and requires Node.js >=22.0.0.
  os: macos
- command: npm install -g @qwen-code/qwen-code@latest
  method: npm
  notes: npm latest is 0.24.7 and requires Node.js >=22.0.0.
  os: linux
- command: npm install -g @qwen-code/qwen-code@latest
  method: npm
  notes: Requires Node.js >=22.0.0; creates qwen.cmd and qwen.ps1 shims on Windows.
  os: windows
- command: brew install qwen-code
  method: brew
  notes: Homebrew stable is 0.24.7 (bottled); the keg installed on this host was 0.19.8, so an installed binary can lag the formula.
  os: macos
- command: brew install qwen-code
  method: brew
  notes: The Homebrew formula supports Linux bottles per the official quickstart.
  os: linux
subcommands:
- description: Removed legacy authentication command that prints migration guidance and exits.
  name: auth
  non_interactive: true
  notes: Prints guidance pointing at interactive /auth or CI provider environment variables and the --openai-* flags; verified on 0.24.7 (probes-0247).
- description: Dispatcher for DashScope Batch API workflow tasks.
  name: batch
  non_interactive: true
  notes: Requires a subcommand (demandCommand). Every leaf completes without a terminal; several leaves submit billed work.
- description: Assemble, submit, and record an agent-prepared batch plan.
  name: batch run
  non_interactive: true
  notes: Billed DashScope submission; reads a plan JSON usually written by the /batch-api skill.
- description: Reconcile, download, validate, and deliver a workflow task's results.
  name: batch collect
  non_interactive: true
  notes: --wait polls over HTTP until the batch settles.
- description: Resubmit only the failed items of a workflow task.
  name: batch retry
  non_interactive: true
  notes: Billed; --max-output-tokens is required to resend items that were truncated.
- description: Cancel a workflow task's active batch.
  name: batch cancel
  non_interactive: true
  notes: Already-completed requests are still billed.
- description: List recorded workflow tasks with their project.
  name: batch list
  non_interactive: true
  notes: Local record listing; no own options.
- description: Verify credentials and the Batch route and show the settings a run would freeze.
  name: batch check
  non_interactive: true
  notes: Explicitly performs no billed request; useful as a preflight.
- description: Delete a workflow task's local record.
  name: batch clean
  non_interactive: true
  notes: Cancels nothing; refuses while a batch may be running unless --force is given.
- description: Dispatcher for the agent board, a shared work board for cooperating agents.
  name: board
  non_interactive: true
  notes: Group options --board, --as, and --json cascade to every leaf; board show without --board errors with Pass --board <name>.
- description: Print the board once.
  name: board show
  non_interactive: true
  notes: Requires --board; --json emits JSON (probes-0247).
- description: Create a task on the board.
  name: board task
  non_interactive: true
  notes: Positional subject; --owner assigns ownership.
- description: Take ownership of a task.
  name: board claim
  non_interactive: true
  notes: Positional task id.
- description: Complete a task you own.
  name: board done
  non_interactive: true
  notes: Positional task id; --note records a completion note.
- description: Ask another actor on the board a question.
  name: board ask
  non_interactive: true
  notes: --wait blocks until answered or the --timeout (default 30 seconds) elapses; bounded, and no human prompt is involved.
- description: Answer an ask addressed to you.
  name: board answer
  non_interactive: true
  notes: Positionals ask id and answer.
- description: Decline an ask addressed to you.
  name: board decline
  non_interactive: true
  notes: Positionals ask id and reason.
- description: Remove settled board items older than a cutoff.
  name: board prune
  non_interactive: true
  notes: --older-than defaults to 7 days.
- description: Dispatcher for messaging channel integrations (Telegram, Discord, WeChat, and more).
  name: channel
  non_interactive: false
  notes: Requires a subcommand. Leaves range from daemon control to QR-code pairing flows.
- description: Start channels, all when no name is given.
  name: channel start
  non_interactive: false
  notes: Long-running channel workers; pairing may be required first.
- description: Stop the running channel service.
  name: channel stop
  non_interactive: false
  notes: Mutates running daemon state.
- description: Show channel service status.
  name: channel status
  non_interactive: true
  notes: Prints status and exits 0 when no service is running (probes-0247).
- description: Reload the daemon-managed channel worker so it re-reads settings.json.
  name: channel reload
  non_interactive: false
  notes: Targets a running qwen serve daemon.
- description: Set the channel selection for a running qwen serve daemon.
  name: channel set
  non_interactive: false
  notes: Takes one or more channel names.
- description: Manage DM and group pairing requests.
  name: channel pairing
  non_interactive: false
  notes: Has its own leaves channel pairing list <name> and channel pairing approve <name> <code>.
- description: Configure the WeChat channel, logging in via QR code.
  name: channel configure-weixin
  non_interactive: false
  notes: QR-code login requires a person with a phone.
- description: Dispatcher for Qwen Code extension management.
  name: extensions
  non_interactive: false
  notes: Requires a subcommand; install prompts for consent unless --consent is passed. The other leaves were probed to complete without prompting on a missing extension name (probes-0247).
- description: Install an extension from a git URL, local path or archive, archive URL, scoped npm package, or claude marketplace reference.
  name: extensions install
  non_interactive: false
  notes: Prompts for a security confirmation unless --consent is given.
- description: Uninstall an extension.
  name: extensions uninstall
  non_interactive: true
  notes: Completes with an error when the name does not exist (probes-0247).
- description: List installed extensions.
  name: extensions list
  non_interactive: true
  notes: Text output; no JSON mode found.
- description: Update all extensions or a named extension to the latest version.
  name: extensions update
  non_interactive: true
  notes: --all or a name is required; prints usage otherwise and exits.
- description: Disable an extension.
  name: extensions disable
  non_interactive: true
  notes: Optional --scope; completes without prompting.
- description: Enable an extension.
  name: extensions enable
  non_interactive: true
  notes: Optional --scope; completes without prompting.
- description: Link an extension from a local path so edits are reflected live.
  name: extensions link
  non_interactive: true
  notes: Mutates extension state; no prompt observed in help or source.
- description: Create a new extension from a boilerplate example.
  name: extensions new
  non_interactive: true
  notes: Writes files; takes a path and an optional template.
- description: Manage per-extension settings.
  name: extensions settings
  non_interactive: true
  notes: Leaves extensions settings set [--scope] <name> <setting> and extensions settings list <name>.
- description: Manage marketplace sources for discovering extensions.
  name: extensions sources
  non_interactive: true
  notes: Leaves extensions sources add <source>, remove <name>, list, and update <name>.
- description: Hook management placeholder directing users to the /hooks slash command.
  name: hooks
  non_interactive: true
  notes: 'Alias: hook. Running qwen hooks bare on 0.24.7 prints nothing and exits 0; hook configuration lives in settings.json and the interactive /hooks dialog.'
- description: Dispatcher for MCP server management.
  name: mcp
  non_interactive: true
  notes: Requires a subcommand; every leaf completes without a terminal, though add, remove, approve, and reject mutate persistent settings or approval state.
- description: Add an MCP server from a stdio command or an sse/http URL.
  name: mcp add
  non_interactive: true
  notes: Positionals name and commandOrUrl plus variadic args; writes user or project settings according to --scope.
- description: Remove a configured MCP server.
  name: mcp remove
  non_interactive: true
  notes: Positional name; --scope selects which settings file to edit.
- description: List all configured MCP servers.
  name: mcp list
  non_interactive: true
  notes: Text output; no JSON mode and no own options.
- description: Reconnect to MCP servers.
  name: mcp reconnect
  non_interactive: true
  notes: Optional server name or --all.
- description: Approve a pending MCP server.
  name: mcp approve
  non_interactive: true
  notes: Optional name or --all; mutates approval state.
- description: Reject a pending MCP server.
  name: mcp reject
  non_interactive: true
  notes: Optional name or --all; mutates approval state.
- description: Run a code review non-interactively, plus the internal helpers used by the /review skill.
  name: review
  non_interactive: true
  notes: The public entry is review run. The remaining roughly forty-one leaves (parse-args, match-remote, meta, issue-context, fetch-diff, comment-body, fetch-pr, capture-local, plan-diff, cache-commit, repo-context, pr-context, comment-status, load-rules, agent-prompt, emit-workflow, build-test, base-tree, scratch-tree, test-delta, fix-delta, drive, ab-drive, mock-provider, extract-step, script-lint, dedup-candidates, revert-hunk, resolve-anchors, check-coverage, cost-ledger, presubmit, test-efficacy, test-plan, findings, recover-findings, publish-assets, compose-review, save-artifact, submit, cleanup) are internal skill helpers, summarized here rather than inventoried individually.
- description: Run a full /review non-interactively and print the verdict.
  name: review run
  non_interactive: true
  notes: Machine-readable with --json; --fail-on makes a verdict exit 3 for CI gating; child sessions default to approval-mode yolo because headless runs cannot answer prompts.
- description: Inspect the sandbox backend, verify it, or run one command inside it.
  name: sandbox
  non_interactive: true
  notes: Bare invocation reports the effective execution policy as text; --verify proves the kernel boundary; a variadic cmd positional after -- runs one confined command.
- description: Run Qwen Code as a local HTTP daemon.
  name: serve
  non_interactive: true
  notes: Long-running daemon with no TTY; loopback is auth-free unless --require-auth is set; the handler blocks forever, so SIGINT and SIGTERM drive shutdown.
- description: Dispatcher for saved-session management.
  name: sessions
  non_interactive: true
  notes: Requires a subcommand; all leaves complete without a terminal.
- description: List saved sessions.
  name: sessions list
  non_interactive: true
  notes: --json emits one JSON object per line on stdout, with the hasMore hint kept on stderr.
- description: List registered and managed Qwen Code sessions.
  name: sessions ps
  non_interactive: true
  notes: --json emits JSON Lines.
- description: Manage the controller tokens that may drive your sessions.
  name: sessions controllers
  non_interactive: true
  notes: Leaves sessions controllers add (--label, --json), list (--json), and remove <id>; add prints the minted token once.
- description: Check for Qwen Code updates and install if available.
  name: update
  non_interactive: true
  notes: No own options beyond the globals; not executed by this research because it mutates the installation.
cli_switches:
- aliases:
  - -h
  attachment: []
  default: 'false'
  description: Show help for the current command path.
  evidence_ids:
  - source-config-parser
  - help-0247-root
  example: qwen --help
  flag: --help
  invocation_scope:
  - applies_to: global
  notes: Registered at the root via help() with alias h, so it works before and after subcommand words.
  scope:
  - diagnostics
  value: none
  value_type: none
- aliases:
  - -v
  attachment: []
  default: 'false'
  description: Print the CLI version and exit.
  evidence_ids:
  - source-config-parser
  - parse-test-0247
  example: qwen --version
  flag: --version
  invocation_scope:
  - applies_to: global
  notes: 'Verified at group and leaf paths: qwen sessions --version and qwen sessions list --version both print 0.24.7.'
  scope:
  - diagnostics
  value: none
  value_type: none
- attachment: []
  description: Enable telemetry sending; the other --telemetry-* flags only set values.
  evidence_ids:
  - source-top-level-options
  - help-0247-root
  example: qwen --telemetry "summarize"
  flag: --telemetry
  invocation_scope:
  - applies_to: global
  notes: Deprecated in favor of telemetry.enabled in settings.json; supports the --no- form.
  scope:
  - telemetry
  value: none
  value_type: none
- attachment:
  - space
  - equals
  description: Set the telemetry target, overriding settings files.
  evidence_ids:
  - source-top-level-options
  - parse-test-0247
  example: qwen --telemetry-target local "summarize"
  flag: --telemetry-target
  invocation_scope:
  - applies_to: global
  notes: Deprecated; choice-validated, and the Invalid values error observed at a leaf path proves the global scope.
  scope:
  - telemetry
  value: local | gcp
  value_optional: false
  value_type: string
- attachment:
  - space
  - equals
  description: Set the OTLP endpoint for telemetry.
  evidence_ids:
  - source-top-level-options
  - help-0247-root
  example: qwen --telemetry-otlp-endpoint http://localhost:4317 "run"
  flag: --telemetry-otlp-endpoint
  invocation_scope:
  - applies_to: global
  notes: Deprecated in favor of telemetry.otlpEndpoint.
  scope:
  - telemetry
  value: <URL>
  value_optional: false
  value_type: string
- attachment:
  - space
  - equals
  default: grpc
  description: Set the OTLP protocol for telemetry.
  evidence_ids:
  - source-top-level-options
  - help-0247-root
  example: qwen --telemetry-otlp-protocol http "run"
  flag: --telemetry-otlp-protocol
  invocation_scope:
  - applies_to: global
  notes: Deprecated; choices grpc and http.
  scope:
  - telemetry
  value: grpc | http
  value_optional: false
  value_type: string
- attachment: []
  description: Enable logging of user prompts for telemetry.
  evidence_ids:
  - source-top-level-options
  - help-0247-root
  example: qwen --telemetry-log-prompts "run"
  flag: --telemetry-log-prompts
  invocation_scope:
  - applies_to: global
  notes: Deprecated in favor of telemetry.logPrompts; supports the --no- form.
  scope:
  - telemetry
  value: none
  value_type: none
- attachment:
  - space
  - equals
  description: Redirect all telemetry output to a file.
  evidence_ids:
  - source-top-level-options
  - help-0247-root
  example: qwen --telemetry-outfile ./qwen-telemetry.jsonl "run"
  flag: --telemetry-outfile
  invocation_scope:
  - applies_to: global
  notes: Deprecated in favor of telemetry.outfile.
  scope:
  - telemetry
  value: <PATH>
  value_optional: false
  value_type: string
- aliases:
  - -d
  attachment: []
  default: 'false'
  description: Run in debug mode.
  evidence_ids:
  - source-top-level-options
  - help-0247-root
  example: qwen --debug "diagnose"
  flag: --debug
  invocation_scope:
  - applies_to: global
  notes: Also settable via the DEBUG and DEBUG_MODE environment variables.
  scope:
  - diagnostics
  value: none
  value_type: none
- attachment: []
  default: 'false'
  description: 'Minimal mode: skip implicit startup auto-discovery and honor only explicit CLI inputs.'
  evidence_ids:
  - source-top-level-options
  - help-0247-root
  example: qwen --bare "summarize this repo"
  flag: --bare
  invocation_scope:
  - applies_to: global
  notes: Wrapper-friendly isolation; also disables settings-sourced output styles.
  scope:
  - isolation
  value: none
  value_type: none
- attachment: []
  description: Disable all customizations (context files, hooks, extensions, skills, MCP servers) for troubleshooting.
  evidence_ids:
  - source-top-level-options
  - help-0247-root
  example: qwen --safe-mode "reproduce this bug"
  flag: --safe-mode
  invocation_scope:
  - applies_to: global
  notes: Also settable with QWEN_CODE_SAFE_MODE=true; the tools.executionSandbox policy is retained.
  scope:
  - isolation
  value: none
  value_type: none
- attachment:
  - space
  - equals
  description: Set the proxy for Qwen Code, like schema://user:password@host:port.
  evidence_ids:
  - source-top-level-options
  - help-0247-root
  example: qwen --proxy http://localhost:7890 "run"
  flag: --proxy
  invocation_scope:
  - applies_to: global
  notes: Deprecated in favor of the proxy setting; HTTPS_PROXY and HTTP_PROXY are fallbacks.
  scope:
  - network
  value: <URL>
  value_optional: false
  value_type: string
- attachment: []
  default: 'false'
  description: Skip TLS certificate verification for API connections.
  evidence_ids:
  - source-top-level-options
  - help-0247-root
  example: qwen --insecure "test local provider"
  flag: --insecure
  invocation_scope:
  - applies_to: global
  notes: Equivalent to QWEN_TLS_INSECURE=1; affects API, OAuth, MCP, and child-process HTTPS.
  scope:
  - network
  value: none
  value_type: none
- attachment: []
  description: Enable chat recording to disk.
  evidence_ids:
  - source-top-level-options
  - parse-test-0247
  example: qwen --no-chat-recording "one-off"
  flag: --chat-recording
  invocation_scope:
  - applies_to: global
  notes: yargs boolean negation --no-chat-recording verified by parse test; disabling prevents --continue and --resume from working.
  scope:
  - sessions
  value: none
  value_type: none
- aliases:
  - -m
  attachment:
  - space
  - equals
  description: Select the model for this session.
  evidence_ids:
  - source-top-level-options
  - docs-settings-page
  - parse-test-0247
  example: qwen --model qwen3-coder-plus "inspect this repo"
  flag: --model
  invocation_scope:
  - applies_to: command
    command: []
  notes: Accepted at the root entrypoint only; qwen sessions list --model is rejected as unknown.
  scope:
  - model_selection
  value: <MODEL>
  value_optional: false
  value_type: string
- attachment:
  - space
  - equals
  description: Advisor model selector for this session; off disables the native Advisor for this run.
  evidence_ids:
  - source-top-level-options
  - help-0247-root
  example: qwen --advisor off "work"
  flag: --advisor
  invocation_scope:
  - applies_to: command
    command: []
  notes: New since the 0.19.x line.
  scope:
  - model_selection
  value: <MODEL|off>
  value_optional: false
  value_type: string
- attachment:
  - space
  - equals
  description: Fallback models for capacity errors (429, 503, 529), repeatable or comma-separated, at most three.
  evidence_ids:
  - source-config-parser
  - yargs-parser-test
  example: qwen --fallback-model qwen3-coder-plus,qwen3-coder-flash "work"
  flag: --fallback-model
  gap: yargs array options accept a zero-value occurrence (parses to an empty list; parser test), so the fewest values is 0, below the contract floor of 1; reading the coerce consumers in source would settle whether an empty list is treated as unset.
  invocation_scope:
  - applies_to: command
    command: []
  notes: yargs array with a comma-splitting coerce; greedy across following non-flag arguments.
  scope:
  - model_selection
  value: <MODEL>[,<MODEL>...]
  value_type: variadic
  variadic_min: unknown
- aliases:
  - -p
  attachment:
  - space
  - equals
  description: Supply a headless prompt; appended to stdin content when stdin is piped.
  evidence_ids:
  - source-top-level-options
  - source-config-parser
  - docs-settings-page
  example: qwen --prompt "summarize"
  flag: --prompt
  invocation_scope:
  - applies_to: command
    command: []
  notes: Deprecated in favor of the positional prompt; rejected together with a positional prompt or with --prompt-interactive.
  scope:
  - non_interactive
  value: <PROMPT>
  value_optional: false
  value_type: string
- aliases:
  - -i
  attachment:
  - space
  - equals
  description: Execute the provided prompt and continue in interactive mode.
  evidence_ids:
  - source-top-level-options
  - source-config-parser
  example: qwen --prompt-interactive "start by reading README"
  flag: --prompt-interactive
  invocation_scope:
  - applies_to: command
    command: []
  notes: An empty value is explicitly handled in config.ts, so the value may be omitted; the positional prompt fills it when both are given.
  scope:
  - interactive
  value: <PROMPT>
  value_optional: true
  value_type: string
- attachment:
  - space
  - equals
  description: Override the main session system prompt for this run.
  evidence_ids:
  - source-top-level-options
  - help-0247-root
  example: qwen "review" --system-prompt "You are a strict reviewer."
  flag: --system-prompt
  invocation_scope:
  - applies_to: command
    command: []
  notes: Existence and value type recorded here; replace-versus-append semantics belong to the system-prompt topic.
  scope:
  - prompting
  value: <TEXT>
  value_optional: false
  value_type: string
- attachment:
  - space
  - equals
  description: Append instructions to the main session system prompt for this run.
  evidence_ids:
  - source-top-level-options
  - help-0247-root
  example: qwen "review" --append-system-prompt "Focus on regressions."
  flag: --append-system-prompt
  invocation_scope:
  - applies_to: command
    command: []
  notes: Can be combined with --system-prompt; semantics belong to the system-prompt topic.
  scope:
  - prompting
  value: <TEXT>
  value_optional: false
  value_type: string
- attachment:
  - space
  - equals
  description: Output style for this run, for example Concise or Explanatory; default selects no style.
  evidence_ids:
  - source-top-level-options
  - docs-settings-page
  example: qwen --output-style Concise "explain"
  flag: --output-style
  invocation_scope:
  - applies_to: command
    command: []
  notes: Overrides general.outputStyle; ignored in --bare and --safe-mode. New since the 0.19.x line.
  scope:
  - prompting
  value: <NAME>
  value_optional: false
  value_type: string
- aliases:
  - -s
  attachment: []
  description: Run the session in a sandbox.
  evidence_ids:
  - source-top-level-options
  - source-sandbox-cmd
  - source-config-parser
  example: qwen --sandbox "run tests"
  flag: --sandbox
  invocation_scope:
  - applies_to: command
    command: []
  - applies_to: command
    command:
    - sandbox
  notes: Declared in the default command and re-declared by the sandbox subcommand; a parse-time check rejects the removed bwrap value in the --sandbox=bwrap, -s=bwrap, and --sandbox bwrap spellings; QWEN_SANDBOX can override.
  scope:
  - sandbox
  value: none
  value_type: none
- attachment:
  - space
  - equals
  description: Sandbox image URI.
  evidence_ids:
  - source-top-level-options
  - source-sandbox-cmd
  - docs-settings-page
  example: qwen --sandbox --sandbox-image ghcr.io/qwenlm/qwen-code:0.24.7 "run"
  flag: --sandbox-image
  invocation_scope:
  - applies_to: command
    command: []
  - applies_to: command
    command:
    - sandbox
  notes: Deprecated in favor of tools.sandboxImage; precedence is --sandbox-image over QWEN_SANDBOX_IMAGE over the setting over the built-in default.
  scope:
  - sandbox
  value: <IMAGE>
  value_optional: false
  value_type: string
- aliases:
  - -y
  attachment: []
  default: 'false'
  description: Automatically accept all actions (YOLO mode).
  evidence_ids:
  - source-top-level-options
  - source-config-parser
  example: qwen --yolo "apply the fix"
  flag: --yolo
  invocation_scope:
  - applies_to: command
    command: []
  notes: Rejected together with --approval-mode; does not enable sandboxing; permission detail belongs to agent-permissions.
  scope:
  - permissions
  value: none
  value_type: none
- attachment:
  - space
  - equals
  default: default
  description: Set the approval mode for tool execution.
  evidence_ids:
  - source-top-level-options
  - parse-test-0247
  example: qwen --approval-mode plan "make a plan"
  flag: --approval-mode
  invocation_scope:
  - applies_to: command
    command: []
  notes: auto uses an LLM classifier that auto-approves safe actions; rejected together with --yolo; space and equals attachment proven by parse test.
  scope:
  - permissions
  value: plan | default | auto-edit | auto | yolo
  value_optional: false
  value_type: string
- attachment: []
  description: Start the agent in ACP mode.
  evidence_ids:
  - source-top-level-options
  - source-config-parser
  example: qwen --acp
  flag: --acp
  invocation_scope:
  - applies_to: command
    command: []
  notes: Replaces the hidden --experimental-acp; incompatible with --json-schema.
  scope:
  - protocol
  value: none
  value_type: none
- attachment: []
  description: Hidden deprecated alias that maps onto --acp.
  evidence_ids:
  - source-config-parser
  example: qwen --experimental-acp
  flag: --experimental-acp
  invocation_scope:
  - applies_to: command
    command: []
  notes: Hidden flag; prints a deprecation warning and enables acp when --acp is absent.
  scope:
  - protocol
  value: none
  value_type: none
- attachment: []
  description: Hidden ignored flag; skills are enabled by default.
  evidence_ids:
  - source-config-parser
  example: qwen --experimental-skills "work"
  flag: --experimental-skills
  invocation_scope:
  - applies_to: command
    command: []
  notes: Hidden flag kept for compatibility; parses but does nothing.
  scope:
  - skills
  value: none
  value_type: none
- attachment: []
  default: 'false'
  description: Enable the experimental LSP feature for code intelligence.
  evidence_ids:
  - source-top-level-options
  - help-0247-subcommands
  example: qwen --experimental-lsp "refactor"
  flag: --experimental-lsp
  invocation_scope:
  - applies_to: command
    command: []
  - applies_to: command
    command:
    - serve
  notes: At serve the same spelling forwards the opt-in to spawned agent sessions.
  scope:
  - lsp
  value: none
  value_type: none
- attachment: []
  default: 'false'
  description: On daemon session load or resume, re-hang a trailing unanswered ask_user_question instead of synthesizing a failed tool result.
  evidence_ids:
  - source-top-level-options
  - help-0247-subcommands
  example: qwen serve --restore-ask-user-question
  flag: --restore-ask-user-question
  invocation_scope:
  - applies_to: command
    command: []
  - applies_to: command
    command:
    - serve
  notes: Declared in both the default command and serve. New since the 0.19.x line.
  scope:
  - sessions
  value: none
  value_type: none
- attachment:
  - space
  - equals
  description: Channel identifier for the default command.
  evidence_ids:
  - source-top-level-options
  - help-0247-root
  example: qwen --channel CI "run checks"
  flag: --channel
  invocation_scope:
  - applies_to: command
    command: []
  notes: The serve subcommand accepts a different repeatable --channel for daemon-managed channel workers; recorded separately.
  scope:
  - integration
  value: VSCode | ACP | SDK | CI | desktop | daemon
  value_optional: false
  value_type: string
- attachment:
  - space
  - equals
  description: Allowed MCP server names.
  evidence_ids:
  - source-config-parser
  - yargs-parser-test
  example: qwen --allowed-mcp-server-names filesystem,github "work"
  flag: --allowed-mcp-server-names
  gap: yargs array options accept a zero-value occurrence (parses to an empty list; parser test), so the fewest values is 0, below the contract floor of 1; reading the coerce consumers in source would settle whether an empty list is treated as unset.
  invocation_scope:
  - applies_to: command
    command: []
  notes: yargs array with comma-splitting coerce; greedy across following non-flag arguments; MCP detail belongs to the mcp topic.
  scope:
  - mcp
  value: <NAME>[,<NAME>...]
  value_type: variadic
  variadic_min: unknown
- attachment:
  - space
  - equals
  description: MCP server configuration as inline JSON with mcpServers format or a path to a JSON file.
  evidence_ids:
  - source-top-level-options
  - help-0247-root
  example: qwen --mcp-config ./mcp.json "use tools"
  flag: --mcp-config
  invocation_scope:
  - applies_to: command
    command: []
  notes: Claudine today directs Qwen users to claudine mcp export qwen --apply rather than runtime injection.
  scope:
  - mcp
  value: <JSON_OR_PATH>
  value_optional: false
  value_type: string
- attachment:
  - space
  - equals
  description: Tools to allow, bypassing confirmation.
  evidence_ids:
  - source-config-parser
  - yargs-parser-test
  example: qwen --allowed-tools read_file,grep "inspect"
  flag: --allowed-tools
  gap: yargs array options accept a zero-value occurrence (parses to an empty list; parser test), so the fewest values is 0, below the contract floor of 1; reading the coerce consumers in source would settle whether an empty list is treated as unset.
  invocation_scope:
  - applies_to: command
    command: []
  notes: yargs array with comma-splitting coerce; greedy across following non-flag arguments; permission detail belongs to agent-permissions.
  scope:
  - permissions
  value: <TOOL>[,<TOOL>...]
  value_type: variadic
  variadic_min: unknown
- aliases:
  - -e
  attachment:
  - space
  - equals
  description: Extensions to use for the session; all are used when not provided.
  evidence_ids:
  - source-config-parser
  - yargs-parser-test
  example: qwen --extensions my-extension "use it"
  flag: --extensions
  gap: yargs array options accept a zero-value occurrence (parses to an empty list; parser test), so the fewest values is 0, below the contract floor of 1; reading the coerce consumers in source would settle whether an empty list is treated as unset.
  invocation_scope:
  - applies_to: command
    command: []
  notes: yargs array with comma-splitting coerce; greedy across following non-flag arguments.
  scope:
  - extensions
  value: <EXT>[,<EXT>...]
  value_type: variadic
  variadic_min: unknown
- aliases:
  - -l
  attachment: []
  description: List all available extensions and exit.
  evidence_ids:
  - source-top-level-options
  - probes-0247
  example: qwen --list-extensions
  flag: --list-extensions
  invocation_scope:
  - applies_to: command
    command: []
  notes: Text output only; verified on 0.24.7.
  scope:
  - extensions
  value: none
  value_type: none
- aliases:
  - --add-dir
  attachment:
  - space
  - equals
  description: Additional directories to include in the workspace, comma-separated or repeated.
  evidence_ids:
  - source-top-level-options
  - source-config-parser
  - yargs-parser-test
  example: qwen --include-directories ../lib,../cli "inspect both"
  flag: --include-directories
  gap: yargs array options accept a zero-value occurrence (parses to an empty list; parser test), so the fewest values is 0, below the contract floor of 1; reading the coerce consumers in source would settle whether an empty list is treated as unset.
  invocation_scope:
  - applies_to: command
    command: []
  notes: Long alias --add-dir; yargs array with comma-splitting coerce; greedy across following non-flag arguments.
  scope:
  - context
  value: <PATH>[,<PATH>...]
  value_type: variadic
  variadic_min: unknown
- attachment: []
  description: Enable logging of OpenAI API calls for debugging and analysis.
  evidence_ids:
  - source-top-level-options
  - docs-settings-page
  example: qwen --openai-logging "debug provider"
  flag: --openai-logging
  invocation_scope:
  - applies_to: command
    command: []
  notes: Logging surfaces belong to agent-logging.
  scope:
  - diagnostics
  value: none
  value_type: none
- attachment:
  - space
  - equals
  description: Custom directory for OpenAI API logs; overrides settings files.
  evidence_ids:
  - source-top-level-options
  - docs-settings-page
  example: qwen --openai-logging --openai-logging-dir ~/qwen-logs "run"
  flag: --openai-logging-dir
  invocation_scope:
  - applies_to: command
    command: []
  notes: Logging surfaces belong to agent-logging.
  scope:
  - diagnostics
  value: <PATH>
  value_optional: false
  value_type: string
- attachment:
  - space
  - equals
  description: OpenAI-compatible API key for authentication.
  evidence_ids:
  - source-top-level-options
  - help-0247-root
  example: qwen --openai-api-key "$KEY" --openai-base-url http://localhost:11434/v1 --model qwen3-coder "run"
  flag: --openai-api-key
  invocation_scope:
  - applies_to: command
    command: []
  notes: Endpoint semantics belong to model-config; visible in process argv, so prefer env vars for secrets.
  scope:
  - auth
  value: <KEY>
  value_optional: false
  value_type: string
- attachment:
  - space
  - equals
  description: OpenAI-compatible base URL for custom endpoints.
  evidence_ids:
  - source-top-level-options
  - help-0247-root
  example: qwen --openai-base-url http://localhost:11434/v1 --model qwen3-coder "run"
  flag: --openai-base-url
  invocation_scope:
  - applies_to: command
    command: []
  notes: Endpoint semantics belong to model-config.
  scope:
  - auth
  value: <URL>
  value_optional: false
  value_type: string
- attachment: []
  description: Enable screen reader mode for accessibility.
  evidence_ids:
  - source-top-level-options
  - help-0247-root
  example: qwen --screen-reader
  flag: --screen-reader
  invocation_scope:
  - applies_to: command
    command: []
  notes: Adjusts the TUI; a ui.accessibility.screenReader setting also exists.
  scope:
  - accessibility
  value: none
  value_type: none
- attachment:
  - space
  - equals
  default: text
  description: Format consumed from standard input.
  evidence_ids:
  - source-top-level-options
  - source-config-parser
  example: qwen --input-format stream-json --output-format stream-json
  flag: --input-format
  invocation_scope:
  - applies_to: command
    command: []
  notes: stream-json input requires --output-format stream-json (checked at parse time).
  scope:
  - io
  value: text | stream-json
  value_optional: false
  value_type: string
- aliases:
  - -o
  attachment:
  - space
  - equals
  default: text
  description: Format of the CLI output.
  evidence_ids:
  - source-top-level-options
  - parse-test-0247
  example: qwen -o stream-json "run task"
  flag: --output-format
  invocation_scope:
  - applies_to: command
    command: []
  notes: The short flag accepts -o json and -o=json but not -ostream-json, which is cluster-parsed and rejected; json buffers while stream-json emits line-delimited events.
  scope:
  - io
  value: text | json | stream-json
  value_optional: false
  value_type: string
- attachment: []
  default: 'false'
  description: Include partial assistant messages in stream-json output.
  evidence_ids:
  - source-top-level-options
  - source-config-parser
  example: qwen -o stream-json --include-partial-messages "write"
  flag: --include-partial-messages
  invocation_scope:
  - applies_to: command
    command: []
  notes: Requires --output-format stream-json (checked at parse time).
  scope:
  - io
  value: none
  value_type: none
- attachment:
  - space
  - equals
  description: File descriptor for structured JSON event output in dual output mode, with the TUI rendering normally on stdout.
  evidence_ids:
  - source-top-level-options
  - parse-test-0247
  example: qwen --json-fd 3 "run" (with fd 3 configured in the spawn call)
  flag: --json-fd
  invocation_scope:
  - applies_to: command
    command: []
  notes: Mutually exclusive with --json-file; yargs number coercion is lenient, and a non-numeric value parses as NaN rather than erroring (observed); the caller must configure spawn stdio.
  scope:
  - dual_output
  value: <FD>
  value_optional: false
  value_type: number
- attachment:
  - space
  - equals
  description: File, FIFO, or /dev/fd/N path for structured JSON event output in dual output mode.
  evidence_ids:
  - source-top-level-options
  - source-config-parser
  example: qwen --json-file ./events.jsonl "run"
  flag: --json-file
  invocation_scope:
  - applies_to: command
    command: []
  notes: Mutually exclusive with --json-fd.
  scope:
  - dual_output
  value: <PATH>
  value_optional: false
  value_type: string
- attachment:
  - space
  - equals
  description: JSON Schema the final headless output must conform to; registers a synthetic structured_output tool and ends the session on the first valid call.
  evidence_ids:
  - source-top-level-options
  - source-config-parser
  example: qwen "summarize" --json-schema @./schema.json
  flag: --json-schema
  invocation_scope:
  - applies_to: command
    command: []
  notes: Rejected with --prompt-interactive, --input-format stream-json, --acp, or an invocation with no prompt, positional, or piped stdin.
  scope:
  - structured_output
  value: <JSON_OR_@PATH>
  value_optional: false
  value_type: string
- attachment:
  - space
  - equals
  description: File path for receiving remote JSONL input commands for bidirectional sync; the TUI watches and processes them.
  evidence_ids:
  - source-top-level-options
  - help-0247-root
  example: qwen --input-file ./commands.jsonl
  flag: --input-file
  invocation_scope:
  - applies_to: command
    command: []
  notes: Companion of the dual-output mode.
  scope:
  - dual_output
  value: <PATH>
  value_optional: false
  value_type: string
- aliases:
  - -c
  attachment: []
  default: 'false'
  description: Resume the most recent session for the current project.
  evidence_ids:
  - source-top-level-options
  - source-config-parser
  example: qwen --continue "next"
  flag: --continue
  invocation_scope:
  - applies_to: command
    command: []
  notes: Rejected together with --resume or --session-id.
  scope:
  - sessions
  value: none
  value_type: none
- aliases:
  - -r
  attachment:
  - space
  - equals
  description: Resume a specific session by ID, or show the session picker when used without an ID.
  evidence_ids:
  - source-top-level-options
  - source-config-parser
  example: qwen --resume 123e4567-e89b-12d3-a456-426614174000 "continue"
  flag: --resume
  invocation_scope:
  - applies_to: command
    command: []
  notes: The value may be omitted, but the picker is interactive, so wrappers should always pass an ID; rejected together with --continue or --session-id; accepts a UUID or a custom session title.
  scope:
  - sessions
  value: <SESSION_ID>
  value_optional: true
  value_type: string
- attachment:
  - space
  - equals
  description: Session ID for a new run; validated as a UUID at parse time.
  evidence_ids:
  - source-top-level-options
  - source-config-parser
  example: qwen --session-id 123e4567-e89b-12d3-a456-426614174000 "run"
  flag: --session-id
  invocation_scope:
  - applies_to: command
    command: []
  notes: Rejected together with --continue or --resume.
  scope:
  - sessions
  value: <UUID>
  value_optional: false
  value_type: string
- attachment: []
  default: 'false'
  description: Create a new forked session from the resumed session.
  evidence_ids:
  - source-top-level-options
  - source-config-parser
  example: qwen --continue --fork-session "try alternate fix"
  flag: --fork-session
  invocation_scope:
  - applies_to: command
    command: []
  notes: Requires --resume or --continue (checked at parse time).
  scope:
  - sessions
  value: none
  value_type: none
- attachment:
  - space
  - equals
  description: Hidden internal session ID used inside sandbox runs.
  evidence_ids:
  - source-config-parser
  example: qwen --sandbox-session-id 123e4567-e89b-12d3-a456-426614174000 "run"
  flag: --sandbox-session-id
  invocation_scope:
  - applies_to: command
    command: []
  notes: Hidden flag; UUID-validated; rejected with --session-id, --continue, or --resume.
  scope:
  - sessions
  value: <UUID>
  value_optional: false
  value_type: string
- attachment:
  - space
  - equals
  description: Start the session inside a git worktree at <repoRoot>/.qwen/worktrees/<slug>/; bare use auto-generates a slug.
  evidence_ids:
  - source-top-level-options
  - help-0247-root
  example: qwen --worktree my-feature "implement"
  flag: --worktree
  invocation_scope:
  - applies_to: command
    command: []
  notes: 'Accepts a slug, a bare flag, #123, or a GitHub pull-request URL, so the value may be omitted; the exit dialog prompts to keep or remove the worktree, a hazard for unattended runs.'
  scope:
  - git
  value: '[SLUG_OR_PR]'
  value_optional: true
  value_type: string
- attachment:
  - space
  - equals
  description: Maximum number of session turns; must be an integer.
  evidence_ids:
  - source-top-level-options
  - help-0247-root
  example: qwen --max-session-turns 8 "run bounded task"
  flag: --max-session-turns
  invocation_scope:
  - applies_to: command
    command: []
  notes: Applies to the structured-output terminal turn as well.
  scope:
  - limits
  value: <N>
  value_optional: false
  value_type: number
- attachment:
  - space
  - equals
  description: Run-level wall-clock budget for headless or unattended runs; aborts with exit code 55 when exceeded.
  evidence_ids:
  - source-top-level-options
  - help-0247-root
  example: qwen --max-wall-time 10m "run bounded task"
  flag: --max-wall-time
  invocation_scope:
  - applies_to: command
    command: []
  notes: 'Typed string, not number: accepts seconds (90) or duration strings (30s, 5m, 1h, 1.5h); sub-second values are rejected as typos.'
  scope:
  - limits
  value: <DURATION>
  value_optional: false
  value_type: string
- attachment:
  - space
  - equals
  default: '-1'
  description: Maximum cumulative tool calls; aborts with exit code 55 when exceeded.
  evidence_ids:
  - source-top-level-options
  - help-0247-root
  example: qwen --max-tool-calls 20 "inspect"
  flag: --max-tool-calls
  invocation_scope:
  - applies_to: command
    command: []
  notes: -1 or unset means unlimited; 0 forbids tool calls; capped at 1,000,000 to catch typos.
  scope:
  - limits
  value: <N>
  value_optional: false
  value_type: number
- attachment:
  - space
  - equals
  default: '5'
  description: Maximum sub-agent nesting depth, 1-based; 1 keeps sub-agents available but disables nesting.
  evidence_ids:
  - source-top-level-options
  - help-0247-root
  example: qwen --max-subagent-depth 1 "do not nest subagents"
  flag: --max-subagent-depth
  invocation_scope:
  - applies_to: command
    command: []
  notes: Capped at 100; overrides model.maxSubagentDepth.
  scope:
  - limits
  value: <N>
  value_optional: false
  value_type: number
- attachment:
  - space
  - equals
  description: Restrict the registered core tools.
  evidence_ids:
  - source-config-parser
  - yargs-parser-test
  example: qwen --core-tools read_file,grep "inspect"
  flag: --core-tools
  gap: yargs array options accept a zero-value occurrence (parses to an empty list; parser test), so the fewest values is 0, below the contract floor of 1; reading the coerce consumers in source would settle whether an empty list is treated as unset.
  invocation_scope:
  - applies_to: command
    command: []
  notes: Whitelist semantics, not auto-approval; yargs array with comma-splitting coerce.
  scope:
  - tools
  value: <TOOL>[,<TOOL>...]
  value_type: variadic
  variadic_min: unknown
- attachment:
  - space
  - equals
  description: Tools to exclude from the default command's toolset.
  evidence_ids:
  - source-config-parser
  - yargs-parser-test
  example: qwen --exclude-tools shell,write_file "inspect only"
  flag: --exclude-tools
  gap: yargs array options accept a zero-value occurrence (parses to an empty list; parser test), so the fewest values is 0, below the contract floor of 1; reading the coerce consumers in source would settle whether an empty list is treated as unset.
  invocation_scope:
  - applies_to: command
    command: []
  notes: A distinct mcp add --exclude-tools exists for MCP server tools; yargs array with comma-splitting coerce.
  scope:
  - tools
  value: <TOOL>[,<TOOL>...]
  value_type: variadic
  variadic_min: unknown
- attachment:
  - space
  - equals
  description: Slash command names to hide or disable, comma-separated or repeated.
  evidence_ids:
  - source-config-parser
  - yargs-parser-test
  - docs-settings-page
  example: qwen --disabled-slash-commands auth,mcp "work"
  flag: --disabled-slash-commands
  gap: yargs array options accept a zero-value occurrence (parses to an empty list; parser test), so the fewest values is 0, below the contract floor of 1; reading the coerce consumers in source would settle whether an empty list is treated as unset.
  invocation_scope:
  - applies_to: command
    command: []
  notes: Merged with slashCommands.disabled and QWEN_DISABLED_SLASH_COMMANDS; matching is case-insensitive.
  scope:
  - slash_commands
  value: <NAME>[,<NAME>...]
  value_type: variadic
  variadic_min: unknown
- attachment:
  - space
  - equals
  description: Authentication type and provider protocol.
  evidence_ids:
  - source-top-level-options
  - docs-quickstart
  example: qwen --auth-type openai --openai-api-key "$KEY" --openai-base-url "$URL" --model qwen3-coder-plus "run"
  flag: --auth-type
  invocation_scope:
  - applies_to: command
    command: []
  notes: openai-responses is new since the 0.19.x line; qwen-oauth is a legacy choice because Qwen OAuth was discontinued on 2026-04-15; endpoint semantics belong to model-config.
  scope:
  - auth
  value: openai | openai-responses | anthropic | qwen-oauth | gemini | vertex-ai
  value_optional: false
  value_type: string
- attachment: []
  default: 'false'
  description: Verify the sandbox kernel boundary instead of only reporting the policy.
  evidence_ids:
  - source-sandbox-cmd
  - help-0247-subcommands
  example: qwen sandbox --verify
  flag: --verify
  invocation_scope:
  - applies_to: command
    command:
    - sandbox
  notes: Scoped to the sandbox subcommand.
  scope:
  - sandbox
  value: none
  value_type: none
- attachment:
  - space
  - equals
  default: '4170'
  description: TCP port to bind for the daemon; 0 requests an OS-assigned ephemeral port.
  evidence_ids:
  - source-serve-cmd
  - help-0247-subcommands
  example: qwen serve --port 4170
  flag: --port
  invocation_scope:
  - applies_to: command
    command:
    - serve
  scope:
  - serve
  value: <PORT>
  value_optional: false
  value_type: number
- attachment:
  - space
  - equals
  default: 127.0.0.1
  description: Interface to bind; loopback is auth-free and anything non-loopback requires a token.
  evidence_ids:
  - source-serve-cmd
  - help-0247-subcommands
  example: qwen serve --hostname 127.0.0.1
  flag: --hostname
  invocation_scope:
  - applies_to: command
    command:
    - serve
  notes: An empty value is rejected as operator error.
  scope:
  - serve
  value: <HOST>
  value_optional: false
  value_type: string
- attachment:
  - space
  - equals
  default: default
  description: Deployment profile; hosted-harness enables a private no-tool Managed Session API on loopback.
  evidence_ids:
  - source-serve-cmd
  - help-0247-subcommands
  example: qwen serve --profile hosted-harness
  flag: --profile
  invocation_scope:
  - applies_to: command
    command:
    - serve
  scope:
  - serve
  value: default | hosted-harness
  value_optional: false
  value_type: string
- attachment:
  - space
  - equals
  description: SHA-256 capability digest for the hosted-harness profile; falls back to QWEN_HOSTED_HARNESS_CAPABILITY_DIGEST.
  evidence_ids:
  - source-serve-cmd
  - help-0247-subcommands
  example: qwen serve --profile hosted-harness --hosted-harness-capability-digest <digest>
  flag: --hosted-harness-capability-digest
  invocation_scope:
  - applies_to: command
    command:
    - serve
  scope:
  - serve
  value: <SHA256>
  value_optional: false
  value_type: string
- attachment:
  - space
  - equals
  description: Bearer token required on every request; falls back to QWEN_SERVER_TOKEN, and a non-loopback bind with neither generates an ephemeral token.
  evidence_ids:
  - source-serve-cmd
  - help-0247-subcommands
  example: qwen serve --token "$QWEN_SERVER_TOKEN"
  flag: --token
  invocation_scope:
  - applies_to: command
    command:
    - serve
  notes: Visible in process argv; prefer the env var on shared hosts.
  scope:
  - serve
  value: <TOKEN>
  value_optional: false
  value_type: string
- attachment:
  - space
  - equals
  default: '32'
  description: Cap on concurrent live sessions; excess spawn requests return 503.
  evidence_ids:
  - source-serve-cmd
  - help-0247-subcommands
  example: qwen serve --max-sessions 64
  flag: --max-sessions
  invocation_scope:
  - applies_to: command
    command:
    - serve
  notes: 0 disables the cap.
  scope:
  - serve
  value: <N>
  value_optional: false
  value_type: number
- attachment:
  - space
  - equals
  default: '0'
  description: Non-negative cap on concurrent live sessions across all workspace runtimes.
  evidence_ids:
  - source-serve-cmd
  - help-0247-subcommands
  example: qwen serve --max-total-sessions 128
  flag: --max-total-sessions
  invocation_scope:
  - applies_to: command
    command:
    - serve
  notes: 0 disables the cap.
  scope:
  - serve
  value: <N>
  value_optional: false
  value_type: number
- attachment:
  - space
  - equals
  default: '5'
  description: Per-session cap on accepted prompts waiting or running; excess prompts return 503.
  evidence_ids:
  - source-serve-cmd
  - help-0247-subcommands
  example: qwen serve --max-pending-prompts-per-session 10
  flag: --max-pending-prompts-per-session
  invocation_scope:
  - applies_to: command
    command:
    - serve
  notes: 0 disables the cap.
  scope:
  - serve
  value: <N>
  value_optional: false
  value_type: number
- attachment:
  - space
  - equals
  description: Absolute workspace path to register; repeat to register isolated workspace runtimes, the first is primary.
  evidence_ids:
  - source-serve-cmd
  - help-0247-subcommands
  - yargs-parser-test
  example: qwen serve --workspace /repo
  flag: --workspace
  gap: yargs array options accept a zero-value occurrence (parses to an empty list; parser test), so the fewest values is 0, below the contract floor of 1; reading the serve builder consumers in source would settle the effective minimum.
  invocation_scope:
  - applies_to: command
    command:
    - serve
  notes: Mismatched POST /session cwd returns 400 workspace_mismatch; defaults to process.cwd(); yargs array.
  scope:
  - serve
  value: <PATH>
  value_type: variadic
  variadic_min: unknown
- attachment:
  - space
  - equals
  default: workspace
  description: How project memory is partitioned across daemon workspaces; git-root preserves the legacy shared scope.
  evidence_ids:
  - source-serve-cmd
  - help-0247-subcommands
  example: qwen serve --memory-project-scope git-root
  flag: --memory-project-scope
  invocation_scope:
  - applies_to: command
    command:
    - serve
  notes: Overrides QWEN_CODE_MEMORY_PROJECT_SCOPE when provided.
  scope:
  - serve
  value: git-root | workspace
  value_optional: false
  value_type: string
- attachment:
  - space
  - equals
  default: '256'
  description: Listener-level TCP connection cap; slow SSE clients are rejected at accept time once full.
  evidence_ids:
  - source-serve-cmd
  - help-0247-subcommands
  example: qwen serve --max-connections 512
  flag: --max-connections
  invocation_scope:
  - applies_to: command
    command:
    - serve
  notes: 0 disables the cap.
  scope:
  - serve
  value: <N>
  value_optional: false
  value_type: number
- attachment: []
  default: 'false'
  description: Refuse to start without a bearer token, even on loopback.
  evidence_ids:
  - source-serve-cmd
  - help-0247-subcommands
  example: qwen serve --require-auth --token "$QWEN_SERVER_TOKEN"
  flag: --require-auth
  invocation_scope:
  - applies_to: command
    command:
    - serve
  notes: With it enabled, /health also requires Authorization.
  scope:
  - serve
  value: none
  value_type: none
- attachment: []
  default: 'false'
  description: Enable direct POST /session/:id/shell execution.
  evidence_ids:
  - source-serve-cmd
  - help-0247-subcommands
  example: qwen serve --enable-session-shell
  flag: --enable-session-shell
  invocation_scope:
  - applies_to: command
    command:
    - serve
  notes: Each call still requires a session-bound client id.
  scope:
  - serve
  value: none
  value_type: none
- attachment:
  - space
  - equals
  description: Path to a PEM certificate file to serve HTTPS instead of HTTP.
  evidence_ids:
  - source-serve-cmd
  - help-0247-subcommands
  example: qwen serve --tls-cert cert.pem --tls-key key.pem
  flag: --tls-cert
  invocation_scope:
  - applies_to: command
    command:
    - serve
  notes: Must be used together with --tls-key.
  scope:
  - serve
  value: <PATH>
  value_optional: false
  value_type: string
- attachment:
  - space
  - equals
  description: Path to a PEM private key; must be used together with --tls-cert.
  evidence_ids:
  - source-serve-cmd
  - help-0247-subcommands
  example: qwen serve --tls-cert cert.pem --tls-key key.pem
  flag: --tls-key
  invocation_scope:
  - applies_to: command
    command:
    - serve
  scope:
  - serve
  value: <PATH>
  value_optional: false
  value_type: string
- attachment:
  - space
  - equals
  description: Serve-scoped repeatable channel selector that starts daemon-managed channel workers; --channel all selects every channel.
  evidence_ids:
  - source-serve-cmd
  - help-0247-subcommands
  example: qwen serve --channel telegram --channel discord
  flag: --channel
  gap: yargs array options accept a zero-value occurrence (parses to an empty list; parser test), so the fewest values is 0, below the contract floor of 1; reading the serve builder consumers in source would settle the effective minimum.
  invocation_scope:
  - applies_to: command
    command:
    - serve
  notes: Different switch from the root --channel (string choices); same spelling, serve-scoped array.
  scope:
  - serve
  - integration
  value: <NAME>
  value_type: variadic
  variadic_min: unknown
- attachment: []
  default: 'true'
  description: Serve the Web Shell UI at the daemon root; --no-web gives an API-only daemon.
  evidence_ids:
  - source-serve-cmd
  - help-0247-subcommands
  example: qwen serve --no-web
  flag: --web
  invocation_scope:
  - applies_to: command
    command:
    - serve
  notes: yargs boolean; the negated spelling is the documented way to disable.
  scope:
  - serve
  value: none
  value_type: none
- attachment: []
  default: 'false'
  description: Open the Web Shell in a browser once the daemon is listening.
  evidence_ids:
  - source-serve-cmd
  - help-0247-subcommands
  example: qwen serve --open
  flag: --open
  invocation_scope:
  - applies_to: command
    command:
    - serve
  notes: With a token configured the launch URL includes the token and is visible in the process list; no-op in headless environments.
  scope:
  - serve
  value: none
  value_type: none
- attachment: []
  default: 'false'
  description: Open the Web Shell with bearer authentication on loopback, delivering the token in the URL fragment.
  evidence_ids:
  - source-serve-cmd
  - help-0247-subcommands
  example: qwen serve --open-with-auth
  flag: --open-with-auth
  invocation_scope:
  - applies_to: command
    command:
    - serve
  notes: In headless environments prints the fragment URL for manual opening.
  scope:
  - serve
  value: none
  value_type: none
- attachment: []
  description: Print the token-bearing QR even when stdout is captured; --no-token-qr suppresses the startup QR for that run.
  evidence_ids:
  - source-serve-cmd
  - help-0247-subcommands
  example: qwen serve --token-qr
  flag: --token-qr
  invocation_scope:
  - applies_to: command
    command:
    - serve
  notes: Also settable via serve.tokenQr; does not govern the Local Control pairing QR.
  scope:
  - serve
  value: none
  value_type: none
- attachment: []
  default: 'false'
  description: Share the Web Shell on the local IPv4 network with its own revocable pairing token and terminal QR code.
  evidence_ids:
  - source-serve-cmd
  - help-0247-subcommands
  example: qwen serve --local-control
  flag: --local-control
  invocation_scope:
  - applies_to: command
    command:
    - serve
  notes: Ctrl+C ends the whole daemon; the Web Shell settings card can turn it off while the daemon keeps running.
  scope:
  - serve
  value: none
  value_type: none
- attachment:
  - space
  - equals
  description: Which local IPv4 address to share when the host is on more than one network.
  evidence_ids:
  - source-serve-cmd
  - help-0247-subcommands
  example: qwen serve --local-control --local-control-address 192.168.1.20
  flag: --local-control-address
  invocation_scope:
  - applies_to: command
    command:
    - serve
  notes: Only needed when --local-control reports an ambiguous choice.
  scope:
  - serve
  value: <IP>
  value_optional: false
  value_type: string
- attachment:
  - space
  - equals
  default: '8000'
  description: Per-session SSE replay ring depth for reconnects that send Last-Event-ID.
  evidence_ids:
  - source-serve-cmd
  - help-0247-subcommands
  example: qwen serve --event-ring-size 16000
  flag: --event-ring-size
  invocation_scope:
  - applies_to: command
    command:
    - serve
  notes: Must be a positive finite integer.
  scope:
  - serve
  value: <N>
  value_optional: false
  value_type: number
- attachment:
  - space
  - equals
  default: '4194304'
  description: Per-session in-memory compacted replay snapshot byte cap for late attaches.
  evidence_ids:
  - source-serve-cmd
  - help-0247-subcommands
  example: qwen serve --compacted-replay-max-bytes 8388608
  flag: --compacted-replay-max-bytes
  invocation_scope:
  - applies_to: command
    command:
    - serve
  notes: Positive safe integer no larger than 256 MiB.
  scope:
  - serve
  value: <N>
  value_optional: false
  value_type: number
- attachment:
  - space
  - equals
  default: '10000'
  description: Per-session baseline cap on replay entries retained in the in-flight live journal.
  evidence_ids:
  - source-serve-cmd
  - help-0247-subcommands
  example: qwen serve --max-journal-events 20000
  flag: --max-journal-events
  invocation_scope:
  - applies_to: command
    command:
    - serve
  notes: Pinning this or --max-journal-bytes disables adaptive growth.
  scope:
  - serve
  value: <N>
  value_optional: false
  value_type: number
- attachment:
  - space
  - equals
  default: '8388608'
  description: Per-session baseline source-event byte cap on the in-flight live journal.
  evidence_ids:
  - source-serve-cmd
  - help-0247-subcommands
  example: qwen serve --max-journal-bytes 16777216
  flag: --max-journal-bytes
  invocation_scope:
  - applies_to: command
    command:
    - serve
  notes: Pinning this or --max-journal-events disables adaptive growth.
  scope:
  - serve
  value: <N>
  value_optional: false
  value_type: number
- attachment: []
  default: 'true'
  description: HTTP bridge mode that preheats the primary qwen --acp child.
  evidence_ids:
  - source-serve-cmd
  - help-0247-subcommands
  example: qwen serve --http-bridge
  flag: --http-bridge
  invocation_scope:
  - applies_to: command
    command:
    - serve
  notes: Stage 2 native in-process mode is not implemented; --no-http-bridge is not honored and falls back to bridge mode.
  scope:
  - serve
  value: none
  value_type: none
- attachment:
  - space
  - equals
  description: Total memory budget in MB for the daemon process tree; unset derives 50 percent of constrained or host memory.
  evidence_ids:
  - source-serve-cmd
  - help-0247-subcommands
  example: qwen serve --memory-budget-mb 8192
  flag: --memory-budget-mb
  invocation_scope:
  - applies_to: command
    command:
    - serve
  notes: Integer in [1024, 1048576]; reported under limits.memory in daemon status.
  scope:
  - serve
  value: <N>
  value_optional: false
  value_type: number
- attachment:
  - space
  - equals
  default: observe
  description: Whether the daemon derives and reports a memory-pressure level from its own RSS and V8 heap.
  evidence_ids:
  - source-serve-cmd
  - help-0247-subcommands
  example: qwen serve --memory-pressure-mode off
  flag: --memory-pressure-mode
  invocation_scope:
  - applies_to: command
    command:
    - serve
  notes: Nothing remediates in either mode.
  scope:
  - serve
  value: off | observe
  value_optional: false
  value_type: string
- attachment:
  - space
  - equals
  default: observe
  description: Whether the daemon models a per-child heap partition of the memory budget.
  evidence_ids:
  - source-serve-cmd
  - help-0247-subcommands
  example: qwen serve --child-heap-mode enforce
  flag: --child-heap-mode
  invocation_scope:
  - applies_to: command
    command:
    - serve
  notes: admit rejects starts past the modeled process limit; enforce additionally applies the modeled old-space ceiling.
  scope:
  - serve
  value: off | observe | admit | enforce
  value_optional: false
  value_type: string
- attachment:
  - space
  - equals
  description: Cap on live MCP clients spawned inside the ACP child for the bound workspace.
  evidence_ids:
  - source-serve-cmd
  - help-0247-subcommands
  example: qwen serve --mcp-client-budget 16
  flag: --mcp-client-budget
  invocation_scope:
  - applies_to: command
    command:
    - serve
  notes: Combine with --mcp-budget-mode; distinct from MCP_SERVER_CONNECTION_BATCH_SIZE.
  scope:
  - serve
  - mcp
  value: <N>
  value_optional: false
  value_type: number
- attachment:
  - space
  - equals
  default: warn
  description: How --mcp-client-budget is enforced at the cap.
  evidence_ids:
  - source-serve-cmd
  - help-0247-subcommands
  example: qwen serve --mcp-client-budget 16 --mcp-budget-mode enforce
  flag: --mcp-budget-mode
  invocation_scope:
  - applies_to: command
    command:
    - serve
  notes: Boot rejects enforce without a budget.
  scope:
  - serve
  - mcp
  value: enforce | warn | off
  value_optional: false
  value_type: string
- attachment:
  - space
  - equals
  description: Cross-origin allowlist for browser clients.
  evidence_ids:
  - source-serve-cmd
  - help-0247-subcommands
  - yargs-parser-test
  example: qwen serve --allow-origin https://app.example.com
  flag: --allow-origin
  gap: yargs array options accept a zero-value occurrence (parses to an empty list; parser test), so the fewest values is 0, below the contract floor of 1; reading the serve builder consumers in source would settle the effective minimum.
  invocation_scope:
  - applies_to: command
    command:
    - serve
  notes: yargs array.
  scope:
  - serve
  value: <ORIGIN>
  value_type: variadic
  variadic_min: unknown
- attachment: []
  default: 'false'
  description: Allow /workspace/auth/provider to install localhost or private-network baseUrl values.
  evidence_ids:
  - source-serve-cmd
  - help-0247-subcommands
  example: qwen serve --allow-private-auth-base-url
  flag: --allow-private-auth-base-url
  invocation_scope:
  - applies_to: command
    command:
    - serve
  notes: Local development with trusted clients only.
  scope:
  - serve
  value: none
  value_type: none
- attachment:
  - space
  - equals
  description: Server-side wallclock cap on POST /session/:id/prompt in milliseconds.
  evidence_ids:
  - source-serve-cmd
  - help-0247-subcommands
  example: qwen serve --prompt-deadline-ms 600000
  flag: --prompt-deadline-ms
  invocation_scope:
  - applies_to: command
    command:
    - serve
  notes: Falls back to QWEN_SERVE_PROMPT_DEADLINE_MS.
  scope:
  - serve
  value: <MS>
  value_optional: false
  value_type: number
- attachment: []
  default: 'false'
  description: 'Experimental: build workspace runtimes with paired Legacy and Managed engines.'
  evidence_ids:
  - source-serve-cmd
  - help-0247-subcommands
  example: qwen serve --experimental-paired-engines
  flag: --experimental-paired-engines
  invocation_scope:
  - applies_to: command
    command:
    - serve
  notes: No Managed engine is available yet.
  scope:
  - serve
  value: none
  value_type: none
- attachment: []
  default: 'false'
  description: Reserved experimental mode; not implemented and rejects startup.
  evidence_ids:
  - source-serve-cmd
  - help-0247-subcommands
  example: qwen serve --experimental-managed-agents
  flag: --experimental-managed-agents
  invocation_scope:
  - applies_to: command
    command:
    - serve
  notes: Present in the 0.24.7 parser and help.
  scope:
  - serve
  value: none
  value_type: none
- attachment: []
  default: 'false'
  description: Reserved experimental mode; not implemented and rejects startup.
  evidence_ids:
  - source-serve-cmd
  - help-0247-subcommands
  example: qwen serve --experimental-managed-runtime-worker
  flag: --experimental-managed-runtime-worker
  invocation_scope:
  - applies_to: command
    command:
    - serve
  notes: Present in the 0.24.7 parser and help.
  scope:
  - serve
  value: none
  value_type: none
- attachment: []
  default: 'false'
  description: Reserved experimental mode; not implemented and rejects startup.
  evidence_ids:
  - source-serve-cmd
  - help-0247-subcommands
  example: qwen serve --experimental-managed-runtime-auto-local
  flag: --experimental-managed-runtime-auto-local
  invocation_scope:
  - applies_to: command
    command:
    - serve
  notes: Present in the 0.24.7 parser and help.
  scope:
  - serve
  value: none
  value_type: none
- attachment:
  - space
  - equals
  description: Reserved experimental Runtime URL; not implemented and rejects startup.
  evidence_ids:
  - source-serve-cmd
  - help-0247-subcommands
  example: qwen serve --experimental-managed-runtime-url https://runtime.example.com
  flag: --experimental-managed-runtime-url
  invocation_scope:
  - applies_to: command
    command:
    - serve
  notes: Present in the 0.24.7 parser and help.
  scope:
  - serve
  value: <URL>
  value_optional: false
  value_type: string
- attachment:
  - space
  - equals
  description: Reserved experimental Runtime credential; not implemented and rejects startup.
  evidence_ids:
  - source-serve-cmd
  - help-0247-subcommands
  example: qwen serve --experimental-managed-runtime-token <token>
  flag: --experimental-managed-runtime-token
  invocation_scope:
  - applies_to: command
    command:
    - serve
  notes: Present in the 0.24.7 parser and help.
  scope:
  - serve
  value: <TOKEN>
  value_optional: false
  value_type: string
- attachment:
  - space
  - equals
  description: Reserved Broker URL for the hosted-harness profile; not implemented and rejects startup.
  evidence_ids:
  - source-serve-cmd
  - help-0247-subcommands
  example: qwen serve --profile hosted-harness --managed-runtime-broker-url https://broker.example.com
  flag: --managed-runtime-broker-url
  invocation_scope:
  - applies_to: command
    command:
    - serve
  notes: Present in the 0.24.7 parser and help.
  scope:
  - serve
  value: <URL>
  value_optional: false
  value_type: string
- attachment:
  - space
  - equals
  description: Reserved Broker credential for the hosted-harness profile; not implemented and rejects startup.
  evidence_ids:
  - source-serve-cmd
  - help-0247-subcommands
  example: qwen serve --profile hosted-harness --managed-runtime-broker-token <token>
  flag: --managed-runtime-broker-token
  invocation_scope:
  - applies_to: command
    command:
    - serve
  notes: Present in the 0.24.7 parser and help.
  scope:
  - serve
  value: <TOKEN>
  value_optional: false
  value_type: string
- attachment:
  - space
  - equals
  description: Per-SSE-connection idle deadline in milliseconds.
  evidence_ids:
  - source-serve-cmd
  - help-0247-subcommands
  example: qwen serve --writer-idle-timeout-ms 120000
  flag: --writer-idle-timeout-ms
  invocation_scope:
  - applies_to: command
    command:
    - serve
  notes: Falls back to QWEN_SERVE_WRITER_IDLE_TIMEOUT_MS.
  scope:
  - serve
  value: <MS>
  value_optional: false
  value_type: number
- attachment:
  - space
  - equals
  description: Compatibility auto-reap delay for an idle workspace ACP child.
  evidence_ids:
  - source-serve-cmd
  - help-0247-subcommands
  example: qwen serve --channel-idle-timeout-ms 30000
  flag: --channel-idle-timeout-ms
  invocation_scope:
  - applies_to: command
    command:
    - serve
  notes: 0 or unset reaps after work drains.
  scope:
  - serve
  value: <MS>
  value_optional: false
  value_type: number
- attachment:
  - space
  - equals
  default: '10000'
  description: ACP child request timeout, including the initialize handshake.
  evidence_ids:
  - source-serve-cmd
  - help-0247-subcommands
  example: qwen serve --initialize-timeout-ms 20000
  flag: --initialize-timeout-ms
  invocation_scope:
  - applies_to: command
    command:
    - serve
  scope:
  - serve
  value: <MS>
  value_optional: false
  value_type: number
- attachment:
  - space
  - equals
  default: '60000'
  description: ACP session load or resume timeout.
  evidence_ids:
  - source-serve-cmd
  - help-0247-subcommands
  example: qwen serve --session-restore-timeout-ms 120000
  flag: --session-restore-timeout-ms
  invocation_scope:
  - applies_to: command
    command:
    - serve
  notes: An explicit --initialize-timeout-ms can raise but never lower this default.
  scope:
  - serve
  value: <MS>
  value_optional: false
  value_type: number
- attachment:
  - space
  - equals
  default: '60000'
  description: Session reaper scan interval; 0 disables.
  evidence_ids:
  - source-serve-cmd
  - help-0247-subcommands
  example: qwen serve --session-reap-interval-ms 30000
  flag: --session-reap-interval-ms
  invocation_scope:
  - applies_to: command
    command:
    - serve
  scope:
  - serve
  value: <MS>
  value_optional: false
  value_type: number
- attachment:
  - space
  - equals
  default: '1800000'
  description: Idle timeout before a disconnected session is reaped; 0 disables.
  evidence_ids:
  - source-serve-cmd
  - help-0247-subcommands
  example: qwen serve --session-idle-timeout-ms 600000
  flag: --session-idle-timeout-ms
  invocation_scope:
  - applies_to: command
    command:
    - serve
  scope:
  - serve
  value: <MS>
  value_optional: false
  value_type: number
- attachment:
  - space
  - equals
  default: '0'
  description: Grace period after a prompt settles before an otherwise-idle session may be auto-closed.
  evidence_ids:
  - source-serve-cmd
  - help-0247-subcommands
  example: qwen serve --session-prompt-settled-close-grace-ms 5000
  flag: --session-prompt-settled-close-grace-ms
  invocation_scope:
  - applies_to: command
    command:
    - serve
  notes: 0 disables the grace period (immediate close).
  scope:
  - serve
  value: <MS>
  value_optional: false
  value_type: number
- attachment:
  - space
  - equals
  default: '0'
  description: Wall-clock timeout for a human permission or ask_user_question response in daemon mode.
  evidence_ids:
  - source-serve-cmd
  - help-0247-subcommands
  example: qwen serve --permission-response-timeout-ms 300000
  flag: --permission-response-timeout-ms
  invocation_scope:
  - applies_to: command
    command:
    - serve
  notes: 0 or unset waits indefinitely.
  scope:
  - serve
  - permissions
  value: <MS>
  value_optional: false
  value_type: number
- attachment:
  - space
  - equals
  default: off
  description: Managed ACP pre-execution policy mode; required fails startup unless a compatible loopback provider is available.
  evidence_ids:
  - source-serve-cmd
  - help-0247-subcommands
  example: qwen serve --external-tool-guard-mode required
  flag: --external-tool-guard-mode
  invocation_scope:
  - applies_to: command
    command:
    - serve
  scope:
  - serve
  - permissions
  value: off | required
  value_optional: false
  value_type: string
- attachment:
  - space
  - equals
  description: Origin-only loopback HTTP(S) endpoint for required external tool guarding.
  evidence_ids:
  - source-serve-cmd
  - help-0247-subcommands
  example: qwen serve --external-tool-guard-mode required --external-tool-guard-endpoint http://127.0.0.1:8787
  flag: --external-tool-guard-endpoint
  invocation_scope:
  - applies_to: command
    command:
    - serve
  scope:
  - serve
  - permissions
  value: <URL>
  value_optional: false
  value_type: string
- attachment:
  - space
  - equals
  default: '3000'
  description: Per-handshake or prepare external tool guard timeout in milliseconds.
  evidence_ids:
  - source-serve-cmd
  - help-0247-subcommands
  example: qwen serve --external-tool-guard-timeout-ms 5000
  flag: --external-tool-guard-timeout-ms
  invocation_scope:
  - applies_to: command
    command:
    - serve
  scope:
  - serve
  - permissions
  value: <MS>
  value_optional: false
  value_type: number
- attachment: []
  description: Enable per-tier HTTP rate limiting for prompt, mutation, and read requests.
  evidence_ids:
  - source-serve-cmd
  - help-0247-subcommands
  example: qwen serve --rate-limit
  flag: --rate-limit
  invocation_scope:
  - applies_to: command
    command:
    - serve
  notes: Health, heartbeat, SSE, and /acp are exempt; also enabled by QWEN_SERVE_RATE_LIMIT.
  scope:
  - serve
  value: none
  value_type: none
- attachment:
  - space
  - equals
  default: '10'
  description: Max prompt requests per window per client; requires --rate-limit.
  evidence_ids:
  - source-serve-cmd
  - help-0247-subcommands
  example: qwen serve --rate-limit --rate-limit-prompt 20
  flag: --rate-limit-prompt
  invocation_scope:
  - applies_to: command
    command:
    - serve
  scope:
  - serve
  value: <N>
  value_optional: false
  value_type: number
- attachment:
  - space
  - equals
  default: '30'
  description: Max mutation requests per window per client; requires --rate-limit.
  evidence_ids:
  - source-serve-cmd
  - help-0247-subcommands
  example: qwen serve --rate-limit --rate-limit-mutation 60
  flag: --rate-limit-mutation
  invocation_scope:
  - applies_to: command
    command:
    - serve
  scope:
  - serve
  value: <N>
  value_optional: false
  value_type: number
- attachment:
  - space
  - equals
  default: '120'
  description: Max read requests per window per client; requires --rate-limit.
  evidence_ids:
  - source-serve-cmd
  - help-0247-subcommands
  example: qwen serve --rate-limit --rate-limit-read 240
  flag: --rate-limit-read
  invocation_scope:
  - applies_to: command
    command:
    - serve
  scope:
  - serve
  value: <N>
  value_optional: false
  value_type: number
- attachment:
  - space
  - equals
  default: '60000'
  description: Rate limit window duration in milliseconds; requires --rate-limit.
  evidence_ids:
  - source-serve-cmd
  - help-0247-subcommands
  example: qwen serve --rate-limit --rate-limit-window-ms 30000
  flag: --rate-limit-window-ms
  invocation_scope:
  - applies_to: command
    command:
    - serve
  scope:
  - serve
  value: <MS>
  value_optional: false
  value_type: number
- aliases:
  - -s
  attachment:
  - space
  - equals
  default: user
  description: Configuration scope that decides which settings file an MCP change is written to.
  evidence_ids:
  - source-mcp-cmds
  example: qwen mcp add --scope project local python -m server
  flag: --scope
  invocation_scope:
  - applies_to: command
    command:
    - mcp
    - add
  - applies_to: command
    command:
    - mcp
    - remove
  notes: Choice-validated user or project; the extensions install spelling is a different switch.
  scope:
  - mcp
  value: user | project
  value_optional: false
  value_type: string
- aliases:
  - -t
  attachment:
  - space
  - equals
  description: MCP transport type; auto-detected from the URL when omitted.
  evidence_ids:
  - source-mcp-cmds
  example: qwen mcp add --transport http my-server http://localhost:3000/mcp
  flag: --transport
  invocation_scope:
  - applies_to: command
    command:
    - mcp
    - add
  scope:
  - mcp
  value: stdio | sse | http
  value_optional: false
  value_type: string
- aliases:
  - -e
  attachment:
  - space
  - equals
  description: Set one environment variable for the MCP server; repeat the flag for more.
  evidence_ids:
  - source-mcp-cmds
  - yargs-parser-test
  example: qwen mcp add -e TOKEN=abc -e HOST=local node server.js
  flag: --env
  invocation_scope:
  - applies_to: command
    command:
    - mcp
    - add
  notes: Declared as a yargs array with nargs:1, so each occurrence takes exactly one value.
  scope:
  - mcp
  value: KEY=value
  value_optional: false
  value_type: string
- aliases:
  - -H
  attachment:
  - space
  - equals
  description: Set one HTTP header for SSE and HTTP transports; repeat the flag for more.
  evidence_ids:
  - source-mcp-cmds
  - yargs-parser-test
  example: 'qwen mcp add -H "X-Api-Key: abc" --transport http remote http://localhost:3000/mcp'
  flag: --header
  invocation_scope:
  - applies_to: command
    command:
    - mcp
    - add
  notes: Declared as a yargs array with nargs:1, so each occurrence takes exactly one value.
  scope:
  - mcp
  value: 'NAME: value'
  value_optional: false
  value_type: string
- attachment:
  - space
  - equals
  description: MCP server connection timeout in milliseconds.
  evidence_ids:
  - source-mcp-cmds
  example: qwen mcp add --timeout 30000 local python -m server
  flag: --timeout
  invocation_scope:
  - applies_to: command
    command:
    - mcp
    - add
  scope:
  - mcp
  value: <MS>
  value_optional: false
  value_type: number
- attachment: []
  description: Trust the server, bypassing its tool call confirmations in a trusted workspace.
  evidence_ids:
  - source-mcp-cmds
  example: qwen mcp add --trust local python -m server
  flag: --trust
  invocation_scope:
  - applies_to: command
    command:
    - mcp
    - add
  notes: Mutates persistent trust state; permission semantics belong to agent-permissions.
  scope:
  - mcp
  - permissions
  value: none
  value_type: none
- attachment:
  - space
  - equals
  description: Set the description for the MCP server.
  evidence_ids:
  - source-mcp-cmds
  example: qwen mcp add --description "Local tools" local python -m server
  flag: --description
  invocation_scope:
  - applies_to: command
    command:
    - mcp
    - add
  scope:
  - mcp
  value: <TEXT>
  value_optional: false
  value_type: string
- attachment:
  - space
  - equals
  description: Comma-separated list of the MCP server's tools to include.
  evidence_ids:
  - source-mcp-cmds
  - yargs-parser-test
  example: qwen mcp add --include-tools search,fetch remote http://localhost:3000/mcp
  flag: --include-tools
  gap: yargs array options accept a zero-value occurrence (parses to an empty list; parser test), so the fewest values is 0, below the contract floor of 1; reading the add-command consumers in source would settle the effective minimum.
  invocation_scope:
  - applies_to: command
    command:
    - mcp
    - add
  notes: yargs array with comma splitting.
  scope:
  - mcp
  value: <TOOL>[,<TOOL>...]
  value_type: variadic
  variadic_min: unknown
- attachment:
  - space
  - equals
  description: Comma-separated list of the MCP server's tools to exclude.
  evidence_ids:
  - source-mcp-cmds
  - yargs-parser-test
  example: qwen mcp add --exclude-tools shell remote http://localhost:3000/mcp
  flag: --exclude-tools
  gap: yargs array options accept a zero-value occurrence (parses to an empty list; parser test), so the fewest values is 0, below the contract floor of 1; reading the add-command consumers in source would settle the effective minimum.
  invocation_scope:
  - applies_to: command
    command:
    - mcp
    - add
  notes: Distinct from the root default-command --exclude-tools, which excludes core tools.
  scope:
  - mcp
  value: <TOOL>[,<TOOL>...]
  value_type: variadic
  variadic_min: unknown
- attachment:
  - space
  - equals
  description: OAuth client ID for MCP server authentication.
  evidence_ids:
  - source-mcp-cmds
  example: qwen mcp add --transport http --oauth-client-id <id> remote http://localhost:3000/mcp
  flag: --oauth-client-id
  invocation_scope:
  - applies_to: command
    command:
    - mcp
    - add
  notes: Only meaningful for sse and http transports.
  scope:
  - mcp
  value: <ID>
  value_optional: false
  value_type: string
- attachment:
  - space
  - equals
  description: OAuth client secret for MCP server authentication.
  evidence_ids:
  - source-mcp-cmds
  example: qwen mcp add --transport http --oauth-client-secret <secret> remote http://localhost:3000/mcp
  flag: --oauth-client-secret
  invocation_scope:
  - applies_to: command
    command:
    - mcp
    - add
  notes: Visible in process argv.
  scope:
  - mcp
  value: <SECRET>
  value_optional: false
  value_type: string
- attachment:
  - space
  - equals
  description: OAuth redirect URI for MCP server authentication; defaults to localhost for local setups.
  evidence_ids:
  - source-mcp-cmds
  example: qwen mcp add --transport sse --oauth-redirect-uri https://example.com/oauth/callback remote https://example.com/sse
  flag: --oauth-redirect-uri
  invocation_scope:
  - applies_to: command
    command:
    - mcp
    - add
  scope:
  - mcp
  value: <URI>
  value_optional: false
  value_type: string
- attachment:
  - space
  - equals
  description: OAuth authorization URL for MCP server authentication.
  evidence_ids:
  - source-mcp-cmds
  example: qwen mcp add --transport http --oauth-authorization-url https://provider.example.com/authorize remote http://localhost:3000/mcp
  flag: --oauth-authorization-url
  invocation_scope:
  - applies_to: command
    command:
    - mcp
    - add
  scope:
  - mcp
  value: <URL>
  value_optional: false
  value_type: string
- attachment:
  - space
  - equals
  description: OAuth token URL for MCP server authentication.
  evidence_ids:
  - source-mcp-cmds
  example: qwen mcp add --transport http --oauth-token-url https://provider.example.com/token remote http://localhost:3000/mcp
  flag: --oauth-token-url
  invocation_scope:
  - applies_to: command
    command:
    - mcp
    - add
  scope:
  - mcp
  value: <URL>
  value_optional: false
  value_type: string
- attachment:
  - space
  - equals
  description: OAuth scopes for MCP server authentication, comma-separated.
  evidence_ids:
  - source-mcp-cmds
  - yargs-parser-test
  example: qwen mcp add --transport http --oauth-scopes scope1,scope2 remote http://localhost:3000/mcp
  flag: --oauth-scopes
  gap: yargs array options accept a zero-value occurrence (parses to an empty list; parser test), so the fewest values is 0, below the contract floor of 1; reading the add-command consumers in source would settle the effective minimum.
  invocation_scope:
  - applies_to: command
    command:
    - mcp
    - add
  notes: yargs array with comma splitting.
  scope:
  - mcp
  value: <SCOPE>[,<SCOPE>...]
  value_type: variadic
  variadic_min: unknown
- attachment: []
  default: 'false'
  description: Apply the operation to all matching items.
  evidence_ids:
  - source-mcp-cmds
  - source-extensions-cmds
  example: qwen mcp approve --all
  flag: --all
  invocation_scope:
  - applies_to: command
    command:
    - mcp
    - approve
  - applies_to: command
    command:
    - mcp
    - reject
  - applies_to: command
    command:
    - mcp
    - reconnect
  - applies_to: command
    command:
    - extensions
    - update
  notes: For extensions update, updates all extensions.
  scope:
  - mcp
  - extensions
  value: none
  value_type: none
- attachment: []
  default: 'false'
  description: 'Emit JSON output: JSON Lines for the session listings, a single object for controllers add, board JSON at the board leaves, and the full review verdict for review run.'
  evidence_ids:
  - source-sessions-cmds
  - source-board-cmd
  - source-review-run-cmd
  example: qwen sessions list --json
  flag: --json
  invocation_scope:
  - applies_to: command
    command:
    - sessions
    - list
  - applies_to: command
    command:
    - sessions
    - ps
  - applies_to: command
    command:
    - sessions
    - controllers
  - applies_to: command
    command:
    - board
  - applies_to: command
    command:
    - review
    - run
  notes: On board the flag is declared on the group builder and cascades to every leaf.
  scope:
  - io
  value: none
  value_type: none
- attachment:
  - space
  - equals
  default: '20'
  description: Maximum number of sessions to show.
  evidence_ids:
  - source-sessions-cmds
  example: qwen sessions list --json --limit 100
  flag: --limit
  invocation_scope:
  - applies_to: command
    command:
    - sessions
    - list
  notes: Non-integer or non-positive values coerce back to 20.
  scope:
  - sessions
  value: <N>
  value_optional: false
  value_type: number
- attachment:
  - space
  - equals
  description: What to call a controller token when minting it, for later revocation.
  evidence_ids:
  - source-sessions-cmds
  example: qwen sessions controllers add --label ci-runner
  flag: --label
  invocation_scope:
  - applies_to: command
    command:
    - sessions
    - controllers
  notes: Belongs to the controllers add leaf of the sessions controllers path.
  scope:
  - sessions
  value: <TEXT>
  value_optional: false
  value_type: string
- attachment: []
  default: 'false'
  description: Assemble and show items, frozen settings, and the estimate without uploading.
  evidence_ids:
  - source-batch-cmd
  example: qwen batch run --dry-run plan.json
  flag: --dry-run
  invocation_scope:
  - applies_to: command
    command:
    - batch
    - run
  notes: Prints a snapshot digest usable with --expect; rejected together with --expect.
  scope:
  - batch
  value: none
  value_type: none
- attachment:
  - space
  - equals
  description: Submit only if the batch still matches this snapshot digest from --dry-run.
  evidence_ids:
  - source-batch-cmd
  example: qwen batch run --expect <digest> plan.json
  flag: --expect
  invocation_scope:
  - applies_to: command
    command:
    - batch
    - run
  notes: Rejected together with --dry-run.
  scope:
  - batch
  value: <DIGEST>
  value_optional: false
  value_type: string
- attachment: []
  default: 'false'
  description: Poll until the batch settles then collect, or on board ask, wait for an answer.
  evidence_ids:
  - source-batch-cmd
  - source-board-cmd
  example: qwen batch collect --wait <task-id>
  flag: --wait
  invocation_scope:
  - applies_to: command
    command:
    - batch
    - collect
  - applies_to: command
    command:
    - board
    - ask
  notes: Two distinct switches share the spelling at different paths.
  scope:
  - batch
  - board
  value: none
  value_type: none
- attachment:
  - space
  - equals
  default: '30'
  description: Seconds to wait with --wait before giving up (batch collect), or seconds to wait for a board answer (board ask).
  evidence_ids:
  - source-batch-cmd
  - source-board-cmd
  example: qwen batch collect --wait --timeout 600 <task-id>
  flag: --timeout
  invocation_scope:
  - applies_to: command
    command:
    - batch
    - collect
  - applies_to: command
    command:
    - board
    - ask
  notes: batch collect has no default and validates a positive number; the board ask default is 30.
  scope:
  - batch
  - board
  value: <SECONDS>
  value_optional: false
  value_type: number
- attachment:
  - space
  - equals
  description: Output limit for the retried attempt; required to resend items that were truncated.
  evidence_ids:
  - source-batch-cmd
  example: qwen batch retry --max-output-tokens 8192 <task-id>
  flag: --max-output-tokens
  invocation_scope:
  - applies_to: command
    command:
    - batch
    - retry
  notes: Validated as a positive integer.
  scope:
  - batch
  value: <N>
  value_optional: false
  value_type: number
- attachment: []
  default: 'false'
  description: Delete the workflow task's local record even while a batch may be running.
  evidence_ids:
  - source-batch-cmd
  example: qwen batch clean --force <task-id>
  flag: --force
  invocation_scope:
  - applies_to: command
    command:
    - batch
    - clean
  scope:
  - batch
  value: none
  value_type: none
- attachment:
  - space
  - equals
  description: Board name the command operates on.
  evidence_ids:
  - source-board-cmd
  - probes-0247
  example: qwen board show --board main --json
  flag: --board
  invocation_scope:
  - applies_to: command
    command:
    - board
    - show
  - applies_to: command
    command:
    - board
    - task
  - applies_to: command
    command:
    - board
    - claim
  - applies_to: command
    command:
    - board
    - done
  - applies_to: command
    command:
    - board
    - ask
  - applies_to: command
    command:
    - board
    - answer
  - applies_to: command
    command:
    - board
    - decline
  - applies_to: command
    command:
    - board
    - prune
  notes: Declared on the board group builder and cascades to every leaf; required in practice, since board show without it errors with Pass --board <name>.
  scope:
  - board
  value: <NAME>
  value_optional: false
  value_type: string
- attachment:
  - space
  - equals
  description: Declared actor name for board operations.
  evidence_ids:
  - source-board-cmd
  example: qwen board task --board main --as reviewer "check the build"
  flag: --as
  invocation_scope:
  - applies_to: command
    command:
    - board
    - show
  - applies_to: command
    command:
    - board
    - task
  - applies_to: command
    command:
    - board
    - claim
  - applies_to: command
    command:
    - board
    - done
  - applies_to: command
    command:
    - board
    - ask
  - applies_to: command
    command:
    - board
    - answer
  - applies_to: command
    command:
    - board
    - decline
  - applies_to: command
    command:
    - board
    - prune
  notes: Declared on the board group builder and cascades to every leaf.
  scope:
  - board
  value: <ACTOR>
  value_optional: false
  value_type: string
- attachment:
  - space
  - equals
  description: Assign ownership when creating a board task.
  evidence_ids:
  - source-board-cmd
  example: qwen board task --board main --owner alice "fix the flaky test"
  flag: --owner
  invocation_scope:
  - applies_to: command
    command:
    - board
    - task
  scope:
  - board
  value: <ACTOR>
  value_optional: false
  value_type: string
- attachment:
  - space
  - equals
  description: Completion note recorded when finishing a task you own.
  evidence_ids:
  - source-board-cmd
  example: qwen board done --board main <task-id> --note "verified locally"
  flag: --note
  invocation_scope:
  - applies_to: command
    command:
    - board
    - done
  scope:
  - board
  value: <TEXT>
  value_optional: false
  value_type: string
- attachment:
  - space
  - equals
  description: Topic attached to a board question.
  evidence_ids:
  - source-board-cmd
  example: qwen board ask --board main alice "which port?" --about config
  flag: --about
  invocation_scope:
  - applies_to: command
    command:
    - board
    - ask
  scope:
  - board
  value: <TOPIC>
  value_optional: false
  value_type: string
- attachment:
  - space
  - equals
  default: '900'
  description: How long a board question stays open before expiring.
  evidence_ids:
  - source-board-cmd
  example: qwen board ask --board main alice "which port?" --ttl 3600
  flag: --ttl
  invocation_scope:
  - applies_to: command
    command:
    - board
    - ask
  scope:
  - board
  value: <SECONDS>
  value_optional: false
  value_type: number
- attachment:
  - space
  - equals
  default: '7'
  description: Cutoff in days for removing settled board items.
  evidence_ids:
  - source-board-cmd
  example: qwen board prune --board main --older-than 30
  flag: --older-than
  invocation_scope:
  - applies_to: command
    command:
    - board
    - prune
  scope:
  - board
  value: <DAYS>
  value_optional: false
  value_type: number
- attachment:
  - space
  - equals
  description: The review effort; defaults to high for a PR and medium locally.
  evidence_ids:
  - source-review-run-cmd
  example: qwen review run --effort high 1234
  flag: --effort
  invocation_scope:
  - applies_to: command
    command:
    - review
    - run
  scope:
  - review
  value: low | medium | high
  value_optional: false
  value_type: string
- attachment: []
  default: 'false'
  description: Authorize posting the review to GitHub (PR targets only).
  evidence_ids:
  - source-review-run-cmd
  example: qwen review run --comment 1234
  flag: --comment
  invocation_scope:
  - applies_to: command
    command:
    - review
    - run
  scope:
  - review
  value: none
  value_type: none
- attachment: []
  default: 'false'
  description: Continue an interrupted review of this PR when its on-disk state still matches.
  evidence_ids:
  - source-review-run-cmd
  example: qwen review run --resume 1234
  flag: --resume
  invocation_scope:
  - applies_to: command
    command:
    - review
    - run
  notes: Boolean at this path, unlike the root --resume which takes a session ID; falls back to a fresh review when nothing can be resumed.
  scope:
  - review
  value: none
  value_type: none
- attachment:
  - space
  - equals
  default: none
  description: Exit 3 when the review completes with this outcome, so CI can gate on the verdict without parsing output.
  evidence_ids:
  - source-review-run-cmd
  example: qwen review run --json --fail-on request-changes 1234
  flag: --fail-on
  invocation_scope:
  - applies_to: command
    command:
    - review
    - run
  scope:
  - review
  value: none | request-changes
  value_optional: false
  value_type: string
- attachment:
  - space
  - equals
  default: '120'
  description: Terminate the review after this long without a verdict (exit 1).
  evidence_ids:
  - source-review-run-cmd
  example: qwen review run --timeout-minutes 60 1234
  flag: --timeout-minutes
  invocation_scope:
  - applies_to: command
    command:
    - review
    - run
  scope:
  - review
  value: <N>
  value_optional: false
  value_type: number
- attachment:
  - space
  - equals
  default: yolo
  description: Approval mode for the child CLI spawned by the review.
  evidence_ids:
  - source-review-run-cmd
  example: qwen review run --approval-mode default 1234
  flag: --approval-mode
  invocation_scope:
  - applies_to: command
    command:
    - review
    - run
  notes: Defaults to yolo here because headless runs cannot answer confirmation prompts; a distinct default from the root --approval-mode.
  scope:
  - review
  - permissions
  value: plan | default | auto-edit | auto | yolo
  value_optional: false
  value_type: string
- attachment: []
  default: 'false'
  description: Suppress the child CLI progress stream on stderr.
  evidence_ids:
  - source-review-run-cmd
  example: qwen review run --quiet --json 1234
  flag: --quiet
  invocation_scope:
  - applies_to: command
    command:
    - review
    - run
  scope:
  - review
  value: none
  value_type: none
- attachment:
  - space
  - equals
  default: user
  description: 'The scope to install the extension in: user (global), project (current workspace), or workspace as an alias of project.'
  evidence_ids:
  - source-extensions-cmds
  example: qwen extensions install --scope project ./my-ext --consent
  flag: --scope
  invocation_scope:
  - applies_to: command
    command:
    - extensions
    - install
  notes: 'Different switch from mcp add --scope: no alias, and the workspace choice.'
  scope:
  - extensions
  value: user | project | workspace
  value_optional: false
  value_type: string
- attachment:
  - space
  - equals
  description: The scope to enable or disable the extension in; unset enables in all scopes.
  evidence_ids:
  - source-extensions-cmds
  example: qwen extensions disable --scope user my-ext
  flag: --scope
  invocation_scope:
  - applies_to: command
    command:
    - extensions
    - enable
  - applies_to: command
    command:
    - extensions
    - disable
  notes: Free string, not choice-validated, unlike the install spelling.
  scope:
  - extensions
  value: <SCOPE>
  value_optional: false
  value_type: string
- attachment:
  - space
  - equals
  description: Install an extension from a specific git ref.
  evidence_ids:
  - source-extensions-cmds
  example: qwen extensions install https://github.com/org/ext --ref main --consent
  flag: --ref
  invocation_scope:
  - applies_to: command
    command:
    - extensions
    - install
  notes: Not applicable to npm, archive URL, or marketplace extensions.
  scope:
  - extensions
  value: <GIT_REF>
  value_optional: false
  value_type: string
- attachment: []
  default: 'false'
  description: Enable auto-update for an installed extension.
  evidence_ids:
  - source-extensions-cmds
  example: qwen extensions install https://github.com/org/ext --auto-update --consent
  flag: --auto-update
  invocation_scope:
  - applies_to: command
    command:
    - extensions
    - install
  notes: Not applicable to marketplace extensions.
  scope:
  - extensions
  value: none
  value_type: none
- attachment: []
  default: 'false'
  description: Allow pre-release extension versions.
  evidence_ids:
  - source-extensions-cmds
  example: qwen extensions install @scope/ext --pre-release --consent
  flag: --pre-release
  invocation_scope:
  - applies_to: command
    command:
    - extensions
    - install
  scope:
  - extensions
  value: none
  value_type: none
- attachment:
  - space
  - equals
  default: npm default
  description: Use a custom npm registry for npm extensions.
  evidence_ids:
  - source-extensions-cmds
  example: qwen extensions install @scope/ext --registry https://registry.npmjs.org --consent
  flag: --registry
  invocation_scope:
  - applies_to: command
    command:
    - extensions
    - install
  notes: Only for npm extension sources.
  scope:
  - extensions
  value: <URL>
  value_optional: false
  value_type: string
- attachment: []
  default: 'false'
  description: Acknowledge extension security risks and skip the confirmation prompt.
  evidence_ids:
  - source-extensions-cmds
  example: qwen extensions install https://github.com/org/ext --consent
  flag: --consent
  invocation_scope:
  - applies_to: command
    command:
    - extensions
    - install
  notes: The only way to keep extensions install non-interactive.
  scope:
  - extensions
  value: none
  value_type: none
config_paths:
- format: json
  notes: Primary user settings; the whole global dir moves with QWEN_HOME. Written by qwen mcp add/remove --scope user and extension installs.
  os: macos
  path: ~/.qwen/settings.json
  scope: user
- format: json
  notes: Primary user settings; the whole global dir moves with QWEN_HOME.
  os: linux
  path: ~/.qwen/settings.json
  scope: user
- format: json
  notes: Primary user settings; the whole global dir moves with QWEN_HOME.
  os: windows
  path: '%USERPROFILE%\.qwen\settings.json'
  scope: user
- format: json
  notes: Project settings; ignored when the workspace is untrusted. Review and security policy keys are read from operator scopes only.
  os: macos
  path: .qwen/settings.json
  scope: repo
- format: json
  notes: Project settings; ignored when the workspace is untrusted.
  os: linux
  path: .qwen/settings.json
  scope: repo
- format: json
  notes: Project settings; ignored when the workspace is untrusted.
  os: windows
  path: .qwen\settings.json
  scope: repo
- format: json
  notes: Lowest-precedence system defaults; path overridable via QWEN_CODE_SYSTEM_DEFAULTS_PATH (source-verified).
  os: macos
  path: /Library/Application Support/QwenCode/system-defaults.json
  scope: system
- format: json
  notes: Lowest-precedence system defaults; path overridable via QWEN_CODE_SYSTEM_DEFAULTS_PATH.
  os: linux
  path: /etc/qwen-code/system-defaults.json
  scope: system
- format: json
  notes: Lowest-precedence system defaults; path overridable via QWEN_CODE_SYSTEM_DEFAULTS_PATH.
  os: windows
  path: C:\ProgramData\qwen-code\system-defaults.json
  scope: system
- format: json
  notes: Highest-precedence settings file, overriding user and project; path overridable via QWEN_CODE_SYSTEM_SETTINGS_PATH (source-verified).
  os: macos
  path: /Library/Application Support/QwenCode/settings.json
  scope: system
- format: json
  notes: Highest-precedence settings file; path overridable via QWEN_CODE_SYSTEM_SETTINGS_PATH.
  os: linux
  path: /etc/qwen-code/settings.json
  scope: system
- format: json
  notes: Highest-precedence settings file; path overridable via QWEN_CODE_SYSTEM_SETTINGS_PATH.
  os: windows
  path: C:\ProgramData\qwen-code\settings.json
  scope: system
- format: other
  notes: User-level dotenv; loaded before ~/.env and wins on conflict; existing environment values are never overwritten. Variables rejected from project .env scopes (loader-affecting, TLS, git, sandbox) are still honored from user scope.
  os: macos
  path: ~/.qwen/.env
  scope: user
- format: other
  notes: User-level dotenv; loaded before ~/.env and wins on conflict; existing environment values are never overwritten.
  os: linux
  path: ~/.qwen/.env
  scope: user
- format: other
  notes: User-level dotenv; loaded before ~/.env and wins on conflict.
  os: windows
  path: '%USERPROFILE%\.qwen\.env'
  scope: user
- format: other
  notes: Project dotenv; never subject to the default exclusion list that applies to a plain project .env (DEBUG and DEBUG_MODE are excluded there).
  os: macos
  path: .qwen/.env
  scope: repo
- format: other
  notes: Project dotenv; never subject to the default exclusion list that applies to a plain project .env.
  os: linux
  path: .qwen/.env
  scope: repo
- format: other
  notes: Project dotenv; never subject to the default exclusion list that applies to a plain project .env.
  os: windows
  path: .qwen\.env
  scope: repo
- format: text
  notes: Global hierarchical context file; filename configurable via context.fileName. Project lookups walk cwd ancestors to the git root or home.
  os: macos
  path: ~/.qwen/QWEN.md
  scope: user
- format: text
  notes: Global hierarchical context file; filename configurable via context.fileName.
  os: linux
  path: ~/.qwen/QWEN.md
  scope: user
- format: text
  notes: Global hierarchical context file; filename configurable via context.fileName.
  os: windows
  path: '%USERPROFILE%\.qwen\QWEN.md'
  scope: user
- format: text
  notes: Project context file at the repo root (and ancestors); concatenated into the system prompt. Skipped by --safe-mode.
  os: macos
  path: QWEN.md
  scope: repo
- format: text
  notes: Project context file at the repo root (and ancestors); concatenated into the system prompt.
  os: linux
  path: QWEN.md
  scope: repo
- format: text
  notes: Project context file at the repo root (and ancestors); concatenated into the system prompt.
  os: windows
  path: QWEN.md
  scope: repo
- format: json
  notes: Project MCP server declarations; gated servers require approval state before use.
  os: macos
  path: .mcp.json
  scope: repo
- format: json
  notes: Project MCP server declarations; gated servers require approval state before use.
  os: linux
  path: .mcp.json
  scope: repo
- format: json
  notes: Project MCP server declarations; gated servers require approval state before use.
  os: windows
  path: .mcp.json
  scope: repo
- format: other
  notes: Session transcripts read by qwen sessions list; the munged path encodes the session cwd. Also file-history backups under ~/.qwen/file-history/.
  os: macos
  path: ~/.qwen/projects/<munged-cwd>/chats/<session-uuid>.jsonl
  scope: user
- format: other
  notes: Session transcripts read by qwen sessions list; the munged path encodes the session cwd.
  os: linux
  path: ~/.qwen/projects/<munged-cwd>/chats/<session-uuid>.jsonl
  scope: user
- format: other
  notes: Session transcripts read by qwen sessions list; the munged path encodes the session cwd.
  os: windows
  path: '%USERPROFILE%\.qwen\projects\<munged-cwd>\chats\<session-uuid>.jsonl'
  scope: user
- format: text
  notes: Anonymous installation identifier written on first run; observed on this host.
  os: macos
  path: ~/.qwen/installation_id
  scope: user
- format: text
  notes: Anonymous installation identifier written on first run.
  os: linux
  path: ~/.qwen/installation_id
  scope: user
- format: text
  notes: Anonymous installation identifier written on first run.
  os: windows
  path: '%USERPROFILE%\.qwen\installation_id'
  scope: user
- format: text
  notes: Runtime debug logs; the whole runtime tree (sessions, todos, tmp, debug) relocates with QWEN_RUNTIME_DIR while config files stay under QWEN_HOME.
  os: macos
  path: ~/.qwen/debug/*.txt
  scope: user
- format: text
  notes: Runtime debug logs; relocatable with QWEN_RUNTIME_DIR.
  os: linux
  path: ~/.qwen/debug/*.txt
  scope: user
- format: text
  notes: Runtime debug logs; relocatable with QWEN_RUNTIME_DIR.
  os: windows
  path: '%USERPROFILE%\.qwen\debug\*.txt'
  scope: user
env_vars:
- effect: Overrides the global configuration directory, defaulting to ~/.qwen; accepts absolute or relative paths and expands a leading ~.
  name: QWEN_HOME
- effect: Overrides the runtime output directory for conversations, logs, todos, and temp files; defaults to the QWEN_HOME directory, and config files stay under QWEN_HOME either way.
  name: QWEN_RUNTIME_DIR
- effect: Overrides the system settings file path.
  name: QWEN_CODE_SYSTEM_SETTINGS_PATH
- effect: Overrides the system defaults file path.
  name: QWEN_CODE_SYSTEM_DEFAULTS_PATH
- effect: Enables or selects the sandbox provider as an alternative to the sandbox setting; accepts true, false, docker, podman, sandbox-exec, or a custom command.
  name: QWEN_SANDBOX
- effect: Overrides sandbox image selection for Docker and Podman.
  name: QWEN_SANDBOX_IMAGE
- effect: Switches the macOS Seatbelt (sandbox-exec) profile.
  name: SEATBELT_PROFILE
- effect: Builds a custom sandbox image from .qwen/sandbox.Dockerfile when set.
  name: BUILD_SANDBOX
- effect: Injects extra flags into docker or podman sandbox commands.
  name: SANDBOX_FLAGS
- effect: Enables safe mode where CLI flags cannot be passed.
  name: QWEN_CODE_SAFE_MODE
- effect: Suppresses the warning emitted for headless YOLO runs without sandboxing.
  name: QWEN_CODE_SUPPRESS_YOLO_WARNING
- effect: Set to true or 1 for persistent retry of transient 429 and 529 capacity errors, with exponential backoff capped at five minutes and stderr heartbeat keepalives every 30 seconds.
  name: QWEN_CODE_UNATTENDED_RETRY
- effect: Adds comma-separated slash commands to hide or disable; unioned with the CLI flag and the settings key.
  name: QWEN_DISABLED_SLASH_COMMANDS
- effect: Overrides the UI language.
  name: QWEN_CODE_LANG
- effect: Set to 1 to disable TLS certificate verification, matching --insecure.
  name: QWEN_TLS_INSECURE
- effect: Set to any value to disable all color output.
  name: NO_COLOR
- effect: 'Overrides OSC 8 hyperlink detection: 1 or a non-zero integer forces links on, 0 or a non-numeric value forces them off.'
  name: FORCE_HYPERLINK
- effect: Set to 1 to hard-disable OSC 8 clickable hyperlinks even on capable terminals.
  name: QWEN_DISABLE_HYPERLINKS
- effect: Set to true or 1 to enable verbose debug logging; excluded from project .env files by default.
  name: DEBUG
- effect: Set to true or 1 to enable verbose debug logging; excluded from project .env files by default.
  name: DEBUG_MODE
- effect: Proxy fallback when proxy is not set by CLI flag or settings.
  name: HTTPS_PROXY
- effect: Proxy fallback when proxy is not set by CLI flag or settings.
  name: HTTP_PROXY
- effect: Bearer token source for qwen serve and SDK clients.
  name: QWEN_SERVER_TOKEN
- effect: Default server-side deadline for qwen serve prompt requests.
  name: QWEN_SERVE_PROMPT_DEADLINE_MS
- effect: Idle deadline for qwen serve SSE writers.
  name: QWEN_SERVE_WRITER_IDLE_TIMEOUT_MS
- effect: Set to 1 or true to enable qwen serve HTTP rate limiting.
  name: QWEN_SERVE_RATE_LIMIT
- effect: Prompt request rate limit when serve rate limiting is enabled.
  name: QWEN_SERVE_RATE_LIMIT_PROMPT
- effect: Mutation request rate limit when serve rate limiting is enabled.
  name: QWEN_SERVE_RATE_LIMIT_MUTATION
- effect: Read request rate limit when serve rate limiting is enabled.
  name: QWEN_SERVE_RATE_LIMIT_READ
- effect: Serve rate-limit window in milliseconds.
  name: QWEN_SERVE_RATE_LIMIT_WINDOW_MS
- effect: Enables extra qwen serve bridge debug breadcrumbs.
  name: QWEN_SERVE_DEBUG
- effect: Chooses how project memory is partitioned (git-root or workspace); the serve --memory-project-scope flag overrides it.
  name: QWEN_CODE_MEMORY_PROJECT_SCOPE
- effect: Fallback capability digest for the serve hosted-harness profile.
  name: QWEN_HOSTED_HARNESS_CAPABILITY_DIGEST
- effect: Overrides the default maximum output tokens per response with a fixed limit, useful for capacity-constrained self-hosted backends.
  name: QWEN_CODE_MAX_OUTPUT_TOKENS
- effect: Download source for standalone updates; rejected from project .env scopes.
  name: QWEN_UPDATE_BASE_URL
- effect: Set to true or 1 to enable anonymized usage statistics; takes precedence over the privacy.usageStatisticsEnabled setting.
  name: QWEN_USAGE_STATISTICS_ENABLED
machine_introspection:
- command: qwen --version
  machine_readable: false
  notes: Prints a bare semver line; trivially parseable but not structured. Accepted at every command path.
  output_format: text
  purpose: version
  useful_for_codegen: false
- command: qwen sessions list --json --limit 100
  machine_readable: true
  notes: One JSON object per session (sessionId, startTime, mtime, prompt, gitBranch, customTitle, titleSource, filePath, cwd) on stdout; the hasMore hint is kept on stderr so pipelines stay clean. Verified on this host.
  output_format: jsonl
  purpose: other
  useful_for_codegen: false
- command: qwen sessions ps --json
  machine_readable: true
  notes: Lists registered and managed live sessions as JSON Lines; shape not captured on this idle host.
  output_format: jsonl
  purpose: other
  useful_for_codegen: false
- command: qwen sessions controllers list --json
  machine_readable: true
  notes: Lists controller tokens as JSON Lines; controllers add --json prints the minted token as a single JSON object once.
  output_format: jsonl
  purpose: other
  useful_for_codegen: false
- command: qwen review run --json [target]
  machine_readable: true
  notes: Prints the full review verdict as one JSON object; exit 3 gates CI via --fail-on; runs a real billed review.
  output_format: json
  purpose: other
  useful_for_codegen: false
- command: qwen board show --board <name> --json
  machine_readable: true
  notes: Prints the board state as JSON; requires --board.
  output_format: json
  purpose: other
  useful_for_codegen: false
- command: qwen sandbox
  machine_readable: false
  notes: Reports the effective execution policy as text; --verify proves the kernel boundary. Verified on 0.24.7.
  output_format: text
  purpose: doctor
  useful_for_codegen: false
- command: qwen --list-extensions
  machine_readable: false
  notes: Lists extensions and exits; no JSON mode found. Verified on 0.24.7.
  output_format: text
  purpose: plugins
  useful_for_codegen: false
- command: qwen mcp list
  machine_readable: false
  notes: Lists configured MCP servers as text; no own options and no JSON mode.
  output_format: text
  purpose: mcp
  useful_for_codegen: false
- command: qwen serve then GET /capabilities
  machine_readable: true
  notes: Requires starting the HTTP daemon and calling its API; not a fire-and-exit CLI probe.
  output_format: json
  purpose: capabilities
  useful_for_codegen: true
wrapper_notes:
- 'Version skew is real: this host runs Homebrew qwen 0.19.8 while npm latest and the formula are 0.24.7. Never infer the current surface from an arbitrary installed binary; check qwen --version.'
- The parser is yargs 17.7.2 in strict mode. Default-command flags are rejected at every subcommand path with Unknown arguments, even though leaf --help misleadingly displays them; only the thirteen root-declared globals (telemetry set, debug, bare, safe-mode, proxy, insecure, chat-recording) plus --help and --version are accepted everywhere.
- 'Attached short values are NOT supported: -ojson is cluster-parsed and rejected as unknown flags b, g, u. Use -o json or -o=json. Long flags accept both --flag value and --flag=value.'
- Array flags (--allowed-tools, --include-directories, --extensions, --fallback-model, --core-tools, --exclude-tools, --disabled-slash-commands, --allowed-mcp-server-names) greedily consume following non-flag arguments, so qwen --allowed-tools a,b "prompt" swallows the prompt. Put the positional prompt first, or use the = form, or comma-join values.
- The bare root command with no prompt, positional, or piped stdin launches the interactive TUI. Wrappers must always pass a prompt (positional preferred; --prompt is deprecated), pipe stdin, or use a mode flag.
- --output-format stream-json emits line-delimited events for streaming wrappers; --output-format json buffers a JSON array until completion. --json-fd and --json-file give dual structured output while the TUI stays on stdout; --json-fd needs explicit fd plumbing in the spawn call and its number coercion is lenient (NaN slips through).
- --json-schema enforces strict final output but is rejected with --prompt-interactive, --input-format stream-json, --acp, or a no-prompt interactive invocation.
- '--yolo and --approval-mode=yolo do not enable sandboxing; headless YOLO without sandbox prints a stderr warning unless QWEN_CODE_SUPPRESS_YOLO_WARNING=1. review run child sessions default to approval-mode yolo for the same reason: headless runs cannot answer prompts.'
- '--bare and --safe-mode are the isolation controls: bare skips implicit startup discovery, safe-mode disables context files, hooks, extensions, skills, and MCP servers.'
- Untrusted folders ignore project .qwen/settings.json and force risky approval modes back toward default behavior. Review and security policy settings are read from operator scopes only, so a repo cannot set them for its reviewers.
- --worktree prompts on exit to keep or remove the worktree; avoid it for unattended wrappers. --resume without an ID opens an interactive session picker, so always pass the ID.
- auth is a removed command that only prints guidance. Qwen OAuth was discontinued on 2026-04-15; scripted or CI setups should use provider API keys (for example the --openai-api-key and --openai-base-url flags or their env vars) rather than any OAuth flow.
- 'sessions list --json keeps stdout clean: the hasMore hint goes to stderr, so qwen sessions list --json | jq pipelines are safe.'
- The npm package requires Node.js >=22.0.0; older hosts need the standalone installer.
- qwen sessions list reads transcripts under ~/.qwen/projects/<munged-cwd>/chats/, so a wrapper can locate session files for a known cwd without invoking the CLI.
changes:
- Refreshed from npm 0.19.6 to 0.24.7 (local binary 0.15.6 to 0.19.8; Homebrew stable 0.19.5 to 0.24.7); 0.24.7 root help is complete again, no longer the compact form the previous revision had to reconcile with the packaged parser.
- 'Rewrote the switch inventory to contract revision 2: every switch now carries a value type, attachment forms, aliases, and an invocation scope, each resting on named evidence (yargs 17.7.2 declarations in source, parse-level probes on 0.24.7, and a disposable yargs harness).'
- 'Recorded the parser-scoping facts new to this revision: thirteen root globals plus --help/--version accepted at every path; default-command flags rejected at subcommand paths despite leaf help displaying them; short-flag attached values unsupported; array flags greedy with a comma-splitting coerce; hidden flags --experimental-acp, --experimental-skills, and --sandbox-session-id.'
- 'Added new subcommands since 0.19.6: batch (seven leaves), board (eight leaves), sandbox, update, sessions ps, sessions controllers, channel reload and configure-weixin, extensions settings and sources, and review run as the public non-interactive review entry; inventoried serve''s expanded option set (memory budgets, journals, TLS, local control, external tool guard, rate limits, reserved managed-runtime flags).'
- Recorded Qwen OAuth discontinuation (2026-04-15) and the new openai-responses auth-type choice.
- Expanded configuration discovery with the .env layering rules (~/.qwen/.env wins over ~/.env; project .env exclusion lists), the hierarchical QWEN.md context lookup, session transcript storage paths, and the four-layer settings precedence verified in source.
- Added machine introspection entries for sessions ps, sessions controllers list, review run --json, and board show --json.
requires_claudine_update: true
reason: 'Claudine''s generated provider metadata must be regenerated from the revision-2 switch inventory (new flags such as --advisor, --output-style, --fallback-model, --restore-ask-user-question, and the openai-responses auth choice), and the wrapper''s argument partitioning should account for the recorded parser facts: greedy array flags that swallow following positionals, unsupported short-flag attached values, strict rejection of root flags at subcommand paths, and --resume/--worktree being value-optional.'
contract_checked: 2026-10-01
---

# Qwen Code CLI (qwen)

## Overview

Qwen Code is the Qwen team's open-source terminal coding agent, shipped from the [QwenLM/qwen-code](https://github.com/QwenLM/qwen-code) repository as the `@qwen-code/qwen-code` npm package, standalone curl/PowerShell installers, and a Homebrew formula. The command users type is `qwen`. The parser is yargs 17.7.2 in strict mode.

The newest released version I verified on 2026-10-01 is `0.24.7`: `npm view @qwen-code/qwen-code version` reports 0.24.7 as the `latest` dist-tag, the GitHub repository carries the `v0.24.7` tag, Homebrew stable is 0.24.7 (bottled), and `npx --yes @qwen-code/qwen-code@0.24.7 --version` prints `0.24.7`. The binary installed on this host is older: `/opt/homebrew/bin/qwen` reports `0.19.8` (`qwen --version`). All switch records below were established against 0.24.7 unless a record names 0.19.8.

Primary links:

- Homepage: [https://qwen.ai/qwencode](https://qwen.ai/qwencode)
- Repository: [https://github.com/QwenLM/qwen-code](https://github.com/QwenLM/qwen-code)
- General docs: [https://qwenlm.github.io/qwen-code-docs/](https://qwenlm.github.io/qwen-code-docs/)
- CLI reference: [https://qwenlm.github.io/qwen-code-docs/en/users/features/commands/](https://qwenlm.github.io/qwen-code-docs/en/users/features/commands/)

## Installation and Binaries

The public executable name is `qwen` on every operating system. The npm package declares `bin: { "qwen": ... }`; Windows npm installs normally add `qwen.cmd` and `qwen.ps1` shims. On this macOS host Homebrew links `/opt/homebrew/bin/qwen` into the `qwen-code` Cellar keg.

| OS | Method | Command |
| --- | --- | --- |
| macOS | standalone | `curl -fsSL https://qwen-code-assets.oss-cn-hangzhou.aliyuncs.com/installation/install-qwen-standalone.sh \| bash` |
| Linux | standalone | `curl -fsSL https://qwen-code-assets.oss-cn-hangzhou.aliyuncs.com/installation/install-qwen-standalone.sh \| bash` |
| Windows | standalone | `irm https://qwen-code-assets.oss-cn-hangzhou.aliyuncs.com/installation/install-qwen-standalone.ps1 \| iex` |
| macOS / Linux / Windows | npm | `npm install -g @qwen-code/qwen-code@latest` |
| macOS / Linux | Homebrew | `brew install qwen-code` |

npm 0.24.7 requires Node.js `>=22.0.0`. The standalone installer is the recommended path on hosts without Node 22.

## Subcommands

The default command `$0 [query..]` is the root entrypoint: with a positional prompt (or `--prompt`, or piped stdin) it runs a headless one-shot task; `-i/--prompt-interactive` runs the prompt and stays interactive; with nothing it launches the TUI. The root entrypoint itself is not listed below.

| Command path | Non-interactive | What it does |
| --- | --- | --- |
| `auth` | yes | Removed; prints migration guidance and exits |
| `batch` + `run`/`collect`/`retry`/`cancel`/`list`/`check`/`clean` | yes | DashScope Batch API workflow tasks; `check` performs no billed request |
| `board` + `show`/`task`/`claim`/`done`/`ask`/`answer`/`decline`/`prune` | yes | Shared work board for cooperating agents; group flags `--board`/`--as`/`--json` cascade to every leaf |
| `channel` + `start`/`stop`/`status`/`reload`/`set`/`pairing`/`configure-weixin` | mixed | Messaging channel integrations; only `status` was verified to complete without a terminal, and `configure-weixin` needs a QR-code login |
| `extensions` + `install`/`uninstall`/`list`/`update`/`disable`/`enable`/`link`/`new`/`settings`/`sources` | mixed | Extension management; `install` prompts without `--consent`, the other leaves were probed to complete without prompting |
| `hooks` (alias `hook`) | yes | Placeholder; bare invocation prints nothing and exits 0 on 0.24.7 |
| `mcp` + `add`/`remove`/`list`/`reconnect`/`approve`/`reject` | yes | MCP server management; `add`/`remove` write settings, `approve`/`reject` mutate approval state |
| `review` (+ ~41 internal helpers) and `review run` | yes | `review run` is the public non-interactive review entry (JSON verdict, CI exit-code gating); the other leaves are /review-skill internals |
| `sandbox [cmd...]` | yes | Report the sandbox policy, `--verify` it, or run one confined command |
| `serve` | yes | Long-running local HTTP daemon; no TTY |
| `sessions` + `list`/`ps`/`controllers` | yes | Session listings (JSON Lines) and controller-token management |
| `update` | yes | Check for updates and install if available |

Deeper leaves exist below `channel pairing` (`list`, `approve`), `sessions controllers` (`add`, `list`, `remove`), `extensions settings` (`set`, `list`), and `extensions sources` (`add`, `remove`, `list`, `update`); they are recorded in their parent's notes.

## CLI Switch Inventory

Inventoried paths: the root entrypoint (all default-command switches), every path marked non-interactive above, and the extensions family for completeness. The full typed inventory is in frontmatter; each record cites its evidence.

How the parser decides ownership of an argument:

- **Thirteen root globals** (`--telemetry*`, `--debug/-d`, `--bare`, `--safe-mode`, `--proxy`, `--insecure`, `--chat-recording`) plus `--help/-h` and `--version/-v` are declared at the root before commands, so yargs accepts them at **every** command path. Verified: `qwen sessions list --telemetry-target bogus` fails with a choice error, and `qwen sessions list --version` prints the version.
- **Default-command switches** (everything from `--model` to `--auth-type`) live inside the `$0 [query..]` builder, so strict mode rejects them at any subcommand path — `qwen sessions list --approval-mode bogus` fails with `Unknown arguments: approval-mode, approvalMode` — **even though leaf `--help` misleadingly displays the full default-command option set**.
- **Value attachment**: long and short flags accept `--flag value` and `--flag=value` (also `-o=json`; the `=` is stripped). Attached short values are **not** supported: `-obogus` is cluster-parsed and rejected as unknown flags. Established by parse tests on 0.24.7 and a disposable harness on yargs 17.7.2 configured exactly as qwen configures it.
- **Variadic switches**: array-typed flags (`--allowed-tools`, `--fallback-model`, `--extensions`, `--include-directories/--add-dir`, `--core-tools`, `--exclude-tools`, `--disabled-slash-commands`, `--allowed-mcp-server-names`) consume every following non-flag argument until the next flag, and qwen adds comma-splitting. An occurrence with zero values parses to an empty list, which the contract floor of 1 cannot express, so those records carry `variadic_min: unknown` with a gap. `mcp add --env`/`--header` use `nargs: 1` (one value per occurrence, repeatable) and are recorded as `string`.
- **Value-optional**: `--resume` (picker without an ID), `--worktree` (bare auto-generates a slug), and `--prompt-interactive` (empty value explicitly handled in source).
- **Hidden but accepted**: `--experimental-acp` (maps to `--acp`), `--experimental-skills` (ignored), `--sandbox-session-id` (internal UUID).
- **Parse-time mutual exclusions**: prompt vs positional prompt, `--prompt` vs `--prompt-interactive`, `--yolo` vs `--approval-mode`, `--continue`/`--resume`/`--session-id` pairings, `--fork-session` requiring resume, `--json-fd` vs `--json-file`, the `--json-schema` combinations, `--input-format stream-json` and `--include-partial-messages` requiring `-o stream-json`, and the `bwrap` migration guard.

Same spelling, different switch: `--channel` is a choices string at the root but a repeatable array at `serve`; `--exclude-tools` is core tools at the root but MCP-server tools at `mcp add`; `--resume` takes a session ID at the root but is a boolean at `review run`; `--approval-mode` defaults to `default` at the root but `yolo` at `review run`; `--scope` differs across `mcp add`/`mcp remove`, `extensions install`, and `extensions enable`/`disable`; `--timeout` differs between `batch collect` and `board ask`.

The system-prompt delivery flags `--system-prompt` and `--append-system-prompt` are recorded with spelling and value type; their replace-vs-append semantics belong to the `system-prompt` topic.

## Configuration Discovery

Settings apply in seven layers, each overriding the last: built-in defaults, system defaults file, user file, project file, system override file, environment variables (including `.env` files), and command-line arguments.

| Scope | macOS | Linux | Windows |
| --- | --- | --- | --- |
| System defaults | `/Library/Application Support/QwenCode/system-defaults.json` | `/etc/qwen-code/system-defaults.json` | `C:\ProgramData\qwen-code\system-defaults.json` |
| User | `~/.qwen/settings.json` | `~/.qwen/settings.json` | `%USERPROFILE%\.qwen\settings.json` |
| Project | `.qwen/settings.json` | `.qwen/settings.json` | `.qwen\settings.json` |
| System override | `/Library/Application Support/QwenCode/settings.json` | `/etc/qwen-code/settings.json` | `C:\ProgramData\qwen-code\settings.json` |

Paths verified in `storage-paths-lite.ts` at v0.24.7; `QWEN_CODE_SYSTEM_SETTINGS_PATH` and `QWEN_CODE_SYSTEM_DEFAULTS_PATH` override the system paths, and `QWEN_HOME` relocates the whole global directory (settings included), while `QWEN_RUNTIME_DIR` relocates only runtime output (sessions, debug logs, temp files).

Environment files load in a documented order: `<QWEN_HOME>/.env` before `~/.env`, the Qwen-specific file winning conflicts, and existing environment values are never overwritten. A plain project `.env` is subject to a default exclusion list (for example `DEBUG`/`DEBUG_MODE`) plus a frozen rejection list covering loader-affecting variables, TLS trust anchors, and git command-execution variables; `.qwen/.env` bypasses the default exclusion list but not the loader-affecting rejections.

Context files default to `QWEN.md` (configurable via `context.fileName`) and load hierarchically: `~/.qwen/QWEN.md` first, then the project root and its ancestors up to the git root or home, concatenated into the system prompt. `.mcp.json` declares project MCP servers. Session transcripts live under `~/.qwen/projects/<munged-cwd>/chats/<uuid>.jsonl` — the same records `qwen sessions list --json` reads. The CLI writes `~/.qwen/installation_id` on first run (observed on this host).

## Environment Variables

General wrapper-relevant variables are in frontmatter. Model-endpoint variables belong to `model-config`, permission variables to `agent-permissions`, MCP variables to `mcp`, and logging variables to `agent-logging`. The most operationally important here: `QWEN_HOME` and `QWEN_RUNTIME_DIR` for layout isolation, `QWEN_CODE_SAFE_MODE` and the `QWEN_SANDBOX*` family for confinement, `QWEN_CODE_UNATTENDED_RETRY` for capacity-error resilience in unattended runs, `NO_COLOR`/`FORCE_HYPERLINK`/`QWEN_DISABLE_HYPERLINKS` for terminal behavior, and the `QWEN_SERVER_TOKEN`/`QWEN_SERVE_*` family for the daemon.

## Machine Introspection

| Command | Format | Notes |
| --- | --- | --- |
| `qwen sessions list --json --limit N` | JSON Lines | Clean stdout (hasMore hint on stderr); verified on this host |
| `qwen sessions ps --json` | JSON Lines | Registered and managed live sessions |
| `qwen sessions controllers list --json` | JSON Lines | Controller tokens; `add --json` prints the minted token once |
| `qwen review run --json [target]` | JSON | Full review verdict; billed run; exit 3 gating via `--fail-on` |
| `qwen board show --board <name> --json` | JSON | Board state; requires `--board` |
| `qwen sandbox` | text | Effective execution policy report; `--verify` proves the boundary |
| `qwen --list-extensions` | text | No JSON mode |
| `qwen mcp list` | text | No JSON mode |
| `qwen serve` + `GET /capabilities` | JSON | Requires the running daemon |

## Wrapper Notes

- Version skew: installed binaries lag upstream (0.19.8 here vs 0.24.7 upstream). Check `qwen --version`; never infer the surface from an arbitrary install.
- The strict parser rejects root flags at subcommand paths even though leaf `--help` shows them. When forwarding a mixed command line, default-command switches are only valid on the root entrypoint.
- `-ojson`-style attached short values fail; use `-o json` or `-o=json`.
- Array flags greedily swallow following non-flag arguments, including a trailing positional prompt. Put the prompt first, use `=`, or comma-join values.
- A bare `qwen` with no prompt, positional, or piped stdin launches the interactive TUI — wrappers must always provide one of the three or an explicit mode flag.
- `--output-format stream-json` for streaming; `--output-format json` buffers. Dual output via `--json-fd`/`--json-file` (mutually exclusive; `--json-fd` needs spawn stdio plumbing and its number validation is lenient).
- YOLO does not sandbox: headless YOLO without sandbox warns on stderr unless `QWEN_CODE_SUPPRESS_YOLO_WARNING=1`; `review run` children default to yolo because headless cannot answer prompts.
- Untrusted folders drop project `.qwen/settings.json`; review/security policy settings are operator-scope only.
- `--worktree` prompts on exit (keep/remove dialog) and `--resume` without an ID opens an interactive picker — both are hazards for unattended runs.
- `auth` is removed and Qwen OAuth was discontinued on 2026-04-15; scripted setups should use API-key auth (`--openai-api-key`, `--openai-base-url`, or their env vars).
- `sessions list --json` is pipeline-safe (structured stdout, hints on stderr).

## Sources

- [Qwen Code overview](https://qwenlm.github.io/qwen-code-docs/en/users/overview/)
- [Qwen Code quickstart](https://qwenlm.github.io/qwen-code-docs/en/users/quickstart/) (install commands, Node 22, OAuth discontinuation)
- [Qwen Code configuration settings](https://qwenlm.github.io/qwen-code-docs/en/users/configuration/settings/) (layers, file locations, env and CLI tables, .env rules, context files)
- [QwenLM/qwen-code at tag v0.24.7 (commit b12edec)](https://github.com/QwenLM/qwen-code/tree/b12edec1401a28fc53cd9e714d5928b285071fc8) — `packages/cli/src/config/top-level-options.ts`, `packages/cli/src/config/config.ts`, `packages/cli/src/config/storage-paths-lite.ts`, `packages/cli/src/commands/{sessions,mcp,serve,sandbox,batch,board,review,extensions}/`
- [@qwen-code/qwen-code on npm](https://www.npmjs.com/package/@qwen-code/qwen-code)
- [Homebrew qwen-code formula](https://formulae.brew.sh/formula/qwen-code)
- Local commands run on 2026-10-01: `qwen --version` (0.19.8), `npm view @qwen-code/qwen-code version dist-tags engines --json`, `brew info qwen-code`, `git ls-remote --tags`, and the npx 0.24.7 probes and parse tests cited in the frontmatter evidence (`parse-test-0247`, `yargs-parser-test`, `probes-0247`)
- Disposable yargs 17.7.2 harness used for parser semantics (recorded in the `yargs-parser-test` evidence entry)

## Changelog

- 2026-10-01: Refreshed from npm 0.19.6 to 0.24.7; recorded local 0.19.8 and Homebrew 0.24.7. Rewrote the document for contract revision 2 with a fully typed switch inventory.
- 2026-10-01: Established and recorded the parser facts: thirteen globals accepted everywhere, default-command flags strict-rejected at subcommand paths, no short-flag attached values, greedy array flags, `--no-` negation, hidden flags, and parse-time mutual exclusions.
- 2026-10-01: Added new command families (batch, board, sandbox, update, sessions ps/controllers, review run) and the expanded serve option set.
- 2026-10-01: Recorded Qwen OAuth discontinuation, the openai-responses auth type, .env layering, hierarchical QWEN.md lookup, and session transcript paths.
- 2026-07-03: Previous revision — npm 0.19.6 surface, per-OS records, first typed pass at the compact-help reconciliation (see git history for the full 0.19.x-era changelog).