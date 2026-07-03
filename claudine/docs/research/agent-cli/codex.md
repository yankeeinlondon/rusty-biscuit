---
$schema: ./_schema.yaml
created: 2026-07-02
last_updated: 2026-07-02
agent: codex
model: default
latest_version: "0.142.5"
homepage: https://developers.openai.com/codex/cli
repo: https://github.com/openai/codex
docs: https://developers.openai.com/codex/
cli_docs: https://developers.openai.com/codex/cli/reference
binaries:
  - os: all
    binary: codex
    alt_binaries: []
    notes: "Primary executable. Local inspection on macOS found /Users/ken/.bun/bin/codex reporting codex-cli 0.142.5."
  - os: windows
    binary: codex.exe
    alt_binaries: ["codex.cmd"]
    notes: "The native command is documented as codex; .exe/.cmd shims are expected for Windows standalone/npm installs but were not locally inspected."
install_methods:
  - os: macos
    method: standalone_binary
    command: "curl -fsSL https://chatgpt.com/codex/install.sh | sh"
    notes: "Official standalone installer for macOS and Linux. Rerun to upgrade standalone installs."
  - os: linux
    method: standalone_binary
    command: "curl -fsSL https://chatgpt.com/codex/install.sh | sh"
    notes: "Official standalone installer for macOS and Linux. Rerun to upgrade standalone installs."
  - os: windows
    method: standalone_binary
    command: "powershell -ExecutionPolicy ByPass -c 'irm https://chatgpt.com/codex/install.ps1 | iex'"
    notes: "Official Windows standalone installer."
  - os: all
    method: npm
    command: "npm install -g @openai/codex"
    notes: "Documented package-manager install. Windows npm may expose a command shim."
  - os: macos
    method: brew
    command: "brew install --cask codex"
    notes: "Documented Homebrew cask install."
  - os: all
    method: standalone_binary
    command: "download from https://github.com/openai/codex/releases/latest"
    notes: "Release archives are platform-specific; macOS/Linux archives contain a platform-named executable that users commonly rename to codex."
subcommands:
  - name: interactive
    description: "Default mode when no subcommand is supplied; launches the terminal UI, optionally with an initial prompt."
    non_interactive: false
    notes: "Accepts global flags and image attachments; first run may prompt for authentication."
  - name: exec
    description: "Runs Codex non-interactively and exits."
    non_interactive: true
    notes: "Alias: e. Reads prompt from argv, stdin, or '-' and can emit JSONL with --json."
  - name: review
    description: "Runs a code review non-interactively."
    non_interactive: true
    notes: "Locally present in 0.142.5 help; supports uncommitted/base/commit review scopes."
  - name: login
    description: "Manages authentication."
    non_interactive: false
    notes: "OAuth/device flows are interactive; --with-api-key and --with-access-token read secrets from stdin."
  - name: logout
    description: "Removes stored authentication credentials."
    non_interactive: false
    notes: "May mutate CODEX_HOME auth state."
  - name: mcp
    description: "Manages external MCP servers."
    non_interactive: false
    notes: "Subcommands include list, get, add, remove, login, and logout. list/get support --json."
  - name: plugin
    description: "Manages Codex plugins."
    non_interactive: false
    notes: "Subcommands include add, list, marketplace, and remove. add/list support --json."
  - name: mcp-server
    description: "Starts Codex as an MCP server over stdio."
    non_interactive: true
    notes: "Intended for another agent or MCP client to consume Codex."
  - name: app-server
    description: "Runs the experimental local app server or related tooling."
    non_interactive: true
    notes: "Can listen on stdio, WebSocket, Unix socket, or off; also exposes schema/binding generation commands."
  - name: remote-control
    description: "Manages the app-server daemon with remote control enabled."
    non_interactive: true
    notes: "Experimental."
  - name: app
    description: "Launches the Codex desktop app or opens the app installer if missing."
    non_interactive: false
    notes: "Locally present in 0.142.5 help; not useful for headless wrappers."
  - name: completion
    description: "Generates shell completion scripts."
    non_interactive: true
    notes: "Supported shells: bash, zsh, fish, power-shell, and elvish."
  - name: update
    description: "Checks for and applies a Codex CLI update when supported by the installed release."
    non_interactive: false
    notes: "Mutates installation; debug builds only print guidance."
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
    notes: "debug models prints the raw model catalog as JSON; --bundled avoids refresh."
  - name: apply
    description: "Applies the latest diff produced by a Codex Cloud task to the local working tree."
    non_interactive: true
    notes: "Alias: a. Mutates the working tree."
  - name: resume
    description: "Resumes a previous interactive session."
    non_interactive: false
    notes: "Picker by default; can use --last."
  - name: archive
    description: "Archives a saved session by id or session name."
    non_interactive: true
    notes: "Mutates saved session state."
  - name: delete
    description: "Permanently deletes a saved session by id or session name."
    non_interactive: true
    notes: "Destructive state mutation."
  - name: unarchive
    description: "Restores an archived session by id or session name."
    non_interactive: true
    notes: "Mutates saved session state."
  - name: fork
    description: "Forks a previous interactive session into a new thread."
    non_interactive: false
    notes: "Picker by default; can use --last."
  - name: cloud
    description: "Browses or executes Codex Cloud tasks from the terminal."
    non_interactive: true
    notes: "Experimental. Subcommands include exec, status, list, apply, and diff."
  - name: exec-server
    description: "Runs the experimental standalone exec-server service."
    non_interactive: true
    notes: "Locally present in 0.142.5 help."
  - name: features
    description: "Inspects and mutates feature flags."
    non_interactive: true
    notes: "list is read-only; enable/disable persist changes in config.toml."
  - name: execpolicy
    description: "Evaluates execpolicy rule files."
    non_interactive: true
    notes: "Documented in the official reference, but not present in local 0.142.5 top-level help."
