---
$schema: ./_schema.yaml
created: 2026-07-02
last_updated: 2026-07-03
agent: codex
model: default
latest_version: "0.142.5"
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
    value: "<key=value>"
    scope: ["global", "config"]
    default: ""
    description: "Override a configuration value for this invocation; dotted paths are supported and values parse as TOML when possible."
    example: "codex -c model='gpt-5.5'"
    notes: "Short form: -c. Overrides take precedence over config.toml."
  - flag: --enable
    value: "<FEATURE>"
    scope: ["global", "features"]
    default: ""
    description: "Enable a feature flag for this invocation."
    example: "codex --enable multi_agent"
    notes: "Repeatable; equivalent to -c features.<name>=true."
  - flag: --disable
    value: "<FEATURE>"
    scope: ["global", "features"]
    default: ""
    description: "Disable a feature flag for this invocation."
    example: "codex --disable browser_use"
    notes: "Repeatable; equivalent to -c features.<name>=false."
  - flag: --strict-config
    value: ""
    scope: ["runtime", "config"]
    default: "false"
    description: "Error when config.toml contains fields this Codex version does not recognize."
    example: "codex exec --strict-config 'summarize'"
    notes: "Supported by runtime commands such as codex, exec, review, resume, fork, app-server, mcp-server, and exec-server."
  - flag: --remote
    value: "<ADDR>"
    scope: ["interactive", "remote"]
    default: ""
    description: "Connect the TUI to a remote app-server endpoint."
    example: "codex --remote ws://127.0.0.1:1455"
    notes: "Accepted forms include ws://host:port, wss://host:port, unix://, and unix://PATH."
  - flag: --remote-auth-token-env
    value: "<ENV_VAR>"
    scope: ["interactive", "remote"]
    default: ""
    description: "Read a bearer token from an environment variable for remote WebSocket authentication."
    example: "codex --remote wss://example.test --remote-auth-token-env CODEX_REMOTE_TOKEN"
    notes: "Requires --remote."
  - flag: --image
    value: "<FILE>..."
    scope: ["interactive", "exec", "input"]
    default: ""
    description: "Attach one or more image files to the initial prompt or exec resume prompt."
    example: "codex -i screenshot.png 'implement this design'"
    notes: "Short form: -i. Docs allow comma-separated paths or repeated flags; local help shows variadic path arguments."
  - flag: --model
    value: "<MODEL>"
    scope: ["interactive", "exec", "model_selection"]
    default: "config/default"
    description: "Select the model the agent should use."
    example: "codex -m gpt-5.5 'summarize this repo'"
    notes: "Short form: -m."
  - flag: --oss
    value: ""
    scope: ["interactive", "exec", "model_selection"]
    default: "false"
    description: "Use the local open-source model provider."
    example: "codex --oss"
    notes: "Equivalent to selecting the oss model provider; docs mention Ollama validation."
  - flag: --local-provider
    value: "<OSS_PROVIDER>"
    scope: ["interactive", "exec", "model_selection"]
    default: "config/default"
    description: "Select the local provider to use with open-source models."
    example: "codex --oss --local-provider ollama"
    notes: "Local 0.142.5 help lists lmstudio and ollama."
  - flag: --profile
    value: "<NAME>"
    scope: ["runtime", "config"]
    default: ""
    description: "Layer $CODEX_HOME/<name>.config.toml on top of the base user config."
    example: "codex --profile work"
    notes: "Short form: -p on most commands; sandbox also uses -p for profile, while -P selects permissions profile."
  - flag: --sandbox
    value: "read-only | workspace-write | danger-full-access"
    scope: ["interactive", "exec", "permissions"]
    default: "read-only for exec; config/default otherwise"
    description: "Select the sandbox policy for model-generated shell commands."
    example: "codex exec --sandbox workspace-write 'run tests'"
    notes: "Short form: -s."
  - flag: --dangerously-bypass-approvals-and-sandbox
    value: ""
    scope: ["interactive", "exec", "permissions"]
    default: "false"
    description: "Run without approval prompts or sandboxing."
    example: "codex exec --dangerously-bypass-approvals-and-sandbox 'run in an external sandbox'"
    notes: "Official docs also document alias --yolo. Wrapper should only use inside an external sandbox."
  - flag: --full-auto
    value: ""
    scope: ["exec", "permissions"]
    default: "false"
    description: "Deprecated compatibility flag for older non-interactive automation."
    example: "codex exec --full-auto 'legacy automation task'"
    notes: "Documented official reference says it prints a warning and maps toward workspace-write automation; local 0.142.5 help omits it."
  - flag: --dangerously-bypass-hook-trust
    value: ""
    scope: ["interactive", "exec", "hooks"]
    default: "false"
    description: "Run enabled hooks without requiring persisted hook trust for this invocation."
    example: "codex exec --dangerously-bypass-hook-trust 'run vetted automation'"
    notes: "Intended only for automation that already vets hook sources."
  - flag: --cd
    value: "<DIR>"
    scope: ["interactive", "exec", "review", "sandbox", "working_directory"]
    default: "current directory"
    description: "Set the working directory or workspace root before executing the task."
    example: "codex exec -C /repo 'summarize'"
    notes: "Short form: -C."
  - flag: --add-dir
    value: "<DIR>"
    scope: ["interactive", "exec", "permissions"]
    default: ""
    description: "Grant additional directories write access alongside the main workspace."
    example: "codex --sandbox workspace-write --add-dir ../shared"
    notes: "Repeatable."
  - flag: --ask-for-approval
    value: "untrusted | on-request | never"
    scope: ["interactive", "permissions"]
    default: "config/default"
    description: "Configure when the model requires human approval before executing a command."
    example: "codex --ask-for-approval on-request"
    notes: "Short form: -a. Local help still lists deprecated on-failure; official docs prefer untrusted, on-request, or never."
  - flag: --search
    value: ""
    scope: ["interactive", "tools"]
    default: "cached/config"
    description: "Enable live web search for the interactive run."
    example: "codex --search 'research this dependency'"
    notes: "Official docs describe cached web search defaults separately; wrapper-owned streaming research should not duplicate this."
  - flag: --no-alt-screen
    value: ""
    scope: ["interactive", "terminal"]
    default: "false"
    description: "Disable alternate screen mode and keep TUI output inline."
    example: "codex --no-alt-screen"
    notes: "Overrides tui.alternate_screen for the run."
  - flag: --skip-git-repo-check
    value: ""
    scope: ["exec"]
    default: "false"
    description: "Allow codex exec to run outside a Git repository."
    example: "codex exec --skip-git-repo-check 'inspect this folder'"
    notes: "Also available on exec resume and exec review."
  - flag: --ephemeral
    value: ""
    scope: ["exec", "state"]
    default: "false"
    description: "Run without persisting session files to disk."
    example: "codex exec --ephemeral 'summarize'"
    notes: "Useful for CI or privacy-sensitive wrapper runs."
  - flag: --ignore-user-config
    value: ""
    scope: ["exec", "config"]
    default: "false"
    description: "Do not load $CODEX_HOME/config.toml while still using CODEX_HOME for auth."
    example: "codex exec --ignore-user-config 'run with defaults'"
    notes: "Exec-specific."
  - flag: --ignore-rules
    value: ""
    scope: ["exec", "permissions"]
    default: "false"
    description: "Do not load user or project execpolicy .rules files."
    example: "codex exec --ignore-rules 'run task'"
    notes: "Exec-specific."
  - flag: --output-schema
    value: "<FILE>"
    scope: ["exec", "output"]
    default: ""
    description: "Path to a JSON Schema file describing the model's final response shape."
    example: "codex exec --output-schema schema.json -o result.json 'extract metadata'"
    notes: "Constrained final output."
  - flag: --color
    value: "always | never | auto"
    scope: ["exec", "output"]
    default: "auto"
    description: "Control ANSI color in exec output."
    example: "codex exec --color never 'summarize'"
    notes: "Exec-specific."
  - flag: --json
    value: ""
    scope: ["exec", "doctor", "mcp", "plugin", "remote-control", "cloud", "output"]
    default: "false"
    description: "Emit machine-readable JSON or JSONL where supported."
    example: "codex exec --json 'summarize'"
    notes: "exec emits JSONL events; doctor emits a JSON report; mcp/plugin/cloud subcommands emit JSON documents."
  - flag: --experimental-json
    value: ""
    scope: ["exec", "output"]
    default: "false"
    description: "Documented alias/experimental form for exec JSONL output."
    example: "codex exec --experimental-json 'summarize'"
    notes: "Official reference documents it with --json; local 0.142.5 help omits it."
  - flag: --output-last-message
    value: "<FILE>"
    scope: ["exec", "output"]
    default: ""
    description: "Write the final assistant message to a file."
    example: "codex exec --json -o final.md 'summarize'"
    notes: "Short form: -o."
  - flag: --uncommitted
    value: ""
    scope: ["review", "exec review"]
    default: "false"
    description: "Review staged, unstaged, and untracked changes."
    example: "codex review --uncommitted"
    notes: "Review-specific."
  - flag: --base
    value: "<BRANCH>"
    scope: ["review", "exec review"]
    default: ""
    description: "Review changes against the given base branch."
    example: "codex review --base main"
    notes: "Review-specific."
  - flag: --commit
    value: "<SHA>"
    scope: ["review", "exec review"]
    default: ""
    description: "Review the changes introduced by a commit."
    example: "codex review --commit HEAD"
    notes: "Review-specific."
  - flag: --title
    value: "<TITLE>"
    scope: ["review", "exec review"]
    default: ""
    description: "Set an optional commit title to display in the review summary."
    example: "codex review --title 'Auth cleanup'"
    notes: "Review-specific."
  - flag: --last
    value: ""
    scope: ["resume", "fork", "exec resume"]
    default: "false"
    description: "Use the most recent session without showing a picker."
    example: "codex exec resume --last 'continue'"
    notes: "Wrapper-safe only for exec resume; top-level resume/fork still enter TUI mode."
  - flag: --all
    value: ""
    scope: ["resume", "fork", "exec resume", "doctor"]
    default: "false"
    description: "For session commands, disable cwd filtering; for doctor, expand long human-readable lists."
    example: "codex exec resume --all --last 'continue'"
    notes: "Meaning is command-specific."
  - flag: --force
    value: ""
    scope: ["delete"]
    default: "false"
    description: "Delete a session without prompting."
    example: "codex delete --force 00000000-0000-0000-0000-000000000000"
    notes: "Local help requires SESSION to be a UUID; names still require confirmation."
  - flag: --with-api-key
    value: ""
    scope: ["login", "auth"]
    default: "false"
    description: "Read an API key from stdin for persisted login."
    example: "printenv OPENAI_API_KEY | codex login --with-api-key"
    notes: "Avoids browser prompts but consumes a secret from stdin."
  - flag: --with-access-token
    value: ""
    scope: ["login", "auth"]
    default: "false"
    description: "Read an access token from stdin for persisted login."
    example: "printenv CODEX_ACCESS_TOKEN | codex login --with-access-token"
    notes: "Avoids browser prompts but consumes a secret from stdin."
  - flag: --device-auth
    value: ""
    scope: ["login", "auth"]
    default: "false"
    description: "Use device authentication."
    example: "codex login --device-auth"
    notes: "Interactive/browser-adjacent; local help has no detailed description."
  - flag: --listen
    value: "stdio:// | ws://IP:PORT | unix:// | unix://PATH | off"
    scope: ["app-server", "exec-server"]
    default: "stdio:// for app-server; ws://IP:PORT for exec-server"
    description: "Select the server transport endpoint."
    example: "codex app-server --listen ws://127.0.0.1:1455"
    notes: "exec-server accepts ws://IP:PORT, stdio, and stdio://."
  - flag: --stdio
    value: ""
    scope: ["app-server"]
    default: "false"
    description: "Use stdio as the app-server transport."
    example: "codex app-server --stdio"
    notes: "Equivalent to --listen stdio://."
  - flag: --analytics-default-enabled
    value: ""
    scope: ["app-server"]
    default: "false"
    description: "Default analytics to enabled for first-party app-server clients unless disabled in config."
    example: "codex app-server --analytics-default-enabled"
    notes: "App-server-specific."
  - flag: --ws-auth
    value: "capability-token | signed-bearer-token"
    scope: ["app-server", "remote"]
    default: ""
    description: "Select WebSocket auth mode for non-loopback app-server listeners."
    example: "codex app-server --listen ws://0.0.0.0:1455 --ws-auth capability-token"
    notes: "Requires matching token or secret configuration."
  - flag: --ws-token-file
    value: "<PATH>"
    scope: ["app-server", "remote"]
    default: ""
    description: "Absolute path to the capability-token file."
    example: "codex app-server --ws-token-file /secure/token"
    notes: "App-server-specific."
  - flag: --ws-token-sha256
    value: "<HEX>"
    scope: ["app-server", "remote"]
    default: ""
    description: "Hex-encoded SHA-256 digest of the capability token."
    example: "codex app-server --ws-token-sha256 <hex>"
    notes: "App-server-specific."
  - flag: --ws-shared-secret-file
    value: "<PATH>"
    scope: ["app-server", "remote"]
    default: ""
    description: "Absolute path to the shared secret file for signed JWT bearer tokens."
    example: "codex app-server --ws-shared-secret-file /secure/secret"
    notes: "App-server-specific."
  - flag: --ws-issuer
    value: "<ISSUER>"
    scope: ["app-server", "remote"]
    default: ""
    description: "Expected issuer for signed JWT bearer tokens."
    example: "codex app-server --ws-issuer https://issuer.example"
    notes: "App-server-specific."
  - flag: --ws-audience
    value: "<AUDIENCE>"
    scope: ["app-server", "remote"]
    default: ""
    description: "Expected audience for signed JWT bearer tokens."
    example: "codex app-server --ws-audience codex"
    notes: "App-server-specific."
  - flag: --ws-max-clock-skew-seconds
    value: "<SECONDS>"
    scope: ["app-server", "remote"]
    default: ""
    description: "Maximum clock skew when validating signed JWT bearer tokens."
    example: "codex app-server --ws-max-clock-skew-seconds 60"
    notes: "App-server-specific."
  - flag: --download-url
    value: "<URL>"
    scope: ["app"]
    default: ""
    description: "Override the app installer download URL."
    example: "codex app --download-url https://example.test/Codex.dmg"
    notes: "Advanced desktop-app option."
  - flag: --summary
    value: ""
    scope: ["doctor"]
    default: "false"
    description: "Show grouped check rows and the final count summary only."
    example: "codex doctor --summary"
    notes: "Doctor-specific."
  - flag: --no-color
    value: ""
    scope: ["doctor"]
    default: "false"
    description: "Disable ANSI color in human-readable doctor output."
    example: "codex doctor --no-color"
    notes: "Doctor-specific."
  - flag: --ascii
    value: ""
    scope: ["doctor"]
    default: "false"
    description: "Use ASCII status labels and separators in human-readable doctor output."
    example: "codex doctor --ascii"
    notes: "Doctor-specific."
  - flag: --bundled
    value: ""
    scope: ["debug models"]
    default: "false"
    description: "Skip refresh and dump only the bundled model catalog shipped with this binary."
    example: "codex debug models --bundled"
    notes: "Prints JSON to stdout."
  - flag: --env
    value: "<KEY=VALUE>"
    scope: ["mcp add", "cloud"]
    default: ""
    description: "For mcp add, set an environment variable for a stdio MCP server; for cloud commands, select/filter a Codex Cloud environment."
    example: "codex mcp add server --env KEY=value -- command"
    notes: "Meaning is command-specific."
  - flag: --url
    value: "<URL>"
    scope: ["mcp add"]
    default: ""
    description: "Register a streamable HTTP MCP server instead of a stdio command."
    example: "codex mcp add docs --url https://mcp.example"
    notes: "Mutually exclusive with COMMAND."
  - flag: --bearer-token-env-var
    value: "<ENV_VAR>"
    scope: ["mcp add"]
    default: ""
    description: "Read a bearer token environment variable for a streamable HTTP MCP server."
    example: "codex mcp add docs --url https://mcp.example --bearer-token-env-var MCP_TOKEN"
    notes: "Only valid with streamable HTTP servers."
  - flag: --oauth-client-id
    value: "<CLIENT_ID>"
    scope: ["mcp add"]
    default: ""
    description: "Set an OAuth client identifier for a streamable HTTP MCP server."
    example: "codex mcp add docs --url https://mcp.example --oauth-client-id client"
    notes: "Requires --url."
  - flag: --oauth-resource
    value: "<RESOURCE>"
    scope: ["mcp add"]
    default: ""
    description: "Set the OAuth resource parameter to include during MCP login."
    example: "codex mcp add docs --url https://mcp.example --oauth-resource resource"
    notes: "Requires --url."
  - flag: --scopes
    value: "<SCOPE,SCOPE>"
    scope: ["mcp login"]
    default: ""
    description: "Set OAuth scopes when logging into a streamable HTTP MCP server."
    example: "codex mcp login docs --scopes read,write"
    notes: "Only for servers that support OAuth."
  - flag: --marketplace
    value: "<MARKETPLACE>"
    scope: ["plugin add", "plugin list", "plugin remove"]
    default: ""
    description: "Select a configured plugin marketplace."
    example: "codex plugin list --marketplace debug"
    notes: "Short form: -m."
  - flag: --available
    value: ""
    scope: ["plugin list"]
    default: "false"
    description: "Include uninstalled marketplace plugins in JSON output."
    example: "codex plugin list --available --json"
    notes: "Requires or is useful with --json."
  - flag: --permissions-profile
    value: "<NAME>"
    scope: ["sandbox"]
    default: ""
    description: "Apply a named permissions profile from the active configuration stack."
    example: "codex sandbox --permissions-profile ci -- echo ok"
    notes: "Short form: -P."
  - flag: --include-managed-config
    value: ""
    scope: ["sandbox"]
    default: "false"
    description: "Include managed requirements while resolving an explicit permissions profile."
    example: "codex sandbox --permissions-profile ci --include-managed-config -- echo ok"
    notes: "Requires --permissions-profile."
  - flag: --allow-unix-socket
    value: "<PATH>"
    scope: ["sandbox", "macos"]
    default: ""
    description: "Allow sandboxed commands to bind or connect AF_UNIX sockets rooted at this path."
    example: "codex sandbox --allow-unix-socket ./sock -- command"
    notes: "Local macOS help lists this flag; repeatable."
  - flag: --log-denials
    value: ""
    scope: ["sandbox", "macos"]
    default: "false"
    description: "Capture macOS sandbox denials via log stream and print them after exit."
    example: "codex sandbox --log-denials -- command"
    notes: "macOS-specific."
  - flag: --rules
    value: "<PATH>"
    scope: ["execpolicy check"]
    default: ""
    description: "Add an execpolicy rule file to evaluate."
    example: "codex execpolicy check --rules ~/.codex/rules/default.rules -- git status"
    notes: "Short form: -r. Repeatable."
  - flag: --pretty
    value: ""
    scope: ["execpolicy check"]
    default: "false"
    description: "Pretty-print execpolicy check JSON."
    example: "codex execpolicy check --pretty --rules policy.rules -- git status"
    notes: "Output remains JSON."
  - flag: --resolve-host-executables
    value: ""
    scope: ["execpolicy check"]
    default: "false"
    description: "Resolve absolute program paths against basename rules when policy permits host executable matching."
    example: "codex execpolicy check --resolve-host-executables --rules policy.rules -- /usr/bin/git status"
    notes: "Execpolicy-specific."
  - flag: --out
    value: "<DIR>"
    scope: ["app-server generate-ts", "app-server generate-json-schema"]
    default: ""
    description: "Output directory for generated TypeScript bindings or JSON Schema bundles."
    example: "codex app-server generate-json-schema --out ./schema"
    notes: "Short form: -o. Required by generation subcommands."
  - flag: --prettier
    value: "<PRETTIER_BIN>"
    scope: ["app-server generate-ts"]
    default: ""
    description: "Optional Prettier executable used to format generated TypeScript files."
    example: "codex app-server generate-ts --out ./ts --prettier ./node_modules/.bin/prettier"
    notes: "Short form: -p."
  - flag: --experimental
    value: ""
    scope: ["app-server generate-ts", "app-server generate-json-schema"]
    default: "false"
    description: "Include experimental methods and fields in generated protocol output."
    example: "codex app-server generate-json-schema --out ./schema --experimental"
    notes: "Generation-specific."
  - flag: --attempts
    value: "<N>"
    scope: ["cloud exec"]
    default: "1"
    description: "Number of assistant attempts for a Codex Cloud task."
    example: "codex cloud exec --env env_id --attempts 2 'fix bug'"
    notes: "Cloud-specific."
  - flag: --branch
    value: "<BRANCH>"
    scope: ["cloud exec"]
    default: "current branch"
    description: "Git branch to run in Codex Cloud."
    example: "codex cloud exec --env env_id --branch main 'fix bug'"
    notes: "Cloud-specific."
  - flag: --limit
    value: "<N>"
    scope: ["cloud list"]
    default: "20"
    description: "Maximum number of Codex Cloud tasks to return."
    example: "codex cloud list --limit 10 --json"
    notes: "Allowed range from local help: 1-20."
  - flag: --cursor
    value: "<CURSOR>"
    scope: ["cloud list"]
    default: ""
    description: "Pagination cursor returned by a previous cloud list call."
    example: "codex cloud list --json --cursor abc"
    notes: "Cloud-specific."
  - flag: --attempt
    value: "<N>"
    scope: ["cloud apply", "cloud diff"]
    default: ""
    description: "Attempt number to apply or display."
    example: "codex cloud diff task_id --attempt 1"
    notes: "One-based."
  - flag: --environment-id
    value: "<ID>"
    scope: ["exec-server"]
    default: ""
    description: "Environment id to attach to when registering remotely."
    example: "codex exec-server --remote https://example.test --environment-id env_id"
    notes: "Exec-server-specific."
  - flag: --name
    value: "<NAME>"
    scope: ["exec-server"]
    default: ""
    description: "Human-readable environment name."
    example: "codex exec-server --name laptop"
    notes: "Exec-server-specific."
  - flag: --use-agent-identity-auth
    value: ""
    scope: ["exec-server"]
    default: "false"
    description: "Use Agent Identity auth from CODEX_ACCESS_TOKEN for remote registration."
    example: "CODEX_ACCESS_TOKEN=... codex exec-server --use-agent-identity-auth"
    notes: "Exec-server-specific."
  - flag: --developer_instructions
    value: "<string via -c developer_instructions=...>"
    scope: ["system-prompt", "config"]
    default: ""
    description: "Config override for additional developer instructions injected into the session."
    example: "codex -c developer_instructions='Follow repo policy.'"
    notes: "No dedicated local CLI flag exists; record only existence here and defer semantics to the sibling system-prompt topic."
  - flag: --model_instructions_file
    value: "<path via -c model_instructions_file=...>"
    scope: ["system-prompt", "config"]
    default: ""
    description: "Config override for replacing built-in model instructions from a file."
    example: "codex -c model_instructions_file='./instructions.txt'"
    notes: "No dedicated local CLI flag exists; record only existence here and defer semantics to the sibling system-prompt topic."
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

