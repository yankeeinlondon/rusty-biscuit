---
$schema: ./_schema.yaml
created: 2026-07-02
last_updated: 2026-07-02
agent: codex
model: default
latest_version: "2.1.199"
homepage: https://claude.ai/code
repo: null
docs: https://code.claude.com/docs/en/overview
cli_docs: https://code.claude.com/docs/en/cli-reference
binaries:
  - os: macos
    binary: claude
    alt_binaries: []
    notes: "Native, Homebrew, and npm installs expose the `claude` command. Local inspection found `/Users/ken/.local/bin/claude`."
  - os: linux
    binary: claude
    alt_binaries: []
    notes: "Native, apt, dnf, apk, and npm installs expose the `claude` command."
  - os: windows
    binary: claude.exe
    alt_binaries: ["claude"]
    notes: "Native and WinGet installs provide a signed `claude.exe`; docs show launching `claude` from PowerShell or CMD after installation."
install_methods:
  - os: macos
    method: other
    command: "curl -fsSL https://claude.ai/install.sh | bash"
    notes: "Recommended native installer; auto-updates in the background."
  - os: linux
    method: other
    command: "curl -fsSL https://claude.ai/install.sh | bash"
    notes: "Recommended native installer for Linux and WSL; auto-updates in the background."
  - os: windows
    method: other
    command: "irm https://claude.ai/install.ps1 | iex"
    notes: "Recommended native PowerShell installer; CMD installer is `curl -fsSL https://claude.ai/install.cmd -o install.cmd && install.cmd && del install.cmd`."
  - os: macos
    method: brew
    command: "brew install --cask claude-code"
    notes: "Stable-channel Homebrew cask. `claude-code@latest` tracks the latest channel."
  - os: windows
    method: winget
    command: "winget install Anthropic.ClaudeCode"
    notes: "WinGet installs do not auto-update unless package-manager auto-update is enabled."
  - os: linux
    method: package_manager
    command: "sudo apt install claude-code"
    notes: "Debian/Ubuntu after adding the signed Anthropic apt repository."
  - os: linux
    method: package_manager
    command: "sudo dnf install claude-code"
    notes: "Fedora/RHEL after adding the signed Anthropic rpm repository."
  - os: linux
    method: package_manager
    command: "apk add claude-code"
    notes: "Alpine after adding the signed Anthropic apk repository."
  - os: all
    method: npm
    command: "npm install -g @anthropic-ai/claude-code"
    notes: "As of v2.1.198, npm requires Node.js 22+ for install-time checks but installs a native binary that does not invoke Node at runtime."
subcommands:
  - name: "<none>"
    description: "Starts an interactive session by default."
    non_interactive: false
    notes: "A positional prompt starts an interactive session with an initial prompt unless `--print` is used."
  - name: "--print"
    description: "Print-mode query that exits after responding."
    non_interactive: true
    notes: "Documented as `claude -p \"query\"`; supports text, JSON, and stream-json output modes."
  - name: "update"
    description: "Updates Claude Code to the latest allowed version."
    non_interactive: true
    notes: "Can be blocked by `DISABLE_UPDATES`; native installs also auto-update."
  - name: "install"
    description: "Installs or reinstalls the native binary, optionally at a version or release channel."
    non_interactive: true
    notes: "Accepts a version such as `2.1.89`, or `stable`/`latest`."
  - name: "auth login"
    description: "Signs in to an Anthropic account."
    non_interactive: false
    notes: "May open a browser, prompt for SSO, or use console/API billing options."
  - name: "auth logout"
    description: "Logs out from the current Anthropic account."
    non_interactive: false
    notes: "Mutates local authentication state."
  - name: "auth status"
    description: "Reports authentication status."
    non_interactive: true
    notes: "Default output is JSON; exits 0 when logged in and 1 when not logged in."
  - name: "agents"
    description: "Opens agent view or prints active/background sessions with `--json`."
    non_interactive: false
    notes: "`claude agents --json` is non-interactive and locally returned a JSON array."
  - name: "attach"
    description: "Attaches to a background session in the current terminal."
    non_interactive: false
    notes: "Requires an interactive terminal."
  - name: "auto-mode defaults"
    description: "Prints the built-in auto-mode classifier rules as JSON."
    non_interactive: true
    notes: "Useful for policy inspection and wrapper diagnostics."
  - name: "auto-mode config"
    description: "Prints effective auto-mode classifier configuration as JSON."
    non_interactive: true
    notes: "Applies user, project, and managed settings before printing."
  - name: "auto-mode critique"
    description: "Gets AI feedback on custom auto-mode rules."
    non_interactive: false
    notes: "May require model access; not treated as static metadata."
  - name: "daemon status"
    description: "Prints background-session supervisor diagnostics."
    non_interactive: true
    notes: "Local output was text, not JSON."
  - name: "daemon stop"
    description: "Stops the background-session supervisor and optionally worker sessions."
    non_interactive: true
    notes: "Use `--any` to confirm stopping an on-demand supervisor."
  - name: "doctor"
    description: "Runs installation and configuration diagnostics."
    non_interactive: true
    notes: "Docs recommend it for diagnostics; local execution did not return promptly in this non-interactive run."
  - name: "gateway"
    description: "Starts the self-hosted Claude apps gateway server."
    non_interactive: true
    notes: "Requires `--config gateway.yaml`; available in v2.1.195 and later."
  - name: "logs"
    description: "Prints recent output from a background session."
    non_interactive: true
    notes: "Takes a background session id."
  - name: "mcp"
    description: "Configures MCP servers."
    non_interactive: false
    notes: "`mcp list` is text-oriented; login/logout subcommands manage OAuth credentials."
  - name: "plugin"
    description: "Manages Claude Code plugins."
    non_interactive: false
    notes: "Alias: `plugins`; `plugin list --json` is a machine-readable subcommand."
  - name: "project purge"
    description: "Deletes local Claude Code state for a project."
    non_interactive: false
    notes: "Use `--dry-run` to preview; `-y`/`--yes` skips confirmation."
  - name: "remote-control"
    description: "Starts a server-mode Remote Control session."
    non_interactive: false
    notes: "Runs a local server and enables control from Claude.ai or Claude apps."
  - name: "respawn"
    description: "Restarts a background session with its conversation intact."
    non_interactive: true
    notes: "Accepts a session id or `--all`."
  - name: "rm"
    description: "Removes a background session from the active list."
    non_interactive: true
    notes: "Transcript remains available through resume."
  - name: "setup-token"
    description: "Generates a long-lived OAuth token for CI and scripts."
    non_interactive: false
    notes: "Prints a secret token; requires a Claude subscription."
  - name: "stop"
    description: "Stops a background session."
    non_interactive: true
    notes: "Alias: `kill`."
  - name: "ultrareview"
    description: "Runs ultrareview non-interactively."
    non_interactive: true
    notes: "Supports `--json`; exits 0 on success and 1 on failure."