cli_switches:
  - flag: --config
    value: "<key=value>"
    scope: ["global", "config"]
    default: ""
    description: "Override a configuration value for this invocation; dotted paths are supported and values parse as TOML when possible."
    example: "codex -c model='gpt-5.5'"
    notes: "Short form: -c. Command-line overrides take precedence over config.toml."
  - flag: --enable
    value: "<FEATURE>"
    scope: ["global", "features"]
    default: ""
    description: "Enable a feature flag for this invocation."
    example: "codex --enable experimental_feature"
    notes: "Repeatable; equivalent to -c features.<name>=true."
  - flag: --disable
    value: "<FEATURE>"
    scope: ["global", "features"]
    default: ""
    description: "Disable a feature flag for this invocation."
    example: "codex --disable experimental_feature"
    notes: "Repeatable; equivalent to -c features.<name>=false."
  - flag: --strict-config
    value: ""
    scope: ["global", "config"]
    default: "false"
    description: "Error when config.toml contains fields this Codex version does not recognize."
    example: "codex --strict-config"
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
    description: "Attach one or more image files to the initial prompt."
    example: "codex -i screenshot.png 'implement this design'"
    notes: "Short form: -i. Docs also describe comma-separated paths."
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
    scope: ["global", "config"]
    default: ""
    description: "Layer $CODEX_HOME/<name>.config.toml on top of the base user config."
    example: "codex --profile work"
    notes: "Short form: -p. The features command does not accept --profile."
  - flag: --sandbox
    value: "read-only | workspace-write | danger-full-access"
    scope: ["interactive", "exec", "permissions"]
    default: "read-only"
    description: "Select the sandbox policy for model-generated shell commands."
    example: "codex exec --sandbox workspace-write 'run tests'"
    notes: "Short form: -s."
  - flag: --dangerously-bypass-approvals-and-sandbox
    value: ""
    scope: ["interactive", "exec", "permissions"]
    default: "false"
    description: "Run without approval prompts or sandboxing."
    example: "codex exec --dangerously-bypass-approvals-and-sandbox 'run in an external sandbox'"
    notes: "Alias: --yolo in official docs. Wrapper should only use inside an external sandbox."
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
    value: "untrusted | on-failure | on-request | never"
    scope: ["interactive", "permissions"]
    default: "config/default"
    description: "Configure when the model requires human approval before executing a command."
    example: "codex --ask-for-approval on-request"
    notes: "Short form: -a. on-failure is deprecated; exec help does not expose this flag."
  - flag: --search
    value: ""
    scope: ["interactive", "tools"]
    default: "cached"
    description: "Enable live web search."
    example: "codex --search 'research this dependency'"
    notes: "Official docs describe live search as TUI-oriented."
  - flag: --no-alt-screen
    value: ""
    scope: ["interactive", "terminal"]
    default: "false"
    description: "Disable alternate screen mode and keep TUI output inline."
    example: "codex --no-alt-screen"
    notes: "Overrides tui.alternate_screen for the run."
  - flag: --skip-git-repo-check
    value: ""
    scope: ["exec", "working_directory"]
    default: "false"
    description: "Allow codex exec to run outside a Git repository."
    example: "codex exec --skip-git-repo-check 'inspect this folder'"
    notes: "Local 0.142.5 exec-specific flag."
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
    scope: ["exec", "doctor", "mcp", "plugin", "cloud", "output"]
    default: "false"
    description: "Emit machine-readable JSON or JSONL where supported."
    example: "codex exec --json 'summarize'"
    notes: "exec emits JSONL events; doctor emits a JSON report; mcp/plugin/cloud subcommands emit JSON documents."
  - flag: --output-last-message
    value: "<FILE>"
    scope: ["exec", "output"]
    default: ""
    description: "Write the final assistant message to a file."
    example: "codex exec --json -o final.md 'summarize'"
    notes: "Short form: -o."
  - flag: --uncommitted
    value: ""
    scope: ["review"]
    default: "false"
    description: "Review staged, unstaged, and untracked changes."
    example: "codex review --uncommitted"
    notes: "Review-specific."
  - flag: --base
    value: "<BRANCH>"
    scope: ["review"]
    default: ""
    description: "Review changes against the given base branch."
    example: "codex review --base main"
    notes: "Review-specific."
  - flag: --commit
    value: "<SHA>"
    scope: ["review"]
    default: ""
    description: "Review the changes introduced by a commit."
    example: "codex review --commit HEAD"
    notes: "Review-specific."
  - flag: --title
    value: "<TITLE>"
    scope: ["review"]
    default: ""
    description: "Set an optional commit title to display in the review summary."
    example: "codex review --title 'Auth cleanup'"
    notes: "Review-specific."
  - flag: --with-api-key
    value: ""
    scope: ["login", "auth"]
    default: "false"
    description: "Read an API key from stdin for persisted login."
    example: "printenv OPENAI_API_KEY | codex login --with-api-key"
    notes: "Avoid interactive prompts but consumes a secret from stdin."
  - flag: --with-access-token
    value: ""
    scope: ["login", "auth"]
    default: "false"
    description: "Read an access token from stdin for persisted login."
    example: "printenv CODEX_ACCESS_TOKEN | codex login --with-access-token"
    notes: "Avoid interactive prompts but consumes a secret from stdin."
  - flag: --device-auth
    value: ""
    scope: ["login", "auth"]
    default: "false"
    description: "Use device authentication."
    example: "codex login --device-auth"
    notes: "Likely interactive/browser-adjacent; local help has no detailed description."
  - flag: --listen
    value: "stdio:// | ws://IP:PORT | unix:// | unix://PATH | off"
    scope: ["app-server"]
    default: "stdio://"
    description: "Select the app-server transport endpoint."
    example: "codex app-server --listen ws://127.0.0.1:1455"
    notes: "Official reference also mentions local development/debugging."
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
    notes: "Requires matching token/secret configuration."
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
  - flag: --all
    value: ""
    scope: ["doctor"]
    default: "false"
    description: "Expand long lists in detailed human-readable doctor output."
    example: "codex doctor --all"
    notes: "Doctor-specific."
  - flag: --ascii
    value: ""
    scope: ["doctor"]
    default: "false"
    description: "Use ASCII status labels and separators in human-readable doctor output."
    example: "codex doctor --ascii"
    notes: "Doctor-specific."
  - flag: --no-color
    value: ""
    scope: ["doctor"]
    default: "false"
    description: "Disable ANSI color in human-readable doctor output."
    example: "codex doctor --no-color"
    notes: "Doctor-specific."
  - flag: --summary
    value: ""
    scope: ["doctor"]
    default: "false"
    description: "Show grouped check rows and the final count summary only."
    example: "codex doctor --summary"
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
    scope: ["mcp add"]
    default: ""
    description: "Set an environment variable when launching a stdio MCP server."
    example: "codex mcp add server --env KEY=value -- command"
    notes: "Only valid with stdio MCP servers; repeatable."
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
    scope: ["mcp add", "mcp login"]
    default: ""
    description: "Set the OAuth resource parameter to include during MCP login."
    example: "codex mcp add docs --url https://mcp.example --oauth-resource resource"
    notes: "Requires --url for add."
  - flag: --scopes
    value: "<scope1,scope2>"
    scope: ["mcp login"]
    default: ""
    description: "Set OAuth scopes when logging into a streamable HTTP MCP server."
    example: "codex mcp login docs --scopes read,write"
    notes: "Only for servers that support OAuth."
  - flag: --marketplace
    value: "<MARKETPLACE>"
    scope: ["plugin add", "plugin list"]
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
config_files:
  - os: all
    scope: user
    path: "$CODEX_HOME/config.toml"
    format: toml
    notes: "Primary durable user config. Default CODEX_HOME is ~/.codex."
  - os: all
    scope: user
    path: "$CODEX_HOME/<profile-name>.config.toml"
    format: toml
    notes: "Profile layer selected with --profile/-p."
  - os: all
    scope: repo
    path: ".codex/config.toml"
    format: toml
    notes: "Project-scoped override loaded only for trusted projects. Provider/auth/telemetry/notification/profile keys are ignored in project-local config."
  - os: all
    scope: user
    path: "$CODEX_HOME/auth.json"
    format: json
    notes: "Stored authentication state; public docs discuss auth under CODEX_HOME but do not present this as a user-editable config file."
  - os: all
    scope: user
    path: "$CODEX_HOME/rules/default.rules"
    format: other
    notes: "User execpolicy rules."
  - os: all
    scope: repo
    path: ".codex/rules/"
    format: other
    notes: "Project execpolicy rules directory."
  - os: all
    scope: user
    path: "$CODEX_HOME/AGENTS.override.md"
    format: text
    notes: "Global override instructions discovered before global AGENTS.md."
  - os: all
    scope: user
    path: "$CODEX_HOME/AGENTS.md"
    format: text
    notes: "Global instructions."
  - os: all
    scope: repo
    path: "AGENTS.md"
    format: text
    notes: "Project instructions discovered from the repository root to cwd."
  - os: all
    scope: system
    path: "requirements.toml"
    format: toml
    notes: "Managed requirements/config reference exists in official config docs; exact discovery path depends on managed deployment and was not locally proven."