# Codex CLI Public Surface

## Overview

Codex CLI is OpenAI's local coding agent for the terminal. The primary command a user types is `codex`, which starts an interactive terminal UI in the current directory. The automation entry point is `codex exec`, which runs a task non-interactively and exits.

The verified stable upstream version is `0.142.5`. I verified this three ways on July 3, 2026: local `codex --version` returned `codex-cli 0.142.5`; `npm view @openai/codex version dist-tags --json` returned `latest: 0.142.5`; and the GitHub repository page listed `0.142.5` as the latest stable release. GitHub releases and npm also show `0.143.0-alpha.35` prereleases, but those are on the alpha channel rather than npm `latest`.

Primary URLs:

| Purpose | URL |
| --- | --- |
| Homepage | [Codex CLI](https://developers.openai.com/codex/cli) |
| Repository | [openai/codex](https://github.com/openai/codex) |
| General docs | [Codex docs](https://developers.openai.com/codex/) |
| CLI reference | [Command line options](https://developers.openai.com/codex/cli/reference) |
| Config reference | [Configuration reference](https://developers.openai.com/codex/config-reference) |
| Environment variables | [Environment variables](https://developers.openai.com/codex/environment-variables) |

## Installation and Binaries

The user-facing command is `codex` on macOS, Linux, and Windows. Local macOS inspection found `/Users/ken/.bun/bin/codex`, managed by bun, dispatching to `/Users/ken/.bun/install/global/node_modules/@openai/codex-darwin-arm64/vendor/aarch64-apple-darwin/bin/codex`. The official docs say Windows users run `codex` natively in PowerShell; native `.exe` and `.cmd` shim details were not locally inspected.

Official installation commands:

| OS | Method | Command |
| --- | --- | --- |
| macOS/Linux | Standalone installer | `curl -fsSL https://chatgpt.com/codex/install.sh \| sh` |
| macOS/Linux | Unattended standalone installer | `curl -fsSL https://chatgpt.com/codex/install.sh \| CODEX_NON_INTERACTIVE=1 sh` |
| Windows | Standalone installer | `powershell -ExecutionPolicy ByPass -c "irm https://chatgpt.com/codex/install.ps1 \| iex"` |
| Windows | Unattended standalone installer | `$env:CODEX_NON_INTERACTIVE=1; irm https://chatgpt.com/codex/install.ps1 \| iex` |
| macOS/Linux/Windows | npm | `npm install -g @openai/codex` |
| macOS | Homebrew | `brew install --cask codex` |
| macOS/Linux/Windows | GitHub release archive | Download from [latest releases](https://github.com/openai/codex/releases/latest). |

Standalone installer paths are configurable with `CODEX_INSTALL_DIR`. The documented default is `~/.local/bin` on macOS/Linux and `%LOCALAPPDATA%\Programs\OpenAI\Codex\bin` on Windows. GitHub release archives for macOS/Linux contain platform-named executables such as `codex-aarch64-apple-darwin` or `codex-x86_64-unknown-linux-musl`; the README says users normally rename the extracted file to `codex`.

## Subcommands

Local `codex --help` in `0.142.5` exposes these top-level commands:

| Command | Description | Automation/interaction notes |
| --- | --- | --- |
| `codex` | Launch the interactive TUI, optionally with an initial prompt. | Requires a TTY for normal use; first run prompts for auth. |
| `codex exec` / `codex e` | Run Codex non-interactively. | Primary wrapper automation entry point. Reads argv/stdin and can emit JSONL. |
| `codex exec resume` | Resume a previous exec session. | Non-interactive when a session id or `--last` is supplied. |
| `codex exec review` | Run review through exec mode. | Non-interactive review automation. |
| `codex review` | Run a code review non-interactively. | Non-interactive. |
| `codex login` | Manage login. | Interactive by default; stdin secret flags avoid browser prompts for persisted login. |
| `codex login status` | Show login status. | Text-only in local help; use `doctor --json` for machine-readable auth state. |
| `codex logout` | Remove stored auth credentials. | Mutates auth state. |
| `codex mcp` | Manage MCP servers. | `list`/`get --json` are scriptable; `add`/`remove` mutate config; OAuth login is interactive. |
| `codex plugin` | Manage plugins and marketplaces. | JSON exists for several subcommands; add/remove/marketplace operations mutate config/cache. |
| `codex mcp-server` | Start Codex as an MCP server over stdio. | Non-interactive service mode. |
| `codex app-server` | Run app-server or related tooling. | Non-interactive service/generation mode, but daemon commands mutate background state. |
| `codex remote-control` | Manage app-server daemon with remote control enabled. | `start`/`stop --json` are scriptable but mutate daemon state. |
| `codex app` | Launch Codex desktop app or installer. | Desktop/browser-like flow, not headless wrapper material. |
| `codex completion` | Generate shell completions. | Non-interactive. Shells: bash, zsh, fish, powershell, elvish. |
| `codex update` | Update Codex to the latest supported version. | Mutates installation; not a wrapper runtime command. |
| `codex doctor` | Diagnose install, config, auth, runtime, Git, terminal, app-server, and thread state. | `--json` is non-interactive and wrapper-relevant. |
| `codex sandbox` | Run a command in a Codex-provided sandbox. | Non-interactive, OS-specific. |
| `codex debug` | Debugging tools. | `debug models` emits JSON; app-server debug tools are specialized. |
| `codex apply` / `codex a` | Apply latest diff from a Codex Cloud task. | Non-interactive but mutates working tree. |
| `codex resume` | Resume an interactive session. | Picker by default; `--last` avoids picker but still opens TUI. |
| `codex archive` | Archive a saved session. | Non-interactive state mutation when session is supplied. |
| `codex delete` | Permanently delete a saved session. | `--force` avoids prompt only for UUID session ids. |
| `codex unarchive` | Restore an archived session. | Non-interactive state mutation. |
| `codex fork` | Fork a previous interactive session. | Picker by default; `--last` avoids picker but still opens TUI. |
| `codex cloud` | Browse or execute Codex Cloud tasks. | `cloud exec` and `cloud list --json` are scriptable; apply mutates working tree. |
| `codex exec-server` | Run experimental standalone exec-server. | Non-interactive service mode. |
| `codex features` | Inspect or mutate feature flags. | `list` is read-only text; `enable`/`disable` write config. |
| `codex execpolicy` | Evaluate execpolicy rule files. | Non-interactive JSON output. Accepted locally and documented, but omitted from local top-level help. |

## CLI Switch Inventory

The local help output and official reference mostly agree, but not perfectly. I trust local help for what this installed `0.142.5` binary accepts. I trust the official reference for documented compatibility flags that local help omits, specifically `--full-auto`, `--experimental-json`, and `codex execpolicy` documentation. Negative probe: `codex execpolicy --help` works locally even though `execpolicy` is absent from `codex --help`.

Global/runtime flags:

| Flag | Value | Scope | Default | Notes |
| --- | --- | --- | --- | --- |
| `-c`, `--config` | `key=value` | Global/config | unset | Parses value as TOML when possible. Example: `codex -c model='gpt-5.5'`. |
| `--enable` | feature key | Global/features | unset | Repeatable; maps to `features.<name>=true`. |
| `--disable` | feature key | Global/features | unset | Repeatable; maps to `features.<name>=false`. |
| `--strict-config` | boolean | Runtime/config | false | Errors on unrecognized config fields. |
| `-m`, `--model` | model id | Interactive/exec | config | Overrides configured model. |
| `--oss` | boolean | Interactive/exec | false | Uses local open-source provider. |
| `--local-provider` | `lmstudio` or `ollama` | Interactive/exec | config/prompt | Selects OSS provider. |
| `-p`, `--profile` | name | Runtime/config | unset | Loads `$CODEX_HOME/<name>.config.toml`. |
| `-C`, `--cd` | directory | Runtime/workspace | cwd | Sets working root. |
| `--add-dir` | directory | Runtime/permissions | unset | Repeatable additional writable roots. |
| `-s`, `--sandbox` | `read-only`, `workspace-write`, `danger-full-access` | Runtime/permissions | exec defaults read-only | Sandbox mode. |
| `--dangerously-bypass-approvals-and-sandbox` | boolean | Runtime/permissions | false | Also documented as `--yolo`; only safe inside an external sandbox. |
| `--dangerously-bypass-hook-trust` | boolean | Runtime/hooks | false | Runs enabled hooks without persisted trust. |
| `-i`, `--image` | file paths | Interactive/exec input | unset | Attach images. Docs allow comma-separated paths or repeated flags; local help shows variadic path args. |

Interactive/TUI flags:

| Flag | Value | Default | Example | Notes |
| --- | --- | --- | --- | --- |
| `--remote` | `ws://...`, `wss://...`, `unix://`, `unix://PATH` | unset | `codex --remote ws://127.0.0.1:1455` | Connect TUI to remote app-server. |
| `--remote-auth-token-env` | env var name | unset | `codex --remote wss://host --remote-auth-token-env CODEX_REMOTE_TOKEN` | Sends bearer token from env. |
| `-a`, `--ask-for-approval` | `untrusted`, `on-request`, `never`; local help also lists deprecated `on-failure` | config | `codex --ask-for-approval on-request` | Approval policy. |
| `--search` | boolean | config/cached | `codex --search "research this"` | Enables live web search. |
| `--no-alt-screen` | boolean | false | `codex --no-alt-screen` | Keeps TUI inline instead of alternate screen. |

`codex exec` and exec-family flags:

| Flag | Value | Default | Example | Notes |
| --- | --- | --- | --- | --- |
| `PROMPT` | string or `-` | stdin/prompt | `codex exec "summarize"` | If stdin is piped and prompt is also supplied, stdin is appended as a `<stdin>` block. |
| `--skip-git-repo-check` | boolean | false | `codex exec --skip-git-repo-check "inspect"` | Allows running outside Git. |
| `--ephemeral` | boolean | false | `codex exec --ephemeral "summarize"` | Avoids persisted session files. |
| `--ignore-user-config` | boolean | false | `codex exec --ignore-user-config "run"` | Skips `$CODEX_HOME/config.toml`; auth still uses `CODEX_HOME`. |
| `--ignore-rules` | boolean | false | `codex exec --ignore-rules "run"` | Skips user and project execpolicy rules. |
| `--output-schema` | file path | unset | `codex exec --output-schema schema.json "extract"` | Requests final response matching JSON Schema. |
| `--color` | `always`, `never`, `auto` | auto | `codex exec --color never "summarize"` | ANSI color control. |
| `--json` | boolean | false | `codex exec --json "summarize"` | JSON Lines event stream on stdout. |
| `--experimental-json` | boolean | false | `codex exec --experimental-json "summarize"` | Officially documented with `--json`; omitted from local help. |
| `-o`, `--output-last-message` | file path | unset | `codex exec --json -o final.md "summarize"` | Writes final assistant message. |
| `--full-auto` | boolean | false | `codex exec --full-auto "legacy task"` | Deprecated compatibility flag documented by OpenAI; local help omits it. |
| `--last` | boolean | false | `codex exec resume --last "continue"` | Exec resume without explicit session id. |
| `--all` | boolean | false | `codex exec resume --all --last "continue"` | Disables cwd filtering for session lookup. |

Review flags:

| Flag | Value | Scope | Example |
| --- | --- | --- | --- |
| `--uncommitted` | boolean | `review`, `exec review` | `codex review --uncommitted` |
| `--base` | branch | `review`, `exec review` | `codex review --base main` |
| `--commit` | SHA | `review`, `exec review` | `codex review --commit HEAD` |
| `--title` | title | `review`, `exec review` | `codex review --title "Auth cleanup"` |

Auth flags:

| Flag | Value | Scope | Example | Notes |
| --- | --- | --- | --- | --- |
| `--with-api-key` | stdin secret | `login` | `printenv OPENAI_API_KEY \| codex login --with-api-key` | Persist API-key login. |
| `--with-access-token` | stdin secret | `login` | `printenv CODEX_ACCESS_TOKEN \| codex login --with-access-token` | Persist token login. |
| `--device-auth` | boolean | `login` | `codex login --device-auth` | Interactive/device flow. |

MCP flags:

| Flag | Value | Scope | Example |
| --- | --- | --- | --- |
| `--json` | boolean | `mcp list`, `mcp get` | `codex mcp list --json` |
| `--env` | `KEY=VALUE` | `mcp add` stdio servers | `codex mcp add docs --env KEY=value -- command` |
| `--url` | URL | `mcp add` HTTP servers | `codex mcp add docs --url https://mcp.example` |
| `--bearer-token-env-var` | env var | `mcp add` HTTP servers | `codex mcp add docs --url https://mcp.example --bearer-token-env-var MCP_TOKEN` |
| `--oauth-client-id` | client id | `mcp add` | `codex mcp add docs --url https://mcp.example --oauth-client-id client` |
| `--oauth-resource` | resource | `mcp add` | `codex mcp add docs --url https://mcp.example --oauth-resource resource` |
| `--scopes` | comma-separated scopes | `mcp login` | `codex mcp login docs --scopes read,write` |

Plugin flags:

| Flag | Value | Scope | Example |
| --- | --- | --- | --- |
| `--json` | boolean | `plugin add`, `plugin list`, `plugin remove`, marketplace commands in docs | `codex plugin list --json` |
| `-m`, `--marketplace` | marketplace name | `plugin add/list/remove` | `codex plugin list --marketplace debug` |
| `--available` | boolean | `plugin list` | `codex plugin list --available --json` |

App-server, remote-control, and exec-server flags:

| Flag | Value | Scope | Notes |
| --- | --- | --- | --- |
| `--listen` | `stdio://`, `unix://`, `unix://PATH`, `ws://IP:PORT`, `off`; exec-server accepts `ws://IP:PORT`, `stdio`, `stdio://` | app-server/exec-server | Transport endpoint. |
| `--stdio` | boolean | app-server | Equivalent to `--listen stdio://`. |
| `--analytics-default-enabled` | boolean | app-server | First-party analytics default. |
| `--ws-auth` | `capability-token`, `signed-bearer-token` | app-server | Non-loopback WebSocket auth mode. |
| `--ws-token-file` | path | app-server | Capability token file. |
| `--ws-token-sha256` | hex | app-server | Capability token digest. |
| `--ws-shared-secret-file` | path | app-server | JWT shared secret file. |
| `--ws-issuer` | issuer | app-server | Expected JWT issuer. |
| `--ws-audience` | audience | app-server | Expected JWT audience. |
| `--ws-max-clock-skew-seconds` | seconds | app-server | JWT validation skew. |
| `--sock` | socket path | `app-server proxy` | Proxy to control socket. |
| `--out` | directory | `app-server generate-ts`, `generate-json-schema` | Required generation output directory. |
| `--prettier` | executable path | `app-server generate-ts` | Optional formatter. |
| `--experimental` | boolean | app-server generation | Include experimental protocol fields. |
| `--json` | boolean | `remote-control start/stop` | Machine-readable daemon result. |
| `--environment-id` | id | exec-server | Remote registration environment id. |
| `--name` | name | exec-server | Human-readable environment name. |
| `--use-agent-identity-auth` | boolean | exec-server | Uses `CODEX_ACCESS_TOKEN`. |

Doctor, sandbox, cloud, session, and execpolicy flags:

| Flag | Value | Scope | Example |
| --- | --- | --- | --- |
| `--summary` | boolean | doctor | `codex doctor --summary` |
| `--all` | boolean | doctor/session lookup | `codex doctor --all` |
| `--no-color` | boolean | doctor | `codex doctor --no-color` |
| `--ascii` | boolean | doctor | `codex doctor --ascii` |
| `-P`, `--permissions-profile` | profile | sandbox | `codex sandbox -P ci -- echo ok` |
| `--include-managed-config` | boolean | sandbox | `codex sandbox -P ci --include-managed-config -- echo ok` |
| `--allow-unix-socket` | path | sandbox on macOS | `codex sandbox --allow-unix-socket ./sock -- command` |
| `--log-denials` | boolean | sandbox on macOS | `codex sandbox --log-denials -- command` |
| `--download-url` | URL | app | `codex app --download-url https://example.test/app.dmg` |
| `--force` | boolean | delete | `codex delete --force <uuid>` |
| `--env` | environment id | `cloud exec`, `cloud list` | `codex cloud exec --env env_id "task"` |
| `--attempts` | number | cloud exec | `codex cloud exec --env env_id --attempts 2 "task"` |
| `--branch` | branch | cloud exec | `codex cloud exec --env env_id --branch main "task"` |
| `--limit` | 1-20 | cloud list | `codex cloud list --limit 10 --json` |
| `--cursor` | cursor | cloud list | `codex cloud list --json --cursor abc` |
| `--attempt` | number | cloud apply/diff | `codex cloud diff task_id --attempt 1` |
| `-r`, `--rules` | path | execpolicy check | `codex execpolicy check --rules policy.rules -- git status` |
| `--pretty` | boolean | execpolicy check | `codex execpolicy check --pretty --rules policy.rules -- git status` |
| `--resolve-host-executables` | boolean | execpolicy check | `codex execpolicy check --resolve-host-executables --rules policy.rules -- /usr/bin/git status` |

System-prompt delivery flags boundary: local `0.142.5` help does not expose dedicated `--append-system-prompt` or `--replace-system-prompt` style flags. The wrapper-relevant instruction surfaces I found are config overrides, for example `codex -c developer_instructions='Follow repo policy.'` and `codex -c model_instructions_file='./instructions.txt'`, plus `AGENTS.md` discovery. This topic records their existence only; replace-versus-append semantics, file-versus-inline behavior, and mode interactions belong to the sibling `system-prompt` topic.

## Configuration Discovery

Codex uses TOML for durable config. Official precedence is:

1. CLI flags and `--config` overrides.
2. Trusted project `.codex/config.toml` files, ordered from project root down to cwd with closest wins.
3. Profile file selected with `--profile`, stored next to user config as `$CODEX_HOME/<profile-name>.config.toml`.
4. User config at `$CODEX_HOME/config.toml`.
5. System config, documented as `/etc/codex/config.toml` on Unix.
6. Built-in defaults.

`CODEX_HOME` defaults to `~/.codex` and controls config, auth, logs, sessions, skills, plugin cache, standalone package metadata, and observed SQLite state. In this session, local `codex doctor --json` reported `CODEX_HOME` as `/Users/ken/.claudine/.codex`; many entries in that directory are symlinks to `/Users/ken/.codex`.

Project-local config is loaded only for trusted projects. Official docs say project-local config cannot override selected machine-local provider, auth, host-owned app request metadata, notification, configuration profile selection, or telemetry routing keys. Local config showed trust entries under `[projects."<path>"]` with `trust_level = "trusted"`.

Instruction discovery is broader than config:

| Scope | Path | Notes |
| --- | --- | --- |
| Global | `$CODEX_HOME/AGENTS.override.md` then `$CODEX_HOME/AGENTS.md` | Codex uses only the first non-empty file at this level. |
| Project | `AGENTS.override.md`, `AGENTS.md`, then configured fallback names along the path from project root to cwd | Codex includes at most one file per directory and stops at `project_doc_max_bytes`, default 32 KiB. |
| User custom agents | `$CODEX_HOME/agents/*.toml` | Custom subagent definitions. |
| Project custom agents | `.codex/agents/*.toml` | Project-scoped custom subagent definitions. |
| Deprecated prompts | `$CODEX_HOME/prompts/*.md` | Deprecated reusable slash-command prompts. |
| Execpolicy rules | `$CODEX_HOME/rules/default.rules`, `.codex/rules/` | Starlark rules; project rules require trusted project state. |
| Auth | `$CODEX_HOME/auth.json` | Stored credentials; not a normal hand-edited config file. |
| SQLite state | `$CODEX_HOME/state_5.sqlite`, `logs_2.sqlite`, `memories_1.sqlite`, `goals_1.sqlite` observed locally | Versioned names may change; `CODEX_SQLITE_HOME` or `sqlite_home` can move SQLite state. |

Wrapper-relevant first-run side effects include auth prompts and writes under `CODEX_HOME`, project trust prompts/state, session transcripts, logs, SQLite databases, plugin cache/state, MCP config, and installer metadata. `codex exec --ephemeral` avoids persisted session files for that run, but it still reads auth and other `CODEX_HOME` state.

## Environment Variables

General CLI/runtime variables:

| Variable | Effect |
| --- | --- |
| `CODEX_HOME` | Sets Codex state root, including config, auth, logs, sessions, skills, and standalone package metadata. The directory must already exist when set. |
| `CODEX_SQLITE_HOME` | Sets where SQLite-backed state is stored. `sqlite_home` config takes precedence. Relative paths resolve from cwd. |
| `CODEX_NON_INTERACTIVE` | For standalone install scripts, `1`, `true`, or `yes` skips installer prompts and uses defaults. |
| `CODEX_INSTALL_DIR` | Changes where standalone installers place the visible `codex` command. Defaults: `~/.local/bin` on macOS/Linux, `%LOCALAPPDATA%\Programs\OpenAI\Codex\bin` on Windows. |
| `CODEX_API_KEY` | Supplies an API key for a single `codex exec` run. Official docs recommend inline use rather than job-wide export around repository-controlled code. |
| `CODEX_ACCESS_TOKEN` | Supplies a ChatGPT or Codex access token for trusted automation. For persisted login, pipe it to `codex login --with-access-token`. |
| `CODEX_CA_CERTIFICATE` | PEM CA bundle for HTTPS, login, and WebSocket clients; takes precedence over `SSL_CERT_FILE`. |
| `SSL_CERT_FILE` | Fallback PEM CA bundle path when `CODEX_CA_CERTIFICATE` is unset. |
| `RUST_LOG` | Controls Rust log verbosity and filtering. `codex exec` defaults to error output unless set more verbosely. |

Provider endpoint variables, permission-specific config, MCP-specific server variables, logging topic details, and streaming behavior are intentionally left to their sibling research topics unless they also affect general CLI behavior.

## Machine Introspection

| Command | Machine-readable | Format | Useful for codegen | Notes |
| --- | --- | --- | --- | --- |
| `codex doctor --json` | Yes | JSON | Yes | Best all-up probe. Local output included `schemaVersion`, `codexVersion`, install paths, `CODEX_HOME`, config path, auth mode, model/provider, feature flags, MCP server count, Git/runtime/terminal info, update status, app-server state, and SQLite integrity checks. |
| `codex debug models [--bundled]` | Yes | JSON | Yes | Raw model catalog with slugs, display names, reasoning levels, service tiers, visibility, API support, and instruction metadata. Treat as sensitive and potentially large. |
| `codex mcp list --json` | Yes | JSON | Yes | Lists MCP servers. Local output showed one enabled streamable HTTP `github` server with token env var and auth status. |
| `codex mcp get --json <name>` | Yes | JSON | Yes | Raw MCP server entry. |
| `codex plugin list --json [--available]` | Yes | JSON | Yes | Local output showed installed `gmail@openai-curated` and `github@openai-curated` plugins. |
| `codex plugin add --json <plugin>` | Yes | JSON | No | Machine-readable mutation result; changes local plugin state. |
| `codex plugin remove --json <plugin>` | Yes | JSON | No | Machine-readable mutation result; changes local plugin state. |
| `codex plugin marketplace list --json` | Yes | JSON | Yes | Official reference documents JSON marketplace inventory. |
| `codex features list` | No | Table/text | Yes, with parser | Lists feature key, stage, and effective state. Local help has no `--json`. |
| `codex app-server generate-json-schema --out <dir> [--experimental]` | Yes | JSON files | Yes | Generates app-server protocol schema bundles. |
| `codex app-server generate-ts --out <dir> [--experimental]` | Yes | TypeScript files | Yes | Generates app-server protocol bindings. |
| `codex app-server daemon version` | Yes | JSON | No | Local help says it prints local CLI and running app-server versions as JSON. |
| `codex remote-control start --json` / `stop --json` | Yes | JSON | No | Machine-readable daemon control, but mutates daemon state. |
| `codex cloud list --json` | Yes | JSON | No | Lists cloud tasks with cursor. Requires cloud auth/state. |
| `codex execpolicy check --rules <file> [--pretty] -- <command>...` | Yes | JSON | Yes | Evaluates Starlark rule files and reports strictest decision and matches. |

Generic `--help` and `--version` are useful for probes but are not listed as machine introspection because they do not expose machine-usable provider state beyond version/help text.

## Wrapper Notes

Use `codex exec` for Claudine non-interactive sessions. Default `codex`, `resume`, `fork`, `login`, and `app` should be treated as interactive unless a specific non-interactive flag or stdin-secret flow is selected.

Prefer local help for accepted argv, but keep a compatibility exception list. Local `0.142.5` accepts `codex execpolicy` while omitting it from top-level help. The official reference documents `--full-auto` and `--experimental-json`, while local help omits them.

For structured automation, use `codex exec --json` for JSONL progress and `--output-last-message <file>` for the final assistant text. If a wrapper supplies both argv prompt and piped stdin, Codex treats stdin as extra context, not as the primary prompt.

Isolate wrapper runs with `CODEX_HOME` when deterministic behavior matters. It controls much more than config: auth, sessions, logs, plugin cache, skills, standalone package metadata, and observed SQLite state. Create the directory before launching Codex.

Use `--ignore-user-config` and `--ignore-rules` when inherited user config or execpolicy would make automation non-deterministic. Use `--ephemeral` when session persistence is unwanted, but do not assume it avoids reading auth or writing all other state.

Do not use `--dangerously-bypass-approvals-and-sandbox` or `--yolo` unless Claudine has placed the run in an external sandbox. The flag disables Codex's own approval prompts and sandboxing.

Treat `codex debug models` output as sensitive diagnostics. Local bundled output was very large and included embedded instruction text.

Project trust affects `.codex/config.toml`, `.codex/rules`, hooks, and project instructions. A wrapper running in an untrusted project may see materially different behavior from a trusted project.

No dedicated system-prompt CLI flags were found in local `0.142.5` help. Use config overrides such as `-c developer_instructions=...` or `-c model_instructions_file=...` only according to the sibling system-prompt topic's semantics.

`codex sandbox` is OS-specific. Local macOS help exposed `--allow-unix-socket` and `--log-denials`; Linux and Windows sandbox help should be inspected on those platforms before generating provider metadata for those flags.

## Changelog

- 2026-07-03: Revalidated installed `codex-cli 0.142.5` against npm `latest` and GitHub stable release metadata; noted newer alpha prereleases without treating them as stable latest.
- 2026-07-03: Expanded subcommand inventory with exec subcommands, login status, app-server/debug/plugin/cloud subordinate automation surfaces, and locally accepted `execpolicy`.
- 2026-07-03: Added wrapper-relevant switch coverage for hidden/documented compatibility flags, cloud flags, app-server generation flags, plugin/MCP JSON flags, session mutation flags, and execpolicy flags.
- 2026-07-03: Reworked configuration discovery into per-OS frontmatter records and documented `CODEX_HOME`, trusted project config, AGENTS discovery, custom agents, prompts, rules, auth, and SQLite state.
- 2026-07-03: Updated environment variables from official docs and expanded machine introspection around doctor, debug models, MCP/plugin JSON, app-server protocol generation, cloud list, and execpolicy check.

## Sources

- [Codex CLI homepage](https://developers.openai.com/codex/cli)
- [Codex CLI command reference](https://developers.openai.com/codex/cli/reference)
- [Codex configuration reference](https://developers.openai.com/codex/config-reference)
- [Codex config basics](https://developers.openai.com/codex/config-basic)
- [Codex environment variables](https://developers.openai.com/codex/environment-variables)
- [Codex non-interactive mode](https://developers.openai.com/codex/noninteractive)
- [Codex rules and execpolicy](https://developers.openai.com/codex/rules)
- [Codex subagents](https://developers.openai.com/codex/subagents)
- [openai/codex repository](https://github.com/openai/codex)
- [openai/codex releases](https://github.com/openai/codex/releases)
- Local command: `command -v codex; codex --version; codex --help`
- Local command: `codex <subcommand> --help` for `exec`, `review`, `login`, `logout`, `mcp`, `plugin`, `mcp-server`, `app-server`, `remote-control`, `app`, `completion`, `update`, `doctor`, `sandbox`, `debug`, `apply`, `resume`, `archive`, `delete`, `unarchive`, `fork`, `cloud`, `exec-server`, and `features`
- Local command: `codex mcp list --json`
- Local command: `codex plugin list --json`
- Local command: `codex debug models --bundled`
- Local command: `codex features list`
- Local command: `codex doctor --json`
- Local command: `codex execpolicy --help; codex execpolicy check --help`
- Local command: `npm view @openai/codex version dist-tags --json`
- Local inspection: `find ~/.codex -maxdepth 3 -type f -print`, `sed -n '1,240p' ~/.codex/config.toml`, and `ls -la ~/.codex`
