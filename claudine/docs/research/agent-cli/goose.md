---
$schema: ./_schema.yaml
created: 2026-04-27
last_updated: 2026-07-02
agent: codex
model: default
latest_version: "v1.41.0"
homepage: https://goose-docs.ai/
repo: https://github.com/aaif-goose/goose
docs: https://goose-docs.ai/docs/
cli_docs: https://goose-docs.ai/docs/guides/goose-cli-commands/
binaries:
  - os: macos
    binary: goose
    alt_binaries: []
    notes: "Installed by the shell installer or Homebrew formula `block-goose-cli`; release asset names use `goose-<arch>-apple-darwin.tar.bz2`."
  - os: linux
    binary: goose
    alt_binaries: []
    notes: "Installed by the shell installer; release asset names include GNU, Vulkan, and musl variants."
  - os: windows
    binary: goose.exe
    alt_binaries: ["goose"]
    notes: "Native Windows release asset is `goose-x86_64-pc-windows-msvc.zip`; PowerShell and Git Bash/MSYS installs place the executable in a user bin directory."
install_methods:
  - os: macos
    method: other
    command: "curl -fsSL https://github.com/aaif-goose/goose/releases/download/stable/download_cli.sh | bash"
    notes: "Official CLI download script. Set `CONFIGURE=false` to skip the interactive configure step; set `GOOSE_VERSION` to pin a release."
  - os: macos
    method: brew
    command: "brew install block-goose-cli"
    notes: "Official Homebrew CLI formula."
  - os: linux
    method: other
    command: "curl -fsSL https://github.com/aaif-goose/goose/releases/download/stable/download_cli.sh | bash"
    notes: "Official CLI download script. Supports `GOOSE_LINUX_VARIANT=standard|vulkan|musl` and `GOOSE_VERSION` pinning."
  - os: windows
    method: other
    command: "curl -fsSL https://github.com/aaif-goose/goose/releases/download/stable/download_cli.sh | bash"
    notes: "Official Git Bash/MSYS2 install path. Native Windows installs default to a user bin directory."
  - os: windows
    method: other
    command: "Invoke-WebRequest -Uri \"https://raw.githubusercontent.com/aaif-goose/goose/main/download_cli.ps1\" -OutFile \"download_cli.ps1\"; .\\download_cli.ps1"
    notes: "Official PowerShell installer. Supports `GOOSE_WINDOWS_VARIANT=standard|cuda`, `GOOSE_VERSION`, and `CONFIGURE=false`."
subcommands:
  - name: configure
    description: "Configures providers, extensions, and other Goose settings interactively."
    non_interactive: false
    notes: "May open a browser for provider login and may prompt for credentials."
  - name: info
    description: "Prints Goose version, config paths, session storage, logs, optional config values, and optional provider check status."
    non_interactive: true
    notes: "`--verbose` prints merged config values as YAML-like text; `--check` performs a provider request."
  - name: doctor
    description: "Starts an interactive no-session diagnostic flow by sending `/doctor` into a Goose session."
    non_interactive: false
    notes: "Source-backed command; not a machine-readable doctor report."
  - name: mcp
    description: "Runs a bundled MCP server by name."
    non_interactive: true
    notes: "Server names are parsed by Goose MCP source; docs show usage as `goose mcp <name>`."
  - name: acp
    description: "Runs Goose as an ACP agent server over stdio."
    non_interactive: true
    notes: "Accepts `--with-builtin`."
  - name: serve
    description: "Starts an ACP server over HTTP and WebSocket."
    non_interactive: true
    notes: "Requires `GOOSE_SERVER__SECRET_KEY` unless `--dangerously-unauthenticated` is used."
  - name: session
    description: "Starts, resumes, forks, edits, lists, removes, imports, exports, or diagnoses interactive sessions."
    non_interactive: false
    notes: "Alias: `s`. Subcommands `list`, `export`, `import`, and `diagnostics` can be used non-interactively when identifiers are supplied."
  - name: project
    description: "Opens the last project directory."
    non_interactive: false
    notes: "Alias: `p`; may launch a file manager or change user focus."
  - name: projects
    description: "Lists recent project directories."
    non_interactive: false
    notes: "Alias: `ps`; source routes to an interactive project picker."
  - name: run
    description: "Executes a prompt, instruction file, stdin, or recipe and exits unless `--interactive` is supplied."
    non_interactive: true
    notes: "Primary wrapper entry point; supports `--output-format json` and `--output-format stream-json`."
  - name: recipe
    description: "Recipe utilities: validate, deeplink, open in Goose Desktop, and list."
    non_interactive: true
    notes: "`recipe open` launches Goose Desktop; `recipe list --format json` is machine-readable."
  - name: skills
    description: "Skill utilities."
    non_interactive: true
    notes: "Current source exposes `skills list` only."
  - name: plugin
    description: "Installs or updates Git-backed Goose plugins."
    non_interactive: false
    notes: "Runs Git/network operations and may be affected by credential prompts in Git."
  - name: schedule
    description: "Manages scheduled recipe jobs."
    non_interactive: true
    notes: "Alias: `sched`; `services-status` and `services-stop` are deprecated."
  - name: gateway
    description: "Manages external platform gateways such as Telegram."
    non_interactive: false
    notes: "Alias: `gw`; start/pair flows may require external credentials or user action."
  - name: update
    description: "Updates the Goose CLI."
    non_interactive: false
    notes: "Feature-gated in source; may run configuration after updating."
  - name: term
    description: "Terminal-integrated session helper with shell init, prompt run, and prompt-status info."
    non_interactive: false
    notes: "`term init` and `term info` are scriptable; `term run` sends prompts into a persistent terminal session."
  - name: tui
    description: "Launches the Goose terminal UI."
    non_interactive: false
    notes: "Feature-gated; resolves `GOOSE_TUI_SCRIPT` or runs npm/npx."
  - name: local-models
    description: "Searches, downloads, lists, and deletes local inference models."
    non_interactive: true
    notes: "Feature-gated; alias: `lm`."
  - name: completion
    description: "Generates shell completions."
    non_interactive: true
    notes: "Supports bash, elvish, fish, nu/nushell, powershell/pwsh, and zsh."
  - name: review
    description: "Reviews the current diff using Goose and optional `.agents/checks/*.md` subagent reviewers."
    non_interactive: true
    notes: "Source-backed command; may spawn orchestrated `goose run` subprocesses."
  - name: validate-extensions
    description: "Validates a bundled-extensions.json file."
    non_interactive: true
    notes: "Hidden source-backed command."