env_vars:
  - name: CODEX_HOME
    effect: "Sets the root for Codex state, including config, auth, logs, sessions, skills, and standalone package metadata. Default is ~/.codex; the directory must already exist when overridden."
  - name: CODEX_SQLITE_HOME
    effect: "Sets where SQLite-backed CLI/app-server state is stored. The sqlite_home config option takes precedence."
  - name: CODEX_NON_INTERACTIVE
    effect: "Installer-only variable that skips installer prompts when set to 1, true, or yes."
  - name: CODEX_INSTALL_DIR
    effect: "Standalone installer variable that changes where the visible codex command is installed."
  - name: CODEX_API_KEY
    effect: "Provides an API key for a single codex exec run; documented as supported only by codex exec."
  - name: CODEX_ACCESS_TOKEN
    effect: "Provides a ChatGPT or Codex access token for CLI/app-server/trusted automation; for persisted login, pipe it to codex login --with-access-token."
  - name: CODEX_CA_CERTIFICATE
    effect: "Points HTTPS/login/WebSocket clients at a PEM CA bundle and takes precedence over SSL_CERT_FILE."
  - name: SSL_CERT_FILE
    effect: "Fallback PEM CA bundle path when CODEX_CA_CERTIFICATE is unset."
  - name: RUST_LOG
    effect: "Controls Rust log filtering and verbosity. codex exec defaults to error output unless a more verbose value is set."
