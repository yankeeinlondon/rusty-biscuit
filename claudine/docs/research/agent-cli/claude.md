---
$schema: ./_schema.yaml
created: 2026-07-02
last_updated: 2026-07-03
agent: codex
model: default
latest_version: "2.1.200"
homepage: https://claude.ai/code
repo: null
docs: https://code.claude.com/docs/en/overview
cli_docs: https://code.claude.com/docs/en/cli-reference
binaries:
  - os: macos
    binary: claude
    alt_binaries: []
    notes: "Native, Homebrew, and npm installs expose `claude`. Local macOS inspection found `/Users/ken/.local/bin/claude`, a symlink to `/Users/ken/.local/share/claude/versions/2.1.200`."
  - os: linux
    binary: claude
    alt_binaries: []
    notes: "Native, apt, dnf, apk, and npm installs expose `claude`."
  - os: windows
    binary: claude.exe
    alt_binaries: ["claude", "claude.cmd"]
    notes: "Native and WinGet installs provide a Windows executable launched as `claude` from PowerShell or CMD; npm installations commonly expose command shims."
install_methods:
  - os: macos
    method: other
    command: "curl -fsSL https://claude.ai/install.sh | bash"
    notes: "Recommended native installer; installs under `~/.local/bin` and `~/.local/share/claude`, and native installs auto-update."
  - os: linux
    method: other
    command: "curl -fsSL https://claude.ai/install.sh | bash"
    notes: "Recommended native installer for Linux and WSL; native installs auto-update."
  - os: windows
    method: other
    command: "irm https://claude.ai/install.ps1 | iex"
    notes: "Recommended native PowerShell installer; CMD form is `curl -fsSL https://claude.ai/install.cmd -o install.cmd && install.cmd && del install.cmd`."
  - os: macos
    method: brew
    command: "brew install --cask claude-code"
    notes: "Stable-channel Homebrew cask. `brew install --cask claude-code@latest` installs the latest channel. Homebrew installs do not auto-update through Claude Code."
  - os: windows
    method: winget
    command: "winget install Anthropic.ClaudeCode"
    notes: "WinGet installs do not auto-update through Claude Code."
  - os: linux
    method: package_manager
    command: "sudo apt install claude-code"
    notes: "Debian/Ubuntu after adding Anthropic's signed apt repository."
  - os: linux
    method: package_manager
    command: "sudo dnf install claude-code"
    notes: "Fedora/RHEL after adding Anthropic's signed rpm repository."
  - os: linux
    method: package_manager
    command: "apk add claude-code"
    notes: "Alpine after adding Anthropic's signed apk repository."
  - os: macos
    method: npm
    command: "npm install -g @anthropic-ai/claude-code"
    notes: "Requires Node.js 22+ for install-time engine checks as of v2.1.198; installs a native binary via optional dependencies."
  - os: linux
    method: npm
    command: "npm install -g @anthropic-ai/claude-code"
    notes: "Requires optional dependencies to be enabled; supported Linux npm binary platforms include glibc and musl x64/ARM64 builds."
  - os: windows
    method: npm
    command: "npm install -g @anthropic-ai/claude-code"
    notes: "Supported npm binary platforms include win32 x64 and ARM64; npm creates command shims."
subcommands:
  - name: "<none>"
    description: "Starts an interactive session by default."
    non_interactive: false
    notes: "A positional prompt starts an interactive session with an initial prompt unless `--print` is used."
  - name: "--print"
    description: "Runs a prompt in print/SDK mode and exits."
    non_interactive: true
    notes: "Documented and locally available as `claude -p \"query\"`; supports text, JSON, and stream-json output."
  - name: "agents"
    description: "Opens agent view for background sessions, or prints sessions with `--json`."
    non_interactive: false
    notes: "`claude agents --json` is non-interactive and locally returned a JSON array; opening agent view needs a TTY."
  - name: "attach"
    description: "Attaches to a background session in the current terminal."
    non_interactive: false
    notes: "Hidden from top-level help in 2.1.200 but `claude attach --help` works; requires an interactive terminal."
  - name: "auth"
    description: "Manages authentication."
    non_interactive: false
    notes: "`auth status` is non-interactive JSON by default; `auth login` and `auth logout` mutate local auth state and may require browser or TTY interaction."
  - name: "auto-mode"
    description: "Inspects auto mode classifier configuration."
    non_interactive: true
    notes: "`defaults` and `config` print JSON; `critique` may call a model and is not static metadata."
  - name: "daemon"
    description: "Manages the background-session supervisor."
    non_interactive: true
    notes: "Hidden from top-level help in 2.1.200 but `claude daemon --help` works; `logs` tails until interrupted."
  - name: "gateway"
    description: "Runs the enterprise auth/telemetry gateway."
    non_interactive: true
    notes: "Requires `--config gateway.yaml`; available in v2.1.195 and later."
  - name: "install"
    description: "Installs or reinstalls the native binary, optionally at a version or channel."
    non_interactive: true
    notes: "Accepts `stable`, `latest`, or a specific version; `--force` reinstalls."
  - name: "logs"
    description: "Prints recent terminal output from a background session."
    non_interactive: true
    notes: "Hidden from top-level help in 2.1.200 but `claude logs --help` works; requires a background session id."
  - name: "mcp"
    description: "Configures and manages MCP servers."
    non_interactive: false
    notes: "`mcp list` and `mcp get` are text-oriented; OAuth login/logout and project-choice reset mutate state."
  - name: "plugin"
    description: "Manages Claude Code plugins."
    non_interactive: false
    notes: "Alias: `plugins`; `plugin list --json` is machine-readable, while install/enable/disable/update mutate state."
  - name: "project"
    description: "Manages Claude Code project state."
    non_interactive: false
    notes: "`project purge --dry-run` previews deletion; mutating purge should use `--yes` for non-interactive runs if the wrapper deliberately requests it."
  - name: "remote-control"
    description: "Starts an interactive Remote Control session."
    non_interactive: false
    notes: "Also exposed as `--remote-control`; local logged-out probe exited 1 with an auth-required message."
  - name: "respawn"
    description: "Restarts one or all background sessions with their conversation intact."
    non_interactive: true
    notes: "Hidden from top-level help in 2.1.200 but `claude respawn --help` works."
  - name: "rm"
    description: "Deletes a background session and its worktree."
    non_interactive: true
    notes: "Hidden from top-level help in 2.1.200 but `claude rm --help` works; destructive for background-session state."
  - name: "setup-token"
    description: "Generates a long-lived authentication token for CI and scripts."
    non_interactive: false
    notes: "Requires a Claude subscription and prints a secret; treat as interactive/secrets flow."
  - name: "stop"
    description: "Stops a background session while keeping its conversation resumable."
    non_interactive: true
    notes: "Hidden from top-level help in 2.1.200 but `claude stop --help` works."
  - name: "ultrareview"
    description: "Runs a cloud-hosted multi-agent code review."
    non_interactive: true
    notes: "Supports `--json` and `--timeout`; requires auth and network access."
  - name: "update"
    description: "Checks for updates and installs if available."
    non_interactive: true
    notes: "Alias: `upgrade`; behavior depends on install method and update policy."
