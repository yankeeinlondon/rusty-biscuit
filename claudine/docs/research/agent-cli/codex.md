---
$schema: ./_schema.yaml
schema_revision: 2
provider: codex
created: 2026-07-02
last_updated: 2026-10-01
agent: claude
model: sonnet
reasoning_effort: high
latest_version: 0.160.0
versions_examined:
- 0.159.3
evidence:
- claim: Spellings, short aliases, placeholders, "..." variadic markers, and which switches each command path lists.
  id: local-help
  limitations: Help text does not show hidden aliases, whether an attached form is accepted, or whether a "..." placeholder consumes greedily.
  location: codex <path> --help for the root and 72 command paths, run on macOS arm64 against the npm-installed binary
  method: local_inspection
  observed_on: 2026-10-01
  version: 0.159.3
- claim: 'clap derive declarations for exec: --json with alias --experimental-json, --color, --output-last-message/-o, --output-schema, and the resume/fork/review argument structs where --image is value_delimiter="," with num_args=1.'
  id: src-exec-cli
  limitations: Declarations only; the fleet did not run an interactive session to see them applied.
  location: https://github.com/openai/codex/blob/rust-v0.159.3/codex-rs/exec/src/cli.rs
  method: source_code
  observed_on: 2026-10-01
  version: 0.159.3
- claim: 'SharedCliOptions: --image/-i with value_delimiter="," and num_args=1.., --model/-m, --oss, --local-provider, --profile/-p, --sandbox/-s, --approve-for-me (alias not-so-yolo), --dangerously-bypass-approvals-and-sandbox (alias yolo), --cd/-C, --worktree, --add-dir.'
  id: src-shared-options
  limitations: Which of these a given command path flattens is read from tui/src/cli.rs, exec/src/cli.rs, and the help output.
  location: https://github.com/openai/codex/blob/rust-v0.159.3/codex-rs/utils/cli/src/shared_options.rs
  method: source_code
  observed_on: 2026-10-01
  version: 0.159.3
- claim: 'Root entrypoint declarations: --strict-config, --ask-for-approval/-a, --search, --no-alt-screen, --no-daemon, the flattened shared options.'
  id: src-tui-cli
  limitations: None for the root switches themselves.
  location: https://github.com/openai/codex/blob/rust-v0.159.3/codex-rs/tui/src/cli.rs
  method: source_code
  observed_on: 2026-10-01
  version: 0.159.3
- claim: 'Top-level clap parser: Subcommand enum with hidden commands, global --enable/--disable, --remote and --remote-auth-token-env, ReviewCommand, DebugPromptInputCommand, DebugModelsCommand.'
  id: src-cli-main
  limitations: Large file; only the argument declarations were read, not the command bodies.
  location: https://github.com/openai/codex/blob/rust-v0.159.3/codex-rs/cli/src/main.rs
  method: source_code
  observed_on: 2026-10-01
  version: 0.159.3
- claim: -c/--config is global, ArgAction::Append, one raw key=value string per occurrence, split at the first "=" and parsed as TOML with a literal-string fallback.
  id: src-config-override
  limitations: Does not cover how a parsed override is merged into configuration.
  location: https://github.com/openai/codex/blob/rust-v0.159.3/codex-rs/utils/cli/src/config_override.rs
  method: source_code
  observed_on: 2026-10-01
  version: 0.159.3
- claim: Every valued switch rejected with "a value is required for '--flag'" when given none (so value_optional is false), and every valueless switch rejected "--flag=x" with "unexpected value" (so it takes none).
  id: test-valued-probes
  limitations: Does not show how many values a valued switch takes beyond one; test-image-greedy covers that.
  location: '148 probes over 14 command paths: a bare valued switch, and a valueless switch given --flag=x, each under a throwaway CODEX_HOME'
  method: disposable_test
  observed_on: 2026-10-01
  version: 0.159.3
- claim: The equals form and, for every switch with a one-character short spelling, the short-attached form are read as the value. Six validating switches (--sandbox, --ask-for-approval, --color, --thread, --max-mib-per-second) rejected the probe value with an error naming the switch, which shows the value was read.
  id: test-attachment-forms
  limitations: Space-separated form is shown by the greedy and bare-switch tests, not by a separate probe per switch.
  location: '86 valued probes: --flag=val and -Xval, each followed by a bare valued switch so the error names the later switch; plus -s/--sandbox=/-sbogus, -abogus, --color=bogus, -cfoo, --config=foo against validating parsers'
  method: disposable_test
  observed_on: 2026-10-01
  version: 0.159.3
- claim: --image/-i in the space form consumes every following operand until the next switch, so a prompt written after it becomes an image path; the equals form, the attached form, a prompt placed first, and "--" each leave the prompt a prompt. At exec resume and exec fork the space form consumes one value and later operands stay positional.
  id: test-image-greedy
  limitations: Run through debug prompt-input and error paths that stop before any model call, never a billed session.
  location: codex debug prompt-input and codex exec --skip-git-repo-check with -i / --image=/-i-attached / -- forms; exec resume and exec fork with -i followed by operands
  method: disposable_test
  observed_on: 2026-10-01
  version: 0.159.3
- claim: --max-mib-per-second is an unsigned integer ("invalid digit found in string" for abc and 1.5); --thread validates a UUID and takes one value per occurrence.
  id: test-number-and-uuid
  limitations: Whether --thread may be repeated was not established because the first value failed validation.
  location: codex migrate-rollouts --max-mib-per-second abc|1.5|-1 and --thread a
  method: disposable_test
  observed_on: 2026-10-01
  version: 0.159.3
- claim: --yolo and --not-so-yolo parse as aliases (the conflict errors name the canonical switches) and --experimental-json parses before --help.
  id: test-aliases
  limitations: Aliases are hidden from help; this establishes acceptance, not that they will persist.
  location: codex exec --yolo --approve-for-me hi; codex exec --not-so-yolo -s read-only hi; codex exec --experimental-json --help
  method: disposable_test
  observed_on: 2026-10-01
  version: 0.159.3
- claim: These commands ran to completion with no terminal, browser, or prompt; doctor and login status exited 1 because the throwaway home had no login.
  id: test-safe-runs
  limitations: exec, review, and the session commands were not run to completion because that costs money.
  location: codex doctor --json --summary, features list, login status, mcp list --json, completion zsh, debug models --bundled, debug prompt-input, migrate-rollouts, run with a throwaway CODEX_HOME and stdin from /dev/null
  method: disposable_test
  observed_on: 2026-10-01
  version: 0.159.3
- claim: 'Command reference: --image takes comma-separated paths or repeats; --yolo is an alias of --dangerously-bypass-approvals-and-sandbox.'
  id: docs-developer-commands
  limitations: Names no version and lists no environment variables; reached through a 308 from developers.openai.com/codex/cli/reference.
  location: https://learn.chatgpt.com/docs/developer-commands?surface=cli
  method: official_docs
  observed_on: 2026-10-01
  version: unknown
- claim: Installer command for macOS and Linux and the binary name codex.
  id: docs-install
  limitations: The Windows tab content did not load; install.ps1 in the repository was read instead.
  location: https://learn.chatgpt.com/docs/codex/cli
  method: official_docs
  observed_on: 2026-10-01
  version: unknown
- claim: 'Configuration layers and their precedence: flags, project .codex/config.toml (trusted projects), profile files, ~/.codex/config.toml, cloud-managed defaults, /etc/codex/config.toml (Unix), built-in defaults; TOML format.'
  id: docs-config
  limitations: Does not mention CODEX_HOME; that comes from install.sh and the binary.
  location: https://learn.chatgpt.com/docs/config-file/config-basic
  method: official_docs
  observed_on: 2026-10-01
  version: unknown
- claim: exec streams progress to stderr and only the final message to stdout; --json emits JSON Lines events; CODEX_API_KEY supplies a key for one run; exec resume --last "prompt"; "-" reads the prompt from stdin.
  id: docs-non-interactive
  limitations: Event field schemas are not reproduced.
  location: https://learn.chatgpt.com/docs/non-interactive-mode
  method: official_docs
  observed_on: 2026-10-01
  version: unknown
- claim: CODEX_INSTALL_DIR defaults to $HOME/.local/bin on Unix and %LOCALAPPDATA%\Programs\OpenAI\Codex\bin on Windows; CODEX_HOME defaults to $HOME/.codex; releases come from releases.openai.com with a GitHub fallback.
  id: src-install-scripts
  limitations: Scripts were read, not executed.
  location: https://github.com/openai/codex/tree/rust-v0.159.3/scripts/install (install.sh, install.ps1)
  method: source_code
  observed_on: 2026-10-01
  version: 0.159.3
- claim: 'Variable names the binary reads: CODEX_HOME, CODEX_SQLITE_HOME, CODEX_API_KEY, CODEX_ACCESS_TOKEN, OPENAI_API_KEY, CODEX_CA_CERTIFICATE, CODEX_THREAD_ID, CODEX_SANDBOX_NETWORK_DISABLED, CODEX_TUI_DISABLE_KEYBOARD_ENHANCEMENT, CODEX_NON_INTERACTIVE, NO_COLOR.'
  id: local-binary-strings
  limitations: A string in the binary shows a name exists, not what it changes; effects below cite documentation or install scripts where they exist.
  location: strings over the darwin-arm64 binary shipped in @openai/codex 0.159.3
  method: local_inspection
  observed_on: 2026-10-01
  version: 0.159.3
- claim: Newest stable release is rust-v0.160.0 (2026-10-01); 0.162.0-alpha.* are prereleases.
  id: upstream-releases
  limitations: 0.160.0 was not installed or inspected.
  location: https://api.github.com/repos/openai/codex/releases/latest
  method: official_docs
  observed_on: 2026-10-01
  version: unknown
homepage: https://developers.openai.com/codex/cli
repo: https://github.com/openai/codex
docs: https://developers.openai.com/codex/
cli_docs: https://developers.openai.com/codex/cli/reference
binaries:
- binary: codex
  notes: 'Confirmed locally: /Users/ken/.nvm/versions/node/v22.20.0/bin/codex is a Node launcher that runs the Mach-O arm64 binary in @openai/codex-darwin-arm64.'
  os: macos
- binary: codex
  notes: From install.sh and the npm package; release archives carry platform-named executables that the installer renames to codex. Not run on Linux here.
  os: linux
- alt_binaries:
  - codex.exe
  - codex.cmd
  binary: codex
  notes: install.ps1 installs under %LOCALAPPDATA%\Programs\OpenAI\Codex\bin; the npm install exposes a codex.cmd shim. Not run on Windows here. docs/install.md in the repository still lists Windows 11 via WSL2.
  os: windows
install_methods:
- command: curl -fsSL https://chatgpt.com/codex/install.sh | sh
  method: standalone_binary
  notes: Installs to $CODEX_INSTALL_DIR, default $HOME/.local/bin; rerun to upgrade. CODEX_NON_INTERACTIVE=1 suppresses installer prompts.
  os: macos
- command: curl -fsSL https://chatgpt.com/codex/install.sh | sh
  method: standalone_binary
  notes: Installs to $CODEX_INSTALL_DIR, default $HOME/.local/bin; rerun to upgrade. CODEX_NON_INTERACTIVE=1 suppresses installer prompts.
  os: linux
- command: powershell -ExecutionPolicy ByPass -c "irm https://chatgpt.com/codex/install.ps1 | iex"
  method: standalone_binary
  notes: Installs to %LOCALAPPDATA%\Programs\OpenAI\Codex\bin unless CODEX_INSTALL_DIR is set; read from install.ps1, not run.
  os: windows
