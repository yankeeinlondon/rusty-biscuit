---
$schema: ./_schema.yaml
created: 2026-04-27
last_updated: 2026-07-03
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
    notes: "Release assets include `goose-aarch64-apple-darwin` and `goose-x86_64-apple-darwin`; the Homebrew CLI formula installs `goose`."
  - os: linux
    binary: goose
    alt_binaries: []
    notes: "Release assets include GNU, Vulkan, and musl variants for x86_64 and aarch64; Linux packages also exist for Desktop."
  - os: windows
    binary: goose.exe
    alt_binaries: ["goose"]
    notes: "Native CLI release assets are `goose-x86_64-pc-windows-msvc.zip` and `goose-x86_64-pc-windows-msvc-cuda.zip`; the shell installer writes `goose.exe`."
install_methods:
  - os: macos
    method: other
    command: "curl -fsSL https://github.com/aaif-goose/goose/releases/download/stable/download_cli.sh | bash"
    notes: "Official CLI installer. Default target is `$HOME/.local/bin`; set `GOOSE_BIN_DIR` to override and `CONFIGURE=false` to skip interactive setup."
  - os: macos
    method: brew
    command: "brew install block-goose-cli"
    notes: "Official Homebrew formula for the CLI."
  - os: linux
    method: other
    command: "curl -fsSL https://github.com/aaif-goose/goose/releases/download/stable/download_cli.sh | bash"
    notes: "Official CLI installer. Default target is `$HOME/.local/bin`; supports `GOOSE_LINUX_VARIANT=standard|vulkan|musl`."
  - os: windows
    method: other
    command: "curl -fsSL https://github.com/aaif-goose/goose/releases/download/stable/download_cli.sh | bash"
    notes: "Official Git Bash/MSYS2 installer. Default native Windows target is `%USERPROFILE%\\goose`; supports `GOOSE_WINDOWS_VARIANT=standard|cuda`."
  - os: windows
    method: other
    command: "Invoke-WebRequest -Uri \"https://raw.githubusercontent.com/aaif-goose/goose/main/download_cli.ps1\" -OutFile \"download_cli.ps1\"; .\\download_cli.ps1"
    notes: "Official PowerShell installer path from docs."
subcommands:
  - name: configure
    description: "Configures providers, extensions, and settings."
    non_interactive: false
    notes: "Interactive menu; may collect credentials or trigger provider auth."
  - name: info
    description: "Prints Goose version, paths, config, and optional provider check status."
    non_interactive: true
    notes: "`--verbose` prints config as human-oriented YAML-like text; `--check` performs a provider request and exits non-zero when unconfigured."
  - name: doctor
    description: "Checks whether Goose setup is working."
    non_interactive: false
    notes: "No flags and no machine-readable mode in 1.41.0; intended diagnostic flow."
  - name: mcp
    description: "Runs one bundled MCP server over stdio."
    non_interactive: true
    notes: "Server argument accepts `autovisualiser`, `computercontroller`, `memory`, or `tutorial`."
  - name: acp
    description: "Runs Goose as an ACP agent server over stdio."
    non_interactive: true
    notes: "Accepts `--with-builtin`."
  - name: serve
    description: "Starts an ACP server over HTTP and WebSocket."
    non_interactive: true
    notes: "Requires `GOOSE_SERVER__SECRET_KEY` unless `--dangerously-unauthenticated` is used."
  - name: session
    description: "Starts or resumes interactive chat sessions, with nested list/remove/export/import/diagnostics utilities."
    non_interactive: false
    notes: "Alias: `s`; nested `list`, `export`, `import`, and `diagnostics` can be scripted when identifiers are supplied."
  - name: project
    description: "Opens the last project directory."
    non_interactive: false
    notes: "Alias: `p`; user-focus side effect."
  - name: projects
    description: "Lists recent project directories."
    non_interactive: false
    notes: "Alias: `ps`; source routes to an interactive picker."
  - name: run
    description: "Executes a prompt, instruction file, stdin, or recipe and exits unless interactive mode is requested."
    non_interactive: true
    notes: "Primary wrapper entry point; supports `--output-format json` and `--output-format stream-json`."
  - name: recipe
    description: "Recipe utilities for validation, deeplinking, opening in Desktop, and listing."
    non_interactive: true
    notes: "`recipe open` launches Goose Desktop; `recipe list --format json` is machine-readable."
  - name: skills
    description: "Skill utilities."
    non_interactive: true
    notes: "Current public nested command is `skills list`, text only."
  - name: plugin
    description: "Installs and updates Git-backed plugins."
    non_interactive: false
    notes: "Network and Git credential prompts are possible."
  - name: schedule
    description: "Manages scheduled recipe jobs."
    non_interactive: true
    notes: "Alias: `sched`; includes deprecated service commands and `cron-help`."
  - name: gateway
    description: "Manages external platform gateways."
    non_interactive: false
    notes: "Alias: `gw`; start/pair flows require external credentials or user action."
  - name: update
    description: "Updates the Goose CLI."
    non_interactive: false
    notes: "`--reconfigure` is interactive; updater mutates the installed binary."
  - name: term
    description: "Terminal-integrated persistent session helpers."
    non_interactive: false
    notes: "`term init`, `term info`, and hidden `term log` are scriptable; `term run` sends prompts into a persistent terminal session."
  - name: tui
    description: "Launches the Goose terminal UI."
    non_interactive: false
    notes: "Resolves `GOOSE_TUI_SCRIPT` or runs Node/npx; requires a TTY-style UI."
  - name: local-models
    description: "Searches, downloads, lists, and deletes local inference models."
    non_interactive: true
    notes: "Alias: `lm`; search/download use Hugging Face/network and local model storage."
  - name: completion
    description: "Generates shell completions."
    non_interactive: true
    notes: "Supports bash, elvish, fish, powershell, nu, and zsh."
  - name: review
    description: "Reviews the current diff using Goose and optional `.agents/checks/*.md` reviewer checks."
    non_interactive: true
    notes: "May spawn multiple `goose run` subprocesses; `--dry-run` prints assembled inputs without running the agent."
  - name: validate-extensions
    description: "Validates a bundled-extensions JSON file."
    non_interactive: true
    notes: "Hidden in top-level help but callable in 1.41.0."
