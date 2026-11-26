---
$schema: ./_schema.yaml
schema_revision: 2
provider: antigravity
created: 2026-07-08
last_updated: 2026-10-01
agent: opencode
model: zai-coding-plan/glm-5.3
reasoning_effort: provider_default
latest_version: 1.2.14
versions_examined:
- 1.2.13
- 1.2.14
homepage: https://antigravity.google/product/antigravity-cli
repo: https://github.com/google-antigravity/antigravity-cli
docs: https://antigravity.google/docs/cli/overview/
cli_docs: https://antigravity.google/docs/cli/reference/
evidence:
- claim: Root `agy --help` on the installed binary lists every documented root switch (--add-dir, --agent, -c/--continue, --conversation, --dangerously-skip-permissions, --disable-slash-commands, --effort, -i/--prompt-interactive, --input-format, --json-schema, --log-file, --mode, --model, --new-project, --output-format, -p/--print, --print-timeout, --project, --prompt, --prompt-interactive, --remote-control, --sandbox) with their defaults, plus the subcommand table (agent, agents, changelog, help, install, mcp, mic-serve, models, plugin, plugins, remote-control, update).
  id: help-root-12214
  limitations: Help output omits the hidden --help, --version, and -v flags; it does not state attachment forms or exit codes, which were established by parse probes.
  location: local command `agy --help` (binary /Users/ken/.local/bin/agy)
  method: local_inspection
  observed_on: 2026-10-01
  version: 1.2.14
- claim: '`agy --version` printed 1.2.13 at the start of the session and 1.2.14 later the same evening, while ~/.gemini/antigravity-cli/updater/update_status.json recorded `{"success":true,"message":"Update successful, restart CLI to use"}` and the binary mtime moved — the background self-updater replaced the installed binary during the research run.'
  id: version-check
  limitations: Does not establish what any switch does; only that the version string and self-update behavior are real.
  location: local commands `agy --version`, `stat /Users/ken/.local/bin/agy`, `cat ~/.gemini/antigravity-cli/updater/update_status.json`
  method: local_inspection
  observed_on: 2026-10-01
  version: 1.2.14
- claim: Per-subcommand `--help` output on 1.2.14 establishes each path's own flag table — install (--dir, --skip-aliases, --skip-path, -h, --help), mcp add (--env/-e, --header/-H, --type/-t, -h, --help plus operand and `--` rules), mcp remove/enable/disable/list and remote-control start/status/stop (-h, --help only, plus --name and --session on start), mic-serve (--addr with default 127.0.0.1:4713), models and agent/agents (-h, --help only), plugin (custom usage, no flag table), and `help <sub>` showing that subcommand's help.
  id: subhelp-battery
  limitations: The `update` subcommand prints only `Usage of update:`; the mcp and remote-control dispatchers show subcommand lists rather than flags.
  location: local commands `agy <sub> --help` for every listed subcommand path
  method: local_inspection
  observed_on: 2026-10-01
  version: 1.2.14
- claim: 'Long value flags take their value by space, by equals, and under a single-dash long spelling — `agy --print-timeout bogus`, `agy --print-timeout=bogus`, and `agy -print-timeout=bogus` all fail with `invalid value "bogus" for flag -print-timeout: time: invalid duration` and exit 2, and `agy -model x --print-timeout bogus` parses -model and reaches the timeout error.'
  id: parse-long-forms
  limitations: Proven directly for --print-timeout and -model; extends to the other long value flags through the shared flag parser, not per-flag probes.
  location: disposable parse-failure probes against the installed binary
  method: disposable_test
  observed_on: 2026-10-01
  version: 1.2.14
- claim: 'Boolean flags reject `=bogus` and never consume the next token — `--new-project=bogus`, `--continue=bogus`, `-c=x`, `--dangerously-skip-permissions=bogus`, `--disable-slash-commands=bogus`, `--remote-control=bogus`, `--sandbox=bogus`, and `--version=bogus` all fail with `invalid boolean value ... strconv.ParseBool` exit 2, `agy --new-project bogus` errors on the stray positional `bogus`, and `-cx` fails with `flags provided but not defined: -cx`.'
  id: parse-bool-forms
  limitations: Proves the equals form and non-consumption for the tested flags; the parser is shared, so untested booleans are recorded from help output plus this mechanism.
  location: disposable parse-failure probes against the installed binary
  method: disposable_test
  observed_on: 2026-10-01
  version: 1.2.14
- claim: 'Every root value flag requires its value — bare `--add-dir`, `--agent`, `--conversation`, `--effort`, `--input-format`, `--json-schema`, `--log-file`, `--mode`, `--model`, `--output-format`, `--print`, `--project`, `--prompt`, `--prompt-interactive`, `-p`, `-i`, and `-v` each fail with `flag needs an argument: -<name>` and exit 2, proving each takes exactly one value that cannot be omitted.'
  id: parse-needs-argument
  limitations: Establishes value-taking, not what the value means.
  location: disposable parse-failure probes against the installed binary
  method: disposable_test
  observed_on: 2026-10-01
  version: 1.2.14
- claim: 'Single-dash short value flags accept attached, equals, and space forms — `agy -phello --print-timeout bogus` and `agy -p=hello --print-timeout bogus` parse the prompt and reach the timeout error, and on the mcp add flagset `-thttp`, `-t=http`, `-eFOO=bar`, `-HX: Y`, `--env=FOO=bar`, and `--header=X: Y` all parse far enough to surface the next flag''s missing value.'
  id: parse-short-attached
  limitations: Short boolean flags reject attached values without `=` (`-cx` is not defined); attachment was not probed for -i beyond the shared parser.
  location: disposable parse-failure probes against the installed binary
  method: disposable_test
  observed_on: 2026-10-01
  version: 1.2.14
- claim: The root parser stops at the first stray positional — `agy hello --print-timeout bogus` errors `unexpected argument "hello"` exit 2 without parsing the flag behind it, `agy -- hello` errors the same way on `hello`, and a single stray token quoting two words (`agy "mcp list"`) errors on that whole token; prompts are never read from positionals, only from -p/--print, -i/--prompt-interactive, or stdin.
  id: parse-positional-stop
  limitations: Does not settle whether a piped stdin prompt without --print runs headless, because exercising it would start a billable session.
  location: disposable parse-failure probes against the installed binary
  method: disposable_test
  observed_on: 2026-10-01
  version: 1.2.14
- claim: Help and version are pre-scanned across the whole argv — `agy hello --help` prints root usage exit 0, `agy -h` prints usage exit 0, and `agy -- --version`, `agy -version`, `agy models --version`, `agy agent --version`, and `agy mcp list --version` all print the version string exit 0, so --version is accepted at every command path and even past the `--` terminator.
  id: parse-prescan-help-version
  limitations: The version scan was probed at root, models, agent, and mcp list paths plus past `--`; it was not probed on every individual subcommand.
  location: disposable parse-failure probes against the installed binary
  method: disposable_test
  observed_on: 2026-10-01
  version: 1.2.14
- claim: 'Startup validation order and exit codes — an unknown `--model` slug fails `error: invalid model selection ...` exit 1, an invalid `--effort` fails listing `valid: low, medium, high, max` exit 1, `--json-schema` without json/stream-json output fails exit 1, `--input-format bogus` fails `unknown --input-format` exit 2 and `--input-format stream-json --output-format json` fails the pairing rule exit 2, while `--output-format bogus`, `--mode bogus`, and `--agent bogus` pass startup far enough that the model error preempts them.'
  id: startup-validation
  limitations: Whether --output-format/--mode/--agent are validated after model resolution could only be settled by a run that starts a billable session.
  location: disposable startup-failure probes against the installed binary (no session started)
  method: disposable_test
  observed_on: 2026-10-01
  version: 1.2.14
- claim: 'Subcommand flagsets reject bad input with their own usage and exit 1 — `agy mcp add -t/--type/--env/--header/-e` and `agy remote-control start --name`, `agy install --dir`, `agy mic-serve --addr` fail with `flag needs an argument` exit 1, `remote-control start --session=bogus` and `install --skip-aliases=bogus` fail with `invalid boolean value` exit 1, `agy mcp enable` fails `usage: mcp enable <name>` exit 1, and `agy mcp bogus` fails `unknown mcp subcommand "bogus"` exit 1.'
  id: mcp-rc-install-mic-flag-tests
  limitations: No syntactically valid mcp add/remove/enable/disable invocation was run because it would rewrite the user's mcp_config.json.
  location: disposable parse-failure probes against the installed binary
  method: disposable_test
  observed_on: 2026-10-01
  version: 1.2.14
- claim: 'The read-only discovery commands run headless — `agy models` writes a slug<TAB>name table to stdout with `Fetching available models...` on stderr, `agy agents` writes one agent name per line to stdout with nothing on stderr, `agy mcp list` writes a NAME/TYPE/STATUS/COMMAND-URL table to stdout, `agy remote-control status` prints `Daemon status: not running` exit 0, and `agy changelog` prints cached release notes.'
  id: introspection-runs
  limitations: models requires an authenticated account; the host is signed in, so unauthenticated behavior was not observable locally.
  location: local commands `agy models`, `agy agents`, `agy mcp list`, `agy remote-control status`, `agy changelog` with stdout/stderr split into files
  method: local_inspection
  observed_on: 2026-10-01
  version: 1.2.14
- claim: 'The machine-readable `--output-format` that changelog 1.1.12 added to `models` and `agents` is gone — `agy models --output-format json` and `agy agents --output-format json` both fail with `flags provided but not defined: -output-format` exit 1 on the installed binary.'
  id: models-json-rejected
  limitations: No changelog entry names the removal; the flag may return.
  location: disposable parse-failure probes against the installed binary
  method: disposable_test
  observed_on: 2026-10-01
  version: 1.2.14
- claim: 'The `update` subcommand''s flag handling is irregular — `agy update --help` prints `Usage of update:` and exits 2 while `agy update -h` fails with `flags provided but not defined: -h` exit 1, so --help is accepted there but -h is not.'
  id: update-flag-quirks
  limitations: '`agy update` itself was not run because it replaces the installed binary.'
  location: disposable parse-failure probes against the installed binary
  method: disposable_test
  observed_on: 2026-10-01
  version: 1.2.14
- claim: 'A TUI launch without a terminal fails with exit 0 — `agy --effort bogus` (no -p) prints `CLI error: bubbletea: error opening TTY: bubbletea: could not open TTY: open /dev/tty: device not configured` and exits 0, so exit code 0 does not imply success.'
  id: tui-no-tty-exit-zero
  limitations: Only the no-TTY failure path was observed; the interactive TUI itself was not driven.
  location: disposable probe `agy --effort bogus < /dev/null` on the installed binary
  method: disposable_test
  observed_on: 2026-10-01
  version: 1.2.14