cli_switches:
  - flag: --help
    value: ""
    scope: ["global"]
    default: "false"
    description: "Prints help."
    example: "goose --help"
    notes: "Clap-generated global switch."
  - flag: --version
    value: ""
    scope: ["global"]
    default: "false"
    description: "Prints installed Goose version."
    example: "goose --version"
    notes: "Clap-generated global switch."
  - flag: -v, --verbose
    value: ""
    scope: ["info"]
    default: "false"
    description: "Shows detailed configuration settings, including merged config values."
    example: "goose info --verbose"
    notes: "Output is text, not declared JSON."
  - flag: --check
    value: ""
    scope: ["info"]
    default: "false"
    description: "Tests provider connection and prints status."
    example: "goose info --check"
    notes: "Performs a real provider request."
  - flag: --with-builtin
    value: "<NAME[,NAME...]>"
    scope: ["acp", "serve", "session", "run"]
    default: ""
    description: "Adds one or more bundled extensions by name."
    example: "goose run --with-builtin developer -t \"summarize this repo\""
    notes: "Comma-delimited in source; `serve` also allows repeated occurrences."
  - flag: --host
    value: "<HOST>"
    scope: ["serve"]
    default: "127.0.0.1"
    description: "Host/interface for the ACP HTTP/WebSocket server."
    example: "goose serve --host 127.0.0.1"
    notes: ""
  - flag: --port
    value: "<PORT>"
    scope: ["serve"]
    default: "3284"
    description: "Port for the ACP HTTP/WebSocket server."
    example: "goose serve --port 3284"
    notes: ""
  - flag: --tls
    value: ""
    scope: ["serve"]
    default: "false"
    description: "Serves ACP over TLS."
    example: "goose serve --tls --tls-cert-path cert.pem --tls-key-path key.pem"
    notes: ""
  - flag: --tls-cert-path
    value: "<PATH>"
    scope: ["serve"]
    default: ""
    description: "TLS certificate path for `goose serve`."
    example: "goose serve --tls-cert-path cert.pem"
    notes: ""
  - flag: --tls-key-path
    value: "<PATH>"
    scope: ["serve"]
    default: ""
    description: "TLS private key path for `goose serve`."
    example: "goose serve --tls-key-path key.pem"
    notes: ""
  - flag: --platform
    value: "<cli|desktop>"
    scope: ["serve"]
    default: "cli"
    description: "Selects the served Goose platform identity."
    example: "goose serve --platform cli"
    notes: "Source enum values are `cli` and `desktop`."
  - flag: --dangerously-unauthenticated
    value: ""
    scope: ["serve"]
    default: "false"
    description: "Starts the ACP endpoint without requiring `GOOSE_SERVER__SECRET_KEY`."
    example: "goose serve --dangerously-unauthenticated"
    notes: "Wrapper should avoid setting this automatically."
  - flag: --allowed-origin
    value: "<ORIGIN>"
    scope: ["serve"]
    default: ""
    description: "Allows an exact Origin value for ACP CORS; repeatable."
    example: "goose serve --allowed-origin http://localhost:3000"
    notes: "Replaces default loopback origins when supplied."
  - flag: -n, --name
    value: "<NAME>"
    scope: ["session", "run", "session remove", "session export", "session diagnostics", "review/identifier"]
    default: ""
    description: "Names or identifies a Goose session."
    example: "goose session --name my-project"
    notes: "Shared identifier group."
  - flag: --session-id
    value: "<SESSION_ID>"
    scope: ["session", "run", "session remove", "session export", "session diagnostics"]
    default: ""
    description: "Identifies a Goose session by ID."
    example: "goose session --resume --session-id 20251108_2"
    notes: "Alias: `--id` in source."
  - flag: --path
    value: "<PATH>"
    scope: ["session", "run", "session remove", "session export", "session diagnostics"]
    default: ""
    description: "Legacy path-based session identifier."
    example: "goose session --resume --path ./session.jsonl"
    notes: "Kept for legacy JSONL/session import paths."
  - flag: -r, --resume
    value: ""
    scope: ["session", "run"]
    default: "false"
    description: "Resumes a previous session or run."
    example: "goose run --resume -t \"continue\""
    notes: "With no identifier, source resumes the most recent session."
  - flag: --fork
    value: ""
    scope: ["session"]
    default: "false"
    description: "Forks a previous session; requires `--resume`."
    example: "goose session --resume --fork --history"
    notes: ""
  - flag: --edit
    value: ""
    scope: ["session"]
    default: "false"
    description: "Opens the session conversation in `$VISUAL`, `$EDITOR`, or `vi` before resuming; requires `--resume`."
    example: "goose session --resume --session-id 20251108_2 --edit"
    notes: "Requires an editor and is not suitable for non-interactive wrappers."
  - flag: --history
    value: ""
    scope: ["session"]
    default: "false"
    description: "Shows previous messages when resuming a session."
    example: "goose session --resume --history"
    notes: "Requires `--resume`."
  - flag: --debug
    value: ""
    scope: ["session", "run"]
    default: "false"
    description: "Shows complete tool responses, detailed parameters, and full paths."
    example: "goose run --debug -t \"inspect failing tests\""
    notes: "May expose sensitive data in output."
  - flag: --max-tool-repetitions
    value: "<NUMBER>"
    scope: ["session", "run"]
    default: ""
    description: "Maximum consecutive identical tool calls allowed."
    example: "goose run --max-tool-repetitions 3 -t \"fix loop\""
    notes: ""
  - flag: --max-turns
    value: "<NUMBER>"
    scope: ["session", "run"]
    default: "1000"
    description: "Maximum turns allowed without user input."
    example: "goose run --max-turns 10 -t \"make a small change\""
    notes: "Can also be controlled by `GOOSE_MAX_TURNS`."
  - flag: --container
    value: "<CONTAINER_ID>"
    scope: ["session", "run"]
    default: ""
    description: "Runs extensions inside the specified Docker container."
    example: "goose run --container devbox -t \"run tests\""
    notes: "Requires Docker/container environment."
  - flag: --with-extension
    value: "<COMMAND>"
    scope: ["session", "run"]
    default: ""
    description: "Adds stdio extensions from full commands; repeatable."
    example: "goose run --with-extension \"npx -y @modelcontextprotocol/server-memory\" -t \"remember this\""
    notes: "Command string may include env assignments and arguments."
  - flag: --with-streamable-http-extension
    value: "<URL>"
    scope: ["session", "run"]
    default: ""
    description: "Adds streamable HTTP extensions; repeatable."
    example: "goose run --with-streamable-http-extension \"http://localhost:8080/mcp\" -t \"use the server\""
    notes: "Source parser also accepts `timeout=<seconds>` in the value."
  - flag: --no-profile
    value: ""
    scope: ["session", "run"]
    default: "false"
    description: "Skips default profile extensions and uses only CLI-specified extensions."
    example: "goose run --no-profile --with-builtin developer -t \"inspect\""
    notes: ""
  - flag: -f, --format
    value: "<text|json>"
    scope: ["session list"]
    default: "text"
    description: "Selects session list output format."
    example: "goose session list --format json"
    notes: "Machine-readable when `json`."
  - flag: --ascending
    value: ""
    scope: ["session list"]
    default: "false"
    description: "Sorts sessions oldest first."
    example: "goose session list --ascending"
    notes: "Default order is descending/newest first."
  - flag: -w, --working_dir
    value: "<PATH>"
    scope: ["session list"]
    default: ""
    description: "Filters sessions by working directory."
    example: "goose session list --working_dir ~/src/project"
    notes: "Short alias `-p` exists in source."
  - flag: -l, --limit
    value: "<NUMBER>"
    scope: ["session list", "schedule sessions", "local-models search"]
    default: ""
    description: "Limits number of results."
    example: "goose session list --limit 10"
    notes: "`local-models search` defaults to 10."
  - flag: -r, --regex
    value: "<PATTERN>"
    scope: ["session remove"]
    default: ""
    description: "Removes sessions matching a regex."
    example: "goose session remove --regex \"project-.*\""
    notes: "Removal can prompt for confirmation."
  - flag: -o, --output
    value: "<FILE>"
    scope: ["session export", "session diagnostics"]
    default: "stdout for export; diagnostics_<session_id>.json for diagnostics"
    description: "Writes export or diagnostics output to a file."
    example: "goose session export -n my-session --format json --output session.json"
    notes: ""
  - flag: --format
    value: "<markdown|json|yaml>"
    scope: ["session export"]
    default: "markdown"
    description: "Selects session export format."
    example: "goose session export --session-id 20251108_4 --format json"
    notes: "JSON and YAML are machine-readable."
  - flag: --nostr
    value: ""
    scope: ["session export", "session import"]
    default: "false"
    description: "Publishes an encrypted Nostr session share link or treats import input as such."
    example: "goose session export --format json --nostr"
    notes: "Network side effects."
  - flag: --relay
    value: "<RELAY>"
    scope: ["session export"]
    default: ""
    description: "Nostr relay URL; repeatable."
    example: "goose session export --nostr --relay wss://relay.example"
    notes: ""
  - flag: -i, --instructions
    value: "<FILE>"
    scope: ["run"]
    default: ""
    description: "Path to an instruction file; use `-` for stdin."
    example: "goose run --instructions -"
    notes: "Conflicts with `--text` and `--recipe`."
  - flag: -t, --text
    value: "<TEXT>"
    scope: ["run"]
    default: ""
    description: "Input text to provide directly to Goose."
    example: "goose run --text \"summarize this repo\""
    notes: "Conflicts with `--instructions` and `--recipe`."
  - flag: --recipe
    value: "<RECIPE_NAME|PATH>"
    scope: ["run"]
    default: ""
    description: "Runs a recipe by configured name or file path."
    example: "goose run --recipe ./recipe.yaml"
    notes: "Conflicts with `--instructions` and `--text`."
  - flag: --system
    value: "<TEXT>"
    scope: ["run"]
    default: ""
    description: "Adds system instructions for the run."
    example: "goose run --system \"Be concise\" -t \"summarize\""
    notes: "Conflicts with `--recipe`."
  - flag: --params
    value: "<KEY=VALUE>"
    scope: ["run", "schedule add"]
    default: ""
    description: "Supplies recipe parameters; repeatable."
    example: "goose run --recipe deploy.yaml --params env=prod"
    notes: ""
  - flag: --sub-recipe
    value: "<RECIPE>"
    scope: ["run"]
    default: ""
    description: "Includes additional sub-recipes; repeatable."
    example: "goose run --recipe main.yaml --sub-recipe audit.yaml"
    notes: ""
  - flag: --explain
    value: ""
    scope: ["run"]
    default: "false"
    description: "Shows recipe title, description, and parameters."
    example: "goose run --recipe build.yaml --explain"
    notes: ""
  - flag: --render-recipe
    value: ""
    scope: ["run"]
    default: "false"
    description: "Prints the rendered recipe instead of running it."
    example: "goose run --recipe build.yaml --render-recipe"
    notes: ""
  - flag: -s, --interactive
    value: ""
    scope: ["run"]
    default: "false"
    description: "Continues in interactive mode after processing initial input."
    example: "goose run -t \"start by inspecting failures\" --interactive"
    notes: "Wrappers should avoid this for batch runs."
  - flag: --no-session
    value: ""
    scope: ["run"]
    default: "false"
    description: "Runs without creating or using a session file."
    example: "goose run --no-session --output-format stream-json -t \"summarize\""
    notes: "Conflicts with `--resume`, `--name`, and `--path`."
  - flag: --stats
    value: ""
    scope: ["run"]
    default: "false"
    description: "Prints generation statistics after the run completes."
    example: "goose run --stats -t \"summarize\""
    notes: ""
  - flag: --scheduled-job-id
    value: "<ID>"
    scope: ["run"]
    default: ""
    description: "Associates a run with a scheduled job."
    example: "goose run --scheduled-job-id daily-report --recipe report.yaml"
    notes: "Hidden internal flag."
  - flag: -q, --quiet
    value: ""
    scope: ["run", "review"]
    default: "false"
    description: "Suppresses non-response output."
    example: "goose run --quiet -t \"answer only\""
    notes: "Important when wrappers need clean text output."
  - flag: --output-format
    value: "<text|json|stream-json>"
    scope: ["run"]
    default: "text"
    description: "Selects run output format."
    example: "goose run --output-format stream-json -t \"summarize this repo\""
    notes: "`stream-json` is newline-delimited JSON events."
  - flag: --provider
    value: "<PROVIDER>"
    scope: ["run", "review"]
    default: ""
    description: "Overrides provider for the run or review."
    example: "goose run --provider anthropic --model claude-sonnet-4-20250514 -t \"inspect\""
    notes: "Overrides `GOOSE_PROVIDER` for the invocation."
  - flag: --model
    value: "<MODEL>"
    scope: ["run", "review"]
    default: ""
    description: "Overrides model for the run or review."
    example: "goose run --provider openai --model gpt-4.1 -t \"summarize\""
    notes: "Overrides `GOOSE_MODEL` for the invocation."
  - flag: -p, --param
    value: "<KEY=VALUE>"
    scope: ["recipe deeplink", "recipe open"]
    default: ""
    description: "Supplies recipe parameters; repeatable."
    example: "goose recipe deeplink my-recipe --param env=prod"
    notes: ""
  - flag: --format
    value: "<text|json>"
    scope: ["recipe list"]
    default: "text"
    description: "Selects recipe list output format."
    example: "goose recipe list --format json"
    notes: "Machine-readable when `json`."
  - flag: -v, --verbose
    value: ""
    scope: ["recipe list"]
    default: "false"
    description: "Shows verbose recipe information including descriptions."
    example: "goose recipe list --verbose"
    notes: ""
  - flag: --auto-update
    value: ""
    scope: ["plugin install"]
    default: "false"
    description: "Marks an installed plugin for automatic update checks before plugin skills are loaded."
    example: "goose plugin install --auto-update https://github.com/example/plugin.git"
    notes: ""
  - flag: --schedule-id
    value: "<ID>"
    scope: ["schedule add", "schedule remove", "schedule sessions", "schedule run-now"]
    default: ""
    description: "Identifies a scheduled recipe job."
    example: "goose schedule run-now --schedule-id daily-report"
    notes: "Alias: `--id` in source."
  - flag: --cron
    value: "<EXPR>"
    scope: ["schedule add"]
    default: ""
    description: "Cron expression for a scheduled job."
    example: "goose schedule add --schedule-id daily --cron \"0 9 * * *\" --recipe-source ./daily.yaml"
    notes: ""
  - flag: --recipe-source
    value: "<PATH|BASE64>"
    scope: ["schedule add"]
    default: ""
    description: "Recipe source path or base64-encoded recipe string."
    example: "goose schedule add --schedule-id daily --cron \"0 9 * * *\" --recipe-source ./daily.yaml"
    notes: ""
  - flag: --bot-token
    value: "<TOKEN>"
    scope: ["gateway start"]
    default: ""
    description: "Gateway platform bot token."
    example: "goose gateway start telegram --bot-token \"$TOKEN\""
    notes: "Secret-bearing argument."
  - flag: --canary
    value: ""
    scope: ["update"]
    default: "false"
    description: "Updates to the canary release instead of stable."
    example: "goose update --canary"
    notes: "Feature-gated update command."
  - flag: -r, --reconfigure
    value: ""
    scope: ["update"]
    default: "false"
    description: "Forces reconfiguration during update."
    example: "goose update --reconfigure"
    notes: "May prompt interactively."
  - flag: --bin-name
    value: "<NAME>"
    scope: ["completion"]
    default: "goose"
    description: "Uses a custom binary name in generated completions."
    example: "goose completion zsh --bin-name goose"
    notes: ""
  - flag: -n, --name
    value: "<NAME>"
    scope: ["term init"]
    default: ""
    description: "Names the terminal-integrated session."
    example: "goose term init zsh --name work"
    notes: ""
  - flag: --default
    value: ""
    scope: ["term init"]
    default: "false"
    description: "Makes Goose the default handler for unknown shell commands."
    example: "goose term init zsh --default"
    notes: "Supported for zsh, bash, and nu according to source help."
  - flag: --prompt
    value: "<FILE>"
    scope: ["review"]
    default: ""
    description: "Path to a Markdown file with a custom base review prompt."
    example: "goose review --prompt REVIEW.md"
    notes: ""
  - flag: --override-model
    value: "<MODEL>"
    scope: ["review"]
    default: ""
    description: "Forces every discovered review check to use a model."
    example: "goose review --override-model claude-sonnet-4-20250514"
    notes: ""
  - flag: --turn-limit
    value: "<N>"
    scope: ["review"]
    default: ""
    description: "Default turn limit for orchestrated review subprocesses and checks."
    example: "goose review --turn-limit 10"
    notes: ""
  - flag: --dry-run
    value: ""
    scope: ["review"]
    default: "false"
    description: "Prints the assembled review prompt and checks instead of running."
    example: "goose review --dry-run"
    notes: ""
  - flag: --no-orchestrate
    value: ""
    scope: ["review"]
    default: "false"
    description: "Disables Rust-driven parallel review orchestration."
    example: "goose review --no-orchestrate"
    notes: ""
  - flag: -i, --instructions
    value: "<TEXT>"
    scope: ["review"]
    default: ""
    description: "Additional free-form instructions to prepend to the review."
    example: "goose review --instructions \"focus on regressions\""
    notes: ""
  - flag: -f, --files
    value: "<FILE>..."
    scope: ["review"]
    default: ""
    description: "Restricts review to specific files."
    example: "goose review --files src/lib.rs"
    notes: ""
  - flag: -c, --check-filter
    value: "<NAME>..."
    scope: ["review"]
    default: ""
    description: "Runs only matching named review checks."
    example: "goose review --check-filter security"
    notes: ""
  - flag: -s, --check-scope
    value: "<DIR>"
    scope: ["review"]
    default: ""
    description: "Alternate directory to search for `.agents/checks/*.md`."
    example: "goose review --check-scope ."
    notes: ""
  - flag: --checks-only
    value: ""
    scope: ["review"]
    default: "false"
    description: "Skips the main correctness pass and only runs check subagents."
    example: "goose review --checks-only"
    notes: ""
  - flag: --summary-only
    value: ""
    scope: ["review"]
    default: "false"
    description: "Prints only the diff summary and skips the full review."
    example: "goose review --summary-only"
    notes: ""
  - flag: --severity
    value: "<LEVEL>"
    scope: ["review"]
    default: "medium"
    description: "Minimum severity to display."
    example: "goose review --severity low"
    notes: ""