cli_switches:
  - flag: --add-dir
    value: "<directories...>"
    scope: ["global", "agents"]
    default: ""
    description: "Adds additional directories Claude can read and edit."
    example: "claude --add-dir ../apps ../lib"
    notes: "Docs say most `.claude/` configuration is not discovered from added directories."
  - flag: --advisor
    value: "<model>"
    scope: ["global"]
    default: ""
    description: "Enables the server-side advisor tool for this session."
    example: "claude --advisor opus"
    notes: "Documented but omitted from local 2.1.200 help; trusted docs because the CLI reference warns help is incomplete."
  - flag: --agent
    value: "<agent>"
    scope: ["global", "agents"]
    default: ""
    description: "Specifies an agent for the current session."
    example: "claude --agent my-custom-agent"
    notes: "Overrides the `agent` setting."
  - flag: --agents
    value: "<json>"
    scope: ["global"]
    default: ""
    description: "Defines custom subagents dynamically via JSON."
    example: "claude --agents '{\"reviewer\":{\"description\":\"Reviews code\",\"prompt\":\"You are a code reviewer\"}}'"
    notes: "Uses subagent frontmatter field names plus `prompt`."
  - flag: --allow-dangerously-skip-permissions
    value: ""
    scope: ["global"]
    default: "false"
    description: "Makes bypass-permissions mode available in the mode cycle without starting in it."
    example: "claude --permission-mode plan --allow-dangerously-skip-permissions"
    notes: "Boolean."
  - flag: --allowedTools
    value: "<tools...>"
    scope: ["global"]
    default: ""
    description: "Allows matching tools without prompting."
    example: "claude --allowedTools \"Bash(git log *)\" Read"
    notes: "Alias: `--allowed-tools`."
  - flag: --append-system-prompt
    value: "<prompt>"
    scope: ["global", "system_prompt"]
    default: ""
    description: "Appends inline system prompt text to the default prompt."
    example: "claude --append-system-prompt \"Always use TypeScript\""
    notes: "Existence only; semantics belong to the sibling `system-prompt` topic."
  - flag: --append-system-prompt-file
    value: "<path>"
    scope: ["global", "system_prompt"]
    default: ""
    description: "Appends system prompt text loaded from a file."
    example: "claude --append-system-prompt-file ./style-rules.txt"
    notes: "Documented but omitted from local 2.1.200 help; semantics belong to the sibling `system-prompt` topic."
  - flag: --ax-screen-reader
    value: ""
    scope: ["global"]
    default: "false"
    description: "Renders screen-reader friendly flat output."
    example: "claude --ax-screen-reader"
    notes: "Boolean; overrides the env/settings accessibility setting."
  - flag: --background
    value: ""
    scope: ["global", "background_agents"]
    default: "false"
    description: "Starts the session as a background agent and returns immediately."
    example: "claude --background \"investigate the flaky test\""
    notes: "Alias: `--bg`; docs say it cannot be combined with `--print`."
  - flag: --bare
    value: ""
    scope: ["global"]
    default: "false"
    description: "Runs minimal mode without most user/project customization discovery."
    example: "claude --bare -p \"query\""
    notes: "Sets `CLAUDE_CODE_SIMPLE=1` and avoids OAuth/keychain reads."
  - flag: --betas
    value: "<headers...>"
    scope: ["global"]
    default: ""
    description: "Adds beta headers to API requests."
    example: "claude --betas interleaved-thinking"
    notes: "Documented for API key users."
  - flag: --brief
    value: ""
    scope: ["global"]
    default: "false"
    description: "Enables the SendUserMessage tool for agent-to-user communication."
    example: "claude --brief"
    notes: "Observed in local 2.1.200 help; not found in the official CLI-reference flag table."
  - flag: --channels
    value: "<channels...>"
    scope: ["global"]
    default: ""
    description: "Subscribes to MCP channel notifications for this session."
    example: "claude --channels plugin:my-notifier@my-marketplace"
    notes: "Documented research preview; omitted from local 2.1.200 help."
  - flag: --chrome
    value: ""
    scope: ["global"]
    default: "false"
    description: "Enables Claude in Chrome integration."
    example: "claude --chrome"
    notes: "Boolean."
  - flag: --continue
    value: ""
    scope: ["global"]
    default: "false"
    description: "Continues the most recent conversation in the current directory."
    example: "claude -c -p \"Check for type errors\""
    notes: "Short alias: `-c`."
  - flag: --dangerously-load-development-channels
    value: "<channels...>"
    scope: ["global"]
    default: ""
    description: "Enables unapproved development channels."
    example: "claude --dangerously-load-development-channels server:webhook"
    notes: "Documented but omitted from local 2.1.200 help; prompts for confirmation."
  - flag: --dangerously-skip-permissions
    value: ""
    scope: ["global", "permissions"]
    default: "false"
    description: "Starts with permission checks bypassed."
    example: "claude --dangerously-skip-permissions"
    notes: "Equivalent to `--permission-mode bypassPermissions`."
  - flag: --debug
    value: "[filter]"
    scope: ["global", "diagnostics"]
    default: "false"
    description: "Enables debug mode with optional category filtering."
    example: "claude --debug api,mcp"
    notes: "Short alias: `-d`."
  - flag: --debug-file
    value: "<path>"
    scope: ["global", "diagnostics"]
    default: ""
    description: "Writes debug logs to a file and implicitly enables debug mode."
    example: "claude --debug-file /tmp/claude-debug.log"
    notes: "Takes precedence over `CLAUDE_CODE_DEBUG_LOGS_DIR`."
  - flag: --disable-slash-commands
    value: ""
    scope: ["global"]
    default: "false"
    description: "Disables all skills and slash commands for the session."
    example: "claude --disable-slash-commands"
    notes: "Boolean."
  - flag: --disallowedTools
    value: "<tools...>"
    scope: ["global"]
    default: ""
    description: "Denies matching tool calls or removes tools from context."
    example: "claude --disallowedTools \"Bash(rm *)\" Edit"
    notes: "Alias: `--disallowed-tools`."
  - flag: --effort
    value: "<low|medium|high|xhigh|max>"
    scope: ["global"]
    default: ""
    description: "Sets effort level for the current session."
    example: "claude --effort high"
    notes: "Overrides `effortLevel` for the session."
  - flag: --enable-auto-mode
    value: ""
    scope: ["global"]
    default: "removed"
    description: "Removed flag; auto mode is now in the mode cycle."
    example: "claude --permission-mode auto"
    notes: "Docs retain it as a removed flag; wrappers should not emit it."
  - flag: --exclude-dynamic-system-prompt-sections
    value: ""
    scope: ["global", "system_prompt"]
    default: "false"
    description: "Moves per-machine default-prompt sections into the first user message."
    example: "claude -p --exclude-dynamic-system-prompt-sections \"query\""
    notes: "Ignored with replacement system-prompt flags; semantics belong to the sibling `system-prompt` topic."
  - flag: --exec
    value: "<command>"
    scope: ["global", "background_agents"]
    default: ""
    description: "Runs a PTY-backed shell command as a background job when used with `--bg`."
    example: "claude --bg --exec 'pytest -x'"
    notes: "Documented but omitted from local 2.1.200 help."
  - flag: --fallback-model
    value: "<models>"
    scope: ["global"]
    default: ""
    description: "Specifies fallback model aliases or IDs for overloaded/unavailable primary models."
    example: "claude --fallback-model sonnet,haiku"
    notes: "Local help says it only works with `--print`; docs frame it as a session flag."
  - flag: --file
    value: "<file_id:relative_path...>"
    scope: ["global"]
    default: ""
    description: "Downloads file resources at startup."
    example: "claude --file file_abc:doc.txt"
    notes: "Observed in local 2.1.200 help; not found in the official CLI-reference flag table."
  - flag: --fork-session
    value: ""
    scope: ["global", "sessions"]
    default: "false"
    description: "Creates a new session ID when resuming."
    example: "claude --resume abc123 --fork-session"
    notes: "Use with `--resume` or `--continue`."
  - flag: --from-pr
    value: "[value]"
    scope: ["global", "sessions"]
    default: ""
    description: "Resumes a session linked to a pull request or opens an interactive picker."
    example: "claude --from-pr 123"
    notes: "Value may be a PR number or supported PR URL."
  - flag: --help
    value: ""
    scope: ["global", "subcommands"]
    default: "false"
    description: "Displays help."
    example: "claude --help"
    notes: "Short alias: `-h`; help is useful but incomplete."
  - flag: --ide
    value: ""
    scope: ["global"]
    default: "false"
    description: "Automatically connects to an IDE on startup if exactly one valid IDE is available."
    example: "claude --ide"
    notes: "Boolean."
  - flag: --include-hook-events
    value: ""
    scope: ["global", "print_mode"]
    default: "false"
    description: "Includes hook lifecycle events in stream-json output."
    example: "claude -p --output-format stream-json --verbose --include-hook-events \"query\""
    notes: "Requires `--output-format stream-json`."
  - flag: --include-partial-messages
    value: ""
    scope: ["global", "print_mode"]
    default: "false"
    description: "Includes partial streaming events as they arrive."
    example: "claude -p --output-format stream-json --verbose --include-partial-messages \"query\""
    notes: "Requires `--print` and stream-json output."
  - flag: --init
    value: ""
    scope: ["global", "print_mode"]
    default: "false"
    description: "Runs setup hooks with the `init` matcher before a print-mode session."
    example: "claude -p --init \"query\""
    notes: "Documented but omitted from local 2.1.200 help."
  - flag: --init-only
    value: ""
    scope: ["global"]
    default: "false"
    description: "Runs setup and SessionStart hooks, then exits without starting a conversation."
    example: "claude --init-only"
    notes: "Documented but omitted from local 2.1.200 help."
  - flag: --input-format
    value: "<text|stream-json>"
    scope: ["global", "print_mode"]
    default: "text"
    description: "Selects print-mode input format."
    example: "claude -p --output-format json --input-format stream-json"
    notes: "Only works with `--print`."
  - flag: --json-schema
    value: "<schema>"
    scope: ["global", "print_mode"]
    default: ""
    description: "Validates final structured output against a JSON Schema."
    example: "claude -p --json-schema '{\"type\":\"object\"}' \"query\""
    notes: "Print mode only."
  - flag: --maintenance
    value: ""
    scope: ["global", "print_mode"]
    default: "false"
    description: "Runs setup hooks with the `maintenance` matcher before a print-mode session."
    example: "claude -p --maintenance \"query\""
    notes: "Documented but omitted from local 2.1.200 help."
  - flag: --max-budget-usd
    value: "<amount>"
    scope: ["global", "print_mode"]
    default: ""
    description: "Stops print-mode execution after a dollar budget is reached."
    example: "claude -p --max-budget-usd 5.00 \"query\""
    notes: "Only works with `--print`."
  - flag: --max-turns
    value: "<count>"
    scope: ["global", "print_mode"]
    default: "unlimited"
    description: "Limits the number of agentic turns in print mode."
    example: "claude -p --max-turns 3 \"query\""
    notes: "Documented but omitted from local 2.1.200 help."
  - flag: --mcp-config
    value: "<configs...>"
    scope: ["global", "mcp", "agents"]
    default: ""
    description: "Loads MCP servers from JSON files or inline JSON strings."
    example: "claude --mcp-config ./mcp.json"
    notes: "Use `--strict-mcp-config` to ignore discovered MCP configuration."
  - flag: --model
    value: "<model>"
    scope: ["global"]
    default: ""
    description: "Sets model alias or full model ID for the session."
    example: "claude --model claude-sonnet-5"
    notes: "Overrides model setting and `ANTHROPIC_MODEL`."
  - flag: --name
    value: "<name>"
    scope: ["global", "sessions"]
    default: ""
    description: "Sets a display name for the session."
    example: "claude -n my-feature-work"
    notes: "Short alias: `-n`."
  - flag: --no-chrome
    value: ""
    scope: ["global"]
    default: "false"
    description: "Disables Claude in Chrome integration for the session."
    example: "claude --no-chrome"
    notes: "Boolean."
  - flag: --no-session-persistence
    value: ""
    scope: ["global", "print_mode"]
    default: "false"
    description: "Disables saving sessions to disk."
    example: "claude -p --no-session-persistence \"query\""
    notes: "Print mode only; `CLAUDE_CODE_SKIP_PROMPT_HISTORY` does the same in any mode."
  - flag: --output-format
    value: "<text|json|stream-json>"
    scope: ["global", "print_mode"]
    default: "text"
    description: "Selects print-mode output format."
    example: "claude -p \"query\" --output-format json"
    notes: "Use stream-json for structured streaming wrappers."
  - flag: --permission-mode
    value: "<default|acceptEdits|plan|auto|dontAsk|bypassPermissions|manual>"
    scope: ["global", "permissions"]
    default: ""
    description: "Sets the starting permission mode."
    example: "claude --permission-mode plan"
    notes: "Local help includes `manual`; docs include `default`."
  - flag: --permission-prompt-tool
    value: "<tool>"
    scope: ["global", "print_mode", "permissions"]
    default: ""
    description: "Delegates non-interactive permission prompts to an MCP tool."
    example: "claude -p --permission-prompt-tool mcp_auth_tool \"query\""
    notes: "Documented but omitted from local 2.1.200 help."
  - flag: --plugin-dir
    value: "<path>"
    scope: ["global", "plugins", "agents"]
    default: "[]"
    description: "Loads a plugin directory or zip for this session."
    example: "claude --plugin-dir ./my-plugin"
    notes: "Repeatable."
  - flag: --plugin-url
    value: "<url>"
    scope: ["global", "plugins"]
    default: "[]"
    description: "Fetches a plugin zip URL for this session."
    example: "claude --plugin-url https://example.com/plugin.zip"
    notes: "Repeatable."
  - flag: --print
    value: ""
    scope: ["global", "print_mode"]
    default: "false"
    description: "Prints a response and exits."
    example: "claude -p \"query\""
    notes: "Short alias: `-p`; skips workspace trust dialog in non-interactive mode."
  - flag: --prompt-suggestions
    value: "[boolean]"
    scope: ["global", "print_mode"]
    default: "false"
    description: "Emits prompt_suggestion messages after turns."
    example: "claude -p --prompt-suggestions --output-format stream-json --verbose \"query\""
    notes: "Requires print mode, stream-json output, and verbose mode."
  - flag: --remote
    value: "<task>"
    scope: ["global", "remote"]
    default: ""
    description: "Creates a new web session on claude.ai."
    example: "claude --remote \"Fix the login bug\""
    notes: "Documented but omitted from local 2.1.200 help; requires cloud auth."
  - flag: --remote-control
    value: "[name]"
    scope: ["global", "remote"]
    default: ""
    description: "Starts an interactive session with Remote Control enabled."
    example: "claude --remote-control \"My Project\""
    notes: "Alias: `--rc` in docs; local logged-out subcommand probe exited 1."
  - flag: --remote-control-session-name-prefix
    value: "<prefix>"
    scope: ["global", "remote"]
    default: "hostname"
    description: "Sets prefix for auto-generated Remote Control session names."
    example: "claude --remote-control-session-name-prefix dev-box"
    notes: "Equivalent env var: `CLAUDE_REMOTE_CONTROL_SESSION_NAME_PREFIX`."
  - flag: --replay-user-messages
    value: ""
    scope: ["global", "print_mode"]
    default: "false"
    description: "Re-emits stdin user messages to stdout for acknowledgement."
    example: "claude -p --input-format stream-json --output-format stream-json --verbose --replay-user-messages"
    notes: "Requires stream-json input and output."
  - flag: --resume
    value: "[session]"
    scope: ["global", "sessions"]
    default: ""
    description: "Resumes a conversation by id/name or opens a picker."
    example: "claude --resume auth-refactor"
    notes: "Short alias: `-r`; picker requires interaction."
  - flag: --safe-mode
    value: ""
    scope: ["global"]
    default: "false"
    description: "Starts with most customizations disabled for troubleshooting."
    example: "claude --safe-mode"
    notes: "Sets `CLAUDE_CODE_SAFE_MODE=1`; managed policy still partly applies."
  - flag: --session-id
    value: "<uuid>"
    scope: ["global", "sessions"]
    default: ""
    description: "Uses a specific UUID for the conversation."
    example: "claude --session-id 550e8400-e29b-41d4-a716-446655440000"
    notes: "Must be a valid UUID."
  - flag: --setting-sources
    value: "<sources>"
    scope: ["global", "configuration", "agents"]
    default: "user,project,local"
    description: "Restricts which setting sources are loaded."
    example: "claude --setting-sources user,project"
    notes: "Comma-separated values: user, project, local."
  - flag: --settings
    value: "<file-or-json>"
    scope: ["global", "configuration", "agents"]
    default: ""
    description: "Loads additional settings from a JSON file or inline JSON."
    example: "claude --settings ./settings.json"
    notes: "Values override same keys in settings files for this session."
  - flag: --strict-mcp-config
    value: ""
    scope: ["global", "mcp", "agents"]
    default: "false"
    description: "Uses only MCP servers from `--mcp-config`."
    example: "claude --strict-mcp-config --mcp-config ./mcp.json"
    notes: "Boolean."
  - flag: --system-prompt
    value: "<prompt>"
    scope: ["global", "system_prompt"]
    default: ""
    description: "Replaces the default system prompt with inline text."
    example: "claude --system-prompt \"You are a Python expert\""
    notes: "Existence only; semantics belong to the sibling `system-prompt` topic."
  - flag: --system-prompt-file
    value: "<path>"
    scope: ["global", "system_prompt"]
    default: ""
    description: "Replaces the default system prompt with file contents."
    example: "claude --system-prompt-file ./prompts/review.txt"
    notes: "Documented but omitted from local 2.1.200 help; semantics belong to the sibling `system-prompt` topic."
  - flag: --teleport
    value: ""
    scope: ["global", "remote"]
    default: "false"
    description: "Resumes a web session in the local terminal."
    example: "claude --teleport"
    notes: "Documented but omitted from local 2.1.200 help; requires claude.ai subscription."
  - flag: --teammate-mode
    value: "<in-process|auto|tmux|iterm2>"
    scope: ["global", "agents"]
    default: "in-process"
    description: "Sets how agent-team teammates display."
    example: "claude --teammate-mode auto"
    notes: "Documented but omitted from local 2.1.200 help."
  - flag: --tmux
    value: "[classic]"
    scope: ["global", "worktree"]
    default: "false"
    description: "Creates a tmux session for a worktree."
    example: "claude -w feature-auth --tmux"
    notes: "Requires `--worktree`."
  - flag: --tools
    value: "<tools...>"
    scope: ["global"]
    default: "default"
    description: "Restricts available built-in tools."
    example: "claude --tools \"Bash,Edit,Read\""
    notes: "Use an empty string to disable all built-in tools."
  - flag: --verbose
    value: ""
    scope: ["global", "print_mode"]
    default: "false"
    description: "Enables verbose turn-by-turn output."
    example: "claude --verbose"
    notes: "Required by several stream-json extensions."
  - flag: --version
    value: ""
    scope: ["global"]
    default: "false"
    description: "Prints the Claude Code version."
    example: "claude --version"
    notes: "Short alias: `-v`."
  - flag: --worktree
    value: "[name]"
    scope: ["global", "worktree"]
    default: ""
    description: "Starts Claude in an isolated git worktree."
    example: "claude -w feature-auth"
    notes: "Short alias: `-w`."
  - flag: --all
    value: ""
    scope: ["agents"]
    default: "false"
    description: "Includes completed sessions in `claude agents --json` output."
    example: "claude agents --json --all"
    notes: "Scoped to `agents`."
  - flag: --cwd
    value: "<path>"
    scope: ["agents"]
    default: ""
    description: "Filters agent view/listing to sessions started under a path."
    example: "claude agents --json --cwd ."
    notes: "Scoped to `agents`."
  - flag: --json
    value: ""
    scope: ["agents", "plugin list", "ultrareview"]
    default: "false"
    description: "Requests JSON output where supported."
    example: "claude plugin list --json"
    notes: "Meaning is scoped to each subcommand."
  - flag: --text
    value: ""
    scope: ["auth status"]
    default: "false"
    description: "Prints human-readable authentication status instead of JSON."
    example: "claude auth status --text"
    notes: "Local logged-out probe exited 1 with text."
  - flag: --force
    value: ""
    scope: ["install"]
    default: "false"
    description: "Forces native binary installation even if already installed."
    example: "claude install --force latest"
    notes: "Scoped to `install`."
  - flag: --config
    value: "<path>"
    scope: ["gateway"]
    default: ""
    description: "Path to gateway YAML configuration."
    example: "claude gateway --config gateway.yaml"
    notes: "Required for gateway server mode."
  - flag: --transport
    value: "<stdio|sse|http>"
    scope: ["mcp add"]
    default: "stdio"
    description: "Selects transport when adding an MCP server."
    example: "claude mcp add --transport http sentry https://mcp.sentry.dev/mcp"
    notes: "Scoped to `mcp add`."
  - flag: --header
    value: "<header>"
    scope: ["mcp add"]
    default: ""
    description: "Adds an HTTP header when adding an HTTP MCP server."
    example: "claude mcp add --transport http corridor https://example.com/mcp --header \"Authorization: Bearer ...\""
    notes: "Scoped to `mcp add`; repeatable."
  - flag: --env
    value: "<KEY=VALUE>"
    scope: ["mcp add"]
    default: ""
    description: "Adds environment variables for a stdio MCP server."
    example: "claude mcp add my-server -e API_KEY=xxx -- npx my-mcp-server"
    notes: "Short alias observed in help example: `-e`."
  - flag: --dry-run
    value: ""
    scope: ["project purge"]
    default: "false"
    description: "Previews project-state deletion."
    example: "claude project purge --dry-run ."
    notes: "Scoped to `project purge`."
  - flag: --yes
    value: ""
    scope: ["project purge"]
    default: "false"
    description: "Skips confirmation for project-state deletion."
    example: "claude project purge --yes ."
    notes: "Alias may be `-y`; use only for deliberate destructive cleanup."
  - flag: --timeout
    value: "<minutes>"
    scope: ["ultrareview"]
    default: "30"
    description: "Maximum minutes to wait for an ultrareview."
    example: "claude ultrareview --timeout 10 --json"
    notes: "Scoped to `ultrareview`."