machine_introspection:
  - command: "codex debug models"
    purpose: models
    machine_readable: true
    output_format: json
    useful_for_codegen: true
    notes: "Prints the raw model catalog Codex sees. May refresh from configured sources."
  - command: "codex debug models --bundled"
    purpose: models
    machine_readable: true
    output_format: json
    useful_for_codegen: true
    notes: "Dumps only the bundled model catalog shipped with the binary; locally verified on 0.142.5."
  - command: "codex doctor --json"
    purpose: doctor
    machine_readable: true
    output_format: json
    useful_for_codegen: false
    notes: "Emits a redacted support report with schemaVersion, codexVersion, check statuses, and local diagnostics."
  - command: "codex mcp list --json"
    purpose: mcp
    machine_readable: true
    output_format: json
    useful_for_codegen: true
    notes: "Lists configured MCP servers."
  - command: "codex mcp get <name> --json"
    purpose: mcp
    machine_readable: true
    output_format: json
    useful_for_codegen: true
    notes: "Shows a specific MCP server configuration."
  - command: "codex plugin list --json"
    purpose: plugins
    machine_readable: true
    output_format: json
    useful_for_codegen: true
    notes: "Lists installed plugins."
  - command: "codex plugin list --available --json"
    purpose: plugins
    machine_readable: true
    output_format: json
    useful_for_codegen: true
    notes: "Includes uninstalled marketplace plugins in the JSON output."
  - command: "codex plugin add <plugin[@marketplace]> --json"
    purpose: plugins
    machine_readable: true
    output_format: json
    useful_for_codegen: false
    notes: "Installs a plugin and returns a JSON result; mutates config/cache."
  - command: "codex app-server generate-json-schema"
    purpose: config_schema
    machine_readable: true
    output_format: json
    useful_for_codegen: true
    notes: "Generates JSON Schema for the app-server protocol; experimental."
  - command: "codex app-server generate-ts"
    purpose: config_schema
    machine_readable: true
    output_format: text
    useful_for_codegen: true
    notes: "Generates TypeScript bindings for the app-server protocol; experimental."
  - command: "codex features list"
    purpose: capabilities
    machine_readable: false
    output_format: text
    useful_for_codegen: false
    notes: "Shows known feature flags, maturity stage, and effective state; no --json was found."
  - command: "codex cloud list --json"
    purpose: other
    machine_readable: true
    output_format: json
    useful_for_codegen: false
    notes: "Cloud task introspection is documented/available under experimental cloud commands, but requires account/cloud state."
  - command: "codex cloud status <task-id> --json"
    purpose: other
    machine_readable: true
    output_format: json
    useful_for_codegen: false
    notes: "Cloud task status introspection; requires account/cloud state."
