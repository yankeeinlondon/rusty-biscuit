---
$schema: ./_schema.yaml
schema_revision: 2
provider: goose
created: 2026-04-27
last_updated: 2026-10-01
agent: opencode
model: zai-coding-plan/glm-5.3
reasoning_effort: provider_default
latest_version: 1.52.0
versions_examined:
- 1.52.0
homepage: https://goose-docs.ai/
repo: https://github.com/aaif-goose/goose
docs: https://goose-docs.ai/docs/
cli_docs: https://goose-docs.ai/docs/guides/goose-cli-commands
evidence:
- claim: The newest upstream release is v1.52.0, published 2026-09-23.
  id: gh-release-latest
  limitations: Release metadata only; establishes no CLI surface.
  location: https://api.github.com/repos/aaif-goose/goose/releases/latest
  method: official_docs
  observed_on: 2026-10-01
  version: 1.52.0
- claim: github.com/block/goose answers Moved Permanently to aaif-goose/goose, and block.github.io/goose/ serves a redirect page pointing to goose-docs.ai.
  id: legacy-redirects
  limitations: Redirect targets only; no CLI behavior.
  location: curl of https://api.github.com/repos/block/goose and https://block.github.io/goose/ on 2026-10-01
  method: local_inspection
  observed_on: 2026-10-01
  version: unknown
- claim: 'Full help tree of the 1.52.0 release binary: subcommands, switch spellings, value placeholders, defaults, and possible values; hidden commands validate-extensions, mcp-probe, and term log are callable; tui, project, and projects are gone.'
  id: binary-help-1-52-0
  limitations: macOS aarch64 build only; help text, not runtime behavior.
  location: goose-aarch64-apple-darwin release asset from tag v1.52.0, extracted to a disposable temp dir; goose --help captured at the root and at every command path, including hidden paths
  method: local_inspection
  observed_on: 2026-10-01
  version: 1.52.0
- claim: goose --version and goose -V print 1.52.0 and exit 0; goose run --version is rejected, so --version is root-only.
  id: binary-version
  limitations: Version banner only.
  location: downloaded v1.52.0 release binary; goose --version and goose -V
  method: local_inspection
  observed_on: 2026-10-01
  version: 1.52.0
- claim: 'Attachment and value-type proofs: run --output-format bogus and --output-format=bogus (space and equals on a long switch), session list -l abc, -labc, and -l=abc (space, short_attached, and equals on a short switch, with the equals sign stripped), run -tx --recipe y and run -t=x --recipe=y (short_attached and equals on -t, proven by the conflict error naming both switches), info --verbose=true and run --quiet=x (boolean flags reject values), run --max-turns xyz (number parsing), run --provider and run --with-builtin with no value (value required), review --files and review --check-filter with no value (variadic requires at least one), run --params noequals (KEY=VALUE parser), serve --platform bogus (possible values), run -t --system z (leading-dash value rejected for -t), run -n a --session-id b (identifier group conflict), run --id xyz --output-format bogus and schedule remove --id foo (hidden --id alias parses), run --resume --no-session (conflict).'
  id: parse-tests
  limitations: Proves forms only for the switches probed; forms for other switches follow from the same clap parser, named in src-clap-cargo.
  location: disposable runs of the v1.52.0 release binary with an isolated GOOSE_PATH_ROOT; every cited probe failed at argument parsing or validation, naming the switch in the error
  method: disposable_test
  observed_on: 2026-10-01
  version: 1.52.0
- claim: With no provider configured, goose run -t ... --no-session exits 1 with 'No provider configured. Run goose configure first.'; goose run -t ... without --no-session still persists a session row in sessions.db before failing; goose run -r with no input exits 1 demanding -i/-t/--recipe.
  id: run-isolated
  limitations: No provider was configured, so no model session ran and success-path output was not observed.
  location: disposable runs of the v1.52.0 binary with GOOSE_PATH_ROOT pointed at an empty temp dir and stdin closed
  method: disposable_test
  observed_on: 2026-10-01
  version: 1.52.0
- claim: Default paths are XDG-style on macOS (~/.config/goose, ~/.local/share/goose, ~/.local/state/goose); GOOSE_PATH_ROOT reroutes config/data/state under itself; goose info creates state/logs/cli/<date>/<timestamp>.log even when config.yaml is missing; goose info --check exits 1 when no provider is configured.
  id: info-isolated
  limitations: macOS host only; Windows defaults are source-derived, not host-observed.
  location: disposable runs of the v1.52.0 binary with GOOSE_PATH_ROOT set and with an isolated HOME (env -i)
  method: disposable_test
  observed_on: 2026-10-01
  version: 1.52.0
- claim: 'goose session list --format json prints a JSON array (observed with one session: id, working_dir, name, user_set_name, session_type, created_at, usage fields); goose recipe list --format json prints []; goose session export --session-id bogus exits 1 ''Session not found''; goose session remove --session-id <real-id> prints the removal list then fails with ''Error: not connected'' on closed stdin, so removal always seeks confirmation.'
  id: json-lists
  limitations: Small-state shapes only; the export JSON body was not produced because exporting an empty session was not exercised.
  location: disposable runs under an isolated GOOSE_PATH_ROOT
  method: disposable_test
  observed_on: 2026-10-01
  version: 1.52.0
- claim: goose skills list prints a text table and discovers skills from the user's real ~/.claude/skills and ~/.research/library directories even when GOOSE_PATH_ROOT is set.
  id: skills-list-scan
  limitations: Output is text only; no JSON flag exists on this path.
  location: disposable run of goose skills list under an isolated GOOSE_PATH_ROOT on 2026-10-01
  method: local_inspection
  observed_on: 2026-10-01
  version: 1.52.0
- claim: 'clap derive declarations for every switch: short and long spellings, hidden aliases (--id on session and schedule identifiers, -p on session list --working_dir), value names, ArgAction::Append repeats, num_args = 1.. on review --files/--check-filter, value_delimiter on --with-builtin, conflicts between input flags, the hidden --scheduled-job-id, and the feature-gated roam command absent from release builds.'
  id: src-cli-rs
  limitations: Declarations, not runtime behavior; runtime effects are covered by the disposable-test evidence.
  location: https://github.com/aaif-goose/goose/blob/v1.52.0/crates/goose-cli/src/cli.rs
  method: source_code
  observed_on: 2026-10-01
  version: 1.52.0
- claim: The argument parser is clap 4.1.14 (derive, std, help, suggestions, usage, color, error-context); clap accepts --name=value and --name value universally, and -xvalue, -x=value, and -x value for one-character spellings, which is the library-level basis for attachment forms not individually probed.
  id: src-clap-cargo
  limitations: Library capability; goose could in principle override per-argument behavior, so probe-level evidence is preferred where it exists.
  location: https://github.com/aaif-goose/goose/blob/v1.52.0/Cargo.toml
  method: source_code
  observed_on: 2026-10-01
  version: 1.52.0
- claim: Paths resolve through etcetera configured with default-features = false (workspace Cargo.toml), so the XDG strategy applies on every OS; the Block/Block/goose AppStrategyArgs are vestigial, and a stale source comment still references an Apple-style Block path; GOOSE_PATH_ROOT must be absolute and reroutes config/, data/, state/, .agents/plugins, and .agents/agents under itself.
  id: src-paths-rs
  limitations: Windows behavior is source-derived; no Windows host was available in this run.
  location: https://github.com/aaif-goose/goose/blob/v1.52.0/crates/goose/src/config/paths.rs
  method: source_code
  observed_on: 2026-10-01
  version: 1.52.0
- claim: Config layering is system config (/etc/goose/config.yaml on Unix, PROGRAMDATA-based on Windows), then every path in GOOSE_ADDITIONAL_CONFIG_FILES, then the user config.yaml; GOOSE_DISABLE_KEYRING is honored from env and config; GOOSE_DISABLE_SESSION_NAMING and GOOSE_PROMPT_EDITOR_ALWAYS exist as config/env values.
  id: src-config-base
  limitations: Provider-owned env overrides are handled elsewhere in the crate and belong to the model-config topic.
  location: https://github.com/aaif-goose/goose/blob/v1.52.0/crates/goose/src/config/base.rs
  method: source_code
  observed_on: 2026-10-01
  version: 1.52.0
- claim: goose mcp SERVER accepts exactly autovisualiser, computercontroller, memory, and tutorial through a case-insensitive, space-stripping FromStr parser.
  id: src-mcp-runner
  limitations: Server behavior over stdio was not exercised.
  location: https://github.com/aaif-goose/goose/blob/v1.52.0/crates/goose-mcp/src/mcp_server_runner.rs
  method: source_code
  observed_on: 2026-10-01
  version: 1.52.0
- claim: 'Documented installs per OS: shell installer with CONFIGURE=false variant, Homebrew block-goose-cli, PowerShell installer for Windows, WSL path, GOOSE_VERSION pinning for CI, and PATH setup under the user''s .local/bin.'
  id: docs-install
  limitations: Unversioned live page; may lag the binary.
  location: https://goose-docs.ai/docs/getting-started/installation
  method: official_docs
  observed_on: 2026-10-01
  version: unknown
- claim: 'Documented command and switch surface, including two claims the 1.52.0 binary contradicts: a session rename subcommand (rejected by the binary) and a goose mcp ''Google Drive'' example (the mcp parser accepts only the four bundled names).'
  id: docs-cli
  limitations: Unversioned live page; where docs and binary disagree this document records the binary.
  location: https://goose-docs.ai/docs/guides/goose-cli-commands
  method: official_docs
  observed_on: 2026-10-01
  version: unknown
- claim: Documented config file set (config.yaml, permission.yaml, secrets.yaml, permissions/tool_permissions.json, prompts/), env-over-config precedence, API keys never read from config.yaml, and Windows path claims that contradict the 1.52.0 source's XDG strategy.
  id: docs-config
  limitations: Unversioned live page; its Windows and macOS default-location tables are stale relative to the binary.
  location: https://goose-docs.ai/docs/guides/config-files
  method: official_docs
  observed_on: 2026-10-01
  version: unknown
- claim: Documented general environment variables and their effects, including GOOSE_PATH_ROOT, GOOSE_DISABLE_SESSION_NAMING, GOOSE_PROMPT_EDITOR, GOOSE_CLI_* output controls, GOOSE_SERVER__SECRET_KEY, GOOSE_TLS variables, recipe variables, and the goose-set GOOSE_TERMINAL, AGENT, and AGENT_SESSION_ID.
  id: docs-env
  limitations: Unversioned live page; model-endpoint, permission, MCP, and telemetry variables it lists belong to their own topics and are not recorded here.
  location: https://goose-docs.ai/docs/guides/environment-variables
  method: official_docs
  observed_on: 2026-10-01
  version: unknown
binaries:
- alt_binaries: []
  binary: goose
  notes: Verified by downloading and running the goose-aarch64-apple-darwin release asset for v1.52.0; goose --version printed 1.52.0. No goose is installed on this host's PATH.
  os: macos
- alt_binaries: []
  binary: goose
  notes: Release assets exist for x86_64 and aarch64 in GNU, Vulkan, and musl variants, plus Desktop deb/rpm/Flatpak packages; name taken from release asset naming and installer docs.
  os: linux
- alt_binaries:
  - goose
  binary: goose.exe
  notes: Release asset goose-x86_64-pc-windows-msvc.zip plus a cuda variant; users type goose with the install directory on PATH. Not host-verified in this run.
  os: windows
install_methods:
- command: curl -fsSL https://github.com/aaif-goose/goose/releases/download/stable/download_cli.sh | bash
  method: other
  notes: Official shell installer; installs to the user's .local/bin by default (GOOSE_BIN_DIR overrides) and runs interactive goose configure unless CONFIGURE=false.
  os: macos
- command: brew install block-goose-cli
  method: brew
  notes: Official Homebrew formula for the precompiled CLI.
  os: macos
- command: curl -fsSL https://github.com/aaif-goose/goose/releases/download/stable/download_cli.sh | bash
  method: other
  notes: Official shell installer; GOOSE_LINUX_VARIANT=standard, vulkan, or musl selects the asset; CONFIGURE=false skips interactive setup.
  os: linux
- command: curl -fsSL https://github.com/aaif-goose/goose/releases/download/stable/download_cli.sh | bash
  method: other
  notes: Git Bash/MSYS2 path; the docs' PowerShell PATH snippet installs under the user profile's .local\bin; GOOSE_WINDOWS_VARIANT=standard or cuda selects the asset; CONFIGURE=false skips interactive setup.
  os: windows