cli_switches:
  - flag: --add-dir
    value: "<directories...>"
    scope: ["global", "filesystem"]
    default: ""
    description: "Adds additional directories Claude can read and edit."
    example: "claude --add-dir ../apps ../lib"
    notes: "Validates each path exists as a directory; most `.claude/` configuration is not discovered from added directories."
  - flag: --advisor
    value: "<model>"
    scope: ["global", "model_selection"]
    default: ""
    description: "Enables the server-side advisor tool for this session with a model alias or full model id."
    example: "claude --advisor opus"
    notes: "Requires v2.1.98 or later; aliases include `opus`, `sonnet`, and `fable`."
  - flag: --agent
    value: "<agent>"
    scope: ["global", "agents"]
    default: ""
    description: "Specifies an agent for the current session."
    example: "claude --agent my-custom-agent"
    notes: "Overrides the `agent` setting."
  - flag: --agents
    value: "<json>"
    scope: ["global", "agents"]
    default: ""
    description: "Defines custom subagents dynamically via JSON."
    example: "claude --agents '{\"reviewer\":{\"description\":\"Reviews code\",\"prompt\":\"You are a code reviewer\"}}'"
    notes: "Uses subagent frontmatter field names plus `prompt`."
  - flag: --allow-dangerously-skip-permissions
    value: ""
    scope: ["global", "permissions"]
    default: "false"
    description: "Adds bypassPermissions to the Shift+Tab mode cycle without starting in it."
    example: "claude --permission-mode plan --allow-dangerously-skip-permissions"
    notes: "Different from starting directly in bypass mode."
  - flag: --allowedTools
    value: "<tools...>"
    scope: ["global", "permissions"]
    default: ""
    description: "Allows matching tools to execute without prompting."
    example: "claude --allowedTools \"Bash(git log *)\" \"Read\""
    notes: "Alias: `--allowed-tools`."
  - flag: --allowed-tools
    value: "<tools...>"
    scope: ["global", "permissions"]
    default: ""
    description: "Alias for `--allowedTools`."
    example: "claude --allowed-tools \"Read\""
    notes: ""
  - flag: --append-system-prompt
    value: "<text>"
    scope: ["global", "system_prompt"]
    default: ""
    description: "Appends custom text to the default system prompt."
    example: "claude --append-system-prompt \"Always use TypeScript\""
    notes: "Works in interactive and non-interactive modes."
  - flag: --append-system-prompt-file
    value: "<path>"
    scope: ["global", "system_prompt"]
    default: ""
    description: "Loads additional system prompt text from a file and appends it to the default prompt."
    example: "claude --append-system-prompt-file ./extra-rules.txt"
    notes: "Works in interactive and non-interactive modes."
  - flag: --ax-screen-reader
    value: ""
    scope: ["global", "accessibility"]
    default: "false"
    description: "Renders screen-reader friendly flat text."
    example: "claude --ax-screen-reader"
    notes: "Takes precedence over `CLAUDE_AX_SCREEN_READER` and the `axScreenReader` setting; requires v2.1.181 or later."
  - flag: --bare
    value: ""
    scope: ["global", "configuration"]
    default: "false"
    description: "Runs minimal mode without auto-discovery of customizations."
    example: "claude --bare -p \"query\""
    notes: "Sets `CLAUDE_CODE_SIMPLE`; only Bash, file read, and file edit tools are available, plus MCP tools explicitly supplied by `--mcp-config`."
  - flag: --betas
    value: "<headers...>"
    scope: ["global", "api"]
    default: ""
    description: "Adds beta headers to API requests."
    example: "claude --betas interleaved-thinking"
    notes: "Documented for API key users."
  - flag: --continue
    value: ""
    scope: ["global", "sessions"]
    default: "false"
    description: "Continues the most recent conversation in the current directory."
    example: "claude -c -p \"Check for type errors\""
    notes: "Short alias: `-c`."
  - flag: -c
    value: ""
    scope: ["global", "sessions"]
    default: "false"
    description: "Short alias for `--continue`."
    example: "claude -c"
    notes: ""
  - flag: --debug
    value: "[topic]"
    scope: ["global", "diagnostics"]
    default: "false"
    description: "Enables debug mode or topic-specific debug logging."
    example: "claude --debug mcp"
    notes: "The `DEBUG=1` environment variable also enables debug mode."
  - flag: --dangerously-skip-permissions
    value: ""
    scope: ["global", "permissions"]
    default: "false"
    description: "Starts in bypass-permissions mode."
    example: "claude --dangerously-skip-permissions -p \"query\""
    notes: "Only a leading occurrence routes to `daemon` subcommands in v2.1.199."
  - flag: --disallowedTools
    value: "<tools...>"
    scope: ["global", "permissions"]
    default: ""
    description: "Denies matching tools."
    example: "claude --disallowedTools \"mcp__*\""
    notes: "Use with `--tools` when MCP tools must also be denied."
  - flag: --effort
    value: "<level>"
    scope: ["global", "model_selection"]
    default: ""
    description: "Sets effort level for models that support effort."
    example: "claude --effort high"
    notes: "Accepted levels are model-dependent; `CLAUDE_EFFORT` is exported to Bash tool subprocesses and hooks."
  - flag: --help
    value: ""
    scope: ["global", "help"]
    default: "false"
    description: "Displays help."
    example: "claude --help"
    notes: "Official docs state `claude --help` does not list every flag."
  - flag: -h
    value: ""
    scope: ["global", "help"]
    default: "false"
    description: "Short alias for `--help`."
    example: "claude -h"
    notes: ""
  - flag: --input-format
    value: "<format>"
    scope: ["print", "io"]
    default: "text"
    description: "Selects input format for print-mode automation."
    example: "claude -p --input-format stream-json --output-format stream-json --verbose"
    notes: "`stream-json` is required for `--replay-user-messages`."
  - flag: --max-budget-usd
    value: "<amount>"
    scope: ["print", "limits"]
    default: ""
    description: "Limits spend for a print-mode query."
    example: "claude -p --max-budget-usd 1.00 \"summarize\""
    notes: "Known from public CLI reference and local help surface; exact enforcement semantics are not described in this research."
  - flag: --max-turns
    value: "<n>"
    scope: ["print", "limits"]
    default: ""
    description: "Limits the number of turns in non-interactive mode."
    example: "claude -p --max-turns 3 \"fix the bug\""
    notes: "Useful for wrappers to bound agent loops."
  - flag: --mcp-config
    value: "<path-or-json>"
    scope: ["global", "mcp"]
    default: ""
    description: "Loads MCP server configuration from a file or inline JSON."
    example: "claude --mcp-config ./mcp.json"
    notes: "Use `--strict-mcp-config` to ignore other MCP sources."
  - flag: --model
    value: "<model>"
    scope: ["global", "model_selection"]
    default: "provider/account default"
    description: "Selects the model for the session."
    example: "claude --model sonnet"
    notes: "Overridden by in-session `/model`; overrides model environment variable/settings for normal model selection."
  - flag: --output-format
    value: "<text|json|stream-json>"
    scope: ["print", "io"]
    default: "text"
    description: "Selects print-mode output format."
    example: "claude -p --output-format stream-json --verbose \"query\""
    notes: "Some stream-json features require `--verbose`."
  - flag: --permission-mode
    value: "<mode>"
    scope: ["global", "permissions"]
    default: ""
    description: "Sets the permission mode for the session."
    example: "claude --permission-mode plan"
    notes: "Can be passed to `claude agents` to set defaults for dispatched sessions."
  - flag: --plugin-dir
    value: "<path>"
    scope: ["global", "plugins"]
    default: ""
    description: "Loads a plugin from a directory or zip archive for this session only."
    example: "claude --plugin-dir ./my-plugin"
    notes: "Repeat the flag for multiple plugins."
  - flag: --plugin-url
    value: "<url>"
    scope: ["global", "plugins"]
    default: ""
    description: "Fetches a plugin zip archive from a URL for this session only."
    example: "claude --plugin-url https://example.com/plugin.zip"
    notes: "Repeat the flag or pass space-separated URLs in one quoted value."
  - flag: --print
    value: ""
    scope: ["global", "non_interactive"]
    default: "false"
    description: "Prints the response without interactive mode."
    example: "claude -p \"query\""
    notes: "Short alias: `-p`."
  - flag: -p
    value: ""
    scope: ["global", "non_interactive"]
    default: "false"
    description: "Short alias for `--print`."
    example: "claude -p \"query\""
    notes: ""
  - flag: --prompt-suggestions
    value: ""
    scope: ["print", "streaming"]
    default: "false"
    description: "Emits a prompt_suggestion message after each turn."
    example: "claude -p --prompt-suggestions --output-format stream-json --verbose \"query\""
    notes: "Requires `--print`, `--output-format stream-json`, and `--verbose`."
  - flag: --remote
    value: "<task>"
    scope: ["global", "remote"]
    default: ""
    description: "Creates a new web session on claude.ai with the provided task description."
    example: "claude --remote \"Fix the login bug\""
    notes: "Requires account and cloud-session support."
  - flag: --remote-control
    value: "[name]"
    scope: ["global", "remote"]
    default: "false"
    description: "Starts an interactive session with Remote Control enabled."
    example: "claude --remote-control \"My Project\""
    notes: "Short alias: `--rc`."
  - flag: --rc
    value: "[name]"
    scope: ["global", "remote"]
    default: "false"
    description: "Short alias for `--remote-control`."
    example: "claude --rc \"My Project\""
    notes: ""
  - flag: --remote-control-session-name-prefix
    value: "<prefix>"
    scope: ["global", "remote"]
    default: "machine hostname"
    description: "Sets the prefix for auto-generated Remote Control session names."
    example: "claude remote-control --remote-control-session-name-prefix dev-box"
    notes: "`CLAUDE_REMOTE_CONTROL_SESSION_NAME_PREFIX` provides the same effect."
  - flag: --replay-user-messages
    value: ""
    scope: ["print", "streaming"]
    default: "false"
    description: "Re-emits user messages from stdin back on stdout for acknowledgment."
    example: "claude -p --input-format stream-json --output-format stream-json --verbose --replay-user-messages"
    notes: "Requires stream-json input and output."
  - flag: --resume
    value: "<session>"
    scope: ["global", "sessions"]
    default: ""
    description: "Resumes a session by id or name."
    example: "claude -r \"auth-refactor\" \"Finish this PR\""
    notes: "Short alias: `-r`."
  - flag: -r
    value: "<session>"
    scope: ["global", "sessions"]
    default: ""
    description: "Short alias for `--resume`."
    example: "claude -r \"auth-refactor\""
    notes: ""
  - flag: --safe-mode
    value: ""
    scope: ["global", "configuration"]
    default: "false"
    description: "Launches with customizations disabled for troubleshooting."
    example: "claude --safe-mode"
    notes: "Sets `CLAUDE_CODE_SAFE_MODE`; managed policy still partially applies."
  - flag: --session-id
    value: "<uuid>"
    scope: ["global", "sessions"]
    default: ""
    description: "Uses a specific session id for the conversation."
    example: "claude --session-id \"550e8400-e29b-41d4-a716-446655440000\""
    notes: "Must be a valid UUID."
  - flag: --setting-sources
    value: "<sources>"
    scope: ["global", "configuration"]
    default: "user,project,local"
    description: "Selects which setting sources to load."
    example: "claude --setting-sources user,project"
    notes: "Comma-separated list using `user`, `project`, and `local`."
  - flag: --settings
    value: "<path-or-json>"
    scope: ["global", "configuration"]
    default: ""
    description: "Supplies a settings JSON file or inline JSON for this session."
    example: "claude --settings ./settings.json"
    notes: "Values override the same keys from discovered settings files; omitted keys keep file-based values."
  - flag: --strict-mcp-config
    value: ""
    scope: ["global", "mcp"]
    default: "false"
    description: "Only uses MCP servers from `--mcp-config`."
    example: "claude --strict-mcp-config --mcp-config ./mcp.json"
    notes: "Ignores all other MCP configurations."
  - flag: --system-prompt
    value: "<text>"
    scope: ["global", "system_prompt"]
    default: ""
    description: "Replaces the entire system prompt with custom text."
    example: "claude --system-prompt \"You are a Python expert\""
    notes: "Mutually exclusive with `--system-prompt-file`; works in interactive and non-interactive modes."
  - flag: --system-prompt-file
    value: "<path>"
    scope: ["global", "system_prompt"]
    default: ""
    description: "Loads a replacement system prompt from a file."
    example: "claude --system-prompt-file ./custom-prompt.txt"
    notes: "Mutually exclusive with `--system-prompt`; works in interactive and non-interactive modes."
  - flag: --teleport
    value: ""
    scope: ["global", "remote"]
    default: "false"
    description: "Resumes a web session in the local terminal."
    example: "claude --teleport"
    notes: "Requires web-session support."
  - flag: --teammate-mode
    value: "<in-process|auto|tmux|iterm2>"
    scope: ["global", "agents"]
    default: "in-process"
    description: "Sets how agent-team teammates display."
    example: "claude --teammate-mode auto"
    notes: "Added in v2.1.186; default changed from `auto` in v2.1.179."
  - flag: --tmux
    value: "[classic]"
    scope: ["global", "worktree"]
    default: "false"
    description: "Creates a tmux session for the worktree."
    example: "claude -w feature-auth --tmux"
    notes: "Requires `--worktree`; uses iTerm2 native panes when available."
  - flag: --tools
    value: "<tools>"
    scope: ["global", "permissions"]
    default: "default"
    description: "Restricts which built-in tools Claude can use."
    example: "claude --tools \"Bash,Edit,Read\""
    notes: "Use empty string to disable all built-in tools; MCP tools are not affected."
  - flag: --verbose
    value: ""
    scope: ["global", "output"]
    default: "false"
    description: "Enables verbose logging and full turn-by-turn output."
    example: "claude --verbose"
    notes: "Overrides the `viewMode` setting for this session."
  - flag: --version
    value: ""
    scope: ["global", "version"]
    default: "false"
    description: "Outputs the version number."
    example: "claude --version"
    notes: "Short alias: `-v`; local output was `2.1.199 (Claude Code)`."
  - flag: -v
    value: ""
    scope: ["global", "version"]
    default: "false"
    description: "Short alias for `--version`."
    example: "claude -v"
    notes: ""
  - flag: --worktree
    value: "[name|#pr|url]"
    scope: ["global", "worktree"]
    default: "false"
    description: "Starts Claude in an isolated git worktree under `.claude/worktrees/<name>`."
    example: "claude -w feature-auth"
    notes: "Short alias: `-w`; a GitHub PR number or URL fetches from origin and branches from it."
  - flag: -w
    value: "[name|#pr|url]"
    scope: ["global", "worktree"]
    default: "false"
    description: "Short alias for `--worktree`."
    example: "claude -w feature-auth"
    notes: ""
  - flag: --json
    value: ""
    scope: ["agents", "plugin list", "ultrareview"]
    default: "false"
    description: "Requests JSON output for commands that support it."
    example: "claude agents --json"
    notes: "`plugin list --json` and `ultrareview --json` are documented; `agents --json` was locally verified."
  - flag: --all
    value: ""
    scope: ["agents", "respawn", "project purge"]
    default: "false"
    description: "Includes or targets all records where supported."
    example: "claude agents --json --all"
    notes: "Meaning is command-specific."
  - flag: --cwd
    value: "<path>"
    scope: ["agents"]
    default: ""
    description: "Filters agent view to sessions started under a directory."
    example: "claude agents --cwd . --json"
    notes: "Documented for `claude agents`."
  - flag: --config
    value: "<path>"
    scope: ["gateway"]
    default: ""
    description: "Supplies gateway configuration."
    example: "claude gateway --config gateway.yaml"
    notes: "Required by `claude gateway`."
  - flag: --text
    value: ""
    scope: ["auth status"]
    default: "false"
    description: "Prints authentication status in human-readable text."
    example: "claude auth status --text"
    notes: "Without `--text`, `auth status` emits JSON."
  - flag: --email
    value: "<email>"
    scope: ["auth login"]
    default: ""
    description: "Pre-fills the login email address."
    example: "claude auth login --email user@example.com"
    notes: "Login may still be interactive."
  - flag: --sso
    value: ""
    scope: ["auth login"]
    default: "false"
    description: "Forces SSO authentication."
    example: "claude auth login --sso"
    notes: ""
  - flag: --console
    value: ""
    scope: ["auth login"]
    default: "false"
    description: "Signs in with Anthropic Console for API usage billing instead of a Claude subscription."
    example: "claude auth login --console"
    notes: ""
  - flag: --no-browser
    value: ""
    scope: ["mcp login"]
    default: "false"
    description: "Prints the OAuth authorization URL instead of opening a browser."
    example: "claude mcp login sentry --no-browser"
    notes: "Still requires pasting the redirect URL back at the prompt."
  - flag: --dry-run
    value: ""
    scope: ["project purge", "plugin prune", "plugin tag"]
    default: "false"
    description: "Previews the operation without applying it."
    example: "claude project purge ~/work/repo --dry-run"
    notes: "Command-specific semantics."
  - flag: --yes
    value: ""
    scope: ["project purge", "plugin prune"]
    default: "false"
    description: "Skips confirmation prompts."
    example: "claude project purge ~/work/repo --yes"
    notes: "Short alias: `-y` where documented."
  - flag: -y
    value: ""
    scope: ["project purge", "plugin prune"]
    default: "false"
    description: "Short alias for `--yes`."
    example: "claude project purge ~/work/repo -y"
    notes: ""
  - flag: --interactive
    value: ""
    scope: ["project purge"]
    default: "false"
    description: "Confirms each item during project purge."
    example: "claude project purge ~/work/repo --interactive"
    notes: "Short alias: `-i`."
  - flag: -i
    value: ""
    scope: ["project purge"]
    default: "false"
    description: "Short alias for `--interactive` in `project purge`."
    example: "claude project purge ~/work/repo -i"
    notes: ""
  - flag: --timeout
    value: "<minutes>"
    scope: ["ultrareview"]
    default: "30"
    description: "Overrides the ultrareview timeout."
    example: "claude ultrareview 1234 --timeout 10"
    notes: "Units are minutes."
  - flag: --any
    value: ""
    scope: ["daemon stop"]
    default: "false"
    description: "Confirms stopping an on-demand supervisor."
    example: "claude daemon stop --any"
    notes: "Documented as the default supervisor mode confirmation."
  - flag: --keep-workers
    value: ""
    scope: ["daemon stop"]
    default: "false"
    description: "Leaves background sessions running so the next supervisor reconnects."
    example: "claude daemon stop --any --keep-workers"
    notes: ""
  - flag: --available
    value: ""
    scope: ["plugin list"]
    default: "false"
    description: "Includes available plugins from marketplaces."
    example: "claude plugin list --json --available"
    notes: "Requires `--json`."
  - flag: --scope
    value: "<user|project|local|managed>"
    scope: ["plugin"]
    default: "user"
    description: "Selects plugin management scope."
    example: "claude plugin disable my-plugin --scope project"
    notes: "Supported values vary by plugin subcommand; short alias `-s` is documented for several plugin commands."