cli_switches:
  - flag: --help
    value: ""
    scope: ["global"]
    default: "false"
    description: "Prints help."
    example: "goose --help"
    notes: "Observed in release help."
  - flag: --version
    value: ""
    scope: ["global"]
    default: "false"
    description: "Prints installed Goose version."
    example: "goose --version"
    notes: "Release binary printed `1.41.0`."
  - flag: -v, --verbose
    value: ""
    scope: ["info"]
    default: "false"
    description: "Shows detailed configuration, including merged config values."
    example: "goose info --verbose"
    notes: "Text output with YAML-like config section."
  - flag: --check
    value: ""
    scope: ["info"]
    default: "false"
    description: "Tests provider connection and prints status."
    example: "goose info --check"
    notes: "Performs a real provider request; observed non-zero when provider is not configured."
  - flag: --with-builtin
    value: "<NAME[,NAME...]>"
    scope: ["acp", "serve", "session", "run"]
    default: ""
    description: "Adds one or more bundled extensions by name."
    example: "goose run --with-builtin developer -t \"summarize this repo\""
    notes: "Comma-delimited."
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
    description: "TLS private-key path for `goose serve`."
    example: "goose serve --tls-key-path key.pem"
    notes: ""
  - flag: --platform
    value: "<cli|desktop>"
    scope: ["serve"]
    default: "cli"
    description: "Selects the served Goose platform identity."
    example: "goose serve --platform cli"
    notes: ""
  - flag: --dangerously-unauthenticated
    value: ""
    scope: ["serve"]
    default: "false"
    description: "Starts the ACP endpoint without requiring `GOOSE_SERVER__SECRET_KEY`."
    example: "goose serve --dangerously-unauthenticated"
    notes: "Wrappers should not add this automatically."
  - flag: --allowed-origin
    value: "<ORIGIN>"
    scope: ["serve"]
    default: ""
    description: "Allows an exact Origin value for ACP CORS; repeatable."
    example: "goose serve --allowed-origin http://localhost:3000"
    notes: "Supplying this replaces default loopback origins."
  - flag: -n, --name
    value: "<NAME>"
    scope: ["session", "run", "session remove", "session export", "session diagnostics"]
    default: ""
    description: "Names or identifies a Goose session."
    example: "goose run --name my-project -t \"start\""
    notes: "Shared identifier group."
  - flag: --session-id, --id
    value: "<SESSION_ID>"
    scope: ["session", "run", "session remove", "session export", "session diagnostics"]
    default: ""
    description: "Identifies a Goose session by ID."
    example: "goose session --resume --session-id 20251108_2"
    notes: "`--id` is a hidden alias from source."
  - flag: --path
    value: "<PATH>"
    scope: ["session", "run", "session remove", "session export", "session diagnostics"]
    default: ""
    description: "Legacy path-based session identifier."
    example: "goose session --resume --path ./20250325_200615.jsonl"
    notes: ""
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
    description: "Creates a new session by copying a previous session; requires `--resume`."
    example: "goose session --resume --fork"
    notes: ""
  - flag: --edit
    value: ""
    scope: ["session"]
    default: "false"
    description: "Opens the session conversation in `$VISUAL`, `$EDITOR`, or `vi` before resuming or forking."
    example: "goose session --resume --session-id 20251108_2 --edit"
    notes: "Requires an editor; unsuitable for non-interactive wrappers."
  - flag: --history
    value: ""
    scope: ["session"]
    default: "false"
    description: "Shows previous messages when resuming a session."
    example: "goose session --resume --history"
    notes: ""
  - flag: --debug
    value: ""
    scope: ["session", "run"]
    default: "false"
    description: "Shows complete tool responses, detailed parameters, and full paths."
    example: "goose run --debug -t \"inspect failing tests\""
    notes: "May expose sensitive data."
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
    notes: "Also configurable through config/env."
  - flag: --container
    value: "<CONTAINER_ID>"
    scope: ["session", "run"]
    default: ""
    description: "Runs extensions inside a specified Docker/container environment."
    example: "goose run --container devbox -t \"run tests\""
    notes: "The extension and built-in Goose support must exist in the container."
  - flag: --with-extension
    value: "<COMMAND>"
    scope: ["session", "run"]
    default: ""
    description: "Adds stdio extensions from full commands; repeatable."
    example: "goose run --with-extension \"npx -y @modelcontextprotocol/server-memory\" -t \"remember this\""
    notes: "Shell quoting matters; values may include env assignments."
  - flag: --with-streamable-http-extension
    value: "<URL [timeout=SECONDS]>"
    scope: ["session", "run"]
    default: ""
    description: "Adds streamable HTTP extensions; repeatable."
    example: "goose run --with-streamable-http-extension \"http://localhost:8080/mcp timeout=100\" -t \"use the server\""
    notes: "Parser accepts optional whitespace-separated `timeout=`."
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
    notes: "Default order is newest first."
  - flag: -w, --working_dir
    value: "<WORKING_DIR>"
    scope: ["session list"]
    default: ""
    description: "Filters sessions by working directory."
    example: "goose session list --working_dir ~/src/project"
    notes: "Observed spelling uses underscore."
  - flag: -l, --limit
    value: "<NUMBER>"
    scope: ["session list", "schedule sessions", "local-models search"]
    default: "10 for local-models search; otherwise unset"
    description: "Limits number of results."
    example: "goose session list --limit 10"
    notes: ""
  - flag: -r, --regex
    value: "<REGEX>"
    scope: ["session remove"]
    default: ""
    description: "Removes sessions matching a regex."
    example: "goose session remove --regex \"project-.*\""
    notes: "Removal is interactive if no identifier or regex is supplied."
  - flag: -o, --output
    value: "<OUTPUT>"
    scope: ["session export", "session diagnostics"]
    default: "stdout for export; command default for diagnostics"
    description: "Writes export or diagnostics output to a path."
    example: "goose session export --session-id 20251108_4 --format json --output session.json"
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
    description: "Publishes an encrypted Nostr session share link or treats import input as one."
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
    description: "Provides additional system instructions for the run."
    example: "goose run --system \"Be concise\" -t \"summarize\""
    notes: "System-prompt delivery semantics belong to [system-prompt Goose research](../system-prompt/goose.md); this topic records only the flag surface."
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
    notes: "Avoid for batch wrappers."
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
    description: "Prints generation statistics after completion."
    example: "goose run --stats -t \"summarize\""
    notes: ""
  - flag: --scheduled-job-id
    value: "<ID>"
    scope: ["run"]
    default: ""
    description: "Associates a run with a scheduled job."
    example: "goose run --scheduled-job-id daily-report --recipe report.yaml"
    notes: "Hidden internal flag from source."
  - flag: -q, --quiet
    value: ""
    scope: ["run", "review"]
    default: "false"
    description: "Suppresses non-response output."
    example: "goose run --quiet -t \"answer only\""
    notes: "Useful for text mode, but structured output is more reliable for wrappers."
  - flag: --output-format
    value: "<text|json|stream-json>"
    scope: ["run"]
    default: "text"
    description: "Selects run output format."
    example: "goose run --output-format stream-json -t \"summarize this repo\""
    notes: "`stream-json` emits newline-delimited JSON event objects."
  - flag: --provider
    value: "<PROVIDER>"
    scope: ["run", "review"]
    default: ""
    description: "Overrides provider for the run or review."
    example: "goose run --provider anthropic --model claude-sonnet-4-20250514 -t \"inspect\""
    notes: "Overrides configured/default provider for the invocation."
  - flag: --model
    value: "<MODEL>"
    scope: ["run", "review"]
    default: ""
    description: "Overrides model for the run or review."
    example: "goose run --provider openai --model gpt-4.1 -t \"summarize\""
    notes: "Overrides configured/default model for the invocation."
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
  - flag: --schedule-id, --id
    value: "<SCHEDULE_ID>"
    scope: ["schedule add", "schedule remove", "schedule sessions", "schedule run-now"]
    default: ""
    description: "Identifies a scheduled recipe job."
    example: "goose schedule run-now --schedule-id daily-report"
    notes: "`--id` is an alias from source."
  - flag: --cron
    value: "<EXPR>"
    scope: ["schedule add"]
    default: ""
    description: "Cron expression for a scheduled job."
    example: "goose schedule add --schedule-id daily --cron \"0 9 * * *\" --recipe-source ./daily.yaml"
    notes: "Required for `schedule add`."
  - flag: --recipe-source
    value: "<PATH|BASE64>"
    scope: ["schedule add"]
    default: ""
    description: "Recipe source path or base64-encoded recipe string."
    example: "goose schedule add --schedule-id daily --cron \"0 9 * * *\" --recipe-source ./daily.yaml"
    notes: "Required for `schedule add`."
  - flag: --bot-token
    value: "<TOKEN>"
    scope: ["gateway start"]
    default: ""
    description: "Gateway platform bot token."
    example: "goose gateway start telegram --bot-token \"$TOKEN\""
    notes: "Secret-bearing argument."
  - flag: -c, --canary
    value: ""
    scope: ["update"]
    default: "false"
    description: "Updates to the canary release instead of stable."
    example: "goose update --canary"
    notes: "Mutates installed binary."
  - flag: -r, --reconfigure
    value: ""
    scope: ["update"]
    default: "false"
    description: "Forces reconfiguration during update."
    example: "goose update --reconfigure"
    notes: "May prompt interactively."
  - flag: --bin-name
    value: "<BIN_NAME>"
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
    notes: "Supported for zsh, bash, and nu according to help."
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
  - flag: <FILE>
    value: "<FILE>"
    scope: ["validate-extensions"]
    default: ""
    description: "Path to bundled-extensions JSON file."
    example: "goose validate-extensions bundled-extensions.json"
    notes: "Hidden command positional argument."