- command: Invoke-WebRequest -Uri "https://raw.githubusercontent.com/aaif-goose/goose/main/download_cli.ps1" -OutFile "download_cli.ps1"; .\download_cli.ps1
  method: other
  notes: Official PowerShell installer from the installation docs.
  os: windows
subcommands:
- description: Interactive setup wizard for providers, extensions, and settings.
  name: configure
  non_interactive: false
  notes: TTY menu flow; may collect credentials, open a browser for OpenRouter or Tetrate auth, or touch the keyring.
- description: Print goose version, resolved paths, and (with --verbose) the merged configuration.
  name: info
  non_interactive: true
  notes: 'Not side-effect free: creates a log under state/logs even when config is missing.'
- description: Check that the goose setup is working.
  name: doctor
  non_interactive: false
  notes: Interactive diagnostic flow; no flags and no machine-readable mode in 1.52.0.
- description: Run one of the MCP servers bundled with goose over stdio.
  name: mcp
  non_interactive: true
  notes: Positional server argument accepts autovisualiser, computercontroller, memory, or tutorial (case-insensitive, spaces ignored).
- description: Run goose as an ACP agent server over stdio.
  name: acp
  non_interactive: true
  notes: Meant to be spawned by ACP clients such as Zed; keyring access can block unattended runs unless GOOSE_DISABLE_KEYRING is set.
- description: Start the ACP server over HTTP and WebSocket.
  name: serve
  non_interactive: true
  notes: Long-running server; requires GOOSE_SERVER__SECRET_KEY unless --dangerously-unauthenticated is passed.
- description: Start or resume an interactive chat session.
  name: session
  non_interactive: false
  notes: Alias s. Nested list, export, import, and diagnostics are non-interactive when an identifier is supplied.
- description: List all available sessions.
  name: session list
  non_interactive: true
  notes: --format json prints a bare JSON array; filterable by working directory and limit.
- description: Remove saved sessions by id, name, or regex.
  name: session remove
  non_interactive: false
  notes: Always shows the sessions to be removed and asks for confirmation; fails on closed stdin, so it is not scriptable.
- description: Export one session as Markdown, JSON, or YAML, or publish an encrypted Nostr share link.
  name: session export
  non_interactive: true
  notes: Scriptable when an identifier is supplied; prompts an interactive picker when none is.
- description: Import a session from a goose export, a Claude Code, Codex, or Pi transcript, or a Nostr share link.
  name: session import
  non_interactive: true
  notes: Input is a positional argument.
- description: Write a JSON diagnostics report for one session.
  name: session diagnostics
  non_interactive: true
  notes: Scriptable when an identifier is supplied; the report contains session content, config, and logs.
- description: Execute a one-shot prompt, instruction file, stdin, or recipe, then exit.
  name: run
  non_interactive: true
  notes: Primary wrapper entry point and the resume path (via -r/--resume); -s/--interactive keeps a terminal session open after the first turn.
- description: Parent command for recipe utilities.
  name: recipe
  non_interactive: true
  notes: Requires a nested subcommand; a bare goose recipe exits with a usage error.
- description: Validate one recipe file.
  name: recipe validate
  non_interactive: true
  notes: Positional recipe name or path; no switches.
- description: Generate a shareable deeplink for a recipe.
  name: recipe deeplink
  non_interactive: true
  notes: Positional recipe name or path.
- description: Open a recipe in goose Desktop.
  name: recipe open
  non_interactive: false
  notes: Launches or focuses the Desktop app.
- description: List available recipes.
  name: recipe list
  non_interactive: true
  notes: --format json is machine-readable.
- description: Parent command for skill utilities.
  name: skills
  non_interactive: true
  notes: Requires a nested subcommand.
- description: List skills available to the goose agent.
  name: skills list
  non_interactive: true
  notes: Text table only; scans the user's real ~/.claude/skills and ~/.research regardless of GOOSE_PATH_ROOT.
- description: Parent command for plugin management.
  name: plugin
  non_interactive: true
  notes: Requires a nested subcommand.
- description: Install a plugin from a git repository URL.
  name: plugin install
  non_interactive: false
  notes: Network and git operations; credential prompts possible.
- description: Update an installed git-backed plugin by name.
  name: plugin update
  non_interactive: false
  notes: Network and git operations; credential prompts possible.
- description: Parent command for scheduled recipe jobs.
  name: schedule
  non_interactive: true
  notes: Alias sched. Requires a nested subcommand.
- description: Add a scheduled job from a cron expression and recipe source.
  name: schedule add
  non_interactive: true
  notes: Copies the recipe into the data directory; --schedule-id, --cron, and --recipe-source are required.
- description: List all scheduled jobs.
  name: schedule list
  non_interactive: true
  notes: Text output.
- description: Remove a scheduled job by id.
  name: schedule remove
  non_interactive: true
  notes: Requires --schedule-id.
- description: List sessions created by one schedule.
  name: schedule sessions
  non_interactive: true
  notes: Requires --schedule-id.
- description: Run a scheduled job immediately.
  name: schedule run-now
  non_interactive: true
  notes: Triggers a real agent run through the scheduler.
- description: 'Deprecated: check status of scheduler services.'
  name: schedule services-status
  non_interactive: true
  notes: Deprecated helper; no external services are needed anymore.
- description: 'Deprecated: stop scheduler services.'
  name: schedule services-stop
  non_interactive: true
  notes: Deprecated helper.
- description: Print cron expression examples and help.
  name: schedule cron-help
  non_interactive: true
  notes: Pure help text.
- description: Parent command for external platform gateways.
  name: gateway
  non_interactive: true
  notes: Alias gw. Requires a nested subcommand.
- description: Show gateway status.
  name: gateway status
  non_interactive: true
  notes: Text output.
- description: Start a gateway of a given type, for example telegram.
  name: gateway start
  non_interactive: false
  notes: Requires --bot-token, a secret-bearing argument; long-running.
- description: Stop a running gateway by type.
  name: gateway stop
  non_interactive: true
  notes: Scriptable.
- description: Generate a pairing code for a gateway.
  name: gateway pair
  non_interactive: false
  notes: Pairing flows require external user action.
- description: Update the goose CLI binary.
  name: update
  non_interactive: false
  notes: Mutates the installed binary; --reconfigure prompts interactively.
- description: Parent command for terminal-integrated persistent sessions.
  name: term
  non_interactive: true
  notes: Requires a nested subcommand.
- description: Print the shell initialization script for terminal integration.
  name: term init
  non_interactive: true
  notes: 'Shells: bash, zsh, fish, nu, powershell.'
- description: Send a prompt into the terminal-integrated session.
  name: term run
  non_interactive: false
  notes: Drives an interactive persistent session tied to the terminal.
- description: Print compact session info (token usage, model) for shell prompts.
  name: term info
  non_interactive: true
  notes: 'Example output shape: five-circle gauge plus model name.'
- description: Log a shell command to the terminal session.
  name: term log
  non_interactive: true
  notes: Hidden from help; called by the shell hook installed by term init.
- description: Parent command for local inference model management.
  name: local-models
  non_interactive: true
  notes: Alias lm. Requires a nested subcommand.
- description: Search Hugging Face for local GGUF and MLX models.
  name: local-models search
  non_interactive: true
  notes: Network call; --json prints results as JSON.
- description: Download a local model by spec.
  name: local-models download
  non_interactive: true
  notes: Network and disk side effects.
- description: List downloaded local models.
  name: local-models list
  non_interactive: true
  notes: Text output.
- description: Delete a downloaded local model by id.
  name: local-models delete
  non_interactive: true
  notes: Mutates local model storage.
- description: Generate the autocompletion script or Nushell module for a shell.
  name: completion
  non_interactive: true
  notes: 'Shells: bash, elvish, fish, powershell (alias pwsh), nu (alias nushell), zsh.'
- description: Review the current diff, or an explicit range, using goose and .agents/checks reviewers.
  name: review
  non_interactive: true
  notes: Spawns up to 4 concurrent goose run subprocesses for checks unless --no-orchestrate; --dry-run prints assembled inputs without running the agent.
- description: Print help for the command or a subcommand.
  name: help
  non_interactive: true
  notes: clap built-in help subcommand.
- description: Validate a bundled-extensions.json file.
  name: validate-extensions
  non_interactive: true
  notes: Hidden from top-level help but callable in 1.52.0; positional file argument.
- description: Start a goose MCP session without an LLM and inspect a stdio MCP server.
  name: mcp-probe
  non_interactive: true
  notes: Hidden from top-level help; takes a stdio extension command and an optional --script JSON probe (use - for stdin).
cli_switches:
- aliases:
  - -h
  attachment: []
  description: Print help for the command it is attached to.
  evidence_ids:
  - binary-help-1-52-0
  - src-clap-cargo
  example: goose run --help
  flag: --help
  invocation_scope:
  - applies_to: global
  notes: clap's automatic help flag; present at every command path.
  scope:
  - meta
  value_type: none
- aliases:
  - -V
  attachment: []
  description: Print the installed goose version and exit.
  evidence_ids:
  - binary-version
  - binary-help-1-52-0
  example: goose --version
  flag: --version
  invocation_scope:
  - applies_to: command
    command: []
  notes: Root entrypoint only; goose run --version is rejected with an unexpected-argument error.
  scope:
  - meta
  value_type: none
- aliases:
  - -v
  attachment: []
  description: Show verbose information including the merged config.yaml values.
  evidence_ids:
  - binary-help-1-52-0
  - parse-tests
  - src-cli-rs
  example: goose info --verbose
  flag: --verbose
  invocation_scope:
  - applies_to: command
    command:
    - info
  notes: Human-oriented YAML-like config block inside text output; --verbose=true is rejected with an unexpected-value error.
  scope:
  - diagnostics
  value_type: none
- attachment: []
  description: Test the provider connection and show its status.
  evidence_ids:
  - binary-help-1-52-0
  - info-isolated
  - src-cli-rs
  example: goose info --check
  flag: --check
  invocation_scope:
  - applies_to: command
    command:
    - info
  notes: Performs a real provider request; exits 1 when no provider is configured.
  scope:
  - diagnostics
  value_type: none
- attachment:
  - space
  - equals
  default: Loaded default profile extensions
  description: Add one or more builtin extensions bundled with goose, by name.
  evidence_ids:
  - binary-help-1-52-0
  - src-cli-rs
  - parse-tests
  example: goose run --with-builtin developer -t "summarize this repo"
  flag: --with-builtin
  invocation_scope:
  - applies_to: command
    command:
    - acp
  - applies_to: command
    command:
    - serve
  - applies_to: command
    command:
    - run
  notes: Declared with a comma value_delimiter, so one occurrence may carry a comma-separated list such as developer,github; on serve the switch is also repeatable. An occurrence with no value is rejected.
  scope:
  - extensions
  value: <NAME>
  value_optional: false
  value_type: string
- attachment: []
  description: Enable scheduled recipe execution in the ACP server.
  evidence_ids:
  - binary-help-1-52-0
  - src-cli-rs
  example: goose serve --enable-scheduler
  flag: --enable-scheduler
  invocation_scope:
  - applies_to: command
    command:
    - acp
  - applies_to: command
    command:
    - serve
  notes: Not present in 1.41.0.
  scope:
  - scheduler
  value_type: none
- attachment:
  - space
  - equals
  default: 127.0.0.1
  description: Host interface for the ACP HTTP and WebSocket server.
  evidence_ids:
  - binary-help-1-52-0
  - src-cli-rs
  example: goose serve --host 0.0.0.0
  flag: --host
  invocation_scope:
  - applies_to: command
    command:
    - serve
  scope:
  - server
  value: <HOST>
  value_optional: false
  value_type: string
- attachment:
  - space
  - equals
  default: '3284'
  description: Port for the ACP HTTP and WebSocket server.
  evidence_ids:
  - binary-help-1-52-0
  - src-cli-rs
  example: goose serve --port 3284
  flag: --port
  invocation_scope:
  - applies_to: command
    command:
    - serve
  notes: Parsed as u16.
  scope:
  - server
  value: <PORT>
  value_optional: false
  value_type: number