- command: npm install -g @openai/codex
  method: npm
  notes: The npm package pulls a per-platform native package such as @openai/codex-darwin-arm64. This host uses it.
  os: macos
- command: npm install -g @openai/codex
  method: npm
  notes: The npm package pulls a per-platform native package such as @openai/codex-darwin-arm64.
  os: linux
- command: npm install -g @openai/codex
  method: npm
  notes: The npm package pulls a per-platform native package such as @openai/codex-darwin-arm64.
  os: windows
- command: brew install --cask codex
  method: brew
  notes: Homebrew cask, named in the binary's update logic alongside npm, bun, and pnpm; not run.
  os: macos
subcommands:
- description: Browse all agent sessions on the shared local app-server daemon.
  name: agents
  non_interactive: false
  notes: 'Not marked non-interactive: Opens an interactive terminal UI. Its switches are not inventoried.'
- description: Run Codex non-interactively.
  name: exec
  non_interactive: true
  notes: Alias e. Reads the prompt from the argument, from stdin when the prompt is omitted or "-", and appends piped stdin as a <stdin> block when both exist. Final message to stdout, progress to stderr; --json makes stdout JSONL.
- description: Resume a previous session by id or pick the most recent with --last.
  name: exec resume
  non_interactive: true
  notes: 'Continues a recorded session. With --last and no second operand the single operand is the prompt (source: ResumeArgsRaw). With neither id nor --last the prompt comes from stdin.'
- description: Fork a previous session by id into a new session.
  name: exec fork
  non_interactive: true
  notes: Forks a recorded session by id or thread name into a new thread; the id is required. An unknown id failed with "Session not found" before any network call.
- description: Run a code review against the current repository.
  name: exec review
  non_interactive: true
  notes: Reviews uncommitted changes, a base branch, or a commit; --uncommitted, --base, and --commit exclude each other and a custom PROMPT.
- description: Run a code review non-interactively.
  name: review
  non_interactive: true
  notes: 'Top-level review. Accepts far fewer switches than exec review: no --model, --json, --ephemeral, or sandbox switches.'
- description: Manage login.
  name: login
  non_interactive: false
  notes: 'Not marked non-interactive: The default flow opens a browser; --with-api-key and --with-access-token read a secret from stdin. Its switches are not inventoried.'
- description: Show login status.
  name: login status
  non_interactive: true
  notes: Prints "Not logged in" and exits 1 when no credentials exist.
- description: Remove stored authentication credentials.
  name: logout
  non_interactive: false
  notes: 'Not marked non-interactive: Changes stored credentials; completion without a person was not verified. Its switches are not inventoried.'
- description: Manage external MCP servers for Codex.
  name: mcp
  non_interactive: false
  notes: Group command that needs one of its subcommands; not itself a runnable path, so its switches are not inventoried.
- description: List configured MCP servers.
  name: mcp list
  non_interactive: true
  notes: Lists configured MCP servers; --json prints a JSON array.
- description: Show one configured MCP server.
  name: mcp get
  non_interactive: false
  notes: 'Not marked non-interactive: Needs a configured server; not run here. Its switches are not inventoried.'
- description: Add an MCP server over a URL or a stdio command.
  name: mcp add
  non_interactive: false
  notes: 'Not marked non-interactive: Writes config.toml. Its switches are not inventoried.'
- description: Remove a configured MCP server.
  name: mcp remove
  non_interactive: false
  notes: 'Not marked non-interactive: Writes config.toml. Its switches are not inventoried.'
- description: Authenticate to an MCP server with OAuth.
  name: mcp login
  non_interactive: false
  notes: 'Not marked non-interactive: Runs an OAuth flow that needs a browser. Its switches are not inventoried.'
- description: Remove stored OAuth credentials for an MCP server.
  name: mcp logout
  non_interactive: false
  notes: 'Not marked non-interactive: Changes stored credentials. Its switches are not inventoried.'
- description: Manage Codex plugins.
  name: plugin
  non_interactive: false
  notes: Group command that needs one of its subcommands; not itself a runnable path, so its switches are not inventoried.
- description: Install a plugin from a configured or remote marketplace.
  name: plugin add
  non_interactive: false
  notes: 'Not marked non-interactive: Needs network access and writes the plugin cache. Its switches are not inventoried.'
- description: List plugins available from configured and remote marketplaces.
  name: plugin list
  non_interactive: false
  notes: 'Not marked non-interactive: Needs network access to remote marketplaces. Its switches are not inventoried.'
- description: Add, list, upgrade, or remove configured plugin marketplaces.
  name: plugin marketplace
  non_interactive: false
  notes: Group command that needs one of its subcommands; not itself a runnable path, so its switches are not inventoried.
- description: Add a local or Git marketplace to the configured marketplace sources.
  name: plugin marketplace add
  non_interactive: false
  notes: 'Not marked non-interactive: Writes configuration and may clone a repository. Its switches are not inventoried.'
- description: List plugin marketplaces Codex is currently considering and their roots.
  name: plugin marketplace list
  non_interactive: false
  notes: 'Not marked non-interactive: Not run here. Its switches are not inventoried.'
- description: Refresh configured Git marketplace snapshots.
  name: plugin marketplace upgrade
  non_interactive: false
  notes: 'Not marked non-interactive: Needs network access and rewrites snapshots. Its switches are not inventoried.'
- description: Remove a configured marketplace source by name.
  name: plugin marketplace remove
  non_interactive: false
  notes: 'Not marked non-interactive: Writes configuration. Its switches are not inventoried.'
- description: Uninstall a plugin and remove its local cache.
  name: plugin remove
  non_interactive: false
  notes: 'Not marked non-interactive: Deletes the plugin cache. Its switches are not inventoried.'
- description: '[experimental] Run the app server or related tooling.'
  name: app-server
  non_interactive: false
  notes: 'Not marked non-interactive: Runs a long-lived server (stdio by default). Its switches are not inventoried.'
- description: Manage the local app-server daemon.
  name: app-server daemon
  non_interactive: false
  notes: Group command that needs one of its subcommands; not itself a runnable path, so its switches are not inventoried.
- description: Install durable local app-server management for SSH-driven use.
  name: app-server daemon bootstrap
  non_interactive: false
  notes: 'Not marked non-interactive: Changes the state of the background app-server daemon. Its switches are not inventoried.'
- description: Start the local app server daemon if it is not already running.
  name: app-server daemon start
  non_interactive: false
  notes: 'Not marked non-interactive: Changes the state of the background app-server daemon. Its switches are not inventoried.'
- description: Restart the local app server daemon.
  name: app-server daemon restart
  non_interactive: false
  notes: 'Not marked non-interactive: Changes the state of the background app-server daemon. Its switches are not inventoried.'
- description: Update the daemon package (may interrupt running work).
  name: app-server daemon update
  non_interactive: false
  notes: 'Not marked non-interactive: Changes the state of the background app-server daemon. Its switches are not inventoried.'
- description: Enable remote control for future starts and a currently running managed daemon.
  name: app-server daemon enable-remote-control
  non_interactive: false
  notes: 'Not marked non-interactive: Changes the state of the background app-server daemon. Its switches are not inventoried.'
- description: Disable remote control for future starts and a currently running managed daemon.
  name: app-server daemon disable-remote-control
  non_interactive: false
  notes: 'Not marked non-interactive: Changes the state of the background app-server daemon. Its switches are not inventoried.'
- description: Stop the local app server daemon.
  name: app-server daemon stop
  non_interactive: false
  notes: 'Not marked non-interactive: Changes the state of the background app-server daemon. Its switches are not inventoried.'
- description: Print local CLI and running app-server versions as JSON.
  name: app-server daemon version
  non_interactive: false
  notes: 'Not marked non-interactive: Needs the daemon; not run here. Its switches are not inventoried.'
- description: Proxy stdio bytes to the running app-server control socket.
  name: app-server proxy
  non_interactive: false
  notes: 'Not marked non-interactive: Relays stdio to a running server until closed. Its switches are not inventoried.'
- description: '[experimental] Generate TypeScript bindings for the app server protocol.'
  name: app-server generate-ts
  non_interactive: false
  notes: 'Not marked non-interactive: Writes files under --out; not run. Its switches are not inventoried.'
- description: '[experimental] Generate JSON Schema for the app server protocol.'
  name: app-server generate-json-schema
  non_interactive: false
  notes: 'Not marked non-interactive: Writes files under --out; not run. Its switches are not inventoried.'
- description: '[experimental] Manage the app-server daemon with remote control enabled.'
  name: remote-control
  non_interactive: false
  notes: 'Not marked non-interactive: Manages a background daemon. Its switches are not inventoried.'
- description: Start the app-server daemon with remote control enabled.
  name: remote-control start
  non_interactive: false
  notes: 'Not marked non-interactive: Starts a background daemon. Its switches are not inventoried.'
- description: Stop the app-server daemon.
  name: remote-control stop
  non_interactive: false
  notes: 'Not marked non-interactive: Stops a background daemon. Its switches are not inventoried.'
- description: Create and print a short-lived manual pairing code.
  name: remote-control pair
  non_interactive: false
  notes: 'Not marked non-interactive: Needs a running daemon and a person to use the pairing code. Its switches are not inventoried.'
- description: Launch the Desktop app (opens the app installer if missing).
  name: app
  non_interactive: false
  notes: 'Not marked non-interactive: Launches the desktop app or its installer. Its switches are not inventoried.'
- description: Generate shell completion scripts.
  name: completion
  non_interactive: true
  notes: Prints a completion script to stdout; the shell defaults to bash.
- description: Update Codex to the latest version.
  name: update
  non_interactive: false
  notes: 'Not marked non-interactive: Changes the installation and may invoke a package manager. Its switches are not inventoried.'
- description: Diagnose local Codex installation, config, auth, and runtime health.
  name: doctor
  non_interactive: true
  notes: Exits 1 when a check fails even though --json still prints a complete report.
- description: Run commands within a Codex-provided sandbox.
  name: sandbox
  non_interactive: false
  notes: 'Not marked non-interactive: Runs an arbitrary command under the platform sandbox; its own switches are a separate surface. Its switches are not inventoried.'
- description: Debugging tools.
  name: debug
  non_interactive: false
  notes: Group command that needs one of its subcommands; not itself a runnable path, so its switches are not inventoried.
- description: Render the raw model catalog as JSON.
  name: debug models
  non_interactive: true
  notes: Prints the model catalog as JSON; --bundled skips the refresh.
- description: 'Tooling: helps debug the app server.'
  name: debug app-server
  non_interactive: false
  notes: Group command that needs one of its subcommands; not itself a runnable path, so its switches are not inventoried.
- description: Send one message through the app-server test client.
  name: debug app-server send-message-v2
  non_interactive: false
  notes: 'Not marked non-interactive: Sends a message through the app-server test client and starts a session. Its switches are not inventoried.'
- description: Render the model-visible prompt input list as JSON.
  name: debug prompt-input
  non_interactive: true
  notes: Renders the prompt items a session would send, as JSON, without contacting a model.
- description: Apply the latest diff produced by Codex agent as a `git apply` to your local working tree.
  name: apply
  non_interactive: false
  notes: 'Not marked non-interactive: Needs a Codex Cloud account and mutates the working tree. Its switches are not inventoried.'
- description: Resume a previous interactive session (picker by default; use --last to continue the most recent).
  name: resume
  non_interactive: false
  notes: 'Not marked non-interactive: Opens a session picker and the interactive terminal UI. Its switches are not inventoried.'
- description: Queue a message for an existing session.
  name: queue
  non_interactive: false
  notes: 'Not marked non-interactive: Needs the shared app-server daemon; completion was not verified. Its switches are not inventoried.'