config_paths:
  - os: macos
    scope: user
    path: "~/.config/goose/config.yaml"
    format: yaml
    notes: "Observed from the 1.41.0 macOS CLI with an isolated HOME and matches current config-file docs."
  - os: linux
    scope: user
    path: "~/.config/goose/config.yaml"
    format: yaml
    notes: "Observed with isolated HOME and matches docs."
  - os: windows
    scope: user
    path: "%APPDATA%\\Block\\goose\\config\\config.yaml"
    format: yaml
    notes: "Documented Windows config path and source app strategy."
  - os: macos
    scope: user
    path: "~/.config/goose/permission.yaml"
    format: yaml
    notes: "Tool permission levels configured by `goose configure`; docs also mention `permission.yaml` next to config."
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
    notes: "Used when file-based secret storage is active; macOS Keychain is default secret storage."
  - os: linux
    scope: user
    path: "~/.config/goose/secrets.yaml"
    format: yaml
    notes: "Used when file-based secret storage is active; system keyring may be used otherwise."
  - os: windows
    scope: user
    path: "%APPDATA%\\Block\\goose\\config\\secrets.yaml"
    format: yaml
    notes: "Used when file-based secret storage is active; Windows Credential Manager is default secret storage."
  - os: macos
    scope: user
    path: "~/.config/goose/permissions/tool_permissions.json"
    format: json
    notes: "Runtime permission decisions; auto-managed."
  - os: linux
    scope: user
    path: "~/.config/goose/permissions/tool_permissions.json"
    format: json
    notes: "Runtime permission decisions; auto-managed."
  - os: windows
    scope: user
    path: "%APPDATA%\\Block\\goose\\config\\permissions\\tool_permissions.json"
    format: json
    notes: "Runtime permission decisions; auto-managed."
  - os: macos
    scope: user
    path: "~/.config/goose/prompts/"
    format: other
    notes: "Customized prompt templates."
  - os: linux
    scope: user
    path: "~/.config/goose/prompts/"
    format: other
    notes: "Customized prompt templates."
  - os: windows
    scope: user
    path: "%APPDATA%\\Block\\goose\\config\\prompts\\"
    format: other
    notes: "Customized prompt templates."
  - os: macos
    scope: user
    path: "~/.local/share/goose/sessions/sessions.db"
    format: other
    notes: "Observed from `goose info` with an isolated HOME."
  - os: linux
    scope: user
    path: "~/.local/share/goose/sessions/sessions.db"
    format: other
    notes: "Observed path from `goose info` with isolated HOME."
  - os: windows
    scope: user
    path: "%APPDATA%\\Block\\goose\\data\\sessions\\sessions.db"
    format: other
    notes: "Expected from source app strategy; use `goose info` to verify on-host."
  - os: macos
    scope: user
    path: "~/.local/state/goose/logs/"
    format: other
    notes: "Observed from `goose info`; `goose info` creates it."
  - os: linux
    scope: user
    path: "~/.local/state/goose/logs/"
    format: other
    notes: "Observed path from `goose info`; `goose info` creates it."
  - os: windows
    scope: user
    path: "%LOCALAPPDATA%\\Block\\goose\\state\\logs\\"
    format: other
    notes: "Expected from source app strategy; known-issues docs mention `%LOCALAPPDATA%\\Block\\goose\\` for local data."
  - os: macos
    scope: user
    path: "~/.agents/plugins/"
    format: other
    notes: "Installed Goose plugins live under `.agents/plugins/<plugin-name>/`."
  - os: linux
    scope: user
    path: "~/.agents/plugins/"
    format: other
    notes: "Installed Goose plugins live under `.agents/plugins/<plugin-name>/`."
  - os: windows
    scope: user
    path: "%USERPROFILE%\\.agents\\plugins\\"
    format: other
    notes: "Source uses the user's home directory for plugins."
  - os: macos
    scope: user
    path: "~/.agents/agents/"
    format: other
    notes: "Source exposes an agents directory under `.agents`; skills are also discovered from `.agents/skills`."
  - os: linux
    scope: user
    path: "~/.agents/agents/"
    format: other
    notes: "Source exposes an agents directory under `.agents`; skills are also discovered from `.agents/skills`."
  - os: windows
    scope: user
    path: "%USERPROFILE%\\.agents\\agents\\"
    format: other
    notes: "Source exposes an agents directory under `.agents`; skills are also discovered from `.agents\\skills`."
  - os: macos
    scope: env
    path: "GOOSE_PATH_ROOT/config/config.yaml"
    format: yaml
    notes: "When `GOOSE_PATH_ROOT` is set, config, data, state, plugins, and agents paths are rooted under that directory."
  - os: linux
    scope: env
    path: "GOOSE_PATH_ROOT/config/config.yaml"
    format: yaml
    notes: "When `GOOSE_PATH_ROOT` is set, config, data, state, plugins, and agents paths are rooted under that directory."
  - os: windows
    scope: env
    path: "%GOOSE_PATH_ROOT%\\config\\config.yaml"
    format: yaml
    notes: "When `GOOSE_PATH_ROOT` is set, config, data, state, plugins, and agents paths are rooted under that directory."