config_paths:
  - os: macos
    scope: user
    path: "~/.claude/settings.json"
    format: json
    notes: "User settings. Local file exists and contains keys such as hooks, model, permissions, statusLine, enabledPlugins, and effortLevel."
  - os: linux
    scope: user
    path: "~/.claude/settings.json"
    format: json
    notes: "Same user settings path under the Linux home directory."
  - os: windows
    scope: user
    path: "%USERPROFILE%\\.claude\\settings.json"
    format: json
    notes: "Windows expansion of `~/.claude/settings.json`."
  - os: macos
    scope: user
    path: "~/.claude.json"
    format: json
    notes: "Mutable user state/cache including install method, project state, OAuth account metadata, feature caches, and usage caches. Local file exists."
  - os: linux
    scope: user
    path: "~/.claude.json"
    format: json
    notes: "Mutable user state/cache written next to the home config directory."
  - os: windows
    scope: user
    path: "%USERPROFILE%\\.claude.json"
    format: json
    notes: "Windows mutable user state/cache path."
  - os: macos
    scope: repo
    path: ".claude/settings.json"
    format: json
    notes: "Project settings checked into source control when present."
  - os: linux
    scope: repo
    path: ".claude/settings.json"
    format: json
    notes: "Project settings checked into source control when present."
  - os: windows
    scope: repo
    path: ".claude\\settings.json"
    format: json
    notes: "Project settings checked into source control when present."
  - os: macos
    scope: repo
    path: ".claude/settings.local.json"
    format: json
    notes: "Local project settings; local repo file exists and is an empty JSON object."
  - os: linux
    scope: repo
    path: ".claude/settings.local.json"
    format: json
    notes: "Local project settings; should be gitignored if created manually."
  - os: windows
    scope: repo
    path: ".claude\\settings.local.json"
    format: json
    notes: "Local project settings; should be gitignored if created manually."
  - os: macos
    scope: repo
    path: ".mcp.json"
    format: json
    notes: "Project-scoped MCP servers. Unapproved servers are shown as pending by `claude mcp list/get`."
  - os: linux
    scope: repo
    path: ".mcp.json"
    format: json
    notes: "Project-scoped MCP servers."
  - os: windows
    scope: repo
    path: ".mcp.json"
    format: json
    notes: "Project-scoped MCP servers."
  - os: macos
    scope: system
    path: "managed-settings.json"
    format: json
    notes: "Enterprise managed settings file; exact system-level location varies by deployment."
  - os: linux
    scope: system
    path: "managed-settings.json"
    format: json
    notes: "Enterprise managed settings file; exact system-level location varies by deployment."
  - os: windows
    scope: system
    path: "managed-settings.json or registry/MDM policy"
    format: json
    notes: "Enterprise managed settings can be delivered through registry/MDM policy or file-based managed settings."
  - os: macos
    scope: env
    path: "CLAUDE_CONFIG_DIR"
    format: other
    notes: "Relocates the default `~/.claude` tree; local keychain credentials may still be outside it."
  - os: linux
    scope: env
    path: "CLAUDE_CONFIG_DIR"
    format: other
    notes: "Relocates the default `~/.claude` tree."
  - os: windows
    scope: env
    path: "CLAUDE_CONFIG_DIR"
    format: other
    notes: "Relocates the default `%USERPROFILE%\\.claude` tree."