- attachment: []
  description: Serve ACP over TLS.
  evidence_ids:
  - binary-help-1-52-0
  - src-cli-rs
  - docs-env
  example: goose serve --tls --tls-cert-path cert.pem --tls-key-path key.pem
  flag: --tls
  invocation_scope:
  - applies_to: command
    command:
    - serve
  notes: The docs describe a certificate fingerprint line printed on startup.
  scope:
  - server
  value_type: none
- attachment:
  - space
  - equals
  description: TLS certificate path for goose serve.
  evidence_ids:
  - binary-help-1-52-0
  - src-cli-rs
  - docs-env
  example: goose serve --tls-cert-path cert.pem
  flag: --tls-cert-path
  invocation_scope:
  - applies_to: command
    command:
    - serve
  notes: Environment equivalents GOOSE_TLS_CERT_PATH and GOOSE_TLS_KEY_PATH exist.
  scope:
  - server
  value: <PATH>
  value_optional: false
  value_type: string
- attachment:
  - space
  - equals
  description: TLS private key path for goose serve.
  evidence_ids:
  - binary-help-1-52-0
  - src-cli-rs
  - docs-env
  example: goose serve --tls-key-path key.pem
  flag: --tls-key-path
  invocation_scope:
  - applies_to: command
    command:
    - serve
  notes: Used together with --tls-cert-path.
  scope:
  - server
  value: <PATH>
  value_optional: false
  value_type: string
- attachment:
  - space
  - equals
  default: cli
  description: Select the goose platform identity the server reports.
  evidence_ids:
  - binary-help-1-52-0
  - parse-tests
  - src-cli-rs
  example: goose serve --platform desktop
  flag: --platform
  invocation_scope:
  - applies_to: command
    command:
    - serve
  notes: Possible values cli and desktop, verified by an invalid-value rejection listing them.
  scope:
  - server
  value: <PLATFORM>
  value_optional: false
  value_type: string
- attachment: []
  description: Start the ACP endpoint without requiring GOOSE_SERVER__SECRET_KEY.
  evidence_ids:
  - binary-help-1-52-0
  - src-cli-rs
  example: goose serve --dangerously-unauthenticated
  flag: --dangerously-unauthenticated
  invocation_scope:
  - applies_to: command
    command:
    - serve
  notes: Wrappers must not add this automatically.
  scope:
  - server
  value_type: none
- attachment:
  - space
  - equals
  description: Allow an exact Origin value for ACP CORS.
  evidence_ids:
  - binary-help-1-52-0
  - src-cli-rs
  example: goose serve --allowed-origin http://localhost:3000
  flag: --allowed-origin
  invocation_scope:
  - applies_to: command
    command:
    - serve
  notes: Repeatable via ArgAction::Append; supplying it replaces the default loopback origins.
  scope:
  - server
  value: <ORIGIN>
  value_optional: false
  value_type: string
- aliases:
  - -i
  attachment:
  - space
  - equals
  - short_attached
  description: Path to an instruction file containing commands; use - for stdin.
  evidence_ids:
  - binary-help-1-52-0
  - src-cli-rs
  - src-clap-cargo
  example: goose run --instructions -
  flag: --instructions
  invocation_scope:
  - applies_to: command
    command:
    - run
  notes: Conflicts with --text and --recipe.
  scope:
  - input
  value: <FILE>
  value_optional: false
  value_type: string
- aliases:
  - -t
  attachment:
  - space
  - equals
  - short_attached
  description: Input text containing commands for goose, in lieu of an instruction file.
  evidence_ids:
  - binary-help-1-52-0
  - src-cli-rs
  - parse-tests
  example: goose run --text "summarize this repo"
  flag: --text
  invocation_scope:
  - applies_to: command
    command:
    - run
  notes: 'Conflicts with --instructions and --recipe. No allow_hyphen_values: a value starting with a dash is rejected with a value-required error, so forward leading-dash prompts as --text=<prompt> or -t<prompt>.'
  scope:
  - input
  value: <TEXT>
  value_optional: false
  value_type: string
- attachment:
  - space
  - equals
  description: Run a recipe by configured name or by full path to a recipe file.
  evidence_ids:
  - binary-help-1-52-0
  - src-cli-rs
  - parse-tests
  example: goose run --recipe ./recipe.yaml
  flag: --recipe
  invocation_scope:
  - applies_to: command
    command:
    - run
  notes: Conflicts with --instructions and --text; --system also conflicts with it.
  scope:
  - input
  value: <RECIPE_NAME or FULL_PATH_TO_RECIPE_FILE>
  value_optional: false
  value_type: string
- attachment:
  - space
  - equals
  description: Provide additional system instructions to customize the agent's behavior.
  evidence_ids:
  - binary-help-1-52-0
  - src-cli-rs
  example: goose run --system "Be concise" -t "summarize"
  flag: --system
  invocation_scope:
  - applies_to: command
    command:
    - run
  notes: Delivery semantics belong to the system-prompt topic; this record captures only the flag surface. Conflicts with --recipe.
  scope:
  - system_prompt
  value: <TEXT>
  value_optional: false
  value_type: string
- attachment:
  - space
  - equals
  description: Pass a key-value parameter to the recipe; may be specified multiple times.
  evidence_ids:
  - binary-help-1-52-0
  - src-cli-rs
  - parse-tests
  example: goose run --recipe deploy.yaml --params env=prod
  flag: --params
  invocation_scope:
  - applies_to: command
    command:
    - run
  - applies_to: command
    command:
    - schedule
    - add
  notes: Repeatable via ArgAction::Append, one KEY=VALUE per occurrence; a value without an equals sign is rejected by the parser.
  scope:
  - recipes
  value: <KEY=VALUE>
  value_optional: false
  value_type: string
- attachment:
  - space
  - equals
  description: Include a sub-recipe alongside the main recipe; may be specified multiple times.
  evidence_ids:
  - binary-help-1-52-0
  - src-cli-rs
  example: goose run --recipe main.yaml --sub-recipe audit.yaml
  flag: --sub-recipe
  invocation_scope:
  - applies_to: command
    command:
    - run
  notes: Values are recipe names from the configured GitHub repo or local YAML paths.
  scope:
  - recipes
  value: <RECIPE>
  value_optional: false
  value_type: string
- attachment: []
  description: Show the recipe's title, description, and parameters instead of running it.
  evidence_ids:
  - binary-help-1-52-0
  - src-cli-rs
  example: goose run --recipe build.yaml --explain
  flag: --explain
  invocation_scope:
  - applies_to: command
    command:
    - run
  scope:
  - recipes
  value_type: none
- attachment: []
  description: Print the rendered recipe instead of running it.
  evidence_ids:
  - binary-help-1-52-0
  - src-cli-rs
  example: goose run --recipe build.yaml --render-recipe
  flag: --render-recipe
  invocation_scope:
  - applies_to: command
    command:
    - run
  scope:
  - recipes
  value_type: none
- aliases:
  - -n
  attachment:
  - space
  - equals
  - short_attached
  description: Identify a session by its user-assigned name.
  evidence_ids:
  - binary-help-1-52-0
  - src-cli-rs
  - parse-tests
  example: goose run --name my-project -t "start"
  flag: --name
  invocation_scope:
  - applies_to: command
    command:
    - run
  - applies_to: command
    command:
    - session
    - export
  - applies_to: command
    command:
    - session
    - diagnostics
  notes: Part of the identifier group shared by run, session, and the session subcommands; at most one of --name, --session-id, and --path may be given, verified by a group-conflict rejection. On run without --resume it names a newly created session.
  scope:
  - session_identity
  value: <NAME>
  value_optional: false
  value_type: string
- aliases:
  - --id
  attachment:
  - space
  - equals
  description: Identify a session by its ID, for example 20251108_2.
  evidence_ids:
  - binary-help-1-52-0
  - src-cli-rs
  - parse-tests
  example: goose run --resume --session-id 20251108_2 -t "continue"
  flag: --session-id
  invocation_scope:
  - applies_to: command
    command:
    - run
  - applies_to: command
    command:
    - session
    - export
  - applies_to: command
    command:
    - session
    - diagnostics
  notes: The --id spelling is a hidden alias from source, verified by parse probes on run and schedule remove. On run it requires --resume.
  scope:
  - session_identity
  value: <SESSION_ID>
  value_optional: false
  value_type: string
- attachment:
  - space
  - equals
  description: Legacy session identifier that extracts the session ID from a .jsonl file path.
  evidence_ids:
  - binary-help-1-52-0
  - src-cli-rs
  example: goose run --resume --path ./20250325_200615.jsonl -t "continue"
  flag: --path
  invocation_scope:
  - applies_to: command
    command:
    - run
  - applies_to: command
    command:
    - session
    - export
  - applies_to: command
    command:
    - session
    - diagnostics
  notes: Kept for backward compatibility with file-based session storage.
  scope:
  - session_identity
  value: <PATH>
  value_optional: false
  value_type: string
- aliases:
  - -s
  attachment: []
  description: Continue in interactive mode after processing the initial input.
  evidence_ids:
  - binary-help-1-52-0
  - src-cli-rs
  example: goose run -t "start by inspecting failures" --interactive
  flag: --interactive
  invocation_scope:
  - applies_to: command
    command:
    - run
  notes: The startup-prompt form for interactive sessions; avoid in batch wrappers.
  scope:
  - behavior
  value_type: none
- attachment: []
  description: Execute without creating or using a session file.
  evidence_ids:
  - binary-help-1-52-0
  - src-cli-rs
  - parse-tests
  - run-isolated
  example: goose run --no-session --output-format stream-json -t "summarize"
  flag: --no-session
  invocation_scope:
  - applies_to: command
    command:
    - run
  notes: Conflicts with --resume, --name, and --path, verified by a conflict rejection. Without it, run persists a session row even when the provider check then fails.
  scope:
  - behavior
  value_type: none
- aliases:
  - -r
  attachment: []
  description: Continue from a previous run, maintaining execution state and context.
  evidence_ids:
  - binary-help-1-52-0
  - src-cli-rs
  example: goose run --resume -t "continue"
  flag: --resume
  invocation_scope:
  - applies_to: command
    command:
    - run
  notes: With no identifier, resumes the most recently used session. This flag is goose's resume path; there is no separate resume subcommand.
  scope:
  - resume
  value_type: none
- attachment: []
  description: Print generation statistics after the run completes.
  evidence_ids:
  - binary-help-1-52-0
  - src-cli-rs
  example: goose run --stats -t "summarize"
  flag: --stats
  invocation_scope:
  - applies_to: command
    command:
    - run
  scope:
  - output
  value_type: none
- attachment: []
  description: Show complete tool responses without truncation and full paths.
  evidence_ids:
  - binary-help-1-52-0
  - src-cli-rs
  example: goose run --debug -t "inspect failing tests"
  flag: --debug
  invocation_scope:
  - applies_to: command
    command:
    - run
  notes: May expose sensitive data in output.
  scope:
  - diagnostics
  value_type: none
- attachment:
  - space
  - equals
  description: Limit how many times the same tool can be called consecutively with identical parameters.
  evidence_ids:
  - binary-help-1-52-0
  - src-cli-rs
  - src-clap-cargo
  example: goose run --max-tool-repetitions 3 -t "fix loop"
  flag: --max-tool-repetitions
  invocation_scope:
  - applies_to: command
    command:
    - run
  notes: Parsed as u32.
  scope:
  - limits
  value: <NUMBER>
  value_optional: false
  value_type: number
- attachment:
  - space
  - equals
  default: 1000 through config or env; unset at the flag level
  description: Limit how many turns the agent can take without asking the user to continue.
  evidence_ids:
  - binary-help-1-52-0
  - src-cli-rs
  - parse-tests
  example: goose run --max-turns 10 -t "make a small change"
  flag: --max-turns
  invocation_scope:
  - applies_to: command
    command:
    - run
  notes: Parsed as u32; a non-numeric value is rejected naming the switch. Also settable as GOOSE_MAX_TURNS.
  scope:
  - limits
  value: <NUMBER>
  value_optional: false
  value_type: number
- attachment:
  - space
  - equals
  description: Run stdio and built-in extensions inside the specified Docker container.
  evidence_ids:
  - binary-help-1-52-0
  - src-cli-rs
  example: goose run --container devbox -t "run tests"
  flag: --container
  invocation_scope:
  - applies_to: command
    command:
    - run
  notes: The extension, and for builtins goose itself, must exist inside the container.
  scope:
  - extensions
  value: <CONTAINER_ID>
  value_optional: false
  value_type: string