env_vars:
  - name: GOOSE_PATH_ROOT
    effect: "Overrides the root directory for Goose config, data, state, plugins, and `.agents` directories; observed `goose info` creates `data/projects.json` and `state/logs` below it."
  - name: GOOSE_ADDITIONAL_CONFIG_FILES
    effect: "Adds extra YAML config files between system config and user config in precedence; source-backed but not surfaced by `goose info --help`."
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
    effect: "Customizes the Ctrl+key shortcut used for newlines in CLI input."
  - name: GOOSE_CLI_SHOW_THINKING
    effect: "Shows model reasoning/thinking output in CLI responses when available."
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
  - name: GOOSE_TUI_SCRIPT
    effect: "Overrides `goose tui` resolution with an existing local `dist/tui.js` script."
  - name: GOOSE_TUI_NPM_SPEC
    effect: "Overrides the npm package spec used by `goose tui` when it falls back to npx."
  - name: GOOSE_TERMINAL
    effect: "Set to `1` by Goose when running commands so shell configuration can detect Goose execution."
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
    notes: "Checks provider/model configuration and performs a provider request; observed non-zero with `provider check failed` when unconfigured."
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
    notes: "Lists skills available to the Goose agent; no JSON flag found in 1.41.0 help."
  - command: "goose local-models list"
    purpose: models
    machine_readable: false
    output_format: text
    useful_for_codegen: false
    notes: "Lists downloaded local models only; not a full provider model catalog and no JSON flag is exposed."
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
    notes: "Streams newline-delimited JSON event objects during a run."