- claim: Plugin help is headless on 1.2.14 — `agy plugin --help` and `agy plugins --help` print the command usage exit 0 (the 1.1.0-era Bubble Tea failure is gone), while `agy plugin list --help` ignores the flag entirely and prints `No imported plugins.` because plugin subcommand tails are not flag-parsed.
  id: plugin-behavior
  limitations: plugin import/install/uninstall were not exercised.
  location: local commands `agy plugin --help`, `agy plugins --help`, `agy plugin list --help`
  method: local_inspection
  observed_on: 2026-10-01
  version: 1.2.14
- claim: The headless-mode documentation establishes the print-mode flag reference (-p/--print/--prompt aliases, --output-format text|json|stream-json, --input-format stream-json stdin protocol, --json-schema string-or-path, --model/--effort/--agent selection, --continue/--conversation resumption, permission soft-denial with exit 0, the JSON envelope fields, the stream event shapes, the status vocabulary, and the exit-code contract including exit 1 for unknown models).
  id: docs-headless
  limitations: The page is undated and lags the binary in places — it still says --print-timeout defaults to 5m (the binary says 0s since 1.2.6), omits `max` from --effort, and still describes --json-schema primitive type names that 1.2.14 rejects.
  location: https://antigravity.google/docs/cli/headless/
  method: official_docs
  observed_on: 2026-10-01
  version: unknown
- claim: The installation documentation gives the per-OS install commands (curl|bash on macOS/Linux to ~/.local/bin, PowerShell `irm ...install.ps1 | iex` and CMD `curl ... install.cmd` to %LOCALAPPDATA%\agy\bin), the --skip-aliases/--skip-path installer flags, the keyring-based sign-in flows, and the GEMINI_API_KEY + modelProvider authentication mode.
  id: docs-install
  limitations: The troubleshooting page contradicts it with a `C:\Program Files\Google\antigravity-cli` PATH example; the install page matches the installer script and local install.
  location: https://antigravity.google/docs/cli/install/
  method: official_docs
  observed_on: 2026-10-01
  version: unknown
- claim: The troubleshooting documentation names AGY_CLI_DISABLE_AUTO_UPDATE (set to true to stop the background self-updater) and describes the updater's 15-minute TTL debounce marker and advisory update.lock under ~/.gemini/antigravity-cli/updater/, plus the per-OS keyring requirements.
  id: docs-troubleshooting
  limitations: Does not enumerate other AGY_* variables or their accepted values.
  location: https://antigravity.google/docs/cli/troubleshooting/
  method: official_docs
  observed_on: 2026-10-01
  version: unknown
- claim: The CLI reference documentation lists the settings.json keys (colorScheme, altScreenMode, toolPermission, artifactReviewPolicy, notifications, showTips, showFeedbackSurvey, editor, editorMode, vimInsertFirst, allowNonWorkspaceAccess, enableTerminalSandbox, useG1Credits, enableTelemetry, verbosity, runningLightSpeed) and their defaults.
  id: docs-reference
  limitations: The page documents TUI slash commands and settings keys, not command-line switches; switch documentation lives on the headless page.
  location: https://antigravity.google/docs/cli/reference/
  method: official_docs
  observed_on: 2026-10-01
  version: unknown
- claim: The cached changelog attributes the surface under study — mcp subcommands with --type/--env/--header (1.1.16), machine-readable models/agents --output-format (1.1.12), mic-serve (1.1.21), remote-control start/status/stop with --name/--session (1.2.0), exit 3 plus the AGY_ERROR stderr line for mid-run failures and the unlimited --print-timeout default (1.2.6), --print-timeout expiry returning partial output with success (1.1.28), headless exit codes reflecting only cascade failures (1.1.20), and 1.2.14's stricter --json-schema.
  id: changelog-cache
  limitations: The cache can describe a newer version than the running binary (it listed 1.2.14 notes while the binary was still 1.2.13); the models/agents --output-format removal is not recorded in it.
  location: local command `agy changelog`
  method: local_inspection
  observed_on: 2026-10-01
  version: 1.2.14
- claim: The published Unix installer accepts `-d, --dir <path>` and `-h, --help`, installs to ~/.local/bin by default, detects darwin/linux amd64+arm64 plus musl and Android/Termux, stages the tarball under ~/.cache/antigravity/staging, verifies SHA-512 from the updater manifest, and exits early if agy already exists.
  id: install-script
  limitations: The Windows install.ps1/install.cmd were not re-fetched this run; their behavior is carried from the 1.1.0-era research and the docs page.
  location: https://antigravity.google/cli/install.sh
  method: source_code
  observed_on: 2026-10-01
  version: unknown
- claim: The GitHub latest-release API returns tag 1.2.14 published 2026-09-30 with darwin/linux/windows amd64+arm64 assets (musl variants for Linux), matching the newest released upstream version.
  id: github-release-latest
  limitations: A point-in-time read; a newer release may appear at any time.
  location: https://api.github.com/repos/google-antigravity/antigravity-cli/releases/latest
  method: official_docs
  observed_on: 2026-10-01
  version: 1.2.14
- claim: The auto-updater manifest for darwin_arm64 served version 1.2.14 with its storage.googleapis.com tarball URL and SHA-512, confirming the update channel the binary followed during the session.
  id: updater-manifest
  limitations: Only the darwin_arm64 manifest was read.
  location: https://antigravity-cli-auto-updater-974169037036.us-central1.run.app/manifests/darwin_arm64.json
  method: official_docs
  observed_on: 2026-10-01
  version: 1.2.14
- claim: 'Strings extracted from the installed binary carry the AGY_* environment-variable inventory (AGY_CLI_DISABLE_AUTO_UPDATE, AGY_CLI_CMD_OUTPUT_PERCENTAGE, AGY_CLI_DISABLE_LATEX, AGY_CLI_DISABLE_MERMAID_ASCII, AGY_CLI_DISABLE_INLINE_MERMAID, AGY_CLI_HIDE_ACCOUNT_INFO, AGY_CLI_HIDE_LOGO, AGY_CLI_FORCE_OSC8, AGY_CLI_EXPERIMENTAL_RENDERING, AGY_CLI_LOGO_STYLE, AGY_CLI_MAC_OPTION_AS_ALT, AGY_CLI_FAST_PROMPT_EDITOR, AGY_CLI_DISABLE_INPUT_MODE_REASSERT, AGY_CLI_DISABLE_ESCAPE_SEQUENCE_OPTIMIZATIONS, AGY_CLI_INTERACTIVE_HEADLESS, AGY_CLI_NONINTERACTIVE_HEADLESS, AGY_CLI_CDE_AUTH_ACTION, AGY_ADC_AUTH, AGY_BROWSER_WS_URL, AGY_BROWSER_ACTIVE_PORT_FILE, AGY_CLI_REMOTE_CONTROL_WEB_VERSION, AGY_REMOTE_CONTROL_VERSION_OVERRIDE, AGY_ENABLE_HUB, AGY_BUSINESS_PAYGO_TIER) and the error format `invalid --effort %q (valid: %s)`.'
  id: binary-strings-env
  limitations: String presence shows the binary reads these names, not their accepted values or exact effects; model-endpoint variables (GEMINI_API_KEY, GOOGLE_GEMINI_BASE_URL, AGY_LLM_GATEWAY_*) were also present and belong to the model-config topic.
  location: local command `strings -a /Users/ken/.local/bin/agy`
  method: local_inspection
  observed_on: 2026-10-01
  version: 1.2.14
- claim: Local configuration state matches the documented layout — ~/.gemini/antigravity-cli/settings.json holds model and trustedWorkspaces, ~/.gemini/config/ holds config.json (userSettings), mcp_config.json, projects/, hooks/, sidecars/, and skills/plugins manifests, and the CLI-private root also holds cli.log, keybindings.json, conversations/, cache/, updater/, builtin/skills/ (including the agy-customizations and antigravity_guide references).
  id: local-config-state
  limitations: A snapshot of one signed-in macOS profile; Windows and Linux spellings come from the docs and installer script, not local inspection.
  location: local files under ~/.gemini/antigravity-cli/ and ~/.gemini/config/
  method: local_inspection
  observed_on: 2026-10-01
  version: 1.2.14
- claim: 'A hidden integer flag -v exists at the root — `agy -v` fails `flag needs an argument: -v`, `agy -v=bogus` fails `invalid value "bogus" for flag -v: strconv.Atoi`, and `agy -v 1 --print-timeout bogus` and `agy -v=1 --print-timeout bogus` parse through, so -v takes one integer by space or equals.'
  id: hidden-v-flag
  limitations: The flag is omitted from --help and no documentation names its purpose or accepted values.
  location: disposable parse-failure probes against the installed binary
  method: disposable_test
  observed_on: 2026-10-01
  version: 1.2.14
binaries:
- alt_binaries: []
  binary: agy
  notes: Installer writes `agy` to `$HOME/.local/bin`; local inspection found `/Users/ken/.local/bin/agy` (187 MB Go binary). The `/Applications/Antigravity.app` and `/Applications/Antigravity IDE.app` bundles are the desktop apps, not the CLI.
  os: macos
- alt_binaries: []
  binary: agy
  notes: Installer writes `agy` to `$HOME/.local/bin`; glibc, musl, and Android/Termux builds ship.
  os: linux
- alt_binaries:
  - agy
  binary: agy.exe
  notes: Installer writes `agy.exe` to `%LOCALAPPDATA%\agy\bin`; users type `agy` from PowerShell or CMD after PATH setup. The troubleshooting page's `C:\Program Files\Google\antigravity-cli` PATH example contradicts the install page and installer; trust `%LOCALAPPDATA%\agy\bin`.
  os: windows
install_methods:
- command: curl -fsSL https://antigravity.google/cli/install.sh | bash
  method: standalone_binary
  notes: Installs to ~/.local/bin by default; `-d, --dir <path>` overrides. Stages under ~/.cache/antigravity/staging, verifies SHA-512, clears macOS quarantine, and calls `agy install`.
  os: macos
- command: curl -fsSL https://antigravity.google/cli/install.sh | bash
  method: standalone_binary
  notes: Same script; detects amd64/arm64, musl, and Android/Termux (Bionic build).
  os: linux
- command: irm https://antigravity.google/cli/install.ps1 | iex
  method: standalone_binary
  notes: PowerShell path; installs to %LOCALAPPDATA%\agy\bin; supports -d/--dir.
  os: windows
- command: curl -fsSL https://antigravity.google/cli/install.cmd -o install.cmd && install.cmd && del install.cmd
  method: standalone_binary
  notes: CMD path; the 1.1.0-era script rejected command lines containing shell metacharacters such as &, |, ;, <, >, and ^.
  os: windows