- description: Archive a saved session by id or session name.
  name: archive
  non_interactive: false
  notes: 'Not marked non-interactive: Changes saved session state; completion without a person was not verified. Its switches are not inventoried.'
- description: Permanently delete a saved session by id or session name.
  name: delete
  non_interactive: false
  notes: 'Not marked non-interactive: Prompts for confirmation unless --force is given with a UUID. Its switches are not inventoried.'
- description: Inspect or migrate legacy local sessions to paginated thread history.
  name: migrate-rollouts
  non_interactive: true
  notes: Reports eligible legacy sessions; changes nothing unless --apply is given.
- description: Unarchive a saved session by id or session name.
  name: unarchive
  non_interactive: false
  notes: 'Not marked non-interactive: Changes saved session state; completion without a person was not verified. Its switches are not inventoried.'
- description: Fork a previous interactive session (picker by default; use --last to fork the most recent).
  name: fork
  non_interactive: false
  notes: 'Not marked non-interactive: Opens a session picker and the interactive terminal UI. Its switches are not inventoried.'
- description: '[EXPERIMENTAL] Browse tasks from Codex Cloud and apply changes locally.'
  name: cloud
  non_interactive: false
  notes: 'Not marked non-interactive: Opens a terminal UI when no subcommand is given. Its switches are not inventoried.'
- description: Submit a new Codex Cloud task without launching the TUI.
  name: cloud exec
  non_interactive: false
  notes: 'Not marked non-interactive: Needs a Codex Cloud account; not run here. Its switches are not inventoried.'
- description: Show the status of a Codex Cloud task.
  name: cloud status
  non_interactive: false
  notes: 'Not marked non-interactive: Needs a Codex Cloud account; not run here. Its switches are not inventoried.'
- description: List Codex Cloud tasks.
  name: cloud list
  non_interactive: false
  notes: 'Not marked non-interactive: Needs a Codex Cloud account; not run here. Its switches are not inventoried.'
- description: Apply the diff for a Codex Cloud task locally.
  name: cloud apply
  non_interactive: false
  notes: 'Not marked non-interactive: Needs a Codex Cloud account and mutates the working tree. Its switches are not inventoried.'
- description: Show the unified diff for a Codex Cloud task.
  name: cloud diff
  non_interactive: false
  notes: 'Not marked non-interactive: Needs a Codex Cloud account; not run here. Its switches are not inventoried.'
- description: '[EXPERIMENTAL] Run the standalone exec-server service.'
  name: exec-server
  non_interactive: false
  notes: 'Not marked non-interactive: Runs a long-lived server. Its switches are not inventoried.'
- description: Register an existing WebSocket exec-server as a remote environment.
  name: exec-server forward
  non_interactive: false
  notes: 'Not marked non-interactive: Runs a long-lived forwarding process. Its switches are not inventoried.'
- description: Inspect feature flags.
  name: features
  non_interactive: false
  notes: Group command that needs one of its subcommands; not itself a runnable path, so its switches are not inventoried.
- description: List known features with their stage and effective state.
  name: features list
  non_interactive: true
  notes: Prints one row per feature with its stage and effective state; plain text.
- description: Enable a feature in config.toml.
  name: features enable
  non_interactive: false
  notes: 'Not marked non-interactive: Writes config.toml; not run so the user configuration stayed unchanged. Its switches are not inventoried.'
- description: Disable a feature in config.toml.
  name: features disable
  non_interactive: false
  notes: 'Not marked non-interactive: Writes config.toml; not run so the user configuration stayed unchanged. Its switches are not inventoried.'
cli_switches:
- aliases:
  - -c
  attachment:
  - space
  - equals
  - short_attached
  description: Override one configuration value for this run; the value is parsed as TOML and falls back to a literal string.
  evidence_ids:
  - src-config-override
  - test-attachment-forms
  - test-valued-probes
  example: codex exec -c model="o3" "summarize this repo"
  flag: --config
  invocation_scope:
  - applies_to: global
  notes: Repeatable, one key=value per occurrence. Split at the first "=". A value without "=" is rejected after parsing with "Invalid override (missing '=')" and exit 1. Quote values containing TOML strings so the shell keeps the inner quotes.
  scope:
  - config
  value: <key=value>
  value_optional: false
  value_type: string
- attachment:
  - space
  - equals
  description: Enable a named feature for this run; equivalent to -c features.<name>=true.
  evidence_ids:
  - src-cli-main
  - test-attachment-forms
  - test-valued-probes
  example: codex exec --enable web_search "check the docs"
  flag: --enable
  invocation_scope:
  - applies_to: global
  notes: Repeatable. An unknown feature name is rejected at startup ("Unknown feature flag") with exit 1, not at parse time.
  scope:
  - config
  value: <FEATURE>
  value_optional: false
  value_type: string
- attachment:
  - space
  - equals
  description: Disable a named feature for this run; equivalent to -c features.<name>=false.
  evidence_ids:
  - src-cli-main
  - test-attachment-forms
  - test-valued-probes
  example: codex exec --disable web_search "offline task"
  flag: --disable
  invocation_scope:
  - applies_to: global
  notes: Repeatable.
  scope:
  - config
  value: <FEATURE>
  value_optional: false
  value_type: string
- aliases:
  - -h
  attachment: []
  description: Print help.
  evidence_ids:
  - local-help
  example: codex exec --help
  flag: --help
  invocation_scope:
  - applies_to: global
  scope:
  - meta
  value_type: none
- aliases:
  - -V
  attachment: []
  description: Print the version.
  evidence_ids:
  - local-help
  - test-safe-runs
  example: codex --version
  flag: --version
  invocation_scope:
  - applies_to: command
    command: []
  - applies_to: command
    command:
    - exec
  notes: Root prints "codex-cli 0.159.3"; exec prints "codex-cli-exec 0.159.3". exec resume, exec fork, exec review, and review reject -V.
  scope:
  - meta
  value_type: none
- attachment: []
  description: Fail when config.toml holds fields this version does not recognize.
  evidence_ids:
  - src-exec-cli
  - src-tui-cli
  - src-cli-main
  - test-valued-probes
  example: codex exec --strict-config "run the task"
  flag: --strict-config
  invocation_scope:
  - applies_to: command
    command: []
  - applies_to: command
    command:
    - exec
  - applies_to: command
    command:
    - exec
    - resume
  - applies_to: command
    command:
    - exec
    - fork
  - applies_to: command
    command:
    - exec
    - review
  - applies_to: command
    command:
    - review
  scope:
  - config
  value_type: none
- aliases:
  - -m
  attachment:
  - space
  - equals
  - short_attached
  description: Model the agent should use.
  evidence_ids:
  - src-shared-options
  - src-exec-cli
  - test-valued-probes
  example: codex exec -m gpt-6.1-sol "explain main.rs"
  flag: --model
  invocation_scope:
  - applies_to: command
    command: []
  - applies_to: command
    command:
    - exec
  - applies_to: command
    command:
    - exec
    - resume
  - applies_to: command
    command:
    - exec
    - fork
  - applies_to: command
    command:
    - exec
    - review
  notes: Free text at parse time; an unknown model fails later at the provider.
  scope:
  - model_selection
  value: <MODEL>
  value_optional: false
  value_type: string
- attachment: []
  description: Use the open-source (local) model provider.
  evidence_ids:
  - src-shared-options
  - test-valued-probes
  example: codex exec --oss "hello"
  flag: --oss
  invocation_scope:
  - applies_to: command
    command: []
  - applies_to: command
    command:
    - exec
  scope:
  - model_selection
  value_type: none
- attachment:
  - space
  - equals
  description: Pick the local provider (lmstudio or ollama) used with --oss.
  evidence_ids:
  - src-shared-options
  - test-valued-probes
  example: codex exec --oss --local-provider ollama "hello"
  flag: --local-provider
  invocation_scope:
  - applies_to: command
    command: []
  - applies_to: command
    command:
    - exec
  notes: Free text at parse time; help names lmstudio and ollama.
  scope:
  - model_selection
  value: <OSS_PROVIDER>
  value_optional: false
  value_type: string
- aliases:
  - -p
  attachment:
  - space
  - equals
  - short_attached
  description: Layer $CODEX_HOME/<name>.config.toml on top of the base user config.
  evidence_ids:
  - src-shared-options
  - test-attachment-forms
  - test-valued-probes
  example: codex exec -p work "run the task"
  flag: --profile
  invocation_scope:
  - applies_to: command
    command: []
  - applies_to: command
    command:
    - exec
  notes: Takes a plain name; a value such as Bad/Name is rejected at parse time with an error naming --profile.
  scope:
  - config
  value: <CONFIG_PROFILE_V2>
  value_optional: false
  value_type: string
- aliases:
  - -s
  attachment:
  - space
  - equals
  - short_attached
  description: Select the sandbox policy for model-generated shell commands.
  evidence_ids:
  - src-shared-options
  - test-attachment-forms
  - test-valued-probes
  example: codex exec -s read-only "audit this repo"
  flag: --sandbox
  invocation_scope:
  - applies_to: command
    command: []
  - applies_to: command
    command:
    - exec
  notes: 'Values: read-only, workspace-write, danger-full-access; anything else is a parse error (exit 2). Not accepted at exec resume, exec fork, or exec review; give it before the subcommand.'
  scope:
  - permissions
  value: <SANDBOX_MODE>
  value_optional: false
  value_type: string
- aliases:
  - --not-so-yolo
  attachment: []
  description: Route approval requests through automatic review inside the workspace-write sandbox.
  evidence_ids:
  - src-shared-options
  - test-aliases
  - test-valued-probes
  example: codex exec --approve-for-me "fix the bug"
  flag: --approve-for-me
  invocation_scope:
  - applies_to: command
    command: []
  - applies_to: command
    command:
    - exec
  notes: Conflicts with --sandbox and --dangerously-bypass-approvals-and-sandbox (parse error, exit 2). --not-so-yolo is hidden from help.
  scope:
  - permissions
  value_type: none
- aliases:
  - --yolo
  attachment: []
  description: Skip every approval prompt and run commands without a sandbox.
  evidence_ids:
  - src-shared-options
  - test-aliases
  - docs-developer-commands
  - test-valued-probes
  example: codex exec --yolo "run the migration"
  flag: --dangerously-bypass-approvals-and-sandbox
  invocation_scope:
  - applies_to: command
    command: []
  - applies_to: command
    command:
    - exec
  - applies_to: command
    command:
    - exec
    - resume
  - applies_to: command
    command:
    - exec
    - fork
  - applies_to: command
    command:
    - exec
    - review
  notes: Dangerous. Global within exec, so it is accepted after resume, fork, and review.
  scope:
  - permissions
  value_type: none
- attachment: []
  description: Run enabled hooks without persisted hook trust for this invocation.
  evidence_ids:
  - src-shared-options
  - test-valued-probes
  example: codex exec --dangerously-bypass-hook-trust "run"
  flag: --dangerously-bypass-hook-trust
  invocation_scope:
  - applies_to: command
    command: []
  - applies_to: command
    command:
    - exec
  - applies_to: command
    command:
    - exec
    - resume
  - applies_to: command
    command:
    - exec
    - fork
  - applies_to: command
    command:
    - exec
    - review
  notes: Dangerous; intended for automation that already vets hook sources.
  scope:
  - permissions
  value_type: none