config_files:
  - os: macos
    scope: user
    path: "~/.config/goose/config.yaml"
    format: yaml
    notes: "Official docs list this path; source currently uses platform app strategy with Block/goose for compatibility, so `goose info` is the safest way to discover the effective path."
  - os: linux
    scope: user
    path: "~/.config/goose/config.yaml"
    format: yaml
    notes: "Official primary config path."
  - os: windows
    scope: user
    path: "%APPDATA%\\Block\\goose\\config\\config.yaml"
    format: yaml
    notes: "Official primary Windows config path."
  - os: linux
    scope: system
    path: "/etc/goose/config.yaml"
    format: yaml
    notes: "Source-backed system config path loaded before additional files and user config."
  - os: macos
    scope: system
    path: "/etc/goose/config.yaml"
    format: yaml
    notes: "Source-backed Unix system config path loaded before additional files and user config."
  - os: windows
    scope: system
    path: "%PROGRAMDATA%\\goose\\config.yaml"
    format: yaml
    notes: "Source-backed system config path; falls back to `C:\\ProgramData\\goose\\config.yaml` if PROGRAMDATA is unset."
  - os: all
    scope: env
    path: "GOOSE_ADDITIONAL_CONFIG_FILES"
    format: yaml
    notes: "Source-backed path-list environment variable for additional config files; loaded after system config and before user config."
  - os: macos
    scope: user
    path: "~/.config/goose/permission.yaml"
    format: yaml
    notes: "Tool permission levels configured by `goose configure`."
  - os: linux
    scope: user
    path: "~/.config/goose/permission.yaml"
    format: yaml
    notes: "Tool permission levels configured by `goose configure`."
  - os: windows
    scope: user
    path: "%APPDATA%\\Block\\goose\\config\\permission.yaml"
    format: yaml
    notes: "Tool permission levels configured by `goose configure`."
  - os: macos
    scope: user
    path: "~/.config/goose/secrets.yaml"
    format: yaml
    notes: "Only used when file-based secret storage is active."
  - os: linux
    scope: user
    path: "~/.config/goose/secrets.yaml"
    format: yaml
    notes: "Only used when file-based secret storage is active."
  - os: windows
    scope: user
    path: "%APPDATA%\\Block\\goose\\config\\secrets.yaml"
    format: yaml
    notes: "Only used when file-based secret storage is active."
  - os: all
    scope: user
    path: "~/.agents/plugins/"
    format: other
    notes: "Installed Goose plugins live under `.agents/plugins/<plugin-name>/`."
  - os: all
    scope: user
    path: "~/.agents/skills/"
    format: other
    notes: "Goose skills are discovered from the shared agents skills directory."