- attachment:
  - space
  - equals
  description: Add a stdio extension from a full command line; may be specified multiple times.
  evidence_ids:
  - binary-help-1-52-0
  - src-cli-rs
  - parse-tests
  example: goose run --with-extension "npx -y @modelcontextprotocol/server-memory" -t "remember this"
  flag: --with-extension
  invocation_scope:
  - applies_to: command
    command:
    - run
  notes: 'Format ''[name:]ENV1=val1 command args...''; shell quoting matters. The documented --with-remote-extension does not exist: the 1.52.0 binary rejects it and suggests --with-extension.'
  scope:
  - extensions
  value: <COMMAND>
  value_optional: false
  value_type: string
- attachment:
  - space
  - equals
  description: Add a streamable HTTP extension from a URL; may be specified multiple times.
  evidence_ids:
  - binary-help-1-52-0
  - src-cli-rs
  example: goose run --with-streamable-http-extension "http://localhost:8080/mcp timeout=100" -t "use the server"
  flag: --with-streamable-http-extension
  invocation_scope:
  - applies_to: command
    command:
    - run
  notes: The single value may carry a whitespace-separated timeout=SECONDS suffix parsed by a custom value parser.
  scope:
  - extensions
  value: <URL>
  value_optional: false
  value_type: string
- attachment: []
  description: Do not load default profile extensions; use only CLI-specified extensions.
  evidence_ids:
  - binary-help-1-52-0
  - src-cli-rs
  example: goose run --no-profile --with-builtin developer -t "inspect"
  flag: --no-profile
  invocation_scope:
  - applies_to: command
    command:
    - run
  scope:
  - extensions
  value_type: none
- aliases:
  - -q
  attachment: []
  description: Suppress non-response output, printing only the model response to stdout.
  evidence_ids:
  - binary-help-1-52-0
  - parse-tests
  - src-cli-rs
  example: goose run --quiet -t "answer only"
  flag: --quiet
  invocation_scope:
  - applies_to: command
    command:
    - run
  - applies_to: command
    command:
    - review
  notes: Passing a value is rejected; structured JSON output is more reliable for wrappers.
  scope:
  - output
  value_type: none
- attachment:
  - space
  - equals
  default: text
  description: Select the run output format.
  evidence_ids:
  - binary-help-1-52-0
  - parse-tests
  - src-cli-rs
  example: goose run --output-format stream-json --no-session -t "summarize this repo"
  flag: --output-format
  invocation_scope:
  - applies_to: command
    command:
    - run
  notes: Possible values text, json, and stream-json, verified by invalid-value rejections in both attachment forms; stream-json emits newline-delimited JSON events.
  scope:
  - output
  value: <FORMAT>
  value_optional: false
  value_type: string
- attachment:
  - space
  - equals
  description: Override the GOOSE_PROVIDER environment variable for this invocation.
  evidence_ids:
  - binary-help-1-52-0
  - parse-tests
  - src-cli-rs
  example: goose run --provider anthropic --model claude-sonnet-4-5 -t "inspect"
  flag: --provider
  invocation_scope:
  - applies_to: command
    command:
    - run
  - applies_to: command
    command:
    - review
  notes: A missing value is rejected with a value-required error. Endpoint variables belong to the model-config topic.
  scope:
  - model_selection
  value: <PROVIDER>
  value_optional: false
  value_type: string
- attachment:
  - space
  - equals
  description: Override the GOOSE_MODEL environment variable for this invocation.
  evidence_ids:
  - binary-help-1-52-0
  - src-cli-rs
  example: goose run --provider openai --model gpt-4.1 -t "summarize"
  flag: --model
  invocation_scope:
  - applies_to: command
    command:
    - run
  - applies_to: command
    command:
    - review
  notes: On review it is the default model for the main agent and for checks that do not declare their own.
  scope:
  - model_selection
  value: <MODEL>
  value_optional: false
  value_type: string
- attachment:
  - space
  - equals
  description: Associate the run with a scheduled job; internal use.
  evidence_ids:
  - src-cli-rs
  - parse-tests
  example: goose run --scheduled-job-id daily-report --recipe report.yaml
  flag: --scheduled-job-id
  invocation_scope:
  - applies_to: command
    command:
    - run
  notes: Hidden from help (hide = true in source); parses on the release binary.
  scope:
  - scheduler
  value: <ID>
  value_optional: false
  value_type: string
- aliases:
  - -p
  attachment:
  - space
  - equals
  - short_attached
  description: Pre-fill one recipe parameter; may be specified multiple times.
  evidence_ids:
  - binary-help-1-52-0
  - src-cli-rs
  example: goose recipe deeplink my-recipe --param env=prod
  flag: --param
  invocation_scope:
  - applies_to: command
    command:
    - recipe
    - deeplink
  - applies_to: command
    command:
    - recipe
    - open
  notes: Unlike run --params this is a plain repeatable string with no KEY=VALUE parser.
  scope:
  - recipes
  value: <KEY=VALUE>
  value_optional: false
  value_type: string
- attachment:
  - space
  - equals
  default: text
  description: Select the recipe list output format.
  evidence_ids:
  - binary-help-1-52-0
  - src-cli-rs
  - json-lists
  example: goose recipe list --format json
  flag: --format
  invocation_scope:
  - applies_to: command
    command:
    - recipe
    - list
  notes: Possible values text and json; json is machine-readable and prints a bare JSON array.
  scope:
  - output
  value: <FORMAT>
  value_optional: false
  value_type: string
- aliases:
  - -v
  attachment: []
  description: Show verbose recipe information including descriptions.
  evidence_ids:
  - binary-help-1-52-0
  - src-cli-rs
  example: goose recipe list --verbose
  flag: --verbose
  invocation_scope:
  - applies_to: command
    command:
    - recipe
    - list
  notes: Same spelling as info --verbose but a different switch at a different path.
  scope:
  - output
  value_type: none
- aliases:
  - --id
  attachment:
  - space
  - equals
  description: Identify a scheduled job by its unique ID.
  evidence_ids:
  - binary-help-1-52-0
  - src-cli-rs
  - parse-tests
  example: goose schedule run-now --schedule-id daily-report
  flag: --schedule-id
  invocation_scope:
  - applies_to: command
    command:
    - schedule
    - add
  - applies_to: command
    command:
    - schedule
    - remove
  - applies_to: command
    command:
    - schedule
    - sessions
  - applies_to: command
    command:
    - schedule
    - run-now
  notes: The --id spelling is a hidden alias from source, verified by a parse probe on schedule remove; required at every listed path.
  scope:
  - scheduler
  value: <SCHEDULE_ID>
  value_optional: false
  value_type: string
- attachment:
  - space
  - equals
  description: Cron expression for when the scheduled job runs.
  evidence_ids:
  - binary-help-1-52-0
  - src-cli-rs
  example: goose schedule add --schedule-id daily --cron "0 9 * * *" --recipe-source ./daily.yaml
  flag: --cron
  invocation_scope:
  - applies_to: command
    command:
    - schedule
    - add
  notes: Required for schedule add.
  scope:
  - scheduler
  value: <CRON>
  value_optional: false
  value_type: string
- attachment:
  - space
  - equals
  description: 'Recipe source: a path to a file or a base64-encoded recipe string.'
  evidence_ids:
  - binary-help-1-52-0
  - src-cli-rs
  example: goose schedule add --schedule-id daily --cron "0 9 * * *" --recipe-source ./daily.yaml
  flag: --recipe-source
  invocation_scope:
  - applies_to: command
    command:
    - schedule
    - add
  notes: Required for schedule add.
  scope:
  - scheduler
  value: <RECIPE_SOURCE>
  value_optional: false
  value_type: string
- aliases:
  - -l
  attachment:
  - space
  - equals
  - short_attached
  default: 10 for local-models search; otherwise no limit is applied
  description: Limit the number of results returned.
  evidence_ids:
  - binary-help-1-52-0
  - parse-tests
  - src-cli-rs
  example: goose session list --limit 10
  flag: --limit
  invocation_scope:
  - applies_to: command
    command:
    - session
    - list
  - applies_to: command
    command:
    - schedule
    - sessions
  - applies_to: command
    command:
    - local-models
    - search
  notes: Parsed as usize; -l abc, -labc, and -l=abc are all rejected with an invalid-digit error naming --limit, proving space, short_attached, and equals attachment.
  scope:
  - output
  value: <LIMIT>
  value_optional: false
  value_type: number
- attachment:
  - space
  - equals
  default: goose
  description: Use a custom binary name in generated completions.
  evidence_ids:
  - binary-help-1-52-0
  - src-cli-rs
  example: goose completion zsh --bin-name goose
  flag: --bin-name
  invocation_scope:
  - applies_to: command
    command:
    - completion
  scope:
  - meta
  value: <BIN_NAME>
  value_optional: false
  value_type: string
- attachment:
  - space
  - equals
  description: Path to a Markdown file with a custom base review prompt.
  evidence_ids:
  - binary-help-1-52-0
  - src-cli-rs
  example: goose review --prompt REVIEW.md
  flag: --prompt
  invocation_scope:
  - applies_to: command
    command:
    - review
  notes: Replaces the embedded default prompt.
  scope:
  - review
  value: <FILE>
  value_optional: false
  value_type: string
- attachment:
  - space
  - equals
  description: Force every discovered check to use this model regardless of its own model field.
  evidence_ids:
  - binary-help-1-52-0
  - src-cli-rs
  example: goose review --override-model claude-sonnet-4-5
  flag: --override-model
  invocation_scope:
  - applies_to: command
    command:
    - review
  scope:
  - review
  - model_selection
  value: <MODEL>
  value_optional: false
  value_type: string
- attachment:
  - space
  - equals
  description: Default turn limit for orchestrated review subprocesses and checks that do not declare their own.
  evidence_ids:
  - binary-help-1-52-0
  - src-cli-rs
  example: goose review --turn-limit 10
  flag: --turn-limit
  invocation_scope:
  - applies_to: command
    command:
    - review
  notes: Parsed as usize; does not cap the legacy --no-orchestrate in-process agent.
  scope:
  - review
  - limits
  value: <N>
  value_optional: false
  value_type: number
- attachment: []
  description: Print the assembled review prompt and discovered checks instead of running the review.
  evidence_ids:
  - binary-help-1-52-0
  - src-cli-rs
  example: goose review --dry-run
  flag: --dry-run
  invocation_scope:
  - applies_to: command
    command:
    - review
  scope:
  - review
  value_type: none
- attachment: []
  description: Disable the Rust-driven parallel orchestrator and use the single-prompt delegation path.
  evidence_ids:
  - binary-help-1-52-0
  - src-cli-rs
  example: goose review --no-orchestrate
  flag: --no-orchestrate
  invocation_scope:
  - applies_to: command
    command:
    - review
  notes: Checks with an explicit tool allowlist require the default orchestrator.
  scope:
  - review
  value_type: none
- aliases:
  - -i
  attachment:
  - space
  - equals
  - short_attached
  description: Additional free-form instructions to prepend to the review.
  evidence_ids:
  - binary-help-1-52-0
  - src-cli-rs
  example: goose review --instructions "focus on regressions"
  flag: --instructions
  invocation_scope:
  - applies_to: command
    command:
    - review
  notes: Same spelling as run --instructions but takes free text, not a file; a separate record for that reason.
  scope:
  - review
  - input
  value: <TEXT>
  value_optional: false
  value_type: string
- aliases:
  - -f
  attachment:
  - space
  - equals
  - short_attached
  description: Restrict the review to a specific set of files.
  evidence_ids:
  - binary-help-1-52-0
  - parse-tests
  - src-cli-rs
  example: goose review --files src/lib.rs docs/guide.md
  flag: --files
  invocation_scope:
  - applies_to: command
    command:
    - review
  notes: num_args = 1.., so one occurrence consumes several space-separated values; an occurrence with no value is rejected with a value-required error.
  scope:
  - review
  value: <FILE>...
  value_type: variadic
  variadic_min: 1
- aliases:
  - -c
  attachment:
  - space
  - equals
  - short_attached
  description: Only run checks whose name matches one of these.
  evidence_ids:
  - binary-help-1-52-0
  - parse-tests
  - src-cli-rs
  example: goose review --check-filter security performance
  flag: --check-filter
  invocation_scope:
  - applies_to: command
    command:
    - review
  notes: num_args = 1..; other discovered checks are skipped.
  scope:
  - review
  value: <NAME>...
  value_type: variadic
  variadic_min: 1