subcommands:
- description: Lists the names of available agents, one per line on stdout.
  name: agent
  non_interactive: true
  notes: Verified headless on 1.2.14 (`agy agent < /dev/null`, exit 0). `agents` is an alias; the usage header spells the canonical command `agent`.
- description: Alias for `agent`; lists available agents.
  name: agents
  non_interactive: true
  notes: Same output and usage header as `agent`.
- description: Shows cached changelog and release notes.
  name: changelog
  non_interactive: true
  notes: Fetches/prints human-readable release notes; the cache can describe a newer version than the running binary (observed listing 1.2.14 while the binary was 1.2.13).
- description: Shows top-level help, or the named subcommand's help (`agy help mcp`).
  name: help
  non_interactive: true
  notes: Accepts a subcommand name as an operand; rejects flags (`agy help --help` and `agy help bogus` fail `unknown subcommand` exit 1).
- description: Configures shell PATH and shell profile settings after the binary is installed.
  name: install
  non_interactive: true
  notes: Called automatically by the installer scripts; mutates shell profile files unless --skip-path and --skip-aliases are given.
- description: Dispatcher for MCP server configuration management.
  name: mcp
  non_interactive: true
  notes: Bare `agy mcp` prints the subcommand list. Unknown sub-subcommands fail `unknown mcp subcommand` exit 1.
- description: Adds or updates an MCP server entry in user-level mcp_config.json.
  name: mcp add
  non_interactive: true
  notes: Usage `agy mcp add [flags] <name> <commandOrUrl> [args...]`; flags must precede <name>; `--` guards commands or args that begin with `-`; writes the config file (not exercised in this research).
- description: Removes an MCP server configuration by name.
  name: mcp remove
  non_interactive: true
  notes: Usage `agy mcp remove <name> [flags]`; missing operand fails with a usage error.
- description: Lists configured MCP servers as a NAME/TYPE/STATUS/COMMAND-URL table on stdout.
  name: mcp list
  non_interactive: true
  notes: Verified headless; table is fixed-width text, not JSON.
- description: Enables an MCP server by name.
  name: mcp enable
  non_interactive: true
  notes: '`agy mcp enable` without a name fails `usage: mcp enable <name>` exit 1.'
- description: Disables an MCP server by name.
  name: mcp disable
  non_interactive: true
  notes: Same shape as `mcp enable`.
- description: Serves this machine's microphone to a CLI on another host for voice dictation.
  name: mic-serve
  non_interactive: false
  notes: Long-running network server (default 127.0.0.1:4713) that never runs to completion, and microphone access can raise an OS-level permission prompt; added in 1.1.21 with /voice.
- description: Lists model slugs and display names for the signed-in account.
  name: models
  non_interactive: true
  notes: Requires authentication; table on stdout, `Fetching available models...` on stderr. The --output-format json mode added in 1.1.12 is rejected again on 1.2.14.
- description: Manages plugins (list, import, install, uninstall, enable, disable, validate, link).
  name: plugin
  non_interactive: false
  notes: '`agy plugin --help` is headless on 1.2.14 (the 1.1.0 Bubble Tea failure is gone), but plugin subcommand tails are not flag-parsed (`plugin list --help` prints the list) and import/install flows were not verified headless.'
- description: Alias for `plugin`.
  name: plugins
  non_interactive: false
  notes: Same behavior as `plugin`.
- description: Dispatcher for the remote-control background daemon.
  name: remote-control
  non_interactive: true
  notes: Bare `agy remote-control` prints the subcommand list exit 0.
- description: Registers and starts the remote-control daemon with the OS service manager.
  name: remote-control start
  non_interactive: false
  notes: Mutating and not exercised; added 1.2.0 with --name and --session. On Linux without a systemd user manager it falls back to a background process (1.2.14 changelog).
- description: Prints whether the remote-control daemon is running.
  name: remote-control status
  non_interactive: true
  notes: 'Verified headless (`Daemon status: not running`, exit 0).'
- description: Stops and unregisters the remote-control daemon.
  name: remote-control stop
  non_interactive: false
  notes: Mutating and not exercised.
- description: Updates the CLI binary from the release channel.
  name: update
  non_interactive: true
  notes: The same headless updater runs in the background during normal use (it replaced 1.2.13 with 1.2.14 during this research). `--help` prints `Usage of update:` exit 2; `-h` is rejected exit 1.
cli_switches:
- aliases:
  - -h
  attachment: []
  description: Show help.
  evidence_ids:
  - help-root-12214
  - subhelp-battery
  - parse-prescan-help-version
  - update-flag-quirks
  - plugin-behavior
  flag: --help
  invocation_scope:
  - applies_to: command
    command: []
  - applies_to: command
    command:
    - agent
  - applies_to: command
    command:
    - agents
  - applies_to: command
    command:
    - changelog
  - applies_to: command
    command:
    - install
  - applies_to: command
    command:
    - mcp
  - applies_to: command
    command:
    - mcp
    - add
  - applies_to: command
    command:
    - mcp
    - remove
  - applies_to: command
    command:
    - mcp
    - list
  - applies_to: command
    command:
    - mcp
    - enable
  - applies_to: command
    command:
    - mcp
    - disable
  - applies_to: command
    command:
    - mic-serve
  - applies_to: command
    command:
    - models
  - applies_to: command
    command:
    - plugin
  - applies_to: command
    command:
    - plugins
  - applies_to: command
    command:
    - remote-control
  - applies_to: command
    command:
    - remote-control
    - start
  - applies_to: command
    command:
    - remote-control
    - status
  - applies_to: command
    command:
    - remote-control
    - stop
  notes: 'Hidden from the root usage listing but honored everywhere listed, exit 0, output on stdout. At the root it is pre-scanned across argv: it wins even after a stray positional (`agy hello --help`) and past `--`. Irregular paths: `update` accepts `--help` but exits 2 and rejects `-h` exit 1; the `help` subcommand rejects `--help`; `plugin <sub>` tails ignore flags. Subcommand help for two-word paths prints that path''s own help (`agy mcp add --help`).'
  scope:
  - help
  value_type: none
- aliases:
  - -version
  attachment: []
  description: Print the installed CLI version and exit.
  evidence_ids:
  - parse-bool-forms
  - parse-prescan-help-version
  - version-check
  flag: --version
  invocation_scope:
  - applies_to: global
  notes: 'Hidden from the root usage listing; a boolean flag (`--version=bogus` fails `invalid boolean value`). Pre-scanned across the whole argv like --help: `agy models --version`, `agy agent --version`, `agy mcp list --version`, and even `agy -- --version` print the version (1.2.14) exit 0. `-v` is NOT an alias — it is a separate hidden integer flag. Do not rely on `--` or subcommand dispatch to shield a literal `--version` token.'
  scope:
  - version
  value_type: none
- attachment:
  - space
  - equals
  description: Add a directory to the workspace (repeatable).
  evidence_ids:
  - help-root-12214
  - parse-needs-argument
  - parse-long-forms
  flag: --add-dir
  invocation_scope:
  - applies_to: command
    command: []
  value_optional: false
  value_type: string
- attachment:
  - space
  - equals
  description: Agent for the current CLI session.
  evidence_ids:
  - help-root-12214
  - parse-needs-argument
  - startup-validation
  flag: --agent
  invocation_scope:
  - applies_to: command
    command: []
  notes: Names an agent from `agy agents`. Not validated before model resolution at startup (an unknown agent passed startup far enough for the model error to preempt it). The stream-json init payload reports the agent only when this flag sets it.
  scope:
  - agent_selection
  value_optional: false
  value_type: string
- aliases:
  - -c
  attachment: []
  description: Continue the most recent conversation.
  evidence_ids:
  - help-root-12214
  - parse-bool-forms
  flag: --continue
  invocation_scope:
  - applies_to: command
    command: []
  notes: Resumes the most recent conversation for the launch; with -p it is the headless resume form (the other is --conversation <id>). `-c=x` is accepted boolean syntax; `-cx` is not.
  scope:
  - resume
  value_type: none
- attachment:
  - space
  - equals
  description: Resume a previous conversation by ID.
  evidence_ids:
  - help-root-12214
  - parse-needs-argument
  flag: --conversation
  invocation_scope:
  - applies_to: command
    command: []
  notes: Takes the conversation_id reported by a previous run's JSON envelope or stream result event.
  scope:
  - resume
  value_optional: false
  value_type: string
- attachment: []
  description: Auto-approve all tool permission requests without prompting.
  evidence_ids:
  - help-root-12214
  - parse-bool-forms
  flag: --dangerously-skip-permissions
  invocation_scope:
  - applies_to: command
    command: []
  notes: Permission semantics and settings interplay belong to the agent-permissions topic; in headless runs it switches the init payload's permission_mode from request-review to always-proceed.
  scope:
  - permissions
  value_type: none
- attachment: []
  description: Disable slash command and skill expansion in print mode.
  evidence_ids:
  - help-root-12214
  - parse-bool-forms
  flag: --disable-slash-commands
  invocation_scope:
  - applies_to: command
    command: []
  notes: Affects -p/--print runs only; interactive-only slash commands otherwise fail with an explicit refusal in print mode (1.1.x changelog).
  scope:
  - non_interactive
  value_type: none
- attachment:
  - space
  - equals
  description: Reasoning effort for the current CLI session (low|medium|high|max).
  evidence_ids:
  - help-root-12214
  - parse-needs-argument
  - startup-validation
  - binary-strings-env
  flag: --effort
  invocation_scope:
  - applies_to: command
    command: []
  notes: 'Validated at startup in headless runs: an invalid value fails exit 1 listing `valid: low, medium, high, max`. Help includes `max`; the headless docs page still says low/medium/high only. Per-model acceptance is recorded at run time (`Effort isn''t adjustable for %s.`).'
  scope:
  - reasoning
  value_optional: false
  value_type: string
- aliases:
  - -i
  attachment:
  - space
  - equals
  - short_attached
  description: Run an initial prompt interactively and continue the session.
  evidence_ids:
  - help-root-12214
  - parse-needs-argument
  - parse-short-attached
  flag: --prompt-interactive
  invocation_scope:
  - applies_to: command
    command: []
  notes: 'Interactive launch form: the prompt is the first turn of a TUI session, so the continued session needs a terminal. `-i` is the provider-documented short alias; short-attached `-itext` follows the shared parser but was probed directly only for `-p`.'
  scope:
  - interactive
  value_optional: false
  value_type: string