env_vars:
  - name: GOOSE_PATH_ROOT
    effect: "Overrides the root directory for Goose config, data, state, plugins, and agent files; useful for wrapper isolation and CI."
  - name: GOOSE_ADDITIONAL_CONFIG_FILES
    effect: "Adds extra YAML config files between system config and user config in precedence."
  - name: GOOSE_DISABLE_KEYRING
    effect: "Disables system keyring secret storage and forces file-based secret behavior when set or configured truthy."
  - name: GOOSE_PROMPT_EDITOR
    effect: "Uses an external editor for composing interactive prompts."
  - name: GOOSE_CLI_THEME
    effect: "Controls CLI markdown theme: light, dark, or ansi."
  - name: GOOSE_CLI_LIGHT_THEME
    effect: "Controls bat syntax theme used for light CLI markdown rendering."
  - name: GOOSE_CLI_DARK_THEME
    effect: "Controls bat syntax theme used for dark CLI markdown rendering."
  - name: GOOSE_CLI_NEWLINE_KEY
    effect: "Customizes the Ctrl+<key> shortcut used for newlines in CLI input."
  - name: GOOSE_CLI_SHOW_THINKING
    effect: "Shows model reasoning/thinking output in CLI responses when models expose it."
  - name: GOOSE_RANDOM_THINKING_MESSAGES
    effect: "Controls random thinking/progress messages in CLI output."
  - name: GOOSE_CLI_SHOW_COST
    effect: "Toggles display of model cost estimates in CLI output."
  - name: GOOSE_MAX_CODE_BLOCK_LINES
    effect: "Sets the line threshold before CLI code blocks are truncated."
  - name: GOOSE_TRUNCATED_SHOW_LINES
    effect: "Controls how many lines are shown before the truncated-lines marker."
  - name: GOOSE_NO_CODE_TRUNCATION
    effect: "Disables CLI code block truncation."
  - name: GOOSE_SEARCH_PATHS
    effect: "Prepends additional directories to PATH for extension commands."
  - name: GOOSE_SHELL
    effect: "Overrides the shell used for Developer extension shell commands."
  - name: GOOSE_SERVER__SECRET_KEY
    effect: "Shared secret required by `goose serve` unless started with `--dangerously-unauthenticated`."
  - name: GOOSE_RECIPE_PATH
    effect: "Additional recipe search directories; colon-separated on Unix and semicolon-separated on Windows."
  - name: GOOSE_RECIPE_GITHUB_REPO
    effect: "GitHub repository to search for recipes."
  - name: GOOSE_RECIPE_RETRY_TIMEOUT_SECONDS
    effect: "Global timeout for recipe success-check commands."
  - name: GOOSE_RECIPE_ON_FAILURE_TIMEOUT_SECONDS
    effect: "Global timeout for recipe on-failure commands."
  - name: GOOSE_TERMINAL
    effect: "Set by Goose when running commands so shell config and scripts can detect Goose execution."
  - name: AGENT
    effect: "Set to `goose` by Goose for cross-agent script compatibility."
  - name: AGENT_SESSION_ID
    effect: "Set in extension/shell contexts to the current Goose session ID."
  - name: GOOSE_BIN_DIR
    effect: "Installer-only variable overriding the target binary directory."
  - name: GOOSE_VERSION
    effect: "Installer-only variable pinning a specific Goose release."
  - name: CANARY
    effect: "Installer-only variable selecting canary release assets when set to true."
  - name: CONFIGURE
    effect: "Installer-only variable; `false` skips interactive `goose configure` after install."
  - name: GOOSE_LINUX_VARIANT
    effect: "Installer-only variable selecting Linux asset variant: standard, vulkan, or musl."
  - name: GOOSE_WINDOWS_VARIANT
    effect: "Installer-only variable selecting Windows asset variant: standard or cuda."
  - name: INSTALL_OS
    effect: "Installer-only override for OS detection: linux, windows, or darwin."
  - name: HTTP_PROXY
    effect: "Standard proxy variable used for Goose network connections."
  - name: HTTPS_PROXY
    effect: "Standard HTTPS proxy variable used for Goose network connections."
  - name: NO_PROXY
    effect: "Standard proxy bypass list."