config_files:
  - os: all
    scope: user
    path: "~/.claude/settings.json"
    format: json
    notes: "User settings; on Windows `~/.claude` resolves to `%USERPROFILE%\\.claude`; `CLAUDE_CONFIG_DIR` relocates this tree."
  - os: all
    scope: repo
    path: ".claude/settings.json"
    format: json
    notes: "Project settings intended for source control."
  - os: all
    scope: repo
    path: ".claude/settings.local.json"
    format: json
    notes: "Project-local personal settings; Claude-created file is gitignored automatically."
  - os: macos
    scope: system
    path: "/Library/Application Support/ClaudeCode/managed-settings.json"
    format: json
    notes: "File-based managed settings."
  - os: linux
    scope: system
    path: "/etc/claude-code/managed-settings.json"
    format: json
    notes: "File-based managed settings for Linux and WSL."
  - os: windows
    scope: system
    path: "C:\\Program Files\\ClaudeCode\\managed-settings.json"
    format: json
    notes: "File-based managed settings. Legacy `C:\\ProgramData\\ClaudeCode\\managed-settings.json` is no longer supported as of v2.1.75."
  - os: macos
    scope: system
    path: "/Library/Application Support/ClaudeCode/managed-settings.d/*.json"
    format: json
    notes: "Drop-in managed settings fragments merged after `managed-settings.json`."
  - os: linux
    scope: system
    path: "/etc/claude-code/managed-settings.d/*.json"
    format: json
    notes: "Drop-in managed settings fragments merged alphabetically."
  - os: windows
    scope: system
    path: "C:\\Program Files\\ClaudeCode\\managed-settings.d\\*.json"
    format: json
    notes: "Drop-in managed settings fragments merged alphabetically."
  - os: all
    scope: user
    path: "~/.claude.json"
    format: json
    notes: "Stores OAuth session metadata, user/local MCP server configurations, per-project state, allowed tools, trust settings, and caches."
  - os: all
    scope: repo
    path: ".mcp.json"
    format: json
    notes: "Project-scoped MCP server configuration."
  - os: all
    scope: user
    path: "~/.claude/CLAUDE.md"
    format: text
    notes: "User memory/instructions file; included here because it is discovered as part of the CLI configuration surface."
  - os: all
    scope: repo
    path: "CLAUDE.md"
    format: text
    notes: "Project memory/instructions file."
  - os: all
    scope: repo
    path: ".claude/CLAUDE.md"
    format: text
    notes: "Project memory/instructions file under `.claude`."
  - os: all
    scope: repo
    path: "CLAUDE.local.md"
    format: text
    notes: "Local project memory file for private preferences."