- attachment:
  - space
  - equals
  description: Input format for print mode (text, stream-json); stream-json reads one NDJSON message per line from stdin and runs a turn for each.
  evidence_ids:
  - help-root-12214
  - parse-needs-argument
  - startup-validation
  - docs-headless
  flag: --input-format
  invocation_scope:
  - applies_to: command
    command: []
  notes: 'Validated before model resolution: `--input-format bogus` fails exit 2 and `stream-json` without `--output-format stream-json` fails the pairing rule exit 2. In stream-json sessions a -p prompt is dropped and prompts arrive as `user` events on stdin; malformed input ends the session with exit 1 (exit 2 for CLI-handled slash commands or control events).'
  scope:
  - non_interactive
  - streaming
  value_optional: false
  value_type: string
- attachment:
  - space
  - equals
  description: Optional JSON schema string or path to a schema file to enforce structured output.
  evidence_ids:
  - help-root-12214
  - parse-needs-argument
  - startup-validation
  - changelog-cache
  - docs-headless
  flag: --json-schema
  invocation_scope:
  - applies_to: command
    command: []
  notes: 'Requires --output-format json or stream-json (otherwise exit 1). Since 1.2.14, plain text, bare type names such as `string`, missing files, and any schema whose root is not `"type": "object"` fail at startup with exit 1; the headless docs page still documents the old primitive-type-name behavior.'
  scope:
  - non_interactive
  - structured_output
  value_optional: false
  value_type: string
- attachment:
  - space
  - equals
  description: Override CLI log file path.
  evidence_ids:
  - help-root-12214
  - parse-needs-argument
  flag: --log-file
  invocation_scope:
  - applies_to: command
    command: []
  notes: Logging surfaces and rotation belong to the agent-logging topic; recorded here only as a value-taking root switch.
  scope:
  - logging
  value_optional: false
  value_type: string
- attachment:
  - space
  - equals
  description: Set the agent execution mode for this session (accept-edits, plan).
  evidence_ids:
  - help-root-12214
  - parse-needs-argument
  - startup-validation
  flag: --mode
  invocation_scope:
  - applies_to: command
    command: []
  notes: Enum spellings as help prints them. Not validated before model resolution at startup (an unknown mode passed startup far enough for the model error to preempt it).
  scope:
  - execution
  value_optional: false
  value_type: string
- attachment:
  - space
  - equals
  description: Model for the current CLI session.
  evidence_ids:
  - help-root-12214
  - parse-needs-argument
  - startup-validation
  - docs-headless
  flag: --model
  invocation_scope:
  - applies_to: command
    command: []
  notes: Takes a slug from `agy models`. Headless runs validate at startup and exit 1 on an unknown slug instead of falling back; the interactive UI may fall back silently. Catalog and endpoint semantics belong to model-config.
  scope:
  - model_selection
  value_optional: false
  value_type: string
- attachment: []
  description: Create a new project for this session.
  evidence_ids:
  - help-root-12214
  - parse-bool-forms
  flag: --new-project
  invocation_scope:
  - applies_to: command
    command: []
  value_type: none
- attachment:
  - space
  - equals
  description: Output format for print mode (text, json, stream-json).
  evidence_ids:
  - help-root-12214
  - parse-needs-argument
  - startup-validation
  - docs-headless
  flag: --output-format
  invocation_scope:
  - applies_to: command
    command: []
  notes: 'Now documented in --help (it was hidden in 1.1.0). `json` emits one envelope with conversation_id/status/response/error/duration_seconds/num_turns/usage (+structured_output/json_schema with --json-schema); `stream-json` emits NDJSON init/step_update/result events. NOT validated before model resolution: a bogus value passed startup while the model error preempted it, so a bad format can survive until after a billable turn. `--json` and `--format json` are not defined flags.'
  scope:
  - non_interactive
  value_optional: false
  value_type: string
- aliases:
  - -p
  - --prompt
  attachment:
  - space
  - equals
  - short_attached
  description: Run a single prompt non-interactively and print the response.
  evidence_ids:
  - help-root-12214
  - parse-needs-argument
  - parse-short-attached
  - docs-headless
  flag: --print
  invocation_scope:
  - applies_to: command
    command: []
  notes: The headless entry point; response on stdout, diagnostics on stderr. `-p`, `--print`, and `--prompt` are the same switch; `-phello` and `-p=hello` parse (proved by parse-through), `-p hello` is the usual form. A prompt starting with `-` needs the equals or attached form. In --input-format stream-json sessions a -p prompt is dropped.
  scope:
  - non_interactive
  value_optional: false
  value_type: string
- attachment:
  - space
  - equals
  default: 0s
  description: Optional time limit for print mode; 0 waits until the turn completes.
  evidence_ids:
  - help-root-12214
  - parse-needs-argument
  - parse-long-forms
  - changelog-cache
  - docs-headless
  flag: --print-timeout
  invocation_scope:
  - applies_to: command
    command: []
  notes: 'Go duration syntax (proved: `bogus` fails `time: invalid duration`). Default changed from 5m to unlimited (0s) in 1.2.6; the headless docs page still says 5m. Since 1.1.28 an expiry mid-turn returns the partial output and exits 0 with a stderr warning instead of failing; headless runs also wait for background tasks under this deadline (30-minute cap).'
  scope:
  - non_interactive
  value_optional: false
  value_type: string
- attachment:
  - space
  - equals
  description: Project ID or project name for the current CLI session.
  evidence_ids:
  - help-root-12214
  - parse-needs-argument
  flag: --project
  invocation_scope:
  - applies_to: command
    command: []
  value_optional: false
  value_type: string
- attachment: []
  description: Create a remote connection for the CLI session on start up.
  evidence_ids:
  - help-root-12214
  - parse-bool-forms
  - changelog-cache
  flag: --remote-control
  invocation_scope:
  - applies_to: command
    command: []
  notes: Session-scoped remote connection (added 1.2.6 changelog era); distinct from the `remote-control` subcommands, which manage the always-on daemon.
  scope:
  - remote_control
  value_type: none
- attachment: []
  description: Run in a sandbox with terminal restrictions enabled.
  evidence_ids:
  - help-root-12214
  - parse-bool-forms
  flag: --sandbox
  invocation_scope:
  - applies_to: command
    command: []
  notes: Sandbox policy and settings (enableTerminalSandbox, proceed-in-sandbox toolPermission) belong to the agent-permissions topic.
  scope:
  - permissions
  - execution
  value_type: none
- attachment:
  - space
  - equals
  description: Hidden integer flag at the root entrypoint; purpose undocumented.
  evidence_ids:
  - hidden-v-flag
  flag: -v
  invocation_scope:
  - applies_to: command
    command: []
  notes: Omitted from --help. `strconv.Atoi` parses the value, so it is a single integer; `-v 1` and `-v=1` both parse. Not an alias of --version. Likely a verbosity or debug level, but no help text, docs page, changelog entry, or binary-strings context names its purpose or accepted values; running a command with and without `-v N` and diffing output volume would settle it.
  scope:
  - diagnostics
  value_optional: false
  value_type: number
- attachment:
  - space
  - equals
  description: Custom directory target to configure PATH for.
  evidence_ids:
  - subhelp-battery
  - mcp-rc-install-mic-flag-tests
  - install-script
  flag: --dir
  invocation_scope:
  - applies_to: command
    command:
    - install
  notes: The installer scripts accept their own `-d, --dir <path>` before calling `agy install`.
  value_optional: false
  value_type: string
- attachment: []
  description: Bypasses shell profile alias purging.
  evidence_ids:
  - subhelp-battery
  - mcp-rc-install-mic-flag-tests
  - docs-install
  flag: --skip-aliases
  invocation_scope:
  - applies_to: command
    command:
    - install
  notes: Also accepted by the installer scripts themselves (docs installation-flags section).
  value_type: none
- attachment: []
  description: Bypasses shell profile PATH appending.
  evidence_ids:
  - subhelp-battery
  - docs-install
  flag: --skip-path
  invocation_scope:
  - applies_to: command
    command:
    - install
  notes: Also accepted by the installer scripts themselves. Boolean like its sibling --skip-aliases (same flag table; `--skip-aliases=bogus` was the directly probed one).
  value_type: none
- aliases:
  - -e
  attachment:
  - space
  - equals
  - short_attached
  description: Environment variable KEY=value (repeatable).
  evidence_ids:
  - subhelp-battery
  - mcp-rc-install-mic-flag-tests
  - parse-short-attached
  flag: --env
  invocation_scope:
  - applies_to: command
    command:
    - mcp
    - add
  notes: 'Repeatable: each occurrence carries one KEY=value. `-eFOO=bar` and `--env=FOO=bar` proved by parse-through on the mcp add flagset.'
  scope:
  - mcp
  value_optional: false
  value_type: string
- aliases:
  - -H
  attachment:
  - space
  - equals
  - short_attached
  description: 'HTTP header Key: Value (repeatable).'
  evidence_ids:
  - subhelp-battery
  - mcp-rc-install-mic-flag-tests
  - parse-short-attached
  flag: --header
  invocation_scope:
  - applies_to: command
    command:
    - mcp
    - add
  notes: 'Repeatable: each occurrence carries one `Key: Value` pair for http servers. `-HX: Y` and `--header=X: Y` proved by parse-through.'
  scope:
  - mcp
  value_optional: false
  value_type: string
- aliases:
  - -t
  attachment:
  - space
  - equals
  - short_attached
  default: stdio
  description: Server type 'stdio' or 'http'.
  evidence_ids:
  - subhelp-battery
  - mcp-rc-install-mic-flag-tests
  - parse-short-attached
  flag: --type
  invocation_scope:
  - applies_to: command
    command:
    - mcp
    - add
  notes: '`-thttp`, `-t=http`, and `-t http` all parse; http/https URLs in <commandOrUrl> are auto-detected, so the flag is usually unnecessary.'
  scope:
  - mcp
  value_optional: false
  value_type: string
- attachment:
  - space
  - equals
  default: 127.0.0.1:4713
  description: Address to listen on; keep it on loopback.
  evidence_ids:
  - subhelp-battery
  - mcp-rc-install-mic-flag-tests
  flag: --addr
  invocation_scope:
  - applies_to: command
    command:
    - mic-serve
  value_optional: false
  value_type: string
- attachment:
  - space
  - equals
  description: Set this machine's instance name shown in Remote Control.
  evidence_ids:
  - subhelp-battery
  - mcp-rc-install-mic-flag-tests
  - changelog-cache
  flag: --name
  invocation_scope:
  - applies_to: command
    command:
    - remote-control
    - start
  notes: Added with the remote-control daemon subcommands in 1.2.0.
  value_optional: false
  value_type: string
- attachment: []
  description: Register the daemon login-scoped (stops at logout) instead of starting at boot.
  evidence_ids:
  - subhelp-battery
  - mcp-rc-install-mic-flag-tests
  flag: --session
  invocation_scope:
  - applies_to: command
    command:
    - remote-control
    - start
  notes: Boolean (`--session=bogus` fails `invalid boolean value`).
  value_type: none