machine_introspection:
  - command: "goose info --verbose"
    purpose: config_dump
    machine_readable: false
    output_format: yaml
    useful_for_codegen: false
    notes: "Prints version, config/session/log paths, and merged config values. The config block is YAML-like but embedded in human text."
  - command: "goose info --check"
    purpose: doctor
    machine_readable: false
    output_format: text
    useful_for_codegen: false
    notes: "Checks provider/model configuration and performs a provider request; useful for diagnostics, not static metadata."
  - command: "goose session list --format json"
    purpose: other
    machine_readable: true
    output_format: json
    useful_for_codegen: false
    notes: "Lists saved sessions and can be filtered by working directory and limit."
  - command: "goose session export --session-id <id> --format json"
    purpose: other
    machine_readable: true
    output_format: json
    useful_for_codegen: false
    notes: "Exports one session as JSON when a stable session identifier is known."
  - command: "goose session diagnostics --session-id <id> --output <file>"
    purpose: doctor
    machine_readable: true
    output_format: json
    useful_for_codegen: false
    notes: "Generates a diagnostics JSON file containing session, system, config, and log data; may include sensitive content."
  - command: "goose recipe list --format json"
    purpose: other
    machine_readable: true
    output_format: json
    useful_for_codegen: false
    notes: "Lists available recipes from configured discovery sources."
  - command: "goose skills list"
    purpose: capabilities
    machine_readable: false
    output_format: text
    useful_for_codegen: false
    notes: "Lists skills available to the Goose agent; no JSON flag found in current source."
  - command: "goose run --output-format json --no-session -t <prompt>"
    purpose: other
    machine_readable: true
    output_format: json
    useful_for_codegen: false
    notes: "Returns a single JSON object after completion. Useful for wrapper execution, not provider metadata generation."
  - command: "goose run --output-format stream-json --no-session -t <prompt>"
    purpose: other
    machine_readable: true
    output_format: jsonl
    useful_for_codegen: false
    notes: "Streams event objects as newline-delimited JSON during a run."