- aliases:
  - -s
  attachment:
  - space
  - equals
  - short_attached
  description: Alternate directory to search for .agents/checks instead of the repo root.
  evidence_ids:
  - binary-help-1-52-0
  - src-cli-rs
  example: goose review --check-scope .
  flag: --check-scope
  invocation_scope:
  - applies_to: command
    command:
    - review
  scope:
  - review
  value: <DIR>
  value_optional: false
  value_type: string
- attachment: []
  description: Skip the main correctness pass and only run check subagents.
  evidence_ids:
  - binary-help-1-52-0
  - src-cli-rs
  example: goose review --checks-only
  flag: --checks-only
  invocation_scope:
  - applies_to: command
    command:
    - review
  scope:
  - review
  value_type: none
- attachment: []
  description: Print only the diff summary and skip the full review.
  evidence_ids:
  - binary-help-1-52-0
  - src-cli-rs
  example: goose review --summary-only
  flag: --summary-only
  invocation_scope:
  - applies_to: command
    command:
    - review
  scope:
  - review
  value_type: none
- attachment:
  - space
  - equals
  default: medium
  description: Minimum severity to display; findings below are dropped.
  evidence_ids:
  - binary-help-1-52-0
  - src-cli-rs
  example: goose review --severity low
  flag: --severity
  invocation_scope:
  - applies_to: command
    command:
    - review
  scope:
  - review
  value: <LEVEL>
  value_optional: false
  value_type: string
- aliases:
  - -f
  attachment:
  - space
  - equals
  - short_attached
  default: text
  description: Select the session list output format.
  evidence_ids:
  - binary-help-1-52-0
  - json-lists
  - src-cli-rs
  example: goose session list --format json
  flag: --format
  invocation_scope:
  - applies_to: command
    command:
    - session
    - list
  notes: Possible values text and json; json prints a bare JSON array. Distinct from recipe list --format, which has no short alias, and from session export --format.
  scope:
  - output
  value: <FORMAT>
  value_optional: false
  value_type: string
- attachment: []
  description: Sort sessions oldest first instead of the default newest first.
  evidence_ids:
  - binary-help-1-52-0
  - src-cli-rs
  example: goose session list --ascending
  flag: --ascending
  invocation_scope:
  - applies_to: command
    command:
    - session
    - list
  scope:
  - output
  value_type: none
- aliases:
  - -w
  - -p
  attachment:
  - space
  - equals
  - short_attached
  description: Filter sessions by working directory.
  evidence_ids:
  - binary-help-1-52-0
  - parse-tests
  - src-cli-rs
  example: goose session list --working_dir ~/src/project
  flag: --working_dir
  invocation_scope:
  - applies_to: command
    command:
    - session
    - list
  notes: The underscore spelling is canonical; -p is a hidden short alias (short_alias in source, verified by a parse probe).
  scope:
  - session_identity
  value: <WORKING_DIR>
  value_optional: false
  value_type: string
- aliases:
  - -o
  attachment:
  - space
  - equals
  - short_attached
  default: stdout
  description: Write the exported session to a path instead of stdout.
  evidence_ids:
  - binary-help-1-52-0
  - src-cli-rs
  example: goose session export --session-id 20251108_4 --format json --output session.json
  flag: --output
  invocation_scope:
  - applies_to: command
    command:
    - session
    - export
  scope:
  - output
  value: <OUTPUT>
  value_optional: false
  value_type: string
- attachment:
  - space
  - equals
  default: markdown
  description: Select the session export format.
  evidence_ids:
  - binary-help-1-52-0
  - src-cli-rs
  example: goose session export --session-id 20251108_4 --format json
  flag: --format
  invocation_scope:
  - applies_to: command
    command:
    - session
    - export
  notes: Possible values markdown, json, and yaml; json and yaml are machine-readable.
  scope:
  - output
  value: <FORMAT>
  value_optional: false
  value_type: string
- attachment: []
  description: Publish the JSON session export as an encrypted Nostr event and print a goose share link.
  evidence_ids:
  - binary-help-1-52-0
  - src-cli-rs
  example: goose session export --format json --nostr
  flag: --nostr
  invocation_scope:
  - applies_to: command
    command:
    - session
    - export
  notes: Network side effect. The same spelling on session import means treat input as a share link; separate record below.
  scope:
  - sharing
  value_type: none
- attachment:
  - space
  - equals
  description: Nostr relay URL to publish to; may be specified multiple times.
  evidence_ids:
  - binary-help-1-52-0
  - src-cli-rs
  example: goose session export --nostr --relay wss://relay.example
  flag: --relay
  invocation_scope:
  - applies_to: command
    command:
    - session
    - export
  notes: Repeatable via ArgAction::Append.
  scope:
  - sharing
  value: <RELAY>
  value_optional: false
  value_type: string
- attachment: []
  description: Treat the input as an encrypted Nostr share link.
  evidence_ids:
  - binary-help-1-52-0
  - src-cli-rs
  example: goose session import --nostr goose://sessions/nostr/<id>
  flag: --nostr
  invocation_scope:
  - applies_to: command
    command:
    - session
    - import
  notes: session import's input is a positional argument, not a switch.
  scope:
  - sharing
  value_type: none
- aliases:
  - -o
  attachment:
  - space
  - equals
  - short_attached
  default: diagnostics_{session_id}.json in the working directory
  description: Write the diagnostics report to a specific path.
  evidence_ids:
  - binary-help-1-52-0
  - docs-cli
  - src-cli-rs
  example: goose session diagnostics --session-id 20251108_5 --output report.json
  flag: --output
  invocation_scope:
  - applies_to: command
    command:
    - session
    - diagnostics
  notes: The output is JSON containing session content, config, and logs; treat it as sensitive.
  scope:
  - diagnostics
  value: <OUTPUT>
  value_optional: false
  value_type: string
- aliases:
  - -n
  attachment:
  - space
  - equals
  - short_attached
  description: Name the terminal-integrated session created by the shell hook.
  evidence_ids:
  - binary-help-1-52-0
  - src-cli-rs
  example: goose term init zsh --name work
  flag: --name
  invocation_scope:
  - applies_to: command
    command:
    - term
    - init
  notes: Distinct from the session identifier --name; scoped to term init.
  scope:
  - terminal
  value: <NAME>
  value_optional: false
  value_type: string
- attachment: []
  description: Make goose the default handler for unknown shell commands.
  evidence_ids:
  - binary-help-1-52-0
  - src-cli-rs
  example: goose term init zsh --default
  flag: --default
  invocation_scope:
  - applies_to: command
    command:
    - term
    - init
  notes: Supported for zsh, bash, and nu.
  scope:
  - terminal
  value_type: none
- attachment:
  - space
  - equals
  description: Only include repos whose id starts with this prefix.
  evidence_ids:
  - binary-help-1-52-0
  - src-cli-rs
  example: goose local-models search qwen --repo-prefix Qwen
  flag: --repo-prefix
  invocation_scope:
  - applies_to: command
    command:
    - local-models
    - search
  scope:
  - local_models
  value: <REPO_PREFIX>
  value_optional: false
  value_type: string
- attachment:
  - space
  - equals
  description: Only include repos whose id ends with this suffix.
  evidence_ids:
  - binary-help-1-52-0
  - src-cli-rs
  example: goose local-models search gguf --repo-suffix GGUF
  flag: --repo-suffix
  invocation_scope:
  - applies_to: command
    command:
    - local-models
    - search
  scope:
  - local_models
  value: <REPO_SUFFIX>
  value_optional: false
  value_type: string
- attachment:
  - space
  - equals
  description: Only include variants whose quantization contains this text.
  evidence_ids:
  - binary-help-1-52-0
  - src-cli-rs
  example: goose local-models search llama --quant Q4
  flag: --quant
  invocation_scope:
  - applies_to: command
    command:
    - local-models
    - search
  scope:
  - local_models
  value: <QUANT>
  value_optional: false
  value_type: string
- attachment:
  - space
  - equals
  description: Override the available memory, in GB, used for recommendations.
  evidence_ids:
  - binary-help-1-52-0
  - src-cli-rs
  example: goose local-models search llama --ram-gb 16
  flag: --ram-gb
  invocation_scope:
  - applies_to: command
    command:
    - local-models
    - search
  notes: Parsed as f64.
  scope:
  - local_models
  value: <RAM_GB>
  value_optional: false
  value_type: number
- attachment: []
  description: Print local-model search results as JSON.
  evidence_ids:
  - binary-help-1-52-0
  - src-cli-rs
  example: goose local-models search qwen --json
  flag: --json
  invocation_scope:
  - applies_to: command
    command:
    - local-models
    - search
  notes: Not present in 1.41.0 and not available on local-models list.
  scope:
  - output
  value_type: none
- attachment:
  - space
  - equals
  description: JSON probe script driving the hidden mcp-probe inspection; use - for stdin.
  evidence_ids:
  - binary-help-1-52-0
  - src-cli-rs
  example: goose mcp-probe "npx -y @modelcontextprotocol/server-memory" --script probe.json
  flag: --script
  invocation_scope:
  - applies_to: command
    command:
    - mcp-probe
  notes: mcp-probe is hidden from help; the positional extension command is required.
  scope:
  - diagnostics
  value: <PATH|->
  value_optional: false
  value_type: string
config_paths:
- format: yaml
  notes: Observed with an isolated HOME on the 1.52.0 binary (XDG strategy; the Block app-strategy arguments in source are vestigial). Env vars override file values.
  os: macos
  path: ~/.config/goose/config.yaml
  scope: user
- format: yaml
  notes: XDG default; honors XDG_CONFIG_HOME per the etcetera XDG strategy.
  os: linux
  path: ~/.config/goose/config.yaml
  scope: user
- format: yaml
  notes: 'Source-derived: etcetera runs without its native feature, so the XDG strategy applies on Windows too. The docs still claim an APPDATA-based Block path, which contradicts the 1.52.0 source; verify on a Windows host before relying on either.'
  os: windows
  path: '%USERPROFILE%\.config\goose\config.yaml'
  scope: user
- format: yaml
  notes: Lowest-precedence system config (source base.rs).
  os: macos
  path: /etc/goose/config.yaml
  scope: system
- format: yaml
  notes: Lowest-precedence system config (source base.rs).
  os: linux
  path: /etc/goose/config.yaml
  scope: system
- format: yaml
  notes: Lowest-precedence system config; falls back to C:\ProgramData\goose\config.yaml when PROGRAMDATA is unset (source base.rs).
  os: windows
  path: '%PROGRAMDATA%\goose\config.yaml'
  scope: system
- format: yaml
  notes: When GOOSE_PATH_ROOT holds an absolute path, config/, data/, state/, .agents/plugins/, and .agents/agents/ all reroute under it.
  os: macos
  path: GOOSE_PATH_ROOT/config/config.yaml
  scope: env
- format: yaml
  notes: Same rerouting as macOS; a relative GOOSE_PATH_ROOT is ignored.
  os: linux
  path: GOOSE_PATH_ROOT/config/config.yaml
  scope: env
- format: yaml
  notes: Same rerouting as macOS.
  os: windows
  path: '%GOOSE_PATH_ROOT%\config\config.yaml'
  scope: env
- format: yaml
  notes: Path-list (OS path separator) of extra YAML configs layered between the system config and the user config.
  os: macos
  path: GOOSE_ADDITIONAL_CONFIG_FILES
  scope: env
- format: yaml
  notes: Path-list (colon-separated) of extra YAML configs between system and user config.
  os: linux
  path: GOOSE_ADDITIONAL_CONFIG_FILES
  scope: env
- format: yaml
  notes: Path-list (semicolon-separated) of extra YAML configs between system and user config.
  os: windows
  path: GOOSE_ADDITIONAL_CONFIG_FILES
  scope: env
- format: yaml
  notes: Tool permission levels written by goose configure; details belong to the agent-permissions topic.
  os: macos
  path: ~/.config/goose/permission.yaml
  scope: user
- format: yaml
  notes: Tool permission levels written by goose configure.
  os: linux
  path: ~/.config/goose/permission.yaml
  scope: user
- format: yaml
  notes: Source-derived XDG location; docs claim an APPDATA-based Block path.
  os: windows
  path: '%USERPROFILE%\.config\goose\permission.yaml'
  scope: user