- aliases:
  - -C
  attachment:
  - space
  - equals
  - short_attached
  description: Use the given directory as the agent working root.
  evidence_ids:
  - src-shared-options
  - test-valued-probes
  example: codex exec -C ./service "run tests"
  flag: --cd
  invocation_scope:
  - applies_to: command
    command: []
  - applies_to: command
    command:
    - exec
  notes: Not accepted at exec resume, exec fork, or exec review; change the process working directory instead or give it before the subcommand.
  scope:
  - workspace
  value: <DIR>
  value_optional: false
  value_type: string
- attachment: []
  description: Run the session in a new managed Git worktree.
  evidence_ids:
  - src-shared-options
  - src-exec-cli
  - test-valued-probes
  example: codex exec --worktree "try the refactor"
  flag: --worktree
  invocation_scope:
  - applies_to: command
    command: []
  - applies_to: command
    command:
    - exec
  - applies_to: command
    command:
    - exec
    - resume
  - applies_to: command
    command:
    - exec
    - fork
  - applies_to: command
    command:
    - exec
    - review
  scope:
  - workspace
  value_type: none
- attachment:
  - space
  - equals
  description: Add a directory that stays writable beside the primary workspace.
  evidence_ids:
  - src-shared-options
  - test-valued-probes
  example: codex exec --add-dir ../shared "update the shared code"
  flag: --add-dir
  invocation_scope:
  - applies_to: command
    command: []
  - applies_to: command
    command:
    - exec
  notes: Repeatable, one directory per occurrence; no comma splitting is declared.
  scope:
  - workspace
  value: <DIR>
  value_optional: false
  value_type: string
- aliases:
  - -i
  attachment:
  - space
  - equals
  - short_attached
  description: Attach image files to the initial prompt.
  evidence_ids:
  - src-shared-options
  - test-image-greedy
  - docs-developer-commands
  - test-valued-probes
  example: codex exec --image=shot.png "describe this screenshot"
  flag: --image
  invocation_scope:
  - applies_to: command
    command: []
  - applies_to: command
    command:
    - exec
  - applies_to: command
    command:
    - debug
    - prompt-input
  notes: 'Greedy in the space form: "-i a.png b.png" makes b.png an image, so a prompt written after the image list is lost and codex then waits for the prompt on stdin. Use --image=PATH, -iPATH, put the prompt first, or end the switches with "--". Comma-separated paths are also split. Repeatable.'
  scope:
  - input
  value: <FILE>
  value_type: variadic
  variadic_min: 1
- aliases:
  - -i
  attachment:
  - space
  - equals
  - short_attached
  description: Attach image files to the prompt sent after resuming or forking.
  evidence_ids:
  - src-exec-cli
  - test-image-greedy
  - docs-developer-commands
  - test-valued-probes
  example: codex exec resume --last -i shot.png "look at this"
  flag: --image
  invocation_scope:
  - applies_to: command
    command:
    - exec
    - resume
  - applies_to: command
    command:
    - exec
    - fork
  notes: One value per occurrence; later operands stay positional. A comma still splits one value into several paths, and the switch may be repeated.
  scope:
  - input
  value: <FILE>
  value_optional: false
  value_type: string
- aliases:
  - -a
  attachment:
  - space
  - equals
  - short_attached
  description: Choose when the model asks for approval before running a command.
  evidence_ids:
  - src-tui-cli
  - test-attachment-forms
  - test-valued-probes
  example: codex -a never "fix the bug"
  flag: --ask-for-approval
  invocation_scope:
  - applies_to: command
    command: []
  notes: 'Help lists on-request and never; anything else is a parse error. Root only: exec has no approval switch, so non-interactive runs set approval_policy with -c.'
  scope:
  - permissions
  value: <APPROVAL_POLICY>
  value_optional: false
  value_type: string
- attachment: []
  description: Enable the live web_search tool.
  evidence_ids:
  - src-tui-cli
  - test-valued-probes
  example: codex --search "latest release notes"
  flag: --search
  invocation_scope:
  - applies_to: command
    command: []
  scope:
  - tools
  value_type: none
- attachment: []
  description: Run the terminal UI inline and keep scrollback.
  evidence_ids:
  - src-tui-cli
  - test-valued-probes
  example: codex --no-alt-screen
  flag: --no-alt-screen
  invocation_scope:
  - applies_to: command
    command: []
  scope:
  - ui
  value_type: none
- attachment: []
  description: Run without the shared background app server.
  evidence_ids:
  - src-tui-cli
  - test-valued-probes
  example: codex --no-daemon
  flag: --no-daemon
  invocation_scope:
  - applies_to: command
    command: []
  scope:
  - runtime
  value_type: none
- attachment:
  - space
  - equals
  description: Connect the terminal UI to a remote app-server endpoint.
  evidence_ids:
  - src-cli-main
  - test-valued-probes
  example: codex --remote ws://127.0.0.1:4500
  flag: --remote
  invocation_scope:
  - applies_to: command
    command: []
  notes: Help names ws://host:port, wss://host:port, unix://, and unix://PATH.
  scope:
  - runtime
  value: <ADDR>
  value_optional: false
  value_type: string
- attachment:
  - space
  - equals
  description: Name the environment variable holding the bearer token for a remote websocket.
  evidence_ids:
  - src-cli-main
  - test-valued-probes
  example: codex --remote wss://host:4500 --remote-auth-token-env REMOTE_TOKEN
  flag: --remote-auth-token-env
  invocation_scope:
  - applies_to: command
    command: []
  notes: Takes the variable name, never the token itself.
  scope:
  - runtime
  value: <ENV_VAR>
  value_optional: false
  value_type: string
- attachment:
  - space
  - equals
  description: Source classification recorded on new or forked threads.
  evidence_ids:
  - src-exec-cli
  - test-valued-probes
  example: codex exec --thread-source automation "run"
  flag: --thread-source
  invocation_scope:
  - applies_to: command
    command:
    - exec
  - applies_to: command
    command:
    - exec
    - resume
  - applies_to: command
    command:
    - exec
    - fork
  - applies_to: command
    command:
    - exec
    - review
  notes: A free-form value passed parsing in the test (bogus was not rejected); the accepted set was not established from source.
  scope:
  - session
  value: <SOURCE>
  value_optional: false
  value_type: string
- attachment: []
  description: Allow running outside a Git repository.
  evidence_ids:
  - src-exec-cli
  - test-valued-probes
  example: codex exec --skip-git-repo-check "hello"
  flag: --skip-git-repo-check
  invocation_scope:
  - applies_to: command
    command:
    - exec
  - applies_to: command
    command:
    - exec
    - resume
  - applies_to: command
    command:
    - exec
    - fork
  - applies_to: command
    command:
    - exec
    - review
  notes: Without it a run outside a trusted Git directory stops with "Not inside a trusted directory" and exit 1.
  scope:
  - workspace
  value_type: none
- attachment: []
  description: Do not persist session files to disk.
  evidence_ids:
  - src-exec-cli
  - test-valued-probes
  example: codex exec --ephemeral "one-off question"
  flag: --ephemeral
  invocation_scope:
  - applies_to: command
    command:
    - exec
  - applies_to: command
    command:
    - exec
    - resume
  - applies_to: command
    command:
    - exec
    - fork
  - applies_to: command
    command:
    - exec
    - review
  scope:
  - session
  value_type: none
- attachment: []
  description: Do not load $CODEX_HOME/config.toml; authentication still uses CODEX_HOME.
  evidence_ids:
  - src-exec-cli
  - test-valued-probes
  example: codex exec --ignore-user-config "hello"
  flag: --ignore-user-config
  invocation_scope:
  - applies_to: command
    command:
    - exec
  - applies_to: command
    command:
    - exec
    - resume
  - applies_to: command
    command:
    - exec
    - fork
  - applies_to: command
    command:
    - exec
    - review
  scope:
  - config
  value_type: none
- attachment: []
  description: Do not load user or project execpolicy .rules files.
  evidence_ids:
  - src-exec-cli
  - test-valued-probes
  example: codex exec --ignore-rules "hello"
  flag: --ignore-rules
  invocation_scope:
  - applies_to: command
    command:
    - exec
  - applies_to: command
    command:
    - exec
    - resume
  - applies_to: command
    command:
    - exec
    - fork
  - applies_to: command
    command:
    - exec
    - review
  scope:
  - permissions
  value_type: none
- attachment:
  - space
  - equals
  description: Path to a JSON Schema describing the final response shape.
  evidence_ids:
  - src-exec-cli
  - test-valued-probes
  example: codex exec --output-schema schema.json "extract the fields"
  flag: --output-schema
  invocation_scope:
  - applies_to: command
    command:
    - exec
  - applies_to: command
    command:
    - exec
    - resume
  - applies_to: command
    command:
    - exec
    - fork
  - applies_to: command
    command:
    - exec
    - review
  scope:
  - output
  value: <FILE>
  value_optional: false
  value_type: string
- attachment:
  - space
  - equals
  default: auto
  description: Choose color for output.
  evidence_ids:
  - src-exec-cli
  - test-attachment-forms
  - test-valued-probes
  example: codex exec --color never "hello"
  flag: --color
  invocation_scope:
  - applies_to: command
    command:
    - exec
  notes: 'Values: always, never, auto. Accepted at exec only, not at exec resume, exec fork, or exec review.'
  scope:
  - output
  value: <COLOR>
  value_optional: false
  value_type: string
- aliases:
  - --experimental-json
  attachment: []
  description: Print events to stdout as JSON Lines.
  evidence_ids:
  - src-exec-cli
  - test-aliases
  - docs-non-interactive
  - test-valued-probes
  example: codex exec --json "hello"
  flag: --json
  invocation_scope:
  - applies_to: command
    command:
    - exec
  - applies_to: command
    command:
    - exec
    - resume
  - applies_to: command
    command:
    - exec
    - fork
  - applies_to: command
    command:
    - exec
    - review
  notes: Without it stdout carries only the final agent message and progress goes to stderr.
  scope:
  - output
  value_type: none
- attachment: []
  description: Print a machine-readable report.
  evidence_ids:
  - local-help
  - test-safe-runs
  - test-valued-probes
  example: codex doctor --json
  flag: --json
  invocation_scope:
  - applies_to: command
    command:
    - doctor
  - applies_to: command
    command:
    - mcp
    - list
  - applies_to: command
    command:
    - migrate-rollouts
  notes: 'doctor: redacted report; mcp list: JSON array; migrate-rollouts: complete per-thread report.'
  scope:
  - output
  value_type: none
- aliases:
  - -o
  attachment:
  - space
  - equals
  - short_attached
  description: Write the agent last message to a file.
  evidence_ids:
  - src-exec-cli
  - test-valued-probes
  example: codex exec -o answer.txt "summarize"
  flag: --output-last-message
  invocation_scope:
  - applies_to: command
    command:
    - exec
  - applies_to: command
    command:
    - exec
    - resume
  - applies_to: command
    command:
    - exec
    - fork
  - applies_to: command
    command:
    - exec
    - review
  scope:
  - output
  value: <FILE>
  value_optional: false
  value_type: string
- attachment: []
  description: Resume the most recent recorded session without naming an id.
  evidence_ids:
  - src-exec-cli
  - test-valued-probes
  example: codex exec resume --last "continue"
  flag: --last
  invocation_scope:
  - applies_to: command
    command:
    - exec
    - resume
  notes: With --last and a single operand, the operand is the prompt, not a session id.
  scope:
  - session
  value_type: none
- attachment: []
  description: Show sessions from every directory instead of filtering by the working directory.
  evidence_ids:
  - src-exec-cli
  - test-valued-probes
  example: codex exec resume --all --last "continue"
  flag: --all
  invocation_scope:
  - applies_to: command
    command:
    - exec
    - resume
  scope:
  - session
  value_type: none