env_vars:
  - name: CLAUDE_CONFIG_DIR
    effect: "Overrides the configuration directory, defaulting to `~/.claude`; settings, session history, plugins, and Linux/Windows credentials move under this path."
  - name: CLAUDE_CODE_SAFE_MODE
    effect: "Starts in safe mode, equivalent to `--safe-mode`, disabling user/project customizations while still applying managed policy in part."
  - name: CLAUDE_CODE_SIMPLE
    effect: "Runs bare/minimal mode, equivalent to `--bare`, with reduced tool and configuration discovery."
  - name: CLAUDE_AX_SCREEN_READER
    effect: "Controls screen-reader friendly output unless overridden by `--ax-screen-reader`."
  - name: CLAUDE_CODE_ACCESSIBILITY
    effect: "Keeps the native terminal cursor visible and disables the inverted-text cursor indicator for screen magnifiers."
  - name: CLAUDE_CODE_TMPDIR
    effect: "Overrides Claude Code's internal temp directory."
  - name: CLAUDE_CODE_PACKAGE_MANAGER_AUTO_UPDATE
    effect: "Lets Claude Code run Homebrew or WinGet package-manager upgrades in the background when a new version is available."
  - name: DISABLE_AUTOUPDATER
    effect: "Disables automatic background update checks; manual `claude update` still works."
  - name: DISABLE_UPDATES
    effect: "Blocks all update paths, including `claude update` and `claude install`."
  - name: DISABLE_INSTALLATION_CHECKS
    effect: "Disables installation warnings; useful only for externally managed installs."
  - name: DISABLE_DOCTOR_COMMAND
    effect: "Hides the interactive `/doctor` command."
  - name: DISABLE_LOGIN_COMMAND
    effect: "Hides the interactive `/login` command for externally managed authentication."
  - name: DISABLE_LOGOUT_COMMAND
    effect: "Hides the interactive `/logout` command."
  - name: DISABLE_TELEMETRY
    effect: "Opts out of telemetry and disables GrowthBook feature-flag fetching."
  - name: DO_NOT_TRACK
    effect: "Cross-tool telemetry opt-out honored as equivalent to `DISABLE_TELEMETRY`."
  - name: DEBUG
    effect: "Truthy values enable debug mode, equivalent to `--debug`; logs go under `~/.claude/debug/` unless redirected."
  - name: CLAUDE_CODE_DEBUG_LOGS_DIR
    effect: "Overrides the directory for debug logs."
  - name: CLAUDE_CODE_SKIP_PROMPT_HISTORY
    effect: "Skips writing prompt history and session transcripts; sessions will not appear in resume or shell history surfaces."
  - name: CLAUDE_CODE_SYNC_PLUGIN_INSTALL
    effect: "In print mode, waits for plugin installation to complete before the first query."
  - name: CLAUDE_CODE_SYNC_PLUGIN_INSTALL_TIMEOUT_MS
    effect: "Bounds synchronous plugin installation wait time."
  - name: CLAUDE_CODE_PLUGIN_CACHE_DIR
    effect: "Overrides the plugins root directory; marketplaces and plugin cache live under it."
  - name: CLAUDE_CODE_PLUGIN_GIT_TIMEOUT_MS
    effect: "Sets git operation timeout for plugin install/update."
  - name: CLAUDE_CODE_PLUGIN_KEEP_MARKETPLACE_ON_FAILURE
    effect: "Keeps an existing marketplace cache if update fails, useful offline."
  - name: CLAUDE_CODE_PLUGIN_PREFER_HTTPS
    effect: "Clones GitHub shorthand plugin sources over HTTPS instead of SSH."
  - name: CLAUDE_CODE_SYNC_SKILLS
    effect: "In print mode, downloads enabled claude.ai skills into `~/.claude/skills/` before the first query and periodically resyncs."
  - name: CLAUDE_CODE_SYNC_SKILLS_WAIT_TIMEOUT_MS
    effect: "Bounds the initial print-mode wait for skill sync."
  - name: CLAUDE_CODE_SYNC_SKILLS_INSTALL_TIMEOUT_MS
    effect: "Bounds mid-session skill resync."
  - name: CLAUDE_CODE_SYNTAX_HIGHLIGHT
    effect: "Set to `false` to disable syntax highlighting in diff output."
  - name: CLAUDE_CODE_TMUX_TRUECOLOR
    effect: "Allows 24-bit truecolor output inside tmux when tmux is configured for truecolor."
  - name: CLAUDE_CODE_USE_POWERSHELL_TOOL
    effect: "Controls availability of the PowerShell tool across platforms."
  - name: CLAUDE_CODE_GIT_BASH_PATH
    effect: "On Windows, points Claude Code at Git Bash when auto-discovery fails."
  - name: CLAUDE_REMOTE_CONTROL_SESSION_NAME_PREFIX
    effect: "Sets the prefix for auto-generated Remote Control session names."
  - name: API_TIMEOUT_MS
    effect: "Sets API request timeout; can also be placed under the `env` key in settings files."
  - name: BASH_DEFAULT_TIMEOUT_MS
    effect: "Sets default Bash tool timeout when placed in shell environment or settings `env`."
  - name: USE_BUILTIN_RIPGREP
    effect: "Set to `0` to use system `rg` instead of the bundled ripgrep."