wrapper_notes:
  - "Use `codex exec`/`codex e` for headless task execution. It reads stdin when the prompt is omitted or `-`, appends piped stdin when a prompt is also supplied, and exits after completion."
  - "`codex exec --json` writes JSONL events to stdout. Official safety guidance recommends pairing `--json` with `--output-last-message` in CI to capture both progress and the final assistant text."
  - "`CODEX_API_KEY` is documented as supported only by `codex exec`; wrappers should set it narrowly for the child process rather than job-wide when running repository-controlled code."
  - "`CODEX_HOME` overrides require an existing directory. Local probes with a nonexistent CODEX_HOME produced warnings or config-load failures before/around otherwise machine-readable output."
  - "Piping large Codex JSON output into a reader that closes early, such as `head`, can trigger a Rust broken-pipe panic message. Wrappers should drain stdout/stderr or terminate intentionally."
  - "Login, MCP OAuth login, update, plugin install/remove, feature enable/disable, and session delete/archive commands mutate user state and may prompt or require a TTY/browser."
  - "Interactive `codex` may prompt for first-run authentication. Prefer pre-authenticated CODEX_HOME, `CODEX_API_KEY` for exec, or explicit stdin-based login flows for automation."
  - "`--dangerously-bypass-approvals-and-sandbox`/`--yolo` removes both approval prompts and sandboxing; only use when Claudine or the host already supplies the sandbox boundary."
  - "Project `.codex/config.toml` is loaded only for trusted projects and cannot override machine-local provider/auth/telemetry/notification/profile keys."
  - "Official reference documents `execpolicy`, but local 0.142.5 top-level help did not expose it. Treat it as a documentation/source caveat until reverified."
