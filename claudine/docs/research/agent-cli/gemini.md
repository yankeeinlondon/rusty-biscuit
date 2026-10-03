---
$schema: ./_schema.yaml
schema_revision: 2
provider: gemini
created: 2026-07-02
last_updated: 2026-10-01
agent: codex
model: gpt-6.1-sol
reasoning_effort: medium
latest_version: 0.62.0
versions_examined:
- 0.61.0
- 0.62.0
evidence:
- claim: parseArguments declares root switches, aliases, types, nargs and coercions; only debug/isCommand/help are available across management paths.
  id: root-source
  limitations: Source declaration rather than live model execution; version is disabled by management builders.
  location: https://github.com/google-gemini/gemini-cli/blob/v0.61.0/packages/cli/src/config/config.ts
  method: source_code
  observed_on: 2026-10-01
  version: 0.61.0
- claim: Root parseArguments and all command module files are byte-identical to 0.61.0 in the examined tags.
  id: latest-source
  limitations: The installed executable is 0.61.0; 0.62.0 runtime was not installed or run.
  location: https://github.com/google-gemini/gemini-cli/blob/v0.62.0/packages/cli/src/config/config.ts
  method: source_code
  observed_on: 2026-10-01
  version: 0.62.0
- claim: CommandModule declarations establish paths, singular aliases, command-specific switches and prompt behavior.
  id: commands-source
  limitations: Prompting depends on settings and extension metadata; help does not run handlers.
  location: https://github.com/google-gemini/gemini-cli/tree/v0.61.0/packages/cli/src/commands
  method: source_code
  observed_on: 2026-10-01
  version: 0.61.0
- claim: 323 isolated yargs 17.7.2 parses using extracted option type/nargs/alias/choices establish space/equals, optional values, camelCase and double-dash aliases, negation and zero-value arrays; short-numeric.json adds 15 numeric attachment parses, and extra-parser-results.json tests nonboolean negations and single-token array consumption.
  id: parser-tests
  limitations: Reconstructed declaration harness excludes application handlers/coercions; body includes reproduction and representative results. It does not prove arbitrary alphabetic short attachments.
  location: /tmp/gemini-cli-research/parser-results.json
  method: disposable_test
  observed_on: 2026-10-01
  version: 0.61.0
- claim: Installed version/help at all 52 paths; invalid output-format and session-id probes establish parsing and spelling acceptance; isolated state-listing probes establish output channels.
  id: binary-tests
  limitations: macOS only, empty isolated user state, no paid model call; root-help.txt and help-*.txt preserve help output.
  location: /tmp/gemini-cli-research/binary-probes.json
  method: local_inspection
  observed_on: 2026-10-01
  version: 0.61.0
homepage: https://geminicli.com/
repo: https://github.com/google-gemini/gemini-cli
docs: https://geminicli.com/docs/
cli_docs: https://geminicli.com/docs/cli/cli-reference/
binaries:
- alt_binaries: []
  binary: gemini
  notes: npm package bin maps gemini to bundle/gemini.js; official examples use gemini. Installed macOS command returned 0.61.0.
  os: macos
- alt_binaries: []
  binary: gemini
  notes: npm package bin maps gemini to bundle/gemini.js; official examples use gemini. Not executed on this OS.
  os: linux
- alt_binaries:
  - gemini.cmd
  - gemini.ps1
  binary: gemini
  notes: npm package bin maps gemini to bundle/gemini.js; official examples use gemini. Not executed on this OS; Windows shims are npm packaging conventions.
  os: windows
install_methods:
- command: npm install -g @google/gemini-cli
  method: npm
  notes: Official stable global install. Requires Node.js 20.0.0+.
  os: macos
- command: npm install -g @google/gemini-cli
  method: npm
  notes: Official stable global install. Requires Node.js 20.0.0+.
  os: linux
- command: npm install -g @google/gemini-cli
  method: npm
  notes: Official stable global install. Requires Node.js 20.0.0+ and PowerShell is supported.
  os: windows
- command: npx @google/gemini-cli
  method: npm
  notes: Official no-permanent-install execution path.
  os: macos
- command: npx @google/gemini-cli
  method: npm
  notes: Official no-permanent-install execution path.
  os: linux
- command: npx @google/gemini-cli
  method: npm
  notes: Official no-permanent-install execution path.
  os: windows
- command: brew install gemini-cli
  method: brew
  notes: Official Homebrew install.
  os: macos
- command: brew install gemini-cli
  method: brew
  notes: Official Homebrew/Linuxbrew install.
  os: linux
- command: sudo port install gemini-cli
  method: package_manager
  notes: Official MacPorts install.
  os: macos
- command: conda create -y -n gemini_env -c conda-forge nodejs && conda activate gemini_env && npm install -g @google/gemini-cli
  method: other
  notes: Official Anaconda path for restricted environments; Gemini CLI is still installed via npm inside the conda environment.
  os: macos
- command: conda create -y -n gemini_env -c conda-forge nodejs && conda activate gemini_env && npm install -g @google/gemini-cli
  method: other
  notes: Official Anaconda path for restricted environments; Gemini CLI is still installed via npm inside the conda environment.
  os: linux
- command: conda create -y -n gemini_env -c conda-forge nodejs && conda activate gemini_env && npm install -g @google/gemini-cli
  method: other
  notes: Official Anaconda path for restricted environments; Gemini CLI is still installed via npm inside the conda environment.
  os: windows
- command: docker run --rm -it us-docker.pkg.dev/gemini-code-dev/gemini-cli/sandbox:0.42.0-nightly.20260428.g59b2dea0e
  method: other
  notes: Exact version-pinned example in tagged installation documentation; -it is interactive.
  os: macos
- command: docker run --rm -it us-docker.pkg.dev/gemini-code-dev/gemini-cli/sandbox:0.42.0-nightly.20260428.g59b2dea0e
  method: other
  notes: Exact version-pinned example in tagged installation documentation; -it is interactive.
  os: linux
- command: docker run --rm -it us-docker.pkg.dev/gemini-code-dev/gemini-cli/sandbox:0.42.0-nightly.20260428.g59b2dea0e
  method: other
  notes: Exact version-pinned example in tagged installation documentation; -it is interactive.
  os: windows
- command: npm run start
  method: source
  notes: Official source-tree development command from the repository root; `npm run start:prod` is the production-mode source run.
  os: macos
- command: npm run start
  method: source
  notes: Official source-tree development command from the repository root; `npm run start:prod` is the production-mode source run.
  os: linux
- command: npm run start
  method: source
  notes: Official source-tree development command from the repository root; `npm run start:prod` is the production-mode source run.
  os: windows
subcommands:
- description: Alias of extensions.
  name: extension
  non_interactive: false
  notes: Command group requires a leaf command; no standalone operation.
- description: Configure extension settings.
  name: extension config
  non_interactive: false
  notes: Can request consent or settings input; do not assume unattended completion. Singular alias of extensions config.
- description: Disables an extension.
  name: extension disable
  non_interactive: true
  notes: Handler terminates without a user prompt; established from source, not by performing mutations. Singular alias of extensions disable.
- description: Enables an extension.
  name: extension enable
  non_interactive: true
  notes: Handler terminates without a user prompt; established from source, not by performing mutations. Singular alias of extensions enable.
- description: Installs an extension from a git repository URL or a local path.
  name: extension install
  non_interactive: false
  notes: Unattended installation requires both --consent and --skip-settings; not exercised. Singular alias of extensions install.
- description: Links an extension from a local path. Updates made to the local path will always be reflected.
  name: extension link
  non_interactive: false
  notes: Can request consent or settings input; do not assume unattended completion. Singular alias of extensions link.
- description: Lists installed extensions.
  name: extension list
  non_interactive: true
  notes: Handler terminates without a user prompt; established from source, not by performing mutations. Singular alias of extensions list.
- description: Create a new extension from a boilerplate example.
  name: extension new
  non_interactive: true
  notes: Handler terminates without a user prompt; established from source, not by performing mutations. Singular alias of extensions new.
- description: Uninstalls one or more extensions.
  name: extension uninstall
  non_interactive: true
  notes: Handler terminates without a user prompt; established from source, not by performing mutations. Singular alias of extensions uninstall.
- description: Updates all extensions or a named extension to the latest version.
  name: extension update
  non_interactive: false
  notes: Can request consent or settings input; do not assume unattended completion. Singular alias of extensions update.
- description: Validates an extension from a local path.
  name: extension validate
  non_interactive: true
  notes: Handler terminates without a user prompt; established from source, not by performing mutations. Singular alias of extensions validate.
- description: Manage extensions.
  name: extensions
  non_interactive: false
  notes: Command group requires a leaf command; no standalone operation.
- description: Configure extension settings.
  name: extensions config
  non_interactive: false
  notes: Can request consent or settings input; do not assume unattended completion.
- description: Disables an extension.
  name: extensions disable
  non_interactive: true
  notes: Handler terminates without a user prompt; established from source, not by performing mutations.
- description: Enables an extension.
  name: extensions enable
  non_interactive: true
  notes: Handler terminates without a user prompt; established from source, not by performing mutations.
- description: Installs an extension from a git repository URL or a local path.
  name: extensions install
  non_interactive: false
  notes: Unattended installation requires both --consent and --skip-settings; not exercised.
- description: Links an extension from a local path. Updates made to the local path will always be reflected.
  name: extensions link
  non_interactive: false
  notes: Can request consent or settings input; do not assume unattended completion.
- description: Lists installed extensions.
  name: extensions list
  non_interactive: true
  notes: Handler terminates without a user prompt; established from source, not by performing mutations.
- description: Create a new extension from a boilerplate example.
  name: extensions new
  non_interactive: true
  notes: Handler terminates without a user prompt; established from source, not by performing mutations.
- description: Uninstalls one or more extensions.
  name: extensions uninstall
  non_interactive: true
  notes: Handler terminates without a user prompt; established from source, not by performing mutations.
- description: Updates all extensions or a named extension to the latest version.
  name: extensions update
  non_interactive: false
  notes: Can request consent or settings input; do not assume unattended completion.
- description: Validates an extension from a local path.
  name: extensions validate
  non_interactive: true
  notes: Handler terminates without a user prompt; established from source, not by performing mutations.
- description: Manage local Gemma model routing.
  name: gemma
  non_interactive: false
  notes: Command group requires a leaf command; no standalone operation.
- description: View LiteRT-LM server logs
  name: gemma logs
  non_interactive: false
  notes: Follows indefinitely by default. Use --lines N --no-follow for a finite, unattended read.
- description: Download and configure Gemma local model routing
  name: gemma setup
  non_interactive: false
  notes: Downloads/configures a local server and requests consent unless --consent is supplied.
- description: Start the LiteRT-LM server
  name: gemma start
  non_interactive: true
  notes: Handler terminates without a user prompt; established from source, not by performing mutations. start/stop affect a local server; status is a finite diagnostic.
- description: Check Gemma local model routing status
  name: gemma status
  non_interactive: true
  notes: Handler terminates without a user prompt; established from source, not by performing mutations. start/stop affect a local server; status is a finite diagnostic.
- description: Stop the LiteRT-LM server
  name: gemma stop
  non_interactive: true
  notes: Handler terminates without a user prompt; established from source, not by performing mutations. start/stop affect a local server; status is a finite diagnostic.
- description: Alias of hooks.
  name: hook
  non_interactive: false
  notes: Command group requires a leaf command; no standalone operation.
- description: Migrate hooks from Claude Code to Gemini CLI
  name: hook migrate
  non_interactive: true
  notes: Handler terminates without a user prompt; established from source, not by performing mutations. --from-claude rewrites user/project hook settings. Singular alias of hooks migrate.
- description: Manage hooks.
  name: hooks
  non_interactive: false
  notes: Command group requires a leaf command; no standalone operation.
- description: Migrate hooks from Claude Code to Gemini CLI
  name: hooks migrate
  non_interactive: true
  notes: Handler terminates without a user prompt; established from source, not by performing mutations. --from-claude rewrites user/project hook settings.
- description: Manage MCP servers.
  name: mcp
  non_interactive: false
  notes: Command group requires a leaf command; no standalone operation.
- description: Add a server
  name: mcp add
  non_interactive: true
  notes: Handler terminates without a user prompt; established from source, not by performing mutations.
- description: Disable an MCP server
  name: mcp disable
  non_interactive: true
  notes: Handler terminates without a user prompt; established from source, not by performing mutations.
- description: Enable an MCP server
  name: mcp enable
  non_interactive: true
  notes: Handler terminates without a user prompt; established from source, not by performing mutations.
- description: List all configured MCP servers
  name: mcp list
  non_interactive: true
  notes: Handler terminates without a user prompt; established from source, not by performing mutations.
- description: Remove a server
  name: mcp remove
  non_interactive: true
  notes: Handler terminates without a user prompt; established from source, not by performing mutations.
- description: Alias of skills.
  name: skill
  non_interactive: false
  notes: Command group requires a leaf command; no standalone operation.
- description: Disables an agent skill.
  name: skill disable
  non_interactive: true
  notes: Handler terminates without a user prompt; established from source, not by performing mutations. Singular alias of skills disable.
- description: Enables an agent skill.
  name: skill enable
  non_interactive: true
  notes: Handler terminates without a user prompt; established from source, not by performing mutations. Singular alias of skills enable.
- description: Installs an agent skill from a git repository URL or a local path.
  name: skill install
  non_interactive: false
  notes: Can request consent or settings input; do not assume unattended completion. Singular alias of skills install.
- description: Links an agent skill from a local path. Updates to the source will be reflected immediately.
  name: skill link
  non_interactive: false
  notes: Can request consent or settings input; do not assume unattended completion. Singular alias of skills link.
- description: Lists discovered agent skills.
  name: skill list
  non_interactive: true
  notes: Handler terminates without a user prompt; established from source, not by performing mutations. Singular alias of skills list.
- description: Uninstalls an agent skill by name.
  name: skill uninstall
  non_interactive: true
  notes: Handler terminates without a user prompt; established from source, not by performing mutations. Singular alias of skills uninstall.
- description: Manage agent skills.
  name: skills
  non_interactive: false
  notes: Command group requires a leaf command; no standalone operation.
- description: Disables an agent skill.
  name: skills disable
  non_interactive: true
  notes: Handler terminates without a user prompt; established from source, not by performing mutations.
- description: Enables an agent skill.
  name: skills enable
  non_interactive: true
  notes: Handler terminates without a user prompt; established from source, not by performing mutations.
- description: Installs an agent skill from a git repository URL or a local path.
  name: skills install
  non_interactive: false
  notes: Can request consent or settings input; do not assume unattended completion.
- description: Links an agent skill from a local path. Updates to the source will be reflected immediately.
  name: skills link
  non_interactive: false
  notes: Can request consent or settings input; do not assume unattended completion.
- description: Lists discovered agent skills.
  name: skills list
  non_interactive: true
  notes: Handler terminates without a user prompt; established from source, not by performing mutations.
- description: Uninstalls an agent skill by name.
  name: skills uninstall
  non_interactive: true
  notes: Handler terminates without a user prompt; established from source, not by performing mutations.
cli_switches:
- aliases:
  - --is-command
  attachment: []
  description: Internal flag to indicate if a subcommand is being run
  evidence_ids:
  - root-source
  - parser-tests
  - latest-source
  flag: --isCommand
  invocation_scope:
  - applies_to: global
  notes: Hidden/internal option, included because the parser accepts it; not a stable wrapper API. Yargs also accepts explicit true/false (including a following boolean literal); this contract represents the ordinary valueless spelling.
  value_type: none
- aliases:
  - --no-is-command
  attachment: []
  description: Set isCommand to false using yargs negation.
  evidence_ids:
  - root-source
  - parser-tests
  - latest-source
  flag: --no-isCommand
  invocation_scope:
  - applies_to: global
  notes: Parser-generated negation; not an independently declared option. For nonboolean declarations, application coercions/choices may reject the false value; it consumes no following value.
  value_type: none
- aliases:
  - -d
  - --d
  attachment: []
  description: Run in debug mode (open debug console with F12)
  evidence_ids:
  - root-source
  - parser-tests
  - latest-source
  flag: --debug
  invocation_scope:
  - applies_to: global
  notes: Yargs also accepts explicit true/false (including a following boolean literal); this contract represents the ordinary valueless spelling.
  value_type: none