env_vars:
  - name: CLAUDE_CONFIG_DIR
    effect: "Relocates Claude Code's user configuration and data directory normally addressed as `~/.claude`."
  - name: CLAUDE_CODE_SAFE_MODE
    effect: "Set by `--safe-mode`; disables most user/project customizations while leaving auth, model selection, built-in tools, and permissions available."
  - name: CLAUDE_CODE_SIMPLE
    effect: "Equivalent to `--bare`; uses a minimal setup, disables most customization discovery, and does not read OAuth/keychain credentials."
  - name: CLAUDE_CODE_SIMPLE_SYSTEM_PROMPT
    effect: "Requests a shorter system prompt and abbreviated tool descriptions without disabling normal tool/customization discovery."
  - name: CLAUDE_AX_SCREEN_READER
    effect: "Enables screen-reader friendly flat output unless overridden by `--ax-screen-reader` or settings."
  - name: CLAUDE_CODE_TMPDIR
    effect: "Selects where Claude Code creates temporary files."
  - name: CLAUDE_CODE_DISABLE_NONESSENTIAL_TRAFFIC
    effect: "Disables nonessential background/network traffic."
  - name: DISABLE_AUTOUPDATER
    effect: "Disables automatic update checks/installation."
  - name: DISABLE_BUG_COMMAND
    effect: "Disables the `/bug` feedback command."
  - name: DISABLE_COST_WARNINGS
    effect: "Suppresses cost warning messages."
  - name: DISABLE_ERROR_REPORTING
    effect: "Disables automatic error reporting."
  - name: DISABLE_NON_ESSENTIAL_MODEL_CALLS
    effect: "Disables nonessential model calls."
  - name: DISABLE_TELEMETRY
    effect: "Disables telemetry."
  - name: FORCE_AUTOUPDATER
    effect: "Forces updater behavior even when normal install-method detection would not."
  - name: CLAUDE_CODE_PACKAGE_MANAGER_AUTO_UPDATE
    effect: "Allows Claude Code to run Homebrew or WinGet upgrade commands in the background when updates are available."
  - name: CLAUDE_CODE_DISABLE_TERMINAL_TITLE
    effect: "Disables terminal title updates."
  - name: CLAUDE_CODE_DISABLE_TERMINAL_ALTERNATE_SCREEN
    effect: "Disables use of the terminal alternate screen."
  - name: CLAUDE_CODE_SHELL_PREFIX
    effect: "Adds a wrapper prefix to shell commands launched by Claude Code."
  - name: SHELL
    effect: "Influences which shell Claude Code uses on Unix-like systems."
  - name: COMSPEC
    effect: "Influences shell discovery on Windows."
  - name: CLAUDE_CODE_USE_POWERSHELL_TOOL
    effect: "Controls availability/use of the PowerShell tool."
  - name: CLAUDE_CODE_GIT_BASH_PATH
    effect: "On Windows, points Claude Code at Git Bash when auto-discovery fails."
  - name: CLAUDE_CODE_SKIP_PROMPT_HISTORY
    effect: "Skips writing prompt history and session transcripts; sessions do not appear in resume/continue/history."
  - name: CLAUDE_CODE_SYNC_PLUGIN_INSTALL
    effect: "In print mode, waits for plugin installation before the first query."
  - name: CLAUDE_CODE_SYNC_PLUGIN_INSTALL_TIMEOUT_MS
    effect: "Bounds synchronous plugin-install waiting in milliseconds."
  - name: CLAUDE_CODE_SYNC_SKILLS
    effect: "In print mode, downloads enabled claude.ai skills into `~/.claude/skills/` before the first query and periodically resyncs."
  - name: CLAUDE_CODE_SYNC_SKILLS_WAIT_TIMEOUT_MS
    effect: "Bounds the initial print-mode wait for skill sync."
  - name: CLAUDE_CODE_SYNC_SKILLS_INSTALL_TIMEOUT_MS
    effect: "Bounds mid-session skill resync."
  - name: CLAUDE_CODE_SUBPROCESS_ENV_SCRUB
    effect: "Strips Anthropic and cloud-provider credentials from subprocess environments; on Linux also isolates Bash subprocesses in a PID namespace."
  - name: CLAUDE_CODE_SYNTAX_HIGHLIGHT
    effect: "Set to `false` to disable syntax highlighting in diff output."
  - name: CLAUDE_CODE_TMUX_TRUECOLOR
    effect: "Allows 24-bit truecolor output inside tmux when tmux is configured for truecolor."
  - name: CLAUDE_CODE_PLUGIN_PREFER_HTTPS
    effect: "Clones GitHub shorthand plugin sources over HTTPS instead of SSH."
  - name: CLAUDE_REMOTE_CONTROL_SESSION_NAME_PREFIX
    effect: "Sets the prefix for auto-generated Remote Control session names."
  - name: API_TIMEOUT_MS
    effect: "Sets API request timeout; can also be placed under the `env` key in settings files."
  - name: BASH_DEFAULT_TIMEOUT_MS
    effect: "Sets default Bash tool timeout when placed in shell environment or settings `env`."
  - name: USE_BUILTIN_RIPGREP
    effect: "Set to `0` to use system `rg` instead of Claude Code's bundled ripgrep."