- attachment: []
  description: Show the update subcommand's usage.
  evidence_ids:
  - update-flag-quirks
  - subhelp-battery
  flag: --help
  invocation_scope:
  - applies_to: command
    command:
    - update
  notes: 'Prints only `Usage of update:` and exits 2; `-h` is rejected with `flags provided but not defined: -h` exit 1, so no short alias is recorded. Kept as a separate record from the main --help entry because both spelling set and exit code differ.'
  value_type: none
config_paths:
- format: json
  notes: CLI-private settings; the CLI and the interactive /config editor write it. Local file held model and trustedWorkspaces; documented keys include toolPermission, artifactReviewPolicy, permissions.allow, modelProvider, colorScheme, verbosity, enableTerminalSandbox, editor, notifications.
  os: macos
  path: ~/.gemini/antigravity-cli/settings.json
  scope: user
- format: json
  notes: Same home-relative path and keys.
  os: linux
  path: ~/.gemini/antigravity-cli/settings.json
  scope: user
- format: json
  notes: Windows spelling of the same settings file.
  os: windows
  path: '%USERPROFILE%\.gemini\antigravity-cli\settings.json'
  scope: user
- format: json
  notes: Command-to-key mapping written by the /keybindings editor.
  os: macos
  path: ~/.gemini/antigravity-cli/keybindings.json
  scope: user
- format: json
  notes: Same file, Linux spelling.
  os: linux
  path: ~/.gemini/antigravity-cli/keybindings.json
  scope: user
- format: json
  notes: Windows spelling.
  os: windows
  path: '%USERPROFILE%\.gemini\antigravity-cli\keybindings.json'
  scope: user
- format: json
  notes: Shared Antigravity configuration root; local file held a userSettings object.
  os: macos
  path: ~/.gemini/config/config.json
  scope: user
- format: json
  notes: Shared Antigravity configuration root.
  os: linux
  path: ~/.gemini/config/config.json
  scope: user
- format: json
  notes: Windows spelling.
  os: windows
  path: '%USERPROFILE%\.gemini\config\config.json'
  scope: user
- format: json
  notes: Global MCP server configuration that `agy mcp add/remove/enable/disable` edit; depth belongs to the mcp topic.
  os: macos
  path: ~/.gemini/config/mcp_config.json
  scope: user
- format: json
  notes: Global MCP server configuration.
  os: linux
  path: ~/.gemini/config/mcp_config.json
  scope: user
- format: json
  notes: Windows spelling.
  os: windows
  path: '%USERPROFILE%\.gemini\config\mcp_config.json'
  scope: user
- format: json
  notes: Per-project resources mapping folder URIs; a workspace-to-project cache also lives under ~/.gemini/antigravity-cli/cache/.
  os: macos
  path: ~/.gemini/config/projects/<project-id>.json
  scope: user
- format: json
  notes: Per-project resources.
  os: linux
  path: ~/.gemini/config/projects/<project-id>.json
  scope: user
- format: json
  notes: Windows spelling.
  os: windows
  path: '%USERPROFILE%\.gemini\config\projects\<project-id>.json'
  scope: user
- format: json
  notes: Optional explicit skills registry (entries + inherits manifests); rules.json and agents.json follow the same schema in the same root.
  os: macos
  path: ~/.gemini/config/skills.json
  scope: user
- format: json
  notes: Optional explicit skills registry.
  os: linux
  path: ~/.gemini/config/skills.json
  scope: user
- format: json
  notes: Windows spelling.
  os: windows
  path: '%USERPROFILE%\.gemini\config\skills.json'
  scope: user
- format: json
  notes: Optional explicit plugin registry, same manifest schema.
  os: macos
  path: ~/.gemini/config/plugins.json
  scope: user
- format: json
  notes: Optional explicit plugin registry.
  os: linux
  path: ~/.gemini/config/plugins.json
  scope: user
- format: json
  notes: Windows spelling.
  os: windows
  path: '%USERPROFILE%\.gemini\config\plugins.json'
  scope: user
- format: other
  notes: Workspace customization root (skills/, rules, agents, hooks.json, skills.json, agents.json); alternates .agent/, _agents/, and _agent/ are also discovered.
  os: macos
  path: <repo>/.agents/
  scope: repo
- format: other
  notes: Workspace customization root with the same alternates.
  os: linux
  path: <repo>/.agents/
  scope: repo
- format: other
  notes: Windows spelling; alternates .agent\, _agents\, _agent\.
  os: windows
  path: <repo>\.agents\
  scope: repo
- format: json
  notes: Named lifecycle hooks (PreToolUse, PostToolUse, PreInvocation, ...) executed as shell commands; merged across plugins and configs.
  os: macos
  path: <repo>/.agents/hooks.json
  scope: repo
- format: json
  notes: Named lifecycle hooks.
  os: linux
  path: <repo>/.agents/hooks.json
  scope: repo
- format: json
  notes: Windows spelling.
  os: windows
  path: <repo>\.agents\hooks.json
  scope: repo
- format: json
  notes: Repo-level manifest pointing at shared skill directories (team sharing via VCS).
  os: macos
  path: <repo>/.agents/skills.json
  scope: repo
- format: json
  notes: Repo-level skills manifest.
  os: linux
  path: <repo>/.agents/skills.json
  scope: repo
- format: json
  notes: Windows spelling.
  os: windows
  path: <repo>\.agents\skills.json
  scope: repo
env_vars:
- effect: Set to true to disable the background self-updater that replaces the installed binary (documented on the troubleshooting page); without it the CLI checks for updates on a 15-minute debounce and applies them during normal runs.
  name: AGY_CLI_DISABLE_AUTO_UPDATE
- effect: Caps the maximum height of command outputs in the TUI as a percentage of terminal height.
  name: AGY_CLI_CMD_OUTPUT_PERCENTAGE
- effect: Disables LaTeX/math rendering globally in the CLI.
  name: AGY_CLI_DISABLE_LATEX
- effect: Disables ASCII rendering of Mermaid diagrams; present in the installed binary's strings, exact accepted values undocumented.
  name: AGY_CLI_DISABLE_MERMAID_ASCII
- effect: Disables inline Mermaid diagram rendering; present in the installed binary's strings, exact accepted values undocumented.
  name: AGY_CLI_DISABLE_INLINE_MERMAID
- effect: Hides the account email and plan tier from the TUI header.
  name: AGY_CLI_HIDE_ACCOUNT_INFO
- effect: Suppresses the startup logo; present in the installed binary's strings, exact accepted values undocumented.
  name: AGY_CLI_HIDE_LOGO
- effect: Forces OSC8 hyperlink rendering; present in the installed binary's strings, exact accepted values undocumented.
  name: AGY_CLI_FORCE_OSC8
- effect: Rendering feature toggle; present in the installed binary's strings, exact accepted values undocumented.
  name: AGY_CLI_EXPERIMENTAL_RENDERING
- effect: Overrides the logo style; present in the installed binary's strings, exact accepted values undocumented.
  name: AGY_CLI_LOGO_STYLE
- effect: macOS option-key-as-alt behavior toggle; present in the installed binary's strings, exact accepted values undocumented.
  name: AGY_CLI_MAC_OPTION_AS_ALT
- effect: Fast prompt editor toggle; present in the installed binary's strings, exact accepted values undocumented.
  name: AGY_CLI_FAST_PROMPT_EDITOR
- effect: Suppresses input-mode reassertion; present in the installed binary's strings, exact accepted values undocumented.
  name: AGY_CLI_DISABLE_INPUT_MODE_REASSERT
- effect: Disables escape-sequence optimization; present in the installed binary's strings, exact accepted values undocumented.
  name: AGY_CLI_DISABLE_ESCAPE_SEQUENCE_OPTIMIZATIONS
- effect: Headless override for interactive sessions (test/automation posture); present in the installed binary's strings, exact accepted values undocumented.
  name: AGY_CLI_INTERACTIVE_HEADLESS
- effect: Headless override for non-interactive sessions; present in the installed binary's strings, exact accepted values undocumented.
  name: AGY_CLI_NONINTERACTIVE_HEADLESS
- effect: 'Drives a CDE auth helper; the binary rejects unknown values with `Unknown AGY_CLI_CDE_AUTH_ACTION: %q (supported: check, login)`.'
  name: AGY_CLI_CDE_AUTH_ACTION
- effect: Application-default-credential token source override; the sign-out path instructs `unset AGY_ADC_AUTH` to log out.
  name: AGY_ADC_AUTH
- effect: WebSocket URL of the browser instance the browser tools attach to; present in the installed binary's strings.
  name: AGY_BROWSER_WS_URL
- effect: File the browser control layer reads the active DevTools port from; present in the installed binary's strings.
  name: AGY_BROWSER_ACTIVE_PORT_FILE
- effect: Pins the Remote Control web UI version; present in the installed binary's strings, exact accepted values undocumented.
  name: AGY_CLI_REMOTE_CONTROL_WEB_VERSION
- effect: Overrides the remote-control component version; present in the installed binary's strings, exact accepted values undocumented.
  name: AGY_REMOTE_CONTROL_VERSION_OVERRIDE
- effect: Undocumented internal toggle; present in the installed binary's strings, purpose not established.
  name: AGY_ENABLE_HUB
- effect: Overrides the business pay-as-you-go tier selection; present in the installed binary's strings, exact accepted values undocumented.
  name: AGY_BUSINESS_PAYGO_TIER
machine_introspection:
- command: agy --version
  machine_readable: true
  notes: Single semver line on stdout, exit 0. Works from any argv position (pre-scanned), so `agy models --version` and even `agy -- --version` report it.
  output_format: text
  purpose: version
  useful_for_codegen: false
- command: agy models
  machine_readable: false
  notes: Slug<TAB>display-name table on stdout with `Fetching available models...` on stderr; requires a signed-in account. The --output-format json mode added in 1.1.12 is rejected again on 1.2.14.
  output_format: text
  purpose: models
  useful_for_codegen: false
- command: agy agents
  machine_readable: false
  notes: One agent name per line on stdout, nothing on stderr; same output as `agy agent`.
  output_format: text
  purpose: other
  useful_for_codegen: false
- command: agy mcp list
  machine_readable: false
  notes: NAME/TYPE/STATUS/COMMAND-URL fixed-width table on stdout, exit 0; no JSON mode.
  output_format: table
  purpose: mcp
  useful_for_codegen: false
- command: agy remote-control status
  machine_readable: false
  notes: 'Daemon status lines; prints `Daemon status: not running` with exit 0 when no daemon is registered.'
  output_format: text
  purpose: other
  useful_for_codegen: false
- command: agy -p "<prompt>" --output-format json
  machine_readable: true
  notes: One JSON envelope (conversation_id, status, response, error, duration_seconds, num_turns, usage) on completion. Starts a billable model session; not a catalog endpoint.
  output_format: json
  purpose: other
  useful_for_codegen: false