- format: yaml
  notes: Plaintext secrets used only when file-based secret storage is active (keyring unavailable or GOOSE_DISABLE_KEYRING set).
  os: macos
  path: ~/.config/goose/secrets.yaml
  scope: user
- format: yaml
  notes: Plaintext secrets when file-based storage is active.
  os: linux
  path: ~/.config/goose/secrets.yaml
  scope: user
- format: yaml
  notes: Plaintext secrets when file-based storage is active; docs claim an APPDATA-based Block path.
  os: windows
  path: '%USERPROFILE%\.config\goose\secrets.yaml'
  scope: user
- format: json
  notes: Auto-managed runtime permission decisions.
  os: macos
  path: ~/.config/goose/permissions/tool_permissions.json
  scope: user
- format: json
  notes: Auto-managed runtime permission decisions.
  os: linux
  path: ~/.config/goose/permissions/tool_permissions.json
  scope: user
- format: json
  notes: Auto-managed runtime permission decisions (XDG-derived location).
  os: windows
  path: '%USERPROFILE%\.config\goose\permissions\tool_permissions.json'
  scope: user
- format: other
  notes: Customized prompt templates directory.
  os: macos
  path: ~/.config/goose/prompts/
  scope: user
- format: other
  notes: Customized prompt templates directory.
  os: linux
  path: ~/.config/goose/prompts/
  scope: user
- format: other
  notes: Customized prompt templates directory.
  os: windows
  path: '%USERPROFILE%\.config\goose\prompts\'
  scope: user
- format: other
  notes: SQLite session store since 1.10.0 (legacy .jsonl files are no longer managed). Observed under an isolated HOME.
  os: macos
  path: ~/.local/share/goose/sessions/sessions.db
  scope: user
- format: other
  notes: SQLite session store.
  os: linux
  path: ~/.local/share/goose/sessions/sessions.db
  scope: user
- format: other
  notes: SQLite session store (XDG-derived location).
  os: windows
  path: '%USERPROFILE%\.local\share\goose\sessions\sessions.db'
  scope: user
- format: other
  notes: Copies of recipes registered by schedule add (documented).
  os: macos
  path: ~/.local/share/goose/scheduled_recipes
  scope: user
- format: other
  notes: Copies of recipes registered by schedule add.
  os: linux
  path: ~/.local/share/goose/scheduled_recipes
  scope: user
- format: other
  notes: Copies of recipes registered by schedule add.
  os: windows
  path: '%USERPROFILE%\.local\share\goose\scheduled_recipes'
  scope: user
- format: other
  notes: Log tree; goose info creates a dated CLI log file here even with no config present. Log content belongs to the agent-logging topic.
  os: macos
  path: ~/.local/state/goose/logs/
  scope: user
- format: other
  notes: Log tree; same creation side effect.
  os: linux
  path: ~/.local/state/goose/logs/
  scope: user
- format: other
  notes: Log tree (XDG-derived location).
  os: windows
  path: '%USERPROFILE%\.local\state\goose\logs\'
  scope: user
- format: other
  notes: Installed plugins live under ~/.agents/plugins/<plugin-name>/ (home-based, not XDG).
  os: macos
  path: ~/.agents/plugins/
  scope: user
- format: other
  notes: Installed plugins under ~/.agents/plugins/<plugin-name>/.
  os: linux
  path: ~/.agents/plugins/
  scope: user
- format: other
  notes: Installed plugins (home-based).
  os: windows
  path: '%USERPROFILE%\.agents\plugins\'
  scope: user
- format: other
  notes: Subagent definitions directory exposed by source; skills also discover from ~/.agents/skills.
  os: macos
  path: ~/.agents/agents/
  scope: user
- format: other
  notes: Subagent definitions directory; skills from ~/.agents/skills.
  os: linux
  path: ~/.agents/agents/
  scope: user
- format: other
  notes: Subagent definitions directory; skills from ~/.agents/skills.
  os: windows
  path: '%USERPROFILE%\.agents\agents\'
  scope: user
env_vars:
- effect: Absolute path rerouting all goose config/, data/, state/, and .agents directories under it; a relative value is ignored.
  name: GOOSE_PATH_ROOT
- effect: Path list of extra YAML config files layered between the system config and the user config.
  name: GOOSE_ADDITIONAL_CONFIG_FILES
- effect: Disables system keyring secret storage, forcing file-based secrets.yaml; set it for unattended runs to avoid keychain prompts that block.
  name: GOOSE_DISABLE_KEYRING
- effect: Disables the background model call that names sessions; headless runs keep the default name.
  name: GOOSE_DISABLE_SESSION_NAMING
- effect: Uses an external editor for composing interactive prompts instead of CLI input.
  name: GOOSE_PROMPT_EDITOR
- effect: Force-opens the external prompt editor for every turn rather than only long prompts.
  name: GOOSE_PROMPT_EDITOR_ALWAYS
- effect: 'Sets the CLI markdown theme: light, dark, or ansi.'
  name: GOOSE_CLI_THEME
- effect: Sets the bat syntax theme used for light-mode rendering.
  name: GOOSE_CLI_LIGHT_THEME
- effect: Sets the bat syntax theme used for dark-mode rendering.
  name: GOOSE_CLI_DARK_THEME
- effect: Customizes the Ctrl+key character used to insert newlines in CLI input.
  name: GOOSE_CLI_NEWLINE_KEY
- effect: Rings the terminal bell when an interactive turn finishes or tool approval is required.
  name: GOOSE_CLI_BELL
- effect: Shows model reasoning and thinking output in CLI responses when the model exposes it.
  name: GOOSE_CLI_SHOW_THINKING
- effect: Controls the random progress messages shown while processing.
  name: GOOSE_RANDOM_THINKING_MESSAGES
- effect: Toggles display of model cost estimates in CLI output.
  name: GOOSE_CLI_SHOW_COST
- effect: Line threshold before CLI code blocks are truncated to a temp file.
  name: GOOSE_MAX_CODE_BLOCK_LINES
- effect: Lines shown before the truncated-lines marker.
  name: GOOSE_TRUNCATED_SHOW_LINES
- effect: Disables CLI code block truncation entirely.
  name: GOOSE_NO_CODE_TRUNCATION
- effect: Prepends directories to the command search path used for extension commands.
  name: GOOSE_SEARCH_PATHS
- effect: Overrides the shell used for Developer extension shell commands; goose injects the right flags per shell.
  name: GOOSE_SHELL
- effect: Shared secret required by goose serve unless --dangerously-unauthenticated is passed.
  name: GOOSE_SERVER__SECRET_KEY
- effect: Environment equivalent of goose serve --tls.
  name: GOOSE_TLS
- effect: Environment equivalent of goose serve --tls-cert-path; enables TLS together with GOOSE_TLS_KEY_PATH.
  name: GOOSE_TLS_CERT_PATH
- effect: Environment equivalent of goose serve --tls-key-path; enables TLS together with GOOSE_TLS_CERT_PATH.
  name: GOOSE_TLS_KEY_PATH
- effect: Additional recipe search directories; colon-separated on Unix and semicolon-separated on Windows.
  name: GOOSE_RECIPE_PATH
- effect: GitHub repository, as owner/repo, searched for recipes and sub-recipes.
  name: GOOSE_RECIPE_GITHUB_REPO
- effect: Global timeout for recipe success-check commands.
  name: GOOSE_RECIPE_RETRY_TIMEOUT_SECONDS
- effect: Global timeout for recipe on-failure commands.
  name: GOOSE_RECIPE_ON_FAILURE_TIMEOUT_SECONDS
- effect: Documentation root used by the goose-doc-guide skill for offline docs.
  name: GOOSE_DOCS_ROOT
- effect: Fixes the local OAuth callback port for identity providers that forbid wildcard redirect URIs.
  name: GOOSE_OAUTH_CALLBACK_PORT
- effect: Set to 1 by goose when executing commands, so shell configs can detect goose execution.
  name: GOOSE_TERMINAL
- effect: Set to goose by goose in command and extension contexts for cross-agent detection.
  name: AGENT
- effect: Current goose session id, exported to stdio extensions and Developer shell commands.
  name: AGENT_SESSION_ID
- effect: 'Installer-only: overrides the target binary directory of the shell installer.'
  name: GOOSE_BIN_DIR
- effect: 'Installer-only: pins a specific release for reproducible installs (documented for CI).'
  name: GOOSE_VERSION
- effect: 'Installer-only: selects canary release assets when true.'
  name: CANARY
- effect: 'Installer-only: false skips the interactive goose configure step after install.'
  name: CONFIGURE
- effect: 'Installer-only: selects the Linux asset variant standard, vulkan, or musl.'
  name: GOOSE_LINUX_VARIANT
- effect: 'Installer-only: selects the Windows asset variant standard or cuda.'
  name: GOOSE_WINDOWS_VARIANT
- effect: 'Installer-only: overrides OS detection with linux, windows, or darwin.'
  name: INSTALL_OS
machine_introspection:
- command: goose info --verbose
  machine_readable: false
  notes: Prints version, resolved paths, and merged config as YAML-like blocks inside human text; also reports missing paths. Creates a log file under state/logs as a side effect.
  output_format: yaml
  purpose: config_dump
  useful_for_codegen: false
- command: goose info --check
  machine_readable: false
  notes: Performs a provider request; exits 1 reporting an unconfigured provider when none is set.
  output_format: text
  purpose: doctor
  useful_for_codegen: false
- command: goose session list --format json
  machine_readable: true
  notes: Bare JSON array of sessions (id, working_dir, name, user_set_name, session_type, timestamps, usage); filterable by --working_dir and --limit.
  output_format: json
  purpose: other
  useful_for_codegen: false
- command: goose session export --session-id <id> --format json
  machine_readable: true
  notes: Full session backup as JSON; exits 1 with Session not found for unknown ids.
  output_format: json
  purpose: other
  useful_for_codegen: false
- command: goose session diagnostics --session-id <id> --output <file>
  machine_readable: true
  notes: Writes a diagnostics JSON with system info, full session content, config files, and logs; treat the file as sensitive.
  output_format: json
  purpose: doctor
  useful_for_codegen: false
- command: goose recipe list --format json
  machine_readable: true
  notes: Bare JSON array of discovered recipes; observed printing an empty array on a clean root.
  output_format: json
  purpose: other
  useful_for_codegen: false
- command: goose skills list
  machine_readable: false
  notes: Text table of skills with token counts and source locations; scans the user's real ~/.claude/skills and ~/.research regardless of GOOSE_PATH_ROOT; no JSON flag.
  output_format: table
  purpose: capabilities
  useful_for_codegen: false
- command: goose local-models search <query> --json
  machine_readable: true
  notes: Hugging Face search results as JSON; a network call. goose local-models list covers only downloaded models and stays text.
  output_format: json
  purpose: models
  useful_for_codegen: false
- command: goose run --output-format json --no-session -t <prompt>
  machine_readable: true
  notes: One-shot run returning a single JSON object after completion; requires a configured provider.
  output_format: json
  purpose: other
  useful_for_codegen: false
- command: goose run --output-format stream-json --no-session -t <prompt>
  machine_readable: true
  notes: Streams newline-delimited JSON events during the run; the best live wrapper surface.
  output_format: jsonl
  purpose: other
  useful_for_codegen: false