- aliases:
  - --no-d
  attachment: []
  description: Set debug to false using yargs negation.
  evidence_ids:
  - root-source
  - parser-tests
  - latest-source
  flag: --no-debug
  invocation_scope:
  - applies_to: global
  notes: Parser-generated negation; not an independently declared option. For nonboolean declarations, application coercions/choices may reject the false value; it consumes no following value.
  value_type: none
- aliases:
  - -m
  - --m
  attachment:
  - space
  - equals
  - short_attached
  description: Model
  evidence_ids:
  - root-source
  - parser-tests
  - latest-source
  flag: --model
  invocation_scope:
  - applies_to: command
    command: []
  notes: 'short_attached is value-dependent: numeric tails (for example -m123) are consumed, but alphabetic tails are parsed as short-option groups. Prefer space or equals; -x=VALUE also works.'
  value: <VALUE>
  value_optional: false
  value_type: string
- aliases:
  - --no-m
  attachment: []
  description: Set model to false using yargs negation.
  evidence_ids:
  - root-source
  - parser-tests
  - latest-source
  flag: --no-model
  invocation_scope:
  - applies_to: command
    command: []
  notes: Parser-generated negation; not an independently declared option. For nonboolean declarations, application coercions/choices may reject the false value; it consumes no following value.
  value_type: none
- aliases:
  - -p
  - --p
  attachment:
  - space
  - equals
  - short_attached
  description: Run in non-interactive (headless) mode with the given prompt. Appended to input on stdin (if any).
  evidence_ids:
  - root-source
  - parser-tests
  - latest-source
  flag: --prompt
  invocation_scope:
  - applies_to: command
    command: []
  notes: 'short_attached is value-dependent: numeric tails (for example -p123) are consumed, but alphabetic tails are parsed as short-option groups. Prefer space or equals; -x=VALUE also works.'
  value: <VALUE>
  value_optional: false
  value_type: string
- aliases:
  - --no-p
  attachment: []
  description: Set prompt to false using yargs negation.
  evidence_ids:
  - root-source
  - parser-tests
  - latest-source
  flag: --no-prompt
  invocation_scope:
  - applies_to: command
    command: []
  notes: Parser-generated negation; not an independently declared option. For nonboolean declarations, application coercions/choices may reject the false value; it consumes no following value.
  value_type: none
- aliases:
  - -i
  - --i
  - --promptInteractive
  attachment:
  - space
  - equals
  - short_attached
  description: Execute the provided prompt and continue in interactive mode
  evidence_ids:
  - root-source
  - parser-tests
  - latest-source
  flag: --prompt-interactive
  invocation_scope:
  - applies_to: command
    command: []
  notes: 'short_attached is value-dependent: numeric tails (for example -i123) are consumed, but alphabetic tails are parsed as short-option groups. Prefer space or equals; -x=VALUE also works.'
  value: <VALUE>
  value_optional: false
  value_type: string
- aliases:
  - --no-i
  - --no-promptInteractive
  attachment: []
  description: Set prompt-interactive to false using yargs negation.
  evidence_ids:
  - root-source
  - parser-tests
  - latest-source
  flag: --no-prompt-interactive
  invocation_scope:
  - applies_to: command
    command: []
  notes: Parser-generated negation; not an independently declared option. For nonboolean declarations, application coercions/choices may reject the false value; it consumes no following value.
  value_type: none
- aliases:
  - --skipTrust
  attachment: []
  description: Trust the current workspace for this session.
  evidence_ids:
  - root-source
  - parser-tests
  - latest-source
  flag: --skip-trust
  invocation_scope:
  - applies_to: command
    command: []
  notes: Yargs also accepts explicit true/false (including a following boolean literal); this contract represents the ordinary valueless spelling.
  value_type: none
- aliases:
  - --no-skipTrust
  attachment: []
  description: Set skip-trust to false using yargs negation.
  evidence_ids:
  - root-source
  - parser-tests
  - latest-source
  flag: --no-skip-trust
  invocation_scope:
  - applies_to: command
    command: []
  notes: Parser-generated negation; not an independently declared option. For nonboolean declarations, application coercions/choices may reject the false value; it consumes no following value.
  value_type: none
- aliases:
  - -w
  - --w
  attachment:
  - space
  - equals
  - short_attached
  description: Start Gemini in a new git worktree. If no name is provided, one is generated automatically.
  evidence_ids:
  - root-source
  - parser-tests
  - latest-source
  flag: --worktree
  invocation_scope:
  - applies_to: command
    command: []
  notes: 'short_attached is value-dependent: numeric tails (for example -w123) are consumed, but alphabetic tails are parsed as short-option groups. Prefer space or equals; -x=VALUE also works.'
  value: <VALUE>
  value_optional: true
  value_type: string
- aliases:
  - --no-w
  attachment: []
  description: Set worktree to false using yargs negation.
  evidence_ids:
  - root-source
  - parser-tests
  - latest-source
  flag: --no-worktree
  invocation_scope:
  - applies_to: command
    command: []
  notes: Parser-generated negation; not an independently declared option. For nonboolean declarations, application coercions/choices may reject the false value; it consumes no following value.
  value_type: none
- aliases:
  - -s
  - --s
  attachment: []
  description: Run in sandbox?
  evidence_ids:
  - root-source
  - parser-tests
  - latest-source
  flag: --sandbox
  invocation_scope:
  - applies_to: command
    command: []
  notes: Yargs also accepts explicit true/false (including a following boolean literal); this contract represents the ordinary valueless spelling.
  value_type: none
- aliases:
  - --no-s
  attachment: []
  description: Set sandbox to false using yargs negation.
  evidence_ids:
  - root-source
  - parser-tests
  - latest-source
  flag: --no-sandbox
  invocation_scope:
  - applies_to: command
    command: []
  notes: Parser-generated negation; not an independently declared option. For nonboolean declarations, application coercions/choices may reject the false value; it consumes no following value.
  value_type: none