- command: agy -p "<prompt>" --output-format stream-json
  machine_readable: true
  notes: NDJSON events (init, step_update, result) carrying tools, permission_mode, per-step usage, tool_info, and subagent_info. Same billing caveat.
  output_format: jsonl
  purpose: other
  useful_for_codegen: false
- command: agy changelog
  machine_readable: false
  notes: Cached, dated release notes; can describe a newer version than the running binary.
  output_format: text
  purpose: other
  useful_for_codegen: false
wrapper_notes:
- 'The binary self-updates in the background: during this research the installed agy moved from 1.2.13 to 1.2.14 mid-session (binary mtime and updater/update_status.json changed). Pin with AGY_CLI_DISABLE_AUTO_UPDATE=true when a wrapper needs a stable version; the updater uses a 15-minute TTL debounce and an advisory update.lock under ~/.gemini/antigravity-cli/updater/.'
- 'Exit codes are layered, not boolean: 0 success (but also a TUI launch failing without a TTY — `CLI error: bubbletea: error opening TTY` exits 0); 1 startup validation (unknown --model/--effort, --json-schema gating) and subcommand flag errors; 2 root flag parse errors (invalid value, missing value, undefined flag, stray positional, input/output-format pairing); 3 a mid-run model/agent failure after partial output, accompanied by a structured `AGY_ERROR: {...}` line on stderr. Parse stderr or the JSON status field; never trust exit 0 alone.'
- The root flag parser stops at the first stray positional and rejects it (`unexpected argument ... Prompts are read only from -p/--print, -i/--prompt-interactive, or stdin`, exit 2); value flags placed after that positional are never parsed. Prompts must ride a flag, never a bare argument.
- '--help/-h and --version are pre-scanned across the entire argv before normal parsing: `agy hello --help` prints usage, `agy models --version` prints the version, and `agy -- --version` still prints the version. A literal --version or --help token intended for a child process cannot be shielded with `--`.'
- 'One-dash long spellings are accepted parser-wide: `-print-timeout=bogus`, `-model x`, and `-version` all parse exactly like their two-dash forms. A wrapper partitioning arguments should treat `-<longname>` and `--<longname>` alike.'
- 'Boolean flags never consume the following token (`--new-project bogus` errors on the stray `bogus`) and reject attached values without `=` (`-cx` is `flags provided but not defined: -cx`; `-c=x` is invalid boolean value unless x parses as bool). Short value flags accept attached (`-phello`), equals (`-p=hello`), and space forms.'
- --output-format is not validated before model resolution (a bogus value passed startup while the model check preempted it), so a mistyped format can waste a billable turn before failing; --input-format IS validated early (exit 2).
- 'mcp subcommand flags must precede their operands; `agy mcp add` documents `--` before the command to pass arguments that begin with `-`. The plugin family is the exception: plugin subcommand tails are not flag-parsed at all (`plugin list --help` prints the list).'
- 'Discovery output is stream-clean on 1.2.14: models/agents/mcp list write data to stdout with progress and errors on stderr — capture stdout directly. Older versions mingled them.'
- '`agy changelog` can describe a newer version than the running binary (it listed 1.2.14 notes while the binary was 1.2.13); do not use it as a version probe — use `agy --version`.'
- Unauthenticated headless runs no longer exit 0 (the 1.1.0 behavior); the docs state a no-terminal, unauthenticated run exits with an `authentication required` error instead of hanging. Not verifiable on this signed-in host.
- --print-timeout expiry mid-turn returns the partial output and exits 0 with a stderr warning (since 1.1.28); treat a short response plus that warning as a timeout, not a clean completion.
- In --input-format stream-json sessions the CLI reads prompts from stdin only (a -p prompt is dropped), ends with exit 1 on malformed input and exit 2 for CLI-handled slash commands or control events, and exits 0 when stdin closes after the final result.
- 'Windows: the CLI is agy.exe in %LOCALAPPDATA%\agy\bin (the troubleshooting page''s C:\Program Files path contradicts the installer; trust the installer). The CMD installer rejects command lines containing &, |, ;, <, >, or ^.'
- The CLI writes state under ~/.gemini/antigravity-cli/ on every run (cli.log, cache/, updater/, conversations/, project cache) and `agy install` mutates shell profiles unless skipped; sandbox a wrapper's HOME when hermetic behavior matters.
changes:
- Verified against installed 1.2.13 and 1.2.14 (the binary self-updated mid-research; upstream latest is 1.2.14, published 2026-09-30) and re-typed every switch record to contract revision 2 with parse-probe evidence for value types and attachment forms.
- 'New subcommands since the 1.1.0 document: agent/agents, mcp (add, remove, list, enable, disable), mic-serve, and remote-control (start, status, stop); plugin help is now headless (the Bubble Tea failure is gone).'
- 'New root switches: --agent, --disable-slash-commands, --effort, --input-format (stream-json stdin sessions), --json-schema, --remote-control; --output-format is now documented in help and gained stream-json; --prompt-interactive/-i retained.'
- --print-timeout default changed from 5m to 0s/unlimited (1.2.6), and expiry now returns partial output with exit 0 (1.1.28); the headless docs page still says 5m.
- 'Exit-code taxonomy recorded: root parse errors exit 2, subcommand flag errors and startup validation exit 1, mid-run failures exit 3 with an AGY_ERROR JSON line on stderr, and TUI-launch failures exit 0.'
- 'Hidden switches found: --version (bool, pre-scanned across argv, works past -- and after subcommand tokens) and a hidden integer -v; -h is honored at the root even though help does not list it.'
- 'Parser semantics established by test: parsing stops at the first stray positional; help/version are pre-scanned; one-dash long spellings accepted; booleans reject attached values without =; short value flags accept space/equals/attached.'
- The machine-readable --output-format on models/agents (added 1.1.12) is rejected again on 1.2.14; models is a stdout TSV with stderr progress.
- Environment-variable inventory expanded from binary strings (24 general AGY_* variables recorded; model-endpoint variables left to model-config) and AGY_CLI_DISABLE_AUTO_UPDATE confirmed documented.
- Config discovery updated for the customization manifest schema (entries/inherits in skills.json/plugins.json/rules.json/agents.json), repo .agents/hooks.json lifecycle hooks, and the sidecars directory.
requires_claudine_update: true
reason: 'The verified surface has moved far past the metadata generated from the 1.1.0-era document: new root switches (--agent, --effort, --input-format, --json-schema, --disable-slash-commands, --remote-control, documented --output-format), new subcommand families (agent, mcp, mic-serve, remote-control), the unlimited --print-timeout default, the 1/2/3 exit-code taxonomy with the AGY_ERROR line, and the first typed switch inventory for argument partitioning all require regenerating the antigravity provider metadata and revisiting the wrapper''s argument handling.'
contract_checked: 2026-10-01
---

# Antigravity CLI (`agy`) — Command-Line Surface

## Overview

Antigravity CLI is Google's terminal interface to the Antigravity agent platform. Google ships it as a single self-contained Go binary named `agy` (about 187 MB on macOS) that contains the terminal UI, the headless print mode, and a background self-updater. The primary command a user or wrapper types is `agy`.

The version verified for this research is **1.2.14**, and the binary under observation moved from **1.2.13** to **1.2.14** *during* the session: `agy --version` first returned `1.2.13`, the background updater then replaced `/Users/ken/.local/bin/agy` (mtime and `~/.gemini/antigravity-cli/updater/update_status.json` both changed), and subsequent runs returned `1.2.14`. That live self-update is itself a verified finding, not an anomaly. Upstream, the GitHub latest-release API returned tag `1.2.14` (published 2026-09-30) and the auto-updater manifest served `1.2.14`, so 1.2.14 is the newest released version. Help output was re-verified on the updated binary and is identical across the two versions examined.

| Resource | URL |
| --- | --- |
| Homepage | <https://antigravity.google/product/antigravity-cli> |
| Repository (releases) | <https://github.com/google-antigravity/antigravity-cli> |
| General docs | <https://antigravity.google/docs/cli/overview/> |
| CLI reference (slash commands, settings keys) | <https://antigravity.google/docs/cli/reference/> |
| Headless mode (command-line flags) | <https://antigravity.google/docs/cli/headless/> |
| Installation and auth | <https://antigravity.google/docs/cli/install/> |
| Troubleshooting | <https://antigravity.google/docs/cli/troubleshooting/> |

The docs site was restructured since the 1.1.0-era research: the old `/docs/cli-overview` and `/docs/cli-reference` URLs are gone, the reference page now covers TUI slash commands and `settings.json` keys, and command-line flag documentation lives on the headless-mode page.

## Installation and Binaries

The command is `agy` on macOS and Linux and `agy.exe` on Windows.

| OS | Binary | Default install path | Official command |
| --- | --- | --- | --- |
| macOS | `agy` | `~/.local/bin/agy` | `curl -fsSL https://antigravity.google/cli/install.sh \| bash` |
| Linux | `agy` | `~/.local/bin/agy` | `curl -fsSL https://antigravity.google/cli/install.sh \| bash` |
| Windows (PowerShell) | `agy.exe` | `%LOCALAPPDATA%\agy\bin\agy.exe` | `irm https://antigravity.google/cli/install.ps1 \| iex` |
| Windows (CMD) | `agy.exe` | `%LOCALAPPDATA%\agy\bin\agy.exe` | `curl -fsSL https://antigravity.google/cli/install.cmd -o install.cmd && install.cmd && del install.cmd` |

The Unix installer (read in full this run) accepts `-d, --dir <path>` and `-h, --help`, detects `darwin`/`linux` on `amd64`/`arm64` plus musl and Android/Termux, stages the download under `~/.cache/antigravity/staging`, verifies the SHA-512 from the updater manifest at `https://antigravity-cli-auto-updater-974169037036.us-central1.run.app/manifests/<platform>.json`, clears macOS quarantine attributes, and exits early when `agy` already exists — because the CLI updates itself in the background from then on. The installer scripts also honor `--skip-aliases` and `--skip-path`, which they forward to `agy install`. One docs-site contradiction to know about: the troubleshooting page shows a PowerShell PATH example pointing at `C:\Program Files\Google\antigravity-cli`, while the install page and the installer itself use `%LOCALAPPDATA%\agy\bin`; trust the installer.

Local host evidence: `/Users/ken/.local/bin/agy` (187 MB, Go binary). `/Applications/Antigravity.app` (desktop app) and `/Applications/Antigravity IDE.app` are separate products, not the CLI.

## Subcommands

Root help lists twelve subcommand words; with the second level, the full set of native command paths is:

| Command path | Description | Non-interactive |
| --- | --- | --- |
| `agent` | Lists available agents, one name per line on stdout | Yes (verified) |
| `agents` | Alias for `agent` (usage header says `agy agent`) | Yes |
| `changelog` | Shows cached changelog and release notes | Yes |
| `help` | Shows top-level help, or `agy help <sub>` for one subcommand | Yes |
| `install` | Configures shell PATH and profile settings | Yes, but mutates shell profiles unless `--skip-path`/`--skip-aliases` |
| `mcp` | Dispatcher for MCP server management | Yes |
| `mcp add` | Adds/updates an MCP server (`[flags] <name> <commandOrUrl> [args...]`) | Yes; writes `mcp_config.json` |
| `mcp remove` | Removes an MCP server by name | Yes |
| `mcp list` | Lists configured servers as a table on stdout | Yes (verified) |
| `mcp enable` / `mcp disable` | Enables/disables a server by name | Yes |
| `mic-serve` | Serves the local microphone to a CLI on another host | No — long-running server; OS microphone permission may prompt |
| `models` | Lists model slugs for the signed-in account | Yes (verified; requires auth) |
| `plugin` / `plugins` | Plugin management (list, import, install, uninstall, enable, disable, validate, link) | Mixed — help and `list` are headless on 1.2.14; other flows unverified |
| `remote-control` | Dispatcher for the remote-control daemon | Yes |
| `remote-control start` | Registers/starts the daemon (`--name`, `--session`) | Not exercised; mutates OS service registration |
| `remote-control status` | Prints daemon status | Yes (verified) |
| `remote-control stop` | Stops and unregisters the daemon | Not exercised; mutating |
| `update` | Updates the CLI binary | Yes by design (the same updater runs headless in the background); not run directly |

There is no `resume` subcommand — conversation resumption is a root-switch concern (`--continue`/`--conversation` with `-p`), plus the interactive `/resume` slash command inside the TUI.

## CLI Switch Inventory

Inventoried paths: the root entrypoint (`agy ...`) and every path marked non-interactive above, including the resume-by-switch forms; `install`, `mic-serve`, `remote-control start`, and the `mcp` family are included even where marked interactive or mutating because their flag tables are fully established. Every value type and attachment form below was established by disposable parse probes against the installed 1.2.14 binary (an invalid value rejected with an error naming the switch), the flag tables in each path's `--help`, or the headless documentation — never by inference from a placeholder.

### How the parser reads a command line

The root parser is a Go-`flag`-style parser with two Antigravity-specific layers, all established by test:

- **Value forms.** Long flags take one value by space (`--print-timeout 30s`), by equals (`--print-timeout=30s`), and under a single-dash long spelling (`-print-timeout=30s`). Short one-character value flags (`-p`, `-i`, `-e`, `-t`, `-H`) accept space, equals (`-p=hello`), and attached (`-phello`) forms.
- **Booleans.** Boolean flags take no value: they never consume the next token (`agy --new-project bogus` errors on the stray `bogus`) and reject an attached value unless it follows `=` and parses as a bool (`-c=x` → `invalid boolean value "x"`; `-cx` → `flags provided but not defined: -cx`).
- **Parsing stops at the first stray positional.** `agy hello --print-timeout bogus` fails on `hello` (exit 2) without ever reading the flag behind it; prompts are only read from `-p`/`--print`, `-i`/`--prompt-interactive`, or stdin, never from bare arguments.
- **Help and version are pre-scanned across the whole argv.** `agy hello --help` prints usage exit 0; `agy models --version` prints the version; `agy -- --version` still prints the version. `--version` is therefore effectively *global* — accepted at every command path — and cannot be shielded by `--`.
- **`--` terminates flags** (`agy -- hello` treats `hello` as a positional and errors), except for the help/version pre-scan.
- **Per-subcommand flagsets.** Each subcommand path parses its own flags and rejects undefined ones with its own usage (`agy mcp list --json` → mcp list usage, exit 1; `agy agent --model x` → agent usage, exit 1). Root switches do not apply below a subcommand. `mcp add`'s help states the operand rule explicitly: flags must precede `<name>`, and `--` guards a command or args that begin with `-`. The `plugin` family is the exception — its subcommand tails are not flag-parsed at all.

### Root switches (`agy` with no subcommand)

| Flag | Aliases | Value | Default | Attachment | Notes |
| --- | --- | --- | --- | --- | --- |
| `--add-dir` | — | one path, repeatable | `[]` | space, equals | Each occurrence adds one directory. |
| `--agent` | — | agent name | unset | space, equals | Name from `agy agents`; not validated before model resolution. |
| `--continue` | `-c` | none | off | — | Continue most recent conversation. |
| `--conversation` | — | conversation ID | unset | space, equals | Resume a specific conversation. |
| `--dangerously-skip-permissions` | — | none | off | — | Permission semantics: agent-permissions topic. |
| `--disable-slash-commands` | — | none | off | — | Print mode only. |
| `--effort` | — | `low\|medium\|high\|max` | unset | space, equals | Startup-validated in headless (exit 1); help lists `max`, docs page omits it. |
| `--prompt-interactive` | `-i` | prompt | unset | space, equals, attached | Interactive first turn; needs a TTY after it. |
| `--input-format` | — | `text\|stream-json` | `text` | space, equals | Validated early (exit 2); stream-json requires stream-json output; prompts arrive as `user` events on stdin. |
| `--json-schema` | — | schema string or file path | unset | space, equals | Requires `--output-format json\|stream-json`; 1.2.14 rejects non-object roots at startup, exit 1. |
| `--log-file` | — | path | default log path | space, equals | Logging depth: agent-logging topic. |
| `--mode` | — | `accept-edits\|plan` | unset | space, equals | Not validated before model resolution. |
| `--model` | — | model slug | from settings | space, equals | Headless exits 1 on unknown slug; interactive may fall back. Catalog: model-config topic. |
| `--new-project` | — | none | off | — | |
| `--output-format` | — | `text\|json\|stream-json` | `text` | space, equals | Envelope/event shapes in headless docs; *not* validated before model resolution. |
| `--print` | `-p`, `--prompt` | prompt | unset | space, equals, attached (`-p`) | The headless entry point. |
| `--print-timeout` | — | Go duration | `0s` (unlimited) | space, equals | Default changed 5m → 0s in 1.2.6; expiry returns partial output, exit 0. |
| `--project` | — | project ID or name | inferred | space, equals | |
| `--remote-control` | — | none | off | — | Session-scoped remote connection at startup. |
| `--sandbox` | — | none | off | — | Sandbox policy: agent-permissions topic. |
| `--help` | `-h` | none | — | — | Hidden from usage; honored everywhere, pre-scanned past positionals and `--`. |
| `--version` | `-version` | none | — | — | Hidden bool; pre-scanned globally, works past `--` and after subcommand tokens. `-v` is a different flag. |
| `-v` | — | one integer | unset | space, equals | Hidden (`strconv.Atoi`); purpose undocumented — see the record's gap. |

Negative results worth recording: `--json`, `--format`, `--verbose`, `-o`, and `--system-prompt`/`--append-system-prompt`/`--replace-system-prompt`/`--instruction` are all `flags provided but not defined`. No system-prompt delivery switch exists in 1.2.14 (the system-prompt topic owns that area if one appears).

### Subcommand switches

| Command path | Flag | Aliases | Value | Default | Attachment |
| --- | --- | --- | --- | --- | --- |
| `install` | `--dir` | — | path | binary dir | space, equals |
| `install` | `--skip-aliases` | — | none | off | — |
| `install` | `--skip-path` | — | none | off | — |
| `mcp add` | `--env` | `-e` | `KEY=value`, repeatable | `[]` | space, equals, attached |
| `mcp add` | `--header` | `-H` | `Key: Value`, repeatable | `[]` | space, equals, attached |
| `mcp add` | `--type` | `-t` | `stdio\|http` | `stdio` | space, equals, attached |
| `mic-serve` | `--addr` | — | listen address | `127.0.0.1:4713` | space, equals |
| `remote-control start` | `--name` | — | instance label | generated | space, equals |
| `remote-control start` | `--session` | — | none | off | — |
| `update` | `--help` | — | none | — | prints `Usage of update:`, exit **2**; `-h` rejected exit 1 |
| most paths | `-h`, `--help` | — | none | — | exit 0; see parser notes for the irregular paths |

### Exit codes observed

| Exit | Where |
| --- | --- |
| 0 | success; `--help`/`--version`; `--print-timeout` expiry with partial output; **and** a TUI launch failing without a TTY (`CLI error: bubbletea ...`) |
| 1 | startup validation (unknown `--model`, invalid `--effort`, `--json-schema` gating); subcommand flag errors and unknown sub-subcommands; unknown model in headless |
| 2 | root flag parse errors (invalid value, missing value, undefined flag, stray positional); `--input-format` pairing errors; `update --help` |
| 3 | mid-run model/agent failure after partial output, with a structured `AGY_ERROR: {...}` JSON line on stderr (canonical status, HTTP or gRPC code, retryability, error ID) |

## Configuration Discovery

Two user-level roots and one repo root:

- `~/.gemini/antigravity-cli/` — CLI-private settings and state: `settings.json` (local file held `model` and `trustedWorkspaces`; documented keys include `toolPermission`, `artifactReviewPolicy`, `permissions.allow`, `modelProvider`, `colorScheme`, `verbosity`, `enableTerminalSandbox`, `editor`, `notifications`), `keybindings.json`, plus written state — `cli.log`, `cache/`, `conversations/`, `updater/`, `installation_id`, `builtin/skills/`.
- `~/.gemini/config/` — shared Antigravity configuration: `config.json` (`userSettings`), `mcp_config.json` (what `agy mcp add/remove/enable/disable` edit), `projects/<project-id>.json`, `skills.json`/`plugins.json`/`rules.json`/`agents.json` manifests (`entries` + `inherits` schema with `include_only`/`exclude` filters), `hooks/`, `sidecars/`.
- `<repo>/.agents/` — workspace customization root (alternates `.agent/`, `_agents/`, `_agent/`): skills, rules, agents, `hooks.json` (named `PreToolUse`/`PostToolUse`/`PreInvocation` shell-command hooks), and manifest files.

Windows uses `%USERPROFILE%\.gemini\...` spellings of the same layout. Side effects a wrapper should expect: every run writes logs, caches, and updater state under the CLI-private root; `agy install` mutates shell profiles unless skipped; trust prompts persist into `trustedWorkspaces`; authentication lives in the OS keyring (Apple Keychain, Linux secret-service, Windows Credential Manager), with `GEMINI_API_KEY` + `modelProvider: "gemini"` in `settings.json` as the key-based alternative.

## Environment Variables