wrapper_notes:
- 'The project moved: github.com/block/goose answers Moved Permanently to aaif-goose/goose, and block.github.io/goose redirects to goose-docs.ai. Use the AAIF repository and goose-docs.ai URLs; some artifact names still contain the old Block identity.'
- No goose binary is installed on this host. All local evidence comes from the downloaded v1.52.0 macOS arm64 release asset run in a disposable temp dir with an isolated GOOSE_PATH_ROOT.
- 'Default paths are XDG-style on every OS in 1.52.0: ~/.config/goose, ~/.local/share/goose, ~/.local/state/goose. The docs'' macOS and Windows default-location tables are stale; the Windows location recorded here is source-derived and should be confirmed on a Windows host.'
- 'goose info is not side-effect free: it creates a dated log file under state/logs even when config.yaml is missing.'
- goose run persists a session row in sessions.db even when the provider check then fails; pass --no-session for hermetic wrapper runs.
- goose session remove always seeks confirmation and fails on closed stdin; it is not scriptable.
- 'Input flags are mutually exclusive: -t/--text, -i/--instructions, and --recipe conflict pairwise, and --system conflicts with --recipe. --no-session conflicts with --resume, --name, and --path.'
- The identifier group (-n/--name, --session-id with --id alias, --path) accepts at most one switch per invocation.
- '-t/--text has no allow_hyphen_values: a prompt starting with a dash is rejected with a value-required error. Forward such prompts as --text=<prompt> or -t<prompt>.'
- goose run --output-format stream-json --no-session is the best live wrapper surface; json emits one object after completion. Parse errors exit 2; runtime errors exit 1.
- goose skills list reads the user's real ~/.claude/skills and ~/.research directories even under GOOSE_PATH_ROOT; skill discovery is not isolated by it.
- 'Docs and binary drift in 1.52.0: session rename is documented but rejected by the binary; the goose mcp example naming ''Google Drive'' does not parse (only the four bundled server names do); --with-remote-extension remains rejected with a suggestion to use --with-extension.'
- 'Hidden but callable surfaces: validate-extensions, mcp-probe, term log, run --scheduled-job-id, and the --id aliases on session and schedule identifiers.'
- 'Interactive surfaces to avoid in wrappers: configure, doctor, session at the top level, run --interactive, update --reconfigure, term run, plugin install and update, gateway start and pair, and recipe open.'
- goose serve requires GOOSE_SERVER__SECRET_KEY unless --dangerously-unauthenticated is passed; wrappers must not add that flag themselves.
- Keychain access can block unattended processes on macOS; set GOOSE_DISABLE_KEYRING and supply provider credentials through the environment for headless acp and serve runs.
- Provider API keys are never read from config.yaml; they live in the keyring, in secrets.yaml, or in provider environment variables. Debug output, diagnostics files, session exports, and verbose info can contain prompts, tool output, paths, config, and secrets.
changes:
- Refreshed from release v1.41.0 to v1.52.0 (published 2026-09-23), verifying the release binary directly; the document now follows contract revision 2 with a typed switch inventory covering value types, optional values, attachment forms, and invocation scopes.
- 'Removed commands: tui, project, and projects are gone from the 1.52.0 binary. A new roam command exists only behind a compile-time feature and is absent from the release binary. New hidden command mcp-probe with --script.'
- 'New switches: --provider and --model on run and review; --enable-scheduler on acp and serve; local-models search gained --json, --repo-prefix, --repo-suffix, --quant, and --ram-gb; session list --working_dir gained the hidden -p short alias.'
- 'Corrected default path discovery: XDG-style paths on every OS via etcetera without the native feature; recorded the stale docs claims for macOS and Windows default locations and flagged the Windows location as source-derived.'
- 'Recorded environment variables new to this refresh: GOOSE_DISABLE_SESSION_NAMING, GOOSE_PROMPT_EDITOR_ALWAYS, GOOSE_CLI_BELL, GOOSE_TLS, GOOSE_TLS_CERT_PATH, GOOSE_TLS_KEY_PATH, GOOSE_OAUTH_CALLBACK_PORT, and GOOSE_DOCS_ROOT.'
- 'Verified wrapper-relevant behavior on the binary: run persists a session row before failing on an unconfigured provider; session remove always asks confirmation; skills list scans the user''s real ~/.claude/skills; session rename is documented but absent; --with-remote-extension is still rejected.'
- 'Attachment and value-type facts now rest on named evidence: clap 4.1.14 declarations pinned at tag v1.52.0 plus disposable parse probes whose errors name the switch.'
requires_claudine_update: true
reason: 'The typed switch inventory feeds the generated ProviderInfo catalog, and 1.52.0 changed the surface Claudine wraps: run gained --provider and --model, the resume path is still run -r, tui/project/projects were removed, new hidden flags and aliases (--id, -p, --scheduled-job-id, mcp-probe) affect argument partitioning, and the corrected XDG default paths (especially the source-derived Windows location) affect config discovery and MCP export guidance.'
contract_checked: 2026-10-01
---

# Goose CLI Research

## Overview

Goose is an open-source local AI agent with Desktop, CLI, and API surfaces, stewarded by the Agentic AI Foundation (AAIF). The command a user types is `goose`; the automation entry point a wrapper drives is `goose run`. The argument parser is clap 4 (derive), pinned at `clap = "4.1.14"` in the workspace manifest, and it is the library-level basis for the attachment forms recorded here.