machine_introspection:
  - command: "claude auth status"
    purpose: env
    machine_readable: true
    output_format: json
    useful_for_codegen: false
    notes: "Reports `loggedIn`, `authMethod`, and `apiProvider`; exits 1 when not logged in, which is an expected state."
  - command: "claude agents --json"
    purpose: other
    machine_readable: true
    output_format: json
    useful_for_codegen: false
    notes: "Lists active sessions with pid, cwd, kind, start timestamp, session id/name, and status. Useful for wrapper diagnostics and resume UX."
  - command: "claude agents --json --all"
    purpose: other
    machine_readable: true
    output_format: json
    useful_for_codegen: false
    notes: "Includes completed background sessions according to docs."
  - command: "claude auto-mode defaults"
    purpose: config_schema
    machine_readable: true
    output_format: json
    useful_for_codegen: true
    notes: "Prints built-in auto-mode classifier rules as JSON."
  - command: "claude auto-mode config"
    purpose: config_dump
    machine_readable: true
    output_format: json
    useful_for_codegen: true
    notes: "Prints effective auto-mode classifier config after settings are applied."
  - command: "claude plugin list --json"
    purpose: plugins
    machine_readable: true
    output_format: json
    useful_for_codegen: false
    notes: "Lists installed plugins with version, marketplace/source, enable status, install path, and plugin-provided MCP servers when present."
  - command: "claude plugin list --json --available"
    purpose: plugins
    machine_readable: true
    output_format: json
    useful_for_codegen: false
    notes: "Includes available plugins from marketplaces."
  - command: "claude daemon status"
    purpose: doctor
    machine_readable: false
    output_format: text
    useful_for_codegen: false
    notes: "Prints supervisor state, socket directory, worker count, roster, and log presence; local output was text."
  - command: "claude doctor"
    purpose: doctor
    machine_readable: false
    output_format: text
    useful_for_codegen: false
    notes: "Officially recommended for installation/config diagnostics. Local non-interactive execution did not return within 10 seconds and was interrupted."
  - command: "claude mcp list"
    purpose: mcp
    machine_readable: false
    output_format: text
    useful_for_codegen: false
    notes: "Local output was human text. MCP is covered by the narrower MCP research topic."