- attachment: []
  description: Expand long lists in detailed human output.
  evidence_ids:
  - local-help
  - test-valued-probes
  example: codex doctor --all
  flag: --all
  invocation_scope:
  - applies_to: command
    command:
    - doctor
  scope:
  - output
  value_type: none
- attachment: []
  description: Review staged, unstaged, and untracked changes.
  evidence_ids:
  - src-exec-cli
  - test-valued-probes
  example: codex review --uncommitted
  flag: --uncommitted
  invocation_scope:
  - applies_to: command
    command:
    - exec
    - review
  - applies_to: command
    command:
    - review
  notes: Conflicts with --base, --commit, and a PROMPT (parse error, exit 2).
  scope:
  - review
  value_type: none
- attachment:
  - space
  - equals
  description: Review changes against a base branch.
  evidence_ids:
  - src-exec-cli
  - test-valued-probes
  example: codex review --base main
  flag: --base
  invocation_scope:
  - applies_to: command
    command:
    - exec
    - review
  - applies_to: command
    command:
    - review
  notes: Conflicts with --uncommitted, --commit, and a PROMPT.
  scope:
  - review
  value: <BRANCH>
  value_optional: false
  value_type: string
- attachment:
  - space
  - equals
  description: Review the changes a commit introduced.
  evidence_ids:
  - src-exec-cli
  - test-valued-probes
  example: codex review --commit HEAD
  flag: --commit
  invocation_scope:
  - applies_to: command
    command:
    - exec
    - review
  - applies_to: command
    command:
    - review
  notes: Conflicts with --uncommitted, --base, and a PROMPT.
  scope:
  - review
  value: <SHA>
  value_optional: false
  value_type: string
- attachment:
  - space
  - equals
  description: Commit title shown in the review summary.
  evidence_ids:
  - src-exec-cli
  - test-valued-probes
  example: codex review --commit HEAD --title "Fix parser"
  flag: --title
  invocation_scope:
  - applies_to: command
    command:
    - exec
    - review
  - applies_to: command
    command:
    - review
  notes: Requires --commit.
  scope:
  - review
  value: <TITLE>
  value_optional: false
  value_type: string
- attachment: []
  description: Skip the refresh and dump only the bundled model catalog.
  evidence_ids:
  - src-cli-main
  - test-safe-runs
  - test-valued-probes
  example: codex debug models --bundled
  flag: --bundled
  invocation_scope:
  - applies_to: command
    command:
    - debug
    - models
  scope:
  - introspection
  value_type: none
- attachment: []
  description: Show only grouped check rows and the final count.
  evidence_ids:
  - local-help
  - test-valued-probes
  example: codex doctor --summary
  flag: --summary
  invocation_scope:
  - applies_to: command
    command:
    - doctor
  scope:
  - output
  value_type: none
- attachment: []
  description: Disable ANSI color in human output.
  evidence_ids:
  - local-help
  - test-valued-probes
  example: codex doctor --no-color
  flag: --no-color
  invocation_scope:
  - applies_to: command
    command:
    - doctor
  scope:
  - output
  value_type: none
- attachment: []
  description: Use ASCII status labels and separators in human output.
  evidence_ids:
  - local-help
  - test-valued-probes
  example: codex doctor --ascii
  flag: --ascii
  invocation_scope:
  - applies_to: command
    command:
    - doctor
  scope:
  - output
  value_type: none
- attachment: []
  description: Publish the migration; without it the command only reports.
  evidence_ids:
  - local-help
  - test-valued-probes
  example: codex migrate-rollouts --apply
  flag: --apply
  invocation_scope:
  - applies_to: command
    command:
    - migrate-rollouts
  scope:
  - session
  value_type: none
- attachment:
  - space
  - equals
  description: Restrict inspection or migration to a thread id.
  evidence_ids:
  - local-help
  - test-number-and-uuid
  - test-valued-probes
  example: codex migrate-rollouts --thread 0123456789abcdef0123456789abcdef
  flag: --thread
  invocation_scope:
  - applies_to: command
    command:
    - migrate-rollouts
  notes: Must be a UUID; a malformed value is rejected at parse time. Help says "one or more"; repetition was not established.
  scope:
  - session
  value: <THREAD_ID>
  value_optional: false
  value_type: string
- attachment:
  - space
  - equals
  description: Limit aggregate rollout read and write throughput, in MiB per second.
  evidence_ids:
  - local-help
  - test-number-and-uuid
  - test-valued-probes
  example: codex migrate-rollouts --max-mib-per-second 20
  flag: --max-mib-per-second
  invocation_scope:
  - applies_to: command
    command:
    - migrate-rollouts
  notes: Unsigned integer; fractions and negative numbers are rejected.
  scope:
  - session
  value: <MIB>
  value_optional: false
  value_type: number
- attachment: []
  description: Print one line for each inspected rollout.
  evidence_ids:
  - local-help
  - test-valued-probes
  example: codex migrate-rollouts --verbose
  flag: --verbose
  invocation_scope:
  - applies_to: command
    command:
    - migrate-rollouts
  scope:
  - output
  value_type: none
config_paths:
- format: toml
  notes: User configuration; the CLI also edits it for features enable/disable and mcp add/remove. CODEX_HOME replaces the directory.
  os: macos
  path: ~/.codex/config.toml
  scope: user
- format: other
  notes: Directory override; holds auth.json, sessions, skills, rules, and SQLite state. A throwaway value starts with no login and creates skills/.system on first run.
  os: macos
  path: $CODEX_HOME
  scope: env
- format: toml
  notes: Profile layered over the base config by --profile <name>; the name must be a plain name.
  os: macos
  path: ~/.codex/<name>.config.toml
  scope: user
- format: toml
  notes: Project layer, read from the project root down to the working directory with the closest winning; trusted projects only.
  os: macos
  path: .codex/config.toml
  scope: repo
- format: toml
  notes: System layer below user config; documented as Unix only. Absent on this host.
  os: macos
  path: /etc/codex/config.toml
  scope: system
- format: toml
  notes: User configuration; the CLI also edits it for features enable/disable and mcp add/remove. CODEX_HOME replaces the directory.
  os: linux
  path: ~/.codex/config.toml
  scope: user
- format: other
  notes: Directory override; holds auth.json, sessions, skills, rules, and SQLite state. A throwaway value starts with no login and creates skills/.system on first run.
  os: linux
  path: $CODEX_HOME
  scope: env
- format: toml
  notes: Profile layered over the base config by --profile <name>; the name must be a plain name.
  os: linux
  path: ~/.codex/<name>.config.toml
  scope: user
- format: toml
  notes: Project layer, read from the project root down to the working directory with the closest winning; trusted projects only.
  os: linux
  path: .codex/config.toml
  scope: repo
- format: toml
  notes: System layer below user config; documented as Unix only. Absent on this host.
  os: linux
  path: /etc/codex/config.toml
  scope: system
- format: toml
  notes: User configuration; the CLI also edits it for features enable/disable and mcp add/remove. CODEX_HOME replaces the directory. Windows location follows the documented ~ mapping and was not inspected.
  os: windows
  path: '%USERPROFILE%\.codex\config.toml'
  scope: user
- format: other
  notes: Directory override; holds auth.json, sessions, skills, rules, and SQLite state. A throwaway value starts with no login and creates skills/.system on first run.
  os: windows
  path: '%CODEX_HOME%'
  scope: env
- format: toml
  notes: Profile layered over the base config by --profile <name>; the name must be a plain name.
  os: windows
  path: '%USERPROFILE%\.codex\<name>.config.toml'
  scope: user
- format: toml
  notes: Project layer, read from the project root down to the working directory with the closest winning; trusted projects only.
  os: windows
  path: .codex/config.toml
  scope: repo
- format: json
  notes: Stored credentials. Present locally; its contents were not read.
  os: macos
  path: ~/.codex/auth.json
  scope: user
- format: json
  notes: Stored credentials; same layout as macOS, not inspected on Linux.
  os: linux
  path: ~/.codex/auth.json
  scope: user
- format: json
  notes: Stored credentials; same layout as macOS, not inspected on Windows.
  os: windows
  path: '%USERPROFILE%\.codex\auth.json'
  scope: user
env_vars:
- effect: Replaces ~/.codex as the home for configuration, credentials, sessions, skills, rules, and state; also where profile files and the standalone install live.
  name: CODEX_HOME
- effect: Moves the SQLite state databases; when unset they follow CODEX_HOME or the configured sqlite_home.
  name: CODEX_SQLITE_HOME
- effect: Supplies an API key for one run of exec or review without a stored login (documented for exec, review, and exec-server --remote).
  name: CODEX_API_KEY
- effect: Named in the binary and in the help for codex login --with-access-token as the source of an access token; whether exec reads it directly was not established.
  name: CODEX_ACCESS_TOKEN
- effect: Read as an API-key credential when no stored login or CODEX_API_KEY applies; precedence against CODEX_API_KEY was not established.
  name: OPENAI_API_KEY
- effect: Path to a CA bundle trusted for Codex HTTPS and websocket traffic; SSL_CERT_FILE is the fallback the binary also names.
  name: CODEX_CA_CERTIFICATE
- effect: Standard tracing filter for the Rust logging stack, documented in docs/install.md.
  name: RUST_LOG
- effect: Named in the binary next to the --no-color handling; its exact effect was not established.
  name: NO_COLOR
- effect: Named in the binary; by its name it turns off the terminal keyboard-enhancement protocol in the terminal UI. Not run.
  name: CODEX_TUI_DISABLE_KEYBOARD_ENHANCEMENT
- effect: Present in the binary beside the exec-command path, which suggests Codex passes the thread id to commands it runs; not confirmed by a run.
  name: CODEX_THREAD_ID
- effect: Present in the binary with the value 1, which suggests it marks commands whose sandbox disables network access; not confirmed by a run.
  name: CODEX_SANDBOX_NETWORK_DISABLED
- effect: 'Installer-only: directory the install scripts put the codex executable in.'
  name: CODEX_INSTALL_DIR
- effect: 'Installer and updater only: set to 1 to run the install script without prompting.'
  name: CODEX_NON_INTERACTIVE
machine_introspection:
- command: codex --version
  machine_readable: false
  notes: Prints "codex-cli 0.159.3".
  output_format: text
  purpose: version
  useful_for_codegen: false
- command: codex doctor --json
  machine_readable: true
  notes: Redacted report with schemaVersion, overallStatus, codexVersion, and checks keyed by id. Exits 1 when overallStatus is fail while still printing the full report.
  output_format: json
  purpose: doctor
  useful_for_codegen: false
- command: codex debug models
  machine_readable: true
  notes: 'Raw model catalog (about 650 KB bundled): slug, display_name, default_reasoning_level, supported_reasoning_levels. Without --bundled it attempts a refresh, so it may use the network.'
  output_format: json
  purpose: models
  useful_for_codegen: true
- command: codex debug models --bundled
  machine_readable: true
  notes: Offline; the catalog compiled into this binary.
  output_format: json
  purpose: models
  useful_for_codegen: true
- command: codex debug prompt-input [PROMPT]
  machine_readable: true
  notes: JSON array of the prompt items a session would send, including developer instructions and the skills list. Contacts no model.
  output_format: json
  purpose: other
  useful_for_codegen: false
- command: codex features list
  machine_readable: false
  notes: 'Whitespace-aligned columns: feature name, stage (for example "under development", "experimental", "removed"), effective state. No JSON mode.'
  output_format: table
  purpose: capabilities
  useful_for_codegen: false
- command: codex mcp list --json
  machine_readable: true
  notes: JSON array of configured servers; [] when none.
  output_format: json
  purpose: mcp
  useful_for_codegen: false