machine_introspection:
  - command: "claude auth status"
    purpose: env
    machine_readable: true
    output_format: json
    useful_for_codegen: false
    notes: "Reports `loggedIn`, `authMethod`, and `apiProvider`; local logged-out run emitted JSON and exited 1."
  - command: "claude agents --json"
    purpose: other
    machine_readable: true
    output_format: json
    useful_for_codegen: false
    notes: "Lists active sessions with pid, cwd, kind, startedAt, sessionId, name, status, and waitingFor when applicable."
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
    notes: "Prints built-in auto-mode classifier environment, allow, soft_deny, and hard_deny rules."
  - command: "claude auto-mode config"
    purpose: config_dump
    machine_readable: true
    output_format: json
    useful_for_codegen: true
    notes: "Prints effective auto-mode config after settings are applied."
  - command: "claude plugin list --json"
    purpose: plugins
    machine_readable: true
    output_format: json
    useful_for_codegen: false
    notes: "Local output was a JSON array of installed plugins."
  - command: "claude plugin list --json --available"
    purpose: plugins
    machine_readable: true
    output_format: json
    useful_for_codegen: false
    notes: "Documented plugin inventory including available marketplace plugins."
  - command: "claude daemon status"
    purpose: doctor
    machine_readable: false
    output_format: text
    useful_for_codegen: false
    notes: "Prints supervisor state, socket directory, worker count, roster, and log presence; local run exited 1 with `not running`, an expected state."
  - command: "claude doctor"
    purpose: doctor
    machine_readable: false
    output_format: text
    useful_for_codegen: false
    notes: "Official diagnostics command. Local non-interactive run timed out after 20 seconds with no useful text."
  - command: "claude mcp list"
    purpose: mcp
    machine_readable: false
    output_format: text
    useful_for_codegen: false
    notes: "Local output was human text: no MCP servers configured. MCP details belong to the narrower MCP topic."