wrapper_notes:
  - "Current official project location is AAIF (`aaif-goose/goose`, `goose-docs.ai`); older Block URLs may redirect or be stale."
  - "No `goose` binary is installed on this host, so this research uses official docs and source rather than local help output."
  - "`goose run --output-format stream-json` is the best wrapper surface for live event parsing; `json` emits one object after completion."
  - "Stream event source currently defines top-level event types `message`, `notification`, `error`, and `complete`; notification payload fields are flattened."
  - "`goose run --quiet` suppresses non-response output for text mode, but wrappers should prefer structured output for reliable parsing."
  - "`goose configure`, `goose doctor`, `goose session --edit`, `goose update --reconfigure`, gateway flows, and plugin Git operations are interactive or may require external credentials."
  - "The official install scripts run `goose configure` by default; wrappers and CI should set `CONFIGURE=false` for non-interactive installs."
  - "Use `GOOSE_PATH_ROOT` to isolate config/data/state for tests or wrapper-managed runs."
  - "`goose serve` requires `GOOSE_SERVER__SECRET_KEY` unless `--dangerously-unauthenticated` is supplied."
  - "Session diagnostics and debug output can contain prompts, tool output, config, logs, paths, and secrets; do not collect them silently."
  - "Windows native CLI support is x86_64 only in the current installer scripts; Windows ARM64 is rejected by the PowerShell installer."
  - "Config path documentation and source are not perfectly aligned for macOS because source keeps Block app-strategy directories for backward compatibility; use `goose info` for the effective path."