- command: codex exec --json "<prompt>"
  machine_readable: true
  notes: Streams thread.started, turn.*, item.*, and error events per official docs; runs a billed model session.
  output_format: jsonl
  purpose: other
  useful_for_codegen: false
- command: codex migrate-rollouts --json
  machine_readable: true
  notes: Complete per-thread report; read-only without --apply.
  output_format: json
  purpose: other
  useful_for_codegen: false
- command: codex login status
  machine_readable: false
  notes: Prints "Not logged in" and exits 1 without credentials; exit status is the machine-readable part.
  output_format: text
  purpose: other
  useful_for_codegen: false
- command: codex completion zsh
  machine_readable: false
  notes: About 240 KB zsh completion script; also bash, elvish, fish, and powershell.
  output_format: text
  purpose: help
  useful_for_codegen: false
- command: codex app-server generate-json-schema --out <DIR>
  machine_readable: true
  notes: Writes the app-server protocol schema into the directory; not run here.
  output_format: unknown
  purpose: config_schema
  useful_for_codegen: false
wrapper_notes:
- Codex CLI 0.159.3 was installed and run on macOS arm64; Linux and Windows behavior comes from install scripts and documentation only.
- '--image/-i is greedy at the root, exec, and debug prompt-input: "codex exec -i a.png b.png" treats b.png as a second image and then waits on stdin for a prompt. Write --image=PATH or -iPATH, put the prompt before the switch, or end the switches with "--". At exec resume and exec fork the space form takes one value.'
- Codex reads the prompt from stdin when the argument is omitted or is "-", and when both exist appends stdin as a <stdin> block. Give it a closed stdin or a real prompt; with nothing available it prints "No prompt provided via stdin." and exits 1.
- Non-global exec switches (--sandbox, --cd, --color, --oss, --profile, --add-dir) are not accepted after resume, fork, or review. Place them before the subcommand; the source merges the exec-level values into the subcommand. Only --model, --dangerously-bypass-approvals-and-sandbox, --dangerously-bypass-hook-trust, --worktree, and the exec globals (--json, --output-schema, --output-last-message, --ephemeral, --skip-git-repo-check, --ignore-user-config, --ignore-rules, --thread-source, --strict-config) apply after it. Source-derived, not run, because a run would be billed.
- 'exec has no --ask-for-approval; the interactive root has. A non-interactive run reports "approval: never" and takes its policy from configuration, so pass -c approval_policy=... when a different policy is needed.'
- An unauthenticated exec still prints its banner (workdir, model, sandbox, session id) on stderr, retries a websocket, and exits 101 on 401. exec resume with an unknown session operand did not fail a lookup before the network step, while exec fork failed immediately with "Session not found"; check the id yourself before resuming.
- Parse errors exit 2 with a message naming the switch. Runtime errors for -c without "=" or an unknown --enable name exit 1. doctor --json and login status exit 1 on a failed check or missing login, so read the output rather than only the status.
- 'A first word that is not a subcommand is taken as the interactive PROMPT: "codex mcp-server --help" printed the root help rather than an error. The mcp-server command from earlier versions no longer exists. A typo therefore starts a terminal UI instead of failing.'
- -c values are TOML first and a literal string second, so -c model=o3 and -c model="o3" both work but -c key=[a,b] needs quoting for the shell. Root-level -c values are lower precedence than the subcommand's.
- --ephemeral and --ignore-user-config change what is read or written but authentication still comes from CODEX_HOME. Pointing CODEX_HOME at a fresh directory removes the login and creates a skills/.system folder there.
- 'Hidden commands exist outside help output: execpolicy, tcp-tunnel, responses-api-proxy, stdio-to-uds, and debug trace-reduce and clear-memories. --yolo, --not-so-yolo, and --experimental-json are hidden aliases; none are documented as stable.'
- The developers.openai.com/codex pages now answer 308 redirects to learn.chatgpt.com/docs; use the new locations.
changes:
- 'Rewritten for contract revision 2: the switch inventory is typed per command path, with value types and attachment forms established by clap source and 234 parser probes instead of help text.'
- Version moves from 0.142.5 to 0.159.3 installed; the newest stable upstream is 0.160.0, up from 0.142.5.
- '--image is two different switches: greedy and variadic at the root, exec, and debug prompt-input; one value per occurrence at exec resume and exec fork. The previous document did not distinguish them.'
- 'Commands added since the last version: agents, queue, migrate-rollouts, exec fork, and mcp list/get/add/remove/login/logout, plugin marketplace, and app-server daemon trees are enumerated. The mcp-server command and the visible execpolicy command are gone (execpolicy remains hidden).'
- 'New switches: --worktree, --thread-source, --approve-for-me (alias --not-so-yolo), --no-daemon, --remote, and --profile now layers $CODEX_HOME/<name>.config.toml.'
- Official documentation moved from developers.openai.com/codex to learn.chatgpt.com/docs through 308 redirects.
- The installer scripts now name releases.openai.com with a GitHub fallback, and CODEX_NON_INTERACTIVE and CODEX_INSTALL_DIR are installer-only variables.
requires_claudine_update: true
reason: The typed inventory is now available to claudine-gen. Claudine must treat -i/--image as greedy at the root and exec but single-valued at exec resume and exec fork, keep sandbox, cwd, and profile switches before the subcommand, and drop mcp-server and execpolicy from its model of the CLI.
contract_checked: 2026-10-01
---

# Codex CLI: Command-Line Surface

## Overview

Codex CLI is OpenAI's terminal coding agent. It is written in Rust, shipped as one executable named `codex`, and developed in the open at [openai/codex](https://github.com/openai/codex). Running `codex` starts an interactive terminal UI; `codex exec` and `codex review` run one task to completion without a terminal.

This document was verified against **codex-cli 0.159.3**, the version `codex --version` reported on macOS arm64 (npm-installed). The newest stable release upstream is **0.160.0**, published 2026-10-01; newer `0.162.0-alpha.*` tags are prereleases. The switch records come from the clap declarations at tag `rust-v0.159.3` and from 234 disposable parser probes, not from help text alone.

| Link | URL |
| ---- | --- |
| Product page | <https://developers.openai.com/codex/cli> |
| Documentation | <https://developers.openai.com/codex/> |
| Command reference | <https://developers.openai.com/codex/cli/reference> |
| Source | <https://github.com/openai/codex> |

The `developers.openai.com/codex` pages answer with a 308 redirect to `learn.chatgpt.com/docs`; the links above are the stable names and the sections below cite the redirected locations.

## Installation and Binaries

The command is `codex` on every operating system. On Windows the npm install also exposes `codex.cmd` and the standalone installer a `codex.exe`.

| OS | Method | Command | Notes |
| -- | ------ | ------- | ----- |
| macos | standalone_binary | `curl -fsSL https://chatgpt.com/codex/install.sh | sh` | Installs to $CODEX_INSTALL_DIR, default $HOME/.local/bin; rerun to upgrade. CODEX_NON_INTERACTIVE=1 suppresses installer prompts. |
| linux | standalone_binary | `curl -fsSL https://chatgpt.com/codex/install.sh | sh` | Installs to $CODEX_INSTALL_DIR, default $HOME/.local/bin; rerun to upgrade. CODEX_NON_INTERACTIVE=1 suppresses installer prompts. |
| windows | standalone_binary | `powershell -ExecutionPolicy ByPass -c "irm https://chatgpt.com/codex/install.ps1 | iex"` | Installs to %LOCALAPPDATA%\Programs\OpenAI\Codex\bin unless CODEX_INSTALL_DIR is set; read from install.ps1, not run. |
| macos | npm | `npm install -g @openai/codex` | The npm package pulls a per-platform native package such as @openai/codex-darwin-arm64. This host uses it. |
| linux | npm | `npm install -g @openai/codex` | The npm package pulls a per-platform native package such as @openai/codex-darwin-arm64. |
| windows | npm | `npm install -g @openai/codex` | The npm package pulls a per-platform native package such as @openai/codex-darwin-arm64. |
| macos | brew | `brew install --cask codex` | Homebrew cask, named in the binary's update logic alongside npm, bun, and pnpm; not run. |

## Subcommands

`codex` takes an optional `PROMPT` and, instead of one, a command path. Only paths that were run to completion here, or whose behavior the source and official documentation state, are marked non-interactive. Every other path is listed with the reason it was not, and its switches are not inventoried. Hidden internal commands (`execpolicy`, `tcp-tunnel`, `responses-api-proxy`, `stdio-to-uds`, `debug trace-reduce`, `clear-memories`) are omitted.

| Command path | What it does | Non-interactive |
| ------------ | ------------ | --------------- |
| `codex agents` | Browse all agent sessions on the shared local app-server daemon. | no |
| `codex exec` | Run Codex non-interactively. | yes |
| `codex exec resume` | Resume a previous session by id or pick the most recent with --last. | yes |
| `codex exec fork` | Fork a previous session by id into a new session. | yes |
| `codex exec review` | Run a code review against the current repository. | yes |
| `codex review` | Run a code review non-interactively. | yes |
| `codex login` | Manage login. | no |
| `codex login status` | Show login status. | yes |
| `codex logout` | Remove stored authentication credentials. | no |
| `codex mcp` | Manage external MCP servers for Codex. | no |
| `codex mcp list` | List configured MCP servers. | yes |
| `codex mcp get` | Show one configured MCP server. | no |
| `codex mcp add` | Add an MCP server over a URL or a stdio command. | no |
| `codex mcp remove` | Remove a configured MCP server. | no |
| `codex mcp login` | Authenticate to an MCP server with OAuth. | no |
| `codex mcp logout` | Remove stored OAuth credentials for an MCP server. | no |
| `codex plugin` | Manage Codex plugins. | no |
| `codex plugin add` | Install a plugin from a configured or remote marketplace. | no |
| `codex plugin list` | List plugins available from configured and remote marketplaces. | no |
| `codex plugin marketplace` | Add, list, upgrade, or remove configured plugin marketplaces. | no |
| `codex plugin marketplace add` | Add a local or Git marketplace to the configured marketplace sources. | no |
| `codex plugin marketplace list` | List plugin marketplaces Codex is currently considering and their roots. | no |
| `codex plugin marketplace upgrade` | Refresh configured Git marketplace snapshots. | no |
| `codex plugin marketplace remove` | Remove a configured marketplace source by name. | no |
| `codex plugin remove` | Uninstall a plugin and remove its local cache. | no |
| `codex app-server` | [experimental] Run the app server or related tooling. | no |
| `codex app-server daemon` | Manage the local app-server daemon. | no |
| `codex app-server daemon bootstrap` | Install durable local app-server management for SSH-driven use. | no |
| `codex app-server daemon start` | Start the local app server daemon if it is not already running. | no |
| `codex app-server daemon restart` | Restart the local app server daemon. | no |
| `codex app-server daemon update` | Update the daemon package (may interrupt running work). | no |
| `codex app-server daemon enable-remote-control` | Enable remote control for future starts and a currently running managed daemon. | no |
| `codex app-server daemon disable-remote-control` | Disable remote control for future starts and a currently running managed daemon. | no |
| `codex app-server daemon stop` | Stop the local app server daemon. | no |
| `codex app-server daemon version` | Print local CLI and running app-server versions as JSON. | no |
| `codex app-server proxy` | Proxy stdio bytes to the running app-server control socket. | no |
| `codex app-server generate-ts` | [experimental] Generate TypeScript bindings for the app server protocol. | no |
| `codex app-server generate-json-schema` | [experimental] Generate JSON Schema for the app server protocol. | no |
| `codex remote-control` | [experimental] Manage the app-server daemon with remote control enabled. | no |
| `codex remote-control start` | Start the app-server daemon with remote control enabled. | no |
| `codex remote-control stop` | Stop the app-server daemon. | no |
| `codex remote-control pair` | Create and print a short-lived manual pairing code. | no |
| `codex app` | Launch the Desktop app (opens the app installer if missing). | no |
| `codex completion` | Generate shell completion scripts. | yes |
| `codex update` | Update Codex to the latest version. | no |
| `codex doctor` | Diagnose local Codex installation, config, auth, and runtime health. | yes |
| `codex sandbox` | Run commands within a Codex-provided sandbox. | no |
| `codex debug` | Debugging tools. | no |
| `codex debug models` | Render the raw model catalog as JSON. | yes |
| `codex debug app-server` | Tooling: helps debug the app server. | no |
| `codex debug app-server send-message-v2` | Send one message through the app-server test client. | no |
| `codex debug prompt-input` | Render the model-visible prompt input list as JSON. | yes |
| `codex apply` | Apply the latest diff produced by Codex agent as a `git apply` to your local working tree. | no |
| `codex resume` | Resume a previous interactive session (picker by default; use --last to continue the most recent). | no |
| `codex queue` | Queue a message for an existing session. | no |
| `codex archive` | Archive a saved session by id or session name. | no |
| `codex delete` | Permanently delete a saved session by id or session name. | no |
| `codex migrate-rollouts` | Inspect or migrate legacy local sessions to paginated thread history. | yes |
| `codex unarchive` | Unarchive a saved session by id or session name. | no |
| `codex fork` | Fork a previous interactive session (picker by default; use --last to fork the most recent). | no |
| `codex cloud` | [EXPERIMENTAL] Browse tasks from Codex Cloud and apply changes locally. | no |
| `codex cloud exec` | Submit a new Codex Cloud task without launching the TUI. | no |
| `codex cloud status` | Show the status of a Codex Cloud task. | no |
| `codex cloud list` | List Codex Cloud tasks. | no |
| `codex cloud apply` | Apply the diff for a Codex Cloud task locally. | no |
| `codex cloud diff` | Show the unified diff for a Codex Cloud task. | no |
| `codex exec-server` | [EXPERIMENTAL] Run the standalone exec-server service. | no |
| `codex exec-server forward` | Register an existing WebSocket exec-server as a remote environment. | no |
| `codex features` | Inspect feature flags. | no |
| `codex features list` | List known features with their stage and effective state. | yes |
| `codex features enable` | Enable a feature in config.toml. | no |
| `codex features disable` | Disable a feature in config.toml. | no |