changes: []
requires_claudine_update: true
reason: "Existing Codex research was pre-schema and stale; current public/local CLI surface adds schema fields, version 0.142.5, install paths, command/flag inventory, machine introspection commands, and wrapper caveats that should update generated provider metadata and wrappers."
---

# Codex CLI

## Overview

Codex CLI is OpenAI's open-source terminal coding agent. The official CLI docs describe it as a local terminal client that can inspect repositories, edit files, and run commands. The public executable is `codex`.

This research was verified on 2026-07-02 against the official OpenAI Developers docs, the public `openai/codex` repository, and local CLI inspection. The locally installed binary at `/Users/ken/.bun/bin/codex` reported `codex-cli 0.142.5`, matching the latest GitHub release visible during research.

## Installation and Binaries

The primary command is `codex` on macOS, Linux, and Windows. The standalone Windows installer and npm installs are expected to create platform-specific shims such as `codex.exe` or `codex.cmd`, but the official command spelling remains `codex`.

Official installation methods:

| OS | Method | Command |
| --- | --- | --- |
| macOS/Linux | standalone installer | `curl -fsSL https://chatgpt.com/codex/install.sh \| sh` |
| macOS/Linux | unattended standalone installer | `curl -fsSL https://chatgpt.com/codex/install.sh \| CODEX_NON_INTERACTIVE=1 sh` |
| Windows | standalone installer | `powershell -ExecutionPolicy ByPass -c "irm https://chatgpt.com/codex/install.ps1 \| iex"` |
| all | npm | `npm install -g @openai/codex` |
| macOS | Homebrew | `brew install --cask codex` |
| all | release archive | download from the latest GitHub release |

Standalone installer defaults are controlled by `CODEX_INSTALL_DIR`: `~/.local/bin` on macOS/Linux and `%LOCALAPPDATA%\Programs\OpenAI\Codex\bin` on Windows. The standalone package cache remains under `$CODEX_HOME/packages/standalone`.

## Subcommands

Local 0.142.5 help exposes these top-level commands:

`exec`, `review`, `login`, `logout`, `mcp`, `plugin`, `mcp-server`, `app-server`, `remote-control`, `app`, `completion`, `update`, `doctor`, `sandbox`, `debug`, `apply`, `resume`, `archive`, `delete`, `unarchive`, `fork`, `cloud`, `exec-server`, `features`, and `help`.

The default `codex` mode launches the interactive terminal UI. `codex exec` is the primary non-interactive wrapper entry point. `codex review` is also non-interactive for code review. `doctor --json`, `debug models`, `mcp list/get --json`, and plugin JSON modes are useful state-discovery surfaces.

The official command reference also lists `codex execpolicy`, but local 0.142.5 top-level help did not expose that command. Treat `execpolicy` as a documented-but-not-locally-present caveat until the source or a newer binary is rechecked.

## CLI Switch Inventory

The frontmatter contains the switch inventory with scope, values, defaults where documented or locally printed, examples, and wrapper notes.

Important wrapper-facing groups:

- Global/config: `--config/-c`, `--enable`, `--disable`, `--strict-config`, `--profile/-p`.
- Runtime/model: `--model/-m`, `--oss`, `--local-provider`, `--image/-i`.
- Permissions: `--sandbox/-s`, `--ask-for-approval/-a`, `--add-dir`, `--dangerously-bypass-approvals-and-sandbox`/`--yolo`, `--dangerously-bypass-hook-trust`.
- Non-interactive exec: `--skip-git-repo-check`, `--ephemeral`, `--ignore-user-config`, `--ignore-rules`, `--output-schema`, `--color`, `--json`, `--output-last-message/-o`.
- Machine-readable state: `doctor --json`, `mcp list/get --json`, `plugin list/add --json`, `debug models`.
- App-server/remote: `--listen`, `--stdio`, `--remote`, `--remote-auth-token-env`, `--ws-*`.

## Configuration Discovery

Codex uses TOML for durable configuration. User config lives at `$CODEX_HOME/config.toml`, defaulting to `~/.codex/config.toml`. Profiles live next to it as `$CODEX_HOME/<profile-name>.config.toml` and are selected with `--profile/-p`.

Project-local `.codex/config.toml` files are loaded only for trusted projects. Official docs state that project config cannot override machine-local provider, auth, app request metadata, notification, profile selection, or telemetry routing keys; those belong in user-level config.

Instruction discovery includes `$CODEX_HOME/AGENTS.override.md`, `$CODEX_HOME/AGENTS.md`, and project `AGENTS.md` files walked from repository root to the current directory. Execpolicy rules are discovered from `$CODEX_HOME/rules/default.rules` and `.codex/rules/`.

## Environment Variables

General public variables from the official environment-variable reference are recorded in frontmatter. `CODEX_HOME`, `CODEX_SQLITE_HOME`, installer variables, auth/network variables, and `RUST_LOG` are the relevant wrapper-level variables.

Do not treat arbitrary provider API key names as fixed Codex env vars. Codex can read provider-specific secrets through configured `env_key` names, but those belong to model-provider configuration rather than the general CLI surface.

## Machine Introspection

Useful machine-readable commands:

| Command | Format | Use |
| --- | --- | --- |
| `codex debug models` | JSON | Effective/raw model catalog. |
| `codex debug models --bundled` | JSON | Bundled model catalog shipped with the binary. |
| `codex doctor --json` | JSON | Redacted diagnostic report and installed version. |
| `codex mcp list --json` | JSON | Configured MCP servers. |
| `codex mcp get <name> --json` | JSON | One MCP server definition. |
| `codex plugin list --json` | JSON | Installed plugins. |
| `codex plugin list --available --json` | JSON | Marketplace plugin inventory. |
| `codex app-server generate-json-schema` | JSON | App-server protocol schema. |
| `codex app-server generate-ts` | TypeScript text | App-server protocol bindings. |

`codex features list` is useful for diagnostics but no JSON mode was found. `codex cloud` commands may have JSON modes useful for cloud task state, but they require account/cloud state and are not stable wrapper metadata.

## Wrapper Notes

Use `codex exec` for automated runs. It can read an inline prompt, read stdin, or use `-` as a stdin prompt sentinel. For CI, prefer `--json` plus `--output-last-message` so the wrapper can consume progress events while preserving the final assistant message.

Authentication and state handling need care. `CODEX_API_KEY` is documented for one `codex exec` run only. `CODEX_HOME` is powerful but must point to an existing directory; invalid homes can produce warnings or config-load errors that interfere with clean parsing. Login, OAuth, update, plugin mutation, feature mutation, and delete/archive commands should be considered interactive or stateful unless explicitly wrapped.

Wrappers should drain stdout/stderr or control termination explicitly. Local probing showed that closing stdout early with `head` can surface a Rust broken-pipe panic message.

## Sources

- [Codex CLI overview](https://developers.openai.com/codex/cli)
- [Codex CLI command reference](https://developers.openai.com/codex/cli/reference)
- [Codex config reference](https://developers.openai.com/codex/config-reference)
- [Codex environment variables](https://developers.openai.com/codex/environment-variables)
- [Codex non-interactive mode](https://developers.openai.com/codex/noninteractive)
- [openai/codex GitHub repository](https://github.com/openai/codex)
- Local inspection: `codex --version`, `codex --help`, and selected subcommand help against `codex-cli 0.142.5` on macOS.