The version verified in this run is **1.52.0**: the newest upstream release is [v1.52.0](https://github.com/aaif-goose/goose/releases/tag/v1.52.0) (published 2026-09-23, read from the [releases API](https://api.github.com/repos/aaif-goose/goose/releases/latest) on 2026-10-01), and the downloaded `goose-aarch64-apple-darwin` release asset printed `1.52.0` for both `goose --version` and `goose -V`. No `goose` is installed on this host's `PATH`; every local observation comes from that disposable binary, run with an isolated `GOOSE_PATH_ROOT`.

The legacy locations still redirect: [github.com/block/goose](https://github.com/block/goose) answers *Moved Permanently* to [aaif-goose/goose](https://github.com/aaif-goose/goose), and [block.github.io/goose](https://block.github.io/goose/) serves a "goose has moved" page pointing at [goose-docs.ai](https://goose-docs.ai/).

Primary links:

- Homepage: [goose-docs.ai](https://goose-docs.ai/)
- Repository: [aaif-goose/goose](https://github.com/aaif-goose/goose)
- General docs: [goose-docs.ai/docs](https://goose-docs.ai/docs/)
- CLI reference: [CLI Commands guide](https://goose-docs.ai/docs/guides/goose-cli-commands)

## Installation and Binaries

The executable is `goose` on macOS and Linux and `goose.exe` in the native Windows release asset; Windows users type `goose` once the install directory is on `PATH`.

| OS | Method | Command | Notes |
| --- | --- | --- | --- |
| macOS | Shell installer | `curl -fsSL https://github.com/aaif-goose/goose/releases/download/stable/download_cli.sh \| bash` | Installs under the user's `.local/bin` (`GOOSE_BIN_DIR` overrides) and runs interactive `goose configure` unless `CONFIGURE=false`. |
| macOS | Homebrew | `brew install block-goose-cli` | Precompiled CLI formula. |
| Linux | Shell installer | `curl -fsSL https://github.com/aaif-goose/goose/releases/download/stable/download_cli.sh \| bash` | `GOOSE_LINUX_VARIANT` selects standard, vulkan, or musl. |
| Windows | Shell installer (Git Bash/MSYS2) | `curl -fsSL https://github.com/aaif-goose/goose/releases/download/stable/download_cli.sh \| bash` | `GOOSE_WINDOWS_VARIANT` selects standard or cuda; the docs' PowerShell PATH snippet installs under the user profile's `.local\bin`. |
| Windows | PowerShell | `Invoke-WebRequest -Uri "https://raw.githubusercontent.com/aaif-goose/goose/main/download_cli.ps1" -OutFile "download_cli.ps1"; .\download_cli.ps1` | Official PowerShell installer. |
| Windows | WSL | the same shell installer, inside the distro | Docs recommend the native install instead. |

`GOOSE_VERSION` pins a release for reproducible CI installs. The `stable` release tag carries the same assets as numbered releases, including Linux deb/rpm/Flatpak Desktop packages.

## Subcommands

Every native command path below the executable, as listed by the 1.52.0 binary (aliases in parentheses). The root entrypoint itself is not a subcommand. Paths marked *non-interactive* run to completion with no terminal, browser, or person answering a prompt.

| Path | Alias | Non-interactive | Description |
| --- | --- | --- | --- |
| `configure` | | no | Interactive provider/extension/settings wizard; may open a browser or touch the keyring. |
| `info` | | yes | Version, resolved paths, merged config (`--verbose`); provider check (`--check`, exit 1 when unconfigured). |
| `doctor` | | no | Interactive setup diagnostic. |
| `mcp` | | yes | Run one bundled MCP server over stdio: `autovisualiser`, `computercontroller`, `memory`, `tutorial`. |
| `acp` | | yes | ACP agent server over stdio, meant to be spawned by clients such as Zed. |
| `serve` | | yes | ACP server over HTTP/WebSocket; needs `GOOSE_SERVER__SECRET_KEY` unless `--dangerously-unauthenticated`. |
| `session` | `s` | no | Interactive chat; nested `list`, `export`, `import`, `diagnostics` are scriptable when an identifier is supplied. |
| `run` | | yes | One-shot prompt/instruction-file/stdin/recipe execution; the resume path is `run -r`/`--resume`. |
| `recipe validate` | | yes | Validate one recipe file. |
| `recipe deeplink` | | yes | Generate a shareable deeplink. |
| `recipe open` | | no | Open a recipe in goose Desktop. |
| `recipe list` | | yes | List recipes; `--format json`. |
| `skills list` | | yes | Text table of discoverable skills. |
| `plugin install` | | no | Git-backed plugin install; credential prompts possible. |
| `plugin update` | | no | Git-backed plugin update. |
| `schedule add` | `sched` | yes | Register a cron job from a recipe; id, cron, and recipe-source required. |
| `schedule list` | `sched` | yes | List scheduled jobs (text). |
| `schedule remove` | `sched` | yes | Remove a job by `--schedule-id`. |
| `schedule sessions` | `sched` | yes | List sessions created by one schedule. |
| `schedule run-now` | `sched` | yes | Trigger a job immediately (runs a real agent). |
| `schedule services-status` | `sched` | yes | Deprecated services check. |
| `schedule services-stop` | `sched` | yes | Deprecated services stop. |
| `schedule cron-help` | `sched` | yes | Cron expression help. |
| `gateway status` | `gw` | yes | Show gateway status. |
| `gateway start` | `gw` | no | Start a gateway; needs `--bot-token`. |
| `gateway stop` | `gw` | yes | Stop a gateway by type. |
| `gateway pair` | `gw` | no | Generate a pairing code; external user action. |
| `update` | | no | Mutate the installed binary; `--reconfigure` prompts. |
| `term init` | | yes | Print the shell integration script. |
| `term run` | | no | Drive the interactive terminal session. |
| `term info` | | yes | Compact session info for shell prompts. |
| `term log` | | yes | Hidden; logs a shell command to the session. |
| `local-models search` | `lm` | yes | Hugging Face search; network; `--json`. |
| `local-models download` | `lm` | yes | Download a local model. |
| `local-models list` | `lm` | yes | List downloaded models (text). |
| `local-models delete` | `lm` | yes | Delete a downloaded model. |
| `completion` | | yes | Shell completions / Nushell module. |
| `review` | | yes | Diff review; may spawn up to 4 concurrent `goose run` subprocesses. |
| `help` | | yes | clap built-in help. |
| `validate-extensions` | | yes | Hidden; validates a bundled-extensions JSON file. |
| `mcp-probe` | | yes | Hidden; inspects a stdio MCP server without an LLM. |

Parent commands (`recipe`, `skills`, `plugin`, `schedule`, `gateway`, `term`, `local-models`) require a nested subcommand and exit with a usage error when called bare. Removed since 1.41.0: `tui`, `project`, and `projects` are rejected as unrecognized subcommands. A `roam` command exists in source behind a compile-time feature and is not in the release binary.

## CLI Switch Inventory

Inventoried from the release binary's help at the root entrypoint and at **every** path marked non-interactive above, including the `run` resume surface, cross-checked against the clap declarations in [`crates/goose-cli/src/cli.rs`](https://github.com/aaif-goose/goose/blob/v1.52.0/crates/goose-cli/src/cli.rs) at tag v1.52.0. Switches of interactive paths (`configure`, `doctor`, top-level `session`, `update`, `plugin`, `gateway start`/`pair`, `recipe open`, `term run`) are intentionally not recorded. The frontmatter `cli_switches` array is the full typed inventory; the essentials:

- **Parser**: clap 4.1.14 (derive). Long switches accept `space` and `equals` (`--text hi`, `--text=hi`); one-character spellings additionally accept `short_attached` (`-thi`) and equals with the sign stripped (`-t=hi`, verified by `-l=abc` failing with `invalid value 'abc'`).
- **Value types**: every boolean flag is `none` (passing a value is rejected with "no more were expected"); `--port`, `--limit`, `--max-turns`, `--max-tool-repetitions`, `--turn-limit`, and `--ram-gb` are `number`; `review --files` and `review --check-filter` are `variadic` (`num_args = 1..`, minimum one value); everything else takes exactly one `string`. No switch has an optional value.
- **Repeatable is not variadic**: `--with-extension`, `--with-streamable-http-extension`, `--allowed-origin`, `--relay`, `--params`, `--sub-recipe`, and `-p/--param` may each repeat, one value per occurrence, so they are `string`. `--with-builtin` additionally splits one occurrence on commas (`value_delimiter`).
- **Root entrypoint**: only `-h/--help` (present at every path) and `-V/--version` (root only; `goose run --version` is rejected).
- **Hidden spellings that still parse**: `--id` (alias of `--session-id` and of `--schedule-id`), `-p` (alias of `session list --working_dir`), `run --scheduled-job-id`, and `mcp-probe --script`.

The `goose run` facts a wrapper needs most:

| Concern | Switches |
| --- | --- |
| Input (mutually exclusive) | `-t/--text <TEXT>`, `-i/--instructions <FILE>` (`-` means stdin), `--recipe <name or path>` |
| Session identity (at most one) | `-n/--name`, `--session-id` (alias `--id`), `--path`; all conflict with `--no-session` |
| Resume | `-r/--resume` (most recent session when no identifier is given) |
| Output | `--output-format` (text, json, or stream-json; default text), `-q/--quiet`, `--stats` |
| Model selection | `--provider <PROVIDER>`, `--model <MODEL>` (override the `GOOSE_PROVIDER`/`GOOSE_MODEL` environment variables) |
| Extensions | `--with-extension`, `--with-streamable-http-extension`, `--with-builtin`, `--no-profile`, `--container` |
| Bounds | `--max-turns`, `--max-tool-repetitions` |
| System prompt | `--system <TEXT>` — spelling and value type recorded here; delivery semantics belong to the [system-prompt Goose research](../system-prompt/goose.md) |

How the facts were established: spellings, defaults, and possible values come from the binary's help; value types and attachment forms come from the clap declarations plus disposable probes whose failures name the switch — for example `run --output-format=bogus` fails with `invalid value 'bogus' for '--output-format <FORMAT>'`, `session list -labc` fails with `invalid value 'abc' for '--limit <LIMIT>'`, and `review --files` alone fails with "a value is required ... but none was supplied". A `--help` run exits before validation and proves nothing about parsing, so it was never used as attachment evidence.

## Configuration Discovery

Goose layers YAML configuration: the system config first (`/etc/goose/config.yaml` on Unix, under `PROGRAMDATA` on Windows), then every path listed in `GOOSE_ADDITIONAL_CONFIG_FILES`, then the user config. Environment variables override file values, and defaults sit below both. The most reliable discovery command is `goose info`, which prints the resolved paths — and creates a log file under `state/logs` as a side effect.

Default locations changed shape: goose 1.52.0 depends on `etcetera` **without** its `native` feature, so the XDG strategy applies on every OS and the `Block`/`goose` app-strategy arguments in `paths.rs` are vestigial (a stale source comment still cites an Apple-style Block path). Observed with an isolated `HOME` on macOS:

- Config: `~/.config/goose/config.yaml`
- Sessions DB: `~/.local/share/goose/sessions/sessions.db` (SQLite since 1.10.0)
- Logs: `~/.local/state/goose/logs/`

The docs pages still publish a macOS `~/Library/Application Support/Block/goose/` table and a Windows `%APPDATA%\Block\goose\` path; both contradict the 1.52.0 source. The Windows defaults recorded in the frontmatter are therefore source-derived (`%USERPROFILE%\.config\goose`, `\.local\share\goose`, `\.local\state\goose`) and were not host-verified in this run — confirm on a Windows host before relying on them.

Other discovered files: `permission.yaml` (tool permission levels, written by `goose configure`), `secrets.yaml` (only when file-based secret storage is active), `permissions/tool_permissions.json` (auto-managed runtime decisions), `prompts/` (custom prompt templates), `scheduled_recipes` (copies made by `schedule add`), `~/.agents/plugins/<name>/` (installed plugins), and `~/.agents/agents/` with `~/.agents/skills` (subagents and skills).

`GOOSE_PATH_ROOT` reroutes everything — `config/`, `data/`, `state/`, `.agents/plugins/`, `.agents/agents/` — under one absolute root (relative values are ignored), which is the cleanest way to isolate a wrapper's goose state. Note the limits of that isolation: `goose skills list` still read the user's real `~/.claude/skills` and `~/.research/library` in this run.

Provider API keys are never read from `config.yaml`; they live in the system keyring, in `secrets.yaml` when file-based storage is active, or in provider environment variables.

## Environment Variables

The frontmatter records the general CLI and runtime variables. Variables owned by narrower topics are deliberately absent: model endpoint and provider variables (`GOOSE_PROVIDER`, `GOOSE_MODEL`, `GOOSE_PROVIDER__*`, thinking and retry controls) belong to model-config; permission variables (`GOOSE_MODE`, toolshim, allowlist) to agent-permissions; MCP-specific variables to mcp; log-location and telemetry variables (OTEL, Langfuse) to agent-logging.

Highest-impact for wrappers:

- `GOOSE_PATH_ROOT` isolates config, data, state, and `.agents`; use it for hermetic runs.
- `GOOSE_DISABLE_KEYRING=1` avoids keychain authorization prompts that block unattended `acp`/`serve` runs (supply credentials through the environment instead).
- `GOOSE_DISABLE_SESSION_NAMING=true` suppresses the background model call that names sessions — useful in CI.
- `GOOSE_SERVER__SECRET_KEY` gates `goose serve` unless `--dangerously-unauthenticated` is passed.
- `GOOSE_PROMPT_EDITOR` can route interactive prompt composition through an external editor — avoid it in headless contexts.
- `GOOSE_CLI_THEME`, `GOOSE_CLI_SHOW_THINKING`, `GOOSE_CLI_SHOW_COST`, `GOOSE_CLI_BELL`, and the code-truncation variables shape human-facing output only.
- Goose sets `GOOSE_TERMINAL=1`, `AGENT=goose`, and `AGENT_SESSION_ID` in extension and shell contexts so scripts can detect and correlate goose execution.
- The installer-only variables (`GOOSE_BIN_DIR`, `GOOSE_VERSION`, `CANARY`, `CONFIGURE`, `GOOSE_LINUX_VARIANT`, `GOOSE_WINDOWS_VARIANT`, `INSTALL_OS`) affect `download_cli.sh`, not the running CLI.

## Machine Introspection

No single doctor, config, or model-catalog dump exists in 1.52.0, but several commands emit parseable output:

```bash
goose info --verbose                                     # human text with a YAML-like config block
goose info --check                                       # provider check; exit 1 when unconfigured
goose session list --format json                         # bare JSON array
goose session export --session-id <id> --format json     # full session backup
goose session diagnostics --session-id <id> --output f   # JSON diagnostics (sensitive)
goose recipe list --format json                          # bare JSON array
goose skills list                                        # text table only
goose local-models search <query> --json                 # Hugging Face results as JSON
goose run --output-format json --no-session -t "..."     # one JSON object after completion
goose run --output-format stream-json --no-session -t "..."  # NDJSON events
```

`goose local-models list` covers only downloaded local models (text). `goose skills list` has no JSON flag. Diagnostics files, session exports, and debug output can contain prompts, tool results, config, and secrets — do not collect them silently.

## Wrapper Notes

- Wrap `goose run --output-format stream-json --no-session -t <prompt>` for live execution and `--output-format json` for batch. Parse errors exit 2 (clap); runtime errors exit 1.
- Forward prompts safely: `-t` rejects values that start with a dash, so use `--text=<prompt>` or `-t<prompt>` for leading-dash prompts. Put `run` first so remaining flags parse as run options.
- Prefer `--no-session` for hermetic runs — without it, `goose run` writes a session row even when the provider check then fails — and `GOOSE_PATH_ROOT` for whole-state isolation (understanding that skill discovery still reads the real home).
- Do not script `goose session remove` (it always seeks confirmation) or any interactive surface: `configure`, `doctor`, top-level `session`, `run --interactive`, `update --reconfigure`, `term run`, `plugin install`/`update`, `gateway start`/`pair`, `recipe open`.
- Identifier and input rules the partitioner must respect: at most one of `--name`/`--session-id`/`--path`; `-t`/`-i`/`--recipe` mutually exclusive; `--system` conflicts with `--recipe`; `--no-session` conflicts with `--resume`, `--name`, and `--path`.
- Docs can lag the binary: `session rename` and `--with-remote-extension` are documented-adjacent but rejected in 1.52.0, and the `goose mcp` "Google Drive" example does not parse. Trust the binary's rejections — clap's error-context feature adds "similar argument exists" suggestions.
- `goose serve` requires `GOOSE_SERVER__SECRET_KEY` unless `--dangerously-unauthenticated` is passed; never add that flag automatically.
- Expect side effects from read-looking commands: `goose info` creates a log file, `goose run` creates sessions, `schedule add` copies recipes.
- Treat keyring-adjacent failures in unattended environments as a `GOOSE_DISABLE_KEYRING=1` matter, per the docs' ACP guidance.

## Sources

- [Goose release v1.52.0](https://github.com/aaif-goose/goose/releases/tag/v1.52.0) and the [releases API](https://api.github.com/repos/aaif-goose/goose/releases/latest), read 2026-10-01
- [Legacy repository block/goose](https://github.com/block/goose) (Moved Permanently) and [legacy docs site block.github.io/goose](https://block.github.io/goose/) (redirect page)
- [Install goose](https://goose-docs.ai/docs/getting-started/installation)
- [CLI Commands guide](https://goose-docs.ai/docs/guides/goose-cli-commands)
- [Configuration Files guide](https://goose-docs.ai/docs/guides/config-files)
- [Environment Variables guide](https://goose-docs.ai/docs/guides/environment-variables)
- [crates/goose-cli/src/cli.rs at v1.52.0](https://github.com/aaif-goose/goose/blob/v1.52.0/crates/goose-cli/src/cli.rs) — clap declarations for every switch
- [Workspace Cargo.toml at v1.52.0](https://github.com/aaif-goose/goose/blob/v1.52.0/Cargo.toml) — clap 4.1.14 and etcetera 0.11 (default-features off)
- [crates/goose/src/config/paths.rs at v1.52.0](https://github.com/aaif-goose/goose/blob/v1.52.0/crates/goose/src/config/paths.rs) — `GOOSE_PATH_ROOT` and the XDG strategy
- [crates/goose/src/config/base.rs at v1.52.0](https://github.com/aaif-goose/goose/blob/v1.52.0/crates/goose/src/config/base.rs) — config layering and env handling
- [crates/goose-mcp/src/mcp_server_runner.rs at v1.52.0](https://github.com/aaif-goose/goose/blob/v1.52.0/crates/goose-mcp/src/mcp_server_runner.rs) — bundled MCP server names
- Local: the downloaded `goose-aarch64-apple-darwin` release asset for v1.52.0, run in a disposable temp dir — `goose --version`, the full `--help` tree at every command path, and hidden-command probes
- Local: disposable parse probes whose clap errors name the switch (attachment, value-type, conflict, and alias proofs), run with an isolated `GOOSE_PATH_ROOT`
- Local: isolated-`HOME` and isolated-`GOOSE_PATH_ROOT` runs of `goose info`, `goose info --check`, `goose run`, `goose session list`/`export`/`remove`, `goose recipe list`, `goose skills list`, `goose schedule list`, `goose local-models list`, and `goose gateway status`

## Changelog

- 2026-10-01: refreshed to release v1.52.0 against the downloaded macOS binary; rewrote the switch inventory to contract revision 2 (typed value types, attachment forms, invocation scopes) with per-record evidence.
- 2026-10-01: recorded removals (`tui`, `project`, `projects`), additions (`--provider`/`--model` on run, `--enable-scheduler` on acp/serve, `local-models search --json` and its filters, hidden `mcp-probe`), and the hidden `--id`/`-p` aliases verified by parse probes.
- 2026-10-01: corrected default path discovery to XDG-on-every-OS from the 1.52.0 source; flagged the docs' macOS and Windows default-location tables as stale and the Windows location as source-derived.
- 2026-10-01: verified on the binary that `run` persists a session row before the provider check fails, that `session remove` always asks confirmation, that `skills list` scans the user's real `~/.claude/skills`, and that `session rename` remains documented but absent.
- 2026-07-03 and earlier: previous refresh against v1.41.0 under the revision 1 contract.