wrapper_notes:
  - "Use `claude -p` / `--print` for non-interactive wrapper runs; plain `claude \"query\"` starts an interactive session with an initial prompt."
  - "`--output-format stream-json` is the primary structured stream output for print mode; prompt suggestions and replayed user messages require stream-json plus `--verbose`."
  - "`claude --help` is not a complete flag source; official docs explicitly say absence from help does not mean a flag is unavailable."
  - "`claude auth status` emits JSON but exits 1 for the expected unauthenticated state; wrappers should classify that separately from crashes."
  - "Login, MCP OAuth login, Remote Control, attach, and many plugin/auth operations may require a TTY, browser, or state mutation."
  - "`claude doctor` is useful diagnostically but should be run with a timeout in non-interactive wrappers; it did not return promptly during this research run."
  - "`--bare` / `CLAUDE_CODE_SIMPLE` bypasses most user/project customization discovery and does not read OAuth/keychain credentials, so wrappers using it need API-key or helper-based auth."
  - "`--safe-mode` disables most customizations but managed policy can still partially apply."
  - "`CLAUDE_CONFIG_DIR` is the cleanest wrapper isolation knob for config/session/plugin state; on macOS credentials can still come from Keychain."
  - "Native/npm installs now spawn a per-platform native binary rather than bundled JavaScript; npm optional dependencies must be present."
  - "Homebrew stable cask can lag latest by about a week; npm `latest` and local `claude --version` both reported 2.1.199 on 2026-07-02."
  - "Windows without Git for Windows uses PowerShell for shell commands; with Git for Windows it uses Git Bash unless configured otherwise."
  - "Package-manager installs generally do not auto-update by default; native installs do."