wrapper_notes:
  - "The requested `block/goose` repository currently redirects through GitHub to `aaif-goose/goose`; release assets and installer commands use `aaif-goose/goose`, while some artifact names still contain `io.github.block`."
  - "No `goose` executable is installed on this host's PATH. A temporary macOS release binary was downloaded and inspected; it reports `1.41.0` while the latest release tag is `v1.41.0`."
  - "`goose info` is not side-effect-free: under an isolated HOME/`GOOSE_PATH_ROOT`, it created `data/projects.json` and `state/logs/cli/...` even with missing config."
  - "This Codex session has `HOME=/Users/ken/.claudine`; a normal user HOME changes Goose's default paths. Prefer `goose info` or explicit `GOOSE_PATH_ROOT` over hard-coded paths."
  - "`goose run --output-format stream-json --no-session -t ...` is the best live wrapper surface; `json` emits one object after completion."
  - "`goose run --quiet` suppresses non-response text output, but wrappers should prefer structured JSON modes for reliable parsing."
  - "The running-tasks docs mention `--with-remote-extension`, but the 1.41.0 binary rejects it and suggests `--with-extension`; this document trusts release help over that docs line."
  - "`--system <TEXT>` exists for additional system instructions; deeper delivery semantics belong to the sibling system-prompt research."
  - "`goose configure`, `goose doctor`, `goose session --edit`, `goose run --interactive`, `goose update --reconfigure`, gateway flows, and plugin Git operations are interactive or may require external credentials."
  - "The official install scripts run `goose configure` by default; wrappers and CI should set `CONFIGURE=false` for non-interactive installs."
  - "`goose serve` requires `GOOSE_SERVER__SECRET_KEY` unless `--dangerously-unauthenticated` is supplied."
  - "Session diagnostics, debug output, and verbose config output can contain prompts, tool output, config, logs, paths, and secrets; do not collect them silently."