## CLI Switch Inventory

Inventoried paths: the root entrypoint `codex` and the non-interactive paths `exec`, `exec resume`, `exec fork`, `exec review`, `review`, `login status`, `mcp list`, `completion`, `doctor`, `debug models`, `debug prompt-input`, `migrate-rollouts`, `features list`. A value type, an attachment form, or an alias appears here only after one of these established it:

- **clap declarations** at tag `rust-v0.159.3` in `exec/src/cli.rs`, `utils/cli/src/shared_options.rs`, `tui/src/cli.rs`, `cli/src/main.rs`, and `utils/cli/src/config_override.rs`. Codex uses the clap derive parser.
- **Parser probes** under a throwaway `CODEX_HOME`: every valued switch given no value must fail with "a value is required for '--flag'", every valueless switch given `--flag=x` must fail with "unexpected value", and every valued switch must read `--flag=val` and `-Xval` as its value. 148 probes over 14 paths and 86 attachment probes all behaved as recorded.
- **Greedy-consumption tests** through `debug prompt-input`, which stops before any model call.

`-c/--config`, `--enable`, `--disable`, and `--help/-h` appear in the help of the root and all 72 command paths and are recorded as global. `--version/-V` is accepted only at the root and `exec`.

```mermaid
flowchart TD
    A["codex exec -i a.png b.png"] --> B{"--image form"}
    B -->|space form| C["a.png and b.png are both images; prompt lost"]
    B -->|"--image=a.png or -ia.png"| D["only a.png is an image; b.png stays the prompt"]
    B -->|"prompt first, or after --"| E["prompt is kept"]
```

| Switch | Value type | Attachment | Command paths | Evidence |
| ------ | ---------- | ---------- | ------------- | -------- |
| `--config`, `-c` | string | space, equals, short_attached | every path | src-config-override, test-attachment-forms, test-valued-probes |
| `--enable` | string | space, equals | every path | src-cli-main, test-attachment-forms, test-valued-probes |
| `--disable` | string | space, equals | every path | src-cli-main, test-attachment-forms, test-valued-probes |
| `--help`, `-h` | none | none | every path | local-help |
| `--version`, `-V` | none | none | `codex`, `codex exec` | local-help, test-safe-runs |
| `--strict-config` | none | none | `codex`, `codex exec`, `codex exec resume`, `codex exec fork`, `codex exec review`, `codex review` | src-exec-cli, src-tui-cli, src-cli-main, test-valued-probes |
| `--model`, `-m` | string | space, equals, short_attached | `codex`, `codex exec`, `codex exec resume`, `codex exec fork`, `codex exec review` | src-shared-options, src-exec-cli, test-valued-probes |
| `--oss` | none | none | `codex`, `codex exec` | src-shared-options, test-valued-probes |
| `--local-provider` | string | space, equals | `codex`, `codex exec` | src-shared-options, test-valued-probes |
| `--profile`, `-p` | string | space, equals, short_attached | `codex`, `codex exec` | src-shared-options, test-attachment-forms, test-valued-probes |
| `--sandbox`, `-s` | string | space, equals, short_attached | `codex`, `codex exec` | src-shared-options, test-attachment-forms, test-valued-probes |
| `--approve-for-me`, `--not-so-yolo` | none | none | `codex`, `codex exec` | src-shared-options, test-aliases, test-valued-probes |
| `--dangerously-bypass-approvals-and-sandbox`, `--yolo` | none | none | `codex`, `codex exec`, `codex exec resume`, `codex exec fork`, `codex exec review` | src-shared-options, test-aliases, docs-developer-commands, test-valued-probes |
| `--dangerously-bypass-hook-trust` | none | none | `codex`, `codex exec`, `codex exec resume`, `codex exec fork`, `codex exec review` | src-shared-options, test-valued-probes |
| `--cd`, `-C` | string | space, equals, short_attached | `codex`, `codex exec` | src-shared-options, test-valued-probes |
| `--worktree` | none | none | `codex`, `codex exec`, `codex exec resume`, `codex exec fork`, `codex exec review` | src-shared-options, src-exec-cli, test-valued-probes |
| `--add-dir` | string | space, equals | `codex`, `codex exec` | src-shared-options, test-valued-probes |
| `--image`, `-i` | variadic (min 1) | space, equals, short_attached | `codex`, `codex exec`, `codex debug prompt-input` | src-shared-options, test-image-greedy, docs-developer-commands, test-valued-probes |
| `--image`, `-i` | string | space, equals, short_attached | `codex exec resume`, `codex exec fork` | src-exec-cli, test-image-greedy, docs-developer-commands, test-valued-probes |
| `--ask-for-approval`, `-a` | string | space, equals, short_attached | `codex` | src-tui-cli, test-attachment-forms, test-valued-probes |
| `--search` | none | none | `codex` | src-tui-cli, test-valued-probes |
| `--no-alt-screen` | none | none | `codex` | src-tui-cli, test-valued-probes |
| `--no-daemon` | none | none | `codex` | src-tui-cli, test-valued-probes |
| `--remote` | string | space, equals | `codex` | src-cli-main, test-valued-probes |
| `--remote-auth-token-env` | string | space, equals | `codex` | src-cli-main, test-valued-probes |
| `--thread-source` | string | space, equals | `codex exec`, `codex exec resume`, `codex exec fork`, `codex exec review` | src-exec-cli, test-valued-probes |
| `--skip-git-repo-check` | none | none | `codex exec`, `codex exec resume`, `codex exec fork`, `codex exec review` | src-exec-cli, test-valued-probes |
| `--ephemeral` | none | none | `codex exec`, `codex exec resume`, `codex exec fork`, `codex exec review` | src-exec-cli, test-valued-probes |
| `--ignore-user-config` | none | none | `codex exec`, `codex exec resume`, `codex exec fork`, `codex exec review` | src-exec-cli, test-valued-probes |
| `--ignore-rules` | none | none | `codex exec`, `codex exec resume`, `codex exec fork`, `codex exec review` | src-exec-cli, test-valued-probes |
| `--output-schema` | string | space, equals | `codex exec`, `codex exec resume`, `codex exec fork`, `codex exec review` | src-exec-cli, test-valued-probes |
| `--color` | string | space, equals | `codex exec` | src-exec-cli, test-attachment-forms, test-valued-probes |
| `--json`, `--experimental-json` | none | none | `codex exec`, `codex exec resume`, `codex exec fork`, `codex exec review` | src-exec-cli, test-aliases, docs-non-interactive, test-valued-probes |
| `--json` | none | none | `codex doctor`, `codex mcp list`, `codex migrate-rollouts` | local-help, test-safe-runs, test-valued-probes |
| `--output-last-message`, `-o` | string | space, equals, short_attached | `codex exec`, `codex exec resume`, `codex exec fork`, `codex exec review` | src-exec-cli, test-valued-probes |
| `--last` | none | none | `codex exec resume` | src-exec-cli, test-valued-probes |
| `--all` | none | none | `codex exec resume` | src-exec-cli, test-valued-probes |
| `--all` | none | none | `codex doctor` | local-help, test-valued-probes |
| `--uncommitted` | none | none | `codex exec review`, `codex review` | src-exec-cli, test-valued-probes |
| `--base` | string | space, equals | `codex exec review`, `codex review` | src-exec-cli, test-valued-probes |
| `--commit` | string | space, equals | `codex exec review`, `codex review` | src-exec-cli, test-valued-probes |
| `--title` | string | space, equals | `codex exec review`, `codex review` | src-exec-cli, test-valued-probes |
| `--bundled` | none | none | `codex debug models` | src-cli-main, test-safe-runs, test-valued-probes |
| `--summary` | none | none | `codex doctor` | local-help, test-valued-probes |
| `--no-color` | none | none | `codex doctor` | local-help, test-valued-probes |
| `--ascii` | none | none | `codex doctor` | local-help, test-valued-probes |
| `--apply` | none | none | `codex migrate-rollouts` | local-help, test-valued-probes |
| `--thread` | string | space, equals | `codex migrate-rollouts` | local-help, test-number-and-uuid, test-valued-probes |
| `--max-mib-per-second` | number | space, equals | `codex migrate-rollouts` | local-help, test-number-and-uuid, test-valued-probes |
| `--verbose` | none | none | `codex migrate-rollouts` | local-help, test-valued-probes |

`value_optional` is false for every valued switch: each one rejected a missing value. No switch takes an optional value.

## Configuration Discovery

Configuration is TOML, merged in this order from highest to lowest: command-line flags and `-c` overrides, project `.codex/config.toml` (trusted projects only), profile files chosen with `--profile`, user `~/.codex/config.toml`, cloud-managed defaults, the Unix-only system file `/etc/codex/config.toml`, then built-in defaults. `CODEX_HOME` moves the whole user directory.