changes: []
requires_claudine_update: true
reason: "Claude Code now exposes wrapper-relevant public CLI surfaces not represented in the older agent-cli research, including JSON auth status, JSON agent listing, auto-mode JSON config/defaults, native binary/package-manager installs, `--bare`, `--safe-mode`, Remote Control flags, and expanded system-prompt flags."
---

## Overview

Claude Code is Anthropic's official agentic coding CLI. The public CLI uses the `claude` command on macOS/Linux and is launched as `claude` from Windows shells, backed by a native `claude.exe` binary on Windows. It starts an interactive session by default; non-interactive wrappers should use `-p` / `--print`.

The latest verified version for this research is `2.1.199`: local `claude --version` returned `2.1.199 (Claude Code)`, and `npm view @anthropic-ai/claude-code` reported `latest: 2.1.199`, `stable: 2.1.191`, and `next: 2.1.200` on 2026-07-02. No public source repository for the Claude Code CLI itself was found in the official docs.

## Installation and Binaries

Official installation options include the native installer, Homebrew casks on macOS, WinGet on Windows, signed apt/dnf/apk repositories on Linux, and the global npm package `@anthropic-ai/claude-code`.

The native installer commands are:

```sh
curl -fsSL https://claude.ai/install.sh | bash
irm https://claude.ai/install.ps1 | iex
curl -fsSL https://claude.ai/install.cmd -o install.cmd && install.cmd && del install.cmd
```

Homebrew provides `claude-code` for the stable channel and `claude-code@latest` for the latest channel. WinGet uses `winget install Anthropic.ClaudeCode`. Linux package-manager installs use package name `claude-code` after adding Anthropic's signed package repository.

The npm install is:

```sh
npm install -g @anthropic-ai/claude-code
```