General CLI and runtime variables (model-endpoint variables — `GEMINI_API_KEY`, `GOOGLE_GEMINI_BASE_URL`, `CLOUD_CODE_URL`, the `AGY_LLM_GATEWAY_*` family, `AGY_CLI_MODEL_API_MAX_RETRIES`, `AGY_CLI_MODEL_EXPERIMENT_PROFILE`, `JETSKI_SERVICE_ENDPOINT_URL` — belong to the model-config topic; `AGY_CLI_DISABLE_SAFETY_FILTERING` to agent-permissions; telemetry sampling to agent-logging):

| Variable | Effect |
| --- | --- |
| `AGY_CLI_DISABLE_AUTO_UPDATE` | Set to `true` to disable the background self-updater (documented); the default behavior replaces the installed binary on a 15-minute debounce. |
| `AGY_CLI_CMD_OUTPUT_PERCENTAGE` | Caps TUI command-output height as a percentage of terminal height. |
| `AGY_CLI_DISABLE_LATEX` | Disables LaTeX/math rendering. |
| `AGY_CLI_DISABLE_MERMAID_ASCII` | Disables ASCII rendering of Mermaid diagrams (binary strings). |
| `AGY_CLI_DISABLE_INLINE_MERMAID` | Disables inline Mermaid rendering (binary strings). |
| `AGY_CLI_HIDE_ACCOUNT_INFO` | Hides account email and plan tier in the header. |
| `AGY_CLI_HIDE_LOGO` | Suppresses the startup logo (binary strings). |
| `AGY_CLI_FORCE_OSC8` | Forces OSC8 hyperlink rendering (binary strings). |
| `AGY_CLI_EXPERIMENTAL_RENDERING` | Rendering feature toggle (binary strings). |
| `AGY_CLI_LOGO_STYLE` | Logo style override (binary strings). |
| `AGY_CLI_MAC_OPTION_AS_ALT` | macOS option-as-alt toggle (binary strings). |
| `AGY_CLI_FAST_PROMPT_EDITOR` | Fast prompt editor toggle (binary strings). |
| `AGY_CLI_DISABLE_INPUT_MODE_REASSERT` | Suppresses input-mode reassertion (binary strings). |
| `AGY_CLI_DISABLE_ESCAPE_SEQUENCE_OPTIMIZATIONS` | Disables escape-sequence optimization (binary strings). |
| `AGY_CLI_INTERACTIVE_HEADLESS` | Headless override for interactive sessions (binary strings). |
| `AGY_CLI_NONINTERACTIVE_HEADLESS` | Headless override for non-interactive sessions (binary strings). |
| `AGY_CLI_CDE_AUTH_ACTION` | CDE auth helper action; rejected values name `check, login`. |
| `AGY_ADC_AUTH` | ADC token source override; sign-out instructs unsetting it. |
| `AGY_BROWSER_WS_URL` | Browser-control WebSocket URL (binary strings). |
| `AGY_BROWSER_ACTIVE_PORT_FILE` | Browser active DevTools port file (binary strings). |
| `AGY_CLI_REMOTE_CONTROL_WEB_VERSION` | Pins the Remote Control web UI version (binary strings). |
| `AGY_REMOTE_CONTROL_VERSION_OVERRIDE` | Overrides the remote-control component version (binary strings). |
| `AGY_ENABLE_HUB` | Undocumented internal toggle (binary strings). |
| `AGY_BUSINESS_PAYGO_TIER` | Business pay-as-you-go tier override (binary strings). |

"Binary strings" means the name is present in the installed 1.2.14 binary and its purpose is consistent with adjacent error strings, but no help text, docs page, or changelog entry states its accepted values.

## Machine Introspection

| Command | Purpose | Machine-readable | Format | Notes |
| --- | --- | --- | --- | --- |
| `agy --version` | version | yes | text | One semver line on stdout, exit 0; honored from any argv position. |
| `agy models` | models | no | text | `slug<TAB>name` on stdout, `Fetching available models...` on stderr; needs sign-in. The `--output-format json` mode added in 1.1.12 is rejected again in 1.2.14. |
| `agy agents` | agent roster | no | text | One name per line on stdout. |
| `agy mcp list` | MCP state | no | table | `NAME TYPE STATUS COMMAND/URL` on stdout. |
| `agy remote-control status` | daemon state | no | text | Exit 0 with `not running` when unregistered. |
| `agy -p "<prompt>" --output-format json` | run result | yes | json | Envelope with `conversation_id`, `status`, `response`, `error`, `usage`; starts a billable session. |
| `agy -p "<prompt>" --output-format stream-json` | run events | yes | jsonl | NDJSON `init`/`step_update`/`result` with tools, permission_mode, per-step usage, `tool_info`, `subagent_info`. |
| `agy changelog` | release notes | no | text | Cached; can describe a newer version than the binary. |

`--help` output is not listed as introspection. There is no config-dump, doctor, or schema command in the public surface.

## Wrapper Notes

- **The binary mutates itself.** The background updater replaced 1.2.13 with 1.2.14 mid-research. Pin with `AGY_CLI_DISABLE_AUTO_UPDATE=true`; expect `~/.gemini/antigravity-cli/updater/update.lock` and a 15-minute `last_check.timestamp` debounce.
- **Exit 0 is not success.** A TUI launch without a TTY exits 0 (`CLI error: bubbletea: error opening TTY`). Parse stderr and the JSON `status` field; the failure vocabulary is exit 1 (startup validation, subcommand flag errors), exit 2 (root parse errors, format pairing), exit 3 (mid-run failure, with the `AGY_ERROR: {...}` stderr line).
- **Arguments for the agent must be flags.** The root parser stops at the first stray positional and rejects it; a prompt after `--` or as a bare word is an error, not a prompt.
- **`--version` and `--help` are pre-scanned everywhere** — past positionals, past `--`, and after subcommand tokens. Never forward a literal `--version`/`--help` intended for something else.
- **Treat `-longname` and `--longname` as the same flag** (observed for `-print-timeout`, `-model`, `-version`).
- **Booleans do not eat the next token**, and short-bool clusters like `-cx` are undefined; only `-c=x` style parses.
- **`--output-format` is not validated before the model call** — a typo can burn a billable turn; `--input-format` fails fast instead.
- **Discovery output is stream-clean** (models/agents/mcp list data on stdout, progress on stderr) as of the 1.1.23/1.1.12-era fixes; subcommands also no longer hang on an inherited, unclosed stdin pipe.
- **stream-json sessions** read prompts from stdin only (a `-p` value is dropped), exit 1 on malformed input, 2 on CLI-handled slash commands or control events, and 0 after stdin closes post-result.
- **`--print-timeout` expiry returns partial output with exit 0** and a stderr warning — distinguishable from a clean completion only by that warning.
- **Unauthenticated headless runs now fail loudly** per the docs (`authentication required`, non-zero) instead of the 1.1.0-era exit-0 auth failure; authenticate once interactively, or use `GEMINI_API_KEY` + `modelProvider` (model-config topic).
- **The changelog cache can run ahead of the binary**; use `--version` for the running version.
- **Windows**: resolve `agy.exe` under `%LOCALAPPDATA%\agy\bin` (not the `C:\Program Files\Google\antigravity-cli` path the troubleshooting page shows), and remember the CMD installer rejects command lines containing `&`, `|`, `;`, `<`, `>`, or `^`.
- **Every run writes state** under `~/.gemini/antigravity-cli/` (logs, caches, conversations, updater); sandbox `HOME` for hermetic wrapping, and expect `trustedWorkspaces` trust prompts on first contact with a new directory.

## Sources

- [Headless mode (flags, formats, exit codes, stream protocol)](https://antigravity.google/docs/cli/headless/)
- [Installation and auth](https://antigravity.google/docs/cli/install/)
- [Troubleshooting (self-updater, keyring, AGY_CLI_DISABLE_AUTO_UPDATE)](https://antigravity.google/docs/cli/troubleshooting/)
- [CLI reference (slash commands, settings.json keys)](https://antigravity.google/docs/cli/reference/)
- [CLI overview](https://antigravity.google/docs/cli/overview/) and [product page](https://antigravity.google/product/antigravity-cli)
- [Repository](https://github.com/google-antigravity/antigravity-cli) and [latest release API](https://api.github.com/repos/google-antigravity/antigravity-cli/releases/latest)
- [Unix installer script](https://antigravity.google/cli/install.sh) and [darwin_arm64 updater manifest](https://antigravity-cli-auto-updater-974169037036.us-central1.run.app/manifests/darwin_arm64.json)
- Local commands on the installed binary (`/Users/ken/.local/bin/agy`): `--version`, `--help`, per-subcommand `--help` batteries, and the disposable parse probes recorded in the frontmatter evidence (`print-long-forms`, `parse-bool-forms`, `parse-needs-argument`, `parse-short-attached`, `parse-positional-stop`, `parse-prescan-help-version`, `startup-validation`, `mcp-rc-install-mic-flag-tests`, `update-flag-quirks`, `hidden-v-flag`, `models-json-rejected`, `tui-no-tty-exit-zero`)
- Local discovery runs: `agy models`, `agy agents`, `agy mcp list`, `agy remote-control status`, `agy changelog`, `agy plugin --help`, `agy plugin list --help` (stdout/stderr split into files)
- Local files inspected: `~/.gemini/antigravity-cli/settings.json`, `~/.gemini/antigravity-cli/updater/update_status.json`, `~/.gemini/config/` layout, `~/.gemini/antigravity-cli/builtin/skills/` (agy-customizations docs, antigravity_guide CLI reference, automation sidecar guide), and `strings` of the installed binary

## Changelog

- 2026-10-01: Revision 2 rewrite. Verified against installed 1.2.13 and 1.2.14 (the binary self-updated during the run; upstream latest 1.2.14, published 2026-09-30). Retyped the entire switch inventory with parse-probe evidence (value types, attachment forms, per-path scopes, exit codes); recorded the parser's stop-at-first-positional and help/version pre-scan semantics; found hidden `--version` (global) and integer `-v`; added the new subcommand families (`agent`, `mcp`, `mic-serve`, `remote-control`) and new root switches (`--agent`, `--disable-slash-commands`, `--effort`, `--input-format`, `--json-schema`, `--remote-control`, documented `--output-format`); recorded the 1/2/3 exit-code taxonomy with the `AGY_ERROR` stderr line; noted `--print-timeout` now defaults to unlimited; expanded the environment-variable inventory from binary strings; updated config discovery for manifest schemas, `hooks.json`, and sidecars; noted that `models`/`agents` machine-readable `--output-format` (1.1.12) is rejected again, that plugin help no longer needs a TTY, and that the docs site was restructured (old `/docs/cli-overview` and `/docs/cli-reference` URLs are gone).
- 2026-09-28 (prior document): added the interactive startup-prompt form (`--prompt-interactive`).
- 2026-07-08 (prior document): initial research at 1.1.0.