changes:
  - "Verified current release `v1.41.0` from the GitHub releases API and inspected the downloaded 1.41.0 macOS binary instead of relying only on source/docs."
  - "Recorded that `goose` is not installed on this host's PATH and that local user config was not found under `/Users/ken`."
  - "Updated project-location notes: requested Block URLs now resolve to/use the AAIF repository and docs, while compatibility paths still use `Block/goose`."
  - "Captured observed `goose info` side effects under isolated roots."
  - "Added hidden-but-callable `validate-extensions` and hidden `term log` awareness."
  - "Recorded the docs/help disagreement where `--with-remote-extension` is documented but rejected by the 1.41.0 binary."
requires_claudine_update: true
reason: "Claudine's Goose metadata/wrapper should account for the verified 1.41.0 binary behavior, AAIF repository/docs location, `goose info` side effects, hidden callable commands, `GOOSE_PATH_ROOT` isolation, structured `stream-json` execution, and rejection of the documented `--with-remote-extension` flag."
---

# Goose CLI Research

## Overview

Goose is an open-source, local AI agent with Desktop, CLI, and API surfaces. The CLI is shipped by the Goose project now published under the Agentic AI Foundation GitHub organization, with continuing Block compatibility in paths and artifact names. The primary command a user types is `goose`; the primary automation entry point is `goose run`.

The current upstream release verified during this run is `v1.41.0`, published on 2026-07-03 according to the GitHub releases API. I also downloaded the `v1.41.0` macOS release asset into `/tmp` and ran the binary; `goose --version` printed `1.41.0`. No `goose` executable was installed on this host's `PATH`, so the temporary release binary is the local binary evidence.

Primary URLs:

- Homepage: [goose-docs.ai](https://goose-docs.ai/)
- Repository: [aaif-goose/goose](https://github.com/aaif-goose/goose)
- Requested legacy repository: [block/goose](https://github.com/block/goose), which resolves to the AAIF repository
- General docs: [goose-docs.ai/docs](https://goose-docs.ai/docs/)
- CLI reference: [CLI Commands](https://goose-docs.ai/docs/guides/goose-cli-commands/)

## Installation and Binaries

The CLI executable is `goose` on macOS and Linux and `goose.exe` in the native Windows release asset. Windows users still run the command as `goose` when the executable directory is on `PATH`.

Official installation commands from the docs and release installer:

| OS | Method | Command | Notes |
| --- | --- | --- | --- |
| macOS | Shell installer | `curl -fsSL https://github.com/aaif-goose/goose/releases/download/stable/download_cli.sh \| bash` | Installs `goose` to `$HOME/.local/bin` by default. Set `GOOSE_BIN_DIR` to override. |
| macOS | Shell installer, no configure | `curl -fsSL https://github.com/aaif-goose/goose/releases/download/stable/download_cli.sh \| CONFIGURE=false bash` | Avoids the default interactive `goose configure` step. |
| macOS | Homebrew | `brew install block-goose-cli` | Homebrew formula for CLI only. |
| Linux | Shell installer | `curl -fsSL https://github.com/aaif-goose/goose/releases/download/stable/download_cli.sh \| bash` | Installs `goose` to `$HOME/.local/bin`; supports `GOOSE_LINUX_VARIANT=standard|vulkan|musl`. |
| Linux | Shell installer, no configure | `curl -fsSL https://github.com/aaif-goose/goose/releases/download/stable/download_cli.sh \| CONFIGURE=false bash` | Recommended for CI and wrappers. |
| Windows | Git Bash/MSYS2 shell installer | `curl -fsSL https://github.com/aaif-goose/goose/releases/download/stable/download_cli.sh \| bash` | Native Windows default install directory in the shell script is `%USERPROFILE%\goose`; release asset contains `goose.exe`. |
| Windows | PowerShell installer | `Invoke-WebRequest -Uri "https://raw.githubusercontent.com/aaif-goose/goose/main/download_cli.ps1" -OutFile "download_cli.ps1"; .\download_cli.ps1` | Official docs path for native PowerShell install. |
| Windows | WSL | `curl -fsSL https://github.com/aaif-goose/goose/releases/download/stable/download_cli.sh \| bash` | Installs the Linux binary inside WSL. |

Release `v1.41.0` contains macOS `aarch64`/`x86_64`, Linux GNU/Vulkan/musl for `aarch64` and `x86_64`, and Windows `x86_64` standard/CUDA CLI assets. The shell installer rejects native Windows ARM64.

## Subcommands

| Command | Alias | Description | Automation / interaction |
| --- | --- | --- | --- |
| `configure` | | Configure providers, extensions, and settings. | Interactive TTY flow; may require credentials or auth. |
| `info` | | Print version, paths, config, and optional provider check. | Scriptable, but text output. `--check` can perform network/provider calls and exits non-zero when unconfigured. |
| `doctor` | | Check whether setup is working. | Interactive/human diagnostic; no JSON flag found. |
| `mcp <server>` | | Run bundled MCP servers: `autovisualiser`, `computercontroller`, `memory`, `tutorial`. | Non-interactive server process over stdio. |
| `acp` | | Run Goose as an ACP agent server over stdio. | Non-interactive server process. |
| `serve` | | Start ACP over HTTP/WebSocket. | Non-interactive server process; requires secret env var unless unsafe flag is used. |
| `session` | `s` | Start/resume interactive chat sessions and expose nested session utilities. | Top-level session is interactive; `list`, `export`, `import`, `diagnostics` are scriptable with identifiers. |
| `project` | `p` | Open the last project directory. | User-focus side effect. |
| `projects` | `ps` | List recent project directories. | Source routes to an interactive picker. |
| `run` | | Execute instructions from text, file/stdin, or recipe. | Primary non-interactive automation command unless `--interactive` is supplied. |
| `recipe` | | Validate, deeplink, open, or list recipes. | Mostly scriptable; `recipe open` launches Goose Desktop. |
| `skills` | | List skills available to the Goose agent. | Scriptable text output. |
| `plugin` | | Install or update Git-backed plugins. | Network/Git operations; credential prompts possible. |
| `schedule` | `sched` | Add/list/remove/run scheduled recipe jobs and inspect scheduled sessions. | Scriptable, though it mutates scheduler state. |
| `gateway` | `gw` | Manage external platform gateways. | Start/pair flows require tokens and external user action. |
| `update` | | Update the Goose CLI. | Mutates installed binary; `--reconfigure` can prompt. |
| `term` | | Terminal-integrated persistent sessions. | `init`, `info`, and hidden `log` are scriptable; `run` sends prompts to a persistent session. |
| `tui` | | Launch the Goose terminal UI. | Interactive TUI; may invoke Node/npx. |
| `local-models` | `lm` | Search/download/list/delete local inference models. | Scriptable but network/storage side effects for search/download/delete. |
| `completion` | | Generate shell completions. | Scriptable. |
| `review` | | Review the current diff using Goose and `.agents/checks/*.md`. | Scriptable; may spawn multiple `goose run` subprocesses. |
| `validate-extensions` | | Validate bundled extension JSON. | Hidden in top-level help but callable and scriptable. |

## CLI Switch Inventory

The frontmatter `cli_switches` array is the full switch inventory captured from release `1.41.0` help and current source for hidden flags. Important wrapper switches:

- `goose run --output-format text|json|stream-json`, default `text`.
- `goose run --no-session` to avoid persisted session files.
- `goose run --provider <PROVIDER> --model <MODEL>` for per-run model/provider overrides.
- `goose run --quiet` for cleaner text output when structured output is not used.
- `goose run --max-turns <NUMBER>` and `--max-tool-repetitions <NUMBER>` to bound long or looping runs.
- `goose run --with-extension`, `--with-streamable-http-extension`, `--with-builtin`, and `--no-profile` to control extension surfaces.
- `goose run --system <TEXT>` exists for additional system instructions. See the sibling [system-prompt Goose research](../system-prompt/goose.md) for delivery semantics.
- `goose info --verbose` gives human-readable effective paths/config.
- `goose session list --format json`, `goose session export --format json`, and `goose recipe list --format json` provide parseable inventories.

Docs/help disagreement: the Running Tasks guide mentions `--with-remote-extension`, but the `1.41.0` binary rejects it with `unexpected argument '--with-remote-extension'` and suggests `--with-extension`. The inventory trusts release help and source over that docs line.

## Configuration Discovery

Goose uses YAML config files plus state/data directories. The most reliable discovery command is `goose info`, because docs simplify some paths while source uses platform app directories with `Block/goose` preserved for backward compatibility.

Observed with an isolated normal `HOME`, `goose info` reports:

- Config dir: `~/.config/goose`
- Config yaml: `~/.config/goose/config.yaml`
- Sessions DB: `~/.local/share/goose/sessions/sessions.db`
- Logs dir: `~/.local/state/goose/logs`

When `GOOSE_PATH_ROOT` is set, source routes config, data, state, plugins, and agents under that root:

- `<root>/config/config.yaml`
- `<root>/data/sessions/sessions.db`
- `<root>/state/logs/`
- `<root>/.agents/plugins/`
- `<root>/.agents/agents/`

Important side effect: `goose info` is not read-only. Under an isolated `GOOSE_PATH_ROOT`, it created `data/projects.json` and `state/logs/cli/<date>/<timestamp>.log` even though `config/config.yaml` was missing.

No real user Goose config was found under `/Users/ken/.config`, `/Users/ken/Library/Application Support`, `/Users/ken/.local/share`, `/Users/ken/.local/state`, or `/Users/ken/.agents/plugins` during this run. The active Codex process has `HOME=/Users/ken/.claudine`, so default Goose paths in this session differ from a normal user shell.

## Environment Variables

General CLI/runtime variables are captured in frontmatter. Provider endpoint/API-key variables, provider model variables, permission policy variables, MCP-specific variables, logging variables, and streaming-specific variables are intentionally left to narrower topics unless they affect general wrapper behavior.

High-impact wrapper variables:

- `GOOSE_PATH_ROOT` isolates config/data/state and is the safest way to keep wrapper runs from mutating user Goose state.
- `GOOSE_DISABLE_KEYRING` changes secret storage behavior.
- `GOOSE_PROMPT_EDITOR` can trigger editor-based prompting in interactive sessions.
- `GOOSE_CLI_THEME`, `GOOSE_CLI_SHOW_THINKING`, `GOOSE_RANDOM_THINKING_MESSAGES`, and code-truncation variables affect human output.
- `GOOSE_SERVER__SECRET_KEY` is required for authenticated `goose serve`.
- `GOOSE_TUI_SCRIPT` and `GOOSE_TUI_NPM_SPEC` affect `goose tui` resolution.
- `CONFIGURE=false` prevents install scripts from launching interactive configuration.
- Goose sets `GOOSE_TERMINAL=1`, `AGENT=goose`, and `AGENT_SESSION_ID` in command/extension contexts so scripts can detect agent execution.

## Machine Introspection

Goose has useful parseable commands for sessions, recipes, and run output, but no single documented `doctor --json`, config-schema dump, full model catalog dump, MCP server list, or tool catalog dump was found in `1.41.0`.

Useful commands:

```bash
goose info --verbose
goose info --check
goose session list --format json
goose session export --session-id <id> --format json
goose session diagnostics --session-id <id> --output <file>
goose recipe list --format json
goose skills list
goose local-models list
goose run --output-format json --no-session -t "<prompt>"
goose run --output-format stream-json --no-session -t "<prompt>"
```

`goose info --verbose` is useful for discovery but not clean machine data. `goose session diagnostics` is JSON but may include sensitive session/config/log content. `goose local-models list` only covers downloaded local models and is not a provider model catalog.

## Wrapper Notes

Use `goose run --output-format stream-json --no-session -t ...` for live wrappers. Use `--output-format json` for batch workflows that only need final structured output. Use `GOOSE_PATH_ROOT` for isolation and expect `goose info` to create state/log files.

Avoid interactive surfaces unless explicitly requested: `configure`, `doctor`, `session --edit`, `run --interactive`, update reconfiguration, gateway pair/start flows, plugin install/update, and `tui`.

Do not rely on docs-only `--with-remote-extension`; the verified `1.41.0` binary rejects it. Use `--with-extension` for stdio commands and `--with-streamable-http-extension` for streamable HTTP MCP endpoints.

Treat debug output, diagnostics output, verbose config, and session exports as sensitive. They can include prompts, tool results, local paths, config, logs, and secrets.

## Changelog

- Updated on 2026-07-03 against release `v1.41.0` from GitHub and the downloaded macOS release binary.
- Replaced prior "no local binary" evidence with a stronger finding: no installed `goose` is on `PATH`, but a temporary release binary was inspected and reports `1.41.0`.
- Added local config discovery results: no real user Goose config found under `/Users/ken`; `goose info` creates `projects.json` and log directories under an isolated root.
- Clarified Block-to-AAIF project relocation while preserving compatibility notes for `Block/goose` paths and artifact names.
- Added hidden callable surfaces (`validate-extensions`, `term log`) and current `cron-help` schedule command.
- Recorded that `--with-remote-extension` is documented in one guide but rejected by the verified binary.

## Sources

- [Goose homepage](https://goose-docs.ai/)
- [Goose installation docs](https://goose-docs.ai/docs/getting-started/installation/)
- [Goose CLI commands docs](https://goose-docs.ai/docs/guides/goose-cli-commands/)
- [Running Tasks guide](https://goose-docs.ai/docs/guides/running-tasks/)
- [Configuration Files guide](https://goose-docs.ai/docs/guides/config-files/)
- [Environment Variables guide](https://goose-docs.ai/docs/guides/environment-variables/)
- [Known Issues guide](https://goose-docs.ai/docs/troubleshooting/known-issues/)
- [Goose repository](https://github.com/aaif-goose/goose)
- [Requested legacy Block repository](https://github.com/block/goose)
- [Goose CLI source](https://github.com/aaif-goose/goose/blob/main/crates/goose-cli/src/cli.rs)
- [Goose path source](https://github.com/aaif-goose/goose/blob/main/crates/goose/src/config/paths.rs)
- [Goose MCP server runner source](https://github.com/aaif-goose/goose/blob/main/crates/goose-mcp/src/mcp_server_runner.rs)
- [Latest release `v1.41.0`](https://github.com/aaif-goose/goose/releases/tag/v1.41.0)
- Local command: `command -v goose; goose --version; goose --help` returned `command not found` for installed Goose in this session.
- Local command: downloaded `goose-aarch64-apple-darwin.tar.bz2` from release `v1.41.0`; `/tmp/.../goose --version` printed `1.41.0`.
- Local command: `/tmp/.../goose --help` and nested `--help` probes for every top-level and nested command.
- Local command: `GOOSE_PATH_ROOT=/tmp/... /tmp/.../goose info --verbose` and `goose info --check` to observe config paths, side effects, and unconfigured-provider failure.
- Local command: searched `/Users/ken/.config`, `/Users/ken/Library/Application Support`, `/Users/ken/.local/share`, `/Users/ken/.local/state`, and `/Users/ken/.agents` for Goose config/state.