wrapper_notes:
  - "Use `claude -p` / `--print` for non-interactive wrapper runs; plain `claude \"query\"` starts an interactive session with an initial prompt."
  - "`--output-format stream-json` is the primary structured stream output for print mode; prompt suggestions, partial messages, hook events, and replayed user messages require stream-json and often `--verbose`."
  - "`claude --help` is incomplete by design; official docs explicitly say absence from help does not mean a flag is unavailable. Local v2.1.200 help omits documented flags such as `--advisor`, `--system-prompt-file`, `--append-system-prompt-file`, `--max-turns`, `--permission-prompt-tool`, `--remote`, `--teleport`, and `--teammate-mode`."
  - "Several background-session commands (`daemon`, `attach`, `logs`, `respawn`, `rm`, `stop`) are hidden from top-level help in v2.1.200 but still respond to `--help`."
  - "`claude auth status` emits JSON but exits 1 for the expected unauthenticated state; wrappers should parse stdout before classifying the exit as a crash."
  - "`claude daemon status` exits 1 when no daemon is running; this is an expected diagnostic state."
  - "`claude doctor` should be guarded with a timeout in wrappers; local non-interactive execution timed out after 20 seconds."
  - "Login, MCP OAuth login, Remote Control, attach, setup-token, and many plugin/auth operations may require a TTY, browser, subscription, or state mutation."
  - "`--bare` / `CLAUDE_CODE_SIMPLE` avoids most discovery and avoids OAuth/keychain credential reads, so wrappers using it need API-key or helper-based auth."
  - "`--safe-mode` disables most customizations but managed policy can still partially apply."
  - "`CLAUDE_CONFIG_DIR` is the broadest public isolation knob for config/session/plugin state; on macOS credentials can still come from Keychain unless `--bare` is used."
  - "Native/npm installs spawn a per-platform native binary rather than bundled JavaScript; npm optional dependencies must be present."
  - "Homebrew stable can lag latest; on 2026-07-03 npm `latest` and local `claude --version` reported 2.1.200 while public changelog docs topped out at 2.1.199."
  - "Windows without Git for Windows uses PowerShell for shell commands; with Git for Windows it uses Git Bash unless configured otherwise."
  - "Package-manager installs generally do not auto-update by default; native installs do."
changes:
  - "Updated verified latest version from 2.1.199 to 2.1.200 based on local `claude --version` and npm registry dist-tags."
  - "Recorded that the public changelog page currently tops out at 2.1.199 even though npm/latest and the local binary are 2.1.200."
  - "Added local v2.1.200 observations that hidden background-session commands still work even though top-level help omits them."
  - "Expanded the switch inventory with v2.1.200 top-level help and official-doc flags including background, remote, worktree, init, maintenance, max-turns, permission-prompt, plugin URL/dir, and system-prompt file flags."
  - "Refreshed configuration discovery from local `~/.claude/settings.json`, `~/.claude.json`, and repo `.claude/settings.local.json` key inspection."
  - "Updated machine introspection findings for `auth status`, `agents --json`, auto-mode JSON commands, plugin JSON listing, daemon status, doctor timeout behavior, and MCP list text output."
requires_claudine_update: true
reason: "Claudine's Claude wrapper/provider metadata should account for the verified 2.1.200 CLI surface: hidden-but-working background-session commands, incomplete help output, new or newly verified launch flags (`--background`, `--worktree`, `--remote`, `--teleport`, `--max-turns`, `--permission-prompt-tool`, system-prompt file flags), and expected non-zero exits for auth/daemon diagnostic states."
---

# Claude Code CLI Surface

## Overview

Claude Code is Anthropic's official agentic coding CLI. The public terminal command is `claude`: running `claude` starts an interactive session, `claude "query"` starts an interactive session with an initial prompt, and `claude -p "query"` runs print/SDK mode and exits.

The latest verified version for this research is `2.1.200`. I verified it on 2026-07-03 in two ways: local `claude --version` returned `2.1.200 (Claude Code)`, and `npm view @anthropic-ai/claude-code version dist-tags --json` returned `version: 2.1.200`, `latest: 2.1.200`, `next: 2.1.200`, and `stable: 2.1.193`. The public changelog page was useful for release context but currently tops out at `2.1.199`, so it lags the package registry and local binary for this specific verification.

Primary URLs:

| Resource | URL |
| --- | --- |
| Homepage | [https://claude.ai/code](https://claude.ai/code) |
| Repository | unknown; the public docs link to a generated changelog source, but no public CLI source repository is documented |
| General docs | [https://code.claude.com/docs/en/overview](https://code.claude.com/docs/en/overview) |
| CLI reference | [https://code.claude.com/docs/en/cli-reference](https://code.claude.com/docs/en/cli-reference) |

## Installation and Binaries

The installed macOS binary on this host is `/Users/ken/.local/bin/claude`, a symlink to `/Users/ken/.local/share/claude/versions/2.1.200`; `file` identifies the target as a native Mach-O arm64 executable. Official docs say the native and npm installers now install native binaries rather than a Node.js runtime wrapper.

| OS | Binary | Alternate shims | Install methods |
| --- | --- | --- | --- |
| macOS | `claude` | none observed | Native installer, Homebrew cask, npm |
| Linux | `claude` | none documented | Native installer, apt, dnf, apk, npm |
| Windows | `claude.exe` | `claude`, npm command shims such as `claude.cmd` | Native PowerShell/CMD installer, WinGet, npm |

Official install commands:

```sh
curl -fsSL https://claude.ai/install.sh | bash
irm https://claude.ai/install.ps1 | iex
curl -fsSL https://claude.ai/install.cmd -o install.cmd && install.cmd && del install.cmd
brew install --cask claude-code
brew install --cask claude-code@latest
winget install Anthropic.ClaudeCode
sudo apt install claude-code
sudo dnf install claude-code
apk add claude-code
npm install -g @anthropic-ai/claude-code
```

The apt, dnf, and apk commands require adding Anthropic's signed package repository first. Native installs auto-update. Homebrew, WinGet, apt, dnf, and apk installs do not auto-update through Claude Code by default; Homebrew and WinGet can opt into package-manager auto-update with `CLAUDE_CODE_PACKAGE_MANAGER_AUTO_UPDATE=1`.

## Subcommands

| Command or mode | Description | Non-interactive wrapper use |
| --- | --- | --- |
| `claude` | Opens an interactive session. | No |
| `claude "query"` | Opens an interactive session seeded with a prompt. | No |
| `claude -p "query"` | Runs print/SDK mode and exits. | Yes |
| `claude agents` | Opens agent view; `--json` prints active sessions. | Mixed; `--json` is scriptable |
| `claude attach <id>` | Attaches to a background session. | No |
| `claude auth login` | Signs in to Anthropic/Claude. | No |
| `claude auth logout` | Logs out and mutates auth state. | Usually no |
| `claude auth status` | Prints auth status as JSON by default. | Yes |
| `claude auto-mode defaults` | Prints built-in auto-mode rules as JSON. | Yes |
| `claude auto-mode config` | Prints effective auto-mode rules as JSON. | Yes |
| `claude auto-mode critique` | Gets AI feedback on custom auto-mode rules. | No static metadata use |
| `claude daemon status` | Prints background-session supervisor diagnostics. | Yes, text only |
| `claude daemon run` | Runs the supervisor in the foreground. | Long-running |
| `claude daemon logs` | Tails daemon logs. | Long-running |
| `claude daemon stop` | Stops the supervisor and optionally workers. | Yes, mutating |
| `claude daemon uninstall` | Removes service integration. | Yes, destructive |
| `claude gateway` | Runs the enterprise auth/telemetry gateway. | Long-running server |
| `claude install [target]` | Installs/reinstalls native build. | Yes, mutating |
| `claude logs <id>` | Prints background-session terminal output. | Yes |
| `claude mcp ...` | Manages MCP servers. | Mixed; list/get are text, login/logout mutate credentials |
| `claude plugin ...` | Manages plugins. | Mixed; `plugin list --json` is scriptable |
| `claude project purge` | Deletes Claude Code project state. | Mutating; use `--dry-run` for preview |
| `claude remote-control` / `--remote-control` | Starts Remote Control. | No; requires login/subscription |
| `claude respawn <id>|--all` | Restarts background sessions. | Yes, mutating |
| `claude rm <id>` | Deletes a background session and its worktree. | Yes, destructive |
| `claude setup-token` | Creates a long-lived token. | No; secret-bearing auth flow |
| `claude stop <id>` | Stops a background session. | Yes |
| `claude ultrareview` | Runs cloud-hosted code review. | Yes, with auth/network |
| `claude update` / `upgrade` | Checks for and installs updates. | Yes, mutating |

Top-level help in local v2.1.200 omits `daemon`, `attach`, `logs`, `respawn`, `rm`, and `stop`, but direct `--help` probes for those command names succeeded. Wrappers should not infer removal from top-level help alone.

## CLI Switch Inventory

The frontmatter `cli_switches` list is the detailed inventory. It combines local v2.1.200 help, targeted subcommand help, and official CLI documentation. When local help and docs disagree, this document trusts official docs for documented flags because the CLI reference explicitly says `claude --help` does not list every flag. Local help is still treated as evidence for observed-only flags such as `--brief` and `--file`.

Wrapper-critical groups:

| Scope | Switches |
| --- | --- |
| Non-interactive execution | `-p`/`--print`, `--input-format`, `--output-format`, `--json-schema`, `--max-turns`, `--max-budget-usd`, `--no-session-persistence` |
| Structured stream extensions | `--include-hook-events`, `--include-partial-messages`, `--prompt-suggestions`, `--replay-user-messages`, `--verbose` |
| Config isolation | `--settings`, `--setting-sources`, `--bare`, `--safe-mode`, `--mcp-config`, `--strict-mcp-config`, `CLAUDE_CONFIG_DIR` |
| Background/session management | `--background`/`--bg`, `--worktree`/`-w`, `--tmux`, `--name`/`-n`, `--session-id`, `--resume`/`-r`, `--continue`/`-c`, `--fork-session`, `--from-pr` |
| Remote/cloud entry points | `--remote`, `--remote-control`/`--rc`, `--remote-control-session-name-prefix`, `--teleport` |
| System-prompt delivery | `--system-prompt`, `--system-prompt-file`, `--append-system-prompt`, `--append-system-prompt-file`, `--exclude-dynamic-system-prompt-sections` |

System-prompt flags exist in both interactive and non-interactive modes. This topic records their names, value shapes, and example invocations only; replace-vs-append semantics, file-vs-inline behavior, and mode interactions belong to the sibling `system-prompt` research topic.

## Configuration Discovery

Claude Code discovers hierarchical JSON settings and mutable state:

| Scope | macOS/Linux path | Windows path | Notes |
| --- | --- | --- | --- |
| User settings | `~/.claude/settings.json` | `%USERPROFILE%\.claude\settings.json` | Local file exists and contains hooks, permissions, model, status line, plugins, and related user settings. |
| Mutable user state | `~/.claude.json` | `%USERPROFILE%\.claude.json` | Local file exists and contains install/update state, projects, OAuth account metadata, feature caches, and usage caches. |
| Project settings | `.claude/settings.json` | `.claude\settings.json` | Shareable project settings. Not present in this repo during inspection. |
| Local project settings | `.claude/settings.local.json` | `.claude\settings.local.json` | Local/private project settings. This repo has an empty JSON object at that path. |
| Project MCP | `.mcp.json` | `.mcp.json` | Project-scoped MCP servers; unapproved servers are reported as pending. |
| Managed settings | varies; `managed-settings.json`, MDM, or policy delivery | registry/MDM or `managed-settings.json` | Highest-precedence enterprise policy. |
| Config-dir override | `CLAUDE_CONFIG_DIR` | `CLAUDE_CONFIG_DIR` | Relocates the normal `.claude` tree. |

Settings precedence is managed policy, command-line/session settings, local project, project, then user. Docs note that permission rules merge differently from scalar settings, and invalid managed entries are stripped tolerantly rather than disabling all policy. Claude Code also writes transcripts, prompt history, file snapshots, caches, plugin data, and logs under `~/.claude`; first use may create or update these files.

Trust and state caveats:

- Print mode skips the workspace trust dialog, so wrappers should only use it in trusted directories.
- Settings files that fail validation are silently ignored in print mode instead of showing an error dialog.
- `--bare` skips most customization discovery and avoids OAuth/keychain credential reads.
- `--safe-mode` disables most user/project customizations but still allows some managed policy to apply.

## Environment Variables

The frontmatter `env_vars` list records general CLI/runtime variables only. Model-endpoint variables, permission policy variables, MCP-specific variables, logging/telemetry variables, and streaming-specific variables are intentionally left to their narrower topics unless they also change general CLI behavior.

Important general runtime controls include:

| Variable | Effect |
| --- | --- |
| `CLAUDE_CONFIG_DIR` | Relocates user configuration/data normally under `~/.claude`. |
| `CLAUDE_CODE_SAFE_MODE` | Mirrors `--safe-mode`; disables most customizations for troubleshooting. |
| `CLAUDE_CODE_SIMPLE` | Mirrors `--bare`; minimal execution and no OAuth/keychain credential reads. |
| `CLAUDE_CODE_SKIP_PROMPT_HISTORY` | Avoids writing prompt history and transcripts. |
| `CLAUDE_CODE_SYNC_PLUGIN_INSTALL` | Makes print mode wait for plugin installation before the first query. |
| `CLAUDE_CODE_SYNC_SKILLS` | Makes print mode sync enabled claude.ai skills before the first query and periodically after. |
| `CLAUDE_CODE_SUBPROCESS_ENV_SCRUB` | Removes provider credentials from child process environments; adds Linux PID namespace isolation for Bash subprocesses. |
| `CLAUDE_CODE_GIT_BASH_PATH` | Points Windows shell-tool discovery at Git Bash. |
| `CLAUDE_REMOTE_CONTROL_SESSION_NAME_PREFIX` | Sets default Remote Control session name prefix. |
| `USE_BUILTIN_RIPGREP` | Set to `0` to use system `rg` instead of the bundled ripgrep. |

Environment variables can be set in the shell or under the `env` key in settings files. Where the same behavior has both an environment variable and a settings field, the environment variable takes precedence.

## Machine Introspection

Machine-usable or wrapper-useful probes:

| Command | Format | Machine-readable | Wrapper use |
| --- | --- | --- | --- |
| `claude auth status` | JSON | Yes | Provider readiness; exits 1 when logged out. |
| `claude agents --json` | JSON array | Yes | Active session discovery. |
| `claude agents --json --all` | JSON array | Yes | Active plus completed background-session discovery. |
| `claude auto-mode defaults` | JSON | Yes | Built-in policy/catalog inspection. |
| `claude auto-mode config` | JSON | Yes | Effective auto-mode configuration. |
| `claude plugin list --json` | JSON array | Yes | Installed plugin inventory. |
| `claude plugin list --json --available` | JSON | Yes | Installed plus available plugin inventory, per docs. |
| `claude daemon status` | Text | No | Supervisor diagnostics; exits 1 when not running. |
| `claude doctor` | Text | No | Install/config diagnostics; local non-interactive run timed out after 20 seconds. |
| `claude mcp list` | Text | No | Human MCP summary; local output reported no servers configured. |

Generic `--help` and `--version` are not counted as machine introspection in frontmatter because they do not expose structured provider state. They remain important research evidence: `--version` verified the installed version, and help output revealed current visible/hidden command differences.

## Wrapper Notes

Use `claude -p` for automation. Plain `claude "query"` is not a one-shot command; it opens an interactive session seeded with the prompt. Pick `--output-format` explicitly, and use `stream-json` for structured streaming wrappers.

Expected non-zero exits need special handling. `claude auth status` returns useful JSON and exits `1` when logged out. `claude daemon status` exits `1` when no supervisor is running. `claude doctor` is a diagnostics command but should be run with a wrapper timeout; it did not return useful output within 20 seconds locally.

Help is not an authoritative complete inventory. Official docs list flags omitted by local v2.1.200 help, and local top-level help omits several hidden-but-working background-session commands. A wrapper metadata refresh should combine docs, targeted command probes, and negative probes.

For isolated runs, prefer `CLAUDE_CONFIG_DIR` plus explicit `--settings`, `--setting-sources`, `--mcp-config`, and `--strict-mcp-config` where appropriate. `--bare` is stronger but changes authentication behavior by avoiding OAuth/keychain credentials. `--safe-mode` is useful for troubleshooting user customization failures, but managed policy can still affect a run.

Windows shell behavior matters. Docs recommend Git for Windows; without it, Claude Code uses PowerShell for shell commands. Use `CLAUDE_CODE_GIT_BASH_PATH` when Git Bash auto-discovery fails.

## Changelog

- 2026-07-03: Updated verified latest version from `2.1.199` to `2.1.200` using local `claude --version` and npm registry dist-tags.
- 2026-07-03: Recorded that the public changelog currently tops out at `2.1.199`, so npm/local binary evidence was preferred for latest-version verification.
- 2026-07-03: Added v2.1.200 local observations for hidden-but-working background-session commands (`daemon`, `attach`, `logs`, `respawn`, `rm`, `stop`).
- 2026-07-03: Expanded switch inventory with official-doc and local-help flags for background sessions, worktrees, remote sessions, setup hooks, structured print mode, plugins, and system-prompt file delivery.
- 2026-07-03: Refreshed local configuration discovery from `~/.claude/settings.json`, `~/.claude.json`, and repo `.claude/settings.local.json` without exposing private values.
- 2026-07-03: Updated introspection notes for JSON auth/session/plugin/auto-mode surfaces and expected text/non-zero diagnostic states.

## Sources

- [Claude Code overview](https://code.claude.com/docs/en/overview)
- [Claude Code CLI reference](https://code.claude.com/docs/en/cli-reference)
- [Advanced setup](https://code.claude.com/docs/en/setup)
- [Claude Code settings](https://code.claude.com/docs/en/settings)
- [Environment variables](https://code.claude.com/docs/en/env-vars)
- [Explore the .claude directory](https://code.claude.com/docs/en/claude-directory)
- [Debug your configuration](https://code.claude.com/docs/en/debug-your-config)
- [Plugins reference](https://code.claude.com/docs/en/plugins-reference)
- [Claude Code changelog](https://code.claude.com/docs/en/changelog)
- Local inspection on 2026-07-03: `command -v claude`, `file "$(command -v claude)"`, `ls -l "$(command -v claude)"`, `claude --version`, `claude --help`, `claude auth --help`, `claude agents --help`, `claude auto-mode --help`, `claude mcp --help`, `claude plugin --help`, `claude project --help`, `claude install --help`, `claude gateway --help`, `claude ultrareview --help`, `claude daemon --help`, `claude attach --help`, `claude logs --help`, `claude respawn --help`, `claude rm --help`, `claude stop --help`, `claude remote-control --help`, `claude auth status`, `claude auth status --text`, `claude agents --json`, `claude auto-mode defaults`, `claude auto-mode config`, `claude plugin list --json`, `claude mcp list`, `claude daemon status`, `claude doctor`, `npm view @anthropic-ai/claude-code version dist-tags --json`, and key-only inspection of `~/.claude/settings.json`, `~/.claude.json`, and `.claude/settings.local.json`.