changes: []
requires_claudine_update: true
reason: "Goose's current CLI surface includes new commands and wrapper-relevant switches (`serve`, `doctor`, `plugin`, `skills`, `review`, `stream-json`, install/config isolation variables) that should be reflected in Claudine provider metadata and wrapper behavior."
---

## Overview

Goose is an open-source local AI agent with Desktop, CLI, and API surfaces. The project has moved from Block-owned URLs to the Agentic AI Foundation namespace: the current official repository is [aaif-goose/goose](https://github.com/aaif-goose/goose), and the current public docs are at [goose-docs.ai](https://goose-docs.ai/). The requested Block site still exists as a historical/migration entry point, but current release assets and install commands use the AAIF repository.

The current latest GitHub release observed during this research is `v1.41.0`, published from the AAIF repository. No local `goose` executable was present on this host, so the CLI inventory is based on official documentation and the current `clap` source in `crates/goose-cli/src/cli.rs`.

## Installation and Binaries

The executable is `goose` on macOS and Linux and `goose.exe` on native Windows. Official CLI installation is by the release download scripts or Homebrew on macOS:

```bash
curl -fsSL https://github.com/aaif-goose/goose/releases/download/stable/download_cli.sh | bash
curl -fsSL https://github.com/aaif-goose/goose/releases/download/stable/download_cli.sh | CONFIGURE=false bash
brew install block-goose-cli
```

Windows has both Git Bash/MSYS and PowerShell installer paths. The PowerShell installer downloads `goose-x86_64-pc-windows-msvc.zip` by default and rejects Windows ARM64. The shell installer supports Linux, macOS, Windows-like shells, and WSL, with release-asset variants controlled by `GOOSE_LINUX_VARIANT` and `GOOSE_WINDOWS_VARIANT`.

Install scripts run `goose configure` by default. Non-interactive installers should set `CONFIGURE=false`; reproducible CI installs should set `GOOSE_VERSION`.

## Subcommands

Current top-level commands from docs and source:

| Command | Alias | Purpose | Wrapper relevance |
| --- | --- | --- | --- |
| `configure` | | Interactive configuration for providers and extensions | TTY/browser/credential flow |
| `info` | | Version, paths, config, optional provider check | Useful diagnostics; text output |
| `doctor` | | Interactive setup diagnostic | Not machine-readable |
| `mcp` | | Runs a bundled MCP server | Server process |
| `acp` | | ACP agent server over stdio | Integration server |
| `serve` | | ACP HTTP/WebSocket server | Requires secret unless unsafe flag used |
| `session` | `s` | Interactive sessions plus list/remove/export/import/diagnostics | Mixed interactive and scriptable subcommands |
| `project` | `p` | Opens last project directory | User-focus side effect |
| `projects` | `ps` | Lists/selects recent projects | Source routes to interactive project picker |
| `run` | | Non-interactive task execution | Primary wrapper entry point |
| `recipe` | | Recipe validate/deeplink/open/list | `list --format json` is parseable |
| `skills` | | Skill utilities | `skills list` only; text output |
| `plugin` | | Install/update Git plugins | Network and possible Git credential prompts |
| `schedule` | `sched` | Scheduled recipe jobs | Scriptable |
| `gateway` | `gw` | External chat gateway management | Token and pairing flows |
| `update` | | CLI self-update | Feature-gated; may configure interactively |
| `term` | | Terminal-integrated sessions | Shell integration |
| `tui` | | Text UI launcher | Feature-gated; node/npx resolution |
| `local-models` | `lm` | Local inference model management | Feature-gated |
| `completion` | | Shell completions | Scriptable |
| `review` | | Diff review using Goose and `.agents/checks` | Scriptable but can spawn runs |
| `validate-extensions` | | Hidden bundled-extension validator | Hidden |

## CLI Switch Inventory

The frontmatter `cli_switches` array contains the full source-backed switch inventory relevant to public and hidden top-level commands. Important wrapper switches are:

- `goose run --output-format text|json|stream-json`, default `text`.
- `goose run --no-session` to avoid persisted session state.
- `goose run --provider <PROVIDER> --model <MODEL>` for per-run overrides.
- `goose run --quiet` for cleaner text output.
- `goose run --max-turns <NUMBER>` and `--max-tool-repetitions <NUMBER>` to bound long or looping runs.
- `goose run --with-extension`, `--with-streamable-http-extension`, `--with-builtin`, and `--no-profile` to control extension surfaces.
- `goose info --verbose` for human-readable effective config and paths.
- `goose session list --format json`, `goose session export --format json`, and `goose recipe list --format json` for parseable inventory.

## Configuration Discovery

Official docs state the primary YAML config file is:

- macOS/Linux: `~/.config/goose/config.yaml`
- Windows: `%APPDATA%\Block\goose\config\config.yaml`

Related files include `permission.yaml`, `secrets.yaml` when file-backed secret storage is active, `permissions/tool_permissions.json`, and `prompts/`. Source also loads a system YAML config first (`/etc/goose/config.yaml` on Unix, `%PROGRAMDATA%\goose\config.yaml` or `C:\ProgramData\goose\config.yaml` on Windows), then files from `GOOSE_ADDITIONAL_CONFIG_FILES`, then the user config.

One caveat matters for wrappers: current source uses platform app-directory discovery with `Block/goose` kept for backward compatibility. The docs simplify this to `~/.config/goose/config.yaml` on macOS/Linux. A wrapper that needs the effective path should run `goose info` rather than hard-code one path.

## Environment Variables

General CLI/runtime variables are listed in frontmatter. Variables intentionally excluded from that list include provider endpoint/API key variables and model-selection variables that belong in the narrower model-config research topic, except where they directly affect install or wrapper isolation.

High-impact wrapper variables:

- `GOOSE_PATH_ROOT` isolates all Goose config/data/state directories.
- `GOOSE_ADDITIONAL_CONFIG_FILES` adds layered config files.
- `GOOSE_DISABLE_KEYRING` changes secret-storage behavior.
- `GOOSE_PROMPT_EDITOR` can force editor-based prompting in interactive sessions.
- `GOOSE_CLI_THEME`, `GOOSE_CLI_SHOW_THINKING`, `GOOSE_RANDOM_THINKING_MESSAGES`, and code truncation variables affect human output.
- `GOOSE_SERVER__SECRET_KEY` is required for authenticated `goose serve`.
- `CONFIGURE=false` prevents install scripts from launching interactive configuration.

## Machine Introspection

Goose has several useful parseable commands, but no single documented `doctor --json`, config-schema dump, model catalog dump, or tool catalog dump was found.

Useful commands:

```bash
goose info --verbose
goose info --check
goose session list --format json
goose session export --session-id <id> --format json
goose session diagnostics --session-id <id> --output <file>
goose recipe list --format json
goose run --output-format json --no-session -t "<prompt>"
goose run --output-format stream-json --no-session -t "<prompt>"
```

`goose info --verbose` is especially useful for discovery but is not pure machine-readable output: it prints labeled sections and an indented YAML-like config block. `goose session diagnostics` is JSON but may include sensitive session/config/log content.

## Wrapper Notes

Use `goose run --output-format stream-json --no-session -t ...` for live wrappers. Current stream source defines event variants `message`, `notification`, `error`, and `complete`; `notification` uses flattened `log` or `progress` payload data. Batch `json` is suitable for CI-style runs that only need final output.

Avoid interactive surfaces in wrappers unless a TTY flow is intentional: `configure`, `doctor`, `session --edit`, `run --interactive`, update reconfiguration, gateway pair/start flows, and plugin install/update can prompt, open browsers, or invoke Git/network credentials.

For isolated wrapper execution, set `GOOSE_PATH_ROOT` to a controlled directory and pass `--no-session` where persistence is not wanted. For clean text output, pass `--quiet`, but structured JSON modes are more reliable.

## Changelog

Not an update-mode run; `changes` is empty.

## Sources

- [Goose homepage](https://goose-docs.ai/)
- [Goose installation docs](https://goose-docs.ai/docs/getting-started/installation/)
- [Goose CLI commands docs](https://goose-docs.ai/docs/guides/goose-cli-commands/)
- [Running tasks guide](https://goose-docs.ai/docs/guides/running-tasks/)
- [Configuration files guide](https://goose-docs.ai/docs/guides/config-files/)
- [Environment variables guide](https://goose-docs.ai/docs/guides/environment-variables/)
- [Goose repository](https://github.com/aaif-goose/goose)
- [Goose CLI source](https://github.com/aaif-goose/goose/blob/main/crates/goose-cli/src/cli.rs)
- [Goose session stream source](https://github.com/aaif-goose/goose/blob/main/crates/goose-cli/src/session/mod.rs)
- [Goose config path source](https://github.com/aaif-goose/goose/blob/main/crates/goose/src/config/paths.rs)
- [Latest release API result](https://github.com/aaif-goose/goose/releases/tag/v1.41.0)