| OS | Scope | Path | Notes |
| -- | ----- | ---- | ----- |
| macos | user | `~/.codex/config.toml` | User configuration; the CLI also edits it for features enable/disable and mcp add/remove. CODEX_HOME replaces the directory. |
| macos | env | `$CODEX_HOME` | Directory override; holds auth.json, sessions, skills, rules, and SQLite state. A throwaway value starts with no login and creates skills/.system on first run. |
| macos | user | `~/.codex/<name>.config.toml` | Profile layered over the base config by --profile <name>; the name must be a plain name. |
| macos | repo | `.codex/config.toml` | Project layer, read from the project root down to the working directory with the closest winning; trusted projects only. |
| macos | system | `/etc/codex/config.toml` | System layer below user config; documented as Unix only. Absent on this host. |
| linux | user | `~/.codex/config.toml` | User configuration; the CLI also edits it for features enable/disable and mcp add/remove. CODEX_HOME replaces the directory. |
| linux | env | `$CODEX_HOME` | Directory override; holds auth.json, sessions, skills, rules, and SQLite state. A throwaway value starts with no login and creates skills/.system on first run. |
| linux | user | `~/.codex/<name>.config.toml` | Profile layered over the base config by --profile <name>; the name must be a plain name. |
| linux | repo | `.codex/config.toml` | Project layer, read from the project root down to the working directory with the closest winning; trusted projects only. |
| linux | system | `/etc/codex/config.toml` | System layer below user config; documented as Unix only. Absent on this host. |
| windows | user | `%USERPROFILE%\.codex\config.toml` | User configuration; the CLI also edits it for features enable/disable and mcp add/remove. CODEX_HOME replaces the directory. Windows location follows the documented ~ mapping and was not inspected. |
| windows | env | `%CODEX_HOME%` | Directory override; holds auth.json, sessions, skills, rules, and SQLite state. A throwaway value starts with no login and creates skills/.system on first run. |
| windows | user | `%USERPROFILE%\.codex\<name>.config.toml` | Profile layered over the base config by --profile <name>; the name must be a plain name. |
| windows | repo | `.codex/config.toml` | Project layer, read from the project root down to the working directory with the closest winning; trusted projects only. |
| macos | user | `~/.codex/auth.json` | Stored credentials. Present locally; its contents were not read. |
| linux | user | `~/.codex/auth.json` | Stored credentials; same layout as macOS, not inspected on Linux. |
| windows | user | `%USERPROFILE%\.codex\auth.json` | Stored credentials; same layout as macOS, not inspected on Windows. |

The CLI writes `config.toml` itself for `features enable`, `features disable`, and `mcp add/remove`, and writes sessions, SQLite databases, and caches under the same directory.

## Environment Variables

Only general runtime variables are listed here. Model endpoints belong to `model-config`, permissions to `agent-permissions`, MCP to `mcp`, and logging to `agent-logging`.

| Variable | Effect |
| -------- | ------ |
| `CODEX_HOME` | Replaces ~/.codex as the home for configuration, credentials, sessions, skills, rules, and state; also where profile files and the standalone install live. |
| `CODEX_SQLITE_HOME` | Moves the SQLite state databases; when unset they follow CODEX_HOME or the configured sqlite_home. |
| `CODEX_API_KEY` | Supplies an API key for one run of exec or review without a stored login (documented for exec, review, and exec-server --remote). |
| `CODEX_ACCESS_TOKEN` | Named in the binary and in the help for codex login --with-access-token as the source of an access token; whether exec reads it directly was not established. |
| `OPENAI_API_KEY` | Read as an API-key credential when no stored login or CODEX_API_KEY applies; precedence against CODEX_API_KEY was not established. |
| `CODEX_CA_CERTIFICATE` | Path to a CA bundle trusted for Codex HTTPS and websocket traffic; SSL_CERT_FILE is the fallback the binary also names. |
| `RUST_LOG` | Standard tracing filter for the Rust logging stack, documented in docs/install.md. |
| `NO_COLOR` | Named in the binary next to the --no-color handling; its exact effect was not established. |
| `CODEX_TUI_DISABLE_KEYBOARD_ENHANCEMENT` | Named in the binary; by its name it turns off the terminal keyboard-enhancement protocol in the terminal UI. Not run. |
| `CODEX_THREAD_ID` | Present in the binary beside the exec-command path, which suggests Codex passes the thread id to commands it runs; not confirmed by a run. |
| `CODEX_SANDBOX_NETWORK_DISABLED` | Present in the binary with the value 1, which suggests it marks commands whose sandbox disables network access; not confirmed by a run. |
| `CODEX_INSTALL_DIR` | Installer-only: directory the install scripts put the codex executable in. |
| `CODEX_NON_INTERACTIVE` | Installer and updater only: set to 1 to run the install script without prompting. |

## Machine Introspection

| Command | Output | Notes |
| ------- | ------ | ----- |
| `codex --version` | text | Prints "codex-cli 0.159.3". |
| `codex doctor --json` | json | Redacted report with schemaVersion, overallStatus, codexVersion, and checks keyed by id. Exits 1 when overallStatus is fail while still printing the full report. |
| `codex debug models` | json | Raw model catalog (about 650 KB bundled): slug, display_name, default_reasoning_level, supported_reasoning_levels. Without --bundled it attempts a refresh, so it may use the network. |
| `codex debug models --bundled` | json | Offline; the catalog compiled into this binary. |
| `codex debug prompt-input [PROMPT]` | json | JSON array of the prompt items a session would send, including developer instructions and the skills list. Contacts no model. |
| `codex features list` | table | Whitespace-aligned columns: feature name, stage (for example "under development", "experimental", "removed"), effective state. No JSON mode. |
| `codex mcp list --json` | json | JSON array of configured servers; [] when none. |
| `codex exec --json "<prompt>"` | jsonl | Streams thread.started, turn.*, item.*, and error events per official docs; runs a billed model session. |
| `codex migrate-rollouts --json` | json | Complete per-thread report; read-only without --apply. |
| `codex login status` | text | Prints "Not logged in" and exits 1 without credentials; exit status is the machine-readable part. |
| `codex completion zsh` | text | About 240 KB zsh completion script; also bash, elvish, fish, and powershell. |
| `codex app-server generate-json-schema --out <DIR>` | unknown | Writes the app-server protocol schema into the directory; not run here. |

## Wrapper Notes

- Codex CLI 0.159.3 was installed and run on macOS arm64; Linux and Windows behavior comes from install scripts and documentation only.
- --image/-i is greedy at the root, exec, and debug prompt-input: "codex exec -i a.png b.png" treats b.png as a second image and then waits on stdin for a prompt. Write --image=PATH or -iPATH, put the prompt before the switch, or end the switches with "--". At exec resume and exec fork the space form takes one value.
- Codex reads the prompt from stdin when the argument is omitted or is "-", and when both exist appends stdin as a <stdin> block. Give it a closed stdin or a real prompt; with nothing available it prints "No prompt provided via stdin." and exits 1.
- Non-global exec switches (--sandbox, --cd, --color, --oss, --profile, --add-dir) are not accepted after resume, fork, or review. Place them before the subcommand; the source merges the exec-level values into the subcommand. Only --model, --dangerously-bypass-approvals-and-sandbox, --dangerously-bypass-hook-trust, --worktree, and the exec globals (--json, --output-schema, --output-last-message, --ephemeral, --skip-git-repo-check, --ignore-user-config, --ignore-rules, --thread-source, --strict-config) apply after it. Source-derived, not run, because a run would be billed.
- exec has no --ask-for-approval; the interactive root has. A non-interactive run reports "approval: never" and takes its policy from configuration, so pass -c approval_policy=... when a different policy is needed.
- An unauthenticated exec still prints its banner (workdir, model, sandbox, session id) on stderr, retries a websocket, and exits 101 on 401. exec resume with an unknown session operand did not fail a lookup before the network step, while exec fork failed immediately with "Session not found"; check the id yourself before resuming.
- Parse errors exit 2 with a message naming the switch. Runtime errors for -c without "=" or an unknown --enable name exit 1. doctor --json and login status exit 1 on a failed check or missing login, so read the output rather than only the status.
- A first word that is not a subcommand is taken as the interactive PROMPT: "codex mcp-server --help" printed the root help rather than an error. The mcp-server command from earlier versions no longer exists. A typo therefore starts a terminal UI instead of failing.
- -c values are TOML first and a literal string second, so -c model=o3 and -c model="o3" both work but -c key=[a,b] needs quoting for the shell. Root-level -c values are lower precedence than the subcommand's.
- --ephemeral and --ignore-user-config change what is read or written but authentication still comes from CODEX_HOME. Pointing CODEX_HOME at a fresh directory removes the login and creates a skills/.system folder there.
- Hidden commands exist outside help output: execpolicy, tcp-tunnel, responses-api-proxy, stdio-to-uds, and debug trace-reduce and clear-memories. --yolo, --not-so-yolo, and --experimental-json are hidden aliases; none are documented as stable.
- The developers.openai.com/codex pages now answer 308 redirects to learn.chatgpt.com/docs; use the new locations.

## Sources

- [Codex CLI overview and install](https://learn.chatgpt.com/docs/codex/cli), reached from <https://developers.openai.com/codex/cli>
- [Command reference](https://learn.chatgpt.com/docs/developer-commands?surface=cli), reached from <https://developers.openai.com/codex/cli/reference>
- [Configuration basics](https://learn.chatgpt.com/docs/config-file/config-basic)
- [Non-interactive mode](https://learn.chatgpt.com/docs/non-interactive-mode)
- [`codex-rs/exec/src/cli.rs`](https://github.com/openai/codex/blob/rust-v0.159.3/codex-rs/exec/src/cli.rs)
- [`codex-rs/utils/cli/src/shared_options.rs`](https://github.com/openai/codex/blob/rust-v0.159.3/codex-rs/utils/cli/src/shared_options.rs)
- [`codex-rs/tui/src/cli.rs`](https://github.com/openai/codex/blob/rust-v0.159.3/codex-rs/tui/src/cli.rs)
- [`codex-rs/cli/src/main.rs`](https://github.com/openai/codex/blob/rust-v0.159.3/codex-rs/cli/src/main.rs)
- [`codex-rs/utils/cli/src/config_override.rs`](https://github.com/openai/codex/blob/rust-v0.159.3/codex-rs/utils/cli/src/config_override.rs)
- [`scripts/install`](https://github.com/openai/codex/tree/rust-v0.159.3/scripts/install) (install.sh, install.ps1)
- [`docs/install.md`](https://github.com/openai/codex/blob/rust-v0.159.3/docs/install.md)
- [GitHub releases](https://github.com/openai/codex/releases)
- Local inspection: `codex --version` and `codex <path> --help` for the root and 72 paths, the throwaway-home parser probes, and `strings` over the installed binary, all on 2026-10-01.

## Changelog

- Rewritten for contract revision 2: the switch inventory is typed per command path, with value types and attachment forms established by clap source and 234 parser probes instead of help text.
- Version moves from 0.142.5 to 0.159.3 installed; the newest stable upstream is 0.160.0, up from 0.142.5.
- --image is two different switches: greedy and variadic at the root, exec, and debug prompt-input; one value per occurrence at exec resume and exec fork. The previous document did not distinguish them.
- Commands added since the last version: agents, queue, migrate-rollouts, exec fork, and mcp list/get/add/remove/login/logout, plugin marketplace, and app-server daemon trees are enumerated. The mcp-server command and the visible execpolicy command are gone (execpolicy remains hidden).
- New switches: --worktree, --thread-source, --approve-for-me (alias --not-so-yolo), --no-daemon, --remote, and --profile now layers $CODEX_HOME/<name>.config.toml.
- Official documentation moved from developers.openai.com/codex to learn.chatgpt.com/docs through 308 redirects.
- The installer scripts now name releases.openai.com with a GitHub fallback, and CODEX_NON_INTERACTIVE and CODEX_INSTALL_DIR are installer-only variables.