Official setup docs say npm now installs the same native binary as the standalone installer through per-platform optional dependencies. Supported npm binary platforms are `darwin-arm64`, `darwin-x64`, `linux-x64`, `linux-arm64`, `linux-x64-musl`, `linux-arm64-musl`, `win32-x64`, and `win32-arm64`.

## Subcommands

The documented top-level command surface includes interactive sessions, print mode, update/install, authentication, agent/background-session management, auto-mode inspection, daemon diagnostics/control, gateway server mode, MCP management, plugin management, project state purge, Remote Control, session respawn/removal/stop, CI token generation, and ultrareview.

Important non-interactive subcommands for wrappers are:

- `claude -p "query"` for one-shot execution.
- `claude auth status` for JSON authentication status.
- `claude agents --json` for active/background session listing.
- `claude auto-mode defaults` and `claude auto-mode config` for JSON policy inspection.
- `claude plugin list --json` for plugin inventory.
- `claude ultrareview [target] --json` for ultrareview's raw payload.

Commands likely to need a TTY, browser, or user interaction include `auth login`, `attach`, Remote Control sessions, MCP OAuth login, and most mutating plugin/auth workflows.

## CLI Switch Inventory

The frontmatter `cli_switches` section records the wrapper-relevant inventory verified from official CLI documentation and local help/version inspection. The official docs warn that `claude --help` does not list every supported flag, so wrappers should treat the docs as more authoritative than help output.

System prompt flags are wrapper-critical:

- `--system-prompt` and `--system-prompt-file` replace the default prompt and are mutually exclusive.
- `--append-system-prompt` and `--append-system-prompt-file` append to the default prompt and may be combined with either replacement form.
- All four work in interactive and non-interactive modes.

For non-interactive execution, use `--print` and select output with `--output-format`. The stream-json protocol is required by `--prompt-suggestions` and `--replay-user-messages`, and both also require `--verbose` according to the docs.

## Configuration Discovery

Claude Code uses hierarchical JSON settings:

- User settings: `~/.claude/settings.json`.
- Project settings: `.claude/settings.json`.
- Local project settings: `.claude/settings.local.json`.
- Managed settings: server-managed settings, MDM/registry/plist policy, or file-based `managed-settings.json`.

Settings precedence is managed, then command-line arguments, then local, then project, then user. Permission rules merge differently from scalar settings.

Additional discovered state includes `~/.claude.json` for OAuth/session metadata, MCP configurations, trust and per-project state, and `.mcp.json` for project MCP servers. Memory and instruction files are also discovered from `~/.claude/CLAUDE.md`, `CLAUDE.md`, `.claude/CLAUDE.md`, and `CLAUDE.local.md`. `CLAUDE_CONFIG_DIR` relocates the default `~/.claude` tree.

## Environment Variables

The official environment variable page is broad and includes model selection, authentication, MCP, permissions, logging, telemetry, streaming, and provider-routing variables. This document only records general CLI/runtime variables in frontmatter.

Wrapper-impacting general variables include `CLAUDE_CONFIG_DIR`, `CLAUDE_CODE_SAFE_MODE`, `CLAUDE_CODE_SIMPLE`, `CLAUDE_AX_SCREEN_READER`, `CLAUDE_CODE_TMPDIR`, update-disabling variables, plugin/skill sync variables for print mode, and Windows shell-selection variables. Environment variables can also be set through settings files under the `env` key and are read at startup.

## Machine Introspection

Useful machine-readable surfaces found:

- `claude auth status`: JSON auth state; exits `1` when not logged in.
- `claude agents --json`: JSON active/background sessions.
- `claude auto-mode defaults`: JSON built-in classifier rules.
- `claude auto-mode config`: JSON effective classifier config.
- `claude plugin list --json`: JSON plugin inventory.

Useful but text-oriented diagnostics:

- `claude daemon status`.
- `claude doctor`.
- `claude mcp list`.

`claude doctor` should be guarded with a timeout in wrappers. In this non-interactive run it did not return within 10 seconds and was interrupted.

## Wrapper Notes

Claudine should prefer `claude -p` for non-interactive execution and should explicitly choose `--output-format` rather than inferring from defaults. For structured streaming, require `--output-format stream-json`; add `--verbose` when using prompt suggestions or replayed user messages.

Auth status needs special handling because an unauthenticated user is represented as a JSON object plus exit code `1`, not as malformed output. A wrapper that reports provider readiness should parse stdout before classifying the exit.

For isolation, `CLAUDE_CONFIG_DIR` is the broadest public knob. On macOS, credentials may still live in Keychain. `--bare` is stronger for avoiding project/user customization, but it also avoids OAuth/keychain credentials and requires API-key or helper-based authentication.

## Changelog

No update-mode changelog entries were requested for this first `claude.md` research file.

## Sources

- [Claude Code overview](https://code.claude.com/docs/en/overview)
- [Claude Code CLI reference](https://code.claude.com/docs/en/cli-reference)
- [Advanced setup](https://code.claude.com/docs/en/setup)
- [Claude Code settings](https://code.claude.com/docs/en/settings)
- [Environment variables](https://code.claude.com/docs/en/env-vars)
- [Explore the .claude directory](https://code.claude.com/docs/en/claude-directory)
- [Debug your configuration](https://code.claude.com/docs/en/debug-your-config)
- [Configure auto mode](https://code.claude.com/docs/en/auto-mode-config)
- [Plugins reference](https://code.claude.com/docs/en/plugins-reference)
- Local inspection on 2026-07-02: `command -v claude`, `claude --version`, `claude auth status`, `claude agents --json`, `claude auto-mode defaults`, `claude auto-mode config`, `claude plugin list --json`, `claude daemon status`, `claude mcp list`, and `npm view @anthropic-ai/claude-code version dist-tags --json`.