- aliases:
  - -y
  - --y
  attachment: []
  description: Automatically accept all actions (aka YOLO mode, see https://www.youtube.com/watch?v=xvFZjo5PgG0 for more details)?
  evidence_ids:
  - root-source
  - parser-tests
  - latest-source
  flag: --yolo
  invocation_scope:
  - applies_to: command
    command: []
  notes: Yargs also accepts explicit true/false (including a following boolean literal); this contract represents the ordinary valueless spelling.
  value_type: none
- aliases:
  - --no-y
  attachment: []
  description: Set yolo to false using yargs negation.
  evidence_ids:
  - root-source
  - parser-tests
  - latest-source
  flag: --no-yolo
  invocation_scope:
  - applies_to: command
    command: []
  notes: Parser-generated negation; not an independently declared option. For nonboolean declarations, application coercions/choices may reject the false value; it consumes no following value.
  value_type: none
- aliases:
  - --approvalMode
  attachment:
  - space
  - equals
  description: 'Set the approval mode: default (prompt for approval), auto_edit (auto-approve edit tools), yolo (auto-approve all tools), plan (read-only mode)'
  evidence_ids:
  - root-source
  - parser-tests
  - latest-source
  flag: --approval-mode
  invocation_scope:
  - applies_to: command
    command: []
  value: <default|auto_edit|yolo|plan>
  value_optional: false
  value_type: string
- aliases:
  - --no-approvalMode
  attachment: []
  description: Set approval-mode to false using yargs negation.
  evidence_ids:
  - root-source
  - parser-tests
  - latest-source
  flag: --no-approval-mode
  invocation_scope:
  - applies_to: command
    command: []
  notes: Parser-generated negation; not an independently declared option. For nonboolean declarations, application coercions/choices may reject the false value; it consumes no following value.
  value_type: none
- attachment:
  - space
  - equals
  description: Additional policy files or directories to load (comma-separated or multiple --policy)
  evidence_ids:
  - root-source
  - parser-tests
  - latest-source
  flag: --policy
  invocation_scope:
  - applies_to: command
    command: []
  notes: Each occurrence consumes exactly one token; repeat the option for more values. Root array options additionally split commas.
  value: <VALUE>
  value_optional: false
  value_type: string
- attachment: []
  description: Set policy to false using yargs negation.
  evidence_ids:
  - root-source
  - parser-tests
  - latest-source
  flag: --no-policy
  invocation_scope:
  - applies_to: command
    command: []
  notes: Parser-generated negation; not an independently declared option. For nonboolean declarations, application coercions/choices may reject the false value; it consumes no following value.
  value_type: none
- aliases:
  - --adminPolicy
  attachment:
  - space
  - equals
  description: Additional admin policy files or directories to load (comma-separated or multiple --admin-policy)
  evidence_ids:
  - root-source
  - parser-tests
  - latest-source
  flag: --admin-policy
  invocation_scope:
  - applies_to: command
    command: []
  notes: Each occurrence consumes exactly one token; repeat the option for more values. Root array options additionally split commas.
  value: <VALUE>
  value_optional: false
  value_type: string
- aliases:
  - --no-adminPolicy
  attachment: []
  description: Set admin-policy to false using yargs negation.
  evidence_ids:
  - root-source
  - parser-tests
  - latest-source
  flag: --no-admin-policy
  invocation_scope:
  - applies_to: command
    command: []
  notes: Parser-generated negation; not an independently declared option. For nonboolean declarations, application coercions/choices may reject the false value; it consumes no following value.
  value_type: none
- attachment: []
  description: Starts the agent in ACP mode
  evidence_ids:
  - root-source
  - parser-tests
  - latest-source
  flag: --acp
  invocation_scope:
  - applies_to: command
    command: []
  notes: Yargs also accepts explicit true/false (including a following boolean literal); this contract represents the ordinary valueless spelling.
  value_type: none
- attachment: []
  description: Set acp to false using yargs negation.
  evidence_ids:
  - root-source
  - parser-tests
  - latest-source
  flag: --no-acp
  invocation_scope:
  - applies_to: command
    command: []
  notes: Parser-generated negation; not an independently declared option. For nonboolean declarations, application coercions/choices may reject the false value; it consumes no following value.
  value_type: none
- aliases:
  - --experimentalAcp
  attachment: []
  description: Starts the agent in ACP mode (deprecated, use --acp instead)
  evidence_ids:
  - root-source
  - parser-tests
  - latest-source
  flag: --experimental-acp
  invocation_scope:
  - applies_to: command
    command: []
  notes: Yargs also accepts explicit true/false (including a following boolean literal); this contract represents the ordinary valueless spelling.
  value_type: none
- aliases:
  - --no-experimentalAcp
  attachment: []
  description: Set experimental-acp to false using yargs negation.
  evidence_ids:
  - root-source
  - parser-tests
  - latest-source
  flag: --no-experimental-acp
  invocation_scope:
  - applies_to: command
    command: []
  notes: Parser-generated negation; not an independently declared option. For nonboolean declarations, application coercions/choices may reject the false value; it consumes no following value.
  value_type: none
- aliases:
  - --allowedMcpServerNames
  attachment:
  - space
  - equals
  description: Allowed MCP server names
  evidence_ids:
  - root-source
  - parser-tests
  - latest-source
  flag: --allowed-mcp-server-names
  invocation_scope:
  - applies_to: command
    command: []
  notes: Each occurrence consumes exactly one token; repeat the option for more values. Root array options additionally split commas.
  value: <VALUE>
  value_optional: false
  value_type: string
- aliases:
  - --no-allowedMcpServerNames
  attachment: []
  description: Set allowed-mcp-server-names to false using yargs negation.
  evidence_ids:
  - root-source
  - parser-tests
  - latest-source
  flag: --no-allowed-mcp-server-names
  invocation_scope:
  - applies_to: command
    command: []
  notes: Parser-generated negation; not an independently declared option. For nonboolean declarations, application coercions/choices may reject the false value; it consumes no following value.
  value_type: none
- aliases:
  - --allowedTools
  attachment:
  - space
  - equals
  description: '[DEPRECATED: Use Policy Engine instead See https://geminicli.com/docs/core/policy-engine] Tools that are allowed to run without confirmation'
  evidence_ids:
  - root-source
  - parser-tests
  - latest-source
  flag: --allowed-tools
  invocation_scope:
  - applies_to: command
    command: []
  notes: Each occurrence consumes exactly one token; repeat the option for more values. Root array options additionally split commas.
  value: <VALUE>
  value_optional: false
  value_type: string
- aliases:
  - --no-allowedTools
  attachment: []
  description: Set allowed-tools to false using yargs negation.
  evidence_ids:
  - root-source
  - parser-tests
  - latest-source
  flag: --no-allowed-tools
  invocation_scope:
  - applies_to: command
    command: []
  notes: Parser-generated negation; not an independently declared option. For nonboolean declarations, application coercions/choices may reject the false value; it consumes no following value.
  value_type: none
- aliases:
  - -e
  - --e
  attachment:
  - space
  - equals
  - short_attached
  description: A list of extensions to use. If not provided, all extensions are used.
  evidence_ids:
  - root-source
  - parser-tests
  - latest-source
  flag: --extensions
  invocation_scope:
  - applies_to: command
    command: []
  notes: 'short_attached is value-dependent: numeric tails (for example -e123) are consumed, but alphabetic tails are parsed as short-option groups. Prefer space or equals; -x=VALUE also works. Each occurrence consumes exactly one token; repeat the option for more values. Root array options additionally split commas.'
  value: <VALUE>
  value_optional: false
  value_type: string
- aliases:
  - --no-e
  attachment: []
  description: Set extensions to false using yargs negation.
  evidence_ids:
  - root-source
  - parser-tests
  - latest-source
  flag: --no-extensions
  invocation_scope:
  - applies_to: command
    command: []
  notes: Parser-generated negation; not an independently declared option. For nonboolean declarations, application coercions/choices may reject the false value; it consumes no following value.
  value_type: none
- aliases:
  - -l
  - --l
  - --listExtensions
  attachment: []
  description: List all available extensions and exit.
  evidence_ids:
  - root-source
  - parser-tests
  - latest-source
  flag: --list-extensions
  invocation_scope:
  - applies_to: command
    command: []
  notes: Yargs also accepts explicit true/false (including a following boolean literal); this contract represents the ordinary valueless spelling.
  value_type: none
- aliases:
  - --no-l
  - --no-listExtensions
  attachment: []
  description: Set list-extensions to false using yargs negation.
  evidence_ids:
  - root-source
  - parser-tests
  - latest-source
  flag: --no-list-extensions
  invocation_scope:
  - applies_to: command
    command: []
  notes: Parser-generated negation; not an independently declared option. For nonboolean declarations, application coercions/choices may reject the false value; it consumes no following value.
  value_type: none
- aliases:
  - -r
  - --r
  attachment:
  - space
  - equals
  - short_attached
  description: Resume a previous session. Use "latest" for most recent or index number (e.g. --resume 5)
  evidence_ids:
  - root-source
  - parser-tests
  - latest-source
  flag: --resume
  invocation_scope:
  - applies_to: command
    command: []
  notes: 'short_attached is value-dependent: numeric tails (for example -r123) are consumed, but alphabetic tails are parsed as short-option groups. Prefer space or equals; -x=VALUE also works.'
  value: <VALUE>
  value_optional: true
  value_type: string
- aliases:
  - --no-r
  attachment: []
  description: Set resume to false using yargs negation.
  evidence_ids:
  - root-source
  - parser-tests
  - latest-source
  flag: --no-resume
  invocation_scope:
  - applies_to: command
    command: []
  notes: Parser-generated negation; not an independently declared option. For nonboolean declarations, application coercions/choices may reject the false value; it consumes no following value.
  value_type: none
- aliases:
  - --sessionFile
  attachment:
  - space
  - equals
  description: Load a session from a JSON file
  evidence_ids:
  - root-source
  - parser-tests
  - latest-source
  flag: --session-file
  invocation_scope:
  - applies_to: command
    command: []
  value: <VALUE>
  value_optional: false
  value_type: string
- aliases:
  - --no-sessionFile
  attachment: []
  description: Set session-file to false using yargs negation.
  evidence_ids:
  - root-source
  - parser-tests
  - latest-source
  flag: --no-session-file
  invocation_scope:
  - applies_to: command
    command: []
  notes: Parser-generated negation; not an independently declared option. For nonboolean declarations, application coercions/choices may reject the false value; it consumes no following value.
  value_type: none
- aliases:
  - --sessionId
  attachment:
  - space
  - equals
  description: Start a new session with a manually provided UUID.
  evidence_ids:
  - root-source
  - parser-tests
  - latest-source
  - binary-tests
  flag: --session-id
  invocation_scope:
  - applies_to: command
    command: []
  value: <VALUE>
  value_optional: false
  value_type: string
- aliases:
  - --no-sessionId
  attachment: []
  description: Set session-id to false using yargs negation.
  evidence_ids:
  - root-source
  - parser-tests
  - latest-source
  - binary-tests
  flag: --no-session-id
  invocation_scope:
  - applies_to: command
    command: []
  notes: Parser-generated negation; not an independently declared option. For nonboolean declarations, application coercions/choices may reject the false value; it consumes no following value.
  value_type: none
- aliases:
  - --listSessions
  attachment: []
  description: List available sessions for the current project and exit.
  evidence_ids:
  - root-source
  - parser-tests
  - latest-source
  flag: --list-sessions
  invocation_scope:
  - applies_to: command
    command: []
  notes: Yargs also accepts explicit true/false (including a following boolean literal); this contract represents the ordinary valueless spelling.
  value_type: none
- aliases:
  - --no-listSessions
  attachment: []
  description: Set list-sessions to false using yargs negation.
  evidence_ids:
  - root-source
  - parser-tests
  - latest-source
  flag: --no-list-sessions
  invocation_scope:
  - applies_to: command
    command: []
  notes: Parser-generated negation; not an independently declared option. For nonboolean declarations, application coercions/choices may reject the false value; it consumes no following value.
  value_type: none
- aliases:
  - --deleteSession
  attachment:
  - space
  - equals
  description: Delete a session by index number (use --list-sessions to see available sessions).
  evidence_ids:
  - root-source
  - parser-tests
  - latest-source
  flag: --delete-session
  invocation_scope:
  - applies_to: command
    command: []
  value: <VALUE>
  value_optional: true
  value_type: string
- aliases:
  - --no-deleteSession
  attachment: []
  description: Set delete-session to false using yargs negation.
  evidence_ids:
  - root-source
  - parser-tests
  - latest-source
  flag: --no-delete-session
  invocation_scope:
  - applies_to: command
    command: []
  notes: Parser-generated negation; not an independently declared option. For nonboolean declarations, application coercions/choices may reject the false value; it consumes no following value.
  value_type: none
- aliases:
  - --includeDirectories
  attachment:
  - space
  - equals
  description: Additional directories to include in the workspace (comma-separated or multiple --include-directories)
  evidence_ids:
  - root-source
  - parser-tests
  - latest-source
  flag: --include-directories
  invocation_scope:
  - applies_to: command
    command: []
  notes: Each occurrence consumes exactly one token; repeat the option for more values. Root array options additionally split commas.
  value: <VALUE>
  value_optional: false
  value_type: string
- aliases:
  - --no-includeDirectories
  attachment: []
  description: Set include-directories to false using yargs negation.
  evidence_ids:
  - root-source
  - parser-tests
  - latest-source
  flag: --no-include-directories
  invocation_scope:
  - applies_to: command
    command: []
  notes: Parser-generated negation; not an independently declared option. For nonboolean declarations, application coercions/choices may reject the false value; it consumes no following value.
  value_type: none
- aliases:
  - --screenReader
  attachment: []
  description: Enable screen reader mode for accessibility.
  evidence_ids:
  - root-source
  - parser-tests
  - latest-source
  flag: --screen-reader
  invocation_scope:
  - applies_to: command
    command: []
  notes: Yargs also accepts explicit true/false (including a following boolean literal); this contract represents the ordinary valueless spelling.
  value_type: none
- aliases:
  - --no-screenReader
  attachment: []
  description: Set screen-reader to false using yargs negation.
  evidence_ids:
  - root-source
  - parser-tests
  - latest-source
  flag: --no-screen-reader
  invocation_scope:
  - applies_to: command
    command: []
  notes: Parser-generated negation; not an independently declared option. For nonboolean declarations, application coercions/choices may reject the false value; it consumes no following value.
  value_type: none
- aliases:
  - -o
  - --o
  - --outputFormat
  attachment:
  - space
  - equals
  - short_attached
  description: The format of the CLI output.
  evidence_ids:
  - root-source
  - parser-tests
  - latest-source
  - binary-tests
  flag: --output-format
  invocation_scope:
  - applies_to: command
    command: []
  notes: 'short_attached is value-dependent: numeric tails (for example -o123) are consumed, but alphabetic tails are parsed as short-option groups. Prefer space or equals; -x=VALUE also works.'
  value: <text|json|stream-json>
  value_optional: false
  value_type: string
- aliases:
  - --no-o
  - --no-outputFormat
  attachment: []
  description: Set output-format to false using yargs negation.
  evidence_ids:
  - root-source
  - parser-tests
  - latest-source
  - binary-tests
  flag: --no-output-format
  invocation_scope:
  - applies_to: command
    command: []
  notes: Parser-generated negation; not an independently declared option. For nonboolean declarations, application coercions/choices may reject the false value; it consumes no following value.
  value_type: none
- aliases:
  - --fakeResponses
  attachment:
  - space
  - equals
  description: Path to a file with fake model responses for testing.
  evidence_ids:
  - root-source
  - parser-tests
  - latest-source
  flag: --fake-responses
  invocation_scope:
  - applies_to: command
    command: []
  notes: Hidden/internal option, included because the parser accepts it; not a stable wrapper API.
  value: <VALUE>
  value_optional: true
  value_type: string
- aliases:
  - --no-fakeResponses
  attachment: []
  description: Set fake-responses to false using yargs negation.
  evidence_ids:
  - root-source
  - parser-tests
  - latest-source
  flag: --no-fake-responses
  invocation_scope:
  - applies_to: command
    command: []
  notes: Parser-generated negation; not an independently declared option. For nonboolean declarations, application coercions/choices may reject the false value; it consumes no following value.
  value_type: none
- aliases:
  - --fakeResponsesNonStrict
  attachment:
  - space
  - equals
  description: Path to a file with fake model responses for testing (non-strict mode).
  evidence_ids:
  - root-source
  - parser-tests
  - latest-source
  flag: --fake-responses-non-strict
  invocation_scope:
  - applies_to: command
    command: []
  notes: Hidden/internal option, included because the parser accepts it; not a stable wrapper API.
  value: <VALUE>
  value_optional: true
  value_type: string
- aliases:
  - --no-fakeResponsesNonStrict
  attachment: []
  description: Set fake-responses-non-strict to false using yargs negation.
  evidence_ids:
  - root-source
  - parser-tests
  - latest-source
  flag: --no-fake-responses-non-strict
  invocation_scope:
  - applies_to: command
    command: []
  notes: Parser-generated negation; not an independently declared option. For nonboolean declarations, application coercions/choices may reject the false value; it consumes no following value.
  value_type: none
- aliases:
  - --recordResponses
  attachment:
  - space
  - equals
  description: Path to a file to record model responses for testing.
  evidence_ids:
  - root-source
  - parser-tests
  - latest-source
  flag: --record-responses
  invocation_scope:
  - applies_to: command
    command: []
  notes: Hidden/internal option, included because the parser accepts it; not a stable wrapper API.
  value: <VALUE>
  value_optional: true
  value_type: string
- aliases:
  - --no-recordResponses
  attachment: []
  description: Set record-responses to false using yargs negation.
  evidence_ids:
  - root-source
  - parser-tests
  - latest-source
  flag: --no-record-responses
  invocation_scope:
  - applies_to: command
    command: []
  notes: Parser-generated negation; not an independently declared option. For nonboolean declarations, application coercions/choices may reject the false value; it consumes no following value.
  value_type: none
- aliases:
  - --rawOutput
  attachment: []
  description: 'Disable sanitization of model output (e.g. allow ANSI escape sequences). WARNING: This can be a security risk if the model output is untrusted.'
  evidence_ids:
  - root-source
  - parser-tests
  - latest-source
  flag: --raw-output
  invocation_scope:
  - applies_to: command
    command: []
  notes: Yargs also accepts explicit true/false (including a following boolean literal); this contract represents the ordinary valueless spelling.
  value_type: none
- aliases:
  - --no-rawOutput
  attachment: []
  description: Set raw-output to false using yargs negation.
  evidence_ids:
  - root-source
  - parser-tests
  - latest-source
  flag: --no-raw-output
  invocation_scope:
  - applies_to: command
    command: []
  notes: Parser-generated negation; not an independently declared option. For nonboolean declarations, application coercions/choices may reject the false value; it consumes no following value.
  value_type: none
- aliases:
  - --acceptRawOutputRisk
  attachment: []
  description: Suppress the security warning when using --raw-output.
  evidence_ids:
  - root-source
  - parser-tests
  - latest-source
  flag: --accept-raw-output-risk
  invocation_scope:
  - applies_to: command
    command: []
  notes: Yargs also accepts explicit true/false (including a following boolean literal); this contract represents the ordinary valueless spelling.
  value_type: none
- aliases:
  - --no-acceptRawOutputRisk
  attachment: []
  description: Set accept-raw-output-risk to false using yargs negation.
  evidence_ids:
  - root-source
  - parser-tests
  - latest-source
  flag: --no-accept-raw-output-risk
  invocation_scope:
  - applies_to: command
    command: []
  notes: Parser-generated negation; not an independently declared option. For nonboolean declarations, application coercions/choices may reject the false value; it consumes no following value.
  value_type: none
- aliases:
  - -h
  - --h
  attachment: []
  description: Show help.
  evidence_ids:
  - root-source
  - parser-tests
  - latest-source
  - binary-tests
  flag: --help
  invocation_scope:
  - applies_to: global
  notes: Yargs also accepts explicit true/false (including a following boolean literal); this contract represents the ordinary valueless spelling.
  value_type: none
- aliases:
  - --no-h
  attachment: []
  description: Set help to false using yargs negation.
  evidence_ids:
  - root-source
  - parser-tests
  - latest-source
  - binary-tests
  flag: --no-help
  invocation_scope:
  - applies_to: global
  notes: Parser-generated negation; not an independently declared option. For nonboolean declarations, application coercions/choices may reject the false value; it consumes no following value.
  value_type: none
- aliases:
  - -v
  - --v
  attachment: []
  description: Show version number.
  evidence_ids:
  - root-source
  - parser-tests
  - latest-source
  - binary-tests
  flag: --version
  invocation_scope:
  - applies_to: command
    command: []
  notes: Yargs also accepts explicit true/false (including a following boolean literal); this contract represents the ordinary valueless spelling.
  value_type: none
- aliases:
  - --no-v
  attachment: []
  description: Set version to false using yargs negation.
  evidence_ids:
  - root-source
  - parser-tests
  - latest-source
  - binary-tests
  flag: --no-version
  invocation_scope:
  - applies_to: command
    command: []
  notes: Parser-generated negation; not an independently declared option. For nonboolean declarations, application coercions/choices may reject the false value; it consumes no following value.
  value_type: none
- aliases:
  - -s
  - --s
  attachment:
  - space
  - equals
  - short_attached
  description: Configuration scope (user or project)
  evidence_ids:
  - commands-source
  - parser-tests
  flag: --scope
  invocation_scope:
  - applies_to: command
    command:
    - mcp
    - add
  notes: 'short_attached is value-dependent: numeric tails (for example -s123) are consumed, but alphabetic tails are parsed as short-option groups. Prefer space or equals; -x=VALUE also works.'
  value: <user|project>
  value_optional: false
  value_type: string
- aliases:
  - --no-s
  attachment: []
  description: Set scope to false using yargs negation.
  evidence_ids:
  - commands-source
  - parser-tests
  flag: --no-scope
  invocation_scope:
  - applies_to: command
    command:
    - mcp
    - add
  notes: Parser-generated negation; not an independently declared option. For nonboolean declarations, application coercions/choices may reject the false value; it consumes no following value.
  value_type: none
- aliases:
  - -t
  - --t
  - --type
  attachment:
  - space
  - equals
  - short_attached
  description: Transport type (stdio, sse, http)
  evidence_ids:
  - commands-source
  - parser-tests
  flag: --transport
  invocation_scope:
  - applies_to: command
    command:
    - mcp
    - add
  notes: 'short_attached is value-dependent: numeric tails (for example -t123) are consumed, but alphabetic tails are parsed as short-option groups. Prefer space or equals; -x=VALUE also works.'
  value: <stdio|sse|http>
  value_optional: false
  value_type: string
- aliases:
  - --no-t
  - --no-type
  attachment: []
  description: Set transport to false using yargs negation.
  evidence_ids:
  - commands-source
  - parser-tests
  flag: --no-transport
  invocation_scope:
  - applies_to: command
    command:
    - mcp
    - add
  notes: Parser-generated negation; not an independently declared option. For nonboolean declarations, application coercions/choices may reject the false value; it consumes no following value.
  value_type: none
- aliases:
  - -e
  - --e
  attachment:
  - space
  - equals
  - short_attached
  description: Set environment variables (e.g. -e KEY=value)
  evidence_ids:
  - commands-source
  - parser-tests
  flag: --env
  invocation_scope:
  - applies_to: command
    command:
    - mcp
    - add
  notes: 'short_attached is value-dependent: numeric tails (for example -e123) are consumed, but alphabetic tails are parsed as short-option groups. Prefer space or equals; -x=VALUE also works. Each occurrence consumes exactly one token; repeat the option for more values. Root array options additionally split commas.'
  value: <VALUE>
  value_optional: false
  value_type: string
- aliases:
  - --no-e
  attachment: []
  description: Set env to false using yargs negation.
  evidence_ids:
  - commands-source
  - parser-tests
  flag: --no-env
  invocation_scope:
  - applies_to: command
    command:
    - mcp
    - add
  notes: Parser-generated negation; not an independently declared option. For nonboolean declarations, application coercions/choices may reject the false value; it consumes no following value.
  value_type: none
- aliases:
  - -H
  - --H
  attachment:
  - space
  - equals
  - short_attached
  description: 'Set HTTP headers for SSE and HTTP transports (e.g. -H "X-Api-Key: abc123" -H "Authorization: Bearer abc123")'
  evidence_ids:
  - commands-source
  - parser-tests
  flag: --header
  invocation_scope:
  - applies_to: command
    command:
    - mcp
    - add
  notes: 'short_attached is value-dependent: numeric tails (for example -H123) are consumed, but alphabetic tails are parsed as short-option groups. Prefer space or equals; -x=VALUE also works. Each occurrence consumes exactly one token; repeat the option for more values. Root array options additionally split commas.'
  value: <VALUE>
  value_optional: false
  value_type: string
- aliases:
  - --no-H
  attachment: []
  description: Set header to false using yargs negation.
  evidence_ids:
  - commands-source
  - parser-tests
  flag: --no-header
  invocation_scope:
  - applies_to: command
    command:
    - mcp
    - add
  notes: Parser-generated negation; not an independently declared option. For nonboolean declarations, application coercions/choices may reject the false value; it consumes no following value.
  value_type: none
- attachment:
  - space
  - equals
  description: Set connection timeout in milliseconds
  evidence_ids:
  - commands-source
  - parser-tests
  flag: --timeout
  invocation_scope:
  - applies_to: command
    command:
    - mcp
    - add
  value: <N>
  value_optional: true
  value_type: number
- attachment: []
  description: Set timeout to zero using yargs negation.
  evidence_ids:
  - commands-source
  - parser-tests
  flag: --no-timeout
  invocation_scope:
  - applies_to: command
    command:
    - mcp
    - add
  notes: Parser-generated negation; not an independently declared option. For nonboolean declarations, application coercions/choices may reject the false value; it consumes no following value.
  value_type: none
- attachment: []
  description: Trust the server (bypass all tool call confirmation prompts)
  evidence_ids:
  - commands-source
  - parser-tests
  flag: --trust
  invocation_scope:
  - applies_to: command
    command:
    - mcp
    - add
  notes: Yargs also accepts explicit true/false (including a following boolean literal); this contract represents the ordinary valueless spelling.
  value_type: none
- attachment: []
  description: Set trust to false using yargs negation.
  evidence_ids:
  - commands-source
  - parser-tests
  flag: --no-trust
  invocation_scope:
  - applies_to: command
    command:
    - mcp
    - add
  notes: Parser-generated negation; not an independently declared option. For nonboolean declarations, application coercions/choices may reject the false value; it consumes no following value.
  value_type: none
- attachment:
  - space
  - equals
  description: Set the description for the server
  evidence_ids:
  - commands-source
  - parser-tests
  flag: --description
  invocation_scope:
  - applies_to: command
    command:
    - mcp
    - add
  value: <VALUE>
  value_optional: true
  value_type: string
- attachment: []
  description: Set description to false using yargs negation.
  evidence_ids:
  - commands-source
  - parser-tests
  flag: --no-description
  invocation_scope:
  - applies_to: command
    command:
    - mcp
    - add
  notes: Parser-generated negation; not an independently declared option. For nonboolean declarations, application coercions/choices may reject the false value; it consumes no following value.
  value_type: none
- aliases:
  - --includeTools
  attachment:
  - space
  - equals
  description: A comma-separated list of tools to include
  evidence_ids:
  - commands-source
  - parser-tests
  flag: --include-tools
  gap: 'The matched yargs parser accepts zero values ([]), but revision 2 permits only positive minima or unknown. Reproduce .option(''include-tools'', {type: ''array'', string: true}) with --include-tools alone; a contract extension allowing 0 is needed to encode the observed minimum.'
  invocation_scope:
  - applies_to: command
    command:
    - mcp
    - add
  value: <TOOL ...>
  value_type: variadic
  variadic_min: unknown
- aliases:
  - --no-includeTools
  attachment: []
  description: Set include-tools to false using yargs negation.
  evidence_ids:
  - commands-source
  - parser-tests
  flag: --no-include-tools
  invocation_scope:
  - applies_to: command
    command:
    - mcp
    - add
  notes: Parser-generated negation; not an independently declared option. For nonboolean declarations, application coercions/choices may reject the false value; it consumes no following value.
  value_type: none
- aliases:
  - --excludeTools
  attachment:
  - space
  - equals
  description: A comma-separated list of tools to exclude
  evidence_ids:
  - commands-source
  - parser-tests
  flag: --exclude-tools
  gap: 'The matched yargs parser accepts zero values ([]), but revision 2 permits only positive minima or unknown. Reproduce .option(''exclude-tools'', {type: ''array'', string: true}) with --exclude-tools alone; a contract extension allowing 0 is needed to encode the observed minimum.'
  invocation_scope:
  - applies_to: command
    command:
    - mcp
    - add
  value: <TOOL ...>
  value_type: variadic
  variadic_min: unknown
- aliases:
  - --no-excludeTools
  attachment: []
  description: Set exclude-tools to false using yargs negation.
  evidence_ids:
  - commands-source
  - parser-tests
  flag: --no-exclude-tools
  invocation_scope:
  - applies_to: command
    command:
    - mcp
    - add
  notes: Parser-generated negation; not an independently declared option. For nonboolean declarations, application coercions/choices may reject the false value; it consumes no following value.
  value_type: none
- attachment: []
  description: Clear session-only disable
  evidence_ids:
  - commands-source
  - parser-tests
  flag: --session
  invocation_scope:
  - applies_to: command
    command:
    - mcp
    - enable
  notes: Yargs also accepts explicit true/false (including a following boolean literal); this contract represents the ordinary valueless spelling.
  value_type: none
- attachment: []
  description: Set session to false using yargs negation.
  evidence_ids:
  - commands-source
  - parser-tests
  flag: --no-session
  invocation_scope:
  - applies_to: command
    command:
    - mcp
    - enable
  notes: Parser-generated negation; not an independently declared option. For nonboolean declarations, application coercions/choices may reject the false value; it consumes no following value.
  value_type: none
- attachment: []
  description: Disable for current session only
  evidence_ids:
  - commands-source
  - parser-tests
  flag: --session
  invocation_scope:
  - applies_to: command
    command:
    - mcp
    - disable
  notes: Yargs also accepts explicit true/false (including a following boolean literal); this contract represents the ordinary valueless spelling.
  value_type: none
- attachment: []
  description: Set session to false using yargs negation.
  evidence_ids:
  - commands-source
  - parser-tests
  flag: --no-session
  invocation_scope:
  - applies_to: command
    command:
    - mcp
    - disable
  notes: Parser-generated negation; not an independently declared option. For nonboolean declarations, application coercions/choices may reject the false value; it consumes no following value.
  value_type: none
- aliases:
  - -s
  - --s
  attachment:
  - space
  - equals
  - short_attached
  description: Configuration scope (user or project)
  evidence_ids:
  - commands-source
  - parser-tests
  flag: --scope
  invocation_scope:
  - applies_to: command
    command:
    - mcp
    - remove
  notes: 'short_attached is value-dependent: numeric tails (for example -s123) are consumed, but alphabetic tails are parsed as short-option groups. Prefer space or equals; -x=VALUE also works.'
  value: <user|project>
  value_optional: false
  value_type: string
- aliases:
  - --no-s
  attachment: []
  description: Set scope to false using yargs negation.
  evidence_ids:
  - commands-source
  - parser-tests
  flag: --no-scope
  invocation_scope:
  - applies_to: command
    command:
    - mcp
    - remove
  notes: Parser-generated negation; not an independently declared option. For nonboolean declarations, application coercions/choices may reject the false value; it consumes no following value.
  value_type: none
- attachment:
  - space
  - equals
  description: The scope to set the setting in.
  evidence_ids:
  - commands-source
  - parser-tests
  flag: --scope
  invocation_scope:
  - applies_to: command
    command:
    - extensions
    - config
  - applies_to: command
    command:
    - extension
    - config
  value: <user|workspace>
  value_optional: false
  value_type: string
- attachment: []
  description: Set scope to false using yargs negation.
  evidence_ids:
  - commands-source
  - parser-tests
  flag: --no-scope
  invocation_scope:
  - applies_to: command
    command:
    - extensions
    - config
  - applies_to: command
    command:
    - extension
    - config
  notes: Parser-generated negation; not an independently declared option. For nonboolean declarations, application coercions/choices may reject the false value; it consumes no following value.
  value_type: none
- attachment:
  - space
  - equals
  description: The scope to disable the extension in.
  evidence_ids:
  - commands-source
  - parser-tests
  flag: --scope
  invocation_scope:
  - applies_to: command
    command:
    - extensions
    - disable
  - applies_to: command
    command:
    - extension
    - disable
  value: <VALUE>
  value_optional: true
  value_type: string
- attachment: []
  description: Set scope to false using yargs negation.
  evidence_ids:
  - commands-source
  - parser-tests
  flag: --no-scope
  invocation_scope:
  - applies_to: command
    command:
    - extensions
    - disable
  - applies_to: command
    command:
    - extension
    - disable
  notes: Parser-generated negation; not an independently declared option. For nonboolean declarations, application coercions/choices may reject the false value; it consumes no following value.
  value_type: none
- attachment:
  - space
  - equals
  description: The scope to enable the extension in. If not set, will be enabled in all scopes.
  evidence_ids:
  - commands-source
  - parser-tests
  flag: --scope
  invocation_scope:
  - applies_to: command
    command:
    - extensions
    - enable
  - applies_to: command
    command:
    - extension
    - enable
  value: <VALUE>
  value_optional: true
  value_type: string
- attachment: []
  description: Set scope to false using yargs negation.
  evidence_ids:
  - commands-source
  - parser-tests
  flag: --no-scope
  invocation_scope:
  - applies_to: command
    command:
    - extensions
    - enable
  - applies_to: command
    command:
    - extension
    - enable
  notes: Parser-generated negation; not an independently declared option. For nonboolean declarations, application coercions/choices may reject the false value; it consumes no following value.
  value_type: none
- attachment:
  - space
  - equals
  description: The git ref to install from.
  evidence_ids:
  - commands-source
  - parser-tests
  flag: --ref
  invocation_scope:
  - applies_to: command
    command:
    - extensions
    - install
  - applies_to: command
    command:
    - extension
    - install
  value: <VALUE>
  value_optional: true
  value_type: string
- attachment: []
  description: Set ref to false using yargs negation.
  evidence_ids:
  - commands-source
  - parser-tests
  flag: --no-ref
  invocation_scope:
  - applies_to: command
    command:
    - extensions
    - install
  - applies_to: command
    command:
    - extension
    - install
  notes: Parser-generated negation; not an independently declared option. For nonboolean declarations, application coercions/choices may reject the false value; it consumes no following value.
  value_type: none
- aliases:
  - --autoUpdate
  attachment: []
  description: Enable auto-update for this extension.
  evidence_ids:
  - commands-source
  - parser-tests
  flag: --auto-update
  invocation_scope:
  - applies_to: command
    command:
    - extensions
    - install
  - applies_to: command
    command:
    - extension
    - install
  notes: Yargs also accepts explicit true/false (including a following boolean literal); this contract represents the ordinary valueless spelling.
  value_type: none
- aliases:
  - --no-autoUpdate
  attachment: []
  description: Set auto-update to false using yargs negation.
  evidence_ids:
  - commands-source
  - parser-tests
  flag: --no-auto-update
  invocation_scope:
  - applies_to: command
    command:
    - extensions
    - install
  - applies_to: command
    command:
    - extension
    - install
  notes: Parser-generated negation; not an independently declared option. For nonboolean declarations, application coercions/choices may reject the false value; it consumes no following value.
  value_type: none
- aliases:
  - --preRelease
  attachment: []
  description: Enable pre-release versions for this extension.
  evidence_ids:
  - commands-source
  - parser-tests
  flag: --pre-release
  invocation_scope:
  - applies_to: command
    command:
    - extensions
    - install
  - applies_to: command
    command:
    - extension
    - install
  notes: Yargs also accepts explicit true/false (including a following boolean literal); this contract represents the ordinary valueless spelling.
  value_type: none
- aliases:
  - --no-preRelease
  attachment: []
  description: Set pre-release to false using yargs negation.
  evidence_ids:
  - commands-source
  - parser-tests
  flag: --no-pre-release
  invocation_scope:
  - applies_to: command
    command:
    - extensions
    - install
  - applies_to: command
    command:
    - extension
    - install
  notes: Parser-generated negation; not an independently declared option. For nonboolean declarations, application coercions/choices may reject the false value; it consumes no following value.
  value_type: none
- attachment: []
  description: Acknowledge the security risks of installing an extension and skip the confirmation prompt.
  evidence_ids:
  - commands-source
  - parser-tests
  flag: --consent
  invocation_scope:
  - applies_to: command
    command:
    - extensions
    - install
  - applies_to: command
    command:
    - extension
    - install
  notes: Yargs also accepts explicit true/false (including a following boolean literal); this contract represents the ordinary valueless spelling.
  value_type: none
- attachment: []
  description: Set consent to false using yargs negation.
  evidence_ids:
  - commands-source
  - parser-tests
  flag: --no-consent
  invocation_scope:
  - applies_to: command
    command:
    - extensions
    - install
  - applies_to: command
    command:
    - extension
    - install
  notes: Parser-generated negation; not an independently declared option. For nonboolean declarations, application coercions/choices may reject the false value; it consumes no following value.
  value_type: none
- aliases:
  - --skipSettings
  attachment: []
  description: Skip the configuration on install process.
  evidence_ids:
  - commands-source
  - parser-tests
  flag: --skip-settings
  invocation_scope:
  - applies_to: command
    command:
    - extensions
    - install
  - applies_to: command
    command:
    - extension
    - install
  notes: Yargs also accepts explicit true/false (including a following boolean literal); this contract represents the ordinary valueless spelling.
  value_type: none
- aliases:
  - --no-skipSettings
  attachment: []
  description: Set skip-settings to false using yargs negation.
  evidence_ids:
  - commands-source
  - parser-tests
  flag: --no-skip-settings
  invocation_scope:
  - applies_to: command
    command:
    - extensions
    - install
  - applies_to: command
    command:
    - extension
    - install
  notes: Parser-generated negation; not an independently declared option. For nonboolean declarations, application coercions/choices may reject the false value; it consumes no following value.
  value_type: none
- attachment: []
  description: Acknowledge the security risks of installing an extension and skip the confirmation prompt.
  evidence_ids:
  - commands-source
  - parser-tests
  flag: --consent
  invocation_scope:
  - applies_to: command
    command:
    - extensions
    - link
  - applies_to: command
    command:
    - extension
    - link
  notes: Yargs also accepts explicit true/false (including a following boolean literal); this contract represents the ordinary valueless spelling.
  value_type: none
- attachment: []
  description: Set consent to false using yargs negation.
  evidence_ids:
  - commands-source
  - parser-tests
  flag: --no-consent
  invocation_scope:
  - applies_to: command
    command:
    - extensions
    - link
  - applies_to: command
    command:
    - extension
    - link
  notes: Parser-generated negation; not an independently declared option. For nonboolean declarations, application coercions/choices may reject the false value; it consumes no following value.
  value_type: none
- aliases:
  - -o
  - --o
  - --outputFormat
  attachment:
  - space
  - equals
  - short_attached
  description: The format of the CLI output.
  evidence_ids:
  - commands-source
  - parser-tests
  - binary-tests
  flag: --output-format
  invocation_scope:
  - applies_to: command
    command:
    - extensions
    - list
  - applies_to: command
    command:
    - extension
    - list
  notes: 'short_attached is value-dependent: numeric tails (for example -o123) are consumed, but alphabetic tails are parsed as short-option groups. Prefer space or equals; -x=VALUE also works.'
  value: <text|json>
  value_optional: false
  value_type: string
- aliases:
  - --no-o
  - --no-outputFormat
  attachment: []
  description: Set output-format to false using yargs negation.
  evidence_ids:
  - commands-source
  - parser-tests
  - binary-tests
  flag: --no-output-format
  invocation_scope:
  - applies_to: command
    command:
    - extensions
    - list
  - applies_to: command
    command:
    - extension
    - list
  notes: Parser-generated negation; not an independently declared option. For nonboolean declarations, application coercions/choices may reject the false value; it consumes no following value.
  value_type: none
- attachment: []
  description: Uninstall all installed extensions.
  evidence_ids:
  - commands-source
  - parser-tests
  flag: --all
  invocation_scope:
  - applies_to: command
    command:
    - extensions
    - uninstall
  - applies_to: command
    command:
    - extension
    - uninstall
  notes: Yargs also accepts explicit true/false (including a following boolean literal); this contract represents the ordinary valueless spelling.
  value_type: none
- attachment: []
  description: Set all to false using yargs negation.
  evidence_ids:
  - commands-source
  - parser-tests
  flag: --no-all
  invocation_scope:
  - applies_to: command
    command:
    - extensions
    - uninstall
  - applies_to: command
    command:
    - extension
    - uninstall
  notes: Parser-generated negation; not an independently declared option. For nonboolean declarations, application coercions/choices may reject the false value; it consumes no following value.
  value_type: none
- attachment: []
  description: Update all extensions.
  evidence_ids:
  - commands-source
  - parser-tests
  flag: --all
  invocation_scope:
  - applies_to: command
    command:
    - extensions
    - update
  - applies_to: command
    command:
    - extension
    - update
  notes: Yargs also accepts explicit true/false (including a following boolean literal); this contract represents the ordinary valueless spelling.
  value_type: none
- attachment: []
  description: Set all to false using yargs negation.
  evidence_ids:
  - commands-source
  - parser-tests
  flag: --no-all
  invocation_scope:
  - applies_to: command
    command:
    - extensions
    - update
  - applies_to: command
    command:
    - extension
    - update
  notes: Parser-generated negation; not an independently declared option. For nonboolean declarations, application coercions/choices may reject the false value; it consumes no following value.
  value_type: none
- aliases:
  - -s
  - --s
  attachment:
  - space
  - equals
  - short_attached
  description: The scope to disable the skill in (user or workspace).
  evidence_ids:
  - commands-source
  - parser-tests
  flag: --scope
  invocation_scope:
  - applies_to: command
    command:
    - skills
    - disable
  - applies_to: command
    command:
    - skill
    - disable
  notes: 'short_attached is value-dependent: numeric tails (for example -s123) are consumed, but alphabetic tails are parsed as short-option groups. Prefer space or equals; -x=VALUE also works.'
  value: <user|workspace>
  value_optional: false
  value_type: string
- aliases:
  - --no-s
  attachment: []
  description: Set scope to false using yargs negation.
  evidence_ids:
  - commands-source
  - parser-tests
  flag: --no-scope
  invocation_scope:
  - applies_to: command
    command:
    - skills
    - disable
  - applies_to: command
    command:
    - skill
    - disable
  notes: Parser-generated negation; not an independently declared option. For nonboolean declarations, application coercions/choices may reject the false value; it consumes no following value.
  value_type: none
- attachment:
  - space
  - equals
  description: The scope to install the skill into. Defaults to "user" (global).
  evidence_ids:
  - commands-source
  - parser-tests
  flag: --scope
  invocation_scope:
  - applies_to: command
    command:
    - skills
    - install
  - applies_to: command
    command:
    - skill
    - install
  value: <user|workspace>
  value_optional: false
  value_type: string
- attachment: []
  description: Set scope to false using yargs negation.
  evidence_ids:
  - commands-source
  - parser-tests
  flag: --no-scope
  invocation_scope:
  - applies_to: command
    command:
    - skills
    - install
  - applies_to: command
    command:
    - skill
    - install
  notes: Parser-generated negation; not an independently declared option. For nonboolean declarations, application coercions/choices may reject the false value; it consumes no following value.
  value_type: none
- attachment:
  - space
  - equals
  description: Sub-path within the repository to install from (only used for git repository sources).
  evidence_ids:
  - commands-source
  - parser-tests
  flag: --path
  invocation_scope:
  - applies_to: command
    command:
    - skills
    - install
  - applies_to: command
    command:
    - skill
    - install
  value: <VALUE>
  value_optional: true
  value_type: string
- attachment: []
  description: Set path to false using yargs negation.
  evidence_ids:
  - commands-source
  - parser-tests
  flag: --no-path
  invocation_scope:
  - applies_to: command
    command:
    - skills
    - install
  - applies_to: command
    command:
    - skill
    - install
  notes: Parser-generated negation; not an independently declared option. For nonboolean declarations, application coercions/choices may reject the false value; it consumes no following value.
  value_type: none
- attachment: []
  description: Acknowledge the security risks of installing a skill and skip the confirmation prompt.
  evidence_ids:
  - commands-source
  - parser-tests
  flag: --consent
  invocation_scope:
  - applies_to: command
    command:
    - skills
    - install
  - applies_to: command
    command:
    - skill
    - install
  notes: Yargs also accepts explicit true/false (including a following boolean literal); this contract represents the ordinary valueless spelling.
  value_type: none
- attachment: []
  description: Set consent to false using yargs negation.
  evidence_ids:
  - commands-source
  - parser-tests
  flag: --no-consent
  invocation_scope:
  - applies_to: command
    command:
    - skills
    - install
  - applies_to: command
    command:
    - skill
    - install
  notes: Parser-generated negation; not an independently declared option. For nonboolean declarations, application coercions/choices may reject the false value; it consumes no following value.
  value_type: none
- attachment:
  - space
  - equals
  description: The scope to link the skill into. Defaults to "user" (global).
  evidence_ids:
  - commands-source
  - parser-tests
  flag: --scope
  invocation_scope:
  - applies_to: command
    command:
    - skills
    - link
  - applies_to: command
    command:
    - skill
    - link
  value: <user|workspace>
  value_optional: false
  value_type: string
- attachment: []
  description: Set scope to false using yargs negation.
  evidence_ids:
  - commands-source
  - parser-tests
  flag: --no-scope
  invocation_scope:
  - applies_to: command
    command:
    - skills
    - link
  - applies_to: command
    command:
    - skill
    - link
  notes: Parser-generated negation; not an independently declared option. For nonboolean declarations, application coercions/choices may reject the false value; it consumes no following value.
  value_type: none
- attachment: []
  description: Acknowledge the security risks of linking a skill and skip the confirmation prompt.
  evidence_ids:
  - commands-source
  - parser-tests
  flag: --consent
  invocation_scope:
  - applies_to: command
    command:
    - skills
    - link
  - applies_to: command
    command:
    - skill
    - link
  notes: Yargs also accepts explicit true/false (including a following boolean literal); this contract represents the ordinary valueless spelling.
  value_type: none
- attachment: []
  description: Set consent to false using yargs negation.
  evidence_ids:
  - commands-source
  - parser-tests
  flag: --no-consent
  invocation_scope:
  - applies_to: command
    command:
    - skills
    - link
  - applies_to: command
    command:
    - skill
    - link
  notes: Parser-generated negation; not an independently declared option. For nonboolean declarations, application coercions/choices may reject the false value; it consumes no following value.
  value_type: none
- attachment: []
  description: Show all skills, including built-in ones.
  evidence_ids:
  - commands-source
  - parser-tests
  flag: --all
  invocation_scope:
  - applies_to: command
    command:
    - skills
    - list
  - applies_to: command
    command:
    - skill
    - list
  notes: Yargs also accepts explicit true/false (including a following boolean literal); this contract represents the ordinary valueless spelling.
  value_type: none
- attachment: []
  description: Set all to false using yargs negation.
  evidence_ids:
  - commands-source
  - parser-tests
  flag: --no-all
  invocation_scope:
  - applies_to: command
    command:
    - skills
    - list
  - applies_to: command
    command:
    - skill
    - list
  notes: Parser-generated negation; not an independently declared option. For nonboolean declarations, application coercions/choices may reject the false value; it consumes no following value.
  value_type: none
- attachment:
  - space
  - equals
  description: The scope to uninstall the skill from. Defaults to "user" (global).
  evidence_ids:
  - commands-source
  - parser-tests
  flag: --scope
  invocation_scope:
  - applies_to: command
    command:
    - skills
    - uninstall
  - applies_to: command
    command:
    - skill
    - uninstall
  value: <user|workspace>
  value_optional: false
  value_type: string
- attachment: []
  description: Set scope to false using yargs negation.
  evidence_ids:
  - commands-source
  - parser-tests
  flag: --no-scope
  invocation_scope:
  - applies_to: command
    command:
    - skills
    - uninstall
  - applies_to: command
    command:
    - skill
    - uninstall
  notes: Parser-generated negation; not an independently declared option. For nonboolean declarations, application coercions/choices may reject the false value; it consumes no following value.
  value_type: none
- aliases:
  - --fromClaude
  attachment: []
  description: Migrate from Claude Code hooks
  evidence_ids:
  - commands-source
  - parser-tests
  flag: --from-claude
  invocation_scope:
  - applies_to: command
    command:
    - hooks
    - migrate
  - applies_to: command
    command:
    - hook
    - migrate
  notes: Yargs also accepts explicit true/false (including a following boolean literal); this contract represents the ordinary valueless spelling.
  value_type: none
- aliases:
  - --no-fromClaude
  attachment: []
  description: Set from-claude to false using yargs negation.
  evidence_ids:
  - commands-source
  - parser-tests
  flag: --no-from-claude
  invocation_scope:
  - applies_to: command
    command:
    - hooks
    - migrate
  - applies_to: command
    command:
    - hook
    - migrate
  notes: Parser-generated negation; not an independently declared option. For nonboolean declarations, application coercions/choices may reject the false value; it consumes no following value.
  value_type: none
- aliases:
  - -n
  - --n
  attachment:
  - space
  - equals
  - short_attached
  description: Show the last N lines and exit (omit to follow live)
  evidence_ids:
  - commands-source
  - parser-tests
  flag: --lines
  invocation_scope:
  - applies_to: command
    command:
    - gemma
    - logs
  notes: 'short_attached is value-dependent: numeric tails (for example -n123) are consumed, but alphabetic tails are parsed as short-option groups. Prefer space or equals; -x=VALUE also works.'
  value: <N>
  value_optional: true
  value_type: number
- aliases:
  - --no-n
  attachment: []
  description: Set lines to zero using yargs negation.
  evidence_ids:
  - commands-source
  - parser-tests
  flag: --no-lines
  invocation_scope:
  - applies_to: command
    command:
    - gemma
    - logs
  notes: Parser-generated negation; not an independently declared option. For nonboolean declarations, application coercions/choices may reject the false value; it consumes no following value.
  value_type: none
- aliases:
  - -f
  - --f
  attachment: []
  description: Follow log output (defaults to true when --lines is omitted)
  evidence_ids:
  - commands-source
  - parser-tests
  flag: --follow
  invocation_scope:
  - applies_to: command
    command:
    - gemma
    - logs
  notes: Yargs also accepts explicit true/false (including a following boolean literal); this contract represents the ordinary valueless spelling.
  value_type: none
- aliases:
  - --no-f
  attachment: []
  description: Set follow to false using yargs negation.
  evidence_ids:
  - commands-source
  - parser-tests
  flag: --no-follow
  invocation_scope:
  - applies_to: command
    command:
    - gemma
    - logs
  notes: Parser-generated negation; not an independently declared option. For nonboolean declarations, application coercions/choices may reject the false value; it consumes no following value.
  value_type: none
- attachment:
  - space
  - equals
  description: Port for the LiteRT server
  evidence_ids:
  - commands-source
  - parser-tests
  flag: --port
  invocation_scope:
  - applies_to: command
    command:
    - gemma
    - setup
  value: <N>
  value_optional: true
  value_type: number
- attachment: []
  description: Set port to zero using yargs negation.
  evidence_ids:
  - commands-source
  - parser-tests
  flag: --no-port
  invocation_scope:
  - applies_to: command
    command:
    - gemma
    - setup
  notes: Parser-generated negation; not an independently declared option. For nonboolean declarations, application coercions/choices may reject the false value; it consumes no following value.
  value_type: none
- aliases:
  - --skipModel
  attachment: []
  description: Skip model download (binary only)
  evidence_ids:
  - commands-source
  - parser-tests
  flag: --skip-model
  invocation_scope:
  - applies_to: command
    command:
    - gemma
    - setup
  notes: Yargs also accepts explicit true/false (including a following boolean literal); this contract represents the ordinary valueless spelling.
  value_type: none
- aliases:
  - --no-skipModel
  attachment: []
  description: Set skip-model to false using yargs negation.
  evidence_ids:
  - commands-source
  - parser-tests
  flag: --no-skip-model
  invocation_scope:
  - applies_to: command
    command:
    - gemma
    - setup
  notes: Parser-generated negation; not an independently declared option. For nonboolean declarations, application coercions/choices may reject the false value; it consumes no following value.
  value_type: none
- attachment: []
  description: Start the server after setup
  evidence_ids:
  - commands-source
  - parser-tests
  flag: --start
  invocation_scope:
  - applies_to: command
    command:
    - gemma
    - setup
  notes: Yargs also accepts explicit true/false (including a following boolean literal); this contract represents the ordinary valueless spelling.
  value_type: none
- attachment: []
  description: Set start to false using yargs negation.
  evidence_ids:
  - commands-source
  - parser-tests
  flag: --no-start
  invocation_scope:
  - applies_to: command
    command:
    - gemma
    - setup
  notes: Parser-generated negation; not an independently declared option. For nonboolean declarations, application coercions/choices may reject the false value; it consumes no following value.
  value_type: none
- attachment: []
  description: Re-download binary and model even if already present
  evidence_ids:
  - commands-source
  - parser-tests
  flag: --force
  invocation_scope:
  - applies_to: command
    command:
    - gemma
    - setup
  notes: Yargs also accepts explicit true/false (including a following boolean literal); this contract represents the ordinary valueless spelling.
  value_type: none
- attachment: []
  description: Set force to false using yargs negation.
  evidence_ids:
  - commands-source
  - parser-tests
  flag: --no-force
  invocation_scope:
  - applies_to: command
    command:
    - gemma
    - setup
  notes: Parser-generated negation; not an independently declared option. For nonboolean declarations, application coercions/choices may reject the false value; it consumes no following value.
  value_type: none
- attachment: []
  description: Skip interactive consent prompt (implies acceptance)
  evidence_ids:
  - commands-source
  - parser-tests
  flag: --consent
  invocation_scope:
  - applies_to: command
    command:
    - gemma
    - setup
  notes: Yargs also accepts explicit true/false (including a following boolean literal); this contract represents the ordinary valueless spelling.
  value_type: none
- attachment: []
  description: Set consent to false using yargs negation.
  evidence_ids:
  - commands-source
  - parser-tests
  flag: --no-consent
  invocation_scope:
  - applies_to: command
    command:
    - gemma
    - setup
  notes: Parser-generated negation; not an independently declared option. For nonboolean declarations, application coercions/choices may reject the false value; it consumes no following value.
  value_type: none
- attachment:
  - space
  - equals
  description: Port for the LiteRT server
  evidence_ids:
  - commands-source
  - parser-tests
  flag: --port
  invocation_scope:
  - applies_to: command
    command:
    - gemma
    - start
  value: <N>
  value_optional: true
  value_type: number
- attachment: []
  description: Set port to zero using yargs negation.
  evidence_ids:
  - commands-source
  - parser-tests
  flag: --no-port
  invocation_scope:
  - applies_to: command
    command:
    - gemma
    - start
  notes: Parser-generated negation; not an independently declared option. For nonboolean declarations, application coercions/choices may reject the false value; it consumes no following value.
  value_type: none
- attachment:
  - space
  - equals
  description: Port to check for the LiteRT server
  evidence_ids:
  - commands-source
  - parser-tests
  flag: --port
  invocation_scope:
  - applies_to: command
    command:
    - gemma
    - status
  value: <N>
  value_optional: true
  value_type: number
- attachment: []
  description: Set port to zero using yargs negation.
  evidence_ids:
  - commands-source
  - parser-tests
  flag: --no-port
  invocation_scope:
  - applies_to: command
    command:
    - gemma
    - status
  notes: Parser-generated negation; not an independently declared option. For nonboolean declarations, application coercions/choices may reject the false value; it consumes no following value.
  value_type: none
- attachment:
  - space
  - equals
  description: Port where the LiteRT server is running
  evidence_ids:
  - commands-source
  - parser-tests
  flag: --port
  invocation_scope:
  - applies_to: command
    command:
    - gemma
    - stop
  value: <N>
  value_optional: true
  value_type: number
- attachment: []
  description: Set port to zero using yargs negation.
  evidence_ids:
  - commands-source
  - parser-tests
  flag: --no-port
  invocation_scope:
  - applies_to: command
    command:
    - gemma
    - stop
  notes: Parser-generated negation; not an independently declared option. For nonboolean declarations, application coercions/choices may reject the false value; it consumes no following value.
  value_type: none
config_paths:
- format: jsonc
  notes: Lowest-priority settings file; GEMINI_CLI_SYSTEM_DEFAULTS_PATH overrides path.
  os: macos
  path: /Library/Application Support/GeminiCli/system-defaults.json
  scope: system
- format: jsonc
  notes: User settings; GEMINI_CLI_HOME replaces home root. Settings commands may write this file.
  os: macos
  path: ~/.gemini/settings.json
  scope: user
- format: jsonc
  notes: Workspace settings; ignored when workspace is untrusted.
  os: macos
  path: .gemini/settings.json
  scope: repo
- format: jsonc
  notes: System override settings; GEMINI_CLI_SYSTEM_SETTINGS_PATH overrides path.
  os: macos
  path: /Library/Application Support/GeminiCli/settings.json
  scope: system
- format: text
  notes: Preferred at each trusted directory in upward .env search; first matching file wins.
  os: macos
  path: .gemini/.env
  scope: repo
- format: text
  notes: Fallback at each directory in upward search; trust and excluded-variable rules apply.
  os: macos
  path: .env
  scope: repo
- format: text
  notes: Home fallback for .env discovery.
  os: macos
  path: ~/.gemini/.env
  scope: user
- format: text
  notes: Generic home fallback for .env discovery.
  os: macos
  path: ~/.env
  scope: user
- format: text
  notes: Global instructional context; context.fileName can change the filename.
  os: macos
  path: ~/.gemini/GEMINI.md
  scope: user
- format: text
  notes: Hierarchical project instructional context; context.fileName can change the filename.
  os: macos
  path: GEMINI.md
  scope: repo
- format: text
  notes: Ignore patterns for file discovery.
  os: macos
  path: .geminiignore
  scope: repo
- format: json
  notes: Trust state; override owned by agent-permissions topic.
  os: macos
  path: ~/.gemini/trustedFolders.json
  scope: user
- format: jsonc
  notes: Lowest-priority settings file; GEMINI_CLI_SYSTEM_DEFAULTS_PATH overrides path.
  os: linux
  path: /etc/gemini-cli/system-defaults.json
  scope: system
- format: jsonc
  notes: User settings; GEMINI_CLI_HOME replaces home root. Settings commands may write this file.
  os: linux
  path: ~/.gemini/settings.json
  scope: user
- format: jsonc
  notes: Workspace settings; ignored when workspace is untrusted.
  os: linux
  path: .gemini/settings.json
  scope: repo
- format: jsonc
  notes: System override settings; GEMINI_CLI_SYSTEM_SETTINGS_PATH overrides path.
  os: linux
  path: /etc/gemini-cli/settings.json
  scope: system
- format: text
  notes: Preferred at each trusted directory in upward .env search; first matching file wins.
  os: linux
  path: .gemini/.env
  scope: repo
- format: text
  notes: Fallback at each directory in upward search; trust and excluded-variable rules apply.
  os: linux
  path: .env
  scope: repo
- format: text
  notes: Home fallback for .env discovery.
  os: linux
  path: ~/.gemini/.env
  scope: user
- format: text
  notes: Generic home fallback for .env discovery.
  os: linux
  path: ~/.env
  scope: user
- format: text
  notes: Global instructional context; context.fileName can change the filename.
  os: linux
  path: ~/.gemini/GEMINI.md
  scope: user
- format: text
  notes: Hierarchical project instructional context; context.fileName can change the filename.
  os: linux
  path: GEMINI.md
  scope: repo
- format: text
  notes: Ignore patterns for file discovery.
  os: linux
  path: .geminiignore
  scope: repo
- format: json
  notes: Trust state; override owned by agent-permissions topic.
  os: linux
  path: ~/.gemini/trustedFolders.json
  scope: user
- format: jsonc
  notes: Lowest-priority settings file; GEMINI_CLI_SYSTEM_DEFAULTS_PATH overrides path.
  os: windows
  path: C:\ProgramData\gemini-cli\system-defaults.json
  scope: system
- format: jsonc
  notes: User settings; GEMINI_CLI_HOME replaces home root. Settings commands may write this file.
  os: windows
  path: '%USERPROFILE%\.gemini\settings.json'
  scope: user
- format: jsonc
  notes: Workspace settings; ignored when workspace is untrusted.
  os: windows
  path: .gemini\settings.json
  scope: repo
- format: jsonc
  notes: System override settings; GEMINI_CLI_SYSTEM_SETTINGS_PATH overrides path.
  os: windows
  path: C:\ProgramData\gemini-cli\settings.json
  scope: system
- format: text
  notes: Preferred at each trusted directory in upward .env search; first matching file wins.
  os: windows
  path: .gemini\.env
  scope: repo
- format: text
  notes: Fallback at each directory in upward search; trust and excluded-variable rules apply.
  os: windows
  path: .env
  scope: repo
- format: text
  notes: Home fallback for .env discovery.
  os: windows
  path: '%USERPROFILE%\.gemini\.env'
  scope: user
- format: text
  notes: Generic home fallback for .env discovery.
  os: windows
  path: '%USERPROFILE%\.env'
  scope: user
- format: text
  notes: Global instructional context; context.fileName can change the filename.
  os: windows
  path: '%USERPROFILE%\.gemini\GEMINI.md'
  scope: user
- format: text
  notes: Hierarchical project instructional context; context.fileName can change the filename.
  os: windows
  path: GEMINI.md
  scope: repo
- format: text
  notes: Ignore patterns for file discovery.
  os: windows
  path: .geminiignore
  scope: repo
- format: json
  notes: Trust state; override owned by agent-permissions topic.
  os: windows
  path: '%USERPROFILE%\.gemini\trustedFolders.json'
  scope: user
env_vars:
- effect: Replaces the home root used for .gemini configuration and storage (not the .gemini directory itself).
  name: GEMINI_CLI_HOME
- effect: Selects the system defaults settings file.
  name: GEMINI_CLI_SYSTEM_DEFAULTS_PATH
- effect: Selects the system override settings file.
  name: GEMINI_CLI_SYSTEM_SETTINGS_PATH
- effect: Any nonempty value prevents the normal child relaunch used for process/memory setup.
  name: GEMINI_CLI_NO_RELAUNCH
- effect: Internal relaunch marker preventing repeated worktree setup; wrappers should not set it.
  name: GEMINI_CLI_WORKTREE_HANDLED
- effect: Overrides the extension registry URI used by extension configuration.
  name: GEMINI_CLI_EXTENSION_REGISTRY_URI
- effect: Identifies the IDE process for integration discovery.
  name: GEMINI_CLI_IDE_PID
- effect: Provides the IDE workspace path.
  name: GEMINI_CLI_IDE_WORKSPACE_PATH
- effect: Provides the companion IDE server port.
  name: GEMINI_CLI_IDE_SERVER_PORT
- effect: Provides the IDE connection authentication token; do not log it.
  name: GEMINI_CLI_IDE_AUTH_TOKEN
- effect: Overrides the IDE companion command for stdio transport.
  name: GEMINI_CLI_IDE_SERVER_STDIO_COMMAND
- effect: Provides companion command arguments as a JSON array string.
  name: GEMINI_CLI_IDE_SERVER_STDIO_ARGS
- effect: Overrides the context shown in the terminal window title.
  name: CLI_TITLE
- effect: A nonempty value selects the no-color theme and disables theme configuration.
  name: NO_COLOR
- effect: A nonempty value disables automatic browser opening; authentication can still need manual input.
  name: NO_BROWSER
- effect: Preferred external editor command when no editor is selected in settings.
  name: VISUAL
- effect: Fallback external editor command after VISUAL.
  name: EDITOR
- effect: Forces generic keybinding hints instead of terminal-specific hints.
  name: FORCE_GENERIC_KEYBINDING_HINTS
- effect: Fallback network proxy, after HTTPS_PROXY and https_proxy.
  name: HTTP_PROXY
- effect: Preferred proxy URL for CLI requests.
  name: HTTPS_PROXY
- effect: Lowercase HTTP proxy fallback after HTTP_PROXY.
  name: http_proxy
- effect: Lowercase HTTPS proxy fallback after HTTPS_PROXY.
  name: https_proxy
- effect: Comma-separated destinations that bypass proxy selection.
  name: NO_PROXY
- effect: Lowercase fallback for NO_PROXY.
  name: no_proxy
- effect: Participates in terminal capability detection.
  name: TERM
- effect: Identifies terminal host for UI and keybinding behavior.
  name: TERM_PROGRAM
- effect: Participates in terminal-specific feature detection.
  name: TERM_PROGRAM_VERSION
- effect: Participates in terminal color-capability detection.
  name: COLORTERM
- effect: Identifies a tmux environment during terminal detection.
  name: TMUX
- effect: Identifies a screen environment during terminal detection.
  name: STY
- effect: Participates in CI/environment detection and non-user runtime behavior.
  name: CI
- effect: Node runtime options; CLI relaunch and sandbox logic handle this variable.
  name: NODE_OPTIONS
- effect: Executable search path for CLI and tools; do not hard-code the installation location.
  name: PATH
machine_introspection:
- command: gemini extensions list --output-format json
  machine_readable: true
  notes: Installed 0.61.0 returned [] on stderr, with empty stdout in isolated state. Source uses debugLogger.log(JSON.stringify(...)); capture channels separately and enforce a timeout. Lists installed extensions, not provider capabilities.
  output_format: json
  purpose: plugins
  useful_for_codegen: false
- command: gemini --list-sessions
  machine_readable: false
  notes: Project-specific human-readable session list; isolated probe returned no sessions on stdout.
  output_format: text
  purpose: other
  useful_for_codegen: false
- command: gemini mcp list
  machine_readable: false
  notes: Human-readable configured-server state; isolated probe printed no servers on stderr. No JSON option declared.
  output_format: text
  purpose: mcp
  useful_for_codegen: false
- command: gemini skills list --all
  machine_readable: false
  notes: Human-readable discovered skills, including built-ins; successful probe had startup warnings on stderr.
  output_format: text
  purpose: other
  useful_for_codegen: false
- command: gemini gemma status
  machine_readable: false
  notes: Finite local-server diagnostic; isolated probe returned text and exit 1 because the local binary was not installed.
  output_format: text
  purpose: doctor
  useful_for_codegen: false
wrapper_notes:
- Installed gemini --version is 0.61.0; upstream GitHub latest and npm latest are 0.62.0. Declarations match, but runtime evidence is macOS 0.61.0 only.
- Use --prompt VALUE for an explicit headless run; positional queries enter interactive mode with a TTY. Resume is a root --resume/-r option, not a subcommand.
- 'Root array switches use nargs: 1 and consume one token per occurrence. MCP add include-tools/exclude-tools are greedy arrays and also accept zero values; the contract cannot express minimum zero.'
- 'Yargs default short-option groups make alphabetic attached values unsafe: -phello is not a prompt value. Numeric tails are consumed. Prefer -p VALUE or --prompt=VALUE.'
- Yargs generates camelCase and double-dash short aliases, boolean negations, and accepts explicit boolean literals. Ordinary boolean records are valueless; literal true/false consumption is not expressible in revision 2.
- extensions list --output-format json emitted JSON to stderr and no stdout. Capture both streams separately; never assume successful machine introspection is on stdout.
- Management commands may request consent or extension settings, write settings, download packages, or start/stop a server. Help succeeds without performing those operations.
- GEMINI_CLI_HOME provides a dedicated .gemini root; isolate system settings overrides and CWD as well when making disposable probes.
- Use argv arrays and preserve --. MCP add forwards unknown options and arguments after -- to the configured server command.
- raw-output disables terminal sanitization; accept-raw-output-risk only suppresses its warning. Keep terminal sanitization when rendering untrusted model output.
- Workspace trust and tool policy can make headless runs fail; do not add trust/approval bypasses implicitly. Permission environment variables belong to agent-permissions.
- List/status commands need deadlines. Help at all 52 paths completed in this refresh; previous help timeouts did not recur.
- Installed settings.json has top-level keys general, hooks, security, tools, ui. Credentials and account/state files exist under ~/.gemini; no secret values were printed.
- No system-prompt delivery switch is declared in either examined root parser. GEMINI_SYSTEM_MD and GEMINI_WRITE_SYSTEM_MD are environment-string controls (boolean-like string or path); semantics belong to system-prompt.
changes:
- Converted the old untyped inventory to schema revision 2 with evidence, exact command scopes, aliases, attachment forms, and optional-value records.
- Verified installed 0.61.0 and upstream stable 0.62.0; compared tagged parser/command declarations.
- Expanded to all 52 command paths including singular aliases; removed fictitious default and unverified update paths.
- Corrected repeated root arrays to single-value consumption, documented short attachment restrictions, and represented zero-minimum variadic arrays as a contract gap.
- Observed extension JSON on stderr and successful bounded help; removed stale claims of broken long flags and universal help hangs.
- Moved permission/model/MCP/logging environment details outside this topic; refreshed configuration and sanitized local observations.
requires_claudine_update: true
reason: The revision-2 catalog must use the verified single-token root arrays, optional resume/worktree values, command scopes and parser aliases. Wrapper introspection must handle extension JSON on stderr. The contract needs a way to represent zero-minimum arrays and optional boolean-literal consumption.
contract_checked: 2026-10-01
---

# Gemini CLI Public Command-Line Surface

## Overview

Gemini CLI is Google's open-source terminal agent. The `gemini` executable starts an interactive interface by default; `--prompt` selects an unattended model run. Installed `gemini --version` returned **0.61.0** on 2026-10-01. GitHub's latest stable release API and npm's `latest` package metadata both reported **0.62.0**, released September 29. Tagged root parser and command declarations are identical in these two releases.

[Homepage](https://geminicli.com/) · [Repository](https://github.com/google-gemini/gemini-cli) · [Documentation](https://geminicli.com/docs/) · [CLI reference](https://geminicli.com/docs/cli/cli-reference/).

## Installation and Binaries

The npm package declares `gemini: bundle/gemini.js` and Node.js `>=20`. The installed macOS executable resolves to `/Users/ken/.nvm/versions/node/v22.20.0/lib/node_modules/@google/gemini-cli/bundle/gemini.js`. Official commands use `gemini` on macOS, Linux, and Windows; Windows npm installations ordinarily provide `gemini.cmd` and `gemini.ps1` shims. Windows/Linux execution was not tested in this research.

| Operating systems | Method | Command |
| --- | --- | --- |
| macos, linux, windows | npm | `npm install -g @google/gemini-cli` |
| macos, linux, windows | npm | `npx @google/gemini-cli` |
| macos, linux | brew | `brew install gemini-cli` |
| macos | package_manager | `sudo port install gemini-cli` |
| macos, linux, windows | other | `conda create -y -n gemini_env -c conda-forge nodejs && conda activate gemini_env && npm install -g @google/gemini-cli` |
| macos, linux, windows | other | `docker run --rm -it us-docker.pkg.dev/gemini-code-dev/gemini-cli/sandbox:0.42.0-nightly.20260428.g59b2dea0e` |
| macos, linux, windows | source | `npm run start` |

The source workflow also documents `npm run start:prod` and `npm link packages/cli`. npm stable/preview/nightly channels exist; stable is the default. The documented container example uses `-it` and is interactive. Recommended baselines are macOS 15+, Windows 11 24H2+, and Ubuntu 20.04+. See [tagged installation instructions](https://github.com/google-gemini/gemini-cli/blob/v0.61.0/docs/get-started/installation.mdx).

## Subcommands

Every native path below `gemini` is listed here, including aliases. The bare executable is not a subcommand. `true` means the handler terminates without a terminal, browser, or answered prompt, although it can fail or perform a mutation. Group entries require a leaf command. All 52 paths were inspected with `gemini <path> --help`, stdin closed, and an eight-second deadline; every help call exited 0. Help establishes availability, not unattended handler behavior.

| Path | Non-interactive | Purpose / conditions |
| --- | --- | --- |
| `extension` | false | Alias of extensions. Command group requires a leaf command; no standalone operation. |
| `extension config` | false | Configure extension settings. Can request consent or settings input; do not assume unattended completion. Singular alias of extensions config. |
| `extension disable` | true | Disables an extension. Handler terminates without a user prompt; established from source, not by performing mutations. Singular alias of extensions disable. |
| `extension enable` | true | Enables an extension. Handler terminates without a user prompt; established from source, not by performing mutations. Singular alias of extensions enable. |
| `extension install` | false | Installs an extension from a git repository URL or a local path. Unattended installation requires both --consent and --skip-settings; not exercised. Singular alias of extensions install. |
| `extension link` | false | Links an extension from a local path. Updates made to the local path will always be reflected. Can request consent or settings input; do not assume unattended completion. Singular alias of extensions link. |
| `extension list` | true | Lists installed extensions. Handler terminates without a user prompt; established from source, not by performing mutations. Singular alias of extensions list. |
| `extension new` | true | Create a new extension from a boilerplate example. Handler terminates without a user prompt; established from source, not by performing mutations. Singular alias of extensions new. |
| `extension uninstall` | true | Uninstalls one or more extensions. Handler terminates without a user prompt; established from source, not by performing mutations. Singular alias of extensions uninstall. |
| `extension update` | false | Updates all extensions or a named extension to the latest version. Can request consent or settings input; do not assume unattended completion. Singular alias of extensions update. |
| `extension validate` | true | Validates an extension from a local path. Handler terminates without a user prompt; established from source, not by performing mutations. Singular alias of extensions validate. |
| `extensions` | false | Manage extensions. Command group requires a leaf command; no standalone operation. |
| `extensions config` | false | Configure extension settings. Can request consent or settings input; do not assume unattended completion. |
| `extensions disable` | true | Disables an extension. Handler terminates without a user prompt; established from source, not by performing mutations. |
| `extensions enable` | true | Enables an extension. Handler terminates without a user prompt; established from source, not by performing mutations. |
| `extensions install` | false | Installs an extension from a git repository URL or a local path. Unattended installation requires both --consent and --skip-settings; not exercised. |
| `extensions link` | false | Links an extension from a local path. Updates made to the local path will always be reflected. Can request consent or settings input; do not assume unattended completion. |
| `extensions list` | true | Lists installed extensions. Handler terminates without a user prompt; established from source, not by performing mutations. |
| `extensions new` | true | Create a new extension from a boilerplate example. Handler terminates without a user prompt; established from source, not by performing mutations. |
| `extensions uninstall` | true | Uninstalls one or more extensions. Handler terminates without a user prompt; established from source, not by performing mutations. |
| `extensions update` | false | Updates all extensions or a named extension to the latest version. Can request consent or settings input; do not assume unattended completion. |
| `extensions validate` | true | Validates an extension from a local path. Handler terminates without a user prompt; established from source, not by performing mutations. |
| `gemma` | false | Manage local Gemma model routing. Command group requires a leaf command; no standalone operation. |
| `gemma logs` | false | View LiteRT-LM server logs Follows indefinitely by default. Use --lines N --no-follow for a finite, unattended read. |
| `gemma setup` | false | Download and configure Gemma local model routing Downloads/configures a local server and requests consent unless --consent is supplied. |
| `gemma start` | true | Start the LiteRT-LM server Handler terminates without a user prompt; established from source, not by performing mutations. start/stop affect a local server; status is a finite diagnostic. |
| `gemma status` | true | Check Gemma local model routing status Handler terminates without a user prompt; established from source, not by performing mutations. start/stop affect a local server; status is a finite diagnostic. |
| `gemma stop` | true | Stop the LiteRT-LM server Handler terminates without a user prompt; established from source, not by performing mutations. start/stop affect a local server; status is a finite diagnostic. |
| `hook` | false | Alias of hooks. Command group requires a leaf command; no standalone operation. |
| `hook migrate` | true | Migrate hooks from Claude Code to Gemini CLI Handler terminates without a user prompt; established from source, not by performing mutations. --from-claude rewrites user/project hook settings. Singular alias of hooks migrate. |
| `hooks` | false | Manage hooks. Command group requires a leaf command; no standalone operation. |
| `hooks migrate` | true | Migrate hooks from Claude Code to Gemini CLI Handler terminates without a user prompt; established from source, not by performing mutations. --from-claude rewrites user/project hook settings. |
| `mcp` | false | Manage MCP servers. Command group requires a leaf command; no standalone operation. |
| `mcp add` | true | Add a server Handler terminates without a user prompt; established from source, not by performing mutations. |
| `mcp disable` | true | Disable an MCP server Handler terminates without a user prompt; established from source, not by performing mutations. |
| `mcp enable` | true | Enable an MCP server Handler terminates without a user prompt; established from source, not by performing mutations. |
| `mcp list` | true | List all configured MCP servers Handler terminates without a user prompt; established from source, not by performing mutations. |
| `mcp remove` | true | Remove a server Handler terminates without a user prompt; established from source, not by performing mutations. |
| `skill` | false | Alias of skills. Command group requires a leaf command; no standalone operation. |
| `skill disable` | true | Disables an agent skill. Handler terminates without a user prompt; established from source, not by performing mutations. Singular alias of skills disable. |
| `skill enable` | true | Enables an agent skill. Handler terminates without a user prompt; established from source, not by performing mutations. Singular alias of skills enable. |
| `skill install` | false | Installs an agent skill from a git repository URL or a local path. Can request consent or settings input; do not assume unattended completion. Singular alias of skills install. |
| `skill link` | false | Links an agent skill from a local path. Updates to the source will be reflected immediately. Can request consent or settings input; do not assume unattended completion. Singular alias of skills link. |
| `skill list` | true | Lists discovered agent skills. Handler terminates without a user prompt; established from source, not by performing mutations. Singular alias of skills list. |
| `skill uninstall` | true | Uninstalls an agent skill by name. Handler terminates without a user prompt; established from source, not by performing mutations. Singular alias of skills uninstall. |
| `skills` | false | Manage agent skills. Command group requires a leaf command; no standalone operation. |
| `skills disable` | true | Disables an agent skill. Handler terminates without a user prompt; established from source, not by performing mutations. |
| `skills enable` | true | Enables an agent skill. Handler terminates without a user prompt; established from source, not by performing mutations. |
| `skills install` | false | Installs an agent skill from a git repository URL or a local path. Can request consent or settings input; do not assume unattended completion. |
| `skills link` | false | Links an agent skill from a local path. Updates to the source will be reflected immediately. Can request consent or settings input; do not assume unattended completion. |
| `skills list` | true | Lists discovered agent skills. Handler terminates without a user prompt; established from source, not by performing mutations. |
| `skills uninstall` | true | Uninstalls an agent skill by name. Handler terminates without a user prompt; established from source, not by performing mutations. |

There is no native `resume`, `default`, `exec`, `login`, or `update` path in either examined command registry. `/` commands typed inside the interactive interface are not process subcommands.

## CLI Switch Inventory

Inventoried the root and every management leaf, including paths that can prompt. Root options live in [`parseArguments`](https://github.com/google-gemini/gemini-cli/blob/v0.61.0/packages/cli/src/config/config.ts); management options live in the [command modules](https://github.com/google-gemini/gemini-cli/tree/v0.61.0/packages/cli/src/commands). Parsing uses **yargs 17.7.2**, with **yargs-parser 21.1.1**, default camel-case expansion and short-option grouping. Root-specific switches do not become global merely because they appear before a command. Only `--debug`, `--help`, and hidden `--isCommand` are declared outside the default command and available across all paths; management builders disable version.

The tables give every explicit spelling and accepted parser-generated alias. Singular command aliases accept the same leaf switches as their plural spelling. Paths with no leaf options inherit only the global flags. Parser-generated negation records are included in frontmatter; each `--no-NAME` sets its option to false (numeric declarations coerce it to zero) and accepts the corresponding camelCase and double-dash short alias shown by its positive record. Negation also recognizes nonboolean names, but their choices/coercions may reject false. Boolean flags ordinarily take no value, but yargs also consumes literal `true`/`false`, separated or attached with `=`. Revision 2 has no boolean-value type, so those extra forms are described here instead of assigning string arity.

Space and equals forms are established by isolated parser results, not help. `short_attached` is conditional: `-p123` works, while `-phello` is split into short-option groups. A numeric tail or the parser's punctuation branch can produce an attached value; use a space or `=` for general text. `-p=hello` is accepted.


### Root entrypoint

| Switch and aliases | Value consumption | Attachments | Purpose |
| --- | --- | --- | --- |
| `--isCommand`, `--is-command` | none | valueless | Internal flag to indicate if a subcommand is being run |
| `--debug`, `-d`, `--d` | none | valueless | Run in debug mode (open debug console with F12) |
| `--model`, `-m`, `--m` | string; one required value | space, equals, short_attached | Model |
| `--prompt`, `-p`, `--p` | string; one required value | space, equals, short_attached | Run in non-interactive (headless) mode with the given prompt. Appended to input on stdin (if any). |
| `--prompt-interactive`, `-i`, `--i`, `--promptInteractive` | string; one required value | space, equals, short_attached | Execute the provided prompt and continue in interactive mode |
| `--skip-trust`, `--skipTrust` | none | valueless | Trust the current workspace for this session. |
| `--worktree`, `-w`, `--w` | string; optional value | space, equals, short_attached | Start Gemini in a new git worktree. If no name is provided, one is generated automatically. |
| `--sandbox`, `-s`, `--s` | none | valueless | Run in sandbox? |
| `--yolo`, `-y`, `--y` | none | valueless | Automatically accept all actions (aka YOLO mode, see https://www.youtube.com/watch?v=xvFZjo5PgG0 for more details)? |
| `--approval-mode`, `--approvalMode` | string; one required value | space, equals | Set the approval mode: default (prompt for approval), auto_edit (auto-approve edit tools), yolo (auto-approve all tools), plan (read-only mode) |
| `--policy` | string; one required value | space, equals | Additional policy files or directories to load (comma-separated or multiple --policy) |
| `--admin-policy`, `--adminPolicy` | string; one required value | space, equals | Additional admin policy files or directories to load (comma-separated or multiple --admin-policy) |
| `--acp` | none | valueless | Starts the agent in ACP mode |
| `--experimental-acp`, `--experimentalAcp` | none | valueless | Starts the agent in ACP mode (deprecated, use --acp instead) |
| `--allowed-mcp-server-names`, `--allowedMcpServerNames` | string; one required value | space, equals | Allowed MCP server names |
| `--allowed-tools`, `--allowedTools` | string; one required value | space, equals | [DEPRECATED: Use Policy Engine instead See https://geminicli.com/docs/core/policy-engine] Tools that are allowed to run without confirmation |
| `--extensions`, `-e`, `--e` | string; one required value | space, equals, short_attached | A list of extensions to use. If not provided, all extensions are used. |
| `--list-extensions`, `-l`, `--l`, `--listExtensions` | none | valueless | List all available extensions and exit. |
| `--resume`, `-r`, `--r` | string; optional value | space, equals, short_attached | Resume a previous session. Use "latest" for most recent or index number (e.g. --resume 5) |
| `--session-file`, `--sessionFile` | string; one required value | space, equals | Load a session from a JSON file |
| `--session-id`, `--sessionId` | string; one required value | space, equals | Start a new session with a manually provided UUID. |
| `--list-sessions`, `--listSessions` | none | valueless | List available sessions for the current project and exit. |
| `--delete-session`, `--deleteSession` | string; optional value | space, equals | Delete a session by index number (use --list-sessions to see available sessions). |
| `--include-directories`, `--includeDirectories` | string; one required value | space, equals | Additional directories to include in the workspace (comma-separated or multiple --include-directories) |
| `--screen-reader`, `--screenReader` | none | valueless | Enable screen reader mode for accessibility. |
| `--output-format`, `-o`, `--o`, `--outputFormat` | string; one required value | space, equals, short_attached | The format of the CLI output. |
| `--fake-responses`, `--fakeResponses` | string; optional value | space, equals | Path to a file with fake model responses for testing. |
| `--fake-responses-non-strict`, `--fakeResponsesNonStrict` | string; optional value | space, equals | Path to a file with fake model responses for testing (non-strict mode). |
| `--record-responses`, `--recordResponses` | string; optional value | space, equals | Path to a file to record model responses for testing. |
| `--raw-output`, `--rawOutput` | none | valueless | Disable sanitization of model output (e.g. allow ANSI escape sequences). WARNING: This can be a security risk if the model output is untrusted. |
| `--accept-raw-output-risk`, `--acceptRawOutputRisk` | none | valueless | Suppress the security warning when using --raw-output. |
| `--help`, `-h`, `--h` | none | valueless | Show help. |
| `--version`, `-v`, `--v` | none | valueless | Show version number. |

### `mcp add`

| Switch and aliases | Value consumption | Attachments | Purpose |
| --- | --- | --- | --- |
| `--scope`, `-s`, `--s` | string; one required value | space, equals, short_attached | Configuration scope (user or project) |
| `--transport`, `-t`, `--t`, `--type` | string; one required value | space, equals, short_attached | Transport type (stdio, sse, http) |
| `--env`, `-e`, `--e` | string; one required value | space, equals, short_attached | Set environment variables (e.g. -e KEY=value) |
| `--header`, `-H`, `--H` | string; one required value | space, equals, short_attached | Set HTTP headers for SSE and HTTP transports (e.g. -H "X-Api-Key: abc123" -H "Authorization: Bearer abc123") |
| `--timeout` | number; optional value | space, equals | Set connection timeout in milliseconds |
| `--trust` | none | valueless | Trust the server (bypass all tool call confirmation prompts) |
| `--description` | string; optional value | space, equals | Set the description for the server |
| `--include-tools`, `--includeTools` | variadic; minimum unknown in contract (parser permits zero) | space, equals | A comma-separated list of tools to include |
| `--exclude-tools`, `--excludeTools` | variadic; minimum unknown in contract (parser permits zero) | space, equals | A comma-separated list of tools to exclude |

### `mcp enable`

| Switch and aliases | Value consumption | Attachments | Purpose |
| --- | --- | --- | --- |
| `--session` | none | valueless | Clear session-only disable |

### `mcp disable`

| Switch and aliases | Value consumption | Attachments | Purpose |
| --- | --- | --- | --- |
| `--session` | none | valueless | Disable for current session only |

### `mcp remove`

| Switch and aliases | Value consumption | Attachments | Purpose |
| --- | --- | --- | --- |
| `--scope`, `-s`, `--s` | string; one required value | space, equals, short_attached | Configuration scope (user or project) |

### `extensions config`

| Switch and aliases | Value consumption | Attachments | Purpose |
| --- | --- | --- | --- |
| `--scope` | string; one required value | space, equals | The scope to set the setting in. |

### `extensions disable`

| Switch and aliases | Value consumption | Attachments | Purpose |
| --- | --- | --- | --- |
| `--scope` | string; optional value | space, equals | The scope to disable the extension in. |

### `extensions enable`

| Switch and aliases | Value consumption | Attachments | Purpose |
| --- | --- | --- | --- |
| `--scope` | string; optional value | space, equals | The scope to enable the extension in. If not set, will be enabled in all scopes. |

### `extensions install`

| Switch and aliases | Value consumption | Attachments | Purpose |
| --- | --- | --- | --- |
| `--ref` | string; optional value | space, equals | The git ref to install from. |
| `--auto-update`, `--autoUpdate` | none | valueless | Enable auto-update for this extension. |
| `--pre-release`, `--preRelease` | none | valueless | Enable pre-release versions for this extension. |
| `--consent` | none | valueless | Acknowledge the security risks of installing an extension and skip the confirmation prompt. |
| `--skip-settings`, `--skipSettings` | none | valueless | Skip the configuration on install process. |

### `extensions link`

| Switch and aliases | Value consumption | Attachments | Purpose |
| --- | --- | --- | --- |
| `--consent` | none | valueless | Acknowledge the security risks of installing an extension and skip the confirmation prompt. |

### `extensions list`

| Switch and aliases | Value consumption | Attachments | Purpose |
| --- | --- | --- | --- |
| `--output-format`, `-o`, `--o`, `--outputFormat` | string; one required value | space, equals, short_attached | The format of the CLI output. |

### `extensions uninstall`

| Switch and aliases | Value consumption | Attachments | Purpose |
| --- | --- | --- | --- |
| `--all` | none | valueless | Uninstall all installed extensions. |

### `extensions update`

| Switch and aliases | Value consumption | Attachments | Purpose |
| --- | --- | --- | --- |
| `--all` | none | valueless | Update all extensions. |

### `skills disable`

| Switch and aliases | Value consumption | Attachments | Purpose |
| --- | --- | --- | --- |
| `--scope`, `-s`, `--s` | string; one required value | space, equals, short_attached | The scope to disable the skill in (user or workspace). |

### `skills install`

| Switch and aliases | Value consumption | Attachments | Purpose |
| --- | --- | --- | --- |
| `--scope` | string; one required value | space, equals | The scope to install the skill into. Defaults to "user" (global). |
| `--path` | string; optional value | space, equals | Sub-path within the repository to install from (only used for git repository sources). |
| `--consent` | none | valueless | Acknowledge the security risks of installing a skill and skip the confirmation prompt. |

### `skills link`

| Switch and aliases | Value consumption | Attachments | Purpose |
| --- | --- | --- | --- |
| `--scope` | string; one required value | space, equals | The scope to link the skill into. Defaults to "user" (global). |
| `--consent` | none | valueless | Acknowledge the security risks of linking a skill and skip the confirmation prompt. |

### `skills list`

| Switch and aliases | Value consumption | Attachments | Purpose |
| --- | --- | --- | --- |
| `--all` | none | valueless | Show all skills, including built-in ones. |

### `skills uninstall`

| Switch and aliases | Value consumption | Attachments | Purpose |
| --- | --- | --- | --- |
| `--scope` | string; one required value | space, equals | The scope to uninstall the skill from. Defaults to "user" (global). |

### `hooks migrate`

| Switch and aliases | Value consumption | Attachments | Purpose |
| --- | --- | --- | --- |
| `--from-claude`, `--fromClaude` | none | valueless | Migrate from Claude Code hooks |

### `gemma logs`

| Switch and aliases | Value consumption | Attachments | Purpose |
| --- | --- | --- | --- |
| `--lines`, `-n`, `--n` | number; optional value | space, equals, short_attached | Show the last N lines and exit (omit to follow live) |
| `--follow`, `-f`, `--f` | none | valueless | Follow log output (defaults to true when --lines is omitted) |

### `gemma setup`

| Switch and aliases | Value consumption | Attachments | Purpose |
| --- | --- | --- | --- |
| `--port` | number; optional value | space, equals | Port for the LiteRT server |
| `--skip-model`, `--skipModel` | none | valueless | Skip model download (binary only) |
| `--start` | none | valueless | Start the server after setup |
| `--force` | none | valueless | Re-download binary and model even if already present |
| `--consent` | none | valueless | Skip interactive consent prompt (implies acceptance) |

### `gemma start`

| Switch and aliases | Value consumption | Attachments | Purpose |
| --- | --- | --- | --- |
| `--port` | number; optional value | space, equals | Port for the LiteRT server |

### `gemma status`

| Switch and aliases | Value consumption | Attachments | Purpose |
| --- | --- | --- | --- |
| `--port` | number; optional value | space, equals | Port to check for the LiteRT server |

### `gemma stop`

| Switch and aliases | Value consumption | Attachments | Purpose |
| --- | --- | --- | --- |
| `--port` | number; optional value | space, equals | Port where the LiteRT server is running |

Root array declarations (`--policy`, `--admin-policy`, `--allowed-mcp-server-names`, `--allowed-tools`, `--extensions`, `--include-directories`) specify `type: array`, `string: true`, **`nargs: 1`**. Thus `--policy a b` consumes only `a`; `b` remains positional. Each accepts repetition and comma splitting. MCP `--env`/`--header` also specify `nargs: 1`, without root comma coercion. MCP `--include-tools`/`--exclude-tools` omit `nargs`: the parser greedily consumes following values and permits an empty array. Their minimum is known to be zero, but this contract cannot encode zero; frontmatter carries `unknown` and the reproduction gap.

`--resume` without a value is coerced to `latest`; `--worktree` without a value gets a generated name and requires `experimental.worktrees`. String declarations without `nargs` allow omitted values at parser level, even when a handler rejects them or substitutes a default. The three session selectors `--resume`, `--session-id`, and `--session-file` conflict. Source accepts session IDs containing letters, digits, dashes, and underscores; despite help saying UUID, it does not require UUID syntax.

Disposable observations used a temporary CWD, `GEMINI_CLI_HOME`, absent system settings files, closed stdin, and a ten-second deadline. Representative results:

| Check | Result |
| --- | --- |
| `gemini --output-format INVALID` | Exit 1; error names output-format and the supplied value. |
| `gemini --outputFormat=INVALID` / `gemini --o INVALID` | Same value-naming rejection; confirms generated aliases. |
| `gemini -o=INVALID` | Value-naming rejection; equals form accepted. |
| `gemini -oINVALID` | Unknown short letters; alphabetic attachment fails. |
| `gemini --session-id bad!` | Exit 1; error names invalid session ID and permitted characters. |
| Matched parser: `.option("policy", {type:"array", string:true, nargs:1})`, `--policy a b` | Source consumption leaves `b` positional; array stores one value. |
| Matched parser: `.option("resume", {type:"string", alias:"r"})`, `--resume` | Empty string before application coercion. |
| Matched parser: `.option("include-tools", {type:"array", string:true})`, `--include-tools` | Empty array; zero values accepted. |
| Matched parser: numeric short tails, e.g. `-p123`, `-n123` | Prompt stores string `123`; lines stores number `123`. |

The disposable harness reconstructed option `type`, `nargs`, `string`, aliases and choices from the tagged declarations and called `yargs(argv).strict().exitProcess(false).help(false).version(false).option(name, declaration).parseSync()`. It did not call Gemini handlers or models. Source defines application coercions separately. Local artifacts are listed in Sources.

No system-prompt delivery flag is declared. The controls `GEMINI_SYSTEM_MD` and `GEMINI_WRITE_SYSTEM_MD` exist as environment strings accepting boolean-like strings or paths; they are not `--` switches. Their delivery semantics belong to the system-prompt topic.

## Configuration Discovery

Settings precedence is defaults → system defaults → user → workspace → system overrides; runtime environment and CLI options then affect resolved configuration. Settings parsing removes JSON comments before `JSON.parse`, so these are JSONC-capable files despite the `.json` extension. Workspace trust can suppress local settings and context. [Settings loader](https://github.com/google-gemini/gemini-cli/blob/v0.61.0/packages/cli/src/config/settings.ts) and [storage paths](https://github.com/google-gemini/gemini-cli/blob/v0.61.0/packages/core/src/config/storage.ts) establish the locations.

| Scope | macOS | Linux | Windows |
| --- | --- | --- |
| System defaults | `/Library/Application Support/GeminiCli/system-defaults.json` | `/etc/gemini-cli/system-defaults.json` | `C:\ProgramData\gemini-cli\system-defaults.json` |
| System overrides | `/Library/Application Support/GeminiCli/settings.json` | `/etc/gemini-cli/settings.json` | `C:\ProgramData\gemini-cli\settings.json` |
| User | `~/.gemini/settings.json` | `~/.gemini/settings.json` | `%USERPROFILE%\.gemini\settings.json` |
| Workspace | `.gemini/settings.json` | `.gemini/settings.json` | `.gemini\settings.json` |

`GEMINI_CLI_HOME` substitutes the home root, then the CLI appends `.gemini`. It also affects global runtime state. System-default and system-override paths have independent environment overrides. User/project settings may be written by management commands. Settings values support environment substitution; the settings schema is available as a static repository artifact, not a CLI dump command.

For environment loading, the source searches upward from the working directory, trying trusted `.gemini/.env` before `.env` at each level. It uses the first eligible file and then falls back to home `.gemini/.env` or `~/.env`. Existing process environment values win. Generic project `.env` loading has excluded-variable rules; Gemini-specific files receive different handling.

`~/.gemini/GEMINI.md` and hierarchical project `GEMINI.md` supply context; `context.fileName` can change the name. `.geminiignore` controls discovery. The CLI also discovers commands, skills, agents, extensions, and policies under its user/workspace resource directories; global/workspace `.agents` resource directories participate in skill/agent discovery. These resources are separate from settings precedence.

Sanitized local inspection found `settings.json`, `GEMINI.md`, `trustedFolders.json`, `google_accounts.json`, `oauth_creds.json`, `projects.json`, `state.json`, `installation_id`, `mcp-oauth-tokens-v2.json`, plus `config`, `agents`, `skills`, `history`, `tmp`, and Antigravity-related entries. Settings top-level keys were `general`, `hooks`, `security`, `tools`, `ui`. Presence establishes local files only: it does not prove every legacy filename is read by current Gemini. Credential values were not inspected or printed.

## Environment Variables

The following general runtime controls were verified in tagged CLI/core source. Model-endpoint/authentication configuration, permission/sandbox policy, MCP configuration, logging/telemetry, and streaming-specific variables belong to their own topics. In particular, this frontmatter does not republish `GEMINI_MODEL`, API credentials, trust bypasses, sandbox variables, or debug/logging controls.

| Variable | Effect |
| --- | --- |
| `GEMINI_CLI_HOME` | Replaces the home root used for .gemini configuration and storage (not the .gemini directory itself). |
| `GEMINI_CLI_SYSTEM_DEFAULTS_PATH` | Selects the system defaults settings file. |
| `GEMINI_CLI_SYSTEM_SETTINGS_PATH` | Selects the system override settings file. |
| `GEMINI_CLI_NO_RELAUNCH` | Any nonempty value prevents the normal child relaunch used for process/memory setup. |
| `GEMINI_CLI_WORKTREE_HANDLED` | Internal relaunch marker preventing repeated worktree setup; wrappers should not set it. |
| `GEMINI_CLI_EXTENSION_REGISTRY_URI` | Overrides the extension registry URI used by extension configuration. |
| `GEMINI_CLI_IDE_PID` | Identifies the IDE process for integration discovery. |
| `GEMINI_CLI_IDE_WORKSPACE_PATH` | Provides the IDE workspace path. |
| `GEMINI_CLI_IDE_SERVER_PORT` | Provides the companion IDE server port. |
| `GEMINI_CLI_IDE_AUTH_TOKEN` | Provides the IDE connection authentication token; do not log it. |
| `GEMINI_CLI_IDE_SERVER_STDIO_COMMAND` | Overrides the IDE companion command for stdio transport. |
| `GEMINI_CLI_IDE_SERVER_STDIO_ARGS` | Provides companion command arguments as a JSON array string. |
| `CLI_TITLE` | Overrides the context shown in the terminal window title. |
| `NO_COLOR` | A nonempty value selects the no-color theme and disables theme configuration. |
| `NO_BROWSER` | A nonempty value disables automatic browser opening; authentication can still need manual input. |
| `VISUAL` | Preferred external editor command when no editor is selected in settings. |
| `EDITOR` | Fallback external editor command after VISUAL. |
| `FORCE_GENERIC_KEYBINDING_HINTS` | Forces generic keybinding hints instead of terminal-specific hints. |
| `HTTP_PROXY` | Fallback network proxy, after HTTPS_PROXY and https_proxy. |
| `HTTPS_PROXY` | Preferred proxy URL for CLI requests. |
| `http_proxy` | Lowercase HTTP proxy fallback after HTTP_PROXY. |
| `https_proxy` | Lowercase HTTPS proxy fallback after HTTPS_PROXY. |
| `NO_PROXY` | Comma-separated destinations that bypass proxy selection. |
| `no_proxy` | Lowercase fallback for NO_PROXY. |
| `TERM` | Participates in terminal capability detection. |
| `TERM_PROGRAM` | Identifies terminal host for UI and keybinding behavior. |
| `TERM_PROGRAM_VERSION` | Participates in terminal-specific feature detection. |
| `COLORTERM` | Participates in terminal color-capability detection. |
| `TMUX` | Identifies a tmux environment during terminal detection. |
| `STY` | Identifies a screen environment during terminal detection. |
| `CI` | Participates in CI/environment detection and non-user runtime behavior. |
| `NODE_OPTIONS` | Node runtime options; CLI relaunch and sandbox logic handle this variable. |
| `PATH` | Executable search path for CLI and tools; do not hard-code the installation location. |

Terminal identification variables are capability inputs, not a promise that UI is safe without a terminal. `NO_BROWSER` prevents automatic opening but does not make an authentication flow unattended. Internal relaunch markers are recorded for discovery, not recommended wrapper inputs.

## Machine Introspection

| Command | Format | Observed behavior |
| --- | --- | --- |
| `gemini extensions list --output-format json` | json | Installed 0.61.0 returned [] on stderr, with empty stdout in isolated state. Source uses debugLogger.log(JSON.stringify(...)); capture channels separately and enforce a timeout. Lists installed extensions, not provider capabilities. |
| `gemini --list-sessions` | text | Project-specific human-readable session list; isolated probe returned no sessions on stdout. |
| `gemini mcp list` | text | Human-readable configured-server state; isolated probe printed no servers on stderr. No JSON option declared. |
| `gemini skills list --all` | text | Human-readable discovered skills, including built-ins; successful probe had startup warnings on stderr. |
| `gemini gemma status` | text | Finite local-server diagnostic; isolated probe returned text and exit 1 because the local binary was not installed. |

`gemini extensions list --output-format json` is the only declared JSON state-reporting command found. Its isolated macOS 0.61.0 run returned exit 0, **empty stdout**, and **`[]` on stderr**. Do not merge stderr warnings into an assumed JSON document. Neither `--help` nor `--version` declares a machine-readable mode. The command registry contains no effective-config dump, model-catalog listing, general doctor, JSON MCP listing, tool-registry dump, or capability report. The published `schemas/settings.schema.json` is a static schema, not runtime provider state.

Headless `--output-format json` and `stream-json` are model execution output formats, not introspection commands. JSON mode reports response/statistics/error; stream-json is JSONL. No paid model run was performed.

## Wrapper Notes

- Installed gemini --version is 0.61.0; upstream GitHub latest and npm latest are 0.62.0. Declarations match, but runtime evidence is macOS 0.61.0 only.
- Use --prompt VALUE for an explicit headless run; positional queries enter interactive mode with a TTY. Resume is a root --resume/-r option, not a subcommand.
- Root array switches use nargs: 1 and consume one token per occurrence. MCP add include-tools/exclude-tools are greedy arrays and also accept zero values; the contract cannot express minimum zero.
- Yargs default short-option groups make alphabetic attached values unsafe: -phello is not a prompt value. Numeric tails are consumed. Prefer -p VALUE or --prompt=VALUE.
- Yargs generates camelCase and double-dash short aliases, boolean negations, and accepts explicit boolean literals. Ordinary boolean records are valueless; literal true/false consumption is not expressible in revision 2.
- extensions list --output-format json emitted JSON to stderr and no stdout. Capture both streams separately; never assume successful machine introspection is on stdout.
- Management commands may request consent or extension settings, write settings, download packages, or start/stop a server. Help succeeds without performing those operations.
- GEMINI_CLI_HOME provides a dedicated .gemini root; isolate system settings overrides and CWD as well when making disposable probes.
- Use argv arrays and preserve --. MCP add forwards unknown options and arguments after -- to the configured server command.
- raw-output disables terminal sanitization; accept-raw-output-risk only suppresses its warning. Keep terminal sanitization when rendering untrusted model output.
- Workspace trust and tool policy can make headless runs fail; do not add trust/approval bypasses implicitly. Permission environment variables belong to agent-permissions.
- List/status commands need deadlines. Help at all 52 paths completed in this refresh; previous help timeouts did not recur.
- Installed settings.json has top-level keys general, hooks, security, tools, ui. Credentials and account/state files exist under ~/.gemini; no secret values were printed.
- No system-prompt delivery switch is declared in either examined root parser. GEMINI_SYSTEM_MD and GEMINI_WRITE_SYSTEM_MD are environment-string controls (boolean-like string or path); semantics belong to system-prompt.

The tagged [headless reference](https://github.com/google-gemini/gemini-cli/blob/v0.61.0/docs/cli/headless.md) documents exit 0 (success), 1 (general/API failure), 42 (input error), and 53 (turn limit). Actual parser failures above exit 1, so do not assume all bad arguments exit 42. Trust/configuration failures can use additional codes; this refresh did not re-test the previous document's exit-55 trust claim. Source and documentation differ on this distinction.

## Sources

- [Gemini CLI homepage](https://geminicli.com/).
- [General documentation](https://geminicli.com/docs/).
- [Live configuration reference](https://geminicli.com/docs/reference/configuration/).
- [Live installation documentation](https://geminicli.com/docs/get-started/installation/).
- [CLI cheatsheet](https://geminicli.com/docs/cli/cli-reference/).
- [GitHub latest stable release](https://github.com/google-gemini/gemini-cli/releases/tag/v0.62.0); the releases/latest API reported v0.62.0, published 2026-09-29.
- [npm latest package metadata](https://registry.npmjs.org/@google%2fgemini-cli/latest), fetched 2026-10-01.
- [0.61.0 CLI package manifest](https://github.com/google-gemini/gemini-cli/blob/v0.61.0/packages/cli/package.json) and [lockfile](https://github.com/google-gemini/gemini-cli/blob/v0.61.0/package-lock.json).
- [0.61.0 root parser](https://github.com/google-gemini/gemini-cli/blob/v0.61.0/packages/cli/src/config/config.ts) and [0.62.0 root parser](https://github.com/google-gemini/gemini-cli/blob/v0.62.0/packages/cli/src/config/config.ts).
- [0.61.0 management modules](https://github.com/google-gemini/gemini-cli/tree/v0.61.0/packages/cli/src/commands) and [0.62.0 management modules](https://github.com/google-gemini/gemini-cli/tree/v0.62.0/packages/cli/src/commands).
- [Settings loader](https://github.com/google-gemini/gemini-cli/blob/v0.61.0/packages/cli/src/config/settings.ts), [storage](https://github.com/google-gemini/gemini-cli/blob/v0.61.0/packages/core/src/config/storage.ts), and [home override](https://github.com/google-gemini/gemini-cli/blob/v0.61.0/packages/core/src/utils/paths.ts).
- [Prompt environment controls](https://github.com/google-gemini/gemini-cli/blob/v0.61.0/packages/core/src/prompts/promptProvider.ts).
- [Relaunch handling](https://github.com/google-gemini/gemini-cli/blob/v0.61.0/packages/cli/src/utils/relaunch.ts), [IDE connection inputs](https://github.com/google-gemini/gemini-cli/blob/v0.61.0/packages/core/src/ide/ide-connection-utils.ts), and [terminal title](https://github.com/google-gemini/gemini-cli/blob/v0.61.0/packages/cli/src/utils/windowTitle.ts).
- [yargs-parser 21.1.1 parser source](https://github.com/yargs/yargs-parser/blob/v21.1.1/lib/yargs-parser.ts), default configuration and short-option parsing branches.
- Sanitized local observations: `/tmp/gemini-cli-research/root-help.txt`, `help-*.txt`, `binary-probes.json`, `parser-results.json`, `short-numeric.json`, and `extra-parser-results.json`; harness `/tmp/gemini-cli-research/parser/check.cjs`. No model execution or user configuration mutation was used.

## Changelog

- 2026-10-01: Converted the old untyped inventory to schema revision 2 with evidence, exact command scopes, aliases, attachment forms, and optional-value records.
- 2026-10-01: Verified installed 0.61.0 and upstream stable 0.62.0; compared tagged parser/command declarations.
- 2026-10-01: Expanded to all 52 command paths including singular aliases; removed fictitious default and unverified update paths.
- 2026-10-01: Corrected repeated root arrays to single-value consumption, documented short attachment restrictions, and represented zero-minimum variadic arrays as a contract gap.
- 2026-10-01: Observed extension JSON on stderr and successful bounded help; removed stale claims of broken long flags and universal help hangs.
- 2026-10-01: Moved permission/model/MCP/logging environment details outside this topic; refreshed configuration and sanitized local observations.